//! A save on PostgreSQL: one read-write transaction managed by hand, each
//! row locked with `FOR UPDATE` before it is compared and changed.

use std::time::Instant;

use tokio_postgres::SimpleQueryMessage;

use super::{Conn, column_metas, query_error, row_values};
use crate::dialect::RowUpdate;
use crate::script::retry_cancelled;
use crate::write::changed_since_loaded;
use crate::{ChangeSet, ColumnMeta, Conflict, Dialect, Error, Result, Value, WriteOutcome};

/// What the statements of a save came to, before its transaction ends.
enum Applied {
    Rows(Vec<Vec<Value>>),
    Conflicts(Vec<Conflict>),
    Failed { row: usize, error: Error },
}

impl Conn {
    /// See [`crate::Connection::write`]. Rows are read through the
    /// simple-query protocol, as a page reads them, so a loaded value and
    /// the same value read here are the same `Value`. The transaction is
    /// managed by hand, as a script's is: every way out of it is taken
    /// here, and none is left to a value being dropped.
    pub async fn write(&self, changes: &ChangeSet) -> Result<WriteOutcome> {
        // Every statement is built first: a value that cannot be sent
        // fails the save before the server hears of it.
        let mut updates = Vec::with_capacity(changes.rows.len());
        for (row, change) in changes.rows.iter().enumerate() {
            let built = Dialect::Postgres
                .update_row(&changes.object, change)
                .and_then(sendable);
            match built {
                Ok(update) => updates.push(update),
                Err(error) => return Ok(WriteOutcome::Failed { row, error }),
            }
        }
        let client = self.client.lock().await;
        let started = Instant::now();
        // From here every path ends the transaction, whatever of it began.
        let applied = match begin(&client).await {
            Ok(()) => apply(&client, changes, &updates).await,
            Err(error) => Err(error),
        };
        // `Some` is a COMMIT the server refused, undone by the rollback
        // after it: a cancel that lands on a COMMIT before it takes hold
        // leaves the transaction open, and failed. A cancel meant for a
        // statement can land on the rollback too: it runs once more.
        let ended = match &applied {
            Ok(Applied::Rows(_)) => match execute(&client, "COMMIT").await {
                Ok(()) => Ok(None),
                Err(refused) => {
                    retry_cancelled!(execute(&client, "ROLLBACK")).map(|()| Some(refused))
                }
            },
            _ => retry_cancelled!(execute(&client, "ROLLBACK")).map(|()| None),
        };
        // Looked at before `applied`: a transaction that could not be ended
        // closes the session whatever the save itself came to. Left open,
        // it would hold its rows, and it may still be able to write.
        let uncommitted = ended.map_err(|error| {
            Error::ConnectionLost(format!("could not end the save's transaction: {error}"))
        })?;
        if let Some(refused) = uncommitted {
            return Err(refused);
        }
        Ok(match applied? {
            Applied::Rows(rows) => WriteOutcome::Written {
                rows,
                elapsed: started.elapsed(),
            },
            Applied::Conflicts(conflicts) => WriteOutcome::Conflicts(conflicts),
            Applied::Failed { row, error } => WriteOutcome::Failed { row, error },
        })
    }
}

/// Refuses a statement that holds a NUL. PostgreSQL text cannot hold one,
/// and the driver cannot put one in a message: it fails in a way that reads
/// as a lost session. The key's values and the new ones are all in the
/// `UPDATE`, so the reads by key need no check of their own.
fn sendable(update: RowUpdate) -> Result<RowUpdate> {
    if update.sql.text.contains('\0') {
        return Err(Error::query("PostgreSQL text cannot hold a NUL character"));
    }
    Ok(update)
}

/// Runs one of the save's own statements, which gives no rows.
async fn execute(client: &tokio_postgres::Client, statement: &str) -> Result<()> {
    client.batch_execute(statement).await.map_err(query_error)
}

/// Starts the save's transaction: read-write whatever the session's default
/// is (a server's own default may be read-only), and with the server
/// reading what the driver sends as what it is, UTF-8. The values of a save
/// are literals in its text, and under another client encoding the server
/// would store other characters and show them back as the ones sent. The
/// driver asks for UTF-8 when it connects and a script's `SET` goes with
/// its rollback, so this holds already on a session of the app's own; a
/// pooler can hand a transaction a server session someone else left
/// otherwise. `LOCAL`: it ends with the transaction.
///
/// One message, so a failure of either statement leaves at most a failed
/// transaction, which the rollback every save ends with takes away.
async fn begin(client: &tokio_postgres::Client) -> Result<()> {
    execute(
        client,
        "START TRANSACTION READ WRITE; SET LOCAL client_encoding = 'UTF8'",
    )
    .await
}

/// The rows of `text`, typed by `columns`.
async fn rows(
    client: &tokio_postgres::Client,
    text: &str,
    columns: &[ColumnMeta],
) -> Result<Vec<Vec<Value>>> {
    let messages = client.simple_query(text).await.map_err(query_error)?;
    let mut rows = Vec::new();
    for message in messages {
        if let SimpleQueryMessage::Row(row) = message {
            rows.push(row_values(&row, columns)?);
        }
    }
    Ok(rows)
}

/// The steps of a save inside its transaction: lock and compare every row,
/// update every row, read every row back. `Err` leaves the transaction to
/// be rolled back like every other end but `Rows`.
async fn apply(
    client: &tokio_postgres::Client,
    changes: &ChangeSet,
    updates: &[RowUpdate],
) -> Result<Applied> {
    let dialect = Dialect::Postgres;
    // The table's columns and their types, to read rows as a page does.
    // Preparing it also holds the table as it is until the transaction
    // ends: nobody adds or drops a column between the reads below.
    let statement = client
        .prepare(&format!(
            "SELECT * FROM {}",
            dialect.qualified(&changes.object)
        ))
        .await
        .map_err(query_error)?;
    let columns = column_metas(&statement);
    let names: Vec<String> = columns.iter().map(|column| column.name.clone()).collect();

    // Every row is locked and compared before any is changed. The lock is
    // what makes the comparison hold: a change someone has not committed
    // yet is waited for, and then it is their row that is read.
    let mut conflicts = Vec::new();
    for (row, change) in changes.rows.iter().enumerate() {
        let select = dialect.select_row(&changes.object, &change.key, true);
        let mut found = rows(client, &select.text, &columns).await?;
        if found.len() > 1 {
            return Err(more_than_one());
        }
        match found.pop() {
            None => conflicts.push(Conflict { row, server: None }),
            Some(server) => {
                if changed_since_loaded(change, &names, &server)? {
                    conflicts.push(Conflict {
                        row,
                        server: Some(server),
                    });
                }
            }
        }
    }
    if !conflicts.is_empty() {
        return Ok(Applied::Conflicts(conflicts));
    }
    for (row, update) in updates.iter().enumerate() {
        let messages = match client
            .simple_query(&update.sql.text)
            .await
            .map_err(query_error)
        {
            Ok(messages) => messages,
            // A cancel or a lost session ends the save; anything else is
            // the statement's own failure.
            Err(error @ (Error::Cancelled | Error::ConnectionLost(_))) => return Err(error),
            Err(error) => return Ok(Applied::Failed { row, error }),
        };
        let touched = messages
            .iter()
            .find_map(|message| match message {
                SimpleQueryMessage::CommandComplete(count) => Some(*count),
                _ => None,
            })
            .unwrap_or(0);
        if touched != 1 {
            return Ok(Applied::Failed {
                row,
                error: Error::query(format!(
                    "the save would have changed {touched} rows where it meant one"
                )),
            });
        }
    }
    let mut saved = Vec::with_capacity(changes.rows.len());
    for change in &changes.rows {
        let select = dialect.select_row(&changes.object, &change.key, false);
        let mut found = rows(client, &select.text, &columns).await?;
        // A trigger the update fired can give a second row the key, or
        // take it from the first. Either way the row to hand back is no
        // longer known, and the save is undone.
        if found.len() > 1 {
            return Err(more_than_one());
        }
        saved.push(
            found
                .pop()
                .ok_or_else(|| Error::query("a saved row could not be read back"))?,
        );
    }
    Ok(Applied::Rows(saved))
}

fn more_than_one() -> Error {
    Error::query("a row's key matches more than one row")
}

#[cfg(test)]
mod tests {
    use std::future::Future;
    use std::panic::AssertUnwindSafe;

    use futures_util::FutureExt;

    use super::super::first_text;
    use super::super::tests::{session, test_url};
    use super::*;
    use crate::{CellChange, NewValue, ObjectRef, RowChange, StopFlag};

    /// The backend spawns nothing for a save, but awaits it on a task that
    /// was spawned: its future must be `Send`. This fails to compile, not
    /// to run.
    #[test]
    fn a_save_can_be_awaited_on_a_spawned_task() {
        fn send<T: Send>(_: &T) {}
        let _check = |conn: &Conn, changes: &ChangeSet| send(&conn.write(changes));
    }

    #[test]
    fn a_statement_holding_a_nul_is_not_sent() {
        let object = ObjectRef::new("public", "people");
        let update = |key: &str, new: &str| {
            let change = RowChange {
                key: vec![("id".into(), Value::Text(key.into()))],
                set: vec![CellChange {
                    column: "name".into(),
                    type_name: "text".into(),
                    loaded: Value::Null,
                    new: NewValue::Text(new.into()),
                }],
            };
            Dialect::Postgres.update_row(&object, &change).unwrap()
        };
        assert!(sendable(update("a", "b")).is_ok());
        // In a new value and in the key alike.
        assert!(sendable(update("a", "b\0c")).is_err());
        assert!(sendable(update("a\0", "b")).is_err());
    }

    /// A writable connection outside the adapter, for arranging and probing.
    async fn admin(url: &str) -> tokio_postgres::Client {
        let mut config: tokio_postgres::Config = url.parse().unwrap();
        config.ssl_mode(tokio_postgres::config::SslMode::Disable);
        let (client, connection) = config.connect(tokio_postgres::NoTls).await.unwrap();
        tokio::spawn(connection);
        client
    }

    /// Runs `test` on a table of its own, `table (id text PRIMARY KEY, body
    /// text)`, dropped however the test ends. The fixture's tables are every
    /// test's, and no test writes to them.
    async fn on_its_own_table<T>(url: &str, table: &str, test: impl Future<Output = T>) -> T {
        let admin = admin(url).await;
        let drop = format!("DROP TABLE IF EXISTS {table}");
        admin.batch_execute(&drop).await.unwrap();
        let outcome = AssertUnwindSafe(async {
            admin
                .batch_execute(&format!(
                    "CREATE TABLE {table} (id text PRIMARY KEY, body text)"
                ))
                .await
                .unwrap();
            test.await
        })
        .catch_unwind()
        .await;
        // A transaction a save left open would hold the table for good.
        let dropped = admin
            .batch_execute(&format!("SET lock_timeout = '10s'; {drop}"))
            .await;
        let value = outcome.unwrap_or_else(|panic| std::panic::resume_unwind(panic));
        dropped.unwrap();
        value
    }

    /// The row `id` of `table` gets `body`.
    fn body(table: &str, id: &str, body: &str) -> ChangeSet {
        ChangeSet {
            object: ObjectRef::new("public", table),
            rows: vec![RowChange {
                key: vec![("id".into(), Value::Text(id.into()))],
                set: vec![CellChange {
                    column: "body".into(),
                    type_name: "text".into(),
                    loaded: Value::Text("before".into()),
                    new: NewValue::Text(body.into()),
                }],
            }],
        }
    }

    /// The session's value of a setting.
    async fn setting(conn: &Conn, name: &str) -> String {
        let client = conn.client.lock().await;
        let messages = client.simple_query(&format!("SHOW {name}")).await.unwrap();
        first_text(&messages).unwrap()
    }

    /// The unit tests' session is the app's read-only one: its default is a
    /// read-only transaction, as a server's own default can be on a
    /// writable session. The front door refuses a save on it; past the door
    /// it is a test of the transaction saying read-write itself.
    #[tokio::test]
    async fn a_save_is_read_write_on_a_session_whose_default_is_not() {
        let Some(url) = test_url() else {
            return;
        };
        on_its_own_table(&url, "write_unit_default", async {
            let admin = admin(&url).await;
            admin
                .batch_execute("INSERT INTO write_unit_default VALUES ('a', 'before')")
                .await
                .unwrap();
            let conn = session(&url).await;
            assert_eq!(setting(&conn, "default_transaction_read_only").await, "on");
            let outcome = conn.write(&body("write_unit_default", "a", "after")).await;
            assert!(
                matches!(outcome, Ok(WriteOutcome::Written { .. })),
                "{outcome:?}"
            );
            let stored: String = admin
                .query_one("SELECT body FROM write_unit_default", &[])
                .await
                .unwrap()
                .get(0);
            assert_eq!(stored, "after");
            // The session's own default is as it was.
            assert_eq!(setting(&conn, "default_transaction_read_only").await, "on");
        })
        .await;
    }

    /// Text whose literal a setting could change the reading of: quotes,
    /// backslashes, and characters that are more than one byte.
    const AWKWARD: [&str; 6] = [
        r"O'Brien \' and \\ and '' and a last \",
        "naïve café, 漢字, 🙂",
        r"C:\temp\né",
        "'",
        r"\",
        r"\x00ff",
    ];

    /// The settings that change how the server reads a statement's text,
    /// forced onto the session as nothing in the app can leave them (a
    /// script's `SET` goes with its rollback): a save stores its text
    /// exactly all the same, and finds its row by a key of such text.
    #[tokio::test]
    async fn a_save_stores_its_text_exactly_whatever_the_session_reads_strings_as() {
        let Some(url) = test_url() else {
            return;
        };
        on_its_own_table(&url, "write_unit_text", async {
            let admin = admin(&url).await;
            for left in [
                "SET standard_conforming_strings = off; SET backslash_quote = off",
                "SET standard_conforming_strings = off; SET backslash_quote = on",
                "SET client_encoding = 'LATIN1'",
                "SET client_encoding = 'LATIN1'; SET standard_conforming_strings = off",
            ] {
                admin
                    .batch_execute("TRUNCATE write_unit_text")
                    .await
                    .unwrap();
                for text in AWKWARD {
                    admin
                        .execute(
                            "INSERT INTO write_unit_text VALUES ($1, 'before')",
                            &[&text],
                        )
                        .await
                        .unwrap();
                }
                let conn = session(&url).await;
                conn.client.lock().await.batch_execute(left).await.unwrap();
                let mut changes = body("write_unit_text", AWKWARD[0], AWKWARD[0]);
                for text in &AWKWARD[1..] {
                    changes
                        .rows
                        .extend(body("write_unit_text", text, text).rows);
                }
                let outcome = conn.write(&changes).await;
                // What came back reads as what was sent.
                let WriteOutcome::Written { rows, .. } = outcome.unwrap() else {
                    panic!("{left}: not written");
                };
                let sent: Vec<Vec<Value>> = AWKWARD
                    .iter()
                    .map(|text| vec![Value::Text((*text).into()); 2])
                    .collect();
                assert_eq!(rows, sent, "{left}");
                // And what another session reads, through the protocol
                // that carries values as they are, is what was sent.
                let stored = admin
                    .query("SELECT id, body FROM write_unit_text", &[])
                    .await
                    .unwrap();
                assert_eq!(stored.len(), AWKWARD.len(), "{left}");
                for row in &stored {
                    let (id, body): (String, String) = (row.get(0), row.get(1));
                    assert_eq!(body, id, "{left}");
                    assert!(AWKWARD.contains(&id.as_str()), "{left}: {id}");
                }
            }
        })
        .await;
    }

    /// What a save would feel of a session, were a script to leave it
    /// there: how text and values are read, where names are looked up,
    /// whether triggers and constraints run, how long it waits. A script's
    /// transaction is rolled back, and every one of them goes with it. Run
    /// past the refusal, which stops some of these before the server sees
    /// them.
    #[tokio::test]
    async fn what_a_script_sets_does_not_outlast_it() {
        let Some(url) = test_url() else {
            return;
        };
        const SETS: [(&str, &str); 14] = [
            ("standard_conforming_strings", "off"),
            ("backslash_quote", "on"),
            ("client_encoding", "'LATIN1'"),
            ("search_path", "pg_catalog"),
            ("TimeZone", "'Asia/Tokyo'"),
            ("DateStyle", "'German, DMY'"),
            ("IntervalStyle", "'iso_8601'"),
            ("bytea_output", "'escape'"),
            ("extra_float_digits", "-5"),
            ("default_transaction_isolation", "'serializable'"),
            ("row_security", "off"),
            ("lock_timeout", "1"),
            ("statement_timeout", "100000"),
            // For a superuser only; the test server's user is one.
            ("session_replication_role", "replica"),
        ];
        let conn = session(&url).await;
        let mut before = Vec::new();
        for (name, _) in SETS {
            before.push(setting(&conn, name).await);
        }
        let mut script: Vec<String> = SETS
            .iter()
            .map(|(name, value)| format!("SET {name} = {value}"))
            .collect();
        script.push("SELECT set_config('standard_conforming_strings', 'off', false)".into());
        script.push("SHOW standard_conforming_strings".into());
        let outcome = conn
            .run_script(&script, 10, &StopFlag::new())
            .await
            .unwrap();
        // Every one of them took, inside the script.
        assert_eq!(outcome.results.len(), script.len(), "{outcome:?}");
        assert!(
            matches!(
                &outcome.results[script.len() - 1].outcome,
                crate::StatementOutcome::Rows { rows, .. }
                    if rows[0][0] == Value::Text("off".into())
            ),
            "{outcome:?}"
        );
        for ((name, _), before) in SETS.iter().zip(&before) {
            assert_eq!(&setting(&conn, name).await, before, "{name}");
        }
    }
}
