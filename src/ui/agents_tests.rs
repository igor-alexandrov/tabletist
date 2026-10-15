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
                connection,
                ..
            }) => Some((*asked, connection.clone())),
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
            harness.click("All connections");
            harness.settle();
        }
        assert_eq!(log_asked(&harness).1, None, "{}", look.name);
    }
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
