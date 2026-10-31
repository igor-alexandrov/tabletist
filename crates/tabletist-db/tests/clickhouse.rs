//! ClickHouse's HTTP interface against a server. Needs one: run
//! `docker compose up -d clickhouse` and set TABLETIST_TEST_CLICKHOUSE_URL,
//! and TABLETIST_TEST_CLICKHOUSE_HTTPS_URL for TLS (see compose.yaml).
//! Without them every test prints "skipped" and passes.

// `allow-unwrap-in-tests` does not cover helpers in an integration-test crate.
#![allow(clippy::unwrap_used)]

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use tabletist_db::clickhouse::{Endpoint, Login, Session};
use tabletist_db::{Error, TlsMode, Value};

/// The endpoint and login from `http(s)://user:password@host:port/database`.
fn from_url(variable: &str, tls: TlsMode) -> Option<(Endpoint, Login)> {
    let Some(url) = std::env::var(variable)
        .ok()
        .filter(|url| !url.trim().is_empty())
    else {
        eprintln!("skipped: {variable} is not set");
        return None;
    };
    let url = url::Url::parse(&url).unwrap();
    Some((
        Endpoint {
            host: url.host_str().unwrap().to_owned(),
            port: url.port().unwrap(),
            via: None,
            tls,
            ca_file: None,
        },
        Login {
            user: url.username().to_owned(),
            password: url.password().unwrap_or_default().to_owned(),
            database: url.path().trim_start_matches('/').to_owned(),
        },
    ))
}

fn plain() -> Option<(Endpoint, Login)> {
    from_url("TABLETIST_TEST_CLICKHOUSE_URL", TlsMode::Disable)
}

fn https(tls: TlsMode) -> Option<(Endpoint, Login)> {
    from_url("TABLETIST_TEST_CLICKHOUSE_HTTPS_URL", tls)
}

/// The CA that signed the compose server's certificate.
fn ca_file() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../compose/clickhouse/ca.pem")
}

async fn session() -> Option<Session> {
    let (endpoint, login) = plain()?;
    Some(Session::connect(endpoint, login).await.unwrap())
}

/// Cancels until `running` ends, which a first cancel can miss when the
/// statement has not started yet.
async fn cancel_until_done(
    session: &Session,
    running: tokio::task::JoinHandle<tabletist_db::Result<tabletist_db::RowPage>>,
) -> tabletist_db::Result<tabletist_db::RowPage> {
    let canceller = session.canceller();
    tokio::time::timeout(Duration::from_secs(30), async {
        loop {
            tokio::time::sleep(Duration::from_millis(200)).await;
            canceller.cancel().await.unwrap();
            if running.is_finished() {
                return running.await.unwrap();
            }
        }
    })
    .await
    .expect("the query was not cancelled")
}

#[tokio::test]
async fn the_server_says_its_version() {
    let Some(session) = session().await else {
        return;
    };
    let version = session.server_version().await.unwrap();
    assert!(
        version.chars().next().unwrap().is_ascii_digit(),
        "{version}"
    );
    assert!(!session.is_encrypted());
}

#[tokio::test]
async fn a_page_keeps_its_rows_and_says_whether_more_follow() {
    let Some(session) = session().await else {
        return;
    };
    // A comparison is a UInt8 in ClickHouse: `toBool` makes it a Bool.
    let sql = "SELECT number, toString(number) AS s, toBool(number % 2 = 0) AS even \
               FROM numbers(1000)";
    let page = session.select_page(sql, 10).await.unwrap();
    let names: Vec<_> = page.columns.iter().map(|c| c.name.as_str()).collect();
    assert_eq!(names, ["number", "s", "even"]);
    assert_eq!(page.columns[0].type_name, "UInt64");
    assert_eq!(page.rows.len(), 10);
    assert!(page.has_more);
    assert_eq!(
        page.rows[3],
        [Value::Int(3), Value::Text("3".into()), Value::Bool(false)]
    );

    let page = session.select_page(sql, 5000).await.unwrap();
    assert_eq!(page.rows.len(), 1000);
    assert!(!page.has_more);
}

#[tokio::test]
async fn a_page_stopped_early_leaves_the_session_working() {
    let Some(session) = session().await else {
        return;
    };
    // Each page drops its connection mid-answer; the next statement opens
    // or reuses another.
    for _ in 0..3 {
        let page = session
            .select_page("SELECT number FROM system.numbers", 5)
            .await
            .unwrap();
        assert_eq!(page.rows.len(), 5);
        assert!(!session.server_version().await.unwrap().is_empty());
    }
}

#[tokio::test]
async fn a_session_idle_past_the_servers_keep_alive_still_works() {
    let Some(session) = session().await else {
        return;
    };
    assert!(!session.server_version().await.unwrap().is_empty());
    // The server closes a connection idle past its `keep_alive_timeout`
    // (25.8 says 10 seconds in its `Keep-Alive` header; another image in
    // compose.yaml may need a longer wait here).
    tokio::time::sleep(Duration::from_secs(11)).await;
    assert!(!session.server_version().await.unwrap().is_empty());
}

#[tokio::test]
async fn a_read_cannot_write() {
    let Some(session) = session().await else {
        return;
    };
    let error = session
        .select_page("CREATE TABLE spike (a Int8) ENGINE = Memory", 10)
        .await
        .unwrap_err();
    let Error::Query { code, .. } = error else {
        panic!("{error:?}");
    };
    // READONLY
    assert_eq!(code.as_deref(), Some("164"));
}

#[tokio::test]
async fn an_error_after_rows_were_sent_is_not_lost() {
    let Some(session) = session().await else {
        return;
    };
    let sql = "SELECT number, throwIf(number = 150000, 'boom') FROM system.numbers \
               LIMIT 200000 SETTINGS max_block_size = 1000";
    let error = session.select_page(sql, 200_000).await.unwrap_err();
    let Error::Query { code, message, .. } = error else {
        panic!("{error:?}");
    };
    // FUNCTION_THROW_IF_VALUE_IS_NON_ZERO
    assert_eq!(code.as_deref(), Some("395"));
    assert!(message.contains("boom"), "{message}");
    // The session still works after the server closed that connection.
    assert!(!session.server_version().await.unwrap().is_empty());
}

#[tokio::test]
async fn a_running_query_is_cancelled() {
    let Some(session) = session().await else {
        return;
    };
    let session = Arc::new(session);
    let slow = Arc::clone(&session);
    let running = tokio::spawn(async move {
        slow.select_page("SELECT sum(number) FROM numbers(100000000000)", 1)
            .await
    });
    let outcome = cancel_until_done(&session, running).await;
    assert_eq!(outcome.unwrap_err(), Error::Cancelled);
}

#[tokio::test]
async fn a_session_runs_one_statement_at_a_time_so_cancel_finds_it() {
    let Some(session) = session().await else {
        return;
    };
    let session = Arc::new(session);
    let slow = Arc::clone(&session);
    let running = tokio::spawn(async move {
        slow.select_page("SELECT sum(number) FROM numbers(100000000000)", 1)
            .await
    });
    // Run beside the slow one, a quick statement must not leave the
    // canceller with nothing to stop once it is done.
    let (quick, slow) = tokio::join!(
        session.select_page("SELECT 1", 1),
        cancel_until_done(&session, running)
    );
    assert_eq!(slow.unwrap_err(), Error::Cancelled);
    // Cancelled too if it ran first, while the canceller kept cancelling.
    assert!(matches!(quick, Ok(_) | Err(Error::Cancelled)), "{quick:?}");
}

#[tokio::test]
async fn a_wrong_password_is_an_authentication_error() {
    let Some((endpoint, mut login)) = plain() else {
        return;
    };
    login.password = "not-the-password".into();
    let session = Session::connect(endpoint, login).await.unwrap();
    let error = session.server_version().await.unwrap_err();
    assert!(matches!(error, Error::Auth(_)), "{error:?}");
}

#[tokio::test]
async fn prefer_takes_tls_where_the_port_has_it_and_plain_http_where_not() {
    let (Some((tls_port, login)), Some((mut plain_port, _))) = (https(TlsMode::Prefer), plain())
    else {
        return;
    };
    let session = Session::connect(tls_port, login.clone()).await.unwrap();
    assert!(session.is_encrypted());
    assert!(!session.server_version().await.unwrap().is_empty());

    plain_port.tls = TlsMode::Prefer;
    let session = Session::connect(plain_port, login).await.unwrap();
    assert!(!session.is_encrypted());
    assert!(!session.server_version().await.unwrap().is_empty());
}

#[tokio::test]
async fn require_refuses_a_port_without_tls() {
    let Some((mut endpoint, login)) = plain() else {
        return;
    };
    endpoint.tls = TlsMode::Require;
    let error = Session::connect(endpoint, login).await.err().unwrap();
    assert!(matches!(error, Error::Tls(_)), "{error:?}");
}

#[tokio::test]
async fn verify_full_checks_the_chain_and_the_name() {
    let Some((mut endpoint, login)) = https(TlsMode::VerifyFull) else {
        return;
    };
    // The system's roots do not know the test CA.
    let error = Session::connect(endpoint.clone(), login.clone())
        .await
        .err()
        .unwrap();
    assert!(matches!(error, Error::Tls(_)), "{error:?}");

    endpoint.ca_file = Some(ca_file());
    let session = Session::connect(endpoint.clone(), login.clone())
        .await
        .unwrap();
    assert!(session.is_encrypted());
    assert!(!session.server_version().await.unwrap().is_empty());

    // Reached under another name (as through a tunnel), the certificate
    // does not match: verify-full refuses, verify-ca does not check names.
    endpoint.via = Some(endpoint.port);
    endpoint.host = "db.example.invalid".into();
    let error = Session::connect(endpoint.clone(), login.clone())
        .await
        .err()
        .unwrap();
    assert!(matches!(error, Error::Tls(_)), "{error:?}");
    endpoint.tls = TlsMode::VerifyCa;
    let session = Session::connect(endpoint, login).await.unwrap();
    assert!(!session.server_version().await.unwrap().is_empty());
}
