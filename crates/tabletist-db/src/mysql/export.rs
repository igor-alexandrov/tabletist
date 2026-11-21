//! An export's reads on MySQL and MariaDB, in as few round trips as the
//! rows allow: a table's shape in one message, then its rows (or a SQL
//! result's) in one more, streamed as they arrive (see
//! [`Conn::stream_in_one`](super::Conn)).

use super::{BINARY_COLUMNS, PRIMARY_KEY, Shape, inlined, query_error, script, typed};
use crate::{Error, ObjectRef, Result};

/// Whether a table has a FLOAT column, as [`super::LISTED_COLUMNS`] tells
/// one: any column whose type starts with it.
const TABLE_FLOATS: &str = "SELECT EXISTS (SELECT 1 FROM information_schema.columns \
                            WHERE table_schema = ? AND table_name = ? \
                              AND LOWER(column_type) LIKE 'float%')";

/// `object`'s [`Shape`] in one round trip: its primary key, its byte
/// string columns and whether it has a FLOAT, asked as one message.
pub(super) async fn read_shape(conn: &mut mysql_async::Conn, object: &ObjectRef) -> Result<Shape> {
    let at = [object.schema.as_str(), object.name.as_str()];
    let questions = [
        inlined(PRIMARY_KEY, &at),
        inlined(BINARY_COLUMNS, &at),
        inlined(TABLE_FLOATS, &at),
    ];
    let (answers, error) = script::answers(conn, &questions).await;
    if let Some(error) = error {
        return Err(query_error(error));
    }
    let Ok([key, binary, floats]) = <[script::Answer; 3]>::try_from(answers) else {
        return Err(Error::query(
            "unexpected data from the server: an answer is missing",
        ));
    };
    Ok(Shape {
        key: typed(key.rows)?,
        binary: typed(binary.rows)?,
        floats: typed::<bool>(floats.rows)?
            .first()
            .copied()
            .unwrap_or(false),
    })
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::Ordering;

    use mysql_async::prelude::Queryable;

    use super::super::Conn;
    use super::super::tests::{counted_session, in_transaction, test_url};
    use crate::adapter::Adapter;
    use crate::{Access, ColumnMeta, ObjectRef, Result, RowQuery, StopFlag, Streamed, Value};

    /// One test at a time creates the export tables: creating one twice
    /// at once can fail.
    static TABLES: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

    /// The digits 0 to 9, as rows of `d`, to build numbered rows from on
    /// MySQL and MariaDB alike (MariaDB has no `cte_max_recursion_depth`).
    const DIGITS: &str = "(SELECT 0 d UNION ALL SELECT 1 UNION ALL SELECT 2 UNION ALL SELECT 3 \
                          UNION ALL SELECT 4 UNION ALL SELECT 5 UNION ALL SELECT 6 \
                          UNION ALL SELECT 7 UNION ALL SELECT 8 UNION ALL SELECT 9)";

    /// The export tables, in the test database, and its name:
    /// `export_trips` (12,000 rows of an `int` key and a name),
    /// `export_floats` (one FLOAT row) and `export_types` (a row of each
    /// common type, and a row of NULLs).
    async fn export_tables(url: &str) -> String {
        let _turn = TABLES.lock().await;
        let opts = mysql_async::Opts::from_url(&format!("{url}?prefer_socket=false")).unwrap();
        let mut admin = mysql_async::Conn::new(opts).await.unwrap();
        admin
            .query_drop(format!(
                "CREATE TABLE IF NOT EXISTS export_trips (id int PRIMARY KEY, name varchar(20)); \
                 INSERT IGNORE INTO export_trips (id, name) \
                 SELECT n + 1, CONCAT('row ', n + 1) FROM ( \
                   SELECT a.d + 10 * b.d + 100 * c.d + 1000 * e.d + 10000 * f.d AS n \
                   FROM {DIGITS} a, {DIGITS} b, {DIGITS} c, {DIGITS} e, {DIGITS} f) g \
                 WHERE n < 12000; \
                 CREATE TABLE IF NOT EXISTS export_floats (id int PRIMARY KEY, f float); \
                 INSERT IGNORE INTO export_floats VALUES (1, 1.2345678); \
                 CREATE TABLE IF NOT EXISTS export_types (id int PRIMARY KEY, \
                   big bigint unsigned, money decimal(10, 2), ratio double, \
                   stamp datetime(3), day date, span time, yr year, name varchar(20), \
                   bytes varbinary(4), doc json); \
                 INSERT IGNORE INTO export_types VALUES \
                   (1, 18446744073709551615, 12.34, 0.1, '2024-01-02 03:04:05.678', \
                    '2024-01-02', '-01:02:03', 2024, 'ñame', 0x00ff10, '{{\"a\": [1, 2]}}'), \
                   (2, NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL)"
            ))
            .await
            .unwrap();
        admin
            .query_first("SELECT DATABASE()")
            .await
            .unwrap()
            .unwrap()
    }

    /// A table export as the export runs one: the statement it shows,
    /// then its rows streamed, with what `to` was sent.
    async fn exported_table(
        conn: &Conn,
        query: &RowQuery,
        columns: &[String],
        stop: &StopFlag,
    ) -> (
        Result<String>,
        Result<u64>,
        Vec<ColumnMeta>,
        Vec<Vec<Vec<Value>>>,
    ) {
        let shown = conn.export_statement(query, columns).await;
        let (sent, metas, batches) =
            streamed(|to| async move { conn.stream_rows(query, columns, 5_000, &to, stop).await })
                .await;
        (shown, sent, metas, batches)
    }

    /// A SQL result's export: what it sent `to`.
    async fn exported_statement(
        conn: &Conn,
        text: &str,
        stop: &StopFlag,
    ) -> (Result<u64>, Vec<ColumnMeta>, Vec<Vec<Vec<Value>>>) {
        streamed(|to| async move { conn.stream_statement(text, 5_000, &to, stop).await }).await
    }

    /// Runs `stream` with a sender whose receiver is read as it goes, as
    /// the export's writer reads it: the outcome, the columns, the batches.
    async fn streamed<F, Fut>(stream: F) -> (Result<u64>, Vec<ColumnMeta>, Vec<Vec<Vec<Value>>>)
    where
        F: FnOnce(tokio::sync::mpsc::Sender<Streamed>) -> Fut,
        Fut: std::future::Future<Output = Result<u64>>,
    {
        let (to, mut from) = tokio::sync::mpsc::channel(1);
        let reading = async move {
            let (mut columns, mut batches) = (Vec::new(), Vec::new());
            while let Some(streamed) = from.recv().await {
                match streamed {
                    Streamed::Columns(metas) => columns = metas,
                    Streamed::Rows(rows) => batches.push(rows),
                }
            }
            (columns, batches)
        };
        let (sent, (columns, batches)) = tokio::join!(stream(to), reading);
        (sent, columns, batches)
    }

    fn trips_query(schema: &str) -> (RowQuery, Vec<String>) {
        (
            RowQuery::new(ObjectRef::new(schema, "export_trips"), 0),
            vec!["id".to_owned(), "name".to_owned()],
        )
    }

    /// A table export reads the table's shape once, for the statement it
    /// shows and for its rows: the shape's one message, the start, the
    /// prepare, the run, the rollback.
    #[tokio::test]
    async fn a_table_export_reads_its_shape_once() {
        let Some(url) = test_url() else {
            return;
        };
        let schema = export_tables(&url).await;
        let (conn, asked) = counted_session(&url, Access::ReadOnly).await;
        let (query, columns) = trips_query(&schema);
        let before = asked.load(Ordering::SeqCst);
        let (shown, sent, _, batches) =
            exported_table(&conn, &query, &columns, &StopFlag::new()).await;
        let trips = asked.load(Ordering::SeqCst) - before;
        assert!(shown.unwrap().contains("ORDER BY"));
        assert_eq!(sent.unwrap(), 12_000);
        let sizes: Vec<usize> = batches.iter().map(Vec::len).collect();
        assert_eq!(sizes, [5_000, 5_000, 2_000]);
        assert_eq!(trips, 5);
        assert!(!in_transaction(&conn).await);
    }
}
