//! A save on MySQL: one read-write transaction managed as text, each row
//! locked with `FOR UPDATE` before it is compared and changed.

use std::time::Instant;

use mysql_async::consts::StatusFlags;
use mysql_async::prelude::Queryable;

use super::{
    Conn, column_metas, driver_parameter, execute, from_row, params, query_error, row_values,
    status,
};
use crate::script::retry_cancelled;
use crate::write::{
    Applied, Stored, changed_since_loaded, conflicts_of, more_than_one, named_twice, not_read_back,
    not_stopped, same_row_twice, spelled_otherwise,
};
use crate::{
    ChangeSet, ColumnClass, ColumnMeta, Dialect, Error, InsertValue, NewValue, ObjectRef, Result,
    RowChange, RowInsert, Sql, StopFlag, Value, WriteOutcome, column_class,
};

/// One row's statements, built before anything is sent.
struct Statements {
    /// The row by its key, locked.
    lock: Sql,
    update: Sql,
    /// The row by its key again, after the updates.
    read_back: Sql,
}

/// Ends a transaction and starts no other, whatever the server's
/// `completion_type` says a `COMMIT` or a `ROLLBACK` does next: chained,
/// the session would be left inside a read-write transaction.
const COMMIT: &str = "COMMIT AND NO CHAIN NO RELEASE";
const ROLLBACK: &str = "ROLLBACK AND NO CHAIN NO RELEASE";

impl Conn {
    /// See [`crate::Connection::write`]. The transaction is started and
    /// ended as text, not through the driver's transaction options: the
    /// driver opens a read-only transaction as `SET TRANSACTION READ ONLY`
    /// then `START TRANSACTION`, and a cancel between the two leaves "the
    /// next transaction is read-only" pending. `stop` is asked before each
    /// statement: `KILL QUERY` does nothing when it arrives between two.
    pub(super) async fn save(&self, changes: &ChangeSet, stop: &StopFlag) -> Result<WriteOutcome> {
        // Every statement is built first: a set that cannot be written as
        // MySQL reads it, or a value that cannot be sent, fails the save
        // before the server hears of it.
        for change in &changes.rows {
            names_its_columns_once(change)?;
        }
        let mut statements = Vec::with_capacity(changes.rows.len());
        for (row, change) in changes.rows.iter().enumerate() {
            match build(&changes.object, change) {
                Ok(built) => statements.push(built),
                Err(error) => return Ok(WriteOutcome::Failed { row, error }),
            }
        }
        let mut inserts = Vec::with_capacity(changes.inserts.len());
        for (insert, row) in changes.inserts.iter().enumerate() {
            match build_insert(&changes.object, row) {
                Ok(built) => inserts.push(built),
                Err(error) => return Ok(WriteOutcome::FailedInsert { insert, error }),
            }
        }
        // A save that changes no row reads none, and takes its table with
        // a read of its own (see `hold`).
        let hold = if changes.rows.is_empty() {
            Some(hold(&changes.object)?)
        } else {
            None
        };
        let written = Written {
            statements: &statements,
            inserts: &inserts,
            hold: hold.as_ref(),
        };
        let mut conn = self.conn.lock().await;
        // Stopped before it began: nothing is sent.
        not_stopped(stop)?;
        let started = Instant::now();
        // Asked before the transaction, so that the usual refusal costs no
        // lock; asked again inside it, where the answer holds (see `apply`).
        transactional(&mut conn, &changes.object).await?;
        not_stopped(stop)?;
        // From here every path ends the transaction, whatever of it began:
        // a `begin` whose status check failed has one open too.
        let applied = match begin(&mut conn).await {
            Ok(()) => apply(&mut conn, changes, written, stop).await,
            Err(error) => Err(error),
        };
        // The last moment a stop is heard. Once COMMIT is sent the save is
        // written, whatever arrives after it.
        let applied = match applied {
            Ok(Applied::Rows(rows)) => not_stopped(stop).map(|()| Applied::Rows(rows)),
            other => other,
        };
        // `Some` is a COMMIT the server refused, undone by the rollback
        // after it. A cancel meant for a statement can land on a rollback:
        // it runs once more.
        let ended = match &applied {
            Ok(Applied::Rows(_)) => match execute(&mut conn, COMMIT).await {
                Ok(()) => Ok(None),
                Err(refused) => {
                    retry_cancelled!(execute(&mut conn, ROLLBACK)).map(|()| Some(refused))
                }
            },
            _ => retry_cancelled!(execute(&mut conn, ROLLBACK)).map(|()| None),
        };
        // Looked at before `applied`: a transaction that could not be ended
        // closes the session whatever the save itself came to. Left open, it
        // would hold its rows, and the next START TRANSACTION would commit
        // them: half a save.
        let uncommitted = ended.map_err(|error| {
            Error::ConnectionLost(format!("could not end the save's transaction: {error}"))
        })?;
        if let Some(refused) = uncommitted {
            return Err(refused);
        }
        Ok(applied?.outcome(started))
    }
}

/// Refuses a row whose names MySQL would read as one column twice. It
/// matches a column's name without regard to case, and `ChangeSet::check`
/// compares names exactly: `ID` in the set and `id` in the key pass it and
/// would change the row's own key, and `name` beside `NAME` would set one
/// column twice, the last one winning. Checked here on the names alone,
/// before anything is sent; the spelling itself is checked against the
/// table's once the row is read (`spelled_otherwise`).
fn names_its_columns_once(change: &RowChange) -> Result<()> {
    let same = |a: &str, b: &str| a.to_lowercase() == b.to_lowercase();
    for (index, cell) in change.set.iter().enumerate() {
        if change
            .key
            .iter()
            .any(|(column, _)| same(column, &cell.column))
        {
            return Err(Error::query(format!(
                "{} is part of the row's key and cannot be changed",
                cell.column
            )));
        }
        if change.set[..index]
            .iter()
            .any(|earlier| same(&earlier.column, &cell.column))
        {
            return Err(Error::query(format!(
                "{} is changed twice in one row",
                cell.column
            )));
        }
    }
    Ok(())
}

/// The statements of one row's change. Each is one the driver would send
/// as it stands: it reads `:name` in a statement's text as a parameter and
/// sends a `?` in its place, which in a name would make the statement
/// another one. A quoted name hides a colon from it, so this refuses
/// nothing a table can be called today; it is here so that only text the
/// driver leaves alone reaches the server.
fn build(object: &ObjectRef, change: &RowChange) -> Result<Statements> {
    let dialect = Dialect::MySql;
    let statements = Statements {
        lock: dialect.select_row(object, &change.key, true),
        update: dialect.update_row(object, change)?.sql,
        read_back: dialect.select_row(object, &change.key, false),
    };
    for sql in [&statements.lock, &statements.update, &statements.read_back] {
        if driver_parameter(&sql.text).is_err() {
            return Err(Error::query(
                "the MySQL driver would read part of a name as a parameter, so the save cannot \
                 be sent",
            ));
        }
    }
    Ok(statements)
}

/// The `INSERT` of one new row, as the driver would send it as it stands
/// (see [`build`]).
fn build_insert(object: &ObjectRef, row: &RowInsert) -> Result<Sql> {
    // MySQL matches a column's name without regard to case.
    if let Some(name) = named_twice(row, |a, b| a.to_lowercase() == b.to_lowercase()) {
        return Err(Error::query(format!("{name} is set twice in one new row")));
    }
    let sql = Dialect::MySql.insert_row(object, row)?.sql;
    if driver_parameter(&sql.text).is_err() {
        return Err(Error::query(
            "the MySQL driver would read part of a name as a parameter, so the save cannot \
             be sent",
        ));
    }
    Ok(sql)
}

/// A read that takes the table and no row of it. A save that changes rows
/// holds its table from its first locking read on; one of new rows alone
/// reads nothing before it asks for the table's engine, and without this
/// someone could still change the engine, or the key, before the first
/// `INSERT` (see the comment in [`apply`]). A locking read, so it takes no
/// snapshot either.
fn hold(object: &ObjectRef) -> Result<Sql> {
    let sql = Sql {
        text: format!(
            "SELECT 1 FROM {} LIMIT 0 FOR UPDATE",
            Dialect::MySql.qualified(object)
        ),
        params: Vec::new(),
    };
    if driver_parameter(&sql.text).is_err() {
        return Err(Error::query(
            "the MySQL driver would read part of a name as a parameter, so the save cannot \
             be sent",
        ));
    }
    Ok(sql)
}

/// The statements of a save, built before anything is sent.
#[derive(Clone, Copy)]
struct Written<'a> {
    /// Of each changed row.
    statements: &'a [Statements],
    /// The `INSERT` of each new row.
    inserts: &'a [Sql],
    /// What takes the table, for a save that changes no row.
    hold: Option<&'a Sql>,
}

/// What finds a row an `INSERT` made: the table's primary key, and the
/// column its counter fills.
struct FoundBy {
    key: Vec<String>,
    counter: Option<String>,
}

impl FoundBy {
    /// Asked inside the save's transaction, once its table is held (by the
    /// locking reads of its changed rows, or by [`hold`]): nobody changes
    /// the key between this and the rows it is used to read.
    async fn of(conn: &mut mysql_async::Conn, object: &ObjectRef, stop: &StopFlag) -> Result<Self> {
        let at = (&object.schema, &object.name, &object.schema, &object.name);
        not_stopped(stop)?;
        // The names by their bytes as well, as `transactional` matches
        // them, and each with a `LIMIT` of its own: a server's default
        // `sql_select_limit` can be 0.
        let key: Vec<mysql_async::Row> = conn
            .exec(
                "SELECT column_name FROM information_schema.key_column_usage \
                 WHERE table_schema = ? AND table_name = ? \
                   AND CAST(table_schema AS BINARY) = CAST(? AS BINARY) \
                   AND CAST(table_name AS BINARY) = CAST(? AS BINARY) \
                   AND constraint_name = 'PRIMARY' \
                 ORDER BY ordinal_position LIMIT 64",
                at,
            )
            .await
            .map_err(query_error)?;
        not_stopped(stop)?;
        let counter: Vec<mysql_async::Row> = conn
            .exec(
                "SELECT column_name FROM information_schema.columns \
                 WHERE table_schema = ? AND table_name = ? \
                   AND CAST(table_schema AS BINARY) = CAST(? AS BINARY) \
                   AND CAST(table_name AS BINARY) = CAST(? AS BINARY) \
                   AND extra LIKE '%auto_increment%' \
                 LIMIT 1",
                at,
            )
            .await
            .map_err(query_error)?;
        Ok(Self {
            key: key.into_iter().map(from_row).collect::<Result<_>>()?,
            counter: counter.into_iter().next().map(from_row).transpose()?,
        })
    }

    /// The key of the row `insert` made, whose counter gave it `id`: each
    /// column of the primary key with the counter's value where it is the
    /// counter's column and the counter gave one (a `0` or a NULL sent for
    /// it is not what was stored), and otherwise the value sent for it.
    /// `None` when the row cannot be found again for sure: the table has no
    /// primary key, the database filled a key column some other way (a
    /// default), or a key column's value cannot be matched exactly.
    fn key(&self, insert: &RowInsert, id: Option<u64>) -> Option<Vec<(String, Value)>> {
        if self.key.is_empty() {
            return None;
        }
        self.key
            .iter()
            .map(|name| {
                let counted = id.filter(|_| self.counter.as_deref() == Some(name.as_str()));
                let sent = insert.set.iter().find(|cell| cell.column == *name);
                let value = match (counted, sent) {
                    (Some(id), _) => Value::Int(i64::try_from(id).ok()?),
                    (None, Some(cell)) => sent_key(cell)?,
                    (None, None) => return None,
                };
                Some((name.clone(), value))
            })
            .collect()
    }
}

/// A value sent for a key column, as what finds its row again. A whole
/// number goes as a number: bound as text, MySQL would compare it with the
/// column as two doubles, and past 2^53 find a neighbour. Text goes as
/// text. Every other class is one whose stored form is not the text that
/// was sent (a decimal is rounded, a float is not its text, a date is
/// normalised), and its row is not looked for.
fn sent_key(cell: &InsertValue) -> Option<Value> {
    let NewValue::Text(text) = &cell.new else {
        return None;
    };
    if inexact(&cell.type_name).is_some() {
        return None;
    }
    match column_class(Dialect::MySql, &cell.type_name) {
        ColumnClass::Integer { .. } => text.trim().parse().ok().map(Value::Int),
        ColumnClass::Text { .. } => Some(Value::Text(text.as_str().into())),
        ColumnClass::Decimal { .. }
        | ColumnClass::Float
        | ColumnClass::Boolean
        | ColumnClass::Json
        | ColumnClass::Binary
        | ColumnClass::Other => None,
    }
}

/// Runs one new row's `INSERT`. The inner `Err` is the statement's own
/// failure: the server's error, or a warning (see [`update`]). The outer is
/// a cancel, a stop or a lost session. `Ok(Ok(id))` is the value the
/// table's counter gave the row, when it gave one.
async fn insert_row(
    conn: &mut mysql_async::Conn,
    sql: &Sql,
    stop: &StopFlag,
) -> Result<std::result::Result<Option<u64>, Error>> {
    let statement = match prepare(conn, sql, stop).await {
        Ok(statement) => statement,
        Err(error @ (Error::Cancelled | Error::ConnectionLost(_))) => return Err(error),
        Err(error) => return Ok(Err(error)),
    };
    not_stopped(stop)?;
    if let Err(error) = conn.exec_drop(&statement, params(&sql.params)).await {
        return match query_error(error) {
            error @ (Error::Cancelled | Error::ConnectionLost(_)) => Err(error),
            error => Ok(Err(error)),
        };
    }
    // Read before the warnings are asked for: that is a statement too.
    let id = conn.last_insert_id().filter(|id| *id > 0);
    // Outside strict mode a column without a default takes its type's
    // zero, and a value is cut to fit, with only a warning. What was
    // stored is then not what was asked for, and the save fails.
    if conn.get_warnings() > 0 {
        return warning(conn).await.map(Err);
    }
    Ok(Ok(id))
}

/// Whether the table's engine has transactions. One transaction is the
/// promise of a save, and an engine without them (MyISAM) cannot roll back:
/// a statement that failed would leave the rows before it written. A view
/// has no engine, and is refused with them.
///
/// `information_schema.tables` does not list a TEMPORARY table, which hides
/// a table of the same name: the engine found would then be the hidden
/// one's. No session of the app has one. A script cannot create it, since
/// its transaction is read-only, and the reset that ends every script drops
/// the session's temporary tables.
///
/// The names are matched by their bytes as well: where table names keep
/// their letters, `Foo` and `foo` are two tables, and older servers compare
/// these columns without regard to case, so the engine found could be the
/// other's. The plain comparison stays beside it, for the lookup it lets
/// the server do.
///
/// With a `LIMIT` of its own: a server's default `sql_select_limit` can be
/// 0, and the answer must still come.
async fn transactional(conn: &mut mysql_async::Conn, object: &ObjectRef) -> Result<()> {
    let row: Option<mysql_async::Row> = conn
        .exec_first(
            "SELECT e.transactions FROM information_schema.tables t \
             JOIN information_schema.engines e ON e.engine = t.engine \
             WHERE t.table_schema = ? AND t.table_name = ? \
               AND CAST(t.table_schema AS BINARY) = CAST(? AS BINARY) \
               AND CAST(t.table_name AS BINARY) = CAST(? AS BINARY) LIMIT 1",
            (&object.schema, &object.name, &object.schema, &object.name),
        )
        .await
        .map_err(query_error)?;
    // The column can be NULL, for an engine the server does not have.
    let transactions: Option<String> = row.map(from_row).transpose()?.flatten();
    if transactions.as_deref() == Some("YES") {
        Ok(())
    } else {
        Err(Error::Unsupported(
            "saving needs a table whose engine has transactions, such as InnoDB",
        ))
    }
}

/// Starts the save's transaction, read-write whatever the session is (a
/// script's fence makes a writable session read-only while it runs), and
/// holds the server to saying so. A `begin` that fails this check has a
/// transaction open, which is why `write` rolls back after it too.
///
/// It starts from no transaction. `START TRANSACTION` commits one that is
/// open, and though the app leaves none (a save and a script each end their
/// own), a save whose future was dropped half way would leave its own, for
/// this one to commit. The status the server sent with its last answer says
/// whether one is open, at no cost, and that one is rolled back first.
async fn begin(conn: &mut mysql_async::Conn) -> Result<()> {
    if status(conn).contains(StatusFlags::SERVER_STATUS_IN_TRANS) {
        execute(conn, ROLLBACK).await?;
    }
    execute(conn, "START TRANSACTION READ WRITE").await?;
    started(status(conn))
}

/// Whether the status after `START TRANSACTION READ WRITE` says what it
/// must: inside a transaction, and not a read-only one.
fn started(status: StatusFlags) -> Result<()> {
    if status.contains(StatusFlags::SERVER_STATUS_IN_TRANS)
        && !status.contains(StatusFlags::SERVER_STATUS_IN_TRANS_READONLY)
    {
        Ok(())
    } else {
        Err(Error::query(
            "the server did not start a read-write transaction",
        ))
    }
}

/// Prepares one of the save's statements. Each runs prepared, with its
/// values bound: a value is never part of the text, so nothing in it is
/// read as SQL, by the server or by the driver. Preparing is a message of
/// its own, so `stop` is asked before it, and again by whoever runs what
/// it prepared.
async fn prepare(
    conn: &mut mysql_async::Conn,
    sql: &Sql,
    stop: &StopFlag,
) -> Result<mysql_async::Statement> {
    not_stopped(stop)?;
    let statement = conn.prep(sql.text.as_str()).await.map_err(query_error)?;
    // The driver closes the connection when a statement runs without a
    // value for each of its parameters.
    if usize::from(statement.num_params()) != sql.params.len() {
        return Err(Error::query(
            "the statement does not take the values the save has for it",
        ));
    }
    Ok(statement)
}

/// The rows of a statement, with their columns.
async fn rows(
    conn: &mut mysql_async::Conn,
    sql: &Sql,
    stop: &StopFlag,
) -> Result<(Vec<ColumnMeta>, Vec<Vec<Value>>)> {
    let statement = prepare(conn, sql, stop).await?;
    not_stopped(stop)?;
    let mut result = conn
        .exec_iter(&statement, params(&sql.params))
        .await
        .map_err(query_error)?;
    let columns = column_metas(result.columns_ref());
    let mut rows = Vec::new();
    while let Some(row) = result.next().await.map_err(query_error)? {
        rows.push(row_values(row, &columns));
    }
    result.drop_result().await.map_err(query_error)?;
    Ok((columns, rows))
}

/// The type of a column, by the name the driver gives it (`type_name`),
/// when a key of that type cannot be sure of its row:
///
/// - a `timestamp` is shown in the session's time zone, without the zone.
///   In the hour a zone repeats when its clocks go back, two instants read
///   alike, and the key of either finds the same one of them;
/// - a `bit` is shown as its bytes, and the server reads bound bytes as a
///   number written out, so the key finds no row, or another's;
/// - a `float` is shown by its shortest text and goes as a double, which is
///   not the float widened, so the key finds no row.
///
/// A `double`, a `datetime` and a `decimal` are matched as they are shown.
fn inexact(type_name: &str) -> Option<&str> {
    let name = type_name.split([' ', '(']).next().unwrap_or_default();
    matches!(name, "timestamp" | "bit" | "float").then_some(name)
}

/// A key column of `change` that cannot be matched exactly, and its type,
/// among the row's `columns`.
fn inexact_key<'a>(change: &'a RowChange, columns: &'a [ColumnMeta]) -> Option<(&'a str, &'a str)> {
    change.key.iter().find_map(|(name, _)| {
        let column = columns.iter().find(|column| column.name == *name)?;
        Some((name.as_str(), inexact(&column.type_name)?))
    })
}

/// A failure that is the statement's own. A cancel or a lost session ends
/// the save instead.
fn own(error: Error) -> Result<Option<Error>> {
    match error {
        error @ (Error::Cancelled | Error::ConnectionLost(_)) => Err(error),
        error => Ok(Some(error)),
    }
}

/// Runs one row's `UPDATE`. `Ok(Some(..))` is the statement's own failure:
/// the server's error, more than the one row changed, or a warning. `Err`
/// is a cancel, a stop or a lost session.
async fn update(conn: &mut mysql_async::Conn, sql: &Sql, stop: &StopFlag) -> Result<Option<Error>> {
    let statement = match prepare(conn, sql, stop).await {
        Ok(statement) => statement,
        Err(error) => return own(error),
    };
    not_stopped(stop)?;
    if let Err(error) = conn.exec_drop(&statement, params(&sql.params)).await {
        return own(query_error(error));
    }
    // The count is of rows changed, not of rows found: a row given the
    // value it has counts as none. The row is locked and was just read, so
    // none or one is right.
    let changed = conn.affected_rows();
    if changed > 1 {
        return Ok(Some(Error::query(format!(
            "the save would have changed {changed} rows where it meant one"
        ))));
    }
    // Outside strict mode the server cuts or adjusts a value it cannot
    // store and says so only in a warning, and in any mode it says in a
    // note that it rounded a decimal to its column's scale (the session
    // keeps notes on, see `prepare_session`). What was stored is then not
    // what was typed, and the save fails. Not every such change is said:
    // `'1.6'` goes into a TINYINT as 2 and `16777217` into a FLOAT as
    // 16777216 without a warning or a note, in the default mode too. Those
    // are written, and the row read back shows what was stored.
    if conn.get_warnings() > 0 {
        return warning(conn).await.map(Some);
    }
    Ok(None)
}

/// The first warning of the statement that just ran, as its error.
async fn warning(conn: &mut mysql_async::Conn) -> Result<Error> {
    let rows: Vec<mysql_async::Row> = conn.query("SHOW WARNINGS").await.map_err(query_error)?;
    let Some(row) = rows.into_iter().next() else {
        return Ok(Error::query(
            "the statement raised a warning the server does not list",
        ));
    };
    let (_level, code, message): (String, u32, String) = from_row(row)?;
    Ok(Error::Query {
        code: Some(code.to_string()),
        message,
        detail: None,
        hint: None,
    })
}

/// The steps of a save inside its transaction: lock and compare every row,
/// update every row, read every row back. `Err` leaves the transaction to
/// be rolled back like every other end but `Rows`, and a `stop` found
/// before any of its statements is such an end.
async fn apply(
    conn: &mut mysql_async::Conn,
    changes: &ChangeSet,
    written: Written<'_>,
    stop: &StopFlag,
) -> Result<Applied> {
    let Written {
        statements,
        inserts,
        hold,
    } = written;
    // Every row is locked and compared before any is changed. The lock is
    // what makes the comparison hold: a change someone has not committed
    // yet is waited for, and then it is their row that is read.
    // Each change's row as its read found it, kept until every change has
    // read its own: two that read the same row are one row named twice.
    let mut names = Vec::new();
    let mut read = Vec::with_capacity(changes.rows.len());
    let mut conflicts = Vec::new();
    for (row, (change, statement)) in changes.rows.iter().zip(statements).enumerate() {
        let (columns, mut found) = rows(conn, &statement.lock, stop).await?;
        names = columns.iter().map(|column| column.name.clone()).collect();
        if let Some(name) = spelled_otherwise(change, &names) {
            return Err(Error::query(format!(
                "the table spells {name} another way, so the save cannot be sure which column \
                 it names"
            )));
        }
        // Known only now: a key comes without its columns' types, and the
        // read's answer is the first to carry them.
        if let Some((name, type_name)) = inexact_key(change, &columns) {
            return Err(Error::query(format!(
                "{name} is a {type_name} column, and a key of that type cannot be matched \
                 exactly, so the save cannot be sure which row it names"
            )));
        }
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
    // Asked again now that the reads hold the table: its metadata lock
    // lasts until the transaction ends, so an `ALTER TABLE` waits, and the
    // engine found here is the engine the updates run on. Before the reads
    // someone could still have changed it, and on an engine without
    // transactions the updates would not be undone.
    //
    // The answer is current only because this is the transaction's first
    // read that takes no lock. A locking read sees what is committed; a
    // plain read sees the transaction's snapshot, which the first plain
    // read takes. Here that is this one, once the table is held. A plain
    // read before the locking reads would take the snapshot while someone
    // could still change the engine, and `information_schema` would show
    // this read the engine as it was then. So no plain read goes before it.
    //
    // A save of new rows alone has no such read, and takes the table with
    // one that reads no row (`hold`).
    if let Some(hold) = hold {
        rows(conn, hold, stop).await?;
    }
    not_stopped(stop)?;
    transactional(conn, &changes.object).await?;
    if !conflicts.is_empty() {
        return Ok(Applied::Conflicts(conflicts_of(conflicts, read)));
    }
    // The new rows, before any row is changed: the order a review shows
    // them in. What finds each again is noted as it is made, and asked for
    // only here, after the plain read above took the transaction's
    // snapshot.
    let found_by = if inserts.is_empty() {
        None
    } else {
        Some(FoundBy::of(conn, &changes.object, stop).await?)
    };
    let mut made = Vec::with_capacity(inserts.len());
    for (insert, (row, sql)) in changes.inserts.iter().zip(inserts).enumerate() {
        match insert_row(conn, sql, stop).await? {
            Ok(id) => made.push(found_by.as_ref().and_then(|by| by.key(row, id))),
            Err(error) => return Ok(Applied::FailedInsert { insert, error }),
        }
    }
    for (row, statement) in statements.iter().enumerate() {
        if let Some(error) = update(conn, &statement.update, stop).await? {
            return Ok(Applied::Failed { row, error });
        }
    }
    let mut saved = Vec::with_capacity(statements.len());
    for statement in statements {
        let (_, mut found) = rows(conn, &statement.read_back, stop).await?;
        // A change can give a second row the key (a trigger, a column the
        // database works out from another), or take it from the first.
        // Either way the row to hand back is no longer known, and the
        // save is undone.
        if found.len() > 1 {
            return Err(more_than_one());
        }
        saved.push(found.pop().ok_or_else(not_read_back)?);
    }
    // The new rows last, as they stand once every statement has run.
    let mut inserted = Vec::with_capacity(made.len());
    for key in made {
        let Some(key) = key else {
            inserted.push(None);
            continue;
        };
        let select = Dialect::MySql.select_row(&changes.object, &key, false);
        // A name the driver would read as a parameter: not looked for.
        if driver_parameter(&select.text).is_err() {
            inserted.push(None);
            continue;
        }
        let (_, mut found) = rows(conn, &select, stop).await?;
        // More than one row by a primary key is no key at all.
        if found.len() > 1 {
            return Err(more_than_one());
        }
        // None: a trigger changed the row's key, and the row is unknown.
        inserted.push(found.pop());
    }
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
    use mysql_async::Opts;

    use super::super::prepare_session;
    use super::super::tests::{session, test_url};
    use super::*;
    use crate::adapter::Adapter;
    use crate::{Access, CellChange, NewValue, RowQuery};

    /// The backend awaits a save on a task it spawned: its future must be
    /// `Send`. This fails to compile, not to run.
    #[test]
    fn a_save_can_be_awaited_on_a_spawned_task() {
        fn send<T: Send>(_: &T) {}
        let _check =
            |conn: &Conn, changes: &ChangeSet| send(&conn.write(changes, &StopFlag::new()));
    }

    /// A server (or a proxy in front of one) that does not say the
    /// transaction is open, or marks it read-only, has not started the
    /// transaction a save needs.
    #[test]
    fn a_transaction_the_server_does_not_mark_read_write_fails_the_start() {
        let autocommit = StatusFlags::SERVER_STATUS_AUTOCOMMIT;
        let in_transaction = autocommit | StatusFlags::SERVER_STATUS_IN_TRANS;
        let read_only = in_transaction | StatusFlags::SERVER_STATUS_IN_TRANS_READONLY;
        assert_eq!(started(in_transaction), Ok(()));
        assert_eq!(started(StatusFlags::SERVER_STATUS_IN_TRANS), Ok(()));
        for status in [read_only, autocommit, StatusFlags::empty()] {
            assert_eq!(
                started(status),
                Err(Error::query(
                    "the server did not start a read-write transaction"
                )),
                "{status:?}"
            );
        }
    }

    fn change(key: &[&str], set: &[&str]) -> RowChange {
        RowChange {
            key: key
                .iter()
                .map(|name| ((*name).to_owned(), Value::Int(1)))
                .collect(),
            set: set
                .iter()
                .map(|name| CellChange {
                    column: (*name).to_owned(),
                    type_name: "varchar(255)".into(),
                    loaded: Value::Null,
                    new: NewValue::Text("new".into()),
                })
                .collect(),
        }
    }

    #[test]
    fn names_that_differ_only_in_case_are_one_column() {
        assert_eq!(names_its_columns_once(&change(&["id"], &["name"])), Ok(()));
        assert_eq!(
            names_its_columns_once(&change(&["a", "b"], &["c", "d"])),
            Ok(())
        );
        // An accent makes another column, as it does to MySQL.
        assert_eq!(names_its_columns_once(&change(&["a"], &["á"])), Ok(()));
        for (key, set, said) in [
            (&["id"][..], &["ID"][..], "part of the row's key"),
            (
                &["a", "Id"][..],
                &["name", "iD"][..],
                "part of the row's key",
            ),
            (&["id"][..], &["name", "NAME"][..], "changed twice"),
            (
                &["id"][..],
                &["Straße", "STRASSE", "straße"][..],
                "changed twice",
            ),
        ] {
            let refused = names_its_columns_once(&change(key, set))
                .unwrap_err()
                .to_string();
            assert!(refused.contains(said), "{key:?} {set:?}: {refused}");
        }
    }

    #[test]
    fn a_key_of_a_type_that_is_not_matched_exactly_is_found_by_its_types_name() {
        // As the driver names a result column's type, and as the catalog
        // spells one in full.
        for (type_name, inexact_as) in [
            ("timestamp", Some("timestamp")),
            ("timestamp(6)", Some("timestamp")),
            ("bit", Some("bit")),
            ("bit(8)", Some("bit")),
            ("float", Some("float")),
            ("float unsigned", Some("float")),
            ("float(7,3) unsigned", Some("float")),
            ("double", None),
            ("double unsigned", None),
            ("datetime", None),
            ("datetime(6)", None),
            ("decimal", None),
            ("decimal(6,2)", None),
            ("int", None),
            ("varchar", None),
            ("binary", None),
            ("", None),
        ] {
            assert_eq!(inexact(type_name), inexact_as, "{type_name}");
        }
        let column = |name: &str, type_name: &str| ColumnMeta {
            name: name.into(),
            type_name: type_name.into(),
            kind: crate::ValueKind::Other,
        };
        let columns = [
            column("id", "int"),
            column("at", "timestamp"),
            column("ratio", "float unsigned"),
            column("name", "varchar"),
        ];
        assert_eq!(inexact_key(&change(&["id"], &["name"]), &columns), None);
        // Only a key's column: a save may set one of such a type.
        assert_eq!(inexact_key(&change(&["id"], &["at"]), &columns), None);
        assert_eq!(
            inexact_key(&change(&["id", "at"], &["name"]), &columns),
            Some(("at", "timestamp"))
        );
        assert_eq!(
            inexact_key(&change(&["ratio"], &["name"]), &columns),
            Some(("ratio", "float"))
        );
    }

    #[test]
    fn only_a_name_spelled_as_the_table_spells_it_is_taken() {
        let columns = ["id".to_owned(), "Name".to_owned()];
        assert_eq!(
            spelled_otherwise(&change(&["id"], &["Name"]), &columns),
            None
        );
        assert_eq!(
            spelled_otherwise(&change(&["ID"], &["Name"]), &columns),
            Some("ID")
        );
        assert_eq!(
            spelled_otherwise(&change(&["id"], &["name"]), &columns),
            Some("name")
        );
    }

    /// A writable connection outside the adapter, for arranging and probing.
    async fn admin(url: &str) -> mysql_async::Conn {
        let opts = Opts::from_url(&format!("{url}?prefer_socket=false")).unwrap();
        mysql_async::Conn::new(opts).await.unwrap()
    }

    /// Runs `test` on a table of its own, `table (id INT PRIMARY KEY, body
    /// VARCHAR(3), amount DECIMAL(6, 2))` with the row `(1, 'abc', 1.00)`,
    /// dropped however the test ends. The fixture's tables are every
    /// test's, and no test writes to them.
    async fn on_its_own_table<T>(url: &str, table: &str, test: impl Future<Output = T>) -> T {
        let columns = "id INT PRIMARY KEY, body VARCHAR(3), amount DECIMAL(6, 2)";
        on_a_table_of(url, table, columns, "(1, 'abc', 1.00)", test).await
    }

    /// Runs `test` on a table of its own with these columns and rows,
    /// dropped however the test ends. The rows are written with UTC as the
    /// time zone.
    async fn on_a_table_of<T>(
        url: &str,
        table: &str,
        columns: &str,
        rows: &str,
        test: impl Future<Output = T>,
    ) -> T {
        let mut admin = admin(url).await;
        let drop = format!("DROP TABLE IF EXISTS {table}");
        admin.query_drop(&drop).await.unwrap();
        let outcome = AssertUnwindSafe(async {
            admin
                .query_drop(format!("CREATE TABLE {table} ({columns})"))
                .await
                .unwrap();
            admin
                .query_drop("SET SESSION time_zone = '+00:00'")
                .await
                .unwrap();
            admin
                .query_drop(format!("INSERT INTO {table} VALUES {rows}"))
                .await
                .unwrap();
            test.await
        })
        .catch_unwind()
        .await;
        // A transaction a save left open would hold the table for good.
        admin
            .query_drop("SET SESSION lock_wait_timeout = 10")
            .await
            .unwrap();
        let dropped = admin.query_drop(&drop).await;
        let value = outcome.unwrap_or_else(|panic| std::panic::resume_unwind(panic));
        dropped.unwrap();
        value
    }

    /// The row 1 of `table` gets `new` in `column`, which the page held as
    /// `loaded`.
    fn set(table: &str, column: &str, type_name: &str, loaded: Value, new: &str) -> ChangeSet {
        ChangeSet {
            object: ObjectRef::new("tabletist", table),
            inserts: Vec::new(),
            rows: vec![RowChange {
                key: vec![("id".into(), Value::Int(1))],
                set: vec![CellChange {
                    column: column.into(),
                    type_name: type_name.into(),
                    loaded,
                    new: NewValue::Text(new.into()),
                }],
            }],
        }
    }

    fn body(table: &str, loaded: &str, new: &str) -> ChangeSet {
        set(table, "body", "varchar(3)", Value::Text(loaded.into()), new)
    }

    /// The table's one row, as another session reads it.
    async fn stored(url: &str, table: &str) -> (String, String) {
        admin(url)
            .await
            .query_first(format!("SELECT body, CAST(amount AS CHAR) FROM {table}"))
            .await
            .unwrap()
            .unwrap()
    }

    /// Whether the session is in a transaction, and its own read-only
    /// setting.
    async fn standing(conn: &Conn) -> (bool, i64) {
        let mut conn = conn.conn.lock().await;
        let read_only: i64 = conn
            .query_first("SELECT @@session.transaction_read_only")
            .await
            .unwrap()
            .unwrap();
        (
            status(&conn).contains(StatusFlags::SERVER_STATUS_IN_TRANS),
            read_only,
        )
    }

    /// The unit tests' session is the app's read-only one: every
    /// transaction it starts is read-only unless it says otherwise, which
    /// is also what a script's fence makes of a writable session. The front
    /// door refuses a save on it; past the door it is a test of the
    /// transaction saying read-write itself, and of the session standing
    /// afterwards as it stood, however the save ended.
    #[tokio::test]
    async fn a_save_is_read_write_on_a_session_that_is_not_and_leaves_it_as_it_was() {
        let Some(url) = test_url() else {
            return;
        };
        on_its_own_table(&url, "write_unit_standing", async {
            let table = "write_unit_standing";
            let conn = session(&url).await;
            assert_eq!(standing(&conn).await, (false, 1));
            type Ended = fn(&Result<WriteOutcome>) -> bool;
            let exits: [(&str, ChangeSet, Ended); 6] = [
                ("a conflict", body(table, "xyz", "new"), |ended| {
                    matches!(ended, Ok(WriteOutcome::Conflicts(_)))
                }),
                (
                    "a statement the server refuses",
                    body(table, "abc", "too long"),
                    |ended| matches!(ended, Ok(WriteOutcome::Failed { row: 0, .. })),
                ),
                (
                    "a column the table does not have",
                    set(table, "nick", "varchar(3)", Value::Null, "new"),
                    |ended| matches!(ended, Err(Error::Query { .. })),
                ),
                (
                    "a name the table spells otherwise",
                    set(
                        table,
                        "BODY",
                        "varchar(3)",
                        Value::Text("abc".into()),
                        "new",
                    ),
                    |ended| matches!(ended, Err(Error::Query { .. })),
                ),
                (
                    "a table that is not there",
                    body("write_unit_nowhere", "abc", "new"),
                    |ended| matches!(ended, Err(Error::Unsupported(_))),
                ),
                (
                    "a save that is written",
                    body(table, "abc", "new"),
                    |ended| matches!(ended, Ok(WriteOutcome::Written { .. })),
                ),
            ];
            for (exit, changes, ended_so) in exits {
                let ended = conn.write(&changes, &StopFlag::new()).await;
                assert!(ended_so(&ended), "{exit}: {ended:?}");
                assert_eq!(standing(&conn).await, (false, 1), "{exit}");
            }
            assert_eq!(stored(&url, table).await.0, "new");
        })
        .await;
    }

    /// With `completion_type` at `CHAIN`, which a server can have as its
    /// default, a plain `COMMIT` or `ROLLBACK` starts the next transaction
    /// as it ends this one, read-write like it. A save's do not.
    #[tokio::test]
    async fn a_save_ends_its_transaction_where_a_commit_would_start_the_next() {
        let Some(url) = test_url() else {
            return;
        };
        on_its_own_table(&url, "write_unit_chain", async {
            let conn = session(&url).await;
            conn.conn
                .lock()
                .await
                .query_drop("SET SESSION completion_type = 'CHAIN'")
                .await
                .unwrap();
            let conflict = conn
                .write(&body("write_unit_chain", "xyz", "new"), &StopFlag::new())
                .await;
            assert!(
                matches!(conflict, Ok(WriteOutcome::Conflicts(_))),
                "{conflict:?}"
            );
            assert_eq!(standing(&conn).await, (false, 1));
            let written = conn
                .write(&body("write_unit_chain", "abc", "new"), &StopFlag::new())
                .await;
            assert!(
                matches!(written, Ok(WriteOutcome::Written { .. })),
                "{written:?}"
            );
            assert_eq!(standing(&conn).await, (false, 1));
            assert_eq!(stored(&url, "write_unit_chain").await.0, "new");
        })
        .await;
    }

    /// Outside strict mode the server cuts a value to fit and says so only
    /// in a warning. A script cannot set `sql_mode`, but a server can have
    /// that as its default: the warning fails the save, with its text.
    #[tokio::test]
    async fn a_warning_rolls_the_save_back() {
        let Some(url) = test_url() else {
            return;
        };
        on_its_own_table(&url, "write_unit_warning", async {
            let table = "write_unit_warning";
            let conn = session(&url).await;
            conn.conn
                .lock()
                .await
                .query_drop("SET SESSION sql_mode = ''")
                .await
                .unwrap();
            let outcome = conn
                .write(&body(table, "abc", "abcdef"), &StopFlag::new())
                .await;
            assert!(
                matches!(
                    &outcome,
                    Ok(WriteOutcome::Failed { row: 0, error: Error::Query { code, message, .. } })
                        if message.contains("truncated") && code.as_deref() == Some("1265")
                ),
                "{outcome:?}"
            );
            assert_eq!(stored(&url, table).await.0, "abc");
            // In any mode a decimal with more places than the column keeps
            // is rounded, with a note: what was typed is not what would be
            // stored.
            let conn = session(&url).await;
            let amount = Value::Text("1.00".into());
            let outcome = conn
                .write(
                    &set(table, "amount", "decimal(6,2)", amount.clone(), "99.955"),
                    &StopFlag::new(),
                )
                .await;
            assert!(
                matches!(
                    &outcome,
                    Ok(WriteOutcome::Failed { row: 0, error: Error::Query { message, .. } })
                        if message.contains("truncated")
                ),
                "{outcome:?}"
            );
            assert_eq!(stored(&url, table).await.1, "1.00");
            // What fits is written.
            let outcome = conn
                .write(
                    &set(table, "amount", "decimal(6,2)", amount, "99.95"),
                    &StopFlag::new(),
                )
                .await;
            assert!(
                matches!(outcome, Ok(WriteOutcome::Written { .. })),
                "{outcome:?}"
            );
            assert_eq!(stored(&url, table).await.1, "99.95");
        })
        .await;
    }

    /// With `sql_notes` off, which a server can have as its default, the
    /// server rounds a decimal without a word, and the warning check would
    /// see nothing. The session's own setting stands in for the server's
    /// default here, which a test cannot set: the settings every connect
    /// and every reset end with are applied over it.
    #[tokio::test]
    async fn a_rounded_value_fails_the_save_where_the_servers_default_is_no_notes() {
        let Some(url) = test_url() else {
            return;
        };
        async fn notes(conn: &Conn) -> Option<i64> {
            conn.conn
                .lock()
                .await
                .query_first("SELECT @@session.sql_notes LIMIT 1")
                .await
                .unwrap()
        }
        on_its_own_table(&url, "write_unit_notes", async {
            let table = "write_unit_notes";
            let conn = session(&url).await;
            assert_eq!(notes(&conn).await, Some(1));
            {
                let mut conn = conn.conn.lock().await;
                conn.query_drop("SET SESSION sql_notes = 0").await.unwrap();
                prepare_session(&mut conn, Access::ReadOnly).await.unwrap();
            }
            assert_eq!(notes(&conn).await, Some(1));
            let amount = Value::Text("1.00".into());
            let outcome = conn
                .write(
                    &set(table, "amount", "decimal(6,2)", amount, "99.955"),
                    &StopFlag::new(),
                )
                .await;
            assert!(
                matches!(
                    &outcome,
                    Ok(WriteOutcome::Failed { row: 0, error: Error::Query { message, .. } })
                        if message.contains("truncated")
                ),
                "{outcome:?}"
            );
            assert_eq!(stored(&url, table).await.1, "1.00");
            // And after a script, whose reset undoes what the connect set.
            let script = ["SET sql_notes = 0".to_owned()];
            conn.script(&script, 10, crate::ScriptMode::ReadOnly, &StopFlag::new())
                .await
                .unwrap();
            assert_eq!(notes(&conn).await, Some(1));
        })
        .await;
    }

    /// A save starts from no transaction. Nothing in the app leaves one
    /// open (a save and a script each end their own), but a save whose
    /// future was dropped half way would, and `START TRANSACTION` commits
    /// what is open: the next save would commit that one's rows.
    #[tokio::test]
    async fn a_save_does_not_commit_what_a_transaction_left_open_wrote() {
        let Some(url) = test_url() else {
            return;
        };
        on_its_own_table(&url, "write_unit_left_open", async {
            let table = "write_unit_left_open";
            let conn = session(&url).await;
            {
                let mut conn = conn.conn.lock().await;
                conn.query_drop("START TRANSACTION READ WRITE")
                    .await
                    .unwrap();
                conn.query_drop(format!("INSERT INTO {table} VALUES (2, 'pln', 2.00)"))
                    .await
                    .unwrap();
            }
            let outcome = conn
                .write(&body(table, "abc", "new"), &StopFlag::new())
                .await;
            assert!(
                matches!(outcome, Ok(WriteOutcome::Written { .. })),
                "{outcome:?}"
            );
            assert_eq!(standing(&conn).await, (false, 1));
            let bodies: Vec<String> = admin(&url)
                .await
                .query(format!("SELECT body FROM {table} ORDER BY id"))
                .await
                .unwrap();
            assert_eq!(bodies, ["new"]);
        })
        .await;
    }

    /// The engine is asked for with a `LIMIT` of its own: a server whose
    /// default `sql_select_limit` is 0 gives no row otherwise, and every
    /// save would be refused as if its table had no transactions.
    #[tokio::test]
    async fn a_save_finds_the_tables_engine_whatever_the_select_limit() {
        let Some(url) = test_url() else {
            return;
        };
        on_its_own_table(&url, "write_unit_limit", async {
            let conn = session(&url).await;
            conn.conn
                .lock()
                .await
                .query_drop("SET SESSION sql_select_limit = 0")
                .await
                .unwrap();
            let outcome = conn
                .write(&body("write_unit_limit", "abc", "new"), &StopFlag::new())
                .await;
            assert!(
                matches!(outcome, Ok(WriteOutcome::Written { .. })),
                "{outcome:?}"
            );
            assert_eq!(stored(&url, "write_unit_limit").await.0, "new");
        })
        .await;
    }

    /// A `TIMESTAMP` is shown in the session's time zone, without the zone.
    /// In the hour a zone repeats when its clocks go back, two instants
    /// read alike, and the key of one would find the other: the second row
    /// here, edited alone, was written to the first. So such a key is
    /// refused, and neither row changes.
    #[tokio::test]
    async fn a_timestamp_key_is_refused_where_two_instants_read_alike() {
        let Some(url) = test_url() else {
            return;
        };
        let table = "write_unit_instants";
        let columns = "at TIMESTAMP PRIMARY KEY, body VARCHAR(8)";
        // Half past two in Berlin, before and after the clocks go back.
        let rows = "('2026-10-25 00:30:00', 'before'), ('2026-10-25 01:30:00', 'before')";
        on_a_table_of(&url, table, columns, rows, async {
            let conn = session(&url).await;
            conn.conn
                .lock()
                .await
                .query_drop("SET SESSION time_zone = 'Europe/Berlin'")
                .await
                .unwrap();
            let object = ObjectRef::new("tabletist", table);
            let page = conn
                .fetch_rows(&RowQuery::new(object.clone(), 10))
                .await
                .unwrap();
            let shown = Value::Text("2026-10-25 02:30:00".into());
            assert_eq!(page.rows[0][0], shown);
            assert_eq!(page.rows[1][0], shown);
            let changes = ChangeSet {
                object,
                inserts: Vec::new(),
                rows: vec![RowChange {
                    key: vec![("at".into(), page.rows[1][0].clone())],
                    set: vec![CellChange {
                        column: "body".into(),
                        type_name: "varchar(8)".into(),
                        loaded: page.rows[1][1].clone(),
                        new: NewValue::Text("after".into()),
                    }],
                }],
            };
            let outcome = conn.write(&changes, &StopFlag::new()).await;
            assert!(
                matches!(
                    &outcome,
                    Err(Error::Query { message, .. })
                        if message.contains("at is a timestamp column")
                            && message.contains("cannot be matched exactly")
                ),
                "{outcome:?}"
            );
            assert_eq!(standing(&conn).await, (false, 1));
            let bodies: Vec<String> = admin(&url)
                .await
                .query(format!("SELECT body FROM {table} ORDER BY at"))
                .await
                .unwrap();
            assert_eq!(bodies, ["before", "before"]);
        })
        .await;
    }

    /// What a save would feel of a session, were a script to leave it
    /// there: how text is read and in which character set, whether foreign
    /// keys and unique indexes are checked, what the clock says, whether a
    /// rounded value is noted, how many rows a read gives. MySQL does not
    /// undo a `SET` with the transaction; the reset that ends every script
    /// does, and the connect-time settings are applied again. Run past the
    /// refusal, which stops some of these before the server sees them.
    #[tokio::test]
    async fn what_a_script_sets_does_not_outlast_it() {
        let Some(url) = test_url() else {
            return;
        };
        const SETTINGS: [&str; 16] = [
            "@@session.sql_mode",
            "@@session.character_set_client",
            "@@session.character_set_connection",
            "@@session.collation_connection",
            "@@session.foreign_key_checks",
            "@@session.unique_checks",
            "@@session.sql_safe_updates",
            "@@session.time_zone",
            "@@session.innodb_lock_wait_timeout",
            "@@session.transaction_isolation",
            "@@session.autocommit",
            "@@session.sql_notes",
            "@@session.sql_select_limit",
            "@@session.sql_auto_is_null",
            // The clock a column's ON UPDATE CURRENT_TIMESTAMP reads.
            "@@session.timestamp > 1000000000",
            "@@session.transaction_read_only",
        ];
        const SETS: [&str; 14] = [
            "SET sql_mode = 'NO_BACKSLASH_ESCAPES,ANSI_QUOTES,PIPES_AS_CONCAT'",
            "SET NAMES latin1",
            "SET foreign_key_checks = 0",
            "SET unique_checks = 0",
            "SET sql_safe_updates = 1",
            "SET time_zone = '+05:00'",
            "SET innodb_lock_wait_timeout = 1",
            "SET SESSION transaction_isolation = 'READ-UNCOMMITTED'",
            "SET autocommit = 0",
            "SET sql_notes = 0",
            "SET sql_select_limit = 0",
            "SET sql_auto_is_null = 1",
            "SET timestamp = 1",
            "SET @tabletist_left_over = 1",
        ];
        async fn settings(conn: &Conn) -> Vec<String> {
            let row: mysql_async::Row = conn
                .conn
                .lock()
                .await
                .query_first(format!(
                    "SELECT {}, @tabletist_left_over LIMIT 1",
                    SETTINGS.join(", ")
                ))
                .await
                .unwrap()
                .unwrap();
            row.unwrap()
                .into_iter()
                .map(|value| format!("{value:?}"))
                .collect()
        }
        let conn = session(&url).await;
        let connected = settings(&conn).await;
        let script: Vec<String> = SETS.iter().map(|&set| set.to_owned()).collect();
        let outcome = conn
            .script(&script, 10, crate::ScriptMode::ReadOnly, &StopFlag::new())
            .await
            .unwrap();
        // Every one of them took, inside the script.
        assert_eq!(outcome.results.len(), SETS.len(), "{outcome:?}");
        assert!(
            outcome
                .results
                .iter()
                .all(|result| matches!(result.outcome, crate::StatementOutcome::Done { .. })),
            "{outcome:?}"
        );
        let after = settings(&conn).await;
        for ((name, connected), after) in SETTINGS
            .iter()
            .chain(&["@tabletist_left_over"])
            .zip(&connected)
            .zip(&after)
        {
            assert_eq!(after, connected, "{name}");
        }
    }
}
