//! A cancel that is over only once the server has acted on it.
//!
//! The driver's own cancel writes the CancelRequest and returns: the
//! server may act on it later, when the session has gone on to its next
//! command, and that command is the one it stops. The server closes a
//! cancel's connection only after it has signalled the session's backend,
//! so this one waits for that close. A backend that gets the signal while
//! it waits for a command drops it.

use std::io;
use std::net::SocketAddr;
use std::pin::Pin;
use std::task::{Context, Poll, ready};
use std::time::Duration;

use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use tokio::net::TcpStream;
use tokio_postgres::CancelToken;
use tokio_postgres::tls::MakeTlsConnect;
use tokio_postgres_rustls::MakeRustlsConnect;

use crate::{Error, Result};

/// How long a cancel may take, from its connect to the server's close.
/// The session's next command waits for it, so a server or a middlebox
/// that never closes holds it up no longer than this.
const CANCEL_TIMEOUT: Duration = Duration::from_secs(10);

/// What a [`crate::CancelHandle`] needs to cancel the query a session runs.
#[derive(Clone)]
pub(crate) struct Cancel {
    pub(super) token: CancelToken,
    pub(super) tls: MakeRustlsConnect,
    /// The address the session reached and the host name TLS checks; the
    /// cancel goes to that same address. `None` when the session went to
    /// one the driver chose (a Unix socket, say): there the driver's own
    /// cancel goes, and does not wait.
    pub(super) to: Option<(SocketAddr, String)>,
}

impl Cancel {
    pub(crate) async fn cancel(&self) -> Result<()> {
        let failed = |error: String| Error::Connect(format!("could not cancel: {error}"));
        let Some((address, host)) = &self.to else {
            return self
                .token
                .cancel_query(self.tls.clone())
                .await
                .map_err(|error| failed(error.to_string()));
        };
        let sent = async {
            let socket = TcpStream::connect(address)
                .await
                .map_err(|error| failed(error.to_string()))?;
            // Sent at once, as the driver sends its own.
            let _ = socket.set_nodelay(true);
            let tls = MakeTlsConnect::<UntilClosed>::make_tls_connect(&mut self.tls.clone(), host)
                .unwrap_or_else(|never| match never {});
            self.token
                .cancel_query_raw(UntilClosed::new(socket), tls)
                .await
                .map_err(|error| failed(error.to_string()))
        };
        tokio::time::timeout(CANCEL_TIMEOUT, sent)
            .await
            .unwrap_or(Err(Error::Timeout))
    }
}

/// A cancel's socket, whose shutdown is over only once the server has
/// closed its end: the driver shuts the socket down once it has written
/// the request, and returns when the shutdown is over. Under TLS the
/// driver's shutdown sends the close_notify and then shuts this down.
struct UntilClosed {
    socket: TcpStream,
    /// Whether the write half is shut already.
    shut: bool,
}

impl UntilClosed {
    fn new(socket: TcpStream) -> Self {
        Self {
            socket,
            shut: false,
        }
    }
}

impl AsyncRead for UntilClosed {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        Pin::new(&mut self.socket).poll_read(cx, buf)
    }
}

impl AsyncWrite for UntilClosed {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        Pin::new(&mut self.socket).poll_write(cx, buf)
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.socket).poll_flush(cx)
    }

    /// Shuts the write half, then reads until the server closes. What it
    /// sends before that (its own close_notify) is of no use. A read that
    /// fails is a close too: a server that exits with bytes unread (a
    /// client's close_notify) resets the connection.
    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        if !self.shut {
            ready!(Pin::new(&mut self.socket).poll_shutdown(cx))?;
            self.shut = true;
        }
        let mut scratch = [0u8; 256];
        loop {
            let mut buf = ReadBuf::new(&mut scratch);
            match ready!(Pin::new(&mut self.socket).poll_read(cx, &mut buf)) {
                Ok(()) if buf.filled().is_empty() => return Poll::Ready(Ok(())),
                Ok(()) => {}
                Err(_) => return Poll::Ready(Ok(())),
            }
        }
    }
}
