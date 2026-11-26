//! The saved connections a Files connection's session attaches: what the
//! app sends with a Connect or a Test, and their secrets, which only the
//! backend reads, from the keyring, as the session opens. No password
//! comes from the app, or goes into the spec, the log or History.

use tabletist_db::files::{Attach, Attaching};
use tabletist_db::{ConnectSpec, Error, Secrets};

use crate::connections::{ConnectionId, PasswordMode, SavedConnections};
use crate::model::SecretKind;
use crate::secrets::{Keyring, password_account, ssh_account};

/// A saved connection to attach: the name SQL calls it by, and the
/// saved connection as it is now, `None` when it was deleted.
#[derive(Debug, Clone, PartialEq)]
pub struct AttachTo {
    pub name: String,
    pub saved: Option<AttachSaved>,
}

/// What a saved connection to attach is: how it is reached, and where
/// its secrets are kept.
#[derive(Debug, Clone, PartialEq)]
pub struct AttachSaved {
    pub id: ConnectionId,
    pub spec: ConnectSpec,
    pub password: PasswordMode,
    pub ssh_secret: PasswordMode,
}

impl AttachTo {
    /// What `attach` names, among the saved `connections`.
    pub fn of(attach: &[Attach], connections: &SavedConnections) -> Vec<Self> {
        attach
            .iter()
            .map(|each| Self {
                name: each.name.clone(),
                saved: connections
                    .get(&ConnectionId(each.connection.clone()))
                    .map(|saved| AttachSaved {
                        id: saved.id.clone(),
                        spec: saved.spec.clone(),
                        password: saved.password,
                        ssh_secret: saved.ssh_secret,
                    }),
            })
            .collect()
    }
}

/// Each of `attach` with its secrets, read from `keyring` now: why one
/// cannot be attached, where one cannot.
pub(super) async fn attaching(
    attach: Vec<AttachTo>,
    keyring: &Keyring,
) -> Result<Vec<Attaching>, Error> {
    let mut each = Vec::with_capacity(attach.len());
    for to in attach {
        let name = to.name;
        let cannot = |why: &str| Error::Connect(format!("Can't attach {name}: {why}"));
        let Some(saved) = to.saved else {
            return Err(cannot("its saved connection was deleted"));
        };
        let mut secrets = Secrets::default();
        // A file is opened here, whatever tunnel its spec still names.
        let ssh = match saved.spec.driver {
            tabletist_db::Driver::Postgres | tabletist_db::Driver::MySql => {
                SecretKind::for_ssh(&saved.spec)
            }
            tabletist_db::Driver::Sqlite
            | tabletist_db::Driver::BigQuery
            | tabletist_db::Driver::Files => None,
        };
        let needed = [
            (
                Some(SecretKind::Database),
                saved.password,
                password_account(&saved.id),
            ),
            (
                ssh,
                match ssh {
                    Some(_) => saved.ssh_secret,
                    None => PasswordMode::None,
                },
                ssh_account(&saved.id),
            ),
        ];
        for (kind, mode, account) in needed {
            let Some(kind) = kind else {
                continue;
            };
            match mode {
                PasswordMode::None => {}
                PasswordMode::Ask => {
                    return Err(cannot("its password is asked for on every connect"));
                }
                PasswordMode::Keyring => match keyring.read(&account).wait().await {
                    Ok(Some(secret)) => *kind.slot(&mut secrets) = Some(secret),
                    Ok(None) => return Err(cannot("the keyring has no password for it")),
                    Err(error) => return Err(cannot(&error.to_string())),
                },
            }
        }
        each.push(Attaching {
            name,
            spec: saved.spec,
            secrets,
        });
    }
    Ok(each)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn to(name: &str, spec: ConnectSpec, password: PasswordMode) -> AttachTo {
        AttachTo {
            name: name.into(),
            saved: Some(AttachSaved {
                id: ConnectionId(format!("id-{name}")),
                spec,
                password,
                ssh_secret: PasswordMode::None,
            }),
        }
    }

    fn postgres() -> ConnectSpec {
        ConnectSpec {
            user: "reader".into(),
            database: "bookshop".into(),
            ..ConnectSpec::default()
        }
    }

    #[tokio::test]
    async fn a_password_is_read_from_the_keyring_as_it_is_attached() {
        let keyring = Keyring::memory();
        keyring
            .write(
                &password_account(&ConnectionId("id-shop".into())),
                crate::secrets::SecretString("s3cret".into()),
            )
            .wait()
            .await
            .unwrap();
        let each = attaching(
            vec![
                to("shop", postgres(), PasswordMode::Keyring),
                to(
                    "lite",
                    ConnectSpec::sqlite("/data/bookshop.db"),
                    PasswordMode::None,
                ),
            ],
            &keyring,
        )
        .await
        .unwrap();
        assert_eq!(each[0].name, "shop");
        assert_eq!(each[0].secrets.password.as_deref(), Some("s3cret"));
        assert_eq!(each[1].secrets, Secrets::default());
    }

    /// A file needs no SSH secret, whatever tunnel its spec still names.
    #[tokio::test]
    async fn a_file_reads_no_ssh_secret() {
        let mut spec = ConnectSpec::sqlite("/data/bookshop.db");
        spec.ssh = Some(tabletist_db::SshSpec {
            host: "bastion".into(),
            port: None,
            user: "reader".into(),
            auth: tabletist_db::SshAuth::Password,
        });
        let mut file = to("lite", spec, PasswordMode::None);
        if let Some(saved) = &mut file.saved {
            saved.ssh_secret = PasswordMode::Keyring;
        }
        let each = attaching(vec![file], &Keyring::memory()).await.unwrap();
        assert_eq!(each[0].secrets, Secrets::default());
    }

    #[tokio::test]
    async fn one_that_cannot_be_attached_says_why() {
        let keyring = Keyring::memory();
        let why = |attach: AttachTo| {
            let keyring = keyring.clone();
            async move {
                match attaching(vec![attach], &keyring).await {
                    Err(error) => error.to_string(),
                    Ok(_) => panic!("it was attached"),
                }
            }
        };
        assert!(
            why(to("shop", postgres(), PasswordMode::Keyring))
                .await
                .contains("Can't attach shop: the keyring has no password for it")
        );
        assert!(
            why(to("shop", postgres(), PasswordMode::Ask))
                .await
                .contains("Can't attach shop: its password is asked for on every connect")
        );
        assert!(
            why(AttachTo {
                name: "shop".into(),
                saved: None,
            })
            .await
            .contains("Can't attach shop: its saved connection was deleted")
        );
    }
}
