//! The line under the editor of a date or a time: which columns have it,
//! what it says, what a commit stores, and the choices a change of the
//! clocks offers. On the Bookshop's `book_covers.created_at`, its type
//! changed for each test, with the clock stopped on Saturday 24 October
//! 2026 at 10:00 in New York.
//!
//! Only what is said and what is stored are checked. No colour, size or
//! picture of the design is.

use egui::accesskit::Role;
use egui::{Key, Modifiers};
use tabletist_db::{Driver, NewValue, Value, ValueKind};

use crate::model::{Action, CellPos, ConnTabId, EditStart, Pane, TabId};
use crate::settings::{Settings, Zone};
use crate::testing::{Harness, labels};

/// `created_at`, by its place in the page.
const MADE: usize = 4;

/// Saturday 24 October 2026, 10:00 in New York.
fn saturday() -> jiff::Timestamp {
    jiff::civil::date(2026, 10, 24)
        .at(10, 0, 0, 0)
        .to_zoned(jiff::tz::TimeZone::get("America/New_York").unwrap())
        .unwrap()
        .timestamp()
}

fn in_zone(harness: &mut Harness, name: &str) {
    let settings = Settings {
        tz: Zone::Named(name.into()),
        ..harness.app.settings.clone()
    };
    harness.app.apply_settings(settings);
    harness.settle();
}

/// The covers' tab on `driver`, `created_at` of the type `type_name` and
/// holding `values` (the first row's first), shown in New York, with the
/// cursor on the first row's `created_at`.
fn covers_with(driver: Driver, type_name: &str, values: &[Value]) -> (Harness, ConnTabId, TabId) {
    let mut harness = Harness::new();
    harness.app.stopped_clock = Some(saturday());
    let tab = harness.connect_fake_as(false);
    harness.app.workspace_mut(tab).unwrap().driver = driver;
    harness.app.apply(Action::OpenObject {
        tab,
        object: tabletist_db::ObjectRef::new("main", "book_covers"),
        kind: tabletist_db::ObjectKind::Table,
        pin: true,
    });
    let mut structure = crate::testing::book_covers_structure();
    structure.columns[MADE].type_name = type_name.into();
    harness.answer_structure(structure);
    let mut page = crate::testing::book_covers_page(values.len().max(1));
    page.columns[MADE].type_name = type_name.into();
    page.columns[MADE].kind = ValueKind::Temporal;
    for (row, value) in page.rows.iter_mut().zip(values) {
        row[MADE] = value.clone();
    }
    harness.answer_rows(page);
    let id = harness.app.workspace(tab).unwrap().active_tab.unwrap();
    harness.app.workspace_mut(tab).unwrap().pane = Pane::Grid;
    in_zone(&mut harness, "America/New_York");
    harness.app.apply(Action::SelectCell {
        tab,
        id,
        cell: CellPos { row: 0, col: MADE },
    });
    harness.settle();
    (harness, tab, id)
}

/// [`covers_with`], `created_at` a foreign key to the Bookshop's
/// `orders.placed_at`.
fn keyed_covers(driver: Driver, type_name: &str, value: &str) -> (Harness, ConnTabId, TabId) {
    let (mut harness, tab, id) = covers(driver, type_name, value);
    let object = harness
        .app
        .workspace_mut(tab)
        .unwrap()
        .object_tab_mut(id)
        .unwrap();
    let structure = object.structure.value.as_mut().unwrap();
    structure.foreign_keys = vec![tabletist_db::ForeignKeyInfo {
        columns: vec!["created_at".into()],
        ref_schema: "main".into(),
        ref_table: "orders".into(),
        ref_columns: vec!["placed_at".into()],
        ..tabletist_db::ForeignKeyInfo::default()
    }];
    harness.settle();
    (harness, tab, id)
}

fn covers(driver: Driver, type_name: &str, value: &str) -> (Harness, ConnTabId, TabId) {
    covers_with(driver, type_name, &[Value::Text(value.into())])
}

fn open(harness: &mut Harness, tab: ConnTabId, id: TabId) {
    harness.app.apply(Action::EditCell {
        tab,
        id,
        cell: CellPos { row: 0, col: MADE },
        start: EditStart::Value,
    });
    harness.settle();
}

/// Opens the editor and types `text` in the place of its own.
fn type_in(harness: &mut Harness, tab: ConnTabId, id: TabId, text: &str) {
    open(harness, tab, id);
    editor_mut(harness, tab, id).text = text.to_owned();
    harness.app.apply(Action::EditorTyped { tab, id });
    harness.settle();
}

fn editor_mut(harness: &mut Harness, tab: ConnTabId, id: TabId) -> &mut crate::edit::Editor {
    let workspace = harness.app.workspace_mut(tab).unwrap();
    let object = workspace.object_tab_mut(id).unwrap();
    object.edits.editor.as_mut().expect("an editor")
}

fn editor_text(harness: &Harness, tab: ConnTabId, id: TabId) -> Option<String> {
    let object = harness.app.workspace(tab)?.object_tab(id)?;
    Some(object.edits.editor.as_ref()?.text.clone())
}

/// The cell's pending value, where it has one.
fn pending(harness: &Harness, tab: ConnTabId, id: TabId) -> Option<NewValue> {
    let object = harness.app.workspace(tab)?.object_tab(id)?;
    Some(object.edits.cells.get(&(0, MADE))?.new.clone())
}

fn pending_text(harness: &Harness, tab: ConnTabId, id: TabId) -> Option<String> {
    match pending(harness, tab, id)? {
        NewValue::Text(text) => Some(text),
        other => panic!("not a text: {other:?}"),
    }
}

fn says(harness: &mut Harness, words: &str) -> bool {
    harness.has(words)
}

#[test]
fn each_kind_of_date_or_time_column_has_the_line_and_no_other_does() {
    use Driver::{MySql, Postgres, Sqlite};
    for (driver, type_name, value, named) in [
        (Postgres, "date", "2026-01-15", Some("created_at, date")),
        (
            Postgres,
            "timestamp without time zone",
            "2026-01-15 09:30:00",
            Some("created_at, date and time"),
        ),
        (
            Postgres,
            "timestamp with time zone",
            "2026-01-15 14:30:00+00",
            Some("created_at, date and time"),
        ),
        (
            Postgres,
            "time without time zone",
            "09:30:00",
            Some("created_at, time"),
        ),
        (
            Postgres,
            "time with time zone",
            "09:30:00-05",
            Some("created_at, time"),
        ),
        (MySql, "date", "2026-01-15", Some("created_at, date")),
        (
            MySql,
            "datetime",
            "2026-01-15 09:30:00",
            Some("created_at, date and time"),
        ),
        (
            MySql,
            "timestamp",
            "2026-01-15 09:30:00",
            Some("created_at, date and time"),
        ),
        (MySql, "time", "09:30:00", Some("created_at, time")),
        (Sqlite, "DATE", "2026-01-15", Some("created_at, date")),
        (
            Sqlite,
            "DATETIME",
            "2026-01-15 09:30:00",
            Some("created_at, date and time"),
        ),
        (Sqlite, "TIME", "09:30:00", Some("created_at, time")),
        (Postgres, "interval", "1 day", None),
        (Postgres, "date[]", "{2026-01-15}", None),
        (MySql, "year", "2026", None),
    ] {
        let (mut harness, tab, id) = covers(driver, type_name, value);
        open(&mut harness, tab, id);
        let tree = harness.settle();
        let dialog = tree
            .nodes
            .iter()
            .filter(|(_, node)| node.role() == Role::Dialog)
            .find_map(|(_, node)| {
                node.label()
                    .filter(|label| label.starts_with("created_at, "))
            });
        assert_eq!(dialog, named, "{driver:?} {type_name}");
    }
}

#[test]
fn the_line_says_what_a_commit_stores() {
    let (mut harness, tab, id) = covers(Driver::Postgres, "timestamptz", "2026-01-15 14:30:00+00");
    open(&mut harness, tab, id);
    // Opened on the value as the cell shows it, in New York, with its
    // offset: what is sent.
    assert_eq!(
        editor_text(&harness, tab, id).as_deref(),
        Some("2026-01-15 09:30:00-05")
    );
    assert!(says(&mut harness, "Thu 15 Jan 2026, 09:30 EST"));
    harness.press(Key::Escape, Modifiers::NONE);
    type_in(&mut harness, tab, id, "tomorrow 9am");
    let said = "Sun 25 Oct 2026, 09:00 EDT · tomorrow · sent as 2026-10-25 09:00:00-04";
    assert!(says(&mut harness, said), "{:?}", labels(&harness.settle()));
    // The clock moves on before the commit: what is stored is what was said.
    harness.app.stopped_clock = Some(saturday() + jiff::SignedDuration::from_hours(15));
    harness.press(Key::Enter, Modifiers::NONE);
    assert_eq!(editor_text(&harness, tab, id), None);
    assert_eq!(
        pending_text(&harness, tab, id).as_deref(),
        Some("2026-10-25 09:00:00-04")
    );
}

#[test]
fn now_alone_is_the_time_by_the_clock_and_now_with_brackets_the_databases() {
    let (mut harness, tab, id) = covers(Driver::Postgres, "timestamptz", "2026-01-15 14:30:00+00");
    type_in(&mut harness, tab, id, "now");
    harness.press(Key::Enter, Modifiers::NONE);
    assert_eq!(
        pending_text(&harness, tab, id).as_deref(),
        Some("2026-10-24 10:00:00-04")
    );
    type_in(&mut harness, tab, id, "now()");
    harness.press(Key::Enter, Modifiers::NONE);
    assert_eq!(pending(&harness, tab, id), Some(NewValue::Now));
}

#[test]
fn a_text_that_is_no_date_is_refused_and_esc_keeps_the_cell() {
    let (mut harness, tab, id) = covers(Driver::Postgres, "date", "2026-01-15");
    type_in(&mut harness, tab, id, "2026-02-30");
    assert!(says(&mut harness, "There is no 30 February."));
    harness.press(Key::Enter, Modifiers::NONE);
    assert_eq!(
        editor_text(&harness, tab, id).as_deref(),
        Some("2026-02-30")
    );
    assert_eq!(pending(&harness, tab, id), None);
    harness.press(Key::Escape, Modifiers::NONE);
    assert_eq!(editor_text(&harness, tab, id), None);
    assert_eq!(pending(&harness, tab, id), None);
    let (mut harness, tab, id) = covers(Driver::Postgres, "time", "09:30:00");
    type_in(&mut harness, tab, id, "25:00");
    assert!(says(&mut harness, "25:00 is not a time of day."));
    harness.press(Key::Enter, Modifiers::NONE);
    assert!(editor_text(&harness, tab, id).is_some());
}

#[test]
fn enter_applies_and_a_click_away_applies_too() {
    let (mut harness, tab, id) = covers(Driver::Postgres, "date", "2026-01-15");
    type_in(&mut harness, tab, id, "15 Feb 2026");
    harness.press(Key::Enter, Modifiers::NONE);
    assert_eq!(
        pending_text(&harness, tab, id).as_deref(),
        Some("2026-02-15")
    );
    type_in(&mut harness, tab, id, "16 Feb 2026");
    harness.app.apply(Action::LeaveEdit {
        tab,
        id,
        cell: CellPos { row: 0, col: MADE },
        place: crate::edit::EditorPlace::Grid,
    });
    assert_eq!(editor_text(&harness, tab, id), None);
    assert_eq!(
        pending_text(&harness, tab, id).as_deref(),
        Some("2026-02-16")
    );
    // The day it was, written another way, is no change.
    type_in(&mut harness, tab, id, "Jan 15 2026");
    harness.press(Key::Enter, Modifiers::NONE);
    assert_eq!(pending(&harness, tab, id), None);
}

#[test]
fn a_time_the_clocks_skip_is_refused_until_the_time_after_is_taken() {
    let (mut harness, tab, id) = covers(Driver::Postgres, "timestamptz", "2026-01-15 14:30:00+00");
    type_in(&mut harness, tab, id, "2026-03-08 02:30");
    assert!(says(
        &mut harness,
        "02:30 doesn't exist on 8 Mar in New York; clocks go forward."
    ));
    harness.press(Key::Enter, Modifiers::NONE);
    assert_eq!(pending(&harness, tab, id), None);
    harness.click("Use 03:30");
    assert_eq!(
        editor_text(&harness, tab, id).as_deref(),
        Some("2026-03-08 03:30")
    );
    harness.press(Key::Enter, Modifiers::NONE);
    assert_eq!(
        pending_text(&harness, tab, id).as_deref(),
        Some("2026-03-08 03:30:00-04")
    );
}

#[test]
fn a_time_the_clocks_go_back_over_takes_summer_time_unless_the_other_is_chosen() {
    let (mut harness, tab, id) = covers(Driver::Postgres, "timestamptz", "2026-01-15 14:30:00+00");
    type_in(&mut harness, tab, id, "2026-11-01 01:30");
    assert!(says(
        &mut harness,
        "01:30 happens twice on 1 Nov in New York; clocks go back. · sent as 2026-11-01 01:30:00-04"
    ));
    harness.press(Key::Enter, Modifiers::NONE);
    assert_eq!(
        pending_text(&harness, tab, id).as_deref(),
        Some("2026-11-01 01:30:00-04")
    );
    type_in(&mut harness, tab, id, "2026-11-01 01:30");
    harness.click("01:30 EST");
    assert_eq!(
        editor_text(&harness, tab, id).as_deref(),
        Some("2026-11-01 01:30-05")
    );
    harness.press(Key::Enter, Modifiers::NONE);
    assert_eq!(
        pending_text(&harness, tab, id).as_deref(),
        Some("2026-11-01 01:30:00-05")
    );
}

#[test]
fn a_timestamp_without_a_zone_is_stored_as_typed_whatever_the_zone() {
    for zone in ["America/New_York", "Asia/Kolkata"] {
        let (mut harness, tab, id) = covers(Driver::Postgres, "timestamp", "2026-01-15 09:30:00");
        in_zone(&mut harness, zone);
        type_in(&mut harness, tab, id, "2026-03-08 02:30");
        assert!(says(
            &mut harness,
            "Sun 8 Mar 2026, 02:30 · no zone · sent as 2026-03-08 02:30:00"
        ));
        harness.press(Key::Enter, Modifiers::NONE);
        assert_eq!(
            pending_text(&harness, tab, id).as_deref(),
            Some("2026-03-08 02:30:00"),
            "{zone}"
        );
    }
}

#[test]
fn mysql_refuses_what_its_types_do_not_hold() {
    let (mut harness, tab, id) = covers(Driver::MySql, "timestamp", "2026-01-15 09:30:00");
    type_in(&mut harness, tab, id, "2040-05-01 08:00");
    assert!(says(
        &mut harness,
        "MySQL TIMESTAMP holds 1970-01-01 to 2038-01-19 UTC."
    ));
    harness.press(Key::Enter, Modifiers::NONE);
    assert_eq!(pending(&harness, tab, id), None);
    let (mut harness, tab, id) = covers(Driver::MySql, "date", "2026-01-15");
    type_in(&mut harness, tab, id, "0999-12-31");
    assert!(says(
        &mut harness,
        "MySQL DATE holds 1000-01-01 to 9999-12-31."
    ));
    harness.press(Key::Enter, Modifiers::NONE);
    assert_eq!(pending(&harness, tab, id), None);
}

#[test]
fn a_mysql_zero_date_is_said_and_kept_until_another_is_typed() {
    let (mut harness, tab, id) = covers(Driver::MySql, "date", "0000-00-00");
    open(&mut harness, tab, id);
    assert!(says(
        &mut harness,
        "MySQL zero date · pick a date or set NULL"
    ));
    harness.press(Key::Enter, Modifiers::NONE);
    assert_eq!(editor_text(&harness, tab, id), None);
    assert_eq!(pending(&harness, tab, id), None);
}

#[test]
fn infinity_and_days_before_christ_go_back_as_they_came() {
    let (mut harness, tab, id) = covers(Driver::Postgres, "date", "infinity");
    open(&mut harness, tab, id);
    assert!(says(&mut harness, "No end date · stored as infinity"));
    harness.press(Key::Enter, Modifiers::NONE);
    assert_eq!(pending(&harness, tab, id), None);
    type_in(&mut harness, tab, id, "-infinity");
    harness.press(Key::Enter, Modifiers::NONE);
    assert_eq!(
        pending_text(&harness, tab, id).as_deref(),
        Some("-infinity")
    );
    let (mut harness, tab, id) = covers(Driver::Postgres, "date", "0044-03-15 BC");
    type_in(&mut harness, tab, id, "0044-03-15 BC");
    harness.press(Key::Enter, Modifiers::NONE);
    assert_eq!(pending(&harness, tab, id), None);
    type_in(&mut harness, tab, id, "0044-03-16 bc");
    harness.press(Key::Enter, Modifiers::NONE);
    assert_eq!(
        pending_text(&harness, tab, id).as_deref(),
        Some("0044-03-16 BC")
    );
}

#[test]
fn sqlite_writes_seconds_where_its_values_are_seconds_and_text_elsewhere() {
    // 24 and 23 October 2026, at midnight UTC.
    let seconds = [Value::Int(1_792_800_000), Value::Int(1_792_713_600)];
    let (mut harness, tab, id) = covers_with(Driver::Sqlite, "DATE", &seconds);
    open(&mut harness, tab, id);
    // Opened on the day the seconds are: closed untouched, no change.
    assert_eq!(
        editor_text(&harness, tab, id).as_deref(),
        Some("2026-10-24")
    );
    harness.press(Key::Enter, Modifiers::NONE);
    assert_eq!(pending(&harness, tab, id), None);
    type_in(&mut harness, tab, id, "25 Oct 2026");
    assert!(says(
        &mut harness,
        "Sun 25 Oct 2026 · tomorrow · stored as seconds since 1970, like the other rows · sent as 1792886400"
    ));
    harness.press(Key::Enter, Modifiers::NONE);
    assert_eq!(
        pending_text(&harness, tab, id).as_deref(),
        Some("1792886400")
    );
    // The values nobody edited keep theirs.
    let object = harness.app.workspace(tab).unwrap().object_tab(id).unwrap();
    assert_eq!(object.edits.cells.len(), 1);
    let text = [Value::Text("2026-10-24".into())];
    let (mut harness, tab, id) = covers_with(Driver::Sqlite, "DATE", &text);
    type_in(&mut harness, tab, id, "25 Oct 2026");
    harness.press(Key::Enter, Modifiers::NONE);
    assert_eq!(
        pending_text(&harness, tab, id).as_deref(),
        Some("2026-10-25")
    );
}

#[test]
fn null_and_default_are_typed_only_where_the_column_takes_them() {
    // `created_at` has a default, and takes no NULL.
    let (mut harness, tab, id) = covers(Driver::Postgres, "date", "2026-01-15");
    type_in(&mut harness, tab, id, "DEFAULT");
    harness.press(Key::Enter, Modifiers::NONE);
    assert!(matches!(
        pending(&harness, tab, id),
        Some(NewValue::Default { .. })
    ));
    type_in(&mut harness, tab, id, "NULL");
    assert!(says(
        &mut harness,
        "Not a date. Try 2026-01-15, 15 Jan 2026 or tomorrow."
    ));
    harness.press(Key::Enter, Modifiers::NONE);
    assert!(editor_text(&harness, tab, id).is_some());
}

#[test]
fn the_row_panel_has_the_line_under_its_field() {
    let (mut harness, tab, id) = covers(Driver::Postgres, "date", "2026-01-15");
    harness.app.workspace_mut(tab).unwrap().row_panel = true;
    harness.settle();
    harness.app.apply(Action::EditField {
        tab,
        id,
        cell: CellPos { row: 0, col: MADE },
        start: EditStart::Value,
    });
    harness.settle();
    editor_mut(&mut harness, tab, id).text = "+6d".into();
    harness.app.apply(Action::EditorTyped { tab, id });
    assert!(says(
        &mut harness,
        "Fri 30 Oct 2026 · in 6 days · sent as 2026-10-30"
    ));
    harness.press(Key::Enter, Modifiers::NONE);
    assert_eq!(
        pending_text(&harness, tab, id).as_deref(),
        Some("2026-10-30")
    );
}

#[test]
fn the_terminal_look_heads_its_float_with_the_column_and_the_zone() {
    let (mut harness, tab, id) = covers(Driver::Postgres, "timestamptz", "2026-01-15 14:30:00+00");
    harness.set_look(crate::theme::Look::omarchy());
    type_in(&mut harness, tab, id, "2026-02-30 09:30");
    assert!(says(&mut harness, "! there is no 30 february."));
    assert!(harness.painted_color("america/new_york").is_some());
}

#[test]
fn a_date_typed_back_as_another_writing_of_it_is_no_change() {
    // BC, with the offset the server wrote: the editor opens on it as
    // sent, and the zone shows it with New York's own clock then.
    let (mut harness, tab, id) =
        covers(Driver::Postgres, "timestamptz", "0044-03-15 14:52:04+00 BC");
    type_in(&mut harness, tab, id, "0044-03-15 14:52:04+00 BC");
    harness.press(Key::Enter, Modifiers::NONE);
    assert_eq!(pending(&harness, tab, id), None);
    // A fraction of zeros, or one with more of them than the server wrote.
    let (mut harness, tab, id) = covers(Driver::Postgres, "timestamp", "2026-01-15 09:30:12.25");
    type_in(&mut harness, tab, id, "2026-01-15 09:30:12.250");
    harness.press(Key::Enter, Modifiers::NONE);
    assert_eq!(pending(&harness, tab, id), None);
    let (mut harness, tab, id) = covers(Driver::Postgres, "timestamp", "2026-01-15 09:30:00");
    type_in(&mut harness, tab, id, "2026-01-15 09:30:00.000");
    harness.press(Key::Enter, Modifiers::NONE);
    assert_eq!(pending(&harness, tab, id), None);
}

#[test]
fn a_date_that_is_a_foreign_key_says_why_its_text_is_refused() {
    let (mut harness, tab, id) = keyed_covers(Driver::Postgres, "date", "2026-01-15");
    open(&mut harness, tab, id);
    // The picker asks for the columns of the table the key points at.
    let column = |name: &str, type_name: &str| tabletist_db::ColumnInfo {
        name: name.into(),
        type_name: type_name.into(),
        ..tabletist_db::ColumnInfo::default()
    };
    harness.answer_structure(tabletist_db::Structure {
        columns: vec![column("id", "integer"), column("placed_at", "date")],
        primary_key: vec!["id".into()],
        ..tabletist_db::Structure::default()
    });
    harness.answer_lookups(&[]);
    editor_mut(&mut harness, tab, id).text = "2026-02-30".into();
    harness.app.apply(Action::EditorTyped { tab, id });
    harness.settle();
    // The text is a search until it is taken: no order holds that day,
    // and Enter takes the text.
    harness.answer_lookups(&[]);
    harness.settle();
    harness.press(Key::Enter, Modifiers::NONE);
    assert!(editor_text(&harness, tab, id).is_some());
    assert!(
        says(&mut harness, "There is no 30 February."),
        "{:?}",
        labels(&harness.settle())
    );
}

#[test]
fn a_timestamptz_whose_zone_is_not_read_yet_is_sent_in_utc() {
    let (mut harness, tab, id) = covers(Driver::Postgres, "timestamptz", "2026-01-15 14:30:00+00");
    // A named zone the database does not know: values are shown as sent.
    in_zone(&mut harness, "Mars/Olympus");
    type_in(&mut harness, tab, id, "2026-01-15 09:30");
    assert!(says(
        &mut harness,
        "Thu 15 Jan 2026, 09:30 UTC · sent as 2026-01-15 09:30:00+00"
    ));
    type_in(&mut harness, tab, id, "tomorrow 9am");
    harness.press(Key::Enter, Modifiers::NONE);
    assert_eq!(
        pending_text(&harness, tab, id).as_deref(),
        Some("2026-10-25 09:00:00+00")
    );
}

#[test]
fn a_time_the_clocks_skip_is_a_warning_and_no_day_is_a_fault() {
    let (mut harness, tab, id) = covers(Driver::Postgres, "timestamptz", "2026-01-15 14:30:00+00");
    type_in(&mut harness, tab, id, "2026-03-08 02:30");
    let problem = editor_mut(&mut harness, tab, id)
        .problem
        .clone()
        .expect("refused");
    assert!(problem.warns());
    type_in(&mut harness, tab, id, "2026-02-30 02:30");
    let problem = editor_mut(&mut harness, tab, id)
        .problem
        .clone()
        .expect("refused");
    assert!(!problem.warns());
}
