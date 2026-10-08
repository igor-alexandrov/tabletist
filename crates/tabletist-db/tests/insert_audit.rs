//! The row insert spec's error cases, on a Bookshop made in each engine: a
//! taken `isbn`, a publisher that is not there, a NULL where none may be,
//! and a `kind` the CHECK refuses. What each test holds is what the
//! database hands the app for it, which is all the app has to word a
//! failed cell from, and what a save sends for a new row.
//!
//! These were written for the audit of row inserting against its spec
//! (`docs/superpowers/specs/2026-10-07-inserting-rows-design.md`). Each
//! names the finding of that audit it shows (INS-...). They hold what the
//! drivers do today, so they pass: where the spec asks for something else,
//! the test says so.
//!
//! PostgreSQL and MySQL need their servers (see compose.yaml). Without the
//! variables those tests print "skipped" and pass.

// `allow-unwrap-in-tests` does not cover helpers in an integration-test crate.
#![allow(clippy::unwrap_used)]

use std::time::Duration;

use mysql_async::prelude::Queryable;
use tabletist_db::{
    Access, ChangeSet, ConnectSpec, Connection, Dialect, Error, HostKeys, InsertValue, NewValue,
    ObjectRef, RowInsert, RowQuery, Secrets, StopFlag, Structure, TlsMode, Value, WriteOutcome,
};

/// Harbor Press, as the design's picker finds it.
const HARBOR_PRESS: &str = "9100000000000000004";
/// No publisher has this id.
const NO_PUBLISHER: &str = "9100000000000000099";
/// The `isbn` of the book with id 101.
const TAKEN_ISBN: &str = "978-1-4028-9462-6";

fn sets(column: &str, type_name: &str, new: &str) -> InsertValue {
    InsertValue {
        column: column.into(),
        type_name: type_name.into(),
        new: NewValue::Text(new.into()),
    }
}

fn null(column: &str, type_name: &str) -> InsertValue {
    InsertValue {
        column: column.into(),
        type_name: type_name.into(),
        new: NewValue::Null,
    }
}

/// A save of new rows alone.
fn adding(schema: &str, table: &str, inserts: Vec<Vec<InsertValue>>) -> ChangeSet {
    ChangeSet {
        object: ObjectRef::new(schema, table),
        inserts: inserts.into_iter().map(|set| RowInsert { set }).collect(),
        rows: Vec::new(),
    }
}

async fn within<T>(future: impl std::future::Future<Output = T>) -> T {
    tokio::time::timeout(Duration::from_secs(15), future)
        .await
        .expect("the save hung")
}

async fn save(connection: &Connection, changes: &ChangeSet) -> WriteOutcome {
    within(connection.write(changes, &StopFlag::new()))
        .await
        .unwrap()
}

async fn count(connection: &Connection, schema: &str, table: &str) -> usize {
    let query = RowQuery::new(ObjectRef::new(schema, table), 500);
    connection.fetch_rows(&query).await.unwrap().rows.len()
}

/// The code, the message and the detail of a new row's failure, and which
/// of the set's rows it was.
fn failed(outcome: WriteOutcome) -> (usize, Option<String>, String, Option<String>) {
    match outcome {
        WriteOutcome::FailedInsert {
            insert,
            error:
                Error::Query {
                    code,
                    message,
                    detail,
                    ..
                },
        } => (insert, code, message, detail),
        other => panic!("the save was to fail on a new row: {other:?}"),
    }
}

fn column<'a>(structure: &'a Structure, name: &str) -> &'a tabletist_db::ColumnInfo {
    let found = structure.columns.iter().find(|column| column.name == name);
    found.unwrap_or_else(|| panic!("no column {name}"))
}

/// The four cases of the spec, each as the types the engine names: a
/// taken `isbn` in `books`, then no such publisher, a NULL publisher and a
/// `kind` that is not allowed in `book_covers`.
struct Cases {
    schema: &'static str,
    books: &'static str,
    covers: &'static str,
    id: &'static str,
    text: &'static str,
}

impl Cases {
    fn taken_isbn(&self) -> ChangeSet {
        adding(
            self.schema,
            self.books,
            vec![vec![
                sets("isbn", self.text, TAKEN_ISBN),
                sets("title", self.text, "The Lighthouse Keeper's Daughter"),
                sets("publisher_id", self.id, HARBOR_PRESS),
            ]],
        )
    }

    /// A cover that would go in, then the one that fails: what the first
    /// wrote must be undone with it.
    fn no_publisher(&self) -> ChangeSet {
        adding(
            self.schema,
            self.covers,
            vec![
                vec![sets("publisher_id", self.id, HARBOR_PRESS)],
                vec![sets("publisher_id", self.id, NO_PUBLISHER)],
            ],
        )
    }

    fn null_publisher(&self) -> ChangeSet {
        adding(
            self.schema,
            self.covers,
            vec![vec![null("publisher_id", self.id)]],
        )
    }

    fn vinyl(&self) -> ChangeSet {
        adding(
            self.schema,
            self.covers,
            vec![vec![
                sets("publisher_id", self.id, HARBOR_PRESS),
                sets("kind", self.text, "vinyl"),
            ]],
        )
    }
}

mod postgres {
    use super::*;

    const CASES: Cases = Cases {
        schema: "public",
        books: "audit_books",
        covers: "audit_book_covers",
        id: "bigint",
        text: "text",
    };

    const DROP: &str = "DROP TABLE IF EXISTS audit_book_covers, audit_books, audit_publishers";

    const CREATE: &str = "
        CREATE TABLE audit_publishers (
            id bigint PRIMARY KEY,
            name text NOT NULL
        );
        INSERT INTO audit_publishers VALUES
            (9100000000000000001, 'Northlight Books'),
            (9100000000000000004, 'Harbor Press'),
            (9100000000000000007, 'Harbor Lights');
        CREATE TABLE audit_books (
            id bigint GENERATED ALWAYS AS IDENTITY (START WITH 101) PRIMARY KEY,
            isbn text NOT NULL UNIQUE,
            title text NOT NULL,
            format varchar NOT NULL DEFAULT 'hardcover',
            publisher_id bigint NOT NULL REFERENCES audit_publishers (id)
        );
        INSERT INTO audit_books (isbn, title, publisher_id) VALUES
            ('978-1-4028-9462-6', 'The Lighthouse Keeper''s Daughter', 9100000000000000004);
        CREATE TABLE audit_book_covers (
            id bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
            publisher_id bigint NOT NULL REFERENCES audit_publishers (id),
            kind varchar NOT NULL DEFAULT 'print'
                CONSTRAINT audit_book_covers_kind_check CHECK (kind IN ('print', 'ebook', 'audio')),
            image_data jsonb,
            created_at timestamp NOT NULL DEFAULT now(),
            updated_at timestamp NOT NULL DEFAULT now()
        );
        INSERT INTO audit_book_covers (publisher_id, kind) VALUES
            (9100000000000000001, 'print'),
            (9100000000000000001, 'ebook')";

    fn url() -> Option<String> {
        let url = std::env::var("TABLETIST_TEST_PG_URL").ok();
        url.filter(|url| !url.trim().is_empty())
    }

    async fn admin() -> tokio_postgres::Client {
        let mut config: tokio_postgres::Config = url().unwrap().parse().unwrap();
        config.ssl_mode(tokio_postgres::config::SslMode::Disable);
        let (client, connection) = config.connect(tokio_postgres::NoTls).await.unwrap();
        tokio::spawn(connection);
        client
    }

    async fn connect(url: &str) -> Connection {
        let (mut spec, secrets) = ConnectSpec::from_url(url).unwrap();
        spec.tls = TlsMode::Disable;
        Connection::connect_with(&spec, &secrets, &HostKeys::default(), Access::Writable)
            .await
            .unwrap()
    }

    /// The Bookshop's three tables, made anew. The tests of this module
    /// share them, so they are one test.
    async fn bookshop() -> Option<(tokio_postgres::Client, Connection)> {
        let Some(url) = url() else {
            eprintln!("skipped: TABLETIST_TEST_PG_URL is not set");
            return None;
        };
        let admin = admin().await;
        admin.batch_execute(DROP).await.unwrap();
        admin.batch_execute(CREATE).await.unwrap();
        Some((admin, connect(&url).await))
    }

    /// INS-31: PostgreSQL says which rule a new row broke by its SQLSTATE,
    /// and names the column and the value in the error's detail. The
    /// app's `Error` has no column or constraint of its own, so a failed
    /// cell can be worded only from this text.
    #[tokio::test]
    async fn the_bookshops_failures_as_postgres_hands_them_over() {
        let Some((admin, connection)) = bookshop().await else {
            return;
        };
        let (insert, code, message, detail) = failed(save(&connection, &CASES.taken_isbn()).await);
        assert_eq!((insert, code.as_deref()), (0, Some("23505")));
        assert_eq!(
            message,
            r#"duplicate key value violates unique constraint "audit_books_isbn_key""#
        );
        assert_eq!(
            detail.as_deref(),
            Some("Key (isbn)=(978-1-4028-9462-6) already exists.")
        );

        // The second new row fails, and the first is undone with it.
        let (insert, code, message, detail) =
            failed(save(&connection, &CASES.no_publisher()).await);
        assert_eq!((insert, code.as_deref()), (1, Some("23503")));
        assert!(
            message.ends_with(r#"foreign key constraint "audit_book_covers_publisher_id_fkey""#),
            "{message}"
        );
        assert_eq!(
            detail.as_deref(),
            Some(
                r#"Key (publisher_id)=(9100000000000000099) is not present in table "audit_publishers"."#
            )
        );

        let (_, code, message, _) = failed(save(&connection, &CASES.null_publisher()).await);
        assert_eq!(code.as_deref(), Some("23502"));
        assert!(message.contains(r#"column "publisher_id""#), "{message}");

        let (_, code, message, _) = failed(save(&connection, &CASES.vinyl()).await);
        assert_eq!(code.as_deref(), Some("23514"));
        assert!(
            message.ends_with(r#"check constraint "audit_book_covers_kind_check""#),
            "{message}"
        );

        // Nothing of the four saves is in the tables.
        assert_eq!(count(&connection, "public", "audit_books").await, 1);
        assert_eq!(count(&connection, "public", "audit_book_covers").await, 2);
        // The list a paste would check `kind` against is read here.
        let structure = connection.describe(&covers()).await.unwrap();
        let allowed = column(&structure, "kind").allowed_values.clone();
        assert_eq!(
            allowed,
            Some(vec!["print".into(), "ebook".into(), "audio".into()])
        );
        admin.batch_execute(DROP).await.unwrap();
    }

    /// INS-28a, INS-28b: the spec has every value a bound parameter and up
    /// to 100 rows in one statement. PostgreSQL is sent each new row as a
    /// statement of its own, with its values written into the text: what
    /// the server ran is what Review SQL shows, letter for letter.
    #[tokio::test]
    async fn postgres_is_sent_each_new_row_with_its_values_in_the_text() {
        if url().is_none() {
            eprintln!("skipped: TABLETIST_TEST_PG_URL is not set");
            return;
        }
        let admin = admin().await;
        let drop = "DROP TABLE IF EXISTS audit_sent_covers, audit_sent;
                    DROP FUNCTION IF EXISTS audit_sent_log";
        admin.batch_execute(drop).await.unwrap();
        // A trigger that keeps the statement the server is running. With
        // one the save hands its new rows back as unknown, which is not
        // what is looked at here.
        admin
            .batch_execute(
                "CREATE TABLE audit_sent (at serial, query text);
                 CREATE TABLE audit_sent_covers (
                     id bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
                     publisher_id bigint NOT NULL DEFAULT 9100000000000000001,
                     kind varchar NOT NULL DEFAULT 'print'
                 );
                 CREATE FUNCTION audit_sent_log() RETURNS trigger LANGUAGE plpgsql AS $$
                 BEGIN
                     INSERT INTO audit_sent (query) VALUES (current_query());
                     RETURN NEW;
                 END $$;
                 CREATE TRIGGER audit_sent_log BEFORE INSERT ON audit_sent_covers
                     FOR EACH ROW EXECUTE FUNCTION audit_sent_log()",
            )
            .await
            .unwrap();
        let connection = connect(&url().unwrap()).await;
        let changes = adding(
            "public",
            "audit_sent_covers",
            vec![
                vec![sets("publisher_id", "bigint", HARBOR_PRESS)],
                vec![sets("publisher_id", "bigint", "9100000000000000007")],
                Vec::new(),
            ],
        );
        let outcome = save(&connection, &changes).await;
        assert!(
            matches!(outcome, WriteOutcome::Written { .. }),
            "{outcome:?}"
        );
        let sent: Vec<String> = admin
            .query("SELECT query FROM audit_sent ORDER BY at", &[])
            .await
            .unwrap()
            .iter()
            .map(|row| row.get(0))
            .collect();
        let shown: Vec<String> = changes
            .inserts
            .iter()
            .map(|row| {
                let built = Dialect::Postgres.insert_row(&changes.object, row);
                built.unwrap().shown
            })
            .collect();
        assert_eq!(sent, shown);
        assert_eq!(
            sent,
            [
                r#"INSERT INTO "public"."audit_sent_covers" ("publisher_id") VALUES ('9100000000000000004') RETURNING *"#,
                r#"INSERT INTO "public"."audit_sent_covers" ("publisher_id") VALUES ('9100000000000000007') RETURNING *"#,
                r#"INSERT INTO "public"."audit_sent_covers" DEFAULT VALUES RETURNING *"#,
            ]
        );
        admin.batch_execute(drop).await.unwrap();
    }

    /// INS-04c: a role that may read a table and not add to it. Nothing
    /// the app reads of the table says so, so no entry point is disabled
    /// for it: the save is where the user learns it.
    #[tokio::test]
    async fn a_role_that_may_not_insert_learns_it_from_the_save() {
        let Some(url) = url() else {
            eprintln!("skipped: TABLETIST_TEST_PG_URL is not set");
            return;
        };
        let admin = admin().await;
        let drop = "DROP TABLE IF EXISTS audit_role_covers; DROP ROLE IF EXISTS audit_reader";
        admin.batch_execute(drop).await.unwrap();
        admin
            .batch_execute(
                "CREATE ROLE audit_reader LOGIN PASSWORD 'audit_reader';
                 CREATE TABLE audit_role_covers (
                     id bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
                     publisher_id bigint NOT NULL
                 );
                 GRANT SELECT ON audit_role_covers TO audit_reader",
            )
            .await
            .unwrap();
        let (mut spec, _) = ConnectSpec::from_url(&url).unwrap();
        spec.tls = TlsMode::Disable;
        spec.user = "audit_reader".into();
        let secrets = Secrets {
            password: Some("audit_reader".into()),
            ..Secrets::default()
        };
        let reader =
            Connection::connect_with(&spec, &secrets, &HostKeys::default(), Access::Writable)
                .await
                .unwrap();
        // The table reads as any other: a table, with its columns.
        let object = ObjectRef::new("public", "audit_role_covers");
        let structure = reader.describe(&object).await.unwrap();
        assert_eq!(structure.columns.len(), 2);
        let changes = adding(
            "public",
            "audit_role_covers",
            vec![vec![sets("publisher_id", "bigint", HARBOR_PRESS)]],
        );
        let (_, code, message, _) = failed(save(&reader, &changes).await);
        assert_eq!(code.as_deref(), Some("42501"));
        assert_eq!(message, "permission denied for table audit_role_covers");
        drop_connection(reader).await;
        admin.batch_execute(drop).await.unwrap();
    }

    fn covers() -> ObjectRef {
        ObjectRef::new(CASES.schema, CASES.covers)
    }

    /// Ends `connection`'s session before its role is dropped.
    async fn drop_connection(connection: Connection) {
        connection.close().await.unwrap();
    }
}

mod mysql {
    use super::*;

    const CASES: Cases = Cases {
        schema: "tabletist",
        books: "audit_books",
        covers: "audit_book_covers",
        id: "bigint",
        text: "varchar(64)",
    };

    const DROP: &str = "DROP TABLE IF EXISTS audit_book_covers, audit_books, audit_publishers";

    const CREATE: [&str; 6] = [
        "CREATE TABLE audit_publishers (
             id BIGINT PRIMARY KEY,
             name VARCHAR(64) NOT NULL
         )",
        "INSERT INTO audit_publishers VALUES
             (9100000000000000001, 'Northlight Books'),
             (9100000000000000004, 'Harbor Press'),
             (9100000000000000007, 'Harbor Lights')",
        "CREATE TABLE audit_books (
             id BIGINT AUTO_INCREMENT PRIMARY KEY,
             isbn VARCHAR(64) NOT NULL UNIQUE,
             title VARCHAR(64) NOT NULL,
             format VARCHAR(64) NOT NULL DEFAULT 'hardcover',
             publisher_id BIGINT NOT NULL,
             FOREIGN KEY (publisher_id) REFERENCES audit_publishers (id)
         ) AUTO_INCREMENT = 101",
        "INSERT INTO audit_books (isbn, title, publisher_id) VALUES
             ('978-1-4028-9462-6', 'The Lighthouse Keeper''s Daughter', 9100000000000000004)",
        "CREATE TABLE audit_book_covers (
             id BIGINT AUTO_INCREMENT PRIMARY KEY,
             publisher_id BIGINT NOT NULL,
             kind VARCHAR(64) NOT NULL DEFAULT 'print',
             image_data JSON,
             created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
             updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
             FOREIGN KEY (publisher_id) REFERENCES audit_publishers (id),
             CONSTRAINT audit_book_covers_kind_check CHECK (kind IN ('print', 'ebook', 'audio'))
         )",
        "INSERT INTO audit_book_covers (publisher_id, kind) VALUES
             (9100000000000000001, 'print'),
             (9100000000000000001, 'ebook')",
    ];

    fn url() -> Option<String> {
        let url = std::env::var("TABLETIST_TEST_MYSQL_URL").ok();
        url.filter(|url| !url.trim().is_empty())
    }

    /// The Bookshop's three tables, made anew, with a session of the test's
    /// own to drop them with. `None` without a server.
    async fn bookshop() -> Option<(mysql_async::Conn, Connection)> {
        let Some(url) = url() else {
            eprintln!("skipped: TABLETIST_TEST_MYSQL_URL is not set");
            return None;
        };
        let opts = mysql_async::Opts::from_url(&format!("{url}?prefer_socket=false")).unwrap();
        let mut admin = mysql_async::Conn::new(opts).await.unwrap();
        admin.query_drop(DROP).await.unwrap();
        for statement in CREATE {
            admin.query_drop(statement).await.unwrap();
        }
        let (mut spec, secrets) = ConnectSpec::from_url(&url).unwrap();
        spec.tls = TlsMode::Disable;
        let connection =
            Connection::connect_with(&spec, &secrets, &HostKeys::default(), Access::Writable)
                .await
                .unwrap();
        Some((admin, connection))
    }

    /// INS-32a: the spec words a failed cell by MySQL's error number
    /// (1062, 1452, 1048, 3819). The driver keeps the SQLSTATE and drops
    /// the number, and a taken value, a missing parent and a NULL all have
    /// the state 23000: only the message tells them apart.
    #[tokio::test]
    async fn the_bookshops_failures_as_mysql_hands_them_over() {
        let Some((mut admin, connection)) = bookshop().await else {
            return;
        };
        let (insert, code, message, detail) = failed(save(&connection, &CASES.taken_isbn()).await);
        assert_eq!((insert, code.as_deref()), (0, Some("23000")));
        assert_eq!(
            message,
            "Duplicate entry '978-1-4028-9462-6' for key 'audit_books.isbn'"
        );
        assert_eq!(detail, None);

        // The second new row fails, and the first is undone with it.
        let (insert, code, message, _) = failed(save(&connection, &CASES.no_publisher()).await);
        assert_eq!((insert, code.as_deref()), (1, Some("23000")));
        assert!(
            message.starts_with("Cannot add or update a child row: a foreign key constraint fails"),
            "{message}"
        );
        // The value that has no publisher is not in what MySQL says.
        assert!(!message.contains(NO_PUBLISHER), "{message}");

        let (_, code, message, _) = failed(save(&connection, &CASES.null_publisher()).await);
        assert_eq!(code.as_deref(), Some("23000"));
        assert_eq!(message, "Column 'publisher_id' cannot be null");

        let (_, code, message, _) = failed(save(&connection, &CASES.vinyl()).await);
        assert_eq!(code.as_deref(), Some("HY000"));
        assert_eq!(
            message,
            "Check constraint 'audit_book_covers_kind_check' is violated."
        );

        assert_eq!(count(&connection, "tabletist", "audit_books").await, 1);
        assert_eq!(
            count(&connection, "tabletist", "audit_book_covers").await,
            2
        );
        // INS-25a: the list a paste would check `kind` against is not read
        // from a MySQL CHECK.
        let object = ObjectRef::new(CASES.schema, CASES.covers);
        let structure = connection.describe(&object).await.unwrap();
        assert_eq!(column(&structure, "kind").allowed_values, None);
        admin.query_drop(DROP).await.unwrap();
    }
}

mod sqlite {
    use super::*;

    const CASES: Cases = Cases {
        schema: "main",
        books: "books",
        covers: "book_covers",
        id: "INTEGER",
        text: "TEXT",
    };

    const CREATE: &str = "
        CREATE TABLE publishers (
            id INTEGER PRIMARY KEY,
            name TEXT NOT NULL
        );
        INSERT INTO publishers VALUES
            (9100000000000000001, 'Northlight Books'),
            (9100000000000000004, 'Harbor Press'),
            (9100000000000000007, 'Harbor Lights');
        CREATE TABLE books (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            isbn TEXT NOT NULL UNIQUE,
            title TEXT NOT NULL,
            format TEXT NOT NULL DEFAULT 'hardcover',
            publisher_id INTEGER NOT NULL REFERENCES publishers (id)
        );
        INSERT INTO books (id, isbn, title, publisher_id) VALUES
            (101, '978-1-4028-9462-6', 'The Lighthouse Keeper''s Daughter', 9100000000000000004);
        CREATE TABLE book_covers (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            publisher_id INTEGER NOT NULL REFERENCES publishers (id),
            kind TEXT NOT NULL DEFAULT 'print'
                CONSTRAINT book_covers_kind_check CHECK (kind IN ('print', 'ebook', 'audio')),
            image_data TEXT,
            created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
        );
        INSERT INTO book_covers (publisher_id, kind) VALUES
            (9100000000000000001, 'print'),
            (9100000000000000001, 'ebook')";

    async fn bookshop() -> (Connection, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("bookshop.db");
        rusqlite::Connection::open(&path)
            .unwrap()
            .execute_batch(CREATE)
            .unwrap();
        let connection = Connection::connect_with(
            &ConnectSpec::sqlite(&path),
            &Secrets::default(),
            &HostKeys::default(),
            Access::Writable,
        )
        .await
        .unwrap();
        (connection, dir)
    }

    /// INS-32b: SQLite says which rule a new row broke by its extended
    /// result code, as the spec's SQLITE_CONSTRAINT_* has it. Its foreign
    /// key failure names neither the column nor the value.
    #[tokio::test]
    async fn the_bookshops_failures_as_sqlite_hands_them_over() {
        let (connection, _dir) = bookshop().await;
        // SQLITE_CONSTRAINT_UNIQUE.
        let (insert, code, message, detail) = failed(save(&connection, &CASES.taken_isbn()).await);
        assert_eq!((insert, code.as_deref()), (0, Some("2067")));
        assert_eq!(message, "UNIQUE constraint failed: books.isbn");
        assert_eq!(detail, None);

        // SQLITE_CONSTRAINT_FOREIGNKEY: the second new row fails, and the
        // first is undone with it.
        let (insert, code, message, _) = failed(save(&connection, &CASES.no_publisher()).await);
        assert_eq!((insert, code.as_deref()), (1, Some("787")));
        assert_eq!(message, "FOREIGN KEY constraint failed");

        // SQLITE_CONSTRAINT_NOTNULL.
        let (_, code, message, _) = failed(save(&connection, &CASES.null_publisher()).await);
        assert_eq!(code.as_deref(), Some("1299"));
        assert_eq!(
            message,
            "NOT NULL constraint failed: book_covers.publisher_id"
        );

        // SQLITE_CONSTRAINT_CHECK.
        let (_, code, message, _) = failed(save(&connection, &CASES.vinyl()).await);
        assert_eq!(code.as_deref(), Some("275"));
        assert_eq!(message, "CHECK constraint failed: book_covers_kind_check");

        assert_eq!(count(&connection, "main", "books").await, 1);
        assert_eq!(count(&connection, "main", "book_covers").await, 2);
        // INS-25a: the list a paste would check `kind` against is not read
        // from a SQLite CHECK.
        let object = ObjectRef::new(CASES.schema, CASES.covers);
        let structure = connection.describe(&object).await.unwrap();
        assert_eq!(column(&structure, "kind").allowed_values, None);
    }

    /// INS-27c: the spec has `last_insert_rowid()` for a SQLite older than
    /// 3.35. The app's SQLite is its own, and it has `RETURNING`: a new
    /// row comes back with the id and the defaults the database gave it.
    #[tokio::test]
    async fn the_apps_own_sqlite_hands_a_new_row_back() {
        let version: Vec<u32> = rusqlite::version()
            .split('.')
            .map(|part| part.parse().unwrap())
            .collect();
        assert!((version[0], version[1]) >= (3, 35), "{version:?}");
        let (connection, _dir) = bookshop().await;
        let changes = adding(
            "main",
            "book_covers",
            vec![vec![sets("publisher_id", "INTEGER", HARBOR_PRESS)]],
        );
        let WriteOutcome::Written { inserted, .. } = save(&connection, &changes).await else {
            panic!("the save wrote");
        };
        let row = inserted[0].as_ref().expect("the new row");
        assert_eq!(row[0], Value::Int(3));
        assert_eq!(row[1], Value::Int(9_100_000_000_000_000_004));
        assert_eq!(row[2], Value::Text("print".into()));
        assert_eq!(row[3], Value::Null);
        // `created_at` is the moment of the save, not the default's words.
        assert!(
            matches!(&row[4], Value::Text(at) if at.starts_with("20")),
            "{row:?}"
        );
    }
}
