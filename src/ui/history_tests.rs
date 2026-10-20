//! The History tab, headless: what each source records and what it does
//! not, what the tab shows and offers for an entry, its filters, its keys
//! and the ways in. Only the Bookshop's names and data; nothing is drawn
//! to a picture.

use egui::accesskit::Role;
use egui::{Key, Modifiers};

use crate::backend::{AgentsEvent, Command, Event, HistoryCommand, HistoryEvent};
use crate::history::{Entry, Kind, Outcome, Source};
use crate::model::{Action, ConnTabId, FilterRow, HistoryFilterSet, RunMode, Workspace};
use crate::testing::{Harness, rows_outcome, script_outcome};
use crate::theme::Look;

/// A control's name with its keys, as the keymap writes them for `look`:
/// "History (⌘Y)".
fn keyed(harness: &Harness, look: Look, name: &str, command: crate::keymap::Command) -> String {
    let keys = crate::ui::keys::written(&harness.ctx, &look, command);
    format!("{name} ({})", keys.as_str())
}

fn looks() -> [Look; 2] {
    [Look::macos(), Look::omarchy()]
}

/// The Bookshop's `book_covers`, writable, open in a window named as the
/// Bookshop's, in `look`.
fn covers(look: Look) -> (Harness, ConnTabId, crate::model::TabId) {
    let mut harness = Harness::new();
    harness.set_look(look);
    let (tab, id) = harness.book_covers();
    bookshop(&mut harness, tab);
    harness.settle();
    (harness, tab, id)
}

/// Names the window's connection as the Bookshop's dev database.
fn bookshop(harness: &mut Harness, tab: ConnTabId) {
    let workspace = harness.app.workspace_mut(tab).unwrap();
    workspace.name = "Bookshop".into();
    workspace.environment = crate::env::Environment::Dev;
}

fn conn(harness: &Harness, tab: ConnTabId) -> String {
    harness.app.workspace(tab).unwrap().conn_id.0.clone()
}

/// The newest entry the history was asked to write.
fn last_written(harness: &Harness) -> Entry {
    harness
        .history_written()
        .pop()
        .expect("an entry was written")
}

/// Opens a SQL editor, types `text` and runs all of it.
fn run(harness: &mut Harness, tab: ConnTabId, text: &str) -> crate::model::TabId {
    harness.app.apply(Action::NewSqlTab(tab));
    let id = harness.app.workspace(tab).unwrap().active_tab.unwrap();
    let workspace = harness.app.workspace_mut(tab).unwrap();
    let sql = workspace
        .tabs
        .iter_mut()
        .filter_map(|open| open.as_sql_mut())
        .find(|sql| sql.id == id)
        .unwrap();
    sql.text = text.into();
    harness.app.apply(Action::RunSql {
        tab,
        sql_tab: id,
        all: true,
    });
    id
}

/// An agent's call of `run_query` on `tab`'s connection, the log's call
/// `logged`.
fn call(harness: &Harness, tab: ConnTabId, logged: i64) -> tabletist_mcp::Call {
    tabletist_mcp::Call {
        at: std::time::SystemTime::now(),
        session: "session of Claude Code".into(),
        agent: "Claude Code".into(),
        connection: Some(conn(harness, tab)),
        tool: "run_query".into(),
        statement: true,
        logged: Some(logged),
        request: Some("SELECT name FROM publishers WHERE country = 'US'".into()),
        result: "ok".into(),
        message: None,
        rows: Some(14),
        columns: Some(1),
        took: std::time::Duration::from_millis(12),
    }
}

/// An entry of `tab`'s connection, as the file would hand it back.
fn entry(harness: &Harness, tab: ConnTabId, source: Source, sql: Option<&str>) -> Entry {
    Entry {
        at_ms: now_ms(),
        source,
        detail: match source {
            Source::Sql => "Query 1",
            Source::Save | Source::Filter => "book_covers",
            // An agent's name is the Agents Log's: the history keeps none.
            Source::Agent => "",
        }
        .into(),
        tool: (source == Source::Agent).then(|| "run_query".into()),
        connection: conn(harness, tab),
        database: "bookshop_development".into(),
        sql: sql.map(str::to_owned),
        call: (source == Source::Agent).then_some(7),
        kind: match source {
            Source::Save => Kind::Write,
            Source::Sql | Source::Filter | Source::Agent => Kind::Read,
        },
        read_only: true,
        rows: Some(6),
        took_ms: Some(38),
        outcome: match source {
            Source::Save => Outcome::Saved,
            Source::Sql | Source::Filter | Source::Agent => Outcome::Ok,
        },
        code: None,
        message: None,
        query: None,
    }
}

/// Milliseconds of Unix time now.
fn now_ms() -> i64 {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap();
    i64::try_from(now.as_millis()).unwrap()
}

/// Opens the editor on `cell`, sets its text as a field would, and commits.
fn type_cell(
    harness: &mut Harness,
    tab: ConnTabId,
    id: crate::model::TabId,
    cell: crate::model::CellPos,
    text: &str,
) {
    harness.app.apply(Action::EditCell {
        tab,
        id,
        cell,
        start: crate::model::EditStart::Value,
    });
    let object = harness
        .app
        .workspace_mut(tab)
        .unwrap()
        .object_tab_mut(id)
        .unwrap();
    object.edits.editor.as_mut().expect("an editor").text = text.to_owned();
    harness.app.apply(Action::EditorTyped { tab, id });
    harness.app.apply(Action::CommitEdit {
        tab,
        id,
        then: crate::model::Advance::Stay,
    });
}

const READ: &str = "SELECT name, country FROM publishers ORDER BY name";
const SAVE: &str =
    "BEGIN;\nUPDATE \"main\".\"book_covers\" SET \"kind\" = 'ebook' WHERE \"id\" = 2;\nCOMMIT;";

/// The History tab of `tab` shown, holding `entries`.
fn shown_with(harness: &mut Harness, tab: ConnTabId, entries: Vec<Entry>) {
    harness.app.apply(Action::ShowHistory(tab));
    harness.settle();
    harness.answer_history(entries);
}

/// Answers the newest look into the Agents Log with the calls it holds.
fn held(harness: &mut Harness, held: Vec<(i64, String)>) {
    let asked = harness
        .app
        .backend
        .sent
        .iter()
        .rev()
        .find_map(|command| match command {
            Command::History(HistoryCommand::Held { asked, .. }) => Some(*asked),
            _ => None,
        })
        .expect("the Agents Log is asked");
    harness
        .app
        .apply(Action::Backend(Event::History(HistoryEvent::Held {
            asked,
            held: Ok(held),
        })));
    harness.settle();
}

fn history_tabs(harness: &Harness, tab: ConnTabId) -> usize {
    let workspace = harness.app.workspace(tab).unwrap();
    workspace
        .tabs
        .iter()
        .filter(|open| matches!(open, crate::model::Tab::History(_)))
        .count()
}

fn front(harness: &Harness, tab: ConnTabId) -> bool {
    harness
        .app
        .workspace(tab)
        .is_some_and(Workspace::history_in_front)
}

fn key_of(harness: &Harness, tab: ConnTabId, at: usize) -> u64 {
    harness
        .app
        .workspace(tab)
        .unwrap()
        .history_tab()
        .unwrap()
        .entries[at]
        .key
}

fn select(harness: &mut Harness, tab: ConnTabId, at: usize) {
    let key = key_of(harness, tab, at);
    harness.app.apply(Action::SelectHistoryEntry { tab, key });
    harness.settle();
}

fn sql_in_front(harness: &Harness, tab: ConnTabId) -> Option<&crate::model::SqlTab> {
    harness.app.workspace(tab).unwrap().active_sql_tab()
}

fn runs_sent(harness: &Harness) -> usize {
    harness
        .app
        .backend
        .sent
        .iter()
        .filter(|command| matches!(command, Command::RunSql { .. }))
        .count()
}

#[test]
fn each_source_adds_one_entry_with_its_source_and_detail() {
    let (mut harness, tab, id) = covers(Look::macos());
    // The SQL editor: the statement as it ran, by its tab's name.
    run(&mut harness, tab, READ);
    harness.answer_sql(Ok(script_outcome(vec![rows_outcome(6)])), None);
    let sql = last_written(&harness);
    assert_eq!(
        (sql.source, sql.detail.as_str(), sql.sql.as_deref()),
        (Source::Sql, "Query 1", Some(READ))
    );
    assert_eq!(
        (sql.kind, sql.rows, sql.outcome),
        (Kind::Read, Some(6), Outcome::Ok)
    );
    assert_eq!(sql.connection, conn(&harness, tab));
    assert!(sql.read_only, "a new tab runs read-only by default");
    // A grid's save: its whole transaction, its values written in.
    harness.app.apply(Action::ActivateTab { tab, id });
    let cell = crate::model::CellPos { row: 1, col: 2 };
    type_cell(&mut harness, tab, id, cell, "audio");
    harness.app.apply(Action::WriteEdits { tab, id });
    harness.answer_written(Ok(tabletist_db::WriteOutcome::Written {
        inserted: Vec::new(),
        rows: vec![crate::testing::book_covers_page(3).rows[1].clone()],
        gone: Vec::new(),
        elapsed: std::time::Duration::from_millis(9),
    }));
    let save = last_written(&harness);
    assert_eq!(
        (save.source, save.detail.as_str()),
        (Source::Save, "book_covers")
    );
    let text = save.sql.clone().unwrap();
    assert!(text.starts_with("BEGIN;\nUPDATE "), "{text}");
    assert!(text.contains("'audio'"), "the value typed: {text}");
    assert!(text.ends_with("\nCOMMIT;"), "{text}");
    assert_eq!(
        (save.kind, save.rows, save.outcome, save.took_ms),
        (Kind::Write, Some(1), Outcome::Saved, Some(9))
    );
    // A filter: the SELECT it ran, and the filter and sort to set again.
    harness.answer_rows(crate::testing::book_covers_page(3));
    let object = harness
        .app
        .workspace_mut(tab)
        .unwrap()
        .object_tab_mut(id)
        .unwrap();
    object.filter.rows = vec![FilterRow {
        column: "kind".into(),
        op: tabletist_db::FilterOp::Eq,
        value: "print".into(),
    }];
    harness.app.apply(Action::ApplyFilters {
        tab,
        object_tab: id,
    });
    harness.answer_rows(crate::testing::book_covers_page(2));
    let filter = last_written(&harness);
    assert_eq!(
        (filter.source, filter.detail.as_str()),
        (Source::Filter, "book_covers")
    );
    let text = filter.sql.clone().unwrap();
    assert!(
        text.starts_with(r#"SELECT * FROM "main"."book_covers" WHERE "kind" = 'print'"#),
        "{text}"
    );
    assert_eq!(filter.rows, Some(2));
    let kept = filter.query.expect("the filter, to set again");
    assert_eq!(
        kept.filter.rows,
        [("kind".into(), "eq".into(), "print".into())]
    );
    // An agent's run_query: the call, by its place in the Agents Log.
    let before = harness.history_written().len();
    harness
        .app
        .apply(Action::Backend(Event::Agents(AgentsEvent::Called(call(
            &harness, tab, 41,
        )))));
    assert_eq!(harness.history_written().len(), before + 1);
    let agent = last_written(&harness);
    // No name: the client's name is what it sent, the Agents Log's alone.
    assert_eq!(
        (agent.source, agent.detail.as_str(), agent.tool.as_deref()),
        (Source::Agent, "", Some("run_query"))
    );
    assert!(!format!("{agent:?}").contains("Claude"));
    assert_eq!(
        (agent.call, agent.rows, agent.took_ms),
        (Some(41), Some(14), Some(12))
    );
}

#[test]
fn a_whole_tab_of_three_adds_three_and_a_save_of_two_updates_adds_one() {
    let (mut harness, tab, id) = covers(Look::macos());
    run(
        &mut harness,
        tab,
        "SELECT * FROM publishers;\nSELECT count(*) FROM book_covers;\nSELECT 1;",
    );
    harness.answer_sql(
        Ok(script_outcome(vec![
            rows_outcome(6),
            rows_outcome(1),
            rows_outcome(1),
        ])),
        None,
    );
    let written: Vec<String> = harness
        .history_written()
        .into_iter()
        .filter_map(|entry| entry.sql)
        .collect();
    assert_eq!(
        written,
        [
            "SELECT * FROM publishers",
            "SELECT count(*) FROM book_covers",
            "SELECT 1"
        ]
    );
    // Two rows changed: one entry, the whole transaction.
    harness.app.apply(Action::ActivateTab { tab, id });
    for row in [0, 2] {
        let cell = crate::model::CellPos { row, col: 2 };
        type_cell(&mut harness, tab, id, cell, "audio");
    }
    let before = harness.history_written().len();
    harness.app.apply(Action::WriteEdits { tab, id });
    let page = crate::testing::book_covers_page(3).rows;
    harness.answer_written(Ok(tabletist_db::WriteOutcome::Written {
        inserted: Vec::new(),
        rows: vec![page[0].clone(), page[2].clone()],
        gone: Vec::new(),
        elapsed: std::time::Duration::from_millis(9),
    }));
    let written = harness.history_written();
    assert_eq!(written.len(), before + 1);
    let save = written.last().unwrap();
    assert_eq!(save.sql.as_deref().unwrap().matches("UPDATE ").count(), 2);
    assert_eq!(save.rows, Some(2));
}

#[test]
fn catalog_queries_and_plain_table_loads_add_nothing() {
    let (mut harness, tab, id) = covers(Look::macos());
    // The connect, the tree, a table's structure and its rows are in.
    harness.app.apply(Action::Refresh(tab));
    harness.answer_rows(crate::testing::book_covers_page(3));
    harness.app.apply(Action::ActivateTab { tab, id });
    harness.app.apply(Action::RefreshTree(tab));
    harness.settle();
    assert!(harness.history_written().is_empty());
    assert!(
        !harness
            .app
            .backend
            .sent
            .iter()
            .any(|command| matches!(command, Command::History(_))),
        "nothing goes to the history's file"
    );
    // A sort that is cleared again leaves a plain load.
    harness.app.apply(Action::SortBy {
        tab,
        object_tab: id,
        column: "kind".into(),
    });
    harness.answer_rows(crate::testing::book_covers_page(3));
    assert_eq!(harness.history_written().len(), 1, "a sort you set is kept");
    harness.app.apply(Action::ClearSort {
        tab,
        object_tab: id,
    });
    harness.answer_rows(crate::testing::book_covers_page(3));
    assert_eq!(
        harness.history_written().len(),
        1,
        "the table as it is, is not"
    );
}

#[test]
fn an_agents_entry_holds_no_statement_and_links_to_its_call() {
    for look in looks() {
        let (mut harness, tab, _) = covers(look);
        harness.app.settings.mcp_enabled = true;
        let call = call(&harness, tab, 41);
        harness
            .app
            .apply(Action::Backend(Event::Agents(AgentsEvent::Called(call))));
        let written = last_written(&harness);
        // What the agent sent is the Agents Log's alone.
        assert_eq!((written.sql.clone(), written.message.clone()), (None, None));
        assert!(!written.searched().contains("publishers"));
        assert!(!format!("{written:?}").contains("publishers"));
        // The tab, read from the file, shows the entry and links to it.
        shown_with(&mut harness, tab, vec![written.clone()]);
        select(&mut harness, tab, 0);
        assert!(
            !harness.has(&look.label("No longer in the Agents Log")),
            "{}",
            look.name
        );
        // Show in Agents Log, and on macOS its key.
        match look.terminal {
            true => {
                harness.app.workspace_mut(tab).unwrap().pane = crate::model::Pane::Grid;
                super::tests::type_key(&mut harness, Key::A, "a");
            }
            false => harness.press(Key::A, Modifiers::COMMAND | Modifiers::ALT),
        }
        let view = harness
            .app
            .workspace(tab)
            .unwrap()
            .tabs
            .iter()
            .find_map(|open| match open {
                crate::model::Tab::Agents(agents) => Some(agents.view),
                _ => None,
            });
        assert_eq!(view, Some(crate::model::AgentsView::Log), "{}", look.name);
        assert!(harness.app.workspace(tab).unwrap().agents_in_front());
        // The log's page holds the call: it is selected.
        let asked = harness
            .app
            .backend
            .sent
            .iter()
            .rev()
            .find_map(|command| match command {
                Command::Agents(crate::backend::AgentsCommand::ReadLog { asked, .. }) => {
                    Some(*asked)
                }
                _ => None,
            })
            .expect("the log is read");
        let logged = tabletist_mcp::Logged {
            id: 41,
            entry: tabletist_mcp::Entry {
                at_ms: written.at_ms,
                agent: "Claude Code".into(),
                tool: "run_query".into(),
                result: "ok".into(),
                connection_id: Some(conn(&harness, tab)),
                ..tabletist_mcp::Entry::default()
            },
        };
        let page = tabletist_mcp::Page {
            calls: vec![logged],
            total: 1,
            more: false,
            agents: vec!["Claude Code".into()],
        };
        harness
            .app
            .apply(Action::Backend(Event::Agents(AgentsEvent::LogRead {
                asked,
                page: Ok(page),
            })));
        let selected = harness
            .app
            .workspace(tab)
            .unwrap()
            .tabs
            .iter()
            .find_map(|open| match open {
                crate::model::Tab::Agents(agents) => Some(agents.log.selected),
                _ => None,
            })
            .flatten();
        assert_eq!(selected, Some(41), "{}", look.name);
        // Once the log has dropped the call, the entry says so.
        harness.app.apply(Action::ShowHistory(tab));
        harness.answer_history(vec![written.clone()]);
        let asked = harness
            .app
            .backend
            .sent
            .iter()
            .rev()
            .find_map(|command| match command {
                Command::History(HistoryCommand::Held { asked, calls, .. }) => {
                    assert_eq!(calls, &[(41, written.at_ms)], "by its place and time");
                    Some(*asked)
                }
                _ => None,
            })
            .expect("the Agents Log is asked for the call");
        harness
            .app
            .apply(Action::Backend(Event::History(HistoryEvent::Held {
                asked,
                held: Ok(Vec::new()),
            })));
        select(&mut harness, tab, 0);
        assert!(
            harness.has(&look.label("No longer in the Agents Log")),
            "{}",
            look.name
        );
        let key = key_of(&harness, tab, 0);
        harness.app.apply(Action::ShowHistoryCall { tab, key });
        assert!(front(&harness, tab), "a call that is gone shows nothing");
    }
}

#[test]
fn no_entry_keeps_a_result_row() {
    let (mut harness, tab, _) = covers(Look::macos());
    run(&mut harness, tab, "SELECT name FROM publishers");
    let row = tabletist_db::StatementOutcome::Rows {
        columns: vec![tabletist_db::ColumnMeta {
            name: "name".into(),
            type_name: "TEXT".into(),
            kind: tabletist_db::ValueKind::Text,
        }],
        rows: vec![vec![tabletist_db::Value::Text("Harbor Lights".into())]],
        truncated: false,
    };
    harness.answer_sql(Ok(script_outcome(vec![row])), None);
    let written = last_written(&harness);
    assert_eq!(written.rows, Some(1), "how many, never which");
    for field in [
        written.sql.clone().unwrap_or_default(),
        written.detail.clone(),
        written.message.clone().unwrap_or_default(),
        written.database.clone(),
    ] {
        assert!(!field.contains("Harbor Lights"), "{field}");
    }
}

#[test]
fn run_again_is_for_reads_in_a_new_tab_with_the_editor_settings() {
    for look in looks() {
        let (mut harness, tab, _) = covers(look);
        harness.app.settings.sql_limit = 500;
        harness.app.settings.sql_timeout_secs = Some(15);
        let read = entry(&harness, tab, Source::Sql, Some(READ));
        let save = entry(&harness, tab, Source::Save, Some(SAVE));
        shown_with(&mut harness, tab, vec![read, save]);
        if !look.terminal {
            select(&mut harness, tab, 0);
            assert!(harness.has("Run again"), "a read offers it");
            select(&mut harness, tab, 1);
            assert!(!harness.has("Run again"), "a save does not");
            assert!(harness.has("Open in SQL editor"));
        }
        // A write is never run: the terminal's line says how it opens.
        let before = runs_sent(&harness);
        let key = key_of(&harness, tab, 1);
        harness.app.apply(Action::RunHistoryAgain { tab, key });
        assert_eq!(runs_sent(&harness), before, "{}", look.name);
        assert!(front(&harness, tab), "{}", look.name);
        let said = harness
            .app
            .workspace(tab)
            .unwrap()
            .history_tab()
            .unwrap()
            .said
            .clone();
        let words = match look.terminal {
            true => "writes open with enter, into a read-only tab",
            false => "Writes open in a SQL editor, into a read-only tab",
        };
        assert_eq!(said.as_deref(), Some(words));
        // A read runs, in a new tab, read-only by default, with the
        // Editor's limit and timeout.
        let key = key_of(&harness, tab, 0);
        harness.app.apply(Action::RunHistoryAgain { tab, key });
        let editor = sql_in_front(&harness, tab).expect("a new SQL tab in front");
        assert_eq!(
            (editor.text.as_str(), editor.mode),
            (READ, RunMode::ReadOnly)
        );
        let Some(Command::RunSql {
            limit,
            timeout,
            mode,
            statements,
            ..
        }) = harness.app.backend.sent.last()
        else {
            panic!("the read is run: {}", look.name);
        };
        assert_eq!(*limit, 500);
        assert_eq!(*timeout, Some(std::time::Duration::from_secs(15)));
        assert_eq!(*mode, tabletist_db::ScriptMode::ReadOnly);
        assert_eq!(statements.len(), 1);
    }
}

#[test]
fn run_again_on_production_is_read_only_whatever_the_settings() {
    let (mut harness, tab, _) = covers(Look::macos());
    harness.app.settings.new_tab = crate::settings::NewTab::Connection;
    harness.app.workspace_mut(tab).unwrap().environment = crate::env::Environment::Production;
    let read = entry(&harness, tab, Source::Sql, Some(READ));
    shown_with(&mut harness, tab, vec![read]);
    select(&mut harness, tab, 0);
    assert!(harness.has("production"), "the pane says where it runs");
    let key = key_of(&harness, tab, 0);
    harness.app.apply(Action::RunHistoryAgain { tab, key });
    let editor = sql_in_front(&harness, tab).expect("a new SQL tab");
    assert_eq!(editor.mode, RunMode::ReadOnly);
}

#[test]
fn a_write_opens_in_a_read_only_tab_that_does_not_run() {
    let (mut harness, tab, _) = covers(Look::macos());
    harness.app.settings.new_tab = crate::settings::NewTab::Connection;
    let save = entry(&harness, tab, Source::Save, Some(SAVE));
    shown_with(&mut harness, tab, vec![save]);
    select(&mut harness, tab, 0);
    let before = runs_sent(&harness);
    harness.click("Open in SQL editor");
    harness.settle();
    // Its statements, without the BEGIN and COMMIT a run that writes
    // would refuse: the run is one transaction itself.
    let editor = sql_in_front(&harness, tab).expect("a new SQL tab");
    let statements = "UPDATE \"main\".\"book_covers\" SET \"kind\" = 'ebook' WHERE \"id\" = 2;";
    assert_eq!(
        (editor.text.as_str(), editor.mode),
        (statements, RunMode::ReadOnly)
    );
    assert_eq!(runs_sent(&harness), before, "nothing runs");
}

#[test]
fn reapply_filter_opens_the_table_with_its_filter_and_sort() {
    let (mut harness, tab, id) = covers(Look::macos());
    let mut filter = entry(
        &harness,
        tab,
        Source::Filter,
        Some("SELECT * FROM book_covers"),
    );
    filter.query = Some(crate::session::KeptTab {
        schema: "main".into(),
        name: "book_covers".into(),
        kind: "table".into(),
        sort: vec![("created_at".into(), "asc".into())],
        filter: crate::session::KeptFilter {
            rows: vec![("kind".into(), "eq".into(), "print".into())],
            ..Default::default()
        },
        pinned: true,
        rest: Default::default(),
    });
    shown_with(&mut harness, tab, vec![filter]);
    select(&mut harness, tab, 0);
    harness.click("Reapply filter");
    harness.settle();
    // The table's own tab, in front, with the filter and the sort set.
    let workspace = harness.app.workspace(tab).unwrap();
    assert_eq!(workspace.active_tab, Some(id));
    let object = workspace.object_tab(id).unwrap();
    assert_eq!(object.query.filters.len(), 1);
    assert_eq!(object.query.filters[0].value, "print");
    assert_eq!(object.query.sort[0].column, "created_at");
    let Some(Command::FetchRows { query, .. }) = harness
        .app
        .backend
        .sent
        .iter()
        .rev()
        .find(|command| matches!(command, Command::FetchRows { .. }))
    else {
        panic!("the rows are read");
    };
    assert_eq!((query.filters.len(), query.sort.len()), (1, 1));
}

#[test]
fn the_history_keeps_what_the_setting_says_and_off_keeps_nothing() {
    let (mut harness, tab, _) = covers(Look::macos());
    harness.app.settings.history_keep = 3;
    shown_with(&mut harness, tab, Vec::new());
    for n in 0..5 {
        run(&mut harness, tab, &format!("SELECT {n}"));
        harness.answer_sql(Ok(script_outcome(vec![rows_outcome(1)])), None);
    }
    // Each write trims the file to the setting, and the open tab holds
    // what the file does.
    let keeps: Vec<u32> = harness
        .app
        .backend
        .sent
        .iter()
        .filter_map(|command| match command {
            Command::History(HistoryCommand::Append { keep, .. }) => Some(*keep),
            _ => None,
        })
        .collect();
    assert_eq!(keeps, [3; 5]);
    let history = harness.app.workspace(tab).unwrap().history_tab().unwrap();
    let kept: Vec<_> = history
        .entries
        .iter()
        .filter_map(|row| row.entry.sql.as_deref())
        .collect();
    assert_eq!(
        kept,
        ["SELECT 4", "SELECT 3", "SELECT 2"],
        "the newest first"
    );
    // A smaller setting trims what is kept.
    harness.app.apply(Action::SetOption(
        crate::settings::OptionValue::HistoryKeep(100),
    ));
    let trimmed = harness.app.backend.sent.iter().any(|command| {
        matches!(
            command,
            Command::History(HistoryCommand::Trim { keep: 100, .. })
        )
    });
    assert!(trimmed);
    // Don't keep: nothing is recorded, and the tab says it is off.
    harness.app.settings.history_keep = 0;
    let before = harness.history_written().len();
    run(&mut harness, tab, READ);
    harness.answer_sql(Ok(script_outcome(vec![rows_outcome(1)])), None);
    harness
        .app
        .apply(Action::Backend(Event::Agents(AgentsEvent::Called(call(
            &harness, tab, 9,
        )))));
    assert_eq!(harness.history_written().len(), before);
    harness.app.apply(Action::ShowHistory(tab));
    harness.settle();
    assert!(harness.has("History is off"));
    harness.click("Open Settings › Editor");
    assert!(matches!(
        &harness.app.dialog,
        Some(crate::model::Dialog::Settings(dialog))
            if dialog.pane == crate::model::SettingsPane::Editor
    ));
}

#[test]
fn clear_history_clears_this_connection_and_settings_clears_every_one() {
    for look in looks() {
        let (mut harness, tab, _) = covers(look);
        let read = entry(&harness, tab, Source::Sql, Some(READ));
        shown_with(&mut harness, tab, vec![read]);
        // Asked first: the question says what the history holds.
        match look.terminal {
            true => {
                harness.app.workspace_mut(tab).unwrap().command = Some("history clear".into());
                harness.app.apply(Action::RunCommand(tab));
            }
            false => harness.click("Clear history…"),
        }
        harness.settle();
        assert!(
            harness.has(&look.label(
                "Clear history for Bookshop · dev? This can't be undone. It holds the values typed into grid saves, on this computer."
            )),
            "{}",
            look.name
        );
        let cleared = |harness: &Harness| {
            harness
                .app
                .backend
                .sent
                .iter()
                .find_map(|command| match command {
                    Command::History(HistoryCommand::Clear { connection, .. }) => {
                        Some(connection.clone())
                    }
                    _ => None,
                })
        };
        // Kept, with esc in the terminal, Cancel on macOS.
        match look.terminal {
            true => super::tests::type_key(&mut harness, Key::N, "n"),
            false => harness.click("Cancel"),
        }
        harness.settle();
        assert_eq!(cleared(&harness), None, "{}", look.name);
        assert_eq!(
            harness
                .app
                .workspace(tab)
                .unwrap()
                .history_tab()
                .unwrap()
                .entries
                .len(),
            1
        );
        harness.app.apply(Action::ClearHistory(tab));
        harness.settle();
        match look.terminal {
            true => super::tests::type_key(&mut harness, Key::Y, "y"),
            false => harness.click("Clear history"),
        }
        harness.settle();
        assert_eq!(
            cleared(&harness),
            Some(Some(conn(&harness, tab))),
            "{}",
            look.name
        );
        assert!(
            harness
                .app
                .workspace(tab)
                .unwrap()
                .history_tab()
                .unwrap()
                .entries
                .is_empty()
        );
    }
    // Settings › Editor › Clear history… clears every connection's.
    let (mut harness, _, _) = covers(Look::macos());
    harness
        .app
        .apply(Action::ShowSettingsPane(crate::model::SettingsPane::Editor));
    harness.settle();
    harness.click("Clear history…");
    harness.settle();
    harness.click("Clear history");
    harness.settle();
    assert!(harness.app.backend.sent.iter().any(|command| matches!(
        command,
        Command::History(HistoryCommand::Clear {
            connection: None,
            ..
        })
    )));
}

#[test]
fn dont_keep_in_settings_asks_whether_to_delete_what_is_kept() {
    for (look, delete) in [
        (Look::macos(), true),
        (Look::omarchy(), true),
        (Look::macos(), false),
    ] {
        let (mut harness, _, _) = covers(look);
        harness
            .app
            .apply(Action::ShowSettingsPane(crate::model::SettingsPane::Editor));
        harness.settle();
        harness.app.apply(Action::SetOption(
            crate::settings::OptionValue::HistoryKeep(0),
        ));
        harness.settle();
        assert_eq!(harness.app.settings.history_keep, 0);
        let asks = match look.terminal {
            true => "history is off · delete the statements it kept on this computer?",
            false => "History is off. Delete the statements it kept on this computer?",
        };
        assert!(harness.has(asks), "{}", look.name);
        match (look.terminal, delete) {
            (true, _) => {
                // Capital D: Shift held.
                harness.frame(vec![
                    crate::testing::key(Key::D, Modifiers::SHIFT),
                    egui::Event::Text("D".into()),
                ]);
                harness.frame(vec![crate::testing::release(Key::D, Modifiers::SHIFT)]);
            }
            (false, true) => harness.click("Delete"),
            (false, false) => harness.click("Keep"),
        }
        harness.settle();
        let cleared = harness.app.backend.sent.iter().any(|command| {
            matches!(
                command,
                Command::History(HistoryCommand::Clear {
                    connection: None,
                    ..
                })
            )
        });
        assert_eq!(cleared, delete, "{}", look.name);
    }
}

#[test]
fn the_source_the_result_and_the_search_combine() {
    let (mut harness, tab, _) = covers(Look::macos());
    let mut failed = entry(
        &harness,
        tab,
        Source::Sql,
        Some("SELECT id, created FROM book_covers"),
    );
    failed.outcome = Outcome::Error;
    failed.code = Some("42703".into());
    let ok = entry(&harness, tab, Source::Sql, Some("SELECT * FROM publishers"));
    let filter = entry(
        &harness,
        tab,
        Source::Filter,
        Some("SELECT * FROM book_covers"),
    );
    let agent = entry(&harness, tab, Source::Agent, None);
    shown_with(&mut harness, tab, vec![failed, ok, filter, agent]);
    held(&mut harness, vec![(7, "Claude Code".into())]);
    let shown = |harness: &Harness| {
        let history = harness.app.workspace(tab).unwrap().history_tab().unwrap();
        history
            .shown()
            .map(|row| row.entry.source)
            .collect::<Vec<_>>()
    };
    assert!(harness.has("4 statements · keeps the last 1,000 · on this computer"));
    // macOS: the menus and the search.
    harness.click("Source");
    harness.settle();
    harness.click("SQL editor");
    harness.settle();
    assert_eq!(shown(&harness), [Source::Sql, Source::Sql]);
    harness.app.apply(Action::FilterHistory {
        tab,
        filter: HistoryFilterSet::Outcome(Some(Outcome::Error)),
    });
    assert_eq!(shown(&harness), [Source::Sql]);
    harness.app.apply(Action::FilterHistory {
        tab,
        filter: HistoryFilterSet::Search("PUBLISHERS".into()),
    });
    assert!(shown(&harness).is_empty(), "every filter holds at once");
    harness.settle();
    assert!(harness.has("No statements in the history match these filters."));
    // A search never reads into an agent's call: its entry has none.
    harness.app.apply(Action::FilterHistory {
        tab,
        filter: HistoryFilterSet::Line("publishers".into()),
    });
    assert_eq!(shown(&harness), [Source::Sql]);
    harness.app.apply(Action::FilterHistory {
        tab,
        filter: HistoryFilterSet::Line("claude".into()),
    });
    assert_eq!(
        shown(&harness),
        [Source::Agent],
        "an agent's name and tool are"
    );
}

#[test]
fn omarchys_slash_line_reads_its_tokens_and_shows_the_unknown_ones() {
    let (mut harness, tab, _) = covers(Look::omarchy());
    let mut saved = entry(&harness, tab, Source::Save, Some(SAVE));
    saved.outcome = Outcome::Saved;
    let read = entry(&harness, tab, Source::Sql, Some(READ));
    shown_with(&mut harness, tab, vec![saved, read]);
    harness.app.workspace_mut(tab).unwrap().pane = crate::model::Pane::Grid;
    super::tests::type_key(&mut harness, Key::Slash, "/");
    let history = harness.app.workspace(tab).unwrap().history_tab().unwrap();
    assert!(history.line_open);
    harness.frame(vec![egui::Event::Text("source:save kind:print".into())]);
    harness.settle();
    let history = harness.app.workspace(tab).unwrap().history_tab().unwrap();
    assert_eq!(history.filter.source, Some(Source::Save));
    assert_eq!(history.shown().count(), 1);
    // The token it does not know is shown, in red, and changes nothing.
    assert_eq!(
        harness.painted_color("kind:print"),
        Some(harness.app.palette.danger)
    );
    // Esc closes the line and keeps what it says.
    harness.press(Key::Escape, Modifiers::NONE);
    harness.settle();
    let history = harness.app.workspace(tab).unwrap().history_tab().unwrap();
    assert!(!history.line_open);
    assert_eq!(history.filter.source, Some(Source::Save));
}

#[test]
fn omarchys_keys_move_run_open_and_copy() {
    let (mut harness, tab, _) = covers(Look::omarchy());
    let read = entry(&harness, tab, Source::Sql, Some(READ));
    let save = entry(&harness, tab, Source::Save, Some(SAVE));
    shown_with(&mut harness, tab, vec![save, read]);
    harness.app.workspace_mut(tab).unwrap().pane = crate::model::Pane::Grid;
    super::tests::type_key(&mut harness, Key::J, "j");
    let first = key_of(&harness, tab, 0);
    let selected = |harness: &Harness| {
        harness
            .app
            .workspace(tab)
            .unwrap()
            .history_tab()
            .unwrap()
            .selected
    };
    assert_eq!(selected(&harness), Some(first));
    // A save: r says how it opens, y copies it.
    super::tests::type_key(&mut harness, Key::R, "r");
    assert!(harness.has("writes open with enter, into a read-only tab"));
    super::tests::type_key(&mut harness, Key::Y, "y");
    assert_eq!(harness.copied.as_deref(), Some(SAVE));
    // The read, a line down: r runs it.
    super::tests::type_key(&mut harness, Key::J, "j");
    assert_eq!(selected(&harness), Some(key_of(&harness, tab, 1)));
    let before = runs_sent(&harness);
    super::tests::type_key(&mut harness, Key::R, "r");
    assert_eq!(runs_sent(&harness), before + 1);
    assert_eq!(
        sql_in_front(&harness, tab).map(|sql| sql.text.as_str()),
        Some(READ)
    );
}

#[test]
fn every_way_in_shows_one_history_tab_and_none_opens_it_by_itself() {
    let (mut harness, tab, _) = covers(Look::macos());
    // Statements run, and nothing opens it.
    run(&mut harness, tab, READ);
    harness.answer_sql(Ok(script_outcome(vec![rows_outcome(1)])), None);
    assert_eq!(history_tabs(&harness, tab), 0);
    // ⌘Y, from the rows.
    harness.app.workspace_mut(tab).unwrap().pane = crate::model::Pane::Grid;
    let rows = harness
        .app
        .workspace(tab)
        .unwrap()
        .object_tabs()
        .next()
        .unwrap()
        .id;
    harness.app.apply(Action::ActivateTab { tab, id: rows });
    harness.settle();
    harness.press(Key::Y, Modifiers::COMMAND);
    harness.settle();
    assert!(front(&harness, tab));
    // The sidebar's clock, from another tab: the same tab.
    harness.app.apply(Action::ActivateTab { tab, id: rows });
    harness.settle();
    let clock = keyed(
        &harness,
        Look::macos(),
        "History",
        crate::keymap::Command::History,
    );
    harness.click(&clock);
    harness.settle();
    assert!(front(&harness, tab));
    // Quick open's History command, by its name; after `>` the commands
    // are all it lists.
    harness.app.apply(Action::ActivateTab { tab, id: rows });
    harness.app.apply(Action::OpenQuickOpen);
    let listed = |harness: &mut Harness, query: &str| {
        if let Some(crate::model::Dialog::QuickOpen(open)) = &mut harness.app.dialog {
            open.query = query.into();
        }
        let Some(crate::model::Dialog::QuickOpen(open)) = &harness.app.dialog else {
            panic!("quick open");
        };
        harness
            .app
            .quick_open_list(open)
            .items()
            .cloned()
            .collect::<Vec<_>>()
    };
    let history = |item: &crate::app::QuickItem| {
        matches!(
            item,
            crate::app::QuickItem::Command {
                command: crate::app::QuickCommand::History,
                ..
            }
        )
    };
    let commands = listed(&mut harness, "> Hist");
    assert_eq!(commands.len(), 1, "the commands alone");
    assert!(history(&commands[0]));
    let index = listed(&mut harness, "> History").iter().position(history);
    harness.app.apply(Action::QuickOpenPick {
        index,
        in_editor: false,
    });
    assert!(front(&harness, tab));
    assert_eq!(history_tabs(&harness, tab), 1);
    // `:history`, in the terminal look.
    harness.set_look(Look::omarchy());
    harness.app.apply(Action::ActivateTab { tab, id: rows });
    harness.app.workspace_mut(tab).unwrap().command = Some("history".into());
    harness.app.apply(Action::RunCommand(tab));
    assert!(front(&harness, tab));
    assert_eq!(history_tabs(&harness, tab), 1);
    // Closable, like any tab.
    let id = harness
        .app
        .workspace(tab)
        .unwrap()
        .history_tab()
        .unwrap()
        .id;
    harness.app.apply(Action::CloseTab { tab, id });
    assert_eq!(history_tabs(&harness, tab), 0);
}

#[test]
fn cmd_y_is_redo_in_a_text_field_and_history_outside_one() {
    let (mut harness, tab, _) = covers(Look::macos());
    harness.press(Key::T, Modifiers::COMMAND);
    harness.settle();
    harness.frame(vec![egui::Event::Text("select".into())]);
    harness.settle();
    harness.press(Key::Z, Modifiers::COMMAND);
    harness.press(Key::Y, Modifiers::COMMAND);
    harness.settle();
    assert_eq!(history_tabs(&harness, tab), 0, "the editor keeps its redo");
    assert_eq!(sql_in_front(&harness, tab).unwrap().text, "select");
}

#[test]
fn the_clock_is_pressed_only_while_history_is_in_front() {
    let (mut harness, tab, id) = covers(Look::macos());
    let clock = keyed(
        &harness,
        Look::macos(),
        "History",
        crate::keymap::Command::History,
    );
    let main = keyed(
        &harness,
        Look::macos(),
        "New SQL editor",
        crate::keymap::Command::NewSqlTab,
    );
    let pressed = |harness: &mut Harness| {
        let tree = harness.settle();
        let node = crate::testing::node(&tree, &clock, Role::Button).expect("the clock");
        tree.nodes
            .iter()
            .find(|(at, _)| *at == node)
            .and_then(|(_, data)| data.toggled())
    };
    let tree = harness.settle();
    assert!(crate::testing::node(&tree, "SQL editor", Role::Group).is_some());
    assert!(crate::testing::node(&tree, &main, Role::Button).is_some());
    assert_eq!(pressed(&mut harness), Some(egui::accesskit::Toggled::False));
    harness.app.apply(Action::ShowHistory(tab));
    assert_eq!(pressed(&mut harness), Some(egui::accesskit::Toggled::True));
    harness.app.apply(Action::ActivateTab { tab, id });
    assert_eq!(pressed(&mut harness), Some(egui::accesskit::Toggled::False));
}

#[test]
fn the_list_is_a_table_of_rows_by_day_and_the_pane_a_region() {
    let (mut harness, tab, _) = covers(Look::macos());
    let mut yesterday = entry(
        &harness,
        tab,
        Source::Filter,
        Some("SELECT * FROM publishers"),
    );
    yesterday.at_ms -= 26 * 3_600_000;
    let today = entry(&harness, tab, Source::Sql, Some(READ));
    shown_with(&mut harness, tab, vec![today, yesterday]);
    select(&mut harness, tab, 0);
    let tree = harness.settle();
    assert!(crate::testing::node(&tree, "Statements", Role::Table).is_some());
    assert!(crate::testing::node(&tree, "TODAY", Role::RowGroup).is_some());
    assert!(crate::testing::node(&tree, "YESTERDAY", Role::RowGroup).is_some());
    assert!(crate::testing::node(&tree, "Statement", Role::Region).is_some());
    let row = tree.nodes.iter().any(|(_, node)| {
        node.role() == Role::Row
            && node
                .label()
                .or_else(|| node.value())
                .is_some_and(|label| label.contains("SQL editor · Query 1 · SELECT name, country"))
    });
    assert!(row, "a row per entry");
}

#[test]
fn a_new_entry_comes_in_at_the_top_and_the_selection_stays() {
    let (mut harness, tab, _) = covers(Look::macos());
    let read = entry(&harness, tab, Source::Sql, Some(READ));
    shown_with(&mut harness, tab, vec![read]);
    select(&mut harness, tab, 0);
    let selected = key_of(&harness, tab, 0);
    // Run from elsewhere: the tab, behind, takes it in.
    run(&mut harness, tab, "SELECT 1");
    harness.answer_sql(Ok(script_outcome(vec![rows_outcome(1)])), None);
    let history = harness.app.workspace(tab).unwrap().history_tab().unwrap();
    assert_eq!(history.entries[0].entry.sql.as_deref(), Some("SELECT 1"));
    assert_eq!(history.selected, Some(selected));
    assert_eq!(history.entries[1].key, selected);
}

#[test]
fn a_read_answered_after_an_entry_keeps_it() {
    let (mut harness, tab, _) = covers(Look::macos());
    harness.app.apply(Action::ShowHistory(tab));
    // The file's answer was read before this ran: it is not in it.
    run(&mut harness, tab, "SELECT 1");
    harness.answer_sql(Ok(script_outcome(vec![rows_outcome(1)])), None);
    let older = entry(&harness, tab, Source::Sql, Some(READ));
    harness.answer_history(vec![older]);
    let history = harness.app.workspace(tab).unwrap().history_tab().unwrap();
    let kept: Vec<_> = history
        .entries
        .iter()
        .filter_map(|row| row.entry.sql.as_deref())
        .collect();
    assert_eq!(kept, ["SELECT 1", READ]);
}

#[test]
fn settings_editor_shows_the_history_tab() {
    for look in looks() {
        let (mut harness, tab, _) = covers(look);
        harness
            .app
            .apply(Action::ShowSettingsPane(crate::model::SettingsPane::Editor));
        harness.settle();
        match look.terminal {
            true => {
                // Enter on the row the history keeps.
                let rows = crate::model::SettingsPane::Editor.rows();
                let at = rows
                    .iter()
                    .position(|row| {
                        *row == crate::model::SettingsRow::Option(
                            crate::settings::OptionId::HistoryKeep,
                        )
                    })
                    .unwrap();
                harness.app.apply(Action::SelectSettingsRow(at));
                harness.settle();
                harness.press(Key::Enter, Modifiers::NONE);
            }
            false => harness.click("Show History"),
        }
        harness.settle();
        assert!(harness.app.dialog.is_none(), "{}", look.name);
        assert!(front(&harness, tab), "{}", look.name);
    }
}

#[test]
fn the_arrows_and_enter_are_the_lists_and_not_the_trees() {
    for look in looks() {
        let (mut harness, tab, _) = covers(look);
        let read = entry(&harness, tab, Source::Sql, Some(READ));
        let save = entry(&harness, tab, Source::Save, Some(SAVE));
        shown_with(&mut harness, tab, vec![read, save]);
        let tree = harness.app.workspace(tab).unwrap().tree.cursor.clone();
        harness.press(Key::ArrowDown, Modifiers::NONE);
        harness.press(Key::ArrowDown, Modifiers::NONE);
        harness.settle();
        let workspace = harness.app.workspace(tab).unwrap();
        assert_eq!(workspace.tree.cursor, tree, "{}", look.name);
        let second = key_of(&harness, tab, 1);
        let selected = workspace.history_tab().unwrap().selected;
        assert_eq!(selected, Some(second), "{}", look.name);
        if look.terminal {
            // Enter opens the selected statement.
            harness.press(Key::Enter, Modifiers::NONE);
            harness.settle();
            assert!(sql_in_front(&harness, tab).is_some(), "{}", look.name);
        }
    }
}

#[test]
fn while_clear_history_asks_only_its_answers_are_read() {
    let (mut harness, tab, _) = covers(Look::omarchy());
    let read = entry(&harness, tab, Source::Sql, Some(READ));
    shown_with(&mut harness, tab, vec![read]);
    harness.app.workspace_mut(tab).unwrap().pane = crate::model::Pane::Grid;
    super::tests::type_key(&mut harness, Key::J, "j");
    harness.app.apply(Action::ClearHistory(tab));
    harness.settle();
    let before = runs_sent(&harness);
    super::tests::type_key(&mut harness, Key::R, "r");
    assert_eq!(
        runs_sent(&harness),
        before,
        "r runs nothing under the question"
    );
    assert!(
        harness
            .app
            .workspace(tab)
            .unwrap()
            .history_tab()
            .unwrap()
            .clearing
    );
}

#[test]
fn a_call_that_ended_while_the_log_was_asked_is_not_taken_for_gone() {
    let (mut harness, tab, _) = covers(Look::macos());
    let older = entry(&harness, tab, Source::Agent, None);
    shown_with(&mut harness, tab, vec![older]);
    let asked = harness
        .app
        .backend
        .sent
        .iter()
        .rev()
        .find_map(|command| match command {
            Command::History(HistoryCommand::Held { asked, .. }) => Some(*asked),
            _ => None,
        })
        .expect("the log is asked");
    // A call ends meanwhile; the answer is of the one asked about.
    harness
        .app
        .apply(Action::Backend(Event::Agents(AgentsEvent::Called(call(
            &harness, tab, 99,
        )))));
    harness
        .app
        .apply(Action::Backend(Event::History(HistoryEvent::Held {
            asked,
            held: Ok(vec![(7, "Claude Code".into())]),
        })));
    let history = harness.app.workspace(tab).unwrap().history_tab().unwrap();
    assert!(history.gone.is_empty());
}

#[test]
fn the_history_tab_stays_through_a_switch_of_database() {
    let (mut harness, tab, _) = covers(Look::macos());
    harness.app.apply(Action::ShowHistory(tab));
    harness.app.apply(Action::SwitchDatabase {
        tab,
        database: "bookshop_test".into(),
    });
    assert_eq!(history_tabs(&harness, tab), 1);
}

#[test]
fn an_agents_name_is_read_from_the_agents_log_and_not_kept() {
    for look in looks() {
        let (mut harness, tab, _) = covers(look);
        let agent = entry(&harness, tab, Source::Agent, None);
        shown_with(&mut harness, tab, vec![agent]);
        select(&mut harness, tab, 0);
        // Until the log says whose call it is, the word alone.
        let word = look.label("Agent");
        let named = format!("{word} · {}", look.label("Claude Code"));
        let rows = |harness: &mut Harness| {
            let tree = harness.settle();
            crate::testing::labels(&tree)
                .into_iter()
                .filter(|label| label.contains("run_query"))
                .collect::<Vec<_>>()
        };
        assert!(
            rows(&mut harness).iter().all(|row| !row.contains(&named)),
            "{}",
            look.name
        );
        held(&mut harness, vec![(7, "Claude Code".into())]);
        assert!(
            rows(&mut harness).iter().any(|row| row.contains(&named)),
            "{}",
            look.name
        );
        // Nothing of it was written to the history.
        assert!(harness.history_written().is_empty());
    }
}
