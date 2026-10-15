//! An agent's session: how it starts, what the server knows of it, and how
//! the server lets it go.

// `allow-unwrap-in-tests` does not cover helpers in an integration-test crate.
#![allow(clippy::unwrap_used)]

mod support;

use rmcp::model::ProtocolVersion;
use rmcp::service::ClientLifecycleMode;
use rmcp::{ClientServiceExt, ServiceExt};
use serde_json::json;
use support::{Agent, Shop, client, request, serve, server};

#[tokio::test]
async fn a_client_of_the_newer_protocol_is_served_without_a_handshake() {
    let shop = Shop::new();
    let (server, log) = server(&shop);
    let (transport, served, _received) = serve(&server);
    let agent = client("cursor", None)
        .serve_with_lifecycle(
            transport,
            ClientLifecycleMode::Discover {
                preferred_versions: vec![ProtocolVersion::V_2026_07_28],
            },
        )
        .await
        .unwrap();
    let called = support::called(
        agent
            .call_tool(request("list_connections", json!({})))
            .await
            .unwrap(),
    );
    assert_eq!(
        called.answer()["connections"][0]["name"],
        json!("Bookshop · dev")
    );
    // Such a client names itself in each request, not once at the start.
    assert_eq!(server.open_sessions()[0].agent.as_deref(), Some("cursor"));
    let _ = agent.cancel().await;
    served.await.unwrap();
    assert_eq!(support::entries(&log).await[0].agent, "cursor");
}

#[tokio::test]
async fn a_session_is_known_while_it_is_open() {
    let shop = Shop::new();
    let (server, _log) = server(&shop);
    assert_eq!(server.open_sessions(), []);
    let agent = Agent::connect(&server).await;
    // It has a name from its first word, before it calls anything.
    assert_eq!(
        server.open_sessions()[0].agent.as_deref(),
        Some("Claude Code")
    );
    agent.call("list_connections", json!({})).await;
    agent.call("list_connections", json!({})).await;
    let open = server.open_sessions();
    assert_eq!(open.len(), 1);
    assert_eq!(open[0].agent.as_deref(), Some("Claude Code"));
    assert_eq!(open[0].project.as_deref(), Some("tabletist-repo"));
    assert_eq!(open[0].calls, 2);
    let other = Agent::connect_as(&server, client("cursor", None)).await;
    assert_eq!(server.open_sessions().len(), 2);
    assert_ne!(server.open_sessions()[0].id, server.open_sessions()[1].id);
    agent.close().await;
    other.close().await;
    assert_eq!(server.open_sessions(), []);
}

#[tokio::test]
async fn a_session_knows_where_it_works_and_the_server_says_when_that_changes() {
    let shop = Shop::new();
    let (server, _log) = server(&shop);
    let mut changes = server.changes();
    changes.mark_unchanged();
    let agent = Agent::connect(&server).await;
    assert!(changes.has_changed().unwrap(), "a session opened");
    changes.mark_unchanged();
    // A call that names no connection is a call all the same.
    agent.call("list_connections", json!({})).await;
    assert!(changes.has_changed().unwrap(), "a call");
    changes.mark_unchanged();
    let open = &server.open_sessions()[0];
    assert_eq!(open.connections, Vec::<String>::new());
    let first = open.last_call_at.unwrap();
    agent
        .call(
            "run_query",
            json!({ "connection": "Bookshop · dev", "sql": "SELECT 1" }),
        )
        .await;
    assert!(changes.has_changed().unwrap(), "a call on a connection");
    let open = &server.open_sessions()[0];
    assert_eq!(open.connections, ["id:Bookshop · dev"]);
    assert!(open.last_call_at.unwrap() >= first);
    // Named once, however often.
    agent
        .call("describe_schema", json!({ "connection": "Bookshop · dev" }))
        .await;
    assert_eq!(server.open_sessions()[0].connections, ["id:Bookshop · dev"]);
    // Another, then the first again: the one it named last is at the end,
    // where it works now.
    shop.list("Bookshop · staging", &shop.bookshop(), support::LIMITS);
    for name in ["Bookshop · staging", "Bookshop · dev"] {
        agent
            .call("describe_schema", json!({ "connection": name }))
            .await;
    }
    assert_eq!(
        server.open_sessions()[0].connections,
        ["id:Bookshop · staging", "id:Bookshop · dev"]
    );
    changes.mark_unchanged();
    agent.close().await;
    assert!(changes.has_changed().unwrap(), "a session ended");
    assert_eq!(server.open_sessions(), []);
}

#[tokio::test]
async fn a_session_is_where_its_call_names_from_when_the_call_comes() {
    use std::time::{Duration, Instant};
    let shop = Shop::new();
    let (server, _log) = server(&shop);
    let agent = Agent::connect(&server).await;
    let arguments = json!({ "connection": "Bookshop · dev", "sql": support::slow(5) });
    let slow = agent.call("run_query", arguments);
    let meanwhile = async {
        // Here while its first call on the connection still runs.
        let deadline = Instant::now() + Duration::from_millis(500);
        while server.open_sessions()[0].connections.is_empty() {
            assert!(Instant::now() < deadline, "not here while the call runs");
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        assert_eq!(server.open_sessions()[0].connections, ["id:Bookshop · dev"]);
        agent.call("list_connections", json!({})).await;
        server.open_sessions()[0].last_call_at.unwrap()
    };
    let (_, quick) = tokio::join!(slow, meanwhile);
    // The slow call came first and ended last: it takes no time back.
    assert!(server.open_sessions()[0].last_call_at.unwrap() >= quick);
    agent.close().await;
}

#[tokio::test]
async fn a_call_that_ends_after_a_later_one_leaves_the_session_where_the_later_one_named() {
    use std::time::{Duration, Instant};
    let shop = Shop::new();
    let (server, _log) = server(&shop);
    shop.list("Bookshop · staging", &shop.bookshop(), support::LIMITS);
    let agent = Agent::connect(&server).await;
    let arguments = json!({ "connection": "Bookshop · dev", "sql": support::slow(2) });
    let slow = agent.call("run_query", arguments);
    let meanwhile = async {
        // Once the slow call is on dev, a quick one on staging, which
        // ends first.
        let deadline = Instant::now() + Duration::from_millis(500);
        while server.open_sessions()[0].connections.is_empty() {
            assert!(Instant::now() < deadline, "not here while the call runs");
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        agent
            .call(
                "describe_schema",
                json!({ "connection": "Bookshop · staging" }),
            )
            .await;
    };
    tokio::join!(slow, meanwhile);
    // The slow call ended last, and staging was named last.
    assert_eq!(
        server.open_sessions()[0].connections,
        ["id:Bookshop · dev", "id:Bookshop · staging"]
    );
    agent.close().await;
}

#[tokio::test]
async fn a_name_a_client_gives_itself_is_a_label_a_person_can_read() {
    let shop = Shop::new();
    let (server, _log) = server(&shop);
    let label =
        |server: &support::Server<Shop>, at: usize| server.open_sessions()[at].agent.clone();

    // Nothing that moves the cursor, and so long and no longer.
    let name = format!("Evil\u{1b}[31m\n{}", "x".repeat(500));
    let long = Agent::connect_as(&server, client(&name, None)).await;
    let read = label(&server, 0).unwrap();
    assert!(read.starts_with("Evil[31mx"), "{read}");
    assert_eq!(read.chars().count(), 64);

    // Nothing that turns the line around or cannot be seen.
    let turned = Agent::connect_as(
        &server,
        client("Claude\u{202e}edoC\u{200b}\u{feff}\u{2028}", None),
    )
    .await;
    assert_eq!(label(&server, 1).as_deref(), Some("ClaudeedoC"));

    // A title a person could not read does not hide the name.
    let blank = Agent::connect_as(&server, client("cursor", Some(" \u{200b}\n"))).await;
    assert_eq!(label(&server, 2).as_deref(), Some("cursor"));

    // And a client that gives nothing is known as nothing.
    let nameless = Agent::connect_as(&server, client("\u{0}", Some(""))).await;
    assert_eq!(label(&server, 3), None);
    for agent in [long, turned, blank, nameless] {
        agent.close().await;
    }
}

#[tokio::test]
async fn a_session_whose_serving_was_given_up_is_not_left_open() {
    let shop = Shop::new();
    let (server, _log) = server(&shop);
    let (transport, served, _received) = serve(&server);
    let agent = client("claude-code", None).serve(transport).await.unwrap();
    assert_eq!(server.open_sessions().len(), 1);
    served.abort();
    let _ = served.await;
    assert_eq!(server.open_sessions(), []);
    drop(agent);
}

#[tokio::test]
async fn stopping_the_server_ends_every_session_and_serves_no_new_one() {
    let shop = Shop::new();
    let (server, _log) = server(&shop);
    let (first, second) = (Agent::connect(&server).await, Agent::connect(&server).await);
    server.stop();
    first.ended().await;
    second.ended().await;
    assert_eq!(server.open_sessions(), []);
    // A transport handed to it now is closed at once.
    let (transport, served, _received) = serve(&server);
    served.await.unwrap();
    drop(transport);
    assert_eq!(server.open_sessions(), []);
}

#[tokio::test]
async fn each_call_is_told_as_it_ends_without_its_values() {
    let shop = Shop::new();
    let (server, log) = server(&shop);
    let mut calls = server.calls();
    let agent = Agent::connect(&server).await;
    agent
        .call(
            "run_query",
            json!({
                "connection": "Bookshop · dev",
                "sql": "SELECT id,\n       title\n  FROM books WHERE id = ?",
                "params": [1],
            }),
        )
        .await;
    let call = calls.recv().await.unwrap();
    assert_eq!(call.session, server.open_sessions()[0].id);
    assert_eq!(call.connection.as_deref(), Some("id:Bookshop · dev"));
    assert_eq!(call.tool, "run_query");
    assert_eq!(
        call.request.as_deref(),
        Some("SELECT id, title FROM books WHERE id = ?")
    );
    assert_eq!(
        (call.result.as_str(), call.message.as_deref()),
        ("ok", None)
    );
    assert_eq!((call.rows, call.columns), (Some(1), Some(2)));
    // In the log by the time it is told.
    assert_eq!(support::entries(&log).await.len(), 1);
    // A refusal says why, as the log does; a call that names no
    // connection is told with none.
    agent
        .call(
            "run_query",
            json!({ "connection": "Payroll", "sql": "SELECT 1" }),
        )
        .await;
    let refused = calls.recv().await.unwrap();
    assert_eq!(
        (refused.result.as_str(), refused.connection.as_deref()),
        ("refused", None)
    );
    assert!(refused.message.unwrap().contains("Payroll"));
    agent.close().await;
}
