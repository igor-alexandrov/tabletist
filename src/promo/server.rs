//! The server a promotional video's save is answered from, where its
//! scenario says what someone else changed meanwhile: a copy of the
//! scenario's file with that change written in. A save is the real one,
//! run on the copy, and finds the rows the change touched changed on the
//! server. Only that answer is given: a save that would write is refused,
//! since the file the video loads is never written.

use std::path::Path;

use tabletist_db::{
    Access, ChangeSet, ConnectSpec, Connection, Dialect, HostKeys, ScriptEnd, ScriptMode, Secrets,
    StatementOutcome, StopFlag, WriteOutcome,
};

/// The copy's writable session.
pub struct Server {
    connection: Connection,
}

impl Server {
    /// Writes a copy of the file at `path` to `copy` and runs `sql` on the
    /// copy, as someone else would on the server once the video's rows
    /// were loaded. Every statement has to change a row. Why not, where it
    /// does not.
    pub fn stage(
        run: &tokio::runtime::Runtime,
        path: &Path,
        copy: &Path,
        sql: &str,
    ) -> Result<Self, String> {
        std::fs::copy(path, copy)
            .map_err(|error| format!("{} is not copied: {error}", path.display()))?;
        let connection = run
            .block_on(Connection::connect_with(
                &ConnectSpec::sqlite(copy),
                &Secrets::default(),
                &HostKeys::default(),
                Access::Writable,
            ))
            .map_err(|error| format!("the copy does not open: {error}"))?;
        let statements = tabletist_db::sql::statements(Dialect::Sqlite, sql);
        let ran = run
            .block_on(connection.run_script(&statements, 0, ScriptMode::Write, &StopFlag::new()))
            .map_err(|error| format!("meanwhile does not run: {error}"))?;
        for (number, result) in ran.results.iter().enumerate() {
            match &result.outcome {
                StatementOutcome::Done {
                    affected: Some(1..),
                    ..
                } => {}
                StatementOutcome::Error { error, .. } => {
                    return Err(format!(
                        "meanwhile's statement {} failed: {error}",
                        number + 1
                    ));
                }
                StatementOutcome::Done { .. }
                | StatementOutcome::Rows { .. }
                | StatementOutcome::Cancelled => {
                    return Err(format!(
                        "meanwhile's statement {} changed no row",
                        number + 1
                    ));
                }
            }
        }
        if ran.end != ScriptEnd::Committed {
            return Err(format!(
                "meanwhile is not written to the copy: {:?}",
                ran.end
            ));
        }
        Ok(Self { connection })
    }

    /// What the server answers a save of `changes` with: the rows it found
    /// changed. Anything else it says is an error, which names it.
    pub fn answer(
        &self,
        run: &tokio::runtime::Runtime,
        changes: &ChangeSet,
    ) -> Result<WriteOutcome, String> {
        match run.block_on(self.connection.write(changes, &StopFlag::new())) {
            Ok(WriteOutcome::Conflicts(conflicts)) => Ok(WriteOutcome::Conflicts(conflicts)),
            other => Err(format!(
                "the save found no row changed meanwhile, and only a conflict is answered: \
                 the server said {}",
                said(&other)
            )),
        }
    }
}

/// How a save that found no conflict ended, in a few words: its error, or
/// its outcome's kind. A row's values are left out.
fn said(result: &tabletist_db::Result<WriteOutcome>) -> String {
    match result {
        Err(error) => error.to_string(),
        Ok(WriteOutcome::Written { .. }) => "written, on the copy".into(),
        Ok(WriteOutcome::Conflicts(_)) => "conflicts".into(),
        Ok(
            WriteOutcome::Failed { error, .. }
            | WriteOutcome::FailedInsert { error, .. }
            | WriteOutcome::FailedDelete { error, .. },
        ) => format!("failed: {error}"),
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use std::path::{Path, PathBuf};

    use tabletist_db::{
        CellChange, ChangeSet, ConnectSpec, Connection, Dialect, NewValue, ObjectRef, RowChange,
        ScriptMode, Secrets, StatementOutcome, StopFlag, Value, WriteOutcome,
    };

    use super::Server;

    /// The Bookshop changes The Salt Road's price from 18.5 to 17 here.
    const MEANWHILE: &str = "UPDATE books SET price = 17.00 WHERE id = 1";

    fn runtime() -> tokio::runtime::Runtime {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
    }

    /// The Bookshop, written into `dir`.
    fn bookshop(dir: &Path) -> PathBuf {
        let path = dir.join("bookshop.sqlite");
        tabletist_db::fixtures::write_sqlite_bookshop(&path).unwrap();
        path
    }

    /// A save of the book `id`'s price, loaded as `loaded`, as `new`.
    fn price(id: i64, loaded: Value, new: &str) -> ChangeSet {
        ChangeSet {
            object: ObjectRef::new("main", "books"),
            inserts: Vec::new(),
            rows: vec![RowChange {
                key: vec![("id".into(), Value::Int(id))],
                set: vec![CellChange {
                    column: "price".into(),
                    type_name: "NUMERIC(8, 2)".into(),
                    loaded,
                    new: NewValue::Text(new.into()),
                }],
            }],
            deletes: Vec::new(),
        }
    }

    /// The price of the book `id` as the file at `path` holds it.
    fn held(run: &tokio::runtime::Runtime, path: &Path, id: i64) -> Value {
        let spec = ConnectSpec::sqlite(path);
        let connection = run
            .block_on(Connection::connect(&spec, &Secrets::default()))
            .unwrap();
        let select = format!("SELECT price FROM books WHERE id = {id}");
        let statements = tabletist_db::sql::statements(Dialect::Sqlite, &select);
        let ran = run
            .block_on(connection.run_script(&statements, 1, ScriptMode::ReadOnly, &StopFlag::new()))
            .unwrap();
        match &ran.results[0].outcome {
            StatementOutcome::Rows { rows, .. } => rows[0][0].clone(),
            other => panic!("not rows: {other:?}"),
        }
    }

    #[test]
    fn a_save_finds_the_row_changed_meanwhile_and_neither_file_is_written() {
        let dir = tempfile::tempdir().unwrap();
        let (run, path) = (runtime(), bookshop(dir.path()));
        let copy = dir.path().join("server.sqlite");
        let server = Server::stage(&run, &path, &copy, MEANWHILE).unwrap();
        let answer = server.answer(&run, &price(1, Value::Float(18.5), "16.50"));
        let Ok(WriteOutcome::Conflicts(conflicts)) = answer else {
            panic!("not a conflict: {answer:?}");
        };
        assert_eq!(conflicts.len(), 1);
        assert_eq!(conflicts[0].row, 0);
        let row = conflicts[0].server.as_ref().expect("the row is there");
        assert!(row.contains(&Value::Int(17)), "{row:?}");
        // What the video loaded is what the file still holds, and the copy
        // holds what changed meanwhile, not the save's value.
        assert_eq!(held(&run, &path, 1), Value::Float(18.5));
        assert_eq!(held(&run, &copy, 1), Value::Int(17));
    }

    #[test]
    fn a_save_that_finds_no_conflict_is_refused() {
        let dir = tempfile::tempdir().unwrap();
        let (run, path) = (runtime(), bookshop(dir.path()));
        let copy = dir.path().join("server.sqlite");
        let server = Server::stage(&run, &path, &copy, MEANWHILE).unwrap();
        let refused = server
            .answer(&run, &price(2, Value::Int(24), "20"))
            .unwrap_err();
        assert!(
            refused.contains("found no row changed meanwhile") && refused.contains("written"),
            "{refused}"
        );
        assert_eq!(held(&run, &path, 2), Value::Int(24));
    }

    #[test]
    fn meanwhile_has_to_change_a_row() {
        let dir = tempfile::tempdir().unwrap();
        let (run, path) = (runtime(), bookshop(dir.path()));
        let copy = dir.path().join("server.sqlite");
        for (sql, said) in [
            (
                "UPDATE books SET price = 1 WHERE id = 99",
                "statement 1 changed no row",
            ),
            ("UPDATE nothing SET price = 1", "statement 1 failed"),
        ] {
            let error = Server::stage(&run, &path, &copy, sql).err().unwrap();
            assert!(error.contains(said), "{error}");
        }
    }
}
