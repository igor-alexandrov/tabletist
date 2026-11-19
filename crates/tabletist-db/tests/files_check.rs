//! A Files connection's Test: the files it reads and those it cannot, and
//! the check in full that follows it, which finds a row past the sample
//! that a file's reading cannot take, and says where and why.

// `allow-unwrap-in-tests` does not cover helpers in an integration-test crate.
#![allow(clippy::unwrap_used)]

use std::path::Path;

use tabletist_db::files::{Change, Failure, Format, Override, Reading, Source, Why};
use tabletist_db::{ConnectSpec, Connection, Secrets};

/// The Bookshop's exports, `returns.csv` among them with 30,000 returns
/// and then one whose `returned_at` is `N/A`: past the rows a reader
/// samples for types.
fn exports() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    tabletist_db::fixtures::write_bookshop_files(dir.path()).unwrap();
    let mut text = String::from("order_id,returned_at,reason\n");
    for order in 1..=30_000 {
        let day = order % 28 + 1;
        text.push_str(&format!("{order},2026-02-{day:02},damaged\n"));
    }
    text.push_str("30001,N/A,lost in the post\n");
    std::fs::write(dir.path().join("returns.csv"), text).unwrap();
    dir
}

fn spec(dir: &Path, overrides: Vec<Override>) -> ConnectSpec {
    let mut spec = ConnectSpec::files(Source::Folder(dir.to_owned()));
    spec.csv = Reading {
        overrides,
        ..Reading::default()
    };
    spec
}

fn returns(change: Change) -> Override {
    Override {
        file: "returns.csv".into(),
        change,
    }
}

/// What a Test reads of `spec`, and what the check in full finds of each
/// file it names.
async fn test(spec: &ConnectSpec) -> (tabletist_db::FilesRead, Vec<Failure>) {
    let connection = Connection::open_files_to_test(spec).await.unwrap();
    let read = connection.read_files().await.unwrap().unwrap();
    let mut found = Vec::new();
    for name in &read.checks {
        found.extend(connection.check_file(name).await.unwrap());
    }
    (read, found)
}

#[tokio::test]
async fn the_check_in_full_finds_a_value_past_the_sample_and_says_where() {
    let dir = exports();
    let (read, found) = test(&spec(dir.path(), Vec::new())).await;
    // The Test itself reads every file: the value is past its rows.
    assert_eq!(read.files, 7);
    assert!(read.failed.is_empty(), "{:?}", read.failed);
    // The CSV, TSV and NDJSON files are checked in full, not Parquet,
    // JSON arrays or `.duckdb` files.
    let mut checks = read.checks.clone();
    checks.sort();
    assert_eq!(checks, ["authors", "events", "orders", "returns"]);
    assert_eq!(
        found,
        [Failure {
            file: "returns.csv".into(),
            path: dir.path().join("returns.csv"),
            format: Format::Csv,
            // The file's line 30,002, its header row line 1.
            why: Why::Value {
                row: 30_001,
                column: "returned_at".into(),
                value: "N/A".into(),
                kind: "DATE".into(),
            },
        }]
    );
    assert!(found[0].takes_column_fixes());
}

#[tokio::test]
async fn each_fix_reads_the_file_to_its_end() {
    let dir = exports();
    for fix in [
        returns(Change::Text {
            column: "returned_at".into(),
        }),
        returns(Change::Empty {
            column: "returned_at".into(),
            values: vec!["N/A".into()],
        }),
        returns(Change::Skip),
    ] {
        let (read, found) = test(&spec(dir.path(), vec![fix.clone()])).await;
        assert!(found.is_empty(), "{fix:?}: {found:?}");
        // A skipped file is no file of the connection's.
        let (files, checked) = match fix.change {
            Change::Skip => (6, false),
            Change::Text { .. } | Change::Empty { .. } => (7, true),
        };
        assert_eq!(read.files, files, "{fix:?}");
        assert_eq!(read.checks.iter().any(|name| name == "returns"), checked);
    }
}

#[tokio::test]
async fn a_row_with_more_values_and_bad_json_lines_say_their_row_or_line() {
    let dir = tempfile::tempdir().unwrap();
    let mut wide = String::from("id,title\n");
    for id in 1..=30_000 {
        wide.push_str(&format!("{id},Dune\n"));
    }
    wide.push_str("30001,Emma,Austen\n");
    std::fs::write(dir.path().join("books.csv"), wide).unwrap();
    let mut events = String::new();
    for id in 1..=30_000 {
        events.push_str(&format!("{{\"order_id\":{id},\"kind\":\"placed\"}}\n"));
    }
    events.push_str("{\"order_id\":\"N/A\",\"kind\":\"placed\"}\n");
    std::fs::write(dir.path().join("events.ndjson"), events).unwrap();
    let (read, mut found) = test(&spec(dir.path(), Vec::new())).await;
    assert!(read.failed.is_empty(), "{:?}", read.failed);
    found.sort_by(|a, b| a.file.cmp(&b.file));
    let whys: Vec<&Why> = found.iter().map(|failure| &failure.why).collect();
    assert_eq!(
        whys,
        [
            &Why::Columns {
                row: 30_001,
                expected: 2,
                found: 3,
            },
            &Why::Json {
                line: 30_001,
                malformed: false,
            },
        ]
    );
    // Neither is mended by a change to a column: skipping is the fix.
    assert!(found.iter().all(|failure| !failure.takes_column_fixes()));
    // A line that is not JSON at all.
    std::fs::write(
        dir.path().join("events.ndjson"),
        "{\"order_id\":1}\n{\"order_id\": oops}\n",
    )
    .unwrap();
    let (read, _) = test(&spec(dir.path(), Vec::new())).await;
    assert_eq!(
        read.failed
            .iter()
            .map(|failure| &failure.why)
            .collect::<Vec<_>>(),
        [&Why::Json {
            line: 2,
            malformed: true,
        }]
    );
}

#[tokio::test]
async fn a_test_reads_the_files_it_can_where_a_session_does_not_open() {
    let dir = exports();
    // Not Parquet at all: the file does not open.
    std::fs::write(dir.path().join("customers.parquet"), "id,name\n1,Ada\n").unwrap();
    let error = Connection::connect(&spec(dir.path(), Vec::new()), &Secrets::default())
        .await
        .err()
        .unwrap();
    assert!(
        error.to_string().contains("Can't read customers.parquet"),
        "{error}"
    );
    let (read, found) = test(&spec(dir.path(), Vec::new())).await;
    assert_eq!(read.files, 7);
    assert_eq!(
        read.failed
            .iter()
            .map(|failure| &failure.file)
            .collect::<Vec<_>>(),
        ["customers.parquet"]
    );
    assert!(matches!(read.failed[0].why, Why::Other(_)));
    assert!(!read.failed[0].takes_column_fixes());
    // The others are read and checked as ever.
    assert!(read.checks.iter().any(|name| name == "returns"));
    assert_eq!(found.len(), 1);
}

#[tokio::test]
async fn a_quote_in_a_column_s_name_or_its_value_stays_in_it() {
    let dir = tempfile::tempdir().unwrap();
    let mut text = String::from("order_id,\"size\"\"in\"\n");
    for order in 1..=30_000 {
        text.push_str(&format!("{order},{order}\n"));
    }
    // `12"`, quoted as CSV quotes it, on a row that also says ` to '`.
    text.push_str("30001,\"12\"\"\"\n");
    std::fs::write(dir.path().join("sizes.csv"), text).unwrap();
    let (_, found) = test(&spec(dir.path(), Vec::new())).await;
    assert_eq!(
        found.iter().map(|failure| &failure.why).collect::<Vec<_>>(),
        [&Why::Value {
            row: 30_001,
            column: "size\"in".into(),
            value: "12\"".into(),
            kind: "BIGINT".into(),
        }]
    );
    // Read as empty, the value as it is found: the file reads to its end.
    let empty = Override {
        file: "sizes.csv".into(),
        change: Change::Empty {
            column: "size\"in".into(),
            values: vec!["12\"".into()],
        },
    };
    let (_, found) = test(&spec(dir.path(), vec![empty])).await;
    assert!(found.is_empty(), "{found:?}");
}

#[tokio::test]
async fn a_file_that_fails_its_sample_leaves_the_next_one_read() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("a_events.ndjson"), "{\"order_id\":1}\n").unwrap();
    std::fs::write(dir.path().join("b_books.csv"), "id,title\n1,Dune\n2,Emma\n").unwrap();
    let connection = Connection::open_files_to_test(&spec(dir.path(), Vec::new()))
        .await
        .unwrap();
    // Read first, and no longer JSON once the session is open: its sample
    // fails, and the next file is read all the same.
    std::fs::write(dir.path().join("a_events.ndjson"), "{\"order_id\": oops}\n").unwrap();
    let read = connection.read_files().await.unwrap().unwrap();
    assert_eq!(
        read.failed
            .iter()
            .map(|failure| failure.file.as_str())
            .collect::<Vec<_>>(),
        ["a_events.ndjson"],
        "{:?}",
        read.failed
    );
    assert_eq!(read.checks, ["b_books"]);
}

#[tokio::test]
async fn a_test_says_a_listed_file_that_is_gone_and_reads_the_others() {
    let dir = exports();
    let gone = dir.path().join("archive.csv");
    let mut spec = ConnectSpec::files(Source::Files(vec![
        dir.path().join("orders.csv"),
        gone.clone(),
    ]));
    spec.csv = Reading::default();
    // Connecting still fails on it, as in run 1.
    let error = Connection::connect(&spec, &Secrets::default())
        .await
        .err()
        .unwrap();
    assert!(error.to_string().contains("no such file"), "{error}");
    let (read, _) = test(&spec).await;
    assert_eq!(read.files, 2);
    assert_eq!(read.failed.len(), 1, "{:?}", read.failed);
    assert_eq!(read.failed[0].file, "archive.csv");
    assert_eq!(read.failed[0].path, gone);
    assert_eq!(read.failed[0].format, Format::Csv);
    assert!(!read.failed[0].takes_column_fixes());
    assert_eq!(read.checks, ["orders"]);
}

#[tokio::test]
async fn a_gone_file_keeps_its_place_in_the_list_among_the_failures() {
    let dir = exports();
    // Not Parquet at all: it does not open.
    std::fs::write(dir.path().join("customers.parquet"), "id,name\n1,Ada\n").unwrap();
    let spec = ConnectSpec::files(Source::Files(vec![
        dir.path().join("archive.csv"),
        dir.path().join("customers.parquet"),
        dir.path().join("orders.csv"),
    ]));
    let (read, _) = test(&spec).await;
    assert_eq!(
        read.failed
            .iter()
            .map(|failure| failure.file.as_str())
            .collect::<Vec<_>>(),
        ["archive.csv", "customers.parquet"]
    );
}
