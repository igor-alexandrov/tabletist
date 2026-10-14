//! The tools an agent has, in this crate's own words: what each is called,
//! what it takes, and what a call of it answers. `protocol` puts them on
//! the wire.

mod connections;

use std::time::Duration;

use serde_json::{Map, Value as Json, json};

use crate::gateway::Gateway;
use crate::refusal::Refused;

/// A tool as an agent is told of it.
#[derive(Debug, Clone, PartialEq)]
pub struct Tool {
    pub name: &'static str,
    pub title: &'static str,
    pub description: &'static str,
    /// The JSON Schema of its arguments.
    pub input: Json,
}

/// Every tool. Each of them only reads.
pub fn all() -> Vec<Tool> {
    vec![connections::tool()]
}

/// A call that is not one of a tool's: the protocol's error, not a tool's
/// result. Its sentence never repeats what the agent sent: it is the SDK
/// that answers with it, and the SDK traces what it answers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Invalid {
    UnknownTool,
    Arguments(String),
}

impl Invalid {
    pub fn message(&self) -> String {
        match self {
            Self::UnknownTool => {
                let names: Vec<&str> = all().iter().map(|tool| tool.name).collect();
                format!("Unknown tool. Tabletist has {}.", names.join(", "))
            }
            Self::Arguments(what) => what.clone(),
        }
    }
}

/// What a call was about and what came of it, for the log. Never a row's
/// value.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct Note {
    /// The id and the name of the connection the call named, once it was
    /// found.
    pub connection: Option<(String, String)>,
    pub sql: Option<String>,
    /// The values, as the JSON they came in.
    pub params: Option<String>,
    /// The tables `describe_schema` was asked for.
    pub request: Option<String>,
    pub ran_as: Option<&'static str>,
    pub rows: Option<u64>,
    /// The statement's rows, where the answer holds them all.
    pub matched: Option<u64>,
    pub columns: Option<u64>,
    pub took: Option<Duration>,
}

/// A call's end: the answer or the refusal, and the note.
#[derive(Debug)]
pub(crate) struct Done {
    pub answer: Result<Map<String, Json>, Refused>,
    pub note: Note,
}

/// Runs the tool called `name`.
pub(crate) async fn call<G: Gateway>(
    gateway: &G,
    name: &str,
    arguments: &Map<String, Json>,
) -> Result<Done, Invalid> {
    match name {
        connections::NAME => connections::call(gateway, arguments),
        _ => Err(Invalid::UnknownTool),
    }
}

/// The schema of an object: `properties`, all of them strings unless said
/// otherwise, of which `required` must be there.
fn schema(properties: Json, required: &[&str]) -> Json {
    json!({
        "type": "object",
        "properties": properties,
        "required": required,
        "additionalProperties": false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_tools_are_told_as_they_are() {
        let names: Vec<&str> = all().iter().map(|tool| tool.name).collect();
        assert_eq!(names, ["list_connections"]);
        for tool in all() {
            assert_eq!(tool.input["type"], json!("object"), "{}", tool.name);
            assert!(!tool.description.is_empty() && !tool.title.is_empty());
        }
    }

    #[test]
    fn a_tool_that_is_not_there_is_answered_with_the_ones_that_are() {
        assert_eq!(
            Invalid::UnknownTool.message(),
            "Unknown tool. Tabletist has list_connections."
        );
    }
}
