//! Where a database is and what opens it never leaves the app: not to an
//! agent, not into the log of calls, and not into the app's own log.

// `allow-unwrap-in-tests` does not cover helpers in an integration-test crate.
#![allow(clippy::unwrap_used)]

mod support;

use std::path::{Path, PathBuf};
use std::sync::{Mutex, Once};

use serde_json::json;
use support::{Agent, LIMITS, Shop, entries, request, server};

/// The app's log, caught: every record any crate writes while a test runs.
static LINES: Mutex<Vec<Line>> = Mutex::new(Vec::new());

#[derive(Debug, Clone)]
struct Line {
    level: log::Level,
    target: String,
    text: String,
}

struct Caught;

impl log::Log for Caught {
    fn enabled(&self, _metadata: &log::Metadata) -> bool {
        true
    }

    fn log(&self, record: &log::Record) {
        LINES.lock().unwrap().push(Line {
            level: record.level(),
            target: record.target().to_owned(),
            text: record.args().to_string(),
        });
    }

    fn flush(&self) {}
}

fn catch_the_log() {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        log::set_logger(&Caught).unwrap();
        log::set_max_level(log::LevelFilter::Trace);
    });
}

/// Whether a line is the SDK's, or the parser's the check reads with.
/// Both write what a client sent: the SDK a client's name and its
/// notifications from `info` up and every request whole at `debug`, the
/// parser each word of a statement. So the app's log takes no line of
/// theirs. That is the app's filter to keep (`src/entrypoint.rs`), from the
/// change that links this crate into the app.
fn not_the_apps_to_write(line: &Line) -> bool {
    ["rmcp", "sqlparser", "tracing"]
        .iter()
        .any(|theirs| line.target.starts_with(theirs))
}

#[tokio::test]
async fn nothing_says_where_a_database_is() {
    catch_the_log();
    let shop = Shop::new();
    // Two that cannot be opened: no such file, and a file that is no
    // database. The driver's own error names the path of each.
    let nowhere = shop.path("zq-nowhere-5e1c.db");
    let garbage = shop.path("zq-garbage-5e1c.db");
    std::fs::write(&garbage, "not a database, whatever its name says").unwrap();
    shop.list("Bookshop · gone", &nowhere, LIMITS);
    shop.list("Bookshop · garbage", &garbage, LIMITS);
    let (server, log) = server(&shop);
    // A client's name is its own to choose, and so is every word it sends.
    let agent = Agent::connect_as(&server, support::client("zq-client-5e1c", None)).await;

    let statement = "SELECT name FROM authors WHERE name <> 'zq-literal-5e1c'";
    let mut refused = Vec::new();
    for connection in [
        "Bookshop · dev",
        "Bookshop · gone",
        "Bookshop · garbage",
        "Payroll",
    ] {
        let calls = [
            ("list_connections", json!({})),
            ("describe_schema", json!({ "connection": connection })),
            (
                "describe_schema",
                json!({ "connection": connection, "tables": ["authors", "nope"] }),
            ),
            (
                "run_query",
                json!({ "connection": connection, "sql": statement }),
            ),
            (
                "run_query",
                json!({ "connection": connection, "sql": "SELECT ?", "params": ["zq-value-5e1c"] }),
            ),
            (
                "run_query",
                json!({ "connection": connection, "sql": "SELECT nmae FROM authors" }),
            ),
            (
                "run_query",
                json!({ "connection": connection, "sql": "DROP TABLE authors" }),
            ),
        ];
        for (tool, arguments) in calls {
            let called = agent.call(tool, arguments).await;
            if called.is_error {
                refused.push((connection, called.refused().to_owned(), called.text.clone()));
            }
        }
        // And calls that are no tool's.
        for (tool, arguments) in [
            (
                "run_query",
                json!({ "connection": connection, "sql": [statement] }),
            ),
            ("open_file", json!({ "path": "anywhere" })),
        ] {
            assert!(
                agent
                    .client
                    .call_tool(request(tool, arguments))
                    .await
                    .is_err()
            );
        }
    }
    // The two that cannot be opened say one thing, and the same thing.
    for name in ["Bookshop · gone", "Bookshop · garbage"] {
        let said: Vec<&(&str, String, String)> = refused
            .iter()
            .filter(|(connection, code, _)| *connection == name && code == "connection_unavailable")
            .collect();
        assert_eq!(said.len(), 5, "{refused:?}");
        for (_, _, text) in said {
            assert_eq!(
                text,
                &format!("Tabletist could not open {name}. Open it in Tabletist to see why.")
            );
        }
    }

    let sent = agent.received();
    agent.close().await;
    let logged = format!("{:?}", entries(&log).await);
    let lines = LINES.lock().unwrap().clone();
    // The log was caught: this crate's line for a call is in it.
    let ours: Vec<&Line> = lines
        .iter()
        .filter(|line| !not_the_apps_to_write(line))
        .collect();
    let a_call = "agent call: run_query refused (not_allowed) in ";
    assert!(ours.iter().any(|line| line.text.starts_with(a_call)));

    // Where the files are: the directory, and each file's own name.
    let directory = shop
        .path("")
        .to_string_lossy()
        .trim_end_matches('/')
        .to_owned();
    let private = [
        directory.as_str(),
        "zq-nowhere-5e1c",
        "zq-garbage-5e1c",
        "bookshop.db",
    ];
    for word in private {
        assert!(!sent.contains(word), "an agent was sent {word}");
        assert!(!logged.contains(word), "the log of calls holds {word}");
        for line in &lines {
            assert!(
                !line.text.contains(word),
                "the app's log holds {word}: {line:?}"
            );
        }
    }
    // What an agent sent is in the log of calls, on purpose, and in no
    // line the app writes: not at any level, and from no crate.
    assert!(logged.contains("zq-literal-5e1c") && logged.contains("zq-value-5e1c"));
    assert!(logged.contains("zq-client-5e1c"));
    let sent_words = [
        "zq-literal-5e1c",
        "zq-value-5e1c",
        "zq-client-5e1c",
        "SELECT",
        "authors",
        "Payroll",
    ];
    for line in &ours {
        for word in sent_words {
            assert!(
                !line.text.contains(word),
                "the app's log holds {word}: {line:?}"
            );
        }
    }
    // Which is why the SDK's lines are none of the app's: it writes who
    // connected at `info`, and at `debug` the statement.
    let theirs = |level: log::Level, word: &str| {
        lines.iter().any(|line| {
            line.target.starts_with("rmcp") && line.level == level && line.text.contains(word)
        })
    };
    assert!(theirs(log::Level::Info, "zq-client-5e1c"));
    assert!(theirs(log::Level::Debug, "zq-literal-5e1c"));
}

/// Every Rust file under `dir`.
fn sources(dir: &Path, into: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            sources(&path, into);
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            into.push(path);
        }
    }
}

#[test]
fn the_crate_never_names_what_opens_a_database() {
    let mut files = Vec::new();
    sources(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("src"),
        &mut files,
    );
    assert!(files.len() >= 8, "{files:?}");
    for file in files {
        let text = std::fs::read_to_string(&file).unwrap();
        for name in [
            "ConnectSpec",
            "Secrets",
            "HostKeys",
            "connect_with",
            "Connection::connect",
        ] {
            assert!(
                !text.contains(name),
                "{} names {name}: the server reaches a database through its gateway alone",
                file.display()
            );
        }
    }
}
