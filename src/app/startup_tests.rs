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

/// A saved Bookshop: the demo's SQLite file, by its own id.
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
