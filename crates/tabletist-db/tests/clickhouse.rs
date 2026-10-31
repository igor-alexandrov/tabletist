//! The ClickHouse spike against a server. Needs one: run
//! `docker compose up -d clickhouse` and set TABLETIST_TEST_CLICKHOUSE_URL
//! (see compose.yaml). Without it every test prints "skipped" and passes.

// `allow-unwrap-in-tests` does not cover helpers in an integration-test crate.
#![allow(clippy::unwrap_used)]

use std::time::Duration;

use tabletist_db::clickhouse::{Http, Server};
use tabletist_db::{Error, Value};

/// The server from `http://user:password@host:port/database`.
fn server() -> Option<Server> {
    let Some(url) = std::env::var("TABLETIST_TEST_CLICKHOUSE_URL")
        .ok()
        .filter(|url| !url.trim().is_empty())
    else {
        eprintln!("skipped: TABLETIST_TEST_CLICKHOUSE_URL is not set");
        return None;
    };
    let url = url::Url::parse(&url).unwrap();
    Some(Server {
        url: format!(
            "{}://{}",
            url.scheme(),
            url.authority().rsplit('@').next().unwrap()
        ),
        user: url.username().to_owned(),
        password: url.password().unwrap_or_default().to_owned(),
        database: url.path().trim_start_matches('/').to_owned(),
    })
}

fn http() -> Option<Http> {
    Some(Http::new(server()?).unwrap())
}

#[tokio::test]
async fn the_server_says_its_version() {
    let Some(http) = http() else { return };
    let version = http.server_version().await.unwrap();
    assert!(
        version.chars().next().unwrap().is_ascii_digit(),
        "{version}"
    );
}

#[tokio::test]
async fn a_page_keeps_its_rows_and_says_whether_more_follow() {
    let Some(http) = http() else { return };
    // A comparison is a UInt8 in ClickHouse: `toBool` makes it a Bool.
    let sql = "SELECT number, toString(number) AS s, toBool(number % 2 = 0) AS even \
               FROM numbers(1000)";
    let page = http.select_page(sql.into(), 10).await.unwrap();
    let names: Vec<_> = page.columns.iter().map(|c| c.name.as_str()).collect();
    assert_eq!(names, ["number", "s", "even"]);
    assert_eq!(page.columns[0].type_name, "UInt64");
    assert_eq!(page.rows.len(), 10);
    assert!(page.has_more);
    assert_eq!(
        page.rows[3],
        [Value::Int(3), Value::Text("3".into()), Value::Bool(false)]
    );

    let page = http.select_page(sql.into(), 5000).await.unwrap();
    assert_eq!(page.rows.len(), 1000);
    assert!(!page.has_more);
}

#[tokio::test]
async fn a_read_cannot_write() {
    let Some(http) = http() else { return };
    let error = http
        .select_page("CREATE TABLE spike (a Int8) ENGINE = Memory".into(), 10)
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
    let Some(http) = http() else { return };
    let sql = "SELECT number, throwIf(number = 150000, 'boom') FROM system.numbers \
               LIMIT 200000 SETTINGS max_block_size = 1000";
    let error = http.select_page(sql.into(), 200_000).await.unwrap_err();
    let Error::Query { code, message, .. } = error else {
        panic!("{error:?}");
    };
    // FUNCTION_THROW_IF_VALUE_IS_NON_ZERO
    assert_eq!(code.as_deref(), Some("395"));
    assert!(message.contains("boom"), "{message}");
}

#[tokio::test]
async fn a_running_query_is_cancelled() {
    let Some(http) = http() else { return };
    let canceller = http.canceller();
    let running = tokio::spawn(async move {
        http.select_page("SELECT sum(number) FROM numbers(100000000000)".into(), 1)
            .await
    });
    // The query may not have started yet: cancel until it ends.
    let outcome = tokio::time::timeout(Duration::from_secs(30), async {
        loop {
            tokio::time::sleep(Duration::from_millis(200)).await;
            canceller.cancel().await.unwrap();
            if running.is_finished() {
                return running.await.unwrap();
            }
        }
    })
    .await
    .expect("the query was not cancelled");
    assert_eq!(outcome.unwrap_err(), Error::Cancelled);
}

#[tokio::test]
async fn a_session_runs_one_statement_at_a_time_so_cancel_finds_it() {
    let Some(http) = http() else { return };
    let canceller = http.canceller();
    let slow = http.clone();
    let running = tokio::spawn(async move {
        slow.select_page("SELECT sum(number) FROM numbers(100000000000)".into(), 1)
            .await
    });
    // Run beside the slow one, a quick statement must not leave the
    // canceller with nothing to stop once it is done.
    let quick = http.select_page("SELECT 1".into(), 1);
    let cancelling = async {
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
    };
    let (quick, slow) = tokio::join!(quick, cancelling);
    assert_eq!(slow.unwrap_err(), Error::Cancelled);
    // Cancelled too if it ran first, while the canceller kept cancelling.
    assert!(matches!(quick, Ok(_) | Err(Error::Cancelled)), "{quick:?}");
}

#[tokio::test]
async fn a_wrong_password_is_an_authentication_error() {
    let Some(mut server) = server() else { return };
    server.password = "not-the-password".into();
    let error = Http::new(server)
        .unwrap()
        .server_version()
        .await
        .unwrap_err();
    assert!(matches!(error, Error::Auth(_)), "{error:?}");
}
