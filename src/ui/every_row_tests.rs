//! The question before a run that changes every row of a table, in both
//! looks: which runs ask, what the question says, and that nothing is
//! sent before Run anyway.

use egui::{Key, Modifiers};

use crate::backend::Command;
use crate::model::{Action, ConnTabId, Dialog, RunMode, TabId};
use crate::settings::{OptionValue, RunScope};
use crate::testing::Harness;
use crate::theme::Look;
use tabletist_db::ScriptMode;

use super::appearance_settings_tests::{settings_saves, shows};

fn looks() -> [Look; 2] {
    [Look::macos(), Look::omarchy()]
}

/// A writable connection in `look`, with a new SQL editor in `mode` that
/// has the keyboard and holds `text`, its cursor at the end.
fn editor(look: Look, mode: RunMode, text: &str) -> (Harness, ConnTabId, TabId) {
    let mut harness = Harness::new();
    harness.set_look(look);
    let tab = harness.connect_fake_as(false);
    harness.press(Key::T, Modifiers::COMMAND);
    let id = harness.app.workspace(tab).unwrap().active_tab.unwrap();
    harness.app.apply(Action::SetSqlMode {
        tab,
        sql_tab: id,
        mode,
    });
    harness.frame(vec![egui::Event::Paste(text.into())]);
    harness.settle();
    (harness, tab, id)
}

/// The runs sent so far: each one's statements and mode.
fn runs(harness: &Harness) -> Vec<(Vec<String>, ScriptMode)> {
    let sent = harness.app.backend.sent.iter();
    sent.filter_map(|command| match command {
        Command::RunSql {
            statements, mode, ..
        } => Some((statements.iter().map(|s| s.text.clone()).collect(), *mode)),
        _ => None,
    })
    .collect()
}

fn asks(harness: &Harness) -> bool {
    matches!(harness.app.dialog, Some(Dialog::ConfirmEveryRow(_)))
}

fn run_key(harness: &mut Harness) {
    harness.press(Key::Enter, Modifiers::COMMAND);
    harness.settle();
}

fn type_key(harness: &mut Harness, key: Key, text: &str) {
    super::tests::type_key(harness, key, text);
}

#[test]
fn an_update_without_where_asks_and_sends_nothing_before_run_anyway() {
    // macOS, and Windows, which asks the same.
    for look in [Look::macos(), Look::standard()] {
        let (mut harness, _, _) = editor(look, RunMode::ReadWrite, "UPDATE users SET a = 1");
        run_key(&mut harness);
        assert!(asks(&harness), "{}", look.name);
        assert!(runs(&harness).is_empty());
        assert!(harness.has("Update every row in users?"));
        // The fixture's statistics say 1,200,000 rows of users.
        let said = "Line 1 has no WHERE. About 1,200,000 rows, by the database's statistics.";
        assert!(harness.has(said), "{}", look.name);
        harness.click("Run anyway");
        harness.settle();
        assert!(!asks(&harness));
        assert_eq!(
            runs(&harness),
            [(vec!["UPDATE users SET a = 1".to_owned()], ScriptMode::Write)]
        );
        // The editor has the keyboard again.
        assert!(harness.ctx.text_edit_focused(), "{}", look.name);
    }
}

#[test]
fn cancel_esc_and_enter_send_nothing_and_give_the_editor_back() {
    let (mut harness, _, _) = editor(Look::macos(), RunMode::ReadWrite, "DELETE FROM orders");
    run_key(&mut harness);
    assert!(harness.has("Delete every row in orders?"));
    assert!(harness.has("Line 1 has no WHERE. About 3 rows, by the database's statistics."));
    harness.click("Cancel");
    harness.settle();
    assert!(!asks(&harness));
    assert!(harness.ctx.text_edit_focused());
    run_key(&mut harness);
    harness.press(Key::Escape, Modifiers::NONE);
    harness.settle();
    assert!(!asks(&harness));
    // Enter with the keyboard on no button cancels: nothing that writes to
    // every row is the default.
    run_key(&mut harness);
    harness.press(Key::Enter, Modifiers::NONE);
    harness.settle();
    assert!(!asks(&harness));
    assert!(runs(&harness).is_empty());
    // A held run key's repeat, with its modifier, answers nothing.
    run_key(&mut harness);
    harness.press(Key::Enter, Modifiers::COMMAND);
    harness.settle();
    assert!(asks(&harness));
    assert!(runs(&harness).is_empty());
}

#[test]
fn a_bounded_write_a_read_and_a_read_only_tab_never_ask() {
    for look in looks() {
        let bounded = "UPDATE users SET a = 1 WHERE id = 1";
        let (mut harness, _, _) = editor(look, RunMode::ReadWrite, bounded);
        run_key(&mut harness);
        assert!(!asks(&harness), "{}", look.name);
        assert_eq!(
            runs(&harness),
            [(vec![bounded.to_owned()], ScriptMode::Write)]
        );
        // A read in Read-write is a read-only run.
        let (mut harness, _, _) = editor(look, RunMode::ReadWrite, "SELECT * FROM users");
        run_key(&mut harness);
        assert!(!asks(&harness), "{}", look.name);
        assert_eq!(runs(&harness)[0].1, ScriptMode::ReadOnly);
        // On a tab in Read-only the run is read-only: the database refuses
        // the write, and the card offers to run it again.
        let (mut harness, _, _) = editor(look, RunMode::ReadOnly, "DELETE FROM users");
        run_key(&mut harness);
        assert!(!asks(&harness), "{}", look.name);
        assert_eq!(runs(&harness)[0].1, ScriptMode::ReadOnly);
    }
}

#[test]
fn a_table_the_sidebar_does_not_know_is_asked_about_without_rows() {
    let (mut harness, _, _) = editor(Look::macos(), RunMode::ReadWrite, "UPDATE drafts SET a = 1");
    run_key(&mut harness);
    assert!(harness.has("Update every row in drafts?"));
    assert!(harness.has("Line 1 has no WHERE."));
}

#[test]
fn several_statements_are_asked_about_in_one_question() {
    let text = "UPDATE users SET a = 1;\nSELECT 1;\nDELETE FROM orders";
    let (mut harness, _, _) = editor(Look::macos(), RunMode::ReadWrite, text);
    harness
        .app
        .apply(Action::SetOption(OptionValue::RunScope(RunScope::All)));
    run_key(&mut harness);
    assert!(harness.has("Run 2 statements that change every row?"));
    assert!(harness.has("Line 1: Update every row in users, about 1,200,000 rows"));
    assert!(harness.has("Line 3: Delete every row in orders, about 3 rows"));
    harness.click("Run anyway");
    harness.settle();
    assert_eq!(runs(&harness)[0].0.len(), 3);
}

#[test]
fn off_the_setting_lets_update_and_delete_run_and_truncate_still_asks() {
    for text in ["DELETE FROM users", "UPDATE users SET a = 1"] {
        let (mut harness, _, _) = editor(Look::macos(), RunMode::ReadWrite, text);
        harness
            .app
            .apply(Action::SetOption(OptionValue::ConfirmUnsafeWrites(false)));
        run_key(&mut harness);
        assert!(!asks(&harness), "{text}");
        assert_eq!(runs(&harness).len(), 1, "{text}");
    }
    // SQLite has no TRUNCATE; PostgreSQL does.
    let text = "TRUNCATE users, orders";
    let (mut harness, tab, _) = editor(Look::macos(), RunMode::ReadWrite, text);
    harness
        .app
        .apply(Action::SetOption(OptionValue::ConfirmUnsafeWrites(false)));
    harness.app.workspace_mut(tab).unwrap().driver = tabletist_db::Driver::Postgres;
    run_key(&mut harness);
    assert!(harness.has("Truncate users and orders?"));
    assert!(
        harness
            .has("Line 1 takes out every row. About 1,200,003 rows, by the database's statistics.")
    );
}

#[test]
fn the_terminal_asks_in_the_status_line_and_takes_its_letters() {
    let text = "update users set a = 1";
    let (mut harness, tab, id) = editor(Look::omarchy(), RunMode::ReadWrite, text);
    run_key(&mut harness);
    assert!(asks(&harness));
    // In the status line's place, at the window's foot.
    let asked = harness
        .painted_rect("update every row in users?")
        .expect("the question");
    let line = harness
        .ctx
        .data(|data| data.get_temp::<egui::Rect>(super::every_row_prompt::status_line_id(tab)))
        .expect("the status line");
    assert!(line.contains(asked.center()), "{asked:?} in {line:?}");
    assert!(harness.has("update every row in users? about 1,200,000 rows · line 1"));
    assert!(shows(&mut harness, "[y] yes"));
    assert!(shows(&mut harness, "[N] no"));
    // `n` answers no, and types nothing into the editor.
    type_key(&mut harness, Key::N, "n");
    harness.settle();
    assert!(!asks(&harness));
    let sql = harness.app.workspace(tab).unwrap().sql_tab(id).unwrap();
    assert_eq!(sql.text, text);
    // Enter and Esc answer no too.
    for key in [Key::Enter, Key::Escape] {
        run_key(&mut harness);
        assert!(asks(&harness));
        harness.press(key, Modifiers::NONE);
        harness.settle();
        assert!(!asks(&harness), "{key:?}");
    }
    assert!(runs(&harness).is_empty());
    // `N`, as the hint writes it, too.
    run_key(&mut harness);
    type_key(&mut harness, Key::N, "N");
    harness.settle();
    assert!(!asks(&harness));
    assert!(runs(&harness).is_empty());
    // `y` runs it.
    run_key(&mut harness);
    type_key(&mut harness, Key::Y, "y");
    harness.settle();
    assert!(!asks(&harness));
    assert_eq!(runs(&harness), [(vec![text.to_owned()], ScriptMode::Write)]);
    let sql = harness.app.workspace(tab).unwrap().sql_tab(id).unwrap();
    assert_eq!(sql.text, text);
}

#[test]
fn the_keyboard_goes_back_only_where_it_was() {
    // Out of the text in the terminal look, the run key still runs; the
    // answer leaves the keyboard out of the text.
    let (mut harness, tab, id) = editor(Look::omarchy(), RunMode::ReadWrite, "delete from users");
    harness.press(Key::Escape, Modifiers::NONE);
    harness.settle();
    assert!(!harness.ctx.text_edit_focused());
    run_key(&mut harness);
    assert!(asks(&harness));
    type_key(&mut harness, Key::N, "n");
    harness.settle();
    assert!(!asks(&harness));
    assert!(!harness.ctx.text_edit_focused());
    // In the text, it goes back into the text.
    let workspace = harness.app.workspace_mut(tab).unwrap();
    workspace.sql_tab_mut(id).unwrap().focus_editor = true;
    harness.settle();
    assert!(harness.ctx.text_edit_focused());
    run_key(&mut harness);
    type_key(&mut harness, Key::N, "n");
    harness.settle();
    assert!(harness.ctx.text_edit_focused());
}

#[test]
fn a_run_of_the_refused_write_card_asks_too() {
    // Run read-only, then again read-write from the card: the run that
    // writes is the one asked about.
    let (mut harness, tab, id) = editor(Look::macos(), RunMode::ReadOnly, "DELETE FROM users");
    run_key(&mut harness);
    assert_eq!(runs(&harness).len(), 1);
    let refused = crate::testing::script_outcome(vec![crate::testing::refused_write()]);
    harness.answer_sql(Ok(refused), None);
    harness.app.apply(Action::SetSqlMode {
        tab,
        sql_tab: id,
        mode: RunMode::ReadWrite,
    });
    harness.app.apply(Action::RunSqlAgain { tab, sql_tab: id });
    harness.settle();
    assert!(asks(&harness));
    assert_eq!(runs(&harness).len(), 1);
}

#[test]
fn run_without_timeout_asks_again() {
    let (mut harness, tab, id) = editor(Look::macos(), RunMode::ReadWrite, "DELETE FROM users");
    run_key(&mut harness);
    harness.click("Run anyway");
    harness.settle();
    // The timeout stopped it, and nothing was written.
    let stopped = crate::testing::script_outcome(vec![tabletist_db::StatementOutcome::Cancelled]);
    let timeout = crate::backend::CancelReason::Timeout(std::time::Duration::from_secs(30));
    harness.answer_sql(Ok(stopped), Some(timeout));
    harness.settle();
    harness
        .app
        .apply(Action::RunWithoutTimeout { tab, sql_tab: id });
    harness.settle();
    assert!(asks(&harness));
    assert_eq!(runs(&harness).len(), 1);
}

#[test]
fn nothing_else_runs_under_the_question_and_a_switched_database_sends_nothing() {
    let (mut harness, tab, _) = editor(Look::macos(), RunMode::ReadWrite, "DELETE FROM users");
    let other = harness.add_sql_tab(tab);
    run_key(&mut harness);
    assert!(asks(&harness));
    // Another editor's run, asked for under the question: not sent.
    harness.app.apply(Action::RunSql {
        tab,
        sql_tab: other,
        all: true,
    });
    assert!(runs(&harness).is_empty());
    // The tab went to another database meanwhile: Run anyway sends nothing
    // there.
    let workspace = harness.app.workspace_mut(tab).unwrap();
    workspace.spec = tabletist_db::ConnectSpec::sqlite("/tmp/another.db");
    harness.app.apply(Action::RunEveryRow { refocus: false });
    assert!(!asks(&harness));
    assert!(runs(&harness).is_empty());
}

#[test]
fn a_postgres_name_is_looked_up_as_postgres_folds_it() {
    let counted = "Line 1 has no WHERE. About 1,200,000 rows, by the database's statistics.";
    for (text, said) in [
        ("DELETE FROM USERS", counted),
        // Quoted, it is that name exactly, which the sidebar does not have.
        (r#"DELETE FROM "USERS""#, "Line 1 has no WHERE."),
    ] {
        let (mut harness, tab, _) = editor(Look::macos(), RunMode::ReadWrite, text);
        harness.app.workspace_mut(tab).unwrap().driver = tabletist_db::Driver::Postgres;
        run_key(&mut harness);
        assert!(harness.has(said), "{text}");
    }
}

/// Settings on its Editor pane.
fn on_the_pane(harness: &mut Harness) {
    harness.app.apply(Action::ShowSettings);
    harness
        .app
        .apply(Action::ShowSettingsPane(crate::model::SettingsPane::Editor));
    harness.settle();
}

#[test]
fn the_sheet_has_the_confirm_row_last_in_running_queries_and_sets_it() {
    let mut harness = Harness::with_size(egui::vec2(1440.0, 1400.0));
    harness.set_look(Look::macos());
    on_the_pane(&mut harness);
    let top = |harness: &Harness, words: &str| harness.painted_rect(words).expect(words).top();
    let row = "Confirm UPDATE or DELETE without WHERE";
    assert!(top(&harness, "Statement at cursor") < top(&harness, row));
    assert!(top(&harness, row) < top(&harness, "Indent"));
    assert!(harness.has("On every connection that allows writes"));
    let saves = settings_saves(&harness);
    harness.click(row);
    harness.settle();
    assert!(!harness.app.settings.confirm_unsafe_writes);
    assert_eq!(settings_saves(&harness), saves + 1);
    assert!(
        harness
            .app
            .settings_file
            .text
            .contains("confirm_unsafe_writes = false")
    );
}

#[test]
fn the_screen_has_the_confirm_row_after_the_run_key_and_toggles_it() {
    let mut harness = Harness::with_size(egui::vec2(1920.0, 1080.0));
    harness.set_look(Look::omarchy());
    on_the_pane(&mut harness);
    let top = |harness: &Harness, words: &str| harness.painted_rect(words).expect(words).top();
    let row = "confirm write without where";
    // After the run key's row, by its hint.
    assert!(top(&harness, "a selection always wins") < top(&harness, row));
    assert!(top(&harness, row) < top(&harness, "line numbers"));
    assert!(shows(&mut harness, "connections that allow writes"));
    // Down to it from the first row: new tabs, limit, timeout, run key.
    for _ in 0..4 {
        type_key(&mut harness, Key::J, "j");
    }
    harness.press(Key::Space, Modifiers::NONE);
    harness.settle();
    assert!(!harness.app.settings.confirm_unsafe_writes);
}
