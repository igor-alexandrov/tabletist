//! The saved connections a Files connection's session attaches: DuckDB's
//! `postgres`, `mysql` and `sqlite` readers, each database attached
//! read-only under the name SQL calls it by.
//!
//! A PostgreSQL or MySQL database's login goes into a secret of the
//! session's own, in memory, which the `ATTACH` names and which is dropped
//! right after it: the attached database keeps the login it was given,
//! and no secret is left for another `ATTACH` or scan to find. The
//! `ATTACH`'s text holds no password: the connection string it keeps
//! (`duckdb_databases().path`) says only how TLS is used, where a tunnel
//! goes, and that PostgreSQL's transactions are read-only. A statement
//! that holds a password is never put in an error: DuckDB quotes the
//! statement a parser error is in.
//!
//! An attachment is `READ_ONLY`: DuckDB refuses to write to it, and its
//! readers run every statement sent through (`postgres_execute`,
//! `mysql_execute`) in a read-only transaction or on a read-only
//! connection, which the servers enforce. PostgreSQL's session also
//! starts with `default_transaction_read_only` on.

use crate::files::{Attaching, Reader};
use crate::{Dialect, Driver, Error, Result, TlsMode};

/// The statements that attach `attaching` as the session's `at`th
/// attachment, reached through a tunnel's local port `via` where it has
/// one, in order.
pub(super) struct Statements {
    /// Its login, for PostgreSQL and MySQL; none for a SQLite file.
    pub(super) secret: Option<String>,
    pub(super) attach: String,
    /// Drops the secret, once the database is attached.
    pub(super) forget: Option<String>,
    /// The SQLite file, which the session's fence lets DuckDB open.
    pub(super) file: Option<String>,
}

/// Prints whether there is a secret, never the statement: it holds the
/// password.
impl std::fmt::Debug for Statements {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Statements")
            .field("secret", &self.secret.is_some())
            .field("attach", &self.attach)
            .field("forget", &self.forget)
            .field("file", &self.file)
            .finish()
    }
}

/// The reader DuckDB attaches `attaching` with.
pub(super) fn reader(attaching: &Attaching) -> Result<Reader> {
    Reader::of(attaching.spec.driver).ok_or_else(|| {
        Error::Connect(format!(
            "Can't attach {}: only a PostgreSQL, MySQL or SQLite connection can be attached",
            attaching.name
        ))
    })
}

/// The statements for `attaching`; see [`Statements`].
pub(super) fn statements(at: usize, attaching: &Attaching, via: Option<u16>) -> Result<Statements> {
    let spec = &attaching.spec;
    let name = Dialect::DuckDb.ident(&attaching.name).into_owned();
    let secret = format!("tabletist_attach_{at}");
    let literal = |text: &str| Dialect::DuckDb.literal(text);
    let password = attaching.secrets.password.as_deref();
    match spec.driver {
        Driver::Postgres => {
            // libpq's default database is the user's name.
            let database = match spec.database.is_empty() {
                true => &spec.user,
                false => &spec.database,
            };
            let mut login = vec![
                format!("HOST {}", literal(&spec.host)),
                format!("USER {}", literal(&spec.user)),
                format!("DATABASE {}", literal(database)),
            ];
            // Through a tunnel the socket goes to its port on this machine
            // while TLS still checks the host, as the app's own sessions do.
            let mut path = vec![
                ("sslmode", pg_ssl_mode(spec.tls).to_owned()),
                ("application_name", "Tabletist".to_owned()),
                ("options", "-c default_transaction_read_only=on".to_owned()),
            ];
            match via {
                Some(port) => {
                    path.push(("hostaddr", "127.0.0.1".to_owned()));
                    path.push(("port", port.to_string()));
                }
                None => login.push(format!("PORT {}", spec.port)),
            }
            if let Some(ca) = &spec.ca_file {
                path.push(("sslrootcert", path_text(ca)?));
            }
            if let Some(password) = password {
                login.push(format!("PASSWORD {}", literal(password)));
            }
            Ok(Statements {
                secret: Some(format!(
                    "CREATE SECRET {secret} (TYPE postgres, {})",
                    login.join(", ")
                )),
                attach: format!(
                    "ATTACH {} AS {name} (TYPE postgres, SECRET {secret}, READ_ONLY)",
                    literal(&conninfo(&path, Quote::Single))
                ),
                forget: Some(format!("DROP SECRET {secret}")),
                file: None,
            })
        }
        Driver::MySql => {
            let (host, port) = match via {
                Some(port) => ("127.0.0.1", port),
                None => (spec.host.as_str(), spec.port),
            };
            let mut login = vec![
                format!("HOST {}", literal(host)),
                format!("PORT {port}"),
                format!("USER {}", literal(&spec.user)),
                format!("DATABASE {}", literal(&spec.database)),
            ];
            if let Some(password) = password {
                login.push(format!("PASSWORD {}", literal(password)));
            }
            let mut path = vec![("ssl_mode", mysql_ssl_mode(spec.tls).to_owned())];
            if let Some(ca) = &spec.ca_file {
                path.push(("ssl_ca", path_text(ca)?));
            }
            Ok(Statements {
                secret: Some(format!(
                    "CREATE SECRET {secret} (TYPE mysql, {})",
                    login.join(", ")
                )),
                attach: format!(
                    "ATTACH {} AS {name} (TYPE mysql, SECRET {secret}, READ_ONLY)",
                    literal(&conninfo(&path, Quote::Double))
                ),
                forget: Some(format!("DROP SECRET {secret}")),
                file: None,
            })
        }
        Driver::Sqlite => {
            let path = spec.sqlite_path.as_deref().ok_or_else(|| {
                Error::Connect(format!("Can't attach {}: it names no file", attaching.name))
            })?;
            // DuckDB attaches a file that is not there as an empty one.
            if !path.is_file() {
                return Err(Error::Connect(format!(
                    "Can't attach {}: no such file {}",
                    attaching.name,
                    crate::files::shown(path)
                )));
            }
            let path = path_text(path)?;
            Ok(Statements {
                secret: None,
                attach: format!(
                    "ATTACH {} AS {name} (TYPE sqlite, READ_ONLY)",
                    literal(&path)
                ),
                forget: None,
                file: Some(path),
            })
        }
        Driver::BigQuery | Driver::Files => Err(Error::Connect(format!(
            "Can't attach {}: only a PostgreSQL, MySQL or SQLite connection can be attached",
            attaching.name
        ))),
    }
}

/// libpq's word for `mode`.
fn pg_ssl_mode(mode: TlsMode) -> &'static str {
    match mode {
        TlsMode::Disable => "disable",
        TlsMode::Prefer => "prefer",
        TlsMode::Require => "require",
        TlsMode::VerifyCa => "verify-ca",
        TlsMode::VerifyFull => "verify-full",
    }
}

/// The MySQL client's word for `mode`.
fn mysql_ssl_mode(mode: TlsMode) -> &'static str {
    match mode {
        TlsMode::Disable => "disabled",
        TlsMode::Prefer => "preferred",
        TlsMode::Require => "required",
        TlsMode::VerifyCa => "verify_ca",
        TlsMode::VerifyFull => "verify_identity",
    }
}

/// How a connection string quotes a value that is not plain: libpq's in
/// single quotes, MySQL's in double. Either puts a backslash before the
/// quote or a backslash in it.
#[derive(Clone, Copy)]
enum Quote {
    Single,
    Double,
}

/// `key=value` pairs as a connection string has them, a space between: a
/// plain value as it is, any other in `quote`s, a quote or a backslash in it
/// after a backslash.
fn conninfo(pairs: &[(&str, String)], quote: Quote) -> String {
    let plain = |value: &str| {
        !value.is_empty()
            && value
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.' | '/' | ':'))
    };
    pairs
        .iter()
        .map(|(key, value)| match plain(value) {
            true => format!("{key}={value}"),
            false => {
                let mark = match quote {
                    Quote::Single => '\'',
                    Quote::Double => '"',
                };
                let value = value
                    .replace('\\', "\\\\")
                    .replace(mark, &format!("\\{mark}"));
                format!("{key}={mark}{value}{mark}")
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn path_text(path: &std::path::Path) -> Result<String> {
    path.to_str().map(str::to_owned).ok_or_else(|| {
        Error::Connect(format!(
            "Can't attach {}: the path is not valid UTF-8",
            path.display()
        ))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ConnectSpec, Secrets};

    fn attaching(spec: ConnectSpec, password: Option<&str>) -> Attaching {
        Attaching {
            name: "bookshop_dev".into(),
            spec,
            secrets: Secrets {
                password: password.map(str::to_owned),
                ..Secrets::default()
            },
        }
    }

    fn postgres() -> ConnectSpec {
        ConnectSpec {
            driver: Driver::Postgres,
            host: "db.example.com".into(),
            port: 5433,
            user: "reader".into(),
            database: "bookshop_development".into(),
            tls: TlsMode::VerifyFull,
            ca_file: Some("/etc/bookshop/ca.pem".into()),
            ..ConnectSpec::default()
        }
    }

    /// The login and the password go into the secret; the ATTACH, whose
    /// text DuckDB keeps and shows, holds neither.
    #[test]
    fn a_password_goes_into_the_secret_alone() {
        let made = statements(0, &attaching(postgres(), Some("it's s3cret")), None).unwrap();
        let secret = made.secret.clone().unwrap();
        assert_eq!(
            secret,
            "CREATE SECRET tabletist_attach_0 (TYPE postgres, HOST 'db.example.com', \
             USER 'reader', DATABASE 'bookshop_development', PORT 5433, \
             PASSWORD 'it''s s3cret')"
        );
        assert_eq!(
            made.attach,
            "ATTACH 'sslmode=verify-full application_name=Tabletist \
             options=''-c default_transaction_read_only=on'' \
             sslrootcert=/etc/bookshop/ca.pem' AS bookshop_dev \
             (TYPE postgres, SECRET tabletist_attach_0, READ_ONLY)"
        );
        assert!(!made.attach.contains("s3cret"));
        let printed = format!("{made:?}");
        assert!(!printed.contains("s3cret"), "{printed}");
        assert_eq!(
            made.forget.as_deref(),
            Some("DROP SECRET tabletist_attach_0")
        );
        assert_eq!(made.file, None);
    }

    /// Through a tunnel PostgreSQL goes to its port on this machine and
    /// still checks the host; MySQL goes there by host and port.
    #[test]
    fn a_tunnel_takes_the_connection_to_its_port() {
        let made = statements(1, &attaching(postgres(), None), Some(40112)).unwrap();
        let secret = made.secret.unwrap();
        assert!(secret.contains("HOST 'db.example.com'"), "{secret}");
        assert!(!secret.contains("PORT"), "{secret}");
        assert!(!secret.contains("PASSWORD"), "{secret}");
        assert!(
            made.attach.contains("hostaddr=127.0.0.1 port=40112"),
            "{}",
            made.attach
        );
        let mysql = ConnectSpec {
            driver: Driver::MySql,
            host: "db.example.com".into(),
            port: 3306,
            user: "reader".into(),
            database: "bookshop".into(),
            tls: TlsMode::Disable,
            ..ConnectSpec::default()
        };
        let made = statements(2, &attaching(mysql, Some("pw")), Some(40113)).unwrap();
        assert_eq!(
            made.secret.unwrap(),
            "CREATE SECRET tabletist_attach_2 (TYPE mysql, HOST '127.0.0.1', PORT 40113, \
             USER 'reader', DATABASE 'bookshop', PASSWORD 'pw')"
        );
        assert_eq!(
            made.attach,
            "ATTACH 'ssl_mode=disabled' AS bookshop_dev \
             (TYPE mysql, SECRET tabletist_attach_2, READ_ONLY)"
        );
    }

    /// A SQLite file is attached by its path, which the fence lets in;
    /// one that is not there is not.
    #[test]
    fn a_sqlite_file_is_attached_by_its_path() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("o'neil.sqlite3");
        assert!(
            statements(0, &attaching(ConnectSpec::sqlite(&file), None), None)
                .unwrap_err()
                .to_string()
                .contains("Can't attach bookshop_dev: no such file")
        );
        std::fs::write(&file, "").unwrap();
        let path = file.to_str().unwrap().to_owned();
        let made = statements(0, &attaching(ConnectSpec::sqlite(&file), None), None).unwrap();
        assert_eq!(made.secret, None);
        assert_eq!(
            made.attach,
            format!(
                "ATTACH '{}' AS bookshop_dev (TYPE sqlite, READ_ONLY)",
                path.replace('\'', "''")
            )
        );
        assert_eq!(made.file.as_deref(), Some(path.as_str()));
    }

    /// A value with a space, a quote or a backslash stays one value.
    #[test]
    fn a_connection_string_value_is_quoted_where_it_must_be() {
        let pairs = [
            ("sslmode", "verify-full".to_owned()),
            ("sslrootcert", r#"C:\My certs\o'ne"il.pem"#.to_owned()),
        ];
        assert_eq!(
            conninfo(&pairs, Quote::Single),
            r#"sslmode=verify-full sslrootcert='C:\\My certs\\o\'ne"il.pem'"#
        );
        // MySQL's: in double quotes, as its reader takes them.
        assert_eq!(
            conninfo(&pairs, Quote::Double),
            r#"sslmode=verify-full sslrootcert="C:\\My certs\\o'ne\"il.pem""#
        );
    }
}
