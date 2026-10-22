//! The tools, called as an agent calls them: through the SDK's client, over
//! a transport in this process, on the Bookshop.

// `allow-unwrap-in-tests` does not cover helpers in an integration-test crate.
#![allow(clippy::unwrap_used)]

mod support;

use std::time::Duration;

use rmcp::model::ErrorCode;
use rmcp::service::ServiceError;
use serde_json::{Value as Json, json};
use support::{Agent, LIMITS, Shop, request, server};
use tabletist_db::agent::{Limits, TableName, Tables};
use tabletist_mcp::{MAX_ANSWER_BYTES, MAX_CELL_CHARS};

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
    assert_eq!(names, ["list_connections", "describe_schema", "run_query"]);
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
        "Unknown tool. Tabletist has list_connections, describe_schema, run_query."
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

async fn query(agent: &Agent, sql: &str) -> support::Called {
    agent
        .call("run_query", json!({ "connection": DEV, "sql": sql }))
        .await
}

#[tokio::test]
async fn a_connection_that_is_off_does_not_exist_for_an_agent() {
    let shop = Shop::new();
    shop.list("Bookshop · production", &shop.bookshop(), LIMITS);
    let (server, _log) = server(&shop);
    let agent = Agent::connect(&server).await;
    let on_production = json!({ "connection": "Bookshop · production", "sql": "SELECT 1" });
    assert!(
        !agent
            .call("run_query", on_production.clone())
            .await
            .is_error
    );

    // The person turns it Off. The session stays, and the change holds at
    // the next call.
    shop.unlist("Bookshop · production");
    let listed = agent.call("list_connections", json!({})).await;
    assert_eq!(listed.answer()["connections"].as_array().unwrap().len(), 1);
    for (tool, arguments) in [
        ("run_query", on_production),
        // A connection that never was answers the same.
        (
            "run_query",
            json!({ "connection": "Payroll", "sql": "SELECT 1" }),
        ),
    ] {
        let called = agent.call(tool, arguments).await;
        assert_eq!(called.refused(), "unknown_connection");
    }
    let called = agent
        .call(
            "run_query",
            json!({ "connection": "bookshop · dev", "sql": "SELECT 1" }),
        )
        .await;
    assert_eq!(
        called.text, "No connection named bookshop · dev. list_connections has the names.",
        "a name is the whole of it, letter for letter"
    );
    agent.close().await;
}

#[tokio::test]
async fn run_query_answers_columns_and_rows() {
    let shop = Shop::new();
    let (server, _log) = server(&shop);
    let agent = Agent::connect(&server).await;
    let called = agent
        .call(
            "run_query",
            json!({
                "connection": DEV,
                "sql": "SELECT id, name FROM publishers WHERE id <= ? AND name <> ? ORDER BY id",
                "params": [2, "it's \"quoted\""],
            }),
        )
        .await;
    assert_eq!(
        called.answer(),
        &json!({
            "columns": [{ "name": "id", "type": "INTEGER" }, { "name": "name", "type": "TEXT" }],
            "rows": [[1, "Northlight Books"], [2, "Harbor Press"]],
            "row_count": 2,
            "truncated": false,
            "took_ms": called.answer()["took_ms"],
        })
    );
    assert!(called.answer()["took_ms"].is_u64());
    agent.close().await;
}

#[tokio::test]
async fn run_query_explains_a_query_without_running_it() {
    let shop = Shop::new();
    let (server, _log) = server(&shop);
    let agent = Agent::connect(&server).await;
    let called = query(
        &agent,
        "EXPLAIN QUERY PLAN SELECT * FROM books WHERE id = 1",
    )
    .await;
    let answer = called.answer();
    assert!(answer["row_count"].as_u64().unwrap() >= 1);
    assert!(Json::to_string(&answer["rows"]).contains("books"));
    agent.close().await;
}

#[tokio::test]
async fn a_statement_the_check_refuses_never_reaches_the_database() {
    let shop = Shop::new();
    let (server, _log) = server(&shop);
    let agent = Agent::connect(&server).await;
    let cases = [
        (
            "DELETE FROM book_covers WHERE id = 9",
            "not_read_only",
            "run_query only runs reads, and this is a DELETE.",
        ),
        (
            "SELECT 1; DELETE FROM book_covers",
            "too_many_statements",
            "Send one statement per call.",
        ),
        (
            "CREATE TABLE notes (body TEXT)",
            "not_allowed",
            "CREATE TABLE is not available to agents.",
        ),
        (
            "SELECT load_extension('x')",
            "function_not_allowed",
            "load_extension is not available to agents.",
        ),
        (
            "SELECT * FROM books WHERE id = ?",
            "invalid_params",
            "The statement has 1 parameter and 0 were sent.",
        ),
        (
            "SELECT * FROM books WHERE (",
            "not_parsed",
            "Tabletist could not read this statement, so it was not run.",
        ),
    ];
    for (sql, code, said) in cases {
        let called = query(&agent, sql).await;
        assert_eq!(
            (called.refused(), called.text.as_str()),
            (code, said),
            "{sql}"
        );
        assert!(
            !called.text.contains("propose_write"),
            "no sentence names a tool there is not: {}",
            called.text
        );
    }
    assert_eq!(shop.reads_ended(), 0, "nothing was run");
    let called = agent
        .call(
            "run_query",
            json!({ "connection": DEV, "sql": "SELECT ?", "params": [[1, 2]] }),
        )
        .await;
    assert_eq!(called.refused(), "invalid_params");
    agent.close().await;
}

#[tokio::test]
async fn the_databases_own_error_is_the_agents_to_read() {
    let shop = Shop::new();
    let (server, _log) = server(&shop);
    let agent = Agent::connect(&server).await;
    let called = query(&agent, "SELECT nmae FROM authors").await;
    assert_eq!(called.refused(), "query_failed");
    assert!(
        called.text.contains("no such column: nmae"),
        "{}",
        called.text
    );
    // The session is none the worse for it.
    assert!(!query(&agent, "SELECT name FROM authors").await.is_error);
    agent.close().await;
}

#[tokio::test]
async fn an_answer_holds_the_connections_rows_at_most_and_says_when_it_was_cut() {
    let shop = Shop::new();
    let few = Limits { rows: 3, ..LIMITS };
    shop.list("Bookshop · few", &shop.bookshop(), few);
    let (server, _log) = server(&shop);
    let agent = Agent::connect(&server).await;
    let on_few = |sql: &str| json!({ "connection": "Bookshop · few", "sql": sql });
    let called = agent
        .call("run_query", on_few("SELECT id FROM books ORDER BY id"))
        .await;
    let answer = called.answer();
    assert_eq!(answer["rows"], json!([[1], [2], [3]]));
    assert_eq!(answer["row_count"], json!(3));
    assert_eq!(answer["truncated"], json!(true));
    assert_eq!(answer["truncated_by"], json!("row_limit"));

    // Exactly as many rows as the limit is every row.
    let called = agent
        .call(
            "run_query",
            on_few("SELECT id FROM books ORDER BY id LIMIT 3"),
        )
        .await;
    assert_eq!(called.answer()["truncated"], json!(false));
    assert!(called.answer().get("truncated_by").is_none());
    agent.close().await;
}

#[tokio::test]
async fn an_answer_is_kept_under_its_size() {
    let shop = Shop::new();
    let (server, _log) = server(&shop);
    let agent = Agent::connect(&server).await;
    // 400 rows of 1,000 characters: more than an answer may weigh.
    let called = query(
        &agent,
        "WITH RECURSIVE n(i) AS (SELECT 1 UNION ALL SELECT i + 1 FROM n WHERE i < 400) \
         SELECT i, printf('%.*c', 1000, 'x') FROM n",
    )
    .await;
    let answer = called.answer();
    assert_eq!(answer["truncated"], json!(true));
    assert_eq!(answer["truncated_by"], json!("size"));
    let rows = answer["row_count"].as_u64().unwrap();
    assert!((200..300).contains(&rows), "{rows}");
    assert_eq!(
        answer["rows"][0][0],
        json!(1),
        "the first rows are the ones kept"
    );
    assert!(
        called.text.len() <= MAX_ANSWER_BYTES,
        "{}",
        called.text.len()
    );
    assert!(called.text.len() > MAX_ANSWER_BYTES - 2_000);
    agent.close().await;
}

#[tokio::test]
async fn an_answer_is_sent_once() {
    let shop = Shop::new();
    let (server, _log) = server(&shop);
    let agent = Agent::connect(&server).await;
    let before = agent.received().len();
    let called = query(
        &agent,
        "WITH RECURSIVE n(i) AS (SELECT 1 UNION ALL SELECT i + 1 FROM n WHERE i < 400) \
         SELECT i, printf('%.*c', 1000, 'x') FROM n",
    )
    .await;
    assert_eq!(called.answer()["truncated_by"], json!("size"));
    // What went over the transport is the answer, with a quote mark
    // escaped here and there, and not the answer twice.
    let sent = agent.received().len() - before;
    assert!(sent < MAX_ANSWER_BYTES * 5 / 4, "{sent}");
    agent.close().await;
}

#[tokio::test]
async fn columns_that_are_too_much_by_themselves_are_refused() {
    let shop = Shop::new();
    let (server, _log) = server(&shop);
    let agent = Agent::connect(&server).await;
    // A short statement with a long answer and no row in it: two hundred
    // columns, each named by two thousand letters.
    let stars = vec!["*"; 200].join(", ");
    let sql = format!(
        "WITH t AS (SELECT 1 AS {}) SELECT {stars} FROM t WHERE 1 = 0",
        "a".repeat(2_000)
    );
    let called = query(&agent, &sql).await;
    assert_eq!(called.refused(), "too_large");
    assert_eq!(
        called.text,
        "The answer's columns alone are more than an answer may hold. Select fewer columns, \
         or give them shorter names."
    );
    agent.close().await;
}

#[tokio::test]
async fn a_databases_error_is_only_so_long() {
    let shop = Shop::new();
    let (server, _log) = server(&shop);
    let agent = Agent::connect(&server).await;
    // SQLite repeats the path it could not read, all 600,000 letters.
    let called = query(
        &agent,
        "SELECT json_extract('{}', printf('%.*c', 600000, 'x'))",
    )
    .await;
    assert_eq!(called.refused(), "query_failed");
    assert!(called.text.len() < 2_100, "{}", called.text.len());
    agent.close().await;
}

#[tokio::test]
async fn a_long_text_is_cut_and_a_binary_value_described() {
    let shop = Shop::new();
    let (server, _log) = server(&shop);
    let agent = Agent::connect(&server).await;
    let called = query(
        &agent,
        "SELECT printf('%.*c', 5000, 'x'), x'deadbeef', NULL",
    )
    .await;
    let row = &called.answer()["rows"][0];
    assert_eq!(
        row[0],
        json!({ "cut": "x".repeat(MAX_CELL_CHARS), "characters": 5000 })
    );
    assert_eq!(row[1], json!({ "binary_bytes": 4 }));
    assert_eq!(row[2], Json::Null);
    agent.close().await;
}

#[tokio::test]
async fn run_query_takes_its_arguments_and_no_others() {
    let shop = Shop::new();
    let (server, _log) = server(&shop);
    let agent = Agent::connect(&server).await;
    let cases = [
        (
            json!({ "connection": DEV }),
            "run_query needs `sql`, a string.",
        ),
        (
            json!({ "connection": DEV, "sql": ["SELECT 'secret'"] }),
            "run_query needs `sql`, a string.",
        ),
        (
            json!({ "connection": DEV, "sql": "SELECT 1", "limit": 5 }),
            "run_query takes connection, sql, params and nothing else.",
        ),
        (
            json!({ "connection": DEV, "sql": "SELECT ?", "params": "secret" }),
            "run_query: `params` is a list.",
        ),
    ];
    for (arguments, said) in cases {
        assert_eq!(invalid(&agent, "run_query", arguments).await, said);
    }
    agent.close().await;
}

#[tokio::test]
async fn a_second_call_on_a_connection_waits_its_turn_and_then_is_busy() {
    let shop = Shop::new();
    // Only the server's own timer ends the first call: a second and two
    // more. The second call waits the connection's one second.
    shop.without_the_databases_timeout();
    let brief = Limits {
        timeout: Duration::from_secs(1),
        ..LIMITS
    };
    shop.list("Bookshop · brief", &shop.bookshop(), brief);
    let (server, _log) = server(&shop);
    let (first, second) = (Agent::connect(&server).await, Agent::connect(&server).await);
    let on_brief = |sql: String| json!({ "connection": "Bookshop · brief", "sql": sql });
    let (long, waiting) =
        tokio::join!(first.call("run_query", on_brief(support::slow(6))), async {
            tokio::time::sleep(Duration::from_millis(200)).await;
            second.call("run_query", on_brief("SELECT 1".into())).await
        });
    assert_eq!(waiting.refused(), "busy");
    assert_eq!(
        waiting.text,
        "Another agent's query is running on this connection. Try again."
    );
    assert_eq!(long.refused(), "timeout");
    // Its turn comes once the first call has ended.
    let again = second.call("run_query", on_brief("SELECT 1".into())).await;
    assert!(!again.is_error);
    first.close().await;
    second.close().await;
}

#[tokio::test]
async fn describe_schema_lists_the_tables_and_views() {
    let shop = Shop::new();
    let (server, _log) = server(&shop);
    let agent = Agent::connect(&server).await;
    let called = agent
        .call("describe_schema", json!({ "connection": DEV }))
        .await;
    let answer = called.answer();
    assert_eq!(answer["schema"], json!("main"));
    let tables: Vec<(&str, &str)> = answer["tables"]
        .as_array()
        .unwrap()
        .iter()
        .map(|table| {
            (
                table["name"].as_str().unwrap(),
                table["kind"].as_str().unwrap(),
            )
        })
        .collect();
    assert_eq!(
        tables,
        [
            ("authors", "table"),
            ("book_covers", "table"),
            ("books", "table"),
            ("customer_names", "view"),
            ("customers", "table"),
            ("order_items", "table"),
            ("orders", "table"),
            ("publishers", "table"),
            ("recent_orders", "view"),
        ]
    );
    assert!(answer.get("truncated").is_none());
    assert!(answer.get("other_schemas").is_none(), "SQLite has the one");
    agent.close().await;
}

#[tokio::test]
async fn describe_schema_says_what_a_table_is_made_of() {
    let shop = Shop::new();
    let (server, _log) = server(&shop);
    let agent = Agent::connect(&server).await;
    let called = agent
        .call(
            "describe_schema",
            json!({ "connection": DEV, "tables": ["order_items", "no_such_table", "orders"] }),
        )
        .await;
    let answer = called.answer();
    assert_eq!(answer["not_found"], json!(["no_such_table"]));
    let tables = answer["tables"].as_array().unwrap();
    assert_eq!(tables.len(), 2);
    let items = &tables[0];
    assert_eq!(items["name"], json!("order_items"));
    assert_eq!(items["kind"], json!("table"));
    assert!(items.get("schema").is_none(), "its own schema goes unsaid");
    assert_eq!(items["primary_key"], json!(["id"]));
    let columns = items["columns"].as_array().unwrap();
    assert_eq!(columns.len(), 5);
    assert_eq!(
        columns[3],
        json!({ "name": "quantity", "type": "INTEGER", "nullable": false })
    );
    let to_orders = items["foreign_keys"]
        .as_array()
        .unwrap()
        .iter()
        .find(|key| key["references"] == json!("orders"))
        .unwrap();
    assert_eq!(
        to_orders,
        &json!({
            "columns": ["order_id"],
            "references": "orders",
            "referenced_columns": ["id"],
            "on_delete": "CASCADE",
        })
    );
    // And from the other side.
    let orders = &tables[1];
    assert_eq!(
        orders["referenced_by"],
        json!([{ "table": "order_items", "columns": ["order_id"], "on_delete": "CASCADE" }])
    );
    agent.close().await;
}

#[tokio::test]
async fn describe_schema_takes_so_many_tables_at_once() {
    let shop = Shop::new();
    let (server, _log) = server(&shop);
    let agent = Agent::connect(&server).await;
    let many: Vec<String> = (0..21).map(|n| format!("t{n}")).collect();
    let called = agent
        .call(
            "describe_schema",
            json!({ "connection": DEV, "tables": many }),
        )
        .await;
    assert_eq!(called.refused(), "too_large");
    assert_eq!(
        called.text,
        "describe_schema describes at most 20 tables in one call."
    );
    // And a name is no longer than a table's can be: what was asked for
    // comes back in the answer, and goes into the log.
    let called = agent
        .call(
            "describe_schema",
            json!({ "connection": DEV, "tables": ["books", "x".repeat(1_001)] }),
        )
        .await;
    assert_eq!(called.refused(), "too_large");
    assert_eq!(called.text, "A table's name is at most 1000 bytes.");
    let called = agent
        .call(
            "describe_schema",
            json!({ "connection": DEV, "tables": ["books", "x".repeat(1_000)] }),
        )
        .await;
    assert_eq!(called.answer()["not_found"], json!(["x".repeat(1_000)]));
    agent.close().await;
}

#[tokio::test]
async fn describe_schema_takes_a_list_of_names_and_knows_no_connection_that_is_off() {
    let shop = Shop::new();
    let (server, _log) = server(&shop);
    let agent = Agent::connect(&server).await;
    assert_eq!(
        invalid(
            &agent,
            "describe_schema",
            json!({ "connection": DEV, "tables": [1] })
        )
        .await,
        "describe_schema: `tables` is a list of strings."
    );
    let called = agent
        .call(
            "describe_schema",
            json!({ "connection": "Bookshop · production" }),
        )
        .await;
    assert_eq!(called.refused(), "unknown_connection");
    agent.close().await;
}

/// The Bookshop's `customers`, in every schema.
fn customers() -> Tables {
    Tables::AllExcept(vec![TableName {
        schema: None,
        name: "customers".into(),
    }])
}

#[tokio::test]
async fn list_connections_names_what_is_hidden() {
    let shop = Shop::new();
    let (server, _log) = server(&shop);
    let agent = Agent::connect(&server).await;
    shop.hide(DEV, customers());
    let called = agent.call("list_connections", json!({})).await;
    assert_eq!(
        called.answer()["connections"][0]["hidden"],
        json!(["customers"])
    );
    assert!(called.answer()["connections"][0].get("only").is_none());
    shop.hide(
        DEV,
        Tables::Only(vec![TableName {
            schema: Some("main".into()),
            name: "books".into(),
        }]),
    );
    let called = agent.call("list_connections", json!({})).await;
    assert_eq!(
        called.answer()["connections"][0]["only"],
        json!(["main.books"])
    );
    assert!(called.answer()["connections"][0].get("hidden").is_none());
    agent.close().await;
}

#[tokio::test]
async fn a_hidden_table_is_left_out_of_the_schema_and_its_rows_are_refused() {
    let shop = Shop::new();
    let (server, _log) = server(&shop);
    let agent = Agent::connect(&server).await;
    shop.hide(DEV, customers());
    // The list leaves out the table and the view over it.
    let called = agent
        .call("describe_schema", json!({ "connection": DEV }))
        .await;
    let names: Vec<&str> = called.answer()["tables"]
        .as_array()
        .unwrap()
        .iter()
        .map(|table| table["name"].as_str().unwrap())
        .collect();
    assert!(names.contains(&"orders"), "{names:?}");
    assert!(!names.contains(&"customers"), "{names:?}");
    assert!(!names.contains(&"customer_names"), "{names:?}");
    // Asked for by name, it is named as hidden, and the key to it from
    // orders is left out, as it would name its columns.
    let called = agent
        .call(
            "describe_schema",
            json!({ "connection": DEV, "tables": ["customers", "orders", "customer_names"] }),
        )
        .await;
    let answer = called.answer();
    assert_eq!(answer["hidden"], json!(["customers", "customer_names"]));
    let orders = &answer["tables"].as_array().unwrap()[0];
    assert_eq!(orders["name"], json!("orders"));
    assert!(orders.get("foreign_keys").is_none(), "{orders}");
    // Its rows are refused, and nothing is read.
    let called = agent
        .call(
            "run_query",
            json!({ "connection": DEV, "sql": "SELECT * FROM customers" }),
        )
        .await;
    assert_eq!(called.refused(), "table_hidden");
    assert_eq!(
        called.text,
        "customers is hidden for agents on this connection."
    );
    agent.close().await;
}

#[tokio::test]
async fn a_key_of_a_hidden_table_is_left_out_of_what_it_points_at() {
    let shop = Shop::new();
    let (server, _log) = server(&shop);
    let agent = Agent::connect(&server).await;
    shop.hide(
        DEV,
        Tables::AllExcept(vec![TableName {
            schema: None,
            name: "order_items".into(),
        }]),
    );
    // Books are pointed at by their covers and by order items: the hidden
    // one's key would name its columns, and is left out.
    let called = agent
        .call(
            "describe_schema",
            json!({ "connection": DEV, "tables": ["books"] }),
        )
        .await;
    let books = &called.answer()["tables"][0];
    assert_eq!(
        books["referenced_by"],
        json!([{ "table": "book_covers", "columns": ["book_id"], "on_delete": "NO ACTION" }])
    );
    agent.close().await;
}

#[tokio::test]
async fn a_hidden_table_that_is_gone_closes_the_connection_to_agents() {
    let shop = Shop::new();
    let (server, _log) = server(&shop);
    let agent = Agent::connect(&server).await;
    shop.hide(DEV, customers());
    // Renamed: the entry names what is no longer there, and the new name
    // is hidden by nothing.
    shop.change("ALTER TABLE customers RENAME TO clients");
    let look = "Agents may not use this connection until its agent settings are looked at in \
                Tabletist: they hide customers, and its user can see no table of that name.";
    for (tool, arguments) in [
        (
            "run_query",
            json!({ "connection": DEV, "sql": "SELECT * FROM clients" }),
        ),
        ("describe_schema", json!({ "connection": DEV })),
        (
            "describe_schema",
            json!({ "connection": DEV, "tables": ["clients"] }),
        ),
    ] {
        let called = agent.call(tool, arguments).await;
        assert_eq!(called.refused(), "connection_unavailable", "{tool}");
        assert_eq!(called.text, look, "{tool}");
    }
    // The person looks: with the entry put right, the connection is open
    // again.
    shop.hide(
        DEV,
        Tables::AllExcept(vec![TableName {
            schema: None,
            name: "clients".into(),
        }]),
    );
    let called = agent
        .call(
            "run_query",
            json!({ "connection": DEV, "sql": "SELECT count(*) FROM books" }),
        )
        .await;
    assert_eq!(called.answer()["rows"], json!([[12]]));
    agent.close().await;
}

#[tokio::test]
async fn list_connections_names_what_is_masked() {
    let shop = Shop::new();
    let (server, _log) = server(&shop);
    let agent = Agent::connect(&server).await;
    let called = agent.call("list_connections", json!({})).await;
    assert!(called.answer()["connections"][0].get("masked").is_none());
    shop.mask(DEV, &["orders.email", "main.orders.phone"]);
    let called = agent.call("list_connections", json!({})).await;
    assert_eq!(
        called.answer()["connections"][0]["masked"],
        json!(["orders.email", "main.orders.phone"])
    );
    agent.close().await;
}

#[tokio::test]
async fn describe_schema_marks_a_masked_column_and_a_view_that_reads_one() {
    let shop = Shop::new();
    let (server, _log) = server(&shop);
    let agent = Agent::connect(&server).await;
    shop.mask(DEV, &["orders.email"]);
    // In the list, the view that reads it is marked, and the one that
    // does not is not.
    let called = agent
        .call("describe_schema", json!({ "connection": DEV }))
        .await;
    let tables = called.answer()["tables"].as_array().unwrap().clone();
    let named = |name: &str| {
        tables
            .iter()
            .find(|table| table["name"] == json!(name))
            .unwrap_or_else(|| panic!("{name}"))
            .clone()
    };
    assert_eq!(
        named("recent_orders")["reads_masked"],
        json!("orders.email")
    );
    assert!(named("customer_names").get("reads_masked").is_none());
    assert!(named("orders").get("reads_masked").is_none());
    // Described, the column is marked, and so is the view.
    let called = agent
        .call(
            "describe_schema",
            json!({ "connection": DEV, "tables": ["orders", "recent_orders"] }),
        )
        .await;
    let described = called.answer()["tables"].as_array().unwrap().clone();
    let columns = described[0]["columns"].as_array().unwrap();
    let column = |name: &str| {
        columns
            .iter()
            .find(|column| column["name"] == json!(name))
            .unwrap()
            .clone()
    };
    assert_eq!(column("email")["masked"], json!(true));
    assert!(column("id").get("masked").is_none());
    assert_eq!(described[1]["reads_masked"], json!("orders.email"));
    agent.close().await;
}

#[tokio::test]
async fn describe_schema_says_a_tables_comment_unless_it_could_give_a_masked_value() {
    let shop = Shop::new();
    shop.comment("orders", "One order of a customer");
    shop.comment("recent_orders", "The orders of the last thirty days");
    shop.comment("customer_names", "Who ordered");
    let (server, _log) = server(&shop);
    let agent = Agent::connect(&server).await;
    let comments = |answer: &Json| -> Vec<Option<String>> {
        let tables = answer["tables"].as_array().unwrap();
        tables
            .iter()
            .map(|table| {
                table
                    .get("comment")
                    .and_then(Json::as_str)
                    .map(str::to_owned)
            })
            .collect()
    };
    let asked = json!({
        "connection": DEV,
        "tables": ["orders", "recent_orders", "customer_names", "order_items"],
    });
    let called = agent.call("describe_schema", asked.clone()).await;
    assert_eq!(
        comments(called.answer()),
        [
            Some("One order of a customer".to_owned()),
            Some("The orders of the last thirty days".to_owned()),
            Some("Who ordered".to_owned()),
            None,
        ]
    );
    // A table's comment is its author's words and may give a value of its
    // masked column as an example, as a column's comment may: the table
    // with the column and the view that reads it say none.
    shop.mask(DEV, &["orders.email"]);
    let called = agent.call("describe_schema", asked).await;
    assert_eq!(
        comments(called.answer()),
        [None, None, Some("Who ordered".to_owned()), None]
    );
    agent.close().await;
}

#[tokio::test]
async fn run_query_answers_a_masked_column_as_dots_and_refuses_a_use_of_it() {
    let shop = Shop::new();
    let (server, log) = server(&shop);
    let agent = Agent::connect(&server).await;
    shop.mask(DEV, &["orders.email", "orders.phone"]);
    // Its name and its type, `•••` in every row, a NULL too, and marked.
    let called = query(&agent, "SELECT id, email, phone FROM orders WHERE id = 1").await;
    let answer = called.answer();
    assert_eq!(answer["columns"][0].get("masked"), None);
    assert_eq!(answer["columns"][1]["name"], json!("email"));
    assert_eq!(answer["columns"][1]["masked"], json!(true));
    assert_eq!(answer["columns"][2]["masked"], json!(true));
    assert_eq!(answer["rows"], json!([[1, "•••", "•••"]]));
    // The log keeps which columns were masked, and no value.
    let logged = log.latest(1).await.unwrap();
    assert_eq!(logged[0].masked.as_deref(), Some(r#"["email","phone"]"#));
    // A use of it is refused, and so is a view that reads it.
    let called = query(&agent, "SELECT id FROM orders WHERE email LIKE 'a%'").await;
    assert_eq!(called.refused(), "masked_column_used");
    assert!(
        called.text.starts_with("orders.email is masked"),
        "{}",
        called.text
    );
    let called = query(&agent, "SELECT * FROM recent_orders").await;
    assert_eq!(called.refused(), "masked_column_used");
    assert_eq!(
        called.text,
        "recent_orders reads orders.email, which is masked on this connection."
    );
    // An answer with no masked column logs none.
    query(&agent, "SELECT id FROM orders WHERE id = 1").await;
    let logged = log.latest(1).await.unwrap();
    assert_eq!(logged[0].masked, None);
    agent.close().await;
}

#[tokio::test]
async fn a_masked_columns_default_values_and_what_is_computed_from_it_are_not_told() {
    let shop = Shop::new();
    shop.change(
        "CREATE TABLE gift_notes (
             id INTEGER PRIMARY KEY,
             kind TEXT DEFAULT 'gift' CHECK (kind IN ('gift', 'self')),
             shout TEXT GENERATED ALWAYS AS (upper(kind)) VIRTUAL
         );
         INSERT INTO gift_notes (id, kind) VALUES (1, 'self');",
    );
    let (server, _log) = server(&shop);
    let agent = Agent::connect(&server).await;
    shop.mask(DEV, &["gift_notes.kind"]);
    let called = agent
        .call(
            "describe_schema",
            json!({ "connection": DEV, "tables": ["gift_notes"] }),
        )
        .await;
    let columns = called.answer()["tables"][0]["columns"]
        .as_array()
        .unwrap()
        .clone();
    let column = |name: &str| {
        columns
            .iter()
            .find(|column| column["name"] == json!(name))
            .unwrap()
            .clone()
    };
    // Its default and the values it may hold say what its values are.
    let kind = column("kind");
    assert_eq!(kind["masked"], json!(true), "{kind}");
    assert!(kind.get("default").is_none(), "{kind}");
    assert!(kind.get("values").is_none(), "{kind}");
    // A column computed from it is masked with it, and its expression is
    // not told.
    let shout = column("shout");
    assert_eq!(shout["masked"], json!(true), "{shout}");
    assert!(shout.get("default").is_none(), "{shout}");
    let called = query(&agent, "SELECT id, shout FROM gift_notes").await;
    assert_eq!(called.answer()["rows"], json!([[1, "•••"]]));
    let called = query(&agent, "SELECT id FROM gift_notes WHERE shout = 'SELF'").await;
    assert_eq!(called.refused(), "masked_column_used");
    agent.close().await;
}

#[tokio::test]
async fn list_connections_gives_the_administrators_entries_apart() {
    let shop = Shop::new();
    let (server, _log) = server(&shop);
    let agent = Agent::connect(&server).await;
    shop.hide(DEV, customers());
    shop.rule(
        DEV,
        Tables::AllExcept(vec![TableName {
            schema: None,
            name: "orders".into(),
        }]),
        &[],
    );
    // A rule with nothing hidden or masked is not told.
    shop.rule(DEV, Tables::All, &[]);
    shop.rule(
        DEV,
        Tables::Only(vec![TableName {
            schema: None,
            name: "books".into(),
        }]),
        &["books.price"],
    );
    let called = agent.call("list_connections", json!({})).await;
    let listed = &called.answer()["connections"][0];
    assert_eq!(listed["hidden"], json!(["customers"]));
    assert_eq!(
        listed["administrator"],
        json!([
            { "hidden": ["orders"] },
            { "only": ["books"], "masked": ["books.price"] },
        ])
    );
    // Without a rule that says anything, there is none.
    shop.list(DEV, &shop.bookshop(), LIMITS);
    let called = agent.call("list_connections", json!({})).await;
    assert!(
        called.answer()["connections"][0]
            .get("administrator")
            .is_none()
    );
    agent.close().await;
}

#[tokio::test]
async fn a_rules_hidden_table_is_refused_as_the_administrators() {
    let shop = Shop::new();
    let (server, _log) = server(&shop);
    let agent = Agent::connect(&server).await;
    shop.rule(DEV, customers(), &["orders.email"]);
    let called = agent
        .call(
            "run_query",
            json!({ "connection": DEV, "sql": "SELECT * FROM customers" }),
        )
        .await;
    assert_eq!(called.refused(), "table_hidden");
    assert_eq!(
        called.text,
        "customers is hidden for agents by your administrator's policy."
    );
    let called = agent
        .call(
            "run_query",
            json!({ "connection": DEV, "sql": "SELECT id FROM orders ORDER BY email" }),
        )
        .await;
    assert_eq!(called.refused(), "masked_column_used");
    assert!(
        called
            .text
            .starts_with("orders.email is masked by your administrator's policy."),
        "{}",
        called.text
    );
    // Left out of the schema, as the person's are.
    let called = agent
        .call("describe_schema", json!({ "connection": DEV }))
        .await;
    let names: Vec<&str> = called.answer()["tables"]
        .as_array()
        .unwrap()
        .iter()
        .map(|table| table["name"].as_str().unwrap())
        .collect();
    assert!(!names.contains(&"customers"), "{names:?}");
    assert!(names.contains(&"books"), "{names:?}");
    agent.close().await;
}

#[tokio::test]
async fn a_rules_stale_entry_closes_the_connection_until_the_policy_is_fixed() {
    let shop = Shop::new();
    let (server, _log) = server(&shop);
    let agent = Agent::connect(&server).await;
    shop.rule(
        DEV,
        Tables::AllExcept(vec![TableName {
            schema: None,
            name: "ledger".into(),
        }]),
        &[],
    );
    for (tool, arguments) in [
        (
            "run_query",
            json!({ "connection": DEV, "sql": "SELECT count(*) FROM books" }),
        ),
        ("describe_schema", json!({ "connection": DEV })),
    ] {
        let called = agent.call(tool, arguments).await;
        assert_eq!(called.refused(), "connection_unavailable", "{tool}");
        assert_eq!(
            called.text,
            "Agents may not use this connection until your administrator's policy is fixed.",
            "{tool}"
        );
    }
    agent.close().await;
}
