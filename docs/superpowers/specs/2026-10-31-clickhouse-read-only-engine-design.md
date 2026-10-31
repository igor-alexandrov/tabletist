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
| The read-only fence | The server's `readonly=2` behind a first-keyword allowlist of our own |

## What the server does under `readonly`

Probed on 25.8 (compose) on 2026-10-31, and the reason for the fence's
shape. `readonly=2`, which the research doc took for a fence, lets
writes through:

| Statement | `readonly=1` | `readonly=2` |
| --- | --- | --- |
| `INSERT INTO FUNCTION file(…)`, `url(…)` | refused | runs (wrote a file) |
| `BACKUP DATABASE … TO File(…)` | runs | runs |
| `KILL QUERY …` | runs | runs |
| `CREATE TEMPORARY TABLE` | refused | runs (session only) |
| `SET x = …`, `SELECT … SETTINGS x = …` | refused | runs |
| `SET readonly = 0`, `SETTINGS readonly = 0` | refused | refused |
| `SELECT … FROM url(…)` | refused | runs |
| `OPTIMIZE`, `SYSTEM …`, `CREATE TABLE` | refused | refused |
| `SELECT … INTO OUTFILE` (over HTTP) | refused | refused |
| Settings in the URL (`max_result_rows`, `query_id`, `param_*`) | accepted | accepted |

`BACKUP` and `KILL` pass at both levels, so a check of our own is needed
whichever level is sent. With `readonly=2` the editor keeps `SET` and
`SETTINGS`, which ClickHouse users rely on, and reads through `url()` and
`s3()`.

## Architecture

All of it is in `crates/tabletist-db/src/clickhouse/`, beside `pg/` and
`mysql/`, and `crates/tabletist-db/src/clickhouse.rs`:

- `clickhouse/http.rs`, the transport. A tokio `TcpStream` to
  `spec.host:spec.port`, or to `127.0.0.1:via` through an SSH tunnel.
  With TLS, tokio-rustls wraps it with `tls::client_config(mode,
  ca_file)`, and the server name is always `spec.host`, so a tunnel
  checks the real host's certificate. HTTP/1.1 is ureq-proto's sans-IO
  `Call`, which ureq uses inside; it is pinned to ureq's version and
  moves with it. No crate is added: tokio-rustls 0.26 and ureq-proto are
  in the tree. Connections are kept alive in a pool of two: one runs
  statements, the other sends `KILL QUERY` while one runs.
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
URL: `readonly=2`, `max_result_rows` with `result_overflow_mode=break`,
`max_execution_time` where a timeout applies, `query_id`, `session_id`
for a script, and `param_*` for bound values. Rows come back as
`TabSeparatedWithNamesAndTypes`.

A user whose own profile is `readonly=1` is refused every setting.
`connect` asks `SELECT getSetting('readonly')`, and for such a user sends
none: the page's limit goes into the SQL as `LIMIT`, scripts get no
session, and a timeout is the client's own (it cancels with
`KILL QUERY`).

## The engine in `tabletist-db`

### Identity and connecting

- `Driver::ClickHouse` (serde `clickhouse`, label "ClickHouse", default
  port 8123) and `Dialect::ClickHouse`. Every match on the two names it,
  as AGENTS.md asks.
- `Driver::tls_port()`: 8443 for ClickHouse. The dialog moves the port
  between 8123 and 8443 when the TLS mode changes, only while it holds
  the other default.
- URLs: `clickhouse://user:pass@host[:port]/db` reads `sslmode`,
  `ssl-mode`, `ssl_mode`, `sslrootcert` and `ssl-ca` as the other
  engines' URLs do. `http://…` is `disable` on 8123; `https://…` is
  `verify-full` on 8443. The empty database is the user's default.
- SSH as for PostgreSQL and MySQL: `Connection::connect_with` opens the
  tunnel and passes its port as `via`.
- `Driver::writes()`: false for ClickHouse. `Connection::connect_with`
  opens a ClickHouse session `Access::ReadOnly` whatever was asked. The
  adapter's `write` answers `Error::ReadOnly`; a `ScriptMode::Write` run
  is refused before anything is sent, as on every read-only session.
- `server_version` is `SELECT version()`. `session_zone` is the
  `X-ClickHouse-Timezone` header of the first answer.

### Catalog

Databases are schemas, as on MySQL.

- `list_databases`: empty. `list_schemas`: `system.databases`.
  `system`, `INFORMATION_SCHEMA` and `information_schema` are system
  schemas (`is_system_schema`).
- `list_objects`: `system.tables` for the schema. `engine = 'View'` is a
  view, `MaterializedView` a materialized view (that kind's doc stops
  saying PostgreSQL only), anything else a table (`Distributed`,
  `Dictionary`, `Kafka` among them). `total_rows` is the estimate.
- `describe`: `system.columns` for the columns: type, `Nullable(…)` for
  nullable, `default_kind` and `default_expression` (`MATERIALIZED` and
  `ALIAS` are `generated`; `DEFAULT` is the default), comment, and an
  `Enum8`/`Enum16`'s labels as `allowed_values`. No primary key: a
  ClickHouse `PRIMARY KEY` is a sorting key with duplicates, so the
  sorting key (`system.tables.sorting_key`) and each row of
  `system.data_skipping_indices` are non-unique `IndexInfo`s (method
  `sorting key`, or the skipping index's type). No foreign keys either
  way. `row_key` is therefore `None`, and the comment is
  `system.tables.comment`.
- `ddl`: `SHOW CREATE TABLE`, for views and dictionaries too.

### Rows

- `fetch_rows` and `count_rows` build their SQL in `dialect.rs` for
  `Dialect::ClickHouse`: identifiers in backticks (a backtick doubled),
  `ILIKE` for contains, the page's `OFFSET` in the SQL and its size as
  `max_result_rows` (or `LIMIT`, above).
- Values are never spliced into SQL. A filter's value is a parameter
  `{p1:String}` sent as `param_p1`, cast to the column's type in the SQL
  where the operator needs it.
- With no sort chosen, rows are ordered by the sorting key, so paging is
  stable and `ordered_by_key` is true. A table without one (`ORDER BY
  tuple()`) pages unordered, as a heap table does.
- `raw_where` is appended as on the other engines, and is a read under
  the same fence (below).
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

## The SQL editor and the fence

- `sql::statements` splits a script with ClickHouse's lexing: backticks,
  `--`, `#` and `/* */` comments, single-quoted strings with backslash
  escapes, no dollar quotes.
- `sql::refusal`, for `Dialect::ClickHouse`, refuses a statement whose
  first keyword is not `SELECT`, `WITH`, `SHOW`, `DESC`, `DESCRIBE`,
  `EXPLAIN`, `EXISTS`, `SET` or `USE`, and one that names a `FORMAT` or
  `INTO OUTFILE` of its own. It refuses before anything is sent, and the
  sentence names the keyword. `readonly=2` is sent with every statement
  behind it, as `query_only` backs SQLite's authorizer.
- A script run has its own fresh `session_id`, so a `SET` reaches the
  run's later statements and never browsing; the session expires on the
  server. Its statements run in order, each a POST, `limit` rows kept per
  statement. The outcome ends `ScriptEnd::RolledBack`: nothing the run
  did remains, as on every read-only run.
- Cancel: `KILL QUERY WHERE query_id = '…' ASYNC` on the pool's second
  connection; `StopFlag` is read between statements. A killed statement
  ends with code 394, `Error::Cancelled`.
- Errors: any status but 200 is the server's error with
  `X-ClickHouse-Exception-Code`; one written after the rows began is
  read as the spike does (a line ending `\r\n`, tagged or not). 394 is
  `Cancelled`, 159 `Timeout`, 516 and 194 `Auth`, the rest `Query` with
  the code. A lost connection is `ConnectionLost`.

## Agents (MCP)

- `check_read` parses with sqlparser's `ClickHouseDialect`; what it
  cannot parse is refused, as on the other engines. The read then passes
  `sql::refusal`'s allowlist and runs under `readonly=2`, with the
  agent's limits as `max_result_rows` and `max_execution_time` (the
  server enforces the timeout).
- `effects` for ClickHouse refuses the table functions that reach
  outside the server: `url`, `s3`, `s3Cluster`, `gcs`, `azureBlobStorage`,
  `hdfs`, `file`, `remote`, `remoteSecure`, `cluster`,
  `clusterAllReplicas`, `mysql`, `postgresql`, `mongodb`, `redis`,
  `sqlite`, `odbc`, `jdbc`, `executable`, `iceberg` and its variants,
  `deltaLake`, `hudi`, `input`. Those that make rows (`numbers`,
  `zeros`, `values`, `generateRandom`) and the `system` tables stay.
- Placeholders are ClickHouse's own `{name:Type}`, sent as `param_name`
  and bound by the server. The MCP connection listing names the style.
- A connection that hides a table or masks a column (`Sources::
  hides_anything`, or a masked column) refuses every agent read on it,
  with the sentence "ClickHouse connections cannot hide tables or mask
  columns from agents yet". The agent settings of a ClickHouse
  connection say so beside those controls. `agents_catalog` answers
  `Error::Unsupported`; nothing asks it while nothing is hidden.
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
  and skipping indexes; the DDL view shows `SHOW CREATE TABLE`.
- `src/dates.rs` and `src/edit.rs` take the ClickHouse types (above);
  `labels.rs`, `env.rs`, `session.rs` and the workspace and start
  screens name the engine where they name the others.

## Testing

- Unit tests: the reader (moved with its tests), URL parsing, the
  allowlist and its refusals, the lexing, `effects`, the type classes
  and `temporal()`, the `prefer` decision.
- Headless UI tests (`src/testing.rs`): ClickHouse in the dialog, the
  read-only box locked, the port following TLS, the agent settings'
  sentence.
- Against the compose server, gated by `TABLETIST_TEST_CLICKHOUSE_URL`:
  a Bookshop fixture (`crates/tabletist-db/fixtures/clickhouse.sql`);
  the catalog, rows, counts, filters, sorting and DDL; scripts with
  `SET`; cancel; every row of the `readonly` table above, so a server
  that changes them fails a test; TLS on 8443 with a test certificate in
  `compose/clickhouse/` for each `TlsMode`; a tunnel through the `ssh`
  service to `clickhouse:8123`, with TLS's name checked against the
  real host; agents' reads, refusals and the hidden-table refusal.
- A `clickhouse integration` job in `ci.yml` beside the PostgreSQL,
  MySQL and SSH ones, with the `clickhouse`, `agent` and `ddl` suites.
- `tests/engines.rs` holds as it is: no `==`, `!=` or `matches!` on the
  enums.

## Website

`_reference/connections.md` (the engine, its URLs, read-only),
`_reference/settings-and-files.md` (the `clickhouse` driver key), and the
guides that list engines: getting started, the SQL editor (the
allowlist), browsing data, agents (no hiding or masking yet). No version
is named.

## Phases, a plan each

1. **Transport.** `clickhouse/http.rs` with TLS, the `prefer` probe, the
   pool, `KILL QUERY`; the reader moved; the ureq spike removed. Tested
   against compose, with TLS and SSH.
2. **Engine.** `Driver` and `Dialect`, URLs, the adapter (catalog, rows,
   count, DDL, scripts), the fence, types; the fixture and the CI job.
3. **App.** Dialog, tree, structure, grid, dates, managed rules, labels.
4. **Agents.** Dialect, placeholders, `effects`, the refusal while
   anything is hidden or masked.
5. **Website.**

## Not in this spec

Writes of any kind (editor, grid, insert), hiding and masking for
agents, ClickHouse Cloud's idle wake-up shown as connecting, the native
TCP protocol, compression.
