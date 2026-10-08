# Special Values in a Cell: DEFAULT, now, typed keywords, booleans Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A cell can be given a value that is not text: the column's DEFAULT, the time of the save (`now()`), and NULL by typing it. A boolean cell is flipped with Space. Today every typed value is sent as text, so `now()` in a timestamp is refused by the database and `NULL` is four letters.

**Architecture:** `tabletist_db::NewValue` gains `Default` and `Now` beside `Null` and `Text`. `Dialect::new_operand`, the one place a new value becomes part of a statement, writes them as keywords into the text of the `UPDATE` and the `INSERT`, shown and sent alike, never bound. In the app, `edit::read` turns what was typed into a `NewValue` by the column's class (a word is a keyword only where the column is not text), and the editor, the set, Review SQL and the grid go through it. Two actions set a value without typing: `SetDefault` and `CycleBoolean`.

**Tech Stack:** Rust 1.98, egui/eframe, `tabletist-db`. The design is the canvas board "macOS – Editing values, editors by type" (`MacEditTypes`) and "Omarchy – Editing values" (`OmarchyEditValues`), and the user's answers of 2026-10-07 below. This is a part of slice 2 of `docs/superpowers/specs/2026-10-03-value-editing-core-design.md` ("editors by type").

---

## Before you start

- Cargo is `~/.cargo/bin/cargo` (the mise shim fails). Never point `CARGO_TARGET_DIR` at `/tmp`.
- The four checks, from `AGENTS.md`, pass after every task:

```bash
~/.cargo/bin/cargo fmt --all --check
~/.cargo/bin/cargo clippy --locked --workspace --all-targets -- -D warnings
~/.cargo/bin/cargo test --locked --workspace --all-targets
RUSTDOCFLAGS='-D warnings' ~/.cargo/bin/cargo doc --locked --workspace --no-deps
```

- **What was run where this plan was written: nothing.** It was written from reading the tree at `85243d8` (the head of `claude/inserting-rows-2-add-row`, pull request 104). The code blocks are written against names in that tree and were not compiled. Where a block and the compiler disagree, the compiler is right and the task's tests say what must hold. No reviewer has read it.
- **Databases.** Task 1 adds tests to `crates/tabletist-db/tests/` that need servers for PostgreSQL and MySQL:

```bash
docker compose up -d --build --wait postgres mysql
export TABLETIST_TEST_PG_URL=postgres://tabletist:tabletist@localhost:55432/tabletist
export TABLETIST_TEST_MYSQL_URL=mysql://tabletist:tabletist@localhost:53306/tabletist
```

  Without the variables they print "skipped" and pass. If the servers cannot be started in the session, say so in the report and the pull request: CI runs them.
- **The branch.** `claude/special-values`, from `85243d8`. It stands on pull request 104, which is not merged: open this one against that branch, and against `main` once 104 is in.
- House rules: no em dashes; comments say why; code that depends on the engine matches on `Dialect` and names every variant; a view pushes `Action`s; text only through `TextRole`s; what a user typed stays out of logs; every behaviour change has a headless test.
- Commits are signed, one per task. If signing fails ("agent refused operation"), stage the task and tell the user. Chain `git add` and `git commit` with `&&`. Each message ends with:

      Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>

## What the user decided (2026-10-07)

1. **In scope:** `now` for date and time cells, Space cycling a boolean cell, and Set DEFAULT. **Not in scope:** an explicit "Empty ''" control.
2. **Typed words:** a keyword, except in text columns. In a number, date, boolean or JSON column the typed word is the keyword; in a text column it stays the text that was typed.
3. **`now()`** reaches the database as the SQL function, by the server's clock at the save. Review SQL shows the function, and the saved row shows the stored time.

## What the design asks, and what gets built

Each row is the user's to overrule before task 1.

| | The design, or the question it leaves | What gets built | Why |
|---|---|---|---|
| a | Boolean: "space cycles in place, no editor needed" | Space on a boolean cell that can be edited cycles true, false, NULL (NULL only where the column takes it). On any other cell Space shows the row panel, as today | The canvas. It takes Space from the row panel on those cells only |
| b | "DEFAULT ⌘'" (Editing values), "Set DEFAULT · Cell · ⌘'" (Keyboard settings); Omarchy `D` | `Mod+'` and `D` set the selected cell to DEFAULT | The inserting spec gave `⌘'` to "jump between errors" too (its section 9, run 5). The canvas' Keyboard board gives it to Set DEFAULT: run 5 needs another key |
| c | Typed words (not drawn in the canvas: the user's addition) | Read without regard to case, with the spaces around them dropped: `null`; `default`; `now`, `now()`, `current_timestamp`. In a JSON column only the capitals `NULL` and `DEFAULT`: `null` there is a document | JSON has a `null` of its own |
| d | `NULL` typed where the column is NOT NULL | Stays text, and is checked as text: a number column refuses it in the editor | A keyword the save would refuse helps nobody |
| e | `DEFAULT` where the column has no default | Nothing: the key does nothing, and the word stays text | The same |
| f | DEFAULT on SQLite, which has no `DEFAULT` to write in an `UPDATE` or in `VALUES` | The column's default expression itself, in parentheses, as the catalog gives it: `SET "kind" = ('print')` | It is what the keyword would have come to. Review SQL shows it |
| g | DEFAULT in a new row | The cell is unset: the `INSERT` does not name the column | A new row's unset cell is its default already |
| h | `now` as the SQL function | PostgreSQL `now()`, MySQL `CURRENT_TIMESTAMP(6)`, SQLite `CURRENT_TIMESTAMP`. A date column gets `CURRENT_DATE`, a time column `CURRENT_TIME` (`CURRENT_TIME(6)` on MySQL). Only in date and time columns: elsewhere the word is text | MySQL warns when a time is cut to a date, and a save there takes a warning for a failure. SQLite's `CURRENT_TIMESTAMP` is UTC to the second: a table that keeps fractions gets none from it |
| i | A pending DEFAULT or now in the grid | The word `DEFAULT` or `now()`, in the quiet style a cell uses for what stands for a value, on the pending tint. Saved, the cell shows what the database stored | It is not the value yet |
| j | The same in the row panel, in the conflict question and on the clipboard | The word as text | The panel's form for these is with the editors by type |
| k | "Dates and times accept typing first", with a **now** button beside the editor | Task 7: the button, in the grid's editor and the row panel's. Tasks 1 to 6 give `now()` by typing | The button is new drawing in two editors. The word works without it |
| l | Boolean's editor: a control of three segments | Not built: Space cycles, and typing `true` or `false` works as it does today | Editors by type |
| m | "Clearing a field gives `''`, never NULL" | As today: an empty text is the empty string | Unchanged |

## File map

| File | What changes |
|---|---|
| `crates/tabletist-db/src/write.rs` | `NewValue::Default`, `NewValue::Now` |
| `crates/tabletist-db/src/class.rs`, `lib.rs` | `Temporal`, `temporal` |
| `crates/tabletist-db/src/dialect.rs` | `Operand::Raw`, the two values in `new_operand`, `Dialect::now` |
| `crates/tabletist-db/tests/sqlite.rs`, `postgres.rs`, `mysql.rs` | A save with DEFAULT and now, on each engine |
| `src/edit.rs` | `read`, `word`, `is_change` for the two values |
| `src/review.rs` | The keywords' ink |
| `src/app/editing.rs`, `src/app.rs`, `src/model.rs` | `typed`, `editor_problem`, `Action::SetDefault`, `Action::CycleBoolean`, the row panel's and the clipboard's text |
| `src/ui/data_view.rs` | A pending keyword's cell |
| `src/ui/keys.rs` | `Mod+'`, `D`, Space, `SHORTCUTS` |
| `src/ui/cell_editor.rs`, `src/ui/row_form.rs` | The now button (task 7) |
| `docs/_guide/editing-data.md`, `docs/_guide/macos.md`, `docs/_guide/omarchy.md`, `docs/_reference/keyboard-shortcuts.md` | The keys and the words |

---

### Task 1: The database writes DEFAULT and now

**Files:**
- Modify: `crates/tabletist-db/src/write.rs`, `class.rs`, `lib.rs`, `dialect.rs`
- Test: `crates/tabletist-db/src/dialect.rs`, `class.rs`, `crates/tabletist-db/tests/sqlite.rs`, `postgres.rs`, `mysql.rs`

- [ ] **Step 1: Write the failing tests.** In `mod tests` of `crates/tabletist-db/src/class.rs`:

```rust
    #[test]
    fn date_and_time_types_are_told_by_their_first_word() {
        use Temporal::{Date, Time, Timestamp};
        for (name, kind) in [
            ("date", Some(Date)),
            ("DATE", Some(Date)),
            ("time without time zone", Some(Time)),
            ("time(3)", Some(Time)),
            ("timetz", Some(Time)),
            ("timestamp(6) without time zone", Some(Timestamp)),
            ("timestamp with time zone", Some(Timestamp)),
            ("timestamptz", Some(Timestamp)),
            ("datetime(6)", Some(Timestamp)),
            ("DATETIME", Some(Timestamp)),
            // A name that only begins like one is no time.
            ("daterange", None),
            ("interval", None),
            ("bigint", None),
            ("text", None),
            ("", None),
        ] {
            assert_eq!(temporal(name), kind, "{name}");
        }
    }
```

  In `mod tests` of `crates/tabletist-db/src/dialect.rs`, beside `the_same_new_row_on_each_engine` (with its `sets` and `covers` helpers; `cell` below is a `CellChange`, built as the tests of `update_row` there build one):

```rust
    #[test]
    fn default_and_now_are_written_as_keywords_never_bound() {
        let key = vec![("id".to_owned(), Value::Int(2))];
        let change = |column: &str, type_name: &str, new: NewValue| CellChange {
            column: column.into(),
            type_name: type_name.into(),
            loaded: Value::Null,
            new,
        };
        let default = || NewValue::Default {
            expression: Some("'print'".into()),
        };
        for (dialect, schema, stamp, kind, day) in [
            (Dialect::Postgres, "public", "now()", "DEFAULT", "CURRENT_DATE"),
            (Dialect::MySql, "bookshop", "CURRENT_TIMESTAMP(6)", "DEFAULT", "CURRENT_DATE"),
            // SQLite has no DEFAULT to write: the expression itself.
            (Dialect::Sqlite, "main", "CURRENT_TIMESTAMP", "('print')", "CURRENT_DATE"),
        ] {
            let row = RowChange {
                key: key.clone(),
                set: vec![
                    change("kind", "varchar(20)", default()),
                    change("created_at", "timestamp", NewValue::Now),
                    change("printed_on", "date", NewValue::Now),
                ],
            };
            let update = dialect.update_row(&covers(schema), &row).unwrap();
            let literals: Vec<&str> = update.parts.values[..3]
                .iter()
                .map(|range| &update.shown[range.clone()])
                .collect();
            assert_eq!(literals, [kind, stamp, day], "{dialect:?}");
            // Shown and sent alike: only the key's value is bound.
            for literal in &literals {
                assert!(update.sql.text.contains(literal), "{dialect:?}: {literal}");
            }
            let bound = match dialect {
                Dialect::Postgres => 0,
                Dialect::MySql | Dialect::Sqlite => 1,
            };
            assert_eq!(update.sql.params.len(), bound, "{dialect:?}");
            // A new row's values are written the same way.
            let insert = RowInsert {
                set: vec![sets("created_at", "timestamp", NewValue::Now)],
            };
            let insert = dialect.insert_row(&covers(schema), &insert).unwrap();
            let range = insert.parts.literals[0].clone();
            assert_eq!(&insert.shown[range], stamp, "{dialect:?}");
            assert!(insert.sql.params.is_empty(), "{dialect:?}");
        }
        // A column with no default, on SQLite: NULL is what it would get.
        let none = RowChange {
            key: key.clone(),
            set: vec![change("note", "text", NewValue::Default { expression: None })],
        };
        let update = Dialect::Sqlite.update_row(&covers("main"), &none).unwrap();
        assert_eq!(&update.shown[update.parts.values[0].clone()], "NULL");
        // The time of the save is no value for a number.
        let wrong = RowChange {
            key,
            set: vec![change("pages", "integer", NewValue::Now)],
        };
        let error = Dialect::Postgres.update_row(&covers("public"), &wrong).unwrap_err();
        assert_eq!(
            error.to_string(),
            "pages: integer expects a number, not the time of the save"
        );
    }
```

  In `crates/tabletist-db/tests/sqlite.rs`, beside `a_new_row_comes_back_as_the_database_stored_it` and with the table and the helpers that test uses (read it first; name the columns as its table has them):

```rust
#[tokio::test]
async fn default_and_now_are_stored_as_the_database_makes_them() {
    // A table with a default and a time, one row in it: the test's own.
    // Change the row's defaulted column to something else, then save it
    // back to `NewValue::Default { expression }` (the expression as
    // `describe` gives it) and its time to `NewValue::Now`.
    // The row that comes back holds the default's value and a time that
    // is text of the form `YYYY-MM-DD HH:MM:SS`, not the word.
    // Then a new row with its time `NewValue::Now`: it comes back stored.
}
```

  Write its body with the file's own fixtures, and the same test in `postgres.rs` and `mysql.rs` with theirs (there the row comes back with a timestamp value and the default's value). The three are the proof that each engine takes the statement.

- [ ] **Step 2: Run them, and see them fail.** `~/.cargo/bin/cargo test --locked -p tabletist-db --lib` does not compile.

- [ ] **Step 3: The two values.** In `pub enum NewValue` of `crates/tabletist-db/src/write.rs`:

```rust
/// What a cell becomes: NULL, text the database turns into the column's
/// type, or what the database makes itself.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NewValue {
    Null,
    Text(String),
    /// The column's default. `expression` is the default as the catalog
    /// writes it, where the column has one: SQLite has no `DEFAULT` to
    /// write in a statement, and is sent the expression itself.
    Default { expression: Option<String> },
    /// The time the statement runs, by the database's clock: the date or
    /// the time of day where the column keeps only that.
    Now,
}
```

- [ ] **Step 4: Which columns keep a time.** In `crates/tabletist-db/src/class.rs`, and exported from `lib.rs` beside `column_class`:

```rust
/// What a date or time column keeps.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Temporal {
    Date,
    Time,
    Timestamp,
}

/// What the column whose type the catalog names `type_name` keeps of a
/// moment, by the type's first word: the three engines name these types
/// alike. `None` for every other type.
pub fn temporal(type_name: &str) -> Option<Temporal> {
    let word: String = type_name
        .trim()
        .chars()
        .take_while(char::is_ascii_alphabetic)
        .collect::<String>()
        .to_ascii_lowercase();
    match word.as_str() {
        "date" => Some(Temporal::Date),
        "time" | "timetz" => Some(Temporal::Time),
        "timestamp" | "timestamptz" | "datetime" => Some(Temporal::Timestamp),
        _ => None,
    }
}
```

- [ ] **Step 5: The statement.** In `crates/tabletist-db/src/dialect.rs`, `Operand` gains

```rust
    /// Written into the statement as it is, shown and sent alike: a
    /// keyword, or a default's expression as the catalog gave it.
    Raw(String),
```

  `shown` gets the arm `Operand::Raw(text) => text.clone(),`. In `sent`, before a parameter is pushed, `Operand::Raw(text) => return text.clone(),` beside the `Operand::Null` arm. Add to `impl Dialect`:

```rust
    /// The time a statement runs, as this engine writes it for a column
    /// that keeps `kind`.
    fn now(self, kind: Temporal) -> &'static str {
        match (self, kind) {
            (Self::Postgres | Self::MySql | Self::Sqlite, Temporal::Date) => "CURRENT_DATE",
            (Self::Postgres, Temporal::Time) | (Self::Sqlite, Temporal::Time) => "CURRENT_TIME",
            (Self::Postgres, Temporal::Timestamp) => "now()",
            // To the microsecond: without the precision MySQL gives whole
            // seconds, whatever the column keeps.
            (Self::MySql, Temporal::Time) => "CURRENT_TIME(6)",
            (Self::MySql, Temporal::Timestamp) => "CURRENT_TIMESTAMP(6)",
            (Self::Sqlite, Temporal::Timestamp) => "CURRENT_TIMESTAMP",
        }
    }
```

  In `new_operand`, move the `refused` closure above the place the value is taken apart, and replace `let NewValue::Text(text) = new else { return Ok(Operand::Null); };` with:

```rust
        let text = match new {
            NewValue::Null => return Ok(Operand::Null),
            NewValue::Default { expression } => {
                return Ok(Operand::Raw(match self {
                    Self::Postgres | Self::MySql => "DEFAULT".to_owned(),
                    // SQLite has no DEFAULT to write in an UPDATE or among
                    // VALUES: the column's own expression, which is what
                    // the word would have come to, or NULL where it has
                    // none.
                    Self::Sqlite => match expression {
                        Some(expression) => format!("({expression})"),
                        None => "NULL".to_owned(),
                    },
                }));
            }
            NewValue::Now => {
                let kind = temporal(type_name)
                    .ok_or_else(|| refused("a number, not the time of the save"))?;
                return Ok(Operand::Raw(self.now(kind).to_owned()));
            }
            NewValue::Text(text) => text,
        };
```

  The refusal's words are built by `refused` as "{column}: {type} expects {..}": make the test's sentence and this one agree, whichever reads better with the closure as it is ("pages: integer expects a date or a time for the time of the save" is as good). The binary column's refusal stays first, above all of this.

  `sent_key` in `crates/tabletist-db/src/mysql/write.rs` already takes anything but text for a value its row is not looked for by: a new row whose key column is DEFAULT or now comes back unknown, which is right.

- [ ] **Step 6: Run the crate's tests, with the servers if they can be started, then the four checks.** The app does not build until task 2: every `match` over `NewValue` in `src/` lacks two arms. Do task 2's step 3 before the workspace's checks, and commit the two tasks apart only if each compiles alone; otherwise commit them as one, saying so.

- [ ] **Step 7: Commit.**

```bash
git add crates && git commit -m "Write DEFAULT and the time of the save as keywords of a statement"
```

---

### Task 2: What was typed is read by the column

**Files:**
- Modify: `src/edit.rs`, `src/review.rs`, `src/app/editing.rs`, `src/app.rs`, `src/ui/data_view.rs`
- Test: `src/edit.rs`, `src/app.rs` (`mod editing`), `src/review.rs`

- [ ] **Step 1: Write the failing tests.** In `mod tests` of `src/edit.rs`:

```rust
    #[test]
    fn a_typed_word_is_a_keyword_where_the_column_is_not_text() {
        let pg = Dialect::Postgres;
        let column = |type_name: &str, nullable: bool, default: Option<&str>| ColumnInfo {
            name: "c".into(),
            type_name: type_name.into(),
            nullable,
            default: default.map(Into::into),
            ..ColumnInfo::default()
        };
        let text = |text: &str| NewValue::Text(text.into());
        let stamp = column("timestamp without time zone", true, Some("now()"));
        // Whatever its case, and with spaces round it.
        for typed in ["NULL", "null", " Null "] {
            assert_eq!(read(pg, &stamp, typed), NewValue::Null, "{typed}");
        }
        for typed in ["now()", "NOW()", "now", "current_timestamp"] {
            assert_eq!(read(pg, &stamp, typed), NewValue::Now, "{typed}");
        }
        let default = NewValue::Default {
            expression: Some("now()".into()),
        };
        assert_eq!(read(pg, &stamp, "default"), default);
        // A value is a value.
        assert_eq!(read(pg, &stamp, "2026-10-07 10:42:09"), text("2026-10-07 10:42:09"));
        // In a text column every word is text.
        let name = column("text", true, Some("''"));
        for typed in ["NULL", "default", "now()"] {
            assert_eq!(read(pg, &name, typed), text(typed), "{typed}");
        }
        // A keyword the column cannot take is text too: NULL where it is
        // NOT NULL, DEFAULT where there is none, now where it keeps no time.
        let count = column("integer", false, None);
        for typed in ["NULL", "default", "now()"] {
            assert_eq!(read(pg, &count, typed), text(typed), "{typed}");
        }
        // JSON has a null of its own: only the capitals are SQL's.
        let document = column("jsonb", true, Some("'{}'::jsonb"));
        assert_eq!(read(pg, &document, "null"), text("null"));
        assert_eq!(read(pg, &document, "NULL"), NewValue::Null);
        assert!(matches!(read(pg, &document, "DEFAULT"), NewValue::Default { .. }));
        assert_eq!(read(pg, &document, "default"), text("default"));
        // A boolean's own words stay text the column converts.
        let flag = column("boolean", true, None);
        assert_eq!(read(pg, &flag, "true"), text("true"));
        assert_eq!(read(pg, &flag, "null"), NewValue::Null);
    }

    #[test]
    fn default_and_now_are_always_a_change_and_have_a_word() {
        let class = ColumnClass::Other;
        let default = NewValue::Default { expression: None };
        for loaded in [Value::Null, text("2026-10-07")] {
            assert!(is_change(&loaded, &NewValue::Now, class));
            assert!(is_change(&loaded, &default, class));
        }
        assert_eq!(word(&NewValue::Now), Some("now()"));
        assert_eq!(word(&default), Some("DEFAULT"));
        assert_eq!(word(&NewValue::Null), None);
        assert_eq!(word(&NewValue::Text("now()".into())), None);
    }
```

  In `mod editing` of `src/app.rs`'s tests (the `users` table's `id` is INTEGER NOT NULL, `email` TEXT NOT NULL, `meta` JSON and nullable; the covers' `created_at` is TEXT with a default, so give a test the column it needs by changing the tab's structure, as `a_row_nothing_is_required_of_gets_the_selection_and_no_editor` does):

```rust
        #[test]
        fn a_keyword_typed_into_a_cell_is_pending_as_one() {
            let mut harness = Harness::new();
            let (tab, id) = harness.book_covers();
            // `created_at` as a database that names its types would have it.
            let structure = tab_mut(&mut harness, tab, id).structure.value.as_mut();
            let structure = structure.unwrap();
            structure.columns[4].type_name = "timestamp".into();
            structure.columns[4].nullable = true;
            type_into(&mut harness, tab, id, at(0, 4), "now()");
            type_into(&mut harness, tab, id, at(1, 4), "null");
            type_into(&mut harness, tab, id, at(2, 4), "default");
            let edits = &object(&harness, tab, id).edits;
            assert_eq!(edits.cells[&(0, 4)].new, NewValue::Now);
            assert_eq!(edits.cells[&(1, 4)].new, NewValue::Null);
            let default = NewValue::Default {
                expression: Some("CURRENT_TIMESTAMP".into()),
            };
            assert_eq!(edits.cells[&(2, 4)].new, default);
            assert!(edits.cells.values().all(|cell| cell.state == State::Ready));
            // `kind` is text: the same words are values there, and `kind`
            // takes only three.
            type_into(&mut harness, tab, id, at(0, 2), "null");
            let edits = &object(&harness, tab, id).edits;
            assert!(edits.editor.is_some(), "refused, and still open");
            // The save carries them as they are.
            harness.app.apply(Action::CancelEdit { tab, id });
            let before = harness.app.backend.sent.len();
            harness.app.apply(Action::WriteEdits { tab, id });
            let changes = write_since(&harness, before).expect("a Write");
            let sent: Vec<&NewValue> = changes.rows.iter().map(|row| &row.set[0].new).collect();
            assert_eq!(sent, [&NewValue::Now, &NewValue::Null, &default]);
        }

        #[test]
        fn default_typed_into_a_new_row_unsets_the_cell() {
            let (mut harness, tab, id, new) = with_new_row();
            let row = new_row(new);
            // `kind` is text, so the word is typed where words are read:
            // `created_at` has a default, once it is a column of times.
            let structure = tab_mut(&mut harness, tab, id).structure.value.as_mut();
            structure.unwrap().columns[4].type_name = "timestamp".into();
            type_into(&mut harness, tab, id, at(row, 4), "now()");
            assert_eq!(object(&harness, tab, id).edits.cells[&(row, 4)].new, NewValue::Now);
            type_into(&mut harness, tab, id, at(row, 4), "DEFAULT");
            assert!(!object(&harness, tab, id).edits.cells.contains_key(&(row, 4)));
        }
```

  In `mod tests` of `src/review.rs`:

```rust
    #[test]
    fn a_keyword_is_inked_as_one() {
        for literal in ["NULL", "DEFAULT", "now()", "CURRENT_TIMESTAMP", "CURRENT_TIMESTAMP(6)", "CURRENT_DATE"] {
            assert_eq!(value(literal, Values::Shown).ink, Ink::Keyword, "{literal}");
        }
        // A default's expression on SQLite, and any value, is not.
        assert_eq!(value("('print')", Values::Shown).ink, Ink::Text);
        assert_eq!(value("'DEFAULT'", Values::Shown).ink, Ink::Text);
    }
```

- [ ] **Step 2: Run them, and see them fail.**

- [ ] **Step 3: Reading what was typed.** In `src/edit.rs` (import `temporal` from `tabletist_db`), after `is_change`:

```rust
/// What `text`, typed into a cell of `column`, is as the cell's new value.
/// A word is a keyword only where the column is not text, and only where
/// the column can take it: NULL where it may be NULL, DEFAULT where it has
/// one, the time of the save where it keeps a date or a time. Everything
/// else is the text, for the column's check and the database to judge.
pub fn read(dialect: Dialect, column: &ColumnInfo, text: &str) -> NewValue {
    let typed = text.trim();
    let class = column_class(dialect, &column.type_name);
    let is = |word: &str| match class {
        // What is typed into text is the text: the four letters of NULL
        // are a value there. Nothing is typed into a binary column.
        ColumnClass::Text { .. } | ColumnClass::Binary => false,
        // A document `null` is JSON's own: only the capitals are SQL's.
        ColumnClass::Json => typed == word.to_ascii_uppercase(),
        ColumnClass::Integer { .. }
        | ColumnClass::Decimal { .. }
        | ColumnClass::Float
        | ColumnClass::Boolean
        | ColumnClass::Other => typed.eq_ignore_ascii_case(word),
    };
    if is("null") && column.nullable {
        return NewValue::Null;
    }
    if is("default") && column.default.is_some() {
        return NewValue::Default {
            expression: column.default.clone(),
        };
    }
    let now = ["now()", "now", "current_timestamp"];
    if now.into_iter().any(is) && temporal(&column.type_name).is_some() {
        return NewValue::Now;
    }
    NewValue::Text(text.to_owned())
}

/// The word a value that is no value yet is shown as, until the database
/// has made it. `None` for NULL, which a cell draws as it draws any NULL,
/// and for text.
pub fn word(new: &NewValue) -> Option<&'static str> {
    match new {
        NewValue::Default { .. } => Some("DEFAULT"),
        NewValue::Now => Some("now()"),
        NewValue::Null | NewValue::Text(_) => None,
    }
}
```

  In `is_change`, before the `NewValue::Text` arms: `NewValue::Default { .. } | NewValue::Now => true,` with the comment "What the database will make is not what the cell holds, whatever that is."

- [ ] **Step 4: The editor.** In `typed` of `src/app/editing.rs`, the value and what stands against it:

```rust
    let new = crate::edit::read(table.dialect, column, &editor.text);
    let changed = match (&new, loaded) {
        // A new row's unset cell is its default already.
        (NewValue::Default { .. }, None) => false,
        (new, loaded) => loaded.is_none_or(|loaded| is_change(loaded, new, class)),
    };
    // Only text has a check: a keyword is the database's own.
    let problem = match &new {
        NewValue::Text(text) if changed => check(table.dialect, column, text),
        _ => None,
    };
```

  `editor_problem` checks the same way: `match crate::edit::read(table.dialect, column, &editor.text) { NewValue::Text(text) => check(table.dialect, column, &text), _ => None }`. In `edit_cell`, an editor opened on a cell that is pending a keyword starts as one opened on a pending NULL does, empty and untouched: `NewValue::Null | NewValue::Default { .. } | NewValue::Now => String::new(),`. And where it works out the problem of the text it opens with, use the same `match` as `editor_problem`.

- [ ] **Step 5: Everything else that reads a new value.** The compiler names each `match` over `NewValue` that lacks the two arms. What each becomes:

  - `drawn` in `src/ui/data_view.rs` (a pending value as a value a cell draws): `NewValue::Default { .. } | NewValue::Now => Value::Text(crate::edit::word(new).unwrap_or_default().into())`. Task 3 gives the cell its look.
  - `Changes::mark_state` (`typed`, for a problem's words): `None` for the two, as for NULL.
  - `error_line` in the same file: the same.
  - `row_fields` and `copy_text` in `src/app.rs`: the word as text, as `drawn` has it.
  - `shown_lines` in `src/edit.rs` (the conflict question's "yours"): `Some(word)`.

  In `value` of `src/review.rs`, a keyword's ink:

```rust
    let keyword = matches!(literal, "NULL" | "DEFAULT" | "now()")
        || literal.starts_with("CURRENT_");
    let ink = if keyword {
        Ink::Keyword
```

  and add the words to the doc of `Ink::Keyword`.

- [ ] **Step 6: Run the tests, then the four checks.**

- [ ] **Step 7: Commit.**

```bash
git add src && git commit -m "Read NULL, DEFAULT and now() typed into a cell as what they say, where the column is not text"
```

---

### Task 3: A pending keyword looks like one

**Files:**
- Modify: `src/ui/data_view.rs`
- Test: `src/ui/mod.rs`

- [ ] **Step 1: Write the failing test.** In `mod tests` of `src/ui/mod.rs`, with `covers_in` and `make_pending`:

```rust
    #[test]
    fn a_pending_default_and_now_are_shown_as_words_until_saved() {
        for look in Look::ALL {
            let (mut harness, tab, id) = covers_in(look);
            harness.app.workspace_mut(tab).unwrap().row_panel = false;
            let workspace = harness.app.workspace_mut(tab).unwrap();
            let structure = workspace.object_tab_mut(id).unwrap().structure.value.as_mut();
            structure.unwrap().columns[4].type_name = "timestamp".into();
            make_pending(&mut harness, tab, id, (0, 4), "now()");
            make_pending(&mut harness, tab, id, (1, 4), "default");
            harness.settle();
            let palette = harness.app.palette;
            // The words, quieter than a value: what the database will make.
            for word in ["now()", "DEFAULT"] {
                assert!(painted(&harness, word), "{}: {word}", look.name);
                assert!(!painted_in(&harness, word, palette.text), "{}: {word}", look.name);
            }
            // The cells are pending like any other.
            assert!(harness.has("2 changes in 2 rows") || look.terminal, "{}", look.name);
        }
    }
```

  Take the colour a `Style::Quiet` cell is written in from the grid (`draw_cell`), and assert that colour if "not the text's" proves too weak.

- [ ] **Step 2: Run it, and see it fail:** the words are painted in the text's colour.

- [ ] **Step 3: The cell.** In `show` of `src/ui/data_view.rs`, where a pending cell of a page's row is drawn, and in `Changes::new_cell` where a set cell of a new row is:

```rust
/// A pending value's cell: the value as `value` draws it, or, for what
/// the database will make at the save, its word in the style of what
/// stands for a value.
fn pending_cell<'c>(new: &NewValue, value: impl Fn(&Value) -> Cell<'c>) -> Cell<'c> {
    match crate::edit::word(new) {
        Some(word) => Cell {
            text: word.into(),
            style: Style::Quiet,
            ..Cell::default()
        },
        None => value(&drawn(new)),
    }
}
```

  Both places call it where they call `drawn` today. A pending DEFAULT says what it will come to under the pointer: in `mark_state`, for `State::Ready`, when the value is `NewValue::Default { expression: Some(expression) }`, the hint is the expression as `edit::default_shown` reads it, then "from DEFAULT" (the words a new row's unset cell has), after the "was …" line where the cell has one.

- [ ] **Step 4: Run the test, then the four checks.**

- [ ] **Step 5: Commit.**

```bash
git add src && git commit -m "Show a pending DEFAULT and now() as the words they are"
```

---

### Task 4: Set DEFAULT

**Files:**
- Modify: `src/model.rs`, `src/app.rs`, `src/app/editing.rs`, `src/ui/keys.rs`
- Test: `src/app.rs` (`mod editing`), `src/ui/mod.rs`, `src/ui/keys.rs`

- [ ] **Step 1: Write the failing tests.** In `mod editing`:

```rust
        #[test]
        fn set_default_gives_a_cell_its_columns_default() {
            let mut harness = Harness::new();
            let (tab, id) = harness.book_covers();
            // `kind` has one: text or not, the key sets it.
            let cell = at(1, 2);
            harness.app.apply(Action::SelectCell { tab, id, cell });
            harness.app.apply(Action::SetDefault { tab, id });
            let default = NewValue::Default {
                expression: Some("'print'".into()),
            };
            assert_eq!(object(&harness, tab, id).edits.cells[&(1, 2)].new, default);
            // `publisher_id` has none, and `id` is locked: nothing happens.
            for col in [1, 0] {
                let cell = at(1, col);
                harness.app.apply(Action::SelectCell { tab, id, cell });
                harness.app.apply(Action::SetDefault { tab, id });
            }
            assert_eq!(object(&harness, tab, id).edits.cells.len(), 1);
            // In a new row it unsets the cell: unset is its default.
            let place = Place::Top;
            harness.app.apply(Action::AddRow { tab, id, place });
            let new = object(&harness, tab, id).edits.added[0].id;
            type_into(&mut harness, tab, id, at(new_row(new), 2), "ebook");
            harness.app.apply(Action::SetDefault { tab, id });
            assert!(!object(&harness, tab, id).edits.cells.contains_key(&(new_row(new), 2)));
        }
```

  In `mod tests` of `src/ui/mod.rs`: `Mod+'` on the desktop looks and `D` in the terminal's each make the selected `kind` cell pending DEFAULT (`pending_text` gains the two words: see step 4), and a `D` followed by a `d` does not drop a new row (the capital is no half of `dd`):

```rust
    #[test]
    fn mod_quote_and_capital_d_set_default() {
        for look in Look::ALL {
            let (mut harness, tab, id) = covers_in(look);
            select(&mut harness, tab, id, (1, 2));
            if look.terminal {
                harness.frame(vec![
                    crate::testing::key(Key::D, Modifiers::SHIFT),
                    egui::Event::Text("D".into()),
                ]);
                harness.frame(vec![crate::testing::release(Key::D, Modifiers::SHIFT)]);
                harness.settle();
            } else {
                harness.press(Key::Quote, Modifiers::COMMAND);
            }
            let pending = pending_text(&harness, tab, id, (1, 2));
            assert_eq!(pending.as_deref(), Some("DEFAULT"), "{}", look.name);
        }
        // The capital is not the first `d` of `dd`.
        let (mut harness, tab, id) = covers_in(Look::omarchy());
        select(&mut harness, tab, id, (0, 2));
        type_key(&mut harness, Key::O, "o");
        harness.press(Key::Escape, Modifiers::NONE);
        harness.frame(vec![
            crate::testing::key(Key::D, Modifiers::SHIFT),
            egui::Event::Text("D".into()),
        ]);
        harness.frame(vec![crate::testing::release(Key::D, Modifiers::SHIFT)]);
        type_key(&mut harness, Key::D, "d");
        assert_eq!(edits(&harness, tab, id).added.len(), 1);
    }
```

- [ ] **Step 2: Run them, and see them fail.**

- [ ] **Step 3: The action.** In `Action` of `src/model.rs`, after `SetNull`: `/// Give the active cell its column's default, where it has one.` `SetDefault { tab: ConnTabId, id: TabId },`. It joins `dropped_under_a_prompt`, and `App::apply` sends it to:

```rust
    /// Gives the selected cell its column's default, where the column has
    /// one. In a new row that is the cell with nothing set in it.
    pub(super) fn set_default(&mut self, tab: ConnTabId, id: TabId) {
        let verdict = self.table(tab, id, |table, object| {
            let cell = object.selection?;
            if object.edits.editor.is_some() || table.lock(cell).is_some() {
                return None;
            }
            let column = table.column(cell.col)?;
            let expression = column.default.clone()?;
            // A new row's unset cell is its default already.
            let new = table.loaded(cell).map(|_| NewValue::Default {
                expression: Some(expression),
            });
            Some((cell, new))
        });
        let Some(Some((cell, new))) = verdict else {
            return;
        };
        let Some(object) = self.object_tab_mut(tab, id) else {
            return;
        };
        let key = (cell.row, cell.col);
        match new {
            Some(new) => {
                let state = State::Ready;
                object.edits.put(key, Pending { new, state });
                object.pinned = true;
            }
            None => object.edits.revert(key),
        }
        object.fields = None;
    }
```

- [ ] **Step 4: The keys.** In `editing_keys` of `src/ui/keys.rs`, beside `Mod+Backspace`:

```rust
        if take_press(input, Modifiers::COMMAND, Key::Quote) > 0 {
            on_cell(actions);
            actions.push(Action::SetDefault { tab, id });
        }
```

  In `editing_letters`, `D` joins `mine`, with the arm `"D" => { on_cell(actions); actions.push(Action::SetDefault { tab, id }); }`. In `letters`, the `d` arm reads `pressed(Key::D)`, which lets a held Shift through: make it `pressed(Key::D) && !ctx.input(|input| input.modifiers.shift)`, with the comment that the capital is Set DEFAULT's. `SHORTCUTS` gains `("Mod+'", "Set DEFAULT", DESKTOP)` and `("D", "Set DEFAULT", TERMINAL)` after the two Set NULL rows, `D` joins the long row of Omarchy's keys, and `the_shortcut_table_names_the_keys_that_edit_a_cell` asserts both as it asserts Set NULL's. The tests' `pending_text` helper says `"DEFAULT"` and `"now()"` for the two values (`crate::edit::word`).

- [ ] **Step 5: Run the tests, then the four checks.**

- [ ] **Step 6: Commit.**

```bash
git add src && git commit -m "Set a cell to its column's DEFAULT with Mod+' and the terminal's D"
```

---

### Task 5: Space flips a boolean

**Files:**
- Modify: `src/model.rs`, `src/app.rs`, `src/app/editing.rs`, `src/ui/keys.rs`
- Test: `src/app.rs` (`mod editing`), `src/ui/mod.rs`

- [ ] **Step 1: Write the failing tests.** The fixtures have no boolean column: a test gives the covers one by changing the tab's structure (`columns[2].type_name = "BOOLEAN"`, no allowed values, nullable or not) and the page's values in that column (`Value::Int(1)`, `Value::Int(0)`, `Value::Null`: SQLite keeps a flag as a number). In `mod editing`:

```rust
        #[test]
        fn a_boolean_cell_cycles_true_false_and_null() {
            // With the column as above, nullable, row 0 holding 1:
            // CycleBoolean on (0, 2) gives pending "false", then NULL,
            // then nothing pending (true is what it loaded).
            // With the column NOT NULL: "false", then nothing pending.
            // On a cell that loaded NULL: "true", "false", nothing.
            // On a new row's cell: "true", "false", NULL, "true": it is
            // never unset by cycling.
            // On a cell of another class, a locked one, and with an editor
            // open: nothing.
        }
```

  Write it out with `pending_text`-like reads of `edits.cells`. In `src/ui/mod.rs`: Space on that cell flips it in every look and leaves the row panel as it was, and Space on a cell of another column toggles the panel as today.

- [ ] **Step 2: Run them, and see them fail.**

- [ ] **Step 3: The action.** `/// Flip the active cell where it is a boolean's: true, false, and NULL where the column takes it.` `CycleBoolean { tab: ConnTabId, id: TabId },` in `Action`, in `dropped_under_a_prompt`, and:

```rust
    /// Whether the tab's selected cell is a boolean's that can be edited
    /// now: where Space flips the cell.
    pub fn flips(&self, tab: ConnTabId, id: TabId) -> bool {
        self.table(tab, id, |table, object| {
            let Some(cell) = object.selection else {
                return false;
            };
            object.edits.editor.is_none()
                && table.class(cell.col) == Some(ColumnClass::Boolean)
                && table.lock(cell).is_none()
        })
        .unwrap_or(false)
    }

    /// Flips the selected boolean cell: true, then false, then NULL where
    /// the column takes it, then true again. A value that is what the cell
    /// loaded is no change.
    pub(super) fn cycle_boolean(&mut self, tab: ConnTabId, id: TabId) {
        if !self.flips(tab, id) {
            return;
        }
        let next = self.table(tab, id, |table, object| {
            let cell = object.selection?;
            let column = table.column(cell.col)?;
            let loaded = table.loaded(cell);
            // What the cell holds now: what is pending in it, or what it
            // loaded. Anything that is no flag is taken for none.
            let flag = |text: &str| match text.trim().to_ascii_lowercase().as_str() {
                "true" | "1" => Some(true),
                "false" | "0" => Some(false),
                _ => None,
            };
            let now = match object.edits.cells.get(&(cell.row, cell.col)) {
                Some(pending) => match &pending.new {
                    NewValue::Text(text) => flag(text),
                    NewValue::Null | NewValue::Default { .. } | NewValue::Now => None,
                },
                None => loaded.and_then(|loaded| flag(&start_text(loaded, ColumnClass::Boolean))),
            };
            let new = match now {
                Some(true) => NewValue::Text("false".into()),
                Some(false) if column.nullable => NewValue::Null,
                Some(false) | None => NewValue::Text("true".into()),
            };
            let changed =
                loaded.is_none_or(|loaded| is_change(loaded, &new, ColumnClass::Boolean));
            Some((cell, new, changed))
        });
        let Some(Some((cell, new, changed))) = next else {
            return;
        };
        let Some(object) = self.object_tab_mut(tab, id) else {
            return;
        };
        let key = (cell.row, cell.col);
        if changed {
            let state = State::Ready;
            object.edits.put(key, Pending { new, state });
            object.pinned = true;
        } else {
            object.edits.revert(key);
        }
        object.fields = None;
    }
```

  `start_text` writes a loaded flag as `true` or `false` on every engine (`Value::Bool`, and `Value::Int` 1 and 0 for a boolean column), which is what `flag` reads.

- [ ] **Step 4: The key.** In `handle` of `src/ui/keys.rs`, Space is one line, `key(Modifiers::NONE, Key::Space, Action::ToggleRowPanel(tab))`, under `if grid || sql_row`. Beside `adds`, before the input is read:

```rust
    // Space flips a boolean cell where the grid has the keys and the cell
    // can be edited, as the design has it. Everywhere else it shows the
    // row panel.
    let flips = object
        .filter(|_| grid && !editing && !tree_arrows && !focused)
        .filter(|&(tab, id)| app.flips(tab, id));
```

  and the line becomes

```rust
                    let space = match flips {
                        Some((tab, id)) => Action::CycleBoolean { tab, id },
                        None => Action::ToggleRowPanel(tab),
                    };
                    key(Modifiers::NONE, Key::Space, space);
```

  (`tree_arrows` and `focused` are computed further down in `handle` today: move `flips` below them, or them above it.) `SHORTCUTS` gains `("Space", "Flip a boolean cell", ALL)` beside the row that names Space for the row panel, and a test of the table names it.

- [ ] **Step 5: Run the tests, then the four checks.** A test that presses Space on a boolean cell to show the row panel does not exist: the fixtures have no boolean column.

- [ ] **Step 6: Commit.**

```bash
git add src && git commit -m "Flip a boolean cell with Space"
```

---

### Task 6: The pages

**Files:**
- Modify: `docs/_guide/editing-data.md`, `docs/_guide/macos.md`, `docs/_guide/omarchy.md`, `docs/_reference/keyboard-shortcuts.md`

- [ ] **Step 1.** In `docs/_guide/editing-data.md`, the table under "Change a value" gains "Set the value to the column's DEFAULT | Cmd/Ctrl+'" and "Flip a boolean | Space", and after it a short section `### Words that are not text`, saying, from the code as built:

  - In a column that is not text, `NULL`, `DEFAULT` and `now()` typed into a cell are those, not the letters. In a text column they are the text.
  - `now()` is the database's own clock at the moment of the save, in date and time columns. The cell shows the word until then, and the stored time after.
  - `DEFAULT` works where the column has a default. In a new row, a cell left alone is its default already.
  - In a JSON column `null` is a JSON document, and `NULL` in capitals is the database's.

  The two key tables in `macos.md` and `keyboard-shortcuts.md` gain the two rows (`Cmd+'`, `Mod+'`; Space), and `omarchy.md` gains `D` after `x`, and Space.

- [ ] **Step 2.** `cd docs && bin/check` passes.

- [ ] **Step 3: Commit.**

```bash
git add docs/_guide docs/_reference && git commit -m "Say in the pages what NULL, DEFAULT and now() typed into a cell are"
```

---

### Task 7: The now button

The canvas draws a **now** button beside a date or time cell's editor. Tasks 1 to 6 give the same value by typing the word; this task is the button, and can be its own pull request.

**Files:**
- Modify: `src/ui/cell_editor.rs` (`Target`, `Outcome`, `field`, `large`), `src/ui/data_view.rs` (`editor_target`, where the outcome becomes actions), `src/ui/row_form.rs` and `src/ui/row_panel.rs` (the panel's field), `src/model.rs`, `src/app/editing.rs`
- Test: `src/ui/mod.rs`

- [ ] **Step 1: Read first.** `cell_editor::field` draws the field on a cell and returns an `Outcome` (`changed`, `commit`, `cancel`, `left`, `large`); `Target` is what it knows of the cell (`name`, `type_name`, `max_chars`, `json`, `hold`). The row panel's field goes through `row_form`. Read both, and the board again ("macOS – Editing values, editors by type", the timestamp card) with the Artifact tool, and the Components boards for the button's states.

- [ ] **Step 2: The test.** An editor open on a date or time column shows a button named "now"; pressing it closes the editor and leaves the cell pending `NewValue::Now`; an editor on any other column has no such button; the keyboard reaches it with Tab from the field, and Enter presses it. In the three looks, in the grid and in the row panel.

- [ ] **Step 3: Build it.** `Target` gains `now: bool` (`tabletist_db::temporal` of the column's type is `Some`, and the cell is no key of a kind the engine cannot match). `Outcome` gains `now: bool`. The button stands at the field's right end, inside the cell's rectangle, in the look's small button; the field is that much narrower. `Action::SetNow { tab, id }` closes the editor without its text and puts `NewValue::Now` in its cell, as `set_null` puts NULL: it joins `dropped_under_a_prompt`.

- [ ] **Step 4: See it.** Add a throwaway scene to `src/shots.rs` with the editor open on `created_at`, render it (`cargo test --locked --features shots --lib <its name> -- --ignored`), and compare it with the board by eye in the three looks. The pictures stay in `target/`.

- [ ] **Step 5: Run the tests and the four checks, and commit.**

```bash
git add src && git commit -m "Offer now beside a date or time cell's editor"
```

---

## After the last task

- Run the four checks on the branch's head, the shots' clippy, and the three database suites with their variables set. Report what ran and what was only compiled.
- By hand, for the user, on the Bookshop's development database in each look:
  1. A new row on a Rails table: type `now()` into `created_at` and `updated_at`, Save. Both show the stored time.
  2. Type `NULL` into a nullable number or date column: the NULL chip, pending. Into a text column: the four letters.
  3. `Mod+'` (or `D`) on a cell whose column has a default, Save: the default's value.
  4. Space on a boolean cell three times, then on any other cell.
  5. Review SQL with one of each: the keywords, not quoted.
- Stop here. The other editors by type (the enum's list, the foreign key's search, the calendar, the array's chips) are slice 2's own plan.
