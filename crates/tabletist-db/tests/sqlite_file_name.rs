//! A SQLite path is a file name, never a `file:` URI. The test changes the
//! working directory, which the whole process shares, so it has this file
//! (and its own test process) to itself.

// Windows file names cannot hold `?`, and `:` there names a stream.
#![cfg(unix)]
// `allow-unwrap-in-tests` does not cover helpers in an integration-test crate.
#![allow(clippy::unwrap_used)]

use tabletist_db::{ConnectSpec, Connection, Secrets};

#[tokio::test]
async fn a_path_starting_with_file_colon_opens_the_file_of_that_name() {
    let dir = tempfile::tempdir().unwrap();
    // What each name would open if SQLite read it as a URI.
    rusqlite::Connection::open(dir.path().join("shop.db"))
        .unwrap()
        .execute_batch("CREATE TABLE decoy (id INTEGER)")
        .unwrap();
    let names = [
        "file:shop.db",
        "file:shop.db?immutable=1",
        "file:shop.db?nolock=1",
        "file:shop.db?vfs=unix-none",
        "file:shop.db?vfs=missing",
        "file:shop.db?mode=memory",
    ];
    for name in names {
        tabletist_db::fixtures::write_sqlite_demo(&dir.path().join(name)).unwrap();
    }
    std::env::set_current_dir(dir.path()).unwrap();

    for name in names {
        let connection = Connection::connect(&ConnectSpec::sqlite(name), &Secrets::default())
            .await
            .unwrap_or_else(|error| panic!("{name}: {error}"));
        let objects = connection.list_objects("main").await.unwrap();
        let tables: Vec<&str> = objects.iter().map(|object| object.name.as_str()).collect();
        assert!(tables.contains(&"users"), "{name} opened {tables:?}");
        assert!(!tables.contains(&"decoy"), "{name} opened {tables:?}");
    }
}
