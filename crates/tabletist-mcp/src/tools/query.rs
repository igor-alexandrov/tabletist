//! `run_query`: one statement that reads.

use serde_json::{Map, Value as Json, json};
use tabletist_db::agent::{self, Param};

use super::{Arguments, Done, Invalid, Note, Tool, find, kept, schema, settle, within};
use crate::answer::{self, MAX_ANSWER_BYTES};
use crate::gateway::{Gateway, Session};
use crate::refusal::{Refused, code};
use crate::stop::Stop;

pub const NAME: &str = "run_query";

pub fn tool() -> Tool {
    Tool {
        name: NAME,
        title: "Run a query",
        description: "Runs one SELECT (or WITH … SELECT, VALUES, or an EXPLAIN of one) on a \
                      connection, in a read-only transaction that is rolled back. Nothing it \
                      runs can change data. Write values as parameters, in the form \
                      list_connections gives for the connection, and send them in `params`. \
                      The answer has `columns` and `rows`, at most the connection's rows and \
                      within its timeout; `truncated` says that rows were left out, and \
                      `truncated_by` whether by the row limit or by the answer's size. A text \
                      longer than 2,000 characters comes as {\"cut\": its start, \
                      \"characters\": its length}, and a binary value as {\"binary_bytes\": \
                      its size}.",
        input: schema(
            json!({
                "connection": {
                    "type": "string",
                    "description": "The connection's name, from list_connections.",
                },
                "sql": {
                    "type": "string",
                    "description": "One statement that reads.",
                },
                "params": {
                    "type": "array",
                    "description": "The values for the statement's parameters, in order.",
                    "items": { "type": ["string", "number", "boolean", "null"] },
                },
            }),
            &["connection", "sql"],
        ),
    }
}

/// How the log says a read was run.
const RAN_AS: &str = "read-only transaction";

pub async fn call<G: Gateway>(
    gateway: &G,
    stop: &Stop,
    arguments: &Map<String, Json>,
) -> Result<Done, Invalid> {
    let arguments = Arguments::new(NAME, arguments, &["connection", "sql", "params"])?;
    let connection = arguments.text("connection")?;
    let sql = arguments.text("sql")?;
    let given = arguments.list("params")?.unwrap_or_default();
    let mut note = Note {
        sql: Some(kept(sql)),
        params: (!given.is_empty()).then(|| kept_values(given)),
        ..Note::default()
    };
    let answer = run(gateway, stop, connection, sql, given, &mut note).await;
    Ok(Done { answer, note })
}

async fn run<G: Gateway>(
    gateway: &G,
    stop: &Stop,
    connection: &str,
    sql: &str,
    given: &[Json],
    note: &mut Note,
) -> Result<Map<String, Json>, Refused> {
    let listed = find(gateway, connection)?;
    note.connection = Some((listed.id.clone(), listed.name.clone()));
    let params = params(given)?;
    // The check parses on a thread of its own and blocks until it has.
    let (dialect, text) = (listed.dialect, sql.to_owned());
    let read = tokio::task::spawn_blocking(move || agent::check_read(dialect, &text, &params))
        .await
        .map_err(|_| Refused::from(agent::Refusal::NotParsed))??;
    let session = settle(gateway.open(&listed.id, stop.token()).await, stop, &listed)?;
    note.ran_as = Some(RAN_AS);
    let limits = listed.limits;
    let answer = within(stop, limits, session.read(&read, limits)).await;
    let answer = settle(answer, stop, &listed)?;
    let written = answer::write(&answer, MAX_ANSWER_BYTES).map_err(|_| {
        Refused::new(
            code::TOO_LARGE,
            "The answer's columns alone are more than an answer may hold. Select fewer \
             columns, or give them shorter names.",
        )
    })?;
    note.rows = Some(written.rows as u64);
    note.columns = Some(written.columns as u64);
    note.matched = written.cut.is_none().then_some(written.rows as u64);
    note.took = Some(answer.elapsed);
    Ok(written.json)
}

/// An agent's values as the log keeps them: the list as it came, as JSON.
/// Where that is more than the log keeps of a call, its start and its
/// length, which is JSON too: whoever reads the log reads every entry.
fn kept_values(given: &[Json]) -> String {
    let whole = Json::Array(given.to_vec()).to_string();
    if whole.len() <= agent::MAX_SQL_BYTES {
        return whole;
    }
    // A start that fits once it is written as a string: a quote mark in it
    // takes two bytes then, so the start is halved until it does.
    let mut end = agent::MAX_SQL_BYTES / 2;
    loop {
        while !whole.is_char_boundary(end) {
            end -= 1;
        }
        let kept = json!({ "cut": &whole[..end], "bytes": whole.len() }).to_string();
        if kept.len() <= agent::MAX_SQL_BYTES {
            return kept;
        }
        end /= 2;
    }
}

/// An agent's values as the check takes them. A value is a string, a
/// number, true, false or null.
fn params(given: &[Json]) -> Result<Vec<Param>, Refused> {
    given
        .iter()
        .enumerate()
        .map(|(index, value)| {
            let position = index + 1;
            let refused = |what: &str| {
                Refused::new(
                    code::INVALID_PARAMS,
                    format!("Parameter {position} is {what}."),
                )
            };
            match value {
                Json::Null => Ok(Param::Null),
                Json::Bool(value) => Ok(Param::Bool(*value)),
                Json::String(value) => Ok(Param::Text(value.clone())),
                Json::Number(number) => match (number.as_i64(), number.as_u64()) {
                    (Some(value), _) => Ok(Param::Int(value)),
                    // Past what a signed 64-bit integer holds, and no
                    // driver here sends an unsigned one.
                    (None, Some(_)) => Err(refused(
                        "a whole number too large to send as one. Send it as a string",
                    )),
                    (None, None) => number
                        .as_f64()
                        .map(Param::Float)
                        .ok_or_else(|| refused("not a number Tabletist can send")),
                },
                Json::Array(_) => Err(refused(
                    "a list. A parameter is a string, a number, true, false or null",
                )),
                Json::Object(_) => Err(refused(
                    "an object. A parameter is a string, a number, true, false or null",
                )),
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_value_is_a_string_a_number_a_boolean_or_null() {
        let given = json!(["it's", 7, -9_223_372_036_854_775_808_i64, 1.5, true, null]);
        assert_eq!(
            params(given.as_array().unwrap()).unwrap(),
            [
                Param::Text("it's".into()),
                Param::Int(7),
                Param::Int(i64::MIN),
                Param::Float(1.5),
                Param::Bool(true),
                Param::Null,
            ]
        );
    }

    #[test]
    fn the_log_keeps_values_whole_or_their_start_and_always_as_json() {
        let short = [json!("it's"), json!(7), Json::Null];
        assert_eq!(kept_values(&short), r#"["it's",7,null]"#);

        // Every letter a quote mark: written as a string it is twice as
        // long, and what is kept still fits.
        let long = [json!("\"".repeat(80_000)), json!("é".repeat(40_000))];
        let whole = Json::Array(long.to_vec()).to_string();
        let kept = kept_values(&long);
        assert!(kept.len() <= agent::MAX_SQL_BYTES, "{}", kept.len());
        let read: Json = serde_json::from_str(&kept).unwrap();
        assert_eq!(read["bytes"], json!(whole.len()));
        let start = read["cut"].as_str().unwrap();
        assert!(
            whole.starts_with(start) && start.len() >= 10_000,
            "{}",
            start.len()
        );
    }

    #[test]
    fn a_value_that_is_none_of_them_is_refused_by_its_position() {
        let cases = [
            (
                json!([1, [2]]),
                "Parameter 2 is a list. A parameter is a string, a number, true, false or null.",
            ),
            (
                json!([{ "a": 1 }]),
                "Parameter 1 is an object. A parameter is a string, a number, true, false or null.",
            ),
            (
                json!([1, 2, 18_446_744_073_709_551_615_u64]),
                "Parameter 3 is a whole number too large to send as one. Send it as a string.",
            ),
        ];
        for (given, said) in cases {
            let refused = params(given.as_array().unwrap()).unwrap_err();
            assert_eq!(refused.code, "invalid_params");
            assert_eq!(refused.message, said);
        }
    }
}
