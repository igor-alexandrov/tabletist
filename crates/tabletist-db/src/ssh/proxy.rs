//! Reaching the SSH server the way `~/.ssh/config` says, through a
//! ProxyCommand or a ProxyJump, as OpenSSH does. A ProxyCommand runs in the
//! shell, and the SSH session goes over its standard input and output. A
//! ProxyJump is the command OpenSSH makes of one, `ssh -W [host]:port jump`,
//! so the jump hosts are logged in to as `ssh` in a terminal logs in to
//! them: with its config, its agent and its known_hosts. The server at the
//! end is still checked against Tabletist's own trusted keys.

use std::pin::Pin;
use std::process::Stdio;
use std::task::{Context, Poll};
use std::time::Duration;

use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, ReadBuf};
use tokio::process::{Child, ChildStderr, ChildStdin, ChildStdout, Command};
use tokio::task::JoinHandle;

use crate::ssh_config::Proxy;

/// How the tunnel reaches the SSH server when it does not connect to it
/// directly.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Via {
    /// A ProxyCommand, its tokens filled in, for the shell.
    Command(String),
    /// A ProxyJump as written (`ops@gateway:2222,bastion`), and the
    /// arguments of the `ssh` that goes through it.
    Jump { hops: String, args: Vec<String> },
}

/// What a ProxyCommand's tokens stand for.
pub(crate) struct Tokens<'a> {
    /// `%h`: the host name the tunnel connects to.
    pub host: &'a str,
    /// `%p`.
    pub port: u16,
    /// `%r`: the user the tunnel logs in as.
    pub user: &'a str,
    /// `%n`: the host as it was typed.
    pub alias: &'a str,
}

impl Via {
    /// The way `proxy` names to `tokens`'s server, `None` for none.
    pub fn new(proxy: Option<&Proxy>, tokens: &Tokens<'_>) -> Result<Option<Self>, String> {
        match proxy {
            None | Some(Proxy::Off) => Ok(None),
            Some(Proxy::Command(command)) => expand(command, tokens).map(Self::Command).map(Some),
            Some(Proxy::Jump(hops)) => jump(hops, tokens.host, tokens.port).map(Some),
        }
    }

    /// How a message names it.
    pub fn describe(&self) -> String {
        match self {
            Self::Command(_) => "its ProxyCommand".to_owned(),
            Self::Jump { hops, .. } => format!("ProxyJump {hops}"),
        }
    }

    fn command(&self) -> Command {
        let mut command = match self {
            Self::Command(line) => shell(line),
            Self::Jump { args, .. } => {
                let mut command = Command::new("ssh");
                command.args(args);
                command
            }
        };
        command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);
        #[cfg(windows)]
        {
            // CREATE_NO_WINDOW: a window app's child would open a console.
            command.creation_flags(0x0800_0000);
        }
        command
    }

    /// Starts the command: the stream the SSH session goes over, what the
    /// command says on its standard error, and the command itself, which
    /// stops when it is dropped. The tunnel keeps it, not the stream: russh
    /// keeps a stream it gave up on in a task of its own.
    pub fn spawn(&self) -> Result<(ProxyStream, Said, Running), String> {
        let mut child = self
            .command()
            .spawn()
            .map_err(|error| format!("could not start {}: {error}", self.describe()))?;
        let (Some(stdin), Some(stdout), Some(stderr)) =
            (child.stdin.take(), child.stdout.take(), child.stderr.take())
        else {
            return Err(format!("could not start {}", self.describe()));
        };
        let stream = ProxyStream {
            stdin: Some(stdin),
            stdout,
        };
        Ok((stream, Said::collect(stderr), Running(child)))
    }
}

/// OpenSSH runs a ProxyCommand with `exec` in the person's shell, so the
/// shell does not stay between the session and the command.
#[cfg(unix)]
fn shell(line: &str) -> Command {
    let shell = std::env::var_os("SHELL")
        .filter(|shell| !shell.is_empty())
        .unwrap_or_else(|| "/bin/sh".into());
    let mut command = Command::new(shell);
    command.arg("-c").arg(format!("exec {line}"));
    command
}

/// Win32-OpenSSH starts a ProxyCommand's program itself, with the rest of
/// the line as its command line: `cmd` would expand `%` and `!` in it, and
/// stopping `cmd` would leave the program running.
#[cfg(windows)]
fn shell(line: &str) -> Command {
    let (program, rest) = program(line);
    let mut command = Command::new(program);
    if !rest.is_empty() {
        command.raw_arg(rest);
    }
    command
}

/// A command line's program, quoted or not, and what follows it.
#[cfg_attr(not(windows), allow(dead_code))]
fn program(line: &str) -> (&str, &str) {
    let line = line.trim();
    let (program, rest) = match line.strip_prefix('"') {
        Some(quoted) => quoted.split_once('"').unwrap_or((quoted, "")),
        None => line.split_once(char::is_whitespace).unwrap_or((line, "")),
    };
    (program, rest.trim_start())
}

/// Whether a host or user name can go into a command line: none with the
/// shell's special characters, spaces or control characters, or a leading
/// `-`, as OpenSSH checks since it was shown a host name that ran a command
/// through `%h` (CVE-2023-51385). `%`, `!` and `^` are `cmd`'s, for a
/// command that starts `cmd` itself on Windows.
fn plain(value: &str) -> bool {
    !value.starts_with('-')
        && !value
            .chars()
            .any(|c| "'`\"$\\;&<>|(){}%!^".contains(c) || c.is_whitespace() || c.is_control())
}

fn checked<'a>(what: &str, value: &'a str) -> Result<&'a str, String> {
    if plain(value) {
        Ok(value)
    } else {
        Err(format!(
            "the SSH {what} {value:?} has characters a proxy's command line cannot take"
        ))
    }
}

/// `command` with `%h`, `%p`, `%r`, `%n` and `%%` filled in, the tokens
/// OpenSSH takes in a ProxyCommand.
pub(crate) fn expand(command: &str, tokens: &Tokens<'_>) -> Result<String, String> {
    let mut out = String::with_capacity(command.len());
    let mut chars = command.chars();
    while let Some(c) = chars.next() {
        if c != '%' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('%') => out.push('%'),
            Some('h') => out.push_str(checked("host", tokens.host)?),
            Some('p') => out.push_str(&tokens.port.to_string()),
            Some('r') => out.push_str(checked("user", tokens.user)?),
            Some('n') => out.push_str(checked("host", tokens.alias)?),
            Some(other) => {
                return Err(format!(
                    "the ProxyCommand uses %{other}, which it cannot take (only %h, %p, %r, %n \
                     and %%)"
                ));
            }
            None => return Err("the ProxyCommand ends in a lone %".to_owned()),
        }
    }
    Ok(out)
}

/// The arguments of `ssh -W [host]:port` through `hops`: the last hop is
/// the one `ssh` connects to, and those before it are its own ProxyJump,
/// as OpenSSH does it. `BatchMode` makes a jump host that would ask for a
/// password, or for trust in its key, fail with its reason rather than
/// wait for an answer nobody can type.
pub(crate) fn jump(hops: &str, host: &str, port: u16) -> Result<Via, String> {
    let refused = || format!("the ProxyJump {hops:?} is not a list of hosts ssh can take");
    let hops = hops.trim();
    let (before, last) = match hops.rsplit_once(',') {
        Some((before, last)) => (Some(before), last),
        None => (None, hops),
    };
    if !plain(hops) || last.is_empty() || before.is_some_and(str::is_empty) {
        return Err(refused());
    }
    let (user, name, hop_port) = hop(last).ok_or_else(refused)?;
    let mut args = vec!["-o".to_owned(), "BatchMode=yes".to_owned()];
    if let Some(before) = before {
        args.extend(["-J".to_owned(), before.to_owned()]);
    }
    if let Some(user) = user {
        args.extend(["-l".to_owned(), user.to_owned()]);
    }
    if let Some(port) = hop_port {
        args.extend(["-p".to_owned(), port.to_string()]);
    }
    args.extend([
        "-W".to_owned(),
        format!("[{}]:{port}", checked("host", host)?),
        name.to_owned(),
    ]);
    Ok(Via::Jump {
        hops: hops.to_owned(),
        args,
    })
}

/// A ProxyJump hop, `[user@]host[:port]` or `ssh://[user@]host[:port]`,
/// with an IPv6 address in brackets.
fn hop(text: &str) -> Option<(Option<&str>, &str, Option<u16>)> {
    let text = text
        .strip_prefix("ssh://")
        .map_or(text, |uri| uri.trim_end_matches('/'));
    let (user, rest) = match text.rsplit_once('@') {
        Some((user, rest)) => (Some(Some(user).filter(|user| !user.is_empty())?), rest),
        None => (None, text),
    };
    let (name, port) = if let Some(inner) = rest.strip_prefix('[') {
        let (name, after) = inner.split_once(']')?;
        match after {
            "" => (name, None),
            _ => (name, Some(after.strip_prefix(':')?)),
        }
    } else {
        match rest.split_once(':') {
            // More than one colon: an IPv6 address without a port.
            Some((name, port)) if !port.contains(':') => (name, Some(port)),
            _ => (rest, None),
        }
    };
    let port = match port {
        Some(port) => Some(port.parse().ok().filter(|port| *port != 0)?),
        None => None,
    };
    (!name.is_empty()).then_some((user, name, port))
}

/// A started proxy command, stopped when this is dropped.
pub(crate) struct Running(#[allow(dead_code, reason = "kept to be dropped")] Child);

/// The proxy's standard input and output, as one stream for the SSH
/// session.
pub(crate) struct ProxyStream {
    /// `None` once shut down: only dropping it closes the pipe, which is
    /// how the command learns there is nothing more.
    stdin: Option<ChildStdin>,
    stdout: ChildStdout,
}

fn closed() -> std::io::Error {
    std::io::Error::new(
        std::io::ErrorKind::BrokenPipe,
        "the proxy's input is closed",
    )
}

impl AsyncRead for ProxyStream {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        Pin::new(&mut self.get_mut().stdout).poll_read(cx, buf)
    }
}

impl AsyncWrite for ProxyStream {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        match &mut self.get_mut().stdin {
            Some(stdin) => Pin::new(stdin).poll_write(cx, buf),
            None => Poll::Ready(Err(closed())),
        }
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        match &mut self.get_mut().stdin {
            Some(stdin) => Pin::new(stdin).poll_flush(cx),
            None => Poll::Ready(Ok(())),
        }
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        let this = self.get_mut();
        if let Some(stdin) = &mut this.stdin {
            std::task::ready!(Pin::new(stdin).poll_flush(cx))?;
        }
        this.stdin = None;
        Poll::Ready(Ok(()))
    }
}

/// The last lines the proxy wrote to its standard error, which say why it
/// failed (`Permission denied`, `Host key verification failed`). It is
/// read to the end either way, so the command never waits on a full pipe,
/// and as bytes, so text in another encoding or one endless line can
/// neither stop the reading nor fill the memory: only the last
/// `SAID_BYTES` are kept.
pub(crate) struct Said(JoinHandle<String>);

/// How many bytes of the end are kept, how many of their last lines are
/// shown, and how long a line may be.
const SAID_BYTES: usize = 4096;
const SAID_LINES: usize = 3;
const SAID_WIDTH: usize = 200;

impl Said {
    fn collect(mut stderr: ChildStderr) -> Self {
        Self(tokio::spawn(async move {
            let mut tail: Vec<u8> = Vec::with_capacity(SAID_BYTES);
            let mut chunk = [0; 1024];
            while let Ok(read) = stderr.read(&mut chunk).await {
                if read == 0 {
                    break;
                }
                tail.extend_from_slice(&chunk[..read]);
                if tail.len() > SAID_BYTES {
                    tail.drain(..tail.len() - SAID_BYTES);
                }
            }
            last_lines(&String::from_utf8_lossy(&tail))
        }))
    }

    /// What the proxy said, once the failed session stopped it; empty if
    /// it said nothing within a second.
    pub async fn words(self) -> String {
        match tokio::time::timeout(Duration::from_secs(1), self.0).await {
            Ok(Ok(words)) => words,
            Ok(Err(_)) | Err(_) => String::new(),
        }
    }
}

/// The last non-empty lines of `text`, each cut to `SAID_WIDTH`.
fn last_lines(text: &str) -> String {
    let lines: Vec<String> = text
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(|line| line.chars().take(SAID_WIDTH).collect())
        .collect();
    lines[lines.len().saturating_sub(SAID_LINES)..].join("; ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    fn tokens() -> Tokens<'static> {
        Tokens {
            host: "10.0.0.5",
            port: 2222,
            user: "ops",
            alias: "bastion",
        }
    }

    fn args(via: Via) -> Vec<String> {
        match via {
            Via::Jump { args, .. } => args,
            Via::Command(command) => panic!("{command}"),
        }
    }

    #[test]
    fn a_proxy_command_takes_openssh_tokens() {
        assert_eq!(
            expand("ssh -W %h:%p -l %r gateway # %n 100%%", &tokens()).unwrap(),
            "ssh -W 10.0.0.5:2222 -l ops gateway # bastion 100%"
        );
        for (command, says) in [("nc %d", "%d"), ("nc %h %", "lone %")] {
            let error = expand(command, &tokens()).unwrap_err();
            assert!(error.contains(says), "{error}");
        }
    }

    #[test]
    fn a_name_that_would_run_in_the_shell_is_refused() {
        for alias in [
            "a;touch x",
            "$(id)",
            "`id`",
            "-oProxyCommand=x",
            "a b",
            "a\nb",
            "%TEMP%",
            "!VAR!",
            "a^&b",
        ] {
            let tokens = Tokens { alias, ..tokens() };
            let error = expand("nc %n 22", &tokens).unwrap_err();
            assert!(error.contains("characters"), "{alias}: {error}");
            // A command that does not name the alias does not mind it.
            assert!(expand("nc %h %p", &tokens).is_ok());
        }
        assert!(jump("gateway", "a;b", 22).is_err());
    }

    #[test]
    fn no_proxy_and_proxy_none_connect_directly() {
        assert_eq!(Via::new(None, &tokens()), Ok(None));
        assert_eq!(Via::new(Some(&Proxy::Off), &tokens()), Ok(None));
        assert_eq!(
            Via::new(Some(&Proxy::Command("nc %h %p".into())), &tokens()),
            Ok(Some(Via::Command("nc 10.0.0.5 2222".into())))
        );
    }

    #[test]
    fn a_proxy_jump_is_ssh_w_through_its_last_hop() {
        assert_eq!(
            args(jump("gateway", "10.0.0.5", 22).unwrap()),
            ["-o", "BatchMode=yes", "-W", "[10.0.0.5]:22", "gateway"]
        );
        assert_eq!(
            args(jump("edge,ops@gw.example.com:2200", "db", 2222).unwrap()),
            [
                "-o",
                "BatchMode=yes",
                "-J",
                "edge",
                "-l",
                "ops",
                "-p",
                "2200",
                "-W",
                "[db]:2222",
                "gw.example.com"
            ]
        );
        assert_eq!(
            args(jump("ssh://me@[fd00::1]:2022/", "::1", 22).unwrap()),
            [
                "-o",
                "BatchMode=yes",
                "-l",
                "me",
                "-p",
                "2022",
                "-W",
                "[::1]:22",
                "fd00::1"
            ]
        );
        assert_eq!(
            args(jump("fd00::1", "db", 22).unwrap()),
            ["-o", "BatchMode=yes", "-W", "[db]:22", "fd00::1"]
        );
    }

    #[test]
    fn a_proxy_jump_ssh_cannot_take_is_refused() {
        for hops in [
            "",
            "-oProxyCommand=x",
            "a,",
            ",a",
            "gw:port",
            "gw:0",
            "@gw",
            "[gw",
            "a b",
        ] {
            let error = jump(hops, "db", 22).unwrap_err();
            assert!(error.contains("ProxyJump"), "{hops}: {error}");
        }
    }

    #[test]
    fn describe_names_the_jump_hosts() {
        assert_eq!(
            jump("ops@gateway", "db", 22).unwrap().describe(),
            "ProxyJump ops@gateway"
        );
        assert_eq!(
            Via::Command("nc db 22".into()).describe(),
            "its ProxyCommand"
        );
    }

    /// The command's output is the stream's input and the other way round,
    /// and what it writes to standard error is kept for the message.
    #[cfg(unix)]
    #[tokio::test]
    async fn the_stream_is_the_commands_input_and_output() {
        let via = Via::Command("sh -c 'echo starting >&2; cat; echo done >&2'".into());
        let (mut stream, said, _running) = via.spawn().unwrap();
        stream.write_all(b"SSH-2.0-test\r\n").await.unwrap();
        let mut back = [0; 14];
        stream.read_exact(&mut back).await.unwrap();
        assert_eq!(&back, b"SSH-2.0-test\r\n");
        stream.shutdown().await.unwrap();
        let mut rest = Vec::new();
        stream.read_to_end(&mut rest).await.unwrap();
        assert!(rest.is_empty());
        assert_eq!(said.words().await, "starting; done");
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn dropping_the_running_command_stops_it() {
        let via = Via::Command("sleep 30".into());
        let (_stream, said, running) = via.spawn().unwrap();
        drop(running);
        // The command's stderr closes once it is killed, well before 30s.
        let started = std::time::Instant::now();
        assert_eq!(said.words().await, "");
        assert!(started.elapsed() < Duration::from_millis(900));
    }

    #[test]
    fn the_last_lines_are_kept_short() {
        let long = "x".repeat(SAID_WIDTH + 50);
        assert_eq!(
            last_lines(&format!("one\n\ntwo\r\nthree\n{long}\n")),
            format!("two; three; {}", "x".repeat(SAID_WIDTH))
        );
        assert_eq!(last_lines(""), "");
    }

    /// An endless line and bytes that are not UTF-8 neither stop the
    /// reading nor are kept whole.
    #[cfg(unix)]
    #[tokio::test]
    async fn stderr_is_read_to_the_end_whatever_it_holds() {
        let via = Via::Command(
            r#"sh -c 'head -c 100000 /dev/zero | tr "\0" x >&2; printf "\n\377\376 bad\nlast\n" >&2'"#
                .into(),
        );
        let (_stream, said, _running) = via.spawn().unwrap();
        let words = said.words().await;
        assert!(words.ends_with("\u{fffd}\u{fffd} bad; last"), "{words}");
        assert!(words.len() < SAID_BYTES, "{}", words.len());
    }

    #[test]
    fn a_windows_command_line_starts_with_its_program() {
        assert_eq!(
            program(r#""C:\Program Files\Proxy\nc.exe" "%h" 22"#),
            (r"C:\Program Files\Proxy\nc.exe", r#""%h" 22"#)
        );
        assert_eq!(program("  nc.exe  db 22"), ("nc.exe", "db 22"));
        assert_eq!(program("connect"), ("connect", ""));
    }
}
