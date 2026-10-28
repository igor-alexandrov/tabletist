//! Which node a step's label means. A tree holds several of one name: a
//! table's is a tree item, a tab and a heading once it is open, and a
//! column's is its header and a choice in a filter's menu. egui hands its
//! nodes over in no set order, so the first of them is any of them.
//!
//! - `open` takes the sidebar tree's items alone ([`tree_items`]).
//! - `click` takes the node with the label, whatever its role
//!   ([`candidates`]).
//! - Of several, [`choose`] takes the one in an area over the window (a
//!   popup, a menu, a dialog), which a click there would reach. Otherwise
//!   none: the runner then names them all, for the scenario to say another.
//!
//! No GPU here, so the tests run in the normal suite.

use std::collections::HashMap;

use egui::accesskit::{NodeId, Role, TreeUpdate};

/// A node a step's label could mean.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Candidate {
    pub role: Role,
    /// In window points.
    pub rect: egui::Rect,
    /// In an area over the window (a popup, a menu, a dialog), not in the
    /// window's own UI.
    pub over: bool,
}

/// What a label picked.
#[derive(Debug, PartialEq)]
pub enum Pick {
    /// No node with bounds has the label.
    Nothing,
    One(egui::Rect),
    /// Several, and none of them is the only one over the window.
    Ambiguous(Vec<Candidate>),
}

/// The nodes called `label` that have bounds, whatever their role. Plain
/// labels carry their text as the value, so that counts as the name too.
pub fn candidates(tree: &TreeUpdate, label: &str) -> Vec<Candidate> {
    let parents: HashMap<NodeId, NodeId> = tree
        .nodes
        .iter()
        .flat_map(|(id, node)| node.children().iter().map(move |child| (*child, *id)))
        .collect();
    let roles: HashMap<NodeId, Role> = tree
        .nodes
        .iter()
        .map(|(id, node)| (*id, node.role()))
        .collect();
    tree.nodes
        .iter()
        .filter(|(_, node)| node.label().or_else(|| node.value()) == Some(label))
        .filter_map(|(id, node)| {
            let bounds = node.bounds()?;
            Some(Candidate {
                role: node.role(),
                rect: egui::Rect::from_min_max(
                    egui::pos2(bounds.x0 as f32, bounds.y0 as f32),
                    egui::pos2(bounds.x1 as f32, bounds.y1 as f32),
                ),
                over: over(&parents, &roles, *id),
            })
        })
        .collect()
}

/// Whether `id` is in an area over the window. egui gives the root one
/// child for the window's own UI, a container, and one for each area over
/// it, with no role: the child that holds `id` says which.
fn over(parents: &HashMap<NodeId, NodeId>, roles: &HashMap<NodeId, Role>, id: NodeId) -> bool {
    let mut top = id;
    while let Some(parent) = parents.get(&top) {
        if !parents.contains_key(parent) {
            // `parent` is the root.
            break;
        }
        top = *parent;
    }
    roles.get(&top) != Some(&Role::GenericContainer)
}

/// The sidebar tree's items called `object`: the buttons inside the group
/// called "Objects", the tree's one Tab stop (`ui::sidebar`).
pub fn tree_items(tree: &TreeUpdate, object: &str) -> Vec<Candidate> {
    let Some(area) = candidates(tree, "Objects")
        .into_iter()
        .find(|candidate| candidate.role == Role::Group)
    else {
        return Vec::new();
    };
    candidates(tree, object)
        .into_iter()
        .filter(|item| item.role == Role::Button && area.rect.contains(item.rect.center()))
        .collect()
}

/// The one of `candidates` a click means: the only one, or else the only
/// one over the window.
pub fn choose(candidates: Vec<Candidate>) -> Pick {
    match candidates.as_slice() {
        [] => return Pick::Nothing,
        [one] => return Pick::One(one.rect),
        _ => {}
    }
    let over: Vec<&Candidate> = candidates.iter().filter(|one| one.over).collect();
    match over.as_slice() {
        [one] => Pick::One(one.rect),
        _ => Pick::Ambiguous(candidates),
    }
}

#[cfg(test)]
mod tests {
    use egui::accesskit::{Node, Tree, TreeId};

    use super::*;

    /// A node as the test lays it out: its role, label, bounds and
    /// children.
    struct Laid(u64, Role, Option<&'static str>, Option<[f64; 4]>, Vec<u64>);

    fn tree(laid: Vec<Laid>) -> TreeUpdate {
        let nodes = laid
            .into_iter()
            .map(|Laid(id, role, label, bounds, children)| {
                let mut node = Node::new(role);
                if let Some(label) = label {
                    node.set_label(label);
                }
                if let Some([x0, y0, x1, y1]) = bounds {
                    node.set_bounds(egui::accesskit::Rect { x0, y0, x1, y1 });
                }
                node.set_children(children.into_iter().map(NodeId).collect::<Vec<_>>());
                (NodeId(id), node)
            })
            .collect();
        TreeUpdate {
            nodes,
            tree: Some(Tree::new(NodeId(0))),
            tree_id: TreeId::ROOT,
            focus: NodeId(0),
        }
    }

    /// The window with `books` open and a filter's column menu showing,
    /// as the runner saw it: the tree's item, the tab's title, the
    /// heading, the column header and the menu's choice.
    fn window() -> TreeUpdate {
        use Role::{Button, GenericContainer, Group, Label, Unknown, Window};
        tree(vec![
            Laid(0, Window, None, None, vec![1, 9]),
            Laid(1, GenericContainer, None, None, vec![2, 3, 4, 5, 6, 7, 8]),
            Laid(
                2,
                Group,
                Some("Objects"),
                Some([0.0, 156.0, 248.0, 651.0]),
                vec![],
            ),
            Laid(
                3,
                Button,
                Some("books"),
                Some([0.0, 204.0, 248.0, 228.0]),
                vec![],
            ),
            Laid(
                4,
                Label,
                Some("books"),
                Some([269.0, 111.0, 316.0, 133.0]),
                vec![],
            ),
            Laid(
                5,
                Button,
                Some("title"),
                Some([313.0, 287.0, 540.0, 332.0]),
                vec![],
            ),
            Laid(
                6,
                Button,
                Some("books"),
                Some([249.0, 60.0, 399.0, 96.0]),
                vec![],
            ),
            Laid(7, Button, Some("Lanterns"), None, vec![]),
            Laid(
                8,
                Button,
                Some("Lanterns"),
                Some([313.0, 406.0, 540.0, 443.0]),
                vec![],
            ),
            Laid(
                9,
                Unknown,
                None,
                Some([261.0, 248.0, 421.0, 454.0]),
                vec![10],
            ),
            Laid(10, GenericContainer, None, None, vec![11]),
            Laid(
                11,
                Button,
                Some("title"),
                Some([266.0, 282.0, 416.0, 310.0]),
                vec![],
            ),
        ])
    }

    fn rect(x0: f32, y0: f32, x1: f32, y1: f32) -> egui::Rect {
        egui::Rect::from_min_max(egui::pos2(x0, y0), egui::pos2(x1, y1))
    }

    #[test]
    fn open_takes_the_tree_s_item_of_the_names_on_screen() {
        let window = window();
        assert_eq!(candidates(&window, "books").len(), 3);
        assert_eq!(
            choose(tree_items(&window, "books")),
            Pick::One(rect(0.0, 204.0, 248.0, 228.0))
        );
        assert_eq!(choose(tree_items(&window, "title")), Pick::Nothing);
    }

    #[test]
    fn a_click_takes_the_only_node_with_bounds() {
        assert_eq!(
            choose(candidates(&window(), "Lanterns")),
            Pick::One(rect(313.0, 406.0, 540.0, 443.0))
        );
        assert_eq!(choose(candidates(&window(), "Raw WHERE")), Pick::Nothing);
    }

    #[test]
    fn of_several_a_click_takes_the_one_over_the_window() {
        let found = candidates(&window(), "title");
        assert_eq!(found.iter().filter(|one| one.over).count(), 1);
        assert_eq!(choose(found), Pick::One(rect(266.0, 282.0, 416.0, 310.0)));
    }

    #[test]
    fn of_several_in_the_window_a_click_takes_none() {
        let Pick::Ambiguous(found) = choose(candidates(&window(), "books")) else {
            panic!("three nodes are called books");
        };
        assert_eq!(found.len(), 3);
        assert!(found.iter().all(|one| !one.over));
    }
}
