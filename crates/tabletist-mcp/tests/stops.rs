//! How a call ends that does not end by itself: the connection's time, the
//! client's cancel, a client that is gone, and the person's stop.

// `allow-unwrap-in-tests` does not cover helpers in an integration-test crate.
#![allow(clippy::unwrap_used)]

mod support;

use std::time::{Duration, Instant};

use rmcp::model::{CallToolRequest, ClientRequest};
use rmcp::service::{PeerRequestOptions, ServiceError};
use serde_json::json;
use support::{Agent, LIMITS, Raw, Shop, entries, request, server, slow};
use tabletist_db::agent::Limits;
use tabletist_mcp::Log;
use tokio::io::AsyncWriteExt;

const BRIEF: &str = "Bookshop · brief";

/// A Bookshop whose statements may take half a second.
fn brief_shop() -> Shop {
    let shop = Shop::new();
    let brief = Limits {
        timeout: Duration::from_millis(500),
        ..LIMITS
    };
    shop.list(BRIEF, &shop.bookshop(), brief);
    shop
}

fn long_query() -> serde_json::Value {
    json!({ "connection": BRIEF, "sql": slow(6) })
}

/// Waits for the log to hold `count` entries.
async fn logged(log: &Log, count: usize) -> Vec<tabletist_mcp::Entry> {
    let deadline = Instant::now() + Duration::from_secs(20);
    loop {
        let entries = entries(log).await;
        if entries.len() >= count {
            return entries;
        }
        assert!(Instant::now() < deadline, "the log has {}", entries.len());
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
}

#[tokio::test]
async fn the_database_ends_a_statement_that_takes_too_long() {
    let shop = brief_shop();
    let (server, log) = server(&shop);
    let agent = Agent::connect(&server).await;
    let started = Instant::now();
    let called = agent.call("run_query", long_query()).await;
    assert_eq!(called.refused(), "timeout");
    assert_eq!(
        called.text,
        "The query was stopped after 0.5 s, this connection's limit."
    );
    assert!(
        started.elapsed() < Duration::from_secs(2),
        "{:?}",
        started.elapsed()
    );
    // The connection answers the next call.
    let next = agent
        .call(
            "run_query",
            json!({ "connection": BRIEF, "sql": "SELECT 1" }),
        )
        .await;
    assert!(!next.is_error);
    agent.close().await;
    assert_eq!(entries(&log).await[0].result, "timeout");
}

#[tokio::test]
async fn the_servers_own_timer_ends_what_the_database_did_not() {
    let shop = brief_shop();
    shop.without_the_databases_timeout();
    let (server, _log) = server(&shop);
    let agent = Agent::connect(&server).await;
    let started = Instant::now();
    let called = agent.call("run_query", long_query()).await;
    assert_eq!(called.refused(), "timeout");
    // The connection's half second, and the two the database is given to
    // end the statement by itself.
    let took = started.elapsed();
    assert!(took >= Duration::from_millis(2_500), "{took:?}");
    assert!(took < Duration::from_secs(5), "{took:?}");
    assert_eq!(shop.reads_ended(), 1, "the read was awaited to its end");
    agent.close().await;
}

#[tokio::test]
async fn a_call_the_client_cancels_is_ended_at_the_database() {
    let shop = Shop::new();
    let (server, log) = server(&shop);
    let agent = Agent::connect(&server).await;
    // The SDK's client cancels a request it has waited this long for.
    let arguments = json!({ "connection": "Bookshop · dev", "sql": slow(6) });
    let sent = agent
        .client
        .send_cancellable_request(
            ClientRequest::CallToolRequest(CallToolRequest::new(request("run_query", arguments))),
            PeerRequestOptions::with_timeout(Duration::from_millis(300)),
        )
        .await
        .unwrap();
    let started = Instant::now();
    let answer = sent.await_response().await;
    assert!(
        matches!(answer, Err(ServiceError::Timeout { .. })),
        "{answer:?}"
    );
    let entries = logged(&log, 1).await;
    assert_eq!(entries[0].result, "cancelled");
    assert_eq!(entries[0].code.as_deref(), Some("cancelled"));
    assert!(started.elapsed() < Duration::from_secs(5));
    assert_eq!(shop.reads_ended(), 1);
    // The session goes on.
    assert!(!agent.call("list_connections", json!({})).await.is_error);
    agent.close().await;
}

#[tokio::test]
async fn a_client_that_is_gone_leaves_nothing_running() {
    let shop = Shop::new();
    let (server, log) = server(&shop);
    let agent = Agent::connect(&server).await;
    let arguments = json!({ "connection": "Bookshop · dev", "sql": slow(6) });
    let sent = agent
        .client
        .send_cancellable_request(
            ClientRequest::CallToolRequest(CallToolRequest::new(request("run_query", arguments))),
            PeerRequestOptions::no_options(),
        )
        .await
        .unwrap();
    tokio::time::sleep(Duration::from_millis(300)).await;
    let started = Instant::now();
    drop(sent);
    // Closing returns when the server has let the session go. The SDK
    // goes on answering for five seconds after a client closed its end,
    // for one that sent its calls and waits. Then the server ends what
    // still runs: the statement would have taken minutes.
    agent.close().await;
    let took = started.elapsed();
    assert!(took < Duration::from_secs(10), "{took:?}");
    let entries = entries(&log).await;
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].result, "cancelled");
    assert_eq!(shop.reads_ended(), 1);
    assert_eq!(server.open_sessions(), []);
}

#[tokio::test]
async fn a_client_that_closed_its_writing_end_still_gets_its_answer() {
    let shop = Shop::new();
    let (server, _log) = server(&shop);
    let mut raw = Raw::connect(&server, 64 * 1024).await;
    raw.call(
        1,
        "run_query",
        json!({ "connection": "Bookshop · dev", "sql": "SELECT count(*) FROM books" }),
    )
    .await;
    raw.write.shutdown().await.unwrap();
    let answer = raw.next().await;
    assert_eq!(answer["id"], json!(1));
    assert_eq!(answer["result"].get("isError"), Some(&json!(false)));
    let text = answer["result"]["content"][0]["text"].as_str().unwrap();
    assert!(text.contains("\"rows\":[[12]]"), "{text}");
    raw.served.await.unwrap();
}

#[tokio::test]
async fn a_stop_closes_a_session_whose_client_reads_nothing() {
    let shop = Shop::new();
    let (server, _log) = server(&shop);
    // A transport that holds a kilobyte: the answers soon have nowhere to
    // go, and the client never takes one.
    let mut raw = Raw::connect(&server, 1024).await;
    for id in 1..=40 {
        raw.call(id, "list_connections", json!({})).await;
    }
    tokio::time::sleep(Duration::from_millis(300)).await;
    let started = Instant::now();
    let id = server.open_sessions()[0].id.clone();
    assert!(server.stop_session(&id));
    // The SDK would wait for that client for good.
    tokio::time::timeout(Duration::from_secs(15), &mut raw.served)
        .await
        .expect("the session was let go")
        .unwrap();
    assert!(
        started.elapsed() >= Duration::from_secs(2),
        "{:?}",
        started.elapsed()
    );
    assert_eq!(server.open_sessions(), []);
}

#[tokio::test]
async fn a_message_longer_than_the_server_reads_ends_the_session() {
    let shop = Shop::new();
    let (server, _log) = server(&shop);
    let mut raw = Raw::connect(&server, 64 * 1024).await;
    // Two megabytes, and no end of a line.
    let chunk = vec![b'x'; 64 * 1024];
    let wrote = async {
        for _ in 0..32 {
            if raw.write.write_all(&chunk).await.is_err() {
                break;
            }
        }
    };
    let (_, served) = tokio::join!(wrote, async {
        tokio::time::timeout(Duration::from_secs(15), &mut raw.served).await
    });
    served.expect("the session ended").unwrap();
    assert_eq!(server.open_sessions(), []);
}

#[tokio::test]
async fn a_session_runs_so_many_calls_at_once_and_refuses_the_next() {
    let shop = Shop::new();
    let (server, _log) = server(&shop);
    let mut raw = Raw::connect(&server, 1024 * 1024).await;
    // The first runs for minutes if nothing ends it, and the others wait
    // for the connection behind it.
    for id in 1..=20 {
        raw.call(
            id,
            "run_query",
            json!({ "connection": "Bookshop · dev", "sql": slow(6) }),
        )
        .await;
    }
    // Sixteen are running or waiting. The four behind them are refused at
    // once, the last four that were sent.
    let mut refused = Vec::new();
    for _ in 0..4 {
        let answer = raw.next().await;
        assert_eq!(
            answer["result"]["structuredContent"]["refused"],
            json!("busy")
        );
        assert_eq!(
            answer["result"]["content"][0]["text"],
            json!("Too many calls at once. Wait for an answer before the next call.")
        );
        refused.push(answer["id"].as_u64().unwrap());
    }
    refused.sort();
    assert_eq!(refused, [17, 18, 19, 20]);
    server.stop();
    for _ in 0..16 {
        let answer = raw.next().await;
        assert_eq!(
            answer["result"]["structuredContent"]["refused"],
            json!("server_stopped")
        );
    }
    raw.served.await.unwrap();
}

#[tokio::test]
async fn stopping_a_session_answers_its_call_and_closes_it() {
    let shop = Shop::new();
    let (server, log) = server(&shop);
    let (stopped, other) = (Agent::connect(&server).await, Agent::connect(&server).await);
    let id = server.open_sessions()[0].id.clone();
    let arguments = json!({ "connection": "Bookshop · dev", "sql": slow(6) });
    let started = Instant::now();
    let (called, ()) = tokio::join!(stopped.call("run_query", arguments), async {
        tokio::time::sleep(Duration::from_millis(300)).await;
        assert!(server.stop_session(&id));
    });
    assert_eq!(called.refused(), "server_stopped");
    assert_eq!(called.text, "Tabletist's agent server was stopped.");
    assert!(
        started.elapsed() < Duration::from_secs(5),
        "{:?}",
        started.elapsed()
    );
    // The server closes that session, and no other.
    stopped.ended().await;
    assert_eq!(server.open_sessions().len(), 1);
    assert!(!other.call("list_connections", json!({})).await.is_error);
    assert!(!server.stop_session(&id), "it is gone");
    other.close().await;
    let entries = entries(&log).await;
    assert_eq!(entries[0].result, "stopped");
    assert_eq!(entries[0].code.as_deref(), Some("server_stopped"));
}

#[tokio::test]
async fn stopping_the_server_answers_a_call_in_flight_that_it_was_stopped() {
    let shop = Shop::new();
    let (server, log) = server(&shop);
    let agent = Agent::connect(&server).await;
    let arguments = json!({ "connection": "Bookshop · dev", "sql": slow(6) });
    let (called, ()) = tokio::join!(agent.call("run_query", arguments), async {
        tokio::time::sleep(Duration::from_millis(300)).await;
        server.stop();
    });
    // The person's stop, though the SDK also cancels the call when the
    // session closes.
    assert_eq!(called.refused(), "server_stopped");
    agent.ended().await;
    assert_eq!(entries(&log).await[0].result, "stopped");
}
