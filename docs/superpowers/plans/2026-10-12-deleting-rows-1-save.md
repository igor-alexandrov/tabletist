# Deleting Rows, Run 1: The Save Deletes Rows Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A save can carry rows to delete beside its new and changed rows: `tabletist-db` builds one `DELETE` per row, by the row's key, runs them in the save's one transaction after its `UPDATE`s on PostgreSQL, MySQL and SQLite, says which of them were already gone, and fails the row a foreign key holds.

**Architecture:** `ChangeSet` gains `deletes: Vec<RowDelete>` beside `inserts` and `rows`, so there is still one set, one `Connection::write` and one transaction. `Dialect::delete_row` builds the statement a user reads and the one the driver sends from the same key, as `update_row` does. Each driver reads every row to delete under its lock in the phase that reads the changed rows, runs the deletes after the updates, and answers with `WriteOutcome::Written { gone, .. }` (the deletes whose row was not there) or `WriteOutcome::FailedDelete`.

**Tech Stack:** Rust 1.98, `tokio-postgres`, `mysql_async`, `rusqlite` (bundled SQLite). Spec: `docs/superpowers/specs/2026-10-12-deleting-rows-design.md` ("spec N" below is a section of its Behavior, "test N" an entry of its Tests).

Status: built on 2026-10-12, one commit per task. Each task's blocks are the plan's as written, but for these:

- Task 4, step 2: six of the eight SQLite tests failed before the driver deleted, as the step says, but not the six it names, and with "no such column: id": from task 3 on the comparison of the rows a save read is given the deletes' keys, and no read of theirs yet. So between tasks 3 and 4 a set with deletes was refused on every engine, not written with nothing deleted. `sqlite_writes_new_rows_before_it_deletes` failed with the rest, and `sqlite_refuses_a_key_the_table_spells_otherwise` passed, for that reason and not its own.
- Task 4: `sqlite_refuses_a_key_the_table_spells_otherwise` asserts the error's words ("spells ID another way"), since any error passed it.
- Task 6: `delete` matches a cancel and a lost session itself, as the drivers' loops do, and does not go through `own`.
- PostgreSQL's and MySQL's seven tests each were compiled and not run: no server can be started in the session. CI runs them.

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

- `src/shots.rs` is outside those four and builds a `WriteOutcome` by hand. Task 1 gives that type a field, so it also runs `~/.cargo/bin/cargo clippy --locked --features shots --lib --tests -- -D warnings`.
- **What was run where this plan was written: nothing of it.** It was written from reading the tree at `89a7e88` and the design canvas. The code blocks are written against names that exist in that tree and were not compiled. Treat a block as what the code should come to, and expect to correct a name, a type or a borrow on the first build. Where a block and the compiler disagree, the compiler is right and the task's tests say what must hold. Where a test's expected SQL and the builder's differ only in how a name or a value is written, the builder is right: fix the expectation and say so in the commit.
- **Databases.** Tasks 5 and 6 add tests that need servers. There is no Docker in a Claude session (`docker.sock` is refused), so there those tests print "skipped" and pass, which proves nothing. Say so in the task's report and in the pull request: "PostgreSQL and MySQL delete tests were compiled, not run; CI runs them." Where servers can be started:

```bash
docker compose up -d --build --wait postgres mysql
export TABLETIST_TEST_PG_URL=postgres://tabletist:tabletist@localhost:55432/tabletist
export TABLETIST_TEST_MYSQL_URL=mysql://tabletist:tabletist@localhost:53306/tabletist
```

- **The branch** is `claude/row-deletion-table-view-903d1e`, in the worktree this plan is in, on `main` at `89a7e88`. Before task 1, commit the spec and this plan on their own.
- House rules that bite here: no em dashes anywhere; comments explain why, in the surrounding code's voice; code that depends on the engine matches on `Dialect` and names every variant (no `==`, `!=`, `matches!` or `_` arm; `tests/engines.rs` finds the first three); `crates/tabletist-db` has no UI dependency; what a page loaded never reaches a log (`ChangeSet`'s `Debug` prints counts, not values); do not weaken a lint or delete a test to get green.
- Commits are signed, one per task, after its checks pass. If signing fails ("agent refused operation"), 1Password is locked: do not commit unsigned, stage the task and tell the user. Chain `git add` and `git commit` with `&&`. Each message ends with:

      Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>

- `cargo test` takes one name to filter by before `--` and any number after it.

## The runs

The spec is one feature and one pull request. It is planned as runs that each work alone, each with its own plan, as inserting rows was. Each run's commits go on this branch.

| Run | What it builds | Seen by a user |
|---|---|---|
| **1 (this plan)** | The save deletes rows: `DELETE` in the change set, the builder, the three drivers, "already gone", a foreign key's refusal | No |
| 2 | Marking in the grid and the inspector: ⌫, `dd`, Delete and Restore, the marked row in both looks, the reasons a row cannot be deleted, the counts, Review SQL and `:diff`, the production confirm, the save's results, the labels and announcements, the website's pages | Yes |
| 3 | What a delete takes with it: the foreign keys that point at the table, the `ON DELETE CASCADE` and `SET NULL` warnings with their counts, and a refused delete's "3 rows in book_covers still point at this row · Open rows" | Yes |
| 4 | Many rows at once: a selection of rows, `d` on a visual selection, `{count}dd`, the cap of 1,000, and `IN (…)` batches of 100. See row b below: this needs parts the app does not have, and is better a spec of its own | Yes |

Run 2 is the first a user sees, and the longest: about as much work as Add row was. Its plan is written when run 1 is in, from the tree as it then stands.

## What the spec asks, and what the app can do today

Read from the tree at `89a7e88` and from the canvas' boards (`OmarchyWideEdit`, `OmarchyEdit`, `OmarchyEditValues`, `OmarchyInsert`, `Main`, `MainEdit`, `MacEditFlow`, `MacInsertStates`, both `MacComponents`). Each row is the user's to overrule: rows marked "1" before this run, the rest before run 2.

| | The spec | What the app has | What gets built | Run |
|---|---|---|---|---|
| a | ⌘Z and `u` unmark a row; "undo and redo cover mark and unmark" | No undo stack. ⌘Z and `u` put one pending cell back (`Action::RevertCell`). Redo is in the keymap as not built | ⌘Z and `u` on a marked row unmark it, before they touch a cell. No redo, and no undo of a restore | 2 |
| b | Several selected rows are marked at once; `d` in a visual row selection; `{count}dd`; a cap of 1,000 | The selection is one cell (`selection: Option<CellPos>`). There is no row selection, no visual mode and no count before a key in any look | One row for each press. Marking many rows first needs a row selection in the desktop looks, a visual mode and counts in the Omarchy look: run 4, or a spec of its own. The cap and the batches come with it | 4 |
| c | No DELETE privilege disables the command: "No permission to delete from {table}" | The app reads no INSERT, UPDATE or DELETE privilege, by a decision of the insert audit (INS-04c): "the save is where the user learns it" | Not disabled. The server's refusal (42501, MySQL 1142) is the row's error after the save, in its own words | 2 |
| d | In a query's result: "Rows from a query can't be deleted here" | A result's row panel has no Duplicate and no Delete, and ⌫ and `dd` are not keys of a result's grid | A result keeps no Delete button. Whether ⌫ and `dd` can say the sentence there depends on the keymap's scopes: run 2's plan says | 2 |
| e | Values are always bound: `DELETE FROM book_covers WHERE id = $1;` | PostgreSQL saves send literals through the simple-query protocol, so a loaded value and a saved one are the same `Value`. MySQL and SQLite bind. Names are qualified and quoted | The same: `DELETE FROM "public"."book_covers" WHERE "id" = 9` on PostgreSQL, the key bound on MySQL and SQLite. What Review SQL shows is what is sent | 1 |
| f | Batches of up to 100 rows in one `IN (…)` | One statement per changed row. A batch that a foreign key refuses cannot say which of its rows was held | One `DELETE` per row, which is the spec's first line. Batches come with row b | 4 |
| g | 0 rows affected means the row is already gone | Every driver finds a row by its key and locks it before it writes | A row to delete is read under the lock first. Not found: it is reported as already gone, no `DELETE` is sent for it, and the save goes on. Found: its `DELETE` must remove exactly one row. The same result, and the checks that a key names one row and is matched exactly (a MySQL `float` key finds no row) still hold: such a key is an error, never "already deleted" | 1 |
| h | A warning for each `ON DELETE CASCADE` or `SET NULL` key that points at the table, with a count | The structure holds a table's own foreign keys. Nothing reads the keys that point at it, and nothing counts referencing rows | Run 3: the incoming keys per driver (SQLite has no pragma for them: every table's list is read), the counts, the lines in Review SQL, `:diff` and the production confirm | 3 |
| i | A refused delete says "3 rows in book_covers still point at this row · Open rows" | PostgreSQL names the constraint, MySQL after this run (error 1451), SQLite names nothing ("FOREIGN KEY constraint failed") | Run 2 shows the server's message on the row. Run 3 adds the count and Open rows, from row h's incoming keys | 2, 3 |
| j | Error navigation (⌥⌘↓ / ⌥⌘↑, `]e` / `[e`) includes delete errors | Those commands are in the keymap as not built. Nothing lists the rows with errors | Not built here. A failed delete marks its row as in trouble, which is what error navigation will walk when it is built | later |
| k | "{object} can't be deleted from" and "{table} has no primary key, so rows can't be deleted here" | The same cases say "Views cannot be edited" and "{table} has no primary key or unique index, so a row can't be targeted safely" for editing | Delete's own sentences, as the spec writes them. A table without a primary key but with a unique index over NOT NULL columns has a key in this app, and its rows can be deleted | 2 |
| l | "`{n} pending` as today" in the Omarchy status line | Today's line reads "3 pending · 2 rows" and "+1 new": new rows are counted apart | A third count, "-1 to delete", in the danger colour | 2 |
| m | "Save 1 insert, 1 update and 1 delete to bookshop_production?" | "Save 1 new row and 2 changes to production?", the connection and its database on the line under it | "Save 1 new row, 2 changes and 1 deleted row to production?": the app's words, with the deletes counted | 2 |
| n | The banner: "Nothing was saved. N rows failed, so the whole transaction rolled back." | A save stops at its first failing statement, so one row fails at a time | "Nothing was saved. 1 row could not be deleted, so the whole transaction rolled back." | 2 |
| o | A changed row and a deleted row in one save | The changed rows are read back after the save's statements | The deletes run before the rows are read back. Where a delete takes a changed row with it (a table whose key points at itself with `ON DELETE CASCADE`), the save fails and is undone: "a saved row could not be read back" | 1 |
| q | Rows are deleted in the order they were marked | One `DELETE` per row, in the set's order | On a table whose key points at itself without an action (`RESTRICT`, `NO ACTION`), a parent marked before its child fails, though deleting both is allowed. The row's error says which key holds it. Ordering a save's deletes by the table's own keys is run 3's, with the incoming keys | 3 |
| p | Tests use only the Bookshop data | The database crate's older tests use the crate's own fixture | This run's tests make a Bookshop of their own in each engine (`publishers`, `book_covers`, `book_authors`) | 1 |

## What was decided for this run

- **A delete names its row by the row's key and nothing else.** `RowDelete { key }`. No loaded value is compared: the spec says not to add checks on other columns. A row someone else changed is deleted as it now stands.
- **A row cannot be changed and deleted in one save.** `ChangeSet::check` refuses a set whose delete has the key of one of its changed rows. Run 2 leaves a marked row's edits in the store and out of the set (spec 1, "Existing edits"), so the app never sends one.
- **Already gone is part of a save that went through.** `WriteOutcome::Written` gains `gone: Vec<usize>`, the places in the set's `deletes` whose row was not there. It is not a conflict: a conflict writes nothing, and the spec keeps the rest of the save.
- **A delete's failure is its own outcome.** `WriteOutcome::FailedDelete { delete, error }`, beside `Failed` and `FailedInsert`. Nothing was written.
- **A row the save itself took is deleted, not failed.** On a table whose key points at itself with `ON DELETE CASCADE` (a tree of shelves), deleting a shelf takes its children. Where a child is also in the set, its own `DELETE` then finds nothing. The driver reads the row again: not there, the save removed it and goes on; still there, something kept it (a trigger, a rule) and the save fails.
- **A new row is not known where a delete can reach it.** On PostgreSQL and SQLite an inserted row is handed back as its `INSERT` left it. A save that also deletes, on a table one of whose foreign keys acts on a delete, hands its new rows back as not known, and the app reads the page again: the new row can name a row the save deletes, straight or through another table. That is as cautious as the same question about updates. MySQL finds each new row again after every statement, so it needs nothing.
- **Between tasks 1 and 6 a set with deletes passes `check` on a driver that does not delete yet**, and such a save answers `Written` with nothing deleted. The app sends no delete, so nothing meets it. Do not try the branch's deletes by hand before task 6.
- **A MySQL delete does not fail on a warning.** An update fails on one because what was stored may not be what was typed. A delete stores nothing.
- **MySQL's error 1451 names its constraint.** `mysql::named` reads it as it reads 1452's. Its columns are left out: they are the referencing table's, and the app looks for a named column in the table it saved to.
- **The app sends no delete yet.** `src/edit.rs` builds every set with `deletes: Vec::new()`. The reducer's `written` takes `FailedDelete` as a refused save until run 2 gives it a row to stand on.

## File map

| File | What changes |
|---|---|
| `crates/tabletist-db/src/write.rs` | `RowDelete`; `ChangeSet::deletes`; its checks and its `Debug`; `WriteOutcome::Written::gone`, `WriteOutcome::FailedDelete`; `Stored::gone`; `same_row_twice` by keys, `read_by` (task 3); `Applied::FailedDelete`, `key_spelled_otherwise` (task 4) |
| `crates/tabletist-db/src/dialect.rs` | `RowDeletion`, `DeleteParts`, `Dialect::delete_row` |
| `crates/tabletist-db/src/lib.rs` | The new names exported; `Connection::write`'s doc |
| `crates/tabletist-db/src/sqlite/write.rs` | The deletes of a SQLite save |
| `crates/tabletist-db/src/pg/write.rs` | The deletes of a PostgreSQL save |
| `crates/tabletist-db/src/mysql/write.rs` | The deletes of a MySQL save |
| `crates/tabletist-db/src/mysql.rs` | `named` reads error 1451 |
| `crates/tabletist-db/tests/delete_rows.rs` | New. The Bookshop in each engine, and what a save that deletes does there |
| `crates/tabletist-db/tests/*.rs`, `src/**` | Every `ChangeSet` built by hand gains `deletes: Vec::new()`, every `Written` built by hand `gone: Vec::new()` |
| `src/edit.rs` | `change_set` sends no deletes; `same_changes` compares them |
| `src/app/editing.rs` | `written` reads the two new outcomes |

---

### Task 1: A change set names rows to delete

**Files:**
- Modify: `crates/tabletist-db/src/write.rs`
- Modify: `crates/tabletist-db/src/lib.rs:54-56` (the `pub use write::{..}`)
- Modify: `src/edit.rs` (`change_set`, `same_changes`), `src/app/editing.rs` (`written`)
- Modify: every file the compiler names for a missing field (tests, `src/shots.rs`, `src/review.rs`, `src/backend.rs`, `src/ui/review.rs`)

- [ ] **Step 1: Write the failing tests**

In `crates/tabletist-db/src/write.rs`, in `mod tests`, give the helper `set` its new field and add a helper and four tests:

```rust
    fn set(rows: Vec<RowChange>) -> ChangeSet {
        ChangeSet {
            object: ObjectRef::new("main", "users"),
            inserts: Vec::new(),
            rows,
            deletes: Vec::new(),
        }
    }

    fn doomed(key: Vec<(&str, Value)>) -> RowDelete {
        RowDelete {
            key: key
                .into_iter()
                .map(|(name, value)| (name.to_owned(), value))
                .collect(),
        }
    }

    fn deleting(deletes: Vec<RowDelete>) -> ChangeSet {
        ChangeSet {
            object: ObjectRef::new("public", "book_covers"),
            inserts: Vec::new(),
            rows: Vec::new(),
            deletes,
        }
    }

    #[test]
    fn a_set_of_rows_to_delete_alone_is_a_save() {
        let changes = deleting(vec![
            doomed(vec![("id", Value::Int(9))]),
            doomed(vec![("id", Value::Int(10))]),
        ]);
        assert_eq!(changes.check(), Ok(()));
        assert!(deleting(Vec::new()).check().is_err());
    }

    #[test]
    fn a_row_to_delete_needs_a_key_that_finds_one_row() {
        let said = |deletes: Vec<RowDelete>| deleting(deletes).check();
        assert_eq!(
            said(vec![doomed(Vec::new())]),
            Err(Error::query("a row to delete has no key"))
        );
        assert_eq!(
            said(vec![doomed(vec![("id", Value::Null)])]),
            Err(Error::query(
                "a row to delete cannot be found: its key is NULL"
            ))
        );
        assert_eq!(
            said(vec![doomed(vec![
                ("book_id", Value::Int(101)),
                ("book_id", Value::Int(102)),
            ])]),
            Err(Error::query("book_id is named twice in the row's key"))
        );
        // The same row twice, its key's columns in either order.
        let author = |name: &str| Value::Text(name.into());
        assert_eq!(
            said(vec![
                doomed(vec![
                    ("book_id", Value::Int(101)),
                    ("author", author("Ada Quill")),
                ]),
                doomed(vec![
                    ("author", author("Ada Quill")),
                    ("book_id", Value::Int(101)),
                ]),
            ]),
            Err(Error::query("two rows to delete have the same key"))
        );
    }

    #[test]
    fn a_row_is_not_changed_and_deleted_in_one_save() {
        let changes = ChangeSet {
            deletes: vec![doomed(vec![("id", Value::Int(2))])],
            ..set(vec![row(vec![("id", Value::Int(2))], vec![cell("name")])])
        };
        assert_eq!(
            changes.check(),
            Err(Error::query("a row to save is both changed and deleted"))
        );
        // Another row's delete beside it is a save like any other.
        let changes = ChangeSet {
            deletes: vec![doomed(vec![("id", Value::Int(3))])],
            ..set(vec![row(vec![("id", Value::Int(2))], vec![cell("name")])])
        };
        assert_eq!(changes.check(), Ok(()));
    }

    #[test]
    fn a_set_is_printed_without_the_keys_of_its_rows_to_delete() {
        let changes = deleting(vec![doomed(vec![("id", Value::Int(4711))])]);
        let printed = format!("{changes:?}");
        assert!(printed.contains("deletes: 1"), "{printed}");
        assert!(!printed.contains("4711"), "{printed}");
    }
```

In the existing test `a_set_is_printed_by_its_counts_without_the_values_it_carries`, the printed set gains its new count. Change the expected text to:

```rust
            r#"ChangeSet { object: ObjectRef { schema: "main", name: "users" }, inserts: 0, rows: 2, cells: 3, deletes: 0, .. }"#
```

- [ ] **Step 2: Run them to see them fail**

Run: `~/.cargo/bin/cargo test --locked -p tabletist-db --lib write::tests`
Expected: FAIL to compile, "cannot find struct `RowDelete`" and "struct `ChangeSet` has no field named `deletes`".

- [ ] **Step 3: The types**

In `crates/tabletist-db/src/write.rs`, `ChangeSet` gains a field and its `Debug` a count:

```rust
/// Every change of one save, to one table. Written in one transaction, or
/// not at all.
#[derive(Clone, PartialEq)]
pub struct ChangeSet {
    pub object: ObjectRef,
    /// The new rows. They are written before `rows` are changed.
    pub inserts: Vec<RowInsert>,
    pub rows: Vec<RowChange>,
    /// The rows to delete. They go last, after every row is changed.
    pub deletes: Vec<RowDelete>,
}
```

```rust
        f.debug_struct("ChangeSet")
            .field("object", &self.object)
            .field("inserts", &self.inserts.len())
            .field("rows", &self.rows.len())
            .field("cells", &cells)
            .field("deletes", &self.deletes.len())
            .finish_non_exhaustive()
```

After `CellChange`, the new type:

```rust
/// A row to delete: what finds it, and nothing more. No other column is
/// compared with what the page loaded: a row someone changed since is
/// deleted as it now stands, and one that is already gone is no failure
/// (see [`WriteOutcome::Written`]).
#[derive(Debug, Clone, PartialEq)]
pub struct RowDelete {
    /// The row key's columns (`Structure::row_key`) and their loaded
    /// values, as a [`RowChange`]'s.
    pub key: Vec<(String, Value)>,
}
```

`WriteOutcome::Written` gains `gone` after `rows`, and the enum a variant after `FailedInsert`. The enum's first line, "How a save ended. Only `Written` changed anything.", stays true.

```rust
        /// Each changed row as the database now holds it, in the set's
        /// order.
        rows: Vec<Vec<Value>>,
        /// The places, in the set's `deletes`, of the rows that were not
        /// there to delete: someone deleted them first. Every other row
        /// of `deletes` is deleted, and the rest of the save is written.
        gone: Vec<usize>,
        elapsed: Duration,
```

```rust
    /// The statement of `deletes[delete]` could not be built or failed: a
    /// foreign key that holds the row, most often. Nothing was written.
    FailedDelete { delete: usize, error: Error },
```

In the doc of `Written::inserted`, the second bullet names updates only. A delete can reach a new row too. Replace that bullet with:

```rust
        /// - on PostgreSQL and SQLite, in a save that also changes rows,
        ///   on a table one of whose foreign keys acts on an update, and
        ///   in a save that also deletes rows, on a table one of whose
        ///   foreign keys acts on a delete: a changed or a deleted row
        ///   can carry a new one with it;
```

- [ ] **Step 4: The checks**

In `ChangeSet::check`, the first test counts the deletes too:

```rust
        if self.rows.is_empty() && self.inserts.is_empty() && self.deletes.is_empty() {
            return Err(Error::query("there is nothing to save"));
        }
```

and after the loop over `self.rows`, before `Ok(())`:

```rust
        for (index, delete) in self.deletes.iter().enumerate() {
            if delete.key.is_empty() {
                return Err(Error::query("a row to delete has no key"));
            }
            if let Some((column, _)) = delete.key.iter().enumerate().find_map(|(index, named)| {
                delete.key[..index]
                    .iter()
                    .any(|(earlier, _)| *earlier == named.0)
                    .then_some(named)
            }) {
                return Err(Error::query(format!(
                    "{column} is named twice in the row's key"
                )));
            }
            if delete.key.iter().any(|(_, value)| value.is_null()) {
                return Err(Error::query(
                    "a row to delete cannot be found: its key is NULL",
                ));
            }
            if self.deletes[..index]
                .iter()
                .any(|earlier| same_key(&earlier.key, &delete.key))
            {
                return Err(Error::query("two rows to delete have the same key"));
            }
            // Its update would run, and then be thrown away with the row:
            // what the save meant by the two is not known.
            if self.rows.iter().any(|row| same_key(&row.key, &delete.key)) {
                return Err(Error::query("a row to save is both changed and deleted"));
            }
        }
```

- [ ] **Step 5: What a driver hands back**

`Stored` carries the list, and `outcome` passes it on. `Applied` gains its `FailedDelete` in task 4, with the first driver that makes one: a variant nothing constructs is dead code, and clippy fails on it.

```rust
/// What a save wrote, before its transaction ends.
pub(crate) struct Stored {
    pub(crate) inserted: Vec<Option<Vec<Value>>>,
    pub(crate) rows: Vec<Vec<Value>>,
    /// See [`WriteOutcome::Written`]'s `gone`.
    pub(crate) gone: Vec<usize>,
}
```

```rust
            Self::Rows(Stored {
                inserted,
                rows,
                gone,
            }) => WriteOutcome::Written {
                inserted,
                rows,
                gone,
                elapsed: started.elapsed(),
            },
            Self::Conflicts(conflicts) => WriteOutcome::Conflicts(conflicts),
            Self::Failed { row, error } => WriteOutcome::Failed { row, error },
            Self::FailedInsert { insert, error } => WriteOutcome::FailedInsert { insert, error },
```

The three drivers build a `Stored` each (`sqlite/write.rs:380`, `pg/write.rs:307`, `mysql/write.rs:797`). Give each `gone: Vec::new()` for now: tasks 4 to 6 fill it.

In `crates/tabletist-db/src/lib.rs`, export the type:

```rust
pub use write::{
    CellChange, ChangeSet, Conflict, InsertValue, NewValue, RowChange, RowDelete, RowInsert,
    WriteOutcome,
};
```

- [ ] **Step 6: Every set and every outcome built by hand**

Build the workspace's tests and let the compiler name each place:

Run: `~/.cargo/bin/cargo check --locked --workspace --all-targets`

- Every `ChangeSet { object, inserts, rows }` literal gains `deletes: Vec::new()`. They are in the crate's `crates/tabletist-db/src/write.rs`, `crates/tabletist-db/src/sqlite.rs`, `crates/tabletist-db/src/pg/write.rs` and `crates/tabletist-db/src/mysql/write.rs` (their tests), in `crates/tabletist-db/tests/sqlite.rs` (about twenty), `tests/postgres.rs`, `tests/mysql.rs` and `tests/insert_audit.rs` beside it, and in the app's `src/edit.rs`, `src/review.rs`, `src/backend.rs` and `src/ui/review.rs`.
- Every `WriteOutcome::Written { inserted, rows, elapsed }` built as a value gains `gone: Vec::new()`: in the crate's tests, and in the app's `src/app.rs`, `src/shots.rs`, `src/ui/mod.rs` and `src/ui/insert_audit_tests.rs`. A pattern that already ends in `..` needs nothing. A pattern that names every field gains `..` only where the code under it has no use for the list: say so in a comment there only if the match is the app's own (`src/app/editing.rs`, below).
- Change nothing else in those tests: no assertion moves.

In `src/edit.rs`, `same_changes` names every field of a set on purpose, so a new one fails to compile there. Compare the deletes as the rows' keys are compared:

```rust
pub fn same_changes(a: &ChangeSet, b: &ChangeSet) -> bool {
    // Every field by name: one added to a set is one to compare here.
    let (
        ChangeSet {
            object,
            inserts,
            rows,
            deletes,
        },
        ChangeSet {
            object: other,
            inserts: other_inserts,
            rows: others,
            deletes: other_deletes,
        },
    ) = (a, b);
    let same_key = |a: &[(String, Value)], b: &[(String, Value)]| {
        a.len() == b.len()
            && a.iter()
                .zip(b)
                .all(|((name, value), (other, with))| name == other && same_value(value, with))
    };
```

`same_row` then reads `same_key(key, &b.key) && set.len() == b.set.len() && ..` in place of its own comparison of the keys, and the last expression gains:

```rust
        && deletes.len() == other_deletes.len()
        && deletes
            .iter()
            .zip(other_deletes)
            .all(|(a, b)| same_key(&a.key, &b.key))
```

Add a test beside `same_changes`' own tests in `src/edit.rs` (use the module's existing way of making a set if it has one; this one stands alone):

```rust
    #[test]
    fn two_saves_that_delete_other_rows_are_not_the_same_save() {
        let deleting = |id: i64| ChangeSet {
            object: ObjectRef::new("public", "book_covers"),
            inserts: Vec::new(),
            rows: Vec::new(),
            deletes: vec![tabletist_db::RowDelete {
                key: vec![("id".to_owned(), Value::Int(id))],
            }],
        };
        assert!(same_changes(&deleting(9), &deleting(9)));
        assert!(!same_changes(&deleting(9), &deleting(10)));
        let none = ChangeSet {
            deletes: Vec::new(),
            ..deleting(9)
        };
        assert!(!same_changes(&deleting(9), &none));
    }
```

In `src/app/editing.rs`, `written` matches every outcome. Its `Written` arm (near line 1516) ignores `gone` for now, and a new arm takes `FailedDelete`:

```rust
            // No save of the app's deletes a row yet, so none can be gone.
            Ok(WriteOutcome::Written {
                inserted,
                rows,
                gone: _,
                elapsed,
            }) => {
```

```rust
            // No save of the app's deletes a row yet: there is no row to
            // put this on.
            Ok(WriteOutcome::FailedDelete { error, .. }) => {
                object.edits.note = Some(Note::Refused(error));
            }
```

Any other exhaustive match on `WriteOutcome` the compiler names takes `FailedDelete` as it takes `FailedInsert`.

- [ ] **Step 7: Run the tests**

Run: `~/.cargo/bin/cargo test --locked -p tabletist-db --lib write::tests`
Expected: PASS, the four new tests among them.

Run: `~/.cargo/bin/cargo test --locked --lib two_saves_that_delete_other_rows_are_not_the_same_save`
Expected: PASS.

- [ ] **Step 8: Run the four checks, and the shots' clippy**

Run the four checks, then `~/.cargo/bin/cargo clippy --locked --features shots --lib --tests -- -D warnings`.
Expected: all pass. PostgreSQL and MySQL tests print "skipped".

- [ ] **Step 9: Commit**

```bash
git add crates/tabletist-db src && git commit -m "Let a save name rows to delete

A change set holds the rows to delete beside its new and its changed
rows, each by its key alone. A set is refused where a row to delete has
no key that finds one row, or is also changed. A save that went through
says which of those rows were already gone, and a delete that fails is
an outcome of its own. No driver deletes yet, and the app sends none.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 2: The `DELETE` of one row, shown and sent

**Files:**
- Modify: `crates/tabletist-db/src/dialect.rs` (types after `UpdateParts`, the builder after `insert_row`, tests in `mod tests`)
- Modify: `crates/tabletist-db/src/lib.rs:42-44` (the `pub use dialect::{..}`)

- [ ] **Step 1: Write the failing tests**

In `crates/tabletist-db/src/dialect.rs`, in `mod tests`, after `covers`:

```rust
    fn doomed(key: Vec<(&str, Value)>) -> RowDelete {
        RowDelete {
            key: key
                .into_iter()
                .map(|(name, value)| (name.to_owned(), value))
                .collect(),
        }
    }

    /// Spec 4 and test 7: one statement for a row, by its key.
    #[test]
    fn the_same_delete_on_each_engine() {
        let row = doomed(vec![("id", Value::Int(9))]);
        let delete = Dialect::Postgres
            .delete_row(&covers("public"), &row)
            .unwrap();
        assert_eq!(
            delete.shown,
            r#"DELETE FROM "public"."book_covers" WHERE "id" = 9"#
        );
        // PostgreSQL runs exactly what it shows.
        assert_eq!(delete.sql.text, delete.shown);
        assert!(delete.sql.params.is_empty());

        let delete = Dialect::MySql
            .delete_row(&covers("bookshop"), &row)
            .unwrap();
        assert_eq!(
            delete.shown,
            "DELETE FROM `bookshop`.`book_covers` WHERE `id` = 9"
        );
        assert_eq!(
            delete.sql.text,
            "DELETE FROM `bookshop`.`book_covers` WHERE `id` = ?"
        );
        assert_eq!(delete.sql.params, [Value::Int(9)]);

        let delete = Dialect::Sqlite.delete_row(&covers("main"), &row).unwrap();
        assert_eq!(
            delete.shown,
            r#"DELETE FROM "main"."book_covers" WHERE "id" = 9"#
        );
        assert_eq!(
            delete.sql.text,
            r#"DELETE FROM "main"."book_covers" WHERE "id" = ?"#
        );
        assert_eq!(delete.sql.params, [Value::Int(9)]);
    }

    /// Spec 4: a key of several columns uses every one of them.
    #[test]
    fn a_delete_names_every_column_of_its_rows_key() {
        let authors = ObjectRef::new("public", "book_authors");
        let row = doomed(vec![
            ("book_id", Value::Int(101)),
            ("author", text("Sam O'Neil")),
        ]);
        let delete = Dialect::Postgres.delete_row(&authors, &row).unwrap();
        assert_eq!(
            delete.shown,
            r#"DELETE FROM "public"."book_authors" WHERE "book_id" = 101 AND "author" = 'Sam O''Neil'"#
        );
        for dialect in [Dialect::MySql, Dialect::Sqlite] {
            let delete = dialect.delete_row(&authors, &row).unwrap();
            assert_eq!(delete.sql.params, [Value::Int(101), text("Sam O'Neil")]);
            // What is shown is what is bound.
            assert_eq!(inlined(dialect, &delete.sql), delete.shown);
        }
    }

    #[test]
    fn a_delete_says_where_its_parts_stand() {
        let authors = ObjectRef::new("public", "book_authors");
        let row = doomed(vec![
            ("book_id", Value::Int(101)),
            ("author", text("Sam O'Neil")),
        ]);
        let delete = Dialect::Postgres.delete_row(&authors, &row).unwrap();
        let parts = &delete.parts;
        assert!(delete.shown[parts.filter..].starts_with("WHERE "));
        assert_eq!(
            &delete.shown[..parts.filter],
            r#"DELETE FROM "public"."book_authors" "#
        );
        assert!(delete.shown[parts.columns[0]..].starts_with(r#""book_id" = "#));
        assert!(delete.shown[parts.columns[1]..].starts_with(r#""author" = "#));
        assert_eq!(&delete.shown[parts.values[0].clone()], "101");
        assert_eq!(&delete.shown[parts.values[1].clone()], "'Sam O''Neil'");
    }

    /// A key that may not be the row's deletes no row: it could be
    /// another's.
    #[test]
    fn a_delete_is_refused_where_its_key_is_not_known_to_be_the_rows() {
        let lossy = doomed(vec![("isbn", text("978\u{FFFD}"))]);
        assert!(Dialect::Sqlite.delete_row(&books(), &lossy).is_err());
        // The other engines read a key exactly.
        assert!(Dialect::Postgres.delete_row(&books(), &lossy).is_ok());
        assert!(Dialect::MySql.delete_row(&books(), &lossy).is_ok());
        let nul = doomed(vec![("isbn", text("978\0"))]);
        assert_eq!(
            Dialect::Postgres.delete_row(&books(), &nul),
            Err(Error::query("PostgreSQL text cannot hold a NUL character"))
        );
    }
```

Add `RowDelete` to the `use` at the top of `mod tests` (or to `use super::*`'s reach, which the file-level `use crate::{..}` gives once step 3 adds it there).

- [ ] **Step 2: Run them to see them fail**

Run: `~/.cargo/bin/cargo test --locked -p tabletist-db --lib dialect::tests::a_delete`
Expected: FAIL to compile, "no method named `delete_row`".

- [ ] **Step 3: The types and the builder**

In `crates/tabletist-db/src/dialect.rs`, add `RowDelete` to the file's `use crate::{..}`. After `UpdateParts`:

```rust
/// A row's `DELETE`, as a user reads it and as the driver runs it. Both
/// come from the same key, as a [`RowUpdate`]'s do.
#[derive(Debug, Clone, PartialEq)]
pub struct RowDeletion {
    /// The statement with its key's values as literals.
    pub shown: String,
    /// What the driver sends: for PostgreSQL the shown text, for MySQL and
    /// SQLite the same statement with the values bound.
    pub sql: Sql,
    /// Where the parts of `shown` stand.
    pub parts: DeleteParts,
}

/// Where the parts of a shown `DELETE` stand, as byte offsets into its
/// text, noted as the builder writes it: as [`UpdateParts`], so a name or
/// a value that holds ` WHERE ` cannot move them.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DeleteParts {
    /// Where `WHERE` begins: the table's name ends a space before it.
    pub filter: usize,
    /// Where each key column begins: one for each of `values`.
    pub columns: Vec<usize>,
    /// Each key value's place.
    pub values: Vec<std::ops::Range<usize>>,
}
```

After `insert_row`:

```rust
    /// The `DELETE` of one row of a save, by the row's key and nothing
    /// else: no other column is compared. `Err` says why no save sends
    /// the row at all, as [`Dialect::update_row`] does: a SQLite key that
    /// may not be the row's, PostgreSQL text that holds a NUL. Refused
    /// here and nowhere after it, so what a review shows of a row is what
    /// a save does with it.
    pub fn delete_row(self, object: &ObjectRef, row: &RowDelete) -> Result<RowDeletion> {
        self.key_read_exactly(&row.key)?;
        let mut params = Vec::new();
        let lead = format!("DELETE FROM {}", self.qualified(object));
        let clause = self.key_clause(&row.key, &mut params);
        let base = lead.len();
        let parts = DeleteParts {
            // The clause begins with the space before its word.
            filter: base + 1,
            columns: clause.columns.iter().map(|column| base + column).collect(),
            values: clause
                .values
                .iter()
                .map(|value| base + value.start..base + value.end)
                .collect(),
        };
        let shown = format!("{lead}{}", clause.shown);
        let text = format!("{lead}{}", clause.sent);
        // PostgreSQL text cannot hold a NUL, and the driver cannot put one
        // in a message: see `update_row`.
        let refused = match self {
            Self::Postgres => text.contains('\0'),
            Self::MySql | Self::Sqlite => false,
        };
        if refused {
            return Err(Error::query("PostgreSQL text cannot hold a NUL character"));
        }
        Ok(RowDeletion {
            shown,
            sql: Sql { text, params },
            parts,
        })
    }
```

In `crates/tabletist-db/src/lib.rs`:

```rust
pub use dialect::{
    DeleteParts, Dialect, InsertParts, InsertStatement, RowDeletion, RowUpdate, Sql, UpdateParts,
    escape_like, quote_literal,
};
```

- [ ] **Step 4: Run the tests**

Run: `~/.cargo/bin/cargo test --locked -p tabletist-db --lib dialect::tests`
Expected: PASS. If `the_same_delete_on_each_engine` differs only in how a table is qualified on one engine, the builder is right (it writes a table as `update_row` does): fix the expected text.

- [ ] **Step 5: Run the four checks**

Expected: all pass.

- [ ] **Step 6: Commit**

```bash
git add crates/tabletist-db && git commit -m "Build a row's DELETE as it is shown and as it is sent

One statement for a row, by every column of its key and no other. What
is shown has the key's values as literals, and what MySQL and SQLite are
sent binds the same values. PostgreSQL is sent what is shown, as it is
for an UPDATE.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 3: The save's checks read a key, not a change

A row to delete is found by a key as a changed row is, and the same questions are asked of it: do two parts of the save read the same row, and can MySQL match the key exactly. Today each takes a `RowChange`. (Whether the table spells the key's columns so is asked of a key from task 4 on, where the first driver asks it.)

**Files:**
- Modify: `crates/tabletist-db/src/write.rs` (`same_row_twice`, new `read_by`, their tests)
- Modify: `crates/tabletist-db/src/sqlite/write.rs:308`, `src/pg/write.rs:186`, `src/mysql/write.rs:718` (the callers), `src/mysql/write.rs:594` (`inexact_key`) and its tests near line 931

- [ ] **Step 1: Write the failing tests**

In `crates/tabletist-db/src/write.rs`, in `mod tests`:

```rust
    #[test]
    fn a_delete_and_a_change_that_read_the_same_row_are_refused() {
        let columns = ["id".to_owned(), "email".to_owned()];
        let ada = vec![Value::Int(1), Value::Text("ada@example.com".into())];
        let changes = ChangeSet {
            // The row by its number, and the same row by its unique
            // address: `check` sees two keys.
            deletes: vec![doomed(vec![(
                "email",
                Value::Text("ada@example.com".into()),
            )])],
            ..set(vec![row(vec![("id", Value::Int(1))], vec![cell("name")])])
        };
        assert_eq!(changes.check(), Ok(()));
        let rows = [Some(ada.clone())];
        let found = [Some(ada)];
        let (keys, read) = read_by(&changes, &rows, &found);
        assert_eq!(
            same_row_twice(&keys, &columns, &read),
            Err(Error::query("two changes of the save name the same row"))
        );
        // A delete whose row is gone reads no row, and names none twice.
        let gone = [None];
        let (keys, read) = read_by(&changes, &rows, &gone);
        assert_eq!(same_row_twice(&keys, &columns, &read), Ok(()));
    }
```

- [ ] **Step 2: Run them to see them fail**

Run: `~/.cargo/bin/cargo test --locked -p tabletist-db --lib write::tests`
Expected: FAIL to compile, "cannot find function `read_by`".

- [ ] **Step 3: The helpers**

In `crates/tabletist-db/src/write.rs`, a key gets a name, before `same_key`. Without it `read_by`'s return type fails `clippy::type_complexity`:

```rust
/// A row's key: its columns and their loaded values.
pub(crate) type Key = [(String, Value)];
```

`same_row_twice` takes the keys and what each read found, whatever the part of the save that read it. Its doc's last paragraph and its body change:

```rust
/// `keys` holds the key of each row the save read, and `read` the row its
/// locking read found (`None` when it is gone), in the same order: the
/// changed rows and the rows to delete alike (see [`read_by`]). `columns`
/// names the values of every one of them: they are reads of one table in
/// one transaction. Two reads found the same row when their rows hold the
/// same values, as the database returned them, in the key columns of
/// either: each read found one row by a key that names one row, so a row
/// with that key's values is that row.
pub(crate) fn same_row_twice(
    keys: &[&Key],
    columns: &[String],
    read: &[Option<&[Value]>],
) -> Result<()> {
    // Where each key's columns are among a row's values.
    let mut places_of = Vec::with_capacity(keys.len());
    for key in keys {
        let places: Vec<usize> = key
            .iter()
            .map(|(name, _)| {
                columns
                    .iter()
                    .position(|column| column == name)
                    .ok_or_else(|| Error::query(format!("no such column: {name}")))
            })
            .collect::<Result<_>>()?;
        places_of.push(places);
    }
    for (later, theirs) in read.iter().enumerate() {
        let Some(theirs) = theirs else { continue };
        for (earlier, ours) in read.iter().enumerate().take(later) {
            let Some(ours) = ours else { continue };
            let alike = |key: Option<&Vec<usize>>| {
                key.is_some_and(|places| {
                    places.iter().all(|&place| {
                        matches!(
                            (ours.get(place), theirs.get(place)),
                            (Some(ours), Some(theirs)) if same(ours, theirs)
                        )
                    })
                })
            };
            if alike(places_of.get(earlier)) || alike(places_of.get(later)) {
                return Err(Error::query("two changes of the save name the same row"));
            }
        }
    }
    Ok(())
}

/// What [`same_row_twice`] compares, for a save whose changed rows read
/// `rows` and whose rows to delete read `doomed`: every key, and the row
/// each found.
pub(crate) fn read_by<'a>(
    changes: &'a ChangeSet,
    rows: &'a [Option<Vec<Value>>],
    doomed: &'a [Option<Vec<Value>>],
) -> (Vec<&'a Key>, Vec<Option<&'a [Value]>>) {
    let keys = changes
        .rows
        .iter()
        .map(|row| row.key.as_slice())
        .chain(changes.deletes.iter().map(|delete| delete.key.as_slice()))
        .collect();
    let read = rows.iter().chain(doomed).map(Option::as_deref).collect();
    (keys, read)
}
```

The existing test `two_changes_that_read_the_same_row_are_refused` calls the old signature through its closure `said`. Only the closure changes:

```rust
        let said = |rows: &[RowChange], read: &[Option<Vec<Value>>]| {
            let keys: Vec<&Key> = rows.iter().map(|row| row.key.as_slice()).collect();
            let read: Vec<Option<&[Value]>> = read.iter().map(Option::as_deref).collect();
            same_row_twice(&keys, &columns, &read).map_err(|error| error.to_string())
        };
```

- [ ] **Step 4: The callers**

Each driver has no rows to delete yet, and passes an empty list. In `crates/tabletist-db/src/sqlite/write.rs` (line 308), with `read_by` added to the `use crate::write::{..}`:

```rust
    let (keys, found) = read_by(changes, &rows_read, &[]);
    same_row_twice(&keys, &names, &found)?;
```

In `crates/tabletist-db/src/pg/write.rs` (line 186) and `crates/tabletist-db/src/mysql/write.rs` (line 718), the same with `&read` in place of `&rows_read`. Where the borrow of `read` by `found` outlives a later move of `read` (`conflicts_of(conflicts, read)`), put the two lines in a block of their own:

```rust
    {
        let (keys, found) = read_by(changes, &read, &[]);
        same_row_twice(&keys, &names, &found)?;
    }
```

In `crates/tabletist-db/src/mysql/write.rs`, `inexact_key` asks its question of a key:

```rust
/// A column of `key` that cannot be matched exactly, and its type, among
/// the row's `columns`.
fn inexact_key<'a>(
    key: &'a [(String, Value)],
    columns: &'a [ColumnMeta],
) -> Option<(&'a str, &'a str)> {
    key.iter().find_map(|(name, _)| {
        let column = columns.iter().find(|column| column.name == *name)?;
        Some((name.as_str(), inexact(&column.type_name)?))
    })
}
```

Its caller in `apply` passes `&change.key`, and its four tests near line 931 pass `&change(&["id"], &["name"]).key` and so on. No assertion moves.

- [ ] **Step 5: Run the tests**

Run: `~/.cargo/bin/cargo test --locked -p tabletist-db`
Expected: PASS, the SQLite suite's `changes_that_read_the_same_row_are_refused_and_nothing_is_written` among them.

- [ ] **Step 6: Run the four checks**

Expected: all pass.

- [ ] **Step 7: Commit**

```bash
git add crates/tabletist-db && git commit -m "Ask a save's questions of a key, not of a change

Whether two parts of a save read the same row, and whether MySQL can
match a key exactly, are asked of a row's key. A row to delete has a key
and no change, and will be asked the same.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 4: SQLite deletes

**Files:**
- Create: `crates/tabletist-db/tests/delete_rows.rs`
- Modify: `crates/tabletist-db/src/sqlite/write.rs`
- Modify: `crates/tabletist-db/src/write.rs` (`Applied::FailedDelete`, `key_spelled_otherwise`)

- [ ] **Step 1: Write the failing tests**

Create `crates/tabletist-db/tests/delete_rows.rs`. The scenarios are written once and run on each engine's Bookshop; this task adds SQLite's, tasks 5 and 6 the others.

```rust
//! Deleting rows in a save, on a Bookshop made in each engine: a row goes
//! by its key, a row someone deleted first is said and the rest is saved,
//! new rows go in before rows go out, and a row a foreign key holds stays,
//! with everything else the save did undone.
//!
//! "Spec N" is a section of
//! `docs/superpowers/specs/2026-10-12-deleting-rows-design.md`, "test N"
//! an entry of its Tests.
//!
//! PostgreSQL and MySQL need their servers (see compose.yaml). Without the
//! variables those tests print "skipped" and pass.

// `allow-unwrap-in-tests` does not cover helpers in an integration-test crate.
#![allow(clippy::unwrap_used)]

use std::time::Duration;

use tabletist_db::{
    Access, CellChange, ChangeSet, ConnectSpec, Connection, Error, HostKeys, InsertValue,
    NewValue, ObjectRef, RowChange, RowDelete, RowInsert, RowQuery, Secrets, StopFlag, Value,
    WriteOutcome,
};

const NORTHLIGHT: i64 = 9_100_000_000_000_000_001;
const HARBOR_PRESS: i64 = 9_100_000_000_000_000_004;
const HARBOR_LIGHTS: i64 = 9_100_000_000_000_000_007;

/// A Bookshop's tables as one engine names and types them.
struct Shop {
    schema: String,
    publishers: String,
    covers: String,
    authors: String,
    shelves: String,
    /// The type of an id column, and of a text one, as the structure
    /// names them.
    id: &'static str,
    text: &'static str,
}

impl Shop {
    fn table(&self, name: &str) -> ObjectRef {
        ObjectRef::new(&self.schema, name)
    }
}

fn by_id(id: i64) -> RowDelete {
    RowDelete {
        key: vec![("id".to_owned(), Value::Int(id))],
    }
}

fn deleting(object: ObjectRef, deletes: Vec<RowDelete>) -> ChangeSet {
    ChangeSet {
        object,
        inserts: Vec::new(),
        rows: Vec::new(),
        deletes,
    }
}

async fn save(connection: &Connection, changes: &ChangeSet) -> WriteOutcome {
    tokio::time::timeout(
        Duration::from_secs(15),
        connection.write(changes, &StopFlag::new()),
    )
    .await
    .expect("the save hung")
    .unwrap()
}

/// The table's rows, each as text, in the order of their first column.
async fn rows(connection: &Connection, object: ObjectRef) -> Vec<Vec<String>> {
    let page = connection
        .fetch_rows(&RowQuery::new(object, 500))
        .await
        .unwrap();
    let mut rows: Vec<Vec<String>> = page
        .rows
        .iter()
        .map(|row| {
            row.iter()
                .map(|value| match value {
                    Value::Int(number) => number.to_string(),
                    Value::Text(text) => text.to_string(),
                    other => format!("{other:?}"),
                })
                .collect()
        })
        .collect();
    rows.sort();
    rows
}

/// The first column of each of the table's rows.
async fn firsts(connection: &Connection, object: ObjectRef) -> Vec<String> {
    let rows = rows(connection, object).await;
    rows.into_iter().map(|mut row| row.remove(0)).collect()
}

/// Spec 4: a row goes by its key, and the others stay.
async fn a_marked_row_is_deleted(connection: &Connection, shop: &Shop) {
    let covers = shop.table(&shop.covers);
    let outcome = save(connection, &deleting(covers.clone(), vec![by_id(2)])).await;
    let WriteOutcome::Written {
        inserted,
        rows,
        gone,
        ..
    } = outcome
    else {
        panic!("the save wrote: {outcome:?}");
    };
    assert!(inserted.is_empty() && rows.is_empty() && gone.is_empty());
    assert_eq!(firsts(connection, covers).await, ["1", "3"]);
}

/// Spec 4 and test 9: cover 2 was deleted by someone else before this is
/// called. The save says so, deletes cover 1 and adds its new row.
async fn a_row_already_gone_is_said_and_the_rest_is_saved(connection: &Connection, shop: &Shop) {
    let covers = shop.table(&shop.covers);
    let changes = ChangeSet {
        inserts: vec![RowInsert {
            set: vec![InsertValue {
                column: "publisher_id".into(),
                type_name: shop.id.into(),
                new: NewValue::Text(HARBOR_PRESS.to_string()),
            }],
        }],
        ..deleting(covers.clone(), vec![by_id(1), by_id(2)])
    };
    let outcome = save(connection, &changes).await;
    let WriteOutcome::Written { inserted, gone, .. } = outcome else {
        panic!("the save wrote: {outcome:?}");
    };
    // The second of the set's deletes, not the row with id 1.
    assert_eq!(gone, [1]);
    assert_eq!(inserted.len(), 1);
    assert_eq!(firsts(connection, covers).await, ["3", "4"]);
}

/// Spec 4 and test 8: inserts go before deletes. A new publisher cannot
/// take the name of one the same save deletes, and nothing is written.
async fn a_new_row_goes_in_before_a_row_goes_out(connection: &Connection, shop: &Shop) {
    let publishers = shop.table(&shop.publishers);
    let before = rows(connection, publishers.clone()).await;
    let value = |column: &str, type_name: &str, new: String| InsertValue {
        column: column.into(),
        type_name: type_name.into(),
        new: NewValue::Text(new),
    };
    let changes = ChangeSet {
        inserts: vec![RowInsert {
            set: vec![
                value("id", shop.id, "9100000000000000009".into()),
                value("name", shop.text, "Harbor Lights".into()),
            ],
        }],
        ..deleting(publishers.clone(), vec![by_id(HARBOR_LIGHTS)])
    };
    let outcome = save(connection, &changes).await;
    assert!(
        matches!(outcome, WriteOutcome::FailedInsert { insert: 0, .. }),
        "{outcome:?}"
    );
    assert_eq!(rows(connection, publishers).await, before);
}

/// Spec 5 and test 8: Northlight Books has covers, so its delete is
/// refused, and what the save did before it is undone: the publisher it
/// renamed has its name, and the one it deleted first is there. The
/// engine's own error is handed back for its test to read.
async fn a_row_a_foreign_key_holds_stays_and_the_save_is_undone(
    connection: &Connection,
    shop: &Shop,
) -> Error {
    let publishers = shop.table(&shop.publishers);
    let before = rows(connection, publishers.clone()).await;
    let changes = ChangeSet {
        rows: vec![RowChange {
            key: vec![("id".to_owned(), Value::Int(HARBOR_PRESS))],
            set: vec![CellChange {
                column: "name".into(),
                type_name: shop.text.into(),
                loaded: Value::Text("Harbor Press".into()),
                new: NewValue::Text("Harbor House".into()),
            }],
        }],
        ..deleting(
            publishers.clone(),
            vec![by_id(HARBOR_LIGHTS), by_id(NORTHLIGHT)],
        )
    };
    let outcome = save(connection, &changes).await;
    let WriteOutcome::FailedDelete { delete, error } = outcome else {
        panic!("the save was to fail on a row to delete: {outcome:?}");
    };
    // The second of the set's deletes: the first went through, and was
    // undone with the rest.
    assert_eq!(delete, 1);
    assert_eq!(rows(connection, publishers).await, before);
    error
}

/// Spec 4: a key of two columns finds its one row.
async fn a_key_of_two_columns_finds_its_row(connection: &Connection, shop: &Shop) {
    let authors = shop.table(&shop.authors);
    let changes = deleting(
        authors.clone(),
        vec![RowDelete {
            key: vec![
                ("book_id".to_owned(), Value::Int(101)),
                ("author".to_owned(), Value::Text("Sam O'Neil".into())),
            ],
        }],
    );
    let outcome = save(connection, &changes).await;
    assert!(
        matches!(&outcome, WriteOutcome::Written { gone, .. } if gone.is_empty()),
        "{outcome:?}"
    );
    assert_eq!(
        rows(connection, authors).await,
        [["101", "Ada Quill"], ["102", "Ada Quill"]]
    );
}

/// Spec 4 and test 8: updates go before deletes. Harbor Press cannot take
/// the name of a publisher the same save deletes, and nothing is written.
async fn a_row_is_changed_before_a_row_goes_out(connection: &Connection, shop: &Shop) {
    let publishers = shop.table(&shop.publishers);
    let before = rows(connection, publishers.clone()).await;
    let changes = ChangeSet {
        rows: vec![RowChange {
            key: vec![("id".to_owned(), Value::Int(HARBOR_PRESS))],
            set: vec![CellChange {
                column: "name".into(),
                type_name: shop.text.into(),
                loaded: Value::Text("Harbor Press".into()),
                new: NewValue::Text("Harbor Lights".into()),
            }],
        }],
        ..deleting(publishers.clone(), vec![by_id(HARBOR_LIGHTS)])
    };
    let outcome = save(connection, &changes).await;
    assert!(
        matches!(outcome, WriteOutcome::Failed { row: 0, .. }),
        "{outcome:?}"
    );
    assert_eq!(rows(connection, publishers).await, before);
}

/// Shelf 2 stands on shelf 1, and goes with it (`ON DELETE CASCADE`). A
/// save that deletes both is no failure: the second was the save's own
/// doing, and is not said to have been gone before it.
async fn a_row_the_save_took_with_another_is_deleted(connection: &Connection, shop: &Shop) {
    let shelves = shop.table(&shop.shelves);
    let outcome = save(
        connection,
        &deleting(shelves.clone(), vec![by_id(1), by_id(2)]),
    )
    .await;
    assert!(
        matches!(&outcome, WriteOutcome::Written { gone, .. } if gone.is_empty()),
        "{outcome:?}"
    );
    assert_eq!(firsts(connection, shelves).await, ["3"]);
}

mod sqlite {
    use super::*;

    const CREATE: &str = "
        CREATE TABLE publishers (
            id INTEGER PRIMARY KEY,
            name TEXT NOT NULL UNIQUE
        );
        INSERT INTO publishers VALUES
            (9100000000000000001, 'Northlight Books'),
            (9100000000000000004, 'Harbor Press'),
            (9100000000000000007, 'Harbor Lights');
        CREATE TABLE book_covers (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            publisher_id INTEGER NOT NULL REFERENCES publishers (id),
            kind TEXT NOT NULL DEFAULT 'print'
        );
        INSERT INTO book_covers (publisher_id, kind) VALUES
            (9100000000000000001, 'print'),
            (9100000000000000001, 'ebook'),
            (9100000000000000004, 'print');
        CREATE TABLE book_authors (
            book_id INTEGER NOT NULL,
            author TEXT NOT NULL,
            PRIMARY KEY (book_id, author)
        );
        INSERT INTO book_authors VALUES
            (101, 'Ada Quill'), (101, 'Sam O''Neil'), (102, 'Ada Quill');
        CREATE TABLE shelves (
            id INTEGER PRIMARY KEY,
            parent_id INTEGER REFERENCES shelves (id) ON DELETE CASCADE
        );
        INSERT INTO shelves VALUES (1, NULL), (2, 1), (3, NULL)";

    fn shop() -> Shop {
        Shop {
            schema: "main".into(),
            publishers: "publishers".into(),
            covers: "book_covers".into(),
            authors: "book_authors".into(),
            shelves: "shelves".into(),
            id: "INTEGER",
            text: "TEXT",
        }
    }

    async fn bookshop() -> (Connection, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("bookshop.db");
        rusqlite::Connection::open(&path)
            .unwrap()
            .execute_batch(CREATE)
            .unwrap();
        let connection = Connection::connect_with(
            &ConnectSpec::sqlite(&path),
            &Secrets::default(),
            &HostKeys::default(),
            Access::Writable,
        )
        .await
        .unwrap();
        (connection, dir)
    }

    /// A second handle on the Bookshop's file, standing in for another
    /// program.
    fn other_program(dir: &tempfile::TempDir) -> rusqlite::Connection {
        rusqlite::Connection::open(dir.path().join("bookshop.db")).unwrap()
    }

    #[tokio::test]
    async fn sqlite_deletes_a_row_by_its_key() {
        let (connection, _dir) = bookshop().await;
        a_marked_row_is_deleted(&connection, &shop()).await;
    }

    #[tokio::test]
    async fn sqlite_says_a_row_was_already_gone_and_saves_the_rest() {
        let (connection, dir) = bookshop().await;
        other_program(&dir)
            .execute("DELETE FROM book_covers WHERE id = 2", [])
            .unwrap();
        a_row_already_gone_is_said_and_the_rest_is_saved(&connection, &shop()).await;
    }

    #[tokio::test]
    async fn sqlite_writes_new_rows_before_it_deletes() {
        let (connection, _dir) = bookshop().await;
        a_new_row_goes_in_before_a_row_goes_out(&connection, &shop()).await;
    }

    /// 787 is SQLITE_CONSTRAINT_FOREIGNKEY. SQLite names neither the key
    /// nor the table that holds the row.
    #[tokio::test]
    async fn sqlite_keeps_a_row_a_foreign_key_holds() {
        let (connection, _dir) = bookshop().await;
        let error =
            a_row_a_foreign_key_holds_stays_and_the_save_is_undone(&connection, &shop()).await;
        let Error::Query { code, message, .. } = error else {
            panic!("a query error: {error:?}");
        };
        assert_eq!(code.as_deref(), Some("787"));
        assert_eq!(message, "FOREIGN KEY constraint failed");
    }

    #[tokio::test]
    async fn sqlite_deletes_by_a_key_of_two_columns() {
        let (connection, _dir) = bookshop().await;
        a_key_of_two_columns_finds_its_row(&connection, &shop()).await;
    }

    #[tokio::test]
    async fn sqlite_changes_rows_before_it_deletes() {
        let (connection, _dir) = bookshop().await;
        a_row_is_changed_before_a_row_goes_out(&connection, &shop()).await;
    }

    #[tokio::test]
    async fn sqlite_deletes_a_row_another_delete_took_with_it() {
        let (connection, _dir) = bookshop().await;
        a_row_the_save_took_with_another_is_deleted(&connection, &shop()).await;
    }

    /// A key the table spells otherwise finds its row on SQLite all the
    /// same, and the save cannot be sure it is the column it reads as. It
    /// is an error, never a row that is already gone, and the row stays.
    #[tokio::test]
    async fn sqlite_refuses_a_key_the_table_spells_otherwise() {
        let (connection, _dir) = bookshop().await;
        let covers = shop().table("book_covers");
        let changes = deleting(
            covers.clone(),
            vec![RowDelete {
                key: vec![("ID".to_owned(), Value::Int(2))],
            }],
        );
        let outcome = connection.write(&changes, &StopFlag::new()).await;
        assert!(matches!(outcome, Err(Error::Query { .. })), "{outcome:?}");
        assert_eq!(firsts(&connection, covers).await, ["1", "2", "3"]);
    }
}
```

`tempfile`, `rusqlite`, `tokio`, `tokio-postgres` and `mysql_async` are already the crate's to use in a test (`tests/insert_audit.rs` uses all five). Add nothing to `Cargo.toml`.

- [ ] **Step 2: Run them to see them fail**

Run: `~/.cargo/bin/cargo test --locked -p tabletist-db --test delete_rows`
Expected: of the eight SQLite tests, six FAIL and two pass already. `sqlite_deletes_a_row_by_its_key` fails on its last assertion with `["1", "2", "3"]`: the save went through and deleted nothing. `sqlite_writes_new_rows_before_it_deletes` and `sqlite_changes_rows_before_it_deletes` pass before any driver deletes: the name that is taken fails the `INSERT` or the `UPDATE` whether or not the delete runs. They hold the order once the deletes are there.

- [ ] **Step 3: Build each `DELETE` before the file is asked for**

First the two things of `crates/tabletist-db/src/write.rs` that wait for a driver to use them. `Applied` gains a variant and `outcome` its arm:

```rust
pub(crate) enum Applied {
    Rows(Stored),
    Conflicts(Vec<Conflict>),
    Failed { row: usize, error: Error },
    FailedInsert { insert: usize, error: Error },
    FailedDelete { delete: usize, error: Error },
}
```

```rust
            Self::FailedDelete { delete, error } => WriteOutcome::FailedDelete { delete, error },
```

After `spelled_otherwise`, its sibling for a key alone, with a test in `mod tests`:

```rust
/// A column of `key` that is not one of the row's `columns` as the table
/// spells them: [`spelled_otherwise`] for a row that is found and not
/// changed, which has a key and no set.
pub(crate) fn key_spelled_otherwise<'a>(
    key: &'a [(String, Value)],
    columns: &[String],
) -> Option<&'a str> {
    key.iter()
        .map(|(name, _)| name.as_str())
        .find(|name| !columns.iter().any(|column| column == name))
}
```

```rust
    #[test]
    fn a_key_is_spelled_as_the_table_spells_it() {
        let columns = ["id".to_owned(), "name".to_owned()];
        let key = |name: &str| vec![(name.to_owned(), Value::Int(1))];
        assert_eq!(key_spelled_otherwise(&key("id"), &columns), None);
        assert_eq!(key_spelled_otherwise(&key("ID"), &columns), Some("ID"));
    }
```

In `crates/tabletist-db/src/sqlite/write.rs`, add `RowDeletion` to `use crate::dialect::{..}` and `key_spelled_otherwise` to `use crate::write::{..}` (`read_by` is there since task 3). In `write`, after the loop that builds `inserts`:

```rust
    let mut deletions = Vec::with_capacity(changes.deletes.len());
    for (delete, row) in changes.deletes.iter().enumerate() {
        match Dialect::Sqlite.delete_row(&changes.object, row) {
            Ok(built) => deletions.push(built),
            Err(error) => return Ok(WriteOutcome::FailedDelete { delete, error }),
        }
    }
```

and pass them on: `apply(connection, changes, &inserts, &updates, &deletions, stop)`. `apply` gains the parameter `deletions: &[RowDeletion]` after `updates`.

- [ ] **Step 4: Read each row to delete with the rows to change**

`ambiguous` asks its question of a `RowChange`. Give the question a name of its own, so a key can be asked it:

```rust
/// Whether more than one of the row's `columns` reads as `name`. SQLite
/// matches a name without regard to ASCII case, so that is how they are
/// counted.
fn reads_as_several(name: &str, columns: &[String]) -> bool {
    columns
        .iter()
        .filter(|column| column.eq_ignore_ascii_case(name))
        .count()
        > 1
}
```

`ambiguous`'s `.find(..)` becomes `.find(|name| reads_as_several(name, columns))`, and its doc keeps its words.

In `apply`, after the loop over `changes.rows` and before `same_row_twice`:

```rust
    // The rows to delete are found now too, while nothing is written: a
    // key that names two rows, or a column the table spells otherwise,
    // ends the save before it changes anything. `None` is a row that is
    // already gone.
    let mut doomed = Vec::with_capacity(changes.deletes.len());
    for delete in &changes.deletes {
        let select = dialect.select_row(&changes.object, &delete.key, true);
        not_stopped(stop)?;
        let (columns, mut found) = read(connection, &select)?;
        if let Some((name, _)) = delete
            .key
            .iter()
            .find(|(name, _)| reads_as_several(name, &columns))
        {
            return Err(Error::query(format!(
                "{name} names more than one column of the table, so the save cannot tell which \
                 it means"
            )));
        }
        if let Some(name) = key_spelled_otherwise(&delete.key, &columns) {
            return Err(Error::query(format!(
                "the table spells {name} another way, so the save cannot be sure which column \
                 it names"
            )));
        }
        if found.len() > 1 {
            return Err(more_than_one());
        }
        doomed.push(found.pop().map(|server| server.values));
        names = columns;
    }
```

and the comparison reads them:

```rust
    let (keys, found) = read_by(changes, &rows_read, &doomed);
    same_row_twice(&keys, &names, &found)?;
```

(in a block of its own if `rows_read` is moved after it, as task 3 says).

- [ ] **Step 5: Delete after the updates, before the rows are read back**

In `apply`, after the loop over `updates` and before the read-backs:

```rust
    // The rows to delete, last: the order a review shows them in. One
    // that was not there is said and not sent.
    let mut gone = Vec::new();
    let found = changes.deletes.iter().zip(deletions).zip(&doomed);
    for (delete, ((row, deletion), found)) in found.enumerate() {
        if found.is_none() {
            gone.push(delete);
            continue;
        }
        not_stopped(stop)?;
        let params = rusqlite::params_from_iter(deletion.sql.params.iter().map(exact));
        let touched = match connection
            .execute(&deletion.sql.text, params)
            .map_err(map_error)
        {
            Ok(touched) => touched,
            // A cancel or a lost session ends the save; anything else is
            // the statement's own failure: a foreign key that holds the
            // row, most often.
            Err(error @ (Error::Cancelled | Error::ConnectionLost(_))) => return Err(error),
            Err(error) => return Ok(Applied::FailedDelete { delete, error }),
        };
        // The count is of the table's own rows, without those a foreign
        // key took with them. None: a row deleted before this one took it
        // along (a key of the table that points at the table), which is
        // what the save meant, or a trigger kept it, which is not. The
        // row says which.
        if touched == 0 {
            let select = dialect.select_row(&changes.object, &row.key, false);
            not_stopped(stop)?;
            let (_, still) = read(connection, &select)?;
            if still.is_empty() {
                continue;
            }
        }
        if touched != 1 {
            return Ok(Applied::FailedDelete {
                delete,
                error: Error::query(format!(
                    "the save would have deleted {touched} rows where it meant one"
                )),
            });
        }
    }
```

The comment above the read-backs gains a sentence, since a delete can now take a changed row with it:

```rust
        // One row still: a trigger the update fired can have made another
        // that the key also finds, and then which one was saved is not
        // known. None: a row the save deleted took this one with it (a
        // key of the table that points at the table), and the save is
        // undone.
```

- [ ] **Step 6: A new row is not known where a delete can reach it**

After `follows_updates`:

```rust
/// Whether a row of the table follows a row that is deleted: whether one
/// of the table's foreign keys acts when what it refers to is deleted
/// (`CASCADE`, `SET NULL`, `SET DEFAULT`). The table can refer to itself,
/// or to a table that refers back to it.
fn follows_deletes(connection: &rusqlite::Connection, object: &ObjectRef) -> Result<bool> {
    connection
        .query_row(
            "SELECT EXISTS (SELECT 1 FROM pragma_foreign_key_list(?1, ?2) \
                            WHERE upper(on_delete) NOT IN ('NO ACTION', 'RESTRICT'))",
            [&object.name, &object.schema],
            |row| row.get::<_, bool>(0),
        )
        .map_err(map_error)
}
```

and `known` asks it. Its comment gains a paragraph:

```rust
    // A row the save deleted can do the same to a new one, where a key of
    // the table acts on a delete: the new row can name the deleted one,
    // straight or through another table.
    let known = returned.is_empty()
        || !(triggered(connection, &changes.object)?
            || (!updates.is_empty() && follows_updates(connection, &changes.object)?)
            || (!deletions.is_empty() && follows_deletes(connection, &changes.object)?));
```

The function ends with the list:

```rust
    Ok(Applied::Rows(Stored {
        inserted,
        rows,
        gone,
    }))
```

`apply`'s doc reads "read and compare every row, update every row, read every row back". It now reads: "read and compare every row, write the new rows, update every row, delete the rows to delete, read every changed row back".

- [ ] **Step 7: Run the tests**

Run: `~/.cargo/bin/cargo test --locked -p tabletist-db --test delete_rows`
Expected: the eight SQLite tests PASS.

Run: `~/.cargo/bin/cargo test --locked -p tabletist-db --test sqlite`
Expected: PASS, as before.

- [ ] **Step 8: Run the four checks**

Expected: all pass.

- [ ] **Step 9: Commit**

```bash
git add crates/tabletist-db && git commit -m "Delete rows in a SQLite save

A save's rows to delete are found by their keys with its changed rows,
before anything is written, and deleted after its updates. A row that
was not there is said and the rest is saved. A row a foreign key holds
fails the save, and what it wrote is undone.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 5: PostgreSQL deletes

**Files:**
- Modify: `crates/tabletist-db/src/pg/write.rs`
- Modify: `crates/tabletist-db/tests/delete_rows.rs` (a `mod postgres`)

- [ ] **Step 1: Write the tests**

In `crates/tabletist-db/tests/delete_rows.rs`, add `TlsMode` to the `use tabletist_db::{..}` and, after `mod sqlite`:

```rust
mod postgres {
    use super::*;

    /// Written for the tables that begin with `shop`: each test has a
    /// Bookshop of its own, whose tables begin with its own word.
    const DROP: &str =
        "DROP TABLE IF EXISTS shop_shelves, shop_book_authors, shop_book_covers, shop_publishers";

    const CREATE: &str = "
        CREATE TABLE shop_publishers (
            id bigint PRIMARY KEY,
            name text NOT NULL UNIQUE
        );
        INSERT INTO shop_publishers VALUES
            (9100000000000000001, 'Northlight Books'),
            (9100000000000000004, 'Harbor Press'),
            (9100000000000000007, 'Harbor Lights');
        CREATE TABLE shop_book_covers (
            id bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
            publisher_id bigint NOT NULL REFERENCES shop_publishers (id),
            kind varchar NOT NULL DEFAULT 'print'
        );
        INSERT INTO shop_book_covers (publisher_id, kind) VALUES
            (9100000000000000001, 'print'),
            (9100000000000000001, 'ebook'),
            (9100000000000000004, 'print');
        CREATE TABLE shop_book_authors (
            book_id bigint NOT NULL,
            author text NOT NULL,
            PRIMARY KEY (book_id, author)
        );
        INSERT INTO shop_book_authors VALUES
            (101, 'Ada Quill'), (101, 'Sam O''Neil'), (102, 'Ada Quill');
        CREATE TABLE shop_shelves (
            id bigint PRIMARY KEY,
            parent_id bigint REFERENCES shop_shelves (id) ON DELETE CASCADE
        );
        INSERT INTO shop_shelves VALUES (1, NULL), (2, 1), (3, NULL)";

    fn named(sql: &str, prefix: &str) -> String {
        sql.replace("shop_", &format!("{prefix}_"))
    }

    fn shop(prefix: &str) -> Shop {
        Shop {
            schema: "public".into(),
            publishers: format!("{prefix}_publishers"),
            covers: format!("{prefix}_book_covers"),
            authors: format!("{prefix}_book_authors"),
            shelves: format!("{prefix}_shelves"),
            id: "bigint",
            text: "text",
        }
    }

    fn url() -> Option<String> {
        let url = std::env::var("TABLETIST_TEST_PG_URL").ok();
        url.filter(|url| !url.trim().is_empty())
    }

    /// The Bookshop whose tables begin with `prefix`, made anew, with a
    /// session of the test's own standing in for another program. `None`
    /// without a server.
    async fn bookshop(prefix: &str) -> Option<(tokio_postgres::Client, Connection)> {
        let Some(url) = url() else {
            eprintln!("skipped: TABLETIST_TEST_PG_URL is not set");
            return None;
        };
        let mut config: tokio_postgres::Config = url.parse().unwrap();
        config.ssl_mode(tokio_postgres::config::SslMode::Disable);
        let (admin, session) = config.connect(tokio_postgres::NoTls).await.unwrap();
        tokio::spawn(session);
        admin.batch_execute(&named(DROP, prefix)).await.unwrap();
        admin.batch_execute(&named(CREATE, prefix)).await.unwrap();
        let (mut spec, secrets) = ConnectSpec::from_url(&url).unwrap();
        spec.tls = TlsMode::Disable;
        let connection =
            Connection::connect_with(&spec, &secrets, &HostKeys::default(), Access::Writable)
                .await
                .unwrap();
        Some((admin, connection))
    }

    #[tokio::test]
    async fn postgres_deletes_a_row_by_its_key() {
        let Some((admin, connection)) = bookshop("pgdel").await else {
            return;
        };
        a_marked_row_is_deleted(&connection, &shop("pgdel")).await;
        admin.batch_execute(&named(DROP, "pgdel")).await.unwrap();
    }

    #[tokio::test]
    async fn postgres_says_a_row_was_already_gone_and_saves_the_rest() {
        let Some((admin, connection)) = bookshop("pggone").await else {
            return;
        };
        admin
            .batch_execute("DELETE FROM pggone_book_covers WHERE id = 2")
            .await
            .unwrap();
        a_row_already_gone_is_said_and_the_rest_is_saved(&connection, &shop("pggone")).await;
        admin.batch_execute(&named(DROP, "pggone")).await.unwrap();
    }

    #[tokio::test]
    async fn postgres_writes_new_rows_before_it_deletes() {
        let Some((admin, connection)) = bookshop("pgorder").await else {
            return;
        };
        a_new_row_goes_in_before_a_row_goes_out(&connection, &shop("pgorder")).await;
        admin.batch_execute(&named(DROP, "pgorder")).await.unwrap();
    }

    /// Spec 5: 23503, and the constraint that holds the row, which is
    /// what run 3 finds the referencing table by.
    #[tokio::test]
    async fn postgres_keeps_a_row_a_foreign_key_holds() {
        let Some((admin, connection)) = bookshop("pgheld").await else {
            return;
        };
        let error =
            a_row_a_foreign_key_holds_stays_and_the_save_is_undone(&connection, &shop("pgheld"))
                .await;
        let Error::Query {
            code, named: of, ..
        } = error
        else {
            panic!("a query error: {error:?}");
        };
        assert_eq!(code.as_deref(), Some("23503"));
        assert_eq!(
            of.constraint.as_deref(),
            Some("pgheld_book_covers_publisher_id_fkey")
        );
        admin.batch_execute(&named(DROP, "pgheld")).await.unwrap();
    }

    #[tokio::test]
    async fn postgres_deletes_by_a_key_of_two_columns() {
        let Some((admin, connection)) = bookshop("pgtwo").await else {
            return;
        };
        a_key_of_two_columns_finds_its_row(&connection, &shop("pgtwo")).await;
        admin.batch_execute(&named(DROP, "pgtwo")).await.unwrap();
    }

    #[tokio::test]
    async fn postgres_changes_rows_before_it_deletes() {
        let Some((admin, connection)) = bookshop("pgfirst").await else {
            return;
        };
        a_row_is_changed_before_a_row_goes_out(&connection, &shop("pgfirst")).await;
        admin.batch_execute(&named(DROP, "pgfirst")).await.unwrap();
    }

    #[tokio::test]
    async fn postgres_deletes_a_row_another_delete_took_with_it() {
        let Some((admin, connection)) = bookshop("pgtree").await else {
            return;
        };
        a_row_the_save_took_with_another_is_deleted(&connection, &shop("pgtree")).await;
        admin.batch_execute(&named(DROP, "pgtree")).await.unwrap();
    }
}
```

The error's `named` is bound as `of`, so it does not hide the module's function `named`.

- [ ] **Step 2: Run them**

Run: `~/.cargo/bin/cargo test --locked -p tabletist-db --test delete_rows postgres`
Expected, with `TABLETIST_TEST_PG_URL` set: `postgres_deletes_a_row_by_its_key` FAILS with `["1", "2", "3"]`, and the two tests of the order pass already, as on SQLite. Without it: seven "skipped" lines, and the tests pass. Note which it was.

- [ ] **Step 3: Build, read, delete**

In `crates/tabletist-db/src/pg/write.rs`, add `RowDeletion` to `use crate::dialect::{..}`. In `save`, after the loop that builds `inserts`:

```rust
        let mut deletions = Vec::with_capacity(changes.deletes.len());
        for (delete, row) in changes.deletes.iter().enumerate() {
            match Dialect::Postgres.delete_row(&changes.object, row) {
                Ok(built) => deletions.push(built),
                Err(error) => return Ok(WriteOutcome::FailedDelete { delete, error }),
            }
        }
```

and pass them on: `apply(&client, changes, &inserts, &updates, &deletions, stop).await`. `apply` gains `deletions: &[RowDeletion]` after `updates`.

In `apply`, after the loop over `changes.rows`:

```rust
    // The rows to delete are locked now too, while nothing is written: a
    // delete someone has not committed yet is waited for, and then the
    // row is found gone. `None` is a row that is already gone.
    let mut doomed = Vec::with_capacity(changes.deletes.len());
    for delete in &changes.deletes {
        let select = dialect.select_row(&changes.object, &delete.key, true);
        not_stopped(stop)?;
        let mut found = rows(client, &select.text, &columns).await?;
        if found.len() > 1 {
            return Err(more_than_one());
        }
        doomed.push(found.pop());
    }
    // Before a conflict is answered too: a set that names a row twice is
    // not one to offer writing over what the server holds.
    {
        let (keys, found) = read_by(changes, &read, &doomed);
        same_row_twice(&keys, &names, &found)?;
    }
```

(the two-line comment is the one already above `same_row_twice`; keep one copy of it).

After the loop over `updates`, before the read-backs:

```rust
    // The rows to delete, last: the order a review shows them in. One
    // that was not there is said and not sent.
    let mut gone = Vec::new();
    let found = changes.deletes.iter().zip(deletions).zip(&doomed);
    for (delete, ((row, deletion), found)) in found.enumerate() {
        if found.is_none() {
            gone.push(delete);
            continue;
        }
        not_stopped(stop)?;
        let messages = match client
            .simple_query(&deletion.sql.text)
            .await
            .map_err(query_error)
        {
            Ok(messages) => messages,
            // A cancel or a lost session ends the save; anything else is
            // the statement's own failure: a foreign key that holds the
            // row, most often.
            Err(error @ (Error::Cancelled | Error::ConnectionLost(_))) => return Err(error),
            Err(error) => return Ok(Applied::FailedDelete { delete, error }),
        };
        // The count is of the table's own rows, without those a foreign
        // key took with them.
        let touched = messages
            .iter()
            .find_map(|message| match message {
                SimpleQueryMessage::CommandComplete(count) => Some(*count),
                _ => None,
            })
            .unwrap_or(0);
        // None: a row deleted before this one took it along, or a trigger
        // or a rule kept it. The row says which, as on SQLite.
        if touched == 0 {
            let select = dialect.select_row(&changes.object, &row.key, false);
            not_stopped(stop)?;
            if rows(client, &select.text, &columns).await?.is_empty() {
                continue;
            }
        }
        if touched != 1 {
            return Ok(Applied::FailedDelete {
                delete,
                error: Error::query(format!(
                    "the save would have deleted {touched} rows where it meant one"
                )),
            });
        }
    }
```

The comment above the read-backs gains the same sentence as SQLite's: "None: a row the save deleted took this one with it (a key of the table that points at the table), and the save is undone."

- [ ] **Step 4: A new row is not known where a delete can reach it**

The query that answers `known` gains a fourth parameter. Its comment gains a paragraph and the statement a line:

```rust
                // A row the save deleted does the same where a key of the
                // table acts on a delete: a new row can name the deleted
                // one, straight or through another table. Only a save
                // that deletes rows has such a delete.
                "SELECT c.relkind = 'r' AND NOT c.relhasrules \
                        AND NOT EXISTS (SELECT 1 FROM pg_trigger t \
                                        WHERE t.tgrelid = c.oid AND NOT t.tgisinternal) \
                        AND ($3 OR NOT EXISTS (SELECT 1 FROM pg_constraint k \
                                               WHERE k.conrelid = c.oid AND k.contype = 'f' \
                                                 AND k.confupdtype NOT IN ('a', 'r'))) \
                        AND ($4 OR NOT EXISTS (SELECT 1 FROM pg_constraint k \
                                               WHERE k.conrelid = c.oid AND k.contype = 'f' \
                                                 AND k.confdeltype NOT IN ('a', 'r'))) \
                 FROM pg_class c \
                 JOIN pg_namespace n ON n.oid = c.relnamespace \
                 WHERE n.nspname = $1 AND c.relname = $2",
                &[
                    &changes.object.schema,
                    &changes.object.name,
                    &changes.rows.is_empty(),
                    &changes.deletes.is_empty(),
                ],
```

The function ends with the list:

```rust
    Ok(Applied::Rows(Stored {
        inserted,
        rows: saved,
        gone,
    }))
```

`apply`'s doc, as SQLite's: "lock and compare every row, write the new rows, update every row, delete the rows to delete, read every changed row back".

- [ ] **Step 5: Run the tests**

Run: `~/.cargo/bin/cargo test --locked -p tabletist-db --test delete_rows postgres`
Expected with a server: seven PASS. Without one: seven "skipped". Without one, say in the task's report that PostgreSQL's deletes were compiled and not run.

- [ ] **Step 6: Run the four checks**

Expected: all pass.

- [ ] **Step 7: Commit**

```bash
git add crates/tabletist-db && git commit -m "Delete rows in a PostgreSQL save

As on SQLite: the rows to delete are locked and found with the changed
rows, deleted after the updates, and a row that was not there is said
while the rest is saved. A foreign key that holds a row fails the save
with the constraint's name.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 6: MySQL deletes, and the constraint a refused delete names

**Files:**
- Modify: `crates/tabletist-db/src/mysql.rs` (`named`, and its test near line 1100)
- Modify: `crates/tabletist-db/src/mysql/write.rs`
- Modify: `crates/tabletist-db/tests/delete_rows.rs` (a `mod mysql`)

- [ ] **Step 1: Write the failing unit test**

In `crates/tabletist-db/src/mysql.rs`, in the test that calls `of(1452, ..)`, after that assertion:

```rust
        // ER_ROW_IS_REFERENCED_2, a delete a foreign key refuses: the
        // constraint, and no column, since the one it names is the
        // referencing table's and not the table the save wrote to.
        assert_eq!(
            of(
                1451,
                "Cannot delete or update a parent row: a foreign key constraint fails \
                 (`tabletist`.`shop_book_covers`, CONSTRAINT `shop_book_covers_ibfk_1` \
                 FOREIGN KEY (`publisher_id`) REFERENCES `shop_publishers` (`id`))"
            ),
            (Some(1451), Some("shop_book_covers_ibfk_1".into()), vec![])
        );
```

- [ ] **Step 2: Run it to see it fail**

Run: `~/.cargo/bin/cargo test --locked -p tabletist-db --lib mysql::tests`
Expected: FAIL on the new assertion: `(Some(1451), None, [])`.

- [ ] **Step 3: Read the constraint of error 1451**

In `named`, before the arm for `1452`:

```rust
        // ER_ROW_IS_REFERENCED_2: a row another table's foreign key holds.
        // The constraint as it was defined, as 1452's. Its columns are the
        // other table's, and a caller looks for a named column in the
        // table it wrote to, so they are left out.
        1451 => {
            let constraint = message
                .split_once("CONSTRAINT ")
                .and_then(|(_, rest)| quoted(rest, '`'));
            (constraint, Vec::new())
        }
```

Run: `~/.cargo/bin/cargo test --locked -p tabletist-db --lib mysql::tests`
Expected: PASS.

- [ ] **Step 4: Write the integration tests**

In `crates/tabletist-db/tests/delete_rows.rs`, add `use mysql_async::prelude::Queryable;` at the top of the file and, after `mod postgres`:

```rust
mod mysql {
    use super::*;

    const DROP: &str =
        "DROP TABLE IF EXISTS shop_shelves, shop_book_authors, shop_book_covers, shop_publishers";

    const CREATE: [&str; 8] = [
        "CREATE TABLE shop_publishers (
             id BIGINT PRIMARY KEY,
             name VARCHAR(64) NOT NULL UNIQUE
         )",
        "INSERT INTO shop_publishers VALUES
             (9100000000000000001, 'Northlight Books'),
             (9100000000000000004, 'Harbor Press'),
             (9100000000000000007, 'Harbor Lights')",
        "CREATE TABLE shop_book_covers (
             id BIGINT AUTO_INCREMENT PRIMARY KEY,
             publisher_id BIGINT NOT NULL,
             kind VARCHAR(64) NOT NULL DEFAULT 'print',
             FOREIGN KEY (publisher_id) REFERENCES shop_publishers (id)
         )",
        "INSERT INTO shop_book_covers (publisher_id, kind) VALUES
             (9100000000000000001, 'print'),
             (9100000000000000001, 'ebook'),
             (9100000000000000004, 'print')",
        "CREATE TABLE shop_book_authors (
             book_id BIGINT NOT NULL,
             author VARCHAR(64) NOT NULL,
             PRIMARY KEY (book_id, author)
         )",
        "INSERT INTO shop_book_authors VALUES
             (101, 'Ada Quill'), (101, 'Sam O''Neil'), (102, 'Ada Quill')",
        "CREATE TABLE shop_shelves (
             id BIGINT PRIMARY KEY,
             parent_id BIGINT,
             FOREIGN KEY (parent_id) REFERENCES shop_shelves (id) ON DELETE CASCADE
         )",
        "INSERT INTO shop_shelves VALUES (1, NULL), (2, 1), (3, NULL)",
    ];

    fn named(sql: &str, prefix: &str) -> String {
        sql.replace("shop_", &format!("{prefix}_"))
    }

    fn shop(prefix: &str) -> Shop {
        Shop {
            schema: "tabletist".into(),
            publishers: format!("{prefix}_publishers"),
            covers: format!("{prefix}_book_covers"),
            authors: format!("{prefix}_book_authors"),
            shelves: format!("{prefix}_shelves"),
            id: "bigint",
            text: "varchar(64)",
        }
    }

    fn url() -> Option<String> {
        let url = std::env::var("TABLETIST_TEST_MYSQL_URL").ok();
        url.filter(|url| !url.trim().is_empty())
    }

    /// The Bookshop whose tables begin with `prefix`, made anew, with a
    /// session of the test's own standing in for another program. `None`
    /// without a server.
    async fn bookshop(prefix: &str) -> Option<(mysql_async::Conn, Connection)> {
        let Some(url) = url() else {
            eprintln!("skipped: TABLETIST_TEST_MYSQL_URL is not set");
            return None;
        };
        let opts = mysql_async::Opts::from_url(&format!("{url}?prefer_socket=false")).unwrap();
        let mut admin = mysql_async::Conn::new(opts).await.unwrap();
        admin.query_drop(named(DROP, prefix)).await.unwrap();
        for statement in CREATE {
            admin.query_drop(named(statement, prefix)).await.unwrap();
        }
        let (mut spec, secrets) = ConnectSpec::from_url(&url).unwrap();
        spec.tls = TlsMode::Disable;
        let connection =
            Connection::connect_with(&spec, &secrets, &HostKeys::default(), Access::Writable)
                .await
                .unwrap();
        Some((admin, connection))
    }

    #[tokio::test]
    async fn mysql_deletes_a_row_by_its_key() {
        let Some((mut admin, connection)) = bookshop("mydel").await else {
            return;
        };
        a_marked_row_is_deleted(&connection, &shop("mydel")).await;
        admin.query_drop(named(DROP, "mydel")).await.unwrap();
    }

    #[tokio::test]
    async fn mysql_says_a_row_was_already_gone_and_saves_the_rest() {
        let Some((mut admin, connection)) = bookshop("mygone").await else {
            return;
        };
        admin
            .query_drop("DELETE FROM mygone_book_covers WHERE id = 2")
            .await
            .unwrap();
        a_row_already_gone_is_said_and_the_rest_is_saved(&connection, &shop("mygone")).await;
        admin.query_drop(named(DROP, "mygone")).await.unwrap();
    }

    #[tokio::test]
    async fn mysql_writes_new_rows_before_it_deletes() {
        let Some((mut admin, connection)) = bookshop("myorder").await else {
            return;
        };
        a_new_row_goes_in_before_a_row_goes_out(&connection, &shop("myorder")).await;
        admin.query_drop(named(DROP, "myorder")).await.unwrap();
    }

    /// Spec 5: MySQL's 1451, under the SQLSTATE 23000 it shares with other
    /// failures, and the constraint that holds the row.
    #[tokio::test]
    async fn mysql_keeps_a_row_a_foreign_key_holds() {
        let Some((mut admin, connection)) = bookshop("myheld").await else {
            return;
        };
        let error =
            a_row_a_foreign_key_holds_stays_and_the_save_is_undone(&connection, &shop("myheld"))
                .await;
        let Error::Query {
            code, named: of, ..
        } = error
        else {
            panic!("a query error: {error:?}");
        };
        assert_eq!(code.as_deref(), Some("23000"));
        assert_eq!(of.number, Some(1451));
        assert_eq!(of.constraint.as_deref(), Some("myheld_book_covers_ibfk_1"));
        assert!(of.columns.is_empty());
        admin.query_drop(named(DROP, "myheld")).await.unwrap();
    }

    #[tokio::test]
    async fn mysql_deletes_by_a_key_of_two_columns() {
        let Some((mut admin, connection)) = bookshop("mytwo").await else {
            return;
        };
        a_key_of_two_columns_finds_its_row(&connection, &shop("mytwo")).await;
        admin.query_drop(named(DROP, "mytwo")).await.unwrap();
    }

    #[tokio::test]
    async fn mysql_changes_rows_before_it_deletes() {
        let Some((mut admin, connection)) = bookshop("myfirst").await else {
            return;
        };
        a_row_is_changed_before_a_row_goes_out(&connection, &shop("myfirst")).await;
        admin.query_drop(named(DROP, "myfirst")).await.unwrap();
    }

    #[tokio::test]
    async fn mysql_deletes_a_row_another_delete_took_with_it() {
        let Some((mut admin, connection)) = bookshop("mytree").await else {
            return;
        };
        a_row_the_save_took_with_another_is_deleted(&connection, &shop("mytree")).await;
        admin.query_drop(named(DROP, "mytree")).await.unwrap();
    }
}
```

Run: `~/.cargo/bin/cargo test --locked -p tabletist-db --test delete_rows mysql`
Expected, with `TABLETIST_TEST_MYSQL_URL` set: `mysql_deletes_a_row_by_its_key` FAILS with `["1", "2", "3"]`. Without it: seven "skipped" lines.

- [ ] **Step 5: Build each row's statements**

In `crates/tabletist-db/src/mysql/write.rs`, add `RowDelete` to the `use crate::{..}`, and `key_spelled_otherwise` to `use crate::write::{..}`. After `Statements`:

```rust
/// The statements of one row to delete, built before anything is sent.
struct Deletion {
    /// The row by its key, locked.
    lock: Sql,
    delete: Sql,
}
```

After `build_insert`:

```rust
/// The statements of one row to delete, as the driver would send them as
/// they stand (see [`build`]).
fn build_delete(object: &ObjectRef, row: &RowDelete) -> Result<Deletion> {
    let dialect = Dialect::MySql;
    let deletion = Deletion {
        lock: dialect.select_row(object, &row.key, true),
        delete: dialect.delete_row(object, row)?.sql,
    };
    for sql in [&deletion.lock, &deletion.delete] {
        if driver_parameter(&sql.text).is_err() {
            return Err(Error::query(
                "the MySQL driver would read part of a name as a parameter, so the save cannot \
                 be sent",
            ));
        }
    }
    Ok(deletion)
}
```

In `save`, after the loop that builds `inserts`:

```rust
        let mut deletes = Vec::with_capacity(changes.deletes.len());
        for (delete, row) in changes.deletes.iter().enumerate() {
            match build_delete(&changes.object, row) {
                Ok(built) => deletes.push(built),
                Err(error) => return Ok(WriteOutcome::FailedDelete { delete, error }),
            }
        }
```

A save that deletes reads its rows under a lock, and so holds its table without `hold`. The comment and the test change:

```rust
        // A save that changes and deletes no row reads none, and takes its
        // table with a read of its own (see `hold`).
        let hold = if changes.rows.is_empty() && changes.deletes.is_empty() {
```

`Written` gains the list, and `save` passes it:

```rust
    /// Of each row to delete.
    deletes: &'a [Deletion],
```

```rust
        let written = Written {
            statements: &statements,
            inserts: &inserts,
            deletes: &deletes,
            hold: hold.as_ref(),
        };
```

`hold`'s own doc says "A save that changes rows holds its table from its first locking read on; one of new rows alone reads nothing". It now reads "A save that changes or deletes rows holds its table from its first locking read on; one of new rows alone reads nothing".

- [ ] **Step 6: Read, then delete**

After `update`:

```rust
/// Runs one row's `DELETE` and says how many rows it deleted: the table's
/// own, without those a foreign key took with them. The inner `Err` is
/// the statement's own failure, a foreign key that holds the row most
/// often. The outer one is a cancel, a stop or a lost session. A warning
/// fails no delete: an update fails on one because what was stored may
/// not be what was typed, and a delete stores nothing.
async fn delete(
    conn: &mut mysql_async::Conn,
    sql: &Sql,
    stop: &StopFlag,
) -> Result<std::result::Result<u64, Error>> {
    let failed = |error: Error| own(error).map(|error| Err(error.unwrap_or(Error::Cancelled)));
    let statement = match prepare(conn, sql, stop).await {
        Ok(statement) => statement,
        Err(error) => return failed(error),
    };
    not_stopped(stop)?;
    if let Err(error) = conn.exec_drop(&statement, params(&sql.params)).await {
        return failed(query_error(error));
    }
    Ok(Ok(conn.affected_rows()))
}
```

In `apply`, destructure `deletes` from `written` with the others. After the loop over `changes.rows`:

```rust
    // The rows to delete are locked now too, while nothing is written.
    // `None` is a row that is already gone. A key that cannot be matched
    // exactly is an error here, never a row that is gone: it finds no row
    // whether the row is there or not.
    let mut doomed = Vec::with_capacity(deletes.len());
    for (delete, deletion) in changes.deletes.iter().zip(deletes) {
        let (columns, mut found) = rows(conn, &deletion.lock, stop).await?;
        names = columns.iter().map(|column| column.name.clone()).collect();
        if let Some(name) = key_spelled_otherwise(&delete.key, &names) {
            return Err(Error::query(format!(
                "the table spells {name} another way, so the save cannot be sure which column \
                 it names"
            )));
        }
        if let Some((name, type_name)) = inexact_key(&delete.key, &columns) {
            return Err(Error::query(format!(
                "{name} is a {type_name} column, and a key of that type cannot be matched \
                 exactly, so the save cannot be sure which row it names"
            )));
        }
        if found.len() > 1 {
            return Err(more_than_one());
        }
        doomed.push(found.pop());
    }
    // Before a conflict is answered too: a set that names a row twice is
    // not one to offer writing over what the server holds.
    {
        let (keys, found) = read_by(changes, &read, &doomed);
        same_row_twice(&keys, &names, &found)?;
    }
```

After the loop over `statements` that runs the updates, before the read-backs:

```rust
    // The rows to delete, last: the order a review shows them in. One
    // that was not there is said and not sent.
    let mut gone = Vec::new();
    for (place, (deletion, found)) in deletes.iter().zip(&doomed).enumerate() {
        if found.is_none() {
            gone.push(place);
            continue;
        }
        let deleted = match delete(conn, &deletion.delete, stop).await? {
            Ok(deleted) => deleted,
            Err(error) => {
                return Ok(Applied::FailedDelete {
                    delete: place,
                    error,
                });
            }
        };
        // None: a row deleted before this one took it along, or a trigger
        // kept it. The row says which, as on SQLite: its locking read
        // once more.
        if deleted == 0 && rows(conn, &deletion.lock, stop).await?.1.is_empty() {
            continue;
        }
        if deleted != 1 {
            return Ok(Applied::FailedDelete {
                delete: place,
                error: Error::query(format!(
                    "the save would have deleted {deleted} rows where it meant one"
                )),
            });
        }
    }
```

`own` hands back `Some` for every error it does not return as `Err`, so `failed`'s `unwrap_or` never runs: it is there so the closure has no `unwrap`.

The comment above the read-backs gains the sentence: "None: a row the save deleted took this one with it, and the save is undone." The new rows are found again after every statement already ("The new rows last, as they stand once every statement has run"), so a new row a delete took with it is handed back as not known with no further change.

The function ends with the list:

```rust
    Ok(Applied::Rows(Stored {
        inserted,
        rows: saved,
        gone,
    }))
```

`apply`'s doc, as the others': "lock and compare every row, write the new rows, update every row, delete the rows to delete, read every changed row back".

- [ ] **Step 7: Run the tests**

Run: `~/.cargo/bin/cargo test --locked -p tabletist-db --test delete_rows mysql`
Expected with a server: seven PASS. Without one: seven "skipped". Without one, say in the task's report that MySQL's deletes were compiled and not run.

Run: `~/.cargo/bin/cargo test --locked -p tabletist-db --lib mysql`
Expected: PASS.

- [ ] **Step 8: Run the four checks**

Expected: all pass.

- [ ] **Step 9: Commit**

```bash
git add crates/tabletist-db && git commit -m "Delete rows in a MySQL save, and name the key that holds one

As on the other engines: the rows to delete are locked and found with
the changed rows, deleted after the updates, and a row that was not
there is said while the rest is saved. A delete a foreign key refuses
(error 1451) names its constraint.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 7: Say what a save now does

**Files:**
- Modify: `crates/tabletist-db/src/lib.rs` (the doc of `Connection::write`, near line 304)
- Modify: `crates/tabletist-db/src/write.rs` (the file's first line)

- [ ] **Step 1: The docs**

`Connection::write`'s second paragraph reads "The set's new rows are written first, each by one `INSERT`, then its changed rows." Replace that sentence, and add a paragraph after the one it opens:

```rust
    /// The set's new rows are written first, each by one `INSERT`, then
    /// its changed rows, then its rows to delete, each by one `DELETE`.
```

```rust
    /// A row to delete is found by its key and locked with the changed
    /// rows, before anything is written, and no other column of it is
    /// compared. One that is not there is no failure and no conflict:
    /// the rest of the save is written, and [`WriteOutcome::Written`]
    /// says which rows were already gone. One the database refuses to
    /// delete, a row a foreign key holds most often, undoes the save
    /// ([`WriteOutcome::FailedDelete`]).
```

`crates/tabletist-db/src/write.rs` begins "//! Changing rows: what a save asks for, and what came of it." It now begins:

```rust
//! Changing, adding and deleting rows: what a save asks for, and what came
//! of it.
```

- [ ] **Step 2: Run the four checks**

Expected: all pass, `cargo doc` among them (a link to `WriteOutcome::FailedDelete` that does not resolve fails it).

- [ ] **Step 3: Commit**

```bash
git add crates/tabletist-db && git commit -m "Say that a save deletes rows after it changes them

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

## After the last task

- Run the four checks once more on the branch's head, and the shots' clippy.
- Report what ran: SQLite's delete tests ran; PostgreSQL's and MySQL's were compiled and, without servers, not run.
- Do not open the pull request yet: nothing a user sees has changed. It is opened when run 2 is in, unless the user asks for a draft sooner.
- Write run 2's plan from the tree as it then stands (`docs/superpowers/plans/2026-10-12-deleting-rows-2-marking.md`), after reading the boards again. Before it, the user answers the rows of "What the spec asks, and what the app can do today" that are not run 1's.
