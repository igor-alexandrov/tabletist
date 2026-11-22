//! The groups of the Connections list. A connection the person put in a
//! group is under it. One with none falls under a name it shares with
//! another such connection, else under Local or Remote by where it
//! connects. Groups with the same title, in any case, are one.

use tabletist_db::Driver;

use super::SavedConnection;

/// The rule's group of connections that stay on this machine.
pub const LOCAL: &str = "Local";

/// The rule's group of the other connections.
pub const REMOTE: &str = "Remote";

/// Connections under one heading, in the file's order.
pub struct Group<'a> {
    /// As the first of them that names it writes it, or the rule's own.
    pub title: String,
    pub connections: Vec<&'a SavedConnection>,
}

/// The group the person gave `connection`, if any: what they wrote,
/// without the spaces at its ends.
pub fn own(connection: &SavedConnection) -> Option<&str> {
    connection
        .group
        .as_deref()
        .map(str::trim)
        .filter(|group| !group.is_empty())
}

/// The group a connection named `name` falls under while it has none of
/// its own: its name where another of `others` shares it and has no other
/// group (none, or that name), else Local where it stays on this machine
/// (`local`), else Remote.
pub fn automatic<'a>(
    name: &str,
    local: bool,
    others: impl IntoIterator<Item = &'a SavedConnection>,
) -> String {
    let name = name.trim();
    let shared = others
        .into_iter()
        .any(|other| other.name.trim() == name && own(other).is_none_or(|group| same(group, name)));
    match (shared, local) {
        (true, _) => name.to_owned(),
        (false, true) => LOCAL.to_owned(),
        (false, false) => REMOTE.to_owned(),
    }
}

/// The group `connection` is under among `connections`, which hold it.
pub fn of(connection: &SavedConnection, connections: &[&SavedConnection]) -> String {
    match own(connection) {
        Some(group) => group.to_owned(),
        None => automatic(
            &connection.name,
            stays_local(&connection.spec),
            connections
                .iter()
                .copied()
                .filter(|other| other.id != connection.id),
        ),
    }
}

/// `connections` in groups, in the list's order: by title, ignoring case,
/// with Local and then Remote last.
pub fn groups<'a>(connections: &[&'a SavedConnection]) -> Vec<Group<'a>> {
    let mut groups: Vec<Group<'a>> = Vec::new();
    for connection in connections {
        let title = of(connection, connections);
        match groups.iter_mut().find(|group| same(&group.title, &title)) {
            Some(group) => group.connections.push(connection),
            None => groups.push(Group {
                title,
                connections: vec![connection],
            }),
        }
    }
    groups.sort_by_key(|group| place(&group.title));
    groups
}

/// The groups a connection can be put in: those `connections` are in,
/// and `automatic`, the one it falls under without one of its own, in the
/// list's order.
pub fn choices(connections: &[&SavedConnection], automatic: &str) -> Vec<String> {
    let mut titles: Vec<String> = groups(connections)
        .into_iter()
        .map(|group| group.title)
        .collect();
    if !titles.iter().any(|title| same(title, automatic)) {
        titles.push(automatic.to_owned());
        titles.sort_by_key(|title| place(title));
    }
    titles
}

/// What the connection form's Group offers: the groups in the list's
/// order, and the one the connection falls under without its own.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Choices {
    pub titles: Vec<String>,
    pub automatic: String,
}

impl Choices {
    /// For a connection named `name` (`local` where it stays on this
    /// machine) among `connections`, the saved ones; `editing` is its id
    /// when it is one of them.
    pub fn of(
        name: &str,
        local: bool,
        editing: Option<&super::ConnectionId>,
        connections: &[SavedConnection],
    ) -> Self {
        let others = connections
            .iter()
            .filter(|other| Some(&other.id) != editing);
        let automatic = automatic(name.trim(), local, others);
        let all: Vec<&SavedConnection> = connections.iter().collect();
        Self {
            titles: choices(&all, &automatic),
            automatic,
        }
    }

    /// The title the Group shows for `chosen`, the form's own group: as
    /// the list writes it, else as chosen, else the rule's.
    pub fn shown<'a>(&'a self, chosen: Option<&'a str>) -> &'a str {
        let Some(chosen) = chosen.map(str::trim).filter(|chosen| !chosen.is_empty()) else {
            return &self.automatic;
        };
        self.titles
            .iter()
            .find(|title| same(title, chosen))
            .map_or(chosen, String::as_str)
    }
}

/// The group to save for `connection`, given the others saved: its own,
/// unless that is the group the rule gives it anyway.
pub fn keep(connection: &SavedConnection, connections: &[SavedConnection]) -> Option<String> {
    let others = connections.iter().filter(|other| other.id != connection.id);
    let automatic = automatic(
        connection.name.trim(),
        stays_local(&connection.spec),
        others,
    );
    kept(connection.group.as_deref(), &automatic)
}

/// The group to save for a connection given `chosen`, where `automatic`
/// is the one it falls under without: none for a blank one, or for the
/// group the rule gives it anyway, which then follows the rule.
pub fn kept(chosen: Option<&str>, automatic: &str) -> Option<String> {
    let chosen = chosen.map(str::trim).filter(|group| !group.is_empty())?;
    (!same(chosen, automatic)).then(|| chosen.to_owned())
}

/// Whether two titles name one group.
pub fn same(one: &str, other: &str) -> bool {
    one.trim().to_lowercase() == other.trim().to_lowercase()
}

/// Whether a connection of `driver` to `host` stays on this machine: a
/// file, or a local host without a tunnel (`tunnel`).
pub fn stays_on_this_machine(driver: Driver, host: &str, tunnel: bool) -> bool {
    match driver {
        Driver::Sqlite | Driver::Files => true,
        Driver::Postgres | Driver::MySql | Driver::Redis => {
            !tunnel && crate::model::is_local_host(host)
        }
        Driver::BigQuery => false,
    }
}

fn stays_local(spec: &tabletist_db::ConnectSpec) -> bool {
    stays_on_this_machine(spec.driver, &spec.host, spec.ssh.is_some())
}

/// Where a title sorts: the person's and the shared names by their words,
/// then Local, then Remote.
fn place(title: &str) -> (u8, String) {
    let rank = if same(title, LOCAL) {
        1
    } else if same(title, REMOTE) {
        2
    } else {
        0
    };
    (rank, title.trim().to_lowercase())
}

#[cfg(test)]
mod tests {
    use tabletist_db::{ConnectSpec, SshAuth, SshSpec};

    use super::*;
    use crate::connections::{ConnectionId, PasswordMode};
    use crate::env::Environment;

    fn saved(name: &str, group: Option<&str>, spec: ConnectSpec) -> SavedConnection {
        SavedConnection {
            id: ConnectionId::new(),
            name: name.into(),
            group: group.map(Into::into),
            environment: Environment::None,
            read_only: None,
            password: PasswordMode::None,
            ssh_secret: PasswordMode::None,
            spec,
            agents: Default::default(),
        }
    }

    fn server(host: &str) -> ConnectSpec {
        ConnectSpec {
            driver: Driver::Postgres,
            host: host.into(),
            port: 5432,
            ..ConnectSpec::default()
        }
    }

    fn file(path: &str) -> ConnectSpec {
        ConnectSpec::sqlite(path)
    }

    /// The headings and, under each, its connections' names.
    fn shown(connections: &[SavedConnection]) -> Vec<(String, Vec<String>)> {
        let all: Vec<&SavedConnection> = connections.iter().collect();
        groups(&all)
            .into_iter()
            .map(|group| {
                let names = group.connections.iter().map(|c| c.name.clone()).collect();
                (group.title, names)
            })
            .collect()
    }

    fn heading(title: &str, names: &[&str]) -> (String, Vec<String>) {
        (
            title.to_owned(),
            names.iter().map(|name| (*name).to_owned()).collect(),
        )
    }

    #[test]
    fn without_groups_a_shared_name_heads_one_and_the_rest_are_local_or_remote() {
        let tunnelled = ConnectSpec {
            ssh: Some(SshSpec {
                host: "bastion.example.com".into(),
                port: None,
                user: "deploy".into(),
                auth: SshAuth::Agent,
            }),
            ..server("localhost")
        };
        let connections = [
            saved("Rails test", None, server("localhost")),
            saved("Bookshop", None, server("localhost")),
            saved("Reports", None, server("db.example.com")),
            saved("Bookshop", None, server("db.example.com")),
            saved("Playground", None, file("/srv/playground.db")),
            saved("Tunnelled", None, tunnelled),
        ];
        assert_eq!(
            shown(&connections),
            [
                heading("Bookshop", &["Bookshop", "Bookshop"]),
                heading("Local", &["Rails test", "Playground"]),
                heading("Remote", &["Reports", "Tunnelled"]),
            ]
        );
    }

    #[test]
    fn a_group_the_person_gave_heads_its_own_among_the_shared_names() {
        let connections = [
            saved("Rails test", None, server("localhost")),
            saved(
                "Playground",
                Some("Experiments"),
                file("/srv/playground.db"),
            ),
            saved("Bookshop", None, server("localhost")),
            saved("Bookshop", None, server("db.example.com")),
            saved("Reports", Some("  Clients "), server("db.example.com")),
        ];
        assert_eq!(
            shown(&connections),
            [
                heading("Bookshop", &["Bookshop", "Bookshop"]),
                heading("Clients", &["Reports"]),
                heading("Experiments", &["Playground"]),
                heading("Local", &["Rails test"]),
            ]
        );
    }

    #[test]
    fn titles_in_another_case_are_one_group_as_the_first_writes_it() {
        let connections = [
            saved("Playground", Some("experiments"), file("/p.db")),
            saved("Scratch", Some("Experiments"), file("/s.db")),
            // The person's "local" is the rule's Local.
            saved("Reports", Some("local"), server("db.example.com")),
            saved("Rails test", None, server("localhost")),
        ];
        assert_eq!(
            shown(&connections),
            [
                heading("experiments", &["Playground", "Scratch"]),
                heading("local", &["Reports", "Rails test"]),
            ]
        );
    }

    #[test]
    fn a_connection_in_a_group_shares_its_name_with_none() {
        // Two Bookshops, one of them put with the clients: the other is
        // alone with its name, and goes by where it connects.
        let connections = [
            saved("Bookshop", Some("Clients"), server("db.example.com")),
            saved("Bookshop", None, server("localhost")),
        ];
        assert_eq!(
            shown(&connections),
            [
                heading("Clients", &["Bookshop"]),
                heading("Local", &["Bookshop"]),
            ]
        );
        // A blank group is none.
        let blank = [
            saved("Bookshop", Some("  "), server("db.example.com")),
            saved("Bookshop", None, server("localhost")),
        ];
        assert_eq!(
            shown(&blank),
            [heading("Bookshop", &["Bookshop", "Bookshop"])]
        );
    }

    #[test]
    fn a_namesake_put_in_a_group_of_its_name_still_shares_it() {
        // One Bookshop saved in "Bookshop" while the other was elsewhere;
        // that one then has no group: one heading holds both.
        let connections = [
            saved("Bookshop", None, server("localhost")),
            saved("Bookshop", Some("bookshop"), server("db.example.com")),
        ];
        assert_eq!(
            shown(&connections),
            [heading("Bookshop", &["Bookshop", "Bookshop"])]
        );
        // A name read with spaces round it is the same name.
        let spaced = [
            saved("Bookshop ", None, server("localhost")),
            saved("Bookshop", None, server("db.example.com")),
        ];
        assert_eq!(shown(&spaced).len(), 1, "{:?}", shown(&spaced));
    }

    #[test]
    fn the_choices_are_the_groups_shown_and_the_connections_own() {
        let connections = [
            saved("Bookshop", None, server("localhost")),
            saved("Bookshop", None, server("db.example.com")),
            saved("Playground", Some("Experiments"), file("/p.db")),
        ];
        let all: Vec<&SavedConnection> = connections.iter().collect();
        // A new connection to a server: Remote is offered, though no row
        // is there yet.
        let remote = automatic("Reports", false, all.iter().copied());
        assert_eq!(remote, "Remote");
        assert_eq!(
            choices(&all, &remote),
            ["Bookshop", "Experiments", "Remote"]
        );
        // One more Bookshop falls under the name it shares.
        assert_eq!(automatic("Bookshop", true, all.iter().copied()), "Bookshop");
        assert_eq!(choices(&all, "Bookshop"), ["Bookshop", "Experiments"]);
        // The Playground, edited: without its group it would be Local.
        let others = all.iter().copied().filter(|c| c.id != connections[2].id);
        assert_eq!(automatic("Playground", true, others), "Local");
        assert_eq!(choices(&all, "Local"), ["Bookshop", "Experiments", "Local"]);
    }

    #[test]
    fn the_form_offers_the_groups_and_shows_the_one_in_use() {
        let connections = [
            saved("Bookshop", None, server("localhost")),
            saved("Bookshop", None, server("db.example.com")),
            saved("Playground", Some("experiments"), file("/p.db")),
        ];
        // Editing the Playground: its own group as the list writes it, or
        // Local without one.
        let editing = Some(&connections[2].id);
        let offered = Choices::of("Playground", true, editing, &connections);
        assert_eq!(offered.titles, ["Bookshop", "experiments", "Local"]);
        assert_eq!(offered.automatic, "Local");
        assert_eq!(offered.shown(Some("Experiments")), "experiments");
        assert_eq!(offered.shown(None), "Local");
        assert_eq!(offered.shown(Some(" ")), "Local");
        // A group typed that no connection is in yet reads as typed.
        assert_eq!(offered.shown(Some("Clients")), "Clients");
    }

    #[test]
    fn a_connection_keeps_its_group_unless_the_rule_gives_it_anyway() {
        let mut connections = vec![
            saved("Bookshop", None, server("db.example.com")),
            saved("Bookshop", Some("Bookshop"), server("localhost")),
            saved("Playground", Some("Experiments"), file("/p.db")),
        ];
        // The second shares its name with the first: its group is the
        // rule's own.
        assert_eq!(keep(&connections[1], &connections), None);
        assert_eq!(
            keep(&connections[2], &connections),
            Some("Experiments".into())
        );
        connections[2].group = Some("LOCAL".into());
        assert_eq!(keep(&connections[2], &connections), None);
    }

    #[test]
    fn only_a_group_other_than_the_rules_own_is_kept() {
        assert_eq!(kept(Some("Clients"), "Local"), Some("Clients".into()));
        assert_eq!(kept(Some("  Clients  "), "Local"), Some("Clients".into()));
        assert_eq!(kept(Some("local"), "Local"), None);
        assert_eq!(kept(Some("Local"), "Remote"), Some("Local".into()));
        assert_eq!(kept(Some("   "), "Local"), None);
        assert_eq!(kept(None, "Local"), None);
    }

    #[test]
    fn a_file_or_a_local_host_without_a_tunnel_stays_on_this_machine() {
        assert!(stays_on_this_machine(Driver::Sqlite, "", false));
        assert!(stays_on_this_machine(Driver::Files, "", false));
        assert!(stays_on_this_machine(Driver::Postgres, "localhost", false));
        assert!(!stays_on_this_machine(Driver::Postgres, "localhost", true));
        assert!(!stays_on_this_machine(
            Driver::MySql,
            "db.example.com",
            false
        ));
        assert!(!stays_on_this_machine(Driver::BigQuery, "", false));
    }
}
