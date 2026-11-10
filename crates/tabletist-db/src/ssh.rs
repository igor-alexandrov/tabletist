//! SSH tunnels: connect (directly, or through `~/.ssh/config`'s
//! ProxyCommand or ProxyJump), check the host key, authenticate, and
//! forward a local port to the database through `direct-tcpip` channels.

mod proxy;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

use russh::client;
use russh::keys::agent::AgentIdentity;
use russh::keys::agent::client::{AgentClient, AgentStream};
use russh::keys::{HashAlg, PrivateKey, PrivateKeyWithHashAlg, PublicKeyOrCertificate};
use serde::{Deserialize, Serialize};

use crate::ssh_config::{AgentSocket, HostConfig};
use crate::{Error, Result, Secrets, SshAuth, SshSpec, SshStage};

const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);

/// How long a session through a proxy waits for the server's hello, and
/// then to log in. The hello gets longer than `CONNECT_TIMEOUT`: a proxy
/// makes its own connection first, and a ProxyJump also logs in to the
/// jump host. The login gets minutes: an agent can ask the person before
/// it signs (1Password, Secretive). Without a limit, a proxy that stops
/// passing anything on after the hello would hold the login, and the proxy,
/// for as long as the app runs: russh sends no keepalives before a login.
#[derive(Debug, Clone, Copy)]
struct ProxyWaits {
    hello: Duration,
    login: Duration,
}

const PROXY_WAITS: ProxyWaits = ProxyWaits {
    hello: Duration::from_secs(30),
    login: Duration::from_secs(120),
};

/// Trusted SSH host keys: `host:port` to a `SHA256:` fingerprint.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct HostKeys {
    pub hosts: BTreeMap<String, String>,
}

impl HostKeys {
    fn key(host: &str, port: u16) -> String {
        format!("{}:{port}", host.to_lowercase())
    }

    pub fn fingerprint(&self, host: &str, port: u16) -> Option<&str> {
        self.hosts.get(&Self::key(host, port)).map(String::as_str)
    }

    pub fn trust(&mut self, host: &str, port: u16, fingerprint: &str) {
        self.hosts
            .insert(Self::key(host, port), fingerprint.to_owned());
    }
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Verdict {
    Trusted,
    Unknown,
    Changed,
}

pub(crate) fn verdict(known: Option<&str>, seen: &str) -> Verdict {
    match known {
        None => Verdict::Unknown,
        Some(known) if known == seen => Verdict::Trusted,
        Some(_) => Verdict::Changed,
    }
}

fn ssh_error(stage: SshStage, message: impl Into<String>) -> Error {
    Error::Ssh {
        stage,
        message: message.into(),
    }
}

/// Where a tunnel goes and as whom, once `~/.ssh/config` has had its say.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Endpoint {
    pub host: String,
    pub port: u16,
    pub user: String,
    /// The key file, for the key file method.
    pub key: Option<PathBuf>,
    /// The config's ProxyCommand or ProxyJump; `None` connects directly.
    pub via: Option<proxy::Via>,
}

impl Endpoint {
    /// The server and the proxy it is reached through, as an error names
    /// them: `db:22 through ProxyJump gateway`.
    fn reach(&self) -> String {
        match &self.via {
            None => format!("{}:{}", self.host, self.port),
            Some(via) => format!("{}:{} through {}", self.host, self.port, via.describe()),
        }
    }
}

/// `ssh`'s host, port, user and key file as `ssh -p 2222 -l me bastion`
/// takes them: a typed value wins, then the config's, then the default
/// (port 22, the `login` name). The host is the config's HostName, else the
/// alias itself. A ProxyCommand's tokens stand for those values.
pub(crate) fn endpoint(
    ssh: &SshSpec,
    config: &HostConfig,
    login: Option<&str>,
) -> Result<Endpoint> {
    let alias = ssh.host.trim();
    if alias.is_empty() {
        return Err(Error::InvalidSpec("enter the SSH host".into()));
    }
    let user = Some(ssh.user.trim())
        .filter(|user| !user.is_empty())
        .or(config.user.as_deref())
        .or(login)
        .ok_or_else(|| Error::InvalidSpec("enter the SSH user".into()))?;
    let key = match &ssh.auth {
        SshAuth::KeyFile { path } if path.as_os_str().is_empty() => Some(
            config
                .identity_file
                .clone()
                .ok_or_else(|| Error::InvalidSpec("choose a key file for SSH".into()))?,
        ),
        SshAuth::KeyFile { path } => Some(path.clone()),
        SshAuth::Password | SshAuth::Agent => None,
    };
    let host = config.host_name.clone().unwrap_or_else(|| alias.to_owned());
    let port = ssh.port.or(config.port).unwrap_or(22);
    let tokens = proxy::Tokens {
        host: &host,
        port,
        user,
        alias,
    };
    let via = proxy::Via::new(config.proxy.as_ref(), &tokens)
        .map_err(|error| ssh_error(SshStage::Connect, format!("~/.ssh/config: {error}")))?;
    Ok(Endpoint {
        host,
        port,
        user: user.to_owned(),
        key,
        via,
    })
}

/// Accepts the server only when its key matches the trusted one; records
/// what it saw so a refusal can name the fingerprint.
struct Client {
    known: Option<String>,
    seen: Arc<Mutex<Option<String>>>,
}

impl client::Handler for Client {
    type Error = russh::Error;

    async fn check_server_key(
        &mut self,
        key: &PublicKeyOrCertificate,
    ) -> std::result::Result<bool, Self::Error> {
        let fingerprint = match key {
            PublicKeyOrCertificate::PublicKey { key, .. } => key.fingerprint(HashAlg::Sha256),
            PublicKeyOrCertificate::Certificate(cert) => {
                cert.public_key().fingerprint(HashAlg::Sha256)
            }
        }
        .to_string();
        let trusted = verdict(self.known.as_deref(), &fingerprint) == Verdict::Trusted;
        *self.seen.lock().unwrap_or_else(PoisonError::into_inner) = Some(fingerprint);
        Ok(trusted)
    }
}

/// A private key file, decrypted with `passphrase` when it has one.
pub(crate) fn load_key(path: &Path, passphrase: Option<&str>) -> Result<PrivateKey> {
    use russh::keys::Error as KeyError;
    // PEM and PKCS#8 keys fail with format errors rather than
    // `KeyIsEncrypted`, so their own "encrypted" markers are checked too.
    let marked_encrypted = || {
        std::fs::read_to_string(path).is_ok_and(|text| {
            text.contains("BEGIN ENCRYPTED PRIVATE KEY") || text.contains("Proc-Type: 4,ENCRYPTED")
        })
    };
    russh::keys::load_secret_key(path, passphrase).map_err(|error| match error {
        KeyError::KeyIsEncrypted => {
            ssh_error(SshStage::Secret, "the key file needs its passphrase")
        }
        KeyError::IO(io) => ssh_error(
            SshStage::Auth,
            format!("could not read {}: {io}", path.display()),
        ),
        _ if passphrase.is_some() => {
            ssh_error(SshStage::Secret, "the key file's passphrase is wrong")
        }
        _ if marked_encrypted() => ssh_error(SshStage::Secret, "the key file needs its passphrase"),
        other => ssh_error(
            SshStage::Auth,
            format!("could not read the key file {}: {other}", path.display()),
        ),
    })
}

type Agent = AgentClient<Box<dyn AgentStream + Send + Unpin + 'static>>;

/// The agent for `host`: the one `~/.ssh/config` names (`IdentityAgent`,
/// as OpenSSH reads it), else the environment's.
async fn agent(host: &str) -> std::result::Result<Agent, String> {
    match crate::ssh_config::resolve(host).identity_agent {
        Some(AgentSocket::Off) => {
            Err("~/.ssh/config turns the agent off for this host (IdentityAgent none)".into())
        }
        Some(AgentSocket::Path(path)) => named_agent(&path)
            .await
            .map_err(|error| format!("{} (from ~/.ssh/config): {error}", path.display())),
        Some(AgentSocket::Environment) | None => default_agent().await,
    }
}

#[cfg(unix)]
async fn named_agent(path: &Path) -> std::result::Result<Agent, String> {
    AgentClient::connect_uds(path)
        .await
        .map(AgentClient::dynamic)
        .map_err(|error| error.to_string())
}

#[cfg(windows)]
async fn named_agent(path: &Path) -> std::result::Result<Agent, String> {
    AgentClient::connect_named_pipe(path)
        .await
        .map(AgentClient::dynamic)
        .map_err(|error| error.to_string())
}

#[cfg(unix)]
async fn default_agent() -> std::result::Result<Agent, String> {
    AgentClient::connect_env()
        .await
        .map(AgentClient::dynamic)
        .map_err(|error| error.to_string())
}

#[cfg(windows)]
async fn default_agent() -> std::result::Result<Agent, String> {
    match AgentClient::connect_named_pipe(r"\\.\pipe\openssh-ssh-agent").await {
        Ok(agent) => Ok(agent.dynamic()),
        Err(_) => AgentClient::connect_pageant()
            .await
            .map(AgentClient::dynamic)
            .map_err(|error| error.to_string()),
    }
}

async fn authenticate(
    handle: &mut client::Handle<Client>,
    ssh: &SshSpec,
    endpoint: &Endpoint,
    secrets: &Secrets,
) -> Result<()> {
    let auth_error = |error: russh::Error| ssh_error(SshStage::Auth, error.to_string());
    let rsa_hash = handle
        .best_supported_rsa_hash()
        .await
        .ok()
        .flatten()
        .flatten();
    let accepted = match &ssh.auth {
        SshAuth::Password => {
            let Some(password) = secrets.ssh_password.as_deref().filter(|p| !p.is_empty()) else {
                return Err(ssh_error(SshStage::Secret, "enter the SSH password"));
            };
            let result = handle
                .authenticate_password(&endpoint.user, password)
                .await
                .map_err(auth_error)?;
            if !result.success() {
                return Err(ssh_error(
                    SshStage::Secret,
                    "the server refused the SSH password",
                ));
            }
            true
        }
        SshAuth::KeyFile { .. } => {
            let path = endpoint.key.as_deref().unwrap_or(Path::new(""));
            let passphrase = secrets.ssh_passphrase.as_deref().filter(|p| !p.is_empty());
            let key = load_key(path, passphrase)?;
            handle
                .authenticate_publickey(
                    &endpoint.user,
                    PrivateKeyWithHashAlg::new(Arc::new(key), rsa_hash),
                )
                .await
                .map_err(auth_error)?
                .success()
        }
        SshAuth::Agent => {
            let mut agent = agent(ssh.host.trim()).await.map_err(|error| {
                ssh_error(
                    SshStage::Auth,
                    format!("could not reach the SSH agent: {error}"),
                )
            })?;
            let identities = agent
                .request_identities()
                .await
                .map_err(|error| ssh_error(SshStage::Auth, error.to_string()))?;
            let mut accepted = false;
            for identity in identities {
                let AgentIdentity::PublicKey { key, .. } = identity else {
                    continue;
                };
                let hash = matches!(key.algorithm(), russh::keys::Algorithm::Rsa { .. })
                    .then_some(rsa_hash)
                    .flatten();
                let result = handle
                    .authenticate_publickey_with(&endpoint.user, key, hash, &mut agent)
                    .await
                    .map_err(|error| ssh_error(SshStage::Auth, error.to_string()))?;
                if result.success() {
                    accepted = true;
                    break;
                }
            }
            accepted
        }
    };
    if accepted {
        Ok(())
    } else {
        Err(ssh_error(
            SshStage::Auth,
            format!("the server did not accept any key for {}", endpoint.user),
        ))
    }
}

/// Stops a failed session's proxy, and returns what the proxy wrote to its
/// standard error, for the error: ` (it said: Permission denied)`, or an
/// empty string.
async fn stop(running: &mut Option<proxy::Running>, said: Option<proxy::Said>) -> String {
    drop(running.take());
    let words = match said {
        Some(said) => said.words().await,
        None => String::new(),
    };
    if words.is_empty() {
        words
    } else {
        format!(" (it said: {words})")
    }
}

/// A local port forwarded to the database through an SSH session. Dropping
/// it stops accepting; the session ends when the last forwarded socket
/// closes.
pub(crate) struct Tunnel {
    pub port: u16,
    forward_error: Arc<Mutex<Option<Error>>>,
    task: tokio::task::JoinHandle<()>,
    /// The ProxyCommand or ProxyJump the session goes through, stopped
    /// with the tunnel.
    _proxy: Option<proxy::Running>,
}

impl Drop for Tunnel {
    fn drop(&mut self) {
        self.task.abort();
    }
}

impl Tunnel {
    pub async fn open(
        ssh: &SshSpec,
        target_host: &str,
        target_port: u16,
        secrets: &Secrets,
        host_keys: &HostKeys,
    ) -> Result<Self> {
        let host_config = crate::ssh_config::resolve(ssh.host.trim());
        let login = crate::ssh_config::login();
        let endpoint = endpoint(ssh, &host_config, login.as_deref())?;
        let target = (target_host, target_port);
        Self::open_to(&endpoint, ssh, target, secrets, host_keys, PROXY_WAITS).await
    }

    /// [`Tunnel::open`] with the endpoint already read from
    /// `~/.ssh/config`. A session through a proxy waits as `waits` says.
    async fn open_to(
        endpoint: &Endpoint,
        ssh: &SshSpec,
        (target_host, target_port): (&str, u16),
        secrets: &Secrets,
        host_keys: &HostKeys,
        waits: ProxyWaits,
    ) -> Result<Self> {
        let seen = Arc::new(Mutex::new(None));
        let client = Client {
            known: host_keys
                .fingerprint(&endpoint.host, endpoint.port)
                .map(str::to_owned),
            seen: Arc::clone(&seen),
        };
        let config = Arc::new(client::Config {
            nodelay: true,
            inactivity_timeout: None,
            keepalive_interval: Some(Duration::from_secs(30)),
            ..Default::default()
        });
        let (connected, said, mut running, timeout) = match &endpoint.via {
            None => {
                let address = (endpoint.host.clone(), endpoint.port);
                let connected =
                    tokio::time::timeout(CONNECT_TIMEOUT, client::connect(config, address, client))
                        .await;
                (connected, None, None, CONNECT_TIMEOUT)
            }
            Some(via) => {
                let (stream, said, running) = via
                    .spawn()
                    .map_err(|error| ssh_error(SshStage::Connect, error))?;
                let connected = tokio::time::timeout(
                    waits.hello,
                    client::connect_stream(config, stream, client),
                )
                .await;
                (connected, Some(said), Some(running), waits.hello)
            }
        };
        let connected = match connected {
            Ok(connected) => connected,
            Err(_) => {
                return Err(ssh_error(
                    SshStage::Connect,
                    format!(
                        "no answer from {} within {} seconds{}",
                        endpoint.reach(),
                        timeout.as_secs(),
                        stop(&mut running, said).await
                    ),
                ));
            }
        };
        let mut handle = match connected {
            Ok(handle) => handle,
            Err(russh::Error::UnknownKey) => {
                let fingerprint = seen
                    .lock()
                    .unwrap_or_else(PoisonError::into_inner)
                    .clone()
                    .unwrap_or_default();
                return Err(
                    if host_keys
                        .fingerprint(&endpoint.host, endpoint.port)
                        .is_some()
                    {
                        ssh_error(
                            SshStage::HostKeyMismatch {
                                host: endpoint.host.clone(),
                                port: endpoint.port,
                                fingerprint,
                            },
                            "the host key changed since you trusted it, which can mean someone \
                         is intercepting the connection; if the server's key really changed, \
                         remove its line from known_hosts.json",
                        )
                    } else {
                        ssh_error(
                            SshStage::HostKeyUnknown {
                                host: endpoint.host.clone(),
                                port: endpoint.port,
                                fingerprint,
                            },
                            format!("{} is not a trusted host yet", endpoint.host),
                        )
                    },
                );
            }
            Err(error) => {
                return Err(ssh_error(
                    SshStage::Connect,
                    format!(
                        "could not reach {}: {error}{}",
                        endpoint.reach(),
                        stop(&mut running, said).await
                    ),
                ));
            }
        };
        let login = authenticate(&mut handle, ssh, endpoint, secrets);
        if endpoint.via.is_none() {
            login.await?;
        } else if let Ok(logged_in) = tokio::time::timeout(waits.login, login).await {
            logged_in?;
        } else {
            return Err(ssh_error(
                SshStage::Auth,
                format!(
                    "no answer from {} to the login within {} seconds{}",
                    endpoint.reach(),
                    waits.login.as_secs(),
                    stop(&mut running, said).await
                ),
            ));
        }
        let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0))
            .await
            .map_err(|error| ssh_error(SshStage::Forward, error.to_string()))?;
        let port = listener
            .local_addr()
            .map_err(|error| ssh_error(SshStage::Forward, error.to_string()))?
            .port();
        let forward_error = Arc::new(Mutex::new(None));
        let slot = Arc::clone(&forward_error);
        let handle = Arc::new(handle);
        let target = (target_host.to_owned(), target_port);
        let task = tokio::spawn(async move {
            while let Ok((mut socket, peer)) = listener.accept().await {
                let handle = Arc::clone(&handle);
                let slot = Arc::clone(&slot);
                let (host, port) = target.clone();
                tokio::spawn(async move {
                    let channel = handle
                        .channel_open_direct_tcpip(
                            host.clone(),
                            u32::from(port),
                            peer.ip().to_string(),
                            u32::from(peer.port()),
                        )
                        .await;
                    match channel {
                        Ok(channel) => {
                            let mut stream = channel.into_stream();
                            let _ = tokio::io::copy_bidirectional(&mut socket, &mut stream).await;
                        }
                        Err(error) => {
                            *slot.lock().unwrap_or_else(PoisonError::into_inner) = Some(ssh_error(
                                SshStage::Forward,
                                format!("the SSH server could not reach {host}:{port} ({error})"),
                            ));
                        }
                    }
                });
            }
        });
        Ok(Self {
            port,
            forward_error,
            task,
            _proxy: running,
        })
    }

    /// Why a forwarded connection failed, if one did: a clearer error than
    /// the driver's "connection closed".
    pub fn forward_error(&self) -> Option<Error> {
        self.forward_error
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ssh_config::{HostConfig, Proxy};

    fn key(name: &str) -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/ssh")
            .join(name)
    }

    #[test]
    fn host_keys_are_looked_up_by_lowercased_host_and_port() {
        let mut keys = HostKeys::default();
        keys.trust("DB.example.com", 22, "SHA256:abc");
        assert_eq!(keys.fingerprint("db.example.com", 22), Some("SHA256:abc"));
        assert_eq!(keys.fingerprint("db.example.com", 2222), None);
        let json = serde_json::to_string(&keys).unwrap();
        assert_eq!(serde_json::from_str::<HostKeys>(&json).unwrap(), keys);
    }

    #[test]
    fn a_seen_key_is_trusted_unknown_or_changed() {
        assert_eq!(verdict(Some("SHA256:a"), "SHA256:a"), Verdict::Trusted);
        assert_eq!(verdict(None, "SHA256:a"), Verdict::Unknown);
        assert_eq!(verdict(Some("SHA256:b"), "SHA256:a"), Verdict::Changed);
    }

    #[test]
    fn key_files_load_or_explain_why_not() {
        assert!(load_key(&key("id_ed25519"), None).is_ok());
        assert!(load_key(&key("id_ed25519_enc"), Some("tabletist")).is_ok());
        let stage = |result: Result<russh::keys::PrivateKey>| match result {
            Err(Error::Ssh { stage, message }) => (stage, message),
            other => panic!("{:?}", other.map(|_| ())),
        };
        let (needs, message) = stage(load_key(&key("id_ed25519_enc"), None));
        assert_eq!(needs, SshStage::Secret);
        assert!(message.contains("passphrase"), "{message}");
        let (wrong, message) = stage(load_key(&key("id_ed25519_enc"), Some("nope")));
        assert_eq!(wrong, SshStage::Secret);
        assert!(message.contains("passphrase"), "{message}");
        let (missing, message) = stage(load_key(&key("absent"), None));
        assert_eq!(missing, SshStage::Auth);
        assert!(message.contains("absent"), "{message}");
    }

    #[test]
    fn encrypted_pem_and_pkcs8_keys_ask_for_their_passphrase() {
        for name in ["id_rsa_pem_enc", "id_rsa_pkcs8_enc"] {
            for passphrase in [None, Some("nope")] {
                match load_key(&key(name), passphrase) {
                    Err(Error::Ssh {
                        stage: SshStage::Secret,
                        message,
                    }) => assert!(message.contains("passphrase"), "{name}: {message}"),
                    other => panic!("{name} {passphrase:?}: {:?}", other.map(|_| ())),
                }
            }
            assert!(
                load_key(&key(name), Some("tabletist")).is_ok(),
                "{name} with its passphrase"
            );
        }
    }

    /// Needs a running agent holding tests/ssh/id_ed25519 (CI loads it).
    #[tokio::test]
    #[ignore = "needs an SSH agent with the test key"]
    async fn the_agent_lists_its_keys() {
        let mut agent = default_agent().await.unwrap();
        let identities = agent.request_identities().await.unwrap();
        assert!(!identities.is_empty());
    }

    /// An endpoint at `host:port` reached through `command`.
    #[cfg(unix)]
    fn proxied(host: &str, port: u16, user: &str, command: &str) -> Endpoint {
        let config = HostConfig {
            proxy: Some(Proxy::Command(command.into())),
            ..HostConfig::default()
        };
        let spec = SshSpec {
            host: host.into(),
            port: Some(port),
            user: user.into(),
            auth: SshAuth::Password,
        };
        endpoint(&spec, &config, None).unwrap()
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn a_proxy_command_that_fails_says_why() {
        let endpoint = proxied("db", 22, "me", "sh -c 'echo no route to %h >&2; exit 1'");
        let spec = alias(SshAuth::Password);
        let error = Tunnel::open_to(
            &endpoint,
            &spec,
            ("db", 5432),
            &Secrets::default(),
            &HostKeys::default(),
            PROXY_WAITS,
        )
        .await
        .err()
        .unwrap();
        let Error::Ssh {
            stage: SshStage::Connect,
            message,
        } = error
        else {
            panic!("{error:?}");
        };
        assert!(message.contains("through the ProxyCommand"), "{message}");
        assert!(message.contains("it said: no route to db"), "{message}");
    }

    /// A server that says hello through the proxy and then nothing more:
    /// russh keeps the stream in a task of its own after the timeout, and
    /// the proxy has to stop all the same.
    #[cfg(unix)]
    #[tokio::test]
    async fn a_proxy_that_stalls_is_stopped_at_the_timeout() {
        let dir = std::env::temp_dir().join(format!("tabletist-stall-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let pid_file = dir.join("pid");
        let command = format!(
            "sh -c 'echo $$ > {}; printf \"SSH-2.0-stall\\r\\n\"; exec sleep 60'",
            pid_file.display()
        );
        let endpoint = proxied("db", 22, "me", &command);
        let error = Tunnel::open_to(
            &endpoint,
            &alias(SshAuth::Password),
            ("db", 5432),
            &Secrets::default(),
            &HostKeys::default(),
            ProxyWaits {
                hello: Duration::from_secs(1),
                login: PROXY_WAITS.login,
            },
        )
        .await
        .err()
        .unwrap();
        assert!(
            matches!(&error, Error::Ssh { stage: SshStage::Connect, message } if message.contains("no answer")),
            "{error:?}"
        );
        let pid = std::fs::read_to_string(&pid_file).unwrap();
        let _ = std::fs::remove_dir_all(&dir);
        stops(pid.trim()).await;
    }

    /// Waits up to five seconds for the process `pid` to stop: gone, or a
    /// zombie nobody has reaped yet.
    #[cfg(unix)]
    async fn stops(pid: &str) {
        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        loop {
            let state = std::process::Command::new("ps")
                .args(["-o", "stat=", "-p", pid])
                .output()
                .unwrap();
            let state = String::from_utf8_lossy(&state.stdout);
            if state.trim().is_empty() || state.trim_start().starts_with('Z') {
                return;
            }
            assert!(
                std::time::Instant::now() < deadline,
                "the proxy {pid} is still {state}"
            );
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    }

    /// An SSH server that takes the handshake and then never answers a
    /// login.
    #[cfg(unix)]
    struct Silent;

    #[cfg(unix)]
    impl russh::server::Handler for Silent {
        type Error = russh::Error;

        async fn auth_password(
            &mut self,
            _user: &str,
            _password: &str,
        ) -> std::result::Result<russh::server::Auth, Self::Error> {
            std::future::pending().await
        }
    }

    /// A proxy that passes the hello on and then nothing of the login:
    /// the login stops at its limit, and the proxy with it.
    #[cfg(unix)]
    #[tokio::test]
    async fn a_login_through_a_proxy_that_never_answers_stops_at_its_limit() {
        let found = std::process::Command::new("sh")
            .args(["-c", "command -v nc"])
            .output()
            .is_ok_and(|output| output.status.success());
        if !found {
            eprintln!("skipped: nc is not installed");
            return;
        }
        let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0))
            .await
            .unwrap();
        let port = listener.local_addr().unwrap().port();
        let config = Arc::new(russh::server::Config {
            keys: vec![russh::keys::load_secret_key(key("id_ed25519"), None).unwrap()],
            ..Default::default()
        });
        tokio::spawn(async move {
            while let Ok((socket, _)) = listener.accept().await {
                let config = Arc::clone(&config);
                tokio::spawn(async move {
                    if let Ok(session) = russh::server::run_stream(config, socket, Silent).await {
                        let _ = session.await;
                    }
                });
            }
        });
        let dir = std::env::temp_dir().join(format!("tabletist-login-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let pid_file = dir.join("pid");
        let command = format!("sh -c 'echo $$ > {}; exec nc %h %p'", pid_file.display());
        let endpoint = proxied("127.0.0.1", port, "me", &command);
        let spec = SshSpec {
            host: "127.0.0.1".into(),
            port: Some(port),
            user: "me".into(),
            auth: SshAuth::Password,
        };
        let secrets = Secrets {
            ssh_password: Some("secret".into()),
            ..Secrets::default()
        };
        let waits = ProxyWaits {
            hello: Duration::from_secs(5),
            login: Duration::from_secs(1),
        };
        let open = |keys: HostKeys| {
            let (endpoint, spec, secrets) = (&endpoint, &spec, &secrets);
            async move { Tunnel::open_to(endpoint, spec, ("db", 5432), secrets, &keys, waits).await }
        };
        let fingerprint = match open(HostKeys::default()).await {
            Err(Error::Ssh {
                stage: SshStage::HostKeyUnknown { fingerprint, .. },
                ..
            }) => fingerprint,
            other => panic!("{:?}", other.err()),
        };
        let mut keys = HostKeys::default();
        keys.trust("127.0.0.1", port, &fingerprint);
        let error = open(keys).await.err().unwrap();
        assert!(
            matches!(&error, Error::Ssh { stage: SshStage::Auth, message } if message.contains("to the login within 1 seconds")),
            "{error:?}"
        );
        let pid = std::fs::read_to_string(&pid_file).unwrap();
        let _ = std::fs::remove_dir_all(&dir);
        stops(pid.trim()).await;
    }

    /// Opens a tunnel to the compose `ssh` service through `nc` as a
    /// ProxyCommand, and reaches PostgreSQL behind it. Set
    /// TABLETIST_TEST_SSH_URL (as for tests/ssh.rs) to run it.
    #[cfg(unix)]
    #[tokio::test]
    async fn a_proxy_command_carries_the_tunnel() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let Ok(url) = std::env::var("TABLETIST_TEST_SSH_URL") else {
            eprintln!("skipped: TABLETIST_TEST_SSH_URL is not set");
            return;
        };
        let rest = url.strip_prefix("ssh://").unwrap();
        let (login, address) = rest.split_once('@').unwrap();
        let (user, password) = login.split_once(':').unwrap();
        let (host, port) = address.trim_end_matches('/').split_once(':').unwrap();
        let port: u16 = port.parse().unwrap();
        let endpoint = proxied(host, port, user, "nc %h %p");
        let spec = SshSpec {
            host: host.into(),
            port: Some(port),
            user: user.into(),
            auth: SshAuth::Password,
        };
        let secrets = Secrets {
            ssh_password: Some(password.into()),
            ..Secrets::default()
        };
        let open = |keys: HostKeys| {
            let (endpoint, spec, secrets) = (&endpoint, &spec, &secrets);
            async move {
                let target = ("postgres", 5432);
                Tunnel::open_to(endpoint, spec, target, secrets, &keys, PROXY_WAITS).await
            }
        };
        // The server's key is checked through the proxy as without one.
        let fingerprint = match open(HostKeys::default()).await {
            Err(Error::Ssh {
                stage: SshStage::HostKeyUnknown { fingerprint, .. },
                ..
            }) => fingerprint,
            other => panic!("{:?}", other.err()),
        };
        let mut keys = HostKeys::default();
        keys.trust(host, port, &fingerprint);
        let tunnel = open(keys).await.unwrap();
        // PostgreSQL answers an SSLRequest with one byte, S or N.
        let mut socket = tokio::net::TcpStream::connect(("127.0.0.1", tunnel.port))
            .await
            .unwrap();
        socket
            .write_all(&[0, 0, 0, 8, 0x04, 0xd2, 0x16, 0x2f])
            .await
            .unwrap();
        let mut answer = [0];
        socket.read_exact(&mut answer).await.unwrap();
        assert!(matches!(&answer, b"S" | b"N"), "{answer:?}");
    }

    fn alias(auth: SshAuth) -> SshSpec {
        SshSpec {
            host: "bastion".into(),
            port: None,
            user: String::new(),
            auth,
        }
    }

    fn bastion_config() -> HostConfig {
        HostConfig {
            host_name: Some("10.0.0.5".into()),
            port: Some(2222),
            user: Some("ops".into()),
            identity_file: Some("/home/me/.ssh/id_work".into()),
            ..HostConfig::default()
        }
    }

    #[test]
    fn a_host_alias_takes_its_host_port_user_and_key_from_the_config() {
        let spec = alias(SshAuth::KeyFile {
            path: PathBuf::new(),
        });
        assert_eq!(
            endpoint(&spec, &bastion_config(), Some("me")).unwrap(),
            Endpoint {
                host: "10.0.0.5".into(),
                port: 2222,
                user: "ops".into(),
                key: Some("/home/me/.ssh/id_work".into()),
                via: None,
            }
        );
    }

    #[test]
    fn typed_values_win_over_the_config() {
        let spec = SshSpec {
            port: Some(2200),
            user: "deploy".into(),
            ..alias(SshAuth::KeyFile {
                path: "/keys/id_deploy".into(),
            })
        };
        assert_eq!(
            endpoint(&spec, &bastion_config(), Some("me")).unwrap(),
            Endpoint {
                host: "10.0.0.5".into(),
                port: 2200,
                user: "deploy".into(),
                key: Some("/keys/id_deploy".into()),
                via: None,
            }
        );
    }

    #[test]
    fn without_a_config_ssh_defaults_apply() {
        let spec = alias(SshAuth::Agent);
        assert_eq!(
            endpoint(&spec, &HostConfig::default(), Some("me")).unwrap(),
            Endpoint {
                host: "bastion".into(),
                port: 22,
                user: "me".into(),
                key: None,
                via: None,
            }
        );
    }

    #[test]
    fn a_proxy_command_takes_the_endpoints_host_port_and_user() {
        let spec = SshSpec {
            port: Some(2200),
            ..alias(SshAuth::Agent)
        };
        let config = HostConfig {
            proxy: Some(Proxy::Command("nc -X connect %h %p # %r@%n".into())),
            ..bastion_config()
        };
        let endpoint = endpoint(&spec, &config, Some("me")).unwrap();
        assert_eq!(
            endpoint.via,
            Some(proxy::Via::Command(
                "nc -X connect 10.0.0.5 2200 # ops@bastion".into()
            ))
        );
        assert_eq!(endpoint.reach(), "10.0.0.5:2200 through the ProxyCommand");
    }

    #[test]
    fn a_proxy_jump_goes_to_the_endpoint() {
        let config = HostConfig {
            proxy: Some(Proxy::Jump("gateway".into())),
            ..bastion_config()
        };
        let jumped = endpoint(&alias(SshAuth::Agent), &config, Some("me")).unwrap();
        assert_eq!(jumped.reach(), "10.0.0.5:2222 through ProxyJump gateway");
        let Some(proxy::Via::Jump { args, .. }) = jumped.via else {
            panic!("{:?}", jumped.via);
        };
        assert_eq!(
            args,
            ["-o", "BatchMode=yes", "-W", "[10.0.0.5]:2222", "gateway"]
        );
        let direct = HostConfig {
            proxy: Some(Proxy::Off),
            ..bastion_config()
        };
        let endpoint = endpoint(&alias(SshAuth::Agent), &direct, Some("me")).unwrap();
        assert_eq!(endpoint.via, None);
        assert_eq!(endpoint.reach(), "10.0.0.5:2222");
    }

    #[test]
    fn a_proxy_the_tunnel_cannot_run_is_a_connect_error() {
        let typed = SshSpec {
            host: "db;id".into(),
            ..alias(SshAuth::Agent)
        };
        for (spec, proxy, says) in [
            (alias(SshAuth::Agent), Proxy::Command("nc %x".into()), "%x"),
            (
                alias(SshAuth::Agent),
                Proxy::Jump("-oX".into()),
                "ProxyJump",
            ),
            (typed, Proxy::Command("nc %n 22".into()), "characters"),
        ] {
            let config = HostConfig {
                proxy: Some(proxy),
                ..HostConfig::default()
            };
            match endpoint(&spec, &config, Some("me")) {
                Err(Error::Ssh {
                    stage: SshStage::Connect,
                    message,
                }) => {
                    assert!(message.contains("~/.ssh/config"), "{message}");
                    assert!(message.contains(says), "{message}");
                }
                other => panic!("{says}: {other:?}"),
            }
        }
    }

    #[test]
    fn a_missing_host_user_or_key_is_an_invalid_spec() {
        let invalid = |result: Result<Endpoint>| match result {
            Err(Error::InvalidSpec(message)) => message,
            other => panic!("{other:?}"),
        };
        let empty = SshSpec {
            host: "  ".into(),
            ..alias(SshAuth::Agent)
        };
        assert!(invalid(endpoint(&empty, &HostConfig::default(), Some("me"))).contains("host"));
        assert!(
            invalid(endpoint(
                &alias(SshAuth::Agent),
                &HostConfig::default(),
                None
            ))
            .contains("user")
        );
        let keyless = alias(SshAuth::KeyFile {
            path: PathBuf::new(),
        });
        assert!(invalid(endpoint(&keyless, &HostConfig::default(), Some("me"))).contains("key"));
    }
}
