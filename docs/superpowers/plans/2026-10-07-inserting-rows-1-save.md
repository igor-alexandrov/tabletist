# Inserting Rows, Run 1: The Save Inserts Rows Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A save can carry new rows beside changed ones: `tabletist-db` builds one `INSERT` per new row, runs them in the save's one transaction before its `UPDATE`s on PostgreSQL, MySQL and SQLite, and hands each new row back as the database stored it. The structure says which columns the database numbers itself.

**Architecture:** `ChangeSet` gains `inserts: Vec<RowInsert>` beside `rows`, so there is still one set, one `Connection::write` and one transaction. `Dialect::insert_row` builds the statement a user reads and the one the driver sends from the same values, as `update_row` does. Each driver runs the inserts after its conflict check and before its updates; PostgreSQL and SQLite read the new row from `RETURNING *`, MySQL finds it again by its primary key. `WriteOutcome::Written` gains `inserted`, and a failed `INSERT` is `WriteOutcome::FailedInsert`. The app sends no insert yet: this run changes nothing a user sees.

**Tech Stack:** Rust 1.98, `tokio-postgres`, `mysql_async`, `rusqlite` (bundled SQLite). Spec: `docs/superpowers/specs/2026-10-07-inserting-rows-design.md`.

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

- `src/shots.rs` is outside those four and builds a `ColumnInfo`, a `ChangeSet` and a `WriteOutcome` by hand. Tasks 1 and 2 touch it, so they also run `~/.cargo/bin/cargo clippy --locked --features shots --lib --tests -- -D warnings`.
- **What was run where this plan was written: nothing.** It was written from reading the tree at `cc79285` and the design canvas, not from a draft that was built. Every code block is written against names that exist in that tree, but none was compiled. Treat a block as what the code should come to, and expect to correct a name or a borrow on the first build. Where a block and the compiler disagree, the compiler is right and the task's tests say what must hold. A reviewer then read the plan against the tree, without building it, and found five faults, all corrected here: two things that would have been dead code for a commit, two exact strings of `ChangeSet`'s `Debug` that existing tests assert, a test that leaned on the fixture's sequence, a test its filter did not run, and a save of new rows alone that asked MySQL for the table's engine before it held the table.
- **Databases.** Tasks 1, 5 and 6 add tests that need servers. Start them once, and run the suites with the variables set:

```bash
docker compose up -d --build --wait postgres mysql
export TABLETIST_TEST_PG_URL=postgres://tabletist:tabletist@localhost:55432/tabletist
export TABLETIST_TEST_MYSQL_URL=mysql://tabletist:tabletist@localhost:53306/tabletist
```

  Without the variables those tests print "skipped" and pass, which proves nothing. If the servers cannot be started in the session, say so in the task's report and in the pull request: "PostgreSQL and MySQL tests were compiled, not run; CI runs them."
- **The branch.** `claude/inserting-rows-tables-9f7499`, at `cc79285`. Before task 1, commit the spec and this plan on their own.
- House rules that bite here: no em dashes anywhere; comments explain why, in the surrounding code's voice; code that depends on the engine matches on `Dialect` and names every variant (no `==`, `!=`, `matches!` or `_` arm; `tests/engines.rs` finds the first three); `crates/tabletist-db` has no UI dependency; what a user typed never reaches a log (`ChangeSet`'s `Debug` prints counts, not values); do not weaken a lint or delete a test to get green.
- Commits are signed, one per task, after its checks pass. If signing fails ("agent refused operation"), do not commit unsigned: stage the task and tell the user. Each message ends with:

      Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>

## The runs

The spec is one feature. It is planned as runs that each work alone, each with its own plan, as the row inspector was.

| Run | What it builds | Seen by a user |
|---|---|---|
| **1 (this plan)** | The save inserts rows: column identity in the structure, `INSERT` in the change set, the three drivers, what comes back | No |
| 2 | Add row in the grid: pending new rows in the one pending set, the Add row button, `Mod+N`, `o`/`O`, the green row with its required, default and assigned cells, the pending bar, Review SQL, the save and the failed save, the row panel as the "New row" form | Yes |
| 3 | Duplicate (`Mod+D`, `yy p`) and growing by Down on the last row | Yes |
| 4 | Paste rows with its preview, and the batched multi-row `INSERT` | Yes |
| 5 | Errors and after the save: a failed row's message on its cell by error code, "Open row", `]e`/`[e` and `Mod+'`, "Show in sorted position" and `gs`, the filter note | Yes |

Run 2 is the first a user sees, and the longest: about as much work as the row form's fields were. Its plan is written when run 1 is in, from the tree as it then stands.

## What the design asks, and what gets built

Where the app cannot yet do what the canvas and the spec ask, or where they leave a choice. Each is the user's to overrule, before run 1 for the rows marked "1", before run 2 for the rest.

| | The design and the spec | What gets built | Why | Run |
|---|---|---|---|---|
| a | Values are always bound parameters | PostgreSQL sends them as literals in the statement's text; MySQL and SQLite bind them | It is how the save's `UPDATE` goes today: PostgreSQL rows are read through the simple-query protocol, so a loaded value and a saved one are the same `Value`. One builder writes the shown and the sent statement from the same operands | 1 |
| b | `VALUES (9100000000000000004)` | On PostgreSQL `VALUES ('9100000000000000004')`: every new value is a text literal the server converts | As the `UPDATE`'s `SET` writes its values today. SQLite converts by the column's class first and shows the number bare | 1 |
| c | SQLite older than 3.35 uses `last_insert_rowid()` | `RETURNING *` only | SQLite is bundled with the app (`rusqlite`'s `bundled`), so its version is the app's own | 1 |
| d | MySQL: `LAST_INSERT_ID()`, then `SELECT * … WHERE pk = ?`; without an auto-increment key, by the key values sent | The same. Where the row cannot be found again for sure (no primary key, a key column the database filled another way, a key of a type MySQL cannot match exactly: `float`, `timestamp`, `bit`), the row is written and handed back as unknown, and the app will load the page again | A guess at the row would show another row's values as the new one's | 1 |
| e | Batch up to 100 rows into one multi-row `INSERT` | One `INSERT` per row | Only paste makes rows by the hundred. Batching comes with it | 4 |
| f | Identity and serial columns are locked in a new row | The structure gains `identity`: an identity column (always or by default), a serial, `AUTO_INCREMENT`, SQLite's alias of the rowid. Run 2 locks them, so an id cannot be typed for a new row | The spec says locked. Say so if a typed id should be allowed where the database allows one | 1, 2 |
| g | Tests use only the Bookshop data | The app's tests do (run 2 adds a `book_covers` with `publisher_id`). The database crate's tests keep the crate's own fixture and their own tables, as every write test there does, with Bookshop names where a table is made | The crate's fixture is not Bookshop's and is not screenshot material | 1 |
| h | `Mod+N` adds a row | `Mod+N` adds a row while a table's Data view is in front, and stays "New connection" elsewhere | It is "New connection" in every place today. The canvas' Keyboard settings scope it: "Connections" for one, "Table" for the other | 2 |
| i | The new row is pinned under the header "regardless of scroll position" | New rows are the grid's first rows, above the page's, and the grid scrolls to them when one is made. They scroll with the body | Read with the spec's "The grid scrolls to the top if needed". A band that stays put while the body scrolls is a second grid inside the grid | 2 |
| j | An expression default in italics (`now()`) | Dimmed, not italic | The app has no italic face: text is drawn only through `TextRole`s, and none is italic | 2 |
| k | A foreign key column opens the search picker | The text editor | The grid has one editor. Editors by type are slice 2 of the value editing spec, for the grid and the panel at once | later |
| l | Undo and redo cover creating and dropping new rows | `Mod+Z` and `u` put one cell back to unset. A dropped new row is not brought back | The undo stack is slice 4 | later |
| m | Inserts go before the UPDATEs and DELETEs | Before the UPDATEs | The app deletes no rows yet | later |
| n | Hidden when the user has no INSERT privilege, or for a view without INSERT rules | Disabled for a read-only connection, a view and a query's result. A missing privilege is the server's error on save | The app reads no privileges and edits no view | 2 |
| o | "Already used by row id 101 · Open row", "No publisher with id …", each on its cell | Run 2 shows the server's message on the new row. The cell, the wording by code and "Open row" are run 5 | `Error` carries a code and a message, no column or constraint name, and MySQL's error number is dropped for its SQLSTATE | 5 |
| p | "Save 1 insert and 1 update to bookshop_production?" | "Save 1 new row and 1 change to production?", the connection on the line under it | The prompt says "Save 2 changes to production?" today, and the app's word is "change" | 2 |

## What was decided for this run

1. **One set, two lists.** `ChangeSet { object, inserts, rows }`. A set with only inserts is a save; a set with neither is refused as before.
2. **Order inside the transaction.** Lock and compare every changed row; a conflict ends the save before anything is written, new rows included. Then every `INSERT`, in the set's order. Then every `UPDATE`. Then the changed rows are read back. A review shows inserts first because they run first.
3. **What comes back.** `WriteOutcome::Written { inserted, rows, elapsed }`, `inserted` in the set's order, each `Some(row)` in the table's column order as a page reads it, or `None` when MySQL cannot find the row again (row d above).
4. **A failed `INSERT`** is `WriteOutcome::FailedInsert { insert, error }` with the insert's place in the set. Nothing was written. `WriteOutcome::Failed { row, .. }` keeps meaning a changed row.
5. **A new row with nothing set** is `INSERT INTO t DEFAULT VALUES RETURNING *` on PostgreSQL and SQLite, and ``INSERT INTO t () VALUES ()`` on MySQL.
6. **What a new value is.** `InsertValue { column, type_name, new }`: the same `NewValue` a changed cell carries, converted by the same rules (`Dialect::new_operand`), without a loaded value. A binary column is refused as it is for a change.
7. **The app until run 2.** It builds sets with `inserts: Vec::new()`, compares `inserts` in `same_changes`, ignores `inserted`, and takes a `FailedInsert` as a save the database refused. No path sends an insert.

8. **After the pull request's review.** A new row is handed back only where it is known for sure. `RETURNING` gives a row as its `INSERT` left it, before an AFTER trigger ran, and a trigger can move a row's key, so no read finds the row again for sure either. So on a table with a trigger (on PostgreSQL also one with a rule, or one that is not an ordinary table) every new row comes back as `None`, on all three engines, and the app will load the page again. Without one, PostgreSQL and SQLite hand back what `RETURNING` gave, and MySQL finds the row by its key as before. The spec's "the row takes the values from `RETURNING` (… triggers' changes)" and "the row stays in place" therefore hold for a table without triggers; with one, run 2 reloads the page after the save. The user may loosen this. The tasks below show the first shape. Also from the review: PostgreSQL's `identity` asks that the default draws on the sequence the column owns, SQLite's leaves out `INTEGER PRIMARY KEY DESC`, and MySQL matches a sent key column without regard to case. MySQL lists a table's triggers only to a user with the TRIGGER privilege on it, so its save first establishes that privilege from the server's lists of grants, and without it hands every new row back as `None`. `compose.yaml` lets the tests' MySQL user make a trigger, and `compose/mysql-init.sql` makes a user without TRIGGER. An `INSERT` that goes through and returns no row (a BEFORE trigger took the row for itself) is no failure: the save is written and that row is `None`. Left for later: a target that takes an `INSERT` and refuses `RETURNING` (a SQLite virtual table, a PostgreSQL table behind a `DO INSTEAD` rule, a foreign table) fails its save with the database's own message; a plain `INSERT` for those needs the table's kind before the statement is built.

## The spec's tests, and where each is

| Spec test | Run | In this run |
|---|---|---|
| 3 untouched defaults absent from the INSERT; `DEFAULT VALUES` when nothing was set | 1 | Task 3: `a_new_row_names_only_what_was_set`, `a_new_row_with_nothing_set_takes_every_default` |
| 4 identity and generated never in the INSERT | 1, 2 | Task 1 gives the structure `identity`. What the INSERT names is what the set holds (task 3); run 2 keeps locked columns out of the set |
| 8 one transaction, inserts first, a unique violation rolls everything back | 1, 2 | Tasks 4 to 6: `a_new_row_that_fails_undoes_the_rows_before_it`. The cells left pending are run 2 |
| 9 the saved row equals `RETURNING` or the re-select, with the real id and `created_at` | 1, 2 | Tasks 4 to 6: `a_new_row_comes_back_as_the_database_stored_it` |
| 12 a table without a primary key accepts an insert | 1, 2 | Tasks 4 to 6: `a_table_without_a_key_takes_a_new_row`. "Not editable" afterwards is run 2 |
| 13 the SQL for the three engines matches the expected strings | 1 | Task 3: `the_same_new_row_on_each_engine` |
| 1, 2, 10, 11 | 2 | |
| 5 | 3 | |
| 6, 7 | 4 | |

## File map

| File | What changes |
|---|---|
| `crates/tabletist-db/src/catalog.rs` | `ColumnInfo::identity` |
| `crates/tabletist-db/src/pg.rs`, `mysql.rs`, `sqlite.rs` | `describe` fills `identity`; SQLite's in-crate test |
| `crates/tabletist-db/src/write.rs` | `RowInsert`, `InsertValue`, `ChangeSet::inserts`, `check`, `Debug`; `WriteOutcome::Written::inserted`, `WriteOutcome::FailedInsert`; `Stored` (task 2); `Applied::FailedInsert`, `named_twice` (task 4) |
| `crates/tabletist-db/src/dialect.rs` | `InsertStatement`, `Dialect::insert_row`; `new_operand` takes its parts |
| `crates/tabletist-db/src/lib.rs` | exports |
| `crates/tabletist-db/src/sqlite/write.rs`, `pg/write.rs`, `mysql/write.rs` | the inserts of a save |
| `crates/tabletist-db/tests/sqlite.rs`, `postgres.rs`, `mysql.rs` | the tests of the above |
| `src/edit.rs`, `src/app/editing.rs`, `src/backend.rs`, `src/review.rs`, `src/ui/review.rs`, `src/ui/mod.rs`, `src/app.rs`, `src/shots.rs` | compile with the new fields; `same_changes` compares `inserts`; `written` takes the new outcome |
| `docs/superpowers/specs/2026-10-07-inserting-rows-design.md` | its status line, in task 7 |

---

### Task 1: The structure says which columns the database numbers

`ColumnInfo` gains `identity`: the database gives the column its value from a counter of its own when an insert names none. It is not `generated` (a `GENERATED BY DEFAULT` identity, a serial and an `AUTO_INCREMENT` column take a value when one is sent), and it is not a default the user can read (`nextval('…')` says nothing to them).

**Files:**
- Modify: `crates/tabletist-db/src/catalog.rs` (`ColumnInfo`)
- Modify: `crates/tabletist-db/src/pg.rs` (`describe`, the column query and its mapping, near line 533)
- Modify: `crates/tabletist-db/src/mysql.rs` (`describe`, the mapping near line 251)
- Modify: `crates/tabletist-db/src/sqlite.rs` (`columns`, near line 965; tests)
- Modify: every full `ColumnInfo { .. }` literal the compiler names (`src/edit.rs`, `src/ui/mod.rs`, `src/ui/value_tags.rs`, `src/shots.rs`)
- Test: `crates/tabletist-db/src/sqlite.rs` (tests), `crates/tabletist-db/tests/postgres.rs`, `crates/tabletist-db/tests/mysql.rs`

- [ ] **Step 1: Write the failing tests.**

In `crates/tabletist-db/src/sqlite.rs`, after the test `a_generated_column_says_so`:

```rust
    #[tokio::test]
    async fn the_rowids_alias_is_an_identity_column() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("identity.db");
        rusqlite::Connection::open(&path)
            .unwrap()
            .execute_batch(
                "CREATE TABLE books (id INTEGER PRIMARY KEY, title TEXT NOT NULL);
                 CREATE TABLE tags (name TEXT PRIMARY KEY, id INTEGER);
                 CREATE TABLE pairs (a INTEGER, b INTEGER, PRIMARY KEY (a, b));
                 CREATE TABLE codes (id INTEGER PRIMARY KEY, label TEXT) WITHOUT ROWID;
                 CREATE TABLE counts (id INT PRIMARY KEY, n INTEGER)",
            )
            .unwrap();
        let conn = Conn::open(&path, Access::ReadOnly).await.unwrap();
        let mut found = Vec::new();
        for table in ["books", "tags", "pairs", "codes", "counts"] {
            let structure = conn
                .describe(&ObjectRef::new("main", table))
                .await
                .unwrap();
            let identity: Vec<String> = structure
                .columns
                .into_iter()
                .filter(|column| column.identity)
                .map(|column| column.name)
                .collect();
            found.push((table, identity));
        }
        // Only a rowid's alias: one key column, declared INTEGER and
        // nothing else, in a table that has a rowid.
        assert_eq!(
            found,
            [
                ("books", vec!["id".to_owned()]),
                ("tags", Vec::new()),
                ("pairs", Vec::new()),
                ("codes", Vec::new()),
                ("counts", Vec::new()),
            ]
        );
    }
```

In `crates/tabletist-db/tests/postgres.rs`, after `generated_and_always_identity_columns_say_so`:

```rust
#[tokio::test]
async fn identity_and_serial_columns_say_so() {
    let Some(connection) = connect_as(Access::ReadOnly).await else {
        return;
    };
    // A sequence of the test's own: the fixture's are dropped without
    // CASCADE when it loads, and a table left behind that leaned on one
    // would fail every later load.
    on_its_own_tables(
        "DROP TABLE IF EXISTS identity_cols; DROP SEQUENCE IF EXISTS identity_tick",
        "CREATE SEQUENCE identity_tick;
         CREATE TABLE identity_cols (
             id bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
             seq bigint GENERATED BY DEFAULT AS IDENTITY,
             n serial,
             ticked bigint NOT NULL DEFAULT nextval('identity_tick'),
             title text NOT NULL DEFAULT 'none'
         )",
        async move {
            let structure = connection
                .describe(&ObjectRef::new("public", "identity_cols"))
                .await
                .unwrap();
            let identity: Vec<(&str, bool)> = structure
                .columns
                .iter()
                .map(|column| (column.name.as_str(), column.identity))
                .collect();
            // `ticked` draws on a sequence that is not its own: that is a
            // default like any other, and the user may want to see it.
            assert_eq!(
                identity,
                [
                    ("id", true),
                    ("seq", true),
                    ("n", true),
                    ("ticked", false),
                    ("title", false),
                ]
            );
        },
    )
    .await;
}
```

In `crates/tabletist-db/tests/mysql.rs`, after `generated_columns_say_so`:

```rust
#[tokio::test]
async fn an_auto_increment_column_is_an_identity_column() {
    let Some(connection) = connect_as(Access::ReadOnly).await else {
        return;
    };
    on_its_own_tables(
        "identity_cols",
        &["CREATE TABLE identity_cols (
               id INT AUTO_INCREMENT PRIMARY KEY,
               title VARCHAR(20) NOT NULL DEFAULT 'none'
           )"],
        async move {
            let structure = connection
                .describe(&ObjectRef::new("tabletist", "identity_cols"))
                .await
                .unwrap();
            let identity: Vec<(&str, bool)> = structure
                .columns
                .iter()
                .map(|column| (column.name.as_str(), column.identity))
                .collect();
            assert_eq!(identity, [("id", true), ("title", false)]);
        },
    )
    .await;
}
```

- [ ] **Step 2: Run them to see them fail.**

Run: `~/.cargo/bin/cargo test --locked -p tabletist-db identity`
Expected: does not compile, `no field identity on type ColumnInfo`.

- [ ] **Step 3: Write the code.**

`crates/tabletist-db/src/catalog.rs`, at the end of `ColumnInfo`:

```rust
    /// Whether the database gives the column its value from a counter of
    /// its own when an insert names none: an identity column (always or by
    /// default), a serial, MySQL's `AUTO_INCREMENT`, SQLite's alias of the
    /// rowid. Its default, where it has one, says nothing to a user.
    pub identity: bool,
```

`crates/tabletist-db/src/pg.rs`, the column query: after the `attgenerated … OR … attidentity … = 'a'` expression add one more column to the `SELECT` list, and read it in the mapping:

```sql
       COALESCE(to_jsonb(a) ->> 'attgenerated', '') <> ''
           OR COALESCE(to_jsonb(a) ->> 'attidentity', '') = 'a',
       (d.adbin IS NOT NULL OR COALESCE(to_jsonb(a) ->> 'attidentity', '') <> '')
           AND pg_get_serial_sequence(a.attrelid::regclass::text, a.attname::text) IS NOT NULL
```

```rust
                    generated: column(row, 7)?,
                    // A sequence the column owns and still draws on: an
                    // identity's, of either kind, and a serial's. A default
                    // that draws on some other sequence is a default like
                    // any other, and a serial whose default was dropped
                    // keeps its sequence and is numbered by nobody.
                    identity: column(row, 8)?,
```

`crates/tabletist-db/src/mysql.rs`, in the mapping closure, after `generated`:

```rust
                    identity: extra.to_ascii_lowercase().contains("auto_increment"),
```

`crates/tabletist-db/src/sqlite.rs`, `columns` becomes (import `rusqlite::OptionalExtension` at the top of the file if it is not there yet):

```rust
fn columns(connection: &rusqlite::Connection, object: &ObjectRef) -> Result<Vec<ColumnInfo>> {
    // A table WITHOUT ROWID has no rowid for a column to be the alias of.
    let rowid = connection
        .query_row(
            "SELECT wr = 0 FROM pragma_table_list(?1) WHERE schema = ?2",
            [&object.name, &object.schema],
            |row| row.get::<_, bool>(0),
        )
        .optional()
        .map_err(map_error)?
        .unwrap_or(false);
    let mut statement = connection
        .prepare(
            "SELECT name, type, \"notnull\", dflt_value, hidden, pk \
             FROM pragma_table_xinfo(?1, ?2) WHERE hidden <> 1 ORDER BY cid",
        )
        .map_err(map_error)?;
    let columns = statement
        .query_map([&object.name, &object.schema], |row| {
            let type_name = optional_text(row, 1)?.unwrap_or_default();
            let keyed = row.get::<_, i64>(5)? > 0;
            Ok((
                ColumnInfo {
                    name: text(row, 0)?,
                    nullable: row.get::<_, i64>(2)? == 0,
                    default: optional_text(row, 3)?,
                    comment: None,
                    allowed_values: None,
                    // 2 is a virtual generated column, 3 a stored one.
                    generated: matches!(row.get::<_, i64>(4)?, 2 | 3),
                    // Decided below, once the key's columns are counted.
                    identity: keyed && type_name.eq_ignore_ascii_case("INTEGER"),
                    type_name,
                },
                keyed,
            ))
        })
        .map_err(map_error)?
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(map_error)?;
    // The rowid's alias is the key's one column, declared INTEGER and
    // nothing else: SQLite numbers it when an insert names no value.
    let alone = columns.iter().filter(|(_, keyed)| *keyed).count() == 1;
    Ok(columns
        .into_iter()
        .map(|(mut column, _)| {
            column.identity &= rowid && alone;
            column
        })
        .collect())
}
```

Then build the workspace and give every full `ColumnInfo` literal the compiler names `identity: false,` (the ones that end in `..ColumnInfo::default()` need nothing):

Run: `~/.cargo/bin/cargo check --locked --workspace --all-targets && ~/.cargo/bin/cargo clippy --locked --features shots --lib --tests -- -D warnings`

- [ ] **Step 4: Run the tests.**

Run: `~/.cargo/bin/cargo test --locked -p tabletist-db identity`
Expected: PASS: the SQLite test, the two new ones of PostgreSQL and MySQL, and PostgreSQL's `generated_and_always_identity_columns_say_so`, which the filter also finds. The server tests print "skipped" without their variables; run them with the variables set, see "Before you start".

- [ ] **Step 5: Run the four checks, then commit.**

```bash
git add -A crates src
git commit -m "Say in a table's structure which columns the database numbers itself"
```

---

### Task 2: A change set carries new rows, and a save says what became of them

Only the types and what must compile with them. No driver runs an insert yet: each refuses a set that has one, so no commit drops a new row in silence.

**Files:**
- Modify: `crates/tabletist-db/src/write.rs`
- Modify: `crates/tabletist-db/src/lib.rs` (the `pub use write::{..}` line; `Connection::write`'s doc)
- Modify: `crates/tabletist-db/src/pg/write.rs`, `mysql/write.rs`, `sqlite/write.rs` (`Applied::Rows`, the refusal)
- Modify: `src/edit.rs` (`change_set`, `same_changes`), `src/app/editing.rs` (`written`)
- Modify: every `ChangeSet { .. }` literal and every `WriteOutcome::Written { .. }` the compiler names (about fifty of each, most in the three `crates/tabletist-db/tests/*.rs`)
- Test: `crates/tabletist-db/src/write.rs` (tests), `src/edit.rs` (tests)

- [ ] **Step 1: Write the failing tests.** In `crates/tabletist-db/src/write.rs`'s `mod tests` (it has tests of `check`; put these beside them, and use its helpers for an `ObjectRef` if it has one):

```rust
    fn value(column: &str) -> InsertValue {
        InsertValue {
            column: column.into(),
            type_name: "text".into(),
            new: NewValue::Text("x".into()),
        }
    }

    #[test]
    fn a_set_of_new_rows_alone_is_a_save() {
        let changes = ChangeSet {
            object: ObjectRef::new("public", "book_covers"),
            inserts: vec![RowInsert { set: vec![value("kind")] }, RowInsert { set: Vec::new() }],
            rows: Vec::new(),
        };
        assert_eq!(changes.check(), Ok(()));
        let nothing = ChangeSet {
            inserts: Vec::new(),
            ..changes
        };
        assert!(nothing.check().is_err());
    }

    #[test]
    fn a_new_row_that_sets_a_column_twice_is_refused() {
        let changes = ChangeSet {
            object: ObjectRef::new("public", "book_covers"),
            inserts: vec![RowInsert {
                set: vec![value("kind"), value("kind")],
            }],
            rows: Vec::new(),
        };
        assert_eq!(
            changes.check(),
            Err(Error::query("kind is set twice in one new row"))
        );
    }

    #[test]
    fn a_set_is_printed_without_its_values() {
        let changes = ChangeSet {
            object: ObjectRef::new("public", "book_covers"),
            inserts: vec![RowInsert { set: vec![value("kind")] }],
            rows: Vec::new(),
        };
        let printed = format!("{changes:?}");
        assert!(printed.contains("inserts: 1"), "{printed}");
        assert!(!printed.contains('x'), "{printed}");
    }
```

In `src/edit.rs`'s tests, beside the tests of `same_changes`:

```rust
    #[test]
    fn two_saves_that_differ_in_a_new_row_are_not_the_same_save() {
        let with = |text: &str| ChangeSet {
            object: ObjectRef::new("public", "book_covers"),
            inserts: vec![tabletist_db::RowInsert {
                set: vec![tabletist_db::InsertValue {
                    column: "kind".into(),
                    type_name: "character varying".into(),
                    new: NewValue::Text(text.into()),
                }],
            }],
            rows: Vec::new(),
        };
        assert!(same_changes(&with("print"), &with("print")));
        assert!(!same_changes(&with("print"), &with("ebook")));
    }
```

- [ ] **Step 2: Run them to see them fail.**

Run: `~/.cargo/bin/cargo test --locked -p tabletist-db --lib write::`
Expected: does not compile, `cannot find struct RowInsert`.

- [ ] **Step 3: Write the code.**

`crates/tabletist-db/src/write.rs`. `ChangeSet`, its `Debug`, and the new types:

```rust
/// Every change of one save, to one table. Written in one transaction, or
/// not at all.
#[derive(Clone, PartialEq)]
pub struct ChangeSet {
    pub object: ObjectRef,
    /// The new rows. They are written before `rows` are changed.
    pub inserts: Vec<RowInsert>,
    pub rows: Vec<RowChange>,
}

impl std::fmt::Debug for ChangeSet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let cells: usize = self.rows.iter().map(|row| row.set.len()).sum();
        f.debug_struct("ChangeSet")
            .field("object", &self.object)
            .field("inserts", &self.inserts.len())
            .field("rows", &self.rows.len())
            .field("cells", &cells)
            .finish_non_exhaustive()
    }
}

/// A new row: the columns that were given a value. Every other column is
/// left to the database, its default or its counter.
#[derive(Debug, Clone, PartialEq)]
pub struct RowInsert {
    pub set: Vec<InsertValue>,
}

/// One value of a new row.
#[derive(Debug, Clone, PartialEq)]
pub struct InsertValue {
    pub column: String,
    /// The structure's type name (`ColumnInfo::type_name`), which decides
    /// how `new` is sent.
    pub type_name: String,
    pub new: NewValue,
}
```

`WriteOutcome`:

```rust
/// How a save ended. Only `Written` changed anything.
#[derive(Debug, Clone, PartialEq)]
pub enum WriteOutcome {
    Written {
        /// Each new row as the database now holds it, in the order of the
        /// set's `inserts`. `None` for a row that was written and could
        /// not be found again: MySQL hands no row back, and a table
        /// without a primary key gives nothing to look one up by.
        inserted: Vec<Option<Vec<Value>>>,
        /// Each changed row as the database now holds it, in the set's
        /// order.
        rows: Vec<Vec<Value>>,
        elapsed: Duration,
    },
    /// Rows that are gone, or whose changed columns no longer hold what
    /// the page loaded. Nothing was written.
    Conflicts(Vec<Conflict>),
    /// The statement of `rows[row]` could not be built or failed. Nothing
    /// was written.
    Failed { row: usize, error: Error },
    /// The statement of `inserts[insert]` could not be built or failed.
    /// Nothing was written.
    FailedInsert { insert: usize, error: Error },
}
```

`ChangeSet::check`: replace the opening test, and check the inserts before the loop over `rows`:

```rust
        if self.rows.is_empty() && self.inserts.is_empty() {
            return Err(Error::query("there is nothing to save"));
        }
        for insert in &self.inserts {
            if let Some(cell) = insert.set.iter().enumerate().find_map(|(index, cell)| {
                insert.set[..index]
                    .iter()
                    .any(|earlier| earlier.column == cell.column)
                    .then_some(cell)
            }) {
                return Err(Error::query(format!(
                    "{} is set twice in one new row",
                    cell.column
                )));
            }
        }
```

`Applied`, and what a save stored:

```rust
/// What a save wrote, before its transaction ends.
pub(crate) struct Stored {
    pub(crate) inserted: Vec<Option<Vec<Value>>>,
    pub(crate) rows: Vec<Vec<Value>>,
}

/// What the statements of a save came to, before its transaction ends.
/// Each driver ends its own: committed for `Rows`, rolled back for the
/// others.
pub(crate) enum Applied {
    Rows(Stored),
    Conflicts(Vec<Conflict>),
    Failed { row: usize, error: Error },
}

impl Applied {
    /// The outcome of a save that began at `started`, once its transaction
    /// has ended.
    pub(crate) fn outcome(self, started: Instant) -> WriteOutcome {
        match self {
            Self::Rows(Stored { inserted, rows }) => WriteOutcome::Written {
                inserted,
                rows,
                elapsed: started.elapsed(),
            },
            Self::Conflicts(conflicts) => WriteOutcome::Conflicts(conflicts),
            Self::Failed { row, error } => WriteOutcome::Failed { row, error },
        }
    }
}
```

(`Applied` gets its `FailedInsert` in task 4, with the first driver that makes one: a variant nothing builds is dead code, and the checks refuse it.)

`crates/tabletist-db/src/lib.rs`: the export becomes

```rust
pub use write::{
    CellChange, ChangeSet, Conflict, InsertValue, NewValue, RowChange, RowInsert, WriteOutcome,
};
```

The three drivers, so that no commit loses a new row. In each of `pg/write.rs` (`save`), `mysql/write.rs` (`save`) and `sqlite/write.rs` (`write`), as the function's first statement:

```rust
        // Until this engine's save runs them (tasks 4 to 6).
        if !changes.inserts.is_empty() {
            return Err(Error::Unsupported("adding rows is not built for this engine yet"));
        }
```

and where each `apply` ends, `Ok(Applied::Rows(saved))` (PostgreSQL, MySQL) and `Ok(Applied::Rows(rows))` (SQLite) become

```rust
    Ok(Applied::Rows(Stored {
        inserted: Vec::new(),
        rows: saved,
    }))
```

(`rows` for SQLite), with `Stored` added to each file's `use crate::write::{..}`. The patterns `Ok(Applied::Rows(_))` and `Ok(Applied::Rows(rows))` in the three `save` functions stand as they are.

The app. `src/edit.rs`, `change_set`: the literal gains `inserts: Vec::new(),`. `same_changes`:

```rust
    let (
        ChangeSet {
            object,
            inserts,
            rows,
        },
        ChangeSet {
            object: other,
            inserts: other_inserts,
            rows: others,
        },
    ) = (a, b);
```

and its last expression:

```rust
    // A new row holds no float: its values are text or NULL.
    object == other
        && inserts == other_inserts
        && rows.len() == others.len()
        && rows.iter().zip(others).all(|(a, b)| same_row(a, b))
```

`src/app/editing.rs`, `written`: the `Written` arm's pattern becomes `Ok(WriteOutcome::Written { inserted: _, rows, elapsed })` with this comment over it, and one arm is added beside `Failed`:

```rust
            // No save of the app carries a new row yet, so nothing comes
            // back for one.
```

```rust
            Ok(WriteOutcome::FailedInsert { error, .. }) => {
                // As a save the database refused, until a tab holds new
                // rows for the failure to be shown on.
                object.edits.fail(None, error);
                object.fields = None;
            }
```

Then let the compiler name the rest:

Run: `~/.cargo/bin/cargo check --locked --workspace --all-targets`

Every `ChangeSet { object, rows }` literal gains `inserts: Vec::new(),` between the two. Every `WriteOutcome::Written { rows, elapsed }` that is built gains `inserted: Vec::new(),`; every one that is matched without `..` gains `..`. The one `match` that names every arm of `WriteOutcome` is `written`'s, done above.

Two tests assert what `ChangeSet`'s `Debug` prints, to the letter, and each gains `inserts: 0, ` before `rows:`: the one in `crates/tabletist-db/src/write.rs`'s tests that expects `… name: "users" }, rows: 2, cells: 3, .. }`, and `a_save_is_printed_without_the_values_it_carries` in `src/backend.rs`, which expects `… rows: 1, cells: 1, .. } }`. Change nothing else in the tests.

Run: `~/.cargo/bin/cargo clippy --locked --features shots --lib --tests -- -D warnings`

- [ ] **Step 4: Run the tests.**

Run: `~/.cargo/bin/cargo test --locked -p tabletist-db --lib write:: && ~/.cargo/bin/cargo test --locked --lib same_save`
Expected: PASS.

- [ ] **Step 5: Run the four checks, then commit.**

```bash
git add -A crates src
git commit -m "Let a change set carry new rows, and a save say what became of them"
```

---

### Task 3: The INSERT of a new row, as it is read and as it is sent

**Files:**
- Modify: `crates/tabletist-db/src/dialect.rs` (`InsertStatement`, `Dialect::insert_row`, `new_operand`)
- Modify: `crates/tabletist-db/src/lib.rs` (export `InsertStatement`)
- Test: `crates/tabletist-db/src/dialect.rs` (tests, beside `an_update_is_shown_with_its_values_as_literals`)

- [ ] **Step 1: Write the failing tests.** They use the tests' own `text` helper (a `Value::Text`); `covers` and `sets` are new:

```rust
    fn covers(schema: &str) -> ObjectRef {
        ObjectRef::new(schema, "book_covers")
    }

    fn sets(column: &str, type_name: &str, new: NewValue) -> InsertValue {
        InsertValue {
            column: column.into(),
            type_name: type_name.into(),
            new,
        }
    }

    /// The canvas' new row: a cover of the publisher Harbor Press.
    const PUBLISHER: &str = "9100000000000000004";

    #[test]
    fn the_same_new_row_on_each_engine() {
        let row = |type_name: &str| RowInsert {
            set: vec![sets("publisher_id", type_name, NewValue::Text(PUBLISHER.into()))],
        };
        let insert = Dialect::Postgres
            .insert_row(&covers("public"), &row("bigint"))
            .unwrap();
        assert_eq!(
            insert.shown,
            r#"INSERT INTO "public"."book_covers" ("publisher_id") VALUES ('9100000000000000004') RETURNING *"#
        );
        // PostgreSQL runs exactly what it shows.
        assert_eq!(insert.sql.text, insert.shown);
        assert!(insert.sql.params.is_empty());

        let insert = Dialect::MySql
            .insert_row(&covers("bookshop"), &row("bigint"))
            .unwrap();
        assert_eq!(
            insert.shown,
            "INSERT INTO `bookshop`.`book_covers` (`publisher_id`) VALUES ('9100000000000000004')"
        );
        assert_eq!(
            insert.sql.text,
            "INSERT INTO `bookshop`.`book_covers` (`publisher_id`) VALUES (?)"
        );
        assert_eq!(insert.sql.params, [text(PUBLISHER)]);

        // SQLite stores text as text: a number is made one here.
        let insert = Dialect::Sqlite
            .insert_row(&covers("main"), &row("INTEGER"))
            .unwrap();
        assert_eq!(
            insert.shown,
            r#"INSERT INTO "main"."book_covers" ("publisher_id") VALUES (9100000000000000004) RETURNING *"#
        );
        assert_eq!(
            insert.sql.text,
            r#"INSERT INTO "main"."book_covers" ("publisher_id") VALUES (?) RETURNING *"#
        );
        assert_eq!(insert.sql.params, [Value::Int(9_100_000_000_000_000_004)]);
    }

    #[test]
    fn a_new_row_names_only_what_was_set() {
        let row = RowInsert {
            set: vec![
                sets("publisher_id", "bigint", NewValue::Text(PUBLISHER.into())),
                sets("image_data", "jsonb", NewValue::Null),
            ],
        };
        let insert = Dialect::Postgres
            .insert_row(&covers("public"), &row)
            .unwrap();
        // Not `kind`, `created_at` or `id`: the database fills those.
        assert_eq!(
            insert.shown,
            r#"INSERT INTO "public"."book_covers" ("publisher_id", "image_data") VALUES ('9100000000000000004', NULL) RETURNING *"#
        );
        let insert = Dialect::MySql.insert_row(&covers("bookshop"), &row).unwrap();
        // NULL is written out, never bound, as in an UPDATE.
        assert_eq!(
            insert.sql.text,
            "INSERT INTO `bookshop`.`book_covers` (`publisher_id`, `image_data`) VALUES (?, NULL)"
        );
        assert_eq!(insert.sql.params, [text(PUBLISHER)]);
    }

    #[test]
    fn a_new_row_with_nothing_set_takes_every_default() {
        let row = RowInsert { set: Vec::new() };
        let shown = |dialect: Dialect, schema: &str| {
            let insert = dialect.insert_row(&covers(schema), &row).unwrap();
            assert_eq!(insert.sql.text, insert.shown);
            assert!(insert.sql.params.is_empty());
            insert.shown
        };
        assert_eq!(
            shown(Dialect::Postgres, "public"),
            r#"INSERT INTO "public"."book_covers" DEFAULT VALUES RETURNING *"#
        );
        assert_eq!(
            shown(Dialect::Sqlite, "main"),
            r#"INSERT INTO "main"."book_covers" DEFAULT VALUES RETURNING *"#
        );
        // MySQL has no DEFAULT VALUES.
        assert_eq!(
            shown(Dialect::MySql, "bookshop"),
            "INSERT INTO `bookshop`.`book_covers` () VALUES ()"
        );
    }

    #[test]
    fn a_new_value_its_column_cannot_take_is_refused_before_it_is_sent() {
        let row = |type_name: &str, new: &str| RowInsert {
            set: vec![sets("n", type_name, NewValue::Text(new.into()))],
        };
        // SQLite would store the text as it is.
        assert!(Dialect::Sqlite.insert_row(&covers("main"), &row("INTEGER", "seven")).is_err());
        // Bytes are not sent as text, for a new row as for a changed one.
        assert!(Dialect::Postgres.insert_row(&covers("public"), &row("bytea", "x")).is_err());
        // PostgreSQL text cannot hold a NUL.
        assert!(Dialect::Postgres.insert_row(&covers("public"), &row("text", "a\0b")).is_err());
        assert!(Dialect::Sqlite.insert_row(&covers("main"), &row("TEXT", "a\0b")).is_ok());
    }
```

- [ ] **Step 2: Run them to see them fail.**

Run: `~/.cargo/bin/cargo test --locked -p tabletist-db --lib new_row`
Expected: does not compile, `no method named insert_row found for enum Dialect`.

- [ ] **Step 3: Write the code.** `crates/tabletist-db/src/dialect.rs`, after `UpdateParts`:

```rust
/// A new row's `INSERT`, as a user reads it and as the driver runs it. Both
/// come from the same values, as a [`RowUpdate`]'s do.
#[derive(Debug, Clone, PartialEq)]
pub struct InsertStatement {
    /// The statement with its values as literals.
    pub shown: String,
    /// What the driver sends: for PostgreSQL the shown text, for MySQL and
    /// SQLite the same statement with the values bound.
    pub sql: Sql,
}
```

`new_operand` takes what it reads of a change, so a new value, which has no loaded one, goes through the same rules. Its signature and the three places that read `change` become:

```rust
    /// A cell's new value as an operand. PostgreSQL and MySQL convert text
    /// to the column's type themselves. Where the database would store the
    /// text as it is, it is converted here, by the column's class: numbers
    /// on SQLite, and a boolean on SQLite and MySQL, which keep one as 1 or
    /// 0. Text that cannot be converted is refused, naming the column.
    /// `loaded` is what the cell held, and `None` for a new row's.
    fn new_operand(
        self,
        column: &str,
        type_name: &str,
        loaded: Option<&Value>,
        new: &NewValue,
    ) -> Result<Operand> {
        let class = column_class(self, type_name);
        // A binary column is never sent text (MySQL would store a `bit`'s
        // text as the characters' codes), and is not edited at all yet: a
        // NULL for one is refused with the rest.
        // By what the cell held too: a SQLite column of any declared type
        // can hold a blob.
        if class == ColumnClass::Binary || matches!(loaded, Some(Value::Bytes(_))) {
            return Err(Error::query(format!(
                "{column}: binary values cannot be edited yet"
            )));
        }
        let NewValue::Text(text) = new else {
            return Ok(Operand::Null);
        };
        let refused = |expects: &str| {
            Error::query(format!(
                "{column}: {} expects {expects}",
                if type_name.is_empty() {
                    "the column"
                } else {
                    type_name
                }
            ))
        };
```

and the SQLite arm that looks at what the cell held:

```rust
            (Self::Sqlite, ColumnClass::Other)
                if matches!(loaded, Some(Value::Int(_) | Value::Float(_))) =>
```

The rest of the function is unchanged. Its one caller, in `update_row`, becomes:

```rust
            let operand = self.new_operand(
                &change.column,
                &change.type_name,
                Some(&change.loaded),
                &change.new,
            )?;
```

After `update_row`:

```rust
    /// The `INSERT` of one new row of a save: the columns that were set,
    /// and nothing for the others, which the database fills. `Err` names
    /// the value that cannot be sent in its column's form. Refused here and
    /// nowhere after it, as [`Dialect::update_row`] refuses, so what a
    /// review shows of a new row is what a save does with it.
    ///
    /// PostgreSQL and SQLite hand the row back, as it was stored; MySQL
    /// cannot, and its save finds the row again.
    pub fn insert_row(self, object: &ObjectRef, row: &RowInsert) -> Result<InsertStatement> {
        let table = self.qualified(object);
        let back = match self {
            Self::Postgres | Self::Sqlite => " RETURNING *",
            Self::MySql => "",
        };
        if row.set.is_empty() {
            let text = match self {
                Self::Postgres | Self::Sqlite => {
                    format!("INSERT INTO {table} DEFAULT VALUES{back}")
                }
                Self::MySql => format!("INSERT INTO {table} () VALUES (){back}"),
            };
            return Ok(InsertStatement {
                shown: text.clone(),
                sql: Sql {
                    text,
                    params: Vec::new(),
                },
            });
        }
        let mut params = Vec::new();
        let mut names = Vec::with_capacity(row.set.len());
        let mut shown = Vec::with_capacity(row.set.len());
        let mut sent = Vec::with_capacity(row.set.len());
        for cell in &row.set {
            let operand = self.new_operand(&cell.column, &cell.type_name, None, &cell.new)?;
            names.push(self.quote_ident(&cell.column));
            shown.push(self.shown(&operand));
            sent.push(self.sent(&operand, &mut params));
        }
        let head = format!("INSERT INTO {table} ({}) VALUES (", names.join(", "));
        let shown = format!("{head}{}){back}", shown.join(", "));
        let text = format!("{head}{}){back}", sent.join(", "));
        // PostgreSQL text cannot hold a NUL, and the driver cannot put one
        // in a message: see `update_row`.
        let refused = match self {
            Self::Postgres => text.contains('\0'),
            Self::MySql | Self::Sqlite => false,
        };
        if refused {
            return Err(Error::query("PostgreSQL text cannot hold a NUL character"));
        }
        Ok(InsertStatement {
            shown,
            sql: Sql { text, params },
        })
    }
```

Add `RowInsert` to the file's `use crate::{..}` (and `InsertValue` to the tests'), and `InsertStatement` to `lib.rs`'s `pub use dialect::{..}`.

- [ ] **Step 4: Run the tests.**

Run: `~/.cargo/bin/cargo test --locked -p tabletist-db --lib dialect::`
Expected: PASS, the four new tests and every test of `update_row` as before.

- [ ] **Step 5: Run the four checks, then commit.**

```bash
git add -A crates
git commit -m "Build the INSERT of a new row, as it is read and as it is sent"
```

---

### Task 4: SQLite's save writes the new rows

**Files:**
- Modify: `crates/tabletist-db/src/write.rs` (`Applied::FailedInsert`, `named_twice`)
- Modify: `crates/tabletist-db/src/sqlite/write.rs` (`write`, `apply`)
- Test: `crates/tabletist-db/src/write.rs` (tests), `crates/tabletist-db/tests/sqlite.rs` (after `a_statement_that_fails_undoes_the_rows_before_it`)

- [ ] **Step 1: Write the failing tests.** In `crates/tabletist-db/src/write.rs`'s tests, with task 2's `value` helper:

```rust
    #[test]
    fn a_name_in_other_letters_is_the_same_column_where_the_engine_says_so() {
        let row = RowInsert {
            set: vec![value("kind"), value("title"), value("KIND"), value("É"), value("é")],
        };
        // SQLite folds ASCII letters only; MySQL folds every letter.
        let ascii = |a: &str, b: &str| a.eq_ignore_ascii_case(b);
        let every = |a: &str, b: &str| a.to_lowercase() == b.to_lowercase();
        assert_eq!(named_twice(&row, ascii), Some("KIND"));
        let accents = RowInsert {
            set: vec![value("É"), value("é")],
        };
        assert_eq!(named_twice(&accents, ascii), None);
        assert_eq!(named_twice(&accents, every), Some("é"));
        assert_eq!(named_twice(&RowInsert { set: vec![value("kind")] }, every), None);
    }
```

In `crates/tabletist-db/tests/sqlite.rs`, add `InsertValue` and `RowInsert` to the file's `use tabletist_db::{..}`.

```rust
/// A new row of `users`: each (column, declared type, new value).
fn new_user(cells: &[(&str, &str, NewValue)]) -> RowInsert {
    RowInsert {
        set: cells
            .iter()
            .map(|(column, type_name, new)| InsertValue {
                column: (*column).into(),
                type_name: (*type_name).into(),
                new: new.clone(),
            })
            .collect(),
    }
}

fn adding(table: &str, inserts: Vec<RowInsert>, rows: Vec<RowChange>) -> ChangeSet {
    ChangeSet {
        object: ObjectRef::new("main", table),
        inserts,
        rows,
    }
}

#[tokio::test]
async fn a_new_row_comes_back_as_the_database_stored_it() {
    let (connection, _dir) = fixture_as(Access::Writable).await;
    let changes = adding(
        "users",
        vec![new_user(&[("email", "TEXT", to("new@example.com"))])],
        Vec::new(),
    );
    let WriteOutcome::Written { inserted, rows, .. } = connection
        .write(&changes, &StopFlag::new())
        .await
        .unwrap()
    else {
        panic!("the save wrote");
    };
    assert!(rows.is_empty());
    // The fixture's users are 1 to 5: the rowid's alias gives the next.
    let (columns, stored) = user(&connection, 6).await;
    assert_eq!(inserted, [Some(stored.clone())]);
    let at = |name: &str| &stored[columns.iter().position(|column| column == name).unwrap()];
    assert_eq!(*at("id"), Value::Int(6));
    assert_eq!(*at("email"), Value::Text("new@example.com".into()));
    // What was not set is the column's default, or NULL.
    assert_eq!(*at("created_at"), Value::Text("2026-01-01 00:00:00".into()));
    assert_eq!(*at("name"), Value::Null);
}

#[tokio::test]
async fn new_rows_and_changed_rows_are_one_save() {
    let (connection, _dir) = fixture_as(Access::Writable).await;
    let mut changes = rename(1, "Ada Lovelace", "Grace");
    changes.inserts = vec![new_user(&[
        ("email", "TEXT", to("new@example.com")),
        ("active", "BOOLEAN", to("false")),
    ])];
    let WriteOutcome::Written { inserted, rows, .. } = connection
        .write(&changes, &StopFlag::new())
        .await
        .unwrap()
    else {
        panic!("the save wrote");
    };
    assert_eq!(inserted, [Some(user(&connection, 6).await.1)]);
    assert_eq!(rows, [user(&connection, 1).await.1]);
}

#[tokio::test]
async fn a_new_row_that_fails_undoes_the_rows_before_it() {
    let (connection, _dir) = fixture_as(Access::Writable).await;
    let before = connection.fetch_rows(&users(10)).await.unwrap().rows;
    let mut changes = rename(1, "Ada Lovelace", "Grace");
    changes.inserts = vec![
        new_user(&[("email", "TEXT", to("new@example.com"))]),
        // UNIQUE: Ada has it.
        new_user(&[("email", "TEXT", to("ada@example.com"))]),
    ];
    let outcome = connection
        .write(&changes, &StopFlag::new())
        .await
        .unwrap();
    assert!(
        matches!(
            &outcome,
            // 2067 is SQLITE_CONSTRAINT_UNIQUE.
            WriteOutcome::FailedInsert { insert: 1, error: Error::Query { code, .. } }
                if code.as_deref() == Some("2067")
        ),
        "{outcome:?}"
    );
    // Neither the first new row nor the change is there.
    assert_eq!(connection.fetch_rows(&users(10)).await.unwrap().rows, before);
    // And the session refuses writes again, as after any save.
    assert_eq!(query_only(&connection).await, Value::Int(1));
}

#[tokio::test]
async fn a_new_row_that_leaves_a_required_column_out_is_the_databases_error() {
    let (connection, _dir) = fixture_as(Access::Writable).await;
    // `email` is NOT NULL and has no default.
    let outcome = connection
        .write(
            &adding("users", vec![new_user(&[])], Vec::new()),
            &StopFlag::new(),
        )
        .await
        .unwrap();
    assert!(
        matches!(
            &outcome,
            // 1299 is SQLITE_CONSTRAINT_NOTNULL.
            WriteOutcome::FailedInsert { insert: 0, error: Error::Query { code, .. } }
                if code.as_deref() == Some("1299")
        ),
        "{outcome:?}"
    );
}

#[tokio::test]
async fn a_table_without_a_key_takes_a_new_row() {
    let (connection, dir) = fixture_as(Access::Writable).await;
    other_program(&dir)
        .execute_batch(
            "CREATE TABLE cover_notes (line TEXT, kind TEXT DEFAULT 'print');
             CREATE TABLE cover_stamps (at TEXT DEFAULT 'never')",
        )
        .unwrap();
    let note = RowInsert {
        set: vec![InsertValue {
            column: "line".into(),
            type_name: "TEXT".into(),
            new: to("first"),
        }],
    };
    let outcome = connection
        .write(&adding("cover_notes", vec![note], Vec::new()), &StopFlag::new())
        .await
        .unwrap();
    let WriteOutcome::Written { inserted, .. } = outcome else {
        panic!("the save wrote");
    };
    assert_eq!(
        inserted,
        [Some(vec![
            Value::Text("first".into()),
            Value::Text("print".into())
        ])]
    );
    // Nothing set at all: DEFAULT VALUES.
    let outcome = connection
        .write(
            &adding("cover_stamps", vec![RowInsert { set: Vec::new() }], Vec::new()),
            &StopFlag::new(),
        )
        .await
        .unwrap();
    let WriteOutcome::Written { inserted, .. } = outcome else {
        panic!("the save wrote");
    };
    assert_eq!(inserted, [Some(vec![Value::Text("never".into())])]);
}
```

(`users(10)` is the file's own query of the first ten users, which `user` builds on. If `rename`'s set is not `mut`-friendly as written, build the set with `adding("users", inserts, rename(..).rows)`.)

- [ ] **Step 2: Run them to see them fail.**

Run: `~/.cargo/bin/cargo test --locked -p tabletist-db --test sqlite new_row`
Expected: does not compile at first (`cannot find function named_twice` in the crate's own tests). Once step 3's `write.rs` part is in, the five tests of `tests/sqlite.rs` FAIL with `Unsupported("adding rows is not built for this engine yet")`.

- [ ] **Step 3: Write the code.** `crates/tabletist-db/src/write.rs`. `Applied` gains the failure of a new row, and `outcome` its arm:

```rust
    Failed { row: usize, error: Error },
    FailedInsert { insert: usize, error: Error },
```

```rust
            Self::FailedInsert { insert, error } => WriteOutcome::FailedInsert { insert, error },
```

After `spelled_otherwise`:

```rust
/// A column `insert` names a second time, as an engine that matches names
/// without regard to case reads them: `same` is how it compares two.
/// `ChangeSet::check` compares names exactly, and lets `kind` beside `KIND`
/// through: MySQL and SQLite would take both for one column.
pub(crate) fn named_twice(insert: &RowInsert, same: impl Fn(&str, &str) -> bool) -> Option<&str> {
    insert.set.iter().enumerate().find_map(|(index, cell)| {
        insert.set[..index]
            .iter()
            .any(|earlier| same(&earlier.column, &cell.column))
            .then_some(cell.column.as_str())
    })
}
```

`crates/tabletist-db/src/sqlite/write.rs`. In `write`, the refusal of task 2 goes, and the inserts are built with the updates, before the file is asked for:

```rust
    let mut inserts = Vec::with_capacity(changes.inserts.len());
    for (insert, row) in changes.inserts.iter().enumerate() {
        // SQLite takes a name in other ASCII letters for the column too.
        let built = match named_twice(row, |a, b| a.eq_ignore_ascii_case(b)) {
            Some(name) => Err(Error::query(format!(
                "{name} is set twice in one new row"
            ))),
            None => Dialect::Sqlite.insert_row(&changes.object, row),
        };
        match built {
            Ok(built) => inserts.push(built),
            Err(error) => return Ok(WriteOutcome::FailedInsert { insert, error }),
        }
    }
```

`apply` is called with them (`apply(connection, changes, &inserts, &updates, stop)`) and takes `inserts: &[InsertStatement]` after `changes`. In it, after the conflicts are answered (`return Ok(Applied::Conflicts(..))`) and before the loop over `updates`:

```rust
    // The new rows, before any row is changed: the order a review shows
    // them in. `RETURNING *` gives each as the table now holds it.
    let mut inserted = Vec::with_capacity(inserts.len());
    for (insert, statement) in inserts.iter().enumerate() {
        not_stopped(stop)?;
        let mut made = match read(connection, &statement.sql) {
            Ok((_, made)) => made,
            // A cancel or a lost session ends the save; anything else is
            // the statement's own failure.
            Err(error @ (Error::Cancelled | Error::ConnectionLost(_))) => return Err(error),
            Err(error) => return Ok(Applied::FailedInsert { insert, error }),
        };
        // One statement makes one row. A trigger that ran in its place can
        // have made none, and then what was stored is not known.
        let row = made
            .pop()
            .filter(|_| made.is_empty())
            .ok_or_else(not_read_back)?;
        inserted.push(Some(row.values));
    }
```

and the function's end becomes

```rust
    Ok(Applied::Rows(Stored { inserted, rows }))
```

Add `named_twice` to the file's `use crate::write::{..}` and `InsertStatement` to its `use crate::dialect::{..}`.

- [ ] **Step 4: Run the tests.**

Run: `~/.cargo/bin/cargo test --locked -p tabletist-db --test sqlite`
Expected: PASS, the five new tests and every write test as before. (The fence is down for a save and `begin` lifts `query_only`, so the `INSERT` passes where the `UPDATE` does. `read` takes a statement that is no `SELECT`: it only prepares the text for its columns.) Then `~/.cargo/bin/cargo test --locked -p tabletist-db --lib write::`: PASS.

- [ ] **Step 5: Run the four checks, then commit.**

```bash
git add -A crates
git commit -m "Write a save's new rows on SQLite, and hand each back as it was stored"
```

---

### Task 5: PostgreSQL's save writes the new rows

**Files:**
- Modify: `crates/tabletist-db/src/pg/write.rs` (`save`, `apply`)
- Test: `crates/tabletist-db/tests/postgres.rs` (after `a_statement_that_fails_undoes_the_rows_before_it`)

- [ ] **Step 1: Write the failing tests.** Add `InsertValue` and `RowInsert` to the file's imports.

```rust
/// Bookshop's covers, as the design's new row needs them: an identity key,
/// a required column, a unique one, and two defaults.
fn covers(table: &str) -> String {
    format!(
        "CREATE TABLE {table} (
             id bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
             publisher_id bigint NOT NULL,
             kind varchar NOT NULL DEFAULT 'print',
             isbn text UNIQUE,
             created_at timestamp NOT NULL DEFAULT '2026-10-07 10:42:09'
         );
         INSERT INTO {table} (publisher_id, kind, isbn) VALUES
             (9100000000000000001, 'print', '978-1-4028-9462-6'),
             (9100000000000000001, 'ebook', NULL)"
    )
}

fn sets(column: &str, type_name: &str, new: &str) -> InsertValue {
    InsertValue {
        column: column.into(),
        type_name: type_name.into(),
        new: to(new),
    }
}

#[tokio::test]
async fn a_new_row_comes_back_as_the_database_stored_it() {
    let Some(connection) = connect_as(Access::Writable).await else {
        return;
    };
    on_its_own_tables(
        &drop_table("covers_new"),
        &covers("covers_new"),
        async move {
            let mut changes = changes_to("covers_new", Vec::new());
            changes.inserts = vec![RowInsert {
                set: vec![sets("publisher_id", "bigint", "9100000000000000004")],
            }];
            let outcome = within(connection.write(&changes, &StopFlag::new()))
                .await
                .unwrap();
            let WriteOutcome::Written { inserted, rows, .. } = outcome else {
                panic!("the save wrote");
            };
            assert!(rows.is_empty());
            // Two covers were there: the identity gives 3.
            let (columns, stored) = row_of(&connection, "covers_new", 3).await;
            assert_eq!(inserted, [Some(stored.clone())]);
            let at = |name: &str| &stored[columns.iter().position(|column| column == name).unwrap()];
            assert_eq!(*at("id"), Value::Int(3));
            assert_eq!(*at("publisher_id"), Value::Int(9_100_000_000_000_000_004));
            assert_eq!(*at("kind"), Value::Text("print".into()));
            assert_eq!(*at("isbn"), Value::Null);
        },
    )
    .await;
}

#[tokio::test]
async fn a_new_row_that_fails_undoes_the_rows_before_it() {
    let Some(connection) = connect_as(Access::Writable).await else {
        return;
    };
    on_its_own_tables(
        &drop_table("covers_undone"),
        &covers("covers_undone"),
        async move {
            let (columns, second) = row_of(&connection, "covers_undone", 2).await;
            let mut changes = changes_to(
                "covers_undone",
                vec![by_id(
                    2,
                    vec![cell(&columns, &second, "kind", "character varying", to("audio"))],
                )],
            );
            changes.inserts = vec![
                RowInsert {
                    set: vec![sets("publisher_id", "bigint", "9100000000000000004")],
                },
                // UNIQUE: the first cover has it.
                RowInsert {
                    set: vec![
                        sets("publisher_id", "bigint", "9100000000000000004"),
                        sets("isbn", "text", "978-1-4028-9462-6"),
                    ],
                },
            ];
            let outcome = within(connection.write(&changes, &StopFlag::new()))
                .await
                .unwrap();
            assert!(
                matches!(
                    &outcome,
                    WriteOutcome::FailedInsert { insert: 1, error: Error::Query { code, .. } }
                        if code.as_deref() == Some("23505")
                ),
                "{outcome:?}"
            );
            // Neither the first new row nor the change is there.
            assert_eq!(page_of(&connection, "covers_undone").await.1.len(), 2);
            assert_eq!(row_of(&connection, "covers_undone", 2).await.1, second);
        },
    )
    .await;
}

#[tokio::test]
async fn a_table_without_a_key_takes_a_new_row() {
    let Some(connection) = connect_as(Access::Writable).await else {
        return;
    };
    on_its_own_tables(
        &drop_table("cover_stamps"),
        "CREATE TABLE cover_stamps (line text DEFAULT 'none', n integer DEFAULT 7)",
        async move {
            let mut changes = changes_to("cover_stamps", Vec::new());
            // Nothing set: DEFAULT VALUES.
            changes.inserts = vec![RowInsert { set: Vec::new() }];
            let outcome = within(connection.write(&changes, &StopFlag::new()))
                .await
                .unwrap();
            let WriteOutcome::Written { inserted, .. } = outcome else {
                panic!("the save wrote");
            };
            assert_eq!(
                inserted,
                [Some(vec![Value::Text("none".into()), Value::Int(7)])]
            );
        },
    )
    .await;
}
```

(`page_of(&connection, table)` gives the table's columns and rows, as the file's doc comment over it says; if its return is shaped otherwise, count its rows as its other callers do.)

- [ ] **Step 2: Run them to see them fail.**

Run: `~/.cargo/bin/cargo test --locked -p tabletist-db --test postgres new_row` (with `TABLETIST_TEST_PG_URL` set)
Expected: FAIL, `Unsupported("adding rows is not built for this engine yet")`.

- [ ] **Step 3: Write the code.** `crates/tabletist-db/src/pg/write.rs`. In `save`, the refusal of task 2 goes, and after the loop that builds `updates`:

```rust
        let mut inserts = Vec::with_capacity(changes.inserts.len());
        for (insert, row) in changes.inserts.iter().enumerate() {
            match Dialect::Postgres.insert_row(&changes.object, row) {
                Ok(built) => inserts.push(built),
                Err(error) => return Ok(WriteOutcome::FailedInsert { insert, error }),
            }
        }
```

`apply` is called with them (`apply(&client, changes, &inserts, &updates, stop)`) and takes `inserts: &[InsertStatement]` after `changes`. In it, after the conflicts are answered and before the loop over `updates`:

```rust
    // The new rows, before any row is changed: the order a review shows
    // them in. `RETURNING *` gives each in the table's column order, the
    // order `columns` was read in.
    let mut inserted = Vec::with_capacity(inserts.len());
    for (insert, statement) in inserts.iter().enumerate() {
        not_stopped(stop)?;
        let messages = match client
            .simple_query(&statement.sql.text)
            .await
            .map_err(query_error)
        {
            Ok(messages) => messages,
            // A cancel or a lost session ends the save; anything else is
            // the statement's own failure.
            Err(error @ (Error::Cancelled | Error::ConnectionLost(_))) => return Err(error),
            Err(error) => return Ok(Applied::FailedInsert { insert, error }),
        };
        let mut made = Vec::new();
        for message in messages {
            if let SimpleQueryMessage::Row(row) = message {
                made.push(row_values(&row, &columns)?);
            }
        }
        // One statement makes one row. A trigger can skip the row, or a
        // rule write somewhere else: then what was stored is not known.
        let row = made
            .pop()
            .filter(|_| made.is_empty())
            .ok_or_else(not_read_back)?;
        inserted.push(Some(row));
    }
```

and the function's end becomes

```rust
    Ok(Applied::Rows(Stored {
        inserted,
        rows: saved,
    }))
```

Add `InsertStatement` to the file's `use crate::dialect::{..}`.

- [ ] **Step 4: Run the tests.**

Run: `~/.cargo/bin/cargo test --locked -p tabletist-db --test postgres` and `~/.cargo/bin/cargo test --locked -p tabletist-db --lib pg::` (with `TABLETIST_TEST_PG_URL` set)
Expected: PASS, none skipped.

- [ ] **Step 5: Run the four checks, then commit.**

```bash
git add -A crates
git commit -m "Write a save's new rows on PostgreSQL, and hand each back as it was stored"
```

---

### Task 6: MySQL's save writes the new rows, and finds them again

MySQL hands no row back. The save notes what finds each new row as it makes it (the primary key's columns, with the value sent for each or the counter's), and reads the rows once every statement has run.

**Files:**
- Modify: `crates/tabletist-db/src/mysql/write.rs` (`save`, `apply`, new `build_insert`, `hold`, `insert_row`, `FoundBy`, `sent_key`)
- Test: `crates/tabletist-db/tests/mysql.rs` (after `a_statement_that_fails_undoes_the_rows_before_it`)

- [ ] **Step 1: Write the failing tests.** Add `InsertValue` and `RowInsert` to the file's imports.

```rust
/// Bookshop's covers, as the design's new row needs them: a counted key, a
/// required column, a unique one, and two defaults.
fn covers(table: &str) -> [String; 2] {
    [
        format!(
            "CREATE TABLE {table} (
                 id BIGINT AUTO_INCREMENT PRIMARY KEY,
                 publisher_id BIGINT NOT NULL,
                 kind VARCHAR(20) NOT NULL DEFAULT 'print',
                 isbn VARCHAR(32) UNIQUE,
                 created_at DATETIME NOT NULL DEFAULT '2026-10-07 10:42:09'
             )"
        ),
        format!(
            "INSERT INTO {table} (publisher_id, kind, isbn) VALUES
                 (9100000000000000001, 'print', '978-1-4028-9462-6'),
                 (9100000000000000001, 'ebook', NULL)"
        ),
    ]
}

fn sets(column: &str, type_name: &str, new: &str) -> InsertValue {
    InsertValue {
        column: column.into(),
        type_name: type_name.into(),
        new: to(new),
    }
}

#[tokio::test]
async fn a_new_row_comes_back_as_the_database_stored_it() {
    let Some(connection) = connect_as(Access::Writable).await else {
        return;
    };
    let covers = covers("covers_new");
    on_its_own_tables("covers_new", &[&covers[0], &covers[1]], async move {
        let mut changes = changes_to("covers_new", Vec::new());
        changes.inserts = vec![RowInsert {
            set: vec![sets("publisher_id", "bigint", "9100000000000000004")],
        }];
        let outcome = within(connection.write(&changes, &StopFlag::new()))
            .await
            .unwrap();
        let WriteOutcome::Written { inserted, rows, .. } = outcome else {
            panic!("the save wrote");
        };
        assert!(rows.is_empty());
        // Two covers were there: the counter gives 3.
        let (columns, stored) = row_of(&connection, "covers_new", 3).await;
        assert_eq!(inserted, [Some(stored.clone())]);
        let at = |name: &str| &stored[columns.iter().position(|column| column == name).unwrap()];
        assert_eq!(*at("publisher_id"), Value::Int(9_100_000_000_000_000_004));
        assert_eq!(*at("kind"), Value::Text("print".into()));
    })
    .await;
}

#[tokio::test]
async fn a_new_row_whose_key_was_typed_is_found_by_it() {
    let Some(connection) = connect_as(Access::Writable).await else {
        return;
    };
    let people = people("people_new");
    on_its_own_tables("people_new", &[&people[0], &people[1]], async move {
        let mut changes = changes_to("people_new", Vec::new());
        changes.inserts = vec![RowInsert {
            set: vec![sets("id", "int", "7"), sets("email", VARCHAR, "g@x")],
        }];
        let outcome = within(connection.write(&changes, &StopFlag::new()))
            .await
            .unwrap();
        let WriteOutcome::Written { inserted, .. } = outcome else {
            panic!("the save wrote");
        };
        assert_eq!(inserted, [Some(row_of(&connection, "people_new", 7).await.1)]);
    })
    .await;
}

#[tokio::test]
async fn a_new_row_that_fails_undoes_the_rows_before_it() {
    let Some(connection) = connect_as(Access::Writable).await else {
        return;
    };
    let covers = covers("covers_undone");
    on_its_own_tables("covers_undone", &[&covers[0], &covers[1]], async move {
        let (columns, second) = row_of(&connection, "covers_undone", 2).await;
        let mut changes = changes_to(
            "covers_undone",
            vec![by_id(
                2,
                vec![cell(&columns, &second, "kind", "varchar(20)", to("audio"))],
            )],
        );
        changes.inserts = vec![
            RowInsert {
                set: vec![sets("publisher_id", "bigint", "9100000000000000004")],
            },
            // UNIQUE: the first cover has it.
            RowInsert {
                set: vec![
                    sets("publisher_id", "bigint", "9100000000000000004"),
                    sets("isbn", "varchar(32)", "978-1-4028-9462-6"),
                ],
            },
        ];
        let outcome = within(connection.write(&changes, &StopFlag::new()))
            .await
            .unwrap();
        assert!(
            matches!(
                &outcome,
                // MySQL's 1062, by its SQLSTATE.
                WriteOutcome::FailedInsert { insert: 1, error: Error::Query { code, .. } }
                    if code.as_deref() == Some("23000")
            ),
            "{outcome:?}"
        );
        // Neither the first new row nor the change is there.
        assert_eq!(page_of(&connection, "covers_undone").await.1.len(), 2);
        assert_eq!(row_of(&connection, "covers_undone", 2).await.1, second);
    })
    .await;
}

#[tokio::test]
async fn a_table_without_a_key_takes_a_new_row_and_cannot_hand_it_back() {
    let Some(connection) = connect_as(Access::Writable).await else {
        return;
    };
    on_its_own_tables(
        "cover_stamps",
        &["CREATE TABLE cover_stamps (line VARCHAR(20) DEFAULT 'none', n INT DEFAULT 7)"],
        async move {
            let mut changes = changes_to("cover_stamps", Vec::new());
            // Nothing set: `() VALUES ()`.
            changes.inserts = vec![RowInsert { set: Vec::new() }];
            let outcome = within(connection.write(&changes, &StopFlag::new()))
                .await
                .unwrap();
            let WriteOutcome::Written { inserted, .. } = outcome else {
                panic!("the save wrote");
            };
            // Written, and no key to find it by.
            assert_eq!(inserted, [None]);
            assert_eq!(
                page_of(&connection, "cover_stamps").await.1,
                [vec![Value::Text("none".into()), Value::Int(7)]]
            );
        },
    )
    .await;
}
```

And beside `a_table_that_lost_its_transactions_while_the_save_waited_is_refused`, its sibling for a save that changes no row. It is that test with another set and another last check: the save has no row to read, so what makes it wait for the table, and ask for the engine only once it has it, is the read that takes the table.

```rust
/// A save of new rows alone reads no row, and still asks for the engine
/// only once it holds the table.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_save_of_new_rows_alone_holds_its_table_before_it_asks_for_the_engine() {
    let Some(connection) = connect_as(Access::Writable).await else {
        return;
    };
    let mut admin = admin().await;
    let engines: Vec<String> = admin
        .query("SELECT engine FROM information_schema.engines WHERE support IN ('YES', 'DEFAULT')")
        .await
        .unwrap();
    if !engines.iter().any(|engine| engine == "MyISAM") {
        eprintln!("skipped: the server has no MyISAM");
        return;
    }
    on_its_own_tables(
        "insert_altered",
        // No key longer than MyISAM takes.
        &[
            "CREATE TABLE insert_altered (id INT PRIMARY KEY, name VARCHAR(50))",
            "INSERT INTO insert_altered VALUES (1, 'Ada')",
        ],
        async move {
            let mut changes = changes_to("insert_altered", Vec::new());
            changes.inserts = vec![RowInsert {
                set: vec![sets("id", "int", "2"), sets("name", "varchar(50)", "Grace")],
            }];
            let mut theirs = self::admin().await;
            theirs
                .query_drop("LOCK TABLES insert_altered WRITE")
                .await
                .unwrap();
            let connection = std::sync::Arc::new(connection);
            let saving = {
                let connection = std::sync::Arc::clone(&connection);
                tokio::spawn(async move { connection.write(&changes, &StopFlag::new()).await })
            };
            waits_on_the_server(&mut admin, "insert_altered", "metadata lock").await;
            theirs
                .query_drop("ALTER TABLE insert_altered ENGINE = MyISAM")
                .await
                .unwrap();
            theirs.query_drop("UNLOCK TABLES").await.unwrap();
            let outcome = within(saving).await.unwrap();
            assert!(matches!(outcome, Err(Error::Unsupported(_))), "{outcome:?}");
            // Nothing was written: on this engine it could not have been undone.
            assert_eq!(page_of(&connection, "insert_altered").await.1.len(), 1);
            assert!(nothing_holds(&mut admin, "insert_altered").await);
        },
    )
    .await;
}
```

(`waits_on_the_server` looks for a statement of the save that names the table and waits in that state. If it finds the save by the text of its row read, give it the text `hold` sends.)

- [ ] **Step 2: Run them to see them fail.**

Run: `~/.cargo/bin/cargo test --locked -p tabletist-db --test mysql new_row` (with `TABLETIST_TEST_MYSQL_URL` set)
Expected: FAIL, `Unsupported("adding rows is not built for this engine yet")`, the sibling test too (where the server has MyISAM).

- [ ] **Step 3: Write the code.** `crates/tabletist-db/src/mysql/write.rs`. In `save`, the refusal of task 2 goes, and after the loop that builds `statements`:

```rust
        let mut inserts = Vec::with_capacity(changes.inserts.len());
        for (insert, row) in changes.inserts.iter().enumerate() {
            match build_insert(&changes.object, row) {
                Ok(built) => inserts.push(built),
                Err(error) => return Ok(WriteOutcome::FailedInsert { insert, error }),
            }
        }
```

After `build`:

```rust
/// The `INSERT` of one new row, as the driver would send it as it stands
/// (see [`build`]).
fn build_insert(object: &ObjectRef, row: &RowInsert) -> Result<Sql> {
    // MySQL matches a column's name without regard to case.
    if let Some(name) = named_twice(row, |a, b| a.to_lowercase() == b.to_lowercase()) {
        return Err(Error::query(format!(
            "{name} is set twice in one new row"
        )));
    }
    let sql = Dialect::MySql.insert_row(object, row)?.sql;
    if driver_parameter(&sql.text).is_err() {
        return Err(Error::query(
            "the MySQL driver would read part of a name as a parameter, so the save cannot \
             be sent",
        ));
    }
    Ok(sql)
}

/// What finds a row an `INSERT` made: the table's primary key, and the
/// column its counter fills.
struct FoundBy {
    key: Vec<String>,
    counter: Option<String>,
}

impl FoundBy {
    /// Asked inside the save's transaction, once its table is held (by the
    /// locking reads of its changed rows, or by [`hold`]): nobody changes
    /// the key between this and the rows it is used to read.
    async fn of(conn: &mut mysql_async::Conn, object: &ObjectRef, stop: &StopFlag) -> Result<Self> {
        let at = (&object.schema, &object.name, &object.schema, &object.name);
        not_stopped(stop)?;
        // The names by their bytes as well, as `transactional` matches
        // them, and each with a `LIMIT` of its own: a server's default
        // `sql_select_limit` can be 0.
        let key: Vec<mysql_async::Row> = conn
            .exec(
                "SELECT column_name FROM information_schema.key_column_usage \
                 WHERE table_schema = ? AND table_name = ? \
                   AND CAST(table_schema AS BINARY) = CAST(? AS BINARY) \
                   AND CAST(table_name AS BINARY) = CAST(? AS BINARY) \
                   AND constraint_name = 'PRIMARY' \
                 ORDER BY ordinal_position LIMIT 64",
                at,
            )
            .await
            .map_err(query_error)?;
        not_stopped(stop)?;
        let counter: Vec<mysql_async::Row> = conn
            .exec(
                "SELECT column_name FROM information_schema.columns \
                 WHERE table_schema = ? AND table_name = ? \
                   AND CAST(table_schema AS BINARY) = CAST(? AS BINARY) \
                   AND CAST(table_name AS BINARY) = CAST(? AS BINARY) \
                   AND extra LIKE '%auto_increment%' \
                 LIMIT 1",
                at,
            )
            .await
            .map_err(query_error)?;
        Ok(Self {
            key: key.into_iter().map(from_row).collect::<Result<_>>()?,
            counter: counter.into_iter().next().map(from_row).transpose()?,
        })
    }

    /// The key of the row `insert` made, whose counter gave it `id`: each
    /// column of the primary key with the counter's value where it is the
    /// counter's column and the counter gave one (a `0` or a NULL sent for
    /// it is not what was stored), and otherwise the value sent for it.
    /// `None` when the row cannot be found again for sure: the
    /// table has no primary key, the database filled a key column some
    /// other way (a default), or a key column's value cannot be matched
    /// exactly.
    fn key(&self, insert: &RowInsert, id: Option<u64>) -> Option<Vec<(String, Value)>> {
        if self.key.is_empty() {
            return None;
        }
        self.key
            .iter()
            .map(|name| {
                let counted = id.filter(|_| self.counter.as_deref() == Some(name.as_str()));
                let sent = insert.set.iter().find(|cell| cell.column == *name);
                let value = match (counted, sent) {
                    (Some(id), _) => Value::Int(i64::try_from(id).ok()?),
                    (None, Some(cell)) => sent_key(cell)?,
                    (None, None) => return None,
                };
                Some((name.clone(), value))
            })
            .collect()
    }
}

/// A value sent for a key column, as what finds its row again. A whole
/// number goes as a number: bound as text, MySQL would compare it with the
/// column as two doubles, and past 2^53 find a neighbour. Text goes as
/// text. Every other class is one whose stored form is not the text that
/// was sent (a decimal is rounded, a float is not its text, a date is
/// normalised), and its row is not looked for.
fn sent_key(cell: &InsertValue) -> Option<Value> {
    let NewValue::Text(text) = &cell.new else {
        return None;
    };
    if inexact(&cell.type_name).is_some() {
        return None;
    }
    match column_class(Dialect::MySql, &cell.type_name) {
        ColumnClass::Integer { .. } => text.trim().parse().ok().map(Value::Int),
        ColumnClass::Text { .. } => Some(Value::Text(text.as_str().into())),
        ColumnClass::Decimal { .. }
        | ColumnClass::Float
        | ColumnClass::Boolean
        | ColumnClass::Json
        | ColumnClass::Binary
        | ColumnClass::Other => None,
    }
}

/// A read that takes the table and no row of it. A save that changes rows
/// holds its table from its first locking read on; one of new rows alone
/// reads nothing before it asks for the table's engine, and without this
/// someone could still change the engine, or the key, before the first
/// `INSERT` (see the comment in [`apply`]). A locking read, so it takes no
/// snapshot either.
fn hold(object: &ObjectRef) -> Result<Sql> {
    let sql = Sql {
        text: format!(
            "SELECT 1 FROM {} LIMIT 0 FOR UPDATE",
            Dialect::MySql.qualified(object)
        ),
        params: Vec::new(),
    };
    if driver_parameter(&sql.text).is_err() {
        return Err(Error::query(
            "the MySQL driver would read part of a name as a parameter, so the save cannot \
             be sent",
        ));
    }
    Ok(sql)
}

/// Runs one new row's `INSERT`. The inner `Err` is the statement's own
/// failure: the server's error, or a warning (see [`update`]). The outer is
/// a cancel, a stop or a lost session. `Ok(Ok(id))` is the value the
/// table's counter gave the row, when it gave one.
async fn insert_row(
    conn: &mut mysql_async::Conn,
    sql: &Sql,
    stop: &StopFlag,
) -> Result<std::result::Result<Option<u64>, Error>> {
    let statement = match prepare(conn, sql, stop).await {
        Ok(statement) => statement,
        Err(error @ (Error::Cancelled | Error::ConnectionLost(_))) => return Err(error),
        Err(error) => return Ok(Err(error)),
    };
    not_stopped(stop)?;
    if let Err(error) = conn.exec_drop(&statement, params(&sql.params)).await {
        return match query_error(error) {
            error @ (Error::Cancelled | Error::ConnectionLost(_)) => Err(error),
            error => Ok(Err(error)),
        };
    }
    // Read before the warnings are asked for: that is a statement too.
    let id = conn.last_insert_id().filter(|id| *id > 0);
    // Outside strict mode a column without a default takes its type's
    // zero, and a value is cut to fit, with only a warning. What was
    // stored is then not what was asked for, and the save fails.
    if conn.get_warnings() > 0 {
        return warning(conn).await.map(Err);
    }
    Ok(Ok(id))
}
```

`apply` is called with the inserts (`apply(&mut conn, changes, &statements, &inserts, stop)`) and takes `inserts: &[Sql]` after `statements`.

In it, between the loop of locking reads and the `not_stopped(stop)?; transactional(conn, &changes.object).await?;` that follows `same_row_twice`, a save with no changed row takes its table:

```rust
    // No changed row, so no read above holds the table: take it before
    // the engine is asked for.
    if changes.rows.is_empty() {
        let (_, none) = rows(conn, &hold(&changes.object)?, stop).await?;
        debug_assert!(none.is_empty());
    }
```

Then, after the conflicts are answered and before the loop over `statements` that updates:

```rust
    // The new rows, before any row is changed: the order a review shows
    // them in. What finds each again is noted as it is made, and asked for
    // only here, after the plain read above took the transaction's
    // snapshot (see the comment over it).
    let found_by = if inserts.is_empty() {
        None
    } else {
        Some(FoundBy::of(conn, &changes.object, stop).await?)
    };
    let mut made = Vec::with_capacity(inserts.len());
    for (insert, (row, sql)) in changes.inserts.iter().zip(inserts).enumerate() {
        match insert_row(conn, sql, stop).await? {
            Ok(id) => made.push(found_by.as_ref().and_then(|by| by.key(row, id))),
            Err(error) => return Ok(Applied::FailedInsert { insert, error }),
        }
    }
```

and after the loop that reads the changed rows back, in place of `Ok(Applied::Rows(saved))`:

```rust
    // The new rows last, as they stand once every statement has run.
    let mut inserted = Vec::with_capacity(made.len());
    for key in made {
        let Some(key) = key else {
            inserted.push(None);
            continue;
        };
        let select = Dialect::MySql.select_row(&changes.object, &key, false);
        // A name the driver would read as a parameter: not looked for.
        if driver_parameter(&select.text).is_err() {
            inserted.push(None);
            continue;
        }
        let (_, mut found) = rows(conn, &select, stop).await?;
        // More than one row by a primary key is no key at all.
        if found.len() > 1 {
            return Err(more_than_one());
        }
        // None: a trigger changed the row's key, and the row is unknown.
        inserted.push(found.pop());
    }
    Ok(Applied::Rows(Stored {
        inserted,
        rows: saved,
    }))
```

Add what the new code names to the file's imports: `named_twice` from `crate::write` (`Stored` came in task 2, `from_row` is there), and `ColumnClass`, `InsertValue`, `NewValue`, `RowInsert`, `column_class` from `crate`.

As built: `hold`'s statement is made in `save` with the others, before anything is sent, and `apply` takes the save's statements as one value (`Written { statements, inserts, hold }`) in place of three arguments. The code blocks above show the first shape; the tree has the second.

Extend the comment over the second `transactional` call in `apply` with one sentence: "A save of new rows alone has no such read, and takes the table with one that reads no row (`hold`)."

- [ ] **Step 4: Run the tests.**

Run: `~/.cargo/bin/cargo test --locked -p tabletist-db --test mysql` and `~/.cargo/bin/cargo test --locked -p tabletist-db --lib mysql::` (with `TABLETIST_TEST_MYSQL_URL` set)
Expected: PASS, none skipped.

- [ ] **Step 5: Run the four checks, then commit.**

```bash
git add -A crates
git commit -m "Write a save's new rows on MySQL, and find each again by its key"
```

---

### Task 7: Say what is built, and what waits

**Files:**
- Modify: `docs/superpowers/specs/2026-10-07-inserting-rows-design.md` (its status paragraph)
- Modify: `crates/tabletist-db/src/lib.rs` (`Connection::write`'s doc comment)

- [ ] **Step 1: The spec's status.** Replace its first paragraph ("Date: 2026-10-07. Status: planned in runs. Run 1 …") with:

```markdown
Date: 2026-10-07. Status: built in runs, listed in
`docs/superpowers/plans/2026-10-07-inserting-rows-1-save.md`. Run 1 is
built: a save carries new rows and the three drivers write them. Nothing in
the app adds a row yet; that is run 2. The plan's "What the design asks, and
what gets built" says what of the text below the app can do, and in which
run.
```

- [ ] **Step 2: The call's doc.** In `crates/tabletist-db/src/lib.rs`, in the doc comment over `Connection::write`, add after its sentence about one transaction:

```rust
    /// The set's new rows are written first, each by one `INSERT`, then
    /// its changed rows. A new row comes back as the database stored it,
    /// or as `None` where MySQL cannot find it again.
```

- [ ] **Step 3: Run the four checks, then commit.**

```bash
git add -A docs crates
git commit -m "Say that a save writes new rows, and that adding one in the grid is next"
```

---

## After the last task

- Run the four checks once more on the branch's head, and the three database suites with their variables set. Report what ran and what was only compiled.
- No user-visible behaviour changed, so the website's pages, `SHORTCUTS` and the README stay as they are.
- Stop here. Run 2 gets its own plan, written from the tree as this run leaves it, starting from the rows h to p of "What the design asks, and what gets built".
