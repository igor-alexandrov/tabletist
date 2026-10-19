//! Settings › Editor, step 2: what the SQL editor's results offer when the
//! row limit cut a result (Show all) or the timeout stopped a run (Run
//! without timeout), and what each sends.

use std::time::Duration;

use egui::accesskit::Role;
use egui::{Key, Modifiers};
use tabletist_db::{Driver, ScriptEnd, ScriptMode, StatementOutcome};

use crate::backend::{CancelReason, Command};
use crate::model::{Action, ConnTabId, ResultPane, SqlTab, TabId};
use crate::settings::NewTab;
use crate::testing::{
    Harness, cut_outcome, done_outcome, node, rows_outcome, script_outcome, write_outcome,
};
use crate::theme::Look;

const TIMEOUT: CancelReason = CancelReason::Timeout(Duration::from_secs(30));

/// A connection that takes writes and speaks as `driver`, in `look`, with
/// a new SQL editor opened as the connection has it (read-write) that
/// holds `text` and has the keyboard.
fn editor(look: Look, driver: Driver, text: &str) -> (Harness, ConnTabId, TabId) {
    let mut harness = Harness::new();
    harness.set_look(look);
    harness.app.settings.new_tab = NewTab::Connection;
    let tab = harness.connect_fake_as(false);
    harness.app.workspace_mut(tab).unwrap().driver = driver;
    harness.press(Key::T, Modifiers::COMMAND);
    harness.frame(vec![egui::Event::Paste(text.into())]);
    harness.settle();
    let id = harness.app.workspace(tab).unwrap().active_tab.unwrap();
    (harness, tab, id)
}

fn sql_mut(harness: &mut Harness, tab: ConnTabId, id: TabId) -> &mut SqlTab {
    harness
        .app
        .workspace_mut(tab)
        .unwrap()
        .sql_tab_mut(id)
        .unwrap()
}

fn run(harness: &mut Harness) {
    harness.press(Key::Enter, Modifiers::COMMAND);
}

fn run_all(harness: &mut Harness) {
    harness.press(Key::Enter, Modifiers::COMMAND | Modifiers::SHIFT);
}

/// A run as it was sent.
#[derive(Debug, PartialEq)]
struct Sent {
    statements: Vec<String>,
    mode: ScriptMode,
    limit: u32,
    timeout: Option<Duration>,
}

fn last_run(harness: &Harness) -> Sent {
    let mut sent = harness.app.backend.sent.iter().rev();
    sent.find_map(|command| match command {
        Command::RunSql {
            statements,
            mode,
            limit,
            timeout,
            ..
        } => Some(Sent {
            statements: statements.iter().map(|s| s.text.clone()).collect(),
            mode: *mode,
            limit: *limit,
            timeout: *timeout,
        }),
        _ => None,
    })
    .expect("a run was sent")
}

fn runs_sent(harness: &Harness) -> usize {
    let sent = harness.app.backend.sent.iter();
    sent.filter(|command| matches!(command, Command::RunSql { .. }))
        .count()
}

fn show_all(harness: &mut Harness, tab: ConnTabId, id: TabId) {
    harness.app.apply(Action::ShowAllRows { tab, sql_tab: id });
}

fn without_timeout(harness: &mut Harness, tab: ConnTabId, id: TabId) {
    harness
        .app
        .apply(Action::RunWithoutTimeout { tab, sql_tab: id });
}

#[test]
fn show_all_runs_the_cut_statement_alone_with_no_limit_and_the_runs_timeout() {
    let text = "SELECT 1;\nSELECT * FROM users;\nSELECT 3";
    let (mut harness, tab, id) = editor(Look::macos(), Driver::Sqlite, text);
    run_all(&mut harness);
    let outcomes = vec![rows_outcome(1), cut_outcome(1_000), done_outcome(None)];
    harness.answer_sql(Ok(script_outcome(outcomes)), None);
    let before = runs_sent(&harness);
    show_all(&mut harness, tab, id);
    assert_eq!(runs_sent(&harness), before + 1);
    let sent = last_run(&harness);
    assert_eq!(sent.statements.len(), 1);
    assert!(sent.statements[0].contains("FROM users"), "{sent:?}");
    assert_eq!(
        (sent.mode, sent.limit, sent.timeout),
        (
            ScriptMode::ReadOnly,
            u32::MAX,
            Some(Duration::from_secs(30))
        )
    );
    // In flight, it is not sent twice; once whole, there is nothing to show.
    show_all(&mut harness, tab, id);
    assert_eq!(runs_sent(&harness), before + 1);
    harness.answer_sql(Ok(script_outcome(vec![rows_outcome(3)])), None);
    show_all(&mut harness, tab, id);
    assert_eq!(runs_sent(&harness), before + 1);
}

#[test]
fn show_all_sends_nothing_for_a_write_run_or_once_the_text_is_another() {
    // A run sent to write is never sent again.
    let text = "UPDATE users SET email = 'x' WHERE id = 1 RETURNING id";
    let (mut harness, tab, id) = editor(Look::macos(), Driver::Sqlite, text);
    run(&mut harness);
    assert_eq!(last_run(&harness).mode, ScriptMode::Write);
    let outcome = write_outcome(vec![cut_outcome(1_000)], ScriptEnd::Committed);
    harness.answer_sql(Ok(outcome), None);
    let before = runs_sent(&harness);
    show_all(&mut harness, tab, id);
    assert_eq!(runs_sent(&harness), before);
    // A read, once the text is another than the one that ran.
    let (mut harness, tab, id) = editor(Look::macos(), Driver::Sqlite, "SELECT * FROM users");
    run(&mut harness);
    harness.answer_sql(Ok(script_outcome(vec![cut_outcome(1_000)])), None);
    sql_mut(&mut harness, tab, id).text.push(' ');
    let before = runs_sent(&harness);
    show_all(&mut harness, tab, id);
    assert_eq!(runs_sent(&harness), before);
    sql_mut(&mut harness, tab, id).text.pop();
    show_all(&mut harness, tab, id);
    assert_eq!(runs_sent(&harness), before + 1);
}

#[test]
fn run_without_timeout_sends_the_stopped_run_again_with_none() {
    let (mut harness, tab, id) = editor(Look::macos(), Driver::Sqlite, "SELECT 1;\nSELECT 2");
    run_all(&mut harness);
    let stopped = vec![rows_outcome(1), StatementOutcome::Cancelled];
    harness.answer_sql(Ok(script_outcome(stopped.clone())), Some(TIMEOUT));
    let before = runs_sent(&harness);
    without_timeout(&mut harness, tab, id);
    let sent = last_run(&harness);
    assert_eq!(runs_sent(&harness), before + 1);
    assert_eq!(sent.statements.len(), 2);
    assert_eq!(
        (sent.mode, sent.limit, sent.timeout),
        (ScriptMode::ReadOnly, 1_000, None)
    );
    // Cancelled by hand: nothing to send.
    harness.answer_sql(Ok(script_outcome(stopped)), Some(CancelReason::User));
    without_timeout(&mut harness, tab, id);
    assert_eq!(runs_sent(&harness), before + 1);
}

#[test]
fn each_link_lifts_its_own_cap_and_keeps_the_runs_other_one() {
    let (mut harness, tab, id) = editor(Look::macos(), Driver::Sqlite, "SELECT * FROM users");
    run(&mut harness);
    harness.answer_sql(Ok(script_outcome(vec![cut_outcome(1_000)])), None);
    show_all(&mut harness, tab, id);
    // Show all ran into the timeout: run again, it has neither cap.
    let stopped = script_outcome(vec![StatementOutcome::Cancelled]);
    harness.answer_sql(Ok(stopped), Some(TIMEOUT));
    without_timeout(&mut harness, tab, id);
    let sent = last_run(&harness);
    assert_eq!((sent.limit, sent.timeout), (u32::MAX, None));
    // A run of the editor has its caps again.
    harness.answer_sql(Ok(script_outcome(vec![rows_outcome(3)])), None);
    run(&mut harness);
    let sent = last_run(&harness);
    assert_eq!(
        (sent.limit, sent.timeout),
        (1_000, Some(Duration::from_secs(30)))
    );
}

#[test]
fn run_without_timeout_sends_a_write_again_only_where_nothing_was_written() {
    let text = "CREATE TABLE t (id int);\nUPDATE users SET email = 'x' WHERE id = 1";
    let stopped = vec![done_outcome(None), StatementOutcome::Cancelled];
    // MySQL committed the CREATE by itself: nothing is sent again.
    let (mut harness, tab, id) = editor(Look::macos(), Driver::MySql, text);
    run_all(&mut harness);
    assert_eq!(last_run(&harness).mode, ScriptMode::Write);
    let partly = ScriptEnd::Partly { committed: 1 };
    harness.answer_sql(Ok(write_outcome(stopped.clone(), partly)), Some(TIMEOUT));
    let before = runs_sent(&harness);
    without_timeout(&mut harness, tab, id);
    assert_eq!(runs_sent(&harness), before);
    // Rolled back whole: sent again, to write, as the tab's mode has it.
    let (mut harness, tab, id) = editor(Look::macos(), Driver::Postgres, text);
    run_all(&mut harness);
    let outcome = write_outcome(stopped, ScriptEnd::RolledBack);
    harness.answer_sql(Ok(outcome), Some(TIMEOUT));
    without_timeout(&mut harness, tab, id);
    let sent = last_run(&harness);
    assert_eq!((sent.mode, sent.timeout), (ScriptMode::Write, None));
}

/// Answers the connect a switch of database sent.
fn connected(harness: &mut Harness) {
    let crate::backend::Command::Connect {
        session, request, ..
    } = *crate::testing::last_sent(&harness.app)
    else {
        panic!("expected Connect");
    };
    harness
        .app
        .apply(Action::Backend(crate::backend::Event::Connected {
            session,
            request,
            driver: Driver::Sqlite,
            encrypted: false,
            access: crate::testing::asked_access(&harness.app),
        }));
}

#[test]
fn the_links_send_nothing_once_the_tab_is_on_another_database() {
    for timed_out in [false, true] {
        let text = "SELECT * FROM users";
        let (mut harness, tab, id) = editor(Look::macos(), Driver::Sqlite, text);
        run(&mut harness);
        if timed_out {
            let stopped = script_outcome(vec![StatementOutcome::Cancelled]);
            harness.answer_sql(Ok(stopped), Some(TIMEOUT));
        } else {
            harness.answer_sql(Ok(script_outcome(vec![cut_outcome(1_000)])), None);
        }
        // The SQL tab and its last run stay over a switch of database.
        harness.app.apply(Action::SwitchDatabase {
            tab,
            database: "other".into(),
        });
        connected(&mut harness);
        assert!(sql_mut(&mut harness, tab, id).last_run().is_some());
        let before = runs_sent(&harness);
        show_all(&mut harness, tab, id);
        without_timeout(&mut harness, tab, id);
        assert_eq!(runs_sent(&harness), before, "timed out: {timed_out}");
    }
}

fn sql(harness: &Harness, tab: ConnTabId, id: TabId) -> &SqlTab {
    harness.app.workspace(tab).unwrap().sql_tab(id).unwrap()
}

#[test]
fn the_terminal_looks_letters_take_the_links_on_screen() {
    let (mut harness, tab, id) = editor(Look::omarchy(), Driver::Sqlite, "SELECT * FROM users");
    run(&mut harness);
    harness.answer_sql(Ok(script_outcome(vec![cut_outcome(1_000)])), None);
    assert_eq!(sql(&harness, tab, id).pane, ResultPane::Results);
    let before = runs_sent(&harness);
    // In the editor's text a letter is typed; out of it, `t` offers
    // nothing here and `a` shows all.
    harness.press(Key::Escape, Modifiers::NONE);
    harness.press(Key::T, Modifiers::NONE);
    assert_eq!(runs_sent(&harness), before);
    harness.press(Key::A, Modifiers::NONE);
    assert_eq!(runs_sent(&harness), before + 1);
    assert_eq!(last_run(&harness).limit, u32::MAX);
    // Stopped by the timeout, with no rows: Messages, and `t`.
    let stopped = script_outcome(vec![StatementOutcome::Cancelled]);
    harness.answer_sql(Ok(stopped), Some(TIMEOUT));
    assert_eq!(sql(&harness, tab, id).pane, ResultPane::Messages);
    harness.press(Key::T, Modifiers::NONE);
    assert_eq!(runs_sent(&harness), before + 2);
    assert_eq!(last_run(&harness).timeout, None);
}

#[test]
fn the_other_looks_have_no_letters_for_the_links() {
    let (mut harness, tab, id) = editor(Look::macos(), Driver::Sqlite, "SELECT * FROM users");
    run(&mut harness);
    harness.answer_sql(Ok(script_outcome(vec![cut_outcome(1_000)])), None);
    let before = runs_sent(&harness);
    harness.press(Key::Escape, Modifiers::NONE);
    harness.press(Key::A, Modifiers::NONE);
    assert_eq!(runs_sent(&harness), before);
    // The letter was not typed into the text either.
    assert_eq!(sql(&harness, tab, id).text, "SELECT * FROM users");
}

#[test]
fn a_letter_takes_its_link_only_in_the_pane_that_shows_the_link() {
    // Run without timeout is on the Messages; the Results show the rows.
    let (mut harness, tab, id) = editor(Look::omarchy(), Driver::Sqlite, "SELECT 1;\nSELECT 2");
    run_all(&mut harness);
    let stopped = vec![rows_outcome(2), StatementOutcome::Cancelled];
    harness.answer_sql(Ok(script_outcome(stopped)), Some(TIMEOUT));
    assert_eq!(sql(&harness, tab, id).pane, ResultPane::Messages);
    let before = runs_sent(&harness);
    harness.click("Results");
    assert_eq!(sql(&harness, tab, id).pane, ResultPane::Results);
    harness.press(Key::Escape, Modifiers::NONE);
    harness.press(Key::T, Modifiers::NONE);
    assert_eq!(runs_sent(&harness), before);
    assert_eq!(sql(&harness, tab, id).text, "SELECT 1;\nSELECT 2");
    harness.click("Messages");
    harness.press(Key::Escape, Modifiers::NONE);
    harness.press(Key::T, Modifiers::NONE);
    assert_eq!(runs_sent(&harness), before + 1);
    assert_eq!(last_run(&harness).timeout, None);

    // Show all is on the Results; the Messages do not offer it.
    let (mut harness, tab, id) = editor(Look::omarchy(), Driver::Sqlite, "SELECT * FROM users");
    run(&mut harness);
    harness.answer_sql(Ok(script_outcome(vec![cut_outcome(1_000)])), None);
    let before = runs_sent(&harness);
    harness.click("Messages");
    assert_eq!(sql(&harness, tab, id).pane, ResultPane::Messages);
    harness.press(Key::Escape, Modifiers::NONE);
    harness.press(Key::A, Modifiers::NONE);
    assert_eq!(runs_sent(&harness), before);
    assert_eq!(sql(&harness, tab, id).text, "SELECT * FROM users");
    harness.click("Results");
    harness.press(Key::Escape, Modifiers::NONE);
    harness.press(Key::A, Modifiers::NONE);
    assert_eq!(runs_sent(&harness), before + 1);
    assert_eq!(last_run(&harness).limit, u32::MAX);
}

#[test]
fn the_link_stands_on_the_line_of_the_cancel_only() {
    for look in [Look::macos(), Look::omarchy()] {
        let text = "SELECT 1;\nSELECT 2;\nSELECT 3";
        let (mut harness, tab, id) = editor(look, Driver::Sqlite, text);
        run_all(&mut harness);
        let outcomes = vec![rows_outcome(2), StatementOutcome::Cancelled];
        harness.answer_sql(Ok(script_outcome(outcomes)), Some(TIMEOUT));
        // A timeout opens Messages, rows or not. Results shows the rows,
        // and says nothing of the cancel.
        assert_eq!(sql(&harness, tab, id).pane, ResultPane::Messages);
        harness.click("Results");
        let tree = harness.settle();
        assert!(node(&tree, "Run without timeout", Role::Button).is_none());
        harness.click("Messages");
        let tree = harness.settle();
        let line = crate::testing::bounds(&tree, "Line 2: Cancelled after 30 s", Role::Label)
            .unwrap_or_else(|| panic!("the line in {}", look.name));
        let link = crate::testing::bounds(&tree, "Run without timeout", Role::Button)
            .unwrap_or_else(|| panic!("the link in {}", look.name));
        assert!(
            link.center().y >= line.top() && link.center().y <= line.bottom(),
            "{link:?} beside {line:?} in {}",
            look.name
        );
    }
}
