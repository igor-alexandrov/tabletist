//! Frames for the promotional video: `PROMO_DB=<a SQLite file> cargo test
//! --features shots --lib promo -- --ignored` writes PNGs to target/promo/.
//! The app is drawn by the harness, and what it asks its backend is
//! answered by the real SQLite driver reading that file, read-only.

#![allow(clippy::unwrap_used)]

use std::path::PathBuf;

use tabletist_db::{Access, ConnectSpec, Connection, ObjectKind, ObjectRef, Secrets, StopFlag};

use crate::backend::{Command, Event};
use crate::connections::{ConnectionId, PasswordMode, SavedConnection};
use crate::env::Environment;
use crate::model::{Action, CellPos, ConnTabId, EditStart};
use crate::testing::Harness;

const SIZE: egui::Vec2 = egui::vec2(1200.0, 750.0);

fn out(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target/promo")
        .join(name)
}

/// The file's connection and how many of the harness's commands it has
/// answered.
struct Live {
    runtime: tokio::runtime::Runtime,
    connection: Connection,
    answered: usize,
    /// Numbers the frames in the order they are written.
    frame: usize,
}

impl Live {
    fn open(path: &str) -> Self {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let spec = ConnectSpec::sqlite(path);
        let connection = runtime
            .block_on(Connection::connect(&spec, &Secrets::default()))
            .unwrap();
        Self {
            runtime,
            connection,
            answered: 0,
            frame: 0,
        }
    }

    /// Answers every command the app sent since the last call, and those
    /// the answers make it send.
    fn pump(&mut self, harness: &mut Harness) {
        while let Some(command) = harness.app.backend.sent.get(self.answered) {
            self.answered += 1;
            let connection = &self.connection;
            let event = match command {
                Command::Connect {
                    session,
                    request,
                    access,
                    ..
                } => Some(Event::Connected {
                    session: *session,
                    request: *request,
                    driver: connection.driver(),
                    encrypted: false,
                    access: *access,
                }),
                Command::ListSchemas { session, request } => Some(Event::Schemas {
                    session: *session,
                    request: *request,
                    result: self.runtime.block_on(connection.list_schemas()),
                }),
                Command::ListObjects {
                    session,
                    request,
                    schema,
                } => Some(Event::Objects {
                    session: *session,
                    request: *request,
                    schema: schema.clone(),
                    result: self.runtime.block_on(connection.list_objects(schema)),
                }),
                Command::Describe {
                    session,
                    request,
                    object,
                } => Some(Event::Structure {
                    session: *session,
                    request: *request,
                    result: self.runtime.block_on(connection.describe(object)),
                }),
                Command::FetchRows {
                    session,
                    request,
                    query,
                } => Some(Event::Rows {
                    session: *session,
                    request: *request,
                    result: self.runtime.block_on(connection.fetch_rows(query)),
                }),
                Command::CountRows {
                    session,
                    request,
                    query,
                } => Some(Event::Count {
                    session: *session,
                    request: *request,
                    result: self.runtime.block_on(connection.count_rows(query)),
                }),
                Command::ListDatabases { session, request } => Some(Event::Databases {
                    session: *session,
                    request: *request,
                    result: self.runtime.block_on(connection.list_databases()),
                }),
                Command::ServerVersion { session, request } => Some(Event::ServerVersion {
                    session: *session,
                    request: *request,
                    result: self.runtime.block_on(connection.server_version()),
                }),
                Command::RunSql {
                    session,
                    request,
                    statements,
                    limit,
                    mode,
                    ..
                } => Some(Event::SqlRan {
                    session: *session,
                    request: *request,
                    result: self.runtime.block_on(connection.run_script(
                        statements,
                        *limit,
                        *mode,
                        &StopFlag::new(),
                    )),
                    cancel: None,
                }),
                // Nothing is written, saved or closed here.
                Command::Write { .. }
                | Command::Test { .. }
                | Command::Close { .. }
                | Command::Cancel { .. }
                | Command::LoadSecret { .. }
                | Command::StoreSecret { .. }
                | Command::Save { .. }
                | Command::WatchSettings { .. }
                | Command::EditSettingsFile { .. }
                | Command::RevealSettingsFile { .. }
                | Command::Flush { .. } => None,
            };
            if let Some(event) = event {
                harness.app.apply(Action::Backend(event));
            }
        }
    }

    /// Answers what is asked, draws, and writes the frame as `name`.
    fn shot(&mut self, harness: &mut Harness, name: &str) {
        self.pump(harness);
        harness.settle();
        self.pump(harness);
        harness.shot(&out(&format!("{:03}-{name}.png", self.frame)));
        self.frame += 1;
    }
}

/// The file as a saved connection, as the picker lists it.
fn saved(path: &str) -> SavedConnection {
    SavedConnection {
        id: ConnectionId::new(),
        name: "Fizzy".into(),
        environment: Environment::Dev,
        read_only: Some(false),
        password: PasswordMode::None,
        ssh_secret: PasswordMode::None,
        spec: ConnectSpec::sqlite(path),
    }
}

fn connect(harness: &mut Harness, live: &mut Live, path: &str) -> ConnTabId {
    let saved = saved(path);
    let conn = saved.id.clone();
    harness.app.connections.upsert(saved);
    let tab = harness.app.active_tab_id();
    harness.app.apply(Action::Connect { tab, conn });
    live.pump(harness);
    assert_eq!(live.connection.access(), Access::ReadOnly);
    tab
}

#[test]
#[ignore = "renders with wgpu; run with --features shots -- --ignored"]
fn promo() {
    let Ok(path) = std::env::var("PROMO_DB") else {
        println!("skipped: PROMO_DB names no SQLite file");
        return;
    };
    let _ = std::fs::remove_dir_all(out(""));
    let mut harness = Harness::for_shots(SIZE, 2.0, false, crate::theme::Look::macos());
    // Measured on a macOS 26 window with a unified compact toolbar.
    harness.app.titlebar = crate::app::TitleBar {
        height: 38.0,
        inset: 80.0,
    };
    let live = &mut Live::open(&path);
    let harness = &mut harness;

    harness.app.connections.upsert(saved(&path));
    live.shot(harness, "picker");
    let tab = connect(harness, live, &path);
    live.shot(harness, "connected");
    harness.app.apply(Action::OpenObject {
        tab,
        object: ObjectRef::new("main", "cards"),
        kind: ObjectKind::Table,
        pin: true,
    });
    live.shot(harness, "cards");
    // The grid has the window's width: a row's fields are not shown beside it.
    harness.app.apply(Action::ToggleRowPanel(tab));
    let id = harness.app.workspace(tab).unwrap().active_tab.unwrap();

    // The keyboard walks down the titles.
    for row in 0..5 {
        let cell = CellPos { row, col: TITLE };
        harness.app.apply(Action::SelectCell { tab, id, cell });
        live.shot(harness, "browse");
    }

    // A filter typed on the title.
    harness.app.apply(Action::ToggleFilterBar(tab));
    live.shot(harness, "filter");
    for end in 0..=FIND.len() {
        let workspace = harness.app.workspace_mut(tab).unwrap();
        let object = workspace.object_tab_mut(id).unwrap();
        object.filter.rows = vec![crate::model::FilterRow {
            column: "title".into(),
            op: tabletist_db::FilterOp::Contains,
            value: FIND[..end].into(),
        }];
        live.shot(harness, "filter");
    }
    harness.app.apply(Action::ApplyFilters {
        tab,
        object_tab: id,
    });
    live.shot(harness, "filtered");

    // One of the titles typed over, then what a save would send.
    let cell = CellPos { row: 1, col: TITLE };
    harness.app.apply(Action::SelectCell { tab, id, cell });
    live.shot(harness, "edit");
    let start = EditStart::Value;
    harness.app.apply(Action::EditCell {
        tab,
        id,
        cell,
        start,
    });
    live.shot(harness, "edit");
    for end in KEPT.len()..=RETITLED.len() {
        let workspace = harness.app.workspace_mut(tab).unwrap();
        let editor = workspace.object_tab_mut(id).unwrap().edits.editor.as_mut();
        editor.expect("the cell's editor").text = RETITLED[..end].into();
        harness.app.apply(Action::EditorTyped { tab, id });
        live.shot(harness, "edit");
    }
    harness
        .app
        .apply(crate::testing::leave_edit(&harness.app, tab, id));
    live.shot(harness, "pending");
    let show = true;
    harness.app.apply(Action::ReviewEdits { tab, id, show });
    live.shot(harness, "review");

    // A statement typed in a SQL editor, and its rows.
    harness.app.apply(Action::NewSqlTab(tab));
    live.shot(harness, "sql");
    let sql_tab = harness.app.workspace(tab).unwrap().active_tab.unwrap();
    type_sql(harness, live, SQL_START);
    // The list is open on `boa`: down to `boards`, and Tab takes it.
    let step = 1;
    harness
        .app
        .apply(Action::MoveCompletion { tab, sql_tab, step });
    live.shot(harness, "pick");
    let row = None;
    harness
        .app
        .apply(Action::AcceptCompletion { tab, sql_tab, row });
    live.shot(harness, "picked");
    type_sql(harness, live, SQL_END);
    harness.app.apply(Action::CloseCompletion { tab, sql_tab });
    live.shot(harness, "typed");
    let all = false;
    harness.app.apply(Action::RunSql { tab, sql_tab, all });
    live.shot(harness, "ran");
}

/// The title's place among the columns of `cards`.
const TITLE: usize = 9;

/// What the filter is typed to find.
const FIND: &str = "search";

/// What the edited title keeps of its text, and what it becomes.
const KEPT: &str = "Implement ";
const RETITLED: &str = "Implement full-text search";

/// The statement the SQL editor is given: typed up to a table's first
/// letters, and from the name the completion list wrote.
const SQL_START: &str = "SELECT boards.name, count(*) AS cards\n  FROM cards\n  JOIN boa";
const SQL_END: &str = " ON boards.id = cards.board_id\n GROUP BY boards.name;";

/// Types `text` into the SQL editor that has the keyboard, a frame for
/// each character.
fn type_sql(harness: &mut Harness, live: &mut Live, text: &str) {
    for character in text.chars() {
        let event = if character == '\n' {
            egui::Event::Paste("\n".into())
        } else {
            egui::Event::Text(character.into())
        };
        harness.frame(vec![event]);
        live.shot(harness, "sql");
    }
}
