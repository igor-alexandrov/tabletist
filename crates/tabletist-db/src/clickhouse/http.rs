//! ClickHouse's HTTP interface on the transport the other engines have: a
//! tokio `TcpStream`, wrapped by tokio-rustls with `tls::client_config`,
//! and HTTP/1.1 spoken by ureq-proto, which ureq uses inside.
//!
//! The stream goes to the server, or to an SSH tunnel's local port; TLS
//! checks the server's own name either way. Connections are kept alive,
//! two at most: one runs a statement while the other can cancel it.

use std::io;
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::{Arc, Mutex, PoisonError};
use std::task::{Context, Poll, ready};
use std::time::{Duration, Instant};

use rustls::pki_types::ServerName;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, ReadBuf};
use tokio::net::TcpStream;
use tokio_rustls::TlsConnector;
use ureq_proto::BodyMode;
use ureq_proto::client::state::RecvBody;
use ureq_proto::client::{
    Await100Result, Call, RecvBodyResult, RecvResponseResult, SendRequestResult,
};
use ureq_proto::http::{HeaderMap, Request};

use super::read::server_error;
use crate::{Error, Result, TlsMode, tls};

/// For opening a connection only. Nothing times a read, on purpose: a
/// statement's time is the server's `max_execution_time` and the person's
/// cancel (`KILL QUERY`), which the engine sets in its next phase.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(15);

/// How soon after a request a kept connection's end, with no byte of
/// answer, still means the server closed it idle, before reading it: that
/// close crosses our request within milliseconds, while a running query's
/// silence is longer.
const RETRY_WINDOW: Duration = Duration::from_secs(1);

/// Connections kept for the next request: one runs, one cancels.
const KEPT: usize = 2;

/// Where the server is and how to reach it.
#[derive(Debug, Clone, PartialEq)]
pub struct Endpoint {
    pub host: String,
    pub port: u16,
    /// An SSH tunnel's local port, which stands for `host:port`.
    pub via: Option<u16>,
    pub tls: TlsMode,
    pub ca_file: Option<PathBuf>,
}

/// Who to be on the server. No `Debug`: it holds the password.
#[derive(Clone)]
pub struct Login {
    pub user: String,
    pub password: String,
    /// Empty for the user's default database.
    pub database: String,
}

trait Io: AsyncRead + AsyncWrite + Unpin + Send {}
impl<T: AsyncRead + AsyncWrite + Unpin + Send> Io for T {}

type Stream = Box<dyn Io>;
type Kept = Arc<Mutex<Vec<Stream>>>;

/// The server's HTTP interface, and the connections kept to it.
pub(crate) struct Http {
    endpoint: Endpoint,
    login: Login,
    /// `None` for plain HTTP: `disable`, or a `prefer` the server did not
    /// take up.
    tls: Option<TlsConnector>,
    kept: Kept,
}

/// A 200 answer: its headers, and its body to read.
pub(crate) struct Answer {
    pub headers: HeaderMap,
    pub body: Body,
}

impl Answer {
    pub fn header(&self, name: &str) -> Option<String> {
        self.headers
            .get(name)
            .and_then(|value| value.to_str().ok())
            .map(str::to_owned)
    }
}

impl Http {
    /// Opens the first connection. Under `prefer` it decides TLS for the
    /// session: a server that answers the handshake with anything but TLS
    /// is spoken to in plain HTTP.
    pub async fn connect(endpoint: Endpoint, login: Login) -> Result<Self> {
        let tls = match endpoint.tls {
            TlsMode::Disable => None,
            mode @ (TlsMode::Prefer
            | TlsMode::Require
            | TlsMode::VerifyCa
            | TlsMode::VerifyFull) => {
                let config = tls::client_config(mode, endpoint.ca_file.as_deref())?;
                Some(TlsConnector::from(Arc::new(config)))
            }
        };
        let mut http = Self {
            endpoint,
            login,
            tls,
            kept: Arc::default(),
        };
        let stream = match http.open().await {
            Err(Opening::NoTls(_)) if http.endpoint.tls == TlsMode::Prefer => {
                http.tls = None;
                http.open().await.map_err(Opening::into_error)?
            }
            other => other.map_err(Opening::into_error)?,
        };
        http.keep(stream);
        Ok(http)
    }

    pub fn is_encrypted(&self) -> bool {
        self.tls.is_some()
    }

    /// Sends `sql` with `settings` in the URL. Any status but 200 is the
    /// server's error, read here.
    pub async fn post(&self, sql: &str, settings: &[(&str, &str)]) -> Result<Answer> {
        let exchanged = match self.take() {
            // The server closes a connection idle past its
            // `keep_alive_timeout`, and `take` drops those it sees closed.
            // One closed as we wrote is replaced, once: safe only because
            // the write failed, or the connection ended within
            // `RETRY_WINDOW` with no byte of answer, so nothing ran (or so
            // little, on a server that died then, that a read runs again). A
            // later end may be a running query's connection cut, and the
            // query goes on: sending it again would run it twice.
            Some(kept) => match self.exchange(kept, sql, settings).await {
                Err(Failure::Unsent(_)) => self.exchange(self.fresh().await?, sql, settings).await,
                other => other,
            },
            None => self.exchange(self.fresh().await?, sql, settings).await,
        };
        let (status, headers, mut body) = exchanged.map_err(Failure::into_error)?;
        if status == 200 {
            return Ok(Answer { headers, body });
        }
        let code = headers
            .get("x-clickhouse-exception-code")
            .and_then(|value| value.to_str().ok())
            .map(str::to_owned);
        let mut message = Vec::new();
        let _ = (&mut body).take(64 * 1024).read_to_end(&mut message).await;
        Err(server_error(
            code.as_deref(),
            &String::from_utf8_lossy(&message),
        ))
    }

    async fn fresh(&self) -> Result<Stream> {
        self.open().await.map_err(Opening::into_error)
    }

    async fn exchange(
        &self,
        mut stream: Stream,
        sql: &str,
        settings: &[(&str, &str)],
    ) -> std::result::Result<(u16, HeaderMap, Body), Failure> {
        let unsent = |error: io::Error| Failure::Unsent(lost(error));
        let mut request = Request::post(self.uri(settings))
            .header("X-ClickHouse-User", &self.login.user)
            .header("X-ClickHouse-Key", &self.login.password)
            .header("Content-Length", sql.len());
        if !self.login.database.is_empty() {
            request = request.header("X-ClickHouse-Database", &self.login.database);
        }
        let request = request.body(()).map_err(failed(protocol))?;

        let mut out = vec![0; 16 * 1024];
        let mut call = Call::new(request).map_err(failed(protocol))?.proceed();
        while !call.can_proceed() {
            let n = call.write(&mut out).map_err(failed(protocol))?;
            stream.write_all(&out[..n]).await.map_err(unsent)?;
        }
        let mut call = match call.proceed().map_err(failed(protocol))? {
            Some(SendRequestResult::SendBody(call)) => call,
            Some(SendRequestResult::Await100(call)) => {
                match call.proceed().map_err(failed(protocol))? {
                    Await100Result::SendBody(call) => call,
                    Await100Result::RecvResponse(_) => return Err(Failure::Lost(unexpected())),
                }
            }
            Some(SendRequestResult::RecvResponse(_)) | None => {
                return Err(Failure::Lost(unexpected()));
            }
        };
        let mut sent = 0;
        loop {
            let (used, n) = call
                .write(&sql.as_bytes()[sent..], &mut out)
                .map_err(failed(protocol))?;
            sent += used;
            stream.write_all(&out[..n]).await.map_err(unsent)?;
            if sent == sql.len() && call.can_proceed() {
                break;
            }
        }
        stream.flush().await.map_err(unsent)?;
        let sent_at = Instant::now();
        let mut call = call.proceed().ok_or_else(|| Failure::Lost(unexpected()))?;

        let mut input = Input::default();
        let response = loop {
            let (used, response) = call
                .try_response(input.pending(), false)
                .map_err(failed(protocol))?;
            input.consume(used);
            if let Some(response) = response {
                break response;
            }
            match input.fill(&mut stream).await {
                Ok(0) | Err(_) if !input.received => {
                    let error = Error::ConnectionLost(
                        "the server closed the connection before it answered".into(),
                    );
                    return Err(if sent_at.elapsed() < RETRY_WINDOW {
                        Failure::Unsent(error)
                    } else {
                        Failure::Lost(error)
                    });
                }
                Ok(0) => {
                    return Err(Failure::Lost(Error::ConnectionLost(
                        "the server closed the connection mid-answer".into(),
                    )));
                }
                Ok(_) => {}
                Err(error) => return Err(Failure::Lost(lost(error))),
            }
        };
        let (parts, ()) = response.into_parts();
        let body = match call.proceed() {
            Some(RecvResponseResult::RecvBody(call)) => Body {
                stream: Some(stream),
                call: Some(call),
                input,
                kept: Arc::clone(&self.kept),
            },
            Some(RecvResponseResult::Cleanup(cleanup)) => {
                if !cleanup.must_close_connection() {
                    self.keep(stream);
                }
                Body::empty(Arc::clone(&self.kept))
            }
            Some(RecvResponseResult::Redirect(_)) | None => {
                return Err(Failure::Lost(unexpected()));
            }
        };
        Ok((parts.status.as_u16(), parts.headers, body))
    }

    /// The request's URI: the server as TLS names it, `settings` as its
    /// query. The connection itself may go to a tunnel instead.
    fn uri(&self, settings: &[(&str, &str)]) -> String {
        let scheme = if self.tls.is_some() { "https" } else { "http" };
        let query = url::form_urlencoded::Serializer::new(String::new())
            .extend_pairs(settings)
            .finish();
        format!(
            "{scheme}://{}:{}/?{query}",
            authority_host(&self.endpoint.host),
            self.endpoint.port
        )
    }

    /// A new connection, with TLS where the session has it.
    async fn open(&self) -> std::result::Result<Stream, Opening> {
        let (host, port) = match self.endpoint.via {
            Some(via) => ("127.0.0.1", via),
            None => (self.endpoint.host.as_str(), self.endpoint.port),
        };
        let tcp = tokio::time::timeout(CONNECT_TIMEOUT, TcpStream::connect((host, port)))
            .await
            .map_err(|_| Opening::Failed(Error::Timeout))?
            .map_err(|error| Opening::Failed(Error::Connect(error.to_string())))?;
        let _ = tcp.set_nodelay(true);
        let Some(tls) = &self.tls else {
            return Ok(Box::new(tcp));
        };
        let name = ServerName::try_from(self.endpoint.host.clone())
            .map_err(|error| Opening::Failed(Error::Tls(error.to_string())))?;
        match tokio::time::timeout(CONNECT_TIMEOUT, tls.connect(name, tcp)).await {
            Err(_) => Err(Opening::Failed(Error::Timeout)),
            Ok(Ok(stream)) => Ok(Box::new(stream)),
            Ok(Err(error)) if speaks_no_tls(&error) => Err(Opening::NoTls(error)),
            Ok(Err(error)) => Err(Opening::Failed(Error::Tls(error.to_string()))),
        }
    }

    /// A kept connection still open, if there is one.
    fn take(&self) -> Option<Stream> {
        let mut kept = self.kept.lock().unwrap_or_else(PoisonError::into_inner);
        while let Some(mut stream) = kept.pop() {
            if is_idle(&mut stream) {
                return Some(stream);
            }
        }
        None
    }

    fn keep(&self, stream: Stream) {
        keep(&self.kept, stream);
    }
}

fn keep(kept: &Kept, stream: Stream) {
    let mut kept = kept.lock().unwrap_or_else(PoisonError::into_inner);
    if kept.len() < KEPT {
        kept.push(stream);
    }
}

/// Whether a kept connection is open with nothing to read, as one waiting
/// for our next request is. Asked before writing to it, so a connection
/// the server closed while it sat idle is dropped, not sent a statement:
/// a read polled once, without waiting, is pending on an open one, and
/// ready on one closed, failed, or holding bytes nobody asked for.
fn is_idle(stream: &mut Stream) -> bool {
    let mut cx = Context::from_waker(std::task::Waker::noop());
    let mut byte = [0; 1];
    let mut read = ReadBuf::new(&mut byte);
    Pin::new(stream).poll_read(&mut cx, &mut read).is_pending()
}

/// How an exchange failed: in a way that shows the server cannot have run
/// the statement, or in any other.
enum Failure {
    /// The request was not written, or the connection ended within
    /// `RETRY_WINDOW` of it with no byte of answer.
    Unsent(Error),
    Lost(Error),
}

impl Failure {
    fn into_error(self) -> Error {
        match self {
            Self::Unsent(error) | Self::Lost(error) => error,
        }
    }
}

fn failed<E>(to_error: impl Fn(E) -> Error) -> impl Fn(E) -> Failure {
    move |error| Failure::Lost(to_error(error))
}

/// Why a connection did not open.
enum Opening {
    /// The server answered the TLS handshake with something else.
    NoTls(io::Error),
    Failed(Error),
}

impl Opening {
    fn into_error(self) -> Error {
        match self {
            Self::NoTls(error) => Error::Tls(format!("the server does not speak TLS: {error}")),
            Self::Failed(error) => error,
        }
    }
}

/// Whether a failed handshake means the server does not speak TLS on the
/// port: it answered with bytes that are no TLS record, or closed. As
/// with libpq's `prefer`, a TLS server that hangs up mid-handshake is
/// taken for one without TLS too, so the session goes on in plain HTTP.
fn speaks_no_tls(error: &io::Error) -> bool {
    if let Some(rustls::Error::InvalidMessage(_)) = error
        .get_ref()
        .and_then(|inner| inner.downcast_ref::<rustls::Error>())
    {
        return true;
    }
    error.kind() == io::ErrorKind::UnexpectedEof
}

/// An IPv6 address in brackets, as a URI writes it.
fn authority_host(host: &str) -> String {
    if host.contains(':') {
        format!("[{host}]")
    } else {
        host.to_owned()
    }
}

fn protocol(error: impl std::fmt::Display) -> Error {
    Error::ConnectionLost(format!("HTTP: {error}"))
}

fn lost(error: io::Error) -> Error {
    Error::ConnectionLost(error.to_string())
}

fn unexpected() -> Error {
    Error::ConnectionLost("the server answered out of turn".into())
}

/// Bytes read from the connection and not yet handed to ureq-proto.
#[derive(Default)]
struct Input {
    buffer: Vec<u8>,
    start: usize,
    /// Whether the server sent anything yet.
    received: bool,
}

impl Input {
    fn pending(&self) -> &[u8] {
        &self.buffer[self.start..]
    }

    fn consume(&mut self, used: usize) {
        self.start += used;
        if self.start == self.buffer.len() {
            self.buffer.clear();
            self.start = 0;
        }
    }

    async fn fill(&mut self, stream: &mut Stream) -> io::Result<usize> {
        let mut chunk = [0; 16 * 1024];
        let n = stream.read(&mut chunk).await?;
        self.buffer.extend_from_slice(&chunk[..n]);
        self.received |= n > 0;
        Ok(n)
    }

    fn poll_fill(&mut self, stream: &mut Stream, cx: &mut Context<'_>) -> Poll<io::Result<usize>> {
        let mut chunk = [0; 16 * 1024];
        let mut read = ReadBuf::new(&mut chunk);
        ready!(Pin::new(stream).poll_read(cx, &mut read))?;
        let n = read.filled().len();
        self.buffer.extend_from_slice(read.filled());
        Poll::Ready(Ok(n))
    }
}

/// An answer's body. Read to its end, its connection is kept for the next
/// request; dropped before, the connection closes, which stops what the
/// server was sending.
pub(crate) struct Body {
    stream: Option<Stream>,
    call: Option<Call<RecvBody>>,
    input: Input,
    kept: Kept,
}

impl Body {
    fn empty(kept: Kept) -> Self {
        Self {
            stream: None,
            call: None,
            input: Input::default(),
            kept,
        }
    }

    /// The body is over: its connection goes back, unless it must close.
    fn finish(&mut self) {
        let (Some(call), Some(stream)) = (self.call.take(), self.stream.take()) else {
            return;
        };
        if let Some(RecvBodyResult::Cleanup(cleanup)) = call.proceed()
            && !cleanup.must_close_connection()
            && self.input.pending().is_empty()
        {
            keep(&self.kept, stream);
        }
    }
}

impl AsyncRead for Body {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        let this = self.get_mut();
        loop {
            let (Some(call), Some(stream)) = (this.call.as_mut(), this.stream.as_mut()) else {
                return Poll::Ready(Ok(()));
            };
            let close_delimited = matches!(call.body_mode(), BodyMode::CloseDelimited);
            if call.can_proceed() && !close_delimited {
                this.finish();
                return Poll::Ready(Ok(()));
            }
            if buf.remaining() == 0 {
                return Poll::Ready(Ok(()));
            }
            if !this.input.pending().is_empty() {
                let (used, made) = call
                    .read(this.input.pending(), buf.initialize_unfilled())
                    .map_err(io::Error::other)?;
                this.input.consume(used);
                if made > 0 {
                    buf.advance(made);
                    return Poll::Ready(Ok(()));
                }
                if used > 0 {
                    continue;
                }
            }
            if ready!(this.input.poll_fill(stream, cx))? == 0 {
                if close_delimited {
                    this.call = None;
                    this.stream = None;
                    return Poll::Ready(Ok(()));
                }
                return Poll::Ready(Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "the server closed the connection mid-answer",
                )));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};

    use super::*;

    #[test]
    fn a_record_that_is_not_tls_or_a_close_means_no_tls() {
        let corrupt = io::Error::new(
            io::ErrorKind::InvalidData,
            rustls::Error::InvalidMessage(rustls::InvalidMessage::InvalidContentType),
        );
        assert!(speaks_no_tls(&corrupt));
        assert!(speaks_no_tls(&io::Error::from(
            io::ErrorKind::UnexpectedEof
        )));
        let untrusted = io::Error::new(
            io::ErrorKind::InvalidData,
            rustls::Error::InvalidCertificate(rustls::CertificateError::UnknownIssuer),
        );
        assert!(!speaks_no_tls(&untrusted));
    }

    /// A server that answers each connection's first request with `ok`,
    /// keep-alive, and then closes it, as ClickHouse does with a connection
    /// idle past its `keep_alive_timeout`.
    async fn closing_server() -> u16 {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        tokio::spawn(async move {
            while let Ok((mut socket, _)) = listener.accept().await {
                tokio::spawn(async move {
                    // The whole request before the answer: closing with
                    // bytes unread would send a reset, which on Windows can
                    // discard the answer.
                    if read_request(&mut socket).await {
                        let _ = socket.write_all(OK).await;
                    }
                });
            }
        });
        port
    }

    /// Reads one request, head and body (`go`), whole. False at the end of
    /// the connection.
    async fn read_request(socket: &mut TcpStream) -> bool {
        let mut request = Vec::new();
        let mut chunk = [0; 1024];
        while !request.ends_with(b"\r\n\r\ngo") {
            match socket.read(&mut chunk).await {
                Ok(0) | Err(_) => return false,
                Ok(n) => request.extend_from_slice(&chunk[..n]),
            }
        }
        true
    }

    const OK: &[u8] = b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\nok";

    /// A server that answers each connection's first request and then
    /// closes its side, but still reads and counts what comes after.
    async fn half_closing_server(requests: Arc<AtomicUsize>) -> u16 {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        tokio::spawn(async move {
            while let Ok((mut socket, _)) = listener.accept().await {
                let requests = Arc::clone(&requests);
                tokio::spawn(async move {
                    if !read_request(&mut socket).await {
                        return;
                    }
                    requests.fetch_add(1, Ordering::SeqCst);
                    let _ = socket.write_all(OK).await;
                    let _ = socket.shutdown().await;
                    while read_request(&mut socket).await {
                        requests.fetch_add(1, Ordering::SeqCst);
                    }
                });
            }
        });
        port
    }

    /// A server that answers each connection's first request, and is
    /// silent on the second for longer than `RETRY_WINDOW`, as a long query
    /// is, until the connection is cut.
    async fn silent_server(requests: Arc<AtomicUsize>) -> u16 {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        tokio::spawn(async move {
            while let Ok((mut socket, _)) = listener.accept().await {
                let requests = Arc::clone(&requests);
                tokio::spawn(async move {
                    if !read_request(&mut socket).await {
                        return;
                    }
                    requests.fetch_add(1, Ordering::SeqCst);
                    let _ = socket.write_all(OK).await;
                    if !read_request(&mut socket).await {
                        return;
                    }
                    requests.fetch_add(1, Ordering::SeqCst);
                    tokio::time::sleep(Duration::from_millis(1500)).await;
                });
            }
        });
        port
    }

    async fn connect(port: u16) -> Http {
        let endpoint = Endpoint {
            host: "127.0.0.1".into(),
            port,
            via: None,
            tls: TlsMode::Disable,
            ca_file: None,
        };
        let login = Login {
            user: "u".into(),
            password: "p".into(),
            database: String::new(),
        };
        Http::connect(endpoint, login).await.unwrap()
    }

    async fn post_ok(http: &Http) {
        let mut answer = http.post("go", &[]).await.unwrap();
        let mut body = String::new();
        answer.body.read_to_string(&mut body).await.unwrap();
        assert_eq!(body, "ok");
    }

    #[tokio::test]
    async fn a_kept_connection_the_server_closed_is_replaced() {
        let http = connect(closing_server().await).await;
        for _ in 0..3 {
            post_ok(&http).await;
            // Let the server's close arrive before the next request.
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    }

    #[tokio::test]
    async fn a_kept_connection_the_server_closed_is_not_written_to() {
        let requests = Arc::new(AtomicUsize::new(0));
        let http = connect(half_closing_server(Arc::clone(&requests)).await).await;
        for _ in 0..3 {
            post_ok(&http).await;
            // Time for the server's FIN to arrive before the next request.
            tokio::time::sleep(Duration::from_millis(200)).await;
        }
        assert_eq!(requests.load(Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn a_connection_cut_after_a_long_silence_is_not_sent_again() {
        let requests = Arc::new(AtomicUsize::new(0));
        let http = connect(silent_server(Arc::clone(&requests)).await).await;
        post_ok(&http).await;
        let cut = http.post("go", &[]).await;
        tokio::time::sleep(Duration::from_millis(50)).await;
        assert_eq!(requests.load(Ordering::SeqCst), 2);
        assert!(matches!(cut, Err(Error::ConnectionLost(_))));
    }

    #[test]
    fn an_ipv6_host_is_bracketed() {
        assert_eq!(authority_host("::1"), "[::1]");
        assert_eq!(authority_host("db.example.com"), "db.example.com");
    }
}
