# MCP server, Phase 3a, run 1a: a masked column's rule, read off the tokens. Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A connection's masked columns (`orders.email`) can be selected as they are and used no other way: `crates/tabletist-db` refuses an agent's statement that filters, joins, sorts, groups or computes on one (`masked_column_used`), reads one through a view, or reads a whole row of its table, and stops agents on a connection whose masked column was renamed or dropped. A connection that masks a column closes the system catalogs and the users' functions as one that hides a table does. Nothing is visible: the app and the server pass no masked column until run 2, and the values are replaced in run 1b.

**Architecture:** `agent::ColumnName` is an entry as the person writes it. `agent/hidden/masked.rs` reads a statement's tokens, as 1c's reading of the tables' names does (B.2: the tree is not walked), and goes by a masked column's name alone: a word that is one may stand only bare in the outermost `SELECT`'s list, whatever table it is on. `Hidden::masking` keeps the entries with what the catalog says of them: the partitions and the child tables of their tables, the views that read one (column by column where the catalog says so, PostgreSQL's `pg_depend` and SQLite's authorizer, table by table on MySQL), and the entries that name no column. Each driver reads that much more of its catalog where something is masked, and SQLite's authorizer refuses a masked column a view reads.

**Tech Stack:** Rust 2024, tokio-postgres, mysql_async, rusqlite with SQLite's authorizer, the app's own SQL tokenizer.

---

## Where this sits

Phase 3a of `docs/superpowers/plans/2026-10-13-mcp-server.md`, masked columns, which comes before Phase 2 since 2026-10-17. 3a is four runs, and the first is in two: this one, 1a, the rule; then 1b, each engine's fence and the values replaced by the result columns' origins. Phase 1 (pull requests 122 to 149) built the server, its interface and the hidden tables, whose checks this run extends.

## What the plan asks, and what is built

| B.4 | Built | Why |
|---|---|---|
| Rule 5: "every name in the statement must resolve to a column that is not masked … a name the checker cannot place is refused", a resolver over the tree | A masked column's name refused wherever it stands but bare in the outermost list, whatever table it is on | Settled on 2026-10-17: the tree is not walked (B.2), and a reading by name errs toward refusing. `customers.email` is refused beside a masked `orders.email`, and the sentence says so |
| Rules 1 to 4 | Read off the tokens: the outermost list, its `DISTINCT`, `GROUP BY`, `HAVING`, windows and set operations, `ORDER BY` a place or an alias, `*` inside the statement, `NATURAL`, the table or its alias as a value, and a column list after its alias | As B.4 lists them, and `AS t(a, b, c)` from F.2 |
| Rule 6: views | Refused where they read a masked column: by `pg_depend` and the definition PostgreSQL writes back (a whole row), by SQLite's authorizer, and on MySQL any view over the table | `pg_depend` keeps no column for a whole row a view reads; `pg_get_viewdef` writes it as `x.*` |
| Rule 7: writes | Not built | Phase 2's, which comes last |
| C.3: "followed by attribute number on PostgreSQL" | A stop on all three | Settled on 2026-10-17, as for a hidden table since 1c-2a |

## Before you start

- Work on a branch of `main`. One commit for each task, in the order below. Every commit passes `cargo fmt --all --check`, `cargo clippy --locked --workspace --all-targets -- -D warnings` and `cargo test --locked --workspace --all-targets`.
- On Igor's machine `cargo` is `~/.cargo/bin/cargo` (the mise shim fails), and a build in a worktree needs `CARGO_TARGET_DIR` set to that worktree's `target/`.
- The server tests of `crates/tabletist-db/tests/agent.rs` run against PostgreSQL and MySQL where `TABLETIST_TEST_PG_URL` and `TABLETIST_TEST_MYSQL_URL` are set (`docker compose up -d --wait postgres mysql`), and print "skipped" otherwise.
- Read `AGENTS.md`. The diffs below apply with `git apply`. Run `cargo fmt --all` before each commit: the code here is already formatted.

## What was run before this plan was written

Every task's state was built here, passed `cargo fmt --check` and clippy with `-D warnings`, and its tests failed and then passed as each step says. The last state passed `cargo test --locked --workspace --all-targets` with both test servers (3,546 tests, none failed) and `RUSTDOCFLAGS='-D warnings' cargo doc --locked --workspace --no-deps`. Not run: CI.

## What this run settles

| Question | Decision | Why |
|---|---|---|
| How a masked column's uses are found | Off the tokens, by its name alone: allowed only bare in the outermost `SELECT`'s list | Agreed on 2026-10-17. The tree is not walked (B.2), and a word with the name, whatever table it is on, errs toward refusing |
| What backs the reading | Each engine's own fence, built in run 1b; no rewriting | Agreed on 2026-10-17: the SQL that runs is the SQL that was sent and logged |
| A masked column's alias | Refused wherever else it is written | Found by the run's own probing: SQLite reads an alias in a `WHERE`, and SQLite and MySQL in an `ORDER BY`'s expressions |
| `ORDER BY` a place | A number reached past parentheses and signs, `(2)` and `+2` too | Found by the probing: SQLite reads them as the place |
| A view | Refused where it reads a masked column; one an entry names has its column masked, as a table's | MySQL says what a view reads table by table, so there any view over the table is refused |
| A partition or a child table | Its column is masked with its table's | Reading it reads the same rows |
| A masked column renamed or dropped | Agents stop on the connection, on all three | Agreed on 2026-10-17, as for a hidden table |
| A connection that masks and hides nothing | The system catalogs and the users' functions are closed | Question 13: they describe or can read every column |

### What this run leaves out

- The values: an allowed `SELECT email FROM orders` answers the real ones until run 1b replaces them by each engine's result-column origins, and no answer is marked `masked` yet.
- Each engine's own fence for what the reading misses: SQLite's authorizer answering "ignore", PostgreSQL's plan read for where a column is used. Run 1b.
- The setting, the server's side and the editor: runs 2 to 4.

## File structure

| File | What changes |
|---|---|
| `crates/tabletist-db/src/agent.rs` | `Refusal::MaskedColumnUsed`, `MaskedUse` and their sentences; `SettingsNeedALook` names masked columns too |
| `crates/tabletist-db/src/agent/hidden.rs` | `Hidden::masking`, `guards_anything`, `masks`, `check_column_read`; the masked columns' tables, views and missing entries; `Catalog::columns` and `column_reads`, `Relation::view`; the table places the reading of names keeps |
| `crates/tabletist-db/src/agent/hidden/masked.rs` | New: `ColumnName`, and the rule read off the tokens |
| `crates/tabletist-db/src/lib.rs`, `adapter.rs` | `run_agent_read` and `agent_hidden` take the masked columns |
| `crates/tabletist-db/src/pg/hidden.rs`, `pg/script.rs`, `pg.rs` | The columns, what each view reads of them, and a whole row a view reads |
| `crates/tabletist-db/src/mysql/hidden.rs`, `mysql/script.rs`, `mysql.rs` | The columns |
| `crates/tabletist-db/src/sqlite.rs`, `sqlite/fence.rs` | The columns, what each view reads of them, and the authorizer's refusal of a masked column a view reads |
| `crates/tabletist-db/tests/agent.rs` | The masked-column rows of F.2 that this run builds, on all three |
| `crates/tabletist-mcp/tests/support/mod.rs`, `src/backend/agents.rs` | Pass no masked column |
| `docs/superpowers/plans/2026-10-13-mcp-server.md` | Phase 2 last; 3a's questions settled and cut; B.4 and C.3 as built |

---

### Task 1: A masked column, and the rule read off the tokens

- `agent::ColumnName`: `orders.email` or `sales.orders.email`, a part in double quotes where it holds a dot or a quote, written back as it was read; its error never says the entry's words. `TableName` and it share one reading of the parts.
- `agent/hidden/masked.rs` reads the statement's pieces (with two more kinds: `*`, and a number as written) and the table places the reading of names now keeps, each with its alias and whether a column list follows it. A word that names a masked column is refused (`MaskedUse::Named`) wherever it stands but bare in the outermost `SELECT`'s list (`email`, `o.email`, with or without an alias), whatever table it is on. Beside a masked column, or a star over a table with one, the outermost query may have no `DISTINCT`, `GROUP BY`, `HAVING`, `WINDOW`, `QUALIFY`, window function or set operation (`Clause`), and no `ORDER BY` a place (`Sorted`, with `(2)` and `+2`, which SQLite reads as one). The alias a masked column is selected under stands for it (`Alias`). Everywhere else: `*` over such a table (`Star`), the table or its alias as a value and `q.*` (`Row`), `NATURAL` (`Natural`), a column list after its alias (`Renamed`).
- `Hidden::masking` keeps the entries with what the catalog says: the tables that have one, a view an entry names too, and the partitions and child tables of those; the views that read one (`Catalog::column_reads` where the catalog says what a view reads column by column, else any view over such a table), refused by name (`MaskedUse::Through`); the entries that name no column of `Catalog::columns`, which stop agents (`SettingsNeedALook` now names tables and columns). `Relation::view` says which relations are views; each driver fills it. While anything is hidden or masked (`guards_anything`), the system catalogs and the users' functions are closed. `masks` and `check_column_read` are for the engines.

**Files:**
- Create: `crates/tabletist-db/src/agent/hidden/masked.rs`
- Modify: `crates/tabletist-db/src/agent.rs`, `crates/tabletist-db/src/agent/hidden.rs`, `crates/tabletist-db/src/pg/hidden.rs`, `crates/tabletist-db/src/mysql/hidden.rs`, `crates/tabletist-db/src/sqlite.rs`, `crates/tabletist-db/tests/agent.rs` (the stop it already checks, in its new shape)

- [ ] **Step 1: Write the failing test**

````diff
diff --git a/crates/tabletist-db/src/agent/hidden.rs b/crates/tabletist-db/src/agent/hidden.rs
index edba397..17e0778 100644
--- a/crates/tabletist-db/src/agent/hidden.rs
+++ b/crates/tabletist-db/src/agent/hidden.rs
@@ -1000,6 +1000,7 @@ mod tests {
                 .collect(),
             opaque: false,
             part: false,
+            view: false,
         }
     }
 
@@ -1021,8 +1022,7 @@ mod tests {
                 relation("public", "events_2026", &[("public", "events")]),
             ],
             routines: vec![("public".into(), "names".into())],
-            unnamed_calls: Vec::new(),
-            table_functions: Vec::new(),
+            ..Catalog::default()
         }
     }
 
@@ -1475,17 +1475,17 @@ mod tests {
         ]));
         assert_eq!(
             renamed.check_settings(),
-            Err(Refusal::SettingsNeedALook(vec![
-                "clients".into(),
-                "sales.orders".into()
-            ]))
+            Err(Refusal::SettingsNeedALook {
+                tables: vec!["clients".into(), "sales.orders".into()],
+                columns: Vec::new(),
+            })
         );
         assert_eq!(
             statement(&renamed, "SELECT * FROM books"),
-            Err(Refusal::SettingsNeedALook(vec![
-                "clients".into(),
-                "sales.orders".into()
-            ]))
+            Err(Refusal::SettingsNeedALook {
+                tables: vec!["clients".into(), "sales.orders".into()],
+                columns: Vec::new(),
+            })
         );
         // A name in another case is the table's; a system catalog's is
         // hidden with anything.
@@ -1550,4 +1550,190 @@ mod tests {
         // An empty "Only" hides every table.
         assert!(Tables::Only(Vec::new()).hides_anything());
     }
+
+    /// The Bookshop with masked columns: `orders` and its partition, a
+    /// view that reads `orders.email`, one that reads only its `id`, one
+    /// over the first, and the columns of `orders`.
+    fn masking(masked: &[&str], column_reads: Option<Vec<ColumnRead>>) -> Hidden {
+        let view = |name: &str, on: &[(&str, &str)]| Relation {
+            view: true,
+            ..relation("public", name, on)
+        };
+        let catalog = Catalog {
+            relations: vec![
+                relation("public", "books", &[]),
+                relation("public", "orders", &[]),
+                Relation {
+                    part: true,
+                    ..relation("public", "orders_2026", &[("public", "orders")])
+                },
+                view("recent_orders", &[("public", "orders")]),
+                view("order_ids", &[("public", "orders")]),
+                view("recent_mail", &[("public", "recent_orders")]),
+            ],
+            routines: vec![("public".into(), "names".into())],
+            columns: ["id", "email", "phone"]
+                .iter()
+                .map(|column| ("public".into(), "orders".into(), (*column).into()))
+                .chain([("public".into(), "order_ids".into(), "id".into())])
+                .collect(),
+            column_reads,
+            ..Catalog::default()
+        };
+        let masked: Vec<ColumnName> = masked.iter().map(|entry| entry.parse().unwrap()).collect();
+        Hidden::masking(Dialect::Postgres, &catalog, &Tables::All, &masked)
+    }
+
+    /// What PostgreSQL says the views read, column by column.
+    fn reads() -> Option<Vec<ColumnRead>> {
+        let read = |view: &str, table: &str, column: &str| ColumnRead {
+            schema: "public".into(),
+            view: view.into(),
+            table_schema: "public".into(),
+            table: table.into(),
+            column: column.into(),
+        };
+        Some(vec![
+            read("recent_orders", "orders", "email"),
+            read("recent_orders", "orders", "id"),
+            read("order_ids", "orders", "id"),
+        ])
+    }
+
+    fn masked_used(column: &str, how: MaskedUse) -> Result<(), Refusal> {
+        Err(Refusal::MaskedColumnUsed {
+            column: column.into(),
+            how,
+        })
+    }
+
+    #[test]
+    fn a_view_that_reads_a_masked_column_is_refused() {
+        let hidden = masking(&["orders.email"], reads());
+        let through = |view: &str| masked_used("orders.email", MaskedUse::Through(view.into()));
+        assert_eq!(
+            statement(&hidden, "SELECT * FROM recent_orders"),
+            through("recent_orders")
+        );
+        // A view over it reads it too; one that reads another column does
+        // not.
+        assert_eq!(
+            statement(&hidden, "SELECT * FROM recent_mail"),
+            through("recent_mail")
+        );
+        assert_eq!(statement(&hidden, "SELECT * FROM order_ids"), Ok(()));
+        // As the engine names it, and not hidden by it.
+        assert_eq!(
+            hidden.check_engine_read(Some("public"), "recent_orders"),
+            through("recent_orders")
+        );
+        assert!(!hidden.hides("public", "recent_orders"));
+        // Where the catalog does not say what a view reads column by
+        // column, a view over the table reads it.
+        let unsaid = masking(&["orders.email"], None);
+        assert_eq!(
+            statement(&unsaid, "SELECT * FROM order_ids"),
+            through("order_ids")
+        );
+        // A whole row a view reads is every column of it.
+        let mut rows = reads().unwrap();
+        rows.push(ColumnRead {
+            column: "*".into(),
+            ..rows[2].clone()
+        });
+        let whole = masking(&["orders.email"], Some(rows));
+        assert_eq!(
+            statement(&whole, "SELECT * FROM order_ids"),
+            through("order_ids")
+        );
+    }
+
+    #[test]
+    fn a_masked_column_is_masked_in_the_partitions_of_its_table() {
+        let hidden = masking(&["orders.email"], reads());
+        assert_eq!(
+            hidden.masks(Some("public"), "orders_2026", "email"),
+            Some("orders.email".into())
+        );
+        assert_eq!(hidden.masks(Some("public"), "orders_2026", "id"), None);
+        assert_eq!(
+            hidden.masks(Some("public"), "ORDERS", "Email"),
+            Some("orders.email".into())
+        );
+        // So a star over the partition is a star over the table.
+        assert_eq!(
+            statement(&hidden, "SELECT x.id FROM (SELECT * FROM orders_2026) x"),
+            masked_used("orders.email", MaskedUse::Star)
+        );
+        // The engine's read of it for a view, or for a common table
+        // expression, is refused; one the statement makes is its check's.
+        assert_eq!(
+            hidden.check_column_read(Some("main"), "orders", "email", Some("w")),
+            masked_used("orders.email", MaskedUse::Through("w".into()))
+        );
+        assert_eq!(
+            hidden.check_column_read(Some("main"), "orders", "email", None),
+            Ok(())
+        );
+        assert_eq!(
+            hidden.check_column_read(Some("main"), "orders", "id", Some("w")),
+            Ok(())
+        );
+        // A view's own column, masked: the view reads, as a table with a
+        // masked column does.
+        let view = masking(&["order_ids.id"], reads());
+        assert_eq!(view.check_settings(), Ok(()));
+        assert_eq!(
+            view.masks(Some("public"), "order_ids", "id"),
+            Some("order_ids.id".into())
+        );
+        assert_eq!(statement(&view, "SELECT * FROM order_ids"), Ok(()));
+        assert_eq!(
+            statement(&view, "SELECT x.n FROM (SELECT * FROM order_ids) x"),
+            masked_used("order_ids.id", MaskedUse::Star)
+        );
+    }
+
+    #[test]
+    fn a_masked_column_renamed_or_dropped_stops_agents_on_the_connection() {
+        let gone = masking(
+            &["orders.email", "orders.mail", "sales.orders.email"],
+            reads(),
+        );
+        let look = Refusal::SettingsNeedALook {
+            tables: Vec::new(),
+            columns: vec!["orders.mail".into(), "sales.orders.email".into()],
+        };
+        assert_eq!(gone.check_settings(), Err(look.clone()));
+        assert_eq!(statement(&gone, "SELECT * FROM books"), Err(look.clone()));
+        assert_eq!(masking(&["ORDERS.EMAIL"], reads()).check_settings(), Ok(()));
+        // The sentence names them.
+        let said = look.to_string();
+        assert!(
+            said.contains("they mask orders.mail, sales.orders.email"),
+            "{said}"
+        );
+    }
+
+    #[test]
+    fn a_connection_that_masks_a_column_closes_the_catalogs_and_the_users_functions() {
+        let hidden = masking(&["orders.email"], reads());
+        assert!(hidden.guards_anything());
+        assert!(!hidden.hides_anything());
+        assert_eq!(
+            statement(&hidden, "SELECT relname FROM pg_catalog.pg_class"),
+            Err(Refusal::SystemCatalog)
+        );
+        assert_eq!(
+            statement(&hidden, "SELECT names()"),
+            Err(Refusal::FunctionNotAllowed("names".into()))
+        );
+        // Masking nothing, they are open.
+        let open = masking(&[], reads());
+        assert!(!open.guards_anything());
+        assert_eq!(
+            statement(&open, "SELECT relname FROM pg_catalog.pg_class"),
+            Ok(())
+        );
+    }
 }
diff --git a/crates/tabletist-db/src/agent/hidden/masked.rs b/crates/tabletist-db/src/agent/hidden/masked.rs
new file mode 100644
index 0000000..a995d07
--- /dev/null
+++ b/crates/tabletist-db/src/agent/hidden/masked.rs
@@ -0,0 +1,247 @@
+//! Masked columns: the columns of a connection whose values agents get as
+//! `•••` (the MCP plan's B.4), and the reading of a statement that keeps
+//! what is in them from being filtered, joined, sorted or computed on.
+//!
+//! The statement is read off its tokens, as its tables' names are (B.2:
+//! the tree is not walked), and by a masked column's name alone: a word
+//! that is one, whatever table or alias it is written with, may stand only
+//! bare in the outermost `SELECT`'s list, where its value is replaced as it
+//! is answered. That errs toward refusing. What the reading misses is each
+//! engine's to stop.
+
+#[cfg(test)]
+mod tests {
+    use super::*;
+    use crate::Dialect;
+    use crate::agent::check_read;
+
+    fn column(written: &str) -> ColumnName {
+        written.parse().unwrap()
+    }
+
+    /// `sql` checked on a connection that masks `orders.email`.
+    fn on(dialect: Dialect, sql: &str) -> Result<(), MaskedUse> {
+        let masked = [column("orders.email")];
+        let read = check_read(dialect, sql, &[]).unwrap_or_else(|error| panic!("{sql}: {error:?}"));
+        check(&read, &Masks::new(&masked, [])).map_err(|refusal| match refusal {
+            Refusal::MaskedColumnUsed { column, how } => {
+                assert_eq!(column, "orders.email", "{sql}");
+                how
+            }
+            other => panic!("{sql}: {other:?}"),
+        })
+    }
+
+    fn pg(sql: &str) -> Result<(), MaskedUse> {
+        on(Dialect::Postgres, sql)
+    }
+
+    #[test]
+    fn a_masked_column_is_written_as_the_person_writes_it() {
+        let read = column("orders.email");
+        assert_eq!(
+            (
+                read.schema.as_deref(),
+                read.table.as_str(),
+                read.column.as_str()
+            ),
+            (None, "orders", "email")
+        );
+        let full = column("sales.orders.email");
+        assert_eq!(full.schema.as_deref(), Some("sales"));
+        assert_eq!(full.to_string(), "sales.orders.email");
+        let quoted = column(r#""odd.table".email"#);
+        assert_eq!(quoted.table, "odd.table");
+        assert_eq!(quoted.to_string(), r#""odd.table".email"#);
+        // A column alone is no entry, nor are four parts; the error never
+        // says the entry's words.
+        let alone = "email".parse::<ColumnName>().unwrap_err();
+        assert!(alone.contains("no table"), "{alone}");
+        assert!(!alone.contains("email"), "{alone}");
+        assert!("a.b.c.d".parse::<ColumnName>().is_err());
+        assert!("orders.".parse::<ColumnName>().is_err());
+        // In another case, and in any schema without one.
+        assert!(read.names(Some("public"), "Orders", "EMAIL"));
+        assert!(!full.names(Some("public"), "orders", "email"));
+        assert!(full.names(None, "orders", "email"));
+    }
+
+    #[test]
+    fn a_masked_column_is_selected_as_it_is_and_used_no_other_way() {
+        let named = || Err(MaskedUse::Named("email".into()));
+        // The brief's cases.
+        assert_eq!(pg("SELECT id FROM orders WHERE email LIKE 'a%'"), named());
+        assert_eq!(pg("SELECT lower(email) FROM orders"), named());
+        assert_eq!(pg("SELECT id FROM orders ORDER BY email"), named());
+        assert_eq!(
+            pg("SELECT id, email FROM orders ORDER BY 2"),
+            Err(MaskedUse::Sorted("2".into()))
+        );
+        for sorted in ["(2)", "+2", "((2))", "2 COLLATE nocase"] {
+            let sql = format!("SELECT id, email FROM orders ORDER BY {sorted}");
+            assert_eq!(
+                on(Dialect::Sqlite, &sql),
+                Err(MaskedUse::Sorted("2".into())),
+                "{sql}"
+            );
+        }
+        assert!(pg("SELECT email FROM orders GROUP BY email").is_err());
+        assert_eq!(pg("SELECT count(DISTINCT email) FROM orders"), named());
+        assert_eq!(
+            pg("SELECT o.id FROM orders o JOIN customers c ON c.email = o.email"),
+            named()
+        );
+        // And the plan's.
+        assert_eq!(
+            pg("SELECT DISTINCT email FROM orders"),
+            Err(MaskedUse::Clause("DISTINCT"))
+        );
+        assert_eq!(
+            pg("SELECT DISTINCT ON (id) email FROM orders"),
+            Err(MaskedUse::Clause("DISTINCT"))
+        );
+        assert_eq!(
+            pg("SELECT email FROM orders UNION SELECT name FROM customers"),
+            Err(MaskedUse::Clause("UNION, INTERSECT or EXCEPT"))
+        );
+        assert_eq!(
+            pg("SELECT * FROM orders NATURAL JOIN customers"),
+            Err(MaskedUse::Natural)
+        );
+        assert_eq!(
+            pg("SELECT o.id FROM orders o JOIN customers c USING (email)"),
+            named()
+        );
+        assert_eq!(
+            pg("SELECT x.id FROM (SELECT * FROM orders) x WHERE x.id > 1"),
+            Err(MaskedUse::Star)
+        );
+        assert_eq!(
+            pg("SELECT c FROM orders AS t(a, b, c)"),
+            Err(MaskedUse::Renamed("t".into()))
+        );
+        assert_eq!(
+            pg("SELECT o FROM orders o"),
+            Err(MaskedUse::Row("o".into()))
+        );
+        assert_eq!(
+            pg("SELECT to_jsonb(o) FROM orders o"),
+            Err(MaskedUse::Row("o".into()))
+        );
+        assert_eq!(
+            pg("SELECT row_to_json(o.*) FROM orders o"),
+            Err(MaskedUse::Row("o.*".into()))
+        );
+        assert_eq!(
+            pg("SELECT id, row_number() OVER (ORDER BY email) FROM orders"),
+            named()
+        );
+        assert_eq!(
+            pg("SELECT email, row_number() OVER (ORDER BY id) FROM orders"),
+            Err(MaskedUse::Clause("a window function"))
+        );
+        assert_eq!(pg("SELECT id FROM orders WHERE email IS NULL"), named());
+        assert_eq!(
+            pg("SELECT CASE WHEN email LIKE 'a%' THEN 1 END FROM orders"),
+            named()
+        );
+        assert_eq!(pg("SELECT email::text FROM orders"), named());
+        assert_eq!(pg("SELECT CAST(email AS text) FROM orders"), named());
+        // Its alias stands for it, wherever else it is written.
+        assert_eq!(
+            pg("SELECT email AS e FROM orders ORDER BY e"),
+            Err(MaskedUse::Alias("e".into()))
+        );
+        assert_eq!(
+            pg("WITH w AS (SELECT email FROM orders) SELECT * FROM w"),
+            named()
+        );
+        // By its name, whatever its table.
+        assert_eq!(pg("SELECT id FROM customers WHERE email = 'x'"), named());
+        assert_eq!(pg(r#"SELECT id FROM orders WHERE "EMAIL" = 'x'"#), named());
+    }
+
+    #[test]
+    fn what_selects_a_masked_column_as_it_is_is_taken() {
+        for sql in [
+            "SELECT id, email FROM orders",
+            "SELECT * FROM orders",
+            "SELECT *, id FROM orders",
+            "SELECT o.email AS e FROM orders o",
+            "SELECT public.orders.email FROM public.orders",
+            "SELECT o.* FROM orders o JOIN customers c ON c.id = o.customer_id",
+            "SELECT count(*) FROM orders",
+            "SELECT email FROM orders ORDER BY id LIMIT 5",
+            "SELECT id FROM orders WHERE id IN (SELECT order_id FROM order_items)",
+            "SELECT 2 * 3, email FROM orders",
+            "EXPLAIN SELECT email FROM orders",
+            "SELECT email FROM orders WHERE id = 3",
+            // Beside no masked column, a query is as free as ever.
+            "SELECT DISTINCT id FROM orders",
+            "SELECT id FROM orders GROUP BY id",
+            "SELECT * FROM customers UNION SELECT * FROM customers",
+            "SELECT * FROM customers NATURAL JOIN books",
+        ] {
+            assert_eq!(pg(sql), Ok(()), "{sql}");
+        }
+    }
+
+    #[test]
+    fn mysql_and_sqlite_are_read_the_same_way() {
+        let named = || Err(MaskedUse::Named("email".into()));
+        assert_eq!(
+            on(
+                Dialect::MySql,
+                "SELECT `email` FROM orders WHERE `email` = 'x'"
+            ),
+            named()
+        );
+        assert_eq!(
+            on(Dialect::MySql, "SELECT SQL_NO_CACHE email FROM orders"),
+            Ok(())
+        );
+        assert_eq!(
+            on(Dialect::MySql, "SELECT DISTINCTROW email FROM orders"),
+            Err(MaskedUse::Clause("DISTINCT"))
+        );
+        assert_eq!(
+            on(
+                Dialect::Sqlite,
+                "SELECT id FROM orders WHERE email GLOB 'a*'"
+            ),
+            named()
+        );
+        assert_eq!(
+            on(
+                Dialect::Sqlite,
+                "SELECT [email] FROM orders ORDER BY [email]"
+            ),
+            named()
+        );
+        assert_eq!(on(Dialect::Sqlite, "SELECT id, email FROM orders"), Ok(()));
+        // SQLite reads an alias in a WHERE, and SQLite and MySQL in an
+        // ORDER BY's expressions.
+        let alias = || Err(MaskedUse::Alias("e".into()));
+        assert_eq!(
+            on(
+                Dialect::Sqlite,
+                "SELECT email AS e FROM orders WHERE e LIKE 'a%'"
+            ),
+            alias()
+        );
+        assert_eq!(
+            on(
+                Dialect::Sqlite,
+                "SELECT id, email e FROM orders ORDER BY lower(e)"
+            ),
+            alias()
+        );
+        assert_eq!(
+            on(
+                Dialect::MySql,
+                "SELECT id, email AS e FROM orders ORDER BY concat(e, '')"
+            ),
+            alias()
+        );
+    }
+}
diff --git a/crates/tabletist-db/src/mysql/hidden.rs b/crates/tabletist-db/src/mysql/hidden.rs
index c0c711a..f93310a 100644
--- a/crates/tabletist-db/src/mysql/hidden.rs
+++ b/crates/tabletist-db/src/mysql/hidden.rs
@@ -264,12 +264,12 @@ mod tests {
             depends_on: Vec::new(),
             opaque: false,
             part: false,
+            view: false,
         };
         let catalog = Catalog {
             relations: vec![relation("books"), relation("customers")],
             routines: vec![("bookshop".into(), "customer_count".into())],
-            unnamed_calls: Vec::new(),
-            table_functions: Vec::new(),
+            ..Catalog::default()
         };
         Hidden::new(Dialect::MySql, &catalog, &tables)
     }
diff --git a/crates/tabletist-db/src/pg/hidden.rs b/crates/tabletist-db/src/pg/hidden.rs
index 5d8dc80..f6700f9 100644
--- a/crates/tabletist-db/src/pg/hidden.rs
+++ b/crates/tabletist-db/src/pg/hidden.rs
@@ -281,6 +281,7 @@ mod tests {
                 .collect(),
             opaque: false,
             part: false,
+            view: false,
         };
         let catalog = Catalog {
             relations: vec![
@@ -290,8 +291,7 @@ mod tests {
                 relation("events_2026", &["events"]),
             ],
             routines: vec![("public".into(), "names".into())],
-            unnamed_calls: Vec::new(),
-            table_functions: Vec::new(),
+            ..Catalog::default()
         };
         Hidden::new(Dialect::Postgres, &catalog, &tables)
     }
diff --git a/crates/tabletist-db/tests/agent.rs b/crates/tabletist-db/tests/agent.rs
index 835b0e0..d1cb49a 100644
--- a/crates/tabletist-db/tests/agent.rs
+++ b/crates/tabletist-db/tests/agent.rs
@@ -470,7 +470,10 @@ async fn a_hidden_table_is_never_read(connection: &Connection, on: &str) {
     let error = hiding(connection, &gone, "SELECT count(*) FROM books")
         .await
         .unwrap_err();
-    let look = Refusal::SettingsNeedALook(vec!["clients".into()]);
+    let look = Refusal::SettingsNeedALook {
+        tables: vec!["clients".into()],
+        columns: Vec::new(),
+    };
     assert_eq!(error, Error::Agent(look.clone()), "{on}");
     let hidden = connection.agent_hidden(&gone).await.unwrap();
     assert_eq!(hidden.check_settings(), Err(look), "{on}");
````

- [ ] **Step 2: Run it to see that it does not build**

Run: `cargo test --locked -p tabletist-db --lib masked`

Expected: it does not build:

```text
error[E0425]: cannot find type `ColumnName` in this scope
error[E0599]: no associated function or constant named `masking` found for struct `agent::hidden::Hidden` in the current scope
error: could not compile `tabletist-db` (lib test) due to 33 previous errors
```

- [ ] **Step 3: The rule, and what the catalog says of the entries**

````diff
diff --git a/crates/tabletist-db/src/agent.rs b/crates/tabletist-db/src/agent.rs
index 390f8da..83f3a67 100644
--- a/crates/tabletist-db/src/agent.rs
+++ b/crates/tabletist-db/src/agent.rs
@@ -31,7 +31,7 @@ use crate::{Dialect, sql};
 mod hidden;
 mod run;
 
-pub use hidden::{Catalog, Hidden, Relation, TableName, Tables};
+pub use hidden::{Catalog, ColumnName, ColumnRead, Hidden, Relation, TableName, Tables};
 pub(crate) use hidden::{calls_a_refused_function, named_relations, planned};
 pub use run::{Answer, Limits};
 pub(crate) use run::{Sent, answer, count_mismatch};
@@ -143,15 +143,54 @@ pub enum Refusal {
         name: String,
         through: Option<String>,
     },
-    /// The system catalogs, on a connection that hides a table: they
-    /// describe every one.
+    /// The system catalogs, on a connection that hides a table or masks a
+    /// column: they describe every one.
     SystemCatalog,
+    /// A masked column read otherwise than selected as it is, by its entry
+    /// as the person writes it, and how the statement reads it.
+    MaskedColumnUsed {
+        column: String,
+        how: MaskedUse,
+    },
     /// The connection's "All except" list names tables its user cannot
-    /// see, as it writes them: a hidden table renamed, or dropped, which
-    /// look the same, or on MySQL one the user has no right to. Agents may not use the connection until the person looks
-    /// (the MCP plan's C.3). Its code is the server's for a connection it
-    /// cannot open.
-    SettingsNeedALook(Vec<String>),
+    /// see, or its masked columns name columns it cannot, as the person
+    /// writes them: one renamed, or dropped, which look the same, or on
+    /// MySQL one the user has no right to. Agents may not use the
+    /// connection until the person looks (the MCP plan's C.3). Its code is
+    /// the server's for a connection it cannot open.
+    SettingsNeedALook {
+        tables: Vec<String>,
+        columns: Vec<String>,
+    },
+}
+
+/// How a statement reads a masked column otherwise than selecting it as it
+/// is in its outermost `SELECT` (the MCP plan's B.4).
+#[derive(Debug, Clone, PartialEq, Eq)]
+pub enum MaskedUse {
+    /// A word that is its name, anywhere else: as the column's name, since
+    /// a column of that name is refused whatever its table.
+    Named(String),
+    /// `*` over its table in a subquery or a common table expression,
+    /// where the outer query could filter on it under another name.
+    Star,
+    /// Its table's whole row, by the table's name or its alias as a value,
+    /// as written.
+    Row(String),
+    /// A `NATURAL JOIN`, which may join on it.
+    Natural,
+    /// A list of column names after its table's alias, as written, which
+    /// gives its columns other names.
+    Renamed(String),
+    /// A clause of the outermost query beside it, as named.
+    Clause(&'static str),
+    /// An `ORDER BY` of a place in the list, as written, which may be its.
+    Sorted(String),
+    /// The alias it is selected under, written elsewhere in the statement,
+    /// where SQLite and MySQL read it as the column.
+    Alias(String),
+    /// A view or a common table expression that reads it, by its name.
+    Through(String),
 }
 
 /// What was too large.
@@ -202,7 +241,8 @@ impl Refusal {
             Self::InvalidParams(_) => "invalid_params",
             Self::TableHidden { .. } => "table_hidden",
             Self::SystemCatalog => "system_catalog",
-            Self::SettingsNeedALook(_) => "connection_unavailable",
+            Self::MaskedColumnUsed { .. } => "masked_column_used",
+            Self::SettingsNeedALook { .. } => "connection_unavailable",
         }
     }
 }
@@ -280,12 +320,28 @@ impl fmt::Display for Refusal {
                 f,
                 "{through} reads {name}, which is hidden for agents on this connection."
             ),
-            Self::SettingsNeedALook(missing) => write!(
-                f,
-                "Agents may not use this connection until its agent settings are looked at in \
-                 Tabletist: they hide {}, and its user can see no table of that name.",
-                missing.join(", ")
-            ),
+            Self::SettingsNeedALook { tables, columns } => {
+                let mut said = Vec::new();
+                if !tables.is_empty() {
+                    said.push(format!(
+                        "they hide {}, and its user can see no table of that name",
+                        tables.join(", ")
+                    ));
+                }
+                if !columns.is_empty() {
+                    said.push(format!(
+                        "they mask {}, and its user can see no column of that name",
+                        columns.join(", ")
+                    ));
+                }
+                write!(
+                    f,
+                    "Agents may not use this connection until its agent settings are looked at in \
+                     Tabletist: {}.",
+                    said.join("; ")
+                )
+            }
+            Self::MaskedColumnUsed { column, how } => masked_sentence(f, column, how),
             Self::SystemCatalog => f.write_str(
                 "The system catalogs are not available on this connection. Use describe_schema.",
             ),
@@ -293,6 +349,57 @@ impl fmt::Display for Refusal {
     }
 }
 
+/// The sentence for a masked column `column` read as `how` reads it.
+fn masked_sentence(f: &mut fmt::Formatter<'_>, column: &str, how: &MaskedUse) -> fmt::Result {
+    let selected = "It can be selected as it is in the outermost SELECT";
+    match how {
+        MaskedUse::Named(name) => write!(
+            f,
+            "{column} is masked on this connection. {selected}, and not filtered, joined, \
+             sorted, grouped or computed on, and that goes for any column named {name}."
+        ),
+        MaskedUse::Star => write!(
+            f,
+            "{column} is masked on this connection, so * over its table is taken only in the \
+             outermost SELECT. Name the columns a subquery needs."
+        ),
+        MaskedUse::Row(written) => write!(
+            f,
+            "{column} is masked on this connection, and {written} reads its table's whole row. \
+             Select the columns instead."
+        ),
+        MaskedUse::Natural => write!(
+            f,
+            "{column} is masked on this connection, and a NATURAL JOIN may join on it. Join \
+             with ON instead."
+        ),
+        MaskedUse::Renamed(written) => write!(
+            f,
+            "{column} is masked on this connection, and the column names after {written} give \
+             its table's columns other names. Name each column with AS instead."
+        ),
+        MaskedUse::Clause(clause) => write!(
+            f,
+            "{column} is masked on this connection, and a query that selects it may not use \
+             {clause}. {selected}, in a plain query."
+        ),
+        MaskedUse::Sorted(written) => write!(
+            f,
+            "{column} is masked on this connection, and ORDER BY {written} may sort by it. \
+             Sort by the other columns, by name."
+        ),
+        MaskedUse::Alias(alias) => write!(
+            f,
+            "{column} is masked on this connection, and it is selected as {alias}, which then \
+             stands for it: {alias} may be written nowhere else in the statement."
+        ),
+        MaskedUse::Through(view) => write!(
+            f,
+            "{view} reads {column}, which is masked on this connection."
+        ),
+    }
+}
+
 /// "1 parameter", "2 parameters".
 fn parameters(count: usize) -> String {
     match count {
diff --git a/crates/tabletist-db/src/agent/hidden.rs b/crates/tabletist-db/src/agent/hidden.rs
index 17e0778..976a3cf 100644
--- a/crates/tabletist-db/src/agent/hidden.rs
+++ b/crates/tabletist-db/src/agent/hidden.rs
@@ -20,10 +20,20 @@
 //! Names are compared without regard to case, which is the strict reading
 //! where a database tells case apart: a quoted `"Customers"` on PostgreSQL
 //! is taken for `customers` too, and refused where that is hidden.
+//!
+//! The masked columns are checked with the hidden tables ([`masked`]): a
+//! connection that masks a column closes the system catalogs and the
+//! users' functions as one that hides a table does, and a view that reads
+//! a masked column is refused.
+
+mod masked;
 
 use std::collections::{HashMap, HashSet};
 
-use super::{Read, Refusal};
+pub use masked::ColumnName;
+use masked::Masks;
+
+use super::{MaskedUse, Read, Refusal};
 use crate::Dialect;
 use crate::sql::{self, TokenKind};
 
@@ -71,55 +81,7 @@ impl std::str::FromStr for TableName {
     type Err = String;
 
     fn from_str(written: &str) -> Result<Self, String> {
-        if written.is_empty() {
-            return Err("is empty".into());
-        }
-        let mut parts: Vec<String> = Vec::new();
-        let mut chars = written.chars().peekable();
-        loop {
-            let mut part = String::new();
-            let quoted = chars.peek() == Some(&'"');
-            if quoted {
-                chars.next();
-                loop {
-                    match chars.next() {
-                        Some('"') if chars.peek() == Some(&'"') => {
-                            chars.next();
-                            part.push('"');
-                        }
-                        Some('"') => break,
-                        Some(c) => part.push(c),
-                        None => return Err("has a quote that is not closed".into()),
-                    }
-                }
-            } else {
-                while let Some(&c) = chars.peek()
-                    && c != '.'
-                {
-                    if c == '"' {
-                        return Err(
-                            "has a quote inside a name: put the whole name in quotes".into()
-                        );
-                    }
-                    part.push(c);
-                    chars.next();
-                }
-            }
-            match (part.is_empty(), quoted) {
-                (true, true) => return Err("has a quoted name that is empty".into()),
-                (true, false) => return Err("has nothing before or after a dot".into()),
-                (false, _) => {}
-            }
-            parts.push(part);
-            match chars.next() {
-                None => break,
-                Some('.') => {}
-                Some(_) => {
-                    return Err("goes on after a quoted name: a dot comes next".into());
-                }
-            }
-        }
-        match <[String; 2]>::try_from(parts) {
+        match <[String; 2]>::try_from(parts(written)?) {
             Ok([schema, name]) => Ok(Self {
                 schema: Some(schema),
                 name,
@@ -134,6 +96,59 @@ impl std::str::FromStr for TableName {
     }
 }
 
+/// The parts of an entry as the person writes it, joined by dots, a part
+/// in double quotes where it holds a dot or a quote. The error says what
+/// is wrong without the entry's words.
+fn parts(written: &str) -> Result<Vec<String>, String> {
+    if written.is_empty() {
+        return Err("is empty".into());
+    }
+    let mut parts: Vec<String> = Vec::new();
+    let mut chars = written.chars().peekable();
+    loop {
+        let mut part = String::new();
+        let quoted = chars.peek() == Some(&'"');
+        if quoted {
+            chars.next();
+            loop {
+                match chars.next() {
+                    Some('"') if chars.peek() == Some(&'"') => {
+                        chars.next();
+                        part.push('"');
+                    }
+                    Some('"') => break,
+                    Some(c) => part.push(c),
+                    None => return Err("has a quote that is not closed".into()),
+                }
+            }
+        } else {
+            while let Some(&c) = chars.peek()
+                && c != '.'
+            {
+                if c == '"' {
+                    return Err("has a quote inside a name: put the whole name in quotes".into());
+                }
+                part.push(c);
+                chars.next();
+            }
+        }
+        match (part.is_empty(), quoted) {
+            (true, true) => return Err("has a quoted name that is empty".into()),
+            (true, false) => return Err("has nothing before or after a dot".into()),
+            (false, _) => {}
+        }
+        parts.push(part);
+        match chars.next() {
+            None => break,
+            Some('.') => {}
+            Some(_) => {
+                return Err("goes on after a quoted name: a dot comes next".into());
+            }
+        }
+    }
+    Ok(parts)
+}
+
 impl TableName {
     /// Whether this names the relation `name` of `schema`.
     fn names(&self, schema: Option<&str>, name: &str) -> bool {
@@ -196,6 +211,23 @@ pub struct Relation {
     /// (SQLite's shadow tables). Hidden with that one, and read with it,
     /// not hidden for want of its own name in an "Only" list.
     pub part: bool,
+    /// Whether it is a view or a materialized view, whose own data is
+    /// what it reads: one that reads a masked column is refused, where a
+    /// partition or a child table of a table with one has it masked too.
+    pub view: bool,
+}
+
+/// A column a view reads, as a catalog that says so column by column
+/// gives it: the view by its schema and name, the column by its table's
+/// schema and name and its own. A column of `*` is a whole row, which
+/// PostgreSQL keeps no column of.
+#[derive(Debug, Clone, PartialEq, Eq)]
+pub struct ColumnRead {
+    pub schema: String,
+    pub view: String,
+    pub table_schema: String,
+    pub table: String,
+    pub column: String,
 }
 
 /// What a driver reads of its database's catalog for the checks: its
@@ -213,6 +245,14 @@ pub struct Catalog {
     /// engine reports reads of as a table's (SQLite's `json_each`): one the
     /// catalog does not list as a relation is never hidden.
     pub table_functions: Vec<String>,
+    /// The columns of the tables the masked columns' entries name, by
+    /// schema, table and column: what says an entry still names a column.
+    /// Read only where something is masked.
+    pub columns: Vec<(String, String, String)>,
+    /// The masked columns each view reads, where the catalog says what a
+    /// view reads column by column. `None` where it does not: a view over a
+    /// table with a masked column is then taken to read it.
+    pub column_reads: Option<Vec<ColumnRead>>,
 }
 
 /// A relation of the catalog, as [`Hidden`] keeps it.
@@ -221,6 +261,9 @@ struct Listed {
     schema: String,
     name: String,
     why: Option<Why>,
+    /// For a view that reads a masked column, the column, as the person
+    /// writes it.
+    masks: Option<String>,
 }
 
 /// Why a relation is hidden.
@@ -252,12 +295,32 @@ pub struct Hidden {
     /// The entries of an "All except" list that name no relation of the
     /// catalog, as the person writes them.
     missing: Vec<String>,
+    /// The columns the person masks.
+    masked: Vec<ColumnName>,
+    /// The tables that have a masked column, by schema and name, with the
+    /// entry's place in `masked`: the entries' own, and the partitions and
+    /// child tables of those.
+    masked_tables: Vec<(String, String, usize)>,
+    /// The masked columns' entries that name no column of the catalog, as
+    /// the person writes them.
+    masked_missing: Vec<String>,
 }
 
 impl Hidden {
     /// What `tables` hides of `catalog`, on a database that speaks
     /// `dialect`.
     pub fn new(dialect: Dialect, catalog: &Catalog, tables: &Tables) -> Self {
+        Self::masking(dialect, catalog, tables, &[])
+    }
+
+    /// What `tables` hides of `catalog`, and what `masked` masks.
+    pub fn masking(
+        dialect: Dialect,
+        catalog: &Catalog,
+        tables: &Tables,
+        masked: &[ColumnName],
+    ) -> Self {
+        let guards = tables.hides_anything() || !masked.is_empty();
         let index: HashMap<(String, String), usize> = catalog
             .relations
             .iter()
@@ -279,8 +342,10 @@ impl Hidden {
         for at in 0..catalog.relations.len() {
             hidden_why(at, catalog, &depends, tables, &mut why);
         }
+        let masked_tables = masked_tables(catalog, &depends, masked);
+        let masks = masked_views(catalog, &depends, masked, &masked_tables);
         let mut relations: HashMap<String, Vec<Listed>> = HashMap::new();
-        for (relation, why) in catalog.relations.iter().zip(why) {
+        for ((relation, why), masks) in catalog.relations.iter().zip(why).zip(masks) {
             relations
                 .entry(fold(&relation.name))
                 .or_default()
@@ -288,9 +353,10 @@ impl Hidden {
                     schema: relation.schema.clone(),
                     name: relation.name.clone(),
                     why: why.flatten(),
+                    masks: masks.map(|at| masked[at].to_string()),
                 });
         }
-        let routines = match tables.hides_anything() {
+        let routines = match guards {
             true => catalog
                 .routines
                 .iter()
@@ -298,10 +364,21 @@ impl Hidden {
                 .collect(),
             false => HashSet::new(),
         };
-        let unnamed_calls = match tables.hides_anything() {
+        let unnamed_calls = match guards {
             true => catalog.unnamed_calls.clone(),
             false => Vec::new(),
         };
+        let masked_missing = masked
+            .iter()
+            .filter(|entry| {
+                !system_catalog(dialect, entry.schema.as_deref(), &entry.table)
+                    && !catalog
+                        .columns
+                        .iter()
+                        .any(|(schema, table, column)| entry.names(Some(schema), table, column))
+            })
+            .map(ToString::to_string)
+            .collect();
         Self {
             dialect,
             tables: tables.clone(),
@@ -314,6 +391,9 @@ impl Hidden {
                 .map(|name| fold(name))
                 .collect(),
             missing: missing(dialect, catalog, tables),
+            masked: masked.to_vec(),
+            masked_tables,
+            masked_missing,
         }
     }
 
@@ -322,20 +402,84 @@ impl Hidden {
         self.tables.hides_anything()
     }
 
+    /// Whether anything is hidden or masked: then the system catalogs and
+    /// the functions the users wrote are closed, and the statements are
+    /// checked.
+    pub fn guards_anything(&self) -> bool {
+        self.hides_anything() || !self.masked.is_empty()
+    }
+
     /// Whether agents may use the connection at all: not while its "All
-    /// except" list names a table it does not have. A hidden table that was
-    /// renamed is that, and its new name is hidden by nothing.
+    /// except" list names a table it does not have, or a masked column's
+    /// entry names no column. A hidden table or a masked column that was
+    /// renamed is that, and its new name is hidden or masked by nothing.
     pub fn check_settings(&self) -> Result<(), Refusal> {
-        match self.missing.is_empty() {
+        match self.missing.is_empty() && self.masked_missing.is_empty() {
             true => Ok(()),
-            false => Err(Refusal::SettingsNeedALook(self.missing.clone())),
+            false => Err(Refusal::SettingsNeedALook {
+                tables: self.missing.clone(),
+                columns: self.masked_missing.clone(),
+            }),
         }
     }
 
     /// Whether the relation `name` of `schema` is hidden from agents: by
-    /// the person's names, by what it reads, or as a system catalog.
+    /// the person's names, by what it reads, or as a system catalog. A view
+    /// that reads a masked column is refused, and not hidden by this.
     pub fn hides(&self, schema: &str, name: &str) -> bool {
-        self.check_engine_read(Some(schema), name).is_err()
+        matches!(
+            self.check_engine_read(Some(schema), name),
+            Err(refusal) if !matches!(refusal, Refusal::MaskedColumnUsed { .. })
+        )
+    }
+
+    /// The masked column `column` of the relation `table` of `schema`, as
+    /// the person writes its entry, where it is one: a column the entries
+    /// name, or one of a partition or a child table of their tables.
+    pub fn masks(&self, schema: Option<&str>, table: &str, column: &str) -> Option<String> {
+        let by_entry = self
+            .masked
+            .iter()
+            .find(|entry| entry.names(schema, table, column));
+        let by_table = || {
+            self.masked_tables
+                .iter()
+                .filter(|(own, name, _)| {
+                    same(name, table) && schema.is_none_or(|schema| same(own, schema))
+                })
+                .map(|(_, _, at)| &self.masked[*at])
+                .find(|entry| same(&entry.column, column))
+        };
+        by_entry.or_else(by_table).map(ToString::to_string)
+    }
+
+    /// Checks a column the engine says a statement reads, with the view or
+    /// the common table expression it reads it for (SQLite's authorizer):
+    /// a masked one read for one is refused, naming it. One the statement
+    /// reads itself is its check's.
+    pub fn check_column_read(
+        &self,
+        schema: Option<&str>,
+        table: &str,
+        column: &str,
+        source: Option<&str>,
+    ) -> Result<(), Refusal> {
+        match (source, self.masks(schema, table, column)) {
+            (Some(view), Some(column)) => Err(Refusal::MaskedColumnUsed {
+                column,
+                how: MaskedUse::Through(view.to_owned()),
+            }),
+            _ => Ok(()),
+        }
+    }
+
+    /// What the reading of a statement knows of the masked columns.
+    fn masks_known(&self) -> Masks<'_> {
+        let more = self
+            .masked_tables
+            .iter()
+            .map(|(_, name, at)| (name.clone(), &self.masked[*at]));
+        Masks::new(&self.masked, more)
     }
 
     /// Whether `schema` is a schema of the database: one the catalog lists
@@ -354,7 +498,7 @@ impl Hidden {
     /// table made since it was read) is hidden as the person's own names
     /// hide it.
     pub fn check_engine_read(&self, schema: Option<&str>, name: &str) -> Result<(), Refusal> {
-        if !self.hides_anything() {
+        if !self.guards_anything() {
             return Ok(());
         }
         if system_catalog(self.dialect, schema, name) {
@@ -376,7 +520,7 @@ impl Hidden {
     /// system catalog, or a listed relation that is hidden; a name the
     /// catalog does not list is passed over.
     pub fn check_name(&self, schema: Option<&str>, name: &str) -> Result<(), Refusal> {
-        if !self.hides_anything() {
+        if !self.guards_anything() {
             return Ok(());
         }
         if system_catalog(self.dialect, schema, name) {
@@ -405,7 +549,7 @@ impl Hidden {
     /// answer, as a common table expression's is: the engine's own check
     /// sees what it reads.
     pub fn check_statement(&self, read: &Read) -> Result<(), Refusal> {
-        if !self.hides_anything() {
+        if !self.guards_anything() {
             return Ok(());
         }
         self.check_settings()?;
@@ -451,7 +595,7 @@ impl Hidden {
             }
             Dialect::MySql | Dialect::Sqlite => {}
         }
-        Ok(())
+        masked::check(read, &self.masks_known())
     }
 
     /// The listed relation `name` of `schema` that is hidden, or else any
@@ -465,7 +609,9 @@ impl Hidden {
         let first = matching.clone().next()?;
         Some(
             matching
+                .clone()
                 .find(|listed| listed.why.is_some())
+                .or_else(|| matching.find(|listed| listed.masks.is_some()))
                 .unwrap_or(first),
         )
     }
@@ -491,10 +637,17 @@ fn missing(dialect: Dialect, catalog: &Catalog, tables: &Tables) -> Vec<String>
         .collect()
 }
 
-/// Refuses `found` where it is hidden, by the names the catalog writes.
+/// Refuses `found` where it is hidden, by the names the catalog writes,
+/// or where it is a view that reads a masked column.
 fn refused(found: &Listed) -> Result<(), Refusal> {
     match &found.why {
-        None => Ok(()),
+        None => match &found.masks {
+            Some(column) => Err(Refusal::MaskedColumnUsed {
+                column: column.clone(),
+                how: MaskedUse::Through(found.name.clone()),
+            }),
+            None => Ok(()),
+        },
         Some(Why::Named) => Err(Refusal::TableHidden {
             name: found.name.clone(),
             through: None,
@@ -506,6 +659,106 @@ fn refused(found: &Listed) -> Result<(), Refusal> {
     }
 }
 
+/// The tables that have a masked column, by schema and name, with an
+/// entry's place in `masked`, once for each entry: each relation an entry
+/// names, a view too, and the partitions and the child tables of those
+/// tables, at any depth.
+fn masked_tables(
+    catalog: &Catalog,
+    depends: &[Vec<usize>],
+    masked: &[ColumnName],
+) -> Vec<(String, String, usize)> {
+    let mut found = Vec::new();
+    for (entry_at, entry) in masked.iter().enumerate() {
+        let mut has: Vec<bool> = catalog
+            .relations
+            .iter()
+            .map(|relation| entry.names_table(Some(&relation.schema), &relation.name))
+            .collect();
+        // A child's parent is what it depends on: taken from it until
+        // nothing more is, which a cycle cannot keep from ending.
+        loop {
+            let mut more = false;
+            for at in 0..catalog.relations.len() {
+                if !has[at] && !catalog.relations[at].view && depends[at].iter().any(|&on| has[on])
+                {
+                    has[at] = true;
+                    more = true;
+                }
+            }
+            if !more {
+                break;
+            }
+        }
+        for (relation, _) in catalog.relations.iter().zip(has).filter(|(_, has)| *has) {
+            found.push((relation.schema.clone(), relation.name.clone(), entry_at));
+        }
+    }
+    found
+}
+
+/// For each relation, the place in `masked` of the masked column it reads,
+/// where it is a view that reads one: by what the catalog says it reads
+/// column by column, or, where it does not say, by a table with a masked
+/// column that it reads. A view over such a view reads it too.
+fn masked_views(
+    catalog: &Catalog,
+    depends: &[Vec<usize>],
+    masked: &[ColumnName],
+    tables: &[(String, String, usize)],
+) -> Vec<Option<usize>> {
+    // An entry of the table `name` of `schema`, of the column `column`
+    // where one is given.
+    let entry_of = |schema: &str, name: &str, column: Option<&str>| {
+        tables
+            .iter()
+            .filter(|(own, table, _)| same(own, schema) && same(table, name))
+            .map(|(_, _, at)| *at)
+            .find(|&at| column.is_none_or(|column| same(&masked[at].column, column)))
+    };
+    let mut found: Vec<Option<usize>> = catalog
+        .relations
+        .iter()
+        .enumerate()
+        .map(|(at, relation)| {
+            if !relation.view {
+                return None;
+            }
+            match &catalog.column_reads {
+                Some(reads) => reads
+                    .iter()
+                    .filter(|read| {
+                        same(&read.schema, &relation.schema) && same(&read.view, &relation.name)
+                    })
+                    .find_map(|read| {
+                        let column = (read.column != "*").then_some(read.column.as_str());
+                        entry_of(&read.table_schema, &read.table, column)
+                    }),
+                None => depends[at].iter().find_map(|&on| {
+                    let on = &catalog.relations[on];
+                    entry_of(&on.schema, &on.name, None)
+                }),
+            }
+        })
+        .collect();
+    loop {
+        let mut more = false;
+        for at in 0..catalog.relations.len() {
+            if found[at].is_some() || !catalog.relations[at].view {
+                continue;
+            }
+            if let Some(entry) = depends[at].iter().find_map(|&on| found[on]) {
+                found[at] = Some(entry);
+                more = true;
+            }
+        }
+        if !more {
+            break;
+        }
+    }
+    found
+}
+
 /// Whether the relation at `at` is hidden, and why, worked out once: by
 /// its own name, else by the first hidden relation it depends on. A
 /// relation met again while its own answer is being worked out (a cycle
@@ -659,6 +912,25 @@ struct Names {
     /// Every name with a dot before it: a column, or on PostgreSQL a
     /// function called as a column of a row.
     after_dot: Vec<String>,
+    /// Each table's place, every time one is read, by where its pieces
+    /// are: for the masked columns, whose reading needs to know which
+    /// words name a table.
+    places: Vec<Place>,
+}
+
+/// A relation's name in a table's place, by where it stands among the
+/// pieces it was read from.
+#[derive(Debug, PartialEq, Eq)]
+struct Place {
+    name: Name,
+    /// The pieces of the name, its schema's with it.
+    from: usize,
+    to: usize,
+    /// The piece of its alias, where it is given one.
+    alias: Option<usize>,
+    /// Whether a list of column names follows the alias, which gives the
+    /// relation's columns other names: `orders AS o(a, b, c)`.
+    renames: bool,
 }
 
 /// A token as [`names`] reads it.
@@ -673,6 +945,10 @@ enum Piece {
     Open,
     Close,
     Comma,
+    /// `*`, which selects or multiplies.
+    Star,
+    /// A number, as written: an `ORDER BY` of one sorts by a place.
+    Number(String),
     Other,
 }
 
@@ -728,7 +1004,11 @@ const JOINS: &[&str] = &["JOIN", "STRAIGHT_JOIN"];
 /// tree is not walked). It errs toward finding: a name that is a column's
 /// in a table's place is checked as a table's too.
 fn names(dialect: Dialect, text: &str) -> Names {
-    let pieces = pieces(dialect, text);
+    read_names(&pieces(dialect, text))
+}
+
+/// [`names`], of the pieces already read.
+fn read_names(pieces: &[Piece]) -> Names {
     let mut found = Names::default();
     // Depth of parentheses, and the depths at which a `FROM`'s list is
     // being read.
@@ -755,7 +1035,7 @@ fn names(dialect: Dialect, text: &str) -> Names {
                 });
                 if table_goes && !query {
                     from_at.push(depth);
-                    table_place(&pieces, at + 1, &mut found);
+                    table_place(pieces, at + 1, &mut found);
                 }
             }
             Piece::Close => {
@@ -763,20 +1043,20 @@ fn names(dialect: Dialect, text: &str) -> Names {
                 depth = depth.saturating_sub(1);
             }
             Piece::Comma if from_at.last() == Some(&depth) => {
-                table_place(&pieces, at + 1, &mut found);
+                table_place(pieces, at + 1, &mut found);
             }
             Piece::Word { .. } if piece.is("FROM") => {
                 from_at.retain(|&level| level < depth);
                 from_at.push(depth);
-                table_place(&pieces, at + 1, &mut found);
+                table_place(pieces, at + 1, &mut found);
             }
             Piece::Word { .. } if JOINS.iter().any(|join| piece.is(join)) => {
-                table_place(&pieces, at + 1, &mut found);
+                table_place(pieces, at + 1, &mut found);
             }
             Piece::Word { .. } if ENDS_FROM.iter().any(|end| piece.is(end)) => {
                 from_at.retain(|&level| level < depth);
             }
-            Piece::Word { .. } if piece.is("WITH") => defined(&pieces, at + 1, &mut found),
+            Piece::Word { .. } if piece.is("WITH") => defined(pieces, at + 1, &mut found),
             _ => {}
         }
         table_goes = places_table || (table_goes && matches!(piece, Piece::Open));
@@ -790,7 +1070,7 @@ fn names(dialect: Dialect, text: &str) -> Names {
             found.after_dot.push(name.to_owned());
         }
         // A word with a parenthesis after it is a call, wherever it is.
-        if let Some((name, after)) = dotted(&pieces, at, Piece::word)
+        if let Some((name, after)) = dotted(pieces, at, Piece::word)
             && pieces.get(after) == Some(&Piece::Open)
             && !found.functions.contains(&name)
         {
@@ -821,6 +1101,22 @@ fn table_place(pieces: &[Piece], mut at: usize, found: &mut Names) {
     if pieces.get(after) == Some(&Piece::Open) {
         return;
     }
+    // Its alias: after `AS` any word, without it a name only.
+    let (alias, next) = match pieces.get(after) {
+        Some(piece) if piece.is("AS") => match pieces.get(after + 1).and_then(Piece::word) {
+            Some(_) => (Some(after + 1), after + 2),
+            None => (None, after + 1),
+        },
+        Some(piece) if piece.name().is_some() => (Some(after), after + 1),
+        _ => (None, after),
+    };
+    found.places.push(Place {
+        name: name.clone(),
+        from: at,
+        to: after,
+        alias,
+        renames: alias.is_some() && pieces.get(next) == Some(&Piece::Open),
+    });
     if !found.relations.contains(&name) {
         found.relations.push(name);
     }
@@ -931,8 +1227,9 @@ fn pieces(dialect: Dialect, text: &str) -> Vec<Piece> {
                     "," => Piece::Comma,
                     _ => Piece::Other,
                 },
+                TokenKind::Operator if written == "*" => Piece::Star,
+                TokenKind::Number => Piece::Number(written.to_owned()),
                 TokenKind::String
-                | TokenKind::Number
                 | TokenKind::ExecutableComment
                 | TokenKind::Operator
                 | TokenKind::Semicolon => Piece::Other,
diff --git a/crates/tabletist-db/src/agent/hidden/masked.rs b/crates/tabletist-db/src/agent/hidden/masked.rs
index a995d07..859db99 100644
--- a/crates/tabletist-db/src/agent/hidden/masked.rs
+++ b/crates/tabletist-db/src/agent/hidden/masked.rs
@@ -9,6 +9,507 @@
 //! is answered. That errs toward refusing. What the reading misses is each
 //! engine's to stop.
 
+use std::collections::{HashMap, HashSet};
+
+use super::{Piece, Read, Refusal, closing, fold, parts, pieces, read_names, same};
+use crate::agent::MaskedUse;
+
+/// A column the person masks, as they name it: in its table, and in its
+/// schema where they give one. One without a schema is that column of the
+/// table of that name in every schema.
+#[derive(Debug, Clone, PartialEq, Eq)]
+pub struct ColumnName {
+    pub schema: Option<String>,
+    pub table: String,
+    pub column: String,
+}
+
+/// As the person writes it: `orders.email`, or `sales.orders.email`, a
+/// part in double quotes where it holds a dot or a quote, as
+/// [`super::TableName`]'s.
+impl std::fmt::Display for ColumnName {
+    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
+        let part = |part: &str| match part.contains(['.', '"']) {
+            true => format!("\"{}\"", part.replace('"', "\"\"")),
+            false => part.to_owned(),
+        };
+        if let Some(schema) = &self.schema {
+            write!(f, "{}.", part(schema))?;
+        }
+        write!(f, "{}.{}", part(&self.table), part(&self.column))
+    }
+}
+
+/// An entry as the person writes it, as `Display` writes it back. The
+/// error says what is wrong without the entry's words.
+impl std::str::FromStr for ColumnName {
+    type Err = String;
+
+    fn from_str(written: &str) -> Result<Self, String> {
+        let mut parts = parts(written)?;
+        let (Some(column), Some(table)) = (parts.pop(), parts.pop()) else {
+            return Err("has no table: write the table, a dot, and the column".into());
+        };
+        let schema = parts.pop();
+        if !parts.is_empty() {
+            return Err(
+                "has more than two dots: it is a table and a column, or a schema, a table and a \
+                 column"
+                    .into(),
+            );
+        }
+        Ok(Self {
+            schema,
+            table,
+            column,
+        })
+    }
+}
+
+impl ColumnName {
+    /// Whether this names the column `column` of the relation `table` of
+    /// `schema`.
+    pub(crate) fn names(&self, schema: Option<&str>, table: &str, column: &str) -> bool {
+        same(&self.column, column) && self.names_table(schema, table)
+    }
+
+    /// Whether this is a column of the relation `table` of `schema`.
+    pub(crate) fn names_table(&self, schema: Option<&str>, table: &str) -> bool {
+        same(&self.table, table)
+            && match (&self.schema, schema) {
+                (Some(own), Some(schema)) => same(own, schema),
+                _ => true,
+            }
+    }
+}
+
+/// What the reading knows of the masked columns: their names, and the
+/// names of the tables that have one, their parts and their child tables
+/// with them, each with the entry a refusal names.
+pub(super) struct Masks<'a> {
+    columns: HashMap<String, &'a ColumnName>,
+    tables: HashMap<String, &'a ColumnName>,
+}
+
+impl<'a> Masks<'a> {
+    /// `masked`, and `more`: the other tables that have one of its
+    /// columns, by name, each with its entry.
+    pub(super) fn new(
+        masked: &'a [ColumnName],
+        more: impl IntoIterator<Item = (String, &'a ColumnName)>,
+    ) -> Self {
+        let mut columns = HashMap::new();
+        let mut tables = HashMap::new();
+        for entry in masked {
+            columns.entry(fold(&entry.column)).or_insert(entry);
+            tables.entry(fold(&entry.table)).or_insert(entry);
+        }
+        for (table, entry) in more {
+            tables.entry(fold(&table)).or_insert(entry);
+        }
+        Self { columns, tables }
+    }
+
+    fn column(&self, word: &str) -> Option<&'a ColumnName> {
+        self.columns.get(&fold(word)).copied()
+    }
+
+    pub(super) fn table(&self, word: &str) -> Option<&'a ColumnName> {
+        self.tables.get(&fold(word)).copied()
+    }
+
+    /// The entry a refusal names when the statement names no table of
+    /// one.
+    fn any(&self) -> Option<&'a ColumnName> {
+        self.columns.values().next().copied()
+    }
+}
+
+/// The refusal for a use `how` of the masked column `entry`.
+fn used(entry: &ColumnName, how: MaskedUse) -> Refusal {
+    Refusal::MaskedColumnUsed {
+        column: entry.to_string(),
+        how,
+    }
+}
+
+/// The words that end the outermost `SELECT`'s list at its depth.
+const ENDS_LIST: &[&str] = &[
+    "FROM",
+    "INTO",
+    "WHERE",
+    "GROUP",
+    "HAVING",
+    "WINDOW",
+    "QUALIFY",
+    "ORDER",
+    "LIMIT",
+    "OFFSET",
+    "FETCH",
+    "UNION",
+    "INTERSECT",
+    "EXCEPT",
+    "MINUS",
+    "FOR",
+    "LOCK",
+];
+
+/// The words between `SELECT` and its list that change nothing of what it
+/// selects: `ALL`, and MySQL's hints.
+const MODIFIERS: &[&str] = &[
+    "ALL",
+    "HIGH_PRIORITY",
+    "STRAIGHT_JOIN",
+    "SQL_SMALL_RESULT",
+    "SQL_BIG_RESULT",
+    "SQL_BUFFER_RESULT",
+    "SQL_NO_CACHE",
+    "SQL_CACHE",
+    "SQL_CALC_FOUND_ROWS",
+];
+
+/// The clauses an outermost query may not have beside a masked column,
+/// by the word each begins with, and as a refusal names it.
+const CLAUSES: &[(&str, &str)] = &[
+    ("GROUP", "GROUP BY"),
+    ("HAVING", "HAVING"),
+    ("WINDOW", "WINDOW"),
+    ("QUALIFY", "QUALIFY"),
+    ("OVER", "a window function"),
+    ("UNION", "UNION, INTERSECT or EXCEPT"),
+    ("INTERSECT", "UNION, INTERSECT or EXCEPT"),
+    ("EXCEPT", "UNION, INTERSECT or EXCEPT"),
+    ("MINUS", "UNION, INTERSECT or EXCEPT"),
+];
+
+/// What stands in the outermost list.
+enum Item<'p> {
+    /// A name by itself: its parts joined by dots, the last the column's,
+    /// and the alias it is given.
+    Bare {
+        parts: usize,
+        column: &'p str,
+        alias: Option<&'p str>,
+    },
+    /// `*`, or `q.*` with its qualifier.
+    Star { qualifier: Option<&'p str> },
+    /// Anything else.
+    Other,
+}
+
+/// Reads one item of a list for what it is.
+fn item(slice: &[Piece]) -> Item<'_> {
+    if let [rest @ .., Piece::Star] = slice {
+        return match rest {
+            [] => Item::Star { qualifier: None },
+            [name @ .., Piece::Dot] if dotted_only(name) => Item::Star {
+                qualifier: name.last().and_then(Piece::word),
+            },
+            _ => Item::Other,
+        };
+    }
+    let mut at = 0;
+    let mut parts = 0;
+    let mut column = None;
+    while let Some(word) = slice.get(at).and_then(Piece::word) {
+        column = Some(word);
+        parts += 1;
+        at += 1;
+        if slice.get(at) != Some(&Piece::Dot) {
+            break;
+        }
+        at += 1;
+    }
+    let Some(column) = column else {
+        return Item::Other;
+    };
+    if slice.get(at - 1) == Some(&Piece::Dot) {
+        return Item::Other;
+    }
+    // An alias: after `AS` any word, without it a name only.
+    let alias = match &slice[at..] {
+        [] => None,
+        [as_, alias] if as_.is("AS") => match alias.word() {
+            Some(alias) => Some(alias),
+            None => return Item::Other,
+        },
+        [alias] => match alias.name() {
+            Some(alias) => Some(alias),
+            None => return Item::Other,
+        },
+        _ => return Item::Other,
+    };
+    Item::Bare {
+        parts,
+        column,
+        alias,
+    }
+}
+
+/// Whether `slice` is a name and nothing else: words joined by dots.
+fn dotted_only(slice: &[Piece]) -> bool {
+    slice.len() % 2 == 1
+        && slice.iter().enumerate().all(|(at, piece)| match at % 2 {
+            0 => piece.word().is_some(),
+            _ => *piece == Piece::Dot,
+        })
+}
+
+/// `pieces[from..to]` split by its commas at the depth of `from`.
+fn split(pieces: &[Piece], depth: &[usize], from: usize, to: usize) -> Vec<(usize, usize)> {
+    let mut items = Vec::new();
+    let mut start = from;
+    for at in from..to {
+        if pieces[at] == Piece::Comma && depth[at] == depth[from] {
+            items.push((start, at));
+            start = at + 1;
+        }
+    }
+    if start < to {
+        items.push((start, to));
+    }
+    items
+}
+
+/// The depth of each piece: a parenthesis is at the depth it opens from,
+/// and what it holds one deeper.
+fn depths(pieces: &[Piece]) -> Vec<usize> {
+    let mut depth = Vec::with_capacity(pieces.len());
+    let mut level = 0_usize;
+    for piece in pieces {
+        if *piece == Piece::Close {
+            level = level.saturating_sub(1);
+        }
+        depth.push(level);
+        if *piece == Piece::Open {
+            level += 1;
+        }
+    }
+    depth
+}
+
+/// Checks `read` against the masked columns `masks` knows: refused where
+/// a word that names one stands anywhere but bare in the outermost
+/// `SELECT`'s list, and where a statement over a table that has one reads
+/// it some other way (B.4's rules 1 to 4).
+pub(super) fn check(read: &Read, masks: &Masks<'_>) -> Result<(), Refusal> {
+    let Some(any) = masks.any() else {
+        return Ok(());
+    };
+    // A name written with escapes (`U&"…"`), which the reading by names
+    // would not see, is refused before this is asked (`check_read`).
+    let pieces = pieces(read.dialect(), read.text());
+    let depth = depths(&pieces);
+    let names = read_names(&pieces);
+    // The first table the statement reads that has a masked column, the
+    // names that stand for those tables (their own and their aliases), and
+    // the pieces of every table's place, which name tables, not columns.
+    let mut over: Option<&ColumnName> = None;
+    let mut standing: HashMap<String, &ColumnName> = HashMap::new();
+    let mut in_place: HashSet<usize> = HashSet::new();
+    for place in &names.places {
+        in_place.extend(place.from..place.to);
+        in_place.extend(place.alias);
+        let Some(entry) = masks.table(&place.name.name) else {
+            continue;
+        };
+        over.get_or_insert(entry);
+        standing.insert(fold(&place.name.name), entry);
+        let alias = place.alias.and_then(|at| pieces[at].word());
+        if let Some(alias) = alias {
+            standing.insert(fold(alias), entry);
+        }
+        // `orders AS o(a, b, c)` gives its columns other names.
+        if place.renames {
+            let written = alias.unwrap_or(&place.name.name);
+            return Err(used(entry, MaskedUse::Renamed(written.to_owned())));
+        }
+    }
+    // The outermost `SELECT` and its list: what stands bare there is
+    // allowed, and is what a plain query must keep to.
+    let mut allowed: HashSet<usize> = HashSet::new();
+    let mut masked_items: Vec<Option<&str>> = Vec::new();
+    let mut distinct = false;
+    let top = (0..pieces.len()).find(|&at| depth[at] == 0 && pieces[at].is("SELECT"));
+    let mut list_end = pieces.len();
+    if let Some(top) = top {
+        let mut start = top + 1;
+        while let Some(piece) = pieces.get(start) {
+            if piece.is("DISTINCT") || piece.is("DISTINCTROW") {
+                distinct = true;
+                start += 1;
+                // PostgreSQL's `DISTINCT ON (…)`.
+                if pieces.get(start).is_some_and(|on| on.is("ON"))
+                    && pieces.get(start + 1) == Some(&Piece::Open)
+                {
+                    start = closing(&pieces, start + 1) + 1;
+                }
+            } else if MODIFIERS.iter().any(|word| piece.is(word)) {
+                start += 1;
+            } else {
+                break;
+            }
+        }
+        list_end = (start..pieces.len())
+            .find(|&at| depth[at] == 0 && ENDS_LIST.iter().any(|word| pieces[at].is(word)))
+            .unwrap_or(pieces.len());
+        for (from, to) in split(&pieces, &depth, start, list_end) {
+            match item(&pieces[from..to]) {
+                Item::Bare {
+                    parts,
+                    column,
+                    alias,
+                } => {
+                    if parts == 1
+                        && let Some(entry) = standing.get(&fold(column))
+                    {
+                        return Err(used(entry, MaskedUse::Row(column.to_owned())));
+                    }
+                    allowed.extend(from..to);
+                    let masked = masks.column(column).is_some()
+                        || alias.is_some_and(|alias| masks.column(alias).is_some());
+                    if masked {
+                        masked_items.push(alias);
+                    }
+                }
+                Item::Star { qualifier } => {
+                    allowed.extend(from..to);
+                    let masked = match qualifier {
+                        Some(qualifier) => standing.contains_key(&fold(qualifier)),
+                        None => over.is_some(),
+                    };
+                    if masked {
+                        masked_items.push(None);
+                    }
+                }
+                Item::Other => {}
+            }
+        }
+    }
+    // `TABLE orders` is refused before this is asked (`check_read`).
+    // Rule 2: beside a masked column, the outermost query is a plain one.
+    if !masked_items.is_empty() {
+        let entry = over.unwrap_or(any);
+        if distinct {
+            return Err(used(entry, MaskedUse::Clause("DISTINCT")));
+        }
+        for at in (0..pieces.len()).filter(|&at| depth[at] == 0) {
+            if let Some((_, clause)) = CLAUSES.iter().find(|(word, _)| pieces[at].is(word)) {
+                return Err(used(entry, MaskedUse::Clause(clause)));
+            }
+        }
+        // `ORDER BY` a place, which may be a masked column's: SQLite reads
+        // `(2)` and `+2` as the place too.
+        let order = (list_end..pieces.len()).find(|&at| {
+            depth[at] == 0
+                && pieces[at].is("ORDER")
+                && pieces.get(at + 1).is_some_and(|by| by.is("BY"))
+        });
+        if let Some(order) = order {
+            let end = (order + 2..pieces.len())
+                .find(|&at| {
+                    depth[at] == 0
+                        && ["LIMIT", "OFFSET", "FETCH", "FOR", "LOCK"]
+                            .iter()
+                            .any(|word| pieces[at].is(word))
+                })
+                .unwrap_or(pieces.len());
+            for (from, to) in split(&pieces, &depth, order + 2, end) {
+                let place = pieces[from..to]
+                    .iter()
+                    .find(|piece| !matches!(piece, Piece::Open | Piece::Other));
+                if let Some(Piece::Number(written)) = place {
+                    return Err(used(entry, MaskedUse::Sorted(written.clone())));
+                }
+            }
+        }
+    }
+    // A masked column's alias stands for it: SQLite reads one in a
+    // `WHERE`, and SQLite and MySQL in an `ORDER BY`'s expressions.
+    let aliases: Vec<&str> = masked_items.iter().flatten().copied().collect();
+    // Rules 3 and 4, everywhere else.
+    for (at, piece) in pieces.iter().enumerate() {
+        if allowed.contains(&at) || in_place.contains(&at) {
+            continue;
+        }
+        match piece {
+            Piece::Word { text, .. } => {
+                if let Some(entry) = masks.column(text) {
+                    return Err(used(entry, MaskedUse::Named(entry.column.clone())));
+                }
+                if let Some(alias) = aliases.iter().find(|alias| same(alias, text)) {
+                    let entry = over.unwrap_or(any);
+                    return Err(used(entry, MaskedUse::Alias((*alias).to_owned())));
+                }
+                if piece.is("NATURAL")
+                    && let Some(entry) = over
+                {
+                    return Err(used(entry, MaskedUse::Natural));
+                }
+                // The table or its alias as a value: its whole row.
+                let qualifies = pieces.get(at + 1) == Some(&Piece::Dot);
+                let after_dot = at > 0 && pieces[at - 1] == Piece::Dot;
+                if !qualifies
+                    && !after_dot
+                    && let Some(entry) = standing.get(&fold(text))
+                {
+                    return Err(used(entry, MaskedUse::Row(text.clone())));
+                }
+            }
+            Piece::Star if selects(&pieces, at) => {
+                let qualifier = (at >= 2 && pieces[at - 1] == Piece::Dot)
+                    .then(|| pieces[at - 2].word())
+                    .flatten();
+                match qualifier {
+                    Some(qualifier) => {
+                        if let Some(entry) = standing.get(&fold(qualifier)) {
+                            let row = format!("{qualifier}.*");
+                            return Err(used(entry, MaskedUse::Row(row)));
+                        }
+                    }
+                    None => {
+                        if let Some(entry) = over {
+                            return Err(used(entry, MaskedUse::Star));
+                        }
+                    }
+                }
+            }
+            _ => {}
+        }
+    }
+    Ok(())
+}
+
+/// Whether the star at `at` selects columns, as in a list (`SELECT *`,
+/// `, *`, `q.*`, `DISTINCT ON (…) *`), and is not `count(*)` or a product.
+fn selects(pieces: &[Piece], at: usize) -> bool {
+    let Some(before) = at.checked_sub(1).map(|before| &pieces[before]) else {
+        return false;
+    };
+    match before {
+        Piece::Comma | Piece::Dot => true,
+        Piece::Word { .. } => ["SELECT", "DISTINCT", "DISTINCTROW"]
+            .iter()
+            .chain(MODIFIERS)
+            .any(|word| before.is(word)),
+        Piece::Close => {
+            let mut depth = 0_usize;
+            let open = (0..at).rev().find(|&back| {
+                match pieces[back] {
+                    Piece::Close => depth += 1,
+                    Piece::Open => depth = depth.saturating_sub(1),
+                    _ => {}
+                }
+                depth == 0
+            });
+            open.is_some_and(|open| {
+                open >= 2 && pieces[open - 1].is("ON") && pieces[open - 2].is("DISTINCT")
+            })
+        }
+        _ => false,
+    }
+}
+
 #[cfg(test)]
 mod tests {
     use super::*;
diff --git a/crates/tabletist-db/src/mysql/hidden.rs b/crates/tabletist-db/src/mysql/hidden.rs
index f93310a..6340ced 100644
--- a/crates/tabletist-db/src/mysql/hidden.rs
+++ b/crates/tabletist-db/src/mysql/hidden.rs
@@ -234,6 +234,7 @@ pub(super) async fn catalog(conn: &mut mysql_async::Conn) -> Result<Catalog> {
                 name,
                 depends_on,
                 part: false,
+                view,
             }
         })
         .collect();
@@ -247,8 +248,7 @@ pub(super) async fn catalog(conn: &mut mysql_async::Conn) -> Result<Catalog> {
     Ok(Catalog {
         relations,
         routines,
-        unnamed_calls: Vec::new(),
-        table_functions: Vec::new(),
+        ..Catalog::default()
     })
 }
 
diff --git a/crates/tabletist-db/src/pg/hidden.rs b/crates/tabletist-db/src/pg/hidden.rs
index f6700f9..5080a4c 100644
--- a/crates/tabletist-db/src/pg/hidden.rs
+++ b/crates/tabletist-db/src/pg/hidden.rs
@@ -71,7 +71,8 @@ SELECT n.nspname, c.relname, dn.nspname, dc.relname, c.oid IN (SELECT relation F
         || coalesce((
             SELECT string_agg(pg_get_expr(pol.polqual, pol.polrelid), ' ')
             FROM pg_policy pol WHERE pol.polrelid = c.oid
-        ), '')
+        ), ''),
+    c.relkind IN ('v', 'm')
 FROM pg_class c
 JOIN pg_namespace n ON n.oid = c.relnamespace
 LEFT JOIN depends ON depends.relation = c.oid
@@ -205,6 +206,7 @@ pub(super) async fn catalog(client: &tokio_postgres::Client) -> Result<Catalog>
                 opaque: row.get(4) != Some("f")
                     || calls_a_refused_function(Dialect::Postgres, row.get(6).unwrap_or("")),
                 part: row.get(5) == Some("t"),
+                view: row.get(7) == Some("t"),
             });
         }
         if let (Some(on_schema), Some(on_name), Some(last)) =
@@ -228,7 +230,7 @@ pub(super) async fn catalog(client: &tokio_postgres::Client) -> Result<Catalog>
         relations,
         routines,
         unnamed_calls,
-        table_functions: Vec::new(),
+        ..Catalog::default()
     })
 }
 
diff --git a/crates/tabletist-db/src/sqlite.rs b/crates/tabletist-db/src/sqlite.rs
index 67ee7de..9e03081 100644
--- a/crates/tabletist-db/src/sqlite.rs
+++ b/crates/tabletist-db/src/sqlite.rs
@@ -1243,6 +1243,7 @@ fn agents_catalog(connection: &rusqlite::Connection, fences: &Fences) -> Result<
                 depends_on: Vec::new(),
                 opaque: false,
                 part,
+                view,
             });
         }
         if let (Some(owner), Some(last)) = (owner, relations.last_mut()) {
````

- [ ] **Step 4: Run the tests**

Run: `cargo test --locked -p tabletist-db --lib -- masked hidden`

Expected: PASS.

```text
test result: ok. 26 passed; 0 failed; 0 ignored; 0 measured; 467 filtered out
```

- [ ] **Step 5: Commit**

```bash
cargo fmt --all
cargo clippy --locked --workspace --all-targets -- -D warnings
git add crates/tabletist-db/src/agent.rs crates/tabletist-db/src/agent/hidden.rs crates/tabletist-db/src/agent/hidden/masked.rs crates/tabletist-db/src/pg/hidden.rs crates/tabletist-db/src/mysql/hidden.rs crates/tabletist-db/src/sqlite.rs crates/tabletist-db/tests/agent.rs
git commit -m "Read a masked column's uses off an agent's statement, and what the catalog says of it"
```

---

### Task 2: Each driver checks the masked columns

- `Connection::run_agent_read` and the adapters' `run_read` take `masked: &[agent::ColumnName]` after `tables`, and `Connection::agent_hidden` and `agents_catalog` take it too. The catalog is read, the statement checked and the plan asked for where anything is hidden or masked. The app and the MCP crate's harness pass none until run 2.
- PostgreSQL reads the columns of the entries' tables, what each view and materialized view reads of the masked columns' names (`pg_depend`), and a whole row a view reads from its definition as the server writes it back (`x.*`), which `pg_depend` keeps no column for.
- MySQL reads the columns; its catalog says what a view reads table by table, so a view over a table with a masked column is refused.
- SQLite reads the columns (`pragma_table_xinfo`) and records the column of each read as a view is prepared, so it says what a view reads column by column; its authorizer refuses a masked column read for a view or a common table expression.

**Files:**
- Modify: `crates/tabletist-db/src/lib.rs`, `crates/tabletist-db/src/adapter.rs`, `crates/tabletist-db/src/pg.rs`, `crates/tabletist-db/src/pg/hidden.rs`, `crates/tabletist-db/src/pg/script.rs`, `crates/tabletist-db/src/mysql.rs`, `crates/tabletist-db/src/mysql/hidden.rs`, `crates/tabletist-db/src/mysql/script.rs`, `crates/tabletist-db/src/sqlite.rs`, `crates/tabletist-db/src/sqlite/fence.rs`, `crates/tabletist-db/src/agent.rs`, `crates/tabletist-db/src/agent/hidden.rs`, `crates/tabletist-db/src/agent/hidden/masked.rs`, `crates/tabletist-db/tests/agent.rs`, `crates/tabletist-mcp/tests/support/mod.rs`, `src/backend/agents.rs`

- [ ] **Step 1: Write the failing test**

````diff
diff --git a/crates/tabletist-db/src/agent/hidden/masked.rs b/crates/tabletist-db/src/agent/hidden/masked.rs
index 859db99..eb1c857 100644
--- a/crates/tabletist-db/src/agent/hidden/masked.rs
+++ b/crates/tabletist-db/src/agent/hidden/masked.rs
@@ -745,4 +745,15 @@ mod tests {
             alias()
         );
     }
+
+    #[test]
+    fn a_view_reads_a_whole_row_where_its_definition_writes_one() {
+        let masked = [column("orders.email")];
+        let masks = Masks::new(&masked, []);
+        let row = |definition: &str| reads_whole_row(Dialect::Postgres, definition, &masks);
+        assert!(row(" SELECT x.*::public.orders AS x\n   FROM orders x;"));
+        assert!(row(" SELECT to_jsonb(orders.*) AS j\n   FROM orders;"));
+        assert!(!row(" SELECT x.id,\n    x.email\n   FROM orders x;"));
+        assert!(!row(" SELECT c.* FROM customers c;"));
+    }
 }
diff --git a/crates/tabletist-db/src/mysql/script.rs b/crates/tabletist-db/src/mysql/script.rs
index af9dc21..c4b96e8 100644
--- a/crates/tabletist-db/src/mysql/script.rs
+++ b/crates/tabletist-db/src/mysql/script.rs
@@ -684,7 +684,7 @@ mod tests {
             send(&conn.server_version());
         };
         let _agent = |conn: &Conn, read: &Read, limits: Limits, stop: &StopFlag| {
-            send(&conn.run_read(read, limits, &crate::agent::Tables::All, stop));
+            send(&conn.run_read(read, limits, &crate::agent::Tables::All, &[], stop));
         };
     }
 
diff --git a/crates/tabletist-db/src/pg/script.rs b/crates/tabletist-db/src/pg/script.rs
index 40f004d..166bf81 100644
--- a/crates/tabletist-db/src/pg/script.rs
+++ b/crates/tabletist-db/src/pg/script.rs
@@ -708,7 +708,7 @@ mod tests {
             send(&conn.server_version());
         };
         let _agent = |conn: &Conn, read: &Read, limits: Limits, stop: &StopFlag| {
-            send(&conn.run_read(read, limits, &crate::agent::Tables::All, stop));
+            send(&conn.run_read(read, limits, &crate::agent::Tables::All, &[], stop));
         };
     }
 
diff --git a/crates/tabletist-db/src/sqlite.rs b/crates/tabletist-db/src/sqlite.rs
index 9e03081..d1de250 100644
--- a/crates/tabletist-db/src/sqlite.rs
+++ b/crates/tabletist-db/src/sqlite.rs
@@ -2292,7 +2292,7 @@ mod tests {
             let (conn, _dir) = virtual_tables().await;
             let read = crate::agent::check_read(Dialect::Sqlite, sql, &[]).unwrap();
             let outcome = conn
-                .run_read(&read, limits, &only(table), &StopFlag::new())
+                .run_read(&read, limits, &only(table), &[], &StopFlag::new())
                 .await;
             let answer =
                 outcome.and_then(|outcome| crate::agent::answer(outcome, &StopFlag::new()));
@@ -2310,7 +2310,9 @@ mod tests {
         }]);
         let read =
             crate::agent::check_read(Dialect::Sqlite, "SELECT * FROM notes_content", &[]).unwrap();
-        let outcome = conn.run_read(&read, limits, &notes, &StopFlag::new()).await;
+        let outcome = conn
+            .run_read(&read, limits, &notes, &[], &StopFlag::new())
+            .await;
         assert_eq!(
             outcome.map(|_| ()),
             Err(Error::Agent(crate::agent::Refusal::TableHidden {
@@ -3131,9 +3133,15 @@ mod tests {
             rows: 10,
             timeout: Duration::from_millis(250),
         };
-        conn.run_read(&read, limits, &crate::agent::Tables::All, &StopFlag::new())
-            .await
-            .unwrap();
+        conn.run_read(
+            &read,
+            limits,
+            &crate::agent::Tables::All,
+            &[],
+            &StopFlag::new(),
+        )
+        .await
+        .unwrap();
         assert_eq!(wait(&conn).await, Ok(5_000));
     }
 
diff --git a/crates/tabletist-db/tests/agent.rs b/crates/tabletist-db/tests/agent.rs
index d1cb49a..9b41754 100644
--- a/crates/tabletist-db/tests/agent.rs
+++ b/crates/tabletist-db/tests/agent.rs
@@ -9,7 +9,9 @@
 
 use std::time::{Duration, Instant};
 
-use tabletist_db::agent::{self, Answer, Limits, Param, Refusal, TableName, Tables};
+use tabletist_db::agent::{
+    self, Answer, ColumnName, Limits, MaskedUse, Param, Refusal, TableName, Tables,
+};
 use tabletist_db::{
     Access, ConnectSpec, Connection, Dialect, Error, HostKeys, ScriptMode, Secrets,
     StatementOutcome, StopFlag, Value, sql,
@@ -117,7 +119,7 @@ async fn asked(
     let read = agent::check_read(connection.dialect(), &sql, params)
         .unwrap_or_else(|refusal| panic!("{sql}: {refusal}"));
     connection
-        .run_agent_read(&read, limits, &Tables::All, &StopFlag::new())
+        .run_agent_read(&read, limits, &Tables::All, &[], &StopFlag::new())
         .await
 }
 
@@ -254,7 +256,7 @@ async fn a_read_that_is_stopped_or_fails_says_so(connection: &Connection, on: &s
     stopped.stop();
     let books = agent::check_read(connection.dialect(), "SELECT title FROM books", &[]).unwrap();
     let ended = connection
-        .run_agent_read(&books, LIMITS, &Tables::All, &stopped)
+        .run_agent_read(&books, LIMITS, &Tables::All, &[], &stopped)
         .await;
     assert_eq!(ended.unwrap_err(), Error::Cancelled, "{on}");
     // The database's own error, in its words.
@@ -277,7 +279,7 @@ async fn a_read_checked_for_another_database_is_not_run(connection: &Connection,
     };
     let read = agent::check_read(other, "SELECT title FROM books", &[]).unwrap();
     let error = connection
-        .run_agent_read(&read, LIMITS, &Tables::All, &StopFlag::new())
+        .run_agent_read(&read, LIMITS, &Tables::All, &[], &StopFlag::new())
         .await
         .unwrap_err();
     assert_eq!(
@@ -301,7 +303,7 @@ async fn hiding(connection: &Connection, tables: &Tables, sql: &str) -> Result<A
     let read = agent::check_read(connection.dialect(), sql, &[])
         .unwrap_or_else(|refusal| panic!("{sql}: {refusal}"));
     connection
-        .run_agent_read(&read, LIMITS, tables, &StopFlag::new())
+        .run_agent_read(&read, LIMITS, tables, &[], &StopFlag::new())
         .await
 }
 
@@ -438,7 +440,7 @@ async fn a_hidden_table_is_never_read(connection: &Connection, on: &str) {
             let greeting = "SELECT * FROM shop_greeting";
             let error = hiding(connection, &customers, greeting).await.unwrap_err();
             assert_eq!(error, hidden_refusal("shop_greeting", None), "{on}");
-            let hidden = connection.agent_hidden(&customers).await.unwrap();
+            let hidden = connection.agent_hidden(&customers, &[]).await.unwrap();
             assert!(hidden.hides("bookshop", "shop_greeting"), "{on}");
             let found = hiding(connection, &Tables::All, greeting).await;
             assert_eq!(
@@ -460,7 +462,7 @@ async fn a_hidden_table_is_never_read(connection: &Connection, on: &str) {
         Dialect::Postgres => "public",
         Dialect::MySql => "bookshop",
     };
-    let hidden = connection.agent_hidden(&customers).await.unwrap();
+    let hidden = connection.agent_hidden(&customers, &[]).await.unwrap();
     assert!(hidden.hides(home, "customers"), "{on}");
     assert!(hidden.hides(home, "customer_names"), "{on}");
     assert!(!hidden.hides(home, "books"), "{on}");
@@ -475,7 +477,7 @@ async fn a_hidden_table_is_never_read(connection: &Connection, on: &str) {
         columns: Vec::new(),
     };
     assert_eq!(error, Error::Agent(look.clone()), "{on}");
-    let hidden = connection.agent_hidden(&gone).await.unwrap();
+    let hidden = connection.agent_hidden(&gone, &[]).await.unwrap();
     assert_eq!(hidden.check_settings(), Err(look), "{on}");
     // Nothing hidden: the table reads.
     let found = count(hiding(connection, &Tables::All, "SELECT count(*) FROM customers").await);
@@ -483,6 +485,115 @@ async fn a_hidden_table_is_never_read(connection: &Connection, on: &str) {
     the_bookshop_is_stocked(connection, on).await;
 }
 
+/// What the connection's database answers an agent that sends `sql` on a
+/// connection that masks the columns `masked` names, and hides nothing.
+async fn masking(connection: &Connection, masked: &[&str], sql: &str) -> Result<Answer, Error> {
+    let masked: Vec<ColumnName> = masked.iter().map(|entry| entry.parse().unwrap()).collect();
+    let read = agent::check_read(connection.dialect(), sql, &[])
+        .unwrap_or_else(|refusal| panic!("{sql}: {refusal}"));
+    connection
+        .run_agent_read(&read, LIMITS, &Tables::All, &masked, &StopFlag::new())
+        .await
+}
+
+/// Refused as a use of the masked column `orders.email`.
+fn masked_refusal(how: MaskedUse) -> Error {
+    Error::Agent(Refusal::MaskedColumnUsed {
+        column: "orders.email".into(),
+        how,
+    })
+}
+
+/// The masked-column rows of the plan's F.2 (B.4) that 3a's first run
+/// builds, on the Bookshop: what may not run, and what still does. The
+/// values themselves are not replaced yet.
+async fn a_masked_column_is_used_only_as_it_is(connection: &Connection, on: &str) {
+    let email = ["orders.email"];
+    let count = |answer: Result<Answer, Error>| answer.map(|answer| answer.rows.len());
+    // Selected as it is, the column reads; so does a view that does not
+    // read it, and what has none.
+    for (sql, rows) in [
+        ("SELECT id, email FROM orders WHERE id <= 3 ORDER BY id", 3),
+        ("SELECT * FROM orders WHERE id = 1", 1),
+        ("SELECT count(*) FROM orders", 1),
+        ("SELECT * FROM customer_names", 8),
+    ] {
+        let found = count(masking(connection, &email, sql).await);
+        assert_eq!(found, Ok(rows), "{on}: {sql}");
+    }
+    // Used, by its name, whatever the table.
+    let named = masked_refusal(MaskedUse::Named("email".into()));
+    for sql in [
+        "SELECT id FROM orders WHERE email LIKE 'a%'",
+        "SELECT id FROM orders ORDER BY email",
+        "SELECT count(DISTINCT email) FROM orders",
+        "SELECT id FROM customers WHERE email = 'x'",
+    ] {
+        let found = masking(connection, &email, sql).await.unwrap_err();
+        assert_eq!(found, named, "{on}: {sql}");
+    }
+    assert_eq!(
+        masking(connection, &email, "SELECT DISTINCT email FROM orders")
+            .await
+            .unwrap_err(),
+        masked_refusal(MaskedUse::Clause("DISTINCT")),
+        "{on}"
+    );
+    assert_eq!(
+        masking(
+            connection,
+            &email,
+            "SELECT o.id FROM (SELECT * FROM orders) o"
+        )
+        .await
+        .unwrap_err(),
+        masked_refusal(MaskedUse::Star),
+        "{on}"
+    );
+    // A view that reads it, however little of the view is asked for.
+    let through = masked_refusal(MaskedUse::Through("recent_orders".into()));
+    for sql in [
+        "SELECT * FROM recent_orders",
+        "SELECT count(*) FROM recent_orders",
+    ] {
+        let found = masking(connection, &email, sql).await.unwrap_err();
+        assert_eq!(found, through, "{on}: {sql}");
+    }
+    // The system catalogs are closed, as on a connection that hides.
+    let catalog = match connection.dialect() {
+        Dialect::Postgres => "SELECT count(*) FROM pg_catalog.pg_class",
+        Dialect::MySql => "SELECT count(*) FROM information_schema.TABLES",
+        Dialect::Sqlite => "SELECT count(*) FROM sqlite_master",
+    };
+    assert_eq!(
+        masking(connection, &email, catalog).await.unwrap_err(),
+        Error::Agent(Refusal::SystemCatalog),
+        "{on}"
+    );
+    // A masked column renamed, or dropped, stops agents on the connection.
+    let gone = masking(connection, &["orders.mail"], "SELECT count(*) FROM books")
+        .await
+        .unwrap_err();
+    let look = Refusal::SettingsNeedALook {
+        tables: Vec::new(),
+        columns: vec!["orders.mail".into()],
+    };
+    assert_eq!(gone, Error::Agent(look.clone()), "{on}");
+    let masked: Vec<ColumnName> = vec!["orders.mail".parse().unwrap()];
+    let hidden = connection
+        .agent_hidden(&Tables::All, &masked)
+        .await
+        .unwrap();
+    assert_eq!(hidden.check_settings(), Err(look), "{on}");
+    let masked: Vec<ColumnName> = vec!["ORDERS.EMAIL".parse().unwrap()];
+    let hidden = connection
+        .agent_hidden(&Tables::All, &masked)
+        .await
+        .unwrap();
+    assert_eq!(hidden.check_settings(), Ok(()), "{on}");
+    the_bookshop_is_stocked(connection, on).await;
+}
+
 /// What every engine is asked.
 async fn an_agent_reads_the_bookshop(connection: &Connection, on: &str) {
     a_read_answers_its_rows(connection, on).await;
@@ -548,7 +659,7 @@ mod sqlite {
         )
         .await;
         let customers = Tables::AllExcept(vec![named("customers")]);
-        let hidden = connection.agent_hidden(&customers).await.unwrap();
+        let hidden = connection.agent_hidden(&customers, &[]).await.unwrap();
         for view in ["v_cte", "v_string", "v_in", "v_over"] {
             assert!(hidden.hides("main", view), "{view}");
         }
@@ -561,6 +672,12 @@ mod sqlite {
         a_hidden_table_is_never_read(&connection, "sqlite").await;
     }
 
+    #[tokio::test]
+    async fn a_masked_column_is_used_only_as_it_is_on_sqlite() {
+        let (connection, _dir) = bookshop().await;
+        a_masked_column_is_used_only_as_it_is(&connection, "sqlite").await;
+    }
+
     #[tokio::test]
     async fn sqlite_takes_each_value_as_its_own_type() {
         let (connection, _dir) = bookshop().await;
@@ -841,6 +958,14 @@ mod postgres {
         a_hidden_table_is_never_read(&connection, "postgres").await;
     }
 
+    #[tokio::test]
+    async fn a_masked_column_is_used_only_as_it_is_on_postgres() {
+        let Some(connection) = bookshop().await else {
+            return;
+        };
+        a_masked_column_is_used_only_as_it_is(&connection, "postgres").await;
+    }
+
     #[tokio::test]
     async fn postgres_reads_a_value_as_the_type_of_its_placeholder() {
         let Some(connection) = bookshop().await else {
@@ -873,7 +998,7 @@ mod postgres {
         let twice = "SELECT title FROM books WHERE id = $1 OR author_id = $1 ORDER BY id";
         let read_twice = agent::check_read(Dialect::Postgres, twice, &[Param::Int(3)]).unwrap();
         let found = connection
-            .run_agent_read(&read_twice, LIMITS, &Tables::All, &StopFlag::new())
+            .run_agent_read(&read_twice, LIMITS, &Tables::All, &[], &StopFlag::new())
             .await
             .unwrap();
         assert_eq!(found.rows, [[text("Lanterns")], [text("Ninety-One Doors")]]);
@@ -902,7 +1027,7 @@ mod postgres {
         let sql = "SELECT '$1', $q$ $1 $q$, title FROM books WHERE id = $1";
         let read = agent::check_read(Dialect::Postgres, sql, &[Param::Int(3)]).unwrap();
         let found = connection
-            .run_agent_read(&read, LIMITS, &Tables::All, &StopFlag::new())
+            .run_agent_read(&read, LIMITS, &Tables::All, &[], &StopFlag::new())
             .await
             .unwrap();
         assert_eq!(found.rows, [[text("$1"), text(" $1 "), text("Lanterns")]]);
@@ -1107,6 +1232,14 @@ mod mysql {
         a_hidden_table_is_never_read(&connection, "mysql").await;
     }
 
+    #[tokio::test]
+    async fn a_masked_column_is_used_only_as_it_is_on_mysql() {
+        let Some(connection) = bookshop().await else {
+            return;
+        };
+        a_masked_column_is_used_only_as_it_is(&connection, "mysql").await;
+    }
+
     #[tokio::test]
     async fn mysql_takes_each_value_as_its_own_type() {
         let Some(connection) = bookshop().await else {
diff --git a/crates/tabletist-mcp/tests/support/mod.rs b/crates/tabletist-mcp/tests/support/mod.rs
index 30ec23d..44e7046 100644
--- a/crates/tabletist-mcp/tests/support/mod.rs
+++ b/crates/tabletist-mcp/tests/support/mod.rs
@@ -211,7 +211,7 @@ impl Session for ShopSession {
     }
 
     async fn hidden(&self, tables: &Tables) -> Result<Hidden, Failure> {
-        Ok(self.connection.agent_hidden(tables).await?)
+        Ok(self.connection.agent_hidden(tables, &[]).await?)
     }
 
     async fn read(&self, read: &Read, limits: Limits, tables: &Tables) -> Result<Answer, Failure> {
@@ -226,7 +226,9 @@ impl Session for ShopSession {
             },
         };
         let flag = StopFlag::new();
-        let run = self.connection.run_agent_read(read, limits, tables, &flag);
+        let run = self
+            .connection
+            .run_agent_read(read, limits, tables, &[], &flag);
         tokio::pin!(run);
         let answer = tokio::select! {
             answer = &mut run => answer,
````

- [ ] **Step 2: Run it to see that it does not build**

Run: `cargo test --locked -p tabletist-db --test agent masked`

Expected: it does not build, as the drivers do not take the masked columns yet:

```text
error[E0061]: this method takes 4 arguments but 5 arguments were supplied
error[E0061]: this method takes 1 argument but 2 arguments were supplied
error: could not compile `tabletist-db` (test "agent") due to 13 previous errors
```

- [ ] **Step 3: The drivers**

````diff
diff --git a/crates/tabletist-db/src/adapter.rs b/crates/tabletist-db/src/adapter.rs
index 4a34b3c..ba5dbbb 100644
--- a/crates/tabletist-db/src/adapter.rs
+++ b/crates/tabletist-db/src/adapter.rs
@@ -1,6 +1,6 @@
 //! What the adapter of every database provides.
 
-use crate::agent::{Catalog, Limits, Read, Tables};
+use crate::agent::{Catalog, ColumnName, Limits, Read, Tables};
 use crate::{
     CancelHandle, ChangeSet, Lookup, LookupPage, ObjectInfo, ObjectRef, Result, RowPage, RowQuery,
     ScriptMode, ScriptOutcome, StopFlag, Structure, WriteOutcome,
@@ -44,18 +44,21 @@ pub(crate) trait Adapter {
 
     /// `read` has passed [`crate::agent::check_read`] for this database's
     /// dialect. The outcome is a read-only script's of one statement, or
-    /// [`crate::Error::Agent`] where it reads what `tables` hides.
+    /// [`crate::Error::Agent`] where it reads what `tables` hides, or a
+    /// column `masked` names otherwise than as it is.
     async fn run_read(
         &self,
         read: &Read,
         limits: Limits,
         tables: &Tables,
+        masked: &[ColumnName],
         stop: &StopFlag,
     ) -> Result<ScriptOutcome>;
 
     /// What the database's catalog says for what an agent's connection
-    /// hides: its relations, what each reads, and the users' functions.
-    async fn agents_catalog(&self) -> Result<Catalog>;
+    /// hides and masks: its relations, what each reads, the users'
+    /// functions, and the columns of the tables `masked` names.
+    async fn agents_catalog(&self, masked: &[ColumnName]) -> Result<Catalog>;
 
     /// `changes` have passed their own check, and the session may write.
     async fn write(&self, changes: &ChangeSet, stop: &StopFlag) -> Result<WriteOutcome>;
diff --git a/crates/tabletist-db/src/agent.rs b/crates/tabletist-db/src/agent.rs
index 83f3a67..f82b17a 100644
--- a/crates/tabletist-db/src/agent.rs
+++ b/crates/tabletist-db/src/agent.rs
@@ -31,6 +31,7 @@ use crate::{Dialect, sql};
 mod hidden;
 mod run;
 
+pub(crate) use hidden::reads_whole_row;
 pub use hidden::{Catalog, ColumnName, ColumnRead, Hidden, Relation, TableName, Tables};
 pub(crate) use hidden::{calls_a_refused_function, named_relations, planned};
 pub use run::{Answer, Limits};
diff --git a/crates/tabletist-db/src/agent/hidden.rs b/crates/tabletist-db/src/agent/hidden.rs
index 976a3cf..f7b3976 100644
--- a/crates/tabletist-db/src/agent/hidden.rs
+++ b/crates/tabletist-db/src/agent/hidden.rs
@@ -759,6 +759,12 @@ fn masked_views(
     found
 }
 
+/// Whether a view's definition, as PostgreSQL writes it back, reads a whole
+/// row of a table with a masked column (for the catalog's [`ColumnRead`]).
+pub(crate) fn reads_whole_row(dialect: Dialect, definition: &str, masked: &[ColumnName]) -> bool {
+    masked::reads_whole_row(dialect, definition, &Masks::new(masked, []))
+}
+
 /// Whether the relation at `at` is hidden, and why, worked out once: by
 /// its own name, else by the first hidden relation it depends on. A
 /// relation met again while its own answer is being worked out (a cycle
diff --git a/crates/tabletist-db/src/agent/hidden/masked.rs b/crates/tabletist-db/src/agent/hidden/masked.rs
index eb1c857..fddab4a 100644
--- a/crates/tabletist-db/src/agent/hidden/masked.rs
+++ b/crates/tabletist-db/src/agent/hidden/masked.rs
@@ -510,6 +510,34 @@ fn selects(pieces: &[Piece], at: usize) -> bool {
     }
 }
 
+/// Whether a view's definition, as PostgreSQL writes it back, reads a
+/// whole row of a table `masks` knows. PostgreSQL keeps no column of its
+/// own for a whole row a view reads, and writes it back as `x.*`; a star
+/// the view selects is written back as its columns.
+pub(super) fn reads_whole_row(
+    dialect: crate::Dialect,
+    definition: &str,
+    masks: &Masks<'_>,
+) -> bool {
+    let pieces = pieces(dialect, definition);
+    let names = read_names(&pieces);
+    let standing: HashSet<String> = names
+        .places
+        .iter()
+        .filter(|place| masks.table(&place.name.name).is_some())
+        .flat_map(|place| {
+            let alias = place.alias.and_then(|at| pieces[at].word()).map(fold);
+            std::iter::once(fold(&place.name.name)).chain(alias)
+        })
+        .collect();
+    pieces.windows(3).any(|window| match window {
+        [qualifier, Piece::Dot, Piece::Star] => qualifier
+            .word()
+            .is_some_and(|word| standing.contains(&fold(word))),
+        _ => false,
+    })
+}
+
 #[cfg(test)]
 mod tests {
     use super::*;
diff --git a/crates/tabletist-db/src/lib.rs b/crates/tabletist-db/src/lib.rs
index 9d50965..9f722b5 100644
--- a/crates/tabletist-db/src/lib.rs
+++ b/crates/tabletist-db/src/lib.rs
@@ -251,12 +251,18 @@ impl Connection {
     /// asked for in the run's own transaction. Nothing hidden is read
     /// (the MCP plan's B.3).
     ///
+    /// Where `masked` names a column, the read is refused when its text
+    /// uses one otherwise than selecting it as it is, or reads one through
+    /// a view; and a renamed or dropped one refuses every read (B.4, C.3).
+    /// The values are not replaced here yet.
+    ///
     /// The future must be awaited to its end, as [`Self::run_script`]'s.
     pub async fn run_agent_read(
         &self,
         read: &agent::Read,
         limits: agent::Limits,
         tables: &agent::Tables,
+        masked: &[agent::ColumnName],
         stop: &StopFlag,
     ) -> Result<agent::Answer> {
         if read.dialect() != self.dialect() {
@@ -265,26 +271,36 @@ impl Connection {
             ));
         }
         let outcome = match &self.inner {
-            Inner::Sqlite(conn) => conn.run_read(read, limits, tables, stop).await,
-            Inner::Postgres(conn) => conn.run_read(read, limits, tables, stop).await,
-            Inner::MySql(conn) => conn.run_read(read, limits, tables, stop).await,
+            Inner::Sqlite(conn) => conn.run_read(read, limits, tables, masked, stop).await,
+            Inner::Postgres(conn) => conn.run_read(read, limits, tables, masked, stop).await,
+            Inner::MySql(conn) => conn.run_read(read, limits, tables, masked, stop).await,
         }?;
         agent::answer(outcome, stop)
     }
 
-    /// What `tables` hides on this connection's database as it is now: for
-    /// what the server tells an agent of the schema, which leaves it out.
-    /// The catalog is read only where something is hidden.
-    pub async fn agent_hidden(&self, tables: &agent::Tables) -> Result<agent::Hidden> {
-        let catalog = match tables.hides_anything() {
+    /// What `tables` hides and `masked` masks on this connection's
+    /// database as it is now: for what the server tells an agent of the
+    /// schema, which leaves the hidden out. The catalog is read only where
+    /// something is hidden or masked.
+    pub async fn agent_hidden(
+        &self,
+        tables: &agent::Tables,
+        masked: &[agent::ColumnName],
+    ) -> Result<agent::Hidden> {
+        let catalog = match tables.hides_anything() || !masked.is_empty() {
             true => match &self.inner {
-                Inner::Sqlite(conn) => conn.agents_catalog().await?,
-                Inner::Postgres(conn) => conn.agents_catalog().await?,
-                Inner::MySql(conn) => conn.agents_catalog().await?,
+                Inner::Sqlite(conn) => conn.agents_catalog(masked).await?,
+                Inner::Postgres(conn) => conn.agents_catalog(masked).await?,
+                Inner::MySql(conn) => conn.agents_catalog(masked).await?,
             },
             false => agent::Catalog::default(),
         };
-        Ok(agent::Hidden::new(self.dialect(), &catalog, tables))
+        Ok(agent::Hidden::masking(
+            self.dialect(),
+            &catalog,
+            tables,
+            masked,
+        ))
     }
 
     /// The server's name and version for the footer: `PostgreSQL 17.2`,
diff --git a/crates/tabletist-db/src/mysql.rs b/crates/tabletist-db/src/mysql.rs
index b9e5771..bdcecaf 100644
--- a/crates/tabletist-db/src/mysql.rs
+++ b/crates/tabletist-db/src/mysql.rs
@@ -20,7 +20,7 @@ use mysql_async::{DriverError, IoError, Opts, OptsBuilder, Params, SslOpts, TxOp
 use mysql_common::named_params::ParsedNamedParams;
 
 use crate::adapter::Adapter;
-use crate::agent::{Catalog, Limits, Read, Tables};
+use crate::agent::{Catalog, ColumnName, Limits, Read, Tables};
 use crate::script::retry_cancelled;
 use crate::{
     Access, CancelHandle, CancelInner, ChangeSet, ColumnClass, ColumnInfo, ColumnMeta, ConnectSpec,
@@ -518,14 +518,15 @@ impl Adapter for Conn {
         read: &Read,
         limits: Limits,
         tables: &Tables,
+        masked: &[ColumnName],
         stop: &StopFlag,
     ) -> Result<ScriptOutcome> {
-        self.agent_read(read, limits, tables, stop).await
+        self.agent_read(read, limits, tables, masked, stop).await
     }
 
-    async fn agents_catalog(&self) -> Result<Catalog> {
+    async fn agents_catalog(&self, masked: &[ColumnName]) -> Result<Catalog> {
         let mut conn = self.conn.lock().await;
-        hidden::catalog(&mut conn).await
+        hidden::catalog(&mut conn, masked).await
     }
 
     async fn write(&self, changes: &ChangeSet, stop: &StopFlag) -> Result<WriteOutcome> {
diff --git a/crates/tabletist-db/src/mysql/hidden.rs b/crates/tabletist-db/src/mysql/hidden.rs
index 6340ced..de84bc8 100644
--- a/crates/tabletist-db/src/mysql/hidden.rs
+++ b/crates/tabletist-db/src/mysql/hidden.rs
@@ -15,7 +15,9 @@
 use mysql_async::prelude::Queryable;
 
 use super::query_error;
-use crate::agent::{Catalog, Hidden, Read, Refusal, Relation, Tables, named_relations, planned};
+use crate::agent::{
+    Catalog, ColumnName, Hidden, Read, Refusal, Relation, Tables, named_relations, planned,
+};
 use crate::sql::{self, TokenKind};
 use crate::{Dialect, Error, Result};
 
@@ -34,9 +36,10 @@ pub(super) async fn check(
     conn: &mut mysql_async::Conn,
     read: &Read,
     tables: &Tables,
+    masked: &[ColumnName],
 ) -> Result<Option<Error>> {
-    let catalog = catalog(conn).await?;
-    let hidden = Hidden::new(Dialect::MySql, &catalog, tables);
+    let catalog = catalog(conn, masked).await?;
+    let hidden = Hidden::masking(Dialect::MySql, &catalog, tables, masked);
     if let Err(refusal) = hidden.check_statement(read) {
         return Ok(Some(Error::Agent(refusal)));
     }
@@ -169,8 +172,14 @@ fn rewritten_reads(note: &str, hidden: &Hidden) -> std::result::Result<(), Refus
 /// The catalog as the open transaction sees it: the relations of every
 /// schema but the system's, what each view reads where the server says so
 /// (`VIEW_TABLE_USAGE`, MySQL 8.0.13 and later), and the users' routines.
-/// Where it does not say, every view is taken to read what is hidden.
-pub(super) async fn catalog(conn: &mut mysql_async::Conn) -> Result<Catalog> {
+/// Where it does not say, every view is taken to read what is hidden. With
+/// the columns of the tables `masked` names; MySQL says what a view reads
+/// table by table, not column by column, so a view over a table with a
+/// masked column is taken to read it.
+pub(super) async fn catalog(
+    conn: &mut mysql_async::Conn,
+    masked: &[ColumnName],
+) -> Result<Catalog> {
     let listed: Vec<(String, String, String)> = conn
         .query(format!(
             "SELECT TABLE_SCHEMA, TABLE_NAME, TABLE_TYPE FROM information_schema.TABLES \
@@ -245,9 +254,29 @@ pub(super) async fn catalog(conn: &mut mysql_async::Conn) -> Result<Catalog> {
         ))
         .await
         .map_err(query_error)?;
+    let columns: Vec<(String, String, String)> = match masked.is_empty() {
+        true => Vec::new(),
+        false => {
+            let tables: Vec<String> = masked
+                .iter()
+                .map(|entry| entry.table.to_lowercase())
+                .collect();
+            let marks = vec!["?"; tables.len()].join(", ");
+            conn.exec(
+                format!(
+                    "SELECT TABLE_SCHEMA, TABLE_NAME, COLUMN_NAME FROM information_schema.COLUMNS \
+                     WHERE TABLE_SCHEMA NOT IN {SYSTEM} AND LOWER(TABLE_NAME) IN ({marks}) {WHOLE}"
+                ),
+                tables,
+            )
+            .await
+            .map_err(query_error)?
+        }
+    };
     Ok(Catalog {
         relations,
         routines,
+        columns,
         ..Catalog::default()
     })
 }
diff --git a/crates/tabletist-db/src/mysql/script.rs b/crates/tabletist-db/src/mysql/script.rs
index c4b96e8..a20960c 100644
--- a/crates/tabletist-db/src/mysql/script.rs
+++ b/crates/tabletist-db/src/mysql/script.rs
@@ -65,6 +65,7 @@ impl Conn {
         read: &Read,
         limits: Limits,
         tables: &crate::agent::Tables,
+        masked: &[crate::agent::ColumnName],
         stop: &StopFlag,
     ) -> Result<ScriptOutcome> {
         let mut conn = self.conn.lock().await;
@@ -79,7 +80,12 @@ impl Conn {
             timeout: limits.timeout,
         };
         let access = self.access;
-        let hiding = tables.hides_anything().then_some((read, tables));
+        let guards = tables.hides_anything() || !masked.is_empty();
+        let hiding = guards.then_some(Hiding {
+            read,
+            tables,
+            masked,
+        });
         read_only(
             &mut conn,
             &texts,
@@ -93,10 +99,19 @@ impl Conn {
     }
 }
 
+/// An agent's statement, with what its connection hides and masks.
+#[derive(Clone, Copy)]
+struct Hiding<'a> {
+    read: &'a Read,
+    tables: &'a crate::agent::Tables,
+    masked: &'a [crate::agent::ColumnName],
+}
+
 /// The run that only reads: its transaction, its statements, and the
 /// close. `sent` is there when the run is of an agent's one statement, and
-/// `hiding` when its connection hides tables: the statement is checked
-/// against them in the transaction, and runs only if it reads none.
+/// `hiding` when its connection hides tables or masks columns: the
+/// statement is checked against them in the transaction, and runs only if
+/// it reads nothing it may not.
 async fn read_only(
     conn: &mut mysql_async::Conn,
     texts: &[String],
@@ -104,7 +119,7 @@ async fn read_only(
     access: Access,
     stop: &StopFlag,
     sent: Option<Sent<'_>>,
-    hiding: Option<(&Read, &crate::agent::Tables)>,
+    hiding: Option<Hiding<'_>>,
 ) -> Result<ScriptOutcome> {
     let mut outcome = ScriptOutcome::default();
     let ended = match open(conn, limit, access, sent).await {
@@ -142,17 +157,17 @@ async fn read_only(
 }
 
 /// Why an agent's statement may not run on a connection that hides tables
-/// (`super::hidden`): a refusal, the server's own error, or a cancel.
-/// `None` where it may run, or where nothing is hidden. `Err` is a lost
-/// session.
+/// or masks columns (`super::hidden`): a refusal, the server's own error,
+/// or a cancel. `None` where it may run, or where nothing is hidden or
+/// masked. `Err` is a lost session.
 async fn refused_for_hiding(
     conn: &mut mysql_async::Conn,
-    hiding: Option<(&Read, &crate::agent::Tables)>,
+    hiding: Option<Hiding<'_>>,
 ) -> Result<Option<Error>> {
-    let Some((read, tables)) = hiding else {
+    let Some(hiding) = hiding else {
         return Ok(None);
     };
-    match super::hidden::check(conn, read, tables).await {
+    match super::hidden::check(conn, hiding.read, hiding.tables, hiding.masked).await {
         Ok(refused) => Ok(refused),
         Err(error) if error.is_connection_lost() => Err(error),
         Err(error) => Ok(Some(error)),
diff --git a/crates/tabletist-db/src/pg.rs b/crates/tabletist-db/src/pg.rs
index 26795ae..7bf2ae4 100644
--- a/crates/tabletist-db/src/pg.rs
+++ b/crates/tabletist-db/src/pg.rs
@@ -12,7 +12,7 @@ use tokio_postgres::tls::{MakeTlsConnect, TlsConnect};
 use tokio_postgres::{SimpleQueryMessage, Socket};
 
 use crate::adapter::Adapter;
-use crate::agent::{Catalog, Limits, Read, Tables};
+use crate::agent::{Catalog, ColumnName, Limits, Read, Tables};
 use crate::{
     Access, CancelHandle, CancelInner, ChangeSet, ColumnInfo, ColumnMeta, ConnectSpec, Dialect,
     Error, ForeignKeyInfo, IndexInfo, Lookup, LookupPage, MAX_LISTED, Named, ObjectInfo,
@@ -833,14 +833,15 @@ impl Adapter for Conn {
         read: &Read,
         limits: Limits,
         tables: &Tables,
+        masked: &[ColumnName],
         stop: &StopFlag,
     ) -> Result<ScriptOutcome> {
-        self.agent_read(read, limits, tables, stop).await
+        self.agent_read(read, limits, tables, masked, stop).await
     }
 
-    async fn agents_catalog(&self) -> Result<Catalog> {
+    async fn agents_catalog(&self, masked: &[ColumnName]) -> Result<Catalog> {
         let client = self.client.lock().await;
-        hidden::catalog(&client).await
+        hidden::catalog(&client, masked).await
     }
 
     async fn write(&self, changes: &ChangeSet, stop: &StopFlag) -> Result<WriteOutcome> {
diff --git a/crates/tabletist-db/src/pg/hidden.rs b/crates/tabletist-db/src/pg/hidden.rs
index 5080a4c..4f3200f 100644
--- a/crates/tabletist-db/src/pg/hidden.rs
+++ b/crates/tabletist-db/src/pg/hidden.rs
@@ -13,8 +13,8 @@ use tokio_postgres::SimpleQueryMessage;
 
 use super::query_error;
 use crate::agent::{
-    Catalog, Hidden, Read, Refusal, Relation, Tables, calls_a_refused_function, function_allowed,
-    planned,
+    Catalog, ColumnName, ColumnRead, Hidden, Read, Refusal, Relation, Tables,
+    calls_a_refused_function, function_allowed, planned, reads_whole_row,
 };
 use crate::{Dialect, Error, Result};
 
@@ -86,6 +86,32 @@ ORDER BY n.nspname, c.relname"
     )
 }
 
+/// The columns of the relations of the names in `$1`, folded to lower
+/// case: the masked columns' tables.
+const COLUMNS_SQL: &str = "\
+SELECT n.nspname, c.relname, a.attname
+FROM pg_attribute a
+JOIN pg_class c ON c.oid = a.attrelid
+JOIN pg_namespace n ON n.oid = c.relnamespace
+WHERE a.attnum > 0 AND NOT a.attisdropped
+    AND c.relkind IN ('r', 'v', 'm', 'p', 'f')
+    AND lower(c.relname) = ANY($1)";
+
+/// The columns of the names in `$1`, folded to lower case, that a view's or
+/// a materialized view's rules read, as `pg_depend` keeps them: the view,
+/// and the table and column it reads.
+const COLUMN_READS_SQL: &str = "\
+SELECT DISTINCT vn.nspname, v.relname, tn.nspname, t.relname, a.attname
+FROM pg_rewrite r
+JOIN pg_class v ON v.oid = r.ev_class
+JOIN pg_namespace vn ON vn.oid = v.relnamespace
+JOIN pg_depend d ON d.classid = 'pg_rewrite'::regclass AND d.objid = r.oid
+    AND d.refclassid = 'pg_class'::regclass AND d.refobjid <> r.ev_class AND d.refobjsubid > 0
+JOIN pg_class t ON t.oid = d.refobjid
+JOIN pg_namespace tn ON tn.oid = t.relnamespace
+JOIN pg_attribute a ON a.attrelid = t.oid AND a.attnum = d.refobjsubid
+WHERE lower(a.attname) = ANY($1)";
+
 /// The functions the users wrote, by schema and name.
 fn routines_sql() -> String {
     format!("WITH {USER_ROUTINES} SELECT DISTINCT nspname, proname FROM user_routines")
@@ -126,9 +152,10 @@ pub(super) async fn check(
     client: &tokio_postgres::Client,
     read: &Read,
     tables: &Tables,
+    masked: &[ColumnName],
 ) -> Result<Option<Error>> {
-    let catalog = catalog(client).await?;
-    let hidden = Hidden::new(Dialect::Postgres, &catalog, tables);
+    let catalog = catalog(client, masked).await?;
+    let hidden = Hidden::masking(Dialect::Postgres, &catalog, tables, masked);
     if let Err(refusal) = hidden.check_statement(read) {
         return Ok(Some(Error::Agent(refusal)));
     }
@@ -186,9 +213,16 @@ fn planned_reads(plan: &serde_json::Value, hidden: &Hidden) -> std::result::Resu
     }
 }
 
-/// The catalog as the open transaction sees it.
-pub(super) async fn catalog(client: &tokio_postgres::Client) -> Result<Catalog> {
+/// The catalog as the open transaction sees it, with what it says of the
+/// columns `masked` names.
+pub(super) async fn catalog(
+    client: &tokio_postgres::Client,
+    masked: &[ColumnName],
+) -> Result<Catalog> {
     let mut relations: Vec<Relation> = Vec::new();
+    // The views whose definitions read a whole row of a table with a
+    // masked column.
+    let mut whole_rows: Vec<usize> = Vec::new();
     for row in rows(client, &relations_sql()).await? {
         let (Some(schema), Some(name)) = (row.get(0), row.get(1)) else {
             continue;
@@ -208,6 +242,12 @@ pub(super) async fn catalog(client: &tokio_postgres::Client) -> Result<Catalog>
                 part: row.get(5) == Some("t"),
                 view: row.get(7) == Some("t"),
             });
+            if !masked.is_empty()
+                && row.get(7) == Some("t")
+                && reads_whole_row(Dialect::Postgres, row.get(6).unwrap_or(""), masked)
+            {
+                whole_rows.push(relations.len() - 1);
+            }
         }
         if let (Some(on_schema), Some(on_name), Some(last)) =
             (row.get(2), row.get(3), relations.last_mut())
@@ -226,10 +266,56 @@ pub(super) async fn catalog(client: &tokio_postgres::Client) -> Result<Catalog>
         .iter()
         .filter_map(|row| Some(row.get(0)?.to_owned()))
         .collect();
+    let (columns, column_reads) = match masked.is_empty() {
+        true => (Vec::new(), None),
+        false => {
+            let tables: Vec<String> = masked
+                .iter()
+                .map(|entry| entry.table.to_lowercase())
+                .collect();
+            let names: Vec<String> = masked
+                .iter()
+                .map(|entry| entry.column.to_lowercase())
+                .collect();
+            let columns = client
+                .query(COLUMNS_SQL, &[&tables])
+                .await
+                .map_err(query_error)?
+                .iter()
+                .map(|row| (row.get(0), row.get(1), row.get(2)))
+                .collect();
+            let mut reads: Vec<ColumnRead> = client
+                .query(COLUMN_READS_SQL, &[&names])
+                .await
+                .map_err(query_error)?
+                .iter()
+                .map(|row| ColumnRead {
+                    schema: row.get(0),
+                    view: row.get(1),
+                    table_schema: row.get(2),
+                    table: row.get(3),
+                    column: row.get(4),
+                })
+                .collect();
+            for &at in &whole_rows {
+                let view = &relations[at];
+                reads.extend(view.depends_on.iter().map(|(schema, table)| ColumnRead {
+                    schema: view.schema.clone(),
+                    view: view.name.clone(),
+                    table_schema: schema.clone(),
+                    table: table.clone(),
+                    column: "*".into(),
+                }));
+            }
+            (columns, Some(reads))
+        }
+    };
     Ok(Catalog {
         relations,
         routines,
         unnamed_calls,
+        columns,
+        column_reads,
         ..Catalog::default()
     })
 }
diff --git a/crates/tabletist-db/src/pg/script.rs b/crates/tabletist-db/src/pg/script.rs
index 166bf81..bbec0e0 100644
--- a/crates/tabletist-db/src/pg/script.rs
+++ b/crates/tabletist-db/src/pg/script.rs
@@ -51,6 +51,7 @@ impl Conn {
         read: &Read,
         limits: Limits,
         tables: &crate::agent::Tables,
+        masked: &[crate::agent::ColumnName],
         stop: &StopFlag,
     ) -> Result<ScriptOutcome> {
         let client = self.client.lock().await;
@@ -59,7 +60,12 @@ impl Conn {
             params: read.params(),
             timeout: limits.timeout,
         };
-        let hiding = tables.hides_anything().then_some((read, tables));
+        let guards = tables.hides_anything() || !masked.is_empty();
+        let hiding = guards.then_some(Hiding {
+            read,
+            tables,
+            masked,
+        });
         read_only(
             &client,
             &texts,
@@ -72,17 +78,26 @@ impl Conn {
     }
 }
 
+/// An agent's statement, with what its connection hides and masks.
+#[derive(Clone, Copy)]
+struct Hiding<'a> {
+    read: &'a Read,
+    tables: &'a crate::agent::Tables,
+    masked: &'a [crate::agent::ColumnName],
+}
+
 /// The run that only reads: its transaction, its statements, and the
 /// close. `sent` is there when the run is of an agent's one statement, and
-/// `hiding` when its connection hides tables: the statement is checked
-/// against them in the transaction, and runs only if it reads none.
+/// `hiding` when its connection hides tables or masks columns: the
+/// statement is checked against them in the transaction, and runs only if
+/// it reads nothing it may not.
 async fn read_only(
     client: &tokio_postgres::Client,
     texts: &[String],
     limit: usize,
     stop: &StopFlag,
     sent: Option<Sent<'_>>,
-    hiding: Option<(&Read, &crate::agent::Tables)>,
+    hiding: Option<Hiding<'_>>,
 ) -> Result<ScriptOutcome> {
     let mut outcome = ScriptOutcome::default();
     let tx = match open(client, sent).await {
@@ -127,17 +142,18 @@ async fn read_only(
 }
 
 /// Why an agent's statement may not run on a connection that hides tables
-/// (`super::hidden`), and where that leaves the transaction: open after a
-/// refusal, failed after the server's own error. `None` where it may run,
-/// or where nothing is hidden. `Err` is a lost session.
+/// or masks columns (`super::hidden`), and where that leaves the
+/// transaction: open after a refusal, failed after the server's own error.
+/// `None` where it may run, or where nothing is hidden or masked. `Err` is
+/// a lost session.
 async fn refused_for_hiding(
     client: &tokio_postgres::Client,
-    hiding: Option<(&Read, &crate::agent::Tables)>,
+    hiding: Option<Hiding<'_>>,
 ) -> Result<Option<(Error, Tx)>> {
-    let Some((read, tables)) = hiding else {
+    let Some(hiding) = hiding else {
         return Ok(None);
     };
-    match super::hidden::check(client, read, tables).await {
+    match super::hidden::check(client, hiding.read, hiding.tables, hiding.masked).await {
         Ok(None) => Ok(None),
         Ok(Some(refusal @ Error::Agent(_))) => Ok(Some((refusal, Tx::Open))),
         Ok(Some(error)) => Ok(Some((error, Tx::Aborted))),
diff --git a/crates/tabletist-db/src/sqlite.rs b/crates/tabletist-db/src/sqlite.rs
index d1de250..61227b9 100644
--- a/crates/tabletist-db/src/sqlite.rs
+++ b/crates/tabletist-db/src/sqlite.rs
@@ -13,7 +13,10 @@ use rusqlite::types::{FromSql, FromSqlError, FromSqlResult, ValueRef};
 use rusqlite::{ErrorCode, OpenFlags, OptionalExtension};
 
 use crate::adapter::Adapter;
-use crate::agent::{Catalog, Hidden, Limits, Param, Read, Relation, Sent, Tables, count_mismatch};
+use crate::agent::{
+    Catalog, ColumnName, ColumnRead, Hidden, Limits, Param, Read, Relation, Sent, Tables,
+    count_mismatch,
+};
 use crate::script::{cancelled_commit, cleanup_failed};
 use crate::{
     Access, CancelHandle, CancelInner, ChangeSet, ColumnClass, ColumnInfo, ColumnMeta, Dialect,
@@ -913,9 +916,10 @@ impl Adapter for Conn {
         CancelHandle(CancelInner::Sqlite(Arc::clone(&self.interrupt)))
     }
 
-    async fn agents_catalog(&self) -> Result<Catalog> {
+    async fn agents_catalog(&self, masked: &[ColumnName]) -> Result<Catalog> {
         let fences = self.fences.clone();
-        self.run(move |connection| agents_catalog(connection, &fences))
+        let masked = masked.to_vec();
+        self.run(move |connection| agents_catalog(connection, &fences, &masked))
             .await
     }
 
@@ -1151,6 +1155,7 @@ impl Adapter for Conn {
         read: &Read,
         limits: Limits,
         tables: &Tables,
+        masked: &[ColumnName],
         stop: &StopFlag,
     ) -> Result<ScriptOutcome> {
         let stop = stop.clone();
@@ -1158,15 +1163,15 @@ impl Adapter for Conn {
         let fences = self.fences.clone();
         let texts = vec![read.text().to_owned()];
         let params = read.params().to_vec();
-        let (read, tables) = (read.clone(), tables.clone());
+        let (read, tables, masked) = (read.clone(), tables.clone(), masked.to_vec());
         let outcome = self
             .run(move |connection| {
-                // What is hidden, from the catalog as it is now: a table
-                // made a moment ago is in it.
-                let hidden = match tables.hides_anything() {
+                // What is hidden and masked, from the catalog as it is
+                // now: a table made a moment ago is in it.
+                let hidden = match tables.hides_anything() || !masked.is_empty() {
                     true => {
-                        let catalog = agents_catalog(connection, &fences)?;
-                        let hidden = Hidden::new(Dialect::Sqlite, &catalog, &tables);
+                        let catalog = agents_catalog(connection, &fences, &masked)?;
+                        let hidden = Hidden::masking(Dialect::Sqlite, &catalog, &tables, &masked);
                         hidden.check_statement(&read).map_err(Error::Agent)?;
                         Some(Arc::new(hidden))
                     }
@@ -1200,8 +1205,14 @@ impl Adapter for Conn {
 /// schema, a view depends on what SQLite reports it reads as a read of it
 /// is prepared. A shadow table depends
 /// on its virtual table, whose name and an underscore begin its own
-/// (FTS5's `notes_config` is `notes`'s).
-fn agents_catalog(connection: &rusqlite::Connection, fences: &Fences) -> Result<Catalog> {
+/// (FTS5's `notes_config` is `notes`'s). With the columns of the tables
+/// `masked` names, and the masked columns each view reads, as SQLite
+/// reports them column by column.
+fn agents_catalog(
+    connection: &rusqlite::Connection,
+    fences: &Fences,
+    masked: &[ColumnName],
+) -> Result<Catalog> {
     let mut statement = connection
         .prepare(
             "SELECT t.schema, t.name, t.type = 'shadow', v.name, t.type = 'view' \
@@ -1253,6 +1264,11 @@ fn agents_catalog(connection: &rusqlite::Connection, fences: &Fences) -> Result<
     // What each view reads, as SQLite says when it prepares a read of the
     // view. One it cannot prepare (a table it reads is gone) reads what
     // nobody can say, and is hidden while anything is.
+    let masked_names: Vec<String> = masked
+        .iter()
+        .map(|entry| entry.column.to_lowercase())
+        .collect();
+    let mut column_reads: Vec<ColumnRead> = Vec::new();
     for relation in relations
         .iter_mut()
         .filter(|relation| views.contains(&(relation.schema.clone(), relation.name.clone())))
@@ -1264,14 +1280,52 @@ fn agents_catalog(connection: &rusqlite::Connection, fences: &Fences) -> Result<
             quoted(&relation.name)
         );
         match fences.reads_of(|| connection.prepare(&sql).is_ok()) {
-            Some(reads) => relation.depends_on.extend(
-                reads
-                    .into_iter()
-                    .filter(|(schema, name)| *schema != relation.schema || *name != relation.name),
-            ),
+            Some(reads) => {
+                for (schema, name, column) in reads {
+                    if schema == relation.schema && name == relation.name {
+                        continue;
+                    }
+                    if masked_names.contains(&column.to_lowercase()) {
+                        column_reads.push(ColumnRead {
+                            schema: relation.schema.clone(),
+                            view: relation.name.clone(),
+                            table_schema: schema.clone(),
+                            table: name.clone(),
+                            column,
+                        });
+                    }
+                    let on = (schema, name);
+                    if !relation.depends_on.contains(&on) {
+                        relation.depends_on.push(on);
+                    }
+                }
+            }
             None => relation.opaque = true,
         }
     }
+    // The columns of the masked columns' tables.
+    let mut columns = Vec::new();
+    for relation in relations.iter().filter(|relation| {
+        masked
+            .iter()
+            .any(|entry| entry.names_table(Some(&relation.schema), &relation.name))
+    }) {
+        let mut statement = connection
+            .prepare("SELECT name FROM pragma_table_xinfo(?1, ?2)")
+            .map_err(map_error)?;
+        let names = statement
+            .query_map([&relation.name, &relation.schema], |row| {
+                row.get::<_, String>(0)
+            })
+            .map_err(map_error)?
+            .collect::<rusqlite::Result<Vec<_>>>()
+            .map_err(map_error)?;
+        columns.extend(
+            names
+                .into_iter()
+                .map(|column| (relation.schema.clone(), relation.name.clone(), column)),
+        );
+    }
     // The table functions, SQLite's own modules read without a table
     // made of them: the pragmas are refused before this, and `dbstat` and
     // the `sqlite_` ones are system catalogs.
@@ -1286,6 +1340,8 @@ fn agents_catalog(connection: &rusqlite::Connection, fences: &Fences) -> Result<
     Ok(Catalog {
         relations,
         table_functions,
+        columns,
+        column_reads: Some(column_reads),
         ..Catalog::default()
     })
 }
diff --git a/crates/tabletist-db/src/sqlite/fence.rs b/crates/tabletist-db/src/sqlite/fence.rs
index 78974c3..9c348a6 100644
--- a/crates/tabletist-db/src/sqlite/fence.rs
+++ b/crates/tabletist-db/src/sqlite/fence.rs
@@ -158,8 +158,9 @@ struct Shared {
     refused: Mutex<Option<Refusal>>,
     /// What the agent's connection hides, while its statement runs.
     hidden: Mutex<Option<Arc<Hidden>>>,
-    /// The tables read, by schema and name, while they are recorded.
-    reads: Mutex<Option<Vec<(String, String)>>>,
+    /// The tables read, by schema and name, with the column read (empty
+    /// for none), while they are recorded.
+    reads: Mutex<Option<Vec<(String, String, String)>>>,
 }
 
 impl Fences {
@@ -205,7 +206,8 @@ impl Fences {
             && let Some(reads) = self.reads().as_mut()
         {
             let schema = action.database.unwrap_or("main").to_owned();
-            reads.push((schema, table.to_owned()));
+            let column = action.second.unwrap_or_default().to_owned();
+            reads.push((schema, table.to_owned(), column));
         }
         let mut answer = authorize(fence, action);
         let mut hidden_read = None;
@@ -219,6 +221,19 @@ impl Fences {
             answer = Authorization::Deny;
             hidden_read = Some(through_view(refusal, action.source));
         }
+        // A masked column a view or a common table expression reads: what
+        // it does with it, the statement's check cannot see.
+        if fence == Fence::Agent
+            && answer == Authorization::Allow
+            && action.code == ffi::SQLITE_READ
+            && let (Some(table), Some(column)) = (action.first, action.second)
+            && let Some(hidden) = self.hidden()
+            && let Err(refusal) =
+                hidden.check_column_read(action.database, table, column, action.source)
+        {
+            answer = Authorization::Deny;
+            hidden_read = Some(refusal);
+        }
         if fence == Fence::Agent
             && answer == Authorization::Deny
             && let Ok(mut refused) = self.0.refused.lock()
@@ -247,16 +262,20 @@ impl Fences {
     }
 
     /// The tables SQLite reads as it prepares what `prepare` prepares, by
-    /// schema and name: for a view, every table it reads, through the
-    /// views it reads too. `None` where SQLite could not prepare it.
-    pub(super) fn reads_of(&self, prepare: impl FnOnce() -> bool) -> Option<Vec<(String, String)>> {
+    /// schema and name, each with a column it reads of it (empty for none):
+    /// for a view, every table it reads, through the views it reads too.
+    /// `None` where SQLite could not prepare it.
+    pub(super) fn reads_of(
+        &self,
+        prepare: impl FnOnce() -> bool,
+    ) -> Option<Vec<(String, String, String)>> {
         *self.reads() = Some(Vec::new());
         let prepared = prepare();
         let reads = self.reads().take();
         prepared.then_some(reads).flatten()
     }
 
-    fn reads(&self) -> std::sync::MutexGuard<'_, Option<Vec<(String, String)>>> {
+    fn reads(&self) -> std::sync::MutexGuard<'_, Option<Vec<(String, String, String)>>> {
         self.0.reads.lock().unwrap_or_else(PoisonError::into_inner)
     }
 
diff --git a/src/backend/agents.rs b/src/backend/agents.rs
index ecc3aed..5de4b82 100644
--- a/src/backend/agents.rs
+++ b/src/backend/agents.rs
@@ -906,7 +906,7 @@ impl Session for AppSession {
 
     async fn hidden(&self, tables: &Tables) -> Result<Hidden, Failure> {
         let connection = self.connection()?;
-        self.ended(None, connection.agent_hidden(tables)).await
+        self.ended(None, connection.agent_hidden(tables, &[])).await
     }
 
     async fn read(&self, read: &Read, limits: Limits, tables: &Tables) -> Result<Answer, Failure> {
@@ -914,7 +914,7 @@ impl Session for AppSession {
         let flag = StopFlag::new();
         self.ended(
             Some(&flag),
-            connection.run_agent_read(read, limits, tables, &flag),
+            connection.run_agent_read(read, limits, tables, &[], &flag),
         )
         .await
     }
````

- [ ] **Step 4: Run the tests**

Run: `cargo test --locked -p tabletist-db --test agent`, then `cargo test --locked -p tabletist-mcp --tests`, with both test servers

Expected: PASS, with the servers' tests run and none skipped:

```text
test result: ok. 31 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

and every one of `tabletist-mcp`'s test binaries passes (52, 5, 2, 9, 10 and 24 tests).

- [ ] **Step 5: Commit**

```bash
cargo fmt --all
cargo clippy --locked --workspace --all-targets -- -D warnings
git add crates/tabletist-db src/backend/agents.rs crates/tabletist-mcp/tests/support/mod.rs
git commit -m "Check an agent's statement against the masked columns on each database"
```

---

### Task 3: The master plan

- Phase 2 last; 3a's four questions settled and 3a cut into runs, with the first in two; B.4 and C.3 as built for 3a-1a; G's row.

**Files:**
- Modify: `docs/superpowers/plans/2026-10-13-mcp-server.md`

- [ ] **Step 1: The edits**

````diff
diff --git a/docs/superpowers/plans/2026-10-13-mcp-server.md b/docs/superpowers/plans/2026-10-13-mcp-server.md
index 21b2b0b..c525f46 100644
--- a/docs/superpowers/plans/2026-10-13-mcp-server.md
+++ b/docs/superpowers/plans/2026-10-13-mcp-server.md
@@ -84,6 +84,25 @@ side, the setting in `connections.toml`, the editor's Tables control, and
 its list of the tables an open window loaded with the website's guide
 (`2026-10-16-mcp-server-1c-3b-table-list.md`). With 1d dropped, Phase 1 is built.
 
+**Changed on 2026-10-17: approved writes come last.** Phase 2
+(`propose_write`, the approval, notifications, Read + approved writes) is
+built after Phase 3: 3a, masked columns, is next, then 3b, the Log view in
+full, then 2. The numbers stay the brief's. Neither part of 3 needs a
+write: 3b's result filter takes the outcomes there are, and a write's join
+it with Phase 2. Until then agents read and nothing more, as Phase 1 left
+them, and where another section says something comes "with writes (Phase
+2)", it comes last.
+
+**Settled on 2026-10-17: how 3a finds a masked column, what backs it, a
+renamed one, and 3a in four runs.** The checker reads a masked column's
+uses off the tokens, as 1c reads a table's, by its name: whatever table a
+column of that name is on, it is allowed only bare in the outermost
+`SELECT`'s list. A over B is not taken: what backs the checker is each
+engine's own fence, with the SQL that runs the SQL that was sent. A masked
+column renamed or dropped stops agents on all three, as a hidden table
+does. B.4, C.3 and G say each where it stands. B.4's rule 7, a masked
+column in `propose_write`, goes with writes to Phase 2.
+
 ## Contents
 
 1. [What the code says, in short](#1-what-the-code-says-in-short)
@@ -1048,6 +1067,53 @@ If the test matrix of F.2 finds a leak in A that cannot be closed, B is
 added under it, so that a use the checker misses reads `NULL`. That is a
 decision for Phase 3a, with the matrix in hand.
 
+**Settled on 2026-10-17: A, read off the tokens, over the engines'
+fences.** The tree is not walked (B.2), so rule 5's resolver is not built:
+the checker reads the statement's tokens, as 1c's reading of the names
+does, and goes by a masked column's name alone. Each word that is a masked
+column's name, whatever table or alias it is written with, is refused but
+where rule 1 allows it, and rules 2 to 4 are read off the tokens the same
+way. That errs toward refusing: with `orders.email` masked, `WHERE
+customers.email = …` is refused too, and the sentence says so. B is not
+added. What a miss by the reading meets is each engine's own:
+
+- SQLite: the authorizer's "ignore", so that a masked column reads `NULL`
+  wherever it stands, through every view.
+- PostgreSQL: the plan that 1c already asks for, `EXPLAIN (VERBOSE)`,
+  where a masked column may stand only in the top node's output.
+- MySQL: every view over a table with a masked column refused (rule 6),
+  and the result columns' origins.
+
+On all three a result column is replaced by its origin too, as below.
+Rule 7 goes with writes to Phase 2.
+
+**As built in 3a's run 1a** (`2026-10-17-mcp-server-3a-1a-masked-rule.md`):
+the rule, in `crates/tabletist-db/src/agent/hidden/masked.rs`, with the
+hidden tables' checks, behind a `masked` argument of
+`Connection::run_agent_read`; the app passes none until run 2, and the
+values are run 1b's. Where it went otherwise than above:
+
+- **An alias stands for its column.** SQLite reads a result column's
+  alias in a `WHERE`, and SQLite and MySQL in an `ORDER BY`'s expressions
+  (`ORDER BY lower(e)`), so the alias a masked column is selected under is
+  refused wherever else it is written (found by the run's own probing).
+- **SQLite reads `ORDER BY (2)` and `ORDER BY +2` as the place**, so a
+  number reached past parentheses and signs is a place too.
+- **A view an entry names** has its column masked as a table's is, and is
+  not refused for it; a view that reads a masked column is.
+- **PostgreSQL keeps no column for a whole row a view reads**
+  (`pg_depend` records the table alone), and writes it back as `x.*`:
+  such a view over a table with a masked column reads it.
+- **`TABLE orders` and `U&"…"` names** are refused for every agent before
+  the masked columns are asked about (`check_read`), so the rule has
+  nothing to say of them.
+- **An aggregate without `GROUP BY`** beside a masked column
+  (`SELECT email, max(id)`) is taken: it answers one row, whose value is
+  replaced as any is.
+- A partition or a child table of a table with a masked column has it
+  masked, and a masked column named in another schema is taken for one in
+  every schema by the reading, which errs toward refusing.
+
 **The rule.** It applies to any statement that reads a table with a masked
 column.
 
@@ -1643,7 +1709,7 @@ changes under them.
 | A hidden table is dropped | The entry stays, marked "not in this database" in the editor. If the table comes back it is hidden |
 | A hidden table is renamed, PostgreSQL | Followed. The entry keeps the table's OID from the day it was picked, and a rename keeps the OID. The editor shows the new name. The plan check compares OIDs, so there is no gap |
 | A hidden table is renamed, MySQL or SQLite | Not visible as a rename: one name is gone and another is new. **Agents are stopped on that connection** (`connection_unavailable`, "Its agent settings need a look in Tabletist") until the person removes the entry or points it at a table. A leak is the cost of guessing |
-| A masked column is renamed or dropped | The same: followed by attribute number on PostgreSQL, a stop elsewhere |
+| A masked column is renamed or dropped | The same: followed by attribute number on PostgreSQL, a stop elsewhere. **Settled on 2026-10-17:** a stop on all three, as for a hidden table since 1c-2a, with the entry marked in the editor |
 | A new table | Visible under All and All except, hidden under Only. The editor's hint says so beside the control |
 | A new column on a table with masked columns | Visible. Masks are per column |
 
@@ -2484,6 +2550,9 @@ is off until the person turns it on.
 | 1b on Windows: the way in | 1 | Built (`2026-10-15-mcp-server-1b-6-windows.md`) |
 | 1c. Hidden tables | 6, below | Built (`2026-10-15-mcp-server-1c-1-checks.md`, `2026-10-15-mcp-server-1c-2a-server.md`, `2026-10-16-mcp-server-1c-2b-setting.md`, `2026-10-16-mcp-server-1c-3a-tables-control.md`, `2026-10-16-mcp-server-1c-3b-table-list.md`) |
 | 1d. HTTP with tokens | 1 | Dropped for now (2026-10-15) |
+| 3a. Masked columns | 5, below | Run 1a built (`2026-10-17-mcp-server-3a-1a-masked-rule.md`): the rule and its stops. Run 1b, the engines' fences and the values, is next (order changed on 2026-10-17) |
+| 3b. The Log view, in full | Not cut yet | After 3a |
+| 2. Approved writes | Not cut yet | Last, after 3b (moved on 2026-10-17) |
 
 The move of the connections to `connections.toml` came between 1a and 1b
 (pull request 125, `2026-10-13-connections-toml.md`).
@@ -2664,6 +2733,22 @@ What follows is the design, kept for if it is taken up again.
   between A alone and A over B is made here, on the matrix.
 - Tests: the masking rows of F.2, for each driver.
 
+**3a in four runs** (cut on 2026-10-17, as 1c went). Each is a plan and a
+pull request:
+
+1. The checks, in `crates/tabletist-db`, with nothing visible, in two
+   (agreed on 2026-10-17, as 1c's second run was): first the rule read off
+   the tokens, its refusal, a view that reads a masked column refused, a
+   renamed or dropped masked column stopping agents, and the system
+   catalogs and the users' functions closed on a connection that masks
+   (`2026-10-17-mcp-server-3a-1a-masked-rule.md`, built); then each
+   engine's fence and the values replaced by the result columns' origins.
+2. The server's side: the setting read, `list_connections`' masked
+   columns, `describe_schema`'s mark, and `masked: true` in an answer.
+3. The setting in `connections.toml`, as 1c-2b's for tables.
+4. The editor's Masked columns control with **Add column…** and its mark,
+   and the website's guide.
+
 **3b. The Log view, in full.** (This was a log window before 2026-10-13.)
 
 - Files: `src/ui/agents/log.rs` (the agent and result filters, the search,
````

- [ ] **Step 2: Check the whole of it**

Run: `cargo fmt --all --check`, clippy as above, `RUSTDOCFLAGS='-D warnings' cargo doc --locked --workspace --no-deps`, the count of em dashes in the master plan, and `cargo test --locked --workspace --all-targets` with both servers

Expected: fmt and clippy clean, the documentation builds without a warning, no em dash (`0`), and:

```text
3546 passed, 0 failed, 22 ignored
```

- [ ] **Step 3: Commit**

```bash
git add docs/superpowers/plans/2026-10-13-mcp-server.md
git commit -m "Say in the MCP plan that approved writes come last, and how 3a reads a masked column"
```

## Self-review

- **Against the design.** B.4's rules 1 to 6 and F.2's masking rows, but rule 5's resolver, as "What the plan asks, and what is built" says; C.3's stop for a renamed column, as settled on 2026-10-17. The values and the engines' own fences are left to run 1b, and said so.
- **A review before the plan.** A reviewer read the draft (its run was cut short before it ran anything) and found an entry that names a view's column passing the stop and masking nothing on the view: fixed, the view's column masked as a table's, with a test. Its other two are kept: a masked column named in another schema is taken for one in every schema by the reading, which errs toward refusing and is said in B.4; and the catalog's columns are found by a folded name, a scan of `pg_class` and `pg_attribute` that the relations' own query already makes on every call. Then the statements were probed here, on all three: a masked column's alias read in SQLite's `WHERE` and in SQLite's and MySQL's `ORDER BY` expressions, and SQLite's `ORDER BY (2)` and `+2` as places, both leaks, both fixed with tests; the views (a whole row, a column read in a `WHERE`, a materialized view, a view over a view, one that reads only `id`) were run against PostgreSQL, MySQL and SQLite and answered as this plan says.
- **Placeholders.** None: every step has its diff or its command and what it prints.
- **Names.** `ColumnName`, `MaskedUse`, `Refusal::MaskedColumnUsed`, `Hidden::masking`, `guards_anything`, `masks`, `check_column_read`, `Catalog::columns`, `Catalog::column_reads`, `ColumnRead` and `Relation::view` are the same in every task that uses them.
