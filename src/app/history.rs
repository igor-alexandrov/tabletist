//! The query history, as the app keeps it: what each source of statements
//! records, the History tab's reads and filters, and what it does with an
//! entry. The file is the backend's ([`crate::backend::HistoryCommand`]);
//! the entries live here only in each open History tab.

use std::time::Duration;

use tabletist_db::{Error, ScriptOutcome, StatementOutcome};

use super::App;
use crate::backend::{CancelReason, Command, HistoryCommand, HistoryEvent};
use crate::history::{Entry, Kind, Outcome, Source};
use crate::model::ConnTabId;
use crate::model::{
    Action, HistoryFilterSet, HistoryRow, HistoryTab, Pane, RunMode, Tab, TabId, Workspace,
};

/// Milliseconds of Unix time now.
pub(super) fn now_ms() -> i64 {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    i64::try_from(now.as_millis()).unwrap_or(i64::MAX)
}

fn millis(took: Duration) -> u64 {
    u64::try_from(took.as_millis()).unwrap_or(u64::MAX)
}

/// The database's code for `error`, where it has one.
fn code_of(error: &Error) -> Option<String> {
    match error {
        Error::Query { code, .. } => code.clone(),
        _ => None,
    }
}

/// An entry of `workspace`'s connection, from `source` and `detail`, as it
/// is before it runs: what it ran and how it ended are filled in after.
pub(super) fn entry_on(workspace: &Workspace, source: Source, detail: String) -> Entry {
    Entry {
        at_ms: now_ms(),
        source,
        detail,
        tool: None,
        connection: workspace.conn_id.0.clone(),
        database: crate::session::database_of(&workspace.spec),
        sql: None,
        call: None,
        kind: Kind::Read,
        read_only: true,
        rows: None,
        took_ms: None,
        outcome: Outcome::Ok,
        code: None,
        message: None,
        query: None,
    }
}

/// What a SQL editor's run comes to in the history: an entry for each
/// statement that ran, as it was split, from `base`. A run refused as a
/// whole ran nothing: it is an entry of the statement it was refused for
/// (its first, where it names none). A run that failed as a whole may have
/// run some of its statements: each is an entry, with the run's error. A
/// run stopped before any statement began ran nothing, and is none. In a
/// run that writes and was not committed, a statement that went through
/// says it was rolled back.
pub(super) fn run_entries(
    base: &Entry,
    dialect: tabletist_db::Dialect,
    (statements, mode): (&[tabletist_db::sql::Statement], tabletist_db::ScriptMode),
    result: &Result<ScriptOutcome, Error>,
    cancel: Option<CancelReason>,
) -> Vec<Entry> {
    let of = |statement: &tabletist_db::sql::Statement| Entry {
        sql: Some(statement.text.clone()),
        kind: Kind::of(dialect, &statement.text),
        ..base.clone()
    };
    let failed = |statement: &tabletist_db::sql::Statement, error: &Error| {
        let cancelled = *error == Error::Cancelled;
        Entry {
            outcome: match cancelled {
                true => Outcome::Cancelled,
                false => Outcome::Error,
            },
            code: code_of(error),
            message: (!cancelled).then(|| crate::history::first_line(&error.to_string())),
            ..of(statement)
        }
    };
    let outcome = match result {
        Ok(outcome) => outcome,
        // Nothing ran.
        Err(error @ Error::Refused { line, .. }) => {
            let at = statements
                .iter()
                .position(|statement| statement.first_line == *line)
                .unwrap_or_default();
            return statements
                .get(at)
                .map(|statement| failed(statement, error))
                .into_iter()
                .collect();
        }
        Err(error @ Error::ReadOnly) => {
            return statements
                .first()
                .map(|statement| failed(statement, error))
                .into_iter()
                .collect();
        }
        // The session was lost or left its transaction: what ran of it is
        // not said, so each statement is kept with the run's error.
        Err(error) => {
            return statements
                .iter()
                .map(|statement| failed(statement, error))
                .collect();
        }
    };
    // Which statements' work was undone: none of a read, which writes
    // nothing to undo.
    let undone = |at: usize| match (mode, &outcome.end) {
        (tabletist_db::ScriptMode::ReadOnly, _) | (_, tabletist_db::ScriptEnd::Committed) => false,
        (_, tabletist_db::ScriptEnd::RolledBack) => true,
        (
            _,
            tabletist_db::ScriptEnd::Partly { committed }
            | tabletist_db::ScriptEnd::CommitFailed { committed, .. },
        ) => at >= *committed,
    };
    let mut at_ms = base.at_ms;
    let mut entries = Vec::new();
    for (at, (statement, ran)) in statements.iter().zip(&outcome.results).enumerate() {
        let mut entry = of(statement);
        entry.at_ms = at_ms;
        entry.took_ms = Some(millis(ran.elapsed));
        at_ms = at_ms.saturating_add(i64::try_from(millis(ran.elapsed)).unwrap_or_default());
        match &ran.outcome {
            StatementOutcome::Rows { rows, .. } => {
                entry.rows = Some(rows.len() as u64);
            }
            StatementOutcome::Done { affected, .. } => entry.rows = *affected,
            StatementOutcome::Error { error, .. } => {
                entry.outcome = Outcome::Error;
                entry.code = code_of(error);
                entry.message = Some(crate::history::first_line(&error.to_string()));
            }
            StatementOutcome::Cancelled => {
                entry.outcome = Outcome::Cancelled;
                entry.message = match cancel {
                    Some(CancelReason::Timeout(_)) => Some("timeout".to_owned()),
                    Some(CancelReason::User) | None => None,
                };
            }
        }
        if entry.outcome == Outcome::Ok && entry.kind == Kind::Write && undone(at) {
            entry.message = Some("rolled back".to_owned());
        }
        entries.push(entry);
    }
    entries
}

/// A save's statements as one transaction, its values written in: what
/// the history keeps of it. `None` for a set no statement can be made of.
pub(super) fn save_sql(
    dialect: tabletist_db::Dialect,
    changes: &tabletist_db::ChangeSet,
) -> Option<String> {
    let review = crate::review::of(
        dialect,
        changes,
        crate::review::Blocked::default(),
        crate::review::Values::Whole,
    );
    let lines: Vec<String> = review.lines.iter().filter_map(|line| line.sql()).collect();
    if lines.is_empty() {
        return None;
    }
    Some(format!("BEGIN;\n{}\nCOMMIT;", lines.join("\n")))
}

/// What a save came to, in its entry: saved, with the rows it changed,
/// or why nothing was written.
pub(super) fn saved(
    mut entry: Entry,
    saving: &crate::edit::Saving,
    result: &Result<tabletist_db::WriteOutcome, Error>,
) -> Entry {
    use tabletist_db::WriteOutcome;
    entry.took_ms = Some(millis(saving.started.elapsed()));
    let failed = |entry: &mut Entry, error: &Error| {
        entry.outcome = Outcome::Error;
        entry.code = code_of(error);
        entry.message = Some(crate::history::first_line(&error.to_string()));
    };
    match result {
        Ok(WriteOutcome::Written { gone, elapsed, .. }) => {
            entry.outcome = Outcome::Saved;
            entry.took_ms = Some(millis(*elapsed));
            let changed = saving.rows.len() + saving.inserts.len() + saving.deletes.len();
            entry.rows = Some(changed.saturating_sub(gone.len()) as u64);
        }
        Ok(WriteOutcome::Conflicts(conflicts)) => {
            entry.outcome = Outcome::Error;
            entry.message = Some(format!(
                "{} changed on the server since it was loaded; nothing was saved",
                match conflicts.len() {
                    1 => "a row".to_owned(),
                    rows => format!("{rows} rows"),
                }
            ));
        }
        Ok(
            WriteOutcome::Failed { error, .. }
            | WriteOutcome::FailedInsert { error, .. }
            | WriteOutcome::FailedDelete { error, .. },
        ) => failed(&mut entry, error),
        Err(Error::Cancelled) => entry.outcome = Outcome::Cancelled,
        Err(error) => failed(&mut entry, error),
    }
    entry
}

/// A table's load for a filter or a sort set on it, in its entry, from
/// `base`: the `SELECT` it ran with its values written in, the rows it
/// read, and the filter and sort, to set them again.
pub(super) fn filtered(
    base: Entry,
    dialect: tabletist_db::Dialect,
    object: &crate::model::ObjectTab,
    result: &Result<tabletist_db::RowPage, Error>,
) -> Entry {
    // Ordered by the key after the sort, where the table's key is known,
    // as the adapters order it.
    let key = object
        .structure
        .value
        .as_ref()
        .map(|structure| structure.primary_key.clone())
        .unwrap_or_default();
    let started = object.rows.started.map(|started| started.elapsed());
    let mut entry = Entry {
        detail: object.object.name.clone(),
        sql: Some(dialect.shown_rows(&object.query, &key, &[])),
        query: Some(crate::session::KeptTab::of(object)),
        at_ms: base.at_ms.saturating_sub(
            started.map_or(0, |took| i64::try_from(millis(took)).unwrap_or_default()),
        ),
        took_ms: started.map(millis),
        ..base
    };
    match result {
        Ok(page) => {
            entry.rows = Some(page.rows.len() as u64);
            entry.took_ms = Some(millis(page.elapsed));
        }
        Err(Error::Cancelled) => entry.outcome = Outcome::Cancelled,
        Err(error) => {
            entry.outcome = Outcome::Error;
            entry.code = code_of(error);
            entry.message = Some(crate::history::first_line(&error.to_string()));
        }
    }
    entry
}

impl App {
    /// The window's connection as its History tab names it, the look's
    /// word for its environment after it: "Bookshop · dev".
    pub fn history_name(&self, tab: ConnTabId) -> String {
        let Some(workspace) = self.workspace(tab) else {
            return String::new();
        };
        let name = crate::format::display_safe(&workspace.name);
        match workspace
            .environment
            .named(crate::env::Platform::of(&self.look))
        {
            Some(what) => format!("{name} · {what}"),
            None => name.into_owned(),
        }
    }

    /// Takes `entry` into the history: written by the backend, and shown
    /// at the top of each open History tab of its connection, where the
    /// selection stays as it was. Nothing is kept while the history is
    /// off.
    pub(super) fn record(&mut self, entry: Entry) {
        let keep = self.settings.history_keep;
        if keep == 0 {
            return;
        }
        let file = self.dirs.history_file();
        let tabs: Vec<ConnTabId> = self.open_connections().map(|(tab, _)| tab).collect();
        for tab in tabs {
            let key = self.next_id();
            let Some(workspace) = self.workspace_mut(tab) else {
                continue;
            };
            if workspace.conn_id.0 != entry.connection {
                continue;
            }
            let Some(history) = workspace.history_tab_mut() else {
                continue;
            };
            history.entries.insert(0, row(key, entry.clone()));
            history.entries.truncate(keep as usize);
        }
        self.backend.send(Command::History(HistoryCommand::Append {
            file,
            entry: Box::new(entry),
            keep,
        }));
    }

    /// Shows the workspace's History tab: opened at the end of the strip
    /// where it is not open yet, and read again from the file each time it
    /// is shown.
    pub(super) fn show_history(&mut self, tab: ConnTabId) {
        let id = TabId(self.next_id());
        let Some(workspace) = self.workspace_mut(tab) else {
            return;
        };
        let shown = match workspace.history_tab() {
            Some(history) => history.id,
            None => {
                workspace
                    .tabs
                    .push(Tab::History(Box::new(HistoryTab::new(id))));
                id
            }
        };
        workspace.active_tab = Some(shown);
        workspace.pane = Pane::Grid;
        self.read_history(tab);
    }

    /// Reads the History tab's entries from the file.
    fn read_history(&mut self, tab: ConnTabId) {
        let asked = self.next_id();
        let (file, keep) = (self.dirs.history_file(), self.settings.history_keep);
        let Some(workspace) = self.workspace_mut(tab) else {
            return;
        };
        let connection = workspace.conn_id.0.clone();
        let Some(history) = workspace.history_tab_mut() else {
            return;
        };
        history.asked = Some(asked);
        // Off, the tab says so: nothing is read, and nothing is kept.
        if keep == 0 {
            history.asked = None;
            history.read = true;
            history.entries.clear();
            return;
        }
        self.backend.send(Command::History(HistoryCommand::Load {
            file,
            connection,
            keep,
            asked,
        }));
    }

    /// What came of a read of the history, or of a look into the Agents
    /// Log, for the tab that asked.
    pub(super) fn history_event(&mut self, event: HistoryEvent) {
        let tabs: Vec<ConnTabId> = self.open_connections().map(|(tab, _)| tab).collect();
        match event {
            HistoryEvent::Loaded { asked, result } => {
                for tab in tabs {
                    let asked_here = self
                        .workspace(tab)
                        .and_then(Workspace::history_tab)
                        .is_some_and(|history| history.asked == Some(asked));
                    if !asked_here {
                        continue;
                    }
                    let read = result.as_ref().map_or(0, Vec::len);
                    let keys: Vec<u64> = (0..read).map(|_| self.next_id()).collect();
                    let keep = self.settings.history_keep as usize;
                    let Some(history) = self.history_mut(tab) else {
                        continue;
                    };
                    history.asked = None;
                    history.read = true;
                    match &result {
                        Ok(entries) => {
                            history.failed = None;
                            let was = history
                                .selected
                                .and_then(|key| history.row(key))
                                .map(|row| row.entry.clone());
                            // What ran after the read was asked for is not
                            // in what it read: the file took it after.
                            let mut kept: Vec<HistoryRow> = history
                                .entries
                                .drain(..)
                                .filter(|row| row.key > asked)
                                .collect();
                            kept.extend(
                                keys.into_iter()
                                    .zip(entries)
                                    .map(|(key, entry)| row(key, entry.clone())),
                            );
                            kept.truncate(keep);
                            history.entries = kept;
                            // The entry selected stays so, where it is kept.
                            history.selected = was.and_then(|was| {
                                let row = history.entries.iter().find(|row| row.entry == was)?;
                                Some(row.key)
                            });
                        }
                        Err(error) => {
                            log::warn!("the query history could not be read: {error}");
                            history.failed = Some(error.clone());
                        }
                    }
                    self.ask_held(tab);
                }
            }
            HistoryEvent::Held { asked, held } => {
                for tab in tabs {
                    let Some(history) = self
                        .workspace_mut(tab)
                        .and_then(Workspace::history_tab_mut)
                        .filter(|history| {
                            history.held_asked.as_ref().map(|(id, _)| *id) == Some(asked)
                        })
                    else {
                        continue;
                    };
                    let Some((_, calls)) = history.held_asked.take() else {
                        continue;
                    };
                    match &held {
                        Ok(held) => {
                            history.gone = calls
                                .into_iter()
                                .filter(|call| !held.iter().any(|(id, _)| id == call))
                                .collect();
                            for (call, agent) in held {
                                history.named(*call, agent.clone());
                            }
                        }
                        Err(error) => {
                            log::warn!("the Agents Log could not be read: {error}");
                        }
                    }
                }
            }
        }
    }

    /// Asks the Agents Log which of the History tab's agents' calls it
    /// still holds: one that is gone says so in its entry.
    fn ask_held(&mut self, tab: ConnTabId) {
        let asked = self.next_id();
        let log = self.dirs.agents_log_file();
        let Some(history) = self.workspace_mut(tab).and_then(Workspace::history_tab_mut) else {
            return;
        };
        // Each by its place and its time: a place the log gives again is
        // another call's.
        let calls: Vec<(i64, i64)> = history
            .entries
            .iter()
            .filter_map(|row| Some((row.entry.call?, row.entry.at_ms)))
            .collect();
        if calls.is_empty() {
            return;
        }
        let ids = calls.iter().map(|(id, _)| *id).collect();
        history.held_asked = Some((asked, ids));
        self.backend
            .send(Command::History(HistoryCommand::Held { log, calls, asked }));
    }

    /// The History tab's actions.
    pub(super) fn history_action(&mut self, action: Action) {
        match action {
            Action::ShowHistory(tab) => self.show_history(tab),
            Action::FilterHistory { tab, filter } => {
                if let Some(history) = self.history_mut(tab) {
                    match filter {
                        HistoryFilterSet::Source(source) => history.filter.source = source,
                        HistoryFilterSet::Outcome(outcome) => history.filter.outcome = outcome,
                        HistoryFilterSet::Search(text) => history.filter.search = text,
                        HistoryFilterSet::Line(text) => {
                            history.filter = crate::history::Line::read(&text).filter;
                            history.line = text;
                        }
                    }
                }
            }
            Action::ShowHistoryLine { tab, open } => {
                // Each ask gives the field the keyboard, open or not.
                if let Some(history) = self.history_mut(tab) {
                    if open {
                        history.line_asked += 1;
                    }
                    history.line_open = open;
                }
            }
            Action::SelectHistoryEntry { tab, key } => {
                if let Some(history) = self.history_mut(tab) {
                    history.selected = Some(key);
                    history.said = None;
                }
            }
            Action::MoveHistorySelection { tab, step } => {
                if let Some(history) = self.history_mut(tab) {
                    let keys: Vec<u64> = history.shown().map(|row| row.key).collect();
                    let at = history
                        .selected
                        .and_then(|key| keys.iter().position(|shown| *shown == key));
                    let to = match at {
                        Some(at) => at
                            .saturating_add_signed(step)
                            .min(keys.len().saturating_sub(1)),
                        None => 0,
                    };
                    history.selected = keys.get(to).copied().or(history.selected);
                    history.said = None;
                }
            }
            Action::RunHistoryAgain { tab, key } => self.run_history_again(tab, key),
            Action::OpenHistoryEntry { tab, key } => {
                let Some(entry) = self.history_entry(tab, key).cloned() else {
                    return;
                };
                let Some(sql) = entry.sql.clone() else {
                    return;
                };
                let Some(id) = self.new_sql_tab(tab) else {
                    return;
                };
                let read_only = entry.kind == Kind::Write || entry.source == Source::Save;
                // A save's statements without its BEGIN and COMMIT: a run
                // that writes is one transaction, and refuses a script that
                // ends its own.
                let sql = match entry.source {
                    Source::Save => unwrapped(&sql),
                    Source::Sql | Source::Filter | Source::Agent => sql,
                };
                if let Some(editor) = self.sql_tab_mut(tab, id) {
                    editor.text = sql;
                    if read_only {
                        editor.mode = RunMode::ReadOnly;
                    }
                }
            }
            Action::ReapplyHistoryFilter { tab, key } => self.reapply_history_filter(tab, key),
            Action::ShowHistoryCall { tab, key } => {
                let Some(call) = self.history_entry(tab, key).and_then(|entry| entry.call) else {
                    return;
                };
                let gone = self
                    .workspace(tab)
                    .and_then(Workspace::history_tab)
                    .is_some_and(|history| history.gone.contains(&call));
                if gone {
                    return;
                }
                self.show_agents(tab);
                if let Some(agents) = self.workspace_mut(tab).and_then(Workspace::agents_tab_mut) {
                    // Every call of this connection's, so the call is among
                    // them, read on until it is in.
                    agents.view = crate::model::AgentsView::Log;
                    let log = &mut agents.log;
                    (log.all, log.agent, log.result) = (false, None, None);
                    log.search.clear();
                    log.filters_open = false;
                    log.selected = None;
                    log.seek = Some(call);
                }
                self.read_agents_log(tab, false);
            }
            Action::ClearHistory(tab) => {
                self.show_history(tab);
                if let Some(history) = self.history_mut(tab) {
                    history.clearing = true;
                }
            }
            Action::ConfirmClearHistory { tab, clear } => {
                let Some(history) = self.history_mut(tab) else {
                    return;
                };
                history.clearing = false;
                if !clear {
                    return;
                }
                let Some(conn) = self
                    .workspace(tab)
                    .map(|workspace| workspace.conn_id.0.clone())
                else {
                    return;
                };
                self.clear_history(Some(conn));
            }
            _ => {}
        }
    }

    /// Clears the history of the connection `conn`, or of every one: the
    /// file's, and the open History tabs'.
    pub(super) fn clear_history(&mut self, conn: Option<String>) {
        let tabs: Vec<ConnTabId> = self.open_connections().map(|(tab, _)| tab).collect();
        for tab in tabs {
            let Some(workspace) = self.workspace_mut(tab) else {
                continue;
            };
            let here = conn
                .as_ref()
                .is_none_or(|conn| *conn == workspace.conn_id.0);
            if let Some(history) = workspace.history_tab_mut().filter(|_| here) {
                history.entries.clear();
                history.selected = None;
                history.gone.clear();
                // A read still out holds what is cleared now.
                history.asked = None;
                history.read = true;
            }
        }
        self.backend.send(Command::History(HistoryCommand::Clear {
            file: self.dirs.history_file(),
            connection: conn,
        }));
    }

    /// The history keeps `keep` entries per connection from now on: the
    /// file is trimmed to it, and so is each open History tab.
    pub(super) fn trim_history(&mut self, keep: u32) {
        let tabs: Vec<ConnTabId> = self.open_connections().map(|(tab, _)| tab).collect();
        for tab in tabs {
            if let Some(history) = self.history_mut(tab) {
                history.entries.truncate(keep as usize);
            }
        }
        if keep > 0 {
            self.backend.send(Command::History(HistoryCommand::Trim {
                file: self.dirs.history_file(),
                keep,
            }));
        }
    }

    /// Marks the load of the rows just sent for the object tab `id` as one
    /// a filter or a sort the person set asked for, where one is set: a
    /// table read with neither is not kept.
    pub(super) fn note_query_load(&mut self, tab: ConnTabId, id: TabId) {
        if let Some(object) = self.object_tab_mut(tab, id) {
            let query = &object.query;
            let set =
                !query.filters.is_empty() || query.raw_where.is_some() || !query.sort.is_empty();
            object.history_load = object.rows.pending.filter(|_| set);
        }
    }

    fn history_mut(&mut self, tab: ConnTabId) -> Option<&mut HistoryTab> {
        self.workspace_mut(tab).and_then(Workspace::history_tab_mut)
    }

    fn history_entry(&self, tab: ConnTabId, key: u64) -> Option<&Entry> {
        let history = self.workspace(tab)?.history_tab()?;
        Some(&history.row(key)?.entry)
    }

    /// Runs a read of the History tab of `tab` again, in a new SQL editor
    /// with the Editor settings of a new tab: its mode, its row limit and
    /// its timeout. On a connection that opens read-only, or on
    /// production, the editor is read-only whatever the settings. A write
    /// is never run: the terminal look's line says how to open one.
    fn run_history_again(&mut self, tab: ConnTabId, key: u64) {
        let Some(entry) = self.history_entry(tab, key).cloned() else {
            return;
        };
        let (Some(sql), true) = (entry.sql.clone(), entry.runs_again()) else {
            let key = self
                .keymap
                .key(self.layout(), crate::keymap::Command::OpenHistoryEntry);
            let said = match key.as_str() {
                // A look whose key for it is a button's.
                "" => crate::i18n::gettext(
                    self.locale,
                    "Writes open in a SQL editor, into a read-only tab",
                )
                .into_owned(),
                key => crate::i18n::gettext(
                    self.locale,
                    "writes open with {key}, into a read-only tab",
                )
                .replace("{key}", key),
            };
            if let Some(history) = self.history_mut(tab) {
                history.said = Some(said);
            }
            return;
        };
        let guarded = self.workspace(tab).is_some_and(|workspace| {
            workspace.sql_writes().is_err() || workspace.environment.is_production()
        });
        let Some(id) = self.new_sql_tab(tab) else {
            return;
        };
        if let Some(editor) = self.sql_tab_mut(tab, id) {
            editor.text = sql;
            editor.cursor = 0;
            if guarded {
                editor.mode = RunMode::ReadOnly;
            }
        }
        self.apply(Action::RunSql {
            tab,
            sql_tab: id,
            all: true,
        });
    }

    /// Opens the table of a filter's entry, or its tab where it is open,
    /// with the filter and the sort the entry keeps. A tab that holds
    /// edits asks first, as following a foreign key does.
    fn reapply_history_filter(&mut self, tab: ConnTabId, key: u64) {
        let Some(kept) = self
            .history_entry(tab, key)
            .and_then(|entry| entry.query.clone())
        else {
            return;
        };
        let Some((object, kind)) = kept.object() else {
            return;
        };
        self.open_object(tab, object, kind, true);
        let Some(id) = self
            .workspace(tab)
            .and_then(|workspace| workspace.active_tab)
        else {
            return;
        };
        let holds = self
            .workspace(tab)
            .and_then(|workspace| workspace.object_tab(id))
            .is_some_and(|opened| opened.edits.holds());
        if holds {
            let again = Action::ReapplyHistoryFilter { tab, key };
            self.hold(crate::model::Held::Action(Box::new(again)), vec![(tab, id)]);
            return;
        }
        if let Some(opened) = self.object_tab_mut(tab, id) {
            opened.filter = kept.bar();
            opened.query.sort = kept.sorts();
        }
        self.apply_filters(tab, id);
        if let Some(workspace) = self.workspace_mut(tab) {
            workspace.pane = Pane::Grid;
        }
    }

    /// An agent's call that ended, as its entry: on the connection it
    /// named, where it brought a statement. What it sent stays in the
    /// Agents Log, which the entry names the call's place in.
    pub(super) fn record_call(&mut self, call: &tabletist_mcp::Call) {
        let Some(connection) = call.connection.clone().filter(|_| call.statement) else {
            return;
        };
        let database = self
            .connections
            .get(&crate::connections::ConnectionId(connection.clone()))
            .map(|saved| crate::session::database_of(&saved.spec))
            .unwrap_or_default();
        let at_ms = call
            .at
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |at| i64::try_from(at.as_millis()).unwrap_or(i64::MAX));
        self.record(Entry {
            at_ms,
            source: Source::Agent,
            // The agent's name is what it sent: the Agents Log's alone.
            detail: String::new(),
            tool: Some(call.tool.clone()),
            connection,
            database,
            sql: None,
            call: call.logged,
            kind: Kind::Read,
            read_only: true,
            rows: call.rows,
            took_ms: Some(millis(call.took)),
            outcome: Outcome::of_call(&call.result),
            code: None,
            message: None,
            query: None,
        });
        // The tabs that show it name it, from what the server said, until
        // they read it from the log again.
        if let Some(logged) = call.logged {
            for tab in self
                .open_connections()
                .map(|(tab, _)| tab)
                .collect::<Vec<_>>()
            {
                if let Some(history) = self.history_mut(tab) {
                    history.named(logged, call.agent.clone());
                }
            }
        }
    }
}

/// A save's statements, without the `BEGIN` and `COMMIT` around them.
fn unwrapped(sql: &str) -> String {
    let inner = sql.strip_prefix("BEGIN;\n").unwrap_or(sql);
    inner.strip_suffix("\nCOMMIT;").unwrap_or(inner).to_owned()
}

/// An entry of a History tab.
fn row(key: u64, entry: Entry) -> HistoryRow {
    HistoryRow {
        key,
        searched: entry.searched(),
        entry,
    }
}

#[cfg(test)]
mod tests {
    use tabletist_db::{Dialect, Error, ScriptEnd, ScriptMode};

    use crate::history::{Entry, Kind, Outcome, Source};
    use crate::testing::{done_outcome, error_outcome, rows_outcome, script_outcome};

    fn base() -> Entry {
        Entry {
            at_ms: 1_792_000_000_000,
            source: Source::Sql,
            detail: "Query 1".into(),
            tool: None,
            connection: "bookshop".into(),
            database: "bookshop_development".into(),
            sql: None,
            call: None,
            kind: Kind::Read,
            read_only: true,
            rows: None,
            took_ms: None,
            outcome: Outcome::Ok,
            code: None,
            message: None,
            query: None,
        }
    }

    fn statements(text: &str) -> Vec<tabletist_db::sql::Statement> {
        tabletist_db::sql::statements(Dialect::Sqlite, text)
    }

    fn entries(
        text: &str,
        mode: ScriptMode,
        result: Result<tabletist_db::ScriptOutcome, Error>,
    ) -> Vec<Entry> {
        let script = statements(text);
        super::run_entries(&base(), Dialect::Sqlite, (&script, mode), &result, None)
    }

    #[test]
    fn a_refused_run_is_the_entry_of_the_statement_it_was_refused_for() {
        // Two on one line: the refused one is the one the refusal names.
        let refused = Err(Error::Refused {
            line: 2,
            what: "COMMIT".into(),
            mode: ScriptMode::ReadOnly,
        });
        let kept = entries(
            "SELECT 1;\nSELECT 2; COMMIT;",
            ScriptMode::ReadOnly,
            refused,
        );
        assert_eq!(kept.len(), 1);
        assert_eq!(kept[0].sql.as_deref(), Some("SELECT 2"));
        assert_eq!(kept[0].outcome, Outcome::Error);
    }

    #[test]
    fn a_run_lost_partway_keeps_each_statement_with_its_error() {
        let lost = Err(Error::ConnectionLost("closed".into()));
        let kept = entries(
            "UPDATE book_covers SET kind = 'ebook';\nSELECT 1;",
            ScriptMode::Write,
            lost,
        );
        assert_eq!(kept.len(), 2);
        assert!(kept.iter().all(|entry| entry.outcome == Outcome::Error));
    }

    #[test]
    fn a_write_that_was_rolled_back_says_so() {
        let text = "UPDATE book_covers SET kind = 'ebook';\nSELECT * FROM nowhere;";
        let mut outcome = script_outcome(vec![
            done_outcome(Some(5)),
            error_outcome("no such table: nowhere", None),
        ]);
        outcome.end = ScriptEnd::RolledBack;
        let kept = entries(text, ScriptMode::Write, Ok(outcome));
        assert_eq!(kept[0].outcome, Outcome::Ok);
        assert_eq!(kept[0].message.as_deref(), Some("rolled back"));
        assert_eq!(kept[1].outcome, Outcome::Error);
        // Committed, it is kept as it ran.
        let mut outcome = script_outcome(vec![done_outcome(Some(5)), rows_outcome(1)]);
        outcome.end = ScriptEnd::Committed;
        let kept = entries(text, ScriptMode::Write, Ok(outcome));
        assert_eq!(kept[0].message, None);
    }

    #[test]
    fn a_save_opens_without_its_begin_and_commit() {
        assert_eq!(
            super::unwrapped("BEGIN;\nUPDATE a SET b = 1;\nUPDATE a SET b = 2;\nCOMMIT;"),
            "UPDATE a SET b = 1;\nUPDATE a SET b = 2;"
        );
    }
}
