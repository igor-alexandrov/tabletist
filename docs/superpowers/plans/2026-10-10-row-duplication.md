# Duplicating a Row Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A copy of the row under the cursor becomes a pending new row: Mod+D and the row panel's Duplicate in the desktop looks, `yy` then `p` or `P` in the Omarchy look.

**Architecture:** One pure function in `src/edit.rs` turns what a row shows (a `Yank`: its values by column name) into the pending cells of a new row and three lists of column names (copied, cleared, not copied). The reducer makes the yank from the selected row and the copy from the yank, on the path Add row already takes, and keeps the terminal's yank on the tab. The views push three new `Action`s and draw two things from what the new row keeps: the form's line, and `required · unique`.

**Tech Stack:** Rust, egui/eframe (the crmne forks). Spec: `docs/superpowers/specs/2026-10-10-row-duplication-design.md` ("spec N" below is a section of its Behavior, "test N" an entry of its Tests). It is the first half of run 3 of `docs/superpowers/specs/2026-10-07-inserting-rows-design.md`.

Status: written, not built.

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

- **What was run where this plan was written: nothing of it.** It was written from reading the tree at `91fb599`. The code blocks are written against names that exist in that tree and were not compiled. Treat a block as what the code should come to, and expect to correct a name, a type or a borrow on the first build. Where a block and the compiler disagree, the compiler is right and the task's tests say what must hold. Where a test's expected text and the app's differ only in how a value is written (a number's digits, a NULL in TSV), the app is right: fix the expectation and say so in the commit.
- **The branch** is `claude/row-duplication-c25b7a`, in the worktree this plan is in. It holds the spec's two commits on top of `main` at `0474331`.
- House rules that bite here: no em dashes; comments explain why, in the surrounding code's voice; a view pushes `Action`s and changes no state; views draw text only through `TextRole`s; a view never paints a focus ring; every behaviour change has a headless test; code that depends on the engine matches on `Dialect` naming every variant (nothing here adds such a match); do not weaken a lint or delete a test to get green. Task 5 changes what two existing tests expect, and its commit says why.
- `cargo test` takes one name to filter by before `--` and any number after it.
- Commits are signed (1Password), one per task, after its checks pass. If signing fails, do not commit unsigned: leave the task staged and tell the user. Chain `git add` and `git commit` with `&&`. Each message ends with:

      Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>

## What the spec leaves open, and what gets built

| | The spec | What gets built | Why |
|---|---|---|---|
| a | The footer's `yy p duplicate` is "a live key, no longer dashed and faded" | A whole 1 px outline in the palette's outline colour, the key in the text colour and its word dimmed: the dashed cell without its dashes and its fade | The board draws no live one. It is the least that tells the two cells apart |
| b | `yy`: "the first `y` waits the way the first `d` of `dd` waits" | A `y` held down yanks on its first repeat | A `d` held down drops a new row the same way: both are read with their repeats |
| c | "The status line says row yanked" | Said only where a row is selected in the Data view. With none, `yy` copies nothing and says nothing | Nothing was yanked |
| d | The tests' place is not named | A new file, `src/ui/duplicate_tests.rs`, beside `insert_audit_tests.rs`, whose helpers it borrows (they become `pub(super)`). The pure function's tests are in `src/edit.rs` | `src/ui/mod.rs` has 22,000 lines of tests |
| e | Section 4's reasons "in brackets" | `identity`, `generated`, `primary key`, `unique`, `unknown`, `binary`, `too large` | The table's own words |
| f | `YankRow` under a prompt about pending changes | Not among the actions dropped there. `DuplicateRow` and `PutRow` are, as `AddRow` is | A yank changes nothing that is pending |
| g | Spec 4's row 7 names a loaded text over the limit | A pending text over it is not copied either | The spec asks it of the value the yank holds, whichever it is. An editor caps what is typed, so it is rare |

## File map

| File | What changes |
|---|---|
| `src/edit.rs` | `Kept`, `Yank`, `Cleared`, `Left`, `Copied`, `Duplicate`; `Table::yank`, `Table::duplicate`, `Table::first_to_fill`; `NewRow::made`, `NewRow::cleared_unique`; `Edits::add_copy` (task 1) |
| `src/model.rs` | `Action::DuplicateRow`, `YankRow`, `PutRow`; `ObjectTab::yanked` (task 2) |
| `src/app.rs` | the three actions in `App::apply` (task 2) |
| `src/app/editing.rs` | `duplicate_row`, `yank_row`, `put_row`, `put_copy`; `add_row` split into `takes_rows` and `settle_new_row` (task 2) |
| `src/ui/keys.rs` | `take_fresh_press`, Mod+D (task 3); the second `y`, `p` and `P` (task 4); `SHORTCUTS` (task 8) |
| `src/ui/row_panel.rs` | the footer's button and key (task 5); `Fresh::made`, `copy_line`, `form_note` (task 6); `Fresh::unique` and the field's words (task 7) |
| `src/ui/format.rs` | `required_word` (task 7) |
| `src/ui/data_view.rs` | `Changes::unique`, the cell's words (task 7) |
| `src/ui/duplicate_tests.rs` | new: the feature's headless tests (tasks 2 to 7) |
| `src/ui/insert_audit_tests.rs` | helpers become `pub(super)` (task 2); INS-02a and INS-02b out of `ignore` (tasks 3, 4) |
| `src/ui/mod.rs` | `mod duplicate_tests;` (task 2); two tests of the footer (task 5) |
| `docs/_guide/editing-data.md`, `docs/_guide/what-is-tabletist.md`, `docs/_guide/macos.md`, `docs/_guide/omarchy.md`, `docs/_reference/keyboard-shortcuts.md` | the keys and what a copy is (task 8) |
| `docs/superpowers/specs/*.md`, this plan | what is built (task 9) |

---

### Task 1: What a copy of a row holds

Everything here is decided from the page and the structure alone. Nothing of the app is read, and no view changes.

**Files:**
- Modify: `src/edit.rs` (`NewRow` near line 808; `impl Table` near line 283; `Edits::add_row` near line 1176; its `mod tests` from line 1851)

- [ ] **Step 1: Write the failing tests.** In `src/edit.rs`, inside `mod tests`, after the test `a_new_row_that_failed_is_mended_by_a_value_set_in_it`:

They use two helpers the module's tests have already: `text(value)`, a loaded text, and `pending(new)`, a cell ready to save (near line 3537). `typed` is taken there (a column of a type), so a text as a new value is `text_of` here.

```rust
    /// A unique index over `columns`, each a whole column.
    fn unique_index(name: &str, columns: &[&str]) -> IndexInfo {
        let columns: Vec<String> = columns.iter().map(|column| (*column).to_owned()).collect();
        IndexInfo {
            name: name.into(),
            columns: columns.clone(),
            key_columns: Some(columns),
            unique: true,
            ..IndexInfo::default()
        }
    }

    /// The Bookshop's covers: `id` (the database's counter, the key),
    /// `publisher_id` (required), `kind` (a default, a list of values),
    /// `image_data` (a document, NULL in every row) and `created_at` (a
    /// default the database works out).
    fn covers() -> (Structure, RowPage) {
        (
            crate::testing::book_covers_structure(),
            crate::testing::book_covers_page(3),
        )
    }

    /// A copy of the covers' second row, with nothing pending.
    fn copy_of(structure: &Structure, page: &RowPage) -> Duplicate {
        let table = table(Some(structure), page);
        let yank = table.yank(1, &BTreeMap::new()).expect("the second row");
        table.duplicate(&yank)
    }

    /// What `copy` sets in the column `col`.
    fn set_in(copy: &Duplicate, col: usize) -> Option<&NewValue> {
        let mut cells = copy.cells.iter();
        cells.find(|(at, _)| *at == col).map(|(_, cell)| &cell.new)
    }

    /// A text as a cell's new value.
    fn text_of(value: &str) -> NewValue {
        NewValue::Text(value.into())
    }

    #[test]
    fn a_yank_holds_what_the_row_shows() {
        let (structure, page) = covers();
        let table = table(Some(&structure), &page);
        let mut cells = BTreeMap::new();
        cells.insert((1, 2), pending(text_of("audio")));
        let yank = table.yank(1, &cells).expect("the page's row");
        let names: Vec<&str> = yank.cells.iter().map(|(name, _)| name.as_str()).collect();
        assert_eq!(
            names,
            ["id", "publisher_id", "kind", "image_data", "created_at"]
        );
        assert!(yank.cells[0].1 == Kept::Loaded(Value::Int(2)));
        // What is pending, over what was loaded.
        assert!(yank.cells[2].1 == Kept::Pending(text_of("audio")));
        // No such row.
        assert!(table.yank(7, &cells).is_none());
        // A new row gives what is set in it, and nothing for the rest.
        let added = [NewRow {
            id: 0,
            before: 0,
            failed: None,
            made: None,
        }];
        let table = Table {
            added: &added,
            ..table
        };
        cells.insert((new_row(0), 2), pending(text_of("ebook")));
        let yank = table.yank(new_row(0), &cells).expect("the new row");
        assert_eq!(yank.cells.len(), 1);
        assert!(yank.cells[0] == ("kind".to_owned(), Kept::Pending(text_of("ebook"))));
        // A new row the tab does not hold is no row.
        assert!(table.yank(new_row(5), &cells).is_none());
    }

    #[test]
    fn a_copy_clears_what_the_database_gives_and_what_no_other_row_may_hold() {
        let (plain, page) = covers();
        let owned = |columns: &[(&str, Cleared)]| -> Vec<(String, Cleared)> {
            let columns = columns.iter();
            columns.map(|(name, why)| ((*name).to_owned(), *why)).collect()
        };
        let mut structure = plain.clone();
        let index = unique_index("book_covers_publisher_id_key", &["publisher_id"]);
        structure.indexes.push(index);
        let copy = copy_of(&structure, &page);
        assert_eq!(
            copy.made.cleared,
            owned(&[
                ("id", Cleared::Identity),
                ("publisher_id", Cleared::Unique)
            ])
        );
        assert_eq!(copy.made.copied, ["kind", "image_data", "created_at"]);
        assert!(copy.made.left.is_empty());
        assert_eq!(set_in(&copy, 0), None);
        assert_eq!(set_in(&copy, 1), None);
        assert_eq!(set_in(&copy, 2), Some(&text_of("ebook")));
        assert_eq!(set_in(&copy, 4), Some(&text_of("2026-01-12 09:14:03")));
        // A key that is no counter is cleared as the key, and whole.
        let mut keyed = plain.clone();
        keyed.columns[0].identity = false;
        keyed.primary_key = vec!["id".into(), "kind".into()];
        assert_eq!(
            copy_of(&keyed, &page).made.cleared,
            owned(&[("id", Cleared::PrimaryKey), ("kind", Cleared::PrimaryKey)])
        );
        // A column the database computes.
        let mut computed = plain.clone();
        computed.columns[4].generated = true;
        assert_eq!(
            copy_of(&computed, &page).made.cleared,
            owned(&[("id", Cleared::Identity), ("created_at", Cleared::Generated)])
        );
        // A unique index of two columns clears neither: the database
        // judges the pair.
        let mut pair = plain.clone();
        let index = unique_index("by_publisher_and_kind", &["publisher_id", "kind"]);
        pair.indexes.push(index);
        let only_id = owned(&[("id", Cleared::Identity)]);
        assert_eq!(copy_of(&pair, &page).made.cleared, only_id);
        // Nor does one that is not over the whole column, compared as the
        // column compares: its names are not known.
        let mut prefix = plain.clone();
        prefix.indexes.push(IndexInfo {
            key_columns: None,
            ..unique_index("by_prefix", &["publisher_id"])
        });
        assert_eq!(copy_of(&prefix, &page).made.cleared, only_id);
        // Nor an index that holds no row unique.
        let mut loose = plain.clone();
        loose.indexes.push(IndexInfo {
            unique: false,
            ..unique_index("by_publisher", &["publisher_id"])
        });
        assert_eq!(copy_of(&loose, &page).made.cleared, only_id);
        // One over some of the rows counts: the copy might be one of them.
        let mut partial = plain.clone();
        partial.indexes.push(IndexInfo {
            partial: true,
            ..unique_index("live_publishers", &["publisher_id"])
        });
        assert_eq!(
            copy_of(&partial, &page).made.cleared,
            owned(&[
                ("id", Cleared::Identity),
                ("publisher_id", Cleared::Unique)
            ])
        );
    }

    #[test]
    fn a_copy_holds_each_value_as_its_editor_would_start_with_it() {
        let (mut structure, mut page) = covers();
        structure.columns.push(column("in_print", "BOOLEAN"));
        page.columns.push(meta("in_print", "BOOLEAN"));
        for row in &mut page.rows {
            // SQLite and MySQL hold a boolean as 1 and 0.
            row.push(Value::Int(1));
        }
        // A document as it loaded: on one line, with its own spaces.
        let document = r#"{"w":600,  "h":900}"#;
        page.rows[1][3] = text(document);
        let copy = copy_of(&structure, &page);
        assert_eq!(set_in(&copy, 1), Some(&text_of("9100000000000000002")));
        // Not laid out a member to a line, as its editor would lay it.
        assert_eq!(set_in(&copy, 3), Some(&text_of(document)));
        assert_eq!(set_in(&copy, 5), Some(&text_of("true")));
        let mut states = copy.cells.iter().map(|(_, cell)| &cell.state);
        assert!(states.all(|state| *state == State::Ready));
    }

    #[test]
    fn a_null_is_set_only_where_the_column_would_not_be_null_anyway() {
        let (mut structure, mut page) = covers();
        // `kind` takes a NULL, and has its default.
        structure.columns[2].nullable = true;
        page.rows[1][2] = Value::Null;
        let copy = copy_of(&structure, &page);
        // Left alone it would be `print`, not NULL.
        assert_eq!(set_in(&copy, 2), Some(&NewValue::Null));
        // `image_data` has no default: unset, it is NULL already.
        assert_eq!(set_in(&copy, 3), None);
        assert!(copy.made.copied.iter().any(|name| name == "image_data"));
        // A NULL that is pending goes the same way.
        let (_, page) = covers();
        let table = table(Some(&structure), &page);
        let mut cells = BTreeMap::new();
        cells.insert((1, 2), pending(NewValue::Null));
        let copy = table.duplicate(&table.yank(1, &cells).unwrap());
        assert_eq!(set_in(&copy, 2), Some(&NewValue::Null));
    }

    #[test]
    fn a_pending_default_is_left_unset_and_the_moment_of_the_save_is_carried() {
        let (structure, page) = covers();
        let table = table(Some(&structure), &page);
        let mut cells = BTreeMap::new();
        let default = NewValue::Default {
            expression: Some("'print'".into()),
        };
        cells.insert((1, 2), pending(default));
        cells.insert((1, 4), pending(NewValue::Now));
        // What the last save refused of the source is the source's.
        let failed = Pending {
            new: text_of("9100000000000000007"),
            state: State::Failed(Error::query("duplicate key")),
        };
        cells.insert((1, 1), failed);
        let copy = table.duplicate(&table.yank(1, &cells).unwrap());
        // A new row's unset cell is its default already.
        assert_eq!(set_in(&copy, 2), None);
        assert_eq!(set_in(&copy, 4), Some(&NewValue::Now));
        assert_eq!(set_in(&copy, 1), Some(&text_of("9100000000000000007")));
        let mut states = copy.cells.iter().map(|(_, cell)| &cell.state);
        assert!(states.all(|state| *state == State::Ready));
    }

    #[test]
    fn bytes_and_a_text_no_editor_opens_are_not_copied() {
        let (mut structure, mut page) = covers();
        structure.columns.push(column("thumbnail", "BLOB"));
        page.columns.push(meta("thumbnail", "BLOB"));
        for row in &mut page.rows {
            row.push(Value::Bytes(vec![1, 2, 3].into()));
        }
        // Bytes in a column of another type: SQLite keeps what it is given.
        page.rows[1][2] = Value::Bytes(vec![4].into());
        // Longer than any editor opens.
        let long = "x".repeat(MAX_EDIT_BYTES + 1);
        page.rows[1][4] = text(&long);
        // Pending, such a text is no more a value to copy than loaded.
        let table = table(Some(&structure), &page);
        let mut cells = BTreeMap::new();
        cells.insert((1, 1), pending(NewValue::Text(long.clone())));
        let over = table.duplicate(&table.yank(1, &cells).unwrap());
        assert!(over.made.left.contains(&("publisher_id".to_owned(), Left::TooLarge)));
        let copy = copy_of(&structure, &page);
        assert_eq!(
            copy.made.left,
            [
                ("kind".to_owned(), Left::Binary),
                ("created_at".to_owned(), Left::TooLarge),
                ("thumbnail".to_owned(), Left::Binary),
            ]
        );
        assert_eq!(copy.made.copied, ["publisher_id", "image_data"]);
        for col in [2, 4, 5] {
            assert_eq!(set_in(&copy, col), None, "column {col}");
        }
        // A column the structure does not list is the database's to fill.
        let (structure, mut page) = covers();
        page.columns.push(meta("added_since", "TEXT"));
        for row in &mut page.rows {
            row.push(text("x"));
        }
        let copy = copy_of(&structure, &page);
        assert_eq!(copy.made.left, [("added_since".to_owned(), Left::Unknown)]);
        assert_eq!(set_in(&copy, 5), None);
    }

    #[test]
    fn a_copied_value_its_column_does_not_take_is_to_fix() {
        let (structure, mut page) = covers();
        // Not one of the values `kind` lists.
        page.rows[1][2] = text("vinyl");
        let copy = copy_of(&structure, &page);
        let mut cells = copy.cells.iter();
        let (_, cell) = cells.find(|(col, _)| *col == 2).expect("kind is copied");
        assert!(matches!(cell.state, State::ToFix(_)));
    }

    #[test]
    fn a_copy_becomes_a_new_row_with_its_cells_and_what_it_was_made_as() {
        let (mut structure, page) = covers();
        let index = unique_index("book_covers_publisher_id_key", &["publisher_id"]);
        structure.indexes.push(index);
        let mut edits = Edits::default();
        let id = edits.add_copy(Place::Below(1), 3, copy_of(&structure, &page));
        assert_eq!(edits.added.len(), 1);
        assert_eq!(edits.added[0].before, 2);
        let row = new_row(id);
        let set: Vec<(usize, usize)> = edits.cells.keys().copied().collect();
        assert_eq!(set, [(row, 2), (row, 4)]);
        let new = &edits.added[0];
        assert!(new.cleared_unique("publisher_id"));
        // `id` was cleared as the database's own, which asks nothing, and
        // `kind` was not cleared.
        assert!(!new.cleared_unique("id") && !new.cleared_unique("kind"));
        // An added row was made as no copy.
        let other = edits.add_row(Place::Top, 3);
        let added = edits.added.iter().find(|new| new.id == other).unwrap();
        assert!(added.made.is_none() && !added.cleared_unique("publisher_id"));
        // The first cell a save needs a value in: `publisher_id`.
        let table = Table {
            added: &edits.added,
            ..table(Some(&structure), &page)
        };
        assert_eq!(table.first_to_fill(id, &edits.cells), Some(1));
    }

    #[test]
    fn a_cell_that_takes_no_value_is_not_the_first_to_fill() {
        let (mut structure, mut page) = covers();
        // Bytes a save needs: no cell of a new row takes them.
        structure.columns.push(ColumnInfo {
            nullable: false,
            ..column("thumbnail", "BLOB")
        });
        page.columns.push(meta("thumbnail", "BLOB"));
        for row in &mut page.rows {
            row.push(Value::Bytes(vec![1].into()));
        }
        let mut edits = Edits::default();
        let id = edits.add_copy(Place::Top, 3, copy_of(&structure, &page));
        let table = Table {
            added: &edits.added,
            ..table(Some(&structure), &page)
        };
        assert_eq!(table.missing(id, &edits.cells), [5]);
        assert_eq!(table.first_to_fill(id, &edits.cells), None);
    }
```

- [ ] **Step 2: Run them to see them fail.**

Run: `~/.cargo/bin/cargo test --locked --lib a_yank_holds`
Expected: it does not compile: `Duplicate`, `Kept`, `Cleared`, `Left`, `Table::yank`, `Table::duplicate`, `Edits::add_copy`, `NewRow::made` are not there.

- [ ] **Step 3: Give a new row what it was made as.** In `src/edit.rs`, `NewRow` gains a field and a method. The struct becomes:

```rust
#[derive(Debug, Clone, PartialEq)]
pub struct NewRow {
    /// Its own among the tab's new rows while the tab holds any.
    pub id: usize,
    /// The page's row it stands before: as many as the page has rows, for
    /// one after the last.
    pub before: usize,
    /// What its `INSERT` failed with in the last save. It is sent again by
    /// the next.
    pub failed: Option<Error>,
    /// What it was made as, where it is a copy of another row. It says
    /// what the copy was, and does not change as the row is filled in.
    pub made: Option<Copied>,
}

impl NewRow {
    /// Whether the row is a copy that cleared `column` for holding what no
    /// other row may: the column is alone in a unique index, or of the
    /// primary key. A save that needs a value there says so.
    pub fn cleared_unique(&self, column: &str) -> bool {
        self.made.as_ref().is_some_and(|made| {
            let mut cleared = made.cleared.iter();
            cleared.any(|(name, why)| {
                name == column && matches!(why, Cleared::Unique | Cleared::PrimaryKey)
            })
        })
    }
}
```

In `Edits::add_row`, the literal `NewRow { id, before, failed: None }` gains `made: None`.

- [ ] **Step 4: Add the yank, the reasons and the copy.** In `src/edit.rs`, right after `impl NewRow`:

```rust
/// What a row shows in one column, as a copy of the row takes it.
#[derive(Clone, PartialEq)]
pub enum Kept {
    /// What the page loaded there.
    Loaded(Value),
    /// What is pending there.
    Pending(NewValue),
}

/// A row's values by its columns' names: what `yy` keeps on its tab, and
/// what a copy of the row is made from. By name, since the page it was
/// taken from can be another by the time it is put: loaded again, sorted,
/// or with a column more.
#[derive(Clone, PartialEq, Default)]
pub struct Yank {
    pub cells: Vec<(String, Kept)>,
}

/// Without the values: a loaded one can be megabytes, and a pending one is
/// what the user typed, which stays out of logs and panics.
impl std::fmt::Debug for Yank {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Yank {{ cells: {} }}", self.cells.len())
    }
}

/// Why a copy of a row leaves a column for the database or the user: its
/// value is the source row's own.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cleared {
    /// The database numbers it.
    Identity,
    /// The database computes it.
    Generated,
    /// Of the primary key.
    PrimaryKey,
    /// Alone in a unique index.
    Unique,
}

/// Why a copy of a row could not take a column's value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Left {
    /// The structure does not list the column, or the page names it twice.
    Unknown,
    /// Bytes: a pending value has no form for them.
    Binary,
    /// Text over `MAX_EDIT_BYTES`, which no editor opens.
    TooLarge,
}

/// What a copy of a row was made as: the page's columns by name, in the
/// page's order, each in one of the three.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Copied {
    /// The copy has there what the source has: its value, or nothing set
    /// where the source had nothing set.
    pub copied: Vec<String>,
    pub cleared: Vec<(String, Cleared)>,
    pub left: Vec<(String, Left)>,
}

/// A copy of a row, before it is a new row of the set.
pub struct Duplicate {
    /// What it sets, by the page's column.
    pub cells: Vec<(usize, Pending)>,
    pub made: Copied,
}
```

- [ ] **Step 5: Add `Table::yank`, `Table::duplicate` and `Table::first_to_fill`.** In `src/edit.rs`, inside `impl Table<'_>`, after `fields`:

```rust
    /// What the row `row` shows in each of the page's columns, by the
    /// column's name: what is pending in `cells` where anything is, else
    /// what the page loaded. A new row gives what is set in it and nothing
    /// for the rest. `None` for a row that is neither the page's nor a new
    /// row the tab holds.
    pub fn yank(&self, row: usize, cells: &BTreeMap<(usize, usize), Pending>) -> Option<Yank> {
        let loaded = match new_id(row) {
            Some(id) => {
                self.added.iter().find(|new| new.id == id)?;
                None
            }
            None => Some(self.page.rows.get(row)?),
        };
        let columns = self.page.columns.iter().enumerate();
        let cells = columns
            .filter_map(|(col, column)| {
                let held = match cells.get(&(row, col)) {
                    Some(pending) => Kept::Pending(pending.new.clone()),
                    None => Kept::Loaded(loaded?.get(col)?.clone()),
                };
                Some((column.name.clone(), held))
            })
            .collect();
        Some(Yank { cells })
    }

    /// Whether the column `name` is alone in a unique index over the whole
    /// of it, compared as the column compares. `IndexInfo::columns` is not
    /// asked: it is text to show, which PostgreSQL quotes. An index whose
    /// columns are not known by name (an expression, a prefix, another
    /// collation) says nothing here, and the database judges the save.
    fn unique_alone(&self, name: &str) -> bool {
        let Some(structure) = self.structure else {
            return false;
        };
        let mut unique = structure.indexes.iter().filter(|index| index.unique);
        unique.any(|index| {
            let key = index.key_columns.as_deref();
            key.is_some_and(|key| matches!(key, [only] if only == name))
        })
    }

    /// A copy of the row `yank` was taken from, for the page as it is now:
    /// what each of its columns is set to, and what the copy was made as.
    /// For each column the first of these that holds decides: the structure
    /// does not know it; the database numbers it, or computes it; it is of
    /// the primary key, or alone in a unique index; it is bytes, or text no
    /// editor opens. Every other column takes what the yank holds for it.
    pub fn duplicate(&self, yank: &Yank) -> Duplicate {
        let mut cells = Vec::new();
        let mut made = Copied::default();
        let key = self.structure.map_or(&[][..], |structure| structure.primary_key.as_slice());
        for (col, meta) in self.page.columns.iter().enumerate() {
            let name = &meta.name;
            let Some(column) = self.column(col) else {
                made.left.push((name.clone(), Left::Unknown));
                continue;
            };
            let cleared = if column.identity {
                Some(Cleared::Identity)
            } else if column.generated {
                Some(Cleared::Generated)
            } else if key.contains(name) {
                Some(Cleared::PrimaryKey)
            } else if self.unique_alone(name) {
                Some(Cleared::Unique)
            } else {
                None
            };
            if let Some(why) = cleared {
                made.cleared.push((name.clone(), why));
                continue;
            }
            let mut held = yank.cells.iter();
            let held = held.find(|(of, _)| of == name).map(|(_, held)| held);
            let class = column_class(self.dialect, &column.type_name);
            let left = match held {
                _ if class == ColumnClass::Binary => Some(Left::Binary),
                Some(Kept::Loaded(Value::Bytes(_))) => Some(Left::Binary),
                Some(Kept::Loaded(Value::Text(text))) if text.len() > MAX_EDIT_BYTES => {
                    Some(Left::TooLarge)
                }
                Some(Kept::Pending(NewValue::Text(text))) if text.len() > MAX_EDIT_BYTES => {
                    Some(Left::TooLarge)
                }
                _ => None,
            };
            if let Some(why) = left {
                made.left.push((name.clone(), why));
                continue;
            }
            made.copied.push(name.clone());
            // Whether the column, left alone, would be something else than
            // NULL: only there is a NULL set.
            let defaults = match unset(self.dialect, column) {
                Unset::Default(_) | Unset::Expression(_) => true,
                Unset::Assigned | Unset::Null | Unset::Required => false,
            };
            let new = match held {
                // Nothing was set there: nothing is.
                None => None,
                Some(Kept::Loaded(Value::Null) | Kept::Pending(NewValue::Null)) => {
                    defaults.then_some(NewValue::Null)
                }
                // A new row's unset cell is its default already.
                Some(Kept::Pending(NewValue::Default { .. })) => None,
                Some(Kept::Pending(new)) => Some(new.clone()),
                // The whole value as an editor starts with it. A document
                // is the text it loaded as: an editor lays it out, and what
                // is written here is the document as it was.
                Some(Kept::Loaded(value)) => Some(NewValue::Text(match class {
                    ColumnClass::Json => crate::ui::format::plain_text(value),
                    _ => start_text(value, class),
                })),
            };
            let Some(new) = new else {
                continue;
            };
            // Checked as a typed value is. What the source's cell failed
            // with in a save is not carried: the copy has not been sent.
            let state = match &new {
                NewValue::Text(text) => {
                    check(self.dialect, column, text).map_or(State::Ready, State::ToFix)
                }
                NewValue::Null | NewValue::Default { .. } | NewValue::Now => State::Ready,
            };
            cells.push((col, Pending { new, state }));
        }
        Duplicate { cells, made }
    }

    /// The first column, in the page's order, that the new row `id` still
    /// needs a value in and that takes one: where the editor opens on a
    /// copy. A column a save needs and no cell takes (bytes) is passed
    /// over: nothing can be typed there.
    pub fn first_to_fill(
        &self,
        id: usize,
        cells: &BTreeMap<(usize, usize), Pending>,
    ) -> Option<usize> {
        let mut missing = self.missing(id, cells).into_iter();
        missing.find(|&col| self.new_lock(col).is_none())
    }
```

The `match class` in the last arm names two of `ColumnClass`'s variants and a `_`. `ColumnClass` is not the engine's enum (`Driver`, `Dialect`), so the house rule on `_` arms does not reach it, and `start_text` itself matches it so.

- [ ] **Step 6: Add `Edits::add_copy`.** In `src/edit.rs`, inside `impl Edits`, after `add_row`:

```rust
    /// Adds `copy` as a new row at `place` of a page `height` rows long,
    /// and says its id: the row, what the copy sets in it, and what it was
    /// made as.
    pub fn add_copy(&mut self, place: Place, height: usize, copy: Duplicate) -> usize {
        let id = self.add_row(place, height);
        let row = new_row(id);
        for (col, pending) in copy.cells {
            self.cells.insert((row, col), pending);
        }
        if let Some(new) = self.added.iter_mut().find(|new| new.id == id) {
            new.made = Some(copy.made);
        }
        id
    }
```

- [ ] **Step 7: Run the tests.**

Run: `~/.cargo/bin/cargo test --locked --lib -- a_yank_holds a_copy_ a_null_is_set a_pending_default_is_left bytes_and_a_text a_copied_value a_cell_that_takes_no_value`
Expected: PASS, nine tests. If `a_copy_holds_each_value_as_its_editor_would_start_with_it` fails on `in_print`, read `column_class(Dialect::Sqlite, "BOOLEAN")` in `crates/tabletist-db/src/class.rs` and use a type name it reads as a boolean.

- [ ] **Step 8: Run the four checks.** Expected: all pass. Every other `NewRow { .. }` literal the compiler finds gains `made: None`.

- [ ] **Step 9: Commit.**

```bash
git add src/edit.rs && git commit -m "Say what a copy of a row holds, cell by cell

A copy takes what the row shows: what is pending over what was loaded.
It leaves what is the source row's own (the database's counter, a
computed column, the key, a column alone in a unique index) and what no
pending value can carry (bytes, a text no editor opens), and says which
was which. A NULL is set only where the column has a default, so the
INSERT names what it must.

Nothing makes a copy yet: this is what the reducer will ask.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 2: The reducer makes a copy, and keeps a yank

**Files:**
- Modify: `src/model.rs` (`Action`, after `AddRow` near line 352; `ObjectTab` near line 1890 and its `new` near line 2030)
- Modify: `src/app.rs` (`App::apply`, beside `Action::AddRow` near line 932)
- Modify: `src/app/editing.rs` (`dropped_under_a_prompt` near line 30; `add_row` near line 774; its `use crate::edit::{...}`)
- Modify: `src/ui/insert_audit_tests.rs` (its helpers' visibility)
- Modify: `src/ui/mod.rs` (the module list near line 20)
- Create: `src/ui/duplicate_tests.rs`

- [ ] **Step 1: Let the new tests borrow the audit's helpers.** In `src/ui/insert_audit_tests.rs` these `fn`s become `pub(super) fn`, and nothing else of them changes: `covers_in`, `covers_as`, `edits`, `selected`, `painted`, `select`, `type_key`, `set`, `add`, `reviewed`.

- [ ] **Step 2: Write the failing tests.** In `src/ui/mod.rs`, in the list of modules, after `pub mod data_view;` and before the `#[cfg(test)]` of `mod env_tests;`, add:

```rust
#[cfg(test)]
mod duplicate_tests;
```

Create `src/ui/duplicate_tests.rs`:

```rust
//! Duplicating a row, checked against its spec
//! (`docs/superpowers/specs/2026-10-10-row-duplication-design.md`: "spec
//! N" below is a section of its Behavior, "test N" an entry of its Tests).
//! Only behaviour, names and words, on the Bookshop's `book_covers`. What
//! a copy holds cell by cell is tested where it is decided, in `edit.rs`.

use tabletist_db::{IndexInfo, NewValue, ObjectKind, ObjectRef, RowPage, Structure, Value};

use super::insert_audit_tests::{add, covers_as, covers_in, edits, select, selected, set};
use crate::edit::{Cleared, Order, State, new_row};
use crate::model::{
    Action, CellPos, ConnTabId, EditStart, ObjectView, Pane, SaveBlock, TabId,
};
use crate::testing::Harness;
use crate::theme::Look;

/// The Bookshop's covers, writable, the grid holding the keyboard, their
/// structure and their page changed by `vary` before they load.
fn covers(
    look: Look,
    vary: impl FnOnce(&mut Structure, &mut RowPage),
) -> (Harness, ConnTabId, TabId) {
    let mut harness = Harness::new();
    harness.set_look(look);
    let tab = harness.connect_fake_as(false);
    harness.app.apply(Action::OpenObject {
        tab,
        object: ObjectRef::new("main", "book_covers"),
        kind: ObjectKind::Table,
        pin: true,
    });
    let mut structure = crate::testing::book_covers_structure();
    let mut page = crate::testing::book_covers_page(3);
    vary(&mut structure, &mut page);
    harness.answer_structure(structure);
    harness.answer_rows(page);
    let id = harness.app.workspace(tab).unwrap().active_tab.unwrap();
    harness.app.workspace_mut(tab).unwrap().pane = Pane::Grid;
    harness.settle();
    (harness, tab, id)
}

/// A unique index on `publisher_id` alone, as the Bookshop has it.
fn unique_publisher(structure: &mut Structure, _: &mut RowPage) {
    structure.indexes.push(IndexInfo {
        name: "book_covers_publisher_id_key".into(),
        columns: vec!["publisher_id".into()],
        key_columns: Some(vec!["publisher_id".into()]),
        unique: true,
        ..IndexInfo::default()
    });
}

/// What is set in the cell at `at` (row, column), pending.
fn value(harness: &Harness, tab: ConnTabId, id: TabId, at: (usize, usize)) -> Option<NewValue> {
    let cells = &edits(harness, tab, id).cells;
    cells.get(&at).map(|cell| cell.new.clone())
}

fn text(value: &str) -> Option<NewValue> {
    Some(NewValue::Text(value.into()))
}

/// The rows the grid shows, in its order: the page's three and the new
/// ones among them.
fn shown(harness: &Harness, tab: ConnTabId, id: TabId) -> Vec<usize> {
    let order = Order::of(&edits(harness, tab, id).added, 3);
    (0..order.len())
        .filter_map(|place| order.row(place))
        .collect()
}

/// The row of the newest new row.
fn newest(harness: &Harness, tab: ConnTabId, id: TabId) -> usize {
    let added = edits(harness, tab, id).added.iter();
    new_row(added.map(|new| new.id).max().expect("a new row"))
}

fn duplicate(harness: &mut Harness, tab: ConnTabId, id: TabId) {
    harness.app.apply(Action::DuplicateRow { tab, id });
}

fn put(harness: &mut Harness, tab: ConnTabId, id: TabId, above: bool) {
    harness.app.apply(Action::PutRow { tab, id, above });
}

// The reducer: what a copy is, and where it goes.

/// Test 3, the insert spec's test 5: a copy leaves what the database
/// numbers and what no other row may hold, keeps the rest, and opens the
/// editor on the first cell a save needs.
#[test]
fn a_copy_clears_what_it_must_and_opens_the_first_cell_it_needs() {
    let (mut harness, tab, id) = covers(Look::macos(), unique_publisher);
    select(&mut harness, tab, id, (1, 2));
    duplicate(&mut harness, tab, id);
    assert_eq!(edits(&harness, tab, id).added.len(), 1);
    let row = newest(&harness, tab, id);
    assert_eq!(value(&harness, tab, id, (row, 0)), None);
    assert_eq!(value(&harness, tab, id, (row, 1)), None);
    assert_eq!(value(&harness, tab, id, (row, 2)), text("ebook"));
    assert_eq!(value(&harness, tab, id, (row, 3)), None);
    let created = text("2026-01-12 09:14:03");
    assert_eq!(value(&harness, tab, id, (row, 4)), created);
    let now = edits(&harness, tab, id);
    let made = now.added[0].made.as_ref().expect("made as a copy");
    assert_eq!(
        made.cleared,
        [
            ("id".to_owned(), Cleared::Identity),
            ("publisher_id".to_owned(), Cleared::Unique)
        ]
    );
    // `publisher_id`: the one cell a save needs a value in.
    let editor = now.editor.as_ref().expect("an editor");
    assert_eq!(editor.cell, CellPos { row, col: 1 });
    assert_eq!(selected(&harness, tab, id), Some((row, 1)));
    // At the top, as an added row is.
    assert_eq!(shown(&harness, tab, id), [row, 0, 1, 2]);
    // Nothing of the source changed.
    assert_eq!(now.cells.keys().filter(|(of, _)| *of == 1).count(), 0);
}

/// Test 10: where nothing is asked of the copy the selection goes to it,
/// in the column the cursor was in, and no editor opens.
#[test]
fn a_copy_nothing_is_asked_of_takes_the_selection_in_the_cursors_column() {
    let (mut harness, tab, id) = covers_in(Look::macos());
    select(&mut harness, tab, id, (1, 2));
    duplicate(&mut harness, tab, id);
    let row = newest(&harness, tab, id);
    // No index holds `publisher_id` unique here: it is copied.
    let publisher = text("9100000000000000002");
    assert_eq!(value(&harness, tab, id, (row, 1)), publisher);
    assert!(edits(&harness, tab, id).editor.is_none());
    assert_eq!(selected(&harness, tab, id), Some((row, 2)));
}

/// Test 4: the copy holds what is pending in the source, the text of an
/// editor open on it too, and a pending DEFAULT leaves its cell unset.
#[test]
fn a_copy_takes_what_is_pending_and_what_is_being_typed() {
    let (mut harness, tab, id) = covers_in(Look::macos());
    set(&mut harness, tab, id, (1, 2), "audio");
    // An editor left open on the source, with a publisher typed into it.
    let other = "9100000000000000007";
    harness.app.apply(Action::EditCell {
        tab,
        id,
        cell: CellPos { row: 1, col: 1 },
        start: EditStart::Replace(other.into()),
    });
    duplicate(&mut harness, tab, id);
    let row = newest(&harness, tab, id);
    assert_eq!(value(&harness, tab, id, (row, 1)), text(other));
    assert_eq!(value(&harness, tab, id, (row, 2)), text("audio"));
    // The source keeps both, pending.
    assert_eq!(value(&harness, tab, id, (1, 1)), text(other));
    assert_eq!(value(&harness, tab, id, (1, 2)), text("audio"));
    // A pending DEFAULT: the copy's unset cell is its default already.
    let (mut harness, tab, id) = covers_in(Look::macos());
    select(&mut harness, tab, id, (1, 2));
    harness.app.apply(Action::SetDefault { tab, id });
    let pending = value(&harness, tab, id, (1, 2));
    assert!(matches!(pending, Some(NewValue::Default { .. })));
    duplicate(&mut harness, tab, id);
    let row = newest(&harness, tab, id);
    assert_eq!(value(&harness, tab, id, (row, 2)), None);
}

/// Test 6: a new row is copied with what is set in it, and its unset cells
/// stay unset.
#[test]
fn a_new_row_is_copied_with_what_is_set_in_it() {
    let (mut harness, tab, id) = covers_in(Look::macos());
    let first = new_row(add(&mut harness, tab, id));
    set(&mut harness, tab, id, (first, 2), "audio");
    select(&mut harness, tab, id, (first, 2));
    duplicate(&mut harness, tab, id);
    assert_eq!(edits(&harness, tab, id).added.len(), 2);
    let copy = newest(&harness, tab, id);
    assert_ne!(copy, first);
    assert_eq!(value(&harness, tab, id, (copy, 2)), text("audio"));
    assert_eq!(value(&harness, tab, id, (copy, 1)), None);
    // Under the new rows already at the top.
    assert_eq!(shown(&harness, tab, id), [first, copy, 0, 1, 2]);
    // The source set no publisher, so the copy needs one.
    let editor = edits(&harness, tab, id).editor.as_ref().expect("an editor");
    assert_eq!(editor.cell, CellPos { row: copy, col: 1 });
}

/// Test 2 and spec 2: a yanked row is put below the cursor's row, or
/// above it, wherever the cursor has gone since.
#[test]
fn a_yanked_row_is_put_below_the_cursors_row_or_above_it() {
    let (mut harness, tab, id) = covers_in(Look::omarchy());
    select(&mut harness, tab, id, (1, 2));
    harness.app.apply(Action::YankRow { tab, id });
    // The yank adds nothing.
    assert!(edits(&harness, tab, id).added.is_empty());
    select(&mut harness, tab, id, (0, 2));
    put(&mut harness, tab, id, false);
    let below = newest(&harness, tab, id);
    assert_eq!(shown(&harness, tab, id), [0, below, 1, 2]);
    select(&mut harness, tab, id, (2, 2));
    put(&mut harness, tab, id, true);
    let above = newest(&harness, tab, id);
    assert_eq!(shown(&harness, tab, id), [0, below, 1, above, 2]);
    // Each is the yanked row, an `ebook`, not the `print` the cursor was on.
    for row in [below, above] {
        assert_eq!(value(&harness, tab, id, (row, 2)), text("ebook"));
    }
    // And the cursor is on the copy, in the column it was in.
    assert_eq!(selected(&harness, tab, id), Some((above, 2)));
}

/// Test 11: a yank outlives a reload of the page and a discard, and is put
/// as often as it is asked for. Nothing is put before one.
#[test]
fn a_yank_outlives_a_reload_and_a_discard_and_is_put_again() {
    let (mut harness, tab, id) = covers_in(Look::omarchy());
    select(&mut harness, tab, id, (0, 2));
    put(&mut harness, tab, id, false);
    assert!(edits(&harness, tab, id).added.is_empty(), "nothing yanked");
    select(&mut harness, tab, id, (1, 2));
    harness.app.apply(Action::YankRow { tab, id });
    // The page comes back with every cover an `audio`.
    let mut other = crate::testing::book_covers_page(3);
    for row in &mut other.rows {
        row[2] = Value::Text("audio".into());
    }
    harness.app.apply(Action::Refresh(tab));
    harness.answer_rows(other);
    let object = |harness: &Harness| {
        let workspace = harness.app.workspace(tab).unwrap();
        workspace.object_tab(id).unwrap().structure.is_loading()
    };
    if object(&harness) {
        harness.answer_structure(crate::testing::book_covers_structure());
    }
    harness.app.apply(Action::DiscardEdits { tab, id });
    select(&mut harness, tab, id, (0, 2));
    put(&mut harness, tab, id, false);
    let first = newest(&harness, tab, id);
    put(&mut harness, tab, id, false);
    let second = newest(&harness, tab, id);
    assert_eq!(edits(&harness, tab, id).added.len(), 2);
    for row in [first, second] {
        assert_eq!(value(&harness, tab, id, (row, 2)), text("ebook"));
    }
}

/// Test 13: no copy is made where no row is added: a connection that
/// opens read-only, a view, a save in flight, and the Structure view.
#[test]
fn no_copy_is_made_where_no_row_is_taken() {
    let all = |harness: &mut Harness, tab: ConnTabId, id: TabId| {
        harness.app.apply(Action::YankRow { tab, id });
        duplicate(harness, tab, id);
        put(harness, tab, id, false);
        put(harness, tab, id, true);
    };
    for (kind, read_only) in [(ObjectKind::Table, true), (ObjectKind::View, false)] {
        let (mut harness, tab, id) = covers_as(Look::macos(), kind, read_only);
        select(&mut harness, tab, id, (1, 2));
        all(&mut harness, tab, id);
        assert!(edits(&harness, tab, id).added.is_empty(), "{kind:?}");
    }
    // A save in flight: its answer names the new rows it sent.
    let (mut harness, tab, id) = covers_in(Look::macos());
    set(&mut harness, tab, id, (0, 2), "audio");
    select(&mut harness, tab, id, (1, 2));
    harness.app.apply(Action::WriteEdits { tab, id });
    assert!(edits(&harness, tab, id).saving.is_some());
    all(&mut harness, tab, id);
    assert!(edits(&harness, tab, id).added.is_empty(), "saving");
    // The Structure view shows no row to copy.
    let (mut harness, tab, id) = covers_in(Look::macos());
    select(&mut harness, tab, id, (1, 2));
    harness.app.apply(Action::SetView {
        tab,
        object_tab: id,
        view: ObjectView::Structure,
    });
    all(&mut harness, tab, id);
    assert!(edits(&harness, tab, id).added.is_empty(), "structure");
}

/// Test 13: a table without a key takes a copy, as it takes a new row.
#[test]
fn a_table_without_a_key_takes_a_copy() {
    let keyless = |structure: &mut Structure, _: &mut RowPage| structure.primary_key.clear();
    let (mut harness, tab, id) = covers(Look::macos(), keyless);
    select(&mut harness, tab, id, (1, 2));
    duplicate(&mut harness, tab, id);
    let now = edits(&harness, tab, id);
    assert_eq!(now.added.len(), 1);
    // `id` is still the database's to number.
    let made = now.added[0].made.as_ref().unwrap();
    assert_eq!(made.cleared, [("id".to_owned(), Cleared::Identity)]);
}

/// Test 9: a copied value its column does not take is to fix, and a save
/// waits for it.
#[test]
fn a_save_waits_for_a_copied_value_its_column_does_not_take() {
    // Not one of the values `kind` lists.
    let odd = |_: &mut Structure, page: &mut RowPage| {
        page.rows[1][2] = Value::Text("vinyl".into());
    };
    let (mut harness, tab, id) = covers(Look::macos(), odd);
    select(&mut harness, tab, id, (1, 2));
    duplicate(&mut harness, tab, id);
    let row = newest(&harness, tab, id);
    let cell = &edits(&harness, tab, id).cells[&(row, 2)];
    assert!(matches!(cell.state, State::ToFix(_)));
    let blocked = harness.app.save_blocked(tab, id);
    assert!(matches!(blocked, Some(SaveBlock::ToFix)));
}

/// Test 17: Review SQL shows the copy as one `INSERT`, naming the columns
/// it holds and no other.
#[test]
fn review_shows_the_copy_as_one_insert_of_what_it_holds() {
    use super::insert_audit_tests::reviewed;
    let (mut harness, tab, id) = covers_in(Look::macos());
    select(&mut harness, tab, id, (1, 2));
    duplicate(&mut harness, tab, id);
    harness.click("Review SQL");
    harness.settle();
    let (statements, comments) = reviewed(&harness, tab, id);
    assert_eq!((statements.len(), comments), (1, 1), "{statements:?}");
    let named = r#"INSERT INTO "main"."book_covers" ("publisher_id", "kind", "created_at") VALUES ("#;
    assert!(statements[0].starts_with(named), "{statements:?}");
}
```

- [ ] **Step 3: Run them to see them fail.**

Run: `~/.cargo/bin/cargo test --locked --lib ui::duplicate_tests`
Expected: it does not compile: `Action::DuplicateRow`, `Action::YankRow` and `Action::PutRow` are not there.

- [ ] **Step 4: Add the actions and the tab's yank.** In `src/model.rs`, in `enum Action` after `AddRow { .. }`:

```rust
    /// Add a copy of the selected row as a new row at the top, below the
    /// new rows already there: what is pending in the row is what the copy
    /// holds, and an open editor's text with it.
    DuplicateRow {
        tab: ConnTabId,
        id: TabId,
    },
    /// Keep what the selected row shows on the tab, for `PutRow`.
    YankRow {
        tab: ConnTabId,
        id: TabId,
    },
    /// Add a copy of the tab's yanked row right below the selected row, or
    /// right above it. With nothing yanked, nothing.
    PutRow {
        tab: ConnTabId,
        id: TabId,
        above: bool,
    },
```

In `struct ObjectTab`, after `edits`:

```rust
    /// The row the terminal look's `yy` took, for `p` and `P` to put: its
    /// values by column. It is the tab's own and goes with the tab. A
    /// reload, a save and a discard leave it.
    pub yanked: Option<crate::edit::Yank>,
```

and in `ObjectTab::new`, after `edits: crate::edit::Edits::default(),`:

```rust
            yanked: None,
```

- [ ] **Step 5: Apply them.** In `src/app.rs`, in `App::apply`, after the arm of `Action::AddRow`:

```rust
            Action::DuplicateRow { tab, id } => self.duplicate_row(tab, id),
            Action::YankRow { tab, id } => self.yank_row(tab, id),
            Action::PutRow { tab, id, above } => self.put_row(tab, id, above),
```

In `src/app/editing.rs`, in `dropped_under_a_prompt`, after `| Action::AddRow { .. }`:

```rust
            | Action::DuplicateRow { .. }
            | Action::PutRow { .. }
```

- [ ] **Step 6: Split `add_row`, and add the three.** In `src/app/editing.rs`, `Yank` joins the names taken from `crate::edit`. Replace the whole of `add_row` (its doc comment too) with:

```rust
    /// Whether the tab's table takes a new row now: its rows show, and
    /// nothing stands against a row (`Table::no_rows`).
    fn takes_rows(&self, tab: ConnTabId, id: TabId) -> bool {
        let takes = self.table(tab, id, |table, object| {
            object.view == ObjectView::Data && table.no_rows().is_none()
        });
        takes == Some(true)
    }

    /// What follows a new row put into the set, for its cell `cell`: the
    /// editor on the cell where it is to `open`, else the selection there
    /// and the keys with the grid.
    fn settle_new_row(&mut self, tab: ConnTabId, id: TabId, cell: CellPos, open: bool) {
        if let Some(object) = self.object_tab_mut(tab, id) {
            object.edits.why = None;
            object.fields = None;
            // A tab with a new row is no preview to replace.
            object.pinned = true;
        }
        if open {
            self.edit_cell(tab, id, cell, EditStart::Value, EditorPlace::Grid);
            return;
        }
        if let Some(object) = self.object_tab_mut(tab, id) {
            object.selection = Some(cell);
            object.focus_field = None;
        }
        if let Some(workspace) = self.workspace_mut(tab) {
            workspace.pane = Pane::Grid;
            workspace.save_refused = false;
            workspace.review_refused = false;
        }
    }

    /// Adds a new row to the tab's table at `place`, where it takes one,
    /// and opens the editor on the first cell a save needs a value in.
    /// With no such cell the selection goes to the first that takes a
    /// value, and nothing opens: nothing is asked of the user there.
    pub(super) fn add_row(&mut self, tab: ConnTabId, id: TabId, place: Place) {
        // An editor open on another cell keeps its text.
        self.close_editor(tab, id, true);
        if !self.takes_rows(tab, id) {
            return;
        }
        let Some(object) = self.object_tab_mut(tab, id) else {
            return;
        };
        let height = object.page().map_or(0, |page| page.rows.len());
        let new = object.edits.add_row(place, height);
        let row = new_row(new);
        let found = self.table(tab, id, |table, object| {
            let required = table.missing(new, &object.edits.cells).into_iter().next();
            let free = || {
                let mut cols = 0..table.page.columns.len();
                cols.find(|&col| table.lock(CellPos { row, col }).is_none())
            };
            (required, required.or_else(free))
        });
        let (required, col) = found.unwrap_or((None, None));
        let cell = CellPos {
            row,
            col: col.unwrap_or(0),
        };
        self.settle_new_row(tab, id, cell, required.is_some());
    }

    /// Adds a copy of the selected row as a new row at the top.
    pub(super) fn duplicate_row(&mut self, tab: ConnTabId, id: TabId) {
        // Before the row is read: an open editor's text is part of what
        // the row shows, and closed it is pending.
        self.close_editor(tab, id, true);
        let yank = self.table(tab, id, |table, object| {
            table.yank(object.selection?.row, &object.edits.cells)
        });
        if let Some(Some(yank)) = yank {
            self.put_copy(tab, id, &yank, Place::Top);
        }
    }

    /// Keeps what the selected row shows on the tab, where its rows show.
    pub(super) fn yank_row(&mut self, tab: ConnTabId, id: TabId) {
        let yank = self.table(tab, id, |table, object| {
            if object.view != ObjectView::Data {
                return None;
            }
            table.yank(object.selection?.row, &object.edits.cells)
        });
        if let (Some(Some(yank)), Some(object)) = (yank, self.object_tab_mut(tab, id)) {
            object.yanked = Some(yank);
        }
    }

    /// Adds a copy of the tab's yanked row right below the selected row,
    /// or right above it. With no row selected, at the top.
    pub(super) fn put_row(&mut self, tab: ConnTabId, id: TabId, above: bool) {
        self.close_editor(tab, id, true);
        let object = self
            .workspace(tab)
            .and_then(|workspace| workspace.object_tab(id));
        let Some((yank, selection)) = object.and_then(|object| {
            let yank = object.yanked.clone()?;
            Some((yank, object.selection))
        }) else {
            return;
        };
        let place = match (selection, above) {
            (Some(cell), false) => Place::Below(cell.row),
            (Some(cell), true) => Place::Above(cell.row),
            (None, _) => Place::Top,
        };
        self.put_copy(tab, id, &yank, place);
    }

    /// Makes a new row at `place` of what `yank` holds, where the table
    /// takes one. The editor opens on the first cell a save needs a value
    /// in and that takes one. With none the selection goes to the copy, in
    /// the column the cursor was in. Both are the copy's own rules: an
    /// added row opens on its first required cell whatever it is, and
    /// otherwise selects the first cell that takes a value.
    fn put_copy(&mut self, tab: ConnTabId, id: TabId, yank: &Yank, place: Place) {
        if !self.takes_rows(tab, id) {
            return;
        }
        let Some(copy) = self.table(tab, id, |table, _| table.duplicate(yank)) else {
            return;
        };
        let Some(object) = self.object_tab_mut(tab, id) else {
            return;
        };
        let stood = object.selection.map_or(0, |cell| cell.col);
        let height = object.page().map_or(0, |page| page.rows.len());
        let new = object.edits.add_copy(place, height, copy);
        let row = new_row(new);
        let open = self.table(tab, id, |table, object| {
            table.first_to_fill(new, &object.edits.cells)
        });
        let open = open.flatten();
        let cell = CellPos {
            row,
            col: open.unwrap_or(stood),
        };
        self.settle_new_row(tab, id, cell, open.is_some());
    }
```

- [ ] **Step 7: Run the tests.**

Run: `~/.cargo/bin/cargo test --locked --lib ui::duplicate_tests`
Expected: PASS, ten tests.

Run: `~/.cargo/bin/cargo test --locked --lib -- adding_a_row a_row_nothing_is_required_of`
Expected: PASS: Add row behaves as it did.

- [ ] **Step 8: Run the four checks.** Expected: all pass.

- [ ] **Step 9: Commit.**

```bash
git add src/model.rs src/app.rs src/app/editing.rs src/ui/mod.rs src/ui/insert_audit_tests.rs src/ui/duplicate_tests.rs && git commit -m "Make a copy of a row a new row, and keep a yanked row on its tab

Three actions: one copies the selected row to the top, one keeps what
the row shows on the tab, one puts that below the cursor's row or above
it. A copy is refused where a new row is. An open editor is closed
before the row is read, so what is being typed is in the copy.

What follows a new row in the set (the editor or the selection, the
pane) is one function now, told which cell. Add row asks it for what it
always did. A copy asks for the first cell a save needs and that takes
a value, and otherwise for the column the cursor was in.

No key and no button asks for a copy yet.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 3: Mod+D in the desktop looks

**Files:**
- Modify: `src/ui/keys.rs` (`adds` near line 200; the Mod+N block near line 370; beside `take_press` near line 880)
- Modify: `src/ui/insert_audit_tests.rs` (INS-02a near line 191)
- Modify: `src/ui/duplicate_tests.rs`

- [ ] **Step 1: Write the failing tests.** In `src/ui/insert_audit_tests.rs`, delete the line `#[ignore = "INS-02a: Mod+D does not duplicate a row yet"]` above `mod_d_duplicates_the_selected_row`.

In `src/ui/duplicate_tests.rs`, the first `use` lines gain `egui::{Key, Modifiers}`:

```rust
use egui::{Key, Modifiers};
```

and at the end of the file:

```rust
// The desktop looks' key.

/// Test 1: a held Mod+D makes one copy. Its repeats are no presses.
#[test]
fn mod_d_makes_one_copy_however_long_it_is_held() {
    let (mut harness, tab, id) = covers_in(Look::macos());
    select(&mut harness, tab, id, (1, 2));
    let repeat = egui::Event::Key {
        key: Key::D,
        physical_key: None,
        pressed: true,
        repeat: true,
        modifiers: Modifiers::COMMAND,
    };
    harness.frame(vec![crate::testing::key(Key::D, Modifiers::COMMAND)]);
    for _ in 0..3 {
        harness.frame(vec![repeat.clone()]);
    }
    harness.frame(vec![crate::testing::release(Key::D, Modifiers::COMMAND)]);
    harness.settle();
    assert_eq!(edits(&harness, tab, id).added.len(), 1);
    // A second press is a second copy, of the copy the cursor is on.
    harness.press(Key::D, Modifiers::COMMAND);
    assert_eq!(edits(&harness, tab, id).added.len(), 2);
}

/// Test 19: Mod+Shift+D is Review SQL still, and makes no copy.
#[test]
fn mod_shift_d_is_review_sql_and_makes_no_copy() {
    let (mut harness, tab, id) = covers_in(Look::macos());
    set(&mut harness, tab, id, (1, 2), "audio");
    select(&mut harness, tab, id, (1, 2));
    harness.press(Key::D, Modifiers::COMMAND | Modifiers::SHIFT);
    assert!(edits(&harness, tab, id).reviewing);
    assert!(edits(&harness, tab, id).added.is_empty());
}

/// Spec 1: the key is the desktop looks', and copies the selected row:
/// with none selected it does nothing.
#[test]
fn mod_d_needs_a_row_and_is_not_the_terminals_key() {
    let (mut harness, tab, id) = covers_in(Look::omarchy());
    select(&mut harness, tab, id, (1, 2));
    harness.press(Key::D, Modifiers::COMMAND);
    assert!(edits(&harness, tab, id).added.is_empty(), "the terminal");
    let (mut harness, tab, id) = covers_in(Look::macos());
    assert_eq!(selected(&harness, tab, id), None);
    harness.press(Key::D, Modifiers::COMMAND);
    assert!(edits(&harness, tab, id).added.is_empty(), "no row");
    // Nor where no row is taken: a connection that opens read-only.
    let (mut harness, tab, id) = covers_as(Look::macos(), ObjectKind::Table, true);
    select(&mut harness, tab, id, (1, 2));
    harness.press(Key::D, Modifiers::COMMAND);
    assert!(edits(&harness, tab, id).added.is_empty(), "read-only");
}

/// Test 21: like Mod+N, the key does not ask what has the keyboard.
#[test]
fn mod_d_copies_with_the_keyboard_in_the_filters_field() {
    let (mut harness, tab, id) = covers_in(Look::macos());
    select(&mut harness, tab, id, (1, 2));
    harness.press(Key::F, Modifiers::COMMAND);
    assert!(harness.ctx.text_edit_focused(), "the filter's field");
    harness.press(Key::D, Modifiers::COMMAND);
    assert_eq!(edits(&harness, tab, id).added.len(), 1);
}
```

- [ ] **Step 2: Run them to see them fail.**

Run: `~/.cargo/bin/cargo test --locked --lib -- mod_d_ mod_shift_d_`
Expected: `mod_d_duplicates_the_selected_row`, `mod_d_makes_one_copy_however_long_it_is_held` and `mod_d_copies_with_the_keyboard_in_the_filters_field` FAIL with 0 new rows. The other two pass already: they hold what must stay.

- [ ] **Step 3: Read the key.** In `src/ui/keys.rs`, after `take_press`:

```rust
/// Whether `key` went down afresh this frame with exactly `modifiers` (see
/// [`is_press`]). Takes the press, and the repeats of a held key with it,
/// which are no press: what one press does, a held key does once.
fn take_fresh_press(input: &mut egui::InputState, modifiers: Modifiers, key: Key) -> bool {
    let mut fresh = false;
    input.events.retain(|event| {
        if !is_press(event, modifiers, key) {
            return true;
        }
        fresh |= !matches!(event, egui::Event::Key { repeat: true, .. });
        false
    });
    fresh
}
```

After the definition of `adds` (the `let adds = object.filter(...)...;` near line 200):

```rust
    // Mod+D makes a copy of the selected row wherever Mod+N adds a row.
    // Like that key it does not ask what has the keyboard.
    let copies = adds.filter(|&(tab, id)| {
        app.workspace(tab)
            .and_then(|workspace| workspace.object_tab(id))
            .is_some_and(|object| object.selection.is_some())
    });
```

After the block that reads Mod+N (`if let Some((tab, id)) = adds && consume_press(input, Modifiers::COMMAND, Key::N) { ... }`):

```rust
        // With its modifiers exactly: `consume_press` lets an extra Shift
        // through, and Mod+Shift+D is Review SQL, read further down.
        if let Some((tab, id)) = copies
            && take_fresh_press(input, Modifiers::COMMAND, Key::D)
        {
            actions.push(Action::DuplicateRow { tab, id });
        }
```

- [ ] **Step 4: Run the tests.**

Run: `~/.cargo/bin/cargo test --locked --lib -- mod_d_ mod_shift_d_`
Expected: PASS, five tests.

- [ ] **Step 5: Run the four checks.** Expected: all pass.

- [ ] **Step 6: Commit.**

```bash
git add src/ui/keys.rs src/ui/insert_audit_tests.rs src/ui/duplicate_tests.rs && git commit -m "Copy the selected row with Mod+D

Read where Mod+N adds a row, in the looks whose key that is, with a row
selected. It is matched with its modifiers exactly: the way Mod+N is
matched lets an extra Shift through, and Mod+Shift+D is Review SQL. Only
a fresh press makes a copy, and a held key's repeats are taken with it.

The audit's INS-02a is out of ignore.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 4: `yy`, `p` and `P` in the Omarchy look

**Files:**
- Modify: `src/ui/keys.rs` (`editing_letters` near lines 1100 to 1255; `letters`, the cell's `y` near line 1604)
- Modify: `src/ui/insert_audit_tests.rs` (INS-02b near line 201)
- Modify: `src/ui/duplicate_tests.rs`

- [ ] **Step 1: Write the failing tests.** In `src/ui/insert_audit_tests.rs`, delete the line `#[ignore = "INS-02b: yy p does not duplicate a row yet"]` above `yy_p_duplicates_the_row_under_the_cursor`.

In `src/ui/duplicate_tests.rs`, `type_key` joins the names taken from `super::insert_audit_tests`, and at the end of the file:

```rust
// The terminal look's keys.

/// The tab's yank, where it holds one.
fn yanked(harness: &Harness, tab: ConnTabId, id: TabId) -> bool {
    let workspace = harness.app.workspace(tab).unwrap();
    workspace.object_tab(id).unwrap().yanked.is_some()
}

fn yy(harness: &mut Harness) {
    type_key(harness, Key::Y, "y");
    type_key(harness, Key::Y, "y");
}

/// Test 2: `yy` then `p` puts a copy below the cursor's row, `P` above.
#[test]
fn yy_then_p_puts_a_copy_below_and_capital_p_above() {
    let (mut harness, tab, id) = covers_in(Look::omarchy());
    select(&mut harness, tab, id, (1, 2));
    yy(&mut harness);
    assert!(yanked(&harness, tab, id));
    type_key(&mut harness, Key::P, "p");
    let below = newest(&harness, tab, id);
    assert_eq!(shown(&harness, tab, id), [0, 1, below, 2]);
    select(&mut harness, tab, id, (0, 2));
    type_key(&mut harness, Key::P, "P");
    let above = newest(&harness, tab, id);
    assert_eq!(shown(&harness, tab, id), [above, 0, 1, below, 2]);
    for row in [below, above] {
        assert_eq!(value(&harness, tab, id, (row, 2)), text("ebook"));
    }
}

/// Test 12: `yy` puts the row on the system clipboard as TSV, as
/// Mod+Shift+C does, and the status line says the row was yanked.
#[test]
fn yy_puts_the_row_on_the_clipboard_and_says_so() {
    let (mut harness, tab, id) = covers_in(Look::omarchy());
    select(&mut harness, tab, id, (1, 2));
    type_key(&mut harness, Key::Y, "y");
    // The first is the cell's, as it was.
    assert_eq!(harness.copied.as_deref(), Some("ebook"));
    assert!(!yanked(&harness, tab, id));
    type_key(&mut harness, Key::Y, "y");
    let row = harness.app.copy_text(true).expect("the row");
    assert!(row.contains('\t'), "{row:?}");
    assert_eq!(harness.copied, Some(row));
    let said = crate::ui::toast::said(&harness.ctx);
    assert_eq!(said.as_deref(), Some("row yanked"));
}

/// Spec 2: a `y` that is not followed by another yanks nothing, and `p`
/// with nothing yanked adds no row.
#[test]
fn one_y_yanks_nothing_and_p_puts_nothing_then() {
    let (mut harness, tab, id) = covers_in(Look::omarchy());
    select(&mut harness, tab, id, (1, 2));
    type_key(&mut harness, Key::Y, "y");
    type_key(&mut harness, Key::J, "j");
    type_key(&mut harness, Key::Y, "y");
    assert!(!yanked(&harness, tab, id));
    type_key(&mut harness, Key::P, "p");
    assert!(edits(&harness, tab, id).added.is_empty());
    // With no row selected there is nothing to yank, and nothing is said.
    let (mut harness, tab, id) = covers_in(Look::omarchy());
    yy(&mut harness);
    assert!(!yanked(&harness, tab, id));
    assert_eq!(crate::ui::toast::said(&harness.ctx), None);
}

/// Test 21: `yy` yanks with the keys on a field of a loaded row's panel,
/// whose footer names the key.
#[test]
fn yy_yanks_from_a_field_of_the_row_panel() {
    let (mut harness, tab, id) = covers_in(Look::omarchy());
    select(&mut harness, tab, id, (1, 2));
    harness.press(Key::L, Modifiers::CTRL);
    harness.settle();
    let field = crate::ui::row_panel::focused_field(&harness.ctx, tab, id, 5);
    assert!(field.is_some(), "a field of the panel has the keyboard");
    yy(&mut harness);
    assert!(yanked(&harness, tab, id));
}

/// Test 13: on a connection that opens read-only a row is yanked, for the
/// clipboard, and `p` puts nothing.
#[test]
fn p_puts_nothing_where_no_row_is_taken() {
    let (mut harness, tab, id) = covers_as(Look::omarchy(), ObjectKind::Table, true);
    select(&mut harness, tab, id, (1, 2));
    yy(&mut harness);
    assert!(yanked(&harness, tab, id));
    type_key(&mut harness, Key::P, "p");
    assert!(edits(&harness, tab, id).added.is_empty());
}
```

- [ ] **Step 2: Run them to see them fail.**

Run: `~/.cargo/bin/cargo test --locked --lib -- yy_ one_y_yanks p_puts_nothing`
Expected: `yy_p_duplicates_the_row_under_the_cursor`, `yy_then_p_puts_a_copy_below_and_capital_p_above`, `yy_puts_the_row_on_the_clipboard_and_says_so`, `yy_yanks_from_a_field_of_the_row_panel` and `p_puts_nothing_where_no_row_is_taken` FAIL: nothing is yanked. `one_y_yanks_nothing_and_p_puts_nothing_then` passes already.

- [ ] **Step 3: Read `p` and `P`.** In `src/ui/keys.rs`, in `editing_letters`:

The doc comment's sentence "`o` and `O` open a new row below the cursor's and above it," gains, after it: "`p` and `P` put a copy of the yanked row there,".

`mine` becomes:

```rust
    let mine = |text: &str| {
        matches!(
            text,
            "i" | "c" | "x" | "u" | ":" | "o" | "O" | "D" | "p" | "P"
        )
    };
```

and in the `match text.as_str()`, after the arm of `"o" | "O"`:

```rust
                // A copy of the yanked row below the cursor's, or above it
                // with the capital. It may open an editor, as `o` does.
                "p" | "P" => {
                    opened = true;
                    let above = text == "P";
                    actions.push(Action::PutRow { tab, id, above });
                }
```

- [ ] **Step 4: Read the second `y`.** In `src/ui/keys.rs`, in `letters`, the block that begins `if pressed(Key::Y) || sql.is_some() {` gains its last lines, inside the block and after the `if let Some(text) = text { ... }`:

```rust
        // The second `y` of `yy` yanks the cursor's row, for `p` and `P`:
        // wherever the first copied the cell, in the Data view, with the
        // keys on the grid or on a field of the row panel. The first is
        // the cell's `y` still, and waits as the first `d` of `dd` does.
        let row = app
            .workspace(tab)
            .and_then(|workspace| workspace.object_tab(object_tab))
            .filter(|object| object.view == crate::model::ObjectView::Data)
            .is_some_and(|object| object.selection.is_some());
        if sql.is_none() && !tree && row {
            if pending == Some('y') {
                actions.push(Action::YankRow {
                    tab,
                    id: object_tab,
                });
                // The row for the clipboard too, where it is the page's.
                if let Some(text) = app.copy_text(true) {
                    ctx.copy_text(text);
                }
                let said = crate::i18n::gettext(app.locale, "Row yanked");
                crate::ui::toast::say(ctx, &app.look.label(&said));
            } else {
                next_pending = Some('y');
            }
        }
```

- [ ] **Step 5: Run the tests.**

Run: `~/.cargo/bin/cargo test --locked --lib -- yy_ one_y_yanks p_puts_nothing`
Expected: PASS, six tests. If `yy_puts_the_row_on_the_clipboard_and_says_so` fails on the first `y` (`harness.copied` is not `ebook`), the harness kept an earlier frame's copy: assert on what `harness.app.copy_text(false)` gives instead, and keep the rest.

- [ ] **Step 6: Run the four checks.** Expected: all pass.

- [ ] **Step 7: Commit.**

```bash
git add src/ui/keys.rs src/ui/insert_audit_tests.rs src/ui/duplicate_tests.rs && git commit -m "Yank a row with yy and put it with p and P in the terminal look

The first y copies the cell as it did and waits, as the first d of dd
waits. The second keeps the row on its tab, puts it on the clipboard as
Mod+Shift+C does, and the status line says so. p puts a copy below the
cursor's row and P above it, read with the letters that edit.

yy is read on a field of the row panel too, a loaded row's among them:
the footer under that field names the key.

The audit's INS-02b is out of ignore.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 5: The row panel's Duplicate

**Files:**
- Modify: `src/ui/row_panel.rs` (the footer's call near line 919; `editing_footer` near line 2879)
- Modify: `src/ui/mod.rs` (its tests `the_row_panels_duplicate_and_delete_wait` near line 18213 and the comment near line 22905)
- Modify: `src/ui/duplicate_tests.rs`

- [ ] **Step 1: Write the failing tests.** In `src/ui/duplicate_tests.rs`, `egui::accesskit::Role` joins the imports:

```rust
use egui::accesskit::Role;
```

and at the end of the file:

```rust
// The row panel's footer.

/// Whether the button `name` can be pressed, and what it says of itself
/// where it cannot.
fn button(harness: &mut Harness, name: &str) -> (bool, Option<String>) {
    let tree = harness.settle();
    let found = crate::testing::node(&tree, name, Role::Button);
    let found = found.unwrap_or_else(|| panic!("no {name}"));
    let (_, node) = tree.nodes.iter().find(|(node, _)| *node == found).unwrap();
    (!node.is_disabled(), node.description().map(str::to_owned))
}

/// Test 14: Duplicate makes a copy of the panel's row. Delete waits, and
/// says so.
#[test]
fn the_panels_duplicate_makes_a_copy_and_delete_waits() {
    for look in [Look::standard(), Look::macos()] {
        let (mut harness, tab, id) = covers_in(look);
        select(&mut harness, tab, id, (1, 2));
        assert!(button(&mut harness, "Duplicate").0, "{}", look.name);
        let later = "Deleting rows arrives in a later version";
        let delete = button(&mut harness, "Delete");
        assert_eq!(delete, (false, Some(later.to_owned())), "{}", look.name);
        harness.click("Duplicate");
        assert_eq!(edits(&harness, tab, id).added.len(), 1, "{}", look.name);
        let row = newest(&harness, tab, id);
        assert_eq!(value(&harness, tab, id, (row, 2)), text("ebook"));
    }
}

/// Test 13: where no row is taken Duplicate cannot be pressed and says
/// why, in Add row's words.
#[test]
fn the_panels_duplicate_says_why_no_copy_can_be_made() {
    for (kind, read_only, why) in [
        (ObjectKind::Table, true, "This connection opens read-only"),
        (ObjectKind::View, false, "Views cannot be edited"),
    ] {
        let (mut harness, tab, id) = covers_as(Look::macos(), kind, read_only);
        select(&mut harness, tab, id, (1, 2));
        let duplicate = button(&mut harness, "Duplicate");
        assert_eq!(duplicate, (false, Some(why.to_owned())));
        harness.click("Duplicate");
        assert!(edits(&harness, tab, id).added.is_empty(), "{why}");
    }
}

/// Spec 1: what keeps a row's fields from being edited does not keep it
/// from being copied. A table without a key takes a copy.
#[test]
fn a_row_that_cannot_be_edited_can_still_be_copied() {
    let keyless = |structure: &mut Structure, _: &mut RowPage| structure.primary_key.clear();
    let (mut harness, tab, id) = covers(Look::macos(), keyless);
    select(&mut harness, tab, id, (1, 2));
    assert!(button(&mut harness, "Duplicate").0);
    // Delete has the row's own reason.
    let (pressable, why) = button(&mut harness, "Delete");
    assert!(!pressable);
    assert_ne!(why.as_deref(), Some("Deleting rows arrives in a later version"));
    harness.click("Duplicate");
    assert_eq!(edits(&harness, tab, id).added.len(), 1);
}

/// Spec 2: the terminal look's footer names `yy p` as a key that works
/// where a copy can be made, and `dd` as one that waits.
#[test]
fn the_terminals_footer_names_yy_p_as_a_key_that_works() {
    let (mut harness, tab, id) = covers_in(Look::omarchy());
    select(&mut harness, tab, id, (1, 2));
    assert!(button(&mut harness, "yy p duplicate").0);
    assert!(!button(&mut harness, "dd delete").0);
    let (mut harness, tab, id) = covers_as(Look::omarchy(), ObjectKind::Table, true);
    select(&mut harness, tab, id, (1, 2));
    assert!(!button(&mut harness, "yy p duplicate").0);
}
```

- [ ] **Step 2: Change the two tests that hold today's footer.** In `src/ui/mod.rs`, `the_row_panels_duplicate_and_delete_wait` becomes:

```rust
    #[test]
    fn the_row_panels_duplicate_can_be_pressed_and_delete_waits() {
        for look in Look::ALL {
            let (mut harness, tab, id) = editable_in(look);
            select(&mut harness, tab, id, (1, 1));
            let pressable = |harness: &mut Harness| {
                let buttons = footer_buttons(harness, &look);
                buttons
                    .into_iter()
                    .map(|(_, enabled)| enabled)
                    .collect::<Vec<_>>()
            };
            assert_eq!(pressable(&mut harness), [true, false], "{}", look.name);
            // With a value pending as without.
            make_pending(&mut harness, tab, id, (1, 1), "bob@example.com");
            assert_eq!(pressable(&mut harness), [true, false], "{}", look.name);
            // The panel is no mode to put it in: no button edits the row.
            let edit = if look.terminal { "e edit" } else { "Edit" };
            assert!(!has_button(&mut harness, edit), "{}", look.name);
        }
    }
```

And the comment `// Duplicate and Delete wait, as they do everywhere.` near line 22905 becomes `// A read-only connection takes no copy, and Delete waits as it does everywhere.` (the assertion under it stays: its connection opens read-only, and both are disabled there).

- [ ] **Step 3: Run them to see them fail.**

Run: `~/.cargo/bin/cargo test --locked --lib -- the_panels_duplicate a_row_that_cannot_be_edited the_terminals_footer the_row_panels_duplicate`
Expected: FAIL, five tests: Duplicate is disabled everywhere, and Delete says "Duplicating and deleting rows arrive in a later version".

- [ ] **Step 4: Let Duplicate be pressed.** In `src/ui/row_panel.rs`, replace `editing_footer`'s doc comment, its signature and its body down to the line `let height = if look.terminal {`'s closing `};` (the two branches that draw the buttons) with:

```rust
/// The footer of a table's row: Duplicate, which makes a copy of the row,
/// and Delete, which waits for a later version, and under them how a value
/// of the row is edited. `locked` is why no field of the row can be
/// edited, in the look's words: Delete says that under the pointer, and
/// nothing is said of editing. `no_copy` is why no copy of the row can be
/// made, where none can: what keeps a row from being added, which is not
/// what keeps this one from being edited. Says whether a copy was asked
/// for.
fn editing_footer(
    ui: &mut egui::Ui,
    rect: Rect,
    (locked, no_copy): (Option<&str>, Option<&str>),
    look: &Look,
    palette: &Palette,
    locale: crate::i18n::Locale,
) -> bool {
    let side = side(look);
    if !look.terminal {
        ui.painter()
            .rect_filled(rect, CornerRadius::ZERO, palette.panel);
    }
    widgets::hline(
        ui,
        rect.x_range(),
        rect.top() + 0.5,
        if look.terminal {
            palette.outline
        } else {
            palette.surface_hover
        },
    );
    let later = gettext(locale, "Deleting rows arrives in a later version");
    // Why Delete cannot be pressed: the row's reason, where it has one.
    let no_delete = locked.unwrap_or(later.as_ref());
    let inner = rect.shrink2(vec2(side, 0.0));
    let gap = 6.0;
    let top = rect.top() + 1.0 + if look.terminal { 10.0 } else { 12.0 };
    let role = widgets::body(look);
    let measure = |text: &str| role.width(ui.ctx(), look.faces, text);
    let mut copy = false;
    let height = if look.terminal {
        // Two equal cells, the keys in the text colour. What waits is
        // dashed, at 55%. A key that works is whole.
        let keys = [
            ("yy p", "duplicate", no_copy),
            ("dd", "delete", Some(no_delete)),
        ];
        let width = (inner.width() - gap) / 2.0;
        let faded = |color: egui::Color32| palette.panel.lerp_to_gamma(color, 0.55);
        for (index, (key, label, why)) in keys.into_iter().enumerate() {
            let place = Rect::from_min_size(
                pos2(inner.left() + index as f32 * (width + gap), top),
                vec2(width, 28.0),
            );
            let response = ui.interact(place, ui.id().with(("edit", index)), Sense::hover());
            let name = format!("{key} {label}");
            let works = why.is_none();
            response.widget_info(|| WidgetInfo::labeled(WidgetType::Button, works, &name));
            if let Some(why) = why {
                let _ = response.on_hover_text(why);
            }
            let shade = |color: egui::Color32| if works { color } else { faded(color) };
            if works {
                let stroke = Stroke::new(1.0, palette.outline);
                ui.painter()
                    .rect_stroke(place, CornerRadius::ZERO, stroke, StrokeKind::Inside);
            } else {
                dashed(ui, place, faded(palette.outline));
            }
            // A cell too narrow for both shows its key alone.
            let text = Text::one(look, role, key, shade(palette.text));
            let total = measure(&name);
            let (total, text) = if total > width {
                (measure(key), text)
            } else {
                let text = text.space(role, " ");
                (total, text.add(role, label, shade(palette.dim)))
            };
            let x = place.center().x - total / 2.0;
            widgets::paint_text(ui, x, place.center().y, text);
        }
        28.0
    } else {
        // Content-wide buttons sharing what is left equally: a 13 pt icon,
        // 6, the word.
        let labels = [
            (Icon::Copy, gettext(locale, "Duplicate"), no_copy),
            (Icon::Trash2, gettext(locale, "Delete"), Some(no_delete)),
        ];
        let widths: Vec<f32> = labels
            .iter()
            .map(|(_, text, _)| 13.0 + 6.0 + measure(text))
            .collect();
        let extra = (inner.width() - gap - widths.iter().sum::<f32>()) / 2.0;
        let mut x = inner.left();
        for (index, ((icon, text, why), width)) in labels.iter().zip(widths).enumerate() {
            let place = Rect::from_min_size(pos2(x, top), vec2(width + extra, 32.0));
            x += width + extra + gap;
            let button = widgets::ButtonSpec::new(text)
                .icon(*icon)
                .icon_size(13.0)
                .padding(0.0);
            let button = match *why {
                Some(why) => button.disabled(why),
                None => button,
            };
            // Only Duplicate does anything yet: the first of the two.
            let clicked = button.show_at(ui, place, look, palette).clicked();
            copy |= clicked && index == 0;
        }
        32.0
    };
```

The rest of the function stays, but for its early return and its end: `if look.terminal || locked.is_some() { return; }` becomes `return copy;` inside the same `if`, and the function's last line is `copy`.

At its call (near line 919), `editing_footer(ui, foot, locked.as_deref(), &look, &palette, locale);` becomes:

```rust
                // Why no copy of the row can be made, where none can: what
                // keeps a row from being added, in Add row's words, and in
                // the terminal's own where the look is its.
                let no_copy = object.and_then(|object| {
                    let lock = match Table::of(workspace, object) {
                        Some(table) => table.no_rows(),
                        None => Some(Lock::Refreshing),
                    };
                    let name = format::display_safe(&object.object.name);
                    lock.map(|lock| {
                        if look.terminal {
                            crate::ui::workspace::lock_line(lock, &name, &look, locale)
                        } else {
                            crate::ui::cell_editor::lock_text(lock, &name, locale)
                        }
                    })
                });
                let why = (locked.as_deref(), no_copy.as_deref());
                if editing_footer(ui, foot, why, &look, &palette, locale) {
                    actions.push(Action::DuplicateRow { tab, id });
                }
```

`Stroke` and `StrokeKind` are among the names the file takes from `egui` already (its `rect_stroke` calls near line 1808).

- [ ] **Step 5: Run the tests.**

Run: `~/.cargo/bin/cargo test --locked --lib -- the_panels_duplicate a_row_that_cannot_be_edited the_terminals_footer the_row_panels_duplicate the_row_panel_says_at_its_top a_result_rows_panel`
Expected: PASS. The last two filters are the tests that hold a read-only row's footer and a result's: they pass as they did.

- [ ] **Step 6: Run the four checks.** Expected: all pass.

- [ ] **Step 7: Commit.**

```bash
git add src/ui/row_panel.rs src/ui/mod.rs src/ui/duplicate_tests.rs && git commit -m "Let the row panel's Duplicate be pressed

It makes a copy of the panel's row. Where no row can be added it says
why under the pointer, in Add row's words, and that is not the reason
the row's fields cannot be edited: a table without a key takes a copy.
The terminal look's footer shows yy p as a key that works, whole, and
dd as one that waits, dashed as both were.

Delete waits still, and says so of itself alone. Two tests held that
both buttons wait: one now holds that Duplicate does not, and the
other's comment says what it holds of a view.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 6: The form says what the copy was made as

**Files:**
- Modify: `src/ui/row_panel.rs` (`Fresh` near line 239; the notes under the header near line 856; its `mod tests` near line 3013)
- Modify: `src/ui/duplicate_tests.rs`

- [ ] **Step 1: Write the failing tests.** In `src/ui/row_panel.rs`, inside `mod tests`:

```rust
    #[test]
    fn a_copys_line_names_its_columns_and_why_each_was_left() {
        use crate::edit::{Cleared, Copied, Left};
        let names = |names: &[&str]| -> Vec<String> {
            names.iter().map(|name| (*name).to_owned()).collect()
        };
        let locale = crate::i18n::Locale::default();
        let made = Copied {
            copied: names(&["Title", "format", "pages", "price", "stock"]),
            cleared: vec![
                ("id".into(), Cleared::Identity),
                ("isbn".into(), Cleared::Unique),
            ],
            left: vec![("cover".into(), Left::Binary)],
        };
        assert_eq!(
            copy_line(&made, &Look::macos(), locale),
            "Copied: Title, format and 3 more. Cleared: id (identity), isbn (unique). \
             Not copied: cover (binary)."
        );
        // The terminal's words are in its case. A column's name keeps its.
        assert_eq!(
            copy_line(&made, &Look::omarchy(), locale),
            "copied: Title, format and 3 more. cleared: id (identity), isbn (unique). \
             not copied: cover (binary)."
        );
        // Up to three names are written out, and a sentence with no
        // column is left out.
        let few = Copied {
            copied: names(&["a", "b", "c"]),
            ..Copied::default()
        };
        assert_eq!(copy_line(&few, &Look::macos(), locale), "Copied: a, b, c.");
        // Each reason in its words.
        let all = Copied {
            copied: Vec::new(),
            cleared: vec![
                ("a".into(), Cleared::Generated),
                ("b".into(), Cleared::PrimaryKey),
            ],
            left: vec![("c".into(), Left::TooLarge), ("d".into(), Left::Unknown)],
        };
        assert_eq!(
            copy_line(&all, &Look::macos(), locale),
            "Cleared: a (generated), b (primary key). Not copied: c (too large), d (unknown)."
        );
        assert_eq!(copy_line(&Copied::default(), &Look::macos(), locale), "");
    }
```

In `src/ui/duplicate_tests.rs`, at the end of the file:

```rust
// The copy's form.

/// Test 15: the form of a copy says what it was made as, in both looks,
/// above its fields, and goes on saying it as the row is filled in.
#[test]
fn the_copys_form_says_what_it_was_made_as() {
    let line = "Copied: kind, image_data, created_at. \
                Cleared: id (identity), publisher_id (unique).";
    for look in Look::ALL {
        let (mut harness, tab, id) = covers(look, unique_publisher);
        select(&mut harness, tab, id, (1, 2));
        duplicate(&mut harness, tab, id);
        harness.app.apply(Action::CancelEdit { tab, id });
        assert!(harness.has(&look.label(line)), "{}", look.name);
        let row = newest(&harness, tab, id);
        set(&mut harness, tab, id, (row, 1), "9100000000000000007");
        assert!(harness.has(&look.label(line)), "{}: filled in", look.name);
        // An added row's form says nothing of the kind.
        let (mut harness, tab, id) = covers(look, unique_publisher);
        add(&mut harness, tab, id);
        let tree = harness.settle();
        let labels = crate::testing::labels(&tree);
        let copied = look.label("Copied:");
        assert!(
            !labels.iter().any(|label| label.starts_with(&copied)),
            "{}",
            look.name
        );
    }
}
```

- [ ] **Step 2: Run them to see them fail.**

Run: `~/.cargo/bin/cargo test --locked --lib -- a_copys_line the_copys_form`
Expected: it does not compile: `copy_line` is not there.

- [ ] **Step 3: Give the form what the copy was made as.** In `src/ui/row_panel.rs`, `Fresh` gains a field, after `keyless`:

```rust
    /// What the row was made as, where it is a copy of another row.
    made: Option<crate::edit::Copied>,
```

and `Fresh::of` fills it. Before its `Self {`:

```rust
        // The new row itself, for what it keeps of how it was made.
        let new = crate::edit::new_id(row)
            .and_then(|id| table.added.iter().find(|new| new.id == id));
```

and in the literal, after `keyless: table.key().is_none(),`:

```rust
            made: new.and_then(|new| new.made.clone()),
```

- [ ] **Step 4: Say it.** In `src/ui/row_panel.rs`, after `new_row_footer`:

```rust
/// What the form of a copy says of how its row was made: the columns it
/// has the source's values in, the ones it cleared and the ones it could
/// not copy, the last two each with its reason. A sentence with no column
/// is left out. The words are the look's, and a column's name keeps its
/// own case, as it does in the status line.
fn copy_line(made: &crate::edit::Copied, look: &Look, locale: crate::i18n::Locale) -> String {
    use crate::edit::{Cleared, Left};
    let say = |text: &'static str| look.label(&gettext(locale, text));
    let name = |name: &str| format::display_safe(name).into_owned();
    let mut said = Vec::new();
    let copied: Vec<String> = made.copied.iter().map(|column| name(column)).collect();
    // Up to three in full. More would fill the panel: the first two, and
    // how many follow.
    let listed = match copied.as_slice() {
        [] => None,
        [first, second, rest @ ..] if rest.len() > 1 => {
            let (and, more) = (say("and"), say("more"));
            Some(format!("{first}, {second} {and} {} {more}", rest.len()))
        }
        all => Some(all.join(", ")),
    };
    if let Some(listed) = listed {
        said.push(format!("{}: {listed}.", say("Copied")));
    }
    let cleared: Vec<String> = made
        .cleared
        .iter()
        .map(|(column, why)| {
            let why = match why {
                Cleared::Identity => say("identity"),
                Cleared::Generated => say("generated"),
                Cleared::PrimaryKey => say("primary key"),
                Cleared::Unique => say("unique"),
            };
            format!("{} ({why})", name(column))
        })
        .collect();
    if !cleared.is_empty() {
        said.push(format!("{}: {}.", say("Cleared"), cleared.join(", ")));
    }
    let left: Vec<String> = made
        .left
        .iter()
        .map(|(column, why)| {
            let why = match why {
                Left::Unknown => say("unknown"),
                Left::Binary => say("binary"),
                Left::TooLarge => say("too large"),
            };
            format!("{} ({why})", name(column))
        })
        .collect();
    if !left.is_empty() {
        said.push(format!("{}: {}.", say("Not copied"), left.join(", ")));
    }
    said.join(" ")
}

/// A note of a new row's form, under the panel's header from `top` down:
/// its words wrapped to the panel's width over a rule, and said whole to a
/// screen reader. Says where it ends.
fn form_note(
    ui: &mut egui::Ui,
    (full, top): (Rect, f32),
    said: &str,
    (look, palette): (&Look, &Palette),
    divider: egui::Color32,
) -> f32 {
    let side = side(look);
    let room = (full.width() - 2.0 * side).max(24.0);
    let laid = Text::one(look, caption(look), said, palette.secondary)
        .wrap(room)
        .layout(ui.ctx());
    let height = 9.0 + laid.height() + 9.0 + 1.0;
    let strip = Rect::from_min_size(pos2(full.left(), top), vec2(full.width(), height));
    if !look.terminal {
        ui.painter()
            .rect_filled(strip, CornerRadius::ZERO, palette.panel);
    }
    widgets::hline(ui, strip.x_range(), strip.bottom() - 0.5, divider);
    laid.paint(ui.painter(), pos2(strip.left() + side, strip.top() + 9.0));
    widgets::announce(ui, strip, said);
    strip.bottom()
}
```

Then the block that draws the note of a table without a key (from its comment `// A table without a key takes the row, and can then find it` to the closing brace of its `if`, near lines 856 to 879) becomes:

```rust
            // A copy says what it was made as, before it is filled in.
            if let Some(made) = fresh.as_ref().and_then(|fresh| fresh.made.as_ref()) {
                let said = copy_line(made, &look, locale);
                top = form_note(ui, (full, top), &said, (&look, &palette), divider);
            }
            // A table without a key takes the row, and can then find it
            // by nothing: said before the row is filled in.
            if fresh.as_ref().is_some_and(|fresh| fresh.keyless) {
                let said = look.label(&gettext(
                    locale,
                    "This table has no primary key. The row can be inserted, but not edited or \
                     deleted afterwards.",
                ));
                top = form_note(ui, (full, top), &said, (&look, &palette), divider);
            }
```

- [ ] **Step 5: Run the tests.**

Run: `~/.cargo/bin/cargo test --locked --lib -- a_copys_line the_copys_form the_inspector_warns_of_a_table_without_a_key`
Expected: PASS, three tests. The third is the audit's test of the note that `form_note` now draws: it passes as it did.

- [ ] **Step 6: Run the four checks.** Expected: all pass.

- [ ] **Step 7: Commit.**

```bash
git add src/ui/row_panel.rs src/ui/duplicate_tests.rs && git commit -m "Say in a copy's form what it was made as

At the form's top, above the note of a table without a key: the columns
the copy has the source's values in, the ones it cleared and the ones it
could not copy, each of the last two with its reason. It is what the
copy was, kept on the new row, and does not change as the row is filled
in.

The two notes are drawn by one function.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 7: `required · unique`

**Files:**
- Modify: `src/ui/format.rs` (after `plain_text`)
- Modify: `src/ui/data_view.rs` (`Changes` near line 1568, its `of` near line 1706, where `changes.unset` is set near line 1353, `new_cell` near line 1920)
- Modify: `src/ui/row_panel.rs` (`Fresh`; the field near lines 1313, 1410, 1461 and 1725; `unset_words` and `unset_value` near lines 1962 and 1980)
- Modify: `src/ui/duplicate_tests.rs`

- [ ] **Step 1: Write the failing tests.** In `src/ui/duplicate_tests.rs`, `painted` joins the names taken from `super::insert_audit_tests`, and at the end of the file:

```rust
// What a copy's cleared cell says.

/// Test 16: a cell of a copy that a save needs a value in, and that was
/// cleared for holding what no other row may, says so: in the grid's cell
/// and in the form's field, to the eye and to a screen reader. An added
/// row's cell of the same column says only that it is required.
#[test]
fn a_copys_cleared_cell_says_it_is_required_and_unique() {
    let both = "required · unique";
    let field = "publisher_id, INTEGER, required · unique";
    let said = |harness: &mut Harness, label: &str| {
        let tree = harness.settle();
        let labels = crate::testing::labels(&tree);
        labels.iter().any(|found| found.starts_with(label))
    };
    for look in Look::ALL {
        let (mut harness, tab, id) = covers(look, unique_publisher);
        select(&mut harness, tab, id, (1, 2));
        duplicate(&mut harness, tab, id);
        harness.app.apply(Action::CancelEdit { tab, id });
        harness.settle();
        // The grid's cell in the desktop looks, the form's field in all.
        assert!(painted(&harness, both), "{}", look.name);
        assert!(said(&mut harness, field), "{}", look.name);
        // The terminal's status line names what the rows lack together,
        // as it did.
        if look.terminal {
            assert!(painted(&harness, "publisher_id required"));
            assert!(!painted(&harness, "publisher_id required · unique"));
        }
        // An added row: required, and no more.
        let (mut harness, tab, id) = covers(look, unique_publisher);
        add(&mut harness, tab, id);
        harness.settle();
        assert!(!painted(&harness, both), "{}", look.name);
        assert!(painted(&harness, "required"), "{}", look.name);
        assert!(!said(&mut harness, field), "{}", look.name);
        assert!(said(&mut harness, "publisher_id, INTEGER, required"));
    }
    // A column cleared as the database's own asks nothing, and one that
    // was not cleared is required as ever: no copy says `unique` of them.
    let (mut harness, tab, id) = covers_in(Look::macos());
    select(&mut harness, tab, id, (1, 2));
    duplicate(&mut harness, tab, id);
    harness.settle();
    assert!(!painted(&harness, both));
}
```

- [ ] **Step 2: Run it to see it fail.**

Run: `~/.cargo/bin/cargo test --locked --lib a_copys_cleared_cell`
Expected: FAIL on its first assertion: the copy's cell says `required`.

- [ ] **Step 3: The words.** In `src/ui/format.rs`, after `plain_text`:

```rust
/// What a new row's cell or field says where a save needs a value:
/// "required", and "required · unique" in a copy of a row that cleared the
/// column for holding what no other row may.
pub fn required_word(unique: bool, locale: crate::i18n::Locale) -> String {
    let required = crate::i18n::gettext(locale, "required");
    if unique {
        let unique = crate::i18n::gettext(locale, "unique");
        format!("{required} · {unique}")
    } else {
        required.into_owned()
    }
}
```

- [ ] **Step 4: The grid's cell.** In `src/ui/data_view.rs`, `Changes` gains a field after `marker`:

```rust
    /// The cells of new rows that are copies and were cleared there for
    /// holding what no other row may (`NewRow::cleared_unique`), by row
    /// and column.
    unique: BTreeSet<(usize, usize)>,
```

`Changes::of` (and any other literal of `Changes` the compiler names) fills it with `unique: BTreeSet::new(),`.

Where the view sets `changes.unset = unset;`, after that line:

```rust
        changes.unique = object
            .edits
            .added
            .iter()
            .flat_map(|new| {
                let row = crate::edit::new_row(new.id);
                let columns = page.columns.iter().enumerate();
                columns
                    .filter(move |(_, column)| new.cleared_unique(&column.name))
                    .map(move |(col, _)| (row, col))
            })
            .collect();
```

And in `new_cell`, the arm

```rust
                Some(Unset::Required) => Cell {
                    text: say("required").into(),
```

becomes

```rust
                Some(Unset::Required) => Cell {
                    text: format::required_word(self.unique.contains(&at), locale).into(),
```

- [ ] **Step 5: The form's field.** In `src/ui/row_panel.rs`, `Fresh` gains a field after `made`:

```rust
    /// Whether the row is a copy that cleared the column for holding what
    /// no other row may, by the page's columns: where a save needs a
    /// value in one, its field says why.
    unique: Vec<bool>,
```

and `Fresh::of` fills it, after `made: ...,`:

```rust
            unique: table
                .page
                .columns
                .iter()
                .map(|column| new.is_some_and(|new| new.cleared_unique(&column.name)))
                .collect(),
```

In the function that draws a field, after the `let unset = fresh ... ;` near line 1313:

```rust
    // Whether the row is a copy that cleared this column as unique.
    let unique = fresh.is_some_and(|fresh| fresh.unique.get(col).copied().unwrap_or(false));
```

Then the three places that say `required`:

1. What a screen reader hears (near line 1410): `(Some(unset), _) => unset_words(unset, locale),` becomes `(Some(unset), _) => unset_words(unset, unique, locale),`.
2. The chip (near line 1461): `Some(Unset::Required) => Some((gettext(locale, "required"), true)),` becomes `Some(Unset::Required) => Some((format::required_word(unique, locale).into(), true)),`.
3. The terminal's value (near line 1725): `unset_value(ui, unset, (tag, lock), (role, pad), skin)` becomes `unset_value(ui, (unset, unique), (tag, lock), (role, pad), skin)`.

`unset_words` takes the flag:

```rust
/// What a screen reader hears in the place of the value of a new row's
/// field nothing is set in. `unique` says the row is a copy that cleared
/// the column for holding what no other row may.
fn unset_words(unset: &Unset, unique: bool, locale: crate::i18n::Locale) -> String {
    let say = |text: &'static str| gettext(locale, text);
    match unset {
        Unset::Assigned => say("assigned by the database").into_owned(),
        Unset::Default(text) | Unset::Expression(text) => {
            format!("{} {}", say("default"), format::display_safe(text))
        }
        Unset::Null => "NULL".to_owned(),
        Unset::Required => format::required_word(unique, locale),
    }
}
```

and so does `unset_value`: its second parameter `unset: &Unset,` becomes `(unset, unique): (&Unset, bool),`, its doc comment gains the sentence "`unique` says the row is a copy that cleared the column for holding what no other row may.", and in its `Unset::Required` arm

```rust
                let word = Text::one(look, role, &say("required"), palette.danger);
```

becomes

```rust
                let word = format::required_word(unique, locale);
                let word = Text::one(look, role, &word, palette.danger);
```

- [ ] **Step 6: Run the tests.**

Run: `~/.cargo/bin/cargo test --locked --lib -- a_copys_cleared_cell the_inspector`
Expected: PASS. The second filter is the audit's tests of the form, which pass as they did.

If `a_copys_cleared_cell_says_it_is_required_and_unique` fails on the field's label, print the tree's labels (`crate::testing::labels`) and read how the field names itself: the test holds that the label begins with the column, its type and the words, and the words are what this task changes.

- [ ] **Step 7: Run the four checks.** Expected: all pass.

- [ ] **Step 8: Commit.**

```bash
git add src/ui/format.rs src/ui/data_view.rs src/ui/row_panel.rs src/ui/duplicate_tests.rs && git commit -m "Say that a copy's cleared cell is required for being unique

A column a copy cleared for being alone in a unique index, or of the
key, and that a save needs a value in, says required and unique: in the
grid's cell of the desktop looks, and in the form's field of every look,
to the eye and to a screen reader. Every other required cell says what
it said.

The terminal's status line and the pending bar name what all the tab's
new rows lack together, and are not changed.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 8: The keys in the shortcut table, and the website

**Files:**
- Modify: `src/ui/keys.rs` (`SHORTCUTS` near lines 91 and 131; its tests near line 1836)
- Modify: `docs/_guide/editing-data.md`, `docs/_guide/what-is-tabletist.md`, `docs/_reference/keyboard-shortcuts.md`, `docs/_guide/macos.md`, `docs/_guide/omarchy.md`

- [ ] **Step 1: Write the failing test.** In `src/ui/keys.rs`, in the test whose body has `assert_eq!(keys("Add a row"), Some("Mod+N"));` (near line 1840), the two branches gain a line each:

```rust
            if look.terminal {
                assert_eq!(keys("Add a row below or above the cursor"), Some("o, O"));
                assert_eq!(keys("Drop a new row"), Some("dd"));
                assert_eq!(keys("Add a row"), None);
                let put = "Yank the row, put a copy below or above";
                assert_eq!(keys(put), Some("yy, p, P"));
                assert_eq!(keys("Duplicate the row"), None);
            } else {
                assert_eq!(keys("Add a row"), Some("Mod+N"));
                assert_eq!(keys("Drop a new row"), Some("Delete"));
                assert_eq!(keys("Duplicate the row"), Some("Mod+D"));
            }
```

and in `the_shortcut_table_names_omarchys_keys_that_edit`, the list of keys gains `"yy", "p", "P"`:

```rust
        for key in [
            "i", "Enter", "cc", "x", "u", "Mod+S", ":w", ":e!", ":diff", "Y", "Space", "yy", "p",
            "P",
        ] {
```

- [ ] **Step 2: Run them to see them fail.**

Run: `~/.cargo/bin/cargo test --locked --lib the_shortcut_table`
Expected: FAIL, two tests: no row of the table names the keys.

- [ ] **Step 3: List the keys.** In `src/ui/keys.rs`, in `SHORTCUTS`, after `("Mod+N", "Add a row", DESKTOP),` and `("o, O", "Add a row below or above the cursor", TERMINAL),`:

```rust
    ("Mod+D", "Duplicate the row", DESKTOP),
    (
        "yy, p, P",
        "Yank the row, put a copy below or above",
        TERMINAL,
    ),
```

and the row of `"Omarchy: vim keys (shown in the status line)"` names them after `O`: its keys become

```rust
        "j/k, h/l, Ctrl+H/L, [ ], i, Enter, cc, x, D, u, o, O, yy, p, P, dd, f, Mod+S, :w, :e!, :diff, Y, Space, Esc, /, y, s, d, gd, za, t, 1…9",
```

- [ ] **Step 4: Run them.**

Run: `~/.cargo/bin/cargo test --locked --lib the_shortcut_table`
Expected: PASS.

- [ ] **Step 5: The website.**

`docs/_reference/keyboard-shortcuts.md`, in the table under "Editing", after the row of `Mod+N`:

```markdown
| Mod+D | [Duplicate the row]({% link _guide/editing-data.md %}#duplicate-a-row), as a new row at the top of the grid |
```

`docs/_guide/macos.md`, in the table under "Editing", after the row of `Cmd+N`:

```markdown
| Cmd+D | Duplicate the row, as a new row at the top of the grid |
```

`docs/_guide/omarchy.md`, in the table under "Editing", after the row of `` `o`, `O` ``:

```markdown
| `yy` | Yank the cursor's row. The first `y` copies the cell, as `y` alone does |
| `p`, `P` | Put a copy of the yanked row below the cursor's row, or above it |
```

`docs/_guide/editing-data.md`, near its top, `Today you can change values in existing rows and add rows. Duplicating, pasting and deleting rows are not built yet.` becomes:

```markdown
Today you can change values in existing rows, add rows and duplicate a
row. Pasting and deleting rows are not built yet.
```

`docs/_guide/what-is-tabletist.md`, in its list of limits, the item `You can change values in existing rows. Adding, duplicating and deleting rows are not built yet.` is stale twice over (rows are added since pull request 104) and becomes:

```markdown
- You can change values in existing rows, add rows and duplicate a row.
  Deleting rows is not built yet.
```

`docs/_guide/editing-data.md` again, a new section after the whole of "Add a row" (its "In the row panel" part included) and before the next `## ` heading:

```markdown
## Duplicate a row

Select a row and press Cmd/Ctrl+D, or **Duplicate** under the row panel.
A copy of the row appears at the top of the grid as a new row, to be
changed and saved like any other. In the Omarchy look, `yy` yanks the
cursor's row, and `p` puts a copy below the cursor's row, `P` above it.
The yanked row stays for the next `p`, through a reload too, and `yy`
also puts the row on the clipboard.

The copy holds what the row shows, the changes you have not saved among
them. It leaves out what is the row's own:

- the primary key, and a column the database numbers or computes;
- a column that is alone in a unique index. The copy could not be saved
  with the same value.

Where a save needs a value in such a column the cell says **required ·
unique**, and the cursor starts there.

Two kinds of value are not copied: a binary column, and text over 256
KB. The row panel, as the copy's form, says at its top which columns
were copied, which were cleared and why, and which were not copied.

One row is copied at a time.
```

- [ ] **Step 6: Check the site where Ruby is at hand.**

Run: `cd docs && bundle install && bin/check`
Expected: it builds and its links hold. Where there is no Ruby, say so in the task's report: `docs.yml` runs the same check on the pull request.

- [ ] **Step 7: Run the four checks.** Expected: all pass.

- [ ] **Step 8: Commit.**

```bash
git add src/ui/keys.rs docs/_guide/editing-data.md docs/_guide/what-is-tabletist.md docs/_guide/macos.md docs/_guide/omarchy.md docs/_reference/keyboard-shortcuts.md && git commit -m "List the keys that copy a row, and say on the website what a copy is

Mod+D for the desktop looks and yy, p and P for the terminal's, in the
shortcut table and on the pages that follow it. The editing guide says
what a copy holds, what it leaves out and why.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 9: Say in the specs that Duplicate is built

**Files:**
- Modify: `docs/superpowers/specs/2026-10-10-row-duplication-design.md`
- Modify: `docs/superpowers/specs/2026-10-07-inserting-rows-design.md`
- Modify: `docs/superpowers/specs/2026-10-06-row-inspector-inline-edit-design.md`
- Modify: `docs/superpowers/specs/2026-10-06-row-form-design.md`
- Modify: `docs/superpowers/plans/2026-10-10-row-duplication.md` (this file)

No code changes, so no test of its own: the four checks stay green.

- [ ] **Step 1: This spec and this plan.** In `docs/superpowers/specs/2026-10-10-row-duplication-design.md`, `Date: 2026-10-10. Status: designed, not built.` becomes:

```markdown
Date: 2026-10-10. Status: built
(`docs/superpowers/plans/2026-10-10-row-duplication.md`, whose "What the
spec leaves open, and what gets built" says what the build decided).
```

In this plan, `Status: written, not built.` becomes `Status: built.`

- [ ] **Step 2: The insert spec.** In `docs/superpowers/specs/2026-10-07-inserting-rows-design.md`:

Its opening paragraph, after the sentence "The row panel is the new row's form (section 7).", gains: "A row is duplicated (section 4, as `2026-10-10-row-duplication-design.md` builds it)."

Section 4 ("Duplicate") becomes, with the three bullets that change:

```markdown
### 4. Duplicate

- Copies every column of the selected row except:
  - primary key columns;
  - identity, serial and generated columns;
  - columns with a UNIQUE constraint or unique index (single-column only; multi-column unique indexes are copied and left to the database). An index that is not over one whole column, compared as the column compares (an expression, MySQL's prefix, another collation), clears none and is left to the database too.
- A binary column and text over 256 KB are not copied: a pending value has no form for them. The inspector says which columns they were.
- A NULL is set in the copy only where the column has a default. Elsewhere the column is left unset, which is NULL.
- Cleared columns that are required are marked `required · unique` (or `required`), and focus goes to the first of them.
- The inspector shows "Copied: format, title and 31 more. Cleared: id (identity), isbn (unique)."
- Duplicating a pending new row copies its pending values. Duplicating an existing row that has pending edits copies the pending values, not the loaded ones.
- With several rows selected, ⌘D duplicates each of them (at most 500). Omarchy uses `yy` with a count or a visual selection, then `p`. This waits for a selection of several rows, which is slice 4 of the value editing spec: until then one row is copied at a time.
```

In its Delivery table, the row `| 3 | Duplicate, and Down on the last row | |` becomes two:

```markdown
| 3, Duplicate | A copy of the selected row as a new row: ⌘D and the inspector's Duplicate, `yy` then `p` or `P` on Omarchy, the inspector's line of what was copied and cleared, and `required · unique` (`2026-10-10-row-duplication-design.md`, `docs/superpowers/plans/2026-10-10-row-duplication.md`). One row at a time | built |
| 3, the rest | Down on the last row | |
```

Under "Not runs of this spec, and asked of it above", a bullet is added:

```markdown
- Several rows duplicated at once: with the selection of several rows, slice 4 of the value editing spec.
```

And a new section at the end of the file:

```markdown
## Decisions of 2026-10-10

Made with the user while Duplicate was designed. They are D1 to D6 of `2026-10-10-row-duplication-design.md`, which gives each its reason.

| | What was open | Decided |
|---|---|---|
| E1 | A binary column and a text over 256 KB in a copy | Not copied, and said |
| E2 | A NULL in the source | Set only where the column has a default |
| E3 | Several rows at once | Waits for a selection of several rows |
| E4 | A unique index not over one whole column | Clears none: the database judges the save |
| E5 | How Omarchy's `yy` reaches `p` | The row's values are kept on the tab. `yy` also puts the row on the clipboard as TSV |
| E6 | Down on the last row | A later run, apart from Duplicate |
```

- [ ] **Step 3: The two specs of the row panel.** Each says "Duplicate and Delete stay disabled" in the user's own text, which is not rewritten. Each gains one sentence at the end of its opening status paragraph (the first paragraph under its title):

`docs/superpowers/specs/2026-10-06-row-inspector-inline-edit-design.md`: "Duplicate is built since 2026-10-10 (`2026-10-10-row-duplication-design.md`): of the row actions below only Delete still waits."

`docs/superpowers/specs/2026-10-06-row-form-design.md`: "Duplicate is built since 2026-10-10 (`2026-10-10-row-duplication-design.md`): where the text below says Duplicate and Delete stay disabled, only Delete does."

- [ ] **Step 4: Check that no line of the four files has an em dash that was not there.**

Run: `git diff -U0 -- docs/superpowers | grep '^+' | grep -c $'\u2014'`
Expected: `0`.

- [ ] **Step 5: Run the four checks.** Expected: all pass.

- [ ] **Step 6: Commit.**

```bash
git add docs/superpowers && git commit -m "Say in the specs that Duplicate is built

The insert spec's section 4 takes what the duplication spec decided: a
binary value and a text over the editor's limit are not copied, a NULL
is set only where the column has a default, an index not over one whole
column clears nothing, and several rows at once wait for a selection of
several. Its run 3 is Duplicate, built, and Down on the last row, which
is not. The two specs of the row panel say that Duplicate no longer
waits.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

## After the last task

- Run the four checks once more on the branch's head.
- Run the app and try it by hand, in the macOS look and the Omarchy look, on the Bookshop: Mod+D, the button, `yy p`, `yy P`, a copy of a new row, Review SQL, a save. `AGENTS.md` asks that platform coverage be reported honestly: say what was run and on which platform, and what was only compiled.
- The pull request's description has a summary, the spec's tests it covers (1 to 21), and no unblurred screenshot.
