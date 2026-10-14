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
