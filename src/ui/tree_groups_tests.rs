//! The sidebar's prefix groups (issue #286): a group's tables one step in
//! under it, its prefix dropped where drawn, a guide along the open group,
//! the rows a tree for a screen reader, and the terminal look's folds.

use egui::accesskit::{Role, TreeUpdate};
use egui::{Key, Modifiers, Rect};
use tabletist_db::{ObjectKind, ObjectRef};

use super::sidebar::{GUIDE, INDENT, NAME_IN, PAST_GUIDE};
use super::tests::{hover, type_key};
use crate::model::{Action, ConnTabId, TreeNode};
use crate::testing::{Harness, bounds, node};
use crate::theme::Look;
use crate::typography::TextRole;

/// The `book_` group's row, as a screen reader names it.
const BOOK: &str = "book_, 6 tables";

/// Where the macOS chevron begins in its row: the tree's inset, then 8.
const FOLD: f32 = 8.0 + 8.0;

/// A connected tab in `look` whose `main` schema lists the Bookshop's
/// grouped catalog, with `book_` open.
fn grouped(look: Look) -> (Harness, ConnTabId) {
    let mut harness = Harness::new();
    harness.set_look(look);
    let tab = harness.connect_fake();
    let workspace = harness.app.workspace_mut(tab).unwrap();
    workspace.tree.schemas.value = Some(vec!["main".into()]);
    let node = workspace.tree.nodes.entry("main".into()).or_default();
    node.expanded = true;
    node.objects.value = Some(crate::testing::bookshop_grouped_objects());
    node.open_groups.insert("book".into());
    harness.settle();
    (harness, tab)
}

fn table(name: &str) -> TreeNode {
    TreeNode::Object(ObjectRef::new("main", name), ObjectKind::Table)
}

fn group(prefix: &str) -> TreeNode {
    TreeNode::Group("main".into(), prefix.into())
}

/// Where the row a screen reader names `name` sits.
fn row(harness: &mut Harness, name: &str) -> Rect {
    let tree = harness.settle();
    bounds(&tree, name, Role::TreeItem).unwrap_or_else(|| panic!("no row {name:?}"))
}

/// Where the last frame began to paint `text`.
fn drawn(harness: &Harness, text: &str) -> f32 {
    harness
        .painted_rect(text)
        .unwrap_or_else(|| panic!("{text:?} is not drawn"))
        .left()
}

fn close(a: f32, b: f32) -> bool {
    (a - b).abs() < 0.5
}

/// The pieces of the macOS guide the last frame drew: 1 pt wide, in the
/// subtle border's colour.
fn guides(harness: &Harness) -> Vec<Rect> {
    let color = harness.app.palette.surface_active;
    harness
        .fills
        .iter()
        .filter(|(rect, fill)| *fill == color && close(rect.width(), 1.0))
        .map(|(rect, _)| *rect)
        .collect()
}

/// Where the terminal look drew its `│` guides.
fn bars(harness: &Harness) -> Vec<Rect> {
    harness
        .text_rects
        .iter()
        .filter(|(text, _)| text == "│")
        .map(|(_, rect)| *rect)
        .collect()
}

/// One character of the terminal's face.
fn cell(harness: &Harness) -> f32 {
    TextRole::OBody.width(&harness.ctx, harness.app.look.faces, " ")
}

fn cursor(harness: &Harness, tab: ConnTabId) -> Option<TreeNode> {
    harness.app.workspace(tab).unwrap().tree.cursor.clone()
}

fn open_groups(harness: &Harness, tab: ConnTabId) -> Vec<String> {
    let mut open: Vec<String> = harness.app.workspace(tab).unwrap().tree.nodes["main"]
        .open_groups
        .iter()
        .cloned()
        .collect();
    open.sort();
    open
}

fn levels(tree: &TreeUpdate) -> Vec<(String, Option<usize>)> {
    tree.nodes
        .iter()
        .filter(|(_, node)| node.role() == Role::TreeItem)
        .map(|(_, node)| (node.label().unwrap_or_default().to_owned(), node.level()))
        .collect()
}

#[test]
fn grouped_tables_stand_in_from_the_groups_name_and_single_tables_line_up_with_it() {
    let (mut harness, _) = grouped(Look::macos());
    let child = row(&mut harness, "book_covers, table");
    let header = row(&mut harness, BOOK);
    // The group's name one step past its chevron.
    let name = drawn(&harness, "book_");
    assert!(close(name - header.left(), FOLD + INDENT), "{name}");
    // A grouped table's name further in: past the guide, its highlight
    // and the room inside it. 39 in, the group's name at 34.
    let covers = drawn(&harness, "covers") - child.left();
    assert!(
        close(covers, FOLD + GUIDE + PAST_GUIDE + NAME_IN),
        "{covers}"
    );
    assert!(close(covers, 39.0) && close(name - header.left(), 34.0));
    // Single tables in the group's name's column.
    for single in ["authors", "orders", "reviews"] {
        assert!(close(drawn(&harness, single), name), "{single}");
    }
}

#[test]
fn the_terminal_indents_a_groups_tables_two_cells_and_single_tables_with_them() {
    let (mut harness, _) = grouped(Look::omarchy());
    harness.settle();
    let name = drawn(&harness, "book_");
    let covers = drawn(&harness, "covers");
    assert!(
        close(covers - name, 2.0 * cell(&harness)),
        "{name} {covers}"
    );
    for single in ["authors", "orders", "reviews"] {
        assert!(close(drawn(&harness, single), covers), "{single}");
    }
}

#[test]
fn a_grouped_tables_highlight_begins_inside_the_indent() {
    let (mut harness, _) = grouped(Look::macos());
    let selection = harness.app.palette.selection;
    let pill = |harness: &Harness, row: Rect| {
        harness
            .fills
            .iter()
            .find(|(rect, fill)| *fill == selection && row.contains(rect.center()))
            .map(|(rect, _)| *rect)
            .expect("the selected row's pill")
    };
    harness.click("book_covers, table");
    let child = row(&mut harness, "book_covers, table");
    // 4 past the 1 pt guide, which stands 4 into the chevron: 25 in.
    let begins = pill(&harness, child).left() - child.left();
    assert!(close(begins, FOLD + GUIDE + PAST_GUIDE), "{begins}");
    assert!(close(begins, 25.0), "{begins}");
    // A single table's pill fills the tree's inset, as before.
    harness.click("orders, table");
    let single = row(&mut harness, "orders, table");
    let begins = pill(&harness, single).left() - single.left();
    assert!(close(begins, 8.0), "{begins}");
}

#[test]
fn a_grouped_table_drops_its_prefix_where_drawn_and_keeps_it_for_a_screen_reader() {
    let (mut harness, _) = grouped(Look::macos());
    assert!(harness.painted_rect("covers").is_some());
    assert!(harness.painted_rect("book_covers").is_none());
    let tree = harness.settle();
    assert!(node(&tree, "book_covers, table", Role::TreeItem).is_some());
    assert!(node(&tree, "book_excerpts, view", Role::TreeItem).is_some());
    assert!(node(&tree, BOOK, Role::TreeItem).is_some());
    assert!(!crate::testing::labels(&tree).contains(&"book_covers".to_owned()));
    // The pointer over it shows the full name.
    let at = harness.painted_rect("covers").unwrap().center();
    assert!(hover(&mut harness, at).contains(&"book_covers".to_owned()));
}

#[test]
fn the_tree_says_each_rows_level_and_whether_a_group_is_open() {
    let (mut harness, _) = grouped(Look::macos());
    let tree = harness.settle();
    let tree_node = node(&tree, "Objects", Role::Tree);
    assert!(tree_node.is_some(), "the sidebar's list is a tree");
    let levels = levels(&tree);
    let level = |name: &str| {
        levels
            .iter()
            .find(|(label, _)| label == name)
            .and_then(|(_, level)| *level)
    };
    assert_eq!(level(BOOK), Some(1));
    assert_eq!(level("authors, table"), Some(1));
    assert_eq!(level("book_covers, table"), Some(2));
    assert_eq!(level("publisher_, 2 tables"), Some(1));
    let expanded = |name: &str| {
        let id = node(&tree, name, Role::TreeItem).unwrap();
        let (_, found) = tree.nodes.iter().find(|(node, _)| *node == id).unwrap();
        found.is_expanded()
    };
    assert_eq!(expanded(BOOK), Some(true));
    assert_eq!(expanded("publisher_, 2 tables"), Some(false));
    assert_eq!(expanded("orders, table"), None);
}

#[test]
fn a_longer_prefix_stays_in_its_group_and_groups_never_nest() {
    let (harness, tab) = grouped(Look::macos());
    let workspace = harness.app.workspace(tab).unwrap();
    let rows = workspace.tree.visible_rows(workspace.driver, false);
    let sizes = rows
        .iter()
        .find(|row| row.node == table("book_cover_sizes"))
        .expect("book_cover_sizes");
    assert_eq!((sizes.depth, sizes.label.as_str()), (1, "cover_sizes"));
    let groups: Vec<&TreeNode> = rows
        .iter()
        .map(|row| &row.node)
        .filter(|node| matches!(node, TreeNode::Group(..)))
        .collect();
    assert_eq!(groups, [&group("book"), &group("publisher")]);
}

#[test]
fn groups_and_single_tables_sort_together() {
    let (harness, tab) = grouped(Look::macos());
    let workspace = harness.app.workspace(tab).unwrap();
    let top: Vec<String> = workspace
        .tree
        .visible_rows(workspace.driver, false)
        .into_iter()
        .filter(|row| row.depth == 0)
        .map(|row| row.label)
        .collect();
    assert_eq!(top, ["authors", "book_", "orders", "publisher_", "reviews"]);
}

#[test]
fn left_and_right_close_and_open_a_group_and_left_climbs_to_it() {
    let (mut harness, tab) = grouped(Look::macos());
    harness.click("book_covers, table");
    harness.press(Key::ArrowLeft, Modifiers::NONE);
    assert_eq!(cursor(&harness, tab), Some(group("book")));
    harness.press(Key::ArrowLeft, Modifiers::NONE);
    assert_eq!(open_groups(&harness, tab), Vec::<String>::new());
    harness.press(Key::ArrowRight, Modifiers::NONE);
    assert_eq!(open_groups(&harness, tab), ["book"]);
    harness.press(Key::ArrowRight, Modifiers::NONE);
    assert_eq!(cursor(&harness, tab), Some(table("book_awards")));
}

#[test]
fn omarchys_folds_work_on_the_groups_and_t_still_flattens() {
    let (mut harness, tab) = grouped(Look::omarchy());
    harness.app.apply(Action::SetTreeCursor {
        tab,
        node: table("book_covers"),
    });
    // `za` on a table folds its group, and the cursor goes up to it.
    type_key(&mut harness, Key::Z, "z");
    type_key(&mut harness, Key::A, "a");
    assert_eq!(open_groups(&harness, tab), Vec::<String>::new());
    assert_eq!(cursor(&harness, tab), Some(group("book")));
    // On the group it opens it again.
    type_key(&mut harness, Key::Z, "z");
    type_key(&mut harness, Key::A, "a");
    assert_eq!(open_groups(&harness, tab), ["book"]);
    // `zR` opens every group, `zM` folds them all.
    type_key(&mut harness, Key::Z, "z");
    type_key(&mut harness, Key::R, "R");
    assert_eq!(open_groups(&harness, tab), ["book", "publisher"]);
    harness.app.apply(Action::SetTreeCursor {
        tab,
        node: table("publisher_imprints"),
    });
    type_key(&mut harness, Key::Z, "z");
    type_key(&mut harness, Key::M, "M");
    assert_eq!(open_groups(&harness, tab), Vec::<String>::new());
    assert_eq!(cursor(&harness, tab), Some(group("publisher")));
    type_key(&mut harness, Key::T, "t");
    assert!(harness.app.workspace(tab).unwrap().tree.flat);
}

#[test]
fn the_guide_runs_along_the_open_groups_tables_and_no_further() {
    let (mut harness, tab) = grouped(Look::macos());
    let first = row(&mut harness, "book_awards, table");
    let last = row(&mut harness, "book_genres, table");
    let pieces = guides(&harness);
    assert_eq!(pieces.len(), 6, "a piece on each of the group's tables");
    let top = pieces
        .iter()
        .map(|rect| rect.top())
        .fold(f32::MAX, f32::min);
    let bottom = pieces
        .iter()
        .map(|rect| rect.bottom())
        .fold(f32::MIN, f32::max);
    assert!(close(top, first.top()) && close(bottom, last.bottom()));
    // 4 into the chevron: 20 in.
    assert!(
        pieces
            .iter()
            .all(|rect| close(rect.left(), first.left() + FOLD + GUIDE))
    );
    assert!(close(FOLD + GUIDE, 20.0));
    // A folded group has none.
    harness.app.apply(Action::ToggleGroup {
        tab,
        schema: "main".into(),
        prefix: "book".into(),
    });
    harness.settle();
    assert!(guides(&harness).is_empty());

    // The terminal's `│`, under the group's name.
    let (mut harness, _) = grouped(Look::omarchy());
    harness.settle();
    let name = drawn(&harness, "book_");
    let bars = bars(&harness);
    assert_eq!(bars.len(), 6);
    assert!(bars.iter().all(|rect| close(rect.left(), name)));
}

#[test]
fn the_flat_list_keeps_full_names_with_no_groups_or_guides() {
    for look in [Look::macos(), Look::omarchy()] {
        let (mut harness, tab) = grouped(look);
        harness.app.apply(Action::ToggleFlatTree(tab));
        let tree = harness.settle();
        assert!(node(&tree, BOOK, Role::TreeItem).is_none(), "{}", look.name);
        assert!(levels(&tree).iter().all(|(_, level)| *level == Some(1)));
        assert!(guides(&harness).is_empty() && bars(&harness).is_empty());
        let covers = row(&mut harness, "book_covers, table");
        let x = drawn(&harness, "book_covers");
        assert!(close(x, drawn(&harness, "authors")), "{}", look.name);
        // Where a flat list has always drawn its names.
        let glyph = if look.terminal {
            TextRole::OBody.width(&harness.ctx, harness.app.look.faces, "▾")
        } else {
            12.0
        };
        let inset = if look.terminal { 4.0 } else { 8.0 };
        assert!(
            close(x - covers.left(), inset + 8.0 + glyph + 6.0),
            "{}",
            look.name
        );
    }
}

#[test]
fn the_indent_scales_with_the_windows_zoom() {
    for zoom in [1.0, 1.5, 2.0] {
        let (mut harness, _) = grouped(Look::macos());
        harness.ctx.set_zoom_factor(zoom);
        harness.settle();
        // Points stay the same; the screen's pixels grow with the zoom.
        let guide = guides(&harness)[0];
        let step = drawn(&harness, "covers") - guide.left();
        let points = PAST_GUIDE + NAME_IN;
        assert!(close(step, points), "{zoom}: {step}");
        assert!(close(step * harness.ctx.pixels_per_point(), points * zoom));
        assert!(close(guide.width() * harness.ctx.pixels_per_point(), zoom));
    }
}

#[test]
fn the_terminal_status_line_names_the_table_under_the_cursor_in_full() {
    let (mut harness, tab) = grouped(Look::omarchy());
    harness.app.apply(Action::OpenObject {
        tab,
        object: ObjectRef::new("main", "orders"),
        kind: ObjectKind::Table,
        pin: true,
    });
    harness.answer_rows(crate::testing::page(3, false));
    harness.app.apply(Action::SetTreeCursor {
        tab,
        node: table("book_covers"),
    });
    harness.settle();
    assert!(
        harness
            .painted_rect("book_covers · table · 13 rows")
            .is_some()
    );
    // A view has no estimate: no rows said.
    harness.app.apply(Action::SetTreeCursor {
        tab,
        node: TreeNode::Object(ObjectRef::new("main", "book_excerpts"), ObjectKind::View),
    });
    harness.settle();
    assert!(harness.painted_rect("book_excerpts · view").is_some());
}

#[test]
fn with_no_table_open_the_start_line_names_the_table_under_the_cursor() {
    let (mut harness, tab) = grouped(Look::omarchy());
    harness.app.apply(Action::SetTreeCursor {
        tab,
        node: table("book_covers"),
    });
    harness.settle();
    assert!(
        harness
            .painted_rect("book_covers · table · 13 rows")
            .is_some()
    );
    assert!(harness.painted_rect("no table open").is_none());
}

#[test]
fn a_letter_after_a_folds_first_key_is_its_own_where_no_fold_goes_on() {
    let (mut harness, tab) = grouped(Look::omarchy());
    harness.app.apply(Action::SetTreeCursor {
        tab,
        node: table("book_covers"),
    });
    type_key(&mut harness, Key::Z, "z");
    harness.frame(vec![egui::Event::Text("?".into())]);
    assert!(
        harness.has("Keyboard shortcuts"),
        "`z?` is no fold: `?` opens help"
    );
}

#[test]
fn a_groups_name_is_said_as_it_is_written() {
    let (mut harness, tab) = grouped(Look::macos());
    let objects = ["{n}_a", "{n}_b"].map(|name| tabletist_db::ObjectInfo {
        name: name.into(),
        kind: ObjectKind::Table,
        estimated_rows: None,
    });
    let workspace = harness.app.workspace_mut(tab).unwrap();
    workspace.tree.nodes.get_mut("main").unwrap().objects.value = Some(objects.to_vec());
    let tree = harness.settle();
    assert!(node(&tree, "{n}_, 2 tables", Role::TreeItem).is_some());
}

#[test]
fn the_status_line_says_nothing_of_a_row_the_sidebar_hides() {
    let (mut harness, tab) = grouped(Look::omarchy());
    harness.app.apply(Action::SetTreeCursor {
        tab,
        node: table("book_covers"),
    });
    // Its group folded by a click, the cursor left on the hidden row.
    harness.app.apply(Action::ToggleGroup {
        tab,
        schema: "main".into(),
        prefix: "book".into(),
    });
    harness.settle();
    assert!(
        harness
            .painted_rect("book_covers · table · 13 rows")
            .is_none()
    );
    assert!(harness.painted_rect("no table open").is_some());
}

#[test]
fn a_long_name_on_the_start_line_gives_way_at_its_end() {
    let mut harness = Harness::with_size(egui::vec2(720.0, 480.0));
    harness.set_look(Look::omarchy());
    let tab = harness.connect_fake();
    let long = format!("book_{}", "x".repeat(200));
    let workspace = harness.app.workspace_mut(tab).unwrap();
    workspace.tree.schemas.value = Some(vec!["main".into()]);
    let node = workspace.tree.nodes.entry("main".into()).or_default();
    node.expanded = true;
    node.objects.value = Some(vec![tabletist_db::ObjectInfo {
        name: long.clone(),
        kind: ObjectKind::Table,
        estimated_rows: None,
    }]);
    harness.app.apply(Action::SetTreeCursor {
        tab,
        node: table(&long),
    });
    harness.settle();
    let said = format!("{long} · table");
    // Said whole to a screen reader, drawn cut inside the window.
    assert!(harness.has(&said));
    let (drawn, rect) = harness
        .text_rects
        .iter()
        // On the status line, at the window's foot, not the sidebar's row.
        .find(|(text, rect)| text.starts_with("book_xxx") && rect.top() > 440.0)
        .cloned()
        .expect("the name drawn on the status line");
    assert!(drawn.ends_with('…'), "{drawn}");
    assert!(rect.right() <= 720.0 && rect.left() >= 0.0, "{rect:?}");
}

#[test]
fn folding_or_opening_every_group_reaches_those_a_filter_hides() {
    let (mut harness, tab) = grouped(Look::omarchy());
    harness.app.apply(Action::ToggleGroup {
        tab,
        schema: "main".into(),
        prefix: "publisher".into(),
    });
    assert_eq!(open_groups(&harness, tab), ["book", "publisher"]);
    // The filter shows `book_awards` alone: `publisher_` is hidden.
    let filter = |harness: &mut Harness, text: &str| {
        harness.app.workspace_mut(tab).unwrap().tree.filter = text.into();
    };
    filter(&mut harness, "awards");
    harness.app.apply(Action::TreeKey {
        tab,
        key: crate::model::TreeKey::FoldAll,
    });
    filter(&mut harness, "");
    assert_eq!(open_groups(&harness, tab), Vec::<String>::new());
    filter(&mut harness, "awards");
    harness.app.apply(Action::TreeKey {
        tab,
        key: crate::model::TreeKey::OpenAll,
    });
    filter(&mut harness, "");
    assert_eq!(open_groups(&harness, tab), ["book", "publisher"]);
}
