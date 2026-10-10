# Foreign Key Editing, Run 1: The Lookup Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** `tabletist-db` can read the rows of a table a foreign key points at, by a text and by a key, on PostgreSQL, MySQL and SQLite; the backend runs that read as a command; and SQLite's structure names the column an implicit reference means.

**Architecture:** A new `lookup.rs` holds what a search asks (`Lookup`) and answers (`LookupPage`). `Dialect::lookup` writes its one statement beside `select_rows`, from the pieces the filters already use. Each adapter runs it the way it runs a page of rows (a read-only transaction, `query_only` on SQLite) and hands the rows to one shared mapper. The backend gets `Command::Lookup` and `Event::Lookup`, shaped like `FetchRows` and `Rows`. Nothing in the app asks for a lookup yet: run 2 does.

**Tech Stack:** Rust 1.98, `tokio-postgres`, `mysql_async`, `rusqlite` (bundled SQLite). Spec: `docs/superpowers/specs/2026-10-10-foreign-key-editing-design.md` (sections 1 and 3, the implementation notes, tests 1 to 9).

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

- **Databases.** Task 3 needs the servers of `compose.yaml`:

```bash
docker compose up -d --build --wait postgres mysql
export TABLETIST_TEST_PG_URL=postgres://tabletist:tabletist@localhost:55432/tabletist
export TABLETIST_TEST_MYSQL_URL=mysql://tabletist:tabletist@localhost:53306/tabletist
```

  Without the variables those tests print "skipped" and pass, which proves nothing. If the servers cannot be reached, say so in the task's report and in the pull request.
- **What was run where this plan was written.** Nothing. The code blocks were written from reading the tree at `6e04567` and were not compiled, and the statements were not run on a server. Where a block and the compiler disagree, the compiler is right: keep the block's behaviour and its tests, and fix its spelling. Where a server refuses a statement of task 1, fix the statement in `Dialect::lookup` and its unit test together, and say so in the task's report.
- Run `~/.cargo/bin/cargo fmt --all` before each task's checks: some of the blocks below are wider than the formatter leaves them.
- House rules that bite here: no em dashes; comments explain why, in the surrounding code's voice; code that depends on the engine matches on `Dialect` and names every variant (no `==`, no `matches!`, no `_` arm); `crates/tabletist-db` has no UI dependency; what a user typed never reaches a log; do not weaken a lint or delete a test to get green.
- Commits are signed, one per task, after its checks pass. If signing fails ("agent refused operation", "1Password: agent returned an error"), do not commit unsigned: stage the task and tell the user. Chain `git add` and `git commit` with `&&`. Each message ends with:

      Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>

## What the spec asks, and what gets built

| | The spec | What gets built | Why |
|---|---|---|---|
| a | `LookupPart::Pinned(String)` | `LookupPart::Pinned(Value)` | MySQL compares a `BIGINT` with a string as floats: `9100000000000000004` would equal `9100000000000000007`. A whole number has to be sent as one. `Dialect::key_operand` already turns a loaded key into the operand that finds its row, and a save uses it. Run 2 makes the `Value` from the typed text |
| b | "A search is two reads" | One `Lookup` is one read: its `part` says which. The app sends two commands | The first's answer shows before the second's (section 3) |
| c | The matches are "the first rows the database finds", ordered among themselves | A derived table with the limit and `OFFSET 0`, ordered outside it by how each row matched, then by key | No statement sorts a whole table. Without the offset SQLite folds the derived table into the outer query and sorts before it limits (checked in `sqlite3` 3.51 by the plan's review) |
| d | "The database says of each row how it matched" | A third column, a `CASE` that answers 0, 1 or 2 | The app's own search of a label is only for the bold |
| e | An empty text: "the table's first rows by key" | `ORDER BY` the key with the limit, no derived table | The key's index gives them |
| f | SQLite's implicit reference | `describe` fills `ref_columns` with the parent's primary key when it has as many columns as the key. A parent that is not there, or has no primary key, leaves it empty | SQLite itself refuses such a key when a row is written |
| g | Test 9: "and the Structure view names it" | No change to the view. `src/ui/structure.rs` already writes `table (columns)` when `ref_columns` is not empty | The test is on `describe`, where the change is |
| h | "A search's text and its rows are not logged" | `Lookup` and `LookupPage` print without them (`Debug` by hand), as `Command::RunSql` prints without its statements | A `{command:?}` in a log line would otherwise carry a name or an email |

## File map

| File | What changes |
|---|---|
| `crates/tabletist-db/src/lookup.rs` | New. `Lookup`, `LookupPart`, `Matched`, `LookupRow`, `LookupPage`, the row mapper (task 1) |
| `crates/tabletist-db/src/dialect.rs` | `Dialect::lookup`, `Dialect::matches`; the contains filter uses `matches` (task 1) |
| `crates/tabletist-db/src/lib.rs` | `mod lookup`, the exports (task 1), `Connection::lookup` (task 2) |
| `crates/tabletist-db/src/adapter.rs` | `Adapter::lookup` (task 2) |
| `crates/tabletist-db/src/sqlite.rs` | `lookup` (task 2), the implicit reference (task 4) |
| `crates/tabletist-db/src/pg.rs` | `Conn::read`, `fetch_rows` on it, `lookup` (task 2) |
| `crates/tabletist-db/src/mysql.rs` | `lookup` (task 2) |
| `crates/tabletist-db/src/catalog.rs` | What `ForeignKeyInfo::ref_columns` says (task 4) |
| `crates/tabletist-db/tests/lookup.rs` | New. Tests 5 to 9 on the three engines (tasks 2, 3, 4) |
| `.github/workflows/ci.yml` | The PostgreSQL and MySQL jobs run `--test lookup` (task 3) |
| `src/backend.rs` | `Command::Lookup`, `Event::Lookup` (task 5) |
| `src/app.rs` | An arm that drops `Event::Lookup` until run 2 (task 5) |
| `docs/superpowers/specs/2026-10-10-foreign-key-editing-design.md` | Its status, and `Pinned(Value)` (task 6) |

---

### Task 1: What a lookup asks, and its statement

**Files:**
- Create: `crates/tabletist-db/src/lookup.rs`
- Modify: `crates/tabletist-db/src/lib.rs` (the `mod` list near line 11, the exports near line 44)
- Modify: `crates/tabletist-db/src/dialect.rs` (the `use` at line 5, `filter` near line 354, a new method after `count_rows` near line 870, the tests module)

- [ ] **Step 1: Create `crates/tabletist-db/src/lookup.rs` with the types**

```rust
//! Looking a row up in the table a foreign key points at: by a text that
//! is in its label or begins its key, or by its key alone.

use std::fmt;

use crate::{ObjectRef, Value};

/// One read of the table a foreign key points at. A search of the app is
/// two of these: the pinned row, then the matches.
#[derive(Clone, PartialEq)]
pub struct Lookup {
    pub object: ObjectRef,
    /// The column the foreign key references.
    pub key: String,
    /// The column a person reads a row by. `None`: the table has no text
    /// column but its key, and rows are found by the key alone.
    pub label: Option<String>,
    pub part: LookupPart,
    /// The most rows wanted. One more is read, to learn whether there are
    /// more.
    pub limit: u32,
}

/// Which rows a [`Lookup`] reads.
#[derive(Clone, PartialEq)]
pub enum LookupPart {
    /// The row whose key is this, compared with `=`. The caller sends a
    /// whole number for a key that is one: MySQL compares a number with a
    /// string as floats, and two large keys are then the same.
    Pinned(Value),
    /// The rows whose label contains the text, compared as a contains
    /// filter compares, or whose key as text begins with it. An empty
    /// text: the table's first rows by key.
    Matching(String),
}

/// How a row of a [`LookupPage`] was found.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Matched {
    /// Its key is the value, or the text.
    Key,
    /// Its label contains the text.
    Label,
    /// Its key begins with the text and its label does not hold it. Every
    /// row of an empty text's read.
    KeyStart,
}

impl Matched {
    /// What the statement's last column answers for this.
    pub(crate) fn code(self) -> i64 {
        match self {
            Self::Key => 0,
            Self::Label => 1,
            Self::KeyStart => 2,
        }
    }
}

#[derive(Clone, PartialEq)]
pub struct LookupRow {
    pub key: Value,
    /// `None` when the lookup named no label column.
    pub label: Option<Value>,
    pub matched: Matched,
}

#[derive(Clone, PartialEq)]
pub struct LookupPage {
    pub rows: Vec<LookupRow>,
    /// There are more rows than `limit`.
    pub more: bool,
}

// What was typed, and what a table holds, stay out of a log line: a label
// can be a name or an email.
impl fmt::Debug for Lookup {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let part = match &self.part {
            LookupPart::Pinned(_) => "Pinned",
            LookupPart::Matching(_) => "Matching",
        };
        f.debug_struct("Lookup")
            .field("object", &self.object)
            .field("key", &self.key)
            .field("label", &self.label)
            .field("part", &part)
            .field("limit", &self.limit)
            .finish()
    }
}

impl fmt::Debug for LookupPage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LookupPage")
            .field("rows", &self.rows.len())
            .field("more", &self.more)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lookup(label: Option<&str>, limit: u32) -> Lookup {
        Lookup {
            object: ObjectRef::new("main", "publishers"),
            key: "id".into(),
            label: label.map(str::to_owned),
            part: LookupPart::Matching("Ada Lovelace".into()),
            limit,
        }
    }

    #[test]
    fn a_lookup_prints_without_what_was_typed_or_read() {
        let printed = format!("{:?}", lookup(Some("name"), 50));
        assert!(!printed.contains("Ada"), "{printed}");
        assert!(printed.contains("Matching"), "{printed}");
        let page = LookupPage {
            rows: vec![LookupRow {
                key: Value::Int(1),
                label: Some(Value::Text("Ada Lovelace".into())),
                matched: Matched::Label,
            }],
            more: false,
        };
        assert!(!format!("{page:?}").contains("Ada"));
    }
}
```

`LookupRow` has no `Debug` on purpose: nothing prints a row. If a test's `assert_eq!` needs one, compare the row's fields.

What turns a driver's rows into a `LookupPage` is not in this file yet: nothing would call it before task 2, and clippy refuses code nothing calls.

- [ ] **Step 2: Wire the module in `crates/tabletist-db/src/lib.rs`**

In the `mod` list, after `mod error;` and `pub mod fixtures;` (alphabetical order), add:

```rust
mod lookup;
```

After the `pub use error::...;` line add:

```rust
pub use lookup::{Lookup, LookupPage, LookupPart, LookupRow, Matched};
```

- [ ] **Step 3: Write the failing statement tests**

In `crates/tabletist-db/src/dialect.rs`, inside the existing `mod tests` at the end of the file, add (the module already has `use super::*;`; add the three names of `crate::` it does not have yet to that module's imports):

```rust
    use crate::{Lookup, LookupPart, Matched};

    fn looking(label: Option<&str>, part: LookupPart) -> Lookup {
        Lookup {
            object: ObjectRef::new("main", "publishers"),
            key: "id".into(),
            label: label.map(str::to_owned),
            part,
            limit: 50,
        }
    }

    fn texts(sql: &Sql) -> Vec<&str> {
        sql.params
            .iter()
            .map(|value| match value {
                Value::Text(text) => &**text,
                other => panic!("a bound value is {other:?}"),
            })
            .collect()
    }

    #[test]
    fn a_lookup_reads_the_matches_inside_the_limit_and_orders_outside_it() {
        let sql = Dialect::Sqlite.lookup(&looking(
            Some("name"),
            LookupPart::Matching("harb".into()),
        ));
        assert_eq!(
            sql.text,
            "SELECT \"id\", \"name\", \
             CASE WHEN CAST(\"id\" AS TEXT) = ? THEN 0 \
             WHEN CAST(\"name\" AS TEXT) LIKE ? ESCAPE '\\' THEN 1 ELSE 2 END \
             FROM (SELECT \"id\", \"name\" FROM \"main\".\"publishers\" \
             WHERE CAST(\"name\" AS TEXT) LIKE ? ESCAPE '\\' \
             OR CAST(\"id\" AS TEXT) LIKE ? ESCAPE '\\' LIMIT 51 OFFSET 0) AS found \
             ORDER BY 3, 1"
        );
        // In the order the statement names them: these bind by position.
        assert_eq!(texts(&sql), ["harb", "%harb%", "%harb%", "harb%"]);
    }

    #[test]
    fn a_lookup_compares_as_the_contains_filter_does_on_each_engine() {
        let lookup = looking(Some("name"), LookupPart::Matching("harb".into()));
        let pg = Dialect::Postgres.lookup(&lookup);
        assert!(pg.params.is_empty());
        assert!(
            pg.text.contains(r#"CAST("name" AS TEXT) ILIKE E'%harb%'"#),
            "{}",
            pg.text
        );
        assert!(
            pg.text.contains(r#"CAST("id" AS TEXT) ILIKE E'harb%'"#),
            "{}",
            pg.text
        );
        assert!(
            pg.text.contains(r#"FROM "main"."publishers" WHERE"#),
            "{}",
            pg.text
        );
        let my = Dialect::MySql.lookup(&lookup);
        assert!(
            my.text.contains("CAST(`name` AS CHAR) LIKE ?"),
            "{}",
            my.text
        );
        assert!(my.text.contains("LIMIT 51 OFFSET 0) AS found ORDER BY 3, 1"), "{}", my.text);
        assert_eq!(texts(&my), ["harb", "%harb%", "%harb%", "harb%"]);
    }

    #[test]
    fn a_lookup_escapes_the_wildcards_of_what_was_typed() {
        let sql = Dialect::Sqlite.lookup(&looking(
            Some("name"),
            LookupPart::Matching("50%_".into()),
        ));
        assert_eq!(texts(&sql), ["50%_", r"%50\%\_%", r"%50\%\_%", r"50\%\_%"]);
    }

    #[test]
    fn a_lookup_of_a_table_with_no_label_goes_by_the_key_alone() {
        let sql = Dialect::Sqlite.lookup(&looking(None, LookupPart::Matching("12".into())));
        assert_eq!(
            sql.text,
            "SELECT \"id\", CASE WHEN CAST(\"id\" AS TEXT) = ? THEN 0 ELSE 2 END \
             FROM (SELECT \"id\" FROM \"main\".\"publishers\" \
             WHERE CAST(\"id\" AS TEXT) LIKE ? ESCAPE '\\' LIMIT 51 OFFSET 0) AS found \
             ORDER BY 2, 1"
        );
        assert_eq!(texts(&sql), ["12", "12%"]);
    }

    #[test]
    fn a_lookup_for_nothing_reads_the_first_rows_by_key() {
        let sql = Dialect::MySql.lookup(&looking(Some("name"), LookupPart::Matching(String::new())));
        assert_eq!(
            sql.text,
            "SELECT `id`, `name`, 2 FROM `main`.`publishers` ORDER BY 1 LIMIT 51"
        );
        assert!(sql.params.is_empty());
    }

    #[test]
    fn a_pinned_lookup_is_one_equality_on_the_key() {
        let pinned = |value: Value| looking(Some("name"), LookupPart::Pinned(value));
        let number = Value::Int(9_100_000_000_000_000_004);
        // A whole number goes as one: MySQL would compare a string with a
        // BIGINT as floats.
        let my = Dialect::MySql.lookup(&pinned(number.clone()));
        assert_eq!(
            my.text,
            "SELECT `id`, `name`, 0 FROM `main`.`publishers` WHERE `id` = ? LIMIT 51"
        );
        assert_eq!(my.params, [number.clone()]);
        // PostgreSQL has no parameters here: the value is a literal.
        let pg = Dialect::Postgres.lookup(&pinned(number));
        assert_eq!(
            pg.text,
            "SELECT \"id\", \"name\", 0 FROM \"main\".\"publishers\" \
             WHERE \"id\" = 9100000000000000004 LIMIT 51"
        );
        let text = Dialect::Postgres.lookup(&pinned(Value::Text("it's".into())));
        assert!(text.text.contains(r#"WHERE "id" = 'it''s' LIMIT 51"#), "{}", text.text);
    }

    #[test]
    fn the_codes_of_a_match_are_the_ones_the_statement_writes() {
        assert_eq!(
            [Matched::Key.code(), Matched::Label.code(), Matched::KeyStart.code()],
            [0, 1, 2]
        );
    }
```

- [ ] **Step 4: Run them to see them fail**

Run: `~/.cargo/bin/cargo test --locked -p tabletist-db --lib lookup`
Expected: a compile error, `no method named lookup found for enum Dialect`.

- [ ] **Step 5: Write `Dialect::lookup`**

In `crates/tabletist-db/src/dialect.rs`, add `Lookup, LookupPart, Matched` to the `use crate::{...}` list at the top. Then add these two methods to `impl Dialect`, `matches` beside `like_escape` and `lookup` after `count_rows`:

```rust
    /// `column`, as text, like `pattern`: what a contains filter writes,
    /// with this engine's sense of case.
    fn matches(self, column: &str, pattern: String, params: &mut Vec<Value>) -> String {
        let placeholder = self.bind(params, pattern);
        format!(
            "{} {} {placeholder}{}",
            self.as_text(column),
            self.like(),
            self.like_escape()
        )
    }
```

```rust
    /// The one statement of a [`Lookup`]. Its rows are the key, the label
    /// where the lookup names one, and how the row matched
    /// (`Matched::code`).
    ///
    /// The matches are read inside a derived table that holds the limit,
    /// so the database stops at the first rows it finds, and they are
    /// ordered outside it: no table is sorted whole. The columns are
    /// ordered by their place, since a column may be named as the `CASE`
    /// would be.
    pub fn lookup(self, lookup: &Lookup) -> Sql {
        let table = self.qualified(&lookup.object);
        let key = self.quote_ident(&lookup.key);
        let label = lookup.label.as_deref().map(|label| self.quote_ident(label));
        let columns = match &label {
            Some(label) => format!("{key}, {label}"),
            None => key.clone(),
        };
        let limit = u64::from(lookup.limit) + 1;
        let mut params = Vec::new();
        let text = match &lookup.part {
            LookupPart::Pinned(value) => {
                let operand = self.sent(&self.key_operand(value), &mut params);
                format!(
                    "SELECT {columns}, {} FROM {table} WHERE {key} = {operand} LIMIT {limit}",
                    Matched::Key.code()
                )
            }
            LookupPart::Matching(text) if text.is_empty() => format!(
                "SELECT {columns}, {} FROM {table} ORDER BY 1 LIMIT {limit}",
                Matched::KeyStart.code()
            ),
            LookupPart::Matching(text) => {
                let escaped = escape_like(text);
                // Each value is bound where the statement names it: MySQL
                // and SQLite bind by position. The key is compared as
                // text on purpose: SQLite binds a text that reads as a
                // number as one, and only the cast makes `91` equal it.
                let is_key = format!(
                    "{} = {}",
                    self.as_text(&key),
                    self.bind(&mut params, text.clone())
                );
                let how = match &label {
                    Some(label) => format!(
                        "CASE WHEN {is_key} THEN {} WHEN {} THEN {} ELSE {} END",
                        Matched::Key.code(),
                        self.matches(label, format!("%{escaped}%"), &mut params),
                        Matched::Label.code(),
                        Matched::KeyStart.code()
                    ),
                    None => format!(
                        "CASE WHEN {is_key} THEN {} ELSE {} END",
                        Matched::Key.code(),
                        Matched::KeyStart.code()
                    ),
                };
                let mut found = Vec::with_capacity(2);
                if let Some(label) = &label {
                    found.push(self.matches(label, format!("%{escaped}%"), &mut params));
                }
                found.push(self.matches(&key, format!("{escaped}%"), &mut params));
                let order = if label.is_some() { "3, 1" } else { "2, 1" };
                // `OFFSET 0` is for SQLite: without it the derived table
                // is folded into the outer query, and the whole table is
                // scanned and sorted before the limit.
                format!(
                    "SELECT {columns}, {how} FROM (SELECT {columns} FROM {table} WHERE {} \
                     LIMIT {limit} OFFSET 0) AS found ORDER BY {order}",
                    found.join(" OR ")
                )
            }
        };
        Sql { text, params }
    }
```

Then make the contains filter use `matches`, so the two cannot drift apart. In `Dialect::filter`, replace the body of the `FilterOp::Contains | FilterOp::StartsWith` arm:

```rust
            FilterOp::Contains | FilterOp::StartsWith => {
                let escaped = escape_like(&filter.value);
                let pattern = if filter.op == FilterOp::Contains {
                    format!("%{escaped}%")
                } else {
                    format!("{escaped}%")
                };
                self.matches(&column, pattern, params)
            }
```

- [ ] **Step 6: Run the tests**

Run: `~/.cargo/bin/cargo test --locked -p tabletist-db --lib`
Expected: PASS, the new tests and every filter test that was there (`contains_casts_to_text_and_is_case_insensitive_per_dialect` among them: it proves the refactor changed no text).

- [ ] **Step 7: Run the four checks, then commit**

```bash
git add crates/tabletist-db/src/lookup.rs crates/tabletist-db/src/lib.rs crates/tabletist-db/src/dialect.rs && git commit -m "Write the statement that looks a referenced row up

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 2: The three adapters run it, tested on SQLite

The adapter trait has no defaults, so the three adapters get `lookup` in one task. This task tests SQLite, which needs no server. Task 3 tests the other two.

**Files:**
- Modify: `crates/tabletist-db/src/adapter.rs`
- Modify: `crates/tabletist-db/src/lib.rs` (after `count_rows`, near line 281)
- Modify: `crates/tabletist-db/src/sqlite.rs` (the `impl Adapter`, after `count_rows` near line 963)
- Modify: `crates/tabletist-db/src/pg.rs` (`fetch_rows` near line 689, and a new `Conn::read`)
- Modify: `crates/tabletist-db/src/mysql.rs` (the `impl Adapter`, after `count_rows` near line 438)
- Create: `crates/tabletist-db/tests/lookup.rs`

- [ ] **Step 1: Write the failing SQLite tests**

Create `crates/tabletist-db/tests/lookup.rs`:

```rust
//! Looking up the row a foreign key points at, on a Bookshop made in each
//! engine (`docs/superpowers/specs/2026-10-10-foreign-key-editing-design.md`,
//! tests 5 to 9).
//!
//! PostgreSQL and MySQL need their servers (see compose.yaml). Without the
//! variables those tests print "skipped" and pass.

// `allow-unwrap-in-tests` does not cover helpers in an integration-test crate.
#![allow(clippy::unwrap_used)]

use tabletist_db::{
    Access, ConnectSpec, Connection, HostKeys, Lookup, LookupPage, LookupPart, Matched, ObjectRef,
    Secrets, Value,
};

const NORTHLIGHT: i64 = 9_100_000_000_000_000_001;
const HARBOR_PRESS: i64 = 9_100_000_000_000_000_004;
const HARBOR_LIGHTS: i64 = 9_100_000_000_000_000_007;
/// No publisher has this id.
const NO_PUBLISHER: i64 = 9_100_000_000_000_000_099;

/// A read of `schema.table` by its `id` and `name`.
fn publishers(schema: &str, table: &str, part: LookupPart, limit: u32) -> Lookup {
    Lookup {
        object: ObjectRef::new(schema, table),
        key: "id".into(),
        label: Some("name".into()),
        part,
        limit,
    }
}

fn matching(text: &str) -> LookupPart {
    LookupPart::Matching(text.into())
}

/// A page's rows as (key, label, how it matched).
fn found(page: &LookupPage) -> Vec<(i64, String, Matched)> {
    page.rows
        .iter()
        .map(|row| {
            let key = match &row.key {
                Value::Int(key) => *key,
                other => panic!("a key is {other:?}"),
            };
            let label = match &row.label {
                Some(Value::Text(label)) => label.to_string(),
                other => panic!("a label is {other:?}"),
            };
            (key, label, row.matched)
        })
        .collect()
}

/// What every engine answers for the Bookshop's publishers, which are the
/// five of each module's `CREATE`.
async fn the_bookshop_is_searched(connection: &Connection, schema: &str, table: &str) {
    let read = |part: LookupPart, limit: u32| {
        let lookup = publishers(schema, table, part, limit);
        async move { connection.lookup(&lookup).await.unwrap() }
    };

    // Test 5: a label is found whatever its case, and its rows go by key.
    let page = read(matching("harb"), 50).await;
    assert!(!page.more);
    assert_eq!(
        found(&page),
        [
            (HARBOR_PRESS, "Harbor Press".to_owned(), Matched::Label),
            (HARBOR_LIGHTS, "Harbor Lights".to_owned(), Matched::Label),
        ]
    );

    // Test 6: the pinned row, and no row for a key nothing has.
    let page = read(LookupPart::Pinned(Value::Int(HARBOR_PRESS)), 1).await;
    assert_eq!(
        found(&page),
        [(HARBOR_PRESS, "Harbor Press".to_owned(), Matched::Key)]
    );
    let page = read(LookupPart::Pinned(Value::Int(NO_PUBLISHER)), 1).await;
    assert!(page.rows.is_empty() && !page.more);
    // Two keys that are one number as floats are still two rows.
    let page = read(LookupPart::Pinned(Value::Int(HARBOR_LIGHTS)), 1).await;
    assert_eq!(found(&page)[0].1, "Harbor Lights");

    // Test 7: the rows whose label holds the text come before the rows
    // whose key only begins with it.
    let page = read(matching("91"), 50).await;
    assert_eq!(
        found(&page),
        [
            (5, "Studio 91".to_owned(), Matched::Label),
            (NORTHLIGHT, "Northlight Books".to_owned(), Matched::KeyStart),
            (HARBOR_PRESS, "Harbor Press".to_owned(), Matched::KeyStart),
            (HARBOR_LIGHTS, "Harbor Lights".to_owned(), Matched::KeyStart),
        ]
    );

    // A text that is a key says so.
    let page = read(matching("5"), 50).await;
    assert_eq!(found(&page), [(5, "Studio 91".to_owned(), Matched::Key)]);

    // Nothing typed: the first rows by key, and whether there are more.
    let page = read(matching(""), 2).await;
    assert!(page.more);
    assert_eq!(
        found(&page).iter().map(|row| row.0).collect::<Vec<_>>(),
        [5, 6]
    );

    // A wildcard typed is a character to find.
    let page = read(matching("0%"), 50).await;
    assert_eq!(found(&page), [(6, "100% Books".to_owned(), Matched::Label)]);

    // A quote typed ends no string.
    let page = read(matching("'; DROP TABLE x; --"), 50).await;
    assert!(page.rows.is_empty());
    let page = read(matching(""), 50).await;
    assert_eq!(page.rows.len(), 5);
}

mod sqlite {
    use super::*;

    const CREATE: &str = "
        CREATE TABLE publishers (
            id INTEGER PRIMARY KEY,
            name TEXT NOT NULL
        );
        INSERT INTO publishers VALUES
            (5, 'Studio 91'),
            (6, '100% Books'),
            (9100000000000000001, 'Northlight Books'),
            (9100000000000000004, 'Harbor Press'),
            (9100000000000000007, 'Harbor Lights');
        CREATE TABLE shelves (id INTEGER PRIMARY KEY, floor INTEGER);
        INSERT INTO shelves VALUES (12, 1), (120, 1), (3, 2);";

    /// A database of its own, made with `sql`.
    async fn made_with(sql: &str, access: Access) -> (Connection, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("bookshop.db");
        rusqlite::Connection::open(&path)
            .unwrap()
            .execute_batch(sql)
            .unwrap();
        let connection = Connection::connect_with(
            &ConnectSpec::sqlite(&path),
            &Secrets::default(),
            &HostKeys::default(),
            access,
        )
        .await
        .unwrap();
        (connection, dir)
    }

    #[tokio::test]
    async fn the_bookshop_is_searched_on_sqlite() {
        // Test 8: on a session that cannot write.
        let (connection, _dir) = made_with(CREATE, Access::ReadOnly).await;
        the_bookshop_is_searched(&connection, "main", "publishers").await;
        let (connection, _dir) = made_with(CREATE, Access::Writable).await;
        the_bookshop_is_searched(&connection, "main", "publishers").await;
    }

    #[tokio::test]
    async fn a_table_with_no_label_is_searched_by_its_key() {
        let (connection, _dir) = made_with(CREATE, Access::ReadOnly).await;
        let page = connection
            .lookup(&Lookup {
                object: ObjectRef::new("main", "shelves"),
                key: "id".into(),
                label: None,
                part: matching("12"),
                limit: 50,
            })
            .await
            .unwrap();
        let rows: Vec<_> = page
            .rows
            .iter()
            .map(|row| (row.key.clone(), row.label.clone(), row.matched))
            .collect();
        assert_eq!(
            rows,
            [
                (Value::Int(12), None, Matched::Key),
                (Value::Int(120), None, Matched::KeyStart),
            ]
        );
    }

    /// The limit holds inside the derived table: the database stops at
    /// the first rows it finds, and does not sort the table for the best
    /// ones. Row 3's key is the text, and it is not among the first two
    /// found.
    #[tokio::test]
    async fn the_matches_are_the_first_found_not_the_best_of_the_table() {
        const ROWS: &str = "
            CREATE TABLE publishers (id INTEGER PRIMARY KEY, name TEXT NOT NULL);
            INSERT INTO publishers VALUES (1, 'x3'), (2, 'y3'), (3, 'z');";
        let (connection, _dir) = made_with(ROWS, Access::ReadOnly).await;
        let page = connection
            .lookup(&publishers("main", "publishers", matching("3"), 1))
            .await
            .unwrap();
        assert!(page.more);
        assert_eq!(found(&page), [(1, "x3".to_owned(), Matched::Label)]);
    }

    #[tokio::test]
    async fn a_table_that_is_not_there_is_the_databases_error() {
        let (connection, _dir) = made_with(CREATE, Access::ReadOnly).await;
        let error = connection
            .lookup(&publishers("main", "nowhere", matching("a"), 50))
            .await
            .unwrap_err();
        assert!(error.to_string().contains("nowhere"), "{error}");
    }
}
```

`rusqlite` and `tempfile` are already what `tests/insert_audit.rs` uses, so `Cargo.toml` does not change. If the compiler says `rusqlite` is not a dependency of the test crate, look at how `tests/insert_audit.rs` reaches it and do the same.

- [ ] **Step 2: Run them to see them fail**

Run: `~/.cargo/bin/cargo test --locked -p tabletist-db --test lookup`
Expected: a compile error, `no method named lookup found for struct Connection`.

- [ ] **Step 3: The rows a driver read, as a page**

In `crates/tabletist-db/src/lookup.rs`, change the imports to `use crate::{Error, ObjectRef, Result, Value};`, add to `impl Matched`:

```rust
    fn from_cell(cell: &Value) -> Self {
        match cell {
            Value::Int(0) => Self::Key,
            Value::Int(1) => Self::Label,
            _ => Self::KeyStart,
        }
    }
```

add before the tests module:

```rust
/// The rows a driver read for `lookup`, at most one more than its limit,
/// as its page. Each row is the key, the label where the lookup names one,
/// and how the row matched.
pub(crate) fn page(mut rows: Vec<Vec<Value>>, lookup: &Lookup) -> Result<LookupPage> {
    let limit = lookup.limit as usize;
    let more = rows.len() > limit;
    rows.truncate(limit);
    let width = if lookup.label.is_some() { 3 } else { 2 };
    let rows = rows
        .into_iter()
        .map(|row| {
            if row.len() != width {
                return Err(Error::query("the lookup returned a row of another shape"));
            }
            let mut cells = row.into_iter();
            let key = cells.next().unwrap_or(Value::Null);
            let label = lookup.label.as_ref().and_then(|_| cells.next());
            let matched = Matched::from_cell(&cells.next().unwrap_or(Value::Null));
            Ok(LookupRow {
                key,
                label,
                matched,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(LookupPage { rows, more })
}
```

and add to the tests module:

```rust
    #[test]
    fn a_page_keeps_the_limit_and_says_there_are_more() {
        let rows = vec![
            vec![Value::Int(4), Value::Text("Harbor Press".into()), Value::Int(1)],
            vec![Value::Int(7), Value::Text("Harbor Lights".into()), Value::Int(1)],
            vec![Value::Int(40), Value::Null, Value::Int(2)],
        ];
        let page = page(rows, &lookup(Some("name"), 2)).unwrap();
        assert!(page.more);
        assert_eq!(page.rows.len(), 2);
        assert_eq!(page.rows[0].key, Value::Int(4));
        assert_eq!(page.rows[0].label, Some(Value::Text("Harbor Press".into())));
        assert_eq!(page.rows[0].matched, Matched::Label);
    }

    #[test]
    fn a_page_without_a_label_has_two_cells_to_a_row() {
        let rows = vec![vec![Value::Int(4), Value::Int(0)]];
        let page = page(rows, &lookup(None, 50)).unwrap();
        assert!(!page.more);
        assert_eq!(page.rows[0].label, None);
        assert_eq!(page.rows[0].matched, Matched::Key);
    }

    #[test]
    fn a_row_of_another_shape_is_an_error() {
        let rows = vec![vec![Value::Int(4)]];
        assert!(page(rows, &lookup(Some("name"), 50)).is_err());
    }
```

- [ ] **Step 4: Add `lookup` to the adapter trait**

In `crates/tabletist-db/src/adapter.rs`, add `Lookup, LookupPage` to the `use crate::{...}` list, and after `count_rows` in the trait:

```rust
    async fn lookup(&self, lookup: &Lookup) -> Result<LookupPage>;
```

- [ ] **Step 5: `Connection::lookup`**

In `crates/tabletist-db/src/lib.rs`, after `Connection::count_rows`:

```rust
    /// Rows of the table a foreign key points at: the one a key names, or
    /// the ones a text is found in (see [`Lookup`]). It reads as a page of
    /// rows does and cannot write: in a read-only transaction on
    /// PostgreSQL and MySQL, under `query_only` on SQLite. What was typed
    /// is a bound value or an escaped literal, never part of the
    /// statement.
    pub async fn lookup(&self, lookup: &Lookup) -> Result<LookupPage> {
        match &self.inner {
            Inner::Sqlite(conn) => conn.lookup(lookup).await,
            Inner::Postgres(conn) => conn.lookup(lookup).await,
            Inner::MySql(conn) => conn.lookup(lookup).await,
        }
    }
```

Also add one sentence to the crate's top comment in `lib.rs`, where it lists what stays fenced ("row fetches and counts run in read-only transactions"): make it "row fetches, counts and lookups run in read-only transactions".

- [ ] **Step 6: SQLite**

In `crates/tabletist-db/src/sqlite.rs`, add `Lookup, LookupPage` to the file's `use crate::{...}` list, and in `impl Adapter for Conn`, after `count_rows`:

```rust
    async fn lookup(&self, lookup: &Lookup) -> Result<LookupPage> {
        let lookup = lookup.clone();
        // No fence: the statement is the app's own, and what was typed is
        // bound.
        self.browse(move |connection| {
            let sql = Dialect::Sqlite.lookup(&lookup);
            let most = lookup.limit as usize + 1;
            let mut statement = connection.prepare(&sql.text).map_err(map_error)?;
            let width = statement.column_count();
            let mut rows = statement
                .query(rusqlite::params_from_iter(sql.params.iter().map(to_sqlite)))
                .map_err(map_error)?;
            let mut values: Vec<Vec<Value>> = Vec::new();
            while values.len() < most
                && let Some(row) = rows.next().map_err(map_error)?
            {
                let mut cells = Vec::with_capacity(width);
                for index in 0..width {
                    cells.push(from_sqlite(row.get_ref(index).map_err(map_error)?));
                }
                values.push(cells);
            }
            crate::lookup::page(values, &lookup)
        })
        .await
    }
```

- [ ] **Step 7: MySQL**

In `crates/tabletist-db/src/mysql.rs`, add `Lookup, LookupPage` to the `use crate::{...}` list, and in `impl Adapter for Conn`, after `count_rows`:

```rust
    async fn lookup(&self, lookup: &Lookup) -> Result<LookupPage> {
        let sql = Dialect::MySql.lookup(lookup);
        let mut conn = self.conn.lock().await;
        let mut transaction = conn
            .start_transaction(read_only())
            .await
            .map_err(query_error)?;
        // `read_page` stops one row past the limit.
        let outcome = read_page(&mut transaction, &sql, lookup.limit as usize).await;
        let (_, rows) = finish(transaction, outcome).await?;
        crate::lookup::page(rows, lookup)
    }
```

- [ ] **Step 8: PostgreSQL**

In `crates/tabletist-db/src/pg.rs`, add `Lookup, LookupPage` and, if it is not there, `Sql` to the `use crate::{...}` list. `fetch_rows` holds the steps a lookup needs too, so they move into one method of `impl Conn` (put it beside `binary_columns`):

```rust
    /// Runs one reading statement in a read-only transaction and keeps at
    /// most one row more than `limit`. The time is the statement's, from
    /// when the session is this call's.
    async fn read(
        &self,
        sql: &Sql,
        limit: usize,
    ) -> Result<(Vec<ColumnMeta>, Vec<Vec<Value>>, Duration)> {
        let mut client = self.client.lock().await;
        let started = Instant::now();
        let transaction = client
            .build_transaction()
            .read_only(true)
            .start()
            .await
            .map_err(query_error)?;
        // Prepare first: it types the columns, and refuses a second
        // statement before anything runs.
        let statement = transaction.prepare(&sql.text).await.map_err(query_error)?;
        let columns = column_metas(&statement);
        let messages = transaction
            .simple_query(&sql.text)
            .await
            .map_err(query_error)?;
        transaction.rollback().await.map_err(query_error)?;
        let mut rows = Vec::new();
        for message in messages {
            if let SimpleQueryMessage::Row(row) = message {
                if rows.len() > limit {
                    break;
                }
                rows.push(row_values(&row, &columns)?);
            }
        }
        Ok((columns, rows, started.elapsed()))
    }
```

`fetch_rows` in `impl Adapter for Conn` becomes:

```rust
    async fn fetch_rows(&self, query: &RowQuery) -> Result<RowPage> {
        let key = self.primary_key(&query.object).await?;
        let binary = self.binary_columns(query).await?;
        let sql = Dialect::Postgres.select_rows(query, &key, &binary);
        let limit = query.limit as usize;
        let (columns, mut rows, elapsed) = self.read(&sql, limit).await?;
        let has_more = rows.len() > limit;
        rows.truncate(limit);
        Ok(RowPage {
            columns,
            rows,
            has_more,
            ordered_by_key: !key.is_empty(),
            elapsed,
        })
    }
```

and after `count_rows`:

```rust
    async fn lookup(&self, lookup: &Lookup) -> Result<LookupPage> {
        let sql = Dialect::Postgres.lookup(lookup);
        let (_, rows, _) = self.read(&sql, lookup.limit as usize).await?;
        crate::lookup::page(rows, lookup)
    }
```

The `CASE` answers an `int4`, which `value_from_pg_text` reads as `Value::Int`, so `Matched::from_cell` finds its 0, 1 or 2.

- [ ] **Step 9: Run the tests**

Run: `~/.cargo/bin/cargo test --locked -p tabletist-db --lib lookup && ~/.cargo/bin/cargo test --locked -p tabletist-db --test lookup && ~/.cargo/bin/cargo test --locked -p tabletist-db --test sqlite`
Expected: PASS. The page's unit tests and the four `sqlite::` tests of `lookup` pass, and SQLite's own suite still does.

If `the_bookshop_is_searched_on_sqlite` fails on the order of test 7, read the statement it ran (`Dialect::Sqlite.lookup(...)`.text) in the `sqlite3` shell against the same rows before changing anything: the order is `ORDER BY 3, 1`.

- [ ] **Step 10: Run the four checks, then commit**

```bash
git add crates/tabletist-db/src/adapter.rs crates/tabletist-db/src/lookup.rs crates/tabletist-db/src/lib.rs crates/tabletist-db/src/sqlite.rs crates/tabletist-db/src/mysql.rs crates/tabletist-db/src/pg.rs crates/tabletist-db/tests/lookup.rs && git commit -m "Look a referenced row up on the three engines

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 3: PostgreSQL and MySQL answer as SQLite does

**Files:**
- Modify: `crates/tabletist-db/tests/lookup.rs`
- Modify: `.github/workflows/ci.yml` (the two `cargo test ... --test insert_audit` lines, near lines 91 and 105)

- [ ] **Step 1: Write the tests**

Append to `crates/tabletist-db/tests/lookup.rs`. The tables are `lookup_publishers`, a name no other test file uses, since the test binaries share one database:

```rust
mod postgres {
    use super::*;
    use tabletist_db::TlsMode;

    const DROP: &str = "DROP TABLE IF EXISTS lookup_publishers";

    const CREATE: &str = "
        CREATE TABLE lookup_publishers (
            id bigint PRIMARY KEY,
            name text NOT NULL
        );
        INSERT INTO lookup_publishers VALUES
            (5, 'Studio 91'),
            (6, '100% Books'),
            (9100000000000000001, 'Northlight Books'),
            (9100000000000000004, 'Harbor Press'),
            (9100000000000000007, 'Harbor Lights')";

    fn url() -> Option<String> {
        let url = std::env::var("TABLETIST_TEST_PG_URL").ok();
        url.filter(|url| !url.trim().is_empty())
    }

    #[tokio::test]
    async fn the_bookshop_is_searched_on_postgres() {
        let Some(url) = url() else {
            eprintln!("skipped: TABLETIST_TEST_PG_URL is not set");
            return;
        };
        let mut config: tokio_postgres::Config = url.parse().unwrap();
        config.ssl_mode(tokio_postgres::config::SslMode::Disable);
        let (admin, session) = config.connect(tokio_postgres::NoTls).await.unwrap();
        tokio::spawn(session);
        admin.batch_execute(DROP).await.unwrap();
        admin.batch_execute(CREATE).await.unwrap();

        let (mut spec, secrets) = ConnectSpec::from_url(&url).unwrap();
        spec.tls = TlsMode::Disable;
        // Test 8: a session that cannot write looks up as one that can.
        for access in [Access::ReadOnly, Access::Writable] {
            let connection =
                Connection::connect_with(&spec, &secrets, &HostKeys::default(), access)
                    .await
                    .unwrap();
            the_bookshop_is_searched(&connection, "public", "lookup_publishers").await;
            // A text that is no number is no error on a bigint key: the
            // key is compared as text.
            let page = connection
                .lookup(&publishers("public", "lookup_publishers", matching("harbor p"), 50))
                .await
                .unwrap();
            assert_eq!(page.rows.len(), 1);
        }
        admin.batch_execute(DROP).await.unwrap();
    }
}

mod mysql {
    use super::*;
    use mysql_async::prelude::Queryable;
    use tabletist_db::TlsMode;

    const DROP: &str = "DROP TABLE IF EXISTS lookup_publishers";

    const CREATE: [&str; 2] = [
        "CREATE TABLE lookup_publishers (
             id BIGINT PRIMARY KEY,
             name VARCHAR(64) NOT NULL
         )",
        "INSERT INTO lookup_publishers VALUES
             (5, 'Studio 91'),
             (6, '100% Books'),
             (9100000000000000001, 'Northlight Books'),
             (9100000000000000004, 'Harbor Press'),
             (9100000000000000007, 'Harbor Lights')",
    ];

    fn url() -> Option<String> {
        let url = std::env::var("TABLETIST_TEST_MYSQL_URL").ok();
        url.filter(|url| !url.trim().is_empty())
    }

    #[tokio::test]
    async fn the_bookshop_is_searched_on_mysql() {
        let Some(url) = url() else {
            eprintln!("skipped: TABLETIST_TEST_MYSQL_URL is not set");
            return;
        };
        let opts = mysql_async::Opts::from_url(&format!("{url}?prefer_socket=false")).unwrap();
        let mut admin = mysql_async::Conn::new(opts).await.unwrap();
        admin.query_drop(DROP).await.unwrap();
        for statement in CREATE {
            admin.query_drop(statement).await.unwrap();
        }

        let (mut spec, secrets) = ConnectSpec::from_url(&url).unwrap();
        spec.tls = TlsMode::Disable;
        for access in [Access::ReadOnly, Access::Writable] {
            let connection =
                Connection::connect_with(&spec, &secrets, &HostKeys::default(), access)
                    .await
                    .unwrap();
            the_bookshop_is_searched(&connection, "tabletist", "lookup_publishers").await;
        }
        admin.query_drop(DROP).await.unwrap();
    }
}
```

`tokio_postgres` and `mysql_async` are the test crate's already (`tests/insert_audit.rs` uses both). The schema names (`public`, `tabletist`) are the ones `tests/insert_audit.rs` uses for the same servers.

- [ ] **Step 2: Run them with the servers up**

Run, with the two variables of "Before you start" exported:
`~/.cargo/bin/cargo test --locked -p tabletist-db --test lookup -- --nocapture`
Expected: five tests pass and no line says "skipped".

If one of the two fails, the statement is wrong for that engine, not the test: the three engines must give the same rows. Likely places, in the order to look:
1. PostgreSQL refuses the `prepare`: read its message, it names the place in the statement.
2. MySQL's `CASE` comes back as something other than `Value::Int`: print `page.rows[0].matched` and the raw cell, and widen `Matched::from_cell` in `crates/tabletist-db/src/lookup.rs` to read it, with a unit test there.
3. The order of test 7 differs: a collation is ordering column 3 as text. Then it is no number, which is point 2.

Fix `Dialect::lookup` and its unit tests of task 1 together, run the whole `lookup` suite again on all three engines, and say what changed in the task's report.

- [ ] **Step 3: Have CI run them**

In `.github/workflows/ci.yml`, the PostgreSQL job runs
`cargo test --locked -p tabletist-db --lib --test postgres --test insert_audit` and the MySQL job runs
`cargo test --locked -p tabletist-db --lib --test mysql --test insert_audit`. Add ` --test lookup` to the end of both lines. Nothing else in the file changes.

- [ ] **Step 4: Run the four checks, then commit**

```bash
git add crates/tabletist-db/tests/lookup.rs .github/workflows/ci.yml && git commit -m "Hold PostgreSQL and MySQL to the lookup SQLite answers

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 4: SQLite names the column an implicit reference means

`publisher_id INTEGER REFERENCES publishers` names no column and means the parent's primary key. `pragma_foreign_key_list` then answers NULL for the column, and `ForeignKeyInfo::ref_columns` is empty. The app follows such a key by a column with no name today (`src/app.rs`, `Action::FollowSelectedKey`), and the picker would have none to search by.

**Files:**
- Modify: `crates/tabletist-db/src/sqlite.rs` (`foreign_keys`, near line 1233)
- Modify: `crates/tabletist-db/src/catalog.rs` (the comment on `ref_columns`, near line 91)
- Modify: `crates/tabletist-db/tests/lookup.rs` (the `sqlite` module)

- [ ] **Step 1: Write the failing test**

In `crates/tabletist-db/tests/lookup.rs`, inside `mod sqlite`, add:

```rust
    /// Test 9. `REFERENCES publishers` means the parent's primary key.
    #[tokio::test]
    async fn an_implicit_reference_names_the_parents_primary_key() {
        const KEYS: &str = "
            CREATE TABLE publishers (id INTEGER PRIMARY KEY, name TEXT NOT NULL);
            CREATE TABLE pairs (a INTEGER, b INTEGER, PRIMARY KEY (a, b));
            CREATE TABLE keyless (name TEXT);
            CREATE TABLE book_covers (
                id INTEGER PRIMARY KEY,
                publisher_id INTEGER REFERENCES publishers,
                named_id INTEGER REFERENCES publishers (id),
                a INTEGER,
                b INTEGER,
                keyless_id INTEGER REFERENCES keyless,
                gone_id INTEGER REFERENCES nowhere,
                FOREIGN KEY (a, b) REFERENCES pairs
            );";
        let (connection, _dir) = made_with(KEYS, Access::ReadOnly).await;
        let structure = connection
            .describe(&ObjectRef::new("main", "book_covers"))
            .await
            .unwrap();
        let referenced = |column: &str| {
            let key = structure
                .foreign_keys
                .iter()
                .find(|key| key.columns[0] == column)
                .unwrap_or_else(|| panic!("no key of {column}"));
            (key.ref_table.as_str(), key.ref_columns.clone())
        };
        assert_eq!(referenced("publisher_id"), ("publishers", vec!["id".to_owned()]));
        assert_eq!(referenced("named_id"), ("publishers", vec!["id".to_owned()]));
        assert_eq!(
            referenced("a"),
            ("pairs", vec!["a".to_owned(), "b".to_owned()])
        );
        // No primary key to mean, and no table: the key names no column.
        assert_eq!(referenced("keyless_id"), ("keyless", Vec::new()));
        assert_eq!(referenced("gone_id"), ("nowhere", Vec::new()));
    }
```

- [ ] **Step 2: Run it to see it fail**

Run: `~/.cargo/bin/cargo test --locked -p tabletist-db --test lookup an_implicit_reference`
Expected: FAIL at `publisher_id`: left `("publishers", [])`, right `("publishers", ["id"])`.

- [ ] **Step 3: Fill the column in**

In `crates/tabletist-db/src/sqlite.rs`, replace the last line of `foreign_keys` (`Ok(keys.into_iter().map(|(_, key)| key).collect())`) with:

```rust
    let mut keys: Vec<ForeignKeyInfo> = keys.into_iter().map(|(_, key)| key).collect();
    for key in &mut keys {
        if !key.ref_columns.is_empty() {
            continue;
        }
        // `REFERENCES parent` names no column and means the parent's
        // primary key. A parent that is not there, or has no primary key
        // of as many columns, is a key SQLite itself refuses when a row is
        // written: it stays without a column here. A name that is not
        // UTF-8 cannot be written into a statement, so it is no column to
        // follow the key by.
        let parent = ObjectRef::new(key.ref_schema.clone(), key.ref_table.clone());
        let primary = ordering_key(connection, &parent)?;
        if primary.len() == key.columns.len() {
            key.ref_columns = primary;
        }
    }
    Ok(keys)
```

`ordering_key` is the file's own (near line 1146): the primary key's names, or nothing when one is not UTF-8.

- [ ] **Step 4: Say it where the field is declared**

In `crates/tabletist-db/src/catalog.rs`, replace the comment on `ForeignKeyInfo::ref_columns`:

```rust
    /// The columns of `ref_table` the key points at. Empty only on SQLite,
    /// for a key that names none (`REFERENCES parent`) whose parent is not
    /// there or has no primary key of as many columns.
    pub ref_columns: Vec<String>,
```

- [ ] **Step 5: Run the tests**

Run: `~/.cargo/bin/cargo test --locked -p tabletist-db`
Expected: PASS, the new test and every structure test of `tests/sqlite.rs` and `src/sqlite.rs` (their keys name their columns, so nothing of theirs changes).

- [ ] **Step 6: Run the four checks, then commit**

```bash
git add crates/tabletist-db/src/sqlite.rs crates/tabletist-db/src/catalog.rs crates/tabletist-db/tests/lookup.rs && git commit -m "Name the column a SQLite key means when it names none

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 5: The backend runs a lookup

**Files:**
- Modify: `src/backend.rs` (`Command` near line 99, `Event` near line 244, `session_of` near line 1856, `request_of` near line 1879, `fail` near line 1931, the session's command loop near line 2132, the tests module)
- Modify: `src/app.rs` (`apply_event`, near line 2574)

- [ ] **Step 1: Write the failing test**

In `src/backend.rs`, in the tests module, after `first_users_name` (near line 4690), add. `connected_as` is the module's helper: the demo fixture of `crates/tabletist-db/fixtures/sqlite.sql`, whose `users` are Ada (1), Bob (2), Zoë (3), "50% off" (4) and one with no name (5):

```rust
    #[test]
    fn a_lookup_answers_with_the_rows_it_found() {
        let (_dir, mut backend, session) = connected_as(Access::ReadOnly);
        let lookup = |part: tabletist_db::LookupPart| tabletist_db::Lookup {
            object: ObjectRef::new("main", "users"),
            key: "id".into(),
            label: Some("email".into()),
            part,
            limit: 50,
        };
        backend.send(Command::Lookup {
            session,
            request: RequestId(21),
            lookup: lookup(tabletist_db::LookupPart::Matching("ADA".into())),
        });
        match backend.wait(WAIT) {
            Some(Event::Lookup {
                session: SessionId(1),
                request: RequestId(21),
                result: Ok(page),
            }) => {
                assert_eq!(page.rows.len(), 1);
                assert_eq!(page.rows[0].key, Value::Int(1));
                assert_eq!(
                    page.rows[0].label,
                    Some(Value::Text("ada@example.com".into()))
                );
            }
            other => panic!("expected the lookup's rows, got {other:?}"),
        }

        backend.send(Command::Lookup {
            session,
            request: RequestId(22),
            lookup: tabletist_db::Lookup {
                object: ObjectRef::new("main", "nowhere"),
                ..lookup(tabletist_db::LookupPart::Pinned(Value::Int(1)))
            },
        });
        assert!(matches!(
            backend.wait(WAIT),
            Some(Event::Lookup {
                request: RequestId(22),
                result: Err(_),
                ..
            })
        ));
    }

    #[test]
    fn a_lookup_prints_without_what_was_typed() {
        let command = Command::Lookup {
            session: SessionId(1),
            request: RequestId(1),
            lookup: tabletist_db::Lookup {
                object: ObjectRef::new("main", "users"),
                key: "id".into(),
                label: Some("email".into()),
                part: tabletist_db::LookupPart::Matching("ada@example.com".into()),
                limit: 50,
            },
        };
        assert!(!format!("{command:?}").contains("ada@"));
    }
```

`Value` is in the module's imports already (`first_users_name` returns one). If `connected_as` is declared below the place you add these, that is fine: it is a module-level function.

- [ ] **Step 2: Run it to see it fail**

Run: `~/.cargo/bin/cargo test --locked --lib backend::tests::a_lookup`
Expected: a compile error, `no variant named Lookup found for enum Command`.

- [ ] **Step 3: The command and the event**

In `src/backend.rs`, in `enum Command`, after `CountRows`:

```rust
    /// Read rows of the table a foreign key points at (see
    /// `Connection::lookup`). It waits its turn like every command of its
    /// session, and a `Cancel` stops it as it stops a page.
    Lookup {
        session: SessionId,
        request: RequestId,
        lookup: tabletist_db::Lookup,
    },
```

In `enum Event`, after `Count`:

```rust
    Lookup {
        session: SessionId,
        request: RequestId,
        result: Result<tabletist_db::LookupPage, Error>,
    },
```

- [ ] **Step 4: The four places that name every command**

In `session_of`, add to the first arm's list (the commands that have a session), after `| Command::CountRows { session, .. }`:

```rust
        | Command::Lookup { session, .. }
```

In `request_of`, after `| Command::CountRows { request, .. }`:

```rust
        | Command::Lookup { request, .. }
```

In `fail`, after the `Command::CountRows` arm:

```rust
        Command::Lookup {
            session, request, ..
        } => Event::Lookup {
            session,
            request,
            result: Err(error),
        },
```

In the session's command loop (the `match` whose arms call `connection.fetch_rows(query).await` and `connection.count_rows(query).await`), after the `Command::CountRows` arm:

```rust
            Command::Lookup {
                session,
                request,
                lookup,
            } => {
                let result = connection.lookup(lookup).await;
                let lost = lost_error(&result);
                outbox.emit(Event::Lookup {
                    session: *session,
                    request: *request,
                    result,
                });
                lost
            }
```

Then build: `~/.cargo/bin/cargo build --locked 2>&1 | head -60`. The compiler names every other `match` over `Command` or `Event` that has no wildcard. For each one it names in `src/backend.rs`, put `Lookup` beside `CountRows` (or `Event::Lookup` beside `Event::Count`), with the same treatment that arm gives a count. Do not add a `_` arm anywhere.

- [ ] **Step 5: The app drops the answer until run 2 reads it**

`App::apply_event` in `src/app.rs` matches every event. After its `Event::Count { .. }` arm add:

```rust
            // Nothing asks for a lookup yet: the foreign key picker will
            // (docs/superpowers/specs/2026-10-10-foreign-key-editing-design.md).
            Event::Lookup { .. } => {}
```

If the compiler names another exhaustive `match` over `Event` in `src/` (the test harness in `src/testing.rs`, `src/shots.rs`), give it the same empty arm with the same comment.

- [ ] **Step 6: Run the tests**

Run: `~/.cargo/bin/cargo test --locked --lib backend::tests::a_lookup`
Expected: PASS, both tests.

- [ ] **Step 7: Run the four checks, and the screenshots' build**

`src/shots.rs` is outside the four checks and names backend commands:

```bash
~/.cargo/bin/cargo clippy --locked --features shots --lib --tests -- -D warnings
```

Expected: no warnings. Then commit:

```bash
git add src/backend.rs src/app.rs && git commit -m "Run a lookup as a command of the backend

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

(Add `src/testing.rs` or `src/shots.rs` to the `git add` if step 5 touched them.)

---

### Task 6: Say what is built

**Files:**
- Modify: `docs/superpowers/specs/2026-10-10-foreign-key-editing-design.md`

- [ ] **Step 1: The status line**

Replace the spec's status line, `Date: 2026-10-10. Status: not built.`, with:

```markdown
Date: 2026-10-10. Status: built in runs. Run 1
(`docs/superpowers/plans/2026-10-10-foreign-key-editing-1-lookup.md`) is
built: the three drivers look a referenced row up, the backend runs it,
and SQLite names the column an implicit reference means. Nothing in the
app asks for a lookup yet. What is left is listed under "Delivery".
```

- [ ] **Step 2: The pinned value**

In the spec's "Implementation notes", the block that declares `LookupPart` has `Pinned(String)`. Make it what was built, and say why in the sentence under the block:

```markdown
          /// The one row whose key is this, by `=`.
          Pinned(Value),
```

and replace the sentence "The app decides whether a text can be pinned (it has the check for a typed cell), so the crate needs no type of the key." with:

```markdown
  The app decides whether a text can be pinned (it has the check for a
  typed cell) and makes the value: a whole number for a key that is one,
  since MySQL compares a `BIGINT` with a string as floats. So the crate
  needs no type of the key.
```

- [ ] **Step 3: Check the page builds nothing new**

`docs/superpowers/` is not part of the website (`docs/_config.yml` excludes it), so there is no site check to run. Run `git diff --stat` and confirm only the spec changed.

- [ ] **Step 4: Commit**

```bash
git add docs/superpowers/specs/2026-10-10-foreign-key-editing-design.md && git commit -m "Say that the lookup of a referenced row is built

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

## What this leaves for the runs

- **Run 2** (`2026-10-10-foreign-key-editing-2-grid-picker.md`) sends `Command::Lookup` from the cell's editor and reads `Event::Lookup`. It makes the pinned `Value` from the typed text, chooses the label column, and replaces the empty arm of `App::apply_event`.
- **The website** does not change in this run: nothing a user sees is new, except that a SQLite key declared without a column now shows its column in the Structure view and can be followed with `gd`. The pull request says so.
- **Not run where this plan was written:** every statement. Task 3 is where PostgreSQL and MySQL first see them. Its report says which servers answered (their versions), or that they could not be reached.
