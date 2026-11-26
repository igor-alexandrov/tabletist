//! A Files connection's Join with saved connections section, in both
//! looks (the "Connection, Files, details" boards): the saved
//! PostgreSQL, MySQL and SQLite connections it lists, a row ticked under
//! its database's name, a name typed over and checked on Save, a row that
//! cannot be ticked, the Omarchy keys, and what a Test and an open send:
//! the saved connections to attach, never a password.

use egui::{Key, Modifiers};
use tabletist_db::files::{Attach, Reader, Source};
use tabletist_db::{ConnectSpec, Driver, Error};

use crate::backend::attach::{AttachSaved, AttachTo};
use crate::backend::{Command, Event};
use crate::connections::{ConnectionId, PasswordMode, SavedConnection};
use crate::env::Environment;
use crate::model::{Action, ConnectionForm, Dialog, TestState};
use crate::testing::Harness;
use crate::theme::Look;

fn looks() -> [Look; 2] {
    [Look::macos(), Look::omarchy()]
}

fn form(harness: &Harness) -> &ConnectionForm {
    match &harness.app.dialog {
        Some(Dialog::Connection(form)) => form,
        _ => panic!("no connection dialog"),
    }
}

fn form_mut(harness: &mut Harness) -> &mut ConnectionForm {
    match &mut harness.app.dialog {
        Some(Dialog::Connection(form)) => form,
        _ => panic!("no connection dialog"),
    }
}

/// Whether a node says `text`, alone or as part of what it says, in any
/// case: the terminal look says its words in lower case.
fn says(harness: &mut Harness, text: &str) -> bool {
    let tree = harness.settle();
    let text = text.to_lowercase();
    crate::testing::labels(&tree)
        .iter()
        .any(|label| label.to_lowercase().contains(&text))
}

/// The Bookshop's saved databases: development, staging (whose password
/// is asked for on every connect) and production on PostgreSQL, and the
/// SQLite file; and a Files connection, which is not listed.
fn bookshop(harness: &mut Harness) -> [ConnectionId; 4] {
    let mut save = |name: &str, environment, password, spec: ConnectSpec| {
        let saved = SavedConnection {
            id: ConnectionId::new(),
            name: name.into(),
            group: None,
            environment,
            read_only: None,
            password,
            ssh_secret: PasswordMode::None,
            spec,
            agents: Default::default(),
        };
        let id = saved.id.clone();
        harness.app.connections.upsert(saved);
        id
    };
    let postgres = |database: &str| ConnectSpec {
        driver: Driver::Postgres,
        host: "db.bookshop.test".into(),
        user: "reader".into(),
        database: database.into(),
        ..ConnectSpec::default()
    };
    let dev = save(
        "Bookshop",
        Environment::Dev,
        PasswordMode::Keyring,
        postgres("bookshop_development"),
    );
    let staging = save(
        "Bookshop",
        Environment::Staging,
        PasswordMode::Ask,
        postgres("bookshop_staging"),
    );
    let production = save(
        "Bookshop",
        Environment::Production,
        PasswordMode::Keyring,
        postgres("bookshop_production"),
    );
    let file = save(
        "Bookshop",
        Environment::Local,
        PasswordMode::None,
        ConnectSpec::sqlite("/data/bookshop/development.sqlite3"),
    );
    save(
        "Bookshop exports",
        Environment::Local,
        PasswordMode::None,
        ConnectSpec::files(Source::Folder("/data/bookshop-exports".into())),
    );
    [dev, staging, production, file]
}

/// A new Files connection's dialog in `look`, over the Bookshop's
/// exports, the Bookshop's databases saved, its Join section open.
fn joining(look: Look) -> (Harness, [ConnectionId; 4]) {
    let mut harness = Harness::with_size(egui::vec2(1200.0, 1400.0));
    harness.set_look(look);
    let ids = bookshop(&mut harness);
    harness.app.apply(Action::NewConnection);
    harness.settle();
    harness.click(&look.label("Files"));
    form_mut(&mut harness).files_folder = "/data/bookshop-exports".into();
    form_mut(&mut harness).join_open = true;
    harness.settle();
    (harness, ids)
}

fn attached(harness: &Harness) -> Vec<(String, String)> {
    form(harness)
        .files_attach
        .iter()
        .map(|each| (each.connection.clone(), each.name.clone()))
        .collect()
}

#[test]
fn the_section_is_closed_at_first_and_lists_the_saved_databases() {
    for look in looks() {
        let mut harness = Harness::with_size(egui::vec2(1200.0, 1400.0));
        harness.set_look(look);
        bookshop(&mut harness);
        harness.app.apply(Action::NewConnection);
        harness.settle();
        harness.click(&look.label("Files"));
        assert!(!form(&harness).join_open, "{}", look.name);
        assert!(
            says(&mut harness, "Join with saved connections, None attached"),
            "{}",
            look.name
        );
        assert!(!harness.has("Attach Bookshop dev"));
        form_mut(&mut harness).join_open = true;
        for row in [
            "Attach Bookshop dev",
            "Attach Bookshop staging",
            "Attach Bookshop production",
            "Attach Bookshop local",
        ] {
            assert!(harness.has(row), "{}: {row}", look.name);
        }
        // A Files connection is not one to attach.
        assert!(
            !harness.has("Attach Bookshop exports local"),
            "{}",
            look.name
        );
        assert!(
            says(
                &mut harness,
                &look.label("Asks for its password on every connect")
            ),
            "{}",
            look.name
        );
        assert!(
            says(
                &mut harness,
                &look.label("Attached connections open read-only here, as the files do")
            ),
            "{}",
            look.name
        );
    }
}

#[test]
fn a_row_ticked_is_attached_under_its_database_s_name() {
    for look in looks() {
        let (mut harness, [dev, _, production, file]) = joining(look);
        harness.click("Attach Bookshop dev");
        harness.click("Attach Bookshop local");
        harness.click("Attach Bookshop production");
        assert_eq!(
            attached(&harness),
            [
                (dev.0.clone(), "bookshop_development".to_owned()),
                (file.0.clone(), "development".to_owned()),
                (production.0.clone(), "bookshop_production".to_owned()),
            ],
            "{}",
            look.name
        );
        // Closed, the section says them.
        form_mut(&mut harness).join_open = false;
        assert!(
            says(
                &mut harness,
                "Attached as bookshop_development, development, bookshop_production"
            ),
            "{}",
            look.name
        );
        form_mut(&mut harness).join_open = true;
        // The sample query, for the first, and the hint above the rows.
        assert!(
            says(
                &mut harness,
                "JOIN bookshop_development.publishers p ON p.id = r.publisher_id"
            ),
            "{}",
            look.name
        );
        assert!(
            says(&mut harness, "join a file to its live tables in one query"),
            "{}",
            look.name
        );
        // Ticked again, it is let go.
        harness.click("Attach Bookshop local");
        assert_eq!(attached(&harness).len(), 2, "{}", look.name);
    }
}

#[test]
fn a_row_that_asks_for_its_password_cannot_be_ticked() {
    for look in looks() {
        let (mut harness, [_, staging, _, _]) = joining(look);
        harness.click("Attach Bookshop staging");
        assert!(attached(&harness).is_empty(), "{}", look.name);
        // Nor by its action, from elsewhere.
        harness.app.apply(Action::ToggleAttach(staging));
        assert!(attached(&harness).is_empty(), "{}", look.name);
    }
}

#[test]
fn a_row_ticked_before_it_could_not_be_still_unticks() {
    for look in looks() {
        let (mut harness, [dev, ..]) = joining(look);
        harness.click("Attach Bookshop dev");
        // Its password is asked for on every connect from now on.
        let mut saved = harness.app.connections.get(&dev).unwrap().clone();
        saved.password = PasswordMode::Ask;
        harness.app.connections.upsert(saved);
        harness.settle();
        assert!(
            says(&mut harness, "Asks for its password on every connect"),
            "{}",
            look.name
        );
        harness.click("Attach Bookshop dev");
        assert!(attached(&harness).is_empty(), "{}", look.name);
    }
}

#[test]
fn a_test_s_download_goes_with_it() {
    let (mut harness, _) = joining(Look::macos());
    harness.click("Attach Bookshop dev");
    harness.app.apply(Action::TestConnection);
    let TestState::Running(request) = form(&harness).test else {
        panic!("no test");
    };
    harness.app.apply(Action::Backend(Event::FilesDownloading {
        request,
        reader: Reader::Postgres,
    }));
    // A row ticked meanwhile: that Test, and its download, say nothing
    // of what is attached now.
    harness.click("Attach Bookshop production");
    assert_eq!(form(&harness).files_downloading, None);
    assert!(!says(&mut harness, "Downloading the PostgreSQL reader"));
    harness.app.apply(Action::TestConnection);
    assert!(says(&mut harness, "Testing…"));
    // A name typed over forgets a Test that passed.
    form_mut(&mut harness).test = TestState::Passed;
    let field = crate::ui::attach_list::name_id(0);
    harness.ctx.memory_mut(|memory| memory.request_focus(field));
    harness.settle();
    harness.frame(vec![
        crate::testing::key(Key::A, Modifiers::COMMAND),
        egui::Event::Text("dev".into()),
    ]);
    harness.settle();
    assert_eq!(form(&harness).files_attach[0].name, "dev");
    assert_eq!(form(&harness).test, TestState::Idle);
}

#[test]
fn a_name_is_typed_over_and_checked_on_save() {
    let (mut harness, [dev, ..]) = joining(Look::macos());
    harness.click("Attach Bookshop dev");
    form_mut(&mut harness).name = "Bookshop joined".into();
    form_mut(&mut harness).files_attach[0].name = "Bookshop-dev".into();
    harness.app.apply(Action::SaveConnection { connect: false });
    assert_eq!(
        form(&harness).message.as_deref(),
        Some(
            "Bookshop-dev can't name an attached connection: use lowercase letters, digits and _."
        )
    );
    form_mut(&mut harness).files_attach[0].name = "bookshop_dev".into();
    harness.app.apply(Action::SaveConnection { connect: false });
    assert!(
        harness.app.dialog.is_none(),
        "{:?}",
        harness.app.dialog.is_some()
    );
    let saved = harness
        .app
        .connections
        .connections
        .iter()
        .find(|saved| saved.name == "Bookshop joined")
        .cloned()
        .unwrap();
    assert_eq!(
        saved.spec.attach,
        [Attach {
            connection: dev.0.clone(),
            name: "bookshop_dev".into(),
        }]
    );
}

#[test]
fn a_test_sends_what_is_attached_and_says_the_download() {
    for look in looks() {
        let (mut harness, [dev, ..]) = joining(look);
        harness.click("Attach Bookshop dev");
        harness.app.apply(Action::TestConnection);
        let TestState::Running(request) = form(&harness).test else {
            panic!("no test: {:?}", form(&harness).test);
        };
        let saved = harness.app.connections.get(&dev).unwrap().clone();
        let Some(Command::Test {
            attach, secrets, ..
        }) = harness.app.backend.sent.last()
        else {
            panic!("no Test sent");
        };
        assert_eq!(
            attach,
            &[AttachTo {
                name: "bookshop_development".into(),
                saved: Some(AttachSaved {
                    id: dev.clone(),
                    spec: saved.spec.clone(),
                    password: PasswordMode::Keyring,
                    ssh_secret: PasswordMode::None,
                }),
            }]
        );
        // No password goes from the app: the backend reads it.
        assert_eq!(secrets.password, None);
        harness.app.apply(Action::Backend(Event::FilesDownloading {
            request,
            reader: Reader::Postgres,
        }));
        assert!(
            says(
                &mut harness,
                &look.label("Downloading the PostgreSQL reader for DuckDB…")
            ),
            "{}",
            look.name
        );
        let words = "Can't attach bookshop_development: password authentication failed";
        harness.app.apply(Action::Backend(Event::FilesTested {
            request,
            result: Err(Error::Connect(words.into())),
        }));
        assert!(says(&mut harness, &look.label(words)), "{}", look.name);
        assert_eq!(form(&harness).files_downloading, None);
    }
}

#[test]
fn an_open_sends_what_is_attached() {
    let mut harness = Harness::new();
    let [dev, _, production, _] = bookshop(&mut harness);
    // Production is a BigQuery connection now, which can't be attached.
    let mut bigquery = harness.app.connections.get(&production).unwrap().clone();
    bigquery.spec = ConnectSpec::bigquery("bookshop-analytics", None);
    harness.app.connections.upsert(bigquery);
    let mut spec = ConnectSpec::files(Source::Folder("/data/bookshop-exports".into()));
    spec.attach = vec![
        Attach {
            connection: dev.0.clone(),
            name: "bookshop_dev".into(),
        },
        Attach {
            connection: "deleted".into(),
            name: "gone".into(),
        },
        Attach {
            connection: production.0.clone(),
            name: "now_bigquery".into(),
        },
    ];
    let saved = SavedConnection {
        id: ConnectionId::new(),
        name: "Bookshop exports".into(),
        group: None,
        environment: Environment::Local,
        read_only: Some(true),
        password: PasswordMode::None,
        ssh_secret: PasswordMode::None,
        spec,
        agents: Default::default(),
    };
    let id = saved.id.clone();
    harness.app.connections.upsert(saved);
    let tab = harness.app.active_tab_id();
    harness.app.apply(Action::Connect {
        tab,
        conn: id.clone(),
    });
    let Command::Connect { attach, .. } = crate::testing::last_sent(&harness.app) else {
        panic!("expected Connect");
    };
    let names: Vec<(&str, bool)> = attach
        .iter()
        .map(|each| (each.name.as_str(), each.saved.is_some()))
        .collect();
    // One deleted since goes too: the backend says it cannot attach it.
    assert_eq!(
        names,
        [
            ("bookshop_dev", true),
            ("gone", false),
            ("now_bigquery", true)
        ]
    );
    // Edited, the deleted one and the BigQuery one are let go: their rows
    // are not listed to untick.
    harness.app.apply(Action::EditConnection(id));
    assert_eq!(
        attached(&harness),
        [(dev.0.clone(), "bookshop_dev".to_owned())]
    );
}

#[test]
fn omarchy_space_ticks_a_row_and_the_arrows_move_in_the_list() {
    let (mut harness, [dev, ..]) = joining(Look::omarchy());
    let first = crate::ui::attach_list::toggle_id(0);
    harness.ctx.memory_mut(|memory| memory.request_focus(first));
    harness.settle();
    harness.press(Key::Space, Modifiers::NONE);
    assert_eq!(
        attached(&harness),
        [(dev.0.clone(), "bookshop_development".to_owned())]
    );
    let cursor = form(&harness).files_cursor;
    harness.press(Key::ArrowDown, Modifiers::NONE);
    harness.settle();
    assert!(
        harness
            .ctx
            .memory(|memory| memory.has_focus(crate::ui::attach_list::toggle_id(1)))
    );
    assert_eq!(form(&harness).files_cursor, cursor);
    harness.press(Key::ArrowUp, Modifiers::NONE);
    harness.settle();
    assert!(harness.ctx.memory(|memory| memory.has_focus(first)));
}
