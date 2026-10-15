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

use crate::endpoint::{self, NOT_RUNNING, Side};

/// How long the relay tries the endpoint: the app may be starting.
#[cfg(any(unix, windows))]
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

/// The app's socket.
#[cfg(unix)]
type Stream = tokio::net::UnixStream;

/// The app's pipe.
#[cfg(windows)]
type Stream = tokio::net::windows::named_pipe::NamedPipeClient;

/// Connects to the app's socket in `dir`.
#[cfg(unix)]
async fn connect(dir: &Path) -> std::io::Result<Stream> {
    tokio::net::UnixStream::connect(dir.join(endpoint::SOCKET)).await
}

/// Connects to the app's pipe, by the name its server wrote in `dir`. A
/// name that is not one the app makes is not tried: the key would go to
/// whatever it names.
#[cfg(windows)]
async fn connect(dir: &Path) -> std::io::Result<Stream> {
    let name = std::fs::read_to_string(dir.join(endpoint::PIPE))?;
    let name = name.trim();
    if !endpoint::is_pipe(name) {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "not a pipe of Tabletist's",
        ));
    }
    // Busy while the server makes its next instance: tried again.
    tokio::net::windows::named_pipe::ClientOptions::new().open(name)
}

/// The app's endpoint, once it has taken the key. Why not, for the agent.
#[cfg(any(unix, windows))]
async fn reach(dir: &Path) -> Result<Stream, String> {
    use tokio::io::AsyncWriteExt;
    let deadline = tokio::time::Instant::now() + REACH_TIME;
    let mut stream = loop {
        match connect(dir).await {
            Ok(stream) => break stream,
            Err(_) if tokio::time::Instant::now() < deadline => {
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
            Err(_) => return Err(NOT_RUNNING.into()),
        }
    };
    let challenge = endpoint::new_nonce();
    let hello = endpoint::Hello {
        version: env!("CARGO_PKG_VERSION").to_owned(),
        challenge: challenge.clone(),
    };
    stream
        .write_all(&endpoint::line_of(&hello))
        .await
        .map_err(|_| NOT_RUNNING)?;
    let (proof, theirs) = match answer(&mut stream).await {
        Some(endpoint::Answer::Prove { proof, challenge }) => (proof, challenge),
        Some(endpoint::Answer::Refused(said)) => return Err(said),
        Some(endpoint::Answer::Ok) | None => return Err(NO_ANSWER.into()),
    };
    // Read now, the key of the start that answers. What cannot prove it
    // holds it is told nothing: not the relay's proof, nor the agent's
    // first request.
    let key = std::fs::read_to_string(dir.join(endpoint::KEY)).map_err(|_| NOT_RUNNING)?;
    let key = key.trim();
    if !endpoint::same_key(&proof, &endpoint::prove(key, Side::App, &challenge)) {
        return Err(endpoint::NOT_PROVEN.into());
    }
    let reply = endpoint::Reply {
        proof: endpoint::prove(key, Side::Relay, &theirs),
        cwd: std::env::current_dir()
            .ok()
            .map(|cwd| cwd.to_string_lossy().into_owned()),
    };
    stream
        .write_all(&endpoint::line_of(&reply))
        .await
        .map_err(|_| NOT_RUNNING)?;
    match answer(&mut stream).await {
        Some(endpoint::Answer::Ok) => Ok(stream),
        Some(endpoint::Answer::Refused(said)) => Err(said),
        Some(endpoint::Answer::Prove { .. }) | None => Err(NO_ANSWER.into()),
    }
}

/// What the relay says when the app says nothing it can read.
#[cfg(any(unix, windows))]
const NO_ANSWER: &str = "Tabletist did not answer the relay.";

/// The app's next line, within its time.
#[cfg(any(unix, windows))]
async fn answer(stream: &mut Stream) -> Option<endpoint::Answer> {
    let read = tokio::time::timeout(
        endpoint::LINE_TIME,
        endpoint::read_line(stream, endpoint::MAX_LINE),
    )
    .await;
    match read {
        Ok(Ok(Some(line))) => serde_json::from_slice(&line).ok(),
        _ => None,
    }
}

/// Nothing to reach here (`endpoint::AVAILABLE`).
#[cfg(not(any(unix, windows)))]
async fn reach(_dir: &Path) -> Result<std::convert::Infallible, String> {
    Err("Agents cannot reach Tabletist on this system.".into())
}

/// Copies the agent's bytes to the app and the app's to the agent, until
/// the app ends the session. On a socket the agent's end only ends what
/// it sends: the app still answers what it was asked. A pipe cannot be
/// closed one way: there the agent's end ends the session, which a client
/// does once it has its answers.
#[cfg(any(unix, windows))]
async fn pump(stream: Stream) -> ExitCode {
    use tokio::io::AsyncWriteExt;
    let (mut from_app, mut to_app) = tokio::io::split(stream);
    let up = async {
        let mut stdin = tokio::io::stdin();
        let _ = tokio::io::copy(&mut stdin, &mut to_app).await;
        let _ = to_app.shutdown().await;
        if cfg!(unix) {
            std::future::pending::<()>().await;
        }
    };
    let down = async {
        let mut stdout = tokio::io::stdout();
        let copied = tokio::io::copy(&mut from_app, &mut stdout).await;
        let flushed = stdout.flush().await;
        copied.is_ok() && flushed.is_ok()
    };
    let done = tokio::select! {
        done = down => done,
        () = up => true,
    };
    match done {
        true => ExitCode::SUCCESS,
        false => ExitCode::FAILURE,
    }
}

#[cfg(not(any(unix, windows)))]
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

    /// Something at the endpoint in `dir` that is not the app: it does not
    /// hold the key in `dir`. Returns what it reads after the relay's
    /// hello, once the relay has stopped.
    #[cfg(any(unix, windows))]
    async fn impostor(dir: &Path) -> tokio::task::JoinHandle<Vec<u8>> {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        std::fs::create_dir_all(dir).unwrap();
        std::fs::write(dir.join(endpoint::KEY), endpoint::new_key()).unwrap();
        #[cfg(unix)]
        let listener = tokio::net::UnixListener::bind(dir.join(endpoint::SOCKET)).unwrap();
        // A pipe of the name a stopped app left in its file, which another
        // account can make once the app's is gone.
        #[cfg(windows)]
        let pipe = {
            let name = endpoint::new_pipe();
            std::fs::write(dir.join(endpoint::PIPE), &name).unwrap();
            tokio::net::windows::named_pipe::ServerOptions::new()
                .create(&name)
                .unwrap()
        };
        tokio::spawn(async move {
            #[cfg(unix)]
            let (mut stream, _) = listener.accept().await.unwrap();
            #[cfg(windows)]
            let mut stream = {
                pipe.connect().await.unwrap();
                pipe
            };
            let hello = endpoint::read_line(&mut stream, endpoint::MAX_LINE)
                .await
                .unwrap()
                .unwrap();
            let hello: endpoint::Hello = serde_json::from_slice(&hello).unwrap();
            let guess = endpoint::Answer::Prove {
                proof: endpoint::prove(&endpoint::new_key(), Side::App, &hello.challenge),
                challenge: endpoint::new_nonce(),
            };
            stream.write_all(&endpoint::line_of(&guess)).await.unwrap();
            let mut rest = Vec::new();
            let _ = stream.read_to_end(&mut rest).await;
            rest
        })
    }

    #[cfg(any(unix, windows))]
    #[tokio::test]
    async fn what_cannot_prove_it_holds_the_key_is_told_nothing() {
        let dir = tempfile::tempdir().unwrap();
        let at = dir.path().join("run");
        let told = impostor(&at).await;
        assert_eq!(
            reach(&at).await.err().as_deref(),
            Some(endpoint::NOT_PROVEN)
        );
        // No proof of the relay's, no working directory, no request.
        assert_eq!(told.await.unwrap(), Vec::<u8>::new());
    }

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
