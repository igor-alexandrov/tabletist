//! The commands quick open runs and the menu bar offers (macOS, Windows):
//! each a command of the keymap's, with the name quick open lists it by,
//! what it says of where it acts, and what running it does. A command
//! lists only where it applies now: a tab's commands with that tab in
//! front, a save with something to save. The terminal look's commands stay
//! on `:`.

use std::cell::RefCell;

use tabletist_db::Dialect;

use crate::keymap::Command;
use crate::model::{About, Action, ConnTabId, RunMode, Runnable, Tab, TabName};

use super::App;

/// Whether a script has a statement, for the script last asked about.
/// Run All applies only where one has, and the menu bar asks after every
/// frame: tokenizing a script of 5,000 lines takes over a millisecond in
/// a release build, and its text changes only on edits.
#[derive(Default)]
pub struct Scripts(RefCell<Option<Asked>>);

/// A script asked about, and the answer.
struct Asked {
    dialect: Dialect,
    text: String,
    runs: bool,
}

#[cfg(test)]
thread_local! {
    /// How many times this thread tokenized a script for Run All.
    static TOKENIZED: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

impl Scripts {
    /// Whether `text`, in `dialect`, has a statement to run: comments
    /// alone run nothing.
    pub fn runs(&self, dialect: Dialect, text: &str) -> bool {
        let mut asked = self.0.borrow_mut();
        // The dialect is part of the cache's key, not a decision per
        // engine: AGENTS.md's rule against `==` on `Dialect` is about the
        // latter.
        if let Some(asked) = asked.as_ref()
            && asked.dialect == dialect
            && asked.text == text
        {
            return asked.runs;
        }
        #[cfg(test)]
        TOKENIZED.with(|count| count.set(count.get() + 1));
        let runs = !tabletist_db::sql::statements(dialect, text).is_empty();
        match asked.as_mut() {
            // The last script's room, for the next.
            Some(asked) => {
                asked.dialect = dialect;
                asked.text.clear();
                asked.text.push_str(text);
                asked.runs = runs;
            }
            None => {
                *asked = Some(Asked {
                    dialect,
                    text: text.to_owned(),
                    runs,
                });
            }
        }
        runs
    }
}

/// The commands quick open can run, in the order it lists them: the
/// window's, then the tab's in front. Each is built, and does here what
/// its key does in the window, but three: quick open's Reload reloads the
/// schema, which the key does with the keyboard in the sidebar (on a
/// table's rows it reloads them); its Review pending SQL shows the
/// review, which the key shows and hides by turns; and its Agents shows
/// the Agents tab, which the key does but where History has an agent's
/// statement picked (there it shows that call).
pub const RUNNABLE: [Command; 30] = [
    Command::NewSqlTab,
    Command::SwitchDatabase,
    Command::ShowAgents,
    Command::History,
    Command::Reload,
    Command::EditConnection,
    Command::CloseConnection,
    Command::NextConnection,
    Command::PreviousConnection,
    Command::Connections,
    Command::NewConnection,
    Command::ToggleSidebar,
    Command::Settings,
    Command::CheckForUpdates,
    Command::Help,
    Command::LargerText,
    Command::SmallerText,
    Command::DefaultTextSize,
    Command::CloseTab,
    Command::NextTab,
    Command::PreviousTab,
    Command::FilterBar,
    Command::ToggleInspector,
    Command::ReviewSql,
    Command::SaveChanges,
    Command::DiscardChanges,
    Command::RunAll,
    Command::Format,
    Command::ToggleSqlMode,
    Command::CancelQuery,
];

impl TabName {
    fn of(tab: &Tab) -> Self {
        match tab {
            Tab::Object(object) => Self::Object(object.object.name.clone()),
            Tab::Sql(sql) => Self::Query(sql.number),
            Tab::Agents(_) => Self::Agents,
            Tab::History(_) => Self::History,
        }
    }
}

impl App {
    /// The commands quick open offers in `tab`'s window, in the order of
    /// [`RUNNABLE`], each where it applies now. None in the terminal look.
    pub fn runnable(&self, tab: ConnTabId) -> Vec<Runnable> {
        if self.look.terminal {
            return Vec::new();
        }
        RUNNABLE
            .into_iter()
            .filter_map(|command| self.run_of(tab, command).map(|(runnable, _)| runnable))
            .collect()
    }

    /// What running `command` in `tab`'s window does: the actions its key
    /// pushes there ([`RUNNABLE`] says where not). None where it does not
    /// apply now.
    pub(super) fn command_actions(&self, tab: ConnTabId, command: Command) -> Option<Vec<Action>> {
        self.run_of(tab, command).map(|(_, actions)| actions)
    }

    /// `command` as quick open offers it in `tab`'s window, and what
    /// running it does; None where it does not apply now.
    fn run_of(&self, tab: ConnTabId, command: Command) -> Option<(Runnable, Vec<Action>)> {
        if let Some((name, about, actions)) = self.window_run(command) {
            return Some((
                Runnable {
                    command,
                    name,
                    about,
                },
                actions,
            ));
        }
        let workspace = self.workspace(tab)?;
        // The tab in front, and the object or SQL editor it is.
        let front = workspace.active_tab.and_then(|id| workspace.tab(id));
        let object = front.and_then(Tab::as_object);
        let sql = front.and_then(Tab::as_sql);
        // Changes to save or drop: none while a save is under way, which
        // the reducers refuse to touch.
        let pending =
            object.filter(|object| object.edits.pending() && object.edits.saving.is_none());
        let in_front = || front.map(|tab| About::Tab(TabName::of(tab)));
        let (name, about, actions) = match command {
            Command::NewSqlTab => {
                let read_only = workspace.environment.is_production()
                    && (self.settings.new_tab == crate::settings::NewTab::ReadOnly
                        || RunMode::of_new_tab(workspace.sql_writes(), workspace.environment)
                            == RunMode::ReadOnly);
                let about = if read_only {
                    About::OpensReadOnly
                } else {
                    About::Nothing
                };
                ("New SQL query", about, vec![Action::NewSqlTab(tab)])
            }
            // Where the server has another database to switch to.
            Command::SwitchDatabase => {
                let databases = workspace.databases.value.as_ref();
                databases.filter(|databases| databases.len() > 1)?;
                (
                    "Switch database",
                    About::Nothing,
                    vec![Action::ShowDatabaseMenu(tab)],
                )
            }
            // Where agents can reach the app.
            Command::ShowAgents => {
                if !crate::endpoint::AVAILABLE {
                    return None;
                }
                ("Agents", About::Agents, vec![Action::ShowAgents(tab)])
            }
            Command::History => ("History", About::Connection, vec![Action::ShowHistory(tab)]),
            Command::Reload => (
                "Reload schema",
                About::Connection,
                vec![Action::RefreshTree(tab)],
            ),
            Command::EditConnection => {
                self.connections.get(&workspace.conn_id)?;
                let edit = Action::EditConnection(workspace.conn_id.clone());
                ("Edit connection", About::Connection, vec![edit])
            }
            Command::CloseConnection => (
                "Close connection",
                About::Connection,
                vec![Action::CloseConnTab(tab)],
            ),
            // Where there is another window to go to.
            Command::NextConnection | Command::PreviousConnection => {
                if self.tabs.len() < 2 {
                    return None;
                }
                let (name, step) = match command {
                    Command::NextConnection => ("Next connection", 1),
                    _ => ("Previous connection", -1),
                };
                (name, About::Nothing, vec![Action::CycleConnTab(step)])
            }
            Command::ToggleSidebar => {
                let name = if workspace.sidebar_hidden {
                    "Show sidebar"
                } else {
                    "Hide sidebar"
                };
                (name, About::Nothing, vec![Action::ToggleSidebar(tab)])
            }
            // The tab in front's.
            Command::CloseTab => {
                let id = front?.id();
                ("Close tab", in_front()?, vec![Action::CloseTab { tab, id }])
            }
            Command::NextTab | Command::PreviousTab => {
                if workspace.tabs.len() < 2 {
                    return None;
                }
                let (name, step) = match command {
                    Command::NextTab => ("Next tab", 1),
                    _ => ("Previous tab", -1),
                };
                (name, About::Nothing, vec![Action::CycleTab { tab, step }])
            }
            // A table's rows: its Structure view has none to filter. An
            // open bar takes the keyboard back.
            Command::FilterBar => {
                let object =
                    object.filter(|object| object.view == crate::model::ObjectView::Data)?;
                let filter = if object.filter.open {
                    Action::FocusFilterBar(tab)
                } else {
                    Action::ToggleFilterBar(tab)
                };
                ("Filter rows", in_front()?, vec![filter])
            }
            // Where it has a row to show: a table's rows, or the row
            // selected in a SQL editor's result.
            Command::ToggleInspector => {
                let data =
                    object.is_some_and(|object| object.view == crate::model::ObjectView::Data);
                let row = sql.is_some_and(|sql| sql.selected_row().is_some());
                if !data && !row {
                    return None;
                }
                let name = if workspace.row_panel {
                    "Hide row panel"
                } else {
                    "Show row panel"
                };
                (name, in_front()?, vec![Action::ToggleRowPanel(tab)])
            }
            // A table's pending changes: shown, saved or dropped.
            Command::ReviewSql => {
                let object = pending.filter(|object| !object.edits.reviewing)?;
                let id = object.id;
                let review = Action::ReviewEdits {
                    tab,
                    id,
                    show: true,
                };
                ("Review pending SQL", in_front()?, vec![review])
            }
            Command::SaveChanges => {
                let id = pending?.id;
                (
                    "Save changes",
                    in_front()?,
                    vec![Action::WriteEdits { tab, id }],
                )
            }
            Command::DiscardChanges => {
                let id = pending?.id;
                (
                    "Discard all pending",
                    in_front()?,
                    vec![Action::DiscardEdits { tab, id }],
                )
            }
            // A SQL editor's: a script to run or format, and its runs'
            // mode where the connection allows writes.
            Command::RunAll => {
                // The cheap checks first: the script is tokenized last.
                let about = in_front()?;
                let sql = sql.filter(|sql| !sql.is_writing())?;
                // A script with a statement in it: comments alone run
                // nothing.
                if !self.scripts.runs(workspace.driver.dialect(), &sql.text) {
                    return None;
                }
                let run = Action::RunSql {
                    tab,
                    sql_tab: sql.id,
                    all: true,
                };
                ("Run all", about, vec![run])
            }
            Command::Format => {
                let sql = sql.filter(|sql| !sql.text.trim().is_empty())?;
                let format = Action::FormatSql {
                    tab,
                    sql_tab: sql.id,
                };
                ("Format SQL", in_front()?, vec![format])
            }
            Command::ToggleSqlMode => {
                let sql = sql.filter(|_| workspace.sql_writes().is_ok())?;
                let name = match sql.mode {
                    RunMode::ReadOnly => "Read-write tab",
                    RunMode::ReadWrite => "Read-only tab",
                };
                let toggle = Action::ToggleSqlMode {
                    tab,
                    sql_tab: sql.id,
                };
                (name, in_front()?, vec![toggle])
            }
            // While something of the window loads or runs.
            Command::CancelQuery => {
                self.is_busy(tab).then_some(())?;
                (
                    "Cancel running query",
                    About::Connection,
                    vec![Action::CancelQuery(tab)],
                )
            }
            _ => return None,
        };
        let runnable = Runnable {
            command,
            name,
            about,
        };
        Some((runnable, actions))
    }

    /// The commands of the window, not of a connection: they apply on the
    /// connection picker too, which has no workspace. They have no arm in
    /// `run_of`'s match, which asks here first: None here is None there.
    fn window_run(&self, command: Command) -> Option<(&'static str, About, Vec<Action>)> {
        let stepped = |larger| {
            let terminal = self.look.terminal;
            let settings = &self.settings;
            settings.stepped_font_size(terminal, larger) != settings.font_size_in(terminal)
        };
        let run = match command {
            Command::Connections => ("Connections", About::Nothing, vec![Action::ShowConnections]),
            Command::NewConnection => (
                "New connection",
                About::Nothing,
                vec![Action::NewConnection],
            ),
            Command::Settings => ("Settings", About::Nothing, vec![Action::ShowSettings]),
            Command::CheckForUpdates => (
                "Check for Updates…",
                About::Nothing,
                vec![Action::ShowUpdates],
            ),
            Command::Help => ("Keyboard shortcuts", About::Nothing, vec![Action::ShowHelp]),
            // Each where it changes the size: not at the end of the steps,
            // nor back to the look's own where it is that already.
            Command::LargerText => {
                stepped(true).then_some(())?;
                (
                    "Larger text",
                    About::Nothing,
                    vec![Action::TextSize(Some(true))],
                )
            }
            Command::SmallerText => {
                stepped(false).then_some(())?;
                (
                    "Smaller text",
                    About::Nothing,
                    vec![Action::TextSize(Some(false))],
                )
            }
            Command::DefaultTextSize => {
                let terminal = self.look.terminal;
                let default = crate::settings::FontSizes::of(terminal).default;
                if self.settings.font_size_in(terminal) == default {
                    return None;
                }
                (
                    "Default text size",
                    About::Nothing,
                    vec![Action::TextSize(None)],
                )
            }
            _ => return None,
        };
        Some(run)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_script_is_tokenized_again_only_when_its_text_or_dialect_changes() {
        let scripts = Scripts::default();
        let tokenized = || TOKENIZED.with(std::cell::Cell::get);
        let start = tokenized();
        // `#` starts a comment in MySQL only: elsewhere it is code.
        assert!(!scripts.runs(Dialect::MySql, "# note"));
        assert_eq!(tokenized(), start + 1);
        // Asked again, the answer is the one kept.
        assert!(!scripts.runs(Dialect::MySql, "# note"));
        assert_eq!(tokenized(), start + 1);
        // The same text in another dialect is another question.
        assert!(scripts.runs(Dialect::Postgres, "# note"));
        assert_eq!(tokenized(), start + 2);
        // An edit is tokenized once.
        assert!(scripts.runs(Dialect::Postgres, "# note\nselect 1"));
        assert!(scripts.runs(Dialect::Postgres, "# note\nselect 1"));
        assert_eq!(tokenized(), start + 3);
        assert!(!scripts.runs(Dialect::Postgres, "-- only a comment"));
        assert_eq!(tokenized(), start + 4);
    }
}
