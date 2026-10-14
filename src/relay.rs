//! `tabletist mcp`: what an agent starts to use Tabletist's connections. It
//! reaches the running app at its endpoint (`crate::endpoint`), and from
//! then on copies bytes both ways and understands none of them.
//!
//! It writes nothing to stdout but the protocol's bytes, opens no log file
//! (the app's would be made anew, and lose its lines), and shows no
//! dialog. Why it stops goes to stderr, and to the agent as the answer to
//! its first request.

use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::Duration;

use crate::endpoint::{self, NOT_RUNNING};

/// How long the relay tries the endpoint: the app may be starting.
#[cfg(unix)]
const REACH_TIME: Duration = Duration::from_secs(2);

/// How long it waits for the agent's first request, to answer why it
/// stops.
const FIRST_REQUEST_TIME: Duration = Duration::from_secs(10);

/// The longest line of the agent's it reads to find that request.
const MAX_REQUEST: usize = 1024 * 1024;

/// Runs the relay to its end. `endpoint` is the directory to reach, for
/// tests: the app's own (`AppDirs::agents_dir`) without it.
pub fn main(endpoint: Option<PathBuf>) -> ExitCode {
    let dir = endpoint.unwrap_or_else(|| crate::paths::AppDirs::discover().agents_dir());
    let runtime = match tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    {
        Ok(runtime) => runtime,
        Err(error) => {
            say(&format!("could not start: {error}"));
            return ExitCode::FAILURE;
        }
    };
    let code = runtime.block_on(relay(&dir));
    // A read of stdin still waits on its thread: it is not waited for.
    runtime.shutdown_background();
    code
}

async fn relay(dir: &Path) -> ExitCode {
    match reach(dir).await {
        Ok(stream) => pump(stream).await,
        Err(said) => {
            stop_with(&said).await;
            ExitCode::FAILURE
        }
    }
}

/// The app's socket, once it has taken the key. Why not, for the agent.
#[cfg(unix)]
async fn reach(dir: &Path) -> Result<tokio::net::UnixStream, String> {
    use tokio::io::AsyncWriteExt;
    let socket = dir.join(endpoint::SOCKET);
    let deadline = tokio::time::Instant::now() + REACH_TIME;
    let mut stream = loop {
        match tokio::net::UnixStream::connect(&socket).await {
            Ok(stream) => break stream,
            Err(_) if tokio::time::Instant::now() < deadline => {
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
            Err(_) => return Err(NOT_RUNNING.into()),
        }
    };
    let key = std::fs::read_to_string(dir.join(endpoint::KEY)).map_err(|_| NOT_RUNNING)?;
    let hello = endpoint::Hello {
        key: key.trim().to_owned(),
        version: env!("CARGO_PKG_VERSION").to_owned(),
        cwd: std::env::current_dir()
            .ok()
            .map(|cwd| cwd.to_string_lossy().into_owned()),
    };
    stream
        .write_all(&endpoint::line_of(&hello))
        .await
        .map_err(|_| NOT_RUNNING)?;
    let answer = tokio::time::timeout(
        endpoint::LINE_TIME,
        endpoint::read_line(&mut stream, endpoint::MAX_LINE),
    )
    .await;
    let answer = match answer {
        Ok(Ok(Some(line))) => serde_json::from_slice::<endpoint::Answer>(&line).ok(),
        _ => None,
    };
    match answer {
        Some(endpoint::Answer::Ok) => Ok(stream),
        Some(endpoint::Answer::Refused(said)) => Err(said),
        None => Err("Tabletist did not answer the relay.".into()),
    }
}

/// Nothing to reach here yet (`endpoint::AVAILABLE`).
#[cfg(not(unix))]
async fn reach(_dir: &Path) -> Result<std::convert::Infallible, String> {
    Err("Agents cannot reach Tabletist on this system yet.".into())
}

/// Copies the agent's bytes to the app and the app's to the agent, until
/// the app ends the session. The agent's end only ends what it sends: the
/// app still answers what it was asked.
#[cfg(unix)]
async fn pump(stream: tokio::net::UnixStream) -> ExitCode {
    use tokio::io::AsyncWriteExt;
    let (mut from_app, mut to_app) = stream.into_split();
    let up = async {
        let mut stdin = tokio::io::stdin();
        let _ = tokio::io::copy(&mut stdin, &mut to_app).await;
        let _ = to_app.shutdown().await;
        std::future::pending::<()>().await;
    };
    let down = async {
        let mut stdout = tokio::io::stdout();
        let copied = tokio::io::copy(&mut from_app, &mut stdout).await;
        let flushed = stdout.flush().await;
        copied.is_ok() && flushed.is_ok()
    };
    let done = tokio::select! {
        done = down => done,
        () = up => unreachable!(),
    };
    match done {
        true => ExitCode::SUCCESS,
        false => ExitCode::FAILURE,
    }
}

#[cfg(not(unix))]
async fn pump(stream: std::convert::Infallible) -> ExitCode {
    match stream {}
}

/// Says why the relay stops: on stderr, and as the error that answers the
/// agent's first request, which is what a client shows. A client that asks
/// nothing in time is told on stderr alone.
async fn stop_with(said: &str) {
    use tokio::io::AsyncWriteExt;
    say(said);
    // Nothing is copied after this: what the buffer takes past a line is
    // the agent's and goes nowhere.
    let mut stdin = tokio::io::BufReader::new(tokio::io::stdin());
    let found = tokio::time::timeout(FIRST_REQUEST_TIME, async {
        loop {
            match endpoint::read_line(&mut stdin, MAX_REQUEST).await {
                Ok(Some(line)) => {
                    if let Some(id) = request_id(&line) {
                        return Some(id);
                    }
                }
                _ => return None,
            }
        }
    })
    .await;
    if let Ok(Some(id)) = found {
        let mut stdout = tokio::io::stdout();
        let _ = stdout.write_all(&refusal(id, said)).await;
        let _ = stdout.flush().await;
    }
}

/// Writes `said` to stderr. Nothing comes of a stderr that is closed: the
/// agent is still answered.
fn say(said: &str) {
    use std::io::Write;
    let _ = writeln!(std::io::stderr(), "tabletist mcp: {said}");
}

/// The id of a request, where `line` is one: a notification has none, and
/// is not answered.
fn request_id(line: &[u8]) -> Option<serde_json::Value> {
    let message: serde_json::Value = serde_json::from_slice(line).ok()?;
    message.get("method")?;
    message.get("id").cloned()
}

/// The error that answers request `id` with `said`, as a line.
fn refusal(id: serde_json::Value, said: &str) -> Vec<u8> {
    endpoint::line_of(&serde_json::json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": { "code": -32000, "message": said },
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_a_request_is_answered_and_with_its_own_id() {
        assert_eq!(
            request_id(br#"{"jsonrpc":"2.0","id":7,"method":"initialize","params":{}}"#),
            Some(serde_json::json!(7))
        );
        assert_eq!(
            request_id(br#"{"jsonrpc":"2.0","id":"a","method":"tools/list"}"#),
            Some(serde_json::json!("a"))
        );
        assert_eq!(
            request_id(br#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#),
            None
        );
        assert_eq!(request_id(br#"{"jsonrpc":"2.0","id":1,"result":{}}"#), None);
        assert_eq!(request_id(b"not json"), None);
        let line = refusal(serde_json::json!(7), NOT_RUNNING);
        let answer: serde_json::Value = serde_json::from_slice(&line).unwrap();
        assert_eq!(answer["id"], 7);
        assert_eq!(answer["error"]["message"], NOT_RUNNING);
        assert!(line.ends_with(b"\n"));
    }
}
