//! What the app last showed the person of what agents may do: whether the
//! server is on, and each connection's access. A change made outside the
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

use crate::connections::{AgentAccess, ConnectionId, SavedConnection};

/// What agents may do, as the person last saw it.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Shown {
    /// Whether the agent server was on.
    pub enabled: bool,
    /// Each connection's access, by its id. A connection not here was Off.
    pub access: BTreeMap<String, AgentAccess>,
}

/// A change that lets agents do more than the person was shown.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Raised {
    /// The agent server was turned on.
    Server,
    /// A connection's access went up to `to`.
    Access { id: ConnectionId, to: AgentAccess },
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
        }
        // A connection deleted since lets agents do nothing.
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
        let text = std::fs::read_to_string(&path).unwrap();
        assert!(text.contains("\"shop\": \"read\""), "{text}");
        std::fs::write(&path, "{ not json").unwrap();
        assert_eq!(Shown::load(&path), Shown::default());
    }
}
