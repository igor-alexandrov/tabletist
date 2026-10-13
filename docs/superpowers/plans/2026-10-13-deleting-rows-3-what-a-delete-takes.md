# Deleting Rows, Run 3: What a Delete Takes With It Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Before a save that deletes rows, Review SQL, `:diff` and the production confirm say what else the deletes take: the rows another table loses to `ON DELETE CASCADE` and the values it clears to `ON DELETE SET NULL`, each with its count. A delete a foreign key refuses says how many rows of which table still point at the row, and opens them. A save deletes a child before its parent where a table's key points at the table itself.

**Architecture:** The structure of a table gains `referenced_by`: the foreign keys of any table that point at it, read with the rest of the structure on each engine. Counting needs nothing new in the database crate: a count is the existing `Connection::count_rows` over the referencing table, filtered to the marked rows' values, sent through the backend's existing `CountRows` and answered by its `Count`. The pending set keeps what was counted (`Edits::taken`) until the marks change, the reducer asks when a review or the production confirm is made, and puts the answers into the review as comment lines (`Line::Also`), so every place that draws a review's lines draws them. A refused delete keeps who holds its row on its mark (`Doomed::held`).

**Tech Stack:** Rust 1.98, `tokio-postgres`, `mysql_async`, `rusqlite`; egui/eframe (the crmne forks). Spec: `docs/superpowers/specs/2026-10-12-deleting-rows-design.md` ("spec N" is a section of its Behavior, "test N" an entry of its Tests). Runs 1 and 2: `docs/superpowers/plans/2026-10-12-deleting-rows-1-save.md` and `2026-10-12-deleting-rows-2-marking.md`, both built, in pull request `igor-alexandrov/tabletist#120`.

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

- `src/shots.rs` is outside those four and builds `Structure`s by hand. Task 1 adds a field to the type, so it also runs `~/.cargo/bin/cargo clippy --locked --features shots --lib --tests -- -D warnings`.
- **What was run where this plan was written: nothing of it.** It was written from reading the tree at `5043c01` (run 2's head), and checked again at `014990a`, after three fixes its review on the pull request asked for (`Edits::resend_deletes`, a mark that keeps only answered doubts, the note of a row already deleted on both paths of a save's answer). The code blocks are written against names that exist in that tree and were not compiled. Treat a block as what the code should come to, and expect to correct a name, a type or a borrow on the first build. Where a block and the compiler disagree, the compiler is right and the task's tests say what must hold. Line numbers are of `5043c01`: the three fixes moved some by a few lines.
- **Databases.** Task 1 adds assertions to `crates/tabletist-db/tests/postgres.rs` and `tests/mysql.rs`, which the two integration jobs of `ci.yml` already run against servers. There is no Docker in a Claude session: there those tests print "skipped", which proves nothing. After the push, read the two jobs' logs for the new tests' names with `... ok` before saying they ran (`gh api --allow-escape-sequences repos/igor-alexandrov/tabletist/actions/jobs/<id>/logs`).
- **The branch** is `claude/row-deletion-table-view-903d1e`, with pull request #120 open and Auto-fix on. The run's commits are pushed to it; the pull request's description gains what this run builds (see "After the last task").
- House rules that bite here: no em dashes; comments explain why, in the surrounding code's voice; code that depends on the engine matches on `Driver` or `Dialect` and names every variant; `crates/tabletist-db` has no UI dependency; a view pushes `Action`s and changes no state; database work runs on the backend, never on the UI thread; a key is spelled only in `src/keymap.rs`; every behaviour change has a headless test; tests use the Bookshop's data.
- Commits are signed, one per task. If signing fails ("agent refused operation"), 1Password is locked: do not commit unsigned. Keep each task's tree in a scratch git store outside the repository and commit them in order once signing works, as run 2 did. Each message ends with:

      Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>

- The UI tests of this run go into `src/ui/delete_tests.rs`, with its helpers (`mark`, `sent`, `written`, `restrict`, `desktop`).

## What the spec asks, and what gets built

Rows marked "ask" are choices this plan makes where the spec leaves one or the app has no part for it. Say so before task 1 if one should be otherwise.

| | The spec | What gets built | |
|---|---|---|---|
| a | "Use one `count(*)` per such key" | One count per key where the key is one column: the referencing column equal to the marked row's value, or `IN` its values where several rows are marked. A key of several columns is counted once for each marked row, and the counts are added: the app's filters are ANDed, and have no row-value `IN`. A value that holds a comma is counted by itself, since the filter's `IN` splits at commas | |
| b | "Also deletes 3 rows in book_covers (ON DELETE CASCADE)." | A comment line of the review, in the warning colour, after its statements: `-- also deletes 3 rows in book_covers (ON DELETE CASCADE)`. A review's own words are lower case and without a full stop, as its other comments are. The production confirm draws the review's lines, and `:diff` is the review, so both have it | ask |
| c | "Also clears publisher_id in 3 book_covers rows (ON DELETE SET NULL)." | `-- also clears publisher_id in 3 book_covers rows (ON DELETE SET NULL)` | |
| d | Counted "when Review SQL opens" | Asked when a review is made while rows are marked (Review SQL, `:diff`, the production confirm) and the table has such a key. Until the answer, and where a count fails, the line stands without a number: `-- also deletes rows in book_covers (ON DELETE CASCADE)`. A count of 0 has no line | |
| e | Counts | Only the rows that point at the marked rows directly. What those rows' own deletes take in a third table is not counted, and a table whose key points at itself counts rows that are marked too. `SET DEFAULT` has no line: the spec names two actions | ask |
| f | "3 rows in book_covers still point at this row · Open rows" after a restrict violation | After a delete fails with a foreign key's refusal (PostgreSQL 23503, and 23001 from version 18 on for `RESTRICT`; MySQL 1451; SQLite 787, and 1811 for `RESTRICT`) the app counts the rows of each key that points at the row and does not act on a delete. PostgreSQL and MySQL name the constraint, so that key alone is counted; SQLite names none, so each is, and the first that holds rows is said. The sentence stands under the database's words: in the pending bar, in the row panel, and on the Omarchy error line | |
| g | "Open rows opens a filtered tab" | A button among the pending bar's own (beside Review SQL, while the bar's line is of a refused delete with a holder) and a button in the row panel's note open the referencing table filtered to those rows, as following a foreign key opens the referenced one. In the Omarchy look `gd` on the row opens them, and the error line names the key: `gd open rows`. `gd` is "Open referenced row" on a foreign key's cell, and does this only on a row whose delete was refused | ask |
| h | (run 1's row q) Deletes in the order they were marked | A save's deletes are ordered so that a row goes before the row it points at, where the table's own foreign key points at the table. Rows that point at each other in a circle keep the page's order, and the database says what it says | |
| j | Counts that are out | A count is a statement on the tab's session, which runs one statement at a time: a slow count holds the next fetch and the save behind it. The counts are stopped where the tab's other reads are (Cancel, closing the tab), and when the marks change, so a mark toggled with the review open does not queue round after round | |
| i | Where the keys that point at a table come from | Read with the table's structure. On SQLite that is one `pragma_foreign_key_list` for each table of the file, each time a structure is read: SQLite has no list of the keys that point at a table | ask |

## What was decided for this run

- **The keys live in the structure.** `Structure::referenced_by: Vec<ReferencingKey>`. A view and the reducer read them where they read the structure, and a reload of the structure refreshes them.
- **Nothing new counts.** `RowQuery` with `Filter`s and the backend's `CountRows` are what the app counts a table's rows with already. The reducer builds the queries (`edit::taken_by`), keeps which request answers which line, and adds the numbers up.
- **What was counted goes with the marks.** `Edits::taken` is dropped by `mark`, `restore` and `discard`, and by nothing else: a changed cell changes no count.
- **The lines are the review's.** `Review::also(&[Effect])` puts them after its statements, in place of the ones it had. The reducer calls it on the tab's review and on the production prompt's when an answer arrives.
- **Who holds a refused row is on its mark.** `Doomed::holder`, set when the counts answer and only while the mark still has its error, dropped with the error (`fail_delete`, `resend_deletes`). The type is `Holder`: `Held` is the reducer's word for what waits behind a prompt (`src/model.rs`).
- **A count is one of the tab's reads.** `ObjectTab::pending()` lists the counts that are out with the tab's other requests, so Cancel and closing the tab stop them and the tab reads as busy while they run. The reducer cancels them where it drops what they were for.
- **A copy of the review has the lines too.** `review_whole` and the production sheet's copy put them after the statements, as the drawn review has them.

## File map

| File | What changes |
|---|---|
| `crates/tabletist-db/src/catalog.rs` | `ReferencingKey`, `Structure::referenced_by`, `ReferencingKey::acts`, `holds` |
| `crates/tabletist-db/src/pg.rs`, `mysql.rs`, `sqlite.rs` | Each reads the keys that point at the table |
| `crates/tabletist-db/src/lib.rs` | `ReferencingKey` exported |
| `crates/tabletist-db/tests/sqlite.rs`, `postgres.rs`, `mysql.rs` | The fixture's keys, read back |
| `src/edit.rs` | `Effect`, `Taken`, `Holder`, `taken_by`, `held_by`, `is_held`, `delete_order`; `Edits::taken`, `holders`, `counts_out`; `Doomed::holder`; `change_set` orders the deletes |
| `src/review.rs`, `src/ui/review.rs` | `Line::Also`, `Review::also`, its comment and colour |
| `src/model.rs` | `Action::OpenHolders`; `FollowForeignKey` by several columns; `ObjectTab::pending` lists the counts; `forget_session_requests` drops them |
| `src/app.rs`, `src/app/editing.rs` | Asking the counts, routing `Event::Count`, the refused delete's count, opening the holders |
| `src/ui/pending_bar.rs`, `src/ui/row_panel.rs`, `src/ui/data_view.rs`, `src/ui/keys.rs` | The sentence, its link, its button, the Omarchy line and `gd` |
| `src/testing.rs` | `publishers_structure` gains the keys that point at it; `publishers_page` |
| `src/ui/delete_tests.rs` | This run's UI tests |
| `docs/_guide/editing-data.md`, `docs/superpowers/specs/2026-10-12-deleting-rows-design.md` | The warnings, the count, Open rows; "As built" |

---

### Task 1: The structure lists the keys that point at a table

**Files:**
- Modify: `crates/tabletist-db/src/catalog.rs`, `src/lib.rs` (the `pub use catalog::{..}`)
- Modify: `crates/tabletist-db/src/pg.rs` (near line 684), `src/mysql.rs` (near line 370), `src/sqlite.rs` (near lines 906 and 1264)
- Modify: `crates/tabletist-db/tests/sqlite.rs`, `tests/postgres.rs`, `tests/mysql.rs`
- Modify: every `Structure { .. }` built without `..Default::default()` that the compiler names (`src/testing.rs`, `src/shots.rs`, `src/key_picker.rs`, `src/review.rs`)

- [ ] **Step 1: Write the failing tests**

In `crates/tabletist-db/src/catalog.rs`, in `mod tests` (add one if the file has none):

```rust
    fn key(on_delete: &str) -> ReferencingKey {
        ReferencingKey {
            name: Some("book_covers_publisher_id_fkey".into()),
            schema: "public".into(),
            table: "book_covers".into(),
            columns: vec!["publisher_id".into()],
            ref_columns: vec!["id".into()],
            on_delete: on_delete.into(),
        }
    }

    /// What a delete does to the rows that point at it, however the
    /// engine writes the action.
    #[test]
    fn a_key_acts_or_holds_by_its_action() {
        assert_eq!(key("CASCADE").acts(), Some(Acts::Deletes));
        assert_eq!(key("cascade").acts(), Some(Acts::Deletes));
        assert_eq!(key("SET NULL").acts(), Some(Acts::Clears));
        assert_eq!(key("SET DEFAULT").acts(), None);
        assert!(key("NO ACTION").holds());
        assert!(key("RESTRICT").holds());
        assert!(!key("CASCADE").holds());
        assert!(!key("SET DEFAULT").holds());
    }
```

In `crates/tabletist-db/tests/sqlite.rs`, beside the test that reads `structure.foreign_keys[0]` (near line 765):

```rust
/// The keys that point at a table: `orders.user_id` points at `users`.
#[tokio::test]
async fn a_structure_lists_the_keys_that_point_at_its_table() {
    let (connection, _dir) = fixture().await;
    let users = connection
        .describe(&ObjectRef::new("main", "users"))
        .await
        .unwrap();
    assert_eq!(
        users.referenced_by,
        [tabletist_db::ReferencingKey {
            name: None,
            schema: "main".into(),
            table: "orders".into(),
            columns: vec!["user_id".into()],
            ref_columns: vec!["id".into()],
            on_delete: "CASCADE".into(),
        }]
    );
    // And none points at `orders`.
    let orders = connection
        .describe(&ObjectRef::new("main", "orders"))
        .await
        .unwrap();
    assert!(orders.referenced_by.is_empty());
}
```

In `crates/tabletist-db/tests/postgres.rs`, in the file's way (`connect()` hands back `None` without a server):

```rust
/// The keys that point at `users`, from its own schema and from another:
/// `orders` loses its rows with the user, `billing.invoices` its user.
#[tokio::test]
async fn a_structure_lists_the_keys_that_point_at_its_table() {
    let Some(connection) = connect().await else {
        return;
    };
    let users = connection
        .describe(&ObjectRef::new("public", "users"))
        .await
        .unwrap();
    let said: Vec<(String, String, Vec<String>, String)> = users
        .referenced_by
        .iter()
        .map(|key| {
            (
                key.schema.clone(),
                key.table.clone(),
                key.columns.clone(),
                key.on_delete.clone(),
            )
        })
        .collect();
    assert!(said.contains(&(
        "public".into(),
        "orders".into(),
        vec!["user_id".into()],
        "CASCADE".into()
    )));
    assert!(said.contains(&(
        "billing".into(),
        "invoices".into(),
        vec!["user_id".into()],
        "SET NULL".into()
    )));
    let orders = users
        .referenced_by
        .iter()
        .find(|key| key.table == "orders")
        .unwrap();
    assert_eq!(orders.ref_columns, ["id"]);
    assert!(orders.name.is_some());
}
```

In `crates/tabletist-db/tests/mysql.rs` the same test, with the schema `tabletist` for `users` and `orders`, the names `orders_user_id_fkey` and `invoices_user_fkey`, and `CASCADE` for both (`fixtures/mysql.sql`, lines 37 and 61). Other tests of those files may add tables that point at `users`: the assertions above look for the two keys and count none.

- [ ] **Step 2: Run them to see them fail**

Run: `~/.cargo/bin/cargo test --locked -p tabletist-db --lib catalog`
Expected: FAIL to compile: no `ReferencingKey`.

- [ ] **Step 3: The type**

In `crates/tabletist-db/src/catalog.rs`, after `ForeignKeyInfo`:

```rust
/// A foreign key of another table, or of the table itself, that points at
/// the table a [`Structure`] describes: what a row of that table loses, or
/// is held by, when a row of this one is deleted.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ReferencingKey {
    /// SQLite foreign keys have no names.
    pub name: Option<String>,
    /// The table that has the key.
    pub schema: String,
    pub table: String,
    /// Its columns, and the columns of the described table each points at.
    pub columns: Vec<String>,
    pub ref_columns: Vec<String>,
    /// As the engine writes it: `CASCADE`, `SET NULL`, `NO ACTION`.
    pub on_delete: String,
}

/// What a foreign key does to the rows that point at a row that is
/// deleted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Acts {
    /// `ON DELETE CASCADE`: they are deleted with it.
    Deletes,
    /// `ON DELETE SET NULL`: their key is cleared.
    Clears,
}

impl ReferencingKey {
    /// What the key does on a delete, where it does one of the two things
    /// a user is told of. `None` for a key that holds the row, and for
    /// `SET DEFAULT`.
    pub fn acts(&self) -> Option<Acts> {
        match self.on_delete.to_ascii_uppercase().as_str() {
            "CASCADE" => Some(Acts::Deletes),
            "SET NULL" => Some(Acts::Clears),
            _ => None,
        }
    }

    /// Whether the key keeps a row it points at from being deleted.
    pub fn holds(&self) -> bool {
        matches!(
            self.on_delete.to_ascii_uppercase().as_str(),
            "NO ACTION" | "RESTRICT"
        )
    }
}
```

`Structure` gains, after `foreign_keys`:

```rust
    /// The foreign keys that point at the table, its own among them where
    /// it points at itself.
    pub referenced_by: Vec<ReferencingKey>,
```

Export `ReferencingKey` and `Acts` in `crates/tabletist-db/src/lib.rs` beside `ForeignKeyInfo`.

- [ ] **Step 4: PostgreSQL**

In `crates/tabletist-db/src/pg.rs`, after `foreign_keys` is read and before `Ok(Structure {`:

```rust
        // The keys that point at the table: the same catalog, read from
        // the other end.
        let referenced_by = self
            .catalog(
                "SELECT con.conname::text, cn.nspname::text, cc.relname::text, \
                        ARRAY(SELECT a.attname::text FROM unnest(con.conkey) WITH ORDINALITY AS k(attnum, ord) \
                              JOIN pg_attribute a ON a.attrelid = con.conrelid AND a.attnum = k.attnum \
                              ORDER BY k.ord), \
                        ARRAY(SELECT a.attname::text FROM unnest(con.confkey) WITH ORDINALITY AS k(attnum, ord) \
                              JOIN pg_attribute a ON a.attrelid = con.confrelid AND a.attnum = k.attnum \
                              ORDER BY k.ord), \
                        con.confdeltype::text \
                 FROM pg_constraint con \
                 JOIN pg_class cc ON cc.oid = con.conrelid \
                 JOIN pg_namespace cn ON cn.oid = cc.relnamespace \
                 WHERE con.confrelid = $1 AND con.contype = 'f' \
                 ORDER BY cn.nspname, cc.relname, con.conname",
                &[&oid],
            )
            .await?
            .iter()
            .map(|row| {
                let on_delete: String = column(row, 5)?;
                Ok(ReferencingKey {
                    name: Some(column(row, 0)?),
                    schema: column(row, 1)?,
                    table: column(row, 2)?,
                    columns: column(row, 3)?,
                    ref_columns: column(row, 4)?,
                    on_delete: action(&on_delete).into(),
                })
            })
            .collect::<Result<_>>()?;
```

and `referenced_by` joins the `Structure`. A partitioned table that has the key has a row of `pg_constraint` for each of its partitions too, all pointing at this table: without a filter each would be a line of its own. The copies name their parent in `conparentid`, which servers before version 11 do not have, so it is read as the file reads other newer columns (`to_jsonb(..) ->> ..`, near line 590). Add to the `WHERE`:

```sql
AND coalesce((to_jsonb(con) ->> 'conparentid')::oid, 0) = 0
```

- [ ] **Step 5: MySQL**

In `crates/tabletist-db/src/mysql.rs`, after `foreign_keys` is built:

```rust
        // The keys that point at the table: the same views, asked by what
        // a key refers to.
        let referencing: Vec<(String, String, String, String, String, String)> = self
            .catalog(
                "SELECT k.constraint_name, k.table_schema, k.table_name, k.column_name, \
                        k.referenced_column_name, r.delete_rule \
                 FROM information_schema.key_column_usage k \
                 JOIN information_schema.referential_constraints r \
                   ON r.constraint_schema = k.constraint_schema AND r.constraint_name = k.constraint_name \
                  AND r.table_name = k.table_name \
                 WHERE k.referenced_table_schema = ? AND k.referenced_table_name = ? \
                 ORDER BY k.table_schema, k.table_name, k.constraint_name, k.ordinal_position",
                at,
            )
            .await?;
        let mut referenced_by: Vec<ReferencingKey> = Vec::new();
        for (name, schema, table, column, ref_column, on_delete) in referencing {
            let same = referenced_by.last().is_some_and(|key| {
                key.name.as_deref() == Some(name.as_str()) && key.schema == schema && key.table == table
            });
            if !same {
                referenced_by.push(ReferencingKey {
                    name: Some(name),
                    schema,
                    table,
                    columns: Vec::new(),
                    ref_columns: Vec::new(),
                    on_delete,
                });
            }
            if let Some(key) = referenced_by.last_mut() {
                key.columns.push(column);
                key.ref_columns.push(ref_column);
            }
        }
```

`at` is what the query above it binds for the schema and the table: if it was moved into that call, bind the two again as it was built.

- [ ] **Step 6: SQLite**

SQLite lists a table's own keys and no others, so every table's are read. In `crates/tabletist-db/src/sqlite.rs`, after `foreign_keys`:

```rust
/// The foreign keys that point at `object`: every table's own keys, kept
/// where they refer to it. SQLite has no list of them, and matches a
/// table's name without regard to ASCII case.
fn referencing_keys(
    connection: &rusqlite::Connection,
    object: &ObjectRef,
) -> Result<Vec<ReferencingKey>> {
    let tables: Vec<String> = {
        let mut statement = connection
            .prepare(&format!(
                "SELECT name FROM {}.sqlite_master WHERE type = 'table' ORDER BY name",
                Dialect::Sqlite.quote_ident(&object.schema)
            ))
            .map_err(map_error)?;
        statement
            .query_map([], |row| text(row, 0))
            .map_err(map_error)?
            .collect::<std::result::Result<_, _>>()
            .map_err(map_error)?
    };
    let mut keys = Vec::new();
    for table in tables {
        let from = ObjectRef::new(object.schema.clone(), table);
        for key in foreign_keys(connection, &from)? {
            if key.ref_table.eq_ignore_ascii_case(&object.name) {
                keys.push(ReferencingKey {
                    name: None,
                    schema: from.schema.clone(),
                    table: from.name.clone(),
                    columns: key.columns,
                    ref_columns: key.ref_columns,
                    on_delete: key.on_delete,
                });
            }
        }
    }
    Ok(keys)
}
```

Read a table's name as the file's other catalog reads do (`text(row, 0)` is what `foreign_keys` uses for a name that may not be UTF-8; use whatever it is called there). Where the structure is built (near line 906): `referenced_by: referencing_keys(connection, &object)?`.

- [ ] **Step 7: What the compiler names**

Run: `~/.cargo/bin/cargo check --locked --workspace --all-targets`
Every `Structure { .. }` that names its fields gains `referenced_by: Vec::new()`.

- [ ] **Step 8: Run the tests**

Run: `~/.cargo/bin/cargo test --locked -p tabletist-db`
Expected: PASS; the PostgreSQL and MySQL tests print "skipped" without servers.

- [ ] **Step 9: Run the four checks, and the shots' clippy**

Expected: all pass.

- [ ] **Step 10: Commit**

```bash
git add crates src && git commit -m "List the foreign keys that point at a table in its structure

Each engine reads them with the rest of the structure: PostgreSQL and
MySQL from the catalog they read a table's own keys from, SQLite from
every table's own list, since it keeps no other. A key says whether a
delete takes the rows that point at it, clears them, or is held by them.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 2: A child goes before its parent

**Files:**
- Modify: `src/edit.rs` (`delete_order`, `change_set`)

- [ ] **Step 1: Write the failing tests**

In `src/edit.rs`, in `mod tests`. `src/testing.rs` has a Bookshop table that points at itself already (`genres_structure` and `genres_page`, near line 820): use them in place of the hand-made table below if their rows give a chain of three and one apart, and keep the assertions. Otherwise, made by hand:

```rust
    /// `book_categories`: a category stands under another (`parent_id`).
    fn categories(rows: &[(i64, Option<i64>)]) -> (Structure, RowPage) {
        let column = |name: &str| ColumnInfo {
            name: name.into(),
            type_name: "INTEGER".into(),
            nullable: name == "parent_id",
            ..ColumnInfo::default()
        };
        let own = tabletist_db::ForeignKeyInfo {
            name: None,
            columns: vec!["parent_id".into()],
            ref_schema: "main".into(),
            ref_table: "book_categories".into(),
            ref_columns: vec!["id".into()],
            on_update: "NO ACTION".into(),
            on_delete: "NO ACTION".into(),
        };
        let structure = Structure {
            columns: vec![column("id"), column("parent_id")],
            primary_key: vec!["id".into()],
            foreign_keys: vec![own],
            ..Structure::default()
        };
        let meta = |name: &str| ColumnMeta {
            name: name.into(),
            type_name: "INTEGER".into(),
            kind: ValueKind::Numeric,
        };
        let page = RowPage {
            columns: vec![meta("id"), meta("parent_id")],
            rows: rows
                .iter()
                .map(|(id, parent)| vec![Value::Int(*id), parent.map_or(Value::Null, Value::Int)])
                .collect(),
            has_more: false,
            ordered_by_key: true,
            elapsed: Duration::from_millis(1),
        };
        (structure, page)
    }

    /// Row h: a row is deleted before the row it points at.
    #[test]
    fn a_child_is_deleted_before_its_parent() {
        // Fiction (1), under it Crime (2), under that Noir (3); Poetry (4).
        let (structure, page) = categories(&[(1, None), (2, Some(1)), (3, Some(2)), (4, None)]);
        let object = ObjectRef::new("main", "book_categories");
        let mut edits = Edits::default();
        for row in [0, 1, 2, 3] {
            edits.mark(row);
        }
        let table = marked_covers(&structure, &page, &edits);
        let (changes, sent) = change_set(&object, &table, &edits.cells).unwrap();
        let ids: Vec<&Value> = changes.deletes.iter().map(|row| &row.key[0].1).collect();
        // Each child before its parent: what must hold.
        let at = |id: i64| ids.iter().position(|value| **value == Value::Int(id)).unwrap();
        assert!(at(3) < at(2) && at(2) < at(1));
        // The rows nothing points at go first, in the page's order: Noir
        // and Poetry, then Crime, then Fiction.
        assert_eq!(
            ids,
            [&Value::Int(3), &Value::Int(4), &Value::Int(2), &Value::Int(1)]
        );
        // What the answer names by its place is the page's row.
        assert_eq!(sent.deletes, [2, 3, 1, 0]);
    }

    /// Rows that point at each other keep the page's order: no order
    /// deletes them, and the database says so.
    #[test]
    fn rows_in_a_circle_keep_the_pages_order() {
        let (structure, page) = categories(&[(1, Some(2)), (2, Some(1))]);
        let object = ObjectRef::new("main", "book_categories");
        let mut edits = Edits::default();
        edits.mark(0);
        edits.mark(1);
        let table = marked_covers(&structure, &page, &edits);
        let (_, sent) = change_set(&object, &table, &edits.cells).unwrap();
        assert_eq!(sent.deletes, [0, 1]);
    }
```

`change_set` reads the table's name from `object`, so `delete_order` takes it too. `marked_covers` builds a `Table` of any structure and page: rename it `marked_table` if the name misleads here.

- [ ] **Step 2: Run them to see them fail**

Run: `~/.cargo/bin/cargo test --locked --lib edit::tests::a_child`
Expected: FAIL: the deletes are in the page's order, `[1, 2, 3, 4]`.

- [ ] **Step 3: The order**

In `src/edit.rs`, before `change_set`:

```rust
/// The page's rows marked for deletion, in the order a save deletes them:
/// a row before the row it points at, where a foreign key of the table
/// points at the table itself (`object`). A key that holds would refuse
/// the parent while its child is there, though the save deletes both.
/// Rows that point at each other in a circle, and rows nothing orders,
/// keep the page's order.
fn delete_order(object: &ObjectRef, table: &Table<'_>) -> Vec<usize> {
    let marked: Vec<usize> = table.deleted.keys().copied().collect();
    let own: Vec<(Vec<usize>, Vec<usize>)> = table
        .structure
        .map(|structure| structure.foreign_keys.as_slice())
        .unwrap_or_default()
        .iter()
        // SQLite matches a table's name without regard to ASCII case, and
        // a key that names another schema's table is not the table's own.
        .filter(|key| {
            key.ref_table.eq_ignore_ascii_case(&object.name)
                && (key.ref_schema.is_empty() || key.ref_schema == object.schema)
        })
        .filter_map(|key| {
            let place = |name: &String| table.page.columns.iter().position(|c| c.name == *name);
            let from: Option<Vec<usize>> = key.columns.iter().map(place).collect();
            let to: Option<Vec<usize>> = key.ref_columns.iter().map(place).collect();
            let (from, to) = (from?, to?);
            (!from.is_empty() && from.len() == to.len()).then_some((from, to))
        })
        .collect();
    if own.is_empty() || marked.len() < 2 {
        return marked;
    }
    let values = |row: usize, cols: &[usize]| -> Option<Vec<&Value>> {
        let held = table.page.rows.get(row)?;
        let values: Vec<&Value> = cols.iter().filter_map(|&col| held.get(col)).collect();
        // A key with a NULL in it points at no row.
        (values.len() == cols.len() && values.iter().all(|value| !value.is_null()))
            .then_some(values)
    };
    // Whether `child` points at `parent`.
    let points_at = |child: usize, parent: usize| {
        child != parent
            && own.iter().any(|(from, to)| {
                matches!(
                    (values(child, from), values(parent, to)),
                    (Some(a), Some(b)) if a.iter().zip(&b).all(|(a, b)| same_value(a, b))
                )
            })
    };
    // A row is taken once no row still waiting points at it. A pass that
    // takes none has found a circle: what is left keeps its order.
    let mut order = Vec::with_capacity(marked.len());
    let mut waiting = marked;
    while !waiting.is_empty() {
        let free: Vec<usize> = waiting
            .iter()
            .copied()
            .filter(|&row| !waiting.iter().any(|&other| points_at(other, row)))
            .collect();
        if free.is_empty() {
            order.append(&mut waiting);
            break;
        }
        waiting.retain(|row| !free.contains(row));
        order.extend(free);
    }
    order
}
```

In `change_set`, the loop over the rows to delete reads the order:

```rust
    // The rows to delete: a row before the row it points at, else in the
    // page's order.
    let mut deletes = Vec::new();
    for row in delete_order(object, table) {
        deletes.push(RowDelete {
            key: key_of(table.page.rows.get(row)?)?,
        });
        sent.deletes.push(row);
    }
```

- [ ] **Step 4: Run the tests**

Run: `~/.cargo/bin/cargo test --locked --lib edit::tests`
Expected: PASS.

- [ ] **Step 5: Run the four checks**

- [ ] **Step 6: Commit**

```bash
git add src && git commit -m "Delete a row before the row it points at

Where a table's foreign key points at the table itself, a save deletes
the rows that point at a marked row before that row: a key that holds
would refuse the parent while its child is there. Rows nothing orders
keep the page's order.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 3: What the deletes take, as lines of the review

No count is asked yet: this task is the words and where they stand.

**Files:**
- Modify: `src/edit.rs` (`Effect`)
- Modify: `src/review.rs` (`Line::Also`, `Review::also`)
- Modify: `src/ui/review.rs` (`comment`, `comment_color`)

- [ ] **Step 1: Write the failing tests**

In `src/ui/review.rs`, in `every_comment_reads_as_the_spec_writes_it`:

```rust
        let also = |effect: Effect| read(Line::Also(effect));
        let covers = || "book_covers".to_owned();
        assert_eq!(
            also(Effect::Deletes { table: covers(), rows: Some(3) }).as_deref(),
            Some("-- also deletes 3 rows in book_covers (ON DELETE CASCADE)")
        );
        assert_eq!(
            also(Effect::Deletes { table: covers(), rows: Some(1) }).as_deref(),
            Some("-- also deletes 1 row in book_covers (ON DELETE CASCADE)")
        );
        let publisher = || vec!["publisher_id".to_owned()];
        assert_eq!(
            also(Effect::Clears { table: covers(), columns: publisher(), rows: Some(3) }).as_deref(),
            Some("-- also clears publisher_id in 3 book_covers rows (ON DELETE SET NULL)")
        );
        // Not counted yet, or not countable: the line without a number.
        assert_eq!(
            also(Effect::Deletes { table: covers(), rows: None }).as_deref(),
            Some("-- also deletes rows in book_covers (ON DELETE CASCADE)")
        );
        assert_eq!(
            also(Effect::Clears { table: covers(), columns: publisher(), rows: None }).as_deref(),
            Some("-- also clears publisher_id in book_covers rows (ON DELETE SET NULL)")
        );
```

In `src/review.rs`, in `mod tests`:

```rust
    /// The lines stand after the statements, and a later answer takes the
    /// earlier one's place.
    #[test]
    fn what_the_deletes_take_stands_after_the_statements() {
        let changes = deleting(vec![vec![("id", Value::Int(9))]]);
        let mut review = of(Dialect::Sqlite, &changes, Blocked::default(), Values::Shown);
        let statements = review.lines.len();
        let effect = |rows| Effect::Deletes {
            table: "book_covers".into(),
            rows,
        };
        review.also(&[effect(None)]);
        assert_eq!(review.lines.len(), statements + 1);
        assert_eq!(review.lines.last(), Some(&Line::Also(effect(None))));
        review.also(&[effect(Some(3))]);
        assert_eq!(review.lines.len(), statements + 1);
        assert_eq!(review.lines.last(), Some(&Line::Also(effect(Some(3)))));
        // Nothing taken: no line.
        review.also(&[]);
        assert_eq!(review.lines.len(), statements);
    }
```

- [ ] **Step 2: Run them to see them fail**

Expected: FAIL to compile: no `Effect`.

- [ ] **Step 3: The effect and the line**

In `src/edit.rs`, after `Told`:

```rust
/// What a save's deletes do in a table whose foreign key points at the
/// rows: its name as it is shown, and how many of its rows, once counted.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Effect {
    /// `ON DELETE CASCADE`: the rows go with the deleted ones.
    Deletes { table: String, rows: Option<u64> },
    /// `ON DELETE SET NULL`: `columns` are cleared in them.
    Clears {
        table: String,
        columns: Vec<String>,
        rows: Option<u64>,
    },
}
```

In `src/review.rs`, `Line` gains, before `Sql`:

```rust
    /// `-- also deletes 3 rows in book_covers (ON DELETE CASCADE)`: what
    /// the deletes above take with them in another table.
    Also(Effect),
```

and `Review`:

```rust
impl Review {
    /// Puts what the set's deletes take with them after its statements,
    /// in place of what it said of that before.
    pub fn also(&mut self, effects: &[Effect]) {
        self.lines.retain(|line| !matches!(line, Line::Also(_)));
        self.lines.extend(effects.iter().cloned().map(Line::Also));
    }
}
```

Import `Effect` from `crate::edit`.

- [ ] **Step 4: The words and the colour**

In `src/ui/review.rs`, `comment` gains, before `Line::Sql`:

```rust
        Line::Also(Effect::Deletes { table, rows }) => {
            let what = match rows {
                Some(rows) => counted(locale, usize::try_from(*rows).unwrap_or(usize::MAX), "row", "rows"),
                None => say("rows").into_owned(),
            };
            format!("-- {} {what} {} {table} (ON DELETE CASCADE)", say("also deletes"), say("in"))
        }
        Line::Also(Effect::Clears { table, columns, rows }) => {
            let rows = match rows {
                Some(rows) => {
                    let plural = u32::try_from(*rows).unwrap_or(u32::MAX);
                    format!("{rows} {table} {}", ngettext(locale, "row", "rows", plural))
                }
                None => format!("{table} {}", say("rows")),
            };
            format!(
                "-- {} {} {} {rows} (ON DELETE SET NULL)",
                say("also clears"),
                columns.join(", "),
                say("in")
            )
        }
```

(`counted` is `pending_bar::counted`, which the file already uses; `ngettext` from `crate::i18n`.) `comment_color` gives `Line::Also(_)` the warning's colour: `Tone::Warning.color(palette)`.

The clipboard's text (`text`, near line 257) words every comment line, but both copies are made of a fresh review, which has none of these: `App::review_whole` (`src/app/editing.rs`) and the production sheet's Copy SQL (`src/ui/write_prompts.rs`, near line 618). Task 4 gives both the lines. For the sheet, `Review` hands its own back:

```rust
    /// What the review says its deletes take with them.
    pub fn effects(&self) -> Vec<Effect> {
        let also = |line: &Line| match line {
            Line::Also(effect) => Some(effect.clone()),
            _ => None,
        };
        self.lines.iter().filter_map(also).collect()
    }
```

with a line in the test above that `review.effects()` is what `also` was given.

- [ ] **Step 5: Run the tests, then the four checks**

Run: `~/.cargo/bin/cargo test --locked --lib review`
Expected: PASS, and all four checks pass.

- [ ] **Step 6: Commit**

```bash
git add src && git commit -m "Word what a save's deletes take with them

A line of the review, after its statements: the rows another table
loses to ON DELETE CASCADE, and the columns it clears to ON DELETE SET
NULL, each with its count once there is one. Nothing asks for a count
yet.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 4: Count what the deletes take

**Files:**
- Modify: `src/edit.rs` (`Taken`, `taken_by`, `Edits::taken`, the three places that drop it)
- Modify: `src/app/editing.rs` (`count_taken`, called where a review is made and where the production prompt is opened), `src/app.rs` (`Event::Count`)
- Modify: `src/testing.rs` (a `publishers` fixture), `src/ui/delete_tests.rs`

- [ ] **Step 1: The fixture**

`src/testing.rs` has `publishers_structure` already (near line 788; the foreign key picker's tests use it). Give it the two keys that point at the table, and nothing else of it changes:

```rust
        referenced_by: vec![
            // `book_covers` loses its covers with a publisher, and `books`
            // is cleared of it.
            tabletist_db::ReferencingKey {
                name: Some("book_covers_publisher_id_fkey".into()),
                schema: "main".into(),
                table: "book_covers".into(),
                columns: vec!["publisher_id".into()],
                ref_columns: vec!["id".into()],
                on_delete: "CASCADE".into(),
            },
            tabletist_db::ReferencingKey {
                name: Some("books_publisher_id_fkey".into()),
                schema: "main".into(),
                table: "books".into(),
                columns: vec!["publisher_id".into()],
                ref_columns: vec!["id".into()],
                on_delete: "SET NULL".into(),
            },
        ],
```

Its columns are what they are there: where the tests below name `id` and `name`, use the function's own. Beside it, a page:

```rust
/// A page of `publishers`: the three of [`PUBLISHERS`].
pub fn publishers_page() -> RowPage {
    let meta = |name: &str, type_name: &str, kind: ValueKind| ColumnMeta {
        name: name.into(),
        type_name: type_name.into(),
        kind,
    };
    RowPage {
        columns: vec![
            meta("id", "INTEGER", ValueKind::Numeric),
            meta("name", "TEXT", ValueKind::Text),
        ],
        rows: PUBLISHERS
            .iter()
            .map(|(id, name)| vec![Value::Int(*id), Value::Text((*name).into())])
            .collect(),
        has_more: false,
        ordered_by_key: true,
        elapsed: std::time::Duration::from_millis(2),
    }
}
```

(with the columns `publishers_structure` has, in its order). And in `src/ui/delete_tests.rs` a helper as `covers_in` opens `book_covers`:

```rust
/// The Bookshop's publishers, writable, the grid holding the keyboard.
fn publishers_in(look: Look) -> (Harness, ConnTabId, TabId) {
    let mut harness = Harness::new();
    harness.set_look(look);
    let tab = harness.connect_fake_as(false);
    harness.app.apply(Action::OpenObject {
        tab,
        object: tabletist_db::ObjectRef::new("main", "publishers"),
        kind: tabletist_db::ObjectKind::Table,
        pin: true,
    });
    harness.answer_structure(crate::testing::publishers_structure());
    harness.answer_rows(crate::testing::publishers_page());
    let id = harness.app.workspace(tab).unwrap().active_tab.unwrap();
    harness.app.workspace_mut(tab).unwrap().pane = crate::model::Pane::Grid;
    harness.settle();
    (harness, tab, id)
}
```

- [ ] **Step 2: Write the failing tests**

In `src/edit.rs`, in `mod tests`:

```rust
    /// Row a: one count for a key of one column, `IN` the marked rows'
    /// values where there are several.
    #[test]
    fn what_a_delete_takes_is_asked_once_for_each_key() {
        let structure = crate::testing::publishers_structure();
        let page = crate::testing::publishers_page();
        let mut edits = Edits::default();
        edits.mark(0);
        let table = marked_covers(&structure, &page, &edits);
        let asked = taken_by(&table);
        assert_eq!(asked.len(), 2);
        let (effect, queries) = &asked[0];
        assert_eq!(
            *effect,
            Effect::Deletes {
                table: "book_covers".into(),
                rows: None
            }
        );
        assert_eq!(queries.len(), 1);
        assert_eq!(queries[0].object, ObjectRef::new("main", "book_covers"));
        let filter = &queries[0].filters[0];
        assert_eq!(
            (filter.column.as_str(), filter.op, filter.value.as_str()),
            ("publisher_id", FilterOp::Eq, "9100000000000000001")
        );
        assert!(matches!(&asked[1].0, Effect::Clears { table, .. } if table == "books"));
        // Two rows: one count still, by their two values.
        edits.mark(2);
        let table = marked_covers(&structure, &page, &edits);
        let asked = taken_by(&table);
        let filter = &asked[0].1[0].filters[0];
        assert_eq!(filter.op, FilterOp::In);
        assert_eq!(filter.value, "9100000000000000001,9100000000000000007");
        // Nothing marked, nothing asked.
        let none = Edits::default();
        assert!(taken_by(&marked_covers(&structure, &page, &none)).is_empty());
    }
```

In `src/ui/delete_tests.rs`:

```rust
/// The counts a tab is waiting for, by the table each is of.
fn counts_asked(harness: &Harness) -> Vec<(crate::backend::RequestId, String)> {
    let sent = harness.app.backend.sent.iter();
    sent.filter_map(|command| match command {
        Command::CountRows { request, query, .. } if query.object.name != "publishers" => {
            Some((*request, query.object.name.clone()))
        }
        _ => None,
    })
    .collect()
}

fn answer_count(harness: &mut Harness, tab: ConnTabId, request: crate::backend::RequestId, rows: u64) {
    let session = harness.app.workspace(tab).unwrap().session;
    harness.app.apply(Action::Backend(crate::backend::Event::Count {
        session,
        request,
        result: Ok(rows),
    }));
    harness.settle();
}

/// Spec 4, "Cascades", and test 10: Review SQL asks for a count for each
/// key that acts, and says each once it has it.
#[test]
fn review_sql_says_what_the_deletes_take_with_them() {
    let (mut harness, tab, id) = publishers_in(Look::macos());
    mark(&mut harness, tab, id, 0);
    // Marking alone asks nothing.
    assert!(counts_asked(&harness).is_empty());
    let show = true;
    harness.app.apply(Action::ReviewEdits { tab, id, show });
    harness.settle();
    let asked = counts_asked(&harness);
    let tables: Vec<&str> = asked.iter().map(|(_, table)| table.as_str()).collect();
    assert_eq!(tables, ["book_covers", "books"]);
    assert!(harness.has("-- also deletes rows in book_covers (ON DELETE CASCADE)"));
    answer_count(&mut harness, tab, asked[0].0, 3);
    answer_count(&mut harness, tab, asked[1].0, 0);
    assert!(harness.has("-- also deletes 3 rows in book_covers (ON DELETE CASCADE)"));
    // No row of `books` has that publisher: no line.
    assert!(!harness.has("-- also clears publisher_id in books rows (ON DELETE SET NULL)"));
    assert!(!harness.has("-- also clears publisher_id in 0 books rows (ON DELETE SET NULL)"));
    // Kept while the marks stand: closing and opening the review asks
    // nothing again.
    let show = false;
    harness.app.apply(Action::ReviewEdits { tab, id, show });
    let show = true;
    harness.app.apply(Action::ReviewEdits { tab, id, show });
    harness.settle();
    assert_eq!(counts_asked(&harness).len(), 2);
    // Another row marked: asked again.
    harness.app.apply(Action::ReviewEdits { tab, id, show: false });
    mark(&mut harness, tab, id, 1);
    harness.app.apply(Action::ReviewEdits { tab, id, show: true });
    harness.settle();
    assert_eq!(counts_asked(&harness).len(), 4);
}

/// The same lines in the production confirm.
#[test]
fn the_production_confirm_says_what_the_deletes_take_with_them() {
    let (mut harness, tab, id) = publishers_in(Look::macos());
    harness.app.workspace_mut(tab).unwrap().environment = crate::env::Environment::Production;
    mark(&mut harness, tab, id, 0);
    harness.app.apply(Action::WriteEdits { tab, id });
    harness.settle();
    let asked = counts_asked(&harness);
    assert_eq!(asked.len(), 2);
    answer_count(&mut harness, tab, asked[0].0, 3);
    answer_count(&mut harness, tab, asked[1].0, 2);
    assert!(harness.has("-- also deletes 3 rows in book_covers (ON DELETE CASCADE)"));
    assert!(harness.has("-- also clears publisher_id in 2 books rows (ON DELETE SET NULL)"));
}

/// And in the Omarchy look's `:diff`.
#[test]
fn diff_says_what_the_deletes_take_with_them() {
    let (mut harness, tab, id) = publishers_in(Look::omarchy());
    mark(&mut harness, tab, id, 0);
    let show = true;
    harness.app.apply(Action::ReviewEdits { tab, id, show });
    harness.settle();
    let asked = counts_asked(&harness);
    answer_count(&mut harness, tab, asked[0].0, 3);
    assert!(painted(&harness, "-- also deletes 3 rows in book_covers (ON DELETE CASCADE)"));
}
```

`Action::ReviewEdits`'s fields, and how a review's comment line reaches AccessKit, are as the file's neighbours use them (`reviewed` in `insert_audit_tests.rs` reads the review's lines from the set, which is the steadier way to assert: `edits(..).review.as_ref().unwrap().lines.contains(&Line::Also(..))`). Use that where `harness.has` finds no line.

- [ ] **Step 3: Run them to see them fail**

Expected: FAIL to compile: no `taken_by`.

- [ ] **Step 4: What to ask, and what was answered**

In `src/edit.rs`, after `Effect`:

```rust
/// What a tab's deletes take with them in other tables, as far as it was
/// counted. Dropped when the marks change.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Taken {
    /// One for each key that acts, in the structure's order. `rows` is
    /// `None` until every count of the key is in, and where one failed.
    pub effects: Vec<Effect>,
    /// The counts still out: the request, and the effect it is of.
    pub pending: Vec<(RequestId, usize)>,
    /// What was counted for each effect so far.
    counted: Vec<u64>,
    /// The effects one of whose counts failed.
    failed: Vec<usize>,
}

impl Taken {
    /// What is waited for: `asked` are the effects with the requests sent
    /// for each.
    pub fn asked(asked: Vec<(Effect, Vec<RequestId>)>) -> Self {
        let mut taken = Self::default();
        for (place, (effect, requests)) in asked.into_iter().enumerate() {
            taken.effects.push(effect);
            taken.counted.push(0);
            taken
                .pending
                .extend(requests.into_iter().map(|request| (request, place)));
        }
        taken
    }

    /// Takes the answer of `request`. Says whether it was one of these.
    pub fn answer(&mut self, request: RequestId, rows: Option<u64>) -> bool {
        let Some(at) = self.pending.iter().position(|(asked, _)| *asked == request) else {
            return false;
        };
        let (_, place) = self.pending.remove(at);
        match rows {
            Some(rows) => {
                if let Some(counted) = self.counted.get_mut(place) {
                    *counted += rows;
                }
            }
            None => self.failed.push(place),
        }
        // Said once the last count of the key is in.
        let waits = self.pending.iter().any(|(_, of)| *of == place);
        let total = (!waits && !self.failed.contains(&place))
            .then(|| self.counted.get(place).copied())
            .flatten();
        if let Some(effect) = self.effects.get_mut(place) {
            match effect {
                Effect::Deletes { rows, .. } | Effect::Clears { rows, .. } => *rows = total,
            }
        }
        true
    }

    /// The lines a review shows: every effect but one that was counted
    /// and takes nothing.
    pub fn lines(&self) -> Vec<Effect> {
        let nothing = |effect: &&Effect| {
            !matches!(
                effect,
                Effect::Deletes { rows: Some(0), .. } | Effect::Clears { rows: Some(0), .. }
            )
        };
        self.effects.iter().filter(nothing).cloned().collect()
    }
}

/// What to count for the rows of `table` that are marked for deletion:
/// each key that points at the table and acts on a delete, as the effect
/// it has, with the counts that say how many rows it reaches. Empty where
/// nothing is marked or no such key is known.
pub fn taken_by(table: &Table<'_>) -> Vec<(Effect, Vec<RowQuery>)> {
    let rows: Vec<usize> = table.deleted.keys().copied().collect();
    let Some(structure) = table.structure.filter(|_| !rows.is_empty()) else {
        return Vec::new();
    };
    structure
        .referenced_by
        .iter()
        .filter_map(|key| {
            let table_name = crate::ui::format::display_safe(&key.table).into_owned();
            let effect = match key.acts()? {
                Acts::Deletes => Effect::Deletes {
                    table: table_name,
                    rows: None,
                },
                Acts::Clears => Effect::Clears {
                    table: table_name,
                    columns: key
                        .columns
                        .iter()
                        .map(|column| crate::ui::format::display_safe(column).into_owned())
                        .collect(),
                    rows: None,
                },
            };
            // No row can point at a NULL: such a key takes nothing, and
            // has no line.
            match pointing_at(table, key, &rows) {
                Some(queries) if queries.is_empty() => None,
                Some(queries) => Some((effect, queries)),
                // The page does not hold what the key points at: said
                // without a number.
                None => Some((effect, Vec::new())),
            }
        })
        .collect()
}

/// The counts of the rows of `key`'s table that point at the page's
/// `rows`: one where the key is one column, one for each row where it is
/// several, since the app's filters are ANDed. `None` where the page
/// does not hold what the key points at; no count where every row holds
/// a NULL there, which no row points at.
fn pointing_at(table: &Table<'_>, key: &ReferencingKey, rows: &[usize]) -> Option<Vec<RowQuery>> {
    let object = ObjectRef::new(key.schema.clone(), key.table.clone());
    let places: Option<Vec<usize>> = key
        .ref_columns
        .iter()
        .map(|name| table.page.columns.iter().position(|c| c.name == *name))
        .collect();
    let places = places.filter(|places| !places.is_empty() && places.len() == key.columns.len())?;
    // Each marked row's values of the columns the key points at, as a
    // filter reads them. A row with a NULL there is pointed at by none.
    let values: Vec<Vec<String>> = rows
        .iter()
        .filter_map(|&row| {
            let held = table.page.rows.get(row)?;
            places
                .iter()
                .map(|&col| held.get(col).filter(|value| !value.is_null()))
                .map(|value| value.map(crate::ui::format::plain_text))
                .collect::<Option<Vec<String>>>()
        })
        .collect();
    let filter = |column: &String, op: FilterOp, value: String| Filter {
        column: column.clone(),
        op,
        value,
    };
    let count = |filters: Vec<Filter>| {
        let mut query = RowQuery::new(object.clone(), 1);
        query.filters = filters;
        query
    };
    // What the filter's `IN` reads back as it is: it splits at commas,
    // trims each value and drops an empty one (`Dialect::in_values`).
    let listable = |value: &String| {
        !value.is_empty() && !value.contains(',') && value.trim() == value.as_str()
    };
    Some(match key.columns.as_slice() {
        // One column and values `IN` can list: one count.
        [column] if values.len() > 1 && values.iter().all(|row| listable(&row[0])) => {
            let list: Vec<&str> = values.iter().map(|row| row[0].as_str()).collect();
            vec![count(vec![filter(column, FilterOp::In, list.join(","))])]
        }
        columns => values
            .into_iter()
            .map(|row| {
                let filters = columns.iter().zip(row);
                count(filters.map(|(column, value)| filter(column, FilterOp::Eq, value)).collect())
            })
            .collect(),
    })
}
```

Imports: `Acts`, `Filter`, `FilterOp`, `ReferencingKey`, `RowQuery` from `tabletist_db`. `format::plain_text` is what following a foreign key turns a value into a filter's text with (`src/app.rs`, near line 611). A binary key's value is written there as the app shows it, which a filter reads back (`Filter::value`'s doc).

`Edits` gains, after `told`:

```rust
    /// What the marked rows' deletes take with them in other tables, once
    /// a review asked. `None` until one does, and again when the marks
    /// change.
    pub taken: Option<Taken>,
```

`mark` and `restore` set `self.taken = None` where they set `self.review = None`. `discard` drops it with the rest. `fail_delete` leaves it.

The counts that are out are the tab's reads like its page and its own count. `Edits` says which they are:

```rust
    /// The counts that are out for this tab: of what its deletes take,
    /// and of who holds a row (task 5 adds those).
    pub fn counts_out(&self) -> Vec<RequestId> {
        let taken = self.taken.iter().flat_map(|taken| &taken.pending);
        taken.map(|(request, _)| *request).collect()
    }
```

`ObjectTab::pending()` (`src/model.rs`, near line 2117) lists them with the requests it lists, so Cancel and closing the tab stop them (`src/app.rs`, near lines 275 and 3610) and the tab reads as busy while they run. `Workspace::forget_session_requests` (`src/model.rs`, near line 3118) drops `taken` beside the doubts whose questions it drops: its counts will not be answered on the new session, and the next review asks again.

Where the reducer changes the marks (`delete_row`, `restore`, and the `DiscardEdits` arm), it reads `object.edits.counts_out()` before the change and cancels each after it (`self.cancel(session, Some(request))`, as `count_rows` cancels the count it supersedes): a mark toggled with the review open does not leave a round of counts running for nothing. A test: with the review open, marking a second row cancels the first round (`cancelled` in `key_picker_tests.rs` reads the cancels that were sent).

- [ ] **Step 5: Ask where a review is made**

In `src/app/editing.rs`, after `review_whole`:

```rust
    /// Asks how many rows the tab's deletes take with them in other
    /// tables, where rows are marked, a key that acts points at the table,
    /// and nothing was asked since the marks last changed. The answers
    /// come as counts (`taken_counted`).
    pub(super) fn count_taken(&mut self, tab: ConnTabId, id: TabId) {
        let asked = self
            .table(tab, id, |table, object| {
                object.edits.taken.is_none().then(|| crate::edit::taken_by(table))
            })
            .flatten()
            .unwrap_or_default();
        if asked.is_empty() {
            return;
        }
        let Some(session) = self.workspace(tab).map(|workspace| workspace.session) else {
            return;
        };
        let mut sent = Vec::with_capacity(asked.len());
        for (effect, queries) in asked {
            let mut requests = Vec::with_capacity(queries.len());
            for query in queries {
                let request = RequestId(self.next_id());
                self.backend.send(Command::CountRows {
                    session,
                    request,
                    query,
                });
                requests.push(request);
            }
            sent.push((effect, requests));
        }
        let taken = Taken::asked(sent);
        let lines = taken.lines();
        if let Some(object) = self.object_tab_mut(tab, id) {
            object.edits.taken = Some(taken);
            if let Some(review) = object.edits.review.as_mut() {
                review.also(&lines);
            }
        }
        self.also_in_prompt(tab, id, &lines);
    }

    /// The production confirmation of the tab `id`, where one is up: its
    /// review says what the deletes take too.
    fn also_in_prompt(&mut self, tab: ConnTabId, id: TabId, lines: &[Effect]) {
        if let Some(Dialog::ConfirmWrite(prompt)) = self.dialog.as_mut()
            && prompt.tab == tab
            && prompt.id == id
        {
            prompt.review.also(lines);
        }
    }

    /// A count came back: one of a tab's counts of what its deletes take,
    /// where it is one. Says whether it was.
    pub(super) fn taken_counted(
        &mut self,
        session: SessionId,
        request: RequestId,
        rows: Option<u64>,
    ) -> bool {
        let Some(tab) = self.tab_for_session(session) else {
            return false;
        };
        let Some(workspace) = self.workspace_mut(tab) else {
            return false;
        };
        let found = workspace.object_tabs_mut().find_map(|object| {
            let taken = object.edits.taken.as_mut()?;
            taken.answer(request, rows).then(|| {
                let lines = taken.lines();
                if let Some(review) = object.edits.review.as_mut() {
                    review.also(&lines);
                }
                (object.id, lines)
            })
        });
        match found {
            Some((id, lines)) => {
                self.also_in_prompt(tab, id, &lines);
                true
            }
            None => false,
        }
    }
```

A review that is made after the counts came in has no lines of its own. Two places make one (`review_edits` only says that one is wanted):

- `make_reviews` (near line 1545): on the fresh review, `review.also(&taken.lines())` where `edits.taken` is `Some`, then `self.count_taken(tab, id)` once the borrow is over.
- `write_edits` (near line 451), for the production confirm: on its local `review`, **before** it is cloned into the Omarchy look's panel and moved into the prompt, `review.also(&lines)` with the lines of `edits.taken` where there are any. The prompt's review and the panel's are then the same review, which is what the Omarchy box asks before it points at the panel (`pointed_at`, `src/ui/write_prompts.rs`, near line 835). After the prompt is set, `self.count_taken(tab, id)`: it returns at once where the counts are in, and else asks and gives the prompt the lines as they come.

A test for the order that is easy to miss: Review SQL first, its counts answered, then Save on production: the prompt has the lines at once, and no count is asked again.

The two copies have the lines too. `review_whole` puts `taken.lines()` on the review it builds. The production sheet's Copy SQL builds its own from the prompt's set (`src/ui/write_prompts.rs`, near line 618): `whole.also(&prompt.review.effects())`. A test of the clipboard's text for each (the neighbours of `the_clipboards_text_says_what_it_is_and_holds_the_whole_statements`).

A count's request is not the tab's own (`object.count.pending`), so nothing cancels it when the tab's count is asked again. A count that is still out when the marks change answers to a `Taken` that is gone, and is dropped.

In `src/app.rs`, `Event::Count` asks the tab's own count first, as it does, and then this:

```rust
            Event::Count {
                session,
                request,
                result,
            } => {
                let own = self
                    .tab_for_session(session)
                    .and_then(|tab| self.workspace(tab))
                    .is_some_and(|workspace| {
                        workspace
                            .object_tabs()
                            .any(|object| object.count.pending == Some(request))
                    });
                if own {
                    // (the block that is there now)
                } else {
                    // One of a tab's counts of what its deletes take.
                    // Task 5 asks here next: `|| self.held_counted(..)`.
                    let rows = result.as_ref().ok().copied();
                    let _ = self.taken_counted(session, request, rows);
                }
            }
```

Keep the existing block as it is inside `if own`. (`object_tabs()` is the read-only twin of `object_tabs_mut()`: use what the workspace has.)

- [ ] **Step 6: Run the tests**

Run: `~/.cargo/bin/cargo test --locked --lib ui::delete_tests`, then `--lib edit::tests`
Expected: PASS.

- [ ] **Step 7: Run the four checks**

- [ ] **Step 8: Commit**

```bash
git add src && git commit -m "Count what a save's deletes take with them

When a review is made while rows are marked (Review SQL, the Omarchy
look's :diff, the production confirm), the app counts the rows of each
table whose foreign key points at them and acts on a delete, and the
review says each. What was counted is kept until the marks change. A key
of one column is one count; a key of several is one for each marked row.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 5: Who holds a row the database would not delete

**Files:**
- Modify: `src/edit.rs` (`Holder`, `Doomed::holder`, `held_by`, `is_held`)
- Modify: `src/app/editing.rs` (`written`'s `FailedDelete` arm, `held_counted`), `src/app.rs` (`Event::Count`)
- Modify: `src/ui/pending_bar.rs` (`note_said`), `src/ui/row_panel.rs` (the note under the line), `src/ui/data_view.rs` (`error_line`, the cells' hint)
- Modify: `src/testing.rs` (a key that holds), `src/ui/delete_tests.rs`

- [ ] **Step 1: Write the failing tests**

In `src/testing.rs`, `publishers_structure` gains a third key, one that holds: as the two of task 4, named `book_reviews_publisher_id_fkey`, of the table `book_reviews`, `on_delete: "NO ACTION"`. (The test of task 4 that expects two effects still holds: a key that holds is no effect.)

In `src/edit.rs`, in `mod tests`:

```rust
    /// Row f: a refusal that names its constraint is counted for that
    /// key; one that names none, for every key that holds.
    #[test]
    fn a_refused_delete_is_asked_who_holds_its_row() {
        let structure = crate::testing::publishers_structure();
        let page = crate::testing::publishers_page();
        let mut edits = Edits::default();
        edits.mark(0);
        let table = marked_covers(&structure, &page, &edits);
        let refusal = |code: &str, constraint: Option<&str>| Error::Query {
            code: Some(code.into()),
            message: "refused".into(),
            detail: None,
            hint: None,
            named: Box::new(tabletist_db::Named {
                constraint: constraint.map(str::to_owned),
                ..tabletist_db::Named::default()
            }),
        };
        // SQLite (the fixture's engine) names no key: every key that
        // holds. NO ACTION is 787, RESTRICT 1811 with the same words.
        let asked = held_by(&table, 0, &refusal("787", None));
        assert_eq!(asked.len(), 1);
        assert_eq!(asked[0].0.table, ObjectRef::new("main", "book_reviews"));
        assert_eq!(asked[0].0.columns, ["publisher_id"]);
        assert_eq!(asked[0].0.values, ["9100000000000000001"]);
        let restrict = Error::Query {
            code: Some("1811".into()),
            message: "FOREIGN KEY constraint failed".into(),
            detail: None,
            hint: None,
            named: Box::default(),
        };
        assert_eq!(held_by(&table, 0, &restrict).len(), 1);
        // 1811 is every trigger's failure: another message is not a key's.
        assert!(held_by(&table, 0, &refusal("1811", None)).is_empty());
        // PostgreSQL names the key: that one. 23001 is its RESTRICT from
        // version 18 on.
        let postgres = Table {
            dialect: Dialect::Postgres,
            ..table
        };
        for code in ["23503", "23001"] {
            let named = refusal(code, Some("book_reviews_publisher_id_fkey"));
            assert_eq!(held_by(&postgres, 0, &named).len(), 1, "{code}");
        }
        let other = refusal("23503", Some("no_such_key"));
        assert!(held_by(&postgres, 0, &other).is_empty());
        // Another failure is no foreign key's, and one engine's code is
        // not another's.
        assert!(held_by(&postgres, 0, &refusal("23514", None)).is_empty());
        assert!(held_by(&postgres, 0, &refusal("787", None)).is_empty());
        assert!(held_by(&table, 0, &Error::query("no")).is_empty());
    }
```

In `src/ui/delete_tests.rs`:

```rust
fn refused(code: &str, number: Option<u32>, constraint: Option<&str>) -> tabletist_db::Error {
    tabletist_db::Error::Query {
        code: Some(code.into()),
        message: "FOREIGN KEY constraint failed".into(),
        detail: None,
        hint: None,
        named: Box::new(tabletist_db::Named {
            number,
            constraint: constraint.map(str::to_owned),
            columns: Vec::new(),
        }),
    }
}

/// Spec 5 and test 8: after the rollback the row says how many rows of
/// which table still point at it.
#[test]
fn a_refused_delete_says_who_still_points_at_the_row() {
    for look in desktop() {
        let (mut harness, tab, id) = publishers_in(look);
        mark(&mut harness, tab, id, 0);
        harness.app.apply(Action::WriteEdits { tab, id });
        let before = counts_asked(&harness).len();
        harness.answer_written(Ok(tabletist_db::WriteOutcome::FailedDelete {
            delete: 0,
            error: refused("787", None, None),
        }));
        harness.settle();
        let asked = counts_asked(&harness);
        assert_eq!(asked.len(), before + 1);
        let (request, table) = asked.last().unwrap();
        assert_eq!(table, "book_reviews");
        answer_count(&mut harness, tab, *request, 3);
        let held = "3 rows in book_reviews still point at this row";
        assert!(harness.has(held), "{}", look.name);
        let doomed = edits(&harness, tab, id).deleted.get(&0).unwrap();
        assert_eq!(doomed.holder.as_ref().map(|holder| holder.rows), Some(3));
        // Sent again, the row has no error and no holder of the last one.
        harness.app.apply(Action::WriteEdits { tab, id });
        let doomed = edits(&harness, tab, id).deleted.get(&0).unwrap();
        assert!(doomed.failed.is_none() && doomed.holder.is_none());
    }
}

/// MySQL's refusal is told by its number, under the state it shares.
#[test]
fn a_mysql_refusal_is_told_by_its_number() {
    let (mut harness, tab, id) = publishers_in(Look::macos());
    mark(&mut harness, tab, id, 0);
    harness.app.apply(Action::WriteEdits { tab, id });
    let before = counts_asked(&harness).len();
    let key = Some("book_reviews_publisher_id_fkey");
    harness.answer_written(Ok(tabletist_db::WriteOutcome::FailedDelete {
        delete: 0,
        error: refused("23000", Some(1451), key),
    }));
    harness.settle();
    assert_eq!(counts_asked(&harness).len(), before + 1);
}

/// The Omarchy look says it on its error line. (Task 6 adds the key that
/// opens the rows to the line, and to this expectation.)
#[test]
fn the_terminal_says_who_holds_a_row() {
    let (mut harness, tab, id) = publishers_in(Look::omarchy());
    // The line is cut to the grid's width where it is painted: with the
    // row panel away it has the room, and a screen reader is told the
    // whole of it either way.
    harness.app.workspace_mut(tab).unwrap().row_panel = false;
    mark(&mut harness, tab, id, 0);
    harness.app.apply(Action::WriteEdits { tab, id });
    harness.answer_written(Ok(tabletist_db::WriteOutcome::FailedDelete {
        delete: 0,
        error: refused("787", None, None),
    }));
    harness.settle();
    let (request, _) = counts_asked(&harness).last().cloned().unwrap();
    answer_count(&mut harness, tab, request, 1);
    let tree = harness.settle();
    let said = "1 row in book_reviews still points at this row";
    let labels = crate::testing::labels(&tree);
    let line = labels.iter().find(|label| label.starts_with("! 1: "));
    assert!(line.is_some_and(|label| label.ends_with(said)), "{labels:?}");
}
```

The spec's sentence is "3 rows in book_covers still point at this row". One row "points": the verb follows the count.

- [ ] **Step 2: Run them to see them fail**

Expected: FAIL to compile: no `held_by`.

- [ ] **Step 3: The model**

In `src/edit.rs`, `Doomed` gains:

```rust
    /// Who holds the row, where its delete was refused by a foreign key
    /// and the rows that point at it were counted. It goes with the
    /// error it explains.
    pub holder: Option<Holder>,
```

`mark` builds a `Doomed` with `holder: None`. `fail_delete` sets `doomed.holder = None` with the new error, and `resend_deletes` clears it where it clears `failed`. After `Taken`:

```rust
/// The rows of another table that still point at a row whose delete a
/// foreign key refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Holder {
    /// The table that has the key, and the key's columns.
    pub table: ObjectRef,
    pub columns: Vec<String>,
    /// The row's values those columns hold, as a filter reads them.
    pub values: Vec<String>,
    /// How many rows hold them.
    pub rows: u64,
}

/// Whether `error` is a foreign key's refusal of a delete, as `dialect`
/// says one.
pub fn is_held(dialect: Dialect, error: &Error) -> bool {
    let Error::Query {
        code,
        message,
        named,
        ..
    } = error
    else {
        return false;
    };
    let code = code.as_deref();
    match dialect {
        // 23001 is `RESTRICT`'s own from version 18 on; before it, and
        // for `NO ACTION`, 23503.
        Dialect::Postgres => matches!(code, Some("23503" | "23001")),
        // Its state, 23000, is other failures' too: the number says it.
        Dialect::MySql => named.number == Some(1451),
        // `NO ACTION` is SQLITE_CONSTRAINT_FOREIGNKEY. `RESTRICT` is
        // raised as a trigger's failure, SQLITE_CONSTRAINT_TRIGGER, which
        // is every trigger's: told from the others by SQLite's own words,
        // which no setting translates.
        Dialect::Sqlite => {
            code == Some("787")
                || (code == Some("1811") && message == "FOREIGN KEY constraint failed")
        }
    }
}

/// What to count to learn who holds the page's row `row`, whose delete
/// failed with `error`: for each key that could be the one, who would be
/// held (`rows` not counted yet) and the count. The key the error names,
/// where it names one. Else every key that holds.
pub fn held_by(table: &Table<'_>, row: usize, error: &Error) -> Vec<(Holder, RowQuery)> {
    let Some(structure) = table.structure.filter(|_| is_held(table.dialect, error)) else {
        return Vec::new();
    };
    let named = match error {
        Error::Query { named, .. } => named.constraint.as_deref(),
        _ => None,
    };
    structure
        .referenced_by
        .iter()
        .filter(|key| match named {
            Some(name) => key.name.as_deref() == Some(name),
            None => key.holds(),
        })
        .filter_map(|key| {
            let mut queries = pointing_at(table, key, &[row])?;
            let query = queries.pop().filter(|_| queries.is_empty())?;
            let values = query.filters.iter().map(|filter| filter.value.clone());
            let held = Holder {
                table: query.object.clone(),
                columns: key.columns.clone(),
                values: values.collect(),
                rows: 0,
            };
            Some((held, query))
        })
        .collect()
}
```

PostgreSQL names a constraint by its name alone, and two tables can have a key of one name: where `named` matches several, each is counted, and the first that holds rows is said, as for SQLite.

- [ ] **Step 4: Ask after the rollback, and keep the answer**

`Edits` gains, after `taken`:

```rust
    /// The counts out for a row whose delete was refused: the request,
    /// the page's row, and who would hold it.
    pub holders: Vec<(RequestId, usize, Holder)>,
```

dropped by `restore` (for that row: `self.holders.retain(|(_, of, _)| *of != row)`), by `resend_deletes` and by `discard`. `counts_out` (task 4) lists their requests after `taken`'s, so Cancel and closing the tab stop them, and `forget_session_requests` drops them with `taken`.

In `written` (`src/app/editing.rs`), the `FailedDelete` arm asks. The borrow of `object` ends before the backend is used, so collect first:

```rust
            Ok(WriteOutcome::FailedDelete { delete, error }) => {
                let row = saving.deletes.get(delete).copied();
                // Who holds the row, counted now that the save is undone.
                let asked = row
                    .and_then(|row| {
                        let table = Table::of(workspace_ref, object)?;
                        Some((row, crate::edit::held_by(&table, row, &error)))
                    });
                object.edits.fail_delete(row, error);
                // The row panel says what stands against the row.
                object.fields = None;
                // (after the match, with `asked`: send each count and keep
                // `(request, row, held)` in `object.edits.holders`)
            }
```

`written` holds `workspace` mutably while it holds `object`: build the `Table` from what the arm has (`Table::of` needs `&Workspace` and `&ObjectTab`), or compute `asked` through `self.table(tab, id, ..)` after the match, reading the error from `object.edits.deleted[&row].failed`. The second is the simpler borrow: do that, in a small method `count_holders(&mut self, tab, id, row)` called after the match where the arm set a flag.

```rust
    /// Asks who holds the page's row `row`, whose delete was refused.
    fn count_holders(&mut self, tab: ConnTabId, id: TabId, row: usize) {
        let asked = self
            .table(tab, id, |table, object| {
                let error = object.edits.deleted.get(&row)?.failed.as_ref()?;
                Some(crate::edit::held_by(table, row, error))
            })
            .flatten()
            .unwrap_or_default();
        let Some(session) = self.workspace(tab).map(|workspace| workspace.session) else {
            return;
        };
        let mut out = Vec::with_capacity(asked.len());
        for (held, query) in asked {
            let request = RequestId(self.next_id());
            self.backend.send(Command::CountRows {
                session,
                request,
                query,
            });
            out.push((request, row, held));
        }
        if let Some(object) = self.object_tab_mut(tab, id) {
            object.edits.holders = out;
        }
    }

    /// A count came back: one of a refused delete's, where it is one. The
    /// first key that holds rows is the row's holder. Says whether it was.
    pub(super) fn held_counted(
        &mut self,
        session: SessionId,
        request: RequestId,
        rows: Option<u64>,
    ) -> bool {
        let Some(tab) = self.tab_for_session(session) else {
            return false;
        };
        let Some(workspace) = self.workspace_mut(tab) else {
            return false;
        };
        for object in workspace.object_tabs_mut() {
            let edits = &mut object.edits;
            let Some(at) = edits.holders.iter().position(|(asked, ..)| *asked == request) else {
                continue;
            };
            let (_, row, holder) = edits.holders.remove(at);
            // Only of the refusal it was asked about: a row that was sent
            // again since has no error for this to explain.
            if let (Some(rows), Some(doomed)) = (rows.filter(|rows| *rows > 0), edits.deleted.get_mut(&row))
                && doomed.failed.is_some()
                && doomed.holder.is_none()
            {
                doomed.holder = Some(Holder { rows, ..holder });
                // The row panel says it under the row's line.
                object.fields = None;
            }
            return true;
        }
        false
    }
```

In `src/app.rs`, the last branch of `Event::Count` asks this after task 4's: `let _ = self.taken_counted(session, request, rows) || self.held_counted(session, request, rows);`.

- [ ] **Step 5: The sentence**

In `src/ui/pending_bar.rs`, a function the three places share:

```rust
/// "3 rows in book_covers still point at this row": who holds a row
/// whose delete a foreign key refused.
pub(crate) fn holder_said(held: &crate::edit::Holder, locale: Locale) -> String {
    let plural = u32::try_from(held.rows).unwrap_or(u32::MAX);
    format!(
        "{} {} {} {} {}",
        held.rows,
        ngettext(locale, "row", "rows", plural),
        gettext(locale, "in"),
        format::display_safe(&held.table.name),
        ngettext(
            locale,
            "still points at this row",
            "still point at this row",
            plural
        )
    )
}
```

- The pending bar: `note_said`'s `FailedDelete` arm adds the sentence after the database's words, where the row's mark has a holder: `object.edits.deleted.get(row).and_then(|doomed| doomed.holder.as_ref())`. The arm takes `row` from the note.
- The row panel: under the database's words (the `form_note` run 2 added), a second note with the sentence.
- The grid's cells: the hint of a refused row's cells (`Changes::mark`) ends with the sentence.
- The Omarchy error line (`error_line`): `"! {number}:  {failure} · {holder}"`. Task 6 adds the key that opens the rows.

- [ ] **Step 6: Run the tests, then the four checks**

Expected: PASS.

- [ ] **Step 7: Commit**

```bash
git add src && git commit -m "Say who holds a row the database would not delete

After a delete a foreign key refuses, and the rollback, the app counts
the rows that point at the row: of the key the database names, or of
every key that holds where it names none. The pending bar, the row
panel, the row's cells and the Omarchy error line say how many rows of
which table still point at it.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 6: Open rows

**Files:**
- Modify: `src/model.rs` (`Action::OpenHolders`), `src/app.rs` (the action; `follow_foreign_key` by several columns)
- Modify: `src/ui/pending_bar.rs` (a link), `src/ui/row_panel.rs` (a button), `src/ui/keys.rs` (`gd`), `src/ui/data_view.rs` (the line's key)
- Modify: `src/ui/delete_tests.rs`

- [ ] **Step 1: Write the failing tests**

```rust
/// A row of `publishers` whose delete was refused, held by three rows of
/// `book_reviews`.
fn held_publisher(look: Look) -> (Harness, ConnTabId, TabId) {
    let (mut harness, tab, id) = publishers_in(look);
    mark(&mut harness, tab, id, 0);
    harness.app.apply(Action::WriteEdits { tab, id });
    harness.answer_written(Ok(tabletist_db::WriteOutcome::FailedDelete {
        delete: 0,
        error: refused("787", None, None),
    }));
    harness.settle();
    let (request, _) = counts_asked(&harness).last().cloned().unwrap();
    answer_count(&mut harness, tab, request, 3);
    (harness, tab, id)
}

/// The table the workspace has in front, and its filter rows.
fn in_front(harness: &Harness, tab: ConnTabId) -> (String, Vec<(String, String)>) {
    let workspace = harness.app.workspace(tab).unwrap();
    let object = workspace.object_tab(workspace.active_tab.unwrap()).unwrap();
    let rows = object.filter.rows.iter();
    (
        object.object.name.clone(),
        rows.map(|row| (row.column.clone(), row.value.clone())).collect(),
    )
}

/// Spec 5: Open rows opens the referencing table, filtered to the rows
/// that point at the row. The publishers' tab keeps its mark.
#[test]
fn open_rows_opens_the_rows_that_hold_the_row() {
    for look in desktop() {
        let (mut harness, tab, id) = held_publisher(look);
        harness.click("Open rows");
        harness.settle();
        let (table, filters) = in_front(&harness, tab);
        assert_eq!(table, "book_reviews");
        assert_eq!(
            filters,
            [("publisher_id".to_owned(), "9100000000000000001".to_owned())]
        );
        assert!(edits(&harness, tab, id).deleted.contains_key(&0));
    }
}

/// Where the table points at itself the rows that hold a row are in the
/// tab that has the mark: opening them would drop its page, so the app
/// asks first, as it does for any tab that holds changes.
#[test]
fn open_rows_asks_first_where_the_rows_are_the_tabs_own() {
    // On `genres` (`crate::testing::genres_structure`, with a key of its
    // own in `referenced_by` that holds): mark a genre, refuse its
    // delete, answer the count, press Open rows, and hold that the Leave
    // prompt is up and the mark still stands.
}

/// Row g: in the Omarchy look `gd` on the row opens them.
#[test]
fn gd_on_a_refused_row_opens_the_rows_that_hold_it() {
    let (mut harness, tab, _id) = held_publisher(Look::omarchy());
    type_key(&mut harness, Key::G, "g");
    type_key(&mut harness, Key::D, "d");
    assert_eq!(in_front(&harness, tab).0, "book_reviews");
}
```

Two controls may be named "Open rows" (the bar's and the panel's): `harness.click` takes the first. Close the row panel in one pass of the loop (`row_panel = false`) and assert the panel's button with `button(&mut harness, "Open rows")` in the other, so both are held.

- [ ] **Step 2: Run them to see them fail**

Expected: FAIL to compile: no `Action::OpenHolders`.

- [ ] **Step 3: The action**

In `src/model.rs`, after `RestoreRow`:

```rust
    /// Open the rows that hold the selected row, whose delete a foreign
    /// key refused: their table, filtered to them.
    OpenHolders {
        tab: ConnTabId,
        id: TabId,
    },
```

with a third field, `row: usize`: the page's row. The pending bar's button is of the bar's note, which names its row (`Note::FailedDelete { row, .. }`) and may not be of the selected one; the row panel and `gd` pass the selected row.

It is not listed in `dropped_under_a_prompt`: it changes nothing that is pending. `follow_foreign_key` (`src/app.rs`, near line 3618) opens a table filtered by one column. Give it the rows of a filter: its `column: String, value: String` become `by: Vec<(String, String)>`, and its `FilterRow` is built for each. `Action::FollowForeignKey` carries the same list in place of its `column` and `value`, since the function holds itself as that action where the target tab has edits. Every place that builds the action or calls the function passes one pair: `FollowSelectedKey` and the `FollowForeignKey` arm in `src/app.rs`, the row panel's "Open book" link (`src/ui/row_panel.rs`, near line 2723), and the tests near `src/ui/mod.rs:367` and `src/app.rs:12606`. Then:

```rust
            Action::OpenHolders { tab, id, row } => {
                let holder = self
                    .workspace(tab)
                    .and_then(|workspace| workspace.object_tab(id))
                    .and_then(|object| object.edits.deleted.get(&row)?.holder.clone());
                if let Some(holder) = holder {
                    let by = holder.columns.into_iter().zip(holder.values).collect();
                    self.follow_foreign_key(tab, holder.table, by);
                }
            }
```

- [ ] **Step 4: The link, the button and the key**

- **The pending bar.** Its buttons are placed from the right, each a `ButtonSpec` (`place`, near line 372), and what the bar says is cut where they begin. Where the note is `FailedDelete` and its row has a holder, one more quiet button, "Open rows", stands with them, as "Review SQL" does: `actions.push(Action::OpenHolders { tab, id, row })`. It folds into the bar's menu with the others where the bar is narrow.
- **The row panel.** After the note with the sentence, a small button "Open rows", as wide as its word: the same `ButtonSpec` the line's Restore has.
- **The Omarchy look.** In `letters` (`src/ui/keys.rs`), the arm of `Command::OpenReferencedRow` (near line 1899) asks first whether the selected row is one whose delete was refused and has a holder: if so it pushes `OpenHolders` with the selected row, else what it pushes now. The error line ends with `· {gd} open rows`, the key from the keymap (`keys::written(.., Command::OpenReferencedRow)`) and the words in the look's lower case, and task 5's Omarchy test gains them.

`gd` keeps its name in the keymap, "Open referenced row". The website's key tables say what it does on a refused row (task 7).

- [ ] **Step 5: Run the tests, then the four checks**

Expected: PASS, `tests/keys.rs` among them: no key is spelled outside the keymap, and `delete_tests.rs` is one of the files that may name one.

- [ ] **Step 6: Commit**

```bash
git add src && git commit -m "Open the rows that hold a row from where it says so

Open rows, beside the sentence in the pending bar and in the row panel,
opens the referencing table filtered to the rows that still point at the
row. In the Omarchy look gd on the row does, and the error line names
it. Following a foreign key takes a filter of several columns for it.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 7: The website, the spec, and the pull request's words

**Files:**
- Modify: `docs/_guide/editing-data.md` (the section "Delete a row"), `docs/_guide/omarchy.md` (the row of `gd`)
- Modify: `docs/superpowers/specs/2026-10-12-deleting-rows-design.md` ("As built")

- [ ] **Step 1: The guide**

In "Delete a row", after the paragraph on Review SQL:

```markdown
Where another table's foreign key points at the rows you delete, Review
SQL says what the save does there, under its statements: how many rows
that table loses (`ON DELETE CASCADE`), or how many have the key cleared
(`ON DELETE SET NULL`). The production confirmation shows the same lines.
Only the rows that point at yours directly are counted.
```

and the bullet on a refused delete becomes:

```markdown
- If the database refuses a delete because a foreign key still points at
  the row, nothing is saved. The row stays marked and says how many rows
  of which table point at it. **Open rows** opens them (`gd` in the
  Omarchy look). Any other refusal shows the database's message.
```

In `docs/_guide/omarchy.md`, the row of `gd` gains: "On a row whose delete a foreign key refused, the rows that hold it."

- [ ] **Step 2: The spec**

In "As built", the item "Cascades and the restrict count. Not built yet" becomes what this run built, with rows a, b, d, e, g, h, i and j of this plan's table, one line each.

- [ ] **Step 3: Build and check the site**

```bash
cd docs && bin/check
```

- [ ] **Step 4: Run the four checks, then commit**

```bash
git add docs && git commit -m "Say on the website what a delete takes with it, and who holds a row

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

## After the last task

- Run the four checks and the shots' clippy on the branch's head, and `cd docs && bin/check`.
- Mark this plan built: a "Status" paragraph under its head with what was built otherwise than written.
- Push. Pull request #120 runs its CI on the push; Auto-fix is on, so do not poll. Once the run is done, read the two integration jobs' logs for `a_structure_lists_the_keys_that_point_at_its_table ... ok`.
- Edit the pull request's description (`gh pr edit 120 --body-file ..`): tests 8 and 10 are built; "No cascade warnings" leaves the list of what was built otherwise, and rows a, b, e, g and i of this plan's table join it.
- Offer run 4 (many rows at once) as a spec of its own.
