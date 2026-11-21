//! Two databases of one connection: a database opens in a new window
//! beside this one, and leaves this one as it was.

use tabletist_db::{ConnectSpec, Driver, ObjectKind, ObjectRef};

use crate::backend::{Command, Event};
use crate::connections::{PasswordMode, SavedConnection};
use crate::model::{Action, ConnTabId, Dialog, SessionStatus};
use crate::testing::{Harness, answer_tree, bookshop_objects};
use crate::theme::Look;

/// The Bookshop's development server, whose password is asked for on
/// every connect.
fn bookshop_saved() -> SavedConnection {
    let (spec, _) =
        ConnectSpec::from_url("postgres://dev@localhost:5433/bookshop_development").unwrap();
    SavedConnection {
        id: crate::connections::ConnectionId::new(),
        name: "Bookshop".into(),
        environment: crate::env::Environment::Dev,
        read_only: Some(true),
        password: PasswordMode::Ask,
        ssh_secret: PasswordMode::None,
        spec,
        agents: Default::default(),
    }
}

/// Answers the last Connect sent as connected, and returns its session.
fn answer_connect(harness: &mut Harness) -> crate::backend::SessionId {
    let (session, request) = harness
        .app
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
    harness.app.apply(Action::Backend(Event::Connected {
        session,
        request,
        driver: Driver::Postgres,
        encrypted: false,
        access: crate::testing::asked_access(&harness.app),
    }));
    session
}

/// The server's databases, as its listing would have answered.
fn list_databases(harness: &mut Harness, tab: ConnTabId) {
    let workspace = harness.app.workspace_mut(tab).unwrap();
    workspace.databases.value = Some(vec![
        "bookshop_development".into(),
        "bookshop_test".into(),
        "postgres".into(),
    ]);
}

/// The Bookshop connected in `look`, its password typed at the prompt,
/// with `books` open and the server's databases listed.
fn bookshop(look: Look) -> (Harness, ConnTabId) {
    let mut harness = Harness::new();
    harness.set_look(look);
    let saved = bookshop_saved();
    let conn = saved.id.clone();
    harness.app.connections.upsert(saved);
    let tab = harness.app.active_tab_id();
    harness.app.apply(Action::Connect { tab, conn });
    let Some(Dialog::Password(prompt)) = &mut harness.app.dialog else {
        panic!("the password is asked for");
    };
    prompt.password = "secret".into();
    harness.app.apply(Action::SubmitPassword);
    answer_connect(&mut harness);
    answer_tree(&mut harness.app, vec!["public".into()], bookshop_objects());
    list_databases(&mut harness, tab);
    harness.app.apply(Action::OpenObject {
        tab,
        object: ObjectRef::new("public", "books"),
        kind: ObjectKind::Table,
        pin: true,
    });
    (harness, tab)
}

/// The windows the bar shows, in its order.
fn windows(harness: &Harness) -> Vec<ConnTabId> {
    harness.app.open_connections().map(|(tab, _)| tab).collect()
}

/// The database `tab` is on.
fn database(harness: &Harness, tab: ConnTabId) -> String {
    harness.app.workspace(tab).unwrap().spec.database.clone()
}
#[test]
fn opening_a_database_in_a_new_window_leaves_this_one_as_it_was() {
    let (mut harness, first) = bookshop(Look::macos());
    let sent = harness.app.backend.sent.len();
    harness.app.apply(Action::OpenDatabaseInNewWindow {
        tab: first,
        database: "bookshop_test".into(),
    });
    let [one, second] = windows(&harness)[..] else {
        panic!("two windows: {:?}", windows(&harness));
    };
    assert_eq!(one, first);
    assert_eq!(
        harness.app.active_tab_id(),
        second,
        "the new one is in front"
    );
    // It connects with the password the first one holds: nothing asked.
    assert!(harness.app.dialog.is_none(), "no second password prompt");
    let connect = harness.app.backend.sent[sent..]
        .iter()
        .find_map(|command| match command {
            Command::Connect { spec, secrets, .. } => Some((spec, secrets)),
            _ => None,
        })
        .expect("the new window connects");
    assert_eq!(connect.0.database, "bookshop_test");
    assert_eq!(connect.1.password.as_deref(), Some("secret"));
    answer_connect(&mut harness);
    // Each on its own database, both connected, the first one's table
    // still open and its saved connection unchanged.
    assert_eq!(database(&harness, first), "bookshop_development");
    assert_eq!(database(&harness, second), "bookshop_test");
    for tab in [first, second] {
        let status = &harness.app.workspace(tab).unwrap().status;
        assert!(matches!(status, SessionStatus::Connected), "{status:?}");
    }
    let tables = |tab| {
        let workspace = harness.app.workspace(tab).unwrap();
        workspace
            .object_tabs()
            .map(|object| object.object.name.clone())
            .collect::<Vec<_>>()
    };
    assert_eq!(tables(first), vec!["books".to_owned()]);
    assert!(tables(second).is_empty(), "the new window opens on nothing");
    let conn = &harness.app.workspace(first).unwrap().conn_id;
    let saved = harness.app.connections.get(conn).unwrap();
    assert_eq!(saved.spec.database, "bookshop_development");
}

#[test]
fn the_new_window_goes_right_after_the_one_it_came_from() {
    let (mut harness, first) = bookshop(Look::macos());
    // A second connection after it, in a window of its own.
    let mut other = bookshop_saved();
    other.name = "Playground".into();
    other.password = PasswordMode::None;
    let conn = other.id.clone();
    harness.app.connections.upsert(other);
    harness.app.apply(Action::ShowConnections);
    let tab = harness.app.active_tab_id();
    harness.app.apply(Action::Connect { tab, conn });
    answer_connect(&mut harness);
    let playground = harness.app.active_tab_id();
    harness.app.apply(Action::ActivateConnTab(first));
    harness.app.apply(Action::OpenDatabaseInNewWindow {
        tab: first,
        database: "bookshop_test".into(),
    });
    let second = harness.app.active_tab_id();
    assert_eq!(windows(&harness), vec![first, second, playground]);
    // The second window's key is the second place's.
    harness.app.apply(Action::ActivateConnTab(playground));
    harness.app.apply(Action::ActivateConnection(1));
    assert_eq!(harness.app.active_tab_id(), second);
}

#[test]
fn the_current_database_opens_a_second_window_on_it() {
    let (mut harness, first) = bookshop(Look::macos());
    harness.app.apply(Action::OpenDatabaseInNewWindow {
        tab: first,
        database: "bookshop_development".into(),
    });
    let second = harness.app.active_tab_id();
    assert_ne!(second, first);
    assert_eq!(database(&harness, second), "bookshop_development");
    assert!(harness.app.workspace(second).unwrap().tabs.is_empty());
}
#[test]
fn quitting_keeps_the_first_windows_tables_not_a_second_ones() {
    let (mut harness, first) = bookshop(Look::macos());
    let conn = harness.app.workspace(first).unwrap().conn_id.clone();
    let kept = |harness: &Harness| {
        let kept = harness
            .app
            .session
            .connections
            .get(&conn.0)
            .cloned()
            .unwrap();
        let names: Vec<String> = kept.tabs.into_iter().map(|tab| tab.name).collect();
        (kept.database, names)
    };
    let books = ("bookshop_development".to_owned(), vec!["books".to_owned()]);
    // A second window on the database the first is on, empty.
    harness.app.apply(Action::OpenDatabaseInNewWindow {
        tab: first,
        database: "bookshop_development".into(),
    });
    answer_connect(&mut harness);
    harness.app.save_session();
    assert_eq!(kept(&harness), books);
    // And one on another database: it keeps nothing of the connection's.
    harness.app.apply(Action::OpenDatabaseInNewWindow {
        tab: first,
        database: "bookshop_test".into(),
    });
    answer_connect(&mut harness);
    harness.app.save_session();
    assert_eq!(kept(&harness), books);
}
