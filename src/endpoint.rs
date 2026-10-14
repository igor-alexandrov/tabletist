//! How `tabletist mcp` reaches the running app: a socket in a directory
//! private to the user (`AppDirs::agents_dir`), and a key beside it that
//! the relay sends first.
//!
//! The relay sends one line, a [`Hello`], and the app answers one line, an
//! [`Answer`]. After that the bytes are the agent's and the server's, and
//! the relay understands none of them: the protocol lives in the app, so an
//! old relay keeps working with a newer one.

use std::time::Duration;

use serde::{Deserialize, Serialize};
use tokio::io::{AsyncRead, AsyncReadExt};

/// Whether agents can reach the app on this system. Not on Windows yet: its
/// named pipe and its relay are a run of their own.
pub const AVAILABLE: bool = cfg!(unix);

/// What the agent is told when the app is not there to reach, or stops as
/// the relay says its line.
pub const NOT_RUNNING: &str = "Tabletist is not running, or agents are off in it. Open Tabletist, turn agents on, then reconnect.";

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
    /// What the app wrote to [`KEY`] when its server started.
    pub key: String,
    /// The relay's version, for the app's log.
    pub version: String,
    /// The directory the agent started the relay in: its last part names
    /// the session's project for the person.
    pub cwd: Option<String>,
}

/// What the app answers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Answer {
    /// The session begins with the next byte.
    Ok,
    /// It does not, and why, for the person: the relay passes it on to the
    /// agent.
    Refused(String),
}

/// A key: 244 random bits, as two version 4 UUIDs' worth of hex.
pub fn new_key() -> String {
    format!(
        "{}{}",
        uuid::Uuid::new_v4().simple(),
        uuid::Uuid::new_v4().simple()
    )
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

/// The command that connects Claude Code, the agent the boards name, to
/// this Tabletist: `claude mcp add tabletist -- tabletist mcp`, with this
/// program's whole path where `tabletist` on the `PATH` is not this
/// program (the app bundle on macOS). Worked out once.
pub fn agent_command() -> &'static str {
    static COMMAND: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    COMMAND.get_or_init(|| {
        let exe = std::env::current_exe().ok();
        let on_path = std::env::var_os("PATH").and_then(|path| {
            std::env::split_paths(&path)
                .map(|dir| dir.join(EXE))
                .find(|file| file.is_file())
        });
        command_for(&relay_for(exe.as_deref(), on_path.as_deref()))
    })
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

/// How an agent starts the relay: `tabletist mcp` when `on_path`, the
/// program a shell finds by that name, is `exe`, this program; else
/// `exe`'s whole path, as a shell reads it.
fn relay_for(exe: Option<&std::path::Path>, on_path: Option<&std::path::Path>) -> String {
    let same = |a: &std::path::Path, b: &std::path::Path| match (
        std::fs::canonicalize(a),
        std::fs::canonicalize(b),
    ) {
        (Ok(a), Ok(b)) => a == b,
        _ => a == b,
    };
    match exe {
        Some(exe) if !on_path.is_some_and(|found| same(exe, found)) => {
            format!("{} mcp", shell_word(&exe.to_string_lossy()))
        }
        _ => "tabletist mcp".to_owned(),
    }
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
            key: "k".into(),
            version: "1.0.0".into(),
            cwd: Some("/home/ada".into()),
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
        // A newer relay may say more: what this one does not know is
        // passed over.
        let newer: Hello =
            serde_json::from_str(r#"{"key":"k","version":"9","cwd":null,"shell":"zsh"}"#).unwrap();
        assert_eq!(newer.cwd, None);
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
