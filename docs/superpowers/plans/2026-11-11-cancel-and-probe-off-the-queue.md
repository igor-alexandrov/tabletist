# Cancels and the PgBouncer probe off the session queue

A session runs its commands one at a time (`run_session`). Two things hold
that queue up far from the server (~200 ms a round trip) without doing the
user's work.

## 1. A cheap read is cancelled only once it outlasts a cancel

Today a superseded page, describe or lookup that is still running takes a
cancel at once, and the session waits for that cancel before its next
command, so that a late one cannot stop it. A cancel is a new connection:
on MySQL an authenticated one with two queries (6–8 trips), on PostgreSQL a
TCP (and TLS) connection. Clicking through tables waits on one each time,
though the read it stops would have finished in a trip or two.

- `Running` learns whether the running command is a cheap read:
  `FetchRows`, `Describe`, `ReadDdl`, `Lookup`. A count, a script, a save
  and the tree's listings keep today's cancel at once.
- `cancel_request` on a cheap read sends no cancel. It spawns a timer that
  waits a grace and then, under the lock, cancels only if the same request
  still runs, pushing the cancel into `cancels` as today. The timer itself
  is never in `cancels`: the next command never waits on it, only on a
  cancel that was really sent (the one wait the session keeps).
- Its answer is dropped as any superseded answer is: the app already
  ignores a page, describe or lookup for a request it no longer waits on.
- The grace is what a cancel would cost: the time the session took to
  connect, at most 2 s (rent until the rent equals the price). Measured in
  the worker's connect task around `Connection::connect_with`.
- A second Cancel of the same request does not start another timer.
- A Close still stops everything at once (unchanged path).

Tests (backend.rs, with a held-back cancel as the existing ones do):

- a cheap read that finishes within the grace is never cancelled, and the
  next command starts without waiting on a cancel;
- one that outlasts the grace takes one cancel, and the next command waits
  for it;
- a count is still cancelled at once.

## 2. The PgBouncer probe runs beside the queue

`ListDatabases` on PostgreSQL opens a second connection to the `pgbouncer`
database (~6 trips) before the session can run anything else. The probe
needs nothing of the session's connection.

- `tabletist_db::Connection` splits the listing: `database_probe()` returns
  an owned future for the probe (PostgreSQL only; `None` elsewhere) and
  `catalog_databases()` the listing from the session's catalog.
  `list_databases()` stays, composing the two as today.
- `run_session` on `ListDatabases`: spawn the probe, run only the catalog
  query in the queue (one trip), and answer from a spawned task once the
  probe ends: its aliases if it found PgBouncer, the catalog's answer if
  not. A lost connection is seen in the loop as for every command.
- `after_connect` sends `ListDatabases` after the tree's commands, so the
  schemas and objects run first.

Tests: the existing probe tests in pg.rs stay (the probe is the same
code, now an owned future); an app test that a PostgreSQL tab asks for its
schemas before its databases. The session side is covered by the latency
measurement below: it needs a PostgreSQL server.

## Measure

A delay proxy in front of the test PostgreSQL (55432) and MySQL (53306),
100 ms each way, and a throwaway program (not committed) on the backend:

- click-through: fetch a page of table A, then at once cancel it and fetch
  table B; time to B's rows;
- connect to tree: time from Connected to the first `Objects`, and to a
  page fetched right after connect.

Before and after numbers go in the PR. Then `cargo test`, the server tests
with the two URLs, and `cargo clippy --all-targets -- -D warnings`.

MySQL's cancel connection options (`max_allowed_packet`, `wait_timeout`, to
skip the driver's settings query) are left to the MySQL round-trips
session, which owns `mysql.rs` connection options.
