//! The tools, called as an agent calls them: through the SDK's client, over
//! a transport in this process, on the Bookshop.

// `allow-unwrap-in-tests` does not cover helpers in an integration-test crate.
#![allow(clippy::unwrap_used)]

mod support;

use rmcp::model::ErrorCode;
use rmcp::service::ServiceError;
use serde_json::{Value as Json, json};
use support::{Agent, Shop, request, server};

const DEV: &str = "Bookshop · dev";

/// Calls a tool in a way that is no call of it, and gives the sentence of
/// the protocol's error.
async fn invalid(agent: &Agent, tool: &'static str, arguments: Json) -> String {
    let error = agent
        .client
        .call_tool(request(tool, arguments))
        .await
        .unwrap_err();
    let ServiceError::McpError(error) = error else {
        panic!("{tool}: {error:?}");
    };
    assert_eq!(error.code, ErrorCode::INVALID_PARAMS, "{tool}");
    error.message.to_string()
}

#[tokio::test]
async fn an_agent_is_told_the_tools_and_that_each_only_reads() {
    let shop = Shop::new();
    let (server, _log) = server(&shop);
    let agent = Agent::connect(&server).await;
    let tools = agent.client.list_all_tools().await.unwrap();
    let names: Vec<&str> = tools.iter().map(|tool| tool.name.as_ref()).collect();
    assert_eq!(names, ["list_connections"]);
    for tool in &tools {
        let annotations = tool.annotations.as_ref().unwrap();
        assert_eq!(annotations.read_only_hint, Some(true), "{}", tool.name);
        assert!(tool.title.is_some() && tool.description.is_some());
        assert_eq!(tool.input_schema["type"], json!("object"));
    }
    let told = agent.client.peer_info().unwrap();
    let about = told.server_info.as_ref().unwrap();
    assert_eq!(
        (about.name.as_str(), about.version.as_str()),
        ("tabletist", "0.0.0-test")
    );
    agent.close().await;
}

#[tokio::test]
async fn list_connections_names_what_an_agent_may_use() {
    let shop = Shop::new();
    let (server, _log) = server(&shop);
    let agent = Agent::connect(&server).await;
    let called = agent.call("list_connections", json!({})).await;
    assert_eq!(
        called.answer(),
        &json!({
            "connections": [{
                "name": DEV,
                "engine": "SQLite",
                "parameters": "?",
                "access": "read",
                "rows": 500,
                "timeout_s": 10,
            }]
        })
    );
    agent.close().await;
}

#[tokio::test]
async fn a_call_that_is_no_tools_is_the_protocols_error() {
    let shop = Shop::new();
    let (server, log) = server(&shop);
    let agent = Agent::connect(&server).await;
    // The sentence never repeats what was sent, here a name.
    assert_eq!(
        invalid(&agent, "drop_everything", json!({})).await,
        "Unknown tool. Tabletist has list_connections."
    );
    assert_eq!(
        invalid(&agent, "list_connections", json!({ "all": true })).await,
        "list_connections takes no arguments."
    );
    // The session goes on, and neither is a call in the log.
    assert!(!agent.call("list_connections", json!({})).await.is_error);
    agent.close().await;
    assert_eq!(support::entries(&log).await.len(), 1);
}
