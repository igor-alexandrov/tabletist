//! Runs the built binary, the way a launcher or a packager would.

// `allow-unwrap-in-tests` does not cover helpers in an integration-test crate.
#![allow(clippy::unwrap_used)]

use std::process::Command;

fn tabletist() -> Command {
    Command::new(env!("CARGO_BIN_EXE_tabletist"))
}

#[test]
fn version_prints_the_package_version() {
    let output = tabletist()
        .arg("--version")
        .output()
        .expect("run tabletist");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(
        stdout.trim(),
        format!("tabletist {}", env!("CARGO_PKG_VERSION"))
    );
}

#[test]
fn demo_flags_require_demo() {
    let output = tabletist()
        .args(["--demo-shot", "out.png"])
        .output()
        .expect("run tabletist");
    assert!(
        !output.status.success(),
        "--demo-shot without --demo must be rejected"
    );
}

/// `tabletist mcp`, the relay an agent starts, against the app's backend
/// serving agents in a folder of the test's own.
#[cfg(unix)]
mod mcp {
    use std::io::{BufRead, BufReader, Read, Write};
    use std::path::{Path, PathBuf};
    use std::process::{Child, Stdio};
    use std::sync::mpsc;
    use std::time::{Duration, Instant};

    use serde_json::{Value, json};
    use tabletist::backend::{
        AgentsCommand, AgentsEvent, Backend, Command, Event, Waker, agentable,
    };
    use tabletist::connections::{
        AgentAccess, Agents, ConnectionId, PasswordMode, SavedConnection,
    };
    use tabletist::env::Environment;
    use tabletist::secrets::Keyring;
    use tabletist_db::{ConnectSpec, HostKeys};

    use super::tabletist;

    const WAIT: Duration = Duration::from_secs(20);

    /// In the database's path, which nothing an agent is sent may hold.
    const MARKER: &str = "PRIVATE-7f3a";

    fn relay(endpoint: &Path, cwd: &Path) -> Child {
        tabletist()
            .arg("mcp")
            .arg("--endpoint")
            .arg(endpoint)
            .current_dir(cwd)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("run tabletist mcp")
    }

    fn line(message: &Value) -> Vec<u8> {
        let mut line = serde_json::to_vec(message).unwrap();
        line.push(b'\n');
        line
    }

    fn initialize(id: u64) -> Value {
        json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-11-25",
                "capabilities": {},
                "clientInfo": { "name": "claude-code", "version": "1" },
            },
        })
    }

    fn call(id: u64, name: &str, arguments: Value) -> Value {
        json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": "tools/call",
            "params": { "name": name, "arguments": arguments },
        })
    }

    /// The lines of the child's stdout as they come, read on a thread of
    /// their own so that a test that waits cannot wait for good.
    fn lines_of(child: &mut Child) -> mpsc::Receiver<String> {
        let stdout = child.stdout.take().unwrap();
        let (send, lines) = mpsc::channel();
        std::thread::spawn(move || {
            for line in BufReader::new(stdout).lines() {
                let Ok(line) = line else { return };
                if send.send(line).is_err() {
                    return;
                }
            }
        });
        lines
    }

    fn stderr_of(child: &mut Child) -> String {
        let mut stderr = String::new();
        child
            .stderr
            .take()
            .unwrap()
            .read_to_string(&mut stderr)
            .unwrap();
        stderr
    }

    /// Waits for the child to end, at most [`WAIT`].
    fn ended(child: &mut Child) -> std::process::ExitStatus {
        let deadline = Instant::now() + WAIT;
        loop {
            if let Some(status) = child.try_wait().unwrap() {
                return status;
            }
            assert!(Instant::now() < deadline, "tabletist mcp did not end");
            std::thread::sleep(Duration::from_millis(20));
        }
    }

    fn agents_said(backend: &mut Backend) -> AgentsEvent {
        let deadline = Instant::now() + WAIT;
        loop {
            for event in backend.poll() {
                // The sessions as they change are said beside what the
                // server does.
                match event {
                    Event::Agents(AgentsEvent::Sessions(_)) => {}
                    Event::Agents(event) => return event,
                    _ => {}
                }
            }
            assert!(
                Instant::now() < deadline,
                "the backend said nothing of agents"
            );
            std::thread::sleep(Duration::from_millis(20));
        }
    }

    /// Asks the relay that answers the agent's first request with why it
    /// stopped, and returns that and what stderr says.
    fn refused(endpoint: &Path, cwd: &Path) -> (String, String) {
        let mut child = relay(endpoint, cwd);
        child
            .stdin
            .take()
            .unwrap()
            .write_all(&line(&initialize(7)))
            .unwrap();
        let lines = lines_of(&mut child);
        let status = ended(&mut child);
        assert_eq!(status.code(), Some(1));
        let said: Vec<String> = lines.try_iter().collect();
        assert_eq!(said.len(), 1, "one line and nothing else: {said:?}");
        let answer: Value = serde_json::from_str(&said[0]).unwrap();
        assert_eq!(answer["id"], 7);
        let message = answer["error"]["message"].as_str().unwrap().to_owned();
        (message, stderr_of(&mut child))
    }

    #[test]
    fn without_the_app_the_first_request_is_answered_with_why_and_it_ends() {
        let dir = tempfile::tempdir().unwrap();
        let (message, stderr) = refused(&dir.path().join("run"), dir.path());
        assert_eq!(message, tabletist::endpoint::NOT_RUNNING);
        assert!(
            stderr.contains(tabletist::endpoint::NOT_RUNNING),
            "{stderr}"
        );
    }

    #[test]
    fn an_agent_reads_through_the_relay_and_nothing_private_comes_back() {
        let dir = tempfile::tempdir().unwrap();
        let database = dir.path().join(format!("{MARKER}.db"));
        tabletist_db::fixtures::write_sqlite_bookshop(&database).unwrap();
        let connection = SavedConnection {
            id: ConnectionId("shop".into()),
            name: "Bookshop".into(),
            environment: Environment::Dev,
            read_only: None,
            password: PasswordMode::None,
            ssh_secret: PasswordMode::None,
            spec: ConnectSpec::sqlite(database),
            agents: Agents {
                access: AgentAccess::Read,
                ..Agents::default()
            },
        };
        let endpoint: PathBuf = dir.path().join("run");
        let log = dir.path().join("agents.sqlite3");
        let mut backend = Backend::start_with(Waker::default(), Keyring::memory());
        backend.send(Command::Agents(AgentsCommand::Policy {
            connections: agentable(&[connection]),
            host_keys: HostKeys::default(),
        }));
        backend.send(Command::Agents(AgentsCommand::Start {
            log: log.clone(),
            log_days: 30,
            endpoint: Some(endpoint.clone()),
        }));
        assert_eq!(
            agents_said(&mut backend),
            AgentsEvent::Started { kept_aside: None }
        );
        let project = dir.path().join("bookshop-repo");
        std::fs::create_dir(&project).unwrap();
        let mut child = relay(&endpoint, &project);
        let mut stdin = child.stdin.take().unwrap();
        let lines = lines_of(&mut child);
        let mut seen = String::new();
        let mut answer = |message: Value, answered: bool| {
            stdin.write_all(&line(&message)).unwrap();
            if !answered {
                return Value::Null;
            }
            let said = lines.recv_timeout(WAIT).expect("an answer");
            seen.push_str(&said);
            serde_json::from_str::<Value>(&said).unwrap()
        };
        assert!(answer(initialize(0), true)["result"].is_object());
        // A notification, which nothing answers.
        let initialized = json!({ "jsonrpc": "2.0", "method": "notifications/initialized" });
        answer(initialized, false);
        let listed = answer(call(1, "list_connections", json!({})), true);
        let text = listed["result"]["content"][0]["text"]
            .as_str()
            .unwrap()
            .to_owned();
        assert!(text.contains("\"name\":\"Bookshop · dev\""), "{text}");
        let ran = answer(
            call(
                2,
                "run_query",
                json!({ "connection": "Bookshop · dev", "sql": "SELECT count(*) FROM books" }),
            ),
            true,
        );
        let text = ran["result"]["content"][0]["text"]
            .as_str()
            .unwrap()
            .to_owned();
        assert!(text.contains("\"rows\":[[12]]"), "{text}");
        // A table that is not there: the database's error comes back.
        let failed = answer(
            call(
                3,
                "run_query",
                json!({ "connection": "Bookshop · dev", "sql": "SELECT * FROM no_such_table" }),
            ),
            true,
        );
        assert_eq!(failed["result"]["isError"], true, "{failed}");
        // The agent's end ends the session, and the relay with it.
        drop(stdin);
        assert!(ended(&mut child).success());
        let stderr = stderr_of(&mut child);
        assert!(!seen.contains(MARKER), "{seen}");
        assert!(!stderr.contains(MARKER), "{stderr}");
        // The app was told of the session, and where it works.
        let deadline = Instant::now() + WAIT;
        let told = loop {
            let told = backend.poll().into_iter().any(|event| {
                matches!(&event, Event::Agents(AgentsEvent::Sessions(open))
                    if open.iter().any(|session| session.connections == ["shop"]
                        && session.project.as_deref() == Some("bookshop-repo")))
            });
            if told || Instant::now() > deadline {
                break told;
            }
            std::thread::sleep(Duration::from_millis(20));
        };
        assert!(told, "the backend says where the session works");
        // The log has the calls, under the project's name.
        let runtime = tokio::runtime::Runtime::new().unwrap();
        let entries = runtime.block_on(async {
            tabletist_mcp::Log::open(&log)
                .unwrap()
                .log
                .latest(10)
                .await
                .unwrap()
        });
        assert_eq!(entries.len(), 3);
        assert!(
            entries
                .iter()
                .all(|entry| entry.project.as_deref() == Some("bookshop-repo")),
            "{entries:?}"
        );
        // A key that is not the one the app made.
        std::fs::write(endpoint.join("mcp.key"), "0".repeat(64)).unwrap();
        let (message, _) = refused(&endpoint, &project);
        assert!(message.contains("key"), "{message}");
        // Stopped, nothing answers: as if the app were not running.
        backend.send(Command::Agents(AgentsCommand::Stop));
        assert_eq!(agents_said(&mut backend), AgentsEvent::Stopped);
        assert!(!endpoint.join("mcp.sock").exists());
        let (message, _) = refused(&endpoint, &project);
        assert_eq!(message, tabletist::endpoint::NOT_RUNNING);
    }
}
