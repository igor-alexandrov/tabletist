# ClickHouse, phase 1: the transport. Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the ClickHouse spike's ureq transport with the one the design calls for: a tokio `TcpStream`, tokio-rustls with `tls::client_config`, HTTP/1.1 by ureq-proto, the `prefer` probe, a pool of two connections and `KILL QUERY`, tested against the compose server over plain HTTP, every TLS mode and an SSH tunnel.

**Architecture:** `crates/tabletist-db/src/clickhouse.rs` stays a `#[doc(hidden)] pub` module (no `Driver` names it until phase 2) and becomes the root of `clickhouse/http.rs` (the transport) and `clickhouse/read.rs` (the spike's reader, made async). Its public surface is `Session`, `Canceller`, `Endpoint` and `Login`, which phase 2's adapter builds on. The compose ClickHouse serves HTTPS on 8443 with a test CA's certificate.

**Tech Stack:** Rust 1.98, tokio, tokio-rustls 0.26 (ring), ureq-proto 0.6 (client), rustls via `tls::client_config`, ClickHouse 25.8 in compose.

**Spec:** `docs/superpowers/specs/2026-10-31-clickhouse-read-only-engine-design.md` (Architecture, Phases 1).

**Branch:** on top of pull request 246's branch (`claude/clickhouse-connection-research-ac093d`), or a branch from `main` once it is merged: this phase replaces the spike that pull request adds.

**Verified:** every file below was built and run on 2026-10-31 before this plan was written: fmt, clippy `-D warnings`, the unit tests, the 12 server tests and the tunnel test passed against compose (ClickHouse 25.8).

---

## Files

| File | What it is |
| --- | --- |
| `compose/clickhouse/ca.pem` | Create. The test CA's certificate, which the tests trust. |
| `compose/clickhouse/server.pem`, `server.key` | Create. The server's certificate (names `localhost`, `clickhouse`, `127.0.0.1`) and key. Public test material. |
| `compose/clickhouse/tls.xml` | Create. Turns on HTTPS on 8443 with them. |
| `compose/clickhouse/README.md` | Create. How the files were made, and how to make them again. |
| `compose.yaml` | Modify. The clickhouse service publishes 58443 and mounts the four files; the header says how to run the tests. |
| `AGENTS.md` | Modify. The integration-test lines name the clickhouse service and its two URL variables. |
| `crates/tabletist-db/Cargo.toml` | Modify. `ureq` out; `ureq-proto` and `tokio-rustls` in, each with its reason. |
| `crates/tabletist-db/src/clickhouse/read.rs` | Create. The spike's reader, moved, with `read_tsv` async. |
| `crates/tabletist-db/src/clickhouse/http.rs` | Create. The transport. |
| `crates/tabletist-db/src/clickhouse.rs` | Rewrite. `Session` and `Canceller` over `http` and `read`; the tunnel test. |
| `crates/tabletist-db/tests/clickhouse.rs` | Rewrite. The server tests on the new API, with TLS. |

`crates/tabletist-db/src/lib.rs` keeps `#[doc(hidden)] pub mod clickhouse;` as it is.

---

### Task 1: HTTPS on the compose server

**Files:**
- Create: `compose/clickhouse/ca.pem`, `compose/clickhouse/server.pem`, `compose/clickhouse/server.key`, `compose/clickhouse/tls.xml`, `compose/clickhouse/README.md`
- Modify: `compose.yaml`, `AGENTS.md`

- [ ] **Step 1: Make the test CA and the server certificate**

Run from the repository root (OpenSSL 3):

```bash
mkdir -p compose/clickhouse && cd compose/clickhouse
openssl req -x509 -newkey ec -pkeyopt ec_paramgen_curve:prime256v1 -nodes \
  -keyout ca.key -out ca.pem -days 36500 -subj "/CN=Tabletist test CA" \
  -addext "basicConstraints=critical,CA:TRUE" -addext "keyUsage=critical,keyCertSign,cRLSign"
openssl req -newkey ec -pkeyopt ec_paramgen_curve:prime256v1 -nodes \
  -keyout server.key -out server.csr -subj "/CN=localhost"
printf 'subjectAltName=DNS:localhost,DNS:clickhouse,IP:127.0.0.1\nbasicConstraints=critical,CA:FALSE\nkeyUsage=critical,digitalSignature\nextendedKeyUsage=serverAuth\n' > ext.cnf
openssl x509 -req -in server.csr -CA ca.pem -CAkey ca.key -CAcreateserial \
  -out server.pem -days 36500 -extfile ext.cnf
rm ca.key ca.srl server.csr ext.cnf
chmod 644 server.key
cd ../..
```

The CA's key is thrown away: nothing signs with it again. `server.key` is world-readable so the container's `clickhouse` user can read it through the bind mount; it is public test material, like `crates/tabletist-db/tests/ssh/id_ed25519`.

- [ ] **Step 2: Write `compose/clickhouse/tls.xml`**

```xml
<!-- HTTPS on 8443 with the test certificate, for the TLS tests. -->
<clickhouse>
    <https_port>8443</https_port>
    <openSSL>
        <server>
            <certificateFile>/etc/clickhouse-server/certs/server.pem</certificateFile>
            <privateKeyFile>/etc/clickhouse-server/certs/server.key</privateKeyFile>
            <verificationMode>none</verificationMode>
            <loadDefaultCAFile>false</loadDefaultCAFile>
        </server>
    </openSSL>
</clickhouse>
```

- [ ] **Step 3: Write `compose/clickhouse/README.md`**

````markdown
# ClickHouse's test certificate

HTTPS on the compose server's 8443 (`tls.xml`), for the TLS tests in
`crates/tabletist-db`. Public test material: never trust it elsewhere.

- `ca.pem`: a test CA, valid for 100 years. Its key was thrown away.
- `server.pem`, `server.key`: the server's certificate, signed by it, for
  `localhost`, `clickhouse` (the compose network's name, which a test
  reaches through the SSH service) and `127.0.0.1`.

Made with OpenSSL 3, from this directory:

```bash
openssl req -x509 -newkey ec -pkeyopt ec_paramgen_curve:prime256v1 -nodes \
  -keyout ca.key -out ca.pem -days 36500 -subj "/CN=Tabletist test CA" \
  -addext "basicConstraints=critical,CA:TRUE" -addext "keyUsage=critical,keyCertSign,cRLSign"
openssl req -newkey ec -pkeyopt ec_paramgen_curve:prime256v1 -nodes \
  -keyout server.key -out server.csr -subj "/CN=localhost"
printf 'subjectAltName=DNS:localhost,DNS:clickhouse,IP:127.0.0.1\nbasicConstraints=critical,CA:FALSE\nkeyUsage=critical,digitalSignature\nextendedKeyUsage=serverAuth\n' > ext.cnf
openssl x509 -req -in server.csr -CA ca.pem -CAkey ca.key -CAcreateserial \
  -out server.pem -days 36500 -extfile ext.cnf
rm ca.key ca.srl server.csr ext.cnf
chmod 644 server.key
```
````

- [ ] **Step 4: Point the compose service at them**

In `compose.yaml`, the `clickhouse` service's `ports` gains 58443, and it mounts the files. Replace:

```yaml
    ports:
      - "127.0.0.1:58123:8123"
```

with:

```yaml
    ports:
      - "127.0.0.1:58123:8123"
      - "127.0.0.1:58443:8443"
    # HTTPS on 8443 with a test certificate (compose/clickhouse/README.md).
    volumes:
      - ./compose/clickhouse/tls.xml:/etc/clickhouse-server/config.d/tls.xml:ro
      - ./compose/clickhouse/server.pem:/etc/clickhouse-server/certs/server.pem:ro
      - ./compose/clickhouse/server.key:/etc/clickhouse-server/certs/server.key:ro
```

And in the header comment, replace the ClickHouse lines:

```yaml
# The ClickHouse spike (not in CI):
#   docker compose up -d --wait clickhouse
#   TABLETIST_TEST_CLICKHOUSE_URL=http://tabletist:tabletist@localhost:58123/tabletist \
#     cargo test -p tabletist-db --test clickhouse
```

with:

```yaml
# ClickHouse (not in CI until it is an engine), with SSH for its tunnel test:
#   docker compose up -d --build --wait clickhouse ssh
#   TABLETIST_TEST_CLICKHOUSE_URL=http://tabletist:tabletist@localhost:58123/tabletist \
#   TABLETIST_TEST_CLICKHOUSE_HTTPS_URL=https://tabletist:tabletist@localhost:58443/tabletist \
#   TABLETIST_TEST_SSH_URL=ssh://tabletist:tabletist@localhost:52222 \
#     sh -c 'cargo test -p tabletist-db --lib clickhouse && cargo test -p tabletist-db --test clickhouse'
```

(Two runs: cargo's filter matches test names, and the server tests in
`tests/clickhouse.rs` have no `clickhouse` in theirs.)

- [ ] **Step 5: Check the server answers over HTTPS with the test CA**

```bash
docker compose up -d --force-recreate --wait clickhouse
curl -sS --cacert compose/clickhouse/ca.pem -H 'X-ClickHouse-User: tabletist' -H 'X-ClickHouse-Key: tabletist' https://localhost:58443/ --data-binary 'SELECT 1'
```

Expected: `1`. (If an older container still holds the port, `docker compose up` names it; remove it and run again.)

- [ ] **Step 6: Name the service in AGENTS.md**

In `AGENTS.md`, under "Checks", after the paragraph that ends "CI runs all three suites on Linux.", add:

```markdown
ClickHouse is not an engine yet and not in CI: `docker compose up -d --build --wait clickhouse ssh`, then
the three variables `TABLETIST_TEST_CLICKHOUSE_URL=http://tabletist:tabletist@localhost:58123/tabletist`,
`TABLETIST_TEST_CLICKHOUSE_HTTPS_URL=https://tabletist:tabletist@localhost:58443/tabletist` and
`TABLETIST_TEST_SSH_URL=ssh://tabletist:tabletist@localhost:52222` set for both
`cargo test -p tabletist-db --lib clickhouse` and `cargo test -p tabletist-db --test clickhouse`
(cargo's filter matches test names, so one run does not cover both).
```

- [ ] **Step 7: Commit**

```bash
git add compose/clickhouse compose.yaml AGENTS.md
git commit -m "Serve HTTPS from the compose ClickHouse with a test CA's certificate"
```

(The commit message body says why: the TLS tests of phase 1 need every `TlsMode` against a real server, and the tunnel test needs a certificate for `clickhouse`. End it with the session's `Co-Authored-By` line.)

---

### Task 2: The transport

One commit: the new transport replaces the spike's ureq code, and nothing between would build without dead code.

**Files:**
- Modify: `crates/tabletist-db/Cargo.toml`
- Create: `crates/tabletist-db/src/clickhouse/read.rs`, `crates/tabletist-db/src/clickhouse/http.rs`
- Rewrite: `crates/tabletist-db/src/clickhouse.rs`, `crates/tabletist-db/tests/clickhouse.rs`

- [ ] **Step 1: Write the server tests on the new API**

Replace the whole of `crates/tabletist-db/tests/clickhouse.rs` with:

```rust
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
```

New against the spike's tests: `a_page_stopped_early_leaves_the_session_working` (a page drops its connection mid-answer, and the pool must not hand that connection out again), `a_session_idle_past_the_servers_keep_alive_still_works` (the server closes a kept connection after 10 idle seconds; the next request must not fail for it, and this test waits 11 seconds), the session after a mid-stream error, and four TLS tests: `prefer` both ways, `require` on a plain port, `verify-full` with and without the CA, and `verify-ca` under a name the certificate does not hold.

- [ ] **Step 2: Run them to see them fail**

```bash
cargo test --offline -p tabletist-db --test clickhouse
```

Expected: compile errors, `no Session in clickhouse` and the like.

- [ ] **Step 3: Swap the dependencies**

In `crates/tabletist-db/Cargo.toml`, replace the `ureq` entry and its comment:

```toml
# ClickHouse's HTTP interface (the `clickhouse` spike), blocking on
# spawn_blocking as SQLite is. Already in the tree through the app's check
# for updates, with the same features; the official `clickhouse` crate
# would add hyper.
ureq = { version = "3.4", default-features = false, features = ["rustls"] }
```

with:

```toml
# ClickHouse's HTTP interface speaks HTTP/1.1 through ureq-proto, the
# sans-IO protocol ureq is built on (already in the tree through the app's
# check for updates); it moves with ureq's version. Only its client half.
ureq-proto = { version = "0.6", default-features = false, features = ["client"] }
# TLS for ClickHouse's HTTP interface on our own rustls config, with ring
# (already in the tree through tokio-postgres-rustls).
tokio-rustls = { version = "0.26", default-features = false, features = ["ring", "tls12", "logging"] }
```

`default-features = false` on tokio-rustls matters: its defaults turn on aws-lc, and `tls.rs`'s `exactly_one_crypto_provider_is_compiled_in` would fail.

- [ ] **Step 4: Move the reader into `clickhouse/read.rs`, async**

Create `crates/tabletist-db/src/clickhouse/read.rs`. It begins:

```rust
//! Reading what ClickHouse answers: `TabSeparatedWithNamesAndTypes` rows,
//! the cells they hold, and the errors it writes, before the rows or among
//! them.

use tokio::io::{AsyncBufRead, AsyncBufReadExt, AsyncReadExt};

use crate::{ColumnMeta, Error, Result, Value, ValueKind};

pub(super) type Rows = Vec<Vec<Value>>;

/// The error ClickHouse wrote, with its numeric code where it gave one
/// (in a header before the rows, in the message after).
pub(super) fn server_error(code: Option<&str>, message: &str) -> Error {
    let message = message.trim();
    let code = code
        .map(str::to_owned)
        .or_else(|| {
            let rest = message.strip_prefix("Code: ")?;
            let end = rest.find('.')?;
            Some(rest[..end].to_owned())
        })
        .filter(|code| !code.is_empty() && code.bytes().all(|b| b.is_ascii_digit()));
    match code.as_deref() {
        // QUERY_WAS_CANCELLED
        Some("394") => Error::Cancelled,
        // TIMEOUT_EXCEEDED
        Some("159") => Error::Timeout,
        // AUTHENTICATION_FAILED, REQUIRED_PASSWORD
        Some("516" | "194") => Error::Auth(message.to_owned()),
        _ => Error::Query {
            code,
            message: message.to_owned(),
            detail: None,
            hint: None,
            named: Box::default(),
        },
    }
}

/// Reads `TabSeparatedWithNamesAndTypes`: names, types, then at most
/// `wanted` rows, and stops reading there.
///
/// An error after the rows began is written into the body: a line ending
/// in `\r` (which a row cannot hold, for TSV escapes it) starts it.
/// Newer servers (26.10) put `__exception__` lines and the
/// `X-ClickHouse-Exception-Tag` around it; older ones (25.8) write
/// `__exception__` and the message and close the connection.
///
/// An error after the rows wanted is not read. The server sends a block's
/// rows only once the whole block is computed, so such an error is of
/// rows past the page, which the next page meets, as a `LIMIT` on the
/// other engines never computes them.
pub(super) async fn read_tsv(
    mut body: impl AsyncBufRead + Unpin,
    wanted: u64,
    tag: Option<&str>,
) -> Result<(Vec<ColumnMeta>, Rows)> {
    let mut line = Vec::new();
    let names = header(&mut body, &mut line, tag).await?;
    let types = header(&mut body, &mut line, tag).await?;
    if names.len() != types.len() {
        return Err(Error::query(
            "ClickHouse sent names and types that disagree",
        ));
    }
    let columns: Vec<ColumnMeta> = names
        .into_iter()
        .zip(types)
        .map(|(name, type_name)| ColumnMeta {
            kind: kind(&type_name),
            name,
            type_name,
        })
        .collect();
    let mut rows = Vec::new();
    while (rows.len() as u64) < wanted {
        match next(&mut body, &mut line).await? {
            Line::Row => {
                // Counted first: a zip would drop the fields of a long row.
                if fields(&line).count() != columns.len() {
                    return Err(Error::query("ClickHouse sent a row of another width"));
                }
                rows.push(
                    fields(&line)
                        .zip(&columns)
                        .map(|(field, column)| value(&column.type_name, field))
                        .collect(),
                );
            }
            Line::Exception(rest) => return Err(exception(&rest, tag)),
            Line::End => break,
        }
    }
    Ok((columns, rows))
}

/// The next line into `line`, without its `\n`.
async fn next(body: &mut (impl AsyncBufRead + Unpin), line: &mut Vec<u8>) -> Result<Line> {
    line.clear();
    match body.read_until(b'\n', line).await {
        Ok(0) => Ok(Line::End),
        Ok(_) if line.ends_with(b"\r\n") => {
            let mut rest = std::mem::take(line);
            // The old servers close the connection after the message.
            let _ = body.read_to_end(&mut rest).await;
            Ok(Line::Exception(rest))
        }
        Ok(_) if line.last() == Some(&b'\n') => {
            line.pop();
            Ok(Line::Row)
        }
        Ok(_) => Err(Error::ConnectionLost("the server stopped mid-row".into())),
        Err(error) => Err(Error::ConnectionLost(error.to_string())),
    }
}

/// The names or the types: a line of fields, unescaped.
async fn header(
    body: &mut (impl AsyncBufRead + Unpin),
    line: &mut Vec<u8>,
    tag: Option<&str>,
) -> Result<Vec<String>> {
    match next(body, line).await? {
        Line::Row => Ok(fields(line)
            .map(|field| String::from_utf8_lossy(&unescape(field)).into_owned())
            .collect()),
        Line::Exception(rest) => Err(exception(&rest, tag)),
        Line::End => Err(Error::query("ClickHouse sent no column names")),
    }
}
```

After that, move from the spike's `clickhouse.rs`, unchanged and in this order: `enum Line`, `fn fields`, `fn exception`, `fn unescape`, `fn bare`, `fn family`, `fn kind`, `fn value`, and the whole `#[cfg(test)] mod tests`. Then make the tests async, since `read_tsv` is: in the test module, the helper becomes

```rust
    async fn read(body: &str, wanted: u64, tag: Option<&str>) -> Result<(Vec<ColumnMeta>, Rows)> {
        read_tsv(body.as_bytes(), wanted, tag).await
    }
```

and each test that calls `read(…)` (`names_and_types_become_columns_and_rows_values`, `reading_stops_at_the_rows_wanted`, `a_row_of_another_width_is_an_error`, `an_error_after_the_rows_began_is_the_servers_error`, `a_body_that_ends_mid_row_is_a_lost_connection`) becomes `#[tokio::test] async fn`, with `.await` after each `read(…)`. A `&[u8]` is an `AsyncBufRead`, so the bodies stay as they are.

- [ ] **Step 5: Write `clickhouse/http.rs`**

Create `crates/tabletist-db/src/clickhouse/http.rs`:

```rust
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
use std::time::Duration;

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

const CONNECT_TIMEOUT: Duration = Duration::from_secs(15);

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
            // `keep_alive_timeout`: one that did not answer is replaced,
            // once. Safe to send again, for nothing ran.
            Some(kept) => match self.exchange(kept, sql, settings).await {
                Err(Failure::Unanswered(_)) => {
                    self.exchange(self.fresh().await?, sql, settings).await
                }
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
        let unanswered = |error: io::Error| Failure::Unanswered(lost(error));
        let mut request = Request::post(self.uri(settings))
            .header("X-ClickHouse-User", &self.login.user)
            .header("X-ClickHouse-Key", &self.login.password)
            .header("Content-Length", sql.len());
        if !self.login.database.is_empty() {
            request = request.header("X-ClickHouse-Database", &self.login.database);
        }
        let request = request.body(()).map_err(answered(protocol))?;

        let mut out = vec![0; 16 * 1024];
        let mut call = Call::new(request).map_err(answered(protocol))?.proceed();
        while !call.can_proceed() {
            let n = call.write(&mut out).map_err(answered(protocol))?;
            stream.write_all(&out[..n]).await.map_err(unanswered)?;
        }
        let mut call = match call.proceed().map_err(answered(protocol))? {
            Some(SendRequestResult::SendBody(call)) => call,
            Some(SendRequestResult::Await100(call)) => {
                match call.proceed().map_err(answered(protocol))? {
                    Await100Result::SendBody(call) => call,
                    Await100Result::RecvResponse(_) => return Err(Failure::Answered(unexpected())),
                }
            }
            Some(SendRequestResult::RecvResponse(_)) | None => {
                return Err(Failure::Answered(unexpected()));
            }
        };
        let mut sent = 0;
        loop {
            let (used, n) = call
                .write(&sql.as_bytes()[sent..], &mut out)
                .map_err(answered(protocol))?;
            sent += used;
            stream.write_all(&out[..n]).await.map_err(unanswered)?;
            if sent == sql.len() && call.can_proceed() {
                break;
            }
        }
        stream.flush().await.map_err(unanswered)?;
        let mut call = call
            .proceed()
            .ok_or_else(|| Failure::Answered(unexpected()))?;

        let mut input = Input::default();
        let response = loop {
            let (used, response) = call
                .try_response(input.pending(), false)
                .map_err(answered(protocol))?;
            input.consume(used);
            if let Some(response) = response {
                break response;
            }
            match input.fill(&mut stream).await {
                Ok(0) | Err(_) if !input.received => {
                    return Err(Failure::Unanswered(Error::ConnectionLost(
                        "the server closed the connection before it answered".into(),
                    )));
                }
                Ok(0) => {
                    return Err(Failure::Answered(Error::ConnectionLost(
                        "the server closed the connection mid-answer".into(),
                    )));
                }
                Ok(_) => {}
                Err(error) => return Err(Failure::Answered(lost(error))),
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
                return Err(Failure::Answered(unexpected()));
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

    fn take(&self) -> Option<Stream> {
        self.kept
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .pop()
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

/// How an exchange failed: before the server sent a byte of its answer,
/// or after.
enum Failure {
    Unanswered(Error),
    Answered(Error),
}

impl Failure {
    fn into_error(self) -> Error {
        match self {
            Self::Unanswered(error) | Self::Answered(error) => error,
        }
    }
}

fn answered<E>(to_error: impl Fn(E) -> Error) -> impl Fn(E) -> Failure {
    move |error| Failure::Answered(to_error(error))
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
/// port: it answered with bytes that are no TLS record, or closed.
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
                    let mut request = Vec::new();
                    let mut chunk = [0; 1024];
                    // The whole request, head and body (`go`), before the
                    // answer: closing with bytes unread would send a reset,
                    // which on Windows can discard the answer.
                    while !request.ends_with(b"\r\n\r\ngo") {
                        let n = socket.read(&mut chunk).await.unwrap();
                        if n == 0 {
                            return;
                        }
                        request.extend_from_slice(&chunk[..n]);
                    }
                    let answer = b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\nok";
                    let _ = socket.write_all(answer).await;
                });
            }
        });
        port
    }

    #[tokio::test]
    async fn a_kept_connection_the_server_closed_is_replaced() {
        let port = closing_server().await;
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
        let http = Http::connect(endpoint, login).await.unwrap();
        for _ in 0..3 {
            let mut answer = http.post("go", &[]).await.unwrap();
            let mut body = String::new();
            answer.body.read_to_string(&mut body).await.unwrap();
            assert_eq!(body, "ok");
            // Let the server's close arrive before the next request.
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    }

    #[test]
    fn an_ipv6_host_is_bracketed() {
        assert_eq!(authority_host("::1"), "[::1]");
        assert_eq!(authority_host("db.example.com"), "db.example.com");
    }
}
```

How it reads, for a reviewer: `connect` builds the TLS connector from `tls::client_config` (so every `TlsMode` means what it means for PostgreSQL), opens the first connection and, under `prefer`, drops TLS when the handshake meets a server that does not speak it (`speaks_no_tls`: a record rustls cannot parse, or a close). `post` takes a kept connection or opens one; a kept one that fails before the server sent a byte of its answer (`Failure::Unanswered`: the server closed it while idle) is replaced once by a fresh connection, which is safe since nothing ran. `exchange` drives ureq-proto's `Call` through its states: the request line and headers, the body (`Content-Length`, so not chunked), then the response head. The connection always goes to `via` when a tunnel is open, while the URI and TLS's server name stay the real host. `Body` is an `AsyncRead` over the rest: read to its end, its connection goes back to the pool (two at most); dropped early, the connection closes, which is how a page stops the server sending.

- [ ] **Step 6: Rewrite `clickhouse.rs` over them**

Replace the whole of `crates/tabletist-db/src/clickhouse.rs` with:

```rust
//! ClickHouse over its HTTP interface. Not an engine yet: no `Driver`
//! names it (see
//! `docs/superpowers/specs/2026-10-31-clickhouse-read-only-engine-design.md`,
//! whose phase 2 makes it one).
//!
//! Each statement is one POST (`http`). Rows come back as
//! `TabSeparatedWithNamesAndTypes`, read a line at a time (`read`), so a
//! page stops reading where it is full. Every statement carries its own
//! `query_id`, which is how [`Canceller`] stops it: `KILL QUERY` on another
//! connection, for a dropped connection does not stop a query.

mod http;
mod read;

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Instant;

use tokio::io::{AsyncReadExt, BufReader};

use http::Http;
pub use http::{Endpoint, Login};
use read::read_tsv;

use crate::{Error, Result, RowPage, Value};

/// A session with one server: one statement at a time, as a PostgreSQL or
/// MySQL session runs them.
pub struct Session {
    http: Arc<Http>,
    /// Held while a statement runs, so `running` names the one there is.
    one_at_a_time: tokio::sync::Mutex<()>,
    /// The `query_id` of the statement running now, which [`Canceller`]
    /// kills.
    running: Arc<Mutex<Option<String>>>,
}

/// Stops the statement a [`Session`] runs. Doing so when nothing runs does
/// nothing.
#[derive(Clone)]
pub struct Canceller {
    http: Arc<Http>,
    running: Arc<Mutex<Option<String>>>,
}

impl Session {
    pub async fn connect(endpoint: Endpoint, login: Login) -> Result<Self> {
        Ok(Self {
            http: Arc::new(Http::connect(endpoint, login).await?),
            one_at_a_time: tokio::sync::Mutex::new(()),
            running: Arc::default(),
        })
    }

    /// Whether the session's requests go over TLS. `prefer` falls back to
    /// plain HTTP where the server has no TLS on the port.
    pub fn is_encrypted(&self) -> bool {
        self.http.is_encrypted()
    }

    pub fn canceller(&self) -> Canceller {
        Canceller {
            http: Arc::clone(&self.http),
            running: Arc::clone(&self.running),
        }
    }

    pub async fn server_version(&self) -> Result<String> {
        let page = self.select_page("SELECT version()", 1).await?;
        match page.rows.first().and_then(|row| row.first()) {
            Some(Value::Text(version)) => Ok(version.to_string()),
            _ => Err(Error::query("the server did not say its version")),
        }
    }

    /// Runs `sql`, which must not name a `FORMAT`, under the server's
    /// `readonly = 2` (no DDL, settings allowed), and keeps its first
    /// `limit` rows. That is no fence on its own: `INSERT INTO FUNCTION`,
    /// `BACKUP` and `KILL` still run, so `sql` must be a read.
    pub async fn select_page(&self, sql: &str, limit: u32) -> Result<RowPage> {
        let _turn = self.one_at_a_time.lock().await;
        let started = Instant::now();
        let id = query_id();
        *self.running.lock().unwrap_or_else(PoisonError::into_inner) = Some(id.clone());
        let read = self.read_rows(sql, limit, &id).await;
        *self.running.lock().unwrap_or_else(PoisonError::into_inner) = None;
        let (columns, mut rows) = read?;
        let has_more = rows.len() > limit as usize;
        rows.truncate(limit as usize);
        Ok(RowPage {
            columns,
            rows,
            has_more,
            ordered_by_key: false,
            elapsed: started.elapsed(),
        })
    }

    async fn read_rows(
        &self,
        sql: &str,
        limit: u32,
        id: &str,
    ) -> Result<(Vec<crate::ColumnMeta>, read::Rows)> {
        // One row past the page says whether another page exists. The
        // server stops at about that many too, a block past it at most.
        let wanted = u64::from(limit) + 1;
        let wanted_text = wanted.to_string();
        let answer = self
            .http
            .post(
                sql,
                &[
                    ("query_id", id),
                    ("readonly", "2"),
                    ("default_format", "TabSeparatedWithNamesAndTypes"),
                    ("max_result_rows", &wanted_text),
                    ("result_overflow_mode", "break"),
                ],
            )
            .await?;
        let tag = answer.header("x-clickhouse-exception-tag");
        read_tsv(BufReader::new(answer.body), wanted, tag.as_deref()).await
    }
}

impl Canceller {
    pub async fn cancel(&self) -> Result<()> {
        let Some(id) = self
            .running
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
        else {
            return Ok(());
        };
        // `id` is ours (`query_id`): letters, digits and dashes only.
        let sql = format!("KILL QUERY WHERE query_id = '{id}' ASYNC FORMAT Null");
        let mut answer = self.http.post(&sql, &[]).await?;
        let mut drained = Vec::new();
        answer
            .body
            .read_to_end(&mut drained)
            .await
            .map_err(|error| Error::Connect(format!("could not cancel: {error}")))?;
        Ok(())
    }
}

/// A `query_id` no other statement of this process has.
fn query_id() -> String {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    format!(
        "tabletist-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    )
}
```

`select_page` keeps the spike's `readonly=2` and its doc comment's warning: the allowlist that makes the editor safe is phase 2's.

- [ ] **Step 7: Run the unit tests**

```bash
cargo test --offline -p tabletist-db --lib clickhouse
```

Expected: 11 passed (8 moved reader tests, 3 in `http`, among them `a_kept_connection_the_server_closed_is_replaced`, against a local server that closes each connection after one answer).

- [ ] **Step 8: Run the server tests**

```bash
docker compose up -d --wait clickhouse
TABLETIST_TEST_CLICKHOUSE_URL=http://tabletist:tabletist@localhost:58123/tabletist \
TABLETIST_TEST_CLICKHOUSE_HTTPS_URL=https://tabletist:tabletist@localhost:58443/tabletist \
  cargo test --offline -p tabletist-db --test clickhouse
```

Expected: 12 passed, in about 11 seconds (the idle test). Without the two variables, all 12 print "skipped" and pass.

- [ ] **Step 9: Check the lockfile and the rest**

```bash
git diff --stat Cargo.lock
cargo fmt --all --check
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo test --locked -p tabletist-db
```

Expected: `Cargo.lock` changes only in tabletist-db's dependency list (`ureq` out, `tokio-rustls` and `ureq-proto` in): no `[[package]]` is added or removed. fmt, clippy and the tests pass, `tls::tests::exactly_one_crypto_provider_is_compiled_in` among them.

- [ ] **Step 10: Commit**

```bash
git add Cargo.lock crates/tabletist-db/Cargo.toml crates/tabletist-db/src/clickhouse.rs crates/tabletist-db/src/clickhouse crates/tabletist-db/tests/clickhouse.rs
git commit -m "Speak ClickHouse's HTTP on tokio, tokio-rustls and ureq-proto"
```

The body: the spike's ureq could not take `tls::client_config` (no `verify-ca`, no OS roots) or check a tunnelled server's real name; this transport does both, decides `prefer` once at connect, keeps two connections, and adds no crate to the lockfile. End with the `Co-Authored-By` line.

---

### Task 3: Through an SSH tunnel

**Files:**
- Modify: `crates/tabletist-db/src/clickhouse.rs` (a test module at its end)

- [ ] **Step 1: Write the tunnel test**

Append to `crates/tabletist-db/src/clickhouse.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::ssh::Tunnel;
    use crate::{HostKeys, Secrets, SshAuth, SshSpec, SshStage, TlsMode};

    /// The compose SSH server and its password, from
    /// TABLETIST_TEST_SSH_URL (`ssh://user:password@host:port`), or `None`.
    fn ssh() -> Option<(SshSpec, Secrets)> {
        let Ok(url) = std::env::var("TABLETIST_TEST_SSH_URL") else {
            eprintln!("skipped: TABLETIST_TEST_SSH_URL is not set");
            return None;
        };
        let url = url::Url::parse(&url).unwrap();
        Some((
            SshSpec {
                host: url.host_str().unwrap().to_owned(),
                port: url.port(),
                user: url.username().to_owned(),
                auth: SshAuth::Password,
            },
            Secrets {
                ssh_password: url.password().map(str::to_owned),
                ..Secrets::default()
            },
        ))
    }

    /// A tunnel to `clickhouse:port` in the compose network, trusting the
    /// SSH server's key as the app does: from the first refusal.
    async fn tunnel(ssh: &SshSpec, secrets: &Secrets, port: u16) -> Tunnel {
        let error = match Tunnel::open(ssh, "clickhouse", port, secrets, &HostKeys::default()).await
        {
            Ok(_) => panic!("the host key was trusted before it was learned"),
            Err(error) => error,
        };
        let Error::Ssh {
            stage:
                SshStage::HostKeyUnknown {
                    host,
                    port: ssh_port,
                    fingerprint,
                },
            ..
        } = error
        else {
            panic!("{error:?}");
        };
        let mut keys = HostKeys::default();
        keys.trust(&host, ssh_port, &fingerprint);
        Tunnel::open(ssh, "clickhouse", port, secrets, &keys)
            .await
            .unwrap()
    }

    #[tokio::test]
    async fn a_tunnel_carries_the_session_and_tls_checks_the_real_host() {
        let Some((ssh, secrets)) = ssh() else { return };
        let login = Login {
            user: "tabletist".into(),
            password: "tabletist".into(),
            database: "tabletist".into(),
        };

        let plain = tunnel(&ssh, &secrets, 8123).await;
        let endpoint = Endpoint {
            host: "clickhouse".into(),
            port: 8123,
            via: Some(plain.port),
            tls: TlsMode::Disable,
            ca_file: None,
        };
        let session = Session::connect(endpoint, login.clone()).await.unwrap();
        assert!(!session.server_version().await.unwrap().is_empty());

        // The certificate names `clickhouse`, not 127.0.0.1 where the
        // tunnel listens: verify-full passes only by the real host's name.
        let tls = tunnel(&ssh, &secrets, 8443).await;
        let ca_file = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../compose/clickhouse/ca.pem");
        let endpoint = Endpoint {
            host: "clickhouse".into(),
            port: 8443,
            via: Some(tls.port),
            tls: TlsMode::VerifyFull,
            ca_file: Some(ca_file),
        };
        let session = Session::connect(endpoint, login).await.unwrap();
        assert!(session.is_encrypted());
        assert!(!session.server_version().await.unwrap().is_empty());
    }
}
```

It is in the crate, not in `tests/`, because `ssh::Tunnel` is crate-private (as `ssh::tests` are). It reaches `clickhouse:8123` and `clickhouse:8443` by the compose network's name, which only the SSH service resolves, and checks that `verify-full` passes through the tunnel by the real host's name while the connection goes to `127.0.0.1`.

- [ ] **Step 2: Run it**

```bash
docker compose up -d --build --wait clickhouse ssh
TABLETIST_TEST_SSH_URL=ssh://tabletist:tabletist@localhost:52222 \
  cargo test --offline -p tabletist-db --lib clickhouse::tests
```

Expected: 1 passed (Task 2's `Endpoint::via` already carries it). Without the variable it prints "skipped" and passes.

- [ ] **Step 3: Check and commit**

```bash
cargo fmt --all --check
cargo clippy --locked --workspace --all-targets -- -D warnings
git add crates/tabletist-db/src/clickhouse.rs
git commit -m "Test ClickHouse's HTTP through an SSH tunnel, TLS named by the real host"
```

End the message with the `Co-Authored-By` line.

---

### Task 4: The whole check

- [ ] **Step 1: Run every check AGENTS.md lists**

```bash
cargo fmt --all --check
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo test --locked --workspace --all-targets
RUSTDOCFLAGS='-D warnings' cargo doc --locked --workspace --no-deps
```

Expected: all pass.

- [ ] **Step 2: Run the ClickHouse tests with every variable**

```bash
docker compose up -d --build --wait clickhouse ssh
TABLETIST_TEST_CLICKHOUSE_URL=http://tabletist:tabletist@localhost:58123/tabletist \
TABLETIST_TEST_CLICKHOUSE_HTTPS_URL=https://tabletist:tabletist@localhost:58443/tabletist \
TABLETIST_TEST_SSH_URL=ssh://tabletist:tabletist@localhost:52222 \
  sh -c 'cargo test --locked -p tabletist-db --lib clickhouse && cargo test --locked -p tabletist-db --test clickhouse'
```

Expected: 12 in the library (11 unit tests and the tunnel test) and the 12 server tests pass.

- [ ] **Step 3: Report what ran where**

Say in the pull request which platform ran the server tests (they are not in CI yet), as AGENTS.md asks.
