//! Connection groups in the connection form and the Connections list, in
//! both looks: Group offers the list's groups and New group…, a connection
//! given a group is listed under it and stays there after a relaunch, and
//! the rule's own group, chosen, leaves it to the rule.

use egui::accesskit::{self, Node, NodeId, Role, TreeUpdate};
use egui::{Key, Modifiers};

use crate::backend::{Command, StateFile};
use crate::connections::{ConnectionId, PasswordMode, SavedConnection, SavedConnections};
use crate::env::Environment;
use crate::model::Action;
use crate::paths::AppDirs;
use crate::testing::{Harness, bounds};
use crate::theme::Look;

fn looks() -> [Look; 2] {
    [Look::macos(), Look::omarchy()]
}

fn add(harness: &mut Harness, name: &str, group: Option<&str>, path: &str) -> ConnectionId {
    let saved = SavedConnection {
        id: ConnectionId::new(),
        name: name.into(),
        group: group.map(Into::into),
        environment: Environment::Local,
        read_only: None,
        password: PasswordMode::None,
        ssh_secret: PasswordMode::None,
        spec: tabletist_db::ConnectSpec::sqlite(path),
        agents: Default::default(),
    };
    let id = saved.id.clone();
    harness.app.connections.upsert(saved);
    id
}

/// The Bookshop's demo connections, on files: two that share a name, the
/// Playground, and the Rails test. Returns the Playground's and the Rails
/// test's ids.
fn bookshop(look: Look, playground: Option<&str>) -> (Harness, ConnectionId, ConnectionId) {
    let mut harness = Harness::new();
    harness.set_look(look);
    add(
        &mut harness,
        "Bookshop",
        None,
        "/srv/bookshop_development.db",
    );
    add(&mut harness, "Bookshop", None, "/srv/bookshop_staging.db");
    let play = add(&mut harness, "Playground", playground, "/srv/playground.db");
    let test = add(&mut harness, "Rails test", None, "/srv/bookshop_test.db");
    harness.settle();
    (harness, play, test)
}

/// Whether the list shows `row` under the heading `title`: below it, and
/// above the next heading of `titles`, the list's in order.
fn listed_under(harness: &mut Harness, titles: &[&str], title: &str, row: &str) -> bool {
    let tree = harness.settle();
    let top = |label: &str| bounds(&tree, label, Role::Button).map(|rect| rect.top());
    let at = titles.iter().position(|each| *each == title).unwrap();
    let (Some(heading), Some(row)) = (top(title), top(row)) else {
        return false;
    };
    let next = titles.get(at + 1).and_then(|next| top(next));
    heading < row && next.is_none_or(|next| row < next)
}

/// The control with `role` named `name` in the look's case: by its own
/// name, or by the label it is labelled by.
fn find_control<'t>(
    tree: &'t TreeUpdate,
    look: &Look,
    name: &str,
    role: Role,
) -> Option<(NodeId, &'t Node)> {
    let name = look.label(name);
    let label = tree
        .nodes
        .iter()
        .find(|(_, node)| {
            node.role() == Role::Label && node.label().or_else(|| node.value()) == Some(&name)
        })
        .map(|(id, _)| *id);
    tree.nodes
        .iter()
        .find(|(_, node)| {
            node.role() == role
                && (node.label() == Some(&name)
                    || (role == Role::Label && node.value() == Some(&name))
                    || label.is_some_and(|label| node.labelled_by().contains(&label)))
        })
        .map(|(id, node)| (*id, node))
}

/// [`find_control`], which is there.
fn control<'t>(tree: &'t TreeUpdate, look: &Look, name: &str, role: Role) -> (NodeId, &'t Node) {
    find_control(tree, look, name, role)
        .unwrap_or_else(|| panic!("no {name} ({role:?}): {:?}", crate::testing::labels(tree)))
}

/// Clicks the node `target`, as a screen reader does.
fn click(harness: &mut Harness, target: NodeId) {
    harness.frame(vec![egui::Event::AccessKitActionRequest(
        accesskit::ActionRequest {
            target_tree: accesskit::TreeId::ROOT,
            target_node: target,
            action: accesskit::Action::Click,
            data: None,
        },
    )]);
    harness.settle();
}

/// Opens the Group dropdown.
fn open_group(harness: &mut Harness) {
    let look = harness.app.look;
    let tree = harness.settle();
    let (id, _) = control(&tree, &look, "Group", Role::ComboBox);
    click(harness, id);
}

/// The item `name` of the Group dropdown's menu: a button in the menu
/// under it, as wide as it, not a row of the list behind the dialog.
fn menu_item(harness: &mut Harness, name: &str) -> Option<NodeId> {
    let look = harness.app.look;
    let tree = harness.settle();
    let dropdown = control(&tree, &look, "Group", Role::ComboBox)
        .1
        .bounds()
        .unwrap();
    tree.nodes
        .iter()
        .find(|(_, node)| {
            node.role() == Role::Button
                && node.label() == Some(name)
                && node.bounds().is_some_and(|item| {
                    item.y0 >= dropdown.y1 - 1.0
                        && item.x0 >= dropdown.x0 - 8.0
                        && item.x1 <= dropdown.x1 + 8.0
                })
        })
        .map(|(id, _)| *id)
}

fn choose(harness: &mut Harness, name: &str) {
    let item = menu_item(harness, name).unwrap_or_else(|| panic!("no item {name}"));
    click(harness, item);
}

/// The menu's last item: the form's own words, in the look's case, where
/// the groups are as their connections write them.
fn new_group(look: &Look) -> String {
    look.label("New group…")
}

/// What is typed in Group's field, while New group… has it open.
fn group_typed(harness: &mut Harness) -> Option<String> {
    let look = harness.app.look;
    let tree = harness.settle();
    let (_, field) = find_control(&tree, &look, "Group", Role::TextInput)?;
    Some(field.value().unwrap_or_default().to_owned())
}

/// The value the Group dropdown reads.
fn group_shown(harness: &mut Harness) -> Option<String> {
    let look = harness.app.look;
    let tree = harness.settle();
    let (_, node) = control(&tree, &look, "Group", Role::ComboBox);
    node.value().map(str::to_owned)
}

/// What the app asked the backend to write as `connections.toml` last.
fn last_saved(harness: &Harness) -> SavedConnections {
    harness
        .app
        .backend
        .sent
        .iter()
        .rev()
        .find_map(|command| match command {
            Command::Save {
                file: StateFile::Connections(store),
                ..
            } => Some(store.clone()),
            _ => None,
        })
        .expect("a save of the connections")
}

#[test]
fn a_connection_given_a_new_group_in_the_form_is_listed_under_it_after_a_relaunch() {
    for look in looks() {
        let (mut harness, play, _) = bookshop(look, None);
        let titles = ["Bookshop", "Local"];
        assert!(listed_under(&mut harness, &titles, "Local", "Playground"));
        harness.app.apply(Action::EditConnection(play.clone()));
        assert_eq!(group_shown(&mut harness).as_deref(), Some("Local"));
        open_group(&mut harness);
        choose(&mut harness, &new_group(&look));
        // The dropdown is a field now, with the keyboard.
        harness.frame(vec![egui::Event::Text("Experiments".into())]);
        assert_eq!(
            group_typed(&mut harness).as_deref(),
            Some("Experiments"),
            "{}",
            look.name
        );
        harness.app.apply(Action::SaveConnection { connect: false });
        assert!(harness.app.dialog.is_none(), "{}", look.name);
        let saved = harness.app.connections.get(&play).unwrap();
        assert_eq!(saved.group.as_deref(), Some("Experiments"));
        let titles = ["Bookshop", "Experiments", "Local"];
        assert!(
            listed_under(&mut harness, &titles, "Experiments", "Playground"),
            "{}",
            look.name
        );
        assert!(listed_under(&mut harness, &titles, "Local", "Rails test"));
        // The file the backend writes is read at the next start.
        let dir = tempfile::tempdir().unwrap();
        let dirs = AppDirs::at(dir.path());
        last_saved(&harness).save(&dirs.connections_file()).unwrap();
        let loaded = SavedConnections::load(&dirs);
        assert_eq!(loaded.problem, None);
        let mut relaunched = Harness::new();
        relaunched.set_look(look);
        relaunched.app.connections = loaded.store;
        assert!(
            listed_under(&mut relaunched, &titles, "Experiments", "Playground"),
            "{}: after a relaunch",
            look.name
        );
    }
}

#[test]
fn a_group_chosen_from_the_dropdown_is_kept_and_the_rules_own_clears_it() {
    for look in looks() {
        let (mut harness, _, test) = bookshop(look, Some("Experiments"));
        harness.app.apply(Action::EditConnection(test.clone()));
        open_group(&mut harness);
        // The list's groups, the Rails test's own among them, and New
        // group… under them. No row is remote: Remote is not offered.
        for offered in ["Bookshop", "Experiments", "Local", &new_group(&look)] {
            assert!(
                menu_item(&mut harness, offered).is_some(),
                "{}: {offered}",
                look.name
            );
        }
        assert!(menu_item(&mut harness, "Remote").is_none());
        choose(&mut harness, "Experiments");
        assert_eq!(group_shown(&mut harness).as_deref(), Some("Experiments"));
        harness.app.apply(Action::SaveConnection { connect: false });
        let saved = harness.app.connections.get(&test).unwrap();
        assert_eq!(saved.group.as_deref(), Some("Experiments"), "{}", look.name);
        // Local is the group the rule gives it: chosen, the connection has
        // none of its own again.
        harness.app.apply(Action::EditConnection(test.clone()));
        open_group(&mut harness);
        choose(&mut harness, "Local");
        assert_eq!(group_shown(&mut harness).as_deref(), Some("Local"));
        harness.app.apply(Action::SaveConnection { connect: false });
        assert_eq!(harness.app.connections.get(&test).unwrap().group, None);
    }
}

#[test]
fn escape_in_the_new_groups_field_goes_back_to_the_dropdown() {
    for look in looks() {
        let (mut harness, play, _) = bookshop(look, Some("Experiments"));
        harness.app.apply(Action::EditConnection(play.clone()));
        open_group(&mut harness);
        choose(&mut harness, &new_group(&look));
        harness.frame(vec![egui::Event::Text("Clients".into())]);
        harness.press(Key::Escape, Modifiers::NONE);
        assert!(
            harness.app.dialog.is_some(),
            "{}: the dialog stays",
            look.name
        );
        assert_eq!(group_typed(&mut harness), None, "{}", look.name);
        assert_eq!(group_shown(&mut harness).as_deref(), Some("Experiments"));
        // The keyboard is back on the dropdown.
        let focused = harness.ctx.memory(|memory| memory.focused());
        let tree = harness.settle();
        let (dropdown, _) = control(&tree, &look, "Group", Role::ComboBox);
        assert_eq!(
            focused.map(|id| id.accesskit_id()),
            Some(dropdown),
            "{}",
            look.name
        );
        // What was typed is gone: a save keeps the group as it was.
        harness.app.apply(Action::SaveConnection { connect: false });
        let saved = harness.app.connections.get(&play).unwrap();
        assert_eq!(saved.group.as_deref(), Some("Experiments"), "{}", look.name);
    }
}

/// macOS: Group beside Name, as the connection form's board draws it, the
/// name the wider. The terminal: a row of its own between name and
/// environment.
#[test]
fn group_sits_beside_the_name_on_macos_and_under_it_in_the_terminal() {
    for look in looks() {
        let mut harness = Harness::new();
        harness.set_look(look);
        harness.app.apply(Action::NewConnection);
        let tree = harness.settle();
        let name = control(&tree, &look, "Name", Role::TextInput)
            .1
            .bounds()
            .unwrap();
        let group = control(&tree, &look, "Group", Role::ComboBox)
            .1
            .bounds()
            .unwrap();
        if look.terminal {
            assert!(group.y0 >= name.y1, "under the name");
            // The environment's row, by its label: the buttons' group has
            // no bounds of its own.
            let environment = control(&tree, &look, "Environment", Role::Label)
                .1
                .bounds()
                .unwrap();
            assert!(environment.y0 >= group.y1, "above the environment");
        } else {
            assert!((group.y0 - name.y0).abs() < 1.0, "tops in line");
            assert!(group.x0 > name.x1, "beside it");
            let ratio = name.width() / group.width();
            assert!((1.3..1.5).contains(&ratio), "1.4 : 1, not {ratio}");
        }
    }
}
