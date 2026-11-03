//! The server's endpoint: the socket `tabletist mcp` connects to on Linux
//! and macOS, the named pipe on Windows, and the key each side proves it
//! holds before it says anything else.

#[cfg(unix)]
use std::os::unix::fs::{FileTypeExt, MetadataExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use tabletist_mcp::{Origin, Server, Transport};
use tokio::io::{AsyncRead, AsyncWrite, AsyncWriteExt};
#[cfg(unix)]
use tokio::net::UnixListener;
#[cfg(windows)]
use tokio::net::windows::named_pipe::{NamedPipeServer, ServerOptions};
use tokio::sync::{OwnedSemaphorePermit, Semaphore};
use tokio_util::sync::CancellationToken;

use super::AppGateway;
use crate::endpoint::{self, Answer, Hello, Reply, Side};

/// The most agents connected at once. One more is refused: a program of
/// the user's that opens sessions without end runs into it, and the
/// connections to the databases do not.
const MAX_SESSIONS: usize = 32;

/// The most connections that have not proved the key yet, from their
/// [`Hello`] to their [`Reply`]: connections that never say anything cannot
/// use up the app's descriptors, nor the sessions of the agents that have
/// the key. On a socket the next waits in the system's queue, holding
/// nothing of the app's. A pipe has no queue: its one waiting instance
/// goes to the next, and those after it find the pipe busy, and are told
/// so once their relay has tried for its time (`endpoint::BUSY`).
const MAX_GREETINGS: usize = 64;

/// The endpoint while the server listens on it. Dropped, it listens no
/// more, and its socket and key are gone.
pub(super) struct Listening {
    stop: CancellationToken,
    _files: Files,
}

impl Drop for Listening {
    fn drop(&mut self) {
        self.stop.cancel();
    }
}

/// The socket and the key the server made, removed when it stops: each
/// only while it is the one it made, which a later start may have put
/// another in the place of. On Windows the file that names the pipe, in
/// the socket's place: the pipe goes with its last instance. The
/// directory's lock is let go after them.
struct Files {
    #[cfg(unix)]
    socket: Option<(PathBuf, (u64, u64))>,
    #[cfg(windows)]
    pipe: Option<(PathBuf, String)>,
    key: (PathBuf, String),
    _locked: std::fs::File,
}

impl Drop for Files {
    fn drop(&mut self) {
        #[cfg(unix)]
        if let Some((socket, made)) = &self.socket
            && std::fs::symlink_metadata(socket).is_ok_and(|now| (now.dev(), now.ino()) == *made)
        {
            let _ = std::fs::remove_file(socket);
        }
        #[cfg(windows)]
        if let Some(named) = &self.pipe {
            remove_if_still(named);
        }
        remove_if_still(&self.key);
    }
}

/// Removes `file` while it holds what this server wrote in it.
fn remove_if_still((file, wrote): &(PathBuf, String)) {
    if std::fs::read_to_string(file).is_ok_and(|there| there == *wrote) {
        let _ = std::fs::remove_file(file);
    }
}

/// The directory's lock, opened, and made where there is none.
fn lock_file(dir: &Path) -> std::io::Result<std::fs::File> {
    let mut options = std::fs::OpenOptions::new();
    options.read(true).write(true).create(true).truncate(false);
    #[cfg(unix)]
    std::os::unix::fs::OpenOptionsExt::mode(&mut options, 0o600);
    options.open(dir.join(endpoint::LOCK))
}

/// Whether another Tabletist is serving its agents in `dir`: it holds the
/// directory's lock. Asks nothing of its socket, so that its log has no
/// line of the asking.
pub(super) fn served_by_another(dir: &Path) -> bool {
    dir.is_dir()
        && lock_file(dir)
            .is_ok_and(|file| matches!(file.try_lock(), Err(std::fs::TryLockError::WouldBlock)))
}

/// Listens in `dir` for `server`: makes the directory private, a new key,
/// and the socket, or on Windows the pipe. Why it could not, for the
/// person.
pub(super) fn listen(dir: &Path, server: Server<AppGateway>) -> Result<Listening, String> {
    let shown = dir.display();
    crate::util::create_private_dir(dir)
        .map_err(|error| format!("could not make the folder {shown}: {error}"))?;
    // Only its owner can change a folder's mode: one that someone else made
    // there first is refused, not used. On Windows the folder is in the
    // user's own profile, which only they may change.
    #[cfg(unix)]
    std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o700))
        .map_err(|error| format!("the folder {shown} is not yours to use: {error}"))?;
    // Held for as long as it listens: a second start stops here, before
    // it can write its key over this one's.
    let locked = lock_file(dir).map_err(|error| {
        format!(
            "could not open {}: {error}",
            dir.join(endpoint::LOCK).display()
        )
    })?;
    match locked.try_lock() {
        Ok(()) => {}
        Err(std::fs::TryLockError::WouldBlock) => return Err(endpoint::ANOTHER_SERVES.into()),
        Err(std::fs::TryLockError::Error(error)) => {
            return Err(format!(
                "could not lock {}: {error}",
                dir.join(endpoint::LOCK).display()
            ));
        }
    }
    #[cfg(unix)]
    return listen_on_socket(dir, locked, server);
    #[cfg(windows)]
    return listen_on_pipe(dir, locked, server);
}

/// The key, new for this start, written before anything is listened on: a
/// relay that connects the moment it can reads the key of this start, not
/// one an ended start left.
fn new_key(dir: &Path, locked: std::fs::File) -> Result<(Files, String), String> {
    let key = endpoint::new_key();
    let key_file = dir.join(endpoint::KEY);
    let files = Files {
        #[cfg(unix)]
        socket: None,
        #[cfg(windows)]
        pipe: None,
        key: (key_file.clone(), key.clone()),
        _locked: locked,
    };
    crate::util::write_atomic(&key_file, key.as_bytes())
        .map_err(|error| format!("could not write {}: {error}", key_file.display()))?;
    Ok((files, key))
}

/// Listens on the socket in `dir`, whose lock is `locked`.
#[cfg(unix)]
fn listen_on_socket(
    dir: &Path,
    locked: std::fs::File,
    server: Server<AppGateway>,
) -> Result<Listening, String> {
    let socket = dir.join(endpoint::SOCKET);
    match std::fs::symlink_metadata(&socket) {
        // With the lock held, a socket there was left by a Tabletist that
        // ended.
        Ok(found) if found.file_type().is_socket() => {
            std::fs::remove_file(&socket).map_err(|error| {
                format!("could not remove an old {}: {error}", socket.display())
            })?;
        }
        Ok(_) => return Err(format!("{} is in the way", socket.display())),
        Err(_) => {}
    }
    let (mut files, key) = new_key(dir, locked)?;
    let listener = UnixListener::bind(&socket)
        .map_err(|error| format!("could not listen at {}: {error}", socket.display()))?;
    let made = std::fs::symlink_metadata(&socket)
        .map(|made| (made.dev(), made.ino()))
        .map_err(|error| format!("could not look at {}: {error}", socket.display()))?;
    files.socket = Some((socket.clone(), made));
    std::fs::set_permissions(&socket, std::fs::Permissions::from_mode(0o600))
        .map_err(|error| format!("could not make {} private: {error}", socket.display()))?;
    let stop = CancellationToken::new();
    tokio::spawn(accept(listener, Arc::new(key), server, stop.clone()));
    Ok(Listening {
        stop,
        _files: files,
    })
}

/// Listens on a pipe of a new name, which it writes in `dir`, whose lock is
/// `locked`. The first instance is this server's own, or it does not
/// listen; no other computer's relay is taken.
#[cfg(windows)]
fn listen_on_pipe(
    dir: &Path,
    locked: std::fs::File,
    server: Server<AppGateway>,
) -> Result<Listening, String> {
    // The name an ended start left goes before the new key is written: a
    // relay that read it would find, at best, a pipe another account made
    // of that name since.
    let pipe_file = dir.join(endpoint::PIPE);
    match std::fs::remove_file(&pipe_file) {
        Err(error) if error.kind() != std::io::ErrorKind::NotFound => {
            return Err(format!(
                "could not remove an old {}: {error}",
                pipe_file.display()
            ));
        }
        _ => {}
    }
    let (mut files, key) = new_key(dir, locked)?;
    let name = endpoint::new_pipe();
    let first = ServerOptions::new()
        .first_pipe_instance(true)
        .reject_remote_clients(true)
        .create(&name)
        .map_err(|error| format!("could not listen at {name}: {error}"))?;
    // Named once it is there: a relay that reads the name finds the pipe.
    crate::util::write_atomic(&pipe_file, name.as_bytes())
        .map_err(|error| format!("could not write {}: {error}", pipe_file.display()))?;
    files.pipe = Some((pipe_file, name.clone()));
    let stop = CancellationToken::new();
    tokio::spawn(accept(
        Pipe {
            waiting: first,
            name,
        },
        Arc::new(key),
        server,
        stop.clone(),
    ));
    Ok(Listening {
        stop,
        _files: files,
    })
}

/// Where relays connect: a socket's listener, or a pipe's instance that
/// waits for one.
trait Endpoint: Send + 'static {
    type Stream: AsyncRead + AsyncWrite + Unpin + Send + 'static;
    /// The next relay that connects.
    async fn next(&mut self) -> std::io::Result<Self::Stream>;
}

#[cfg(unix)]
impl Endpoint for UnixListener {
    type Stream = tokio::net::UnixStream;

    async fn next(&mut self) -> std::io::Result<Self::Stream> {
        self.accept().await.map(|(stream, _)| stream)
    }
}

/// The pipe: the instance a relay connects to next, and its name, which
/// the next instance takes.
#[cfg(windows)]
struct Pipe {
    waiting: NamedPipeServer,
    name: String,
}

#[cfg(windows)]
impl Endpoint for Pipe {
    type Stream = NamedPipeServer;

    /// The instance a relay connected to; another waits in its place. One
    /// that could not be connected is put aside for another.
    async fn next(&mut self) -> std::io::Result<Self::Stream> {
        let connected = self.waiting.connect().await;
        let next = ServerOptions::new()
            .reject_remote_clients(true)
            .create(&self.name)?;
        let taken = std::mem::replace(&mut self.waiting, next);
        connected.map(|()| taken)
    }
}

/// Takes each relay that connects, until `stop`.
async fn accept<E: Endpoint>(
    mut endpoint: E,
    key: Arc<String>,
    server: Server<AppGateway>,
    stop: CancellationToken,
) {
    let open = Arc::new(AtomicUsize::new(0));
    let greetings = Arc::new(Semaphore::new(MAX_GREETINGS));
    loop {
        // A place among the greetings before the connection is taken.
        let greeting = tokio::select! {
            biased;
            () = stop.cancelled() => return,
            permit = Arc::clone(&greetings).acquire_owned() => match permit {
                Ok(permit) => permit,
                Err(_) => return,
            },
        };
        let stream = tokio::select! {
            biased;
            () = stop.cancelled() => return,
            accepted = endpoint.next() => match accepted {
                Ok(stream) => stream,
                Err(error) => {
                    log::warn!("the agent server could not take a connection: {error}");
                    // Out of descriptors, most likely: not again at once.
                    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                    continue;
                }
            },
        };
        let key = Arc::clone(&key);
        let server = server.clone();
        let open = Arc::clone(&open);
        let stop = stop.clone();
        tokio::spawn(async move {
            greet(stream, &key, &server, &open, greeting, &stop).await;
        });
    }
}

/// One of the sessions counted against [`MAX_SESSIONS`], while it is open.
struct Held(Arc<AtomicUsize>, bool);

impl Held {
    fn take(open: &Arc<AtomicUsize>) -> Self {
        let before = open.fetch_add(1, Ordering::SeqCst);
        Self(Arc::clone(open), before < MAX_SESSIONS)
    }
}

impl Drop for Held {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::SeqCst);
    }
}

/// Reads the relay's [`Hello`], answers it, and serves the session. A stop
/// ends what comes before the session; the session is ended by the
/// server's own stop, which lets its calls end and go into the log. Nothing
/// the relay sent goes into the app's log: any program of the user's can
/// send it.
async fn greet<S: AsyncRead + AsyncWrite + Unpin + Send + 'static>(
    mut stream: S,
    key: &str,
    server: &Server<AppGateway>,
    open: &Arc<AtomicUsize>,
    greeting: OwnedSemaphorePermit,
    stop: &CancellationToken,
) {
    let refuse = async |stream: &mut S, said: &str| {
        let line = endpoint::line_of(&Answer::Refused(said.to_owned()));
        let _ = stream.write_all(&line).await;
        let _ = stream.shutdown().await;
    };
    // The relay's line, within its time and until a stop.
    let line = async |stream: &mut S| {
        tokio::select! {
            biased;
            () = stop.cancelled() => None,
            read = tokio::time::timeout(
                endpoint::LINE_TIME,
                endpoint::read_line(stream, endpoint::MAX_LINE),
            ) => Some(read),
        }
    };
    let Some(read) = line(&mut stream).await else {
        return refuse(&mut stream, endpoint::NOT_RUNNING).await;
    };
    let hello = match read {
        Ok(Ok(Some(line))) => serde_json::from_slice::<Hello>(&line).ok(),
        // Closed before a byte: nothing was asked, and nothing is said.
        Ok(Ok(None)) => return,
        _ => None,
    };
    let Some(hello) = hello else {
        log::warn!("a connection to the agent server said nothing it could read");
        return refuse(&mut stream, endpoint::UNREAD).await;
    };
    // The app's proof first: a relay tells nothing to what cannot give it.
    let challenge = endpoint::new_nonce();
    let prove = Answer::Prove {
        proof: endpoint::prove(key, Side::App, &hello.challenge),
        challenge: challenge.clone(),
    };
    if stream.write_all(&endpoint::line_of(&prove)).await.is_err() {
        return;
    }
    let Some(read) = line(&mut stream).await else {
        return refuse(&mut stream, endpoint::NOT_RUNNING).await;
    };
    drop(greeting);
    let reply = match read {
        Ok(Ok(Some(line))) => serde_json::from_slice::<Reply>(&line).ok(),
        _ => None,
    };
    let reply = reply.filter(|reply| {
        endpoint::same_key(&reply.proof, &endpoint::prove(key, Side::Relay, &challenge))
    });
    let Some(reply) = reply else {
        log::warn!("a relay did not prove it holds the agent server's key");
        return refuse(
            &mut stream,
            "The relay's key is not the one Tabletist made: start the agent's MCP server again.",
        )
        .await;
    };
    // Counted once it has the key: who has none takes no agent's place.
    let held = Held::take(open);
    if !held.1 {
        log::warn!("an agent was refused: {MAX_SESSIONS} are connected");
        return refuse(&mut stream, "Too many agents are connected to Tabletist.").await;
    }
    if stop.is_cancelled() {
        return refuse(&mut stream, endpoint::NOT_RUNNING).await;
    }
    if stream
        .write_all(&endpoint::line_of(&Answer::Ok))
        .await
        .is_err()
    {
        return;
    }
    let origin = Origin {
        transport: Transport::Stdio,
        project: reply.cwd.as_deref().and_then(endpoint::project),
    };
    // Boxed: the session's future is deep, too deep for the compiler to
    // lay out inside this one.
    Box::pin(server.serve(stream, origin)).await;
    drop(held);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::secrets::Keyring;

    fn server(dir: &Path) -> Server<AppGateway> {
        let log = tabletist_mcp::Log::open(&dir.join("agents.sqlite3"))
            .unwrap()
            .log;
        Server::new(AppGateway::new(Keyring::memory()), log, "test")
    }

    /// What a relay connects with.
    #[cfg(unix)]
    type Relay = tokio::net::UnixStream;
    #[cfg(windows)]
    type Relay = tokio::net::windows::named_pipe::NamedPipeClient;

    /// Connects to the endpoint in `dir` as a relay does.
    #[cfg(unix)]
    async fn connect(dir: &Path) -> Relay {
        tokio::net::UnixStream::connect(dir.join(endpoint::SOCKET))
            .await
            .unwrap()
    }

    /// Connects to the endpoint in `dir` as a relay does: by the name the
    /// server wrote, again while its instances are busy.
    #[cfg(windows)]
    async fn connect(dir: &Path) -> Relay {
        let name = std::fs::read_to_string(dir.join(endpoint::PIPE)).unwrap();
        assert!(endpoint::is_pipe(&name), "{name}");
        let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(5);
        loop {
            match tokio::net::windows::named_pipe::ClientOptions::new().open(&name) {
                Ok(client) => return client,
                Err(error) if tokio::time::Instant::now() < deadline => {
                    let _ = error;
                    tokio::time::sleep(std::time::Duration::from_millis(10)).await;
                }
                Err(error) => panic!("{error}"),
            }
        }
    }

    /// The app's next line.
    async fn answer(stream: &mut Relay) -> Answer {
        let line = endpoint::read_line(stream, endpoint::MAX_LINE)
            .await
            .unwrap()
            .unwrap();
        serde_json::from_slice(&line).unwrap()
    }

    /// Says `line` to the endpoint in `dir`, and reads the app's answer.
    async fn say_line(dir: &Path, line: &[u8]) -> (Relay, Answer) {
        let mut stream = connect(dir).await;
        stream.write_all(line).await.unwrap();
        let answer = answer(&mut stream).await;
        (stream, answer)
    }

    /// Greets the endpoint in `dir` as a relay does, as one that holds
    /// `key`, and reads the app's last answer. An app that lets it in has
    /// proved it holds that key.
    async fn say(dir: &Path, key: &str) -> (Relay, Answer) {
        let challenge = endpoint::new_nonce();
        let hello = Hello {
            version: "test".into(),
            challenge: challenge.clone(),
        };
        let (mut stream, first) = say_line(dir, &endpoint::line_of(&hello)).await;
        let Answer::Prove {
            proof,
            challenge: theirs,
        } = first
        else {
            return (stream, first);
        };
        let proven = endpoint::same_key(&proof, &endpoint::prove(key, Side::App, &challenge));
        let reply = Reply {
            proof: endpoint::prove(key, Side::Relay, &theirs),
            cwd: Some("/home/ada/src/bookshop".into()),
        };
        stream.write_all(&endpoint::line_of(&reply)).await.unwrap();
        let last = answer(&mut stream).await;
        if last == Answer::Ok {
            assert!(proven, "the app let a relay in without proving its key");
        }
        (stream, last)
    }

    #[cfg(unix)]
    fn mode(path: &Path) -> u32 {
        std::fs::symlink_metadata(path)
            .unwrap()
            .permissions()
            .mode()
            & 0o777
    }

    #[tokio::test]
    async fn a_relay_with_the_key_is_served_and_one_without_it_is_not() {
        let dir = tempfile::tempdir().unwrap();
        let at = dir.path().join("run");
        let server = server(dir.path());
        let listening = listen(&at, server.clone()).unwrap();
        // Private to the user, the folder and what is in it.
        #[cfg(unix)]
        {
            assert_eq!(mode(&at), 0o700);
            assert_eq!(mode(&at.join(endpoint::SOCKET)), 0o600);
            assert_eq!(mode(&at.join(endpoint::KEY)), 0o600);
        }
        let key = std::fs::read_to_string(at.join(endpoint::KEY)).unwrap();
        let (stream, answer) = say(&at, &key).await;
        assert_eq!(answer, Answer::Ok);
        // From then on the stream is the session's, named for its project.
        let (read, mut write) = tokio::io::split(stream);
        let initialize = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 0,
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-11-25",
                "capabilities": {},
                "clientInfo": { "name": "claude-code", "version": "1" },
            },
        });
        write
            .write_all(&endpoint::line_of(&initialize))
            .await
            .unwrap();
        let mut lines = tokio::io::AsyncBufReadExt::lines(tokio::io::BufReader::new(read));
        let answered: serde_json::Value =
            serde_json::from_str(&lines.next_line().await.unwrap().unwrap()).unwrap();
        assert!(answered["result"].is_object(), "{answered}");
        let sessions = server.open_sessions();
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].project.as_deref(), Some("bookshop"));
        // A key that is not the server's, and a line that is no hello.
        let (_, answer) = say(&at, "not-the-key").await;
        assert!(matches!(answer, Answer::Refused(_)), "{answer:?}");
        let (_, answer) = say_line(&at, b"hello\n").await;
        assert!(matches!(answer, Answer::Refused(_)), "{answer:?}");
        assert_eq!(server.open_sessions().len(), 1);
        // Stopped: the session ends, and the socket, the pipe's name and
        // the key are gone.
        server.stop();
        drop(listening);
        assert_eq!(lines.next_line().await.unwrap(), None);
        for file in [endpoint::SOCKET, endpoint::PIPE, endpoint::KEY] {
            assert!(!at.join(file).exists(), "{file}");
        }
    }

    #[tokio::test]
    async fn a_second_start_stops_at_the_lock_and_a_socket_left_by_an_ended_one_is_old() {
        let dir = tempfile::tempdir().unwrap();
        let at = dir.path().join("run");
        assert!(!served_by_another(&at));
        let first = listen(&at, server(dir.path())).unwrap();
        let key = std::fs::read_to_string(at.join(endpoint::KEY)).unwrap();
        assert!(served_by_another(&at));
        let said = listen(&at, server(dir.path())).err().unwrap();
        assert_eq!(said, endpoint::ANOTHER_SERVES);
        // The first is as it was, its key too: the second stopped before it.
        assert_eq!(
            std::fs::read_to_string(at.join(endpoint::KEY)).unwrap(),
            key
        );
        let (_, answer) = say(&at, &key).await;
        assert_eq!(answer, Answer::Ok);
        drop(first);
        assert!(!served_by_another(&at));
        // A socket left by a Tabletist that ended: nothing answers it.
        #[cfg(unix)]
        drop(std::os::unix::net::UnixListener::bind(at.join(endpoint::SOCKET)).unwrap());
        let _second = listen(&at, server(dir.path())).unwrap();
        let key = std::fs::read_to_string(at.join(endpoint::KEY)).unwrap();
        let (_, answer) = say(&at, &key).await;
        assert_eq!(answer, Answer::Ok);
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn what_is_not_a_socket_is_not_removed_nor_what_another_put_in_its_place() {
        let dir = tempfile::tempdir().unwrap();
        let at = dir.path().join("run");
        std::fs::create_dir_all(&at).unwrap();
        std::fs::write(at.join(endpoint::SOCKET), "mine").unwrap();
        let said = listen(&at, server(dir.path())).err().unwrap();
        assert!(said.contains("in the way"), "{said}");
        assert_eq!(
            std::fs::read_to_string(at.join(endpoint::SOCKET)).unwrap(),
            "mine"
        );
        std::fs::remove_file(at.join(endpoint::SOCKET)).unwrap();
        let listening = listen(&at, server(dir.path())).unwrap();
        // Another socket where it was: the stop leaves it there.
        std::fs::remove_file(at.join(endpoint::SOCKET)).unwrap();
        let other = std::os::unix::net::UnixListener::bind(at.join(endpoint::SOCKET)).unwrap();
        drop(listening);
        assert!(at.join(endpoint::SOCKET).exists());
        drop(other);
    }

    #[tokio::test]
    async fn connections_that_say_nothing_take_no_agents_place() {
        let dir = tempfile::tempdir().unwrap();
        let at = dir.path().join("run");
        let _listening = listen(&at, server(dir.path())).unwrap();
        let mut silent = Vec::new();
        for _ in 0..MAX_SESSIONS + 8 {
            silent.push(connect(&at).await);
        }
        let key = std::fs::read_to_string(at.join(endpoint::KEY)).unwrap();
        let (_, answer) = say(&at, &key).await;
        assert_eq!(answer, Answer::Ok);
    }

    #[tokio::test]
    async fn a_relay_greeting_as_the_server_stops_is_told_it_is_not_running() {
        let dir = tempfile::tempdir().unwrap();
        let at = dir.path().join("run");
        let listening = listen(&at, server(dir.path())).unwrap();
        let mut stream = connect(&at).await;
        // Taken, and waiting for its hello.
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        drop(listening);
        let line = endpoint::read_line(&mut stream, endpoint::MAX_LINE)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            serde_json::from_slice::<Answer>(&line).unwrap(),
            Answer::Refused(endpoint::NOT_RUNNING.into())
        );
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn a_key_left_by_an_ended_start_is_replaced_before_anyone_can_connect() {
        let dir = tempfile::tempdir().unwrap();
        let at = dir.path().join("run");
        std::fs::create_dir_all(&at).unwrap();
        std::fs::write(at.join(endpoint::KEY), "an ended start's").unwrap();
        // A socket that is in the way: nothing listens, and the key that
        // is there stays the one that was.
        std::fs::create_dir(at.join(endpoint::SOCKET)).unwrap();
        assert!(listen(&at, server(dir.path())).is_err());
        assert_eq!(
            std::fs::read_to_string(at.join(endpoint::KEY)).unwrap(),
            "an ended start's"
        );
        std::fs::remove_dir(at.join(endpoint::SOCKET)).unwrap();
        let listening = listen(&at, server(dir.path())).unwrap();
        let key = std::fs::read_to_string(at.join(endpoint::KEY)).unwrap();
        assert_ne!(key, "an ended start's");
        // A key a later start wrote is that start's: the stop leaves it.
        std::fs::write(at.join(endpoint::KEY), "a later start's").unwrap();
        drop(listening);
        assert_eq!(
            std::fs::read_to_string(at.join(endpoint::KEY)).unwrap(),
            "a later start's"
        );
    }

    #[cfg(windows)]
    #[tokio::test]
    async fn each_start_names_a_new_pipe_and_a_stop_leaves_a_later_starts_name() {
        let dir = tempfile::tempdir().unwrap();
        let at = dir.path().join("run");
        std::fs::create_dir_all(&at).unwrap();
        // What an ended start left: replaced before anyone can connect.
        std::fs::write(at.join(endpoint::PIPE), "an ended start's").unwrap();
        std::fs::write(at.join(endpoint::KEY), "an ended start's").unwrap();
        let first = listen(&at, server(dir.path())).unwrap();
        let named = std::fs::read_to_string(at.join(endpoint::PIPE)).unwrap();
        assert!(endpoint::is_pipe(&named), "{named}");
        let key = std::fs::read_to_string(at.join(endpoint::KEY)).unwrap();
        let (_, answer) = say(&at, &key).await;
        assert_eq!(answer, Answer::Ok);
        drop(first);
        assert!(!at.join(endpoint::PIPE).exists());
        // The next start's pipe has a name of its own.
        let second = listen(&at, server(dir.path())).unwrap();
        let renamed = std::fs::read_to_string(at.join(endpoint::PIPE)).unwrap();
        assert_ne!(renamed, named);
        // The pipe of a server that stopped goes with it, once the task that
        // held its waiting instance has ended: a moment after the stop.
        let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(2);
        while tokio::net::windows::named_pipe::ClientOptions::new()
            .open(&named)
            .is_ok()
        {
            assert!(
                tokio::time::Instant::now() < deadline,
                "the old pipe is still there"
            );
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
        // A name a later start wrote is that start's: the stop leaves it.
        std::fs::write(at.join(endpoint::PIPE), "a later start's").unwrap();
        drop(second);
        assert_eq!(
            std::fs::read_to_string(at.join(endpoint::PIPE)).unwrap(),
            "a later start's"
        );
    }

    #[test]
    fn a_session_past_the_most_is_counted_and_refused() {
        let open = Arc::new(AtomicUsize::new(0));
        let held: Vec<Held> = (0..MAX_SESSIONS).map(|_| Held::take(&open)).collect();
        assert!(held.iter().all(|held| held.1));
        let one_more = Held::take(&open);
        assert!(!one_more.1);
        drop(one_more);
        drop(held);
        assert_eq!(open.load(Ordering::SeqCst), 0);
        assert!(Held::take(&open).1);
    }
}
