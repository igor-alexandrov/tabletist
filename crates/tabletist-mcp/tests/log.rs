//! The log: one entry for each call, with what was asked and what came of
//! it, and nothing a database holds.

// `allow-unwrap-in-tests` does not cover helpers in an integration-test crate.
#![allow(clippy::unwrap_used)]

mod support;

use serde_json::json;
use support::{Agent, Shop, entries, server};
use tabletist_mcp::Entry;

const DEV: &str = "Bookshop · dev";

/// What every entry of these tests has.
fn entry(tool: &str, result: &str) -> Entry {
    Entry {
        agent: "Claude Code".into(),
        project: Some("tabletist-repo".into()),
        transport: "stdio".into(),
        tool: tool.into(),
        result: result.into(),
        ..Entry::default()
    }
}

/// An entry without what differs from run to run, which is checked apart.
fn steady(mut entry: Entry) -> Entry {
    assert!(entry.at_ms > 1_700_000_000_000, "{}", entry.at_ms);
    assert!(entry.took_ms.is_some_and(|took| took >= 0));
    assert!(!entry.session.is_empty());
    (entry.at_ms, entry.took_ms, entry.bytes) = (0, None, None);
    entry.session.clear();
    entry
}

#[tokio::test]
async fn every_call_is_an_entry_with_what_was_asked_and_what_came_of_it() {
    let shop = Shop::new();
    let (server, log) = server(&shop);
    let agent = Agent::connect(&server).await;
    let on_dev = Some(format!("id:{DEV}"));
    agent.call("list_connections", json!({})).await;
    agent
        .call("describe_schema", json!({ "connection": DEV }))
        .await;
    agent
        .call(
            "describe_schema",
            json!({ "connection": DEV, "tables": ["book_covers", "publishers"] }),
        )
        .await;
    let answered = agent
        .call(
            "run_query",
            json!({
                "connection": DEV,
                "sql": "  SELECT id, kind FROM book_covers WHERE book_id = ? ORDER BY id;  ",
                "params": [1],
            }),
        )
        .await;
    agent
        .call(
            "run_query",
            json!({ "connection": DEV, "sql": "DELETE FROM book_covers WHERE id = 9" }),
        )
        .await;
    agent
        .call(
            "run_query",
            json!({ "connection": "Payroll", "sql": "SELECT * FROM salaries" }),
        )
        .await;
    agent
        .call(
            "run_query",
            json!({ "connection": DEV, "sql": "SELECT nmae FROM authors" }),
        )
        .await;
    agent.close().await;

    let read = entries(&log).await;
    let sessions: Vec<&str> = read.iter().map(|entry| entry.session.as_str()).collect();
    assert!(sessions.iter().all(|session| *session == sessions[0]));
    let bytes = read[3].bytes;
    assert_eq!(bytes, Some(answered.text.len() as i64));
    let rows = answered.answer()["row_count"].as_i64();
    assert!(rows.is_some_and(|rows| rows > 0));
    let read: Vec<Entry> = read.into_iter().map(steady).collect();
    let expected = [
        Entry {
            rows: Some(1),
            ..entry("list_connections", "ok")
        },
        Entry {
            connection_id: on_dev.clone(),
            connection: Some(DEV.into()),
            rows: Some(9),
            ..entry("describe_schema", "ok")
        },
        Entry {
            connection_id: on_dev.clone(),
            connection: Some(DEV.into()),
            request: Some("book_covers, publishers".into()),
            rows: Some(2),
            columns: Some(7),
            ..entry("describe_schema", "ok")
        },
        Entry {
            connection_id: on_dev.clone(),
            connection: Some(DEV.into()),
            // As the agent sent it.
            sql: Some("  SELECT id, kind FROM book_covers WHERE book_id = ? ORDER BY id;  ".into()),
            params: Some("[1]".into()),
            ran_as: Some("read-only transaction".into()),
            rows,
            matched: rows,
            columns: Some(2),
            ..entry("run_query", "ok")
        },
        Entry {
            connection_id: on_dev.clone(),
            connection: Some(DEV.into()),
            sql: Some("DELETE FROM book_covers WHERE id = 9".into()),
            code: Some("not_read_only".into()),
            message: Some("run_query only runs reads, and this is a DELETE.".into()),
            ..entry("run_query", "refused")
        },
        Entry {
            // A name that is no connection's is in the sentence alone.
            sql: Some("SELECT * FROM salaries".into()),
            code: Some("unknown_connection".into()),
            message: Some("No connection named Payroll. list_connections has the names.".into()),
            ..entry("run_query", "refused")
        },
        Entry {
            connection_id: on_dev,
            connection: Some(DEV.into()),
            sql: Some("SELECT nmae FROM authors".into()),
            code: Some("query_failed".into()),
            // The database's words are the agent's, and can quote a row.
            message: Some("The database refused the statement.".into()),
            ran_as: Some("read-only transaction".into()),
            ..entry("run_query", "error")
        },
    ];
    assert_eq!(read.len(), expected.len());
    for (read, expected) in read.iter().zip(&expected) {
        assert_eq!(read, expected);
    }
}

#[tokio::test]
async fn a_refusal_is_an_entry_with_its_code_whatever_refused_the_call() {
    let shop = Shop::new();
    shop.list(
        "Bookshop · gone",
        &shop.path("no-such-file.db"),
        support::LIMITS,
    );
    let (server, log) = server(&shop);
    let agent = Agent::connect(&server).await;
    let many: Vec<String> = (0..21).map(|n| format!("t{n}")).collect();
    let calls = [
        (
            "run_query",
            json!({ "connection": "Bookshop · gone", "sql": "SELECT 1" }),
        ),
        (
            "describe_schema",
            json!({ "connection": DEV, "tables": many }),
        ),
        (
            "run_query",
            json!({ "connection": DEV, "sql": "SELECT ?", "params": [{ "a": 1 }] }),
        ),
    ];
    for (tool, arguments) in calls {
        assert!(agent.call(tool, arguments).await.is_error, "{tool}");
    }
    agent.close().await;
    let read: Vec<(String, Option<String>, Option<String>)> = entries(&log)
        .await
        .into_iter()
        .map(|entry| (entry.result, entry.code, entry.message))
        .collect();
    let entry = |result: &str, code: &str, message: &str| {
        (
            result.to_owned(),
            Some(code.to_owned()),
            Some(message.to_owned()),
        )
    };
    assert_eq!(
        read,
        [
            entry(
                "error",
                "connection_unavailable",
                "Tabletist could not open Bookshop · gone. Open it in Tabletist to see why.",
            ),
            entry(
                "refused",
                "too_large",
                "describe_schema describes at most 20 tables in one call.",
            ),
            entry(
                "refused",
                "invalid_params",
                "Parameter 1 is an object. A parameter is a string, a number, true, false or null.",
            ),
        ]
    );
}

#[tokio::test]
async fn an_answer_that_was_cut_does_not_say_how_many_rows_there_were() {
    let shop = Shop::new();
    let few = tabletist_db::agent::Limits {
        rows: 3,
        ..support::LIMITS
    };
    shop.list("Bookshop · few", &shop.bookshop(), few);
    let (server, log) = server(&shop);
    let agent = Agent::connect(&server).await;
    agent
        .call(
            "run_query",
            json!({ "connection": "Bookshop · few", "sql": "SELECT id FROM books" }),
        )
        .await;
    agent.close().await;
    let read = entries(&log).await;
    assert_eq!((read[0].rows, read[0].matched), (Some(3), None));
}

#[tokio::test]
async fn the_log_holds_nothing_a_database_holds() {
    // A value no statement of the test spells out.
    let marker = ["zq", "marker", "7f3a"].join("-");
    let shop = Shop::new();
    shop.change(&format!(
        "INSERT INTO customers (id, name, email) VALUES (900, '{marker}', '{marker}@example.com');
         ALTER TABLE customers ADD COLUMN tier TEXT DEFAULT '{marker}';"
    ));
    let (server, log) = server(&shop);
    let agent = Agent::connect(&server).await;
    let calls = [
        ("list_connections", json!({})),
        ("describe_schema", json!({ "connection": DEV })),
        (
            "describe_schema",
            json!({ "connection": DEV, "tables": ["customers"] }),
        ),
        (
            "run_query",
            json!({ "connection": DEV, "sql": "SELECT * FROM customers" }),
        ),
        (
            "run_query",
            json!({ "connection": DEV, "sql": "SELECT name FROM customers WHERE id = ?", "params": [900] }),
        ),
        (
            "run_query",
            json!({ "connection": DEV, "sql": "SELECT * FROM customer_names" }),
        ),
        (
            "run_query",
            json!({ "connection": DEV, "sql": "EXPLAIN QUERY PLAN SELECT * FROM customers" }),
        ),
    ];
    for (tool, arguments) in calls {
        assert!(!agent.call(tool, arguments).await.is_error, "{tool}");
    }
    // A database's own error can quote a row: SQLite says which JSON path
    // it could not read, and here the path is a customer's name. The agent
    // may read that. The log keeps the error without its words.
    let failed = agent
        .call(
            "run_query",
            json!({ "connection": DEV, "sql": "SELECT json_extract('{}', name) FROM customers WHERE id = 900" }),
        )
        .await;
    assert_eq!(failed.refused(), "query_failed");
    assert!(failed.text.contains(marker.as_str()), "{}", failed.text);
    // The agent was sent the value: it is the log that must not have it.
    let sent = agent.received();
    assert!(sent.matches(marker.as_str()).count() >= 5, "{sent}");
    agent.close().await;
    assert_eq!(entries(&log).await.len(), 8);
    drop((server, log));

    let mut files = 0;
    for file in std::fs::read_dir(shop.path("")).unwrap() {
        let path = file.unwrap().path();
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        if !name.starts_with("agents.sqlite3") {
            continue;
        }
        files += 1;
        let bytes = std::fs::read(&path).unwrap();
        let found = bytes
            .windows(marker.len())
            .any(|window| window == marker.as_bytes());
        assert!(!found, "{name} holds a row's value");
    }
    assert!(files >= 1);
}
