//! The table's other ways to its export (run 3b), checked against their
//! spec (`docs/superpowers/specs/2026-11-04-export-copy-as-3b-entry-points-design.md`:
//! "test N" below is an entry of its Tests): the sidebar's menu, quick
//! open, the grid's menu, Copy as over the clipboard's limit, the notice
//! and the toast's keyboard.
//!
//! Behaviour and words, on the Bookshop's tables. No picture of the design
//! is compared.

use egui::accesskit::Role;
use egui::{Key, Modifiers};
use tabletist_db::{ObjectKind, ObjectRef};

use super::insert_audit_tests::{painted, select};
use crate::export_sheet::{ExportSheet, RowsChoice};
use crate::model::{Action, ConnTabId, Dialog, ObjectView, TabId};
use crate::testing::Harness;
use crate::theme::Look;

/// The Bookshop's tables in the tree, writable, `book_covers` open with
/// its rows and the grid holding the keyboard, in `look`.
fn bookshop_in(look: Look) -> (Harness, ConnTabId, TabId) {
    bookshop_with(look, crate::testing::book_covers_page(3))
}

/// [`bookshop_in`], `book_covers` showing `page`.
fn bookshop_with(look: Look, page: tabletist_db::RowPage) -> (Harness, ConnTabId, TabId) {
    let mut harness = Harness::with_size(egui::vec2(1400.0, 1800.0));
    harness.set_look(look);
    harness.app.settings.group_digits = Some(false);
    let mut saved = crate::testing::fixture_connection();
    saved.read_only = Some(false);
    let tab = harness.connect_with(saved, crate::testing::bookshop_objects());
    harness.app.apply(Action::OpenObject {
        tab,
        object: ObjectRef::new("main", "book_covers"),
        kind: ObjectKind::Table,
        pin: true,
    });
    harness.answer_structure(crate::testing::book_covers_structure());
    harness.answer_rows(page);
    let id = harness.app.workspace(tab).unwrap().active_tab.unwrap();
    harness.app.workspace_mut(tab).unwrap().pane = crate::model::Pane::Grid;
    harness.settle();
    (harness, tab, id)
}

fn sheet(harness: &Harness) -> Option<&ExportSheet> {
    match &harness.app.dialog {
        Some(Dialog::Export(sheet)) => Some(sheet),
        _ => None,
    }
}

fn said(harness: &Harness) -> Option<String> {
    crate::ui::toast::said(&harness.ctx)
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

/// A secondary click on the tree's row of `name`.
fn tree_menu(harness: &mut Harness, name: &str) {
    let tree = harness.settle();
    let row = crate::testing::bounds(&tree, name, Role::Button)
        .unwrap_or_else(|| panic!("no tree row {name:?}"));
    secondary(harness, row.center());
}

/// A secondary click on the cell of the grid's `row` (from 1) and first
/// column.
fn grid_menu(harness: &mut Harness, row: usize) {
    let tree = harness.settle();
    let label = format!("Row {row}");
    let at = crate::testing::bounds(&tree, &label, Role::Button).expect("the row");
    let gutter = crate::ui::grid::gutter(&harness.app.look);
    secondary(harness, at.left_center() + egui::vec2(gutter + 10.0, 0.0));
}

/// The tab in front of `tab`: its table and its view.
fn front(harness: &Harness, tab: ConnTabId) -> (String, ObjectView, bool) {
    let object = harness
        .app
        .workspace(tab)
        .unwrap()
        .active_object_tab()
        .expect("a table in front");
    (object.object.name.clone(), object.view, object.pinned)
}

/// How many nodes read `label`, and how many of them cannot be acted on.
fn nodes(harness: &mut Harness, label: &str) -> (usize, usize) {
    let tree = harness.settle();
    let found: Vec<_> = tree
        .nodes
        .iter()
        .filter(|(_, node)| node.label() == Some(label))
        .collect();
    let off = found.iter().filter(|(_, node)| node.is_disabled()).count();
    (found.len(), off)
}

const MENU: [&str; 6] = [
    "Open",
    "Structure",
    "DDL",
    "Copy name",
    "Export…",
    "Import…",
];

/// Test 1: a secondary click on a table's row opens its menu: Open,
/// Structure, DDL, Copy name, Export… and Import…, greyed. In both looks.
/// It puts the tree's cursor on the row.
#[test]
fn the_sidebars_menu_holds_the_tables_ways() {
    for look in [Look::macos(), Look::omarchy()] {
        let (mut harness, tab, _id) = bookshop_in(look);
        let shown = harness.app.look;
        let label = |text: &str| shown.label(text);
        // The header's Export… and the views' switch read the same words:
        // the menu adds one of each.
        let before: Vec<(usize, usize)> = MENU
            .iter()
            .map(|row| nodes(&mut harness, &label(row)))
            .collect();
        tree_menu(&mut harness, "reviews");
        for (row, (count, off)) in MENU.iter().zip(before) {
            let after = nodes(&mut harness, &label(row));
            assert_eq!(after.0, count + 1, "{row:?} in the {} look", look.name);
            let greyed = usize::from(*row == "Import…");
            assert_eq!(after.1, off + greyed, "{row:?} in the {} look", look.name);
        }
        let cursor = harness.app.workspace(tab).unwrap().tree.cursor.clone();
        let reviews =
            crate::model::TreeNode::Object(ObjectRef::new("main", "reviews"), ObjectKind::Table);
        assert_eq!(cursor, Some(reviews), "{}", look.name);
    }
}

/// Test 1: a schema's header opens no menu.
#[test]
fn a_rows_menu_is_a_tables_or_a_views() {
    let (mut harness, tab, _id) = bookshop_in(Look::macos());
    let _ = tab;
    harness.settle();
    let schema = harness.painted_rect("MAIN").expect("the schema's header");
    let before = nodes(&mut harness, "Copy name").0;
    secondary(&mut harness, schema.center());
    assert_eq!(
        nodes(&mut harness, "Copy name").0,
        before,
        "no menu on the schema"
    );
}

/// Test 1: each row of the sidebar's menu does what it says, in both
/// looks.
#[test]
fn the_sidebars_menu_rows_do_what_they_say() {
    for look in [Look::macos(), Look::omarchy()] {
        menu_rows_in(look);
    }
}

fn menu_rows_in(look: Look) {
    let (mut harness, tab, _id) = bookshop_in(look);
    let shown = harness.app.look;
    let label = |text: &str| shown.label(text);
    // The menu's row: the last drawn of those that read so.
    let pick = |harness: &mut Harness, text: &str| {
        // The menu opens at the pointer: of the nodes that read so, the
        // nearest to it.
        let at = harness
            .ctx
            .input(|input| input.pointer.latest_pos())
            .unwrap_or_default();
        let near = |rect: &egui::accesskit::Rect| {
            let x = ((rect.x0 + rect.x1) / 2.0) as f32 - at.x;
            let y = ((rect.y0 + rect.y1) / 2.0) as f32 - at.y;
            x * x + y * y
        };
        let tree = harness.settle();
        let rect = tree
            .nodes
            .iter()
            .filter(|(_, node)| node.label() == Some(text))
            .filter_map(|(_, node)| node.bounds())
            .min_by(|one, two| near(one).total_cmp(&near(two)))
            .expect("the menu's row");
        let pos = egui::pos2(
            ((rect.x0 + rect.x1) / 2.0) as f32,
            ((rect.y0 + rect.y1) / 2.0) as f32,
        );
        let click = |pressed| egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: Modifiers::NONE,
        };
        harness.frame(vec![egui::Event::PointerMoved(pos)]);
        harness.frame(vec![click(true)]);
        harness.frame(vec![click(false)]);
        harness.settle();
    };
    tree_menu(&mut harness, "reviews");
    pick(&mut harness, &label("Open"));
    harness.settle();
    assert_eq!(
        front(&harness, tab),
        ("reviews".into(), ObjectView::Data, true)
    );
    tree_menu(&mut harness, "authors");
    pick(&mut harness, &label("Structure"));
    assert_eq!(front(&harness, tab).0, "authors");
    assert_eq!(front(&harness, tab).1, ObjectView::Structure);
    tree_menu(&mut harness, "editions");
    pick(&mut harness, &label("DDL"));
    assert_eq!(front(&harness, tab).0, "editions");
    assert_eq!(front(&harness, tab).1, ObjectView::Ddl);
    tree_menu(&mut harness, "ratings");
    harness.copied = None;
    pick(&mut harness, &label("Copy name"));
    assert_eq!(harness.copied.as_deref(), Some("ratings"));
    assert_eq!(said(&harness), Some(label("Copied ratings")));
    assert_eq!(
        front(&harness, tab).0,
        "editions",
        "Copy name opens nothing"
    );
    tree_menu(&mut harness, "publishers");
    pick(&mut harness, &label("Export…"));
    assert_eq!(front(&harness, tab).0, "publishers");
    let open = sheet(&harness).expect("Export… opens the sheet over it");
    let front_id = harness
        .app
        .workspace(tab)
        .unwrap()
        .active_object_tab()
        .unwrap()
        .id;
    assert_eq!(open.id, front_id);
}

/// Test 4: Export selection… opens the sheet on the selected rows, every
/// column.
#[test]
fn export_selection_opens_the_sheet_on_the_selection() {
    let (mut harness, tab, id) = bookshop_in(Look::macos());
    harness.app.apply(Action::RowNumberClicked {
        tab,
        id,
        row: 0,
        click: crate::selection::RowClick::Only,
    });
    harness.app.apply(Action::RowNumberClicked {
        tab,
        id,
        row: 2,
        click: crate::selection::RowClick::Extend,
    });
    harness.settle();
    grid_menu(&mut harness, 2);
    assert!(
        painted(&harness, "Export selection…"),
        "{:?}",
        harness.painted
    );
    harness.click("Export selection…");
    harness.settle();
    let open = sheet(&harness).expect("the sheet");
    assert_eq!(open.rows, RowsChoice::Selected);
    assert!(!open.choose, "whole rows: every column");
    assert_eq!(harness.app.export_facts().unwrap().selected, 3);
}

/// R1: a block of cells opens Choose with its columns ticked, first, in
/// the grid's order; ⇧⌘E and the terminal look's `E` do the same.
#[test]
fn a_block_opens_choose_with_its_columns() {
    for look in [Look::macos(), Look::omarchy()] {
        let (mut harness, tab, id) = bookshop_in(look);
        select(&mut harness, tab, id, (0, 2));
        harness.app.apply(Action::ExtendTo {
            tab,
            id,
            cell: crate::model::CellPos { row: 1, col: 1 },
        });
        harness.settle();
        if harness.app.look.terminal {
            super::insert_audit_tests::type_key(&mut harness, Key::E, "E");
        } else {
            harness.press(Key::E, Modifiers::COMMAND | Modifiers::SHIFT);
        }
        harness.settle();
        let open = sheet(&harness).expect("the sheet");
        assert_eq!(open.rows, RowsChoice::Selected, "{}", look.name);
        assert!(open.choose, "{}", look.name);
        let columns: Vec<(&str, bool)> = open
            .columns
            .iter()
            .map(|column| (column.name.as_str(), column.included))
            .collect();
        assert_eq!(
            columns,
            [
                ("publisher_id", true),
                ("kind", true),
                ("id", false),
                ("image_data", false),
                ("created_at", false),
            ],
            "{}",
            look.name
        );
    }
}

/// Test 4: Export selection… on a cell with no range exports its row, and
/// ⇧⌘E there still starts on the rows matching the filter, as before.
#[test]
fn a_single_cell_exports_its_row() {
    let (mut harness, tab, id) = bookshop_in(Look::macos());
    select(&mut harness, tab, id, (1, 0));
    harness.press(Key::E, Modifiers::COMMAND | Modifiers::SHIFT);
    assert_eq!(sheet(&harness).unwrap().rows, RowsChoice::Matching);
    harness.app.apply(Action::CloseDialog);
    grid_menu(&mut harness, 2);
    harness.click("Export selection…");
    harness.settle();
    let open = sheet(&harness).expect("the sheet");
    assert_eq!(open.rows, RowsChoice::Selected);
    harness.app.apply(Action::ChangeExport(
        crate::export_sheet::ExportChange::Destination(
            crate::export_sheet::DestinationChoice::Clipboard,
        ),
    ));
    harness.app.apply(Action::StartExport);
    let job = harness
        .app
        .backend
        .sent
        .iter()
        .find_map(|command| match command {
            crate::backend::Command::Export(job) => Some(job.clone()),
            _ => None,
        })
        .expect("an export");
    let crate::backend::export::ExportRows::Loaded { rows, .. } = &job.rows else {
        panic!("loaded rows");
    };
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0][0], tabletist_db::Value::Int(2), "the cursor's row");
}

/// E3: a SQL result's grid keeps run 2's menu, with no Export selection….
#[test]
fn a_results_grid_menu_has_no_export() {
    let mut harness = Harness::new();
    harness.set_look(Look::macos());
    let tab = harness.connect_fake();
    super::tests::with_sql_result(&mut harness, tab, 3);
    grid_menu(&mut harness, 1);
    assert!(
        painted(&harness, "Copy rows as TSV"),
        "{:?}",
        harness.painted
    );
    assert!(!painted(&harness, "Export selection…"));
}

/// R1: the header's Export… on a block opens Choose with its columns, as
/// ⇧⌘E does.
#[test]
fn the_headers_export_on_a_block_chooses_its_columns() {
    let (mut harness, tab, id) = bookshop_in(Look::macos());
    select(&mut harness, tab, id, (0, 3));
    harness.app.apply(Action::ExtendTo {
        tab,
        id,
        cell: crate::model::CellPos { row: 2, col: 4 },
    });
    harness.settle();
    harness.click("Export…");
    harness.settle();
    let open = sheet(&harness).expect("the sheet");
    assert!(open.choose);
    let ticked: Vec<&str> = open
        .columns
        .iter()
        .filter(|column| column.included)
        .map(|column| column.name.as_str())
        .collect();
    assert_eq!(ticked, ["image_data", "created_at"]);
}
