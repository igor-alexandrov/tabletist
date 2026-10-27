//! Settings' controls to a screen reader, in both looks: a segmented
//! control is a radio group named by its option, and every control of a
//! row is described by what the row says of it: macOS's and Windows's
//! small print, the terminal look's hint.

use egui::accesskit::{Live, Node, NodeId, Role, Toggled, TreeUpdate};

use crate::keymap::Command;
use crate::model::{Action, SettingsPane};
use crate::settings::OptionId;
use crate::testing::Harness;
use crate::theme::Look;

use super::settings::{helper, label, terminal_label};

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

/// What `option` is called in `harness`'s look: the row's label, which
/// names its controls, with the keys it names written in.
fn name(harness: &Harness, option: OptionId) -> String {
    let look = &harness.app.look;
    let label = if look.terminal {
        terminal_label(option)
    } else {
        label(option)
    };
    super::settings::said(&harness.ctx, look, harness.app.locale, label)
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
fn controls<'a>(tree: &'a TreeUpdate, name: &str) -> Vec<&'a Node> {
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

/// macOS's and Windows's: the terminal look's rows are described by their
/// hints (the tests after this one).
#[test]
fn every_rows_controls_are_described_by_its_small_print() {
    {
        let look = Look::macos();
        let mut described = Vec::new();
        for shown in SettingsPane::listed().iter().copied() {
            let mut harness = pane(look, shown);
            let tree = harness.settle();
            for option in OptionId::ALL {
                // The pane's title has it, described by the words under
                // the title (the next test).
                if option == OptionId::AgentsOn {
                    continue;
                }
                let found = controls(&tree, &name(&harness, option));
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
        // Every row that has small print, or words beside its control in
        // its place.
        described.sort_by_key(|option| OptionId::ALL.iter().position(|all| all == option));
        let expected: Vec<OptionId> = OptionId::ALL
            .into_iter()
            .filter(|option| helper(*option).is_some())
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
            let name = name(&harness, option);
            let name = name.as_str();
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

/// A key of the screen, as the terminal look writes it.
fn key(harness: &Harness, command: Command) -> String {
    crate::ui::keys::written(&harness.ctx, &harness.app.look, command)
        .as_str()
        .to_owned()
}

/// Checks that every control of each of `shown`'s option rows, in the
/// terminal look, is described by the hint the row paints (`None`: it
/// paints none), and that `hints` names every option the pane shows.
fn described_by_hints(
    harness: &mut Harness,
    shown: SettingsPane,
    hints: &[(OptionId, Option<&str>)],
) {
    let tree = harness.settle();
    let mut seen = Vec::new();
    for option in OptionId::ALL {
        let found = controls(&tree, &name(harness, option));
        if found.is_empty() {
            continue;
        }
        let (_, said) = hints
            .iter()
            .find(|(listed, _)| *listed == option)
            .unwrap_or_else(|| panic!("{option:?} on {shown:?} says nothing in the test"));
        for node in found {
            assert_eq!(node.description(), *said, "{:?} on {shown:?}", node.label());
        }
        seen.push(option);
    }
    let listed: Vec<OptionId> = hints.iter().map(|(option, _)| *option).collect();
    for option in &listed {
        assert!(
            seen.contains(option),
            "no controls of {option:?} on {shown:?}"
        );
    }
}

#[test]
fn the_terminal_generals_rows_are_described_by_their_hints() {
    let mut harness = pane(Look::omarchy(), SettingsPane::General);
    let timestamp = super::settings::sample_timestamp(harness.app.settings.timestamps);
    described_by_hints(
        &mut harness,
        SettingsPane::General,
        &[
            (
                OptionId::OnLaunch,
                Some("or connections · a chosen connection"),
            ),
            (OptionId::RestoreTabs, Some("per connection")),
            (OptionId::PageSize, Some("table view")),
            (OptionId::Timestamps, Some(timestamp)),
            (OptionId::Tz, Some("sent with offset")),
            (OptionId::GroupDigits, Some("grouping off")),
            // Two tags, as the grid shows them.
            (OptionId::ValueTags, Some("print  ebook")),
            // The prompt's lines that set it.
            (
                OptionId::DatePicker,
                Some(":set calendar · :set nocalendar"),
            ),
            (
                OptionId::ProductionReadOnly,
                Some("default for new connections"),
            ),
            (OptionId::ConfirmCloseEdits, None),
            (OptionId::IdleDisconnect, Some("tabs reconnect on use")),
        ],
    );
}

#[test]
fn the_terminal_appearances_rows_are_described_by_their_hints() {
    let mut harness = pane(Look::omarchy(), SettingsPane::Appearance);
    let size = format!(
        "{} / {} anywhere",
        key(&harness, Command::LargerText),
        key(&harness, Command::SmallerText)
    );
    let list = format!("default; {} toggles", key(&harness, Command::ToggleTree));
    described_by_hints(
        &mut harness,
        SettingsPane::Appearance,
        &[
            (OptionId::Theme, Some("now: dark, from the system")),
            // The row has no control but its own: nothing is read from
            // fontconfig in a test.
            (OptionId::Font, Some("now: jetbrains mono, bundled")),
            (OptionId::FontSize, Some(&size)),
            (OptionId::RowHeight, Some("1 · 1.5 · 2 lines")),
            (OptionId::Zebra, None),
            (OptionId::ColumnTypes, Some("second header line")),
            (OptionId::TableList, Some(&list)),
            (OptionId::EnvBar, Some("PROD always tinted")),
        ],
    );
}

#[test]
fn the_terminal_editors_rows_are_described_by_their_hints() {
    let mut harness = pane(Look::omarchy(), SettingsPane::Editor);
    let tabs = format!(
        "{} / {} per tab",
        key(&harness, Command::ReadOnlyTab),
        key(&harness, Command::ReadWriteTab)
    );
    described_by_hints(
        &mut harness,
        SettingsPane::Editor,
        &[
            (OptionId::NewTab, Some(&tabs)),
            (
                OptionId::RowLimit,
                Some("caps results you browse · show all and exports get every row"),
            ),
            (OptionId::Timeout, None),
            // The terminal look's keys are vim's: a selection is visual
            // mode's.
            (OptionId::RunScope, Some("a visual selection always wins")),
            (
                OptionId::ConfirmUnsafeWrites,
                Some("connections that allow writes"),
            ),
            (
                OptionId::EditorKeys,
                Some("-- INSERT -- shown in the status line"),
            ),
            (OptionId::LineNumbers, None),
            (OptionId::Indent, None),
            (OptionId::Autocomplete, Some("schema objects")),
            (OptionId::ClosePairs, None),
            (OptionId::Wrap, None),
            (OptionId::Keywords, None),
            (OptionId::Commas, None),
            (
                OptionId::HistoryKeep,
                Some("per connection · sql, saves, filters, agents · never results"),
            ),
        ],
    );
}

#[test]
fn a_hint_left_out_for_want_of_room_still_describes_its_row() {
    let hint = "caps results you browse · show all and exports get every row";
    // As wide as the rows' columns and a little of the hint: the hint is
    // whole or left out, and here it is left out.
    let mut harness = Harness::with_size(egui::vec2(900.0, 1400.0));
    harness.set_look(Look::omarchy());
    harness.app.apply(Action::ShowSettings);
    harness
        .app
        .apply(Action::ShowSettingsPane(SettingsPane::Editor));
    let tree = harness.settle();
    assert!(
        !harness.painted.iter().any(|(text, _)| text.contains(hint)),
        "the hint is painted"
    );
    let found = controls(&tree, &name(&harness, OptionId::RowLimit));
    assert!(!found.is_empty(), "no controls of the row limit");
    for node in found {
        assert_eq!(node.description(), Some(hint), "{:?}", node.label());
    }
}

#[test]
fn the_terminal_agents_rows_are_described_by_their_hints() {
    if !SettingsPane::listed().contains(&SettingsPane::Agents) {
        return;
    }
    let mut harness = pane(Look::omarchy(), SettingsPane::Agents);
    described_by_hints(
        &mut harness,
        SettingsPane::Agents,
        &[
            (OptionId::AgentsOn, Some("while tabletist is open")),
            // No connection window is open: nothing to show the log in.
            (OptionId::LogDays, Some("on this computer")),
            (OptionId::AgentsNotify, Some("a desktop notification")),
        ],
    );
    // The rows that are no option's: their buttons, and the line for other
    // agents, which the cursor passes, its hint said as its value is,
    // whether the command beside it leaves the room to paint it or not.
    let access = format!("{} on a row", key(&harness, Command::ChangeAgentAccess));
    let tree = harness.settle();
    for (row, said) in [
        ("Status", "agents can't connect"),
        ("Set for each connection", access.as_str()),
    ] {
        let id = crate::testing::node(&tree, row, Role::Button)
            .unwrap_or_else(|| panic!("no row {row}"));
        let (_, node) = tree.nodes.iter().find(|(node, _)| *node == id).expect(row);
        assert_eq!(node.description(), Some(said), "{row}");
    }
    assert!(crate::testing::node(&tree, "same server, stdio", Role::Label).is_some());
}

/// Whether `said` is in the tree as a polite live region: read out as it
/// appears.
pub(super) fn heard(tree: &TreeUpdate, said: &str) -> bool {
    tree.nodes
        .iter()
        .filter(|(_, node)| node.live() == Some(Live::Polite))
        .any(|(_, node)| node.label().or_else(|| node.value()) == Some(said))
}
