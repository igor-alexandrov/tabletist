//! Redis, against a `redis-server` each test starts on a port of its own
//! (`fixtures::redis_server`), seeded with the Bookshop's keys. Without the
//! binary each test prints "skipped" and passes.

use std::net::TcpListener;
use std::time::Duration;

use tabletist_db::fixtures::redis_server::TestServer;
use tabletist_db::redis::{self, Cursor, KeyType, Page, READS};
use tabletist_db::{ConnectSpec, Connection, Error, HostKeys, Secrets};

fn spec(server: &TestServer) -> ConnectSpec {
    ConnectSpec::redis("127.0.0.1", server.port)
}

async fn open(spec: &ConnectSpec, secrets: &Secrets) -> tabletist_db::Result<Connection> {
    Connection::connect_with(
        spec,
        secrets,
        &HostKeys::default(),
        tabletist_db::Access::ReadOnly,
    )
    .await
}

fn password(password: &str) -> Secrets {
    Secrets {
        password: Some(password.into()),
        ..Secrets::default()
    }
}

#[tokio::test]
async fn a_connection_probes_the_server_and_names_itself() {
    let Some(server) = TestServer::start(&[]) else {
        return;
    };
    server.seed_bookshop(None);
    let connection = open(&spec(&server), &Secrets::default()).await.unwrap();
    let handle = connection.redis().unwrap();
    let probe = handle.probe(0).await.unwrap();
    assert_eq!(probe.server.name, "Redis");
    assert!(!probe.server.version.is_empty());
    assert_eq!(probe.keys, Some(65));
    assert_eq!(probe.keys_in(0), 65);
    assert_eq!(probe.databases, 16);
    assert_eq!(probe.refused, None);
    // CLIENT SETNAME: the server lists the connection by the app's name.
    let clients = server.send(&["CLIENT", "LIST"]).unwrap();
    assert!(clients.contains("name=tabletist"), "{clients}");
    assert_eq!(
        connection.server_version().await.unwrap(),
        probe.server.label()
    );
    // Everything SQL is refused.
    assert!(matches!(
        connection.list_schemas().await,
        Err(Error::Unsupported(_))
    ));
    assert_eq!(connection.dialect(), None);
}

#[tokio::test]
async fn the_database_count_follows_the_server() {
    let Some(server) = TestServer::start(&["--databases", "32"]) else {
        return;
    };
    server.seed_many(20, 7);
    let mut spec = spec(&server);
    spec.redis.as_mut().unwrap().database = 20;
    let connection = open(&spec, &Secrets::default()).await.unwrap();
    let probe = connection.redis().unwrap().probe(20).await.unwrap();
    assert_eq!(probe.databases, 32);
    assert_eq!(probe.keys, Some(7));
    assert_eq!(probe.keys_in(20), 7);
}

#[tokio::test]
async fn select_moves_both_connections() {
    let Some(server) = TestServer::start(&[]) else {
        return;
    };
    server.seed_bookshop(None);
    server.seed_many(2, 38);
    let connection = open(&spec(&server), &Secrets::default()).await.unwrap();
    let handle = connection.redis().unwrap();
    assert_eq!(handle.dbsize().await.unwrap(), 65);
    handle.select(2).await.unwrap();
    assert_eq!(handle.dbsize().await.unwrap(), 38);
    let (_, names) = handle.scan(0, "*", 1000).await.unwrap();
    assert!(names.iter().all(|name| name.starts_with(b"item:")));
    assert_eq!(
        handle.types(&[b"item:1".to_vec()]).await.unwrap(),
        vec![KeyType::String]
    );
}

#[tokio::test]
async fn sign_in_failures_name_their_cause() {
    let Some(server) = TestServer::start(&["--requirepass", "s3cret"]) else {
        return;
    };
    let spec = spec(&server);
    // No password for a server that wants one.
    let error = open(&spec, &Secrets::default()).await.err().unwrap();
    assert!(matches!(&error, Error::Auth(_)), "{error:?}");
    assert_eq!(redis::code(&error), Some("NOAUTH"));
    // The wrong one.
    let error = open(&spec, &password("nope")).await.err().unwrap();
    assert_eq!(redis::code(&error), Some("WRONGPASS"), "{error:?}");
    // The right one, for the default user and for a user named.
    assert!(open(&spec, &password("s3cret")).await.is_ok());
    let mut named = spec.clone();
    named.user = "default".into();
    assert!(open(&named, &password("s3cret")).await.is_ok());
}

#[tokio::test]
async fn an_acl_user_without_scan_is_told() {
    let Some(server) = TestServer::start(&[]) else {
        return;
    };
    server
        .send(&[
            "ACL", "SETUSER", "reader", "on", ">pw", "~*", "+@all", "-scan",
        ])
        .unwrap();
    let mut spec = spec(&server);
    spec.user = "reader".into();
    let connection = open(&spec, &password("pw")).await.unwrap();
    let probe = connection.redis().unwrap().probe(0).await.unwrap();
    assert_eq!(probe.refused.as_deref(), Some("SCAN"));
}

#[tokio::test]
async fn nothing_listening_is_a_refusal() {
    let port = {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.local_addr().unwrap().port()
    };
    let error = open(&ConnectSpec::redis("127.0.0.1", port), &Secrets::default())
        .await
        .err()
        .unwrap();
    assert_eq!(error, Error::Connect(redis::REFUSED.into()));
}

#[tokio::test]
async fn a_server_that_closes_the_handshake_may_want_tls() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    // Accepts, and hangs up at once, as a TLS port does on plain text.
    tokio::spawn(async move {
        while let Ok((socket, _)) = listener.accept().await {
            drop(socket);
        }
    });
    let error = open(&ConnectSpec::redis("127.0.0.1", port), &Secrets::default())
        .await
        .err()
        .unwrap();
    assert_eq!(error, Error::Connect(redis::CLOSED_IN_HANDSHAKE.into()));
}

#[tokio::test]
async fn a_server_that_never_answers_times_out() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let held = tokio::spawn(async move {
        let mut held = Vec::new();
        while let Ok((socket, _)) = listener.accept().await {
            held.push(socket);
        }
    });
    let started = std::time::Instant::now();
    let error = open(&ConnectSpec::redis("127.0.0.1", port), &Secrets::default())
        .await
        .err()
        .unwrap();
    assert_eq!(error, Error::Timeout);
    assert!(started.elapsed() >= redis::CONNECT_TIMEOUT - Duration::from_millis(100));
    held.abort();
}

#[tokio::test]
async fn every_type_reads_with_its_command() {
    let Some(server) = TestServer::start(&[]) else {
        return;
    };
    server.seed_bookshop(None);
    let connection = open(&spec(&server), &Secrets::default()).await.unwrap();
    let handle = connection.redis().unwrap();
    let read = |key: &'static str, count: u32, from: Cursor| async move {
        let info = handle.describe(key.as_bytes()).await.unwrap();
        let read = handle
            .read(key.as_bytes(), &info.kind, info.length, &from, count)
            .await
            .unwrap();
        (info, read)
    };

    let (info, book) = read("book:4", 1000, Cursor::Start).await;
    assert_eq!(info.kind, KeyType::Hash);
    assert_eq!(info.length, Some(6));
    assert_eq!(info.ttl_ms, None);
    assert!(info.memory.is_some());
    assert!(info.encoding.is_some());
    assert_eq!(book.command, "HGETALL");
    let Page::Pairs(fields) = &book.page else {
        panic!("{book:?}")
    };
    assert!(fields.contains(&(b"title".to_vec(), b"The Cartographer's Daughter".to_vec())));

    let (info, session) = read("session:0001", 1000, Cursor::Start).await;
    assert_eq!(info.kind, KeyType::String);
    assert!(info.ttl_ms.is_some_and(|ttl| ttl > 0));
    assert_eq!(session.command, "GET");

    let (_, queue) = read("queue:emails", 2, Cursor::Start).await;
    assert_eq!(queue.command, "LRANGE");
    assert_eq!(
        queue.page,
        Page::Items(vec![
            b"order-confirmed:1001".to_vec(),
            b"order-shipped:1001".to_vec()
        ])
    );
    assert_eq!(queue.next, Some(Cursor::Offset(2)));
    let (_, rest) = read("queue:emails", 2, Cursor::Offset(2)).await;
    assert_eq!(rest.page, Page::Items(vec![b"password-reset:7".to_vec()]));
    assert_eq!(rest.next, None);

    let (_, tags) = read("tags:1", 1000, Cursor::Start).await;
    assert_eq!(tags.command, "SSCAN");
    let Page::Items(members) = &tags.page else {
        panic!("{tags:?}")
    };
    assert_eq!(members.len(), 3);

    let (_, top) = read("top:books", 3, Cursor::Start).await;
    assert_eq!(top.command, "ZRANGE");
    let Page::Scored(members) = &top.page else {
        panic!("{top:?}")
    };
    assert_eq!(members[0], (b"book:9".to_vec(), "3".into()));
    assert_eq!(top.next, Some(Cursor::Offset(3)));

    let (info, events) = read("events", 2, Cursor::Start).await;
    assert_eq!(info.kind, KeyType::Stream);
    assert_eq!(events.command, "XRANGE");
    let Page::Entries(entries) = &events.page else {
        panic!("{events:?}")
    };
    assert_eq!(entries[0].0, "1761832931000-0");
    assert_eq!(
        entries[0].1[0],
        (b"kind".to_vec(), b"order_placed".to_vec())
    );
    let next = events.next.clone().unwrap();
    let (_, last) = read("events", 2, next).await;
    let Page::Entries(entries) = &last.page else {
        panic!("{last:?}")
    };
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].0, "1761832931000-2");
}

#[tokio::test]
async fn long_and_binary_strings_are_cut_and_kept_as_bytes() {
    let Some(server) = TestServer::start(&[]) else {
        return;
    };
    let big = vec![b'a'; 1024 * 1024];
    let binary = vec![0u8, 159, 146, 150, 255];
    server
        .send_all(
            None,
            &[
                vec![b"SET".to_vec(), b"big".to_vec(), big],
                vec![b"SET".to_vec(), b"cover".to_vec(), binary.clone()],
            ],
        )
        .unwrap();
    let connection = open(&spec(&server), &Secrets::default()).await.unwrap();
    let handle = connection.redis().unwrap();
    let info = handle.describe(b"big").await.unwrap();
    assert_eq!(info.length, Some(1024 * 1024));
    let read = handle
        .read(b"big", &info.kind, info.length, &Cursor::Start, 1000)
        .await
        .unwrap();
    assert_eq!(read.command, "GETRANGE");
    let Page::Text { bytes, length } = read.page else {
        panic!()
    };
    assert_eq!(bytes.len() as u64, redis::STRING_LIMIT);
    assert_eq!(length, 1024 * 1024);
    let info = handle.describe(b"cover").await.unwrap();
    let read = handle
        .read(b"cover", &info.kind, info.length, &Cursor::Start, 1000)
        .await
        .unwrap();
    assert_eq!(
        read.page,
        Page::Text {
            bytes: binary,
            length: 5
        }
    );
}

#[tokio::test]
async fn a_large_hash_is_read_by_hscan_in_pages() {
    let Some(server) = TestServer::start(&[]) else {
        return;
    };
    let mut hset = vec![b"HSET".to_vec(), b"big:hash".to_vec()];
    for n in 0..1500 {
        hset.push(format!("f{n}").into_bytes());
        hset.push(b"v".to_vec());
    }
    server.send_all(None, &[hset]).unwrap();
    let connection = open(&spec(&server), &Secrets::default()).await.unwrap();
    let handle = connection.redis().unwrap();
    let info = handle.describe(b"big:hash").await.unwrap();
    assert_eq!(info.length, Some(1500));
    let mut from = Cursor::Start;
    let mut fields = std::collections::HashSet::new();
    loop {
        let read = handle
            .read(b"big:hash", &info.kind, info.length, &from, 500)
            .await
            .unwrap();
        assert_eq!(read.command, "HSCAN");
        let Page::Pairs(pairs) = read.page else {
            panic!()
        };
        fields.extend(pairs.into_iter().map(|(field, _)| field));
        match read.next {
            Some(next) => from = next,
            None => break,
        }
    }
    assert_eq!(fields.len(), 1500);
}

#[tokio::test]
async fn only_the_read_list_reaches_the_server() {
    let Some(server) = TestServer::start(&[]) else {
        return;
    };
    server.seed_bookshop(None);
    let connection = open(&spec(&server), &Secrets::default()).await.unwrap();
    let handle = connection.redis().unwrap();
    // Everything the browser does.
    handle.probe(0).await.unwrap();
    let (_, names) = handle.scan(0, "book:*", 1000).await.unwrap();
    handle.types(&names).await.unwrap();
    for key in [
        "book:1",
        "events",
        "top:books",
        "tags:1",
        "queue:emails",
        "config",
    ] {
        let info = handle.describe(key.as_bytes()).await.unwrap();
        handle
            .read(key.as_bytes(), &info.kind, info.length, &Cursor::Start, 10)
            .await
            .unwrap();
    }
    handle.select(1).await.unwrap();
    handle.dbsize().await.unwrap();
    let sent = handle.sent();
    for name in &sent {
        assert!(READS.contains(name), "{name} is not on the read list");
    }
    assert!(sent.contains(&"CLIENT SETNAME"));
    assert!(!sent.contains(&"KEYS"));
    // A write is refused before it is sent: the keys stay.
    handle.select(0).await.unwrap();
    for args in [
        &["FLUSHALL"][..],
        &["SET", "config", "{}"],
        &["DEL", "book:1"],
        &["CONFIG", "SET", "maxmemory", "1"],
    ] {
        let refused = handle.run(args).await;
        assert!(matches!(refused, Err(Error::Refused { .. })), "{args:?}");
    }
    assert_eq!(server.send(&["DBSIZE"]).unwrap(), "65");
    assert_eq!(
        server.send(&["GET", "config"]).unwrap().chars().next(),
        Some('{')
    );
    // The server saw no command off the list: KEYS least of all.
    let stats = server.send(&["INFO", "commandstats"]).unwrap();
    for line in stats.split_whitespace() {
        let Some(name) = line.strip_prefix("cmdstat_") else {
            continue;
        };
        let name = name
            .split(':')
            .next()
            .unwrap_or_default()
            .to_ascii_uppercase();
        let name = name.replace('|', " ");
        // The test's own socket sent these.
        if [
            "FLUSHDB",
            "HSET",
            "SADD",
            "SET",
            "RPUSH",
            "ZADD",
            "XADD",
            "CLIENT LIST",
            "INFO",
            "DBSIZE",
            "GET",
        ]
        .contains(&name.as_str())
        {
            continue;
        }
        assert!(READS.contains(&name.as_str()), "the server ran {name}");
    }
    assert!(!stats.contains("cmdstat_keys"));
}
