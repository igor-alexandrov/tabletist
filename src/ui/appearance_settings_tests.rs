//! Settings › Appearance, in both looks: what each option does to the
//! grids and the sidebar's list, and the pane that sets them with a click
//! or a key and writes them to the file.

use egui::accesskit::Role;

use crate::backend::{Command, StateFile};
use crate::model::{Action, ConnTabId, Dialog, SettingsPane, TabId};
use crate::settings::{EnvBar, OptionValue, RowHeight, Settings, TableList};
use crate::testing::{Harness, bounds};
use crate::theme::Look;

fn looks() -> [Look; 2] {
    [Look::macos(), Look::omarchy()]
}

/// The settings changed by `change`, as an edit of the file changes them.
fn change(harness: &mut Harness, change: impl FnOnce(&mut Settings)) {
    let mut settings = harness.app.settings.clone();
    change(&mut settings);
    harness.app.apply_settings(settings);
    harness.settle();
    // A scroll asked for lands two frames later.
    harness.settle();
}

/// `book_covers` with `rows` rows, in `look`.
fn covers(look: Look, rows: usize) -> (Harness, ConnTabId, TabId) {
    let mut harness = Harness::new();
    harness.set_look(look);
    let tab = harness.connect_fake_as(false);
    harness.app.apply(Action::OpenObject {
        tab,
        object: tabletist_db::ObjectRef::new("main", "book_covers"),
        kind: tabletist_db::ObjectKind::Table,
        pin: true,
    });
    harness.answer_structure(crate::testing::book_covers_structure());
    harness.answer_rows(crate::testing::book_covers_page(rows));
    harness.settle();
    let id = harness.app.workspace(tab).unwrap().active_tab.unwrap();
    (harness, tab, id)
}

/// Whether the last frame painted a text that has `words` in it.
fn shows(harness: &mut Harness, words: &str) -> bool {
    harness.settle();
    harness.painted.iter().any(|(text, _)| text.contains(words))
}

/// The grid's rows as drawn: each row's number and its place.
fn rows(harness: &mut Harness) -> Vec<(u64, egui::Rect)> {
    let tree = harness.settle();
    (1..=500)
        .filter_map(|number| {
            bounds(&tree, &format!("Row {number}"), Role::Button).map(|rect| (number, rect))
        })
        .collect()
}

/// The rows drawn on the panel's colour: the stripes.
fn shaded(harness: &mut Harness) -> Vec<u64> {
    let panel = harness.app.palette.panel;
    let rows = rows(harness);
    rows.into_iter()
        .filter(|(_, rect)| {
            harness
                .fills
                .iter()
                .any(|(fill, color)| *color == panel && fill.contains_rect(*rect))
        })
        .map(|(number, _)| number)
        .collect()
}

/// The grid's own place: its header's top.
fn grid_top(harness: &mut Harness) -> f32 {
    let tree = harness.settle();
    bounds(&tree, "Rows", Role::Group).expect("the grid").top()
}

#[test]
fn row_density_sets_the_rows_height_in_each_look() {
    for look in looks() {
        let (mut harness, _, _) = covers(look, 3);
        // With no file: today's rows.
        let regular = if look.terminal { 30.0 } else { 37.0 };
        for (_, rect) in rows(&mut harness) {
            assert_eq!(rect.height(), regular, "{}", look.name);
        }
        for (height, expected) in [
            (RowHeight::Compact, if look.terminal { 20.0 } else { 29.0 }),
            (RowHeight::Roomy, if look.terminal { 40.0 } else { 45.0 }),
            (RowHeight::Regular, regular),
        ] {
            change(&mut harness, |settings| settings.row_height = height);
            let rows = rows(&mut harness);
            assert_eq!(rows.len(), 3, "{}", look.name);
            for (_, rect) in rows {
                assert_eq!(rect.height(), expected, "{} {height:?}", look.name);
            }
        }
    }
}

#[test]
fn row_density_keeps_the_top_row_where_it_was() {
    for look in looks() {
        let (mut harness, _, _) = covers(look, 300);
        let middle = harness.size / 2.0;
        for _ in 0..10 {
            harness.frame(vec![
                egui::Event::PointerMoved(middle.to_pos2()),
                egui::Event::MouseWheel {
                    unit: egui::MouseWheelUnit::Point,
                    delta: egui::vec2(0.0, -400.0),
                    modifiers: egui::Modifiers::NONE,
                    phase: egui::TouchPhase::Move,
                },
            ]);
        }
        // The wheel scrolls smoothly: until it has stopped.
        for _ in 0..5 {
            harness.finish_animations();
        }
        // The row at the top: the first one under the sticky header.
        let under = grid_top(&mut harness) + crate::ui::grid::header_height(&look) + 1.0;
        let top = |harness: &mut Harness| {
            rows(harness)
                .into_iter()
                .find(|(_, rect)| rect.bottom() > under)
                .map(|(number, _)| number)
                .expect("a row at the top")
        };
        let before = top(&mut harness);
        assert!(before > 50, "{} scrolled to {before}", look.name);
        for height in [RowHeight::Roomy, RowHeight::Compact, RowHeight::Regular] {
            change(&mut harness, |settings| settings.row_height = height);
            assert_eq!(top(&mut harness), before, "{} {height:?}", look.name);
        }
    }
}

#[test]
fn alternate_rows_shade_the_odd_rows_where_they_are_on() {
    for look in looks() {
        let (mut harness, _, _) = covers(look, 4);
        // As drawn: off on macOS, on in the terminal.
        let default: Vec<u64> = if look.terminal { vec![2, 4] } else { vec![] };
        assert_eq!(shaded(&mut harness), default, "{}", look.name);
        change(&mut harness, |settings| settings.zebra = Some(true));
        assert_eq!(shaded(&mut harness), vec![2, 4], "{}", look.name);
        change(&mut harness, |settings| settings.zebra = Some(false));
        assert_eq!(shaded(&mut harness), Vec::<u64>::new(), "{}", look.name);
    }
}

#[test]
fn the_sql_editors_results_take_the_density_and_the_stripes() {
    for look in looks() {
        let mut harness = Harness::new();
        harness.set_look(look);
        let tab = harness.connect_fake();
        super::tests::with_sql_result(&mut harness, tab, 4);
        change(&mut harness, |settings| {
            settings.row_height = RowHeight::Roomy;
            settings.zebra = Some(true);
        });
        let rows = rows(&mut harness);
        assert_eq!(rows.len(), 4, "{}", look.name);
        let roomy = if look.terminal { 40.0 } else { 45.0 };
        for (_, rect) in rows {
            assert_eq!(rect.height(), roomy, "{}", look.name);
        }
        assert_eq!(shaded(&mut harness), vec![2, 4], "{}", look.name);
    }
}

#[test]
fn without_column_types_the_header_is_one_line() {
    for look in looks() {
        let (mut harness, _, _) = covers(look, 3);
        let header = |harness: &mut Harness| {
            let top = grid_top(harness);
            rows(harness)[0].1.top() - top
        };
        let near = |height: f32, expected: f32| (height - expected).abs() < 1.0;
        let two = header(&mut harness);
        let expected = if look.terminal { 41.0 } else { 45.0 };
        assert!(near(two, expected), "{} {two}", look.name);
        // The id column's type, or the key the terminal writes for it.
        let typed = |harness: &mut Harness| {
            harness.settle();
            harness
                .painted
                .iter()
                .any(|(text, _)| text.eq_ignore_ascii_case("integer") || text == "pk")
        };
        assert!(typed(&mut harness), "{} {:?}", look.name, harness.painted);
        change(&mut harness, |settings| settings.column_types = false);
        let one = header(&mut harness);
        let expected = if look.terminal { 27.0 } else { 31.0 };
        assert!(near(one, expected), "{} {one}", look.name);
        assert!(shows(&mut harness, "publisher_id"), "{}", look.name);
        assert!(!typed(&mut harness), "{}", look.name);
    }
}

#[test]
fn the_table_list_default_applies_to_connections_without_a_choice() {
    let mut harness = Harness::new();
    let tab = harness.connect_fake();
    let flat = |harness: &Harness, tab| harness.app.workspace(tab).unwrap().tree.flat;
    assert!(!flat(&harness, tab), "a tree until set");
    change(&mut harness, |settings| {
        settings.table_list = TableList::Flat
    });
    assert!(flat(&harness, tab), "the open connection follows");
    // The sidebar's toggle is the connection's choice, and saved.
    harness.app.apply(Action::ToggleFlatTree(tab));
    assert!(!flat(&harness, tab));
    let conn = harness.app.workspace(tab).unwrap().conn_id.clone();
    assert_eq!(harness.app.table_lists.get(&conn.0), Some(&TableList::Tree));
    assert!(harness.app.backend.sent.iter().any(|command| matches!(
        command,
        Command::Save { file: StateFile::TableLists(lists), path }
            if lists.get(&conn.0) == Some(&serde_json::json!("tree"))
                && *path == harness.app.dirs.table_lists_file()
    )));
    // The setting no longer moves it, there or in a new window on it.
    change(&mut harness, |settings| {
        settings.table_list = TableList::Tree
    });
    change(&mut harness, |settings| {
        settings.table_list = TableList::Flat
    });
    assert!(!flat(&harness, tab));
    harness.app.apply(Action::Connect {
        tab,
        conn: conn.clone(),
    });
    assert!(!flat(&harness, tab), "opened again");
    // Another connection, with no choice, takes the setting's.
    let other = harness.connect_fake();
    assert!(flat(&harness, other));
    // Deleting the connection forgets it.
    harness.app.apply(Action::DeleteConnection(conn.clone()));
    assert!(!harness.app.table_lists.contains_key(&conn.0));
}

#[test]
fn a_saved_table_list_is_read_back_at_a_start() {
    let dir = tempfile::tempdir().unwrap();
    let dirs = crate::paths::AppDirs::at(dir.path());
    dirs.ensure().unwrap();
    let lists = std::collections::BTreeMap::from([("green".to_owned(), TableList::Flat)]);
    // As the backend saves it.
    crate::util::save_json(&dirs.table_lists_file(), &lists).unwrap();
    let app = crate::app::App::new(
        dirs,
        Settings::default().into(),
        crate::backend::Backend::recording(),
    );
    assert_eq!(app.table_lists, lists);
    assert!(app.lists_flat(&crate::connections::ConnectionId("green".into())));
    assert!(!app.lists_flat(&crate::connections::ConnectionId("blue".into())));
}

#[test]
fn a_connection_open_twice_lists_its_tables_alike() {
    let mut harness = Harness::new();
    let first = harness.connect_fake();
    let conn = harness.app.workspace(first).unwrap().conn_id.clone();
    harness.app.apply(Action::ShowConnections);
    let second = harness.app.active_tab_id();
    harness.app.apply(Action::Connect {
        tab: second,
        conn: conn.clone(),
    });
    assert_ne!(first, second);
    harness.app.apply(Action::ToggleFlatTree(first));
    for tab in [first, second] {
        assert!(harness.app.workspace(tab).unwrap().tree.flat, "{tab:?}");
    }
}

#[test]
fn a_connection_that_is_not_saved_keeps_its_list_while_the_app_runs() {
    let mut harness = Harness::new();
    let tab = harness.connect_fake();
    let conn = harness.app.workspace(tab).unwrap().conn_id.clone();
    harness.app.connections.remove(&conn);
    harness.app.apply(Action::ToggleFlatTree(tab));
    // Not written: the file is the saved connections'.
    let written = harness
        .app
        .backend
        .sent
        .iter()
        .rev()
        .find_map(|command| match command {
            Command::Save {
                file: StateFile::TableLists(lists),
                ..
            } => Some(lists.clone()),
            _ => None,
        });
    assert_eq!(written, Some(std::collections::BTreeMap::new()));
    // And the setting does not move it.
    change(&mut harness, |settings| {
        settings.table_list = TableList::Flat
    });
    change(&mut harness, |settings| {
        settings.table_list = TableList::Tree
    });
    assert!(harness.app.workspace(tab).unwrap().tree.flat);
}

#[test]
fn a_list_this_version_does_not_know_costs_only_its_connection() {
    let dir = tempfile::tempdir().unwrap();
    let dirs = crate::paths::AppDirs::at(dir.path());
    dirs.ensure().unwrap();
    std::fs::write(
        dirs.table_lists_file(),
        br#"{"green": "flat", "blue": "grouped", "red": 3}"#,
    )
    .unwrap();
    let app = crate::app::App::new(
        dirs,
        Settings::default().into(),
        crate::backend::Backend::recording(),
    );
    let expected = std::collections::BTreeMap::from([("green".to_owned(), TableList::Flat)]);
    assert_eq!(app.table_lists, expected);
}

#[test]
fn another_database_lists_its_tables_as_the_one_before() {
    let mut harness = Harness::new();
    let tab = harness.connect_fake();
    harness.app.apply(Action::ToggleFlatTree(tab));
    harness.app.apply(Action::SwitchDatabase {
        tab,
        database: "bookshop_review".into(),
    });
    assert!(harness.app.workspace(tab).unwrap().tree.flat);
}

/// The pane Settings shows, while it is open.
fn pane(harness: &Harness) -> Option<SettingsPane> {
    match &harness.app.dialog {
        Some(Dialog::Settings(dialog)) => Some(dialog.pane),
        _ => None,
    }
}

/// How many times the settings were saved.
fn settings_saves(harness: &Harness) -> usize {
    harness
        .app
        .backend
        .sent
        .iter()
        .filter(|command| {
            matches!(
                command,
                Command::Save {
                    file: StateFile::Settings(_),
                    ..
                }
            )
        })
        .count()
}

/// Settings on its Appearance pane.
fn on_the_pane(harness: &mut Harness) {
    harness.app.apply(Action::ShowSettings);
    harness
        .app
        .apply(Action::ShowSettingsPane(SettingsPane::Appearance));
    harness.settle();
}

#[test]
fn settings_lists_an_appearance_pane_after_general() {
    for look in looks() {
        let mut harness = Harness::new();
        harness.set_look(look);
        harness.app.apply(Action::ShowSettings);
        harness.settle();
        if look.terminal {
            harness.press(egui::Key::L, egui::Modifiers::CTRL);
        } else {
            harness.click("Appearance");
        }
        harness.settle();
        assert_eq!(
            pane(&harness),
            Some(SettingsPane::Appearance),
            "{}",
            look.name
        );
        let first = if look.terminal {
            "Row height"
        } else {
            "Row density"
        };
        assert!(harness.has(first), "{}", look.name);
        assert!(!harness.has("Rows per page"), "{}", look.name);
    }
}

#[test]
fn the_sheet_sets_each_appearance_option_with_a_click() {
    let mut harness = Harness::new();
    harness.set_look(Look::macos());
    on_the_pane(&mut harness);
    // The board's words.
    for words in [
        "GRID",
        "ENVIRONMENT COLOR",
        "Second line in headers: int8, varchar, jsonb",
        "Default for new connections; the sidebar toggle overrides it",
        "Production always uses the full tint",
    ] {
        assert!(shows(&mut harness, words), "{words}");
    }
    let saves = settings_saves(&harness);
    harness.click("Row density: Compact · 28");
    harness.click("Alternate row shading");
    harness.click("Show column types");
    harness.click("Table list: Flat");
    harness.click("Window header: Stripe only");
    harness.settle();
    let settings = &harness.app.settings;
    assert_eq!(settings.row_height, RowHeight::Compact);
    assert_eq!(settings.zebra, Some(true), "off on macOS until set");
    assert!(!settings.column_types);
    assert_eq!(settings.table_list, TableList::Flat);
    assert_eq!(settings.env_bar, EnvBar::Label);
    assert_eq!(settings_saves(&harness), saves + 5);
}

#[test]
fn the_screen_sets_each_appearance_option_with_its_keys() {
    let mut harness = Harness::new();
    harness.set_look(Look::omarchy());
    on_the_pane(&mut harness);
    for words in [
        "grid",
        "environment",
        "1 · 1.5 · 2 lines",
        "second header line",
        "toggles",
        "PROD always tinted",
    ] {
        assert!(shows(&mut harness, words), "{words}");
    }
    let saves = settings_saves(&harness);
    let key = |harness: &mut Harness, key: egui::Key, text: &str| {
        super::tests::type_key(harness, key, text);
    };
    // Row height: one to the left, compact.
    key(&mut harness, egui::Key::H, "h");
    // Alternate rows, on in the terminal until set: space turns it off.
    key(&mut harness, egui::Key::J, "j");
    harness.press(egui::Key::Space, egui::Modifiers::NONE);
    // Column types off, the list flat, the bar's label alone.
    key(&mut harness, egui::Key::J, "j");
    harness.press(egui::Key::Space, egui::Modifiers::NONE);
    key(&mut harness, egui::Key::J, "j");
    key(&mut harness, egui::Key::L, "l");
    key(&mut harness, egui::Key::J, "j");
    key(&mut harness, egui::Key::L, "l");
    harness.settle();
    let settings = &harness.app.settings;
    assert_eq!(settings.row_height, RowHeight::Compact);
    assert_eq!(settings.zebra, Some(false));
    assert!(!settings.column_types);
    assert_eq!(settings.table_list, TableList::Flat);
    assert_eq!(settings.env_bar, EnvBar::Label);
    assert_eq!(settings_saves(&harness), saves + 5);
    // R puts the bar back, and the stripes back to the look's.
    harness.press(egui::Key::R, egui::Modifiers::SHIFT);
    for _ in 0..3 {
        key(&mut harness, egui::Key::K, "k");
    }
    harness.press(egui::Key::R, egui::Modifiers::SHIFT);
    harness.settle();
    assert_eq!(harness.app.settings.env_bar, EnvBar::Tint);
    assert_eq!(harness.app.settings.zebra, None);
}

#[test]
fn the_density_segments_name_the_rows_of_their_look() {
    // Windows draws the sheet too, with its own rows.
    for (look, heights) in [
        (Look::macos(), ["28", "36", "44"]),
        (Look::standard(), ["17", "25", "33"]),
    ] {
        let mut harness = Harness::new();
        harness.set_look(look);
        on_the_pane(&mut harness);
        for (word, height) in ["Compact", "Regular", "Roomy"].into_iter().zip(heights) {
            let name = format!("Row density: {word} · {height}");
            assert!(harness.has(&name), "{} {name}", look.name);
        }
    }
}

#[test]
fn reset_to_defaults_puts_the_appearance_options_back() {
    let mut harness = Harness::new();
    harness.set_look(Look::macos());
    for value in [
        OptionValue::RowHeight(RowHeight::Roomy),
        OptionValue::Zebra(Some(true)),
        OptionValue::ColumnTypes(false),
        OptionValue::TableList(TableList::Flat),
        OptionValue::EnvBar(EnvBar::Label),
        OptionValue::PageSize(500),
    ] {
        harness.app.apply(Action::SetOption(value));
    }
    on_the_pane(&mut harness);
    harness.app.apply(Action::ResetSettings);
    harness.app.apply(Action::ConfirmResetSettings(true));
    harness.settle();
    let settings = &harness.app.settings;
    let defaults = Settings::default();
    assert_eq!(settings.row_height, defaults.row_height);
    assert_eq!(settings.zebra, None);
    assert_eq!(settings.column_types, defaults.column_types);
    assert_eq!(settings.table_list, defaults.table_list);
    assert_eq!(settings.env_bar, defaults.env_bar);
    // Another pane's options are its own.
    assert_eq!(settings.page_size, 500);
}

#[test]
fn a_reset_and_a_toggle_in_one_frame_turn_the_look_s_stripes_off() {
    let mut harness = Harness::new();
    harness.set_look(Look::omarchy());
    on_the_pane(&mut harness);
    super::tests::type_key(&mut harness, egui::Key::J, "j");
    change(&mut harness, |settings| settings.zebra = Some(true));
    // R puts them back to the look's, which shows them; Space then turns
    // them off.
    harness.frame(vec![
        crate::testing::key(egui::Key::R, egui::Modifiers::SHIFT),
        egui::Event::Text("R".into()),
        crate::testing::key(egui::Key::Space, egui::Modifiers::NONE),
    ]);
    harness.settle();
    assert_eq!(harness.app.settings.zebra, Some(false));
}
