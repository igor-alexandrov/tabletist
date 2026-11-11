# Fewer round trips to open and page a PostgreSQL table

Far from the server (~200 ms a round trip) every await on the driver that
waits for an answer costs a trip, and a session runs its commands one at a
time, so the trips add up. #303 cut the SQL editor's read-only run from 7
trips to 4. This plan does the same for the table view.

## Where the trips go today

| Path | Trips | What |
| --- | --- | --- |
| A page, a sort, a filter (`fetch_rows`) | 6, or 8 with a bytes filter | key 2 (`client.query(&str)` prepares, then runs), bytea columns 0 or 2, then the read: BEGIN, prepare, query, ROLLBACK |
| The exact count (`count_rows`) | 4, or 6 | bytea columns 0 or 2, then BEGIN, prepare, query, ROLLBACK |
| A foreign key's lookup (`lookup`) | 4 | the read |
| The structure (`describe`) | 14 | seven catalog queries of 2 trips each, one after another: relation, columns, indexes, keys, keys pointing here, comment, primary key |
| Opening a table | 20 before the first row | the app sends Describe, then FetchRows, and the session runs them in that order |

## Steps, one PR each

### 1. A page in 3 trips (#304)

- A trip counter for tests: a proxy in front of the test server that holds
  back the server's answers a little, so what the client sends together
  arrives together, and counts each time the client sends while it waits on
  nothing.
- `catalog()` runs through `query_typed`, its parameters typed (text, oid,
  bool): one trip each, not two. Every catalog query gains, the sidebar's
  listings and the structure's too.
- The key and the bytea columns come from one catalog query.
- The read: `BEGIN READ ONLY` and the prepare go out together, then the
  query and its ROLLBACK as one message. The prepare still runs inside the
  read-only transaction and still answers before the query is sent, so it
  still refuses a second statement in a raw WHERE (`query.rs`), and the
  transaction still keeps writes out on a writable session. On an error the
  ROLLBACK is sent on its own.
- `count_rows` and `lookup` read the same way: 2 trips (3 for a count with a
  bytes filter). The export's statement and stream use the one catalog query
  too.
- Result: a page 6–8 → 3, a count 4–6 → 2–3, a lookup 4 → 2, the structure
  14 → 7.

### 2. The structure in 1 trip

- The five catalog queries go out together under the one lock
  (`try_join!`). Each finds the table by its schema and name itself, so
  none waits for the relation's oid; the comment's query also says whether
  the table is there.
- The primary key from the index rows, which carry `primary` and
  `key_columns`; no query of its own.
- A test counts the trips of opening a table: structure and page.
- Result: the structure 7 → 1, opening a table 10 → 4.

### 3. Rows first (app)

- `open_object` and the reopened-tab branch of `fetch_rows` send FetchRows
  before Describe, as Reload does: the first rows wait on nothing. MySQL
  gains too.
- Result: opening a table shows rows after 3 trips, and its structure
  after 4; 20 before the plan.

## Later, not in this plan

- `pg/ddl.rs` and `pg/hidden.rs` still call `client.query(&str)`: 2 trips
  each, and the DDL view's run one after another.
- A page could keep the table's key from the last page and skip the catalog
  query; a stale key would order a page wrong, so it needs care.
