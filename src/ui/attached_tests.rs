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
use crate::model::{Action, Dialog};
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

/// Where the last frame painted the schema `text` in the sidebar's header:
/// in any case, as macOS's section labels are capitals, and cut where the
/// header has no room for all of it.
fn painted(harness: &Harness, text: &str) -> Option<egui::Rect> {
    harness
        .text_rects
        .iter()
        .find(|(painted, _)| {
            let painted = painted.to_lowercase();
            painted == text
                || painted
                    .strip_suffix('…')
                    .is_some_and(|cut| text.starts_with(cut))
        })
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

/// A long attached name is cut to keep its badge and chevron clear of
/// Reload and the tree/flat switch at the header's right.
#[test]
fn a_long_attached_name_keeps_clear_of_the_header_s_buttons() {
    for look in looks() {
        let name = look.name;
        let mut harness = Harness::with_size(egui::vec2(1200.0, 800.0));
        harness.set_look(look);
        let id = bookshop(&mut harness);
        let tab = opened(&mut harness, &id);
        let long = "bookshop_development_reports_archive_2026";
        let workspace = harness.app.workspace_mut(tab).unwrap();
        workspace.attached[0].name = long.into();
        if let Some(schemas) = &mut workspace.tree.schemas.value {
            schemas[1] = long.into();
        }
        harness.app.apply(Action::ShowSchema {
            tab,
            schema: long.into(),
        });
        // Its tables answered: Reload is drawn where a spinner was.
        let asked = match last_sent(&harness.app) {
            Command::ListObjects {
                session,
                request,
                schema,
            } => Some((*session, *request, schema.clone())),
            _ => None,
        };
        if let Some((session, request, schema)) = asked {
            harness.app.apply(Action::Backend(Event::Objects {
                session,
                request,
                schema,
                result: Ok(Vec::new()),
            }));
        }
        let tree = harness.settle();
        let cut =
            painted(&harness, long).unwrap_or_else(|| panic!("{name}: the name is not drawn"));
        assert!(
            harness.painted_rect(long).is_none(),
            "{name}: the name is not cut"
        );
        let badge = harness
            .painted_rect(Environment::Dev.label(Platform::of(&look)))
            .unwrap_or_else(|| panic!("{name}: no badge"));
        assert!(badge.left() > cut.right(), "{name}");
        let buttons = match look.terminal {
            true => harness.painted_rect("tree").unwrap().left(),
            false => crate::testing::bounds(&tree, "Reload objects", egui::accesskit::Role::Button)
                .unwrap()
                .left(),
        };
        assert!(badge.right() < buttons, "{name}: {badge:?} under {buttons}");
    }
}

/// One rule parts the own schemas from the attached ones: above the first
/// attached schema, one of no environment included, which has no badge,
/// and none between two attached schemas.
#[test]
fn one_rule_parts_the_own_schemas_from_the_attached_ones() {
    let schemas: Vec<String> = ["main", "sales_archive", "scratch", "bookshop_development"]
        .map(str::to_owned)
        .to_vec();
    let attached = [
        ("scratch".to_owned(), None),
        ("bookshop_development".to_owned(), Some(Environment::Dev)),
    ];
    let rows: Vec<(String, bool, Option<Environment>)> =
        crate::ui::sidebar::schema_choices(&schemas, Some("main"), &attached)
            .into_iter()
            .map(|choice| (choice.text, choice.rule_before, choice.badge))
            .collect();
    let row = |name: &str, rule, badge| (name.to_owned(), rule, badge);
    assert_eq!(
        rows,
        [
            row("main", false, None),
            row("sales_archive", false, None),
            row("scratch", true, None),
            row("bookshop_development", false, Some(Environment::Dev)),
        ]
    );
}

#[test]
fn the_agents_part_names_the_attachments_agents_don_t_reach() {
    for look in looks() {
        let name = look.name;
        let mut harness = Harness::with_size(egui::vec2(1200.0, 1600.0));
        harness.set_look(look);
        let id = bookshop(&mut harness);
        harness.app.apply(Action::EditConnection(id));
        if let Some(Dialog::Connection(form)) = &mut harness.app.dialog {
            form.agents_open = true;
        }
        harness.settle();
        let says = |harness: &mut Harness, text: &str| {
            let tree = harness.settle();
            let text = text.to_lowercase();
            crate::testing::labels(&tree)
                .iter()
                .any(|label| label.to_lowercase().contains(&text))
        };
        assert!(
            says(
                &mut harness,
                "Agents don't reach bookshop_production: its own agent access is Off."
            ),
            "{name}"
        );
        assert!(
            says(
                &mut harness,
                "Agents don't reach bookshop_staging: it hides tables or masks columns."
            ),
            "{name}"
        );
        assert!(!says(&mut harness, "reach bookshop_development"), "{name}");
        // With agents Off here, nothing of them is said.
        if let Some(Dialog::Connection(form)) = &mut harness.app.dialog {
            form.agents.access = AgentAccess::Off;
        }
        assert!(!says(&mut harness, "Agents don't reach"), "{name}");
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
