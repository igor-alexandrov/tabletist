//! The menu bar's model, which items are enabled, what a click does, and
//! what a replayed key gives egui. Headless.

use egui::{Event, Key, Modifiers};
use tabletist_menubar::Via;

use crate::app::{App, RUNNABLE};
use crate::keymap::{Command, EditItem, MenuKey, Mods};
use crate::model::{Action, ConnTabId, Dialog};
use crate::testing::Harness;

use super::{Handling, Item, Platform, Replay, bar, handling};

fn items(platform: Platform) -> Vec<Item> {
    bar(platform)
        .into_iter()
        .flat_map(|menu| menu.items)
        .collect()
}

#[test]
fn every_runnable_command_is_in_one_item() {
    for platform in [Platform::MacOs, Platform::Windows] {
        let items = items(platform);
        for command in RUNNABLE {
            if command == Command::ShowAgents && !crate::endpoint::AVAILABLE {
                assert!(!items.contains(&Item::Command(command)));
                continue;
            }
            let count = items
                .iter()
                .filter(|item| **item == Item::Command(command))
                .count();
            assert_eq!(count, 1, "{platform:?}: {command:?}");
        }
        // Nothing but RUNNABLE's commands.
        for item in &items {
            if let Item::Command(command) = item {
                assert!(RUNNABLE.contains(command), "{command:?}");
            }
        }
    }
}

#[test]
fn every_item_has_a_title_and_separators_only_between_items() {
    for platform in [Platform::MacOs, Platform::Windows] {
        for menu in bar(platform) {
            assert!(!menu.title.is_empty());
            let first = menu.items.first();
            let last = menu.items.last();
            assert_ne!(first, Some(&Item::Separator), "{}", menu.title);
            assert_ne!(last, Some(&Item::Separator), "{}", menu.title);
            for pair in menu.items.windows(2) {
                assert!(pair != [Item::Separator, Item::Separator], "{}", menu.title);
            }
            for item in &menu.items {
                if *item != Item::Separator {
                    assert!(!super::title(*item, false).is_empty(), "{item:?}");
                }
            }
        }
    }
}

#[test]
fn each_platform_has_its_own_items() {
    let mac = bar(Platform::MacOs);
    let windows = bar(Platform::Windows);
    let titles = |menus: &[super::Menu]| menus.iter().map(|menu| menu.title).collect::<Vec<_>>();
    assert_eq!(
        titles(&mac),
        [
            "Tabletist",
            "File",
            "Edit",
            "View",
            "Query",
            "Window",
            "Help"
        ]
    );
    assert_eq!(
        titles(&windows),
        ["File", "Edit", "View", "Query", "Window", "Help"]
    );
    // Exit and About in File and Help on Windows; About in the app menu
    // on the Mac, which has no Exit.
    assert!(items(Platform::Windows).contains(&Item::Exit));
    assert!(!items(Platform::MacOs).contains(&Item::Exit));
    assert_eq!(
        windows.last().map(|help| help.items.last()),
        Some(Some(&Item::About))
    );
    assert_eq!(mac[0].items[0], Item::About);
    // The Edit menu is the app's own on both.
    for platform in [Platform::MacOs, Platform::Windows] {
        for edit in EditItem::ALL {
            assert!(
                items(platform).contains(&Item::Edit(edit)),
                "{platform:?}: {edit:?}"
            );
        }
    }
}

#[test]
fn the_window_s_commands_are_enabled_on_the_connection_picker() {
    // A new harness shows the picker, which has no workspace.
    let harness = Harness::new();
    let app = &harness.app;
    for command in [
        Command::Settings,
        Command::Help,
        Command::NewConnection,
        Command::Connections,
        Command::LargerText,
        Command::SmallerText,
    ] {
        assert!(
            app.menu_state(Item::Command(command)).enabled,
            "{command:?}"
        );
    }
    // A connection's commands are not, nor Quick Open, which searches one.
    assert!(
        !app.menu_state(Item::Command(Command::CloseConnection))
            .enabled
    );
    assert!(!app.menu_state(Item::QuickOpen).enabled);
}

#[test]
fn an_item_is_greyed_where_quick_open_hides_its_command() {
    let mut harness = Harness::new();
    harness.connect_fake();
    harness.settle();
    // One database: nothing to switch to.
    let app = &harness.app;
    assert!(
        !app.menu_state(Item::Command(Command::SwitchDatabase))
            .enabled
    );
    // A connection is there to close.
    assert!(
        app.menu_state(Item::Command(Command::CloseConnection))
            .enabled
    );
    // No SQL editor in front: nothing to run.
    assert!(!app.menu_state(Item::Command(Command::RunAll)).enabled);
}

#[test]
fn under_a_dialog_only_what_its_keys_reach_stays_enabled() {
    let mut harness = Harness::new();
    harness.connect_fake();
    harness.app.apply(Action::ShowAbout);
    harness.settle();
    let app = &harness.app;
    assert!(matches!(app.dialog, Some(Dialog::About)));
    // The text sizes' keys work under a dialog.
    assert!(app.menu_state(Item::Command(Command::LargerText)).enabled);
    // A new SQL editor's do not.
    assert!(!app.menu_state(Item::Command(Command::NewSqlTab)).enabled);
    // About, Quick Open and the Edit items stay enabled: their actions
    // and egui decide.
    assert!(app.menu_state(Item::About).enabled);
    assert!(app.menu_state(Item::QuickOpen).enabled);
    assert!(app.menu_state(Item::Edit(EditItem::Copy)).enabled);
}

#[test]
fn settings_stays_enabled_under_the_shortcuts_dialog() {
    let mut harness = Harness::new();
    harness.app.apply(Action::ShowHelp);
    harness.settle();
    assert!(
        harness
            .app
            .menu_state(Item::Command(Command::Settings))
            .enabled
    );
    harness.app.choose(Item::Command(Command::Settings));
    harness.settle();
    assert!(matches!(harness.app.dialog, Some(Dialog::Settings(_))));
}

#[test]
fn a_toggle_says_what_a_click_does() {
    let mut harness = Harness::new();
    let tab = harness.connect_fake();
    harness.settle();
    let sidebar = Item::Command(Command::ToggleSidebar);
    assert_eq!(harness.app.menu_state(sidebar).title, "Hide Sidebar");
    harness.app.apply(Action::ToggleSidebar(tab));
    harness.settle();
    assert_eq!(harness.app.menu_state(sidebar).title, "Show Sidebar");
    // Greyed, a toggle shows its first title.
    let mode = Item::Command(Command::ToggleSqlMode);
    assert!(!harness.app.menu_state(mode).enabled);
    assert_eq!(harness.app.menu_state(mode).title, "Make Tab Read-Only");
}

#[test]
fn a_clicked_command_does_what_quick_open_s_row_does() {
    let mut harness = Harness::new();
    let tab = harness.connect_fake();
    harness.settle();
    harness.app.actions.clear();
    harness.app.choose(Item::Command(Command::NewSqlTab));
    // Quick open's row for it pushes `NewSqlTab` for the window in front
    // (`run_of`). `Action` derives only `Debug`, so match it.
    assert!(
        matches!(harness.app.actions.as_slice(), [Action::NewSqlTab(t)] if *t == tab),
        "{:?}",
        harness.app.actions
    );
    // A command that does not apply pushes nothing.
    harness.app.choose(Item::Command(Command::SaveChanges));
    assert_eq!(harness.app.actions.len(), 1);
}

#[test]
fn under_a_dialog_only_the_text_sizes_keys_reach() {
    let mut harness = Harness::new();
    assert!(
        RUNNABLE
            .into_iter()
            .all(|command| harness.app.key_reaches(command))
    );
    harness.app.apply(Action::ShowAbout);
    assert!(matches!(harness.app.dialog, Some(Dialog::About)));
    // Their lines hold in a prompt too. A command that gains such a line
    // is clickable under every dialog: decide it here.
    let reach: Vec<Command> = RUNNABLE
        .into_iter()
        .filter(|command| harness.app.key_reaches(*command))
        .collect();
    assert_eq!(
        reach,
        [
            Command::LargerText,
            Command::SmallerText,
            Command::DefaultTextSize
        ]
    );
}

/// What quick open's row for `command` says in `tab`'s window.
fn quick_name(app: &App, tab: ConnTabId, command: Command) -> &'static str {
    let mut runnable = app.runnable(tab).into_iter();
    let row = runnable.find(|runnable| runnable.command == command);
    row.unwrap_or_else(|| panic!("quick open lists {command:?}"))
        .name
}

/// `command`'s item, enabled, with the title quick open's row has (its
/// words, as a menu writes them); the title.
fn toggle_title(app: &App, tab: ConnTabId, command: Command) -> &'static str {
    let state = app.menu_state(Item::Command(command));
    assert!(state.enabled, "{command:?}");
    let quick = quick_name(app, tab, command);
    let same = match command {
        Command::ToggleSqlMode => {
            (state.title == "Make Tab Read-Write") == (quick == "Read-write tab")
        }
        _ => state.title.eq_ignore_ascii_case(quick),
    };
    assert!(same, "{command:?}: {:?} against {quick:?}", state.title);
    state.title
}

#[test]
fn each_toggle_says_what_quick_open_s_row_says_both_ways() {
    let mut harness = Harness::new();
    // A writable connection with a table's rows in front: the row panel
    // has a row to show.
    let (tab, _) = harness.editable();
    harness.settle();
    let walk = |harness: &mut Harness, command: Command, toggle: &dyn Fn() -> Action| {
        let before = toggle_title(&harness.app, tab, command);
        harness.app.apply(toggle());
        harness.settle();
        let after = toggle_title(&harness.app, tab, command);
        assert_ne!(before, after, "{command:?}");
        harness.app.apply(toggle());
        harness.settle();
        assert_eq!(toggle_title(&harness.app, tab, command), before);
    };
    walk(&mut harness, Command::ToggleSidebar, &|| {
        Action::ToggleSidebar(tab)
    });
    walk(&mut harness, Command::ToggleInspector, &|| {
        Action::ToggleRowPanel(tab)
    });
    // A SQL editor in front, where the connection takes writes.
    harness.app.apply(Action::NewSqlTab(tab));
    harness.settle();
    let sql_tab = harness
        .app
        .workspace(tab)
        .and_then(|workspace| workspace.active_tab);
    let sql_tab = sql_tab.expect("the editor is in front");
    walk(&mut harness, Command::ToggleSqlMode, &|| {
        Action::ToggleSqlMode { tab, sql_tab }
    });
}

fn cmd_key(key: Key, shift: bool) -> MenuKey {
    MenuKey {
        mods: Mods {
            cmd: true,
            shift,
            ..Mods::default()
        },
        key,
    }
}

/// The modifiers a real press of `key` holds, as `Replay` writes them.
fn held(key: MenuKey) -> Modifiers {
    super::replay::modifiers(key.mods)
}

#[test]
fn a_key_s_replay_comes_before_its_release() {
    let run_all = cmd_key(Key::Enter, true);
    let mut replay = Replay::default();
    // A quick tap: the release is already in the frame's input.
    let mut events = vec![crate::testing::release(Key::Enter, held(run_all))];
    replay.frame(&mut events, &[(run_all, Via::Key)]);
    assert!(matches!(
        events[0],
        Event::Key {
            key: Key::Enter,
            pressed: true,
            ..
        }
    ));
    assert!(matches!(
        events[1],
        Event::Key {
            key: Key::Enter,
            pressed: false,
            ..
        }
    ));
}

#[test]
fn a_click_s_replay_holds_its_modifiers_for_one_frame_and_releases_its_key() {
    let undo = cmd_key(Key::Z, false);
    let mut replay = Replay::default();
    let mut events = Vec::new();
    replay.frame(&mut events, &[(undo, Via::Click)]);
    assert_eq!(events.len(), 3, "{events:?}");
    assert_eq!(events[0], Event::ModifiersChanged(held(undo)));
    assert!(matches!(
        events[1],
        Event::Key {
            key: Key::Z,
            pressed: true,
            ..
        }
    ));
    assert!(matches!(
        events[2],
        Event::Key {
            key: Key::Z,
            pressed: false,
            ..
        }
    ));
    // The next frame gives back what is really held.
    let mut next = Vec::new();
    replay.frame(&mut next, &[]);
    assert_eq!(next, [Event::ModifiersChanged(Modifiers::NONE)]);
    // And then nothing more.
    let mut after = Vec::new();
    replay.frame(&mut after, &[]);
    assert!(after.is_empty());
}

#[test]
fn copy_and_cut_are_the_window_s_clipboard_events() {
    let mut replay = Replay::default();
    let mut events = Vec::new();
    replay.frame(&mut events, &[(cmd_key(Key::C, false), Via::Click)]);
    assert!(events.contains(&Event::Copy), "{events:?}");
    let mut events = Vec::new();
    replay.frame(&mut events, &[(cmd_key(Key::X, false), Via::Key)]);
    assert!(events.contains(&Event::Cut), "{events:?}");
}

/// `RunSql` commands the backend was sent, as
/// `src/ui/editor_results_tests.rs`'s `runs_sent` counts them.
fn runs_sent(harness: &Harness) -> usize {
    let sent = harness.app.backend.sent.iter();
    sent.filter(|command| matches!(command, crate::backend::Command::RunSql { .. }))
        .count()
}

/// A connected window with an empty SQL editor in front, which has the
/// keyboard.
fn sql_editor() -> (Harness, ConnTabId) {
    let mut harness = Harness::new();
    let tab = harness.connect_fake();
    harness.app.apply(Action::NewSqlTab(tab));
    harness.settle();
    (harness, tab)
}

/// The SQL editor in front of `tab`'s window.
fn sql_text(harness: &Harness, tab: ConnTabId) -> &str {
    let workspace = harness.app.workspace(tab).unwrap();
    &workspace.active_sql_tab().unwrap().text
}

#[test]
fn run_all_replayed_runs_once_when_held_and_again_for_a_second_tap() {
    // A SQL editor in front with a statement, as `src/ui/mod.rs`'s
    // `sql_harness` and `set_sql` make one.
    let (mut harness, tab) = sql_editor();
    {
        let workspace = harness.app.workspace_mut(tab).unwrap();
        let id = workspace.active_tab.unwrap();
        let sql = workspace.sql_tab_mut(id).unwrap();
        sql.text = "select 1".into();
        sql.cursor = 0;
    }
    harness.settle();
    let key = harness
        .app
        .keymap
        .menu_key(Command::RunAll, harness.app.layout())
        .expect("Run All has a chord");
    let mut replay = Replay::default();
    let before = runs_sent(&harness);
    // Held: AppKit sends the key equivalent again; egui still holds the
    // key, so the later ones are repeats.
    for _ in 0..3 {
        let mut events = Vec::new();
        replay.frame(&mut events, &[(key, Via::Key)]);
        harness.frame(events);
    }
    harness.frame(vec![crate::testing::release(key.key, held(key))]);
    assert_eq!(runs_sent(&harness), before + 1, "held, it runs once");
    // The run comes back, as `src/ui/editor_results_tests.rs` answers one.
    harness.answer_sql(
        Ok(crate::testing::script_outcome(vec![
            crate::testing::rows_outcome(1),
        ])),
        None,
    );
    harness.settle();
    // Then a second tap, whose release is already in the frame's input.
    let mut events = vec![crate::testing::release(key.key, held(key))];
    replay.frame(&mut events, &[(key, Via::Key)]);
    harness.frame(events);
    assert_eq!(runs_sent(&harness), before + 2, "a second tap runs again");
    // The tap's key went back up: a press held after it is new, not a
    // repeat of a key egui still holds.
    harness.answer_sql(
        Ok(crate::testing::script_outcome(vec![
            crate::testing::rows_outcome(1),
        ])),
        None,
    );
    harness.settle();
    let mut events = Vec::new();
    replay.frame(&mut events, &[(key, Via::Key)]);
    harness.frame(events);
    assert_eq!(runs_sent(&harness), before + 3, "a third press runs");
}

#[test]
fn a_clicked_copy_copies_the_grid_s_cell() {
    // A cell selected with the grid having the keyboard, as
    // `src/ui/mod.rs` sets it up before `harness.copy`.
    let mut harness = Harness::new();
    let (tab, id) = harness.book_covers();
    harness.app.apply(Action::SelectCell {
        tab,
        id,
        cell: crate::model::CellPos { row: 0, col: 1 },
    });
    harness.settle();
    harness.app.workspace_mut(tab).unwrap().pane = crate::model::Pane::Grid;
    let copy = harness
        .app
        .keymap
        .edit_key(EditItem::Copy, harness.app.layout())
        .expect("Copy's chord");
    let mut replay = Replay::default();
    let mut events = Vec::new();
    replay.frame(&mut events, &[(copy, Via::Click)]);
    harness.frame(events);
    assert!(harness.copied.is_some(), "the grid copied its cell");
    // The next frame lets the modifiers go, as `harness.copy` does.
    let mut next = Vec::new();
    replay.frame(&mut next, &[]);
    assert_eq!(next, [Event::ModifiersChanged(Modifiers::NONE)]);
    harness.frame(next);
}

#[test]
fn a_replayed_cmd_y_in_a_text_field_is_redo_not_history() {
    // Typed, then taken back, as `src/ui/complete_tests.rs` does before
    // its redo by either key.
    let (mut harness, tab) = sql_editor();
    harness.frame(vec![Event::Text("x".into())]);
    harness.settle();
    let typed = sql_text(&harness, tab).to_owned();
    assert!(!typed.is_empty());
    let layout = harness.app.layout();
    let undo = harness.app.keymap.edit_key(EditItem::Undo, layout);
    let undo = undo.expect("Undo's chord");
    harness.press(undo.key, held(undo));
    assert_eq!(sql_text(&harness, tab), "");
    let history = harness.app.keymap.menu_key(Command::History, layout);
    let history = history.expect("History has a chord");
    // Its key is the text field's redo: the editor reads it before the
    // keymap does.
    let mut replay = Replay::default();
    let mut events = Vec::new();
    replay.frame(&mut events, &[(history, Via::Key)]);
    harness.frame(events);
    harness.frame(vec![crate::testing::release(history.key, held(history))]);
    harness.settle();
    assert_eq!(sql_text(&harness, tab), typed, "the text came back");
    let workspace = harness.app.workspace(tab).unwrap();
    let front = workspace.active_tab.and_then(|id| workspace.tab(id));
    assert!(
        front.and_then(crate::model::Tab::as_sql).is_some(),
        "no History tab opened"
    );
}

#[test]
fn a_replayed_cmd_r_on_a_table_s_rows_reloads_the_rows() {
    // A table's rows in front with the grid having the keyboard, as
    // `src/ui/mod.rs`'s `with_page` and `focus_grid` set it up.
    let mut harness = Harness::new();
    let tab = harness.connect_fake();
    harness.click("users");
    harness.answer_rows(crate::testing::page(5, false));
    harness.app.workspace_mut(tab).unwrap().pane = crate::model::Pane::Grid;
    harness.settle();
    let reload = harness
        .app
        .keymap
        .menu_key(Command::Reload, harness.app.layout())
        .expect("Reload has a chord");
    let mut replay = Replay::default();
    let mut events = vec![crate::testing::release(reload.key, held(reload))];
    replay.frame(&mut events, &[(reload, Via::Key)]);
    harness.frame(events);
    assert!(
        matches!(
            crate::testing::last_sent(&harness.app),
            crate::backend::Command::FetchRows { .. }
        ),
        "{:?}",
        crate::testing::last_sent(&harness.app)
    );
}

#[test]
fn a_key_under_a_dialog_that_the_keymap_would_not_read_is_a_click() {
    let mut harness = Harness::new();
    harness.app.apply(Action::ShowHelp);
    harness.settle();
    let settings = Item::Command(Command::Settings);
    let key = super::key_of(&harness.app.keymap, harness.app.layout(), settings);
    // `⌘,` over the shortcuts dialog: `keys::handle` does not run there,
    // so a replay would do nothing; it runs as a click and opens Settings.
    assert_eq!(
        handling(&harness.app, settings, Via::Key, key),
        Handling::Click
    );
    // Without a dialog it is the key's.
    harness.app.dialog = None;
    assert!(matches!(
        handling(&harness.app, settings, Via::Key, key),
        Handling::Replay(_, Via::Key)
    ));
    // Paste is always the window's; the other Edit items always replay.
    let paste = Item::Edit(EditItem::Paste);
    let copy = Item::Edit(EditItem::Copy);
    let layout = harness.app.layout();
    let copy_key = harness.app.keymap.edit_key(EditItem::Copy, layout);
    assert_eq!(
        handling(&harness.app, paste, Via::Click, None),
        Handling::Click
    );
    assert!(matches!(
        handling(&harness.app, copy, Via::Click, copy_key),
        Handling::Replay(_, Via::Click)
    ));
}

#[test]
fn a_greyed_command_chosen_by_a_key_or_a_script_does_nothing() {
    let mut harness = Harness::new();
    harness.connect_fake();
    harness.app.apply(Action::ShowAbout);
    harness.settle();
    let new_sql = Item::Command(Command::NewSqlTab);
    let key = super::key_of(&harness.app.keymap, harness.app.layout(), new_sql);
    // Greyed under the dialog, and its key not read there: AppKit still
    // performs it (the item is enabled outside an open menu), and the app
    // drops it rather than open a tab behind the dialog.
    assert!(!harness.app.menu_state(new_sql).enabled);
    assert_eq!(
        handling(&harness.app, new_sql, Via::Key, key),
        Handling::Ignore
    );
    // A script's click on the greyed item, likewise.
    assert_eq!(
        handling(&harness.app, new_sql, Via::Click, key),
        Handling::Ignore
    );
}
