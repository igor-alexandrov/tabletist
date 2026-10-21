//! Settings › Agents, in both looks: the pane switch, the server's switch
//! and the log's options, how each agent connects, and the ways from it to
//! the Agents tab and Connections, and back from the tab.

use crate::connections::AgentAccess;
use crate::model::{Action, AgentsView, ConnTabId, Dialog, SettingsPane, Tab};
use crate::testing::Harness;
use crate::theme::Look;

fn looks() -> [Look; 2] {
    [Look::macos(), Look::omarchy()]
}

/// A window connected to the fake connection, in `look`, with agents
/// allowed to read it.
fn connected(look: Look) -> (Harness, ConnTabId) {
    let mut harness = Harness::new();
    harness.set_look(look);
    let tab = harness.connect_fake();
    let conn = harness.app.workspace(tab).unwrap().conn_id.clone();
    let mut saved = harness.app.connections.get(&conn).unwrap().clone();
    saved.agents.access = AgentAccess::Read;
    harness.app.connections.upsert(saved);
    harness.app.check_agents();
    let held = harness.app.agents_held.clone();
    harness.app.apply(Action::KeepAgentChanges(held));
    harness.settle();
    (harness, tab)
}

/// The pane Settings shows, while it is open.
fn pane(harness: &Harness) -> Option<SettingsPane> {
    match &harness.app.dialog {
        Some(Dialog::Settings(dialog)) => Some(dialog.pane),
        _ => None,
    }
}

/// The view of `tab`'s Agents tab, while it is in front.
fn agents_view(harness: &Harness, tab: ConnTabId) -> Option<AgentsView> {
    let workspace = harness.app.workspace(tab)?;
    if !workspace.agents_in_front() {
        return None;
    }
    workspace.tabs.iter().find_map(|open| match open {
        Tab::Agents(agents) => Some(agents.view),
        Tab::Object(_) | Tab::Sql(_) | Tab::History(_) => None,
    })
}

/// Settings on its Agents pane, the cursor moved down `rows` rows in the
/// terminal look.
fn on_the_pane(harness: &mut Harness, rows: usize) {
    harness.app.apply(Action::ShowAgentSettings);
    harness.settle();
    for _ in 0..rows {
        super::tests::type_key(harness, egui::Key::J, "j");
    }
}

#[test]
fn settings_lists_an_agents_pane_after_appearance() {
    for look in looks() {
        let mut harness = Harness::new();
        harness.set_look(look);
        harness.app.apply(Action::ShowSettings);
        harness.settle();
        assert_eq!(pane(&harness), Some(SettingsPane::General), "{}", look.name);
        assert!(!harness.has("Let agents use Tabletist"), "{}", look.name);
        // Past Appearance, Editor and Keyboard, the panes between them.
        if look.terminal {
            for _ in 0..4 {
                harness.press(egui::Key::L, egui::Modifiers::CTRL);
            }
        } else {
            harness.click("Agents");
        }
        harness.settle();
        assert_eq!(pane(&harness), Some(SettingsPane::Agents), "{}", look.name);
        assert!(harness.has("Let agents use Tabletist"), "{}", look.name);
        assert!(!harness.has("Rows per page"), "{}", look.name);
        // And back.
        if look.terminal {
            for _ in 0..4 {
                harness.press(egui::Key::H, egui::Modifiers::CTRL);
            }
        } else {
            harness.click("General");
        }
        harness.settle();
        assert_eq!(pane(&harness), Some(SettingsPane::General), "{}", look.name);
        assert!(harness.has("Rows per page"), "{}", look.name);
    }
}

#[test]
fn the_agents_pane_turns_the_server_on_and_sets_the_log_and_the_notification() {
    for look in looks() {
        let mut harness = Harness::new();
        harness.set_look(look);
        on_the_pane(&mut harness, 0);
        assert!(!harness.app.settings.mcp_enabled);
        if look.terminal {
            harness.press(egui::Key::Space, egui::Modifiers::NONE);
        } else {
            harness.click("Let agents use Tabletist");
        }
        harness.settle();
        assert!(harness.app.settings.mcp_enabled, "{}", look.name);
        assert!(harness.app.agents_on(), "{}", look.name);
        // The log's days, from 30 to 90.
        if look.terminal {
            for _ in 0..3 {
                super::tests::type_key(&mut harness, egui::Key::J, "j");
            }
            super::tests::type_key(&mut harness, egui::Key::L, "l");
        } else {
            harness.click("Keep the activity log");
            harness.click("Keep the activity log 90 days");
        }
        harness.settle();
        assert_eq!(harness.app.settings.mcp_log_days, 90, "{}", look.name);
        // The notification, off.
        if look.terminal {
            super::tests::type_key(&mut harness, egui::Key::J, "j");
            harness.press(egui::Key::Space, egui::Modifiers::NONE);
        } else {
            harness.click("Notify when an agent connects");
        }
        harness.settle();
        assert!(!harness.app.settings.mcp_notify, "{}", look.name);
        // Off again: what Stop all agents was.
        if look.terminal {
            for _ in 0..4 {
                super::tests::type_key(&mut harness, egui::Key::K, "k");
            }
            harness.press(egui::Key::Space, egui::Modifiers::NONE);
        } else {
            harness.click("Let agents use Tabletist");
        }
        harness.settle();
        assert!(!harness.app.settings.mcp_enabled, "{}", look.name);
    }
}

#[test]
fn the_servers_switch_shows_what_is_in_force_and_keeps_a_change_held_from_outside() {
    let mut harness = Harness::new();
    // The file turned the server on, outside the app: held.
    harness.app.settings.mcp_enabled = true;
    harness.app.check_agents();
    assert!(!harness.app.agents_on());
    on_the_pane(&mut harness, 0);
    // The switch says off, as the server is.
    assert!(harness.has("Off"));
    harness.click("Let agents use Tabletist");
    harness.settle();
    // On is the person's own: what was held is kept, not written again.
    assert!(harness.app.agents_on());
    assert!(harness.app.agents_held.is_empty());
    // On once more changes nothing, and writes nothing.
    harness.app.backend.sent.clear();
    harness
        .app
        .apply(Action::SetOption(crate::settings::OptionValue::AgentsOn(
            true,
        )));
    assert!(
        harness.app.backend.sent.is_empty(),
        "{:?}",
        harness.app.backend.sent
    );
}

#[test]
fn reset_option_on_the_servers_switch_leaves_it_on() {
    let mut harness = Harness::new();
    harness.set_look(Look::omarchy());
    harness.app.apply(Action::TurnAgentsOn);
    on_the_pane(&mut harness, 0);
    harness.press(egui::Key::R, egui::Modifiers::SHIFT);
    assert!(harness.app.agents_on());
    // Nor does the footer offer it there.
    harness.click("Reset option");
    harness.settle();
    assert!(harness.app.agents_on());
    // On the log's row it puts the days back.
    harness
        .app
        .apply(Action::SetOption(crate::settings::OptionValue::LogDays(90)));
    for _ in 0..3 {
        super::tests::type_key(&mut harness, egui::Key::J, "j");
    }
    harness.press(egui::Key::R, egui::Modifiers::SHIFT);
    assert_eq!(harness.app.settings.mcp_log_days, 30);
}

#[test]
fn reset_on_the_agents_pane_puts_back_its_own_options() {
    let mut harness = Harness::new();
    harness.app.change_settings(|settings| {
        settings.page_size = 500;
        settings.mcp_log_days = 90;
        settings.mcp_notify = false;
    });
    harness.app.apply(Action::TurnAgentsOn);
    on_the_pane(&mut harness, 0);
    harness.app.apply(Action::ResetSettings);
    harness.app.apply(Action::ConfirmResetSettings(true));
    assert_eq!(harness.app.settings.mcp_log_days, 30);
    assert!(harness.app.settings.mcp_notify);
    // The server's switch is not a default to put back: off, it would end
    // every agent's session.
    assert!(harness.app.agents_on());
    // General's are General's.
    assert_eq!(harness.app.settings.page_size, 500);
    // A question asked on one pane is not answered on the other.
    harness.app.apply(Action::ResetSettings);
    harness
        .app
        .apply(Action::ShowSettingsPane(SettingsPane::General));
    harness.app.apply(Action::ConfirmResetSettings(true));
    assert_eq!(harness.app.settings.page_size, 500);
}

#[test]
fn the_agents_pane_says_how_each_agent_connects_and_copies_it() {
    // Worked out, as the backend's start does off the UI thread.
    let command = crate::endpoint::agent_command();
    let (mut harness, _) = connected(Look::macos());
    on_the_pane(&mut harness, 0);
    assert!(harness.has(command));
    harness.click("Copy");
    harness.settle();
    assert_eq!(harness.copied.as_deref(), Some(command));
    // Cursor's entry for its mcp.json.
    harness.click("Connect an agent: Cursor");
    harness.settle();
    let entry = crate::endpoint::agent_setup_ready(crate::endpoint::Agent::Cursor).unwrap();
    assert!(harness.has(entry));
    harness.click("Copy");
    harness.settle();
    assert_eq!(harness.copied.as_deref(), Some(entry));
    assert_eq!(
        crate::ui::toast::said(&harness.ctx).as_deref(),
        Some("Copied the entry for mcp.json")
    );
    // Any other agent: the command it starts.
    harness.click("Connect an agent: Other");
    harness.settle();
    let relay = crate::endpoint::agent_setup_ready(crate::endpoint::Agent::Other).unwrap();
    assert!(harness.has(relay));

    // The terminal look: `y` copies Claude Code's command.
    let (mut harness, _) = connected(Look::omarchy());
    on_the_pane(&mut harness, 0);
    super::tests::type_key(&mut harness, egui::Key::Y, "y");
    assert_eq!(harness.copied.as_deref(), Some(command));
    // On General it is nothing.
    harness
        .app
        .apply(Action::ShowSettingsPane(SettingsPane::General));
    harness.settle();
    harness.copied = None;
    super::tests::type_key(&mut harness, egui::Key::Y, "y");
    assert_eq!(harness.copied, None);
}

#[test]
fn settings_shows_the_agents_tab_and_its_log_of_the_window_in_front() {
    for look in looks() {
        let (mut harness, tab) = connected(look);
        // The status row, in the terminal look.
        on_the_pane(&mut harness, 1);
        if look.terminal {
            harness.press(egui::Key::Enter, egui::Modifiers::NONE);
        } else {
            harness.click("Show Agents tab");
        }
        harness.settle();
        assert!(harness.app.dialog.is_none(), "{}", look.name);
        assert_eq!(
            agents_view(&harness, tab),
            Some(AgentsView::Activity),
            "{}",
            look.name
        );
        // The log's row.
        on_the_pane(&mut harness, 3);
        if look.terminal {
            harness.press(egui::Key::Enter, egui::Modifiers::NONE);
        } else {
            harness.click("Show log");
        }
        harness.settle();
        assert!(harness.app.dialog.is_none(), "{}", look.name);
        assert_eq!(
            agents_view(&harness, tab),
            Some(AgentsView::Log),
            "{}",
            look.name
        );
    }
}

#[test]
fn with_no_window_open_settings_has_no_agents_tab_to_show() {
    for look in looks() {
        let mut harness = Harness::new();
        harness.set_look(look);
        on_the_pane(&mut harness, 1);
        if look.terminal {
            harness.press(egui::Key::Enter, egui::Modifiers::NONE);
        } else {
            // Disabled: a click is nothing.
            harness.click("Show Agents tab");
        }
        harness.settle();
        assert_eq!(pane(&harness), Some(SettingsPane::Agents), "{}", look.name);
        harness
            .app
            .apply(Action::ShowAgentsFromSettings(AgentsView::Log));
        assert_eq!(pane(&harness), Some(SettingsPane::Agents), "{}", look.name);
    }
}

#[test]
fn settings_opens_connections_where_access_is_set() {
    for look in looks() {
        let (mut harness, tab) = connected(look);
        // The access row, in the terminal look.
        on_the_pane(&mut harness, 2);
        if look.terminal {
            harness.press(egui::Key::Enter, egui::Modifiers::NONE);
        } else {
            harness.click("Open Connections");
        }
        harness.settle();
        assert!(harness.app.dialog.is_none(), "{}", look.name);
        let front = harness.app.active_tab_id();
        assert_ne!(front, tab, "{}", look.name);
        assert!(harness.app.workspace(front).is_none(), "{}", look.name);
    }
}

#[test]
fn the_agents_tab_opens_its_settings_where_the_server_is_off_and_where_nobody_is_here() {
    for look in looks() {
        let (mut harness, tab) = connected(look);
        harness.app.apply(Action::ShowAgents(tab));
        harness.settle();
        // The server off: Settings…
        if look.terminal {
            super::tests::type_key(&mut harness, egui::Key::S, "s");
        } else {
            harness.click("Settings…");
        }
        harness.settle();
        assert_eq!(pane(&harness), Some(SettingsPane::Agents), "{}", look.name);
        harness.app.apply(Action::CloseDialog);
        // Nobody here: Other agents…
        harness.app.apply(Action::TurnAgentsOn);
        harness.settle();
        if look.terminal {
            super::tests::type_key(&mut harness, egui::Key::S, "s");
        } else {
            harness.click("Other agents…");
        }
        harness.settle();
        assert_eq!(pane(&harness), Some(SettingsPane::Agents), "{}", look.name);
    }
}

#[test]
fn an_enter_held_on_the_status_row_shows_the_agents_tab_and_goes_no_further() {
    let (mut harness, tab) = connected(Look::omarchy());
    harness.app.settings.mcp_enabled = true;
    harness.app.serve_agents();
    // An agent works on another connection: the tab's Enter shows its
    // window.
    let mut ledger = crate::testing::fixture_connection();
    ledger.id = crate::connections::ConnectionId("ledger".into());
    ledger.name = "Ledger".into();
    harness.app.connections.upsert(ledger);
    harness
        .app
        .agents
        .sessions
        .push(tabletist_mcp::SessionInfo {
            id: "elsewhere".into(),
            agent: Some("Cursor".into()),
            project: None,
            transport: tabletist_mcp::Transport::Stdio,
            connected_at: std::time::SystemTime::UNIX_EPOCH,
            calls: 1,
            connections: vec!["ledger".into()],
            last_call_at: None,
        });
    on_the_pane(&mut harness, 1);
    // Enter held: its press, then what the window repeats of it.
    for _ in 0..4 {
        harness.frame(vec![crate::testing::key(
            egui::Key::Enter,
            egui::Modifiers::NONE,
        )]);
    }
    harness.frame(vec![crate::testing::release(
        egui::Key::Enter,
        egui::Modifiers::NONE,
    )]);
    harness.settle();
    assert_eq!(
        harness.app.active_tab_id(),
        tab,
        "it went on past the Agents tab"
    );
    assert_eq!(agents_view(&harness, tab), Some(AgentsView::Activity));
}

#[test]
fn the_log_row_says_enter_shows_the_log_only_where_there_is_a_window() {
    let shows = |harness: &Harness| {
        let hint = |text: &String| text.contains("shows it");
        harness.painted.iter().any(|(text, _)| hint(text))
    };
    // Wide enough for the rows' hints beside the file.
    let wide = || {
        let mut harness = Harness::with_size(egui::vec2(1440.0, 900.0));
        harness.set_look(Look::omarchy());
        harness
    };
    let mut harness = wide();
    on_the_pane(&mut harness, 3);
    assert!(!shows(&harness));
    assert!(
        harness
            .painted
            .iter()
            .any(|(text, _)| text == "on this computer")
    );
    let mut harness = wide();
    harness.connect_fake();
    on_the_pane(&mut harness, 3);
    assert!(shows(&harness));
}

#[test]
fn the_agents_panes_columns_stay_where_they_are_when_the_server_turns_on() {
    let mut harness = Harness::with_size(egui::vec2(1440.0, 900.0));
    harness.set_look(Look::omarchy());
    on_the_pane(&mut harness, 0);
    let column = |harness: &Harness| {
        let found = harness
            .text_rects
            .iter()
            .find(|(text, _)| text == "in connections");
        found.map(|(_, rect)| rect.left())
    };
    let off = column(&harness);
    assert!(off.is_some());
    harness.press(egui::Key::Space, egui::Modifiers::NONE);
    harness.settle();
    assert!(harness.app.agents_on());
    assert_eq!(column(&harness), off);
}

#[test]
fn the_settings_footer_keeps_close_in_the_narrowest_window() {
    let mut harness = Harness::with_size(egui::vec2(720.0, 480.0));
    harness.set_look(Look::omarchy());
    harness.connect_fake();
    // The status row: the most hints the Agents pane has.
    on_the_pane(&mut harness, 1);
    let close = harness
        .text_rects
        .iter()
        .find(|(text, _)| text.contains("close"))
        .map(|(_, rect)| *rect);
    assert!(close.is_some_and(|rect| rect.right() <= 720.0), "{close:?}");
}
