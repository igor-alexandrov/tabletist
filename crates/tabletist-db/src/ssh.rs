//! SSH tunnels: connect, check the host key, authenticate, and forward a
//! local port to the database through `direct-tcpip` channels.

use std::collections::BTreeMap;
use std::path::Path;
#[cfg(test)]
use std::path::PathBuf;
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

use russh::client;
use russh::keys::agent::AgentIdentity;
use russh::keys::agent::client::{AgentClient, AgentStream};
use russh::keys::{HashAlg, PrivateKey, PrivateKeyWithHashAlg, PublicKeyOrCertificate};
use serde::{Deserialize, Serialize};

use crate::{Error, Result, Secrets, SshAuth, SshSpec, SshStage};

const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);

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

#[cfg(unix)]
async fn agent() -> std::result::Result<Agent, String> {
    AgentClient::connect_env()
        .await
        .map(AgentClient::dynamic)
        .map_err(|error| error.to_string())
}

#[cfg(windows)]
async fn agent() -> std::result::Result<Agent, String> {
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
                .authenticate_password(&ssh.user, password)
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
        SshAuth::KeyFile { path } => {
            let passphrase = secrets.ssh_passphrase.as_deref().filter(|p| !p.is_empty());
            let key = load_key(path, passphrase)?;
            handle
                .authenticate_publickey(
                    &ssh.user,
                    PrivateKeyWithHashAlg::new(Arc::new(key), rsa_hash),
                )
                .await
                .map_err(auth_error)?
                .success()
        }
        SshAuth::Agent => {
            let mut agent = agent().await.map_err(|error| {
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
                    .authenticate_publickey_with(&ssh.user, key, hash, &mut agent)
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
            format!("the server did not accept any key for {}", ssh.user),
        ))
    }
}

/// A local port forwarded to the database through an SSH session. Dropping
/// it stops accepting; the session ends when the last forwarded socket
/// closes.
pub(crate) struct Tunnel {
    pub port: u16,
    forward_error: Arc<Mutex<Option<Error>>>,
    task: tokio::task::JoinHandle<()>,
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
        if ssh.host.trim().is_empty() || ssh.user.trim().is_empty() {
            return Err(Error::InvalidSpec("enter the SSH host and user".into()));
        }
        let seen = Arc::new(Mutex::new(None));
        let client = Client {
            known: host_keys
                .fingerprint(&ssh.host, ssh.port)
                .map(str::to_owned),
            seen: Arc::clone(&seen),
        };
        let config = Arc::new(client::Config {
            nodelay: true,
            inactivity_timeout: None,
            keepalive_interval: Some(Duration::from_secs(30)),
            ..Default::default()
        });
        let address = (ssh.host.clone(), ssh.port);
        let connected =
            tokio::time::timeout(CONNECT_TIMEOUT, client::connect(config, address, client))
                .await
                .map_err(|_| {
                    ssh_error(
                        SshStage::Connect,
                        format!("no answer from {}:{} within 10 seconds", ssh.host, ssh.port),
                    )
                })?;
        let mut handle = match connected {
            Ok(handle) => handle,
            Err(russh::Error::UnknownKey) => {
                let fingerprint = seen
                    .lock()
                    .unwrap_or_else(PoisonError::into_inner)
                    .clone()
                    .unwrap_or_default();
                return Err(if host_keys.fingerprint(&ssh.host, ssh.port).is_some() {
                    ssh_error(
                        SshStage::HostKeyMismatch { fingerprint },
                        "the host key changed since you trusted it, which can mean someone \
                         is intercepting the connection; if the server's key really changed, \
                         remove its line from known_hosts.json",
                    )
                } else {
                    ssh_error(
                        SshStage::HostKeyUnknown { fingerprint },
                        format!("{} is not a trusted host yet", ssh.host),
                    )
                });
            }
            Err(error) => {
                return Err(ssh_error(
                    SshStage::Connect,
                    format!("could not reach {}:{}: {error}", ssh.host, ssh.port),
                ));
            }
        };
        authenticate(&mut handle, ssh, secrets).await?;
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
        let mut agent = agent().await.unwrap();
        let identities = agent.request_identities().await.unwrap();
        assert!(!identities.is_empty());
    }
}
