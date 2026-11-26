//! A Files connection's attached connections outside its dialog, in both
//! looks (the "Files, attached connections" boards): each one's schema
//! with its environment's badge in the sidebar, and what agents reach of
//! them, as the Agents part says and as the policy agents get holds it.

use tabletist_db::agent::Tables;
use tabletist_db::files::{Attach, Source};
use tabletist_db::{ConnectSpec, Driver, ObjectInfo, ObjectKind};

use crate::backend::{AgentsCommand, Command, Event};
use crate::connections::{AgentAccess, ConnectionId, PasswordMode, SavedConnection};
use crate::env::{Environment, Platform};
use crate::model::Action;
use crate::testing::{Harness, answer_tree, last_sent};
use crate::theme::Look;

fn looks() -> [Look; 2] {
    [Look::macos(), Look::omarchy()]
}

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

/// Opens the Files connection `id` in the active tab through the recording
/// backend, its tree answered: `main` and the three attached schemas.
fn opened(harness: &mut Harness, id: &ConnectionId) -> crate::model::ConnTabId {
    let tab = harness.app.active_tab_id();
    harness.app.apply(Action::Connect {
        tab,
        conn: id.clone(),
    });
    let Command::Connect {
        session, request, ..
    } = *last_sent(&harness.app)
    else {
        panic!("expected Connect");
    };
    harness.app.apply(Action::Backend(Event::Connected {
        session,
        request,
        driver: Driver::Files,
        encrypted: false,
        access: crate::testing::asked_access(&harness.app),
    }));
    let schemas = [
        "main",
        "bookshop_development",
        "bookshop_production",
        "bookshop_staging",
    ];
    let orders = ObjectInfo {
        name: "orders".into(),
        kind: ObjectKind::Table,
        estimated_rows: Some(40),
    };
    answer_tree(
        &mut harness.app,
        schemas.map(str::to_owned).to_vec(),
        vec![orders],
    );
    harness.settle();
    tab
}

/// Where the last frame painted `text`, in any case: macOS's section
/// labels are capitals.
fn painted(harness: &Harness, text: &str) -> Option<egui::Rect> {
    harness
        .text_rects
        .iter()
        .find(|(painted, _)| painted.eq_ignore_ascii_case(text))
        .map(|(_, rect)| *rect)
}

#[test]
fn an_attached_schema_shows_its_environment_s_badge() {
    for look in looks() {
        let name = look.name;
        let mut harness = Harness::with_size(egui::vec2(1200.0, 800.0));
        harness.set_look(look);
        let id = bookshop(&mut harness);
        let tab = opened(&mut harness, &id);
        let label = |environment: Environment| environment.label(Platform::of(&look));
        // The files' own schema has none.
        assert!(
            harness.painted_rect(label(Environment::Dev)).is_none(),
            "{name}"
        );
        harness.app.apply(Action::ShowSchema {
            tab,
            schema: "bookshop_development".into(),
        });
        harness.settle();
        let schema = painted(&harness, "bookshop_development")
            .unwrap_or_else(|| panic!("{name}: the schema is not drawn"));
        let badge = harness
            .painted_rect(label(Environment::Dev))
            .unwrap_or_else(|| panic!("{name}: no badge"));
        assert!(badge.left() > schema.right(), "{name}");
        assert!((badge.center().y - schema.center().y).abs() < 2.0, "{name}");
        // The menu lists the attached schemas after the files' own, each
        // with its badge.
        harness.click("Schema");
        harness.settle();
        let menu = |text: &str| {
            harness
                .painted_rect(text)
                .unwrap_or_else(|| panic!("{name}: {text} is not in the menu"))
        };
        let production = menu("bookshop_production");
        let badge = menu(label(Environment::Production));
        assert!(badge.left() > production.right(), "{name}");
        assert!(menu("main").top() < production.top(), "{name}");
        assert!(
            harness.painted_rect(label(Environment::Staging)).is_some(),
            "{name}"
        );
    }
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
