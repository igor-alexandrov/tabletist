//! What is kept of the windows for the next launch (`session.json`), and
//! what a launch and an open of a connection make of it.

use super::*;
use crate::backend::Backend;
use crate::session::{Kept, Session};
use tabletist_db::{FilterOp, ObjectKind, ObjectRef, SortDir};

/// An app over a directory of its own, as the tests of the other state
/// files have it.
fn app_in(dir: &std::path::Path) -> App {
    App::new(
        AppDirs::at(dir),
        Settings::default().into(),
        Backend::recording(),
    )
}

/// A saved Bookshop on a SQLite file, by its own id.
fn bookshop(app: &mut App, name: &str) -> crate::connections::ConnectionId {
    let mut saved = crate::testing::fixture_connection();
    saved.name = name.into();
    saved.spec = tabletist_db::ConnectSpec::sqlite("/tmp/bookshop.db");
    let id = saved.id.clone();
    app.connections.upsert(saved);
    id
}

/// Opens `conn` in a window of its own, in front, and says it connected.
fn open_window(app: &mut App, conn: &crate::connections::ConnectionId) -> ConnTabId {
    let tab = match app.picker_index() {
        Some(index) => app.tabs[index].id,
        None => {
            let tab = app.picker_tab();
            let id = tab.id;
            app.tabs.push(tab);
            id
        }
    };
    app.apply(Action::Connect {
        tab,
        conn: conn.clone(),
    });
    let index = app.tab_index(tab).unwrap();
    app.active = index;
    tab
}

fn books(app: &mut App, tab: ConnTabId, name: &str) -> TabId {
    app.open_object(tab, ObjectRef::new("main", name), ObjectKind::Table, true);
    app.workspace(tab).unwrap().active_tab.unwrap()
}

/// The newest session.json the app sent to be written.
fn written(app: &App) -> Option<Session> {
    let path = app.dirs.session_file();
    app.backend
        .sent
        .iter()
        .rev()
        .find_map(|command| match command {
            Command::Save {
                path: to,
                file: StateFile::Session(session),
            } if *to == path => Some(session.clone()),
            _ => None,
        })
}

#[test]
fn closing_a_window_keeps_its_tables_with_their_sort_and_filters() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = app_in(dir.path());
    let conn = bookshop(&mut app, "Bookshop");
    let tab = open_window(&mut app, &conn);
    let titles = books(&mut app, tab, "books");
    app.apply(Action::SortBy {
        tab,
        object_tab: titles,
        column: "title".into(),
    });
    let authors = books(&mut app, tab, "authors");
    if let Some(object) = app.object_tab_mut(tab, authors) {
        object.filter.open = true;
        object.filter.rows.push(crate::model::FilterRow {
            column: "country".into(),
            op: FilterOp::Eq,
            value: "IE".into(),
        });
    }
    app.apply(Action::ApplyFilters {
        tab,
        object_tab: authors,
    });
    // A SQL editor is not kept, and the table in front is.
    app.apply(Action::NewSqlTab(tab));
    app.apply(Action::ActivateTab { tab, id: titles });
    assert!(
        written(&app).is_none(),
        "nothing is written while it is open"
    );
    app.apply(Action::CloseConnTab(tab));
    let session = written(&app).expect("written as the window closed");
    let kept = &session.connections[&conn.0];
    assert_eq!(kept.database, "/tmp/bookshop.db");
    assert_eq!(kept.active, Some(0));
    let names: Vec<&str> = kept.tabs.iter().map(|tab| tab.name.as_str()).collect();
    assert_eq!(names, ["books", "authors"]);
    assert_eq!(kept.tabs[0].sort, vec![("title".into(), "asc".into())]);
    assert_eq!(
        kept.tabs[1].filter.rows,
        vec![("country".into(), "eq".into(), "IE".into())]
    );
    // The window is gone: the picker is what is open.
    assert_eq!(session.open, vec![None]);
}

#[test]
fn a_sql_editor_in_front_leaves_no_table_in_front() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = app_in(dir.path());
    let conn = bookshop(&mut app, "Bookshop");
    let tab = open_window(&mut app, &conn);
    books(&mut app, tab, "books");
    app.apply(Action::NewSqlTab(tab));
    let kept = Kept::of(app.workspace(tab).unwrap());
    assert_eq!(kept.tabs.len(), 1);
    assert_eq!(kept.active, None);
}

#[test]
fn the_file_says_which_windows_are_open_in_order_and_which_is_in_front() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = app_in(dir.path());
    let first = bookshop(&mut app, "Bookshop");
    let second = bookshop(&mut app, "Bookshop");
    open_window(&mut app, &first);
    let tab = open_window(&mut app, &second);
    books(&mut app, tab, "books");
    app.apply(Action::ShowConnections);
    let picker = app.active;
    app.active = picker - 1;
    app.save_session();
    let session = written(&app).expect("written");
    assert_eq!(
        session.open,
        vec![Some(first.clone()), Some(second.clone()), None]
    );
    assert_eq!(session.active, 1);
    // Every open window's tables, whether it closed or not.
    assert_eq!(session.connections[&second.0].tabs[0].name, "books");
    assert!(session.connections[&first.0].tabs.is_empty());
}

#[test]
fn another_connection_put_in_a_window_keeps_the_one_it_had() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = app_in(dir.path());
    let first = bookshop(&mut app, "Bookshop");
    let second = bookshop(&mut app, "Bookshop copy");
    let tab = open_window(&mut app, &first);
    books(&mut app, tab, "books");
    app.apply(Action::Connect {
        tab,
        conn: second.clone(),
    });
    let session = written(&app).expect("written as the first closed");
    assert_eq!(session.connections[&first.0].tabs[0].name, "books");
    // And a disconnect with no picker open puts the picker in its place.
    books(&mut app, tab, "authors");
    app.apply(Action::Disconnect(tab));
    let session = written(&app).expect("written as the second closed");
    assert_eq!(session.connections[&second.0].tabs[0].name, "authors");
    assert_eq!(session.open, vec![None]);
}

#[test]
fn a_connection_put_in_its_own_window_reopens_what_the_window_just_had() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = app_in(dir.path());
    let conn = bookshop(&mut app, "Bookshop");
    let tab = open_window(&mut app, &conn);
    books(&mut app, tab, "books");
    app.apply(Action::Connect {
        tab,
        conn: conn.clone(),
    });
    let workspace = app.workspace(tab).unwrap();
    let front = workspace.active_object_tab().expect("books is in front");
    assert_eq!(front.object.name, "books");
    assert!(front.reopened);
    let session = written(&app).expect("written as the window's connection closed");
    assert_eq!(session.connections[&conn.0].tabs[0].name, "books");
}

#[test]
fn a_deleted_connection_takes_its_tables_out_of_the_file() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = app_in(dir.path());
    let conn = bookshop(&mut app, "Bookshop");
    let tab = open_window(&mut app, &conn);
    books(&mut app, tab, "books");
    app.apply(Action::CloseConnTab(tab));
    assert!(written(&app).unwrap().connections.contains_key(&conn.0));
    app.apply(Action::DeleteConnection(conn.clone()));
    assert!(!written(&app).unwrap().connections.contains_key(&conn.0));
}

#[test]
fn a_connection_deleted_while_its_window_is_open_is_in_no_part_of_the_file() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = app_in(dir.path());
    let conn = bookshop(&mut app, "Bookshop");
    let tab = open_window(&mut app, &conn);
    books(&mut app, tab, "books");
    app.apply(Action::DeleteConnection(conn.clone()));
    app.save_session();
    let session = written(&app).unwrap();
    assert!(!session.open.contains(&Some(conn.clone())));
    assert!(!session.connections.contains_key(&conn.0));
}

#[test]
fn a_connection_deleted_from_the_file_by_hand_is_taken_out_of_the_session_at_the_next_write() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = app_in(dir.path());
    let kept = bookshop(&mut app, "Bookshop");
    write_files(&app);
    let entry = serde_json::json!({
        "database": "/tmp/bookshop.db",
        "tabs": [{"schema": "main", "name": "books", "kind": "table"}]
    });
    // One that connections.toml has, one it lacks, and one of a newer
    // version that this one cannot read.
    let text = serde_json::json!({
        "open": [kept.0],
        "connections": {
            kept.0.clone(): entry.clone(),
            "gone": entry,
            "gone-too": {"tabs": "not a list"},
        }
    });
    std::fs::write(app.dirs.session_file(), text.to_string()).unwrap();
    let mut app = start(dir.path());
    app.save_session();
    let session = written(&app).unwrap();
    assert!(session.connections.contains_key(&kept.0));
    assert!(!session.connections.contains_key("gone"));
    let path = dir.path().join("written.json");
    session.save(&path).unwrap();
    let file = std::fs::read_to_string(&path).unwrap();
    assert!(!file.contains("gone"), "{file}");
}

#[test]
fn the_file_is_read_at_the_start() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = app_in(dir.path());
    let conn = bookshop(&mut app, "Bookshop");
    let tab = open_window(&mut app, &conn);
    books(&mut app, tab, "books");
    app.apply(Action::CloseConnTab(tab));
    // As the backend writes it.
    let session = written(&app).unwrap();
    session.save(&app.dirs.session_file()).unwrap();
    assert_eq!(app_in(dir.path()).session, session);
}

#[test]
fn nothing_is_written_over_the_file_while_the_connections_cannot_be_read() {
    let dir = tempfile::tempdir().unwrap();
    let dirs = AppDirs::at(dir.path());
    std::fs::create_dir_all(&dirs.config).unwrap();
    std::fs::write(
        dirs.connections_file(),
        "version = 3\n\n[[connections]]\nname = 1\n",
    )
    .unwrap();
    let mut app = app_in(dir.path());
    assert!(app.notice.is_some());
    app.save_session();
    assert!(written(&app).is_none());
}

#[test]
fn a_sort_the_other_way_is_kept_as_desc() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = app_in(dir.path());
    let conn = bookshop(&mut app, "Bookshop");
    let tab = open_window(&mut app, &conn);
    let id = books(&mut app, tab, "books");
    for _ in 0..2 {
        app.apply(Action::SortBy {
            tab,
            object_tab: id,
            column: "price".into(),
        });
    }
    assert_eq!(
        app.workspace(tab)
            .unwrap()
            .object_tab(id)
            .unwrap()
            .query
            .sort[0]
            .dir,
        SortDir::Desc
    );
    let kept = Kept::of(app.workspace(tab).unwrap());
    assert_eq!(kept.tabs[0].sort, vec![("price".into(), "desc".into())]);
}

#[test]
fn a_window_switched_to_another_database_keeps_what_its_connection_had() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = app_in(dir.path());
    let conn = bookshop(&mut app, "Bookshop");
    let tab = open_window(&mut app, &conn);
    books(&mut app, tab, "books");
    app.apply(Action::CloseConnTab(tab));
    let tab = open_window(&mut app, &conn);
    // As a switch of database leaves it: another database, its tables.
    if let Some(workspace) = app.workspace_mut(tab) {
        workspace.spec = tabletist_db::ConnectSpec::sqlite("/tmp/bookshop_archive.db");
    }
    books(&mut app, tab, "archived_books");
    app.apply(Action::CloseConnTab(tab));
    let session = written(&app).unwrap();
    let names: Vec<&str> = session.connections[&conn.0]
        .tabs
        .iter()
        .map(|tab| tab.name.as_str())
        .collect();
    assert_eq!(names, ["books"]);
}

#[test]
fn a_window_switched_to_another_database_keeps_the_tables_of_its_own_first() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = app_in(dir.path());
    let conn = bookshop_at(
        &mut app,
        "postgres://demo@localhost:5432/bookshop_development",
    );
    let tab = open_window(&mut app, &conn);
    connected_to(&mut app, tabletist_db::Driver::Postgres);
    books(&mut app, tab, "books");
    app.apply(Action::SwitchDatabase {
        tab,
        database: "bookshop_preview".into(),
    });
    assert_eq!(app.workspace(tab).unwrap().tabs.len(), 0);
    app.apply(Action::CloseConnTab(tab));
    let session = written(&app).unwrap();
    let kept = &session.connections[&conn.0];
    assert_eq!(kept.database, "bookshop_development");
    let names: Vec<&str> = kept.tabs.iter().map(|tab| tab.name.as_str()).collect();
    assert_eq!(names, ["books"]);
}

#[test]
fn a_window_whose_row_panel_is_hidden_keeps_that_and_opens_without_it() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = app_in(dir.path());
    let conn = bookshop(&mut app, "Bookshop");
    let tab = open_window(&mut app, &conn);
    books(&mut app, tab, "books");
    app.apply(Action::ToggleRowPanel(tab));
    assert!(!app.workspace(tab).unwrap().row_panel);
    app.apply(Action::CloseConnTab(tab));
    let session = written(&app).expect("written as the window closed");
    assert!(!session.connections[&conn.0].row_panel);
    // Opened again, the window is as it was left.
    let tab = open_window(&mut app, &conn);
    let workspace = app.workspace(tab).unwrap();
    assert!(!workspace.row_panel);
    assert_eq!(workspace.active_object_tab().unwrap().object.name, "books");
    // Shown again, it opens shown, as a window does.
    app.apply(Action::ToggleRowPanel(tab));
    app.apply(Action::CloseConnTab(tab));
    let session = written(&app).unwrap();
    assert!(session.connections[&conn.0].row_panel);
    let tab = open_window(&mut app, &conn);
    assert!(app.workspace(tab).unwrap().row_panel);
}

#[test]
fn the_row_panel_comes_back_as_it_was_at_quit_whether_or_not_the_tables_do() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = app_in(dir.path());
    let conn = bookshop(&mut app, "Bookshop");
    let tab = open_window(&mut app, &conn);
    books(&mut app, tab, "books");
    app.apply(Action::ToggleRowPanel(tab));
    app.save_session();
    // As the backend writes it, read at the next launch.
    let session = written(&app).expect("written at quit");
    assert!(!session.connections[&conn.0].row_panel);
    session.save(&app.dirs.session_file()).unwrap();
    let mut app = app_in(dir.path());
    app.settings.restore_tabs = false;
    // The connection, as connections.toml would have it.
    let mut saved = crate::testing::fixture_connection();
    saved.id = conn.clone();
    saved.spec = tabletist_db::ConnectSpec::sqlite("/tmp/bookshop.db");
    app.connections.upsert(saved);
    let tab = open_window(&mut app, &conn);
    let workspace = app.workspace(tab).unwrap();
    assert!(!workspace.row_panel, "hidden, as it was");
    assert!(workspace.tabs.is_empty(), "the tables are not reopened");
}

#[test]
fn a_window_switched_to_another_database_keeps_whether_its_row_panel_is_shown() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = app_in(dir.path());
    let conn = bookshop(&mut app, "Bookshop");
    let tab = open_window(&mut app, &conn);
    books(&mut app, tab, "books");
    app.apply(Action::CloseConnTab(tab));
    let tab = open_window(&mut app, &conn);
    // As a switch of database leaves it: another database, its tables.
    if let Some(workspace) = app.workspace_mut(tab) {
        workspace.spec = tabletist_db::ConnectSpec::sqlite("/tmp/bookshop_archive.db");
    }
    books(&mut app, tab, "archived_books");
    app.apply(Action::ToggleRowPanel(tab));
    app.apply(Action::CloseConnTab(tab));
    let session = written(&app).unwrap();
    let kept = &session.connections[&conn.0];
    assert!(!kept.row_panel);
    let names: Vec<&str> = kept.tabs.iter().map(|tab| tab.name.as_str()).collect();
    assert_eq!(names, ["books"], "its own database's tables stay");
    // Opened again: its own tables, and the panel as the window left it.
    let tab = open_window(&mut app, &conn);
    let workspace = app.workspace(tab).unwrap();
    assert!(!workspace.row_panel);
    assert_eq!(workspace.active_object_tab().unwrap().object.name, "books");
    // A window of which no table was ever kept keeps the panel too.
    let other = bookshop(&mut app, "Archive");
    let tab = open_window(&mut app, &other);
    if let Some(workspace) = app.workspace_mut(tab) {
        workspace.spec = tabletist_db::ConnectSpec::sqlite("/tmp/bookshop_archive.db");
    }
    app.apply(Action::ToggleRowPanel(tab));
    app.apply(Action::CloseConnTab(tab));
    let session = written(&app).unwrap();
    let kept = &session.connections[&other.0];
    assert!(!kept.row_panel);
    assert_eq!(kept.database, "/tmp/bookshop.db", "the connection's own");
    assert!(kept.tabs.is_empty());
    let tab = open_window(&mut app, &other);
    assert!(!app.workspace(tab).unwrap().row_panel);
}

/// Says the newest connect is up, as the backend does.
fn connected(app: &mut App) {
    connected_to(app, tabletist_db::Driver::Sqlite);
}

/// Says the newest connect is up, to a server of `driver`.
fn connected_to(app: &mut App, driver: tabletist_db::Driver) {
    let (session, request) = app
        .backend
        .sent
        .iter()
        .rev()
        .find_map(|command| match command {
            Command::Connect {
                session, request, ..
            } => Some((*session, *request)),
            _ => None,
        })
        .expect("a Connect was sent");
    app.apply(Action::Backend(crate::backend::Event::Connected {
        session,
        request,
        driver,
        encrypted: false,
        access: crate::testing::asked_access(app),
    }));
}

/// The pages asked for since the `from`th command, by table.
fn fetched(app: &App, from: usize) -> Vec<tabletist_db::RowQuery> {
    app.backend.sent[from..]
        .iter()
        .filter_map(|command| match command {
            Command::FetchRows { query, .. } => Some(query.clone()),
            _ => None,
        })
        .collect()
}

/// The tables whose structure was asked for since the `from`th command.
fn described(app: &App, from: usize) -> Vec<ObjectRef> {
    app.backend.sent[from..]
        .iter()
        .filter_map(|command| match command {
            Command::Describe { object, .. } => Some(object.clone()),
            _ => None,
        })
        .collect()
}

/// Bookshop's window with `books` sorted by title and `authors` filtered
/// to Ireland, `books` in front, closed: what the session keeps of it.
fn closed_with_tables(app: &mut App) -> crate::connections::ConnectionId {
    let conn = bookshop(app, "Bookshop");
    let tab = open_window(app, &conn);
    connected(app);
    let titles = books(app, tab, "books");
    app.apply(Action::SortBy {
        tab,
        object_tab: titles,
        column: "title".into(),
    });
    let authors = books(app, tab, "authors");
    if let Some(object) = app.object_tab_mut(tab, authors) {
        object.filter.rows.push(crate::model::FilterRow {
            column: "country".into(),
            op: FilterOp::Eq,
            value: "IE".into(),
        });
    }
    app.apply(Action::ApplyFilters {
        tab,
        object_tab: authors,
    });
    app.apply(Action::ActivateTab { tab, id: titles });
    app.apply(Action::CloseConnTab(tab));
    conn
}

#[test]
fn a_connection_opened_again_reopens_its_tables_and_reads_the_one_in_front() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = app_in(dir.path());
    let conn = closed_with_tables(&mut app);
    let tab = open_window(&mut app, &conn);
    app.apply_actions();
    let workspace = app.workspace(tab).unwrap();
    let names: Vec<&str> = workspace
        .object_tabs()
        .map(|object| object.object.name.as_str())
        .collect();
    assert_eq!(names, ["books", "authors"]);
    let front = workspace.active_object_tab().unwrap();
    assert_eq!(front.object.name, "books");
    let authors = workspace.object_tabs().nth(1).unwrap().id;
    let before = app.backend.sent.len();
    connected(&mut app);
    app.apply_actions();
    let pages = fetched(&app, before);
    assert_eq!(pages.len(), 1, "{pages:?}");
    assert_eq!(pages[0].object.name, "books");
    assert_eq!(pages[0].sort[0].column, "title");
    assert_eq!(pages[0].sort[0].dir, SortDir::Asc);
    let books = ObjectRef::new("main", "books");
    assert_eq!(described(&app, before), [books]);
    // The other is read when it is first shown, with its filter.
    let before = app.backend.sent.len();
    app.apply(Action::ActivateTab { tab, id: authors });
    app.apply_actions();
    let pages = fetched(&app, before);
    assert_eq!(pages.len(), 1, "{pages:?}");
    assert_eq!(pages[0].object.name, "authors");
    assert_eq!(described(&app, before), [ObjectRef::new("main", "authors")]);
    assert_eq!(pages[0].filters[0].column, "country");
    assert_eq!(pages[0].filters[0].value, "IE");
    let object = app.workspace(tab).unwrap().object_tab(authors).unwrap();
    assert_eq!(object.filter.rows.len(), 1);
    // And only then.
    let before = app.backend.sent.len();
    app.apply_actions();
    assert!(fetched(&app, before).is_empty());
}

/// A table closed in its DDL view comes back in it, and its definition is
/// read with its rows once the connect is up, and only once.
#[test]
fn a_table_closed_in_its_ddl_view_reopens_in_it_and_reads_its_definition() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = app_in(dir.path());
    let conn = bookshop(&mut app, "Bookshop");
    let tab = open_window(&mut app, &conn);
    connected(&mut app);
    let titles = books(&mut app, tab, "books");
    app.apply(Action::SetView {
        tab,
        object_tab: titles,
        view: crate::model::ObjectView::Ddl,
    });
    app.apply(Action::CloseConnTab(tab));
    let tab = open_window(&mut app, &conn);
    app.apply_actions();
    let front = app.workspace(tab).unwrap().active_object_tab().unwrap();
    assert_eq!(front.view, crate::model::ObjectView::Ddl);
    let before = app.backend.sent.len();
    connected(&mut app);
    app.apply_actions();
    let read: Vec<ObjectRef> = app.backend.sent[before..]
        .iter()
        .filter_map(|command| match command {
            Command::ReadDdl { object, .. } => Some(object.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(read, [ObjectRef::new("main", "books")]);
}

#[test]
fn a_reopened_table_read_by_following_a_key_gets_its_structure_once() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = app_in(dir.path());
    let conn = closed_with_tables(&mut app);
    let tab = open_window(&mut app, &conn);
    // Before the actions are applied, the front table is read by the
    // connect alone, and asked its structure once.
    let before = app.backend.sent.len();
    connected(&mut app);
    assert_eq!(described(&app, before), [ObjectRef::new("main", "books")]);
    app.apply_actions();
    let authors = ObjectRef::new("main", "authors");
    let before = app.backend.sent.len();
    app.apply(Action::OpenFiltered {
        tab,
        object: authors.clone(),
        by: vec![("id".into(), "7".into())],
    });
    app.apply_actions();
    assert_eq!(described(&app, before), std::slice::from_ref(&authors));
    let pages = fetched(&app, before);
    assert_eq!(pages.len(), 1, "{pages:?}");
    assert_eq!(pages[0].object, authors);
    assert_eq!(pages[0].filters[0].column, "id");
}

#[test]
fn the_trees_estimates_reach_the_reopened_tables() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = app_in(dir.path());
    let conn = closed_with_tables(&mut app);
    let tab = open_window(&mut app, &conn);
    connected(&mut app);
    app.apply_actions();
    let (session, request) = app
        .backend
        .sent
        .iter()
        .rev()
        .find_map(|command| match command {
            Command::ListSchemas { session, request } => Some((*session, *request)),
            _ => None,
        })
        .expect("the schemas were asked for");
    app.apply(Action::Backend(crate::backend::Event::Schemas {
        session,
        request,
        result: Ok(vec!["main".into()]),
    }));
    let request = app
        .backend
        .sent
        .iter()
        .rev()
        .find_map(|command| match command {
            Command::ListObjects { request, .. } => Some(*request),
            _ => None,
        })
        .expect("the objects were asked for");
    let listed = |name: &str, estimate| tabletist_db::ObjectInfo {
        name: name.into(),
        kind: ObjectKind::Table,
        estimated_rows: estimate,
    };
    app.apply(Action::Backend(crate::backend::Event::Objects {
        session,
        request,
        schema: "main".into(),
        result: Ok(vec![
            listed("authors", Some(1_200)),
            listed("books", Some(48_000)),
        ]),
    }));
    // The one in front was read with the connect, before the tree was
    // listed; the other has not been read at all.
    let estimates: Vec<Option<u64>> = app
        .workspace(tab)
        .unwrap()
        .object_tabs()
        .map(|object| object.estimated_rows)
        .collect();
    assert_eq!(estimates, [Some(48_000), Some(1_200)]);
}

#[test]
fn with_restore_off_a_connection_opens_with_no_table() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = app_in(dir.path());
    let conn = closed_with_tables(&mut app);
    app.apply(Action::SetOption(
        crate::settings::OptionValue::RestoreTabs(false),
    ));
    assert!(
        app.settings_file
            .text
            .contains("[startup]\nrestore_tabs = false\n")
    );
    let tab = open_window(&mut app, &conn);
    assert_eq!(app.workspace(tab).unwrap().tabs.len(), 0);
    // What it had is still kept, for when the option is on again.
    assert_eq!(app.session.connections[&conn.0].tabs.len(), 2);
}

#[test]
fn tables_of_another_database_are_not_reopened() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = app_in(dir.path());
    let conn = closed_with_tables(&mut app);
    app.session.connections.get_mut(&conn.0).unwrap().database = "bookshop_preview".into();
    let tab = open_window(&mut app, &conn);
    assert_eq!(app.workspace(tab).unwrap().tabs.len(), 0);
}

#[test]
fn a_reopened_table_that_is_gone_says_why_as_an_open_one_does() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = app_in(dir.path());
    let conn = closed_with_tables(&mut app);
    let tab = open_window(&mut app, &conn);
    connected(&mut app);
    app.apply_actions();
    let (session, request) = app
        .backend
        .sent
        .iter()
        .rev()
        .find_map(|command| match command {
            Command::FetchRows {
                session, request, ..
            } => Some((*session, *request)),
            _ => None,
        })
        .unwrap();
    app.apply(Action::Backend(crate::backend::Event::Rows {
        session,
        request,
        result: Err(tabletist_db::Error::Query {
            code: None,
            message: "no such table: books".into(),
            detail: None,
            hint: None,
            named: Default::default(),
        }),
    }));
    let front = app.workspace(tab).unwrap().active_object_tab().unwrap();
    assert!(front.rows.shown_error().is_some());
    // The tab stays, and keeps what it was for the next time.
    assert_eq!(front.query.sort[0].column, "title");
}

/// The Bookshop on a server, by its URL, as the screenshots have it.
fn bookshop_at(app: &mut App, url: &str) -> crate::connections::ConnectionId {
    let (spec, _) = tabletist_db::ConnectSpec::from_url(url).unwrap();
    let mut saved = crate::testing::fixture_connection();
    saved.name = "Bookshop".into();
    saved.spec = spec;
    let id = saved.id.clone();
    app.connections.upsert(saved);
    id
}

/// Writes what the app holds of its connections and settings, as the
/// backend would, for the next start to read.
fn write_files(app: &App) {
    app.dirs.ensure().unwrap();
    app.connections.save(&app.dirs.connections_file()).unwrap();
    std::fs::write(app.dirs.settings_file(), &app.settings_file.text).unwrap();
}

/// A start over `dir`, with the settings its file has.
fn start(dir: &std::path::Path) -> App {
    let dirs = AppDirs::at(dir);
    let settings = Settings::load(&dirs);
    App::new(dirs, settings, Backend::recording())
}

/// The saved connection each window of the bar has, in order: `None` for
/// the picker.
fn windows(app: &App) -> Vec<Option<crate::connections::ConnectionId>> {
    app.tabs
        .iter()
        .map(|tab| match &tab.content {
            ConnTabContent::Workspace(workspace) => Some(workspace.conn_id.clone()),
            ConnTabContent::Picker(_) => None,
        })
        .collect()
}

#[test]
fn a_launch_restores_the_windows_open_at_the_last_quit() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = app_in(dir.path());
    let first = bookshop(&mut app, "Bookshop");
    let second = bookshop(&mut app, "Bookshop");
    open_window(&mut app, &first);
    let tab = open_window(&mut app, &second);
    books(&mut app, tab, "books");
    app.save_session();
    write_files(&app);
    written(&app)
        .unwrap()
        .save(&app.dirs.session_file())
        .unwrap();
    let app = start(dir.path());
    assert_eq!(windows(&app), vec![Some(first), Some(second.clone())]);
    // The one in front is in front again, connecting, with its tables.
    assert_eq!(app.active, 1);
    let workspace = app.workspace(app.tabs[1].id).unwrap();
    assert_eq!(workspace.conn_id, second);
    assert!(matches!(workspace.status, SessionStatus::Connecting { .. }));
    assert_eq!(workspace.object_tabs().count(), 1);
    // The one in front connects first, then the other, each matched to its
    // window by the session and the request it holds.
    let connects: Vec<(SessionId, RequestId)> = app
        .backend
        .sent
        .iter()
        .filter_map(|command| match command {
            Command::Connect {
                session, request, ..
            } => Some((*session, *request)),
            _ => None,
        })
        .collect();
    let held = |index: usize| {
        let workspace = app.workspace(app.tabs[index].id).unwrap();
        match workspace.status {
            SessionStatus::Connecting { request } => (workspace.session, request),
            _ => panic!("window {index} is not connecting"),
        }
    };
    assert_eq!(connects, vec![held(1), held(0)]);
}

#[test]
fn a_launch_with_no_session_and_a_deleted_window_leaves_them_out() {
    let dir = tempfile::tempdir().unwrap();
    // No file: the picker.
    let app = start(dir.path());
    assert_eq!(windows(&app), vec![None]);
    // A window of a connection deleted since, and the picker.
    let mut app = app_in(dir.path());
    let kept = bookshop(&mut app, "Bookshop");
    write_files(&app);
    let mut session = Session::default();
    session.open = vec![
        Some(crate::connections::ConnectionId("deleted".into())),
        None,
        Some(kept.clone()),
    ];
    session.active = 2;
    session.save(&app.dirs.session_file()).unwrap();
    let app = start(dir.path());
    assert_eq!(windows(&app), vec![None, Some(kept)]);
    assert_eq!(app.active, 1);
}

#[test]
fn a_launch_set_to_connections_shows_them_whatever_was_open() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = app_in(dir.path());
    let conn = bookshop(&mut app, "Bookshop");
    open_window(&mut app, &conn);
    app.save_session();
    app.apply(Action::SetOption(crate::settings::OptionValue::OnLaunch(
        crate::settings::Launch::Connections,
    )));
    write_files(&app);
    written(&app)
        .unwrap()
        .save(&app.dirs.session_file())
        .unwrap();
    let app = start(dir.path());
    assert_eq!(windows(&app), vec![None]);
}

#[test]
fn a_launch_opens_the_chosen_connection_by_its_id_of_two_of_one_name() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = app_in(dir.path());
    bookshop_at(
        &mut app,
        "postgres://demo@localhost:5432/bookshop_development",
    );
    let chosen = bookshop_at(&mut app, "postgres://demo@localhost:5432/bookshop_preview");
    app.apply(Action::SetOption(crate::settings::OptionValue::OnLaunch(
        crate::settings::Launch::Connection {
            id: chosen.clone(),
            name: "Bookshop".into(),
        },
    )));
    // The id is what the file holds, and the name its comment.
    let line = format!("on_launch = \"conn:{}\"  # Bookshop\n", chosen.0);
    assert!(
        app.settings_file.text.contains(&line),
        "{}",
        app.settings_file.text
    );
    write_files(&app);
    let app = start(dir.path());
    assert_eq!(windows(&app), vec![Some(chosen.clone())]);
    let workspace = app.workspace(app.tabs[0].id).unwrap();
    assert_eq!(workspace.spec.database, "bookshop_preview");
    assert!(app.notice.is_none());
}

#[test]
fn a_deleted_chosen_connection_shows_connections_says_so_once_and_sets_the_option_back() {
    let dir = tempfile::tempdir().unwrap();
    let dirs = AppDirs::at(dir.path());
    dirs.ensure().unwrap();
    std::fs::write(
        dirs.settings_file(),
        "[startup]\non_launch = \"conn:6a0c1f0e-5a52-4b7b-9c56-3f0c8f2f7a11\"  # Bookshop\n",
    )
    .unwrap();
    let app = start(dir.path());
    assert_eq!(windows(&app), vec![None]);
    assert_eq!(
        app.notice.as_deref(),
        Some(
            "The connection chosen to open at launch was deleted. \
             Tabletist showed Connections instead."
        )
    );
    assert_eq!(app.settings.on_launch, crate::settings::Launch::Connections);
    let saved = app.backend.sent.iter().find_map(|command| match command {
        Command::Save {
            file: StateFile::Settings(text),
            ..
        } => Some(text.clone()),
        _ => None,
    });
    let saved = saved.expect("the settings were written");
    assert!(saved.contains("on_launch = \"connections\""), "{saved}");
    // The next start has nothing to say.
    std::fs::write(dirs.settings_file(), saved).unwrap();
    let again = start(dir.path());
    assert!(again.notice.is_none());
    assert_eq!(windows(&again), vec![None]);
}

#[test]
fn with_connections_that_cannot_be_read_the_chosen_one_is_not_taken_for_deleted() {
    let dir = tempfile::tempdir().unwrap();
    let dirs = AppDirs::at(dir.path());
    dirs.ensure().unwrap();
    std::fs::write(
        dirs.connections_file(),
        "version = 3\n\n[[connections]]\nname = 1\n",
    )
    .unwrap();
    std::fs::write(
        dirs.settings_file(),
        "[startup]\non_launch = \"conn:6a0c1f0e-5a52-4b7b-9c56-3f0c8f2f7a11\"\n",
    )
    .unwrap();
    let app = start(dir.path());
    assert_eq!(windows(&app), vec![None]);
    assert!(matches!(
        app.settings.on_launch,
        crate::settings::Launch::Connection { .. }
    ));
    let notice = app.notice.clone().unwrap();
    assert!(!notice.contains("chosen to open at launch"), "{notice}");
}

#[test]
fn deleting_the_chosen_connection_sets_the_launch_to_connections() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = app_in(dir.path());
    let conn = bookshop(&mut app, "Bookshop");
    app.apply(Action::SetOption(crate::settings::OptionValue::OnLaunch(
        crate::settings::Launch::Connection {
            id: conn.clone(),
            name: "Bookshop".into(),
        },
    )));
    app.apply(Action::DeleteConnection(conn));
    assert_eq!(app.settings.on_launch, crate::settings::Launch::Connections);
    assert!(
        app.settings_file
            .text
            .contains("on_launch = \"connections\"  # restore | connections | \"conn:<id>\"\n"),
        "{}",
        app.settings_file.text
    );
    assert!(app.notice.is_none());
}

#[test]
fn tables_of_another_sqlite_file_are_not_reopened() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = app_in(dir.path());
    let conn = closed_with_tables(&mut app);
    let mut saved = app.connections.get(&conn).unwrap().clone();
    saved.spec = tabletist_db::ConnectSpec::sqlite("/tmp/bookshop_archive.db");
    app.connections.upsert(saved);
    let tab = open_window(&mut app, &conn);
    assert_eq!(app.workspace(tab).unwrap().tabs.len(), 0);
}

#[test]
fn a_restore_asks_first_for_the_window_in_front() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = app_in(dir.path());
    let first = bookshop_at(
        &mut app,
        "postgres://demo@localhost:5432/bookshop_development",
    );
    let second = bookshop_at(&mut app, "postgres://demo@localhost:5432/bookshop_preview");
    for id in [&first, &second] {
        let mut saved = app.connections.get(id).unwrap().clone();
        saved.password = crate::connections::PasswordMode::Ask;
        app.connections.upsert(saved);
    }
    write_files(&app);
    let mut session = Session::default();
    session.open = vec![Some(first.clone()), Some(second.clone())];
    session.active = 1;
    session.save(&app.dirs.session_file()).unwrap();
    let app = start(dir.path());
    // In their order, the second in front, and asking.
    assert_eq!(windows(&app), vec![Some(first), Some(second)]);
    assert_eq!(app.active, 1);
    let front = app.tabs[1].id;
    match &app.dialog {
        Some(crate::model::Dialog::Password(prompt)) => assert_eq!(prompt.tab, front),
        other => panic!("{other:?}"),
    }
    // The window behind found the dialog open: it waits for Reconnect,
    // which asks then.
    let behind = app.workspace(app.tabs[0].id).unwrap();
    assert!(behind.needs_prompt.is_some());
    assert!(matches!(behind.status, SessionStatus::Disconnected(_)));
}

#[test]
fn a_window_behind_that_asks_before_the_front_one_takes_the_question() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = app_in(dir.path());
    let first = bookshop_at(
        &mut app,
        "postgres://demo@localhost:5432/bookshop_development",
    );
    let second = bookshop_at(&mut app, "postgres://demo@localhost:5432/bookshop_preview");
    // The front window reads its password from the keyring, which takes
    // a while; the one behind asks at once.
    for (id, mode) in [
        (&first, crate::connections::PasswordMode::Ask),
        (&second, crate::connections::PasswordMode::Keyring),
    ] {
        let mut saved = app.connections.get(id).unwrap().clone();
        saved.password = mode;
        app.connections.upsert(saved);
    }
    write_files(&app);
    let mut session = Session::default();
    session.open = vec![Some(first.clone()), Some(second.clone())];
    session.active = 1;
    session.save(&app.dirs.session_file()).unwrap();
    let app = start(dir.path());
    assert_eq!(app.active, 1);
    let (behind, front) = (app.tabs[0].id, app.tabs[1].id);
    match &app.dialog {
        Some(crate::model::Dialog::Password(prompt)) => assert_eq!(prompt.tab, behind),
        other => panic!("{other:?}"),
    }
    // The front window is waiting for the keyring.
    let front_session = app.workspace(front).unwrap().session;
    assert!(app.pending_secrets.values().any(|purpose| matches!(
        purpose,
        SecretPurpose::Connect { tab, .. } if *tab == front
    )));
    assert!(
        app.backend
            .sent
            .iter()
            .any(|command| matches!(command, Command::LoadSecret { .. }))
    );
    assert!(app.backend.sent.iter().all(|command| !matches!(
        command,
        Command::Connect { session, .. } if *session == front_session
    )));
}

#[test]
fn renaming_the_chosen_connection_renames_its_comment() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = app_in(dir.path());
    let conn = bookshop(&mut app, "Bookshop");
    app.apply(Action::SetOption(crate::settings::OptionValue::OnLaunch(
        crate::settings::Launch::Connection {
            id: conn.clone(),
            name: "Bookshop".into(),
        },
    )));
    app.apply(Action::EditConnection(conn.clone()));
    match app.dialog.as_mut() {
        Some(crate::model::Dialog::Connection(form)) => form.name = "Bookshop staging".into(),
        other => panic!("{other:?}"),
    }
    app.apply(Action::SaveConnection { connect: false });
    let line = format!("on_launch = \"conn:{}\"  # Bookshop staging\n", conn.0);
    assert!(
        app.settings_file.text.contains(&line),
        "{}",
        app.settings_file.text
    );
    // Written, as any change of the settings is.
    let saved = app
        .backend
        .sent
        .iter()
        .rev()
        .find_map(|command| match command {
            Command::Save {
                file: StateFile::Settings(text),
                ..
            } => Some(text.clone()),
            _ => None,
        });
    assert_eq!(saved.as_deref(), Some(app.settings_file.text.as_str()));
}

#[test]
fn a_launch_restores_the_picker_after_a_window_and_in_front() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = app_in(dir.path());
    let kept = bookshop(&mut app, "Bookshop");
    write_files(&app);
    let mut session = Session::default();
    session.open = vec![Some(kept.clone()), None, None];
    session.active = 1;
    session.save(&app.dirs.session_file()).unwrap();
    let app = start(dir.path());
    assert_eq!(windows(&app), vec![Some(kept), None]);
    assert_eq!(app.active, 1);
}

/// A copy an update's helper started opens the windows of the restart,
/// whatever the launch setting and the tables' setting say, and their SQL
/// editors with their text, and deletes what kept them.
#[test]
fn a_relaunch_after_an_update_restores_the_windows_and_their_sql_editors() {
    use crate::update::relaunch::{Editor, Relaunch, Window};
    let dir = tempfile::tempdir().unwrap();
    let mut app = app_in(dir.path());
    let conn = bookshop(&mut app, "Bookshop");
    let tab = open_window(&mut app, &conn);
    books(&mut app, tab, "book_covers");
    app.save_session();
    app.apply(Action::SetOption(crate::settings::OptionValue::OnLaunch(
        crate::settings::Launch::Connections,
    )));
    app.change_settings(|settings| settings.restore_tabs = false);
    write_files(&app);
    written(&app)
        .unwrap()
        .save(&app.dirs.session_file())
        .unwrap();
    let kept = Relaunch {
        windows: vec![Window {
            connection: conn.0.clone(),
            editors: vec![Editor {
                number: 3,
                text: "SELECT * FROM book_covers".into(),
                writes: false,
                at: 1,
            }],
            front: None,
        }],
    };
    let path = app.dirs.relaunch_file();
    crate::util::save_json(&path, &kept).unwrap();
    // A launch of its own shows Connections, as the setting says.
    assert_eq!(windows(&start(dir.path())), vec![None]);
    let dirs = AppDirs::at(dir.path());
    let settings = Settings::load(&dirs);
    let app = App::relaunched(dirs, settings, Backend::recording());
    assert_eq!(windows(&app), vec![Some(conn)]);
    let workspace = app.workspace(app.tabs[0].id).unwrap();
    assert_eq!(workspace.object_tabs().count(), 1);
    let editors: Vec<(u32, &str)> = workspace
        .tabs
        .iter()
        .filter_map(|open| open.as_sql())
        .map(|sql| (sql.number, sql.text.as_str()))
        .collect();
    assert_eq!(editors, [(3, "SELECT * FROM book_covers")]);
    // The next new one counts on.
    assert_eq!(workspace.next_query, 4);
    assert!(!path.exists());
}

/// The SQL editors of `app`'s windows, by window, as their text.
fn sql_texts(app: &App) -> Vec<Vec<String>> {
    app.tabs
        .iter()
        .map(|tab| match &tab.content {
            ConnTabContent::Workspace(workspace) => workspace
                .tabs
                .iter()
                .filter_map(|open| open.as_sql())
                .map(|sql| sql.text.clone())
                .collect(),
            ConnTabContent::Picker(_) => Vec::new(),
        })
        .collect()
}

/// A SQL editor holding `text`, in front of `tab`'s window.
fn add_sql(app: &mut App, tab: ConnTabId, text: &str) -> TabId {
    let id = TabId(app.next_id());
    let workspace = app.workspace_mut(tab).unwrap();
    let sql = workspace.push_sql_tab(id, 1_000, None);
    sql.text = text.into();
    workspace.active_tab = Some(id);
    id
}

/// Restart now in `app`, then a relaunch over the files it leaves: the
/// relaunch file the backend writes, and the session the quit saves.
fn restart_and_relaunch(mut app: App, dir: &std::path::Path) -> App {
    app.updates.install = crate::update::Install::Ready {
        version: "99.0.0".into(),
        handed: None,
    };
    app.apply(Action::RestartToUpdate);
    let kept = app
        .backend
        .sent
        .iter()
        .find_map(|sent| match sent {
            Command::Updates(crate::backend::UpdatesCommand::Install { relaunch, .. }) => {
                Some(relaunch.clone())
            }
            _ => None,
        })
        .expect("handed over");
    app.save_session();
    write_files(&app);
    written(&app)
        .unwrap()
        .save(&app.dirs.session_file())
        .unwrap();
    crate::util::save_json(&app.dirs.relaunch_file(), &kept).unwrap();
    let dirs = AppDirs::at(dir);
    let settings = Settings::load(&dirs);
    App::relaunched(dirs, settings, Backend::recording())
}

/// One connection in two windows, the second in front, which connects
/// first: each window gets its own editors back.
#[test]
fn each_window_of_one_connection_gets_its_own_editors_back() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = app_in(dir.path());
    let conn = bookshop(&mut app, "Bookshop");
    let first = open_window(&mut app, &conn);
    add_sql(&mut app, first, "SELECT 'first window'");
    let second = open_window(&mut app, &conn);
    add_sql(&mut app, second, "SELECT 'second window'");
    app.active = app.tab_index(second).unwrap();
    let app = restart_and_relaunch(app, dir.path());
    assert_eq!(
        sql_texts(&app),
        [
            vec!["SELECT 'first window'"],
            vec!["SELECT 'second window'"]
        ]
    );
}

/// A SQL editor in front between two tables is in front again, between
/// them, in its mode.
#[test]
fn a_relaunch_puts_each_sql_editor_back_in_its_place() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = app_in(dir.path());
    let conn = bookshop(&mut app, "Bookshop");
    let tab = open_window(&mut app, &conn);
    books(&mut app, tab, "book_covers");
    let sql = add_sql(&mut app, tab, "SELECT 2");
    if let Some(editor) = app.sql_tab_mut(tab, sql) {
        editor.mode = RunMode::ReadWrite;
    }
    books(&mut app, tab, "publishers");
    app.workspace_mut(tab).unwrap().active_tab = Some(sql);
    let app = restart_and_relaunch(app, dir.path());
    let workspace = app.workspace(app.tabs[0].id).unwrap();
    let kinds: Vec<bool> = workspace
        .tabs
        .iter()
        .map(|open| open.as_sql().is_some())
        .collect();
    assert_eq!(kinds, [false, true, false]);
    let front = workspace
        .active_tab
        .and_then(|id| workspace.tabs.iter().find(|open| open.id() == id))
        .and_then(|open| open.as_sql());
    assert_eq!(
        front.map(|sql| (sql.text.as_str(), sql.mode)),
        Some(("SELECT 2", RunMode::ReadWrite))
    );
}
