# MCP server, Phase 3a, run 4: the editor's Masked columns control. Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** The connection editor's Agents part gains **Masked columns**, in both looks: the columns whose values agents get as `•••`, as chips with a ×, and **Add column…**, whose list offers the tables an open window's sidebar loaded and then the columns of the one chosen, fetched from that window. A masked column the database does not have is marked "not in this database". What the person changes is theirs, as their tables are. The website's guide says how to mask a column.

**Architecture:** `LoadedTables` (`src/model.rs`) gains the columns the windows fetched, and says what Add column… offers (`offered_columns`, as `OfferedColumn` rows) and whether a masked column is the database's (`has_column`, as the agent server's check reads it). `ConnectionForm` gains the control's field, its pick and its error, and `take_typed_column`. `src/ui/connect_dialog/agents.rs` draws the control beside Tables on macOS and as the `masked` row on Omarchy, sharing the Tables control's list, chips and keys. The dialog asks an open window for the columns it needs (`App::load_agent_columns`, through the SQL editor's `load_columns`), and `App::save_dialog` takes a changed list as the person's.

**Tech Stack:** Rust 2024, egui.

---

## Where this sits

Phase 3a of `docs/superpowers/plans/2026-10-13-mcp-server.md`, masked columns, in five runs. Runs 1a, 1b and 2 (pull requests 150, 152 and 153) built the checks and the server's side, and run 3 (pull request 157) the setting in `connections.toml`, which the editor carried as it was. This run, 4, is the last of 3a: the editor's control and the website's guide, as 1c-3a and 1c-3b were for tables. 3b, the Log view in full, comes next.

## What the plan asks, and what is built

| The plan | Built | Why |
|---|---|---|
| D.3: "Add column…": a list from the connection's catalog when a session is open, typed names when none is | The tables an open window's sidebar loaded, then the columns of the one chosen, asked of that window as the SQL editor's completion asks | As Add table…'s list (1c-3b): what the windows know, and no new reading of the catalog. Agreed on 2026-10-18 |
| The board: "The agent sees the name and type; values come back as •••." (macOS), "name and type only, values •••" (Omarchy) | "Agents see the name and type; values come back as •••. A query that filters, joins, sorts or computes with one is refused, in any table with a column of that name." In both looks | What is refused, said where it is set, as the Tables hint does. Agreed on 2026-10-18 |
| The boards draw no list, no mark and no line under it | As Add table…'s: the Components boards' picker, the mark in the chip, and the line of C.3 | One control for the two lists |
| The macOS board: Tables and Masked columns side by side | As drawn | Agreed on 2026-10-18 |
| The Omarchy board: `h`/`l` in the footer | The row works as the editor's rows do today: Tab and Enter or Space | As the Tables row (1c-3a) |

## Before you start

- Work on a branch of `main`. One commit for each task, in the order below. Every commit passes `cargo fmt --all --check`, `cargo clippy --locked --workspace --all-targets -- -D warnings` and `cargo test --locked --workspace --all-targets`.
- On Igor's machine `cargo` is `~/.cargo/bin/cargo` (the mise shim fails), and a build in a worktree needs `CARGO_TARGET_DIR` set to that worktree's `target/`.
- Read `AGENTS.md`. The Agents part shows only where agents can reach the app (`cfg(any(unix, windows))`). The diffs below apply with `git apply`. Run `cargo fmt --all` before each commit: the code here is already formatted.

## What was run before this plan was written

Every task's state was built here, passed `cargo fmt --check` and clippy with `-D warnings`, and its tests failed and then passed as each step says. The last state passed `cargo test --locked --workspace --all-targets` with both test servers (3,599 tests, none failed), `RUSTDOCFLAGS='-D warnings' cargo doc --locked --workspace --no-deps` and `docs/bin/check`. Not run: CI, and the control by hand against a database: its place beside Tables, the list and the mark were looked at by eye in headless renders of both looks.

## What this run settles

| Question | Decision | Why |
|---|---|---|
| Where Add column…'s list comes from | The tables an open window's sidebar loaded, then the columns of the one chosen, asked of that window as the SQL editor's completion asks | As Add table…'s (1c-3b): what the windows know. Agreed on 2026-10-18 |
| How a table is chosen | A row `orders.`; Enter or a click puts it in the field, the caret after it; Tab or a click away takes what is typed | A table is a step, not an entry |
| What a name without a schema offers | The columns of every schema's table of that name, written without a schema | Such an entry masks that column in every schema |
| When a masked column is marked | Only where what was loaded and fetched says the database has no such column: the table is not in a loaded schema, or its columns were fetched and none is it; a name without a schema only when every schema is loaded | As the agent server's check (`Hidden::check_settings`); what is not known is never marked |
| What the list says of a table's columns | "Columns of orders", "Loading the columns of orders…" while a connected window fetches them, "The columns of orders could not be read" | A failed fetch is no table without columns (found by the review) |
| Which columns are asked for | Those of each table a masked column names, when the form shows it, and of the table typed | The mark needs them; the list needs one table's |
| The layout on macOS | Beside Tables, as the board draws them | Agreed on 2026-10-18 |
| The hint | "Agents see the name and type; values come back as •••. A query that filters, joins, sorts or computes with one is refused, in any table with a column of that name." In both looks | Agreed on 2026-10-18 |
| The save | A changed list is the person's and shown; one left as opened keeps the file's; a change of the person's is refused where the file masks more since the form opened, and the form shows what is in force | As the tables (1c-3a) |

### What this run leaves out

- A list of the columns of every table at once: a table is chosen first, so that one window is asked for one table's columns.
- `h`/`l` on Omarchy's rows, as for Tables.
- A hidden column of a SQLite virtual table (an FTS5 table's own): the window's columns leave it out and the agent server counts it, so a masked one is marked "not in this database" wrongly (found by the review).

## File structure

| File | What changes |
|---|---|
| `src/model.rs` | `LoadedTables::columns`, `named`, `has_column`, `offered_columns`; `OfferedColumn`, `ColumnsOffered`; the form's field, pick and error, `add_masked`, `take_typed_column` |
| `src/app.rs` | `loaded_tables` with the columns fetched; `load_agent_columns`; the editor's save |
| `src/agents_shown.rs` | `same_columns`, for the save |
| `src/ui/connect_dialog/agents.rs` | The control, in both looks; the list, the chips and the keys shared with Tables |
| `src/ui/connect_dialog/mod.rs` | The columns asked for |
| `src/keymap.rs` | Up and Down's name |
| `src/ui/agents_tests.rs` | The control's tests |
| `docs/_guide/agents.md`, `docs/_reference/connections.md`, `docs/_guide/macos.md`, `docs/_reference/keyboard-shortcuts.md` | Mask columns; the control; the keys |
| `docs/superpowers/plans/2026-10-13-mcp-server.md` | B.4, D.3 and G as built |

---

### Task 1: What Add column… offers, and which masked column is in no table

- `LoadedTables::columns`, `columns_loading` and `columns_failed`: the columns the windows fetched, by table, and the tables whose columns are on their way or could not be fetched.
- `LoadedTables::named`: the loaded tables a name stands for, one in every schema for a name without one. `has_column`: whether a masked column is the database's, where what was loaded and fetched can say, as the agent server's check reads it (a table's own columns, never those of one whose name differs in case): a table not in the database has none; a table whose columns were not fetched, or a schema not loaded, says nothing; a name without a schema is missing only when every schema is loaded.
- `LoadedTables::offered_columns` (`ColumnsOffered`, `OfferedColumn`): for `orders.em` (read as the file reads names, quotes and all), where `orders` is loaded, its columns that match, none an entry masks already; else the loaded tables that match, each written `orders.`.
- `ConnectionForm`: the masked columns as opened, Add column…'s field, its pick and its error; `add_masked` (once, in any case) and `take_typed_column` (the column chosen, or what is typed; a table chosen is no column).

**Files:**
- Modify: `src/model.rs`

- [ ] **Step 1: Write the failing test**

````diff
diff --git a/src/model.rs b/src/model.rs
index 6480512..edef720 100644
--- a/src/model.rs
+++ b/src/model.rs
@@ -5599,6 +5599,136 @@ mod tests {
         assert!(back.ssh_secret_is_stale(), "another port, another server");
     }
 
+    #[test]
+    fn add_column_offers_tables_then_their_columns_and_marks_a_column_in_none() {
+        use tabletist_db::agent::ColumnName;
+        let entry = |written: &str| -> ColumnName { written.parse().unwrap() };
+        let rows = |offered: ColumnsOffered| -> Vec<String> {
+            offered.rows.iter().map(ToString::to_string).collect()
+        };
+        let loaded = LoadedTables {
+            driver: Some(Driver::Postgres),
+            home: Some("public".into()),
+            schemas: vec!["public".into(), "sales".into(), "audit".into()],
+            schemas_cut: false,
+            loaded: vec![
+                ("public".into(), vec!["customers".into(), "orders".into()]),
+                ("sales".into(), vec!["orders".into()]),
+            ],
+            columns: vec![
+                (
+                    ObjectRef::new("public", "orders"),
+                    vec!["id".into(), "email".into(), "phone".into()],
+                ),
+                (
+                    ObjectRef::new("sales", "orders"),
+                    vec!["id".into(), "email".into(), "total".into()],
+                ),
+            ],
+            columns_loading: vec![ObjectRef::new("public", "customers")],
+            columns_failed: Vec::new(),
+        };
+        // A table typed: the loaded ones that match, each to be chosen for
+        // its columns.
+        assert_eq!(
+            rows(loaded.offered_columns("ord", &[])),
+            ["public.orders.", "sales.orders."]
+        );
+        assert_eq!(rows(loaded.offered_columns("cust", &[])), ["customers."]);
+        // A table and a dot: its columns, every schema's for a name
+        // without one, written as typed; what follows the dot narrows them.
+        let offered = loaded.offered_columns("orders.", &[]);
+        assert_eq!(
+            offered.tables,
+            [
+                ObjectRef::new("public", "orders"),
+                ObjectRef::new("sales", "orders")
+            ]
+        );
+        assert_eq!(
+            rows(offered),
+            ["orders.email", "orders.id", "orders.phone", "orders.total"]
+        );
+        assert_eq!(
+            rows(loaded.offered_columns("sales.orders.to", &[])),
+            ["sales.orders.total"]
+        );
+        // What an entry masks already is not offered: one without a
+        // schema masks every schema's.
+        assert_eq!(
+            rows(loaded.offered_columns("orders.", &[entry("ORDERS.email")])),
+            ["orders.id", "orders.phone", "orders.total"]
+        );
+        assert_eq!(
+            rows(loaded.offered_columns("orders.em", &[entry("sales.orders.email")])),
+            ["orders.email"]
+        );
+        // A table whose columns are on their way offers none yet.
+        let waiting = loaded.offered_columns("customers.", &[]);
+        assert!(waiting.rows.is_empty());
+        assert_eq!(waiting.tables, [ObjectRef::new("public", "customers")]);
+        // A column of a fetched table: known either way. A table not in
+        // the database has none; one whose columns are not fetched, or in
+        // a schema not loaded, is not known.
+        assert_eq!(loaded.has_column(&entry("orders.email")), Some(true));
+        assert_eq!(loaded.has_column(&entry("sales.orders.phone")), Some(false));
+        assert_eq!(loaded.has_column(&entry("public.orders.mail")), Some(false));
+        assert_eq!(
+            loaded.has_column(&entry("sales.clients.email")),
+            Some(false)
+        );
+        assert_eq!(loaded.has_column(&entry("customers.email")), None);
+        assert_eq!(loaded.has_column(&entry("audit.log.who")), None);
+        // Without a schema: any schema's table of that name, so missing
+        // only when every schema is loaded.
+        assert_eq!(loaded.has_column(&entry("orders.mail")), None);
+        let whole = LoadedTables {
+            schemas: vec!["public".into(), "sales".into()],
+            ..loaded.clone()
+        };
+        assert_eq!(whole.has_column(&entry("orders.mail")), Some(false));
+        assert_eq!(whole.has_column(&entry("orders.TOTAL")), Some(true));
+        // The system's are never marked.
+        assert_eq!(whole.has_column(&entry("pg_catalog.pg_class.oid")), None);
+        // Two tables whose names differ in case are two tables: the
+        // columns of one are not the other's. An entry names both, in any
+        // case, as the agent server reads it.
+        let cased = LoadedTables {
+            driver: Some(Driver::Postgres),
+            home: Some("public".into()),
+            schemas: vec!["public".into()],
+            loaded: vec![("public".into(), vec!["Orders".into(), "orders".into()])],
+            columns: vec![(ObjectRef::new("public", "orders"), vec!["id".into()])],
+            ..LoadedTables::default()
+        };
+        assert_eq!(cased.has_column(&entry("public.Orders.email")), None);
+        assert_eq!(cased.has_column(&entry("orders.email")), None);
+        assert_eq!(cased.has_column(&entry("public.orders.email")), None);
+        let both = LoadedTables {
+            columns: vec![
+                (ObjectRef::new("public", "orders"), vec!["id".into()]),
+                (ObjectRef::new("public", "Orders"), vec!["email".into()]),
+            ],
+            ..cased.clone()
+        };
+        assert_eq!(both.has_column(&entry("public.orders.email")), Some(true));
+        assert_eq!(both.has_column(&entry("orders.phone")), Some(false));
+        // A table whose name holds a dot is read in its quotes.
+        let dotted = LoadedTables {
+            driver: Some(Driver::Postgres),
+            home: Some("public".into()),
+            schemas: vec!["public".into()],
+            loaded: vec![("public".into(), vec!["my.table".into()])],
+            columns: vec![(ObjectRef::new("public", "my.table"), vec!["note".into()])],
+            ..LoadedTables::default()
+        };
+        assert_eq!(rows(dotted.offered_columns("my", &[])), ["\"my.table\"."]);
+        assert_eq!(
+            rows(dotted.offered_columns("\"my.table\".", &[])),
+            ["\"my.table\".note"]
+        );
+    }
+
     #[test]
     fn what_a_window_loaded_is_offered_and_marks_a_name_in_no_table() {
         let names = |written: &[&str]| -> Vec<tabletist_db::agent::TableName> {
@@ -5620,6 +5750,7 @@ mod tests {
                 ),
                 ("sales".into(), vec!["orders".into(), "customers".into()]),
             ],
+            ..LoadedTables::default()
         };
         // A home table whose name no other schema has goes without its
         // schema; one another schema shares, or under "Only", with it; and
@@ -5689,6 +5820,7 @@ mod tests {
             schemas: vec!["main".into()],
             schemas_cut: false,
             loaded: vec![("main".into(), vec!["books".into()])],
+            ..LoadedTables::default()
         };
         assert_eq!(sqlite.has(&name("sqlite_stat1")), None);
         assert_eq!(sqlite.has(&name("dbstat")), None);
````

- [ ] **Step 2: Run it to see that it does not build**

Run: `cargo test --locked --lib -- add_column_offers_tables what_a_window_loaded`

Expected: it does not build:

```text
error[E0425]: cannot find type `ColumnsOffered` in this scope
error[E0560]: struct `model::LoadedTables` has no field named `columns`
error[E0599]: no method named `offered_columns` found for struct `model::LoadedTables` in the current scope
error[E0599]: no method named `has_column` found for struct `model::LoadedTables` in the current scope
error: could not compile `tabletist` (lib test) due to 31 previous errors
```

- [ ] **Step 3: The columns, offered and checked**

````diff
diff --git a/src/model.rs b/src/model.rs
index edef720..45c5ce7 100644
--- a/src/model.rs
+++ b/src/model.rs
@@ -1120,6 +1120,45 @@ pub struct LoadedTables {
     pub schemas_cut: bool,
     /// Each loaded schema with its tables' and views' names.
     pub loaded: Vec<(String, Vec<String>)>,
+    /// The tables and views whose columns a window fetched, each with its
+    /// schema and its columns' names.
+    pub columns: Vec<(ObjectRef, Vec<String>)>,
+    /// The tables and views whose columns are on their way.
+    pub columns_loading: Vec<ObjectRef>,
+    /// The tables and views whose columns could not be fetched.
+    pub columns_failed: Vec<ObjectRef>,
+}
+
+/// A row of Add column…'s list: a table, to be chosen for its columns, or
+/// one of its columns, to be masked.
+#[derive(Debug, Clone, PartialEq, Eq)]
+pub enum OfferedColumn {
+    Table(tabletist_db::agent::TableName),
+    Column(tabletist_db::agent::ColumnName),
+}
+
+/// As the row says it: `orders.` for a table, as the field takes it to
+/// list its columns, and `orders.email` for a column.
+impl std::fmt::Display for OfferedColumn {
+    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
+        match self {
+            Self::Table(table) => write!(f, "{table}."),
+            Self::Column(column) => write!(f, "{column}"),
+        }
+    }
+}
+
+/// What Add column…'s list shows for what is typed: its rows, and the
+/// tables whose columns they are, where the text names a loaded one.
+#[derive(Debug, Clone, Default, PartialEq, Eq)]
+pub struct ColumnsOffered {
+    pub rows: Vec<OfferedColumn>,
+    /// The table the text names, as typed, where it names a loaded one:
+    /// then the rows are its columns.
+    pub of: Option<tabletist_db::agent::TableName>,
+    /// The loaded tables that name stands for: one in every schema for a
+    /// name without one.
+    pub tables: Vec<ObjectRef>,
 }
 
 /// How many tables Add table…'s list shows at once.
@@ -1249,25 +1288,183 @@ impl LoadedTables {
                 if self.loaded.iter().any(|(_, names)| in_schema(names)) {
                     return Some(true);
                 }
-                let all = !self.schemas.is_empty()
-                    && !self.schemas_cut
-                    && self.schemas.iter().all(|schema| {
-                        self.loaded
-                            .iter()
-                            .any(|(loaded, names)| same(loaded, schema) && whole(names.len()))
-                    });
-                all.then_some(false)
+                self.whole().then_some(false)
             }
         }
     }
+
+    /// Whether every schema of the database is loaded, each wholly: then
+    /// what no loaded schema has, no schema has.
+    fn whole(&self) -> bool {
+        let whole = |count: usize| count < tabletist_db::MAX_LISTED as usize;
+        !self.schemas.is_empty()
+            && !self.schemas_cut
+            && self.schemas.iter().all(|schema| {
+                self.loaded
+                    .iter()
+                    .any(|(loaded, names)| same_name(loaded, schema) && whole(names.len()))
+            })
+    }
+
+    /// The loaded tables and views `schema` and `table` name: the one of
+    /// that schema, or, without a schema, the one of that name in each.
+    /// None of the system's.
+    pub fn named(&self, schema: Option<&str>, table: &str) -> Vec<ObjectRef> {
+        let name = tabletist_db::agent::TableName {
+            schema: schema.map(str::to_owned),
+            name: table.to_owned(),
+        };
+        if self.system_table(&name) {
+            return Vec::new();
+        }
+        self.loaded
+            .iter()
+            .filter(|(loaded, _)| !self.system_schema(loaded))
+            .filter(|(loaded, _)| schema.is_none_or(|schema| same_name(schema, loaded)))
+            .flat_map(|(loaded, names)| {
+                names
+                    .iter()
+                    .filter(|known| same_name(known, table))
+                    .map(move |known| ObjectRef::new(loaded.clone(), known.clone()))
+            })
+            .collect()
+    }
+
+    /// The columns a window fetched of `object`, where it fetched them:
+    /// that table's own, by its name as the catalog has it, and not those
+    /// of one whose name differs only in case.
+    fn columns_of(&self, object: &ObjectRef) -> Option<&[String]> {
+        self.columns
+            .iter()
+            .find(|(known, _)| known == object)
+            .map(|(_, columns)| columns.as_slice())
+    }
+
+    /// Whether `entry` names a column of the database, where what the
+    /// windows loaded and fetched can say: `Some(false)` marks it, as the
+    /// agent server finds it missing. A table not in the database has no
+    /// column; one whose columns were not fetched is not known; an entry
+    /// without a schema is the table of that name in any schema, so it is
+    /// missing only when every schema is loaded.
+    pub fn has_column(&self, entry: &tabletist_db::agent::ColumnName) -> Option<bool> {
+        let table = tabletist_db::agent::TableName {
+            schema: entry.schema.clone(),
+            name: entry.table.clone(),
+        };
+        if !self.has(&table)? {
+            return Some(false);
+        }
+        let mut known = true;
+        for object in self.named(entry.schema.as_deref(), &entry.table) {
+            match self.columns_of(&object) {
+                Some(columns) if columns.iter().any(|name| same_name(name, &entry.column)) => {
+                    return Some(true);
+                }
+                Some(_) => {}
+                None => known = false,
+            }
+        }
+        (known && (entry.schema.is_some() || self.whole())).then_some(false)
+    }
+
+    /// Add column…'s list for `typed`. A table and a dot, `orders.em`,
+    /// where the table is loaded: its columns that match what follows the
+    /// dot, written with the table as typed (with its schema where one is
+    /// typed), none an entry of `listed` already masks. Else the loaded
+    /// tables that match, as Add table… offers them, each to be chosen for
+    /// its columns. At most [`OFFERED`] rows; nothing for nothing typed.
+    pub fn offered_columns(
+        &self,
+        typed: &str,
+        listed: &[tabletist_db::agent::ColumnName],
+    ) -> ColumnsOffered {
+        use tabletist_db::agent::{ColumnName, TableName};
+        // Read as the file reads a name, quotes and all: a table and a dot,
+        // or a table, a dot and the start of a column.
+        let typed_now = typed.trim();
+        let parts = match typed_now.strip_suffix('.') {
+            Some(front) => front
+                .parse::<TableName>()
+                .ok()
+                .map(|of| (of, String::new())),
+            None => typed_now.parse::<ColumnName>().ok().map(|entry| {
+                let of = TableName {
+                    schema: entry.schema,
+                    name: entry.table,
+                };
+                (of, entry.column)
+            }),
+        };
+        if let Some((of, rest)) = parts {
+            let rest = rest.as_str();
+            let tables = self.named(of.schema.as_deref(), &of.name);
+            if !tables.is_empty() {
+                let masks = |column: &str| {
+                    listed.iter().any(|entry| {
+                        same_name(&entry.table, &of.name)
+                            && same_name(&entry.column, column)
+                            && match (&entry.schema, &of.schema) {
+                                (None, _) => true,
+                                (Some(own), Some(schema)) => same_name(own, schema),
+                                (Some(_), None) => false,
+                            }
+                    })
+                };
+                let mut seen = HashSet::new();
+                let mut scored: Vec<(i64, String, String)> = tables
+                    .iter()
+                    .filter_map(|table| self.columns_of(table))
+                    .flatten()
+                    .filter(|column| seen.insert(column.to_lowercase()))
+                    .filter(|column| !masks(column))
+                    .filter_map(|column| {
+                        let score = match rest.is_empty() {
+                            true => 0,
+                            false => crate::util::fuzzy_score(rest, column)?,
+                        };
+                        Some((score, column.to_lowercase(), column.clone()))
+                    })
+                    .collect();
+                scored.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.cmp(&b.1)));
+                scored.truncate(OFFERED);
+                // The table's own name, as the catalog has it.
+                let table = tables[0].name.clone();
+                let schema = of.schema.as_ref().map(|_| tables[0].schema.clone());
+                let rows = scored
+                    .into_iter()
+                    .map(|(_, _, column)| {
+                        OfferedColumn::Column(ColumnName {
+                            schema: schema.clone(),
+                            table: table.clone(),
+                            column,
+                        })
+                    })
+                    .collect();
+                return ColumnsOffered {
+                    rows,
+                    of: Some(of),
+                    tables,
+                };
+            }
+        }
+        ColumnsOffered {
+            rows: self
+                .offered(typed, &[], false)
+                .into_iter()
+                .map(OfferedColumn::Table)
+                .collect(),
+            ..ColumnsOffered::default()
+        }
+    }
 }
 
-/// Where the Tables control puts the keyboard on the next frame.
+/// Where the Tables or the Masked columns control puts the keyboard on
+/// the next frame.
 #[derive(Debug, Clone, Copy, PartialEq, Eq)]
 pub enum TableFocus {
     /// The field for a name: opened, or still open after a name.
     Field,
-    /// Add table…, once the field is closed with Escape.
+    /// Add table… or Add column…, once the field is closed with Escape.
     Add,
 }
 
@@ -1294,17 +1491,30 @@ pub struct ConnectionForm {
     /// The tables in force when the dialog opened: others are the
     /// person's own, and the same keep what the file says.
     pub agents_tables_opened: tabletist_db::agent::Tables,
-    /// The masked columns in force when the dialog opened, carried to the
-    /// save as they are: the editor has no control for them yet.
+    /// The Masked columns control: the columns whose values agents get
+    /// as `•••`.
     pub agents_masked: Vec<tabletist_db::agent::ColumnName>,
+    /// The masked columns in force when the dialog opened: others are the
+    /// person's own, and the same keep what the file says.
+    pub agents_masked_opened: Vec<tabletist_db::agent::ColumnName>,
+    /// The name being typed after Add column…, while its field is open.
+    pub agents_column_typed: Option<String>,
+    /// Why the column's name typed does not read, said under the names.
+    pub agents_column_error: Option<String>,
+    /// Where the Masked columns control puts the keyboard on the next
+    /// frame.
+    pub agents_column_focus: Option<TableFocus>,
+    /// The row of Add column…'s list the keyboard is on, by its place.
+    pub agents_column_pick: Option<usize>,
     /// The name being typed after Add table…, while its field is open.
     pub agents_table_typed: Option<String>,
     /// Why the name typed does not read, said under the names.
     pub agents_table_error: Option<String>,
     /// Where the Tables control puts the keyboard on the next frame.
     pub agents_table_focus: Option<TableFocus>,
-    /// What an open window of the connection has loaded, for Add table…'s
-    /// list and the mark of a name in no table; `None` with none open.
+    /// What an open window of the connection has loaded, for the lists of
+    /// Add table… and Add column…, and the mark of a name in no table or
+    /// column; `None` with none open.
     pub agents_loaded: Option<LoadedTables>,
     /// The table of Add table…'s list the keyboard is on, by its place.
     pub agents_table_pick: Option<usize>,
@@ -1431,6 +1641,11 @@ impl Default for ConnectionForm {
             agents_tables: TablesChoice::default(),
             agents_tables_opened: tabletist_db::agent::Tables::All,
             agents_masked: Vec::new(),
+            agents_masked_opened: Vec::new(),
+            agents_column_typed: None,
+            agents_column_error: None,
+            agents_column_focus: None,
+            agents_column_pick: None,
             agents_table_typed: None,
             agents_table_error: None,
             agents_table_focus: None,
@@ -1584,6 +1799,11 @@ impl ConnectionForm {
             agents_tables: TablesChoice::of(&saved.agents.tables),
             agents_tables_opened: saved.agents.tables.clone(),
             agents_masked: saved.agents.masked.clone(),
+            agents_masked_opened: saved.agents.masked.clone(),
+            agents_column_typed: None,
+            agents_column_error: None,
+            agents_column_focus: None,
+            agents_column_pick: None,
             agents_table_typed: None,
             agents_table_error: None,
             agents_table_focus: None,
@@ -1888,6 +2108,61 @@ impl ConnectionForm {
         }
     }
 
+    /// Masks the column `written` names, where no entry masks it already
+    /// as written, in any case; `Err` with why it does not read.
+    pub fn add_masked(&mut self, written: &str) -> Result<(), String> {
+        let entry: tabletist_db::agent::ColumnName = written.trim().parse()?;
+        let key = |entry: &tabletist_db::agent::ColumnName| entry.to_string().to_lowercase();
+        if !self
+            .agents_masked
+            .iter()
+            .any(|known| key(known) == key(&entry))
+        {
+            self.agents_masked.push(entry);
+        }
+        Ok(())
+    }
+
+    /// Add column…'s list for what its field holds.
+    pub fn columns_offered(&self, typed: &str) -> ColumnsOffered {
+        self.agents_loaded
+            .as_ref()
+            .map(|loaded| loaded.offered_columns(typed, &self.agents_masked))
+            .unwrap_or_default()
+    }
+
+    /// Takes into the Masked columns control a name typed and not yet
+    /// added, as a Save does: added, or `Err` with why it does not read,
+    /// which is then also said under the names. The column chosen in Add
+    /// column…'s list is added in place of what is typed; a table chosen
+    /// there is no column, and what is typed is taken.
+    pub fn take_typed_column(&mut self) -> Result<(), String> {
+        let Some(mut typed) = self.agents_column_typed.clone() else {
+            return Ok(());
+        };
+        if let Some(at) = self.agents_column_pick.take()
+            && let Some(OfferedColumn::Column(picked)) =
+                self.columns_offered(&typed).rows.into_iter().nth(at)
+        {
+            typed = picked.to_string();
+        }
+        if typed.trim().is_empty() {
+            self.agents_column_typed = None;
+            return Ok(());
+        }
+        match self.add_masked(&typed) {
+            Ok(()) => {
+                self.agents_column_typed = None;
+                self.agents_column_error = None;
+                Ok(())
+            }
+            Err(why) => {
+                self.agents_column_error = Some(why.clone());
+                Err(why)
+            }
+        }
+    }
+
     /// What agents may do, as the fields say it: the rows and the timeout
     /// in the ranges a connection may be given.
     pub fn agents_to_saved(&self) -> Result<crate::connections::Agents, String> {
````

- [ ] **Step 4: Run the tests**

Run: `cargo test --locked --lib -- add_column_offers_tables what_a_window_loaded`

Expected:

```text
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 2588 filtered out
```

- [ ] **Step 5: Commit**

```bash
cargo fmt --all
cargo clippy --locked --workspace --all-targets -- -D warnings
git add src/model.rs
git commit -m "Say what Add column… offers, and which masked column is in no table"
```

---

### Task 2: The editor's Masked columns control

- `src/ui/connect_dialog/agents.rs`: Masked columns beside Tables on macOS and the `masked` row on Omarchy: the chips with their ×, **Add column…** and its field, its list, the mark and its line, the error, and the hint. The list, the chips, Up and Down and the form's Cancel are the Tables control's, shared (`offered_list`, `chip`, `moved`, `cancel_asked`, `released_on`). Enter or a click on a table puts it in the field, the caret after it. The list's note says whose columns they are, that they are on their way, or that they could not be read. The row chosen follows its value as the rows change (`follow_pick`), in both lists.
- `App::loaded_tables` gathers the columns the windows fetched, and which are on their way (while the window's session is there) or failed; `App::load_agent_columns` asks an open window for those of the tables the masked columns name and of the one typed, through the SQL editor's `load_columns`: once, and a failed one not again.
- `App::save_dialog`: a name typed and not added is added, or refuses the save; masked columns the person changed are theirs and shown; ones left as opened keep what the file says; a change of the person's over one made in the file while the form was open is refused, and the form shows them as they are now.
- `FormTableMove` is named for both lists.

**Files:**
- Modify: `src/app.rs`, `src/agents_shown.rs`, `src/keymap.rs`, `src/ui/connect_dialog/agents.rs`, `src/ui/connect_dialog/mod.rs`, `src/ui/agents_tests.rs`

- [ ] **Step 1: Write the failing tests**

````diff
diff --git a/src/app.rs b/src/app.rs
index 9f3b055..6fea333 100644
--- a/src/app.rs
+++ b/src/app.rs
@@ -7435,7 +7435,7 @@ mod tests {
         app.apply(Action::KeepAgentChanges(app.agents_held.clone()));
         assert!(app.agents_held.is_empty());
         assert_eq!(sent(&app), Vec::<ColumnName>::new());
-        // The editor has no control for them: its save keeps what the file
+        // An editor that leaves them as it opened them keeps what the file
         // says when it saves.
         app.apply(Action::EditConnection(conn.clone()));
         file_says(&mut app, email.clone());
diff --git a/src/ui/agents_tests.rs b/src/ui/agents_tests.rs
index fb856d7..80bde1d 100644
--- a/src/ui/agents_tests.rs
+++ b/src/ui/agents_tests.rs
@@ -1521,6 +1521,420 @@ fn add_table_offers_the_windows_tables_and_marks_a_name_in_none() {
     }
 }
 
+/// `name` typed for Add column… (its field opened where it is not open
+/// already), then Enter.
+#[cfg(any(unix, windows))]
+fn add_column(harness: &mut Harness, look: &Look, name: &str) {
+    type_column(harness, look, name);
+    harness.press(egui::Key::Enter, egui::Modifiers::NONE);
+    harness.settle();
+}
+
+/// `name` typed in Add column…'s field, opened where it is not open.
+#[cfg(any(unix, windows))]
+fn type_column(harness: &mut Harness, look: &Look, name: &str) {
+    if form(harness).agents_column_typed.is_none() {
+        harness.click(&look.label("Add column…"));
+    }
+    harness.settle();
+    harness.frame(vec![egui::Event::Text(name.into())]);
+    harness.settle();
+}
+
+/// How many times the backend was asked for the columns of `table`.
+#[cfg(any(unix, windows))]
+fn asked_to_describe(harness: &Harness, table: &str) -> usize {
+    use crate::backend::Command;
+    let sent = harness.app.backend.sent.iter();
+    sent.filter(
+        |command| matches!(command, Command::Describe { object, .. } if object.name == table),
+    )
+    .count()
+}
+
+/// Answers the newest request for the columns of `table`: `columns`.
+#[cfg(any(unix, windows))]
+fn describe(harness: &mut Harness, table: &str, columns: &[&str]) {
+    use crate::backend::Command;
+    let sent = harness.app.backend.sent.iter().rev();
+    let (session, request) = sent
+        .filter_map(|command| match command {
+            Command::Describe {
+                session,
+                request,
+                object,
+            } if object.name == table => Some((*session, *request)),
+            _ => None,
+        })
+        .next()
+        .unwrap_or_else(|| panic!("no Describe of {table} was sent"));
+    let structure = tabletist_db::Structure {
+        columns: columns
+            .iter()
+            .map(|name| tabletist_db::ColumnInfo {
+                name: (*name).into(),
+                type_name: "TEXT".into(),
+                ..Default::default()
+            })
+            .collect(),
+        ..Default::default()
+    };
+    harness.app.apply(Action::Backend(Event::Structure {
+        session,
+        request,
+        result: Ok(structure),
+    }));
+    harness.settle();
+}
+
+#[cfg(any(unix, windows))]
+#[test]
+fn the_editors_masked_columns_control_masks_what_agents_get_as_dots() {
+    let columns = |written: &[&str]| -> Vec<tabletist_db::agent::ColumnName> {
+        written.iter().map(|entry| entry.parse().unwrap()).collect()
+    };
+    let hint = "Agents see the name and type; values come back as •••. A query that filters, \
+                joins, sorts or computes with one is refused, in any table with a column of \
+                that name.";
+    let no_table = "The column's name has no table: write the table, a dot, and the column.";
+    let too_many = "The column's name has more than two dots: it is a table and a column, or a \
+                    schema, a table and a column.";
+    for look in looks() {
+        let (mut harness, conn) = on_the_list(look);
+        at_agents(&mut harness, &conn);
+        assert!(harness.has(&look.label(hint)), "{}", look.name);
+        // Enter masks a column and keeps the field, with the keyboard, for
+        // the next.
+        add_column(&mut harness, &look, "orders.email");
+        add_column(&mut harness, &look, "sales.orders.phone");
+        assert_eq!(
+            form(&harness).agents_column_typed.as_deref(),
+            Some(""),
+            "{}",
+            look.name
+        );
+        // The same one again, in another case, is there once.
+        add_column(&mut harness, &look, "ORDERS.EMAIL");
+        // One that does not read is said under the names, and the field
+        // keeps it to be put right.
+        add_column(&mut harness, &look, "email");
+        assert!(harness.has(&look.label(no_table)), "{}", look.name);
+        assert_eq!(
+            form(&harness).agents_column_typed.as_deref(),
+            Some("email"),
+            "{}",
+            look.name
+        );
+        // Escape closes the field and nothing else.
+        harness.press(egui::Key::Escape, egui::Modifiers::NONE);
+        harness.settle();
+        assert!(harness.app.dialog.is_some(), "{}", look.name);
+        assert_eq!(form(&harness).agents_column_typed, None, "{}", look.name);
+        assert!(!harness.has(&look.label(no_table)), "{}", look.name);
+        harness.click(&look.label("Remove sales.orders.phone"));
+        harness.settle();
+        // A name typed and not yet added is added by the save.
+        type_column(&mut harness, &look, "orders.phone");
+        harness.app.apply(Action::SaveConnection { connect: false });
+        let saved = harness.app.connections.get(&conn).unwrap();
+        assert_eq!(
+            saved.agents.masked,
+            columns(&["orders.email", "orders.phone"]),
+            "{}",
+            look.name
+        );
+        // The person's own: shown, so that comparing again holds nothing.
+        harness.app.check_agents();
+        assert!(harness.app.agents_held.is_empty(), "{}", look.name);
+        // Opened again it shows them. A name that does not read, typed and
+        // not added, refuses the save and says why.
+        at_agents(&mut harness, &conn);
+        assert!(
+            harness.has(&look.label("Remove orders.phone")),
+            "{}",
+            look.name
+        );
+        type_column(&mut harness, &look, "a.b.c.d");
+        harness.app.apply(Action::SaveConnection { connect: false });
+        assert!(harness.app.dialog.is_some(), "{}", look.name);
+        assert_eq!(
+            form(&harness).message.as_deref(),
+            Some(too_many),
+            "{}",
+            look.name
+        );
+        form_mut(&mut harness).agents_column_typed = None;
+        form_mut(&mut harness).agents_column_error = None;
+        harness.settle();
+        // A name left in the field when Tab moves on is added.
+        type_column(&mut harness, &look, "orders.total");
+        harness.press(egui::Key::Tab, egui::Modifiers::NONE);
+        harness.settle();
+        assert!(
+            harness.has(&look.label("Remove orders.total")),
+            "{}",
+            look.name
+        );
+        assert_eq!(form(&harness).agents_column_typed, None, "{}", look.name);
+        // Masking less, by the person: theirs, and never held.
+        for name in ["orders.email", "orders.phone", "orders.total"] {
+            harness.click(&look.label(&format!("Remove {name}")));
+            harness.settle();
+        }
+        harness.app.apply(Action::SaveConnection { connect: false });
+        assert_eq!(
+            harness.app.connections.get(&conn).unwrap().agents.masked,
+            columns(&[]),
+            "{}",
+            look.name
+        );
+        harness.app.check_agents();
+        assert!(harness.app.agents_held.is_empty(), "{}", look.name);
+    }
+}
+
+#[cfg(any(unix, windows))]
+#[test]
+fn masked_columns_changed_outside_while_the_editor_is_open_are_not_saved_over() {
+    let columns = |written: &[&str]| -> Vec<tabletist_db::agent::ColumnName> {
+        written.iter().map(|entry| entry.parse().unwrap()).collect()
+    };
+    let (mut harness, conn) = on_the_list(Look::macos());
+    at_agents(&mut harness, &conn);
+    add_column(&mut harness, &Look::macos(), "orders.email");
+    // The file masks another column meanwhile.
+    let mut saved = harness.app.connections.get(&conn).unwrap().clone();
+    saved.agents.masked = columns(&["customers.phone"]);
+    harness.app.connections.upsert(saved);
+    harness.app.check_agents();
+    harness.app.apply(Action::SaveConnection { connect: false });
+    assert!(harness.app.dialog.is_some());
+    assert_eq!(form(&harness).agents_masked, columns(&["customers.phone"]));
+    assert!(
+        form(&harness)
+            .message
+            .as_deref()
+            .is_some_and(|said| said.starts_with("The masked columns were changed outside"))
+    );
+    // Saved again, it keeps what the file says.
+    harness.app.apply(Action::SaveConnection { connect: false });
+    assert_eq!(
+        harness.app.connections.get(&conn).unwrap().agents.masked,
+        columns(&["customers.phone"])
+    );
+}
+
+#[cfg(any(unix, windows))]
+#[test]
+fn add_column_offers_the_tables_then_the_columns_it_fetches_and_marks_one_in_none() {
+    let gone = |look: &Look| {
+        look.label(
+            "Agents may not use this connection while a masked column is not in its database. \
+             Take the name out, or add the column's new name.",
+        )
+    };
+    for look in looks() {
+        let (mut harness, tab) = connected(look);
+        loaded(&mut harness, tab, &["customers", "orders"]);
+        let conn = harness.app.workspace(tab).unwrap().conn_id.clone();
+        let mut saved = harness.app.connections.get(&conn).unwrap().clone();
+        saved.agents.masked = vec!["orders.mail".parse().unwrap()];
+        harness.app.connections.upsert(saved);
+        harness.app.check_agents();
+        at_agents(&mut harness, &conn);
+        // The columns of a table a masked column names are asked of the
+        // window, once; a column none of them is is marked, and what that
+        // does to agents is said.
+        assert_eq!(asked_to_describe(&harness, "orders"), 1, "{}", look.name);
+        let mark = look.label("orders.mail, not in this database");
+        assert!(!harness.has(&mark), "{}", look.name);
+        describe(&mut harness, "orders", &["id", "email", "phone"]);
+        assert!(harness.has(&mark), "{}", look.name);
+        assert!(harness.has(&gone(&look)), "{}", look.name);
+        assert_eq!(asked_to_describe(&harness, "orders"), 1, "{}", look.name);
+        // Typed, the window's tables that match are offered; Enter on one
+        // puts it in the field, and its columns are offered.
+        type_column(&mut harness, &look, "ord");
+        assert!(harness.has("orders."), "{}", look.name);
+        harness.press(egui::Key::ArrowDown, egui::Modifiers::NONE);
+        harness.press(egui::Key::Enter, egui::Modifiers::NONE);
+        harness.settle();
+        assert_eq!(
+            form(&harness).agents_column_typed.as_deref(),
+            Some("orders."),
+            "{}",
+            look.name
+        );
+        assert!(
+            harness.has(&look.label("Columns of orders")),
+            "{}",
+            look.name
+        );
+        harness.frame(vec![egui::Event::Text("em".into())]);
+        harness.settle();
+        harness.press(egui::Key::ArrowDown, egui::Modifiers::NONE);
+        harness.press(egui::Key::Enter, egui::Modifiers::NONE);
+        harness.settle();
+        assert!(
+            harness.has(&look.label("Remove orders.email")),
+            "{}",
+            look.name
+        );
+        assert_eq!(
+            form(&harness).agents_column_typed.as_deref(),
+            Some(""),
+            "{}",
+            look.name
+        );
+        // A table whose columns no window has: asked for, and the list
+        // says they are on their way; a click on one masks it.
+        type_column(&mut harness, &look, "customers.");
+        assert_eq!(asked_to_describe(&harness, "customers"), 1, "{}", look.name);
+        assert!(
+            harness.has(&look.label("Loading the columns of customers…")),
+            "{}",
+            look.name
+        );
+        describe(&mut harness, "customers", &["id", "name"]);
+        let row = harness
+            .text_rects
+            .iter()
+            .rev()
+            .find(|(text, _)| text == "customers.name")
+            .map(|(_, rect)| *rect)
+            .expect("the list's column");
+        press_at(&mut harness, row.center());
+        assert!(
+            harness.has(&look.label("Remove customers.name")),
+            "{}",
+            look.name
+        );
+        harness.press(egui::Key::Escape, egui::Modifiers::NONE);
+        harness.settle();
+        // Taken out, nothing is marked and nothing said.
+        harness.click(&look.label("Remove orders.mail"));
+        harness.settle();
+        assert!(!harness.has(&gone(&look)), "{}", look.name);
+    }
+}
+
+#[cfg(any(unix, windows))]
+#[test]
+fn add_columns_list_keeps_the_row_chosen_and_says_what_did_not_come() {
+    use crate::model::{Fetch, SessionStatus};
+    use tabletist_db::{ObjectInfo, ObjectKind, ObjectRef};
+    for look in looks() {
+        let (mut harness, tab) = connected(look);
+        // `orders` in two schemas: one's columns are here, the other's on
+        // their way.
+        let workspace = harness.app.workspace_mut(tab).unwrap();
+        workspace.tree.schemas.value = Some(vec!["main".into(), "other".into()]);
+        for schema in ["main", "other"] {
+            let node = workspace.tree.nodes.entry(schema.into()).or_default();
+            node.objects.value = Some(vec![ObjectInfo {
+                name: "orders".into(),
+                kind: ObjectKind::Table,
+                estimated_rows: None,
+            }]);
+        }
+        let fetched = Fetch {
+            value: Some(
+                ["id", "phone"]
+                    .iter()
+                    .map(|name| tabletist_db::ColumnInfo {
+                        name: (*name).into(),
+                        ..Default::default()
+                    })
+                    .collect(),
+            ),
+            ..Fetch::default()
+        };
+        workspace
+            .columns
+            .insert(ObjectRef::new("main", "orders"), fetched);
+        let conn = workspace.conn_id.clone();
+        at_agents(&mut harness, &conn);
+        type_column(&mut harness, &look, "orders.");
+        assert_eq!(asked_to_describe(&harness, "orders"), 1, "{}", look.name);
+        // `phone` chosen, then the other schema's columns come and put a
+        // row before it: Enter still masks `phone`.
+        harness.press(egui::Key::ArrowDown, egui::Modifiers::NONE);
+        harness.press(egui::Key::ArrowDown, egui::Modifiers::NONE);
+        assert_eq!(form(&harness).agents_column_pick, Some(1), "{}", look.name);
+        describe(&mut harness, "orders", &["email"]);
+        harness.press(egui::Key::Enter, egui::Modifiers::NONE);
+        harness.settle();
+        assert!(
+            harness.has(&look.label("Remove orders.phone")),
+            "{}",
+            look.name
+        );
+        // A table whose columns could not be read says so.
+        let workspace = harness.app.workspace_mut(tab).unwrap();
+        workspace
+            .tree
+            .nodes
+            .entry("main".into())
+            .or_default()
+            .objects
+            .value = Some(vec![ObjectInfo {
+            name: "carts".into(),
+            kind: ObjectKind::Table,
+            estimated_rows: None,
+        }]);
+        type_column(&mut harness, &look, "carts.");
+        let (session, request) = harness
+            .app
+            .backend
+            .sent
+            .iter()
+            .rev()
+            .find_map(|command| match command {
+                crate::backend::Command::Describe {
+                    session,
+                    request,
+                    object,
+                } if object.name == "carts" => Some((*session, *request)),
+                _ => None,
+            })
+            .expect("a Describe of carts");
+        harness.app.apply(Action::Backend(Event::Structure {
+            session,
+            request,
+            result: Err(tabletist_db::Error::Io("gone".into())),
+        }));
+        harness.settle();
+        assert!(
+            harness.has(&look.label("The columns of carts could not be read")),
+            "{}",
+            look.name
+        );
+        // Columns on their way from a window whose session is gone are on
+        // their way no more.
+        harness.press(egui::Key::Escape, egui::Modifiers::NONE);
+        harness.settle();
+        let workspace = harness.app.workspace_mut(tab).unwrap();
+        workspace
+            .tree
+            .nodes
+            .entry("main".into())
+            .or_default()
+            .objects
+            .value = Some(vec![ObjectInfo {
+            name: "books".into(),
+            kind: ObjectKind::Table,
+            estimated_rows: None,
+        }]);
+        type_column(&mut harness, &look, "books.");
+        let waiting = look.label("Loading the columns of books…");
+        assert!(harness.has(&waiting), "{}", look.name);
+        harness.app.workspace_mut(tab).unwrap().status =
+            SessionStatus::Disconnected(tabletist_db::Error::ConnectionLost("gone".into()));
+        harness.settle();
+        assert!(!harness.has(&waiting), "{}", look.name);
+    }
+}
+
 #[cfg(any(unix, windows))]
 #[test]
 fn allow_agents_opens_the_editor_with_read_chosen() {
````

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test --locked --lib -- masked_columns add_column_offers_the_tables add_columns_list tables_control add_table_offers` (the Tables control's tests pass: they are run again in Step 4, after the code they share is moved)

Expected: the four new tests fail, as nothing draws Add column… yet, and the Tables control's three pass:

```text
test ui::agents_tests::masked_columns_changed_outside_while_the_editor_is_open_are_not_saved_over ... FAILED
test ui::agents_tests::the_editors_masked_columns_control_masks_what_agents_get_as_dots ... FAILED
test ui::agents_tests::add_column_offers_the_tables_then_the_columns_it_fetches_and_marks_one_in_none ... FAILED
test ui::agents_tests::add_columns_list_keeps_the_row_chosen_and_says_what_did_not_come ... FAILED
test result: FAILED. 3 passed; 4 failed; 0 ignored; 0 measured; 2587 filtered out
```

- [ ] **Step 3: The control**

````diff
diff --git a/src/agents_shown.rs b/src/agents_shown.rs
index 725e5eb..1ee86d1 100644
--- a/src/agents_shown.rs
+++ b/src/agents_shown.rs
@@ -252,7 +252,7 @@ impl Shown {
 }
 
 /// Whether `a` and `b` mask the same columns, in any order and case.
-fn same_columns(a: &[ColumnName], b: &[ColumnName]) -> bool {
+pub(crate) fn same_columns(a: &[ColumnName], b: &[ColumnName]) -> bool {
     let within = |a: &[ColumnName], b: &[ColumnName]| {
         a.iter()
             .all(|entry| b.iter().any(|other| same_column(entry, other)))
diff --git a/src/app.rs b/src/app.rs
index 6fea333..1f57b05 100644
--- a/src/app.rs
+++ b/src/app.rs
@@ -346,9 +346,9 @@ impl App {
 
     /// What the open windows of the saved connection `conn` have loaded in
     /// their sidebars: each schema's tables and views, every window's
-    /// together, the system's schemas left out. A window switched to
-    /// another database than the saved one's is another database, and is
-    /// not counted. `None` with none open.
+    /// together, the system's schemas left out, and the columns they
+    /// fetched. A window switched to another database than the saved one's
+    /// is another database, and is not counted. `None` with none open.
     pub fn loaded_tables(
         &self,
         conn: &crate::connections::ConnectionId,
@@ -409,13 +409,84 @@ impl App {
                     }
                 }
             }
+            for (object, columns) in workspace
+                .columns
+                .iter()
+                .filter(|(object, _)| user(&object.schema))
+            {
+                let known = |objects: &[ObjectRef]| objects.contains(object);
+                match &columns.value {
+                    Some(columns) => {
+                        if !tables.columns.iter().any(|(known, _)| known == object) {
+                            let names = columns.iter().map(|column| column.name.clone());
+                            tables.columns.push((object.clone(), names.collect()));
+                        }
+                    }
+                    // On its way only while the window's session is there
+                    // to answer.
+                    None if columns.is_loading()
+                        && matches!(workspace.status, SessionStatus::Connected)
+                        && !known(&tables.columns_loading) =>
+                    {
+                        tables.columns_loading.push(object.clone());
+                    }
+                    None if columns.error.is_some() && !known(&tables.columns_failed) => {
+                        tables.columns_failed.push(object.clone());
+                    }
+                    None => {}
+                }
+            }
         }
         if let Some(tables) = found.as_mut() {
             tables.loaded.sort();
+            // Fetched by one window, it is on its way, or failed, no more.
+            let fetched: Vec<ObjectRef> = tables
+                .columns
+                .iter()
+                .map(|(known, _)| known.clone())
+                .collect();
+            tables
+                .columns_loading
+                .retain(|object| !fetched.contains(object));
+            tables
+                .columns_failed
+                .retain(|object| !fetched.contains(object));
         }
         found
     }
 
+    /// Asks an open window of the saved connection `conn` for the columns
+    /// of `objects`, for the Masked columns control: each one no window
+    /// has, nor asked for, nor failed to get.
+    pub fn load_agent_columns(
+        &mut self,
+        conn: &crate::connections::ConnectionId,
+        objects: &[ObjectRef],
+    ) {
+        let Some(saved) = self.connections.get(conn) else {
+            return;
+        };
+        let windows: Vec<ConnTabId> = self
+            .open_connections()
+            .filter(|(_, workspace)| {
+                workspace.conn_id == *conn
+                    && workspace.spec.database == saved.spec.database
+                    && matches!(workspace.status, SessionStatus::Connected)
+            })
+            .map(|(tab, _)| tab)
+            .collect();
+        for object in objects {
+            let asked = windows.iter().any(|tab| {
+                self.workspace(*tab)
+                    .and_then(|workspace| workspace.columns.get(object))
+                    .is_some_and(|kept| !kept.needs_load())
+            });
+            if let (false, Some(tab)) = (asked, windows.first()) {
+                self.load_columns(*tab, object.clone());
+            }
+        }
+    }
+
     pub fn tab_for_session(&self, session: SessionId) -> Option<ConnTabId> {
         self.tabs.iter().find_map(|tab| match &tab.content {
             ConnTabContent::Workspace(workspace) if workspace.session == session => Some(tab.id),
@@ -2093,6 +2164,7 @@ impl App {
                     form.agents_tables = crate::model::TablesChoice::of(&tables);
                     form.agents_tables_opened = tables;
                     form.agents_masked = self.agents_masked(saved);
+                    form.agents_masked_opened = form.agents_masked.clone();
                     self.dialog = Some(Dialog::Connection(Box::new(form)));
                     self.list_ssh_hosts();
                 }
@@ -3849,14 +3921,16 @@ impl App {
     }
 
     fn save_dialog(&mut self, connect: bool) {
-        // The tables in force now, of the connection being edited.
-        let tables_now = match &self.dialog {
+        // The tables and the masked columns in force now, of the connection
+        // being edited.
+        let (tables_now, masked_now) = match &self.dialog {
             Some(Dialog::Connection(form)) => form
                 .editing
                 .as_ref()
                 .and_then(|id| self.connections.get(id))
-                .map(|saved| self.agents_tables(saved)),
-            _ => None,
+                .map(|saved| (self.agents_tables(saved), self.agents_masked(saved)))
+                .unzip(),
+            _ => (None, None),
         };
         let Some(Dialog::Connection(form)) = &mut self.dialog else {
             return;
@@ -3872,6 +3946,11 @@ impl App {
             form.show_agents = true;
             return;
         }
+        if let Err(why) = form.take_typed_column() {
+            form.message = Some(format!("The column's name {why}."));
+            form.show_agents = true;
+            return;
+        }
         // Tables the file changed while the dialog was open are not saved
         // over: what the person changes is what they saw. They are shown
         // as they are now, to be changed again.
@@ -3893,6 +3972,24 @@ impl App {
                 return;
             }
         }
+        // The masked columns likewise.
+        let masked_opened = form.agents_masked_opened.clone();
+        if let Some(now) = &masked_now {
+            use crate::agents_shown::same_columns;
+            let changed = !same_columns(&form.agents_masked, &masked_opened);
+            let moved = !same_columns(now, &masked_opened);
+            if changed && moved {
+                form.agents_masked = now.clone();
+                form.agents_masked_opened = now.clone();
+                form.message = Some(
+                    "The masked columns were changed outside Tabletist while this was open. They \
+                     show as they are now: make your change again."
+                        .into(),
+                );
+                form.show_agents = true;
+                return;
+            }
+        }
         // The connection being edited was deleted while the dialog was open:
         // saving must not bring it back.
         if let Some(id) = &form.editing
@@ -3925,16 +4022,28 @@ impl App {
         // their own, and one they raised is shown, never held. The access
         // the dialog opened with keeps what the file says now: a change
         // made outside the app stays held, and one put back stays back.
-        // The tables likewise: those the person changed are their own and
-        // shown, wider or narrower; the same ones keep what the file says.
-        // The masked columns, which the editor does not change yet, keep
-        // what the file says.
-        if let Some(was) = self.connections.get(&saved.id) {
-            saved.agents.masked = was.agents.masked.clone();
+        // The tables and the masked columns likewise: those the person
+        // changed are their own and shown, wider or narrower; the same ones
+        // keep what the file says.
+        if let Some(was) = self
+            .connections
+            .get(&saved.id)
+            .map(|was| was.agents.clone())
+        {
+            if crate::agents_shown::same_columns(&saved.agents.masked, &masked_opened) {
+                saved.agents.masked = was.masked;
+            } else {
+                self.agents_shown
+                    .keep(&[crate::agents_shown::Raised::Masked {
+                        id: saved.id.clone(),
+                        to: saved.agents.masked.clone(),
+                    }]);
+                self.save_agents_shown();
+            }
             let same = crate::model::TablesChoice::of(&saved.agents.tables)
                 == crate::model::TablesChoice::of(&tables_opened);
             if same {
-                saved.agents.tables = was.agents.tables.clone();
+                saved.agents.tables = was.tables;
             } else {
                 self.agents_shown
                     .keep(&[crate::agents_shown::Raised::Tables {
diff --git a/src/keymap.rs b/src/keymap.rs
index 199ec4b..4e9da81 100644
--- a/src/keymap.rs
+++ b/src/keymap.rs
@@ -782,7 +782,7 @@ impl Command {
             C::FormSaveAndConnect => info("Save and connect", "", Prompts),
             C::FormForgetPassword => info("Forget the saved password", "", Prompts),
             C::FormCancel => info("Cancel", "", Prompts),
-            C::FormTableMove => info("Move in Add table…'s list", "", Prompts),
+            C::FormTableMove => info("Move in Add table…'s or Add column…'s list", "", Prompts),
             C::SettingsMove => info("Move in the settings", "", Prompts),
             C::SettingsChange => info("Change the option", "", Prompts),
             C::SettingsToggle => info("Toggle the option", "", Prompts),
diff --git a/src/ui/connect_dialog/agents.rs b/src/ui/connect_dialog/agents.rs
index 97091f3..86850fc 100644
--- a/src/ui/connect_dialog/agents.rs
+++ b/src/ui/connect_dialog/agents.rs
@@ -1,7 +1,8 @@
 //! The connection editor's Agents part: what agents may do on the
-//! connection (Off or Read), the tables they may not read, how many rows
-//! an answer holds, and how long a statement may run. The boards' rows but
-//! masked columns, which come with 3a, and the third level, with writes.
+//! connection (Off or Read), the tables they may not read, the columns
+//! whose values they get masked, how many rows an answer holds, and how
+//! long a statement may run. The boards' rows but the third level, which
+//! comes with writes.
 
 use egui::{
     Align, CornerRadius, Rect, Sense, Stroke, StrokeKind, Ui, WidgetInfo, WidgetType, vec2,
@@ -38,6 +39,15 @@ const ONLY_HINT: &str = "Agents may read these tables and no others, the ones ma
                          included. A function from an extension can still read the others. For \
                          strict isolation, use a database user without access.";
 
+/// What the Masked columns control says under it: what agents get of a
+/// masked column, and what they may not do with one (the MCP plan's B.4).
+/// Not the board's "The agent sees the name and type; values come back as
+/// •••." alone: a statement that uses one otherwise is refused, wherever a
+/// column of its name is. Agreed on 2026-10-18.
+const MASKED_HINT: &str = "Agents see the name and type; values come back as •••. A query that \
+                           filters, joins, sorts or computes with one is refused, in any table \
+                           with a column of that name.";
+
 /// The hint for the mode chosen: none for "All", which hides nothing.
 fn tables_hint(form: &ConnectionForm) -> Option<&'static str> {
     match form.agents_tables.mode {
@@ -133,7 +143,7 @@ fn table_names(ui: &mut Ui, form: &mut ConnectionForm, skin: &Skin) {
     ui.horizontal_wrapped(|ui| {
         ui.spacing_mut().item_spacing = vec2(6.0, 6.0);
         for (at, (name, missing)) in written.iter().zip(&missing).enumerate() {
-            if chip(ui, name, at, *missing, height, skin) {
+            if chip(ui, "agents-table", name, at, *missing, height, skin) {
                 removed = Some(at);
             }
         }
@@ -153,6 +163,8 @@ fn table_names(ui: &mut Ui, form: &mut ConnectionForm, skin: &Skin) {
                 // Up and Down move in the list, taken before the field
                 // reads them, so that they do not move its caret too.
                 let offered = offered_for(form, &before);
+                let rows: Vec<String> = offered.iter().map(ToString::to_string).collect();
+                follow_pick(ui, list, &rows, &mut form.agents_table_pick);
                 if !offered.is_empty() && ui.memory(|memory| memory.has_focus(id)) {
                     let step = moved(ui, skin);
                     if step != 0 {
@@ -189,17 +201,7 @@ fn table_names(ui: &mut Ui, form: &mut ConnectionForm, skin: &Skin) {
                 if focus == Some(TableFocus::Field) {
                     field.request_focus();
                 }
-                // A release over the list, where the field gives up the
-                // keyboard, is no leaving it: the list was last seen there.
-                // Only a release this frame: a pointer that rests there says
-                // nothing of a Tab.
-                let released = ui.input(|input| input.pointer.any_released());
-                let on_list = released
-                    && ui
-                        .ctx()
-                        .memory(|memory| memory.area_rect(list))
-                        .zip(ui.input(|input| input.pointer.latest_pos()))
-                        .is_some_and(|(rect, at)| rect.contains(at));
+                let on_list = released_on(ui, list);
                 // Only while the field has the keyboard, or lets it go this
                 // frame (to a press on the list): a field left open with a
                 // name that does not read lists nothing over the fields
@@ -208,7 +210,10 @@ fn table_names(ui: &mut Ui, form: &mut ConnectionForm, skin: &Skin) {
                 let clicked = match offered.is_empty() || !listing {
                     true => None,
                     false => {
-                        offered_list(ui, list, field.rect, &offered, form.agents_table_pick, skin)
+                        let rows: Vec<String> = offered.iter().map(ToString::to_string).collect();
+                        let note = skin.say("Tables loaded in the sidebar");
+                        let pick = form.agents_table_pick;
+                        offered_list(ui, list, field.rect, &rows, pick, &note, skin)
                     }
                 };
                 // Screen readers hear the table the keyboard is on.
@@ -223,20 +228,7 @@ fn table_names(ui: &mut Ui, form: &mut ConnectionForm, skin: &Skin) {
                     }
                 });
                 let pressed = |key| ui.input(|input| input.key_pressed(key));
-                // The form's Cancel, as the keymap has it, while the field
-                // has the keyboard or has just let it go: the field's own,
-                // taken out of the frame so that the dialog is not closed
-                // by it.
-                let cancelled = (field.has_focus() || field.lost_focus()) && {
-                    let keymap = crate::ui::keys::published(ui.ctx());
-                    let layout = crate::keymap::Layout::of(skin.look);
-                    ui.input_mut(|input| {
-                        let cancel = crate::keymap::Command::FormCancel;
-                        let scope = crate::keymap::Scope::Prompt;
-                        crate::ui::keys::asked(input, &keymap, layout, cancel, scope, true).count
-                            > 0
-                    })
-                };
+                let cancelled = cancel_asked(ui, &field, skin);
                 if let Some(at) = clicked {
                     form.agents_table_typed = Some(offered[at].to_string());
                     form.agents_table_pick = None;
@@ -293,6 +285,304 @@ fn table_names(ui: &mut Ui, form: &mut ConnectionForm, skin: &Skin) {
     }
 }
 
+/// The masked columns, each with its × to take it out, and Add column…
+/// after them, which opens a field for a name, as Add table… does. Its
+/// list offers the tables the sidebar loaded; one chosen goes into the
+/// field, `orders.`, and the list offers its columns, fetched from an open
+/// window, to be masked.
+fn column_names(ui: &mut Ui, form: &mut ConnectionForm, skin: &Skin) {
+    let written: Vec<String> = form.agents_masked.iter().map(ToString::to_string).collect();
+    // Which name no column of the database, as the windows know it.
+    let missing: Vec<bool> = form
+        .agents_masked
+        .iter()
+        .map(|entry| {
+            form.agents_loaded
+                .as_ref()
+                .and_then(|loaded| loaded.has_column(entry))
+                == Some(false)
+        })
+        .collect();
+    let height = if skin.look.terminal { 21.0 } else { 24.0 };
+    let mut removed = None;
+    let focus = form.agents_column_focus.take();
+    ui.horizontal_wrapped(|ui| {
+        ui.spacing_mut().item_spacing = vec2(6.0, 6.0);
+        for (at, (name, missing)) in written.iter().zip(&missing).enumerate() {
+            if chip(ui, "agents-column", name, at, *missing, height, skin) {
+                removed = Some(at);
+            }
+        }
+        let Some(before) = form.agents_column_typed.clone() else {
+            let add = skin.say("Add column…");
+            let button = ButtonSpec::new(&add).padding(8.0).role(TextRole::pick(
+                skin.look,
+                TextRole::Secondary,
+                TextRole::OBody,
+            ));
+            let response = button.show(ui, height, skin.look, skin.palette);
+            if focus == Some(TableFocus::Add) {
+                response.request_focus();
+            }
+            if response.clicked() {
+                form.agents_column_typed = Some(String::new());
+                form.agents_column_focus = Some(TableFocus::Field);
+            }
+            return;
+        };
+        let role = widgets::code(skin.look);
+        let id = ui.id().with("agents-column-typed");
+        let list = id.with("list");
+        let offered = form.columns_offered(&before);
+        let rows: Vec<String> = offered.rows.iter().map(ToString::to_string).collect();
+        follow_pick(ui, list, &rows, &mut form.agents_column_pick);
+        if !offered.rows.is_empty() && ui.memory(|memory| memory.has_focus(id)) {
+            let step = moved(ui, skin);
+            if step != 0 {
+                let last = offered.rows.len() as isize - 1;
+                form.agents_column_pick = Some(match form.agents_column_pick {
+                    None if step > 0 => 0,
+                    None => last as usize,
+                    Some(at) => (at as isize + step).rem_euclid(last + 1) as usize,
+                });
+            }
+        }
+        let placeholder = super::hint(ui, &skin.say("table.column"), role, skin);
+        let Some(typed) = form.agents_column_typed.as_mut() else {
+            return;
+        };
+        let field = ui.add_sized(
+            vec2(200.0, height),
+            input(ui, typed, role, height, skin)
+                .id(id)
+                .hint_text(placeholder),
+        );
+        let typed = typed.clone();
+        if field.changed() {
+            form.agents_column_pick = None;
+        }
+        let offered = match typed == before {
+            true => offered,
+            false => form.columns_offered(&typed),
+        };
+        if form
+            .agents_column_pick
+            .is_some_and(|at| at >= offered.rows.len())
+        {
+            form.agents_column_pick = None;
+        }
+        if focus == Some(TableFocus::Field) {
+            field.request_focus();
+        }
+        let on_list = released_on(ui, list);
+        // What the list says it offers: the tables the sidebar loaded, or
+        // the columns of the one typed, which may be on their way or may
+        // not have come.
+        let among = |objects: &[tabletist_db::ObjectRef]| {
+            offered.tables.iter().any(|object| objects.contains(object))
+        };
+        let (loading, failed) = form
+            .agents_loaded
+            .as_ref()
+            .map(|loaded| {
+                (
+                    among(&loaded.columns_loading),
+                    among(&loaded.columns_failed),
+                )
+            })
+            .unwrap_or_default();
+        let note = match &offered.of {
+            None => skin.say("Tables loaded in the sidebar"),
+            Some(table) => {
+                let said = match (loading, failed) {
+                    (true, _) => skin.say("Loading the columns of {table}…"),
+                    (false, true) if offered.rows.is_empty() => {
+                        skin.say("The columns of {table} could not be read")
+                    }
+                    _ => skin.say("Columns of {table}"),
+                };
+                said.replace("{table}", &table.to_string())
+            }
+        };
+        let shows = !offered.rows.is_empty() || loading || failed;
+        let listing = field.has_focus() || field.lost_focus();
+        let clicked = match shows && listing {
+            false => None,
+            true => {
+                let rows: Vec<String> = offered.rows.iter().map(ToString::to_string).collect();
+                let pick = form.agents_column_pick;
+                offered_list(ui, list, field.rect, &rows, pick, &note, skin)
+            }
+        };
+        // Screen readers hear the row the keyboard is on.
+        let name = skin.say("Column to mask");
+        let chosen = form
+            .agents_column_pick
+            .map(|at| list.with(at).accesskit_id());
+        ui.ctx().accesskit_node_builder(field.id, |node| {
+            node.set_label(name.as_str());
+            if let Some(chosen) = chosen {
+                node.set_active_descendant(chosen);
+            }
+        });
+        let pressed = |key| ui.input(|input| input.key_pressed(key));
+        let cancelled = cancel_asked(ui, &field, skin);
+        // Enter on a table, or a click on one, puts it in the field for its
+        // columns, the caret after it.
+        let into = |form: &mut ConnectionForm, at: usize| match &offered.rows[at] {
+            crate::model::OfferedColumn::Table(table) => {
+                let typed = format!("{table}.");
+                let mut state =
+                    egui::text_edit::TextEditState::load(ui.ctx(), id).unwrap_or_default();
+                let end = egui::text::CCursor::new(typed.chars().count());
+                state
+                    .cursor
+                    .set_char_range(Some(egui::text::CCursorRange::one(end)));
+                state.store(ui.ctx(), id);
+                form.agents_column_typed = Some(typed);
+                form.agents_column_pick = None;
+                form.agents_column_focus = Some(TableFocus::Field);
+                true
+            }
+            crate::model::OfferedColumn::Column(_) => false,
+        };
+        if let Some(at) = clicked {
+            if !into(form, at) {
+                form.agents_column_typed = Some(offered.rows[at].to_string());
+                form.agents_column_pick = None;
+                if form.take_typed_column().is_ok() {
+                    form.agents_column_typed = Some(String::new());
+                }
+                form.agents_column_focus = Some(TableFocus::Field);
+            }
+        } else if cancelled {
+            field.surrender_focus();
+            form.agents_column_pick = None;
+            form.agents_column_typed = None;
+            form.agents_column_error = None;
+            form.agents_column_focus = Some(TableFocus::Add);
+        } else if field.lost_focus() && on_list && shows {
+            form.agents_column_focus = Some(TableFocus::Field);
+        } else if field.lost_focus() {
+            let entered = pressed(egui::Key::Enter);
+            let pick = form.agents_column_pick;
+            if entered && pick.is_some_and(|at| into(form, at)) {
+                return;
+            }
+            let empty = typed.trim().is_empty() && pick.is_none();
+            match form.take_typed_column() {
+                Ok(()) if entered && !empty => {
+                    form.agents_column_typed = Some(String::new());
+                    form.agents_column_focus = Some(TableFocus::Field);
+                }
+                Ok(()) => form.agents_column_typed = None,
+                Err(_) if entered => form.agents_column_focus = Some(TableFocus::Field),
+                Err(_) => {}
+            }
+        }
+    });
+    if let Some(at) = removed
+        && at < form.agents_masked.len()
+    {
+        form.agents_masked.remove(at);
+    }
+}
+
+/// Under masked columns one of which names no column of the database:
+/// what that does to agents (the MCP plan's C.3), and how to put it right.
+fn column_missing(ui: &mut Ui, form: &ConnectionForm, skin: &Skin) {
+    let Some(loaded) = &form.agents_loaded else {
+        return;
+    };
+    if !form
+        .agents_masked
+        .iter()
+        .any(|entry| loaded.has_column(entry) == Some(false))
+    {
+        return;
+    }
+    let said = skin.say(
+        "Agents may not use this connection while a masked column is not in its database. Take \
+         the name out, or add the column's new name.",
+    );
+    ui.add_space(4.0);
+    let room = ui.available_width();
+    Text::one(
+        skin.look,
+        TextRole::pick(skin.look, TextRole::Secondary, TextRole::OSecondary),
+        &said,
+        skin.palette.warning,
+    )
+    .wrap(room)
+    .layout(ui.ctx())
+    .label(ui);
+}
+
+/// Why the column's name typed does not read, under the names.
+fn column_error(ui: &mut Ui, form: &ConnectionForm, skin: &Skin) {
+    if let Some(why) = &form.agents_column_error {
+        let said = skin.say("The column's name {why}.").replace("{why}", why);
+        ui.add_space(4.0);
+        let room = ui.available_width();
+        Text::one(
+            skin.look,
+            TextRole::pick(skin.look, TextRole::Secondary, TextRole::OSecondary),
+            &said,
+            skin.palette.danger,
+        )
+        .wrap(room)
+        .layout(ui.ctx())
+        .label(ui);
+    }
+}
+
+/// Keeps the row the keyboard is on in a list whose rows changed under
+/// it (a table's columns came, a sidebar loaded more): the same row where
+/// it is still offered, none where it is not, so that Enter adds what was
+/// chosen. `rows` are the ones offered now; the last ones are kept with
+/// `list`.
+fn follow_pick(ui: &Ui, list: egui::Id, rows: &[String], pick: &mut Option<usize>) {
+    let kept = list.with("rows");
+    let before: Option<Vec<String>> = ui.data(|data| data.get_temp(kept));
+    if let (Some(before), Some(at)) = (before, *pick)
+        && before != rows
+    {
+        *pick = before
+            .get(at)
+            .and_then(|row| rows.iter().position(|now| now == row));
+    }
+    ui.data_mut(|data| data.insert_temp(kept, rows.to_vec()));
+}
+
+/// Whether the form's Cancel, as the keymap has it, was pressed while
+/// `field` has the keyboard or has just let it go: the field's own, taken
+/// out of the frame so that the dialog is not closed by it.
+fn cancel_asked(ui: &Ui, field: &egui::Response, skin: &Skin) -> bool {
+    (field.has_focus() || field.lost_focus()) && {
+        let keymap = crate::ui::keys::published(ui.ctx());
+        let layout = crate::keymap::Layout::of(skin.look);
+        ui.input_mut(|input| {
+            let cancel = crate::keymap::Command::FormCancel;
+            let scope = crate::keymap::Scope::Prompt;
+            crate::ui::keys::asked(input, &keymap, layout, cancel, scope, true).count > 0
+        })
+    }
+}
+
+/// Whether the pointer was released over the list `list` this frame. A
+/// release there, where the field gives up the keyboard, is no leaving it:
+/// the list was last seen there. Only a release this frame: a pointer that
+/// rests there says nothing of a Tab.
+fn released_on(ui: &Ui, list: egui::Id) -> bool {
+    let released = ui.input(|input| input.pointer.any_released());
+    released
+        && ui
+            .ctx()
+            .memory(|memory| memory.area_rect(list))
+            .zip(ui.input(|input| input.pointer.latest_pos()))
+            .is_some_and(|(rect, at)| rect.contains(at))
+}
+
 /// How far Add table…'s list was asked to move this frame: up by one,
 /// down by one, or not, by the keys the keymap gives the form for it, as
 /// quick open's list reads its own.
@@ -312,15 +602,16 @@ fn moved(ui: &mut Ui, skin: &Skin) -> isize {
     })
 }
 
-/// The window's tables that match what is typed, floating under the field
-/// at `under`, the one the keyboard is on (`pick`) marked, with a line that
-/// says where they come from. The place of the one clicked.
+/// The rows offered for what is typed, floating under the field at
+/// `under`, the one the keyboard is on (`pick`) marked, with a line, `note`,
+/// that says where they come from. The place of the one clicked.
 fn offered_list(
     ui: &mut Ui,
     id: egui::Id,
     under: Rect,
-    offered: &[tabletist_db::agent::TableName],
+    offered: &[String],
     pick: Option<usize>,
+    note: &str,
     skin: &Skin,
 ) -> Option<usize> {
     let Skin { look, palette, .. } = *skin;
@@ -365,8 +656,7 @@ fn offered_list(
                 .inner_margin(egui::Margin::same(4))
                 .show(ui, |ui| {
                     ui.set_width(width);
-                    for (at, name) in offered.iter().enumerate() {
-                        let written = name.to_string();
+                    for (at, written) in offered.iter().enumerate() {
                         let (rect, _) = ui.allocate_exact_size(vec2(width, row), Sense::hover());
                         let response = ui.interact(rect, id.with(at), Sense::click());
                         let chosen = pick == Some(at);
@@ -375,12 +665,12 @@ fn offered_list(
                                 WidgetType::SelectableLabel,
                                 ui.is_enabled(),
                                 chosen,
-                                &written,
+                                written,
                             )
                         });
                         widgets::selection(ui, rect, chosen, response.hovered(), look, palette);
                         let ink = widgets::selection_text(chosen, look, palette);
-                        let text = Text::one(look, role, &written, ink).layout(ui.ctx());
+                        let text = Text::one(look, role, written, ink).layout(ui.ctx());
                         let y = rect.center().y - text.galley.size().y / 2.0;
                         ui.painter()
                             .galley(egui::pos2(rect.left() + 8.0, y), text.galley, ink);
@@ -388,10 +678,9 @@ fn offered_list(
                             clicked = Some(at);
                         }
                     }
-                    let said = skin.say("Tables loaded in the sidebar");
                     let small = TextRole::pick(look, TextRole::Secondary, TextRole::OSecondary);
                     ui.add_space(2.0);
-                    widgets::label(ui, small, &said, palette.faint, look);
+                    widgets::label(ui, small, note, palette.faint, look);
                 });
         });
     clicked
@@ -452,7 +741,15 @@ fn table_error(ui: &mut Ui, form: &ConnectionForm, skin: &Skin) {
 
 /// A listed name, `customers ×`, bordered; the × takes it out, and screen
 /// readers hear it as "Remove customers". Whether it was pressed.
-fn chip(ui: &mut Ui, name: &str, at: usize, missing: bool, height: f32, skin: &Skin) -> bool {
+fn chip(
+    ui: &mut Ui,
+    kind: &str,
+    name: &str,
+    at: usize,
+    missing: bool,
+    height: f32,
+    skin: &Skin,
+) -> bool {
     let Skin { look, palette, .. } = *skin;
     let role = widgets::code(look);
     let cross = if look.terminal { "x" } else { "×" };
@@ -497,7 +794,7 @@ fn chip(ui: &mut Ui, name: &str, at: usize, missing: bool, height: f32, skin: &S
         let said = format!("{name}, {mark}");
         ui.interact(
             place,
-            ui.id().with(("agents-table-missing", at, name)),
+            ui.id().with((kind, "missing", at, name)),
             Sense::hover(),
         )
         .widget_info(|| WidgetInfo::labeled(WidgetType::Label, true, &said));
@@ -508,7 +805,7 @@ fn chip(ui: &mut Ui, name: &str, at: usize, missing: bool, height: f32, skin: &S
     );
     let response = ui.interact(
         cell,
-        ui.id().with(("agents-table-remove", at, name)),
+        ui.id().with((kind, "remove", at, name)),
         Sense::click(),
     );
     let label = skin.say("Remove {name}").replace("{name}", name);
@@ -586,23 +883,38 @@ pub(super) fn sheet(ui: &mut Ui, form: &mut ConnectionForm, skin: &Skin) {
             sentence(ui, small, &said, skin);
         }
         ui.add_space(12.0);
-        let tables = skin.say("Tables");
-        widgets::label(ui, small, &tables, palette.secondary, look);
-        ui.add_space(5.0);
-        tables_mode(ui, form, &tables, Group::Track(28.0), skin);
-        if form.agents_tables.names().is_some() {
-            ui.add_space(6.0);
-            table_names(ui, form, skin);
-            table_error(ui, form, skin);
-            table_missing(ui, form, skin);
-        }
-        if let Some(hint) = tables_hint(form) {
-            ui.add_space(6.0);
-            sentence(ui, small, &skin.say(hint), skin);
-        }
+        // Tables and Masked columns side by side, as the board has them.
+        let columns = [super::Column::Fr(1.0), super::Column::Fr(1.0)];
+        super::row(ui, 12.0, &columns, |ui, column| match column {
+            0 => {
+                let tables = skin.say("Tables");
+                widgets::label(ui, small, &tables, palette.secondary, look);
+                ui.add_space(5.0);
+                tables_mode(ui, form, &tables, Group::Track(28.0), skin);
+                if form.agents_tables.names().is_some() {
+                    ui.add_space(6.0);
+                    table_names(ui, form, skin);
+                    table_error(ui, form, skin);
+                    table_missing(ui, form, skin);
+                }
+                if let Some(hint) = tables_hint(form) {
+                    ui.add_space(6.0);
+                    sentence(ui, small, &skin.say(hint), skin);
+                }
+            }
+            _ => {
+                let masked = skin.say("Masked columns");
+                widgets::label(ui, small, &masked, palette.secondary, look);
+                ui.add_space(5.0);
+                column_names(ui, form, skin);
+                column_error(ui, form, skin);
+                column_missing(ui, form, skin);
+                ui.add_space(6.0);
+                sentence(ui, small, &skin.say(MASKED_HINT), skin);
+            }
+        });
         ui.add_space(12.0);
         let (role, height) = (skin.value_role(), skin.field_height());
-        let columns = [super::Column::Fr(1.0), super::Column::Fr(1.0)];
         super::row(ui, 12.0, &columns, |ui, column| {
             let (label, text) = match column {
                 0 => (skin.say("Rows per answer"), &mut form.agents_rows),
@@ -647,6 +959,14 @@ pub(super) fn terminal(ui: &mut Ui, form: &mut ConnectionForm, skin: &Skin) {
         sentence(ui, TextRole::OSecondary, &skin.say(hint), skin);
         ui.add_space(4.0);
     }
+    let masked = skin.say("Masked");
+    super::terminal::terminal_row_of(ui, &masked, line + 4.0, skin, |ui, _| {
+        column_names(ui, form, skin);
+    });
+    column_error(ui, form, skin);
+    column_missing(ui, form, skin);
+    sentence(ui, TextRole::OSecondary, &skin.say(MASKED_HINT), skin);
+    ui.add_space(4.0);
     let (role, height) = (TextRole::OBody, skin.field_height());
     super::terminal::terminal_row(ui, &skin.say("Rows per answer"), skin, |ui, label| {
         ui.add(input(ui, &mut form.agents_rows, role, height, skin))
diff --git a/src/ui/connect_dialog/mod.rs b/src/ui/connect_dialog/mod.rs
index e3d38b9..186429c 100644
--- a/src/ui/connect_dialog/mod.rs
+++ b/src/ui/connect_dialog/mod.rs
@@ -240,20 +240,44 @@ pub fn show(app: &mut App, ctx: &egui::Context) {
         crate::ui::keys::asked(input, &keymap, layout, command, Scope::Prompt, true)
     };
     // What an open window of the connection has loaded, for the Tables
-    // control: read again on each frame, as a sidebar loads.
-    // Only for a mode with a list: what it costs is spent on the list.
+    // and Masked columns controls: read again on each frame, as a sidebar
+    // loads. Only for a list of names: what it costs is spent on them.
     let loaded = match &app.dialog {
-        Some(Dialog::Connection(form)) if form.agents_tables.names().is_some() => form
-            .editing
-            .as_ref()
-            .and_then(|conn| app.loaded_tables(conn)),
+        Some(Dialog::Connection(form))
+            if form.agents_tables.names().is_some()
+                || !form.agents_masked.is_empty()
+                || form.agents_column_typed.is_some() =>
+        {
+            form.editing
+                .as_ref()
+                .and_then(|conn| app.loaded_tables(conn).map(|loaded| (conn.clone(), loaded)))
+        }
         _ => None,
     };
+    // The columns of the tables the masked columns name, for their mark,
+    // and of the one Add column…'s field names, for its list.
+    let wanted = match (&app.dialog, &loaded) {
+        (Some(Dialog::Connection(form)), Some((conn, loaded))) => {
+            let mut wanted: Vec<tabletist_db::ObjectRef> = form
+                .agents_masked
+                .iter()
+                .flat_map(|entry| loaded.named(entry.schema.as_deref(), &entry.table))
+                .collect();
+            if let Some(typed) = &form.agents_column_typed {
+                wanted.extend(loaded.offered_columns(typed, &[]).tables);
+            }
+            Some((conn.clone(), wanted))
+        }
+        _ => None,
+    };
+    if let Some((conn, wanted)) = wanted {
+        app.load_agent_columns(&conn, &wanted);
+    }
     let Some(Dialog::Connection(form)) = &mut app.dialog else {
         ctx.data_mut(|data| data.remove::<Placement>(placement_id()));
         return;
     };
-    form.agents_loaded = loaded;
+    form.agents_loaded = loaded.map(|(_, loaded)| loaded);
     let placement: Placement = ctx
         .data(|data| data.get_temp(placement_id()))
         .unwrap_or_default();
````

- [ ] **Step 4: Run the tests**

Run: `cargo test --locked --lib -- masked_columns add_column_offers_the_tables add_columns_list tables_control add_table_offers`

Expected:

```text
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 2587 filtered out
```

- [ ] **Step 5: Commit**

```bash
cargo fmt --all
cargo clippy --locked --workspace --all-targets -- -D warnings
git add src/app.rs src/agents_shown.rs src/keymap.rs src/ui/connect_dialog/agents.rs src/ui/connect_dialog/mod.rs src/ui/agents_tests.rs
git commit -m "Add the editor's Masked columns control, with Add column… and its mark"
```

---

### Task 3: The website

- `docs/_guide/agents.md`: **Mask columns**: what agents get of a masked column, what is refused, Add column…'s list and its mark, and what a renamed column does.
- `docs/_reference/connections.md`: the control in the Agents section, and a change made in the file while the form is open.
- `docs/_guide/macos.md`, `docs/_reference/keyboard-shortcuts.md`: Up and Down in Add column…'s list.

**Files:**
- Modify: `docs/_guide/agents.md`, `docs/_reference/connections.md`, `docs/_guide/macos.md`, `docs/_reference/keyboard-shortcuts.md`

- [ ] **Step 1: The pages**

````diff
diff --git a/docs/_guide/agents.md b/docs/_guide/agents.md
index 20119bc..84c190a 100644
--- a/docs/_guide/agents.md
+++ b/docs/_guide/agents.md
@@ -155,6 +155,43 @@ waits for your answer: see [below](#changes-made-outside-tabletist).
 [Settings and Files]({% link _reference/settings-and-files.md %}#the-connections-file)
 has the keys.
 
+## Mask columns
+
+A connection can give agents some columns' names and types, and never
+their values. In its form's **Agents** section, **Masked columns** lists
+them, and **Add column…** takes a table and a column, `orders.email`, or
+with the schema, `sales.orders.email`; one without a schema is that
+column of the table of that name in every schema.
+
+While a window of the connection is open, Add column… lists the tables
+that window's sidebar has loaded as you type. Choose one and it goes into
+the field, `orders.`, and the list shows that table's columns, which
+Tabletist asks the window for. A masked column that the loaded schemas
+and fetched columns show is not in the database is marked **not in this
+database**. What the sidebar has not loaded is not known, and not marked.
+
+What agents get of a masked column:
+
+- `describe_schema` gives its name and type, marked masked, and leaves
+  out its default, its comment and the values it may hold.
+- `run_query` answers `•••` for every value, an empty one too, where a
+  query selects the column as it is.
+- A query that filters, joins, sorts, groups or computes on it is
+  refused, and so is one that does that to any column of the same name in
+  any table. So is a view that reads it.
+
+Its partitions and child tables have it masked too, and so does a
+generated column of its table. If a masked column is renamed or dropped,
+agents may not use the connection at all until you take it out, or put in
+its new name: its new name would be masked by nothing.
+
+A function that comes with an extension can still read a masked column.
+For strict isolation, give the connection a database user without access
+to it.
+
+A change in `connections.toml` that masks less, made outside Tabletist,
+waits for your answer: see [below](#changes-made-outside-tabletist).
+
 ## See what agents do
 
 Each connection window has an Agents tab for its connection. It opens
diff --git a/docs/_guide/macos.md b/docs/_guide/macos.md
index 82c30eb..514d8d5 100644
--- a/docs/_guide/macos.md
+++ b/docs/_guide/macos.md
@@ -88,7 +88,7 @@ a Mac keyboard labels Delete.
 | Cmd+S | Save |
 | Cmd+Return | Save and connect |
 | Backspace, Forward Delete | In the empty Password field: forget the saved password |
-| Up, Down | In Add table…'s list of tables: move. Return adds the one chosen, and Esc closes the field |
+| Up, Down | In Add table…'s or Add column…'s list: move. Return adds the one chosen, or in Add column…'s puts a table chosen in the field for its columns, and Esc closes the field |
 | Esc | Cancel |
 
 ### The sidebar
diff --git a/docs/_reference/connections.md b/docs/_reference/connections.md
index 190b1df..3064743 100644
--- a/docs/_reference/connections.md
+++ b/docs/_reference/connections.md
@@ -164,8 +164,8 @@ server is on.
   connection's **Agents** section, and **Show Agents tab** shows its
   Agents tab.
 - In the connection's form, the **Agents** section has the access,
-  **Tables**, **Rows per answer** (1 to 5000, 500 at first) and the
-  **Statement timeout** in seconds (1 to 60, 10 at first).
+  **Tables**, **Masked columns**, **Rows per answer** (1 to 5000, 500 at
+  first) and the **Statement timeout** in seconds (1 to 60, 10 at first).
 - **Tables** is **All**, **All except** or **Only**. With **All except**,
   agents may read every table but the ones listed; with **Only**, the ones
   listed and no others, so that a table made later is hidden too. **Add table…** opens
@@ -183,12 +183,27 @@ server is on.
   never what is in the hidden tables: not their rows, their columns, or
   the views over them. A function from an extension can still read them;
   for strict isolation, use a database user without access.
+- **Masked columns** lists the columns whose values agents get as `•••`.
+  **Add column…** opens a field for a table and a column, `orders.email`
+  or `sales.orders.email`, which works as Add table…'s does: Enter adds
+  it, Escape closes the field, **×** takes one out, and one still in the
+  field when you save is added. While a window of the connection is open,
+  the field lists the tables its sidebar has loaded; Enter or a click on
+  one puts it in the field, and the list then shows its columns, which the
+  window is asked for. A masked column that the loaded schemas and fetched
+  columns show is not in the database is marked **not in this database**,
+  and agents may not use the connection until it is taken out or put
+  right. Agents see a masked column's name and type, and a query that
+  filters, joins, sorts or computes with it is refused, in any table with
+  a column of that name.
 - If `connections.toml` hides more tables while the form is open, saving
   a change of yours to them is refused, and the form shows them as they
   are now, for you to make your change again. A change in the file that
   hides less is held until you answer it, and is not in force: a save that
   changes the tables goes through over it, and one that leaves them as
-  shown keeps it, still held.
+  shown keeps it, still held. Masked columns are kept the same way: if
+  the file masks more while the form is open, saving a change of yours to
+  them is refused and the form shows them as they are now.
 - An Agents tab that says agents can't use a connection has **Allow
   agents…**, which opens that section with **Read** chosen. Nothing
   changes until you save.
diff --git a/docs/_reference/keyboard-shortcuts.md b/docs/_reference/keyboard-shortcuts.md
index 4691ce2..8f2d603 100644
--- a/docs/_reference/keyboard-shortcuts.md
+++ b/docs/_reference/keyboard-shortcuts.md
@@ -43,7 +43,7 @@ In the connection dialog:
 | Mod+T | Test |
 | Mod+Enter | Save and connect |
 | Backspace, Delete | In the empty Password field: forget the saved password |
-| Up, Down | In Add table…'s list of tables: move. Enter adds the one chosen, and Esc closes the field |
+| Up, Down | In Add table…'s or Add column…'s list: move. Enter adds the one chosen, or in Add column…'s puts a table chosen in the field for its columns, and Esc closes the field |
 | Esc | Cancel |
 
 ## The window
````

- [ ] **Step 2: Check them**

Run: `docs/bin/check` and `grep -c "$(printf '\342\200\224')" docs/_guide/agents.md docs/_guide/macos.md docs/_reference/connections.md docs/_reference/keyboard-shortcuts.md` (a count of em dashes)

Expected: `HTML-Proofer finished successfully.`, and `0` for each page.

- [ ] **Step 3: Commit**

```bash
git add docs/_guide/agents.md docs/_reference/connections.md docs/_guide/macos.md docs/_reference/keyboard-shortcuts.md
git commit -m "Say on the website how to mask a column for agents"
```

---

### Task 4: The master plan

- B.4 and D.3 as built for 3a-4; G's row and 3a's run 4: built.

**Files:**
- Modify: `docs/superpowers/plans/2026-10-13-mcp-server.md`

- [ ] **Step 1: The edits**

````diff
diff --git a/docs/superpowers/plans/2026-10-13-mcp-server.md b/docs/superpowers/plans/2026-10-13-mcp-server.md
index 1141cf2..a3c7909 100644
--- a/docs/superpowers/plans/2026-10-13-mcp-server.md
+++ b/docs/superpowers/plans/2026-10-13-mcp-server.md
@@ -1204,6 +1204,10 @@ once in the bar. The editor has no control for them until run 4: its
 save keeps what the file says. The app's gateway passes them to the
 server, which has used them since run 2.
 
+**As built in 3a's run 4** (`2026-10-18-mcp-server-3a-4-columns-control.md`):
+the editor's control, in D.3 below, and the website's guide, "Mask
+columns" in `docs/_guide/agents.md`.
+
 **The rule.** It applies to any statement that reads a table with a masked
 column.
 
@@ -2041,6 +2045,55 @@ otherwise:
   a change of every row's, for another time.
 - The hint (B.3) is under the control, in both looks.
 
+**As built for 3a** (`2026-10-18-mcp-server-3a-4-columns-control.md`).
+Agreed on 2026-10-18, where it went otherwise:
+
+- **Masked columns**, in both looks: the masked columns as chips with a
+  ×, and **Add column…**, which opens a field for a table and a column as
+  the file writes them (`orders.email`, `sales.orders.email`) and works
+  as Add table…'s does: Enter, a name the field holds when it loses the
+  keyboard or at a save, Escape, a name that does not read said under the
+  names and refusing a save. On macOS it stands beside Tables, as the
+  board draws them; on Omarchy it is the `masked` row after the tables.
+- The list: while the field holds a name, the tables the sidebar of an
+  open window loaded that match it, as Add table…'s list has them, each
+  written with a dot after it (`orders.`). Enter or a click on one puts it
+  in the field, the caret after it, and the list then offers that table's
+  columns (every schema's table of that name for one without a schema),
+  none an entry masks already, at most eight, with a line that says
+  whose columns they are or that they are on their way. They are asked of
+  an open window as the SQL editor's completion asks for them, and kept
+  with its columns: once, a failed one not again. Up and Down move in the
+  list with `FormTableMove`. Tab or a click away takes what is typed: a
+  table chosen is no column.
+- The mark: the columns of every table a masked column names are asked
+  for when the form shows them, and a masked column that the loaded
+  schemas and fetched columns show is no column of the database is marked
+  "not in this database", with the line that says agents may not use the
+  connection until it is taken out or put right (C.3). A column of a table
+  whose columns were not fetched, or of a schema not loaded, is not
+  marked; one without a schema only when every schema is loaded. One is
+  marked wrongly, and left so: a hidden column of a SQLite virtual table
+  (an FTS5 table's own), which the window's columns leave out and the
+  agent server counts.
+- The run's review found the columns of one table taken for those of
+  another whose name differs in case, the row chosen in the list changing
+  as more columns came, "Loading the columns…" kept after the window's
+  session was gone, a table whose name holds a dot offering no columns,
+  and a fetch that failed shown as a table without columns: each is fixed
+  with a test. The row chosen now follows its value in Add table…'s list
+  too.
+- The form opens with the masked columns in force; ones the person
+  changed are theirs and shown, more or fewer, and ones left as they
+  opened keep what the file says, a change made outside staying held, as
+  the tables do. Where the masked columns in force changed while the form
+  was open, a change of the person's is refused and the form shows them as
+  they are now.
+- The hint is not the board's "The agent sees the name and type; values
+  come back as •••." alone: "Agents see the name and type; values come
+  back as •••. A query that filters, joins, sorts or computes with one is
+  refused, in any table with a column of that name." In both looks.
+
 ### D.4 The Agents tab
 
 Boards: `macOS – Agents tab, activity and approval`, `macOS – Agents tab, no
@@ -2640,7 +2693,7 @@ is off until the person turns it on.
 | 1b on Windows: the way in | 1 | Built (`2026-10-15-mcp-server-1b-6-windows.md`) |
 | 1c. Hidden tables | 6, below | Built (`2026-10-15-mcp-server-1c-1-checks.md`, `2026-10-15-mcp-server-1c-2a-server.md`, `2026-10-16-mcp-server-1c-2b-setting.md`, `2026-10-16-mcp-server-1c-3a-tables-control.md`, `2026-10-16-mcp-server-1c-3b-table-list.md`) |
 | 1d. HTTP with tokens | 1 | Dropped for now (2026-10-15) |
-| 3a. Masked columns | 5, below | Runs 1a to 3 built (`2026-10-17-mcp-server-3a-1a-masked-rule.md`, `2026-10-17-mcp-server-3a-1b-masked-values.md`, `2026-10-17-mcp-server-3a-2-server.md`, `2026-10-17-mcp-server-3a-3-setting.md`): the rule, its stops, the engines' fences, the values, the server's side and the setting. Run 4, the editor's Masked columns control and the website's guide, is next (order changed on 2026-10-17) |
+| 3a. Masked columns | 5, below | Built (`2026-10-17-mcp-server-3a-1a-masked-rule.md`, `2026-10-17-mcp-server-3a-1b-masked-values.md`, `2026-10-17-mcp-server-3a-2-server.md`, `2026-10-17-mcp-server-3a-3-setting.md`, `2026-10-18-mcp-server-3a-4-columns-control.md`): the rule, its stops, the engines' fences, the values, the server's side, the setting, and the editor's control with the website's guide. 3b is next (order changed on 2026-10-17) |
 | 3b. The Log view, in full | Not cut yet | After 3a |
 | 2. Approved writes | Not cut yet | Last, after 3b (moved on 2026-10-17) |
 
@@ -2840,7 +2893,8 @@ pull request:
 3. The setting in `connections.toml`, as 1c-2b's for tables
    (`2026-10-17-mcp-server-3a-3-setting.md`, built).
 4. The editor's Masked columns control with **Add column…** and its mark,
-   and the website's guide.
+   and the website's guide
+   (`2026-10-18-mcp-server-3a-4-columns-control.md`, built).
 
 **3b. The Log view, in full.** (This was a log window before 2026-10-13.)
 
````

- [ ] **Step 2: Check the whole of it**

Run: `cargo fmt --all --check`, clippy as above, `RUSTDOCFLAGS='-D warnings' cargo doc --locked --workspace --no-deps`, the count of em dashes in the master plan, and `cargo test --locked --workspace --all-targets` with both servers

Expected: fmt and clippy clean, the documentation builds without a warning, no em dash (`0`), and:

```text
3599 passed, 0 failed, 22 ignored
```

- [ ] **Step 3: Commit**

```bash
git add docs/superpowers/plans/2026-10-13-mcp-server.md
git commit -m "Say in the MCP plan that the Masked columns control is built"
```

## Self-review

- **Against the design.** The macOS board's Masked columns beside Tables, with its chips and **Add column…**; the Omarchy board's `masked` row; D.3's list and mark; C.3's line. The departures are in "What the plan asks, and what is built", each agreed on 2026-10-18 or carried from 1c-3.
- **A review before the plan.** A reviewer drove the draft through the app's tests in both looks: the save over a held change, keys and clicks in the list, a failed fetch, chips of the two controls side by side, and the Tables control after the code it shares was moved. It found the columns of one table taken for those of another whose name differs in case (a mark where the server finds the column), the row chosen changing as more columns came, "Loading the columns…" kept after the window's session was gone, a table whose name holds a dot offering no columns, a failed fetch shown as a table without columns, and the reference page saying a save is refused where it is not. All fixed, with a test each but the page. It also found a SQLite virtual table's hidden column marked wrongly, which the window's columns leave out: left as it is, and said in the master plan.
- **By eye.** Headless renders of both looks: the control beside Tables and as the `masked` row, the list of a table's columns with its note, and the mark with its line. Not committed.
- **Placeholders.** None: every step has its diff or its command and what it prints.
- **Names.** `LoadedTables::columns`, `columns_loading`, `columns_failed`, `named`, `has_column`, `offered_columns`, `OfferedColumn`, `ColumnsOffered`, `ConnectionForm::agents_masked_opened`, `agents_column_typed`, `agents_column_pick`, `agents_column_error`, `agents_column_focus`, `take_typed_column`, `App::load_agent_columns` and `follow_pick` are the same in every task that uses them.
