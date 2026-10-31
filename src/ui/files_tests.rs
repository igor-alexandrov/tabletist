//! The Files connection type in the connection dialog and the connections
//! list, in both looks: the Type row and its kept fields, the Source (a
//! folder or a list) and what was found there, Test, the fixed read-only
//! box, agents, the URL tab that is off, the Omarchy keys, the question
//! before a used connection changes type, and the list's rows.

use std::path::PathBuf;

use egui::accesskit::Role;
use egui::{Key, Modifiers};
use tabletist_db::files::{Format, Found, Scan, Source};
use tabletist_db::{ConnectSpec, Driver, FilesRead};

use crate::backend::{Command, Event};
use crate::connections::{AgentAccess, ConnectionId, PasswordMode, SavedConnection};
use crate::env::Environment;
use crate::model::{Action, ConnectionForm, Dialog, FilesFound, FilesKind, PickTarget, TestState};
use crate::testing::{Harness, node};
use crate::theme::Look;

fn looks() -> [Look; 2] {
    [Look::macos(), Look::omarchy()]
}

/// Whether a node says `text`, alone or as part of what it says: the
/// terminal look runs a part's sentences together.
fn says(harness: &mut Harness, text: &str) -> bool {
    let tree = harness.settle();
    crate::testing::labels(&tree)
        .iter()
        .any(|label| label.contains(text))
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

/// A new connection's dialog in `look`, its type Files.
fn new_files(look: Look) -> Harness {
    let mut harness = Harness::new();
    harness.set_look(look);
    harness.app.apply(Action::NewConnection);
    harness.settle();
    harness.click(&look.label("Files"));
    harness
}

/// The Bookshop's exports as a scan of `folder` finds them.
fn exports(folder: &str) -> Scan {
    let found = |name: &str, format: Format, size: u64| Found {
        path: PathBuf::from(folder).join(name),
        format,
        size,
        name: name.split('.').next().unwrap_or_default().to_owned(),
    };
    Scan {
        files: vec![
            found("orders.csv", Format::Csv, 1_600_000_000),
            found("customers.parquet", Format::Parquet, 400_000_000),
            found("events.ndjson", Format::Ndjson, 450_000_000),
            found("returns.csv", Format::Csv, 50_000_000),
        ],
        missing: Vec::new(),
    }
}

/// Answers the dialog's last read of its Source with `result`.
fn source_read(harness: &mut Harness, result: Result<Scan, String>) {
    let FilesFound::Reading(request) = form(harness).files_found else {
        panic!("no read of the source: {:?}", form(harness).files_found);
    };
    harness
        .app
        .apply(Action::Backend(Event::SourceRead { request, result }));
    harness.settle();
}

/// Picks `folder` as the dialog's folder, as the file dialog answers.
fn pick_folder(harness: &mut Harness, folder: &str) {
    harness.app.apply(Action::PickFolder);
    let request = form(harness).pick_request.expect("a pick in flight");
    assert_eq!(form(harness).pick_target, PickTarget::Folder);
    harness.app.apply(Action::Backend(Event::FilePicked {
        request,
        path: Some(folder.into()),
    }));
    harness.settle();
}

#[test]
fn a_new_files_connection_is_local_read_only_and_lets_agents_read() {
    for look in looks() {
        let harness = new_files(look);
        let form = form(&harness);
        assert_eq!(form.driver, Driver::Files, "{}", look.name);
        assert_eq!(form.environment(), Environment::Local);
        assert_eq!(form.agents.access, AgentAccess::Read);
    }
    // Every other type still starts with agents Off, and a type chosen
    // again moves them back while the person has not chosen.
    let mut harness = new_files(Look::macos());
    harness.click("PostgreSQL");
    assert_eq!(form(&harness).agents.access, AgentAccess::Off);
    harness.click("Files");
    assert_eq!(form(&harness).agents.access, AgentAccess::Read);
    // Production lets agents do nothing until the person says so, as
    // its environment is clicked; Local lets them read again.
    harness.click("Production");
    assert_eq!(form(&harness).agents.access, AgentAccess::Off);
    harness.click("Local");
    assert_eq!(form(&harness).agents.access, AgentAccess::Read);
    // A choice of the person's own stays.
    harness.click("Off");
    harness.click("Dev");
    assert_eq!(form(&harness).agents.access, AgentAccess::Off);
}

#[test]
fn the_type_is_a_radio_group_of_four_on_a_row_of_its_own() {
    for look in looks() {
        let mut harness = new_files(look);
        let tree = harness.settle();
        assert!(node(&tree, &look.label("Type"), Role::RadioGroup).is_some());
        for name in ["SQLite", "PostgreSQL", "MySQL", "Files"] {
            assert!(
                node(&tree, &look.label(name), Role::RadioButton).is_some(),
                "{}: {name}",
                look.name
            );
        }
        assert!(
            node(&tree, &look.label("Source"), Role::RadioGroup).is_some(),
            "{}: the Source choice",
            look.name
        );
        // Type comes before Name: it changes what follows.
        let top = |label: &str| {
            harness
                .painted_rect(&look.label(label))
                .unwrap_or_else(|| panic!("{label} is not painted"))
                .top()
        };
        assert!(top("Type") < top("Name"), "{}", look.name);
    }
    let mut harness = new_files(Look::macos());
    assert!(harness.has("Opens CSV, Parquet, JSON, Excel and .duckdb files as tables"));
    harness.click("SQLite");
    assert!(harness.has("One database file on this computer"));
    harness.click("PostgreSQL");
    assert!(harness.has("Changes the fields below"));
}

#[test]
fn changing_the_type_keeps_the_name_and_every_field_typed() {
    let mut harness = new_files(Look::macos());
    form_mut(&mut harness).name = "Bookshop".into();
    form_mut(&mut harness).environment = Some(Environment::Dev);
    harness.click("PostgreSQL");
    form_mut(&mut harness).host = "db.bookshop.example".into();
    harness.click("Files");
    pick_folder(&mut harness, "/data/bookshop-exports");
    harness.click("PostgreSQL");
    assert_eq!(form(&harness).host, "db.bookshop.example");
    assert_eq!(form(&harness).name, "Bookshop");
    assert_eq!(form(&harness).environment, Some(Environment::Dev));
    harness.click("Files");
    assert_eq!(form(&harness).files_folder, "/data/bookshop-exports");
}

#[test]
fn a_folder_is_read_and_its_files_found() {
    for look in looks() {
        let mut harness = new_files(look);
        pick_folder(&mut harness, "/data/bookshop-exports");
        // The folder names the connection, and the backend reads it.
        assert_eq!(form(&harness).name, "bookshop-exports");
        assert_eq!(
            harness.app.backend.sources_read.last(),
            Some(&Source::Folder("/data/bookshop-exports".into()))
        );
        source_read(&mut harness, Ok(exports("/data/bookshop-exports")));
        match look.terminal {
            true => assert!(harness.has(
                "4 files · 2.5 gb · csv, parquet, ndjson → orders, customers, events, returns"
            )),
            false => {
                assert!(harness.has("4 files · 2.5 GB · CSV, Parquet, NDJSON"));
                for name in ["orders", "customers", "events", "returns"] {
                    assert!(harness.has(name), "{name}");
                }
                assert!(harness.has(
                    "Every CSV, Parquet, JSON, NDJSON, Excel and .duckdb file in the folder \
                     becomes a table. Files added later show up on the next open."
                ));
            }
        }
        // Saved, it opens that folder.
        let saved = form(&harness).to_saved().unwrap();
        assert_eq!(
            saved.spec.files,
            Some(Source::Folder("/data/bookshop-exports".into()))
        );
        assert_eq!(saved.password, PasswordMode::None);
        // Saved read-only, as its box shows, whatever its environment.
        assert_eq!(saved.read_only, Some(true));
    }
}

#[test]
fn a_folder_that_is_gone_says_so_and_the_form_stays_editable() {
    for look in looks() {
        let mut harness = new_files(look);
        pick_folder(&mut harness, "/data/bookshop-exports");
        source_read(
            &mut harness,
            Err("Can't open /data/bookshop-exports: no such folder".into()),
        );
        assert!(harness.has(
            &look.label("This folder isn't there any more. Choose it again, or pick another.")
        ));
        // Another folder is read again.
        pick_folder(&mut harness, "/data/other");
        assert!(matches!(form(&harness).files_found, FilesFound::Reading(_)));
    }
}

#[test]
fn files_are_added_and_taken_off_the_list_and_stay_on_the_disk() {
    for look in looks() {
        let mut harness = new_files(look);
        harness.app.apply(Action::SetFilesKind(FilesKind::Files));
        harness.app.apply(Action::PickDataFiles);
        let request = form(&harness).pick_request.unwrap();
        assert_eq!(form(&harness).pick_target, PickTarget::Files);
        harness.app.apply(Action::Backend(Event::FilesPicked {
            request,
            paths: vec![
                "/data/exports/orders.csv".into(),
                "/data/exports/customers.parquet".into(),
                "/data/exports/orders.csv".into(),
            ],
        }));
        harness.settle();
        assert_eq!(
            form(&harness).files_list,
            [
                "/data/exports/orders.csv",
                "/data/exports/customers.parquet"
            ]
        );
        let tree = harness.settle();
        assert!(
            node(&tree, &look.label("Files"), Role::List).is_some(),
            "{}",
            look.name
        );
        assert!(node(&tree, "/data/exports/orders.csv", Role::ListItem).is_some());
        harness.click("Remove orders.csv");
        assert_eq!(
            form(&harness).files_list,
            ["/data/exports/customers.parquet"]
        );
        assert_eq!(
            harness.app.backend.sources_read.last(),
            Some(&Source::Files(vec![
                "/data/exports/customers.parquet".into()
            ]))
        );
        // Nothing but the read of the list went to the backend: no file
        // is touched.
        assert!(
            !harness
                .app
                .backend
                .sent
                .iter()
                .any(|sent| matches!(sent, Command::Save { .. })),
            "{}",
            look.name
        );
    }
}

#[test]
fn a_test_reads_every_file_and_says_how_much() {
    for look in looks() {
        let mut harness = new_files(look);
        pick_folder(&mut harness, "/data/bookshop-exports");
        source_read(&mut harness, Ok(exports("/data/bookshop-exports")));
        harness.app.apply(Action::TestConnection);
        let TestState::Running(request) = form(&harness).test else {
            panic!("no test: {:?}", form(&harness).test);
        };
        assert!(matches!(
            harness.app.backend.sent.last(),
            Some(Command::Test { spec, .. }) if spec.files.is_some()
        ));
        harness.app.apply(Action::Backend(Event::FilesTested {
            request,
            result: Ok(FilesRead {
                files: 3,
                bytes: 2_400_000_000,
            }),
        }));
        let said = look.label("Read 3 files · 2.4 GB");
        let tree = harness.settle();
        assert!(
            crate::testing::labels(&tree)
                .iter()
                .any(|label| label.starts_with(&said)),
            "{}: {:?}",
            look.name,
            crate::testing::labels(&tree)
        );
        // A folder gone by the time of the Test names it.
        harness.app.apply(Action::TestConnection);
        let TestState::Running(request) = form(&harness).test else {
            panic!("no test");
        };
        harness.app.apply(Action::Backend(Event::FilesTested {
            request,
            result: Err(tabletist_db::Error::Connect(
                "Can't open ~/Data/bookshop-exports: no such folder".into(),
            )),
        }));
        assert_eq!(
            form(&harness).test,
            TestState::Failed("Can't open ~/Data/bookshop-exports: no such folder".into())
        );
    }
}

#[test]
fn files_open_read_only_and_the_box_cannot_be_turned_off() {
    for look in looks() {
        let mut harness = new_files(look);
        let name = "Open read-only";
        let tree = harness.settle();
        let check = node(&tree, name, Role::CheckBox).expect("the read-only box");
        let found = &tree.nodes.iter().find(|(id, _)| *id == check).unwrap().1;
        assert!(found.is_disabled(), "{}", look.name);
        harness.click(name);
        assert_eq!(form(&harness).read_only, None, "{}", look.name);
        pick_folder(&mut harness, "/data/bookshop-exports");
        assert_eq!(form(&harness).to_saved().unwrap().read_only, Some(true));
        // The terminal look paints its promise beside the box.
        match look.terminal {
            true => assert!(harness.painted_rect("[x] files open read-only").is_some()),
            false => assert!(harness.has("Files open read-only.")),
        }
    }
}

#[test]
fn agents_have_no_tables_or_masked_columns_on_files() {
    for look in looks() {
        let mut harness = new_files(look);
        if !crate::endpoint::AVAILABLE {
            return;
        }
        assert!(
            says(
                &mut harness,
                &look.label("Agents can query the files, never change them.")
            ),
            "{}",
            look.name
        );
        for gone in ["Tables", "Masked columns", "Masked", "All except"] {
            assert!(!harness.has(&look.label(gone)), "{}: {gone}", look.name);
        }
    }
}

#[test]
fn the_url_tab_is_off_for_files_and_says_why() {
    let why = "Files connections have no URL. Choose a folder or files under Source.";
    let mut harness = new_files(Look::macos());
    let tree = harness.settle();
    let tab = node(&tree, "URL", Role::RadioButton).expect("the URL tab");
    let url = &tree.nodes.iter().find(|(id, _)| *id == tab).unwrap().1;
    assert!(url.is_disabled());
    assert_eq!(url.description(), Some(why));
    harness.click("URL");
    assert!(!form(&harness).url_mode);
    // The terminal look says so where its key would be, and `u` shows
    // nothing.
    let mut harness = new_files(Look::omarchy());
    assert!(harness.has("no url for files"));
    harness.press(Key::U, Modifiers::NONE);
    assert!(!form(&harness).url_mode);
}

#[test]
fn a_path_typed_as_the_url_opens_it_as_files() {
    let mut harness = Harness::new();
    harness.app.apply(Action::NewConnection);
    harness.settle();
    form_mut(&mut harness).url = "/data/bookshop-exports".into();
    harness.app.apply(Action::ApplyUrl);
    assert_eq!(form(&harness).driver, Driver::Files);
    assert_eq!(form(&harness).files_kind, FilesKind::Folder);
    assert_eq!(form(&harness).files_folder, "/data/bookshop-exports");
    assert_eq!(form(&harness).name, "bookshop-exports");
    assert_eq!(
        harness.app.backend.sources_read.last(),
        Some(&Source::Folder("/data/bookshop-exports".into()))
    );
    // A data file's path is a list of that file.
    form_mut(&mut harness).url = "/data/bookshop-exports/orders.csv".into();
    harness.app.apply(Action::ApplyUrl);
    assert_eq!(form(&harness).files_kind, FilesKind::Files);
    assert_eq!(
        form(&harness).files_list,
        ["/data/bookshop-exports/orders.csv"]
    );
    // A SQLite database's path stays SQLite.
    form_mut(&mut harness).url = "/data/shop.db".into();
    harness.app.apply(Action::ApplyUrl);
    assert_eq!(form(&harness).driver, Driver::Sqlite);
    assert_eq!(form(&harness).sqlite_path, "/data/shop.db");
}

/// A saved PostgreSQL connection like the one `used_bookshop` makes.
fn harness_saved(id: &ConnectionId) -> SavedConnection {
    let (spec, _) =
        ConnectSpec::from_url("postgres://clerk@localhost/bookshop_development").unwrap();
    SavedConnection {
        id: id.clone(),
        name: "Bookshop".into(),
        environment: Environment::Dev,
        read_only: None,
        password: PasswordMode::None,
        ssh_secret: PasswordMode::None,
        spec,
        agents: Default::default(),
    }
}

/// A saved PostgreSQL connection named Bookshop, used once.
fn used_bookshop(harness: &mut Harness) -> ConnectionId {
    let (spec, _) =
        ConnectSpec::from_url("postgres://clerk@localhost/bookshop_development").unwrap();
    let saved = SavedConnection {
        id: ConnectionId::new(),
        name: "Bookshop".into(),
        environment: Environment::Dev,
        read_only: None,
        password: PasswordMode::None,
        ssh_secret: PasswordMode::None,
        spec,
        agents: Default::default(),
    };
    let id = saved.id.clone();
    harness.app.connections.upsert(saved);
    harness.app.connections.mark_used(&id, 1_790_000_000);
    id
}

#[test]
fn a_used_connection_saved_as_another_type_is_asked_about_first() {
    let mut harness = Harness::new();
    let id = used_bookshop(&mut harness);
    harness.app.apply(Action::EditConnection(id.clone()));
    harness.settle();
    harness.click("Files");
    pick_folder(&mut harness, "/data/bookshop-exports");
    harness.app.apply(Action::SaveConnection { connect: false });
    assert_eq!(
        form(&harness).message.as_deref(),
        Some(
            "Change Bookshop from PostgreSQL to Files? Its server settings will be removed. \
             Save again to change it."
        )
    );
    assert_eq!(
        harness.app.connections.get(&id).unwrap().spec.driver,
        Driver::Postgres
    );
    harness.app.apply(Action::SaveConnection { connect: false });
    assert!(harness.app.dialog.is_none());
    let saved = harness.app.connections.get(&id).unwrap();
    assert_eq!(saved.spec.driver, Driver::Files);
    assert!(saved.spec.host.is_empty());
    // One never used changes without a question.
    let mut harness = Harness::new();
    let id = used_bookshop(&mut harness);
    harness.app.connections = crate::connections::SavedConnections::default();
    let mut never = SavedConnection {
        id: ConnectionId::new(),
        ..harness_saved(&id)
    };
    never.name = "Bookshop".into();
    let id = never.id.clone();
    harness.app.connections.upsert(never);
    harness.app.apply(Action::EditConnection(id.clone()));
    harness.settle();
    harness.click("Files");
    pick_folder(&mut harness, "/data/bookshop-exports");
    harness.app.apply(Action::SaveConnection { connect: false });
    assert!(harness.app.dialog.is_none());
}

#[test]
fn the_terminal_keys_pick_add_remove_and_move() {
    // A saved one opens with the keyboard in no field: its letters are
    // keys.
    let mut harness = Harness::new();
    harness.set_look(Look::omarchy());
    let id = saved_exports(&mut harness, Source::Folder("/d".into()));
    harness.app.apply(Action::EditConnection(id));
    harness.settle();
    harness.press(Key::O, Modifiers::CTRL);
    assert_eq!(form(&harness).pick_target, PickTarget::Folder);
    assert!(form(&harness).pick_request.is_some());
    harness.app.apply(Action::SetFilesKind(FilesKind::Files));
    harness.settle();
    harness.press(Key::O, Modifiers::CTRL);
    assert_eq!(form(&harness).pick_target, PickTarget::Files);
    form_mut(&mut harness).pick_request = None;
    harness.press(Key::A, Modifiers::NONE);
    let request = form(&harness).pick_request.expect("a adds files");
    harness.app.apply(Action::Backend(Event::FilesPicked {
        request,
        paths: vec![
            "/d/orders.csv".into(),
            "/d/customers.parquet".into(),
            "/d/events.ndjson".into(),
        ],
    }));
    harness.settle();
    assert_eq!(form(&harness).files_cursor, Some(2));
    harness.press(Key::ArrowUp, Modifiers::NONE);
    harness.press(Key::ArrowUp, Modifiers::NONE);
    assert_eq!(form(&harness).files_cursor, Some(0));
    harness.press(Key::ArrowDown, Modifiers::NONE);
    harness.press(Key::X, Modifiers::NONE);
    assert_eq!(
        form(&harness).files_list,
        ["/d/orders.csv", "/d/events.ndjson"]
    );
    // The footer's keys, as the keymap writes them.
    let keymap = crate::keymap::Keymap::default();
    for (command, word) in [
        (crate::keymap::Command::FormNextField, "next"),
        (crate::keymap::Command::FormChoose, "choose"),
        (crate::keymap::Command::FormPick, "pick"),
        (crate::keymap::Command::FormTest, "test"),
        (crate::keymap::Command::FormSave, "save"),
        (crate::keymap::Command::FormCancel, "cancel"),
    ] {
        let key = keymap.key(crate::keymap::Layout::Omarchy, command);
        harness.settle();
        let hint = format!("{} {word}", key.as_str());
        assert!(harness.painted_rect(&hint).is_some(), "{hint}");
    }
}

#[test]
fn no_editor_key_is_h_or_l() {
    let keymap = crate::keymap::Keymap::default();
    for layout in [crate::keymap::Layout::Mac, crate::keymap::Layout::Omarchy] {
        for letter in ["h", "l"] {
            let found = keymap.typed(
                layout,
                crate::keymap::Scope::Prompt,
                &|holds| holds == crate::keymap::When::ConnectionForm,
                letter,
            );
            assert!(
                matches!(found, crate::keymap::Typed::Nothing),
                "{layout:?} {letter}: {found:?}"
            );
        }
    }
    // The same question finds the form's own letters.
    let found = keymap.typed(
        crate::keymap::Layout::Omarchy,
        crate::keymap::Scope::Prompt,
        &|holds| holds == crate::keymap::When::ConnectionForm,
        "a",
    );
    assert!(matches!(
        found,
        crate::keymap::Typed::Command(crate::keymap::Command::FormAddFiles)
    ));
}

#[test]
fn the_terminal_keys_test_save_and_cancel_a_files_connection() {
    let mut harness = Harness::new();
    harness.set_look(Look::omarchy());
    let id = saved_exports(&mut harness, Source::Folder("/d".into()));
    harness.app.apply(Action::EditConnection(id.clone()));
    harness.settle();
    harness.press(Key::T, Modifiers::CTRL);
    assert!(matches!(form(&harness).test, TestState::Running(_)));
    harness.press(Key::Escape, Modifiers::NONE);
    assert!(harness.app.dialog.is_none(), "the cancel key closes");
    harness.app.apply(Action::EditConnection(id.clone()));
    harness.settle();
    form_mut(&mut harness).name = "Bookshop exports, renamed".into();
    harness.press(Key::S, Modifiers::CTRL);
    assert!(harness.app.dialog.is_none(), "the save key saves");
    assert_eq!(
        harness.app.connections.get(&id).unwrap().name,
        "Bookshop exports, renamed"
    );
}

#[test]
fn a_connection_made_files_drops_what_agents_may_not_read() {
    let mut harness = Harness::new();
    let id = used_bookshop(&mut harness);
    let mut saved = harness.app.connections.get(&id).unwrap().clone();
    saved.agents.access = AgentAccess::Read;
    saved.agents.tables =
        tabletist_db::agent::Tables::AllExcept(vec!["customers".parse().unwrap()]);
    harness.app.connections.upsert(saved);
    harness.app.apply(Action::EditConnection(id.clone()));
    harness.settle();
    harness.click("Files");
    pick_folder(&mut harness, "/data/bookshop-exports");
    harness.app.apply(Action::SaveConnection { connect: false });
    assert!(
        form(&harness).message.as_deref().is_some_and(
            |said| said.contains("What agents may not read or get masked is taken off too.")
        ),
        "{:?}",
        form(&harness).message
    );
    harness.app.apply(Action::SaveConnection { connect: false });
    let saved = harness.app.connections.get(&id).unwrap();
    assert_eq!(saved.agents.tables, tabletist_db::agent::Tables::All);
    assert!(saved.agents.masked.is_empty());
}

#[test]
fn postgresql_made_mysql_is_not_asked_about() {
    let mut harness = Harness::new();
    let id = used_bookshop(&mut harness);
    harness.app.apply(Action::EditConnection(id.clone()));
    harness.settle();
    harness.click("MySQL");
    harness.app.apply(Action::SaveConnection { connect: false });
    assert!(
        harness.app.dialog.is_none(),
        "{:?}",
        harness.app.dialog.is_some()
    );
    assert_eq!(
        harness.app.connections.get(&id).unwrap().spec.driver,
        Driver::MySql
    );
}

/// A saved Files connection over the Bookshop's exports.
fn saved_exports(harness: &mut Harness, source: Source) -> ConnectionId {
    let saved = SavedConnection {
        id: ConnectionId::new(),
        name: "Bookshop exports".into(),
        environment: Environment::Local,
        read_only: Some(true),
        password: PasswordMode::None,
        ssh_secret: PasswordMode::None,
        spec: ConnectSpec::files(source),
        agents: Default::default(),
    };
    let id = saved.id.clone();
    harness.app.connections.upsert(saved);
    id
}
