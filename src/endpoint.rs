//! How `tabletist mcp` reaches the running app: a socket in a directory
//! private to the user (`AppDirs::agents_dir`), and a key beside it that
//! the relay sends first.
//!
//! The relay sends one line, a [`Hello`] with a challenge. The app answers
//! with its proof that it holds the key, and a challenge of its own
//! ([`Answer::Prove`]); the relay checks it before it says anything else,
//! and sends its own proof ([`Reply`]); the app answers [`Answer::Ok`].
//! Neither sends the key, so what does not hold it learns nothing: not the
//! key from a relay that reached something else at the endpoint, which on
//! Windows another account can make once the app's pipe is gone, nor an
//! agent's requests. After that the bytes are the agent's and the
//! server's, and the relay understands none of them: the protocol lives in
//! the app.

use std::time::Duration;

use serde::{Deserialize, Serialize};
use tokio::io::{AsyncRead, AsyncReadExt};

/// Whether agents can reach the app on this system. Not on Windows yet: its
/// named pipe and its relay are a run of their own.
pub const AVAILABLE: bool = cfg!(unix);

/// What the agent is told when the app is not there to reach, or stops as
/// the relay says its line.
pub const NOT_RUNNING: &str = "Tabletist is not running, or agents are off in it. Open Tabletist, turn agents on, then reconnect.";

/// What the agent is told when what answered at the endpoint could not
/// prove it holds the key: not this Tabletist, or a key file that changed
/// since. It was told nothing.
pub const NOT_PROVEN: &str = "What answered at Tabletist's endpoint did not prove it holds Tabletist's key, and was told nothing. Open Tabletist, then start the agent's MCP server again.";

/// Why a server does not start where another Tabletist serves agents
/// already: the person is told it after the app's own words.
pub const ANOTHER_SERVES: &str = "another Tabletist is serving agents already";

/// The socket's name in the directory.
pub const SOCKET: &str = "mcp.sock";

/// The name of the file a server holds locked while it listens in the
/// directory: a second one that finds it locked stops before it touches
/// the socket or the key. It is never removed: a lock is on the file, and
/// a start that removed it would lock one another start no longer sees.
pub const LOCK: &str = "mcp.lock";

/// The key's name in the directory: new each time the server starts.
pub const KEY: &str = "mcp.key";

/// On Windows, the name of the file in the directory that holds the pipe's
/// name, which is new each time the server starts: no other account can
/// know it before the server makes the pipe, and so none can make one of
/// that name first and be sent the key.
pub const PIPE: &str = "mcp.pipe";

/// How every pipe's name starts. The rest is [`new_pipe`]'s hex.
pub const PIPE_PREFIX: &str = r"\\.\pipe\tabletist-mcp-";

/// The longest path of a socket macOS takes (its `sun_path` holds 104
/// bytes, the last of them the end of the string).
pub const MAX_SOCKET_PATH: usize = 103;

/// The longest line of a [`Hello`] or an [`Answer`]: a working directory
/// fits many times over.
pub const MAX_LINE: usize = 16 * 1024;

/// How long either side waits for the other's line.
pub const LINE_TIME: Duration = Duration::from_secs(5);

/// What the relay says first.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Hello {
    /// The relay's version, for the app's log.
    pub version: String,
    /// What the app proves it holds the key with ([`new_nonce`]).
    pub challenge: String,
}

/// What the app answers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Answer {
    /// The app holds the key: `proof` is [`prove`]'s for the relay's
    /// challenge, from the app's [`Side`]. The relay proves it holds the
    /// key too, for `challenge`.
    Prove { proof: String, challenge: String },
    /// The session begins with the next byte.
    Ok,
    /// It does not, and why, for the person: the relay passes it on to the
    /// agent.
    Refused(String),
}

/// The relay's proof that it holds the key, for the app's challenge, and
/// what it tells only an app that has proved it holds the key too.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Reply {
    pub proof: String,
    /// The directory the agent started the relay in: its last part names
    /// the session's project for the person.
    pub cwd: Option<String>,
}

/// Who proves: each side's proof is its own, so neither can hand back the
/// other's.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    App,
    Relay,
}

/// A new challenge: 122 random bits, as hex.
pub fn new_nonce() -> String {
    uuid::Uuid::new_v4().simple().to_string()
}

/// The proof that `side` holds `key`, for `challenge`: an HMAC-SHA-256 of
/// the side's name and the challenge under the key, as hex.
pub fn prove(key: &str, side: Side, challenge: &str) -> String {
    use hmac::{KeyInit, Mac};
    let mut mac = hmac::Hmac::<sha2::Sha256>::new_from_slice(key.as_bytes())
        .expect("HMAC takes a key of any length");
    let name: &[u8] = match side {
        Side::App => b"tabletist app\n",
        Side::Relay => b"tabletist relay\n",
    };
    mac.update(name);
    mac.update(challenge.as_bytes());
    mac.finalize()
        .into_bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

/// A key: 244 random bits, as two version 4 UUIDs' worth of hex.
pub fn new_key() -> String {
    format!(
        "{}{}",
        uuid::Uuid::new_v4().simple(),
        uuid::Uuid::new_v4().simple()
    )
}

/// A new name for the server's pipe.
pub fn new_pipe() -> String {
    format!("{PIPE_PREFIX}{}", uuid::Uuid::new_v4().simple())
}

/// Whether `name`, as read from [`PIPE`], is a name [`new_pipe`] makes: a
/// pipe on this computer, never a path to another one's.
pub fn is_pipe(name: &str) -> bool {
    name.strip_prefix(PIPE_PREFIX)
        .is_some_and(|rest| rest.len() == 32 && rest.bytes().all(|b| b.is_ascii_hexdigit()))
}

/// Whether `given` is `key`, in a time that does not tell how much of it
/// was right.
pub fn same_key(given: &str, key: &str) -> bool {
    let (given, key) = (given.as_bytes(), key.as_bytes());
    given.len() == key.len()
        && given
            .iter()
            .zip(key)
            .fold(0u8, |differ, (a, b)| differ | (a ^ b))
            == 0
}

/// The name of a session's project: the last part of the directory the
/// agent started in, without control characters, at most 64 characters.
pub fn project(cwd: &str) -> Option<String> {
    let last = std::path::Path::new(cwd).file_name()?.to_string_lossy();
    // Nothing that moves the cursor, turns the line around or cannot be
    // seen, as an agent's own name (`tabletist_mcp`'s `label`).
    let unseen = |c: char| {
        c.is_control()
            || matches!(
                c,
                '\u{200B}'..='\u{200F}'
                    | '\u{2028}'..='\u{202E}'
                    | '\u{2060}'..='\u{2069}'
                    | '\u{FEFF}'
            )
    };
    let name: String = last.chars().filter(|c| !unseen(*c)).take(64).collect();
    (!name.is_empty()).then_some(name)
}

/// Reads one line of at most `max` bytes, a byte at a time: nothing past
/// its end is taken from `from`, which then carries the session. `None` at
/// the end of the stream before any byte, and an error for a line too long
/// or cut short.
pub async fn read_line<R: AsyncRead + Unpin>(
    from: &mut R,
    max: usize,
) -> std::io::Result<Option<Vec<u8>>> {
    let mut line = Vec::new();
    loop {
        let mut byte = [0u8; 1];
        if from.read(&mut byte).await? == 0 {
            return match line.is_empty() {
                true => Ok(None),
                false => Err(std::io::ErrorKind::UnexpectedEof.into()),
            };
        }
        if byte[0] == b'\n' {
            return Ok(Some(line));
        }
        if line.len() == max {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "the line is too long",
            ));
        }
        line.push(byte[0]);
    }
}

/// The agents Settings says how to connect: each is told in its own way.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Agent {
    /// A command to run once.
    #[default]
    ClaudeCode,
    /// An entry of its `mcp.json`.
    Cursor,
    /// Any agent that starts an MCP server over stdio: the command it
    /// starts.
    Other,
}

impl Agent {
    /// In the order Settings shows them.
    pub const ALL: [Agent; 3] = [Self::ClaudeCode, Self::Cursor, Self::Other];
}

/// What connects each agent, once worked out.
struct Setups {
    claude_code: String,
    cursor: String,
    other: String,
}

static SETUPS: std::sync::OnceLock<Setups> = std::sync::OnceLock::new();

/// [`agent_command`] where it is worked out already: what the UI reads.
pub fn agent_command_ready() -> Option<&'static str> {
    agent_setup_ready(Agent::ClaudeCode)
}

/// What connects `agent`, where it is worked out already.
pub fn agent_setup_ready(agent: Agent) -> Option<&'static str> {
    let setups = SETUPS.get()?;
    Some(match agent {
        Agent::ClaudeCode => &setups.claude_code,
        Agent::Cursor => &setups.cursor,
        Agent::Other => &setups.other,
    })
}

/// The command that connects Claude Code, the agent the boards name, to
/// this Tabletist: `claude mcp add tabletist -- tabletist mcp`, with this
/// program's whole path where `tabletist` on the `PATH` is not this
/// program (the app bundle on macOS). Worked out once, with what connects
/// the other agents, by looking at the disk: on the backend's runtime as
/// it starts (`Backend::start_with`), never on the UI thread, which asks
/// [`agent_setup_ready`].
pub fn agent_command() -> &'static str {
    &SETUPS
        .get_or_init(|| {
            let exe = std::env::current_exe().ok();
            let on_path = std::env::var_os("PATH").and_then(|path| {
                std::env::split_paths(&path)
                    .map(|dir| dir.join(EXE))
                    .find(|file| file.is_file())
            });
            let program = program_for(exe.as_deref(), on_path.as_deref());
            let relay = relay_of(&program);
            Setups {
                claude_code: command_for(&relay),
                cursor: cursor_entry(&program),
                other: relay,
            }
        })
        .claude_code
}

/// This program's file name.
const EXE: &str = if cfg!(windows) {
    "tabletist.exe"
} else {
    "tabletist"
};

/// What adds `relay` to Claude Code as Tabletist's MCP server.
fn command_for(relay: &str) -> String {
    format!("claude mcp add tabletist -- {relay}")
}

/// Cursor's entry for Tabletist in its `mcp.json`: `program` started with
/// `mcp`.
fn cursor_entry(program: &str) -> String {
    let command = serde_json::to_string(program).unwrap_or_default();
    format!(
        "{{\n  \"mcpServers\": {{\n    \"tabletist\": {{ \"command\": {command}, \"args\": [\"mcp\"] }}\n  }}\n}}"
    )
}

/// The program an agent starts as the relay: `tabletist` when `on_path`,
/// the program a shell finds by that name, is `exe`, this program; else
/// `exe`'s whole path.
fn program_for(exe: Option<&std::path::Path>, on_path: Option<&std::path::Path>) -> String {
    let same = |a: &std::path::Path, b: &std::path::Path| match (
        std::fs::canonicalize(a),
        std::fs::canonicalize(b),
    ) {
        (Ok(a), Ok(b)) => a == b,
        _ => a == b,
    };
    match exe {
        Some(exe) if !on_path.is_some_and(|found| same(exe, found)) => {
            exe.to_string_lossy().into_owned()
        }
        _ => "tabletist".to_owned(),
    }
}

/// How a shell starts the relay of `program`.
fn relay_of(program: &str) -> String {
    format!("{} mcp", shell_word(program))
}

/// How an agent starts the relay: `tabletist mcp`, or this program's whole
/// path as a shell reads it (see [`program_for`]).
#[cfg(test)]
fn relay_for(exe: Option<&std::path::Path>, on_path: Option<&std::path::Path>) -> String {
    relay_of(&program_for(exe, on_path))
}

/// `word` as a POSIX shell reads it back: as it is when nothing in it is
/// special, else in single quotes.
fn shell_word(word: &str) -> String {
    let plain = !word.is_empty()
        && word
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "/._-+:@%,=".contains(c));
    match plain {
        true => word.to_owned(),
        false => format!("'{}'", word.replace('\'', "'\\''")),
    }
}

/// `value` as one line of JSON, with its end.
pub fn line_of<T: Serialize>(value: &T) -> Vec<u8> {
    let mut line = serde_json::to_vec(value).unwrap_or_default();
    line.push(b'\n');
    line
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_key_is_long_new_each_time_and_compared_whole() {
        let key = new_key();
        assert_eq!(key.len(), 64);
        assert!(key.chars().all(|c| c.is_ascii_hexdigit()), "{key}");
        assert_ne!(key, new_key());
        assert!(same_key(&key, &key));
        assert!(!same_key(&key[..63], &key));
        assert!(!same_key(&format!("{key}0"), &key));
        let mut other = key.clone().into_bytes();
        other[63] = if other[63] == b'0' { b'1' } else { b'0' };
        assert!(!same_key(&String::from_utf8(other).unwrap(), &key));
        assert!(!same_key("", &key));
    }

    #[test]
    fn a_project_is_the_last_part_of_the_directory_and_nothing_odd() {
        assert_eq!(
            project("/home/ada/src/bookshop").as_deref(),
            Some("bookshop")
        );
        assert_eq!(project("/").as_deref(), None);
        assert_eq!(project("").as_deref(), None);
        assert_eq!(
            project("/home/ada/book\u{1b}[2Jshop").as_deref(),
            Some("book[2Jshop")
        );
        assert_eq!(
            project(&format!("/x/{}", "a".repeat(100))).unwrap().len(),
            64
        );
        assert_eq!(
            project("/home/ada/repo\u{202e}gnp.exe\u{200b}").as_deref(),
            Some("repognp.exe")
        );
    }

    #[test]
    fn the_lines_are_json_and_an_answer_names_its_kind() {
        let hello = Hello {
            version: "1.0.0".into(),
            challenge: new_nonce(),
        };
        let line = line_of(&hello);
        assert!(line.ends_with(b"\n"));
        assert_eq!(
            serde_json::from_slice::<Hello>(&line[..line.len() - 1]).unwrap(),
            hello
        );
        assert_eq!(line_of(&Answer::Ok), b"\"ok\"\n");
        assert_eq!(
            line_of(&Answer::Refused("no".into())),
            b"{\"refused\":\"no\"}\n"
        );
        assert_eq!(
            line_of(&Answer::Prove {
                proof: "p".into(),
                challenge: "c".into()
            }),
            b"{\"prove\":{\"proof\":\"p\",\"challenge\":\"c\"}}\n"
        );
        // A newer relay may say more: what this one does not know is
        // passed over.
        let newer: Reply =
            serde_json::from_str(r#"{"proof":"p","cwd":null,"shell":"zsh"}"#).unwrap();
        assert_eq!(newer.cwd, None);
    }

    #[test]
    fn a_proof_is_the_keys_and_the_sides_and_the_challenges() {
        let key = new_key();
        let challenge = new_nonce();
        let proof = prove(&key, Side::App, &challenge);
        // HMAC-SHA-256, as hex.
        assert_eq!(proof.len(), 64);
        assert!(proof.bytes().all(|b| b.is_ascii_hexdigit()), "{proof}");
        assert_eq!(prove(&key, Side::App, &challenge), proof);
        // Another key, another side or another challenge: another proof.
        // The app's cannot be handed back as the relay's.
        assert_ne!(prove(&new_key(), Side::App, &challenge), proof);
        assert_ne!(prove(&key, Side::Relay, &challenge), proof);
        assert_ne!(prove(&key, Side::App, &new_nonce()), proof);
        // RFC 4231's second case, under the app's side's name: the HMAC is
        // the standard one.
        use hmac::{KeyInit, Mac};
        let mut mac = hmac::Hmac::<sha2::Sha256>::new_from_slice(b"Jefe").unwrap();
        mac.update(b"what do ya want for nothing?");
        let standard: String = mac
            .finalize()
            .into_bytes()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect();
        assert_eq!(
            standard,
            "5bdcc146bf60754e6a042426089575c75a003f089d2739839dec58b964ec3843"
        );
    }

    #[test]
    fn the_command_starts_this_program_and_quotes_its_path_where_it_must() {
        use std::path::Path;
        let exe = Path::new("/usr/bin/tabletist");
        assert_eq!(relay_for(Some(exe), Some(exe)), "tabletist mcp");
        // Another tabletist on the PATH, or none: this one, by its path.
        let other = Path::new("/opt/old/tabletist");
        assert_eq!(relay_for(Some(exe), Some(other)), "/usr/bin/tabletist mcp");
        assert_eq!(relay_for(Some(exe), None), "/usr/bin/tabletist mcp");
        let bundle = Path::new("/Applications/Tabletist.app/Contents/MacOS/tabletist");
        assert_eq!(
            relay_for(Some(bundle), None),
            "/Applications/Tabletist.app/Contents/MacOS/tabletist mcp"
        );
        // Quoted where it must be, as this system's shell reads it.
        let spaced = Path::new("/Users/ada/My Apps/Tabletist's/tabletist");
        assert_eq!(
            relay_for(Some(spaced), None),
            "'/Users/ada/My Apps/Tabletist'\\''s/tabletist' mcp"
        );
        // Not known at all: the name.
        assert_eq!(relay_for(None, None), "tabletist mcp");
        assert_eq!(
            command_for("tabletist mcp"),
            "claude mcp add tabletist -- tabletist mcp"
        );
    }

    #[test]
    fn a_pipes_name_is_new_each_time_and_one_read_back_is_checked() {
        let name = new_pipe();
        assert!(name.starts_with(r"\\.\pipe\tabletist-mcp-"), "{name}");
        assert!(is_pipe(&name));
        assert_ne!(name, new_pipe());
        // Nothing else is taken for one: another computer's pipe least of
        // all, which the key would be sent to.
        for other in [
            r"\\server\pipe\tabletist-mcp-0123456789abcdef0123456789abcdef",
            r"\\.\pipe\tabletist-mcp-",
            r"\\.\pipe\tabletist-mcp-0123456789abcdef0123456789abcdeg",
            r"\\.\pipe\tabletist-mcp-0123456789abcdef0123456789abcdef0",
            "",
        ] {
            assert!(!is_pipe(other), "{other}");
        }
    }

    #[test]
    fn the_command_is_ready_once_it_is_worked_out() {
        let command = agent_command();
        assert_eq!(agent_command_ready(), Some(command));
        assert!(
            command.starts_with("claude mcp add tabletist -- "),
            "{command}"
        );
        assert!(command.ends_with(" mcp"), "{command}");
        // The others start the same relay.
        let relay = agent_setup_ready(Agent::Other).unwrap();
        assert_eq!(command, command_for(relay));
        let cursor = agent_setup_ready(Agent::Cursor).unwrap();
        assert!(cursor.contains("\"args\": [\"mcp\"]"), "{cursor}");
    }

    #[test]
    fn cursors_entry_starts_the_program_by_its_path_as_json_writes_it() {
        let entry = cursor_entry("tabletist");
        let read: serde_json::Value = serde_json::from_str(&entry).unwrap();
        let server = &read["mcpServers"]["tabletist"];
        assert_eq!(server["command"], "tabletist");
        assert_eq!(server["args"], serde_json::json!(["mcp"]));
        // A path a shell would quote is written as JSON writes it.
        let spaced = "C:\\Program Files\\Tabletist's \"app\"\\tabletist";
        let read: serde_json::Value = serde_json::from_str(&cursor_entry(spaced)).unwrap();
        assert_eq!(read["mcpServers"]["tabletist"]["command"], spaced);
    }

    #[tokio::test]
    async fn a_line_is_read_to_its_end_and_no_further() {
        let mut from: &[u8] = b"{\"key\":1}\nafter";
        let line = read_line(&mut from, MAX_LINE).await.unwrap();
        assert_eq!(line.as_deref(), Some(&b"{\"key\":1}"[..]));
        assert_eq!(from, b"after");
        let mut empty: &[u8] = b"";
        assert_eq!(read_line(&mut empty, MAX_LINE).await.unwrap(), None);
        let mut cut: &[u8] = b"no end";
        assert!(read_line(&mut cut, MAX_LINE).await.is_err());
        let long = vec![b'a'; 11];
        let mut long: &[u8] = &long;
        assert!(read_line(&mut long, 10).await.is_err());
        let mut fits: &[u8] = b"aaaaaaaaaa\n";
        assert_eq!(read_line(&mut fits, 10).await.unwrap().unwrap().len(), 10);
    }
}
