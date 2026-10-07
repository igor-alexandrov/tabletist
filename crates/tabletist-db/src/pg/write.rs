//! A save on PostgreSQL: one read-write transaction managed by hand, each
//! row locked with `FOR UPDATE` before it is compared and changed.

use std::time::Instant;

use tokio_postgres::SimpleQueryMessage;

use super::{Conn, column, column_metas, query_error, row_values};
use crate::dialect::{InsertStatement, RowUpdate};
use crate::script::retry_cancelled;
use crate::write::{
    Applied, Stored, changed_since_loaded, conflicts_of, more_than_one, not_read_back, not_stopped,
    same_row_twice,
};
use crate::{ChangeSet, ColumnMeta, Dialect, Error, Result, StopFlag, Value, WriteOutcome};

impl Conn {
    /// See [`crate::Connection::write`]. Rows are read through the
    /// simple-query protocol, as a page reads them, so a loaded value and
    /// the same value read here are the same `Value`. The transaction is
    /// managed by hand, as a script's is: every way out of it is taken
    /// here, and none is left to a value being dropped. `stop` is asked
    /// before each statement: a cancel request does nothing when it arrives
    /// between two.
    pub(super) async fn save(&self, changes: &ChangeSet, stop: &StopFlag) -> Result<WriteOutcome> {
        // Every statement is built first: a value that cannot be sent
        // fails the save before the server hears of it. Text that holds a
        // NUL is such a value, which the builder refuses: the driver could
        // not put it in a message.
        let mut updates = Vec::with_capacity(changes.rows.len());
        for (row, change) in changes.rows.iter().enumerate() {
            let built = Dialect::Postgres.update_row(&changes.object, change);
            match built {
                Ok(update) => updates.push(update),
                Err(error) => return Ok(WriteOutcome::Failed { row, error }),
            }
        }
        let mut inserts = Vec::with_capacity(changes.inserts.len());
        for (insert, row) in changes.inserts.iter().enumerate() {
            match Dialect::Postgres.insert_row(&changes.object, row) {
                Ok(built) => inserts.push(built),
                Err(error) => return Ok(WriteOutcome::FailedInsert { insert, error }),
            }
        }
        let client = self.client.lock().await;
        // Stopped before it began: nothing is sent.
        not_stopped(stop)?;
        let started = Instant::now();
        // From here every path ends the transaction, whatever of it began.
        let applied = match begin(&client).await {
            Ok(()) => apply(&client, changes, &inserts, &updates, stop).await,
            Err(error) => Err(error),
        };
        // The last moment a stop is heard. Once COMMIT is sent the save is
        // written, whatever arrives after it.
        let applied = match applied {
            Ok(Applied::Rows(rows)) => not_stopped(stop).map(|()| Applied::Rows(rows)),
            other => other,
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
        Ok(applied?.outcome(started))
    }
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
/// The `ROLLBACK` before it makes the save start from no transaction. The
/// app leaves none open: a save and a script each end their own. But a save
/// whose future was dropped half way would leave its transaction, in which
/// `START TRANSACTION` only warns, and this save's `COMMIT` would then
/// commit that one's rows with its own. With nothing open the `ROLLBACK`
/// is a warning and no more.
///
/// One message, so a failure of any statement leaves at most a failed
/// transaction, which the rollback every save ends with takes away.
async fn begin(client: &tokio_postgres::Client) -> Result<()> {
    execute(
        client,
        "ROLLBACK; START TRANSACTION READ WRITE; SET LOCAL client_encoding = 'UTF8'",
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
/// be rolled back like every other end but `Rows`, and a `stop` found
/// before any of its statements is such an end.
async fn apply(
    client: &tokio_postgres::Client,
    changes: &ChangeSet,
    inserts: &[InsertStatement],
    updates: &[RowUpdate],
    stop: &StopFlag,
) -> Result<Applied> {
    let dialect = Dialect::Postgres;
    not_stopped(stop)?;
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
    // Each change's row as its read found it, kept until every change has
    // read its own: two that read the same row are one row named twice.
    let mut read = Vec::with_capacity(changes.rows.len());
    let mut conflicts = Vec::new();
    for (row, change) in changes.rows.iter().enumerate() {
        let select = dialect.select_row(&changes.object, &change.key, true);
        not_stopped(stop)?;
        let mut found = rows(client, &select.text, &columns).await?;
        if found.len() > 1 {
            return Err(more_than_one());
        }
        let server = found.pop();
        let differs = match &server {
            None => true,
            Some(server) => changed_since_loaded(change, &names, server)?,
        };
        if differs {
            conflicts.push(row);
        }
        read.push(server);
    }
    // Before a conflict is answered too: a set that names a row twice is
    // not one to offer writing over what the server holds.
    same_row_twice(&changes.rows, &names, &read)?;
    if !conflicts.is_empty() {
        return Ok(Applied::Conflicts(conflicts_of(conflicts, read)));
    }
    // The new rows, before any row is changed: the order a review shows
    // them in. `RETURNING *` gives each in the table's column order, the
    // order `columns` was read in, and as its `INSERT` left it.
    let mut returned = Vec::with_capacity(inserts.len());
    for (insert, statement) in inserts.iter().enumerate() {
        not_stopped(stop)?;
        let messages = match client
            .simple_query(&statement.sql.text)
            .await
            .map_err(query_error)
        {
            Ok(messages) => messages,
            // A cancel or a lost session ends the save; anything else is
            // the statement's own failure.
            Err(error @ (Error::Cancelled | Error::ConnectionLost(_))) => return Err(error),
            Err(error) => return Ok(Applied::FailedInsert { insert, error }),
        };
        let mut made = Vec::new();
        for message in messages {
            if let SimpleQueryMessage::Row(row) = message {
                made.push(row_values(&row, &columns)?);
            }
        }
        // One statement makes one row, but for a trigger or a rule: a
        // BEFORE trigger can take the row for itself and store it
        // elsewhere, or nowhere. The statement went through all the same,
        // and what became of its row is then not known.
        returned.push(made.pop().filter(|_| made.is_empty()));
    }
    for (row, update) in updates.iter().enumerate() {
        not_stopped(stop)?;
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
        not_stopped(stop)?;
        let mut found = rows(client, &select.text, &columns).await?;
        // A trigger the update fired can give a second row the key, or
        // take it from the first. Either way the row to hand back is no
        // longer known, and the save is undone.
        if found.len() > 1 {
            return Err(more_than_one());
        }
        saved.push(found.pop().ok_or_else(not_read_back)?);
    }
    // What an `INSERT` returned is the row the table holds only where
    // nothing ran after it: a trigger or a rule can change the row, and no
    // read finds it again for sure, since a trigger can move its key too.
    // On such a table the new rows are written and handed back as not
    // known. Asked now, with the table held by what was written to it:
    // nobody adds a trigger before the save ends.
    let known = if returned.is_empty() {
        true
    } else {
        not_stopped(stop)?;
        client
            .query_opt(
                // An ordinary table: a row written through a partitioned
                // one goes to a partition, with triggers of its own. The
                // triggers PostgreSQL keeps for a foreign key only check.
                //
                // A foreign key of the table whose ON UPDATE acts (anything
                // but `a`, no action, and `r`, restrict) does a trigger's
                // work without one: a row changed after the inserts can
                // carry a new row with it. Only a save that changes rows
                // has such an update.
                "SELECT c.relkind = 'r' AND NOT c.relhasrules \
                        AND NOT EXISTS (SELECT 1 FROM pg_trigger t \
                                        WHERE t.tgrelid = c.oid AND NOT t.tgisinternal) \
                        AND ($3 OR NOT EXISTS (SELECT 1 FROM pg_constraint k \
                                               WHERE k.conrelid = c.oid AND k.contype = 'f' \
                                                 AND k.confupdtype NOT IN ('a', 'r'))) \
                 FROM pg_class c \
                 JOIN pg_namespace n ON n.oid = c.relnamespace \
                 WHERE n.nspname = $1 AND c.relname = $2",
                &[
                    &changes.object.schema,
                    &changes.object.name,
                    &changes.rows.is_empty(),
                ],
            )
            .await
            .map_err(query_error)?
            .map(|row| column(&row, 0))
            .transpose()?
            .unwrap_or(false)
    };
    let inserted = returned
        .into_iter()
        .map(|row| row.filter(|_| known))
        .collect();
    Ok(Applied::Rows(Stored {
        inserted,
        rows: saved,
    }))
}

#[cfg(test)]
mod tests {
    use std::future::Future;
    use std::panic::AssertUnwindSafe;

    use futures_util::FutureExt;

    use super::super::first_text;
    use super::super::tests::{session, session_with, test_url};
    use super::*;
    use crate::adapter::Adapter;
    use crate::{Access, CellChange, NewValue, ObjectRef, RowChange, RowQuery};

    /// The backend spawns nothing for a save, but awaits it on a task that
    /// was spawned: its future must be `Send`. This fails to compile, not
    /// to run.
    #[test]
    fn a_save_can_be_awaited_on_a_spawned_task() {
        fn send<T: Send>(_: &T) {}
        let _check =
            |conn: &Conn, changes: &ChangeSet| send(&conn.write(changes, &StopFlag::new()));
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
            Dialect::Postgres.update_row(&object, &change)
        };
        assert!(update("a", "b").is_ok());
        // In a new value and in the key alike: the builder makes no
        // statement of either, and a save sends what it makes.
        assert!(update("a", "b\0c").is_err());
        assert!(update("a\0", "b").is_err());
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
        on_a_table_of(url, table, "id text PRIMARY KEY, body text", test).await
    }

    /// Runs `test` on a table of its own with these columns, dropped
    /// however the test ends.
    async fn on_a_table_of<T>(
        url: &str,
        table: &str,
        columns: &str,
        test: impl Future<Output = T>,
    ) -> T {
        let admin = admin(url).await;
        let drop = format!("DROP TABLE IF EXISTS {table}");
        admin.batch_execute(&drop).await.unwrap();
        let outcome = AssertUnwindSafe(async {
            admin
                .batch_execute(&format!("CREATE TABLE {table} ({columns})"))
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
            inserts: Vec::new(),
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
            let outcome = conn
                .write(&body("write_unit_default", "a", "after"), &StopFlag::new())
                .await;
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

    /// Defaults under which the server prints values a save cannot send
    /// back: a float cut to fifteen digits (the default before PostgreSQL
    /// 12), and a time with its zone as an abbreviation that reads back as
    /// another zone (`IST` is Israel's to the server, not India's).
    const LOSSY: &str = "-c extra_float_digits=0 -c DateStyle=German,DMY -c TimeZone=Asia/Kolkata";

    /// What a page shows is what a save sends back, so the driver's session
    /// prints every digit of a float and a time with its offset, whatever
    /// the server, the database or the role has as a default.
    #[tokio::test]
    async fn a_session_prints_values_exactly_whatever_the_servers_defaults() {
        let Some(url) = test_url() else {
            return;
        };
        for access in [Access::ReadOnly, Access::Writable] {
            let conn = session_with(&url, access, LOSSY).await;
            assert_eq!(
                setting(&conn, "extra_float_digits").await,
                "3",
                "{access:?}"
            );
            let style = setting(&conn, "DateStyle").await;
            assert!(style.starts_with("ISO"), "{access:?}: {style}");
            // The zone is the server's still: only how it prints changed.
            assert_eq!(setting(&conn, "TimeZone").await, "Asia/Kolkata");
        }
    }

    /// Two rows whose keys a lossy session prints alike: neighbouring
    /// floats, and two times of which one, printed with its zone's
    /// abbreviation, reads back as the other. The second row's key, as the
    /// page shows it, finds the second row.
    #[tokio::test]
    async fn a_key_finds_its_own_row_whatever_the_servers_defaults_print() {
        let Some(url) = test_url() else {
            return;
        };
        let table = "write_unit_exact";
        let columns = "k float8 PRIMARY KEY, at timestamptz NOT NULL UNIQUE, body text";
        on_a_table_of(&url, table, columns, async {
            let admin = admin(&url).await;
            // 06:00 in India is 00:30 UTC, and `06:00 IST` read as Israel's
            // time is 04:00 UTC, the first row's.
            admin
                .batch_execute(&format!(
                    "INSERT INTO {table} VALUES
                         (0.3, '2026-03-29 04:00:00.123456+00', 'before'),
                         (0.30000000000000004, '2026-03-29 00:30:00.123456+00', 'before')"
                ))
                .await
                .unwrap();
            let conn = session_with(&url, Access::Writable, LOSSY).await;
            let object = ObjectRef::new("public", table);
            for (index, key, new) in [(0, "k", "by its number"), (1, "at", "by its time")] {
                let page = conn
                    .fetch_rows(&RowQuery::new(object.clone(), 10))
                    .await
                    .unwrap();
                // In key order: the second row is the greater float's.
                let second = &page.rows[1];
                let changes = ChangeSet {
                    object: object.clone(),
                    inserts: Vec::new(),
                    rows: vec![RowChange {
                        key: vec![(key.into(), second[index].clone())],
                        set: vec![CellChange {
                            column: "body".into(),
                            type_name: "text".into(),
                            loaded: second[2].clone(),
                            new: NewValue::Text(new.into()),
                        }],
                    }],
                };
                let outcome = conn.write(&changes, &StopFlag::new()).await;
                assert!(
                    matches!(outcome, Ok(WriteOutcome::Written { .. })),
                    "{key}: {outcome:?}"
                );
                // Read by another session, through the protocol that
                // carries values as they are.
                let bodies: Vec<String> = admin
                    .query(&format!("SELECT body FROM {table} ORDER BY k"), &[])
                    .await
                    .unwrap()
                    .iter()
                    .map(|row| row.get(0))
                    .collect();
                assert_eq!(bodies, ["before", new], "{key}");
            }
        })
        .await;
    }

    /// A save starts from no transaction. Nothing in the app leaves one
    /// open (a save and a script each end their own), but a save whose
    /// future was dropped half way would: its rows must not be committed
    /// by the next save, nor its failure be the next save's.
    #[tokio::test]
    async fn a_save_does_not_commit_what_a_transaction_left_open_wrote() {
        let Some(url) = test_url() else {
            return;
        };
        on_its_own_table(&url, "write_unit_left_open", async {
            let table = "write_unit_left_open";
            let admin = admin(&url).await;
            admin
                .batch_execute(&format!("INSERT INTO {table} VALUES ('a', 'before')"))
                .await
                .unwrap();
            let mut held = "before";
            for (left, new) in [
                (
                    format!("BEGIN READ WRITE; INSERT INTO {table} VALUES ('planted', 'planted')"),
                    "after a transaction left open",
                ),
                // One that failed is left by its statement's error.
                (
                    format!(
                        "BEGIN READ WRITE; INSERT INTO {table} VALUES ('planted', 'planted'); \
                         SELECT 1 / 0"
                    ),
                    "after a transaction left failed",
                ),
                // And none: the usual start.
                ("SELECT 1".to_owned(), "after no transaction"),
            ] {
                let conn = session(&url).await;
                let planted = conn.client.lock().await.batch_execute(&left).await;
                assert_eq!(planted.is_err(), left.contains("1 / 0"), "{left}");
                let mut changes = body(table, "a", new);
                changes.rows[0].set[0].loaded = Value::Text(held.into());
                held = new;
                let outcome = conn.write(&changes, &StopFlag::new()).await;
                assert!(
                    matches!(outcome, Ok(WriteOutcome::Written { .. })),
                    "{new}: {outcome:?}"
                );
                let rows: Vec<(String, String)> = admin
                    .query(&format!("SELECT id, body FROM {table} ORDER BY id"), &[])
                    .await
                    .unwrap()
                    .iter()
                    .map(|row| (row.get(0), row.get(1)))
                    .collect();
                assert_eq!(rows, [("a".to_owned(), new.to_owned())], "{new}");
            }
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
                let outcome = conn.write(&changes, &StopFlag::new()).await;
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
        let mut sets = vec![
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
        ];
        let conn = session(&url).await;
        // Whether triggers run is a superuser's to set.
        if setting(&conn, "is_superuser").await == "on" {
            sets.push(("session_replication_role", "replica"));
        }
        let mut before = Vec::new();
        for (name, _) in &sets {
            before.push(setting(&conn, name).await);
        }
        let mut script: Vec<String> = sets
            .iter()
            .map(|(name, value)| format!("SET {name} = {value}"))
            .collect();
        script.push("SELECT set_config('standard_conforming_strings', 'off', false)".into());
        script.push("SHOW standard_conforming_strings".into());
        let outcome = conn
            .script(&script, 10, crate::ScriptMode::ReadOnly, &StopFlag::new())
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
        for ((name, _), before) in sets.iter().zip(&before) {
            assert_eq!(&setting(&conn, name).await, before, "{name}");
        }
        // And how the session prints a float and a time is the driver's
        // own again, which a save's keys rely on.
        assert_eq!(setting(&conn, "extra_float_digits").await, "3");
        assert!(setting(&conn, "DateStyle").await.starts_with("ISO"));
    }
}
