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
    /// The URL without its password, safe to leave on screen.
    pub without_password: String,
}

impl ParsedUrl {
    /// Parses `postgres://`, `postgresql://`, `mysql://`, `mariadb://` and
    /// `sqlite:` URLs. A TLS setting named twice (under any of its
    /// spellings) is an error rather than the last one silently winning.
    pub fn parse(url: &str) -> Result<Self> {
        let url = url.trim();
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
                without_password: url.to_owned(),
            });
        }
        let mut parsed = url::Url::parse(url)
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
        let without_password = if secrets.password.is_some() {
            // Cannot fail: the URL has a host, since it has a password.
            let _ = parsed.set_password(None);
            parsed.to_string()
        } else {
            url.to_owned()
        };
        Ok(Self {
            spec,
            secrets,
            names_tls,
            names_ca_file,
            without_password,
        })
    }
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
        let parsed = ParsedUrl::parse(url)?;
        Ok((parsed.spec, parsed.secrets))
    }

    /// The TLS mode as it is enforced: PostgreSQL treats `require` with a CA
    /// file as `verify-ca`, like libpq (MySQL refuses that combination).
    pub fn effective_tls(&self) -> TlsMode {
        match (self.driver, self.tls, &self.ca_file) {
            (Driver::Postgres, TlsMode::Require, Some(_)) => TlsMode::VerifyCa,
            (_, tls, _) => tls,
        }
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
    fn the_url_left_on_screen_has_no_password() {
        let parsed = ParsedUrl::parse("postgres://me:s%40cret@db.example.com:6543/app").unwrap();
        assert_eq!(parsed.secrets.password.as_deref(), Some("s@cret"));
        assert_eq!(
            parsed.without_password,
            "postgres://me@db.example.com:6543/app"
        );
        let parsed = ParsedUrl::parse(" mysql://me@h/db ").unwrap();
        assert_eq!(parsed.without_password, "mysql://me@h/db");
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
