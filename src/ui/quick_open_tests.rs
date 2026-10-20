//! Quick open: opening and closing it from every part of a window, what
//! it lists with nothing typed and with a query, how it ranks, the scope
//! of every open connection, what opening a result does, the Omarchy
//! picker's digits, and how quickly a large schema answers. The Bookshop
//! demo data only.

use egui::accesskit::Role;
use egui::{Key, Modifiers};
use tabletist_db::{ObjectInfo, ObjectKind, ObjectRef};

use crate::app::{
    About, GROUP_SHOWN, QuickCommand, QuickHeading, QuickItem, QuickRow, Runnable, StartRow,
};
use crate::backend::Command;
use crate::connections::SavedConnection;
use crate::env::Environment;
use crate::model::{Action, ConnTabId, Dialog, QuickGroup, QuickOpen};
use crate::search::Tier;
use crate::testing::{Harness, bookshop_objects, fixture_connection};
use crate::theme::Look;

/// The Bookshop's development connection.
fn bookshop_dev() -> SavedConnection {
    let mut saved = fixture_connection();
    saved.name = "Bookshop".into();
    saved
}

/// The Bookshop's production connection, on its own file.
fn bookshop_production() -> SavedConnection {
    let mut saved = fixture_connection();
    saved.name = "Bookshop".into();
    saved.environment = Environment::Production;
    saved.spec = tabletist_db::ConnectSpec::sqlite("/tmp/bookshop_production.db");
    saved
}

/// The Bookshop's objects with the publishers' tables and views of the
/// boards: six tables and two views whose names start `pub`.
fn bookshop_with_publishers() -> Vec<ObjectInfo> {
    let table = |name: &str, rows: u64| ObjectInfo {
        name: name.into(),
        kind: ObjectKind::Table,
        estimated_rows: Some(rows),
    };
    let view = |name: &str| ObjectInfo {
        name: name.into(),
        kind: ObjectKind::View,
        estimated_rows: None,
    };
    let mut objects: Vec<ObjectInfo> = bookshop_objects()
        .into_iter()
        .filter(|info| info.name != "publishers")
        .collect();
    objects.extend([
        table("publishers", 6),
        table("publishers_members", 6_100),
        table("publishers_settings", 6),
        table("publishers_contacts", 14),
        table("publishers_imprints", 22),
        table("publishers_regions", 4),
        view("publishers_overview"),
        view("publishers_billing_status"),
    ]);
    objects
}

/// A Bookshop development window in `look`, its start screen showing.
fn bookshop(look: Look) -> (Harness, ConnTabId) {
    let mut harness = Harness::new();
    harness.set_look(look);
    let tab = harness.connect_with(bookshop_dev(), bookshop_with_publishers());
    (harness, tab)
}

/// Opens `name` of the Bookshop in `tab`, its first page read.
fn open_table(harness: &mut Harness, tab: ConnTabId, name: &str) {
    harness.app.apply(Action::OpenObject {
        tab,
        object: ObjectRef::new("main", name),
        kind: ObjectKind::Table,
        pin: true,
    });
    harness.answer_rows(crate::testing::page(3, false));
}

/// The open quick open.
fn quick_open(harness: &Harness) -> &QuickOpen {
    match &harness.app.dialog {
        Some(Dialog::QuickOpen(open)) => open,
        _ => panic!("quick open is not open"),
    }
}

/// Types `text` into what has the keyboard, one frame.
fn type_text(harness: &mut Harness, text: &str) {
    harness.frame(vec![egui::Event::Text(text.into())]);
    harness.settle();
}

/// Opens quick open with its key in `look`, and types `query`.
fn ask(harness: &mut Harness, look: Look, query: &str) {
    harness.press(Key::P, chord(look));
    assert!(
        matches!(harness.app.dialog, Some(Dialog::QuickOpen(_))),
        "{}",
        look.name
    );
    if !query.is_empty() {
        type_text(harness, query);
    }
}

/// Quick open's key, held with P: ⌘P on macOS, ctrl+p on Omarchy.
fn chord(look: Look) -> Modifiers {
    if look.terminal {
        Modifiers::CTRL
    } else {
        Modifiers::COMMAND
    }
}

/// The names of the listed results, and of the headings, in order.
fn listed(harness: &Harness) -> Vec<String> {
    let list = harness.app.quick_open_list(quick_open(harness));
    list.rows
        .iter()
        .map(|row| match row {
            QuickRow::Heading(heading) => format!("# {heading:?}"),
            QuickRow::Item(QuickItem::Object { found, .. }) => found.object.name.clone(),
            QuickRow::Item(QuickItem::Start { row, .. }) => match row {
                StartRow::Recent(crate::model::StartRecent::Object { object, .. })
                | StartRow::Largest(object, _) => object.name.clone(),
                StartRow::Recent(crate::model::StartRecent::Sql { number, .. }) => {
                    format!("Query {number}")
                }
            },
            QuickRow::Item(QuickItem::Command { command, .. }) => {
                format!("> {}", harness.app.command_name(command))
            }
            QuickRow::Item(QuickItem::More { hidden, .. }) => format!("+{hidden}"),
        })
        .collect()
}

/// The name a screen reader would read for what has the keyboard.
fn focused(harness: &mut Harness) -> String {
    super::tests::focused_name(&harness.settle())
}

/// Steps with F6 until the keyboard is on `name`.
fn reach(harness: &mut Harness, name: &str) {
    let mut seen = vec![focused(harness)];
    while seen.last().is_none_or(|last| last != name) {
        assert!(seen.len() <= 12, "{name} not reached: {seen:?}");
        harness.press(Key::F6, Modifiers::NONE);
        seen.push(focused(harness));
    }
}

/// A click at `at`: the pointer moves there, presses and lets go.
fn click_at(harness: &mut Harness, at: egui::Pos2) {
    harness.frame(vec![egui::Event::PointerMoved(at)]);
    let button = |pressed| egui::Event::PointerButton {
        pos: at,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: Modifiers::NONE,
    };
    harness.frame(vec![button(true)]);
    harness.frame(vec![button(false)]);
    harness.settle();
}

/// Test 1: the key opens quick open from the rows, the SQL editor, the
/// start screen and the Agents tab, in both looks; Esc and a click
/// outside close it.
#[test]
fn the_key_opens_quick_open_from_every_part_of_a_window() {
    for look in [Look::macos(), Look::omarchy()] {
        for place in ["start", "rows", "sql", "agents"] {
            let (mut harness, tab) = bookshop(look);
            match place {
                "rows" => open_table(&mut harness, tab, "authors"),
                "sql" => {
                    harness.app.apply(Action::NewSqlTab(tab));
                    harness.settle();
                }
                "agents" => {
                    harness.app.apply(Action::ShowAgents(tab));
                    harness.settle();
                }
                _ => {}
            }
            harness.settle();
            ask(&mut harness, look, "");
            harness.press(Key::Escape, Modifiers::NONE);
            assert!(harness.app.dialog.is_none(), "{} {place}", look.name);
            // A click outside the panel closes it too.
            ask(&mut harness, look, "");
            click_at(&mut harness, egui::pos2(40.0, 760.0));
            assert!(harness.app.dialog.is_none(), "{} {place}", look.name);
        }
    }
}

/// Test 1: what had the keyboard has it again, after Esc and after a
/// click outside.
#[test]
fn closing_quick_open_gives_the_keyboard_back_to_what_had_it() {
    for look in [Look::macos(), Look::omarchy()] {
        let (mut harness, tab) = bookshop(look);
        open_table(&mut harness, tab, "authors");
        reach(&mut harness, "Rows");
        ask(&mut harness, look, "");
        harness.press(Key::Escape, Modifiers::NONE);
        assert_eq!(focused(&mut harness), "Rows", "{}", look.name);
        reach(&mut harness, "Objects");
        ask(&mut harness, look, "pub");
        click_at(&mut harness, egui::pos2(40.0, 760.0));
        assert!(harness.app.dialog.is_none());
        assert_eq!(focused(&mut harness), "Objects", "{}", look.name);
    }
}

/// The key again while quick open is open keeps the one panel, and
/// selects what its field holds: what is typed next replaces it.
#[test]
fn the_key_again_selects_what_is_typed() {
    let (mut harness, _tab) = bookshop(Look::macos());
    ask(&mut harness, Look::macos(), "pub");
    harness.press(Key::P, Modifiers::COMMAND);
    harness.settle();
    assert_eq!(quick_open(&harness).query, "pub");
    type_text(&mut harness, "cov");
    assert_eq!(quick_open(&harness).query, "cov");
}

/// Test 2: with nothing typed, the recents are the start screen's list,
/// then (macOS) the commands.
#[test]
fn with_nothing_typed_the_recents_are_the_start_screens() {
    for look in [Look::macos(), Look::omarchy()] {
        let (mut harness, tab) = bookshop(look);
        // Six opened: the start screen lists the newest five.
        for name in [
            "authors",
            "editions",
            "book_covers",
            "publishers",
            "ratings",
            "reviews",
        ] {
            open_table(&mut harness, tab, name);
        }
        ask(&mut harness, look, "");
        let open = quick_open(&harness);
        let list = harness.app.quick_open_list(open);
        let recents: Vec<StartRow> = list
            .items()
            .filter_map(|item| match item {
                QuickItem::Start { row, .. } => Some(row.clone()),
                _ => None,
            })
            .collect();
        let start: Vec<StartRow> = harness
            .app
            .start_recents(tab)
            .into_iter()
            .map(StartRow::Recent)
            .collect();
        assert_eq!(recents, start, "{}", look.name);
        assert_eq!(recents.len(), crate::recents::SHOWN);
        assert_eq!(list.rows[0], QuickRow::Heading(QuickHeading::Recent));
        let commands: Vec<String> = listed(&harness)
            .into_iter()
            .filter(|name| name.starts_with("> "))
            .collect();
        if look.terminal {
            // Omarchy's commands stay on `:`.
            assert!(commands.is_empty());
        } else {
            assert_eq!(commands[0], "> New SQL query");
        }
    }
}

/// Test 3: `pub` lists the publishers' tables, three and a "more" row,
/// then the views, each under its heading; the "more" row shows the rest
/// in place.
#[test]
fn a_query_lists_tables_then_views_three_of_each_and_more() {
    let (mut harness, _tab) = bookshop(Look::macos());
    ask(&mut harness, Look::macos(), "pub");
    assert_eq!(
        listed(&harness),
        [
            "# Tables",
            "publishers",
            "publishers_members",
            "publishers_regions",
            "+3",
            "# Views",
            "publishers_overview",
            "publishers_billing_status",
        ]
    );
    // The "more" row is the fourth result: Enter on it shows the group.
    for _ in 0..3 {
        harness.press(Key::ArrowDown, Modifiers::NONE);
    }
    harness.press(Key::Enter, Modifiers::NONE);
    assert_eq!(quick_open(&harness).expanded, [QuickGroup::Tables]);
    let names = listed(&harness);
    assert_eq!(names.len(), 10, "{names:?}");
    assert!(!names.contains(&"+3".to_owned()));
    // What it hid is picked now: Enter opens it.
    assert_eq!(quick_open(&harness).selected, 3);
    // Typing again folds it.
    type_text(&mut harness, "l");
    assert!(quick_open(&harness).expanded.is_empty());
}

/// Test 3, Omarchy: one numbered list of every match, the kind in a
/// column, no headings and no commands.
#[test]
fn omarchy_lists_every_match_in_one_list() {
    let (mut harness, _tab) = bookshop(Look::omarchy());
    ask(&mut harness, Look::omarchy(), "pub");
    let names = listed(&harness);
    assert_eq!(names.len(), 8, "{names:?}");
    assert!(names.iter().all(|name| !name.starts_with('#')));
    // The count stands at the prompt's right: eight shown, none hidden.
    assert!(harness.painted_color("8 matches").is_some());
    type_text(&mut harness, "l");
    assert!(harness.painted_color("8 matches").is_some());
}

/// Test 4: exact names, then prefixes, then word starts, then any run;
/// `bc` finds `book_covers`; a recent object ranks first in its tier.
#[test]
fn matches_rank_by_tier_then_by_recency() {
    let (mut harness, tab) = bookshop(Look::macos());
    let found = |harness: &Harness, query: &str| -> Vec<(String, Tier)> {
        harness
            .app
            .search_objects(tab, query)
            .into_iter()
            .map(|found| (found.object.name.clone(), found.tier))
            .collect()
    };
    assert_eq!(
        found(&harness, "covers"),
        [
            ("covers".to_owned(), Tier::Exact),
            ("book_covers".to_owned(), Tier::WordStarts),
            ("missing_covers".to_owned(), Tier::WordStarts),
        ]
    );
    assert_eq!(
        found(&harness, "bc"),
        [("book_covers".to_owned(), Tier::WordStarts)]
    );
    assert_eq!(found(&harness, "vers")[0].1, Tier::Substring);
    // Shorter names first within a tier, then by name, until one of
    // them is opened.
    let names: Vec<String> = found(&harness, "publishers_")
        .into_iter()
        .map(|(name, _)| name)
        .collect();
    assert_eq!(names[..2], ["publishers_members", "publishers_regions"]);
    open_table(&mut harness, tab, "publishers_settings");
    let names: Vec<String> = found(&harness, "publishers_")
        .into_iter()
        .map(|(name, _)| name)
        .collect();
    assert_eq!(names[0], "publishers_settings");
    // The exact name still comes first.
    assert_eq!(found(&harness, "publishers")[0].0, "publishers");
}

/// Test 5: typing asks the database nothing: the results come from the
/// listed schema.
#[test]
fn typing_queries_nothing() {
    for look in [Look::macos(), Look::omarchy()] {
        let (mut harness, _tab) = bookshop(look);
        ask(&mut harness, look, "");
        let sent = harness.app.backend.sent.len();
        for letter in ["p", "u", "b", "l"] {
            type_text(&mut harness, letter);
        }
        harness.press(Key::Tab, Modifiers::NONE);
        type_text(&mut harness, "i");
        let asked: Vec<&Command> = harness.app.backend.sent[sent..]
            .iter()
            .filter(|command| {
                matches!(
                    command,
                    Command::ListSchemas { .. }
                        | Command::ListObjects { .. }
                        | Command::Describe { .. }
                        | Command::FetchRows { .. }
                        | Command::CountRows { .. }
                        | Command::RunSql { .. }
                )
            })
            .collect();
        assert!(asked.is_empty(), "{}: {asked:?}", look.name);
        assert_eq!(quick_open(&harness).query, "publi");
    }
}

/// Test 7: Tab searches every open connection, grouped by connection;
/// opening a production result brings its window to the front with the
/// table open; the next open searches this connection again.
#[test]
fn tab_searches_every_open_connection() {
    for look in [Look::macos(), Look::omarchy()] {
        let (mut harness, dev) = bookshop(look);
        harness.app.apply(Action::ShowConnections);
        let production = harness.connect_with(bookshop_production(), bookshop_objects());
        harness.app.apply(Action::ActivateConnTab(dev));
        harness.settle();
        ask(&mut harness, look, "cov");
        harness.press(Key::Tab, Modifiers::NONE);
        assert!(quick_open(&harness).everywhere, "{}", look.name);
        let names = listed(&harness);
        let headings: Vec<&String> = names.iter().filter(|name| name.starts_with('#')).collect();
        assert_eq!(
            headings,
            [
                &format!("# {:?}", QuickHeading::Connection(dev)),
                &format!("# {:?}", QuickHeading::Connection(production)),
            ],
            "{}",
            look.name
        );
        // The production window's `covers`: the fourth result, after
        // this window's three.
        let list = harness.app.quick_open_list(quick_open(&harness));
        let index = list
            .items()
            .position(|item| {
                matches!(item, QuickItem::Object { tab, found }
                    if *tab == production && found.object.name == "covers")
            })
            .expect("production's covers");
        harness.app.apply(Action::QuickOpenPick {
            index: Some(index),
            in_editor: false,
        });
        harness.settle();
        assert_eq!(harness.app.active_tab_id(), production, "{}", look.name);
        let workspace = harness.app.workspace(production).unwrap();
        let object = workspace.object_tab(workspace.active_tab.unwrap()).unwrap();
        assert_eq!(object.object.name, "covers");
        // Tab again searches this connection; and each open starts there.
        ask(&mut harness, look, "cov");
        assert!(!quick_open(&harness).everywhere);
        harness.press(Key::Tab, Modifiers::NONE);
        harness.press(Key::Tab, Modifiers::NONE);
        assert!(!quick_open(&harness).everywhere);
    }
}

/// Test 7, macOS: the chip says what is searched, and on production it is
/// the red pill.
#[test]
fn the_chip_names_what_is_searched() {
    let (mut harness, _dev) = bookshop(Look::macos());
    ask(&mut harness, Look::macos(), "");
    assert!(harness.has("Bookshop · dev"));
    harness.press(Key::Tab, Modifiers::NONE);
    assert!(harness.has("All open connections"));
    harness.press(Key::Escape, Modifiers::NONE);
    let mut harness = Harness::new();
    harness.set_look(Look::macos());
    harness.connect_with(bookshop_production(), bookshop_objects());
    ask(&mut harness, Look::macos(), "");
    let pill = |(text, color): &(String, egui::Color32)| {
        text == "production" && *color == egui::Color32::WHITE
    };
    assert!(
        harness.painted.iter().any(pill),
        "the pill's words on the danger red"
    );
}

/// Nothing matches: the panel says where it looked and offers every open
/// connection; there it offers nothing more.
#[test]
fn no_match_says_where_it_looked() {
    let (mut harness, _tab) = bookshop(Look::macos());
    ask(&mut harness, Look::macos(), "zzz");
    assert!(harness.has("No objects match “zzz” in Bookshop · dev"));
    // The link names the key that does the same.
    let key = harness.app.keymap.key_in(
        harness.app.layout(),
        crate::keymap::Command::QuickOpenScope,
        crate::keymap::Scope::Prompt,
    );
    let link = format!("Search all open connections {}", key.as_str());
    harness.click(&link);
    assert!(quick_open(&harness).everywhere);
    harness.settle();
    assert!(harness.has("No objects match “zzz” in open connections"));
    assert!(!harness.has(&link));
}

/// Test 8: Enter opens a result as a tab, or shows the tab it has; ⌥↩
/// opens a SQL editor reading its first rows, with the Editor's row
/// limit.
#[test]
fn enter_opens_or_shows_a_tab_and_option_enter_opens_an_editor() {
    let (mut harness, tab) = bookshop(Look::macos());
    ask(&mut harness, Look::macos(), "authors");
    harness.press(Key::Enter, Modifiers::NONE);
    assert!(harness.app.dialog.is_none());
    open_table(&mut harness, tab, "editions");
    let tabs = harness.app.workspace(tab).unwrap().tabs.len();
    ask(&mut harness, Look::macos(), "authors");
    harness.press(Key::Enter, Modifiers::NONE);
    let workspace = harness.app.workspace(tab).unwrap();
    assert_eq!(workspace.tabs.len(), tabs, "the tab it had is shown");
    let object = workspace.object_tab(workspace.active_tab.unwrap()).unwrap();
    assert_eq!(object.object.name, "authors");
    harness.app.settings.sql_limit = 250;
    ask(&mut harness, Look::macos(), "editions");
    harness.press(Key::Enter, Modifiers::ALT);
    let workspace = harness.app.workspace(tab).unwrap();
    let sql = workspace.active_sql_tab().expect("a SQL editor");
    let dialect = workspace.driver.dialect();
    let expected = format!(
        "SELECT * FROM {}.{} LIMIT 250",
        dialect.ident("main"),
        dialect.ident("editions")
    );
    assert_eq!(sql.text, expected);
}

/// The macOS commands with nothing typed, and what they do; on production
/// New SQL query says its editor opens read-only.
#[test]
fn the_commands_do_what_they_say() {
    let (mut harness, tab) = bookshop(Look::macos());
    ask(&mut harness, Look::macos(), "new sql");
    let list = harness.app.quick_open_list(quick_open(&harness));
    let first = list.items().next().cloned();
    assert!(
        matches!(
            &first,
            Some(QuickItem::Command {
                command: QuickCommand::Run(Runnable {
                    command: crate::keymap::Command::NewSqlTab,
                    about: About::Nothing,
                    ..
                }),
                ..
            })
        ),
        "{first:?}"
    );
    harness.press(Key::Enter, Modifiers::NONE);
    assert!(harness.app.dialog.is_none());
    assert!(
        harness
            .app
            .workspace(tab)
            .unwrap()
            .active_sql_tab()
            .is_some()
    );
    // One database: no Switch database to offer.
    ask(&mut harness, Look::macos(), "switch");
    assert!(!listed(&harness).contains(&"> Switch database".to_owned()));
    harness.press(Key::Escape, Modifiers::NONE);
    let mut harness = Harness::new();
    harness.set_look(Look::macos());
    harness.connect_with(bookshop_production(), bookshop_objects());
    ask(&mut harness, Look::macos(), "");
    harness.settle();
    assert!(harness.painted_color("opens read-only").is_some());
}

/// The Agents command is offered only where agents can reach the app.
#[test]
fn the_agents_command_is_offered_where_agents_can_reach_the_app() {
    let (mut harness, _tab) = bookshop(Look::macos());
    ask(&mut harness, Look::macos(), "");
    let offered = listed(&harness).contains(&"> Agents".to_owned());
    assert_eq!(offered, crate::endpoint::AVAILABLE);
}

/// Opening a result adds it to the connection's recents.
#[test]
fn opening_a_result_makes_it_recent() {
    let (mut harness, tab) = bookshop(Look::macos());
    ask(&mut harness, Look::macos(), "ratings");
    harness.press(Key::Enter, Modifiers::NONE);
    let recents = harness.app.start_recents(tab);
    assert!(matches!(
        recents.first(),
        Some(crate::model::StartRecent::Object { object, .. }) if object.name == "ratings"
    ));
}

/// Test 10: Omarchy's digits open a numbered recent while nothing is
/// typed; once anything is, they are typed, and so is `>`; `:` still
/// opens the command line once quick open is closed.
#[test]
fn omarchy_digits_pick_only_while_nothing_is_typed() {
    let look = Look::omarchy();
    let (mut harness, tab) = bookshop(look);
    for name in ["authors", "editions"] {
        open_table(&mut harness, tab, name);
    }
    ask(&mut harness, look, "");
    // `2` is the second recent: authors, opened before editions.
    harness.frame(vec![
        crate::testing::key(Key::Num2, Modifiers::NONE),
        egui::Event::Text("2".into()),
    ]);
    harness.frame(vec![crate::testing::release(Key::Num2, Modifiers::NONE)]);
    harness.settle();
    assert!(harness.app.dialog.is_none());
    let workspace = harness.app.workspace(tab).unwrap();
    let object = workspace.object_tab(workspace.active_tab.unwrap()).unwrap();
    assert_eq!(object.object.name, "authors");
    // Typed after a letter, a digit is the query's.
    ask(&mut harness, look, "a");
    harness.frame(vec![
        crate::testing::key(Key::Num2, Modifiers::NONE),
        egui::Event::Text("2".into()),
    ]);
    harness.settle();
    type_text(&mut harness, ">");
    assert_eq!(quick_open(&harness).query, "a2>");
    harness.press(Key::Escape, Modifiers::NONE);
    assert!(harness.app.dialog.is_none());
    super::tests::type_key(&mut harness, Key::Colon, ":");
    assert!(harness.app.workspace(tab).unwrap().command.is_some());
}

/// Omarchy's ctrl+n and ctrl+p move the pick. (The SQL editor's open
/// completion list keeps its ctrl+p: `ui::complete_tests`.)
#[test]
fn omarchy_ctrl_n_and_ctrl_p_move_the_pick() {
    let look = Look::omarchy();
    let (mut harness, _tab) = bookshop(look);
    ask(&mut harness, look, "pub");
    harness.press(Key::N, Modifiers::CTRL);
    harness.press(Key::N, Modifiers::CTRL);
    assert_eq!(quick_open(&harness).selected, 2);
    harness.press(Key::P, Modifiers::CTRL);
    assert_eq!(quick_open(&harness).selected, 1);
    assert!(matches!(harness.app.dialog, Some(Dialog::QuickOpen(_))));
}

/// Test 11: the macOS sidebar's search field opens quick open.
#[test]
fn the_sidebar_field_opens_quick_open() {
    for look in [Look::macos(), Look::standard()] {
        let (mut harness, _tab) = bookshop(look);
        harness.settle();
        harness.click("Find any object…");
        assert!(
            matches!(harness.app.dialog, Some(Dialog::QuickOpen(_))),
            "{}",
            look.name
        );
        harness.press(Key::Escape, Modifiers::NONE);
        // With the keyboard on it, Enter opens quick open, and what is
        // typed goes into quick open's field.
        reach(&mut harness, "Find any object…");
        harness.press(Key::Enter, Modifiers::NONE);
        assert!(matches!(harness.app.dialog, Some(Dialog::QuickOpen(_))));
        harness.press(Key::Escape, Modifiers::NONE);
        reach(&mut harness, "Find any object…");
        type_text(&mut harness, "pu");
        type_text(&mut harness, "b");
        assert_eq!(quick_open(&harness).query, "pub", "{}", look.name);
    }
    // The terminal look's `/` line filters the tree, as it did.
    let (mut harness, tab) = bookshop(Look::omarchy());
    harness.settle();
    assert!(!harness.has("Find any object…"));
    reach(&mut harness, "Objects");
    super::tests::type_key(&mut harness, Key::Slash, "/");
    type_text(&mut harness, "pub");
    assert!(harness.app.dialog.is_none());
    assert_eq!(harness.app.workspace(tab).unwrap().tree.filter, "pub");
}

/// The panel is a dialog named for the look; the field is a combobox
/// that controls the list; the picked result is said with its kind, its
/// schema and its estimate; headings are labels, not options.
#[test]
fn a_screen_reader_reads_the_panel() {
    for (look, name) in [
        (Look::macos(), "Quick open"),
        (Look::omarchy(), "Find object"),
    ] {
        let (mut harness, _tab) = bookshop(look);
        ask(&mut harness, look, "pub");
        let tree = harness.settle();
        assert!(
            crate::testing::node(&tree, name, Role::Dialog).is_some(),
            "{}",
            look.name
        );
        let combo = tree
            .nodes
            .iter()
            .find(|(_, node)| node.role() == Role::EditableComboBox)
            .expect("the field");
        assert_eq!(combo.1.controls().len(), 1);
        let estimate = if look.terminal { "≈ 6" } else { "≈ 6 rows" };
        let picked = format!("publishers, table, main, {estimate}");
        let option = tree
            .nodes
            .iter()
            .find(|(_, node)| {
                node.role() == Role::ListBoxOption && node.is_selected() == Some(true)
            })
            .expect("a picked option");
        assert_eq!(option.1.label(), Some(picked.as_str()), "{}", look.name);
        assert_eq!(combo.1.active_descendant(), Some(option.0));
        if !look.terminal {
            assert!(crate::testing::node(&tree, "Tables", Role::Label).is_some());
            assert!(crate::testing::node(&tree, "Tables", Role::ListBoxOption).is_none());
        }
    }
}

/// A long name ends in "…" and shows in full on hover.
#[test]
fn a_long_name_is_cut_and_shown_in_full_on_hover() {
    let long =
        "publishers_with_an_uncommonly_long_name_that_runs_well_past_the_right_edge_of_quick_open";
    let mut objects = bookshop_objects();
    objects.push(ObjectInfo {
        name: long.into(),
        kind: ObjectKind::Table,
        estimated_rows: None,
    });
    let mut harness = Harness::new();
    harness.set_look(Look::macos());
    harness.connect_with(bookshop_dev(), objects);
    ask(&mut harness, Look::macos(), "publishers_with");
    let tree = harness.settle();
    let label = format!("{long}, table, main");
    let row = crate::testing::bounds(&tree, &label, Role::ListBoxOption).expect("the row");
    let shown = |tree: &egui::accesskit::TreeUpdate| {
        crate::testing::labels(tree)
            .iter()
            .any(|label| label == long)
    };
    assert!(!shown(&tree), "no tooltip before the pointer comes");
    harness.frame(vec![egui::Event::PointerMoved(row.center())]);
    // Past the tooltip's delay: each frame is 1/60 s.
    for _ in 0..60 {
        harness.frame(Vec::new());
    }
    let tree = harness.frame(Vec::new());
    assert!(shown(&tree), "the name in full on hover");
}

/// The list keeps a wheel scroll: it scrolls only when the pick moves.
#[test]
fn the_list_keeps_a_wheel_scroll() {
    let mut objects = bookshop_objects();
    objects.extend((0..60).map(|i| ObjectInfo {
        name: format!("reviews_{i:03}"),
        kind: ObjectKind::Table,
        estimated_rows: None,
    }));
    let mut harness = Harness::new();
    harness.set_look(Look::omarchy());
    harness.connect_with(bookshop_dev(), objects);
    ask(&mut harness, Look::omarchy(), "reviews_");
    let tree = harness.settle();
    let row = crate::testing::bounds(&tree, "reviews_000, table, main", Role::ListBoxOption)
        .expect("a result");
    harness.frame(vec![egui::Event::PointerMoved(row.center())]);
    harness.frame(vec![
        egui::Event::PointerMoved(row.center()),
        egui::Event::MouseWheel {
            unit: egui::MouseWheelUnit::Point,
            delta: egui::vec2(0.0, -200.0),
            modifiers: Modifiers::NONE,
            phase: egui::TouchPhase::Move,
        },
    ]);
    for _ in 0..120 {
        harness.frame(vec![]);
    }
    let offset = quick_open(&harness).scroll_offset;
    assert!(offset > 50.0, "snapped back to {offset}");
}

/// Spaces alone are nothing typed: the digits still open a recent.
#[test]
fn omarchy_digits_pick_after_spaces_alone() {
    let look = Look::omarchy();
    let (mut harness, tab) = bookshop(look);
    for name in ["authors", "editions"] {
        open_table(&mut harness, tab, name);
    }
    ask(&mut harness, look, " ");
    harness.frame(vec![
        crate::testing::key(Key::Num2, Modifiers::NONE),
        egui::Event::Text("2".into()),
    ]);
    harness.settle();
    assert!(harness.app.dialog.is_none());
    let workspace = harness.app.workspace(tab).unwrap();
    let object = workspace.object_tab(workspace.active_tab.unwrap()).unwrap();
    assert_eq!(object.object.name, "authors");
}

/// A dot alone names no schema: it matches nothing.
#[test]
fn a_dot_alone_matches_nothing() {
    let (harness, tab) = bookshop(Look::macos());
    assert!(harness.app.search_objects(tab, ".").is_empty());
    assert!(!harness.app.search_objects(tab, "main.").is_empty());
}

/// The pick moves from where it shows, after the list shrank under it.
#[test]
fn the_pick_moves_from_where_it_shows() {
    let (mut harness, _tab) = bookshop(Look::macos());
    ask(&mut harness, Look::macos(), "cov");
    // A pick held by no result, past the list's end.
    if let Some(Dialog::QuickOpen(open)) = &mut harness.app.dialog {
        open.selected = 7;
        open.picked = None;
    }
    let count = harness
        .app
        .quick_open_list(quick_open(&harness))
        .items()
        .count();
    assert!(count < 7);
    harness.press(Key::ArrowUp, Modifiers::NONE);
    assert_eq!(quick_open(&harness).selected, count - 2);
}

/// Quick open goes when the window it was opened in closes.
#[test]
fn quick_open_closes_with_its_window() {
    let (mut harness, dev) = bookshop(Look::macos());
    harness.app.apply(Action::ShowConnections);
    harness.connect_with(bookshop_production(), bookshop_objects());
    harness.app.apply(Action::ActivateConnTab(dev));
    ask(&mut harness, Look::macos(), "cov");
    harness.app.apply(Action::CloseConnTab(dev));
    harness.settle();
    assert!(harness.app.dialog.is_none());
}

/// Omarchy's count of what the list hides takes the connections'
/// headings into account: they take lines too.
#[test]
fn omarchy_counts_what_its_headings_push_out_of_view() {
    let look = Look::omarchy();
    let (mut harness, dev) = bookshop(look);
    harness.app.apply(Action::ShowConnections);
    harness.connect_with(bookshop_production(), bookshop_with_publishers());
    harness.app.apply(Action::ActivateConnTab(dev));
    harness.settle();
    ask(&mut harness, look, "pub");
    harness.press(Key::Tab, Modifiers::NONE);
    harness.settle();
    // Sixteen matches under two headings: the eight lines show a heading
    // and seven of them, and nine are hidden.
    assert!(
        harness.painted_color("16 matches · 9 hidden").is_some(),
        "{:?}",
        harness
            .painted
            .iter()
            .filter(|(text, _)| text.contains("matches"))
            .collect::<Vec<_>>()
    );
}

/// Test 12: with 10,000 objects in a schema, each keystroke's results are
/// worked out within 16 ms. A wall clock in a debug build on a shared CI
/// machine many times slower than a desk's cannot hold a fixed budget, so
/// the machine is measured too: each try also times taking the same names
/// apart for matching once (what the index saves every keystroke), and a
/// keystroke fails only where it is both over 16 ms and slower than that.
/// On a desk a keystroke takes 1.4 to 4.3 ms, and taking the names apart
/// about 9.5.
#[test]
fn a_large_schema_answers_each_keystroke_within_a_frame() {
    let words = [
        "book",
        "cover",
        "edition",
        "author",
        "review",
        "rating",
        "publisher",
    ];
    let names: Vec<String> = (0..10_000)
        .map(|i| {
            format!(
                "{}_{}_{i:05}",
                words[i % words.len()],
                words[(i / 7) % words.len()]
            )
        })
        .collect();
    let objects: Vec<ObjectInfo> = (0..10_000)
        .map(|i| ObjectInfo {
            name: format!(
                "{}_{}_{i:05}",
                words[i % words.len()],
                words[(i / 7) % words.len()]
            ),
            kind: if i % 5 == 0 {
                ObjectKind::View
            } else {
                ObjectKind::Table
            },
            estimated_rows: Some(i as u64),
        })
        .collect();
    let mut harness = Harness::new();
    harness.set_look(Look::macos());
    let tab = harness.connect_with(bookshop_dev(), objects);
    let mut open = QuickOpen {
        tab,
        query: String::new(),
        selected: 0,
        everywhere: false,
        expanded: Vec::new(),
        caret: None,
        scrolled_to: None,
        scroll_offset: 0.0,
        picked: None,
    };
    // Each keystroke of a query, and taking the names apart beside it, the
    // fastest of five tries of each taken as its time, so that a pause of
    // the machine's is neither's.
    let _ = harness.app.quick_open_list(&open);
    for typed in [
        "p", "pu", "pub", "pub_", "pub_c", "bc", "zz", "e", "o", "main.e",
    ] {
        open.query = typed.into();
        let (mut best, mut pass) = (std::time::Duration::MAX, std::time::Duration::MAX);
        for _ in 0..5 {
            let started = std::time::Instant::now();
            let apart: Vec<crate::search::Name> = names
                .iter()
                .map(|name| crate::search::Name::new(name))
                .collect();
            std::hint::black_box(apart);
            pass = pass.min(started.elapsed());
            let started = std::time::Instant::now();
            std::hint::black_box(harness.app.quick_open_list(&open));
            best = best.min(started.elapsed());
        }
        assert!(
            best < std::time::Duration::from_millis(16) || best < pass,
            "{typed}: {best:?}, taking the names apart {pass:?}"
        );
    }
}

/// The index among quick open's results of the one listed as `name`.
fn result_named(harness: &Harness, name: &str) -> usize {
    let list = harness.app.quick_open_list(quick_open(harness));
    let names: Vec<String> = listed(harness)
        .into_iter()
        .filter(|name| !name.starts_with("# "))
        .collect();
    assert_eq!(names.len(), list.items().count());
    names
        .iter()
        .position(|listed| listed == name)
        .unwrap_or_else(|| panic!("{name} is not listed: {names:?}"))
}

/// Test 6: `>` lists the commands alone, and hides those that do not apply
/// now.
#[test]
fn the_prefix_lists_the_commands_that_apply() {
    let look = Look::macos();
    let (mut harness, tab) = bookshop(look);
    ask(&mut harness, look, ">");
    let shown = listed(&harness);
    assert_eq!(shown[0], "# Commands");
    assert!(
        shown[1..].iter().all(|name| name.starts_with("> ")),
        "{shown:?}"
    );
    for offered in [
        "> New SQL query",
        "> Reload schema",
        "> Edit connection",
        "> Hide sidebar",
        "> Settings",
        "> Larger text",
    ] {
        assert!(shown.contains(&offered.to_owned()), "{offered}: {shown:?}");
    }
    // The start screen of the one window: no tab to close or filter,
    // nothing to save or run, no other window, the text at its own size;
    // and no setting's row until a word is typed.
    for hidden in [
        "> Close tab",
        "> Next tab",
        "> Filter rows",
        "> Save changes",
        "> Run all",
        "> Next connection",
        "> Default text size",
        "> Cancel running query",
    ] {
        assert!(!shown.contains(&hidden.to_owned()), "{hidden}: {shown:?}");
    }
    assert!(!shown.iter().any(|name| name.starts_with("> Settings ›")));
    // A row writes a key the window reads, as a screen reader hears it:
    // New SQL query's; none for Edit connection and New connection, whose
    // keys are the Connections picker's alone, nor for Reload schema, whose
    // key on a table's rows reloads them.
    let layout = crate::keymap::Layout::Mac;
    let new_sql = harness
        .app
        .keymap
        .key(layout, crate::keymap::Command::NewSqlTab);
    let new_sql = format!("New SQL query, command, {}", new_sql.as_str());
    for label in [
        new_sql.as_str(),
        "Edit connection, command, Bookshop · dev",
        "New connection, command",
        "Reload schema, command, Bookshop · dev",
    ] {
        assert!(harness.has(label), "{label}");
    }
    harness.press(Key::Escape, Modifiers::NONE);
    // A table in front brings its own.
    open_table(&mut harness, tab, "book_covers");
    ask(&mut harness, look, ">");
    let shown = listed(&harness);
    assert!(shown.contains(&"> Close tab".to_owned()), "{shown:?}");
    assert!(shown.contains(&"> Filter rows".to_owned()), "{shown:?}");
    assert!(!shown.contains(&"> Save changes".to_owned()), "{shown:?}");
    harness.press(Key::Escape, Modifiers::NONE);
    // What follows `>` finds commands, never a table: `reviews` matches
    // `re` too.
    ask(&mut harness, look, "> re");
    let shown = listed(&harness);
    assert_eq!(shown[1], "> Reload schema");
    assert!(!shown.contains(&"reviews".to_owned()), "{shown:?}");
}

/// Test 6, its settings: a setting's row opens Settings at that row: its
/// pane, the row on the selection's ground, its switch with the keyboard.
#[test]
fn a_setting_opens_settings_at_its_row() {
    let look = Look::macos();
    let (mut harness, _tab) = bookshop(look);
    ask(&mut harness, look, "> re");
    let restore = "> Settings › General › Restore tabs and filters";
    let index = result_named(&harness, restore);
    harness.app.apply(Action::QuickOpenPick {
        index: Some(index),
        in_editor: false,
    });
    harness.settle();
    let Some(Dialog::Settings(dialog)) = &harness.app.dialog else {
        panic!("Settings is not open");
    };
    assert_eq!(dialog.pane, crate::model::SettingsPane::General);
    assert_eq!(focused(&mut harness), "Restore tabs and filters");
    let grounded = |harness: &Harness| {
        let label = harness
            .painted_rect("Restore tabs and filters")
            .expect("the row's label");
        let selection = harness.app.palette.selection;
        harness
            .fills
            .iter()
            .any(|(rect, color)| *color == selection && rect.contains_rect(label))
    };
    assert!(grounded(&harness), "the row is on the selection's ground");
    // The keyboard stays where it was put: it is given once.
    harness.press(Key::Tab, Modifiers::NONE);
    assert_ne!(focused(&mut harness), "Restore tabs and filters");
    // Another pane leaves the row behind.
    for pane in [
        crate::model::SettingsPane::Appearance,
        crate::model::SettingsPane::General,
    ] {
        harness.app.apply(Action::ShowSettingsPane(pane));
        harness.settle();
    }
    assert!(!grounded(&harness));
}

/// A setting's row that the sheet shows only scrolled, in a short window,
/// is scrolled to.
#[test]
fn a_setting_below_the_fold_is_scrolled_to() {
    let look = Look::macos();
    let mut harness = Harness::with_size(egui::vec2(1280.0, 560.0));
    harness.set_look(look);
    harness.connect_with(bookshop_dev(), bookshop_objects());
    // Opened plainly, the sheet shows the row under its footer's line.
    harness.app.apply(Action::ShowSettings);
    harness.settle();
    let row = harness.painted_rect("Disconnect when idle");
    let footer = harness.painted_rect("Stored in").expect("the footer");
    assert!(row.is_none_or(|row| row.top() > footer.top()), "{row:?}");
    harness.app.apply(Action::CloseDialog);
    harness.settle();
    ask(&mut harness, look, "> idle");
    let index = result_named(&harness, "> Settings › General › Disconnect when idle");
    harness.app.apply(Action::QuickOpenPick {
        index: Some(index),
        in_editor: false,
    });
    harness.settle();
    let row = harness
        .painted_rect("Disconnect when idle")
        .expect("the row's label");
    let footer = harness.painted_rect("Stored in").expect("the footer");
    assert!(row.bottom() < footer.top(), "{row:?} under {footer:?}");
    assert_eq!(focused(&mut harness), "Disconnect when idle");
}

/// The commands quick open runs are the keymap's, built, each once; and
/// each listed does what it names, here a few that change what shows.
#[test]
fn listed_commands_do_what_they_name() {
    let mut seen = Vec::new();
    for command in crate::app::RUNNABLE {
        assert!(command.info().built, "{command:?}");
        assert!(!seen.contains(&command), "{command:?} twice");
        seen.push(command);
    }
    let look = Look::macos();
    let (mut harness, tab) = bookshop(look);
    let run = |harness: &mut Harness, name: &str| {
        ask(harness, look, ">");
        let index = result_named(harness, &format!("> {name}"));
        harness.app.apply(Action::QuickOpenPick {
            index: Some(index),
            in_editor: false,
        });
        harness.settle();
    };
    run(&mut harness, "Hide sidebar");
    assert!(harness.app.workspace(tab).unwrap().sidebar_hidden);
    run(&mut harness, "Show sidebar");
    assert!(!harness.app.workspace(tab).unwrap().sidebar_hidden);
    run(&mut harness, "Larger text");
    assert!(harness.app.settings.font_size.is_some());
    run(&mut harness, "Default text size");
    assert!(harness.app.settings.font_size.is_none());
    open_table(&mut harness, tab, "book_covers");
    open_table(&mut harness, tab, "authors");
    let panel = harness.app.workspace(tab).unwrap().row_panel;
    let name = if panel {
        "Hide row panel"
    } else {
        "Show row panel"
    };
    run(&mut harness, name);
    assert_eq!(harness.app.workspace(tab).unwrap().row_panel, !panel);
    run(&mut harness, "Previous tab");
    let workspace = harness.app.workspace(tab).unwrap();
    let front = workspace.active_object_tab().unwrap();
    assert_eq!(front.object.name, "book_covers");
    run(&mut harness, "Close tab");
    let workspace = harness.app.workspace(tab).unwrap();
    assert_eq!(workspace.tabs.len(), 1);
    // A SQL editor's run and format, once it holds a script.
    run(&mut harness, "New SQL query");
    ask(&mut harness, look, ">");
    assert!(!listed(&harness).contains(&"> Run all".to_owned()));
    harness.press(Key::Escape, Modifiers::NONE);
    let workspace = harness.app.workspace_mut(tab).unwrap();
    let id = workspace.active_tab.unwrap();
    workspace.sql_tab_mut(id).unwrap().text = "select 1".into();
    run(&mut harness, "Format SQL");
    let workspace = harness.app.workspace(tab).unwrap();
    assert_eq!(workspace.sql_tab(id).unwrap().text, "SELECT 1");
    run(&mut harness, "Settings");
    assert!(matches!(harness.app.dialog, Some(Dialog::Settings(_))));
}

/// A Bookshop development window that writes, its start screen showing.
fn bookshop_writable(look: Look) -> (Harness, ConnTabId) {
    let mut harness = Harness::new();
    harness.set_look(look);
    let mut saved = bookshop_dev();
    saved.read_only = Some(false);
    let tab = harness.connect_with(saved, bookshop_with_publishers());
    (harness, tab)
}

/// `book_covers` open on its rows, with a row deleted and not saved.
fn covers_with_a_change(harness: &mut Harness, tab: ConnTabId) -> crate::model::TabId {
    harness.app.apply(Action::OpenObject {
        tab,
        object: ObjectRef::new("main", "book_covers"),
        kind: ObjectKind::Table,
        pin: true,
    });
    harness.answer_structure(crate::testing::book_covers_structure());
    harness.answer_rows(crate::testing::book_covers_page(3));
    let id = harness.app.workspace(tab).unwrap().active_tab.unwrap();
    harness.app.workspace_mut(tab).unwrap().pane = crate::model::Pane::Grid;
    harness.settle();
    let cell = crate::model::CellPos { row: 1, col: 1 };
    harness.app.apply(Action::SelectCell { tab, id, cell });
    harness.app.apply(Action::DeleteRow { tab, id });
    harness.settle();
    id
}

/// Whether `book_covers` still holds its change.
fn changed(harness: &Harness, tab: ConnTabId, id: crate::model::TabId) -> bool {
    let workspace = harness.app.workspace(tab).unwrap();
    workspace.object_tab(id).unwrap().edits.pending()
}

/// The pick is the command it was put on, not a place in the list, which
/// changes by itself: a command that comes above it moves it, and one that
/// goes leaves Enter nothing to open (a review finding: Enter ran the
/// command that slid into its place).
#[test]
fn the_pick_holds_its_command_while_the_list_changes() {
    let look = Look::macos();
    let (mut harness, first) = bookshop_writable(look);
    harness.app.apply(Action::ShowConnections);
    let second = harness.connect_with(bookshop_production(), bookshop_objects());
    ask(&mut harness, look, ">");
    let next = result_named(&harness, "> Next connection");
    for _ in 0..next {
        harness.press(Key::ArrowDown, Modifiers::NONE);
    }
    // The server's other databases come in: Switch database is listed
    // above the pick.
    let databases = Some(vec!["bookshop".into(), "bookshop_archive".into()]);
    harness.app.workspace_mut(second).unwrap().databases.value = databases;
    harness.settle();
    assert_eq!(result_named(&harness, "> Next connection"), next + 1);
    harness.press(Key::Enter, Modifiers::NONE);
    assert!(harness.app.workspace(second).is_some(), "not closed");
    assert_eq!(harness.app.active_tab_id(), first);
    // A count in flight lists Cancel, after Discard all pending; the count
    // ends before Enter.
    let id = covers_with_a_change(&mut harness, first);
    let request = Some(crate::backend::RequestId(9_999));
    let workspace = harness.app.workspace_mut(first).unwrap();
    workspace.object_tab_mut(id).unwrap().count.pending = request;
    ask(&mut harness, look, ">");
    let cancel = result_named(&harness, "> Cancel running query");
    assert_eq!(cancel - 1, result_named(&harness, "> Discard all pending"));
    for _ in 0..cancel {
        harness.press(Key::ArrowDown, Modifiers::NONE);
    }
    let workspace = harness.app.workspace_mut(first).unwrap();
    workspace.object_tab_mut(id).unwrap().count.pending = None;
    harness.press(Key::Enter, Modifiers::NONE);
    assert!(changed(&harness, first, id), "nothing discarded");
    assert!(matches!(harness.app.dialog, Some(Dialog::QuickOpen(_))));
    // The next move starts from where the pick stood.
    harness.press(Key::ArrowUp, Modifiers::NONE);
    let discard = result_named(&harness, "> Discard all pending");
    let list = harness.app.quick_open_list(quick_open(&harness));
    let picked = harness.app.quick_open_picked(quick_open(&harness), &list);
    assert_eq!(picked, Some(discard));
}

/// A command that would do nothing is not listed: a save's three while a
/// save is under way, Run all with comments alone (review findings).
#[test]
fn commands_with_nothing_to_do_are_not_listed() {
    let look = Look::macos();
    let (mut harness, tab) = bookshop_writable(look);
    let id = covers_with_a_change(&mut harness, tab);
    ask(&mut harness, look, ">");
    let shown = listed(&harness);
    assert!(shown.contains(&"> Save changes".to_owned()), "{shown:?}");
    harness.press(Key::Escape, Modifiers::NONE);
    harness.app.apply(Action::WriteEdits { tab, id });
    ask(&mut harness, look, ">");
    let shown = listed(&harness);
    for hidden in [
        "> Save changes",
        "> Discard all pending",
        "> Review pending SQL",
    ] {
        assert!(!shown.contains(&hidden.to_owned()), "{hidden}: {shown:?}");
    }
    harness.press(Key::Escape, Modifiers::NONE);
    // The Structure view has no rows to filter.
    let view = crate::model::ObjectView::Structure;
    harness.app.apply(Action::SetView {
        tab,
        object_tab: id,
        view,
    });
    ask(&mut harness, look, ">");
    assert!(!listed(&harness).contains(&"> Filter rows".to_owned()));
    harness.press(Key::Escape, Modifiers::NONE);
    // An empty editor has nothing to run or format, comments alone
    // nothing to run.
    harness.app.apply(Action::NewSqlTab(tab));
    ask(&mut harness, look, ">");
    let shown = listed(&harness);
    assert!(!shown.contains(&"> Run all".to_owned()), "{shown:?}");
    assert!(!shown.contains(&"> Format SQL".to_owned()), "{shown:?}");
    harness.press(Key::Escape, Modifiers::NONE);
    let workspace = harness.app.workspace_mut(tab).unwrap();
    let sql = workspace.active_tab.unwrap();
    workspace.sql_tab_mut(sql).unwrap().text = "-- just a note".into();
    ask(&mut harness, look, ">");
    let shown = listed(&harness);
    assert!(!shown.contains(&"> Run all".to_owned()), "{shown:?}");
    assert!(shown.contains(&"> Format SQL".to_owned()), "{shown:?}");
    harness.press(Key::Escape, Modifiers::NONE);
    // The text's size: no larger at the largest, and no default where
    // the size set is the look's own.
    for _ in 0..20 {
        harness.app.apply(Action::TextSize(Some(true)));
    }
    ask(&mut harness, look, ">");
    assert!(!listed(&harness).contains(&"> Larger text".to_owned()));
    harness.press(Key::Escape, Modifiers::NONE);
    harness.app.apply(Action::TextSize(None));
    harness.app.apply(Action::TextSize(Some(true)));
    harness.app.apply(Action::TextSize(Some(false)));
    assert!(harness.app.settings.font_size.is_some());
    ask(&mut harness, look, ">");
    assert!(!listed(&harness).contains(&"> Default text size".to_owned()));
}

/// The Windows look lists the commands after `>` as macOS does; Omarchy
/// types it, and finds nothing by it.
#[test]
fn the_prefix_in_the_windows_and_omarchy_looks() {
    let look = Look::standard();
    let (mut harness, _tab) = bookshop(look);
    ask(&mut harness, look, "> new sql");
    assert_eq!(listed(&harness)[1], "> New SQL query");
    let look = Look::omarchy();
    let (mut harness, _tab) = bookshop(look);
    ask(&mut harness, look, ">");
    assert_eq!(quick_open(&harness).query, ">");
    assert!(listed(&harness).is_empty(), "{:?}", listed(&harness));
}

/// Another window brings the commands that go to it.
#[test]
fn the_next_window_is_offered_where_there_is_one() {
    let look = Look::macos();
    let (mut harness, _tab) = bookshop(look);
    ask(&mut harness, look, ">");
    assert!(!listed(&harness).contains(&"> Next connection".to_owned()));
    harness.press(Key::Escape, Modifiers::NONE);
    harness.app.apply(Action::ShowConnections);
    harness.connect_with(bookshop_production(), bookshop_objects());
    ask(&mut harness, look, ">");
    let shown = listed(&harness);
    assert!(shown.contains(&"> Next connection".to_owned()), "{shown:?}");
    assert!(shown.contains(&"> Previous connection".to_owned()));
}

/// Typed without `>`, the commands and settings that match come after the
/// objects, folded after three as each kind of object is; with nothing
/// typed, only the suggested three.
#[test]
fn the_commands_group_folds_after_three() {
    let look = Look::macos();
    let (mut harness, _tab) = bookshop(look);
    ask(&mut harness, look, "");
    let commands: Vec<String> = listed(&harness)
        .into_iter()
        .filter(|name| name.starts_with("> "))
        .collect();
    assert!(commands.len() <= 3, "{commands:?}");
    harness.press(Key::Escape, Modifiers::NONE);
    harness.app.apply(Action::ShowConnections);
    harness.connect_with(bookshop_production(), bookshop_objects());
    ask(&mut harness, look, "connection");
    let list = harness.app.quick_open_list(quick_open(&harness));
    let heading = list
        .rows
        .iter()
        .position(|row| *row == QuickRow::Heading(QuickHeading::Commands))
        .expect("the commands' heading");
    let after = &list.rows[heading + 1..];
    assert_eq!(after.len(), GROUP_SHOWN + 1, "{after:?}");
    let Some(QuickRow::Item(QuickItem::More { group, hidden })) = after.last() else {
        panic!("{after:?}");
    };
    assert_eq!(*group, QuickGroup::Commands);
    assert!(*hidden > 0);
    let more = list.items().count() - 1;
    harness.app.apply(Action::QuickOpenPick {
        index: Some(more),
        in_editor: false,
    });
    let list = harness.app.quick_open_list(quick_open(&harness));
    assert!(
        list.items()
            .all(|item| !matches!(item, QuickItem::More { .. }))
    );
}

/// The footer has `> commands` on macOS; Omarchy's commands stay on `:`.
/// What `>` finds nothing for is said.
#[test]
fn the_footer_names_the_prefix_and_no_command_is_said() {
    for look in [Look::macos(), Look::omarchy()] {
        let (mut harness, _tab) = bookshop(look);
        ask(&mut harness, look, "");
        let hint = harness
            .painted
            .iter()
            .any(|(text, _)| text.ends_with(" commands"));
        assert_eq!(hint, !look.terminal, "{}", look.name);
    }
    let look = Look::macos();
    let (mut harness, _tab) = bookshop(look);
    ask(&mut harness, look, "> zzz");
    assert!(harness.painted_color("No commands match “zzz”").is_some());
}
