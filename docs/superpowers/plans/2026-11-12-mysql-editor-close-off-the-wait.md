# MySQL round trips, step 5: the SQL editor run's reset off the wait — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use rubypowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A read-only MySQL SQL editor run of one `SELECT` hands back its outcome after 4 round trips, not 6 (about 400 ms less at 200 ms a round trip). The session reset and the settings after it still run before anything else uses the session, and a failure there still closes the session.

**Architecture:** Once the close has confirmed where the session stands and rolled back, the outcome is known: the reset and the settings that follow it change nothing of it. So the run hands its outcome back there, and a task it spawns holds the session's lock (`lock_owned`) through `COM_RESET_CONNECTION` and the settings. Whatever uses the session next waits for that lock. A reset or settings that fail mark the session closed under the lock (`Session::closed`), and every later use of the session answers `ConnectionLost` with that error, which closes it, as the run's own error does today.

**Tech Stack:** Rust, mysql_async 0.37.1, tokio, the test MySQL 8.4 on 53306, MariaDB 11.4 on 53307.

---

## Where the trips go

Today (main 11f6a4a7), a one-`SELECT` run on a read-only session: the open, the prepare, the execute, the close (check and rollback), `COM_RESET_CONNECTION`, the settings. 6, all waited for; 1,211 ms through 100 ms each way.

After: the open, the prepare, the execute, the close; the outcome is back. The reset and the settings follow, before the session's next use. 4 waited for, 6 in all.

### Not done, and why

- **The settings in the next run's open** (one message less in all): between runs the session would be reset and not set, read-write on a read-only connection, for whatever uses it next (table browsing, an export, an agent). Every use would have to know to set it first. Off the wait, it saves nothing a user waits for.
- **The statement through the text protocol** (SQL `PREPARE … FROM @text; EXECUTE`, still held to one statement by the server, in the close's message: 2 waited for). Values come back as text: a `FLOAT` 1234567 comes back as `1234570`, the server's six digits, where the prepared protocol sends the float itself. And a `CALL` answers with result sets and an OK of its own, which the close's answers after it in the same message cannot be told from. The prepare and the execute stay.
- **The open in the prepare's or the execute's message**: those are commands of their own, and mysql_async cannot write a second command before it reads the first one's answer. The open's answer must be read before the statement runs.
- **The outcome before the close**: the close's check is part of the outcome (a statement in last place may have left read-only, and the run then fails). Shown before it, a result could turn into an error after the fact.

## Task 1: count the trips (red)

- [x] `a_select_waits_for_four_answers` in `mysql::script::tests` (single-threaded runtime, so the spawned task does not run before the count is read): through `counted_session`, the second run of `SELECT 1` has 4 trips behind it when it returns, and 6 once the session's lock is free; `SELECT 1; SELECT 2`, 7 and 9. Read-only and writable. On main: 6 and 6, 9 and 9.

## Task 2: the session's lock and its closed mark

- [x] `Session { conn, closed: Option<Error> }` behind `Arc<tokio::sync::Mutex<…>>` in `Conn`. `Session::conn()` is the connection, or `Err(ConnectionLost)` once closed. Every use goes through it.

## Task 3: the reset and the settings in a task of their own

- [x] `close` splits: `end` (the check and the rollback, as now) and `put_back` (the reset and the settings, as now, retrying a cancel).
- [x] A run that ended cleanly (`Unconfirmed` or `Unopened`, rolled back) hands `put_back` to a task with the owned lock and returns its outcome. A failure there marks the session closed with `cleanup_failed`'s error.
- [x] Every other end (`Left`, `Broken`, a failed rollback) runs `put_back` before it returns, as today: the run's error closes the session.
- [x] The agent read ends the same way.

## Task 4: failure and cancel on the moved steps

A test proxy's swap (`swapping_session`) now changes the first query of the text only: the same text sent again goes through, as a step run once more after a cancel does.

- [x] `settings_that_fail_after_the_outcome_close_the_session`: the settings after the reset swapped for one the server refuses. The run's outcome is the `SELECT`'s rows; then a run, a schema list and an agent's read are each `ConnectionLost`, and none of them reaches the server.
- [x] `a_cancel_on_the_settings_after_the_outcome_runs_them_again`: the settings swapped for a sleep, which the run's outcome does not wait for; cancelled, they run once more (7 trips in all), and the session is as it connected.
- [x] Tests that count or look at the session after a run (`a_session_is_prepared_and_named_in_one_message`, `a_cancel_on_the_close_runs_the_cancelled_step_again`, `a_script_keeps_no_lock`) wait for the session's next use first. A named lock a script took is held as long as before: the reset that drops it comes at the same moment, the outcome before it.

## Task 5: check and ship

- [x] `cargo fmt --all -- --check`, `cargo clippy --locked --workspace --all-targets -- -D warnings`, `cargo test --locked -p tabletist-db` against MySQL and PostgreSQL; the `mysql::` unit tests against MariaDB 11.4 (the six that fail there fail on main too: they assume MySQL).
- [x] Measure through a 100 ms-each-way proxy: before/after.
- [x] PR against main.
