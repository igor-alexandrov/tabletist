//! A Redis server for tests, started from the `redis-server` binary on a
//! free local port and stopped when it is dropped, and the Bookshop's
//! keys to seed it with. The tests write through a socket of their own:
//! the crate's connection sends only reads.
//!
//! Without the binary (or with `TABLETIST_TEST_REDIS_SERVER` naming one
//! that is not there) a test is skipped, as the other servers' are
//! without their variables.

use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

/// A running `redis-server`, stopped on drop.
pub struct TestServer {
    child: Child,
    pub port: u16,
    dir: std::path::PathBuf,
}

impl Drop for TestServer {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

/// The binary the tests start.
fn binary() -> String {
    std::env::var("TABLETIST_TEST_REDIS_SERVER").unwrap_or_else(|_| "redis-server".into())
}

/// A port nothing listens on, for now.
fn free_port() -> Option<u16> {
    let listener = TcpListener::bind("127.0.0.1:0").ok()?;
    Some(listener.local_addr().ok()?.port())
}

/// One command as RESP: an array of bulk strings.
pub fn encode(args: &[&[u8]]) -> Vec<u8> {
    let mut out = format!("*{}\r\n", args.len()).into_bytes();
    for arg in args {
        out.extend(format!("${}\r\n", arg.len()).into_bytes());
        out.extend_from_slice(arg);
        out.extend(b"\r\n");
    }
    out
}

/// Reads one RESP2 reply, flattened to its text: an error is `Err`.
fn reply(reader: &mut impl BufRead) -> std::io::Result<Result<String, String>> {
    let mut line = String::new();
    reader.read_line(&mut line)?;
    let line = line.trim_end_matches("\r\n");
    let (kind, rest) = line.split_at(line.len().min(1));
    Ok(match kind {
        "+" | ":" => Ok(rest.to_owned()),
        "-" => Err(rest.to_owned()),
        "$" => {
            let len: i64 = rest.parse().unwrap_or(-1);
            if len < 0 {
                Ok(String::new())
            } else {
                let mut buf = vec![0; usize::try_from(len).unwrap_or(0) + 2];
                reader.read_exact(&mut buf)?;
                buf.truncate(buf.len() - 2);
                Ok(String::from_utf8_lossy(&buf).into_owned())
            }
        }
        "*" => {
            let len: i64 = rest.parse().unwrap_or(0);
            let mut parts = Vec::new();
            for _ in 0..len.max(0) {
                match reply(reader)? {
                    Ok(part) => parts.push(part),
                    Err(error) => return Ok(Err(error)),
                }
            }
            Ok(parts.join(" "))
        }
        other => Err(format!("unexpected reply {other:?}")),
    })
}

impl TestServer {
    /// Starts a server with `args` beside its own (`--requirepass`,
    /// `--user`, TLS's). `None`, with a line saying so, without the binary.
    pub fn start(args: &[&str]) -> Option<Self> {
        let port = free_port()?;
        let dir =
            std::env::temp_dir().join(format!("tabletist-redis-{}-{port}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let spawned = Command::new(binary())
            .args(["--port", &port.to_string(), "--bind", "127.0.0.1"])
            .args(["--save", "", "--appendonly", "no", "--protected-mode", "no"])
            .arg("--dir")
            .arg(&dir)
            .args(args)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn();
        let child = match spawned {
            Ok(child) => child,
            Err(error) => {
                println!("skipped: no redis-server to start ({error})");
                return None;
            }
        };
        let server = Self { child, port, dir };
        let deadline = Instant::now() + Duration::from_secs(10);
        while Instant::now() < deadline {
            if TcpStream::connect(("127.0.0.1", port)).is_ok() {
                return Some(server);
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        println!("skipped: redis-server did not start on {port}");
        None
    }

    /// Sends each of `commands` on one socket, signed in with `auth`
    /// first where it is given, and returns their replies in order.
    pub fn send_all(
        &self,
        auth: Option<&str>,
        commands: &[Vec<Vec<u8>>],
    ) -> std::io::Result<Vec<Result<String, String>>> {
        let stream = TcpStream::connect(("127.0.0.1", self.port))?;
        stream.set_read_timeout(Some(Duration::from_secs(30)))?;
        let mut writer = stream.try_clone()?;
        let mut reader = BufReader::new(stream);
        if let Some(password) = auth {
            writer.write_all(&encode(&[b"AUTH", password.as_bytes()]))?;
            let _ = reply(&mut reader)?;
        }
        let mut replies = Vec::with_capacity(commands.len());
        for chunk in commands.chunks(1000) {
            let mut out = Vec::new();
            for command in chunk {
                let args: Vec<&[u8]> = command.iter().map(Vec::as_slice).collect();
                out.extend(encode(&args));
            }
            writer.write_all(&out)?;
            for _ in chunk {
                replies.push(reply(&mut reader)?);
            }
        }
        Ok(replies)
    }

    /// Sends one command, unauthenticated.
    pub fn send(&self, args: &[&str]) -> Result<String, String> {
        let command = args.iter().map(|arg| arg.as_bytes().to_vec()).collect();
        self.send_all(None, &[command])
            .map_err(|error| error.to_string())?
            .remove(0)
    }

    /// Fills database 0 with the Bookshop's keys ([`bookshop`]).
    pub fn seed_bookshop(&self, auth: Option<&str>) {
        let commands = bookshop();
        if let Ok(replies) = self.send_all(auth, &commands) {
            for reply in replies {
                assert!(reply.is_ok(), "seeding failed: {reply:?}");
            }
        }
    }

    /// Fills database `db` with `count` plain keys, `item:<n>`.
    pub fn seed_many(&self, db: u32, count: u64) {
        let mut commands = vec![vec![b"SELECT".to_vec(), db.to_string().into_bytes()]];
        let mut batch = vec![b"MSET".to_vec()];
        for n in 0..count {
            batch.push(format!("item:{n}").into_bytes());
            batch.push(b"1".to_vec());
            if batch.len() > 2000 {
                commands.push(std::mem::replace(&mut batch, vec![b"MSET".to_vec()]));
            }
        }
        if batch.len() > 1 {
            commands.push(batch);
        }
        let replies = self.send_all(None, &commands).unwrap_or_default();
        assert!(replies.iter().all(Result::is_ok), "seeding failed");
    }
}

/// The Bookshop's books, as the SQL fixtures have them: id, title,
/// publisher, price.
const BOOKS: [(u32, &str, u32, &str); 12] = [
    (1, "The Salt Road", 1, "18.50"),
    (2, "A Field Guide to Fog", 1, "24.00"),
    (3, "Lanterns", 2, "12.99"),
    (4, "The Cartographer's Daughter", 2, "21.75"),
    (5, "Small Engines", 3, "15.00"),
    (6, "Winter Harbour", 3, "19.99"),
    (7, "Notes on Bread", 4, "30.00"),
    (8, "Ninety-One Doors", 4, "16.40"),
    (9, "The Quiet Ledger", 5, "9.99"),
    (10, "Tidewater", 2, "22.00"),
    (11, "An Atlas of Small Things", 1, "27.30"),
    (12, "100% Rain", 5, "11.25"),
];

/// The Bookshop as Redis keys: `book:<id>` hashes, `cart:<id>` hashes,
/// `session:<id>` strings that expire, `tags:<id>` sets, `queue:emails`
/// (a list), `top:books` (a sorted set), `events` (a stream), `config` (a
/// string of JSON). 65 keys in all.
pub fn bookshop() -> Vec<Vec<Vec<u8>>> {
    let args = |words: &[&str]| words.iter().map(|word| word.as_bytes().to_vec()).collect();
    let mut commands: Vec<Vec<Vec<u8>>> = vec![args(&["FLUSHDB"])];
    for (id, title, publisher, price) in BOOKS {
        let key = format!("book:{id}");
        let publisher = publisher.to_string();
        let stock = (id * 3 % 17).to_string();
        commands.push(args(&[
            "HSET",
            &key,
            "title",
            title,
            "publisher_id",
            &publisher,
            "format",
            if id % 2 == 0 {
                "hardcover"
            } else {
                "paperback"
            },
            "price",
            price,
            "stock",
            &stock,
            "updated_at",
            "2026-10-30T14:02:11Z",
        ]));
        let tags = format!("tags:{id}");
        commands.push(args(&["SADD", &tags, "fiction", "new", "staff-pick"]));
    }
    // A book's reviews nest a group under `book:`.
    commands.push(args(&[
        "HSET",
        "book:reviews:12",
        "stars",
        "5",
        "by",
        "reader_7",
    ]));
    commands.push(args(&[
        "SADD",
        "tags:all",
        "fiction",
        "new",
        "staff-pick",
        "poetry",
    ]));
    for cart in 1..=4 {
        let key = format!("cart:{cart}");
        commands.push(args(&["HSET", &key, "book:3", "1", "book:7", "2"]));
    }
    for session in 1..=31 {
        let key = format!("session:{session:04}");
        let value = format!(
            "{{\"customer_id\":{session},\"cart\":\"cart:{}\"}}",
            session % 4 + 1
        );
        commands.push(args(&["SET", &key, &value, "EX", "720"]));
    }
    commands.push(args(&[
        "RPUSH",
        "queue:emails",
        "order-confirmed:1001",
        "order-shipped:1001",
        "password-reset:7",
    ]));
    commands.push(args(&[
        "ZADD",
        "top:books",
        "3",
        "book:9",
        "12",
        "book:1",
        "27",
        "book:4",
        "41",
        "book:11",
    ]));
    for (n, kind) in ["order_placed", "order_paid", "order_shipped"]
        .iter()
        .enumerate()
    {
        let id = format!("1761832931000-{n}");
        commands.push(args(&[
            "XADD", "events", &id, "kind", kind, "order", "1001",
        ]));
    }
    commands.push(args(&[
        "SET",
        "config",
        r#"{"currency":"EUR","shipping":{"free_over":40,"zones":["eu","uk"]}}"#,
    ]));
    commands
}
