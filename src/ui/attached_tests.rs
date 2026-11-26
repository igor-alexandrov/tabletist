//! A Files connection's attached connections outside its dialog, in both
//! looks (the "Files, attached connections" boards): each one's schema
//! with its environment's badge in the sidebar, and what agents reach of
//! them, as the Agents part says and as the policy agents get holds it.

use tabletist_db::ConnectSpec;
use tabletist_db::agent::Tables;
use tabletist_db::files::{Attach, Source};

use crate::backend::{AgentsCommand, Command};
use crate::connections::{AgentAccess, ConnectionId, PasswordMode, SavedConnection};
use crate::env::Environment;
use crate::testing::Harness;

fn saved(name: &str, environment: Environment, spec: ConnectSpec) -> SavedConnection {
    SavedConnection {
        id: ConnectionId::new(),
        name: name.into(),
        group: None,
        environment,
        read_only: None,
        password: PasswordMode::None,
        ssh_secret: PasswordMode::None,
        spec,
        agents: crate::connections::Agents {
            access: AgentAccess::Read,
            ..Default::default()
        },
    }
}

/// The Bookshop's SQLite files saved three ways: development, which agents
/// may read; production, which they may not; staging, whose customers
/// they may not see. And a Files connection over its exports attaching
/// all three.
fn bookshop(harness: &mut Harness) -> ConnectionId {
    let sqlite = |file: &str| ConnectSpec::sqlite(format!("/data/{file}.sqlite3"));
    let dev = saved("Bookshop", Environment::Dev, sqlite("development"));
    let mut production = saved("Bookshop", Environment::Production, sqlite("production"));
    production.agents.access = AgentAccess::Off;
    let mut staging = saved("Bookshop", Environment::Staging, sqlite("staging"));
    staging.agents.tables = Tables::AllExcept(vec!["customers".parse().unwrap()]);
    let attach = |saved: &SavedConnection, name: &str| Attach {
        connection: saved.id.0.clone(),
        name: name.into(),
    };
    let mut spec = ConnectSpec::files(Source::Folder("/data/bookshop-exports".into()));
    spec.attach = vec![
        attach(&dev, "bookshop_development"),
        attach(&production, "bookshop_production"),
        attach(&staging, "bookshop_staging"),
    ];
    let files = saved("Bookshop exports", Environment::Local, spec);
    let id = files.id.clone();
    for saved in [dev, production, staging, files] {
        harness.app.connections.upsert(saved);
    }
    id
}

#[test]
fn agents_get_a_files_connection_with_what_its_attachments_let_them_read() {
    let mut harness = Harness::new();
    let id = bookshop(&mut harness);
    harness.app.serve_agents();
    let files = harness
        .app
        .backend
        .sent
        .iter()
        .rev()
        .find_map(|command| match command {
            Command::Agents(AgentsCommand::Policy { connections, .. }) => {
                connections.iter().find(|agentable| agentable.id == id)
            }
            _ => None,
        })
        .expect("the Files connection is agents'");
    let attached: Vec<(&str, Option<Environment>)> = files
        .attach
        .iter()
        .map(|to| {
            let environment = to.saved.as_ref().map(|saved| saved.environment);
            (to.name.as_str(), environment)
        })
        .collect();
    assert_eq!(attached, [("bookshop_development", Some(Environment::Dev))]);
}
