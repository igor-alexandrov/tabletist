# A MariaDB cancel by query ID: design note

Status: decided on 2026-11-11. `KILL QUERY ID` is dropped. Alternative 1 is
built, stacked on the spare connection (#310). Alternative 2 waits for
measurements.

## The question

On MySQL and MariaDB a cancel is `KILL QUERY <thread id>`, sent through the
session's spare connection. It stops whatever the thread is running when it
arrives. So `run_session` waits for every outstanding cancel before it starts
the next command. MariaDB 10.0.5+ has `KILL QUERY ID <query id>`, which names
one statement. Could a MariaDB cancel be made so it can never stop the next
command, so the wait can go there?

## Recommendation

Drop `KILL QUERY ID` for now. It can be made *almost* safe, but it does not
reach "never" (see the dispatch gap below). It needs a tag on every statement
the session sends. It cannot cancel a stored routine's body. It silently
cancels nothing behind MaxScale or ProxySQL. And it saves at most one round
trip, after some cancels, on MariaDB only.

If the wait is worth removing, there is a narrower fix that works on every
engine: redo once an app read that comes back `Cancelled` when no cancel was
sent for it. Even a `KILL QUERY ID` design would need that backstop to be
airtight. Details are under "Alternatives".

## What MariaDB does

Read in MariaDB 10.11's source (`sql/sql_parse.cc`, `sp_head.cc`,
`sql_prepare.cc`, `sql_show.cc`), not only in the docs.

- **The target.** `KILL QUERY ID q` walks the server's threads and takes the
  *first* one whose current `query_id` is `q` (`find_thread_callback`). Killing
  a thread of the same user needs no privilege (`priv_user_matches`), and the
  spare signs in as the session's user. An ID that matches no thread gives
  error 1957, "Unknown query id".
- **The ID stays after the statement ends.** A thread keeps its last
  `query_id` while it waits idle. So a kill that comes just after the statement
  ended still finds the thread. That is harmless: `dispatch_command` calls
  `reset_kill_query()` before every command, so a kill that landed on an idle
  thread is cleared. (#307 found the same for MySQL 8.4 by testing.)
- **The dispatch gap.** In `dispatch_command`, `reset_kill_query()` runs a few
  instructions *before* `set_query_id(next_query_id())`, and no lock covers
  both. A kill that finds the thread in between still sees the old ID. It then
  sets the flag on the command that is just starting, which fails with
  "interrupted". The window is nanoseconds wide, and only a kill that was
  already late can hit it. But "never" does not hold.
- **Stored routines and triggers.** Each statement inside a routine gets a new
  `query_id` (`sp_lex_keeper::reset_lex_and_exec_core`). `PROCESSLIST.INFO`
  then shows that inner statement's text (`sp_instr_stmt::execute`). The
  caller's ID comes back only once the routine returns. While a `CALL`'s body,
  a stored function or a trigger runs, neither the outer ID nor the outer text
  can be seen.
- **Shared IDs.** `COM_PING` and `COM_STATISTICS` take the server's latest
  query ID without a new one (`CF_SKIP_QUERY_ID`). Another thread can briefly
  hold the same ID as ours. The kill then takes the first match, which can be
  that thread: the kill is refused and ours is not stopped. This is rare.
- **No other way to learn the ID.** MariaDB has no variable or function for
  the current statement's query ID. Session tracking has types for system
  variables, the schema, state change, GTIDs and the transaction, but none for
  the query ID (`SessionStateType` in mysql_common 0.37.3). The OK packet does
  not carry it either. mysql_async 0.37 exposes the thread ID, affected rows,
  `info()` and session state changes, and nothing more. The only place to read
  the ID is `information_schema.PROCESSLIST.QUERY_ID`. The ID counter is global
  to the server, so a session cannot predict the ID of its next statement.
- **One message.** `KILL` takes an expression but refuses a subquery
  (`ER_SUBQUERIES_NOT_SUPPORTED`). A user variable does work, so the lookup and
  the kill can go as one message on the spare. That keeps a cancel at one round
  trip:

  ```sql
  SET @q := (SELECT QUERY_ID FROM information_schema.PROCESSLIST
             WHERE ID = 123 AND INFO LIKE '/* tabletist 1 */%' LIMIT 1);
  KILL QUERY ID COALESCE(@q, -1)
  ```

  `-1` matches no thread, so a lookup that finds nothing gives 1957. A `NULL`
  would be read as ID 0, which an idle new thread can have.
- **Versions.** `KILL QUERY ID` and `PROCESSLIST.QUERY_ID` came in 10.0.5. The
  SQL editor already needs 10.2.4. MySQL has neither, so MySQL keeps the wait
  whatever is chosen.

## The idea, evaluated

**Matching by the statement's text does not work.** If two commands in a row
send the same text, the text cannot tell them apart. A prepared statement's
`INFO` is its prepared text, and it is expanded with the values when a log is
on (`expanded_query`). Use a tag the app puts in front of every statement
instead, like `/* tabletist 1 */`. The tag survives in `INFO` for plain and
prepared statements alike, and it needs no shared slot.

**A request number in the tag breaks the statement cache.** mysql_async caches
prepared statements by their text (32 by default). If every request has its
own text, every catalog query is prepared again, which is one round trip each.
Those are the round trips #304 and #305 just removed. The fix is to alternate
two tags, 0 and 1, from one command to the next. The next command then never
shares a tag with the command before it. A command waits only for an
outstanding cancel with its own tag, which belongs to the command two back. The
cache holds both forms of each statement.

**Scripts.** Every statement of a run carries the run's tag. A cancel stops
whichever of them is running, as today, and `keep_cancelling` stays as it is.
The cleanup (the rollback and the session reset) can be sent without the tag,
so a cancel never interrupts it. `retry_cancelled!` stays as a backstop.

**Stored routines break it.** While a `CALL`'s body runs, the tag is not in
`INFO`, and the lookup finds nothing. A `CALL` in the SQL editor could not be
cancelled. `keep_cancelling` sends again and again, but each cancel misses
while the body runs. The same goes for a page of a view that calls a stored
function, and for a save that fires a trigger. Scripts would have to keep
`KILL QUERY <thread>` and the wait. Only the app's own reads would gain.

**Proxies.** Behind MaxScale or ProxySQL, the thread ID from the handshake is
the proxy's, and `PROCESSLIST` shows the backend's IDs. The lookup would never
match, so a cancel would do nothing and say nothing. The fix is to compare
`CONNECTION_ID()` with the handshake ID at connect, in the identity question
that is already sent, so it costs no round trip, and to fall back when the two
differ.

**Size.** About 24 places send statements (`mysql.rs`, `write.rs`,
`hidden.rs`, `script.rs`). Each would go through a wrapper that adds the tag.
The cancel handle would carry the tag, and `Running` would record each
command's tag. The editor's statements get the prefix too. As long as the tag
has no newline, the line numbers in MariaDB's error messages stay right.

## What the wait costs today

The wait lasts until the `KILL` answers, which on the spare is one round trip.

- If the kill stops the read, the read's answer and the kill's answer arrive at
  about the same time. The wait costs close to nothing.
- If the read ended by itself just before the kill landed, the wait costs up to
  one round trip.
- If the spare had to be opened again (after `wait_timeout`, or a lost
  connection), the cancel costs a sign-in, the server check and the kill:
  several round trips. This is the costly case, and it is also when a late kill
  is most likely.

#306 already sends no cancel to a read that ends within its grace. So there are
fewer cancels to wait for at all.

## Alternatives

1. **Keep the spare from going stale.** When the spare opens, send
   `SET SESSION wait_timeout = CAST(GREATEST(@@session.wait_timeout, 28800) AS UNSIGNED)`
   in the same message as its server check. It raises the timeout to MySQL's
   default of eight hours and never lowers it. No privilege is needed for the
   session's own setting, and it costs no round trip. This makes the costly
   case above rare, on MySQL and MariaDB alike. MySQL 8.4 refuses
   `GREATEST(...)` without the cast (error 1232). A server that refuses the
   setting still gets its spare: the check is sent again without it.
2. **Redo a read that a stray cancel stopped.** Drop the wait only before the
   app's own reads: a page, a structure, a definition, a lookup, a count. If
   such a read comes back `Cancelled` and no cancel was sent for it, run it
   once more. A read is safe to redo, and the stray cancel is spent once it
   lands. Scripts and saves keep the wait: they are long, and rarely come right
   after a cancel. This is `retry_cancelled!` applied to a whole read. It works
   on MySQL, MariaDB and PostgreSQL (where #307's longer wait could then go for
   reads too), and needs no tags. Now and then a read runs twice: its work so
   far and one round trip are lost.
3. **`KILL QUERY ID`, as above.** Only on MariaDB, only for the app's own
   reads, with the two alternating tags, with a fallback for proxies and
   scripts, and still with 2 as a backstop. It would only make the redos of 2
   rarer.

## Testing

- **Alternatives 1 and 2** can be tested on the MySQL 8.4 in the compose file.
  - For 1: close the spare's idle timeout and check that it is still open
    afterwards.
  - For 2: put the counting proxy from #305 in front of the spare and hold
    back its bytes, as #307's PostgreSQL test does. The `KILL` then lands while
    the next read runs, and that read must still come back with its rows.
- **Alternative 3 needs a MariaDB server.** Add `mariadb:11.4` (LTS) to
  `compose.yaml` on `127.0.0.1:53307`, with `TABLETIST_TEST_MARIADB_URL`. Add a
  CI job that names its test files (`ci.yml` runs only the files it lists).
  Tests:
  - a held-back cancel lands during the next query and does not stop it;
  - a `CALL` whose body sleeps (this shows the miss, and checks the fallback);
  - two commands in a row with the same text;
  - the proxy check, as a unit test.
- **A MariaDB service is worth adding anyway.** The MariaDB paths are untested
  today: `server_identity`'s fallback by host and port, and `can_reset`'s
  10.2.4 check. #308 says so. That is a separate small task.

## Decision

Decided on 2026-11-11:

- **Drop** `KILL QUERY ID`;
- do **1** now, on top of the spare branch;
- do **2** only if measurements still show the wait after #306 and 1.
