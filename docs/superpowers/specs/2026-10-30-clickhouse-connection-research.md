# Connecting to ClickHouse: research

Status: research, nothing decided. Written 2026-10-30.

What it would take for Tabletist to open a ClickHouse database, which way
to talk to it, and where its model differs from the three engines we have.

## Ways in

ClickHouse speaks four protocols. Ports are the defaults.

| Protocol | Port | Notes |
| --- | --- | --- |
| HTTP / HTTPS | 8123 / 8443 | Always on. What ClickHouse Cloud, proxies and most GUIs use. |
| Native TCP | 9000 / 9440 (TLS) | `clickhouse-client`'s. Columnar blocks, revision-negotiated. |
| MySQL wire | none (`mysql_port`, often 9004; 3306 on Cloud) | Off by default, on Cloud too. |
| PostgreSQL wire | none (`postgresql_port`, often 9005) | Off by default. |

### Options for a client

1. **Official `clickhouse` crate** (ClickHouse/clickhouse-rs, 0.15.3, MSRV
   1.89). HTTP, RowBinary. Built for typed rows (`#[derive(Row)]`); a
   query with unknown columns goes through `fetch_bytes(format)`, so we
   would parse a text or `RowBinaryWithNamesAndTypes` stream ourselves
   anyway. `rustls-tls-ring` exists, so no aws-lc. It brings hyper,
   hyper-util, hyper-rustls, lz4_flex, cityhash-rs, bnum, bstr, rand 0.9,
   none of which is in the tree. `Cargo.toml` chose ureq over reqwest
   precisely to keep hyper out.
2. **Our own HTTP client on ureq** (already in the tree, rustls on ring,
   blocking). One statement per POST, run on `spawn_blocking` as SQLite's
   calls are, reading a self-describing text format line by line. The
   protocol is small: auth headers, settings as URL parameters, one
   status code, one mid-stream error marker. No new crate.
3. **Native TCP crates** (klickhouse and its fork nativeclick, and
   clickhouse-native-client 0.1). Third-party and young; the protocol
   revision moves with the server (klickhouse 0.15 is reported to return
   empty results on ClickHouse 26.9 with ZSTD by default). Not worth the
   churn for a GUI that reads pages of rows.
4. **MySQL wire through `mysql_async`** (no new code path). Off by default
   on every server, no prepared statements (our row fetches bind `?`
   parameters), some types sent as strings, a forced
   `prefer_column_name_to_alias = 1`, and auth/TLS quirks. Fine for a quick
   try against a server that has it on, not as the way in.

**Recommendation: option 2**, HTTP on ureq. It matches how Tabletist
already does HTTP, adds no dependency, works with Cloud and with an SSH
tunnel (it is plain TCP to one port), and the parts that matter to a GUI
(settings per request, `query_id`, errors) are all HTTP-level. Option 1 is
the fallback if we want compression or RowBinary decoding someone else
maintains; it costs the hyper stack. To verify before building: that
ureq 3's TLS config can take `tls.rs`'s roots and CA file for
`verify-ca` / `verify-full`, and what `verify-ca` (no hostname check)
needs there.

## The HTTP interface, what a client must handle

- **Auth**: `X-ClickHouse-User` / `X-ClickHouse-Key` headers (not URL
  parameters, which proxies log). Database: `X-ClickHouse-Database`.
- **One statement per request.** Multi-statements are refused by the
  server. `sql::statements` already splits a script.
- **Settings ride on the URL**: `readonly`, `max_result_rows`,
  `result_overflow_mode=break`, `max_execution_time`, `query_id`,
  `session_id`, `default_format`. `SET` only lasts within a `session_id`,
  and a session runs one query at a time (a second gets "session is
  locked"), so browsing must not share the script's session.
- **Errors**: before output, HTTP 500 with the message. After output has
  started, HTTP 200 and a block between `__exception__` lines carrying a
  tag that also comes in `X-ClickHouse-Exception-Tag`. A reader that stops
  at a row count must still look for it. `X-ClickHouse-Exception-Code`
  gives the numeric code (what the agent log would keep).
- **Parameters**: `{name:Type}` in the text, values as `param_name`
  URL/form fields in escaped format. The server binds them, which is what
  `run_agent_read` needs.
- **Progress**: `send_progress_in_http_headers` gives `X-ClickHouse-Progress`.
  `X-ClickHouse-Timezone` gives the session zone for free.
- **Cancel**: a dropped connection does not stop a query by itself. Send
  every statement with our own `query_id` and cancel with
  `KILL QUERY WHERE query_id = '…' ASYNC` on a second request (allowed
  whatever `readonly` is). `cancel_http_readonly_queries_on_client_close`
  also exists; check its reference before relying on it.
- **Format for rows**: `JSONCompactEachRowWithNamesAndTypes` or
  `TabSeparatedWithNamesAndTypes`. Both give names and ClickHouse type
  names first, then one row per line, so a page streams and stops. TSV is
  smaller and lossless (escaped); JSON quotes 64-bit integers only with
  `output_format_json_quote_64bit_integers`, which is on by default.
- `/ping` answers a health check (compose's healthcheck).

## Where the model differs

### No transactions

ClickHouse has no read-only transaction and no rollback (its transactions
are experimental and MergeTree-only). Today every read path, and every
script, leans on one:

- Browsing reads in a read-only transaction. ClickHouse: send `readonly=2`
  on every browsing and agent request (blocks writes and DDL, still lets
  us pass settings). `readonly=1` would also refuse our own
  `max_result_rows` and the like.
- A read-only script always rolls back. ClickHouse: `readonly=2` per
  statement. Note that level 2 permits `CREATE TEMPORARY TABLE` and
  `RESTORE`; `sql::refusal` or `check_read` would refuse `RESTORE`, as the
  agent path refuses other effects.
- A write script commits or rolls back as a whole (`ScriptEnd`). On
  ClickHouse each statement stands on its own. The closest honest outcome
  is MySQL's implicit-commit case: `ScriptEnd::Partly` with what applied.
  The UI's "rolled back" promise cannot be made.
- A user whose profile is `readonly=1` cannot receive any setting in the
  URL. The connection must notice (`SELECT getSetting('readonly')`) and
  fall back to `LIMIT` in SQL.

### Editing rows

Tabletist finds a row by its primary key or a unique non-null index
(`Structure::row_key`) and locks it with `SELECT … FOR UPDATE` before a
save. ClickHouse has neither: its `ORDER BY` / `PRIMARY KEY` is a sorting
key with duplicates allowed, there are no unique indexes and no row locks.
Writes are `ALTER TABLE … UPDATE/DELETE` mutations (asynchronous, heavy),
lightweight `DELETE FROM` (marks rows, visible at once), and lightweight
`UPDATE` (beta, needs `enable_block_number_column` and
`enable_block_offset_column` on the table, cannot touch key columns).

So `row_key` is `None` for every ClickHouse table and the grid is
read-only, which the app already handles. `INSERT` works and could come
later. First release: read-only browsing, SQL editor (writes from the
editor allowed, with the per-statement outcome above).

### Catalog

- Databases are ClickHouse's only namespace. Map them as MySQL does:
  `list_databases` lists them, `list_schemas` gives the one connected.
  System schemas: `system`, `INFORMATION_SCHEMA`, `information_schema`.
- `system.tables`: name, `engine` (table / view / `MaterializedView` /
  `Dictionary`, and `Distributed`, `Kafka`…), `comment`, `total_rows`,
  `is_temporary`.
- `system.columns`: type, `default_kind` / `default_expression`,
  `is_in_primary_key`, `is_in_sorting_key`, `is_in_partition_key`,
  `comment`. `system.data_skipping_indices` for indexes. No foreign keys,
  so lookups and the foreign key picker have nothing to show.
- DDL: `SHOW CREATE TABLE` (as MySQL's `show_create`).
- `count()` on MergeTree is read from metadata, cheap. Paging needs an
  `ORDER BY` for stable pages; default to the sorting key.

### Types

`column_class` and `ValueKind` need a ClickHouse parser that unwraps
`Nullable(…)` and `LowCardinality(…)` first, then:

- `Int8…Int64`, `UInt8…UInt32` to `Int`; `UInt64`, `Int128`, `UInt128`,
  `Int256`, `UInt256` as `Text` with `Numeric` (the existing rule for wide
  integers). `Decimal(P,S)` as text, `Float32/64`, `BFloat16`.
- `Bool`; `String` (bytes, may not be UTF-8: `Bytes` when it is not),
  `FixedString(N)`; `UUID`; `IPv4`, `IPv6`; `Enum8/16`.
- `Date`, `Date32`, `DateTime('tz')`, `DateTime64(p, 'tz')`, `Time`,
  `Time64`: the shared `temporal()` and `src/dates.rs` need the zone in
  the type, not the session.
- `Array(T)`, `Tuple(…)`, `Map(K,V)`, `Nested`, `JSON`, `Variant`,
  `Dynamic`: shown as text, `Json` kind where it helps.

### SQL dialect

- Identifiers: backticks or double quotes, case-sensitive. Strings: single
  quotes with backslash escapes. `#` starts a comment, as on MySQL.
- `ILIKE` exists; `LIMIT n OFFSET m`, also `LIMIT n BY col`. No
  `RETURNING`, no `FOR UPDATE`, no `DEFAULT` in `VALUES`.
- `FORMAT`, `SETTINGS` and `INTO OUTFILE` clauses: `sql::refusal` should
  refuse `INTO OUTFILE` and a `FORMAT` that would break our reader, and
  the editor appends its own format.
- sqlparser 0.63 has `ClickHouseDialect` for `agent::check_read`.
- A ClickHouse list for `reserved.rs` and `sql::keywords`.

### Agents (MCP)

The hidden-table check needs a plan that names what a statement reads.
Candidates: `EXPLAIN PLAN json = 1, actions = 1` (`ReadFromMergeTree`
steps name their table) or `EXPLAIN QUERY TREE`. Views, dictionaries,
table functions (`url()`, `s3()`, `file()`, `remote()`) and `Distributed`
tables all read elsewhere and belong in `effects` / opaque relations. If
no plan is reliable, an agent connection to ClickHouse that hides or masks
anything refuses every read, which is honest. ClickHouse's own row
policies and column grants are the better tool for that case.

## What changes in Tabletist

Counts from a sweep of the code: about 300 match arms on `Driver` /
`Dialect` outside tests, in about 110 functions. A new variant fails to
compile at each of them, which is the point of the rule.

- `crates/tabletist-db`: `Driver::ClickHouse` (`spec.rs`, serde
  `clickhouse`, port 8123, TLS port 8443), `Dialect::ClickHouse`,
  `clickhouse://` / `clickhouses://` URLs (also `https://` with a `?secure`
  flag?), a `clickhouse` adapter module with the 18 `Adapter` methods,
  `Inner::ClickHouse`, `CancelInner` with the `KILL QUERY` route. SSH as
  for PostgreSQL and MySQL: connect to `127.0.0.1:via`, and keep the
  spec's host for TLS's name check.
- Shared code: `dialect.rs` (quoting, `select_rows`, `count_rows`, filters,
  `bind`), `sql.rs` lexing and refusal, `reserved.rs`, `class.rs`,
  `value.rs`, `src/dates.rs`, `src/edit.rs`.
- Agents: `agent.rs` dialect, placeholders (`{name:Type}`), effects,
  hidden-table catalog.
- App: the engine choice in the connection dialog, its URL placeholder,
  `connections.toml` driver key, `managed.rs` rules by driver key, labels,
  workspace and start screens, the MCP connection listing.
- Tests: a `clickhouse` service in `compose.yaml`
  (`clickhouse/clickhouse-server`, 127.0.0.1:58123, healthcheck on
  `/ping`), `TABLETIST_TEST_CLICKHOUSE_URL`, a Bookshop fixture in
  `crates/tabletist-db/fixtures/clickhouse.sql`, a CI job beside the other
  three.
- Website: `_reference/connections.md` and the guides.

## Suggested order

1. Spike: an HTTP reader on ureq that runs one `SELECT` against the
   compose server and fills a `RowPage` from
   `TabSeparatedWithNamesAndTypes`, with the mid-stream error and
   `KILL QUERY`. Decides option 2 against option 1.
2. Read-only engine: connect, catalog, browse, count, DDL, SQL editor in
   read-only mode, types. Grid read-only, no lookups.
3. SQL editor writes with per-statement outcomes.
4. Agents, once a plan reliably names the tables a read touches.
5. Inserting rows, later, if anyone asks.

## Spike results (2026-10-31)

`crates/tabletist-db/src/clickhouse.rs` (not an engine: no `Driver`
names it), tested by `crates/tabletist-db/tests/clickhouse.rs` against
the compose service (25.8 LTS) and against 26.10. All six server tests
pass on both. It is about 560 lines with its unit tests and adds
no crate (ureq was in the tree; `Cargo.lock` gains one line).

What it does: one POST per statement with `readonly=2`,
`max_result_rows` = page + 1 and `result_overflow_mode=break`, reads
`TabSeparatedWithNamesAndTypes` a line at a time and stops at the page,
gives each statement its own `query_id`, and cancels with
`KILL QUERY … ASYNC` from a second request.

What the server showed:

- An error before the rows can come with 404 or 500, not only 500. Any
  status but 200 is an error, with `X-ClickHouse-Exception-Code`.
- An error after the rows began: 25.8 writes `__exception__\r\n`, the
  message and closes the connection mid-chunk (no tag header). 26.10
  writes `\r\n__exception__\r\n<tag>\r\n<message>\n<len> <tag>\r\n__exception__\r\n`
  and sends `X-ClickHouse-Exception-Tag`. A TSV row cannot end in `\r`
  (TSV escapes it), so a line ending `\r\n` starts the error on both.
- The type line is escaped like a field: `DateTime64(3, \'UTC\')`.
- A killed query ends with HTTP 500 and code 394 (`QUERY_WAS_CANCELLED`).
  `KILL QUERY … ASYNC` returns at once.
- `readonly=2` refuses `CREATE TABLE` with code 164 (`READONLY`).
- Comparisons are `UInt8`, not `Bool`: `a = b` shows as 0/1.
- `String` is bytes: `unhex('ff00')` arrives as `\xff\0`, which the
  reader keeps as `Bytes`.

The catch: TLS and SSH. ureq's stable API takes root certificates
(`RootCerts::Specific` for a CA file, Mozilla's roots by default; the
platform verifier needs a feature we do not build) or no verification at
all. It cannot take `tls.rs`'s own `rustls::ClientConfig`, so neither
`verify-ca` (chain, no name check) nor the OS roots the other engines
trust. Through an SSH tunnel the request goes to `127.0.0.1:<via>` while
the certificate names the real host, which needs a custom `Resolver` or
`Connector`: both are in ureq's `unversioned` module, outside semver.

**Revised recommendation.** Keep the HTTP interface, the format and the
reader (`read_tsv`, `value`, `kind`, `server_error` carry over as they
are). For the real engine, put them on the transport the other engines
already have: a tokio `TcpStream` to `via` or the host, wrapped by
tokio-rustls (in the tree, 0.26) with `tls::client_config` and the spec's
host as the server name, and HTTP/1.1 spoken by ureq-proto (in the tree,
sans-IO, what ureq uses inside). That keeps TLS modes and tunnels exactly
as PostgreSQL and MySQL have them, makes the reads async instead of
`spawn_blocking`, and still adds no crate. The official `clickhouse`
crate is no longer needed as a fallback: nothing the spike did needed it.

Not tried: a user whose profile is `readonly=1` (every setting in the
URL is refused), HTTPS, ClickHouse Cloud.

An `Http` runs one statement at a time, as a PostgreSQL or MySQL session
does, so its `Canceller` always names the one running. The engine could
let browsing and a script run side by side, each over its own request,
with a `query_id` each.

## Open questions

- Is the first release read-only for everything, or does the editor write
  from the start?
- `clickhouses://` or `clickhouse://?secure=true` for TLS, and what does
  `TlsMode::Prefer` mean over HTTP (no in-protocol upgrade: it can only
  try HTTPS, then HTTP)?
- Do we want Cloud's idle services (first request can take seconds while
  the service wakes) to show as "connecting", not as a timeout?

## Sources

- [ClickHouse HTTP interface](https://clickhouse.com/docs/interfaces/http)
- [Permissions for queries (`readonly`)](https://clickhouse.com/docs/operations/settings/permissions-for-queries)
- [MySQL interface](https://clickhouse.com/docs/interfaces/mysql)
- [Lightweight UPDATE](https://clickhouse.com/docs/sql-reference/statements/update)
- [Rust client docs](https://clickhouse.com/docs/integrations/language-clients/rust),
  [docs.rs `clickhouse`](https://docs.rs/clickhouse/latest/clickhouse/),
  [its Cargo.toml](https://github.com/ClickHouse/clickhouse-rs/blob/main/Cargo.toml)
- [nativeclick README](https://docs.rs/crate/nativeclick/latest/source/README.md),
  [clickhouse-native-client](https://docs.rs/clickhouse-native-client)
