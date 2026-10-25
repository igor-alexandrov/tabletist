# DDL view, run 1: read and show. Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A table's, view's or materialized view's definition as read-only text, in a third view beside Rows and Structure, read from each engine as the spec says.

**Architecture:** `tabletist-db` gains `Connection::ddl` (one adapter method per engine, no default) returning a `Ddl` of parts, which `Ddl::text()` joins. The backend answers a new `ReadDdl` command; the tab holds the result in a `Fetch`, as it holds its structure. `src/ui/ddl.rs` draws the text in the SQL editor's colours beside a gutter, with its loading and error states; the header's switch becomes a tab list of Rows, Structure and DDL.

**Tech Stack:** Rust, egui (crmne fork), tokio-postgres, mysql_async, rusqlite.

**Spec:** `docs/superpowers/specs/2026-10-24-ddl-view-design.md`. **Base:** `d18267a7` (main, after v0.3.0).

---

## Run 1 of three

This plan is run 1: read and show. Run 2 (Copy, Open in SQL editor, jump links and outline, every key) and run 3 (find in the text, the macOS View menu) get plans of their own. Each run is its own pull request (decided with Igor, against the prompt's one).

## What the design asks, and what is built

| The design or the prompt | Built | Why |
| --- | --- | --- |
| The first view "Data" (boards) / "the build already says Rows" (prompt) | The switch said Data; it is renamed **Rows** / `rows` | The prompt's default |
| `macOS – Structure`'s **Show DDL** button, to remove | Nothing to remove | The build never had it |
| ⌘F "as in the SQL editor" | Run 3 | The SQL editor has no find |
| "Remembered per tab, like Rows and Structure" | The view is kept in `session.json` (Structure too, which was not) | It was not kept at all |
| "Cache with the schema cache" | The tab holds its DDL, as it holds its structure | There is no backend schema cache |
| "Timeouts follow the catalog queries" | No timeout | The catalog reads have none |
| `-- public.in_stock · view · pg_get_viewdef` (view board) | `-- public.in_stock · built from the catalog · PostgreSQL 17.2` | The prompt's one first line |
| `-- Triggers · SHOW CREATE TRIGGER` (MySQL board) | `-- Triggers` | The prompt: section comments only |
| A trigger and a foreign key wrapped over two lines (boards) | One line each | Nothing is reformatted |
| View text with bare table names (view board) | `FROM public.books b` | Read with an empty search path, so the text runs whatever the search path |
| SQLite statements without blank lines (board) | A blank line between statements | The prompt |
| The error's sentence inside the card (states board) | Under the card | As the app's other error states |
| Omarchy `R retry` | `try again R` | The button's word; `R` is Reload's key |

## Checks

Every task ends with these, all passing:

```bash
~/.cargo/bin/cargo fmt --all --check
~/.cargo/bin/cargo clippy --locked --workspace --all-targets -- -D warnings
TABLETIST_TEST_PG_URL=postgres://tabletist:tabletist@localhost:55432/tabletist TABLETIST_TEST_MYSQL_URL=mysql://tabletist:tabletist@localhost:53306/tabletist ~/.cargo/bin/cargo test --locked --workspace --all-targets
RUSTDOCFLAGS='-D warnings' ~/.cargo/bin/cargo doc --locked --workspace --no-deps
```

The server tests need PostgreSQL on 55432 and MySQL on 53306 (`ss -ltn`); without the two URLs they print "skipped" and pass. The MySQL triggers of `book_covers` are made by the tests only where the server trusts the tests' user to (`--log-bin-trust-function-creators=1`, as `compose.yaml` starts it); elsewhere that part says "skipped".

## File structure

- `crates/tabletist-db/src/ddl.rs` (new): `Ddl`, `Part`, `PartKind`, `Source`, and how the text is joined.
- `crates/tabletist-db/src/pg/ddl.rs` (new): PostgreSQL's text, built from the catalog.
- `crates/tabletist-db/src/{lib,adapter,mysql,sqlite,pg}.rs`: `Connection::ddl`, `Adapter::ddl`, MySQL's and SQLite's reads.
- `crates/tabletist-db/fixtures/bookshop/*.sql`: what the tests read.
- `crates/tabletist-db/tests/ddl.rs` and `tests/ddl/*.sql` (new): the engines' tests and their expected texts. `.github/workflows/ci.yml`: both integration jobs run them.
- `crates/tabletist-mcp/tests/{log,tools}.rs`: the Bookshop's new view in what the MCP tests count.
- `src/backend.rs`, `src/model.rs`, `src/app.rs`, `src/app/labels.rs`, `src/session.rs`: the command, the tab's fetch, when it is read, cancelled, forgotten, and kept. `src/history.rs`, `src/ui/history_tests.rs`: the kept tab's new field in their literals.
- `src/testing.rs`: the harness answers a `ReadDdl`, and clicks a tab as it clicks a button. `src/app/startup_tests.rs`: a tab reopened in the view.
- `src/ui/ddl.rs` (new): the view. `src/ui/data_view.rs`: the switch. `src/ui/workspace.rs`: where the view goes, its footer and its key line. `src/ui/keys.rs`: the arrows, and Reload's letter in the text. `src/ui/mod.rs`: the modules, and the switch's tests.
- `src/ui/ddl_tests.rs` (new): the view's headless tests.
- `docs/_guide/browsing-data.md`, `docs/_guide/omarchy.md`: the user guide.

---

### Before Task 1: the spec and the plan

- [ ] Commit the spec (`docs/superpowers/specs/2026-10-24-ddl-view-design.md`) and this plan (`docs/superpowers/plans/2026-10-24-ddl-view-1-read-and-show.md`):

```bash
git add docs/superpowers/specs/2026-10-24-ddl-view-design.md docs/superpowers/plans/2026-10-24-ddl-view-1-read-and-show.md
git commit -m "Spec and plan the DDL view, run 1"
```

---

### Task 1: The definition, read on each engine

**Files:**
- Modify: `.github/workflows/ci.yml`
- Modify: `crates/tabletist-db/fixtures/bookshop/mysql.sql`
- Modify: `crates/tabletist-db/fixtures/bookshop/postgres.sql`
- Modify: `crates/tabletist-db/fixtures/bookshop/sqlite.sql`
- Modify: `crates/tabletist-db/src/adapter.rs`
- Create: `crates/tabletist-db/src/ddl.rs`
- Modify: `crates/tabletist-db/src/lib.rs`
- Modify: `crates/tabletist-db/src/mysql.rs`
- Modify: `crates/tabletist-db/src/pg.rs`
- Create: `crates/tabletist-db/src/pg/ddl.rs`
- Modify: `crates/tabletist-db/src/sqlite.rs`
- Create: `crates/tabletist-db/tests/ddl.rs`
- Create: `crates/tabletist-db/tests/ddl/mysql_book_covers.sql`
- Create: `crates/tabletist-db/tests/ddl/mysql_in_stock.sql`
- Create: `crates/tabletist-db/tests/ddl/postgres_book_covers.sql`
- Create: `crates/tabletist-db/tests/ddl/postgres_in_stock.sql`
- Create: `crates/tabletist-db/tests/ddl/sqlite_book_covers.sql`
- Create: `crates/tabletist-db/tests/ddl/sqlite_in_stock.sql`
- Modify: `crates/tabletist-mcp/tests/log.rs`
- Modify: `crates/tabletist-mcp/tests/tools.rs`

- [ ] **Step 1: Write the failing tests**

The Bookshop's MCP expectations count the new view `in_stock`; the engines' tests read `book_covers` and `in_stock` against the text files.

````diff
diff --git a/crates/tabletist-db/tests/ddl.rs b/crates/tabletist-db/tests/ddl.rs
new file mode 100644
index 00000000..b6b4300d
--- /dev/null
+++ b/crates/tabletist-db/tests/ddl.rs
@@ -0,0 +1,851 @@
+//! The DDL view's text, on the Bookshop: what each engine's `ddl` reads of
+//! `book_covers` and `in_stock`, held to the text files under
+//! `tests/ddl/`; that PostgreSQL's text builds the same catalog again; that
+//! MySQL's is the server's own, and SQLite's what `sqlite_master` keeps.
+//!
+//! The PostgreSQL and MySQL tests need the servers (see AGENTS.md) and
+//! print "skipped" without them.
+
+#![allow(clippy::unwrap_used)]
+
+use tabletist_db::ddl::{PartKind, Source};
+use tabletist_db::{
+    Access, ConnectSpec, Connection, Ddl, Driver, Error, HostKeys, ObjectRef, Secrets, TlsMode,
+};
+
+/// The text file `tests/ddl/<name>.sql`.
+fn expected(name: &str) -> String {
+    let path = format!("{}/tests/ddl/{name}.sql", env!("CARGO_MANIFEST_DIR"));
+    std::fs::read_to_string(&path).unwrap_or_else(|error| panic!("{path}: {error}"))
+}
+
+/// `text` without what the server it was read on varies: PostgreSQL's
+/// version in the first line, MySQL's `AUTO_INCREMENT=` and the user a
+/// `DEFINER` names. A text file ends in a line break, which the text has
+/// not.
+fn steady(text: &str) -> String {
+    let lines: Vec<String> = text
+        .lines()
+        .map(|line| {
+            let line = match line.rsplit_once(" · PostgreSQL ") {
+                Some((start, _)) if line.starts_with("-- ") => format!("{start} · PostgreSQL"),
+                _ => line.to_owned(),
+            };
+            let line = cut_word(&line, " AUTO_INCREMENT=");
+            match line.find("DEFINER=`") {
+                Some(at) => {
+                    let rest = &line[at..];
+                    // DEFINER=`user`@`host`
+                    let end = rest
+                        .match_indices('`')
+                        .nth(3)
+                        .map_or(rest.len(), |(at, _)| at + 1);
+                    format!("{}DEFINER=?{}", &line[..at], &rest[end..])
+                }
+                None => line,
+            }
+        })
+        .collect();
+    format!("{}\n", lines.join("\n"))
+}
+
+/// `line` without `word` and the value after it, up to the next space.
+fn cut_word(line: &str, word: &str) -> String {
+    match line.find(word) {
+        Some(at) => {
+            let rest = &line[at + word.len()..];
+            let end = rest.find(' ').unwrap_or(rest.len());
+            format!("{}{}", &line[..at], &rest[end..])
+        }
+        None => line.to_owned(),
+    }
+}
+
+/// How many statements each part holds, by kind.
+fn counts(ddl: &Ddl) -> Vec<(PartKind, usize)> {
+    ddl.parts
+        .iter()
+        .map(|part| (part.kind, part.statements.len()))
+        .collect()
+}
+
+mod sqlite {
+    use super::*;
+
+    /// A read-only session on a new file holding the Bookshop, and
+    /// `more` run on it first.
+    async fn bookshop(more: &str) -> (Connection, tempfile::TempDir) {
+        let dir = tempfile::tempdir().unwrap();
+        let path = dir.path().join("bookshop.db");
+        tabletist_db::fixtures::write_sqlite_bookshop(&path).unwrap();
+        rusqlite::Connection::open(&path)
+            .unwrap()
+            .execute_batch(more)
+            .unwrap();
+        let connection = Connection::connect(&ConnectSpec::sqlite(&path), &Secrets::default())
+            .await
+            .unwrap();
+        (connection, dir)
+    }
+
+    pub(super) async fn ddl(name: &str) -> Ddl {
+        let (connection, _dir) = bookshop("").await;
+        connection.ddl(&ObjectRef::new("main", name)).await.unwrap()
+    }
+
+    #[tokio::test]
+    async fn book_covers_is_its_stored_text_then_its_indexes_and_triggers() {
+        let ddl = ddl("book_covers").await;
+        assert_eq!(steady(&ddl.text()), expected("sqlite_book_covers"));
+        assert_eq!(ddl.source(), Source::Stored);
+        assert_eq!(
+            counts(&ddl),
+            [
+                (PartKind::Object, 1),
+                (PartKind::Indexes, 2),
+                (PartKind::Triggers, 2),
+            ]
+        );
+    }
+
+    #[tokio::test]
+    async fn in_stock_is_its_stored_text() {
+        let ddl = ddl("in_stock").await;
+        assert_eq!(steady(&ddl.text()), expected("sqlite_in_stock"));
+        assert_eq!(counts(&ddl), [(PartKind::Object, 1)]);
+    }
+
+    /// An index SQLite made for a key keeps no text, and is left out; the
+    /// indexes come before the triggers, each by name, and a statement
+    /// stored without its `;` gets one.
+    #[tokio::test]
+    async fn automatic_indexes_are_left_out_and_the_rest_comes_in_order() {
+        let (connection, _dir) = bookshop(
+            "CREATE TABLE shelves (code TEXT UNIQUE, floor INTEGER, PRIMARY KEY (code, floor));
+             CREATE TRIGGER shelves_b AFTER INSERT ON shelves BEGIN SELECT 1; END;
+             CREATE INDEX shelves_z ON shelves (floor);
+             CREATE TRIGGER shelves_a AFTER DELETE ON shelves BEGIN SELECT 1; END;
+             CREATE INDEX shelves_m ON shelves (code, floor);",
+        )
+        .await;
+        let ddl = connection
+            .ddl(&ObjectRef::new("main", "shelves"))
+            .await
+            .unwrap();
+        assert_eq!(
+            ddl.text(),
+            "CREATE TABLE shelves (code TEXT UNIQUE, floor INTEGER, PRIMARY KEY (code, floor));\n\
+             \n\
+             CREATE INDEX shelves_m ON shelves (code, floor);\n\
+             \n\
+             CREATE INDEX shelves_z ON shelves (floor);\n\
+             \n\
+             CREATE TRIGGER shelves_a AFTER DELETE ON shelves BEGIN SELECT 1; END;\n\
+             \n\
+             CREATE TRIGGER shelves_b AFTER INSERT ON shelves BEGIN SELECT 1; END;"
+        );
+        assert!(!ddl.text().contains("autoindex"));
+    }
+
+    /// A trigger keeps the table's name as it was written in it: one
+    /// written in another case is the table's all the same.
+    #[tokio::test]
+    async fn a_trigger_on_the_table_in_another_case_is_the_tables() {
+        let (connection, _dir) = bookshop(
+            "CREATE TABLE shelves (code TEXT);
+             CREATE TRIGGER shelves_upper AFTER INSERT ON SHELVES BEGIN SELECT 1; END;",
+        )
+        .await;
+        let ddl = connection
+            .ddl(&ObjectRef::new("main", "shelves"))
+            .await
+            .unwrap();
+        assert_eq!(
+            counts(&ddl),
+            [(PartKind::Object, 1), (PartKind::Triggers, 1)]
+        );
+    }
+
+    #[tokio::test]
+    async fn a_name_that_is_no_table_or_view_is_an_error() {
+        let (connection, _dir) = bookshop("").await;
+        let error = connection
+            .ddl(&ObjectRef::new("main", "book_covers_book_id"))
+            .await
+            .unwrap_err();
+        assert_eq!(
+            error.to_string(),
+            "no such table or view: book_covers_book_id"
+        );
+    }
+}
+
+mod postgres {
+    use super::*;
+    use tokio::sync::OnceCell;
+
+    static BOOKSHOP: OnceCell<()> = OnceCell::const_new();
+
+    fn url() -> Option<String> {
+        let url = std::env::var("TABLETIST_TEST_PG_URL").ok();
+        url.filter(|url| !url.trim().is_empty())
+    }
+
+    /// A session that may do anything, on the URL's database or on
+    /// `database`.
+    async fn admin(url: &str, database: Option<&str>) -> tokio_postgres::Client {
+        let mut config: tokio_postgres::Config = url.parse().unwrap();
+        config.ssl_mode(tokio_postgres::config::SslMode::Disable);
+        if let Some(database) = database {
+            config.dbname(database);
+        }
+        let (client, session) = config.connect(tokio_postgres::NoTls).await.unwrap();
+        tokio::spawn(session);
+        client
+    }
+
+    /// `database`, made new and empty.
+    async fn fresh(url: &str, database: &str) -> tokio_postgres::Client {
+        let server = admin(url, None).await;
+        server
+            .batch_execute(&format!("DROP DATABASE IF EXISTS {database} WITH (FORCE)"))
+            .await
+            .unwrap();
+        server
+            .batch_execute(&format!("CREATE DATABASE {database}"))
+            .await
+            .unwrap();
+        admin(url, Some(database)).await
+    }
+
+    /// The Bookshop, in its database of its own, `bookshop`, as the agent
+    /// tests have it.
+    async fn load(url: &str) {
+        BOOKSHOP
+            .get_or_init(|| async {
+                let server = admin(url, None).await;
+                let made = server
+                    .query_opt("SELECT 1 FROM pg_database WHERE datname = 'bookshop'", &[])
+                    .await
+                    .unwrap()
+                    .is_some();
+                if !made {
+                    server
+                        .batch_execute("CREATE DATABASE bookshop")
+                        .await
+                        .unwrap();
+                }
+                admin(url, Some("bookshop"))
+                    .await
+                    .batch_execute(tabletist_db::fixtures::BOOKSHOP_POSTGRES_SQL)
+                    .await
+                    .unwrap();
+            })
+            .await;
+    }
+
+    /// A session of the app's on `database`, or `None` (test skipped).
+    async fn connect(database: &str, access: Access) -> Option<(Connection, String)> {
+        let Some(url) = url() else {
+            eprintln!("skipped: TABLETIST_TEST_PG_URL is not set");
+            return None;
+        };
+        load(&url).await;
+        let (mut spec, secrets) = ConnectSpec::from_url(&url).unwrap();
+        spec.tls = TlsMode::Disable;
+        spec.database = database.into();
+        let connection = Connection::connect_with(&spec, &secrets, &HostKeys::default(), access)
+            .await
+            .unwrap();
+        Some((connection, url))
+    }
+
+    pub(super) async fn bookshop() -> Option<Connection> {
+        connect("bookshop", Access::ReadOnly)
+            .await
+            .map(|(connection, _)| connection)
+    }
+
+    async fn ddl(connection: &Connection, name: &str) -> Ddl {
+        connection
+            .ddl(&ObjectRef::new("public", name))
+            .await
+            .unwrap()
+    }
+
+    #[tokio::test]
+    async fn book_covers_is_built_from_the_catalog() {
+        let Some(connection) = bookshop().await else {
+            return;
+        };
+        let ddl = ddl(&connection, "book_covers").await;
+        assert_eq!(steady(&ddl.text()), expected("postgres_book_covers"));
+        assert_eq!(ddl.source(), Source::Catalog);
+        assert_eq!(
+            counts(&ddl),
+            [
+                (PartKind::Object, 1),
+                (PartKind::Indexes, 2),
+                (PartKind::Triggers, 2),
+                (PartKind::Comments, 2),
+            ]
+        );
+    }
+
+    #[tokio::test]
+    async fn in_stock_is_its_definition_and_its_comment() {
+        let Some(connection) = bookshop().await else {
+            return;
+        };
+        let ddl = ddl(&connection, "in_stock").await;
+        assert_eq!(steady(&ddl.text()), expected("postgres_in_stock"));
+        assert_eq!(
+            counts(&ddl),
+            [(PartKind::Object, 1), (PartKind::Comments, 1)]
+        );
+    }
+
+    /// Every name has its schema, so the text runs whatever the search
+    /// path; a comment's quote is doubled.
+    #[tokio::test]
+    async fn names_have_their_schema_and_quotes_are_doubled() {
+        let Some(connection) = bookshop().await else {
+            return;
+        };
+        let text = ddl(&connection, "book_covers").await.text();
+        for said in [
+            "CREATE TABLE public.book_covers (",
+            "REFERENCES public.books(id)",
+            "ON public.book_covers USING btree (book_id)",
+            "EXECUTE FUNCTION public.book_covers_check()",
+            "COMMENT ON COLUMN public.book_covers.kind IS \
+             'front or back, as the shop''s scanner names it';",
+        ] {
+            assert!(text.contains(said), "{said} in\n{text}");
+        }
+        let text = ddl(&connection, "in_stock").await.text();
+        assert!(text.contains("FROM public.books b"), "{text}");
+        assert!(
+            text.contains(
+                "COMMENT ON VIEW public.in_stock IS 'Books in print, with their publisher''s name';"
+            ),
+            "{text}"
+        );
+    }
+
+    /// What only PostgreSQL's text is built for, each in the line it is
+    /// written in.
+    #[tokio::test]
+    async fn what_only_postgres_has_is_written() {
+        let Some(connection) = bookshop().await else {
+            return;
+        };
+        let cases = [
+            (
+                "price_changes",
+                "    id bigint GENERATED ALWAYS AS IDENTITY NOT NULL,",
+            ),
+            (
+                "price_changes",
+                "    change numeric(8,2) GENERATED ALWAYS AS ((new_price - old_price)) STORED,",
+            ),
+            (
+                "price_changes",
+                "    changed_on date DEFAULT CURRENT_DATE NOT NULL,",
+            ),
+            ("stock_moves", "PARTITION BY RANGE (moved_on);"),
+            (
+                "stock_moves_2026",
+                "CREATE TABLE public.stock_moves_2026 PARTITION OF public.stock_moves",
+            ),
+            (
+                "stock_moves_2026",
+                "FOR VALUES FROM ('2026-01-01') TO ('2027-01-01');",
+            ),
+            (
+                "basket_lines",
+                "CREATE UNLOGGED TABLE public.basket_lines (",
+            ),
+            (
+                "basket_lines",
+                "    CONSTRAINT basket_lines_quantity_check CHECK ((quantity > 0)),",
+            ),
+            (
+                "shelf_bookings",
+                "    shelf text COLLATE pg_catalog.\"C\" NOT NULL,",
+            ),
+            (
+                "shelf_bookings",
+                "    CONSTRAINT shelf_bookings_during_excl EXCLUDE USING gist (during WITH &&)",
+            ),
+            (
+                "book_covers",
+                "ALTER TABLE public.book_covers DISABLE TRIGGER book_covers_audit;",
+            ),
+            (
+                "publisher_totals",
+                "CREATE MATERIALIZED VIEW public.publisher_totals AS",
+            ),
+            ("publisher_totals", "WITH NO DATA;"),
+            (
+                "publisher_totals",
+                "CREATE UNIQUE INDEX publisher_totals_publisher_id \
+                 ON public.publisher_totals USING btree (publisher_id);",
+            ),
+        ];
+        for (name, line) in cases {
+            let text = ddl(&connection, name).await.text();
+            assert!(text.lines().any(|said| said == line), "{line} in\n{text}");
+        }
+        // A partition lists none of the columns its parent gives it.
+        let text = ddl(&connection, "stock_moves_2026").await.text();
+        assert!(!text.contains("book_id"), "{text}");
+    }
+
+    /// The tables, views and materialized views of the Bookshop, in an
+    /// order each can be made in: what it uses before it.
+    const MADE_IN_ORDER: &[&str] = &[
+        "publishers",
+        "authors",
+        "books",
+        "book_covers",
+        "customers",
+        "orders",
+        "order_items",
+        "stock_moves",
+        "stock_moves_2026",
+        "price_changes",
+        "basket_lines",
+        "shelf_bookings",
+        "recent_orders",
+        "customer_names",
+        "customer_tally",
+        "customer_sheet",
+        "in_stock",
+        "publisher_totals",
+    ];
+
+    /// What the catalog says of a relation, read on its own, apart from
+    /// the DDL: its kind, how it is logged and its comment; each column
+    /// with its type, NOT NULL, default, identity, what generates it, its
+    /// collation and its comment; each constraint, index and trigger, the
+    /// trigger's enabled state with it; a view's query, and a partitioned
+    /// table's key and a partition's bound.
+    async fn catalog_of(client: &tokio_postgres::Client, name: &str) -> Vec<String> {
+        let sql = "\
+            SELECT 'relation ' || c.relkind::text || ' ' || c.relpersistence::text || ' ' \
+                   || COALESCE(obj_description(c.oid, 'pg_class'), '-') || ' ' \
+                   || COALESCE(pg_get_viewdef(c.oid), '-') || ' ' \
+                   || COALESCE(pg_get_partkeydef(c.oid), '-') || ' ' \
+                   || COALESCE(pg_get_expr(c.relpartbound, c.oid), '-') \
+            FROM pg_class c WHERE c.oid = $1::text::regclass \
+            UNION ALL \
+            SELECT 'column ' || a.attnum || ' ' || a.attname || ' ' \
+                   || format_type(a.atttypid, a.atttypmod) || ' ' || a.attnotnull || ' ' \
+                   || COALESCE(pg_get_expr(d.adbin, d.adrelid), '-') || ' ' \
+                   || a.attidentity::text || '/' || a.attgenerated::text || ' ' \
+                   || COALESCE(co.collname::text, '-') || ' ' \
+                   || COALESCE(col_description(a.attrelid, a.attnum), '-') \
+            FROM pg_attribute a \
+            LEFT JOIN pg_attrdef d ON d.adrelid = a.attrelid AND d.adnum = a.attnum \
+            LEFT JOIN pg_collation co ON co.oid = a.attcollation \
+            WHERE a.attrelid = $1::text::regclass AND a.attnum > 0 AND NOT a.attisdropped \
+            UNION ALL \
+            SELECT 'constraint ' || conname || ' ' || pg_get_constraintdef(oid) || ' ' \
+                   || convalidated::text \
+            FROM pg_constraint WHERE conrelid = $1::text::regclass \
+            UNION ALL \
+            SELECT 'index ' || pg_get_indexdef(indexrelid) \
+            FROM pg_index WHERE indrelid = $1::text::regclass \
+            UNION ALL \
+            SELECT 'trigger ' || pg_get_triggerdef(oid) || ' ' || tgenabled::text \
+            FROM pg_trigger WHERE tgrelid = $1::text::regclass AND NOT tgisinternal \
+            ORDER BY 1";
+        let name = format!("public.{name}");
+        client
+            .query(sql, &[&name])
+            .await
+            .unwrap()
+            .iter()
+            .map(|row| row.get(0))
+            .collect()
+    }
+
+    /// The DDL of every table and view of the Bookshop, run into a new
+    /// database, makes a catalog that says what the Bookshop's says. The
+    /// functions the triggers and a view call are made first: the DDL
+    /// leaves them out.
+    #[tokio::test]
+    async fn the_text_builds_the_same_catalog_again() {
+        let Some((connection, url)) = connect("bookshop", Access::ReadOnly).await else {
+            return;
+        };
+        let source = admin(&url, Some("bookshop")).await;
+        // Every one, and only those: a relation the fixture gains is
+        // made here too.
+        let listed: Vec<String> = source
+            .query(
+                "SELECT c.relname::text FROM pg_class c \
+                 JOIN pg_namespace n ON n.oid = c.relnamespace \
+                 WHERE n.nspname = 'public' AND c.relkind IN ('r', 'p', 'v', 'm') \
+                 ORDER BY 1",
+                &[],
+            )
+            .await
+            .unwrap()
+            .iter()
+            .map(|row| row.get(0))
+            .collect();
+        let mut made: Vec<&str> = MADE_IN_ORDER.to_vec();
+        made.sort();
+        assert_eq!(listed, made);
+        let copy = fresh(&url, "tabletist_ddl_copy").await;
+        copy.batch_execute(
+            "SET check_function_bodies = off;
+             CREATE FUNCTION public.book_covers_check() RETURNS trigger LANGUAGE plpgsql
+                 AS $$BEGIN RETURN NEW; END$$;
+             CREATE FUNCTION public.customer_count() RETURNS bigint LANGUAGE sql STABLE
+                 AS 'SELECT count(*) FROM public.customers';",
+        )
+        .await
+        .unwrap();
+        for name in MADE_IN_ORDER {
+            let text = ddl(&connection, name).await.text();
+            copy.batch_execute(&text)
+                .await
+                .unwrap_or_else(|error| panic!("{name}: {error}\n{text}"));
+        }
+        for name in MADE_IN_ORDER {
+            assert_eq!(
+                catalog_of(&copy, name).await,
+                catalog_of(&source, name).await,
+                "{name}"
+            );
+        }
+        // The copy goes when the test passes; one that failed stays to be
+        // looked at.
+        drop(copy);
+        admin(&url, None)
+            .await
+            .batch_execute("DROP DATABASE tabletist_ddl_copy WITH (FORCE)")
+            .await
+            .unwrap();
+    }
+
+    /// What `SHOW search_path` answers on `connection`.
+    async fn search_path(connection: &Connection) -> String {
+        let statements =
+            tabletist_db::sql::statements(tabletist_db::Dialect::Postgres, "SHOW search_path");
+        let stop = tabletist_db::StopFlag::new();
+        let outcome = connection
+            .run_script(&statements, 10, tabletist_db::ScriptMode::ReadOnly, &stop)
+            .await
+            .unwrap();
+        format!("{:?}", outcome.results[0].outcome)
+    }
+
+    /// What only a table's own text can say comes back too: what a
+    /// partition and a child set themselves on the columns their parents
+    /// give them, a check that does not hold for the rows already there,
+    /// and a partitioned table's index whose name holds `ON ONLY`.
+    #[tokio::test]
+    async fn what_a_table_set_itself_comes_back() {
+        let Some(url) = url() else {
+            eprintln!("skipped: TABLETIST_TEST_PG_URL is not set");
+            return;
+        };
+        let source = fresh(&url, "tabletist_ddl_cases").await;
+        source
+            .batch_execute(
+                "CREATE TABLE parts (kind integer NOT NULL, note text DEFAULT 'none')
+                     PARTITION BY LIST (kind);
+                 CREATE TABLE parts_one PARTITION OF parts FOR VALUES IN (1);
+                 ALTER TABLE parts_one ALTER COLUMN note SET NOT NULL,
+                     ALTER COLUMN note SET DEFAULT 'one';
+                 CREATE INDEX \"by ON ONLY kind\" ON parts (kind);
+                 CREATE TABLE base (id integer, note text DEFAULT 'base');
+                 CREATE TABLE child () INHERITS (base);
+                 ALTER TABLE child ALTER COLUMN note DROP DEFAULT,
+                     ALTER COLUMN id SET NOT NULL;
+                 CREATE TABLE amounts (n integer);
+                 INSERT INTO amounts VALUES (5);
+                 ALTER TABLE amounts ADD CONSTRAINT amounts_small CHECK (n < 3) NOT VALID;",
+            )
+            .await
+            .unwrap();
+        let (connection, _) = connect("tabletist_ddl_cases", Access::ReadOnly)
+            .await
+            .unwrap();
+        let copy = fresh(&url, "tabletist_ddl_cases_copy").await;
+        let names = ["parts", "parts_one", "base", "child", "amounts"];
+        for name in names {
+            let text = ddl(&connection, name).await.text();
+            copy.batch_execute(&text)
+                .await
+                .unwrap_or_else(|error| panic!("{name}: {error}\n{text}"));
+        }
+        for name in names {
+            assert_eq!(
+                catalog_of(&copy, name).await,
+                catalog_of(&source, name).await,
+                "{name}"
+            );
+        }
+        drop((source, copy, connection));
+        let server = admin(&url, None).await;
+        for database in ["tabletist_ddl_cases", "tabletist_ddl_cases_copy"] {
+            server
+                .batch_execute(&format!("DROP DATABASE {database} WITH (FORCE)"))
+                .await
+                .unwrap();
+        }
+    }
+
+    /// The read leaves the session as it was: the search path it empties
+    /// is the transaction's, which is gone. A session that writes reads it
+    /// the same way.
+    #[tokio::test]
+    async fn the_read_leaves_the_session_as_it_was() {
+        let Some((connection, _)) = connect("bookshop", Access::Writable).await else {
+            return;
+        };
+        let before = search_path(&connection).await;
+        ddl(&connection, "book_covers").await;
+        assert_eq!(search_path(&connection).await, before);
+    }
+
+    #[tokio::test]
+    async fn every_bookshop_table_and_view_has_a_ddl_on_a_read_only_session() {
+        let Some(connection) = bookshop().await else {
+            return;
+        };
+        for name in MADE_IN_ORDER {
+            ddl(&connection, name).await;
+        }
+    }
+}
+
+mod mysql {
+    use super::*;
+    use mysql_async::prelude::Queryable;
+    use tokio::sync::OnceCell;
+
+    /// Loaded once for this file's tests, with the triggers the fixture
+    /// cannot make: whether the server let the tests' user make them.
+    static BOOKSHOP: OnceCell<bool> = OnceCell::const_new();
+
+    fn url() -> Option<String> {
+        let url = std::env::var("TABLETIST_TEST_MYSQL_URL").ok();
+        url.filter(|url| !url.trim().is_empty())
+    }
+
+    /// A session of the driver's own on `database`.
+    async fn admin(url: &str, database: &str) -> mysql_async::Conn {
+        let (server, _database) = url.rsplit_once('/').unwrap();
+        let opts = mysql_async::Opts::from_url(&format!("{server}/{database}?prefer_socket=false"))
+            .unwrap();
+        mysql_async::Conn::new(opts).await.unwrap()
+    }
+
+    /// The Bookshop, in its database of its own (compose/mysql-init.sql
+    /// makes it), and `book_covers`' two triggers, made in the order their
+    /// names are not. Whether they were made: with its binary log on, the
+    /// server lets the tests' user make one only where compose.yaml has it
+    /// trust them.
+    async fn load(url: &str) -> bool {
+        *BOOKSHOP
+            .get_or_init(|| async {
+                let mut conn = admin(url, "bookshop").await;
+                for statement in tabletist_db::fixtures::BOOKSHOP_MYSQL_SQL.split(";\n") {
+                    let statement: String = statement
+                        .lines()
+                        .filter(|line| !line.trim_start().starts_with("--"))
+                        .collect::<Vec<_>>()
+                        .join("\n");
+                    if !statement.trim().is_empty() {
+                        conn.query_drop(statement).await.unwrap();
+                    }
+                }
+                let made = conn
+                    .query_drop(
+                        "CREATE TRIGGER book_covers_lower_kind BEFORE INSERT ON book_covers \
+                         FOR EACH ROW SET NEW.kind = LOWER(NEW.kind)",
+                    )
+                    .await;
+                if let Err(error) = made {
+                    eprintln!("the server lets the tests' user make no trigger: {error}");
+                    return false;
+                }
+                conn.query_drop(
+                    "CREATE TRIGGER book_covers_keep_kind BEFORE UPDATE ON book_covers \
+                     FOR EACH ROW SET NEW.kind = LOWER(NEW.kind)",
+                )
+                .await
+                .unwrap();
+                true
+            })
+            .await
+    }
+
+    /// A read-only session of the app's on the Bookshop, whether
+    /// `book_covers` has its triggers, and the URL; or `None` (test
+    /// skipped).
+    pub(super) async fn bookshop() -> Option<(Connection, bool, String)> {
+        let Some(url) = url() else {
+            eprintln!("skipped: TABLETIST_TEST_MYSQL_URL is not set");
+            return None;
+        };
+        let triggers = load(&url).await;
+        let (mut spec, secrets) = ConnectSpec::from_url(&url).unwrap();
+        spec.tls = TlsMode::Disable;
+        spec.database = "bookshop".into();
+        let connection =
+            Connection::connect_with(&spec, &secrets, &HostKeys::default(), Access::ReadOnly)
+                .await
+                .unwrap();
+        Some((connection, triggers, url))
+    }
+
+    async fn ddl(connection: &Connection, name: &str) -> Ddl {
+        connection
+            .ddl(&ObjectRef::new("bookshop", name))
+            .await
+            .unwrap()
+    }
+
+    /// `SHOW CREATE ...`'s text, read by the driver alone.
+    async fn shown(url: &str, sql: &str, column: usize) -> String {
+        let row: mysql_async::Row = admin(url, "bookshop")
+            .await
+            .query_first(sql)
+            .await
+            .unwrap()
+            .unwrap();
+        row.get(column).unwrap()
+    }
+
+    #[tokio::test]
+    async fn book_covers_is_what_the_server_shows_then_its_triggers_by_name() {
+        let Some((connection, triggers, url)) = bookshop().await else {
+            return;
+        };
+        let ddl = ddl(&connection, "book_covers").await;
+        assert_eq!(ddl.source(), Source::Stored);
+        // The server's own text, byte for byte.
+        assert_eq!(
+            ddl.parts[0].statements,
+            [shown(&url, "SHOW CREATE TABLE book_covers", 1).await]
+        );
+        if !triggers {
+            eprintln!("skipped: book_covers has no triggers on this server");
+            return;
+        }
+        assert_eq!(steady(&ddl.text()), expected("mysql_book_covers"));
+        let triggers = ddl.part(PartKind::Triggers).unwrap();
+        assert_eq!(
+            triggers.statements,
+            [
+                shown(&url, "SHOW CREATE TRIGGER book_covers_keep_kind", 2).await,
+                shown(&url, "SHOW CREATE TRIGGER book_covers_lower_kind", 2).await,
+            ]
+        );
+        assert_eq!(
+            counts(&ddl),
+            [(PartKind::Object, 1), (PartKind::Triggers, 2)]
+        );
+    }
+
+    #[tokio::test]
+    async fn in_stock_is_what_the_server_shows() {
+        let Some((connection, _, url)) = bookshop().await else {
+            return;
+        };
+        let ddl = ddl(&connection, "in_stock").await;
+        assert_eq!(
+            ddl.parts[0].statements,
+            [shown(&url, "SHOW CREATE VIEW in_stock", 1).await]
+        );
+        assert_eq!(steady(&ddl.text()), expected("mysql_in_stock"));
+    }
+
+    /// A user who may read a view but not see its definition is told so
+    /// by the server: its number and words, and no text at all.
+    #[tokio::test]
+    async fn a_user_who_may_not_see_the_definition_gets_the_servers_error() {
+        let Some((_, _, url)) = bookshop().await else {
+            return;
+        };
+        admin(&url, "tabletist")
+            .await
+            .query_drop("CREATE OR REPLACE VIEW ddl_denied AS SELECT 1 AS one")
+            .await
+            .unwrap();
+        // compose/mysql-init.sql's `writer`, who has SELECT and no SHOW VIEW.
+        let (mut spec, mut secrets) = ConnectSpec::from_url(&url).unwrap();
+        spec.tls = TlsMode::Disable;
+        spec.user = "writer".into();
+        secrets.password = Some("writer".into());
+        let connection =
+            match Connection::connect_with(&spec, &secrets, &HostKeys::default(), Access::ReadOnly)
+                .await
+            {
+                Ok(connection) => connection,
+                Err(error) => {
+                    eprintln!("skipped: the server has no `writer`: {error}");
+                    return;
+                }
+            };
+        let error = connection
+            .ddl(&ObjectRef::new("tabletist", "ddl_denied"))
+            .await
+            .unwrap_err();
+        admin(&url, "tabletist")
+            .await
+            .query_drop("DROP VIEW ddl_denied")
+            .await
+            .unwrap();
+        let Error::Query { message, named, .. } = &error else {
+            panic!("{error:?}");
+        };
+        assert_eq!(named.number, Some(1142), "{error:?}");
+        assert!(
+            message.starts_with("SHOW VIEW command denied to user 'writer'"),
+            "{message}"
+        );
+    }
+}
+
+/// Every engine reads a DDL of the Bookshop's `book_covers`, its `CREATE
+/// TABLE` first: named by `Driver`, so a new engine is named here too.
+#[tokio::test]
+async fn every_engine_reads_book_covers() {
+    for driver in [Driver::Sqlite, Driver::Postgres, Driver::MySql] {
+        let ddl = match driver {
+            Driver::Sqlite => Some(sqlite::ddl("book_covers").await),
+            Driver::Postgres => match postgres::bookshop().await {
+                Some(connection) => Some(
+                    connection
+                        .ddl(&ObjectRef::new("public", "book_covers"))
+                        .await
+                        .unwrap(),
+                ),
+                None => None,
+            },
+            Driver::MySql => match mysql::bookshop().await {
+                Some((connection, _, _)) => Some(
+                    connection
+                        .ddl(&ObjectRef::new("bookshop", "book_covers"))
+                        .await
+                        .unwrap(),
+                ),
+                None => None,
+            },
+        };
+        let Some(ddl) = ddl else {
+            continue;
+        };
+        assert_eq!(ddl.driver, driver);
+        assert_eq!(ddl.parts[0].kind, PartKind::Object);
+        assert!(ddl.text().contains("TABLE"), "{driver:?}");
+    }
+}
diff --git a/crates/tabletist-db/tests/ddl/mysql_book_covers.sql b/crates/tabletist-db/tests/ddl/mysql_book_covers.sql
new file mode 100644
index 00000000..9183cc1b
--- /dev/null
+++ b/crates/tabletist-db/tests/ddl/mysql_book_covers.sql
@@ -0,0 +1,14 @@
+CREATE TABLE `book_covers` (
+  `id` int NOT NULL,
+  `book_id` int NOT NULL,
+  `kind` varchar(16) NOT NULL,
+  `image` blob,
+  PRIMARY KEY (`id`),
+  UNIQUE KEY `book_covers_book_id_kind` (`book_id`,`kind`),
+  KEY `book_covers_book_id` (`book_id`),
+  CONSTRAINT `book_covers_ibfk_1` FOREIGN KEY (`book_id`) REFERENCES `books` (`id`)
+) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_0900_ai_ci;
+
+-- Triggers
+CREATE DEFINER=? TRIGGER `book_covers_keep_kind` BEFORE UPDATE ON `book_covers` FOR EACH ROW SET NEW.kind = LOWER(NEW.kind);
+CREATE DEFINER=? TRIGGER `book_covers_lower_kind` BEFORE INSERT ON `book_covers` FOR EACH ROW SET NEW.kind = LOWER(NEW.kind);
diff --git a/crates/tabletist-db/tests/ddl/mysql_in_stock.sql b/crates/tabletist-db/tests/ddl/mysql_in_stock.sql
new file mode 100644
index 00000000..46f6cceb
--- /dev/null
+++ b/crates/tabletist-db/tests/ddl/mysql_in_stock.sql
@@ -0,0 +1 @@
+CREATE ALGORITHM=UNDEFINED DEFINER=? SQL SECURITY DEFINER VIEW `in_stock` AS select `b`.`id` AS `book_id`,`b`.`title` AS `title`,`p`.`name` AS `publisher` from (`books` `b` join `publishers` `p` on((`p`.`id` = `b`.`publisher_id`))) where (0 <> `b`.`in_print`);
diff --git a/crates/tabletist-db/tests/ddl/postgres_book_covers.sql b/crates/tabletist-db/tests/ddl/postgres_book_covers.sql
new file mode 100644
index 00000000..75689245
--- /dev/null
+++ b/crates/tabletist-db/tests/ddl/postgres_book_covers.sql
@@ -0,0 +1,22 @@
+-- public.book_covers · built from the catalog · PostgreSQL
+CREATE TABLE public.book_covers (
+    id integer NOT NULL,
+    book_id integer NOT NULL,
+    kind text NOT NULL,
+    image bytea,
+    CONSTRAINT book_covers_pkey PRIMARY KEY (id),
+    CONSTRAINT book_covers_book_id_fkey FOREIGN KEY (book_id) REFERENCES public.books(id)
+);
+
+-- Indexes
+CREATE INDEX book_covers_book_id ON public.book_covers USING btree (book_id);
+CREATE UNIQUE INDEX book_covers_book_id_kind ON public.book_covers USING btree (book_id, kind);
+
+-- Triggers
+CREATE TRIGGER book_covers_audit AFTER DELETE ON public.book_covers FOR EACH ROW EXECUTE FUNCTION public.book_covers_check();
+ALTER TABLE public.book_covers DISABLE TRIGGER book_covers_audit;
+CREATE TRIGGER book_covers_check BEFORE INSERT OR UPDATE OF image ON public.book_covers FOR EACH ROW EXECUTE FUNCTION public.book_covers_check();
+
+-- Comments
+COMMENT ON TABLE public.book_covers IS 'Cover images, one for each side of a book';
+COMMENT ON COLUMN public.book_covers.kind IS 'front or back, as the shop''s scanner names it';
diff --git a/crates/tabletist-db/tests/ddl/postgres_in_stock.sql b/crates/tabletist-db/tests/ddl/postgres_in_stock.sql
new file mode 100644
index 00000000..a10b50cc
--- /dev/null
+++ b/crates/tabletist-db/tests/ddl/postgres_in_stock.sql
@@ -0,0 +1,11 @@
+-- public.in_stock · built from the catalog · PostgreSQL
+CREATE OR REPLACE VIEW public.in_stock AS
+ SELECT b.id AS book_id,
+    b.title,
+    p.name AS publisher
+   FROM public.books b
+     JOIN public.publishers p ON p.id = b.publisher_id
+  WHERE b.in_print;
+
+-- Comments
+COMMENT ON VIEW public.in_stock IS 'Books in print, with their publisher''s name';
diff --git a/crates/tabletist-db/tests/ddl/sqlite_book_covers.sql b/crates/tabletist-db/tests/ddl/sqlite_book_covers.sql
new file mode 100644
index 00000000..f8f492ea
--- /dev/null
+++ b/crates/tabletist-db/tests/ddl/sqlite_book_covers.sql
@@ -0,0 +1,20 @@
+CREATE TABLE book_covers (
+    id INTEGER PRIMARY KEY,
+    book_id INTEGER NOT NULL REFERENCES books (id),
+    kind TEXT NOT NULL,
+    image BLOB
+);
+
+CREATE INDEX book_covers_book_id ON book_covers (book_id);
+
+CREATE UNIQUE INDEX book_covers_book_id_kind ON book_covers (book_id, kind);
+
+CREATE TRIGGER book_covers_keep_kind AFTER UPDATE OF kind ON book_covers
+BEGIN
+    UPDATE book_covers SET kind = lower(NEW.kind) WHERE id = NEW.id;
+END;
+
+CREATE TRIGGER book_covers_lower_kind AFTER INSERT ON book_covers
+BEGIN
+    UPDATE book_covers SET kind = lower(NEW.kind) WHERE id = NEW.id;
+END;
diff --git a/crates/tabletist-db/tests/ddl/sqlite_in_stock.sql b/crates/tabletist-db/tests/ddl/sqlite_in_stock.sql
new file mode 100644
index 00000000..173a55a5
--- /dev/null
+++ b/crates/tabletist-db/tests/ddl/sqlite_in_stock.sql
@@ -0,0 +1,4 @@
+CREATE VIEW in_stock AS
+    SELECT b.id AS book_id, b.title, p.name AS publisher
+    FROM books b JOIN publishers p ON p.id = b.publisher_id
+    WHERE b.in_print;
diff --git a/crates/tabletist-mcp/tests/log.rs b/crates/tabletist-mcp/tests/log.rs
index 2fbc4bdd..38f340ad 100644
--- a/crates/tabletist-mcp/tests/log.rs
+++ b/crates/tabletist-mcp/tests/log.rs
@@ -96,7 +96,7 @@ async fn every_call_is_an_entry_with_what_was_asked_and_what_came_of_it() {
         Entry {
             connection_id: on_dev.clone(),
             connection: Some(DEV.into()),
-            rows: Some(9),
+            rows: Some(10),
             ..entry("describe_schema", "ok")
         },
         Entry {
diff --git a/crates/tabletist-mcp/tests/tools.rs b/crates/tabletist-mcp/tests/tools.rs
index 75473b1a..23633c54 100644
--- a/crates/tabletist-mcp/tests/tools.rs
+++ b/crates/tabletist-mcp/tests/tools.rs
@@ -497,6 +497,7 @@ async fn describe_schema_lists_the_tables_and_views() {
             ("books", "table"),
             ("customer_names", "view"),
             ("customers", "table"),
+            ("in_stock", "view"),
             ("order_items", "table"),
             ("orders", "table"),
             ("publishers", "table"),
````

- [ ] **Step 2: Run them to see them fail**

```bash
TABLETIST_TEST_PG_URL=postgres://tabletist:tabletist@localhost:55432/tabletist TABLETIST_TEST_MYSQL_URL=mysql://tabletist:tabletist@localhost:53306/tabletist ~/.cargo/bin/cargo test --locked -p tabletist-db --test ddl
```

Expected: it does not compile, `Connection` has no `ddl`:

```
error[E0432]: unresolved import `tabletist_db::ddl`
error[E0432]: unresolved import `tabletist_db::Ddl`
error[E0599]: no method named `ddl` found for struct `tabletist_db::Connection` in the current scope
error[E0599]: no method named `ddl` found for reference `&tabletist_db::Connection` in the current scope
error[E0599]: no method named `ddl` found for reference `&tabletist_db::Connection` in the current scope
error[E0599]: no method named `ddl` found for struct `tabletist_db::Connection` in the current scope
error[E0599]: no method named `ddl` found for struct `tabletist_db::Connection` in the current scope
error[E0599]: no method named `ddl` found for struct `tabletist_db::Connection` in the current scope
error[E0599]: no method named `ddl` found for struct `tabletist_db::Connection` in the current scope
error[E0599]: no method named `ddl` found for struct `tabletist_db::Connection` in the current scope
error[E0599]: no method named `ddl` found for struct `tabletist_db::Connection` in the current scope
```

```bash
~/.cargo/bin/cargo test --locked -p tabletist-mcp --test log --test tools
```

Expected: the Bookshop has no `in_stock` yet:

```
test every_call_is_an_entry_with_what_was_asked_and_what_came_of_it ... FAILED
test result: FAILED. 4 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
```

(Cargo stops at the first suite that fails; `tools` fails the same way, its list of tables lacking `in_stock`.)

- [ ] **Step 3: Write the read, and the fixtures it reads**

````diff
diff --git a/.github/workflows/ci.yml b/.github/workflows/ci.yml
index 43fbc73e..8505ba08 100644
--- a/.github/workflows/ci.yml
+++ b/.github/workflows/ci.yml
@@ -100,7 +100,7 @@ jobs:
             pg_isready -h localhost -p 55432 && break
             sleep 1
           done
-      - run: cargo test --locked -p tabletist-db --lib --test postgres --test insert_audit --test lookup --test delete_rows --test agent
+      - run: cargo test --locked -p tabletist-db --lib --test postgres --test insert_audit --test lookup --test delete_rows --test agent --test ddl
         env:
           TABLETIST_TEST_PG_URL: postgres://tabletist:tabletist@localhost:55432/tabletist
       # The app's gateway for the MCP server, which opens its own sessions.
@@ -123,7 +123,7 @@ jobs:
       - uses: Swatinem/rust-cache@v2
       - name: Start MySQL
         run: docker compose up -d --wait mysql
-      - run: cargo test --locked -p tabletist-db --lib --test mysql --test insert_audit --test lookup --test delete_rows --test agent
+      - run: cargo test --locked -p tabletist-db --lib --test mysql --test insert_audit --test lookup --test delete_rows --test agent --test ddl
         env:
           TABLETIST_TEST_MYSQL_URL: mysql://tabletist:tabletist@localhost:53306/tabletist
       # The app's gateway for the MCP server, which opens its own sessions.
diff --git a/crates/tabletist-db/fixtures/bookshop/mysql.sql b/crates/tabletist-db/fixtures/bookshop/mysql.sql
index 7dc357da..09827b46 100644
--- a/crates/tabletist-db/fixtures/bookshop/mysql.sql
+++ b/crates/tabletist-db/fixtures/bookshop/mysql.sql
@@ -4,7 +4,7 @@
 -- Loaded into a database of its own, `bookshop` (compose/mysql-init.sql).
 -- One statement for each `;` at the end of a line.
 
-DROP VIEW IF EXISTS recent_orders, customer_names, shop_greeting;
+DROP VIEW IF EXISTS recent_orders, customer_names, shop_greeting, in_stock;
 DROP TABLE IF EXISTS order_items, orders, customers, book_covers, books, authors, publishers;
 
 CREATE TABLE publishers (
@@ -169,3 +169,13 @@ CREATE VIEW customer_names AS SELECT id, name FROM customers;
 -- as it lists nothing to a user without SHOW VIEW: here alone, as that is
 -- MySQL's.
 CREATE VIEW shop_greeting AS SELECT 'Welcome' AS greeting;
+
+-- What the DDL view is tested on: a table with indexes, and a view. Its
+-- tests give the table its triggers: with its binary log on, the server
+-- lets the tests' user make one only where compose.yaml has it trust them.
+CREATE INDEX book_covers_book_id ON book_covers (book_id);
+CREATE UNIQUE INDEX book_covers_book_id_kind ON book_covers (book_id, kind);
+CREATE VIEW in_stock AS
+    SELECT b.id AS book_id, b.title, p.name AS publisher
+    FROM books b JOIN publishers p ON p.id = b.publisher_id
+    WHERE b.in_print;
diff --git a/crates/tabletist-db/fixtures/bookshop/postgres.sql b/crates/tabletist-db/fixtures/bookshop/postgres.sql
index 2563b068..e6198c37 100644
--- a/crates/tabletist-db/fixtures/bookshop/postgres.sql
+++ b/crates/tabletist-db/fixtures/bookshop/postgres.sql
@@ -3,12 +3,14 @@
 -- engine. The three files hold the same rows. Keep them deterministic.
 -- Loaded into a database of its own, `bookshop`, which the tests make.
 
-DROP VIEW IF EXISTS customer_tally, customer_sheet;
-DROP TABLE IF EXISTS stock_moves;
+DROP MATERIALIZED VIEW IF EXISTS publisher_totals;
+DROP VIEW IF EXISTS customer_tally, customer_sheet, in_stock;
+DROP TABLE IF EXISTS stock_moves, price_changes, basket_lines, shelf_bookings;
 DROP FUNCTION IF EXISTS customer_count();
 DROP FUNCTION IF EXISTS book_reach(books);
 DROP VIEW IF EXISTS recent_orders, customer_names;
 DROP TABLE IF EXISTS order_items, orders, customers, book_covers, books, authors, publishers;
+DROP FUNCTION IF EXISTS book_covers_check();
 
 CREATE TABLE publishers (
     id integer PRIMARY KEY,
@@ -187,3 +189,51 @@ CREATE TABLE stock_moves_2026 PARTITION OF stock_moves
 INSERT INTO stock_moves (book_id, moved_on, quantity) VALUES
     (1, '2026-02-03', 4),
     (2, '2026-03-14', -1);
+
+-- What the DDL view is tested on: a table with indexes, triggers (one
+-- disabled) and comments, and a view, as on the other engines; and here
+-- alone, what only PostgreSQL's text is built for. The triggers are made
+-- in the order their names are not: the text lists them by name.
+CREATE INDEX book_covers_book_id ON book_covers (book_id);
+CREATE UNIQUE INDEX book_covers_book_id_kind ON book_covers (book_id, kind);
+CREATE FUNCTION book_covers_check() RETURNS trigger LANGUAGE plpgsql
+    AS $$BEGIN RETURN NEW; END$$;
+CREATE TRIGGER book_covers_check BEFORE INSERT OR UPDATE OF image ON book_covers
+    FOR EACH ROW EXECUTE FUNCTION book_covers_check();
+CREATE TRIGGER book_covers_audit AFTER DELETE ON book_covers
+    FOR EACH ROW EXECUTE FUNCTION book_covers_check();
+ALTER TABLE book_covers DISABLE TRIGGER book_covers_audit;
+COMMENT ON TABLE book_covers IS 'Cover images, one for each side of a book';
+COMMENT ON COLUMN book_covers.kind IS 'front or back, as the shop''s scanner names it';
+CREATE VIEW in_stock AS
+    SELECT b.id AS book_id, b.title, p.name AS publisher
+    FROM books b JOIN publishers p ON p.id = b.publisher_id
+    WHERE b.in_print;
+COMMENT ON VIEW in_stock IS 'Books in print, with their publisher''s name';
+-- An identity and a generated column.
+CREATE TABLE price_changes (
+    id bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
+    book_id integer NOT NULL REFERENCES books (id),
+    old_price numeric(8, 2) NOT NULL,
+    new_price numeric(8, 2) NOT NULL,
+    change numeric(8, 2) GENERATED ALWAYS AS (new_price - old_price) STORED,
+    changed_on date NOT NULL DEFAULT CURRENT_DATE
+);
+-- A table the server does not log, with a check of its own.
+CREATE UNLOGGED TABLE basket_lines (
+    customer_id integer NOT NULL REFERENCES customers (id),
+    book_id integer NOT NULL REFERENCES books (id),
+    quantity integer NOT NULL CHECK (quantity > 0)
+);
+-- An exclusion constraint: one booking of a shelf at a time.
+CREATE TABLE shelf_bookings (
+    shelf text COLLATE "C" NOT NULL,
+    during daterange NOT NULL,
+    EXCLUDE USING gist (during WITH &&)
+);
+-- A materialized view, with an index.
+CREATE MATERIALIZED VIEW publisher_totals AS
+    SELECT p.id AS publisher_id, count(b.id) AS books
+    FROM publishers p LEFT JOIN books b ON b.publisher_id = p.id
+    GROUP BY p.id;
+CREATE UNIQUE INDEX publisher_totals_publisher_id ON publisher_totals (publisher_id);
diff --git a/crates/tabletist-db/fixtures/bookshop/sqlite.sql b/crates/tabletist-db/fixtures/bookshop/sqlite.sql
index 9f2ee019..82b1d9af 100644
--- a/crates/tabletist-db/fixtures/bookshop/sqlite.sql
+++ b/crates/tabletist-db/fixtures/bookshop/sqlite.sql
@@ -153,3 +153,21 @@ INSERT INTO order_items (id, order_id, book_id, quantity, price) VALUES
 CREATE VIEW recent_orders AS
     SELECT id, email, placed_at, total FROM orders WHERE placed_at >= '2026-01-01';
 CREATE VIEW customer_names AS SELECT id, name FROM customers;
+
+-- What the DDL view is tested on: a table with indexes and triggers, and a
+-- view. The triggers are made in the order their names are not: the text
+-- lists them by name.
+CREATE INDEX book_covers_book_id ON book_covers (book_id);
+CREATE UNIQUE INDEX book_covers_book_id_kind ON book_covers (book_id, kind);
+CREATE TRIGGER book_covers_lower_kind AFTER INSERT ON book_covers
+BEGIN
+    UPDATE book_covers SET kind = lower(NEW.kind) WHERE id = NEW.id;
+END;
+CREATE TRIGGER book_covers_keep_kind AFTER UPDATE OF kind ON book_covers
+BEGIN
+    UPDATE book_covers SET kind = lower(NEW.kind) WHERE id = NEW.id;
+END;
+CREATE VIEW in_stock AS
+    SELECT b.id AS book_id, b.title, p.name AS publisher
+    FROM books b JOIN publishers p ON p.id = b.publisher_id
+    WHERE b.in_print;
diff --git a/crates/tabletist-db/src/adapter.rs b/crates/tabletist-db/src/adapter.rs
index 9bd4aa06..5094ba54 100644
--- a/crates/tabletist-db/src/adapter.rs
+++ b/crates/tabletist-db/src/adapter.rs
@@ -2,8 +2,8 @@
 
 use crate::agent::{Catalog, ColumnName, Limits, Read, Sources};
 use crate::{
-    CancelHandle, ChangeSet, Lookup, LookupPage, ObjectInfo, ObjectRef, Result, RowPage, RowQuery,
-    ScriptMode, ScriptOutcome, StopFlag, Structure, WriteOutcome,
+    CancelHandle, ChangeSet, Ddl, Lookup, LookupPage, ObjectInfo, ObjectRef, Result, RowPage,
+    RowQuery, ScriptMode, ScriptOutcome, StopFlag, Structure, WriteOutcome,
 };
 
 /// An open session with one kind of database. [`crate::Connection`] holds
@@ -27,6 +27,8 @@ pub(crate) trait Adapter {
 
     async fn describe(&self, object: &ObjectRef) -> Result<Structure>;
 
+    async fn ddl(&self, object: &ObjectRef) -> Result<Ddl>;
+
     async fn fetch_rows(&self, query: &RowQuery) -> Result<RowPage>;
 
     async fn count_rows(&self, query: &RowQuery) -> Result<u64>;
diff --git a/crates/tabletist-db/src/ddl.rs b/crates/tabletist-db/src/ddl.rs
new file mode 100644
index 00000000..c91c05d3
--- /dev/null
+++ b/crates/tabletist-db/src/ddl.rs
@@ -0,0 +1,263 @@
+//! A table's or a view's definition as text, for the DDL view: what
+//! PostgreSQL's catalog builds, and what MySQL and SQLite keep.
+
+use crate::Driver;
+
+/// What a part of a definition holds.
+#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
+pub enum PartKind {
+    /// The `CREATE` of the table or the view itself.
+    Object,
+    Indexes,
+    Triggers,
+    Comments,
+}
+
+impl PartKind {
+    /// The comment over the part where the text puts several parts
+    /// together: SQL, so never translated.
+    fn heading(self) -> &'static str {
+        match self {
+            Self::Object => "",
+            Self::Indexes => "-- Indexes",
+            Self::Triggers => "-- Triggers",
+            Self::Comments => "-- Comments",
+        }
+    }
+}
+
+/// One part of a definition and its statements, each as the database gave
+/// it or as the catalog built it. A trigger is one statement, with the
+/// `ALTER TABLE` after it where it is not enabled as a new trigger is.
+#[derive(Debug, Clone, PartialEq, Eq)]
+pub struct Part {
+    pub kind: PartKind,
+    /// At least one: a part with nothing in it is left out.
+    pub statements: Vec<String>,
+}
+
+/// A table's or a view's definition, by its parts, so that what lists the
+/// parts and the text come from the same data.
+#[derive(Debug, Clone, PartialEq, Eq)]
+pub struct Ddl {
+    /// Whose text it is: each joins its parts its own way.
+    pub driver: Driver,
+    /// The server it was read from, as the footer of a SQL editor names
+    /// it: `PostgreSQL 17.2`, `MySQL 8.4.3`, `SQLite 3.46.0`.
+    pub server: String,
+    /// The comment the text starts with, without its `-- `: what the text
+    /// is of and where it comes from. PostgreSQL's only, whose text the
+    /// app builds.
+    pub heading: Option<String>,
+    /// The object's part first, then the others in the order the text
+    /// gives them.
+    pub parts: Vec<Part>,
+}
+
+/// Where a definition's text comes from.
+#[derive(Debug, Clone, Copy, PartialEq, Eq)]
+pub enum Source {
+    /// Built from the catalog: PostgreSQL has no `SHOW CREATE TABLE`.
+    Catalog,
+    /// As the database keeps it, and returns it.
+    Stored,
+}
+
+impl Ddl {
+    /// The parts, the object's first, with the empty ones left out.
+    pub(crate) fn new(
+        driver: Driver,
+        server: String,
+        heading: Option<String>,
+        parts: Vec<Part>,
+    ) -> Self {
+        let parts = parts
+            .into_iter()
+            .filter(|part| !part.statements.is_empty())
+            .collect();
+        Self {
+            driver,
+            server,
+            heading,
+            parts,
+        }
+    }
+
+    pub fn source(&self) -> Source {
+        match self.driver {
+            Driver::Postgres => Source::Catalog,
+            Driver::MySql | Driver::Sqlite => Source::Stored,
+        }
+    }
+
+    /// The part of `kind`, where it holds anything.
+    pub fn part(&self, kind: PartKind) -> Option<&Part> {
+        self.parts.iter().find(|part| part.kind == kind)
+    }
+
+    /// The whole text, each statement ending in `;`. PostgreSQL's starts
+    /// with its heading, and it and MySQL's put a blank line and a comment
+    /// over each part after the object's. SQLite's keeps its statements
+    /// apart by a blank line, and adds nothing else.
+    pub fn text(&self) -> String {
+        let mut text = String::new();
+        if let Some(heading) = &self.heading {
+            text.push_str("-- ");
+            text.push_str(heading);
+            text.push('\n');
+        }
+        let sections = match self.driver {
+            Driver::Postgres | Driver::MySql => true,
+            Driver::Sqlite => false,
+        };
+        let mut written = false;
+        for part in &self.parts {
+            for (index, statement) in part.statements.iter().enumerate() {
+                let first = index == 0;
+                if written {
+                    // A part's statements are one under the other.
+                    text.push_str(if sections && !first { "\n" } else { "\n\n" });
+                }
+                if sections && first && part.kind != PartKind::Object {
+                    text.push_str(part.kind.heading());
+                    text.push('\n');
+                }
+                text.push_str(&terminated(statement));
+                written = true;
+            }
+        }
+        text
+    }
+}
+
+/// `statement` ending in `;`, as the text joins it: the one it has, or one
+/// added after it. Where the last line holds a `--` comment, which would
+/// take in a `;` after it (or holds one already), it goes on a line of its
+/// own.
+fn terminated(statement: &str) -> String {
+    let statement = statement.trim_end();
+    let last = statement.lines().last().unwrap_or_default();
+    if last.contains("--") {
+        format!("{statement}\n;")
+    } else if statement.ends_with(';') {
+        statement.to_owned()
+    } else {
+        format!("{statement};")
+    }
+}
+
+#[cfg(test)]
+mod tests {
+    use super::*;
+
+    fn part(kind: PartKind, statements: &[&str]) -> Part {
+        Part {
+            kind,
+            statements: statements.iter().map(|s| s.to_string()).collect(),
+        }
+    }
+
+    #[test]
+    fn postgres_text_starts_with_its_heading_and_names_each_part() {
+        let ddl = Ddl::new(
+            Driver::Postgres,
+            "PostgreSQL 17.2".into(),
+            Some("public.t · built from the catalog · PostgreSQL 17.2".into()),
+            vec![
+                part(
+                    PartKind::Object,
+                    &["CREATE TABLE public.t (\n    id integer\n);"],
+                ),
+                part(
+                    PartKind::Indexes,
+                    &[
+                        "CREATE INDEX a ON public.t (id)",
+                        "CREATE INDEX b ON public.t (id)",
+                    ],
+                ),
+                part(PartKind::Triggers, &[]),
+                part(PartKind::Comments, &["COMMENT ON TABLE public.t IS 'x';"]),
+            ],
+        );
+        assert_eq!(
+            ddl.text(),
+            "-- public.t · built from the catalog · PostgreSQL 17.2\n\
+             CREATE TABLE public.t (\n    id integer\n);\n\
+             \n\
+             -- Indexes\n\
+             CREATE INDEX a ON public.t (id);\n\
+             CREATE INDEX b ON public.t (id);\n\
+             \n\
+             -- Comments\n\
+             COMMENT ON TABLE public.t IS 'x';"
+        );
+        // The empty part is no part.
+        assert_eq!(ddl.part(PartKind::Triggers), None);
+        assert_eq!(ddl.source(), Source::Catalog);
+    }
+
+    #[test]
+    fn mysql_text_puts_its_triggers_under_a_comment() {
+        let ddl = Ddl::new(
+            Driver::MySql,
+            "MySQL 8.4.3".into(),
+            None,
+            vec![
+                part(
+                    PartKind::Object,
+                    &["CREATE TABLE `t` (\n  `id` int\n) ENGINE=InnoDB"],
+                ),
+                part(
+                    PartKind::Triggers,
+                    &["CREATE TRIGGER `a` BEFORE INSERT ON `t` FOR EACH ROW SET NEW.id = 1"],
+                ),
+            ],
+        );
+        assert_eq!(
+            ddl.text(),
+            "CREATE TABLE `t` (\n  `id` int\n) ENGINE=InnoDB;\n\
+             \n\
+             -- Triggers\n\
+             CREATE TRIGGER `a` BEFORE INSERT ON `t` FOR EACH ROW SET NEW.id = 1;"
+        );
+        assert_eq!(ddl.source(), Source::Stored);
+    }
+
+    #[test]
+    fn sqlite_text_keeps_its_statements_apart_by_a_blank_line() {
+        let ddl = Ddl::new(
+            Driver::Sqlite,
+            "SQLite 3.46.0".into(),
+            None,
+            vec![
+                part(PartKind::Object, &["CREATE TABLE t (id integer)"]),
+                part(PartKind::Indexes, &["CREATE INDEX a ON t (id)"]),
+                part(
+                    PartKind::Triggers,
+                    &["CREATE TRIGGER x AFTER UPDATE ON t BEGIN SELECT 1; END"],
+                ),
+            ],
+        );
+        assert_eq!(
+            ddl.text(),
+            "CREATE TABLE t (id integer);\n\
+             \n\
+             CREATE INDEX a ON t (id);\n\
+             \n\
+             CREATE TRIGGER x AFTER UPDATE ON t BEGIN SELECT 1; END;"
+        );
+    }
+
+    /// A statement whose last line ends in a `--` comment gets its `;` on
+    /// a line of its own: after the comment, it would be part of it.
+    #[test]
+    fn a_semicolon_is_never_put_in_a_comment() {
+        assert_eq!(
+            terminated("SET NEW.id = NEW.id -- keep it"),
+            "SET NEW.id = NEW.id -- keep it\n;"
+        );
+        assert_eq!(terminated("SELECT 1 -- one;"), "SELECT 1 -- one;\n;");
+        assert_eq!(terminated("SELECT 1;"), "SELECT 1;");
+        assert_eq!(terminated("SELECT '--'"), "SELECT '--'\n;");
+    }
+}
diff --git a/crates/tabletist-db/src/lib.rs b/crates/tabletist-db/src/lib.rs
index db3eed77..a898d275 100644
--- a/crates/tabletist-db/src/lib.rs
+++ b/crates/tabletist-db/src/lib.rs
@@ -15,6 +15,7 @@ mod catalog;
 mod check;
 mod class;
 pub mod complete;
+pub mod ddl;
 mod dialect;
 mod error;
 pub mod fixtures;
@@ -42,6 +43,7 @@ pub use catalog::{
     ReferencingKey, Structure,
 };
 pub use class::{ColumnClass, Temporal, column_class, temporal};
+pub use ddl::Ddl;
 pub use dialect::{
     DeleteParts, Dialect, InsertParts, InsertStatement, RowDeletion, RowUpdate, Sql, UpdateParts,
     escape_like, quote_literal,
@@ -348,6 +350,20 @@ impl Connection {
         }
     }
 
+    /// The definition of the table or view `object` as text, by its parts
+    /// (see [`Ddl`]): built from PostgreSQL's catalog, and as MySQL's
+    /// `SHOW CREATE` and SQLite's `sqlite_master` return it. It only
+    /// reads the catalog: in a read-only transaction on PostgreSQL and
+    /// MySQL, under `query_only` on SQLite. All of it or an error, never a
+    /// part.
+    pub async fn ddl(&self, object: &ObjectRef) -> Result<Ddl> {
+        match &self.inner {
+            Inner::Sqlite(conn) => conn.ddl(object).await,
+            Inner::Postgres(conn) => conn.ddl(object).await,
+            Inner::MySql(conn) => conn.ddl(object).await,
+        }
+    }
+
     /// One page of up to `query.limit` rows (plus one to learn whether there
     /// are more). Each cell comes back whole, however large, and PostgreSQL's
     /// simple-query protocol buffers the whole page before the first row is
diff --git a/crates/tabletist-db/src/mysql.rs b/crates/tabletist-db/src/mysql.rs
index f5d0019a..ef3c1631 100644
--- a/crates/tabletist-db/src/mysql.rs
+++ b/crates/tabletist-db/src/mysql.rs
@@ -24,9 +24,10 @@ use crate::agent::{Catalog, ColumnName, Limits, Read, Sources};
 use crate::script::retry_cancelled;
 use crate::{
     Access, CancelHandle, CancelInner, ChangeSet, ColumnClass, ColumnInfo, ColumnMeta, ConnectSpec,
-    Dialect, Error, ForeignKeyInfo, IndexInfo, Lookup, LookupPage, MAX_LISTED, Named, ObjectInfo,
-    ObjectKind, ObjectRef, ReferencingKey, Result, RowPage, RowQuery, ScriptMode, ScriptOutcome,
-    Secrets, StopFlag, Structure, TlsMode, Value, ValueKind, WriteOutcome, column_class,
+    Ddl, Dialect, Error, ForeignKeyInfo, IndexInfo, Lookup, LookupPage, MAX_LISTED, Named,
+    ObjectInfo, ObjectKind, ObjectRef, ReferencingKey, Result, RowPage, RowQuery, ScriptMode,
+    ScriptOutcome, Secrets, StopFlag, Structure, TlsMode, Value, ValueKind, WriteOutcome,
+    column_class,
 };
 
 /// A column as `describe` reads it from `information_schema.columns`: its
@@ -466,6 +467,21 @@ impl Adapter for Conn {
         })
     }
 
+    /// `SHOW CREATE TABLE`, or `SHOW CREATE VIEW` for a view, as the
+    /// server returns it, then `SHOW CREATE TRIGGER` for each of the
+    /// table's triggers by name, in one read-only transaction.
+    async fn ddl(&self, object: &ObjectRef) -> Result<Ddl> {
+        let mut conn = self.conn.lock().await;
+        let mut transaction = conn
+            .start_transaction(read_only())
+            .await
+            .map_err(query_error)?;
+        let outcome = show_create(&mut transaction, object).await;
+        let parts = finish(transaction, outcome).await?;
+        let server = version_name(&self.version);
+        Ok(Ddl::new(crate::Driver::MySql, server, None, parts))
+    }
+
     /// One page, through a prepared statement inside a read-only
     /// transaction. Reading stops after `limit + 1` rows.
     async fn fetch_rows(&self, query: &RowQuery) -> Result<RowPage> {
@@ -693,6 +709,83 @@ fn parameter(spelled: &str, hint: Option<&str>) -> Error {
     }
 }
 
+/// The parts of `object`'s definition, read by [`Adapter::ddl`] in
+/// `transaction`.
+async fn show_create(
+    transaction: &mut mysql_async::Transaction<'_>,
+    object: &ObjectRef,
+) -> Result<Vec<crate::ddl::Part>> {
+    use crate::ddl::{Part, PartKind};
+    let kind: Option<String> = transaction
+        .exec_first(
+            "SELECT table_type FROM information_schema.tables \
+             WHERE table_schema = ? AND table_name = ?",
+            (&object.schema, &object.name),
+        )
+        .await
+        .map_err(query_error)?;
+    let Some(kind) = kind else {
+        return Err(Error::query(format!(
+            "no such table or view: {}",
+            object.name
+        )));
+    };
+    let name = format!(
+        "{}.{}",
+        Dialect::MySql.quote_ident(&object.schema),
+        Dialect::MySql.quote_ident(&object.name)
+    );
+    let shown = if kind.contains("VIEW") {
+        format!("SHOW CREATE VIEW {name}")
+    } else {
+        format!("SHOW CREATE TABLE {name}")
+    };
+    // Its second column, whichever it is: `Create Table` or `Create View`.
+    let own = nth_text(transaction, &shown, 1).await?;
+    let names: Vec<String> = transaction
+        .exec(
+            "SELECT trigger_name FROM information_schema.triggers \
+             WHERE event_object_schema = ? AND event_object_table = ? \
+             ORDER BY trigger_name",
+            (&object.schema, &object.name),
+        )
+        .await
+        .map_err(query_error)?;
+    let mut triggers = Vec::with_capacity(names.len());
+    for trigger in names {
+        let shown = format!(
+            "SHOW CREATE TRIGGER {}.{}",
+            Dialect::MySql.quote_ident(&object.schema),
+            Dialect::MySql.quote_ident(&trigger)
+        );
+        // `SQL Original Statement`, after the name and the `sql_mode`.
+        triggers.push(nth_text(transaction, &shown, 2).await?);
+    }
+    Ok(vec![
+        Part {
+            kind: PartKind::Object,
+            statements: vec![own],
+        },
+        Part {
+            kind: PartKind::Triggers,
+            statements: triggers,
+        },
+    ])
+}
+
+/// Column `index` of the one row `sql` answers, as text.
+async fn nth_text(
+    transaction: &mut mysql_async::Transaction<'_>,
+    sql: &str,
+    index: usize,
+) -> Result<String> {
+    let row: Option<mysql_async::Row> = transaction.query_first(sql).await.map_err(query_error)?;
+    row.and_then(|row| row.get_opt::<String, _>(index))
+        .transpose()
+        .map_err(|_| Error::query("unexpected data from the server: the text is not text"))?
+        .ok_or_else(|| Error::query(format!("{sql} returned nothing")))
+}
+
 /// Ends a read transaction whatever happened inside it. mysql_async only
 /// rolls a dropped transaction back on the connection's next use, so a
 /// failed or cancelled read would otherwise keep its locks until then.
diff --git a/crates/tabletist-db/src/pg.rs b/crates/tabletist-db/src/pg.rs
index c03c07c4..d1559727 100644
--- a/crates/tabletist-db/src/pg.rs
+++ b/crates/tabletist-db/src/pg.rs
@@ -14,14 +14,15 @@ use tokio_postgres::{SimpleQueryMessage, Socket};
 use crate::adapter::Adapter;
 use crate::agent::{Catalog, ColumnName, Limits, Read, Sources};
 use crate::{
-    Access, CancelHandle, CancelInner, ChangeSet, ColumnInfo, ColumnMeta, ConnectSpec, Dialect,
-    Error, ForeignKeyInfo, IndexInfo, Lookup, LookupPage, MAX_LISTED, Named, ObjectInfo,
+    Access, CancelHandle, CancelInner, ChangeSet, ColumnInfo, ColumnMeta, ConnectSpec, Ddl,
+    Dialect, Error, ForeignKeyInfo, IndexInfo, Lookup, LookupPage, MAX_LISTED, Named, ObjectInfo,
     ObjectKind, ObjectRef, ReferencingKey, Result, RowPage, RowQuery, ScriptMode, ScriptOutcome,
     Secrets, Sql, StopFlag, Structure, Value, ValueKind, WriteOutcome, value_from_pg_text,
 };
 use tokio_postgres::error::SqlState;
 use tokio_postgres_rustls::MakeRustlsConnect;
 
+mod ddl;
 mod hidden;
 mod script;
 mod write;
@@ -569,6 +570,10 @@ impl Adapter for Conn {
             .collect()
     }
 
+    async fn ddl(&self, object: &ObjectRef) -> Result<Ddl> {
+        self.read_ddl(object).await
+    }
+
     async fn describe(&self, object: &ObjectRef) -> Result<Structure> {
         let oid = self.relation(object).await?;
         let columns = self
diff --git a/crates/tabletist-db/src/pg/ddl.rs b/crates/tabletist-db/src/pg/ddl.rs
new file mode 100644
index 00000000..8ac3c0f6
--- /dev/null
+++ b/crates/tabletist-db/src/pg/ddl.rs
@@ -0,0 +1,444 @@
+//! A table's or a view's definition, built from PostgreSQL's catalog, which
+//! has no `SHOW CREATE TABLE`: in the order pg_dump writes it, the
+//! `CREATE` first, then the indexes that back no constraint, the triggers,
+//! and the comments. The owner, the grants, the row-level policies and the
+//! sequences the table owns are left out.
+//!
+//! It is read with the search path empty, so the server writes every name
+//! with its schema, and the text runs whatever the search path it is run
+//! with.
+
+use super::{column, query_error};
+use crate::ddl::{Part, PartKind};
+use crate::{Ddl, Driver, Error, ObjectRef, Result};
+
+/// The relation itself, by schema and name.
+const RELATION: &str = "\
+SELECT c.oid, c.relkind::text, c.relpersistence::text, c.relispartition, \
+       quote_ident(n.nspname) || '.' || quote_ident(c.relname), \
+       array_to_string(c.reloptions, ', '), \
+       CASE WHEN c.relkind = 'p' THEN pg_get_partkeydef(c.oid) END, \
+       CASE WHEN c.relispartition THEN pg_get_expr(c.relpartbound, c.oid) END, \
+       ARRAY(SELECT i.inhparent::regclass::text FROM pg_inherits i \
+             WHERE i.inhrelid = c.oid ORDER BY i.inhseqno), \
+       quote_literal(obj_description(c.oid, 'pg_class')), \
+       CASE WHEN c.relkind IN ('v', 'm') THEN pg_get_viewdef(c.oid, true) END, \
+       current_setting('server_version') \
+FROM pg_class c JOIN pg_namespace n ON n.oid = c.relnamespace \
+WHERE n.nspname = $1 AND c.relname = $2";
+
+/// Its columns. A collation is the column's own where it is not its
+/// type's. Whether the column is an identity or generated is read through
+/// `to_jsonb`, as `describe` reads it: `attgenerated` is PostgreSQL 12's.
+/// For a column a parent gives the table, whether the parent's is NOT
+/// NULL and its default (the first parent's that has the column): what
+/// the table set itself is what differs.
+const COLUMNS: &str = "\
+SELECT quote_ident(a.attname), format_type(a.atttypid, a.atttypmod), \
+       CASE WHEN a.attcollation <> 0 AND a.attcollation <> t.typcollation \
+            THEN quote_ident(cn.nspname) || '.' || quote_ident(co.collname) END, \
+       pg_get_expr(d.adbin, d.adrelid), \
+       COALESCE(to_jsonb(a) ->> 'attidentity', ''), \
+       COALESCE(to_jsonb(a) ->> 'attgenerated', ''), \
+       a.attnotnull, a.attislocal, \
+       quote_literal(col_description(a.attrelid, a.attnum)), \
+       EXISTS (SELECT 1 FROM pg_inherits i \
+               JOIN pg_attribute pa ON pa.attrelid = i.inhparent AND pa.attname = a.attname \
+               WHERE i.inhrelid = a.attrelid AND pa.attnotnull), \
+       (SELECT pg_get_expr(pd.adbin, pd.adrelid) FROM pg_inherits i \
+        JOIN pg_attribute pa ON pa.attrelid = i.inhparent AND pa.attname = a.attname \
+        JOIN pg_attrdef pd ON pd.adrelid = pa.attrelid AND pd.adnum = pa.attnum \
+        WHERE i.inhrelid = a.attrelid ORDER BY i.inhseqno LIMIT 1) \
+FROM pg_attribute a \
+JOIN pg_type t ON t.oid = a.atttypid \
+LEFT JOIN pg_attrdef d ON d.adrelid = a.attrelid AND d.adnum = a.attnum \
+LEFT JOIN pg_collation co ON co.oid = a.attcollation \
+LEFT JOIN pg_namespace cn ON cn.oid = co.collnamespace \
+WHERE a.attrelid = $1 AND a.attnum > 0 AND NOT a.attisdropped \
+ORDER BY a.attnum";
+
+/// The table's own constraints, not those a partition or a child has from
+/// its parent: primary key, unique, check, foreign key, exclusion, each
+/// kind by name, and whether each holds for the rows already there. A NOT
+/// NULL (a constraint of its own from PostgreSQL 18) is the column's.
+const CONSTRAINTS: &str = "\
+SELECT quote_ident(conname), pg_get_constraintdef(oid), convalidated \
+FROM pg_constraint \
+WHERE conrelid = $1 AND contype IN ('p', 'u', 'c', 'f', 'x') AND conislocal \
+ORDER BY array_position(ARRAY['p', 'u', 'c', 'f', 'x'], contype::text), conname";
+
+/// The indexes that back no constraint, by name. A partition's index that
+/// is its parent's is the parent's to make. Whether the index is a
+/// partitioned table's comes with it.
+const INDEXES: &str = "\
+SELECT pg_get_indexdef(i.indexrelid), ic.relkind = 'I' \
+FROM pg_index i JOIN pg_class ic ON ic.oid = i.indexrelid \
+WHERE i.indrelid = $1 \
+  AND NOT EXISTS (SELECT 1 FROM pg_constraint k \
+                  WHERE k.conrelid = i.indrelid AND k.conindid = i.indexrelid \
+                    AND k.contype IN ('p', 'u', 'x')) \
+  AND NOT EXISTS (SELECT 1 FROM pg_inherits h WHERE h.inhrelid = i.indexrelid) \
+ORDER BY ic.relname";
+
+/// The triggers a person made, by name: not the internal ones of a foreign
+/// key, nor a partition's that is its parent's (`tgparentid`, read
+/// through `to_jsonb`: PostgreSQL 13's).
+const TRIGGERS: &str = "\
+SELECT pg_get_triggerdef(t.oid), quote_ident(t.tgname), t.tgenabled::text \
+FROM pg_trigger t \
+WHERE t.tgrelid = $1 AND NOT t.tgisinternal \
+  AND COALESCE((to_jsonb(t) ->> 'tgparentid')::bigint, 0) = 0 \
+ORDER BY t.tgname";
+
+/// A column as the `CREATE TABLE` and the comments need it.
+struct Column {
+    name: String,
+    type_name: String,
+    collation: Option<String>,
+    expression: Option<String>,
+    identity: String,
+    generated: String,
+    not_null: bool,
+    local: bool,
+    comment: Option<String>,
+    /// For a column a parent gives the table: whether the parent's is NOT
+    /// NULL, and its default.
+    parent_not_null: bool,
+    parent_default: Option<String>,
+}
+
+/// What a relation is, for its `CREATE` and its `COMMENT ON`.
+#[derive(Clone, Copy, PartialEq, Eq)]
+enum Kind {
+    Table,
+    View,
+    MaterializedView,
+}
+
+impl Kind {
+    fn noun(self) -> &'static str {
+        match self {
+            Self::Table => "TABLE",
+            Self::View => "VIEW",
+            Self::MaterializedView => "MATERIALIZED VIEW",
+        }
+    }
+}
+
+impl super::Conn {
+    /// See [`crate::Connection::ddl`]: in a read-only transaction, with
+    /// the search path empty for the transaction alone.
+    pub(super) async fn read_ddl(&self, object: &ObjectRef) -> Result<Ddl> {
+        let mut client = self.client.lock().await;
+        let transaction = client
+            .build_transaction()
+            .read_only(true)
+            .start()
+            .await
+            .map_err(query_error)?;
+        transaction
+            .batch_execute("SET LOCAL search_path = ''")
+            .await
+            .map_err(query_error)?;
+        let built = build(&transaction, object).await;
+        // The read's own error wins over one from the rollback.
+        let rolled_back = transaction.rollback().await;
+        let ddl = built?;
+        rolled_back.map_err(query_error)?;
+        Ok(ddl)
+    }
+}
+
+async fn build(transaction: &tokio_postgres::Transaction<'_>, object: &ObjectRef) -> Result<Ddl> {
+    let rows = transaction
+        .query(RELATION, &[&object.schema, &object.name])
+        .await
+        .map_err(query_error)?;
+    let Some(relation) = rows.first() else {
+        return Err(Error::query(format!(
+            "no such table or view: {}",
+            object.name
+        )));
+    };
+    let oid: u32 = column(relation, 0)?;
+    let relkind: String = column(relation, 1)?;
+    let kind = match relkind.as_str() {
+        "r" | "p" => Kind::Table,
+        "v" => Kind::View,
+        "m" => Kind::MaterializedView,
+        "f" => return Err(Error::Unsupported("the DDL of a foreign table")),
+        _ => {
+            return Err(Error::query(format!(
+                "not a table or a view: {}",
+                object.name
+            )));
+        }
+    };
+    let unlogged = column::<String>(relation, 2)? == "u";
+    let partition: bool = column(relation, 3)?;
+    let name: String = column(relation, 4)?;
+    let options: Option<String> = column(relation, 5)?;
+    let options = options.filter(|options| !options.is_empty());
+    let partition_key: Option<String> = column(relation, 6)?;
+    let bound: Option<String> = column(relation, 7)?;
+    let parents: Vec<String> = column(relation, 8)?;
+    let comment: Option<String> = column(relation, 9)?;
+    let view: Option<String> = column(relation, 10)?;
+    let version: String = column(relation, 11)?;
+    // "17.2 (Debian 17.2-1.pgdg120+1)" reads as "17.2".
+    let version = version.split_whitespace().next().unwrap_or_default();
+    let columns = transaction
+        .query(COLUMNS, &[&oid])
+        .await
+        .map_err(query_error)?
+        .iter()
+        .map(|row| {
+            Ok(Column {
+                name: column(row, 0)?,
+                type_name: column(row, 1)?,
+                collation: column(row, 2)?,
+                expression: column(row, 3)?,
+                identity: column(row, 4)?,
+                generated: column(row, 5)?,
+                not_null: column(row, 6)?,
+                local: column(row, 7)?,
+                comment: column(row, 8)?,
+                parent_not_null: column(row, 9)?,
+                parent_default: column(row, 10)?,
+            })
+        })
+        .collect::<Result<Vec<_>>>()?;
+    let with = options
+        .as_deref()
+        .map(|options| format!(" WITH ({options})"))
+        .unwrap_or_default();
+    let own = match kind {
+        Kind::View => {
+            let view = view.unwrap_or_default();
+            format!("CREATE OR REPLACE VIEW {name}{with} AS\n{view}")
+        }
+        Kind::MaterializedView => {
+            let view = view.unwrap_or_default();
+            let view = view.trim_end().trim_end_matches(';');
+            format!("CREATE MATERIALIZED VIEW {name}{with} AS\n{view}\nWITH NO DATA;")
+        }
+        Kind::Table => {
+            let mut constraints = Vec::new();
+            let mut not_valid = Vec::new();
+            for row in transaction
+                .query(CONSTRAINTS, &[&oid])
+                .await
+                .map_err(query_error)?
+            {
+                let (constraint, definition): (String, String) =
+                    (column(&row, 0)?, column(&row, 1)?);
+                let valid: bool = column(&row, 2)?;
+                // One that does not hold for the rows already there is
+                // added after the table: in `CREATE TABLE` it would be
+                // checked, and made valid.
+                match valid {
+                    true => constraints.push(format!("    CONSTRAINT {constraint} {definition}")),
+                    false => not_valid.push(format!(
+                        "ALTER TABLE ONLY {name} ADD CONSTRAINT {constraint} {definition};"
+                    )),
+                }
+            }
+            let table = Table {
+                name: &name,
+                unlogged,
+                columns: &columns,
+                constraints,
+                // A partition's parent is its one parent, and its bound
+                // says it is one.
+                parent: bound.as_ref().filter(|_| partition).and(parents.first()),
+                bound: bound.as_deref().filter(|_| partition),
+                parents: if partition { &[] } else { &parents },
+                partition_key: partition_key.as_deref(),
+                options: options.as_deref(),
+                not_valid,
+            };
+            table.create()
+        }
+    };
+    let indexes = transaction
+        .query(INDEXES, &[&oid])
+        .await
+        .map_err(query_error)?
+        .iter()
+        .map(|row| {
+            let definition: String = column(row, 0)?;
+            let partitioned: bool = column(row, 1)?;
+            // A partitioned table's index is written `ON ONLY`, which
+            // makes it on the table alone, for pg_dump to attach each
+            // partition's to it. Without the partitions' (left out
+            // above), the index is made on the table and its partitions.
+            // The table's name follows: an index's name can hold the words.
+            Ok(match partitioned {
+                true => definition.replacen(
+                    &format!(" ON ONLY {name} USING "),
+                    &format!(" ON {name} USING "),
+                    1,
+                ),
+                false => definition,
+            })
+        })
+        .collect::<Result<Vec<_>>>()?;
+    let triggers = transaction
+        .query(TRIGGERS, &[&oid])
+        .await
+        .map_err(query_error)?
+        .iter()
+        .map(|row| {
+            let definition: String = column(row, 0)?;
+            let trigger: String = column(row, 1)?;
+            let enabled: String = column(row, 2)?;
+            // As a new trigger is: fires unless the session replicates.
+            let set = match enabled.as_str() {
+                "D" => Some("DISABLE"),
+                "R" => Some("ENABLE REPLICA"),
+                "A" => Some("ENABLE ALWAYS"),
+                _ => None,
+            };
+            Ok(match set {
+                Some(set) => format!("{definition};\nALTER TABLE {name} {set} TRIGGER {trigger};"),
+                None => definition,
+            })
+        })
+        .collect::<Result<Vec<_>>>()?;
+    let noun = kind.noun();
+    let comments = comment
+        .map(|comment| format!("COMMENT ON {noun} {name} IS {comment};"))
+        .into_iter()
+        .chain(columns.iter().filter_map(|column| {
+            let comment = column.comment.as_ref()?;
+            Some(format!(
+                "COMMENT ON COLUMN {name}.{} IS {comment};",
+                column.name
+            ))
+        }))
+        .collect();
+    let parts = vec![
+        Part {
+            kind: PartKind::Object,
+            statements: vec![own],
+        },
+        Part {
+            kind: PartKind::Indexes,
+            statements: indexes,
+        },
+        Part {
+            kind: PartKind::Triggers,
+            statements: triggers,
+        },
+        Part {
+            kind: PartKind::Comments,
+            statements: comments,
+        },
+    ];
+    let server = format!("PostgreSQL {version}");
+    let heading = format!("{name} · built from the catalog · {server}");
+    Ok(Ddl::new(Driver::Postgres, server, Some(heading), parts))
+}
+
+/// What a table's `CREATE TABLE` says.
+struct Table<'a> {
+    name: &'a str,
+    unlogged: bool,
+    columns: &'a [Column],
+    /// `CONSTRAINT name ...`, each a line of the list.
+    constraints: Vec<String>,
+    /// The table it is a partition of, with the bound it holds.
+    parent: Option<&'a String>,
+    bound: Option<&'a str>,
+    /// The tables it inherits from, where it is no partition.
+    parents: &'a [String],
+    partition_key: Option<&'a str>,
+    options: Option<&'a str>,
+    /// `ALTER TABLE … ADD CONSTRAINT … NOT VALID;`, each.
+    not_valid: Vec<String>,
+}
+
+impl Table<'_> {
+    fn create(&self) -> String {
+        let unlogged = if self.unlogged { "UNLOGGED " } else { "" };
+        let mut text = format!("CREATE {unlogged}TABLE {}", self.name);
+        if let Some(parent) = self.parent {
+            text.push_str(&format!(" PARTITION OF {parent}"));
+        }
+        // The columns a parent gives the table are the parent's to make.
+        let mut lines: Vec<String> = self
+            .columns
+            .iter()
+            .filter(|column| column.local)
+            .map(column_line)
+            .collect();
+        lines.extend(self.constraints.iter().cloned());
+        match (self.parent.is_some(), lines.is_empty()) {
+            // A partition with nothing of its own lists nothing.
+            (true, true) => {}
+            (false, true) => text.push_str(" (\n)"),
+            (_, false) => text.push_str(&format!(" (\n{}\n)", lines.join(",\n"))),
+        }
+        if let Some(bound) = self.bound {
+            text.push_str(&format!("\n{bound}"));
+        }
+        if !self.parents.is_empty() {
+            text.push_str(&format!("\nINHERITS ({})", self.parents.join(", ")));
+        }
+        if let Some(key) = self.partition_key {
+            text.push_str(&format!("\nPARTITION BY {key}"));
+        }
+        if let Some(options) = self.options {
+            text.push_str(&format!("\nWITH ({options})"));
+        }
+        text.push(';');
+        // What the table set itself on a column its parent gives it.
+        for column in self.columns.iter().filter(|column| !column.local) {
+            let generates = !column.identity.is_empty() || !column.generated.is_empty();
+            if generates {
+                continue;
+            }
+            let alter = format!(
+                "\nALTER TABLE ONLY {} ALTER COLUMN {}",
+                self.name, column.name
+            );
+            match (&column.expression, &column.parent_default) {
+                (Some(own), parents) if Some(own) != parents.as_ref() => {
+                    text.push_str(&format!("{alter} SET DEFAULT {own};"));
+                }
+                (None, Some(_)) => text.push_str(&format!("{alter} DROP DEFAULT;")),
+                _ => {}
+            }
+            if column.not_null && !column.parent_not_null {
+                text.push_str(&format!("{alter} SET NOT NULL;"));
+            }
+        }
+        for constraint in &self.not_valid {
+            text.push('\n');
+            text.push_str(constraint);
+        }
+        text
+    }
+}
+
+/// A column's line of the `CREATE TABLE`: its name and type, its own
+/// collation, then its default, or what generates it, then `NOT NULL`.
+fn column_line(column: &Column) -> String {
+    let mut line = format!("    {} {}", column.name, column.type_name);
+    if let Some(collation) = &column.collation {
+        line.push_str(&format!(" COLLATE {collation}"));
+    }
+    let expression = column.expression.as_deref().unwrap_or_default();
+    match (column.identity.as_str(), column.generated.as_str()) {
+        ("a", _) => line.push_str(" GENERATED ALWAYS AS IDENTITY"),
+        ("d", _) => line.push_str(" GENERATED BY DEFAULT AS IDENTITY"),
+        (_, "s") => line.push_str(&format!(" GENERATED ALWAYS AS ({expression}) STORED")),
+        (_, "v") => line.push_str(&format!(" GENERATED ALWAYS AS ({expression}) VIRTUAL")),
+        _ if column.expression.is_some() => line.push_str(&format!(" DEFAULT {expression}")),
+        _ => {}
+    }
+    if column.not_null {
+        line.push_str(" NOT NULL");
+    }
+    line
+}
diff --git a/crates/tabletist-db/src/sqlite.rs b/crates/tabletist-db/src/sqlite.rs
index b8588cf7..6d71f89f 100644
--- a/crates/tabletist-db/src/sqlite.rs
+++ b/crates/tabletist-db/src/sqlite.rs
@@ -19,8 +19,8 @@ use crate::agent::{
 };
 use crate::script::{cancelled_commit, cleanup_failed};
 use crate::{
-    Access, CancelHandle, CancelInner, ChangeSet, ColumnClass, ColumnInfo, ColumnMeta, Dialect,
-    Error, ForeignKeyInfo, IndexInfo, Lookup, LookupPage, MAX_LISTED, Named, ObjectInfo,
+    Access, CancelHandle, CancelInner, ChangeSet, ColumnClass, ColumnInfo, ColumnMeta, Ddl,
+    Dialect, Error, ForeignKeyInfo, IndexInfo, Lookup, LookupPage, MAX_LISTED, Named, ObjectInfo,
     ObjectKind, ObjectRef, ReferencingKey, Result, RowPage, RowQuery, ScriptEnd, ScriptMode,
     ScriptOutcome, StatementOutcome, StatementResult, StopFlag, Structure, Value, ValueKind,
     WriteOutcome, column_class,
@@ -1026,6 +1026,75 @@ impl Adapter for Conn {
         .await
     }
 
+    /// The text `sqlite_master` keeps of the table or view, then of its
+    /// indexes and its triggers, each by name. A trigger keeps the table's
+    /// name as it was written in it, in any case. An index SQLite made itself
+    /// (for a key or a `UNIQUE`) keeps none, and is left out.
+    async fn ddl(&self, object: &ObjectRef) -> Result<Ddl> {
+        use crate::ddl::{Part, PartKind};
+        let master = format!(
+            "{}.sqlite_master",
+            Dialect::Sqlite.quote_ident(&object.schema)
+        );
+        let object = object.clone();
+        self.run(move |connection| {
+            let own: Option<Option<String>> = connection
+                .query_row(
+                    &format!(
+                        "SELECT sql FROM {master} WHERE name = ?1 AND type IN ('table', 'view')"
+                    ),
+                    [&object.name],
+                    |row| optional_text(row, 0),
+                )
+                .optional()
+                .map_err(map_error)?;
+            let Some(Some(own)) = own else {
+                return Err(Error::query(format!(
+                    "no such table or view: {}",
+                    object.name
+                )));
+            };
+            let of_kind = |kind: &str| -> Result<Vec<String>> {
+                let mut statement = connection
+                    .prepare(&format!(
+                        "SELECT sql FROM {master} \
+                         WHERE tbl_name = ?1 COLLATE NOCASE AND type = ?2 AND sql IS NOT NULL \
+                         ORDER BY name"
+                    ))
+                    .map_err(map_error)?;
+                statement
+                    .query_map([object.name.as_str(), kind], |row| text(row, 0))
+                    .map_err(map_error)?
+                    .collect::<std::result::Result<Vec<_>, _>>()
+                    .map_err(map_error)
+            };
+            let parts = vec![
+                Part {
+                    kind: PartKind::Object,
+                    statements: vec![own],
+                },
+                Part {
+                    kind: PartKind::Indexes,
+                    statements: of_kind("index")?,
+                },
+                Part {
+                    kind: PartKind::Triggers,
+                    statements: of_kind("trigger")?,
+                },
+            ];
+            let version: String = connection
+                .query_row("SELECT sqlite_version()", [], |row| row.get(0))
+                .map_err(map_error)?;
+            Ok(Ddl::new(
+                crate::Driver::Sqlite,
+                format!("SQLite {version}"),
+                None,
+                parts,
+            ))
+        })
+        .await
+    }
+
     async fn fetch_rows(&self, query: &RowQuery) -> Result<RowPage> {
         check_raw_where(query)?;
         let query = query.clone();
````

- [ ] **Step 4: Run the tests to see them pass**

```bash
TABLETIST_TEST_PG_URL=postgres://tabletist:tabletist@localhost:55432/tabletist TABLETIST_TEST_MYSQL_URL=mysql://tabletist:tabletist@localhost:53306/tabletist ~/.cargo/bin/cargo test --locked -p tabletist-db -p tabletist-mcp
```

Expected: all pass, `tests/ddl.rs` with 17.

- [ ] **Step 5: Run the checks** (above). Expected: every suite passes, the app's library with 3303 tests and `tabletist-db`'s `tests/ddl.rs` with 17.

- [ ] **Step 6: Commit**

```bash
git add .github/workflows/ci.yml crates
git commit -m "Read a table's or a view's DDL on each engine"
```

---

### Task 2: The tab reads, holds and keeps its definition

**Files:**
- Modify: `src/app.rs`
- Modify: `src/app/labels.rs`
- Modify: `src/app/startup_tests.rs`
- Modify: `src/backend.rs`
- Modify: `src/history.rs`
- Modify: `src/model.rs`
- Modify: `src/session.rs`
- Modify: `src/testing.rs`
- Modify: `src/ui/history_tests.rs`

- [ ] **Step 1: Write the failing tests**

The harness answers a `ReadDdl` (and clicks a tab as it clicks a button); a table closed in its DDL view comes back in it and reads its definition once; `src/session.rs`'s tests keep the view by its word.

````diff
diff --git a/src/app/startup_tests.rs b/src/app/startup_tests.rs
index 963f55ae..ab386758 100644
--- a/src/app/startup_tests.rs
+++ b/src/app/startup_tests.rs
@@ -576,6 +576,39 @@ fn a_connection_opened_again_reopens_its_tables_and_reads_the_one_in_front() {
     assert!(fetched(&app, before).is_empty());
 }
 
+/// A table closed in its DDL view comes back in it, and its definition is
+/// read with its rows once the connect is up, and only once.
+#[test]
+fn a_table_closed_in_its_ddl_view_reopens_in_it_and_reads_its_definition() {
+    let dir = tempfile::tempdir().unwrap();
+    let mut app = app_in(dir.path());
+    let conn = bookshop(&mut app, "Bookshop");
+    let tab = open_window(&mut app, &conn);
+    connected(&mut app);
+    let titles = books(&mut app, tab, "books");
+    app.apply(Action::SetView {
+        tab,
+        object_tab: titles,
+        view: crate::model::ObjectView::Ddl,
+    });
+    app.apply(Action::CloseConnTab(tab));
+    let tab = open_window(&mut app, &conn);
+    app.apply_actions();
+    let front = app.workspace(tab).unwrap().active_object_tab().unwrap();
+    assert_eq!(front.view, crate::model::ObjectView::Ddl);
+    let before = app.backend.sent.len();
+    connected(&mut app);
+    app.apply_actions();
+    let read: Vec<ObjectRef> = app.backend.sent[before..]
+        .iter()
+        .filter_map(|command| match command {
+            Command::ReadDdl { object, .. } => Some(object.clone()),
+            _ => None,
+        })
+        .collect();
+    assert_eq!(read, [ObjectRef::new("main", "books")]);
+}
+
 #[test]
 fn a_reopened_table_read_by_following_a_key_gets_its_structure_once() {
     let dir = tempfile::tempdir().unwrap();
diff --git a/src/session.rs b/src/session.rs
index e3ea6c6c..587d0a8f 100644
--- a/src/session.rs
+++ b/src/session.rs
@@ -449,6 +449,7 @@ mod tests {
                 ..KeptFilter::default()
             },
             pinned: true,
+            view: "ddl".into(),
             rest: BTreeMap::new(),
         }
     }
@@ -716,6 +717,25 @@ mod tests {
         assert_eq!(KeptTab::of(&tab), books());
     }
 
+    /// The view is kept by its word, and the rows by none; a word this
+    /// version does not know reads as the rows.
+    #[test]
+    fn a_tab_reopens_in_the_view_it_was_kept_in() {
+        let tab = books().reopen(TabId(7), 1_000).unwrap();
+        assert_eq!(tab.view, ObjectView::Ddl);
+        let mut kept = books();
+        kept.view = "structure".into();
+        let tab = kept.reopen(TabId(7), 1_000).unwrap();
+        assert_eq!(tab.view, ObjectView::Structure);
+        assert_eq!(KeptTab::of(&tab).view, "structure");
+        kept.view = "outline".into();
+        let mut tab = kept.reopen(TabId(7), 1_000).unwrap();
+        assert_eq!(tab.view, ObjectView::Data);
+        tab.view = ObjectView::Data;
+        let written = serde_json::to_value(KeptTab::of(&tab)).unwrap();
+        assert!(written.get("view").is_none(), "{written}");
+    }
+
     #[test]
     fn words_this_version_does_not_know_are_left_out() {
         let mut tab = books();
diff --git a/src/testing.rs b/src/testing.rs
index b9d79673..10a56e88 100644
--- a/src/testing.rs
+++ b/src/testing.rs
@@ -247,6 +247,7 @@ impl Harness {
     pub fn click(&mut self, label: &str) {
         let tree = self.settle();
         let target = node(&tree, label, Role::Button)
+            .or_else(|| node(&tree, label, Role::Tab))
             .or_else(|| {
                 tree.nodes
                     .iter()
@@ -1343,6 +1344,28 @@ impl Harness {
         }));
     }
 
+    /// Answers the newest `ReadDdl` sent with `result`.
+    pub fn answer_ddl(&mut self, result: Result<tabletist_db::Ddl, tabletist_db::Error>) {
+        let (session, request) = self
+            .app
+            .backend
+            .sent
+            .iter()
+            .rev()
+            .find_map(|command| match command {
+                Command::ReadDdl {
+                    session, request, ..
+                } => Some((*session, *request)),
+                _ => None,
+            })
+            .expect("a ReadDdl was sent");
+        self.app.apply(Action::Backend(Event::Ddl {
+            session,
+            request,
+            result,
+        }));
+    }
+
     /// Answers the newest `Write` sent.
     pub fn answer_written(
         &mut self,
````

- [ ] **Step 2: Run them to see them fail**

```bash
~/.cargo/bin/cargo test --locked --lib -- session:: startup_tests
```

Expected: it does not compile, there is no `ReadDdl`, `Event::Ddl`, `ObjectView::Ddl` or kept tab's `view`:

```
error[E0433]: cannot find type `ObjectView` in this scope
error[E0433]: cannot find type `ObjectView` in this scope
error[E0433]: cannot find type `ObjectView` in this scope
error[E0433]: cannot find type `ObjectView` in this scope
error[E0599]: no variant, associated function, or constant named `Ddl` found for enum `model::ObjectView` in the current scope
error[E0599]: no variant, associated function, or constant named `Ddl` found for enum `model::ObjectView` in the current scope
error[E0599]: no variant named `ReadDdl` found for enum `backend::Command`
error[E0560]: struct `session::KeptTab` has no field named `view`
error[E0609]: no field `view` on type `session::KeptTab`
error[E0609]: no field `view` on type `session::KeptTab`
error[E0609]: no field `view` on type `session::KeptTab`
error[E0599]: no variant named `ReadDdl` found for enum `backend::Command`
error[E0599]: no variant named `Ddl` found for enum `backend::Event`
```

- [ ] **Step 3: Write the command, the tab's fetch, and the view kept with the tab**

````diff
diff --git a/src/app.rs b/src/app.rs
index ee76992a..2934fd58 100644
--- a/src/app.rs
+++ b/src/app.rs
@@ -1301,6 +1301,23 @@ impl App {
                 if changes {
                     self.close_editor(tab, object_tab, true);
                 }
+                // Leaving the DDL view stops its read: the view reads it
+                // again when it is shown next. A definition it already
+                // had stays.
+                let left = self.object_tab_mut(tab, object_tab).and_then(|object| {
+                    let leaves = object.view == ObjectView::Ddl && view != ObjectView::Ddl;
+                    leaves.then(|| {
+                        object.ddl.started = None;
+                        object.ddl.asked = false;
+                        object.ddl.pending.take()
+                    })?
+                });
+                if let Some(session) = self.workspace(tab).map(|workspace| workspace.session) {
+                    self.cancel(session, left);
+                }
+                let read = self
+                    .object_tab_mut(tab, object_tab)
+                    .is_some_and(|object| view == ObjectView::Ddl && object.ddl.needs_load());
                 let describe = self.object_tab_mut(tab, object_tab).is_some_and(|object| {
                     object.view = view;
                     // A visual selection is of the rows on screen: another
@@ -1317,6 +1334,9 @@ impl App {
                 if describe {
                     self.describe(tab, object_tab);
                 }
+                if read {
+                    self.read_ddl(tab, object_tab);
+                }
                 self.forget_refusal(tab);
             }
             Action::NextPage { tab, object_tab } => {
@@ -1960,6 +1980,7 @@ impl App {
                     });
                 match active {
                     Some((id, described)) => {
+                        self.reload_ddl(tab, id);
                         self.reset_count(tab, id);
                         // Over an error box a refresh is a retry: what the
                         // error replaced is not what comes back if it is
@@ -2621,6 +2642,12 @@ impl App {
                 }
                 self.describe(tab, object_tab);
             }
+            Action::RetryDdl { tab, object_tab } => {
+                if let Some(object) = self.object_tab_mut(tab, object_tab) {
+                    object.ddl.value = None;
+                }
+                self.read_ddl(tab, object_tab);
+            }
             Action::SetDriver(driver) => {
                 if let Some(Dialog::Connection(form)) = &mut self.dialog {
                     // A port left at the old driver's default follows the driver.
@@ -4119,6 +4146,27 @@ impl App {
                 // A picker that waited for these columns learns them.
                 self.picker_columns(tab);
             }
+            Event::Ddl {
+                session,
+                request,
+                result,
+            } => {
+                let Some(tab) = self.tab_for_session(session) else {
+                    return;
+                };
+                let waiting = self.workspace_mut(tab).and_then(|workspace| {
+                    workspace
+                        .object_tabs_mut()
+                        .find(|object| object.ddl.pending == Some(request))
+                });
+                if let Some(object) = waiting {
+                    let read = result.is_ok();
+                    object.ddl.finish(request, result);
+                    if read {
+                        object.ddl_read = Some(std::time::Instant::now());
+                    }
+                }
+            }
             Event::SecretLoaded { request, result } => {
                 match self.pending_secrets.remove(&request) {
                     Some(SecretPurpose::Connect {
@@ -5529,6 +5577,22 @@ impl App {
                 object.count.asked = asked;
             }
         }
+        // The definition the tab in front shows in its DDL view is read
+        // on the new session; one that was on its way, or was lost, on a
+        // tab behind is forgotten, for its view to read when shown. A
+        // reopened tab reads it with its rows.
+        let mut ddl = Vec::new();
+        for object in workspace.object_tabs_mut() {
+            let is_active = Some(object.id) == active;
+            if object.reopened {
+                continue;
+            }
+            if is_active && object.view == ObjectView::Ddl {
+                ddl.push(object.id);
+            } else if object.ddl.is_loading() || lost(&object.ddl) || object.ddl.asked {
+                object.ddl = Fetch::default();
+            }
+        }
         let stale: Vec<(TabId, bool, bool, bool)> = workspace
             .object_tabs()
             .map(|object| {
@@ -5577,6 +5641,9 @@ impl App {
                 self.count_rows(tab, id);
             }
         }
+        for id in ddl {
+            self.read_ddl(tab, id);
+        }
         if let Some((object, kind)) = pending {
             self.open_object(tab, object, kind, true);
         }
@@ -5609,6 +5676,12 @@ impl App {
                 )
             })
             .collect();
+        // A definition asked for meanwhile, whatever the tab holds.
+        let ddl: Vec<TabId> = workspace
+            .object_tabs()
+            .filter(|object| object.ddl.asked && !object.reopened)
+            .map(|object| object.id)
+            .collect();
         for (id, rows, structure) in waited {
             if rows {
                 self.fetch_rows(tab, id);
@@ -5617,6 +5690,9 @@ impl App {
                 self.describe(tab, id);
             }
         }
+        for id in ddl {
+            self.read_ddl(tab, id);
+        }
     }
 }
 
@@ -6919,6 +6995,14 @@ impl App {
         // brings it.
         if reopened {
             self.describe(tab, id);
+            // And the definition, where it was reopened in the DDL view.
+            let ddl = self
+                .workspace(tab)
+                .and_then(|workspace| workspace.object_tab(id))
+                .is_some_and(|object| object.view == ObjectView::Ddl);
+            if ddl {
+                self.read_ddl(tab, id);
+            }
         }
         self.fetch_page(tab, id, size);
     }
@@ -7123,6 +7207,57 @@ impl App {
         });
     }
 
+    /// Asks for the definition the DDL view of the tab `id` shows,
+    /// replacing a read still on its way.
+    pub fn read_ddl(&mut self, tab: ConnTabId, id: TabId) {
+        let reads = self.wake_to_read(tab);
+        let request = RequestId(self.next_id());
+        let Some(workspace) = self.workspace_mut(tab) else {
+            return;
+        };
+        let session = workspace.session;
+        let Some(object) = workspace.object_tab_mut(id) else {
+            return;
+        };
+        if !reads {
+            object.ddl.asked = true;
+            return;
+        }
+        let superseded = object.ddl.pending;
+        object.ddl.start(request);
+        let target = object.object.clone();
+        self.cancel(session, superseded);
+        self.backend.send(Command::ReadDdl {
+            session,
+            request,
+            object: target,
+        });
+    }
+
+    /// Reload's part for the DDL view of the tab `id`: the definition is
+    /// read again where the view shows, and forgotten where it does not,
+    /// for the view to read when it is shown next. One under an error
+    /// goes, as a retry's does.
+    fn reload_ddl(&mut self, tab: ConnTabId, id: TabId) {
+        let Some(workspace) = self.workspace_mut(tab) else {
+            return;
+        };
+        let session = workspace.session;
+        let Some(object) = workspace.object_tab_mut(id) else {
+            return;
+        };
+        if object.view == ObjectView::Ddl {
+            if object.ddl.shown_error().is_some() {
+                object.ddl.value = None;
+            }
+            self.read_ddl(tab, id);
+        } else {
+            let pending = std::mem::take(&mut object.ddl).pending;
+            object.ddl_read = None;
+            self.cancel(session, pending);
+        }
+    }
+
     /// Text for the clipboard: the selected cell, or its whole row as TSV.
     /// What the grid shows: a pending cell gives its new value, as the row
     /// panel's copy does.
diff --git a/src/app/labels.rs b/src/app/labels.rs
index c3b48327..35ae4e4e 100644
--- a/src/app/labels.rs
+++ b/src/app/labels.rs
@@ -76,7 +76,7 @@ impl App {
 
     /// The chooser of the tab `id`, where it has a place for one: its
     /// Structure view's, on the tab itself, while that view shows, and
-    /// else the one of its editor's picker.
+    /// else the one of its editor's picker. The DDL view has none.
     fn chooser_mut(&mut self, tab: ConnTabId, id: TabId) -> Option<&mut Option<Chooser>> {
         let object = self.object_tab_mut(tab, id)?;
         match object.view {
@@ -85,6 +85,7 @@ impl App {
                 let picker = object.edits.editor.as_mut()?.picker.as_mut()?;
                 Some(&mut picker.chooser)
             }
+            ObjectView::Ddl => None,
         }
     }
 
@@ -112,6 +113,7 @@ impl App {
                 let lines = workspace.label_lines(&picker.target, &[&picker.key]);
                 Some((picker.target.clone(), lines))
             }
+            ObjectView::Ddl => None,
         }
     }
 
@@ -187,6 +189,8 @@ impl App {
                         }
                     })
                 }
+                // Nothing there chooses a label.
+                ObjectView::Ddl => None,
             }
         });
         let Some(asked) = asked else {
diff --git a/src/backend.rs b/src/backend.rs
index 51d65b43..a454e952 100644
--- a/src/backend.rs
+++ b/src/backend.rs
@@ -99,6 +99,14 @@ pub enum Command {
         request: RequestId,
         object: ObjectRef,
     },
+    /// Read the definition of a table or view for the DDL view (see
+    /// `Connection::ddl`): the app's own read of the catalog, never a run
+    /// of the SQL editor's, and kept in no history.
+    ReadDdl {
+        session: SessionId,
+        request: RequestId,
+        object: ObjectRef,
+    },
     FetchRows {
         session: SessionId,
         request: RequestId,
@@ -310,6 +318,11 @@ pub enum Event {
         request: RequestId,
         result: Result<Structure, Error>,
     },
+    Ddl {
+        session: SessionId,
+        request: RequestId,
+        result: Result<tabletist_db::Ddl, Error>,
+    },
     Rows {
         session: SessionId,
         request: RequestId,
@@ -2107,6 +2120,7 @@ fn session_of(command: &Command) -> SessionId {
         | Command::ListSchemas { session, .. }
         | Command::ListObjects { session, .. }
         | Command::Describe { session, .. }
+        | Command::ReadDdl { session, .. }
         | Command::FetchRows { session, .. }
         | Command::CountRows { session, .. }
         | Command::Lookup { session, .. }
@@ -2138,6 +2152,7 @@ fn request_of(command: &Command) -> Option<RequestId> {
         Command::ListSchemas { request, .. }
         | Command::ListObjects { request, .. }
         | Command::Describe { request, .. }
+        | Command::ReadDdl { request, .. }
         | Command::FetchRows { request, .. }
         | Command::CountRows { request, .. }
         | Command::Lookup { request, .. }
@@ -2191,6 +2206,13 @@ fn fail(outbox: &Outbox, command: Command, error: Error) {
             request,
             result: Err(error),
         },
+        Command::ReadDdl {
+            session, request, ..
+        } => Event::Ddl {
+            session,
+            request,
+            result: Err(error),
+        },
         Command::FetchRows {
             session, request, ..
         } => Event::Rows {
@@ -2399,6 +2421,20 @@ async fn run_session(
                 });
                 lost
             }
+            Command::ReadDdl {
+                session,
+                request,
+                object,
+            } => {
+                let result = connection.ddl(object).await;
+                let lost = lost_error(&result);
+                outbox.emit(Event::Ddl {
+                    session: *session,
+                    request: *request,
+                    result,
+                });
+                lost
+            }
             Command::FetchRows {
                 session,
                 request,
diff --git a/src/history.rs b/src/history.rs
index e6de8ef5..0baf2f93 100644
--- a/src/history.rs
+++ b/src/history.rs
@@ -680,6 +680,7 @@ mod tests {
                 ..Default::default()
             },
             pinned: true,
+            view: String::new(),
             rest: Default::default(),
         });
         let mut failed = read("SELECT id, created FROM book_covers");
diff --git a/src/model.rs b/src/model.rs
index 39acf853..01269ec5 100644
--- a/src/model.rs
+++ b/src/model.rs
@@ -1098,6 +1098,12 @@ pub enum Action {
         tab: ConnTabId,
         object_tab: TabId,
     },
+    /// Read the DDL view's definition again: its Try again, and the view
+    /// that holds none and waits for none.
+    RetryDdl {
+        tab: ConnTabId,
+        object_tab: TabId,
+    },
     /// The connection dialog's driver switch.
     SetDriver(Driver),
     /// Drop the connection dialog's saved database password: saved with
@@ -3579,9 +3585,12 @@ impl StartRecent {
 
 #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
 pub enum ObjectView {
+    /// The rows, which the switch calls Rows.
     #[default]
     Data,
     Structure,
+    /// The definition as text (`tabletist_db::Ddl`).
+    Ddl,
 }
 
 /// A cell of an object tab's page or of a SQL result: its row there, and
@@ -3681,6 +3690,13 @@ pub struct ObjectTab {
     pub page_size: u32,
     pub rows: Fetch<RowPage>,
     pub structure: Fetch<Structure>,
+    /// The definition the DDL view shows, read when the view is first
+    /// shown and kept with the tab, as its structure is: a refresh reads
+    /// it again. A read the view was left during is dropped.
+    pub ddl: Fetch<tabletist_db::Ddl>,
+    /// When the definition the tab holds arrived: the view says how long
+    /// ago it was read.
+    pub ddl_read: Option<std::time::Instant>,
     pub selection: Option<CellPos>,
     /// The rows selected with the cursor's, where there are several.
     /// Written with the cursor, by [`ObjectTab::set_cursor`].
@@ -3928,6 +3944,8 @@ impl ObjectTab {
             view: ObjectView::Data,
             rows: Fetch::default(),
             structure: Fetch::default(),
+            ddl: Fetch::default(),
+            ddl_read: None,
             selection: None,
             range: None,
             estimated_rows,
@@ -3957,6 +3975,7 @@ impl ObjectTab {
         [
             self.rows.pending,
             self.structure.pending,
+            self.ddl.pending,
             self.count.pending,
             self.edits.saving.as_ref().map(|saving| saving.request),
         ]
diff --git a/src/session.rs b/src/session.rs
index 587d0a8f..4a533d87 100644
--- a/src/session.rs
+++ b/src/session.rs
@@ -10,7 +10,7 @@ use serde::{Deserialize, Serialize};
 use tabletist_db::{FilterOp, ObjectKind, ObjectRef, Sort, SortDir};
 
 use crate::connections::ConnectionId;
-use crate::model::{FilterBar, FilterRow, ObjectTab, TabId, Workspace};
+use crate::model::{FilterBar, FilterRow, ObjectTab, ObjectView, TabId, Workspace};
 
 /// The version this one writes.
 const VERSION: u32 = 1;
@@ -92,6 +92,10 @@ pub struct KeptTab {
     /// A preview tab is not pinned: the next single click replaces it.
     #[serde(default = "on", skip_serializing_if = "is_on")]
     pub pinned: bool,
+    /// The view it shows: `structure` or `ddl`. None for the rows, and
+    /// for a word this version does not know.
+    #[serde(default, skip_serializing_if = "String::is_empty")]
+    pub view: String,
     /// What a newer version wrote of the tab that this one does not know.
     #[serde(flatten)]
     pub rest: BTreeMap<String, serde_json::Value>,
@@ -314,6 +318,7 @@ impl KeptTab {
                 ..KeptFilter::default()
             },
             pinned: object.pinned,
+            view: view_name(object.view).to_owned(),
             rest: BTreeMap::new(),
         }
     }
@@ -329,6 +334,7 @@ impl KeptTab {
         tab.query.sort = self.sorts();
         tab.filter = self.bar();
         (tab.query.filters, tab.query.raw_where) = tab.filter.to_query();
+        tab.view = view_of(&self.view);
         tab.reopened = true;
         Some(tab)
     }
@@ -378,6 +384,22 @@ impl KeptTab {
     }
 }
 
+fn view_name(view: ObjectView) -> &'static str {
+    match view {
+        ObjectView::Data => "",
+        ObjectView::Structure => "structure",
+        ObjectView::Ddl => "ddl",
+    }
+}
+
+fn view_of(name: &str) -> ObjectView {
+    match name {
+        "structure" => ObjectView::Structure,
+        "ddl" => ObjectView::Ddl,
+        _ => ObjectView::Data,
+    }
+}
+
 fn kind_name(kind: ObjectKind) -> &'static str {
     match kind {
         ObjectKind::Table => "table",
diff --git a/src/ui/history_tests.rs b/src/ui/history_tests.rs
index 29bc21d9..4877348c 100644
--- a/src/ui/history_tests.rs
+++ b/src/ui/history_tests.rs
@@ -698,6 +698,7 @@ fn reapply_filter_opens_the_table_with_its_filter_and_sort() {
             ..Default::default()
         },
         pinned: true,
+        view: String::new(),
         rest: Default::default(),
     });
     shown_with(&mut harness, tab, vec![filter]);
````

- [ ] **Step 4: Run the tests to see them pass**

```bash
~/.cargo/bin/cargo test --locked --lib -- session:: startup_tests
```

Expected: all pass. (No view shows the DDL yet: `ObjectView::Ddl` draws the rows until Task 3.)

- [ ] **Step 5: Run the checks** (above). Expected: every suite passes, the app's library with 3305 tests.

- [ ] **Step 6: Commit**

```bash
git add src/backend.rs src/model.rs src/app.rs src/app/labels.rs src/app/startup_tests.rs src/session.rs src/history.rs src/ui/history_tests.rs src/testing.rs
git commit -m "Read and keep a table's DDL with its tab"
```

---

### Task 3: The DDL view

**Files:**
- Modify: `docs/_guide/browsing-data.md`
- Modify: `docs/_guide/omarchy.md`
- Modify: `src/ui/data_view.rs`
- Create: `src/ui/ddl.rs`
- Create: `src/ui/ddl_tests.rs`
- Modify: `src/ui/keys.rs`
- Modify: `src/ui/mod.rs`
- Modify: `src/ui/workspace.rs`

- [ ] **Step 1: Write the failing tests**

The switch's tests find **Rows** and its tabs, in `src/ui/mod.rs` and `src/ui/data_view.rs` (the grid is also named "Rows": a test that tabs to the grid asks for its role); the view's own tests.

````diff
diff --git a/src/ui/data_view.rs b/src/ui/data_view.rs
index 99eb36a4..634bb0dd 100644
--- a/src/ui/data_view.rs
+++ b/src/ui/data_view.rs
@@ -2860,8 +2860,9 @@ mod tests {
             } else {
                 "13 rows · 6 columns · public"
             };
-            let mut parts = vec![("Data", bounds(&tree, "Data", Role::Button))];
-            parts.push(("Structure", bounds(&tree, "Structure", Role::Button)));
+            let mut parts = vec![("Rows", bounds(&tree, "Rows", Role::Tab))];
+            parts.push(("Structure", bounds(&tree, "Structure", Role::Tab)));
+            parts.push(("DDL", bounds(&tree, "DDL", Role::Tab)));
             assert!(
                 parts.iter().all(|(_, rect)| rect.is_some()),
                 "the switch is missing in {}",
diff --git a/src/ui/ddl_tests.rs b/src/ui/ddl_tests.rs
new file mode 100644
index 00000000..44bfe37b
--- /dev/null
+++ b/src/ui/ddl_tests.rs
@@ -0,0 +1,751 @@
+//! The DDL view, checked against its spec
+//! (`docs/superpowers/specs/2026-10-24-ddl-view-design.md`): the third
+//! view of a table, read once and kept with the tab, its text read-only,
+//! and a read that failed said as the database said it.
+
+use std::time::{Duration, Instant};
+
+use egui::accesskit::Role;
+use egui::{Key, Modifiers};
+use tabletist_db::ddl::{Part, PartKind};
+use tabletist_db::{Ddl, Driver, Error, Named, ObjectKind, ObjectRef};
+
+use crate::backend::Command;
+use crate::model::{Action, ConnTabId, ObjectView, TabId};
+use crate::testing::{Harness, book_covers_page, book_covers_structure, node};
+use crate::theme::Look;
+
+/// The Bookshop's `book_covers` as `driver` writes it: a table, two
+/// indexes, two triggers (PostgreSQL's built from its catalog, with
+/// comments).
+fn book_covers(driver: Driver) -> Ddl {
+    let part = |kind, statements: &[&str]| Part {
+        kind,
+        statements: statements.iter().map(|s| s.to_string()).collect(),
+    };
+    match driver {
+        Driver::Postgres => Ddl {
+            driver,
+            server: "PostgreSQL 17.2".into(),
+            heading: Some("public.book_covers · built from the catalog · PostgreSQL 17.2".into()),
+            parts: vec![
+                part(
+                    PartKind::Object,
+                    &[
+                        "CREATE TABLE public.book_covers (\n    id integer NOT NULL,\n    \
+                       kind text NOT NULL,\n    CONSTRAINT book_covers_pkey PRIMARY KEY (id)\n);",
+                    ],
+                ),
+                part(
+                    PartKind::Indexes,
+                    &[
+                        "CREATE INDEX book_covers_book_id ON public.book_covers USING btree (book_id)",
+                    ],
+                ),
+                part(
+                    PartKind::Comments,
+                    &[
+                        "COMMENT ON TABLE public.book_covers IS 'Cover images, one for each side of a book';",
+                    ],
+                ),
+            ],
+        },
+        Driver::MySql => Ddl {
+            driver,
+            server: "MySQL 8.4.3".into(),
+            heading: None,
+            parts: vec![part(
+                PartKind::Object,
+                &[
+                    "CREATE TABLE `book_covers` (\n  `id` int NOT NULL,\n  PRIMARY KEY (`id`)\n) ENGINE=InnoDB",
+                ],
+            )],
+        },
+        Driver::Sqlite => Ddl {
+            driver,
+            server: "SQLite 3.46.0".into(),
+            heading: None,
+            parts: vec![
+                part(
+                    PartKind::Object,
+                    &["CREATE TABLE book_covers (\n    id INTEGER PRIMARY KEY\n)"],
+                ),
+                part(
+                    PartKind::Indexes,
+                    &["CREATE INDEX book_covers_book_id ON book_covers (book_id)"],
+                ),
+            ],
+        },
+    }
+}
+
+/// The Bookshop's covers on `driver`, in `look`, their rows read.
+fn covers(look: Look, driver: Driver) -> (Harness, ConnTabId, TabId) {
+    let mut harness = Harness::new();
+    harness.set_look(look);
+    let tab = harness.connect_fake();
+    harness.app.workspace_mut(tab).unwrap().driver = driver;
+    harness.app.apply(Action::OpenObject {
+        tab,
+        object: ObjectRef::new("main", "book_covers"),
+        kind: ObjectKind::Table,
+        pin: true,
+    });
+    harness.answer_structure(book_covers_structure());
+    harness.answer_rows(book_covers_page(3));
+    let id = harness.app.workspace(tab).unwrap().active_tab.unwrap();
+    harness.settle();
+    (harness, tab, id)
+}
+
+fn set_view(harness: &mut Harness, tab: ConnTabId, id: TabId, view: ObjectView) {
+    harness.app.apply(Action::SetView {
+        tab,
+        object_tab: id,
+        view,
+    });
+    harness.settle();
+}
+
+/// The `ReadDdl`s sent since `from`, with what each asked for.
+fn reads(harness: &Harness, from: usize) -> Vec<ObjectRef> {
+    harness.app.backend.sent[from..]
+        .iter()
+        .filter_map(|command| match command {
+            Command::ReadDdl { object, .. } => Some(object.clone()),
+            _ => None,
+        })
+        .collect()
+}
+
+/// The covers' DDL view on `driver` in `look`, its definition read.
+fn ddl_view(look: Look, driver: Driver) -> (Harness, ConnTabId, TabId) {
+    let (mut harness, tab, id) = covers(look, driver);
+    set_view(&mut harness, tab, id, ObjectView::Ddl);
+    harness.answer_ddl(Ok(book_covers(driver)));
+    harness.settle();
+    (harness, tab, id)
+}
+
+/// The DDL view is the switch's third tab, after Rows and Structure,
+/// chosen when it shows.
+#[test]
+fn the_switch_is_rows_structure_ddl_as_tabs() {
+    for look in Look::ALL {
+        let (mut harness, _tab, _id) = ddl_view(look, Driver::Postgres);
+        let tree = harness.settle();
+        let list = node(&tree, "View", Role::TabList).expect("the switch is a tab list");
+        let of = |id| {
+            tree.nodes
+                .iter()
+                .find(|(node, _)| *node == id)
+                .map(|(_, node)| node)
+        };
+        // Its children in their order, which a screen reader reads.
+        let tabs: Vec<(String, bool)> = of(list)
+            .unwrap()
+            .children()
+            .iter()
+            .filter_map(|id| of(*id))
+            .filter(|node| node.role() == Role::Tab)
+            .map(|node| {
+                (
+                    node.label().unwrap_or_default().to_owned(),
+                    node.is_selected() == Some(true),
+                )
+            })
+            .collect();
+        assert_eq!(
+            tabs,
+            [
+                ("Rows".to_owned(), false),
+                ("Structure".to_owned(), false),
+                ("DDL".to_owned(), true)
+            ],
+            "{}",
+            look.name
+        );
+    }
+}
+
+/// The view reads the definition once, with the
+/// app's own command (no run of the SQL editor's, nothing kept in
+/// History), and shows it as a read-only text named for the table.
+#[test]
+fn the_view_reads_the_definition_and_shows_it_read_only() {
+    for look in Look::ALL {
+        let (mut harness, tab, id) = covers(look, Driver::Postgres);
+        let from = harness.app.backend.sent.len();
+        set_view(&mut harness, tab, id, ObjectView::Ddl);
+        let sent = &harness.app.backend.sent[from..];
+        assert!(
+            sent.iter()
+                .all(|command| matches!(command, Command::ReadDdl { .. })),
+            "{sent:?}"
+        );
+        assert_eq!(
+            reads(&harness, from),
+            [ObjectRef::new("main", "book_covers")]
+        );
+        harness.answer_ddl(Ok(book_covers(Driver::Postgres)));
+        let tree = harness.settle();
+        let text = node(&tree, "DDL of book_covers", Role::MultilineTextInput)
+            .expect("the text is a multi-line field");
+        let (_, field) = tree.nodes.iter().find(|(node, _)| *node == text).unwrap();
+        assert!(field.is_read_only(), "{}", look.name);
+        assert_eq!(
+            field.value(),
+            Some(book_covers(Driver::Postgres).text().as_str()),
+            "{}",
+            look.name
+        );
+        // A number for each line, beside it.
+        for number in ["1", "2", "8"] {
+            assert!(
+                harness.painted.iter().any(|(text, _)| text == number),
+                "{number} in {}",
+                look.name
+            );
+        }
+        // Nothing of it reached a SQL editor's run or the history.
+        let sent = &harness.app.backend.sent[from..];
+        assert!(
+            !sent
+                .iter()
+                .any(|command| matches!(command, Command::RunSql { .. } | Command::History(_))),
+            "{sent:?}"
+        );
+    }
+}
+
+/// A definition read is kept with the tab; the view shown
+/// again reads nothing, and Reload reads it again.
+#[test]
+fn a_second_open_reads_nothing_and_reload_reads_again() {
+    let (mut harness, tab, id) = ddl_view(Look::macos(), Driver::Postgres);
+    let from = harness.app.backend.sent.len();
+    set_view(&mut harness, tab, id, ObjectView::Data);
+    set_view(&mut harness, tab, id, ObjectView::Ddl);
+    assert_eq!(reads(&harness, from), []);
+    assert!(harness.has("DDL of book_covers"));
+    harness.app.apply(Action::Refresh(tab));
+    assert_eq!(
+        reads(&harness, from),
+        [ObjectRef::new("main", "book_covers")]
+    );
+    // The text stays while it is read again.
+    assert!(harness.has("DDL of book_covers"));
+}
+
+/// Reload in another view forgets the definition: the DDL view reads it
+/// when it is shown next.
+#[test]
+fn reload_in_another_view_forgets_the_definition() {
+    let (mut harness, tab, id) = ddl_view(Look::macos(), Driver::Postgres);
+    set_view(&mut harness, tab, id, ObjectView::Data);
+    let from = harness.app.backend.sent.len();
+    harness.app.apply(Action::Refresh(tab));
+    assert_eq!(reads(&harness, from), []);
+    set_view(&mut harness, tab, id, ObjectView::Ddl);
+    assert_eq!(
+        reads(&harness, from),
+        [ObjectRef::new("main", "book_covers")]
+    );
+}
+
+/// Leaving the view while it reads stops the read, and
+/// coming back reads again.
+#[test]
+fn leaving_the_view_cancels_its_read() {
+    let (mut harness, tab, id) = covers(Look::macos(), Driver::Postgres);
+    set_view(&mut harness, tab, id, ObjectView::Ddl);
+    let request = harness
+        .app
+        .backend
+        .sent
+        .iter()
+        .rev()
+        .find_map(|command| match command {
+            Command::ReadDdl { request, .. } => Some(*request),
+            _ => None,
+        })
+        .expect("a read was sent");
+    let from = harness.app.backend.sent.len();
+    set_view(&mut harness, tab, id, ObjectView::Structure);
+    assert!(
+        harness.app.backend.sent[from..]
+            .iter()
+            .any(|command| matches!(command, Command::Cancel { request: r, .. } if *r == request)),
+        "{:?}",
+        &harness.app.backend.sent[from..]
+    );
+    let object = harness.app.workspace(tab).unwrap().object_tab(id).unwrap();
+    assert!(!object.ddl.is_loading());
+    let from = harness.app.backend.sent.len();
+    set_view(&mut harness, tab, id, ObjectView::Ddl);
+    assert_eq!(
+        reads(&harness, from),
+        [ObjectRef::new("main", "book_covers")]
+    );
+}
+
+/// MySQL's refusal, as the server words it.
+fn denied() -> Error {
+    Error::Query {
+        code: Some("42000".into()),
+        message: "SHOW command denied to user 'reader'@'%' for table 'book_covers'".into(),
+        detail: None,
+        hint: None,
+        named: Box::new(Named {
+            number: Some(1142),
+            ..Named::default()
+        }),
+    }
+}
+
+/// A read the server refused says whose definition, the
+/// server's number and words, and what it means; no text shows, and the
+/// other views work on.
+#[test]
+fn a_refused_read_says_the_servers_error_and_shows_no_text() {
+    for look in Look::ALL {
+        let (mut harness, tab, id) = covers(look, Driver::MySql);
+        set_view(&mut harness, tab, id, ObjectView::Ddl);
+        harness.answer_ddl(Err(denied()));
+        let title = look.label("Couldn't read the definition of book_covers");
+        assert!(harness.has(&title), "{}", look.name);
+        assert!(
+            harness.has("1142 · SHOW command denied to user 'reader'@'%' for table 'book_covers'"),
+            "{}",
+            look.name
+        );
+        assert!(
+            harness.has(
+                "The connection's user can't run SHOW CREATE TABLE here. Rows and Structure still work."
+            ),
+            "{}",
+            look.name
+        );
+        assert!(!harness.has("DDL of book_covers"), "{}", look.name);
+        // Copy error copies the number and the words.
+        harness.click("Copy error");
+        assert_eq!(
+            harness.copied.as_deref(),
+            Some("1142 · SHOW command denied to user 'reader'@'%' for table 'book_covers'")
+        );
+        // Try again reads again.
+        let from = harness.app.backend.sent.len();
+        harness.click("Try again");
+        assert_eq!(
+            reads(&harness, from),
+            [ObjectRef::new("main", "book_covers")]
+        );
+        // The rows and the structure show as they did.
+        harness.answer_ddl(Err(denied()));
+        set_view(&mut harness, tab, id, ObjectView::Data);
+        assert!(harness.has("Row 1"), "{}", look.name);
+        set_view(&mut harness, tab, id, ObjectView::Structure);
+        assert!(harness.has("Columns"), "{}", look.name);
+    }
+}
+
+/// A read that goes on says so once it has lasted: the macOS header and
+/// the terminal's line, each announced.
+#[test]
+fn a_long_read_says_it_reads_the_definition() {
+    for look in Look::ALL {
+        let (mut harness, tab, id) = covers(look, Driver::Postgres);
+        set_view(&mut harness, tab, id, ObjectView::Ddl);
+        let words = if look.terminal {
+            "reading the definition…"
+        } else {
+            "Reading the definition…"
+        };
+        assert!(!harness.has(words), "not at once, in {}", look.name);
+        harness
+            .app
+            .workspace_mut(tab)
+            .unwrap()
+            .object_tab_mut(id)
+            .unwrap()
+            .ddl
+            .started = Some(Instant::now() - Duration::from_secs(1));
+        let tree = harness.settle();
+        let said = node(&tree, words, Role::Label).expect("the wait is said");
+        let (_, said) = tree.nodes.iter().find(|(id, _)| *id == said).unwrap();
+        assert_eq!(
+            said.live(),
+            Some(egui::accesskit::Live::Polite),
+            "{}",
+            look.name
+        );
+    }
+}
+
+/// Where the text comes from, by engine: PostgreSQL's built from the
+/// catalog, with what it leaves out; MySQL's and SQLite's as stored.
+#[test]
+fn the_view_says_where_its_text_comes_from() {
+    // Each engine's source, its name, and whether the text leaves
+    // anything out.
+    let cases = [
+        (
+            Driver::Postgres,
+            "built from the catalog",
+            "PostgreSQL",
+            true,
+        ),
+        (Driver::MySql, "as stored in the database", "MySQL", false),
+        (Driver::Sqlite, "as stored in the database", "SQLite", false),
+    ];
+    for (driver, source, engine, leaves_out) in cases {
+        let (mut harness, _, _) = ddl_view(Look::macos(), driver);
+        assert!(
+            harness.has(&format!("Read-only · {source} · read just now")),
+            "{driver:?}"
+        );
+        let lines = book_covers(driver).text().lines().count();
+        assert!(
+            harness.has(&format!("{lines} lines · {engine}")),
+            "{driver:?}"
+        );
+        let left_out =
+            "Not included: owner, grants, sequences owned by the table, row-level policies";
+        assert_eq!(harness.has(left_out), leaves_out, "{driver:?}");
+        // Reload, with its key.
+        let reload =
+            crate::ui::keys::written(&harness.ctx, &Look::macos(), crate::keymap::Command::Reload);
+        assert!(harness.has("Reload"), "{driver:?}");
+        assert!(
+            harness.painted.iter().any(|(text, _)| *text == *reload),
+            "{driver:?}"
+        );
+        // The terminal says it over the text.
+        let (mut harness, _, _) = ddl_view(Look::omarchy(), driver);
+        let server = book_covers(driver).server.to_lowercase();
+        assert!(
+            harness.has(&format!("ddl · read-only · {source} · {server}")),
+            "{driver:?}"
+        );
+    }
+}
+
+/// The source line's Reload reads the definition again.
+#[test]
+fn the_source_lines_reload_reads_again() {
+    let (mut harness, _, _) = ddl_view(Look::macos(), Driver::Sqlite);
+    let from = harness.app.backend.sent.len();
+    harness.click("Reload");
+    assert_eq!(
+        reads(&harness, from),
+        [ObjectRef::new("main", "book_covers")]
+    );
+}
+
+/// The text is selected and copied as any text: all of it with the
+/// keyboard.
+#[test]
+fn the_text_is_selected_and_copied_with_the_keyboard() {
+    let (mut harness, tab, id) = ddl_view(Look::macos(), Driver::Postgres);
+    let field = super::ddl::text_id(tab, id);
+    harness.ctx.memory_mut(|memory| memory.request_focus(field));
+    harness.settle();
+    harness.press(Key::A, Modifiers::COMMAND);
+    harness.copy(false);
+    assert_eq!(
+        harness.copied.as_deref(),
+        Some(book_covers(Driver::Postgres).text().as_str())
+    );
+    // And it cannot be typed into.
+    harness.frame(vec![egui::Event::Text("x".into())]);
+    let object = harness.app.workspace(tab).unwrap().object_tab(id).unwrap();
+    assert_eq!(
+        object.ddl.value.as_ref().map(Ddl::text),
+        Some(book_covers(Driver::Postgres).text())
+    );
+}
+
+/// Every engine's definition shows in the view.
+#[test]
+fn every_engine_shows_its_definition() {
+    for driver in [Driver::Postgres, Driver::MySql, Driver::Sqlite] {
+        let (mut harness, _, _) = ddl_view(Look::macos(), driver);
+        let tree = harness.settle();
+        let text = node(&tree, "DDL of book_covers", Role::MultilineTextInput);
+        assert!(text.is_some(), "{driver:?}");
+    }
+}
+
+/// "read just now" to "read 2 h ago".
+#[test]
+fn the_age_is_said_in_its_largest_unit() {
+    let locale = crate::i18n::Locale::English;
+    let said = |seconds| super::ddl::read_ago(Duration::from_secs(seconds), locale);
+    assert_eq!(said(0), "read just now");
+    assert_eq!(said(12), "read 12 s ago");
+    assert_eq!(said(180), "read 3 min ago");
+    assert_eq!(said(7_300), "read 2 h ago");
+}
+
+/// The keys of `c` typed, as a window sends them: its key, then its text.
+fn letter(harness: &mut Harness, c: char) {
+    let key = Key::from_name(&c.to_ascii_uppercase().to_string()).expect("a letter's key");
+    let shift = if c.is_uppercase() {
+        Modifiers::SHIFT
+    } else {
+        Modifiers::NONE
+    };
+    harness.frame(vec![
+        crate::testing::key(key, shift),
+        egui::Event::Text(c.to_string()),
+    ]);
+    harness.frame(vec![crate::testing::release(key, shift)]);
+    harness.settle();
+}
+
+/// Gives the text the keyboard, as F6 does.
+fn focus_text(harness: &mut Harness, tab: ConnTabId, id: TabId) {
+    let field = super::ddl::text_id(tab, id);
+    harness.ctx.memory_mut(|memory| memory.request_focus(field));
+    harness.settle();
+}
+
+/// The text field's value, as a screen reader reads it.
+fn field_value(harness: &mut Harness) -> Option<String> {
+    let tree = harness.settle();
+    let field = node(&tree, "DDL of book_covers", Role::MultilineTextInput)?;
+    let (_, field) = tree.nodes.iter().find(|(id, _)| *id == field)?;
+    field.value().map(str::to_owned)
+}
+
+/// The sentence of a refusal shows only where the database refused: a
+/// table gone since is no refusal. PostgreSQL's refusal has its own.
+#[test]
+fn only_a_refusal_says_what_the_user_may_not_do() {
+    let (mut harness, tab, id) = covers(Look::macos(), Driver::MySql);
+    set_view(&mut harness, tab, id, ObjectView::Ddl);
+    harness.answer_ddl(Err(Error::query("no such table or view: book_covers")));
+    assert!(harness.has("Couldn't read the definition of book_covers"));
+    assert!(harness.has("no such table or view: book_covers"));
+    assert!(!harness.has(
+        "The connection's user can't run SHOW CREATE TABLE here. Rows and Structure still work."
+    ));
+    let (mut harness, tab, id) = covers(Look::macos(), Driver::Postgres);
+    set_view(&mut harness, tab, id, ObjectView::Ddl);
+    harness.answer_ddl(Err(Error::Query {
+        code: Some("42501".into()),
+        message: "permission denied for table book_covers".into(),
+        detail: None,
+        hint: None,
+        named: Box::default(),
+    }));
+    assert!(harness.has("42501 · permission denied for table book_covers"));
+    assert!(harness.has(
+        "The connection's user can't read this part of the catalog. Rows and Structure still work."
+    ));
+}
+
+/// Try again drops what the error replaced, and reads again.
+#[test]
+fn try_again_drops_the_text_under_the_error() {
+    let (mut harness, tab, id) = ddl_view(Look::macos(), Driver::Postgres);
+    harness.app.apply(Action::Refresh(tab));
+    harness.answer_ddl(Err(denied()));
+    assert!(harness.has("Try again"));
+    let from = harness.app.backend.sent.len();
+    harness.click("Try again");
+    let object = harness.app.workspace(tab).unwrap().object_tab(id).unwrap();
+    assert!(object.ddl.value.is_none());
+    assert!(object.ddl.is_loading());
+    assert_eq!(
+        reads(&harness, from),
+        [ObjectRef::new("main", "book_covers")]
+    );
+}
+
+/// A new session reads the definition the tab in front shows. One that
+/// was on its way on a tab behind is forgotten, and read when that tab
+/// comes in front again.
+#[test]
+fn a_new_session_reads_the_definition_in_front_and_later_the_one_behind() {
+    let (mut harness, tab, id) = covers(Look::macos(), Driver::Postgres);
+    set_view(&mut harness, tab, id, ObjectView::Ddl);
+    // The read is on its way when another tab comes in front.
+    harness.click("users");
+    let users = harness.app.workspace(tab).unwrap().active_tab.unwrap();
+    assert_ne!(users, id);
+    let from = harness.app.backend.sent.len();
+    harness.reconnect_fake_as(tab, true);
+    harness.settle();
+    assert_eq!(reads(&harness, from), []);
+    let object = harness.app.workspace(tab).unwrap().object_tab(id).unwrap();
+    assert!(!object.ddl.is_loading());
+    harness.app.apply(Action::ActivateTab { tab, id });
+    harness.settle();
+    assert_eq!(
+        reads(&harness, from),
+        [ObjectRef::new("main", "book_covers")]
+    );
+    // In front, it is read on the next new session at once.
+    harness.answer_ddl(Ok(book_covers(Driver::Postgres)));
+    let from = harness.app.backend.sent.len();
+    harness.reconnect_fake_as(tab, true);
+    assert_eq!(
+        reads(&harness, from),
+        [ObjectRef::new("main", "book_covers")]
+    );
+    harness.settle();
+    assert_eq!(reads(&harness, from).len(), 1, "once");
+}
+
+/// Closing the tab stops its read.
+#[test]
+fn closing_the_tab_cancels_its_read() {
+    let (mut harness, tab, id) = covers(Look::macos(), Driver::Postgres);
+    set_view(&mut harness, tab, id, ObjectView::Ddl);
+    let request = harness
+        .app
+        .workspace(tab)
+        .unwrap()
+        .object_tab(id)
+        .unwrap()
+        .ddl
+        .pending
+        .expect("a read on its way");
+    let from = harness.app.backend.sent.len();
+    harness.app.apply(Action::CloseTab { tab, id });
+    assert!(
+        harness.app.backend.sent[from..]
+            .iter()
+            .any(|command| matches!(command, Command::Cancel { request: r, .. } if *r == request))
+    );
+}
+
+/// The DDL view has no Add row; the other views keep theirs.
+#[test]
+fn the_ddl_view_has_no_add_row() {
+    let (mut harness, tab, id) = covers(Look::macos(), Driver::Postgres);
+    harness.app.workspace_mut(tab).unwrap().access = tabletist_db::Access::Writable;
+    assert!(harness.has("Add row"));
+    set_view(&mut harness, tab, id, ObjectView::Structure);
+    assert!(harness.has("Add row"));
+    set_view(&mut harness, tab, id, ObjectView::Ddl);
+    harness.answer_ddl(Ok(book_covers(Driver::Postgres)));
+    assert!(!harness.has("Add row"));
+    // And under the text, how to copy part of it.
+    assert!(harness.has("Select text to copy part of it"));
+}
+
+/// The terminal's line names the view's keys, not the rows', and how many
+/// lines the text has.
+#[test]
+fn the_terminal_line_names_the_views_keys_and_its_lines() {
+    let (harness, _, _) = ddl_view(Look::omarchy(), Driver::Postgres);
+    let lines = book_covers(Driver::Postgres).text().lines().count();
+    let painted = |said: &str| harness.painted.iter().any(|(text, _)| text == said);
+    assert!(painted(&format!("{lines} lines")));
+    // Each key with its word, as the line paints a hint.
+    let key = |command| crate::ui::keys::written(&harness.ctx, &Look::omarchy(), command);
+    let (reload, tables) = (
+        key(crate::keymap::Command::Reload),
+        key(crate::keymap::Command::ToggleSidebar),
+    );
+    assert!(painted(&format!("{} reload", &*reload)));
+    assert!(painted(&format!("{} tables", &*tables)));
+    let rows = key(crate::keymap::Command::MoveRow);
+    assert!(
+        !painted(&format!("{} row", &*rows)),
+        "the rows' keys are not named"
+    );
+}
+
+/// The arrows stay off the sidebar while the DDL view shows.
+#[test]
+fn the_arrows_do_not_go_to_the_sidebar_from_the_ddl_view() {
+    let (mut harness, tab, _) = ddl_view(Look::macos(), Driver::Postgres);
+    harness.app.workspace_mut(tab).unwrap().pane = crate::model::Pane::Grid;
+    let before = harness.app.workspace(tab).unwrap().tree.cursor.clone();
+    harness.press(Key::ArrowDown, Modifiers::NONE);
+    assert_eq!(harness.app.workspace(tab).unwrap().tree.cursor, before);
+}
+
+/// F6 brings the keyboard to the text, in every look.
+#[test]
+fn f6_comes_to_the_text() {
+    for look in Look::ALL {
+        let (mut harness, _, _) = ddl_view(look, Driver::Postgres);
+        let mut seen = Vec::new();
+        for _ in 0..8 {
+            harness.press(Key::F6, Modifiers::NONE);
+            seen.push(super::tests::focused_name(&harness.settle()));
+        }
+        assert!(
+            seen.iter().any(|name| name == "DDL of book_covers"),
+            "{seen:?} in {}",
+            look.name
+        );
+    }
+}
+
+/// Nothing typed, pasted or cut changes the text.
+#[test]
+fn the_text_cannot_be_changed() {
+    let (mut harness, tab, id) = ddl_view(Look::macos(), Driver::Postgres);
+    focus_text(&mut harness, tab, id);
+    let text = book_covers(Driver::Postgres).text();
+    harness.frame(vec![egui::Event::Text("x".into())]);
+    harness.press(Key::Backspace, Modifiers::NONE);
+    harness.press(Key::Enter, Modifiers::NONE);
+    harness.frame(vec![egui::Event::Paste("DROP TABLE books;".into())]);
+    harness.frame(vec![egui::Event::Cut]);
+    assert_eq!(field_value(&mut harness).as_deref(), Some(text.as_str()));
+}
+
+/// With the keyboard in the text, Reload reads it again: Omarchy's `R`
+/// and the others' chord. The rows' letters do nothing there.
+#[test]
+fn reload_reads_again_from_the_text() {
+    let (mut harness, tab, id) = ddl_view(Look::omarchy(), Driver::Postgres);
+    focus_text(&mut harness, tab, id);
+    let from = harness.app.backend.sent.len();
+    for c in ['o', 'x', 'd', 'd', 'i'] {
+        letter(&mut harness, c);
+    }
+    let object = harness.app.workspace(tab).unwrap().object_tab(id).unwrap();
+    assert!(object.edits.added.is_empty() && object.edits.deleted.is_empty());
+    assert!(object.edits.editor.is_none());
+    assert_eq!(reads(&harness, from), []);
+    letter(&mut harness, 'R');
+    assert_eq!(
+        reads(&harness, from),
+        [ObjectRef::new("main", "book_covers")]
+    );
+    let (mut harness, tab, id) = ddl_view(Look::macos(), Driver::Postgres);
+    focus_text(&mut harness, tab, id);
+    let from = harness.app.backend.sent.len();
+    harness.press(Key::R, Modifiers::COMMAND);
+    assert_eq!(
+        reads(&harness, from),
+        [ObjectRef::new("main", "book_covers")]
+    );
+}
+
+/// The keyboard selects in the text as in any: Shift with an arrow from
+/// where it came in, at the top.
+#[test]
+fn shift_with_an_arrow_selects_from_the_top() {
+    for look in Look::ALL {
+        let (mut harness, tab, id) = ddl_view(look, Driver::Postgres);
+        focus_text(&mut harness, tab, id);
+        harness.press(Key::ArrowDown, Modifiers::SHIFT);
+        harness.copy(false);
+        let first = book_covers(Driver::Postgres).text();
+        let first = format!("{}\n", first.lines().next().unwrap());
+        assert_eq!(
+            harness.copied.as_deref(),
+            Some(first.as_str()),
+            "{}",
+            look.name
+        );
+    }
+}
diff --git a/src/ui/mod.rs b/src/ui/mod.rs
index e5405ef1..cccdce9c 100644
--- a/src/ui/mod.rs
+++ b/src/ui/mod.rs
@@ -23,6 +23,9 @@ mod complete_tests;
 pub mod conflict_prompt;
 pub mod connect_dialog;
 pub mod data_view;
+pub mod ddl;
+#[cfg(test)]
+mod ddl_tests;
 #[cfg(test)]
 mod delete_many_tests;
 #[cfg(test)]
@@ -6541,7 +6544,7 @@ value_tags   = true
         harness.click("Row 1");
         assert!(harness.has("user1@example.com"));
         harness.click("Structure");
-        harness.click("Data");
+        harness.click("Rows");
         assert!(harness.has("user1@example.com"));
     }
 
@@ -6792,7 +6795,7 @@ value_tags   = true
         focus_grid(&mut harness, tab);
         harness.app.apply(crate::model::Action::ToggleRowPanel(tab));
         assert!(!harness.app.workspace(tab).unwrap().row_panel);
-        focus(&mut harness, "Structure", egui::accesskit::Role::Button);
+        focus(&mut harness, "Structure", egui::accesskit::Role::Tab);
         assert_eq!(focused_name(&harness.settle()), "Structure");
         harness.press(Key::Enter, Modifiers::NONE);
         let workspace = harness.app.workspace(tab).unwrap();
@@ -12055,17 +12058,25 @@ value_tags   = true
             harness.set_look(look);
             harness.click("orders");
             harness.answer_rows(crate::testing::page(3, true));
-            let tab_to = |harness: &mut Harness, name: &str| {
+            // The stop named `name`, of `role` where one is given: the
+            // switch's Rows tab comes before the rows.
+            let tab_to = |harness: &mut Harness,
+                          name: &str,
+                          role: Option<egui::accesskit::Role>| {
                 for _ in 0..60 {
                     harness.press(Key::Tab, Modifiers::NONE);
-                    if focused_name(&harness.settle()) == name {
+                    let tree = harness.settle();
+                    let focused = tree.nodes.iter().find(|(id, _)| *id == tree.focus);
+                    let of_role = role
+                        .is_none_or(|role| focused.is_some_and(|(_, node)| node.role() == role));
+                    if focused_name(&tree) == name && of_role {
                         return;
                     }
                 }
                 panic!("{name} is no Tab stop in {}", look.name);
             };
             // Onto the tree: its cursor is on the open table, and moves.
-            tab_to(&mut harness, "Objects");
+            tab_to(&mut harness, "Objects", None);
             let workspace = harness.app.workspace(tab).unwrap();
             assert_eq!(workspace.pane, Pane::Tree, "{}", look.name);
             let before = workspace.tree.cursor.clone();
@@ -12075,7 +12086,7 @@ value_tags   = true
             assert_ne!(workspace.tree.cursor, before, "{}", look.name);
             assert_eq!(focused_name(&harness.settle()), "Objects", "{}", look.name);
             // Onto the grid: its first cell, and the arrows are its own.
-            tab_to(&mut harness, "Rows");
+            tab_to(&mut harness, "Rows", Some(egui::accesskit::Role::Group));
             let selection = |harness: &Harness| {
                 let workspace = harness.app.workspace(tab).unwrap();
                 workspace.active_object_tab().unwrap().selection
@@ -12116,13 +12127,17 @@ value_tags   = true
             let workspace = harness.app.workspace(tab).unwrap();
             workspace.active_object_tab().unwrap().view
         };
-        focus(&mut harness, "Data", egui::accesskit::Role::Button);
+        focus(&mut harness, "Rows", egui::accesskit::Role::Tab);
         harness.press(Key::ArrowRight, Modifiers::NONE);
         assert_eq!(view(&harness), crate::model::ObjectView::Structure);
         assert_eq!(focused_name(&harness.settle()), "Structure");
+        harness.press(Key::ArrowRight, Modifiers::NONE);
+        assert_eq!(view(&harness), crate::model::ObjectView::Ddl);
+        assert_eq!(focused_name(&harness.settle()), "DDL");
+        harness.press(Key::ArrowLeft, Modifiers::NONE);
         harness.press(Key::ArrowLeft, Modifiers::NONE);
         assert_eq!(view(&harness), crate::model::ObjectView::Data);
-        assert_eq!(focused_name(&harness.settle()), "Data");
+        assert_eq!(focused_name(&harness.settle()), "Rows");
         // Tab passes the choice not made.
         let mut stops = std::collections::HashSet::new();
         for _ in 0..60 {
@@ -12130,7 +12145,7 @@ value_tags   = true
             stops.insert(focused_name(&harness.settle()));
         }
         assert!(
-            stops.contains("Data") && !stops.contains("Structure"),
+            stops.contains("Rows") && !stops.contains("Structure") && !stops.contains("DDL"),
             "{stops:?}"
         );
     }
@@ -12159,9 +12174,10 @@ value_tags   = true
             };
             assert_eq!(
                 forward,
+                // The switch's Rows tab, then the rows.
                 [
                     "orders tab",
-                    "Data",
+                    "Rows",
                     "Rows",
                     "Connections",
                     field,
````

- [ ] **Step 2: Run them to see them fail**

```bash
~/.cargo/bin/cargo test --locked --lib ui::ddl_tests
```

Expected: it does not compile, there is no `ui::ddl`:

```
error[E0583]: file not found for module `ddl`
error: could not compile `tabletist` (lib test) due to 1 previous error
```

- [ ] **Step 3: Write the view, the switch, its footer and its key line**

````diff
diff --git a/docs/_guide/browsing-data.md b/docs/_guide/browsing-data.md
index fa3f0fa0..7269f471 100644
--- a/docs/_guide/browsing-data.md
+++ b/docs/_guide/browsing-data.md
@@ -1,6 +1,6 @@
 ---
 title: Browsing Data
-description: The sidebar, the data grid, sorting, filters, the row panel and the Structure view.
+description: The sidebar, the data grid, sorting, filters, the row panel, and the Structure and DDL views.
 nav_order: 3
 ---
 
@@ -87,7 +87,7 @@ A foreign key has a link next to it that opens the row it points to.
 
 ## Structure
 
-Switch a table from **Data** to **Structure** to read its columns, indexes
+Switch a table from **Rows** to **Structure** to read its columns, indexes
 and foreign keys.
 
 {% include shot.html file="macos-structure" alt="The Structure view of a table: its comment, the label for foreign keys, then its columns with their comments and its indexes" %}
@@ -116,6 +116,39 @@ No cell is edited and nothing is written to the database, so this works on
 a read-only connection. The Omarchy look does the same with `gl` and the
 `:label` lines of [its Structure view]({% link _guide/omarchy.md %}#the-structure-view).
 
+## DDL
+
+**DDL**, the third view of a table, a view or a materialized view, shows
+its definition as text you can select and copy but not change, in the SQL
+editor's colours, with line numbers. Long lines do not wrap: scroll
+sideways to read them. Each tab keeps the view it shows, and a tab the
+window reopens comes back in it.
+
+- On **PostgreSQL**, which has no `SHOW CREATE TABLE`, the text is built
+  from the catalog in the order `pg_dump` writes it: the `CREATE TABLE`
+  with its columns and constraints, then the indexes that back no
+  constraint, the triggers (a disabled one followed by the `ALTER TABLE`
+  that disables it) and the comments. Tables, types and functions are
+  named with their schema, so the text runs whatever the search path. The
+  owner, the grants, the row-level policies and the sequences the table
+  owns are not included, as the footer says. Nor are an identity's
+  sequence options (its start and step), a tablespace or a table's access
+  method. Each table's foreign keys are in its own `CREATE TABLE`, so two
+  tables that point at each other cannot be made from their texts as
+  they are.
+- On **MySQL** it is `SHOW CREATE TABLE` (or `SHOW CREATE VIEW`) exactly
+  as the server returns it, then each trigger's `SHOW CREATE TRIGGER`.
+- On **SQLite** it is the text the file keeps for the table or view, then
+  for its indexes and triggers. Indexes SQLite made by itself keep no
+  text and are left out.
+
+The definition is read once, the first time the view shows, and kept with
+the tab: Reload (Cmd/Ctrl+R, or **Reload** over the text) reads it again,
+with the rows and the structure, as Reload does in the other views.
+Reading it only reads the catalog, and adds nothing to History. Where the
+connection's user may not read it, the view shows the database's error
+and **Try again**, never part of a text; Rows and Structure still work.
+
 ## Copying
 
 On macOS and Windows, Cmd/Ctrl+C copies the selected cell and
diff --git a/docs/_guide/omarchy.md b/docs/_guide/omarchy.md
index 8e32a3f3..a10d09f5 100644
--- a/docs/_guide/omarchy.md
+++ b/docs/_guide/omarchy.md
@@ -155,8 +155,8 @@ With the search open, the matches replace the start lists:
 | `R` | Reload |
 | Ctrl+C | Cancel the running query |
 
-The Structure and Data views are clicked: switching between them has no
-key. Nor has copying a whole row.
+The `rows`, `structure` and `ddl` views are clicked: switching between
+them has no key yet. Nor has copying a whole row.
 
 ### The Agents tab
 
@@ -198,6 +198,14 @@ foreign key that points at it:
 what it does. A key or a column of bytes is not taken, and the status line
 says why.
 
+### The DDL view
+
+`ddl` in a table's header shows its definition as text, as
+[Browsing Data]({% link _guide/browsing-data.md %}#ddl) says: read-only,
+with line numbers. The header says where it comes from and from which
+server. `R` reads it again, with the keyboard in the text too, and after
+an error tries again.
+
 ### The row panel
 
 | Keys | Do |
diff --git a/src/ui/data_view.rs b/src/ui/data_view.rs
index 634bb0dd..efa4298c 100644
--- a/src/ui/data_view.rs
+++ b/src/ui/data_view.rs
@@ -153,7 +153,26 @@ pub fn paint_named(ui: &egui::Ui, x: f32, y: f32, text: Text, name: &str) -> f32
     width
 }
 
-/// The object's name and counts, the Data/Structure switch, and Add row.
+/// Makes `ui`'s node the switch of views, named "View": the views are its
+/// tabs.
+fn tab_list(ui: &egui::Ui, locale: crate::i18n::Locale) {
+    let name = gettext(locale, "View");
+    ui.ctx().accesskit_node_builder(ui.unique_id(), |node| {
+        node.set_role(egui::accesskit::Role::TabList);
+        node.set_label(name.as_ref());
+    });
+}
+
+/// Makes `response`'s node a tab of the switch, chosen or not.
+fn view_tab(ui: &egui::Ui, response: &egui::Response, selected: bool) {
+    ui.ctx().accesskit_node_builder(response.id, |node| {
+        node.set_role(egui::accesskit::Role::Tab);
+        node.set_selected(selected);
+    });
+}
+
+/// The object's name and counts, the Rows · Structure · DDL switch, and
+/// Add row (none in the DDL view).
 pub fn header(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId, object_tab: TabId) {
     let locale = app.locale;
     let palette = app.palette;
@@ -165,7 +184,14 @@ pub fn header(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId, object_tab: TabI
     let view = object.view;
     // The rows to add: the terminal's header counts them at its right.
     let added = object.edits.added.len();
-    let parts = subtitle(object, &look, locale);
+    // The terminal's DDL view says what its text is, in the place of the
+    // counts.
+    // How long the DDL view's read has gone on, while it goes on.
+    let reading = object.ddl.running_for();
+    let parts = match (look.terminal, view) {
+        (true, ObjectView::Ddl) => super::ddl::subtitle(object, locale),
+        _ => subtitle(object, &look, locale),
+    };
     let summary = parts.join(" · ");
     // Why Add row cannot be pressed, where it cannot: the table takes no
     // row, or its page is not there yet.
@@ -205,14 +231,14 @@ pub fn header(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId, object_tab: TabI
             widgets::hline(ui, rect.x_range(), rect.bottom() - 0.5, divider);
             let left = rect.left() + side(&look);
             let right = rect.right() - side(&look);
-            // The views have no key: `d` and `s` are the rows' letters.
             let views = [
-                (ObjectView::Data, gettext(locale, "Data"), "data"),
+                (ObjectView::Data, gettext(locale, "Rows"), "rows"),
                 (
                     ObjectView::Structure,
                     gettext(locale, "Structure"),
                     "structure",
                 ),
+                (ObjectView::Ddl, gettext(locale, "DDL"), "ddl"),
             ];
             let width = |role: TextRole, text: &str| role.width(ui.ctx(), look.faces, text);
             if look.terminal {
@@ -243,12 +269,22 @@ pub fn header(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId, object_tab: TabI
                 {
                     paint_named(ui, x, center, sub(&shown, palette.dim), &summary);
                 }
-                // `data  structure`, the active one underlined.
-                let mut x = right;
-                for ((target, label, salt), width) in views.iter().zip(widths).rev() {
+                // `rows  structure  ddl`, the active one underlined, in
+                // their order from where they start.
+                let all = widths.iter().sum::<f32>() + 14.0 * (widths.len() - 1) as f32;
+                let start = right - all;
+                let list = Rect::from_min_max(
+                    pos2(start - 4.0, rect.top()),
+                    pos2(right + 4.0, rect.bottom()),
+                );
+                let list = ui.new_child(egui::UiBuilder::new().id_salt("views").max_rect(list));
+                tab_list(&list, locale);
+                let ui = &list;
+                let mut x = start;
+                for ((target, label, salt), width) in views.iter().zip(widths) {
                     let label = label.to_lowercase();
                     let hit = Rect::from_min_size(
-                        pos2(x - width - 4.0, rect.top()),
+                        pos2(x - 4.0, rect.top()),
                         vec2(width + 8.0, rect.height()),
                     );
                     let response = ui.interact(hit, ui.id().with(("view", *salt)), Sense::click());
@@ -264,22 +300,18 @@ pub fn header(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId, object_tab: TabI
                     response.widget_info(|| {
                         WidgetInfo::selected(WidgetType::Button, true, selected, &name)
                     });
+                    view_tab(ui, &response, selected);
                     let color = if selected {
                         palette.accent
                     } else {
                         palette.dim
                     };
                     // The word takes the state's colour.
-                    widgets::paint_text(
-                        ui,
-                        x - width,
-                        center,
-                        Text::new(&look).add(role, &label, color),
-                    );
+                    widgets::paint_text(ui, x, center, Text::new(&look).add(role, &label, color));
                     if selected {
                         let y = center + line(ui, role, &look) / 2.0 + 2.0;
                         ui.painter().rect_filled(
-                            Rect::from_min_max(pos2(x - width, y), pos2(x, y + 2.0)),
+                            Rect::from_min_max(pos2(x, y), pos2(x + width, y + 2.0)),
                             CornerRadius::ZERO,
                             palette.accent,
                         );
@@ -291,13 +323,12 @@ pub fn header(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId, object_tab: TabI
                             view: *target,
                         });
                     }
-                    x -= width + 14.0;
+                    x += width + 14.0;
                 }
-                // In the green of a new row, 14 before the first view:
-                // the loop left `x` there.
+                // In the green of a new row, 14 before the first view.
                 if let Some(text) = &added {
                     let text = Text::one(&look, role, text, palette.success);
-                    widgets::paint_text_right(ui, x, center, text);
+                    widgets::paint_text_right(ui, start - 14.0, center, text);
                 }
                 return;
             }
@@ -329,13 +360,30 @@ pub fn header(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId, object_tab: TabI
                 };
                 if short { button.gap(0.0) } else { button }
             };
+            // In the DDL view, what it says while it reads stands where
+            // Add row would.
+            let wait = match view {
+                ObjectView::Ddl => {
+                    let skin = (&look, &palette, locale);
+                    let width = super::ddl::header_wait_width(ui, reading, skin);
+                    if width > 0.0 { width + 12.0 } else { 0.0 }
+                }
+                ObjectView::Data | ObjectView::Structure => 0.0,
+            };
             let stack = |button: Option<bool>| {
                 let button = button.map_or(0.0, |short| 12.0 + add_row(short).width(ui, &look));
-                right - left - 16.0 - 12.0 - switch - button
+                right - left - 16.0 - 12.0 - switch - button - wait
             };
             let title_width = width(title_role, &name);
-            let button = [Some(false), Some(true), None]
-                .into_iter()
+            // The DDL view has no Add row.
+            let rows = if view == ObjectView::Ddl {
+                &[None][..]
+            } else {
+                &[Some(false), Some(true), None][..]
+            };
+            let button = rows
+                .iter()
+                .copied()
                 .find(|button| stack(*button) >= title_width)
                 .flatten();
             let room = stack(button).max(0.0);
@@ -350,6 +398,10 @@ pub fn header(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId, object_tab: TabI
             ui.painter()
                 .rect_filled(track, CornerRadius::same(look.radius), palette.surface);
             x += 3.0;
+            let mut list = ui.new_child(egui::UiBuilder::new().id_salt("views").max_rect(track));
+            tab_list(&list, locale);
+            let header = &mut *ui;
+            let ui = &mut list;
             for (index, ((target, label, _), width)) in views.iter().zip(widths).enumerate() {
                 let cell = Rect::from_min_size(pos2(x, track.top() + 3.0), vec2(width, 28.0));
                 x += width;
@@ -366,6 +418,7 @@ pub fn header(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId, object_tab: TabI
                 response.widget_info(|| {
                     WidgetInfo::selected(WidgetType::Button, true, selected, label.as_ref())
                 });
+                view_tab(ui, &response, selected);
                 let radius = look.radius.saturating_sub(2);
                 focus::hint(ui, &response, cell, focus::Ring::Edge { radius });
                 if selected {
@@ -406,7 +459,11 @@ pub fn header(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId, object_tab: TabI
                     });
                 }
             }
-            if let Some(short) = button {
+            let ui = header;
+            if view == ObjectView::Ddl {
+                let skin = (&look, &palette, locale);
+                super::ddl::header_wait(ui, reading, right, center, skin);
+            } else if let Some(short) = button {
                 let button = add_row(short);
                 let width = button.width(ui, &look);
                 let place =
diff --git a/src/ui/ddl.rs b/src/ui/ddl.rs
new file mode 100644
index 00000000..c385d5ab
--- /dev/null
+++ b/src/ui/ddl.rs
@@ -0,0 +1,589 @@
+//! The DDL view: a table's or a view's definition as read-only text, in
+//! the code face with the SQL editor's colours and line numbers, and where
+//! it comes from. The text is selected and copied as any text is. A read
+//! that failed says the database's error, and never shows part of a text.
+
+use std::borrow::Cow;
+use std::time::Duration;
+
+use egui::{Frame, Id, Margin, Rect, UiBuilder, pos2, vec2};
+use tabletist_db::ddl::Source;
+use tabletist_db::{Driver, ObjectKind};
+
+use crate::app::App;
+use crate::i18n::{Locale, gettext};
+use crate::keymap::Command;
+use crate::model::{Action, ConnTabId, ObjectTab, TabId};
+use crate::theme::{self, Icon, Look, Palette};
+use crate::typography::{Text, TextRole};
+use crate::ui::{focus, format, sql_text, states, widgets};
+
+/// The text's role, and its line numbers': the SQL editor's.
+fn role(look: &Look) -> TextRole {
+    TextRole::pick(look, TextRole::Code, TextRole::OCode)
+}
+
+/// The id of the text field of the DDL view of the tab `id`.
+pub fn text_id(tab: ConnTabId, id: TabId) -> Id {
+    Id::new(("ddl-text", tab.0, id.0))
+}
+
+/// Where the text comes from, as the view says it: built from the
+/// catalog, or as the database keeps it.
+fn source_words(source: Source, locale: Locale) -> Cow<'static, str> {
+    match source {
+        Source::Catalog => gettext(locale, "built from the catalog"),
+        Source::Stored => gettext(locale, "as stored in the database"),
+    }
+}
+
+/// The engine as the footer names it.
+fn engine(driver: Driver) -> &'static str {
+    match driver {
+        Driver::Postgres => "PostgreSQL",
+        Driver::MySql => "MySQL",
+        Driver::Sqlite => "SQLite",
+    }
+}
+
+/// What the text leaves out, where the app builds it: what PostgreSQL's
+/// catalog holds of the table besides its definition.
+fn left_out(driver: Driver, locale: Locale) -> Option<Cow<'static, str>> {
+    match driver {
+        Driver::Postgres => Some(gettext(
+            locale,
+            "Not included: owner, grants, sequences owned by the table, row-level policies",
+        )),
+        Driver::MySql | Driver::Sqlite => None,
+    }
+}
+
+/// "read just now", "read 12 s ago", "read 3 min ago", "read 2 h ago":
+/// how long ago the definition arrived.
+pub fn read_ago(waited: Duration, locale: Locale) -> String {
+    let seconds = waited.as_secs();
+    let ago = |count: u64, unit: &str| {
+        gettext(locale, "read {count} {unit} ago")
+            .replace("{count}", &count.to_string())
+            .replace("{unit}", unit)
+    };
+    match seconds {
+        0 => gettext(locale, "read just now").into_owned(),
+        1..60 => ago(seconds, "s"),
+        60..3600 => ago(seconds / 60, "min"),
+        _ => ago(seconds / 3600, "h"),
+    }
+}
+
+/// The terminal header's summary in the DDL view: "ddl · read-only ·
+/// built from the catalog · postgresql 17.2", as far as it is known.
+pub fn subtitle(object: &ObjectTab, locale: Locale) -> Vec<String> {
+    let mut parts = vec![
+        gettext(locale, "ddl").into_owned(),
+        gettext(locale, "read-only").into_owned(),
+    ];
+    if let Some(ddl) = &object.ddl.value {
+        parts.push(source_words(ddl.source(), locale).into_owned());
+        parts.push(ddl.server.to_lowercase());
+    }
+    parts
+}
+
+/// Whether the read has gone on long enough to say so: the header says it
+/// then, and the view shows where the text will be. A repaint is asked
+/// for until it has.
+fn waited_long(ui: &egui::Ui, waited: Option<Duration>) -> bool {
+    let Some(waited) = waited else {
+        return false;
+    };
+    if states::lasted(waited) {
+        return true;
+    }
+    ui.ctx().request_repaint_after(states::DELAY - waited);
+    false
+}
+
+/// How wide [`header_wait`] paints, for the header to keep the title
+/// clear of it: 0 while nothing is said.
+pub fn header_wait_width(ui: &egui::Ui, waited: Option<Duration>, skin: Skin) -> f32 {
+    let (look, palette, locale) = skin;
+    if !waited.is_some_and(states::lasted) {
+        return 0.0;
+    }
+    let text = gettext(locale, "Reading the definition…");
+    let said = Text::one(look, widgets::secondary(look), &text, palette.secondary);
+    widgets::measure(ui, said) + 8.0 + 14.0
+}
+
+/// The macOS header's end in the DDL view while the definition is read: a
+/// spinner and "Reading the definition…", ending at `right`. Its width,
+/// or 0 when nothing is said.
+pub fn header_wait(
+    ui: &egui::Ui,
+    waited: Option<Duration>,
+    right: f32,
+    center: f32,
+    skin: Skin,
+) -> f32 {
+    let (look, palette, locale) = skin;
+    if !waited_long(ui, waited) {
+        return 0.0;
+    }
+    let text = gettext(locale, "Reading the definition…");
+    let role = widgets::secondary(look);
+    let laid = Text::one(look, role, &text, palette.secondary).layout(ui.ctx());
+    let width = laid.paint_right(ui.painter(), right, center);
+    let left = right - width - 8.0 - 14.0;
+    let ring = Rect::from_center_size(pos2(left + 7.0, center), vec2(14.0, 14.0));
+    states::spinner(ui, ring, palette.accent, palette);
+    let named = Rect::from_min_max(pos2(left, center - 8.0), pos2(right, center + 8.0));
+    widgets::announce_polite(ui, named, &text);
+    width + 8.0 + 14.0
+}
+
+/// The look, the palette and the locale, as the view's parts take them.
+pub type Skin<'a> = (&'a Look, &'a Palette, Locale);
+
+pub fn show(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId, object_tab: TabId) {
+    let (look, palette, locale) = (app.look, app.palette, app.locale);
+    let skin = (&look, &palette, locale);
+    let reload = crate::ui::keys::written(ui.ctx(), &look, Command::Reload);
+    let Some(workspace) = app.workspace(tab) else {
+        return;
+    };
+    let Some(object) = workspace.object_tab(object_tab) else {
+        return;
+    };
+    let mut actions = Vec::new();
+    let area = ui.max_rect();
+    focus::region(ui, focus::Region::Grid, area);
+    // A view that holds nothing and waits for nothing reads its
+    // definition: one forgotten while its tab was behind (a new session, a
+    // Reload elsewhere) is read when the tab comes in front again.
+    if object.ddl.needs_load() && !object.ddl.asked {
+        actions.push(Action::RetryDdl { tab, object_tab });
+    }
+    if let Some(error) = object.ddl.shown_error() {
+        let failed = Failed {
+            error,
+            name: &object.object.name,
+            driver: workspace.driver,
+            kind: object.kind,
+        };
+        Frame::new()
+            .inner_margin(Margin::same(theme::PAD))
+            .show(ui, |ui| {
+                if failed.show(ui, &reload, skin) {
+                    actions.push(Action::RetryDdl { tab, object_tab });
+                }
+            });
+    } else if let Some(ddl) = &object.ddl.value {
+        let text = ddl.text();
+        let name = gettext(locale, "DDL of {name}")
+            .replace("{name}", &format::display_safe(&object.object.name));
+        let mut body = area;
+        if !look.terminal {
+            // The line over the text: where it comes from, and how long ago
+            // it was read, with the key that reads it again.
+            let strip = Rect::from_min_max(area.min, pos2(area.right(), area.top() + 44.0));
+            body.min.y = strip.bottom();
+            widgets::hline(
+                ui,
+                strip.x_range(),
+                strip.bottom() - 0.5,
+                palette.surface_hover,
+            );
+            let read = object.ddl_read.map(|at| at.elapsed());
+            if source_line(ui, strip, ddl.source(), read, &reload, skin) {
+                actions.push(Action::Refresh(tab));
+            }
+            // The age changes once a second, then once a minute.
+            let tick = match read {
+                Some(read) if read.as_secs() >= 60 => 60,
+                _ => 1,
+            };
+            ui.ctx().request_repaint_after(Duration::from_secs(tick));
+        }
+        let dialect = ddl.driver.dialect();
+        let id = text_id(tab, object_tab);
+        let field = Field {
+            id,
+            name: &name,
+            dialect,
+        };
+        field.show(ui, body, &text, skin);
+    } else if waited_long(ui, object.ddl.running_for()) {
+        let text = gettext(locale, "Reading the definition…");
+        if look.terminal {
+            // One line: a spinner, and what it waits for.
+            let text = text.to_lowercase();
+            let center = area.top() + f32::from(theme::PAD) + 8.0;
+            let left = area.left() + f32::from(theme::PAD);
+            let ring = Rect::from_center_size(pos2(left + 6.0, center), vec2(12.0, 12.0));
+            states::spinner(ui, ring, palette.accent, &palette);
+            let said = Text::one(&look, TextRole::OBody, &text, palette.dim);
+            let width = widgets::paint_text(ui, left + 20.0, center, said);
+            let named = Rect::from_min_size(pos2(left, center - 8.0), vec2(width + 20.0, 16.0));
+            widgets::announce_polite(ui, named, &text);
+        } else {
+            // The shape of the text without it: the header says what is
+            // read.
+            let lines = area.shrink2(vec2(10.0, 14.0));
+            grey_lines(ui, lines, &palette);
+        }
+    }
+    app.actions.extend(actions);
+}
+
+/// Lines of grey bars where the text will be: a gutter's number and the
+/// line, of the widths a definition's lines have.
+fn grey_lines(ui: &egui::Ui, rect: Rect, palette: &Palette) {
+    const SHARES: [f32; 8] = [0.45, 0.3, 0.55, 0.4, 0.6, 0.35, 0.5, 0.2];
+    let fill = palette.surface.gamma_multiply(0.7);
+    let pitch = 20.0;
+    let rows = ((rect.height() / pitch) as usize).min(SHARES.len());
+    for (row, share) in SHARES.iter().enumerate().take(rows) {
+        let center = rect.top() + pitch * (row as f32 + 0.5);
+        let number = Rect::from_min_size(pos2(rect.left() + 18.0, center - 5.0), vec2(14.0, 10.0));
+        let room = (rect.width() - 58.0).max(0.0);
+        let line = Rect::from_min_size(
+            pos2(rect.left() + 58.0, center - 5.0),
+            vec2(room * share, 10.0),
+        );
+        for bar in [number, line] {
+            ui.painter()
+                .rect_filled(bar, egui::CornerRadius::same(5), fill);
+        }
+    }
+}
+
+/// "Read-only · built from the catalog · read 2 s ago · Reload ⌘R" at the
+/// end of `strip`: Reload is a link. Returns whether it was clicked.
+fn source_line(
+    ui: &mut egui::Ui,
+    strip: Rect,
+    source: Source,
+    read: Option<Duration>,
+    reload: &crate::keymap::Written,
+    (look, palette, locale): Skin,
+) -> bool {
+    let side = 24.0;
+    let center = strip.center().y;
+    let role = widgets::secondary(look);
+    let mut words = vec![
+        gettext(locale, "Read-only").into_owned(),
+        source_words(source, locale).into_owned(),
+    ];
+    if let Some(read) = read {
+        words.push(read_ago(read, locale));
+    }
+    let said = format!("{} · ", words.join(" · "));
+    let label = gettext(locale, "Reload");
+    let link = widgets::ButtonSpec::new(&label)
+        .link()
+        .shortcut(reload)
+        .padding(4.0)
+        .role(role)
+        .shortcut_role(role);
+    let width = link.width(ui, look);
+    let at = Rect::from_min_size(
+        pos2(strip.right() - side - width, center - 12.0),
+        vec2(width, 24.0),
+    );
+    let clicked = link.show_at(ui, at, look, palette).clicked();
+    let text = Text::one(look, role, &said, palette.dim);
+    let laid = text.layout(ui.ctx());
+    let left = at.left() - laid.paint_right(ui.painter(), at.left(), center);
+    let named = Rect::from_min_max(pos2(left, center - 8.0), pos2(at.left(), center + 8.0));
+    widgets::announce(ui, named, &words.join(" · "));
+    clicked
+}
+
+/// The macOS footer of the DDL view: how many lines and whose, what the
+/// text leaves out, and how to copy part of it.
+pub fn footer(app: &App, ui: &mut egui::Ui, tab: ConnTabId, object_tab: TabId) {
+    let (look, palette, locale) = (app.look, app.palette, app.locale);
+    let Some(workspace) = app.workspace(tab) else {
+        return;
+    };
+    let Some(object) = workspace.object_tab(object_tab) else {
+        return;
+    };
+    let driver = workspace.driver;
+    let lines = object
+        .ddl
+        .value
+        .as_ref()
+        .filter(|_| object.ddl.shown_error().is_none())
+        .map(|ddl| ddl.text().lines().count());
+    egui::Panel::bottom(Id::new(("ddl-footer", tab.0, object_tab.0)))
+        .exact_size(36.0)
+        .resizable(false)
+        .show_separator_line(false)
+        .frame(Frame::new().fill(palette.panel))
+        .show(ui, |ui| {
+            let rect = ui.max_rect();
+            widgets::hline(ui, rect.x_range(), rect.top() + 0.5, palette.surface_hover);
+            let Some(lines) = lines else {
+                return;
+            };
+            let center = rect.center().y;
+            let role = widgets::secondary(&look);
+            let count = u32::try_from(lines).unwrap_or(u32::MAX);
+            let count = crate::i18n::ngettext(locale, "{n} line", "{n} lines", count)
+                .replace("{n}", &lines.to_string());
+            let count = format!("{count} · {}", engine(driver));
+            let mut x = rect.left() + 24.0;
+            x += widgets::paint_label(ui, x, center, Text::one(&look, role, &count, palette.dim));
+            if let Some(left_out) = left_out(driver, locale) {
+                let said = Text::one(&look, role, &left_out, palette.dim);
+                x += 12.0 + widgets::paint_label(ui, x + 12.0, center, said);
+            }
+            // Where there is room for it after the rest.
+            let hint = gettext(locale, "Select text to copy part of it");
+            let hint = || Text::one(&look, role, &hint, palette.dim);
+            let width = widgets::measure(ui, hint());
+            if x + 24.0 + width < rect.right() - 24.0 {
+                let right = rect.right() - 24.0;
+                widgets::paint_text_right(ui, right, center, hint());
+                let at = Rect::from_min_max(
+                    pos2(right - width, center - 8.0),
+                    pos2(right, center + 8.0),
+                );
+                widgets::announce(ui, at, &gettext(locale, "Select text to copy part of it"));
+            }
+        });
+}
+
+/// A read that failed: whose definition, the database's code and words,
+/// what it means for the view, and the ways on.
+struct Failed<'a> {
+    error: &'a tabletist_db::Error,
+    name: &'a str,
+    driver: Driver,
+    kind: ObjectKind,
+}
+
+impl Failed<'_> {
+    /// "1142 · SHOW command denied …": MySQL's number where it gives one,
+    /// else the code, then the database's words.
+    fn said(&self) -> String {
+        match self.error {
+            tabletist_db::Error::Query {
+                code,
+                message,
+                named,
+                ..
+            } => {
+                let code = named
+                    .number
+                    .map(|number| number.to_string())
+                    .or_else(|| code.clone());
+                let message = format::capped(message);
+                match code {
+                    Some(code) => format!("{code} · {message}"),
+                    None => message.into_owned(),
+                }
+            }
+            other => format::capped(&other.to_string()).into_owned(),
+        }
+    }
+
+    /// Whether the database refused the read to the connection's user:
+    /// MySQL's numbers for a privilege it lacks, PostgreSQL's
+    /// `insufficient_privilege`. SQLite has no users.
+    fn refused(&self) -> bool {
+        let tabletist_db::Error::Query { code, named, .. } = self.error else {
+            return false;
+        };
+        match self.driver {
+            Driver::MySql => matches!(named.number, Some(1044 | 1142 | 1143 | 1227)),
+            Driver::Postgres => code.as_deref() == Some("42501"),
+            Driver::Sqlite => false,
+        }
+    }
+
+    /// What the error means for the view, where the database refused the
+    /// read: what the user may not do, and that the rest of the tab still
+    /// works. Nothing for any other error.
+    fn meaning(&self, locale: Locale) -> Option<String> {
+        if !self.refused() {
+            return None;
+        }
+        let words = match (self.driver, self.kind) {
+            (Driver::MySql, ObjectKind::View) => {
+                "The connection's user can't run SHOW CREATE VIEW here. Rows and Structure still work."
+            }
+            (Driver::MySql, ObjectKind::Table | ObjectKind::MaterializedView) => {
+                "The connection's user can't run SHOW CREATE TABLE here. Rows and Structure still work."
+            }
+            (Driver::Postgres | Driver::Sqlite, _) => {
+                "The connection's user can't read this part of the catalog. Rows and Structure still work."
+            }
+        };
+        Some(gettext(locale, words).into_owned())
+    }
+
+    /// All of it, for the clipboard: the code and the words, and what
+    /// else the database said, each under its name.
+    fn details(&self, locale: Locale) -> String {
+        let mut details = self.said();
+        if let tabletist_db::Error::Query { detail, hint, .. } = self.error {
+            for (label, said) in [("Detail", detail), ("Hint", hint)] {
+                if let Some(said) = said {
+                    details.push_str(&format!("\n{}: {said}", gettext(locale, label)));
+                }
+            }
+        }
+        details
+    }
+
+    /// The card, and under it Try again and Copy error (the terminal's
+    /// retry is its key). Returns whether Try again was asked for.
+    fn show(&self, ui: &mut egui::Ui, reload: &crate::keymap::Written, skin: Skin) -> bool {
+        let (look, palette, locale) = skin;
+        let title = gettext(locale, "Couldn't read the definition of {name}")
+            .replace("{name}", &format::display_safe(self.name));
+        let title = look.label(&title);
+        let said = self.said();
+        let mut retried = false;
+        ui.scope(|ui| {
+            ui.spacing_mut().item_spacing = vec2(8.0, 10.0);
+            let card = states::Card {
+                tone: states::Tone::Danger,
+                icon: Icon::CircleAlert,
+                title: &title,
+                text: &said,
+            };
+            states::card(ui, &card, look, palette);
+            if let Some(meaning) = self.meaning(locale) {
+                let width = ui.available_width();
+                Text::one(look, widgets::body(look), &meaning, palette.secondary)
+                    .wrap(width)
+                    .layout(ui.ctx())
+                    .label(ui);
+            }
+            // Said once, as it appears: the title and the database's words.
+            let rect = ui.min_rect();
+            widgets::announce_polite(ui, rect, &format!("{title}. {said}"));
+            let height = states::button_height(look);
+            ui.horizontal(|ui| {
+                let again = look.label(&gettext(locale, "Try again"));
+                let again = if look.terminal {
+                    states::key_button(&again, reload, look)
+                } else {
+                    states::button(&again, look)
+                };
+                if again
+                    .label("Try again")
+                    .show(ui, height, look, palette)
+                    .clicked()
+                {
+                    retried = true;
+                }
+                let copy = look.label(&gettext(locale, "Copy error"));
+                let copy = states::button(&copy, look).label("Copy error").quiet();
+                if copy.show(ui, height, look, palette).clicked() {
+                    ui.ctx().copy_text(self.details(locale));
+                }
+            });
+        });
+        retried
+    }
+}
+
+/// The text's field: its id, its name for a screen reader, and the
+/// dialect it is coloured in.
+struct Field<'a> {
+    id: Id,
+    name: &'a str,
+    dialect: tabletist_db::Dialect,
+}
+
+impl Field<'_> {
+    /// `text` in `rect`, beside a gutter of its line numbers: read-only,
+    /// selected and copied as any text, scrolled both ways (its lines do
+    /// not wrap).
+    fn show(&self, ui: &mut egui::Ui, rect: Rect, text: &str, (look, palette, _): Skin) {
+        let role = role(look);
+        let lines = text.lines().count().max(1);
+        let digits = lines.ilog10() as usize + 1;
+        let digit = role.width(ui.ctx(), look.faces, "0");
+        // macOS: 44 pt of numbers, 14 of it after them, from 10 in. The
+        // terminal: from the pane's edge.
+        let inset = if look.terminal { 0.0 } else { 10.0 };
+        let gutter = (digits as f32 * digit + 14.0 + 8.0).ceil().max(44.0);
+        let top = if look.terminal { 8.0 } else { 14.0 };
+        let text_pane =
+            Rect::from_min_max(pos2(rect.left() + inset + gutter, rect.top()), rect.max);
+        let mut child = ui.new_child(UiBuilder::new().id_salt(self.id).max_rect(text_pane));
+        child.set_clip_rect(text_pane.intersect(ui.clip_rect()));
+        // The keyboard that comes by F6 or Tab starts at the top: egui
+        // puts the cursor at the end, from where nothing selects down. A
+        // press puts it where it lands.
+        let had = self.id.with("had-keyboard");
+        let has = ui.memory(|memory| memory.has_focus(self.id));
+        let came = has && !ui.data(|data| data.get_temp::<bool>(had)).unwrap_or(false);
+        if came && !ui.input(|input| input.pointer.any_pressed()) {
+            let mut state = egui::TextEdit::load_state(ui.ctx(), self.id).unwrap_or_default();
+            let top = egui::text::CCursorRange::one(egui::text::CCursor::new(0));
+            state.cursor.set_char_range(Some(top));
+            egui::TextEdit::store_state(ui.ctx(), self.id, state);
+        }
+        ui.data_mut(|data| data.insert_temp(had, has));
+        let font = role.font_id(look.faces);
+        let mut layouter = sql_text::layouter(look, palette, self.dialect, self.id, false);
+        let scrolled = egui::ScrollArea::both()
+            .id_salt(("ddl-scroll", self.id))
+            .auto_shrink([false, false])
+            .show(&mut child, |ui| {
+                // The text the field reads and cannot change.
+                let mut fixed: &str = text;
+                egui::TextEdit::multiline(&mut fixed)
+                    .id(self.id)
+                    .font(font)
+                    .frame(Frame::new().inner_margin(Margin {
+                        left: 0,
+                        right: 16,
+                        top: top as i8,
+                        bottom: 16,
+                    }))
+                    .desired_width(f32::INFINITY)
+                    .desired_rows(lines)
+                    .layouter(&mut layouter)
+                    .show(ui)
+            });
+        let output = scrolled.inner;
+        ui.ctx().accesskit_node_builder(self.id, |node| {
+            node.set_label(self.name);
+            node.set_read_only();
+        });
+        // The keyboard comes here with F6, as it goes to the rows. A field
+        // that cannot be typed into draws no caret: a ring inside the
+        // text's area, its numbers with it, says where the keyboard is.
+        focus::claim(ui, focus::Region::Grid, &output.response);
+        let ring = focus::Ring::Inset { radius: 0 };
+        focus::hint(ui, &output.response, rect, ring);
+        // The numbers of the lines in sight: one row each, as nothing
+        // wraps.
+        let clip = rect.intersect(ui.clip_rect());
+        let rows = &output.galley.rows;
+        let origin = output.galley_pos;
+        let first = rows.partition_point(|row| origin.y + row.max_y() < clip.top());
+        let right = rect.left() + inset + gutter - 14.0;
+        let color = if look.terminal {
+            palette.dim
+        } else {
+            palette.faint
+        };
+        let painter = ui.painter().with_clip_rect(clip);
+        for (index, row) in rows.iter().enumerate().skip(first) {
+            let top = origin.y + row.min_y();
+            if top > clip.bottom() {
+                break;
+            }
+            let number = Text::one(look, role, &(index + 1).to_string(), color).layout(ui.ctx());
+            number.paint_right(&painter, right, top + row.height() / 2.0);
+        }
+    }
+}
diff --git a/src/ui/keys.rs b/src/ui/keys.rs
index 7066f4c6..dcae9aba 100644
--- a/src/ui/keys.rs
+++ b/src/ui/keys.rs
@@ -444,6 +444,12 @@ pub fn handle(app: &mut App, ctx: &egui::Context) {
     // An editor that just opened takes the keyboard when its field is
     // first drawn, later in this frame: the keys are its own already, and
     // none of them is the grid's (Space, an arrow). So with the prompt.
+    // The DDL view's text, which cannot be typed into, has the keyboard:
+    // its keys are the field's, but for Reload's letter (below).
+    let in_ddl_text = object.is_some_and(|(tab, id)| {
+        let text = crate::ui::ddl::text_id(tab, id);
+        ctx.memory(|memory| memory.has_focus(text))
+    });
     let editing = ctx.text_edit_focused() || open || field_asked || prompt;
     // What the prompt refused is said until the next key, whatever the key
     // goes on to do. A fresh press: the Enter that ran the line may still
@@ -495,11 +501,19 @@ pub fn handle(app: &mut App, ctx: &egui::Context) {
             .and_then(|workspace| workspace.object_tab(id))
             .is_some_and(|object| object.view == crate::model::ObjectView::Structure)
     });
+    // The DDL view of the table on screen: its text has the arrows while
+    // it has the keyboard, and none goes to the tree from it.
+    let ddl = object.is_some_and(|(tab, id)| {
+        app.workspace(tab)
+            .and_then(|workspace| workspace.object_tab(id))
+            .is_some_and(|object| object.view == crate::model::ObjectView::Ddl)
+    });
     // The History tab's list is somewhere else: its arrows are its own,
     // unless the person went to the tree.
     let tree_arrows = !editing
         && app.workspace(active).is_some_and(|workspace| {
-            let nowhere_else = !any_grid && structure.is_none() && !workspace.history_in_front();
+            let nowhere_else =
+                !any_grid && structure.is_none() && !ddl && !workspace.history_in_front();
             workspace.pane == crate::model::Pane::Tree || nowhere_else
         });
     // That view has the keys: the tree does not, nor a field, a button or
@@ -1254,6 +1268,16 @@ pub fn handle(app: &mut App, ctx: &egui::Context) {
         // The keys are a field's: a first key that waited is not the
         // first of what is typed once they are the grid's again.
         forget_pending(ctx);
+        // Nothing is typed into the DDL view's text: Reload's letter
+        // reads it again from there. (Its chord is read in every field.)
+        if in_ddl_text && app.dialog.is_none() {
+            for char in typed_chars(ctx) {
+                if window_letter(app, scopes, char) == Some(Command::Reload) {
+                    take_char(ctx, char);
+                    actions.push(Action::Refresh(active));
+                }
+            }
+        }
     }
     // Ahead of what the key asked for, which may be the prompt again. Put
     // there only now: a letter that edits acts only where nothing else was
diff --git a/src/ui/workspace.rs b/src/ui/workspace.rs
index 482f6e35..89a2de5c 100644
--- a/src/ui/workspace.rs
+++ b/src/ui/workspace.rs
@@ -111,7 +111,10 @@ pub fn show(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId) {
                         super::data_view::toolbar(app, ui, tab, object_tab);
                     }
                     if !look.terminal {
-                        super::data_view::footer(app, ui, tab, object_tab);
+                        match view {
+                            Some(ObjectView::Ddl) => super::ddl::footer(app, ui, tab, object_tab),
+                            _ => super::data_view::footer(app, ui, tab, object_tab),
+                        }
                         // Bottom panels stack upwards: the bar of pending
                         // changes stands on the footer.
                         super::pending_bar::show(app, ui, tab, object_tab);
@@ -125,6 +128,7 @@ pub fn show(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId) {
                             Some(ObjectView::Structure) => {
                                 super::structure::show(app, ui, tab, object_tab)
                             }
+                            Some(ObjectView::Ddl) => super::ddl::show(app, ui, tab, object_tab),
                             _ => {
                                 if super::filter_bar::is_open(app, tab, object_tab) {
                                     let palette = app.palette;
@@ -2173,6 +2177,12 @@ fn status_line(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId) {
     let (column_key, use_key) = (key(Command::MoveInStructure), key(Command::UseAsLabel));
     let (chooser_key, automatic_key) =
         (key(Command::OpenLabelChooser), key(Command::LabelAutomatic));
+    // The DDL view of the table in front: its line names that view's keys,
+    // and says how long the text is in the place of the rows' range.
+    let ddl = workspace
+        .active_object_tab()
+        .filter(|object| object.view == ObjectView::Ddl);
+    let reload_key = key(Command::Reload);
     // The completion list's own key, while it is open with a row to put
     // in. One whose row is about to be inserted is not open any more: the
     // mode line is drawn before the editor inserts it.
@@ -2180,7 +2190,21 @@ fn status_line(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId) {
         .active_sql_tab()
         .and_then(|sql| sql.completion.as_ref())
         .is_some_and(|list| !list.accept && !list.candidates.is_empty());
+    let lines = ddl.and_then(|object| {
+        let shown = object
+            .ddl
+            .value
+            .as_ref()
+            .filter(|_| object.ddl.shown_error().is_none());
+        let lines = shown?.text().lines().count();
+        let count = u32::try_from(lines).unwrap_or(u32::MAX);
+        Some(
+            crate::i18n::ngettext(locale, "{n} line", "{n} lines", count)
+                .replace("{n}", &lines.to_string()),
+        )
+    });
     let summary = editor
+        .or(lines)
         .or_else(|| {
             let object = workspace.active_object_tab()?;
             let page = object.page()?;
@@ -2356,6 +2380,20 @@ fn status_line(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId) {
             if editing.pending.is_some() || editing.added.is_some() {
                 structure_hints.push((&review_key, &*review, true));
             }
+            // The DDL view's: its text is read again, and the rows' keys
+            // do nothing on it. What is pending is saved and reviewed from
+            // here too.
+            let ddl_words = ["reload", "tables"].map(|word| gettext(locale, word));
+            let mut ddl_hints: Vec<widgets::Hint<'_>> = vec![
+                (&reload_key, &*ddl_words[0], true),
+                (&sidebar_key, &*ddl_words[1], true),
+            ];
+            if !read_only {
+                ddl_hints.push((&save_key, &*write, true));
+            }
+            if editing.pending.is_some() || editing.added.is_some() {
+                ddl_hints.push((&review_key, &*review, true));
+            }
             let (copy_word, on_word) = (gettext(locale, "copy"), gettext(locale, "turn on"));
             let allow_word = gettext(locale, "allow agents");
             let settings_word = gettext(locale, "settings");
@@ -2496,6 +2534,8 @@ fn status_line(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId) {
                 &editor_hints
             } else if structure.is_some() {
                 &structure_hints
+            } else if ddl.is_some() {
+                &ddl_hints
             } else {
                 &table_hints
             };
````

- [ ] **Step 4: Run the tests to see them pass**

```bash
~/.cargo/bin/cargo test --locked --lib ui::
```

Expected: all pass, `ui::ddl_tests` with 23.

- [ ] **Step 5: Run the checks** (above). Expected: every suite passes, the app's library with 3328 tests.

- [ ] **Step 6: Look at it** (not a test, nothing committed)

Append a throwaway `#[test] #[ignore]` scene to `src/shots.rs` built on `both(name, |harness| ..)`: `workspace(harness)`, `SetView` to `ObjectView::Ddl`, and `harness.answer_ddl(Ok(..))` with a PostgreSQL `Ddl` of `book_images` (the shots' Bookshop table that `workspace` opens); one scene answering a MySQL `Error::Query` with `Named { number: Some(1142), .. }`; one with the read's `started` a second back. Render with `~/.cargo/bin/cargo test --locked --features shots --lib shots::<scene> -- --ignored`, compare `target/shots/*.png` with the three boards by eye, then remove the scene. The PNGs stay local.

- [ ] **Step 7: Commit**

```bash
git add src/ui docs/_guide
git commit -m "Show a table's DDL as a third view beside Rows and Structure"
```

---

## The pull request

Title: "DDL view, run 1: read and show". The body, without em dashes in what it quotes of the UI:

- **Summary**, quoting "There is no way to see the DDL.", and that this is run 1 of three.
- **How PostgreSQL's text is built** (the spec's Database section) and what is left out: owner, grants, row-level policies, sequences owned by the table; also tablespaces and the access method; a foreign table is not supported.
- **The `Driver` match**: `Connection::ddl` names every engine, through `Adapter::ddl`, which has no default; `tests/ddl.rs` reads `book_covers` on each `Driver`.
- **Where the cache lives**: on the tab (`ObjectTab::ddl`), as its structure; what reads, cancels and forgets it.
- **Keys**: none added in this run; Reload's letter now works with the keyboard in the text. The keymap was checked for run 2's keys, and decided with Igor: `s` will be Structure from every view (Replace cell keeps `cc`), and ⌃⌘1/2/3 will be Alt+1/2/3 on Windows.
- **Open questions and defaults**: Rows, not Data (built here); `r` for the rows on Omarchy (run 2).
- **Where the build differs from the boards**: the table above.
- **Not testable here**: which statements the server ran (no `performance_schema` for the tests' user, no PostgreSQL statement log); what is tested instead.
- **A dev server**: once this branch loaded the Bookshop, main's fixture cannot drop `books` (the new tables point at it); drop PostgreSQL's `bookshop` database to go back.
- No screenshots (or only blurred ones).
