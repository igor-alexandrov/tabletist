//! The fences around text that is not the app's own. SQLite asks a
//! connection's authorizer whenever it prepares a statement, after its own
//! parse, so what a SQL editor script, a table's raw WHERE or an agent's
//! statement may do is decided here on what SQLite read, not on what our
//! tokenizer made of the text. `Conn::open` installs the authorizer; a
//! fence goes up around a script's statement, around a page or count query
//! and around an agent's statement, and is down for everything the app
//! runs itself.
//!
//! The authorizer is `tabletist-sqlite-ffi`'s, not rusqlite's, which panics
//! on a name that is not UTF-8, whatever the fence. Here such a name arrives
//! with U+FFFD for those bytes and is judged like any other. It cannot pass
//! for a name a fence refuses: those are ASCII, and so is every pragma's
//! name that SQLite knows.

use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::{Arc, Mutex, PoisonError};

use rusqlite::ffi;
use tabletist_sqlite_ffi::{Action, Authorization};

use crate::Dialect;
use crate::agent::{Hidden, Refusal, function_allowed};

/// Whose text SQLite is preparing, which decides what it may do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub(super) enum Fence {
    /// The app's own statements.
    Off = 0,
    /// A statement of a SQL editor script.
    Script = 1,
    /// A table's page or count, which holds the raw WHERE.
    Filter = 2,
    /// A statement an agent sent, which passed `agent::check_read`.
    Agent = 3,
}

/// What SQLite may do for text behind `fence`. Asked after SQLite's own
/// parse, so no spelling gets past it; that is why it does not go through
/// `sql::refusal`, whose tokenizer and SQLite's do not always agree.
pub(super) fn authorize(fence: Fence, action: &Action<'_>) -> Authorization {
    let allowed = match fence {
        Fence::Off => true,
        // A filter's text is one SELECT, and a statement hidden behind it
        // is only ever prepared, never run (rusqlite prepares the tail to
        // find it). So what must not happen is what takes effect when
        // SQLite prepares it, and a pragma with a value does. One without a
        // value changes no setting and no file when it is prepared, and
        // SQLite's own virtual tables ask for them (FTS5 for
        // `data_version`). Those that act when they are run are never run,
        // except `optimize` read as a table, whose write `query_only`
        // refuses; the two that would rewrite a file are denied by name all
        // the same. The write statements R*Tree prepares when a table
        // connects are not run by a SELECT either.
        Fence::Filter => match action.code {
            // ATTACH and DETACH by their code alone: SQLite hands over the
            // name only when it is a plain string.
            ffi::SQLITE_TRANSACTION
            | ffi::SQLITE_SAVEPOINT
            | ffi::SQLITE_ATTACH
            | ffi::SQLITE_DETACH => false,
            ffi::SQLITE_PRAGMA => {
                action.second.is_none()
                    && !["wal_checkpoint", "incremental_vacuum"]
                        .iter()
                        .any(|name| pragma_is(action, name))
            }
            _ => true,
        },
        // A write is left to `query_only`, whose error the app knows as a
        // refused write. What must not happen is the script leaving its
        // transaction or lifting what refuses the write. Nor may it take
        // the file for the session: an exclusive `locking_mode` outlives
        // the run, and in a rollback journal setting it back does not let
        // the lock go (only the session's next read of the file does), so
        // every other program would be shut out of the file meanwhile.
        // Inside the one transaction a script runs in it gains nothing.
        Fence::Script => match action.code {
            ffi::SQLITE_TRANSACTION | ffi::SQLITE_SAVEPOINT => false,
            ffi::SQLITE_PRAGMA => {
                !pragma_is(action, "wal_checkpoint")
                    && !(action.second.is_some()
                        && ["query_only", "writable_schema", "locking_mode"]
                            .iter()
                            .any(|name| pragma_is(action, name)))
            }
            _ => true,
        },
        // An agent's statement is one read, by a check that reads its
        // text. SQLite reads it again here, and what the two make of it
        // can differ: a view of the database's own can call a function or
        // read a pragma that the text never names. So this fence lists
        // what a read does and refuses the rest, where the other two list
        // what to refuse.
        Fence::Agent => match action.code {
            ffi::SQLITE_SELECT | ffi::SQLITE_READ | ffi::SQLITE_RECURSIVE => true,
            // The check's list of functions with effects, asked of the
            // names SQLite found.
            ffi::SQLITE_FUNCTION => action
                .second
                .is_some_and(|name| function_allowed(Dialect::Sqlite, name).is_ok()),
            // What FTS5 asks for as it reads, and no other pragma: one
            // read as a table (`pragma_table_info`) arrives here too, with
            // its argument as its value or with none.
            ffi::SQLITE_PRAGMA => action.second.is_none() && pragma_is(action, "data_version"),
            // The write statements R*Tree prepares when a table connects,
            // which a SELECT does not run. Running one is `query_only`'s
            // to refuse, as behind a filter.
            ffi::SQLITE_INSERT | ffi::SQLITE_UPDATE | ffi::SQLITE_DELETE => true,
            _ => false,
        },
    };
    if allowed {
        Authorization::Allow
    } else {
        Authorization::Deny
    }
}

/// Whether the pragma SQLite asks about is `name`. Its name comes without
/// its schema and unquoted, in the letters the text wrote it in.
fn pragma_is(action: &Action<'_>, name: &str) -> bool {
    action
        .first
        .is_some_and(|asked| asked.eq_ignore_ascii_case(name))
}

/// What an agent is told of an action its fence denied: a function by its
/// name, a pragma by its own, and the rest by what kind of statement asks
/// for it.
fn refusal(action: &Action<'_>) -> Refusal {
    let what = match (action.code, action.first, action.second) {
        (ffi::SQLITE_FUNCTION, _, Some(name)) => {
            return Refusal::FunctionNotAllowed(name.to_owned());
        }
        (ffi::SQLITE_FUNCTION, _, None) => "a function without a name".to_owned(),
        (ffi::SQLITE_PRAGMA, Some(name), _) => format!("PRAGMA {name}"),
        (ffi::SQLITE_PRAGMA, None, _) => "a PRAGMA".to_owned(),
        (ffi::SQLITE_ATTACH, ..) => "ATTACH".to_owned(),
        (ffi::SQLITE_DETACH, ..) => "DETACH".to_owned(),
        (ffi::SQLITE_TRANSACTION | ffi::SQLITE_SAVEPOINT, ..) => {
            "a transaction statement".to_owned()
        }
        _ => "a statement that is not a read".to_owned(),
    };
    Refusal::NotAllowed(what)
}

/// Which fence is up, shared with the authorizer SQLite calls, and what
/// an agent's fence denied since it went up.
#[derive(Clone, Default)]
pub(super) struct Fences(Arc<Shared>);

#[derive(Default)]
struct Shared {
    up: AtomicU8,
    refused: Mutex<Option<Refusal>>,
    /// What the agent's connection hides, while its statement runs.
    hidden: Mutex<Option<Arc<Hidden>>>,
    /// The tables read, while they are recorded.
    reads: Mutex<Option<Vec<TableRead>>>,
}

impl Fences {
    /// Puts `fence` up until the returned value drops. Fences do not nest:
    /// the drop takes down whatever is up.
    pub(super) fn fence(&self, fence: Fence) -> Fenced<'_> {
        debug_assert_eq!(self.current(), Fence::Off, "fences do not nest");
        if let Ok(mut refused) = self.0.refused.lock() {
            *refused = None;
        }
        self.0.up.store(fence as u8, Ordering::SeqCst);
        Fenced(self)
    }

    /// The fence that is up, for the authorizer.
    pub(super) fn current(&self) -> Fence {
        match self.0.up.load(Ordering::SeqCst) {
            0 => Fence::Off,
            1 => Fence::Script,
            2 => Fence::Filter,
            // Only the four are ever stored. A byte that is none of them
            // counts as the strictest: `Agent` denies all that `Filter`
            // does, and `Filter` all that `Script` does, so nothing runs
            // unfenced over it.
            _ => Fence::Agent,
        }
    }

    /// The authorizer's answer for the fence that is up. SQLite says only
    /// "not authorized" of a statement it refuses, so what an agent's
    /// fence denies is kept, the first of it, for the run to say.
    ///
    /// Behind an agent's fence a read of a table is also asked of what the
    /// connection hides. SQLite reports every relation a statement reads,
    /// after its own parse: a view's own name and then its tables', each
    /// with the view it is read for, a table that only `count(*)` or
    /// `EXISTS` reads (with an empty column), and its own `sqlite_schema`.
    /// So nothing hidden is read, however the text was written.
    pub(super) fn ask(&self, action: &Action<'_>) -> Authorization {
        let fence = self.current();
        if action.code == ffi::SQLITE_READ
            && let Some(table) = action.first
            && let Some(reads) = self.reads().as_mut()
        {
            reads.push(TableRead {
                schema: action.database.unwrap_or("main").to_owned(),
                table: table.to_owned(),
                column: action.second.unwrap_or_default().to_owned(),
                source: action.source.map(str::to_owned),
            });
        }
        let mut answer = authorize(fence, action);
        let mut hidden_read = None;
        if fence == Fence::Agent
            && answer == Authorization::Allow
            && action.code == ffi::SQLITE_READ
            && let Some(table) = action.first
            && let Some(hidden) = self.hidden()
            && let Err(refusal) = hidden.check_engine_read(action.database, table)
        {
            answer = Authorization::Deny;
            hidden_read = Some(through_view(refusal, action.source));
        }
        // A masked column the statement reads itself is read as `NULL`,
        // wherever the statement uses it: what the check of its text
        // missed reads nothing. A selected one is replaced as it is
        // answered, by where its result column comes from.
        if fence == Fence::Agent
            && answer == Authorization::Allow
            && action.code == ffi::SQLITE_READ
            && action.source.is_none()
            && let (Some(table), Some(column)) = (action.first, action.second)
            && let Some(hidden) = self.hidden()
            && hidden.masks(action.database, table, column).is_some()
        {
            return Authorization::Ignore;
        }
        // A masked column a view or a common table expression reads: what
        // it does with it, the statement's check cannot see.
        if fence == Fence::Agent
            && answer == Authorization::Allow
            && action.code == ffi::SQLITE_READ
            && let (Some(table), Some(column)) = (action.first, action.second)
            && let Some(hidden) = self.hidden()
            && let Err(refusal) =
                hidden.check_column_read(action.database, table, column, action.source)
        {
            answer = Authorization::Deny;
            hidden_read = Some(refusal);
        }
        if fence == Fence::Agent
            && answer == Authorization::Deny
            && let Ok(mut refused) = self.0.refused.lock()
        {
            refused.get_or_insert_with(|| hidden_read.unwrap_or_else(|| refusal(action)));
        }
        answer
    }

    /// Hides from an agent's statements what `hidden` hides, until the
    /// returned value drops.
    pub(super) fn hide(&self, hidden: Option<Arc<Hidden>>) -> Hiding<'_> {
        *self.held() = hidden;
        Hiding(self)
    }

    /// What is hidden now.
    fn hidden(&self) -> Option<Arc<Hidden>> {
        self.held().clone()
    }

    /// What is hidden, held. A lock that a panic left behind still holds
    /// it: what is hidden is never let go of by an error.
    fn held(&self) -> std::sync::MutexGuard<'_, Option<Arc<Hidden>>> {
        self.0.hidden.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// The tables SQLite reads as it prepares what `prepare` prepares, each
    /// with a column it reads of it: for a view, every table it reads,
    /// through the views it reads too. `None` where SQLite could not
    /// prepare it.
    pub(super) fn reads_of(&self, prepare: impl FnOnce() -> bool) -> Option<Vec<TableRead>> {
        *self.reads() = Some(Vec::new());
        let prepared = prepare();
        let reads = self.reads().take();
        prepared.then_some(reads).flatten()
    }

    fn reads(&self) -> std::sync::MutexGuard<'_, Option<Vec<TableRead>>> {
        self.0.reads.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// What an agent's fence denied since the last fence went up.
    pub(super) fn refused(&self) -> Option<Refusal> {
        self.0
            .refused
            .lock()
            .ok()
            .and_then(|refused| refused.clone())
    }
}

/// A hidden table read for a view the statement named: the view is named
/// with it, as the agent asked for the view. A common table expression is
/// such a source too, and is named the same way.
fn through_view(refusal: Refusal, source: Option<&str>) -> Refusal {
    match (refusal, source) {
        (
            Refusal::TableHidden {
                name,
                through: None,
                by,
            },
            Some(view),
        ) if !view.eq_ignore_ascii_case(&name) => Refusal::TableHidden {
            name,
            through: Some(view.to_owned()),
            by,
        },
        (refusal, _) => refusal,
    }
}

/// A read SQLite asked its authorizer about.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct TableRead {
    pub schema: String,
    pub table: String,
    /// Empty for a read of no column (`count(*)`).
    pub column: String,
    /// The view, trigger or common table expression it is read for, where
    /// it is not the statement's own.
    pub source: Option<String>,
}

/// Lets go of what is hidden when dropped, on every path.
pub(super) struct Hiding<'a>(&'a Fences);

impl Drop for Hiding<'_> {
    fn drop(&mut self) {
        *self.0.held() = None;
    }
}

/// Takes the fence down when dropped, on every path.
pub(super) struct Fenced<'a>(&'a Fences);

impl Drop for Fenced<'_> {
    fn drop(&mut self) {
        self.0.0.up.store(Fence::Off as u8, Ordering::SeqCst);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// An action on `main` from the statement's own text.
    fn action<'a>(code: i32, first: Option<&'a str>, second: Option<&'a str>) -> Action<'a> {
        Action {
            code,
            first,
            second,
            database: Some("main"),
            source: None,
        }
    }

    #[test]
    fn each_fence_allows_what_its_text_may_do() {
        let pragma = |name, value| action(ffi::SQLITE_PRAGMA, Some(name), value);
        let transaction = action(ffi::SQLITE_TRANSACTION, Some("BEGIN"), None);
        let savepoint = action(ffi::SQLITE_SAVEPOINT, Some("BEGIN"), Some("s"));
        let select = action(ffi::SQLITE_SELECT, None, None);
        let read = action(ffi::SQLITE_READ, Some("users"), Some("email"));
        let function = action(ffi::SQLITE_FUNCTION, None, Some("count"));
        let recursive = action(ffi::SQLITE_RECURSIVE, None, None);
        let update = action(ffi::SQLITE_UPDATE, Some("users"), Some("email"));
        let insert = action(ffi::SQLITE_INSERT, Some("users"), None);
        let delete = action(ffi::SQLITE_DELETE, Some("users"), None);
        let attach = action(ffi::SQLITE_ATTACH, Some("other.db"), None);
        let detach = action(ffi::SQLITE_DETACH, Some("other"), None);
        // ATTACH or DETACH with a name that is not a plain string, as in
        // `ATTACH 'x' || '' AS y`: SQLite hands over no name.
        let unnamed = |code| action(code, None, None);
        // Names that are not UTF-8, as they arrive.
        let lossy_read = action(ffi::SQLITE_READ, Some("t"), Some("caf\u{FFFD}"));
        let table = [
            (Fence::Off, pragma("query_only", Some("0")), true),
            (Fence::Off, transaction, true),
            (Fence::Script, transaction, false),
            (Fence::Script, savepoint, false),
            (Fence::Script, pragma("query_only", Some("0")), false),
            // SQLite hands the name over as it was written.
            (Fence::Script, pragma("QUERY_ONLY", Some("OFF")), false),
            (Fence::Script, pragma("writable_schema", Some("ON")), false),
            (Fence::Script, pragma("wal_checkpoint", None), false),
            (
                Fence::Script,
                pragma("wal_checkpoint", Some("TRUNCATE")),
                false,
            ),
            (Fence::Script, pragma("query_only", None), true),
            // The file's lock is not a script's to keep, in whatever
            // letters; asking what the mode is changes nothing.
            (
                Fence::Script,
                pragma("locking_mode", Some("EXCLUSIVE")),
                false,
            ),
            (Fence::Script, pragma("LOCKING_MODE", Some("normal")), false),
            (Fence::Script, pragma("locking_mode", None), true),
            // A script may set these for its own run: the session's
            // settings are put back after it (`set_session_pragmas`).
            (
                Fence::Script,
                pragma("ignore_check_constraints", Some("ON")),
                true,
            ),
            (
                Fence::Script,
                pragma("recursive_triggers", Some("ON")),
                true,
            ),
            (
                Fence::Script,
                pragma("legacy_alter_table", Some("ON")),
                true,
            ),
            (Fence::Script, pragma("foreign_keys", Some("ON")), true),
            (Fence::Script, pragma("table_info", Some("users")), true),
            (Fence::Script, select, true),
            (Fence::Script, update, true),
            (Fence::Script, attach, true),
            (Fence::Script, unnamed(ffi::SQLITE_ATTACH), true),
            (Fence::Script, lossy_read, true),
            (Fence::Script, pragma("caf\u{FFFD}", Some("1")), true),
            (Fence::Filter, select, true),
            (Fence::Filter, read, true),
            (Fence::Filter, function, true),
            (Fence::Filter, recursive, true),
            // With a value, whatever the name: a table-valued pragma's
            // argument arrives as one too.
            (Fence::Filter, pragma("foreign_keys", Some("0")), false),
            (Fence::Filter, pragma("query_only", Some("0")), false),
            (Fence::Filter, pragma("table_info", Some("users")), false),
            (Fence::Filter, transaction, false),
            (Fence::Filter, savepoint, false),
            (Fence::Filter, attach, false),
            (Fence::Filter, detach, false),
            (Fence::Filter, unnamed(ffi::SQLITE_ATTACH), false),
            (Fence::Filter, unnamed(ffi::SQLITE_DETACH), false),
            // The two that would rewrite a file if they were run, by name
            // and in whatever letters.
            (Fence::Filter, pragma("wal_checkpoint", None), false),
            (Fence::Filter, pragma("WAL_Checkpoint", None), false),
            (Fence::Filter, pragma("incremental_vacuum", None), false),
            (Fence::Filter, pragma("INCREMENTAL_VACUUM", None), false),
            // What FTS5 asks for while it reads.
            (Fence::Filter, pragma("data_version", None), true),
            (Fence::Filter, pragma("query_only", None), true),
            // What R*Tree prepares when a table connects; running one is
            // `query_only`'s to refuse.
            (Fence::Filter, insert, true),
            (Fence::Filter, update, true),
            (Fence::Filter, delete, true),
            (Fence::Filter, lossy_read, true),
            (Fence::Filter, pragma("caf\u{FFFD}", None), true),
            (Fence::Filter, pragma("caf\u{FFFD}", Some("1")), false),
        ];
        for (fence, action, allowed) in table {
            let expected = if allowed {
                Authorization::Allow
            } else {
                Authorization::Deny
            };
            assert_eq!(authorize(fence, &action), expected, "{fence:?} {action:?}");
        }
        // `Fences::current` counts on this: what a script may not do, a
        // filter may not do either.
        for (fence, action, allowed) in table {
            if fence == Fence::Script && !allowed {
                assert_eq!(
                    authorize(Fence::Filter, &action),
                    Authorization::Deny,
                    "{action:?}"
                );
            }
        }
    }

    #[test]
    fn an_agents_statement_may_only_read() {
        let pragma = |name, value| action(ffi::SQLITE_PRAGMA, Some(name), value);
        let function = |name| action(ffi::SQLITE_FUNCTION, None, Some(name));
        let through_a_view = Action {
            source: Some("recent_orders"),
            ..action(ffi::SQLITE_READ, Some("orders"), Some("email"))
        };
        let table = [
            (action(ffi::SQLITE_SELECT, None, None), true),
            (action(ffi::SQLITE_READ, Some("books"), Some("title")), true),
            (through_a_view, true),
            (action(ffi::SQLITE_RECURSIVE, None, None), true),
            (function("count"), true),
            (function("json_extract"), true),
            // The check's list for SQLite, asked of what SQLite read: in
            // whatever letters, and from inside a view too.
            (function("load_extension"), false),
            (function("LOAD_EXTENSION"), false),
            (function("fts3_tokenizer"), false),
            (function("readfile"), false),
            (action(ffi::SQLITE_FUNCTION, None, None), false),
            // What FTS5 asks as it reads, and no other pragma: not one
            // without a value, which a filter may read.
            (pragma("data_version", None), true),
            (pragma("data_version", Some("1")), false),
            (pragma("table_info", Some("customers")), false),
            (pragma("database_list", None), false),
            (pragma("optimize", None), false),
            (pragma("integrity_check", None), false),
            (pragma("query_only", None), false),
            // What R*Tree prepares when a table connects. Running one is
            // `query_only`'s to refuse.
            (action(ffi::SQLITE_INSERT, Some("places_node"), None), true),
            (action(ffi::SQLITE_DELETE, Some("places_node"), None), true),
            (
                action(ffi::SQLITE_UPDATE, Some("places_node"), Some("data")),
                true,
            ),
            // Everything else, by not being on the list.
            (action(ffi::SQLITE_TRANSACTION, Some("BEGIN"), None), false),
            (
                action(ffi::SQLITE_SAVEPOINT, Some("BEGIN"), Some("s")),
                false,
            ),
            (action(ffi::SQLITE_ATTACH, Some("other.db"), None), false),
            (action(ffi::SQLITE_DETACH, Some("other"), None), false),
            (action(ffi::SQLITE_CREATE_TABLE, Some("t"), None), false),
            (action(ffi::SQLITE_CREATE_TEMP_VIEW, Some("v"), None), false),
            (action(ffi::SQLITE_DROP_TABLE, Some("books"), None), false),
            (
                action(ffi::SQLITE_ALTER_TABLE, Some("main"), Some("books")),
                false,
            ),
            (action(ffi::SQLITE_ANALYZE, Some("books"), None), false),
            (
                action(ffi::SQLITE_REINDEX, Some("books_title"), None),
                false,
            ),
            (
                action(ffi::SQLITE_CREATE_VTABLE, Some("t"), Some("fts5")),
                false,
            ),
            // A code SQLite does not have yet.
            (action(9_999, None, None), false),
        ];
        for (action, allowed) in table {
            let expected = if allowed {
                Authorization::Allow
            } else {
                Authorization::Deny
            };
            assert_eq!(authorize(Fence::Agent, &action), expected, "{action:?}");
            // `Fences::current` counts on this: what a filter may not do,
            // an agent's statement may not do either.
            if authorize(Fence::Filter, &action) == Authorization::Deny {
                assert_eq!(expected, Authorization::Deny, "{action:?}");
            }
        }
    }

    #[test]
    fn what_an_agents_statement_was_denied_is_kept_for_the_run_to_say() {
        let fences = Fences::default();
        let asked = fences.clone();
        let pragma = action(ffi::SQLITE_PRAGMA, Some("table_info"), Some("customers"));
        let function = action(ffi::SQLITE_FUNCTION, None, Some("load_extension"));
        let read = action(ffi::SQLITE_READ, Some("books"), Some("title"));
        {
            let _fenced = fences.fence(Fence::Agent);
            assert_eq!(asked.ask(&read), Authorization::Allow);
            assert_eq!(fences.refused(), None);
            assert_eq!(asked.ask(&pragma), Authorization::Deny);
            // The first denial is the one that is told.
            assert_eq!(asked.ask(&function), Authorization::Deny);
            assert_eq!(
                fences.refused(),
                Some(Refusal::NotAllowed("PRAGMA table_info".into()))
            );
        }
        // A fence that goes up starts with nothing denied.
        {
            let _fenced = fences.fence(Fence::Agent);
            assert_eq!(fences.refused(), None);
            assert_eq!(asked.ask(&function), Authorization::Deny);
            assert_eq!(
                fences.refused(),
                Some(Refusal::FunctionNotAllowed("load_extension".into()))
            );
        }
        // Only an agent's fence keeps one: a script's denial is SQLite's
        // own error, as before.
        let _fenced = fences.fence(Fence::Script);
        let begin = action(ffi::SQLITE_TRANSACTION, Some("BEGIN"), None);
        assert_eq!(asked.ask(&begin), Authorization::Deny);
        assert_eq!(fences.refused(), None);
    }

    #[test]
    fn a_masked_column_the_statement_reads_is_read_as_null_and_one_a_view_reads_is_refused() {
        use crate::agent::{Catalog, ColumnName, Relation, Tables};
        let catalog = Catalog {
            relations: vec![Relation {
                schema: "main".into(),
                name: "orders".into(),
                depends_on: Vec::new(),
                opaque: false,
                part: false,
                view: false,
            }],
            columns: vec![("main".into(), "orders".into(), "email".into())],
            ..Catalog::default()
        };
        let masked: Vec<ColumnName> = vec!["orders.email".parse().unwrap()];
        let hidden = Hidden::masking(Dialect::Sqlite, &catalog, &Tables::All, &masked);
        let fences = Fences::default();
        let _hiding = fences.hide(Some(Arc::new(hidden)));
        let email = action(ffi::SQLITE_READ, Some("orders"), Some("EMAIL"));
        let id = action(ffi::SQLITE_READ, Some("orders"), Some("id"));
        let for_view = Action {
            source: Some("recent_orders"),
            ..email
        };
        let _fenced = fences.fence(Fence::Agent);
        assert_eq!(fences.ask(&email), Authorization::Ignore);
        assert_eq!(fences.ask(&id), Authorization::Allow);
        assert_eq!(fences.refused(), None);
        assert_eq!(fences.ask(&for_view), Authorization::Deny);
        assert_eq!(
            fences.refused(),
            Some(Refusal::MaskedColumnUsed {
                column: "orders.email".into(),
                how: crate::agent::MaskedUse::Through("recent_orders".into()),
                by: crate::agent::By::Person,
            })
        );
    }

    #[test]
    fn a_denial_is_named_by_what_sqlite_was_asked() {
        let named = |code, first, second| refusal(&action(code, first, second));
        let not_allowed = |what: &str| Refusal::NotAllowed(what.into());
        assert_eq!(
            named(ffi::SQLITE_FUNCTION, None, Some("load_extension")),
            Refusal::FunctionNotAllowed("load_extension".into())
        );
        assert_eq!(
            named(ffi::SQLITE_PRAGMA, Some("table_info"), Some("customers")),
            not_allowed("PRAGMA table_info")
        );
        assert_eq!(
            named(ffi::SQLITE_ATTACH, Some("other.db"), None),
            not_allowed("ATTACH")
        );
        assert_eq!(
            named(ffi::SQLITE_DETACH, Some("other"), None),
            not_allowed("DETACH")
        );
        for code in [ffi::SQLITE_TRANSACTION, ffi::SQLITE_SAVEPOINT] {
            assert_eq!(
                named(code, Some("BEGIN"), None),
                not_allowed("a transaction statement")
            );
        }
        // Anything else is a statement that is no read, whatever it is.
        for code in [ffi::SQLITE_CREATE_TABLE, ffi::SQLITE_ANALYZE, 9_999] {
            assert_eq!(
                named(code, Some("t"), None),
                not_allowed("a statement that is not a read")
            );
        }
        // A name SQLite did not give.
        assert_eq!(
            named(ffi::SQLITE_PRAGMA, None, None),
            not_allowed("a PRAGMA")
        );
        assert_eq!(
            named(ffi::SQLITE_FUNCTION, None, None),
            not_allowed("a function without a name")
        );
    }

    #[test]
    fn a_fence_is_up_until_its_guard_drops() {
        let fences = Fences::default();
        let asked = fences.clone();
        assert_eq!(asked.current(), Fence::Off);
        {
            let _fenced = fences.fence(Fence::Script);
            assert_eq!(asked.current(), Fence::Script);
        }
        assert_eq!(asked.current(), Fence::Off);
    }

    #[test]
    fn a_byte_that_is_no_fence_counts_as_the_strictest_one() {
        let fences = Fences::default();
        fences.0.up.store(7, Ordering::SeqCst);
        assert_eq!(fences.current(), Fence::Agent);
    }

    // Only a debug build checks it.
    #[test]
    #[cfg(debug_assertions)]
    #[should_panic(expected = "fences do not nest")]
    fn a_fence_inside_another_is_a_bug() {
        let fences = Fences::default();
        let _script = fences.fence(Fence::Script);
        let _filter = fences.fence(Fence::Filter);
    }
}
