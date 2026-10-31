# ClickHouse, read-only

Tabletist opens a ClickHouse database as a fourth engine: browse, count,
filter, read the structure and the DDL, run reads in the SQL editor, and
let agents read it through the MCP server. Nothing writes in this spec.

It builds on the research and the HTTP spike of pull request 246:
`2026-10-30-clickhouse-connection-research.md` and
`crates/tabletist-db/src/clickhouse.rs`.

## Decisions taken with the user

Asked and answered on 2026-10-31.

| Question | Decision |
| --- | --- |
| What a ClickHouse connection does first | Read-only everywhere, and agents read it too |
| Which URLs are read | `clickhouse://` with `sslmode` and `sslrootcert`, and `http://` and `https://` as ClickHouse Cloud shows them |
| What TLS `prefer` means without an in-protocol upgrade | Try a TLS handshake once at connect (no certificate check, as libpq's `prefer`); if the server does not speak TLS, plain HTTP for the session |
| Transport | Our own: tokio, tokio-rustls with `tls::client_config`, HTTP/1.1 by ureq-proto. Not the spike's ureq, not the official `clickhouse` crate |
| The SQL editor's fence | The server's `readonly=2` behind an allowlist of our own, so `SET` and `SETTINGS` work |

## What the server does under `readonly`

Probed on 25.8 (compose) on 2026-10-31. It is why the fence has the
shape below. "Runs" in a write's row means the write happened; the
probe's files and backups were removed after.

| Statement | `readonly=1` | `readonly=2` |
| --- | --- | --- |
| `INSERT INTO FUNCTION file(…)`, `url(…)` | refused | runs (wrote a file) |
| `EXPLAIN PIPELINE INSERT INTO FUNCTION file(…) …` | refused | runs (created the file) |
| `EXPLAIN SYNTAX INSERT INTO FUNCTION file(…) …` | not probed | no file |
| `BACKUP DATABASE … TO File(…)` | runs | runs |
| `KILL QUERY …` | runs | runs |
| `CREATE TEMPORARY TABLE` | refused | runs (session only) |
| `SET x = …`, top-level `SETTINGS x = …` | refused | runs, and lifts the URL's limits |
| `SETTINGS` in a subquery | runs | runs; the URL's `max_execution_time` and `max_result_rows` still hold |
| `SET readonly = 0`, `SETTINGS readonly = 0` | refused | refused |
| `SELECT … FROM url(…)`, `file(…)`, `fileCluster(…)` | refused | runs |
| `numbers()`, `generateRandom()`, `system.*`, `sleep()` | runs | runs |
| `SELECT file('…')`, the scalar function (a file in `user_files`) | runs | runs |
| `OPTIMIZE`, `SYSTEM …`, `CREATE TABLE` | refused | refused |
| `SELECT … INTO OUTFILE` (over HTTP) | refused | refused |
| Settings in the URL (`max_result_rows`, `query_id`, `param_*`) | accepted | accepted |

So no level is a fence alone: `BACKUP` and `KILL` pass both, and under
`readonly=2` `INSERT INTO FUNCTION` runs, wrapped in `EXPLAIN PIPELINE`
too. Each kind of request gets the strictest level that serves it, and
the editor and agents get an allowlist of our own in front:

| Request | Level | Our check |
| --- | --- | --- |
| Browsing: catalog, rows, counts, DDL, `raw_where` | `readonly=1` | SQL built by `dialect.rs`; `raw_where` checked (below) |
| SQL editor | `readonly=2` | the allowlist |
| Agents | `readonly=1` | the allowlist without `SET` and `USE`, `check_read`, `effects` |

## Architecture

All of it is in `crates/tabletist-db/src/clickhouse/`, beside `pg/` and
`mysql/`, and `crates/tabletist-db/src/clickhouse.rs`:

- `clickhouse/http.rs`, the transport. A tokio `TcpStream` to
  `spec.host:spec.port`, or to `127.0.0.1:via` through an SSH tunnel.
  With TLS, tokio-rustls wraps it with `tls::client_config(mode,
  ca_file)`, and the server name is always `spec.host`, so a tunnel
  checks the real host's certificate. HTTP/1.1 is ureq-proto's sans-IO
  `Call`, which ureq uses inside. Connections are kept alive in a pool
  of two: one runs statements, the other sends `KILL QUERY` while one
  runs.
- New direct dependencies of `tabletist-db`, each with its line in
  `Cargo.toml`: tokio-rustls (0.26, in the tree through
  tokio-postgres-rustls) and ureq-proto (0.6, in the tree through ureq,
  pinned to ureq's version and moved with it). The lockfile gains no
  crate.
- `prefer` is decided once, at connect: a TLS handshake that checks no
  certificate. A server that answers it with anything but TLS gets plain
  HTTP for the rest of the session, and `is_encrypted()` says which.
- `clickhouse/read.rs`, the spike's reader as it is (`read_tsv`, `value`,
  `kind`, `server_error`), on an async buffered reader.
- `clickhouse.rs`, the `Conn` that is the `Adapter`. One statement at a
  time, as on PostgreSQL and MySQL (the spike's lock), each with its own
  `query_id`. `CancelInner::ClickHouse` holds the transport and the
  running `query_id`.
- Removed: the spike's `ureq` code, and `ureq` from `tabletist-db`'s
  dependencies.

Every request is a POST of one statement. The user, password and
database go in the `X-ClickHouse-User`, `X-ClickHouse-Key` and
`X-ClickHouse-Database` headers, never in the URL. Settings go in the
URL: `readonly` (per the table above), `query_id`, `session_id` for a
script, `param_*` for bound values, and for the editor and agents
`max_result_rows` with `result_overflow_mode=break` and
`max_execution_time`. Rows come back as `TabSeparatedWithNamesAndTypes`.

A user whose own profile is `readonly=1` already has `readonly=1` in
force, and is refused every setting. `connect` asks
`SELECT getSetting('readonly')`; for such a user the URL carries only
`query_id`, `param_*` and the format. Browsing is unchanged (its limits
are `LIMIT` in the SQL). The editor and agents get the row cap from the
reader (it stops at the cap and drops the connection) and the timeout
from the client (it cancels with `KILL QUERY`), and a script's `SET`
fails with the server's own error. The editor's reader stops at the cap
and drops the connection for every user anyway: under `readonly=2` a
top-level `SETTINGS` lifts `max_result_rows`.

## The engine in `tabletist-db`

### Identity and connecting

- `Driver::ClickHouse` (serde `clickhouse`, label "ClickHouse", default
  port 8123) and `Dialect::ClickHouse`. Every match on the two names it,
  as AGENTS.md asks.
- New: `Driver::tls_port()`, the port a TLS connection defaults to:
  8443 for ClickHouse, `default_port()` for PostgreSQL and MySQL, 0 for
  SQLite. The dialog moves the port between `default_port()` and
  `tls_port()` when the TLS mode changes, only while it holds the other
  one; where the two are equal nothing moves.
- New: `Driver::writes()`: false for ClickHouse, true for the others.
  `Connection::connect_with` opens a session of a driver that does not
  write `Access::ReadOnly` whatever was asked. The adapter's `write`
  answers `Error::ReadOnly`; a `ScriptMode::Write` run is refused before
  anything is sent, as on every read-only session.
- URLs: `clickhouse://user:pass@host[:port]/db` reads `sslmode`,
  `ssl-mode`, `ssl_mode`, `sslrootcert` and `ssl-ca` as the other
  engines' URLs do. `http://…` is `disable` on 8123; `https://…` is
  `verify-full` on 8443. The empty database is the user's default.
- SSH as for PostgreSQL and MySQL: `Connection::connect_with` opens the
  tunnel and passes its port as `via`.
- `server_version` is `SELECT version()`. `session_zone` is the
  `X-ClickHouse-Timezone` header of the first answer.

### Catalog

Databases are schemas, as on MySQL.

- `list_databases`: empty. `list_schemas`: `system.databases`. In the
  app, `is_system_schema` (`src/model.rs`) names `system`,
  `INFORMATION_SCHEMA` and `information_schema`.
- `list_objects`: `system.tables` for the schema. `engine = 'View'` is a
  view, `MaterializedView` a materialized view (that kind's doc stops
  saying PostgreSQL only), anything else a table (`Distributed`,
  `Dictionary`, `Kafka` among them). `total_rows` is the estimate.
- `describe`: `system.columns` for the columns: type, `Nullable(…)` for
  nullable, `default_kind` and `default_expression` (`MATERIALIZED` and
  `ALIAS` are `generated`; `DEFAULT` is the default), comment, and an
  `Enum8`/`Enum16`'s labels as `allowed_values`. No primary key: a
  ClickHouse `PRIMARY KEY` is a sorting key with duplicates. The sorting
  key (`system.tables.sorting_key`, an expression) and each row of
  `system.data_skipping_indices` are non-unique `IndexInfo`s (method
  `sorting key`, or the skipping index's type), with `key_columns`
  `None`. No foreign keys either way. `row_key` is therefore `None`, and
  the comment is `system.tables.comment`.
- `ddl`: `SHOW CREATE TABLE`, and `SHOW CREATE DICTIONARY` for an object
  whose engine is `Dictionary` (the plan checks which the server wants).
  `Ddl::source` is `Stored`: the server keeps the definition. One
  statement, so `laid_out` makes no sections.

### Rows

- `fetch_rows` and `count_rows` build their SQL in `dialect.rs` for
  `Dialect::ClickHouse`: identifiers in backticks (a backtick doubled),
  `ILIKE` for contains, `LIMIT n + 1 OFFSET m` in the SQL. `LIMIT`,
  not `max_result_rows`: a sort on a column outside the key is then a
  top-N partial sort on the server, not a sort of the whole table.
- Values are never spliced into SQL. A filter's value is a parameter
  `{p1:String}` sent as `param_p1`, cast to the column's type in the SQL
  where the operator needs it.
- With no sort chosen, rows are ordered by the sorting key's expression
  as `system.tables.sorting_key` gives it (the server's own text; none
  for `tuple()`), so the server reads in order
  and pages mostly keep their rows. `ordered_by_key` stays false: the
  key allows duplicates, so `OFFSET` over ties is not stable, and the
  grid keeps its unordered notice.
- `raw_where` goes on its own lines as on the other engines, and the
  request runs under `readonly=1`. Before it is sent, the ClickHouse
  lexer refuses in it a `;`, a `FORMAT`, `SETTINGS` or `INTO OUTFILE`
  word outside strings and comments, a parenthesis it closes that it did
  not open, and text ending inside a string or a comment. The sentence
  says a column of that name is written in backticks.
- `lookup` answers an empty page: there are no foreign keys to look up.

### Types

- `class.rs` gains a ClickHouse parser over the spike's `kind`, which
  unwraps `Nullable(…)` and `LowCardinality(…)` first: integers as
  `Integer { min, max }` (`UInt64` and wider as text over `i64`, as on
  MySQL), `Decimal(P, S)`, floats, `Bool`, `String` and `FixedString`
  as text (bytes where they are not UTF-8), `UUID`, `IPv4`, `IPv6`,
  enums, `JSON`, and `Array`, `Tuple`, `Map`, `Nested`, `Variant`,
  `Dynamic` as other.
- `temporal()` reads `Date`, `Date32`, `DateTime`, `DateTime64(p)`,
  `Time` and `Time64`; a type that names its zone (`DateTime('UTC')`,
  `DateTime64(3, 'Asia/Tokyo')`) is shown in it, else in the session's.
- A comparison is a `UInt8`, so `a = b` shows 0 or 1: the server's type,
  shown as it is.

### Errors

Any status but 200 is the server's error with
`X-ClickHouse-Exception-Code`; one written after the rows began is read
as the spike does (a line ending `\r\n`, tagged or not). 394 is
`Cancelled`, 159 `Timeout`, 516 and 194 `Auth`, the rest `Query` with
the code (164, `READONLY`, among them: the server's sentence says why).
A lost connection is `ConnectionLost`.

## The SQL editor

### Lexing (`sql.rs`, `Dialect::ClickHouse`)

As probed on 25.8:

- Comments: `--` (no space needed after it), `#` followed by a space or
  `!`, and `/* */`, which nest as on PostgreSQL. `/*! */` is a comment,
  not run as on MySQL.
- Strings: single quotes, with backslash escapes and a doubled quote.
  Heredocs `$$…$$` and `$tag$…$tag$`, as PostgreSQL's dollar quotes.
- Names: backticks (a backtick doubled) and double quotes. `$` may be
  part of a word.
- `{name:Type}` is one token, a placeholder: its `:` is not a named
  parameter's. The token is a name, a colon and a type (names,
  parentheses, commas, numbers and quoted strings) and nothing else; any
  other text in braces is not a placeholder.

### The allowlist (`sql::refusal`)

Refused before anything is sent, the sentence naming what it refused:

- a statement whose first keyword is not `SELECT`, `WITH`, `SHOW`,
  `DESC`, `DESCRIBE`, `EXISTS`, `EXPLAIN`, `SET` or `USE`;
- an `EXPLAIN` whose explained statement does not begin with `SELECT` or
  `WITH`, after `EXPLAIN`'s own kind (`AST`, `SYNTAX`, `QUERY TREE`,
  `PLAN`, `PIPELINE`, `ESTIMATE`, `TABLE OVERRIDE`) and its
  `name = value` options;
- a statement that names a `FORMAT` or `INTO OUTFILE` of its own.

`readonly=2` is sent with every editor statement behind it, as
`query_only` backs SQLite's authorizer. Tests cover every row of the
probe table, so a server that changes one fails a test.

### Running a script

- A script run has its own fresh `session_id`, so a `SET` reaches the
  run's later statements and never browsing; the session expires on the
  server. Statements run in order, each a POST, `limit` rows kept per
  statement. The outcome ends `ScriptEnd::RolledBack`: nothing the run
  did remains, as on every read-only run.
- Cancel: `KILL QUERY WHERE query_id = '…' ASYNC` on the pool's second
  connection; `StopFlag` is read between statements. A killed statement
  ends with code 394, `Error::Cancelled`.
- `reserved.rs` and `sql::keywords` gain ClickHouse's keywords, which
  highlighting and completion read. The formatter keeps every word's
  case outside its keywords (`structure` false, as on MySQL: ClickHouse
  names and many functions are case-sensitive), and `dashes` is false.

## Agents (MCP)

- An agent's read runs under `readonly=1` (no `SET`, no top-level
  `SETTINGS`, no `url()` or `file()`), with the agent's limits as
  `max_result_rows` and `max_execution_time`: a `SETTINGS` in a subquery
  does not lift them (probed). `check_read` also refuses a `SETTINGS`
  clause at any depth, read from sqlparser's tree (a query's `settings`),
  not from words, so `system.settings` stays readable.
- `check_read` takes the allowlist above without `SET` and `USE`, then
  parses with sqlparser's `ClickHouseDialect`; what it cannot parse is
  refused, as on the other engines.
- `effects` refuses, as a second fence behind `readonly=1`, every table
  function but those that only make rows (`numbers`, `numbers_mt`,
  `zeros`, `zeros_mt`, `values`, `generateRandom`, `null`): an allowlist,
  since ClickHouse adds table functions each release. A table function
  is found by its place in sqlparser's tree (a table with arguments, at
  any depth, under sqlparser's recursion limit), not by its name: `file`,
  `null` and `values` are scalar functions too. Of scalar functions it
  refuses `file` (it reads a file in `user_files` under `readonly=1`,
  probed), `sleep` and `sleepEachRow`, as PostgreSQL's and MySQL's lists
  refuse theirs.
- Placeholders are ClickHouse's own: the agent writes `{p1:Type}`,
  `{p2:Type}`, …, where the n-th value it sends is `p<n>` and is sent as
  `param_p<n>`. A name may repeat; any other name, a `pN` past the
  values sent, or braces holding anything but a placeholder token, is
  refused. The server binds them. For sqlparser, which
  reads `{p:String}` as a dictionary, each placeholder is handed over as
  `NULL` at the same place; what is sent is the text as written. The MCP
  connection listing names the style.
- A connection that hides a table or masks a column
  (`Sources::guards_anything`) refuses every agent read on it, with the
  sentence "ClickHouse connections cannot hide tables or mask columns
  from agents yet". The agent settings of a ClickHouse connection say so
  beside those controls. `agents_catalog` answers `Error::Unsupported`;
  nothing asks it while nothing is guarded. In `agent/hidden.rs` and
  `hidden/masked.rs`, the ClickHouse arms are unreachable for that
  reason and answer as SQLite's do where a type is needed.
- What an agent sent goes into the log of calls only, and an error's
  code, not its words, as for the other engines.

## The app

- The connection dialog offers ClickHouse in its engine choice
  (`choice.rs`), with the URL placeholder
  `clickhouse://user@host:8123/database`, TLS and SSH as for PostgreSQL
  and MySQL, and the port moving with the TLS mode (above).
- The read-only box is ticked and disabled for ClickHouse, with the line
  "ClickHouse connections open read-only".
- `connections.toml` keeps the driver as `clickhouse`; `managed.rs`
  rules match it by that key.
- The tree lists the system databases as system schemas; the bare schema
  is the spec's database, else `default`.
- The grid is read-only, as for any table without a row key: no edit,
  insert, delete, duplicate or lookup. Structure lists the sorting key
  and skipping indexes; the DDL view shows `SHOW CREATE`. In
  `ui/ddl.rs`, code 497 (`ACCESS_DENIED`) is a refused read, with
  "The connection's user can't run SHOW CREATE here. Rows and Structure
  still work."
- `ui/format.rs`'s `refuses_writes` takes code 164 (`READONLY`).
- `src/dates.rs` and `src/edit.rs` take the ClickHouse types (above);
  `labels.rs`, `env.rs`, `session.rs`, `review.rs` and the workspace and
  start screens name the engine where they name the others.

## Testing

- Unit tests: the reader (moved with its tests), URL parsing, the
  lexing (nested comments, `#` forms, heredocs, `{name:Type}`), the
  allowlist with every probed `EXPLAIN` form, `raw_where`'s refusals,
  `effects`, placeholders, the type classes and `temporal()`, the
  `prefer` decision, `tls_port` and `writes` for every driver.
- Headless UI tests (`src/testing.rs`): ClickHouse in the dialog, the
  read-only box locked, the port following TLS, the agent settings'
  sentence.
- Against the compose server, gated by `TABLETIST_TEST_CLICKHOUSE_URL`:
  a Bookshop fixture (`crates/tabletist-db/fixtures/clickhouse.sql`);
  the catalog, rows, counts, filters, sorting, `raw_where` and DDL;
  scripts with `SET`; cancel; every row of the probe table at the level
  each kind of request sends; TLS on 8443 with a test certificate in
  `compose/clickhouse/` for each `TlsMode`; a tunnel through the `ssh`
  service to `clickhouse:8123`, with TLS's name checked against the real
  host; agents' reads, refusals and the guarded-connection refusal.
- A `clickhouse integration` job in `ci.yml` beside the PostgreSQL,
  MySQL and SSH ones, with the `clickhouse`, `agent` and `ddl` suites.
  AGENTS.md's integration-test lines gain the `clickhouse` service and
  `TABLETIST_TEST_CLICKHOUSE_URL`.
- `tests/engines.rs` holds as it is: no `==`, `!=` or `matches!` on the
  enums.

## Website

`_reference/connections.md` (the engine, its URLs, read-only),
`_reference/settings-and-files.md` (the `clickhouse` driver key), and the
guides that list engines: getting started, the SQL editor (the
allowlist), browsing data, agents (no hiding or masking yet). No version
is named.

## Phases, a plan each

Each phase leaves `main` working and its checks passing.

1. **Transport.** `clickhouse/http.rs` with TLS, the `prefer` probe, the
   pool, `KILL QUERY`; the reader moved; the ureq spike removed; the
   compose TLS port and certificate. Its tests are in-crate (`http.rs`'s
   own, gated by the URL variable, as `ssh::tests` are) and open their
   tunnel with `ssh::Tunnel::open` themselves. Nothing names a
   `Driver` yet.
2. **Engine.** `Driver::ClickHouse` and `Dialect::ClickHouse`, URLs,
   `tls_port` and `writes`, the adapter (catalog, rows, count, DDL,
   scripts), lexing and the allowlist, types, errors; the fixture and the
   CI job. Every match in the app and in the agent code compiles here
   with its final answer where it is small (labels, dates, formats,
   `refuses_writes`, `ui/ddl.rs`), and with two holds until their phase:
   the dialog's engine choice does not list ClickHouse and a pasted
   `clickhouse://`, `http://` or `https://` URL is refused there as "not
   offered yet" (phase 3); every agent read on a ClickHouse connection
   is refused and the MCP listing leaves those connections out (phase 4).
3. **App.** The dialog's choice and URLs, the read-only box, the port
   moving with TLS, tree, structure, managed rules.
4. **Agents.** `check_read`, `effects`, placeholders, the guarded
   refusal, the MCP listing.
5. **Website.**

## Not in this spec

Writes of any kind (editor, grid, insert), hiding and masking for
agents, ClickHouse Cloud's idle wake-up shown as connecting, the native
TCP protocol, compression.
