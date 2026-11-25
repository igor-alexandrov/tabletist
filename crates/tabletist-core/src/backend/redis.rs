//! What a Redis window asks of its session, and what comes back. Every
//! read runs on the session's queue, on its main connection, but a scan:
//! that runs as a task of its own on the scans' connection, steps
//! arriving as they are read, so a key opened meanwhile never waits for
//! it.

use std::time::{Duration, Instant};

use tabletist_db::redis::{Cursor, Handle, KeyInfo, KeyType, Loaded, Probe};
use tabletist_db::{Connection, Error, StopFlag};

use super::{Event, Outbox, RequestId, SessionId, lost_error};
use crate::keyspace::{ScanEnd, ScanStep};

/// What a Redis window asks.
#[derive(Debug)]
pub enum RedisAsk {
    /// The server, its databases and their keys, on `database`.
    Probe { database: u32 },
    /// Moves both connections to `database`.
    Select { database: u32 },
    /// Scans the names matching `pattern`, `count` at a time, from
    /// `cursor`: `DBSIZE` first when it starts from 0, a step at a time,
    /// until the cursor comes back, `limit` names have come (when there is
    /// one), or `stop` is set. `pace` is the wait between steps.
    Scan {
        pattern: String,
        count: u32,
        cursor: u64,
        limit: Option<u64>,
        pace: Option<Duration>,
        stop: StopFlag,
    },
    /// The types of `keys`, in one round trip.
    Types { keys: Vec<Vec<u8>> },
    /// A key's header and the first `count` of its value.
    Open { key: Vec<u8>, count: u32 },
    /// The next page of a key's value.
    Read {
        key: Vec<u8>,
        kind: KeyType,
        length: Option<u64>,
        from: Cursor,
        count: u32,
    },
}

/// What comes back for a [`RedisAsk`].
#[derive(Debug)]
pub enum RedisAnswer {
    Probed(Result<Probe, Error>),
    Selected {
        database: u32,
        result: Result<(), Error>,
    },
    /// One step of a scan: as many come as it takes.
    Scanned(Result<ScanStep, Error>),
    Types {
        keys: Vec<Vec<u8>>,
        result: Result<Vec<KeyType>, Error>,
    },
    Opened {
        key: Vec<u8>,
        result: Result<(KeyInfo, Loaded), Error>,
    },
    Read {
        key: Vec<u8>,
        result: Result<Loaded, Error>,
    },
}

/// The answer to `ask` that failed, or was never run, with `error`.
pub(super) fn failed(ask: RedisAsk, error: Error) -> RedisAnswer {
    match ask {
        RedisAsk::Probe { .. } => RedisAnswer::Probed(Err(error)),
        RedisAsk::Select { database } => RedisAnswer::Selected {
            database,
            result: Err(error),
        },
        RedisAsk::Scan { .. } => RedisAnswer::Scanned(Err(error)),
        RedisAsk::Types { keys } => RedisAnswer::Types {
            keys,
            result: Err(error),
        },
        RedisAsk::Open { key, .. } => RedisAnswer::Opened {
            key,
            result: Err(error),
        },
        RedisAsk::Read { key, .. } => RedisAnswer::Read {
            key,
            result: Err(error),
        },
    }
}

/// Runs `ask` on the session's `connection`, and says what it answered:
/// the error that lost the connection, where one did.
pub(super) async fn run(
    connection: &Connection,
    (session, request): (SessionId, RequestId),
    ask: &RedisAsk,
    outbox: &Outbox,
) -> Option<Error> {
    let emit = |answer: RedisAnswer| {
        outbox.emit(Event::Redis {
            session,
            request,
            answer,
        });
    };
    let Some(handle) = connection.redis() else {
        let error = Error::Unsupported("this connection is not a Redis one");
        emit(failed(clone_of(ask), error));
        return None;
    };
    match ask {
        RedisAsk::Probe { database } => {
            let result = handle.probe(*database).await;
            let lost = lost_error(&result);
            emit(RedisAnswer::Probed(result));
            lost
        }
        RedisAsk::Select { database } => {
            let result = handle.select(*database).await;
            let lost = lost_error(&result);
            emit(RedisAnswer::Selected {
                database: *database,
                result,
            });
            lost
        }
        RedisAsk::Scan {
            pattern,
            count,
            cursor,
            limit,
            pace,
            stop,
        } => {
            let scan = Scan {
                handle: handle.clone(),
                pattern: pattern.clone(),
                count: *count,
                cursor: *cursor,
                limit: *limit,
                pace: *pace,
                stop: stop.clone(),
            };
            let outbox = outbox.clone();
            tokio::spawn(async move {
                scan.run(|step| {
                    outbox.emit(Event::Redis {
                        session,
                        request,
                        answer: RedisAnswer::Scanned(step),
                    });
                })
                .await;
            });
            None
        }
        RedisAsk::Types { keys } => {
            let result = handle.types(keys).await;
            let lost = lost_error(&result);
            emit(RedisAnswer::Types {
                keys: keys.clone(),
                result,
            });
            lost
        }
        RedisAsk::Open { key, count } => {
            let result = open(handle, key, *count).await;
            let lost = lost_error(&result);
            emit(RedisAnswer::Opened {
                key: key.clone(),
                result,
            });
            lost
        }
        RedisAsk::Read {
            key,
            kind,
            length,
            from,
            count,
        } => {
            let result = handle.read(key, kind, *length, from, *count).await;
            let lost = lost_error(&result);
            emit(RedisAnswer::Read {
                key: key.clone(),
                result,
            });
            lost
        }
    }
}

/// A key's header, then its first page.
async fn open(handle: &Handle, key: &[u8], count: u32) -> Result<(KeyInfo, Loaded), Error> {
    let info = handle.describe(key).await?;
    let loaded = handle
        .read(key, &info.kind, info.length, &Cursor::Start, count)
        .await?;
    Ok((info, loaded))
}

/// The parts of `ask` that its failure names, for a session that has no
/// Redis connection to run it on.
fn clone_of(ask: &RedisAsk) -> RedisAsk {
    match ask {
        RedisAsk::Probe { database } => RedisAsk::Probe {
            database: *database,
        },
        RedisAsk::Select { database } => RedisAsk::Select {
            database: *database,
        },
        RedisAsk::Scan { stop, .. } => RedisAsk::Scan {
            pattern: String::new(),
            count: 0,
            cursor: 0,
            limit: None,
            pace: None,
            stop: stop.clone(),
        },
        RedisAsk::Types { keys } => RedisAsk::Types { keys: keys.clone() },
        RedisAsk::Open { key, count } => RedisAsk::Open {
            key: key.clone(),
            count: *count,
        },
        RedisAsk::Read {
            key,
            kind,
            length,
            from,
            count,
        } => RedisAsk::Read {
            key: key.clone(),
            kind: kind.clone(),
            length: *length,
            from: from.clone(),
            count: *count,
        },
    }
}

/// A scan as its task runs it.
struct Scan {
    handle: Handle,
    pattern: String,
    count: u32,
    cursor: u64,
    limit: Option<u64>,
    pace: Option<Duration>,
    stop: StopFlag,
}

impl Scan {
    /// Steps until the walk ends, sending each to `send`: `KEYS` is never
    /// used, so a large database never blocks the server.
    async fn run(self, send: impl Fn(Result<ScanStep, Error>)) {
        let mut cursor = self.cursor;
        let mut total = None;
        if cursor == 0 {
            match self.handle.dbsize().await {
                Ok(size) => total = Some(size),
                Err(error) => return send(Err(error)),
            }
        }
        let mut sent: u64 = 0;
        loop {
            if self.stop.is_stopped() {
                return send(Ok(ScanStep {
                    total: total.take(),
                    names: Vec::new(),
                    cursor,
                    end: ScanEnd::Stopped,
                    took: Duration::ZERO,
                }));
            }
            let started = Instant::now();
            let (next, names) = match self.handle.scan(cursor, &self.pattern, self.count).await {
                Ok(step) => step,
                Err(error) => return send(Err(error)),
            };
            sent += names.len() as u64;
            cursor = next;
            let end = if next == 0 {
                ScanEnd::Done
            } else if self.limit.is_some_and(|limit| sent >= limit) {
                ScanEnd::Limited
            } else {
                ScanEnd::More
            };
            send(Ok(ScanStep {
                total: total.take(),
                names,
                cursor,
                end,
                took: started.elapsed(),
            }));
            if end != ScanEnd::More {
                return;
            }
            if let Some(pace) = self.pace {
                tokio::time::sleep(pace).await;
            }
        }
    }
}

/// Against a `redis-server` each test starts (`fixtures::redis_server`);
/// without the binary each says "skipped" and passes.
#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use tabletist_db::fixtures::redis_server::TestServer;
    use tabletist_db::{Access, Browsing, ConnectSpec, HostKeys, Secrets};

    use super::*;
    use crate::backend::{Backend, Command, Waker};
    use crate::keyspace::{Browser, ScanState};
    use crate::secrets::Keyring;

    const WAIT: Duration = Duration::from_secs(60);

    /// A backend with a session on `server`, as `SessionId(1)`.
    fn connected(server: &TestServer) -> Backend {
        let mut backend = Backend::start_with(Waker::default(), Keyring::memory());
        backend.send(Command::Connect {
            session: SessionId(1),
            request: RequestId(1),
            spec: ConnectSpec::redis("127.0.0.1", server.port),
            secrets: Secrets::default(),
            host_keys: HostKeys::default(),
            access: Access::ReadOnly,
        });
        match backend.wait(WAIT) {
            Some(Event::Connected { .. }) => backend,
            other => panic!("expected Connected, got {other:?}"),
        }
    }

    /// Sends the scan `browser` is set for, as `request`, from `cursor`.
    fn scan(backend: &mut Backend, browser: &Browser, request: RequestId, limit: Option<u64>) {
        backend.send(Command::Redis {
            session: SessionId(1),
            request,
            ask: RedisAsk::Scan {
                pattern: browser.scan.pattern.clone(),
                count: browser.browsing.page,
                cursor: browser.scan.cursor,
                limit,
                pace: None,
                stop: browser.scan.stop.clone(),
            },
        });
    }

    /// Takes the scan's steps until it is no longer running.
    fn until_settled(backend: &mut Backend, browser: &mut Browser) {
        let deadline = Instant::now() + WAIT;
        while browser.scan.running() && Instant::now() < deadline {
            if let Some(Event::Redis {
                request,
                answer: RedisAnswer::Scanned(step),
                ..
            }) = backend.wait(WAIT)
            {
                browser.take_step(request, step);
            }
        }
    }

    #[test]
    fn a_large_database_stops_at_the_limit_and_scan_all_holds_the_rest() {
        let Some(server) = TestServer::start(&[]) else {
            return;
        };
        server.seed_many(0, 250_000);
        let mut backend = connected(&server);
        let mut browser = Browser::new(0, Browsing::default(), false);
        browser.restart(RequestId(2), "*");
        scan(&mut backend, &browser, RequestId(2), browser.limit());
        until_settled(&mut backend, &mut browser);
        assert_eq!(browser.scan.state, ScanState::Limited);
        assert_eq!(browser.scan.total, Some(250_000));
        let held = browser.tree.len();
        assert!((100_000..102_000).contains(&held), "{held} held");
        // The group counts what the sample holds.
        assert_eq!(
            browser.tree.group(b"item:").map(|group| group.count()),
            Some(held)
        );
        // Scan all goes on from the cursor, and holds every key.
        browser.resume(RequestId(3));
        scan(&mut backend, &browser, RequestId(3), None);
        until_settled(&mut backend, &mut browser);
        assert_eq!(browser.scan.state, ScanState::Done);
        assert_eq!(browser.tree.len(), 250_000);
        // KEYS was never sent.
        let stats = server.send(&["INFO", "commandstats"]).unwrap();
        assert!(stats.contains("cmdstat_scan"), "{stats}");
        assert!(!stats.contains("cmdstat_keys"), "{stats}");
    }

    #[test]
    fn stop_ends_the_scans_task() {
        let Some(server) = TestServer::start(&[]) else {
            return;
        };
        server.seed_many(0, 50_000);
        let mut backend = connected(&server);
        let mut browser = Browser::new(0, Browsing::default(), false);
        browser.restart(RequestId(2), "*");
        let stop = browser.scan.stop.clone();
        // Slow steps, so the stop lands while it runs.
        backend.send(Command::Redis {
            session: SessionId(1),
            request: RequestId(2),
            ask: RedisAsk::Scan {
                pattern: "*".into(),
                count: 500,
                cursor: 0,
                limit: None,
                pace: Some(Duration::from_millis(50)),
                stop: stop.clone(),
            },
        });
        match backend.wait(WAIT) {
            Some(Event::Redis {
                request,
                answer: RedisAnswer::Scanned(step),
                ..
            }) => {
                browser.take_step(request, step);
            }
            other => panic!("expected a step, got {other:?}"),
        }
        browser.stop();
        assert!(stop.is_stopped());
        // The task says it stopped, and sends nothing after.
        let ended = loop {
            match backend.wait(WAIT) {
                Some(Event::Redis {
                    answer: RedisAnswer::Scanned(Ok(step)),
                    ..
                }) if step.end != ScanEnd::More => break step.end,
                Some(_) => {}
                None => panic!("the scan never ended"),
            }
        };
        assert_eq!(ended, ScanEnd::Stopped);
        assert!(backend.wait(Duration::from_millis(300)).is_none());
        assert!(browser.tree.len() < 50_000);
    }

    #[test]
    fn a_pattern_holds_only_the_names_that_match() {
        let Some(server) = TestServer::start(&[]) else {
            return;
        };
        server.seed_many(0, 20_000);
        let mut backend = connected(&server);
        let mut browser = Browser::new(0, Browsing::default(), false);
        browser.restart(RequestId(2), "item:1*");
        scan(&mut backend, &browser, RequestId(2), browser.limit());
        until_settled(&mut backend, &mut browser);
        assert_eq!(browser.scan.state, ScanState::Done);
        // item:1, item:10–19, item:100–199, item:1000–1999, item:10000–19999.
        assert_eq!(browser.tree.len(), 11_111);
        assert!(
            browser
                .tree
                .names()
                .iter()
                .all(|name| name.starts_with(b"item:1"))
        );
    }

    #[test]
    fn a_key_opened_while_a_scan_runs_never_waits_for_it() {
        let Some(server) = TestServer::start(&[]) else {
            return;
        };
        server.seed_bookshop(None);
        server.seed_many(0, 20_000);
        let mut backend = connected(&server);
        // A slow scan: 40 steps 100 ms apart.
        let stop = StopFlag::new();
        backend.send(Command::Redis {
            session: SessionId(1),
            request: RequestId(3),
            ask: RedisAsk::Scan {
                pattern: "*".into(),
                count: 500,
                cursor: 0,
                limit: None,
                pace: Some(Duration::from_millis(100)),
                stop: stop.clone(),
            },
        });
        let asked = Instant::now();
        backend.send(Command::Redis {
            session: SessionId(1),
            request: RequestId(4),
            ask: RedisAsk::Open {
                key: b"book:4".to_vec(),
                count: 1000,
            },
        });
        let opened = loop {
            match backend.wait(WAIT) {
                Some(Event::Redis {
                    request: RequestId(4),
                    answer: RedisAnswer::Opened { result, .. },
                    ..
                }) => break result,
                Some(_) => {}
                None => panic!("the key never opened"),
            }
        };
        let (info, _) = opened.expect("book:4 opens");
        assert_eq!(info.kind, tabletist_db::redis::KeyType::Hash);
        assert!(
            asked.elapsed() < Duration::from_secs(2),
            "{:?}",
            asked.elapsed()
        );
        stop.stop();
    }
}
