//! Settings' controls to a screen reader, in both looks: a segmented
//! control is a radio group named by its option, and every control of a
//! row is described by the row's small print.

use egui::accesskit::{Node, NodeId, Role, Toggled, TreeUpdate};

use crate::model::{Action, SettingsPane};
use crate::settings::OptionId;
use crate::testing::Harness;
use crate::theme::Look;

use super::settings::{helper, label, small_print, terminal_label};

fn looks() -> [Look; 2] {
    [Look::macos(), Look::omarchy()]
}

/// Settings open on `pane`, in `look`, in a window tall enough for every
/// row.
fn pane(look: Look, pane: SettingsPane) -> Harness {
    let mut harness = Harness::with_size(egui::vec2(1920.0, 2400.0));
    harness.set_look(look);
    harness.app.apply(Action::ShowSettings);
    harness.app.apply(Action::ShowSettingsPane(pane));
    harness.settle();
    harness
}

/// What `option` is called in `look`: the row's label, which names its
/// controls.
fn name(look: &Look, option: OptionId) -> &'static str {
    if look.terminal {
        terminal_label(option)
    } else {
        label(option)
    }
}

/// The option whose row has `option`'s control: macOS's and Windows's
/// data font row has the size's menu.
fn row(look: &Look, option: OptionId) -> OptionId {
    if option == OptionId::FontSize && !look.terminal {
        OptionId::Font
    } else {
        option
    }
}

/// Whether `role` is one a person sets something with.
fn control(role: Role) -> bool {
    matches!(
        role,
        Role::ComboBox | Role::CheckBox | Role::RadioButton | Role::Button
    )
}

/// The controls `option`'s row has: named by the option alone (a menu, a
/// switch, the terminal look's row) or by the option and a value of it.
fn controls<'a>(tree: &'a TreeUpdate, look: &Look, option: OptionId) -> Vec<&'a Node> {
    let name = name(look, option);
    let valued = format!("{name}: ");
    tree.nodes
        .iter()
        .map(|(_, node)| node)
        .filter(|node| control(node.role()))
        .filter(|node| {
            node.label()
                .is_some_and(|label| label == name || label.starts_with(&valued))
        })
        .collect()
}

#[test]
fn every_rows_controls_are_described_by_its_small_print() {
    for look in looks() {
        let mut described = Vec::new();
        for shown in SettingsPane::listed().iter().copied() {
            let mut harness = pane(look, shown);
            let tree = harness.settle();
            for option in OptionId::ALL {
                // The pane's title has it, described by the words under
                // the title (the next test).
                if option == OptionId::AgentsOn && !look.terminal {
                    continue;
                }
                let found = controls(&tree, &look, option);
                if found.is_empty() {
                    continue;
                }
                let said = helper(row(&look, option));
                for node in found {
                    assert_eq!(
                        node.description(),
                        said,
                        "{:?} in {} on {shown:?}",
                        node.label(),
                        look.name
                    );
                }
                let row = row(&look, option);
                if said.is_some() && !described.contains(&row) {
                    described.push(row);
                }
            }
        }
        // Every row that has small print, as each look lists its rows:
        // the terminal look has no interface size, and the Editor pane has
        // no row yet for some of its options.
        described.sort_by_key(|option| OptionId::ALL.iter().position(|all| all == option));
        let expected: Vec<OptionId> = OptionId::ALL
            .into_iter()
            .filter(|option| small_print(*option).is_some())
            .filter(|option| !(look.terminal && *option == OptionId::InterfaceSize))
            .filter(|option| !OptionId::EDITOR_LATER.contains(option))
            .collect();
        assert_eq!(described, expected, "in {}", look.name);
    }
}

#[test]
fn options_of_a_few_values_are_radio_groups_named_by_the_option() {
    let segmented = [
        (SettingsPane::General, OptionId::Timestamps),
        (SettingsPane::General, OptionId::GroupDigits),
        (SettingsPane::Appearance, OptionId::InterfaceSize),
        (SettingsPane::Appearance, OptionId::RowHeight),
        (SettingsPane::Appearance, OptionId::TableList),
        (SettingsPane::Appearance, OptionId::EnvBar),
    ];
    for look in looks() {
        // The terminal look's switches are its two values too.
        let check = look
            .terminal
            .then_some((SettingsPane::General, OptionId::ValueTags));
        for (shown, option) in segmented.into_iter().chain(check) {
            if look.terminal && option == OptionId::InterfaceSize {
                continue;
            }
            let mut harness = pane(look, shown);
            let tree = harness.settle();
            let name = name(&look, option);
            let buttons = radio_buttons(&tree, name)
                .unwrap_or_else(|| panic!("no radio group {name} in {}", look.name));
            assert!(buttons.len() >= 2, "{name} in {}", look.name);
            let chosen = buttons
                .iter()
                .filter(|node| node.toggled() == Some(Toggled::True))
                .count();
            assert_eq!(chosen, 1, "{name} in {}", look.name);
            let valued = format!("{name}: ");
            assert!(
                buttons
                    .iter()
                    .all(|node| node.label().is_some_and(|label| label.starts_with(&valued))),
                "{name} in {}",
                look.name
            );
        }
    }
}

/// The radio buttons in the radio group named `name`, where there is one.
fn radio_buttons<'a>(tree: &'a TreeUpdate, name: &str) -> Option<Vec<&'a Node>> {
    let (_, group) = tree
        .nodes
        .iter()
        .find(|(_, node)| node.role() == Role::RadioGroup && node.label() == Some(name))?;
    let children: &[NodeId] = group.children();
    Some(
        tree.nodes
            .iter()
            .filter(|(id, node)| children.contains(id) && node.role() == Role::RadioButton)
            .map(|(_, node)| node)
            .collect(),
    )
}

#[test]
fn the_agents_panes_own_controls_are_described_by_what_is_written_beside_them() {
    if !SettingsPane::listed().contains(&SettingsPane::Agents) {
        return;
    }
    let mut harness = pane(Look::macos(), SettingsPane::Agents);
    let tree = harness.settle();
    let buttons = radio_buttons(&tree, "Connect an agent").expect("the agent's radio group");
    assert_eq!(buttons.len(), 3);
    for node in buttons {
        assert_eq!(
            node.description(),
            Some("Run this once in your project:"),
            "{:?}",
            node.label()
        );
    }
    let switch = tree
        .nodes
        .iter()
        .map(|(_, node)| node)
        .find(|node| {
            node.role() == Role::CheckBox && node.label() == Some(label(OptionId::AgentsOn))
        })
        .expect("the server's switch");
    assert!(
        switch
            .description()
            .is_some_and(|said| said.starts_with("Let coding agents query your databases")),
        "{:?}",
        switch.description()
    );
}

#[test]
fn a_missing_data_fonts_warning_describes_both_its_menus() {
    let mut harness = Harness::with_size(egui::vec2(1920.0, 2400.0));
    harness.set_look(Look::macos());
    harness.app.change_settings(|settings| {
        settings.font = Some("Fira Code".into());
    });
    harness.app.apply(Action::ShowSettings);
    harness
        .app
        .apply(Action::ShowSettingsPane(SettingsPane::Appearance));
    let tree = harness.settle();
    let warning = "“Fira Code” is not installed: IBM Plex Mono is used";
    for menu in [label(OptionId::Font), label(OptionId::FontSize)] {
        let node = tree
            .nodes
            .iter()
            .map(|(_, node)| node)
            .find(|node| node.role() == Role::ComboBox && node.label() == Some(menu))
            .unwrap_or_else(|| panic!("no menu {menu}"));
        assert_eq!(node.description(), Some(warning), "{menu}");
    }
}
