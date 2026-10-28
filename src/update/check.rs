//! The check: one GET of the latest release, which says nothing of the
//! person but this version, the system and the processor.

use std::io::Read;
use std::time::Duration;

use super::release::Latest;

/// Where the check asks.
pub const LATEST: &str = "https://api.github.com/repos/igor-alexandrov/tabletist/releases/latest";

/// What the check card says it is asking, while it asks.
pub const ASKED: &str = "github.com/igor-alexandrov/tabletist/releases";

/// How long a check waits for its answer, all told.
pub const TIMEOUT: Duration = Duration::from_secs(10);

/// The largest answer read: a release's notes and its files' names.
const LIMIT: u64 = 1024 * 1024;

/// Where a redirect may lead: the API itself, as for a repository that
/// moved. Any other is not followed.
const API: &str = "https://api.github.com/";

/// How many of those redirects are followed.
const REDIRECTS: usize = 3;

/// Why a check found nothing.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Failed {
    /// No answer within this long.
    TimedOut(Duration),
    /// No connection to GitHub: offline, or its name did not resolve.
    Unreachable,
    /// GitHub answered with this status.
    Status(u16),
    /// An answer that was no release.
    Unreadable,
}

impl Failed {
    /// What the card says of it, under its title. English: the caller
    /// translates, with `{secs}` and `{status}` put in after.
    pub fn said(&self) -> &'static str {
        match self {
            Self::TimedOut(_) => {
                "Couldn't reach github.com: the connection timed out after {secs} s."
            }
            Self::Unreachable => "Couldn't reach github.com. Check the connection to the internet.",
            Self::Status(403 | 429) => {
                "GitHub is limiting checks from this network. Try again later."
            }
            Self::Status(_) => "GitHub answered with an error ({status}).",
            Self::Unreadable => "GitHub's answer wasn't a release.",
        }
    }

    /// [`Failed::said`], with its numbers in.
    pub fn with_numbers(&self, said: &str) -> String {
        let secs = match self {
            Self::TimedOut(after) => after.as_secs(),
            Self::Unreachable | Self::Status(_) | Self::Unreadable => 0,
        };
        let status = match self {
            Self::Status(status) => *status,
            Self::TimedOut(_) | Self::Unreachable | Self::Unreadable => 0,
        };
        said.replace("{secs}", &secs.to_string())
            .replace("{status}", &status.to_string())
    }
}

/// Asks `url` for the latest release, sending `user_agent` and nothing
/// else of the person: no cookie, no id. Blocks for up to `timeout`.
pub fn latest(url: &str, user_agent: &str, timeout: Duration) -> Result<Latest, Failed> {
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .http_status_as_error(false)
        // Followed here, and only to the API's own host.
        .max_redirects(0)
        .timeout_global(Some(timeout))
        .build()
        .into();
    let mut url = url.to_owned();
    let mut redirects = 0;
    let response = loop {
        let response = agent
            .get(&url)
            .header("Accept", "application/vnd.github+json")
            .header("User-Agent", user_agent)
            .call()
            .map_err(|error| match error {
                ureq::Error::Timeout(_) => Failed::TimedOut(timeout),
                _ => Failed::Unreachable,
            })?;
        let status = response.status().as_u16();
        let location = response
            .headers()
            .get("location")
            .and_then(|value| value.to_str().ok())
            .filter(|location| location.starts_with(API));
        match (status, location) {
            (200, _) => break response,
            (301 | 302 | 307 | 308, Some(location)) if redirects < REDIRECTS => {
                url = location.to_owned();
                redirects += 1;
            }
            _ => return Err(Failed::Status(status)),
        }
    };
    let mut body = Vec::new();
    response
        .into_body()
        .into_reader()
        .take(LIMIT)
        .read_to_end(&mut body)
        .map_err(|error| {
            // ureq says a timeout in the body as its own error, inside.
            let inner = error
                .get_ref()
                .and_then(|inner| inner.downcast_ref::<ureq::Error>());
            match (error.kind(), inner) {
                (std::io::ErrorKind::TimedOut, _) | (_, Some(ureq::Error::Timeout(_))) => {
                    Failed::TimedOut(timeout)
                }
                _ => Failed::Unreachable,
            }
        })?;
    let latest: Latest = serde_json::from_slice(&body).map_err(|_| Failed::Unreadable)?;
    if latest.tag_name.is_empty() {
        return Err(Failed::Unreadable);
    }
    Ok(latest)
}

/// What the check says of this copy: `Tabletist/0.3.0 (macOS 15.1;
/// arm64)`, `Tabletist/0.3.0 (Linux; x86_64)`. `macos` is the system's
/// version on a Mac.
pub fn user_agent(version: &str, macos: Option<&str>) -> String {
    let chip = match std::env::consts::ARCH {
        // As Apple names the chip.
        "aarch64" if cfg!(target_os = "macos") => "arm64",
        arch => arch,
    };
    let system = if cfg!(target_os = "macos") {
        match macos {
            Some(version) => format!("macOS {version}"),
            None => "macOS".to_owned(),
        }
    } else if cfg!(windows) {
        "Windows".to_owned()
    } else if cfg!(target_os = "linux") {
        "Linux".to_owned()
    } else {
        std::env::consts::OS.to_owned()
    };
    format!("Tabletist/{version} ({system}; {chip})")
}

/// The Mac's system version (`15.1`), asked of `sw_vers` once. None
/// elsewhere, or where it says nothing.
pub fn macos_version() -> Option<String> {
    if !cfg!(target_os = "macos") {
        return None;
    }
    static VERSION: std::sync::OnceLock<Option<String>> = std::sync::OnceLock::new();
    VERSION
        .get_or_init(|| {
            let output = std::process::Command::new("/usr/bin/sw_vers")
                .arg("-productVersion")
                .output()
                .ok()?;
            let version = String::from_utf8(output.stdout).ok()?.trim().to_owned();
            (!version.is_empty()).then_some(version)
        })
        .clone()
}

#[cfg(test)]
pub(crate) mod tests {
    use std::io::{BufRead, BufReader, Write};
    use std::net::TcpListener;
    use std::thread::JoinHandle;

    use super::*;

    /// The latest release as the fake server gives it: Bookshop's notes
    /// are the app's own.
    pub(crate) const RELEASE: &str = include_str!("../../tests/fixtures/update/latest.json");

    /// Answers one request on a local port with `answer`, after `wait`,
    /// and gives back the request's lines, the request line first.
    pub(crate) fn serve_once(answer: String, wait: Duration) -> (String, JoinHandle<Vec<String>>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!(
            "http://{}/repos/igor-alexandrov/tabletist/releases/latest",
            listener.local_addr().unwrap()
        );
        let server = std::thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            let mut reader = BufReader::new(stream.try_clone().unwrap());
            let mut head = Vec::new();
            loop {
                let mut line = String::new();
                reader.read_line(&mut line).unwrap();
                if line == "\r\n" || line.is_empty() {
                    break;
                }
                head.push(line.trim_end().to_owned());
            }
            std::thread::sleep(wait);
            // The client may have given up by now.
            let _ = (&stream).write_all(answer.as_bytes());
            head
        });
        (url, server)
    }

    pub(crate) fn ok(body: &str) -> String {
        format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        )
    }

    #[test]
    fn the_check_reads_the_latest_release() {
        let (url, server) = serve_once(ok(RELEASE), Duration::ZERO);
        let latest = latest(&url, "Tabletist/0.3.0 (Linux; x86_64)", TIMEOUT).unwrap();
        server.join().unwrap();
        assert_eq!(latest.tag_name, "v0.4.0");
        assert!(!latest.assets.is_empty());
    }

    /// Only the user agent says anything: no cookie, no id, no other
    /// header of the client's.
    #[test]
    fn the_check_sends_the_user_agent_and_nothing_else() {
        let (url, server) = serve_once(ok(RELEASE), Duration::ZERO);
        latest(&url, "Tabletist/0.3.0 (Linux; x86_64)", TIMEOUT).unwrap();
        let head = server.join().unwrap();
        assert_eq!(
            head[0],
            "GET /repos/igor-alexandrov/tabletist/releases/latest HTTP/1.1"
        );
        let mut names: Vec<String> = head[1..]
            .iter()
            .map(|line| line.split(':').next().unwrap().to_ascii_lowercase())
            .collect();
        names.sort();
        assert_eq!(names, ["accept", "host", "user-agent"], "{head:?}");
        let sent = |line: &str| head.iter().any(|sent| sent.eq_ignore_ascii_case(line));
        assert!(
            sent("user-agent: Tabletist/0.3.0 (Linux; x86_64)"),
            "{head:?}"
        );
        assert!(sent("accept: application/vnd.github+json"), "{head:?}");
    }

    #[test]
    fn no_answer_in_time_is_a_timeout() {
        let (url, server) = serve_once(ok(RELEASE), Duration::from_millis(1_500));
        let wait = Duration::from_millis(200);
        assert_eq!(
            latest(&url, "Tabletist/0.3.0", wait),
            Err(Failed::TimedOut(wait))
        );
        server.join().unwrap();
    }

    #[test]
    fn no_server_is_unreachable() {
        // A port nothing listens on.
        let port = TcpListener::bind("127.0.0.1:0")
            .unwrap()
            .local_addr()
            .unwrap()
            .port();
        let url = format!("http://127.0.0.1:{port}/releases/latest");
        assert_eq!(
            latest(&url, "Tabletist/0.3.0", TIMEOUT),
            Err(Failed::Unreachable)
        );
    }

    #[test]
    fn an_error_status_or_no_release_fails_the_check() {
        let (url, server) = serve_once(
            "HTTP/1.1 404 Not Found\r\nContent-Length: 9\r\nConnection: close\r\n\r\nnot found"
                .into(),
            Duration::ZERO,
        );
        assert_eq!(
            latest(&url, "Tabletist/0.3.0", TIMEOUT),
            Err(Failed::Status(404))
        );
        server.join().unwrap();
        let (url, server) = serve_once(ok("{\"message\":\"hi\"}"), Duration::ZERO);
        assert_eq!(
            latest(&url, "Tabletist/0.3.0", TIMEOUT),
            Err(Failed::Unreadable)
        );
        server.join().unwrap();
    }

    #[test]
    fn the_user_agent_names_the_version_the_system_and_the_chip() {
        let agent = user_agent("0.3.0", Some("15.1"));
        if cfg!(target_os = "linux") {
            assert_eq!(
                agent,
                format!("Tabletist/0.3.0 (Linux; {})", std::env::consts::ARCH)
            );
        }
        if cfg!(target_os = "macos") {
            assert!(
                agent.starts_with("Tabletist/0.3.0 (macOS 15.1; "),
                "{agent}"
            );
        }
    }

    #[test]
    fn what_a_failure_says_has_its_numbers() {
        let failed = Failed::TimedOut(TIMEOUT);
        assert_eq!(
            failed.with_numbers(failed.said()),
            "Couldn't reach github.com: the connection timed out after 10 s."
        );
        let failed = Failed::Status(502);
        assert_eq!(
            failed.with_numbers(failed.said()),
            "GitHub answered with an error (502)."
        );
    }

    /// A redirect to a host that is not the API's is not followed: the
    /// request and its User-Agent go to GitHub's API only.
    #[test]
    fn a_redirect_to_another_host_is_not_followed() {
        let (elsewhere, _other) = serve_once(ok(RELEASE), Duration::ZERO);
        // The same machine by another name: another host.
        let elsewhere = elsewhere.replace("127.0.0.1", "localhost");
        let (url, server) = serve_once(
            format!(
                "HTTP/1.1 302 Found\r\nLocation: {elsewhere}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
            ),
            Duration::ZERO,
        );
        let result = latest(&url, "Tabletist/0.3.0 (Linux; x86_64)", TIMEOUT);
        server.join().unwrap();
        assert!(
            result.is_err(),
            "followed the redirect to {elsewhere}: {result:?}"
        );
    }

    /// Headers in time, then the body stalls: still a timeout, said as one.
    #[test]
    fn a_body_that_stalls_is_a_timeout() {
        use std::io::Write as _;
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}/releases/latest", listener.local_addr().unwrap());
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut buf = [0u8; 4096];
            let _ = std::io::Read::read(&mut stream, &mut buf);
            let _ = stream.write_all(
                b"HTTP/1.1 200 OK\r\nContent-Length: 100000\r\nConnection: close\r\n\r\n{\"tag",
            );
            std::thread::sleep(Duration::from_millis(1_500));
        });
        let wait = Duration::from_millis(300);
        let started = std::time::Instant::now();
        let result = latest(&url, "Tabletist/0.3.0", wait);
        assert!(
            started.elapsed() < Duration::from_secs(1),
            "{:?}",
            started.elapsed()
        );
        assert_eq!(result, Err(Failed::TimedOut(wait)));
        server.join().unwrap();
    }
}
