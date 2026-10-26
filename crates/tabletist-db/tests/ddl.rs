//! The DDL view's text, on the Bookshop: what each engine's `ddl` reads of
//! `book_covers` and `in_stock`, held to the text files under
//! `tests/ddl/`; that PostgreSQL's text builds the same catalog again; that
//! MySQL's is the server's own, and SQLite's what `sqlite_master` keeps.
//!
//! The PostgreSQL and MySQL tests need the servers (see AGENTS.md) and
//! print "skipped" without them.

#![allow(clippy::unwrap_used)]

use tabletist_db::ddl::{PartKind, Source};
use tabletist_db::{
    Access, ConnectSpec, Connection, Ddl, Driver, Error, HostKeys, ObjectRef, Secrets, TlsMode,
};

/// The text file `tests/ddl/<name>.sql`.
fn expected(name: &str) -> String {
    let path = format!("{}/tests/ddl/{name}.sql", env!("CARGO_MANIFEST_DIR"));
    std::fs::read_to_string(&path).unwrap_or_else(|error| panic!("{path}: {error}"))
}

/// `text` without what the server it was read on varies: PostgreSQL's
/// version in the first line, MySQL's `AUTO_INCREMENT=` and the user a
/// `DEFINER` names. A text file ends in a line break, which the text has
/// not.
fn steady(text: &str) -> String {
    let lines: Vec<String> = text
        .lines()
        .map(|line| {
            let line = match line.rsplit_once(" · PostgreSQL ") {
                Some((start, _)) if line.starts_with("-- ") => format!("{start} · PostgreSQL"),
                _ => line.to_owned(),
            };
            let line = cut_word(&line, " AUTO_INCREMENT=");
            match line.find("DEFINER=`") {
                Some(at) => {
                    let rest = &line[at..];
                    // DEFINER=`user`@`host`
                    let end = rest
                        .match_indices('`')
                        .nth(3)
                        .map_or(rest.len(), |(at, _)| at + 1);
                    format!("{}DEFINER=?{}", &line[..at], &rest[end..])
                }
                None => line,
            }
        })
        .collect();
    format!("{}\n", lines.join("\n"))
}

/// `line` without `word` and the value after it, up to the next space.
fn cut_word(line: &str, word: &str) -> String {
    match line.find(word) {
        Some(at) => {
            let rest = &line[at + word.len()..];
            let end = rest.find(' ').unwrap_or(rest.len());
            format!("{}{}", &line[..at], &rest[end..])
        }
        None => line.to_owned(),
    }
}

/// How many statements each part holds, by kind.
fn counts(ddl: &Ddl) -> Vec<(PartKind, usize)> {
    ddl.parts
        .iter()
        .map(|part| (part.kind, part.statements.len()))
        .collect()
}

mod sqlite {
    use super::*;

    /// A read-only session on a new file holding the Bookshop, and
    /// `more` run on it first.
    async fn bookshop(more: &str) -> (Connection, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("bookshop.db");
        tabletist_db::fixtures::write_sqlite_bookshop(&path).unwrap();
        rusqlite::Connection::open(&path)
            .unwrap()
            .execute_batch(more)
            .unwrap();
        let connection = Connection::connect(&ConnectSpec::sqlite(&path), &Secrets::default())
            .await
            .unwrap();
        (connection, dir)
    }

    pub(super) async fn ddl(name: &str) -> Ddl {
        let (connection, _dir) = bookshop("").await;
        connection.ddl(&ObjectRef::new("main", name)).await.unwrap()
    }

    #[tokio::test]
    async fn book_covers_is_its_stored_text_then_its_indexes_and_triggers() {
        let ddl = ddl("book_covers").await;
        assert_eq!(steady(&ddl.text()), expected("sqlite_book_covers"));
        assert_eq!(ddl.source(), Source::Stored);
        assert_eq!(
            counts(&ddl),
            [
                (PartKind::Object, 1),
                (PartKind::Indexes, 2),
                (PartKind::Triggers, 2),
            ]
        );
    }

    #[tokio::test]
    async fn in_stock_is_its_stored_text() {
        let ddl = ddl("in_stock").await;
        assert_eq!(steady(&ddl.text()), expected("sqlite_in_stock"));
        assert_eq!(counts(&ddl), [(PartKind::Object, 1)]);
    }

    /// An index SQLite made for a key keeps no text, and is left out; the
    /// indexes come before the triggers, each by name, and a statement
    /// stored without its `;` gets one.
    #[tokio::test]
    async fn automatic_indexes_are_left_out_and_the_rest_comes_in_order() {
        let (connection, _dir) = bookshop(
            "CREATE TABLE shelves (code TEXT UNIQUE, floor INTEGER, PRIMARY KEY (code, floor));
             CREATE TRIGGER shelves_b AFTER INSERT ON shelves BEGIN SELECT 1; END;
             CREATE INDEX shelves_z ON shelves (floor);
             CREATE TRIGGER shelves_a AFTER DELETE ON shelves BEGIN SELECT 1; END;
             CREATE INDEX shelves_m ON shelves (code, floor);",
        )
        .await;
        let ddl = connection
            .ddl(&ObjectRef::new("main", "shelves"))
            .await
            .unwrap();
        assert_eq!(
            ddl.text(),
            "CREATE TABLE shelves (code TEXT UNIQUE, floor INTEGER, PRIMARY KEY (code, floor));\n\
             \n\
             CREATE INDEX shelves_m ON shelves (code, floor);\n\
             \n\
             CREATE INDEX shelves_z ON shelves (floor);\n\
             \n\
             CREATE TRIGGER shelves_a AFTER DELETE ON shelves BEGIN SELECT 1; END;\n\
             \n\
             CREATE TRIGGER shelves_b AFTER INSERT ON shelves BEGIN SELECT 1; END;"
        );
        assert!(!ddl.text().contains("autoindex"));
    }

    /// A trigger keeps the table's name as it was written in it: one
    /// written in another case is the table's all the same.
    #[tokio::test]
    async fn a_trigger_on_the_table_in_another_case_is_the_tables() {
        let (connection, _dir) = bookshop(
            "CREATE TABLE shelves (code TEXT);
             CREATE TRIGGER shelves_upper AFTER INSERT ON SHELVES BEGIN SELECT 1; END;",
        )
        .await;
        let ddl = connection
            .ddl(&ObjectRef::new("main", "shelves"))
            .await
            .unwrap();
        assert_eq!(
            counts(&ddl),
            [(PartKind::Object, 1), (PartKind::Triggers, 1)]
        );
    }

    #[tokio::test]
    async fn a_name_that_is_no_table_or_view_is_an_error() {
        let (connection, _dir) = bookshop("").await;
        let error = connection
            .ddl(&ObjectRef::new("main", "book_covers_book_id"))
            .await
            .unwrap_err();
        assert_eq!(
            error.to_string(),
            "no such table or view: book_covers_book_id"
        );
    }
}

mod postgres {
    use super::*;
    use tokio::sync::OnceCell;

    static BOOKSHOP: OnceCell<()> = OnceCell::const_new();

    fn url() -> Option<String> {
        let url = std::env::var("TABLETIST_TEST_PG_URL").ok();
        url.filter(|url| !url.trim().is_empty())
    }

    /// A session that may do anything, on the URL's database or on
    /// `database`.
    async fn admin(url: &str, database: Option<&str>) -> tokio_postgres::Client {
        let mut config: tokio_postgres::Config = url.parse().unwrap();
        config.ssl_mode(tokio_postgres::config::SslMode::Disable);
        if let Some(database) = database {
            config.dbname(database);
        }
        let (client, session) = config.connect(tokio_postgres::NoTls).await.unwrap();
        tokio::spawn(session);
        client
    }

    /// `database`, made new and empty.
    async fn fresh(url: &str, database: &str) -> tokio_postgres::Client {
        let server = admin(url, None).await;
        server
            .batch_execute(&format!("DROP DATABASE IF EXISTS {database} WITH (FORCE)"))
            .await
            .unwrap();
        server
            .batch_execute(&format!("CREATE DATABASE {database}"))
            .await
            .unwrap();
        admin(url, Some(database)).await
    }

    /// The Bookshop, in its database of its own, `bookshop`, as the agent
    /// tests have it.
    async fn load(url: &str) {
        BOOKSHOP
            .get_or_init(|| async {
                let server = admin(url, None).await;
                let made = server
                    .query_opt("SELECT 1 FROM pg_database WHERE datname = 'bookshop'", &[])
                    .await
                    .unwrap()
                    .is_some();
                if !made {
                    server
                        .batch_execute("CREATE DATABASE bookshop")
                        .await
                        .unwrap();
                }
                admin(url, Some("bookshop"))
                    .await
                    .batch_execute(tabletist_db::fixtures::BOOKSHOP_POSTGRES_SQL)
                    .await
                    .unwrap();
            })
            .await;
    }

    /// A session of the app's on `database`, or `None` (test skipped).
    async fn connect(database: &str, access: Access) -> Option<(Connection, String)> {
        let Some(url) = url() else {
            eprintln!("skipped: TABLETIST_TEST_PG_URL is not set");
            return None;
        };
        load(&url).await;
        let (mut spec, secrets) = ConnectSpec::from_url(&url).unwrap();
        spec.tls = TlsMode::Disable;
        spec.database = database.into();
        let connection = Connection::connect_with(&spec, &secrets, &HostKeys::default(), access)
            .await
            .unwrap();
        Some((connection, url))
    }

    pub(super) async fn bookshop() -> Option<Connection> {
        connect("bookshop", Access::ReadOnly)
            .await
            .map(|(connection, _)| connection)
    }

    async fn ddl(connection: &Connection, name: &str) -> Ddl {
        connection
            .ddl(&ObjectRef::new("public", name))
            .await
            .unwrap()
    }

    #[tokio::test]
    async fn book_covers_is_built_from_the_catalog() {
        let Some(connection) = bookshop().await else {
            return;
        };
        let ddl = ddl(&connection, "book_covers").await;
        assert_eq!(steady(&ddl.text()), expected("postgres_book_covers"));
        assert_eq!(ddl.source(), Source::Catalog);
        assert_eq!(
            counts(&ddl),
            [
                (PartKind::Object, 1),
                (PartKind::Indexes, 2),
                (PartKind::Triggers, 2),
                (PartKind::Comments, 2),
            ]
        );
    }

    #[tokio::test]
    async fn in_stock_is_its_definition_and_its_comment() {
        let Some(connection) = bookshop().await else {
            return;
        };
        let ddl = ddl(&connection, "in_stock").await;
        assert_eq!(steady(&ddl.text()), expected("postgres_in_stock"));
        assert_eq!(
            counts(&ddl),
            [(PartKind::Object, 1), (PartKind::Comments, 1)]
        );
    }

    /// Every name has its schema, so the text runs whatever the search
    /// path; a comment's quote is doubled.
    #[tokio::test]
    async fn names_have_their_schema_and_quotes_are_doubled() {
        let Some(connection) = bookshop().await else {
            return;
        };
        let text = ddl(&connection, "book_covers").await.text();
        for said in [
            "CREATE TABLE public.book_covers (",
            "REFERENCES public.books(id)",
            "ON public.book_covers USING btree (book_id)",
            "EXECUTE FUNCTION public.book_covers_check()",
            "COMMENT ON COLUMN public.book_covers.kind IS \
             'front or back, as the shop''s scanner names it';",
        ] {
            assert!(text.contains(said), "{said} in\n{text}");
        }
        let text = ddl(&connection, "in_stock").await.text();
        assert!(text.contains("FROM public.books b"), "{text}");
        assert!(
            text.contains(
                "COMMENT ON VIEW public.in_stock IS 'Books in print, with their publisher''s name';"
            ),
            "{text}"
        );
    }

    /// What only PostgreSQL's text is built for, each in the line it is
    /// written in.
    #[tokio::test]
    async fn what_only_postgres_has_is_written() {
        let Some(connection) = bookshop().await else {
            return;
        };
        let cases = [
            (
                "price_changes",
                "    id bigint GENERATED ALWAYS AS IDENTITY NOT NULL,",
            ),
            (
                "price_changes",
                "    change numeric(8,2) GENERATED ALWAYS AS ((new_price - old_price)) STORED,",
            ),
            (
                "price_changes",
                "    changed_on date DEFAULT CURRENT_DATE NOT NULL,",
            ),
            ("stock_moves", "PARTITION BY RANGE (moved_on);"),
            (
                "stock_moves_2026",
                "CREATE TABLE public.stock_moves_2026 PARTITION OF public.stock_moves",
            ),
            (
                "stock_moves_2026",
                "FOR VALUES FROM ('2026-01-01') TO ('2027-01-01');",
            ),
            (
                "basket_lines",
                "CREATE UNLOGGED TABLE public.basket_lines (",
            ),
            (
                "basket_lines",
                "    CONSTRAINT basket_lines_quantity_check CHECK ((quantity > 0)),",
            ),
            (
                "shelf_bookings",
                "    shelf text COLLATE pg_catalog.\"C\" NOT NULL,",
            ),
            (
                "shelf_bookings",
                "    CONSTRAINT shelf_bookings_during_excl EXCLUDE USING gist (during WITH &&)",
            ),
            (
                "book_covers",
                "ALTER TABLE public.book_covers DISABLE TRIGGER book_covers_audit;",
            ),
            (
                "publisher_totals",
                "CREATE MATERIALIZED VIEW public.publisher_totals AS",
            ),
            ("publisher_totals", "WITH NO DATA;"),
            (
                "publisher_totals",
                "CREATE UNIQUE INDEX publisher_totals_publisher_id \
                 ON public.publisher_totals USING btree (publisher_id);",
            ),
        ];
        for (name, line) in cases {
            let text = ddl(&connection, name).await.text();
            assert!(text.lines().any(|said| said == line), "{line} in\n{text}");
        }
        // A partition lists none of the columns its parent gives it.
        let text = ddl(&connection, "stock_moves_2026").await.text();
        assert!(!text.contains("book_id"), "{text}");
    }

    /// The tables, views and materialized views of the Bookshop, in an
    /// order each can be made in: what it uses before it.
    const MADE_IN_ORDER: &[&str] = &[
        "publishers",
        "authors",
        "books",
        "book_covers",
        "customers",
        "orders",
        "order_items",
        "stock_moves",
        "stock_moves_2026",
        "price_changes",
        "basket_lines",
        "shelf_bookings",
        "recent_orders",
        "customer_names",
        "customer_tally",
        "customer_sheet",
        "in_stock",
        "publisher_totals",
    ];

    /// What the catalog says of a relation, read on its own, apart from
    /// the DDL: its kind, how it is logged and its comment; each column
    /// with its type, NOT NULL, default, identity, what generates it, its
    /// collation and its comment; each constraint, index and trigger, the
    /// trigger's enabled state with it; a view's query, and a partitioned
    /// table's key and a partition's bound.
    async fn catalog_of(client: &tokio_postgres::Client, name: &str) -> Vec<String> {
        let sql = "\
            SELECT 'relation ' || c.relkind::text || ' ' || c.relpersistence::text || ' ' \
                   || COALESCE(obj_description(c.oid, 'pg_class'), '-') || ' ' \
                   || COALESCE(pg_get_viewdef(c.oid), '-') || ' ' \
                   || COALESCE(pg_get_partkeydef(c.oid), '-') || ' ' \
                   || COALESCE(pg_get_expr(c.relpartbound, c.oid), '-') \
            FROM pg_class c WHERE c.oid = $1::text::regclass \
            UNION ALL \
            SELECT 'column ' || a.attnum || ' ' || a.attname || ' ' \
                   || format_type(a.atttypid, a.atttypmod) || ' ' || a.attnotnull || ' ' \
                   || COALESCE(pg_get_expr(d.adbin, d.adrelid), '-') || ' ' \
                   || a.attidentity::text || '/' || a.attgenerated::text || ' ' \
                   || COALESCE(co.collname::text, '-') || ' ' \
                   || COALESCE(col_description(a.attrelid, a.attnum), '-') \
            FROM pg_attribute a \
            LEFT JOIN pg_attrdef d ON d.adrelid = a.attrelid AND d.adnum = a.attnum \
            LEFT JOIN pg_collation co ON co.oid = a.attcollation \
            WHERE a.attrelid = $1::text::regclass AND a.attnum > 0 AND NOT a.attisdropped \
            UNION ALL \
            SELECT 'constraint ' || conname || ' ' || pg_get_constraintdef(oid) || ' ' \
                   || convalidated::text \
            FROM pg_constraint WHERE conrelid = $1::text::regclass \
            UNION ALL \
            SELECT 'index ' || pg_get_indexdef(indexrelid) \
            FROM pg_index WHERE indrelid = $1::text::regclass \
            UNION ALL \
            SELECT 'trigger ' || pg_get_triggerdef(oid) || ' ' || tgenabled::text \
            FROM pg_trigger WHERE tgrelid = $1::text::regclass AND NOT tgisinternal \
            ORDER BY 1";
        let name = format!("public.{name}");
        client
            .query(sql, &[&name])
            .await
            .unwrap()
            .iter()
            .map(|row| row.get(0))
            .collect()
    }

    /// The DDL of every table and view of the Bookshop, run into a new
    /// database, makes a catalog that says what the Bookshop's says. The
    /// functions the triggers and a view call are made first: the DDL
    /// leaves them out.
    #[tokio::test]
    async fn the_text_builds_the_same_catalog_again() {
        let Some((connection, url)) = connect("bookshop", Access::ReadOnly).await else {
            return;
        };
        let source = admin(&url, Some("bookshop")).await;
        // Every one, and only those: a relation the fixture gains is
        // made here too.
        let listed: Vec<String> = source
            .query(
                "SELECT c.relname::text FROM pg_class c \
                 JOIN pg_namespace n ON n.oid = c.relnamespace \
                 WHERE n.nspname = 'public' AND c.relkind IN ('r', 'p', 'v', 'm') \
                 ORDER BY 1",
                &[],
            )
            .await
            .unwrap()
            .iter()
            .map(|row| row.get(0))
            .collect();
        let mut made: Vec<&str> = MADE_IN_ORDER.to_vec();
        made.sort();
        assert_eq!(listed, made);
        let copy = fresh(&url, "tabletist_ddl_copy").await;
        copy.batch_execute(
            "SET check_function_bodies = off;
             CREATE FUNCTION public.book_covers_check() RETURNS trigger LANGUAGE plpgsql
                 AS $$BEGIN RETURN NEW; END$$;
             CREATE FUNCTION public.customer_count() RETURNS bigint LANGUAGE sql STABLE
                 AS 'SELECT count(*) FROM public.customers';",
        )
        .await
        .unwrap();
        for name in MADE_IN_ORDER {
            let text = ddl(&connection, name).await.text();
            copy.batch_execute(&text)
                .await
                .unwrap_or_else(|error| panic!("{name}: {error}\n{text}"));
        }
        for name in MADE_IN_ORDER {
            assert_eq!(
                catalog_of(&copy, name).await,
                catalog_of(&source, name).await,
                "{name}"
            );
        }
        // The copy goes when the test passes; one that failed stays to be
        // looked at.
        drop(copy);
        admin(&url, None)
            .await
            .batch_execute("DROP DATABASE tabletist_ddl_copy WITH (FORCE)")
            .await
            .unwrap();
    }

    /// What `SHOW search_path` answers on `connection`.
    async fn search_path(connection: &Connection) -> String {
        let statements =
            tabletist_db::sql::statements(tabletist_db::Dialect::Postgres, "SHOW search_path");
        let stop = tabletist_db::StopFlag::new();
        let outcome = connection
            .run_script(&statements, 10, tabletist_db::ScriptMode::ReadOnly, &stop)
            .await
            .unwrap();
        format!("{:?}", outcome.results[0].outcome)
    }

    /// What only a table's own text can say comes back too: what a
    /// partition and a child set themselves on the columns their parents
    /// give them, a check that does not hold for the rows already there,
    /// and a partitioned table's index whose name holds `ON ONLY`.
    #[tokio::test]
    async fn what_a_table_set_itself_comes_back() {
        let Some(url) = url() else {
            eprintln!("skipped: TABLETIST_TEST_PG_URL is not set");
            return;
        };
        let source = fresh(&url, "tabletist_ddl_cases").await;
        source
            .batch_execute(
                "CREATE TABLE parts (kind integer NOT NULL, note text DEFAULT 'none')
                     PARTITION BY LIST (kind);
                 CREATE TABLE parts_one PARTITION OF parts FOR VALUES IN (1);
                 ALTER TABLE parts_one ALTER COLUMN note SET NOT NULL,
                     ALTER COLUMN note SET DEFAULT 'one';
                 CREATE INDEX \"by ON ONLY kind\" ON parts (kind);
                 CREATE TABLE base (id integer, note text DEFAULT 'base');
                 CREATE TABLE child () INHERITS (base);
                 ALTER TABLE child ALTER COLUMN note DROP DEFAULT,
                     ALTER COLUMN id SET NOT NULL;
                 CREATE TABLE amounts (n integer);
                 INSERT INTO amounts VALUES (5);
                 ALTER TABLE amounts ADD CONSTRAINT amounts_small CHECK (n < 3) NOT VALID;",
            )
            .await
            .unwrap();
        let (connection, _) = connect("tabletist_ddl_cases", Access::ReadOnly)
            .await
            .unwrap();
        let copy = fresh(&url, "tabletist_ddl_cases_copy").await;
        let names = ["parts", "parts_one", "base", "child", "amounts"];
        for name in names {
            let text = ddl(&connection, name).await.text();
            copy.batch_execute(&text)
                .await
                .unwrap_or_else(|error| panic!("{name}: {error}\n{text}"));
        }
        for name in names {
            assert_eq!(
                catalog_of(&copy, name).await,
                catalog_of(&source, name).await,
                "{name}"
            );
        }
        drop((source, copy, connection));
        let server = admin(&url, None).await;
        for database in ["tabletist_ddl_cases", "tabletist_ddl_cases_copy"] {
            server
                .batch_execute(&format!("DROP DATABASE {database} WITH (FORCE)"))
                .await
                .unwrap();
        }
    }

    /// The read leaves the session as it was: the search path it empties
    /// is the transaction's, which is gone. A session that writes reads it
    /// the same way.
    #[tokio::test]
    async fn the_read_leaves_the_session_as_it_was() {
        let Some((connection, _)) = connect("bookshop", Access::Writable).await else {
            return;
        };
        let before = search_path(&connection).await;
        ddl(&connection, "book_covers").await;
        assert_eq!(search_path(&connection).await, before);
    }

    #[tokio::test]
    async fn every_bookshop_table_and_view_has_a_ddl_on_a_read_only_session() {
        let Some(connection) = bookshop().await else {
            return;
        };
        for name in MADE_IN_ORDER {
            ddl(&connection, name).await;
        }
    }
}

mod mysql {
    use super::*;
    use mysql_async::prelude::Queryable;
    use tokio::sync::OnceCell;

    /// Loaded once for this file's tests, with the triggers the fixture
    /// cannot make: whether the server let the tests' user make them.
    static BOOKSHOP: OnceCell<bool> = OnceCell::const_new();

    fn url() -> Option<String> {
        let url = std::env::var("TABLETIST_TEST_MYSQL_URL").ok();
        url.filter(|url| !url.trim().is_empty())
    }

    /// A session of the driver's own on `database`.
    async fn admin(url: &str, database: &str) -> mysql_async::Conn {
        let (server, _database) = url.rsplit_once('/').unwrap();
        let opts = mysql_async::Opts::from_url(&format!("{server}/{database}?prefer_socket=false"))
            .unwrap();
        mysql_async::Conn::new(opts).await.unwrap()
    }

    /// The Bookshop, in its database of its own (compose/mysql-init.sql
    /// makes it), and `book_covers`' two triggers, made in the order their
    /// names are not. Whether they were made: with its binary log on, the
    /// server lets the tests' user make one only where compose.yaml has it
    /// trust them.
    async fn load(url: &str) -> bool {
        *BOOKSHOP
            .get_or_init(|| async {
                let mut conn = admin(url, "bookshop").await;
                for statement in tabletist_db::fixtures::BOOKSHOP_MYSQL_SQL.split(";\n") {
                    let statement: String = statement
                        .lines()
                        .filter(|line| !line.trim_start().starts_with("--"))
                        .collect::<Vec<_>>()
                        .join("\n");
                    if !statement.trim().is_empty() {
                        conn.query_drop(statement).await.unwrap();
                    }
                }
                let made = conn
                    .query_drop(
                        "CREATE TRIGGER book_covers_lower_kind BEFORE INSERT ON book_covers \
                         FOR EACH ROW SET NEW.kind = LOWER(NEW.kind)",
                    )
                    .await;
                if let Err(error) = made {
                    eprintln!("the server lets the tests' user make no trigger: {error}");
                    return false;
                }
                conn.query_drop(
                    "CREATE TRIGGER book_covers_keep_kind BEFORE UPDATE ON book_covers \
                     FOR EACH ROW SET NEW.kind = LOWER(NEW.kind)",
                )
                .await
                .unwrap();
                true
            })
            .await
    }

    /// A read-only session of the app's on the Bookshop, whether
    /// `book_covers` has its triggers, and the URL; or `None` (test
    /// skipped).
    pub(super) async fn bookshop() -> Option<(Connection, bool, String)> {
        let Some(url) = url() else {
            eprintln!("skipped: TABLETIST_TEST_MYSQL_URL is not set");
            return None;
        };
        let triggers = load(&url).await;
        let (mut spec, secrets) = ConnectSpec::from_url(&url).unwrap();
        spec.tls = TlsMode::Disable;
        spec.database = "bookshop".into();
        let connection =
            Connection::connect_with(&spec, &secrets, &HostKeys::default(), Access::ReadOnly)
                .await
                .unwrap();
        Some((connection, triggers, url))
    }

    async fn ddl(connection: &Connection, name: &str) -> Ddl {
        connection
            .ddl(&ObjectRef::new("bookshop", name))
            .await
            .unwrap()
    }

    /// `SHOW CREATE ...`'s text, read by the driver alone.
    async fn shown(url: &str, sql: &str, column: usize) -> String {
        let row: mysql_async::Row = admin(url, "bookshop")
            .await
            .query_first(sql)
            .await
            .unwrap()
            .unwrap();
        row.get(column).unwrap()
    }

    #[tokio::test]
    async fn book_covers_is_what_the_server_shows_then_its_triggers_by_name() {
        let Some((connection, triggers, url)) = bookshop().await else {
            return;
        };
        let ddl = ddl(&connection, "book_covers").await;
        assert_eq!(ddl.source(), Source::Stored);
        // The server's own text, byte for byte.
        assert_eq!(
            ddl.parts[0].statements,
            [shown(&url, "SHOW CREATE TABLE book_covers", 1).await]
        );
        if !triggers {
            eprintln!("skipped: book_covers has no triggers on this server");
            return;
        }
        assert_eq!(steady(&ddl.text()), expected("mysql_book_covers"));
        let triggers = ddl.part(PartKind::Triggers).unwrap();
        assert_eq!(
            triggers.statements,
            [
                shown(&url, "SHOW CREATE TRIGGER book_covers_keep_kind", 2).await,
                shown(&url, "SHOW CREATE TRIGGER book_covers_lower_kind", 2).await,
            ]
        );
        assert_eq!(
            counts(&ddl),
            [(PartKind::Object, 1), (PartKind::Triggers, 2)]
        );
    }

    #[tokio::test]
    async fn in_stock_is_what_the_server_shows() {
        let Some((connection, _, url)) = bookshop().await else {
            return;
        };
        let ddl = ddl(&connection, "in_stock").await;
        assert_eq!(
            ddl.parts[0].statements,
            [shown(&url, "SHOW CREATE VIEW in_stock", 1).await]
        );
        assert_eq!(steady(&ddl.text()), expected("mysql_in_stock"));
    }

    /// A user who may read a view but not see its definition is told so
    /// by the server: its number and words, and no text at all.
    #[tokio::test]
    async fn a_user_who_may_not_see_the_definition_gets_the_servers_error() {
        let Some((_, _, url)) = bookshop().await else {
            return;
        };
        admin(&url, "tabletist")
            .await
            .query_drop("CREATE OR REPLACE VIEW ddl_denied AS SELECT 1 AS one")
            .await
            .unwrap();
        // compose/mysql-init.sql's `writer`, who has SELECT and no SHOW VIEW.
        let (mut spec, mut secrets) = ConnectSpec::from_url(&url).unwrap();
        spec.tls = TlsMode::Disable;
        spec.user = "writer".into();
        secrets.password = Some("writer".into());
        let connection =
            match Connection::connect_with(&spec, &secrets, &HostKeys::default(), Access::ReadOnly)
                .await
            {
                Ok(connection) => connection,
                Err(error) => {
                    eprintln!("skipped: the server has no `writer`: {error}");
                    return;
                }
            };
        let error = connection
            .ddl(&ObjectRef::new("tabletist", "ddl_denied"))
            .await
            .unwrap_err();
        admin(&url, "tabletist")
            .await
            .query_drop("DROP VIEW ddl_denied")
            .await
            .unwrap();
        let Error::Query { message, named, .. } = &error else {
            panic!("{error:?}");
        };
        assert_eq!(named.number, Some(1142), "{error:?}");
        assert!(
            message.starts_with("SHOW VIEW command denied to user 'writer'"),
            "{message}"
        );
    }
}

/// Every engine reads a DDL of the Bookshop's `book_covers`, its `CREATE
/// TABLE` first: named by `Driver`, so a new engine is named here too.
#[tokio::test]
async fn every_engine_reads_book_covers() {
    for driver in [Driver::Sqlite, Driver::Postgres, Driver::MySql] {
        let ddl = match driver {
            Driver::Sqlite => Some(sqlite::ddl("book_covers").await),
            Driver::Postgres => match postgres::bookshop().await {
                Some(connection) => Some(
                    connection
                        .ddl(&ObjectRef::new("public", "book_covers"))
                        .await
                        .unwrap(),
                ),
                None => None,
            },
            Driver::MySql => match mysql::bookshop().await {
                Some((connection, _, _)) => Some(
                    connection
                        .ddl(&ObjectRef::new("bookshop", "book_covers"))
                        .await
                        .unwrap(),
                ),
                None => None,
            },
        };
        let Some(ddl) = ddl else {
            continue;
        };
        assert_eq!(ddl.driver, driver);
        assert_eq!(ddl.parts[0].kind, PartKind::Object);
        assert!(ddl.text().contains("TABLE"), "{driver:?}");
    }
}
