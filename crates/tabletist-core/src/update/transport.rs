//! The updater's HTTP client: one GET at a time, as fastframe-update asks
//! of a transport. For a download it waits while the network is away,
//! from the first request on; its bodies go on where they stopped when the
//! connection drops or goes quiet, with `Range`; and everything fails at
//! once when the download is cancelled. fastframe-update has none of
//! that, and to it the reconnects are one unbroken stream, whose whole
//! hash it still checks.

use std::io::{self, Read};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, PoisonError, mpsc};
use std::time::{Duration, Instant};

use fastframe_update::{Request, Response, Transport};

/// What the body says when the connection did not come back in time: the
/// app says Couldn't download (`download::Failure`).
pub const LOST: &str = "The download's connection was lost and didn't come back";

/// What a request says that got no answer: no network, or no server.
pub const UNREACHABLE: &str = "The update server could not be reached";

/// What the body says once the download is cancelled.
pub const CANCELLED: &str = "The download was cancelled";

/// How often a dropped download connects again, and for how long, and
/// how long a connection may go quiet before it counts as dropped.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Patience {
    pub retry: Duration,
    pub give_up: Duration,
    pub quiet: Duration,
}

/// Every 2 seconds, for up to 10 minutes; a connection quiet for 8
/// seconds has lost its network, as when Wi-Fi goes without the socket
/// closing.
pub const PATIENCE: Patience = Patience {
    retry: Duration::from_secs(2),
    give_up: Duration::from_secs(10 * 60),
    quiet: Duration::from_secs(8),
};

/// How often a body waiting for its connection looks at its cancel.
const TICK: Duration = Duration::from_millis(50);

/// How long a download's request may go unanswered before the download
/// says it waits: a quick reconnect says nothing.
const SLOW: Duration = Duration::from_secs(1);

/// What a download shares with whoever started it: its cancel, and
/// whether it waits for the network, which it tells as it changes.
pub struct Download {
    cancelled: AtomicBool,
    waiting: AtomicBool,
    told: Box<dyn Fn(bool) + Send + Sync>,
    patience: Patience,
}

impl std::fmt::Debug for Download {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Download")
            .field("cancelled", &self.cancelled())
            .field("waiting", &self.waiting())
            .field("patience", &self.patience)
            .finish_non_exhaustive()
    }
}

impl Download {
    /// A download that calls `told` each time it starts or stops waiting.
    pub fn new(told: impl Fn(bool) + Send + Sync + 'static) -> Self {
        Self {
            cancelled: AtomicBool::new(false),
            waiting: AtomicBool::new(false),
            told: Box::new(told),
            patience: PATIENCE,
        }
    }

    /// The same, connecting again by `patience`: a test's is short.
    pub fn with_patience(mut self, patience: Patience) -> Self {
        self.patience = patience;
        self
    }

    /// Stops it: the body's next read fails, and fastframe-update deletes
    /// what it wrote.
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
    }

    pub fn cancelled(&self) -> bool {
        self.cancelled.load(Ordering::SeqCst)
    }

    /// Whether it waits for the network to come back.
    pub fn waiting(&self) -> bool {
        self.waiting.load(Ordering::SeqCst)
    }

    fn wait(&self, waiting: bool) {
        if self.waiting.swap(waiting, Ordering::SeqCst) != waiting {
            (self.told)(waiting);
        }
    }

    fn stop_if_cancelled(&self) -> io::Result<()> {
        match self.cancelled() {
            true => Err(io::Error::other(CANCELLED)),
            false => Ok(()),
        }
    }

    /// Sleeps `time`, a slice at a time, and stops early once cancelled.
    fn pause(&self, time: Duration) -> io::Result<()> {
        let until = Instant::now() + time;
        loop {
            self.stop_if_cancelled()?;
            let left = until.saturating_duration_since(Instant::now());
            if left.is_zero() {
                return Ok(());
            }
            std::thread::sleep(left.min(Duration::from_millis(50)));
        }
    }

    /// Sends `get` on a thread of its own and waits for its answer a slice
    /// at a time, so that a cancel is seen while it connects, looks up the
    /// host or waits for the head; one unanswered for `SLOW` says it
    /// waits. A cancel stops it before it is sent, too. The thread of a
    /// request given up on ends at ureq's own timeout.
    fn send(&self, get: Get) -> io::Result<Answered> {
        self.stop_if_cancelled()?;
        let (send, answer) = mpsc::sync_channel(1);
        std::thread::Builder::new()
            .name("tabletist-update-request".into())
            .spawn(move || {
                let _ = send.send(get.call());
            })?;
        let since = Instant::now();
        loop {
            self.stop_if_cancelled()?;
            match answer.recv_timeout(TICK) {
                Ok(answered) => return Ok(answered),
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    if since.elapsed() >= SLOW {
                        self.wait(true);
                    }
                }
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    return Err(io::Error::other("The update request's thread ended"));
                }
            }
        }
    }
}

/// The client itself: the agent, and what it says of this copy.
struct Client {
    agent: ureq::Agent,
    user_agent: String,
}

/// What a request got: an answer, or none.
type Answered = Result<ureq::http::Response<ureq::Body>, ureq::Error>;

/// One GET as the updater sends it, its own, so that it may go to a
/// thread: from byte `range` on, where it is given.
#[derive(Clone)]
struct Get {
    client: Arc<Client>,
    url: String,
    accept: String,
    range: Option<u64>,
}

impl Get {
    fn call(&self) -> Answered {
        let mut request = self
            .client
            .agent
            .get(&self.url)
            .header("Accept", &self.accept)
            .header("User-Agent", &self.client.user_agent);
        if let Some(from) = self.range {
            request = request.header("Range", &format!("bytes={from}-"));
        }
        request.call()
    }
}

/// A redirect the updater was given: where its chain of redirects began,
/// and the hosts on the way, which the updater allowed. The updater asks
/// for where it leads next, in its own form of the address.
#[derive(Clone, Debug)]
struct Chain {
    start: String,
    hosts: Vec<String>,
}

/// The updater's HTTP client: redirects handed back rather than followed
/// (the updater follows them, to GitHub's hosts only), any status returned
/// as an answer, 15 seconds to connect and 15 minutes for a whole answer,
/// as fastframe-update asks of a transport. It says what the check says of
/// this copy, and nothing else.
pub struct UreqTransport {
    client: Arc<Client>,
    /// The download it is for, where it is for one: its bodies resume.
    download: Option<Arc<Download>>,
    /// The last redirect handed back.
    chain: Mutex<Option<Chain>>,
}

impl UreqTransport {
    /// For the updater's calls that download nothing: a body that drops
    /// is an error, as for any client.
    pub fn new() -> Self {
        let user_agent =
            super::check::user_agent(super::CURRENT, super::check::macos_version().as_deref());
        Self::with_user_agent(user_agent, None)
    }

    /// For `download`: its bodies go on where they stopped.
    pub fn downloading(download: Arc<Download>) -> Self {
        let user_agent =
            super::check::user_agent(super::CURRENT, super::check::macos_version().as_deref());
        Self::with_user_agent(user_agent, Some(download))
    }

    fn with_user_agent(user_agent: String, download: Option<Arc<Download>>) -> Self {
        let config = ureq::Agent::config_builder()
            .max_redirects(0)
            .http_status_as_error(false)
            .timeout_connect(Some(Duration::from_secs(15)))
            .timeout_recv_response(Some(Duration::from_secs(30)))
            .timeout_global(Some(Duration::from_secs(15 * 60)))
            .build();
        Self {
            client: Arc::new(Client {
                agent: config.into(),
                user_agent,
            }),
            download,
            chain: Mutex::new(None),
        }
    }
}

impl Default for UreqTransport {
    fn default() -> Self {
        Self::new()
    }
}

impl UreqTransport {
    /// Sends `request`. For a download, no answer is waited out: asked
    /// again every `retry`, saying it waits, until `give_up` has passed;
    /// and a cancel stops each attempt, a redirect's hop too.
    fn call(&self, request: &Request<'_>) -> anyhow::Result<ureq::http::Response<ureq::Body>> {
        let get = Get {
            client: Arc::clone(&self.client),
            url: request.url.to_owned(),
            accept: request.accept.to_owned(),
            range: None,
        };
        let Some(download) = &self.download else {
            return get
                .call()
                .map_err(|error| anyhow::Error::new(error).context(UNREACHABLE));
        };
        let since = Instant::now();
        loop {
            match download.send(get.clone())? {
                Ok(response) => {
                    download.wait(false);
                    return Ok(response);
                }
                Err(error) if since.elapsed() >= download.patience.give_up => {
                    download.wait(false);
                    return Err(anyhow::Error::new(error).context(UNREACHABLE));
                }
                Err(error) => {
                    log::info!("the update server did not answer: {error}");
                    download.wait(true);
                    download.pause(download.patience.retry)?;
                }
            }
        }
    }
}

impl Transport for UreqTransport {
    fn get(&self, request: &Request<'_>) -> anyhow::Result<Response> {
        let response = self.call(request)?;
        let status = response.status().as_u16();
        let location = header(&response, "location");
        // Where this request's chain began: a reconnect starts there, for
        // a redirect's address may be signed for a few minutes only. The
        // updater follows a redirect with its very next request.
        let mut chain = self.chain.lock().unwrap_or_else(PoisonError::into_inner);
        let (start, mut hosts) = match chain.take() {
            Some(chain) => (chain.start, chain.hosts),
            None => (request.url.to_owned(), Vec::new()),
        };
        hosts.extend(host(request.url));
        if (300..400).contains(&status)
            && let Some(location) = &location
        {
            *chain = Some(Chain { start, hosts });
            drop(chain);
            return Ok(Response {
                status,
                location: Some(location.clone()),
                body: Box::new(io::empty()),
            });
        }
        drop(chain);
        let download = match (&self.download, status) {
            (Some(download), 200..=299) => Arc::clone(download),
            _ => {
                return Ok(Response {
                    status,
                    location,
                    body: Box::new(response.into_body().into_reader()),
                });
            }
        };
        let length = response.body().content_length();
        let body = Body {
            pieces: Some(pump(Box::new(response.into_body().into_reader()))?),
            held: Vec::new(),
            skip: 0,
            read: 0,
            length,
            heard: Instant::now(),
            lost: None,
            tries: 0,
            again: Again {
                client: Arc::clone(&self.client),
                url: start,
                accept: request.accept.to_owned(),
                hosts,
            },
            download,
        };
        Ok(Response {
            status,
            location: None,
            body: Box::new(body),
        })
    }
}

/// A header's value, where it is text.
fn header<B>(response: &ureq::http::Response<B>, name: &str) -> Option<String> {
    response
        .headers()
        .get(name)
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned)
}

/// The host of `url`, with its port: `github.com`, `127.0.0.1:8123`.
fn host(url: &str) -> Option<String> {
    let (_, rest) = url.split_once("://")?;
    let end = rest.find(['/', '?', '#']).unwrap_or(rest.len());
    Some(rest[..end].to_ascii_lowercase())
}

/// Whether a reconnect may follow a redirect to `url`: to a host the
/// updater's own redirects went through, over HTTPS (plain HTTP on this
/// computer only, as for a local feed), with no user in it.
fn may_follow(url: &str, hosts: &[String]) -> bool {
    let Some(host) = host(url) else {
        return false;
    };
    let secure = match url.split_once("://").map(|(scheme, _)| scheme) {
        Some(scheme) if scheme.eq_ignore_ascii_case("https") => true,
        Some(scheme) if scheme.eq_ignore_ascii_case("http") => {
            host.starts_with("127.0.0.1") || host.starts_with("[::1]")
        }
        _ => false,
    };
    secure && !host.contains('@') && hosts.contains(&host)
}

/// `location` as an address of its own, where it is one relative to
/// `base`: GitHub's are absolute, a server's may not be.
fn absolute(base: &str, location: &str) -> String {
    if location.contains("://") {
        return location.to_owned();
    }
    let scheme = base.split_once("://").map_or("https", |(scheme, _)| scheme);
    if let Some(rest) = location.strip_prefix("//") {
        return format!("{scheme}://{rest}");
    }
    let origin = match host(base) {
        Some(host) => format!("{scheme}://{host}"),
        None => return location.to_owned(),
    };
    if location.starts_with('/') {
        return format!("{origin}{location}");
    }
    let path = &base[origin.len().min(base.len())..];
    let path = path.split(['?', '#']).next().unwrap_or_default();
    let folder = path.rfind('/').map_or("/", |at| &path[..=at]);
    format!("{origin}{folder}{location}")
}

/// How a download asks again for the rest of its file.
struct Again {
    client: Arc<Client>,
    /// Where the chain of redirects began.
    url: String,
    accept: String,
    /// The hosts its redirects may go to: those the updater allowed.
    hosts: Vec<String>,
}

/// Why asking again gave nothing to read.
enum Missed {
    /// No answer: the network is away, or the server is. Asked again.
    Offline(io::Error),
    /// An answer that is no part of the file: asking again would get the
    /// same.
    Refused(io::Error),
    /// The download was cancelled while it asked, or no thread could ask:
    /// it stops at once.
    Stopped(io::Error),
}

impl Again {
    /// Asks `download`'s file from byte `from` on, following redirects to
    /// the hosts the updater allowed. Gives the answer's body, and how many
    /// of its bytes were read already: a server that ignores the range
    /// sends the whole file again.
    fn ask(&self, from: u64, download: &Download) -> Result<(Box<dyn Read + Send>, u64), Missed> {
        let refused = |text: String| Missed::Refused(io::Error::other(text));
        let mut url = self.url.clone();
        for _ in 0..=5 {
            let get = Get {
                client: Arc::clone(&self.client),
                url: url.clone(),
                accept: self.accept.clone(),
                range: Some(from),
            };
            let response = download
                .send(get)
                .map_err(Missed::Stopped)?
                .map_err(|error| Missed::Offline(io::Error::other(error)))?;
            let status = response.status().as_u16();
            match status {
                206 => {
                    let starts = header(&response, "content-range")
                        .and_then(|range| range_start(&range))
                        .unwrap_or_default();
                    if starts != from {
                        return Err(refused(format!(
                            "The update server sent the download from byte {starts}, not {from}"
                        )));
                    }
                    return Ok((Box::new(response.into_body().into_reader()), 0));
                }
                200 => return Ok((Box::new(response.into_body().into_reader()), from)),
                300..=399 => {
                    let Some(location) = header(&response, "location") else {
                        return Err(refused(
                            "The update server sent a redirect without a location".into(),
                        ));
                    };
                    let next = absolute(&url, &location);
                    if !may_follow(&next, &self.hosts) {
                        return Err(refused("Update redirect is not allowed".into()));
                    }
                    url = next;
                }
                // The same words as fastframe-update's.
                status => return Err(refused(format!("The update server answered {status}"))),
            }
        }
        Err(refused(
            "The update server redirected too many times".into(),
        ))
    }
}

/// Where `bytes 18400-31199/31200` starts.
fn range_start(range: &str) -> Option<u64> {
    let range = range.trim().strip_prefix("bytes")?.trim_start();
    let (start, _) = range.split_once('-')?;
    start.trim().parse().ok()
}

/// A piece of a connection's body as its pump read it: empty at its end.
type Piece = io::Result<Vec<u8>>;

/// Reads `reader` on a thread of its own into a short queue, so that a
/// body never blocks on a connection gone quiet. The thread ends with the
/// reader, or once the body stops listening: a connection left quiet ends
/// it at ureq's own timeout.
fn pump(mut reader: Box<dyn Read + Send>) -> io::Result<mpsc::Receiver<Piece>> {
    let (send, pieces) = mpsc::sync_channel(4);
    std::thread::Builder::new()
        .name("tabletist-update-body".into())
        .spawn(move || {
            let mut buffer = vec![0; 64 * 1024];
            loop {
                let piece = match reader.read(&mut buffer) {
                    Ok(read) => Ok(buffer[..read].to_vec()),
                    Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
                    Err(error) => Err(error),
                };
                let more = matches!(&piece, Ok(bytes) if !bytes.is_empty());
                if send.send(piece).is_err() || !more {
                    return;
                }
            }
        })?;
    Ok(pieces)
}

/// A download's body: what the first answer sends, then, where its
/// connection drops or goes quiet, the rest of the file from where it
/// stopped.
struct Body {
    /// The connection's body, as its pump reads it.
    pieces: Option<mpsc::Receiver<Piece>>,
    /// What came and was not handed on yet.
    held: Vec<u8>,
    /// How many bytes the connection sends again before the rest: a
    /// server that ignored the range.
    skip: u64,
    /// The bytes handed on so far.
    read: u64,
    /// The whole file's size, where the first answer said it.
    length: Option<u64>,
    /// When the connection last sent anything.
    heard: Instant,
    /// Since when no new byte of the file has come: from a drop until
    /// bytes come again. The download gives up `give_up` after it.
    lost: Option<Instant>,
    /// The reconnects since new bytes last came: each after the first
    /// waits `retry` first.
    tries: u32,
    again: Again,
    download: Arc<Download>,
}

impl Read for Body {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        loop {
            self.download.stop_if_cancelled()?;
            if !self.held.is_empty() {
                let count = buffer.len().min(self.held.len());
                buffer[..count].copy_from_slice(&self.held[..count]);
                self.held.drain(..count);
                self.read += count as u64;
                return Ok(count);
            }
            let Some(pieces) = &self.pieces else {
                self.resume()?;
                continue;
            };
            match pieces.recv_timeout(TICK) {
                // Short of the size it said: the connection closed early.
                Ok(Ok(bytes)) if bytes.is_empty() => {
                    if !self.length.is_some_and(|length| self.read < length) {
                        return Ok(0);
                    }
                    self.dropped("it closed early");
                }
                Ok(Ok(bytes)) => self.came(&bytes),
                Ok(Err(error)) => self.dropped(&error.to_string()),
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    if self.heard.elapsed() >= self.download.patience.quiet {
                        self.dropped("it went quiet");
                        self.download.wait(true);
                    }
                }
                Err(mpsc::RecvTimeoutError::Disconnected) => self.dropped("its reader ended"),
            }
        }
    }
}

impl Body {
    /// Bytes the connection sent: what it sent again is dropped, and the
    /// rest held to hand on. New bytes end a wait.
    fn came(&mut self, bytes: &[u8]) {
        self.heard = Instant::now();
        let again = usize::try_from(self.skip)
            .unwrap_or(usize::MAX)
            .min(bytes.len());
        self.skip -= again as u64;
        if again < bytes.len() {
            self.held.extend_from_slice(&bytes[again..]);
            self.lost = None;
            self.tries = 0;
            self.download.wait(false);
        }
    }

    /// The connection is gone, for `why`: the next read asks again.
    fn dropped(&mut self, why: &str) {
        log::info!(
            "the update download dropped after {} bytes: {why}",
            self.read
        );
        self.pieces = None;
        self.lost.get_or_insert_with(Instant::now);
    }

    /// Asks for the rest: at once after bytes came, otherwise after
    /// `retry`, waiting meanwhile, until `give_up` has passed since the
    /// file last came.
    fn resume(&mut self) -> io::Result<()> {
        let patience = self.download.patience;
        loop {
            self.download.stop_if_cancelled()?;
            let lost = *self.lost.get_or_insert_with(Instant::now);
            if lost.elapsed() >= patience.give_up {
                self.download.wait(false);
                log::info!("the update download gave up waiting");
                return Err(io::Error::other(LOST));
            }
            if self.tries > 0 {
                self.download.wait(true);
                self.download.pause(patience.retry)?;
            }
            self.tries += 1;
            match self.again.ask(self.read, &self.download) {
                Ok((reader, skip)) => {
                    self.pieces = Some(pump(reader)?);
                    self.skip = skip;
                    self.heard = Instant::now();
                    return Ok(());
                }
                Err(Missed::Refused(error)) => {
                    self.download.wait(false);
                    return Err(error);
                }
                Err(Missed::Stopped(error)) => return Err(error),
                Err(Missed::Offline(error)) => {
                    log::info!("the update server did not answer: {error}");
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::io::{BufRead, BufReader, Write};
    use std::net::{TcpListener, TcpStream};
    use std::sync::mpsc;

    use super::*;

    /// The file the fake server serves: a release's bytes, each its place.
    fn file() -> Vec<u8> {
        (0..200_000u32).map(|at| (at % 251) as u8).collect()
    }

    /// What a request asked: its request line and its headers, lower case.
    #[derive(Clone, Debug)]
    struct Asked {
        line: String,
        headers: Vec<(String, String)>,
    }

    impl Asked {
        fn range(&self) -> Option<u64> {
            self.headers
                .iter()
                .find(|(name, _)| name == "range")
                .and_then(|(_, value)| value.strip_prefix("bytes="))
                .and_then(|value| value.trim_end_matches('-').parse().ok())
        }
    }

    fn read_request(stream: &TcpStream) -> Asked {
        let mut reader = BufReader::new(stream.try_clone().unwrap());
        let mut line = String::new();
        reader.read_line(&mut line).unwrap();
        let mut headers = Vec::new();
        loop {
            let mut header = String::new();
            reader.read_line(&mut header).unwrap();
            if header == "\r\n" || header.is_empty() {
                break;
            }
            if let Some((name, value)) = header.trim_end().split_once(':') {
                headers.push((name.to_ascii_lowercase(), value.trim().to_owned()));
            }
        }
        Asked {
            line: line.trim_end().to_owned(),
            headers,
        }
    }

    /// How the fake server answers one connection.
    enum Answer {
        /// The file from `from` on, as a 206 where `ranged`, closed after
        /// `send` bytes where it is given.
        File {
            from: u64,
            send: Option<usize>,
            ranged: bool,
        },
        /// A connection closed with no answer: the network is away.
        Nothing,
        /// The file slowly, a chunk at a time, until the client stops
        /// reading.
        Slowly,
    }

    /// Serves `answers`, one connection each, on a local port, and gives
    /// back the address and what each request asked.
    fn serve(answers: Vec<Answer>) -> (String, mpsc::Receiver<Asked>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!(
            "http://{}/download/v99.0.0/tabletist.tar.gz",
            listener.local_addr().unwrap()
        );
        let (asked, heard) = mpsc::channel();
        std::thread::spawn(move || {
            let file = file();
            for answer in answers {
                let Ok((mut stream, _)) = listener.accept() else {
                    return;
                };
                let request = read_request(&stream);
                let _ = asked.send(request);
                match answer {
                    Answer::Nothing => drop(stream),
                    Answer::File { from, send, ranged } => {
                        let from = usize::try_from(from).unwrap();
                        let rest = &file[from..];
                        let head = match ranged {
                            true => format!(
                                "HTTP/1.1 206 Partial Content\r\nContent-Length: {}\r\nContent-Range: bytes {from}-{}/{}\r\nConnection: close\r\n\r\n",
                                rest.len(),
                                file.len() - 1,
                                file.len()
                            ),
                            false => format!(
                                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                                rest.len()
                            ),
                        };
                        let _ = stream.write_all(head.as_bytes());
                        let sent = send.unwrap_or(rest.len()).min(rest.len());
                        let _ = stream.write_all(&rest[..sent]);
                        let _ = stream.flush();
                        // A cut answer: closed short of its length.
                        let _ = stream.shutdown(std::net::Shutdown::Both);
                    }
                    Answer::Slowly => {
                        let head = format!(
                            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                            file.len()
                        );
                        let _ = stream.write_all(head.as_bytes());
                        for chunk in file.chunks(1_000) {
                            if stream.write_all(chunk).is_err() {
                                break;
                            }
                            std::thread::sleep(Duration::from_millis(20));
                        }
                    }
                }
            }
        });
        (url, heard)
    }

    const QUICK: Patience = Patience {
        retry: Duration::from_millis(50),
        give_up: Duration::from_secs(20),
        quiet: Duration::from_secs(20),
    };

    /// A download's transport, and each time it said it waited or not.
    fn downloading(patience: Patience) -> (UreqTransport, Arc<Download>, mpsc::Receiver<bool>) {
        let (told, heard) = mpsc::channel();
        let download = Download::new(move |waiting| {
            let _ = told.send(waiting);
        })
        .with_patience(patience);
        let download = Arc::new(download);
        let transport = UreqTransport::with_user_agent(
            "Tabletist/0.3.0 (Linux; x86_64)".into(),
            Some(Arc::clone(&download)),
        );
        (transport, download, heard)
    }

    fn get(transport: &UreqTransport, url: &str) -> Response {
        transport
            .get(&Request {
                url,
                accept: "application/octet-stream",
                user_agent: "ignored",
            })
            .unwrap()
    }

    fn read_all(response: Response) -> io::Result<Vec<u8>> {
        let mut body = response.body;
        let mut read = Vec::new();
        body.read_to_end(&mut read)?;
        Ok(read)
    }

    #[test]
    fn a_download_resumes_where_its_connection_dropped() {
        let (url, asked) = serve(vec![
            Answer::File {
                from: 0,
                send: Some(70_000),
                ranged: false,
            },
            Answer::File {
                from: 70_000,
                send: None,
                ranged: true,
            },
        ]);
        let (transport, download, waited) = downloading(QUICK);
        let read = read_all(get(&transport, &url)).unwrap();
        assert_eq!(read.len(), file().len());
        assert!(read == file(), "the bytes differ");
        let first = asked.recv().unwrap();
        assert_eq!(first.range(), None);
        let again = asked.recv().unwrap();
        assert_eq!(again.range(), Some(70_000), "{again:?}");
        assert!(
            again.line.starts_with("GET /download/v99.0.0/"),
            "{again:?}"
        );
        // The client says only what the first request said, and the range.
        let mut names: Vec<&str> = again
            .headers
            .iter()
            .map(|(name, _)| name.as_str())
            .collect();
        names.sort_unstable();
        assert_eq!(names, ["accept", "host", "range", "user-agent"]);
        // Back at once: it never waited.
        assert!(waited.try_recv().is_err());
        assert!(!download.waiting());
    }

    #[test]
    fn a_download_waits_while_the_network_is_away_then_goes_on() {
        let (url, _asked) = serve(vec![
            Answer::File {
                from: 0,
                send: Some(50_000),
                ranged: false,
            },
            Answer::Nothing,
            Answer::Nothing,
            Answer::Nothing,
            Answer::File {
                from: 50_000,
                send: None,
                ranged: true,
            },
        ]);
        let (transport, download, waited) = downloading(QUICK);
        let read = read_all(get(&transport, &url)).unwrap();
        assert!(read == file(), "the bytes differ");
        // It said it waited, then that it went on.
        assert_eq!(waited.try_iter().collect::<Vec<_>>(), [true, false]);
        assert!(!download.waiting());
    }

    #[test]
    fn a_download_that_does_not_come_back_in_time_is_lost() {
        let (url, _asked) = serve(vec![
            Answer::File {
                from: 0,
                send: Some(50_000),
                ranged: false,
            },
            Answer::Nothing,
            Answer::Nothing,
            Answer::Nothing,
            Answer::Nothing,
            Answer::Nothing,
            Answer::Nothing,
        ]);
        let patience = Patience {
            retry: Duration::from_millis(20),
            give_up: Duration::from_millis(100),
            quiet: Duration::from_secs(20),
        };
        let (transport, download, _waited) = downloading(patience);
        let error = read_all(get(&transport, &url)).unwrap_err();
        assert_eq!(error.to_string(), LOST);
        assert!(!download.waiting());
    }

    #[test]
    fn a_server_that_ignores_the_range_has_what_was_read_skipped() {
        let (url, asked) = serve(vec![
            Answer::File {
                from: 0,
                send: Some(120_000),
                ranged: false,
            },
            // The whole file, as a 200.
            Answer::File {
                from: 0,
                send: None,
                ranged: false,
            },
        ]);
        let (transport, _download, _waited) = downloading(QUICK);
        let read = read_all(get(&transport, &url)).unwrap();
        assert_eq!(read.len(), file().len());
        assert!(read == file(), "the bytes differ");
        let _ = asked.recv().unwrap();
        assert_eq!(asked.recv().unwrap().range(), Some(120_000));
    }

    #[test]
    fn a_cancel_fails_the_next_read() {
        let (url, _asked) = serve(vec![Answer::Slowly]);
        let (transport, download, _waited) = downloading(QUICK);
        let mut body = get(&transport, &url).body;
        let mut buffer = [0; 512];
        assert!(body.read(&mut buffer).unwrap() > 0);
        download.cancel();
        let error = body.read(&mut buffer).unwrap_err();
        assert_eq!(error.to_string(), CANCELLED);
    }

    /// While it waits for the network, a cancel ends the wait.
    #[test]
    fn a_cancel_ends_the_wait_for_the_network() {
        let (url, _asked) = serve(vec![
            Answer::File {
                from: 0,
                send: Some(10_000),
                ranged: false,
            },
            Answer::Nothing,
            Answer::Nothing,
            Answer::Nothing,
            Answer::Nothing,
            Answer::Nothing,
            Answer::Nothing,
            Answer::Nothing,
            Answer::Nothing,
        ]);
        let patience = Patience {
            retry: Duration::from_secs(5),
            give_up: Duration::from_secs(60),
            quiet: Duration::from_secs(20),
        };
        let (transport, download, waited) = downloading(patience);
        let body = get(&transport, &url).body;
        let reading = std::thread::spawn(move || read_all(Response::ok(body)));
        assert_eq!(waited.recv_timeout(Duration::from_secs(20)), Ok(true));
        let started = Instant::now();
        download.cancel();
        let error = reading.join().unwrap().unwrap_err();
        assert_eq!(error.to_string(), CANCELLED);
        assert!(
            started.elapsed() < Duration::from_secs(2),
            "{:?}",
            started.elapsed()
        );
    }

    /// The calls that download nothing are as any client's: a body that
    /// drops is an error.
    #[test]
    fn a_body_of_another_call_does_not_resume() {
        let (url, _asked) = serve(vec![Answer::File {
            from: 0,
            send: Some(10_000),
            ranged: false,
        }]);
        let transport =
            UreqTransport::with_user_agent("Tabletist/0.3.0 (Linux; x86_64)".into(), None);
        assert!(read_all(get(&transport, &url)).is_err());
    }

    /// A reconnect starts where the redirect chain began, for the address
    /// a redirect gives may be signed for a few minutes only.
    #[test]
    fn a_reconnect_starts_from_the_first_address_of_its_redirects() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        let (asked, heard) = mpsc::channel();
        std::thread::spawn(move || {
            let file = file();
            for round in 0..4 {
                let Ok((mut stream, _)) = listener.accept() else {
                    return;
                };
                let request = read_request(&stream);
                let _ = asked.send(request.line.clone());
                let signed = request.line.contains("/signed");
                let head = match (signed, request_range(&request)) {
                    (false, _) => format!(
                        "HTTP/1.1 302 Found\r\nLocation: /signed?round={round}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
                    ),
                    (true, None) => format!(
                        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                        file.len()
                    ),
                    (true, Some(from)) => format!(
                        "HTTP/1.1 206 Partial Content\r\nContent-Length: {}\r\nContent-Range: bytes {from}-{}/{}\r\nConnection: close\r\n\r\n",
                        file.len() - from,
                        file.len() - 1,
                        file.len()
                    ),
                };
                let _ = stream.write_all(head.as_bytes());
                match (signed, request_range(&request)) {
                    (true, None) => {
                        let _ = stream.write_all(&file[..30_000]);
                    }
                    (true, Some(from)) => {
                        let _ = stream.write_all(&file[from..]);
                    }
                    (false, _) => {}
                }
                let _ = stream.flush();
                let _ = stream.shutdown(std::net::Shutdown::Both);
            }
        });
        let (transport, _download, _waited) = downloading(QUICK);
        let first = get(
            &transport,
            &format!("{base}/download/v99.0.0/tabletist.tar.gz"),
        );
        assert_eq!(first.status, 302);
        let target = absolute(
            &format!("{base}/download/v99.0.0/tabletist.tar.gz"),
            first.location.as_deref().unwrap(),
        );
        let read = read_all(get(&transport, &target)).unwrap();
        assert!(read == file(), "the bytes differ");
        let lines: Vec<String> = heard.try_iter().collect();
        assert_eq!(
            lines,
            [
                "GET /download/v99.0.0/tabletist.tar.gz HTTP/1.1",
                "GET /signed?round=0 HTTP/1.1",
                "GET /download/v99.0.0/tabletist.tar.gz HTTP/1.1",
                "GET /signed?round=2 HTTP/1.1",
            ]
        );
    }

    fn request_range(asked: &Asked) -> Option<usize> {
        asked.range().map(|from| usize::try_from(from).unwrap())
    }

    #[test]
    fn a_redirect_is_made_absolute_against_its_request() {
        let base = "https://github.com/o/r/releases/download/v1/file.tar.gz";
        assert_eq!(
            absolute(base, "https://release-assets.githubusercontent.com/x?sig=1"),
            "https://release-assets.githubusercontent.com/x?sig=1"
        );
        assert_eq!(absolute(base, "/o/r/other"), "https://github.com/o/r/other");
        assert_eq!(
            absolute(base, "next.tar.gz"),
            "https://github.com/o/r/releases/download/v1/next.tar.gz"
        );
        assert_eq!(
            absolute(base, "//objects.githubusercontent.com/y"),
            "https://objects.githubusercontent.com/y"
        );
        assert_eq!(
            host("http://127.0.0.1:8123/a?b").as_deref(),
            Some("127.0.0.1:8123")
        );
        assert_eq!(range_start("bytes 18400-31199/31200"), Some(18_400));
        assert_eq!(range_start("items 1-2"), None);
    }

    /// Serves every connection, the n-th as `answer` says, with the
    /// request line and the range it asked, until the test ends.
    fn serve_each(
        answer: impl Fn(usize, &str, Option<u64>, &mut TcpStream) + Send + 'static,
    ) -> (
        String,
        Arc<std::sync::atomic::AtomicUsize>,
        mpsc::Receiver<String>,
    ) {
        use std::sync::atomic::AtomicUsize;
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        let count = Arc::new(AtomicUsize::new(0));
        let counted = Arc::clone(&count);
        let (asked, heard) = mpsc::channel();
        std::thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(mut stream) = stream else {
                    return;
                };
                let n = counted.fetch_add(1, Ordering::SeqCst);
                let request = read_request(&stream);
                let _ = asked.send(request.line.clone());
                answer(n, &request.line, request.range(), &mut stream);
            }
        });
        (base, count, heard)
    }

    /// The head of a 200 for the whole file, or of a 206 from `from`.
    fn head(from: Option<u64>) -> String {
        let length = file().len();
        match from {
            None => {
                format!("HTTP/1.1 200 OK\r\nContent-Length: {length}\r\nConnection: close\r\n\r\n")
            }
            Some(from) => format!(
                "HTTP/1.1 206 Partial Content\r\nContent-Length: {}\r\nContent-Range: bytes {from}-{}/{length}\r\nConnection: close\r\n\r\n",
                length as u64 - from,
                length - 1
            ),
        }
    }

    /// 50 kB of the file, then nothing, and no close: the network went
    /// with the socket open, as when Wi-Fi drops.
    fn goes_quiet(_: usize, _: &str, _: Option<u64>, stream: &mut TcpStream) {
        let _ = stream.write_all(head(None).as_bytes());
        let _ = stream.write_all(&file()[..50_000]);
        let _ = stream.flush();
        std::thread::sleep(Duration::from_secs(20));
    }

    /// Reads `body` on a thread of its own: what came, or the error.
    fn read_aside(mut body: Box<dyn Read + Send>) -> mpsc::Receiver<Result<Vec<u8>, String>> {
        let (done, finished) = mpsc::channel();
        std::thread::spawn(move || {
            let mut read = Vec::new();
            let result = body.read_to_end(&mut read).map(|_| read);
            let _ = done.send(result.map_err(|error| error.to_string()));
        });
        finished
    }

    #[test]
    fn a_cancel_ends_a_read_of_a_connection_gone_quiet() {
        let (base, _, _) = serve_each(goes_quiet);
        let (transport, download, _waited) = downloading(PATIENCE);
        let mut body = get(&transport, &format!("{base}/file")).body;
        let mut buffer = vec![0; 64 * 1024];
        let mut got = 0;
        while got < 50_000 {
            got += body.read(&mut buffer).unwrap();
        }
        let finished = read_aside(body);
        std::thread::sleep(Duration::from_millis(200));
        download.cancel();
        let result = finished.recv_timeout(Duration::from_secs(2));
        assert_eq!(result, Ok(Err(CANCELLED.to_owned())));
    }

    /// The reconnect after a drop is taken and never answered: a cancel
    /// still ends the read at once, not at ureq's timeout.
    #[test]
    fn a_cancel_ends_a_reconnect_that_gets_no_answer() {
        let (base, _, asked) = serve_each(|n, _, _, stream| {
            if n == 0 {
                let _ = stream.write_all(head(None).as_bytes());
                let _ = stream.write_all(&file()[..50_000]);
                let _ = stream.flush();
                let _ = stream.shutdown(std::net::Shutdown::Both);
            } else {
                std::thread::sleep(Duration::from_secs(60));
            }
        });
        let (transport, download, _waited) = downloading(PATIENCE);
        let finished = read_aside(get(&transport, &format!("{base}/file")).body);
        // The first request, then the reconnect.
        for _ in 0..2 {
            assert!(asked.recv_timeout(Duration::from_secs(10)).is_ok());
        }
        std::thread::sleep(Duration::from_millis(500));
        download.cancel();
        let result = finished.recv_timeout(Duration::from_secs(2));
        assert_eq!(result, Ok(Err(CANCELLED.to_owned())));
    }

    #[test]
    fn a_connection_gone_quiet_waits_for_the_network() {
        let (base, _, _) = serve_each(goes_quiet);
        let patience = Patience {
            quiet: Duration::from_millis(300),
            ..PATIENCE
        };
        let (transport, download, waited) = downloading(patience);
        let _finished = read_aside(get(&transport, &format!("{base}/file")).body);
        assert_eq!(waited.recv_timeout(Duration::from_secs(10)), Ok(true));
        download.cancel();
        // The reconnect sends again what came: no new byte, so it still
        // waits.
        assert!(download.waiting());
        // The default notices well before a person gives up on it.
        const { assert!(PATIENCE.quiet.as_secs() < 10) };
    }

    /// A server that answers each reconnect with its head, then drops it:
    /// the reconnects are paced, and the download gives up in time.
    #[test]
    fn a_reconnect_that_drops_at_once_is_paced_and_gives_up() {
        let (base, count, _) = serve_each(|n, _, range, stream| {
            if n == 0 {
                let _ = stream.write_all(head(None).as_bytes());
                let _ = stream.write_all(&file()[..70_000]);
            } else {
                let _ = stream.write_all(head(Some(range.unwrap_or(0))).as_bytes());
            }
            let _ = stream.flush();
            let _ = stream.shutdown(std::net::Shutdown::Both);
        });
        let patience = Patience {
            retry: Duration::from_millis(100),
            give_up: Duration::from_secs(1),
            quiet: Duration::from_secs(20),
        };
        let (transport, download, _waited) = downloading(patience);
        let finished = read_aside(get(&transport, &format!("{base}/file")).body);
        let result = finished.recv_timeout(Duration::from_secs(5));
        download.cancel();
        assert_eq!(result, Ok(Err(LOST.to_owned())));
        let asked = count.load(Ordering::SeqCst);
        assert!(asked < 20, "asked {asked} times in a second");
    }

    /// The updater asks for a redirect's target in its own form of the
    /// address (`url` resolves `..`): the reconnect still starts where the
    /// chain began.
    #[test]
    fn a_reconnect_starts_from_the_first_address_whatever_form_the_redirect_takes() {
        let (base, _, asked) = serve_each(|n, line, range, stream| {
            if line.contains("/download/") {
                let _ = stream.write_all(
                    b"HTTP/1.1 302 Found\r\nLocation: /signed/../blob?sig=1\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                );
            } else {
                let from = range.unwrap_or(0);
                let rest = &file()[usize::try_from(from).unwrap()..];
                let _ = stream.write_all(head(range).as_bytes());
                let send = if n == 1 { 70_000 } else { rest.len() };
                let _ = stream.write_all(&rest[..send]);
            }
            let _ = stream.flush();
            let _ = stream.shutdown(std::net::Shutdown::Both);
        });
        let (transport, _download, _waited) = downloading(QUICK);
        let first = get(
            &transport,
            &format!("{base}/download/v99.0.0/tabletist.tar.gz"),
        );
        assert_eq!(first.status, 302);
        let read = read_all(get(&transport, &format!("{base}/blob?sig=1"))).unwrap();
        assert!(read == file(), "the bytes differ");
        let lines: Vec<String> = asked.try_iter().collect();
        assert_eq!(
            lines.get(2).map(String::as_str),
            Some("GET /download/v99.0.0/tabletist.tar.gz HTTP/1.1"),
            "{lines:?}"
        );
    }

    /// No network when the download starts: it waits as after a drop.
    #[test]
    fn a_download_that_starts_offline_waits_for_the_network() {
        let (base, _, _) = serve_each(|n, _, _, stream| {
            if n >= 2 {
                let _ = stream.write_all(head(None).as_bytes());
                let _ = stream.write_all(&file());
                let _ = stream.flush();
            }
            let _ = stream.shutdown(std::net::Shutdown::Both);
        });
        let (transport, download, waited) = downloading(QUICK);
        let read = read_all(get(&transport, &format!("{base}/file"))).unwrap();
        assert!(read == file(), "the bytes differ");
        assert_eq!(waited.try_iter().collect::<Vec<_>>(), [true, false]);
        assert!(!download.waiting());
    }

    #[test]
    fn a_reconnect_follows_redirects_over_https_to_the_hosts_it_went_through() {
        let hosts = vec![
            "github.com".to_owned(),
            "release-assets.githubusercontent.com".to_owned(),
            "127.0.0.1:8123".to_owned(),
        ];
        assert!(may_follow(
            "https://release-assets.githubusercontent.com/x?sig=1",
            &hosts
        ));
        assert!(may_follow("http://127.0.0.1:8123/blob", &hosts));
        for refused in [
            "http://github.com/o/r/file",
            "https://user@github.com/o/r/file",
            "https://example.com/file",
            "ftp://github.com/file",
        ] {
            assert!(!may_follow(refused, &hosts), "{refused}");
        }
    }
}
