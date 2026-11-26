//! The saved connections a Files connection can attach, as its Join
//! section lists them, and the name SQL calls each by once it is.

use tabletist_db::Driver;
use tabletist_db::files::Attach;

use super::{ConnectionId, PasswordMode, SavedConnection};
use crate::env::Environment;

/// A saved connection the Join section lists: what its row says, and why
/// it cannot be attached where it cannot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Joinable {
    pub id: ConnectionId,
    pub name: String,
    pub environment: Environment,
    pub driver: Driver,
    /// Its database's name, or a SQLite file's: what the row says after
    /// the engine.
    pub database: String,
    /// It opens read-only: production, or its box ticked.
    pub read_only: bool,
    pub blocked: Option<Blocked>,
}

/// Why a saved connection's row cannot be ticked.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Blocked {
    /// Its password is asked for on every connect: no keyring holds it
    /// to attach it with.
    AsksPassword,
    /// A MySQL connection that names no database: DuckDB has no schema
    /// for `name.table` to read.
    NoDatabase,
}

/// The saved PostgreSQL, MySQL and SQLite connections among
/// `connections`, in the list's order: those a Files connection can
/// attach.
pub fn joinable(connections: &[SavedConnection]) -> Vec<Joinable> {
    connections
        .iter()
        .filter_map(|saved| {
            let spec = &saved.spec;
            let database = match spec.driver {
                // libpq's default database is the user's name.
                Driver::Postgres if spec.database.is_empty() => spec.user.clone(),
                Driver::Postgres | Driver::MySql => spec.database.clone(),
                Driver::Sqlite => spec
                    .sqlite_path
                    .as_deref()
                    .and_then(std::path::Path::file_name)
                    .map(|name| name.to_string_lossy().into_owned())
                    .unwrap_or_default(),
                Driver::BigQuery | Driver::Files => return None,
            };
            // An SSH secret counts only where there is a tunnel.
            let ssh_secret = match spec.ssh {
                Some(_) => saved.ssh_secret,
                None => PasswordMode::None,
            };
            let asks = [saved.password, ssh_secret]
                .into_iter()
                .any(|mode| mode == PasswordMode::Ask);
            let no_database = match spec.driver {
                Driver::MySql => spec.database.trim().is_empty(),
                Driver::Postgres | Driver::Sqlite | Driver::BigQuery | Driver::Files => false,
            };
            let blocked = match (asks, no_database) {
                (true, _) => Some(Blocked::AsksPassword),
                (false, true) => Some(Blocked::NoDatabase),
                (false, false) => None,
            };
            Some(Joinable {
                id: saved.id.clone(),
                name: saved.name.clone(),
                environment: saved.environment,
                driver: spec.driver,
                database,
                read_only: saved.read_only(),
                blocked,
            })
        })
        .collect()
}

/// The name SQL calls `saved` by when it is attached beside `attached`:
/// its database's ([`tabletist_db::files::attach_name`]), with `_2`, `_3`
/// and on after it where another attached connection, a schema of the
/// connection's own (`taken`) or one of DuckDB's has it.
pub fn default_name(saved: &SavedConnection, attached: &[Attach], taken: &[String]) -> String {
    let wanted = tabletist_db::files::attach_name(&saved.spec);
    let free = |name: &str| {
        !tabletist_db::files::reserved_name(name)
            && !attached.iter().any(|each| each.name == name)
            && !taken.iter().any(|each| each == name)
    };
    if free(&wanted) {
        return wanted;
    }
    (2..)
        .map(|count| format!("{wanted}_{count}"))
        .find(|name| free(name))
        .unwrap_or(wanted)
}

/// What is wrong with the names `attached` are called by, among the
/// schemas of the connection's own (`taken`): the first name that is not
/// a plain one, is DuckDB's own, is taken, or is given twice.
pub fn name_problem(attached: &[Attach], taken: &[String]) -> Option<String> {
    for (at, each) in attached.iter().enumerate() {
        let name = each.name.as_str();
        if name.is_empty() {
            return Some("Name each attached connection.".into());
        }
        if tabletist_db::files::table_name(name) != name {
            return Some(format!(
                "{name} can't name an attached connection: use lowercase letters, digits and _."
            ));
        }
        if tabletist_db::files::reserved_name(name) || taken.iter().any(|other| other == name) {
            return Some(format!(
                "{name} is taken: name the attached connection another way."
            ));
        }
        if attached[..at].iter().any(|other| other.name == name) {
            return Some(format!("Two attached connections are named {name}."));
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::connections::Agents;
    use tabletist_db::ConnectSpec;

    fn saved(name: &str, environment: Environment, spec: ConnectSpec) -> SavedConnection {
        SavedConnection {
            id: ConnectionId(format!("id-{name}")),
            name: name.into(),
            group: None,
            environment,
            read_only: None,
            password: PasswordMode::Keyring,
            ssh_secret: PasswordMode::None,
            spec,
            agents: Agents::default(),
        }
    }

    fn postgres(database: &str) -> ConnectSpec {
        ConnectSpec {
            driver: Driver::Postgres,
            user: "reader".into(),
            database: database.into(),
            ..ConnectSpec::default()
        }
    }

    #[test]
    fn only_databases_duckdb_reads_are_listed_with_why_one_cannot_be_ticked() {
        let mut asks = saved(
            "Bookshop",
            Environment::Staging,
            postgres("bookshop_staging"),
        );
        asks.password = PasswordMode::Ask;
        let mysql = ConnectSpec {
            driver: Driver::MySql,
            user: "reader".into(),
            ..ConnectSpec::default()
        };
        let connections = [
            saved(
                "Bookshop",
                Environment::Dev,
                postgres("bookshop_development"),
            ),
            asks,
            saved("Bookshop", Environment::Production, postgres("")),
            saved("Shop", Environment::Local, mysql),
            saved(
                "Bookshop",
                Environment::Local,
                ConnectSpec::sqlite("/data/development.sqlite3"),
            ),
            saved(
                "Exports",
                Environment::Local,
                ConnectSpec::files(tabletist_db::files::Source::Folder("/data".into())),
            ),
        ];
        let listed = joinable(&connections);
        let said: Vec<(&str, &str, bool, Option<Blocked>)> = listed
            .iter()
            .map(|each| {
                (
                    each.name.as_str(),
                    each.database.as_str(),
                    each.read_only,
                    each.blocked,
                )
            })
            .collect();
        assert_eq!(
            said,
            [
                ("Bookshop", "bookshop_development", false, None),
                (
                    "Bookshop",
                    "bookshop_staging",
                    false,
                    Some(Blocked::AsksPassword)
                ),
                ("Bookshop", "reader", true, None),
                ("Shop", "", false, Some(Blocked::NoDatabase)),
                ("Bookshop", "development.sqlite3", false, None),
            ]
        );
    }

    #[test]
    fn a_name_is_the_database_s_until_it_is_taken() {
        let dev = saved(
            "Bookshop",
            Environment::Dev,
            postgres("bookshop_development"),
        );
        assert_eq!(default_name(&dev, &[], &[]), "bookshop_development");
        let attached = [Attach {
            connection: "other".into(),
            name: "bookshop_development".into(),
        }];
        assert_eq!(
            default_name(&dev, &attached, &["bookshop_development_2".into()]),
            "bookshop_development_3"
        );
        let file = saved(
            "Bookshop",
            Environment::Local,
            ConnectSpec::sqlite("/data/Development-2.sqlite3"),
        );
        assert_eq!(default_name(&file, &[], &[]), "development_2");
        // One of DuckDB's own names is never given.
        let main = saved("Main", Environment::Local, postgres("main"));
        assert_eq!(default_name(&main, &[], &[]), "main_2");
    }

    #[test]
    fn a_name_must_be_plain_free_and_once() {
        let named = |names: &[&str]| -> Vec<Attach> {
            names
                .iter()
                .map(|name| Attach {
                    connection: format!("id-{name}"),
                    name: (*name).into(),
                })
                .collect()
        };
        assert_eq!(name_problem(&named(&["bookshop_dev", "shop"]), &[]), None);
        assert_eq!(
            name_problem(&named(&[""]), &[]).as_deref(),
            Some("Name each attached connection.")
        );
        assert_eq!(
            name_problem(&named(&["Bookshop-dev"]), &[]).as_deref(),
            Some(
                "Bookshop-dev can't name an attached connection: use lowercase letters, \
                 digits and _."
            )
        );
        assert_eq!(
            name_problem(&named(&["bookshop"]), &["bookshop".into()]).as_deref(),
            Some("bookshop is taken: name the attached connection another way.")
        );
        assert_eq!(
            name_problem(&named(&["memory"]), &[]).as_deref(),
            Some("memory is taken: name the attached connection another way.")
        );
        assert_eq!(
            name_problem(&named(&["shop", "shop"]), &[]).as_deref(),
            Some("Two attached connections are named shop.")
        );
    }
}
