# Batch 6: SSH Tunnel Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Connect to PostgreSQL and MySQL through an SSH tunnel (password, key file with optional passphrase, or agent), with host keys trusted on first use and refused when they change, SSH secrets kept like database passwords, and an SSH section in the connection dialog.

**Architecture:** `tabletist-db` gains `ssh.rs` on `russh`: it connects, checks the host key against a `HostKeys` store it is given, authenticates, binds `127.0.0.1:0`, and forwards each accepted socket through a `direct-tcpip` channel to the database host (resolved on the SSH server). `Connection` becomes a struct that owns the driver connection and, after it, the tunnel, so dropping a connection closes both. Drivers connect to the local port but keep the real host name for TLS (`tokio_postgres::Config::hostaddr`, `mysql_async` TLS host override). The app keeps trusted keys in `known_hosts.json`, asks before trusting a new key, refuses a changed one, and gathers the database password and the SSH secret one at a time (keyring, else prompt).

**Tech Stack:** russh 0.63 (`default-features = false`, features `ring`, `rsa`, `flate2`; no aws-lc); an Alpine OpenSSH container for integration tests.

**Spec:** `docs/superpowers/specs/2026-09-27-tabletist-design.md` (sections 4.6, 4.7, 5.4, 7; batch 6 in section 9; the Windows agent risk in section 10)

## Global Constraints

- Everything from earlier batches' Global Constraints still applies. Verify with `cargo test --locked --workspace --all-targets` and `cargo clippy --locked --workspace --all-targets -- -D warnings`; run `cargo fmt --all` before every commit; commits are signed.
- russh builds on `ring` only: `default-features = false, features = ["ring", "rsa", "flate2"]`. The existing test that rustls has exactly one crypto provider must still pass.
- Secrets (database password, SSH password, key passphrase) never appear in `ConnectSpec`, `connections.json`, logs, errors or `Debug` output. The SSH secret is kept in the keyring under `connection/<id>/ssh-secret`.
- Host keys: first contact reports `Error::Ssh { stage: HostKeyUnknown { fingerprint } }` and the app asks; a changed key is `HostKeyMismatch` and is refused, with no override in any dialog. Fingerprints are OpenSSH's `SHA256:<base64>` form. Store: `config/known_hosts.json`, keyed `host:port` (host lowercased).
- The database host is resolved by the SSH server, never locally. With TLS through a tunnel, certificates are checked against the database host name the user typed, never `127.0.0.1`.
- SSH integration tests read `TABLETIST_TEST_SSH_URL` (`ssh://tabletist:tabletist@localhost:52222`); without it they print "skipped" and pass. Agent tests also need `TABLETIST_TEST_SSH_AGENT=1` and the test key loaded in the running agent.
- Test keys live in `crates/tabletist-db/tests/ssh/`. They are for the local test container only; say so in a README there.
- Never use em dashes. One topic per commit on `main`.

## Review Focus

1. **A host key that changed since it was trusted:** refused every time, never silently re-trusted, and no button anywhere trusts it. Test: Task 1 `a_changed_host_key_is_refused`, Task 4 `a_changed_host_key_shows_the_error_and_no_prompt`.
2. **A database host only the SSH server can resolve** (a private DNS name, a Docker service name): the tunnel still reaches it. Test: Task 2 `postgres_through_the_tunnel_lists_schemas` targets `postgres`, which does not resolve on the test machine.
3. **TLS through a tunnel:** the certificate is checked against the typed host, not `127.0.0.1`. Test: Task 2 `a_tunnelled_postgres_config_keeps_the_host_name_for_tls` and `a_tunnelled_mysql_opts_keep_the_host_name_for_tls`.
4. **Closing a tab or a failed connect:** the SSH session and local port close with the connection; nothing keeps listening. Test: Task 2 `closing_the_connection_closes_the_tunnel`.
5. **A wrong or missing SSH password or passphrase:** prompts for it, never resends the rejected one, saves it only after the server accepts it. Test: Task 5 `a_rejected_ssh_secret_prompts_and_is_not_resent` and `an_ssh_secret_from_the_prompt_is_saved_after_connecting`.

---

## File Structure

```
compose.yaml                                  + ssh service (built from compose/ssh), healthchecks
compose/ssh/Dockerfile                        Alpine OpenSSH: password + test keys, forwarding on
.github/workflows/ci.yml                      + ssh integration job, agent checks (Linux, Windows)
AGENTS.md                                     + how to run the SSH suite
crates/tabletist-db/Cargo.toml                + russh; tokio net + io-util
crates/tabletist-db/src/ssh.rs                HostKeys, Tunnel (connect, trust, auth, forward)
crates/tabletist-db/src/error.rs              + SshStage::Secret
crates/tabletist-db/src/lib.rs                Connection struct { inner, tunnel }, connect_with
crates/tabletist-db/src/pg.rs                 config() split out; tunnel port via hostaddr
crates/tabletist-db/src/mysql.rs              opts via tunnel with TLS host override
crates/tabletist-db/src/spec.rs               summary() names the SSH host
crates/tabletist-db/tests/ssh.rs              env-gated integration tests
crates/tabletist-db/tests/ssh/                test keys (plain, encrypted) + README
src/known_hosts.rs                            load/save HostKeys
src/backend.rs                                Connect/Test carry HostKeys; pick_key_file
src/connections.rs                            SavedConnection.ssh_secret
src/secrets.rs                                ssh_account()
src/model.rs                                  form SSH fields, SecretKind, HostKeyPrompt, TestState::Untrusted
src/app.rs                                    trust flow, secret gathering, SSH secret keyring
src/ui/connect_dialog.rs                      SSH section
src/ui/host_key_prompt.rs                     trust dialog
src/ui/password_prompt.rs                     title per SecretKind
src/ui/workspace.rs                           "via SSH" in the top bar
src/ui/mod.rs                                 dispatch the trust dialog; UI tests
```

## Pre-flight notes (verified in a spike against OpenSSH 10 on 2026-09-28)

- `russh::client::connect(Arc<Config>, (host, port), handler)`; `Handler::check_server_key(&mut self, &russh::keys::PublicKeyOrCertificate) -> Result<bool, Self::Error>`. Returning `Ok(false)` makes `connect` fail with `russh::Error::UnknownKey`.
- `PublicKey::fingerprint(HashAlg::Sha256).to_string()` gives `SHA256:4Uec...` exactly like `ssh-keygen -lf`.
- `load_secret_key(path, None)` on an encrypted key is `russh::keys::Error::KeyIsEncrypted`; a wrong passphrase is `russh::keys::Error::SshKey(_)`; a missing file is `russh::keys::Error::IO(_)`.
- `authenticate_publickey`, `authenticate_password` and `authenticate_publickey_with(user, key, hash, &mut agent)` return `AuthResult`; `.success()`.
- `channel_open_direct_tcpip(&self, host, port as u32, origin_ip, origin_port as u32)`; an unreachable target is `Failed to open channel (ConnectFailed)`. `Channel::into_stream()` works with `tokio::io::copy_bidirectional`.
- Dropping the `client::Handle` ends the session: open channels read EOF.
- Agent: `AgentClient::connect_env()` (Unix), `connect_named_pipe(r"\\.\pipe\openssh-ssh-agent")` and `connect_pageant()` (Windows); `.dynamic()` erases the stream type; `request_identities()` gives `AgentIdentity::PublicKey { key, .. }`.
- tokio-postgres 0.7.18: `Config::host` names the TLS peer, `Config::hostaddr` is where the socket connects; the cancel token keeps both.
- mysql_async 0.37: `SslOpts::with_danger_tls_hostname_override(Some(host))`.

---

### Task 1: The tunnel (`ssh.rs`), host keys, and the SSH test server

**Files:**
- Modify: `crates/tabletist-db/Cargo.toml`, `crates/tabletist-db/src/error.rs`, `crates/tabletist-db/src/lib.rs`
- Create: `crates/tabletist-db/src/ssh.rs`, `crates/tabletist-db/tests/ssh/{id_ed25519,id_ed25519.pub,id_ed25519_enc,id_ed25519_enc.pub,README.md}`, `compose/ssh/Dockerfile`, `crates/tabletist-db/tests/ssh.rs`
- Modify: `compose.yaml`

**Interfaces:**
- Produces: `pub struct HostKeys` (serde, `Default`, `Clone`, `PartialEq`) with `fingerprint(&self, host: &str, port: u16) -> Option<&str>` and `trust(&mut self, host: &str, port: u16, fingerprint: &str)`; `pub(crate) struct Tunnel { pub port: u16, .. }` with `Tunnel::open(ssh: &SshSpec, target_host: &str, target_port: u16, secrets: &Secrets, host_keys: &HostKeys) -> Result<Tunnel>` and `forward_error(&self) -> Option<Error>`; `SshStage::Secret` (the SSH password or passphrase is missing or wrong: ask for it).

- [ ] **Step 1: Dependencies and the new stage**

`crates/tabletist-db/Cargo.toml`, in `[dependencies]` (and add `"net", "io-util"` to the existing `tokio` features there):

```toml
# SSH tunnels, on ring (no aws-lc to build).
russh = { version = "0.63", default-features = false, features = ["ring", "rsa", "flate2"] }
```

`crates/tabletist-db/src/error.rs`, in `SshStage` after `Auth`:

```rust
    /// The SSH password or key passphrase is missing or wrong: ask for it.
    Secret,
```

and in its `Display` match: `Self::Auth | Self::Secret => f.write_str("authentication"),` (replacing the `Auth` arm).

`crates/tabletist-db/src/lib.rs`: add `mod ssh;` to the module list and `pub use ssh::HostKeys;` to the re-exports.

- [ ] **Step 2: Test keys and the container**

```bash
mkdir -p crates/tabletist-db/tests/ssh
ssh-keygen -q -t ed25519 -N '' -C tabletist-test -f crates/tabletist-db/tests/ssh/id_ed25519
ssh-keygen -q -t ed25519 -N tabletist -C tabletist-test-enc -f crates/tabletist-db/tests/ssh/id_ed25519_enc
```

`crates/tabletist-db/tests/ssh/README.md`:

```markdown
# Test keys

These keys only open the throwaway SSH container in `compose.yaml`
(`compose/ssh`). They are public on purpose; never authorize them anywhere
else. `id_ed25519_enc`'s passphrase is `tabletist`.
```

`compose/ssh/Dockerfile`:

```dockerfile
# A throwaway SSH server for the tunnel tests: user tabletist, password
# tabletist, and the test keys from crates/tabletist-db/tests/ssh.
FROM alpine:3.22
RUN apk add --no-cache openssh \
 && ssh-keygen -A \
 && adduser -D -s /bin/sh tabletist \
 && echo 'tabletist:tabletist' | chpasswd \
 && sed -i -e 's/^#\?AllowTcpForwarding.*/AllowTcpForwarding yes/' \
           -e 's/^#\?PasswordAuthentication.*/PasswordAuthentication yes/' /etc/ssh/sshd_config \
 && mkdir -p /home/tabletist/.ssh
COPY crates/tabletist-db/tests/ssh/id_ed25519.pub crates/tabletist-db/tests/ssh/id_ed25519_enc.pub /tmp/keys/
RUN cat /tmp/keys/*.pub > /home/tabletist/.ssh/authorized_keys \
 && chown -R tabletist:tabletist /home/tabletist/.ssh \
 && chmod 700 /home/tabletist/.ssh && chmod 600 /home/tabletist/.ssh/authorized_keys
EXPOSE 22
CMD ["/usr/sbin/sshd", "-D", "-e"]
```

`compose.yaml`: add to the header comment `#   docker compose up -d --build --wait postgres mysql ssh` and `#   TABLETIST_TEST_SSH_URL=ssh://tabletist:tabletist@localhost:52222 \`; give `postgres` a healthcheck so `--wait` works for it too:

```yaml
    healthcheck:
      test: ["CMD", "pg_isready", "-U", "tabletist"]
      interval: 2s
      timeout: 5s
      retries: 30
```

and add the service:

```yaml
  # SSH server for tunnel tests. It reaches the databases by service name
  # (postgres:5432, mysql:3306), which the host cannot resolve.
  ssh:
    build:
      context: .
      dockerfile: compose/ssh/Dockerfile
    ports:
      - "52222:22"
    healthcheck:
      test: ["CMD", "nc", "-z", "127.0.0.1", "22"]
      interval: 2s
      timeout: 5s
      retries: 30
```

- [ ] **Step 3: Write the failing unit tests**

Create `crates/tabletist-db/src/ssh.rs` with only the tests module and the signatures they need stubbed as `todo!()` (so it compiles and the tests fail):

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn key(name: &str) -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/ssh").join(name)
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
        let stage = |result: Result<_>| match result {
            Err(Error::Ssh { stage, message }) => (stage, message),
            other => panic!("{:?}", other.map(|_: russh::keys::PrivateKey| ())),
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

    /// Needs a running agent holding tests/ssh/id_ed25519 (CI loads it).
    #[tokio::test]
    #[ignore = "needs an SSH agent with the test key"]
    async fn the_agent_lists_its_keys() {
        let mut agent = agent().await.unwrap();
        let identities = agent.request_identities().await.unwrap();
        assert!(!identities.is_empty());
    }
}
```

- [ ] **Step 4: Run them to watch them fail**

Run: `cargo test --locked -p tabletist-db --lib ssh::`
Expected: FAIL (panics at `todo!()`), the agent test ignored.

- [ ] **Step 5: Implement `ssh.rs`**

Replace the stubs with the module body (the tests module stays at the bottom):

```rust
//! SSH tunnels: connect, check the host key, authenticate, and forward a
//! local port to the database through `direct-tcpip` channels.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
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
        self.hosts.insert(Self::key(host, port), fingerprint.to_owned());
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
        *self.seen.lock().unwrap() = Some(fingerprint);
        Ok(trusted)
    }
}

/// A private key file, decrypted with `passphrase` when it has one.
pub(crate) fn load_key(path: &Path, passphrase: Option<&str>) -> Result<PrivateKey> {
    use russh::keys::Error as KeyError;
    russh::keys::load_secret_key(path, passphrase).map_err(|error| match error {
        KeyError::KeyIsEncrypted => ssh_error(SshStage::Secret, "the key file needs its passphrase"),
        KeyError::SshKey(_) if passphrase.is_some() => {
            ssh_error(SshStage::Secret, "the key file's passphrase is wrong")
        }
        KeyError::IO(io) => ssh_error(
            SshStage::Auth,
            format!("could not read {}: {io}", path.display()),
        ),
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
    let rsa_hash = handle.best_supported_rsa_hash().await.ok().flatten().flatten();
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
                return Err(ssh_error(SshStage::Secret, "the server refused the SSH password"));
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
                ssh_error(SshStage::Auth, format!("could not reach the SSH agent: {error}"))
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
            known: host_keys.fingerprint(&ssh.host, ssh.port).map(str::to_owned),
            seen: Arc::clone(&seen),
        };
        let config = Arc::new(client::Config {
            nodelay: true,
            inactivity_timeout: None,
            keepalive_interval: Some(Duration::from_secs(30)),
            ..Default::default()
        });
        let address = (ssh.host.clone(), ssh.port);
        let connected = tokio::time::timeout(CONNECT_TIMEOUT, client::connect(config, address, client))
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
                let fingerprint = seen.lock().unwrap().clone().unwrap_or_default();
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
                            *slot.lock().unwrap() = Some(ssh_error(
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
        self.forward_error.lock().unwrap().clone()
    }
}
```

If a russh path differs from the spike's (for example `AgentStream`'s module or `Algorithm`), fix the import and ledger it as a ruling; the behavior above is what matters.

- [ ] **Step 6: Run the unit tests**

Run: `cargo test --locked -p tabletist-db --lib ssh::`
Expected: PASS (3 passed, 1 ignored). Then `cargo test --locked -p tabletist-db --lib tls::` still passes (one rustls provider).

- [ ] **Step 7: Write the integration tests (tunnel level)**

`crates/tabletist-db/tests/ssh.rs` reaches `Tunnel` only through `Connection` (Task 2), so here add the file with the helpers and the host-key and auth tests, which need no database driver change; they use `Connection::connect_with`, which Task 2 adds. To keep this task's tests runnable now, add a temporary public entry point in `lib.rs` that Task 2 replaces:

```rust
#[doc(hidden)]
pub async fn open_tunnel_for_tests(
    ssh: &SshSpec,
    target_host: &str,
    target_port: u16,
    secrets: &Secrets,
    host_keys: &HostKeys,
) -> Result<u16> {
    let tunnel = ssh::Tunnel::open(ssh, target_host, target_port, secrets, host_keys).await?;
    let port = tunnel.port;
    std::mem::forget(tunnel);
    Ok(port)
}
```

`crates/tabletist-db/tests/ssh.rs`:

```rust
//! SSH tunnel tests against the compose `ssh` service. Set
//! TABLETIST_TEST_SSH_URL=ssh://tabletist:tabletist@localhost:52222 to run.

use std::path::{Path, PathBuf};

use tabletist_db::{Error, HostKeys, Secrets, SshAuth, SshSpec, SshStage};

/// The SSH server and its password, or `None` (test skipped).
pub fn server() -> Option<(SshSpec, String)> {
    let Ok(url) = std::env::var("TABLETIST_TEST_SSH_URL") else {
        eprintln!("skipped: TABLETIST_TEST_SSH_URL is not set");
        return None;
    };
    let rest = url.strip_prefix("ssh://").expect("an ssh:// URL");
    let (login, address) = rest.split_once('@').expect("user:password@host:port");
    let (user, password) = login.split_once(':').expect("user:password");
    let (host, port) = address.split_once(':').expect("host:port");
    Some((
        SshSpec {
            host: host.into(),
            port: port.trim_end_matches('/').parse().unwrap(),
            user: user.into(),
            auth: SshAuth::Password,
        },
        password.into(),
    ))
}

pub fn key(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/ssh").join(name)
}

pub fn with_password(password: &str) -> Secrets {
    Secrets {
        ssh_password: Some(password.into()),
        ..Secrets::default()
    }
}

/// Learns the server's fingerprint the way the app does: from the refusal.
pub async fn trusted(ssh: &SshSpec) -> HostKeys {
    let error = tabletist_db::open_tunnel_for_tests(ssh, "postgres", 5432, &Secrets::default(), &HostKeys::default())
        .await
        .unwrap_err();
    let Error::Ssh { stage: SshStage::HostKeyUnknown { fingerprint }, .. } = error else {
        panic!("{error:?}");
    };
    let mut keys = HostKeys::default();
    keys.trust(&ssh.host, ssh.port, &fingerprint);
    keys
}

#[tokio::test]
async fn an_unknown_host_key_is_reported_with_its_fingerprint() {
    let Some((ssh, password)) = server() else { return };
    match tabletist_db::open_tunnel_for_tests(&ssh, "postgres", 5432, &with_password(&password), &HostKeys::default()).await {
        Err(Error::Ssh { stage: SshStage::HostKeyUnknown { fingerprint }, .. }) => {
            assert!(fingerprint.starts_with("SHA256:"), "{fingerprint}")
        }
        other => panic!("{other:?}"),
    }
}

#[tokio::test]
async fn a_changed_host_key_is_refused() {
    let Some((ssh, password)) = server() else { return };
    let mut keys = HostKeys::default();
    keys.trust(&ssh.host, ssh.port, "SHA256:not-the-servers-key");
    match tabletist_db::open_tunnel_for_tests(&ssh, "postgres", 5432, &with_password(&password), &keys).await {
        Err(Error::Ssh { stage: SshStage::HostKeyMismatch { fingerprint }, message }) => {
            assert!(fingerprint.starts_with("SHA256:"));
            assert!(message.contains("known_hosts.json"), "{message}");
        }
        other => panic!("{other:?}"),
    }
}

#[tokio::test]
async fn a_password_opens_the_tunnel_and_a_wrong_one_asks_again() {
    let Some((ssh, password)) = server() else { return };
    let keys = trusted(&ssh).await;
    assert!(tabletist_db::open_tunnel_for_tests(&ssh, "postgres", 5432, &with_password(&password), &keys).await.is_ok());
    for secrets in [with_password("wrong"), Secrets::default()] {
        match tabletist_db::open_tunnel_for_tests(&ssh, "postgres", 5432, &secrets, &keys).await {
            Err(Error::Ssh { stage: SshStage::Secret, .. }) => {}
            other => panic!("{other:?}"),
        }
    }
}

#[tokio::test]
async fn key_files_open_the_tunnel_with_their_passphrase() {
    let Some((mut ssh, _)) = server() else { return };
    let keys = trusted(&ssh).await;
    ssh.auth = SshAuth::KeyFile { path: key("id_ed25519") };
    assert!(tabletist_db::open_tunnel_for_tests(&ssh, "postgres", 5432, &Secrets::default(), &keys).await.is_ok());
    ssh.auth = SshAuth::KeyFile { path: key("id_ed25519_enc") };
    match tabletist_db::open_tunnel_for_tests(&ssh, "postgres", 5432, &Secrets::default(), &keys).await {
        Err(Error::Ssh { stage: SshStage::Secret, .. }) => {}
        other => panic!("{other:?}"),
    }
    let secrets = Secrets { ssh_passphrase: Some("tabletist".into()), ..Secrets::default() };
    assert!(tabletist_db::open_tunnel_for_tests(&ssh, "postgres", 5432, &secrets, &keys).await.is_ok());
}

#[tokio::test]
async fn the_agent_opens_the_tunnel() {
    let Some((mut ssh, _)) = server() else { return };
    if std::env::var("TABLETIST_TEST_SSH_AGENT").is_err() {
        eprintln!("skipped: TABLETIST_TEST_SSH_AGENT is not set");
        return;
    }
    let keys = trusted(&ssh).await;
    ssh.auth = SshAuth::Agent;
    assert!(tabletist_db::open_tunnel_for_tests(&ssh, "postgres", 5432, &Secrets::default(), &keys).await.is_ok());
}
```

- [ ] **Step 8: Run against the container**

Run: `docker compose up -d --build --wait ssh postgres` (the user runs this if the session cannot reach Docker), then
`TABLETIST_TEST_SSH_URL=ssh://tabletist:tabletist@localhost:52222 cargo test --locked -p tabletist-db --test ssh`
Expected: PASS (5 tests; the agent one prints "skipped" unless `TABLETIST_TEST_SSH_AGENT=1` with the test key in the agent). Without the variable: all print "skipped" and pass.

- [ ] **Step 9: Commit**

```bash
cargo fmt --all
git add crates/tabletist-db compose.yaml compose/ssh
git commit -m "Open SSH tunnels with trusted host keys, passwords, keys and agents"
```

---

### Task 2: `Connection` owns its tunnel; drivers connect through it

**Files:**
- Modify: `crates/tabletist-db/src/lib.rs`, `crates/tabletist-db/src/pg.rs`, `crates/tabletist-db/src/mysql.rs`, `crates/tabletist-db/src/spec.rs`, `crates/tabletist-db/tests/ssh.rs`

**Interfaces:**
- Consumes: `ssh::Tunnel`, `HostKeys` (Task 1).
- Produces: `pub struct Connection` (no public variants) with `Connection::connect(spec, secrets)` unchanged and `Connection::connect_with(spec: &ConnectSpec, secrets: &Secrets, host_keys: &HostKeys) -> Result<Connection>`; `#[doc(hidden)] pub fn tunnel_port(&self) -> Option<u16>`; `pg::config(spec, secrets, via: Option<u16>) -> tokio_postgres::Config`; `mysql::builder(spec, secrets, via: Option<u16>) -> OptsBuilder` (plus TLS). `ConnectSpec::summary()` ends with ` via <user>@<ssh host>` when tunnelled. `open_tunnel_for_tests` is removed.

- [ ] **Step 1: Write the failing tests**

In `pg.rs` tests:

```rust
    #[test]
    fn a_tunnelled_postgres_config_keeps_the_host_name_for_tls() {
        let (spec, secrets) = ConnectSpec::from_url("postgres://me@db.example.com:5432/app").unwrap();
        let config = config(&spec, &secrets, Some(40001));
        assert_eq!(
            config.get_hosts(),
            &[tokio_postgres::config::Host::Tcp("db.example.com".into())]
        );
        assert_eq!(config.get_hostaddrs(), &[std::net::IpAddr::from([127, 0, 0, 1])]);
        assert_eq!(config.get_ports(), &[40001]);
        let direct = config_direct(&spec, &secrets);
        assert!(direct.get_hostaddrs().is_empty());
        assert_eq!(direct.get_ports(), &[5432]);
    }
```

(`config_direct` is just `config(spec, secrets, None)`; write it inline in the test instead if you prefer.)

In `mysql.rs` tests:

```rust
    #[test]
    fn a_tunnelled_mysql_opts_keep_the_host_name_for_tls() {
        let (mut spec, secrets) = ConnectSpec::from_url("mysql://me@db.example.com/shop").unwrap();
        spec.tls = TlsMode::VerifyFull;
        let opts: Opts = with_tls(builder(&spec, &secrets, Some(40002)), &spec, Some(40002)).into();
        assert_eq!(opts.ip_or_hostname(), "127.0.0.1");
        assert_eq!(opts.tcp_port(), 40002);
        assert_eq!(
            opts.ssl_opts().and_then(|ssl| ssl.tls_hostname_override()),
            Some("db.example.com")
        );
    }
```

In `spec.rs` tests:

```rust
    #[test]
    fn the_summary_names_the_ssh_host() {
        let (mut spec, _) = ConnectSpec::from_url("postgres://me@db/app").unwrap();
        spec.ssh = Some(SshSpec {
            host: "bastion".into(),
            port: 22,
            user: "ops".into(),
            auth: SshAuth::Agent,
        });
        assert_eq!(spec.summary(), "me@db:5432/app via ops@bastion");
    }
```

In `tests/ssh.rs`, replace every `tabletist_db::open_tunnel_for_tests(&ssh, "postgres", 5432, &secrets, &keys)` call with a `Connection::connect_with(&pg_via(&ssh), &secrets_with_db(secrets), &keys)` form, and add:

```rust
use std::time::Duration;
use tabletist_db::{Connection, ConnectSpec, Driver, ObjectRef, RowQuery, TlsMode};

/// PostgreSQL in the compose network, as the SSH server sees it.
fn pg_via(ssh: &SshSpec) -> ConnectSpec {
    let (mut spec, _) = ConnectSpec::from_url("postgres://tabletist@postgres:5432/tabletist").unwrap();
    spec.tls = TlsMode::Disable;
    spec.ssh = Some(ssh.clone());
    spec
}

fn mysql_via(ssh: &SshSpec) -> ConnectSpec {
    let (mut spec, _) = ConnectSpec::from_url("mysql://tabletist@mysql:3306/tabletist").unwrap();
    spec.tls = TlsMode::Disable;
    spec.ssh = Some(ssh.clone());
    spec
}

/// The database password next to the SSH secret.
fn secrets_with_db(mut secrets: Secrets) -> Secrets {
    secrets.password = Some("tabletist".into());
    secrets
}

#[tokio::test]
async fn postgres_through_the_tunnel_lists_schemas() {
    let Some((ssh, password)) = server() else { return };
    let keys = trusted(&ssh).await;
    let connection = Connection::connect_with(&pg_via(&ssh), &secrets_with_db(with_password(&password)), &keys)
        .await
        .unwrap();
    assert!(connection.list_schemas().await.unwrap().contains(&"public".to_owned()));
}

#[tokio::test]
async fn postgres_tls_works_through_the_tunnel() {
    let Some((ssh, password)) = server() else { return };
    let keys = trusted(&ssh).await;
    let mut spec = pg_via(&ssh);
    spec.tls = TlsMode::Require;
    let connection = Connection::connect_with(&spec, &secrets_with_db(with_password(&password)), &keys).await;
    assert!(connection.is_ok(), "{:?}", connection.err());
}

#[tokio::test]
async fn a_database_the_ssh_server_cannot_reach_is_a_forward_error() {
    let Some((ssh, password)) = server() else { return };
    let keys = trusted(&ssh).await;
    let mut spec = pg_via(&ssh);
    spec.host = "nowhere.invalid".into();
    match Connection::connect_with(&spec, &secrets_with_db(with_password(&password)), &keys).await {
        Err(Error::Ssh { stage: SshStage::Forward, message }) => assert!(message.contains("nowhere.invalid"), "{message}"),
        other => panic!("{:?}", other.err()),
    }
}

#[tokio::test]
async fn postgres_cancel_goes_through_the_tunnel() {
    let Some((ssh, password)) = server() else { return };
    let keys = trusted(&ssh).await;
    let connection = std::sync::Arc::new(
        Connection::connect_with(&pg_via(&ssh), &secrets_with_db(with_password(&password)), &keys).await.unwrap(),
    );
    let cancel = connection.cancel_handle();
    let running = {
        let connection = std::sync::Arc::clone(&connection);
        tokio::spawn(async move {
            let mut query = RowQuery::new(ObjectRef::new("pg_catalog", "pg_namespace"), 5);
            query.raw_where = Some("pg_sleep(30) IS NOT NULL".into());
            connection.fetch_rows(&query).await
        })
    };
    let deadline = std::time::Instant::now() + Duration::from_secs(20);
    while !running.is_finished() {
        assert!(std::time::Instant::now() < deadline, "cancel must stop the query");
        tokio::time::sleep(Duration::from_millis(300)).await;
        cancel.cancel().await.unwrap();
    }
    assert_eq!(running.await.unwrap().err(), Some(Error::Cancelled));
}

#[tokio::test]
async fn mysql_through_the_tunnel_reads_and_cancels() {
    let Some((ssh, password)) = server() else { return };
    let keys = trusted(&ssh).await;
    let connection = std::sync::Arc::new(
        Connection::connect_with(&mysql_via(&ssh), &secrets_with_db(with_password(&password)), &keys).await.unwrap(),
    );
    assert_eq!(connection.driver(), Driver::MySql);
    assert!(connection.list_schemas().await.unwrap().contains(&"tabletist".to_owned()));
    let cancel = connection.cancel_handle();
    let running = {
        let connection = std::sync::Arc::clone(&connection);
        tokio::spawn(async move {
            let mut query = RowQuery::new(ObjectRef::new("information_schema", "CHARACTER_SETS"), 5);
            query.raw_where = Some("SLEEP(30) = 0".into());
            connection.fetch_rows(&query).await
        })
    };
    let deadline = std::time::Instant::now() + Duration::from_secs(20);
    while !running.is_finished() {
        assert!(std::time::Instant::now() < deadline, "cancel must stop the query");
        tokio::time::sleep(Duration::from_millis(300)).await;
        cancel.cancel().await.unwrap();
    }
    assert!(running.await.unwrap().is_err());
}

#[tokio::test]
async fn closing_the_connection_closes_the_tunnel() {
    let Some((ssh, password)) = server() else { return };
    let keys = trusted(&ssh).await;
    let connection = Connection::connect_with(&pg_via(&ssh), &secrets_with_db(with_password(&password)), &keys)
        .await
        .unwrap();
    let port = connection.tunnel_port().expect("a tunnel");
    connection.close().await.unwrap();
    tokio::time::sleep(Duration::from_millis(200)).await;
    assert!(tokio::net::TcpStream::connect(("127.0.0.1", port)).await.is_err());
}
```

`trusted()` becomes `Connection::connect_with(&pg_via(ssh), &Secrets::default(), &HostKeys::default())` with the same `HostKeyUnknown` match. `tokio` needs `"net"` in the dev-dependency features.

- [ ] **Step 2: Run them to watch them fail**

Run: `cargo test --locked -p tabletist-db --lib a_tunnelled the_summary_names` and the SSH suite with the container.
Expected: compile errors (`config`, `builder`, `with_tls`, `connect_with`, `tunnel_port` do not exist), which is the failure for new API.

- [ ] **Step 3: Split out the PostgreSQL config**

In `pg.rs`, move the `Config` building out of `connect` into:

```rust
/// The client config; through a tunnel the socket goes to 127.0.0.1:`via`
/// while TLS still checks `spec.host`.
pub(crate) fn config(spec: &ConnectSpec, secrets: &Secrets, via: Option<u16>) -> tokio_postgres::Config {
    let mut config = tokio_postgres::Config::new();
    config
        .host(&spec.host)
        .port(via.unwrap_or(spec.port))
        .user(&spec.user)
        // libpq's default database is the user's name.
        .dbname(if spec.database.is_empty() { &spec.user } else { &spec.database })
        .application_name("Tabletist")
        .connect_timeout(Duration::from_secs(10))
        .ssl_mode(crate::tls::ssl_mode(spec.tls));
    if via.is_some() {
        config.hostaddr(std::net::IpAddr::from([127, 0, 0, 1]));
    }
    if let Some(password) = &secrets.password {
        config.password(password);
    }
    config
}
```

`Conn::connect(spec, secrets, via: Option<u16>)` keeps its user check and TLS setup and calls `config(spec, secrets, via).connect(tls.clone())`.

- [ ] **Step 4: Split out the MySQL options**

In `mysql.rs`, turn the builder and TLS into:

```rust
pub(crate) fn builder(spec: &ConnectSpec, secrets: &Secrets, via: Option<u16>) -> OptsBuilder {
    let (host, port) = match via {
        Some(port) => ("127.0.0.1".to_owned(), port),
        None => (spec.host.clone(), spec.port),
    };
    OptsBuilder::default()
        .ip_or_hostname(host)
        .tcp_port(port)
        .user(Some(spec.user.clone()))
        .pass(secrets.password.clone())
        .db_name((!spec.database.is_empty()).then(|| spec.database.clone()))
        // Never switch a "localhost" connection to the Unix socket.
        .prefer_socket(false)
}

/// The builder with TLS for the spec's mode; through a tunnel the
/// certificate is still checked against `spec.host`.
pub(crate) fn with_tls(builder: OptsBuilder, spec: &ConnectSpec, via: Option<u16>) -> OptsBuilder {
    let ssl = ssl_opts(spec.tls, spec.ca_file.as_deref()).map(|ssl| match via {
        Some(_) => ssl.with_danger_tls_hostname_override(Some(spec.host.clone())),
        None => ssl,
    });
    builder.ssl_opts(ssl)
}
```

`Conn::connect(spec, secrets, via)` uses `builder(..)` where it built one before, `with_tls(builder.clone(), spec, via).into()` for the TLS attempt, and the plain `builder` for the Prefer fallback exactly as today. The cancel connection reuses the stored `Opts`, which already point at the tunnel.

- [ ] **Step 5: Make `Connection` a struct that owns the tunnel**

In `lib.rs`:

```rust
pub struct Connection {
    inner: Inner,
    /// Declared after `inner`, so the driver closes before its tunnel.
    tunnel: Option<ssh::Tunnel>,
}

enum Inner {
    Sqlite(sqlite::Conn),
    Postgres(Box<pg::Conn>),
    MySql(Box<mysql::Conn>),
}

impl Connection {
    pub async fn connect(spec: &ConnectSpec, secrets: &Secrets) -> Result<Self> {
        Self::connect_with(spec, secrets, &HostKeys::default()).await
    }

    /// Connects, through an SSH tunnel when the spec has one; `host_keys`
    /// are the SSH host keys the user trusts.
    pub async fn connect_with(spec: &ConnectSpec, secrets: &Secrets, host_keys: &HostKeys) -> Result<Self> {
        let ssh = spec.ssh.as_ref().filter(|_| spec.driver != Driver::Sqlite);
        let Some(ssh) = ssh else {
            return Ok(Self { inner: Inner::connect(spec, secrets, None).await?, tunnel: None });
        };
        let tunnel = ssh::Tunnel::open(ssh, &spec.host, spec.port, secrets, host_keys).await?;
        match Inner::connect(spec, secrets, Some(tunnel.port)).await {
            Ok(inner) => Ok(Self { inner, tunnel: Some(tunnel) }),
            Err(error) => Err(tunnel.forward_error().unwrap_or(error)),
        }
    }

    #[doc(hidden)]
    pub fn tunnel_port(&self) -> Option<u16> {
        self.tunnel.as_ref().map(|tunnel| tunnel.port)
    }
    // driver(), dialect(), list_*, describe, fetch_rows, count_rows,
    // cancel_handle: unchanged bodies, matching on `&self.inner` with
    // `Inner::` variants. close(self): `drop(self.inner)` per variant as
    // today, then `drop(self.tunnel)`.
}

impl Inner {
    async fn connect(spec: &ConnectSpec, secrets: &Secrets, via: Option<u16>) -> Result<Self> {
        // The old Connection::connect body, passing `via` to pg and mysql.
    }
}
```

Remove `open_tunnel_for_tests`. No code outside `lib.rs` names the variants (checked: `src/` and `tests/` only call `Connection::connect`).

In `spec.rs` `summary()`, before returning `text`:

```rust
        if let Some(ssh) = &self.ssh {
            text.push_str(&format!(" via {}@{}", ssh.user, ssh.host));
        }
```

- [ ] **Step 6: Run everything**

Run: `cargo test --locked --workspace --all-targets` (PostgreSQL, MySQL and SSH variables set, containers up).
Expected: PASS, including the 11 SSH tests (agent one skipped without the agent variable).

- [ ] **Step 7: Commit**

```bash
cargo fmt --all
git add crates/tabletist-db
git commit -m "Connect PostgreSQL and MySQL through SSH tunnels owned by the connection"
```

---

### Task 3: CI for the tunnel and the agent

**Files:**
- Modify: `.github/workflows/ci.yml`, `AGENTS.md`

**Interfaces:** none (CI only).

- [ ] **Step 1: The SSH integration job**

Add after the `mysql` job:

```yaml
  ssh:
    name: ssh integration
    runs-on: ubuntu-latest
    timeout-minutes: 30
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2
      - name: Start PostgreSQL, MySQL and SSH
        run: docker compose up -d --build --wait postgres mysql ssh
      - name: Load the test key into an agent
        run: |
          chmod 600 crates/tabletist-db/tests/ssh/id_ed25519
          eval "$(ssh-agent -s)"
          ssh-add crates/tabletist-db/tests/ssh/id_ed25519
          echo "SSH_AUTH_SOCK=$SSH_AUTH_SOCK" >> "$GITHUB_ENV"
      - run: cargo test --locked -p tabletist-db --test ssh
        env:
          TABLETIST_TEST_SSH_URL: ssh://tabletist:tabletist@localhost:52222
          TABLETIST_TEST_SSH_AGENT: "1"
      - run: cargo test --locked -p tabletist-db --lib ssh::tests::the_agent_lists_its_keys -- --ignored --exact
```

- [ ] **Step 2: The Windows agent check (the spec's open risk)**

In the `test` matrix job, after the keyring round trip:

```yaml
      - name: SSH agent (Windows OpenSSH)
        if: runner.os == 'Windows'
        timeout-minutes: 5
        shell: pwsh
        run: |
          Set-Service ssh-agent -StartupType Manual
          Start-Service ssh-agent
          $key = "crates/tabletist-db/tests/ssh/id_ed25519"
          icacls $key /inheritance:r /grant:r "$($env:USERNAME):(R)"
          ssh-add $key
          cargo test --locked -p tabletist-db --lib ssh::tests::the_agent_lists_its_keys -- --ignored --exact
```

- [ ] **Step 3: AGENTS.md**

Under the integration-test section, add: how to start `ssh` (`docker compose up -d --build --wait postgres mysql ssh`), the `TABLETIST_TEST_SSH_URL` value, and that agent tests need `TABLETIST_TEST_SSH_AGENT=1` plus `ssh-add crates/tabletist-db/tests/ssh/id_ed25519` (after `chmod 600`).

- [ ] **Step 4: Verify and commit**

Run: `cargo test --locked --workspace --all-targets` locally (unchanged), then commit:

```bash
git add .github/workflows/ci.yml AGENTS.md
git commit -m "Run the SSH tunnel suite and agent checks in CI"
```

After the push, CI's `ssh integration` and Windows `SSH agent` steps are this task's real verification; ledger their result (and a ruling if Windows needs a different step).

---

### Task 4: Trusting host keys in the app

**Files:**
- Create: `src/known_hosts.rs`, `src/ui/host_key_prompt.rs`
- Modify: `src/lib.rs` (or wherever modules are declared), `src/backend.rs`, `src/model.rs`, `src/app.rs`, `src/ui/mod.rs`, `src/ui/connect_dialog.rs`

**Interfaces:**
- Consumes: `tabletist_db::{HostKeys, Connection::connect_with, SshStage}`.
- Produces: `App.host_keys: HostKeys`; `Command::Connect { .., host_keys: HostKeys }` and `Command::Test { .., host_keys: HostKeys }`; `Dialog::HostKey(Box<HostKeyPrompt>)` with `HostKeyPrompt { tab: ConnTabId, host: String, port: u16, fingerprint: String }`; `Action::TrustHostKey`, `Action::CancelHostKey`, `Action::TrustTestHostKey`; `TestState::Untrusted { host: String, port: u16, fingerprint: String }`.

- [ ] **Step 1: Write the failing reducer tests** (in `app.rs` tests)

```rust
    fn ssh_saved(app: &mut App) -> ConnectionId {
        let (mut spec, _) = ConnectSpec::from_url("postgres://me@db/app").unwrap();
        spec.ssh = Some(tabletist_db::SshSpec {
            host: "bastion".into(),
            port: 22,
            user: "ops".into(),
            auth: tabletist_db::SshAuth::Agent,
        });
        let saved = SavedConnection {
            id: ConnectionId::new(),
            name: "Prod".into(),
            color: ColorTag::Red,
            password: PasswordMode::None,
            ssh_secret: PasswordMode::None,
            spec,
        };
        let id = saved.id.clone();
        app.connections.upsert(saved);
        id
    }

    fn fail_last_connect(app: &mut App, error: tabletist_db::Error) {
        let Some(Command::Connect { session, request, .. }) = app.backend.sent.last() else {
            panic!("expected a Connect");
        };
        let (session, request) = (*session, *request);
        app.apply(Action::Backend(Event::ConnectFailed { session, request, error }));
    }

    fn unknown_key() -> tabletist_db::Error {
        tabletist_db::Error::Ssh {
            stage: SshStage::HostKeyUnknown { fingerprint: "SHA256:abc".into() },
            message: "bastion is not a trusted host yet".into(),
        }
    }

    #[test]
    fn an_unknown_host_key_asks_and_trusting_saves_it_and_reconnects() {
        let (mut app, dir) = app();
        let conn = ssh_saved(&mut app);
        let tab = app.active_tab_id();
        app.apply(Action::Connect { tab, conn });
        fail_last_connect(&mut app, unknown_key());
        match &app.dialog {
            Some(Dialog::HostKey(prompt)) => {
                assert_eq!((prompt.host.as_str(), prompt.port), ("bastion", 22));
                assert_eq!(prompt.fingerprint, "SHA256:abc");
            }
            other => panic!("{other:?}"),
        }
        app.apply(Action::TrustHostKey);
        assert!(app.dialog.is_none());
        assert_eq!(app.host_keys.fingerprint("bastion", 22), Some("SHA256:abc"));
        let saved = crate::known_hosts::load(&AppDirs::at(dir.path()).known_hosts_file());
        assert_eq!(saved.fingerprint("bastion", 22), Some("SHA256:abc"));
        match app.backend.sent.last() {
            Some(Command::Connect { host_keys, .. }) => {
                assert_eq!(host_keys.fingerprint("bastion", 22), Some("SHA256:abc"))
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn cancelling_the_host_key_prompt_trusts_nothing() {
        let (mut app, _dir) = app();
        let conn = ssh_saved(&mut app);
        let tab = app.active_tab_id();
        app.apply(Action::Connect { tab, conn });
        fail_last_connect(&mut app, unknown_key());
        let sent = app.backend.sent.len();
        app.apply(Action::CancelHostKey);
        assert!(app.dialog.is_none());
        assert_eq!(app.host_keys, HostKeys::default());
        assert_eq!(app.backend.sent.len(), sent, "no reconnect");
        assert!(matches!(app.workspace(tab).unwrap().status, SessionStatus::Disconnected(_)));
    }

    #[test]
    fn a_changed_host_key_shows_the_error_and_no_prompt() {
        let (mut app, _dir) = app();
        let conn = ssh_saved(&mut app);
        let tab = app.active_tab_id();
        app.apply(Action::Connect { tab, conn });
        fail_last_connect(
            &mut app,
            tabletist_db::Error::Ssh {
                stage: SshStage::HostKeyMismatch { fingerprint: "SHA256:new".into() },
                message: "the host key changed".into(),
            },
        );
        assert!(app.dialog.is_none());
        match &app.workspace(tab).unwrap().status {
            SessionStatus::Disconnected(error) => assert!(error.to_string().contains("changed")),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn an_unknown_host_key_during_test_offers_trust_in_the_dialog() {
        let (mut app, _dir) = app();
        postgres_form(&mut app);
        let form = form(&mut app);
        form.ssh = true;
        form.ssh_host = "bastion".into();
        form.ssh_user = "ops".into();
        form.ssh_auth = SshAuthKind::Agent;
        app.apply(Action::TestConnection);
        let Some(Command::Test { request, .. }) = app.backend.sent.last() else { panic!() };
        let request = *request;
        app.apply(Action::Backend(Event::Tested { request, result: Err(unknown_key()) }));
        assert!(matches!(form(&mut app).test, TestState::Untrusted { .. }));
        app.apply(Action::TrustTestHostKey);
        assert_eq!(app.host_keys.fingerprint("bastion", 22), Some("SHA256:abc"));
        assert!(matches!(app.backend.sent.last(), Some(Command::Test { .. })));
        assert!(matches!(form(&mut app).test, TestState::Running(_)));
    }
```

(`ssh_secret`, `form.ssh*` and `SshAuthKind` come from Tasks 5 and 6. To keep the order, add in this task the plain fields they need: `SavedConnection.ssh_secret: PasswordMode` with `#[serde(default)]`, and on `ConnectionForm` `ssh: bool`, `ssh_host: String`, `ssh_port: String` (default `"22"`), `ssh_user: String`, `ssh_auth: SshAuthKind` (`Password`, `KeyFile`, `Agent`; default `Password`), `ssh_key_file: String`, `ssh_secret: String`, `ssh_secret_mode: PasswordMode` (default `Keyring`), `has_saved_ssh_secret: bool`; and make `ConnectionForm::to_spec` build `ssh: Some(SshSpec { .. })` when `form.ssh` is set, as shown in Task 6 Step 3. Task 6 adds the UI and validation tests for them.)

- [ ] **Step 2: Run them to watch them fail**

Run: `cargo test --locked --lib host_key`
Expected: compile errors for the new names, then failures once stubs exist.

- [ ] **Step 3: The store and the plumbing**

`src/known_hosts.rs`:

```rust
//! Trusted SSH host keys, saved in config/known_hosts.json.

use std::path::Path;

use tabletist_db::HostKeys;

pub fn load(path: &Path) -> HostKeys {
    crate::util::load_json(path)
}

pub fn save(path: &Path, keys: &HostKeys) -> std::io::Result<()> {
    crate::util::save_json(path, keys)
}
```

Declare the module next to `connections`. `App` gains `pub host_keys: HostKeys`, loaded in `App::new` with `crate::known_hosts::load(&dirs.known_hosts_file())` before `dirs` moves into the struct.

`backend.rs`: add `host_keys: HostKeys` to `Command::Connect` and `Command::Test`, and call `Connection::connect_with(&spec, &secrets, &host_keys)` in both handlers. Every place that builds these commands passes `self.host_keys.clone()` (clone it before borrowing a workspace mutably). Pattern matches that use `..` are unaffected.

- [ ] **Step 4: The flow**

`model.rs`:

```rust
/// Asks whether to trust an SSH host seen for the first time.
#[derive(Debug)]
pub struct HostKeyPrompt {
    pub tab: ConnTabId,
    pub host: String,
    pub port: u16,
    pub fingerprint: String,
}
```

`Dialog::HostKey(Box<HostKeyPrompt>)`; `TestState::Untrusted { host: String, port: u16, fingerprint: String }`; actions `TrustHostKey`, `CancelHostKey`, `TrustTestHostKey`.

`app.rs`, in the `ConnectFailed` handler after `workspace.status = SessionStatus::Disconnected(error)` is set (keep the auth branch as it is):

```rust
                if let tabletist_db::Error::Ssh {
                    stage: SshStage::HostKeyUnknown { fingerprint },
                    ..
                } = &error_for_prompt
                    && self.dialog.is_none()
                    && let Some(ssh) = self.workspace(tab).and_then(|w| w.spec.ssh.clone())
                {
                    self.dialog = Some(Dialog::HostKey(Box::new(HostKeyPrompt {
                        tab,
                        host: ssh.host,
                        port: ssh.port,
                        fingerprint: fingerprint.clone(),
                    })));
                }
```

(`error_for_prompt` is a clone of `error` taken before it moves into the status.) When a dialog is open, the tab just shows the error; Reconnect asks later.

Reducer arms:

```rust
            Action::TrustHostKey => {
                if let Some(Dialog::HostKey(prompt)) = self.dialog.take() {
                    self.trust(&prompt.host, prompt.port, &prompt.fingerprint);
                    self.reconnect(prompt.tab);
                }
            }
            Action::CancelHostKey => {
                if matches!(self.dialog, Some(Dialog::HostKey(_))) {
                    self.dialog = None;
                }
            }
            Action::TrustTestHostKey => {
                let untrusted = match &self.dialog {
                    Some(Dialog::Connection(form)) => match &form.test {
                        TestState::Untrusted { host, port, fingerprint } => {
                            Some((host.clone(), *port, fingerprint.clone()))
                        }
                        _ => None,
                    },
                    _ => None,
                };
                if let Some((host, port, fingerprint)) = untrusted {
                    self.trust(&host, port, &fingerprint);
                    self.apply(Action::TestConnection);
                }
            }
```

```rust
    fn trust(&mut self, host: &str, port: u16, fingerprint: &str) {
        self.host_keys.trust(host, port, fingerprint);
        if let Err(error) = crate::known_hosts::save(&self.dirs.known_hosts_file(), &self.host_keys) {
            log::error!("could not save known hosts: {error}");
        }
    }
```

In the `Tested` handler, map `Err(Error::Ssh { stage: SshStage::HostKeyUnknown { fingerprint }, .. })` to `TestState::Untrusted { host, port, fingerprint }` using the form's `ssh_host` and parsed `ssh_port`; every other result as today.

- [ ] **Step 5: The dialog UI**

`src/ui/host_key_prompt.rs` (modal like `password_prompt.rs`): title "Trust this SSH host?"; text `"{host}:{port} has a key this computer has not seen before. Compare its fingerprint with the server's before trusting it."`; the fingerprint in `theme::mono(13.0)`, selectable; buttons **Trust and connect** (`Action::TrustHostKey`) and **Cancel** (`Action::CancelHostKey`); Escape cancels. Dispatch it in `ui/mod.rs` after `password_prompt::show`.

In `connect_dialog.rs`, in the `match &form.test` block:

```rust
            TestState::Untrusted { host, fingerprint, .. } => {
                ui.label(
                    RichText::new(format!(
                        "{} {host}: {fingerprint}",
                        gettext(locale, "Unknown SSH host key for")
                    ))
                    .color(palette.warning),
                );
                if ui.button(gettext(locale, "Trust and test")).clicked() {
                    actions.push(Action::TrustTestHostKey);
                }
            }
```

Add a headless test in `ui/mod.rs` that puts a `Dialog::HostKey` in `harness.app.dialog`, runs a frame, asserts `harness.has("SHA256:abc")`, clicks "Trust and connect", and asserts the dialog closed and `host_keys` holds the key.

- [ ] **Step 6: Run and commit**

Run: `cargo test --locked --workspace --all-targets`
Expected: PASS.

```bash
cargo fmt --all
git add src
git commit -m "Ask before trusting a new SSH host key and refuse a changed one"
```

---

### Task 5: SSH secrets: keyring, prompts, retries

**Files:**
- Modify: `src/secrets.rs`, `src/model.rs`, `src/app.rs`, `src/ui/password_prompt.rs`

**Interfaces:**
- Consumes: `SavedConnection.ssh_secret`, form SSH fields (Task 4), `SshStage::Secret` (Task 1).
- Produces: `secrets::ssh_account(&ConnectionId) -> String` (`connection/<id>/ssh-secret`); `pub enum SecretKind { Database, SshPassword, SshPassphrase }` with `SecretKind::for_ssh(&ConnectSpec) -> Option<SecretKind>` (Password auth: `SshPassword`; key file: `SshPassphrase`; agent or no SSH: `None`); `PasswordPrompt.kind: SecretKind`; `Workspace.ssh_mode: PasswordMode`, `Workspace.save_ssh: bool`, `Workspace.needs_ssh_prompt: Option<String>`.

- [ ] **Step 1: Write the failing tests** (in `app.rs` tests)

```rust
    fn ssh_password_saved(app: &mut App, mode: PasswordMode) -> ConnectionId {
        let conn = ssh_saved(app);
        let saved = app.connections.connections.iter_mut().find(|c| c.id == conn).unwrap();
        saved.spec.ssh.as_mut().unwrap().auth = tabletist_db::SshAuth::Password;
        saved.ssh_secret = mode;
        conn
    }

    fn last_connect_secrets(app: &App) -> Secrets {
        match app.backend.sent.last() {
            Some(Command::Connect { secrets, .. }) => secrets.clone(),
            other => panic!("expected a Connect, got {other:?}"),
        }
    }

    #[test]
    fn a_keyring_ssh_password_is_loaded_before_connecting() {
        let (mut app, _dir) = app();
        let conn = ssh_password_saved(&mut app, PasswordMode::Keyring);
        let tab = app.active_tab_id();
        app.apply(Action::Connect { tab, conn: conn.clone() });
        let Some(Command::LoadSecret { request, account }) = app.backend.sent.last() else { panic!() };
        assert_eq!(account, &crate::secrets::ssh_account(&conn));
        let request = *request;
        app.apply(Action::Backend(Event::SecretLoaded { request, result: Ok(Some(SecretString("s3".into()))) }));
        assert_eq!(last_connect_secrets(&app).ssh_password.as_deref(), Some("s3"));
    }

    #[test]
    fn an_ask_ssh_password_prompts_with_its_own_title() {
        let (mut app, _dir) = app();
        let conn = ssh_password_saved(&mut app, PasswordMode::Ask);
        let tab = app.active_tab_id();
        app.apply(Action::Connect { tab, conn });
        assert_eq!(prompt(&mut app).kind, SecretKind::SshPassword);
        prompt(&mut app).password = "typed".into();
        app.apply(Action::SubmitPassword);
        assert_eq!(last_connect_secrets(&app).ssh_password.as_deref(), Some("typed"));
    }

    #[test]
    fn both_secrets_are_asked_one_after_the_other() {
        let (mut app, _dir) = app();
        let conn = ssh_password_saved(&mut app, PasswordMode::Ask);
        app.connections.connections.iter_mut().find(|c| c.id == conn).unwrap().password = PasswordMode::Ask;
        let tab = app.active_tab_id();
        app.apply(Action::Connect { tab, conn });
        assert_eq!(prompt(&mut app).kind, SecretKind::Database);
        prompt(&mut app).password = "db".into();
        app.apply(Action::SubmitPassword);
        assert_eq!(prompt(&mut app).kind, SecretKind::SshPassword);
        prompt(&mut app).password = "ssh".into();
        app.apply(Action::SubmitPassword);
        let secrets = last_connect_secrets(&app);
        assert_eq!((secrets.password.as_deref(), secrets.ssh_password.as_deref()), (Some("db"), Some("ssh")));
    }

    #[test]
    fn a_rejected_ssh_secret_prompts_and_is_not_resent() {
        let (mut app, _dir) = app();
        let conn = ssh_password_saved(&mut app, PasswordMode::Ask);
        let tab = app.active_tab_id();
        app.apply(Action::Connect { tab, conn });
        prompt(&mut app).password = "wrong".into();
        app.apply(Action::SubmitPassword);
        fail_last_connect(&mut app, tabletist_db::Error::Ssh {
            stage: SshStage::Secret,
            message: "the server refused the SSH password".into(),
        });
        assert_eq!(prompt(&mut app).kind, SecretKind::SshPassword);
        assert!(prompt(&mut app).message.as_deref().unwrap().contains("refused"));
        assert_eq!(app.workspace(tab).unwrap().secrets.ssh_password, None);
    }

    #[test]
    fn an_ssh_secret_from_the_prompt_is_saved_after_connecting() {
        let (mut app, _dir) = app();
        let conn = ssh_password_saved(&mut app, PasswordMode::Ask);
        let tab = app.active_tab_id();
        app.apply(Action::Connect { tab, conn: conn.clone() });
        prompt(&mut app).password = "good".into();
        prompt(&mut app).save = true;
        app.apply(Action::SubmitPassword);
        assert!(sent_secrets(&app).is_empty(), "not before the server accepts it");
        let Some(Command::Connect { session, request, .. }) = app.backend.sent.last() else { panic!() };
        let (session, request) = (*session, *request);
        app.apply(Action::Backend(Event::Connected { session, request, driver: Driver::Postgres }));
        assert!(app.backend.sent.iter().any(|c| matches!(c,
            Command::StoreSecret { account, secret: Some(_), .. } if account == &crate::secrets::ssh_account(&conn))));
        assert_eq!(app.connections.get(&conn).unwrap().ssh_secret, PasswordMode::Keyring);
    }

    #[test]
    fn a_key_that_needs_a_passphrase_asks_for_it() {
        let (mut app, _dir) = app();
        let conn = ssh_saved(&mut app);
        app.connections.connections.iter_mut().find(|c| c.id == conn).unwrap()
            .spec.ssh.as_mut().unwrap().auth = tabletist_db::SshAuth::KeyFile { path: "/k".into() };
        let tab = app.active_tab_id();
        app.apply(Action::Connect { tab, conn });
        fail_last_connect(&mut app, tabletist_db::Error::Ssh {
            stage: SshStage::Secret,
            message: "the key file needs its passphrase".into(),
        });
        assert_eq!(prompt(&mut app).kind, SecretKind::SshPassphrase);
        prompt(&mut app).password = "pp".into();
        app.apply(Action::SubmitPassword);
        assert_eq!(last_connect_secrets(&app).ssh_passphrase.as_deref(), Some("pp"));
    }

    #[test]
    fn saving_the_form_stores_the_typed_ssh_secret_and_deleting_removes_it() {
        let (mut app, _dir) = app();
        postgres_form(&mut app);
        let form = form(&mut app);
        form.ssh = true;
        form.ssh_host = "bastion".into();
        form.ssh_user = "ops".into();
        form.ssh_auth = SshAuthKind::Password;
        form.ssh_secret = "s3".into();
        app.apply(Action::SaveConnection { connect: false });
        let id = app.connections.connections[0].id.clone();
        assert_eq!(app.connections.connections[0].ssh_secret, PasswordMode::Keyring);
        assert!(app.backend.sent.iter().any(|c| matches!(c,
            Command::StoreSecret { account, secret: Some(_), .. } if account == &crate::secrets::ssh_account(&id))));
        app.apply(Action::DeleteConnection(id.clone()));
        assert!(app.backend.sent.iter().any(|c| matches!(c,
            Command::StoreSecret { account, secret: None, .. } if account == &crate::secrets::ssh_account(&id))));
    }

    #[test]
    fn testing_loads_both_saved_secrets() {
        let (mut app, _dir) = app();
        let conn = ssh_password_saved(&mut app, PasswordMode::Keyring);
        app.connections.connections.iter_mut().find(|c| c.id == conn).unwrap().password = PasswordMode::Keyring;
        app.apply(Action::EditConnection(conn.clone()));
        app.apply(Action::TestConnection);
        let loads: Vec<_> = app.backend.sent.iter().filter_map(|c| match c {
            Command::LoadSecret { request, account } => Some((*request, account.clone())),
            _ => None,
        }).collect();
        assert_eq!(loads.len(), 2);
        for (request, account) in loads {
            let secret = if account.ends_with("ssh-secret") { "ssh" } else { "db" };
            app.apply(Action::Backend(Event::SecretLoaded { request, result: Ok(Some(SecretString(secret.into()))) }));
        }
        match app.backend.sent.last() {
            Some(Command::Test { secrets, .. }) => {
                assert_eq!((secrets.password.as_deref(), secrets.ssh_password.as_deref()), (Some("db"), Some("ssh")))
            }
            other => panic!("{other:?}"),
        }
    }
```


- [ ] **Step 2: Run them to watch them fail**

Run: `cargo test --locked --lib ssh_`
Expected: compile errors, then failures.

- [ ] **Step 3: The pieces**

`secrets.rs`:

```rust
pub fn ssh_account(id: &ConnectionId) -> String {
    format!("connection/{}/ssh-secret", id.0)
}
```

`model.rs`:

```rust
/// Which secret a prompt asks for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecretKind {
    Database,
    SshPassword,
    SshPassphrase,
}

impl SecretKind {
    /// The SSH secret this spec can use, if any.
    pub fn for_ssh(spec: &ConnectSpec) -> Option<Self> {
        match spec.ssh.as_ref()?.auth {
            SshAuth::Password => Some(Self::SshPassword),
            SshAuth::KeyFile { .. } => Some(Self::SshPassphrase),
            SshAuth::Agent => None,
        }
    }

    pub fn slot(self, secrets: &mut Secrets) -> &mut Option<String> {
        match self {
            Self::Database => &mut secrets.password,
            Self::SshPassword => &mut secrets.ssh_password,
            Self::SshPassphrase => &mut secrets.ssh_passphrase,
        }
    }
}
```

`PasswordPrompt` gains `kind: SecretKind`; `Workspace` gains `ssh_mode`, `save_ssh`, `needs_ssh_prompt` (initialized in `connect_tab` from `saved.ssh_secret`, `false`, `None`). `SecretPurpose` becomes `Connect(ConnTabId, SecretKind)` and `Test(SecretKind)`.

- [ ] **Step 4: Gather secrets one at a time**

Replace `authenticate` with:

```rust
    /// Fills the tab's secrets one at a time (keyring, else prompt), then
    /// connects. An empty answer is kept as `Some("")` so it is not asked
    /// again; `send_connect` sends it as no password.
    fn authenticate(&mut self, tab: ConnTabId) {
        let Some(workspace) = self.workspace(tab) else {
            return;
        };
        if let Some(reason) = workspace.needs_prompt.clone() {
            self.prompt_password(tab, SecretKind::Database, Some(reason));
            return;
        }
        let steps = [
            (SecretKind::Database, workspace.password_mode, workspace.secrets.password.is_some()),
        ]
        .into_iter()
        .chain(SecretKind::for_ssh(&workspace.spec).map(|kind| {
            (kind, workspace.ssh_mode, kind.slot(&mut workspace.secrets.clone()).is_some())
        }));
        let ssh_reason = workspace.needs_ssh_prompt.clone();
        let conn = workspace.conn_id.clone();
        for (kind, mode, known) in steps.collect::<Vec<_>>() {
            if kind != SecretKind::Database && let Some(reason) = ssh_reason.clone() {
                self.prompt_password(tab, kind, Some(reason));
                return;
            }
            if known || mode == PasswordMode::None {
                continue;
            }
            match mode {
                PasswordMode::Keyring => {
                    let account = if kind == SecretKind::Database {
                        password_account(&conn)
                    } else {
                        ssh_account(&conn)
                    };
                    let request = RequestId(self.next_id());
                    self.pending_secrets.insert(request, SecretPurpose::Connect(tab, kind));
                    self.backend.send(Command::LoadSecret { request, account });
                }
                _ => self.prompt_password(tab, kind, None),
            }
            return;
        }
        let secrets = self.workspace(tab).map(|w| w.secrets.clone()).unwrap_or_default();
        self.send_connect(tab, secrets);
    }
```

`send_connect` stores `secrets` in the workspace as given and sends a copy with every `Some("")` turned into `None`. `SecretLoaded` for `Connect(tab, kind)`: `Ok(Some(secret))` fills `kind.slot(&mut workspace.secrets)` and calls `authenticate(tab)`; `Ok(None)` and `Err` prompt for `kind` with today's messages. `connect_tab` with a typed password fills `secrets.password` and calls `authenticate` (it then continues to the SSH secret). `reconnect` calls `authenticate` whenever a needed secret is missing (the `known` check covers both secrets).

`prompt_password(tab, kind, message)` sets `kind` and `save: mode == Keyring` for that kind's mode. `SubmitPassword`: take the prompt, write the answer (possibly empty) into `kind.slot(&mut workspace.secrets)`, set `save_password` or `save_ssh` (only when the answer is not empty), clear that kind's `needs_*`, `ensure_connecting(tab)`, then `authenticate(tab)`. `CancelPassword` unchanged.

`ConnectFailed`: keep the `Auth` branch for the database password. Add: `Error::Ssh { stage: SshStage::Secret, .. }` clears the SSH slots of `workspace.secrets`, sets `save_ssh = false` and `needs_ssh_prompt = Some(message)`, then prompts for `SecretKind::for_ssh(&spec)` (when a dialog is open it waits for Reconnect, as the database prompt does).

`save_accepted_password` also handles `save_ssh`: store the SSH slot's value under `ssh_account`, set `workspace.ssh_mode` and the saved connection's `ssh_secret` to `Keyring`, save connections.

- [ ] **Step 5: Save, delete, test**

`save_dialog`: after the database password handling, the same for the SSH secret with `ssh_account`, `form.ssh_secret`, `saved.ssh_secret` and the previous `ssh_secret`. `to_saved` (model.rs) sets `ssh_secret`:

```rust
        let ssh_secret = match (&spec.ssh, self.ssh_auth) {
            (None, _) | (Some(_), SshAuthKind::Agent) => PasswordMode::None,
            // Nothing typed and nothing saved: an unencrypted key, or ask
            // for the password when connecting.
            _ if self.ssh_secret_mode == PasswordMode::Keyring
                && self.ssh_secret.is_empty()
                && !self.has_saved_ssh_secret =>
            {
                PasswordMode::None
            }
            _ => self.ssh_secret_mode,
        };
```

`DeleteConnection` also deletes `ssh_account` when `removed.ssh_secret == Keyring`.

`TestConnection`: build `form.test_secrets` from the typed fields (the SSH value goes into `SecretKind::for_ssh(&spec)`'s slot); for each blank field whose secret is saved, send `LoadSecret` with `SecretPurpose::Test(kind)` and count it in `form.test_waiting`; with none pending, send `Test` at once. `SecretLoaded` for `Test(kind)` fills the slot, decrements `test_waiting`, and sends `Test` at zero (a missing saved secret fails the test with today's message). Add `test_secrets: Secrets` and `test_waiting: u8` to `ConnectionForm`.

- [ ] **Step 6: Prompt title**

`password_prompt.rs`: the title is `"Password for"`, `"SSH password for"` or `"Key passphrase for"` by `prompt.kind`, followed by the name.

- [ ] **Step 7: Run and commit**

Run: `cargo test --locked --workspace --all-targets`
Expected: PASS, including every earlier password test (`an_ask_connection_prompts_and_submitting_connects`, `a_blank_answer_in_the_prompt_connects_without_a_password`, `saving_from_the_prompt_stores_the_password_and_switches_to_keyring`).

```bash
cargo fmt --all
git add src
git commit -m "Keep SSH passwords and passphrases in the keyring or ask for them"
```

---

### Task 6: The SSH section in the connection dialog

**Files:**
- Modify: `src/model.rs`, `src/app.rs`, `src/backend.rs`, `src/ui/connect_dialog.rs`, `src/ui/workspace.rs`, `src/ui/mod.rs`

**Interfaces:**
- Consumes: form SSH fields (Task 4), `SecretKind` (Task 5).
- Produces: `Action::PickKeyFile`; `ConnectionForm.pick_target: PickTarget` (`Sqlite`, `KeyFile`); `Backend::pick_key_file(request)`.

- [ ] **Step 1: Write the failing tests**

`model.rs` tests:

```rust
    fn ssh_form() -> ConnectionForm {
        ConnectionForm {
            name: "Prod".into(),
            driver: Driver::Postgres,
            host: "db.internal".into(),
            port: "5432".into(),
            user: "me".into(),
            ssh: true,
            ssh_host: "bastion".into(),
            ssh_port: "22".into(),
            ssh_user: "ops".into(),
            ssh_auth: SshAuthKind::KeyFile,
            ssh_key_file: "/home/me/.ssh/id_ed25519".into(),
            ..ConnectionForm::default()
        }
    }

    #[test]
    fn an_ssh_form_becomes_a_spec_and_back() {
        let saved = ssh_form().to_saved().unwrap();
        assert_eq!(
            saved.spec.ssh,
            Some(SshSpec {
                host: "bastion".into(),
                port: 22,
                user: "ops".into(),
                auth: SshAuth::KeyFile { path: "/home/me/.ssh/id_ed25519".into() },
            })
        );
        assert_eq!(saved.ssh_secret, PasswordMode::None, "unencrypted key, nothing typed");
        let back = ConnectionForm::from_saved(&saved);
        assert!(back.ssh);
        assert_eq!(back.ssh_key_file, "/home/me/.ssh/id_ed25519");
        assert_eq!(back.ssh_auth, SshAuthKind::KeyFile);
    }

    #[test]
    fn an_ssh_form_needs_host_user_port_and_key_file() {
        for (change, message) in [
            ((|f: &mut ConnectionForm| f.ssh_host.clear()) as fn(&mut ConnectionForm), "SSH host"),
            (|f| f.ssh_user.clear(), "SSH user"),
            (|f| f.ssh_port = "twenty-two".into(), "SSH port"),
            (|f| f.ssh_key_file.clear(), "key file"),
        ] {
            let mut form = ssh_form();
            change(&mut form);
            let error = form.to_spec().unwrap_err();
            assert!(error.contains(message), "{error}");
        }
    }

    #[test]
    fn unticking_ssh_drops_the_tunnel() {
        let mut form = ssh_form();
        form.ssh = false;
        assert_eq!(form.to_spec().unwrap().ssh, None);
    }
```

`ui/mod.rs` tests:

```rust
    #[test]
    fn the_ssh_section_shows_the_fields_for_each_method() {
        let mut harness = Harness::new();
        harness.press(Key::N, Modifiers::COMMAND);
        harness.click("PostgreSQL");
        harness.click("SSH tunnel");
        harness.click("Connect through SSH");
        for label in ["SSH host", "SSH port", "SSH user", "Authentication", "SSH password"] {
            assert!(harness.has(label), "{label}");
        }
        harness.click("Authentication");
        harness.click("Key file");
        assert!(harness.has("Key file"));
        assert!(harness.has("Passphrase"));
        assert!(!harness.has("SSH password"));
    }

    #[test]
    fn sqlite_has_no_ssh_section() {
        let mut harness = Harness::new();
        harness.press(Key::N, Modifiers::COMMAND);
        assert!(!harness.has("SSH tunnel"));
    }
```

In `app.rs` tests: picking a key file fills `ssh_key_file`, not `sqlite_path`:

```rust
    #[test]
    fn a_picked_key_file_fills_the_ssh_field() {
        let (mut app, _dir) = app();
        postgres_form(&mut app);
        app.apply(Action::PickKeyFile);
        let request = form(&mut app).pick_request.unwrap();
        app.apply(Action::Backend(Event::FilePicked { request, path: Some("/k/id".into()) }));
        assert_eq!(form(&mut app).ssh_key_file, "/k/id");
        assert_eq!(form(&mut app).sqlite_path, "");
    }
```

- [ ] **Step 2: Run them to watch them fail**

Run: `cargo test --locked --lib ssh_form ssh_section key_file`
Expected: failures (no validation, no section, no `PickKeyFile`).

- [ ] **Step 3: Form to spec and back**

In `to_spec`, for PostgreSQL and MySQL, build the SSH part before the `Ok(ConnectSpec { .. })` and use it for `ssh`:

```rust
                let ssh = if self.ssh {
                    let host = self.ssh_host.trim();
                    if host.is_empty() {
                        return Err("Enter the SSH host.".into());
                    }
                    let port: u16 = self.ssh_port.trim().parse()
                        .map_err(|_| "Enter the SSH port number.".to_owned())?;
                    let user = self.ssh_user.trim();
                    if user.is_empty() {
                        return Err("Enter the SSH user.".into());
                    }
                    let auth = match self.ssh_auth {
                        SshAuthKind::Password => SshAuth::Password,
                        SshAuthKind::Agent => SshAuth::Agent,
                        SshAuthKind::KeyFile => {
                            let path = self.ssh_key_file.trim();
                            if path.is_empty() {
                                return Err("Choose a key file for SSH.".into());
                            }
                            SshAuth::KeyFile { path: path.into() }
                        }
                    };
                    Some(SshSpec { host: host.to_owned(), port, user: user.to_owned(), auth })
                } else {
                    None
                };
```

`from_saved` fills `ssh`, `ssh_host`, `ssh_port`, `ssh_user`, `ssh_auth`, `ssh_key_file` from `spec.ssh`, `ssh_secret_mode` from `saved.ssh_secret` (keeping `Keyring` when it is `None`, as the database password does), and `has_saved_ssh_secret: saved.ssh_secret == PasswordMode::Keyring`.

- [ ] **Step 4: The key file picker**

`backend.rs`: `pick_key_file(request)` like `pick_sqlite_file`, titled "Choose an SSH private key", starting in `~/.ssh` when it exists (`directories::BaseDirs::new().map(|d| d.home_dir().join(".ssh"))`, already a dependency), no extension filter. `Action::PickKeyFile` sets `form.pick_target = PickTarget::KeyFile` and `pick_request`; `PickSqliteFile` sets `PickTarget::Sqlite`. `FilePicked` writes into `sqlite_path` (and names the connection, as today) or `ssh_key_file` by `pick_target`.

- [ ] **Step 5: The section**

In `connect_dialog.rs`, after the grid for PostgreSQL and MySQL (not SQLite):

```rust
        if form.driver != Driver::Sqlite {
            egui::CollapsingHeader::new(gettext(locale, "SSH tunnel"))
                .default_open(form.ssh)
                .show(ui, |ui| {
                    ui.checkbox(&mut form.ssh, gettext(locale, "Connect through SSH"));
                    ui.add_enabled_ui(form.ssh, |ui| {
                        egui::Grid::new("ssh-form").num_columns(2).spacing([12.0, 8.0]).show(ui, |ui| {
                            ui.label(gettext(locale, "SSH host"));
                            ui.horizontal(|ui| {
                                ui.add(TextEdit::singleline(&mut form.ssh_host).desired_width(250.0));
                                ui.label(gettext(locale, "SSH port"));
                                ui.add(TextEdit::singleline(&mut form.ssh_port).desired_width(60.0));
                            });
                            ui.end_row();
                            ui.label(gettext(locale, "SSH user"));
                            ui.add(TextEdit::singleline(&mut form.ssh_user).desired_width(f32::INFINITY));
                            ui.end_row();
                            ui.label(gettext(locale, "Authentication"));
                            egui::ComboBox::from_id_salt("ssh-auth")
                                .selected_text(form.ssh_auth.label())
                                .show_ui(ui, |ui| {
                                    for kind in SshAuthKind::ALL {
                                        ui.selectable_value(&mut form.ssh_auth, kind, kind.label());
                                    }
                                });
                            ui.end_row();
                            if form.ssh_auth == SshAuthKind::KeyFile {
                                ui.label(gettext(locale, "Key file"));
                                ui.horizontal(|ui| {
                                    ui.add(TextEdit::singleline(&mut form.ssh_key_file).desired_width(300.0));
                                    if ui.button(gettext(locale, "Choose…")).clicked() {
                                        actions.push(Action::PickKeyFile);
                                    }
                                });
                                ui.end_row();
                            }
                            if form.ssh_auth != SshAuthKind::Agent {
                                let label = if form.ssh_auth == SshAuthKind::Password {
                                    gettext(locale, "SSH password")
                                } else {
                                    gettext(locale, "Passphrase")
                                };
                                ui.label(label);
                                let hint = if form.has_saved_ssh_secret {
                                    gettext(locale, "Saved in keyring")
                                } else {
                                    gettext(locale, "")
                                };
                                ui.add(TextEdit::singleline(&mut form.ssh_secret).password(true)
                                    .hint_text(hint).desired_width(f32::INFINITY));
                                ui.end_row();
                                ui.label("");
                                egui::ComboBox::from_id_salt("ssh-secret-mode")
                                    .selected_text(match form.ssh_secret_mode {
                                        PasswordMode::Ask => gettext(locale, "Ask every time"),
                                        _ => gettext(locale, "Save in keyring"),
                                    })
                                    .show_ui(ui, |ui| {
                                        ui.selectable_value(&mut form.ssh_secret_mode, PasswordMode::Keyring, gettext(locale, "Save in keyring"));
                                        ui.selectable_value(&mut form.ssh_secret_mode, PasswordMode::Ask, gettext(locale, "Ask every time"));
                                    });
                                ui.end_row();
                            }
                        });
                    });
                });
        }
```

`SshAuthKind` gets `ALL` and `label()` ("Password", "Key file", "Agent"). If the headless test cannot click the combo item by label, drive it the way the TLS combo tests do and ledger the ruling.

- [ ] **Step 6: The top bar**

In `workspace.rs` `top_bar`, next to the TLS label: when `workspace.spec.ssh` is `Some(ssh)`, a label `format!("{} {}", gettext(locale, "via SSH"), ssh.host)` in `palette.secondary`. The picker and top bar summaries already say `via user@host` (Task 2).

- [ ] **Step 7: Run and commit**

Run: `cargo test --locked --workspace --all-targets` and `cargo clippy --locked --workspace --all-targets -- -D warnings`
Expected: PASS, no warnings.

```bash
cargo fmt --all
git add src
git commit -m "Add the SSH tunnel section to the connection dialog"
```

- [ ] **Step 8: By hand (when a display is available)**

`cargo run`, new PostgreSQL connection to `postgres:5432` user `tabletist` via SSH `localhost:52222` user `tabletist` with password `tabletist`: the trust prompt shows the fingerprint (compare with `docker compose exec ssh ssh-keygen -lf /etc/ssh/ssh_host_ed25519_key.pub`), then the tree loads. Ledger it as not done if there is no display.
