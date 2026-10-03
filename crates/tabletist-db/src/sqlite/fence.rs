//! The fences around text that is not the app's own. SQLite asks a
//! connection's authorizer whenever it prepares a statement, after its own
//! parse, so what a SQL editor script or a table's raw WHERE may do is
//! decided here on what SQLite read, not on what our tokenizer made of the
//! text. `Conn::open` installs the authorizer; a fence goes up around a
//! script's statement and around a page or count query, and is down for
//! everything the app runs itself.

use std::sync::Arc;
use std::sync::atomic::{AtomicU8, Ordering};

use rusqlite::ffi;
use rusqlite::hooks::{AuthAction, Authorization};

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
}

/// What SQLite may do for text behind `fence`. Asked after SQLite's own
/// parse, so no spelling gets past it; that is why it does not go through
/// `sql::refusal`, whose tokenizer and SQLite's do not always agree.
pub(super) fn authorize(fence: Fence, action: &AuthAction<'_>) -> Authorization {
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
        Fence::Filter => match action {
            AuthAction::Transaction { .. }
            | AuthAction::Savepoint { .. }
            | AuthAction::Attach { .. }
            | AuthAction::Detach { .. } => false,
            // ATTACH and DETACH again: SQLite hands over the name only when
            // it is a plain string, and without one rusqlite does not know
            // the action.
            AuthAction::Unknown { code, .. } => {
                !matches!(*code, ffi::SQLITE_ATTACH | ffi::SQLITE_DETACH)
            }
            AuthAction::Pragma {
                pragma_name,
                pragma_value,
            } => {
                pragma_value.is_none()
                    && !["wal_checkpoint", "incremental_vacuum"]
                        .iter()
                        .any(|name| pragma_name.eq_ignore_ascii_case(name))
            }
            _ => true,
        },
        // A write is left to `query_only`, whose error the app knows as a
        // refused write. What must not happen is the script leaving its
        // transaction or lifting what refuses the write.
        Fence::Script => match action {
            AuthAction::Transaction { .. } | AuthAction::Savepoint { .. } => false,
            // The name comes without its schema and unquoted, in the
            // letters the script wrote it in.
            AuthAction::Pragma {
                pragma_name,
                pragma_value,
            } => {
                let name = pragma_name.to_ascii_lowercase();
                name != "wal_checkpoint"
                    && !(pragma_value.is_some()
                        && matches!(name.as_str(), "query_only" | "writable_schema"))
            }
            _ => true,
        },
    };
    if allowed {
        Authorization::Allow
    } else {
        Authorization::Deny
    }
}

/// Which fence is up, shared with the authorizer SQLite calls.
#[derive(Clone, Default)]
pub(super) struct Fences(Arc<AtomicU8>);

impl Fences {
    /// Puts `fence` up until the returned value drops. Fences do not nest:
    /// the drop takes down whatever is up.
    pub(super) fn fence(&self, fence: Fence) -> Fenced<'_> {
        debug_assert_eq!(self.current(), Fence::Off, "fences do not nest");
        self.0.store(fence as u8, Ordering::SeqCst);
        Fenced(self)
    }

    /// The fence that is up, for the authorizer.
    pub(super) fn current(&self) -> Fence {
        match self.0.load(Ordering::SeqCst) {
            0 => Fence::Off,
            1 => Fence::Script,
            // Only the three are ever stored. A byte that is none of them
            // counts as the strictest: `Filter` denies all that `Script`
            // does, so nothing runs unfenced over it.
            _ => Fence::Filter,
        }
    }
}

/// Takes the fence down when dropped, on every path.
pub(super) struct Fenced<'a>(&'a Fences);

impl Drop for Fenced<'_> {
    fn drop(&mut self) {
        self.0.0.store(Fence::Off as u8, Ordering::SeqCst);
    }
}

#[cfg(test)]
mod tests {
    use rusqlite::hooks::TransactionOperation;

    use super::*;

    #[test]
    fn each_fence_allows_what_its_text_may_do() {
        let pragma = |pragma_name, pragma_value| AuthAction::Pragma {
            pragma_name,
            pragma_value,
        };
        let transaction = AuthAction::Transaction {
            operation: TransactionOperation::Unknown,
        };
        let savepoint = AuthAction::Savepoint {
            operation: TransactionOperation::Begin,
            savepoint_name: "s",
        };
        let read = AuthAction::Read {
            table_name: "users",
            column_name: "email",
        };
        let function = AuthAction::Function {
            function_name: "count",
        };
        let update = AuthAction::Update {
            table_name: "users",
            column_name: "email",
        };
        let insert = AuthAction::Insert {
            table_name: "users",
        };
        let delete = AuthAction::Delete {
            table_name: "users",
        };
        let attach = AuthAction::Attach {
            filename: "other.db",
        };
        let detach = AuthAction::Detach {
            database_name: "other",
        };
        // ATTACH or DETACH with a name that is not a plain string, as in
        // `ATTACH 'x' || '' AS y`: SQLite hands over no name, and without
        // one rusqlite does not know the action.
        let unnamed = |code| AuthAction::Unknown {
            code,
            arg1: None,
            arg2: None,
        };
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
            (Fence::Script, pragma("foreign_keys", Some("ON")), true),
            (Fence::Script, pragma("table_info", Some("users")), true),
            (Fence::Script, AuthAction::Select, true),
            (Fence::Script, update, true),
            (Fence::Script, attach, true),
            (Fence::Script, unnamed(ffi::SQLITE_ATTACH), true),
            (Fence::Filter, AuthAction::Select, true),
            (Fence::Filter, read, true),
            (Fence::Filter, function, true),
            (Fence::Filter, AuthAction::Recursive, true),
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
        fences.0.store(7, Ordering::SeqCst);
        assert_eq!(fences.current(), Fence::Filter);
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
