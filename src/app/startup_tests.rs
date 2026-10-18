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

/// Says the newest connect is up, as the backend does.
fn connected(app: &mut App) {
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
        driver: tabletist_db::Driver::Sqlite,
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
