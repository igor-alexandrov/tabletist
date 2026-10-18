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
        connected_at: at(),
        calls: 14,
        connections,
        last_call_at: Some(SystemTime::now()),
    }
}

/// When the tests' sessions connected and their calls came.
fn at() -> SystemTime {
    SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1_791_880_982)
}

/// "connected 10:43", as the clock says it here.
fn connected_at() -> String {
    format!("connected {}", crate::clock::hour(at()))
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
        // In the list at the left in the terminal look, its line over the
        // calls; in a row with Stop session in the macOS look.
        let said = if look.terminal {
            format!(
                "claude code  tabletist-repo · {} · 14 calls · runs as dev",
                connected_at()
            )
        } else {
            format!(
                "Claude Code · tabletist-repo · {} · 14 calls · Runs as dev",
                connected_at()
            )
        };
        assert!(harness.has(&said), "{}", look.name);
        assert!(
            harness.has(&look.label("No calls on this connection yet.")),
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
    assert!(harness.has(&format!(
        "Claude Code · tabletist-repo · {} · 14 calls",
        connected_at()
    )));
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
    let last = format!(
        "Agent 0 · tabletist-repo · {} · 14 calls · Runs as dev",
        connected_at()
    );
    let painted = format!("Agent 0  tabletist-repo · {} · 14 calls", connected_at());
    let seen = |harness: &Harness| {
        harness
            .painted_rect(&painted)
            .is_some_and(|row| row.bottom() <= status.top())
    };
    assert!(!seen(&harness));
    assert!(harness.has(&last));
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

/// What the files say, changed outside the app: the server on, and agents
/// may read `tab`'s connection.
#[cfg(any(unix, windows))]
fn changed_outside(harness: &mut Harness, tab: ConnTabId) {
    let conn = harness.app.workspace(tab).unwrap().conn_id.clone();
    let mut saved = harness.app.connections.get(&conn).unwrap().clone();
    saved.agents.access = AgentAccess::Read;
    harness.app.connections.upsert(saved);
    harness.app.settings.mcp_enabled = true;
    harness.app.check_agents();
    harness.settle();
}

#[cfg(any(unix, windows))]
#[test]
fn the_held_bar_says_what_changed_and_its_buttons_answer_it() {
    for look in looks() {
        let (mut harness, tab) = connected(look);
        harness.settle();
        let bar = harness.painted_rect("Fixture").expect("the connection bar");
        changed_outside(&mut harness, tab);
        let said = crate::ui::agents_held_said(&harness.app).unwrap();
        assert!(said.contains("agents may read Fixture · dev"), "{said}");
        assert!(harness.has(&said), "{}", look.name);
        // Under the connection bar, which stays at the window's top.
        assert_eq!(harness.painted_rect("Fixture"), Some(bar), "{}", look.name);
        let shown = harness.painted_rect(&said).expect("the bar's words");
        assert!(shown.top() >= bar.bottom(), "{}", look.name);
        // Held, the entry says what is in force.
        assert!(harness.has(&entry(&look, "Server off")), "{}", look.name);
        harness.click("Keep");
        harness.settle();
        assert!(harness.app.agents_held.is_empty(), "{}", look.name);
        assert!(!harness.has(&said), "{}", look.name);
        assert!(
            harness.has(&entry(&look, "None connected")),
            "{}",
            look.name
        );
        // Changed again, and put back.
        harness.app.settings.mcp_enabled = false;
        harness.app.check_agents();
        changed_outside(&mut harness, tab);
        assert!(!harness.app.agents_held.is_empty(), "{}", look.name);
        harness.click("Put back");
        harness.settle();
        assert!(harness.app.agents_held.is_empty(), "{}", look.name);
        assert!(!harness.app.settings.mcp_enabled, "{}", look.name);
        assert!(harness.has(&entry(&look, "Server off")), "{}", look.name);
    }
}

#[cfg(any(unix, windows))]
#[test]
fn while_a_change_is_held_the_tab_says_what_is_in_force() {
    use crate::agents_shown::Raised;
    let (mut harness, tab) = connected(Look::macos());
    changed_outside(&mut harness, tab);
    // A report of sessions while the server's turning on is held is not
    // taken: the server is off.
    let here = session(&harness, Some(tab), "Claude Code");
    said(&mut harness, AgentsEvent::Sessions(vec![here.clone()]));
    assert!(harness.app.agents.sessions.is_empty());
    // The server's turning on is kept, the connection's access is not.
    harness
        .app
        .apply(Action::KeepAgentChanges(vec![Raised::Server]));
    harness.app.apply(Action::ShowAgents(tab));
    harness.settle();
    assert!(harness.has(&entry(&Look::macos(), "Off here")));
    assert!(harness.has("Agents on Fixture · dev · Off"));
    assert!(!harness.has("Agents on Fixture · dev · Read"));
    said(&mut harness, AgentsEvent::Sessions(vec![here]));
    assert_eq!(harness.app.agents.sessions.len(), 1);
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

/// A saved connection elsewhere, Ledger · staging, and a session of
/// `agent` that works there and only there.
fn working_elsewhere(harness: &mut Harness, agent: &str) -> crate::connections::ConnectionId {
    let mut ledger = crate::testing::fixture_connection();
    ledger.id = crate::connections::ConnectionId("ledger".into());
    ledger.name = "Ledger".into();
    ledger.environment = crate::env::Environment::Staging;
    harness.app.connections.upsert(ledger.clone());
    let mut session = session(harness, None, agent);
    session.connections = vec!["ledger".into()];
    harness.app.agents.sessions.push(session);
    ledger.id
}

/// A call of the session of `agent`'s on `tab`'s connection, or on another.
fn call(harness: &Harness, tab: Option<ConnTabId>, sql: &str) -> tabletist_mcp::Call {
    tabletist_mcp::Call {
        at: at(),
        session: "session of Claude Code".into(),
        connection: Some(tab.map_or("ledger".into(), |tab| {
            harness.app.workspace(tab).unwrap().conn_id.0.clone()
        })),
        tool: "run_query".into(),
        request: Some(sql.into()),
        result: "ok".into(),
        message: None,
        rows: Some(1),
        columns: Some(1),
        took: std::time::Duration::from_millis(2),
    }
}

#[test]
fn the_feed_shows_the_calls_on_this_connection_as_they_end() {
    for look in looks() {
        let (mut harness, tab) = connected(look);
        harness.app.settings.mcp_enabled = true;
        allow(&mut harness, tab, AgentAccess::Read);
        let here = session(&harness, Some(tab), "Claude Code");
        said(&mut harness, AgentsEvent::Sessions(vec![here]));
        harness.app.apply(Action::ShowAgents(tab));
        let mine = call(&harness, Some(tab), "SELECT 1");
        said(&mut harness, AgentsEvent::Called(mine));
        let theirs = call(&harness, None, "SELECT 2");
        said(&mut harness, AgentsEvent::Called(theirs));
        let row = look.label(&crate::clock::time(at()));
        assert!(
            harness.has(&format!("{row} · run_query · SELECT 1 · 1 row · 2 ms")),
            "{}",
            look.name
        );
        assert!(
            !harness.has(&format!("{row} · run_query · SELECT 2 · 1 row · 2 ms")),
            "{}",
            look.name
        );
        // A refusal says why, as the log does.
        let mut refused = call(&harness, Some(tab), "DELETE FROM books");
        refused.result = "refused".into();
        refused.message = Some("Only SELECT is read here.".into());
        refused.rows = None;
        said(&mut harness, AgentsEvent::Called(refused));
        assert!(
            harness.has(&format!(
                "{row} · run_query · DELETE FROM books · refused · Only SELECT is read here."
            )),
            "{}",
            look.name
        );
    }
}

#[test]
fn stop_session_and_stop_agents_here_are_the_tabs_buttons() {
    let (mut harness, tab) = connected(Look::macos());
    harness.app.settings.mcp_enabled = true;
    allow(&mut harness, tab, AgentAccess::Read);
    harness.app.serve_agents();
    let here = session(&harness, Some(tab), "Claude Code");
    said(&mut harness, AgentsEvent::Sessions(vec![here]));
    harness.app.apply(Action::ShowAgents(tab));
    harness.settle();
    let stopped = |harness: &Harness| {
        harness
            .app
            .backend
            .sent
            .iter()
            .filter(|sent| {
                matches!(sent, crate::backend::Command::Agents(
                    crate::backend::AgentsCommand::StopSession(id)) if id == "session of Claude Code")
            })
            .count()
    };
    harness.click("Stop session");
    harness.settle();
    assert_eq!(stopped(&harness), 1);
    harness.click("Stop agents here");
    harness.settle();
    assert_eq!(stopped(&harness), 2);
    let conn = harness.app.workspace(tab).unwrap().conn_id.clone();
    assert_eq!(
        harness.app.connections.get(&conn).unwrap().agents.access,
        AgentAccess::Off
    );
    assert_eq!(
        crate::ui::toast::said(&harness.ctx).as_deref(),
        Some("Stopped 1 agent. Access is now Off for Fixture · dev.")
    );
    assert!(harness.has(&entry(&Look::macos(), "Off here")));
}

#[test]
fn also_names_who_works_elsewhere_and_show_opens_its_window_on_its_agents() {
    let (mut harness, tab) = connected(Look::macos());
    harness.app.settings.mcp_enabled = true;
    allow(&mut harness, tab, AgentAccess::Read);
    let ledger = working_elsewhere(&mut harness, "Cursor");
    harness.app.apply(Action::ShowAgents(tab));
    harness.settle();
    // Said in every state that the header has, nobody here too.
    assert!(harness.has("Also: Cursor on Ledger · staging"));
    let mut nowhere = session(&harness, None, "Zed");
    nowhere.last_call_at = None;
    harness.app.agents.sessions.push(nowhere);
    assert!(harness.has("Also: Cursor on Ledger · staging and 1 more"));
    let windows = harness.app.tabs.len();
    harness.click("Show");
    harness.settle();
    assert_eq!(harness.app.tabs.len(), windows + 1);
    let shown = harness.app.active_tab_id();
    let workspace = harness.app.workspace(shown).unwrap();
    assert_eq!(workspace.conn_id, ledger);
    assert!(workspace.agents_in_front());
}

#[test]
fn omarchys_keys_move_among_the_sessions_stop_one_and_show_where_one_works() {
    let (mut harness, tab) = connected(Look::omarchy());
    harness.app.settings.mcp_enabled = true;
    allow(&mut harness, tab, AgentAccess::Read);
    harness.app.serve_agents();
    let here = session(&harness, Some(tab), "Claude Code");
    harness.app.agents.sessions.push(here);
    let ledger = working_elsewhere(&mut harness, "Cursor");
    harness.app.apply(Action::ShowAgents(tab));
    harness.settle();
    let selected = |harness: &Harness| harness.app.agents_selected(tab).map(|s| s.id.clone());
    assert_eq!(
        selected(&harness).as_deref(),
        Some("session of Claude Code")
    );
    // Enter on a session here shows nothing more.
    let windows = harness.app.tabs.len();
    harness.press(egui::Key::Enter, egui::Modifiers::NONE);
    assert_eq!(harness.app.tabs.len(), windows);
    super::tests::type_key(&mut harness, egui::Key::J, "j");
    assert_eq!(selected(&harness).as_deref(), Some("session of Cursor"));
    // Past the last, it stays.
    super::tests::type_key(&mut harness, egui::Key::J, "j");
    assert_eq!(selected(&harness).as_deref(), Some("session of Cursor"));
    // `S` stops the selected one, wherever it works.
    super::tests::type_key(&mut harness, egui::Key::S, "S");
    assert!(harness.app.backend.sent.iter().any(|sent| matches!(sent,
        crate::backend::Command::Agents(crate::backend::AgentsCommand::StopSession(id))
            if id == "session of Cursor")));
    // Enter shows its connection's window, on its Agents tab.
    harness.press(egui::Key::Enter, egui::Modifiers::NONE);
    assert_eq!(harness.app.tabs.len(), windows + 1);
    let shown = harness.app.active_tab_id();
    assert_eq!(harness.app.workspace(shown).unwrap().conn_id, ledger);
    // Back, `k`, and `X` stops who is here and turns access Off.
    harness.app.apply(Action::ActivateConnTab(tab));
    super::tests::type_key(&mut harness, egui::Key::K, "k");
    super::tests::type_key(&mut harness, egui::Key::X, "X");
    let conn = harness.app.workspace(tab).unwrap().conn_id.clone();
    assert_eq!(
        harness.app.connections.get(&conn).unwrap().agents.access,
        AgentAccess::Off
    );
    // This look floats no toast: its status line says it.
    assert!(
        harness
            .has(&Look::omarchy().label("Stopped 1 agent. Access is now Off for Fixture · dev."))
    );
}

/// Two sessions here: A, which called last, then B.
fn two_here(harness: &mut Harness, tab: ConnTabId) {
    let now = SystemTime::now();
    let mut a = session(harness, Some(tab), "A");
    a.last_call_at = Some(now);
    let mut b = session(harness, Some(tab), "B");
    b.last_call_at = Some(now - std::time::Duration::from_secs(5));
    said(harness, AgentsEvent::Sessions(vec![a, b]));
}

#[test]
fn the_selection_stays_on_the_session_the_person_saw_when_the_list_reorders() {
    let (mut harness, tab) = connected(Look::omarchy());
    harness.app.settings.mcp_enabled = true;
    allow(&mut harness, tab, AgentAccess::Read);
    harness.app.serve_agents();
    two_here(&mut harness, tab);
    harness.app.apply(Action::ShowAgents(tab));
    super::tests::type_key(&mut harness, egui::Key::J, "j");
    let selected = |harness: &Harness| harness.app.agents_selected(tab).map(|s| s.id.clone());
    assert_eq!(selected(&harness).as_deref(), Some("session of B"));
    // B calls: it goes first in the list, and stays the one selected.
    let mut sessions = harness.app.agents.sessions.clone();
    sessions[1].last_call_at = Some(SystemTime::now() + std::time::Duration::from_secs(20));
    said(&mut harness, AgentsEvent::Sessions(sessions.clone()));
    assert_eq!(selected(&harness).as_deref(), Some("session of B"));
    super::tests::type_key(&mut harness, egui::Key::S, "S");
    let stopped: Vec<_> = harness
        .app
        .backend
        .sent
        .iter()
        .filter_map(|sent| match sent {
            crate::backend::Command::Agents(crate::backend::AgentsCommand::StopSession(id)) => {
                Some(id.as_str())
            }
            _ => None,
        })
        .collect();
    assert_eq!(stopped, ["session of B"]);
    // B ends: the first is selected, and `k` from it stays there.
    said(
        &mut harness,
        AgentsEvent::Sessions(vec![sessions[0].clone()]),
    );
    assert_eq!(selected(&harness).as_deref(), Some("session of A"));
    super::tests::type_key(&mut harness, egui::Key::K, "k");
    assert_eq!(selected(&harness).as_deref(), Some("session of A"));
}

#[test]
fn with_sessions_here_the_arrows_still_move_in_the_tree_and_enter_leaves_a_button_its_own() {
    for look in looks() {
        let (mut harness, tab) = connected(look);
        harness.app.settings.mcp_enabled = true;
        allow(&mut harness, tab, AgentAccess::Read);
        let here = session(&harness, Some(tab), "Claude Code");
        said(&mut harness, AgentsEvent::Sessions(vec![here]));
        harness.app.apply(Action::ShowAgents(tab));
        harness.settle();
        let cursor = |harness: &Harness| harness.app.workspace(tab).unwrap().tree.cursor.clone();
        let before = cursor(&harness);
        harness.press(egui::Key::ArrowDown, egui::Modifiers::NONE);
        assert_ne!(cursor(&harness), before, "{}", look.name);
    }
    // Enter on a focused button is the button's: here, Disconnect.
    let (mut harness, tab) = connected(Look::omarchy());
    harness.app.settings.mcp_enabled = true;
    allow(&mut harness, tab, AgentAccess::Read);
    let here = session(&harness, Some(tab), "Claude Code");
    said(&mut harness, AgentsEvent::Sessions(vec![here]));
    harness.app.apply(Action::ShowAgents(tab));
    harness.press(egui::Key::Tab, egui::Modifiers::NONE);
    harness.press(egui::Key::Tab, egui::Modifiers::NONE);
    harness.press(egui::Key::Enter, egui::Modifiers::NONE);
    assert!(harness.app.workspace(tab).is_none(), "disconnected");
}

#[test]
fn omarchy_lists_who_works_elsewhere_while_nobody_is_here() {
    let (mut harness, tab) = connected(Look::omarchy());
    harness.app.settings.mcp_enabled = true;
    allow(&mut harness, tab, AgentAccess::Read);
    harness.app.serve_agents();
    let ledger = working_elsewhere(&mut harness, "Cursor");
    harness.app.apply(Action::ShowAgents(tab));
    harness.settle();
    assert!(harness.has(&entry(&Look::omarchy(), "None connected")));
    assert!(harness.has("cursor · Ledger · staging · enter shows"));
    // With the way to connect one beside it.
    assert!(harness.has(crate::endpoint::agent_command()));
    // `X` has nobody here to stop: nothing changes.
    super::tests::type_key(&mut harness, egui::Key::X, "X");
    let conn = harness.app.workspace(tab).unwrap().conn_id.clone();
    assert_eq!(
        harness.app.connections.get(&conn).unwrap().agents.access,
        AgentAccess::Read
    );
    harness.press(egui::Key::Enter, egui::Modifiers::NONE);
    let shown = harness.app.active_tab_id();
    assert_eq!(harness.app.workspace(shown).unwrap().conn_id, ledger);
}

/// The id of the log's read the app asked for last, and whose calls.
fn log_asked(harness: &Harness) -> (u64, Option<String>) {
    harness
        .app
        .backend
        .sent
        .iter()
        .rev()
        .find_map(|sent| match sent {
            crate::backend::Command::Agents(crate::backend::AgentsCommand::ReadLog {
                asked,
                filter,
                ..
            }) => Some((*asked, filter.connection.clone())),
            _ => None,
        })
        .expect("a read of the log")
}

/// Answers the log's read with one call of Claude Code's.
fn log_answered(harness: &mut Harness) {
    let (asked, connection) = log_asked(harness);
    let entry = tabletist_mcp::Entry {
        at_ms: 1_791_880_982_000,
        session: "s".into(),
        agent: "Claude Code".into(),
        transport: "stdio".into(),
        connection_id: connection,
        connection: Some("Fixture · dev".into()),
        tool: "run_query".into(),
        sql: Some("SELECT  id\n  FROM books".into()),
        result: "ok".into(),
        rows: Some(3),
        columns: Some(2),
        took_ms: Some(4),
        ..tabletist_mcp::Entry::default()
    };
    let page = tabletist_mcp::Page {
        calls: vec![tabletist_mcp::Logged { id: 5, entry }],
        total: 1,
        more: false,
        agents: vec!["Claude Code".into(), "Cursor".into()],
    };
    said(
        harness,
        AgentsEvent::LogRead {
            asked,
            page: Ok(page),
        },
    );
}

#[test]
fn the_log_lists_the_calls_and_says_what_the_selected_one_came_to() {
    for look in looks() {
        let (mut harness, tab) = connected(look);
        harness.app.settings.mcp_enabled = true;
        allow(&mut harness, tab, AgentAccess::Read);
        harness.app.apply(Action::ShowAgents(tab));
        harness.settle();
        // From nobody here: Show log, which reads this connection's calls.
        harness.click("Show log");
        harness.settle();
        let conn = harness.app.workspace(tab).unwrap().conn_id.0.clone();
        assert_eq!(
            log_asked(&harness).1.as_deref(),
            Some(conn.as_str()),
            "{}",
            look.name
        );
        log_answered(&mut harness);
        let at = crate::clock::of_millis(1_791_880_982_000);
        let row = format!(
            "{} · {} · run_query · SELECT id FROM books · 3 · 4 · ok",
            look.label(&crate::clock::day(at)),
            look.label("Claude Code"),
        );
        assert!(harness.has(&row), "{}", look.name);
        assert!(
            harness.has(&look.label("This connection · 1 call · last 30 days · on this computer")),
            "{}",
            look.name
        );
        // Selected, it says what it came to, and that rows are not kept.
        let request = harness
            .painted_rect("SELECT id FROM books")
            .expect("the request");
        super::tests::click_at(&mut harness, request.center());
        harness.settle();
        let detail = look.label(&format!(
            "Claude Code · {} · 3 rows, 2 columns returned · 4 ms · rows themselves are not logged",
            crate::clock::moment(at),
        ));
        assert!(harness.has(&detail), "{}", look.name);
        // Every connection's calls.
        if look.terminal {
            super::tests::type_key(&mut harness, egui::Key::L, "L");
        } else {
            harness.click("Calls on");
            harness.settle();
            harness.click("All connections");
            harness.settle();
        }
        assert_eq!(log_asked(&harness).1, None, "{}", look.name);
    }
}

/// The filter of the last read of the log.
fn log_filter(harness: &Harness) -> tabletist_mcp::LogFilter {
    harness
        .app
        .backend
        .sent
        .iter()
        .rev()
        .find_map(|sent| match sent {
            crate::backend::Command::Agents(crate::backend::AgentsCommand::ReadLog {
                filter,
                ..
            }) => Some(filter.clone()),
            _ => None,
        })
        .expect("a read of the log")
}

/// Answers the log's last read with no call, of a log in which Claude
/// Code and Cursor called: what a filter that matches nothing reads.
fn log_answered_empty(harness: &mut Harness) {
    let (asked, _) = log_asked(harness);
    let page = tabletist_mcp::Page {
        agents: vec!["Claude Code".into(), "Cursor".into()],
        ..tabletist_mcp::Page::default()
    };
    said(
        harness,
        AgentsEvent::LogRead {
            asked,
            page: Ok(page),
        },
    );
}

/// Gives the keyboard to the field named `label`, as a screen reader
/// would.
fn focus_field(harness: &mut Harness, label: &str) {
    use egui::accesskit::Role;
    let tree = harness.settle();
    let field = tree
        .nodes
        .iter()
        .find(|(_, node)| node.role() == Role::TextInput && node.label() == Some(label))
        .map(|(id, _)| *id)
        .unwrap_or_else(|| panic!("no field named {label:?}"));
    harness.frame(vec![egui::Event::AccessKitActionRequest(
        egui::accesskit::ActionRequest {
            target_tree: egui::accesskit::TreeId::ROOT,
            target_node: field,
            action: egui::accesskit::Action::Focus,
            data: None,
        },
    )]);
    harness.settle();
}

/// The Log view of `tab`'s connection, read once: one call of Claude
/// Code's, in a log Cursor called on too.
fn on_the_log(look: Look) -> (Harness, ConnTabId) {
    let (mut harness, tab) = connected(look);
    harness.app.settings.mcp_enabled = true;
    allow(&mut harness, tab, AgentAccess::Read);
    harness.app.apply(Action::ShowAgentsView {
        tab,
        view: crate::model::AgentsView::Log,
    });
    harness.settle();
    log_answered(&mut harness);
    assert!(harness.painted_rect("SELECT id FROM books").is_some());
    (harness, tab)
}

#[test]
fn the_logs_filters_read_an_agents_calls_a_result_and_a_search() {
    let (mut harness, tab) = on_the_log(Look::macos());
    let conn = harness.app.workspace(tab).unwrap().conn_id.0.clone();
    // The agents that called on the connection are offered.
    harness.click("Agent");
    harness.settle();
    assert!(harness.has("Claude Code"));
    harness.click("Cursor");
    harness.settle();
    let mut wanted = tabletist_mcp::LogFilter {
        connection: Some(conn.clone()),
        agent: Some("Cursor".into()),
        ..tabletist_mcp::LogFilter::default()
    };
    assert_eq!(log_filter(&harness), wanted);
    // How they ended, as the list says it.
    harness.click("Result");
    harness.settle();
    harness.click("refused");
    harness.settle();
    wanted.result = Some("refused".into());
    assert_eq!(log_filter(&harness), wanted);
    // What the requests hold, as it is typed.
    focus_field(&mut harness, "Search requests");
    harness.frame(vec![egui::Event::Text("from books".into())]);
    harness.settle();
    wanted.search = Some("from books".into());
    assert_eq!(log_filter(&harness), wanted);
    // Matched by none: said so. One that matches is counted as such.
    log_answered_empty(&mut harness);
    assert!(harness.has("No calls in the log match these filters."));
    harness.frame(vec![egui::Event::Text("s".into())]);
    harness.settle();
    log_answered(&mut harness);
    assert!(harness.has("This connection · 1 call matches · last 30 days · on this computer"));
    wanted.search = Some("from bookss".into());
    // Every agent again, and every connection: the other filters stay.
    harness.click("Agent");
    harness.settle();
    harness.click("All agents");
    harness.settle();
    harness.click("Calls on");
    harness.settle();
    harness.click("All connections");
    harness.settle();
    wanted.agent = None;
    wanted.connection = None;
    assert_eq!(log_filter(&harness), wanted);
}

#[test]
fn omarchys_slash_opens_the_logs_filters_and_esc_keeps_them() {
    let (mut harness, tab) = on_the_log(Look::omarchy());
    let conn = harness.app.workspace(tab).unwrap().conn_id.0.clone();
    let view = |harness: &Harness| {
        harness
            .app
            .workspace(tab)
            .unwrap()
            .tabs
            .iter()
            .find_map(|open| match open {
                crate::model::Tab::Agents(agents) => Some((agents.view, agents.log.filters_open)),
                _ => None,
            })
    };
    use crate::model::AgentsView;
    assert!(harness.painted_rect("/ filter").is_some());
    super::tests::type_key(&mut harness, egui::Key::Slash, "/");
    harness.settle();
    assert_eq!(view(&harness), Some((AgentsView::Log, true)));
    // The search has the keyboard: what is typed is the search's, the
    // tab's letters among it.
    harness.frame(vec![egui::Event::Text("all".into())]);
    harness.settle();
    assert_eq!(view(&harness), Some((AgentsView::Log, true)));
    assert_eq!(log_filter(&harness).search.as_deref(), Some("all"));
    // Tab goes to the agents, then to Claude Code's, which Space picks.
    harness.press(egui::Key::Tab, egui::Modifiers::NONE);
    harness.press(egui::Key::Tab, egui::Modifiers::NONE);
    harness.press(egui::Key::Space, egui::Modifiers::NONE);
    harness.settle();
    assert_eq!(log_filter(&harness).agent.as_deref(), Some("Claude Code"));
    // Esc closes them and keeps them, which the line under the bar says.
    harness.press(egui::Key::Escape, egui::Modifiers::NONE);
    harness.settle();
    assert_eq!(view(&harness), Some((AgentsView::Log, false)));
    assert!(harness.has("filters agent claude code · search “all”"));
    assert_eq!(
        log_filter(&harness),
        tabletist_mcp::LogFilter {
            connection: Some(conn),
            agent: Some("Claude Code".into()),
            result: None,
            search: Some("all".into()),
        }
    );
    // Open again, the search has the keyboard again, with what it holds.
    super::tests::type_key(&mut harness, egui::Key::Slash, "/");
    harness.settle();
    harness.press(egui::Key::Backspace, egui::Modifiers::NONE);
    harness.settle();
    assert_eq!(log_filter(&harness).search.as_deref(), Some("al"));
    // Left from a word with `l`, and shown again: they are closed, and `/`
    // gives the search the keyboard once more.
    harness.press(egui::Key::Tab, egui::Modifiers::NONE);
    super::tests::type_key(&mut harness, egui::Key::L, "l");
    assert_eq!(view(&harness), Some((AgentsView::Activity, false)));
    super::tests::type_key(&mut harness, egui::Key::L, "l");
    assert_eq!(view(&harness), Some((AgentsView::Log, false)));
    super::tests::type_key(&mut harness, egui::Key::Slash, "/");
    harness.settle();
    harness.frame(vec![egui::Event::Text("l".into())]);
    harness.settle();
    assert_eq!(log_filter(&harness).search.as_deref(), Some("all"));
    assert_eq!(view(&harness), Some((AgentsView::Log, true)));
}

#[test]
fn omarchy_shows_the_log_with_l_and_the_agents_log_line() {
    let (mut harness, tab) = connected(Look::omarchy());
    harness.app.apply(Action::ShowAgents(tab));
    let view = |harness: &Harness| {
        harness
            .app
            .workspace(tab)
            .unwrap()
            .tabs
            .iter()
            .find_map(|open| match open {
                crate::model::Tab::Agents(agents) => Some(agents.view),
                _ => None,
            })
    };
    // With the server off too: the log is the file's.
    super::tests::type_key(&mut harness, egui::Key::L, "l");
    assert_eq!(view(&harness), Some(crate::model::AgentsView::Log));
    super::tests::type_key(&mut harness, egui::Key::L, "l");
    assert_eq!(view(&harness), Some(crate::model::AgentsView::Activity));
    // From another tab, by the prompt's line.
    let query = harness.add_sql_tab(tab);
    harness.app.apply(Action::ActivateTab { tab, id: query });
    harness.app.workspace_mut(tab).unwrap().command = Some("agents log".into());
    harness.app.apply(Action::RunCommand(tab));
    assert!(front(&harness, tab));
    assert_eq!(view(&harness), Some(crate::model::AgentsView::Log));
}

#[test]
fn in_the_log_view_the_activitys_letters_do_nothing_and_l_is_named() {
    // Worked out, as the backend's start does off the UI thread.
    crate::endpoint::agent_command();
    let (mut harness, tab) = connected(Look::omarchy());
    allow(&mut harness, tab, AgentAccess::Read);
    harness.app.apply(Action::ShowAgents(tab));
    harness.settle();
    // The status line names the tab's own key for the log, not the
    // prompt's line.
    assert!(harness.painted_rect(":agents log").is_none());
    super::tests::type_key(&mut harness, egui::Key::L, "l");
    harness.settle();
    assert!(harness.painted_rect(":agents log").is_none());
    // The server off: `o` turns nothing on from the log.
    super::tests::type_key(&mut harness, egui::Key::O, "o");
    assert!(!harness.app.settings.mcp_enabled);
    // On: `y` copies nothing from the log.
    harness.app.settings.mcp_enabled = true;
    super::tests::type_key(&mut harness, egui::Key::Y, "y");
    assert_eq!(crate::ui::toast::said(&harness.ctx), None);
}

#[test]
fn a_call_selected_with_j_is_kept_in_sight() {
    let (mut harness, tab) = connected(Look::omarchy());
    harness.app.apply(Action::ShowAgentsView {
        tab,
        view: crate::model::AgentsView::Log,
    });
    let (asked, _) = log_asked(&harness);
    let calls = (0..200)
        .map(|n| tabletist_mcp::Logged {
            id: 1_000 - n,
            entry: tabletist_mcp::Entry {
                at_ms: 1_791_880_982_000,
                agent: "Claude Code".into(),
                tool: "run_query".into(),
                sql: Some(format!("SELECT {n}")),
                result: "ok".into(),
                ..tabletist_mcp::Entry::default()
            },
        })
        .collect();
    let page = tabletist_mcp::Page {
        calls,
        total: 200,
        more: false,
        agents: Vec::new(),
    };
    said(
        &mut harness,
        AgentsEvent::LogRead {
            asked,
            page: Ok(page),
        },
    );
    // Only the lines in sight are laid out.
    assert!(harness.painted_rect("SELECT 0").is_some());
    assert!(harness.painted_rect("SELECT 150").is_none());
    for _ in 0..61 {
        super::tests::type_key(&mut harness, egui::Key::J, "j");
    }
    harness.settle();
    harness.settle();
    let row = harness
        .painted_rect("SELECT 60")
        .expect("the selected call");
    assert!(row.bottom() <= harness.size.y, "{row:?}");
}

/// The Connections list with one saved connection, Shop · dev, selected.
#[cfg(any(unix, windows))]
fn on_the_list(look: Look) -> (Harness, crate::connections::ConnectionId) {
    let mut harness = Harness::new();
    harness.set_look(look);
    let mut shop = crate::testing::fixture_connection();
    shop.name = "Shop".into();
    let conn = shop.id.clone();
    harness.app.connections.upsert(shop);
    let tab = harness.app.active_tab_id();
    harness.app.apply(Action::SelectConnection {
        tab,
        conn: Some(conn.clone()),
    });
    harness.settle();
    (harness, conn)
}

#[cfg(any(unix, windows))]
fn access_of(harness: &Harness, conn: &crate::connections::ConnectionId) -> AgentAccess {
    harness.app.connections.get(conn).unwrap().agents.access
}

#[cfg(any(unix, windows))]
#[test]
fn the_list_says_what_agents_may_do_and_its_menu_changes_it() {
    let (mut harness, conn) = on_the_list(Look::macos());
    assert!(harness.has("Agents Off"));
    harness.click("More actions for Shop");
    harness.settle();
    harness.click("Agents Read");
    harness.settle();
    assert_eq!(access_of(&harness, &conn), AgentAccess::Read);
    assert!(harness.has("Agents Read"));
    // Tables, columns and limits…: the editor, at its Agents part. (A
    // click no pointer made leaves the menu open.)
    harness.click("Tables, columns and limits…");
    harness.settle();
    assert!(matches!(
        harness.app.dialog,
        Some(crate::model::Dialog::Connection(_))
    ));
    assert!(harness.has("Access through Tabletist (MCP)"));
}

#[cfg(any(unix, windows))]
#[test]
fn the_lists_menu_shows_the_agents_tab() {
    let (mut harness, conn) = on_the_list(Look::macos());
    harness.click("More actions for Shop");
    harness.settle();
    harness.click("Show Agents tab");
    harness.settle();
    let shown = harness.app.active_tab_id();
    let workspace = harness.app.workspace(shown).expect("its window");
    assert_eq!(workspace.conn_id, conn);
    assert!(workspace.agents_in_front());
}

/// Types `text` over what the editor's field labelled `label` holds: the
/// field is found by the label that names it, as a screen reader finds it.
#[cfg(any(unix, windows))]
fn type_over(harness: &mut Harness, label: &str, text: &str) {
    use egui::accesskit::Role;
    let tree = harness.settle();
    let named = crate::testing::node(&tree, label, Role::Label)
        .unwrap_or_else(|| panic!("no label {label:?}"));
    let field = tree
        .nodes
        .iter()
        .find(|(_, node)| node.role() == Role::TextInput && node.labelled_by().contains(&named))
        .map(|(id, _)| *id)
        .unwrap_or_else(|| panic!("no field labelled {label:?}"));
    harness.frame(vec![egui::Event::AccessKitActionRequest(
        egui::accesskit::ActionRequest {
            target_tree: egui::accesskit::TreeId::ROOT,
            target_node: field,
            action: egui::accesskit::Action::Focus,
            data: None,
        },
    )]);
    harness.settle();
    harness.press(egui::Key::A, egui::Modifiers::COMMAND);
    harness.frame(vec![egui::Event::Text(text.into())]);
    harness.settle();
}

#[cfg(any(unix, windows))]
#[test]
fn omarchys_box_sets_what_agents_may_do_from_the_list() {
    let (mut harness, conn) = on_the_list(Look::omarchy());
    // At the end of the row's line.
    let line_ends = |harness: &Harness, end: &str| {
        harness
            .text_rects
            .iter()
            .any(|(text, _)| text.ends_with(end))
    };
    assert!(line_ends(&harness, "· agents off"));
    super::tests::type_key(&mut harness, egui::Key::A, "a");
    assert!(matches!(
        harness.app.dialog,
        Some(crate::model::Dialog::AgentAccess(_))
    ));
    assert!(harness.has("agents · Shop · dev"));
    // `j` to read, Enter sets it.
    super::tests::type_key(&mut harness, egui::Key::J, "j");
    harness.press(egui::Key::Enter, egui::Modifiers::NONE);
    assert!(harness.app.dialog.is_none());
    assert_eq!(access_of(&harness, &conn), AgentAccess::Read);
    harness.settle();
    assert!(line_ends(&harness, "· agents read"));
    // Esc leaves it as it was.
    super::tests::type_key(&mut harness, egui::Key::A, "a");
    super::tests::type_key(&mut harness, egui::Key::K, "k");
    harness.press(egui::Key::Escape, egui::Modifiers::NONE);
    assert!(harness.app.dialog.is_none());
    assert_eq!(access_of(&harness, &conn), AgentAccess::Read);
    // `t`: the connection's window, on its Agents tab.
    super::tests::type_key(&mut harness, egui::Key::A, "a");
    super::tests::type_key(&mut harness, egui::Key::T, "t");
    let shown = harness.app.active_tab_id();
    let workspace = harness.app.workspace(shown).expect("its window");
    assert_eq!(workspace.conn_id, conn);
    assert!(workspace.agents_in_front());
}

#[cfg(any(unix, windows))]
#[test]
fn the_editors_agents_part_sets_access_rows_and_timeout() {
    for look in looks() {
        let (mut harness, conn) = on_the_list(look);
        harness.app.apply(Action::EditAgents {
            conn: conn.clone(),
            allow: false,
        });
        // A scroll is animated: it lands some frames on.
        for _ in 0..20 {
            harness.settle();
        }
        harness.click(&look.label("Read"));
        harness.settle();
        type_over(&mut harness, &look.label("Rows per answer"), "50");
        let timeout = if look.terminal {
            "Timeout, seconds"
        } else {
            "Statement timeout, seconds"
        };
        type_over(&mut harness, &look.label(timeout), "30");
        harness.app.apply(Action::SaveConnection { connect: false });
        let agents = &harness.app.connections.get(&conn).unwrap().agents;
        assert_eq!(agents.access, AgentAccess::Read, "{}", look.name);
        assert_eq!(
            (agents.rows, agents.timeout_secs),
            (50, 30),
            "{}",
            look.name
        );
        assert!(harness.app.agents_held.is_empty(), "{}", look.name);
    }
}

/// The connection dialog's form.
#[cfg(any(unix, windows))]
fn form(harness: &Harness) -> &crate::model::ConnectionForm {
    match &harness.app.dialog {
        Some(crate::model::Dialog::Connection(form)) => form,
        _ => panic!("no connection dialog"),
    }
}

#[cfg(any(unix, windows))]
fn form_mut(harness: &mut Harness) -> &mut crate::model::ConnectionForm {
    match &mut harness.app.dialog {
        Some(crate::model::Dialog::Connection(form)) => form,
        _ => panic!("no connection dialog"),
    }
}

/// Opens the editor at its Agents part, and waits for its scroll.
#[cfg(any(unix, windows))]
fn at_agents(harness: &mut Harness, conn: &crate::connections::ConnectionId) {
    harness.app.apply(Action::EditAgents {
        conn: conn.clone(),
        allow: false,
    });
    for _ in 0..20 {
        harness.settle();
    }
}

/// `name` typed for Add table… (its field opened where it is not open
/// already), then Enter.
#[cfg(any(unix, windows))]
fn add_table(harness: &mut Harness, look: &Look, name: &str) {
    type_table(harness, look, name);
    harness.press(egui::Key::Enter, egui::Modifiers::NONE);
    harness.settle();
}

/// `name` typed in Add table…'s field, opened where it is not open.
#[cfg(any(unix, windows))]
fn type_table(harness: &mut Harness, look: &Look, name: &str) {
    if form(harness).agents_table_typed.is_none() {
        harness.click(&look.label("Add table…"));
    }
    harness.settle();
    harness.frame(vec![egui::Event::Text(name.into())]);
    harness.settle();
}

#[cfg(any(unix, windows))]
#[test]
fn the_editors_tables_control_lists_what_agents_may_not_read() {
    use tabletist_db::agent::Tables;
    let names = |written: &[&str]| -> Vec<tabletist_db::agent::TableName> {
        written.iter().map(|entry| entry.parse().unwrap()).collect()
    };
    let too_many =
        "The table's name has more than one dot: it is a table, or a schema and a table.";
    for look in looks() {
        let (mut harness, conn) = on_the_list(look);
        at_agents(&mut harness, &conn);
        harness.click(&look.label("Read"));
        harness.click(&look.label("All except"));
        harness.settle();
        // Enter adds a name and keeps the field, with the keyboard, for
        // the next.
        add_table(&mut harness, &look, "customers");
        add_table(&mut harness, &look, "sales.orders");
        assert_eq!(form(&harness).agents_table_typed.as_deref(), Some(""));
        // The same name again, in another case, is there once.
        add_table(&mut harness, &look, "Customers");
        // One that does not read is said under the names, and the field
        // keeps it to be put right.
        add_table(&mut harness, &look, "a.b.c");
        assert!(harness.has(&look.label(too_many)), "{}", look.name);
        assert_eq!(form(&harness).agents_table_typed.as_deref(), Some("a.b.c"));
        // Escape closes the field and nothing else.
        harness.press(egui::Key::Escape, egui::Modifiers::NONE);
        harness.settle();
        assert!(harness.app.dialog.is_some(), "{}", look.name);
        assert_eq!(form(&harness).agents_table_typed, None, "{}", look.name);
        assert!(!harness.has(&look.label(too_many)), "{}", look.name);
        harness.click(&look.label("Remove sales.orders"));
        harness.settle();
        // A name typed and not added goes to its list when another mode is
        // chosen; one that does not read keeps the mode until put right.
        type_table(&mut harness, &look, "a.b.c");
        harness.click(&look.label("Only"));
        harness.settle();
        assert_eq!(
            form(&harness).agents_tables.mode,
            crate::model::TablesMode::AllExcept,
            "{}",
            look.name
        );
        form_mut(&mut harness).agents_table_typed = Some(String::new());
        harness.settle();
        type_table(&mut harness, &look, "orders");
        harness.click(&look.label("Only"));
        harness.settle();
        harness.click(&look.label("All except"));
        harness.settle();
        assert!(harness.has(&look.label("Remove orders")), "{}", look.name);
        harness.click(&look.label("Remove orders"));
        harness.settle();
        // "Only" keeps a list of its own, and "All except" finds its again.
        harness.click(&look.label("Only"));
        harness.settle();
        assert!(
            !harness.has(&look.label("Remove customers")),
            "{}",
            look.name
        );
        let only = "Agents may read these tables and no others, the ones made later included. A \
                    function from an extension can still read the others. For strict isolation, \
                    use a database user without access.";
        assert!(harness.has(&look.label(only)), "{}", look.name);
        add_table(&mut harness, &look, "books");
        harness.press(egui::Key::Escape, egui::Modifiers::NONE);
        harness.click(&look.label("All except"));
        harness.settle();
        assert!(
            harness.has(&look.label("Remove customers")),
            "{}",
            look.name
        );
        // A name typed and not yet added is added by the save.
        type_table(&mut harness, &look, "payroll");
        harness.app.apply(Action::SaveConnection { connect: false });
        let saved = harness.app.connections.get(&conn).unwrap();
        assert_eq!(
            saved.agents.tables,
            Tables::AllExcept(names(&["customers", "payroll"])),
            "{}",
            look.name
        );
        // The person's own: shown, so that comparing again holds nothing.
        harness.app.check_agents();
        assert!(harness.app.agents_held.is_empty(), "{}", look.name);
        // Opened again it shows them. A name that does not read, typed and
        // not added, refuses the save and says why.
        at_agents(&mut harness, &conn);
        assert!(
            harness.has(&look.label("Remove customers")),
            "{}",
            look.name
        );
        type_table(&mut harness, &look, "a.b.c");
        harness.app.apply(Action::SaveConnection { connect: false });
        assert!(harness.app.dialog.is_some(), "{}", look.name);
        assert_eq!(
            form(&harness).message.as_deref(),
            Some(too_many),
            "{}",
            look.name
        );
        form_mut(&mut harness).agents_table_typed = None;
        form_mut(&mut harness).agents_table_error = None;
        harness.settle();
        // A name left in the field when Tab moves on is added.
        type_table(&mut harness, &look, "orders");
        harness.press(egui::Key::Tab, egui::Modifiers::NONE);
        harness.settle();
        assert!(harness.has(&look.label("Remove orders")), "{}", look.name);
        assert_eq!(form(&harness).agents_table_typed, None, "{}", look.name);
        // Widened by the person: theirs, and never held.
        harness.click(&look.label("All"));
        harness.settle();
        harness.app.apply(Action::SaveConnection { connect: false });
        assert_eq!(
            harness.app.connections.get(&conn).unwrap().agents.tables,
            Tables::All,
            "{}",
            look.name
        );
        harness.app.check_agents();
        assert!(harness.app.agents_held.is_empty(), "{}", look.name);
    }
}

/// A press and a release of the pointer at `at`.
#[cfg(any(unix, windows))]
fn press_at(harness: &mut Harness, at: egui::Pos2) {
    let press = |pressed| egui::Event::PointerButton {
        pos: at,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: egui::Modifiers::NONE,
    };
    harness.frame(vec![egui::Event::PointerMoved(at)]);
    harness.frame(vec![press(true)]);
    harness.frame(vec![press(false)]);
    harness.settle();
}

/// The tables loaded in the tree of `tab`'s window: `main`'s, these.
#[cfg(any(unix, windows))]
fn loaded(harness: &mut Harness, tab: ConnTabId, names: &[&str]) {
    use tabletist_db::{ObjectInfo, ObjectKind};
    let workspace = harness.app.workspace_mut(tab).unwrap();
    workspace.tree.schemas.value = Some(vec!["main".into()]);
    let node = workspace.tree.nodes.entry("main".into()).or_default();
    node.objects.value = Some(
        names
            .iter()
            .map(|name| ObjectInfo {
                name: (*name).into(),
                kind: ObjectKind::Table,
                estimated_rows: None,
            })
            .collect(),
    );
}

#[cfg(any(unix, windows))]
#[test]
fn two_windows_of_a_connection_are_known_together_and_another_database_is_not() {
    let (mut harness, first) = connected(Look::macos());
    // A second window, of the same connection.
    harness.app.apply(Action::ShowConnections);
    let second = harness.connect_fake();
    assert_ne!(first, second);
    // The fixture is saved anew at each connect: the second window is
    // made the first's connection's.
    let conn = harness.app.workspace(first).unwrap().conn_id.clone();
    harness.app.workspace_mut(second).unwrap().conn_id = conn.clone();
    loaded(&mut harness, first, &["books", "orders"]);
    loaded(&mut harness, second, &["books"]);
    let known = harness.app.loaded_tables(&conn).unwrap();
    assert_eq!(known.has(&"orders".parse().unwrap()), Some(true));
    assert!(!known.schemas_cut);
    // One window's list of schemas cut by the catalog: a schema not in it
    // is not known.
    harness
        .app
        .workspace_mut(second)
        .unwrap()
        .tree
        .schemas
        .value = Some(
        (0..tabletist_db::MAX_LISTED)
            .map(|at| format!("s{at:05}"))
            .collect(),
    );
    let known = harness.app.loaded_tables(&conn).unwrap();
    assert!(known.schemas_cut);
    assert_eq!(known.has(&"zz_sales.orders".parse().unwrap()), None);
    // A window switched to another database is not this connection's.
    harness.app.workspace_mut(first).unwrap().spec.database = "other".into();
    harness.app.workspace_mut(second).unwrap().spec.database = "other".into();
    assert_eq!(harness.app.loaded_tables(&conn), None);
}

#[cfg(any(unix, windows))]
#[test]
fn add_table_offers_the_windows_tables_and_marks_a_name_in_none() {
    use tabletist_db::{ObjectInfo, ObjectKind};
    let gone = |look: &Look| {
        look.label(
            "Agents may not use this connection while a hidden table is not in its database. \
             Take the name out, or add the table's new name.",
        )
    };
    for look in looks() {
        let (mut harness, tab) = connected(look);
        // The window's sidebar has loaded its one schema.
        let workspace = harness.app.workspace_mut(tab).unwrap();
        workspace.tree.schemas.value = Some(vec!["main".into()]);
        let node = workspace.tree.nodes.entry("main".into()).or_default();
        node.objects.value = Some(
            ["authors", "books", "customers", "orders"]
                .iter()
                .map(|name| ObjectInfo {
                    name: (*name).into(),
                    kind: ObjectKind::Table,
                    estimated_rows: None,
                })
                .collect(),
        );
        let conn = workspace.conn_id.clone();
        allow(&mut harness, tab, AgentAccess::Read);
        let mut saved = harness.app.connections.get(&conn).unwrap().clone();
        saved.agents.tables = tabletist_db::agent::Tables::AllExcept(vec![
            "customers".parse().unwrap(),
            "old_orders".parse().unwrap(),
        ]);
        harness.app.connections.upsert(saved);
        harness.app.check_agents();
        at_agents(&mut harness, &conn);
        // A name in no table of the database is marked, and what that does
        // to agents is said.
        let mark = |name: &str| look.label(&format!("{name}, not in this database"));
        assert!(harness.has(&mark("old_orders")), "{}", look.name);
        assert!(!harness.has(&mark("customers")), "{}", look.name);
        assert!(harness.has(&gone(&look)), "{}", look.name);
        // Typed, the window's tables that match are offered; down picks
        // the first, and Enter adds it.
        type_table(&mut harness, &look, "ord");
        assert!(harness.has("orders"), "{}", look.name);
        harness.press(egui::Key::ArrowDown, egui::Modifiers::NONE);
        harness.press(egui::Key::Enter, egui::Modifiers::NONE);
        harness.settle();
        assert!(harness.has(&look.label("Remove orders")), "{}", look.name);
        // Up and Down move in the list, and not the field's caret.
        type_table(&mut harness, &look, "boo");
        harness.press(egui::Key::ArrowDown, egui::Modifiers::NONE);
        harness.press(egui::Key::ArrowUp, egui::Modifiers::NONE);
        harness.frame(vec![egui::Event::Text("k".into())]);
        harness.settle();
        assert_eq!(
            form(&harness).agents_table_typed.as_deref(),
            Some("book"),
            "{}",
            look.name
        );
        // A press on the list's note is no leaving the field.
        let note = harness
            .painted_rect(&look.label("Tables loaded in the sidebar"))
            .expect("the list's note");
        press_at(&mut harness, note.center());
        assert_eq!(
            form(&harness).agents_table_typed.as_deref(),
            Some("book"),
            "{}",
            look.name
        );
        // Tab with a table chosen adds that table.
        harness.press(egui::Key::ArrowDown, egui::Modifiers::NONE);
        harness.press(egui::Key::Tab, egui::Modifiers::NONE);
        harness.settle();
        assert!(harness.has(&look.label("Remove books")), "{}", look.name);
        // A field left open with a name that does not read lists nothing.
        let note = look.label("Tables loaded in the sidebar");
        type_table(&mut harness, &look, "\"aut");
        assert!(harness.has(&note), "{}", look.name);
        harness.press(egui::Key::Tab, egui::Modifiers::NONE);
        harness.settle();
        assert!(form(&harness).agents_table_error.is_some(), "{}", look.name);
        assert!(!harness.has(&note), "{}", look.name);
        let opened = form_mut(&mut harness);
        opened.agents_table_typed = None;
        opened.agents_table_error = None;
        // A click on a table adds it, and the field is empty for the next.
        type_table(&mut harness, &look, "auth");
        // The list's, painted over the sidebar's.
        let row = harness
            .text_rects
            .iter()
            .rev()
            .find(|(text, _)| text == "authors")
            .map(|(_, rect)| *rect)
            .expect("the list's table");
        press_at(&mut harness, row.center());
        assert!(harness.has(&look.label("Remove authors")), "{}", look.name);
        assert_eq!(
            form(&harness).agents_table_typed.as_deref(),
            Some(""),
            "{}",
            look.name
        );
        // A name typed that is in no list is added as typed.
        add_table(&mut harness, &look, "payroll");
        assert!(harness.has(&mark("payroll")), "{}", look.name);
        harness.press(egui::Key::Escape, egui::Modifiers::NONE);
        harness.settle();
        // Taken out, nothing is marked and nothing said.
        harness.click(&look.label("Remove old_orders"));
        harness.click(&look.label("Remove payroll"));
        harness.click(&look.label("Remove books"));
        harness.settle();
        assert!(!harness.has(&gone(&look)), "{}", look.name);
    }
}

/// `name` typed for Add column… (its field opened where it is not open
/// already), then Enter.
#[cfg(any(unix, windows))]
fn add_column(harness: &mut Harness, look: &Look, name: &str) {
    type_column(harness, look, name);
    harness.press(egui::Key::Enter, egui::Modifiers::NONE);
    harness.settle();
}

/// `name` typed in Add column…'s field, opened where it is not open.
#[cfg(any(unix, windows))]
fn type_column(harness: &mut Harness, look: &Look, name: &str) {
    if form(harness).agents_column_typed.is_none() {
        harness.click(&look.label("Add column…"));
    }
    harness.settle();
    harness.frame(vec![egui::Event::Text(name.into())]);
    harness.settle();
}

/// How many times the backend was asked for the columns of `table`.
#[cfg(any(unix, windows))]
fn asked_to_describe(harness: &Harness, table: &str) -> usize {
    use crate::backend::Command;
    let sent = harness.app.backend.sent.iter();
    sent.filter(
        |command| matches!(command, Command::Describe { object, .. } if object.name == table),
    )
    .count()
}

/// Answers the newest request for the columns of `table`: `columns`.
#[cfg(any(unix, windows))]
fn describe(harness: &mut Harness, table: &str, columns: &[&str]) {
    use crate::backend::Command;
    let sent = harness.app.backend.sent.iter().rev();
    let (session, request) = sent
        .filter_map(|command| match command {
            Command::Describe {
                session,
                request,
                object,
            } if object.name == table => Some((*session, *request)),
            _ => None,
        })
        .next()
        .unwrap_or_else(|| panic!("no Describe of {table} was sent"));
    let structure = tabletist_db::Structure {
        columns: columns
            .iter()
            .map(|name| tabletist_db::ColumnInfo {
                name: (*name).into(),
                type_name: "TEXT".into(),
                ..Default::default()
            })
            .collect(),
        ..Default::default()
    };
    harness.app.apply(Action::Backend(Event::Structure {
        session,
        request,
        result: Ok(structure),
    }));
    harness.settle();
}

#[cfg(any(unix, windows))]
#[test]
fn the_editors_masked_columns_control_masks_what_agents_get_as_dots() {
    let columns = |written: &[&str]| -> Vec<tabletist_db::agent::ColumnName> {
        written.iter().map(|entry| entry.parse().unwrap()).collect()
    };
    let hint = "Agents see the name and type; values come back as •••. A query that filters, \
                joins, sorts or computes with one is refused, in any table with a column of \
                that name.";
    let no_table = "The column's name has no table: write the table, a dot, and the column.";
    let too_many = "The column's name has more than two dots: it is a table and a column, or a \
                    schema, a table and a column.";
    for look in looks() {
        let (mut harness, conn) = on_the_list(look);
        at_agents(&mut harness, &conn);
        assert!(harness.has(&look.label(hint)), "{}", look.name);
        // Enter masks a column and keeps the field, with the keyboard, for
        // the next.
        add_column(&mut harness, &look, "orders.email");
        add_column(&mut harness, &look, "sales.orders.phone");
        assert_eq!(
            form(&harness).agents_column_typed.as_deref(),
            Some(""),
            "{}",
            look.name
        );
        // The same one again, in another case, is there once.
        add_column(&mut harness, &look, "ORDERS.EMAIL");
        // One that does not read is said under the names, and the field
        // keeps it to be put right.
        add_column(&mut harness, &look, "email");
        assert!(harness.has(&look.label(no_table)), "{}", look.name);
        assert_eq!(
            form(&harness).agents_column_typed.as_deref(),
            Some("email"),
            "{}",
            look.name
        );
        // Escape closes the field and nothing else.
        harness.press(egui::Key::Escape, egui::Modifiers::NONE);
        harness.settle();
        assert!(harness.app.dialog.is_some(), "{}", look.name);
        assert_eq!(form(&harness).agents_column_typed, None, "{}", look.name);
        assert!(!harness.has(&look.label(no_table)), "{}", look.name);
        harness.click(&look.label("Remove sales.orders.phone"));
        harness.settle();
        // A name typed and not yet added is added by the save.
        type_column(&mut harness, &look, "orders.phone");
        harness.app.apply(Action::SaveConnection { connect: false });
        let saved = harness.app.connections.get(&conn).unwrap();
        assert_eq!(
            saved.agents.masked,
            columns(&["orders.email", "orders.phone"]),
            "{}",
            look.name
        );
        // The person's own: shown, so that comparing again holds nothing.
        harness.app.check_agents();
        assert!(harness.app.agents_held.is_empty(), "{}", look.name);
        // Opened again it shows them. A name that does not read, typed and
        // not added, refuses the save and says why.
        at_agents(&mut harness, &conn);
        assert!(
            harness.has(&look.label("Remove orders.phone")),
            "{}",
            look.name
        );
        type_column(&mut harness, &look, "a.b.c.d");
        harness.app.apply(Action::SaveConnection { connect: false });
        assert!(harness.app.dialog.is_some(), "{}", look.name);
        assert_eq!(
            form(&harness).message.as_deref(),
            Some(too_many),
            "{}",
            look.name
        );
        form_mut(&mut harness).agents_column_typed = None;
        form_mut(&mut harness).agents_column_error = None;
        harness.settle();
        // A name left in the field when Tab moves on is added.
        type_column(&mut harness, &look, "orders.total");
        harness.press(egui::Key::Tab, egui::Modifiers::NONE);
        harness.settle();
        assert!(
            harness.has(&look.label("Remove orders.total")),
            "{}",
            look.name
        );
        assert_eq!(form(&harness).agents_column_typed, None, "{}", look.name);
        // Masking less, by the person: theirs, and never held.
        for name in ["orders.email", "orders.phone", "orders.total"] {
            harness.click(&look.label(&format!("Remove {name}")));
            harness.settle();
        }
        harness.app.apply(Action::SaveConnection { connect: false });
        assert_eq!(
            harness.app.connections.get(&conn).unwrap().agents.masked,
            columns(&[]),
            "{}",
            look.name
        );
        harness.app.check_agents();
        assert!(harness.app.agents_held.is_empty(), "{}", look.name);
    }
}

#[cfg(any(unix, windows))]
#[test]
fn masked_columns_changed_outside_while_the_editor_is_open_are_not_saved_over() {
    let columns = |written: &[&str]| -> Vec<tabletist_db::agent::ColumnName> {
        written.iter().map(|entry| entry.parse().unwrap()).collect()
    };
    let (mut harness, conn) = on_the_list(Look::macos());
    at_agents(&mut harness, &conn);
    add_column(&mut harness, &Look::macos(), "orders.email");
    // The file masks another column meanwhile.
    let mut saved = harness.app.connections.get(&conn).unwrap().clone();
    saved.agents.masked = columns(&["customers.phone"]);
    harness.app.connections.upsert(saved);
    harness.app.check_agents();
    harness.app.apply(Action::SaveConnection { connect: false });
    assert!(harness.app.dialog.is_some());
    assert_eq!(form(&harness).agents_masked, columns(&["customers.phone"]));
    assert!(
        form(&harness)
            .message
            .as_deref()
            .is_some_and(|said| said.starts_with("The masked columns were changed outside"))
    );
    // Saved again, it keeps what the file says.
    harness.app.apply(Action::SaveConnection { connect: false });
    assert_eq!(
        harness.app.connections.get(&conn).unwrap().agents.masked,
        columns(&["customers.phone"])
    );
}

#[cfg(any(unix, windows))]
#[test]
fn add_column_offers_the_tables_then_the_columns_it_fetches_and_marks_one_in_none() {
    let gone = |look: &Look| {
        look.label(
            "Agents may not use this connection while a masked column is not in its database. \
             Take the name out, or add the column's new name.",
        )
    };
    for look in looks() {
        let (mut harness, tab) = connected(look);
        loaded(&mut harness, tab, &["customers", "orders"]);
        let conn = harness.app.workspace(tab).unwrap().conn_id.clone();
        let mut saved = harness.app.connections.get(&conn).unwrap().clone();
        saved.agents.masked = vec!["orders.mail".parse().unwrap()];
        harness.app.connections.upsert(saved);
        harness.app.check_agents();
        at_agents(&mut harness, &conn);
        // The columns of a table a masked column names are asked of the
        // window, once; a column none of them is is marked, and what that
        // does to agents is said.
        assert_eq!(asked_to_describe(&harness, "orders"), 1, "{}", look.name);
        let mark = look.label("orders.mail, not in this database");
        assert!(!harness.has(&mark), "{}", look.name);
        describe(&mut harness, "orders", &["id", "email", "phone"]);
        assert!(harness.has(&mark), "{}", look.name);
        assert!(harness.has(&gone(&look)), "{}", look.name);
        assert_eq!(asked_to_describe(&harness, "orders"), 1, "{}", look.name);
        // Typed, the window's tables that match are offered; Enter on one
        // puts it in the field, and its columns are offered.
        type_column(&mut harness, &look, "ord");
        assert!(harness.has("orders."), "{}", look.name);
        harness.press(egui::Key::ArrowDown, egui::Modifiers::NONE);
        harness.press(egui::Key::Enter, egui::Modifiers::NONE);
        harness.settle();
        assert_eq!(
            form(&harness).agents_column_typed.as_deref(),
            Some("orders."),
            "{}",
            look.name
        );
        assert!(
            harness.has(&look.label("Columns of orders")),
            "{}",
            look.name
        );
        harness.frame(vec![egui::Event::Text("em".into())]);
        harness.settle();
        harness.press(egui::Key::ArrowDown, egui::Modifiers::NONE);
        harness.press(egui::Key::Enter, egui::Modifiers::NONE);
        harness.settle();
        assert!(
            harness.has(&look.label("Remove orders.email")),
            "{}",
            look.name
        );
        assert_eq!(
            form(&harness).agents_column_typed.as_deref(),
            Some(""),
            "{}",
            look.name
        );
        // A table whose columns no window has: asked for, and the list
        // says they are on their way; a click on one masks it.
        type_column(&mut harness, &look, "customers.");
        assert_eq!(asked_to_describe(&harness, "customers"), 1, "{}", look.name);
        assert!(
            harness.has(&look.label("Loading the columns of customers…")),
            "{}",
            look.name
        );
        describe(&mut harness, "customers", &["id", "name"]);
        let row = harness
            .text_rects
            .iter()
            .rev()
            .find(|(text, _)| text == "customers.name")
            .map(|(_, rect)| *rect)
            .expect("the list's column");
        press_at(&mut harness, row.center());
        assert!(
            harness.has(&look.label("Remove customers.name")),
            "{}",
            look.name
        );
        harness.press(egui::Key::Escape, egui::Modifiers::NONE);
        harness.settle();
        // Taken out, nothing is marked and nothing said.
        harness.click(&look.label("Remove orders.mail"));
        harness.settle();
        assert!(!harness.has(&gone(&look)), "{}", look.name);
    }
}

#[cfg(any(unix, windows))]
#[test]
fn add_columns_list_keeps_the_row_chosen_and_says_what_did_not_come() {
    use crate::model::{Fetch, SessionStatus};
    use tabletist_db::{ObjectInfo, ObjectKind, ObjectRef};
    for look in looks() {
        let (mut harness, tab) = connected(look);
        // `orders` in two schemas: one's columns are here, the other's on
        // their way.
        let workspace = harness.app.workspace_mut(tab).unwrap();
        workspace.tree.schemas.value = Some(vec!["main".into(), "other".into()]);
        for schema in ["main", "other"] {
            let node = workspace.tree.nodes.entry(schema.into()).or_default();
            node.objects.value = Some(vec![ObjectInfo {
                name: "orders".into(),
                kind: ObjectKind::Table,
                estimated_rows: None,
            }]);
        }
        let fetched = Fetch {
            value: Some(
                ["id", "phone"]
                    .iter()
                    .map(|name| tabletist_db::ColumnInfo {
                        name: (*name).into(),
                        ..Default::default()
                    })
                    .collect(),
            ),
            ..Fetch::default()
        };
        workspace
            .columns
            .insert(ObjectRef::new("main", "orders"), fetched);
        let conn = workspace.conn_id.clone();
        at_agents(&mut harness, &conn);
        type_column(&mut harness, &look, "orders.");
        assert_eq!(asked_to_describe(&harness, "orders"), 1, "{}", look.name);
        // `phone` chosen, then the other schema's columns come and put a
        // row before it: Enter still masks `phone`.
        harness.press(egui::Key::ArrowDown, egui::Modifiers::NONE);
        harness.press(egui::Key::ArrowDown, egui::Modifiers::NONE);
        assert_eq!(form(&harness).agents_column_pick, Some(1), "{}", look.name);
        describe(&mut harness, "orders", &["email"]);
        harness.press(egui::Key::Enter, egui::Modifiers::NONE);
        harness.settle();
        assert!(
            harness.has(&look.label("Remove orders.phone")),
            "{}",
            look.name
        );
        // A table whose columns could not be read says so.
        let workspace = harness.app.workspace_mut(tab).unwrap();
        workspace
            .tree
            .nodes
            .entry("main".into())
            .or_default()
            .objects
            .value = Some(vec![ObjectInfo {
            name: "carts".into(),
            kind: ObjectKind::Table,
            estimated_rows: None,
        }]);
        type_column(&mut harness, &look, "carts.");
        let (session, request) = harness
            .app
            .backend
            .sent
            .iter()
            .rev()
            .find_map(|command| match command {
                crate::backend::Command::Describe {
                    session,
                    request,
                    object,
                } if object.name == "carts" => Some((*session, *request)),
                _ => None,
            })
            .expect("a Describe of carts");
        harness.app.apply(Action::Backend(Event::Structure {
            session,
            request,
            result: Err(tabletist_db::Error::Io("gone".into())),
        }));
        harness.settle();
        assert!(
            harness.has(&look.label("The columns of carts could not be read")),
            "{}",
            look.name
        );
        // Columns on their way from a window whose session is gone are on
        // their way no more.
        harness.press(egui::Key::Escape, egui::Modifiers::NONE);
        harness.settle();
        let workspace = harness.app.workspace_mut(tab).unwrap();
        workspace
            .tree
            .nodes
            .entry("main".into())
            .or_default()
            .objects
            .value = Some(vec![ObjectInfo {
            name: "books".into(),
            kind: ObjectKind::Table,
            estimated_rows: None,
        }]);
        type_column(&mut harness, &look, "books.");
        let waiting = look.label("Loading the columns of books…");
        assert!(harness.has(&waiting), "{}", look.name);
        harness.app.workspace_mut(tab).unwrap().status =
            SessionStatus::Disconnected(tabletist_db::Error::ConnectionLost("gone".into()));
        harness.settle();
        assert!(!harness.has(&waiting), "{}", look.name);
    }
}

#[cfg(any(unix, windows))]
#[test]
fn allow_agents_opens_the_editor_with_read_chosen() {
    for look in looks() {
        let (mut harness, tab) = connected(look);
        harness.app.settings.mcp_enabled = true;
        harness.app.apply(Action::ShowAgents(tab));
        harness.settle();
        let conn = harness.app.workspace(tab).unwrap().conn_id.clone();
        if look.terminal {
            super::tests::type_key(&mut harness, egui::Key::E, "e");
        } else {
            harness.click("Allow agents…");
        }
        harness.settle();
        let Some(crate::model::Dialog::Connection(form)) = &harness.app.dialog else {
            panic!("{}: no editor", look.name);
        };
        assert_eq!(form.agents.access, AgentAccess::Read, "{}", look.name);
        // Nothing until Save.
        assert_eq!(
            access_of(&harness, &conn),
            AgentAccess::Off,
            "{}",
            look.name
        );
        harness.app.apply(Action::SaveConnection { connect: false });
        assert_eq!(
            access_of(&harness, &conn),
            AgentAccess::Read,
            "{}",
            look.name
        );
    }
}

#[cfg(any(unix, windows))]
#[test]
fn the_editor_opens_at_its_agents_part() {
    for look in looks() {
        // A short window, and a server's form: the part is under the fold.
        let mut harness = Harness::with_size(egui::vec2(1280.0, 520.0));
        harness.set_look(look);
        let mut shop = crate::testing::fixture_connection();
        let (spec, _) =
            tabletist_db::ConnectSpec::from_url("postgres://demo@localhost:5432/bookshop").unwrap();
        shop.spec = spec;
        let conn = shop.id.clone();
        harness.app.connections.upsert(shop);
        harness.app.apply(Action::EditConnection(conn.clone()));
        harness.settle();
        let heading = if look.terminal {
            look.label("Agents")
        } else {
            "Access through Tabletist (MCP)".to_owned()
        };
        let seen = |harness: &Harness| {
            harness
                .text_rects
                .iter()
                .any(|(text, rect)| *text == heading && rect.bottom() < harness.size.y - 120.0)
        };
        assert!(
            !seen(&harness),
            "{}: in sight without being asked",
            look.name
        );
        harness.app.apply(Action::CloseDialog);
        harness.app.apply(Action::EditAgents { conn, allow: false });
        // A scroll is animated: it lands some frames on.
        for _ in 0..20 {
            harness.settle();
        }
        let shown = harness
            .text_rects
            .iter()
            .rev()
            .find(|(text, _)| *text == heading)
            .map(|(_, rect)| *rect);
        // In sight, above the dialog's footer.
        let shown = shown.expect("the Agents part");
        assert!(
            shown.bottom() < harness.size.y - 120.0,
            "{}: {shown:?}",
            look.name
        );
    }
}

#[cfg(any(unix, windows))]
#[test]
fn the_limits_are_in_sight_when_the_editor_opens_at_them() {
    for (look, size) in [(Look::macos(), 600.0), (Look::omarchy(), 700.0)] {
        let mut harness = Harness::with_size(egui::vec2(1280.0, size));
        harness.set_look(look);
        let mut shop = crate::testing::fixture_connection();
        let (spec, _) =
            tabletist_db::ConnectSpec::from_url("postgres://demo@localhost:5432/bookshop").unwrap();
        shop.spec = spec;
        let conn = shop.id.clone();
        harness.app.connections.upsert(shop);
        harness.app.apply(Action::EditAgents { conn, allow: false });
        for _ in 0..20 {
            harness.settle();
        }
        let rows = look.label("Rows per answer");
        let shown = harness
            .text_rects
            .iter()
            .rev()
            .find(|(text, _)| *text == rows)
            .map(|(_, rect)| *rect);
        let shown = shown.unwrap_or_else(|| panic!("{}: no rows per answer", look.name));
        assert!(
            shown.bottom() < harness.size.y - 60.0,
            "{}: {shown:?}",
            look.name
        );
    }
}

#[cfg(any(unix, windows))]
#[test]
fn an_enter_held_or_shifted_in_the_box_does_nothing_behind_it() {
    use egui::{Key, Modifiers};
    let (mut harness, conn) = on_the_list(Look::omarchy());
    let windows = harness.app.tabs.len();
    super::tests::type_key(&mut harness, Key::A, "a");
    super::tests::type_key(&mut harness, Key::J, "j");
    // Shift+Enter is no answer.
    harness.press(Key::Enter, Modifiers::SHIFT);
    assert_eq!(access_of(&harness, &conn), AgentAccess::Off);
    assert!(harness.app.dialog.is_some());
    // Enter held: it sets the level, and what it repeats connects nothing.
    for _ in 0..4 {
        harness.frame(vec![crate::testing::key(Key::Enter, Modifiers::NONE)]);
    }
    harness.frame(vec![crate::testing::release(Key::Enter, Modifiers::NONE)]);
    harness.settle();
    assert_eq!(access_of(&harness, &conn), AgentAccess::Read);
    assert_eq!(harness.app.tabs.len(), windows);
    assert!(
        !harness
            .app
            .backend
            .sent
            .iter()
            .any(|sent| matches!(sent, crate::backend::Command::Connect { .. }))
    );
}

#[cfg(any(unix, windows))]
#[test]
fn no_allow_agents_is_offered_for_a_connection_saved_no_more() {
    let (mut harness, tab) = connected(Look::omarchy());
    harness.app.settings.mcp_enabled = true;
    harness.app.apply(Action::ShowAgents(tab));
    harness.settle();
    let offered = |harness: &Harness| {
        harness
            .text_rects
            .iter()
            .any(|(text, _)| text.contains("allow agents"))
    };
    assert!(offered(&harness));
    let conn = harness.app.workspace(tab).unwrap().conn_id.clone();
    harness.app.connections.remove(&conn);
    harness.settle();
    assert!(!offered(&harness));
}

#[cfg(any(unix, windows))]
#[test]
fn the_agents_column_and_hint_step_back_where_the_window_has_no_room() {
    // macOS at 1024: Last used stays clear of the row's buttons.
    let mut harness = Harness::with_size(egui::vec2(1024.0, 700.0));
    let mut shop = crate::testing::fixture_connection();
    shop.name = "Shop".into();
    let conn = shop.id.clone();
    harness.app.connections.upsert(shop);
    let tab = harness.app.active_tab_id();
    harness.app.apply(Action::SelectConnection {
        tab,
        conn: Some(conn),
    });
    harness.settle();
    let used = harness.painted_rect("Last used never").expect("last used");
    let edit = harness.painted_rect("Edit").expect("Edit");
    assert!(used.right() < edit.left(), "{used:?} {edit:?}");
    // Omarchy at 720: the footer's hints stay clear of the keyring's note.
    let (mut harness, _) = on_the_list(Look::omarchy());
    harness.size = egui::vec2(720.0, 600.0);
    harness.settle();
    let note = harness.painted_rect("secrets: keyring").expect("the note");
    let hints = harness
        .text_rects
        .iter()
        .filter(|(_, rect)| {
            (rect.center().y - note.center().y).abs() < 2.0 && rect.left() < note.left()
        })
        .map(|(_, rect)| rect.right())
        .fold(0.0_f32, f32::max);
    assert!(hints < note.left(), "{hints} {note:?}");
}
