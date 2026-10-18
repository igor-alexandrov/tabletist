//! What the app last showed the person of what agents may do: whether the
//! server is on, and each connection's access, the tables it hides and the
//! columns it masks. A change made outside the
//! app (a hand, a program, a synced dotfile, a backup) that lets agents do
//! more than that is held until the person keeps it or puts it back; one
//! that lets them do less is taken at once.
//!
//! Kept in the state directory (`AppDirs::agents_shown_file`). A file that
//! is not there, or cannot be read, shows nothing: whatever lets agents in
//! is held then, once.

use std::collections::BTreeMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

use tabletist_db::agent::{ColumnName, TableName, Tables};

use crate::connections::{AgentAccess, ConnectionId, SavedConnection};

/// What agents may do, as the person last saw it.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Shown {
    /// Whether the agent server was on.
    pub enabled: bool,
    /// Each connection's access, by its id. A connection not here was Off.
    pub access: BTreeMap<String, AgentAccess>,
    /// The tables each connection hides, by its id. A connection not here
    /// hid none.
    pub tables: BTreeMap<String, ShownTables>,
    /// The columns each connection masks, by its id, each entry as the
    /// person writes it. A connection not here masked none.
    pub masked: BTreeMap<String, Vec<String>>,
}

/// The columns a connection was shown to mask.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ShownMasked {
    /// The entries that read.
    pub columns: Vec<ColumnName>,
    /// Whether an entry did not read, which only a hand could write: what
    /// it masked cannot be said, so a change against it is held.
    pub unread: bool,
}

/// The tables a connection hid, each entry as the person writes it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ShownTables {
    Hidden(Vec<String>),
    Only(Vec<String>),
}

/// A change that lets agents do more than the person was shown.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Raised {
    /// The agent server was turned on.
    Server,
    /// A connection's access went up to `to`.
    Access { id: ConnectionId, to: AgentAccess },
    /// A connection hides less: `to`.
    Tables { id: ConnectionId, to: Tables },
    /// A connection masks less: `to`.
    Masked {
        id: ConnectionId,
        to: Vec<ColumnName>,
    },
}

impl Shown {
    pub fn load(path: &Path) -> Self {
        crate::util::load_json(path)
    }

    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        crate::util::save_json(path, self)
    }

    /// The access `id` was shown at.
    pub fn access_of(&self, id: &ConnectionId) -> AgentAccess {
        self.access.get(&id.0).copied().unwrap_or_default()
    }

    /// The tables `id` was shown to hide. An entry the file holds that
    /// does not read, which only a hand could write, cannot say what it
    /// hid: an "All except" list with one is taken for hiding every table,
    /// and an "Only" list leaves it out. Either way a change against it is
    /// held, and what is in force meanwhile hides no less than was shown.
    pub fn tables_of(&self, id: &ConnectionId) -> Tables {
        let read = |written: &[String]| -> Option<Vec<TableName>> {
            written.iter().map(|entry| entry.parse().ok()).collect()
        };
        match self.tables.get(&id.0) {
            None => Tables::All,
            Some(ShownTables::Hidden(written)) => match read(written) {
                Some(names) => normal(Tables::AllExcept(names)),
                None => Tables::Only(Vec::new()),
            },
            Some(ShownTables::Only(written)) => Tables::Only(
                written
                    .iter()
                    .filter_map(|entry| entry.parse().ok())
                    .collect(),
            ),
        }
    }

    /// The columns `id` was shown to mask.
    pub fn masked_of(&self, id: &ConnectionId) -> ShownMasked {
        let written = self
            .masked
            .get(&id.0)
            .map(Vec::as_slice)
            .unwrap_or_default();
        let columns: Vec<ColumnName> = written
            .iter()
            .filter_map(|entry| entry.parse().ok())
            .collect();
        ShownMasked {
            unread: columns.len() != written.len(),
            columns,
        }
    }

    /// What the files say now, `enabled` and `connections`, against what
    /// was shown: the changes that let agents do more, to be held. What
    /// lets them do less is taken into `self` at once; whether anything
    /// was is the second part, so that it is saved.
    pub fn compare(
        &mut self,
        enabled: bool,
        connections: &[SavedConnection],
    ) -> (Vec<Raised>, bool) {
        let mut raised = Vec::new();
        let mut lowered = false;
        match (self.enabled, enabled) {
            (false, true) => raised.push(Raised::Server),
            (true, false) => {
                self.enabled = false;
                lowered = true;
            }
            _ => {}
        }
        for saved in connections {
            let (was, now) = (self.access_of(&saved.id), saved.agents.access);
            if now > was {
                raised.push(Raised::Access {
                    id: saved.id.clone(),
                    to: now,
                });
            } else if now < was {
                self.set(&saved.id, now);
                lowered = true;
            }
            let (was, now) = (
                self.tables_of(&saved.id),
                normal(saved.agents.tables.clone()),
            );
            if now == was {
                continue;
            }
            match hides_no_less(&now, &was) {
                true => {
                    self.set_tables(&saved.id, &now);
                    lowered = true;
                }
                false => raised.push(Raised::Tables {
                    id: saved.id.clone(),
                    to: now,
                }),
            }
        }
        for saved in connections {
            let (was, now) = (self.masked_of(&saved.id), &saved.agents.masked);
            if !was.unread && same_columns(now, &was.columns) {
                continue;
            }
            match masks_no_less(now, &was) {
                true => {
                    self.set_masked(&saved.id, now);
                    lowered = true;
                }
                false => raised.push(Raised::Masked {
                    id: saved.id.clone(),
                    to: now.clone(),
                }),
            }
        }
        // A connection deleted since lets agents do nothing. What it hid is
        // kept: a start that read no connection at all (a file kept aside,
        // or one of a newer version) deleted nothing, and when it comes
        // back, a list that hides less is held.
        let before = self.access.len();
        self.access
            .retain(|id, _| connections.iter().any(|saved| saved.id.0 == *id));
        lowered |= self.access.len() != before;
        (raised, lowered)
    }

    /// Takes what was held as the person's: they kept it.
    pub fn keep(&mut self, raised: &[Raised]) {
        for raise in raised {
            match raise {
                Raised::Server => self.enabled = true,
                Raised::Access { id, to } => self.set(id, *to),
                Raised::Tables { id, to } => self.set_tables(id, to),
                Raised::Masked { id, to } => self.set_masked(id, to),
            }
        }
    }

    fn set_masked(&mut self, id: &ConnectionId, masked: &[ColumnName]) {
        match masked.is_empty() {
            true => {
                self.masked.remove(&id.0);
            }
            false => {
                let written = masked.iter().map(ToString::to_string).collect();
                self.masked.insert(id.0.clone(), written);
            }
        }
    }

    fn set_tables(&mut self, id: &ConnectionId, tables: &Tables) {
        let written = |names: &[TableName]| names.iter().map(ToString::to_string).collect();
        match normal(tables.clone()) {
            Tables::All => {
                self.tables.remove(&id.0);
            }
            Tables::AllExcept(names) => {
                self.tables
                    .insert(id.0.clone(), ShownTables::Hidden(written(&names)));
            }
            Tables::Only(names) => {
                self.tables
                    .insert(id.0.clone(), ShownTables::Only(written(&names)));
            }
        }
    }

    fn set(&mut self, id: &ConnectionId, access: AgentAccess) {
        match access {
            AgentAccess::Off => {
                self.access.remove(&id.0);
            }
            AgentAccess::Read => {
                self.access.insert(id.0.clone(), access);
            }
        }
    }
}

/// Whether `a` and `b` mask the same columns, in any order and case.
fn same_columns(a: &[ColumnName], b: &[ColumnName]) -> bool {
    let within = |a: &[ColumnName], b: &[ColumnName]| {
        a.iter()
            .all(|entry| b.iter().any(|other| same_column(entry, other)))
    };
    within(a, b) && within(b, a)
}

/// Whether two entries are one, without regard to case.
fn same_column(a: &ColumnName, b: &ColumnName) -> bool {
    let same = |a: &str, b: &str| a.to_lowercase() == b.to_lowercase();
    same(&a.table, &b.table)
        && same(&a.column, &b.column)
        && match (&a.schema, &b.schema) {
            (None, None) => true,
            (Some(a), Some(b)) => same(a, b),
            _ => false,
        }
}

/// Whether agents get masked under `now` every column they did under
/// `was`: each entry `was` had is covered by one `now` has. An entry
/// without a schema is that column in every schema, so it covers one with
/// a schema, and not the other way round. A shown entry that did not read
/// masked what nobody can say: nothing then masks no less.
fn masks_no_less(now: &[ColumnName], was: &ShownMasked) -> bool {
    let same = |a: &str, b: &str| a.to_lowercase() == b.to_lowercase();
    let covers = |wide: &ColumnName, narrow: &ColumnName| {
        same(&wide.table, &narrow.table)
            && same(&wide.column, &narrow.column)
            && match (&wide.schema, &narrow.schema) {
                (None, _) => true,
                (Some(wide), Some(narrow)) => same(wide, narrow),
                (Some(_), None) => false,
            }
    };
    !was.unread
        && was
            .columns
            .iter()
            .all(|masked| now.iter().any(|still| covers(still, masked)))
}

/// `tables`, an empty "All except" list as "All".
fn normal(tables: Tables) -> Tables {
    match tables {
        Tables::AllExcept(names) if names.is_empty() => Tables::All,
        tables => tables,
    }
}

/// Whether agents may read under `now` nothing they could not under `was`:
/// every table `was` hid is hidden still. An entry without a schema is that
/// name in every schema, so it covers one with a schema, and not the other
/// way round. Names are compared without regard to case.
fn hides_no_less(now: &Tables, was: &Tables) -> bool {
    let same = |a: &str, b: &str| a.to_lowercase() == b.to_lowercase();
    // Every table `narrow` names, `wide` names too.
    let covers = |wide: &TableName, narrow: &TableName| {
        same(&wide.name, &narrow.name)
            && match (&wide.schema, &narrow.schema) {
                (None, _) => true,
                (Some(wide), Some(narrow)) => same(wide, narrow),
                (Some(_), None) => false,
            }
    };
    match (was, now) {
        (Tables::All, _) => true,
        (_, Tables::All) => false,
        (Tables::AllExcept(was), Tables::AllExcept(now)) => was
            .iter()
            .all(|hidden| now.iter().any(|still| covers(still, hidden))),
        (Tables::Only(was), Tables::Only(now)) => now
            .iter()
            .all(|readable| was.iter().any(|could| covers(could, readable))),
        // What a list names is not all an "Only" list reads: a partition
        // or a shadow table goes with the table it is part of, which a
        // name cannot tell, and a renamed hidden table is readable by its
        // new name. Only a list that names nothing hides no less.
        (Tables::AllExcept(_), Tables::Only(now)) => now.is_empty(),
        // Every table that is not named was hidden, and is readable now.
        (Tables::Only(_), Tables::AllExcept(_)) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::connections::Agents;

    fn saved(id: &str, access: AgentAccess) -> SavedConnection {
        SavedConnection {
            id: ConnectionId(id.into()),
            name: id.into(),
            environment: crate::env::Environment::Dev,
            read_only: None,
            password: crate::connections::PasswordMode::None,
            ssh_secret: crate::connections::PasswordMode::None,
            spec: tabletist_db::ConnectSpec::sqlite("/tmp/shop.db"),
            agents: Agents {
                access,
                ..Agents::default()
            },
        }
    }

    #[test]
    fn what_lets_agents_do_more_is_held_and_what_lets_them_do_less_is_taken() {
        use AgentAccess::{Off, Read};
        let mut shown = Shown::default();
        // Nothing was shown: the server on and a connection Read are held.
        let (raised, lowered) = shown.compare(true, &[saved("shop", Read), saved("old", Off)]);
        assert_eq!(
            raised,
            [
                Raised::Server,
                Raised::Access {
                    id: ConnectionId("shop".into()),
                    to: Read
                }
            ]
        );
        assert!(!lowered);
        assert_eq!(shown, Shown::default(), "nothing held is taken");
        shown.keep(&raised);
        assert!(shown.enabled);
        assert_eq!(shown.access_of(&ConnectionId("shop".into())), Read);
        // As shown: nothing to hold, nothing to take.
        assert_eq!(
            shown.compare(true, &[saved("shop", Read)]),
            (Vec::new(), false)
        );
        // Less: taken at once.
        let (raised, lowered) = shown.compare(false, &[saved("shop", Off)]);
        assert_eq!(raised, []);
        assert!(lowered);
        assert_eq!(shown, Shown::default());
    }

    fn hiding(id: &str, tables: Tables) -> SavedConnection {
        let mut saved = saved(id, AgentAccess::Read);
        saved.agents.tables = tables;
        saved
    }

    fn names(written: &[&str]) -> Vec<TableName> {
        written.iter().map(|entry| entry.parse().unwrap()).collect()
    }

    #[test]
    fn a_change_that_hides_less_is_held_and_one_that_hides_more_is_taken() {
        let id = || ConnectionId("shop".into());
        let mut shown = Shown::default();
        shown.keep(&[Raised::Access {
            id: id(),
            to: AgentAccess::Read,
        }]);
        // Hiding a table where none was: taken.
        let customers = Tables::AllExcept(names(&["customers"]));
        let (raised, lowered) = shown.compare(false, &[hiding("shop", customers.clone())]);
        assert_eq!((raised, lowered), (Vec::new(), true));
        assert_eq!(shown.tables_of(&id()), customers);
        // Each way to read more is held, with what it would read.
        for more in [
            Tables::All,
            Tables::AllExcept(Vec::new()),
            Tables::AllExcept(names(&["orders"])),
            Tables::AllExcept(names(&["sales.customers"])),
            Tables::Only(names(&["customers"])),
            Tables::Only(names(&["books", "public.customers"])),
            // A partition of a hidden table goes with its parent, and a
            // hidden table renamed reads by its new name: a name cannot
            // tell either from a table that was readable.
            Tables::Only(names(&["books"])),
        ] {
            let mut copy = shown.clone();
            let (raised, lowered) = copy.compare(false, &[hiding("shop", more.clone())]);
            assert!(!lowered, "{more:?}");
            assert_eq!(raised.len(), 1, "{more:?}");
            assert!(
                matches!(&raised[0], Raised::Tables { id: held, .. } if *held == id()),
                "{more:?}"
            );
            assert_eq!(copy.tables_of(&id()), customers, "what was shown holds");
        }
        // Each way to read less, or the same in other words, is taken.
        for less in [
            Tables::AllExcept(names(&["customers", "orders"])),
            Tables::AllExcept(names(&["CUSTOMERS"])),
            Tables::Only(Vec::new()),
        ] {
            let mut copy = shown.clone();
            let (raised, _) = copy.compare(false, &[hiding("shop", less.clone())]);
            assert_eq!(raised, [], "{less:?}");
            assert_eq!(copy.tables_of(&id()), less, "{less:?}");
        }
        // From an "Only" list, adding a table is held; taking one is not.
        let mut only = Shown::default();
        only.keep(&[
            Raised::Access {
                id: id(),
                to: AgentAccess::Read,
            },
            Raised::Tables {
                id: id(),
                to: Tables::Only(names(&["books", "orders"])),
            },
        ]);
        let wider = Tables::Only(names(&["books", "orders", "customers"]));
        let (raised, _) = only.clone().compare(false, &[hiding("shop", wider)]);
        assert_eq!(raised.len(), 1);
        let narrower = Tables::Only(names(&["books"]));
        let (raised, _) = only.clone().compare(false, &[hiding("shop", narrower)]);
        assert_eq!(raised, []);
        let except = Tables::AllExcept(names(&["customers"]));
        let (raised, _) = only.compare(false, &[hiding("shop", except)]);
        assert_eq!(raised.len(), 1, "every table not named was hidden");
    }

    #[test]
    fn what_a_connection_hid_is_kept_while_it_is_not_there() {
        let id = || ConnectionId("shop".into());
        let mut shown = Shown::default();
        shown.keep(&[
            Raised::Access {
                id: id(),
                to: AgentAccess::Read,
            },
            Raised::Tables {
                id: id(),
                to: Tables::AllExcept(names(&["customers"])),
            },
        ]);
        // A start that read no connection, then the file again without
        // the list: held.
        shown.compare(false, &[]);
        let (raised, _) = shown.compare(false, &[hiding("shop", Tables::All)]);
        assert!(
            raised
                .iter()
                .any(|raise| matches!(raise, Raised::Tables { .. })),
            "{raised:?}"
        );
    }

    fn masking(id: &str, masked: &[&str]) -> SavedConnection {
        let mut saved = saved(id, AgentAccess::Read);
        saved.agents.masked = masked.iter().map(|entry| entry.parse().unwrap()).collect();
        saved
    }

    #[test]
    fn a_change_that_masks_less_is_held_and_one_that_masks_more_is_taken() {
        let id = || ConnectionId("shop".into());
        let mut shown = Shown::default();
        shown.keep(&[Raised::Access {
            id: id(),
            to: AgentAccess::Read,
        }]);
        // More, from the file: taken at once, and remembered.
        let (raised, lowered) = shown.compare(false, &[masking("shop", &["orders.email"])]);
        assert_eq!(raised, []);
        assert!(lowered);
        assert_eq!(shown.masked["shop"], ["orders.email"]);
        // The same in another case is no change.
        let (raised, lowered) = shown.compare(false, &[masking("shop", &["Orders.EMAIL"])]);
        assert_eq!((raised, lowered), (Vec::new(), false));
        // Another entry: more.
        let (raised, _) = shown.compare(false, &[masking("shop", &["orders.email", "phone.x"])]);
        assert_eq!(raised, []);
        // Less: held, with what the file says now.
        let (raised, lowered) = shown.compare(false, &[masking("shop", &["phone.x"])]);
        assert_eq!(
            raised,
            [Raised::Masked {
                id: id(),
                to: vec!["phone.x".parse().unwrap()],
            }]
        );
        assert!(!lowered);
        // So is a schema's entry in place of one for every schema.
        shown.keep(&[Raised::Masked {
            id: id(),
            to: vec!["orders.email".parse().unwrap()],
        }]);
        let (raised, _) = shown.compare(false, &[masking("shop", &["sales.orders.email"])]);
        assert_eq!(raised.len(), 1);
        // Kept: what was shown is the file's.
        shown.keep(&raised);
        assert_eq!(shown.masked["shop"], ["sales.orders.email"]);
        assert_eq!(shown.compare(false, &[masking("shop", &[])]).0.len(), 1);
    }

    #[test]
    fn a_shown_mask_that_does_not_read_holds_every_change() {
        let mut shown = Shown::default();
        shown.keep(&[Raised::Access {
            id: ConnectionId("shop".into()),
            to: AgentAccess::Read,
        }]);
        shown
            .masked
            .insert("shop".into(), vec!["orders.email".into(), "email".into()]);
        let was = shown.masked_of(&ConnectionId("shop".into()));
        assert_eq!(was.columns, vec!["orders.email".parse().unwrap()]);
        assert!(was.unread);
        // Even the readable entry again, with more.
        let (raised, _) =
            shown.compare(false, &[masking("shop", &["orders.email", "orders.phone"])]);
        assert_eq!(raised.len(), 1);
    }

    #[test]
    fn a_shown_list_that_does_not_read_hides_every_table() {
        let mut shown = Shown::default();
        shown.tables.insert(
            "shop".into(),
            ShownTables::Hidden(vec!["\"cust.omers".into()]),
        );
        let id = ConnectionId("shop".into());
        assert_eq!(shown.tables_of(&id), Tables::Only(Vec::new()));
        shown.keep(&[Raised::Access {
            id: id.clone(),
            to: AgentAccess::Read,
        }]);
        let (raised, _) = shown.compare(false, &[hiding("shop", Tables::All)]);
        assert_eq!(raised.len(), 1, "{raised:?}");
    }

    #[test]
    fn a_connection_deleted_since_is_forgotten() {
        let mut shown = Shown::default();
        shown.keep(&[Raised::Access {
            id: ConnectionId("gone".into()),
            to: AgentAccess::Read,
        }]);
        let (raised, lowered) = shown.compare(false, &[]);
        assert_eq!(raised, []);
        assert!(lowered);
        assert!(shown.access.is_empty());
        // Back under the same id, it is new, and held.
        let (raised, _) = shown.compare(false, &[saved("gone", AgentAccess::Read)]);
        assert_eq!(raised.len(), 1);
    }

    #[test]
    fn the_file_keeps_what_was_shown_and_a_bad_one_shows_nothing() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("agents_shown.json");
        assert_eq!(Shown::load(&path), Shown::default());
        let mut shown = Shown::default();
        shown.keep(&[
            Raised::Server,
            Raised::Access {
                id: ConnectionId("shop".into()),
                to: AgentAccess::Read,
            },
        ]);
        shown.save(&path).unwrap();
        assert_eq!(Shown::load(&path), shown);
        shown.keep(&[Raised::Tables {
            id: ConnectionId("shop".into()),
            to: Tables::AllExcept(names(&["customers", "\"odd.name\""])),
        }]);
        shown.save(&path).unwrap();
        assert_eq!(Shown::load(&path), shown);
        let text = std::fs::read_to_string(&path).unwrap();
        assert!(text.contains("\"shop\": \"read\""), "{text}");
        assert!(text.contains("\"hidden\""), "{text}");
        std::fs::write(&path, "{ not json").unwrap();
        assert_eq!(Shown::load(&path), Shown::default());
    }
}
