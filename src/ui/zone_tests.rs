//! The zone a `timestamptz` value is shown in (Settings › General, Time
//! zone for timestamptz), on the Bookshop's `book_covers.created_at`.
//!
//! Only what is shown and what is copied are checked. No colour, size or
//! picture of the design is.

use egui::{Key, Modifiers};

use crate::backend::Command;
use crate::model::{Action, CellPos, ConnTabId, Pane, TabId};
use crate::settings::{Settings, Zone};
use crate::testing::{COVERS_MADE, Harness, script_outcome, stamped_covers_page};

/// `created_at`, by its place in the page.
const MADE: usize = 4;

fn zone(harness: &mut Harness, zone: Zone) {
    let settings = Settings {
        tz: zone,
        ..harness.app.settings.clone()
    };
    harness.app.apply_settings(settings);
    harness.settle();
}

fn new_york() -> Zone {
    Zone::Named("America/New_York".into())
}

fn fetches(harness: &Harness) -> usize {
    harness
        .app
        .backend
        .sent
        .iter()
        .filter(|command| matches!(command, Command::FetchRows { .. }))
        .count()
}

/// The covers' tab with the cursor on the first row's `created_at`.
fn covers() -> (Harness, ConnTabId, TabId) {
    let mut harness = Harness::new();
    let (tab, id) = harness.stamped_covers();
    harness.app.workspace_mut(tab).unwrap().pane = Pane::Grid;
    harness.app.apply(Action::SelectCell {
        tab,
        id,
        cell: CellPos { row: 0, col: MADE },
    });
    harness.settle();
    (harness, tab, id)
}

#[test]
fn a_moment_is_shown_in_the_zone_and_nothing_is_fetched_again() {
    let (mut harness, _tab, _id) = covers();
    // System is UTC in a test: the moment as the server sent it, to the
    // second.
    assert!(harness.painted_color("2026-01-12 09:14:03+00").is_some());
    let before = fetches(&harness);
    zone(&mut harness, new_york());
    assert!(harness.painted_color("2026-01-12 04:14:03-05").is_some());
    assert!(harness.painted_color("2026-01-12 09:14:03+00").is_none());
    zone(&mut harness, Zone::Named("Asia/Kolkata".into()));
    assert!(harness.painted_color("2026-01-12 14:44:03+05:30").is_some());
    assert_eq!(fetches(&harness), before);
}

#[test]
fn the_system_zone_is_the_one_the_app_found_at_its_start() {
    let mut harness = Harness::new();
    harness.app.system_zone = jiff::tz::db().get("Asia/Tokyo").unwrap();
    harness.stamped_covers();
    harness.settle();
    assert_eq!(harness.app.settings.tz, Zone::System);
    assert!(harness.painted_color("2026-01-12 18:14:03+09").is_some());
    zone(&mut harness, Zone::Utc);
    assert!(harness.painted_color("2026-01-12 09:14:03+00").is_some());
}

#[test]
fn full_precision_keeps_the_fraction_the_server_sent() {
    let (mut harness, _tab, _id) = covers();
    let settings = Settings {
        timestamps: crate::settings::Timestamps::Full,
        tz: new_york(),
        ..harness.app.settings.clone()
    };
    harness.app.apply_settings(settings);
    harness.settle();
    assert!(
        harness
            .painted_color("2026-01-12 04:14:03.482915-05")
            .is_some()
    );
}

#[test]
fn copy_gives_the_value_the_server_sent() {
    let (mut harness, _tab, _id) = covers();
    zone(&mut harness, new_york());
    harness.copy(false);
    assert_eq!(harness.copied.as_deref(), Some(COVERS_MADE));
}

#[test]
fn the_row_panel_shows_the_moment_in_the_zone_and_follows_a_change() {
    let (mut harness, tab, _id) = covers();
    harness.app.workspace_mut(tab).unwrap().row_panel = true;
    harness.settle();
    // The panel writes the whole value: its fraction too.
    assert!(
        harness
            .painted_color("2026-01-12 09:14:03.482915+00")
            .is_some()
    );
    zone(&mut harness, new_york());
    assert!(
        harness
            .painted_color("2026-01-12 04:14:03.482915-05")
            .is_some()
    );
}

#[test]
fn a_text_column_that_reads_as_a_moment_is_left_alone() {
    let mut harness = Harness::new();
    let tab = harness.connect_fake_as(false);
    harness.app.apply(Action::OpenObject {
        tab,
        object: tabletist_db::ObjectRef::new("main", "book_covers"),
        kind: tabletist_db::ObjectKind::Table,
        pin: true,
    });
    harness.answer_structure(crate::testing::book_covers_structure());
    let mut page = crate::testing::book_covers_page(1);
    page.rows[0][MADE] = tabletist_db::Value::Text(COVERS_MADE.into());
    harness.answer_rows(page);
    zone(&mut harness, new_york());
    assert!(harness.painted_color(COVERS_MADE).is_some());
}

#[test]
fn a_sql_result_shows_the_moment_in_the_zone() {
    let mut harness = Harness::new();
    let tab = harness.connect_fake();
    harness.app.workspace_mut(tab).unwrap().driver = tabletist_db::Driver::Postgres;
    harness.press(Key::T, Modifiers::COMMAND);
    harness.frame(vec![egui::Event::Paste(
        "SELECT created_at FROM book_covers".into(),
    )]);
    harness.press(Key::Enter, Modifiers::COMMAND);
    let page = stamped_covers_page(1);
    harness.answer_sql(
        Ok(script_outcome(vec![tabletist_db::StatementOutcome::Rows {
            columns: vec![page.columns[MADE].clone()],
            rows: vec![vec![page.rows[0][MADE].clone()]],
            truncated: false,
        }])),
        None,
    );
    zone(&mut harness, new_york());
    assert!(harness.painted_color("2026-01-12 04:14:03-05").is_some());
}

#[test]
fn a_zone_set_by_hand_in_the_file_shows_at_once() {
    let (mut harness, _tab, _id) = covers();
    harness
        .app
        .apply(Action::Backend(crate::backend::Event::SettingsFile {
            text: "[data]\ntz = \"America/New_York\"\n".into(),
            own: false,
        }));
    harness.settle();
    assert_eq!(harness.app.settings.tz, new_york());
    assert!(harness.painted_color("2026-01-12 04:14:03-05").is_some());
}

#[test]
fn the_row_panel_shows_the_system_zone_too() {
    let mut harness = Harness::new();
    harness.app.system_zone = jiff::tz::db().get("Asia/Tokyo").unwrap();
    let (tab, id) = harness.stamped_covers();
    harness.app.workspace_mut(tab).unwrap().pane = Pane::Grid;
    harness.app.workspace_mut(tab).unwrap().row_panel = true;
    harness.app.apply(Action::SelectCell {
        tab,
        id,
        cell: CellPos { row: 0, col: MADE },
    });
    harness.settle();
    assert!(
        harness
            .painted_color("2026-01-12 18:14:03.482915+09")
            .is_some()
    );
}

/// The covers' tab with its values on SQLite, which keeps a date as the
/// text it was given.
fn covers_on_sqlite() -> (Harness, ConnTabId, TabId) {
    let (mut harness, tab, id) = covers();
    harness.app.workspace_mut(tab).unwrap().driver = tabletist_db::Driver::Sqlite;
    harness.settle();
    (harness, tab, id)
}

/// Opens the editor on `created_at` of the first row, from its value.
fn open_editor(harness: &mut Harness, tab: ConnTabId, id: TabId) {
    harness.app.apply(Action::EditCell {
        tab,
        id,
        cell: CellPos { row: 0, col: MADE },
        start: crate::model::EditStart::Value,
    });
}

fn editor_text(harness: &Harness, tab: ConnTabId, id: TabId) -> String {
    let workspace = harness.app.workspace(tab).unwrap();
    let object = workspace.object_tab(id).unwrap();
    object
        .edits
        .editor
        .as_ref()
        .expect("an editor")
        .text
        .clone()
}

/// Types `text` into the open editor and commits it.
fn commit(harness: &mut Harness, tab: ConnTabId, id: TabId, text: &str) {
    let workspace = harness.app.workspace_mut(tab).unwrap();
    let object = workspace.object_tab_mut(id).unwrap();
    object.edits.editor.as_mut().expect("an editor").text = text.to_owned();
    harness.app.apply(Action::EditorTyped { tab, id });
    harness.app.apply(Action::CommitEdit {
        tab,
        id,
        then: crate::model::Advance::Stay,
    });
}

/// The cell's pending text, where it has one.
fn pending(harness: &Harness, tab: ConnTabId, id: TabId) -> Option<String> {
    let workspace = harness.app.workspace(tab).unwrap();
    let object = workspace.object_tab(id).unwrap();
    match &object.edits.cells.get(&(0, MADE))?.new {
        tabletist_db::NewValue::Text(text) => Some(text.clone()),
        other => panic!("not a text: {other:?}"),
    }
}

#[test]
fn the_editor_opens_on_the_moment_as_the_cell_shows_it() {
    let (mut harness, tab, id) = covers();
    zone(&mut harness, new_york());
    open_editor(&mut harness, tab, id);
    assert_eq!(
        editor_text(&harness, tab, id),
        "2026-01-12 04:14:03.482915-05"
    );
    // Committed as it opened, or as the server wrote it: the moment that
    // was loaded, and no change.
    commit(&mut harness, tab, id, "2026-01-12 04:14:03.482915-05");
    assert_eq!(pending(&harness, tab, id), None);
    open_editor(&mut harness, tab, id);
    commit(&mut harness, tab, id, COVERS_MADE);
    assert_eq!(pending(&harness, tab, id), None);
}

#[test]
fn a_moment_typed_without_an_offset_is_sent_with_the_zones() {
    let (mut harness, tab, id) = covers();
    zone(&mut harness, new_york());
    open_editor(&mut harness, tab, id);
    commit(&mut harness, tab, id, "2026-01-12 06:00:00");
    assert_eq!(
        pending(&harness, tab, id).as_deref(),
        Some("2026-01-12 06:00:00-05")
    );
    // One typed with an offset is sent as typed.
    open_editor(&mut harness, tab, id);
    commit(&mut harness, tab, id, "2026-01-12 06:00:00+01");
    assert_eq!(
        pending(&harness, tab, id).as_deref(),
        Some("2026-01-12 06:00:00+01")
    );
}

#[test]
fn an_engine_that_keeps_no_offset_is_sent_what_was_typed() {
    let (mut harness, tab, id) = covers_on_sqlite();
    zone(&mut harness, new_york());
    open_editor(&mut harness, tab, id);
    commit(&mut harness, tab, id, "2026-01-12 06:00:00");
    assert_eq!(
        pending(&harness, tab, id).as_deref(),
        Some("2026-01-12 06:00:00")
    );
}

#[test]
fn a_text_column_takes_a_moment_written_another_way_as_a_change() {
    // `created_at` is TEXT here: what it holds is text, not a moment.
    let mut harness = Harness::new();
    let tab = harness.connect_fake_as(false);
    harness.app.apply(Action::OpenObject {
        tab,
        object: tabletist_db::ObjectRef::new("main", "book_covers"),
        kind: tabletist_db::ObjectKind::Table,
        pin: true,
    });
    harness.answer_structure(crate::testing::book_covers_structure());
    let mut page = crate::testing::book_covers_page(1);
    page.rows[0][MADE] = tabletist_db::Value::Text(COVERS_MADE.into());
    harness.answer_rows(page);
    let id = harness.app.workspace(tab).unwrap().active_tab.unwrap();
    open_editor(&mut harness, tab, id);
    commit(&mut harness, tab, id, "2026-01-12T09:14:03.482915Z");
    assert_eq!(
        pending(&harness, tab, id).as_deref(),
        Some("2026-01-12T09:14:03.482915Z")
    );
}

/// Gives the keyboard to the field labelled `label`, as a screen reader
/// does: a click through AccessKit presses, and a field takes no press.
fn focus(harness: &mut Harness, label: &str) {
    let tree = harness.settle();
    let (target, _) = tree
        .nodes
        .iter()
        .find(|(_, node)| node.label() == Some(label))
        .unwrap_or_else(|| panic!("nothing labelled {label:?}"));
    let target = *target;
    harness.frame(vec![egui::Event::AccessKitActionRequest(
        egui::accesskit::ActionRequest {
            target_tree: egui::accesskit::TreeId::ROOT,
            target_node: target,
            action: egui::accesskit::Action::Focus,
            data: None,
        },
    )]);
}

/// The Settings sheet of `look`, open.
fn sheet(look: crate::theme::Look) -> Harness {
    let mut harness = Harness::new();
    harness.set_look(look);
    harness.press(Key::Comma, Modifiers::COMMAND);
    assert!(matches!(
        harness.app.dialog,
        Some(crate::model::Dialog::Settings(_))
    ));
    harness
}

#[test]
fn the_sheets_menu_picks_a_zone_found_by_its_name() {
    for look in [crate::theme::Look::macos(), crate::theme::Look::standard()] {
        let mut harness = sheet(look);
        // Its row, and what it says under its label.
        assert!(harness.has("Time zone for timestamptz"), "{}", look.name);
        assert!(
            harness.has("Values are shown in this zone and sent with their offset"),
            "{}",
            look.name
        );
        harness.click("Time zone for timestamptz");
        // The computer's zone with its name, and UTC, first.
        assert!(harness.has("Time zone for timestamptz System · UTC"));
        harness.click("Time zone for timestamptz UTC");
        assert_eq!(harness.app.settings.tz, Zone::Utc, "{}", look.name);
        // A zone by its name: typed into the menu's field, then picked.
        harness.click("Time zone for timestamptz");
        focus(&mut harness, "Search zones");
        harness.frame(vec![egui::Event::Text("lisb".into())]);
        harness.settle();
        assert!(!harness.has("Time zone for timestamptz Asia/Tokyo"));
        harness.click("Time zone for timestamptz Europe/Lisbon");
        assert_eq!(harness.app.settings.tz, Zone::Named("Europe/Lisbon".into()));
        assert!(
            harness
                .app
                .settings_file
                .text
                .contains("tz = \"Europe/Lisbon\"")
        );
        // Opened again, it lists every zone.
        harness.click("Time zone for timestamptz");
        assert!(
            harness.has("Time zone for timestamptz Asia/Tokyo"),
            "{}",
            look.name
        );
    }
}

#[test]
fn the_terminal_screen_names_the_row_as_its_artboard_does() {
    let mut harness = Harness::new();
    harness.set_look(crate::theme::Look::omarchy());
    harness.press(Key::Comma, Modifiers::COMMAND);
    assert!(harness.painted_color("timestamptz zone").is_some());
    assert!(harness.has("timestamptz zone"));
}

#[test]
fn the_file_pane_marks_the_zones_line() {
    let mut harness = Harness::with_size(egui::vec2(1920.0, 1080.0));
    harness.set_look(crate::theme::Look::omarchy());
    harness.press(Key::Comma, Modifiers::COMMAND);
    harness
        .app
        .apply(Action::Backend(crate::backend::Event::SettingsFile {
            text: "[data]\npage_size = 500\ntz = \"utc\"\n".into(),
            own: false,
        }));
    for _ in 0..2 {
        harness.press(Key::J, Modifiers::NONE);
    }
    let selection = harness.app.palette.selection;
    let line = harness.painted_rect("tz = ").expect("the zone's line");
    assert!(harness.fills.iter().any(|(rect, color)| {
        *color == selection
            && rect.y_range().contains(line.center().y)
            && rect.x_range().contains(line.center().x)
    }));
}

#[test]
fn a_step_off_the_files_zone_can_step_back_to_it() {
    let mut harness = Harness::with_size(egui::vec2(1920.0, 1080.0));
    harness.set_look(crate::theme::Look::omarchy());
    harness.press(Key::Comma, Modifiers::COMMAND);
    harness
        .app
        .apply(Action::Backend(crate::backend::Event::SettingsFile {
            text: "[data]\ntz = \"America/New_York\"\n".into(),
            own: false,
        }));
    for _ in 0..2 {
        harness.press(Key::J, Modifiers::NONE);
    }
    harness.press(Key::H, Modifiers::NONE);
    assert_eq!(harness.app.settings.tz, Zone::Utc);
    harness.press(Key::L, Modifiers::NONE);
    assert_eq!(harness.app.settings.tz, new_york());
    assert!(harness.painted_color("‹ america/new_york ›").is_some());
}

#[test]
fn a_zone_the_database_does_not_know_is_reported_in_both_looks() {
    let text = "[data]\ntz = \"Mars/Olympus\"\n";
    // Omarchy: the line in the colour of an error.
    let mut harness = Harness::with_size(egui::vec2(1920.0, 1080.0));
    harness.set_look(crate::theme::Look::omarchy());
    harness.press(Key::Comma, Modifiers::COMMAND);
    harness
        .app
        .apply(Action::Backend(crate::backend::Event::SettingsFile {
            text: text.into(),
            own: false,
        }));
    harness.settle();
    assert_eq!(harness.app.settings.tz, Zone::System);
    let danger = harness.app.palette.danger;
    assert_eq!(harness.painted_color("tz = \"Mars/Olympus\""), Some(danger));
    // macOS: the footer counts it.
    let mut harness = sheet(crate::theme::Look::macos());
    harness
        .app
        .apply(Action::Backend(crate::backend::Event::SettingsFile {
            text: text.into(),
            own: false,
        }));
    harness.settle();
    assert!(harness.has("1 line in the file could not be read and was ignored"));
}

#[test]
fn a_sqlite_text_with_an_offset_is_shown_and_edited_as_stored() {
    let (mut harness, tab, id) = covers_on_sqlite();
    harness.app.workspace_mut(tab).unwrap().row_panel = true;
    zone(&mut harness, new_york());
    // The grid, to the second, and the row panel, whole: as stored.
    assert!(harness.painted_color("2026-01-12 09:14:03+00").is_some());
    assert!(harness.painted_color(COVERS_MADE).is_some());
    assert!(harness.painted_color("2026-01-12 04:14:03-05").is_none());
    // The editor opens on the text, and the same moment written another
    // way is another text.
    open_editor(&mut harness, tab, id);
    assert_eq!(editor_text(&harness, tab, id), COVERS_MADE);
    commit(&mut harness, tab, id, "2026-01-12 04:14:03.482915-05");
    assert_eq!(
        pending(&harness, tab, id).as_deref(),
        Some("2026-01-12 04:14:03.482915-05")
    );
}

#[test]
fn the_zone_database_is_the_backends_to_read() {
    let harness = Harness::new();
    assert!(
        harness
            .app
            .backend
            .sent
            .iter()
            .any(|command| matches!(command, Command::LoadZones))
    );
}

#[test]
fn a_named_zone_shows_values_as_sent_until_the_database_is_read() {
    let mut harness = Harness::new();
    harness.app.zones = None;
    harness
        .app
        .apply(Action::Backend(crate::backend::Event::SettingsFile {
            text: "[data]\ntz = \"America/New_York\"\n".into(),
            own: false,
        }));
    harness.stamped_covers();
    harness.settle();
    assert!(harness.painted_color("2026-01-12 09:14:03+00").is_some());
    harness
        .app
        .apply(Action::Backend(crate::backend::Event::Zones(
            std::sync::Arc::new(crate::testing::zones().clone()),
        )));
    harness.settle();
    assert!(harness.painted_color("2026-01-12 04:14:03-05").is_some());
}

#[test]
fn a_zone_the_database_does_not_know_is_ignored_once_it_is_read() {
    let mut harness = Harness::new();
    harness.app.zones = None;
    harness
        .app
        .apply(Action::Backend(crate::backend::Event::SettingsFile {
            text: "[data]\npage_size = 500\ntz = \"Mars/Olympus\"\n".into(),
            own: false,
        }));
    // Of a zone's form: taken until the database says otherwise.
    assert_eq!(harness.app.settings.tz, Zone::Named("Mars/Olympus".into()));
    assert!(harness.app.settings_file.invalid.is_empty());
    harness
        .app
        .apply(Action::Backend(crate::backend::Event::Zones(
            std::sync::Arc::new(crate::testing::zones().clone()),
        )));
    assert_eq!(harness.app.settings.tz, Zone::System);
    assert_eq!(harness.app.settings_file.invalid, [3]);
    assert_eq!(harness.app.settings.page_size, 500);
    // Nothing is written: the line stays the person's to mend.
    assert!(
        !harness
            .app
            .backend
            .sent
            .iter()
            .any(|command| matches!(command, Command::Save { .. }))
    );
}

#[test]
fn the_menu_lists_no_zones_before_the_database_is_read() {
    let mut harness = sheet(crate::theme::Look::macos());
    harness.app.zones = None;
    harness.click("Time zone for timestamptz");
    assert!(harness.has("Time zone for timestamptz UTC"));
    assert!(!harness.has("Time zone for timestamptz Asia/Tokyo"));
}

#[test]
fn a_zone_the_database_turned_down_is_no_step_of_the_terminal_screen() {
    let mut harness = Harness::with_size(egui::vec2(1920.0, 1080.0));
    harness.set_look(crate::theme::Look::omarchy());
    harness.app.zones = None;
    harness
        .app
        .apply(Action::Backend(crate::backend::Event::SettingsFile {
            text: "[data]\ntz = \"Mars/Olympus\"\n".into(),
            own: false,
        }));
    harness
        .app
        .apply(Action::Backend(crate::backend::Event::Zones(
            std::sync::Arc::new(crate::testing::zones().clone()),
        )));
    assert_eq!(harness.app.settings.tz, Zone::System);
    assert_eq!(harness.app.named_zone, None);
    harness.press(Key::Comma, Modifiers::COMMAND);
    for _ in 0..2 {
        harness.press(Key::J, Modifiers::NONE);
    }
    harness.press(Key::L, Modifiers::NONE);
    assert_eq!(harness.app.settings.tz, Zone::Utc);
    // From UTC there is no step further: the refused name is not one.
    harness.press(Key::L, Modifiers::NONE);
    assert_eq!(harness.app.settings.tz, Zone::Utc);
    assert!(!harness.app.settings_file.text.contains("Mars/Olympus"));
}
