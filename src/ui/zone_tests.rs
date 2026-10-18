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
    let _tab = harness.connect_fake();
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
