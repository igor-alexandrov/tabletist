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
}

impl Driver {
    pub fn default_port(self) -> u16 {
        match self {
            Self::Postgres => 5432,
            Self::MySql => 3306,
            Self::Sqlite => 0,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Postgres => "PostgreSQL",
            Self::MySql => "MySQL",
            Self::Sqlite => "SQLite",
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
    KeyFile { path: PathBuf },
    Agent,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SshSpec {
    pub host: String,
    pub port: u16,
    pub user: String,
    pub auth: SshAuth,
}

/// Everything needed to connect except secrets. Saved in connections.json.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ConnectSpec {
    pub driver: Driver,
    pub host: String,
    pub port: u16,
    pub user: String,
    pub database: String,
    pub sqlite_path: Option<PathBuf>,
    pub tls: TlsMode,
    pub ca_file: Option<PathBuf>,
    pub ssh: Option<SshSpec>,
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
            tls: TlsMode::Prefer,
            ca_file: None,
            ssh: None,
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

impl ConnectSpec {
    pub fn sqlite(path: impl Into<PathBuf>) -> Self {
        Self {
            driver: Driver::Sqlite,
            host: String::new(),
            port: 0,
            sqlite_path: Some(path.into()),
            ..Self::default()
        }
    }

    /// Parses `postgres://`, `postgresql://`, `mysql://`, `mariadb://` and
    /// `sqlite:` URLs. The password, if any, comes back in `Secrets`.
    pub fn from_url(url: &str) -> Result<(Self, Secrets)> {
        let url = url.trim();
        if let Some(rest) = url.strip_prefix("sqlite:") {
            let path = rest.strip_prefix("//").unwrap_or(rest);
            if path.is_empty() {
                return Err(Error::InvalidSpec("the SQLite URL names no file".into()));
            }
            return Ok((Self::sqlite(decode(path)), Secrets::default()));
        }
        let parsed = url::Url::parse(url)
            .map_err(|error| Error::InvalidSpec(format!("not a connection URL: {error}")))?;
        let driver = match parsed.scheme() {
            "postgres" | "postgresql" => Driver::Postgres,
            "mysql" | "mariadb" => Driver::MySql,
            other => {
                return Err(Error::InvalidSpec(format!(
                    "unsupported scheme {other:?}; use postgres://, mysql:// or sqlite:"
                )));
            }
        };
        let host = parsed
            .host_str()
            .unwrap_or("localhost")
            .trim_start_matches('[')
            .trim_end_matches(']')
            .to_owned();
        let mut spec = Self {
            driver,
            host,
            port: parsed.port().unwrap_or(driver.default_port()),
            user: decode(parsed.username()),
            database: decode(parsed.path().trim_start_matches('/')),
            ..Self::default()
        };
        for (key, value) in parsed.query_pairs() {
            match key.as_ref() {
                "sslmode" | "ssl-mode" | "ssl_mode" => {
                    spec.tls = TlsMode::parse(&value)
                        .ok_or_else(|| Error::InvalidSpec(format!("unknown TLS mode {value:?}")))?;
                }
                "sslrootcert" | "ssl-ca" => spec.ca_file = Some(PathBuf::from(value.as_ref())),
                _ => log::debug!("ignoring URL parameter {key}"),
            }
        }
        let secrets = Secrets {
            password: parsed.password().map(decode),
            ..Secrets::default()
        };
        Ok((spec, secrets))
    }

    /// A one-line description without secrets: `user@host:port/db`, or the
    /// file name for SQLite.
    pub fn summary(&self) -> String {
        if self.driver == Driver::Sqlite {
            return self
                .sqlite_path
                .as_ref()
                .and_then(|path| path.file_name())
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_default();
        }
        let mut text = String::new();
        if !self.user.is_empty() {
            text.push_str(&self.user);
            text.push('@');
        }
        text.push_str(&format!("{}:{}", self.host, self.port));
        if !self.database.is_empty() {
            text.push('/');
            text.push_str(&self.database);
        }
        if let Some(ssh) = &self.ssh {
            text.push_str(&format!(" via {}@{}", ssh.user, ssh.host));
        }
        text
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn summaries_never_include_secrets() {
        let (spec, _) = ConnectSpec::from_url("postgres://me:secret@h:5432/db").unwrap();
        assert_eq!(spec.summary(), "me@h:5432/db");
        assert!(!spec.summary().contains("secret"));
        assert_eq!(ConnectSpec::sqlite("/tmp/data/app.db").summary(), "app.db");
        let (spec, _) = ConnectSpec::from_url("mysql://h").unwrap();
        assert_eq!(spec.summary(), "h:3306");
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
                port: 22,
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
            port: 22,
            user: "ops".into(),
            auth: SshAuth::Agent,
        });
        assert_eq!(spec.summary(), "me@db:5432/app via ops@bastion");
    }
}
