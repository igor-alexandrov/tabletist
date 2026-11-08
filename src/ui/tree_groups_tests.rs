//! The sidebar's prefix groups (issue #286): a group's tables one step in
//! under it, its prefix dropped where drawn, a guide along the open group,
//! the rows a tree for a screen reader, and the terminal look's folds.

use egui::accesskit::{Role, TreeUpdate};

use super::tests::hover;
use crate::model::ConnTabId;
use crate::testing::{Harness, node};
use crate::theme::Look;

/// The `book_` group's row, as a screen reader names it.
const BOOK: &str = "book_, 6 tables";

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

fn levels(tree: &TreeUpdate) -> Vec<(String, Option<usize>)> {
    tree.nodes
        .iter()
        .filter(|(_, node)| node.role() == Role::TreeItem)
        .map(|(_, node)| (node.label().unwrap_or_default().to_owned(), node.level()))
        .collect()
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
