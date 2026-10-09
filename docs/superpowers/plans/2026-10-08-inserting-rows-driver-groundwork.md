# Inserting Rows, Driver Groundwork Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A database error says what it is of (the constraint, the columns, and on MySQL the server's error number), and the structure of a MySQL or SQLite table lists the values a `CHECK (col IN (...))` allows, as PostgreSQL's does.

**Architecture:** `Error::Query` gains one field, `named: Named`, filled by each driver where it turns its own error into ours: PostgreSQL from the error's fields, MySQL and SQLite from the error's number and message. `crate::check` gains the two readers the other engines need (MySQL's stored clause, SQLite's `CREATE TABLE` text) in front of the parser PostgreSQL already uses, and `describe` fills `ColumnInfo::allowed_values` on all three. Nothing in the app reads `Named` yet: run 5 of the insert spec does. The list is read today, on every engine: a column with one draws its values as tags (`src/ui/value_tags.rs`), and a typed value that is not in it is refused before the save (`edit::check`, `Problem::NotOneOf`). So tasks 5 and 6 change what a MySQL or SQLite user sees, and the list must be read only where the database would agree with it (row e below).

**Tech Stack:** Rust 1.98, `tokio-postgres`, `mysql_async`, `rusqlite` (bundled SQLite). Spec: `docs/superpowers/specs/2026-10-07-inserting-rows-design.md` (section 9, the implementation notes, decisions A4, B10 and B11). Findings: INS-32a and INS-25a of the audit in pull request 108.

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

- `src/shots.rs` is outside those four and builds `Error::Query` by hand. Task 1 touches it, so task 1 also runs `~/.cargo/bin/cargo clippy --locked --features shots --lib --tests -- -D warnings`.
- **Databases.** Tasks 2, 3 and 5 need the servers. They are the ones of `compose.yaml`:

```bash
export TABLETIST_TEST_PG_URL=postgres://tabletist:tabletist@localhost:55432/tabletist
export TABLETIST_TEST_MYSQL_URL=mysql://tabletist:tabletist@localhost:53306/tabletist
```

  Without the variables those tests print "skipped" and pass, which proves nothing. If the servers cannot be reached, say so in the task's report and in the pull request.
- **What was run where this plan was written.** The four error cases were run on PostgreSQL 17.11, MySQL 8.4.11 and SQLite 3.50.2 for the audit, and MySQL's stored CHECK clauses were read from the live server: the messages and clauses quoted below are what those servers answered. The code blocks were written from reading the tree at `63df88f` and were not compiled. Where a block and the compiler disagree, the compiler is right and the task's tests say what must hold.
- **The branch.** A new one from `main` once pull request 108 is merged: the tests this plan takes out of `ignore` are in `crates/tabletist-db/tests/insert_audit.rs`, which that pull request adds. This plan is the branch's first commit.
- Run `~/.cargo/bin/cargo fmt --all` before each task's checks: some of the blocks below are wider than the formatter leaves them.
- House rules that bite here: no em dashes; comments explain why, in the surrounding code's voice; code that depends on the engine matches on `Dialect` and names every variant; `crates/tabletist-db` has no UI dependency; what a user typed never reaches a log; do not weaken a lint or delete a test to get green.
- Commits are signed, one per task, after its checks pass. If signing fails ("agent refused operation"), do not commit unsigned: stage the task and tell the user. Chain `git add` and `git commit` with `&&`. Each message ends with:

      Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>

## What the spec asks, and what gets built

**The user accepted every row on 2026-10-09**, row e's narrowing of decision B11 among them. Task 7 writes it into the spec.

| | The spec | What gets built | Why |
|---|---|---|---|
| a | "An error of the database carries … the column and the constraint it names" | What the database itself names, and no more. PostgreSQL names a unique or foreign key violation's constraint and not its columns; SQLite names a foreign key failure's nothing | Run 5 finds a constraint's columns in the structure it already holds. Reading them out of a message's prose here would depend on the server's language |
| b | MySQL's number "beside the SQLSTATE" | `Named::number`, for every MySQL server error. `code` stays the SQLSTATE | Three failures share the state 23000 |
| c | "The values a CHECK allows are read on MySQL and SQLite as they are on PostgreSQL" | A constraint that is nothing but `col IN ('a', 'b')`, as on PostgreSQL. A MySQL `ENUM`'s values are not read here | PostgreSQL's reader takes that form alone. An `ENUM` is a type, not a CHECK: it wants its own line in the spec |
| d | MySQL before 8.0.16, and MariaDB | A server without `information_schema.check_constraints` reads as a table with no lists. MariaDB is asked its own way (its list has the table's name, and its constraint names are a table's own), and its clause is parsed as it stands | Neither is on the test servers: the MariaDB path is written from its documentation and **not run**. MySQL's clause is told by its escaped quotes |
| e | A CHECK's list is read on MySQL and SQLite | Only for a text column whose comparison is exact. MySQL: a collation that ends `_bin`, `_cs` or `_cs_ks` (it tells case apart). SQLite: a table whose statement names no `COLLATE`. Elsewhere no list is read, as today | The app refuses a value that is not in the list letter for letter, before the save. Under MySQL's default collations `CHECK (kind IN ('print'))` takes `Print`, and so does a SQLite `COLLATE NOCASE` column: a list read there would refuse what the database takes. Accepted, knowing that on a MySQL database with default collations this reads no list at all. The other way is to read every list and have the app's check ignore case on MySQL, which is a change to `edit::check` and still misses accents. Even an exact collation pads: `utf8mb4_bin` takes `'print '` with a space after it, which the app's list refuses. That is left as it is |
| f | A user sees nothing new | On MySQL and SQLite, where a list is read, its values are drawn as tags and a typed value outside it is refused before the save, as on PostgreSQL | It is what the list does in the app today. The pull request says so |

## File map

| File | What changes |
|---|---|
| `crates/tabletist-db/src/error.rs` | `Named`, `Error::Query::named` (task 1) |
| `crates/tabletist-db/src/lib.rs` | exports `Named` (task 1) |
| every site that builds `Error::Query` field by field | `named: Named::default()` (task 1) |
| `crates/tabletist-db/src/pg.rs` | `query_error` fills `named` (task 2) |
| `crates/tabletist-db/src/mysql.rs` | `named`, `query_error` fills it (task 3); `describe` reads CHECK lists (task 5) |
| `crates/tabletist-db/src/sqlite.rs` | `named`, `map_error` fills it (task 4); `columns` reads CHECK lists (task 6) |
| `crates/tabletist-db/src/check.rs` | `mysql_allowed_values` (task 5), `in_create_table`, `names_a_collation` (task 6) |
| `crates/tabletist-db/src/mysql/write.rs` | a warning's error is named too (task 3) |
| `.github/workflows/ci.yml` | the two database jobs run `insert_audit` (task 7) |
| `crates/tabletist-db/tests/insert_audit.rs` | what each engine names; three tests out of `ignore` (tasks 2 to 6) |
| `docs/superpowers/specs/2026-10-07-inserting-rows-design.md` | its Delivery table, and where a list is read (task 7) |

---

### Task 1: `Named`, and a query error that carries it

**Files:**
- Modify: `crates/tabletist-db/src/error.rs`
- Modify: `crates/tabletist-db/src/lib.rs` (the `pub use error::` line)
- Modify: every site the compiler names (see step 4)

- [ ] **Step 1: Write the failing test.** In `crates/tabletist-db/src/error.rs`, inside `mod tests`:

```rust
    #[test]
    fn an_error_made_from_words_alone_names_nothing() {
        let Error::Query { named, .. } = Error::query("no such table") else {
            panic!("a query error");
        };
        assert_eq!(named, Named::default());
        assert_eq!((named.number, named.constraint), (None, None));
        assert!(named.columns.is_empty());
    }
```

  If `mod tests` does not import the parent's names, add `use super::Named;` beside its other imports.

- [ ] **Step 2: Run it to see it fail.**

Run: `~/.cargo/bin/cargo test --locked -p tabletist-db --lib error::tests::an_error_made_from_words_alone_names_nothing`
Expected: does not compile, "cannot find type `Named`".

- [ ] **Step 3: Add `Named` and the field.** In `crates/tabletist-db/src/error.rs`, above `pub enum Error`:

```rust
/// What the database names in an error beside its words: what a failed
/// cell is found by, and what its message is chosen by. Each part is there
/// only where the database itself gives it.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Named {
    /// MySQL's error number (1062, 1452): three of its constraint
    /// failures share one SQLSTATE. `None` on PostgreSQL and SQLite, whose
    /// `code` tells their failures apart.
    pub number: Option<u32>,
    /// The constraint that was broken, or the index of a unique one.
    pub constraint: Option<String>,
    /// The columns the database says the failure is of. Empty where it
    /// names a constraint alone, or nothing.
    pub columns: Vec<String>,
}
```

  In `Error::Query`, after `hint`:

```rust
        hint: Option<String>,
        /// What the error is of, where the database says.
        named: Named,
```

  In `Error::query`:

```rust
        Self::Query {
            code: None,
            message: message.into(),
            detail: None,
            hint: None,
            named: Named::default(),
        }
```

  In `crates/tabletist-db/src/lib.rs`, the line `pub use error::{Error, Result, SshStage};` becomes:

```rust
pub use error::{Error, Named, Result, SshStage};
```

- [ ] **Step 4: Give every site that builds a query error its `named`.** The compiler lists them, the database crate first: it stops there until that crate builds, so this takes two passes.

Run: `~/.cargo/bin/cargo check --locked -p tabletist-db --all-targets 2>&1 | grep -E '^error\[E0063\]' -A4 | grep -- '-->' | sort -u`
Expected: the sites of `crates/tabletist-db/src/{mysql.rs, mysql/write.rs, pg.rs, sqlite.rs}`, the two `connect_error` functions among them.

Run, once that passes: `~/.cargo/bin/cargo check --locked --workspace --all-targets 2>&1 | grep -E '^error\[E0063\]' -A4 | grep -- '-->' | sort -u`
Expected: the app's sites, in `src/testing.rs` and under `src/ui/` (`format.rs`, `insert_audit_tests.rs`, `mod.rs`, `sql_results.rs`). About twenty-six in all.

  At each (a literal with every field written out), add the field after `hint`. In the database crate:

```rust
                    hint: None,
                    named: Named::default(),
```

  with `Named` added to the file's `use crate::...` line. In the app, where the crate is not imported by name:

```rust
            hint: None,
            named: tabletist_db::Named::default(),
```

  Change no other line: tasks 2 to 4 are where a driver says anything. No pattern needs changing: every one that reads a query error ends with `..`.

  Then the same for the scenes, which the workspace's check does not build:

Run: `~/.cargo/bin/cargo clippy --locked --features shots --lib --tests -- -D warnings 2>&1 | grep -- '-->' | sort -u`
Expected: the six sites of `src/shots.rs`. Give each `named: tabletist_db::Named::default(),`.

- [ ] **Step 5: Run the four checks, and the shots' clippy.**

Expected: all pass. `error::tests::an_error_made_from_words_alone_names_nothing` passes. No test's count changes but that one.

- [ ] **Step 6: Commit.**

```bash
git add -A && git commit -m "Give a query error room for what the database names

An error carried a code, a message, a detail and a hint. A failed cell
is found by the column or the constraint the database names, and on
MySQL a failure is told from another by the server's number, which the
driver dropped for its SQLSTATE. Named holds the three, and every query
error has one: empty until the drivers fill it.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 2: PostgreSQL says the constraint and the column

**Files:**
- Modify: `crates/tabletist-db/src/pg.rs` (`query_error`)
- Test: `crates/tabletist-db/tests/insert_audit.rs`

- [ ] **Step 1: Write the failing test.** In `crates/tabletist-db/tests/insert_audit.rs`, add `Named` to the `use tabletist_db::{...}` list, and after `fn failure`:

```rust
/// What the database names in the failure of `changes`' save.
async fn names(connection: &Connection, changes: &ChangeSet) -> Named {
    match failure(save(connection, changes).await) {
        Error::Query { named, .. } => named,
        other => panic!("a query error: {other:?}"),
    }
}

fn of_constraint(name: &str) -> Named {
    Named {
        constraint: Some(name.into()),
        ..Named::default()
    }
}

fn of_column(name: &str) -> Named {
    Named {
        columns: vec![name.into()],
        ..Named::default()
    }
}
```

  In `postgres::the_bookshops_failures_as_postgres_hands_them_over`, before the comment "Nothing of the four saves is in the tables.":

```rust
        // What each failure is of, as PostgreSQL's error names it: the
        // constraint of a taken value, a missing parent and a refused
        // value, the column of a NULL.
        assert_eq!(
            names(&connection, &cases.taken_isbn()).await,
            of_constraint("audit_books_isbn_key")
        );
        assert_eq!(
            names(&connection, &cases.no_publisher()).await,
            of_constraint("audit_book_covers_publisher_id_fkey")
        );
        assert_eq!(
            names(&connection, &cases.null_publisher()).await,
            of_column("publisher_id")
        );
        assert_eq!(
            names(&connection, &cases.vinyl()).await,
            of_constraint("audit_book_covers_kind_check")
        );
```

- [ ] **Step 2: Run it to see it fail.**

Run: `~/.cargo/bin/cargo test --locked -p tabletist-db --test insert_audit postgres::the_bookshops_failures`
Expected: FAIL at the first of the four, `left: Named { number: None, constraint: None, columns: [] }`.

- [ ] **Step 3: Fill `named` from the error's fields.** In `crates/tabletist-db/src/pg.rs`, in `query_error`, the `Error::Query` it returns becomes:

```rust
        return Error::Query {
            code: Some(db.code().code().to_owned()),
            message: db.message().to_owned(),
            detail: db.detail().map(str::to_owned),
            hint: db.hint().map(str::to_owned),
            // The server's own fields, which do not change with the
            // language of its messages. It names a NOT NULL failure's
            // column, and the constraint of the others.
            named: Named {
                number: None,
                constraint: db.constraint().map(str::to_owned),
                columns: db.column().map(str::to_owned).into_iter().collect(),
            },
        };
```

  with `Named` on the file's `use crate::...` line.

- [ ] **Step 4: Correct the test's comment, which this makes untrue.** Above `the_bookshops_failures_as_postgres_hands_them_over`, the sentence "The app's `Error` has no column or constraint of its own, so a failed cell can be worded only from this text." becomes "The error names the constraint, and the column of a NULL: what run 5 finds a failed cell by."

- [ ] **Step 5: Run the test, then the four checks.**

Run: `~/.cargo/bin/cargo test --locked -p tabletist-db --test insert_audit postgres`
Expected: PASS, 3 passed and 1 ignored.

- [ ] **Step 6: Commit.**

```bash
git add -A && git commit -m "Say which constraint or column a PostgreSQL error is of

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 3: MySQL keeps its number, and says what a failure is of

MySQL names a failure only in its message. The parts read here are the ones the server quotes, by the error's number, never by the message's words: `lc_messages` translates those.

**Files:**
- Modify: `crates/tabletist-db/src/mysql.rs` (`query_error`, and `named` above it)
- Test: `crates/tabletist-db/src/mysql.rs` (unit), `crates/tabletist-db/tests/insert_audit.rs`

- [ ] **Step 1: Write the failing unit test.** In `crates/tabletist-db/src/mysql.rs`, in its `mod tests` (make one at the file's end, with `use super::*;`, if it has none):

```rust
    #[test]
    fn a_failure_is_named_from_its_number_and_what_the_message_quotes() {
        let of = |number: u32, message: &str| {
            let named = named(number, message);
            (named.number, named.constraint, named.columns)
        };
        // The messages are MySQL 8.4's, read from a live server.
        assert_eq!(
            of(
                1062,
                "Duplicate entry '978-1-4028-9462-6' for key 'audit_books.isbn'"
            ),
            (Some(1062), Some("isbn".into()), vec![])
        );
        // Before 8.0.19 the key came without its table.
        assert_eq!(
            of(1062, "Duplicate entry 'it''s' for key 'isbn'"),
            (Some(1062), Some("isbn".into()), vec![])
        );
        assert_eq!(
            of(
                1452,
                "Cannot add or update a child row: a foreign key constraint fails \
                 (`tabletist`.`audit_book_covers`, CONSTRAINT `audit_book_covers_ibfk_1` \
                 FOREIGN KEY (`publisher_id`) REFERENCES `audit_publishers` (`id`))"
            ),
            (
                Some(1452),
                Some("audit_book_covers_ibfk_1".into()),
                vec!["publisher_id".to_owned()]
            )
        );
        assert_eq!(
            of(1048, "Column 'publisher_id' cannot be null"),
            (Some(1048), None, vec!["publisher_id".to_owned()])
        );
        assert_eq!(
            of(1364, "Field 'publisher_id' doesn't have a default value"),
            (Some(1364), None, vec!["publisher_id".to_owned()])
        );
        assert_eq!(
            of(
                3819,
                "Check constraint 'audit_book_covers_kind_check' is violated."
            ),
            (Some(3819), Some("audit_book_covers_kind_check".into()), vec![])
        );
        // Any other error keeps its number and names nothing.
        assert_eq!(
            of(1146, "Table 'tabletist.nope' doesn't exist"),
            (Some(1146), None, vec![])
        );
    }
```

- [ ] **Step 2: Run it to see it fail.**

Run: `~/.cargo/bin/cargo test --locked -p tabletist-db --lib mysql::tests::a_failure_is_named`
Expected: does not compile, "cannot find function `named`".

- [ ] **Step 3: Write `named`, and use it.** In `crates/tabletist-db/src/mysql.rs`, above `query_error`:

```rust
/// What the server's error `number` with this `message` is of. MySQL says
/// it only in the message, so the parts it quotes are read by the number:
/// never by the message's words, which follow the server's language.
pub(crate) fn named(number: u32, message: &str) -> Named {
    // What stands between the first two of `quote` from `from` on.
    let quoted = |from: &str, quote: char| -> Option<String> {
        let rest = &from[from.find(quote)? + quote.len_utf8()..];
        Some(rest[..rest.find(quote)?].to_owned())
    };
    let (constraint, columns) = match number {
        // ER_DUP_ENTRY: "... for key 'table.key'", the key last. The
        // value comes before it and may hold a quote of its own.
        1062 => {
            let key = message.strip_suffix('\'').and_then(|rest| rest.rsplit('\'').next());
            let key = key.map(|key| key.rsplit('.').next().unwrap_or(key).to_owned());
            (key, Vec::new())
        }
        // ER_NO_REFERENCED_ROW_2: the constraint as it was defined, which
        // is SQL and not translated.
        1452 => {
            let constraint = message
                .split_once("CONSTRAINT ")
                .and_then(|(_, rest)| quoted(rest, '`'));
            let columns = message
                .split_once("FOREIGN KEY (")
                .and_then(|(_, rest)| rest.split_once(')'))
                .map(|(columns, _)| {
                    columns
                        .split(',')
                        .map(|column| column.trim().trim_matches('`').to_owned())
                        .collect()
                })
                .unwrap_or_default();
            (constraint, columns)
        }
        // ER_BAD_NULL_ERROR, ER_NO_DEFAULT_FOR_FIELD: the column, quoted.
        1048 | 1364 => (None, quoted(message, '\'').into_iter().collect()),
        // ER_CHECK_CONSTRAINT_VIOLATED: the constraint, quoted.
        3819 => (quoted(message, '\''), Vec::new()),
        _ => (None, Vec::new()),
    };
    Named {
        number: Some(number),
        constraint,
        columns,
    }
}
```

  In `query_error`, the server's arm:

```rust
        mysql_async::Error::Server(server) => Error::Query {
            named: named(u32::from(server.code), &server.message),
            code: Some(server.state),
            message: server.message,
            detail: None,
            hint: None,
        },
```

  (`named` is read before `server.message` is moved.) Add `Named` to the file's `use crate::...` line.

  A statement that only warns comes back another way. Outside strict mode a column without a default takes its type's zero with warning 1364, and the save makes an error of the warning: in `crates/tabletist-db/src/mysql/write.rs`, `warning` ends with an `Error::Query` whose `code` is the warning's number. Name it the same way:

```rust
    let (_level, code, message): (String, u32, String) = from_row(row)?;
    Ok(Error::Query {
        code: Some(code.to_string()),
        named: super::named(code, &message),
        message,
        detail: None,
        hint: None,
    })
```

  `connect_error` keeps an empty `Named`: a refused connection is of no column.

- [ ] **Step 4: Run the unit test.**

Run: `~/.cargo/bin/cargo test --locked -p tabletist-db --lib mysql::tests::a_failure_is_named`
Expected: PASS.

- [ ] **Step 5: Say it of the live server.** In `crates/tabletist-db/tests/insert_audit.rs`:

  Take `a_mysql_failure_keeps_its_error_number` out of `ignore`: delete its `#[ignore = "INS-32a: ..."]` line, and reword the first line of its doc comment from "INS-32a. Spec 9:" to "Spec 9:".

  In `mysql::the_bookshops_failures_as_mysql_hands_them_over`, before the first `assert_eq!(count(`:

```rust
        // What each failure is of, read from its number and its message.
        let numbered = |number: u32, named: Named| Named {
            number: Some(number),
            ..named
        };
        assert_eq!(
            names(&connection, &cases.taken_isbn()).await,
            numbered(1062, of_constraint("isbn"))
        );
        assert_eq!(
            names(&connection, &cases.no_publisher()).await,
            numbered(
                1452,
                Named {
                    columns: vec!["publisher_id".into()],
                    ..of_constraint("audit_book_covers_ibfk_1")
                }
            )
        );
        assert_eq!(
            names(&connection, &cases.null_publisher()).await,
            numbered(1048, of_column("publisher_id"))
        );
        assert_eq!(
            names(&connection, &cases.vinyl()).await,
            numbered(3819, of_constraint("audit_book_covers_kind_check"))
        );
```

- [ ] **Step 6: Run them, then the four checks.**

Run: `~/.cargo/bin/cargo test --locked -p tabletist-db --test insert_audit mysql`
Expected: PASS, 2 passed and 1 ignored (`mysql_reads_the_values_a_check_allows`, task 5's).

- [ ] **Step 7: Commit.**

```bash
git add -A && git commit -m "Keep MySQL's error number, and say what a failure is of

A taken value, a missing parent and a NULL all have the SQLSTATE 23000:
the number is what tells them apart, and the driver dropped it. It is
kept now, and the constraint and the columns are read from what the
message quotes, by the number and never by the message's words.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 4: SQLite says the column or the constraint

**Files:**
- Modify: `crates/tabletist-db/src/sqlite.rs` (`map_error`, and `named` above it)
- Test: `crates/tabletist-db/src/sqlite.rs` (unit), `crates/tabletist-db/tests/insert_audit.rs`

- [ ] **Step 1: Write the failing unit test.** In `crates/tabletist-db/src/sqlite.rs`, in its `mod tests`:

```rust
    #[test]
    fn a_failure_is_named_from_its_code_and_its_message() {
        let of = |code: i32, message: &str| {
            let named = named(code, message);
            (named.constraint, named.columns)
        };
        // SQLITE_CONSTRAINT_UNIQUE, of one column and of two.
        assert_eq!(
            of(2067, "UNIQUE constraint failed: books.isbn"),
            (None, vec!["isbn".to_owned()])
        );
        assert_eq!(
            of(2067, "UNIQUE constraint failed: books.title, books.format"),
            (None, vec!["title".to_owned(), "format".to_owned()])
        );
        // Of an index over an expression, which has no column to name.
        assert_eq!(
            of(2067, "UNIQUE constraint failed: index 'books_lower_isbn'"),
            (Some("books_lower_isbn".into()), vec![])
        );
        // SQLITE_CONSTRAINT_PRIMARYKEY and SQLITE_CONSTRAINT_NOTNULL.
        assert_eq!(
            of(1555, "UNIQUE constraint failed: books.id"),
            (None, vec!["id".to_owned()])
        );
        assert_eq!(
            of(1299, "NOT NULL constraint failed: book_covers.publisher_id"),
            (None, vec!["publisher_id".to_owned()])
        );
        // SQLITE_CONSTRAINT_CHECK names the constraint.
        assert_eq!(
            of(275, "CHECK constraint failed: book_covers_kind_check"),
            (Some("book_covers_kind_check".into()), vec![])
        );
        // SQLITE_CONSTRAINT_FOREIGNKEY names nothing, and no other error.
        assert_eq!(of(787, "FOREIGN KEY constraint failed"), (None, vec![]));
        assert_eq!(of(1, "no such table: nope"), (None, vec![]));
        assert_eq!(named(2067, "UNIQUE constraint failed: books.isbn").number, None);
    }
```

- [ ] **Step 2: Run it to see it fail.**

Run: `~/.cargo/bin/cargo test --locked -p tabletist-db --lib sqlite::tests::a_failure_is_named`
Expected: does not compile, "cannot find function `named`".

- [ ] **Step 3: Write `named`, and use it.** In `crates/tabletist-db/src/sqlite.rs`, above `map_error`:

```rust
/// What a failure with this extended result `code` and `message` is of.
/// SQLite's messages are its own English and no setting's, so what follows
/// the colon is read as it stands.
fn named(code: i32, message: &str) -> Named {
    let Some((_, what)) = message.split_once(": ") else {
        return Named::default();
    };
    // `table.column`, a column for each part.
    let columns = || -> Vec<String> {
        what.split(", ")
            .map(|part| part.rsplit('.').next().unwrap_or(part).to_owned())
            .collect()
    };
    match code {
        // SQLITE_CONSTRAINT_UNIQUE, _PRIMARYKEY and _ROWID. An index over
        // an expression is named in the place of its columns.
        2067 | 1555 | 2579 => match what.strip_prefix("index '") {
            Some(index) => Named {
                constraint: Some(index.trim_end_matches('\'').to_owned()),
                ..Named::default()
            },
            None => Named {
                columns: columns(),
                ..Named::default()
            },
        },
        // SQLITE_CONSTRAINT_NOTNULL.
        1299 => Named {
            columns: columns(),
            ..Named::default()
        },
        // SQLITE_CONSTRAINT_CHECK: the constraint's name, or its text
        // where it has none.
        275 => Named {
            constraint: Some(what.to_owned()),
            ..Named::default()
        },
        _ => Named::default(),
    }
}
```

  In `map_error`, the arm that builds `Error::Query`:

```rust
                _ => Error::Query {
                    code: Some(failure.extended_code.to_string()),
                    named: named(failure.extended_code, &message),
                    message,
                    detail: None,
                    hint: None,
                },
```

  Add `Named` to the file's `use crate::...` line.

- [ ] **Step 4: Run the unit test.**

Run: `~/.cargo/bin/cargo test --locked -p tabletist-db --lib sqlite::tests::a_failure_is_named`
Expected: PASS.

- [ ] **Step 5: Say it of a real database.** In `sqlite::the_bookshops_failures_as_sqlite_hands_them_over`, before `assert_eq!(count(&connection, "main", "books").await, 1);`:

```rust
        // What each failure is of. The foreign key's names nothing.
        assert_eq!(
            names(&connection, &cases.taken_isbn()).await,
            of_column("isbn")
        );
        assert_eq!(
            names(&connection, &cases.no_publisher()).await,
            Named::default()
        );
        assert_eq!(
            names(&connection, &cases.null_publisher()).await,
            of_column("publisher_id")
        );
        assert_eq!(
            names(&connection, &cases.vinyl()).await,
            of_constraint("book_covers_kind_check")
        );
```

- [ ] **Step 6: Run it, then the four checks.**

Run: `~/.cargo/bin/cargo test --locked -p tabletist-db --test insert_audit sqlite`
Expected: PASS, 2 passed and 1 ignored (task 6's).

- [ ] **Step 7: Commit.**

```bash
git add -A && git commit -m "Say which column or constraint a SQLite error is of

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 5: MySQL's CHECK lists

MySQL keeps a CHECK's condition in `information_schema.check_constraints`, written its own way and escaped once more. For `CHECK (kind IN ('print', 'e''book', 'au\\dio'))` the server holds:

```
(`kind` in (_utf8mb4\'print\',_utf8mb4\'e\\\'book\',_utf8mb4\'au\\\\dio\'))
```

So: every backslash doubled and every quote behind one; names in backticks; each string behind its character set; and inside a string, a quote and a backslash each behind a backslash.

**Files:**
- Modify: `crates/tabletist-db/src/check.rs`
- Modify: `crates/tabletist-db/src/mysql.rs` (`describe`)
- Test: `crates/tabletist-db/src/check.rs` (unit), `crates/tabletist-db/tests/insert_audit.rs`

- [ ] **Step 1: Write the failing unit test.** In `crates/tabletist-db/src/check.rs`, in `mod tests`:

```rust
    #[test]
    fn a_list_parses_as_mysql_keeps_it() {
        use super::mysql_allowed_values as values;
        // As `information_schema.check_constraints` holds it (MySQL 8.4).
        let kept = r"(`kind` in (_utf8mb4\'print\',_utf8mb4\'e\\\'book\',_utf8mb4\'au\\\\dio\'))";
        assert_eq!(values(kept, "kind"), list(&["print", "e'book", r"au\dio"]));
        // Another column's list is not this one's.
        assert_eq!(values(kept, "format"), None);
        // A name with a space, as it was declared.
        assert_eq!(
            values(r"(`odd name` in (_utf8mb4\'x\',_utf8mb4\'y\'))", "odd name"),
            list(&["x", "y"])
        );
        // Anything more than a plain list is not one.
        assert_eq!(values("(`n` > 0)", "n"), None);
        assert_eq!(
            values(
                r"((`kind` = _utf8mb4\'print\') or (`kind` = _utf8mb4\'ebook\'))",
                "kind"
            ),
            None
        );
        // An escape that stands for another character is no plain value.
        assert_eq!(values(r"(`kind` in (_utf8mb4\'a\\nb\'))", "kind"), None);
        // A clause that is not escaped (MariaDB's) is read as it stands.
        assert_eq!(values("`kind` in ('print','ebook')", "kind"), list(&["print", "ebook"]));
    }
```

- [ ] **Step 2: Run it to see it fail.**

Run: `~/.cargo/bin/cargo test --locked -p tabletist-db --lib check::tests::a_list_parses_as_mysql_keeps_it`
Expected: does not compile, "unresolved import `super::mysql_allowed_values`".

- [ ] **Step 3: Write the reader.** In `crates/tabletist-db/src/check.rs`, after `allowed_values`:

```rust
/// As [`allowed_values`], of a CHECK's condition as MySQL keeps it in
/// `information_schema.check_constraints`: escaped once more than it was
/// written, with its names in backticks and each string behind its
/// character set. It is written over into what the parser reads.
pub(crate) fn mysql_allowed_values(clause: &str, column: &str) -> Option<Vec<String>> {
    // MySQL's own escaping leaves no quote without a backslash before it.
    // A clause with a bare one is not in that form (MariaDB keeps it as
    // written) and is read as it stands.
    let mut before = ' ';
    let escaped = clause.chars().all(|character| {
        let bare = character == '\'' && before != '\\';
        before = character;
        !bare
    });
    let clause = if escaped {
        let mut plain = String::with_capacity(clause.len());
        let mut characters = clause.chars();
        while let Some(character) = characters.next() {
            match character {
                '\\' => plain.push(characters.next()?),
                other => plain.push(other),
            }
        }
        plain
    } else {
        clause.to_owned()
    };
    allowed_values(&from_mysql(&clause)?, column)
}

/// A MySQL condition in the parser's own writing: a name in double quotes,
/// a string with its quotes doubled and nothing before it. `None` for a
/// string with an escape that stands for another character (`\n`), which
/// is no plain value.
fn from_mysql(clause: &str) -> Option<String> {
    let mut written = String::with_capacity(clause.len());
    let mut rest = clause;
    while let Some(character) = rest.chars().next() {
        if character == '`' {
            // A name: a doubled backtick is one of its own.
            let mut name = String::new();
            let mut body = rest[1..].char_indices().peekable();
            let mut end = None;
            while let Some((at, letter)) = body.next() {
                if letter != '`' {
                    name.push(letter);
                } else if body.next_if(|(_, next)| *next == '`').is_some() {
                    name.push('`');
                } else {
                    end = Some(at + 2);
                    break;
                }
            }
            written.push('"');
            written.push_str(&name.replace('"', "\"\""));
            written.push('"');
            rest = &rest[end?..];
        } else if character == '\'' {
            // A string: `\'` and `\\` are the quote and the backslash.
            let mut value = String::new();
            let mut body = rest[1..].char_indices().peekable();
            let mut end = None;
            while let Some((at, letter)) = body.next() {
                match letter {
                    '\\' => match body.next()?.1 {
                        escaped @ ('\'' | '\\') => value.push(escaped),
                        _ => return None,
                    },
                    '\'' if body.next_if(|(_, next)| *next == '\'').is_some() => value.push('\''),
                    '\'' => {
                        end = Some(at + 2);
                        break;
                    }
                    other => value.push(other),
                }
            }
            written.push('\'');
            written.push_str(&value.replace('\'', "''"));
            written.push('\'');
            rest = &rest[end?..];
        } else if character == '_' && introduces(rest) {
            // A string's character set (`_utf8mb4'...'`): not part of it.
            rest = &rest[rest.find('\'')?..];
        } else {
            written.push(character);
            rest = &rest[character.len_utf8()..];
        }
    }
    Some(written)
}

/// Whether `rest` begins with a character set's name right before a
/// string: `_utf8mb4'`.
fn introduces(rest: &str) -> bool {
    let name = rest[1..]
        .find(|character: char| !character.is_ascii_alphanumeric())
        .map(|end| &rest[1..][end..]);
    name.is_some_and(|after| after.starts_with('\''))
}
```

  A `_` that begins a column's name (`_kind`) is never taken for a character set here: MySQL writes names in backticks, so a bare `_word'` can only be one.

- [ ] **Step 4: Run the unit test.**

Run: `~/.cargo/bin/cargo test --locked -p tabletist-db --lib check::tests`
Expected: PASS, the new test and the five that were there.

- [ ] **Step 5: Read the lists in `describe`, where the database would agree with them.** In `crates/tabletist-db/src/mysql.rs`, in `describe`:

  The columns' query gains each column's collation. `COALESCE(extra, '')` becomes `COALESCE(extra, ''), COALESCE(collation_name, '')`, and the tuple it is read into gains a seventh `String`:

```rust
        let columns: Vec<(String, String, String, Option<String>, String, String, String)> = self
```

  After the `if columns.is_empty() { ... }` block and before `let columns = columns.into_iter().map(`:

```rust
        // The conditions of the table's CHECK constraints that hold.
        // MariaDB is asked first, by the table's name: its constraint
        // names are a table's own, so MySQL's question below could answer
        // with another table's. MySQL has no such column there and
        // refuses, and is asked its own way. A server with no list at all
        // (MySQL before 8.0.16) has no CHECK that holds either.
        let of_table = self
            .catalog::<String>(
                "SELECT check_clause FROM information_schema.check_constraints \
                 WHERE constraint_schema = ? AND table_name = ? \
                 ORDER BY constraint_name",
                at,
            )
            .await;
        let checks: Vec<String> = match of_table {
            Ok(checks) => checks,
            Err(Error::Query { .. }) => match self
                .catalog(
                    "SELECT cc.check_clause \
                     FROM information_schema.table_constraints tc \
                     JOIN information_schema.check_constraints cc \
                       ON cc.constraint_schema = tc.constraint_schema \
                      AND cc.constraint_name = tc.constraint_name \
                     WHERE tc.table_schema = ? AND tc.table_name = ? \
                       AND tc.constraint_type = 'CHECK' AND tc.enforced = 'YES' \
                     ORDER BY cc.constraint_name",
                    at,
                )
                .await
            {
                Ok(checks) => checks,
                Err(Error::Query { .. }) => Vec::new(),
                Err(other) => return Err(other),
            },
            Err(other) => return Err(other),
        };
```

  The closure that builds each `ColumnInfo` takes the seventh field, and `allowed_values: None,` becomes a list where the column is text and compares exactly:

```rust
                |(name, type_name, nullable, default, comment, extra, collation)| {
                    // A list is the app's to hold a typed value to, letter
                    // for letter. MySQL's default collations take `Print`
                    // for `print`, so only a column that tells them apart
                    // has one here.
                    let text = matches!(
                        column_class(Dialect::MySql, &type_name),
                        ColumnClass::Text { .. }
                    );
                    // By its end: `_cs` inside a name is a language's
                    // tag (`utf8mb4_cs_0900_ai_ci` is Czech, and takes
                    // either case).
                    let exact = ["_bin", "_cs", "_cs_ks"]
                        .iter()
                        .any(|end| collation.ends_with(end));
                    let allowed_values = (text && exact)
                        .then(|| {
                            checks
                                .iter()
                                .find_map(|check| crate::check::mysql_allowed_values(check, &name))
                        })
                        .flatten();
                    ColumnInfo {
                        name,
                        type_name,
                        nullable: nullable == "YES",
                        default,
                        comment: (!comment.is_empty()).then_some(comment),
                        allowed_values,
```

  with the closure's other fields and its closing brace as they are. Import `column_class` and `ColumnClass` from the crate's root (`use crate::{ColumnClass, column_class};`): the file does not have them yet.

- [ ] **Step 6: Take the live test out of `ignore`.** In `crates/tabletist-db/tests/insert_audit.rs`, on `mysql_reads_the_values_a_check_allows`: delete its `#[ignore = "INS-25a: ..."]` line, and reword "INS-25a. Spec 5:" to "Spec 5:". Its Bookshop is the module's `CREATE`, whose `kind` is a `VARCHAR(64)` in the server's default collation, which does not tell case apart: the test would read no list. In `CREATE`, the `kind` column of `audit_book_covers` becomes ``kind VARCHAR(64) COLLATE utf8mb4_bin NOT NULL DEFAULT 'print',``. Then add, in the same test before its tables are dropped, the other half: a column that does not compare exactly has no list.

```rust
        // `format` of the books has the server's default collation, which
        // takes `Hardcover` for `hardcover`: no list is read for it.
        admin
            .query_drop(named(
                "ALTER TABLE audit_books ADD CONSTRAINT audit_books_format_check \
                 CHECK (format IN ('hardcover', 'paperback'))",
                "audit_check",
            ))
            .await
            .unwrap();
        let books = ObjectRef::new(cases.schema, cases.books.clone());
        let structure = connection.describe(&books).await.unwrap();
        let loose = column(&structure, "format").allowed_values.clone();
```

  and after the drop, beside the assertion that is there: `assert_eq!(loose, None);`. (`cases.covers` is moved into `covers` above it: build `books` before that line, or clone as here.)

- [ ] **Step 7: Run it, then the four checks.**

Run: `~/.cargo/bin/cargo test --locked -p tabletist-db --test insert_audit mysql`
Expected: PASS, 3 passed and none ignored. The clause quoted at this task's head was read from a column in the server's default collation. If a `utf8mb4_bin` column's clause is kept another way and the list comes back `None`, print the clause the server holds and make `mysql_allowed_values` read that form too, with a unit test of it.

- [ ] **Step 8: Commit.**

```bash
git add -A && git commit -m "Read the values a MySQL CHECK allows into the structure

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 6: SQLite's CHECK lists

SQLite keeps no list of a table's CHECK constraints: they are in the `CREATE TABLE` it was made with, which `sqlite_master` holds as it was written.

**Files:**
- Modify: `crates/tabletist-db/src/check.rs`
- Modify: `crates/tabletist-db/src/sqlite.rs` (`columns`)
- Test: `crates/tabletist-db/src/check.rs` (unit), `crates/tabletist-db/tests/insert_audit.rs`

- [ ] **Step 1: Write the failing unit test.** In `crates/tabletist-db/src/check.rs`, in `mod tests`:

```rust
    #[test]
    fn the_checks_of_a_create_table_are_found_where_they_are_code() {
        use super::in_create_table as checks;
        let sql = "CREATE TABLE book_covers (
            id INTEGER PRIMARY KEY,
            kind TEXT NOT NULL DEFAULT 'print'
                CONSTRAINT book_covers_kind_check CHECK (kind IN ('print', 'ebook')),
            note TEXT DEFAULT 'CHECK (note IN (''a''))', -- CHECK (id IN ('x'))
            /* CHECK (id IN ('y')) */
            \"check\" TEXT,
            n INTEGER check(n > (1 + 1)),
            CHECK ((kind) IN ('print', 'ebook'))
        )";
        assert_eq!(
            checks(sql),
            [
                "(kind IN ('print', 'ebook'))",
                "(n > (1 + 1))",
                "((kind) IN ('print', 'ebook'))",
            ]
        );
        // What each gives the parser.
        let lists: Vec<_> = checks(sql)
            .iter()
            .map(|check| values(check, "kind"))
            .collect();
        assert_eq!(lists, [list(&["print", "ebook"]), None, list(&["print", "ebook"])]);
        // A table with none, and a text that ends inside one.
        assert!(checks("CREATE TABLE t (a TEXT)").is_empty());
        assert!(checks("CREATE TABLE t (a TEXT CHECK (a IN ('x'").is_empty());
        // Whether the statement names a collation, where that is code.
        use super::names_a_collation as collates;
        assert!(!collates(sql));
        assert!(collates("CREATE TABLE t (a TEXT COLLATE NOCASE CHECK (a IN ('x')))"));
        assert!(collates("CREATE TABLE t (a TEXT collate nocase)"));
        assert!(!collates("CREATE TABLE t (a TEXT DEFAULT 'COLLATE', \"collate\" TEXT)"));
        assert!(!collates("CREATE TABLE collated (a TEXT) -- COLLATE NOCASE"));
    }
```

- [ ] **Step 2: Run it to see it fail.**

Run: `~/.cargo/bin/cargo test --locked -p tabletist-db --lib check::tests::the_checks_of_a_create_table`
Expected: does not compile, "unresolved import `super::in_create_table`".

- [ ] **Step 3: Write the reader.** In `crates/tabletist-db/src/check.rs`, after `mysql_allowed_values`:

```rust
/// The condition of each `CHECK (...)` in a `CREATE TABLE` statement, with
/// its parentheses, in the order they stand. SQLite keeps no list of them:
/// the statement, as it was written, is all there is. A `CHECK` inside a
/// string, a quoted name or a comment is none.
pub(crate) fn in_create_table(sql: &str) -> Vec<&str> {
    let mut checks = Vec::new();
    let mut at = 0;
    // Whether the letters before `at` are part of the same word.
    let mut in_word = false;
    while at < sql.len() {
        let rest = &sql[at..];
        if let Some(skipped) = skip(rest) {
            at += skipped;
            in_word = false;
            continue;
        }
        let head = rest.get(..5).filter(|head| head.eq_ignore_ascii_case("check"));
        if let Some(head) = head.filter(|_| !in_word) {
            let after = rest[head.len()..].trim_start();
            if after.starts_with('(') {
                let open = sql.len() - after.len();
                let Some(length) = closed(after) else {
                    return checks;
                };
                checks.push(&sql[open..open + length]);
                at = open + length;
                in_word = false;
                continue;
            }
        }
        let character = rest.chars().next().unwrap_or(' ');
        in_word = is_ident_char(character);
        at += character.len_utf8();
    }
    checks
}

/// Whether a `CREATE TABLE` statement names a collation for anything, as
/// code and not in a string, a quoted name or a comment.
pub(crate) fn names_a_collation(sql: &str) -> bool {
    let mut at = 0;
    let mut in_word = false;
    while at < sql.len() {
        let rest = &sql[at..];
        if let Some(skipped) = skip(rest) {
            at += skipped;
            in_word = false;
            continue;
        }
        let word = rest.get(..7).filter(|head| head.eq_ignore_ascii_case("collate"));
        if word.is_some() && !in_word && !rest[7..].starts_with(is_ident_char) {
            return true;
        }
        let character = rest.chars().next().unwrap_or(' ');
        in_word = is_ident_char(character);
        at += character.len_utf8();
    }
    false
}

/// How long the string, the quoted name or the comment that `rest` begins
/// with is. `None` where it begins with none.
fn skip(rest: &str) -> Option<usize> {
    let quote = match rest.chars().next()? {
        '\'' => '\'',
        '"' => '"',
        '`' => '`',
        '[' => ']',
        '-' if rest.starts_with("--") => {
            return Some(rest.find('\n').map_or(rest.len(), |end| end + 1));
        }
        '/' if rest.starts_with("/*") => {
            return Some(rest[2..].find("*/").map_or(rest.len(), |end| end + 4));
        }
        _ => return None,
    };
    // To the closing quote: a doubled one is the quote itself, but in a
    // bracketed name, which has no way to hold its bracket.
    let mut body = rest[1..].char_indices().peekable();
    while let Some((index, character)) = body.next() {
        if character != quote {
            continue;
        }
        if quote != ']' && body.next_if(|(_, next)| *next == quote).is_some() {
            continue;
        }
        return Some(index + 1 + quote.len_utf8());
    }
    Some(rest.len())
}

/// How long the parenthesised text that `rest` begins with is, its closing
/// parenthesis counted. `None` where it never closes.
fn closed(rest: &str) -> Option<usize> {
    let mut depth = 0usize;
    let mut at = 0;
    while at < rest.len() {
        if let Some(skipped) = skip(&rest[at..]) {
            at += skipped;
            continue;
        }
        let character = rest[at..].chars().next()?;
        at += character.len_utf8();
        match character {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    return Some(at);
                }
            }
            _ => {}
        }
    }
    None
}
```

- [ ] **Step 4: Run the unit tests.**

Run: `~/.cargo/bin/cargo test --locked -p tabletist-db --lib check::tests`
Expected: PASS, seven tests (the five that were there, task 5's and this one).

- [ ] **Step 5: Read the lists in `columns`.** In `crates/tabletist-db/src/sqlite.rs`, in `fn columns`, before `let mut statement = connection.prepare(`:

```rust
    // The table's CHECK conditions, from the statement it was made with:
    // SQLite keeps them nowhere else. A table with no statement (a virtual
    // one's shadow, an internal one) has none.
    let made: Option<String> = connection
        .query_row(
            &format!(
                "SELECT sql FROM {}.sqlite_master WHERE type = 'table' AND name = ?1",
                Dialect::Sqlite.quote_ident(&object.schema)
            ),
            [&object.name],
            // Lossy, as every name of this file is read: a statement can
            // hold bytes that are no UTF-8, and a table with such a name
            // is still described.
            |row| optional_text(row, 0),
        )
        .optional()
        .map_err(map_error)?
        .flatten();
    // A list is the app's to hold a typed value to, letter for letter. A
    // column that compares otherwise (`COLLATE NOCASE`) takes what the
    // list would refuse, and SQLite does not say which column that is: a
    // table whose statement names a collation anywhere has no lists.
    let made = made.filter(|sql| !crate::check::names_a_collation(sql));
    let checks = made.as_deref().map(crate::check::in_create_table);
    let checks = checks.unwrap_or_default();
    // SQLite matches a name without regard to its case. The parser folds
    // a bare one to lower case and keeps a quoted one as written.
    let allowed = |name: &str| {
        checks.iter().find_map(|check| {
            crate::check::allowed_values(check, name)
                .or_else(|| crate::check::allowed_values(check, &name.to_lowercase()))
        })
    };
```

  and in the `query_map` closure, bind the name first and use it:

```rust
            let name = text(row, 0)?;
            // A text column's alone, as on PostgreSQL.
            let texts = matches!(
                column_class(Dialect::Sqlite, &type_name),
                ColumnClass::Text { .. }
            );
            Ok((
                ColumnInfo {
                    allowed_values: texts.then(|| allowed(&name)).flatten(),
                    name,
                    nullable: row.get::<_, i64>(2)? == 0,
```

  (replacing `name: text(row, 0)?,` and `allowed_values: None,`; the other fields stay, and `type_name` is bound above them already). `Dialect` and `OptionalExtension` are imported in `sqlite.rs`; `column_class` and `ColumnClass` are not: add `use crate::{ColumnClass, column_class};`. The audit's Bookshop declares `kind TEXT`, which is a text column, and names no collation. The statement can be NULL (an internal table's), which is why the row is read as an `Option<String>` and flattened. `columns` is also what a page's fetch and its count ask for binary columns by, so a statement that cannot be read must never be an error here: `tests/sqlite.rs` has two tables with names that are not UTF-8 (`latin1_names`) that hold this.

- [ ] **Step 6: Take the live test out of `ignore`.** In `crates/tabletist-db/tests/insert_audit.rs`, on `sqlite_reads_the_values_a_check_allows`: delete its `#[ignore = "INS-25a: ..."]` line, and reword "INS-25a. Spec 5:" to "Spec 5:".

- [ ] **Step 7: Run it, then the four checks.**

Run: `~/.cargo/bin/cargo test --locked -p tabletist-db --test insert_audit`
Expected: PASS, 9 passed and 1 ignored (`postgres_is_sent_rows_that_set_the_same_columns_together`, which is run 4's).

- [ ] **Step 8: Commit.**

```bash
git add -A && git commit -m "Read the values a SQLite CHECK allows into the structure

SQLite keeps no list of a table's CHECK constraints. They are read out
of the statement the table was made with, where they are code and not
a string, a quoted name or a comment, and given to the parser that reads
PostgreSQL's.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 7: Run the audit's tests in CI, and say that this is built

**Files:**
- Modify: `.github/workflows/ci.yml` (the `postgres` and `mysql` jobs)
- Modify: `docs/superpowers/specs/2026-10-07-inserting-rows-design.md` (section 3, the implementation notes, and the table under "Delivery")

- [ ] **Step 1: Have the database jobs run the audit's tests.** They run one test file each, so the PostgreSQL and MySQL tests of `insert_audit.rs` print "skipped" in CI. In `.github/workflows/ci.yml`:

```yaml
      - run: cargo test --locked -p tabletist-db --lib --test postgres --test insert_audit
```

```yaml
      - run: cargo test --locked -p tabletist-db --lib --test mysql --test insert_audit
```

  each in its own job, with the `env` that is under it. (Each job then runs the other engine's tests of that file too, which skip there.)

- [ ] **Step 2: Say in the spec where a list is read.** In section 3, the sentence "A CHECK's list of allowed values is read on all three engines, not on PostgreSQL alone." becomes:

```markdown
A CHECK's list of allowed values is read on all three engines, for a text column whose comparison is exact: the app holds a typed value to the list letter for letter. On MySQL that is a collation that ends `_bin`, `_cs` or `_cs_ks`; on SQLite a table whose statement names no `COLLATE`.
```

  In the implementation notes, "The values a CHECK allows are read on MySQL and SQLite as they are on PostgreSQL." gains the same condition: append " for a text column that compares exactly (section 3)".

- [ ] **Step 3: Add the Delivery row and trim the two it takes from.** In the Delivery table, after the "Fixes" row, add:

```markdown
| Groundwork | In the drivers, for runs 4 and 5: an error carries the constraint, the columns and MySQL's number, and a CHECK's list is read on MySQL and SQLite (`docs/superpowers/plans/2026-10-08-inserting-rows-driver-groundwork.md`) | built |
```

  In run 4's row, drop ", and CHECK lists read on MySQL and SQLite". In run 5's row, drop "the column, the constraint and MySQL's number in an error, ".

- [ ] **Step 4: Run the four checks.** Expected: all pass. `docs/_guide/browsing-data.md` says the grid "draws enum, CHECK and boolean values as colored tags" and names no engine: it is true of MySQL and SQLite now, where a list is read, and needs no change.

- [ ] **Step 5: Commit.**

```bash
git add -A && git commit -m "Run the insert audit's tests in CI, and say the groundwork is built

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

## What this leaves for the runs

- Run 5 reads `Named`: it puts a failure on the cell of `named.columns`, or on the columns the structure gives for `named.constraint` (an index's, a foreign key's), and chooses its message by `code` on PostgreSQL and SQLite and by `named.number` on MySQL. A failure that names neither marks the row and no cell.
- Run 4's paste preview and run 2b's inspector read `allowed_values` on every engine.
- The pull request's description lists the three tests taken out of `ignore`, says which servers the tests ran against, that the MariaDB path was not run, and what a MySQL or SQLite user now sees: where a list is read, its values are tags and a typed value outside it is refused before the save.
