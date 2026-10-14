//! `list_connections`: the connections an agent may use.

use std::time::Duration;

use serde_json::{Map, Value as Json, json};
use tabletist_db::Dialect;

use super::{Done, Invalid, Note, Tool, schema};
use crate::gateway::{Gateway, Listed};

pub const NAME: &str = "list_connections";

pub fn tool() -> Tool {
    Tool {
        name: NAME,
        title: "List connections",
        description: "The database connections you may use, with what each is and its limits. \
                      Call a connection by its name, exactly as it is written here. A \
                      connection the person has not opened to agents is not listed.",
        input: schema(json!({}), &[]),
    }
}

pub fn call<G: Gateway>(gateway: &G, arguments: &Map<String, Json>) -> Result<Done, Invalid> {
    if !arguments.is_empty() {
        return Err(Invalid::Arguments(format!("{NAME} takes no arguments.")));
    }
    let listed = gateway.connections();
    let mut answer = Map::new();
    answer.insert(
        "connections".into(),
        Json::Array(listed.iter().map(entry).collect()),
    );
    Ok(Done {
        answer: Ok(answer),
        note: Note {
            rows: Some(listed.len() as u64),
            ..Note::default()
        },
    })
}

fn entry(listed: &Listed) -> Json {
    // What an agent must know to write a statement the database reads.
    let (engine, parameters) = match listed.dialect {
        Dialect::Postgres => ("PostgreSQL", "$1, $2"),
        Dialect::MySql => ("MySQL", "?"),
        Dialect::Sqlite => ("SQLite", "?"),
    };
    json!({
        "name": listed.name,
        "engine": engine,
        "parameters": parameters,
        "access": "read",
        "rows": listed.limits.rows,
        "timeout_s": seconds(listed.limits.timeout),
    })
}

/// Whole seconds as a whole number.
fn seconds(time: Duration) -> Json {
    match time.subsec_nanos() {
        0 => json!(time.as_secs()),
        _ => json!(time.as_secs_f64()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_timeout_is_written_in_seconds() {
        assert_eq!(seconds(Duration::from_secs(10)), json!(10));
        assert_eq!(seconds(Duration::from_millis(1_500)), json!(1.5));
    }
}
