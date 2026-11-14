# MySQL round trips, step 3: the SQL editor run's open and close as one message each — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use rubypowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A read-only MySQL SQL editor run of one `SELECT` asks the server 6 times, not 9 (about 600 ms less at 200 ms a round trip), with every guard it has today.

**Architecture:** Step 3 of `2026-11-11-mysql-round-trips.md`, as decided there (Igor, 2026-11-11): keep both guards, folded into the messages next to them. The open is one multi-statement text message ending in the read-only question; the close's check and its `ROLLBACK` are one message. The run reads each message's answers one statement at a time, so it knows which steps ran, and runs the steps a message did not reach on their own, as today. The first statement goes without the check before it: the open has just answered it (as igor-alexandrov/tabletist#303 did for PostgreSQL).

**Tech Stack:** Rust, mysql_async 0.37.1, the test MySQL on 53306.

---

## Where the trips go

Today (main 5750cf1f), a one-`SELECT` run on a read-only session: `START TRANSACTION READ ONLY`, `SET sql_select_limit`, the check, the prepare, the execute, the check, `ROLLBACK`, `COM_RESET_CONNECTION`, the settings. 9.

After: the open (one message: the fence on a writable session, the start, the row limit, an agent's time limit, the question), the prepare, the execute, the close (one message: `DO 0`, the question, the rollback), the reset, the settings. 6, on a writable session too.

### Why not 3 or 4

The reset is `COM_RESET_CONNECTION`, a command of its own, and the settings must follow it before anything else uses the session; the prepare is `COM_STMT_PREPARE`, which the execute needs the answer of. mysql_async 0.37 sends a command and reads its answer before it sends the next: it cannot pipeline. Two ways further, neither in this step:

- **Pipelining** (reset + settings, and prepare + execute, each written before reading): needs a driver that writes several commands before reading. 4 trips.
- **Answer before the close**: the run hands the outcome back before its close and keeps the session's lock until the close is done; a close that fails turns the shown outcome into the error. The user waits 3 trips. Changes how the backend reports a run.

Not done, and not to be done: skipping the reset by what the statements were (statement names never decide safety), or leaving the settings for the next use of the session (a reset session with the server's defaults would be read-write between runs).

## Reading a message one answer at a time

- `query_iter` returns once the first statement answered; its error is the first statement's.
- The driver keeps a later statement's error as the pending result of that statement: `QueryResult::collect` on that statement's turn returns it. After an error the server sends nothing more.
- `collect` reads the statement's rows and then the next statement's first packet. A statement without rows answers with an OK packet whose status (in a transaction, read-only) the driver keeps (`Conn::last_ok_packet`); a result set's header does not change it. So the status after a `SELECT` can be read only when it is the message's last statement, and the status at the close's check is read from a `DO 0` before it.
- `QueryResult::new(&mut conn)` picks up where a dropped `QueryResult` left off: the pending result lives on the connection.

## Task 1: count the trips (red)

- [ ] `a_select_asks_the_server_six_times` in `mysql::script::tests`: on a read-only and on a writable session (`counted_session`), the second run of `SELECT 1` asks 6 times, of `SELECT 1; SELECT 2` 9 (each later statement adds the check, the prepare, the execute); no transaction is left open and the settings are as at connect. On main: 9 and 12 (10 and 13 writable).

## Task 2: the open as one message

- [ ] `opening(access, limit, sent, names)` builds the message: `READ_ONLY` on a writable session, `START TRANSACTION READ ONLY`, `SET SESSION sql_select_limit = N`, an agent's time limit (MySQL's or MariaDB's form, by the server), `SELECT @@session.<read-only name> LIMIT 1` last.
- [ ] `open` sends it and reads each answer. The last statement's status and value must say `Inside` (`started`, then `stands`); anything else fails the open (it closes the session, as a failed start does today).
- [ ] An unknown setting (1193) on the time limit or the question: the rest runs step by step (`limit_time`, `standing`), which tries the other name.
- [ ] A cancel on any step: `Err(Cancelled)`, as today (`Ended::Unopened`).
- [ ] The read-only setting's names in the server's order (`tx_read_only` first on MariaDB before 11.1), passed down from `Conn`.
- [ ] `statements` skips the check before the first statement: the open has just answered it. An agent's check against what its connection hides runs in between (`EXPLAIN`, `SHOW WARNINGS`): those end no transaction.

## Task 3: the close's check and rollback as one message

- [ ] For `Ended::Unconfirmed` the close sends `DO 0; SELECT @@session.<name> LIMIT 1; ROLLBACK AND NO CHAIN NO RELEASE`. The status after `DO 0` and the question's value give `stands`.
- [ ] The message failed before the question answered (a cancel, an unknown setting, any error): the steps run on their own as today, `retry_cancelled!(standing)` and then the rollback.
- [ ] The rollback answered with a cancel: it runs once more. Any other error: the cleanup failed, as today.
- [ ] The rollback is `write::ROLLBACK` (`AND NO CHAIN NO RELEASE`): a script's `completion_type` cannot chain a transaction or close the session.
- [ ] Reset and settings stay as they are.

## Task 4: failure and cancel on the grouped steps

A test proxy (`proxied_session` in `mysql::tests`) gains a swap: a query whose text is exactly one string goes to the server as another. Tests in `mysql::script::tests`:

- [ ] `the_open_fails_on_any_of_its_steps`: the open with its start swapped for one the server refuses (`START TRANSACTION READ ONLY, WITH CONSISTENT SNAPSHOT` inside an XA transaction is not needed: swap the row limit for `SET SESSION sql_select_limit = 'x'`), and with the question swapped for one that answers 0: the run fails as a lost connection, no statement ran, the session is as it connected (read-only).
- [ ] `a_cancel_on_the_open_runs_no_statement`: the open's question swapped for `SELECT SLEEP(10)`, cancelled: the run is stopped with no result, the session as it connected.
- [ ] `a_cancel_on_the_close_runs_the_cancelled_step_again`: the close's question swapped for a sleep, cancelled: the steps run on their own, the run ends cleanly. The close's rollback swapped for a sleep, cancelled: the rollback runs once more.
- [ ] `the_close_finds_a_read_write_transaction_in_one_message`: a run whose last statement leaves a read-write transaction (`START TRANSACTION READ WRITE`, past the refusal) fails with `LeftReadOnly`, rolled back, the session read-only.
- [ ] An unknown setting on the question (swapped for `@@session.tabletist_nope`): the step-by-step path answers.

## Task 5: check and ship

- [ ] `cargo fmt --all -- --check`, `cargo clippy --locked --workspace --all-targets -- -D warnings`, `cargo test --locked -p tabletist-db` against MySQL and PostgreSQL.
- [ ] Measure through a 100 ms-each-way proxy: before/after for the PR description.
- [ ] PR against main.
