# MySQL round trips, step 4: opening a table — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use rubypowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Opening a MySQL or MariaDB table (`describe`, then the first page) asks the server far fewer times: the structure in one message, not eight (seven on MariaDB), and a page in three, not four (one more each when a statement is not prepared yet). Every guard and every result stays as it is.

**Architecture:** As PostgreSQL did in #304 and #309, where MySQL allows it. mysql_async cannot pipeline, but one text message of several statements (`CLIENT_MULTI_STATEMENTS`, on by default) answers each in turn, read one statement at a time by `script::answers` (#315). The catalog questions keep their SQL with `?`, and the schema and table go in as `_utf8mb4 X'…'` literals: hex, so no quote, backslash or `sql_mode` can change what the text says, and the same character set and coercibility as the bound parameter they replace. The page's SELECT stays a prepared statement: it carries the raw WHERE, and only a prepared statement is held to one statement.

**Tech Stack:** Rust, mysql_async 0.37.1, the test MySQL 8.4 on 53306, MariaDB 11.4 on 53307.

---

## Where the trips go

Measured on main 2c30b343 through 100 ms each way: MySQL open 2,424 ms (12 trips: structure 8, page 4), MariaDB 2,225 ms (structure 7). A next page is 4, or 5 when its text is new.

- **Structure today:** columns, MariaDB's checks (MySQL refuses), MySQL's checks, indexes, keys out, keys in, the comment, the primary key. 8 on MySQL, 7 on MariaDB, each a prepared statement (once more the first time, for the prepare).
- **Structure after:** one message: columns, indexes, keys out, keys in, comment, primary key, then the checks the server's kind answers (`self.version` says MariaDB). Last, since a server without the list refuses it, and that loses nothing before it. A refusal then asks the other kind's question alone, then takes none, as today. 1 trip.
- **Page today:** the primary key, (the binary columns with a bytes filter), `START TRANSACTION READ ONLY`, the prepare (first time), the execute, the rollback. 4, or 5.
- **Page after:** one message starts the transaction and asks the key (and the binary columns) inside it; the prepare (first time), the execute, the rollback. 3, or 4. `LIMIT ? OFFSET ?` are bound, so a next page's statement is the one already prepared: 3.

### Not done, and why

- The rollback after the execute: a separate command. Left for the next command, the idle session would hold the read's metadata locks (an `ALTER TABLE` elsewhere waits) and the snapshot.
- The SELECT through the text protocol or SQL `PREPARE … EXECUTE` in the start's message (2 trips): rows would come as text and need retyping from the column metadata, and values would go in as user variables. A change of result shape for one trip; a step of its own if wanted.

## Task 1: count the trips (red)

- [x] `opening_a_table_asks_the_server_four_times` in `mysql::tests`: through `counted_session`, a fresh session's `describe` then `fetch_rows`; then the same again (statements prepared); then the next page. On main: 15+5 fresh, 8+4 again, next page 4. After: 1+4, 1+3, 3.

## Task 2: the structure as one message

- [x] The catalog questions as constants with `?` for schema and table; `inlined(sql, values)` puts in each value as `_utf8mb4 X'…'`.
- [x] `describe` sends them as one message through `script::answers` (made `pub(super)`), reads each answer as before, and builds the same `Structure`.
- [x] Checks: the server's kind first and last in the message; on a query error the other kind alone, then none.
- [x] A test-only `describe_one_by_one`: today's prepared questions, the same parsing. `the_structure_is_the_same_in_one_message`: a table with checks, indexes, keys both ways, a comment, a composite key, and names with quotes, backticks, a backslash and non-ASCII letters, compared on MySQL, and on MariaDB by pointing `TABLETIST_TEST_MYSQL_URL` at it.
- [x] `a_refused_question_in_the_structure_s_message`: the proxy swaps the message for one whose checks question, then whose columns question, names a missing view: the first still describes (after the other way's question), the second fails.

## Task 3: the page's start and its questions as one message

- [x] `begin_read_asking(conn, questions)`: (`ROLLBACK` when a transaction is open), `START TRANSACTION READ ONLY`, the questions; their rows, or the error. A failure still ends with `end_read`.
- [x] `fetch_rows` and `count_rows` ask the key and the binary columns that way.
- [x] MySQL's page binds `LIMIT ? OFFSET ?`; the dialect tests that pin MySQL's parameters take the two after the filters'.

## Task 4: check and ship

- [x] `cargo fmt --all -- --check`, clippy `-D warnings`, `cargo test --locked -p tabletist-db` against MySQL and PostgreSQL; the MySQL suites against MariaDB where they ran before.
- [x] Measure through a 100 ms-each-way proxy, before and after, MySQL and MariaDB.
- [ ] PR against main.
