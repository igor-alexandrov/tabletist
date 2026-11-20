//! A Files Test where DuckDB's Excel reader cannot be installed: its
//! Excel files are among the files it cannot read, and the others are
//! read. Connecting still fails. A test binary of its own, as the folder
//! DuckDB installs readers into is set once for the process.

// `allow-unwrap-in-tests` does not cover helpers in an integration-test crate.
#![allow(clippy::unwrap_used)]

use tabletist_db::files::{Format, Source, Why};
use tabletist_db::{ConnectSpec, Connection, Secrets};

#[tokio::test]
async fn a_test_says_the_excel_files_and_reads_the_others() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("orders.csv"), "id,total\n1,12.50\n").unwrap();
    std::fs::write(dir.path().join("stock.xlsx"), "not read").unwrap();
    // A file where the folder should be: nothing can be installed in it.
    let blocked = dir.path().join("extensions");
    std::fs::write(&blocked, "").unwrap();
    tabletist_db::set_extension_directory(blocked);
    let spec = ConnectSpec::files(Source::Folder(dir.path().to_owned()));
    let error = Connection::connect(&spec, &Secrets::default())
        .await
        .err()
        .unwrap();
    assert!(
        error.to_string().contains("Can't read Excel files"),
        "{error}"
    );
    let connection = Connection::open_files_to_test(&spec).await.unwrap();
    let read = connection.read_files().await.unwrap().unwrap();
    assert_eq!(read.failed.len(), 1, "{:?}", read.failed);
    assert_eq!(read.failed[0].file, "stock.xlsx");
    assert_eq!(read.failed[0].format, Format::Excel);
    match &read.failed[0].why {
        Why::Other(said) => assert!(said.contains("Excel reader"), "{said}"),
        other => panic!("{other:?}"),
    }
    assert_eq!(read.checks, ["orders"]);
}
