//! Two databases of one connection: the database menu opens a row's
//! database in a new window beside this one, in both looks, and leaves
//! this one as it was.

use egui::accesskit::Role;
use egui::{Key, Modifiers};
use tabletist_db::{ConnectSpec, Driver, ObjectKind, ObjectRef};

use crate::backend::{Command, Event};
use crate::connections::{PasswordMode, SavedConnection};
use crate::model::{Action, ConnTabId, Dialog, SessionStatus};
use crate::testing::{Harness, answer_tree, bookshop_objects, bounds, labels};
use crate::theme::Look;
use crate::ui::workspace::database_menu_id;

/// The Bookshop's development server, whose password is asked for on
/// every connect.
fn bookshop_saved() -> SavedConnection {
    let (spec, _) =
        ConnectSpec::from_url("postgres://dev@localhost:5433/bookshop_development").unwrap();
    SavedConnection {
        id: crate::connections::ConnectionId::new(),
        name: "Bookshop".into(),
        group: None,
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
fn the_new_window_is_on_the_server_this_one_connected_to() {
    let (mut harness, first) = bookshop(Look::macos());
    // The saved connection now names another server; this window is
    // still on the one it connected to, with that one's password.
    let conn = harness.app.workspace(first).unwrap().conn_id.clone();
    let mut edited = harness.app.connections.get(&conn).unwrap().clone();
    edited.spec.host = "db.example.com".into();
    harness.app.connections.upsert(edited);
    let sent = harness.app.backend.sent.len();
    harness.app.apply(Action::OpenDatabaseInNewWindow {
        tab: first,
        database: "bookshop_test".into(),
    });
    let (spec, secrets) = harness.app.backend.sent[sent..]
        .iter()
        .find_map(|command| match command {
            Command::Connect { spec, secrets, .. } => Some((spec, secrets)),
            _ => None,
        })
        .expect("the new window connects");
    assert_eq!(
        spec.host, "localhost",
        "the password goes to its own server"
    );
    assert_eq!(spec.database, "bookshop_test");
    assert_eq!(secrets.password.as_deref(), Some("secret"));
}

#[test]
fn the_key_is_not_listed_in_the_keyboard_settings() {
    let keymap = crate::keymap::Keymap::default();
    for layout in [
        crate::keymap::Layout::Mac,
        crate::keymap::Layout::Windows,
        crate::keymap::Layout::Omarchy,
    ] {
        let listed = keymap
            .rows(layout)
            .iter()
            .any(|row| row.command == crate::keymap::Command::OpenDatabaseInNewWindow);
        assert!(!listed, "{layout:?}");
    }
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

// The menu: what opens a new window from it, and what it says.

/// Opens the bar's own chip's menu of `tab`, as quick open's Switch
/// database asks for it.
fn open_menu(harness: &mut Harness, tab: ConnTabId) {
    harness.settle();
    harness.app.workspace_mut(tab).unwrap().database_menu_asked = true;
    harness.settle();
    assert!(egui::Popup::is_id_open(
        &harness.ctx,
        database_menu_id(tab, false)
    ));
}

/// Clicks the menu's row of `name` by the pointer, with `modifiers` held,
/// as a window tells of them: before the pointer moves.
fn click_row(harness: &mut Harness, name: &str, modifiers: Modifiers) {
    let tree = harness.settle();
    let row = bounds(&tree, name, Role::Button)
        .unwrap_or_else(|| panic!("no row {name:?}: {:?}", labels(&tree)));
    let pos = row.left_center() + egui::vec2(40.0, 0.0);
    let press = |pressed| egui::Event::PointerButton {
        pos,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers,
    };
    harness.frame(vec![
        egui::Event::ModifiersChanged(modifiers),
        egui::Event::PointerMoved(pos),
    ]);
    harness.frame(vec![press(true)]);
    harness.frame(vec![press(false)]);
    harness.frame(vec![egui::Event::ModifiersChanged(Modifiers::NONE)]);
    harness.settle();
}

/// The key of Open database in a new window, as `look` writes it.
fn new_window_key(harness: &Harness, look: &Look) -> String {
    let command = crate::keymap::Command::OpenDatabaseInNewWindow;
    crate::ui::keys::written(&harness.ctx, look, command)
        .as_str()
        .to_owned()
}

/// Moves the keyboard down the open menu to the row of `name`.
fn key_to(harness: &mut Harness, name: &str) {
    for _ in 0..4 {
        harness.press(Key::ArrowDown, Modifiers::NONE);
        if crate::ui::tests::focused_name(&harness.settle()) == name {
            return;
        }
    }
    panic!("the keyboard never came to {name:?}");
}
#[test]
fn cmd_click_opens_a_row_in_a_new_window_and_a_click_switches() {
    let (mut harness, first) = bookshop(Look::macos());
    open_menu(&mut harness, first);
    click_row(&mut harness, "bookshop_test", Modifiers::COMMAND);
    assert_eq!(windows(&harness).len(), 2);
    assert_eq!(database(&harness, first), "bookshop_development");
    let second = harness.app.active_tab_id();
    assert_eq!(database(&harness, second), "bookshop_test");
    assert!(
        !egui::Popup::is_id_open(&harness.ctx, database_menu_id(first, false)),
        "the menu closed"
    );
    // Back in the first window, a plain click switches it.
    answer_connect(&mut harness);
    harness.app.apply(Action::ActivateConnTab(first));
    open_menu(&mut harness, first);
    click_row(&mut harness, "postgres", Modifiers::NONE);
    assert_eq!(windows(&harness).len(), 2, "no other window");
    assert_eq!(database(&harness, first), "postgres");
}

#[test]
fn cmd_enter_on_the_lit_row_opens_it_in_a_new_window() {
    let (mut harness, first) = bookshop(Look::macos());
    open_menu(&mut harness, first);
    key_to(&mut harness, "bookshop_test");
    // The lit row says what the key does, and no other row does.
    assert!(harness.painted_rect("New window").is_some());
    let key = new_window_key(&harness, &Look::macos());
    assert!(!key.is_empty());
    assert!(harness.painted_rect(&key).is_some());
    harness.press(Key::Enter, Modifiers::COMMAND);
    let second = harness.app.active_tab_id();
    assert_ne!(second, first);
    assert_eq!(database(&harness, second), "bookshop_test");
    assert_eq!(database(&harness, first), "bookshop_development");
    assert!(!egui::Popup::is_id_open(
        &harness.ctx,
        database_menu_id(first, false)
    ));
}

#[test]
fn enter_alone_on_the_lit_row_switches_this_window() {
    let (mut harness, first) = bookshop(Look::macos());
    open_menu(&mut harness, first);
    key_to(&mut harness, "bookshop_test");
    harness.press(Key::Enter, Modifiers::NONE);
    assert_eq!(windows(&harness), vec![first]);
    assert_eq!(database(&harness, first), "bookshop_test");
}

#[test]
fn the_hint_shows_on_the_lit_row_only() {
    let (mut harness, first) = bookshop(Look::macos());
    open_menu(&mut harness, first);
    // Nothing lit yet: the pointer is off the menu, the keyboard on the
    // chip.
    assert!(harness.painted_rect("New window").is_none());
    key_to(&mut harness, "bookshop_test");
    let lit = harness.painted_rect("New window").unwrap();
    let tree = harness.settle();
    let row = bounds(&tree, "bookshop_test", Role::Button).unwrap();
    assert!(row.contains(lit.center()), "on bookshop_test's row");
}

#[test]
fn the_omarchy_look_opens_it_with_ctrl_enter_and_says_so() {
    let (mut harness, first) = bookshop(Look::omarchy());
    open_menu(&mut harness, first);
    key_to(&mut harness, "bookshop_test");
    assert!(harness.painted_rect("new window").is_some());
    let key = new_window_key(&harness, &Look::omarchy());
    assert!(!key.is_empty());
    assert!(harness.painted_rect(&key).is_some());
    harness.press(Key::Enter, Modifiers::CTRL);
    let second = harness.app.active_tab_id();
    assert_ne!(second, first);
    assert_eq!(database(&harness, second), "bookshop_test");
}

#[test]
fn the_start_screens_database_menu_opens_a_new_window_too() {
    let (mut harness, first) = bookshop(Look::macos());
    // The start screen shows with no tab open.
    let id = harness.app.workspace(first).unwrap().active_tab.unwrap();
    harness.app.apply(Action::CloseTab { tab: first, id });
    harness.click("Switch database");
    assert!(egui::Popup::is_id_open(
        &harness.ctx,
        database_menu_id(first, true)
    ));
    click_row(&mut harness, "bookshop_test", Modifiers::COMMAND);
    let second = harness.app.active_tab_id();
    assert_ne!(second, first);
    assert_eq!(database(&harness, second), "bookshop_test");
}

#[test]
fn the_omarchy_key_opens_the_menu_with_the_keyboard_on_its_rows() {
    let (mut harness, first) = bookshop(Look::omarchy());
    harness.settle();
    harness.press(Key::D, Modifiers::CTRL | Modifiers::SHIFT);
    assert!(egui::Popup::is_id_open(
        &harness.ctx,
        database_menu_id(first, false)
    ));
    key_to(&mut harness, "bookshop_test");
    harness.press(Key::Enter, Modifiers::CTRL);
    let second = harness.app.active_tab_id();
    assert_ne!(second, first);
    assert_eq!(database(&harness, second), "bookshop_test");
}

/// Sends `select 1` to a new SQL editor of `tab`, in front, and returns it.
fn with_sql_editor(harness: &mut Harness, tab: ConnTabId) -> crate::model::TabId {
    harness.app.apply(Action::NewSqlTab(tab));
    let id = harness.app.workspace(tab).unwrap().active_tab.unwrap();
    let workspace = harness.app.workspace_mut(tab).unwrap();
    let sql = workspace.sql_tab_mut(id).unwrap();
    sql.text = "select 1".into();
    sql.cursor = 0;
    harness.settle();
    id
}

#[test]
fn the_key_opens_a_window_and_runs_no_sql_in_front() {
    for look in [Look::macos(), Look::omarchy()] {
        let modifiers = if look.terminal {
            Modifiers::CTRL
        } else {
            Modifiers::COMMAND
        };
        let (mut harness, first) = bookshop(look);
        with_sql_editor(&mut harness, first);
        let sent = harness.app.backend.sent.len();
        open_menu(&mut harness, first);
        key_to(&mut harness, "bookshop_test");
        harness.press(Key::Enter, modifiers);
        let runs = harness.app.backend.sent[sent..]
            .iter()
            .filter(|command| matches!(command, Command::RunSql { .. }))
            .count();
        assert_eq!(runs, 0, "the SQL ran");
        assert_eq!(windows(&harness).len(), 2, "no new window");
    }
}

#[test]
fn a_click_on_the_chip_leaves_the_keyboard_where_it_was() {
    let (mut harness, first) = bookshop(Look::macos());
    harness.settle();
    let before = harness.ctx.memory(|memory| memory.focused());
    let chip = harness.painted_rect("bookshop_development").unwrap();
    let press = |pressed| egui::Event::PointerButton {
        pos: chip.center(),
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: Modifiers::NONE,
    };
    harness.frame(vec![egui::Event::PointerMoved(chip.center())]);
    harness.frame(vec![press(true)]);
    harness.frame(vec![press(false)]);
    harness.settle();
    assert!(egui::Popup::is_id_open(
        &harness.ctx,
        database_menu_id(first, false)
    ));
    assert_eq!(harness.ctx.memory(|memory| memory.focused()), before);
}

#[test]
fn esc_gives_the_keyboard_back_where_it_was() {
    for look in [Look::macos(), Look::omarchy()] {
        let terminal = look.terminal;
        let (mut harness, first) = bookshop(look);
        with_sql_editor(&mut harness, first);
        if terminal {
            harness.start_typing();
        }
        harness.settle();
        let before = harness.ctx.memory(|memory| memory.focused());
        assert!(before.is_some(), "the editor has the keyboard");
        open_menu(&mut harness, first);
        harness.press(Key::Escape, Modifiers::NONE);
        assert!(!egui::Popup::is_id_open(
            &harness.ctx,
            database_menu_id(first, false)
        ));
        assert_eq!(
            harness.ctx.memory(|memory| memory.focused()),
            before,
            "terminal: {terminal}"
        );
    }
}

#[test]
fn the_menu_leaves_the_keyboard_nowhere_when_it_was_nowhere() {
    for look in [Look::macos(), Look::omarchy()] {
        let terminal = look.terminal;
        let (mut harness, first) = bookshop(look);
        harness.settle();
        assert_eq!(harness.ctx.memory(|memory| memory.focused()), None);
        // Closed by Esc, and by a pick.
        open_menu(&mut harness, first);
        harness.press(Key::Escape, Modifiers::NONE);
        assert!(!egui::Popup::is_id_open(
            &harness.ctx,
            database_menu_id(first, false)
        ));
        harness.settle();
        let focused = harness.ctx.memory(|memory| memory.focused());
        assert_eq!(focused, None, "Esc, terminal: {terminal}");
        open_menu(&mut harness, first);
        key_to(&mut harness, "bookshop_test");
        harness.press(Key::Enter, Modifiers::NONE);
        assert_eq!(database(&harness, first), "bookshop_test");
        harness.settle();
        let focused = harness.ctx.memory(|memory| memory.focused());
        assert_eq!(focused, None, "a pick, terminal: {terminal}");
    }
}

#[test]
fn the_menu_keeps_its_width_as_the_keyboard_moves() {
    let (mut harness, first) = bookshop(Look::macos());
    open_menu(&mut harness, first);
    let width = |harness: &mut Harness| {
        let tree = harness.settle();
        bounds(&tree, "postgres", Role::Button).unwrap().width()
    };
    let unlit = width(&mut harness);
    key_to(&mut harness, "bookshop_test");
    assert_eq!(width(&mut harness), unlit, "lit bookshop_test");
    key_to(&mut harness, "postgres");
    assert_eq!(width(&mut harness), unlit, "lit postgres");
}

#[test]
fn the_hint_is_on_the_keyboards_row_when_the_pointer_is_on_another() {
    let (mut harness, first) = bookshop(Look::macos());
    open_menu(&mut harness, first);
    key_to(&mut harness, "bookshop_test");
    let tree = harness.settle();
    let keyboard = bounds(&tree, "bookshop_test", Role::Button).unwrap();
    let pointer = bounds(&tree, "postgres", Role::Button).unwrap();
    harness.frame(vec![egui::Event::PointerMoved(pointer.center())]);
    harness.settle();
    let hints: Vec<egui::Rect> = harness
        .text_rects
        .iter()
        .filter(|(text, _)| text == "New window")
        .map(|(_, rect)| *rect)
        .collect();
    assert_eq!(hints.len(), 1, "{hints:?}");
    assert!(keyboard.contains(hints[0].center()));
    // And the key goes where the hint is.
    harness.press(Key::Enter, Modifiers::COMMAND);
    let second = harness.app.active_tab_id();
    assert_eq!(database(&harness, second), "bookshop_test");
}

/// The picker in `look` with the fixture connection open in a window and
/// its row selected. Returns the picker's tab and the open window.
fn picker_with_it_open(look: Look) -> (Harness, ConnTabId, ConnTabId) {
    let mut harness = Harness::new();
    harness.set_look(look);
    let open = harness.connect_fake();
    harness.app.apply(Action::ShowConnections);
    let picker = harness.app.active_tab_id();
    let conn = Some(harness.app.workspace(open).unwrap().conn_id.clone());
    harness
        .app
        .apply(Action::SelectConnection { tab: picker, conn });
    harness.settle();
    (harness, picker, open)
}

/// The windows on `open`'s connection, `open` among them.
fn windows_on(harness: &Harness, open: ConnTabId) -> usize {
    let conn = &harness.app.workspace(open).unwrap().conn_id;
    let tabs = harness.app.tabs.iter();
    tabs.filter(|tab| {
        let workspace = harness.app.workspace(tab.id);
        workspace.is_some_and(|workspace| &workspace.conn_id == conn)
    })
    .count()
}

/// Right-clicks the picker's row of the fixture connection, as a mouse
/// does, and returns where its menu's row of `name` is.
fn row_menu(harness: &mut Harness, name: &str) -> egui::Rect {
    let tree = harness.settle();
    let at = bounds(&tree, "Fixture", Role::Button)
        .unwrap_or_else(|| panic!("no row: {:?}", labels(&tree)))
        .center();
    let press = |pressed| egui::Event::PointerButton {
        pos: at,
        button: egui::PointerButton::Secondary,
        pressed,
        modifiers: Modifiers::NONE,
    };
    harness.frame(vec![egui::Event::PointerMoved(at)]);
    harness.frame(vec![press(true)]);
    harness.frame(vec![press(false)]);
    let tree = harness.settle();
    bounds(&tree, name, Role::Button)
        .unwrap_or_else(|| panic!("no {name:?} in the menu: {:?}", labels(&tree)))
}

#[test]
fn open_another_in_an_open_connections_menu_opens_one_more_window() {
    for look in [Look::macos(), Look::omarchy()] {
        let (mut harness, picker, open) = picker_with_it_open(look);
        row_menu(&mut harness, "Open another");
        assert!(!harness.has("Connect again"), "{}", look.name);
        assert_eq!(windows_on(&harness, open), 1, "{}", look.name);
        harness.click("Open another");
        // The picker's tab connects, on the saved database, beside the
        // window that had it open.
        assert_eq!(windows_on(&harness, open), 2, "{}", look.name);
        assert!(harness.app.workspace(picker).is_some(), "{}", look.name);
        assert!(harness.app.workspace(open).is_some(), "{}", look.name);
    }
}

#[test]
fn open_another_names_its_key_while_it_is_lit() {
    for look in Look::ALL {
        let (mut harness, ..) = picker_with_it_open(look);
        let command = crate::keymap::Command::ConnectAgain;
        let key = crate::ui::keys::written(&harness.ctx, &look, command);
        let key = key.as_str().to_owned();
        let show = row_menu(&mut harness, "Show");
        harness.frame(vec![egui::Event::PointerMoved(show.center())]);
        harness.settle();
        assert_eq!(harness.painted_rect(&key), None, "{}: Show lit", look.name);
        let tree = harness.settle();
        let another = bounds(&tree, "Open another", Role::Button).unwrap();
        harness.frame(vec![egui::Event::PointerMoved(another.center())]);
        harness.settle();
        let hint = harness
            .painted_rect(&key)
            .unwrap_or_else(|| panic!("{}: no {key:?}", look.name));
        assert!(another.contains(hint.center()), "{}: {hint:?}", look.name);
    }
}

/// The key acts on the selected row, and a right-click selects nothing:
/// the menu of a row that is not selected names no key it would not keep.
#[test]
fn open_another_names_no_key_on_a_row_that_is_not_selected() {
    for look in Look::ALL {
        let (mut harness, picker, _) = picker_with_it_open(look);
        harness.app.apply(Action::SelectConnection {
            tab: picker,
            conn: None,
        });
        let command = crate::keymap::Command::ConnectAgain;
        let key = crate::ui::keys::written(&harness.ctx, &look, command);
        let key = key.as_str().to_owned();
        let another = row_menu(&mut harness, "Open another");
        harness.frame(vec![egui::Event::PointerMoved(another.center())]);
        harness.settle();
        assert_eq!(harness.painted_rect(&key), None, "{}", look.name);
    }
}
