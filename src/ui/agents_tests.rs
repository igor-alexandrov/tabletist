//! The Agents tab and its entry in the sidebar, in both looks: what they
//! say of the connection, the ways in, and that nothing the server says
//! opens the tab or brings it forward.

use std::time::SystemTime;

use crate::backend::{AgentsEvent, Event};
use crate::connections::AgentAccess;
use crate::model::{Action, AgentsHere, ConnTabId};
use crate::testing::Harness;
use crate::theme::Look;

fn looks() -> [Look; 2] {
    [Look::macos(), Look::omarchy()]
}

fn connected(look: Look) -> (Harness, ConnTabId) {
    let mut harness = Harness::new();
    harness.set_look(look);
    let tab = harness.connect_fake();
    harness.settle();
    (harness, tab)
}

/// Sets what agents may do on `tab`'s saved connection, as the person
/// who keeps it: a change of the files that is not held.
fn allow(harness: &mut Harness, tab: ConnTabId, access: AgentAccess) {
    let conn = harness.app.workspace(tab).unwrap().conn_id.clone();
    let mut saved = harness.app.connections.get(&conn).unwrap().clone();
    saved.agents.access = access;
    harness.app.connections.upsert(saved);
    harness.app.check_agents();
    let held = harness.app.agents_held.clone();
    harness.app.apply(Action::KeepAgentChanges(held));
}

/// A session of `agent` whose calls have named `tab`'s connection, or
/// none.
fn session(harness: &Harness, tab: Option<ConnTabId>, agent: &str) -> tabletist_mcp::SessionInfo {
    let connections = tab
        .map(|tab| vec![harness.app.workspace(tab).unwrap().conn_id.0.clone()])
        .unwrap_or_default();
    tabletist_mcp::SessionInfo {
        id: format!("session of {agent}"),
        agent: Some(agent.to_owned()),
        project: Some("tabletist-repo".into()),
        transport: tabletist_mcp::Transport::Stdio,
        connected_at: SystemTime::now(),
        calls: 14,
        connections,
        last_call_at: Some(SystemTime::now()),
    }
}

fn said(harness: &mut Harness, event: AgentsEvent) {
    harness.app.apply(Action::Backend(Event::Agents(event)));
    harness.settle();
}

fn front(harness: &Harness, tab: ConnTabId) -> bool {
    harness.app.workspace(tab).unwrap().agents_in_front()
}

fn agents_tabs(harness: &Harness, tab: ConnTabId) -> usize {
    let workspace = harness.app.workspace(tab).unwrap();
    workspace
        .tabs
        .iter()
        .filter(|open| matches!(open, crate::model::Tab::Agents(_)))
        .count()
}

/// What the entry is called on screen, in `look`'s case.
fn entry(look: &Look, state: &str) -> String {
    look.label(&format!("Agents, {state}"))
}

#[test]
fn the_entry_says_the_server_then_the_access_then_who_is_here() {
    for look in looks() {
        let (mut harness, tab) = connected(look);
        // Off in the settings, whatever the connection allows.
        allow(&mut harness, tab, AgentAccess::Read);
        harness.settle();
        assert!(harness.has(&entry(&look, "Server off")), "{}", look.name);
        harness.app.settings.mcp_enabled = true;
        allow(&mut harness, tab, AgentAccess::Off);
        harness.settle();
        assert!(harness.has(&entry(&look, "Off here")), "{}", look.name);
        allow(&mut harness, tab, AgentAccess::Read);
        harness.settle();
        assert!(
            harness.has(&entry(&look, "None connected")),
            "{}",
            look.name
        );
        // An agent that has named no connection is nowhere yet.
        let nowhere = session(&harness, None, "Cursor");
        said(&mut harness, AgentsEvent::Sessions(vec![nowhere.clone()]));
        assert!(
            harness.has(&entry(&look, "None connected")),
            "{}",
            look.name
        );
        let here = session(&harness, Some(tab), "Claude Code");
        said(&mut harness, AgentsEvent::Sessions(vec![nowhere, here]));
        assert!(harness.has(&entry(&look, "Claude Code")), "{}", look.name);
        // A server that stops has nobody on it.
        said(&mut harness, AgentsEvent::Stopped);
        assert!(
            harness.has(&entry(&look, "None connected")),
            "{}",
            look.name
        );
    }
}

#[test]
fn with_several_agents_here_the_one_that_called_last_is_named() {
    let (mut harness, tab) = connected(Look::macos());
    harness.app.settings.mcp_enabled = true;
    allow(&mut harness, tab, AgentAccess::Read);
    let mut earlier = session(&harness, Some(tab), "Cursor");
    earlier.last_call_at = Some(SystemTime::UNIX_EPOCH);
    let later = session(&harness, Some(tab), "Claude Code");
    said(&mut harness, AgentsEvent::Sessions(vec![later, earlier]));
    assert_eq!(
        harness.app.agents_here(tab),
        Some(AgentsHere::Connected {
            agent: "Claude Code".into(),
            count: 2
        })
    );
}

#[test]
fn every_way_in_shows_the_one_tab_and_none_opens_it_by_itself() {
    let (mut harness, tab) = connected(Look::macos());
    harness.app.settings.mcp_enabled = true;
    allow(&mut harness, tab, AgentAccess::Read);
    // Nothing the server says opens the tab.
    said(&mut harness, AgentsEvent::Started { kept_aside: None });
    let here = session(&harness, Some(tab), "Claude Code");
    said(&mut harness, AgentsEvent::Sessions(vec![here]));
    assert_eq!(agents_tabs(&harness, tab), 0);
    // The entry.
    harness.click("Agents, Claude Code");
    harness.settle();
    assert!(front(&harness, tab));
    // Another tab in front: what the server says leaves it there.
    let query = harness.add_sql_tab(tab);
    harness.app.apply(Action::ActivateTab { tab, id: query });
    said(&mut harness, AgentsEvent::Sessions(Vec::new()));
    said(&mut harness, AgentsEvent::Stopped);
    assert!(!front(&harness, tab));
    // The key: the same tab.
    harness.press(
        egui::Key::A,
        egui::Modifiers::COMMAND | egui::Modifiers::ALT,
    );
    harness.settle();
    assert!(front(&harness, tab));
    assert_eq!(agents_tabs(&harness, tab), 1);
    // Quick open, from the words that ask for it.
    harness.app.apply(Action::ActivateTab { tab, id: query });
    harness.app.apply(Action::OpenQuickOpen);
    if let Some(crate::model::Dialog::QuickOpen(open)) = &mut harness.app.dialog {
        open.query = "age".into();
    }
    harness.settle();
    assert_eq!(
        harness.app.quick_open_found(tab, "age").first(),
        Some(&crate::app::Found::Agents)
    );
    harness.app.apply(Action::QuickOpenPick);
    assert!(front(&harness, tab));
    assert_eq!(agents_tabs(&harness, tab), 1);
    // One letter, or letters only somewhere in the word, are a table's.
    for typed in ["a", "s", "gen", "act"] {
        assert_ne!(
            harness.app.quick_open_found(tab, typed).first(),
            Some(&crate::app::Found::Agents),
            "{typed}"
        );
    }
    // And with nothing typed, quick open is the objects'.
    assert!(
        !harness
            .app
            .quick_open_found(tab, "")
            .contains(&crate::app::Found::Agents)
    );
}

#[test]
fn omarchy_shows_the_tab_with_ga_and_with_the_agents_line() {
    let (mut harness, tab) = connected(Look::omarchy());
    // From the grid.
    super::tests::type_key(&mut harness, egui::Key::G, "g");
    super::tests::type_key(&mut harness, egui::Key::A, "a");
    assert!(front(&harness, tab));
    assert_eq!(agents_tabs(&harness, tab), 1);
    // From another tab, by the prompt's line.
    let query = harness.add_sql_tab(tab);
    harness.app.apply(Action::ActivateTab { tab, id: query });
    harness.app.workspace_mut(tab).unwrap().command = Some("agents".into());
    harness.app.apply(Action::RunCommand(tab));
    assert!(front(&harness, tab));
    assert_eq!(agents_tabs(&harness, tab), 1);
}

#[test]
fn the_tab_shows_its_state_and_its_buttons_do_what_they_say() {
    for look in looks() {
        let (mut harness, tab) = connected(look);
        allow(&mut harness, tab, AgentAccess::Read);
        harness.app.apply(Action::ShowAgents(tab));
        harness.settle();
        // The server off: Turn on, which changes no connection's access.
        assert!(harness.has("The agent server is off"), "{}", look.name);
        harness.click("Turn on");
        harness.settle();
        assert!(harness.app.settings.mcp_enabled, "{}", look.name);
        assert_eq!(
            harness.app.agents_here(tab),
            Some(AgentsHere::NoneConnected)
        );
        // Nobody here: the command, and Copy.
        let command = crate::endpoint::agent_command();
        assert!(harness.has(command), "{}", look.name);
        harness.click("Copy");
        harness.settle();
        assert_eq!(
            crate::ui::toast::said(&harness.ctx).as_deref(),
            Some("Copied the command"),
            "{}",
            look.name
        );
        // Off here: no command to copy.
        allow(&mut harness, tab, AgentAccess::Off);
        harness.settle();
        assert!(!harness.has(command), "{}", look.name);
        // Who is here.
        allow(&mut harness, tab, AgentAccess::Read);
        let here = session(&harness, Some(tab), "Claude Code");
        said(&mut harness, AgentsEvent::Sessions(vec![here]));
        let name = if look.terminal {
            "claude code"
        } else {
            "Claude Code"
        };
        assert!(
            harness.has(&format!(
                "{name} · tabletist-repo · connected just now · 14 calls · Runs as dev"
            )),
            "{}",
            look.name
        );
    }
}

#[test]
fn omarchys_letters_copy_and_turn_on_where_the_tab_offers_them() {
    // Worked out, as the backend's start does off the UI thread.
    crate::endpoint::agent_command();
    let (mut harness, tab) = connected(Look::omarchy());
    allow(&mut harness, tab, AgentAccess::Read);
    harness.app.apply(Action::ShowAgents(tab));
    harness.settle();
    // `y` copies nothing while the server is off.
    super::tests::type_key(&mut harness, egui::Key::Y, "y");
    assert_eq!(crate::ui::toast::said(&harness.ctx), None);
    super::tests::type_key(&mut harness, egui::Key::O, "o");
    assert!(harness.app.settings.mcp_enabled);
    // `o` once more is nothing: the server is on.
    let before = harness.app.settings.clone();
    super::tests::type_key(&mut harness, egui::Key::O, "o");
    assert_eq!(harness.app.settings, before);
    super::tests::type_key(&mut harness, egui::Key::Y, "y");
    assert_eq!(
        crate::ui::toast::said(&harness.ctx).as_deref(),
        Some("Copied the command")
    );
    // This look floats no toast: its status line says it.
    assert!(harness.has("copied the command"));
}

#[test]
fn the_tab_names_the_connection_as_an_agent_does_and_counts_only_who_is_here() {
    let (mut harness, tab) = connected(Look::macos());
    harness.app.settings.mcp_enabled = true;
    allow(&mut harness, tab, AgentAccess::Read);
    // A connection with no environment is its name alone.
    let conn = harness.app.workspace(tab).unwrap().conn_id.clone();
    let mut saved = harness.app.connections.get(&conn).unwrap().clone();
    saved.environment = crate::env::Environment::None;
    harness.app.connections.upsert(saved);
    harness.app.apply(Action::ShowAgents(tab));
    harness.settle();
    assert!(harness.has("No agents connected to Fixture"));
    let here = session(&harness, Some(tab), "Claude Code");
    said(&mut harness, AgentsEvent::Sessions(vec![here]));
    assert!(harness.has("Claude Code · tabletist-repo · connected just now · 14 calls"));
    assert!(harness.has("1 agent here · Server: stdio"));
    // Its access Off, the agent that worked here is not here.
    allow(&mut harness, tab, AgentAccess::Off);
    harness.settle();
    assert!(harness.has("No agents here · Server: stdio"));
}

#[test]
fn a_start_that_failed_leaves_nobody_here() {
    let (mut harness, tab) = connected(Look::macos());
    harness.app.settings.mcp_enabled = true;
    allow(&mut harness, tab, AgentAccess::Read);
    let here = session(&harness, Some(tab), "Claude Code");
    said(&mut harness, AgentsEvent::Sessions(vec![here]));
    assert!(harness.has("Agents, Claude Code"));
    // A start while a server ran stops it first, then fails to listen.
    said(
        &mut harness,
        AgentsEvent::Failed(crate::endpoint::ANOTHER_SERVES.into()),
    );
    assert!(harness.app.agents.sessions.is_empty());
    assert!(harness.has("Agents, None connected"));
}

#[test]
fn what_the_server_says_while_it_is_off_is_not_kept() {
    let (mut harness, tab) = connected(Look::macos());
    allow(&mut harness, tab, AgentAccess::Read);
    let here = session(&harness, Some(tab), "Claude Code");
    said(&mut harness, AgentsEvent::Sessions(vec![here]));
    assert!(harness.app.agents.sessions.is_empty());
}

#[test]
fn more_sessions_than_the_tab_has_room_for_scroll_to_the_last() {
    let (mut harness, tab) = connected(Look::macos());
    harness.app.settings.mcp_enabled = true;
    allow(&mut harness, tab, AgentAccess::Read);
    let many: Vec<_> = (0..40)
        .map(|n| session(&harness, Some(tab), &format!("Agent {n}")))
        .collect();
    said(&mut harness, AgentsEvent::Sessions(many));
    harness.app.apply(Action::ShowAgents(tab));
    harness.settle();
    let status = harness
        .painted_rect("40 agents here · Server: stdio")
        .unwrap();
    // The last row, the agent that called first, is under the status bar,
    // out of sight, until scrolled.
    let last = "Agent 0 · tabletist-repo · connected just now · 14 calls · Runs as dev";
    let seen = |harness: &Harness| {
        harness
            .painted_rect("Agent 0  tabletist-repo · connected just now · 14 calls · Runs as dev")
            .is_some_and(|row| row.bottom() <= status.top())
    };
    assert!(!seen(&harness));
    assert!(harness.has(last));
    let middle = harness.size / 2.0;
    for _ in 0..20 {
        harness.frame(vec![
            egui::Event::PointerMoved(middle.to_pos2()),
            egui::Event::MouseWheel {
                unit: egui::MouseWheelUnit::Point,
                delta: egui::vec2(0.0, -400.0),
                modifiers: egui::Modifiers::NONE,
                phase: egui::TouchPhase::Move,
            },
        ]);
    }
    harness.settle();
    assert!(seen(&harness));
}

#[test]
fn closing_the_tab_stops_nothing() {
    let (mut harness, tab) = connected(Look::macos());
    harness.app.settings.mcp_enabled = true;
    allow(&mut harness, tab, AgentAccess::Read);
    let here = session(&harness, Some(tab), "Claude Code");
    said(&mut harness, AgentsEvent::Sessions(vec![here]));
    harness.app.apply(Action::ShowAgents(tab));
    assert!(front(&harness, tab));
    let id = harness.app.workspace(tab).unwrap().agents_tab().unwrap();
    let sent = harness.app.backend.sent.len();
    harness.app.apply(Action::CloseTab { tab, id });
    assert_eq!(agents_tabs(&harness, tab), 0);
    assert_eq!(harness.app.agents.sessions.len(), 1);
    assert_eq!(harness.app.backend.sent.len(), sent, "nothing is sent");
}
