//! The menu bar's model, which items are enabled, what a click does, and
//! what a replayed key gives egui. Headless.

use std::collections::HashMap;

use egui::{Event, Key, Modifiers};
use tabletist_menubar::{Entry, Via};

use crate::app::{App, RUNNABLE};
use crate::keymap::{Command, EditItem, MenuKey, Mods, Row, Scope, When};
use crate::model::{Action, ConnTabId, Dialog};
use crate::testing::Harness;

use super::bar::{Change, Described, Feed, Id, Keyed, admitted, changes, describe, keys_of, route};
use super::{Handling, Item, Platform, Replay, bar, handling, key_of, window_command};

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
    let pending = replay.frame(&mut events, &[(undo, Via::Click)]);
    assert!(pending, "a restore waits for the next frame");
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
    assert!(!replay.frame(&mut next, &[]), "nothing waits after it");
    assert_eq!(next, [Event::ModifiersChanged(Modifiers::NONE)]);
    // And then nothing more.
    let mut after = Vec::new();
    replay.frame(&mut after, &[]);
    assert!(after.is_empty());
}

#[test]
fn a_click_s_replay_gives_back_what_winit_said_is_held() {
    let undo = cmd_key(Key::Z, false);
    let mut replay = Replay::default();
    // Shift is down when the item is clicked.
    let mut events = vec![Event::ModifiersChanged(Modifiers::SHIFT)];
    replay.frame(&mut events, &[(undo, Via::Click)]);
    assert_eq!(
        events.last(),
        Some(&crate::testing::release(Key::Z, held(undo)))
    );
    let mut next = Vec::new();
    replay.frame(&mut next, &[]);
    assert_eq!(next, [Event::ModifiersChanged(Modifiers::SHIFT)]);
}

#[test]
fn a_frame_that_says_what_is_held_gets_no_restore() {
    let undo = cmd_key(Key::Z, false);
    let mut replay = Replay::default();
    let mut events = Vec::new();
    replay.frame(&mut events, &[(undo, Via::Click)]);
    // Winit says what is held itself: that stands, alone.
    let told = Event::ModifiersChanged(Modifiers::ALT);
    let mut next = vec![told.clone()];
    assert!(!replay.frame(&mut next, &[]));
    assert_eq!(next, [told]);
    // And it is what a later click gives back.
    let mut events = Vec::new();
    replay.frame(&mut events, &[(undo, Via::Click)]);
    let mut next = Vec::new();
    replay.frame(&mut next, &[]);
    assert_eq!(next, [Event::ModifiersChanged(Modifiers::ALT)]);
}

/// Whether each key press in `events` is a repeat, as egui reads them
/// from a frame whose keys start up.
fn repeats(events: Vec<Event>) -> Vec<bool> {
    let ctx = egui::Context::default();
    let mut seen = Vec::new();
    let input = egui::RawInput {
        events,
        ..Default::default()
    };
    let mut output = ctx.run_ui(input, |ui| seen = ui.input(|input| input.events.clone()));
    output.textures_delta.clear();
    seen.iter()
        .filter_map(|event| match event {
            Event::Key {
                pressed: true,
                repeat,
                ..
            } => Some(*repeat),
            _ => None,
        })
        .collect()
}

#[test]
fn two_presses_of_one_key_in_one_frame_are_a_press_and_a_repeat() {
    // AppKit sends each autorepeat of a held chord as one more choice by
    // key, with nothing to tell it from a tap: those must stay repeats
    // (Run All may write). So two quick taps in one frame, rarer than a
    // held chord, are one press and a repeat.
    let run_all = cmd_key(Key::Enter, true);
    let mut replay = Replay::default();
    let up = crate::testing::release(Key::Enter, held(run_all));
    let mut events = vec![up.clone(), up];
    let tap = (run_all, Via::Key);
    replay.frame(&mut events, &[tap, tap]);
    let pressed: Vec<Option<bool>> = events
        .iter()
        .map(|event| match event {
            Event::Key { pressed, .. } => Some(*pressed),
            _ => None,
        })
        .collect();
    assert_eq!(
        pressed,
        [Some(true), Some(true), Some(false), Some(false)],
        "{events:?}"
    );
    assert_eq!(repeats(events), [false, true]);
}

#[test]
fn a_key_s_replay_comes_before_a_focus_loss() {
    let run_all = cmd_key(Key::Enter, true);
    let mut replay = Replay::default();
    replay.frame(&mut vec![Event::ModifiersChanged(Modifiers::SHIFT)], &[]);
    // The key went down, then the window lost focus before its release:
    // egui lets every key go there, so the press must come first.
    let mut events = vec![Event::WindowFocused(false)];
    replay.frame(&mut events, &[(run_all, Via::Key)]);
    assert!(
        matches!(
            events.as_slice(),
            [
                Event::Key {
                    key: Key::Enter,
                    pressed: true,
                    ..
                },
                Event::WindowFocused(false)
            ]
        ),
        "{events:?}"
    );
    // Nothing is held after it: a click gives back no Shift.
    let mut events = Vec::new();
    replay.frame(&mut events, &[(cmd_key(Key::Z, false), Via::Click)]);
    let mut next = Vec::new();
    replay.frame(&mut next, &[]);
    assert_eq!(next, [Event::ModifiersChanged(Modifiers::NONE)]);
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
fn a_held_run_all_s_autorepeats_and_release_in_one_frame_run_nothing_more() {
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
    // The chord goes down: egui holds the key from here.
    let mut events = Vec::new();
    replay.frame(&mut events, &[(key, Via::Key)]);
    harness.frame(events);
    assert_eq!(runs_sent(&harness), before + 1);
    harness.answer_sql(
        Ok(crate::testing::script_outcome(vec![
            crate::testing::rows_outcome(1),
        ])),
        None,
    );
    harness.settle();
    // A long frame: two autorepeats, then the release.
    let mut events = vec![crate::testing::release(key.key, held(key))];
    replay.frame(&mut events, &[(key, Via::Key), (key, Via::Key)]);
    harness.frame(events);
    assert_eq!(runs_sent(&harness), before + 1, "the repeats ran nothing");
    let down = harness
        .ctx
        .input(|input| input.keys_down.contains(&key.key));
    assert!(!down, "the key went back up");
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
    let fetched = |harness: &Harness| {
        let sent = harness.app.backend.sent.iter();
        sent.filter(|command| matches!(command, crate::backend::Command::FetchRows { .. }))
            .count()
    };
    let schemas = |harness: &Harness| {
        let sent = harness.app.backend.sent.iter();
        sent.filter(|command| {
            matches!(
                command,
                crate::backend::Command::ListSchemas { .. }
                    | crate::backend::Command::ListObjects { .. }
            )
        })
        .count()
    };
    let (rows, tree) = (fetched(&harness), schemas(&harness));
    let mut replay = Replay::default();
    let mut events = vec![crate::testing::release(reload.key, held(reload))];
    replay.frame(&mut events, &[(reload, Via::Key)]);
    harness.frame(events);
    assert_eq!(fetched(&harness), rows + 1, "the rows were fetched again");
    assert_eq!(schemas(&harness), tree, "not the schema");
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

#[test]
fn an_edit_item_without_a_key_is_greyed_but_paste() {
    let mut harness = Harness::new();
    let copy = Item::Edit(EditItem::Copy);
    let paste = Item::Edit(EditItem::Paste);
    // The standard look's layout has the Edit items' keys.
    assert!(harness.app.menu_state(copy).enabled);
    assert!(harness.app.menu_state(paste).enabled);
    // The Omarchy look's has none: a click would replay nothing. Paste is
    // the window's, which reads the clipboard.
    harness.set_look(crate::theme::Look::omarchy());
    let layout = harness.app.layout();
    assert_eq!(harness.app.keymap.edit_key(EditItem::Copy, layout), None);
    assert!(!harness.app.menu_state(copy).enabled);
    assert!(harness.app.menu_state(paste).enabled);
    assert_eq!(
        handling(&harness.app, copy, Via::Click, None),
        Handling::Ignore
    );
}

#[test]
fn the_bar_described_has_each_item_once_as_built() {
    let harness = Harness::new();
    let app = &harness.app;
    for platform in [Platform::MacOs, Platform::Windows] {
        let described = describe(platform, app);
        let menus = bar(platform);
        assert_eq!(described.menus.len(), menus.len());
        for (built, menu) in described.menus.iter().zip(&menus) {
            // Windows gives each menu an access key.
            let title = match platform {
                Platform::MacOs => menu.title.to_owned(),
                Platform::Windows => format!("&{}", menu.title),
            };
            assert_eq!(built.title, title);
            assert_eq!(built.entries.len(), menu.items.len(), "{}", menu.title);
            for (entry, &item) in built.entries.iter().zip(&menu.items) {
                match (entry, item) {
                    (Entry::Separator, Item::Separator) => {}
                    (Entry::Native(native, title), Item::Native(of)) => {
                        assert_eq!(*native, of);
                        assert_eq!(title.as_deref(), Some(super::title(item, false)));
                    }
                    (Entry::Item { id, title, key }, item) => {
                        assert_eq!(described.items.get(id), Some(&item));
                        assert_eq!(title, super::title(item, false));
                        let state = described.shown[id];
                        assert!(state.enabled, "{item:?}");
                        assert_eq!(state.title, super::title(item, false));
                        // The keymap's key, as the menu shows it.
                        let of = key_of(&app.keymap, app.layout(), item);
                        assert_eq!(key.map(|key| key.key), of.map(|key| key.key));
                        assert_eq!(key.map(|key| key.cmd), of.map(|key| key.mods.cmd));
                    }
                    (_, item) => panic!("{platform:?}: {item:?} described as another entry"),
                }
            }
        }
        // Every item but the separators and the platform's own has an id.
        let ours = menus
            .iter()
            .flat_map(|menu| &menu.items)
            .filter(|item| !matches!(item, Item::Separator | Item::Native(_)))
            .count();
        assert_eq!(described.items.len(), ours);
        assert_eq!(described.shown.len(), ours);
    }
}

/// The id `platform`'s bar gives `item`.
fn id_of(items: &HashMap<Id, Item>, item: Item) -> Id {
    let found = items.iter().find(|(_, of)| **of == item);
    *found.expect("the item is in the bar").0
}

#[test]
fn a_choice_is_routed_as_menu_handling_says() {
    let (mut harness, tab) = sql_editor();
    {
        let workspace = harness.app.workspace_mut(tab).unwrap();
        let id = workspace.active_tab.unwrap();
        workspace.sql_tab_mut(id).unwrap().text = "select 1".into();
    }
    harness.settle();
    let app = &harness.app;
    let Described { mut items, .. } = describe(Platform::MacOs, app);
    let keys = keys_of(&items, app);
    let run_all = id_of(&items, Item::Command(Command::RunAll));
    let settings = id_of(&items, Item::Command(Command::Settings));
    let save = id_of(&items, Item::Command(Command::SaveChanges));
    let paste = id_of(&items, Item::Edit(EditItem::Paste));
    let run_key = keys[&run_all].expect("Run All has a chord");
    let save_key = keys[&save].expect("Save has a chord");
    let (presses, clicks) = route(
        app,
        &items,
        &keys,
        vec![
            // A key is given back as it was pressed.
            (run_all, Via::Key),
            // A click runs the command.
            (settings, Via::Click),
            // Nothing to save: greyed, so a click does nothing, but the
            // key goes where it goes without a menu.
            (save, Via::Click),
            (save, Via::Key),
            // The window reads the clipboard.
            (paste, Via::Key),
        ],
    );
    assert_eq!(presses, [(run_key, Via::Key), (save_key, Via::Key)]);
    assert_eq!(
        clicks,
        [
            Item::Command(Command::Settings),
            Item::Edit(EditItem::Paste)
        ]
    );
    // An id the bar no longer has is passed over.
    items.remove(&settings);
    let (presses, clicks) = route(app, &items, &keys, vec![(settings, Via::Click)]);
    assert!(presses.is_empty() && clicks.is_empty());
}

#[test]
fn the_bar_is_told_only_what_changed() {
    let mut harness = Harness::new();
    let Described {
        items, mut shown, ..
    } = describe(Platform::MacOs, &harness.app);
    let close = id_of(&items, Item::Command(Command::CloseConnection));
    let sidebar = id_of(&items, Item::Command(Command::ToggleSidebar));
    // Built, every item is enabled: on the picker a connection's are not.
    let first = changes(&mut shown, &items, &harness.app);
    assert!(first.contains(&Change::Enabled(close, false)), "{first:?}");
    assert!(!first.contains(&Change::Enabled(
        id_of(&items, Item::Command(Command::Settings)),
        false
    )));
    // As it was, nothing.
    assert_eq!(changes(&mut shown, &items, &harness.app), []);
    // Connected, the sidebar shown: enabled, and its title says so.
    let tab = harness.connect_fake();
    harness.settle();
    let connected = changes(&mut shown, &items, &harness.app);
    assert!(
        connected.contains(&Change::Enabled(close, true)),
        "{connected:?}"
    );
    assert!(
        connected.contains(&Change::Enabled(sidebar, true)),
        "{connected:?}"
    );
    assert!(
        connected.contains(&Change::Title(sidebar, "Hide Sidebar")),
        "{connected:?}"
    );
    harness.app.apply(Action::ToggleSidebar(tab));
    harness.settle();
    assert_eq!(
        changes(&mut shown, &items, &harness.app),
        [Change::Title(sidebar, "Show Sidebar")]
    );
}

#[test]
fn the_bar_is_built_again_only_when_an_item_s_key_changed() {
    let mut harness = Harness::new();
    let Described { items, .. } = describe(Platform::MacOs, &harness.app);
    let mut keyed = Keyed::new(&items, &harness.app);
    // The same keymap and look: nothing to ask.
    assert_eq!(keyed.rekey(&items, &harness.app), None);
    // Every settings change makes a new keymap: the keys are as they were.
    let before = harness.app.keymap.clone();
    harness
        .app
        .change_settings(|settings| settings.page_size = 500);
    assert!(!std::sync::Arc::ptr_eq(&before, &harness.app.keymap));
    assert_eq!(keyed.rekey(&items, &harness.app), None);
    // A key taken away is a key changed: muda cannot take it off the
    // item, so the bar is built again. (The sidebar's toggle has one
    // line, so unbinding it leaves no key.)
    let row = Row {
        command: Command::ToggleSidebar,
        scope: Scope::Global,
        when: When::Always,
    };
    let layout = harness.app.layout();
    let unbound = harness.app.settings.keys.unbind(layout, row);
    harness
        .app
        .change_settings(|settings| settings.keys = unbound);
    let changed = keyed.rekey(&items, &harness.app).expect("a key changed");
    let sidebar = id_of(&items, Item::Command(Command::ToggleSidebar));
    assert!(keyed.keys[&sidebar].is_some());
    assert_eq!(changed[&sidebar], None);
    let unchanged = |id: &Id| changed[id] == keyed.keys[id];
    assert!(items.keys().filter(|id| **id != sidebar).all(unchanged));
    // The keymap is recorded though the keys are not kept (a rebuild muda
    // refused): it is asked once.
    assert_eq!(keyed.rekey(&items, &harness.app), None);
    keyed.keys = changed;
    // Another look writes other keys with the same keymap.
    harness.app.look = crate::theme::Look::omarchy();
    assert!(keyed.rekey(&items, &harness.app).is_some());
}

#[test]
fn run_all_is_enabled_as_each_edit_leaves_the_script() {
    let (mut harness, tab) = sql_editor();
    let run_all = Item::Command(Command::RunAll);
    let set = |harness: &mut Harness, text: &str| {
        let workspace = harness.app.workspace_mut(tab).unwrap();
        let id = workspace.active_tab.unwrap();
        workspace.sql_tab_mut(id).unwrap().text = text.into();
    };
    // Comments alone run nothing; the answer follows each edit.
    for (text, runs) in [
        ("-- nothing yet", false),
        ("-- nothing yet\nselect 1", true),
        ("-- nothing yet\nselect 1", true),
        ("", false),
    ] {
        set(&mut harness, text);
        assert_eq!(harness.app.menu_state(run_all).enabled, runs, "{text:?}");
    }
}

/// A frame's input as winit gives it: `events`, with the window focused
/// or not and minimized or not.
fn input(events: Vec<Event>, focused: bool, minimized: bool) -> egui::RawInput {
    let mut raw = egui::RawInput {
        focused,
        events,
        ..Default::default()
    };
    let info = raw.viewports.entry(raw.viewport_id).or_default();
    info.minimized = Some(minimized);
    info.occluded = Some(false);
    raw
}

/// A frame's input as eframe's `prepare_raw_input` makes it after a frame
/// of `logic` alone: that frame's input, the hook's events among them,
/// then the new.
fn refed(kept: Vec<Event>, mut new: egui::RawInput) -> egui::RawInput {
    let mut events = kept;
    events.append(&mut new.events);
    new.events = events;
    new
}

#[test]
fn a_minimized_or_unfocused_frame_replays_nothing() {
    let undo = cmd_key(Key::Z, false);
    let mut feed = Feed::default();
    // Clicked: the chord's modifiers held for this frame, then let go.
    let mut raw = input(Vec::new(), true, false);
    assert!(feed.input(&mut raw, &[(undo, Via::Click)]));
    assert_eq!(raw.events.len(), 3, "{:?}", raw.events);
    feed.drawn();
    // Minimized: a frame of `logic` alone. A key and a click are dropped,
    // and the restore waits.
    let mut raw = input(Vec::new(), true, true);
    assert!(!feed.input(&mut raw, &[(undo, Via::Key), (undo, Via::Click)]));
    assert_eq!(raw.events, []);
    // Back, eframe hands the kept input over again: what winit says is
    // held comes back.
    let mut raw = refed(raw.events, input(Vec::new(), true, false));
    assert!(!feed.input(&mut raw, &[]));
    assert_eq!(raw.events, [Event::ModifiersChanged(Modifiers::NONE)]);
    feed.drawn();
    // Not focused: a key down reaches nothing, as without a menu.
    let mut raw = input(Vec::new(), false, false);
    assert!(!feed.input(&mut raw, &[(undo, Via::Key)]));
    assert_eq!(raw.events, []);
}

#[test]
fn a_frame_kept_undrawn_is_not_read_again() {
    let undo = cmd_key(Key::Z, false);
    let mut feed = Feed::default();
    let winit = vec![Event::ModifiersChanged(Modifiers::NONE)];
    let mut raw = input(winit, true, false);
    assert!(feed.input(&mut raw, &[(undo, Via::Click)]));
    let ours = raw.events.clone();
    assert!(
        ours.contains(&Event::ModifiersChanged(held(undo))),
        "{ours:?}"
    );
    // No pass drew it (a redraw the compositor held back): eframe hands
    // it over again, and its events go to egui once, as they were.
    let mut raw = refed(raw.events, input(Vec::new(), true, false));
    assert!(!feed.input(&mut raw, &[(undo, Via::Key)]));
    assert_eq!(raw.events, ours);
    feed.drawn();
    // The next frame lets the chord's modifiers go: what winit said is
    // held, not the replay's.
    let mut raw = input(Vec::new(), true, false);
    assert!(!feed.input(&mut raw, &[]));
    assert_eq!(raw.events, [Event::ModifiersChanged(Modifiers::NONE)]);
}

#[test]
fn exit_and_paste_are_the_window_s_and_the_rest_the_app_s() {
    assert_eq!(
        window_command(Item::Exit),
        Some(egui::ViewportCommand::Close)
    );
    assert_eq!(
        window_command(Item::Edit(EditItem::Paste)),
        Some(egui::ViewportCommand::RequestPaste)
    );
    for platform in [Platform::MacOs, Platform::Windows] {
        for item in items(platform) {
            if !matches!(item, Item::Exit | Item::Edit(EditItem::Paste)) {
                assert_eq!(window_command(item), None, "{item:?}");
            }
        }
    }
}

#[test]
fn about_and_quick_open_clicked_push_their_actions() {
    let mut harness = Harness::new();
    harness.app.actions.clear();
    harness.app.choose(Item::About);
    assert!(
        matches!(harness.app.actions.as_slice(), [Action::ShowAbout]),
        "{:?}",
        harness.app.actions
    );
    harness.app.actions.clear();
    harness.app.choose(Item::QuickOpen);
    assert!(
        matches!(harness.app.actions.as_slice(), [Action::OpenQuickOpen]),
        "{:?}",
        harness.app.actions
    );
}

#[test]
fn a_restore_left_for_an_undrawn_frame_asks_for_a_frame_when_drawn() {
    let undo = cmd_key(Key::Z, false);
    let mut feed = Feed::default();
    let mut raw = input(Vec::new(), true, false);
    assert!(feed.input(&mut raw, &[(undo, Via::Click)]));
    // The frame it asked for came, undrawn, and fed nothing: drawn, the
    // restore still waits, so the bar asks for another frame.
    let mut raw = refed(raw.events, input(Vec::new(), true, false));
    assert!(!feed.input(&mut raw, &[]));
    assert!(feed.drawn());
    let mut raw = input(Vec::new(), true, false);
    assert!(!feed.input(&mut raw, &[]));
    assert_eq!(raw.events, [Event::ModifiersChanged(Modifiers::NONE)]);
    // Nothing waits now.
    assert!(!feed.drawn());
}

#[test]
fn a_paste_chosen_while_the_window_takes_no_keys_is_dropped() {
    let paste = Item::Edit(EditItem::Paste);
    let settings = Item::Command(Command::Settings);
    let clicks = || vec![paste, settings];
    assert_eq!(
        admitted(&input(Vec::new(), true, false), clicks()),
        [paste, settings]
    );
    // Not focused, or minimized: the other clicks still run.
    assert_eq!(
        admitted(&input(Vec::new(), false, false), clicks()),
        [settings]
    );
    assert_eq!(
        admitted(&input(Vec::new(), true, true), clicks()),
        [settings]
    );
}
