//! How a Files connection reads its CSV and TSV files: the connection's
//! Reading CSV (header, delimiter, column types) and a file's overrides,
//! as the call each view is made with says them and as the rows read.

// `allow-unwrap-in-tests` does not cover helpers in an integration-test crate.
#![allow(clippy::unwrap_used)]

use std::path::Path;

use tabletist_db::files::{Change, Delimiter, Header, Override, Reading, Source, Types};
use tabletist_db::{
    ConnectSpec, Connection, Dialect, ObjectRef, ScriptMode, Secrets, StatementOutcome, StopFlag,
    Value, sql,
};

async fn open(dir: &Path, reading: Reading) -> Result<Connection, tabletist_db::Error> {
    let mut spec = ConnectSpec::files(Source::Folder(dir.to_owned()));
    spec.csv = reading;
    Connection::connect(&spec, &Secrets::default()).await
}

/// The Bookshop's exports in a folder of their own.
fn exports() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    tabletist_db::fixtures::write_bookshop_files(dir.path()).unwrap();
    dir
}

/// A folder holding only `returns.csv`: 30,000 returns, then one whose
/// `returned_at` is `N/A`, past the rows a reader samples for types.
fn late_returns() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let mut text = String::from("order_id,returned_at,reason\n");
    for order in 1..=30_000 {
        let day = order % 28 + 1;
        text.push_str(&format!("{order},2026-02-{day:02},damaged\n"));
    }
    text.push_str("30001,N/A,lost in the post\n");
    std::fs::write(dir.path().join("returns.csv"), text).unwrap();
    dir
}

/// The statement a table's view was made with, without the file's path.
async fn made(connection: &Connection, table: &str, dir: &Path) -> String {
    let ddl = connection
        .ddl(&ObjectRef::new("main", table))
        .await
        .unwrap();
    // The folder and the separator after it, which is `\\` on Windows.
    ddl.text()
        .replace(&dir.display().to_string(), "<dir>")
        .replace("<dir>\\", "<dir>/")
        .trim_end_matches(';')
        .to_owned()
}

/// The rows of a script of one statement, or its error.
async fn read(connection: &Connection, text: &str) -> Result<Vec<Vec<Value>>, String> {
    let statements = sql::statements(Dialect::DuckDb, text);
    let outcome = connection
        .run_script(&statements, 100, ScriptMode::ReadOnly, &StopFlag::new())
        .await
        .map_err(|error| error.to_string())?;
    match outcome.results.into_iter().next().unwrap().outcome {
        StatementOutcome::Rows { rows, .. } => Ok(rows),
        StatementOutcome::Error { error, .. } => Err(error.to_string()),
        other => panic!("{text}: {other:?}"),
    }
}

fn text_of(value: &Value) -> String {
    match value {
        Value::Text(text) => text.to_string(),
        Value::Int(number) => number.to_string(),
        other => format!("{other:?}"),
    }
}

fn change(file: &str, change: Change) -> Override {
    Override {
        file: file.to_owned(),
        change,
    }
}

#[tokio::test]
async fn each_option_changes_the_call_a_csv_and_a_tsv_are_made_with() {
    let dir = exports();
    let cases = [
        (
            Reading::default(),
            "read_csv('<dir>/orders.csv', auto_detect = true, sample_size = 20000)",
            "read_csv('<dir>/authors.tsv', auto_detect = true, sample_size = 20000, delim = '\\t')",
        ),
        (
            Reading {
                header: Header::Yes,
                ..Reading::default()
            },
            "read_csv('<dir>/orders.csv', header = true, auto_detect = true, sample_size = 20000)",
            "read_csv('<dir>/authors.tsv', header = true, auto_detect = true, sample_size = 20000, \
             delim = '\\t')",
        ),
        (
            Reading {
                header: Header::No,
                types: Types::All,
                ..Reading::default()
            },
            "read_csv('<dir>/orders.csv', header = false, auto_detect = true, sample_size = -1)",
            "read_csv('<dir>/authors.tsv', header = false, auto_detect = true, sample_size = -1, \
             delim = '\\t')",
        ),
        // A TSV keeps its tab whatever the CSVs are parted by.
        (
            Reading {
                delimiter: Delimiter::Semicolon,
                types: Types::Text,
                ..Reading::default()
            },
            "read_csv('<dir>/orders.csv', auto_detect = true, all_varchar = true, delim = ';')",
            "read_csv('<dir>/authors.tsv', auto_detect = true, all_varchar = true, delim = '\\t')",
        ),
        (
            Reading {
                delimiter: Delimiter::Tab,
                ..Reading::default()
            },
            "read_csv('<dir>/orders.csv', auto_detect = true, sample_size = 20000, delim = '\\t')",
            "read_csv('<dir>/authors.tsv', auto_detect = true, sample_size = 20000, delim = '\\t')",
        ),
        (
            Reading {
                delimiter: Delimiter::Pipe,
                ..Reading::default()
            },
            "read_csv('<dir>/orders.csv', auto_detect = true, sample_size = 20000, delim = '|')",
            "read_csv('<dir>/authors.tsv', auto_detect = true, sample_size = 20000, delim = '\\t')",
        ),
    ];
    for (reading, orders, authors) in cases {
        // A delimiter that is not the file's reads one column, never fails.
        let connection = open(dir.path(), reading.clone()).await.unwrap();
        assert_eq!(
            made(&connection, "orders", dir.path()).await,
            format!("CREATE VIEW orders AS SELECT * FROM {orders}"),
            "{reading:?}"
        );
        assert_eq!(
            made(&connection, "authors", dir.path()).await,
            format!("CREATE VIEW authors AS SELECT * FROM {authors}"),
            "{reading:?}"
        );
        // The other formats read as they did.
        assert!(
            made(&connection, "customers", dir.path())
                .await
                .ends_with("read_parquet('<dir>/customers.parquet')")
        );
    }
}

#[tokio::test]
async fn a_value_past_the_sample_fails_the_default_reading_and_each_way_reads_it() {
    let dir = late_returns();
    let count = "SELECT typeof(returned_at), count(*), count(returned_at) FROM returns \
                 GROUP BY 1";
    let read_as = |rows: Vec<Vec<Value>>| -> Vec<String> { rows[0].iter().map(text_of).collect() };

    let connection = open(dir.path(), Reading::default()).await.unwrap();
    let error = read(&connection, count).await.unwrap_err();
    assert!(error.contains("N/A"), "{error}");

    let all = Reading {
        types: Types::All,
        ..Reading::default()
    };
    let connection = open(dir.path(), all).await.unwrap();
    assert_eq!(
        read_as(read(&connection, count).await.unwrap()),
        ["VARCHAR", "30001", "30001"]
    );

    let text = Reading {
        types: Types::Text,
        ..Reading::default()
    };
    let connection = open(dir.path(), text).await.unwrap();
    assert_eq!(
        read_as(read(&connection, count).await.unwrap()),
        ["VARCHAR", "30001", "30001"]
    );

    let as_text = Reading {
        overrides: vec![change(
            "returns.csv",
            Change::Text {
                column: "returned_at".into(),
            },
        )],
        ..Reading::default()
    };
    let connection = open(dir.path(), as_text).await.unwrap();
    assert_eq!(
        read_as(read(&connection, count).await.unwrap()),
        ["VARCHAR", "30001", "30001"]
    );
    assert_eq!(
        made(&connection, "returns", dir.path()).await,
        "CREATE VIEW returns AS SELECT * FROM read_csv('<dir>/returns.csv', auto_detect = true, \
         sample_size = 20000, types = {'returned_at': 'VARCHAR'})"
    );

    // Empty: the column stays a date, and its N/A is NULL.
    let as_empty = Reading {
        overrides: vec![change(
            "returns.csv",
            Change::Empty {
                column: "returned_at".into(),
                values: vec!["N/A".into()],
            },
        )],
        ..Reading::default()
    };
    let connection = open(dir.path(), as_empty).await.unwrap();
    assert_eq!(
        read_as(read(&connection, count).await.unwrap()),
        ["DATE", "30001", "30000"]
    );
    assert_eq!(
        made(&connection, "returns", dir.path()).await,
        "CREATE VIEW returns AS SELECT * REPLACE (CAST(NULLIF(returned_at, 'N/A') AS DATE) AS \
         returned_at) FROM read_csv('<dir>/returns.csv', auto_detect = true, sample_size = 20000, \
         types = {'returned_at': 'VARCHAR'})"
    );

    let skipped = Reading {
        overrides: vec![change("returns.csv", Change::Skip)],
        ..Reading::default()
    };
    let connection = open(dir.path(), skipped).await.unwrap();
    let error = read(&connection, count).await.unwrap_err();
    assert!(error.contains("returns"), "{error}");
    assert_eq!(connection.read_files().await.unwrap().unwrap().files, 0);
}

#[tokio::test]
async fn an_override_counts_only_for_its_own_file_and_a_column_it_has() {
    let dir = exports();
    let reading = Reading {
        overrides: vec![
            // No such column in returns.csv: it does nothing.
            change(
                "returns.csv",
                Change::Text {
                    column: "shipped_at".into(),
                },
            ),
            // A file the connection does not open.
            change("archive.csv", Change::Skip),
            // Read as text and as empty: text wins, no cast back.
            change(
                "returns.csv",
                Change::Text {
                    column: "reason".into(),
                },
            ),
            change(
                "returns.csv",
                Change::Empty {
                    column: "reason".into(),
                    values: vec!["damaged".into(), "lost".into()],
                },
            ),
            change(
                "returns.csv",
                Change::Empty {
                    column: "returned_at".into(),
                    values: vec!["N/A".into(), "-".into()],
                },
            ),
        ],
        ..Reading::default()
    };
    let connection = open(dir.path(), reading).await.unwrap();
    assert_eq!(
        made(&connection, "returns", dir.path()).await,
        "CREATE VIEW returns AS SELECT * REPLACE (CAST(NULLIF(NULLIF(returned_at, 'N/A'), '-') \
         AS DATE) AS returned_at) FROM read_csv('<dir>/returns.csv', auto_detect = true, \
         sample_size = 20000, types = {'reason': 'VARCHAR', 'returned_at': 'VARCHAR'})"
    );
    // orders.csv is read as it was.
    assert_eq!(
        made(&connection, "orders", dir.path()).await,
        "CREATE VIEW orders AS SELECT * FROM read_csv('<dir>/orders.csv', auto_detect = true, \
         sample_size = 20000)"
    );
    // The Structure view says the call's own options.
    let structure = connection
        .describe(&ObjectRef::new("main", "returns"))
        .await
        .unwrap();
    assert_eq!(
        structure.file.unwrap().options,
        "auto_detect = true, sample_size = 20000, types = {'reason': 'VARCHAR', 'returned_at': \
         'VARCHAR'}"
    );
    let rows = read(
        &connection,
        "SELECT count(*), count(returned_at) FROM returns",
    )
    .await
    .unwrap();
    assert_eq!(rows[0][0], rows[0][1]);
}

/// A folder holding only `returns.csv` whose `returned_at` has `N/A` and
/// `-` among its first rows, inside the sample.
fn early_returns() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let mut text = String::from("order_id,returned_at,reason\n");
    text.push_str("1,N/A,lost in the post\n2,-,damaged\n");
    for order in 3..=40 {
        let day = order % 28 + 1;
        text.push_str(&format!("{order},2026-02-{day:02},damaged\n"));
    }
    std::fs::write(dir.path().join("returns.csv"), text).unwrap();
    dir
}

#[tokio::test]
async fn values_read_as_empty_keep_the_column_its_type_wherever_they_are() {
    let count = "SELECT typeof(returned_at), count(*), count(returned_at) FROM returns \
                 GROUP BY 1";
    let read_as = |rows: Vec<Vec<Value>>| -> Vec<String> { rows[0].iter().map(text_of).collect() };
    let empty = |column: &str, values: &[&str]| {
        change(
            "returns.csv",
            Change::Empty {
                column: column.into(),
                values: values.iter().map(|value| (*value).to_owned()).collect(),
            },
        )
    };
    // Inside the sample, two overrides of one column, named in another
    // case: one cast, the column a date.
    let dir = early_returns();
    let reading = Reading {
        overrides: vec![empty("Returned_At", &["N/A"]), empty("returned_at", &["-"])],
        ..Reading::default()
    };
    let connection = open(dir.path(), reading).await.unwrap();
    assert_eq!(
        read_as(read(&connection, count).await.unwrap()),
        ["DATE", "40", "38"]
    );
    assert_eq!(
        made(&connection, "returns", dir.path()).await,
        "CREATE VIEW returns AS SELECT * REPLACE (CAST(NULLIF(NULLIF(returned_at, 'N/A'), '-') \
         AS DATE) AS returned_at) FROM read_csv('<dir>/returns.csv', auto_detect = true, \
         sample_size = 20000, types = {'returned_at': 'VARCHAR'})"
    );
    // Every row read for types: still a date.
    let dir = late_returns();
    let reading = Reading {
        types: Types::All,
        overrides: vec![empty("returned_at", &["N/A"])],
        ..Reading::default()
    };
    let connection = open(dir.path(), reading).await.unwrap();
    assert_eq!(
        read_as(read(&connection, count).await.unwrap()),
        ["DATE", "30001", "30000"]
    );
    // No values: nothing changes.
    let dir = early_returns();
    let reading = Reading {
        overrides: vec![empty("returned_at", &[])],
        ..Reading::default()
    };
    let connection = open(dir.path(), reading).await.unwrap();
    assert_eq!(
        made(&connection, "returns", dir.path()).await,
        "CREATE VIEW returns AS SELECT * FROM read_csv('<dir>/returns.csv', auto_detect = true, \
         sample_size = 20000)"
    );
}

/// A folder holding only `returns.csv`, its text in full.
fn returns_of(text: &str) -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("returns.csv"), text).unwrap();
    dir
}

#[tokio::test]
async fn a_column_read_as_empty_is_typed_by_its_own_values_and_dates_keep_their_format() {
    let empty = |column: &str, value: &str| {
        change(
            "returns.csv",
            Change::Empty {
                column: column.into(),
                values: vec![value.to_owned()],
            },
        )
    };
    let all = |rows: Vec<Vec<Value>>| -> Vec<Vec<String>> {
        rows.iter()
            .map(|row| row.iter().map(text_of).collect())
            .collect()
    };
    // Day-first dates: still dates, read in the format the sniffer found.
    let dir = returns_of("order_id,returned_at\n1,31/01/2026\n2,N/A\n3,15/02/2026\n");
    let reading = Reading {
        overrides: vec![empty("returned_at", "N/A")],
        ..Reading::default()
    };
    let connection = open(dir.path(), reading).await.unwrap();
    assert_eq!(
        all(read(
            &connection,
            "SELECT typeof(returned_at), CAST(returned_at AS VARCHAR) FROM returns ORDER BY order_id"
        )
        .await
        .unwrap()),
        [
            ["DATE", "2026-01-31"],
            ["DATE", "Null"],
            ["DATE", "2026-02-15"],
        ]
    );
    // One column's empty values do not type another: `b` keeps its `N/A`,
    // so it stays text, while `a` is a number.
    let dir = returns_of("order_id,a,b\n1,N/A,N/A\n2,42,42\n3,43,-\n");
    let reading = Reading {
        overrides: vec![empty("a", "N/A"), empty("b", "-")],
        ..Reading::default()
    };
    let connection = open(dir.path(), reading).await.unwrap();
    assert_eq!(
        all(read(
            &connection,
            "SELECT typeof(a), typeof(b), count(a), count(b) FROM returns GROUP BY 1, 2"
        )
        .await
        .unwrap()),
        [["BIGINT", "VARCHAR", "2", "2"]]
    );
    // An override of a column the file does not have changes nothing:
    // `N/A` in returned_at keeps it text.
    let dir = returns_of("order_id,returned_at\n1,2026-01-31\n2,N/A\n3,-\n");
    let reading = Reading {
        overrides: vec![empty("shipped_at", "N/A"), empty("returned_at", "-")],
        ..Reading::default()
    };
    let connection = open(dir.path(), reading).await.unwrap();
    assert_eq!(
        all(read(
            &connection,
            "SELECT typeof(returned_at), count(*), count(returned_at) FROM returns GROUP BY 1"
        )
        .await
        .unwrap()),
        [["VARCHAR", "3", "2"]]
    );
}

#[tokio::test]
async fn an_override_by_path_is_of_that_file_alone_and_a_skipped_file_may_be_gone() {
    let current = tempfile::tempdir().unwrap();
    let archive = tempfile::tempdir().unwrap();
    std::fs::write(current.path().join("orders.csv"), "id\n1\n").unwrap();
    std::fs::write(archive.path().join("orders.csv"), "id\n2\n").unwrap();
    let kept = current.path().join("orders.csv");
    let gone = archive.path().join("orders.csv");
    let list = Source::Files(vec![kept.clone(), gone.clone()]);
    // A path skips that file alone.
    let mut spec = ConnectSpec::files(list.clone());
    spec.csv.overrides = vec![change(&gone.display().to_string(), Change::Skip)];
    let connection = Connection::connect(&spec, &Secrets::default())
        .await
        .unwrap();
    let tables = read(
        &connection,
        "SELECT count(*) FROM duckdb_views() WHERE NOT internal",
    )
    .await
    .unwrap();
    assert_eq!(text_of(&tables[0][0]), "1");
    // The one left is the current file, not the archive's.
    let ids = read(&connection, "SELECT id FROM orders").await.unwrap();
    assert_eq!(text_of(&ids[0][0]), "1");
    // The skipped file is gone: the connection still opens.
    std::fs::remove_file(&gone).unwrap();
    Connection::connect(&spec, &Secrets::default())
        .await
        .unwrap();
    // A name is of both files.
    let mut spec = ConnectSpec::files(Source::Files(vec![kept.clone()]));
    spec.csv.overrides = vec![change("orders.csv", Change::Skip)];
    let connection = Connection::connect(&spec, &Secrets::default())
        .await
        .unwrap();
    let tables = read(
        &connection,
        "SELECT count(*) FROM duckdb_views() WHERE NOT internal",
    )
    .await
    .unwrap();
    assert_eq!(text_of(&tables[0][0]), "0");
}

#[tokio::test]
async fn a_value_read_as_empty_that_is_the_header_s_word_keeps_the_column_s_type() {
    // `amount` heads the column and is read as empty in it: DuckDB names
    // the column anew while it looks for its type, which is found by its
    // place, not its name.
    for (value, kind) in [("amount", "BIGINT"), ("-", "BIGINT")] {
        let dir = returns_of("amount\namount\n42\n43\n-\n");
        let reading = Reading {
            header: Header::Yes,
            overrides: vec![change(
                "returns.csv",
                Change::Empty {
                    column: "amount".into(),
                    values: vec![value.to_owned(), "-".into(), "amount".into()],
                },
            )],
            ..Reading::default()
        };
        let connection = open(dir.path(), reading).await.unwrap();
        let rows = read(&connection, "SELECT DISTINCT typeof(amount) FROM returns")
            .await
            .unwrap();
        assert_eq!(text_of(&rows[0][0]), kind, "{value}");
    }
}

#[tokio::test]
async fn a_file_with_no_header_row_counts_every_row() {
    let dir = returns_of("10,20\n30,40\n");
    let connection = open(dir.path(), Reading::default()).await.unwrap();
    let objects = connection.list_objects("main").await.unwrap();
    assert_eq!(objects[0].estimated_rows, Some(2));
}

/// A listed file that is skipped is not looked at: one the system cannot
/// look at (a link to itself) does not stop the others opening.
#[cfg(unix)]
#[tokio::test]
async fn a_skipped_file_that_cannot_be_looked_at_is_no_matter() {
    let dir = tempfile::tempdir().unwrap();
    let kept = dir.path().join("orders.csv");
    std::fs::write(&kept, "id\n1\n").unwrap();
    let looped = dir.path().join("returns.csv");
    std::os::unix::fs::symlink(&looped, &looped).unwrap();
    let mut spec = ConnectSpec::files(Source::Files(vec![kept, looped.clone()]));
    assert!(
        Connection::connect(&spec, &Secrets::default())
            .await
            .is_err()
    );
    spec.csv.overrides = vec![change("returns.csv", Change::Skip)];
    Connection::connect(&spec, &Secrets::default())
        .await
        .unwrap();
}
