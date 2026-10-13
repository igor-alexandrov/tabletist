# Deleting Many Rows, Slice 3: Deletes in Runs Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A save deletes its marked rows in runs of up to 100, `WHERE "id" IN (…)`, and Review SQL shows them that way. A run the database refuses is undone and sent one row at a time, so the error still lands on its row. The counts of what a delete takes stay bounded with many rows marked, and the note of rows already gone stays short.

**Architecture:** The app sends what it sends today, one `RowDelete` for each marked row. `tabletist-db` cuts them into runs (`delete_runs`) and spells a run (`Dialect::delete_run`); the review and the three drivers call the same two functions. Each driver keeps its one-row delete as a function of its own and adds the run around it: a savepoint, the run's statement, and the fallback to one row at a time. Which errors are a busy database is the dialect's to say (`Dialect::busy`).

**Tech Stack:** Rust 1.98, `tokio-postgres`, `mysql_async`, `rusqlite`; egui/eframe. Spec: `docs/superpowers/specs/2026-10-13-deleting-many-rows-design.md` ("spec N" is one of its sections, "test N" an entry of its Tests, "M N" a row of its decisions, all approved at their defaults). This is slice 3 of its three: sections 4 to 6, and tests 13 to 18. Slices 1 and 2 are built, in pull request `igor-alexandrov/tabletist#120`.

Line numbers are of head `ac63a1b`.

---

## Where the plan leaves the spec, for a yes

| # | The spec says | The plan does | Why |
|---|---|---|---|
| R1 | `write::delete_runs(&[RowDelete], built: &[bool])` "leaves a row that was not built out of any run" | It covers every row: a row that was not built is a run of its own, of one | The review walks the runs in order and needs the refused row's place among them |
| R2 | `Line::Delete` carries the comment it is given | A new `Line::DeleteRun(usize)` for `-- delete 3 rows`; `Line::Delete` keeps the row's name | The view words the lines, in the user's language |
| R3 | A run is laid out "as one row's is: the table, then the `WHERE`" | One line where the key is one column, as one row's is. Two where it is several: the table, then ` WHERE ("a", "b") IN ((…), (…));` on one line | A line for each row of the run would be 100 lines |
| R4 | A driver's steps for a run | The same, and one more: a run that cannot be spelled (it cannot happen, each row was built) goes one row at a time | No `unwrap`, and no new way for a save to fail |
| R5 | "A key of several columns is counted … up to 100 marked rows" | Above 100 rows the key's line stands without a number by the path that already says "the page does not hold what the key points at" (`pointing_at` gives `None`) | One existing state, no new one |
| R6 | Test 15: "a run with one row a foreign key holds rolls the save back and names that row" | The scenario that is already there (`a_row_a_foreign_key_holds_stays_and_the_save_is_undone`): its two deletes are now one run | It asserts exactly that, on all three engines |
| R7 | MySQL: the `EXPLAIN` of a run by a key of two columns must not be a scan of the whole table | Checked on a table of 300 rows made for it. If the server scans, stop and report: do not weaken the check | On three rows any plan is a scan |

## Before you start

- Cargo is `~/.cargo/bin/cargo` (the mise shim fails). Never point `CARGO_TARGET_DIR` at `/tmp`.
- The four checks, from `AGENTS.md`. "Run the four checks" below means these, and all four pass after every task:

```bash
~/.cargo/bin/cargo fmt --all --check
~/.cargo/bin/cargo clippy --locked --workspace --all-targets -- -D warnings
~/.cargo/bin/cargo test --locked --workspace --all-targets
RUSTDOCFLAGS='-D warnings' ~/.cargo/bin/cargo doc --locked --workspace --no-deps
```

- The blocks below are not all as rustfmt lays them out. Run `~/.cargo/bin/cargo fmt --all` before the four checks of each task, and commit what it gives.
- After task 5, also: `~/.cargo/bin/cargo clippy --locked --features shots --lib --tests -- -D warnings`.
- **No Docker here.** PostgreSQL and MySQL tests print `skipped` and pass. Tasks 3 and 4 are checked by reading, by the compiler, and by CI: the pull request's integration jobs run `--test delete_rows` against both servers on every push. After the push at the end, read the two jobs' result once. Auto-fix reports a failure by itself; do not poll.
- The engines differ: write each engine's code with `match` on every `Dialect` by name (`tests/engines.rs`), and type-check nothing for another platform (this slice has no platform code).
- One commit per task, signed. `error: 1Password: agent refused operation` means 1Password is locked: stage the task, record its tree (`git write-tree`), go on, and commit the trees in order once it is unlocked. Never commit unsigned.
- Commit messages end with `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.
- Write each test first and see it fail before the code that makes it pass.
- Tests use the Bookshop only. UI copy is short and plain, with no em dashes.

## Files

| File | What it gets |
|---|---|
| `crates/tabletist-db/src/write.rs` | `MAX_RUN`, `delete_runs`, the savepoint's three statements, `deleted_instead` |
| `crates/tabletist-db/src/dialect.rs` | `Dialect::delete_run`, `Dialect::busy`, `DeleteParts` for a run |
| `crates/tabletist-db/src/lib.rs` | `delete_runs` and `MAX_RUN` exported |
| `crates/tabletist-db/src/sqlite/write.rs`, `pg/write.rs`, `mysql/write.rs` | `delete_one` (today's loop body), `delete_many` (the run), the loop over runs |
| `crates/tabletist-db/tests/delete_rows.rs` | Two tables more on each engine (`stock`, `genres`), five scenarios, SQLite's two triggers, MySQL's `EXPLAIN` |
| `src/review.rs`, `src/ui/review.rs` | `Line::DeleteRun`, `lay_run`, the deletes walked by run |
| `src/edit.rs` | `pointing_at` counts in parts of 1,000 values and stops at 100 counts |
| `src/ui/pending_bar.rs` | The note of rows already gone names three and counts the rest |
| `docs/_guide/editing-data.md`, both specs, this plan | What a save sends, and what was built |

---

### Task 1: Cut the deletes into runs, and spell a run

**Files:**
- Modify: `crates/tabletist-db/src/write.rs` (after `RowDelete` at 79, its tests)
- Modify: `crates/tabletist-db/src/dialect.rs` (`DeleteParts` at 95, after `delete_row` at 880, tests at 1597)
- Modify: `crates/tabletist-db/src/lib.rs:56`

- [ ] **Step 1: Write the failing tests**

`crates/tabletist-db/src/write.rs`, in its tests module (read its head first for what it imports, and build a `RowDelete` as its other tests do):

```rust
    fn by(columns: &[&str], id: i64) -> RowDelete {
        RowDelete {
            key: columns
                .iter()
                .map(|column| ((*column).to_owned(), Value::Int(id)))
                .collect(),
        }
    }

    #[test]
    fn deletes_are_cut_into_runs_of_neighbours_with_the_same_key() {
        let rows: Vec<RowDelete> = (0..250).map(|id| by(&["id"], id)).collect();
        let built = vec![true; rows.len()];
        assert_eq!(delete_runs(&rows, &built), [0..100, 100..200, 200..250]);
        // A key of other columns begins a run, and so does one in another
        // order.
        let rows = [
            by(&["id"], 1),
            by(&["id"], 2),
            by(&["book_id", "author_id"], 3),
            by(&["author_id", "book_id"], 4),
            by(&["author_id", "book_id"], 5),
        ];
        assert_eq!(delete_runs(&rows, &[true; 5]), [0..2, 2..3, 3..5]);
        // A row that was not built stands alone, between two runs.
        let rows: Vec<RowDelete> = (0..5).map(|id| by(&["id"], id)).collect();
        let built = [true, true, false, true, true];
        assert_eq!(delete_runs(&rows, &built), [0..2, 2..3, 3..5]);
        assert!(delete_runs(&[], &[]).is_empty());
    }
```

`crates/tabletist-db/src/dialect.rs`, in its tests, after `a_delete_says_where_its_parts_stand`:

```rust
    /// Spec 4 and test 13: a run of rows is one statement, `IN` its keys.
    #[test]
    fn a_run_of_deletes_on_each_engine() {
        let rows = [9, 10, 11].map(|id| doomed(vec![("id", Value::Int(id))]));
        let rows: Vec<&RowDelete> = rows.iter().collect();
        let run = Dialect::Postgres
            .delete_run(&covers("public"), &rows)
            .unwrap();
        assert_eq!(
            run.shown,
            r#"DELETE FROM "public"."book_covers" WHERE "id" IN (9, 10, 11)"#
        );
        assert_eq!(run.sql.text, run.shown);
        assert!(run.sql.params.is_empty());
        let run = Dialect::MySql
            .delete_run(&covers("bookshop"), &rows)
            .unwrap();
        assert_eq!(
            run.sql.text,
            "DELETE FROM `bookshop`.`book_covers` WHERE `id` IN (?, ?, ?)"
        );
        assert_eq!(run.sql.params, [9, 10, 11].map(Value::Int));
        assert_eq!(inlined(Dialect::MySql, &run.sql), run.shown);
        let run = Dialect::Sqlite.delete_run(&covers("main"), &rows).unwrap();
        assert_eq!(
            run.sql.text,
            r#"DELETE FROM "main"."book_covers" WHERE "id" IN (?, ?, ?)"#
        );
        assert_eq!(inlined(Dialect::Sqlite, &run.sql), run.shown);
    }

    /// A key of several columns: rows on the right of `IN`, which SQLite
    /// takes only as `VALUES`.
    #[test]
    fn a_run_by_a_key_of_two_columns_on_each_engine() {
        let authors = ObjectRef::new("public", "book_authors");
        let rows = [
            doomed(vec![("book_id", Value::Int(101)), ("author", text("Ada Quill"))]),
            doomed(vec![("book_id", Value::Int(101)), ("author", text("Sam O'Neil"))]),
        ];
        let rows: Vec<&RowDelete> = rows.iter().collect();
        let run = Dialect::Postgres.delete_run(&authors, &rows).unwrap();
        assert_eq!(
            run.shown,
            r#"DELETE FROM "public"."book_authors" WHERE ("book_id", "author") IN ((101, 'Ada Quill'), (101, 'Sam O''Neil'))"#
        );
        let run = Dialect::MySql.delete_run(&authors, &rows).unwrap();
        assert_eq!(
            run.sql.text,
            "DELETE FROM `public`.`book_authors` WHERE (`book_id`, `author`) IN ((?, ?), (?, ?))"
        );
        assert_eq!(inlined(Dialect::MySql, &run.sql), run.shown);
        let run = Dialect::Sqlite.delete_run(&authors, &rows).unwrap();
        assert_eq!(
            run.sql.text,
            r#"DELETE FROM "public"."book_authors" WHERE ("book_id", "author") IN (VALUES (?, ?), (?, ?))"#
        );
        assert_eq!(inlined(Dialect::Sqlite, &run.sql), run.shown);
        // Its parts: each column once, each value in the rows' order.
        let parts = &run.parts;
        assert!(run.shown[parts.filter..].starts_with("WHERE "));
        assert_eq!(parts.columns.len(), 2);
        assert!(run.shown[parts.columns[1]..].starts_with(r#""author""#));
        let values: Vec<&str> = parts.values.iter().map(|at| &run.shown[at.clone()]).collect();
        assert_eq!(values, ["101", "'Ada Quill'", "101", "'Sam O''Neil'"]);
    }

    /// A run of rows whose keys differ, or of fewer than two, is no run.
    #[test]
    fn a_run_needs_two_rows_with_the_same_key_columns() {
        let one = doomed(vec![("id", Value::Int(9))]);
        let other = doomed(vec![("isbn", Value::Int(9))]);
        let covers = covers("public");
        assert!(Dialect::Postgres.delete_run(&covers, &[&one]).is_err());
        assert!(Dialect::Postgres.delete_run(&covers, &[&one, &other]).is_err());
        // A row no save sends refuses the run it is in.
        let lossy = doomed(vec![("id", text("a\u{FFFD}"))]);
        let fine = doomed(vec![("id", text("a"))]);
        assert!(Dialect::Sqlite.delete_run(&covers, &[&fine, &lossy]).is_err());
    }

    /// Spec 4, step 5: what each engine says when it was busy, not when a
    /// row was wrong.
    #[test]
    fn each_engine_says_it_was_busy_in_its_own_codes() {
        let coded = |code: &str, number: Option<u32>| Error::Query {
            code: Some(code.into()),
            message: "no".into(),
            detail: None,
            hint: None,
            named: Box::new(crate::Named {
                number,
                constraint: None,
            }),
        };
        for code in ["40P01", "55P03", "40001"] {
            assert!(Dialect::Postgres.busy(&coded(code, None)), "{code}");
        }
        assert!(!Dialect::Postgres.busy(&coded("23503", None)));
        for number in [1205, 1213] {
            assert!(Dialect::MySql.busy(&coded("HY000", Some(number))), "{number}");
        }
        assert!(!Dialect::MySql.busy(&coded("23000", Some(1451))));
        // SQLite's code is the extended one: busy (5) and locked (6) under
        // whatever stands above them.
        for code in ["5", "6", "261", "517", "262"] {
            assert!(Dialect::Sqlite.busy(&coded(code, None)), "{code}");
        }
        assert!(!Dialect::Sqlite.busy(&coded("787", None)));
        assert!(!Dialect::Sqlite.busy(&coded("1811", None)));
        assert!(!Dialect::Sqlite.busy(&Error::query("no")));
    }
```

(If `Named` has other fields, fill them as `tests/delete_rows.rs` or `src/ui/delete_tests.rs::refused` does. `doomed`, `covers`, `text` and `inlined` are the tests' own helpers at 1585.)

- [ ] **Step 2: Run them to see them fail**

Run: `~/.cargo/bin/cargo test --locked -p tabletist-db --lib a_run`
Expected: does not compile: `no method named delete_run`, `cannot find function delete_runs`.

- [ ] **Step 3: The runs**

`crates/tabletist-db/src/write.rs`, after `struct RowDelete`:

```rust
/// How many rows one `DELETE` of a save names, at most.
pub const MAX_RUN: usize = 100;

/// The save's deletes, cut into runs in their order: neighbours whose
/// keys name the same columns in the same order, at most [`MAX_RUN`] of
/// them, go in one statement ([`crate::Dialect::delete_run`]). `built`
/// says of each row whether its own statement could be built
/// ([`crate::Dialect::delete_row`]): one that could not is a run of its
/// own, since no statement holds it. Every row is in exactly one run.
pub fn delete_runs(deletes: &[RowDelete], built: &[bool]) -> Vec<std::ops::Range<usize>> {
    let same_columns = |a: &RowDelete, b: &RowDelete| {
        a.key.len() == b.key.len() && a.key.iter().zip(&b.key).all(|(a, b)| a.0 == b.0)
    };
    let sent = |row: usize| built.get(row).copied().unwrap_or(false);
    let mut runs: Vec<std::ops::Range<usize>> = Vec::new();
    for (row, delete) in deletes.iter().enumerate() {
        let joins = runs.last().is_some_and(|run| {
            sent(row)
                && sent(run.start)
                && run.len() < MAX_RUN
                && same_columns(&deletes[run.start], delete)
        });
        match runs.last_mut() {
            Some(run) if joins => run.end = row + 1,
            _ => runs.push(row..row + 1),
        }
    }
    runs
}

/// A point inside the save's transaction to come back to: before a run of
/// deletes, so a run the database refuses can be undone and its rows sent
/// one at a time. The three statements are every engine's.
pub(crate) const SAVEPOINT: &str = "SAVEPOINT tabletist_run";
pub(crate) const ROLLBACK_TO: &str = "ROLLBACK TO SAVEPOINT tabletist_run";
pub(crate) const RELEASE: &str = "RELEASE SAVEPOINT tabletist_run";

/// A statement deleted another number of rows than it named.
pub(crate) fn deleted_instead(deleted: u64, meant: usize) -> Error {
    let meant = if meant == 1 {
        "one".to_owned()
    } else {
        meant.to_string()
    };
    Error::query(format!(
        "the save would have deleted {deleted} rows where it meant {meant}"
    ))
}
```

`crates/tabletist-db/src/lib.rs`: add `MAX_RUN` and `delete_runs` to `pub use write::{…}`.

(The savepoint's statements and `deleted_instead` have no user until Task 2. If clippy calls them dead code, write them in Task 2 instead: they are named here so the three drivers share them.)

- [ ] **Step 4: Spell a run, and say what busy is**

`crates/tabletist-db/src/dialect.rs`. `DeleteParts`' doc gains a sentence, and `columns`' too:

```rust
/// … A run's (`Dialect::delete_run`) has each key column once, and each
/// value's place row by row: as many values as the rows times the columns.
```

After `delete_row`:

```rust
    /// One `DELETE` for a run of a save's rows ([`crate::delete_runs`]):
    /// `WHERE "id" IN (..)`, or `WHERE ("a", "b") IN ((..), (..))` for a
    /// key of several columns. `Err` where it is no run (fewer than two
    /// rows, or keys that do not name the same columns in the same order),
    /// and where one of its rows is one no save sends
    /// ([`Dialect::delete_row`]). The rows it finds are the rows its
    /// rows' own statements find: each value is spelled as there, and
    /// compared as `=` compares it.
    pub fn delete_run(self, object: &ObjectRef, rows: &[&RowDelete]) -> Result<RowDeletion> {
        let [first, _, ..] = rows else {
            return Err(Error::query("a run of deletes is two rows or more"));
        };
        let same = |row: &&RowDelete| {
            row.key.len() == first.key.len()
                && row.key.iter().zip(&first.key).all(|(a, b)| a.0 == b.0)
        };
        if first.key.is_empty() || !rows.iter().all(same) {
            return Err(Error::query("the rows of a run have the same key columns"));
        }
        for row in rows {
            self.key_read_exactly(&row.key)?;
        }
        let several = first.key.len() > 1;
        let mut params = Vec::new();
        let mut shown = format!("DELETE FROM {}", self.qualified(object));
        let mut parts = DeleteParts {
            // The clause begins with the space before its word.
            filter: shown.len() + 1,
            ..DeleteParts::default()
        };
        shown.push_str(" WHERE ");
        let mut sent = shown.clone();
        if several {
            shown.push('(');
            sent.push('(');
        }
        for (index, (column, _)) in first.key.iter().enumerate() {
            let column = self.quote_ident(column);
            let lead = if index == 0 { "" } else { ", " };
            shown.push_str(lead);
            parts.columns.push(shown.len());
            shown.push_str(&column);
            let _ = write!(sent, "{lead}{column}");
        }
        // A list of rows on the right of `IN`: SQLite takes one only as
        // `VALUES`.
        let open = match (self, several) {
            (_, false) => " IN (",
            (Self::Postgres | Self::MySql, true) => ") IN (",
            (Self::Sqlite, true) => ") IN (VALUES ",
        };
        shown.push_str(open);
        sent.push_str(open);
        for (row, delete) in rows.iter().enumerate() {
            let lead = match (row, several) {
                (0, false) => "",
                (_, false) => ", ",
                (0, true) => "(",
                (_, true) => ", (",
            };
            shown.push_str(lead);
            sent.push_str(lead);
            for (index, (_, value)) in delete.key.iter().enumerate() {
                let operand = self.key_operand(value);
                let lead = if index == 0 { "" } else { ", " };
                shown.push_str(lead);
                let literal = self.shown(&operand);
                let at = shown.len();
                parts.values.push(at..at + literal.len());
                shown.push_str(&literal);
                let bound = self.sent(&operand, &mut params);
                let _ = write!(sent, "{lead}{bound}");
            }
            if several {
                shown.push(')');
                sent.push(')');
            }
        }
        shown.push(')');
        sent.push(')');
        // PostgreSQL text cannot hold a NUL: see `delete_row`.
        let refused = match self {
            Self::Postgres => sent.contains('\0'),
            Self::MySql | Self::Sqlite => false,
        };
        if refused {
            return Err(Error::query("PostgreSQL text cannot hold a NUL character"));
        }
        Ok(RowDeletion {
            shown,
            sql: Sql { text: sent, params },
            parts,
        })
    }

    /// Whether `error` says the database was busy, not that a statement
    /// was wrong: a lock that was waited for too long, a deadlock, a
    /// transaction it could not put in order with another. A run of
    /// deletes that ends so is not sent again row by row: each row could
    /// wait as long once more.
    pub fn busy(self, error: &Error) -> bool {
        let Error::Query { code, named, .. } = error else {
            return false;
        };
        match self {
            // Deadlock detected, lock not available, serialization failure.
            Self::Postgres => matches!(code.as_deref(), Some("40P01" | "55P03" | "40001")),
            // Lock wait timeout, deadlock.
            Self::MySql => matches!(named.number, Some(1205 | 1213)),
            // SQLITE_BUSY and SQLITE_LOCKED, under whatever the extended
            // code adds above its low byte.
            Self::Sqlite => code
                .as_deref()
                .and_then(|code| code.parse::<u32>().ok())
                .is_some_and(|code| matches!(code & 0xff, 5 | 6)),
        }
    }
```

Read `key_clause` (656) first: `key_operand`, `shown` and `sent` are used here as it uses them. Where their names or signatures differ from this block, follow `key_clause`.

- [ ] **Step 5: Run the tests**

Run: `~/.cargo/bin/cargo fmt --all`, `~/.cargo/bin/cargo test --locked -p tabletist-db --lib`, then the four checks.
Expected: PASS. `tests/engines.rs` still passes: both new functions name every dialect.

- [ ] **Step 6: Commit**

```bash
git add crates/tabletist-db/src/write.rs crates/tabletist-db/src/dialect.rs crates/tabletist-db/src/lib.rs
git commit -m "Cut a save's deletes into runs and spell a run on each engine

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 2: SQLite deletes in runs

**Files:**
- Modify: `crates/tabletist-db/src/sqlite/write.rs` (the delete loop at 413-456)
- Modify: `crates/tabletist-db/tests/delete_rows.rs` (`Shop` at 30, scenarios after 269, `mod sqlite` at 271)

- [ ] **Step 1: Two tables more, and the scenarios**

`crates/tabletist-db/tests/delete_rows.rs`. `Shop` gains `stock: String` and `genres: String`, set in each engine's `shop()` as the others are (`"stock"`, `"genres"` for SQLite; `format!("{prefix}_stock")`, `format!("{prefix}_genres")` for the other two, which Tasks 3 and 4 use).

A helper, beside `by_id`:

```rust
/// The rows of the stock table, as the values of one `INSERT`: 300 rows,
/// numbered from 1.
fn stock_values() -> String {
    let rows: Vec<String> = (1..=300).map(|id| format!("({id})")).collect();
    rows.join(", ")
}
```

SQLite's `CREATE` becomes a function, since one of its tables is made of 300 rows:

```rust
    fn create() -> String {
        format!(
            "{CREATE};
             CREATE TABLE stock (id INTEGER PRIMARY KEY);
             INSERT INTO stock VALUES {};
             CREATE TABLE genres (
                 id INTEGER PRIMARY KEY,
                 parent_id INTEGER REFERENCES genres (id)
             );
             INSERT INTO genres VALUES (1, NULL), (2, 1), (3, NULL)",
            stock_values()
        )
    }
```

and `bookshop()` runs `execute_batch(&create())`. (`genres`' key holds: no `ON DELETE`. The parent's id is the lower.)

The scenarios, after `a_row_the_save_took_with_another_is_deleted`. "Spec" and "test" in these are the many-rows spec's (`2026-10-13-deleting-many-rows-design.md`); say so in the file's head comment.

```rust
/// Spec 4 and test 15: 250 rows of one save go in three runs, and are
/// gone.
async fn many_rows_go_in_runs(connection: &Connection, shop: &Shop) {
    let stock = shop.table(&shop.stock);
    let deletes: Vec<RowDelete> = (1..=250).map(by_id).collect();
    let outcome = save(connection, &deleting(stock.clone(), deletes)).await;
    assert!(
        matches!(&outcome, WriteOutcome::Written { gone, .. } if gone.is_empty()),
        "{outcome:?}"
    );
    let left = firsts(connection, stock).await;
    assert_eq!(left.len(), 50);
    assert!(left.contains(&"251".to_owned()) && left.contains(&"300".to_owned()));
}

/// Test 15: rows 5 and 7 were deleted by someone else before this is
/// called. The save says which of its deletes they were, and deletes the
/// rest of their run.
async fn rows_already_gone_leave_the_rest_of_their_run(connection: &Connection, shop: &Shop) {
    let stock = shop.table(&shop.stock);
    let deletes: Vec<RowDelete> = (1..=10).map(by_id).collect();
    let outcome = save(connection, &deleting(stock.clone(), deletes)).await;
    let WriteOutcome::Written { gone, .. } = outcome else {
        panic!("the save wrote: {outcome:?}");
    };
    assert_eq!(gone, [4, 6]);
    assert_eq!(firsts(connection, stock).await.len(), 290);
}

/// Spec 4 and test 15: a run by a key of two columns finds its rows.
async fn a_run_by_a_key_of_two_columns_finds_its_rows(connection: &Connection, shop: &Shop) {
    let authors = shop.table(&shop.authors);
    let by = |book: i64, author: &str| RowDelete {
        key: vec![
            ("book_id".to_owned(), Value::Int(book)),
            ("author".to_owned(), Value::Text(author.into())),
        ],
    };
    let changes = deleting(
        authors.clone(),
        vec![by(101, "Ada Quill"), by(101, "Sam O'Neil")],
    );
    let outcome = save(connection, &changes).await;
    assert!(
        matches!(&outcome, WriteOutcome::Written { gone, .. } if gone.is_empty()),
        "{outcome:?}"
    );
    assert_eq!(rows(connection, authors).await, [["102", "Ada Quill"]]);
}

/// Spec 4, step 6, and test 15: Crime (2) stands under Fiction (1), by a
/// key that holds. The app sends the child first, and the two are one
/// run. An engine that checks the key row by row refuses the run and
/// takes the rows one at a time; either way both are gone.
async fn a_row_and_the_row_it_points_at_go_in_one_run(connection: &Connection, shop: &Shop) {
    let genres = shop.table(&shop.genres);
    let outcome = save(
        connection,
        &deleting(genres.clone(), vec![by_id(2), by_id(1)]),
    )
    .await;
    assert!(
        matches!(&outcome, WriteOutcome::Written { gone, .. } if gone.is_empty()),
        "{outcome:?}"
    );
    assert_eq!(firsts(connection, genres).await, ["3"]);
}
```

In `mod sqlite`, a test for each (the second deletes rows 5 and 7 first, through `other_program(&dir)`), and two of SQLite's own:

```rust
    #[tokio::test]
    async fn sqlite_deletes_many_rows_in_runs() {
        let (connection, _dir) = bookshop().await;
        many_rows_go_in_runs(&connection, &shop()).await;
    }

    #[tokio::test]
    async fn sqlite_says_which_rows_of_a_run_were_gone() {
        let (connection, dir) = bookshop().await;
        other_program(&dir)
            .execute_batch("DELETE FROM stock WHERE id IN (5, 7)")
            .unwrap();
        rows_already_gone_leave_the_rest_of_their_run(&connection, &shop()).await;
    }

    #[tokio::test]
    async fn sqlite_deletes_a_run_by_a_key_of_two_columns() {
        let (connection, _dir) = bookshop().await;
        a_run_by_a_key_of_two_columns_finds_its_rows(&connection, &shop()).await;
    }

    #[tokio::test]
    async fn sqlite_deletes_a_row_and_the_row_it_points_at_in_one_run() {
        let (connection, _dir) = bookshop().await;
        a_row_and_the_row_it_points_at_go_in_one_run(&connection, &shop()).await;
    }

    /// Spec 4, step 3, and test 16: a trigger keeps one row of a run. The
    /// save fails on that row, and nothing of it is written.
    #[tokio::test]
    async fn sqlite_fails_the_row_of_a_run_a_trigger_kept() {
        let (connection, dir) = bookshop().await;
        other_program(&dir)
            .execute_batch(
                "CREATE TRIGGER keep_seven BEFORE DELETE ON stock WHEN OLD.id = 7
                 BEGIN SELECT RAISE(IGNORE); END",
            )
            .unwrap();
        let stock = shop().table("stock");
        let deletes: Vec<RowDelete> = (5..=9).map(by_id).collect();
        let outcome = save(&connection, &deleting(stock.clone(), deletes)).await;
        let WriteOutcome::FailedDelete { delete, error } = outcome else {
            panic!("the save was to fail on a row to delete: {outcome:?}");
        };
        assert_eq!(delete, 2);
        assert!(error.to_string().contains("deleted 0 rows"), "{error}");
        assert_eq!(firsts(&connection, stock).await.len(), 300);
    }

    /// Spec 4, step 7, and test 16: a trigger rolls the transaction back
    /// under a run. Nothing more is sent, and the run's error is its first
    /// row's.
    #[tokio::test]
    async fn sqlite_sends_nothing_after_a_trigger_rolled_the_save_back() {
        let (connection, dir) = bookshop().await;
        other_program(&dir)
            .execute_batch(
                "CREATE TRIGGER stop_at_seven BEFORE DELETE ON stock WHEN OLD.id = 7
                 BEGIN SELECT RAISE(ROLLBACK, 'seven stays'); END",
            )
            .unwrap();
        let stock = shop().table("stock");
        let deletes: Vec<RowDelete> = (5..=9).map(by_id).collect();
        let outcome = save(&connection, &deleting(stock.clone(), deletes)).await;
        let WriteOutcome::FailedDelete { delete, error } = outcome else {
            panic!("the save was to fail on a row to delete: {outcome:?}");
        };
        assert_eq!(delete, 0);
        assert!(error.to_string().contains("seven stays"), "{error}");
        // Not one row went: had the rows been sent one at a time after
        // the rollback, each would have been written for good.
        assert_eq!(firsts(&connection, stock).await.len(), 300);
        // And the session saves again.
        let outcome = save(&connection, &deleting(stock.clone(), vec![by_id(1)])).await;
        assert!(matches!(outcome, WriteOutcome::Written { .. }), "{outcome:?}");
    }
```

- [ ] **Step 2: Run them to see which fail**

Run: `~/.cargo/bin/cargo test --locked -p tabletist-db --test delete_rows sqlite`
Expected: the four shared scenarios PASS already (one statement a row deletes the same rows). `sqlite_fails_the_row_of_a_run_a_trigger_kept` passes too, on today's one-row path. `sqlite_sends_nothing_after_a_trigger_rolled_the_save_back` FAILS: today the trigger's failure is row 7's own, `delete == 2`. The scenarios are what the runs must keep; a test that the statements are runs is next.

No test here says "one statement" from outside the driver. What says it is Task 1's `delete_run` strings, and the trigger that rolls back: with one statement a row its failure is row 7's (`delete == 2`), and with a run it is the run's first row's (`delete == 0`).

- [ ] **Step 3: The run in the driver**

`crates/tabletist-db/src/sqlite/write.rs`. Add to its `use crate::write::{…}`: `RELEASE`, `ROLLBACK_TO`, `SAVEPOINT`, `deleted_instead`, and `crate::delete_runs`, `crate::RowDelete`.

The loop over the deletes (413-456) becomes a loop over runs, and its body two functions:

```rust
    // The rows to delete, last: the order a review shows them in, in runs
    // of neighbours. One that was not there is said and not sent.
    let mut gone = Vec::new();
    // Every row's statement was built: the save does not begin otherwise.
    let built = vec![true; changes.deletes.len()];
    for run in delete_runs(&changes.deletes, &built) {
        let (live, missing): (Vec<usize>, Vec<usize>) =
            run.partition(|&delete| doomed[delete].is_some());
        gone.extend(missing);
        let failed = match live.as_slice() {
            [] => None,
            [delete] => delete_one(connection, changes, &deletions[*delete], *delete, stop)?,
            _ => delete_many(connection, changes, deletions, &live, stop)?,
        };
        if let Some(failed) = failed {
            return Ok(failed);
        }
    }
```

```rust
/// Deletes one row of the save by its own statement. `Ok(Some(..))` is
/// how the save ends where the row is not deleted; `Err` is a cancel, a
/// stop or a lost session.
fn delete_one(
    connection: &rusqlite::Connection,
    changes: &ChangeSet,
    deletion: &RowDeletion,
    delete: usize,
    stop: &StopFlag,
) -> Result<Option<Applied>> {
    not_stopped(stop)?;
    let params = rusqlite::params_from_iter(deletion.sql.params.iter().map(exact));
    let touched = match connection
        .execute(&deletion.sql.text, params)
        .map_err(map_error)
    {
        Ok(touched) => touched,
        // A cancel or a lost session ends the save; anything else is the
        // statement's own failure: a foreign key that holds the row, most
        // often.
        Err(error @ (Error::Cancelled | Error::ConnectionLost(_))) => return Err(error),
        Err(error) => return Ok(Some(Applied::FailedDelete { delete, error })),
    };
    // The count is of the table's own rows, without those a foreign key
    // took with them. None: a row deleted before this one took it along (a
    // key of the table that points at the table), which is what the save
    // meant, or a trigger kept it, which is not. The row says which.
    if touched == 0 && !still_there(connection, changes, delete, stop)? {
        return Ok(None);
    }
    Ok((touched != 1).then(|| Applied::FailedDelete {
        delete,
        error: deleted_instead(touched as u64, 1),
    }))
}

/// Whether the row of the save's delete `delete` is still in the table.
fn still_there(
    connection: &rusqlite::Connection,
    changes: &ChangeSet,
    delete: usize,
    stop: &StopFlag,
) -> Result<bool> {
    let key = &changes.deletes[delete].key;
    let select = Dialect::Sqlite.select_row(&changes.object, key, false);
    not_stopped(stop)?;
    let (_, still) = read(connection, &select)?;
    Ok(!still.is_empty())
}

/// Deletes the rows `live` of the save (places among its deletes, each
/// found by its read, two or more) in one statement. A run the database
/// refuses is undone to the savepoint before it, and its rows go one at a
/// time, so the error is its row's.
fn delete_many(
    connection: &rusqlite::Connection,
    changes: &ChangeSet,
    deletions: &[RowDeletion],
    live: &[usize],
    stop: &StopFlag,
) -> Result<Option<Applied>> {
    let singly = || -> Result<Option<Applied>> {
        for &delete in live {
            let failed = delete_one(connection, changes, &deletions[delete], delete, stop)?;
            if failed.is_some() {
                return Ok(failed);
            }
        }
        Ok(None)
    };
    let first = live[0];
    let rows: Vec<&RowDelete> = live.iter().map(|&delete| &changes.deletes[delete]).collect();
    // Each row was built, so the run is: were it not, the rows go as ever.
    let Ok(run) = Dialect::Sqlite.delete_run(&changes.object, &rows) else {
        return singly();
    };
    not_stopped(stop)?;
    connection.execute_batch(SAVEPOINT).map_err(map_error)?;
    let params = rusqlite::params_from_iter(run.sql.params.iter().map(exact));
    let touched = match connection.execute(&run.sql.text, params).map_err(map_error) {
        Ok(touched) => touched,
        Err(error @ (Error::Cancelled | Error::ConnectionLost(_))) => return Err(error),
        // Busy: sent again row by row, each row could wait as long.
        Err(error) if Dialect::Sqlite.busy(&error) => {
            return Ok(Some(Applied::FailedDelete {
                delete: first,
                error,
            }));
        }
        Err(error) => {
            return match connection.execute_batch(ROLLBACK_TO).map_err(map_error) {
                Ok(()) => {
                    let failed = singly()?;
                    if failed.is_none() {
                        connection.execute_batch(RELEASE).map_err(map_error)?;
                    }
                    Ok(failed)
                }
                Err(lost @ (Error::Cancelled | Error::ConnectionLost(_))) => Err(lost),
                // No savepoint to come back to: the transaction is gone
                // (a trigger rolled it back), and a row sent now would be
                // written for good. The run's error ends the save.
                Err(_) => Ok(Some(Applied::FailedDelete {
                    delete: first,
                    error,
                })),
            };
        }
    };
    if touched > live.len() {
        return Ok(Some(Applied::FailedDelete {
            delete: first,
            error: deleted_instead(touched as u64, live.len()),
        }));
    }
    if touched < live.len() {
        // A row of the run that a delete of this save took along (a key
        // of the table that points at the table) is what the save meant.
        // One a trigger kept is not.
        for &delete in live {
            if still_there(connection, changes, delete, stop)? {
                return Ok(Some(Applied::FailedDelete {
                    delete,
                    error: deleted_instead(0, 1),
                }));
            }
        }
    }
    connection.execute_batch(RELEASE).map_err(map_error)?;
    Ok(None)
}
```

`live[0]` is in bounds: the caller sends two rows or more. If clippy's `indexing_slicing` is denied in this crate, take `first` with `let [first, ..] = *live else { return Ok(None) };`.

- [ ] **Step 4: Run the tests**

Run: `~/.cargo/bin/cargo fmt --all`, `~/.cargo/bin/cargo test --locked -p tabletist-db`, then the four checks.
Expected: PASS, the two trigger tests among them. `sqlite_keeps_a_row_a_foreign_key_holds` still names the second of its two deletes: the run of two is refused, undone, and the second row fails alone.

- [ ] **Step 5: Commit**

```bash
git add crates/tabletist-db/src/sqlite/write.rs crates/tabletist-db/tests/delete_rows.rs crates/tabletist-db/src/write.rs
git commit -m "Delete a save's rows in runs on SQLite

A run the file refuses is undone to a savepoint and its rows go one
at a time, so the error is its row's. Where the savepoint is gone,
nothing more is sent.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 3: PostgreSQL deletes in runs

**Files:**
- Modify: `crates/tabletist-db/src/pg/write.rs` (the delete loop at 272-323)
- Modify: `crates/tabletist-db/tests/delete_rows.rs` (`mod postgres` at 419)

- [ ] **Step 1: The tables and the tests**

`mod postgres`. `DROP` names two tables more, first (`shop_genres, shop_stock, …`). `CREATE` becomes `fn create() -> String`, as SQLite's did:

```rust
    fn create() -> String {
        format!(
            "{CREATE};
             CREATE TABLE shop_stock (id bigint PRIMARY KEY);
             INSERT INTO shop_stock VALUES {};
             CREATE TABLE shop_genres (
                 id bigint PRIMARY KEY,
                 parent_id bigint REFERENCES shop_genres (id)
             );
             INSERT INTO shop_genres VALUES (1, NULL), (2, 1), (3, NULL)",
            stock_values()
        )
    }
```

`bookshop()` runs `named(&create(), prefix)`. A test for each of the four scenarios, each with a prefix of its own (`pgruns`, `pggone`, `pgtwo`, `pgkin`), ended with `admin.batch_execute(&named(DROP, prefix))` as the others are. The second deletes rows 5 and 7 first: `admin.batch_execute(&named("DELETE FROM shop_stock WHERE id IN (5, 7)", "pggone"))`.

- [ ] **Step 2: The run in the driver**

`crates/tabletist-db/src/pg/write.rs`. The same three pieces as SQLite's, async, with this driver's calls. Read 272-323 first: `delete_one` is that loop's body, moved.

```rust
/// How many of the table's own rows a statement touched.
fn touched(messages: &[SimpleQueryMessage]) -> u64 {
    let count = messages.iter().find_map(|message| match message {
        SimpleQueryMessage::CommandComplete(count) => Some(*count),
        _ => None,
    });
    count.unwrap_or(0)
}
```

```rust
async fn delete_one(
    client: &tokio_postgres::Client,
    changes: &ChangeSet,
    columns: &[ColumnMeta],
    deletion: &RowDeletion,
    delete: usize,
    stop: &StopFlag,
) -> Result<Option<Applied>> {
    not_stopped(stop)?;
    let messages = match client
        .simple_query(&deletion.sql.text)
        .await
        .map_err(query_error)
    {
        Ok(messages) => messages,
        Err(error @ (Error::Cancelled | Error::ConnectionLost(_))) => return Err(error),
        Err(error) => return Ok(Some(Applied::FailedDelete { delete, error })),
    };
    let touched = touched(&messages);
    if touched == 0 && !still_there(client, changes, columns, delete, stop).await? {
        return Ok(None);
    }
    Ok((touched != 1).then(|| Applied::FailedDelete {
        delete,
        error: deleted_instead(touched, 1),
    }))
}

async fn still_there(
    client: &tokio_postgres::Client,
    changes: &ChangeSet,
    columns: &[ColumnMeta],
    delete: usize,
    stop: &StopFlag,
) -> Result<bool> {
    let key = &changes.deletes[delete].key;
    let select = Dialect::Postgres.select_row(&changes.object, key, false);
    not_stopped(stop)?;
    Ok(!rows(client, &select.text, columns).await?.is_empty())
}
```

`delete_many` is SQLite's with `execute(client, SAVEPOINT).await?`, `client.simple_query(&run.sql.text).await.map_err(query_error)`, `touched(&messages)`, `Dialect::Postgres` throughout, and `singly` written as a loop in place (an async closure borrows awkwardly):

```rust
async fn delete_many(
    client: &tokio_postgres::Client,
    changes: &ChangeSet,
    columns: &[ColumnMeta],
    deletions: &[RowDeletion],
    live: &[usize],
    stop: &StopFlag,
) -> Result<Option<Applied>> {
    let first = live[0];
    let rows: Vec<&RowDelete> = live.iter().map(|&delete| &changes.deletes[delete]).collect();
    let run = Dialect::Postgres.delete_run(&changes.object, &rows).ok();
    // What the run's statement came to: how many rows, or that its rows
    // are to go one at a time (it could not be spelled, or was refused
    // and undone).
    let touched = match &run {
        None => None,
        Some(run) => {
            not_stopped(stop)?;
            execute(client, SAVEPOINT).await?;
            match client.simple_query(&run.sql.text).await.map_err(query_error) {
                Ok(messages) => Some(touched(&messages)),
                Err(error @ (Error::Cancelled | Error::ConnectionLost(_))) => return Err(error),
                // Busy: sent again row by row, each row could wait as
                // long.
                Err(error) if Dialect::Postgres.busy(&error) => {
                    return Ok(Some(Applied::FailedDelete {
                        delete: first,
                        error,
                    }));
                }
                // Undone, the transaction stands again as it did before
                // the run. A rollback that fails is a lost session or a
                // cancel that came late: either ends the save.
                Err(_) => {
                    execute(client, ROLLBACK_TO).await?;
                    None
                }
            }
        }
    };
    let Some(touched) = touched else {
        for &delete in live {
            let failed =
                delete_one(client, changes, columns, &deletions[delete], delete, stop).await?;
            if failed.is_some() {
                return Ok(failed);
            }
        }
        if run.is_some() {
            execute(client, RELEASE).await?;
        }
        return Ok(None);
    };
    if touched > live.len() as u64 {
        return Ok(Some(Applied::FailedDelete {
            delete: first,
            error: deleted_instead(touched, live.len()),
        }));
    }
    if touched < live.len() as u64 {
        for &delete in live {
            if still_there(client, changes, columns, delete, stop).await? {
                return Ok(Some(Applied::FailedDelete {
                    delete,
                    error: deleted_instead(0, 1),
                }));
            }
        }
    }
    execute(client, RELEASE).await?;
    Ok(None)
}
```

On PostgreSQL a failed `ROLLBACK TO SAVEPOINT` does not mean the transaction is gone (a deadlock leaves the savepoint usable): it is an error of the session, and `?` ends the save with it, which is spec 4's step 7 for this engine ("a cancel or a lost session ends the save as it does anywhere").

The loop is SQLite's: `for run in delete_runs(…)`, `live` and `missing` by `doomed[delete].is_some()`, then `delete_one(…).await?` or `delete_many(…).await?`.

- [ ] **Step 3: Check what can be checked here**

Run: `~/.cargo/bin/cargo fmt --all`, then the four checks.
Expected: PASS, with the PostgreSQL tests printing `skipped`. Then read the diff of `pg/write.rs` once against SQLite's, step for step: savepoint before the run, busy, rollback-to, one at a time, release, fewer, more.

- [ ] **Step 4: Commit**

```bash
git add crates/tabletist-db/src/pg/write.rs crates/tabletist-db/tests/delete_rows.rs
git commit -m "Delete a save's rows in runs on PostgreSQL

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 4: MySQL deletes in runs

**Files:**
- Modify: `crates/tabletist-db/src/mysql/write.rs` (the delete loop at 862-896)
- Modify: `crates/tabletist-db/tests/delete_rows.rs` (`mod mysql` at 585)

- [ ] **Step 1: The tables and the tests**

`mod mysql`. `DROP` names `shop_genres, shop_stock` first. `CREATE` is an array of statements: make it `fn create() -> Vec<String>` that holds the eight, then:

```rust
            "CREATE TABLE shop_stock (id BIGINT PRIMARY KEY)".to_owned(),
            format!("INSERT INTO shop_stock VALUES {}", stock_values()),
            "CREATE TABLE shop_genres (
                 id BIGINT PRIMARY KEY,
                 parent_id BIGINT,
                 FOREIGN KEY (parent_id) REFERENCES shop_genres (id)
             )"
            .to_owned(),
            "INSERT INTO shop_genres VALUES (1, NULL), (2, 1), (3, NULL)".to_owned(),
            // A key of two columns over many rows, for the plan of a run.
            "CREATE TABLE shop_slots (
                 shelf BIGINT NOT NULL,
                 slot BIGINT NOT NULL,
                 PRIMARY KEY (shelf, slot)
             )"
            .to_owned(),
            format!(
                "INSERT INTO shop_slots VALUES {}",
                (1..=300)
                    .map(|slot| format!("({}, {slot})", slot % 7))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
```

(`shop_slots` joins `DROP`.) A test for each of the four scenarios, as PostgreSQL's (`myruns`, `mygone`, `mytwo`, `mykin`), and the plan's check:

```rust
    /// R7: a run by a key of two columns finds its rows by the key. A
    /// server that scanned the table for it would lock every row where a
    /// row's own statement locks one.
    #[tokio::test]
    async fn mysql_finds_the_rows_of_a_run_by_their_key() {
        let Some((mut admin, _connection)) = bookshop("myplan").await else {
            return;
        };
        let explain = named(
            "EXPLAIN DELETE FROM shop_slots WHERE (shelf, slot) IN ((1, 1), (2, 2), (3, 3))",
            "myplan",
        );
        let plan: Vec<mysql_async::Row> = admin.query(explain).await.unwrap();
        let access: Vec<String> = plan
            .iter()
            .map(|row| row.get::<Option<String>, _>("type").flatten().unwrap_or_default())
            .collect();
        assert!(!access.is_empty(), "the plan has a row for the table");
        assert!(
            access.iter().all(|access| access != "ALL" && access != "index"),
            "the run scans the table: {access:?}"
        );
        admin.query_drop(named(DROP, "myplan")).await.unwrap();
    }
```

If this fails in CI, stop. Report the plan MySQL gave. The choice is then the user's: runs only for a key of one column on MySQL, or as they are.

- [ ] **Step 2: The run in the driver**

`crates/tabletist-db/src/mysql/write.rs`. `delete_one` is the loop's body at 866-894, moved; this driver's row is a `Deletion { lock, delete }` and its "is it still there" is the row's locking read once more:

```rust
async fn delete_one(
    conn: &mut mysql_async::Conn,
    deletion: &Deletion,
    place: usize,
    stop: &StopFlag,
) -> Result<Option<Applied>> {
    let deleted = match delete(conn, &deletion.delete, stop).await? {
        Ok(deleted) => deleted,
        Err(error) => {
            return Ok(Some(Applied::FailedDelete {
                delete: place,
                error,
            }));
        }
    };
    if deleted == 0 && rows(conn, &deletion.lock, stop).await?.1.is_empty() {
        return Ok(None);
    }
    Ok((deleted != 1).then(|| Applied::FailedDelete {
        delete: place,
        error: deleted_instead(deleted, 1),
    }))
}
```

`delete_many` follows PostgreSQL's shape. Its own points:

- the run's statement is `Dialect::MySql.delete_run(&changes.object, &rows)`'s `sql`, and it is no run where `driver_parameter(&sql.text).is_err()` (then the rows go one at a time, without a savepoint);
- the savepoint's statements go through `execute(conn, SAVEPOINT)`, which sends text, not a prepared statement;
- the run runs through `delete(conn, &sql, stop).await?`, which already tells a cancel and a lost session (the outer `Err`) from the statement's own failure (the inner one);
- a failed `ROLLBACK TO` is told apart: a cancel or a lost session ends the save (`Err`), anything else means the transaction is gone (a deadlock rolled it back, and its number is `busy` so the code is not reached for one, but another failure may do the same) and the run's error is `FailedDelete` for its first row.

```rust
async fn delete_many(
    conn: &mut mysql_async::Conn,
    changes: &ChangeSet,
    deletes: &[Deletion],
    live: &[usize],
    stop: &StopFlag,
) -> Result<Option<Applied>> {
    let first = live[0];
    let rows_of: Vec<&RowDelete> = live.iter().map(|&place| &changes.deletes[place]).collect();
    let run = Dialect::MySql
        .delete_run(&changes.object, &rows_of)
        .ok()
        .map(|run| run.sql)
        .filter(|sql| driver_parameter(&sql.text).is_ok());
    let deleted = match &run {
        None => None,
        Some(sql) => {
            not_stopped(stop)?;
            execute(conn, SAVEPOINT).await?;
            match delete(conn, sql, stop).await? {
                Ok(deleted) => Some(deleted),
                // Busy: sent again row by row, each row could wait as
                // long.
                Err(error) if Dialect::MySql.busy(&error) => {
                    return Ok(Some(Applied::FailedDelete {
                        delete: first,
                        error,
                    }));
                }
                Err(error) => match execute(conn, ROLLBACK_TO).await {
                    Ok(()) => None,
                    Err(lost @ (Error::Cancelled | Error::ConnectionLost(_))) => return Err(lost),
                    // No savepoint to come back to: the transaction is
                    // gone, and a row sent now would be written for good.
                    Err(_) => {
                        return Ok(Some(Applied::FailedDelete {
                            delete: first,
                            error,
                        }));
                    }
                },
            }
        }
    };
    let Some(deleted) = deleted else {
        for &place in live {
            let failed = delete_one(conn, &deletes[place], place, stop).await?;
            if failed.is_some() {
                return Ok(failed);
            }
        }
        if run.is_some() {
            execute(conn, RELEASE).await?;
        }
        return Ok(None);
    };
    if deleted > live.len() as u64 {
        return Ok(Some(Applied::FailedDelete {
            delete: first,
            error: deleted_instead(deleted, live.len()),
        }));
    }
    if deleted < live.len() as u64 {
        for &place in live {
            if !rows(conn, &deletes[place].lock, stop).await?.1.is_empty() {
                return Ok(Some(Applied::FailedDelete {
                    delete: place,
                    error: deleted_instead(0, 1),
                }));
            }
        }
    }
    execute(conn, RELEASE).await?;
    Ok(None)
}
```

The loop is the other two's, over `delete_runs(&changes.deletes, &built)`.

MySQL checks a foreign key row by row. `mysql_deletes_a_row_and_the_row_it_points_at_in_one_run` is the test of the fallback on a server: the run of Crime and Fiction is refused (1451), undone, and goes child first.

- [ ] **Step 3: Check what can be checked here**

Run: `~/.cargo/bin/cargo fmt --all`, then the four checks. Read the diff against PostgreSQL's, step for step.

- [ ] **Step 4: Commit**

```bash
git add crates/tabletist-db/src/mysql/write.rs crates/tabletist-db/tests/delete_rows.rs
git commit -m "Delete a save's rows in runs on MySQL

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 5: Review SQL shows the runs

**Files:**
- Modify: `src/review.rs` (`Line` at 66, the deletes at 295-313, `delete_statement` at 430, tests at 734)
- Modify: `src/ui/review.rs:118, 187` (and every other `match` on `Line` the compiler names)

- [ ] **Step 1: Write the failing tests**

`src/review.rs`, in its tests, after `a_delete_by_a_key_of_two_columns_is_laid_out_in_lines`:

```rust
    /// Spec 5 and test 14: a run of rows is one comment and one
    /// statement, and a row alone keeps its own.
    #[test]
    fn a_run_of_rows_to_delete_is_one_comment_and_one_statement() {
        let ids = |ids: &[i64]| ids.iter().map(|id| vec![("id", Value::Int(*id))]).collect();
        let changes = deleting(ids(&[9, 10, 11]));
        let review = of(
            Dialect::Postgres,
            &changes,
            Blocked::default(),
            Values::Shown,
        );
        assert_eq!(review.lines[0], Line::DeleteRun(3));
        assert_eq!(
            sql(&review),
            [r#"DELETE FROM "public"."book_covers" WHERE "id" IN (9, 10, 11);"#]
        );
        assert_eq!(review.deleted, 3);
        // Each value is a value of its own, to shorten and to colour.
        let Line::Sql(pieces) = &review.lines[1] else {
            panic!("a statement");
        };
        let values: Vec<&str> = pieces
            .iter()
            .filter(|piece| piece.ink == Ink::Value)
            .map(|piece| piece.text.as_str())
            .collect();
        assert_eq!(values, ["9", "10", "11"]);
        // 250 rows: three statements.
        let many: Vec<i64> = (1..=250).collect();
        let review = of(
            Dialect::Sqlite,
            &deleting(ids(&many)),
            Blocked::default(),
            Values::Shown,
        );
        let comments: Vec<&Line> = review
            .lines
            .iter()
            .filter(|line| matches!(line, Line::DeleteRun(_)))
            .collect();
        assert_eq!(
            comments,
            [&Line::DeleteRun(100), &Line::DeleteRun(100), &Line::DeleteRun(50)]
        );
        assert_eq!(sql(&review).len(), 3);
    }

    /// A key of several columns: the table, then the rows.
    #[test]
    fn a_run_by_a_key_of_two_columns_is_two_lines() {
        let row = |book: i64, author: &str| {
            vec![
                ("book_id", Value::Int(book)),
                ("author", Value::Text(author.into())),
            ]
        };
        let changes = deleting(vec![row(101, "Ada Quill"), row(102, "Ada Quill")]);
        let review = of(
            Dialect::Postgres,
            &changes,
            Blocked::default(),
            Values::Shown,
        );
        assert_eq!(
            sql(&review),
            [
                r#"DELETE FROM "public"."book_covers""#,
                r#" WHERE ("book_id", "author") IN ((101, 'Ada Quill'), (102, 'Ada Quill'));"#
            ]
        );
    }

    /// A row no save sends is refused by itself, between the runs of its
    /// neighbours.
    #[test]
    fn a_refused_row_stands_between_two_runs() {
        let fine = |id: &str| vec![("id", Value::Text(id.into()))];
        let changes = deleting(vec![
            fine("a"),
            fine("b"),
            fine("c\u{FFFD}"),
            fine("d"),
            fine("e"),
        ]);
        let review = of(Dialect::Sqlite, &changes, Blocked::default(), Values::Shown);
        let kinds: Vec<&str> = review
            .lines
            .iter()
            .map(|line| match line {
                Line::DeleteRun(_) => "run",
                Line::DeleteRefused { .. } => "refused",
                Line::Sql(_) => "sql",
                _ => "other",
            })
            .collect();
        assert_eq!(kinds, ["run", "sql", "refused", "run", "sql"]);
        assert!(matches!(review.refused, Some((Part::Delete(2), _))));
        assert_eq!(review.deleted, 5);
    }
```

(`Ink::Value` and a piece's `ink` are as `value(literal, values)` makes them: read `fn value` and the `Piece` type at the top of the file, and use their names.)

`src/ui/review.rs`, beside the test at 741:

```rust
        assert_eq!(
            read(Line::DeleteRun(3)).as_deref(),
            Some("-- delete 3 rows")
        );
```

- [ ] **Step 2: Run them to see them fail**

Run: `~/.cargo/bin/cargo test --locked --lib review::`
Expected: does not compile: `no variant named DeleteRun`.

- [ ] **Step 3: The line, the layout, the walk**

`src/review.rs`. In `enum Line`, after `Delete(String)`:

```rust
    /// `-- delete 3 rows`: the rows the one `DELETE` under it removes.
    DeleteRun(usize),
```

`src/ui/review.rs`: its words, beside `Line::Delete`'s (118), and `Line::DeleteRun(_)` wherever `Line::Delete(_)` is matched (187, and what the compiler names):

```rust
        Line::DeleteRun(rows) => format!("-- {} {rows} {}", say("delete"), say("rows")),
```

The deletes of `of` (295-313) are walked by run:

```rust
    // The rows to delete, last, as a save deletes them last, in the runs
    // a save sends them in. No cell holds one back: a marked row's cells
    // are not the set's. Each row's own statement first: a row no save
    // sends is refused by itself, and is in no run.
    let built: Vec<_> = changes
        .deletes
        .iter()
        .map(|row| dialect.delete_row(&changes.object, row))
        .collect();
    let sent: Vec<bool> = built.iter().map(Result::is_ok).collect();
    let mut built: Vec<_> = built.into_iter().map(Some).collect();
    for run in tabletist_db::delete_runs(&changes.deletes, &sent) {
        let rows: Vec<&tabletist_db::RowDelete> = changes.deletes[run.clone()].iter().collect();
        // Two rows or more, each built: one statement, where it can be
        // spelled. It always can; were it not, each row has its own.
        if rows.len() > 1
            && let Ok(deletion) = dialect.delete_run(&changes.object, &rows)
        {
            lines.push(Line::DeleteRun(rows.len()));
            lines.extend(delete_statement(&deletion, values));
            continue;
        }
        for index in run {
            let name = row_name(&changes.deletes[index].key);
            match built[index].take() {
                Some(Ok(deletion)) => {
                    lines.push(Line::Delete(name));
                    lines.extend(delete_statement(&deletion, values));
                }
                Some(Err(error)) => {
                    let reason = format::capped(&error.to_string()).into_owned();
                    lines.push(Line::DeleteRefused {
                        row: name,
                        reason: format::escape_hidden(&reason).into_owned(),
                    });
                    refused.get_or_insert((Part::Delete(index), error));
                }
                None => {}
            }
        }
    }
```

`delete_statement` tries a run's layout after one row's:

```rust
fn delete_statement(deletion: &RowDeletion, values: Values) -> Vec<Line> {
    lay_delete(deletion, values)
        .or_else(|| lay_run(deletion, values))
        .unwrap_or_else(|| {
            // Not the parts this was written for: the statement on one line.
            vec![Line::Sql(vec![
                plain(&deletion.shown, values),
                fixed(Ink::Plain, ";"),
            ])]
        })
}

/// A run's `DELETE`: on one line where its key is one column, as a row's
/// is, and on two where it is several, the table and then the rows. Each
/// value is a piece of its own, shortened and coloured as a value is.
/// `None` where the parts are not a run's: fewer values than two rows of
/// its columns, or not a whole number of rows.
fn lay_run(deletion: &RowDeletion, values: Values) -> Option<Vec<Line>> {
    let RowDeletion { shown, parts, .. } = deletion;
    let columns = parts.columns.len();
    if columns == 0 || parts.values.len() < 2 * columns || parts.values.len() % columns != 0 {
        return None;
    }
    let table = shown
        .get(..parts.filter)?
        .strip_prefix("DELETE FROM")?
        .trim_end();
    let mut head = vec![fixed(Ink::Keyword, "DELETE FROM"), plain(table, values)];
    let mut clause = vec![fixed(Ink::Plain, " "), fixed(Ink::Keyword, "WHERE")];
    let mut at = parts.filter + "WHERE".len();
    for range in &parts.values {
        clause.push(plain(shown.get(at..range.start)?, values));
        clause.push(value(shown.get(range.clone())?, values));
        at = range.end;
    }
    clause.push(plain(shown.get(at..)?, values));
    clause.push(fixed(Ink::Plain, ";"));
    if columns == 1 {
        head.append(&mut clause);
        return Some(vec![Line::Sql(head)]);
    }
    Some(vec![Line::Sql(head), Line::Sql(clause)])
}
```

`lay_delete` already gives up where the values are not one for each column, which a run's never are.

- [ ] **Step 4: Run the tests**

Run: `~/.cargo/bin/cargo fmt --all`, `~/.cargo/bin/cargo test --locked --lib review`, `~/.cargo/bin/cargo test --locked --lib delete_`, then the four checks and the shots' clippy.
Expected: PASS. Tests that mark two neighbouring rows and read the review's SQL now find one statement: where one fails, its expectation is the old shape, and the new one is the run. Change the expectation, not the code, and say so in the plan's status.

- [ ] **Step 5: Commit**

```bash
git add src/review.rs src/ui/review.rs
git commit -m "Show a save's deletes in Review SQL as the runs it sends

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

(With any test file whose expectation changed.)

---

### Task 6: Counts with many rows marked, and a short note

**Files:**
- Modify: `src/edit.rs` (`pointing_at`, its tests)
- Modify: `src/ui/pending_bar.rs:208` (`Note::AlreadyDeleted`)
- Test: `src/edit.rs`, `src/ui/delete_tests.rs`

- [ ] **Step 1: Write the failing tests**

`src/edit.rs`, in its tests, after `what_a_delete_takes_is_asked_once_for_each_key`:

```rust
    /// Spec 5 and test 17: a key counted with `IN` takes a thousand values
    /// to a count, and a key counted row by row is not counted for more
    /// than a hundred rows.
    #[test]
    fn counts_stay_bounded_with_many_rows_marked() {
        let structure = crate::testing::publishers_structure();
        let mut page = crate::testing::publishers_page();
        let row = page.rows[0].clone();
        page.rows = (0..1_500)
            .map(|id| {
                let mut row = row.clone();
                row[0] = Value::Int(id);
                row
            })
            .collect();
        let object = ObjectRef::new("main", "publishers");
        let mut edits = Edits::default();
        for row in 0..1_500 {
            edits.mark(row);
        }
        let asked = taken_by(&object, &marked_covers(&structure, &page, &edits));
        // The key of one column: two counts, of 1,000 values and of 500.
        let (_, queries, _) = &asked[0];
        let listed: Vec<usize> = queries
            .iter()
            .map(|query| query.filters[0].value.split(',').count())
            .collect();
        assert_eq!(listed, [1_000, 500]);
        assert!(queries.iter().all(|query| query.filters[0].op == FilterOp::In));
        // A key of two columns: one count a row up to a hundred rows, and
        // none above, which leaves its line without a number.
        let mut two = structure.clone();
        two.referenced_by[0].columns = vec!["publisher_id".into(), "kind".into()];
        two.referenced_by[0].ref_columns = vec!["id".into(), "name".into()];
        let mut edits = Edits::default();
        for row in 0..100 {
            edits.mark(row);
        }
        let asked = taken_by(&object, &marked_covers(&two, &page, &edits));
        assert_eq!(asked[0].1.len(), 100);
        edits.mark(100);
        let asked = taken_by(&object, &marked_covers(&two, &page, &edits));
        assert!(asked[0].1.is_empty());
        let taken = Taken::asked(vec![(asked[0].0.clone(), Vec::new(), 0)]);
        assert_eq!(taken.lines(), [asked[0].0.clone()]);
    }
```

(Read `publishers_structure` and `publishers_page` first: the test's two-column key names the page's first two columns. If they are not `id` and `name`, name the ones it has.)

`src/ui/delete_tests.rs`, in `a_row_already_deleted_is_said_by_its_key`, after its two-row case:

```rust
    // Up to three are named, and the rest counted.
    let names = |count: usize| (1..=count).map(|id| format!("id {id}")).collect();
    let three = crate::edit::Note::AlreadyDeleted(names(3));
    assert_eq!(
        crate::ui::pending_bar::note_text(&three, object, harness.app.locale),
        "3 rows were already deleted: id 1, id 2, id 3."
    );
    let five = crate::edit::Note::AlreadyDeleted(names(5));
    assert_eq!(
        crate::ui::pending_bar::note_text(&five, object, harness.app.locale),
        "5 rows were already deleted: id 1, id 2, id 3 and 2 more."
    );
```

- [ ] **Step 2: Run them to see them fail**

Run: `~/.cargo/bin/cargo test --locked --lib counts_stay_bounded`, `~/.cargo/bin/cargo test --locked --lib a_row_already_deleted_is_said_by_its_key`
Expected: FAIL: one count of 1,500 values where two were expected; `id 4, id 5.` where `and 2 more.` was.

- [ ] **Step 3: The counts**

`src/edit.rs`, above `pointing_at`:

```rust
/// How many values one count's `IN` lists, at most. More are counted in
/// parts and added: the rows that point at different rows are different
/// rows.
const MAX_IN_VALUES: usize = 1_000;

/// How many counts one key is asked, at most, where it is counted once
/// for each marked row. Above it the key's line stands without a number.
const MAX_ROW_COUNTS: usize = 100;
```

`pointing_at`'s doc gains: "A key counted row by row is not counted for more than [`MAX_ROW_COUNTS`] rows: `None`, as where the page does not hold what the key points at." Its last `match` becomes:

```rust
    Some(match key.columns.as_slice() {
        // One column and values `IN` can list: one count for each
        // thousand of them.
        [column] if values.len() > 1 && values.iter().all(listable) => values
            .chunks(MAX_IN_VALUES)
            .map(|part| {
                let list: Vec<&str> = part.iter().flatten().map(String::as_str).collect();
                count(vec![filter(column, FilterOp::In, list.join(","))])
            })
            .collect(),
        // A count for each row: too many to ask for.
        _ if values.len() > MAX_ROW_COUNTS => return None,
        columns => values
            .into_iter()
            .map(|row| {
                let filters = columns.iter().zip(row);
                count(
                    filters
                        .map(|(column, value)| filter(column, FilterOp::Eq, value))
                        .collect(),
                )
            })
            .collect(),
    })
```

`held_by` asks `pointing_at` of one row and is not touched by either bound.

- [ ] **Step 4: The note**

`src/ui/pending_bar.rs` (208):

```rust
        Note::AlreadyDeleted(rows) => match rows.as_slice() {
            [row] => format!("{} {row} {}", say("Row"), say("was already deleted.")),
            // Up to three by name; of more, three and how many more.
            rows => {
                let named = rows.len().min(3);
                let names = rows[..named].join(", ");
                let said = format!("{} {} {names}", rows.len(), say("rows were already deleted:"));
                match rows.len() - named {
                    0 => format!("{said}."),
                    more => format!("{said} {} {more} {}.", say("and"), say("more")),
                }
            }
        },
```

- [ ] **Step 5: Run the tests**

Run: `~/.cargo/bin/cargo fmt --all`, the two tests, then the four checks.
Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add src/edit.rs src/ui/pending_bar.rs src/ui/delete_tests.rs
git commit -m "Bound the counts of what many deletes take, and shorten the note

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 7: The guide, the specs, and the servers

**Files:**
- Modify: `docs/_guide/editing-data.md:273-277`
- Modify: `docs/superpowers/specs/2026-10-12-deleting-rows-design.md` ("As built", "Several rows"), `docs/superpowers/specs/2026-10-13-deleting-many-rows-design.md` (a status line under its title)
- Modify: this plan (its status)

- [ ] **Step 1: The guide**

`docs/_guide/editing-data.md`. `**Review SQL** shows each row's `DELETE` before you save.` becomes:

```markdown
**Review SQL** shows the `DELETE`s before you save. Rows next to each
other go in one statement, up to 100 rows each. If the database refuses
one of those, the save still names the row it could not delete.
```

- [ ] **Step 2: The specs and the plan**

- The first spec's "Several rows" entry ends: the `IN (…)` batches are built too, and with them test 7.
- The many-rows spec gains, under its title: `Status: built on 2026-10-13, in three slices` with the three plans' names.
- This plan gains its `Status:` line: the date, and each place a task left the plan as written.

- [ ] **Step 3: Check, commit, push**

Run: `cd docs && bin/check`, then the four checks and the shots' clippy on the branch's head.

```bash
git add docs/_guide/editing-data.md docs/superpowers
git commit -m "Say in the guide that a save deletes neighbouring rows together

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
git push
```

Check that the remote head is the commit.

- [ ] **Step 4: The servers**

The pull request's two integration jobs run `tests/delete_rows.rs` against PostgreSQL 17 and MySQL 8.4. Read their result once the run of this push has finished (`gh pr checks 120`, once; Auto-fix sends an event if a check fails). What to look for in each job's log: the new tests by name, `ok`, not `skipped`:

- `postgres_deletes_many_rows_in_runs`, `…_says_which_rows_of_a_run_were_gone`, `…_a_run_by_a_key_of_two_columns`, `…_a_row_and_the_row_it_points_at_in_one_run`;
- the same four for `mysql_`, and `mysql_finds_the_rows_of_a_run_by_their_key`.

A failure there is a failure of Task 3 or 4: fix it, with its test, and push again. If `mysql_finds_the_rows_of_a_run_by_their_key` fails, stop and report (R7).

- [ ] **Step 5: The pull request**

Bring the description of `igor-alexandrov/tabletist#120` up to date: the save's runs in its Summary, test 7 as done, R1 to R7, "Still to come" removed, and what CI said of the two servers.

---

## What this slice does not build

- Reading the rows to delete in batches (M12): each is still read with its own locking `SELECT`.
- Inserts and updates in batches.
- Everything the spec's "Out of scope" lists.
