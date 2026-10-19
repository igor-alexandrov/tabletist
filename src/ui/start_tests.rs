//! The start screen: what a connection's window shows with no tab open,
//! in both looks, and the recents it lists.

use egui::accesskit::Role;
use egui::{Key, Modifiers};
use tabletist_db::{Driver, ObjectInfo, ObjectKind, ObjectRef};

use crate::backend::{Command, Event};
use crate::connections::SavedConnection;
use crate::env::Environment;
use crate::model::{Action, ConnTabId, Dialog, Pane, RunMode};
use crate::recents::Recent;
use crate::testing::{Harness, bookshop_objects, bounds, fixture_connection, labels};
use crate::theme::Look;

/// 2026-10-19 10:00:00 UTC: when the tests happen.
const NOW: u64 = 1_792_404_000;

/// A Bookshop development connection, listed, in `look`, with the clock
/// pinned so that every row was opened just now.
fn bookshop(look: Look) -> (Harness, ConnTabId) {
    crate::util::pin_now(Some(NOW));
    let mut harness = Harness::new();
    harness.set_look(look);
    let tab = harness.connect_with(fixture_connection(), bookshop_objects());
    (harness, tab)
}

/// Opens `name` of the Bookshop in `tab` and closes its tab again: the
/// start screen is back, with it among the recents.
fn visit(harness: &mut Harness, tab: ConnTabId, name: &str) {
    harness.app.apply(Action::OpenObject {
        tab,
        object: ObjectRef::new("main", name),
        kind: ObjectKind::Table,
        pin: true,
    });
    let id = harness.app.workspace(tab).unwrap().active_tab.unwrap();
    harness.app.apply(Action::CloseTab { tab, id });
}

/// The labels of the screen's rows, top to bottom.
fn rows(harness: &mut Harness) -> Vec<String> {
    let tree = harness.settle();
    let mut rows: Vec<(f32, String)> = labels(&tree)
        .into_iter()
        .filter(|label| {
            label.contains(", opened ") || label.contains(", table, ") || label.contains(", view, ")
        })
        .filter_map(|label| Some((bounds(&tree, &label, Role::Button)?.top(), label)))
        .collect();
    rows.sort_by(|a, b| a.0.total_cmp(&b.0));
    rows.into_iter().map(|(_, label)| label).collect()
}

/// The label that has `part` in it; the labels there are, where none
/// does.
fn label_with(harness: &mut Harness, part: &str) -> String {
    let tree = harness.settle();
    let labels = labels(&tree);
    labels
        .iter()
        .find(|label| label.contains(part))
        .cloned()
        .unwrap_or_else(|| panic!("no label with {part:?} among {labels:?}"))
}

/// Whether the last frame painted `text`.
fn painted(harness: &Harness, text: &str) -> bool {
    harness.painted.iter().any(|(painted, _)| painted == text)
}

/// Types one key as the window sends it: the key, and the character it
/// types.
fn type_key(harness: &mut Harness, key: Key, text: &str) {
    harness.settle();
    harness.frame(vec![
        crate::testing::key(key, Modifiers::NONE),
        egui::Event::Text(text.into()),
    ]);
    harness.frame(vec![crate::testing::release(key, Modifiers::NONE)]);
    harness.settle();
}

/// How many times the app asked for `path` to be written.
fn saves_of(harness: &Harness, path: &std::path::Path) -> usize {
    harness
        .app
        .backend
        .sent
        .iter()
        .filter(|command| matches!(command, Command::Save { path: to, .. } if to == path))
        .count()
}

/// Where the production Bookshop is.
fn production_spec() -> tabletist_db::ConnectSpec {
    tabletist_db::ConnectSpec::sqlite("/tmp/bookshop_production.db")
}

/// A saved Bookshop production connection, on its own file.
fn production() -> SavedConnection {
    let mut saved = fixture_connection();
    saved.name = "Bookshop · production".into();
    saved.environment = Environment::Production;
    saved.spec = production_spec();
    saved
}

/// Opens `saved` in a window of its own, in front, listed with the
/// Bookshop's objects.
fn second_window(harness: &mut Harness, saved: SavedConnection) -> ConnTabId {
    harness.app.apply(Action::ShowConnections);
    harness.connect_with(saved, bookshop_objects())
}

/// Test 1: the screen shows with no tab, goes when one opens, and comes
/// back when the last tab closes.
#[test]
fn the_start_screen_shows_with_no_tab_and_comes_back_when_the_last_closes() {
    for (look, said) in [
        (Look::macos(), "No table open"),
        (Look::omarchy(), "no table open"),
    ] {
        let (mut harness, tab) = bookshop(look);
        assert!(harness.has(said), "{}", look.name);
        assert!(!harness.has("Select a table or view in the sidebar"));
        harness.click("book_covers");
        assert!(!harness.has(said), "{}: a tab is open", look.name);
        let id = harness.app.workspace(tab).unwrap().active_tab.unwrap();
        harness.app.apply(Action::CloseTab { tab, id });
        assert!(harness.has(said), "{}: the last tab closed", look.name);
    }
}

/// The connection line: the database, the server as it names itself once
/// it has, and how many tables and views the schema lists.
#[test]
fn the_connection_line_names_the_server_and_counts_the_objects() {
    let (mut harness, tab) = bookshop(Look::macos());
    let line = label_with(&mut harness, " · SQLite · ");
    assert!(line.ends_with("SQLite · 9 tables and views"), "{line}");
    // The version was asked for with the connect, and the line takes it.
    let asked = harness
        .app
        .backend
        .sent
        .iter()
        .rev()
        .find_map(|command| match command {
            Command::ServerVersion { session, request } => Some((*session, *request)),
            _ => None,
        })
        .expect("the version was asked for at connect");
    harness.app.apply(Action::Backend(Event::ServerVersion {
        session: asked.0,
        request: asked.1,
        result: Ok("SQLite 3.46.1".into()),
    }));
    let line = label_with(&mut harness, "SQLite 3.46.1");
    assert!(
        line.ends_with("SQLite 3.46.1 · 9 tables and views"),
        "{line}"
    );
    assert_eq!(harness.app.workspace(tab).unwrap().tabs.len(), 0);
    // The terminal look counts objects.
    harness.set_look(Look::omarchy());
    let line = label_with(&mut harness, "sqlite 3.46.1");
    assert!(line.ends_with("sqlite 3.46.1 · 9 objects"), "{line}");
}

/// Test 7: a production connection's line is the production strip, with
/// Read-only where it is, whatever Environment color says.
#[test]
fn production_shows_the_strip_whatever_the_environment_color_says() {
    for look in [Look::macos(), Look::omarchy()] {
        crate::util::pin_now(Some(NOW));
        let mut harness = Harness::new();
        harness.set_look(look);
        harness.app.settings.env_bar = crate::settings::EnvBar::Label;
        harness.connect_with(production(), bookshop_objects());
        let line = label_with(&mut harness, &look.label("SQLite · 9"));
        let expected = look.label("Read-only · SQLite · 9 tables and views");
        let expected = if look.terminal {
            "read-only · sqlite · 9 objects".to_owned()
        } else {
            expected
        };
        assert!(line.ends_with(&expected), "{}: {line}", look.name);
        let platform = crate::env::Platform::of(&look);
        let colors =
            crate::env::env_colors(Environment::Production, platform, &harness.app.palette);
        assert!(
            harness
                .fills
                .iter()
                .any(|(_, color)| *color == colors.bar_bg()),
            "{}: the strip's tint",
            look.name
        );
        let tag = Environment::Production.label(platform);
        assert!(painted(&harness, tag), "{}: the tag", look.name);
    }
}

/// Tests 2 and 3: recents are the connection's, newest first, five of
/// the eight kept, and Clear empties only this connection's.
#[test]
fn recents_are_per_connection_newest_first_five_of_eight_and_cleared_apart() {
    let (mut harness, dev) = bookshop(Look::macos());
    let dev_id = harness.app.workspace(dev).unwrap().conn_id.clone();
    let names = [
        "authors",
        "book_covers",
        "covers",
        "editions",
        "publishers",
        "ratings",
        "reviews",
        "cover_counts",
        "missing_covers",
    ];
    for (index, name) in names.iter().enumerate() {
        crate::util::pin_now(Some(NOW + index as u64));
        visit(&mut harness, dev, name);
    }
    crate::util::pin_now(Some(NOW + 20));
    let shown = rows(&mut harness);
    assert_eq!(shown.len(), 5, "{shown:?}");
    assert_eq!(
        shown[0], "missing_covers, table, main, opened Just now",
        "{shown:?}"
    );
    assert_eq!(shown[1], "cover_counts, table, main, opened Just now");
    assert_eq!(shown[4], "publishers, table, main, opened Just now");
    let database = crate::session::database_of(&harness.app.workspace(dev).unwrap().spec);
    assert_eq!(harness.app.recents.of(&dev_id.0, &database).len(), 8);
    let file = harness.app.dirs.recents_file();
    assert_eq!(saves_of(&harness, &file), 9, "written at every open");
    assert!(harness.has("Recent on this connection"));
    // Opening one again puts it first.
    crate::util::pin_now(Some(NOW + 30));
    visit(&mut harness, dev, "authors");
    assert_eq!(
        rows(&mut harness)[0],
        "authors, table, main, opened Just now"
    );
    // Production has none of them: its first visit lists the largest tables.
    let production = second_window(&mut harness, production());
    let prod_id = harness.app.workspace(production).unwrap().conn_id.clone();
    assert!(harness.has("Largest tables") || harness.has("Tables"));
    assert!(
        !rows(&mut harness)
            .iter()
            .any(|row| row.contains(", opened ")),
        "nothing was opened on production"
    );
    assert!(harness.app.recents.of(&prod_id.0, &database).is_empty());
    assert!(
        harness
            .app
            .recents
            .of(&prod_id.0, &crate::session::database_of(&production_spec()))
            .is_empty()
    );
    visit(&mut harness, production, "publishers");
    assert_eq!(rows(&mut harness).len(), 1);
    let prod_database =
        crate::session::database_of(&harness.app.workspace(production).unwrap().spec);
    // Clear on development empties development alone.
    harness.app.apply(Action::ActivateConnTab(dev));
    harness.click("Clear");
    assert!(harness.app.recents.of(&dev_id.0, &database).is_empty());
    assert_eq!(harness.app.recents.of(&prod_id.0, &prod_database).len(), 1);
    assert!(!harness.has("Recent on this connection"));
    assert_eq!(saves_of(&harness, &file), 12);
}

/// Test 3: a recent the schema lists no more is dropped, without a word,
/// and the file written without it.
#[test]
fn a_recent_that_no_longer_exists_is_dropped_when_the_schema_is_listed() {
    crate::util::pin_now(Some(NOW));
    let mut harness = Harness::new();
    harness.set_look(Look::macos());
    let saved = fixture_connection();
    let database = crate::session::database_of(&saved.spec);
    for (name, opened) in [("ghost", NOW - 100), ("book_covers", NOW - 200)] {
        let recent = Recent::of(
            &ObjectRef::new("main", name),
            ObjectKind::Table,
            &database,
            opened,
        );
        harness.app.recents.note(&saved.id.0, recent);
    }
    let conn = saved.id.clone();
    let file = harness.app.dirs.recents_file();
    let before = saves_of(&harness, &file);
    let tab = harness.connect_with(saved, bookshop_objects());
    let shown = rows(&mut harness);
    assert_eq!(shown, ["book_covers, table, main, opened 3 min ago"]);
    assert_eq!(harness.app.recents.of(&conn.0, &database).len(), 1);
    assert_eq!(
        saves_of(&harness, &file),
        before + 1,
        "written once it was dropped"
    );
    assert_eq!(harness.app.start_recents(tab).len(), 1);
}

/// Test 4: a first visit lists the largest tables from the catalog's
/// estimates, and nothing is counted or fetched for it. A database with
/// no estimates lists its tables by name.
#[test]
fn the_first_visit_lists_the_largest_tables_from_the_catalog() {
    let (mut harness, tab) = bookshop(Look::macos());
    harness.app.workspace_mut(tab).unwrap().driver = Driver::Postgres;
    let sent = harness.app.backend.sent.len();
    let shown = rows(&mut harness);
    assert_eq!(
        shown,
        [
            "reviews, table, main, ≈ 48,200 rows",
            "ratings, table, main, ≈ 31,900 rows",
            "editions, table, main, ≈ 9,400 rows",
            "authors, table, main, ≈ 3,700 rows",
            "book_covers, table, main, ≈ 13 rows",
        ]
    );
    assert!(harness.has("Largest tables"));
    assert!(harness.has("From table statistics. Nothing is queried."));
    assert!(
        !harness.app.backend.sent[sent..]
            .iter()
            .any(|command| matches!(
                command,
                Command::CountRows { .. } | Command::FetchRows { .. } | Command::RunSql { .. }
            )),
        "nothing is queried for the list"
    );
    // The terminal look writes the estimate short.
    harness.set_look(Look::omarchy());
    assert_eq!(rows(&mut harness)[0], "reviews, table, main, ≈ 48.2k");
    // SQLite has no estimates: its tables by name, under a plainer heading.
    crate::util::pin_now(Some(NOW));
    let mut harness = Harness::new();
    harness.set_look(Look::macos());
    let by_name: Vec<ObjectInfo> = bookshop_objects()
        .into_iter()
        .map(|info| ObjectInfo {
            estimated_rows: None,
            ..info
        })
        .collect();
    harness.connect_with(fixture_connection(), by_name);
    let shown = rows(&mut harness);
    assert_eq!(
        shown,
        [
            "authors, table, main",
            "book_covers, table, main",
            "covers, table, main",
            "editions, table, main",
            "publishers, table, main",
        ]
    );
    assert!(harness.has("Tables") && !harness.has("Largest tables"));
    assert!(harness.has("Nothing is queried."));
    // A row opens its table.
    harness.click("authors, table, main");
    assert!(harness.has("authors tab"));
}

/// A SQL editor closed with a script is a recent, named by its first
/// comment line, and opens again with the script.
#[test]
fn a_closed_sql_editor_with_a_script_is_a_recent_that_opens_again() {
    let (mut harness, tab) = bookshop(Look::macos());
    harness.app.apply(Action::NewSqlTab(tab));
    let id = harness.app.workspace(tab).unwrap().active_tab.unwrap();
    harness.app.apply(Action::CloseTab { tab, id });
    assert!(
        !rows(&mut harness)
            .iter()
            .any(|row| row.contains(", opened ")),
        "an empty editor is nothing to offer"
    );
    harness.app.apply(Action::NewSqlTab(tab));
    let id = harness.app.workspace(tab).unwrap().active_tab.unwrap();
    let script = "-- ebook covers per publisher\nSELECT 1";
    harness
        .app
        .workspace_mut(tab)
        .unwrap()
        .sql_tab_mut(id)
        .unwrap()
        .text = script.into();
    harness.app.apply(Action::CloseTab { tab, id });
    let shown = rows(&mut harness);
    assert_eq!(
        shown,
        ["Query 2, SQL editor, ebook covers per publisher, opened Just now"]
    );
    harness.click(&shown[0]);
    let workspace = harness.app.workspace(tab).unwrap();
    let sql = workspace
        .active_sql_tab()
        .expect("the editor is open again");
    assert_eq!(sql.text, script);
    assert!(workspace.recent_sql.is_empty());
    assert!(!harness.has("No table open"));
}

/// Test 11: the macOS sidebar lists no recents any more.
#[test]
fn the_mac_sidebar_has_no_recent_list() {
    let (mut harness, _tab) = bookshop(Look::macos());
    harness.click("book_covers");
    harness.click("publishers");
    assert!(!harness.has("Recent book_covers"));
    assert!(
        !harness
            .painted
            .iter()
            .any(|(text, _)| text.eq_ignore_ascii_case("recent")),
        "no Recent section"
    );
}

/// The terminal look's status line with no tab: the mode, the keys, and
/// that no table is open.
#[test]
fn the_terminal_status_line_says_start() {
    let (mut harness, tab) = bookshop(Look::omarchy());
    harness.settle();
    assert!(painted(&harness, "-- START --"));
    assert!(harness.has("no table open"));
    harness.click("book_covers");
    harness.settle();
    assert!(!painted(&harness, "-- START --"));
    let id = harness.app.workspace(tab).unwrap().active_tab.unwrap();
    harness.app.apply(Action::CloseTab { tab, id });
    harness.settle();
    assert!(painted(&harness, "-- START --"));
}

/// Test 10: the screen's keys work only while it has the keyboard; in the
/// sidebar `t` is the tree's, and other letters do nothing.
#[test]
fn the_terminal_start_keys_work_only_with_the_start_screen_focused() {
    let (mut harness, tab) = bookshop(Look::omarchy());
    let pane = |harness: &Harness| harness.app.workspace(tab).unwrap().pane;
    // The tree has the keys after a connect, as ever; ctrl+l gives them to
    // the screen.
    assert_eq!(pane(&harness), Pane::Tree);
    harness.press(Key::L, Modifiers::CTRL);
    assert_eq!(pane(&harness), Pane::Grid);
    // A number opens the row it names: the largest table on a first visit.
    type_key(&mut harness, Key::Num1, "1");
    assert!(harness.has("reviews tab"));
    let id = harness.app.workspace(tab).unwrap().active_tab.unwrap();
    harness.app.apply(Action::CloseTab { tab, id });
    assert_eq!(
        rows(&mut harness)[0],
        "reviews, table, main, opened just now"
    );
    // Then the recents, by their numbers.
    type_key(&mut harness, Key::Num1, "1");
    assert!(harness.has("reviews tab"));
    let id = harness.app.workspace(tab).unwrap().active_tab.unwrap();
    harness.app.apply(Action::CloseTab { tab, id });
    type_key(&mut harness, Key::Num2, "2");
    assert!(!harness.has("reviews tab"), "there is no second row");
    // The letters: a new SQL editor, the connection's form, the Agents tab.
    type_key(&mut harness, Key::T, "t");
    assert!(harness.has("Query 1 tab"));
    let id = harness.app.workspace(tab).unwrap().active_tab.unwrap();
    harness.app.apply(Action::CloseTab { tab, id });
    type_key(&mut harness, Key::E, "e");
    assert!(matches!(harness.app.dialog, Some(Dialog::Connection(_))));
    harness.app.apply(Action::CloseDialog);
    if crate::endpoint::AVAILABLE {
        type_key(&mut harness, Key::A, "a");
        assert!(harness.app.workspace(tab).unwrap().agents_in_front());
        let id = harness.app.workspace(tab).unwrap().active_tab.unwrap();
        harness.app.apply(Action::CloseTab { tab, id });
    }
    // The database menu, where there is another database.
    {
        let workspace = harness.app.workspace_mut(tab).unwrap();
        workspace.driver = Driver::Postgres;
        workspace.spec.database = "bookshop_development".into();
        workspace.databases.value =
            Some(vec!["bookshop_development".into(), "bookshop_test".into()]);
    }
    type_key(&mut harness, Key::D, "d");
    let menu = crate::ui::workspace::database_menu_id(tab, true);
    assert!(egui::Popup::is_id_open(&harness.ctx, menu));
    harness.press(Key::Escape, Modifiers::NONE);
    assert!(!egui::Popup::is_id_open(&harness.ctx, menu));
    // Its global chord opens the bar's own chip's menu, anywhere.
    harness.press(Key::D, Modifiers::CTRL | Modifiers::SHIFT);
    let chip = crate::ui::workspace::database_menu_id(tab, false);
    assert!(egui::Popup::is_id_open(&harness.ctx, chip));
    harness.press(Key::Escape, Modifiers::NONE);
    // With the sidebar focused the keys are the tree's: `t` switches the
    // list, the rest does nothing.
    harness.press(Key::H, Modifiers::CTRL);
    assert_eq!(pane(&harness), Pane::Tree);
    let flat = harness.app.workspace(tab).unwrap().tree.flat;
    type_key(&mut harness, Key::T, "t");
    assert_ne!(harness.app.workspace(tab).unwrap().tree.flat, flat);
    type_key(&mut harness, Key::Num1, "1");
    type_key(&mut harness, Key::E, "e");
    type_key(&mut harness, Key::X, "x");
    assert!(harness.app.workspace(tab).unwrap().tabs.is_empty());
    assert!(harness.app.dialog.is_none());
    // And back to the screen.
    harness.press(Key::L, Modifiers::CTRL);
    assert_eq!(pane(&harness), Pane::Grid);
    type_key(&mut harness, Key::Num1, "1");
    assert!(harness.has("reviews tab"));
}

/// The other looks leave the screen's letters and numbers alone: typed,
/// they open nothing.
#[test]
fn the_mac_look_has_no_start_letters() {
    let (mut harness, tab) = bookshop(Look::macos());
    type_key(&mut harness, Key::Num1, "1");
    type_key(&mut harness, Key::T, "t");
    assert!(harness.app.workspace(tab).unwrap().tabs.is_empty());
    // Its buttons do the same work.
    harness.click("New SQL query");
    assert!(harness.has("Query 1 tab"));
}

/// The name a screen reader would read for the focused node.
fn focused_name(tree: &egui::accesskit::TreeUpdate) -> String {
    super::tests::focused_name(tree)
}

/// F6 and Tab reach the screen on macOS, and Enter on a focused row opens
/// it; in the terminal look F6 reaches the pane as ctrl+l does. With no
/// tab the strip is no part to step to.
#[test]
fn f6_and_tab_reach_the_start_screen() {
    let (mut harness, tab) = bookshop(Look::macos());
    visit(&mut harness, tab, "authors");
    let row = "authors, table, main, opened Just now";
    let mut stops = Vec::new();
    for _ in 0..40 {
        harness.press(Key::Tab, Modifiers::NONE);
        stops.push(focused_name(&harness.settle()));
        if stops.iter().any(|stop| stop == row) && stops.iter().any(|stop| stop == "New SQL query")
        {
            break;
        }
    }
    assert!(stops.iter().any(|stop| stop == row), "{stops:?}");
    assert!(
        stops.iter().any(|stop| stop == "New SQL query"),
        "{stops:?}"
    );
    while focused_name(&harness.settle()) != row {
        harness.press(Key::Tab, Modifiers::NONE);
    }
    harness.press(Key::Enter, Modifiers::NONE);
    assert!(harness.has("authors tab"));
    let id = harness.app.workspace(tab).unwrap().active_tab.unwrap();
    harness.app.apply(Action::CloseTab { tab, id });
    let mut seen = Vec::new();
    for _ in 0..8 {
        harness.press(Key::F6, Modifiers::NONE);
        seen.push(focused_name(&harness.settle()));
    }
    assert!(seen.iter().any(|stop| stop == row), "{seen:?}");
    assert!(seen.iter().any(|stop| stop == "Objects"), "{seen:?}");
    // The terminal look's pane.
    let (mut harness, tab) = bookshop(Look::omarchy());
    harness.press(Key::L, Modifiers::CTRL);
    assert_eq!(focused_name(&harness.settle()), "Start");
    harness.press(Key::H, Modifiers::CTRL);
    assert_eq!(focused_name(&harness.settle()), "Objects");
    let mut seen = Vec::new();
    for _ in 0..8 {
        harness.press(Key::F6, Modifiers::NONE);
        seen.push(focused_name(&harness.settle()));
    }
    assert!(seen.iter().any(|stop| stop == "Start"), "{seen:?}");
    assert_eq!(harness.app.workspace(tab).unwrap().tabs.len(), 0);
}

/// A SQL editor opened again from the screen is the editor it was, by
/// its number too, and no number is handed out for it.
#[test]
fn a_reopened_sql_editor_keeps_its_number() {
    let (mut harness, tab) = bookshop(Look::macos());
    for _ in 0..2 {
        harness.app.apply(Action::NewSqlTab(tab));
    }
    let id = harness.app.workspace(tab).unwrap().active_tab.unwrap();
    harness
        .app
        .workspace_mut(tab)
        .unwrap()
        .sql_tab_mut(id)
        .unwrap()
        .text = "SELECT 1".into();
    harness.app.apply(Action::CloseTab { tab, id });
    let first = harness.app.workspace(tab).unwrap().tabs[0].id();
    harness.app.apply(Action::CloseTab { tab, id: first });
    let next = harness.app.workspace(tab).unwrap().next_query;
    assert_eq!(rows(&mut harness), ["Query 2, SQL editor, opened Just now"]);
    harness.app.apply(Action::OpenRecent { tab, index: 0 });
    let workspace = harness.app.workspace(tab).unwrap();
    let sql = workspace.active_sql_tab().expect("the editor");
    assert_eq!((sql.number, &sql.text[..]), (2, "SELECT 1"));
    assert_eq!(workspace.next_query, next, "no number handed out");
    let id = sql.id;
    assert!(harness.has("Query 2 tab"));
    // Closed and opened again, it is still Query 2.
    harness.app.apply(Action::CloseTab { tab, id });
    harness.app.apply(Action::OpenRecent { tab, index: 0 });
    let workspace = harness.app.workspace(tab).unwrap();
    assert_eq!(workspace.active_sql_tab().unwrap().number, 2);
    assert_eq!(workspace.next_query, next);
}

/// A SQL editor opened again from the screen runs as it did: one opened
/// read-only (an agent's statement, or the person's choice) on a
/// writable connection stays read-only, and a read-write one read-write.
#[test]
fn a_reopened_sql_editor_keeps_its_mode() {
    crate::util::pin_now(Some(NOW));
    let mut harness = Harness::new();
    harness.set_look(Look::macos());
    let mut saved = fixture_connection();
    saved.read_only = Some(false);
    let tab = harness.connect_with(saved, bookshop_objects());
    for mode in [RunMode::ReadOnly, RunMode::ReadWrite] {
        harness.app.apply(Action::NewSqlTab(tab));
        let id = harness.app.workspace(tab).unwrap().active_tab.unwrap();
        let sql = harness
            .app
            .workspace_mut(tab)
            .unwrap()
            .sql_tab_mut(id)
            .unwrap();
        sql.text = "UPDATE book_covers SET kind = 'print'".into();
        sql.mode = mode;
        harness.app.apply(Action::CloseTab { tab, id });
        harness.app.apply(Action::OpenRecent { tab, index: 0 });
        let workspace = harness.app.workspace(tab).unwrap();
        let sql = workspace.active_sql_tab().expect("the editor");
        assert_eq!(sql.mode, mode, "{mode:?}");
        let id = sql.id;
        harness.app.apply(Action::CloseTab { tab, id });
    }
}

/// A listing that failed says nothing of what exists: the recents stay,
/// and nothing is written.
#[test]
fn a_failed_listing_keeps_the_recents() {
    crate::util::pin_now(Some(NOW));
    let mut harness = Harness::new();
    harness.set_look(Look::macos());
    let saved = fixture_connection();
    let database = crate::session::database_of(&saved.spec);
    let conn = saved.id.clone();
    for (name, opened) in [("authors", NOW - 100), ("book_covers", NOW - 200)] {
        let recent = Recent::of(
            &ObjectRef::new("main", name),
            ObjectKind::Table,
            &database,
            opened,
        );
        harness.app.recents.note(&conn.0, recent);
    }
    let file = harness.app.dirs.recents_file();
    let before = saves_of(&harness, &file);
    harness.app.connections.upsert(saved);
    let tab = harness.app.active_tab_id();
    harness.app.apply(Action::Connect {
        tab,
        conn: conn.clone(),
    });
    let Command::Connect {
        session, request, ..
    } = *crate::testing::last_sent(&harness.app)
    else {
        panic!("expected Connect");
    };
    harness.app.apply(Action::Backend(Event::Connected {
        session,
        request,
        driver: Driver::Sqlite,
        encrypted: false,
        access: crate::testing::asked_access(&harness.app),
    }));
    let Command::ListSchemas { request, .. } = *crate::testing::last_sent(&harness.app) else {
        panic!("expected ListSchemas");
    };
    harness.app.apply(Action::Backend(Event::Schemas {
        session,
        request,
        result: Ok(vec!["main".into()]),
    }));
    let Command::ListObjects { request, .. } = *crate::testing::last_sent(&harness.app) else {
        panic!("expected ListObjects");
    };
    harness.app.apply(Action::Backend(Event::Objects {
        session,
        request,
        schema: "main".into(),
        result: Err(tabletist_db::Error::query(
            "permission denied for schema main",
        )),
    }));
    assert_eq!(harness.app.recents.of(&conn.0, &database).len(), 2);
    assert_eq!(saves_of(&harness, &file), before, "nothing was written");
    assert_eq!(harness.app.start_recents(tab).len(), 2);
}

/// A connection deleted while its window is open is not written back by
/// an open in that window.
#[test]
fn a_deleted_connection_is_not_written_back_by_an_open() {
    let (mut harness, tab) = bookshop(Look::macos());
    let conn = harness.app.workspace(tab).unwrap().conn_id.clone();
    visit(&mut harness, tab, "authors");
    harness.app.apply(Action::DeleteConnection(conn.clone()));
    let file = harness.app.dirs.recents_file();
    let saves = saves_of(&harness, &file);
    visit(&mut harness, tab, "publishers");
    let database = crate::session::database_of(&harness.app.workspace(tab).unwrap().spec);
    assert!(harness.app.recents.of(&conn.0, &database).is_empty());
    assert_eq!(saves_of(&harness, &file), saves);
    assert!(harness.app.start_recents(tab).is_empty());
}
