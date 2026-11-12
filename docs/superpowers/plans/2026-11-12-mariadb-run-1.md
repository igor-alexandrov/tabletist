# MariaDB, run 1 of 3: the MySQL tests pass on MariaDB 11.4 — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use rubypowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Every MySQL suite that CI runs also passes against MariaDB 11.4, in a CI job of its own, and the driver behaves the same on both where it did not: a failed CHECK's name, a column's default, the order of lists, and connecting in as many messages.

**Architecture:** MariaDB keeps going through the MySQL driver (`Driver::MySql`, `mariadb://` already parses to it). Where MariaDB answers differently, the driver turns its answer into MySQL's form at the edge (`named`, `mariadb_default`, the list order, `server_identity`), so nothing above `tabletist-db` changes. A test that expects what only one server does asks the server's version (`server_version()`, or `Conn::version`) and expects that server's answer; nothing is skipped on MariaDB but the one thing it cannot show.

**Tech Stack:** Rust, mysql_async 0.37.1; the test MySQL 8.4 on 53306 and MariaDB 11.4 on 53307 (`compose.yaml`'s new `mariadb` service).

---

## The three runs

1. **This run.** MariaDB 11.4 in `compose.yaml` and CI; the driver fixes and server-aware tests below. JSON columns show as text, and agents may not read a view on MariaDB while anything is hidden or masked (decided on 2026-11-12: keep the fail-safe).
2. **MariaDB 10.11.** Add `"10.11"` to the CI matrix. Before 11.1 MariaDB knows `tx_read_only` and `tx_isolation`, not `transaction_read_only` and `transaction_isolation`, and its default collation is `utf8mb4_general_ci`, not `utf8mb4_uca1400_ai_ci`. About 25 test spots read the new names (`grep -rn "transaction_read_only\|transaction_isolation" crates/tabletist-db`), and `tests/ddl/mariadb_book_covers.sql` names the collation. Needs a 10.11 server to run against (`MARIADB_VERSION=10.11 docker compose up -d --wait mariadb`): not available in the session that wrote this plan.
3. **JSON on table pages** (decided on 2026-11-12). A MariaDB column of `LONGTEXT` with the check MariaDB adds for `JSON` (`json_valid(`col`)`) is a JSON column on table pages and in the structure; SQL editor results stay text (mysql_async does not read MariaDB's extended column metadata). The check comes from the structure's message, which already reads `MARIADB_CHECKS`.

## What failed, and why

The MySQL suites against MariaDB 11.4.13 on main 11f6a4a7, with `compose/mysql-init.sql` applied: 21 failures.

| Failure | Cause | Where it is fixed |
| --- | --- | --- |
| `a_mysql_failure_keeps_its_error_number`, the CHECK row of `the_bookshops_failures_as_mysql_hands_them_over` | MariaDB's failed CHECK is 4025, ``CONSTRAINT `name` failed for `db`.`table` ``; `named` knew only 3819 | Task 1 |
| `a_session_is_prepared_and_named_in_one_message` (3 messages, not 2) | `@@server_uuid` is asked first and fails on MariaDB, then host and port | Task 2 |
| `objects_have_kinds_and_estimates` | `ORDER BY table_name` follows the catalog's collation, which on MariaDB ignores case: `audit_books` before `audit_book_covers` | Task 3 |
| `users_structure_has_full_types_comments_key_and_indexes` (`created_at`'s default) | MariaDB's `COLUMN_DEFAULT` is SQL: `'2026-01-01 00:00:00.000000'` quoted, and `NULL` for no default, which made every nullable column look as if it had one | Task 4 |
| `cancels_reuse_one_spare_and_replace_a_closed_one`, `a_spare_that_does_not_answer_is_replaced_in_time` | a killed `SLEEP` fails with 1317 on MariaDB, and answers 1 on MySQL | Task 5 |
| `a_cancel_that_reaches_another_server_kills_nothing` | a MariaDB session is named `host …`, not `uuid …` | Task 5 |
| `a_setting_the_server_does_not_know_is_asked_for_under_its_other_name`, `what_an_agents_statement_leaves_on_the_session_is_gone_after_the_run`, `the_time_limit_ends_with_its_run_on_mysql` | the tests read `max_execution_time`, which MariaDB does not have | Task 5 |
| the four agent view tests | MariaDB has no `VIEW_TABLE_USAGE`, so every view is refused as hidden itself (the documented fail-safe) | Task 5 |
| `book_covers_is_what_the_server_shows…`, `in_stock_is_what_the_server_shows` | MariaDB's `SHOW CREATE` text | Task 5 |
| the duplicate key's words in `the_bookshops_failures_as_mysql_hands_them_over` | MariaDB names the key without its table | Task 5 |
| `pages_have_typed_values_and_kinds`, `a_script_has_typed_columns_and_truncates_at_the_limit` | JSON is `LONGTEXT` on MariaDB (run 3) | Task 5 |
| `users_structure…` (`int(11)`), `only_an_index_over_whole_columns_is_the_row_key`, `what_the_prepared_protocol_refuses_is_the_statements_error` | display widths; no index over an expression; MariaDB prepares `GET DIAGNOSTICS` (and every statement the editor lets through) | Task 5 |

`tabletist-core`'s `backend::agents` passes on MariaDB as it is (32 tests).

## The draft

The whole run is drafted and checked: `/tmp/claude-1000/-home-igor-Work-tabletist--claude-worktrees-tabletist-mariadb-support-3ad6d9/92ea89b7-6333-4c23-a212-d4da061af4c1/scratchpad/mariadb-run-1.patch` (with the two new files under `tests/ddl/`), on main 11f6a4a7. On it, every suite below passes against MySQL 8.4 and MariaDB 11.4, and fmt, clippy, the workspace tests and `cargo doc` are clean. Each task takes its part of the draft; the red step runs the task's tests on the tree before its code.

A reviewer then tried to break the draft against both servers. Its findings are in the tasks: a translated 4025 message and a column's own CHECK (Task 1), the host question's fallback (Task 2), binary defaults (Task 4), a loose version check (Task 2), and three doc gaps (Task 6). What held: about 60 column defaults read alike on both servers, byte order unchanged on MySQL, a session and its spare named alike, and no MySQL expectation weakened.

Commands, from the worktree (cargo is `~/.cargo/bin/cargo`):

```bash
MY=mysql://tabletist:tabletist@localhost:53306/tabletist
MARIA=mysql://tabletist:tabletist@localhost:53307/tabletist
SUITES="--lib --test mysql --test insert_audit --test lookup --test delete_rows --test agent --test ddl --test export"
```

## Task 1: a failed CHECK on MariaDB names its constraint

**Files:** `crates/tabletist-db/src/mysql.rs` (`named`, its test `a_failure_is_named_from_its_number_and_what_the_message_quotes`), `crates/tabletist-db/tests/insert_audit.rs`.

MariaDB translates this message (with `lc_messages = es_ES` it is ``No se cumple la RESTRICCIÓN `name` para `db`.`table` ``), so the name is read by its backticks, not its words: the first quoted name is the constraint, the last the table. A column's own CHECK is quoted as `` `table.column` `` and is `column` in `CHECK_CONSTRAINTS`; a doubled backtick is one.

- [ ] **Step 1: tests.** In `a_failure_is_named_from_its_number_and_what_the_message_quotes`, after the 3819 case:

```rust
// MariaDB's, in any language it is told to speak, and a column's
// own CHECK by the column's name, as the catalog has it.
for message in [
    "CONSTRAINT `audit_book_covers_kind_check` failed for \
     `tabletist`.`audit_book_covers`",
    "No se cumple la RESTRICCIÓN `audit_book_covers_kind_check` para \
     `tabletist`.`audit_book_covers`",
] {
    assert_eq!(
        of(4025, message),
        (
            Some(4025),
            Some("audit_book_covers_kind_check".into()),
            vec![]
        )
    );
}
assert_eq!(
    of(4025, "CONSTRAINT `covers.kind` failed for `tabletist`.`covers`"),
    (Some(4025), Some("kind".into()), vec![])
);
assert_eq!(
    of(4025, "CONSTRAINT `we``ird` failed for `tabletist`.`t`"),
    (Some(4025), Some("we`ird".into()), vec![])
);
```

In `tests/insert_audit.rs` (module `mysql`): an `on_mariadb(&Connection)` helper (`server_version()` starts with `"MariaDB "`); `the_bookshops_failures_as_mysql_hands_them_over` expects the key `isbn` (MariaDB) or `audit_books.isbn`, the CHECK's words ``CONSTRAINT `audit_book_covers_kind_check` failed for `tabletist`.`audit_book_covers` `` or MySQL's, and `numbered(4025 or 3819, …)`; `a_mysql_failure_keeps_its_error_number` expects `"4025"` or `"3819"` for the vinyl row.

- [ ] **Step 2: red.** `cargo test -p tabletist-db --lib mysql::tests` fails on the 4025 case (`None` for the constraint); `insert_audit` against `$MARIA` fails on the CHECK's name.
- [ ] **Step 3: code.** A `backticked(message) -> Vec<String>` beside `named` (each backtick-quoted name in order, a doubled backtick read as one), and in `named`, after 3819:

```rust
// MariaDB's ER_CONSTRAINT_FAILED, which it translates: "CONSTRAINT
// `name` failed for `db`.`table`". The constraint is what it quotes
// first, the table what it quotes last; a column's own CHECK is
// named `table.column` there, and `column` in the catalog.
4025 => {
    let names = backticked(message);
    let constraint = names.first().map(|name| match names.last() {
        Some(table) if names.len() > 1 => name
            .strip_prefix(table.as_str())
            .and_then(|rest| rest.strip_prefix('.'))
            .unwrap_or(name)
            .to_owned(),
        _ => name.clone(),
    });
    (constraint, Vec::new())
}
```

- [ ] **Step 4: green.** The unit test; `insert_audit` against `$MY` and `$MARIA`.
- [ ] **Step 5: commit** "Name the CHECK a MariaDB save failed".

## Task 2: a MariaDB session is named in one question

**Files:** `crates/tabletist-db/src/mysql.rs` (`server_identity`, new `server_host`; tests `named_by`, `a_session_is_prepared_and_named_in_one_message`, `a_cancel_that_reaches_another_server_kills_nothing`).

- [ ] **Step 1: tests.** In `mysql::tests`, `fn named_by(session: &Conn) -> &'static str`: `"host "` where `session.version` says MariaDB, else `"uuid "`. `a_session_is_prepared_and_named_in_one_message` and `a_cancel_that_reaches_another_server_kills_nothing` assert `starts_with(named_by(..))`; the first expects a version name of `MariaDB 1` where the session is named by host, else `MySQL 8.`. The count stays 2.
- [ ] **Step 2: red.** Against `$MARIA`: `a_session_is_prepared_and_named_in_one_message` counts 3 messages for `ReadOnly`.
- [ ] **Step 3: code.** `server_identity` first asks MariaDB's question of a server whose greeting says 10 or later, and keeps its answer where `VERSION()` says MariaDB; otherwise as before. The host question moves to `server_host(conn, before)`, which both paths call:

```rust
if conn.server_version() >= (10, 0, 0)
    && let Ok((server, version)) = server_host(conn, before).await
    && version.contains("MariaDB")
{
    return Ok((server, version));
}
```

A failed host question there falls through to the uuid question, as a non-MariaDB `VERSION()` does. MariaDB is told apart by `VERSION()` saying so, as `SessionKind` already does: a server started with another `--version` text is taken for MySQL.

mysql_async reads a MariaDB greeting's `5.5.5-` prefix away (`maria_db_server_version_parsed`), so 10.x and 11.x land here. `a_server_without_a_uuid_is_named_by_its_host_and_port` (the proxy that answers 1193 on MySQL) still covers the old path.

- [ ] **Step 4: green.** `--lib mysql::tests` against `$MY` and `$MARIA`.
- [ ] **Step 5: commit** "Name a MariaDB session in one question".

## Task 3: schemas and objects in byte order

**Files:** `crates/tabletist-db/src/mysql.rs` (`list_schemas`, `list_objects`).

- [ ] **Step 1: red.** `objects_have_kinds_and_estimates` (unchanged) against `$MARIA`: `audit_books` before `audit_book_covers`.
- [ ] **Step 2: code.** `ORDER BY CAST(schema_name AS BINARY)` and `ORDER BY CAST(table_name AS BINARY)`, with a doc line on `list_schemas`: the catalog's collation differs by server (MariaDB's ignores case), and the list is cut at `MAX_LISTED` after the order. `CAST`, not `BINARY x`, which MySQL 8.0.27 deprecates.
- [ ] **Step 3: green.** `--test mysql` against both.
- [ ] **Step 4: commit** "List MySQL and MariaDB schemas and tables in byte order".

## Task 4: MariaDB's column defaults read as MySQL's

**Files:** `crates/tabletist-db/src/mysql.rs` (`Described::structure` takes `mariadb: bool`, its two callers pass `self.version.contains("MariaDB")`, new `mariadb_default`, unit test `a_mariadb_default_is_read_as_mysql_writes_it`), `crates/tabletist-db/tests/mysql.rs` (`users_structure…`: `assert_eq!(column("name").default, None)`).

What the two servers answer for the same table (probed on 2026-11-12):

| Column | MySQL 8.4 | MariaDB 11.4 |
| --- | --- | --- |
| `DEFAULT 'it''s \\ x'` | `it's \ x` | `'it''s \\ x'` |
| `DEFAULT 'NULL'` | `NULL` | `'NULL'` |
| `DEFAULT NULL`, or none and nullable | none | `NULL` |
| `NOT NULL`, no default | none | none |
| `DEFAULT CURRENT_TIMESTAMP` | `CURRENT_TIMESTAMP` | `current_timestamp()` |
| `DEFAULT (1 + 2)` | `(1 + 2)` | `(1 + 2)` |
| `DEFAULT ''` | (empty) | `''` |
| `binary(2) DEFAULT 'ab'` | `0x6162` | `'ab'` |
| `varbinary(4) DEFAULT 0x41FF` | `0x41FF` | `'A?'` (MariaDB's own loss) |

`tabletist-core` reads a MySQL default as MySQL writes it (`edit::unset`, `default_shown`, `default_is_expression`, `set_default`). So `NULL` becomes none, a quoted literal its value (a doubled quote one, `\\`, `\n`, `\r`, `\t`, `\0`, `\Z` unescaped), a binary column's (`ColumnClass::Binary`) in hex as MySQL gives it, and the rest stays. The review checked about 60 column definitions on both servers; only binary columns differed. A save writes `DEFAULT`, never the text, so the loss MariaDB makes is the structure's only.

- [ ] **Step 1: tests.** The unit test:

```rust
#[test]
fn a_mariadb_default_is_read_as_mysql_writes_it() {
    let read = |written: &str| mariadb_default(written.to_owned(), false);
    assert_eq!(read("NULL"), None);
    assert_eq!(read("'NULL'").as_deref(), Some("NULL"));
    assert_eq!(read("''").as_deref(), Some(""));
    assert_eq!(read("'print'").as_deref(), Some("print"));
    assert_eq!(read(r"'it''s \\ x'").as_deref(), Some(r"it's \ x"));
    assert_eq!(read(r"'a\nb'").as_deref(), Some("a\nb"));
    assert_eq!(read("'a(b'").as_deref(), Some("a(b"));
    for kept in ["5", "1.50", "current_timestamp()", "(1 + 2)"] {
        assert_eq!(read(kept).as_deref(), Some(kept));
    }
    // A binary column's, in hex as MySQL gives it.
    let binary = |written: &str| mariadb_default(written.to_owned(), true);
    assert_eq!(binary("'ab'").as_deref(), Some("0x6162"));
    assert_eq!(binary(r"'a''b\0'").as_deref(), Some("0x61276200"));
    assert_eq!(binary("NULL"), None);
    assert_eq!(binary("0x41").as_deref(), Some("0x41"));
}
```

and in `users_structure…`, `column("name").default` is `None`.

- [ ] **Step 2: red.** The unit test does not compile without `mariadb_default`; against `$MARIA`, `users_structure…` fails on `created_at`'s default.
- [ ] **Step 3: code.** `mariadb_default(written: String, binary: bool) -> Option<String>` as in the draft, applied in `structure` where `mariadb`, with `binary` from the column's class.
- [ ] **Step 4: green.** `--lib` and `--test mysql` against both (`users_structure…` still fails on MariaDB's `int(11)` until Task 5).
- [ ] **Step 5: commit** "Read a MariaDB column's default as MySQL writes it".

## Task 5: tests that expect what only MySQL does ask the server

Test changes only, from the draft:

- `src/mysql.rs` tests: `interrupted(slept)` (MySQL's `SLEEP` answers 1, MariaDB's fails with `QUERY_INTERRUPTED`) in the two spare tests; the `SLEEP` comments.
- `src/mysql/script.rs` tests: `time_limit_ms(mariadb)` (`@@session.max_execution_time`, or `CAST(@@session.max_statement_time * 1000 AS SIGNED)`); the other-name test takes the server for the other kind (`mariadb: !mariadb`), and its doc says both ways; the agent-statement test uses `session.kind()`.
- `tests/agent.rs`: `views_say_what_they_read` and `unsaid_view(name, by)`; the view rows of the four shared helpers expect `unsaid_view` where the server is MariaDB (`recent_orders` and `customer_names` by the person, `gift_shouts`, and `customer_names` by the administrator in the rule test); the time-limit test reads MariaDB's limit in milliseconds.
- `tests/ddl.rs`: `expected_of(connection, name)` picks `tests/ddl/mariadb_<name>.sql` (new, from MariaDB 11.4's text) or `mysql_<name>.sql`. `.gitattributes` already holds `tests/ddl/*.sql` to LF.
- `tests/mysql.rs`: `on_mariadb`, `json_of` (`("text", Text)` on MariaDB until run 3); `int(11)` and `bigint(20) unsigned`; the expression index only on MySQL; the prepared-protocol half of `what_the_prepared_protocol_refuses…` only on MySQL, with a note that MariaDB prepares everything the editor lets through (checked: `GET DIAGNOSTICS`, `HELP`, `HANDLER`, `XA RECOVER`, `CHECK TABLE`, `SHOW WARNINGS`; `PREPARE`, which both refuse, the editor refuses first).

- [ ] **Step 1: red.** On the tree after Task 4, `$SUITES` against `$MARIA`: the failures above, nothing else.
- [ ] **Step 2: the test changes**, and the two `mariadb_*.sql` files.
- [ ] **Step 3: green.** `$SUITES` against `$MY` and `$MARIA`; `cargo test -p tabletist-core --lib backend::agents` against both.
- [ ] **Step 4: commit** "Hold the MySQL tests to what MariaDB answers".

## Task 6: MariaDB in compose, CI and the docs

**Files:** `compose.yaml`, `.github/workflows/ci.yml`, `AGENTS.md`, `README.md`, `docs/_reference/connections.md`, `docs/_guide/agents.md`, the module doc of `crates/tabletist-db/src/mysql.rs`.

- [ ] **Step 1.** `compose.yaml`: a `mariadb` service, `mariadb:${MARIADB_VERSION:-11.4}`, on `127.0.0.1:53307`, with the `MARIADB_*` user and database, `compose/mysql-init.sql`, and `healthcheck.sh --connect --innodb_initialized` (TCP, which the init server does not listen on). Not started in the session that wrote this plan (no Docker there): the CI job is its first run.
- [ ] **Step 2.** `ci.yml`: a `mariadb` job, matrix `version: ["11.4"]` (run 2 adds 10.11), the same `--test` list and `backend::agents`, with `TABLETIST_TEST_MYSQL_URL` on 53307.
- [ ] **Step 3.** Docs: README's intro and Development line name MariaDB; connections.md's Type row points at a new `### MariaDB` section after the dialog's screenshot (opened as MySQL, `mariadb://`, JSON shows as text, display widths, agents and views); agents.md's lists of what is hidden and of what a masked column refuses each say every view on MariaDB; AGENTS.md says how to run the MySQL tests against MariaDB; the module doc of `mysql.rs` says the tests run on MariaDB 11.4 and what of an older MariaDB has not met one (the old read-only name; `can_reset`'s lowest version).
- [ ] **Step 4: check.** `cargo fmt --all --check`; `cargo clippy --locked --workspace --all-targets -- -D warnings`; `cargo test --locked --workspace --all-targets`; `RUSTDOCFLAGS='-D warnings' cargo doc --locked --workspace --no-deps`.
- [ ] **Step 5: commit** "Run the MySQL tests against MariaDB in CI", then the PR. CI's `mariadb 11.4 integration` job must show the suites ran: grep its log for `objects_have_kinds_and_estimates ... ok`.
