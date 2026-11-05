//! Copy as and the copy of a selection, checked against their spec
//! (`docs/superpowers/specs/2026-11-03-export-copy-as-2-writers-design.md`:
//! "test N" below is an entry of its Tests).
//!
//! Behaviour and words, on the Bookshop's `book_covers` and on a SQL
//! result. No picture of the design is compared.

use egui::accesskit::Role;
use egui::{Key, Modifiers};
use tabletist_db::Value;

use super::insert_audit_tests::{covers_in, painted, select, type_key};
use crate::model::{Action, ConnTabId, Dialog, TabId};
use crate::testing::Harness;
use crate::theme::Look;

/// Mod+Alt+C, as the window sends it: a copy with Alt held.
fn copy_as_chord(harness: &mut Harness) {
    harness.settle();
    let held = Modifiers {
        alt: true,
        ..Modifiers::COMMAND
    };
    harness.frame(vec![egui::Event::ModifiersChanged(held), egui::Event::Copy]);
    harness.frame(vec![egui::Event::ModifiersChanged(Modifiers::NONE)]);
    harness.settle();
}

fn menu_open(harness: &Harness) -> bool {
    matches!(harness.app.dialog, Some(Dialog::CopyAs(_)))
}

fn said(harness: &Harness) -> Option<String> {
    crate::ui::toast::said(&harness.ctx)
}

/// The rectangle of `publisher_id` and `kind` over the first three rows.
fn rectangle(harness: &mut Harness, tab: ConnTabId, id: TabId) {
    select(harness, tab, id, (0, 1));
    if harness.app.look.terminal {
        type_key(harness, Key::V, "v");
        type_key(harness, Key::J, "j");
        type_key(harness, Key::J, "j");
        type_key(harness, Key::L, "l");
    } else {
        harness.press(Key::ArrowDown, Modifiers::SHIFT);
        harness.press(Key::ArrowDown, Modifiers::SHIFT);
        harness.press(Key::ArrowRight, Modifiers::SHIFT);
    }
}

/// Picks with a letter of the open menu, and returns what was copied.
fn pick(harness: &mut Harness, key: Key, letter: &str) -> Option<String> {
    harness.copied = None;
    type_key(harness, key, letter);
    harness.copied.clone()
}

/// A numeric column is right-aligned, its name too.
const MARKDOWN: &str = "\
|        publisher_id | kind  |
|--------------------:|-------|
| 9100000000000000001 | print |
| 9100000000000000002 | ebook |
| 9100000000000000003 | print |";

/// Test 6: Mod+Alt+C opens the menu over a rectangle, and a letter copies
/// it in that format with the format's options, and says so.
#[test]
fn the_keyboard_menu_copies_a_rectangle_in_each_format() {
    let (mut harness, tab, id) = covers_in(Look::macos());
    rectangle(&mut harness, tab, id);
    let sent = harness.app.backend.sent.len();
    copy_as_chord(&mut harness);
    assert!(menu_open(&harness));
    for row in [
        "CSV",
        "TSV",
        "JSON",
        "Markdown",
        "SQL INSERT",
        "Column names",
    ] {
        assert!(painted(&harness, row), "{row}: {:?}", harness.painted);
    }
    assert_eq!(pick(&mut harness, Key::M, "m").as_deref(), Some(MARKDOWN));
    assert!(!menu_open(&harness));
    assert_eq!(
        said(&harness).as_deref(),
        Some("Copied 3 rows × 2 columns as Markdown")
    );
    let cases = [
        (
            Key::C,
            "c",
            "publisher_id,kind\n9100000000000000001,print\n9100000000000000002,ebook\n9100000000000000003,print",
            "Copied 3 rows × 2 columns as CSV",
        ),
        (
            Key::T,
            "t",
            "publisher_id\tkind\n9100000000000000001\tprint\n9100000000000000002\tebook\n9100000000000000003\tprint",
            "Copied 3 rows × 2 columns as TSV",
        ),
        (
            Key::S,
            "s",
            "INSERT INTO main.book_covers (publisher_id, kind) VALUES\n  (9100000000000000001, 'print'),\n  (9100000000000000002, 'ebook'),\n  (9100000000000000003, 'print');",
            "Copied 3 rows × 2 columns as SQL INSERT · 1 statement",
        ),
        (Key::N, "n", "publisher_id, kind", "Copied 2 column names"),
    ];
    for (key, letter, text, words) in cases {
        copy_as_chord(&mut harness);
        assert_eq!(
            pick(&mut harness, key, letter).as_deref(),
            Some(text),
            "{letter}"
        );
        assert_eq!(said(&harness).as_deref(), Some(words), "{letter}");
    }
    copy_as_chord(&mut harness);
    let json = pick(&mut harness, Key::J, "j").expect("JSON");
    assert!(
        json.starts_with(
            "[\n  {\n    \"publisher_id\": 9100000000000000001,\n    \"kind\": \"print\"\n  },"
        ),
        "{json}"
    );
    // Test 8: a copy records nothing, and sends the backend nothing.
    assert_eq!(harness.app.backend.sent.len(), sent);
}

/// Test 6: the menu's keys: the arrows and Return pick the lit row, Esc
/// closes it with nothing copied.
#[test]
fn the_menus_arrows_and_return_pick_and_esc_closes() {
    let (mut harness, tab, id) = covers_in(Look::macos());
    rectangle(&mut harness, tab, id);
    copy_as_chord(&mut harness);
    harness.press(Key::ArrowDown, Modifiers::NONE);
    harness.press(Key::ArrowDown, Modifiers::NONE);
    harness.press(Key::ArrowUp, Modifiers::NONE);
    harness.copied = None;
    harness.press(Key::Enter, Modifiers::NONE);
    assert!(!menu_open(&harness));
    let tsv = harness.copied.clone().expect("TSV");
    assert!(tsv.starts_with("publisher_id\tkind\n"), "{tsv}");
    copy_as_chord(&mut harness);
    assert!(menu_open(&harness));
    harness.copied = None;
    harness.press(Key::Escape, Modifiers::NONE);
    assert!(!menu_open(&harness));
    assert_eq!(harness.copied, None);
}

/// Test 6: with no range Copy as takes the cursor's row whole, and says
/// so in rows.
#[test]
fn copy_as_with_no_range_takes_the_cursors_row() {
    let (mut harness, tab, id) = covers_in(Look::macos());
    select(&mut harness, tab, id, (1, 2));
    copy_as_chord(&mut harness);
    let csv = pick(&mut harness, Key::C, "c").expect("CSV");
    assert_eq!(
        csv,
        "id,publisher_id,kind,image_data,created_at\n2,9100000000000000002,ebook,,2026-01-12 09:14:03"
    );
    assert_eq!(said(&harness).as_deref(), Some("Copied 1 row as CSV"));
}

/// Test 6: the terminal look's `gy` opens the float over a visual block,
/// a letter copies it, the visual selection is left, and the status line
/// says it.
#[test]
fn gy_opens_the_float_and_a_letter_copies_the_block() {
    let (mut harness, tab, id) = covers_in(Look::omarchy());
    rectangle(&mut harness, tab, id);
    assert!(painted(&harness, "-- VISUAL --"), "{:?}", harness.painted);
    let offers = |text: &str| {
        harness
            .painted
            .iter()
            .any(|(painted, _)| painted.contains(text))
    };
    assert!(offers("gy copy as"), "{:?}", harness.painted);
    assert!(offers("y copy tsv"), "{:?}", harness.painted);
    type_key(&mut harness, Key::G, "g");
    type_key(&mut harness, Key::Y, "y");
    assert!(menu_open(&harness));
    assert!(painted(&harness, "copy as"));
    assert!(painted(&harness, "3 rows × 2 columns"));
    assert!(painted(&harness, "m  markdown"), "{:?}", harness.painted);
    assert_eq!(pick(&mut harness, Key::M, "m").as_deref(), Some(MARKDOWN));
    assert!(!menu_open(&harness));
    let workspace = harness.app.workspace(tab).unwrap();
    assert!(!workspace.object_tab(id).unwrap().visual());
    assert!(painted(&harness, "copied 3 rows × 2 columns as markdown"));
}

/// Test 6: `ctrl+n` and `ctrl+p` move in the float, Enter picks.
#[test]
fn the_floats_keys_move_and_pick() {
    let (mut harness, tab, id) = covers_in(Look::omarchy());
    rectangle(&mut harness, tab, id);
    type_key(&mut harness, Key::G, "g");
    type_key(&mut harness, Key::Y, "y");
    harness.press(Key::N, Modifiers::CTRL);
    harness.press(Key::N, Modifiers::CTRL);
    harness.press(Key::P, Modifiers::CTRL);
    harness.copied = None;
    harness.press(Key::Enter, Modifiers::NONE);
    let tsv = harness.copied.clone().expect("TSV");
    assert!(tsv.starts_with("publisher_id\tkind\n"), "{tsv}");
}

/// Test 6: a SQL result's selection, as a table's: SQL INSERT writes into
/// `result`.
#[test]
fn a_results_selection_is_copied_into_result() {
    let mut harness = Harness::new();
    harness.set_look(Look::macos());
    let tab = harness.connect_fake();
    super::tests::with_sql_result(&mut harness, tab, 3);
    // The pointer on the first row's first cell: the result's grid takes
    // the keys.
    let tree = harness.settle();
    let row = crate::testing::bounds(&tree, "Row 1", Role::Button).expect("Row 1");
    let pos =
        row.left_center() + egui::vec2(crate::ui::grid::gutter(&harness.app.look) + 10.0, 0.0);
    let button = |pressed| egui::Event::PointerButton {
        pos,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: Modifiers::NONE,
    };
    harness.frame(vec![egui::Event::PointerMoved(pos)]);
    harness.frame(vec![button(true)]);
    harness.frame(vec![button(false)]);
    harness.settle();
    harness.press(Key::ArrowDown, Modifiers::SHIFT);
    copy_as_chord(&mut harness);
    let sql = pick(&mut harness, Key::S, "s").expect("SQL INSERT");
    assert!(sql.starts_with("INSERT INTO result ("), "{sql}");
    assert_eq!(sql.lines().count(), 3, "{sql}");
}

/// In the terminal look, a copy of a SQL result says what it copied on
/// the status line, which floats no toast.
#[test]
fn a_results_copy_is_said_on_the_terminal_status_line() {
    let mut harness = Harness::new();
    harness.set_look(Look::omarchy());
    let tab = harness.connect_fake();
    super::tests::with_sql_result(&mut harness, tab, 3);
    let tree = harness.settle();
    let row = crate::testing::bounds(&tree, "Row 1", Role::Button).expect("Row 1");
    let pos =
        row.left_center() + egui::vec2(crate::ui::grid::gutter(&harness.app.look) + 10.0, 0.0);
    let button = |pressed| egui::Event::PointerButton {
        pos,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: Modifiers::NONE,
    };
    harness.frame(vec![egui::Event::PointerMoved(pos)]);
    harness.frame(vec![button(true)]);
    harness.frame(vec![button(false)]);
    harness.settle();
    gy(&mut harness);
    assert!(menu_open(&harness), "{:?}", harness.app.dialog);
    pick(&mut harness, Key::C, "c").expect("CSV");
    let said = |(text, _): &(String, egui::Color32)| text.ends_with(" as csv");
    assert!(harness.painted.iter().any(said), "{:?}", harness.painted);
}

/// Test 6: the row panel's Copy row ▾ copies the cursor's row, whole.
#[test]
fn the_row_panels_copy_row_copies_the_row_in_a_format() {
    let (mut harness, tab, id) = covers_in(Look::macos());
    rectangle(&mut harness, tab, id);
    harness.click("Copy row ▾");
    harness.copied = None;
    harness.click("Copy row as JSON");
    let json = harness.copied.clone().expect("JSON");
    // The cursor's row alone, every column: the range is not what it takes.
    assert!(json.contains("\"id\": 3"), "{json}");
    assert!(!json.contains("\"id\": 1"), "{json}");
    assert!(json.contains("\"image_data\": null"), "{json}");
    assert_eq!(said(&harness).as_deref(), Some("Copied 1 row as JSON"));
}

/// Test 6: a secondary click outside the selection selects its cell, and
/// its menu's Copy as › copies it.
#[test]
fn the_grids_menu_copies_the_cell_under_the_pointer() {
    let (mut harness, tab, id) = covers_in(Look::macos());
    select(&mut harness, tab, id, (0, 0));
    let tree = harness.settle();
    let row = crate::testing::bounds(&tree, "Row 2", Role::Button).expect("Row 2");
    let gutter = crate::ui::grid::gutter(&harness.app.look);
    let pos = row.left_center() + egui::vec2(gutter + 10.0, 0.0);
    let button = |pressed| egui::Event::PointerButton {
        pos,
        button: egui::PointerButton::Secondary,
        pressed,
        modifiers: Modifiers::NONE,
    };
    harness.frame(vec![egui::Event::PointerMoved(pos)]);
    harness.frame(vec![button(true)]);
    harness.frame(vec![button(false)]);
    harness.settle();
    let workspace = harness.app.workspace(tab).unwrap();
    let cursor = workspace.object_tab(id).unwrap().selection;
    assert_eq!(cursor, Some(crate::model::CellPos { row: 1, col: 0 }));
    assert!(
        painted(&harness, "Copy rows as TSV"),
        "{:?}",
        harness.painted
    );
    harness.click("Copy as");
    harness.copied = None;
    harness.click("CSV");
    assert_eq!(
        harness.copied.as_deref(),
        Some(
            "id,publisher_id,kind,image_data,created_at\n2,9100000000000000002,ebook,,2026-01-12 09:14:03"
        )
    );
}

/// The Bookshop's covers with a page of `rows`, the grid holding the keys.
fn covers_of(page: tabletist_db::RowPage) -> (Harness, ConnTabId, TabId) {
    covers_of_in(Look::macos(), page)
}

/// [`covers_of`], in `look`.
fn covers_of_in(look: Look, page: tabletist_db::RowPage) -> (Harness, ConnTabId, TabId) {
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
    harness.answer_rows(page);
    let id = harness.app.workspace(tab).unwrap().active_tab.unwrap();
    harness.app.workspace_mut(tab).unwrap().pane = crate::model::Pane::Grid;
    harness.settle();
    (harness, tab, id)
}

/// Test 7: past 10,000 rows nothing is copied, by Copy as or Mod+C, and
/// the toast says why.
#[test]
fn more_than_ten_thousand_rows_copy_nothing() {
    let (mut harness, tab, id) = covers_of(crate::testing::book_covers_page(10_001));
    select(&mut harness, tab, id, (0, 0));
    harness.press(Key::A, Modifiers::COMMAND);
    harness.copied = None;
    harness.copy(false);
    assert_eq!(harness.copied, None);
    let words = "That's 10,001 rows. Export to a file?";
    assert_eq!(said(&harness).as_deref(), Some(words));
    copy_as_chord(&mut harness);
    assert_eq!(pick(&mut harness, Key::C, "c"), None);
    assert_eq!(said(&harness).as_deref(), Some(words));
}

/// Test 7: over 20 MB nothing is copied either.
#[test]
fn more_than_twenty_megabytes_copy_nothing() {
    let mut page = crate::testing::book_covers_page(2);
    page.rows[1][2] = Value::Text("w".repeat(crate::export::CLIPBOARD_BYTES).into());
    let (mut harness, tab, id) = covers_of(page);
    select(&mut harness, tab, id, (0, 0));
    harness.press(Key::A, Modifiers::COMMAND);
    copy_as_chord(&mut harness);
    assert_eq!(pick(&mut harness, Key::C, "c"), None);
    let words = "That's over 20 MB. Export to a file?";
    assert_eq!(said(&harness).as_deref(), Some(words));
    // Mod+C too.
    harness.copied = None;
    harness.copy(false);
    assert_eq!(harness.copied, None);
    assert_eq!(said(&harness).as_deref(), Some(words));
}

/// Test 7: one cell over 20 MB is not copied either.
#[test]
fn one_cell_over_twenty_megabytes_copies_nothing() {
    let mut page = crate::testing::book_covers_page(2);
    page.rows[1][2] = Value::Text("w".repeat(crate::export::CLIPBOARD_BYTES + 1).into());
    let (mut harness, tab, id) = covers_of(page);
    select(&mut harness, tab, id, (1, 2));
    harness.copied = None;
    harness.copy(false);
    assert_eq!(harness.copied, None);
    let words = "That's over 20 MB. Export to a file?";
    assert_eq!(said(&harness).as_deref(), Some(words));
}

/// Test 7: in the terminal look, a copy the grid's menu makes past the
/// limit says why on the status line, though a refused save's reason was
/// there: a pick with the pointer is no key that would end it.
#[test]
fn a_copy_past_the_limit_takes_the_line_from_a_refused_save() {
    let mut page = crate::testing::book_covers_page(2);
    page.rows[1][2] = Value::Text("w".repeat(crate::export::CLIPBOARD_BYTES + 1).into());
    let (mut harness, tab, id) = covers_of_in(Look::omarchy(), page);
    select(&mut harness, tab, id, (1, 2));
    harness.app.workspace_mut(tab).unwrap().save_refused = true;
    harness.copied = None;
    let pick = crate::model::CopyPick::Selection;
    let how = crate::model::CopyHow::As(crate::export::Format::Csv);
    harness.app.apply(Action::CopyAs { tab, pick, how });
    harness.settle();
    assert_eq!(harness.copied, None);
    assert!(!harness.app.workspace(tab).unwrap().save_refused);
    // The line names the key that exports (run 3b's R6).
    let words = "That's over 20 MB. Export to a file? · E export";
    assert_eq!(said(&harness).as_deref(), Some(words));
}

/// Test 5: `v` then `y` on a new row's cell nothing is set in copies it
/// as the NULL it is.
#[test]
fn v_y_on_an_unset_cell_of_a_new_row_copies_null() {
    let (mut harness, tab, id) = covers_in(Look::omarchy());
    let new = crate::edit::new_row(super::insert_audit_tests::add(&mut harness, tab, id));
    super::insert_audit_tests::set(&mut harness, tab, id, (new, 2), "audio");
    select(&mut harness, tab, id, (new, 4));
    type_key(&mut harness, Key::V, "v");
    harness.copied = None;
    type_key(&mut harness, Key::Y, "y");
    assert_eq!(harness.copied.as_deref(), Some("NULL"));
}

/// Test 5: Mod+C and `y` write TSV with no header row, its NULL empty.
#[test]
fn the_copy_of_a_range_is_tsv_with_no_header() {
    for look in [Look::macos(), Look::omarchy()] {
        let (mut harness, tab, id) = covers_in(look);
        select(&mut harness, tab, id, (0, 2));
        if look.terminal {
            type_key(&mut harness, Key::V, "v");
            type_key(&mut harness, Key::J, "j");
            type_key(&mut harness, Key::L, "l");
            harness.copied = None;
            type_key(&mut harness, Key::Y, "y");
        } else {
            harness.press(Key::ArrowDown, Modifiers::SHIFT);
            harness.press(Key::ArrowRight, Modifiers::SHIFT);
            harness.copied = None;
            harness.copy(false);
        }
        assert_eq!(
            harness.copied.as_deref(),
            Some("print\t\nebook\t"),
            "{}",
            look.name
        );
    }
}

/// `g` then `y`, in the terminal look.
fn gy(harness: &mut Harness) {
    type_key(harness, Key::G, "g");
    type_key(harness, Key::Y, "y");
}

/// A secondary click at `pos`.
fn secondary(harness: &mut Harness, pos: egui::Pos2) {
    let button = |pressed| egui::Event::PointerButton {
        pos,
        button: egui::PointerButton::Secondary,
        pressed,
        modifiers: Modifiers::NONE,
    };
    harness.frame(vec![egui::Event::PointerMoved(pos)]);
    harness.frame(vec![button(true)]);
    harness.frame(vec![button(false)]);
    harness.settle();
}

/// Test 6: a pending DEFAULT is written in SQL INSERT as a save sends it,
/// never as the text 'DEFAULT'.
#[test]
fn a_pending_default_is_copied_as_the_keyword() {
    let (mut harness, tab, id) = covers_in(Look::macos());
    select(&mut harness, tab, id, (1, 2));
    harness.app.apply(Action::SetDefault { tab, id });
    harness.settle();
    copy_as_chord(&mut harness);
    let sql = pick(&mut harness, Key::S, "s").expect("SQL INSERT");
    // SQLite has no DEFAULT among VALUES: the column's own default.
    assert!(
        sql.contains("(2, 9100000000000000002, ('print'), NULL,"),
        "{sql}"
    );
}

/// Test 6: Copy as on a new row with no range copies it, what is not set
/// in it NULL.
#[test]
fn copy_as_on_a_new_row_copies_it() {
    let (mut harness, tab, id) = covers_in(Look::macos());
    let new = crate::edit::new_row(super::insert_audit_tests::add(&mut harness, tab, id));
    super::insert_audit_tests::set(&mut harness, tab, id, (new, 2), "audio");
    select(&mut harness, tab, id, (new, 2));
    copy_as_chord(&mut harness);
    assert!(menu_open(&harness));
    assert_eq!(
        pick(&mut harness, Key::C, "c").as_deref(),
        Some("id,publisher_id,kind,image_data,created_at\n,,audio,,")
    );
}

/// Test 6: in SQL INSERT a new row's unset cells take their column's
/// default, as its save leaves them: on SQLite the default's expression,
/// or NULL where the column has none.
#[test]
fn sql_insert_of_a_new_row_gives_unset_cells_their_default() {
    let (mut harness, tab, id) = covers_in(Look::macos());
    let new = crate::edit::new_row(super::insert_audit_tests::add(&mut harness, tab, id));
    super::insert_audit_tests::set(&mut harness, tab, id, (new, 1), "7");
    select(&mut harness, tab, id, (new, 1));
    copy_as_chord(&mut harness);
    let sql = pick(&mut harness, Key::S, "s").expect("SQL INSERT");
    assert!(
        sql.ends_with("\n  (NULL, 7, ('print'), NULL, (CURRENT_TIMESTAMP));"),
        "{sql}"
    );
}

/// The row panel's title and the line under it end before Copy row ▾. In
/// a narrow window the button gives up its words, its name kept, and the
/// title is cut only where it still does not fit.
#[test]
fn the_row_panels_title_keeps_clear_of_copy_row() {
    let (mut harness, tab, id) = covers_in(Look::macos());
    select(&mut harness, tab, id, (1, 2));
    harness.settle();
    let whole = "Row · id 2";
    assert!(painted(&harness, whole), "{:?}", harness.painted);
    assert!(painted(&harness, "Copy row ▾"));
    harness.size.x = 700.0;
    harness.settle();
    assert!(harness.has("Copy row ▾"));
    assert!(!painted(&harness, "Copy row ▾"), "{:?}", harness.painted);
    assert!(painted(&harness, "▾"), "{:?}", harness.painted);
    let cut = |(text, _): &(String, egui::Color32)| text.starts_with("Ro") && text.ends_with('…');
    let shown = painted(&harness, whole) || harness.painted.iter().any(cut);
    assert!(shown, "{:?}", harness.painted);
}

/// Test 6: `gy` in the row panel of a new row opens nothing, as the macOS
/// look's Copy row ▾ is not drawn there: its TSV would copy nothing.
#[test]
fn gy_in_the_row_panel_of_a_new_row_opens_nothing() {
    let (mut harness, tab, id) = covers_in(Look::omarchy());
    let new = crate::edit::new_row(super::insert_audit_tests::add(&mut harness, tab, id));
    super::insert_audit_tests::set(&mut harness, tab, id, (new, 2), "audio");
    select(&mut harness, tab, id, (new, 2));
    harness.press(Key::L, Modifiers::CTRL);
    assert!(crate::ui::row_panel::focused_field(&harness.ctx, tab, id, 5).is_some());
    gy(&mut harness);
    assert!(!menu_open(&harness), "{:?}", harness.app.dialog);
}

/// Test 6: `gy` in the row panel opens the float on its row: titled for
/// it, five rows, its TSV as Mod+Shift+C's.
#[test]
fn gy_in_the_row_panel_copies_its_row() {
    let (mut harness, tab, id) = covers_in(Look::omarchy());
    select(&mut harness, tab, id, (1, 2));
    harness.press(Key::L, Modifiers::CTRL);
    assert!(crate::ui::row_panel::focused_field(&harness.ctx, tab, id, 5).is_some());
    gy(&mut harness);
    assert!(menu_open(&harness));
    assert!(painted(&harness, "copy row as"), "{:?}", harness.painted);
    assert!(!painted(&harness, "n  column names"));
    // Past the last row the keys stay on it: one up is Markdown.
    for _ in 0..6 {
        harness.press(Key::N, Modifiers::CTRL);
    }
    harness.press(Key::P, Modifiers::CTRL);
    harness.copied = None;
    harness.press(Key::Enter, Modifiers::NONE);
    let markdown = harness.copied.clone().expect("Markdown");
    assert!(markdown.starts_with("|"), "{markdown}");
    let field = crate::ui::row_panel::focused_field(&harness.ctx, tab, id, 5);
    assert!(field.is_some(), "the field keeps the keyboard");
    gy(&mut harness);
    assert!(menu_open(&harness), "{:?}", harness.app.dialog);
    assert_eq!(
        pick(&mut harness, Key::T, "t").as_deref(),
        Some("2\t9100000000000000002\tebook\t\t2026-01-12 09:14:03")
    );
    assert!(
        painted(&harness, "copied 1 row as tsv"),
        "{:?}",
        harness.painted
    );
}

/// Test 6: the row panel's Copy row as TSV says so.
#[test]
fn copy_row_as_tsv_says_so() {
    let (mut harness, tab, id) = covers_in(Look::macos());
    select(&mut harness, tab, id, (1, 2));
    harness.click("Copy row ▾");
    harness.click("Copy row as TSV");
    assert_eq!(said(&harness).as_deref(), Some("Copied 1 row as TSV"));
}

/// Test 6: a secondary click on a row's number selects that row, and its
/// menu copies it.
#[test]
fn the_grids_menu_on_a_row_number_copies_that_row() {
    let (mut harness, tab, id) = covers_in(Look::macos());
    select(&mut harness, tab, id, (0, 0));
    let tree = harness.settle();
    let row = crate::testing::bounds(&tree, "Row 3", Role::Button).expect("Row 3");
    secondary(&mut harness, row.left_center() + egui::vec2(4.0, 0.0));
    harness.click("Copy as");
    harness.copied = None;
    harness.click("CSV");
    let csv = harness.copied.clone().expect("CSV");
    assert!(
        csv.ends_with("\n3,9100000000000000003,print,,2026-01-12 09:14:03"),
        "{csv}"
    );
    assert_eq!(csv.lines().count(), 2, "{csv}");
}

/// Test 6: a secondary click inside the selection keeps it.
#[test]
fn the_grids_menu_inside_the_selection_keeps_it() {
    let (mut harness, tab, id) = covers_in(Look::macos());
    rectangle(&mut harness, tab, id);
    let tree = harness.settle();
    let row = crate::testing::bounds(&tree, "Row 2", Role::Button).expect("Row 2");
    let kind = harness.painted_rect("ebook").expect("the cell");
    secondary(&mut harness, egui::pos2(kind.center().x, row.center().y));
    harness.click("Copy as");
    harness.click("CSV");
    assert_eq!(
        said(&harness).as_deref(),
        Some("Copied 3 rows × 2 columns as CSV")
    );
}
