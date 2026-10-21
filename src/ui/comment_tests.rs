//! The comments the app shows, checked against their spec
//! (`docs/superpowers/specs/2026-10-20-structure-comments-design.md`):
//! the table's over its Structure view, each column's in the view's
//! Comment column, and each column's on its header in the grid.

use egui::accesskit::Role;
use tabletist_db::{ColumnInfo, Driver, Structure};

use super::tests::hover;
use crate::model::{Action, ObjectView};
use crate::testing::{Harness, bounds, labels, page};

fn column(name: &str, comment: Option<&str>) -> ColumnInfo {
    ColumnInfo {
        name: name.into(),
        type_name: "text".into(),
        comment: comment.map(Into::into),
        ..ColumnInfo::default()
    }
}

fn described(columns: Vec<ColumnInfo>, comment: Option<&str>) -> Structure {
    Structure {
        columns,
        primary_key: vec!["id".into()],
        comment: comment.map(Into::into),
        ..Structure::default()
    }
}

/// The fake connection's `users`, as `driver` would describe it, in its
/// Structure view described as `structure`.
fn structure_view(driver: Driver, structure: Structure) -> Harness {
    let mut harness = Harness::new();
    let tab = harness.connect_fake();
    harness.app.workspace_mut(tab).unwrap().driver = driver;
    harness.click("users");
    harness.answer_rows(page(5, false));
    let object_tab = harness.app.workspace(tab).unwrap().active_tab.unwrap();
    harness.app.apply(Action::SetView {
        tab,
        object_tab,
        view: ObjectView::Structure,
    });
    harness.answer_structure(structure);
    harness.settle();
    harness
}

/// The text the last frame painted that begins with `start`.
fn painted_from(harness: &Harness, start: &str) -> String {
    harness
        .painted
        .iter()
        .find(|(text, _)| text.starts_with(start))
        .map(|(text, _)| text.clone())
        .unwrap_or_else(|| panic!("{start}… is painted"))
}

// Test 1 of the spec.
#[test]
fn postgres_and_mysql_show_a_columns_comment_in_the_comment_column() {
    for driver in [Driver::Postgres, Driver::MySql] {
        let columns = vec![column("id", None), column("email", Some("Login address"))];
        let mut harness = structure_view(driver, described(columns, None));
        assert!(harness.has("Comment"), "{driver:?}");
        assert!(harness.has("Login address"), "{driver:?}");
        assert_eq!(painted_from(&harness, "Login address"), "Login address");
    }
}

#[test]
fn a_long_comment_is_cut_and_said_whole() {
    // Under `DISPLAY_LINE_CHARS` (500): `for_display` keeps it one line.
    let long = ["Where the order ships to."; 12].join(" ");
    let columns = vec![column("id", None), column("address", Some(&long))];
    let mut harness = structure_view(Driver::Postgres, described(columns, None));
    // A screen reader hears the whole of it.
    assert!(harness.has(&long));
    let drawn = painted_from(&harness, "Where the order");
    assert!(drawn.ends_with('…'), "{drawn}");
    assert!(drawn.chars().count() < long.chars().count(), "{drawn}");
    // The pointer over it shows it whole: said once more, by the tooltip.
    let at = harness
        .painted_rect(&drawn)
        .expect("where it is drawn")
        .center();
    let before = labels(&harness.settle())
        .iter()
        .filter(|name| **name == long)
        .count();
    let after = hover(&mut harness, at)
        .iter()
        .filter(|name| **name == long)
        .count();
    assert_eq!(after, before + 1);
}

#[test]
fn a_comment_of_more_lines_shows_its_first_and_says_it_whole() {
    let comment = "Login address\nUsed to sign in";
    let columns = vec![column("id", None), column("email", Some(comment))];
    let mut harness = structure_view(Driver::Postgres, described(columns, None));
    assert!(harness.has(comment));
    assert_eq!(painted_from(&harness, "Login address"), "Login address…");
}

#[test]
fn a_comment_of_a_hundred_thousand_characters_on_one_line_is_drawn_cut() {
    let huge = "x".repeat(100_000);
    let columns = vec![column("id", None), column("notes", Some(&huge))];
    let harness = structure_view(Driver::Postgres, described(columns, None));
    let drawn = painted_from(&harness, "xxx");
    assert!(drawn.ends_with('…'), "{} characters", drawn.chars().count());
    let wide = harness
        .painted_rect(&drawn)
        .expect("where it is drawn")
        .width();
    assert!(wide <= 320.5, "{wide} points");
}

// Test 2 of the spec.
#[test]
fn sqlite_has_no_comment_column() {
    let columns = vec![column("id", None), column("email", None)];
    let mut harness = structure_view(Driver::Sqlite, described(columns, None));
    assert!(harness.has("Nullable"));
    assert!(!harness.has("Comment"));
}

// Test 3 of the spec.
#[test]
fn the_tables_comment_stands_over_the_view_and_takes_no_room_without_one() {
    let columns = || vec![column("id", None)];
    let top_of_columns = |harness: &mut Harness| {
        let tree = harness.settle();
        bounds(&tree, "Columns", Role::Label)
            .expect("the Columns heading")
            .top()
    };
    let mut with = structure_view(
        Driver::Postgres,
        described(columns(), Some("People who sign in")),
    );
    assert!(with.has("People who sign in"));
    let tree = with.settle();
    let comment = bounds(&tree, "People who sign in", Role::Label).expect("the comment");
    let label =
        bounds(&tree, "Label for foreign keys", Role::Label).expect("the label card's title");
    assert!(
        comment.bottom() <= label.top(),
        "{comment:?} over {label:?}"
    );
    let mut without = structure_view(Driver::Postgres, described(columns(), None));
    assert!(top_of_columns(&mut without) < top_of_columns(&mut with));
}

#[test]
fn a_table_comment_of_only_spaces_takes_no_room() {
    let top_of_columns = |comment: Option<&str>| {
        let structure = described(vec![column("id", None)], comment);
        let mut harness = structure_view(Driver::Postgres, structure);
        let tree = harness.settle();
        bounds(&tree, "Columns", Role::Label)
            .expect("the Columns heading")
            .top()
    };
    assert_eq!(top_of_columns(Some("  \n ")), top_of_columns(None));
}

#[test]
fn a_comment_is_said_without_the_blank_lines_around_it() {
    let comment = "\n    Login address\n";
    let columns = vec![column("id", None), column("email", Some(comment))];
    let mut harness = structure_view(Driver::Postgres, described(columns, None));
    assert!(harness.has("Login address"));
}
