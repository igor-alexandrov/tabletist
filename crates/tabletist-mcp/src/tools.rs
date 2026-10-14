//! The tools an agent has, in this crate's own words: what each is called,
//! what it takes, and what a call of it answers. `protocol` puts them on
//! the wire.

mod connections;
mod query;
mod schema;

use std::future::Future;
use std::time::Duration;

use serde_json::{Map, Value as Json, json};
use tabletist_db::agent::Limits;

use crate::gateway::{Failure, Gateway, Listed};
use crate::refusal::Refused;
use crate::stop::{Stop, Why};

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
    vec![connections::tool(), schema::tool(), query::tool()]
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

/// How long past its connection's time a call is left before this crate
/// ends it: the database's own timeout comes first, and says more.
pub(crate) const GRACE: Duration = Duration::from_secs(2);

/// Runs the tool called `name`.
pub(crate) async fn call<G: Gateway>(
    gateway: &G,
    stop: &Stop,
    name: &str,
    arguments: &Map<String, Json>,
) -> Result<Done, Invalid> {
    match name {
        connections::NAME => connections::call(gateway, arguments),
        schema::NAME => schema::call(gateway, stop, arguments).await,
        query::NAME => query::call(gateway, stop, arguments).await,
        _ => Err(Invalid::UnknownTool),
    }
}

/// A tool's arguments, read strictly: an argument a tool does not have is
/// refused, so that an agent learns it was not used.
struct Arguments<'a> {
    tool: &'static str,
    given: &'a Map<String, Json>,
}

impl<'a> Arguments<'a> {
    fn new(
        tool: &'static str,
        given: &'a Map<String, Json>,
        known: &[&str],
    ) -> Result<Self, Invalid> {
        if given.keys().any(|key| !known.contains(&key.as_str())) {
            return Err(Invalid::Arguments(format!(
                "{tool} takes {} and nothing else.",
                known.join(", ")
            )));
        }
        Ok(Self { tool, given })
    }

    fn text(&self, key: &str) -> Result<&'a str, Invalid> {
        self.given
            .get(key)
            .and_then(Json::as_str)
            .ok_or_else(|| Invalid::Arguments(format!("{} needs `{key}`, a string.", self.tool)))
    }

    /// An argument that may be left out, or null.
    fn list(&self, key: &str) -> Result<Option<&'a [Json]>, Invalid> {
        match self.given.get(key) {
            None | Some(Json::Null) => Ok(None),
            Some(Json::Array(items)) => Ok(Some(items)),
            Some(_) => Err(Invalid::Arguments(format!(
                "{}: `{key}` is a list.",
                self.tool
            ))),
        }
    }
}

/// The connection an agent named. The name is the whole of it: a name
/// that is not listed is unknown, whatever else it is.
fn find<G: Gateway>(gateway: &G, name: &str) -> Result<Listed, Refused> {
    gateway
        .connections()
        .into_iter()
        .find(|listed| listed.name == name)
        .ok_or_else(|| Refused::unknown_connection(name))
}

/// Awaits `work` to its end, and ends the call when it runs past its
/// connection's time by [`GRACE`]. The work is never dropped: once the call
/// is ended it is the gateway that brings it to its end.
async fn within<T>(stop: &Stop, limits: Limits, work: impl Future<Output = T>) -> T {
    tokio::pin!(work);
    tokio::select! {
        done = &mut work => done,
        () = tokio::time::sleep(limits.timeout.saturating_add(GRACE)) => {
            stop.end(Why::Timeout);
            work.await
        }
    }
}

/// What the work on a connection comes to. A call the person stopped
/// answers that, whatever it had got to: an agent that was stopped is
/// given nothing more.
fn settle<T>(result: Result<T, Failure>, stop: &Stop, listed: &Listed) -> Result<T, Refused> {
    match (result, stop.why()) {
        (_, Some(Why::Stopped)) => Err(Refused::stopped()),
        (Ok(done), _) => Ok(done),
        (Err(failure), why) => Err(Refused::failed(failure, &listed.name, listed.limits, why)),
    }
}

/// What an agent sent, as the log keeps it: as it came, up to the size a
/// statement may have.
fn kept(text: &str) -> String {
    let mut end = text.len().min(tabletist_db::agent::MAX_SQL_BYTES);
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    text[..end].to_owned()
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

    fn ten_seconds() -> Limits {
        Limits {
            rows: 500,
            timeout: Duration::from_secs(10),
        }
    }

    #[tokio::test(start_paused = true)]
    async fn work_that_runs_past_its_time_is_ended_and_awaited_all_the_same() {
        let stop = Stop::new();
        // Work that ends only once it is told to, as a gateway's does.
        let work = async {
            stop.ended().await;
            "ended"
        };
        let started = tokio::time::Instant::now();
        assert_eq!(within(&stop, ten_seconds(), work).await, "ended");
        assert_eq!(started.elapsed(), Duration::from_secs(10) + GRACE);
        assert_eq!(stop.why(), Some(Why::Timeout));
    }

    #[tokio::test(start_paused = true)]
    async fn work_the_database_ends_in_its_own_time_is_left_alone() {
        let stop = Stop::new();
        // Past the connection's time, and within what the database is given
        // to end the statement by itself.
        let work = async {
            tokio::time::sleep(Duration::from_secs(11)).await;
            7
        };
        assert_eq!(within(&stop, ten_seconds(), work).await, 7);
        assert_eq!(stop.why(), None);
    }

    #[test]
    fn the_log_keeps_what_was_sent_up_to_the_size_a_statement_may_have() {
        assert_eq!(kept("SELECT 1"), "SELECT 1");
        let long = "é".repeat(tabletist_db::agent::MAX_SQL_BYTES);
        let kept = kept(&long);
        assert!(kept.len() <= tabletist_db::agent::MAX_SQL_BYTES);
        assert!(long.starts_with(&kept));
    }

    #[test]
    fn the_tools_are_told_as_they_are() {
        let names: Vec<&str> = all().iter().map(|tool| tool.name).collect();
        assert_eq!(names, ["list_connections", "describe_schema", "run_query"]);
        for tool in all() {
            assert_eq!(tool.input["type"], json!("object"), "{}", tool.name);
            assert!(!tool.description.is_empty() && !tool.title.is_empty());
        }
    }

    #[test]
    fn a_tool_that_is_not_there_is_answered_with_the_ones_that_are() {
        assert_eq!(
            Invalid::UnknownTool.message(),
            "Unknown tool. Tabletist has list_connections, describe_schema, run_query."
        );
    }

    #[test]
    fn an_argument_is_refused_without_a_word_of_what_it_held() {
        let given = json!({ "connection": "Bookshop", "sql": ["SELECT secret"] });
        let given = given.as_object().unwrap();
        let arguments = Arguments::new("run_query", given, &["connection", "sql"]).unwrap();
        assert_eq!(
            arguments.text("sql").unwrap_err().message(),
            "run_query needs `sql`, a string."
        );
        let given = json!({ "limit": "SELECT secret" });
        let refused = Arguments::new("run_query", given.as_object().unwrap(), &["sql"]);
        assert_eq!(
            refused.err().unwrap().message(),
            "run_query takes sql and nothing else."
        );
    }
}
