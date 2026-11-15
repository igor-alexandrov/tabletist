# Opening a table in one round trip — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use rubypowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Opening a PostgreSQL, MySQL or MariaDB table puts its rows and its structure on screen after one round trip, not two (0.41 s → 0.20 s at a 200 ms round trip), first open included. The page stays sorted by the primary key, and every read-only guard stays as it is.

**Architecture:** Two runs, a PR each. Run 1 sends a table's structure and its first page in one flight, as one new backend command (`Command::Open`) and one new `Connection::open`. That alone takes a reopen, and any open after the session has read the table's shape, to one trip. Run 2 reads every listed table's shape (key and byte columns, and on MySQL whether it has a FLOAT) in the table listing's own round trip, so the first open knows the shape too. A shape read with the listing can be stale; the page already checks the shape in its own flight and reads again when it changed (`fetch_rows`, PostgreSQL and MySQL), so a stale shape costs one more trip, never a wrong page.

**Tech Stack:** Rust, tokio-postgres 0.7 (pipelined queries), mysql_async 0.37.1 (one multi-statement message through `script::answers`), the test PostgreSQL on 55432, MySQL 8.4 on 53306, MariaDB 11.4 on 53307.

---

## Where the trips go

Measured on main 1d72801e through 100 ms each way (the throwaway `slow_bench`): open a table 409 ms on all three servers, of it 205 ms the structure.

- **Today:** `App::open_object` (`src/app.rs`) sends `Command::Describe`, then `Command::FetchRows`. The session's worker runs one command at a time, so the structure is one trip and the page another.
- **The page's statement needs the key** (`ORDER BY` the primary key). PostgreSQL's `fetch_rows` builds it from `known_shape`, which `describe` filled, and reads the shape again alongside the page (`read_alongside`); MySQL's does the same in one message (`page_in_one`). With no known shape, both read the shape first: one trip more.
- **TablePro** opens a PostgreSQL table in 0.20 s by sending `SELECT * … LIMIT 1000` with no `ORDER BY`. Not done here: the sort by key is what keeps pages stable.

### After

- **Run 1:** `Command::Open` runs `Connection::open(object, query)`: on PostgreSQL the structure's six queries go *alongside* the page in `read_alongside`, inside its `BEGIN READ ONLY … ROLLBACK`; on MySQL the structure's questions go in the page's one message. With a known shape: 1 trip. Without one: the shape first, then the flight: 2 trips, as today.
- **Run 2:** `list_objects` also reads every listed table's shape in the same flight (PostgreSQL) or message (MySQL), and remembers each. The first open then has a known shape: 1 trip.

### Decisions

- **One command, not two in a row.** `open_object` sends `Command::Open { session, structure, rows, object, query }` with both request ids. Two commands in a row cannot be merged reliably: the worker can take the first before the second is sent.
- **Cancels.** A `Cancel` naming either request applies to the Open (one flight on the server). Whatever part was not cancelled still gets its event: its result when the flight finished, or, when the cancel stopped the flight, that part alone run again (`describe` or `fetch_rows`). The skip set works the same for a queued Open: both ids skipped, the Open is skipped; one id skipped, only the other part runs.
- **Events.** The Open answers with the same `Event::Structure` and `Event::Rows` as today, rows first. The app's handling does not change.
- **MySQL's CHECK question goes last, after the page's `ROLLBACK`.** A server without the list refuses it, and a refusal stops the rest of a multi-statement message: last, it loses nothing (as in `describe` today). The refused case then asks the other kind's question alone, as `describe` does.
- **FLOAT tables on MySQL** keep today's way (`page_in_three`) for the page: `Connection::open` then runs `describe` and `fetch_rows` one after the other. Same trips as today.
- **SQLite and BigQuery:** `Adapter::open` has a default that runs `describe`, then `fetch_rows`. Nothing changes for them.
- **The listing's shapes are capped by the listing:** only the tables `list_objects` returns (at most `MAX_LISTED`). On MySQL and MariaDB, `information_schema.columns` for a whole schema is the cost; Task 7 measures it on a schema of 1,000 tables and the plan stops there for a yes if the listing gets more than 50 ms slower.

## Run 1: structure and rows together

### Task 1: count the trips (red)

**Files:** `crates/tabletist-db/src/pg.rs` (tests), `crates/tabletist-db/src/mysql.rs` (tests)

- [ ] `opening_a_table_asks_the_server_once` in `pg::tests`, next to `opening_a_table_asks_the_server_twice`: `round_trips_table`, `trip_counted`; `conn.describe` once (fills the shape, as an earlier open would), then counted: `conn.open(&object, &RowQuery::new(object.clone(), 10))`. Expect the structure's primary key `["id"]`, 2 rows, `ordered_by_key`, and 1 trip.
- [ ] The same in `mysql::tests` with `counted_session`, for `Access::ReadOnly` and `Access::Writable`: 1 message, and `!in_transaction(&conn)` after.
- [ ] Run: `~/.cargo/bin/cargo test -p tabletist-db opening_a_table_asks_the_server_once` with `TABLETIST_TEST_PG_URL` and `TABLETIST_TEST_MYSQL_URL` set. Expected: fails to compile (`open` does not exist yet).

### Task 2: `Adapter::open` and `Connection::open`

**Files:** `crates/tabletist-db/src/adapter.rs`, `crates/tabletist-db/src/lib.rs`

- [ ] `Adapter::open(&self, object: &ObjectRef, query: &RowQuery) -> (Result<Structure>, Result<RowPage>)`, with a default body: `describe`, then `fetch_rows`.
- [ ] `Connection::open` dispatches it like `describe`. Doc: one round trip where the adapter can, and the same results as `describe` then `fetch_rows`.

### Task 3: PostgreSQL's open in one flight

**Files:** `crates/tabletist-db/src/pg.rs`

- [ ] Split `describe` into `structure_reads(&client, object)` (the `try_join!` of its six queries, returning the raw rows) and `structure_from(rows)` (the parsing, the `remember` of the shape, and the `primary_key` fallback).
- [ ] `open`: no known shape → `table_shape` first and remember it. Then build the page from the shape, and `read_alongside(&client, &page_sql, structure_reads(&client, object))`. If the structure's shape differs for this query (`for_query`), read the page again from it, as `fetch_rows` does.
- [ ] A cancel that lands on the flight fails both parts with `Error::Cancelled`.
- [ ] Task 1's PostgreSQL test passes; `opening_a_table_asks_the_server_twice` still passes (`describe` and `fetch_rows` unchanged).

### Task 4: MySQL's open in one message

**Files:** `crates/tabletist-db/src/mysql.rs`

- [ ] `structure_questions` returns the questions without the checks and the checks question apart.
- [ ] `open` with a known shape and no FLOAT: one message: `read_start`, the structure's questions, `PRIMARY_KEY` (and `BINARY_COLUMNS` when the filter reads bytes), `prepared_run`, `ROLLBACK`, then the checks question. Parse the structure as `describe` does, and the page as `page_in_one` does (`answered`). A refused checks question: the other kind alone, as `describe` does.
- [ ] Otherwise: `describe`, then `fetch_rows`.
- [ ] Task 1's MySQL test passes against MySQL, and against MariaDB with `TABLETIST_TEST_MYSQL_URL` pointed at 53307.

### Task 5: `Command::Open` in the backend and the app

**Files:** `crates/tabletist-core/src/backend.rs`, `src/app.rs`

- [ ] `Command::Open { session, structure: RequestId, rows: RequestId, object: ObjectRef, query: RowQuery }`, added everywhere `Describe` and `FetchRows` are matched (session of, request of, the error event, `running.read`).
- [ ] The worker runs `connection.open` and emits `Event::Rows`, then `Event::Structure`. A Cancel or skip of one of its ids follows the Decisions above.
- [ ] `App::open_object` sends `Command::Open` in place of `describe` + `fetch_rows`, starting both `object.structure` and `object.rows` with their ids. The reopen path and every other caller keep `Describe` and `FetchRows`.
- [ ] Update the app tests that look for `Command::Describe`/`Command::FetchRows` after an open (`src/app.rs`, `src/app/startup_tests.rs`). New backend tests: an Open answers both events; a Cancel of its rows id answers the structure and cancels the rows; a skip of one id runs the other part alone.

### Task 6: check, measure and ship run 1

- [ ] `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --locked --workspace` with the PostgreSQL and MySQL URLs, the MySQL suites again against MariaDB.
- [ ] Measure with `slow_bench` (throwaway, not committed), its open done with `conn.open` after a `describe`: expected 0.20 s on all three.
- [ ] PR against main.

## Run 2: shapes with the table listing

### Task 7: listing reads the shapes

**Files:** `crates/tabletist-db/src/pg.rs`, `crates/tabletist-db/src/mysql.rs` (code and tests)

- [ ] Red: `the_first_open_asks_the_server_once` (both adapters): a fresh session, `list_objects(schema)`, then counted `open`: 1 trip.
- [ ] PostgreSQL: `list_objects` sends a second query in the same `try_join!`: every table of the schema with its key columns in key order and its `bytea` columns (`SHAPE_SQL`'s joins, grouped by table), and `remember`s each.
- [ ] MySQL: the listing's message also asks `information_schema.statistics` (`index_name = 'PRIMARY'`, by `seq_in_index`) and `information_schema.columns` (binary types and FLOAT) for the schema; `remember` each table's `Shape`.
- [ ] A stale shape: a test drops and re-adds the key between listing and open, and the page is still the new key's (one trip more).
- [ ] Measure the listing on a schema of 1,000 tables, before and after. More than 50 ms slower on any server: stop and ask.

### Task 8: check, measure and ship run 2

- [ ] The same checks as Task 6.
- [ ] `slow_bench` opens right after `list_objects`, with no `describe` first: expected 0.20 s on all three.
- [ ] PR against main.
