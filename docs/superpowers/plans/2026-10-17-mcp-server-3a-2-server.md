# MCP server, Phase 3a, run 2: the server's side of masked columns. Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** The MCP server knows each connection's masked columns and says so: `list_connections` names them, `describe_schema` marks a masked column and a view that reads one, and leaves out what would say what a masked column's values are, and `run_query` marks a masked column of its answer, whose values are `•••`. The log keeps which columns an answer masked. A generated column of a table with a masked column is masked with it. Nothing is visible yet: the app's gateway passes no masked column until run 3 reads the setting from `connections.toml`.

**Architecture:** `tabletist-db`'s `Hidden::masking` gains entries of its own for the generated columns of a masked column's table, from what each driver's catalog says is generated (`Catalog::generated`), and `Hidden::reads_masked` says which view reads a masked column. In `tabletist-mcp`, `Listed` carries each connection's masked columns, and `Session::hidden` and `Session::read` take them. The tools write what they come to, and the log's `masked` field, there since C.1, is filled.

**Tech Stack:** Rust 2024, `tabletist-db`'s drivers, `rmcp` through `tabletist-mcp`, SQLite for the log.

---

## Where this sits

Phase 3a of `docs/superpowers/plans/2026-10-13-mcp-server.md`, masked columns, in five runs. Runs 1a and 1b (pull requests 150 and 152) built every check in `crates/tabletist-db`. This run, 2, is the server's side, as 1c-2a was for hidden tables. Runs 3 and 4 are the setting and the editor.

## What the plan asks, and what is built

| The plan | Built | Why |
|---|---|---|
| G: "`describe_schema`'s mark" | A masked column marked `"masked": true`, without its default, the values it may hold, the list MySQL writes into an `enum` or a `set` type, or its comment | Each says what its values are, or may: a default is one, and a comment's author may give one as an example (found by the review) |
| B.4, rule 6: "a view that reads a masked column is refused" | Listed in `describe_schema`, marked `"reads_masked": "orders.email"` | Agreed on 2026-10-17: the agent knows before it asks, as it is told a hidden table's name |
| C.1: `masked TEXT -- JSON array of column names` | The answer's masked columns, by their names in the answer | As C.1 has it. An entry (`orders.email`) is not what the answer says a column comes from |
| Not in the plan | A generated column of a table with a masked column is masked with it | Found by the review: `lower(email)` as a generated column answered the email in full, on PostgreSQL and SQLite |

## Before you start

- Work on a branch of `main`. One commit for each task, in the order below. Every commit passes `cargo fmt --all --check`, `cargo clippy --locked --workspace --all-targets -- -D warnings` and `cargo test --locked --workspace --all-targets`.
- On Igor's machine `cargo` is `~/.cargo/bin/cargo` (the mise shim fails), and a build in a worktree needs `CARGO_TARGET_DIR` set to that worktree's `target/`.
- The server tests of `crates/tabletist-db/tests/agent.rs` run against PostgreSQL and MySQL where `TABLETIST_TEST_PG_URL` and `TABLETIST_TEST_MYSQL_URL` are set (`docker compose up -d --wait postgres mysql`), and print "skipped" otherwise.
- Read `AGENTS.md`. The diffs below apply with `git apply`. Run `cargo fmt --all` before each commit: the code here is already formatted.

## What was run before this plan was written

Every task's state was built here, passed `cargo fmt --check` and clippy with `-D warnings`, and its tests failed and then passed as each step says. The last state passed `cargo test --locked --workspace --all-targets` with both test servers (3,564 tests, none failed) and `RUSTDOCFLAGS='-D warnings' cargo doc --locked --workspace --no-deps`. Not run: CI.

## What this run settles

| Question | Decision | Why |
|---|---|---|
| What `describe_schema` shows of a view that reads a masked column | Listed, marked `reads_masked` with the column it reads, in the list and in its description | Agreed on 2026-10-17 |
| What it shows of a masked column | Its name, its type (an `enum` or a `set` without its values), whether it is nullable or generated, and `masked: true` | Its default, the values it may hold and its comment can each say what its values are (found by the review) |
| A generated column of a table with a masked column | Masked with it, by an entry of its own (`public.orders.low`) | One computed from it is its value in another form (found by the review). Whether it is computed from the masked column is not asked: it errs toward masking |
| What the log keeps | The answer's masked columns, by their names in the answer, as a JSON list; nothing where none was masked or the call answered no rows | As C.1 has it |
| What the app passes | No masked column, until the setting is read in run 3 | As 1c-2a passed "All" before 1c-2b |

### What this run leaves out

- The setting in `connections.toml`, from which the app's gateway reads the masked columns: run 3.
- The editor's Masked columns control and the website's guide: run 4.
- A column of another table that holds a copy of a masked column's values (a key that points at it): it is its own table's.

## File structure

| File | What changes |
|---|---|
| `crates/tabletist-db/src/agent/hidden.rs` | `Catalog::generated`; a generated column of a masked column's table masked with it; `Hidden::reads_masked` |
| `crates/tabletist-db/src/pg/hidden.rs`, `mysql/hidden.rs`, `sqlite.rs` | Which columns of the masked columns' tables are generated |
| `crates/tabletist-db/tests/agent.rs` | A generated column, on all three |
| `crates/tabletist-mcp/src/gateway.rs` | `Listed::masked`; `Session::hidden` and `read` take the masked columns |
| `crates/tabletist-mcp/src/tools/connections.rs` | `list_connections`' `masked` |
| `crates/tabletist-mcp/src/tools/schema.rs` | `describe_schema`'s marks, and what it leaves out of a masked column |
| `crates/tabletist-mcp/src/tools/query.rs`, `answer.rs`, `tools.rs`, `server.rs`, `log.rs` | `run_query`'s mark, and the log's `masked` |
| `crates/tabletist-mcp/tests/support/mod.rs`, `tests/tools.rs` | `Shop::mask`; the three tools with masked columns |
| `src/backend/agents.rs` | The app's gateway passes no masked column |
| `docs/superpowers/plans/2026-10-13-mcp-server.md` | B.4 as built for 3a-2; G |

---

### Task 1: A generated column of a table with a masked column is masked with it

- `Catalog::generated`: the generated columns of the tables the masked columns' entries name, as each driver says: PostgreSQL's `attgenerated`, MySQL's `EXTRA` (`VIRTUAL GENERATED`, `STORED GENERATED`, MariaDB's `PERSISTENT`), SQLite's `pragma_table_xinfo` (`hidden` 2 or 3).
- `Hidden::masking` gives each generated column of a table with a masked column, its partitions and child tables too, an entry of its own (`public.orders.low`), with which the rule, the fences and the values treat it as any masked column. Such an entry never stops agents as one that names no column: it is the catalog's own.

**Files:**
- Modify: `crates/tabletist-db/src/agent/hidden.rs`, `crates/tabletist-db/src/pg/hidden.rs`, `crates/tabletist-db/src/mysql/hidden.rs`, `crates/tabletist-db/src/sqlite.rs`, `crates/tabletist-db/tests/agent.rs`

- [ ] **Step 1: Write the failing test**

````diff
diff --git a/crates/tabletist-db/src/agent/hidden.rs b/crates/tabletist-db/src/agent/hidden.rs
index b322b74..31e43e6 100644
--- a/crates/tabletist-db/src/agent/hidden.rs
+++ b/crates/tabletist-db/src/agent/hidden.rs
@@ -2134,6 +2134,39 @@ mod tests {
         );
     }
 
+    #[test]
+    fn a_generated_column_of_a_table_with_a_masked_column_is_masked_with_it() {
+        let catalog = Catalog {
+            relations: vec![
+                relation("public", "orders", &[]),
+                relation("public", "books", &[]),
+            ],
+            columns: vec![
+                ("public".into(), "orders".into(), "email".into()),
+                ("public".into(), "orders".into(), "low".into()),
+                ("public".into(), "books".into(), "slug".into()),
+            ],
+            generated: vec![
+                ("public".into(), "orders".into(), "low".into()),
+                ("public".into(), "books".into(), "slug".into()),
+            ],
+            ..Catalog::default()
+        };
+        let masked: Vec<ColumnName> = vec!["orders.email".parse().unwrap()];
+        let hidden = Hidden::masking(Dialect::Postgres, &catalog, &Tables::All, &masked);
+        assert_eq!(
+            hidden.masks(Some("public"), "orders", "low"),
+            Some("public.orders.low".into())
+        );
+        assert_eq!(hidden.masks(Some("public"), "books", "slug"), None);
+        assert_eq!(
+            statement(&hidden, "SELECT id FROM orders WHERE low LIKE 'a%'"),
+            masked_used("public.orders.low", MaskedUse::Named("low".into()))
+        );
+        assert_eq!(statement(&hidden, "SELECT id, low FROM orders"), Ok(()));
+        assert_eq!(hidden.check_settings(), Ok(()));
+    }
+
     #[test]
     fn a_masked_column_renamed_or_dropped_stops_agents_on_the_connection() {
         let gone = masking(
diff --git a/crates/tabletist-db/tests/agent.rs b/crates/tabletist-db/tests/agent.rs
index cdf5961..6a072be 100644
--- a/crates/tabletist-db/tests/agent.rs
+++ b/crates/tabletist-db/tests/agent.rs
@@ -678,6 +678,38 @@ async fn a_masked_columns_values_are_never_answered(connection: &Connection, on:
     assert_eq!(found, Ok(Vec::new()), "{on}");
 }
 
+/// A generated column computed from a masked column, on the table
+/// `gift_notes` the caller made (`kind`, and `shout` from it): masked with
+/// it, its values answered as `•••` and a use of it refused.
+async fn a_column_computed_from_a_masked_one_is_masked(connection: &Connection, on: &str) {
+    let found = masking(
+        connection,
+        &["gift_notes.kind"],
+        "SELECT id, shout FROM gift_notes WHERE id = 1",
+    )
+    .await
+    .map(|answer| (answer.masked, answer.rows));
+    assert_eq!(
+        found,
+        Ok((
+            vec![false, true],
+            vec![vec![Value::Int(1), Value::Text(agent::MASKED.into())]]
+        )),
+        "{on}"
+    );
+    let found = masking(
+        connection,
+        &["gift_notes.kind"],
+        "SELECT id FROM gift_notes WHERE shout = 'SELF'",
+    )
+    .await
+    .unwrap_err();
+    assert!(
+        matches!(found, Error::Agent(Refusal::MaskedColumnUsed { .. })),
+        "{on}: {found:?}"
+    );
+}
+
 /// What every engine is asked.
 async fn an_agent_reads_the_bookshop(connection: &Connection, on: &str) {
     a_read_answers_its_rows(connection, on).await;
@@ -762,6 +794,17 @@ mod sqlite {
         a_masked_column_is_used_only_as_it_is(&connection, "sqlite").await;
     }
 
+    #[tokio::test]
+    async fn a_column_computed_from_a_masked_one_is_masked_on_sqlite() {
+        let (connection, _dir) = bookshop_with(
+            "CREATE TABLE gift_notes (id INTEGER PRIMARY KEY, kind TEXT,
+                 shout TEXT GENERATED ALWAYS AS (upper(kind)) VIRTUAL);
+             INSERT INTO gift_notes (id, kind) VALUES (1, 'self');",
+        )
+        .await;
+        a_column_computed_from_a_masked_one_is_masked(&connection, "sqlite").await;
+    }
+
     #[tokio::test]
     async fn a_masked_columns_values_are_never_answered_on_sqlite() {
         let (connection, _dir) =
@@ -1115,6 +1158,26 @@ mod postgres {
         a_masked_column_is_used_only_as_it_is(&connection, "postgres").await;
     }
 
+    #[tokio::test]
+    async fn a_column_computed_from_a_masked_one_is_masked_on_postgres() {
+        let Some(connection) = bookshop().await else {
+            return;
+        };
+        let url = std::env::var("TABLETIST_TEST_PG_URL").unwrap();
+        let owner = admin(&url, Some("bookshop")).await;
+        owner
+            .batch_execute(
+                "DROP TABLE IF EXISTS gift_notes;
+                 CREATE TABLE gift_notes (id int PRIMARY KEY, kind text,
+                     shout text GENERATED ALWAYS AS (upper(kind)) STORED);
+                 INSERT INTO gift_notes (id, kind) VALUES (1, 'self');",
+            )
+            .await
+            .unwrap();
+        a_column_computed_from_a_masked_one_is_masked(&connection, "postgres").await;
+        owner.batch_execute("DROP TABLE gift_notes").await.unwrap();
+    }
+
     #[tokio::test]
     async fn a_masked_columns_values_are_never_answered_on_postgres() {
         let Some(connection) = bookshop().await else {
@@ -1420,6 +1483,28 @@ mod mysql {
         a_masked_column_is_used_only_as_it_is(&connection, "mysql").await;
     }
 
+    #[tokio::test]
+    async fn a_column_computed_from_a_masked_one_is_masked_on_mysql() {
+        let Some(connection) = bookshop().await else {
+            return;
+        };
+        let url = std::env::var("TABLETIST_TEST_MYSQL_URL").unwrap();
+        let mut owner = admin(&url).await;
+        for sql in [
+            "DROP TABLE IF EXISTS bookshop.gift_notes",
+            "CREATE TABLE bookshop.gift_notes (id int PRIMARY KEY, kind varchar(10), \
+             shout varchar(10) GENERATED ALWAYS AS (upper(kind)) VIRTUAL)",
+            "INSERT INTO bookshop.gift_notes (id, kind) VALUES (1, 'self')",
+        ] {
+            owner.query_drop(sql).await.unwrap();
+        }
+        a_column_computed_from_a_masked_one_is_masked(&connection, "mysql").await;
+        owner
+            .query_drop("DROP TABLE bookshop.gift_notes")
+            .await
+            .unwrap();
+    }
+
     #[tokio::test]
     async fn a_masked_columns_values_are_never_answered_on_mysql() {
         let Some(connection) = bookshop().await else {
````

- [ ] **Step 2: Run it to see that it does not build**

Run: `cargo test --locked -p tabletist-db --lib generated_column`

Expected: it does not build:

```text
error[E0560]: struct `agent::hidden::Catalog` has no field named `generated`
error: could not compile `tabletist-db` (lib test) due to 1 previous error
```

- [ ] **Step 3: The generated columns, and their entries**

````diff
diff --git a/crates/tabletist-db/src/agent/hidden.rs b/crates/tabletist-db/src/agent/hidden.rs
index 31e43e6..f07ddd0 100644
--- a/crates/tabletist-db/src/agent/hidden.rs
+++ b/crates/tabletist-db/src/agent/hidden.rs
@@ -257,6 +257,10 @@ pub struct Catalog {
     /// view reads (PostgreSQL's): it is read for one. Only where something
     /// is masked.
     pub definitions: Vec<(String, String, String)>,
+    /// The generated columns of the tables the masked columns' entries
+    /// name, by schema, table and column: one may be computed from a
+    /// masked column, and is masked with it.
+    pub generated: Vec<(String, String, String)>,
 }
 
 /// A relation of the catalog, as [`Hidden`] keeps it.
@@ -346,6 +350,29 @@ impl Hidden {
         for at in 0..catalog.relations.len() {
             hidden_why(at, catalog, &depends, tables, &mut why);
         }
+        // A generated column of a table with a masked column may be
+        // computed from it (`lower(email)`): it is masked too, by an entry
+        // of its own.
+        let named = masked_tables(catalog, &depends, masked);
+        let mut every = masked.to_vec();
+        for (schema, table, column) in &catalog.generated {
+            let of_masked = named
+                .iter()
+                .any(|(own, name, _)| same(own, schema) && same(name, table));
+            if of_masked
+                && !every
+                    .iter()
+                    .any(|entry| entry.names(Some(schema), table, column))
+            {
+                every.push(ColumnName {
+                    schema: Some(schema.clone()),
+                    table: table.clone(),
+                    column: column.clone(),
+                });
+            }
+        }
+        let masked_entries = masked;
+        let masked = every.as_slice();
         let masked_tables = masked_tables(catalog, &depends, masked);
         let masks = masked_views(dialect, catalog, &depends, masked, &masked_tables);
         let mut relations: HashMap<String, Vec<Listed>> = HashMap::new();
@@ -372,7 +399,7 @@ impl Hidden {
             true => catalog.unnamed_calls.clone(),
             false => Vec::new(),
         };
-        let masked_missing = masked
+        let masked_missing = masked_entries
             .iter()
             .filter(|entry| {
                 !system_catalog(dialect, entry.schema.as_deref(), &entry.table)
diff --git a/crates/tabletist-db/src/mysql/hidden.rs b/crates/tabletist-db/src/mysql/hidden.rs
index 6cc14b3..46ac324 100644
--- a/crates/tabletist-db/src/mysql/hidden.rs
+++ b/crates/tabletist-db/src/mysql/hidden.rs
@@ -283,7 +283,9 @@ pub(super) async fn catalog(
         ))
         .await
         .map_err(query_error)?;
-    let columns: Vec<(String, String, String)> = match masked.is_empty() {
+    // With whether each is generated, which `EXTRA` says on every server
+    // (`VIRTUAL GENERATED`, `STORED GENERATED`, MariaDB's `PERSISTENT`).
+    let listed: Vec<(String, String, String, String)> = match masked.is_empty() {
         true => Vec::new(),
         false => {
             let tables: Vec<String> = masked
@@ -293,7 +295,8 @@ pub(super) async fn catalog(
             let marks = vec!["?"; tables.len()].join(", ");
             conn.exec(
                 format!(
-                    "SELECT TABLE_SCHEMA, TABLE_NAME, COLUMN_NAME FROM information_schema.COLUMNS \
+                    "SELECT TABLE_SCHEMA, TABLE_NAME, COLUMN_NAME, UPPER(EXTRA) \
+                     FROM information_schema.COLUMNS \
                      WHERE TABLE_SCHEMA NOT IN {SYSTEM} AND LOWER(TABLE_NAME) IN ({marks}) {WHOLE}"
                 ),
                 tables,
@@ -302,10 +305,20 @@ pub(super) async fn catalog(
             .map_err(query_error)?
         }
     };
+    let generated = listed
+        .iter()
+        .filter(|(_, _, _, extra)| extra.contains("GENERATED") || extra.contains("PERSISTENT"))
+        .map(|(schema, table, column, _)| (schema.clone(), table.clone(), column.clone()))
+        .collect();
+    let columns = listed
+        .into_iter()
+        .map(|(schema, table, column, _)| (schema, table, column))
+        .collect();
     Ok(Catalog {
         relations,
         routines,
         columns,
+        generated,
         ..Catalog::default()
     })
 }
diff --git a/crates/tabletist-db/src/pg/hidden.rs b/crates/tabletist-db/src/pg/hidden.rs
index f66690e..f8bff4a 100644
--- a/crates/tabletist-db/src/pg/hidden.rs
+++ b/crates/tabletist-db/src/pg/hidden.rs
@@ -87,9 +87,9 @@ ORDER BY n.nspname, c.relname"
 }
 
 /// The columns of the relations of the names in `$1`, folded to lower
-/// case: the masked columns' tables.
+/// case: the masked columns' tables; and whether each is generated.
 const COLUMNS_SQL: &str = "\
-SELECT n.nspname, c.relname, a.attname
+SELECT n.nspname, c.relname, a.attname, a.attgenerated <> ''
 FROM pg_attribute a
 JOIN pg_class c ON c.oid = a.attrelid
 JOIN pg_namespace n ON n.oid = c.relnamespace
@@ -587,8 +587,8 @@ pub(super) async fn catalog(
         .iter()
         .filter_map(|row| Some(row.get(0)?.to_owned()))
         .collect();
-    let (columns, column_reads) = match masked.is_empty() {
-        true => (Vec::new(), None),
+    let (columns, column_reads, generated) = match masked.is_empty() {
+        true => (Vec::new(), None, Vec::new()),
         false => {
             let tables: Vec<String> = masked
                 .iter()
@@ -598,12 +598,16 @@ pub(super) async fn catalog(
                 .iter()
                 .map(|entry| entry.column.to_lowercase())
                 .collect();
-            let columns = client
+            let listed = client
                 .query(COLUMNS_SQL, &[&tables])
                 .await
-                .map_err(query_error)?
+                .map_err(query_error)?;
+            let column = |row: &tokio_postgres::Row| (row.get(0), row.get(1), row.get(2));
+            let columns = listed.iter().map(column).collect();
+            let generated = listed
                 .iter()
-                .map(|row| (row.get(0), row.get(1), row.get(2)))
+                .filter(|row| row.get::<_, bool>(3))
+                .map(column)
                 .collect();
             let reads: Vec<ColumnRead> = client
                 .query(COLUMN_READS_SQL, &[&names])
@@ -618,7 +622,7 @@ pub(super) async fn catalog(
                     column: row.get(4),
                 })
                 .collect();
-            (columns, Some(reads))
+            (columns, Some(reads), generated)
         }
     };
     Ok(Catalog {
@@ -628,6 +632,7 @@ pub(super) async fn catalog(
         columns,
         column_reads,
         definitions,
+        generated,
         ..Catalog::default()
     })
 }
diff --git a/crates/tabletist-db/src/sqlite.rs b/crates/tabletist-db/src/sqlite.rs
index 8e99616..fcc609d 100644
--- a/crates/tabletist-db/src/sqlite.rs
+++ b/crates/tabletist-db/src/sqlite.rs
@@ -1396,26 +1396,31 @@ fn agents_catalog(
     }
     // The columns of the masked columns' tables.
     let mut columns = Vec::new();
+    let mut generated = Vec::new();
     for relation in relations.iter().filter(|relation| {
         masked
             .iter()
             .any(|entry| entry.names_table(Some(&relation.schema), &relation.name))
     }) {
+        // With whether each is generated: `hidden` is 2 for a virtual
+        // generated column and 3 for a stored one.
         let mut statement = connection
-            .prepare("SELECT name FROM pragma_table_xinfo(?1, ?2)")
+            .prepare("SELECT name, hidden IN (2, 3) FROM pragma_table_xinfo(?1, ?2)")
             .map_err(map_error)?;
         let names = statement
             .query_map([&relation.name, &relation.schema], |row| {
-                row.get::<_, String>(0)
+                Ok((row.get::<_, String>(0)?, row.get::<_, bool>(1)?))
             })
             .map_err(map_error)?
             .collect::<rusqlite::Result<Vec<_>>>()
             .map_err(map_error)?;
-        columns.extend(
-            names
-                .into_iter()
-                .map(|column| (relation.schema.clone(), relation.name.clone(), column)),
-        );
+        for (column, is_generated) in names {
+            let named = (relation.schema.clone(), relation.name.clone(), column);
+            if is_generated {
+                generated.push(named.clone());
+            }
+            columns.push(named);
+        }
     }
     // The table functions, SQLite's own modules read without a table
     // made of them: the pragmas are refused before this, and `dbstat` and
@@ -1433,6 +1438,7 @@ fn agents_catalog(
         table_functions,
         columns,
         column_reads: Some(column_reads),
+        generated,
         ..Catalog::default()
     })
 }
````

- [ ] **Step 4: Run the tests**

Run: `cargo test --locked -p tabletist-db --lib generated_column`, then `cargo test --locked -p tabletist-db --test agent computed` with both test servers

Expected: PASS, the servers' tests run and none skipped:

```text
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 497 filtered out
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 34 filtered out
```

- [ ] **Step 5: Commit**

```bash
cargo fmt --all
cargo clippy --locked --workspace --all-targets -- -D warnings
git add crates/tabletist-db
git commit -m "Mask a generated column of a table with a masked column, which may be computed from it"
```

---

### Task 2: The server knows the masked columns, and list_connections names them

- `Listed::masked`: a connection's masked columns. `Session::hidden` and `Session::read` take them and give them to `Connection::agent_hidden` and `run_agent_read`. The app's gateway passes none until run 3; the tests' `Shop::mask` sets them.
- `list_connections` names them (`"masked": ["orders.email"]`), as the person writes them, where there are any, and its description says what a masked column is.

**Files:**
- Modify: `crates/tabletist-mcp/src/gateway.rs`, `crates/tabletist-mcp/src/tools/connections.rs`, `crates/tabletist-mcp/src/tools/query.rs`, `crates/tabletist-mcp/src/tools/schema.rs`, `crates/tabletist-mcp/tests/support/mod.rs`, `crates/tabletist-mcp/tests/tools.rs`, `src/backend/agents.rs`

- [ ] **Step 1: Write the failing test**

````diff
diff --git a/crates/tabletist-mcp/tests/support/mod.rs b/crates/tabletist-mcp/tests/support/mod.rs
index 44e7046..ec2fc4e 100644
--- a/crates/tabletist-mcp/tests/support/mod.rs
+++ b/crates/tabletist-mcp/tests/support/mod.rs
@@ -22,7 +22,7 @@ use rmcp::model::{
 use rmcp::service::RunningService;
 use rmcp::{RoleClient, ServiceExt};
 use serde_json::Value as Json;
-use tabletist_db::agent::{Answer, Hidden, Limits, Read, Tables};
+use tabletist_db::agent::{Answer, ColumnName, Hidden, Limits, Read, Tables};
 use tabletist_db::{
     Access, ConnectSpec, Connection, Dialect, Error, HostKeys, ObjectInfo, ObjectRef, Secrets,
     StopFlag, Structure,
@@ -94,6 +94,7 @@ impl Shop {
                 dialect: Dialect::Sqlite,
                 limits,
                 tables: Tables::All,
+                masked: Vec::new(),
             },
             path: path.to_owned(),
             turn: Arc::default(),
@@ -111,6 +112,19 @@ impl Shop {
         }
     }
 
+    /// Sets the columns of a listed connection whose values agents get as
+    /// `•••`, by their entries as the person writes them.
+    pub fn mask(&self, name: &str, masked: &[&str]) {
+        let masked: Vec<ColumnName> = masked.iter().map(|entry| entry.parse().unwrap()).collect();
+        let mut connections = self.inner.connections.lock().unwrap();
+        for kept in connections
+            .iter_mut()
+            .filter(|kept| kept.listed.name == name)
+        {
+            kept.listed.masked = masked.clone();
+        }
+    }
+
     /// Turns a connection's access Off.
     pub fn unlist(&self, name: &str) {
         self.inner
@@ -210,11 +224,17 @@ impl Session for ShopSession {
         Ok(self.connection.describe(object).await?)
     }
 
-    async fn hidden(&self, tables: &Tables) -> Result<Hidden, Failure> {
-        Ok(self.connection.agent_hidden(tables, &[]).await?)
+    async fn hidden(&self, tables: &Tables, masked: &[ColumnName]) -> Result<Hidden, Failure> {
+        Ok(self.connection.agent_hidden(tables, masked).await?)
     }
 
-    async fn read(&self, read: &Read, limits: Limits, tables: &Tables) -> Result<Answer, Failure> {
+    async fn read(
+        &self,
+        read: &Read,
+        limits: Limits,
+        tables: &Tables,
+        masked: &[ColumnName],
+    ) -> Result<Answer, Failure> {
         if self.stop.is_cancelled() {
             return Err(Error::Cancelled.into());
         }
@@ -228,7 +248,7 @@ impl Session for ShopSession {
         let flag = StopFlag::new();
         let run = self
             .connection
-            .run_agent_read(read, limits, tables, &[], &flag);
+            .run_agent_read(read, limits, tables, masked, &flag);
         tokio::pin!(run);
         let answer = tokio::select! {
             answer = &mut run => answer,
diff --git a/crates/tabletist-mcp/tests/tools.rs b/crates/tabletist-mcp/tests/tools.rs
index 3f6a460..cabfaf5 100644
--- a/crates/tabletist-mcp/tests/tools.rs
+++ b/crates/tabletist-mcp/tests/tools.rs
@@ -774,3 +774,19 @@ async fn a_hidden_table_that_is_gone_closes_the_connection_to_agents() {
     assert_eq!(called.answer()["rows"], json!([[12]]));
     agent.close().await;
 }
+
+#[tokio::test]
+async fn list_connections_names_what_is_masked() {
+    let shop = Shop::new();
+    let (server, _log) = server(&shop);
+    let agent = Agent::connect(&server).await;
+    let called = agent.call("list_connections", json!({})).await;
+    assert!(called.answer()["connections"][0].get("masked").is_none());
+    shop.mask(DEV, &["orders.email", "main.orders.phone"]);
+    let called = agent.call("list_connections", json!({})).await;
+    assert_eq!(
+        called.answer()["connections"][0]["masked"],
+        json!(["orders.email", "main.orders.phone"])
+    );
+    agent.close().await;
+}
diff --git a/src/backend/agents.rs b/src/backend/agents.rs
index 5de4b82..c661e15 100644
--- a/src/backend/agents.rs
+++ b/src/backend/agents.rs
@@ -976,6 +976,7 @@ mod tests {
                     timeout: Duration::from_secs(10),
                 },
                 &Tables::All,
+                &[],
             )
             .await
     }
@@ -1130,6 +1131,7 @@ mod tests {
                     timeout: Duration::from_secs(10),
                 },
                 &customers,
+                &[],
             )
             .await;
         assert!(
@@ -1141,7 +1143,7 @@ mod tests {
             ),
             "{refused:?}"
         );
-        let hidden = session.hidden(&customers).await.unwrap();
+        let hidden = session.hidden(&customers, &[]).await.unwrap();
         assert!(hidden.hides("main", "customers") && !hidden.hides("main", "books"));
     }
 
````

- [ ] **Step 2: Run it to see that it does not build**

Run: `cargo test --locked -p tabletist-mcp --test tools list_connections_names_what_is_masked`

Expected: it does not build:

```text
error[E0050]: method `hidden` has 3 parameters but the declaration in trait `hidden` has 2
error[E0050]: method `read` has 5 parameters but the declaration in trait `tabletist_mcp::Session::read` has 4
error[E0560]: struct `Listed` has no field named `masked`
error: could not compile `tabletist-mcp` (test "tools") due to 4 previous errors
```

- [ ] **Step 3: The masked columns through the gateway**

````diff
diff --git a/crates/tabletist-mcp/src/gateway.rs b/crates/tabletist-mcp/src/gateway.rs
index b97cfb2..3ead570 100644
--- a/crates/tabletist-mcp/src/gateway.rs
+++ b/crates/tabletist-mcp/src/gateway.rs
@@ -5,7 +5,7 @@
 
 use std::future::Future;
 
-use tabletist_db::agent::{Answer, Hidden, Limits, Read, Tables};
+use tabletist_db::agent::{Answer, ColumnName, Hidden, Limits, Read, Tables};
 use tabletist_db::{Dialect, ObjectInfo, ObjectRef, Structure};
 use tokio_util::sync::CancellationToken;
 
@@ -24,6 +24,9 @@ pub struct Listed {
     /// What agents may not read of it. The names are told to an agent
     /// (the MCP plan's question 8): the rows are what is hidden.
     pub tables: Tables,
+    /// The columns whose values agents get as `•••`, and may select but
+    /// not filter, join, sort or compute on (B.4). Their names are told.
+    pub masked: Vec<ColumnName>,
 }
 
 /// The schemas of a connection.
@@ -97,16 +100,22 @@ pub trait Session: Send + Sync {
         object: &ObjectRef,
     ) -> impl Future<Output = Result<Structure, Failure>> + Send;
 
-    /// What `tables` hides on the database as it is now
+    /// What `tables` hides and `masked` masks on the database as it is now
     /// ([`tabletist_db::Connection::agent_hidden`]).
-    fn hidden(&self, tables: &Tables) -> impl Future<Output = Result<Hidden, Failure>> + Send;
+    fn hidden(
+        &self,
+        tables: &Tables,
+        masked: &[ColumnName],
+    ) -> impl Future<Output = Result<Hidden, Failure>> + Send;
 
     /// Runs a statement that passed the check, reading nothing `tables`
-    /// hides ([`tabletist_db::Connection::run_agent_read`]).
+    /// hides and answering `masked`'s values as `•••`
+    /// ([`tabletist_db::Connection::run_agent_read`]).
     fn read(
         &self,
         read: &Read,
         limits: Limits,
         tables: &Tables,
+        masked: &[ColumnName],
     ) -> impl Future<Output = Result<Answer, Failure>> + Send;
 }
diff --git a/crates/tabletist-mcp/src/tools/connections.rs b/crates/tabletist-mcp/src/tools/connections.rs
index a67c8ff..58580f0 100644
--- a/crates/tabletist-mcp/src/tools/connections.rs
+++ b/crates/tabletist-mcp/src/tools/connections.rs
@@ -20,7 +20,11 @@ pub fn tool() -> Tool {
                       connection the person has not opened to agents is not listed. `hidden` \
                       names the tables you may not read, and `only` the tables you may read \
                       where no others are: a view over a hidden table is hidden with it, and \
-                      while anything is hidden so are the system catalogs.",
+                      while anything is hidden so are the system catalogs. `masked` names the \
+                      columns you may select but not filter, join, sort or compute on, and \
+                      neither may any column of the same name, in any table: each of their \
+                      values answers •••, and a view that reads one (on MySQL, any view over \
+                      its table) may not be read.",
         input: schema(json!({}), &[]),
     }
 }
@@ -72,6 +76,10 @@ fn entry(listed: &Listed) -> Json {
             entry.insert("only".into(), written(names));
         }
     }
+    if !listed.masked.is_empty() {
+        let masked = listed.masked.iter().map(|column| json!(column.to_string()));
+        entry.insert("masked".into(), Json::Array(masked.collect()));
+    }
     Json::Object(entry)
 }
 
diff --git a/crates/tabletist-mcp/src/tools/query.rs b/crates/tabletist-mcp/src/tools/query.rs
index 9afeaa4..3add8aa 100644
--- a/crates/tabletist-mcp/src/tools/query.rs
+++ b/crates/tabletist-mcp/src/tools/query.rs
@@ -86,7 +86,8 @@ async fn run<G: Gateway>(
     let session = settle(gateway.open(&listed.id, stop.token()).await, stop, &listed)?;
     note.ran_as = Some(RAN_AS);
     let limits = listed.limits;
-    let answer = within(stop, limits, session.read(&read, limits, &listed.tables)).await;
+    let reading = session.read(&read, limits, &listed.tables, &listed.masked);
+    let answer = within(stop, limits, reading).await;
     let answer = settle(answer, stop, &listed)?;
     let written = answer::write(&answer, MAX_ANSWER_BYTES).map_err(|_| {
         Refused::new(
diff --git a/crates/tabletist-mcp/src/tools/schema.rs b/crates/tabletist-mcp/src/tools/schema.rs
index ac83697..2ffac5c 100644
--- a/crates/tabletist-mcp/src/tools/schema.rs
+++ b/crates/tabletist-mcp/src/tools/schema.rs
@@ -101,7 +101,7 @@ async fn run<G: Gateway>(
     }
     let session = settle(gateway.open(&listed.id, stop.token()).await, stop, &listed)?;
     let work = async {
-        let hidden = session.hidden(&listed.tables).await?;
+        let hidden = session.hidden(&listed.tables, &listed.masked).await?;
         hidden
             .check_settings()
             .map_err(|refusal| Failure::Database(Error::Agent(refusal)))?;
diff --git a/src/backend/agents.rs b/src/backend/agents.rs
index c661e15..6d89bae 100644
--- a/src/backend/agents.rs
+++ b/src/backend/agents.rs
@@ -12,7 +12,7 @@ use std::sync::atomic::AtomicU32;
 use std::sync::{Arc, Mutex};
 use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
 
-use tabletist_db::agent::{Answer, Hidden, Limits, Read, Tables};
+use tabletist_db::agent::{Answer, ColumnName, Hidden, Limits, Read, Tables};
 use tabletist_db::{
     Access, CancelHandle, ConnectSpec, Connection, Error, HostKeys, ObjectInfo, ObjectRef, Secrets,
     StopFlag, Structure,
@@ -658,6 +658,8 @@ impl Gateway for AppGateway {
                 dialect: agentable.spec.driver.dialect(),
                 limits: agentable.limits,
                 tables: agentable.tables.clone(),
+                // Masked columns are read from the setting in the next run.
+                masked: Vec::new(),
             })
             .collect()
     }
@@ -904,17 +906,24 @@ impl Session for AppSession {
         self.ended(None, connection.describe(object)).await
     }
 
-    async fn hidden(&self, tables: &Tables) -> Result<Hidden, Failure> {
+    async fn hidden(&self, tables: &Tables, masked: &[ColumnName]) -> Result<Hidden, Failure> {
         let connection = self.connection()?;
-        self.ended(None, connection.agent_hidden(tables, &[])).await
+        self.ended(None, connection.agent_hidden(tables, masked))
+            .await
     }
 
-    async fn read(&self, read: &Read, limits: Limits, tables: &Tables) -> Result<Answer, Failure> {
+    async fn read(
+        &self,
+        read: &Read,
+        limits: Limits,
+        tables: &Tables,
+        masked: &[ColumnName],
+    ) -> Result<Answer, Failure> {
         let connection = self.connection()?;
         let flag = StopFlag::new();
         self.ended(
             Some(&flag),
-            connection.run_agent_read(read, limits, tables, &[], &flag),
+            connection.run_agent_read(read, limits, tables, masked, &flag),
         )
         .await
     }
````

- [ ] **Step 4: Run the tests**

Run: `cargo test --locked -p tabletist-mcp --test tools list_connections`

Expected: PASS.

```text
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 22 filtered out
```

- [ ] **Step 5: Commit**

```bash
cargo fmt --all
cargo clippy --locked --workspace --all-targets -- -D warnings
git add crates/tabletist-mcp src/backend/agents.rs
git commit -m "Give the server each connection's masked columns, and name them in list_connections"
```

---

### Task 3: describe_schema marks a masked column and a view that reads one

- A masked column is marked `"masked": true`; its default, the values it may hold, its comment and the values MySQL writes into an `enum` or a `set` type are left out.
- `Hidden::reads_masked`: the masked column a view reads, where it reads one. Such a view is marked `"reads_masked": "orders.email"` in the list and in its description.
- The description says so, and that a column of a masked column's name is refused in any table.

**Files:**
- Modify: `crates/tabletist-db/src/agent/hidden.rs`, `crates/tabletist-mcp/src/tools/schema.rs`, `crates/tabletist-mcp/tests/tools.rs`

- [ ] **Step 1: Write the failing test**

````diff
diff --git a/crates/tabletist-db/src/agent/hidden.rs b/crates/tabletist-db/src/agent/hidden.rs
index f07ddd0..1e0e087 100644
--- a/crates/tabletist-db/src/agent/hidden.rs
+++ b/crates/tabletist-db/src/agent/hidden.rs
@@ -2089,6 +2089,12 @@ mod tests {
             through("recent_orders")
         );
         assert!(!hidden.hides("public", "recent_orders"));
+        assert_eq!(
+            hidden.reads_masked("public", "recent_orders"),
+            Some("orders.email".into())
+        );
+        assert_eq!(hidden.reads_masked("public", "order_ids"), None);
+        assert_eq!(hidden.reads_masked("public", "orders"), None);
         // Where the catalog does not say what a view reads column by
         // column, a view over the table reads it.
         let unsaid = masking(&["orders.email"], None);
diff --git a/crates/tabletist-mcp/src/tools/schema.rs b/crates/tabletist-mcp/src/tools/schema.rs
index 2ffac5c..b3d75dd 100644
--- a/crates/tabletist-mcp/src/tools/schema.rs
+++ b/crates/tabletist-mcp/src/tools/schema.rs
@@ -427,6 +427,15 @@ fn table(
 mod tests {
     use super::*;
 
+    #[test]
+    fn a_masked_columns_type_is_told_without_the_values_mysql_writes_into_it() {
+        assert_eq!(type_of("enum('new','paid','shipped')", true), "enum");
+        assert_eq!(type_of("SET('a','b')", true), "SET");
+        assert_eq!(type_of("enum('new','paid')", false), "enum('new','paid')");
+        assert_eq!(type_of("varchar(255)", true), "varchar(255)");
+        assert_eq!(type_of("text", true), "text");
+    }
+
     #[test]
     fn a_name_is_looked_for_at_home_first_and_then_by_its_schema() {
         let schemas = ["public".to_owned(), "sales".to_owned()];
diff --git a/crates/tabletist-mcp/tests/tools.rs b/crates/tabletist-mcp/tests/tools.rs
index cabfaf5..756be50 100644
--- a/crates/tabletist-mcp/tests/tools.rs
+++ b/crates/tabletist-mcp/tests/tools.rs
@@ -790,3 +790,98 @@ async fn list_connections_names_what_is_masked() {
     );
     agent.close().await;
 }
+
+#[tokio::test]
+async fn describe_schema_marks_a_masked_column_and_a_view_that_reads_one() {
+    let shop = Shop::new();
+    let (server, _log) = server(&shop);
+    let agent = Agent::connect(&server).await;
+    shop.mask(DEV, &["orders.email"]);
+    // In the list, the view that reads it is marked, and the one that
+    // does not is not.
+    let called = agent
+        .call("describe_schema", json!({ "connection": DEV }))
+        .await;
+    let tables = called.answer()["tables"].as_array().unwrap().clone();
+    let named = |name: &str| {
+        tables
+            .iter()
+            .find(|table| table["name"] == json!(name))
+            .unwrap_or_else(|| panic!("{name}"))
+            .clone()
+    };
+    assert_eq!(
+        named("recent_orders")["reads_masked"],
+        json!("orders.email")
+    );
+    assert!(named("customer_names").get("reads_masked").is_none());
+    assert!(named("orders").get("reads_masked").is_none());
+    // Described, the column is marked, and so is the view.
+    let called = agent
+        .call(
+            "describe_schema",
+            json!({ "connection": DEV, "tables": ["orders", "recent_orders"] }),
+        )
+        .await;
+    let described = called.answer()["tables"].as_array().unwrap().clone();
+    let columns = described[0]["columns"].as_array().unwrap();
+    let column = |name: &str| {
+        columns
+            .iter()
+            .find(|column| column["name"] == json!(name))
+            .unwrap()
+            .clone()
+    };
+    assert_eq!(column("email")["masked"], json!(true));
+    assert!(column("id").get("masked").is_none());
+    assert_eq!(described[1]["reads_masked"], json!("orders.email"));
+    agent.close().await;
+}
+
+#[tokio::test]
+async fn a_masked_columns_default_values_and_what_is_computed_from_it_are_not_told() {
+    let shop = Shop::new();
+    shop.change(
+        "CREATE TABLE gift_notes (
+             id INTEGER PRIMARY KEY,
+             kind TEXT DEFAULT 'gift' CHECK (kind IN ('gift', 'self')),
+             shout TEXT GENERATED ALWAYS AS (upper(kind)) VIRTUAL
+         );
+         INSERT INTO gift_notes (id, kind) VALUES (1, 'self');",
+    );
+    let (server, _log) = server(&shop);
+    let agent = Agent::connect(&server).await;
+    shop.mask(DEV, &["gift_notes.kind"]);
+    let called = agent
+        .call(
+            "describe_schema",
+            json!({ "connection": DEV, "tables": ["gift_notes"] }),
+        )
+        .await;
+    let columns = called.answer()["tables"][0]["columns"]
+        .as_array()
+        .unwrap()
+        .clone();
+    let column = |name: &str| {
+        columns
+            .iter()
+            .find(|column| column["name"] == json!(name))
+            .unwrap()
+            .clone()
+    };
+    // Its default and the values it may hold say what its values are.
+    let kind = column("kind");
+    assert_eq!(kind["masked"], json!(true), "{kind}");
+    assert!(kind.get("default").is_none(), "{kind}");
+    assert!(kind.get("values").is_none(), "{kind}");
+    // A column computed from it is masked with it, and its expression is
+    // not told.
+    let shout = column("shout");
+    assert_eq!(shout["masked"], json!(true), "{shout}");
+    assert!(shout.get("default").is_none(), "{shout}");
+    let called = query(&agent, "SELECT id, shout FROM gift_notes").await;
+    assert_eq!(called.answer()["rows"], json!([[1, "•••"]]));
+    let called = query(&agent, "SELECT id FROM gift_notes WHERE shout = 'SELF'").await;
+    assert_eq!(called.refused(), "masked_column_used");
+    agent.close().await;
+}
````

- [ ] **Step 2: Run it to see it fail**

Run: `cargo test --locked -p tabletist-mcp --test tools -- describe_schema_marks a_masked_columns_default`

Expected: FAIL. Nothing is marked yet:

```text
test describe_schema_marks_a_masked_column_and_a_view_that_reads_one ... FAILED
test a_masked_columns_default_values_and_what_is_computed_from_it_are_not_told ... FAILED
  left: Null
 right: String("orders.email")
```

- [ ] **Step 3: The marks**

````diff
diff --git a/crates/tabletist-db/src/agent/hidden.rs b/crates/tabletist-db/src/agent/hidden.rs
index 1e0e087..0d5f889 100644
--- a/crates/tabletist-db/src/agent/hidden.rs
+++ b/crates/tabletist-db/src/agent/hidden.rs
@@ -469,6 +469,18 @@ impl Hidden {
         !self.masked.is_empty()
     }
 
+    /// The masked column the view `name` of `schema` reads, as the person
+    /// writes its entry, where it is a view that reads one: a statement
+    /// that reads it is refused, though the view is not hidden.
+    pub fn reads_masked(&self, schema: &str, name: &str) -> Option<String> {
+        self.relations
+            .get(&fold(name))?
+            .iter()
+            .find(|listed| same(&listed.schema, schema))?
+            .masks
+            .clone()
+    }
+
     /// A masked column of the relation `table` of `schema`, as the person
     /// writes its entry, where it has one: for a whole row of it.
     pub fn masks_table(&self, schema: Option<&str>, table: &str) -> Option<String> {
diff --git a/crates/tabletist-mcp/src/tools/schema.rs b/crates/tabletist-mcp/src/tools/schema.rs
index b3d75dd..88dd675 100644
--- a/crates/tabletist-mcp/src/tools/schema.rs
+++ b/crates/tabletist-mcp/src/tools/schema.rs
@@ -32,7 +32,12 @@ pub fn tool() -> Tool {
                       types, its primary key, its foreign keys and the keys that point at it, \
                       and its indexes. Write a table of another schema as schema.table. A \
                       table the person hides from you is left out of the list, and named under \
-                      `hidden` when you ask for it.",
+                      `hidden` when you ask for it. A column marked `masked` may be selected, \
+                      and answers •••, but not filtered, joined, sorted or computed on, and \
+                      neither may any column of its name, in any table; its default, its \
+                      comment and the values it may hold are left out. A view marked \
+                      `reads_masked` may not be read: it reads such a column, or on MySQL \
+                      reads its table.",
         input: schema(
             json!({
                 "connection": {
@@ -167,6 +172,11 @@ async fn list<S: Session>(session: &S, hidden: &Hidden) -> Result<Described, Fai
         if let Some(rows) = object.estimated_rows {
             entry.insert("rows_estimate".into(), json!(rows));
         }
+        if let Some(home) = &home
+            && let Some(column) = hidden.reads_masked(home, &object.name)
+        {
+            entry.insert("reads_masked".into(), json!(column));
+        }
         let entry = Json::Object(entry);
         if !room.take(&entry) {
             truncated = true;
@@ -313,6 +323,18 @@ impl Room {
     }
 }
 
+/// A column's type as it is told: a masked one's without the values
+/// MySQL writes into it (`enum('new','paid')` is `enum`).
+fn type_of(type_name: &str, masked: bool) -> &str {
+    let listed = ["enum(", "set("].iter().any(|kind| {
+        type_name.len() >= kind.len() && type_name[..kind.len()].eq_ignore_ascii_case(kind)
+    });
+    match (masked, listed) {
+        (true, true) => type_name.split('(').next().unwrap_or(type_name),
+        _ => type_name,
+    }
+}
+
 fn kind(kind: ObjectKind) -> &'static str {
     match kind {
         ObjectKind::Table => "table",
@@ -336,26 +358,41 @@ fn table(
         entry.insert("schema".into(), json!(object.schema));
     }
     entry.insert("kind".into(), json!(kind(object_kind)));
+    if let Some(column) = hidden.reads_masked(&object.schema, &object.name) {
+        entry.insert("reads_masked".into(), json!(column));
+    }
     let columns: Vec<Json> = structure
         .columns
         .iter()
         .map(|column| {
+            let masked = hidden
+                .masks(Some(&object.schema), &object.name, &column.name)
+                .is_some();
             let mut entry = Map::new();
             entry.insert("name".into(), json!(column.name));
-            entry.insert("type".into(), json!(column.type_name));
+            entry.insert("type".into(), json!(type_of(&column.type_name, masked)));
             entry.insert("nullable".into(), json!(column.nullable));
-            // A default and a comment are as long as their author liked.
-            if let Some(default) = &column.default {
-                entry.insert("default".into(), answer::text(default));
-            }
-            if let Some(values) = &column.allowed_values {
-                entry.insert("values".into(), json!(values));
+            if masked {
+                entry.insert("masked".into(), json!(true));
             }
             if column.generated {
                 entry.insert("generated".into(), json!(true));
             }
-            if let Some(comment) = &column.comment {
-                entry.insert("comment".into(), answer::text(comment));
+            // What says what a masked column's values are is left out: its
+            // default, which is one, or what it is computed from; the
+            // values it may hold; and its comment, whose words are its
+            // author's and may give one as an example. A default and a
+            // comment are as long as their author liked.
+            if !masked {
+                if let Some(default) = &column.default {
+                    entry.insert("default".into(), answer::text(default));
+                }
+                if let Some(values) = &column.allowed_values {
+                    entry.insert("values".into(), json!(values));
+                }
+                if let Some(comment) = &column.comment {
+                    entry.insert("comment".into(), answer::text(comment));
+                }
             }
             Json::Object(entry)
         })
````

- [ ] **Step 4: Run the tests**

Run: `cargo test --locked -p tabletist-mcp --test tools -- describe_schema_marks a_masked_columns_default`, then `cargo test --locked -p tabletist-db --lib a_view_that_reads_a_masked_column`

Expected: PASS.

```text
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 25 filtered out
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 498 filtered out
```

- [ ] **Step 5: Commit**

```bash
cargo fmt --all
cargo clippy --locked --workspace --all-targets -- -D warnings
git add crates/tabletist-db/src/agent/hidden.rs crates/tabletist-mcp
git commit -m "Mark a masked column and a view that reads one in describe_schema, and tell nothing of the column's values"
```

---

### Task 4: run_query marks a masked column, and the log keeps which

- An answer's column that is a masked column's is marked `"masked": true`; its values are `•••` from the database layer.
- The call's log entry fills the log's `masked` field with the answer's masked columns, by their names in the answer, as a JSON list, and leaves it empty where none was masked.
- `run_query`'s description says what a masked column answers.

**Files:**
- Modify: `crates/tabletist-mcp/src/answer.rs`, `crates/tabletist-mcp/src/tools/query.rs`, `crates/tabletist-mcp/src/tools.rs`, `crates/tabletist-mcp/src/server.rs`, `crates/tabletist-mcp/src/log.rs`, `crates/tabletist-mcp/tests/tools.rs`

- [ ] **Step 1: Write the failing test**

````diff
diff --git a/crates/tabletist-mcp/src/log.rs b/crates/tabletist-mcp/src/log.rs
index 1c3665d..3f17153 100644
--- a/crates/tabletist-mcp/src/log.rs
+++ b/crates/tabletist-mcp/src/log.rs
@@ -644,6 +644,7 @@ mod tests {
             matched: Some(3),
             columns: Some(1),
             bytes: Some(96),
+            masked: Some(r#"["email"]"#.into()),
             took_ms: Some(4),
         };
         log.record(whole.clone()).await.unwrap();
diff --git a/crates/tabletist-mcp/tests/tools.rs b/crates/tabletist-mcp/tests/tools.rs
index 756be50..adb3b1e 100644
--- a/crates/tabletist-mcp/tests/tools.rs
+++ b/crates/tabletist-mcp/tests/tools.rs
@@ -838,6 +838,44 @@ async fn describe_schema_marks_a_masked_column_and_a_view_that_reads_one() {
     agent.close().await;
 }
 
+#[tokio::test]
+async fn run_query_answers_a_masked_column_as_dots_and_refuses_a_use_of_it() {
+    let shop = Shop::new();
+    let (server, log) = server(&shop);
+    let agent = Agent::connect(&server).await;
+    shop.mask(DEV, &["orders.email", "orders.phone"]);
+    // Its name and its type, `•••` in every row, a NULL too, and marked.
+    let called = query(&agent, "SELECT id, email, phone FROM orders WHERE id = 1").await;
+    let answer = called.answer();
+    assert_eq!(answer["columns"][0].get("masked"), None);
+    assert_eq!(answer["columns"][1]["name"], json!("email"));
+    assert_eq!(answer["columns"][1]["masked"], json!(true));
+    assert_eq!(answer["columns"][2]["masked"], json!(true));
+    assert_eq!(answer["rows"], json!([[1, "•••", "•••"]]));
+    // The log keeps which columns were masked, and no value.
+    let logged = log.latest(1).await.unwrap();
+    assert_eq!(logged[0].masked.as_deref(), Some(r#"["email","phone"]"#));
+    // A use of it is refused, and so is a view that reads it.
+    let called = query(&agent, "SELECT id FROM orders WHERE email LIKE 'a%'").await;
+    assert_eq!(called.refused(), "masked_column_used");
+    assert!(
+        called.text.starts_with("orders.email is masked"),
+        "{}",
+        called.text
+    );
+    let called = query(&agent, "SELECT * FROM recent_orders").await;
+    assert_eq!(called.refused(), "masked_column_used");
+    assert_eq!(
+        called.text,
+        "recent_orders reads orders.email, which is masked on this connection."
+    );
+    // An answer with no masked column logs none.
+    query(&agent, "SELECT id FROM orders WHERE id = 1").await;
+    let logged = log.latest(1).await.unwrap();
+    assert_eq!(logged[0].masked, None);
+    agent.close().await;
+}
+
 #[tokio::test]
 async fn a_masked_columns_default_values_and_what_is_computed_from_it_are_not_told() {
     let shop = Shop::new();
````

- [ ] **Step 2: Run it to see that it does not build**

Run: `cargo test --locked -p tabletist-mcp --test tools run_query_answers_a_masked_column`

Expected: it does not build:

```text
error[E0609]: no field `masked` on type `tabletist_mcp::Entry`
error: could not compile `tabletist-mcp` (test "tools") due to 2 previous errors
```

- [ ] **Step 3: The mark, and the log**

````diff
diff --git a/crates/tabletist-mcp/src/answer.rs b/crates/tabletist-mcp/src/answer.rs
index 345dbc4..d6795f7 100644
--- a/crates/tabletist-mcp/src/answer.rs
+++ b/crates/tabletist-mcp/src/answer.rs
@@ -55,12 +55,17 @@ pub fn write(answer: &Answer, max_bytes: usize) -> Result<Written, TooWide> {
     let columns: Vec<Json> = answer
         .columns
         .iter()
-        .map(|column| {
+        .enumerate()
+        .map(|(at, column)| {
             let mut entry = Map::new();
             entry.insert("name".into(), json!(column.name));
             if !column.type_name.is_empty() {
                 entry.insert("type".into(), json!(column.type_name));
             }
+            // Its every value is `•••`.
+            if answer.masked.get(at) == Some(&true) {
+                entry.insert("masked".into(), json!(true));
+            }
             Json::Object(entry)
         })
         .collect();
diff --git a/crates/tabletist-mcp/src/log.rs b/crates/tabletist-mcp/src/log.rs
index 3f17153..7a2bacf 100644
--- a/crates/tabletist-mcp/src/log.rs
+++ b/crates/tabletist-mcp/src/log.rs
@@ -92,6 +92,9 @@ pub struct Entry {
     pub columns: Option<i64>,
     /// The answer's size as JSON.
     pub bytes: Option<i64>,
+    /// The answer's masked columns, by name, as a JSON list: the call
+    /// answered `•••` for each of their values.
+    pub masked: Option<String>,
     pub took_ms: Option<i64>,
 }
 
@@ -250,9 +253,9 @@ impl Log {
             connection.execute(
                 "INSERT INTO calls (at_ms, session, agent, project, transport, connection_id, \
                  connection, tool, sql, params, request, result, code, message, ran_as, rows, \
-                 matched, columns, bytes, took_ms) \
+                 matched, columns, bytes, masked, took_ms) \
                  VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, \
-                 ?16, ?17, ?18, ?19, ?20)",
+                 ?16, ?17, ?18, ?19, ?20, ?21)",
                 params![
                     entry.at_ms,
                     entry.session,
@@ -273,6 +276,7 @@ impl Log {
                     entry.matched,
                     entry.columns,
                     entry.bytes,
+                    entry.masked,
                     entry.took_ms,
                 ],
             )?;
@@ -398,7 +402,7 @@ impl Log {
 /// The columns of an entry, in [`entry_at`]'s order.
 const COLUMNS: &str = "at_ms, session, agent, project, transport, connection_id, connection, \
     tool, sql, params, request, result, code, message, ran_as, rows, matched, columns, bytes, \
-    took_ms";
+    masked, took_ms";
 
 /// A page of one connection's calls, through its index.
 fn page_of_one() -> String {
@@ -438,7 +442,8 @@ fn entry_at(row: &Row<'_>, at: usize) -> Result<Entry, rusqlite::Error> {
         matched: row.get(at + 16)?,
         columns: row.get(at + 17)?,
         bytes: row.get(at + 18)?,
-        took_ms: row.get(at + 19)?,
+        masked: row.get(at + 19)?,
+        took_ms: row.get(at + 20)?,
     })
 }
 
diff --git a/crates/tabletist-mcp/src/server.rs b/crates/tabletist-mcp/src/server.rs
index 39c3f74..d0e5de1 100644
--- a/crates/tabletist-mcp/src/server.rs
+++ b/crates/tabletist-mcp/src/server.rs
@@ -499,6 +499,7 @@ impl<G: Gateway> Server<G> {
             matched: note.matched.map(count),
             columns: note.columns.map(count),
             bytes: bytes.map(|bytes| count(bytes as u64)),
+            masked: note.masked,
             took_ms: Some(millis(took)),
         };
         if let Err(error) = self.inner.log.record(entry).await {
diff --git a/crates/tabletist-mcp/src/tools.rs b/crates/tabletist-mcp/src/tools.rs
index 2bfb42f..9371ec3 100644
--- a/crates/tabletist-mcp/src/tools.rs
+++ b/crates/tabletist-mcp/src/tools.rs
@@ -69,6 +69,8 @@ pub(crate) struct Note {
     /// The statement's rows, where the answer holds them all.
     pub matched: Option<u64>,
     pub columns: Option<u64>,
+    /// The answer's masked columns, by name, as a JSON list.
+    pub masked: Option<String>,
     pub took: Option<Duration>,
 }
 
diff --git a/crates/tabletist-mcp/src/tools/query.rs b/crates/tabletist-mcp/src/tools/query.rs
index 3add8aa..b6e718a 100644
--- a/crates/tabletist-mcp/src/tools/query.rs
+++ b/crates/tabletist-mcp/src/tools/query.rs
@@ -24,7 +24,8 @@ pub fn tool() -> Tool {
                       `truncated_by` whether by the row limit or by the answer's size. A text \
                       longer than 2,000 characters comes as {\"cut\": its start, \
                       \"characters\": its length}, and a binary value as {\"binary_bytes\": \
-                      its size}.",
+                      its size}. A column marked `masked` answers ••• in every row: it may be \
+                      selected as it is, and not filtered, joined, sorted or computed on.",
         input: schema(
             json!({
                 "connection": {
@@ -100,6 +101,14 @@ async fn run<G: Gateway>(
     note.columns = Some(written.columns as u64);
     note.matched = written.cut.is_none().then_some(written.rows as u64);
     note.took = Some(answer.elapsed);
+    let masked: Vec<&str> = answer
+        .columns
+        .iter()
+        .zip(&answer.masked)
+        .filter(|(_, masked)| **masked)
+        .map(|(column, _)| column.name.as_str())
+        .collect();
+    note.masked = (!masked.is_empty()).then(|| json!(masked).to_string());
     Ok(written.json)
 }
 
````

- [ ] **Step 4: Run the tests**

Run: `cargo test --locked -p tabletist-mcp --test tools run_query_answers_a_masked_column`, then `cargo test --locked -p tabletist-mcp --lib log`

Expected: PASS.

```text
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 27 filtered out
test result: ok. 16 passed; 0 failed; 0 ignored; 0 measured; 37 filtered out
```

- [ ] **Step 5: Commit**

```bash
cargo fmt --all
cargo clippy --locked --workspace --all-targets -- -D warnings
git add crates/tabletist-mcp
git commit -m "Mark a masked column of run_query's answer, and keep in the log which were masked"
```

---

### Task 5: The master plan

- B.4 as built for 3a-2, with the generated columns and what `describe_schema` leaves out; G's row and 3a's run 2: built.

**Files:**
- Modify: `docs/superpowers/plans/2026-10-13-mcp-server.md`

- [ ] **Step 1: The edits**

````diff
diff --git a/docs/superpowers/plans/2026-10-13-mcp-server.md b/docs/superpowers/plans/2026-10-13-mcp-server.md
index 1bf4292..b026fd3 100644
--- a/docs/superpowers/plans/2026-10-13-mcp-server.md
+++ b/docs/superpowers/plans/2026-10-13-mcp-server.md
@@ -1157,6 +1157,32 @@ otherwise than above:
   those that name a relation, an alias or an index: a table named like
   its masked column (`city.city`) is no use of the column.
 
+**As built in 3a's run 2** (`2026-10-17-mcp-server-3a-2-server.md`): the
+server's side. A connection's masked columns come with it from the
+gateway (`Listed::masked`; the app passes none until run 3 reads the
+setting), and go to the database layer with every check and read.
+`list_connections` names them (`"masked": ["orders.email"]`), as it names
+the hidden tables. `describe_schema` marks a masked column
+`"masked": true` and leaves out what says what its values are: its
+default, the values it may hold, the list MySQL writes into an `enum` or
+a `set` type, and its comment, whose author may give one as an example
+(found by the review). A view that reads one stays listed, marked
+`"reads_masked": "orders.email"`, in the list and in its description
+(agreed on 2026-10-17: the agent knows before it asks, as for a hidden
+table it is told the name). `run_query` marks a masked column of its
+answer `"masked": true`, and the log keeps the answer's masked columns,
+by their names in the answer, in its `masked` field, a JSON list, which
+C.1 gave it from the start.
+
+- **A generated column of a table with a masked column** is masked with
+  it, by an entry of its own (`public.orders.low`): one computed from it
+  (`lower(email)`) answered its value in full, and could be filtered on
+  (found by the review, on PostgreSQL and SQLite). Each driver says which
+  columns of the masked columns' tables are generated: PostgreSQL's
+  `attgenerated`, MySQL's `EXTRA`, SQLite's `pragma_table_xinfo`. Whether
+  the column is computed from the masked one is not asked: it errs toward
+  masking.
+
 **The rule.** It applies to any statement that reads a table with a masked
 column.
 
@@ -2593,7 +2619,7 @@ is off until the person turns it on.
 | 1b on Windows: the way in | 1 | Built (`2026-10-15-mcp-server-1b-6-windows.md`) |
 | 1c. Hidden tables | 6, below | Built (`2026-10-15-mcp-server-1c-1-checks.md`, `2026-10-15-mcp-server-1c-2a-server.md`, `2026-10-16-mcp-server-1c-2b-setting.md`, `2026-10-16-mcp-server-1c-3a-tables-control.md`, `2026-10-16-mcp-server-1c-3b-table-list.md`) |
 | 1d. HTTP with tokens | 1 | Dropped for now (2026-10-15) |
-| 3a. Masked columns | 5, below | Runs 1a and 1b built (`2026-10-17-mcp-server-3a-1a-masked-rule.md`, `2026-10-17-mcp-server-3a-1b-masked-values.md`): the rule, its stops, the engines' fences and the values. Run 2, the server's side, is next (order changed on 2026-10-17) |
+| 3a. Masked columns | 5, below | Runs 1a, 1b and 2 built (`2026-10-17-mcp-server-3a-1a-masked-rule.md`, `2026-10-17-mcp-server-3a-1b-masked-values.md`, `2026-10-17-mcp-server-3a-2-server.md`): the rule, its stops, the engines' fences, the values and the server's side. Run 3, the setting in `connections.toml`, is next (order changed on 2026-10-17) |
 | 3b. The Log view, in full | Not cut yet | After 3a |
 | 2. Approved writes | Not cut yet | Last, after 3b (moved on 2026-10-17) |
 
@@ -2788,7 +2814,8 @@ pull request:
    engine's fence and the values replaced by the result columns' origins
    (`2026-10-17-mcp-server-3a-1b-masked-values.md`, built).
 2. The server's side: the setting read, `list_connections`' masked
-   columns, `describe_schema`'s mark, and `masked: true` in an answer.
+   columns, `describe_schema`'s mark, and `masked: true` in an answer
+   (`2026-10-17-mcp-server-3a-2-server.md`, built).
 3. The setting in `connections.toml`, as 1c-2b's for tables.
 4. The editor's Masked columns control with **Add column…** and its mark,
    and the website's guide.
````

- [ ] **Step 2: Check the whole of it**

Run: `cargo fmt --all --check`, clippy as above, `RUSTDOCFLAGS='-D warnings' cargo doc --locked --workspace --no-deps`, the count of em dashes in the master plan, and `cargo test --locked --workspace --all-targets` with both servers

Expected: fmt and clippy clean, the documentation builds without a warning, no em dash (`0`), and:

```text
3564 passed, 0 failed, 22 ignored
```

- [ ] **Step 3: Commit**

```bash
git add docs/superpowers/plans/2026-10-13-mcp-server.md
git commit -m "Say in the MCP plan that the server's side of masked columns is built"
```

## Self-review

- **Against the design.** G's run 2 (`list_connections`' masked columns, `describe_schema`'s mark, `masked: true` in an answer), B.4's marking of the answer and C.1's `masked` field, with the departures in "What the plan asks, and what is built", agreed on 2026-10-17 or found by the review.
- **A review before the plan.** A reviewer ran the draft through the server on SQLite, PostgreSQL and MySQL. It found a generated column computed from a masked column answering its value and taking a filter, on PostgreSQL and SQLite; a masked MySQL `enum` or `set` column's type listing its values; a masked column's default and comment told; no test that its values are left out; and the descriptions calling a view over a masked table on MySQL one that reads the column, without saying that a column of the same name is refused in any table. All fixed, with a test each, on all three where it is the database's. It also noted that the log names the answer's columns, not the entries, which is C.1's, and kept.
- **Placeholders.** None: every step has its diff or its command and what it prints.
- **Names.** `Listed::masked`, `Hidden::reads_masked`, `Catalog::generated`, `Shop::mask`, `Note::masked` and `Entry::masked` are the same in every task that uses them.
