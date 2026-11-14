//! What to connect to. Secrets are kept apart so a spec can be saved to disk.

use std::fmt;
use std::path::PathBuf;

use percent_encoding::percent_decode_str;
use serde::{Deserialize, Serialize};

use crate::{Error, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Driver {
    #[default]
    Postgres,
    #[serde(rename = "mysql")]
    MySql,
    Sqlite,
    BigQuery,
    /// A standalone Redis server (or one that speaks its protocol:
    /// Valkey, KeyDB, Dragonfly). Keys and values, no SQL.
    Redis,
}

impl Driver {
    pub fn default_port(self) -> u16 {
        match self {
            Self::Postgres => 5432,
            Self::MySql => 3306,
            Self::Redis => 6379,
            Self::Sqlite | Self::BigQuery => 0,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Postgres => "PostgreSQL",
            Self::MySql => "MySQL",
            Self::Sqlite => "SQLite",
            Self::BigQuery => "BigQuery",
            Self::Redis => "Redis",
        }
    }

    /// The SQL dialect the driver speaks: none for Redis, which has no
    /// SQL, so nothing that writes or reads SQL is offered for it.
    pub fn dialect(self) -> Option<crate::Dialect> {
        match self {
            Self::Postgres => Some(crate::Dialect::Postgres),
            Self::MySql => Some(crate::Dialect::MySql),
            Self::Sqlite => Some(crate::Dialect::Sqlite),
            Self::BigQuery => Some(crate::Dialect::BigQuery),
            Self::Redis => None,
        }
    }
}

/// What a Redis connection has beside its host, port and user: the
/// database it opens on, its TLS, and how its keys are browsed.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct RedisSpec {
    /// The database the connection opens on (`SELECT n`); the window can
    /// switch to another while it is open.
    pub database: u32,
    pub tls: RedisTls,
    pub browsing: Browsing,
}

/// Redis's TLS (`rediss://`): off, or on with the server's certificate
/// checked against the CA file (else the system's roots), its name
/// checked unless `verify_host` is off, and a client certificate where
/// the server asks for one.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct RedisTls {
    pub enabled: bool,
    pub ca: Option<PathBuf>,
    pub cert: Option<PathBuf>,
    pub key: Option<PathBuf>,
    pub verify_host: bool,
}

impl Default for RedisTls {
    fn default() -> Self {
        Self {
            enabled: false,
            ca: None,
            cert: None,
            key: None,
            verify_host: true,
        }
    }
}

/// How a Redis connection lists its keys: grouped by `separator`,
/// `page` at a time (the SCAN COUNT hint, and the page of an open group),
/// the names that match `pattern`, and at most `key_limit` of them held
/// before the scan stops and asks.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Browsing {
    pub separator: String,
    pub page: u32,
    pub pattern: String,
    pub key_limit: u64,
}

/// The pages a Redis connection can list its keys by.
pub const KEY_PAGES: [u32; 3] = [500, 1000, 5000];

impl Default for Browsing {
    fn default() -> Self {
        Self {
            separator: ":".into(),
            page: 1000,
            pattern: "*".into(),
            key_limit: 100_000,
        }
    }
}

/// libpq's `sslmode` values; MySQL's `ssl-mode` maps onto them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TlsMode {
    Disable,
    #[default]
    Prefer,
    Require,
    VerifyCa,
    VerifyFull,
}

impl TlsMode {
    fn parse(value: &str) -> Option<Self> {
        match value.to_ascii_lowercase().replace('_', "-").as_str() {
            "disable" | "disabled" => Some(Self::Disable),
            "prefer" | "preferred" | "allow" => Some(Self::Prefer),
            "require" | "required" => Some(Self::Require),
            "verify-ca" => Some(Self::VerifyCa),
            "verify-full" | "verify-identity" => Some(Self::VerifyFull),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "method", rename_all = "kebab-case")]
pub enum SshAuth {
    Password,
    /// An empty path takes `~/.ssh/config`'s IdentityFile.
    KeyFile {
        path: PathBuf,
    },
    Agent,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SshSpec {
    /// A host name, or a Host alias from `~/.ssh/config`.
    pub host: String,
    /// `None` takes the config's Port, else 22.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub port: Option<u16>,
    /// Empty takes the config's User, else the login name.
    pub user: String,
    pub auth: SshAuth,
}

/// Everything needed to connect except secrets. Saved in the app's
/// connections.toml, in a form of the app's own: what is derived here
/// reads the connections.json of older versions.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ConnectSpec {
    pub driver: Driver,
    pub host: String,
    pub port: u16,
    pub user: String,
    /// The database name; for BigQuery, the GCP project that is billed.
    pub database: String,
    pub sqlite_path: Option<PathBuf>,
    /// A service-account key file (JSON) for BigQuery. `None` there uses
    /// Google Cloud's application default credentials.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bigquery_key: Option<PathBuf>,
    pub tls: TlsMode,
    pub ca_file: Option<PathBuf>,
    pub ssh: Option<SshSpec>,
    /// Redis's own settings; `None` for every other driver.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub redis: Option<RedisSpec>,
}

impl Default for ConnectSpec {
    fn default() -> Self {
        Self {
            driver: Driver::Postgres,
            host: "localhost".into(),
            port: Driver::Postgres.default_port(),
            user: String::new(),
            database: String::new(),
            sqlite_path: None,
            bigquery_key: None,
            tls: TlsMode::Prefer,
            ca_file: None,
            ssh: None,
            redis: None,
        }
    }
}

/// Passwords and passphrases. Never saved with the spec, never printed.
#[derive(Clone, Default, PartialEq, Eq)]
pub struct Secrets {
    pub password: Option<String>,
    pub ssh_password: Option<String>,
    pub ssh_passphrase: Option<String>,
}

impl fmt::Debug for Secrets {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Secrets { .. }")
    }
}

fn decode(text: &str) -> String {
    percent_decode_str(text).decode_utf8_lossy().into_owned()
}

/// A connection URL, split into the spec, its secrets, and which TLS
/// settings it named (a URL without them should not change a form's).
#[derive(Debug, Clone)]
pub struct ParsedUrl {
    /// Missing TLS settings take their defaults here.
    pub spec: ConnectSpec,
    pub secrets: Secrets,
    /// Whether the URL named a TLS mode (`sslmode`, `ssl-mode`, `ssl_mode`).
    pub names_tls: bool,
    /// Whether the URL named a CA file (`sslrootcert`, `ssl-ca`).
    pub names_ca_file: bool,
}

impl ParsedUrl {
    /// Parses `postgres://`, `postgresql://`, `mysql://`, `mariadb://` and
    /// `sqlite:` URLs. A TLS setting named twice (under any of its
    /// spellings) is an error rather than the last one silently winning.
    pub fn parse(url: &str) -> Result<Self> {
        let url = url.trim();
        if let Some(line) = redis_cli_line(url) {
            return parse_redis_cli(line);
        }
        if let Some(rest) = url.strip_prefix("sqlite:") {
            let path = rest.strip_prefix("//").unwrap_or(rest);
            if path.is_empty() {
                return Err(Error::InvalidSpec("the SQLite URL names no file".into()));
            }
            return Ok(Self {
                spec: ConnectSpec::sqlite(decode(path)),
                secrets: Secrets::default(),
                names_tls: false,
                names_ca_file: false,
            });
        }
        let parsed = url::Url::parse(url)
            .map_err(|error| Error::InvalidSpec(format!("not a connection URL: {error}")))?;
        let driver = match parsed.scheme() {
            "redis" | "rediss" => return parse_redis_url(&parsed),
            "postgres" | "postgresql" => Driver::Postgres,
            "mysql" | "mariadb" => Driver::MySql,
            "bigquery" => {
                // bigquery:project?credentials=/path/key.json — there is
                // no host, user or password to take from the URL.
                let project = parsed
                    .host_str()
                    .map(str::to_owned)
                    .filter(|host| !host.is_empty())
                    .unwrap_or_else(|| decode(parsed.path().trim_start_matches('/')));
                if project.is_empty() {
                    return Err(Error::InvalidSpec(
                        "the BigQuery URL names no project".into(),
                    ));
                }
                let mut key = None;
                for (name, value) in parsed.query_pairs() {
                    if name == "credentials" {
                        key = Some(PathBuf::from(value.as_ref()));
                    } else {
                        log::debug!("ignoring URL parameter {name}");
                    }
                }
                return Ok(Self {
                    spec: ConnectSpec::bigquery(project, key),
                    secrets: Secrets::default(),
                    names_tls: false,
                    names_ca_file: false,
                });
            }
            other => {
                return Err(Error::InvalidSpec(format!(
                    "unsupported scheme {other:?}; use postgres://, mysql://, redis://, sqlite: or bigquery:"
                )));
            }
        };
        let host = parsed
            .host_str()
            .unwrap_or("localhost")
            .trim_start_matches('[')
            .trim_end_matches(']')
            .to_owned();
        let mut spec = ConnectSpec {
            driver,
            host,
            port: parsed.port().unwrap_or(driver.default_port()),
            user: decode(parsed.username()),
            database: decode(parsed.path().trim_start_matches('/')),
            ..ConnectSpec::default()
        };
        let (mut names_tls, mut names_ca_file) = (false, false);
        for (key, value) in parsed.query_pairs() {
            let named = match key.as_ref() {
                "sslmode" | "ssl-mode" | "ssl_mode" => {
                    spec.tls = TlsMode::parse(&value)
                        .ok_or_else(|| Error::InvalidSpec(format!("unknown TLS mode {value:?}")))?;
                    &mut names_tls
                }
                "sslrootcert" | "ssl-ca" => {
                    spec.ca_file = Some(PathBuf::from(value.as_ref()));
                    &mut names_ca_file
                }
                _ => {
                    log::debug!("ignoring URL parameter {key}");
                    continue;
                }
            };
            if std::mem::replace(named, true) {
                return Err(Error::InvalidSpec(format!(
                    "the URL sets {key} more than once"
                )));
            }
        }
        let secrets = Secrets {
            password: parsed.password().map(decode),
            ..Secrets::default()
        };
        Ok(Self {
            spec,
            secrets,
            names_tls,
            names_ca_file,
        })
    }
}

/// The rest of `text` after `redis-cli` and a space, when it is a
/// redis-cli command line.
fn redis_cli_line(text: &str) -> Option<&str> {
    let rest = text.strip_prefix("redis-cli")?;
    (rest.is_empty() || rest.starts_with(char::is_whitespace)).then_some(rest)
}

/// `redis://[username[:password]@]host[:port][/db]`; `rediss://` turns
/// TLS on.
fn parse_redis_url(parsed: &url::Url) -> Result<ParsedUrl> {
    let host = parsed
        .host_str()
        .unwrap_or("localhost")
        .trim_start_matches('[')
        .trim_end_matches(']');
    let mut spec = ConnectSpec::redis(
        if host.is_empty() { "localhost" } else { host },
        parsed.port().unwrap_or(Driver::Redis.default_port()),
    );
    spec.user = decode(parsed.username());
    let redis = spec.redis.get_or_insert_with(RedisSpec::default);
    redis.tls.enabled = parsed.scheme() == "rediss";
    let path = parsed.path().trim_matches('/');
    if !path.is_empty() {
        redis.database = path
            .parse()
            .map_err(|_| Error::InvalidSpec(format!("{path:?} is not a database number")))?;
    }
    // `?db=` names the database too. Other parameters are left alone, and
    // never logged: a stray `?word` would log the word.
    for (key, value) in parsed.query_pairs() {
        if key == "db" {
            redis.database = value
                .parse()
                .map_err(|_| Error::InvalidSpec(format!("{value:?} is not a database number")))?;
        }
    }
    Ok(ParsedUrl {
        spec,
        secrets: Secrets {
            password: parsed.password().map(decode),
            ..Secrets::default()
        },
        names_tls: true,
        names_ca_file: false,
    })
}

/// Splits a command line into its words as a shell would for the simple
/// cases: blanks part them, and single or double quotes keep blanks in.
fn words(line: &str) -> Result<Vec<String>> {
    let mut words = Vec::new();
    let mut word = String::new();
    let mut quote = None;
    let mut started = false;
    let mut chars = line.chars();
    while let Some(c) = chars.next() {
        match (quote, c) {
            (Some(q), c) if c == q => quote = None,
            (Some('"'), '\\') => word.extend(chars.next()),
            (Some(_), c) => word.push(c),
            (None, '\'' | '"') => {
                quote = Some(c);
                started = true;
            }
            (None, '\\') => {
                word.extend(chars.next());
                started = true;
            }
            (None, c) if c.is_whitespace() => {
                if started {
                    words.push(std::mem::take(&mut word));
                    started = false;
                }
            }
            (None, c) => {
                word.push(c);
                started = true;
            }
        }
    }
    if quote.is_some() {
        return Err(Error::InvalidSpec("a quote is not closed".into()));
    }
    if started {
        words.push(word);
    }
    Ok(words)
}

/// `redis-cli -h … -p … -n … [-a …] [--user …] [--pass …] [--tls]
/// [--cacert …] [--cert …] [--key …] [-u URL]`. Anything else on the line
/// (a command to run, other options) is left alone.
fn parse_redis_cli(line: &str) -> Result<ParsedUrl> {
    let mut parsed = ParsedUrl {
        spec: ConnectSpec::redis("127.0.0.1", Driver::Redis.default_port()),
        secrets: Secrets::default(),
        names_tls: true,
        names_ca_file: false,
    };
    let words = words(line)?;
    let mut words = words.into_iter();
    while let Some(word) = words.next() {
        let mut value = |name: &str| {
            words
                .next()
                .ok_or_else(|| Error::InvalidSpec(format!("{name} needs a value")))
        };
        match word.as_str() {
            "-u" => {
                let url = value("-u")?;
                let from = ParsedUrl::parse(&url)?;
                match from.spec.driver {
                    Driver::Redis => parsed = from,
                    Driver::Postgres | Driver::MySql | Driver::Sqlite | Driver::BigQuery => {
                        return Err(Error::InvalidSpec("-u takes a redis:// URL".into()));
                    }
                }
            }
            "-h" => parsed.spec.host = value("-h")?,
            "-p" => {
                let port = value("-p")?;
                parsed.spec.port = port
                    .parse()
                    .map_err(|_| Error::InvalidSpec(format!("{port:?} is not a port")))?;
            }
            "-n" => {
                let db = value("-n")?;
                parsed
                    .spec
                    .redis
                    .get_or_insert_with(RedisSpec::default)
                    .database = db
                    .parse()
                    .map_err(|_| Error::InvalidSpec(format!("{db:?} is not a database number")))?;
            }
            "-a" | "--pass" => parsed.secrets.password = Some(value(&word)?),
            "--user" => parsed.spec.user = value("--user")?,
            "--tls" => {
                parsed
                    .spec
                    .redis
                    .get_or_insert_with(RedisSpec::default)
                    .tls
                    .enabled = true;
            }
            "--cacert" | "--cert" | "--key" => {
                let path = PathBuf::from(value(&word)?);
                let tls = &mut parsed.spec.redis.get_or_insert_with(RedisSpec::default).tls;
                match word.as_str() {
                    "--cacert" => {
                        tls.ca = Some(path);
                        parsed.names_ca_file = true;
                    }
                    "--cert" => tls.cert = Some(path),
                    _ => tls.key = Some(path),
                }
            }
            // Options with a value that says nothing of the connection.
            "-r" | "-i" | "-d" | "-D" | "--sni" | "--cacertdir" | "--eval" | "--pattern"
            | "--count" | "--quoted-pattern" => {
                let _ = value(&word)?;
            }
            // Other words (a command to run, options) are left alone, and
            // never logged: one may be a password (`AUTH user pass`).
            _ => {}
        }
    }
    Ok(parsed)
}

impl ConnectSpec {
    /// A standalone Redis server at `host:port`, on database 0, as the
    /// default user, with no TLS.
    pub fn redis(host: impl Into<String>, port: u16) -> Self {
        Self {
            driver: Driver::Redis,
            host: host.into(),
            port,
            tls: TlsMode::Disable,
            redis: Some(RedisSpec::default()),
            ..Self::default()
        }
    }

    /// The Redis settings of a Redis spec, its defaults where a spec read
    /// from an older file has none.
    pub fn redis_spec(&self) -> RedisSpec {
        self.redis.clone().unwrap_or_default()
    }

    /// The Redis URL of the spec, without its password: the URL tab
    /// shows it, and the password stays in the keyring.
    pub fn redis_url(&self) -> String {
        use percent_encoding::{AsciiSet, CONTROLS, utf8_percent_encode};
        // What a user name cannot hold as it is in a URL's userinfo.
        const USER: &AsciiSet = &CONTROLS
            .add(b' ')
            .add(b'"')
            .add(b'#')
            .add(b'%')
            .add(b'/')
            .add(b':')
            .add(b'<')
            .add(b'>')
            .add(b'?')
            .add(b'@')
            .add(b'[')
            .add(b'\\')
            .add(b']')
            .add(b'^')
            .add(b'`')
            .add(b'{')
            .add(b'|')
            .add(b'}');
        let redis = self.redis_spec();
        let scheme = if redis.tls.enabled { "rediss" } else { "redis" };
        let mut url = format!("{scheme}://");
        if !self.user.is_empty() {
            url.push_str(&utf8_percent_encode(&self.user, USER).to_string());
            url.push('@');
        }
        if self.host.contains(':') {
            url.push_str(&format!("[{}]", self.host));
        } else {
            url.push_str(&self.host);
        }
        url.push_str(&format!(":{}/{}", self.port, redis.database));
        url
    }

    pub fn sqlite(path: impl Into<PathBuf>) -> Self {
        Self {
            driver: Driver::Sqlite,
            host: String::new(),
            port: 0,
            sqlite_path: Some(path.into()),
            ..Self::default()
        }
    }

    /// A BigQuery connection: the billed project, and a service-account
    /// key file where one is chosen (else application default credentials).
    pub fn bigquery(project: impl Into<String>, key: Option<PathBuf>) -> Self {
        Self {
            driver: Driver::BigQuery,
            host: String::new(),
            port: 0,
            database: project.into(),
            bigquery_key: key,
            ..Self::default()
        }
    }

    /// Parses `postgres://`, `postgresql://`, `mysql://`, `mariadb://` and
    /// `sqlite:` URLs. The password, if any, comes back in `Secrets`.
    pub fn from_url(url: &str) -> Result<(Self, Secrets)> {
        let parsed = ParsedUrl::parse(url)?;
        Ok((parsed.spec, parsed.secrets))
    }

    /// The TLS mode as it is enforced: PostgreSQL treats `require` with a CA
    /// file as `verify-ca`, like libpq (MySQL refuses that combination).
    /// Redis's TLS is off, or checks the chain, and the host's name unless
    /// it is told not to.
    pub fn effective_tls(&self) -> TlsMode {
        match (self.driver, self.tls, &self.ca_file) {
            (Driver::Redis, _, _) => {
                let tls = self.redis_spec().tls;
                match (tls.enabled, tls.verify_host) {
                    (false, _) => TlsMode::Disable,
                    (true, true) => TlsMode::VerifyFull,
                    (true, false) => TlsMode::VerifyCa,
                }
            }
            (Driver::Postgres, TlsMode::Require, Some(_)) => TlsMode::VerifyCa,
            (Driver::Postgres | Driver::MySql | Driver::Sqlite | Driver::BigQuery, tls, _) => tls,
        }
    }

    /// A one-line description without secrets: `user@host:port/db`, or the
    /// file name for SQLite.
    pub fn summary(&self) -> String {
        match self.driver {
            Driver::Sqlite => {
                return self
                    .sqlite_path
                    .as_ref()
                    .and_then(|path| path.file_name())
                    .map(|name| name.to_string_lossy().into_owned())
                    .unwrap_or_default();
            }
            // The project alone: there is no host, port or user.
            Driver::BigQuery => return self.database.clone(),
            Driver::Postgres | Driver::MySql | Driver::Redis => {}
        }
        let mut text = String::new();
        if !self.user.is_empty() {
            text.push_str(&self.user);
            text.push('@');
        }
        text.push_str(&format!("{}:{}", self.host, self.port));
        match self.driver {
            Driver::Redis => text.push_str(&format!("/{}", self.redis_spec().database)),
            Driver::Postgres | Driver::MySql | Driver::Sqlite | Driver::BigQuery => {
                if !self.database.is_empty() {
                    text.push('/');
                    text.push_str(&self.database);
                }
            }
        }
        if let Some(ssh) = &self.ssh {
            text.push_str(" via ");
            if !ssh.user.is_empty() {
                text.push_str(&ssh.user);
                text.push('@');
            }
            text.push_str(&ssh.host);
        }
        text
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_driver_has_its_dialect() {
        assert_eq!(Driver::Postgres.dialect(), Some(crate::Dialect::Postgres));
        assert_eq!(Driver::MySql.dialect(), Some(crate::Dialect::MySql));
        assert_eq!(Driver::Sqlite.dialect(), Some(crate::Dialect::Sqlite));
        assert_eq!(Driver::Redis.dialect(), None);
    }

    #[test]
    fn redis_urls_parse_their_user_database_and_tls() {
        let parsed = ParsedUrl::parse("redis://localhost").unwrap();
        assert_eq!(parsed.spec, ConnectSpec::redis("localhost", 6379));
        assert_eq!(parsed.secrets.password, None);

        let parsed =
            ParsedUrl::parse("rediss://app%40corp:p%40ss@cache.example.com:6380/2").unwrap();
        let redis = parsed.spec.redis_spec();
        assert_eq!(parsed.spec.driver, Driver::Redis);
        assert_eq!(parsed.spec.host, "cache.example.com");
        assert_eq!(parsed.spec.port, 6380);
        assert_eq!(parsed.spec.user, "app@corp");
        assert_eq!(redis.database, 2);
        assert!(redis.tls.enabled);
        assert_eq!(parsed.secrets.password.as_deref(), Some("p@ss"));

        // A password alone, for the default user.
        let parsed = ParsedUrl::parse("redis://:secret@h:6379/0").unwrap();
        assert_eq!(parsed.spec.user, "");
        assert_eq!(parsed.secrets.password.as_deref(), Some("secret"));

        assert!(matches!(
            ParsedUrl::parse("redis://h/sixteen"),
            Err(Error::InvalidSpec(_))
        ));
    }

    #[test]
    fn a_redis_url_round_trips_without_its_password() {
        let parsed = ParsedUrl::parse("rediss://reader:hunter2@cache.example.com:6380/3").unwrap();
        let url = parsed.spec.redis_url();
        assert_eq!(url, "rediss://reader@cache.example.com:6380/3");
        assert!(!url.contains("hunter2"));
        assert_eq!(ParsedUrl::parse(&url).unwrap().spec, parsed.spec);
        assert_eq!(parsed.spec.summary(), "reader@cache.example.com:6380/3");
        assert_eq!(
            ConnectSpec::redis("::1", 6379).redis_url(),
            "redis://[::1]:6379/0"
        );
    }

    #[test]
    fn redis_cli_lines_parse_as_connections() {
        let parsed = ParsedUrl::parse(
            "redis-cli -h cache.example.com -p 6380 -n 4 -a 'pa ss' --user reader --tls --cacert /etc/ca.pem",
        )
        .unwrap();
        let redis = parsed.spec.redis_spec();
        assert_eq!(parsed.spec.driver, Driver::Redis);
        assert_eq!(parsed.spec.host, "cache.example.com");
        assert_eq!(parsed.spec.port, 6380);
        assert_eq!(parsed.spec.user, "reader");
        assert_eq!(redis.database, 4);
        assert!(redis.tls.enabled);
        assert_eq!(redis.tls.ca, Some(PathBuf::from("/etc/ca.pem")));
        assert_eq!(parsed.secrets.password.as_deref(), Some("pa ss"));

        let parsed = ParsedUrl::parse("redis-cli").unwrap();
        assert_eq!(parsed.spec, ConnectSpec::redis("127.0.0.1", 6379));
        let parsed = ParsedUrl::parse("redis-cli -u redis://h:7000/1 GET config").unwrap();
        assert_eq!((parsed.spec.host.as_str(), parsed.spec.port), ("h", 7000));
        assert_eq!(parsed.spec.redis_spec().database, 1);
        assert!(matches!(
            ParsedUrl::parse("redis-cli -p"),
            Err(Error::InvalidSpec(_))
        ));
        assert!(matches!(
            ParsedUrl::parse("redis-cli -p http"),
            Err(Error::InvalidSpec(_))
        ));
        // Another program's line is no redis-cli line.
        assert!(ParsedUrl::parse("redis-client -h x").is_err());
    }

    #[test]
    fn postgres_urls_parse_with_decoded_parts() {
        let (spec, secrets) = ConnectSpec::from_url(
            "postgres://app%40corp:p%40ss%2Fw0rd@db.example.com:6543/my%20db?sslmode=verify-full&sslrootcert=/etc/ca.pem",
        )
        .unwrap();
        assert_eq!(spec.driver, Driver::Postgres);
        assert_eq!(spec.host, "db.example.com");
        assert_eq!(spec.port, 6543);
        assert_eq!(spec.user, "app@corp");
        assert_eq!(spec.database, "my db");
        assert_eq!(spec.tls, TlsMode::VerifyFull);
        assert_eq!(spec.ca_file, Some(PathBuf::from("/etc/ca.pem")));
        assert_eq!(secrets.password.as_deref(), Some("p@ss/w0rd"));
    }

    #[test]
    fn postgresql_scheme_and_default_port_work() {
        let (spec, secrets) = ConnectSpec::from_url("postgresql://localhost/app").unwrap();
        assert_eq!(spec.driver, Driver::Postgres);
        assert_eq!(spec.port, 5432);
        assert_eq!(spec.user, "");
        assert_eq!(spec.tls, TlsMode::Prefer);
        assert_eq!(secrets.password, None);
    }

    #[test]
    fn mysql_urls_parse_their_ssl_mode() {
        let (spec, _) =
            ConnectSpec::from_url("mysql://root@127.0.0.1/shop?ssl-mode=REQUIRED").unwrap();
        assert_eq!(spec.driver, Driver::MySql);
        assert_eq!(spec.port, 3306);
        assert_eq!(spec.tls, TlsMode::Require);
        let (spec, _) = ConnectSpec::from_url("mariadb://h/db?ssl-mode=VERIFY_IDENTITY").unwrap();
        assert_eq!(spec.tls, TlsMode::VerifyFull);
    }

    #[test]
    fn ipv6_hosts_lose_their_brackets() {
        let (spec, _) = ConnectSpec::from_url("postgres://[::1]:5433/db").unwrap();
        assert_eq!(spec.host, "::1");
    }

    #[test]
    fn sqlite_urls_name_a_file() {
        let (spec, _) = ConnectSpec::from_url("sqlite:///var/data/app.db").unwrap();
        assert_eq!(spec.driver, Driver::Sqlite);
        assert_eq!(spec.sqlite_path, Some(PathBuf::from("/var/data/app.db")));
        let (spec, _) = ConnectSpec::from_url("sqlite:relative.db").unwrap();
        assert_eq!(spec.sqlite_path, Some(PathBuf::from("relative.db")));
    }

    #[test]
    fn bad_urls_are_invalid_specs() {
        for url in [
            "",
            "sqlite:",
            "oracle://h/db",
            "not a url",
            "postgres://h/db?sslmode=sometimes",
        ] {
            assert!(
                matches!(ConnectSpec::from_url(url), Err(Error::InvalidSpec(_))),
                "{url:?} should be rejected"
            );
        }
    }

    #[test]
    fn postgres_require_with_a_ca_file_is_verify_ca() {
        let (mut spec, _) = ConnectSpec::from_url("postgres://h/db?sslmode=require").unwrap();
        assert_eq!(spec.effective_tls(), TlsMode::Require);
        spec.ca_file = Some("/ca.pem".into());
        assert_eq!(spec.effective_tls(), TlsMode::VerifyCa);
        spec.driver = Driver::MySql;
        assert_eq!(spec.effective_tls(), TlsMode::Require);
    }

    #[test]
    fn a_tls_setting_named_twice_is_refused() {
        for url in [
            "postgres://h/db?sslmode=verify-full&sslmode=disable",
            "mysql://h/db?ssl-mode=VERIFY_IDENTITY&ssl_mode=DISABLED",
            "postgres://h/db?sslrootcert=/a.pem&ssl-ca=/b.pem",
        ] {
            assert!(
                matches!(ParsedUrl::parse(url), Err(Error::InvalidSpec(_))),
                "{url:?} should be rejected"
            );
        }
    }

    #[test]
    fn parsed_urls_say_which_tls_settings_they_name() {
        let parsed = ParsedUrl::parse("postgres://me@h/db").unwrap();
        assert!(!parsed.names_tls && !parsed.names_ca_file);
        let parsed =
            ParsedUrl::parse("postgres://me@h/db?sslmode=require&sslrootcert=/ca.pem").unwrap();
        assert!(parsed.names_tls && parsed.names_ca_file);
    }

    #[test]
    fn a_urls_password_is_percent_decoded_into_the_secrets() {
        let parsed = ParsedUrl::parse("postgres://me:s%40cret@db.example.com:6543/app").unwrap();
        assert_eq!(parsed.secrets.password.as_deref(), Some("s@cret"));
        let parsed = ParsedUrl::parse(" mysql://me@h/db ").unwrap();
        assert_eq!(parsed.secrets.password, None);
    }

    #[test]
    fn summaries_never_include_secrets() {
        let (spec, _) = ConnectSpec::from_url("postgres://me:secret@h:5432/db").unwrap();
        assert_eq!(spec.summary(), "me@h:5432/db");
        assert!(!spec.summary().contains("secret"));
        assert_eq!(ConnectSpec::sqlite("/tmp/data/app.db").summary(), "app.db");
        let (spec, _) = ConnectSpec::from_url("mysql://h").unwrap();
        assert_eq!(spec.summary(), "h:3306");
    }

    #[test]
    fn bigquery_urls_name_a_project_and_a_key() {
        let (spec, secrets) = ConnectSpec::from_url("bigquery:my-project").unwrap();
        assert_eq!(spec, ConnectSpec::bigquery("my-project", None));
        assert_eq!(spec.host, "");
        assert_eq!(spec.port, 0);
        assert_eq!(secrets, Secrets::default());
        assert_eq!(spec.summary(), "my-project");

        let (spec, _) =
            ConnectSpec::from_url("bigquery://my-project?credentials=/keys/sa.json").unwrap();
        assert_eq!(spec.database, "my-project");
        assert_eq!(spec.bigquery_key, Some(PathBuf::from("/keys/sa.json")));

        assert!(matches!(
            ConnectSpec::from_url("bigquery:"),
            Err(Error::InvalidSpec(_))
        ));
        // No secret ever prints.
        let json = serde_json::to_string(&spec).unwrap();
        assert_eq!(serde_json::from_str::<ConnectSpec>(&json).unwrap(), spec);
    }

    #[test]
    fn secrets_do_not_print() {
        let secrets = Secrets {
            password: Some("hunter2".into()),
            ssh_password: Some("s3cret".into()),
            ssh_passphrase: None,
        };
        let printed = format!("{secrets:?}");
        assert!(!printed.contains("hunter2"));
        assert!(!printed.contains("s3cret"));
    }

    #[test]
    fn specs_round_trip_through_json_and_old_files_load() {
        let spec = ConnectSpec {
            ssh: Some(SshSpec {
                host: "bastion".into(),
                port: Some(22),
                user: "ops".into(),
                auth: SshAuth::KeyFile {
                    path: "/home/ops/.ssh/id_ed25519".into(),
                },
            }),
            ..ConnectSpec::from_url("postgres://u@h/db").unwrap().0
        };
        let json = serde_json::to_string(&spec).unwrap();
        assert_eq!(serde_json::from_str::<ConnectSpec>(&json).unwrap(), spec);
        let old: ConnectSpec =
            serde_json::from_str(r#"{"driver": "sqlite", "sqlite_path": "/a.db"}"#).unwrap();
        assert_eq!(old.driver, Driver::Sqlite);
        assert_eq!(old.tls, TlsMode::Prefer);
    }

    #[test]
    fn the_summary_names_the_ssh_host() {
        let (mut spec, _) = ConnectSpec::from_url("postgres://me@db/app").unwrap();
        spec.ssh = Some(SshSpec {
            host: "bastion".into(),
            port: Some(22),
            user: "ops".into(),
            auth: SshAuth::Agent,
        });
        assert_eq!(spec.summary(), "me@db:5432/app via ops@bastion");
    }

    #[test]
    fn an_ssh_port_left_to_the_config_is_left_out_of_the_file() {
        let old: SshSpec = serde_json::from_str(
            r#"{"host": "bastion", "port": 22, "user": "ops", "auth": {"method": "agent"}}"#,
        )
        .unwrap();
        assert_eq!(old.port, Some(22));
        let alias: SshSpec =
            serde_json::from_str(r#"{"host": "bastion", "user": "", "auth": {"method": "agent"}}"#)
                .unwrap();
        assert_eq!(alias.port, None);
        assert!(!serde_json::to_string(&alias).unwrap().contains("port"));
    }

    #[test]
    fn the_summary_leaves_out_an_ssh_user_left_to_the_config() {
        let (mut spec, _) = ConnectSpec::from_url("postgres://me@db/app").unwrap();
        spec.ssh = Some(SshSpec {
            host: "bastion".into(),
            port: None,
            user: String::new(),
            auth: SshAuth::Agent,
        });
        assert_eq!(spec.summary(), "me@db:5432/app via bastion");
    }
}
