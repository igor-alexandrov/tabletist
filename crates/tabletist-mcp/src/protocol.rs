//! The Model Context Protocol, through its SDK. This module alone names the
//! SDK's types: the tools and the server speak this crate's own, so that
//! the SDK can change, or be replaced, behind it.

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use std::time::Duration;

use rmcp::model::{
    CallToolRequestParams, CallToolResponse, CallToolResult, ContentBlock, Implementation,
    InitializeRequestParams, InitializeResult, ListToolsResult, PaginatedRequestParams,
    ServerCapabilities, ServerConfig, Tool, ToolAnnotations,
};
use rmcp::service::{RequestContext, serve_server_with_ct};
use rmcp::{ErrorData, RoleServer, ServerHandler};
use serde_json::{Map, Value as Json, json};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use tokio_util::sync::{CancellationToken, WaitForCancellationFutureOwned};

use crate::gateway::Gateway;
use crate::server::{Live, Reply, Server};
use crate::stop::{Stop, Why};
use crate::tools;

/// What a client is told the server is for.
const INSTRUCTIONS: &str = "Tabletist lets you read the databases its user has opened to agents. \
                            Start with list_connections, which names them. Nothing you run \
                            here can change data. A refusal says why, in a sentence you can \
                            act on.";

/// The most bytes of one message from a client. A statement is at most a
/// tenth of it. The SDK reads a line into memory however long it is.
pub(crate) const MAX_MESSAGE_BYTES: usize = 1_000_000;

/// How long the SDK is given to close a session that was stopped, before
/// its transport is taken from it. It sends what it still has first, and a
/// client that reads nothing would hold it at that for good.
const LET_GO: Duration = Duration::from_secs(3);

/// Serves `live`'s agent on `io` until the client closes it or the session
/// is stopped.
pub(crate) async fn serve<G, T>(server: Server<G>, live: Arc<Live>, io: T)
where
    G: Gateway,
    T: AsyncRead + AsyncWrite + Send + Unpin + 'static,
{
    let handler = Handler {
        server,
        live: live.clone(),
    };
    let shut = CancellationToken::new();
    let io = Watched::new(io, live.stop.clone(), shut.clone());
    let close = CancellationToken::new();
    let start = serve_server_with_ct(handler, io, close.clone());
    tokio::pin!(start);
    let running = tokio::select! {
        running = &mut start => running,
        () = live.stop.ended() => {
            close.cancel();
            start.await
        }
    };
    let Ok(running) = running else {
        // What came first was not the start of a session.
        return;
    };
    let ended = running.waiting();
    tokio::pin!(ended);
    tokio::select! {
        // The client closed its end. The SDK has sent the answers of the
        // calls that ended within five seconds of that, and cancelled
        // none: what still runs is ended by whoever called this.
        _ = &mut ended => {}
        () = live.stop.ended() => {
            // Each call in flight answers that it was stopped. Those
            // answers go out before the transport closes.
            live.settled().await;
            close.cancel();
            if tokio::time::timeout(LET_GO, &mut ended).await.is_err() {
                shut.cancel();
                let _ = ended.await;
            }
        }
    }
}

/// A client's transport, as far as the SDK is given it.
///
/// - A write that fails ends the session's calls: the client is gone, and
///   nobody will read an answer.
/// - A message longer than [`MAX_MESSAGE_BYTES`] fails the read, which
///   ends the session.
/// - Once `shut` is cancelled every read and write fails, a waiting one
///   too, so that the SDK lets go of a transport it cannot finish with.
struct Watched<T> {
    io: T,
    stop: Stop,
    shut: CancellationToken,
    // One for each half: the SDK reads and writes on two tasks, and each
    // must be woken.
    shut_reading: Pin<Box<WaitForCancellationFutureOwned>>,
    shut_writing: Pin<Box<WaitForCancellationFutureOwned>>,
    /// The bytes read since the last end of a line.
    line: usize,
}

impl<T> Watched<T> {
    fn new(io: T, stop: Stop, shut: CancellationToken) -> Self {
        Self {
            io,
            stop,
            shut_reading: Box::pin(shut.clone().cancelled_owned()),
            shut_writing: Box::pin(shut.clone().cancelled_owned()),
            shut,
            line: 0,
        }
    }
}

impl<T> Watched<T> {
    /// Whether the transport was shut, for the half that writes. Asked at
    /// every write, flush and close, so that whichever of them is waiting
    /// is woken when it is: a pipe makes a flush wait where a transport in
    /// memory makes a write wait.
    fn shut_for_writing(&mut self, context: &mut Context<'_>) -> bool {
        self.shut.is_cancelled() || self.shut_writing.as_mut().poll(context).is_ready()
    }
}

fn was_shut() -> std::io::Error {
    std::io::Error::new(
        std::io::ErrorKind::ConnectionAborted,
        "the session was stopped",
    )
}

impl<T: AsyncRead + Unpin> AsyncRead for Watched<T> {
    fn poll_read(
        mut self: Pin<&mut Self>,
        context: &mut Context<'_>,
        buffer: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        if self.shut.is_cancelled() || self.shut_reading.as_mut().poll(context).is_ready() {
            return Poll::Ready(Err(was_shut()));
        }
        let before = buffer.filled().len();
        let polled = Pin::new(&mut self.io).poll_read(context, buffer);
        if let Poll::Ready(Ok(())) = &polled {
            let read = &buffer.filled()[before..];
            self.line = match read.iter().rposition(|byte| *byte == b'\n') {
                Some(end) => read.len() - end - 1,
                None => self.line + read.len(),
            };
            if self.line > MAX_MESSAGE_BYTES {
                return Poll::Ready(Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "a message longer than the server reads",
                )));
            }
        }
        polled
    }
}

impl<T: AsyncWrite + Unpin> AsyncWrite for Watched<T> {
    fn poll_write(
        mut self: Pin<&mut Self>,
        context: &mut Context<'_>,
        bytes: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        if self.shut_for_writing(context) {
            return Poll::Ready(Err(was_shut()));
        }
        let polled = Pin::new(&mut self.io).poll_write(context, bytes);
        if matches!(polled, Poll::Ready(Err(_))) {
            self.stop.end(Why::Client);
        }
        polled
    }

    fn poll_flush(
        mut self: Pin<&mut Self>,
        context: &mut Context<'_>,
    ) -> Poll<std::io::Result<()>> {
        if self.shut_for_writing(context) {
            return Poll::Ready(Err(was_shut()));
        }
        Pin::new(&mut self.io).poll_flush(context)
    }

    fn poll_shutdown(
        mut self: Pin<&mut Self>,
        context: &mut Context<'_>,
    ) -> Poll<std::io::Result<()>> {
        if self.shut_for_writing(context) {
            // Shut already: there is nothing more to say to this client.
            return Poll::Ready(Ok(()));
        }
        Pin::new(&mut self.io).poll_shutdown(context)
    }
}

struct Handler<G: Gateway> {
    server: Server<G>,
    live: Arc<Live>,
}

impl<G: Gateway> ServerHandler for Handler<G> {
    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(
                Implementation::new("tabletist", self.server.version()).with_title("Tabletist"),
            )
            .with_instructions(INSTRUCTIONS)
    }

    /// The SDK's own, and the agent's name, which a client of the older
    /// protocols gives here and nowhere else.
    async fn initialize(
        &self,
        request: InitializeRequestParams,
        context: RequestContext<RoleServer>,
    ) -> Result<InitializeResult, ErrorData> {
        let client = &request.client_info;
        self.live.name_agent(client.title.as_deref(), &client.name);
        context.peer.set_peer_info(request.clone());
        self.negotiate_initialize(&request)
    }

    async fn list_tools(
        &self,
        _request: Option<PaginatedRequestParams>,
        context: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, ErrorData> {
        self.name_agent(&context);
        Ok(ListToolsResult::with_all_items(
            tools::all().into_iter().map(tool).collect(),
        ))
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        context: RequestContext<RoleServer>,
    ) -> Result<CallToolResponse, ErrorData> {
        self.name_agent(&context);
        let arguments = request.arguments.unwrap_or_default();
        let reply = self
            .server
            .call(&self.live, &request.name, &arguments, &context.ct)
            .await
            .map_err(|invalid| ErrorData::invalid_params(invalid.message(), None))?;
        Ok(result(reply).into())
    }
}

impl<G: Gateway> Handler<G> {
    /// Since the protocol's version of 2026-07-28 a client says what it is
    /// called with each request, and has no handshake to say it in.
    fn name_agent(&self, context: &RequestContext<RoleServer>) {
        if let Some(client) = context.client_info() {
            self.live.name_agent(client.title.as_deref(), &client.name);
        }
    }
}

fn tool(tool: tools::Tool) -> Tool {
    let input = match tool.input {
        Json::Object(schema) => schema,
        _ => Map::new(),
    };
    Tool::new(tool.name, tool.description, Arc::new(input))
        .with_title(tool.title)
        .with_annotations(ToolAnnotations::new().read_only(true))
}

/// An answer is its JSON as text, once: a client that hands a model both a
/// text and a structured result would hand it the rows twice. A refusal is
/// a tool's result with `isError`, which a model reads (a protocol error
/// it may never see), with its code where a program finds it.
fn result(reply: Reply) -> CallToolResult {
    match reply {
        Reply::Answer(text) => CallToolResult::success(vec![ContentBlock::text(text)]),
        Reply::Refused(refused) => {
            let mut result = CallToolResult::error(vec![ContentBlock::text(&refused.message)]);
            result.structured_content = Some(json!({
                "refused": refused.code,
                "message": refused.message,
            }));
            result
        }
    }
}

#[cfg(test)]
mod tests {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    use super::*;

    /// A transport whose client takes nothing and sends nothing: a write
    /// goes into a buffer, and a read, a flush and a close never end.
    struct Stuck;

    impl AsyncRead for Stuck {
        fn poll_read(
            self: Pin<&mut Self>,
            _context: &mut Context<'_>,
            _buffer: &mut ReadBuf<'_>,
        ) -> Poll<std::io::Result<()>> {
            Poll::Pending
        }
    }

    impl AsyncWrite for Stuck {
        fn poll_write(
            self: Pin<&mut Self>,
            _context: &mut Context<'_>,
            bytes: &[u8],
        ) -> Poll<std::io::Result<usize>> {
            Poll::Ready(Ok(bytes.len()))
        }

        fn poll_flush(
            self: Pin<&mut Self>,
            _context: &mut Context<'_>,
        ) -> Poll<std::io::Result<()>> {
            Poll::Pending
        }

        fn poll_shutdown(
            self: Pin<&mut Self>,
            _context: &mut Context<'_>,
        ) -> Poll<std::io::Result<()>> {
            Poll::Pending
        }
    }

    /// Whatever the SDK is waiting in when the transport is shut, it is
    /// woken: a duplex in memory only ever makes a write wait, and a pipe
    /// or a buffered writer makes a flush wait.
    #[tokio::test]
    async fn whatever_waits_on_the_transport_ends_when_it_is_shut() {
        for waits_in in ["read", "flush", "close"] {
            let shut = CancellationToken::new();
            let mut watched = Watched::new(Stuck, Stop::new(), shut.clone());
            let waiting = tokio::spawn(async move {
                match waits_in {
                    "read" => watched.read(&mut [0; 8]).await.is_err(),
                    "flush" => watched.flush().await.is_err(),
                    // Nothing more is said to a client the transport was
                    // taken from, so its close is done.
                    _ => watched.shutdown().await.is_ok(),
                }
            });
            tokio::time::sleep(Duration::from_millis(50)).await;
            assert!(!waiting.is_finished(), "{waits_in}");
            shut.cancel();
            let ended = tokio::time::timeout(Duration::from_secs(5), waiting).await;
            assert!(
                ended.is_ok_and(|ended| ended.is_ok_and(|as_it_should| as_it_should)),
                "a {waits_in} that waited was not woken"
            );
        }
    }
}
