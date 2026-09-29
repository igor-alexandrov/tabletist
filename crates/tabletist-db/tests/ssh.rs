//! SSH tunnel tests against the compose `ssh` service. Set
//! TABLETIST_TEST_SSH_URL=ssh://tabletist:tabletist@localhost:52222 to run.

#![allow(clippy::unwrap_used)]

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use tabletist_db::{
    ConnectSpec, Connection, Driver, Error, HostKeys, ObjectRef, RowQuery, Secrets, SshAuth,
    SshSpec, SshStage, TlsMode,
};

/// The SSH server and its password, or `None` (test skipped).
fn server() -> Option<(SshSpec, String)> {
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

fn key(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/ssh")
        .join(name)
}

fn with_password(password: &str) -> Secrets {
    Secrets {
        ssh_password: Some(password.into()),
        ..Secrets::default()
    }
}

/// PostgreSQL in the compose network, as the SSH server sees it.
fn pg_via(ssh: &SshSpec) -> ConnectSpec {
    let (mut spec, _) =
        ConnectSpec::from_url("postgres://tabletist@postgres:5432/tabletist").unwrap();
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

/// Connects to PostgreSQL through the tunnel.
async fn open(ssh: &SshSpec, secrets: &Secrets, keys: &HostKeys) -> Result<Connection, Error> {
    Connection::connect_with(&pg_via(ssh), &secrets_with_db(secrets.clone()), keys).await
}

/// Learns the server's fingerprint the way the app does: from the refusal.
async fn trusted(ssh: &SshSpec) -> HostKeys {
    let Err(error) = open(ssh, &Secrets::default(), &HostKeys::default()).await else {
        panic!("the host key was trusted before it was learned");
    };
    let Error::Ssh {
        stage: SshStage::HostKeyUnknown { fingerprint },
        ..
    } = error
    else {
        panic!("{error:?}");
    };
    let mut keys = HostKeys::default();
    keys.trust(&ssh.host, ssh.port, &fingerprint);
    keys
}

#[tokio::test]
async fn an_unknown_host_key_is_reported_with_its_fingerprint() {
    let Some((ssh, password)) = server() else {
        return;
    };
    match open(&ssh, &with_password(&password), &HostKeys::default()).await {
        Err(Error::Ssh {
            stage: SshStage::HostKeyUnknown { fingerprint },
            ..
        }) => assert!(fingerprint.starts_with("SHA256:"), "{fingerprint}"),
        other => panic!("{:?}", other.err()),
    }
}

#[tokio::test]
async fn a_changed_host_key_is_refused() {
    let Some((ssh, password)) = server() else {
        return;
    };
    let mut keys = HostKeys::default();
    keys.trust(&ssh.host, ssh.port, "SHA256:not-the-servers-key");
    match open(&ssh, &with_password(&password), &keys).await {
        Err(Error::Ssh {
            stage: SshStage::HostKeyMismatch { fingerprint },
            message,
        }) => {
            assert!(fingerprint.starts_with("SHA256:"));
            assert!(message.contains("known_hosts.json"), "{message}");
        }
        other => panic!("{:?}", other.err()),
    }
}

#[tokio::test]
async fn a_password_opens_the_tunnel_and_a_wrong_one_asks_again() {
    let Some((ssh, password)) = server() else {
        return;
    };
    let keys = trusted(&ssh).await;
    let opened = open(&ssh, &with_password(&password), &keys).await;
    assert!(opened.is_ok(), "{:?}", opened.err());
    for secrets in [with_password("wrong"), Secrets::default()] {
        match open(&ssh, &secrets, &keys).await {
            Err(Error::Ssh {
                stage: SshStage::Secret,
                ..
            }) => {}
            other => panic!("{:?}", other.err()),
        }
    }
}

#[tokio::test]
async fn key_files_open_the_tunnel_with_their_passphrase() {
    let Some((mut ssh, _)) = server() else {
        return;
    };
    let keys = trusted(&ssh).await;
    ssh.auth = SshAuth::KeyFile {
        path: key("id_ed25519"),
    };
    let opened = open(&ssh, &Secrets::default(), &keys).await;
    assert!(opened.is_ok(), "{:?}", opened.err());
    ssh.auth = SshAuth::KeyFile {
        path: key("id_ed25519_enc"),
    };
    match open(&ssh, &Secrets::default(), &keys).await {
        Err(Error::Ssh {
            stage: SshStage::Secret,
            ..
        }) => {}
        other => panic!("{:?}", other.err()),
    }
    let secrets = Secrets {
        ssh_passphrase: Some("tabletist".into()),
        ..Secrets::default()
    };
    let opened = open(&ssh, &secrets, &keys).await;
    assert!(opened.is_ok(), "{:?}", opened.err());
}

#[tokio::test]
async fn the_agent_opens_the_tunnel() {
    let Some((mut ssh, _)) = server() else {
        return;
    };
    if std::env::var("TABLETIST_TEST_SSH_AGENT").is_err() {
        eprintln!("skipped: TABLETIST_TEST_SSH_AGENT is not set");
        return;
    }
    let keys = trusted(&ssh).await;
    ssh.auth = SshAuth::Agent;
    let opened = open(&ssh, &Secrets::default(), &keys).await;
    assert!(opened.is_ok(), "{:?}", opened.err());
}

#[tokio::test]
async fn postgres_through_the_tunnel_lists_schemas() {
    let Some((ssh, password)) = server() else {
        return;
    };
    let keys = trusted(&ssh).await;
    let connection = open(&ssh, &with_password(&password), &keys).await.unwrap();
    let schemas = connection.list_schemas().await.unwrap();
    assert!(schemas.contains(&"public".to_owned()), "{schemas:?}");
}

#[tokio::test]
async fn postgres_tls_works_through_the_tunnel() {
    let Some((ssh, password)) = server() else {
        return;
    };
    let keys = trusted(&ssh).await;
    let mut spec = pg_via(&ssh);
    spec.tls = TlsMode::Require;
    let connection =
        Connection::connect_with(&spec, &secrets_with_db(with_password(&password)), &keys).await;
    assert!(connection.is_ok(), "{:?}", connection.err());
}

#[tokio::test]
async fn a_database_the_ssh_server_cannot_reach_is_a_forward_error() {
    let Some((ssh, password)) = server() else {
        return;
    };
    let keys = trusted(&ssh).await;
    let mut spec = pg_via(&ssh);
    spec.host = "nowhere.invalid".into();
    match Connection::connect_with(&spec, &secrets_with_db(with_password(&password)), &keys).await {
        Err(Error::Ssh {
            stage: SshStage::Forward,
            message,
        }) => assert!(message.contains("nowhere.invalid"), "{message}"),
        other => panic!("{:?}", other.err()),
    }
}

/// Cancels `query` on `connection` until it stops, and returns its result.
async fn cancel_until_done(
    connection: Arc<Connection>,
    query: RowQuery,
) -> Result<tabletist_db::RowPage, Error> {
    let cancel = connection.cancel_handle();
    let running = {
        let connection = Arc::clone(&connection);
        tokio::spawn(async move { connection.fetch_rows(&query).await })
    };
    let deadline = Instant::now() + Duration::from_secs(20);
    while !running.is_finished() {
        assert!(Instant::now() < deadline, "cancel must stop the query");
        tokio::time::sleep(Duration::from_millis(300)).await;
        cancel.cancel().await.unwrap();
    }
    running.await.unwrap()
}

#[tokio::test]
async fn postgres_cancel_goes_through_the_tunnel() {
    let Some((ssh, password)) = server() else {
        return;
    };
    let keys = trusted(&ssh).await;
    let connection = Arc::new(open(&ssh, &with_password(&password), &keys).await.unwrap());
    let mut query = RowQuery::new(ObjectRef::new("pg_catalog", "pg_namespace"), 5);
    query.raw_where = Some("pg_sleep(30) IS NOT NULL".into());
    assert_eq!(
        cancel_until_done(connection, query).await.err(),
        Some(Error::Cancelled)
    );
}

#[tokio::test]
async fn mysql_through_the_tunnel_reads_and_cancels() {
    let Some((ssh, password)) = server() else {
        return;
    };
    let keys = trusted(&ssh).await;
    let connection = Connection::connect_with(
        &mysql_via(&ssh),
        &secrets_with_db(with_password(&password)),
        &keys,
    )
    .await
    .unwrap();
    assert_eq!(connection.driver(), Driver::MySql);
    let schemas = connection.list_schemas().await.unwrap();
    assert!(schemas.contains(&"tabletist".to_owned()), "{schemas:?}");
    let mut query = RowQuery::new(ObjectRef::new("information_schema", "CHARACTER_SETS"), 5);
    query.raw_where = Some("SLEEP(30) = 0".into());
    assert!(
        cancel_until_done(Arc::new(connection), query)
            .await
            .is_err()
    );
}

#[tokio::test]
async fn closing_the_connection_closes_the_tunnel() {
    let Some((ssh, password)) = server() else {
        return;
    };
    let keys = trusted(&ssh).await;
    let connection = open(&ssh, &with_password(&password), &keys).await.unwrap();
    let port = connection.tunnel_port().expect("a tunnel");
    connection.close().await.unwrap();
    tokio::time::sleep(Duration::from_millis(200)).await;
    assert!(
        tokio::net::TcpStream::connect(("127.0.0.1", port))
            .await
            .is_err()
    );
}

#[tokio::test]
async fn a_database_host_that_never_answers_times_out_like_a_direct_connect() {
    let Some((ssh, password)) = server() else {
        return;
    };
    let keys = trusted(&ssh).await;
    for mut spec in [pg_via(&ssh), mysql_via(&ssh)] {
        // Routable nowhere: the SSH server's own connect hangs.
        spec.host = "10.255.255.1".into();
        let started = Instant::now();
        let result =
            Connection::connect_with(&spec, &secrets_with_db(with_password(&password)), &keys)
                .await;
        assert!(result.is_err());
        assert!(
            started.elapsed() < Duration::from_secs(15),
            "{:?} took {:?}",
            spec.driver,
            started.elapsed()
        );
    }
}
