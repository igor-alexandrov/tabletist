# Connection::write Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** `tabletist-db` can change the values of existing rows, in one transaction that never overwrites a row someone else changed, and the backend can ask it to. Nothing in the UI calls it yet.

**Architecture:** One writing call, `Connection::write(&ChangeSet)`, refused on a read-only connection before anything is sent. One builder in `dialect.rs` turns a row's change into the `UPDATE` the user will read and the statement the driver runs, so the two cannot drift. Each driver runs the same six steps in its own transaction: begin, lock and read each row by its key, compare the changed columns with what the page loaded, update, read back, commit. The catalog learns which columns are generated, which indexes are partial, and which columns tell one row from another.

**Tech Stack:** Rust 1.98, `tokio-postgres`, `mysql_async`, `rusqlite`. Spec: `docs/superpowers/specs/2026-10-03-value-editing-core-design.md`, sections "What can be edited", "Checks before sending" and "Saving". This plan is its step 2.

---

## Before you start

- Cargo is `~/.cargo/bin/cargo` (the mise shim fails). Never point `CARGO_TARGET_DIR` at `/tmp`.
- Checks, from `AGENTS.md`:

      ~/.cargo/bin/cargo fmt --all --check
      ~/.cargo/bin/cargo clippy --locked --workspace --all-targets -- -D warnings
      RUSTDOCFLAGS='-D warnings' ~/.cargo/bin/cargo doc --locked --workspace --no-deps
      ~/.cargo/bin/cargo test --locked --workspace --all-targets

- The PostgreSQL and MySQL suites need servers; without them they print "skipped" and prove nothing:

      export TABLETIST_TEST_PG_URL=postgres://tabletist:tabletist@localhost:55432/tabletist
      export TABLETIST_TEST_MYSQL_URL=mysql://tabletist:tabletist@localhost:53306/tabletist

- The PostgreSQL and MySQL fixtures are loaded once and shared by every test of a suite. **A write test never changes a fixture table.** It creates a table of its own through the suite's `admin()` connection, under a name no other test uses, and drops it when done. SQLite tests each get their own file and may change it freely.
- **This plan runs on the branch `claude/connection-write`,** off `main`, which holds all of step 1 (`Access`, the fenced sessions, `Workspace::access`, `Command::Connect { access }`, `Event::Connected { access }`).
- **Tasks 1 to 9 are built**, with what their review changed (see "As built" under each, and decisions 10 to 13 below). Only task 10, the documents, is left; it writes the decisions into the spec.
- **How the SQLite driver stands, which the SQLite tasks build on.** Read each function before you edit it.
  - `Conn` holds `inner: Arc<Mutex<rusqlite::Connection>>`, the interrupt handle and `fences`. `open` installs the authorizer with `tabletist_sqlite_ffi::set_authorizer(&connection, ..)` after `set_session_pragmas` and the first read. A save's statements are the app's own: they run with no fence up.
  - Names can hold bytes that are not UTF-8, and rusqlite panics on them. So the catalog reads text through `text` and `optional_text` (as `Lossy`, which also says whether the name is exact), an index is looked up by the bytes of its name, and a statement's columns come from `declared_columns` (through `tabletist_sqlite_ffi::result_columns`). **Never read a statement's column names through rusqlite** (`column_names`, `column_name`, `columns`).
  - `ordering_key` gives a page no key when one of the key's names is not exact, since no SQL can spell that column. A save cannot name it either.
- House rules that bite here: no em dashes anywhere; comments say why, in the surrounding code's voice; no `unsafe`; do not weaken a lint or delete a test to get green.
- Commit after every task. Subjects are plain sentences, each ending with:

      Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>

  If the commit's signing agent is locked ("agent refused operation"), do not bypass it: `git add -A`, `git write-tree`, and report the tree id with the subject.

## What this plan decides beyond the spec

The spec leaves these open; task 10 writes them into it.

1. **`Command::Write` carries no tab.** The reducer finds the tab by the request, as for every other request.
2. **MySQL: only tables whose engine has transactions.** One transaction is the promise, and MyISAM cannot roll back. Others are refused before anything is sent.
3. **SQLite binds a save's values exactly as built.** The filter path turns text that looks like a number into a number (`to_sqlite`); a save does not guess. One case needs a rule of its own: a column with no declared type (or one SQLite gives no affinity) converts nothing, so text would turn a stored number into text. There the builder follows what the cell held: if the loaded value was a number and the new text is one, it goes as a number.
4. **SQLite puts the session in order before it writes.** A script can leave pragmas on the session that outlive its rollback, and a save puts back every one that changes what it writes or whether it can: `main`'s `journal_mode`, `locking_mode`, `ignore_check_constraints`, `recursive_triggers` (with it on, a table's triggers fire on what triggers wrote, and a save writes more than it would have) and `count_changes` (with it on, an `UPDATE` answers with a row and the driver refuses it). The journal mode is restored on `main` only, since without a schema the pragma would set every attached database's mode, and only when neither the mode now nor the mode at open is `wal`: WAL is the file's and another program may have given it, every other mode is the session's. `full_column_names` and `short_column_names` rename a statement's columns, which a page feels as much as a save, so `set_session_pragmas` puts those back after every script.
4a. **SQLite saves only to `main`.** A table of an attached database is refused: the session knows `main`'s journal mode only, and a script's `ATTACH` is the one way such a database appears.
5. **The crate checks only what it converts.** Numbers on SQLite and booleans on SQLite and MySQL are parsed by the statement builder; text it cannot convert is `WriteOutcome::Failed` before anything is sent. The messages a user sees while typing are step 3. Two rules came out of the review of the classes: a column of class `Binary` is refused outright, a NULL for it too (MySQL stores `'1'` in a `BIT(8)` as 49, the character's code), and a MySQL `tinyint(1)` takes `true` and `false` but also any whole number a tinyint holds, since some tables keep more than a flag in one.
6. **A key value that is NULL, a key column that is also changed, a column changed twice, a row named twice:** refused by `ChangeSet::check`. Two changes with the same key would both pass the conflict check against the row as it was, and the later would overwrite the earlier. Keys are the same when they name the same columns with the same values, in whatever order.
7. **MySQL starts its transaction with `START TRANSACTION READ WRITE` as text,** not through the driver's transaction options. The driver opens a read-only transaction as `SET TRANSACTION READ ONLY` then `START TRANSACTION`, and a cancel between the two can leave "next transaction read-only" pending.
8. **PostgreSQL literals** are `'...'` when the text holds no backslash and `E'...'` when it does, so Review SQL reads plainly and still runs as shown.
9. **`Err` from `write` is not always a lost session.** The spec says an `Err` is a failure of the session or the run. A lock another session holds until a timeout, a busy file at `BEGIN IMMEDIATE`, a `COMMIT` the database refuses: each is an ordinary `Error::Query` on a session that lives. Nothing was written in any of them. Only `ConnectionLost` means the session is gone.
10. **SQLite refuses what it did not read exactly.** A name or a text value that is not UTF-8 reads with U+FFFD for the bad bytes, and so can read the same as another. A save refuses three things rather than guess: a name it uses (in the key or the set) that more than one of the row's columns reads as, counted without regard to ASCII case as SQLite matches names (the row would be compared in one column and written in the other); a key whose text holds U+FFFD (it could name another row whose key really is that text; a key that really holds U+FFFD pays for this); and a changed column whose stored text is not UTF-8 (two different values read the same, so the save cannot tell whether someone changed it). Text that really holds U+FFFD in a changed column saves as any other.
11. **A key must name one row, and each driver refuses or prevents what it cannot be sure of.** The read by key asks for two rows at most (`LIMIT 2`) and more than one is an error, before the updates and again when each row is read back: a trigger an update fired can make a second row the key finds, and then which row was saved is not known. PostgreSQL sends a save's values as literals built from what the page read, so every session prints floats in full and dates in ISO (`SET extra_float_digits = 3; SET DateStyle = 'ISO'` at connect, for both access modes): with fewer digits two neighbouring floats print alike, and a zone's abbreviation can read back as another zone. This changes what a page shows on a server configured otherwise. MySQL refuses a row key with a `timestamp`, `bit` or `float` column: a TIMESTAMP shows in the session's zone without it, so two instants of a repeated daylight-saving hour read alike, a BIT bound as bytes is read as a number, and a FLOAT bound as a double misses its row.
12. **PostgreSQL's transaction is managed as text,** not through `tokio_postgres::Transaction`: a dropped `Transaction` only queues its `ROLLBACK` and never reads the answer, and `commit()` and `rollback()` consume it, so one a cancel landed on cannot be tried again. The opening message is `ROLLBACK; START TRANSACTION READ WRITE; SET LOCAL client_encoding = 'UTF8'`. The encoding is pinned because the statement carries its values as literals; nothing in the app can change it, a pooler is the one path left. Text with a NUL is refused before it is sent, since the driver's own refusal reads as a lost connection.
13. **MySQL beyond decisions 2 and 7.** The engine is checked before the transaction and again after the locking reads, which hold the table's metadata lock to the end, so an `ALTER TABLE .. ENGINE` in between cannot leave updates that would not roll back. That second check is current only because it is the transaction's first read that takes no lock. A name spelled in other letters than the table spells it is refused (MySQL takes `ID` and `id` for one column, and a set naming one with a key naming the other changed its own key). A note counts as a warning (a decimal rounded to its column's scale raises only a note), and every session has `sql_notes = 1`, since a server can have notes off. No changed row is taken as the row already holding the value; more than one fails. The transaction ends with `COMMIT` or `ROLLBACK AND NO CHAIN NO RELEASE`, whatever `completion_type` the server has.
14. **A save starts from no transaction.** Each driver rolls back whatever it finds open before it starts its own (PostgreSQL in the opening message, MySQL when the status of the last answer says one is open), so a save whose future was dropped half way cannot have its rows committed by the next. The backend never drops one; this is the guard for the day something does.

## Where a run can stop

Each of these leaves the branch shippable: after task 3 (the catalog and the classes; no behaviour changes), after task 6 (SQLite saves end to end), after task 9 (everything but the docs).

## File map

| File | What changes |
|---|---|
| `crates/tabletist-db/src/catalog.rs` | `ColumnInfo::generated`, `IndexInfo::partial`, `Structure::row_key` |
| `crates/tabletist-db/src/class.rs` (new) | `ColumnClass`, `column_class` |
| `crates/tabletist-db/src/write.rs` (new) | `ChangeSet`, `RowChange`, `CellChange`, `NewValue`, `WriteOutcome`, `Conflict`, the checks shared by the drivers |
| `crates/tabletist-db/src/dialect.rs` | the statement builder: `update_row`, `select_row` |
| `crates/tabletist-db/src/error.rs` | `Error::ReadOnly` |
| `crates/tabletist-db/src/lib.rs` | `Connection::write`, the exports |
| `crates/tabletist-db/src/sqlite/write.rs`, `pg/write.rs`, `mysql/write.rs` (new) | each driver's transaction |
| `crates/tabletist-db/src/{sqlite,pg,mysql}.rs` | the catalog queries; what the write modules need made `pub(super)` |
| `crates/tabletist-db/tests/{sqlite,postgres,mysql}.rs` | the write tests |
| `src/backend.rs` | `Command::Write`, `Event::Written` |
| `src/app.rs` | an arm that leaves `Event::Written` alone until step 3 |
| docs | the spec, the main spec's API list, the crate doc |

---

### Task 1: The catalog says which columns are generated

**Files:**
- Modify: `crates/tabletist-db/src/catalog.rs` (`ColumnInfo`)
- Modify: `crates/tabletist-db/src/pg.rs` (`describe`), `mysql.rs` (`describe`), `sqlite.rs` (`columns`)
- Test: `crates/tabletist-db/tests/postgres.rs`, `tests/mysql.rs`, the test module of `sqlite.rs`

- [ ] **Step 1: The field**

```rust
    /// Whether the database computes the column itself and refuses a value
    /// for it: a generated column, or an identity column that is always
    /// generated.
    pub generated: bool,
```

`ColumnInfo` derives `Default`. Every struct literal of `ColumnInfo` in the workspace gains the field (the three drivers set it in the next steps; a test's literal takes `generated: false` or `..Default::default()`). Find them: `grep -rn "ColumnInfo {" crates src`.

- [ ] **Step 2: Write the failing tests**

SQLite, in the test module of `sqlite.rs` (the file is the test's own):

```rust
    #[tokio::test]
    async fn a_generated_column_says_so() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("generated.db");
        rusqlite::Connection::open(&path)
            .unwrap()
            .execute_batch(
                "CREATE TABLE books (
                     id INTEGER PRIMARY KEY,
                     title TEXT NOT NULL,
                     slug TEXT GENERATED ALWAYS AS (lower(title)) VIRTUAL,
                     shout TEXT GENERATED ALWAYS AS (upper(title)) STORED
                 )",
            )
            .unwrap();
        let conn = Conn::open(&path, Access::ReadOnly).await.unwrap();
        let structure = conn
            .describe(&ObjectRef::new("main", "books"))
            .await
            .unwrap();
        let generated: Vec<(&str, bool)> = structure
            .columns
            .iter()
            .map(|column| (column.name.as_str(), column.generated))
            .collect();
        assert_eq!(
            generated,
            [("id", false), ("title", false), ("slug", true), ("shout", true)]
        );
    }
```

PostgreSQL, in `tests/postgres.rs` (its own table, through `admin()`):

```rust
#[tokio::test]
async fn generated_and_always_identity_columns_say_so() {
    let Some(connection) = connect().await else {
        return;
    };
    let admin = admin().await;
    admin
        .batch_execute(
            "DROP TABLE IF EXISTS catalog_generated;
             CREATE TABLE catalog_generated (
                 id bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
                 seq bigint GENERATED BY DEFAULT AS IDENTITY,
                 title text NOT NULL,
                 slug text GENERATED ALWAYS AS (lower(title)) STORED
             )",
        )
        .await
        .unwrap();
    let structure = connection
        .describe(&ObjectRef::new("public", "catalog_generated"))
        .await;
    admin
        .batch_execute("DROP TABLE catalog_generated")
        .await
        .unwrap();
    let generated: Vec<(String, bool)> = structure
        .unwrap()
        .columns
        .into_iter()
        .map(|column| (column.name, column.generated))
        .collect();
    assert_eq!(
        generated,
        [
            ("id".to_owned(), true),
            // By default: the database takes a value when given one.
            ("seq".to_owned(), false),
            ("title".to_owned(), false),
            ("slug".to_owned(), true),
        ]
    );
}
```

MySQL, in `tests/mysql.rs`, the same shape: a table `catalog_generated (id INT PRIMARY KEY, title VARCHAR(50) NOT NULL, stamp TIMESTAMP DEFAULT CURRENT_TIMESTAMP, slug VARCHAR(50) GENERATED ALWAYS AS (LOWER(title)) VIRTUAL, shout VARCHAR(50) GENERATED ALWAYS AS (UPPER(title)) STORED)` in the fixture's database, expecting `id`, `title` and `stamp` false, `slug` and `shout` true. `stamp` is there because MySQL 8 marks a column with a default expression `DEFAULT_GENERATED`, which is not a generated column.

- [ ] **Step 3: Run and see them fail**

Run: `~/.cargo/bin/cargo test --locked -p tabletist-db generated`
Expected: FAIL: every column reports `false`.

- [ ] **Step 4: Implement**

- **SQLite**, `columns`: select `hidden` too and set `generated: matches!(hidden, 2 | 3)`. `pragma_table_xinfo` gives 2 for a virtual generated column and 3 for a stored one; the query already leaves out 1 (a virtual table's hidden columns).
- **PostgreSQL**, `describe`: add an eighth output column and read it as `generated: column(row, 7)?`:

```sql
COALESCE(to_jsonb(a) ->> 'attgenerated', '') <> '' OR COALESCE(to_jsonb(a) ->> 'attidentity', '') = 'a'
```

  with the comment that the two attributes are read through `to_jsonb` because `attgenerated` only exists from PostgreSQL 12 and `attidentity` from 10, and naming a column a server lacks would fail the whole Structure view.
- **MySQL**, `describe`: select `extra` as a sixth column and set

```rust
                // `VIRTUAL GENERATED`, `STORED GENERATED`, and on an older
                // MariaDB `VIRTUAL` or `PERSISTENT`. Not `DEFAULT_GENERATED`,
                // which MySQL 8 says of a default that is an expression.
                generated: ["VIRTUAL", "STORED", "PERSISTENT"]
                    .iter()
                    .any(|word| extra.to_ascii_uppercase().contains(word)),
```

- [ ] **Step 5: Run**

Run: `~/.cargo/bin/cargo test --locked --workspace --all-targets`
Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add -A && git commit -m "Say in the catalog which columns the database computes"
```

---

### Task 2: Partial indexes and the row key

> **As built:** the review found that an index's display text cannot say whether it names a row. A MySQL prefix index (`UNIQUE (name(1))`) and an index of another collation than its column's both read as plain columns and can match two rows, and PostgreSQL's primary key query took `INCLUDE` columns for key columns. So `IndexInfo` also has `key_columns: Option<Vec<String>>`, set by each driver only when every entry is one whole column compared as the column compares (PostgreSQL: no expression, the column's collation and its default operator class; MySQL: no expression and no prefix; SQLite: no expression), and `Structure::row_key` reads that instead of matching names. A primary key whose own index is not over whole columns (MySQL allows `PRIMARY KEY (name(1))`) is passed over too. SQLite cannot say a column's declared collation, so an index or primary key of another collation is not detected there; a save's own check (it reads the row by its key and refuses more than one) is what stops it. The steps below are the first draft.

**Files:**
- Modify: `crates/tabletist-db/src/catalog.rs` (`IndexInfo`, `Structure`)
- Modify: `crates/tabletist-db/src/pg.rs` (`describe`), `sqlite.rs` (`indexes`), `mysql.rs` (`describe`)
- Test: the test module of `catalog.rs` (new), `tests/postgres.rs`, the test module of `sqlite.rs`

- [ ] **Step 1: The field and the failing tests of the rule**

```rust
    /// Whether the index covers only the rows its condition keeps. Such an
    /// index says nothing about the rest, so it cannot name a row.
    pub partial: bool,
```

Every `IndexInfo` literal gains it (`grep -rn "IndexInfo {" crates src`).

At the end of `catalog.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn column(name: &str, nullable: bool) -> ColumnInfo {
        ColumnInfo {
            name: name.into(),
            nullable,
            ..ColumnInfo::default()
        }
    }

    fn unique(name: &str, columns: &[&str]) -> IndexInfo {
        IndexInfo {
            name: name.into(),
            columns: columns.iter().map(|column| (*column).to_owned()).collect(),
            unique: true,
            ..IndexInfo::default()
        }
    }

    fn table(indexes: Vec<IndexInfo>) -> Structure {
        Structure {
            columns: vec![
                column("id", false),
                column("email", false),
                column("code", false),
                column("nick", true),
                column("odd name", false),
            ],
            indexes,
            ..Structure::default()
        }
    }

    #[test]
    fn the_primary_key_is_the_row_key() {
        let mut structure = table(vec![unique("by_email", &["email"])]);
        structure.primary_key = vec!["id".into()];
        assert_eq!(structure.row_key(), Some(vec!["id".to_owned()]));
    }

    #[test]
    fn without_one_the_first_usable_unique_index_is() {
        let key = |indexes| table(indexes).row_key();
        // By name, whatever order the catalog gave them in.
        assert_eq!(
            key(vec![unique("z", &["code"]), unique("a", &["email"])]),
            Some(vec!["email".to_owned()])
        );
        // Several columns, in the index's order.
        assert_eq!(
            key(vec![unique("pair", &["code", "email"])]),
            Some(vec!["code".to_owned(), "email".to_owned()])
        );
        // PostgreSQL gives a name that needs quotes quoted.
        assert_eq!(
            key(vec![unique("odd", &["\"odd name\""])]),
            Some(vec!["odd name".to_owned()])
        );
    }

    #[test]
    fn an_index_that_cannot_name_a_row_is_passed_over() {
        let key = |indexes| table(indexes).row_key();
        let plain = IndexInfo {
            unique: false,
            ..unique("plain", &["email"])
        };
        let partial = IndexInfo {
            partial: true,
            ..unique("partial", &["email"])
        };
        for index in [
            plain,
            partial,
            // A column that can be NULL: two rows may both hold NULL.
            unique("nullable", &["nick"]),
            unique("mixed", &["email", "nick"]),
            // An expression is not a column.
            unique("expression", &["<expression>"]),
            unique("lowered", &["lower(email)"]),
            unique("empty", &[]),
        ] {
            let name = index.name.clone();
            assert_eq!(key(vec![index]), None, "{name}");
        }
        assert_eq!(key(Vec::new()), None);
        // The next usable one is taken.
        assert_eq!(
            key(vec![unique("a", &["nick"]), unique("b", &["code"])]),
            Some(vec!["code".to_owned()])
        );
    }
}
```

- [ ] **Step 2: Run and see them fail**

Run: `~/.cargo/bin/cargo test --locked -p tabletist-db --lib catalog`
Expected: does not compile: no `row_key`.

- [ ] **Step 3: The rule**

In `catalog.rs`:

```rust
impl Structure {
    /// The columns that tell one row from every other, for a save to find
    /// the row by: the primary key, else the first unique index (by name)
    /// that covers every row and whose entries are all columns that cannot
    /// be NULL. `None` when the table has neither: a row there cannot be
    /// targeted safely.
    pub fn row_key(&self) -> Option<Vec<String>> {
        if !self.primary_key.is_empty() {
            return Some(self.primary_key.clone());
        }
        let mut candidates: Vec<&IndexInfo> = self
            .indexes
            .iter()
            .filter(|index| index.unique && !index.partial && !index.columns.is_empty())
            .collect();
        candidates.sort_by(|a, b| a.name.cmp(&b.name));
        candidates.into_iter().find_map(|index| {
            index
                .columns
                .iter()
                .map(|entry| {
                    let column = self.columns.iter().find(|column| {
                        // PostgreSQL gives an index's column as it would be
                        // written, so one that needs quotes comes quoted.
                        column.name == *entry
                            || format!("\"{}\"", column.name.replace('"', "\"\"")) == *entry
                    })?;
                    (!column.nullable).then(|| column.name.clone())
                })
                .collect()
        })
    }
}
```

- [ ] **Step 4: The drivers, test first**

SQLite, in the test module of `sqlite.rs`: a file with `CREATE TABLE t (a INTEGER NOT NULL, b INTEGER); CREATE UNIQUE INDEX whole ON t (a); CREATE UNIQUE INDEX part ON t (b) WHERE b IS NOT NULL;`, described through `Conn::describe`, expecting `whole` not partial and `part` partial, and `row_key()` to be `["a"]`. PostgreSQL, in `tests/postgres.rs`: the same two indexes on a table `catalog_partial` made through `admin()`, same expectations. Run them: FAIL (`part` is not partial).

Then:
- **SQLite**, `indexes`: `SELECT name, "unique", origin, partial FROM pragma_index_list(?1, ?2) ORDER BY name`, and `partial: partial != 0`.
- **PostgreSQL**, `describe`: add `i.indpred IS NOT NULL` as a sixth output column of the index query and read it as `partial: column(row, 5)?`. And an index that is not valid (`i.indisvalid` is false: a `CREATE UNIQUE INDEX CONCURRENTLY` that failed) does not hold its rows unique: select `i.indisunique AND i.indisvalid` where the query selects `i.indisunique`, with a comment saying why.
- **MySQL**, `describe`: `partial: false` (MySQL and MariaDB have no partial indexes).

- [ ] **Step 5: Run and commit**

Run: `~/.cargo/bin/cargo test --locked --workspace --all-targets`
Expected: PASS.

```bash
git add -A && git commit -m "Find the columns that tell one row from another"
```

---

### Task 3: Column classes

> **As built:** `ColumnClass` also has `Binary` (MySQL `bit`, `binary`, `varbinary` and the blobs; PostgreSQL `bytea`; SQLite blob affinity), and it is `Copy`. A type's arguments count only when all of them are whole numbers from zero up (`numeric(5,-2)` states no digits). On MySQL the first word decides (`float unsigned` is a float; `tinyint(1) unsigned zerofill` is an integer, not the boolean alias). On SQLite only real integer type names are `Integer`; any other name with numeric affinity (`FLOATING POINT`, `NUMERIC(10,2)`) is a `Decimal` without limits, since integer affinity stores a real unchanged. PostgreSQL `bpchar` is text without a limit.

**Files:**
- Create: `crates/tabletist-db/src/class.rs`
- Modify: `crates/tabletist-db/src/lib.rs` (`mod class;`, `pub use class::{ColumnClass, column_class};`)

What a column takes, read from the type name the Structure view shows (`ColumnInfo::type_name`): PostgreSQL's `format_type` (`numeric(14,2)`, `character varying(200)`), MySQL's `column_type` (`bigint unsigned`, `tinyint(1)`), SQLite's declared type. The statement builder uses it to decide a value's form (task 5), and step 3's checks will use it for their messages.

- [ ] **Step 1: Write the failing test**

`crates/tabletist-db/src/class.rs`, tests first:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use ColumnClass::{Boolean, Decimal, Float, Integer, Json, Other, Text};

    fn int(min: i128, max: i128) -> ColumnClass {
        Integer { min, max }
    }

    #[test]
    fn postgres_types_have_classes() {
        for (name, class) in [
            ("smallint", int(-32_768, 32_767)),
            ("integer", int(i128::from(i32::MIN), i128::from(i32::MAX))),
            ("bigint", int(i128::from(i64::MIN), i128::from(i64::MAX))),
            ("numeric(14,2)", Decimal { precision: Some(14), scale: Some(2) }),
            ("numeric(5)", Decimal { precision: Some(5), scale: Some(0) }),
            ("numeric", Decimal { precision: None, scale: None }),
            ("real", Float),
            ("double precision", Float),
            ("boolean", Boolean),
            ("json", Json),
            ("jsonb", Json),
            ("text", Text { max_chars: None }),
            ("character varying", Text { max_chars: None }),
            ("character varying(200)", Text { max_chars: Some(200) }),
            ("character(5)", Text { max_chars: Some(5) }),
            ("timestamp with time zone", Other),
            ("text[]", Other),
            ("integer[]", Other),
            ("mood", Other),
            ("uuid", Other),
        ] {
            assert_eq!(column_class(Dialect::Postgres, name), class, "{name}");
        }
    }

    #[test]
    fn mysql_types_have_classes() {
        for (name, class) in [
            ("tinyint(1)", Boolean),
            ("tinyint", int(-128, 127)),
            ("tinyint(4)", int(-128, 127)),
            ("tinyint unsigned", int(0, 255)),
            ("smallint", int(-32_768, 32_767)),
            ("mediumint unsigned", int(0, 16_777_215)),
            ("int", int(i128::from(i32::MIN), i128::from(i32::MAX))),
            ("int(11)", int(i128::from(i32::MIN), i128::from(i32::MAX))),
            ("int unsigned", int(0, i128::from(u32::MAX))),
            ("bigint", int(i128::from(i64::MIN), i128::from(i64::MAX))),
            ("bigint unsigned", int(0, i128::from(u64::MAX))),
            ("BIGINT UNSIGNED", int(0, i128::from(u64::MAX))),
            ("decimal(14,2)", Decimal { precision: Some(14), scale: Some(2) }),
            ("float", Float),
            ("double", Float),
            ("json", Json),
            ("varchar(255)", Text { max_chars: Some(255) }),
            ("char(3)", Text { max_chars: Some(3) }),
            ("text", Text { max_chars: None }),
            ("longtext", Text { max_chars: None }),
            ("datetime(6)", Other),
            ("enum('happy','sad')", Other),
            ("varbinary(16)", Other),
            ("bit(1)", Other),
        ] {
            assert_eq!(column_class(Dialect::MySql, name), class, "{name}");
        }
    }

    #[test]
    fn sqlite_types_have_the_class_of_their_affinity() {
        let whole = int(i128::from(i64::MIN), i128::from(i64::MAX));
        for (name, class) in [
            ("INTEGER", whole.clone()),
            ("bigint", whole.clone()),
            ("INT UNSIGNED", whole),
            ("BOOLEAN", Boolean),
            ("JSON", Json),
            ("REAL", Float),
            ("double precision", Float),
            // SQLite enforces neither digits nor a length.
            ("NUMERIC(10,2)", Decimal { precision: None, scale: None }),
            ("DECIMAL", Decimal { precision: None, scale: None }),
            ("TEXT", Text { max_chars: None }),
            ("VARCHAR(255)", Text { max_chars: None }),
            ("DATETIME", Other),
            ("BLOB", Other),
            ("", Other),
        ] {
            assert_eq!(column_class(Dialect::Sqlite, name), class, "{name}");
        }
    }
}
```

- [ ] **Step 2: Run and see it fail**

Run: `~/.cargo/bin/cargo test --locked -p tabletist-db --lib class`
Expected: does not compile: no `ColumnClass`.

- [ ] **Step 3: Implement**

```rust
//! What a column takes, read from its type's name: which values a save
//! may send it, and in what form.

use crate::{Dialect, ValueKind};

/// A column's class. A type the app does not know is `Other`: the database
/// alone says what it takes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ColumnClass {
    /// Whole numbers from `min` to `max`.
    Integer { min: i128, max: i128 },
    /// Exact numbers, with the digits and the scale the type states.
    Decimal {
        precision: Option<u32>,
        scale: Option<u32>,
    },
    Float,
    Boolean,
    Json,
    /// Text, at most `max_chars` characters when the type says so.
    Text { max_chars: Option<u32> },
    Other,
}

/// The class of a column whose type the catalog names `type_name`
/// (`ColumnInfo::type_name`).
pub fn column_class(dialect: Dialect, type_name: &str) -> ColumnClass {
    let name = type_name.trim().to_ascii_lowercase();
    match dialect {
        Dialect::Sqlite => sqlite(&name),
        Dialect::Postgres => postgres(&name),
        Dialect::MySql => mysql(&name),
    }
}

/// The numbers in a type's parentheses: `numeric(14,2)` gives `[14, 2]`.
fn arguments(name: &str) -> Vec<u32> {
    let Some((_, rest)) = name.split_once('(') else {
        return Vec::new();
    };
    rest.split(')')
        .next()
        .unwrap_or_default()
        .split(',')
        .filter_map(|argument| argument.trim().parse().ok())
        .collect()
}

/// The type's name without its parentheses and what follows them.
fn base(name: &str) -> &str {
    name.split('(').next().unwrap_or_default().trim()
}

fn signed(bytes: u32) -> ColumnClass {
    let max = (1_i128 << (bytes * 8 - 1)) - 1;
    ColumnClass::Integer { min: -max - 1, max }
}

fn unsigned(bytes: u32) -> ColumnClass {
    ColumnClass::Integer {
        min: 0,
        max: (1_i128 << (bytes * 8)) - 1,
    }
}

fn decimal(name: &str) -> ColumnClass {
    let arguments = arguments(name);
    ColumnClass::Decimal {
        precision: arguments.first().copied(),
        // Digits without a scale: none after the point.
        scale: arguments
            .get(1)
            .copied()
            .or(arguments.first().map(|_| 0)),
    }
}

fn postgres(name: &str) -> ColumnClass {
    // An array of anything is the database's to judge.
    if name.ends_with("[]") {
        return ColumnClass::Other;
    }
    match base(name) {
        "smallint" => signed(2),
        "integer" => signed(4),
        "bigint" => signed(8),
        "numeric" => decimal(name),
        "real" | "double precision" => ColumnClass::Float,
        "boolean" => ColumnClass::Boolean,
        "json" | "jsonb" => ColumnClass::Json,
        "text" => ColumnClass::Text { max_chars: None },
        "character varying" | "character" => ColumnClass::Text {
            max_chars: arguments(name).first().copied(),
        },
        _ => ColumnClass::Other,
    }
}

fn mysql(name: &str) -> ColumnClass {
    // A boolean is a tinyint of display width one.
    if name == "tinyint(1)" {
        return ColumnClass::Boolean;
    }
    let bytes = match base(name).split(' ').next().unwrap_or_default() {
        "tinyint" => Some(1),
        "smallint" => Some(2),
        "mediumint" => Some(3),
        "int" => Some(4),
        "bigint" => Some(8),
        _ => None,
    };
    if let Some(bytes) = bytes {
        return if name.contains("unsigned") {
            unsigned(bytes)
        } else {
            signed(bytes)
        };
    }
    match base(name) {
        "decimal" => decimal(name),
        "float" | "double" => ColumnClass::Float,
        "json" => ColumnClass::Json,
        "varchar" | "char" => ColumnClass::Text {
            max_chars: arguments(name).first().copied(),
        },
        "tinytext" | "text" | "mediumtext" | "longtext" => ColumnClass::Text { max_chars: None },
        _ => ColumnClass::Other,
    }
}

/// By the affinity SQLite gives the declared type, with JSON, boolean and
/// date and time names told apart first, as `ValueKind` tells them.
fn sqlite(name: &str) -> ColumnClass {
    match ValueKind::from_sqlite_decl(name) {
        ValueKind::Json => ColumnClass::Json,
        ValueKind::Bool => ColumnClass::Boolean,
        ValueKind::Text => ColumnClass::Text { max_chars: None },
        ValueKind::Numeric if name.contains("int") => signed(8),
        ValueKind::Numeric
            if ["real", "floa", "doub"].iter().any(|word| name.contains(word)) =>
        {
            ColumnClass::Float
        }
        ValueKind::Numeric => ColumnClass::Decimal {
            precision: None,
            scale: None,
        },
        ValueKind::Temporal | ValueKind::Binary | ValueKind::Other => ColumnClass::Other,
    }
}
```

`mediumint` is three bytes: `signed(3)` and `unsigned(3)` give its range by the same arithmetic. Check `ValueKind::from_sqlite_decl` (in `value.rs`) against the SQLite test table before trusting the mapping; it upper-cases its argument itself.

- [ ] **Step 4: Run and commit**

Run: `~/.cargo/bin/cargo test --locked -p tabletist-db --lib class` then the workspace checks.
Expected: PASS.

```bash
git add -A && git commit -m "Read from a type's name what its column takes"
```

---

### Task 4: What a save asks for, and the front door

> **As built:** as below, and `ChangeSet::check` also refuses a set that names the same row twice (decision 6). The steps below are the first draft.

**Files:**
- Create: `crates/tabletist-db/src/write.rs`
- Modify: `crates/tabletist-db/src/error.rs` (`Error::ReadOnly`), `crates/tabletist-db/src/lib.rs` (`mod write;`, exports, `Connection::write`)
- Test: the test module of `write.rs`, `crates/tabletist-db/tests/sqlite.rs`

- [ ] **Step 1: Write the failing tests**

In `write.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn cell(column: &str) -> CellChange {
        CellChange {
            column: column.into(),
            type_name: "text".into(),
            loaded: Value::Text("old".into()),
            new: NewValue::Text("new".into()),
        }
    }

    fn row(key: Vec<(&str, Value)>, set: Vec<CellChange>) -> RowChange {
        RowChange {
            key: key
                .into_iter()
                .map(|(name, value)| (name.to_owned(), value))
                .collect(),
            set,
        }
    }

    fn set(rows: Vec<RowChange>) -> ChangeSet {
        ChangeSet {
            object: ObjectRef::new("main", "users"),
            rows,
        }
    }

    #[test]
    fn a_set_that_can_be_written_passes() {
        let changes = set(vec![row(vec![("id", Value::Int(1))], vec![cell("name")])]);
        assert_eq!(changes.check(), Ok(()));
    }

    #[test]
    fn a_set_that_cannot_be_written_is_refused_with_its_reason() {
        let id = || vec![("id", Value::Int(1))];
        for (changes, said) in [
            (set(Vec::new()), "nothing to save"),
            (set(vec![row(Vec::new(), vec![cell("name")])]), "no key"),
            (set(vec![row(id(), Vec::new())]), "no change"),
            (
                set(vec![row(vec![("id", Value::Null)], vec![cell("name")])]),
                "key is NULL",
            ),
            (set(vec![row(id(), vec![cell("id")])]), "part of the row's key"),
            (
                set(vec![row(id(), vec![cell("name"), cell("name")])]),
                "changed twice",
            ),
        ] {
            let refused = changes.check().unwrap_err().to_string();
            assert!(refused.contains(said), "{said}: {refused}");
        }
    }

}
```

In `crates/tabletist-db/tests/sqlite.rs` (import `CellChange`, `ChangeSet`, `NewValue`, `RowChange`):

```rust
fn rename(id: i64, loaded: &str, new: &str) -> ChangeSet {
    ChangeSet {
        object: ObjectRef::new("main", "users"),
        rows: vec![RowChange {
            key: vec![("id".into(), Value::Int(id))],
            set: vec![CellChange {
                column: "name".into(),
                type_name: "TEXT".into(),
                loaded: Value::Text(loaded.into()),
                new: NewValue::Text(new.into()),
            }],
        }],
    }
}

#[tokio::test]
async fn a_read_only_connection_refuses_a_save_before_it_reads_it() {
    let (connection, dir) = fixture().await;
    let before = std::fs::read(dir.path().join("fixture.db")).unwrap();
    assert_eq!(
        connection.write(&rename(1, "Ada Lovelace", "Grace")).await,
        Err(Error::ReadOnly)
    );
    // Not even looked at: a set that could never be written gets the same.
    let empty = ChangeSet {
        object: ObjectRef::new("main", "users"),
        rows: Vec::new(),
    };
    assert_eq!(connection.write(&empty).await, Err(Error::ReadOnly));
    assert_eq!(std::fs::read(dir.path().join("fixture.db")).unwrap(), before);
}

#[tokio::test]
async fn a_writable_connection_refuses_a_set_it_cannot_write() {
    let (connection, _dir) = fixture_as(Access::Writable).await;
    let empty = ChangeSet {
        object: ObjectRef::new("main", "users"),
        rows: Vec::new(),
    };
    assert!(matches!(
        connection.write(&empty).await,
        Err(Error::Query { .. })
    ));
}
```

(The fixture's first user is `(1, 'ada@example.com', 'Ada Lovelace', ..)`.)

- [ ] **Step 2: Run and see them fail**

Run: `~/.cargo/bin/cargo test --locked -p tabletist-db --lib write`
Expected: does not compile: no `ChangeSet`.

- [ ] **Step 3: The types and the checks**

`crates/tabletist-db/src/write.rs`:

```rust
//! Changing rows: what a save asks for, and what came of it.

use std::time::Duration;

use crate::{Error, ObjectRef, Result, Value};

/// Every change of one save, to one table. Written in one transaction, or
/// not at all.
#[derive(Debug, Clone, PartialEq)]
pub struct ChangeSet {
    pub object: ObjectRef,
    pub rows: Vec<RowChange>,
}

/// The changes to one row.
#[derive(Debug, Clone, PartialEq)]
pub struct RowChange {
    /// The row key's columns (`Structure::row_key`) and their loaded
    /// values: what finds the row.
    pub key: Vec<(String, Value)>,
    pub set: Vec<CellChange>,
}

/// One cell's change.
#[derive(Debug, Clone, PartialEq)]
pub struct CellChange {
    pub column: String,
    /// The structure's type name (`ColumnInfo::type_name`), which decides
    /// how `new` is sent.
    pub type_name: String,
    /// What the page held. A save writes only while the row still does.
    pub loaded: Value,
    pub new: NewValue,
}

/// What a cell becomes: NULL, or text the database turns into the column's
/// type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NewValue {
    Null,
    Text(String),
}

/// How a save ended. Only `Written` changed anything.
#[derive(Debug, Clone, PartialEq)]
pub enum WriteOutcome {
    /// Each row as the database now holds it, in the set's order.
    Written {
        rows: Vec<Vec<Value>>,
        elapsed: Duration,
    },
    /// Rows that are gone, or whose changed columns no longer hold what
    /// the page loaded. Nothing was written.
    Conflicts(Vec<Conflict>),
    /// The statement of `rows[row]` could not be built or failed. Nothing
    /// was written.
    Failed { row: usize, error: Error },
}

/// A row a save found changed since it was loaded.
#[derive(Debug, Clone, PartialEq)]
pub struct Conflict {
    /// Its place in the set's rows.
    pub row: usize,
    /// The row as the database holds it now; `None` when it is gone.
    pub server: Option<Vec<Value>>,
}

impl ChangeSet {
    /// Refuses a set that cannot be written as it stands, before anything
    /// is sent: a statement without a key would touch every row.
    pub fn check(&self) -> Result<()> {
        if self.rows.is_empty() {
            return Err(Error::query("there is nothing to save"));
        }
        for row in &self.rows {
            if row.key.is_empty() {
                return Err(Error::query("a row to save has no key"));
            }
            if row.set.is_empty() {
                return Err(Error::query("a row to save has no change"));
            }
            if row.key.iter().any(|(_, value)| value.is_null()) {
                return Err(Error::query("a row to save cannot be found: its key is NULL"));
            }
            for (index, change) in row.set.iter().enumerate() {
                if row.key.iter().any(|(column, _)| *column == change.column) {
                    return Err(Error::query(format!(
                        "{} is part of the row's key and cannot be changed",
                        change.column
                    )));
                }
                if row.set[..index]
                    .iter()
                    .any(|earlier| earlier.column == change.column)
                {
                    return Err(Error::query(format!(
                        "{} is changed twice in one row",
                        change.column
                    )));
                }
            }
        }
        Ok(())
    }
}

```

`error.rs`, beside `Refused`:

```rust
    /// A save was asked of a connection that opens read-only. Nothing was
    /// sent.
    #[error("this connection opens read-only")]
    ReadOnly,
```

- [ ] **Step 4: The front door**

`lib.rs`: `mod write;`, `pub use write::{CellChange, ChangeSet, Conflict, NewValue, RowChange, WriteOutcome};`, and in `impl Connection`:

```rust
    /// Writes `changes` in one transaction, or nothing: the crate's only
    /// writing call. Each row is found by its key, locked, and compared
    /// with what the page loaded in the columns the save changes; a row
    /// that differs or is gone makes the whole save a conflict.
    ///
    /// On a read-only connection it is refused before the set is even
    /// looked at. The future must be awaited to its end and never dropped:
    /// a save dropped mid-way would leave its transaction open on the
    /// session. The backend awaits every command to its end, and stops a
    /// save through the session's cancel.
    pub async fn write(&self, changes: &ChangeSet) -> Result<WriteOutcome> {
        if self.access == Access::ReadOnly {
            return Err(Error::ReadOnly);
        }
        changes.check()?;
        match &self.inner {
            Inner::Sqlite(_) | Inner::Postgres(_) | Inner::MySql(_) => Err(Error::Unsupported(
                "saving is not built for this database yet",
            )),
        }
    }
```

(Tasks 6 to 8 replace one arm each.)

- [ ] **Step 5: Run and commit**

Run: `~/.cargo/bin/cargo test --locked -p tabletist-db`
Expected: PASS.

```bash
git add -A && git commit -m "Say what a save asks for and refuse it where it cannot be written"
```

---

### Task 5: The statement builder

> **As built:** as below, and two SQLite literals changed so that the shown text runs when a person pastes it: text holding a NUL byte is shown joined around `char(0)`, as `('a' || char(0) || 'b')` (SQLite's parser stops a quoted string at the NUL, and a cast of the text's bytes would be read in the file's encoding, which stores other text in a UTF-16 file), and an infinite float as `9e999` or `-9e999` (`inf` reads as a column's name). A test in `tests/sqlite.rs` runs the shown text on one file and the save on another, in a UTF-8 and a UTF-16 file, and compares what each stored; the unit test here only proves the two forms take the same values in the same order.

**Files:**
- Modify: `crates/tabletist-db/src/dialect.rs`
- Test: its test module

One builder makes, for a row's change, the `UPDATE` as a user reads it (values as literals) and the statement the driver runs (PostgreSQL: the same text; MySQL and SQLite: placeholders with the values bound). It also decides each value's form, by the column's class.

- [ ] **Step 1: Write the failing tests**

In the test module of `dialect.rs`:

```rust
    use crate::{CellChange, NewValue, RowChange};

    fn change(column: &str, type_name: &str, new: NewValue) -> CellChange {
        CellChange {
            column: column.into(),
            type_name: type_name.into(),
            loaded: Value::Null,
            new,
        }
    }

    fn typed(column: &str, type_name: &str, new: &str) -> CellChange {
        change(column, type_name, NewValue::Text(new.into()))
    }

    fn one(key: Vec<(&str, Value)>, set: Vec<CellChange>) -> RowChange {
        RowChange {
            key: key
                .into_iter()
                .map(|(name, value)| (name.to_owned(), value))
                .collect(),
            set,
        }
    }

    fn books() -> ObjectRef {
        ObjectRef::new("public", "books")
    }

    /// The bound statement with each parameter written in place of its
    /// placeholder, as the builder would show it.
    fn inlined(dialect: Dialect, sql: &Sql) -> String {
        let mut params = sql.params.iter();
        let mut text = String::new();
        for (index, piece) in sql.text.split('?').enumerate() {
            if index > 0 {
                text.push_str(&match params.next().expect("a value for each placeholder") {
                    Value::Int(number) => number.to_string(),
                    Value::Float(number) => format!("{number:?}"),
                    Value::Text(value) => dialect.literal(value),
                    Value::Bytes(bytes) => dialect.bytes_literal(bytes),
                    other => panic!("{other:?} is never bound"),
                });
            }
            text.push_str(piece);
        }
        assert!(params.next().is_none(), "a value without a placeholder");
        text
    }

    #[test]
    fn an_update_is_shown_with_its_values_as_literals() {
        let row = one(
            vec![("id", Value::Int(2))],
            vec![
                typed("kind", "character varying(20)", "ebook"),
                change("alt_text", "text", NewValue::Null),
            ],
        );
        let update = Dialect::Postgres.update_row(&books(), &row).unwrap();
        assert_eq!(
            update.shown,
            r#"UPDATE "public"."books" SET "kind" = 'ebook', "alt_text" = NULL WHERE "id" = 2"#
        );
        // PostgreSQL runs exactly what it shows.
        assert_eq!(update.sql.text, update.shown);
        assert!(update.sql.params.is_empty());
        let update = Dialect::MySql.update_row(&books(), &row).unwrap();
        assert_eq!(
            update.shown,
            "UPDATE `public`.`books` SET `kind` = 'ebook', `alt_text` = NULL WHERE `id` = 2"
        );
        assert_eq!(
            update.sql.text,
            "UPDATE `public`.`books` SET `kind` = ?, `alt_text` = NULL WHERE `id` = ?"
        );
        assert_eq!(update.sql.params, [text("ebook"), Value::Int(2)]);
    }

    #[test]
    fn what_is_shown_is_what_is_bound() {
        let row = one(
            vec![
                ("id", Value::Int(7)),
                ("code", text("it's")),
                ("uid", Value::Bytes(vec![0x01, 0xab].into())),
            ],
            vec![
                typed("title", "TEXT", "O'Brien \\ co"),
                typed("pages", "INTEGER", "612"),
                typed("price", "REAL", "12.5"),
                typed("in_print", "BOOLEAN", "true"),
                change("note", "TEXT", NewValue::Null),
            ],
        );
        for dialect in [Dialect::MySql, Dialect::Sqlite] {
            let row = match dialect {
                // MySQL's own names for the same columns.
                Dialect::MySql => one(
                    row.key
                        .iter()
                        .map(|(name, value)| (name.as_str(), value.clone()))
                        .collect(),
                    vec![
                        typed("title", "varchar(200)", "O'Brien \\ co"),
                        typed("pages", "int", "612"),
                        typed("price", "double", "12.5"),
                        typed("in_print", "tinyint(1)", "true"),
                        change("note", "text", NewValue::Null),
                    ],
                ),
                _ => row.clone(),
            };
            let update = dialect.update_row(&books(), &row).unwrap();
            assert_eq!(inlined(dialect, &update.sql), update.shown, "{dialect:?}");
            let select = dialect.select_row(&books(), &row.key, true);
            assert_eq!(select.params.len(), 3, "{dialect:?}");
        }
    }

    #[test]
    fn a_values_form_follows_its_columns_class() {
        let shown = |dialect: Dialect, cell: CellChange| {
            dialect
                .update_row(&books(), &one(vec![("id", Value::Int(1))], vec![cell]))
                .map(|update| update.shown)
        };
        let set = |dialect: Dialect, cell: CellChange| {
            let shown = shown(dialect, cell).unwrap();
            let start = shown.find(" = ").unwrap() + 3;
            shown[start..shown.find(" WHERE").unwrap()].to_owned()
        };
        // PostgreSQL converts text itself, whatever the type.
        assert_eq!(set(Dialect::Postgres, typed("n", "integer", "12")), "'12'");
        assert_eq!(set(Dialect::Postgres, typed("b", "boolean", "true")), "'true'");
        // A backslash needs the escape form there, and only then.
        assert_eq!(set(Dialect::Postgres, typed("t", "text", r"a\b")), r"E'a\\b'");
        assert_eq!(set(Dialect::Postgres, typed("t", "text", "it's")), "'it''s'");
        // MySQL converts text too, but a boolean is 1 or 0.
        assert_eq!(set(Dialect::MySql, typed("n", "int", "12")), "'12'");
        assert_eq!(set(Dialect::MySql, typed("b", "tinyint(1)", "false")), "0");
        assert_eq!(set(Dialect::MySql, typed("b", "tinyint(1)", "1")), "1");
        assert_eq!(set(Dialect::MySql, typed("t", "text", r"a\b'c")), r"'a\\b''c'");
        // SQLite stores what it is given, so numbers go as numbers.
        assert_eq!(set(Dialect::Sqlite, typed("n", "INTEGER", " 12 ")), "12");
        assert_eq!(set(Dialect::Sqlite, typed("x", "REAL", "1")), "1.0");
        assert_eq!(set(Dialect::Sqlite, typed("d", "NUMERIC", "12")), "12");
        assert_eq!(set(Dialect::Sqlite, typed("d", "NUMERIC", "12.50")), "12.5");
        assert_eq!(set(Dialect::Sqlite, typed("b", "BOOLEAN", "TRUE")), "1");
        assert_eq!(set(Dialect::Sqlite, typed("t", "TEXT", "12")), "'12'");
        assert_eq!(set(Dialect::Sqlite, typed("t", "", "12")), "'12'");
        // A column with no type keeps a number a number, where it held one.
        let held = |loaded: Value, new: &str| CellChange {
            loaded,
            ..typed("t", "", new)
        };
        assert_eq!(set(Dialect::Sqlite, held(Value::Int(5), "6")), "6");
        assert_eq!(set(Dialect::Sqlite, held(Value::Float(1.5), "2")), "2");
        assert_eq!(set(Dialect::Sqlite, held(Value::Int(5), "six")), "'six'");
        assert_eq!(set(Dialect::Sqlite, held(Value::Text("5".into()), "6")), "'6'");
        // A MySQL tinyint(1) takes what a tinyint holds.
        assert_eq!(set(Dialect::MySql, typed("b", "tinyint(1)", "true")), "1");
        assert_eq!(set(Dialect::MySql, typed("b", "tinyint(1)", "5")), "5");
        // What cannot be converted is refused, with the column and its type.
        for (dialect, cell) in [
            (Dialect::Sqlite, typed("pages", "INTEGER", "many")),
            (Dialect::Sqlite, typed("pages", "INTEGER", "1.5")),
            (Dialect::Sqlite, typed("price", "REAL", "NaN")),
            (Dialect::Sqlite, typed("in_print", "BOOLEAN", "maybe")),
            (Dialect::MySql, typed("in_print", "tinyint(1)", "yes")),
            (Dialect::MySql, typed("in_print", "tinyint(1)", "128")),
            // Binary columns are never sent as text.
            (Dialect::MySql, typed("flags", "bit(8)", "1")),
            (Dialect::Postgres, typed("cover", "bytea", "x")),
            (Dialect::Sqlite, typed("cover", "BLOB", "x")),
        ] {
            let column = cell.column.clone();
            let refused = shown(dialect, cell).unwrap_err().to_string();
            assert!(refused.starts_with(&column), "{refused}");
        }
    }

    #[test]
    fn a_row_is_found_by_its_key_in_the_drivers_own_form() {
        let key = |value: Value| vec![("id".to_owned(), value)];
        let select = |dialect: Dialect, value: Value, lock: bool| {
            dialect.select_row(&books(), &key(value), lock)
        };
        assert_eq!(
            select(Dialect::Postgres, Value::Int(2), true).text,
            r#"SELECT * FROM "public"."books" WHERE "id" = 2 FOR UPDATE"#
        );
        assert_eq!(
            select(Dialect::Postgres, text("a-b"), false).text,
            r#"SELECT * FROM "public"."books" WHERE "id" = 'a-b'"#
        );
        assert_eq!(
            select(Dialect::Postgres, Value::Bytes(vec![0x01, 0xab].into()), false).text,
            r#"SELECT * FROM "public"."books" WHERE "id" = E'\\x01ab'"#
        );
        assert_eq!(
            select(Dialect::Postgres, Value::Bool(true), false).text,
            r#"SELECT * FROM "public"."books" WHERE "id" = 'true'"#
        );
        let mysql = select(Dialect::MySql, Value::Bytes(vec![0x01, 0xab].into()), true);
        assert_eq!(
            mysql.text,
            "SELECT * FROM `public`.`books` WHERE `id` = ? FOR UPDATE"
        );
        assert_eq!(mysql.params, [Value::Bytes(vec![0x01, 0xab].into())]);
        // SQLite has no row locks: its transaction holds the file.
        assert_eq!(
            select(Dialect::Sqlite, Value::Int(2), true).text,
            r#"SELECT * FROM "public"."books" WHERE "id" = ?"#
        );
        // Several columns are all asked for.
        let pair = vec![
            ("a".to_owned(), Value::Int(1)),
            ("b".to_owned(), text("x")),
        ];
        assert_eq!(
            Dialect::Postgres.select_row(&books(), &pair, false).text,
            r#"SELECT * FROM "public"."books" WHERE "a" = 1 AND "b" = 'x'"#
        );
    }
```

- [ ] **Step 2: Run and see them fail**

Run: `~/.cargo/bin/cargo test --locked -p tabletist-db --lib dialect`
Expected: does not compile: no `update_row`.

- [ ] **Step 3: Implement**

In `dialect.rs` (imports: `crate::{CellChange, ColumnClass, Error, NewValue, Result, RowChange, column_class}`):

```rust
/// A row's `UPDATE`, as a user reads it and as the driver runs it. Both
/// come from the same values, so they cannot drift apart.
#[derive(Debug, Clone, PartialEq)]
pub struct RowUpdate {
    /// The statement with its values as literals.
    pub shown: String,
    /// What the driver sends: for PostgreSQL the shown text, for MySQL and
    /// SQLite the same statement with the values bound.
    pub sql: Sql,
}

/// A value as a statement holds it. Decided once, here, so the literal a
/// user reads is the value the driver sends.
#[derive(Debug, Clone, PartialEq)]
enum Operand {
    Null,
    Int(i64),
    Float(f64),
    Text(String),
    Bytes(Vec<u8>),
}
```

and in `impl Dialect`:

```rust
    /// `text` as a string literal. PostgreSQL takes the plain form unless
    /// the text holds a backslash, which only the escape form keeps whatever
    /// `standard_conforming_strings` is. MySQL reads a backslash as an
    /// escape in every string (the session keeps `NO_BACKSLASH_ESCAPES`
    /// off), so it is doubled there.
    pub(crate) fn literal(self, text: &str) -> String {
        match self {
            Self::Postgres if text.contains('\\') => quote_literal(text),
            Self::Postgres | Self::Sqlite => format!("'{}'", text.replace('\'', "''")),
            Self::MySql => format!("'{}'", text.replace('\\', "\\\\").replace('\'', "''")),
        }
    }

    /// Bytes as a literal: PostgreSQL's hex `bytea`, `x'..'` elsewhere.
    pub(crate) fn bytes_literal(self, bytes: &[u8]) -> String {
        let mut hex = String::new();
        for byte in bytes {
            let _ = write!(hex, "{byte:02x}");
        }
        match self {
            Self::Postgres => quote_literal(&format!("\\x{hex}")),
            Self::MySql | Self::Sqlite => format!("x'{hex}'"),
        }
    }

    fn shown(self, operand: &Operand) -> String {
        match operand {
            Operand::Null => "NULL".to_owned(),
            Operand::Int(number) => number.to_string(),
            // With its point, so a real never reads as a whole number.
            Operand::Float(number) => format!("{number:?}"),
            Operand::Text(text) => self.literal(text),
            Operand::Bytes(bytes) => self.bytes_literal(bytes),
        }
    }

    /// The operand in the statement the driver runs: its literal for
    /// PostgreSQL, whose rows go through the simple-query protocol, and a
    /// bound value elsewhere. NULL is written out in both.
    fn sent(self, operand: &Operand, params: &mut Vec<Value>) -> String {
        if self == Self::Postgres {
            return self.shown(operand);
        }
        params.push(match operand {
            Operand::Null => return "NULL".to_owned(),
            Operand::Int(number) => Value::Int(*number),
            Operand::Float(number) => Value::Float(*number),
            Operand::Text(text) => Value::Text(text.as_str().into()),
            Operand::Bytes(bytes) => Value::Bytes(bytes.as_slice().into()),
        });
        self.placeholder().to_owned()
    }

    /// A key's loaded value as the operand that finds its row again.
    /// PostgreSQL converts a literal to the column's type, so everything
    /// but a whole number goes as text there.
    fn key_operand(self, value: &Value) -> Operand {
        match (self, value) {
            (_, Value::Null) => Operand::Null,
            (_, Value::Int(number)) => Operand::Int(*number),
            (_, Value::Text(text)) => Operand::Text(text.to_string()),
            (_, Value::Bytes(bytes)) => Operand::Bytes(bytes.to_vec()),
            (Self::Postgres, Value::Bool(flag)) => Operand::Text(flag.to_string()),
            (Self::Postgres, Value::Float(number)) => Operand::Text(number.to_string()),
            (Self::MySql | Self::Sqlite, Value::Bool(flag)) => Operand::Int(i64::from(*flag)),
            (Self::MySql | Self::Sqlite, Value::Float(number)) => Operand::Float(*number),
        }
    }

    /// A cell's new value as an operand. PostgreSQL and MySQL convert text
    /// to the column's type themselves. Where the database would store the
    /// text as it is, it is converted here, by the column's class: numbers
    /// on SQLite, and a boolean on SQLite and MySQL, which keep one as 1 or
    /// 0. Text that cannot be converted is refused, naming the column.
    fn new_operand(self, change: &CellChange) -> Result<Operand> {
        let NewValue::Text(text) = &change.new else {
            return Ok(Operand::Null);
        };
        let refused = |expects: &str| {
            Error::query(format!(
                "{}: {} expects {expects}",
                change.column,
                if change.type_name.is_empty() {
                    "the column"
                } else {
                    &change.type_name
                }
            ))
        };
        let typed = text.trim();
        let whole = || typed.parse::<i64>().ok().map(Operand::Int);
        let real = || {
            typed
                .parse::<f64>()
                .ok()
                .filter(|number| number.is_finite())
                .map(Operand::Float)
        };
        match (self, column_class(self, &change.type_name)) {
            // Never as text: MySQL would store a `bit`'s text as the
            // characters' codes.
            (_, ColumnClass::Binary) => Err(Error::query(format!(
                "{}: binary values cannot be edited yet",
                change.column
            ))),
            (Self::Postgres, _) => Ok(Operand::Text(text.clone())),
            // A tinyint(1) holds any tinyint, and some tables keep more
            // than a flag in one.
            (Self::MySql, ColumnClass::Boolean) => match typed.to_ascii_lowercase().as_str() {
                "true" => Ok(Operand::Int(1)),
                "false" => Ok(Operand::Int(0)),
                _ => typed
                    .parse::<i8>()
                    .map(|number| Operand::Int(i64::from(number)))
                    .map_err(|_| refused("true, false or a whole number from -128 to 127")),
            },
            (Self::Sqlite, ColumnClass::Boolean) => match typed.to_ascii_lowercase().as_str() {
                "true" | "1" => Ok(Operand::Int(1)),
                "false" | "0" => Ok(Operand::Int(0)),
                _ => Err(refused("true or false")),
            },
            // No declared type, or one SQLite gives no affinity: nothing
            // converts the text, so a number stays a number only where the
            // cell held one.
            (Self::Sqlite, ColumnClass::Other)
                if matches!(change.loaded, Value::Int(_) | Value::Float(_)) =>
            {
                Ok(whole()
                    .or_else(real)
                    .unwrap_or_else(|| Operand::Text(text.clone())))
            }
            (Self::Sqlite, ColumnClass::Integer { .. }) => {
                whole().ok_or_else(|| refused("a whole number"))
            }
            (Self::Sqlite, ColumnClass::Float) => real().ok_or_else(|| refused("a number")),
            (Self::Sqlite, ColumnClass::Decimal { .. }) => whole()
                .or_else(real)
                .ok_or_else(|| refused("a number")),
            _ => Ok(Operand::Text(text.clone())),
        }
    }

    /// ` WHERE "a" = .. AND "b" = ..` for a row's key, shown and sent.
    fn key_clause(self, key: &[(String, Value)], params: &mut Vec<Value>) -> (String, String) {
        let mut shown = Vec::with_capacity(key.len());
        let mut sent = Vec::with_capacity(key.len());
        for (column, value) in key {
            let operand = self.key_operand(value);
            let column = self.quote_ident(column);
            shown.push(format!("{column} = {}", self.shown(&operand)));
            sent.push(format!("{column} = {}", self.sent(&operand, params)));
        }
        (
            format!(" WHERE {}", shown.join(" AND ")),
            format!(" WHERE {}", sent.join(" AND ")),
        )
    }

    /// The `UPDATE` of one row of a save. `Err` names the value that cannot
    /// be sent in its column's form.
    pub fn update_row(self, object: &ObjectRef, row: &RowChange) -> Result<RowUpdate> {
        let mut params = Vec::new();
        let mut shown = Vec::with_capacity(row.set.len());
        let mut sent = Vec::with_capacity(row.set.len());
        for change in &row.set {
            let operand = self.new_operand(change)?;
            let column = self.quote_ident(&change.column);
            shown.push(format!("{column} = {}", self.shown(&operand)));
            sent.push(format!("{column} = {}", self.sent(&operand, &mut params)));
        }
        let (shown_key, sent_key) = self.key_clause(&row.key, &mut params);
        let head = format!("UPDATE {} SET ", self.qualified(object));
        Ok(RowUpdate {
            shown: format!("{head}{}{shown_key}", shown.join(", ")),
            sql: Sql {
                text: format!("{head}{}{sent_key}", sent.join(", ")),
                params,
            },
        })
    }

    /// The row of `key`, whole. With `lock` it is held until the
    /// transaction ends, where the database has row locks.
    pub fn select_row(self, object: &ObjectRef, key: &[(String, Value)], lock: bool) -> Sql {
        let mut params = Vec::new();
        let (_, clause) = self.key_clause(key, &mut params);
        let lock = if lock && self != Self::Sqlite {
            " FOR UPDATE"
        } else {
            ""
        };
        Sql {
            text: format!("SELECT * FROM {}{clause}{lock}", self.qualified(object)),
            params,
        }
    }
```

Export `RowUpdate` from `lib.rs` beside `Sql`. If a test's expectation and the code disagree about a literal's exact text, decide by what the database reads, not by the test.

- [ ] **Step 4: Run and commit**

Run: `~/.cargo/bin/cargo test --locked -p tabletist-db --lib dialect` then the workspace checks.
Expected: PASS.

```bash
git add -A && git commit -m "Build a row's UPDATE once, to be read and to be run"
```

---

### Task 6: SQLite saves

> **As built:** the six steps as below, with what building and the review added.
> - **Names and values not read exactly are refused** (decision 10). The first was found while building: with columns `caf\xE9` and `caf\u{FFFD}` the draft compared one and wrote the other.
> - **More is put back before the transaction** (decision 4): `recursive_triggers` and `count_changes` too, and the journal mode from any mode but WAL.
> - `rusqlite` 0.37 has no `DatabaseName`; the schema of `pragma_update` is `Some("main")`.
> - A key whose name is not UTF-8 is an `Err` (the read by key fails: no SQL can spell the column), not `Failed`. Nothing is written either way.
> - The bundled SQLite reads a double-quoted name that matches no column as an error, not as a string, on the session as the driver opens it. A save counts on that: `WHERE "nosuch" = ?` must fail, not compare a string.
> - A changed column whose stored text is not UTF-8 is `Failed` for its row, not a conflict: a conflict offers to write over what the file holds, which would still be unknown.
> - Tests beyond the draft's: a key matching two rows, an update that changes no row (a view's `INSTEAD OF` trigger), a typeless column keeping its kind of value, a typeless key bound exactly, names and values that are not UTF-8, a file another program holds (in the unit tests of `sqlite.rs`, where the busy timeout can be shortened), a cancel in the middle of a save, each pragma a script can leave.
>
> The steps below are the first draft.

**Files:**
- Create: `crates/tabletist-db/src/sqlite/write.rs`
- Modify: `crates/tabletist-db/src/sqlite.rs` (`mod write;`, `Conn::journal_mode`, `Conn::write`), `crates/tabletist-db/src/lib.rs` (the SQLite arm of `Connection::write`)
- Test: `crates/tabletist-db/tests/sqlite.rs`

The six steps of the spec's "Saving" section, on SQLite: `BEGIN IMMEDIATE` holds the file, so reading a row inside it is reading it locked.

- [ ] **Step 1: Write the failing tests**

First, in the test module of `crates/tabletist-db/src/write.rs`, the tests of the two checks every driver's save shares (they are introduced here, with their first use, so nothing is dead code before it):

```rust
    #[test]
    fn floats_are_the_same_by_their_bits() {
        assert!(same(&Value::Float(f64::NAN), &Value::Float(f64::NAN)));
        assert!(!same(&Value::Float(0.1), &Value::Float(0.2)));
        assert!(same(&Value::Null, &Value::Null));
        assert!(!same(&Value::Int(1), &Value::Text("1".into())));
    }

    #[test]
    fn only_a_changed_column_makes_a_conflict() {
        let change = row(vec![("id", Value::Int(1))], vec![cell("name")]);
        let columns = ["id".to_owned(), "name".to_owned(), "email".to_owned()];
        let server = |name: &str, email: &str| {
            vec![
                Value::Int(1),
                Value::Text(name.into()),
                Value::Text(email.into()),
            ]
        };
        assert_eq!(
            changed_since_loaded(&change, &columns, &server("old", "a@x")),
            Ok(false)
        );
        // Another column changing is not this save's business.
        assert_eq!(
            changed_since_loaded(&change, &columns, &server("old", "b@x")),
            Ok(false)
        );
        assert_eq!(
            changed_since_loaded(&change, &columns, &server("theirs", "a@x")),
            Ok(true)
        );
        // A column the table does not have is an error, not a conflict.
        let gone = row(vec![("id", Value::Int(1))], vec![cell("nick")]);
        assert!(changed_since_loaded(&gone, &columns, &server("old", "a@x")).is_err());
    }
```

Then in `crates/tabletist-db/tests/sqlite.rs`. Helpers first (the fixture's `users` has `id INTEGER PRIMARY KEY, email TEXT NOT NULL UNIQUE, name TEXT, created_at DATETIME NOT NULL, active BOOLEAN NOT NULL, meta JSON, avatar BLOB, score REAL`):

```rust
/// A second handle on a fixture's file, standing in for another program.
fn other_program(dir: &tempfile::TempDir) -> rusqlite::Connection {
    rusqlite::Connection::open(dir.path().join("fixture.db")).unwrap()
}

/// The row of `users` with this id, as a page gives it, and the names of
/// its columns.
async fn user(connection: &Connection, id: i64) -> (Vec<String>, Vec<Value>) {
    let mut query = users(10);
    query.filters.push(Filter {
        column: "id".into(),
        op: FilterOp::Eq,
        value: id.to_string(),
    });
    let page = connection.fetch_rows(&query).await.unwrap();
    (
        page.columns.into_iter().map(|column| column.name).collect(),
        page.rows.into_iter().next().unwrap(),
    )
}

/// A save of one row of `users`: each (column, declared type, new text),
/// with what the page holds now as the loaded value.
async fn save(
    connection: &Connection,
    id: i64,
    cells: &[(&str, &str, NewValue)],
) -> (ChangeSet, tabletist_db::Result<WriteOutcome>) {
    let (columns, row) = user(connection, id).await;
    let set = cells
        .iter()
        .map(|(column, type_name, new)| CellChange {
            column: (*column).into(),
            type_name: (*type_name).into(),
            loaded: row[columns.iter().position(|name| name == column).unwrap()].clone(),
            new: new.clone(),
        })
        .collect();
    let changes = ChangeSet {
        object: ObjectRef::new("main", "users"),
        rows: vec![RowChange {
            key: vec![("id".into(), Value::Int(id))],
            set,
        }],
    };
    let outcome = connection.write(&changes).await;
    (changes, outcome)
}

fn to(text: &str) -> NewValue {
    NewValue::Text(text.into())
}
```

The tests:

```rust
#[tokio::test]
async fn a_save_writes_every_kind_of_value_and_reads_the_row_back() {
    let (connection, _dir) = fixture_as(Access::Writable).await;
    let (columns, before) = user(&connection, 1).await;
    let (_, outcome) = save(
        &connection,
        1,
        &[
            ("email", "TEXT", to("new@example.com")),
            ("name", "TEXT", NewValue::Null),
            ("created_at", "DATETIME", to("2027-02-03 04:05:06")),
            ("active", "BOOLEAN", to("false")),
            ("meta", "JSON", to(r#"{"plan": "pro"}"#)),
            ("score", "REAL", to("12.5")),
        ],
    )
    .await;
    let WriteOutcome::Written { rows, .. } = outcome.unwrap() else {
        panic!("not written");
    };
    // What came back is what a page now shows.
    let (_, after) = user(&connection, 1).await;
    assert_eq!(rows, [after.clone()]);
    let cell = |name: &str| after[columns.iter().position(|column| column == name).unwrap()].clone();
    assert_eq!(cell("email"), Value::Text("new@example.com".into()));
    assert_eq!(cell("name"), Value::Null);
    assert_eq!(cell("active"), Value::Int(0));
    assert_eq!(cell("score"), Value::Float(12.5));
    assert_eq!(cell("id"), before[0]);
    // And the row never conflicts with itself: each value written back
    // from what the page holds now.
    let (_, again) = save(
        &connection,
        1,
        &[
            ("email", "TEXT", to("back@example.com")),
            ("created_at", "DATETIME", to("2026-01-01 00:00:00")),
            ("active", "BOOLEAN", to("1")),
            ("meta", "JSON", NewValue::Null),
            ("score", "REAL", to("0.1")),
        ],
    )
    .await;
    assert!(matches!(again, Ok(WriteOutcome::Written { .. })), "{again:?}");
    // Afterwards the session refuses writes as before.
    assert_eq!(query_only(&connection).await, Value::Int(1));
    let outcome = run(&connection, "UPDATE users SET name = 'x'").await.unwrap();
    assert!(matches!(
        outcome.results[0].outcome,
        StatementOutcome::Error { .. }
    ));
}

#[tokio::test]
async fn a_row_changed_by_someone_else_is_a_conflict_and_nothing_is_written() {
    let (connection, dir) = fixture_as(Access::Writable).await;
    // Loaded, then changed behind the page's back.
    let (columns, row) = user(&connection, 1).await;
    let name = columns.iter().position(|column| column == "name").unwrap();
    let changes = ChangeSet {
        object: ObjectRef::new("main", "users"),
        rows: vec![
            RowChange {
                key: vec![("id".into(), Value::Int(2))],
                set: vec![CellChange {
                    column: "name".into(),
                    type_name: "TEXT".into(),
                    loaded: user(&connection, 2).await.1[name].clone(),
                    new: to("Second"),
                }],
            },
            RowChange {
                key: vec![("id".into(), Value::Int(1))],
                set: vec![CellChange {
                    column: "name".into(),
                    type_name: "TEXT".into(),
                    loaded: row[name].clone(),
                    new: to("Mine"),
                }],
            },
        ],
    };
    other_program(&dir)
        .execute("UPDATE users SET name = 'Theirs' WHERE id = 1", [])
        .unwrap();
    let outcome = connection.write(&changes).await.unwrap();
    let WriteOutcome::Conflicts(conflicts) = outcome else {
        panic!("{outcome:?}");
    };
    assert_eq!(conflicts.len(), 1);
    assert_eq!(conflicts[0].row, 1);
    let server = conflicts[0].server.as_ref().unwrap();
    assert_eq!(server[name], Value::Text("Theirs".into()));
    // The other row of the set was not written either.
    assert_ne!(user(&connection, 2).await.1[name], Value::Text("Second".into()));
    assert_eq!(user(&connection, 1).await.1[name], Value::Text("Theirs".into()));
}

#[tokio::test]
async fn a_change_to_a_column_the_save_leaves_alone_is_no_conflict() {
    let (connection, dir) = fixture_as(Access::Writable).await;
    let (columns, row) = user(&connection, 1).await;
    let name = columns.iter().position(|column| column == "name").unwrap();
    let changes = ChangeSet {
        object: ObjectRef::new("main", "users"),
        rows: vec![RowChange {
            key: vec![("id".into(), Value::Int(1))],
            set: vec![CellChange {
                column: "name".into(),
                type_name: "TEXT".into(),
                loaded: row[name].clone(),
                new: to("Mine"),
            }],
        }],
    };
    other_program(&dir)
        .execute("UPDATE users SET score = 99 WHERE id = 1", [])
        .unwrap();
    assert!(matches!(
        connection.write(&changes).await,
        Ok(WriteOutcome::Written { .. })
    ));
}

#[tokio::test]
async fn a_row_that_is_gone_is_a_conflict_without_a_row() {
    let (connection, dir) = fixture_as(Access::Writable).await;
    let (columns, row) = user(&connection, 5).await;
    let name = columns.iter().position(|column| column == "name").unwrap();
    let changes = ChangeSet {
        object: ObjectRef::new("main", "users"),
        rows: vec![RowChange {
            key: vec![("id".into(), Value::Int(5))],
            set: vec![CellChange {
                column: "name".into(),
                type_name: "TEXT".into(),
                loaded: row[name].clone(),
                new: to("Late"),
            }],
        }],
    };
    // Its orders go first: the fixture has foreign keys.
    let other = other_program(&dir);
    other.execute("DELETE FROM orders WHERE user_id = 5", []).unwrap();
    other.execute("DELETE FROM users WHERE id = 5", []).unwrap();
    assert_eq!(
        connection.write(&changes).await,
        Ok(WriteOutcome::Conflicts(vec![Conflict {
            row: 0,
            server: None
        }]))
    );
}

#[tokio::test]
async fn a_statement_that_fails_undoes_the_rows_before_it() {
    let (connection, _dir) = fixture_as(Access::Writable).await;
    let (columns, first) = user(&connection, 1).await;
    let (_, second) = user(&connection, 2).await;
    let at = |name: &str| columns.iter().position(|column| column == name).unwrap();
    let changes = ChangeSet {
        object: ObjectRef::new("main", "users"),
        rows: vec![
            RowChange {
                key: vec![("id".into(), Value::Int(1))],
                set: vec![CellChange {
                    column: "name".into(),
                    type_name: "TEXT".into(),
                    loaded: first[at("name")].clone(),
                    new: to("Written first"),
                }],
            },
            RowChange {
                key: vec![("id".into(), Value::Int(2))],
                set: vec![CellChange {
                    column: "email".into(),
                    type_name: "TEXT".into(),
                    loaded: second[at("email")].clone(),
                    // NOT NULL: the database refuses it.
                    new: NewValue::Null,
                }],
            },
        ],
    };
    let outcome = connection.write(&changes).await.unwrap();
    assert!(
        matches!(outcome, WriteOutcome::Failed { row: 1, .. }),
        "{outcome:?}"
    );
    assert_eq!(user(&connection, 1).await.1, first);
    assert_eq!(query_only(&connection).await, Value::Int(1));
}

#[tokio::test]
async fn a_value_sqlite_would_store_as_text_is_refused_before_anything_is_sent() {
    let (connection, dir) = fixture_as(Access::Writable).await;
    let before = std::fs::read(dir.path().join("fixture.db")).unwrap();
    let (_, outcome) = save(&connection, 1, &[("active", "BOOLEAN", to("maybe"))]).await;
    assert!(
        matches!(outcome, Ok(WriteOutcome::Failed { row: 0, .. })),
        "{outcome:?}"
    );
    let (_, outcome) = save(&connection, 1, &[("score", "REAL", to("high"))]).await;
    assert!(matches!(outcome, Ok(WriteOutcome::Failed { row: 0, .. })));
    assert_eq!(std::fs::read(dir.path().join("fixture.db")).unwrap(), before);
}

#[tokio::test]
async fn a_save_does_not_inherit_what_a_script_left_on_the_session() {
    let (connection, dir) = fixture_as(Access::Writable).await;
    other_program(&dir)
        .execute_batch(
            "CREATE TABLE kinds (id INTEGER PRIMARY KEY, kind TEXT CHECK (kind IN ('a', 'b')));
             INSERT INTO kinds VALUES (1, 'a');",
        )
        .unwrap();
    let mode = setting(&connection, "journal_mode").await;
    // The fence lets a script set these; they last for the session.
    run(
        &connection,
        "PRAGMA ignore_check_constraints = ON; PRAGMA journal_mode = MEMORY",
    )
    .await
    .unwrap();
    let changes = ChangeSet {
        object: ObjectRef::new("main", "kinds"),
        rows: vec![RowChange {
            key: vec![("id".into(), Value::Int(1))],
            set: vec![CellChange {
                column: "kind".into(),
                type_name: "TEXT".into(),
                loaded: Value::Text("a".into()),
                new: to("z"),
            }],
        }],
    };
    // The CHECK holds all the same, and the journal is the file's own.
    let outcome = connection.write(&changes).await.unwrap();
    assert!(
        matches!(outcome, WriteOutcome::Failed { row: 0, .. }),
        "{outcome:?}"
    );
    assert_eq!(setting(&connection, "journal_mode").await, mode);
}
```

Two more, short:

- `a_save_writes_a_decimal_as_a_number`: `orders.total` is `NUMERIC(10,2)` in the fixture. Save `"19.90"` into one order's `total` (key `id`, type name `NUMERIC(10,2)`) and expect `Written` with the cell read back as `Value::Float(19.9)`; then `"20"` and expect `Value::Int(20)`.
- `a_table_of_an_attached_database_is_refused`: a save whose `object.schema` is not `main` is `Err(Error::Unsupported(..))`, before anything is sent.

(`query_only` and `setting` are helpers already in this file; `Conflict`, `WriteOutcome` and the change types need importing. The fixture has users 1 to 5: user 2 is `Bob`, user 5 has a NULL name and no orders, and `orders` rows belong to users 1 and 3 with `ON DELETE CASCADE`.)

- [ ] **Step 2: Run and see them fail**

Run: `~/.cargo/bin/cargo test --locked -p tabletist-db --test sqlite save`
Expected: FAIL with `Unsupported("saving is not built for this database yet")`.

- [ ] **Step 3: Implement**

In `crates/tabletist-db/src/write.rs`, the two checks the tests above are for:

```rust
/// Whether two values are the same value. Floats by their bits: NaN is NaN,
/// and a conflict is never made up by a comparison.
pub(crate) fn same(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Float(a), Value::Float(b)) => a.to_bits() == b.to_bits(),
        _ => a == b,
    }
}

/// Whether the row as the database holds it (`server`, whose values
/// `columns` name) differs from what the page loaded in a column the save
/// changes. Other columns are not this save's business.
pub(crate) fn changed_since_loaded(
    row: &RowChange,
    columns: &[String],
    server: &[Value],
) -> Result<bool> {
    for change in &row.set {
        let found = columns
            .iter()
            .position(|column| *column == change.column)
            .and_then(|index| server.get(index));
        let Some(now) = found else {
            return Err(Error::query(format!("no such column: {}", change.column)));
        };
        if !same(now, &change.loaded) {
            return Ok(true);
        }
    }
    Ok(false)
}
```

`Conn` gains `/// main's journal mode as the session found it, which a save puts back.` `journal_mode: String`, read in `open` after `set_session_pragmas` with `connection.query_row("PRAGMA main.journal_mode", [], |row| row.get::<_, String>(0))`, before the authorizer is installed. `open`'s blocking closure returns the connection and the fences today; it returns the journal mode with them. In `sqlite.rs`: `mod write;` beside `mod fence;` (a child module reaches its parent's private functions through `super::`, so `end_transaction`, `from_sqlite`, `map_error` and `declared_columns` need no change), and:

```rust
    /// See [`crate::Connection::write`]. One blocking job, so a cancel can
    /// never fall between the save's statements.
    pub async fn write(&self, changes: &ChangeSet) -> Result<WriteOutcome> {
        if changes.object.schema != "main" {
            return Err(Error::Unsupported(
                "saving to an attached database is not built yet",
            ));
        }
        let changes = changes.clone();
        let journal_mode = self.journal_mode.clone();
        self.run(move |connection| write::write(connection, &changes, &journal_mode))
            .await
    }
```

`crates/tabletist-db/src/sqlite/write.rs`:

```rust
//! A save on SQLite: one `BEGIN IMMEDIATE` transaction, with `query_only`
//! lifted for as long as it lasts and no longer.

use std::time::Instant;

use super::{end_transaction, from_sqlite, map_error};
use crate::write::changed_since_loaded;
use crate::dialect::RowUpdate;
use crate::{ChangeSet, Conflict, Dialect, Error, Result, Sql, Value, WriteOutcome};

/// What the statements of a save came to, before its transaction ends.
enum Applied {
    Rows(Vec<Vec<Value>>),
    Conflicts(Vec<Conflict>),
    Failed { row: usize, error: Error },
}

pub(super) fn write(
    connection: &rusqlite::Connection,
    changes: &ChangeSet,
    journal_mode: &str,
) -> Result<WriteOutcome> {
    // Every statement is built first: a value that cannot be sent fails
    // the save before the file is even asked for.
    let mut updates = Vec::with_capacity(changes.rows.len());
    for (row, change) in changes.rows.iter().enumerate() {
        match Dialect::Sqlite.update_row(&changes.object, change) {
            Ok(update) => updates.push(update),
            Err(error) => return Ok(WriteOutcome::Failed { row, error }),
        }
    }
    let started = Instant::now();
    let applied =
        begin(connection, journal_mode).and_then(|()| apply(connection, changes, &updates));
    let committed = match &applied {
        Ok(Applied::Rows(_)) => connection.execute_batch("COMMIT").map_err(map_error),
        _ => Ok(()),
    };
    // Whatever is still open is undone (a conflict, a failure, a COMMIT
    // that did not go through), and the session refuses writes again. A
    // session that cannot be put back is closed: it may still be able to
    // write.
    let closed = end_transaction(connection).map_err(|error| {
        Error::ConnectionLost(format!("could not end the save's transaction: {error}"))
    });
    let applied = applied.and_then(|applied| committed.map(|()| applied));
    closed?;
    Ok(match applied? {
        Applied::Rows(rows) => WriteOutcome::Written {
            rows,
            elapsed: started.elapsed(),
        },
        Applied::Conflicts(conflicts) => WriteOutcome::Conflicts(conflicts),
        Applied::Failed { row, error } => WriteOutcome::Failed { row, error },
    })
}

/// Puts back what a script may have left on the session and a write would
/// feel (a journal kept in memory, exclusive locking, CHECK constraints
/// ignored), lifts `query_only`, and takes the file.
fn begin(connection: &rusqlite::Connection, journal_mode: &str) -> Result<()> {
    // `main` only, and only from `memory`, which is what a script can leave
    // (`off` is refused in defensive mode). With no schema the pragma would
    // set every attached database's mode too, and any other mode is one
    // another program gave the file, not ours to undo.
    let now: String = connection
        .query_row("PRAGMA main.journal_mode", [], |row| row.get(0))
        .map_err(map_error)?;
    if now.eq_ignore_ascii_case("memory") && !journal_mode.eq_ignore_ascii_case("memory") {
        connection
            .pragma_update(Some(rusqlite::DatabaseName::Main), "journal_mode", journal_mode)
            .map_err(map_error)?;
    }
    connection
        .pragma_update(None, "locking_mode", "NORMAL")
        .and_then(|()| connection.pragma_update(None, "ignore_check_constraints", false))
        .and_then(|()| connection.pragma_update(None, "query_only", false))
        .and_then(|()| connection.execute_batch("BEGIN IMMEDIATE"))
        .map_err(map_error)
}

/// A value bound exactly as it is. The filter path turns text that reads
/// as a number into one (`to_sqlite`); a save does not guess.
fn exact(value: &Value) -> rusqlite::types::Value {
    use rusqlite::types::Value as Sqlite;
    match value {
        Value::Null => Sqlite::Null,
        Value::Bool(flag) => Sqlite::Integer(i64::from(*flag)),
        Value::Int(number) => Sqlite::Integer(*number),
        Value::Float(number) => Sqlite::Real(*number),
        Value::Text(text) => Sqlite::Text(text.to_string()),
        Value::Bytes(bytes) => Sqlite::Blob(bytes.to_vec()),
    }
}

/// The rows of a statement, with their columns' names.
fn read(connection: &rusqlite::Connection, sql: &Sql) -> Result<(Vec<String>, Vec<Vec<Value>>)> {
    let mut statement = connection.prepare(&sql.text).map_err(map_error)?;
    // Through the driver's own reader, never rusqlite's: a name that is
    // not UTF-8 panics there.
    let columns: Vec<String> = super::declared_columns(connection, &statement, &sql.text)?
        .into_iter()
        .map(|(name, _)| name)
        .collect();
    let mut rows = statement
        .query(rusqlite::params_from_iter(sql.params.iter().map(exact)))
        .map_err(map_error)?;
    let mut values = Vec::new();
    while let Some(row) = rows.next().map_err(map_error)? {
        let mut cells = Vec::with_capacity(columns.len());
        for index in 0..columns.len() {
            cells.push(from_sqlite(row.get_ref(index).map_err(map_error)?));
        }
        values.push(cells);
    }
    Ok((columns, values))
}

fn apply(
    connection: &rusqlite::Connection,
    changes: &ChangeSet,
    updates: &[RowUpdate],
) -> Result<Applied> {
    let dialect = Dialect::Sqlite;
    // The transaction holds the file, so a row read here is the row the
    // update will find.
    let mut conflicts = Vec::new();
    for (row, change) in changes.rows.iter().enumerate() {
        let select = dialect.select_row(&changes.object, &change.key, true);
        let (columns, mut found) = read(connection, &select)?;
        if found.len() > 1 {
            return Err(Error::query("a row's key matches more than one row"));
        }
        match found.pop() {
            None => conflicts.push(Conflict { row, server: None }),
            Some(server) => {
                if changed_since_loaded(change, &columns, &server)? {
                    conflicts.push(Conflict {
                        row,
                        server: Some(server),
                    });
                }
            }
        }
    }
    if !conflicts.is_empty() {
        return Ok(Applied::Conflicts(conflicts));
    }
    for (row, update) in updates.iter().enumerate() {
        let params = rusqlite::params_from_iter(update.sql.params.iter().map(exact));
        let touched = match connection.execute(&update.sql.text, params).map_err(map_error) {
            Ok(touched) => touched,
            // A cancel or a lost session ends the save; anything else is
            // the statement's own failure.
            Err(error @ (Error::Cancelled | Error::ConnectionLost(_))) => return Err(error),
            Err(error) => return Ok(Applied::Failed { row, error }),
        };
        if touched != 1 {
            return Ok(Applied::Failed {
                row,
                error: Error::query(format!(
                    "the save would have changed {touched} rows where it meant one"
                )),
            });
        }
    }
    let mut rows = Vec::with_capacity(changes.rows.len());
    for change in &changes.rows {
        let select = dialect.select_row(&changes.object, &change.key, false);
        let (_, mut found) = read(connection, &select)?;
        rows.push(
            found
                .pop()
                .ok_or_else(|| Error::query("a saved row could not be read back"))?,
        );
    }
    Ok(Applied::Rows(rows))
}
```

`Connection::write` in `lib.rs`: `Inner::Sqlite(conn) => conn.write(changes).await,`.

A save's column names must be the ones a page gave (`CellChange::column` comes from the page): `declared_columns` is what `fetch_rows` uses, so they agree, lossy bytes included. A table whose key has a name that is not UTF-8 cannot be targeted (the page's SQL cannot name the column either); `update_row` would quote the lossy name and SQLite would refuse it, which is a `Failed`, not a wrong row.

Things to verify against rusqlite rather than assume: that `pragma_update` accepts a pragma that answers with a row (`journal_mode`, `locking_mode`) and takes a schema as `Some(rusqlite::DatabaseName::Main)`, and that `end_transaction` (the script runner's cleanup: `ROLLBACK` when one is open, then `set_session_pragmas`, tried twice) is right to reuse as it is. The authorizer's fence is `Off` here: these are the app's own statements, and the values are bound.

- [ ] **Step 4: Run**

Run: `~/.cargo/bin/cargo test --locked -p tabletist-db` three times (interrupts are timing).
Expected: PASS, the stop and cancel tests of the script runner included.

- [ ] **Step 5: Commit**

```bash
git add -A && git commit -m "Save changed rows on SQLite in one transaction"
```

---

### Task 7: PostgreSQL saves

> **As built:** decisions 11, 12 and 14 say what changed from the draft below: the transaction as text, the encoding pin, the NUL refusal, the read-back of exactly one row, the session's float digits and date style, the rollback first. A test lands a cancel on the `COMMIT` (a deferred constraint trigger that sleeps): the save answers `Cancelled`, nothing is written and the session is idle. Not tested, since nothing produces it on demand: a `ROLLBACK` that fails twice, which is `ConnectionLost` by reading. The steps below are the first draft.

**Files:**
- Create: `crates/tabletist-db/src/pg/write.rs`
- Modify: `crates/tabletist-db/src/pg.rs` (`mod write;`, visibility of `column_metas`, `row_values`, `query_error`), `lib.rs` (the PostgreSQL arm)
- Test: `crates/tabletist-db/tests/postgres.rs`

- [ ] **Step 1: Write the failing tests**

In `tests/postgres.rs`. Each test makes a table of its own, named after the test, through `admin()`, and drops it at the end even when an assertion fails (collect the outcome, drop, then assert). Each also drops what an earlier, panicked run may have left before it creates anything. **Never use the fixture's own types (`mood`, `size`):** the fixture drops them without CASCADE when it loads, so a table left behind by a panicked test would fail `load_fixture` for every PostgreSQL test from then on. The types table has an enum of its own, dropped with it:

```sql
DROP TABLE IF EXISTS write_types;
DROP TYPE IF EXISTS write_mood;
CREATE TYPE write_mood AS ENUM ('happy', 'sad');
CREATE TABLE write_types (
    id integer PRIMARY KEY,
    email text NOT NULL UNIQUE,
    name text,
    created_at timestamptz NOT NULL DEFAULT '2026-01-01 00:00:00+00',
    active boolean NOT NULL DEFAULT true,
    meta jsonb,
    score double precision,
    balance numeric(14, 2),
    tags text[],
    mood write_mood,
    uid uuid
);
INSERT INTO write_types VALUES
    (1, 'a@x', 'Ada', '2026-01-01 00:00:00+00', true, '{"a": 1}', 0.1, 12.50, '{x,y}', 'happy',
     '0199a3f2-7c1e-7abc-8def-0123456789ab'),
    (2, 'b@x', 'Bea', '2026-01-02 00:00:00+00', false, NULL, NULL, NULL, NULL, NULL, NULL);
```

Write these tests, with the same helpers as task 6 adapted to a table name and the `public` schema (`row_of(connection, table, id)` through `fetch_rows`, `save(connection, table, id, cells)`), and with type names as `describe` gives them (`text`, `timestamp with time zone`, `boolean`, `jsonb`, `double precision`, `numeric(14,2)`, `text[]`, `write_mood`, `uuid`). `Error::Query::code` is an `Option<String>`: match it with `code.as_deref() == Some("23502")`.

1. `a_save_writes_every_kind_of_value_and_reads_the_row_back`: on row 1 change `email`, `name` (to NULL), `created_at`, `active` (`false`), `meta`, `score` (`12.5`), `balance` (`99.95`), `tags` (`{a,b,c}`), `mood` (`sad`), `uid`; expect `Written`, the returned row equal to what `fetch_rows` now gives, and spot values (`active` is `Value::Bool(false)`, `balance` is `Value::Text("99.95")`). Then write every one of those columns again from what the page now holds: `Written`, never a conflict. That second save is what proves the loaded value of each type compares equal to itself.
2. `a_row_changed_by_someone_else_is_a_conflict_and_nothing_is_written`: two rows in the set; `admin` updates row 1's `name` after the page loaded; expect `Conflicts` with `row: 1`'s place in the set and the server's row, and row 2 unchanged.
3. `a_change_to_a_column_the_save_leaves_alone_is_no_conflict`.
4. `a_row_that_is_gone_is_a_conflict_without_a_row`.
5. `a_statement_that_fails_undoes_the_rows_before_it`: second row sets `email` NULL; expect `Failed { row: 1, error: Error::Query { code: Some("23502"), .. } }` and row 1 unchanged.
6. `a_value_the_column_cannot_take_is_the_databases_error`: `score` to `high`; expect `Failed { row: 0, .. }` with code `22P02`, and the row unchanged.
7. `a_save_on_a_read_only_connection_is_refused` : `connect()` (read-only) gives `Err(Error::ReadOnly)`.
8. `a_save_leaves_the_session_as_it_was`: after a `Written` and after a `Failed`, a script's `SHOW transaction_read_only` still answers `on` and `bypasses_cannot_write`-style `INSERT` still fails with 25006 (a save must not leave its transaction open or change the session).
9. `a_composite_and_a_binary_key_find_their_row`: a table with `PRIMARY KEY (a, b)` where `b` is `bytea`, one row, changed through a key of `Value::Int` and `Value::Bytes`.

- [ ] **Step 2: Run and see them fail**

Run: `~/.cargo/bin/cargo test --locked -p tabletist-db --test postgres save`
Expected: FAIL with `Unsupported`.

- [ ] **Step 3: Implement**

`crates/tabletist-db/src/pg/write.rs`:

```rust
//! A save on PostgreSQL: one read-write transaction, each row locked with
//! `FOR UPDATE` before it is compared and changed.

use std::time::Instant;

use tokio_postgres::SimpleQueryMessage;

use super::{Conn, column_metas, query_error, row_values};
use crate::write::changed_since_loaded;
use crate::{ChangeSet, ColumnMeta, Conflict, Dialect, Error, Result, Value, WriteOutcome};

impl Conn {
    /// See [`crate::Connection::write`]. Rows are read through the
    /// simple-query protocol, as a page reads them, so a loaded value and
    /// the same value read here are the same `Value`.
    pub async fn write(&self, changes: &ChangeSet) -> Result<WriteOutcome> {
        let dialect = Dialect::Postgres;
        // Every statement is built first: a value that cannot be sent
        // fails the save before the server hears of it.
        let mut updates = Vec::with_capacity(changes.rows.len());
        for (row, change) in changes.rows.iter().enumerate() {
            match dialect.update_row(&changes.object, change) {
                Ok(update) => updates.push(update),
                Err(error) => return Ok(WriteOutcome::Failed { row, error }),
            }
        }
        let mut client = self.client.lock().await;
        let started = Instant::now();
        // Read-write whatever the session's default is. Dropped without a
        // commit, it rolls back.
        let transaction = client
            .build_transaction()
            .read_only(false)
            .start()
            .await
            .map_err(query_error)?;
        // The columns and their types, to read rows as a page does.
        let statement = transaction
            .prepare(&format!("SELECT * FROM {}", dialect.qualified(&changes.object)))
            .await
            .map_err(query_error)?;
        let columns = column_metas(&statement);
        let names: Vec<String> = columns.iter().map(|column| column.name.clone()).collect();

        let mut conflicts = Vec::new();
        for (row, change) in changes.rows.iter().enumerate() {
            let select = dialect.select_row(&changes.object, &change.key, true);
            let mut found = rows(&transaction, &select.text, &columns).await?;
            if found.len() > 1 {
                return Err(Error::query("a row's key matches more than one row"));
            }
            match found.pop() {
                None => conflicts.push(Conflict { row, server: None }),
                Some(server) => {
                    if changed_since_loaded(change, &names, &server)? {
                        conflicts.push(Conflict {
                            row,
                            server: Some(server),
                        });
                    }
                }
            }
        }
        if !conflicts.is_empty() {
            transaction.rollback().await.map_err(query_error)?;
            return Ok(WriteOutcome::Conflicts(conflicts));
        }
        for (row, update) in updates.iter().enumerate() {
            let failed = match transaction.simple_query(&update.sql.text).await {
                Ok(messages) => {
                    let touched = messages.iter().find_map(|message| match message {
                        SimpleQueryMessage::CommandComplete(count) => Some(*count),
                        _ => None,
                    });
                    match touched {
                        Some(1) => continue,
                        other => Error::query(format!(
                            "the save would have changed {} rows where it meant one",
                            other.unwrap_or(0)
                        )),
                    }
                }
                Err(error) => match query_error(error) {
                    // A cancel or a lost session ends the save.
                    error @ (Error::Cancelled | Error::ConnectionLost(_)) => return Err(error),
                    error => error,
                },
            };
            transaction.rollback().await.map_err(query_error)?;
            return Ok(WriteOutcome::Failed { row, error: failed });
        }
        let mut saved = Vec::with_capacity(changes.rows.len());
        for change in &changes.rows {
            let select = dialect.select_row(&changes.object, &change.key, false);
            saved.push(
                rows(&transaction, &select.text, &columns)
                    .await?
                    .pop()
                    .ok_or_else(|| Error::query("a saved row could not be read back"))?,
            );
        }
        transaction.commit().await.map_err(query_error)?;
        Ok(WriteOutcome::Written {
            rows: saved,
            elapsed: started.elapsed(),
        })
    }
}

/// The rows of `text`, typed by `columns`.
async fn rows(
    transaction: &tokio_postgres::Transaction<'_>,
    text: &str,
    columns: &[ColumnMeta],
) -> Result<Vec<Vec<Value>>> {
    let messages = transaction.simple_query(text).await.map_err(query_error)?;
    let mut rows = Vec::new();
    for message in messages {
        if let SimpleQueryMessage::Row(row) = message {
            rows.push(row_values(&row, columns)?);
        }
    }
    Ok(rows)
}
```

Check against `tokio_postgres`: that `read_only(false)` begins `READ WRITE` (a writable session has no read-only default, but a server's own default may be), that a `Transaction` dropped on an early `?` return rolls back before the next command on the client, and that `fetch_rows` in `pg.rs` prepares and reads the same way (it does: `prepare`, `column_metas`, `simple_query`, `row_values`). `Connection::write` in `lib.rs`: `Inner::Postgres(conn) => conn.write(changes).await,`.

- [ ] **Step 4: Run and commit**

Run: `~/.cargo/bin/cargo test --locked -p tabletist-db --test postgres` (twice) then the workspace checks.
Expected: PASS.

```bash
git add -A && git commit -m "Save changed rows on PostgreSQL in one transaction"
```

---

### Task 8: MySQL saves

> **As built:** decisions 11, 13 and 14 say what changed from the draft below. The read-back tests use a key over a generated column instead of a trigger (MySQL forbids a trigger from changing its own table, and the test user may not create one). The engine race is pinned: another session holds `LOCK TABLES`, alters the engine while the save waits at its first read, and the save answers `Unsupported` with the row unchanged. Each statement's text goes through the driver's existing `driver_parameter` guard, since `conn.prep` also scans text for `:name`. A view is refused: it has no engine. The steps below are the first draft.

**Files:**
- Create: `crates/tabletist-db/src/mysql/write.rs`
- Modify: `crates/tabletist-db/src/mysql.rs` (`mod write;`, visibility of `column_metas`, `row_values`, `params`, `execute`, `status`, `query_error`), `lib.rs` (the MySQL arm)
- Test: `crates/tabletist-db/tests/mysql.rs`

> **From the review of task 6:** `ChangeSet::check` and `changed_since_loaded` compare column names exactly, and MySQL matches them without regard to case. A set that names `id` in its key and `ID` in its set passes `check` and would change its own key. On SQLite the exact match in `changed_since_loaded` stops it (the name is not among the row's columns); make sure the same holds here, with a test, before the first statement is sent. And the reviewer ran the `FLOAT` key case this plan leaves for step 3: a key of `Float(0.1)` finds no row, bound or shown, so the save reports the row gone. It fails safe; the note under "What this plan leaves for step 3" stands.

MySQL differs from the other two in four ways, each of which a test pins:
- **The transaction is started as text.** `START TRANSACTION READ WRITE`, checked by the server's status (in a transaction, not a read-only one), ended with `COMMIT` or `ROLLBACK` as text. Not `mysql_async`'s transaction options: the driver opens a read-only transaction as `SET TRANSACTION READ ONLY` then `START TRANSACTION`, and a cancel between the two leaves "next transaction read-only" pending.
- **Only a transactional engine.** A table whose engine has no transactions (MyISAM) is refused before the transaction starts.
- **Changed, not matched.** An `UPDATE` that leaves a row as it was reports no row. The row is locked and was just read, so none or one is right and more than one is the failure.
- **A warning is a failure.** Outside strict mode MySQL truncates or adjusts a value and says so only in a warning. A statement that raises one rolls the save back, with the warning's text as its error.

- [ ] **Step 1: Write the failing tests**

In `tests/mysql.rs`, each on a table of its own made through `admin()` in the fixture's database (schema `tabletist`), dropped at the end. The types table mirrors the fixture's `users`: `id INT PRIMARY KEY, email VARCHAR(255) NOT NULL UNIQUE, name VARCHAR(255), created_at DATETIME(6) NOT NULL, active TINYINT(1) NOT NULL, meta JSON, score DOUBLE, balance DECIMAL(14, 2), counter BIGINT UNSIGNED, mood ENUM('happy', 'sad'), birthday DATE, alarm TIME`, with type names as `describe` gives them (`varchar(255)`, `datetime(6)`, `tinyint(1)`, `json`, `double`, `decimal(14,2)`, `bigint unsigned`, `enum('happy','sad')`, `date`, `time`).

The nine tests of task 7, on MySQL (the NOT NULL failure is `Error::Query { code: Some("23000"), .. }`; a bad number in strict mode is `22007` or `HY000`: assert `Failed { row: 0, .. }` and the row unchanged, not the code), plus:

10. `a_boolean_goes_as_one_or_zero`: `active` (`tinyint(1)`) set to `false` is stored as 0, and `true` as 1.
11. `an_unchanged_value_is_not_a_failure`: a save whose new text equals what the column holds (MySQL reports 0 changed rows) is `Written`.
12. `a_warning_rolls_the_save_back`, at the unit level in `mysql/write.rs`, beside the script runner's unit tests, where `session(&url)` gives the raw connection (a script cannot set `sql_mode`, so an integration test has no way to leave strict mode): on a table of its own with a `VARCHAR(3)` column, `SET SESSION sql_mode = ''`, save `'abcdef'` into it, expect `Failed` whose message holds "truncated", and the row unchanged.
13. `a_table_without_transactions_is_refused`: `CREATE TABLE ... ENGINE = MyISAM`; expect `Err(Error::Unsupported(..))` and the row unchanged. Skip with a printed note if the server has no MyISAM (`SHOW ENGINES`).
14. `a_save_is_read_write_on_a_session_that_is_not`, at the unit level: the unit tests' `session()` is read-only at session level (`SET SESSION TRANSACTION READ ONLY`), which is what makes this a test of the explicit `READ WRITE`: a save on it, through `Conn::write` with the connection's access forced to writable as the neighbouring tests force theirs, is `Written`. (A pending one-shot `SET TRANSACTION READ ONLY` cannot be tested this way: the engine check's own statement uses it up.)
15. `a_save_whose_rollback_fails_closes_the_session`, if it can be made to happen: nothing in the suite makes a `ROLLBACK` fail on demand, so say in the report how you checked the order of the two `?`s instead (by reading, or by a temporary change that makes `execute(.., "ROLLBACK")` fail).

- [ ] **Step 2: Run and see them fail**

Run: `~/.cargo/bin/cargo test --locked -p tabletist-db --test mysql save`
Expected: FAIL with `Unsupported("saving is not built ...")`.

- [ ] **Step 3: Implement**

`crates/tabletist-db/src/mysql/write.rs`. Read `mysql/script.rs` first: `begin`, `started`, `close` and `retry_cancelled!` there are the pattern for a transaction managed as text.

```rust
//! A save on MySQL: one read-write transaction managed as text, each row
//! locked with `FOR UPDATE` before it is compared and changed.

use std::time::Instant;

use mysql_async::consts::StatusFlags;
use mysql_async::prelude::Queryable;

use super::{Conn, column_metas, execute, from_row, params, query_error, row_values, status};
use crate::script::retry_cancelled;
use crate::write::changed_since_loaded;
use crate::{ChangeSet, Conflict, Dialect, Error, Result, Sql, Value, WriteOutcome};

/// What the statements of a save came to, before its transaction ends.
enum Applied {
    Rows(Vec<Vec<Value>>),
    Conflicts(Vec<Conflict>),
    Failed { row: usize, error: Error },
}

impl Conn {
    /// See [`crate::Connection::write`].
    pub async fn write(&self, changes: &ChangeSet) -> Result<WriteOutcome> {
        let dialect = Dialect::MySql;
        let mut updates = Vec::with_capacity(changes.rows.len());
        for (row, change) in changes.rows.iter().enumerate() {
            match dialect.update_row(&changes.object, change) {
                Ok(update) => updates.push(update.sql),
                Err(error) => return Ok(WriteOutcome::Failed { row, error }),
            }
        }
        let mut conn = self.conn.lock().await;
        let started = Instant::now();
        transactional(&mut conn, changes).await?;
        // From here every path ends the transaction, whatever of it began:
        // a `begin` whose status check failed has one open too.
        let applied = match begin(&mut conn).await {
            Ok(()) => apply(&mut conn, changes, &updates).await,
            Err(error) => Err(error),
        };
        // `Some` is a COMMIT the server refused, undone by the rollback
        // after it. A cancel meant for a statement can land on a rollback:
        // it runs once more.
        let ended = match &applied {
            Ok(Applied::Rows(_)) => match execute(&mut conn, "COMMIT").await {
                Ok(()) => Ok(None),
                Err(refused) => {
                    retry_cancelled!(execute(&mut conn, "ROLLBACK")).map(|()| Some(refused))
                }
            },
            _ => retry_cancelled!(execute(&mut conn, "ROLLBACK")).map(|()| None),
        };
        // Looked at before `applied`: a transaction that could not be ended
        // closes the session whatever the save itself came to. Left open, it
        // would hold its rows, and the next START TRANSACTION would commit
        // them: half a save.
        let uncommitted = ended.map_err(|error| {
            Error::ConnectionLost(format!("could not end the save's transaction: {error}"))
        })?;
        if let Some(refused) = uncommitted {
            return Err(refused);
        }
        let applied = applied?;
        Ok(match applied {
            Applied::Rows(rows) => WriteOutcome::Written {
                rows,
                elapsed: started.elapsed(),
            },
            Applied::Conflicts(conflicts) => WriteOutcome::Conflicts(conflicts),
            Applied::Failed { row, error } => WriteOutcome::Failed { row, error },
        })
    }
}
```

with these, each with a doc comment in the file's voice:

- `async fn transactional(conn, changes) -> Result<()>`: `SELECT e.transactions FROM information_schema.tables t JOIN information_schema.engines e ON e.engine = t.engine WHERE t.table_schema = ? AND t.table_name = ?`, read as `Option<String>` (the column can be NULL); `"YES"` passes; anything else, or no row, is `Err(Error::Unsupported("saving needs a table whose engine has transactions, such as InnoDB"))`.
- `async fn begin(conn) -> Result<()>`: `execute(conn, "START TRANSACTION READ WRITE")`, then the status must hold `SERVER_STATUS_IN_TRANS` and must not hold `SERVER_STATUS_IN_TRANS_READONLY`; otherwise `Error::query("the server did not start a read-write transaction")`. A `begin` that fails its status check has a transaction open, which is why `write` rolls back after it too.
- Every statement of a save is prepared first and run as a prepared statement, the way `read_page` goes through `prepare` in `mysql.rs`: handed to `exec_*` as text, the driver would scan it for `:name` parameters, and a name or a value holding a colon would be rewritten.
- `async fn rows(conn, sql: &Sql) -> Result<(Vec<String>, Vec<Vec<Value>>)>`: the prepared statement through `conn.exec_iter(&statement, params(&sql.params))`, `column_metas(result.columns_ref())`, every row through `row_values`; the names are the metas' names.
- `async fn apply(conn, changes, updates) -> Result<Applied>`: the same three loops as SQLite's `apply` in task 6 (lock and compare; update; read back), with `select_row(.., true)` for the first and `select_row(.., false)` for the last. After each update: `conn.affected_rows()` greater than 1 is `Failed` ("would have changed N rows"); `conn.get_warnings()` greater than 0 is `Failed` with the first row of `SHOW WARNINGS` as its message (`Level`, `Code`, `Message`: use the message, and the code as `Error::Query::code`). A statement error is `Failed` unless it is `Error::Cancelled` or a lost connection, which end the save with `Err`.

Check against `mysql_async` 0.37 rather than assume: the names of the affected-rows and warning-count accessors after `exec_drop` or on a `QueryResult`, and that a prepared `UPDATE` with bound values reports warnings the same way. `Connection::write` in `lib.rs`: `Inner::MySql(conn) => conn.write(changes).await,`, and with it the `match` no longer needs its `Unsupported` arm.

- [ ] **Step 4: Run and commit**

Run: `~/.cargo/bin/cargo test --locked -p tabletist-db --test mysql` (three times: cancels are timing) and `~/.cargo/bin/cargo test --locked -p tabletist-db --lib mysql`, then the workspace checks.
Expected: PASS.

```bash
git add -A && git commit -m "Save changed rows on MySQL in one transaction"
```

---

### Task 9: The backend carries a save

> **As built:** as below. The session loop ends a session whose save answered `ConnectionLost`, pinned through a test-only seam (`Running::loses`, beside `panics`) that makes a save's answer a lost connection. `Event::Written`'s `Err` is documented as decision 9 has it. The steps below are the first draft.

**Files:**
- Modify: `src/backend.rs` (`Command`, `Event`, `session_of`, `request_of`, `fail`, `skip`, the session loop, tests), `src/app.rs` (`apply_event`)

> **From the review of task 6:** `write` answers `Error::ConnectionLost` when it could not end its transaction, but the crate does not close the handle itself. The session loop must drop the session on that error, as it does for a script's, or a session that may still be able to write stays in use. Pin it with a test if the loop's existing tests can reach it.

- [ ] **Step 1: Write the failing tests**

In the test module of `src/backend.rs`, beside the tests that connect the SQLite fixture (`fixture()`, `woken()`, `WAIT`), three tests. Use the fixture's real first user and a table the test may change (the fixture is a file of the test's own):

```rust
    fn rename(new: &str) -> tabletist_db::ChangeSet {
        tabletist_db::ChangeSet {
            object: ObjectRef::new("main", "users"),
            rows: vec![tabletist_db::RowChange {
                key: vec![("id".into(), Value::Int(1))],
                set: vec![tabletist_db::CellChange {
                    column: "name".into(),
                    type_name: "TEXT".into(),
                    // What the fixture's first user is called.
                    loaded: Value::Text("Ada Lovelace".into()),
                    new: tabletist_db::NewValue::Text(new.into()),
                }],
            }],
        }
    }
```

1. `a_save_on_a_writable_session_is_written`: connect with `Access::Writable` (the backend test `a_session_says_what_it_was_opened_as` shows how), send `Command::Write { session, request: RequestId(11), changes: rename("Grace") }`, expect `Event::Written { session, request: RequestId(11), result: Ok(WriteOutcome::Written { .. }) }`, and a following `FetchRows` shows the new name.
2. `a_save_on_a_read_only_session_is_refused_and_the_session_lives`: connect with `Access::ReadOnly`; expect `Event::Written { result: Err(Error::ReadOnly), .. }`, then a `CountRows` on the same session still answers.
3. `a_save_that_is_skipped_or_fails_with_its_session_says_so`: the patterns the neighbouring tests use for a request cancelled while queued (`skip`) and for a command sent to a session that is gone (`fail`) hold for `Write` too: each answers with an `Event::Written` carrying `Err`.

- [ ] **Step 2: Run and see them fail**

Run: `~/.cargo/bin/cargo test --locked -p tabletist --lib backend`
Expected: does not compile: no `Command::Write`.

- [ ] **Step 3: Implement**

`Command`, after `CountRows`:

```rust
    /// Write a save's changes (see `Connection::write`). Awaited to its
    /// end like every command: a cancel reaches it through the session's
    /// cancel handle, and the save rolls back.
    Write {
        session: SessionId,
        request: RequestId,
        changes: ChangeSet,
    },
```

`Event`, after `Count`:

```rust
    /// A `Write` ended. `Ok` holds how: written, conflicts, or a statement
    /// that failed, and only the first changed anything. `Err` is a save
    /// that never ran or whose session failed under it.
    Written {
        session: SessionId,
        request: RequestId,
        result: Result<WriteOutcome, Error>,
    },
```

- `session_of` and `request_of`: `Write` joins the arms of `CountRows`.
- `fail` and `skip`: `Write` answers with `Event::Written { session, request, result: Err(..) }`, as `CountRows` answers with `Event::Count` (use the same error each of them gives a count).
- The session loop, beside `CountRows`:

```rust
            Command::Write {
                session,
                request,
                changes,
            } => {
                let result = connection.write(changes).await;
                let lost = lost_error(&result);
                outbox.emit(Event::Written {
                    session: *session,
                    request: *request,
                    result,
                });
                lost
            }
```

- Every other exhaustive `match` on `Command` or `Event` in the workspace needs an arm; the compiler names them. In `App::apply_event` (`src/app.rs`):

```rust
            // Nothing sends a Write yet: editing in the grid (step 3 of
            // the value-editing spec) is what reads this.
            Event::Written { .. } => {}
```

- [ ] **Step 4: Run and commit**

Run: `~/.cargo/bin/cargo test --locked --workspace --all-targets`
Expected: PASS.

```bash
git add -A && git commit -m "Carry a save from the app to its session"
```

---

### Task 10: The documents, and the full checks

**Files:**
- Modify: `docs/superpowers/specs/2026-10-03-value-editing-core-design.md`, `docs/superpowers/specs/2026-09-27-tabletist-design.md`, `crates/tabletist-db/src/lib.rs` (the crate doc)

- [ ] **Step 1: The value-editing spec**

- Status line: steps 1 and 2 are built.
- "Saving": `Command::Write { session, request, changes }` (no tab); the MySQL engine rule; that SQLite binds a save's values exactly and puts `journal_mode`, `locking_mode` and `ignore_check_constraints` back first; that a value the builder cannot convert is `Failed` before anything is sent; what `ChangeSet::check` refuses; that MySQL's transaction is started as text and why; the PostgreSQL literal rule. These are among the first nine decisions at the top of this plan: write each where it belongs, as what the code does.
- "What can be edited": the row key rule now lives in `Structure::row_key`; say so in one clause. Add MySQL's engine rule to the list of what is never editable ("a MySQL table whose engine has no transactions").
- Decisions 10 to 14 too, each where it belongs: what SQLite refuses because it was not read exactly; that a key must name one row and what each driver does about it (including that PostgreSQL pages show full float digits and ISO dates, and that a MySQL table keyed by a `timestamp`, `bit` or `float` cannot be saved to); PostgreSQL's transaction as text; MySQL's second engine check, exact names and notes; the rollback first.
- "Saving" says that once `COMMIT` is sent a cancel is no longer honoured. That is not what happens: a cancel that reaches a `COMMIT` before it takes hold undoes the save, which answers `Cancelled`. Fix the sentence, not the code. And a cancel that arrives between two of a save's statements is ignored by PostgreSQL and MySQL, so the save goes on and commits; say so.
- "Saving", on `Err`: it is not always a lost session (decision 9). Say which errors leave the session alive with nothing written, so step 3 keeps the pending set on them.
- A SQLite column with no declared type takes a save's text as text: a number edited there becomes text, since nothing says it should be a number.
- Describe what was built, in the spec's voice. No em dashes.

- [ ] **Step 2: The main spec and the crate doc**

- `2026-09-27-tabletist-design.md`, section 4.1: add `write` to the API listing, and `class.rs`, `write.rs` and the three `*/write.rs` files to the layout in 3.2 if their siblings are listed there. Section 4.4: `ColumnInfo` gains `generated`, `IndexInfo` gains `partial`.
- `crates/tabletist-db/src/lib.rs`, the crate doc: "Nothing in this crate writes to a connected database yet" is no longer true. Say that the crate writes in exactly one place, `Connection::write`, only on a session opened `Access::Writable`, and that everything else stays fenced.

- [ ] **Step 3: Every check**

With both server URLs exported:

```bash
~/.cargo/bin/cargo fmt --all --check
~/.cargo/bin/cargo clippy --locked --workspace --all-targets -- -D warnings
RUSTDOCFLAGS='-D warnings' ~/.cargo/bin/cargo doc --locked --workspace --no-deps
~/.cargo/bin/cargo test --locked --workspace --all-targets
```

Expected: all four clean. Say in the report which database suites really ran.

- [ ] **Step 4: Commit**

```bash
git add -A && git commit -m "Describe the save the crate can now make"
```

---

## What this plan leaves for step 3

- The reducer's handling of `Event::Written`: replacing rows in the page, clearing pending cells, the status line.
- The checks a user sees while typing (their messages, the decimal's digits and scale, a length's counter). `column_class` gives them the class; the statement builder only refuses what it cannot convert.
- **Keys the builder cannot be sure of.** A MySQL key that decodes to text for a numeric column (`DECIMAL`, a `BIGINT UNSIGNED` above `i64::MAX`) is bound as a string, which MySQL may compare as a double; a `FLOAT` key is bound as a double and can miss its own row. Both fail safe (more than one row is an error, no row is a conflict), but such a table cannot be saved. Step 3 should lock its cells, or the key operand should be typed.
- **MySQL's engine rule is known only at save time.** `Structure` has no field for it, so the grid cannot lock a MyISAM table's cells up front. Step 3 decides whether the catalog should say.
- A lock another session holds makes a save wait, on PostgreSQL and MySQL, until the user cancels it. Whether a save should give up by itself is a question for the grid's Saving state.
- **What SQLite refuses at save time, the grid could lock up front** (decision 10): a row whose key text holds U+FFFD, a cell whose text was not read exactly, a column whose name another column reads as. `Value` does not say whether text was read exactly, so today only the save knows. And where two columns read as one name, the grid must take a row's key and loaded values by position, never by name.
- **What Review SQL shows is not always what another client would run.** On MySQL the shown text assumes backslash escapes, which the app's session keeps on; pasted into a session with `NO_BACKSLASH_ESCAPES` a backslash is stored doubled. On SQLite a REAL with a very large exponent, written as text, can read back as a neighbouring double (SQLite's parser), where the bound value is exact. Step 4 decides whether the dialog should say so.
- The read by key takes every matching row before it refuses more than one. A key is unique by `row_key`'s rule, so this only matters where that rule cannot see (a SQLite collation); `LIMIT 2` would bound it.
- **MySQL stores some values adjusted without a word:** in the default mode `'1.6'` into a `TINYINT` is stored as 2 and `16777217` into a `FLOAT` as `16777216.0`, with no warning and no note. The row a save reads back shows it; the checks a user sees while typing should catch what they can.
- **What MySQL refuses at save time, the grid could lock up front:** a table keyed by a `timestamp`, `bit` or `float` column (decision 11). And a TIMESTAMP as a changed column has the compare side of the same problem: a change by someone else between two instants that read alike in a repeated hour is not seen as a conflict.
- **A cancel between two of a save's statements is lost** on PostgreSQL and MySQL: neither server has anything to cancel at that moment, so the save commits and answers `Written`. The answer is true; "Mod+. cancels a running save" holds only while a statement runs.
- **Behind a pooler in transaction mode** a session-level `SET` is not carried between server sessions, so on a server whose default `extra_float_digits` is 0 or below PostgreSQL's float keys are not safe there. Closing it means setting it in each page read's transaction.
- A SQLite table with triggers: `rusqlite`'s count is the statement's own rows, so a trigger's changes do not disturb the "exactly one row" check, but what a trigger wrote is not in the row a save reads back unless it is that row.
