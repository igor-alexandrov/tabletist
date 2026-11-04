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

/// Opens quick open with its key in the harness's look, and types
/// `query`.
fn quick_open(harness: &mut Harness, query: &str) {
    let held = match harness.app.look.terminal {
        true => Modifiers::CTRL,
        false => Modifiers::COMMAND,
    };
    harness.press(Key::P, held);
    assert!(matches!(harness.app.dialog, Some(Dialog::QuickOpen(_))));
    if !query.is_empty() {
        harness.frame(vec![egui::Event::Text(query.into())]);
        harness.settle();
    }
}

/// ⇧⌘E, and the terminal look's `ctrl+e`.
fn export_key(harness: &mut Harness) {
    match harness.app.look.terminal {
        true => harness.press(Key::E, Modifiers::CTRL),
        false => harness.press(Key::E, Modifiers::COMMAND | Modifiers::SHIFT),
    }
    harness.settle();
}

fn quick_open_exporting(harness: &Harness) -> Option<bool> {
    match &harness.app.dialog {
        Some(Dialog::QuickOpen(open)) => Some(open.exporting),
        _ => None,
    }
}

/// Test 2: quick open lists Export book_covers… (the table in front) and
/// Export a table…, each with what it is about; the first opens the sheet.
#[test]
fn quick_open_lists_the_export_rows() {
    let (mut harness, tab, id) = bookshop_in(Look::macos());
    quick_open(&mut harness, ">exp");
    assert!(
        painted(&harness, "Export book_covers…"),
        "{:?}",
        harness.painted
    );
    assert!(painted(&harness, "Export a table…"));
    assert!(painted(&harness, "current table"), "{:?}", harness.painted);
    assert!(painted(&harness, "pick one next"));
    harness.click("Export book_covers…, command, current table, ⇧⌘E");
    harness.settle();
    let open = sheet(&harness).expect("the sheet");
    assert_eq!((open.tab, open.id), (tab, id));
}

/// Test 2 and R3: Export a table… lists this connection's tables and
/// views alone, and ↩ opens the picked one with its sheet.
#[test]
fn export_a_table_lists_tables_and_opens_the_picked_ones_sheet() {
    let (mut harness, tab, _id) = bookshop_in(Look::macos());
    quick_open(&mut harness, ">a table");
    harness.click("Export a table…, command, pick one next");
    harness.settle();
    assert_eq!(
        quick_open_exporting(&harness),
        Some(true),
        "quick open again"
    );
    assert!(
        painted(&harness, "Pick a table to export"),
        "{:?}",
        harness.painted
    );
    // With nothing typed: the start list's tables, no SQL editor, no
    // command.
    assert!(picker_items(&harness).iter().all(Option::is_some));
    harness.frame(vec![egui::Event::Text("rev".into())]);
    harness.settle();
    let listed = picker_items(&harness);
    assert!(
        !listed.is_empty() && listed.iter().all(Option::is_some),
        "{listed:?}"
    );
    assert!(harness.has("reviews"));
    assert!(
        !painted(&harness, "Commands"),
        "no commands: {:?}",
        harness.painted
    );
    harness.press(Key::Enter, Modifiers::NONE);
    harness.settle();
    assert_eq!(front(&harness, tab).0, "reviews");
    assert!(sheet(&harness).is_some(), "its sheet");
}

/// R2: while an export runs, Export's row is its cancel and Export a
/// table… is not listed.
#[test]
fn the_export_rows_give_way_to_cancel_while_one_runs() {
    let (mut harness, _tab, _id) = bookshop_in(Look::macos());
    harness.press(Key::E, Modifiers::COMMAND | Modifiers::SHIFT);
    harness.app.apply(Action::ChangeExport(
        crate::export_sheet::ExportChange::Destination(
            crate::export_sheet::DestinationChoice::Clipboard,
        ),
    ));
    harness.app.apply(Action::StartExport);
    harness.settle();
    assert!(harness.app.exporting.is_some(), "it runs");
    quick_open(&mut harness, ">exp");
    assert!(painted(&harness, "Cancel export"), "{:?}", harness.painted);
    assert!(!painted(&harness, "Export a table…"));
    assert!(!painted(&harness, "Export book_covers…"));
}

/// Test 3: ⇧⌘E and `ctrl+e` on a picked table open it and its sheet.
#[test]
fn the_export_key_on_a_picked_table_opens_its_sheet() {
    for look in [Look::macos(), Look::omarchy()] {
        let (mut harness, tab, _id) = bookshop_in(look);
        quick_open(&mut harness, "ratings");
        export_key(&mut harness);
        assert_eq!(front(&harness, tab).0, "ratings", "{}", look.name);
        let open = sheet(&harness).expect("its sheet");
        let front_id = harness
            .app
            .workspace(tab)
            .unwrap()
            .active_object_tab()
            .unwrap()
            .id;
        assert_eq!(open.id, front_id, "{}", look.name);
    }
}

/// Test 3: on a command's row the key does nothing; quick open stays.
#[test]
fn the_export_key_on_a_command_does_nothing() {
    let (mut harness, tab, _id) = bookshop_in(Look::macos());
    quick_open(&mut harness, ">new sql");
    export_key(&mut harness);
    assert_eq!(quick_open_exporting(&harness), Some(false), "still open");
    assert!(sheet(&harness).is_none());
    assert_eq!(front(&harness, tab).0, "book_covers");
}

/// Test 3: the footer names the key while a table is picked only.
#[test]
fn the_footer_names_the_export_key_on_a_table_only() {
    for look in [Look::macos(), Look::omarchy()] {
        let (mut harness, _tab, _id) = bookshop_in(look);
        let words = match look.terminal {
            true => "ctrl+e export the picked table",
            false => "⇧⌘E export",
        };
        quick_open(&mut harness, "ratings");
        assert!(
            painted(&harness, words),
            "{}: {:?}",
            look.name,
            harness.painted
        );
        harness.app.apply(Action::CloseDialog);
        harness.settle();
        if !look.terminal {
            quick_open(&mut harness, ">new sql");
            assert!(!painted(&harness, words), "{:?}", harness.painted);
        }
    }
}

/// `book_covers` with a value past the clipboard's 20 MB in its second
/// row, in `look`, the whole page selected.
fn too_large_in(look: Look) -> (Harness, ConnTabId, TabId) {
    let mut page = crate::testing::book_covers_page(2);
    page.rows[1][2] = tabletist_db::Value::Text("w".repeat(crate::export::CLIPBOARD_BYTES).into());
    let (mut harness, tab, id) = bookshop_with(look, page);
    select(&mut harness, tab, id, (0, 0));
    harness.app.apply(Action::ExtendTo {
        tab,
        id,
        cell: crate::model::CellPos { row: 1, col: 4 },
    });
    harness.settle();
    (harness, tab, id)
}

fn copy_as_markdown(harness: &mut Harness, tab: ConnTabId) {
    let pick = crate::model::CopyPick::Selection;
    let how = crate::model::CopyHow::As(crate::export::Format::Markdown);
    harness.copied = None;
    harness.app.apply(Action::CopyAs { tab, pick, how });
    harness.settle();
}

/// Test 5 and R6: a copy over the limit copies nothing and its toast
/// offers Export…, which opens the sheet on the selection, in the copy's
/// format, to a file.
#[test]
fn copy_over_the_limit_offers_export() {
    let (mut harness, tab, id) = too_large_in(Look::macos());
    copy_as_markdown(&mut harness, tab);
    assert_eq!(harness.copied, None);
    let words = "That's over 20 MB. Export to a file?";
    assert_eq!(said(&harness).as_deref(), Some(words));
    assert!(painted(&harness, "Export…") && harness.has("Export…"));
    // The header's Export… is a button of its own: the toast's is the
    // last drawn.
    let tree = harness.settle();
    let toast = tree
        .nodes
        .iter()
        .filter(|(_, node)| node.label() == Some("Export…"))
        .count();
    assert_eq!(toast, 2, "the header's and the toast's");
    let at = crate::testing::bounds(&tree, words, Role::Label).expect("the toast");
    let button = tree
        .nodes
        .iter()
        .filter(|(_, node)| node.label() == Some("Export…"))
        .filter_map(|(_, node)| node.bounds())
        .map(|rect| {
            egui::Rect::from_min_max(
                egui::pos2(rect.x0 as f32, rect.y0 as f32),
                egui::pos2(rect.x1 as f32, rect.y1 as f32),
            )
        })
        .find(|rect| (rect.center().y - at.center().y).abs() < 20.0)
        .expect("Export… beside the words");
    let pos = button.center();
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
    let open = sheet(&harness).expect("the sheet");
    assert_eq!((open.tab, open.id), (tab, id));
    assert_eq!(open.rows, RowsChoice::Selected);
    assert_eq!(open.format, crate::export::Format::Markdown);
    assert_eq!(
        open.destination,
        crate::export_sheet::DestinationChoice::File
    );
}

/// R6: the terminal look's line names `E`, the selection stays for it,
/// and `E` opens the float on it.
#[test]
fn the_terminal_lines_offer_is_e() {
    let (mut harness, tab, id) = too_large_in(Look::omarchy());
    super::insert_audit_tests::type_key(&mut harness, Key::V, "v");
    harness.app.apply(Action::ExtendTo {
        tab,
        id,
        cell: crate::model::CellPos { row: 1, col: 4 },
    });
    copy_as_markdown(&mut harness, tab);
    assert_eq!(
        said(&harness).as_deref(),
        Some("That's over 20 MB. Export to a file? · E export")
    );
    let object = harness.app.workspace(tab).unwrap().object_tab(id).unwrap();
    assert!(object.range.is_some(), "the selection waits for E");
    super::insert_audit_tests::type_key(&mut harness, Key::E, "E");
    let open = sheet(&harness).expect("the float");
    assert_eq!(open.rows, RowsChoice::Selected);
    assert_eq!(open.format, crate::export::Format::Markdown, "the copy's");
    assert_eq!(
        open.destination,
        crate::export_sheet::DestinationChoice::File
    );
}

/// More frames than the toast's 4 s, at egui's 1/60 s each.
fn past_the_toast(harness: &mut Harness, pointer: Option<egui::Pos2>) {
    let frames = (crate::ui::toast::SECONDS * 60.0) as usize + 30;
    for _ in 0..frames {
        let events = pointer.map(egui::Event::PointerMoved).into_iter().collect();
        harness.frame(events);
    }
    harness.settle();
}

/// R6: the offer stays while the pointer is over it or its Export… has
/// the keyboard, and goes after 4 s otherwise.
#[test]
fn the_offer_stays_while_the_pointer_or_the_keyboard_is_on_it() {
    let words = "That's over 20 MB. Export to a file?";
    let (mut harness, tab, _id) = too_large_in(Look::macos());
    copy_as_markdown(&mut harness, tab);
    past_the_toast(&mut harness, None);
    assert_eq!(said(&harness), None, "4 s, then gone");
    copy_as_markdown(&mut harness, tab);
    let tree = harness.settle();
    let at = crate::testing::bounds(&tree, words, Role::Label).expect("the toast");
    past_the_toast(&mut harness, Some(at.center()));
    assert_eq!(said(&harness).as_deref(), Some(words), "under the pointer");
    harness.frame(vec![egui::Event::PointerMoved(egui::pos2(5.0, 400.0))]);
    copy_as_markdown(&mut harness, tab);
    toast_key(&mut harness);
    assert_eq!(focused_label(&mut harness).as_deref(), Some("Export…"));
    past_the_toast(&mut harness, None);
    assert_eq!(said(&harness).as_deref(), Some(words), "with the keyboard");
}

/// R6: a refused Copy rows as TSV on a block offers every column; a SQL
/// result's copy offers nothing (run 4).
#[test]
fn the_offer_follows_what_was_copied() {
    let (mut harness, tab, id) = too_large_in(Look::macos());
    select(&mut harness, tab, id, (0, 1));
    harness.app.apply(Action::ExtendTo {
        tab,
        id,
        cell: crate::model::CellPos { row: 1, col: 2 },
    });
    let pick = crate::model::CopyPick::Rows;
    let how = crate::model::CopyHow::Plain;
    harness.app.apply(Action::CopyAs { tab, pick, how });
    harness.settle();
    let offer = crate::ui::toast::offered(&harness.ctx).expect("an offer");
    harness.app.apply(Action::ExportSelection(offer));
    let open = sheet(&harness).expect("the sheet");
    assert!(!open.choose, "whole rows: every column");
    assert_eq!(open.format, crate::export::Format::Tsv);
    harness.app.apply(Action::CloseDialog);
    let mut harness = Harness::new();
    harness.set_look(Look::macos());
    let tab = harness.connect_fake();
    super::tests::with_sql_result(&mut harness, tab, crate::export::CLIPBOARD_ROWS + 1);
    let tree = harness.settle();
    let row = crate::testing::bounds(&tree, "Row 1", Role::Button).expect("Row 1");
    let gutter = crate::ui::grid::gutter(&harness.app.look);
    let pos = row.left_center() + egui::vec2(gutter + 10.0, 0.0);
    let click = |pressed| egui::Event::PointerButton {
        pos,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: Modifiers::NONE,
    };
    harness.frame(vec![egui::Event::PointerMoved(pos)]);
    harness.frame(vec![click(true)]);
    harness.frame(vec![click(false)]);
    harness.press(Key::A, Modifiers::COMMAND);
    copy_as_markdown(&mut harness, tab);
    assert!(said(&harness).is_some_and(|said| said.ends_with("Export to a file?")));
    assert_eq!(crate::ui::toast::offered(&harness.ctx), None, "no offer");
}

/// Test 6 and R7: the export's own clipboard failure offers the file, in
/// both looks.
#[test]
fn the_clipboards_failure_offers_the_file() {
    use crate::backend::export::{ExportFailure, ExportState};
    for look in [Look::macos(), Look::omarchy()] {
        let (mut harness, tab, _id) = bookshop_in(look);
        harness.app.apply(Action::OpenExport { tab, format: None });
        harness.app.apply(Action::ChangeExport(
            crate::export_sheet::ExportChange::Destination(
                crate::export_sheet::DestinationChoice::Clipboard,
            ),
        ));
        harness.app.apply(Action::StartExport);
        let job = harness.app.exporting.as_ref().unwrap().job.id;
        harness
            .app
            .apply(Action::Backend(crate::backend::Event::Export {
                job,
                state: ExportState::Failed(ExportFailure::TooLarge),
            }));
        harness.settle();
        if look.terminal {
            assert!(
                painted(&harness, "✗ that's over 20 mb · :export to a file"),
                "{:?}",
                harness.painted
            );
            harness.app.workspace_mut(tab).unwrap().command = Some("export".into());
            harness.app.apply(Action::RunCommand(tab));
        } else {
            harness.click("Export…");
        }
        harness.settle();
        let open = sheet(&harness).expect("the sheet again");
        assert_eq!(
            open.destination,
            crate::export_sheet::DestinationChoice::File,
            "{}",
            look.name
        );
        assert!(harness.app.exporting.is_none());
    }
}

/// Runs an export of `book_covers` to a file and has it end.
fn export_ends(harness: &mut Harness, tab: ConnTabId, state: crate::backend::export::ExportState) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("book_covers.csv");
    harness.app.apply(Action::ExportTo {
        tab,
        path: path.display().to_string(),
        replace: false,
    });
    harness.settle();
    let job = harness.app.exporting.as_ref().unwrap().job.id;
    harness
        .app
        .apply(Action::Backend(crate::backend::Event::Export {
            job,
            state,
        }));
    harness.settle();
}

/// The notices sent to the backend so far.
fn notices(harness: &Harness) -> Vec<(String, String)> {
    harness
        .app
        .backend
        .sent
        .iter()
        .filter_map(|command| match command {
            crate::backend::Command::Notify { summary, body } => {
                Some((summary.clone(), body.clone()))
            }
            _ => None,
        })
        .collect()
}

fn done() -> crate::backend::export::ExportState {
    crate::backend::export::ExportState::Done {
        rows: 31_400,
        bytes: 7_800_000,
        took: std::time::Duration::from_secs(18),
        copied: None,
        statements: 0,
    }
}

/// Test 7: a file export that ends while the window is behind others is
/// told once, as the board writes it.
#[test]
fn a_finished_export_behind_other_windows_is_told() {
    let (mut harness, tab, _id) = bookshop_in(Look::omarchy());
    harness.app.settings.group_digits = Some(true);
    harness.window_focus(false);
    export_ends(&mut harness, tab, done());
    assert_eq!(
        notices(&harness),
        [(
            "Tabletist".into(),
            "Exported 31,400 rows to book_covers.csv".into()
        )]
    );
    let attention = harness
        .app
        .window_asks
        .iter()
        .any(|ask| matches!(ask, egui::ViewportCommand::RequestUserAttention(_)));
    assert_eq!(attention, cfg!(any(target_os = "macos", windows)));
}

/// Test 7: none while the window is in front, and none for a failure, a
/// cancel or a copy.
#[test]
fn no_notice_while_in_front_or_for_a_failure_or_a_copy() {
    use crate::backend::export::{ExportFailure, ExportState};
    let (mut harness, tab, _id) = bookshop_in(Look::macos());
    export_ends(&mut harness, tab, done());
    assert!(notices(&harness).is_empty(), "in front");
    harness.app.exporting = None;
    harness.window_focus(false);
    export_ends(&mut harness, tab, ExportState::Cancelled);
    harness.app.exporting = None;
    export_ends(
        &mut harness,
        tab,
        ExportState::Failed(ExportFailure::ConnectionLost { rows: 2 }),
    );
    harness.app.exporting = None;
    assert!(notices(&harness).is_empty(), "a cancel, a failure");
    harness.app.apply(Action::OpenExport { tab, format: None });
    harness.app.apply(Action::ChangeExport(
        crate::export_sheet::ExportChange::Destination(
            crate::export_sheet::DestinationChoice::Clipboard,
        ),
    ));
    harness.app.apply(Action::StartExport);
    let job = harness.app.exporting.as_ref().unwrap().job.id;
    harness
        .app
        .apply(Action::Backend(crate::backend::Event::Export {
            job,
            state: ExportState::Done {
                rows: 3,
                bytes: 30,
                took: std::time::Duration::from_millis(5),
                copied: Some("id".into()),
                statements: 0,
            },
        }));
    harness.settle();
    assert!(notices(&harness).is_empty(), "a copy");
}

/// ⌘⇧', as the window sends it.
fn toast_key(harness: &mut Harness) {
    harness.press(Key::Quote, Modifiers::COMMAND | Modifiers::SHIFT);
    harness.settle();
}

/// The label of what has the keyboard.
fn focused_label(harness: &mut Harness) -> Option<String> {
    let id = harness.ctx.memory(|memory| memory.focused())?;
    let tree = harness.settle();
    tree.nodes
        .iter()
        .find(|(node, _)| *node == id.accesskit_id())
        .and_then(|(_, node)| node.label().map(str::to_owned))
}

/// Test 8 and R9: ⌘⇧' gives the running export's Cancel the keyboard, ⇥
/// moves to ×, Esc gives it back.
#[test]
fn the_toast_key_moves_the_keyboard_to_the_toast() {
    let (mut harness, tab, _id) = bookshop_in(Look::macos());
    let dir = tempfile::tempdir().unwrap();
    harness.app.apply(Action::ExportTo {
        tab,
        path: dir.path().join("book_covers.csv").display().to_string(),
        replace: false,
    });
    harness.settle();
    let before = harness.ctx.memory(|memory| memory.focused());
    toast_key(&mut harness);
    assert_eq!(focused_label(&mut harness).as_deref(), Some("Cancel"));
    harness.press(Key::Tab, Modifiers::NONE);
    harness.settle();
    assert_eq!(focused_label(&mut harness).as_deref(), Some("Hide"));
    harness.press(Key::Escape, Modifiers::NONE);
    harness.settle();
    assert_eq!(harness.ctx.memory(|memory| memory.focused()), before);
    assert!(harness.app.exporting.is_some(), "Esc cancels nothing");
    // Enter presses the button that has the keyboard.
    toast_key(&mut harness);
    harness.press(Key::Enter, Modifiers::NONE);
    let cancelled = harness
        .app
        .backend
        .sent
        .iter()
        .any(|command| matches!(command, crate::backend::Command::CancelExport(_)));
    assert!(cancelled, "Enter on Cancel cancels");
}

/// R9: with no toast that has a button, ⌘⇧' does nothing.
#[test]
fn the_toast_key_with_no_toast_does_nothing() {
    let (mut harness, _tab, _id) = bookshop_in(Look::macos());
    let before = harness.ctx.memory(|memory| memory.focused());
    toast_key(&mut harness);
    assert_eq!(harness.ctx.memory(|memory| memory.focused()), before);
}

/// What the picker lists: each result's table or view, `None` for
/// anything else.
fn picker_items(harness: &Harness) -> Vec<Option<(ConnTabId, ObjectRef, ObjectKind)>> {
    let Some(Dialog::QuickOpen(open)) = &harness.app.dialog else {
        panic!("quick open");
    };
    let list = harness.app.quick_open_list(open);
    list.items()
        .filter(|item| !matches!(item, crate::model::QuickItem::More { .. }))
        .map(crate::app::picked_object)
        .collect()
}

/// R3: in Export a table… a "more" row shows the rest, Tab keeps this
/// connection, ⌥↩ exports as ↩ does, and Esc closes.
#[test]
fn the_picker_keeps_its_rows_and_its_connection() {
    let (mut harness, tab, _id) = bookshop_in(Look::macos());
    harness.app.apply(Action::PickTableToExport(tab));
    harness.settle();
    harness.frame(vec![egui::Event::Text("s".into())]);
    harness.settle();
    let more = {
        let Some(Dialog::QuickOpen(open)) = &harness.app.dialog else {
            panic!("quick open");
        };
        let list = harness.app.quick_open_list(open);
        list.items()
            .position(|item| matches!(item, crate::model::QuickItem::More { .. }))
            .expect("a more row")
    };
    harness.app.apply(Action::QuickOpenPick {
        index: Some(more),
        in_editor: false,
    });
    harness.settle();
    let Some(Dialog::QuickOpen(open)) = &harness.app.dialog else {
        panic!("the more row keeps it open");
    };
    assert!(!open.expanded.is_empty(), "the group shows in full");
    harness.app.apply(Action::QuickOpenScope);
    let Some(Dialog::QuickOpen(open)) = &harness.app.dialog else {
        panic!("quick open");
    };
    assert!(!open.everywhere, "this connection's tables alone");
    harness.press(Key::Escape, Modifiers::NONE);
    assert!(harness.app.dialog.is_none(), "Esc closes it");
    harness.app.apply(Action::PickTableToExport(tab));
    harness.settle();
    harness.frame(vec![egui::Event::Text("ratings".into())]);
    harness.settle();
    harness.app.apply(Action::QuickOpenPick {
        index: None,
        in_editor: true,
    });
    harness.settle();
    assert_eq!(front(&harness, tab).0, "ratings");
    assert!(sheet(&harness).is_some(), "⌥↩ exports too");
}

/// R4 and run 3's D5: while an export runs, quick open's key opens
/// nothing, says why, and quick open stays.
#[test]
fn the_export_key_in_quick_open_waits_for_a_running_export() {
    let (mut harness, tab, _id) = bookshop_in(Look::macos());
    let dir = tempfile::tempdir().unwrap();
    harness.app.apply(Action::ExportTo {
        tab,
        path: dir.path().join("book_covers.csv").display().to_string(),
        replace: false,
    });
    quick_open(&mut harness, "ratings");
    export_key(&mut harness);
    assert_eq!(quick_open_exporting(&harness), Some(false), "it stays");
    assert_eq!(
        said(&harness).as_deref(),
        Some("An export is running. Cancel it first, or wait for it.")
    );
    assert_eq!(front(&harness, tab).0, "book_covers");
}

/// File › Export a Table… opens the picker, as quick open's row does.
#[test]
fn the_file_menus_export_a_table_opens_the_picker() {
    let (mut harness, _tab, _id) = bookshop_in(Look::macos());
    let item = crate::menu::Item::Command(crate::keymap::Command::ExportTable);
    assert!(harness.app.menu_state(item).enabled);
    harness.app.choose(item);
    harness.settle();
    assert_eq!(quick_open_exporting(&harness), Some(true));
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

/// R7: the clipboard's failure offers the file with the sheet's choices
/// kept, and the terminal look's line for more than 10,000 rows.
#[test]
fn the_clipboards_failure_keeps_the_sheets_choices() {
    use crate::backend::export::{ExportFailure, ExportState};
    let (mut harness, tab, _id) = bookshop_in(Look::omarchy());
    harness.app.apply(Action::OpenExport {
        tab,
        format: Some(crate::export::Format::Json),
    });
    harness.app.apply(Action::ChangeExport(
        crate::export_sheet::ExportChange::Rows(RowsChoice::Whole),
    ));
    harness.app.apply(Action::ChangeExport(
        crate::export_sheet::ExportChange::Destination(
            crate::export_sheet::DestinationChoice::Clipboard,
        ),
    ));
    harness.app.apply(Action::StartExport);
    let job = harness.app.exporting.as_ref().unwrap().job.id;
    harness
        .app
        .apply(Action::Backend(crate::backend::Event::Export {
            job,
            state: ExportState::Failed(ExportFailure::TooManyRows),
        }));
    harness.settle();
    assert!(
        painted(
            &harness,
            "✗ that's more than 10,000 rows · :export to a file"
        ),
        "{:?}",
        harness.painted
    );
    harness.app.workspace_mut(tab).unwrap().command = Some("export".into());
    harness.app.apply(Action::RunCommand(tab));
    harness.settle();
    let open = sheet(&harness).expect("the float again");
    assert_eq!(open.format, crate::export::Format::Json);
    assert_eq!(open.rows, RowsChoice::Whole);
    assert_eq!(
        open.destination,
        crate::export_sheet::DestinationChoice::File
    );
}

/// R9: ⌘⇧' goes to the first button of the toast there is: Show in
/// folder when done, the failure's action, × where it has none, the pill;
/// a toast with no button takes nothing.
#[test]
fn the_toast_key_goes_to_each_toasts_first_button() {
    use crate::backend::export::{ExportFailure, ExportState};
    let (mut harness, tab, _id) = bookshop_in(Look::macos());
    export_ends(&mut harness, tab, done());
    toast_key(&mut harness);
    assert_eq!(
        focused_label(&mut harness),
        Some(crate::ui::export_toast::reveal_words(harness.app.locale))
    );
    harness.app.exporting = None;
    export_ends(
        &mut harness,
        tab,
        ExportState::Failed(ExportFailure::ConnectionLost { rows: 2 }),
    );
    toast_key(&mut harness);
    assert_eq!(focused_label(&mut harness).as_deref(), Some("Try again"));
    harness.app.exporting = None;
    export_ends(
        &mut harness,
        tab,
        ExportState::Failed(ExportFailure::TooLarge),
    );
    harness.app.exporting = None;
    harness.settle();
    // A running export hidden to its pill.
    let dir = tempfile::tempdir().unwrap();
    harness.app.apply(Action::ExportTo {
        tab,
        path: dir.path().join("book_covers.csv").display().to_string(),
        replace: false,
    });
    harness.app.apply(Action::HideExport(true));
    harness.settle();
    toast_key(&mut harness);
    let pill = focused_label(&mut harness).unwrap_or_default();
    assert!(pill.starts_with("Exporting book_covers"), "{pill:?}");
    harness.app.exporting = None;
    harness.press(Key::Escape, Modifiers::NONE);
    // A toast of words alone.
    harness.app.said = Some("Copied 3 rows as CSV".into());
    harness.settle();
    let before = harness.ctx.memory(|memory| memory.focused());
    toast_key(&mut harness);
    assert_eq!(harness.ctx.memory(|memory| memory.focused()), before);
}

/// R9: a ⌘⇧' pressed as its toast goes takes nothing later; with an
/// export's toast and Copy as's offer both up, Tab stays in the one that
/// has the keyboard.
#[test]
fn the_toast_key_asks_once_and_tab_keeps_to_its_toast() {
    let (mut harness, tab, _id) = too_large_in(Look::macos());
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("book_covers.csv").display().to_string();
    harness.app.apply(Action::ExportTo {
        tab,
        path: path.clone(),
        replace: false,
    });
    harness.settle();
    // The toast goes in the frame ⌘⇧' comes in.
    harness.app.exporting = None;
    let chord = Modifiers::COMMAND | Modifiers::SHIFT;
    harness.frame(vec![crate::testing::key(Key::Quote, chord)]);
    harness.frame(vec![crate::testing::release(Key::Quote, chord)]);
    for _ in 0..10 {
        harness.frame(Vec::new());
    }
    let before = harness.ctx.memory(|memory| memory.focused());
    harness.app.apply(Action::ExportTo {
        tab,
        path,
        replace: true,
    });
    harness.settle();
    assert_eq!(
        harness.ctx.memory(|memory| memory.focused()),
        before,
        "nothing asked for it now"
    );
    // Both toasts: Tab goes round the export's alone.
    copy_as_markdown(&mut harness, tab);
    toast_key(&mut harness);
    let mut seen = vec![focused_label(&mut harness)];
    for _ in 0..3 {
        harness.press(Key::Tab, Modifiers::NONE);
        seen.push(focused_label(&mut harness));
    }
    let words = |text: &str| Some(text.to_owned());
    assert_eq!(
        seen,
        [
            words("Cancel"),
            words("Hide"),
            words("Cancel"),
            words("Hide")
        ]
    );
}

/// R6: the toast's Export… opens over the table copied, though another
/// came to the front since.
#[test]
fn the_offer_opens_over_the_table_copied() {
    let (mut harness, tab, id) = too_large_in(Look::macos());
    copy_as_markdown(&mut harness, tab);
    let offer = crate::ui::toast::offered(&harness.ctx).expect("an offer");
    harness.app.apply(Action::OpenObject {
        tab,
        object: ObjectRef::new("main", "reviews"),
        kind: ObjectKind::Table,
        pin: true,
    });
    harness.settle();
    assert_eq!(front(&harness, tab).0, "reviews");
    harness.app.apply(Action::ExportSelection(offer));
    harness.settle();
    assert_eq!(sheet(&harness).expect("the sheet").id, id);
    assert_eq!(front(&harness, tab).0, "book_covers");
}

/// E5: an end told twice is told once.
#[test]
fn a_done_said_twice_is_told_once() {
    let (mut harness, tab, _id) = bookshop_in(Look::macos());
    harness.window_focus(false);
    export_ends(&mut harness, tab, done());
    if let Some(exporting) = &harness.app.exporting {
        let job = exporting.job.id;
        harness
            .app
            .apply(Action::Backend(crate::backend::Event::Export {
                job,
                state: done(),
            }));
    }
    assert_eq!(notices(&harness).len(), 1);
}
