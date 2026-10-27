//! The calendar under the line of a date's editor: the month the text is
//! in, the days a pointer picks, the time fields, Now and Today, the
//! footer's NULL, DEFAULT and infinities, the first weekday, and the
//! setting that shows it. On the Bookshop's `book_covers.created_at`, its
//! type changed for each test, with the clock stopped on Saturday 24
//! October 2026 at 10:00 in New York.
//!
//! What is read is the tree a screen reader is given, the field's text
//! and what a commit stores. No colour, size or picture of the design is.

use egui::accesskit::{Node, Role};
use egui::{Key, Modifiers};
use tabletist_db::{Driver, NewValue};

use super::date_tests::{
    MADE, covers, covers_with, editor_text, keyed_covers, open, pending, pending_text, says,
    type_in,
};
use crate::model::Action;
use crate::testing::{Harness, labels};

/// The node labelled `label` in the role `role`, where it is drawn.
fn node(harness: &mut Harness, label: &str, role: Role) -> Option<Node> {
    let tree = harness.settle();
    tree.nodes
        .iter()
        .find(|(_, node)| node.role() == role && node.label() == Some(label))
        .map(|(_, node)| node.clone())
}

/// The month the calendar shows, by its grid's name.
fn month(harness: &mut Harness) -> Option<String> {
    let tree = harness.settle();
    tree.nodes
        .iter()
        .find(|(_, node)| node.role() == Role::Grid)
        .and_then(|(_, node)| node.label().map(str::to_owned))
}

fn day(harness: &mut Harness, name: &str) -> Node {
    node(harness, name, Role::Button)
        .unwrap_or_else(|| panic!("no day {name:?}: {:?}", labels(&harness.settle())))
}

/// The days of the month that are selected.
fn selected(harness: &mut Harness) -> Vec<String> {
    let tree = harness.settle();
    tree.nodes
        .iter()
        .filter(|(_, node)| node.role() == Role::Button && node.is_selected() == Some(true))
        .filter_map(|(_, node)| node.label().map(str::to_owned))
        .collect()
}

/// `created_at` takes NULL.
fn nullable(harness: &mut Harness, tab: crate::model::ConnTabId, id: crate::model::TabId) {
    let workspace = harness.app.workspace_mut(tab).unwrap();
    let object = workspace.object_tab_mut(id).unwrap();
    object.structure.value.as_mut().unwrap().columns[MADE].nullable = true;
}

#[test]
fn the_month_follows_the_text_and_marks_the_value_before_and_today() {
    let (mut harness, tab, id) = covers(Driver::Postgres, "timestamptz", "2026-01-15 14:30:00+00");
    open(&mut harness, tab, id);
    assert_eq!(month(&mut harness).as_deref(), Some("January 2026"));
    assert_eq!(selected(&mut harness), ["Thursday 15 January 2026"]);
    // The value before the edit, once the text is another day.
    type_in(&mut harness, tab, id, "2026-01-20 09:30");
    assert_eq!(selected(&mut harness), ["Tuesday 20 January 2026"]);
    let was = day(&mut harness, "Thursday 15 January 2026");
    assert_eq!(was.description(), Some("current value"));
    // A cell with a pending value marks that one: what it shows.
    harness.press(Key::Enter, Modifiers::NONE);
    type_in(&mut harness, tab, id, "2026-01-25 09:30");
    let pending = day(&mut harness, "Tuesday 20 January 2026");
    assert_eq!(pending.description(), Some("current value"));
    assert_eq!(
        day(&mut harness, "Thursday 15 January 2026").description(),
        None
    );
    // Today, in the month a relative word names.
    type_in(&mut harness, tab, id, "+6d");
    assert_eq!(month(&mut harness).as_deref(), Some("October 2026"));
    assert_eq!(selected(&mut harness), ["Friday 30 October 2026"]);
    let today = day(&mut harness, "Saturday 24 October 2026");
    assert_eq!(today.description(), Some("today"));
    // A text that does not read shows the month typed so far, with
    // nothing selected.
    type_in(&mut harness, tab, id, "2026-02-30 09:30");
    assert_eq!(month(&mut harness).as_deref(), Some("February 2026"));
    assert_eq!(selected(&mut harness), Vec::<String>::new());
    // And one that names none stays where it was.
    type_in(&mut harness, tab, id, "soon");
    assert_eq!(month(&mut harness).as_deref(), Some("February 2026"));
    // BC is the year counted back.
    let (mut harness, tab, id) = covers(Driver::Postgres, "date", "0044-03-15 BC");
    open(&mut harness, tab, id);
    assert_eq!(month(&mut harness).as_deref(), Some("March 44 BC"));
}

#[test]
fn a_day_picked_keeps_the_time_and_rewrites_the_text() {
    let (mut harness, tab, id) = covers(Driver::Postgres, "timestamptz", "2026-01-15 14:30:00+00");
    open(&mut harness, tab, id);
    harness.click("Tuesday 20 January 2026");
    assert_eq!(
        editor_text(&harness, tab, id).as_deref(),
        Some("2026-01-20 09:30")
    );
    // The line says what the pick is, and the field keeps the keyboard.
    assert!(says(
        &mut harness,
        "Tue 20 Jan 2026, 09:30 EST · sent as 2026-01-20 09:30:00-05"
    ));
    harness.press(Key::Enter, Modifiers::NONE);
    assert_eq!(
        pending_text(&harness, tab, id).as_deref(),
        Some("2026-01-20 09:30:00-05")
    );
    // A text that does not read dims the month, which still takes a
    // click: the time typed so far is kept.
    type_in(&mut harness, tab, id, "2026-02-30 09:30");
    harness.click("Sunday 15 February 2026");
    assert_eq!(
        editor_text(&harness, tab, id).as_deref(),
        Some("2026-02-15 09:30")
    );
    // A text with no time picks the day at midnight, in the month shown
    // last; a date keeps none.
    type_in(&mut harness, tab, id, "");
    harness.click("Monday 26 January 2026");
    assert_eq!(
        editor_text(&harness, tab, id).as_deref(),
        Some("2026-01-26 00:00")
    );
    let (mut harness, tab, id) = covers(Driver::Postgres, "date", "2026-01-15");
    open(&mut harness, tab, id);
    harness.click("Friday 16 January 2026");
    assert_eq!(
        editor_text(&harness, tab, id).as_deref(),
        Some("2026-01-16")
    );
    // Another month's day, from the weeks the month shares.
    harness.click("Monday 2 February 2026");
    assert_eq!(
        editor_text(&harness, tab, id).as_deref(),
        Some("2026-02-02")
    );
    assert_eq!(month(&mut harness).as_deref(), Some("February 2026"));
}

#[test]
fn now_sets_the_clocks_time_and_today_only_shows_its_month() {
    let (mut harness, tab, id) = covers(Driver::Postgres, "timestamptz", "2026-01-15 14:30:00+00");
    open(&mut harness, tab, id);
    // The arrows and Today move the month, never the value.
    harness.click("Next month");
    assert_eq!(month(&mut harness).as_deref(), Some("February 2026"));
    harness.click("Today");
    assert_eq!(month(&mut harness).as_deref(), Some("October 2026"));
    harness.click("Previous month");
    assert_eq!(month(&mut harness).as_deref(), Some("September 2026"));
    assert_eq!(
        editor_text(&harness, tab, id).as_deref(),
        Some("2026-01-15 09:30:00-05")
    );
    harness.click("Now");
    assert_eq!(
        editor_text(&harness, tab, id).as_deref(),
        Some("2026-10-24 10:00")
    );
    assert_eq!(month(&mut harness).as_deref(), Some("October 2026"));
    harness.press(Key::Enter, Modifiers::NONE);
    assert_eq!(
        pending_text(&harness, tab, id).as_deref(),
        Some("2026-10-24 10:00:00-04")
    );
    // With the calendar the database's now is not beside the field: it is
    // typed, `now()`.
    open(&mut harness, tab, id);
    assert!(node(&mut harness, "now", Role::Button).is_none());
}

#[test]
fn the_second_of_a_repeated_hour_stays_the_second_when_picked_or_now() {
    let (mut harness, tab, id) = covers(Driver::Postgres, "timestamptz", "2026-01-15 14:30:00+00");
    type_in(&mut harness, tab, id, "2026-11-01 01:30");
    harness.click("01:30 EST");
    harness.click("Sunday 1 November 2026");
    assert_eq!(
        editor_text(&harness, tab, id).as_deref(),
        Some("2026-11-01 01:30-05")
    );
    harness.press(Key::Enter, Modifiers::NONE);
    assert_eq!(
        pending_text(&harness, tab, id).as_deref(),
        Some("2026-11-01 01:30:00-05")
    );
    // Now, in the second 01:30.
    harness.app.stopped_clock = Some("2026-11-01T06:30:00Z".parse().unwrap());
    open(&mut harness, tab, id);
    harness.click("Now");
    harness.press(Key::Enter, Modifiers::NONE);
    assert_eq!(
        pending_text(&harness, tab, id).as_deref(),
        Some("2026-11-01 01:30:00-05")
    );
}

#[test]
fn the_month_follows_each_new_text_and_is_each_cells_own() {
    // Moved off with the arrows, the month comes back with a new text in
    // the month it was in, and with Now.
    let (mut harness, tab, id) = covers(Driver::Postgres, "date", "2026-01-15");
    open(&mut harness, tab, id);
    harness.click("Next month");
    type_in(&mut harness, tab, id, "2026-01-20");
    assert_eq!(month(&mut harness).as_deref(), Some("January 2026"));
    let (mut harness, tab, id) = covers(Driver::Postgres, "timestamptz", "2026-10-15 14:30:00+00");
    open(&mut harness, tab, id);
    harness.click("Previous month");
    harness.click("Now");
    assert_eq!(month(&mut harness).as_deref(), Some("October 2026"));
    // Another cell's editor, opened from this one's, starts on its own
    // month: a NULL's, today's.
    let days = [
        tabletist_db::Value::Text("2026-01-15".into()),
        tabletist_db::Value::Null,
    ];
    let (mut harness, tab, id) = covers_with(Driver::Postgres, "date", &days);
    open(&mut harness, tab, id);
    harness.click("Next month");
    harness.app.apply(Action::EditCell {
        tab,
        id,
        cell: crate::model::CellPos { row: 1, col: MADE },
        start: crate::model::EditStart::Value,
    });
    assert_eq!(month(&mut harness).as_deref(), Some("October 2026"));
}

#[test]
fn a_day_the_type_does_not_hold_cannot_be_picked() {
    let (mut harness, tab, id) = covers(Driver::MySql, "date", "1000-01-15");
    open(&mut harness, tab, id);
    assert_eq!(month(&mut harness).as_deref(), Some("January 1000"));
    let before = day(&mut harness, "Tuesday 31 December 999");
    assert!(before.is_disabled());
    harness.click("Tuesday 31 December 999");
    assert_eq!(
        editor_text(&harness, tab, id).as_deref(),
        Some("1000-01-15")
    );
    assert!(!day(&mut harness, "Wednesday 1 January 1000").is_disabled());
    // SQLite's years run from 0000: the year before is shown, and none of
    // its days taken.
    let (mut harness, tab, id) = covers(Driver::Sqlite, "DATE", "0000-01-15");
    open(&mut harness, tab, id);
    harness.click("Previous month");
    assert!(day(&mut harness, "Monday 6 December 2 BC").is_disabled());
}

#[test]
fn the_time_fields_are_spin_buttons_with_their_range() {
    let (mut harness, tab, id) = covers(Driver::Postgres, "time", "09:30:00");
    open(&mut harness, tab, id);
    // A time has no month.
    assert_eq!(month(&mut harness), None);
    let hour = node(&mut harness, "Hour", Role::SpinButton).expect("the hour");
    assert_eq!(hour.numeric_value(), Some(9.0));
    assert_eq!(hour.max_numeric_value(), Some(23.0));
    let minute = node(&mut harness, "Minute", Role::SpinButton).expect("the minute");
    assert_eq!(minute.numeric_value(), Some(30.0));
    assert!(node(&mut harness, "Fraction of a second", Role::SpinButton).is_none());
    // A type that keeps a fraction shows it, to its digits.
    let (mut harness, tab, id) = covers(Driver::Postgres, "timestamp(6)", "2026-01-15 09:30:12.25");
    open(&mut harness, tab, id);
    let fraction =
        node(&mut harness, "Fraction of a second", Role::SpinButton).expect("the fraction");
    assert_eq!(fraction.numeric_value(), Some(250_000.0));
    assert_eq!(fraction.max_numeric_value(), Some(999_999.0));
    // The chip says the timestamp keeps no zone.
    assert!(harness.has("no zone"));
    // A `timetz`'s is its offset.
    let (mut harness, tab, id) = covers(Driver::Postgres, "timetz", "09:30:00-05");
    open(&mut harness, tab, id);
    assert!(harness.has("-05"));
    // A MySQL `TIMESTAMP` whose session's zone is not known: the server's,
    // and no Now, whose time is the server's to say.
    let (mut harness, tab, id) = covers(Driver::MySql, "timestamp", "2026-01-15 09:30:00");
    open(&mut harness, tab, id);
    assert!(harness.has("the server's zone"));
    assert!(node(&mut harness, "Now", Role::Button).is_none());
}

#[test]
fn null_and_default_are_offered_only_where_the_column_takes_them() {
    // `created_at` has a default, and takes no NULL.
    let (mut harness, tab, id) = covers(Driver::Postgres, "date", "2026-01-15");
    open(&mut harness, tab, id);
    assert!(node(&mut harness, "NULL", Role::Button).is_none());
    harness.click("DEFAULT");
    assert_eq!(editor_text(&harness, tab, id), None);
    assert!(matches!(
        pending(&harness, tab, id),
        Some(NewValue::Default { .. })
    ));
    nullable(&mut harness, tab, id);
    open(&mut harness, tab, id);
    harness.click("NULL");
    assert_eq!(editor_text(&harness, tab, id), None);
    assert_eq!(pending(&harness, tab, id), Some(NewValue::Null));
    // The infinities are PostgreSQL's, and pick as a day does.
    open(&mut harness, tab, id);
    harness.click("Infinity");
    assert_eq!(editor_text(&harness, tab, id).as_deref(), Some("infinity"));
    assert!(says(&mut harness, "No end date · stored as infinity"));
    harness.click("Minus infinity");
    assert_eq!(editor_text(&harness, tab, id).as_deref(), Some("-infinity"));
    let (mut harness, tab, id) = covers(Driver::MySql, "date", "2026-01-15");
    open(&mut harness, tab, id);
    assert!(node(&mut harness, "Infinity", Role::Button).is_none());
    assert!(node(&mut harness, "DEFAULT", Role::Button).is_some());
}

#[test]
fn the_setting_shows_the_line_alone_and_reloads_live() {
    let (mut harness, tab, id) = covers(Driver::Postgres, "date", "2026-01-15");
    open(&mut harness, tab, id);
    assert_eq!(month(&mut harness).as_deref(), Some("January 2026"));
    harness.press(Key::Escape, Modifiers::NONE);
    // The file changes while the app runs.
    let text = "[data]\ndate_picker = false\n".to_owned();
    let file = crate::backend::Event::SettingsFile { text, own: false };
    harness.app.apply(Action::Backend(file));
    open(&mut harness, tab, id);
    assert_eq!(month(&mut harness), None);
    assert!(says(&mut harness, "Thu 15 Jan 2026"));
    // The database's now is back beside the field.
    assert!(node(&mut harness, "now", Role::Button).is_some());
}

#[test]
fn the_weeks_start_on_the_locales_first_day() {
    for (first, starts, before) in [
        (
            jiff::civil::Weekday::Monday,
            "Monday 29 December 2025",
            "Sunday 28 December 2025",
        ),
        (
            jiff::civil::Weekday::Sunday,
            "Sunday 28 December 2025",
            "Saturday 27 December 2025",
        ),
    ] {
        let (mut harness, tab, id) = covers(Driver::Postgres, "date", "2026-01-15");
        harness.app.week_start = first;
        open(&mut harness, tab, id);
        let tree = harness.settle();
        let first_day = crate::testing::bounds(&tree, starts, Role::Button).expect(starts);
        let thursday = crate::testing::bounds(&tree, "Thursday 1 January 2026", Role::Button)
            .expect("the first of January");
        // The first day is the first of its row.
        assert!(first_day.left() < thursday.left(), "{first:?}");
        assert!(
            node(&mut harness, before, Role::Button).is_none(),
            "{first:?}"
        );
    }
}

#[test]
fn the_terminal_look_paints_its_month_for_the_eye_and_says_the_value_before_on_its_line() {
    let (mut harness, tab, id) = covers(Driver::Postgres, "timestamptz", "2026-01-15 14:30:00+00");
    harness.set_look(crate::theme::Look::omarchy());
    type_in(&mut harness, tab, id, "2026-01-20 09:30");
    // Its line carries everything: the calendar is no part of the tree.
    assert!(says(
        &mut harness,
        "tue 20 jan 2026, 09:30 est · was 15 jan · sent as 2026-01-20 09:30:00-05"
    ));
    assert_eq!(month(&mut harness), None);
    assert!(node(&mut harness, "Hour", Role::SpinButton).is_none());
    assert!(harness.painted_color("january 2026").is_some());
    assert!(harness.painted_color("09:30:00").is_some());
    assert!(harness.painted_color("est · utc-05").is_some());
}

#[test]
fn the_row_panel_has_the_calendar_under_its_field() {
    let (mut harness, tab, id) = covers(Driver::Postgres, "date", "2026-01-15");
    harness.app.workspace_mut(tab).unwrap().row_panel = true;
    harness.settle();
    harness.app.apply(Action::EditField {
        tab,
        id,
        cell: crate::model::CellPos { row: 0, col: MADE },
        start: crate::model::EditStart::Value,
    });
    assert_eq!(month(&mut harness).as_deref(), Some("January 2026"));
    harness.click("Friday 16 January 2026");
    assert_eq!(
        editor_text(&harness, tab, id).as_deref(),
        Some("2026-01-16")
    );
    harness.press(Key::Enter, Modifiers::NONE);
    assert_eq!(
        pending_text(&harness, tab, id).as_deref(),
        Some("2026-01-16")
    );
}

/// The session zone a MySQL tab was told: `zone`.
fn session_zone(harness: &mut Harness, tab: crate::model::ConnTabId, zone: jiff::tz::TimeZone) {
    let session = harness.app.workspace(tab).unwrap().session;
    let request = crate::backend::RequestId(9_000);
    harness
        .app
        .workspace_mut(tab)
        .unwrap()
        .session_zone
        .start(request);
    harness
        .app
        .apply(Action::Backend(crate::backend::Event::SessionZone {
            session,
            request,
            result: Ok(Some(zone)),
        }));
}

#[test]
fn a_zone_with_no_name_is_told_by_its_offset_on_the_chip() {
    // A session whose `time_zone` is `+08:00` is a fixed offset, which
    // has no place in its name.
    let (mut harness, tab, id) = covers(Driver::MySql, "timestamp", "2026-01-15 09:30:00");
    let east = jiff::tz::Offset::from_seconds(8 * 3_600).unwrap();
    session_zone(&mut harness, tab, jiff::tz::TimeZone::fixed(east));
    open(&mut harness, tab, id);
    assert!(harness.has("+08"), "{:?}", labels(&harness.settle()));
    // And a half hour is written as the server writes it.
    let india = jiff::tz::Offset::from_seconds(5 * 3_600 + 1_800).unwrap();
    session_zone(&mut harness, tab, jiff::tz::TimeZone::fixed(india));
    open(&mut harness, tab, id);
    assert!(harness.has("+05:30"), "{:?}", labels(&harness.settle()));
}

/// The primary button at `at`, going down or coming up.
fn button_at(at: egui::Pos2, pressed: bool) -> egui::Event {
    egui::Event::PointerButton {
        pos: at,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: Modifiers::NONE,
    }
}

/// A click of the primary button at `at`: the pointer moves there, the
/// button goes down and comes up, a frame each, and the field is asked
/// whether it has the keyboard between the two. Not an AccessKit click,
/// which never moves the pointer.
fn click_at(
    harness: &mut Harness,
    (tab, id): (crate::model::ConnTabId, crate::model::TabId),
    at: egui::Pos2,
) {
    let field = crate::ui::cell_editor::field_id(tab, id);
    harness.frame(vec![egui::Event::PointerMoved(at)]);
    harness.frame(vec![button_at(at, true)]);
    harness.frame(Vec::new());
    assert!(
        harness.ctx.memory(|memory| memory.has_focus(field)),
        "the field lost the keyboard to the press"
    );
    harness.frame(vec![button_at(at, false)]);
    harness.settle();
    assert!(
        harness.ctx.memory(|memory| memory.has_focus(field)),
        "the field lost the keyboard to the click"
    );
}

#[test]
fn a_pointer_press_on_a_day_keeps_the_keyboard_in_the_field() {
    let (mut harness, tab, id) = covers(Driver::Postgres, "timestamptz", "2026-01-15 14:30:00+00");
    open(&mut harness, tab, id);
    let tree = harness.settle();
    let at = crate::testing::bounds(&tree, "Tuesday 20 January 2026", Role::Button)
        .expect("the day")
        .center();
    click_at(&mut harness, (tab, id), at);
    // The editor is still open with the day's text, and Enter commits it.
    assert_eq!(
        editor_text(&harness, tab, id).as_deref(),
        Some("2026-01-20 09:30")
    );
    harness.press(Key::Enter, Modifiers::NONE);
    assert_eq!(
        pending_text(&harness, tab, id).as_deref(),
        Some("2026-01-20 09:30:00-05")
    );
}

#[test]
fn a_pointer_press_on_a_day_the_type_does_not_hold_leaves_the_edit_alone() {
    let (mut harness, tab, id) = covers(Driver::MySql, "date", "1000-01-15");
    open(&mut harness, tab, id);
    let tree = harness.settle();
    let at = crate::testing::bounds(&tree, "Tuesday 31 December 999", Role::Button)
        .expect("the day")
        .center();
    click_at(&mut harness, (tab, id), at);
    assert_eq!(
        editor_text(&harness, tab, id).as_deref(),
        Some("1000-01-15")
    );
    // Still the field's: a day typed is taken by Enter.
    type_in(&mut harness, tab, id, "1000-01-20");
    click_at(&mut harness, (tab, id), at);
    harness.press(Key::Enter, Modifiers::NONE);
    assert_eq!(
        pending_text(&harness, tab, id).as_deref(),
        Some("1000-01-20")
    );
}

#[test]
fn a_pointer_press_beside_the_time_fields_keeps_the_keyboard_in_the_field() {
    let (mut harness, tab, id) = covers(Driver::Postgres, "timestamptz", "2026-01-15 14:30:00+00");
    open(&mut harness, tab, id);
    let tree = harness.settle();
    let hour = crate::testing::bounds(&tree, "Hour", Role::SpinButton).expect("the hour");
    // Left of the first field, in the row's room at the side.
    let at = egui::pos2(hour.left() - 4.0, hour.center().y);
    click_at(&mut harness, (tab, id), at);
    assert_eq!(
        editor_text(&harness, tab, id).as_deref(),
        Some("2026-01-15 09:30:00-05")
    );
    harness.press(Key::Enter, Modifiers::NONE);
    assert_eq!(pending(&harness, tab, id), None);
}

#[test]
fn the_popovers_area_is_the_popover_and_no_taller() {
    let (mut harness, tab, id) = covers(Driver::Postgres, "timestamptz", "2026-01-15 14:30:00+00");
    open(&mut harness, tab, id);
    let tree = harness.settle();
    let dialog = tree
        .nodes
        .iter()
        .find(|(_, node)| node.role() == Role::Dialog)
        .and_then(|(_, node)| node.bounds())
        .expect("the popover");
    let area = harness
        .ctx
        .memory(|memory| memory.area_rect(egui::Id::new("date-line")))
        .expect("its area");
    assert!(
        (f64::from(area.height()) - (dialog.y1 - dialog.y0)).abs() < 0.5,
        "{area:?} against {dialog:?}"
    );
}

/// Answers the picker of a foreign key date cell, as the orders' table.
fn answer_orders(harness: &mut Harness) {
    let column = |name: &str, type_name: &str| tabletist_db::ColumnInfo {
        name: name.into(),
        type_name: type_name.into(),
        ..tabletist_db::ColumnInfo::default()
    };
    harness.answer_structure(tabletist_db::Structure {
        columns: vec![column("id", "integer"), column("placed_at", "timestamptz")],
        primary_key: vec!["id".into()],
        ..tabletist_db::Structure::default()
    });
    harness.answer_lookups(&[]);
    harness.settle();
}

#[test]
fn a_foreign_key_date_has_the_now_button_and_no_calendar() {
    let (mut harness, tab, id) =
        keyed_covers(Driver::Postgres, "timestamptz", "2026-01-15 14:30:00+00");
    open(&mut harness, tab, id);
    answer_orders(&mut harness);
    assert_eq!(month(&mut harness), None);
    assert!(node(&mut harness, "now", Role::Button).is_some());
}

#[test]
fn a_foreign_key_date_in_the_row_panel_has_no_calendar_either() {
    let (mut harness, tab, id) =
        keyed_covers(Driver::Postgres, "timestamptz", "2026-01-15 14:30:00+00");
    harness.app.workspace_mut(tab).unwrap().row_panel = true;
    harness.settle();
    harness.app.apply(Action::EditField {
        tab,
        id,
        cell: crate::model::CellPos { row: 0, col: MADE },
        start: crate::model::EditStart::Value,
    });
    answer_orders(&mut harness);
    assert_eq!(month(&mut harness), None);
    assert!(node(&mut harness, "now", Role::Button).is_some());
}
