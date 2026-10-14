# MCP server, Phase 1b, run 1: the server, with nothing to start it. Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A new crate, `tabletist-mcp`, serves an agent over whatever transport it is handed: three tools that only read (`list_connections`, `describe_schema`, `run_query`), an answer kept under a size, refusals with codes that do not change, and a log of every call. Nothing in the app starts it.

**Architecture:** The crate reaches a database only through a `Gateway` trait, which lists the connections an agent may name and opens one by its id, so none of its types can hold a host or a password. `tools` speaks the crate's own types, `protocol` alone names the SDK's, and `server` keeps the sessions, runs each call under its stops and writes it to `log`, a SQLite file. The tests call the server as an agent does, through the SDK's client over a transport in the same process, with a gateway over SQLite files of the Bookshop.

**Tech Stack:** Rust 2024. `rmcp` 3.5 (new), `rusqlite` 0.37, `tokio`, `tokio-util`, `serde_json`, `uuid`. With `rmcp` eight crates are new to `Cargo.lock`: `rmcp`, `schemars`, `schemars_derive`, `serde_derive_internals`, `pastey`, `dyn-clone`, `ref-cast`, `ref-cast-impl`. The tests add `tokio-stream`, through the SDK's client.

---

## Where this sits

This is the first of the five runs of Phase 1b of `docs/superpowers/plans/2026-10-13-mcp-server.md` (section G). Phase 1a, the enforcement core in `tabletist-db`, is merged: the check on a statement, the read that runs it, and SQLite's own fence.

1. **This run: the server, with nothing to start it.** The crate, the gateway's trait, the three tools, the size cap, the refusals, the log's store.
2. **Turning it on from the files, and stdio.** The settings, `[connections.agents]`, the backend's gateway, the relay.
3. **The Agents tab's frame.**
4. **What agents are doing.**
5. **Letting agents in.**

After this run no setting, no transport and no command reaches the server. The app's binary does not link the crate: the root package gets no dependency on it until the second run. So this run can be released with nothing changed for a user.

## Before you start

- Work on a branch of `main`. One commit for each task, in the order below. Every commit passes `cargo fmt --all --check`, `cargo clippy --locked --workspace --all-targets -- -D warnings` and `cargo test --locked --workspace --all-targets`.
- On Igor's machine `cargo` is `~/.cargo/bin/cargo` (the mise shim fails), and a build in a worktree needs `CARGO_TARGET_DIR` set to that worktree's `target/`.
- Read `AGENTS.md`. Four of its rules shape this code: a `match` on `Dialect` names every variant; no `unsafe`; every crate in a `Cargo.toml` is explained; and there are no em dashes in comments or messages.
- The diffs below apply with `git apply`. Run `cargo fmt --all` before each commit: the code here is already formatted, so it should change nothing.
- **`Cargo.lock` is in the diffs** (Tasks 2, 3 and 7). Do not let cargo write it: a resolve from scratch also moves six `windows-sys` pins that have nothing to do with this (seen again while this plan was drafted). Every command here has `--locked`. If a build without it rewrote the file, `git checkout Cargo.lock` and apply the task's diff again.
- **No server is needed.** Every test of the new crate runs on SQLite. The one change to `tabletist-db` (Task 1) is a sentence, tested without a database.
- **Some tests wait on the real clock.** A statement that is ended after half a second, a server's timer two seconds behind it, the SDK's five seconds for a client that left, a lock that is waited out. From Task 5 on, `tests/stops.rs` takes about five seconds and `tests/tools.rs` about three. The crate's own tests take two, from Task 2 on.

## What was run before this plan was written

Every task's state was built, passed `cargo fmt --check` and clippy with `-D warnings`, and its tests failed and then passed as each step says. The last state passed `cargo test --locked --workspace --all-targets` with PostgreSQL 17.11 and MySQL 8.4.11 up (3,342 tests, none failed), and `RUSTDOCFLAGS='-D warnings' cargo doc --locked --workspace --no-deps`. Not run:

- **macOS and Windows.** The CI matrix runs the new crate's tests there. One thing in it is specific to a platform: the log's file is given mode 0600 on Unix. On Windows the state directory is in the user's profile, and nothing more is done.
- **A real agent.** No Claude Code, no Cursor: nothing listens yet. The tests speak to the server through the SDK's own client, by both of the protocol's lifecycles (the handshake of 2025-11-25, and 2026-07-28, which has none), and for what that client would never send, by lines of JSON written by hand.
- **PostgreSQL and MySQL through the server.** The tools were run on SQLite. What a read does on the other two is Phase 1a's (`crates/tabletist-db/tests/agent.rs`), and they meet the server with the backend's gateway, in the second run.

## What this run settles

The master plan names the tools and says what must hold. It does not say what a tool takes and answers. This run does.

| Question | Decision | Why |
|---|---|---|
| Where the line to the databases runs | `Gateway::connections()` lists what an agent may use, and `Gateway::open(id, stop)` gives a `Session` for one call: `schemas`, `objects`, `describe`, `read`. The trait is here. What implements it for the app is the backend's, in the second run | The master plan (A.1) has the gateway "implemented in the backend beside `Worker`", where the keyring and the code that stops a statement are. A session held for one call is how "one call at a time" and `busy` are the gateway's to keep, and how `describe_schema` reads a catalog in several steps without another agent's call between them |
| What `list_connections` answers | For each: `name`, `engine`, `parameters`, `access`, `rows`, `timeout_s` | The board has the name, the access and the limits. `engine` ("PostgreSQL", "MySQL", "SQLite") and `parameters` (`$1, $2` or `?`) are added: an agent cannot write a statement the database reads without them. `hidden` and `masked` come with Phases 1c and 3a. `access` is always `read` until Phase 2 |
| What `describe_schema` answers | Without `tables`: the names of the tables and views of the connection's own schema, each with its kind and a rough count of rows, and the names of the other schemas. With `tables` (at most 20, each of at most 1,000 bytes): each one's columns, primary key, foreign keys, the keys that point at it, and its indexes. A name that is no table is listed under `not_found`, and the rest are described | A database can have thousands of tables, and a description of each is several catalog queries. Two steps keep both the answer and the work bounded. The board's signature is kept: `(connection, tables?)` |
| Which schema is a connection's own | The gateway says (`Schemas::home`): the one the app opens first for that connection. A table elsewhere is written `schema.table` | The rule is the app's (`App::apply`, on `Event::Schemas`), and needs the connection's database name, which this crate does not have |
| What `run_query` answers | `columns` (each a `name` and the database's `type`), `rows` (arrays, in the columns' order), `row_count`, `truncated`, `truncated_by` (`row_limit` or `size`) when it is, `took_ms` | Rows as arrays: a column's name is written once. `truncated_by` is B.8's |
| "N of M" | Not in this run. An answer that was cut says so and does not say of how many | The count is a second statement in the read's own transaction (B.5), which is a change to the three drivers. It is a run of its own in `tabletist-db`, before the Agents tab shows the number (the fourth run) |
| A value that is not sent whole | A text longer than 2,000 characters is `{"cut": <its first 2,000>, "characters": <its length>}`. A binary value is `{"binary_bytes": <its size>}`. A column's default and its comment are cut the same way | A database's value is null, a boolean, a number or a text, so an object in a row can only be one of these two. A mark inside the text could be taken for data |
| The size of an answer | At most 256,000 bytes of JSON. Past that, rows are left out from the end, and `truncated_by` is `size`. The columns count too: an answer whose columns alone pass the size is refused (`too_large`). `describe_schema` is held to the same size, the names it was asked for among it, and names the tables that did not fit under `left_out` | B.5. Two hundred columns named by two thousand letters each are 400 kB without a row, from a statement of under three kilobytes. The draft's reviewer got 3.8 MB from one of six |
| An answer is sent once | As its JSON, in the result's text. No structured content beside it, and no output schema | A client that hands a model both the text and the structured result would hand it the rows twice, and the transport carried three times the size in the draft. A refusal has both (B.8): it is small |
| A refusal | A tool's result with `isError`, the sentence as its text, and `structuredContent` of `{"refused": <code>, "message": <sentence>}`. A sentence is at most 2,000 characters | B.8: a model reads a tool's result, and may never see a protocol error. A database writes an error as long as it likes: SQLite repeated a JSON path of 600,000 letters |
| What is the protocol's error | A tool that does not exist, an argument that is missing or of the wrong type, and an argument the tool does not have: `-32602`. Its sentence never repeats what the agent sent | B.8. The SDK traces every protocol error (see "What the SDK writes") |
| An argument the tool does not have | Refused, not ignored | An agent that sends `"limit": 10` and gets every row has learnt nothing. One that is told "run_query takes connection, sql, params and nothing else" has |
| A value for a parameter | A JSON string, number, `true`, `false` or `null`. A whole number past 64 bits signed is refused (`invalid_params`), with "Send it as a string" | No driver here binds an unsigned 64-bit integer, and as a float it would be another number |
| The check's sentence for a write | "run_query only runs reads, and this is a DELETE." Phase 1a's went on: "Use propose_write to change data." | There is no such tool until Phase 2. Which tool changes data is for the server that has the tools to say, so the sentence leaves `tabletist-db` |
| How a call ends that does not end by itself | The database's own timeout first (Phase 1a). Two seconds behind it, the server's timer ends the call through the gateway. A client's cancel and a person's stop end it the same way. The read's future is awaited to its end in every case | A.5: "awaited to its end, never dropped" |
| A call that was stopped by the person | Answers `server_stopped`, whatever it had got to and whatever else ended it besides | An agent that was stopped is given nothing more. In the draft a call stopped with the whole server answered `cancelled`, because the SDK cancels a call too when its session closes: the person's stop now wins from wherever it was given |
| A client that closes its end | The SDK goes on answering for five seconds, and cancels nothing. When it has let the transport go, the server ends what still runs. A write that fails ends the session's calls at once | A client that sent its calls and closed its writing end waits for its answers, and gets them. One that is gone leaves a statement running for five seconds at most, not for minutes |
| A client that reads nothing | A session that was stopped is closed all the same: its calls are given five seconds to answer, and three seconds after that the server takes the transport from the SDK | The SDK's close waits on a write that never ends. Seen in the draft: the session stayed open for good |
| What a session may take | A message of at most a megabyte: a longer one ends the session. Sixteen calls at once: the next is refused (`busy`) | The SDK reads a line into memory however long it is (200 MB in the draft), and runs every call it is sent |
| A session | An id of its own (a UUID), the agent's label, the project, when it connected, and its count of calls | The label is what the client calls itself: its title, or its name where the title is nothing a person could read. Without characters that move the cursor, turn the line around or cannot be seen, and at most 64 characters. It is no identity (A.5) |
| The project | Given by whoever hands the transport over (`Origin::project`) | The relay's directory, in the second run. Asking the client for its roots is left for later |
| The log's file | The whole table of C.1, the columns of an approval among them, from the first version | So that Phase 2 changes no file |
| What is an entry | Every call a tool answered or refused. Not a protocol error | "For calls that never reach a tool" (B.8) |
| The values in the log | In full, as the JSON they came in, up to 100 kB: past that the text is cut, and is JSON no more | Open question 18 is not answered, and the plan has them in full. A statement and the names of tables are kept up to 100 kB too |
| A database's error in the log | Its code, and not its words: "The database refused the statement (code 42703)." The agent's sentence has the words | A database quotes what it could not read, and that can be a row's value. The log holds no row (C.1). The master plan had the database's message in the log: corrected there |
| A log that cannot be written | The call is answered all the same, and the app's log gets an error | A full disk must not turn every read into an error. Phase 2 decides this again for a write, which must be in the log before it runs |
| A log that cannot be opened | A file that is no database, a damaged one or one of another schema is kept aside, and a new one is made. A file that could not be looked into (it is locked, the disk failed) is an error, and stays where it is | A log that is only busy must not be taken for a broken one. What the server does without its log is the second run's to say |
| The app's own log | One line for each call: the tool, the result, the code, the time | C.1 |

### What the SDK writes

`rmcp` traces through the `tracing` crate, and the app is built with that crate's `log` feature on (`calloop` turns it on, on Linux), which sends what is traced to the app's log. What the SDK traces is what a client sent:

- at `info`, what a client calls itself, every notification whole (a cancel's reason, a progress message), and each cancel;
- at `warn`, every protocol error with the request's id, which is the client's to choose, and for an unknown method its name;
- at `debug`, every request whole: the statement and its values.

`sqlparser`, which the check reads a statement with, writes each word of it at `debug`. And `tracing` writes lines of its own for the SDK's spans.

So the app's log takes no line of the three. Nothing in the app runs the server yet, so nothing is written today. The second run, which links the server into the app, turns them off in the app's filter (`src/entrypoint.rs`) before it does, with a test of `tabletist.log` after a session. Here, `tests/secrets.rs` builds `tracing` as the app does, catches every line any crate writes during a session, holds every crate's but those three to "nothing an agent sent", and shows what the SDK's hold. `AGENTS.md` gets the rule.

### What this run leaves out

- **The backend's gateway**: the sessions of its own on each database, the keyring, `busy`, the ten idle minutes. The second run. The tests here have a gateway over SQLite files.
- **Settings and files**: `[mcp]`, `[connections.agents]`. The second run. A connection's rows and timeout come to this crate through `Listed::limits`.
- **The app's log filter** for `rmcp`, `tracing` and `sqlparser`. The second run, as above.
- **The name an agent sees for a connection** (open question 19). The gateway gives it, and gives no two connections the same: the second run. The master plan had the question met in this run.
- **Telling the app**: that a session opened, that a call began and ended. The second and fourth runs, with what receives it. `Server::open_sessions` is what there is.
- **A session's connections and its state** (`connections`, `state`, `last_call_at` of A.5). The fourth run, which shows them.
- **The log's schedule and its readers**: retention every six hours (the second run, with what starts the server), the filters, the search and the export (the fourth). `Log::forget_before` and `Log::latest` are what there is.
- **The rate limit, the approver, a clock a test controls.** Phase 2: they are `propose_write`'s.
- **Hidden tables, masked columns, system catalogs.** Phases 1c and 3a.
- **A client that floods and reads nothing.** Its calls past the sixteenth are refused at once, and each refusal waits in the SDK to be sent: a few hundred bytes a call, until the session is stopped or the client goes. The SDK reads a transport as fast as it comes and gives no way to hold it back. This is an agent the person started, on their own machine.

### What does not change

The app. No file under `src/` changes, and the root package gets no new dependency. In `tabletist-db`, one sentence, in the four places its test expects it.

## File structure

| File | What it is |
|---|---|
| `crates/tabletist-db/src/agent.rs` | The sentence for a write no longer names a tool |
| `Cargo.toml`, `Cargo.lock` | The crate joins the workspace, with what it brings |
| `crates/tabletist-mcp/Cargo.toml` | New. Each dependency with its reason |
| `crates/tabletist-mcp/src/lib.rs` | New. What the crate is, and what it exports |
| `…/src/log.rs` | New. The log of calls: the file, an entry, retention |
| `…/src/gateway.rs` | New. `Gateway`, `Session`, `Listed`, `Schemas`, `Failure` |
| `…/src/stop.rs` | New. A stop, and why |
| `…/src/refusal.rs` | New. `Refused`, the codes, and what a failure is to an agent |
| `…/src/tools.rs` | New. A tool as an agent is told of it, a call's arguments, and what the tools share |
| `…/src/tools/connections.rs`, `query.rs`, `schema.rs` | New. One tool each |
| `…/src/answer.rs` | New. A read's answer as JSON, under its size |
| `…/src/server.rs` | New. `Server`, the sessions, a call's way into the log |
| `…/src/protocol.rs` | New. The SDK: the only module that names its types. And the transport as the SDK is given it |
| `…/tests/support/mod.rs` | New. A gateway over SQLite files, an agent, and an agent that writes the protocol's lines itself |
| `…/tests/tools.rs`, `sessions.rs`, `stops.rs` | New. The tools, a session, and how a call ends (F.1, in process) |
| `…/tests/log.rs`, `secrets.rs` | New. F.4 and F.5 |
| `tests/engines.rs` | The new crate's sources are read too, for a `Dialect` or a `Driver` that is compared and not matched |
| `AGENTS.md` | The new crate's rules |
| `docs/superpowers/plans/2026-10-13-mcp-server.md` | What this run settled, where the master plan left it open or had it otherwise |

`tools.rs` with `tools/` is as `agent.rs` with `agent/` in `tabletist-db`.

---

### Task 1: The check stops naming a tool it does not know

Phase 1a's sentence for a write ends "Use propose_write to change data." There is no such tool until Phase 2, and an agent that is told of one will call it. Which tool changes data, if any does, is for the server that has the tools to say. The sentence ends at what the statement is.

**Files:**
- Modify: `crates/tabletist-db/src/agent.rs`

- [ ] **Step 1: Write the failing test**

````diff
diff --git a/crates/tabletist-db/src/agent.rs b/crates/tabletist-db/src/agent.rs
index 537e700..1acf42b 100644
--- a/crates/tabletist-db/src/agent.rs
+++ b/crates/tabletist-db/src/agent.rs
@@ -933,23 +933,19 @@ mod tests {
             ),
             (
                 Refusal::NotReadOnly(Writes::Insert),
-                "run_query only runs reads, and this is an INSERT. Use propose_write to change \
-                 data.",
+                "run_query only runs reads, and this is an INSERT.",
             ),
             (
                 Refusal::NotReadOnly(Writes::Update),
-                "run_query only runs reads, and this is an UPDATE. Use propose_write to change \
-                 data.",
+                "run_query only runs reads, and this is an UPDATE.",
             ),
             (
                 Refusal::NotReadOnly(Writes::Delete),
-                "run_query only runs reads, and this is a DELETE. Use propose_write to change \
-                 data.",
+                "run_query only runs reads, and this is a DELETE.",
             ),
             (
                 Refusal::NotReadOnly(Writes::Merge),
-                "run_query only runs reads, and this is a MERGE. Use propose_write to change \
-                 data.",
+                "run_query only runs reads, and this is a MERGE.",
             ),
             (
                 Refusal::NotReadOnly(Writes::Into),
````

- [ ] **Step 2: Run it to see it fail**

Run: `cargo test --locked -p tabletist-db --lib agent::tests::a_refusal_reads`

Expected: FAIL. The sentence still names the tool:

```text
test agent::tests::a_refusal_reads_as_a_sentence_for_the_agent ... FAILED
  left: "run_query only runs reads, and this is an INSERT. Use propose_write to change data."
 right: "run_query only runs reads, and this is an INSERT."
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 466 filtered out
```

- [ ] **Step 3: End the sentence there**

````diff
diff --git a/crates/tabletist-db/src/agent.rs b/crates/tabletist-db/src/agent.rs
index 1acf42b..f83c0ad 100644
--- a/crates/tabletist-db/src/agent.rs
+++ b/crates/tabletist-db/src/agent.rs
@@ -219,11 +219,9 @@ impl fmt::Display for Refusal {
                         );
                     }
                 };
-                write!(
-                    f,
-                    "run_query only runs reads, and this is {what}. Use propose_write to change \
-                     data."
-                )
+                // No more than that: which tool changes data, if any does, is
+                // for the server that has the tools to say.
+                write!(f, "run_query only runs reads, and this is {what}.")
             }
             Self::NotAllowed(what) | Self::FunctionNotAllowed(what) => {
                 write!(f, "{what} is not available to agents.")
@@ -439,8 +437,8 @@ fn plans_only(option: &UtilityOption) -> bool {
     )
 }
 
-/// Why a statement that is not a query is refused: a write is told about
-/// `propose_write`, everything else is named by its first word.
+/// Why a statement that is not a query is refused: a write is told what it
+/// is, everything else is named by its first word.
 fn not_a_read(dialect: Dialect, text: &str, statement: &Statement) -> Refusal {
     match statement {
         Statement::Insert(_) => Refusal::NotReadOnly(Writes::Insert),
````

- [ ] **Step 4: Run the check's tests**

Run: `cargo test --locked -p tabletist-db --lib agent::`

Expected: PASS.

```text
test result: ok. 45 passed; 0 failed; 0 ignored; 0 measured; 422 filtered out
```

- [ ] **Step 5: Commit**

```bash
cargo fmt --all
cargo clippy --locked --workspace --all-targets -- -D warnings
git add crates/tabletist-db/src/agent.rs
git commit -m "Leave the write tool out of the check's sentence, which knows no tools"
```

---

### Task 2: The crate, and the log of calls

The crate starts with the one part of it that stands alone: the log. It is a SQLite file through `rusqlite`, which `tabletist-db` already builds, so this task adds no crate to the tree.

`Log::open` makes the file, private to the user, with the whole table of the master plan's C.1: the columns an approval fills are there from the first version, so that Phase 2 changes no file. A file that is not a log this version reads is kept aside as `<name>.bad`, as `util::keep_aside` keeps a JSON file, and a new one is made. The rename is written again here: the app's helper is in the root crate, which this crate must not depend on. A file that could not be looked into is another thing: it may be a good log that another program holds a lock on, so it is an error and stays where it is.

`record`, `latest` and `forget_before` run on the runtime's blocking pool. `forget_before` is retention's one statement. What calls it every six hours comes with what starts the server.

**Files:**
- Create: `crates/tabletist-mcp/Cargo.toml`, `crates/tabletist-mcp/src/lib.rs`, `crates/tabletist-mcp/src/log.rs`
- Modify: `Cargo.toml` (the workspace's members), `Cargo.lock`

- [ ] **Step 1: The crate, with the log's tests and no log**

The file has its tests and nothing else yet. `Cargo.lock` gains the crate's own entry, by hand: it names crates that are all in the tree.

````diff
diff --git a/Cargo.lock b/Cargo.lock
index aa83ab9..c99ac52 100644
--- a/Cargo.lock
+++ b/Cargo.lock
@@ -5651,6 +5651,15 @@ dependencies = [
  "url",
 ]
 
+[[package]]
+name = "tabletist-mcp"
+version = "0.1.0"
+dependencies = [
+ "rusqlite",
+ "tempfile",
+ "tokio",
+]
+
 [[package]]
 name = "tabletist-sqlite-ffi"
 version = "0.1.0"
diff --git a/Cargo.toml b/Cargo.toml
index 7dc1046..bc1f517 100644
--- a/Cargo.toml
+++ b/Cargo.toml
@@ -1,5 +1,5 @@
 [workspace]
-members = [".", "crates/tabletist-appkit", "crates/tabletist-db", "crates/tabletist-sqlite-ffi"]
+members = [".", "crates/tabletist-appkit", "crates/tabletist-db", "crates/tabletist-mcp", "crates/tabletist-sqlite-ffi"]
 resolver = "3"
 
 [workspace.package]
diff --git a/crates/tabletist-mcp/Cargo.toml b/crates/tabletist-mcp/Cargo.toml
new file mode 100644
index 0000000..ac3c761
--- /dev/null
+++ b/crates/tabletist-mcp/Cargo.toml
@@ -0,0 +1,23 @@
+[package]
+name = "tabletist-mcp"
+version = "0.1.0"
+description = "The MCP server through which agents use Tabletist's connections"
+edition.workspace = true
+rust-version.workspace = true
+publish = false
+
+[lints]
+workspace = true
+
+[dependencies]
+# The log is written on the runtime's blocking pool.
+tokio = { version = "1", features = ["rt"] }
+# The log of calls, a SQLite file. The crate and its bundled SQLite that
+# tabletist-db builds: nothing new.
+rusqlite = { version = "0.37", features = ["bundled"] }
+
+[dev-dependencies]
+# The tests' runtime.
+tokio = { version = "1", features = ["rt", "macros"] }
+# A directory of its own for each test's files.
+tempfile = "3"
diff --git a/crates/tabletist-mcp/src/lib.rs b/crates/tabletist-mcp/src/lib.rs
new file mode 100644
index 0000000..6734bcc
--- /dev/null
+++ b/crates/tabletist-mcp/src/lib.rs
@@ -0,0 +1,7 @@
+//! The MCP server through which agents use Tabletist's connections.
+//!
+//! So far it is the [`Log`] of calls, and nothing that makes a call.
+
+mod log;
+
+pub use log::{Entry, Log, LogError, Opened};
diff --git a/crates/tabletist-mcp/src/log.rs b/crates/tabletist-mcp/src/log.rs
new file mode 100644
index 0000000..031fff2
--- /dev/null
+++ b/crates/tabletist-mcp/src/log.rs
@@ -0,0 +1,199 @@
+//! The log of calls: one entry for each call a tool answered or refused,
+//! in a SQLite file of the person's own. It holds what was asked and what
+//! came of it, and never a row's value.
+//!
+//! The file is opened once and written from the runtime's blocking pool.
+
+#[cfg(test)]
+mod tests {
+    use super::*;
+
+    fn entry(at_ms: i64, tool: &str) -> Entry {
+        Entry {
+            at_ms,
+            session: "s1".into(),
+            agent: "Claude Code".into(),
+            transport: "stdio".into(),
+            tool: tool.into(),
+            result: "ok".into(),
+            ..Entry::default()
+        }
+    }
+
+    fn open(path: &Path) -> Log {
+        let opened = Log::open(path).unwrap();
+        assert_eq!(opened.kept_aside, None);
+        opened.log
+    }
+
+    #[tokio::test]
+    async fn an_entry_is_read_back_as_it_was_written() {
+        let dir = tempfile::tempdir().unwrap();
+        let log = open(&dir.path().join("agents.sqlite3"));
+        let whole = Entry {
+            at_ms: 1_791_000_000_000,
+            session: "5f0c".into(),
+            agent: "Claude Code".into(),
+            project: Some("tabletist-repo".into()),
+            transport: "stdio".into(),
+            connection_id: Some("b7e1".into()),
+            connection: Some("Bookshop · dev".into()),
+            tool: "run_query".into(),
+            sql: Some("SELECT id FROM book_covers WHERE publisher_id = ?".into()),
+            params: Some("[9100000000000000004]".into()),
+            request: None,
+            result: "ok".into(),
+            code: None,
+            message: None,
+            ran_as: Some("read-only transaction".into()),
+            rows: Some(3),
+            matched: Some(3),
+            columns: Some(1),
+            bytes: Some(96),
+            took_ms: Some(4),
+        };
+        log.record(whole.clone()).await.unwrap();
+        log.record(entry(1_791_000_000_500, "list_connections"))
+            .await
+            .unwrap();
+        let read = log.latest(10).await.unwrap();
+        assert_eq!(read.len(), 2);
+        assert_eq!(read[0].tool, "list_connections", "the newest first");
+        assert_eq!(read[1], whole);
+        assert_eq!(log.latest(1).await.unwrap().len(), 1);
+    }
+
+    #[tokio::test]
+    async fn the_log_is_there_again_when_it_is_opened_again() {
+        let dir = tempfile::tempdir().unwrap();
+        let path = dir.path().join("agents.sqlite3");
+        open(&path).record(entry(1, "run_query")).await.unwrap();
+        let again = open(&path);
+        again.record(entry(2, "run_query")).await.unwrap();
+        assert_eq!(again.latest(10).await.unwrap().len(), 2);
+    }
+
+    #[cfg(unix)]
+    #[tokio::test]
+    async fn the_file_is_the_users_alone() {
+        use std::os::unix::fs::PermissionsExt;
+        let dir = tempfile::tempdir().unwrap();
+        let path = dir.path().join("agents.sqlite3");
+        open(&path).record(entry(1, "run_query")).await.unwrap();
+        let mode = std::fs::metadata(&path).unwrap().permissions().mode();
+        assert_eq!(mode & 0o777, 0o600);
+    }
+
+    #[cfg(unix)]
+    #[tokio::test]
+    async fn a_file_that_was_there_becomes_the_users_alone() {
+        use std::os::unix::fs::PermissionsExt;
+        let dir = tempfile::tempdir().unwrap();
+        let path = dir.path().join("agents.sqlite3");
+        std::fs::write(&path, "").unwrap();
+        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644)).unwrap();
+        open(&path).record(entry(1, "run_query")).await.unwrap();
+        let mode = std::fs::metadata(&path).unwrap().permissions().mode();
+        assert_eq!(mode & 0o777, 0o600);
+    }
+
+    #[tokio::test]
+    async fn a_log_that_is_only_locked_is_left_where_it_is() {
+        let dir = tempfile::tempdir().unwrap();
+        let path = dir.path().join("agents.sqlite3");
+        open(&path).record(entry(1, "run_query")).await.unwrap();
+        let other = rusqlite::Connection::open(&path).unwrap();
+        other.execute_batch("BEGIN EXCLUSIVE").unwrap();
+        let locked = Log::open(&path);
+        assert!(locked.is_err(), "{locked:?}");
+        assert!(!dir.path().join("agents.sqlite3.bad").exists());
+        other.execute_batch("ROLLBACK").unwrap();
+        // And it is the log it was.
+        assert_eq!(open(&path).latest(10).await.unwrap().len(), 1);
+    }
+
+    #[tokio::test]
+    async fn entries_older_than_the_day_asked_are_forgotten() {
+        let dir = tempfile::tempdir().unwrap();
+        let log = open(&dir.path().join("agents.sqlite3"));
+        for at_ms in [100, 200, 300] {
+            log.record(entry(at_ms, "run_query")).await.unwrap();
+        }
+        assert_eq!(log.forget_before(300).await.unwrap(), 2);
+        let left = log.latest(10).await.unwrap();
+        assert_eq!(left.len(), 1);
+        assert_eq!(left[0].at_ms, 300, "an entry of that very moment stays");
+        assert_eq!(log.forget_before(300).await.unwrap(), 0);
+    }
+
+    #[tokio::test]
+    async fn what_is_forgotten_gives_its_room_back() {
+        let dir = tempfile::tempdir().unwrap();
+        let path = dir.path().join("agents.sqlite3");
+        let log = open(&path);
+        let mut long = entry(1, "run_query");
+        long.sql = Some("x".repeat(50_000));
+        for _ in 0..20 {
+            log.record(long.clone()).await.unwrap();
+        }
+        let full = std::fs::metadata(&path).unwrap().len();
+        assert_eq!(log.forget_before(2).await.unwrap(), 20);
+        let emptied = std::fs::metadata(&path).unwrap().len();
+        assert!(emptied < full / 4, "{emptied} of {full}");
+    }
+
+    #[tokio::test]
+    async fn a_file_that_is_no_log_is_kept_aside_and_a_new_one_made() {
+        let dir = tempfile::tempdir().unwrap();
+        let path = dir.path().join("agents.sqlite3");
+        std::fs::write(&path, "this is not a database, and is longer than a header").unwrap();
+        let opened = Log::open(&path).unwrap();
+        let aside = dir.path().join("agents.sqlite3.bad");
+        assert_eq!(opened.kept_aside.as_deref(), Some(aside.as_path()));
+        assert!(
+            std::fs::read_to_string(&aside)
+                .unwrap()
+                .starts_with("this is not")
+        );
+        opened.log.record(entry(1, "run_query")).await.unwrap();
+        drop(opened);
+
+        // A second one does not replace the first.
+        std::fs::write(&path, "nor is this one, whatever it is that wrote it").unwrap();
+        let opened = Log::open(&path).unwrap();
+        assert_eq!(
+            opened.kept_aside,
+            Some(dir.path().join("agents.sqlite3.bad.1"))
+        );
+        assert!(
+            std::fs::read_to_string(&aside)
+                .unwrap()
+                .starts_with("this is not")
+        );
+    }
+
+    #[tokio::test]
+    async fn a_log_of_a_later_version_is_kept_aside_and_not_written_to() {
+        let dir = tempfile::tempdir().unwrap();
+        let path = dir.path().join("agents.sqlite3");
+        open(&path).record(entry(1, "run_query")).await.unwrap();
+        rusqlite::Connection::open(&path)
+            .unwrap()
+            .execute_batch("PRAGMA user_version = 2")
+            .unwrap();
+        let opened = Log::open(&path).unwrap();
+        assert!(opened.kept_aside.is_some());
+        assert_eq!(opened.log.latest(10).await.unwrap(), []);
+    }
+
+    #[tokio::test]
+    async fn a_database_of_something_else_is_kept_aside() {
+        let dir = tempfile::tempdir().unwrap();
+        let path = dir.path().join("agents.sqlite3");
+        rusqlite::Connection::open(&path)
+            .unwrap()
+            .execute_batch("CREATE TABLE notes (body TEXT)")
+            .unwrap();
+        assert!(Log::open(&path).unwrap().kept_aside.is_some());
+    }
+}
````

- [ ] **Step 2: Run them to see that they do not build**

Run: `cargo test --locked -p tabletist-mcp`

Expected: it does not build. The crate's root exports a log that is not written yet:

```text
error[E0432]: unresolved imports `log::Entry`, `log::Log`, `log::LogError`, `log::Opened`
error: could not compile `tabletist-mcp` (lib) due to 1 previous error
```
Cargo builds the crate and its tests side by side, so the same kind of error may follow from the tests' build (`lib test`): names that are not there yet.

If cargo says instead that the lock file needs to be updated, the diff's part for `Cargo.lock` was not applied.

- [ ] **Step 3: The log**

````diff
diff --git a/crates/tabletist-mcp/src/log.rs b/crates/tabletist-mcp/src/log.rs
index 031fff2..4e4a535 100644
--- a/crates/tabletist-mcp/src/log.rs
+++ b/crates/tabletist-mcp/src/log.rs
@@ -4,6 +4,386 @@
 //!
 //! The file is opened once and written from the runtime's blocking pool.
 
+use std::fmt;
+use std::path::{Path, PathBuf};
+use std::sync::{Arc, Mutex, PoisonError};
+use std::time::Duration;
+
+use rusqlite::{Connection, ErrorCode, params};
+
+/// The schema's version, in `PRAGMA user_version`.
+const VERSION: i64 = 1;
+
+/// How long a write waits for another connection's lock on the file. This
+/// process is the log's one writer, bar a second Tabletist that is open at
+/// the same time, and an entry is written in an instant.
+const LOCK_WAIT: Duration = Duration::from_secs(2);
+
+/// Every column the plan gives the log. Those a write's approval fills
+/// (`reason`, `note`, `decided_ms`) and the masked columns are here from
+/// the start, so that the file is not changed for them.
+const SCHEMA: &str = "
+CREATE TABLE calls (
+  id            INTEGER PRIMARY KEY,
+  at_ms         INTEGER NOT NULL,
+  session       TEXT    NOT NULL,
+  agent         TEXT    NOT NULL,
+  project       TEXT,
+  transport     TEXT    NOT NULL,
+  connection_id TEXT,
+  connection    TEXT,
+  tool          TEXT    NOT NULL,
+  sql           TEXT,
+  params        TEXT,
+  reason        TEXT,
+  request       TEXT,
+  result        TEXT    NOT NULL,
+  code          TEXT,
+  message       TEXT,
+  note          TEXT,
+  ran_as        TEXT,
+  rows          INTEGER,
+  matched       INTEGER,
+  columns       INTEGER,
+  bytes         INTEGER,
+  masked        TEXT,
+  took_ms       INTEGER,
+  decided_ms    INTEGER
+);
+CREATE INDEX calls_at         ON calls (at_ms);
+CREATE INDEX calls_agent      ON calls (agent, at_ms);
+CREATE INDEX calls_connection ON calls (connection_id, at_ms);
+CREATE INDEX calls_result     ON calls (result, at_ms);
+";
+
+/// One call.
+#[derive(Debug, Clone, PartialEq, Eq, Default)]
+pub struct Entry {
+    /// When the call arrived, in milliseconds of Unix time.
+    pub at_ms: i64,
+    pub session: String,
+    /// What the client calls itself. A label, not an identity.
+    pub agent: String,
+    pub project: Option<String>,
+    pub transport: String,
+    /// `None` for a call that named no connection, or none an agent may
+    /// see.
+    pub connection_id: Option<String>,
+    /// The connection's name as it was then.
+    pub connection: Option<String>,
+    pub tool: String,
+    pub sql: Option<String>,
+    /// The statement's values, as JSON.
+    pub params: Option<String>,
+    /// The tables `describe_schema` was asked for.
+    pub request: Option<String>,
+    /// `ok`, `refused`, `error`, `timeout`, `cancelled` or `stopped`.
+    pub result: String,
+    pub code: Option<String>,
+    /// The refusal's sentence. For a database's own error, its code and
+    /// not its words, which can quote a row.
+    pub message: Option<String>,
+    pub ran_as: Option<String>,
+    /// The rows the agent was sent.
+    pub rows: Option<i64>,
+    /// The statement's rows, where that is known.
+    pub matched: Option<i64>,
+    pub columns: Option<i64>,
+    /// The answer's size as JSON.
+    pub bytes: Option<i64>,
+    pub took_ms: Option<i64>,
+}
+
+/// Why the log could not be read or written. Its words name the file and
+/// SQLite's own error, never a call.
+#[derive(Debug, Clone, PartialEq, Eq)]
+pub struct LogError(String);
+
+impl fmt::Display for LogError {
+    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
+        f.write_str(&self.0)
+    }
+}
+
+impl std::error::Error for LogError {}
+
+impl From<rusqlite::Error> for LogError {
+    fn from(error: rusqlite::Error) -> Self {
+        Self(error.to_string())
+    }
+}
+
+/// The log, open. A clone writes to the same file.
+#[derive(Debug, Clone)]
+pub struct Log {
+    connection: Arc<Mutex<Connection>>,
+}
+
+/// Why a file was not opened as a log.
+enum Unread {
+    /// It is no log this version reads: not a database, a damaged one, or
+    /// one of another schema. It is kept aside.
+    NotALog(String),
+    /// It could not be told what the file is: it is locked, or cannot be
+    /// reached. It is left where it is. A log that is only busy must not
+    /// be taken for a broken one.
+    Failed(LogError),
+}
+
+impl From<rusqlite::Error> for Unread {
+    fn from(error: rusqlite::Error) -> Self {
+        let no_database = matches!(
+            &error,
+            rusqlite::Error::SqliteFailure(failure, _)
+                if matches!(failure.code, ErrorCode::NotADatabase | ErrorCode::DatabaseCorrupt)
+        );
+        match no_database {
+            true => Self::NotALog(error.to_string()),
+            false => Self::Failed(error.into()),
+        }
+    }
+}
+
+impl From<LogError> for Unread {
+    fn from(error: LogError) -> Self {
+        Self::Failed(error)
+    }
+}
+
+/// A log that was opened, and the name a file that could not be read was
+/// kept under, for whoever tells the person.
+#[derive(Debug)]
+pub struct Opened {
+    pub log: Log,
+    pub kept_aside: Option<PathBuf>,
+}
+
+impl Log {
+    /// Opens the log at `path`, making it when it is not there, private to
+    /// the user. A file that is not a log this version reads is kept
+    /// aside as `<name>.bad` and a new one is made: a log that cannot be
+    /// read must not keep the server from starting. A file that could not
+    /// be looked into (it is locked, or the disk failed) is an error, and
+    /// stays where it is.
+    ///
+    /// It blocks on the disk. A caller on a runtime uses `spawn_blocking`.
+    pub fn open(path: &Path) -> Result<Opened, LogError> {
+        match Self::open_as_it_is(path) {
+            Ok(log) => Ok(Opened {
+                log,
+                kept_aside: None,
+            }),
+            Err(Unread::Failed(error)) => Err(error),
+            Err(Unread::NotALog(problem)) => {
+                let aside = keep_aside(path)
+                    .map_err(|error| LogError(format!("{problem}, and {error}")))?;
+                match Self::open_as_it_is(path) {
+                    Ok(log) => Ok(Opened {
+                        log,
+                        kept_aside: Some(aside),
+                    }),
+                    Err(Unread::Failed(error)) => Err(error),
+                    Err(Unread::NotALog(problem)) => Err(LogError(problem)),
+                }
+            }
+        }
+    }
+
+    fn open_as_it_is(path: &Path) -> Result<Self, Unread> {
+        make_private(path)?;
+        let connection = Connection::open(path)?;
+        connection.busy_timeout(LOCK_WAIT)?;
+        let version: i64 = connection.query_row("PRAGMA user_version", [], |row| row.get(0))?;
+        let tables: i64 =
+            connection.query_row("SELECT count(*) FROM sqlite_master", [], |row| row.get(0))?;
+        match (version, tables) {
+            (0, 0) => {
+                // Before the first table: it decides how the file is laid
+                // out, and without it what retention deletes is never
+                // given back to the disk.
+                connection.execute_batch("PRAGMA auto_vacuum = INCREMENTAL")?;
+                connection.execute_batch(&format!(
+                    "BEGIN; {SCHEMA} PRAGMA user_version = {VERSION}; COMMIT;"
+                ))?;
+            }
+            (VERSION, _) => {
+                // Reads the table's definition, so a file with the version
+                // and without the table is found out here.
+                connection
+                    .prepare("SELECT id, at_ms, tool, result FROM calls LIMIT 1")
+                    .map_err(|error| Unread::NotALog(error.to_string()))?;
+            }
+            (other, _) => {
+                return Err(Unread::NotALog(format!(
+                    "the log is of version {other}, and this Tabletist reads version {VERSION}"
+                )));
+            }
+        }
+        Ok(Self {
+            connection: Arc::new(Mutex::new(connection)),
+        })
+    }
+
+    /// Adds an entry.
+    pub async fn record(&self, entry: Entry) -> Result<(), LogError> {
+        self.blocking(move |connection| {
+            connection.execute(
+                "INSERT INTO calls (at_ms, session, agent, project, transport, connection_id, \
+                 connection, tool, sql, params, request, result, code, message, ran_as, rows, \
+                 matched, columns, bytes, took_ms) \
+                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, \
+                 ?16, ?17, ?18, ?19, ?20)",
+                params![
+                    entry.at_ms,
+                    entry.session,
+                    entry.agent,
+                    entry.project,
+                    entry.transport,
+                    entry.connection_id,
+                    entry.connection,
+                    entry.tool,
+                    entry.sql,
+                    entry.params,
+                    entry.request,
+                    entry.result,
+                    entry.code,
+                    entry.message,
+                    entry.ran_as,
+                    entry.rows,
+                    entry.matched,
+                    entry.columns,
+                    entry.bytes,
+                    entry.took_ms,
+                ],
+            )?;
+            Ok(())
+        })
+        .await
+    }
+
+    /// The last `limit` entries, the newest first.
+    pub async fn latest(&self, limit: u32) -> Result<Vec<Entry>, LogError> {
+        self.blocking(move |connection| {
+            let mut statement = connection.prepare(
+                "SELECT at_ms, session, agent, project, transport, connection_id, connection, \
+                 tool, sql, params, request, result, code, message, ran_as, rows, matched, \
+                 columns, bytes, took_ms FROM calls ORDER BY id DESC LIMIT ?1",
+            )?;
+            let entries = statement
+                .query_map([limit], |row| {
+                    Ok(Entry {
+                        at_ms: row.get(0)?,
+                        session: row.get(1)?,
+                        agent: row.get(2)?,
+                        project: row.get(3)?,
+                        transport: row.get(4)?,
+                        connection_id: row.get(5)?,
+                        connection: row.get(6)?,
+                        tool: row.get(7)?,
+                        sql: row.get(8)?,
+                        params: row.get(9)?,
+                        request: row.get(10)?,
+                        result: row.get(11)?,
+                        code: row.get(12)?,
+                        message: row.get(13)?,
+                        ran_as: row.get(14)?,
+                        rows: row.get(15)?,
+                        matched: row.get(16)?,
+                        columns: row.get(17)?,
+                        bytes: row.get(18)?,
+                        took_ms: row.get(19)?,
+                    })
+                })?
+                .collect::<Result<Vec<Entry>, rusqlite::Error>>()?;
+            Ok(entries)
+        })
+        .await
+    }
+
+    /// Deletes every entry of a call that arrived before `at_ms`, gives
+    /// the room back to the disk, and says how many there were.
+    pub async fn forget_before(&self, at_ms: i64) -> Result<usize, LogError> {
+        self.blocking(move |connection| {
+            let gone = connection.execute("DELETE FROM calls WHERE at_ms < ?1", [at_ms])?;
+            // It answers a row for each page it frees: read them all.
+            let mut vacuum = connection.prepare("PRAGMA incremental_vacuum")?;
+            let mut pages = vacuum.query([])?;
+            while pages.next()?.is_some() {}
+            Ok(gone)
+        })
+        .await
+    }
+
+    async fn blocking<T: Send + 'static>(
+        &self,
+        work: impl FnOnce(&Connection) -> Result<T, LogError> + Send + 'static,
+    ) -> Result<T, LogError> {
+        let connection = self.connection.clone();
+        tokio::task::spawn_blocking(move || {
+            let connection = connection.lock().unwrap_or_else(PoisonError::into_inner);
+            work(&connection)
+        })
+        .await
+        .map_err(|error| LogError(format!("the log's task did not end: {error}")))?
+    }
+}
+
+/// Makes the file when it is not there, and makes it its owner's alone
+/// whether it was there or not: SQLite would make it with the permissions
+/// any new file gets, and a file that was there has whatever it was given.
+fn make_private(path: &Path) -> Result<(), LogError> {
+    let failed = |what: &str, error: std::io::Error| {
+        LogError(format!("could not {what} {}: {error}", path.display()))
+    };
+    let mut options = std::fs::OpenOptions::new();
+    options.write(true).create_new(true);
+    #[cfg(unix)]
+    {
+        use std::os::unix::fs::OpenOptionsExt;
+        options.mode(0o600);
+    }
+    match options.open(path) {
+        Ok(_) => {}
+        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
+        Err(error) => return Err(failed("create", error)),
+    }
+    #[cfg(unix)]
+    {
+        use std::os::unix::fs::PermissionsExt;
+        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))
+            .map_err(|error| failed("protect", error))?;
+    }
+    Ok(())
+}
+
+/// Renames a log that cannot be read to `<name>.bad`, or to `<name>.bad.1`
+/// and so on where earlier ones exist, so that a second failure never
+/// replaces the first copy. Gives the name it was kept under.
+fn keep_aside(path: &Path) -> Result<PathBuf, LogError> {
+    let name = |suffix: String| {
+        let mut name = path.as_os_str().to_owned();
+        name.push(suffix);
+        PathBuf::from(name)
+    };
+    let aside = (0..1_000)
+        .map(|n| match n {
+            0 => name(".bad".into()),
+            n => name(format!(".bad.{n}")),
+        })
+        // This also sees a link that leads nowhere, which a rename would
+        // replace.
+        .find(|candidate| candidate.symlink_metadata().is_err())
+        .ok_or_else(|| LogError(format!("every .bad name of {} is taken", path.display())))?;
+    std::fs::rename(path, &aside).map_err(|error| {
+        LogError(format!(
+            "could not keep {} aside as {}: {error}",
+            path.display(),
+            aside.display()
+        ))
+    })?;
+    Ok(aside)
+}
+
 #[cfg(test)]
 mod tests {
     use super::*;
````

- [ ] **Step 4: Run the tests**

Run: `cargo test --locked -p tabletist-mcp`

Expected: PASS, in about two seconds: one test waits out a lock.

```text
     Running unittests src/lib.rs
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

- [ ] **Step 5: Commit**

```bash
cargo fmt --all
cargo clippy --locked --workspace --all-targets -- -D warnings
git add Cargo.toml Cargo.lock crates/tabletist-mcp
git commit -m "Start the MCP server's crate with its log of calls"
```

---

### Task 3: A session, and `list_connections`

The server answers an agent for the first time: a session over a transport, the list of tools, and the one tool that needs no database. Everything a call passes through arrives here, so that the next tasks add tools and nothing else.

- **`gateway.rs`**: the trait. `list_connections` uses `connections()` alone. `open` and `Session` are used from the next task on: a trait's methods are its public face, so the compiler does not call them dead.
- **`stop.rs`**: a cancellation token with the reason it was cancelled for. A child is cancelled with its parent and gives the parent's reason: the server's stop is every session's, and a session's is every call's.
- **`refusal.rs`**: `Refused` and the codes. Of the sentences, only those the server itself says: to a call of a session that has ended, and to one call too many. The rest come with the tool that first says them. A sentence is cut at 2,000 characters where it is made.
- **`tools.rs`**: a tool in this crate's own words, the two ways a call can be no tool's (`Invalid`), and a call's note for the log.
- **`server.rs`**: `Server`, the sessions, and `call`: a call's way through the tools into the log and the app's log. A session is among the open ones through a guard that takes it out when its serving ends, however it ends. A session runs sixteen calls at once.
- **`protocol.rs`**: the SDK. `Handler` puts the tools on the wire, and names the agent from its first word. What a client is told the server is for names the one tool there is, and promises what holds for every tool to come: nothing it runs here changes data. `Watched` is the transport as the SDK is given it: a write that fails ends the session's calls, a message of more than a megabyte fails the read, and once the server shuts it every read and write fails. `serve` closes a session that was stopped once its calls have answered, and shuts the transport when the SDK cannot finish with it.

The tests are the SDK's client on the other end of `tokio::io::duplex`. `tests/support/mod.rs` has the gateway they use, over SQLite files of the Bookshop, the agent, and `Raw`, an agent that writes the protocol's lines itself. It is the one place in the crate that opens a database by where it is. It comes whole in this task, with what the next tasks' tests use of it.

What ends a call in flight (the client's cancel, a stop) is in `server.rs` and `protocol.rs` from this task on. It is tested in Task 5, when there is a tool slow enough to be in flight.

**Files:**
- Create: `crates/tabletist-mcp/src/gateway.rs`, `stop.rs`, `refusal.rs`, `tools.rs`, `tools/connections.rs`, `server.rs`, `protocol.rs`
- Create: `crates/tabletist-mcp/tests/support/mod.rs`, `tests/tools.rs`, `tests/sessions.rs`
- Modify: `crates/tabletist-mcp/Cargo.toml`, `crates/tabletist-mcp/src/lib.rs`, `Cargo.lock`

- [ ] **Step 1: The SDK, the tests, and the tests' own gateway**

`Cargo.lock` gains nine packages: the eight that come with `rmcp`, and `tokio-stream` for its client. Apply the diff; do not let cargo resolve (see "Before you start").

````diff
diff --git a/Cargo.lock b/Cargo.lock
index c99ac52..ad28f87 100644
--- a/Cargo.lock
+++ b/Cargo.lock
@@ -843,6 +843,7 @@ dependencies = [
  "iana-time-zone",
  "js-sys",
  "num-traits",
+ "serde",
  "wasm-bindgen",
  "windows-link",
 ]
@@ -1414,6 +1415,12 @@ name = "dpi"
 version = "0.1.1"
 source = "git+https://github.com/crmne/winit?rev=fc4f39ff0993dd235952b749dd16b856c0194e1f#fc4f39ff0993dd235952b749dd16b856c0194e1f"
 
+[[package]]
+name = "dyn-clone"
+version = "1.0.20"
+source = "registry+https://github.com/rust-lang/crates.io-index"
+checksum = "d0881ea181b1df73ff77ffaaf9c7544ecc11e82fba9b5f27b262a3c73a332555"
+
 [[package]]
 name = "ecdsa"
 version = "0.17.0"
@@ -2649,6 +2656,8 @@ checksum = "cc4e190f5d26ca7051642629da2c52fc03bde85a03197c99408dcd291734c855"
 dependencies = [
  "equivalent",
  "hashbrown 0.17.1",
+ "serde",
+ "serde_core",
 ]
 
 [[package]]
@@ -3985,6 +3994,12 @@ dependencies = [
  "phc",
 ]
 
+[[package]]
+name = "pastey"
+version = "0.2.3"
+source = "registry+https://github.com/rust-lang/crates.io-index"
+checksum = "2ee67f1008b1ba2321834326597b8e186293b049a023cdef258527550b9935b4"
+
 [[package]]
 name = "pbkdf2"
 version = "0.13.0"
@@ -4575,6 +4590,26 @@ dependencies = [
  "thiserror 2.0.21",
 ]
 
+[[package]]
+name = "ref-cast"
+version = "1.0.27"
+source = "registry+https://github.com/rust-lang/crates.io-index"
+checksum = "7e440fb4e4b4147295338efb76001ab9e4efc0e5839df2c47fc5ac2381d365c3"
+dependencies = [
+ "ref-cast-impl",
+]
+
+[[package]]
+name = "ref-cast-impl"
+version = "1.0.27"
+source = "registry+https://github.com/rust-lang/crates.io-index"
+checksum = "92ecd8964f8453721699a1ed72037b0db49ce2f5a5138486ee89bed6f67cdf3a"
+dependencies = [
+ "proc-macro2",
+ "quote",
+ "syn 3.0.6",
+]
+
 [[package]]
 name = "regex"
 version = "1.13.1"
@@ -4684,6 +4719,28 @@ dependencies = [
  "windows-sys 0.52.0",
 ]
 
+[[package]]
+name = "rmcp"
+version = "3.5.1"
+source = "registry+https://github.com/rust-lang/crates.io-index"
+checksum = "ab5023804cbe0304cbe06197ac16e645a0a0d0ffa2953bf735c15c2fa0213350"
+dependencies = [
+ "chrono",
+ "futures",
+ "indexmap",
+ "pastey",
+ "pin-project-lite",
+ "schemars",
+ "serde",
+ "serde_json",
+ "thiserror 2.0.21",
+ "tokio",
+ "tokio-stream",
+ "tokio-util",
+ "tracing",
+ "uuid",
+]
+
 [[package]]
 name = "ron"
 version = "0.12.2"
@@ -4966,6 +5023,32 @@ dependencies = [
  "windows-sys 0.61.2",
 ]
 
+[[package]]
+name = "schemars"
+version = "1.2.3"
+source = "registry+https://github.com/rust-lang/crates.io-index"
+checksum = "ab508826f74a77ca9d5aba6ff19b522583ee3eaf28a19384ff3d0e5835fadf6e"
+dependencies = [
+ "chrono",
+ "dyn-clone",
+ "ref-cast",
+ "schemars_derive",
+ "serde",
+ "serde_json",
+]
+
+[[package]]
+name = "schemars_derive"
+version = "1.2.3"
+source = "registry+https://github.com/rust-lang/crates.io-index"
+checksum = "e1c3a92094fa7d61aa124645844facb6b554dfc797136d0f5fd1f890e2bffc69"
+dependencies = [
+ "proc-macro2",
+ "quote",
+ "serde_derive_internals",
+ "syn 3.0.6",
+]
+
 [[package]]
 name = "scoped-tls"
 version = "1.0.1"
@@ -5088,6 +5171,17 @@ dependencies = [
  "syn 3.0.6",
 ]
 
+[[package]]
+name = "serde_derive_internals"
+version = "0.30.0"
+source = "registry+https://github.com/rust-lang/crates.io-index"
+checksum = "f852137cce035d6a4df67ccce505ff6b3e9fd3a10e3e52b24dc71e650bb1a9bd"
+dependencies = [
+ "proc-macro2",
+ "quote",
+ "syn 3.0.6",
+]
+
 [[package]]
 name = "serde_json"
 version = "1.0.151"
@@ -5655,9 +5749,15 @@ dependencies = [
 name = "tabletist-mcp"
 version = "0.1.0"
 dependencies = [
+ "log",
+ "rmcp",
  "rusqlite",
+ "serde_json",
+ "tabletist-db",
  "tempfile",
  "tokio",
+ "tokio-util",
+ "uuid",
 ]
 
 [[package]]
@@ -5884,6 +5984,17 @@ dependencies = [
  "tokio",
 ]
 
+[[package]]
+name = "tokio-stream"
+version = "0.1.19"
+source = "registry+https://github.com/rust-lang/crates.io-index"
+checksum = "a3d06f0b082ba57c26b79407372e57cf2a1e28124f78e9479fe80322cf53420b"
+dependencies = [
+ "futures-core",
+ "pin-project-lite",
+ "tokio",
+]
+
 [[package]]
 name = "tokio-util"
 version = "0.7.19"
diff --git a/crates/tabletist-mcp/Cargo.toml b/crates/tabletist-mcp/Cargo.toml
index ac3c761..8362a18 100644
--- a/crates/tabletist-mcp/Cargo.toml
+++ b/crates/tabletist-mcp/Cargo.toml
@@ -10,14 +10,40 @@ publish = false
 workspace = true
 
 [dependencies]
-# The log is written on the runtime's blocking pool.
-tokio = { version = "1", features = ["rt"] }
+# The official Rust SDK of the Model Context Protocol: the lifecycle, the
+# messages and the transport, which change faster than this app should have
+# to follow (the protocol dropped its handshake in 2026-07-28, and the SDK
+# serves both). Only `protocol` names its types. Without its defaults: no
+# macros (three tools, their schemas written by hand) and no base64.
+# `server` is the role, and brings the transport over anything that reads
+# and writes. It adds rmcp, schemars with its derive, pastey, dyn-clone and
+# ref-cast; the rest of what it needs is in the tree.
+rmcp = { version = "3.5", default-features = false, features = ["server"] }
+# The check on a statement, the run of a read, and the catalog's types.
+# Never `ConnectSpec` or `Secrets`: this crate reaches a database only
+# through `Gateway`, which opens a connection by its id.
+tabletist-db = { path = "../tabletist-db" }
+# The runtime the server's tasks run on, its timers, and the bounds a
+# transport is served through.
+tokio = { version = "1", features = ["rt", "sync", "time", "macros", "io-util"] }
+# `CancellationToken`: how a call, a session and the server are stopped. The
+# SDK hands one to each request, and it is in the tree through it.
+tokio-util = "0.7"
+# Tool arguments and answers are JSON.
+serde_json = "1"
 # The log of calls, a SQLite file. The crate and its bundled SQLite that
 # tabletist-db builds: nothing new.
 rusqlite = { version = "0.37", features = ["bundled"] }
+# A session's id in the log, which must not repeat from one start of the
+# app to the next.
+uuid = { version = "1", features = ["v4"] }
+# One line for each call in the app's log: the tool, the result, the time.
+log = "0.4"
 
 [dev-dependencies]
-# The tests' runtime.
-tokio = { version = "1", features = ["rt", "macros"] }
+# The SDK's client, for the tests that call the server as an agent does.
+rmcp = { version = "3.5", default-features = false, features = ["client"] }
+# The tests' runtime, on more than one thread, as the app's is.
+tokio = { version = "1", features = ["rt-multi-thread", "macros", "time"] }
 # A directory of its own for each test's files.
 tempfile = "3"
diff --git a/crates/tabletist-mcp/src/lib.rs b/crates/tabletist-mcp/src/lib.rs
index 6734bcc..53a4214 100644
--- a/crates/tabletist-mcp/src/lib.rs
+++ b/crates/tabletist-mcp/src/lib.rs
@@ -1,7 +1,26 @@
 //! The MCP server through which agents use Tabletist's connections.
 //!
-//! So far it is the [`Log`] of calls, and nothing that makes a call.
+//! An agent's tools only read. A refusal is a tool's result, with a code
+//! that does not change and a sentence the agent can act on. Every call is
+//! written to the [`Log`].
+//!
+//! The crate reaches a database only through a [`Gateway`], which lists the
+//! connections an agent may name and opens one by its id. None of its types
+//! holds where a database is or what opens it, so no tool can send that.
+//!
+//! Nothing here listens, and nothing in the app starts it yet: a
+//! [`Server`] serves whatever transport it is handed.
 
+mod gateway;
 mod log;
+mod protocol;
+mod refusal;
+mod server;
+mod stop;
+mod tools;
 
+pub use gateway::{Failure, Gateway, Listed, Schemas, Session};
 pub use log::{Entry, Log, LogError, Opened};
+pub use refusal::{Refused, code};
+pub use server::{Origin, Server, SessionInfo, Transport};
+pub use tools::{Tool, all as tools};
diff --git a/crates/tabletist-mcp/src/refusal.rs b/crates/tabletist-mcp/src/refusal.rs
new file mode 100644
index 0000000..f149161
--- /dev/null
+++ b/crates/tabletist-mcp/src/refusal.rs
@@ -0,0 +1,60 @@
+//! Why a call was not answered: a code an agent may match on, and a
+//! sentence it can act on. A refusal is a tool's result, not a protocol
+//! error, so that the model reads it.
+
+#[cfg(test)]
+mod tests {
+    use super::*;
+
+    #[test]
+    fn the_codes_are_the_ones_agents_were_told() {
+        assert_eq!(
+            [
+                code::UNKNOWN_CONNECTION,
+                code::CONNECTION_UNAVAILABLE,
+                code::TOO_LARGE,
+                code::INVALID_PARAMS,
+                code::TIMEOUT,
+                code::BUSY,
+                code::QUERY_FAILED,
+                code::CANCELLED,
+                code::SERVER_STOPPED,
+            ],
+            [
+                "unknown_connection",
+                "connection_unavailable",
+                "too_large",
+                "invalid_params",
+                "timeout",
+                "busy",
+                "query_failed",
+                "cancelled",
+                "server_stopped",
+            ]
+        );
+    }
+
+    #[test]
+    fn a_sentence_is_only_so_long() {
+        let refused = Refused::new(code::QUERY_FAILED, "é".repeat(600_000));
+        assert_eq!(refused.message.chars().count(), MAX_SENTENCE_CHARS + 1);
+        assert!(refused.message.ends_with('…'));
+        // A sentence is kept in the log as it is said, unless it says
+        // otherwise.
+        let short = Refused::too_many_calls();
+        assert_eq!(short.for_the_log(), short.message);
+        assert_eq!(short.code, "busy");
+    }
+
+    #[test]
+    fn a_call_to_a_session_that_has_ended_says_why() {
+        let stopped = Refused::session_ended(Why::Stopped);
+        assert_eq!(
+            (stopped.code, stopped.result()),
+            ("server_stopped", "stopped")
+        );
+        assert_eq!(stopped.message, "Tabletist's agent server was stopped.");
+        let gone = Refused::session_ended(Why::Client);
+        assert_eq!((gone.code, gone.result()), ("cancelled", "cancelled"));
+    }
+}
diff --git a/crates/tabletist-mcp/src/stop.rs b/crates/tabletist-mcp/src/stop.rs
new file mode 100644
index 0000000..fe16351
--- /dev/null
+++ b/crates/tabletist-mcp/src/stop.rs
@@ -0,0 +1,62 @@
+//! Stopping a call, and remembering why. A call can be told to end by more
+//! than one, and it gives one reason: the person's stop if there was one,
+//! else the first.
+
+#[cfg(test)]
+mod tests {
+    use super::*;
+
+    fn ended(stop: &Stop) -> bool {
+        stop.token.is_cancelled()
+    }
+
+    #[test]
+    fn the_first_reason_holds() {
+        let stop = Stop::new();
+        assert_eq!(stop.why(), None);
+        stop.end(Why::Client);
+        stop.end(Why::Stopped);
+        assert_eq!(stop.why(), Some(Why::Client));
+        assert!(ended(&stop));
+    }
+
+    #[test]
+    fn a_child_ends_with_its_parent_and_gives_its_reason() {
+        // The server's stop, a session's, a call's.
+        let server = Stop::new();
+        let session = server.child();
+        let call = session.child();
+        server.end(Why::Stopped);
+        assert!(ended(&call));
+        assert_eq!(call.why(), Some(Why::Stopped));
+        assert_eq!(session.why(), Some(Why::Stopped));
+    }
+
+    #[test]
+    fn a_child_ended_alone_leaves_its_parent() {
+        let session = Stop::new();
+        let call = session.child();
+        call.end(Why::Client);
+        assert!(!ended(&session));
+        assert_eq!(session.why(), None);
+        assert_eq!(call.why(), Some(Why::Client));
+        assert!(!ended(&session.child()));
+    }
+
+    #[test]
+    fn the_persons_stop_wins_over_what_ended_the_call_besides() {
+        // Whichever came first, and from however far up.
+        let server = Stop::new();
+        let session = server.child();
+        let call = session.child();
+        call.end(Why::Client);
+        server.end(Why::Stopped);
+        assert_eq!(call.why(), Some(Why::Stopped));
+
+        let session = Stop::new();
+        let call = session.child();
+        session.end(Why::Stopped);
+        call.end(Why::Client);
+        assert_eq!(call.why(), Some(Why::Stopped));
+    }
+}
diff --git a/crates/tabletist-mcp/src/tools.rs b/crates/tabletist-mcp/src/tools.rs
new file mode 100644
index 0000000..42f0028
--- /dev/null
+++ b/crates/tabletist-mcp/src/tools.rs
@@ -0,0 +1,26 @@
+//! The tools an agent has, in this crate's own words: what each is called,
+//! what it takes, and what a call of it answers. `protocol` puts them on
+//! the wire.
+
+#[cfg(test)]
+mod tests {
+    use super::*;
+
+    #[test]
+    fn the_tools_are_told_as_they_are() {
+        let names: Vec<&str> = all().iter().map(|tool| tool.name).collect();
+        assert_eq!(names, ["list_connections"]);
+        for tool in all() {
+            assert_eq!(tool.input["type"], json!("object"), "{}", tool.name);
+            assert!(!tool.description.is_empty() && !tool.title.is_empty());
+        }
+    }
+
+    #[test]
+    fn a_tool_that_is_not_there_is_answered_with_the_ones_that_are() {
+        assert_eq!(
+            Invalid::UnknownTool.message(),
+            "Unknown tool. Tabletist has list_connections."
+        );
+    }
+}
diff --git a/crates/tabletist-mcp/src/tools/connections.rs b/crates/tabletist-mcp/src/tools/connections.rs
new file mode 100644
index 0000000..33a90bb
--- /dev/null
+++ b/crates/tabletist-mcp/src/tools/connections.rs
@@ -0,0 +1,12 @@
+//! `list_connections`: the connections an agent may use.
+
+#[cfg(test)]
+mod tests {
+    use super::*;
+
+    #[test]
+    fn a_timeout_is_written_in_seconds() {
+        assert_eq!(seconds(Duration::from_secs(10)), json!(10));
+        assert_eq!(seconds(Duration::from_millis(1_500)), json!(1.5));
+    }
+}
diff --git a/crates/tabletist-mcp/tests/sessions.rs b/crates/tabletist-mcp/tests/sessions.rs
new file mode 100644
index 0000000..00af2d6
--- /dev/null
+++ b/crates/tabletist-mcp/tests/sessions.rs
@@ -0,0 +1,133 @@
+//! An agent's session: how it starts, what the server knows of it, and how
+//! the server lets it go.
+
+// `allow-unwrap-in-tests` does not cover helpers in an integration-test crate.
+#![allow(clippy::unwrap_used)]
+
+mod support;
+
+use rmcp::model::ProtocolVersion;
+use rmcp::service::ClientLifecycleMode;
+use rmcp::{ClientServiceExt, ServiceExt};
+use serde_json::json;
+use support::{Agent, Shop, client, request, serve, server};
+
+#[tokio::test]
+async fn a_client_of_the_newer_protocol_is_served_without_a_handshake() {
+    let shop = Shop::new();
+    let (server, log) = server(&shop);
+    let (transport, served, _received) = serve(&server);
+    let agent = client("cursor", None)
+        .serve_with_lifecycle(
+            transport,
+            ClientLifecycleMode::Discover {
+                preferred_versions: vec![ProtocolVersion::V_2026_07_28],
+            },
+        )
+        .await
+        .unwrap();
+    let called = support::called(
+        agent
+            .call_tool(request("list_connections", json!({})))
+            .await
+            .unwrap(),
+    );
+    assert_eq!(
+        called.answer()["connections"][0]["name"],
+        json!("Bookshop · dev")
+    );
+    // Such a client names itself in each request, not once at the start.
+    assert_eq!(server.open_sessions()[0].agent.as_deref(), Some("cursor"));
+    let _ = agent.cancel().await;
+    served.await.unwrap();
+    assert_eq!(support::entries(&log).await[0].agent, "cursor");
+}
+
+#[tokio::test]
+async fn a_session_is_known_while_it_is_open() {
+    let shop = Shop::new();
+    let (server, _log) = server(&shop);
+    assert_eq!(server.open_sessions(), []);
+    let agent = Agent::connect(&server).await;
+    // It has a name from its first word, before it calls anything.
+    assert_eq!(
+        server.open_sessions()[0].agent.as_deref(),
+        Some("Claude Code")
+    );
+    agent.call("list_connections", json!({})).await;
+    agent.call("list_connections", json!({})).await;
+    let open = server.open_sessions();
+    assert_eq!(open.len(), 1);
+    assert_eq!(open[0].agent.as_deref(), Some("Claude Code"));
+    assert_eq!(open[0].project.as_deref(), Some("tabletist-repo"));
+    assert_eq!(open[0].calls, 2);
+    let other = Agent::connect_as(&server, client("cursor", None)).await;
+    assert_eq!(server.open_sessions().len(), 2);
+    assert_ne!(server.open_sessions()[0].id, server.open_sessions()[1].id);
+    agent.close().await;
+    other.close().await;
+    assert_eq!(server.open_sessions(), []);
+}
+
+#[tokio::test]
+async fn a_name_a_client_gives_itself_is_a_label_a_person_can_read() {
+    let shop = Shop::new();
+    let (server, _log) = server(&shop);
+    let label =
+        |server: &support::Server<Shop>, at: usize| server.open_sessions()[at].agent.clone();
+
+    // Nothing that moves the cursor, and so long and no longer.
+    let name = format!("Evil\u{1b}[31m\n{}", "x".repeat(500));
+    let long = Agent::connect_as(&server, client(&name, None)).await;
+    let read = label(&server, 0).unwrap();
+    assert!(read.starts_with("Evil[31mx"), "{read}");
+    assert_eq!(read.chars().count(), 64);
+
+    // Nothing that turns the line around or cannot be seen.
+    let turned = Agent::connect_as(
+        &server,
+        client("Claude\u{202e}edoC\u{200b}\u{feff}\u{2028}", None),
+    )
+    .await;
+    assert_eq!(label(&server, 1).as_deref(), Some("ClaudeedoC"));
+
+    // A title a person could not read does not hide the name.
+    let blank = Agent::connect_as(&server, client("cursor", Some(" \u{200b}\n"))).await;
+    assert_eq!(label(&server, 2).as_deref(), Some("cursor"));
+
+    // And a client that gives nothing is known as nothing.
+    let nameless = Agent::connect_as(&server, client("\u{0}", Some(""))).await;
+    assert_eq!(label(&server, 3), None);
+    for agent in [long, turned, blank, nameless] {
+        agent.close().await;
+    }
+}
+
+#[tokio::test]
+async fn a_session_whose_serving_was_given_up_is_not_left_open() {
+    let shop = Shop::new();
+    let (server, _log) = server(&shop);
+    let (transport, served, _received) = serve(&server);
+    let agent = client("claude-code", None).serve(transport).await.unwrap();
+    assert_eq!(server.open_sessions().len(), 1);
+    served.abort();
+    let _ = served.await;
+    assert_eq!(server.open_sessions(), []);
+    drop(agent);
+}
+
+#[tokio::test]
+async fn stopping_the_server_ends_every_session_and_serves_no_new_one() {
+    let shop = Shop::new();
+    let (server, _log) = server(&shop);
+    let (first, second) = (Agent::connect(&server).await, Agent::connect(&server).await);
+    server.stop();
+    first.ended().await;
+    second.ended().await;
+    assert_eq!(server.open_sessions(), []);
+    // A transport handed to it now is closed at once.
+    let (transport, served, _received) = serve(&server);
+    served.await.unwrap();
+    drop(transport);
+    assert_eq!(server.open_sessions(), []);
+}
diff --git a/crates/tabletist-mcp/tests/support/mod.rs b/crates/tabletist-mcp/tests/support/mod.rs
new file mode 100644
index 0000000..302330e
--- /dev/null
+++ b/crates/tabletist-mcp/tests/support/mod.rs
@@ -0,0 +1,514 @@
+//! What the tests share: a gateway over SQLite files of the Bookshop, and
+//! an agent that calls the server through the SDK's client.
+//!
+//! This is the one place of the crate that opens a database by where it
+//! is. The crate's own sources never do (`tests/secrets.rs` reads them).
+
+// Each test binary uses its own part of this, and
+// `allow-unwrap-in-tests` does not cover helpers in an integration-test
+// crate.
+#![allow(dead_code, clippy::unwrap_used)]
+
+use std::path::{Path, PathBuf};
+use std::pin::Pin;
+use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
+use std::sync::{Arc, Mutex};
+use std::task::{Context, Poll};
+use std::time::Duration;
+
+use rmcp::model::{
+    CallToolRequestParams, CallToolResult, ClientCapabilities, ClientConfig, Implementation,
+};
+use rmcp::service::RunningService;
+use rmcp::{RoleClient, ServiceExt};
+use serde_json::Value as Json;
+use tabletist_db::agent::{Answer, Limits, Read};
+use tabletist_db::{
+    Access, ConnectSpec, Connection, Dialect, Error, HostKeys, ObjectInfo, ObjectRef, Secrets,
+    StopFlag, Structure,
+};
+pub use tabletist_mcp::Server;
+use tabletist_mcp::{Entry, Failure, Gateway, Listed, Log, Origin, Schemas, Session, Transport};
+use tokio::io::{AsyncBufReadExt, AsyncRead, AsyncWrite, AsyncWriteExt, BufReader, ReadBuf};
+use tokio::sync::OwnedMutexGuard;
+use tokio_util::sync::CancellationToken;
+
+/// The limits of the Bookshop's connections, unless a test gives others.
+pub const LIMITS: Limits = Limits {
+    rows: 500,
+    timeout: Duration::from_secs(10),
+};
+
+/// A gateway over SQLite files in a directory of its own. A clone is the
+/// same gateway.
+#[derive(Clone)]
+pub struct Shop {
+    inner: Arc<Inner>,
+}
+
+struct Inner {
+    dir: tempfile::TempDir,
+    connections: Mutex<Vec<Kept>>,
+    /// Whether the database is given the connection's timeout. Without it
+    /// only the server's own timer ends a statement.
+    database_times_out: AtomicBool,
+    /// The reads that were awaited to their end.
+    reads_ended: AtomicUsize,
+}
+
+#[derive(Clone)]
+struct Kept {
+    listed: Listed,
+    path: PathBuf,
+    /// A connection runs one call at a time.
+    turn: Arc<tokio::sync::Mutex<()>>,
+}
+
+impl Shop {
+    /// A gateway with one connection, "Bookshop · dev", over a new file
+    /// that holds the Bookshop.
+    pub fn new() -> Self {
+        let dir = tempfile::tempdir().unwrap();
+        let path = dir.path().join("bookshop.db");
+        tabletist_db::fixtures::write_sqlite_bookshop(&path).unwrap();
+        let shop = Self {
+            inner: Arc::new(Inner {
+                dir,
+                connections: Mutex::default(),
+                database_times_out: AtomicBool::new(true),
+                reads_ended: AtomicUsize::new(0),
+            }),
+        };
+        shop.list("Bookshop · dev", &path, LIMITS);
+        shop
+    }
+
+    /// Lets agents use the file at `path` under `name`. Its id is the name
+    /// with `id:` before it.
+    pub fn list(&self, name: &str, path: &Path, limits: Limits) {
+        self.unlist(name);
+        self.inner.connections.lock().unwrap().push(Kept {
+            listed: Listed {
+                id: format!("id:{name}"),
+                name: name.to_owned(),
+                dialect: Dialect::Sqlite,
+                limits,
+            },
+            path: path.to_owned(),
+            turn: Arc::default(),
+        });
+    }
+
+    /// Turns a connection's access Off.
+    pub fn unlist(&self, name: &str) {
+        self.inner
+            .connections
+            .lock()
+            .unwrap()
+            .retain(|kept| kept.listed.name != name);
+    }
+
+    /// The Bookshop's file.
+    pub fn bookshop(&self) -> PathBuf {
+        self.path("bookshop.db")
+    }
+
+    /// A path in the gateway's directory.
+    pub fn path(&self, file: &str) -> PathBuf {
+        self.inner.dir.path().join(file)
+    }
+
+    /// Runs `sql` on the Bookshop's file, as its owner would.
+    pub fn change(&self, sql: &str) {
+        rusqlite::Connection::open(self.bookshop())
+            .unwrap()
+            .execute_batch(sql)
+            .unwrap();
+    }
+
+    /// Leaves the statement's time to the server's own timer.
+    pub fn without_the_databases_timeout(&self) {
+        self.inner.database_times_out.store(false, Ordering::SeqCst);
+    }
+
+    pub fn reads_ended(&self) -> usize {
+        self.inner.reads_ended.load(Ordering::SeqCst)
+    }
+}
+
+impl Gateway for Shop {
+    type Session = ShopSession;
+
+    fn connections(&self) -> Vec<Listed> {
+        let connections = self.inner.connections.lock().unwrap();
+        connections.iter().map(|kept| kept.listed.clone()).collect()
+    }
+
+    async fn open(&self, id: &str, stop: &CancellationToken) -> Result<ShopSession, Failure> {
+        let kept = {
+            let connections = self.inner.connections.lock().unwrap();
+            connections
+                .iter()
+                .find(|kept| kept.listed.id == id)
+                .cloned()
+        };
+        let kept = kept.ok_or(Failure::Unavailable)?;
+        let turn = tokio::select! {
+            turn = tokio::time::timeout(kept.listed.limits.timeout, kept.turn.clone().lock_owned()) => {
+                turn.map_err(|_| Failure::Busy)?
+            }
+            () = stop.cancelled() => return Err(Error::Cancelled.into()),
+        };
+        let connection = Connection::connect_with(
+            &ConnectSpec::sqlite(&kept.path),
+            &Secrets::default(),
+            &HostKeys::default(),
+            Access::ReadOnly,
+        )
+        .await?;
+        Ok(ShopSession {
+            connection,
+            stop: stop.clone(),
+            shop: self.clone(),
+            _turn: turn,
+        })
+    }
+}
+
+pub struct ShopSession {
+    connection: Connection,
+    stop: CancellationToken,
+    shop: Shop,
+    _turn: OwnedMutexGuard<()>,
+}
+
+impl Session for ShopSession {
+    async fn schemas(&self) -> Result<Schemas, Failure> {
+        Ok(Schemas {
+            names: self.connection.list_schemas().await?,
+            home: Some("main".into()),
+        })
+    }
+
+    async fn objects(&self, schema: &str) -> Result<Vec<ObjectInfo>, Failure> {
+        Ok(self.connection.list_objects(schema).await?)
+    }
+
+    async fn describe(&self, object: &ObjectRef) -> Result<Structure, Failure> {
+        Ok(self.connection.describe(object).await?)
+    }
+
+    async fn read(&self, read: &Read, limits: Limits) -> Result<Answer, Failure> {
+        if self.stop.is_cancelled() {
+            return Err(Error::Cancelled.into());
+        }
+        let limits = match self.shop.inner.database_times_out.load(Ordering::SeqCst) {
+            true => limits,
+            false => Limits {
+                timeout: Duration::from_secs(3_600),
+                ..limits
+            },
+        };
+        let flag = StopFlag::new();
+        let run = self.connection.run_agent_read(read, limits, &flag);
+        tokio::pin!(run);
+        let answer = tokio::select! {
+            answer = &mut run => answer,
+            () = self.stop.cancelled() => {
+                flag.stop();
+                let _ = self.connection.cancel_handle().cancel().await;
+                // Awaited to its end, as the gateway promises.
+                run.await
+            }
+        };
+        self.shop.inner.reads_ended.fetch_add(1, Ordering::SeqCst);
+        Ok(answer?)
+    }
+}
+
+/// A server over `shop`, with its log in the shop's directory.
+pub fn server(shop: &Shop) -> (Server<Shop>, Log) {
+    let log = Log::open(&shop.path("agents.sqlite3")).unwrap().log;
+    (Server::new(shop.clone(), log.clone(), "0.0.0-test"), log)
+}
+
+/// An agent connected to a server.
+pub struct Agent {
+    pub client: RunningService<RoleClient, ClientConfig>,
+    /// Every byte the server has written to the agent.
+    pub received: Arc<Mutex<Vec<u8>>>,
+    served: tokio::task::JoinHandle<()>,
+}
+
+/// What a tool answered.
+#[derive(Debug, Clone, PartialEq)]
+pub struct Called {
+    /// The result's structured content, which a refusal has.
+    pub structured: Json,
+    pub text: String,
+    pub is_error: bool,
+    /// The text as JSON, which an answer is.
+    read: Json,
+}
+
+impl Called {
+    /// The refusal's code, of a call that was refused.
+    pub fn refused(&self) -> &str {
+        assert!(self.is_error, "not refused: {}", self.text);
+        assert_eq!(self.structured["message"], self.text);
+        self.structured["refused"].as_str().unwrap()
+    }
+
+    /// The answer, of a call that was answered: its text, read as JSON.
+    /// An answer is sent once, so it has no structured content beside it.
+    pub fn answer(&self) -> &Json {
+        assert!(!self.is_error, "refused: {}", self.text);
+        assert_eq!(self.structured, Json::Null);
+        assert!(self.read.is_object(), "{}", self.text);
+        &self.read
+    }
+}
+
+/// What a client says it is.
+pub fn client(name: &str, title: Option<&str>) -> ClientConfig {
+    let mut info = Implementation::new(name, "1.0.0");
+    info.title = title.map(str::to_owned);
+    ClientConfig::new(ClientCapabilities::default(), info)
+}
+
+impl Agent {
+    /// Connects "Claude Code", started in `tabletist-repo`.
+    pub async fn connect(server: &Server<Shop>) -> Self {
+        Self::connect_as(server, client("claude-code", Some("Claude Code"))).await
+    }
+
+    pub async fn connect_as(server: &Server<Shop>, client: ClientConfig) -> Self {
+        let (transport, served, received) = serve(server);
+        let client = client.serve(transport).await.unwrap();
+        Self {
+            client,
+            received,
+            served,
+        }
+    }
+
+    pub async fn call(&self, tool: &'static str, arguments: Json) -> Called {
+        called(
+            self.client
+                .call_tool(request(tool, arguments))
+                .await
+                .unwrap(),
+        )
+    }
+
+    /// Closes the agent's end, and waits for the server to have let the
+    /// session go.
+    pub async fn close(self) {
+        let Self { client, served, .. } = self;
+        let _ = client.cancel().await;
+        served.await.unwrap();
+    }
+
+    /// Waits for the server to have ended the session by itself.
+    pub async fn ended(self) {
+        self.served.await.unwrap();
+    }
+
+    /// What the server has written so far, as text.
+    pub fn received(&self) -> String {
+        String::from_utf8_lossy(&self.received.lock().unwrap()).into_owned()
+    }
+}
+
+/// Starts serving a new session, and gives the agent's end of it.
+pub fn serve(
+    server: &Server<Shop>,
+) -> (
+    Tap<tokio::io::DuplexStream>,
+    tokio::task::JoinHandle<()>,
+    Arc<Mutex<Vec<u8>>>,
+) {
+    serve_with_room(server, 4 * 1024 * 1024)
+}
+
+/// The same, over a transport that holds `room` bytes each way before a
+/// write to it waits.
+pub fn serve_with_room(
+    server: &Server<Shop>,
+    room: usize,
+) -> (
+    Tap<tokio::io::DuplexStream>,
+    tokio::task::JoinHandle<()>,
+    Arc<Mutex<Vec<u8>>>,
+) {
+    let (ours, theirs) = tokio::io::duplex(room);
+    let served = tokio::spawn({
+        let server = server.clone();
+        async move {
+            let origin = Origin {
+                transport: Transport::Stdio,
+                project: Some("tabletist-repo".into()),
+            };
+            server.serve(theirs, origin).await;
+        }
+    });
+    let received = Arc::<Mutex<Vec<u8>>>::default();
+    let tap = Tap {
+        inner: ours,
+        read: received.clone(),
+    };
+    (tap, served, received)
+}
+
+pub fn request(tool: &'static str, arguments: Json) -> CallToolRequestParams {
+    let Json::Object(arguments) = arguments else {
+        panic!("a tool's arguments are an object");
+    };
+    CallToolRequestParams::new(tool).with_arguments(arguments)
+}
+
+pub fn called(result: CallToolResult) -> Called {
+    let text = result
+        .content
+        .iter()
+        .filter_map(|block| block.as_text())
+        .map(|text| text.text.clone())
+        .collect::<Vec<_>>()
+        .join("\n");
+    Called {
+        structured: result.structured_content.unwrap_or(Json::Null),
+        read: serde_json::from_str(&text).unwrap_or(Json::Null),
+        text,
+        is_error: result.is_error.unwrap_or(false),
+    }
+}
+
+/// An agent that writes the protocol's lines itself, for what the SDK's
+/// client would never send.
+pub struct Raw {
+    pub write: tokio::io::WriteHalf<Tap<tokio::io::DuplexStream>>,
+    pub lines: tokio::io::Lines<BufReader<tokio::io::ReadHalf<Tap<tokio::io::DuplexStream>>>>,
+    pub served: tokio::task::JoinHandle<()>,
+}
+
+impl Raw {
+    /// Connects over a transport that holds `room` bytes each way, and
+    /// starts a session by the protocol's handshake.
+    pub async fn connect(server: &Server<Shop>, room: usize) -> Self {
+        let (transport, served, _received) = serve_with_room(server, room);
+        let (read, write) = tokio::io::split(transport);
+        let mut raw = Self {
+            write,
+            lines: BufReader::new(read).lines(),
+            served,
+        };
+        raw.send(serde_json::json!({
+            "jsonrpc": "2.0",
+            "id": 0,
+            "method": "initialize",
+            "params": {
+                "protocolVersion": "2025-11-25",
+                "capabilities": {},
+                "clientInfo": { "name": "raw", "version": "1" },
+            },
+        }))
+        .await;
+        assert_eq!(raw.next().await["id"], serde_json::json!(0));
+        raw.send(serde_json::json!({ "jsonrpc": "2.0", "method": "notifications/initialized" }))
+            .await;
+        raw
+    }
+
+    pub async fn send(&mut self, message: Json) {
+        self.write
+            .write_all(format!("{message}\n").as_bytes())
+            .await
+            .unwrap();
+    }
+
+    /// Sends a call of a tool, numbered `id`.
+    pub async fn call(&mut self, id: u64, tool: &str, arguments: Json) {
+        self.send(serde_json::json!({
+            "jsonrpc": "2.0",
+            "id": id,
+            "method": "tools/call",
+            "params": { "name": tool, "arguments": arguments },
+        }))
+        .await;
+    }
+
+    /// The next message the server sent.
+    pub async fn next(&mut self) -> Json {
+        let line = self.lines.next_line().await.unwrap().unwrap();
+        serde_json::from_str(&line).unwrap()
+    }
+}
+
+/// The entries of the log, the oldest first.
+pub async fn entries(log: &Log) -> Vec<Entry> {
+    let mut entries = log.latest(1_000).await.unwrap();
+    entries.reverse();
+    entries
+}
+
+/// A statement over `copies` of the Bookshop's order items that answers
+/// one row and takes long to: each copy multiplies the work by forty.
+pub fn slow(copies: usize) -> String {
+    let tables: Vec<String> = (1..=copies)
+        .map(|copy| format!("order_items t{copy}"))
+        .collect();
+    let weights: Vec<String> = (1..=copies)
+        .map(|copy| format!("t{copy}.quantity"))
+        .collect();
+    format!(
+        "SELECT count(*) FROM {} WHERE {} < 0",
+        tables.join(", "),
+        weights.join(" + ")
+    )
+}
+
+/// A transport that keeps a copy of what is read from it.
+pub struct Tap<T> {
+    inner: T,
+    read: Arc<Mutex<Vec<u8>>>,
+}
+
+impl<T: AsyncRead + Unpin> AsyncRead for Tap<T> {
+    fn poll_read(
+        mut self: Pin<&mut Self>,
+        context: &mut Context<'_>,
+        buffer: &mut ReadBuf<'_>,
+    ) -> Poll<std::io::Result<()>> {
+        let before = buffer.filled().len();
+        let polled = Pin::new(&mut self.inner).poll_read(context, buffer);
+        self.read
+            .lock()
+            .unwrap()
+            .extend_from_slice(&buffer.filled()[before..]);
+        polled
+    }
+}
+
+impl<T: AsyncWrite + Unpin> AsyncWrite for Tap<T> {
+    fn poll_write(
+        mut self: Pin<&mut Self>,
+        context: &mut Context<'_>,
+        bytes: &[u8],
+    ) -> Poll<std::io::Result<usize>> {
+        Pin::new(&mut self.inner).poll_write(context, bytes)
+    }
+
+    fn poll_flush(
+        mut self: Pin<&mut Self>,
+        context: &mut Context<'_>,
+    ) -> Poll<std::io::Result<()>> {
+        Pin::new(&mut self.inner).poll_flush(context)
+    }
+
+    fn poll_shutdown(
+        mut self: Pin<&mut Self>,
+        context: &mut Context<'_>,
+    ) -> Poll<std::io::Result<()>> {
+        Pin::new(&mut self.inner).poll_shutdown(context)
+    }
+}
diff --git a/crates/tabletist-mcp/tests/tools.rs b/crates/tabletist-mcp/tests/tools.rs
new file mode 100644
index 0000000..8212574
--- /dev/null
+++ b/crates/tabletist-mcp/tests/tools.rs
@@ -0,0 +1,94 @@
+//! The tools, called as an agent calls them: through the SDK's client, over
+//! a transport in this process, on the Bookshop.
+
+// `allow-unwrap-in-tests` does not cover helpers in an integration-test crate.
+#![allow(clippy::unwrap_used)]
+
+mod support;
+
+use rmcp::model::ErrorCode;
+use rmcp::service::ServiceError;
+use serde_json::{Value as Json, json};
+use support::{Agent, Shop, request, server};
+
+const DEV: &str = "Bookshop · dev";
+
+/// Calls a tool in a way that is no call of it, and gives the sentence of
+/// the protocol's error.
+async fn invalid(agent: &Agent, tool: &'static str, arguments: Json) -> String {
+    let error = agent
+        .client
+        .call_tool(request(tool, arguments))
+        .await
+        .unwrap_err();
+    let ServiceError::McpError(error) = error else {
+        panic!("{tool}: {error:?}");
+    };
+    assert_eq!(error.code, ErrorCode::INVALID_PARAMS, "{tool}");
+    error.message.to_string()
+}
+
+#[tokio::test]
+async fn an_agent_is_told_the_tools_and_that_each_only_reads() {
+    let shop = Shop::new();
+    let (server, _log) = server(&shop);
+    let agent = Agent::connect(&server).await;
+    let tools = agent.client.list_all_tools().await.unwrap();
+    let names: Vec<&str> = tools.iter().map(|tool| tool.name.as_ref()).collect();
+    assert_eq!(names, ["list_connections"]);
+    for tool in &tools {
+        let annotations = tool.annotations.as_ref().unwrap();
+        assert_eq!(annotations.read_only_hint, Some(true), "{}", tool.name);
+        assert!(tool.title.is_some() && tool.description.is_some());
+        assert_eq!(tool.input_schema["type"], json!("object"));
+    }
+    let told = agent.client.peer_info().unwrap();
+    let about = told.server_info.as_ref().unwrap();
+    assert_eq!(
+        (about.name.as_str(), about.version.as_str()),
+        ("tabletist", "0.0.0-test")
+    );
+    agent.close().await;
+}
+
+#[tokio::test]
+async fn list_connections_names_what_an_agent_may_use() {
+    let shop = Shop::new();
+    let (server, _log) = server(&shop);
+    let agent = Agent::connect(&server).await;
+    let called = agent.call("list_connections", json!({})).await;
+    assert_eq!(
+        called.answer(),
+        &json!({
+            "connections": [{
+                "name": DEV,
+                "engine": "SQLite",
+                "parameters": "?",
+                "access": "read",
+                "rows": 500,
+                "timeout_s": 10,
+            }]
+        })
+    );
+    agent.close().await;
+}
+
+#[tokio::test]
+async fn a_call_that_is_no_tools_is_the_protocols_error() {
+    let shop = Shop::new();
+    let (server, log) = server(&shop);
+    let agent = Agent::connect(&server).await;
+    // The sentence never repeats what was sent, here a name.
+    assert_eq!(
+        invalid(&agent, "drop_everything", json!({})).await,
+        "Unknown tool. Tabletist has list_connections."
+    );
+    assert_eq!(
+        invalid(&agent, "list_connections", json!({ "all": true })).await,
+        "list_connections takes no arguments."
+    );
+    // The session goes on, and neither is a call in the log.
+    assert!(!agent.call("list_connections", json!({})).await.is_error);
+    agent.close().await;
+    assert_eq!(support::entries(&log).await.len(), 1);
+}
````

- [ ] **Step 2: Run them to see that they do not build**

Run: `cargo test --locked -p tabletist-mcp`

Expected: it does not build. Three modules have no file, and two have only their tests:

```text
error[E0583]: file not found for module `gateway`
error[E0583]: file not found for module `protocol`
error[E0583]: file not found for module `server`
error[E0432]: unresolved imports `refusal::Refused`, `refusal::code`
error[E0432]: unresolved imports `tools::Tool`, `tools::all`
error: could not compile `tabletist-mcp` (lib) due to 5 previous errors
```

Cargo builds the crate and its tests side by side, so the same kind of error may follow from the tests' build (`lib test`): names that are not there yet.

- [ ] **Step 3: The server**

````diff
diff --git a/crates/tabletist-mcp/src/gateway.rs b/crates/tabletist-mcp/src/gateway.rs
new file mode 100644
index 0000000..6b1a39a
--- /dev/null
+++ b/crates/tabletist-mcp/src/gateway.rs
@@ -0,0 +1,104 @@
+//! How the server reaches a database: through whoever holds the
+//! connections, which lists those an agent may name and opens one by its
+//! id. Nothing here says where a database is or what opens it, so no tool
+//! can send that: the server never has it.
+
+use std::future::Future;
+
+use tabletist_db::agent::{Answer, Limits, Read};
+use tabletist_db::{Dialect, ObjectInfo, ObjectRef, Structure};
+use tokio_util::sync::CancellationToken;
+
+/// A connection as an agent may know it.
+#[derive(Debug, Clone, PartialEq, Eq)]
+pub struct Listed {
+    /// The saved connection's id. The log keeps it, so that a connection
+    /// keeps its entries when it is renamed. It is never sent to an agent.
+    pub id: String,
+    /// What an agent calls the connection. No two listed connections share
+    /// a name.
+    pub name: String,
+    pub dialect: Dialect,
+    /// The most rows of an answer, and how long a call may take.
+    pub limits: Limits,
+}
+
+/// The schemas of a connection.
+#[derive(Debug, Clone, PartialEq, Eq, Default)]
+pub struct Schemas {
+    /// Every schema the session lists, the databases' own among them.
+    pub names: Vec<String>,
+    /// The one a name without a schema is looked up in: the schema the app
+    /// opens first for this connection. `None` when there is no saying.
+    pub home: Option<String>,
+}
+
+/// Why the gateway did not answer.
+#[derive(Debug)]
+pub enum Failure {
+    /// The connection cannot be opened without a person, or could not be
+    /// opened at all. Why is for the person: it can name a host or a user.
+    Unavailable,
+    /// Another call held the connection for as long as this one may wait.
+    Busy,
+    /// The database's answer, or the driver's.
+    Database(tabletist_db::Error),
+}
+
+impl From<tabletist_db::Error> for Failure {
+    fn from(error: tabletist_db::Error) -> Self {
+        Self::Database(error)
+    }
+}
+
+/// Whoever holds the connections agents may use.
+///
+/// A connection whose access is Off is not among [`Gateway::connections`]
+/// and cannot be opened: to the server it does not exist.
+pub trait Gateway: Send + Sync + 'static {
+    type Session: Session;
+
+    /// The connections agents may use, as they are now.
+    fn connections(&self) -> Vec<Listed>;
+
+    /// The session of the connection with this id, for one call. A
+    /// connection runs one call at a time: a second one waits here, and is
+    /// [`Failure::Busy`] when it has waited the connection's timeout.
+    /// Opening is the implementation's to bound as well: the server's own
+    /// timer starts when it has the session.
+    ///
+    /// Once `stop` is cancelled, whatever the session is doing is ended,
+    /// and the call that asked answers [`tabletist_db::Error::Cancelled`].
+    /// A statement that was sent is awaited to its end all the same: the
+    /// futures of a session are never dropped half way.
+    fn open(
+        &self,
+        id: &str,
+        stop: &CancellationToken,
+    ) -> impl Future<Output = Result<Self::Session, Failure>> + Send;
+}
+
+/// A connection's session, held for one call. It is read-only at the
+/// database, whatever the connection's own setting is.
+pub trait Session: Send + Sync {
+    fn schemas(&self) -> impl Future<Output = Result<Schemas, Failure>> + Send;
+
+    /// The tables and views of a schema, by name.
+    fn objects(
+        &self,
+        schema: &str,
+    ) -> impl Future<Output = Result<Vec<ObjectInfo>, Failure>> + Send;
+
+    fn describe(
+        &self,
+        object: &ObjectRef,
+    ) -> impl Future<Output = Result<Structure, Failure>> + Send;
+
+    /// Runs a statement that passed the check
+    /// ([`tabletist_db::Connection::run_agent_read`]).
+    fn read(
+        &self,
+        read: &Read,
+        limits: Limits,
+    ) -> impl Future<Output = Result<Answer, Failure>> + Send;
+}
diff --git a/crates/tabletist-mcp/src/protocol.rs b/crates/tabletist-mcp/src/protocol.rs
new file mode 100644
index 0000000..77337fa
--- /dev/null
+++ b/crates/tabletist-mcp/src/protocol.rs
@@ -0,0 +1,285 @@
+//! The Model Context Protocol, through its SDK. This module alone names the
+//! SDK's types: the tools and the server speak this crate's own, so that
+//! the SDK can change, or be replaced, behind it.
+
+use std::future::Future;
+use std::pin::Pin;
+use std::sync::Arc;
+use std::task::{Context, Poll};
+use std::time::Duration;
+
+use rmcp::model::{
+    CallToolRequestParams, CallToolResponse, CallToolResult, ContentBlock, Implementation,
+    InitializeRequestParams, InitializeResult, ListToolsResult, PaginatedRequestParams,
+    ServerCapabilities, ServerConfig, Tool, ToolAnnotations,
+};
+use rmcp::service::{RequestContext, serve_server_with_ct};
+use rmcp::{ErrorData, RoleServer, ServerHandler};
+use serde_json::{Map, Value as Json, json};
+use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
+use tokio_util::sync::{CancellationToken, WaitForCancellationFutureOwned};
+
+use crate::gateway::Gateway;
+use crate::server::{Live, Reply, Server};
+use crate::stop::{Stop, Why};
+use crate::tools;
+
+/// What a client is told the server is for.
+const INSTRUCTIONS: &str = "Tabletist lets you read the databases its user has opened to agents. \
+                            Start with list_connections, which names them. Nothing you run \
+                            here can change data. A refusal says why, in a sentence you can \
+                            act on.";
+
+/// The most bytes of one message from a client. A statement is at most a
+/// tenth of it. The SDK reads a line into memory however long it is.
+pub(crate) const MAX_MESSAGE_BYTES: usize = 1_000_000;
+
+/// How long the SDK is given to close a session that was stopped, before
+/// its transport is taken from it. It sends what it still has first, and a
+/// client that reads nothing would hold it at that for good.
+const LET_GO: Duration = Duration::from_secs(3);
+
+/// Serves `live`'s agent on `io` until the client closes it or the session
+/// is stopped.
+pub(crate) async fn serve<G, T>(server: Server<G>, live: Arc<Live>, io: T)
+where
+    G: Gateway,
+    T: AsyncRead + AsyncWrite + Send + Unpin + 'static,
+{
+    let handler = Handler {
+        server,
+        live: live.clone(),
+    };
+    let shut = CancellationToken::new();
+    let io = Watched::new(io, live.stop.clone(), shut.clone());
+    let close = CancellationToken::new();
+    let start = serve_server_with_ct(handler, io, close.clone());
+    tokio::pin!(start);
+    let running = tokio::select! {
+        running = &mut start => running,
+        () = live.stop.ended() => {
+            close.cancel();
+            start.await
+        }
+    };
+    let Ok(running) = running else {
+        // What came first was not the start of a session.
+        return;
+    };
+    let ended = running.waiting();
+    tokio::pin!(ended);
+    tokio::select! {
+        // The client closed its end. The SDK has sent the answers of the
+        // calls that ended within five seconds of that, and cancelled
+        // none: what still runs is ended by whoever called this.
+        _ = &mut ended => {}
+        () = live.stop.ended() => {
+            // Each call in flight answers that it was stopped. Those
+            // answers go out before the transport closes.
+            live.settled().await;
+            close.cancel();
+            if tokio::time::timeout(LET_GO, &mut ended).await.is_err() {
+                shut.cancel();
+                let _ = ended.await;
+            }
+        }
+    }
+}
+
+/// A client's transport, as far as the SDK is given it.
+///
+/// - A write that fails ends the session's calls: the client is gone, and
+///   nobody will read an answer.
+/// - A message longer than [`MAX_MESSAGE_BYTES`] fails the read, which
+///   ends the session.
+/// - Once `shut` is cancelled every read and write fails, a waiting one
+///   too, so that the SDK lets go of a transport it cannot finish with.
+struct Watched<T> {
+    io: T,
+    stop: Stop,
+    shut: CancellationToken,
+    // One for each half: the SDK reads and writes on two tasks, and each
+    // must be woken.
+    shut_reading: Pin<Box<WaitForCancellationFutureOwned>>,
+    shut_writing: Pin<Box<WaitForCancellationFutureOwned>>,
+    /// The bytes read since the last end of a line.
+    line: usize,
+}
+
+impl<T> Watched<T> {
+    fn new(io: T, stop: Stop, shut: CancellationToken) -> Self {
+        Self {
+            io,
+            stop,
+            shut_reading: Box::pin(shut.clone().cancelled_owned()),
+            shut_writing: Box::pin(shut.clone().cancelled_owned()),
+            shut,
+            line: 0,
+        }
+    }
+}
+
+fn was_shut() -> std::io::Error {
+    std::io::Error::new(
+        std::io::ErrorKind::ConnectionAborted,
+        "the session was stopped",
+    )
+}
+
+impl<T: AsyncRead + Unpin> AsyncRead for Watched<T> {
+    fn poll_read(
+        mut self: Pin<&mut Self>,
+        context: &mut Context<'_>,
+        buffer: &mut ReadBuf<'_>,
+    ) -> Poll<std::io::Result<()>> {
+        if self.shut.is_cancelled() || self.shut_reading.as_mut().poll(context).is_ready() {
+            return Poll::Ready(Err(was_shut()));
+        }
+        let before = buffer.filled().len();
+        let polled = Pin::new(&mut self.io).poll_read(context, buffer);
+        if let Poll::Ready(Ok(())) = &polled {
+            let read = &buffer.filled()[before..];
+            self.line = match read.iter().rposition(|byte| *byte == b'\n') {
+                Some(end) => read.len() - end - 1,
+                None => self.line + read.len(),
+            };
+            if self.line > MAX_MESSAGE_BYTES {
+                return Poll::Ready(Err(std::io::Error::new(
+                    std::io::ErrorKind::InvalidData,
+                    "a message longer than the server reads",
+                )));
+            }
+        }
+        polled
+    }
+}
+
+impl<T: AsyncWrite + Unpin> AsyncWrite for Watched<T> {
+    fn poll_write(
+        mut self: Pin<&mut Self>,
+        context: &mut Context<'_>,
+        bytes: &[u8],
+    ) -> Poll<std::io::Result<usize>> {
+        if self.shut.is_cancelled() || self.shut_writing.as_mut().poll(context).is_ready() {
+            return Poll::Ready(Err(was_shut()));
+        }
+        let polled = Pin::new(&mut self.io).poll_write(context, bytes);
+        if matches!(polled, Poll::Ready(Err(_))) {
+            self.stop.end(Why::Client);
+        }
+        polled
+    }
+
+    fn poll_flush(
+        mut self: Pin<&mut Self>,
+        context: &mut Context<'_>,
+    ) -> Poll<std::io::Result<()>> {
+        if self.shut.is_cancelled() {
+            return Poll::Ready(Err(was_shut()));
+        }
+        Pin::new(&mut self.io).poll_flush(context)
+    }
+
+    fn poll_shutdown(
+        mut self: Pin<&mut Self>,
+        context: &mut Context<'_>,
+    ) -> Poll<std::io::Result<()>> {
+        if self.shut.is_cancelled() {
+            // Shut already: there is nothing more to say to this client.
+            return Poll::Ready(Ok(()));
+        }
+        Pin::new(&mut self.io).poll_shutdown(context)
+    }
+}
+
+struct Handler<G: Gateway> {
+    server: Server<G>,
+    live: Arc<Live>,
+}
+
+impl<G: Gateway> ServerHandler for Handler<G> {
+    fn get_info(&self) -> ServerConfig {
+        ServerConfig::new(ServerCapabilities::builder().enable_tools().build())
+            .with_server_info(
+                Implementation::new("tabletist", self.server.version()).with_title("Tabletist"),
+            )
+            .with_instructions(INSTRUCTIONS)
+    }
+
+    /// The SDK's own, and the agent's name, which a client of the older
+    /// protocols gives here and nowhere else.
+    async fn initialize(
+        &self,
+        request: InitializeRequestParams,
+        context: RequestContext<RoleServer>,
+    ) -> Result<InitializeResult, ErrorData> {
+        let client = &request.client_info;
+        self.live.name_agent(client.title.as_deref(), &client.name);
+        context.peer.set_peer_info(request.clone());
+        self.negotiate_initialize(&request)
+    }
+
+    async fn list_tools(
+        &self,
+        _request: Option<PaginatedRequestParams>,
+        context: RequestContext<RoleServer>,
+    ) -> Result<ListToolsResult, ErrorData> {
+        self.name_agent(&context);
+        Ok(ListToolsResult::with_all_items(
+            tools::all().into_iter().map(tool).collect(),
+        ))
+    }
+
+    async fn call_tool(
+        &self,
+        request: CallToolRequestParams,
+        context: RequestContext<RoleServer>,
+    ) -> Result<CallToolResponse, ErrorData> {
+        self.name_agent(&context);
+        let arguments = request.arguments.unwrap_or_default();
+        let reply = self
+            .server
+            .call(&self.live, &request.name, &arguments, &context.ct)
+            .await
+            .map_err(|invalid| ErrorData::invalid_params(invalid.message(), None))?;
+        Ok(result(reply).into())
+    }
+}
+
+impl<G: Gateway> Handler<G> {
+    /// Since the protocol's version of 2026-07-28 a client says what it is
+    /// called with each request, and has no handshake to say it in.
+    fn name_agent(&self, context: &RequestContext<RoleServer>) {
+        if let Some(client) = context.client_info() {
+            self.live.name_agent(client.title.as_deref(), &client.name);
+        }
+    }
+}
+
+fn tool(tool: tools::Tool) -> Tool {
+    let input = match tool.input {
+        Json::Object(schema) => schema,
+        _ => Map::new(),
+    };
+    Tool::new(tool.name, tool.description, Arc::new(input))
+        .with_title(tool.title)
+        .with_annotations(ToolAnnotations::new().read_only(true))
+}
+
+/// An answer is its JSON as text, once: a client that hands a model both a
+/// text and a structured result would hand it the rows twice. A refusal is
+/// a tool's result with `isError`, which a model reads (a protocol error
+/// it may never see), with its code where a program finds it.
+fn result(reply: Reply) -> CallToolResult {
+    match reply {
+        Reply::Answer(text) => CallToolResult::success(vec![ContentBlock::text(text)]),
+        Reply::Refused(refused) => {
+            let mut result = CallToolResult::error(vec![ContentBlock::text(&refused.message)]);
+            result.structured_content = Some(json!({
+                "refused": refused.code,
+                "message": refused.message,
+            }));
+            result
+        }
+    }
+}
diff --git a/crates/tabletist-mcp/src/refusal.rs b/crates/tabletist-mcp/src/refusal.rs
index f149161..3a41d38 100644
--- a/crates/tabletist-mcp/src/refusal.rs
+++ b/crates/tabletist-mcp/src/refusal.rs
@@ -2,6 +2,98 @@
 //! sentence it can act on. A refusal is a tool's result, not a protocol
 //! error, so that the model reads it.
 
+use crate::stop::Why;
+
+/// The codes of this crate's own refusals. The check's are its own
+/// ([`tabletist_db::agent::Refusal::code`]). None of them ever changes.
+pub mod code {
+    pub const UNKNOWN_CONNECTION: &str = "unknown_connection";
+    pub const CONNECTION_UNAVAILABLE: &str = "connection_unavailable";
+    pub const TOO_LARGE: &str = "too_large";
+    pub const INVALID_PARAMS: &str = "invalid_params";
+    pub const TIMEOUT: &str = "timeout";
+    pub const BUSY: &str = "busy";
+    pub const QUERY_FAILED: &str = "query_failed";
+    pub const CANCELLED: &str = "cancelled";
+    pub const SERVER_STOPPED: &str = "server_stopped";
+}
+
+/// A call that was not answered.
+#[derive(Debug, Clone, PartialEq, Eq)]
+pub struct Refused {
+    /// Stable. See [`code`].
+    pub code: &'static str,
+    /// For the agent. It may be reworded.
+    pub message: String,
+    /// What the log keeps in the sentence's place, where the sentence may
+    /// hold a row's value.
+    logged: Option<String>,
+}
+
+/// The most characters of a sentence. A database writes its errors as long
+/// as it likes, and with what it likes in them.
+pub const MAX_SENTENCE_CHARS: usize = 2_000;
+
+impl Refused {
+    pub(crate) fn new(code: &'static str, message: impl Into<String>) -> Self {
+        let message: String = message.into();
+        let message = match message.char_indices().nth(MAX_SENTENCE_CHARS) {
+            Some((end, _)) => format!("{}…", &message[..end]),
+            None => message,
+        };
+        Self {
+            code,
+            message,
+            logged: None,
+        }
+    }
+
+    /// The sentence as the log keeps it.
+    pub(crate) fn for_the_log(&self) -> &str {
+        self.logged.as_deref().unwrap_or(&self.message)
+    }
+
+    /// A session has as many calls running as it may.
+    pub(crate) fn too_many_calls() -> Self {
+        Self::new(
+            code::BUSY,
+            "Too many calls at once. Wait for an answer before the next call.",
+        )
+    }
+
+    pub(crate) fn cancelled() -> Self {
+        Self::new(code::CANCELLED, "The call was cancelled.")
+    }
+
+    pub(crate) fn stopped() -> Self {
+        Self::new(
+            code::SERVER_STOPPED,
+            "Tabletist's agent server was stopped.",
+        )
+    }
+
+    /// What a call answers that comes to a session which has ended: that
+    /// the person stopped it, or else that there is nobody to answer.
+    pub(crate) fn session_ended(why: Why) -> Self {
+        if why == Why::Stopped {
+            Self::stopped()
+        } else {
+            Self::cancelled()
+        }
+    }
+
+    /// What the log calls the call's end.
+    pub(crate) fn result(&self) -> &'static str {
+        match self.code {
+            code::TIMEOUT => "timeout",
+            code::CANCELLED => "cancelled",
+            code::SERVER_STOPPED => "stopped",
+            code::QUERY_FAILED | code::CONNECTION_UNAVAILABLE => "error",
+            _ => "refused",
+        }
+    }
+}
+
 #[cfg(test)]
 mod tests {
     use super::*;
diff --git a/crates/tabletist-mcp/src/server.rs b/crates/tabletist-mcp/src/server.rs
new file mode 100644
index 0000000..3427a2f
--- /dev/null
+++ b/crates/tabletist-mcp/src/server.rs
@@ -0,0 +1,390 @@
+//! The server: the sessions agents have open, and a call's way through
+//! the tools into the log.
+
+use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
+use std::sync::{Arc, Mutex, OnceLock, PoisonError};
+use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
+
+use serde_json::{Map, Value as Json};
+use tokio::io::{AsyncRead, AsyncWrite};
+use tokio::sync::RwLock;
+use tokio_util::sync::CancellationToken;
+
+use crate::gateway::Gateway;
+use crate::log::{Entry, Log};
+use crate::refusal::Refused;
+use crate::stop::{Stop, Why};
+use crate::tools::{self, Done, Invalid};
+
+/// How an agent is connected.
+#[derive(Debug, Clone, Copy, PartialEq, Eq)]
+pub enum Transport {
+    /// Through `tabletist mcp`, which the agent starts.
+    Stdio,
+}
+
+impl Transport {
+    pub fn as_str(self) -> &'static str {
+        match self {
+            Self::Stdio => "stdio",
+        }
+    }
+}
+
+/// Where a session comes from.
+#[derive(Debug, Clone, PartialEq, Eq)]
+pub struct Origin {
+    pub transport: Transport,
+    /// What the agent is working on, for the person to tell two sessions
+    /// apart: the name of the directory it was started in.
+    pub project: Option<String>,
+}
+
+/// A session as the app shows it.
+#[derive(Debug, Clone, PartialEq, Eq)]
+pub struct SessionInfo {
+    pub id: String,
+    /// What the client calls itself, once it has said. A label, not an
+    /// identity: a client may call itself anything.
+    pub agent: Option<String>,
+    pub project: Option<String>,
+    pub transport: Transport,
+    pub connected_at: SystemTime,
+    pub calls: u64,
+}
+
+/// What stands for an agent's name until it has given one.
+const UNNAMED: &str = "unknown";
+
+/// The most characters of a name a client gives itself.
+const MAX_AGENT_CHARS: usize = 64;
+
+/// How long a session that was stopped is left to send the answers of the
+/// calls it was stopped in, before its transport is closed.
+const LAST_ANSWERS: Duration = Duration::from_secs(5);
+
+/// The most calls of one session that run at once. One more is refused,
+/// so that a client cannot have the server hold calls without end.
+pub(crate) const MAX_CALLS_AT_ONCE: usize = 16;
+
+/// One agent's session, while it is open.
+#[derive(Debug)]
+pub(crate) struct Live {
+    id: String,
+    origin: Origin,
+    agent: OnceLock<String>,
+    connected_at: SystemTime,
+    calls: AtomicU64,
+    /// Ended when the person stops the session or the server, or when the
+    /// client is gone. Every call's stop is a child of it.
+    pub stop: Stop,
+    /// Held shared by each call from its start to its entry in the log.
+    in_flight: RwLock<()>,
+    /// How many calls are running.
+    running: AtomicUsize,
+}
+
+impl Live {
+    /// Names the agent, by the first name its client gives: its title, or
+    /// its name where the title is nothing a person could read.
+    pub fn name_agent(&self, title: Option<&str>, name: &str) {
+        let label = title
+            .map(label)
+            .filter(|label| !label.is_empty())
+            .unwrap_or_else(|| label(name));
+        if !label.is_empty() {
+            let _ = self.agent.set(label);
+        }
+    }
+
+    /// Waits until no call of the session is in flight, as long as
+    /// [`LAST_ANSWERS`].
+    pub async fn settled(&self) {
+        let _ = tokio::time::timeout(LAST_ANSWERS, self.in_flight.write()).await;
+    }
+
+    fn info(&self) -> SessionInfo {
+        SessionInfo {
+            id: self.id.clone(),
+            agent: self.agent.get().cloned(),
+            project: self.origin.project.clone(),
+            transport: self.origin.transport,
+            connected_at: self.connected_at,
+            calls: self.calls.load(Ordering::Relaxed),
+        }
+    }
+}
+
+/// A name a client gives itself, as a label a person reads: nothing that
+/// moves the cursor, turns the line around or cannot be seen, and of a fit
+/// length.
+fn label(name: &str) -> String {
+    let unseen = |c: char| {
+        c.is_control()
+            || matches!(
+                c,
+                '\u{200B}'..='\u{200F}'
+                    | '\u{2028}'..='\u{202E}'
+                    | '\u{2060}'..='\u{2069}'
+                    | '\u{FEFF}'
+            )
+    };
+    let label: String = name
+        .chars()
+        .filter(|c| !unseen(*c))
+        .take(MAX_AGENT_CHARS)
+        .collect();
+    label.trim().to_owned()
+}
+
+/// What a call answers.
+#[derive(Debug)]
+pub(crate) enum Reply {
+    /// The answer, as JSON.
+    Answer(String),
+    Refused(Refused),
+}
+
+/// A session among the open ones, until this is dropped: when its serving
+/// ends, and also when whoever served it gave up half way.
+struct Open<G: Gateway> {
+    server: Server<G>,
+    live: Arc<Live>,
+}
+
+impl<G: Gateway> Drop for Open<G> {
+    fn drop(&mut self) {
+        // Nobody is there to read an answer now.
+        self.live.stop.end(Why::Client);
+        self.server
+            .sessions()
+            .retain(|open| !Arc::ptr_eq(open, &self.live));
+    }
+}
+
+/// One more call running, until this is dropped.
+struct Running<'a>(&'a AtomicUsize);
+
+impl<'a> Running<'a> {
+    /// Counts a call in, and says how many were running before it.
+    fn count(running: &'a AtomicUsize) -> (Self, usize) {
+        let before = running.fetch_add(1, Ordering::SeqCst);
+        (Self(running), before)
+    }
+}
+
+impl Drop for Running<'_> {
+    fn drop(&mut self) {
+        self.0.fetch_sub(1, Ordering::SeqCst);
+    }
+}
+
+/// The MCP server. A clone is the same server.
+///
+/// Nothing in it listens: whoever has a transport an agent is on the other
+/// end of hands it to [`Server::serve`].
+pub struct Server<G: Gateway> {
+    inner: Arc<Inner<G>>,
+}
+
+impl<G: Gateway> Clone for Server<G> {
+    fn clone(&self) -> Self {
+        Self {
+            inner: self.inner.clone(),
+        }
+    }
+}
+
+struct Inner<G> {
+    gateway: G,
+    log: Log,
+    version: String,
+    sessions: Mutex<Vec<Arc<Live>>>,
+    /// Ended by [`Server::stop`]. Every session's stop is a child of it.
+    stop: Stop,
+}
+
+impl<G: Gateway> Server<G> {
+    /// A server over `gateway` that writes its calls to `log`. `version`
+    /// is the app's, which a client is told.
+    pub fn new(gateway: G, log: Log, version: impl Into<String>) -> Self {
+        Self {
+            inner: Arc::new(Inner {
+                gateway,
+                log,
+                version: version.into(),
+                sessions: Mutex::default(),
+                stop: Stop::new(),
+            }),
+        }
+    }
+
+    /// Serves one agent on `io` until either side closes it, or the
+    /// session or the server is stopped. It returns when every call of the
+    /// session has ended and is in the log.
+    pub async fn serve<T>(&self, io: T, origin: Origin)
+    where
+        T: AsyncRead + AsyncWrite + Send + Unpin + 'static,
+    {
+        let live = Arc::new(Live {
+            id: uuid::Uuid::new_v4().to_string(),
+            origin,
+            agent: OnceLock::new(),
+            connected_at: SystemTime::now(),
+            calls: AtomicU64::new(0),
+            stop: self.inner.stop.child(),
+            in_flight: RwLock::new(()),
+            running: AtomicUsize::new(0),
+        });
+        self.sessions().push(live.clone());
+        let open = Open {
+            server: self.clone(),
+            live: live.clone(),
+        };
+        crate::protocol::serve(self.clone(), live.clone(), io).await;
+        // Nobody is there to read an answer now.
+        live.stop.end(Why::Client);
+        let _ = live.in_flight.write().await;
+        drop(open);
+    }
+
+    /// The sessions that are open, the oldest first.
+    pub fn open_sessions(&self) -> Vec<SessionInfo> {
+        self.sessions().iter().map(|live| live.info()).collect()
+    }
+
+    /// Stops one session: each of its calls answers `server_stopped`, and
+    /// its transport is closed. Says whether there was such a session.
+    pub fn stop_session(&self, id: &str) -> bool {
+        let live = self.sessions().iter().find(|live| live.id == id).cloned();
+        match live {
+            Some(live) => {
+                live.stop.end(Why::Stopped);
+                true
+            }
+            None => false,
+        }
+    }
+
+    /// Stops every session, for good: a server that was stopped serves
+    /// nobody again.
+    pub fn stop(&self) {
+        self.inner.stop.end(Why::Stopped);
+    }
+
+    pub(crate) fn version(&self) -> &str {
+        &self.inner.version
+    }
+
+    fn sessions(&self) -> std::sync::MutexGuard<'_, Vec<Arc<Live>>> {
+        self.inner
+            .sessions
+            .lock()
+            .unwrap_or_else(PoisonError::into_inner)
+    }
+
+    /// Runs one call of `live`'s and writes it to the log. `cancelled` is
+    /// the client's own cancel of this call.
+    pub(crate) async fn call(
+        &self,
+        live: &Live,
+        name: &str,
+        arguments: &Map<String, Json>,
+        cancelled: &CancellationToken,
+    ) -> Result<Reply, Invalid> {
+        let arrived = SystemTime::now();
+        let started = Instant::now();
+        let _in_flight = live.in_flight.read().await;
+        let (_running, before) = Running::count(&live.running);
+        let stop = live.stop.child();
+        let refuse = |refused: Refused| {
+            known(name).map(|()| Done {
+                answer: Err(refused),
+                note: tools::Note::default(),
+            })
+        };
+        let done = match live.stop.why() {
+            // A session that was stopped is given nothing more.
+            Some(why) => refuse(Refused::session_ended(why)),
+            None if before >= MAX_CALLS_AT_ONCE => refuse(Refused::too_many_calls()),
+            None => {
+                let work = tools::call(&self.inner.gateway, name, arguments);
+                tokio::pin!(work);
+                tokio::select! {
+                    done = &mut work => done,
+                    () = cancelled.cancelled() => {
+                        stop.end(Why::Client);
+                        work.await
+                    }
+                }
+            }
+        }?;
+        live.calls.fetch_add(1, Ordering::Relaxed);
+        let Done { answer, note } = done;
+        let reply = match answer {
+            // A map of JSON values is always written.
+            Ok(json) => Reply::Answer(serde_json::to_string(&json).unwrap_or_default()),
+            Err(refused) => Reply::Refused(refused),
+        };
+        let took = note.took.unwrap_or_else(|| started.elapsed());
+        let (result, code, message, bytes) = match &reply {
+            Reply::Answer(text) => ("ok", None, None, Some(text.len())),
+            Reply::Refused(refused) => (
+                refused.result(),
+                Some(refused.code),
+                Some(refused.for_the_log().to_owned()),
+                None,
+            ),
+        };
+        log::info!(
+            "agent call: {name} {result}{} in {} ms",
+            code.map(|code| format!(" ({code})")).unwrap_or_default(),
+            took.as_millis()
+        );
+        let (connection_id, connection) = note.connection.unzip();
+        let entry = Entry {
+            at_ms: millis(arrived.duration_since(UNIX_EPOCH).unwrap_or_default()),
+            session: live.id.clone(),
+            agent: live.agent.get().map_or(UNNAMED, String::as_str).to_owned(),
+            project: live.origin.project.clone(),
+            transport: live.origin.transport.as_str().to_owned(),
+            connection_id,
+            connection,
+            tool: name.to_owned(),
+            sql: note.sql,
+            params: note.params,
+            request: note.request,
+            result: result.to_owned(),
+            code: code.map(str::to_owned),
+            message,
+            ran_as: note.ran_as.map(str::to_owned),
+            rows: note.rows.map(count),
+            matched: note.matched.map(count),
+            columns: note.columns.map(count),
+            bytes: bytes.map(|bytes| count(bytes as u64)),
+            took_ms: Some(millis(took)),
+        };
+        if let Err(error) = self.inner.log.record(entry).await {
+            // The call was answered all the same: a full disk must not
+            // turn every read into an error.
+            log::error!("an agent's call is not in the log: {error}");
+        }
+        Ok(reply)
+    }
+}
+
+/// Whether there is a tool of this name.
+fn known(name: &str) -> Result<(), Invalid> {
+    match tools::all().iter().any(|tool| tool.name == name) {
+        true => Ok(()),
+        false => Err(Invalid::UnknownTool),
+    }
+}
+
+fn millis(time: Duration) -> i64 {
+    i64::try_from(time.as_millis()).unwrap_or(i64::MAX)
+}
+
+fn count(count: u64) -> i64 {
+    i64::try_from(count).unwrap_or(i64::MAX)
+}
diff --git a/crates/tabletist-mcp/src/stop.rs b/crates/tabletist-mcp/src/stop.rs
index fe16351..663b814 100644
--- a/crates/tabletist-mcp/src/stop.rs
+++ b/crates/tabletist-mcp/src/stop.rs
@@ -2,6 +2,83 @@
 //! than one, and it gives one reason: the person's stop if there was one,
 //! else the first.
 
+use std::sync::{Arc, OnceLock};
+
+use tokio_util::sync::CancellationToken;
+
+/// Why a call was stopped.
+#[derive(Debug, Clone, Copy, PartialEq, Eq)]
+pub(crate) enum Why {
+    /// The client cancelled the call, or is gone.
+    Client,
+    /// The person stopped the session or the server.
+    Stopped,
+}
+
+/// A token and the reason it was cancelled for. A child is cancelled with
+/// its parent: the server's stop is every session's, and a session's is
+/// every call's.
+#[derive(Debug, Clone)]
+pub(crate) struct Stop {
+    token: CancellationToken,
+    reasons: Arc<Reasons>,
+}
+
+/// A stop's own reason, and its parent's.
+#[derive(Debug, Default)]
+struct Reasons {
+    own: OnceLock<Why>,
+    parent: Option<Arc<Reasons>>,
+}
+
+impl Stop {
+    pub fn new() -> Self {
+        Self {
+            token: CancellationToken::new(),
+            reasons: Arc::default(),
+        }
+    }
+
+    /// A stop that ends when this one does, and can be ended alone.
+    pub fn child(&self) -> Self {
+        Self {
+            token: self.token.child_token(),
+            reasons: Arc::new(Reasons {
+                own: OnceLock::new(),
+                parent: Some(self.reasons.clone()),
+            }),
+        }
+    }
+
+    /// Ends whatever holds this stop or a child of it. Of the reasons one
+    /// stop is given, the first holds.
+    pub fn end(&self, why: Why) {
+        let _ = self.reasons.own.set(why);
+        self.token.cancel();
+    }
+
+    /// Why this stop was ended, or a stop above it. The person's stop wins
+    /// wherever it stands: a call that was stopped says so, though
+    /// something else ended it as well. Otherwise the nearest reason.
+    pub fn why(&self) -> Option<Why> {
+        let mut nearest = None;
+        let mut at = Some(&self.reasons);
+        while let Some(reasons) = at {
+            match reasons.own.get() {
+                Some(Why::Stopped) => return Some(Why::Stopped),
+                Some(why) => nearest = nearest.or(Some(*why)),
+                None => {}
+            }
+            at = reasons.parent.as_ref();
+        }
+        nearest
+    }
+
+    pub async fn ended(&self) {
+        self.token.cancelled().await;
+    }
+}
+
 #[cfg(test)]
 mod tests {
     use super::*;
diff --git a/crates/tabletist-mcp/src/tools.rs b/crates/tabletist-mcp/src/tools.rs
index 42f0028..c858b40 100644
--- a/crates/tabletist-mcp/src/tools.rs
+++ b/crates/tabletist-mcp/src/tools.rs
@@ -2,6 +2,101 @@
 //! what it takes, and what a call of it answers. `protocol` puts them on
 //! the wire.
 
+mod connections;
+
+use std::time::Duration;
+
+use serde_json::{Map, Value as Json, json};
+
+use crate::gateway::Gateway;
+use crate::refusal::Refused;
+
+/// A tool as an agent is told of it.
+#[derive(Debug, Clone, PartialEq)]
+pub struct Tool {
+    pub name: &'static str,
+    pub title: &'static str,
+    pub description: &'static str,
+    /// The JSON Schema of its arguments.
+    pub input: Json,
+}
+
+/// Every tool. Each of them only reads.
+pub fn all() -> Vec<Tool> {
+    vec![connections::tool()]
+}
+
+/// A call that is not one of a tool's: the protocol's error, not a tool's
+/// result. Its sentence never repeats what the agent sent: it is the SDK
+/// that answers with it, and the SDK traces what it answers.
+#[derive(Debug, Clone, PartialEq, Eq)]
+pub(crate) enum Invalid {
+    UnknownTool,
+    Arguments(String),
+}
+
+impl Invalid {
+    pub fn message(&self) -> String {
+        match self {
+            Self::UnknownTool => {
+                let names: Vec<&str> = all().iter().map(|tool| tool.name).collect();
+                format!("Unknown tool. Tabletist has {}.", names.join(", "))
+            }
+            Self::Arguments(what) => what.clone(),
+        }
+    }
+}
+
+/// What a call was about and what came of it, for the log. Never a row's
+/// value.
+#[derive(Debug, Clone, PartialEq, Eq, Default)]
+pub(crate) struct Note {
+    /// The id and the name of the connection the call named, once it was
+    /// found.
+    pub connection: Option<(String, String)>,
+    pub sql: Option<String>,
+    /// The values, as the JSON they came in.
+    pub params: Option<String>,
+    /// The tables `describe_schema` was asked for.
+    pub request: Option<String>,
+    pub ran_as: Option<&'static str>,
+    pub rows: Option<u64>,
+    /// The statement's rows, where the answer holds them all.
+    pub matched: Option<u64>,
+    pub columns: Option<u64>,
+    pub took: Option<Duration>,
+}
+
+/// A call's end: the answer or the refusal, and the note.
+#[derive(Debug)]
+pub(crate) struct Done {
+    pub answer: Result<Map<String, Json>, Refused>,
+    pub note: Note,
+}
+
+/// Runs the tool called `name`.
+pub(crate) async fn call<G: Gateway>(
+    gateway: &G,
+    name: &str,
+    arguments: &Map<String, Json>,
+) -> Result<Done, Invalid> {
+    match name {
+        connections::NAME => connections::call(gateway, arguments),
+        _ => Err(Invalid::UnknownTool),
+    }
+}
+
+/// The schema of an object: `properties`, all of them strings unless said
+/// otherwise, of which `required` must be there.
+fn schema(properties: Json, required: &[&str]) -> Json {
+    json!({
+        "type": "object",
+        "properties": properties,
+        "required": required,
+        "additionalProperties": false,
+    })
+}
+
 #[cfg(test)]
 mod tests {
     use super::*;
diff --git a/crates/tabletist-mcp/src/tools/connections.rs b/crates/tabletist-mcp/src/tools/connections.rs
index 33a90bb..d49096d 100644
--- a/crates/tabletist-mcp/src/tools/connections.rs
+++ b/crates/tabletist-mcp/src/tools/connections.rs
@@ -1,5 +1,70 @@
 //! `list_connections`: the connections an agent may use.
 
+use std::time::Duration;
+
+use serde_json::{Map, Value as Json, json};
+use tabletist_db::Dialect;
+
+use super::{Done, Invalid, Note, Tool, schema};
+use crate::gateway::{Gateway, Listed};
+
+pub const NAME: &str = "list_connections";
+
+pub fn tool() -> Tool {
+    Tool {
+        name: NAME,
+        title: "List connections",
+        description: "The database connections you may use, with what each is and its limits. \
+                      Call a connection by its name, exactly as it is written here. A \
+                      connection the person has not opened to agents is not listed.",
+        input: schema(json!({}), &[]),
+    }
+}
+
+pub fn call<G: Gateway>(gateway: &G, arguments: &Map<String, Json>) -> Result<Done, Invalid> {
+    if !arguments.is_empty() {
+        return Err(Invalid::Arguments(format!("{NAME} takes no arguments.")));
+    }
+    let listed = gateway.connections();
+    let mut answer = Map::new();
+    answer.insert(
+        "connections".into(),
+        Json::Array(listed.iter().map(entry).collect()),
+    );
+    Ok(Done {
+        answer: Ok(answer),
+        note: Note {
+            rows: Some(listed.len() as u64),
+            ..Note::default()
+        },
+    })
+}
+
+fn entry(listed: &Listed) -> Json {
+    // What an agent must know to write a statement the database reads.
+    let (engine, parameters) = match listed.dialect {
+        Dialect::Postgres => ("PostgreSQL", "$1, $2"),
+        Dialect::MySql => ("MySQL", "?"),
+        Dialect::Sqlite => ("SQLite", "?"),
+    };
+    json!({
+        "name": listed.name,
+        "engine": engine,
+        "parameters": parameters,
+        "access": "read",
+        "rows": listed.limits.rows,
+        "timeout_s": seconds(listed.limits.timeout),
+    })
+}
+
+/// Whole seconds as a whole number.
+fn seconds(time: Duration) -> Json {
+    match time.subsec_nanos() {
+        0 => json!(time.as_secs()),
+        _ => json!(time.as_secs_f64()),
+    }
+}
+
 #[cfg(test)]
 mod tests {
     use super::*;
````

- [ ] **Step 4: Run the tests**

Run: `cargo test --locked -p tabletist-mcp`

Expected: PASS.

```text
     Running unittests src/lib.rs
test result: ok. 20 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
     Running tests/sessions.rs
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
     Running tests/tools.rs
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

- [ ] **Step 5: Commit**

```bash
cargo fmt --all
cargo clippy --locked --workspace --all-targets -- -D warnings
git add Cargo.lock crates/tabletist-mcp
git commit -m "Serve an agent's session, and tell it the connections it may use"
```

---

### Task 4: `run_query`

One statement that reads. The tool finds the connection by its name, turns the agent's values into the check's, checks the statement (`agent::check_read`, on the blocking pool: it parses on a thread of its own and waits for it), takes the connection's session from the gateway, runs the read, and writes the answer.

- **`answer.rs`** writes the answer as JSON and keeps it under its size. It counts the bytes as it goes, a row at a time, so the answer is never built past the size and then cut.
- **`refusal.rs`** gets what a failure is to an agent. `Refused::failed` names every variant of `tabletist_db::Error`: one that is about the session answers the one fixed sentence, whatever its words were. A database's own error is the agent's to read, and the log's to keep without its words.
- **`tools.rs`** gets `Arguments`, which refuses an argument a tool does not have, `find`, `kept`, which is what the log keeps of what was sent, and `settle`, which makes a call the person stopped answer that whatever it had got to.

The database's own timeout ends a statement that takes too long (Phase 1a), and answers `timeout` here. What ends a call the database did not end is the next task.

**Files:**
- Create: `crates/tabletist-mcp/src/answer.rs`, `crates/tabletist-mcp/src/tools/query.rs`
- Modify: `crates/tabletist-mcp/src/lib.rs`, `refusal.rs`, `tools.rs`, `stop.rs` (the token a gateway is handed), `server.rs` (a call is handed its stop), `crates/tabletist-mcp/tests/tools.rs`

- [ ] **Step 1: Write the failing tests**

````diff
diff --git a/crates/tabletist-mcp/src/answer.rs b/crates/tabletist-mcp/src/answer.rs
new file mode 100644
index 0000000..2636175
--- /dev/null
+++ b/crates/tabletist-mcp/src/answer.rs
@@ -0,0 +1,169 @@
+//! A read's answer as the JSON an agent gets, kept under a size: a row
+//! limit does not bound an answer, and what is sent goes into the agent's
+//! context.
+
+#[cfg(test)]
+mod tests {
+    use std::time::Duration;
+
+    use tabletist_db::{ColumnMeta, ValueKind};
+
+    use super::*;
+
+    fn column(name: &str, type_name: &str) -> ColumnMeta {
+        ColumnMeta {
+            name: name.into(),
+            type_name: type_name.into(),
+            kind: ValueKind::Other,
+        }
+    }
+
+    fn answer(rows: Vec<Vec<Value>>, truncated: bool) -> Answer {
+        Answer {
+            columns: vec![column("id", "INTEGER"), column("note", "")],
+            rows,
+            truncated,
+            elapsed: Duration::from_millis(4),
+        }
+    }
+
+    fn wrote(answer: &Answer, max_bytes: usize) -> Written {
+        write(answer, max_bytes).unwrap()
+    }
+
+    fn text(text: &str) -> Value {
+        Value::Text(text.into())
+    }
+
+    #[test]
+    fn an_answer_is_its_columns_its_rows_and_how_long_it_took() {
+        let written = wrote(
+            &answer(
+                vec![
+                    vec![Value::Int(1), text("print")],
+                    vec![Value::Int(9_100_000_000_000_000_004), Value::Null],
+                ],
+                false,
+            ),
+            MAX_ANSWER_BYTES,
+        );
+        assert_eq!(
+            Json::Object(written.json.clone()),
+            json!({
+                "columns": [{ "name": "id", "type": "INTEGER" }, { "name": "note" }],
+                "rows": [[1, "print"], [9_100_000_000_000_000_004_i64, null]],
+                "row_count": 2,
+                "truncated": false,
+                "took_ms": 4,
+            })
+        );
+        assert_eq!((written.rows, written.columns, written.cut), (2, 2, None));
+    }
+
+    #[test]
+    fn an_answer_the_row_limit_cut_says_so() {
+        let written = wrote(
+            &answer(vec![vec![Value::Int(1), Value::Null]], true),
+            MAX_ANSWER_BYTES,
+        );
+        assert_eq!(written.json["truncated"], json!(true));
+        assert_eq!(written.json["truncated_by"], json!("row_limit"));
+        assert_eq!(written.cut, Some(Cut::RowLimit));
+    }
+
+    #[test]
+    fn a_long_text_is_cut_and_says_how_long_it_was() {
+        // Counted in characters, not bytes: each of these is two bytes.
+        let long = "é".repeat(MAX_CELL_CHARS + 1);
+        let exact = "é".repeat(MAX_CELL_CHARS);
+        let written = wrote(
+            &answer(
+                vec![
+                    vec![Value::Int(1), text(&long)],
+                    vec![Value::Int(2), text(&exact)],
+                ],
+                false,
+            ),
+            MAX_ANSWER_BYTES,
+        );
+        let rows = written.json["rows"].as_array().unwrap();
+        assert_eq!(
+            rows[0][1],
+            json!({ "cut": exact, "characters": MAX_CELL_CHARS + 1 })
+        );
+        assert_eq!(rows[1][1], json!(exact));
+    }
+
+    #[test]
+    fn a_binary_value_is_described_and_not_sent() {
+        let bytes = Value::Bytes(vec![0xde, 0xad, 0xbe, 0xef].into());
+        let written = wrote(
+            &answer(vec![vec![Value::Int(1), bytes]], false),
+            MAX_ANSWER_BYTES,
+        );
+        assert_eq!(written.json["rows"][0][1], json!({ "binary_bytes": 4 }));
+    }
+
+    #[test]
+    fn a_number_json_cannot_write_is_its_name() {
+        let row = vec![
+            Value::Float(f64::NAN),
+            Value::Float(f64::INFINITY),
+            Value::Float(f64::NEG_INFINITY),
+            Value::Float(1.5),
+            Value::Bool(true),
+        ];
+        let mut answer = answer(vec![row], false);
+        answer.columns = (0..5).map(|n| column(&format!("c{n}"), "")).collect();
+        let written = wrote(&answer, MAX_ANSWER_BYTES);
+        assert_eq!(
+            written.json["rows"][0],
+            json!(["NaN", "Infinity", "-Infinity", 1.5, true])
+        );
+    }
+
+    #[test]
+    fn an_answer_over_the_size_loses_rows_from_the_end_and_says_so() {
+        let rows: Vec<Vec<Value>> = (0..100)
+            .map(|n| vec![Value::Int(n), text(&"x".repeat(90))])
+            .collect();
+        let max = 2_000;
+        let written = wrote(&answer(rows.clone(), false), max);
+        assert_eq!(written.cut, Some(Cut::Size));
+        assert_eq!(written.json["truncated_by"], json!("size"));
+        assert!(written.rows > 0 && written.rows < 100, "{}", written.rows);
+        assert_eq!(written.json["row_count"], json!(written.rows));
+        // The rows that stayed are the first ones.
+        assert_eq!(written.json["rows"][0][0], json!(0));
+        // What is sent is under the size, and one more row would not be.
+        let size = Json::Object(written.json.clone()).to_string().len();
+        assert!(size <= max, "{size}");
+        assert!(size + 100 > max, "{size}");
+        // The size cut is the reason given, whatever the row limit did.
+        assert_eq!(wrote(&answer(rows, true), max).cut, Some(Cut::Size));
+    }
+
+    #[test]
+    fn columns_that_are_too_much_by_themselves_are_no_answer() {
+        let mut wide = answer(Vec::new(), false);
+        wide.columns = (0..2_000).map(|_| column(&"a".repeat(2_000), "")).collect();
+        assert_eq!(write(&wide, MAX_ANSWER_BYTES), Err(TooWide));
+        // Many columns of an ordinary width are an answer.
+        wide.columns = (0..1_600)
+            .map(|n| column(&format!("column_{n}"), "TEXT"))
+            .collect();
+        assert!(write(&wide, MAX_ANSWER_BYTES).is_ok());
+    }
+
+    #[test]
+    fn an_answer_that_fits_exactly_is_sent_whole() {
+        let rows = vec![
+            vec![Value::Int(1), text("a")],
+            vec![Value::Int(2), text("b")],
+        ];
+        let whole = wrote(&answer(rows.clone(), true), MAX_ANSWER_BYTES);
+        let size = Json::Object(whole.json).to_string().len();
+        assert_eq!(wrote(&answer(rows.clone(), true), size).rows, 2);
+        assert_eq!(wrote(&answer(rows, true), size - 1).rows, 1);
+    }
+}
diff --git a/crates/tabletist-mcp/src/lib.rs b/crates/tabletist-mcp/src/lib.rs
index 53a4214..bf3f148 100644
--- a/crates/tabletist-mcp/src/lib.rs
+++ b/crates/tabletist-mcp/src/lib.rs
@@ -4,6 +4,10 @@
 //! that does not change and a sentence the agent can act on. Every call is
 //! written to the [`Log`].
 //!
+//! A statement passes [`tabletist_db::agent::check_read`] before it reaches
+//! a database, runs in a read-only transaction that is rolled back, and its
+//! answer is kept under a size.
+//!
 //! The crate reaches a database only through a [`Gateway`], which lists the
 //! connections an agent may name and opens one by its id. None of its types
 //! holds where a database is or what opens it, so no tool can send that.
@@ -11,6 +15,7 @@
 //! Nothing here listens, and nothing in the app starts it yet: a
 //! [`Server`] serves whatever transport it is handed.
 
+mod answer;
 mod gateway;
 mod log;
 mod protocol;
@@ -19,6 +24,7 @@ mod server;
 mod stop;
 mod tools;
 
+pub use answer::{MAX_ANSWER_BYTES, MAX_CELL_CHARS};
 pub use gateway::{Failure, Gateway, Listed, Schemas, Session};
 pub use log::{Entry, Log, LogError, Opened};
 pub use refusal::{Refused, code};
diff --git a/crates/tabletist-mcp/src/refusal.rs b/crates/tabletist-mcp/src/refusal.rs
index 3a41d38..ae20892 100644
--- a/crates/tabletist-mcp/src/refusal.rs
+++ b/crates/tabletist-mcp/src/refusal.rs
@@ -96,6 +96,9 @@ impl Refused {
 
 #[cfg(test)]
 mod tests {
+    use tabletist_db::Dialect;
+    use tabletist_db::agent::check_read;
+
     use super::*;
 
     #[test]
@@ -149,4 +152,115 @@ mod tests {
         let gone = Refused::session_ended(Why::Client);
         assert_eq!((gone.code, gone.result()), ("cancelled", "cancelled"));
     }
+
+    fn limits() -> Limits {
+        Limits {
+            rows: 500,
+            timeout: Duration::from_secs(10),
+        }
+    }
+
+    fn database(error: Error, why: Option<Why>) -> Refused {
+        Refused::failed(Failure::Database(error), "Bookshop · dev", limits(), why)
+    }
+
+    #[test]
+    fn a_refusal_of_the_check_keeps_its_code_and_its_sentence() {
+        let refusal = check_read(Dialect::Sqlite, "DELETE FROM book_covers", &[]).unwrap_err();
+        let refused = Refused::from(refusal);
+        assert_eq!(refused.code, "not_read_only");
+        assert_eq!(
+            refused.message,
+            "run_query only runs reads, and this is a DELETE."
+        );
+        assert_eq!(refused.result(), "refused");
+    }
+
+    #[test]
+    fn a_failure_to_open_says_one_thing_whatever_went_wrong() {
+        let said = "Tabletist could not open Bookshop · dev. Open it in Tabletist to see why.";
+        for error in [
+            Error::Connect("db.internal.example:5432 refused".into()),
+            Error::Auth("password authentication failed for user \"shop_admin\"".into()),
+            Error::Tls("certificate of db.internal.example".into()),
+            Error::ConnectionLost("db.internal.example closed".into()),
+            Error::InvalidSpec("choose a SQLite file".into()),
+            Error::Io("/home/me/shop.db is not a database".into()),
+            Error::LeftReadOnly,
+        ] {
+            let refused = database(error, None);
+            assert_eq!(refused.code, "connection_unavailable");
+            assert_eq!(refused.message, said);
+            assert_eq!(refused.result(), "error");
+        }
+        let refused = Refused::failed(Failure::Unavailable, "Bookshop · dev", limits(), None);
+        assert_eq!(refused.message, said);
+    }
+
+    #[test]
+    fn the_databases_own_error_comes_with_its_code_and_its_hint() {
+        let error = Error::Query {
+            code: Some("42703".into()),
+            message: "column \"nmae\" does not exist".into(),
+            detail: None,
+            hint: Some("Perhaps you meant to reference the column \"authors.name\".".into()),
+            named: Box::default(),
+        };
+        let refused = database(error, None);
+        assert_eq!(refused.code, "query_failed");
+        assert_eq!(
+            refused.message,
+            "column \"nmae\" does not exist (code 42703)\n\
+             Hint: Perhaps you meant to reference the column \"authors.name\"."
+        );
+        assert_eq!(refused.result(), "error");
+        // Its words are the agent's to read, and not the log's to keep.
+        assert_eq!(
+            refused.for_the_log(),
+            "The database refused the statement (code 42703)."
+        );
+    }
+
+    #[test]
+    fn a_databases_error_without_a_code_is_kept_without_one() {
+        let refused = database(Error::query("x".repeat(600_000)), None);
+        assert_eq!(refused.message.chars().count(), MAX_SENTENCE_CHARS + 1);
+        assert_eq!(refused.for_the_log(), "The database refused the statement.");
+    }
+
+    #[test]
+    fn a_stopped_call_says_who_stopped_it() {
+        let cases = [
+            (Some(Why::Client), "cancelled", "cancelled"),
+            (Some(Why::Stopped), "server_stopped", "stopped"),
+            // The database ended it, and nobody here asked.
+            (None, "cancelled", "cancelled"),
+        ];
+        for (why, code, result) in cases {
+            let refused = database(Error::Cancelled, why);
+            assert_eq!(refused.code, code);
+            assert_eq!(refused.result(), result);
+        }
+    }
+
+    #[test]
+    fn the_databases_own_timeout_names_the_limit() {
+        let refused = database(Error::Timeout, None);
+        assert_eq!(
+            refused.message,
+            "The query was stopped after 10 s, this connection's limit."
+        );
+        assert_eq!(seconds(Duration::from_millis(1_500)), "1.5 s");
+    }
+
+    #[test]
+    fn a_name_that_matches_nothing_is_repeated_only_so_far() {
+        let refused = Refused::unknown_connection(&"x".repeat(5_000));
+        assert_eq!(refused.code, "unknown_connection");
+        assert!(refused.message.chars().count() < 150);
+        assert_eq!(
+            Refused::unknown_connection("Bookshop · production").message,
+            "No connection named Bookshop · production. list_connections has the names."
+        );
+    }
 }
diff --git a/crates/tabletist-mcp/src/tools.rs b/crates/tabletist-mcp/src/tools.rs
index c858b40..68f976a 100644
--- a/crates/tabletist-mcp/src/tools.rs
+++ b/crates/tabletist-mcp/src/tools.rs
@@ -101,10 +101,19 @@ fn schema(properties: Json, required: &[&str]) -> Json {
 mod tests {
     use super::*;
 
+    #[test]
+    fn the_log_keeps_what_was_sent_up_to_the_size_a_statement_may_have() {
+        assert_eq!(kept("SELECT 1"), "SELECT 1");
+        let long = "é".repeat(tabletist_db::agent::MAX_SQL_BYTES);
+        let kept = kept(&long);
+        assert!(kept.len() <= tabletist_db::agent::MAX_SQL_BYTES);
+        assert!(long.starts_with(&kept));
+    }
+
     #[test]
     fn the_tools_are_told_as_they_are() {
         let names: Vec<&str> = all().iter().map(|tool| tool.name).collect();
-        assert_eq!(names, ["list_connections"]);
+        assert_eq!(names, ["list_connections", "run_query"]);
         for tool in all() {
             assert_eq!(tool.input["type"], json!("object"), "{}", tool.name);
             assert!(!tool.description.is_empty() && !tool.title.is_empty());
@@ -115,7 +124,24 @@ mod tests {
     fn a_tool_that_is_not_there_is_answered_with_the_ones_that_are() {
         assert_eq!(
             Invalid::UnknownTool.message(),
-            "Unknown tool. Tabletist has list_connections."
+            "Unknown tool. Tabletist has list_connections, run_query."
+        );
+    }
+
+    #[test]
+    fn an_argument_is_refused_without_a_word_of_what_it_held() {
+        let given = json!({ "connection": "Bookshop", "sql": ["SELECT secret"] });
+        let given = given.as_object().unwrap();
+        let arguments = Arguments::new("run_query", given, &["connection", "sql"]).unwrap();
+        assert_eq!(
+            arguments.text("sql").unwrap_err().message(),
+            "run_query needs `sql`, a string."
+        );
+        let given = json!({ "limit": "SELECT secret" });
+        let refused = Arguments::new("run_query", given.as_object().unwrap(), &["sql"]);
+        assert_eq!(
+            refused.err().unwrap().message(),
+            "run_query takes sql and nothing else."
         );
     }
 }
diff --git a/crates/tabletist-mcp/src/tools/query.rs b/crates/tabletist-mcp/src/tools/query.rs
new file mode 100644
index 0000000..4c2bc89
--- /dev/null
+++ b/crates/tabletist-mcp/src/tools/query.rs
@@ -0,0 +1,45 @@
+//! `run_query`: one statement that reads.
+
+#[cfg(test)]
+mod tests {
+    use super::*;
+
+    #[test]
+    fn a_value_is_a_string_a_number_a_boolean_or_null() {
+        let given = json!(["it's", 7, -9_223_372_036_854_775_808_i64, 1.5, true, null]);
+        assert_eq!(
+            params(given.as_array().unwrap()).unwrap(),
+            [
+                Param::Text("it's".into()),
+                Param::Int(7),
+                Param::Int(i64::MIN),
+                Param::Float(1.5),
+                Param::Bool(true),
+                Param::Null,
+            ]
+        );
+    }
+
+    #[test]
+    fn a_value_that_is_none_of_them_is_refused_by_its_position() {
+        let cases = [
+            (
+                json!([1, [2]]),
+                "Parameter 2 is a list. A parameter is a string, a number, true, false or null.",
+            ),
+            (
+                json!([{ "a": 1 }]),
+                "Parameter 1 is an object. A parameter is a string, a number, true, false or null.",
+            ),
+            (
+                json!([1, 2, 18_446_744_073_709_551_615_u64]),
+                "Parameter 3 is a whole number too large to send as one. Send it as a string.",
+            ),
+        ];
+        for (given, said) in cases {
+            let refused = params(given.as_array().unwrap()).unwrap_err();
+            assert_eq!(refused.code, "invalid_params");
+            assert_eq!(refused.message, said);
+        }
+    }
+}
diff --git a/crates/tabletist-mcp/tests/tools.rs b/crates/tabletist-mcp/tests/tools.rs
index 8212574..1d908ef 100644
--- a/crates/tabletist-mcp/tests/tools.rs
+++ b/crates/tabletist-mcp/tests/tools.rs
@@ -9,7 +9,9 @@ mod support;
 use rmcp::model::ErrorCode;
 use rmcp::service::ServiceError;
 use serde_json::{Value as Json, json};
-use support::{Agent, Shop, request, server};
+use support::{Agent, LIMITS, Shop, request, server};
+use tabletist_db::agent::Limits;
+use tabletist_mcp::{MAX_ANSWER_BYTES, MAX_CELL_CHARS};
 
 const DEV: &str = "Bookshop · dev";
 
@@ -35,7 +37,7 @@ async fn an_agent_is_told_the_tools_and_that_each_only_reads() {
     let agent = Agent::connect(&server).await;
     let tools = agent.client.list_all_tools().await.unwrap();
     let names: Vec<&str> = tools.iter().map(|tool| tool.name.as_ref()).collect();
-    assert_eq!(names, ["list_connections"]);
+    assert_eq!(names, ["list_connections", "run_query"]);
     for tool in &tools {
         let annotations = tool.annotations.as_ref().unwrap();
         assert_eq!(annotations.read_only_hint, Some(true), "{}", tool.name);
@@ -81,7 +83,7 @@ async fn a_call_that_is_no_tools_is_the_protocols_error() {
     // The sentence never repeats what was sent, here a name.
     assert_eq!(
         invalid(&agent, "drop_everything", json!({})).await,
-        "Unknown tool. Tabletist has list_connections."
+        "Unknown tool. Tabletist has list_connections, run_query."
     );
     assert_eq!(
         invalid(&agent, "list_connections", json!({ "all": true })).await,
@@ -92,3 +94,342 @@ async fn a_call_that_is_no_tools_is_the_protocols_error() {
     agent.close().await;
     assert_eq!(support::entries(&log).await.len(), 1);
 }
+
+async fn query(agent: &Agent, sql: &str) -> support::Called {
+    agent
+        .call("run_query", json!({ "connection": DEV, "sql": sql }))
+        .await
+}
+
+#[tokio::test]
+async fn a_connection_that_is_off_does_not_exist_for_an_agent() {
+    let shop = Shop::new();
+    shop.list("Bookshop · production", &shop.bookshop(), LIMITS);
+    let (server, _log) = server(&shop);
+    let agent = Agent::connect(&server).await;
+    let on_production = json!({ "connection": "Bookshop · production", "sql": "SELECT 1" });
+    assert!(
+        !agent
+            .call("run_query", on_production.clone())
+            .await
+            .is_error
+    );
+
+    // The person turns it Off. The session stays, and the change holds at
+    // the next call.
+    shop.unlist("Bookshop · production");
+    let listed = agent.call("list_connections", json!({})).await;
+    assert_eq!(listed.answer()["connections"].as_array().unwrap().len(), 1);
+    for (tool, arguments) in [
+        ("run_query", on_production),
+        // A connection that never was answers the same.
+        (
+            "run_query",
+            json!({ "connection": "Payroll", "sql": "SELECT 1" }),
+        ),
+    ] {
+        let called = agent.call(tool, arguments).await;
+        assert_eq!(called.refused(), "unknown_connection");
+    }
+    let called = agent
+        .call(
+            "run_query",
+            json!({ "connection": "bookshop · dev", "sql": "SELECT 1" }),
+        )
+        .await;
+    assert_eq!(
+        called.text, "No connection named bookshop · dev. list_connections has the names.",
+        "a name is the whole of it, letter for letter"
+    );
+    agent.close().await;
+}
+
+#[tokio::test]
+async fn run_query_answers_columns_and_rows() {
+    let shop = Shop::new();
+    let (server, _log) = server(&shop);
+    let agent = Agent::connect(&server).await;
+    let called = agent
+        .call(
+            "run_query",
+            json!({
+                "connection": DEV,
+                "sql": "SELECT id, name FROM publishers WHERE id <= ? AND name <> ? ORDER BY id",
+                "params": [2, "it's \"quoted\""],
+            }),
+        )
+        .await;
+    assert_eq!(
+        called.answer(),
+        &json!({
+            "columns": [{ "name": "id", "type": "INTEGER" }, { "name": "name", "type": "TEXT" }],
+            "rows": [[1, "Northlight Books"], [2, "Harbor Press"]],
+            "row_count": 2,
+            "truncated": false,
+            "took_ms": called.answer()["took_ms"],
+        })
+    );
+    assert!(called.answer()["took_ms"].is_u64());
+    agent.close().await;
+}
+
+#[tokio::test]
+async fn run_query_explains_a_query_without_running_it() {
+    let shop = Shop::new();
+    let (server, _log) = server(&shop);
+    let agent = Agent::connect(&server).await;
+    let called = query(
+        &agent,
+        "EXPLAIN QUERY PLAN SELECT * FROM books WHERE id = 1",
+    )
+    .await;
+    let answer = called.answer();
+    assert!(answer["row_count"].as_u64().unwrap() >= 1);
+    assert!(Json::to_string(&answer["rows"]).contains("books"));
+    agent.close().await;
+}
+
+#[tokio::test]
+async fn a_statement_the_check_refuses_never_reaches_the_database() {
+    let shop = Shop::new();
+    let (server, _log) = server(&shop);
+    let agent = Agent::connect(&server).await;
+    let cases = [
+        (
+            "DELETE FROM book_covers WHERE id = 9",
+            "not_read_only",
+            "run_query only runs reads, and this is a DELETE.",
+        ),
+        (
+            "SELECT 1; DELETE FROM book_covers",
+            "too_many_statements",
+            "Send one statement per call.",
+        ),
+        (
+            "CREATE TABLE notes (body TEXT)",
+            "not_allowed",
+            "CREATE TABLE is not available to agents.",
+        ),
+        (
+            "SELECT load_extension('x')",
+            "function_not_allowed",
+            "load_extension is not available to agents.",
+        ),
+        (
+            "SELECT * FROM books WHERE id = ?",
+            "invalid_params",
+            "The statement has 1 parameter and 0 were sent.",
+        ),
+        (
+            "SELECT * FROM books WHERE (",
+            "not_parsed",
+            "Tabletist could not read this statement, so it was not run.",
+        ),
+    ];
+    for (sql, code, said) in cases {
+        let called = query(&agent, sql).await;
+        assert_eq!(
+            (called.refused(), called.text.as_str()),
+            (code, said),
+            "{sql}"
+        );
+        assert!(
+            !called.text.contains("propose_write"),
+            "no sentence names a tool there is not: {}",
+            called.text
+        );
+    }
+    assert_eq!(shop.reads_ended(), 0, "nothing was run");
+    let called = agent
+        .call(
+            "run_query",
+            json!({ "connection": DEV, "sql": "SELECT ?", "params": [[1, 2]] }),
+        )
+        .await;
+    assert_eq!(called.refused(), "invalid_params");
+    agent.close().await;
+}
+
+#[tokio::test]
+async fn the_databases_own_error_is_the_agents_to_read() {
+    let shop = Shop::new();
+    let (server, _log) = server(&shop);
+    let agent = Agent::connect(&server).await;
+    let called = query(&agent, "SELECT nmae FROM authors").await;
+    assert_eq!(called.refused(), "query_failed");
+    assert!(
+        called.text.contains("no such column: nmae"),
+        "{}",
+        called.text
+    );
+    // The session is none the worse for it.
+    assert!(!query(&agent, "SELECT name FROM authors").await.is_error);
+    agent.close().await;
+}
+
+#[tokio::test]
+async fn an_answer_holds_the_connections_rows_at_most_and_says_when_it_was_cut() {
+    let shop = Shop::new();
+    let few = Limits { rows: 3, ..LIMITS };
+    shop.list("Bookshop · few", &shop.bookshop(), few);
+    let (server, _log) = server(&shop);
+    let agent = Agent::connect(&server).await;
+    let on_few = |sql: &str| json!({ "connection": "Bookshop · few", "sql": sql });
+    let called = agent
+        .call("run_query", on_few("SELECT id FROM books ORDER BY id"))
+        .await;
+    let answer = called.answer();
+    assert_eq!(answer["rows"], json!([[1], [2], [3]]));
+    assert_eq!(answer["row_count"], json!(3));
+    assert_eq!(answer["truncated"], json!(true));
+    assert_eq!(answer["truncated_by"], json!("row_limit"));
+
+    // Exactly as many rows as the limit is every row.
+    let called = agent
+        .call(
+            "run_query",
+            on_few("SELECT id FROM books ORDER BY id LIMIT 3"),
+        )
+        .await;
+    assert_eq!(called.answer()["truncated"], json!(false));
+    assert!(called.answer().get("truncated_by").is_none());
+    agent.close().await;
+}
+
+#[tokio::test]
+async fn an_answer_is_kept_under_its_size() {
+    let shop = Shop::new();
+    let (server, _log) = server(&shop);
+    let agent = Agent::connect(&server).await;
+    // 400 rows of 1,000 characters: more than an answer may weigh.
+    let called = query(
+        &agent,
+        "WITH RECURSIVE n(i) AS (SELECT 1 UNION ALL SELECT i + 1 FROM n WHERE i < 400) \
+         SELECT i, printf('%.*c', 1000, 'x') FROM n",
+    )
+    .await;
+    let answer = called.answer();
+    assert_eq!(answer["truncated"], json!(true));
+    assert_eq!(answer["truncated_by"], json!("size"));
+    let rows = answer["row_count"].as_u64().unwrap();
+    assert!((200..300).contains(&rows), "{rows}");
+    assert_eq!(
+        answer["rows"][0][0],
+        json!(1),
+        "the first rows are the ones kept"
+    );
+    assert!(
+        called.text.len() <= MAX_ANSWER_BYTES,
+        "{}",
+        called.text.len()
+    );
+    assert!(called.text.len() > MAX_ANSWER_BYTES - 2_000);
+    agent.close().await;
+}
+
+#[tokio::test]
+async fn an_answer_is_sent_once() {
+    let shop = Shop::new();
+    let (server, _log) = server(&shop);
+    let agent = Agent::connect(&server).await;
+    let before = agent.received().len();
+    let called = query(
+        &agent,
+        "WITH RECURSIVE n(i) AS (SELECT 1 UNION ALL SELECT i + 1 FROM n WHERE i < 400) \
+         SELECT i, printf('%.*c', 1000, 'x') FROM n",
+    )
+    .await;
+    assert_eq!(called.answer()["truncated_by"], json!("size"));
+    // What went over the transport is the answer, with a quote mark
+    // escaped here and there, and not the answer twice.
+    let sent = agent.received().len() - before;
+    assert!(sent < MAX_ANSWER_BYTES * 5 / 4, "{sent}");
+    agent.close().await;
+}
+
+#[tokio::test]
+async fn columns_that_are_too_much_by_themselves_are_refused() {
+    let shop = Shop::new();
+    let (server, _log) = server(&shop);
+    let agent = Agent::connect(&server).await;
+    // A short statement with a long answer and no row in it: two hundred
+    // columns, each named by two thousand letters.
+    let stars = vec!["*"; 200].join(", ");
+    let sql = format!(
+        "WITH t AS (SELECT 1 AS {}) SELECT {stars} FROM t WHERE 1 = 0",
+        "a".repeat(2_000)
+    );
+    let called = query(&agent, &sql).await;
+    assert_eq!(called.refused(), "too_large");
+    assert_eq!(
+        called.text,
+        "The answer's columns alone are more than an answer may hold. Select fewer columns, \
+         or give them shorter names."
+    );
+    agent.close().await;
+}
+
+#[tokio::test]
+async fn a_databases_error_is_only_so_long() {
+    let shop = Shop::new();
+    let (server, _log) = server(&shop);
+    let agent = Agent::connect(&server).await;
+    // SQLite repeats the path it could not read, all 600,000 letters.
+    let called = query(
+        &agent,
+        "SELECT json_extract('{}', printf('%.*c', 600000, 'x'))",
+    )
+    .await;
+    assert_eq!(called.refused(), "query_failed");
+    assert!(called.text.len() < 2_100, "{}", called.text.len());
+    agent.close().await;
+}
+
+#[tokio::test]
+async fn a_long_text_is_cut_and_a_binary_value_described() {
+    let shop = Shop::new();
+    let (server, _log) = server(&shop);
+    let agent = Agent::connect(&server).await;
+    let called = query(
+        &agent,
+        "SELECT printf('%.*c', 5000, 'x'), x'deadbeef', NULL",
+    )
+    .await;
+    let row = &called.answer()["rows"][0];
+    assert_eq!(
+        row[0],
+        json!({ "cut": "x".repeat(MAX_CELL_CHARS), "characters": 5000 })
+    );
+    assert_eq!(row[1], json!({ "binary_bytes": 4 }));
+    assert_eq!(row[2], Json::Null);
+    agent.close().await;
+}
+
+#[tokio::test]
+async fn run_query_takes_its_arguments_and_no_others() {
+    let shop = Shop::new();
+    let (server, _log) = server(&shop);
+    let agent = Agent::connect(&server).await;
+    let cases = [
+        (
+            json!({ "connection": DEV }),
+            "run_query needs `sql`, a string.",
+        ),
+        (
+            json!({ "connection": DEV, "sql": ["SELECT 'secret'"] }),
+            "run_query needs `sql`, a string.",
+        ),
+        (
+            json!({ "connection": DEV, "sql": "SELECT 1", "limit": 5 }),
+            "run_query takes connection, sql, params and nothing else.",
+        ),
+        (
+            json!({ "connection": DEV, "sql": "SELECT ?", "params": "secret" }),
+            "run_query: `params` is a list.",
+        ),
+    ];
+    for (arguments, said) in cases {
+        assert_eq!(invalid(&agent, "run_query", arguments).await, said);
+    }
+    agent.close().await;
+}
````

- [ ] **Step 2: Run them to see that they do not build**

Run: `cargo test --locked -p tabletist-mcp`

Expected: it does not build. The first of the errors, and the kind of the rest:

```text
error[E0432]: unresolved imports `answer::MAX_ANSWER_BYTES`, `answer::MAX_CELL_CHARS`
error: could not compile `tabletist-mcp` (lib) due to 1 previous error
error[E0425]: cannot find type `Answer` in this scope
error[E0425]: cannot find type `Written` in this scope
error[E0433]: cannot find type `Arguments` in this scope
error[E0599]: no associated function or constant named `failed` found for struct `refusal::Refused` in the current scope
```

- [ ] **Step 3: The tool, and the answer under its size**

````diff
diff --git a/crates/tabletist-mcp/src/answer.rs b/crates/tabletist-mcp/src/answer.rs
index 2636175..4a3b454 100644
--- a/crates/tabletist-mcp/src/answer.rs
+++ b/crates/tabletist-mcp/src/answer.rs
@@ -2,6 +2,144 @@
 //! limit does not bound an answer, and what is sent goes into the agent's
 //! context.
 
+use serde_json::{Map, Value as Json, json};
+use tabletist_db::Value;
+use tabletist_db::agent::Answer;
+
+/// The most characters of one text value. A longer one is cut, and says
+/// how long it was.
+pub const MAX_CELL_CHARS: usize = 2_000;
+
+/// The most bytes of an answer as JSON. Past it, rows are left out from
+/// the end.
+pub const MAX_ANSWER_BYTES: usize = 256_000;
+
+/// Why an answer does not hold every row of the statement.
+#[derive(Debug, Clone, Copy, PartialEq, Eq)]
+pub enum Cut {
+    /// The statement had more rows than the connection lets an answer hold.
+    RowLimit,
+    /// The rows did not fit [`MAX_ANSWER_BYTES`].
+    Size,
+}
+
+impl Cut {
+    pub fn as_str(self) -> &'static str {
+        match self {
+            Self::RowLimit => "row_limit",
+            Self::Size => "size",
+        }
+    }
+}
+
+/// An answer, written.
+#[derive(Debug, Clone, PartialEq)]
+pub struct Written {
+    /// `columns`, `rows`, `row_count`, `truncated`, `truncated_by` when it
+    /// is, and `took_ms`.
+    pub json: Map<String, Json>,
+    /// The rows that were sent.
+    pub rows: usize,
+    pub columns: usize,
+    pub cut: Option<Cut>,
+}
+
+/// An answer whose columns alone, before its first row, are more than an
+/// answer may weigh.
+#[derive(Debug, Clone, Copy, PartialEq, Eq)]
+pub struct TooWide;
+
+/// Writes `answer` within `max_bytes`, which is [`MAX_ANSWER_BYTES`]
+/// everywhere but in a test.
+pub fn write(answer: &Answer, max_bytes: usize) -> Result<Written, TooWide> {
+    let columns: Vec<Json> = answer
+        .columns
+        .iter()
+        .map(|column| {
+            let mut entry = Map::new();
+            entry.insert("name".into(), json!(column.name));
+            if !column.type_name.is_empty() {
+                entry.insert("type".into(), json!(column.type_name));
+            }
+            Json::Object(entry)
+        })
+        .collect();
+    let took_ms = u64::try_from(answer.elapsed.as_millis()).unwrap_or(u64::MAX);
+    // What the answer weighs before its first row, with the longest words
+    // it may come to hold.
+    let frame = json!({
+        "columns": columns,
+        "rows": [],
+        "row_count": answer.rows.len(),
+        "truncated": true,
+        "truncated_by": Cut::RowLimit.as_str(),
+        "took_ms": took_ms,
+    });
+    let mut bytes = frame.to_string().len();
+    if bytes > max_bytes {
+        return Err(TooWide);
+    }
+    let mut rows = Vec::with_capacity(answer.rows.len());
+    let mut cut = answer.truncated.then_some(Cut::RowLimit);
+    for row in &answer.rows {
+        let row = Json::Array(row.iter().map(cell).collect());
+        // The row, and the comma between it and the one before.
+        bytes += row.to_string().len() + usize::from(!rows.is_empty());
+        if bytes > max_bytes {
+            cut = Some(Cut::Size);
+            break;
+        }
+        rows.push(row);
+    }
+    let sent = rows.len();
+    let mut json = Map::new();
+    json.insert("columns".into(), Json::Array(columns));
+    json.insert("rows".into(), Json::Array(rows));
+    json.insert("row_count".into(), json!(sent));
+    json.insert("truncated".into(), json!(cut.is_some()));
+    if let Some(cut) = cut {
+        json.insert("truncated_by".into(), json!(cut.as_str()));
+    }
+    json.insert("took_ms".into(), json!(took_ms));
+    Ok(Written {
+        json,
+        rows: sent,
+        columns: answer.columns.len(),
+        cut,
+    })
+}
+
+/// One value. A database's value is null, true or false, a number or a
+/// text, so an object is never one: it stands for a value that was not
+/// sent whole.
+fn cell(value: &Value) -> Json {
+    match value {
+        Value::Null => Json::Null,
+        Value::Bool(value) => json!(value),
+        Value::Int(value) => json!(value),
+        Value::Float(value) => match serde_json::Number::from_f64(*value) {
+            Some(number) => Json::Number(number),
+            // JSON has no word for these.
+            None if value.is_nan() => json!("NaN"),
+            None if *value > 0.0 => json!("Infinity"),
+            None => json!("-Infinity"),
+        },
+        Value::Text(value) => text(value),
+        Value::Bytes(bytes) => json!({ "binary_bytes": bytes.len() }),
+    }
+}
+
+/// A text, cut where it is longer than [`MAX_CELL_CHARS`].
+pub(crate) fn text(text: &str) -> Json {
+    match text.char_indices().nth(MAX_CELL_CHARS) {
+        Some((end, _)) => json!({
+            "cut": &text[..end],
+            "characters": text.chars().count(),
+        }),
+        None => json!(text),
+    }
+}
+
 #[cfg(test)]
 mod tests {
     use std::time::Duration;
diff --git a/crates/tabletist-mcp/src/refusal.rs b/crates/tabletist-mcp/src/refusal.rs
index ae20892..04b9635 100644
--- a/crates/tabletist-mcp/src/refusal.rs
+++ b/crates/tabletist-mcp/src/refusal.rs
@@ -2,6 +2,12 @@
 //! sentence it can act on. A refusal is a tool's result, not a protocol
 //! error, so that the model reads it.
 
+use std::time::Duration;
+
+use tabletist_db::Error;
+use tabletist_db::agent::Limits;
+
+use crate::gateway::Failure;
 use crate::stop::Why;
 
 /// The codes of this crate's own refusals. The check's are its own
@@ -30,6 +36,9 @@ pub struct Refused {
     logged: Option<String>,
 }
 
+/// The most of an agent's own text a sentence repeats.
+const MAX_ECHO_CHARS: usize = 80;
+
 /// The most characters of a sentence. A database writes its errors as long
 /// as it likes, and with what it likes in them.
 pub const MAX_SENTENCE_CHARS: usize = 2_000;
@@ -53,6 +62,32 @@ impl Refused {
         self.logged.as_deref().unwrap_or(&self.message)
     }
 
+    /// The name matches nothing an agent may see. A connection that is Off
+    /// answers exactly this.
+    pub(crate) fn unknown_connection(name: &str) -> Self {
+        let name: String = name.chars().take(MAX_ECHO_CHARS).collect();
+        Self::new(
+            code::UNKNOWN_CONNECTION,
+            format!("No connection named {name}. list_connections has the names."),
+        )
+    }
+
+    /// One sentence whatever went wrong: the reason can name a host, a
+    /// user or a file.
+    pub(crate) fn unavailable(connection: &str) -> Self {
+        Self::new(
+            code::CONNECTION_UNAVAILABLE,
+            format!("Tabletist could not open {connection}. Open it in Tabletist to see why."),
+        )
+    }
+
+    pub(crate) fn busy() -> Self {
+        Self::new(
+            code::BUSY,
+            "Another agent's query is running on this connection. Try again.",
+        )
+    }
+
     /// A session has as many calls running as it may.
     pub(crate) fn too_many_calls() -> Self {
         Self::new(
@@ -61,6 +96,16 @@ impl Refused {
         )
     }
 
+    pub(crate) fn timeout(limit: Duration) -> Self {
+        Self::new(
+            code::TIMEOUT,
+            format!(
+                "The query was stopped after {}, this connection's limit.",
+                seconds(limit)
+            ),
+        )
+    }
+
     pub(crate) fn cancelled() -> Self {
         Self::new(code::CANCELLED, "The call was cancelled.")
     }
@@ -82,6 +127,70 @@ impl Refused {
         }
     }
 
+    /// What the gateway's failure is to the agent. `why` is the reason the
+    /// call was stopped for, when it was.
+    pub(crate) fn failed(
+        failure: Failure,
+        connection: &str,
+        limits: Limits,
+        why: Option<Why>,
+    ) -> Self {
+        let error = match failure {
+            Failure::Unavailable => return Self::unavailable(connection),
+            Failure::Busy => return Self::busy(),
+            Failure::Database(error) => error,
+        };
+        match error {
+            // SQLite read the statement as something the check did not.
+            Error::Agent(refusal) => refusal.into(),
+            Error::Timeout => Self::timeout(limits.timeout),
+            Error::Cancelled => match why {
+                Some(Why::Stopped) => Self::stopped(),
+                // Or the database ended it, and nobody here asked.
+                Some(Why::Client) | None => Self::cancelled(),
+            },
+            Error::Query {
+                code,
+                message,
+                detail,
+                hint,
+                ..
+            } => {
+                let coded = match &code {
+                    Some(code) => format!(" (code {code})"),
+                    None => String::new(),
+                };
+                let mut sentence = format!("{message}{coded}");
+                for (label, line) in [("Detail", detail), ("Hint", hint)] {
+                    if let Some(line) = line {
+                        sentence.push_str(&format!("\n{label}: {line}"));
+                    }
+                }
+                // A database quotes what it could not read, and that can
+                // be a row's value. The agent may read the row. The log
+                // holds no row, so it keeps the error's code alone.
+                Self {
+                    logged: Some(format!("The database refused the statement{coded}.")),
+                    ..Self::new(code::QUERY_FAILED, sentence)
+                }
+            }
+            // Each of these is about the session, and its words may hold
+            // what an agent is not told.
+            Error::Connect(_)
+            | Error::Auth(_)
+            | Error::Tls(_)
+            | Error::Ssh { .. }
+            | Error::ConnectionLost(_)
+            | Error::InvalidSpec(_)
+            | Error::Unsupported(_)
+            | Error::Io(_)
+            | Error::Refused { .. }
+            | Error::ReadOnly
+            | Error::LeftReadOnly
+            | Error::LeftTransaction => Self::unavailable(connection),
+        }
+    }
+
     /// What the log calls the call's end.
     pub(crate) fn result(&self) -> &'static str {
         match self.code {
@@ -94,6 +203,22 @@ impl Refused {
     }
 }
 
+impl From<tabletist_db::agent::Refusal> for Refused {
+    fn from(refusal: tabletist_db::agent::Refusal) -> Self {
+        Self::new(refusal.code(), refusal.to_string())
+    }
+}
+
+/// "10 s", "1.5 s".
+fn seconds(time: Duration) -> String {
+    let millis = time.as_millis();
+    if millis.is_multiple_of(1_000) {
+        format!("{} s", millis / 1_000)
+    } else {
+        format!("{} s", time.as_secs_f64())
+    }
+}
+
 #[cfg(test)]
 mod tests {
     use tabletist_db::Dialect;
diff --git a/crates/tabletist-mcp/src/server.rs b/crates/tabletist-mcp/src/server.rs
index 3427a2f..cf9df6c 100644
--- a/crates/tabletist-mcp/src/server.rs
+++ b/crates/tabletist-mcp/src/server.rs
@@ -308,7 +308,7 @@ impl<G: Gateway> Server<G> {
             Some(why) => refuse(Refused::session_ended(why)),
             None if before >= MAX_CALLS_AT_ONCE => refuse(Refused::too_many_calls()),
             None => {
-                let work = tools::call(&self.inner.gateway, name, arguments);
+                let work = tools::call(&self.inner.gateway, &stop, name, arguments);
                 tokio::pin!(work);
                 tokio::select! {
                     done = &mut work => done,
diff --git a/crates/tabletist-mcp/src/stop.rs b/crates/tabletist-mcp/src/stop.rs
index 663b814..176bc58 100644
--- a/crates/tabletist-mcp/src/stop.rs
+++ b/crates/tabletist-mcp/src/stop.rs
@@ -74,6 +74,12 @@ impl Stop {
         nearest
     }
 
+    /// For a gateway, which ends what a session is doing when this is
+    /// cancelled.
+    pub fn token(&self) -> &CancellationToken {
+        &self.token
+    }
+
     pub async fn ended(&self) {
         self.token.cancelled().await;
     }
diff --git a/crates/tabletist-mcp/src/tools.rs b/crates/tabletist-mcp/src/tools.rs
index 68f976a..731e0aa 100644
--- a/crates/tabletist-mcp/src/tools.rs
+++ b/crates/tabletist-mcp/src/tools.rs
@@ -3,13 +3,15 @@
 //! the wire.
 
 mod connections;
+mod query;
 
 use std::time::Duration;
 
 use serde_json::{Map, Value as Json, json};
 
-use crate::gateway::Gateway;
+use crate::gateway::{Failure, Gateway, Listed};
 use crate::refusal::Refused;
+use crate::stop::{Stop, Why};
 
 /// A tool as an agent is told of it.
 #[derive(Debug, Clone, PartialEq)]
@@ -23,7 +25,7 @@ pub struct Tool {
 
 /// Every tool. Each of them only reads.
 pub fn all() -> Vec<Tool> {
-    vec![connections::tool()]
+    vec![connections::tool(), query::tool()]
 }
 
 /// A call that is not one of a tool's: the protocol's error, not a tool's
@@ -77,15 +79,90 @@ pub(crate) struct Done {
 /// Runs the tool called `name`.
 pub(crate) async fn call<G: Gateway>(
     gateway: &G,
+    stop: &Stop,
     name: &str,
     arguments: &Map<String, Json>,
 ) -> Result<Done, Invalid> {
     match name {
         connections::NAME => connections::call(gateway, arguments),
+        query::NAME => query::call(gateway, stop, arguments).await,
         _ => Err(Invalid::UnknownTool),
     }
 }
 
+/// A tool's arguments, read strictly: an argument a tool does not have is
+/// refused, so that an agent learns it was not used.
+struct Arguments<'a> {
+    tool: &'static str,
+    given: &'a Map<String, Json>,
+}
+
+impl<'a> Arguments<'a> {
+    fn new(
+        tool: &'static str,
+        given: &'a Map<String, Json>,
+        known: &[&str],
+    ) -> Result<Self, Invalid> {
+        if given.keys().any(|key| !known.contains(&key.as_str())) {
+            return Err(Invalid::Arguments(format!(
+                "{tool} takes {} and nothing else.",
+                known.join(", ")
+            )));
+        }
+        Ok(Self { tool, given })
+    }
+
+    fn text(&self, key: &str) -> Result<&'a str, Invalid> {
+        self.given
+            .get(key)
+            .and_then(Json::as_str)
+            .ok_or_else(|| Invalid::Arguments(format!("{} needs `{key}`, a string.", self.tool)))
+    }
+
+    /// An argument that may be left out, or null.
+    fn list(&self, key: &str) -> Result<Option<&'a [Json]>, Invalid> {
+        match self.given.get(key) {
+            None | Some(Json::Null) => Ok(None),
+            Some(Json::Array(items)) => Ok(Some(items)),
+            Some(_) => Err(Invalid::Arguments(format!(
+                "{}: `{key}` is a list.",
+                self.tool
+            ))),
+        }
+    }
+}
+
+/// The connection an agent named. The name is the whole of it: a name
+/// that is not listed is unknown, whatever else it is.
+fn find<G: Gateway>(gateway: &G, name: &str) -> Result<Listed, Refused> {
+    gateway
+        .connections()
+        .into_iter()
+        .find(|listed| listed.name == name)
+        .ok_or_else(|| Refused::unknown_connection(name))
+}
+
+/// What the work on a connection comes to. A call the person stopped
+/// answers that, whatever it had got to: an agent that was stopped is
+/// given nothing more.
+fn settle<T>(result: Result<T, Failure>, stop: &Stop, listed: &Listed) -> Result<T, Refused> {
+    match (result, stop.why()) {
+        (_, Some(Why::Stopped)) => Err(Refused::stopped()),
+        (Ok(done), _) => Ok(done),
+        (Err(failure), why) => Err(Refused::failed(failure, &listed.name, listed.limits, why)),
+    }
+}
+
+/// What an agent sent, as the log keeps it: as it came, up to the size a
+/// statement may have.
+fn kept(text: &str) -> String {
+    let mut end = text.len().min(tabletist_db::agent::MAX_SQL_BYTES);
+    while !text.is_char_boundary(end) {
+        end -= 1;
+    }
+    text[..end].to_owned()
+}
+
 /// The schema of an object: `properties`, all of them strings unless said
 /// otherwise, of which `required` must be there.
 fn schema(properties: Json, required: &[&str]) -> Json {
diff --git a/crates/tabletist-mcp/src/tools/query.rs b/crates/tabletist-mcp/src/tools/query.rs
index 4c2bc89..9595d50 100644
--- a/crates/tabletist-mcp/src/tools/query.rs
+++ b/crates/tabletist-mcp/src/tools/query.rs
@@ -1,5 +1,148 @@
 //! `run_query`: one statement that reads.
 
+use serde_json::{Map, Value as Json, json};
+use tabletist_db::agent::{self, Param};
+
+use super::{Arguments, Done, Invalid, Note, Tool, find, kept, schema, settle};
+use crate::answer::{self, MAX_ANSWER_BYTES};
+use crate::gateway::{Gateway, Session};
+use crate::refusal::{Refused, code};
+use crate::stop::Stop;
+
+pub const NAME: &str = "run_query";
+
+pub fn tool() -> Tool {
+    Tool {
+        name: NAME,
+        title: "Run a query",
+        description: "Runs one SELECT (or WITH … SELECT, VALUES, or an EXPLAIN of one) on a \
+                      connection, in a read-only transaction that is rolled back. Nothing it \
+                      runs can change data. Write values as parameters, in the form \
+                      list_connections gives for the connection, and send them in `params`. \
+                      The answer has `columns` and `rows`, at most the connection's rows and \
+                      within its timeout; `truncated` says that rows were left out, and \
+                      `truncated_by` whether by the row limit or by the answer's size. A text \
+                      longer than 2,000 characters comes as {\"cut\": its start, \
+                      \"characters\": its length}, and a binary value as {\"binary_bytes\": \
+                      its size}.",
+        input: schema(
+            json!({
+                "connection": {
+                    "type": "string",
+                    "description": "The connection's name, from list_connections.",
+                },
+                "sql": {
+                    "type": "string",
+                    "description": "One statement that reads.",
+                },
+                "params": {
+                    "type": "array",
+                    "description": "The values for the statement's parameters, in order.",
+                    "items": { "type": ["string", "number", "boolean", "null"] },
+                },
+            }),
+            &["connection", "sql"],
+        ),
+    }
+}
+
+/// How the log says a read was run.
+const RAN_AS: &str = "read-only transaction";
+
+pub async fn call<G: Gateway>(
+    gateway: &G,
+    stop: &Stop,
+    arguments: &Map<String, Json>,
+) -> Result<Done, Invalid> {
+    let arguments = Arguments::new(NAME, arguments, &["connection", "sql", "params"])?;
+    let connection = arguments.text("connection")?;
+    let sql = arguments.text("sql")?;
+    let given = arguments.list("params")?.unwrap_or_default();
+    let mut note = Note {
+        sql: Some(kept(sql)),
+        params: (!given.is_empty()).then(|| kept(&Json::Array(given.to_vec()).to_string())),
+        ..Note::default()
+    };
+    let answer = run(gateway, stop, connection, sql, given, &mut note).await;
+    Ok(Done { answer, note })
+}
+
+async fn run<G: Gateway>(
+    gateway: &G,
+    stop: &Stop,
+    connection: &str,
+    sql: &str,
+    given: &[Json],
+    note: &mut Note,
+) -> Result<Map<String, Json>, Refused> {
+    let listed = find(gateway, connection)?;
+    note.connection = Some((listed.id.clone(), listed.name.clone()));
+    let params = params(given)?;
+    // The check parses on a thread of its own and blocks until it has.
+    let (dialect, text) = (listed.dialect, sql.to_owned());
+    let read = tokio::task::spawn_blocking(move || agent::check_read(dialect, &text, &params))
+        .await
+        .map_err(|_| Refused::from(agent::Refusal::NotParsed))??;
+    let session = settle(gateway.open(&listed.id, stop.token()).await, stop, &listed)?;
+    note.ran_as = Some(RAN_AS);
+    let limits = listed.limits;
+    let answer = session.read(&read, limits).await;
+    let answer = settle(answer, stop, &listed)?;
+    let written = answer::write(&answer, MAX_ANSWER_BYTES).map_err(|_| {
+        Refused::new(
+            code::TOO_LARGE,
+            "The answer's columns alone are more than an answer may hold. Select fewer \
+             columns, or give them shorter names.",
+        )
+    })?;
+    note.rows = Some(written.rows as u64);
+    note.columns = Some(written.columns as u64);
+    note.matched = written.cut.is_none().then_some(written.rows as u64);
+    note.took = Some(answer.elapsed);
+    Ok(written.json)
+}
+
+/// An agent's values as the check takes them. A value is a string, a
+/// number, true, false or null.
+fn params(given: &[Json]) -> Result<Vec<Param>, Refused> {
+    given
+        .iter()
+        .enumerate()
+        .map(|(index, value)| {
+            let position = index + 1;
+            let refused = |what: &str| {
+                Refused::new(
+                    code::INVALID_PARAMS,
+                    format!("Parameter {position} is {what}."),
+                )
+            };
+            match value {
+                Json::Null => Ok(Param::Null),
+                Json::Bool(value) => Ok(Param::Bool(*value)),
+                Json::String(value) => Ok(Param::Text(value.clone())),
+                Json::Number(number) => match (number.as_i64(), number.as_u64()) {
+                    (Some(value), _) => Ok(Param::Int(value)),
+                    // Past what a signed 64-bit integer holds, and no
+                    // driver here sends an unsigned one.
+                    (None, Some(_)) => Err(refused(
+                        "a whole number too large to send as one. Send it as a string",
+                    )),
+                    (None, None) => number
+                        .as_f64()
+                        .map(Param::Float)
+                        .ok_or_else(|| refused("not a number Tabletist can send")),
+                },
+                Json::Array(_) => Err(refused(
+                    "a list. A parameter is a string, a number, true, false or null",
+                )),
+                Json::Object(_) => Err(refused(
+                    "an object. A parameter is a string, a number, true, false or null",
+                )),
+            }
+        })
+        .collect()
+}
+
 #[cfg(test)]
 mod tests {
     use super::*;
````

- [ ] **Step 4: Run the tests**

Run: `cargo test --locked -p tabletist-mcp`

Expected: PASS.

```text
     Running unittests src/lib.rs
test result: ok. 39 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
     Running tests/sessions.rs
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
     Running tests/tools.rs
test result: ok. 15 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

- [ ] **Step 5: Commit**

```bash
cargo fmt --all
cargo clippy --locked --workspace --all-targets -- -D warnings
git add crates/tabletist-mcp
git commit -m "Run an agent's read, and keep its answer under a size"
```

---

### Task 5: A call that does not end by itself

Four things end a call: the connection's time, the client's cancel, a client that is gone, and the person's stop. The last three are in the server since Task 3 and had no tool slow enough to test them on. This task adds the first, and the tests of all four.

`within` awaits the work on a connection and, two seconds past the connection's time, ends the call through the gateway and goes on awaiting it. The database's own timeout comes first and says more, so the timer is behind it: it is for a database that did not keep its word. Its own two tests run on a paused clock.

`tests/stops.rs` runs on the real clock, on a statement that takes minutes if nothing ends it (`support::slow`). Besides the four ends it has what a client can do to a session: close its writing end and wait, read nothing, send a message without an end, and send more calls than a session runs.

**Files:**
- Create: `crates/tabletist-mcp/tests/stops.rs`
- Modify: `crates/tabletist-mcp/Cargo.toml` (a paused clock for the tests), `crates/tabletist-mcp/src/stop.rs`, `refusal.rs`, `tools.rs`, `tools/query.rs`, `crates/tabletist-mcp/tests/tools.rs`

- [ ] **Step 1: Write the failing tests**

````diff
diff --git a/crates/tabletist-mcp/Cargo.toml b/crates/tabletist-mcp/Cargo.toml
index 8362a18..67e45b2 100644
--- a/crates/tabletist-mcp/Cargo.toml
+++ b/crates/tabletist-mcp/Cargo.toml
@@ -43,7 +43,9 @@ log = "0.4"
 [dev-dependencies]
 # The SDK's client, for the tests that call the server as an agent does.
 rmcp = { version = "3.5", default-features = false, features = ["client"] }
-# The tests' runtime, on more than one thread, as the app's is.
-tokio = { version = "1", features = ["rt-multi-thread", "macros", "time"] }
+# The tests' runtime, on more than one thread, as the app's is. `test-util`:
+# a paused clock for the test of the server's own timer, which waits ten
+# seconds and two more.
+tokio = { version = "1", features = ["rt-multi-thread", "macros", "time", "test-util"] }
 # A directory of its own for each test's files.
 tempfile = "3"
diff --git a/crates/tabletist-mcp/src/refusal.rs b/crates/tabletist-mcp/src/refusal.rs
index 04b9635..8119b9d 100644
--- a/crates/tabletist-mcp/src/refusal.rs
+++ b/crates/tabletist-mcp/src/refusal.rs
@@ -360,6 +360,7 @@ mod tests {
             (Some(Why::Stopped), "server_stopped", "stopped"),
             // The database ended it, and nobody here asked.
             (None, "cancelled", "cancelled"),
+            (Some(Why::Timeout), "timeout", "timeout"),
         ];
         for (why, code, result) in cases {
             let refused = database(Error::Cancelled, why);
diff --git a/crates/tabletist-mcp/src/stop.rs b/crates/tabletist-mcp/src/stop.rs
index 176bc58..f4b5dee 100644
--- a/crates/tabletist-mcp/src/stop.rs
+++ b/crates/tabletist-mcp/src/stop.rs
@@ -142,4 +142,20 @@ mod tests {
         call.end(Why::Client);
         assert_eq!(call.why(), Some(Why::Stopped));
     }
+
+    #[test]
+    fn a_timer_gives_way_to_the_person_and_to_nobody_else() {
+        let session = Stop::new();
+        let call = session.child();
+        call.end(Why::Timeout);
+        session.end(Why::Stopped);
+        assert_eq!(call.why(), Some(Why::Stopped));
+
+        // A client that is gone does not outrank the call's own timer.
+        let session = Stop::new();
+        let call = session.child();
+        call.end(Why::Timeout);
+        session.end(Why::Client);
+        assert_eq!(call.why(), Some(Why::Timeout));
+    }
 }
diff --git a/crates/tabletist-mcp/src/tools.rs b/crates/tabletist-mcp/src/tools.rs
index 731e0aa..7979bb3 100644
--- a/crates/tabletist-mcp/src/tools.rs
+++ b/crates/tabletist-mcp/src/tools.rs
@@ -178,6 +178,40 @@ fn schema(properties: Json, required: &[&str]) -> Json {
 mod tests {
     use super::*;
 
+    fn ten_seconds() -> Limits {
+        Limits {
+            rows: 500,
+            timeout: Duration::from_secs(10),
+        }
+    }
+
+    #[tokio::test(start_paused = true)]
+    async fn work_that_runs_past_its_time_is_ended_and_awaited_all_the_same() {
+        let stop = Stop::new();
+        // Work that ends only once it is told to, as a gateway's does.
+        let work = async {
+            stop.ended().await;
+            "ended"
+        };
+        let started = tokio::time::Instant::now();
+        assert_eq!(within(&stop, ten_seconds(), work).await, "ended");
+        assert_eq!(started.elapsed(), Duration::from_secs(10) + GRACE);
+        assert_eq!(stop.why(), Some(Why::Timeout));
+    }
+
+    #[tokio::test(start_paused = true)]
+    async fn work_the_database_ends_in_its_own_time_is_left_alone() {
+        let stop = Stop::new();
+        // Past the connection's time, and within what the database is given
+        // to end the statement by itself.
+        let work = async {
+            tokio::time::sleep(Duration::from_secs(11)).await;
+            7
+        };
+        assert_eq!(within(&stop, ten_seconds(), work).await, 7);
+        assert_eq!(stop.why(), None);
+    }
+
     #[test]
     fn the_log_keeps_what_was_sent_up_to_the_size_a_statement_may_have() {
         assert_eq!(kept("SELECT 1"), "SELECT 1");
diff --git a/crates/tabletist-mcp/tests/stops.rs b/crates/tabletist-mcp/tests/stops.rs
new file mode 100644
index 0000000..4031cff
--- /dev/null
+++ b/crates/tabletist-mcp/tests/stops.rs
@@ -0,0 +1,314 @@
+//! How a call ends that does not end by itself: the connection's time, the
+//! client's cancel, a client that is gone, and the person's stop.
+
+// `allow-unwrap-in-tests` does not cover helpers in an integration-test crate.
+#![allow(clippy::unwrap_used)]
+
+mod support;
+
+use std::time::{Duration, Instant};
+
+use rmcp::model::{CallToolRequest, ClientRequest};
+use rmcp::service::{PeerRequestOptions, ServiceError};
+use serde_json::json;
+use support::{Agent, LIMITS, Raw, Shop, entries, request, server, slow};
+use tabletist_db::agent::Limits;
+use tabletist_mcp::Log;
+use tokio::io::AsyncWriteExt;
+
+const BRIEF: &str = "Bookshop · brief";
+
+/// A Bookshop whose statements may take half a second.
+fn brief_shop() -> Shop {
+    let shop = Shop::new();
+    let brief = Limits {
+        timeout: Duration::from_millis(500),
+        ..LIMITS
+    };
+    shop.list(BRIEF, &shop.bookshop(), brief);
+    shop
+}
+
+fn long_query() -> serde_json::Value {
+    json!({ "connection": BRIEF, "sql": slow(6) })
+}
+
+/// Waits for the log to hold `count` entries.
+async fn logged(log: &Log, count: usize) -> Vec<tabletist_mcp::Entry> {
+    let deadline = Instant::now() + Duration::from_secs(20);
+    loop {
+        let entries = entries(log).await;
+        if entries.len() >= count {
+            return entries;
+        }
+        assert!(Instant::now() < deadline, "the log has {}", entries.len());
+        tokio::time::sleep(Duration::from_millis(20)).await;
+    }
+}
+
+#[tokio::test]
+async fn the_database_ends_a_statement_that_takes_too_long() {
+    let shop = brief_shop();
+    let (server, log) = server(&shop);
+    let agent = Agent::connect(&server).await;
+    let started = Instant::now();
+    let called = agent.call("run_query", long_query()).await;
+    assert_eq!(called.refused(), "timeout");
+    assert_eq!(
+        called.text,
+        "The query was stopped after 0.5 s, this connection's limit."
+    );
+    assert!(
+        started.elapsed() < Duration::from_secs(2),
+        "{:?}",
+        started.elapsed()
+    );
+    // The connection answers the next call.
+    let next = agent
+        .call(
+            "run_query",
+            json!({ "connection": BRIEF, "sql": "SELECT 1" }),
+        )
+        .await;
+    assert!(!next.is_error);
+    agent.close().await;
+    assert_eq!(entries(&log).await[0].result, "timeout");
+}
+
+#[tokio::test]
+async fn the_servers_own_timer_ends_what_the_database_did_not() {
+    let shop = brief_shop();
+    shop.without_the_databases_timeout();
+    let (server, _log) = server(&shop);
+    let agent = Agent::connect(&server).await;
+    let started = Instant::now();
+    let called = agent.call("run_query", long_query()).await;
+    assert_eq!(called.refused(), "timeout");
+    // The connection's half second, and the two the database is given to
+    // end the statement by itself.
+    let took = started.elapsed();
+    assert!(took >= Duration::from_millis(2_500), "{took:?}");
+    assert!(took < Duration::from_secs(5), "{took:?}");
+    assert_eq!(shop.reads_ended(), 1, "the read was awaited to its end");
+    agent.close().await;
+}
+
+#[tokio::test]
+async fn a_call_the_client_cancels_is_ended_at_the_database() {
+    let shop = Shop::new();
+    let (server, log) = server(&shop);
+    let agent = Agent::connect(&server).await;
+    // The SDK's client cancels a request it has waited this long for.
+    let arguments = json!({ "connection": "Bookshop · dev", "sql": slow(6) });
+    let sent = agent
+        .client
+        .send_cancellable_request(
+            ClientRequest::CallToolRequest(CallToolRequest::new(request("run_query", arguments))),
+            PeerRequestOptions::with_timeout(Duration::from_millis(300)),
+        )
+        .await
+        .unwrap();
+    let started = Instant::now();
+    let answer = sent.await_response().await;
+    assert!(
+        matches!(answer, Err(ServiceError::Timeout { .. })),
+        "{answer:?}"
+    );
+    let entries = logged(&log, 1).await;
+    assert_eq!(entries[0].result, "cancelled");
+    assert_eq!(entries[0].code.as_deref(), Some("cancelled"));
+    assert!(started.elapsed() < Duration::from_secs(5));
+    assert_eq!(shop.reads_ended(), 1);
+    // The session goes on.
+    assert!(!agent.call("list_connections", json!({})).await.is_error);
+    agent.close().await;
+}
+
+#[tokio::test]
+async fn a_client_that_is_gone_leaves_nothing_running() {
+    let shop = Shop::new();
+    let (server, log) = server(&shop);
+    let agent = Agent::connect(&server).await;
+    let arguments = json!({ "connection": "Bookshop · dev", "sql": slow(6) });
+    let sent = agent
+        .client
+        .send_cancellable_request(
+            ClientRequest::CallToolRequest(CallToolRequest::new(request("run_query", arguments))),
+            PeerRequestOptions::no_options(),
+        )
+        .await
+        .unwrap();
+    tokio::time::sleep(Duration::from_millis(300)).await;
+    let started = Instant::now();
+    drop(sent);
+    // Closing returns when the server has let the session go. The SDK
+    // goes on answering for five seconds after a client closed its end,
+    // for one that sent its calls and waits. Then the server ends what
+    // still runs: the statement would have taken minutes.
+    agent.close().await;
+    let took = started.elapsed();
+    assert!(took < Duration::from_secs(10), "{took:?}");
+    let entries = entries(&log).await;
+    assert_eq!(entries.len(), 1);
+    assert_eq!(entries[0].result, "cancelled");
+    assert_eq!(shop.reads_ended(), 1);
+    assert_eq!(server.open_sessions(), []);
+}
+
+#[tokio::test]
+async fn a_client_that_closed_its_writing_end_still_gets_its_answer() {
+    let shop = Shop::new();
+    let (server, _log) = server(&shop);
+    let mut raw = Raw::connect(&server, 64 * 1024).await;
+    raw.call(
+        1,
+        "run_query",
+        json!({ "connection": "Bookshop · dev", "sql": "SELECT count(*) FROM books" }),
+    )
+    .await;
+    raw.write.shutdown().await.unwrap();
+    let answer = raw.next().await;
+    assert_eq!(answer["id"], json!(1));
+    assert_eq!(answer["result"].get("isError"), Some(&json!(false)));
+    let text = answer["result"]["content"][0]["text"].as_str().unwrap();
+    assert!(text.contains("\"rows\":[[12]]"), "{text}");
+    raw.served.await.unwrap();
+}
+
+#[tokio::test]
+async fn a_stop_closes_a_session_whose_client_reads_nothing() {
+    let shop = Shop::new();
+    let (server, _log) = server(&shop);
+    // A transport that holds a kilobyte: the answers soon have nowhere to
+    // go, and the client never takes one.
+    let mut raw = Raw::connect(&server, 1024).await;
+    for id in 1..=40 {
+        raw.call(id, "list_connections", json!({})).await;
+    }
+    tokio::time::sleep(Duration::from_millis(300)).await;
+    let started = Instant::now();
+    let id = server.open_sessions()[0].id.clone();
+    assert!(server.stop_session(&id));
+    // The SDK would wait for that client for good.
+    tokio::time::timeout(Duration::from_secs(15), &mut raw.served)
+        .await
+        .expect("the session was let go")
+        .unwrap();
+    assert!(
+        started.elapsed() >= Duration::from_secs(2),
+        "{:?}",
+        started.elapsed()
+    );
+    assert_eq!(server.open_sessions(), []);
+}
+
+#[tokio::test]
+async fn a_message_longer_than_the_server_reads_ends_the_session() {
+    let shop = Shop::new();
+    let (server, _log) = server(&shop);
+    let mut raw = Raw::connect(&server, 64 * 1024).await;
+    // Two megabytes, and no end of a line.
+    let chunk = vec![b'x'; 64 * 1024];
+    let wrote = async {
+        for _ in 0..32 {
+            if raw.write.write_all(&chunk).await.is_err() {
+                break;
+            }
+        }
+    };
+    let (_, served) = tokio::join!(wrote, async {
+        tokio::time::timeout(Duration::from_secs(15), &mut raw.served).await
+    });
+    served.expect("the session ended").unwrap();
+    assert_eq!(server.open_sessions(), []);
+}
+
+#[tokio::test]
+async fn a_session_runs_so_many_calls_at_once_and_refuses_the_next() {
+    let shop = Shop::new();
+    let (server, _log) = server(&shop);
+    let mut raw = Raw::connect(&server, 1024 * 1024).await;
+    // The first runs for minutes if nothing ends it, and the others wait
+    // for the connection behind it.
+    for id in 1..=20 {
+        raw.call(
+            id,
+            "run_query",
+            json!({ "connection": "Bookshop · dev", "sql": slow(6) }),
+        )
+        .await;
+    }
+    // Sixteen are running or waiting. The four behind them are refused at
+    // once, the last four that were sent.
+    let mut refused = Vec::new();
+    for _ in 0..4 {
+        let answer = raw.next().await;
+        assert_eq!(
+            answer["result"]["structuredContent"]["refused"],
+            json!("busy")
+        );
+        assert_eq!(
+            answer["result"]["content"][0]["text"],
+            json!("Too many calls at once. Wait for an answer before the next call.")
+        );
+        refused.push(answer["id"].as_u64().unwrap());
+    }
+    refused.sort();
+    assert_eq!(refused, [17, 18, 19, 20]);
+    server.stop();
+    for _ in 0..16 {
+        let answer = raw.next().await;
+        assert_eq!(
+            answer["result"]["structuredContent"]["refused"],
+            json!("server_stopped")
+        );
+    }
+    raw.served.await.unwrap();
+}
+
+#[tokio::test]
+async fn stopping_a_session_answers_its_call_and_closes_it() {
+    let shop = Shop::new();
+    let (server, log) = server(&shop);
+    let (stopped, other) = (Agent::connect(&server).await, Agent::connect(&server).await);
+    let id = server.open_sessions()[0].id.clone();
+    let arguments = json!({ "connection": "Bookshop · dev", "sql": slow(6) });
+    let started = Instant::now();
+    let (called, ()) = tokio::join!(stopped.call("run_query", arguments), async {
+        tokio::time::sleep(Duration::from_millis(300)).await;
+        assert!(server.stop_session(&id));
+    });
+    assert_eq!(called.refused(), "server_stopped");
+    assert_eq!(called.text, "Tabletist's agent server was stopped.");
+    assert!(
+        started.elapsed() < Duration::from_secs(5),
+        "{:?}",
+        started.elapsed()
+    );
+    // The server closes that session, and no other.
+    stopped.ended().await;
+    assert_eq!(server.open_sessions().len(), 1);
+    assert!(!other.call("list_connections", json!({})).await.is_error);
+    assert!(!server.stop_session(&id), "it is gone");
+    other.close().await;
+    let entries = entries(&log).await;
+    assert_eq!(entries[0].result, "stopped");
+    assert_eq!(entries[0].code.as_deref(), Some("server_stopped"));
+}
+
+#[tokio::test]
+async fn stopping_the_server_answers_a_call_in_flight_that_it_was_stopped() {
+    let shop = Shop::new();
+    let (server, log) = server(&shop);
+    let agent = Agent::connect(&server).await;
+    let arguments = json!({ "connection": "Bookshop · dev", "sql": slow(6) });
+    let (called, ()) = tokio::join!(agent.call("run_query", arguments), async {
+        tokio::time::sleep(Duration::from_millis(300)).await;
+        server.stop();
+    });
+    // The person's stop, though the SDK also cancels the call when the
+    // session closes.
+    assert_eq!(called.refused(), "server_stopped");
+    agent.ended().await;
+    assert_eq!(entries(&log).await[0].result, "stopped");
+}
diff --git a/crates/tabletist-mcp/tests/tools.rs b/crates/tabletist-mcp/tests/tools.rs
index 1d908ef..7537762 100644
--- a/crates/tabletist-mcp/tests/tools.rs
+++ b/crates/tabletist-mcp/tests/tools.rs
@@ -6,6 +6,8 @@
 
 mod support;
 
+use std::time::Duration;
+
 use rmcp::model::ErrorCode;
 use rmcp::service::ServiceError;
 use serde_json::{Value as Json, json};
@@ -433,3 +435,35 @@ async fn run_query_takes_its_arguments_and_no_others() {
     }
     agent.close().await;
 }
+
+#[tokio::test]
+async fn a_second_call_on_a_connection_waits_its_turn_and_then_is_busy() {
+    let shop = Shop::new();
+    // Only the server's own timer ends the first call: a second and two
+    // more. The second call waits the connection's one second.
+    shop.without_the_databases_timeout();
+    let brief = Limits {
+        timeout: Duration::from_secs(1),
+        ..LIMITS
+    };
+    shop.list("Bookshop · brief", &shop.bookshop(), brief);
+    let (server, _log) = server(&shop);
+    let (first, second) = (Agent::connect(&server).await, Agent::connect(&server).await);
+    let on_brief = |sql: String| json!({ "connection": "Bookshop · brief", "sql": sql });
+    let (long, waiting) =
+        tokio::join!(first.call("run_query", on_brief(support::slow(6))), async {
+            tokio::time::sleep(Duration::from_millis(200)).await;
+            second.call("run_query", on_brief("SELECT 1".into())).await
+        });
+    assert_eq!(waiting.refused(), "busy");
+    assert_eq!(
+        waiting.text,
+        "Another agent's query is running on this connection. Try again."
+    );
+    assert_eq!(long.refused(), "timeout");
+    // Its turn comes once the first call has ended.
+    let again = second.call("run_query", on_brief("SELECT 1".into())).await;
+    assert!(!again.is_error);
+    first.close().await;
+    second.close().await;
+}
````

- [ ] **Step 2: Run them to see that they do not build**

Run: `cargo test --locked -p tabletist-mcp --lib`

Expected: it does not build. There is no timer, and no reason for it to give:

```text
error[E0425]: cannot find value `GRACE` in this scope
error[E0599]: no variant, associated function, or constant named `Timeout` found for enum `stop::Why` in the current scope
error[E0425]: cannot find function `within` in this scope
error: could not compile `tabletist-mcp` (lib test) due to 10 previous errors
```

`tests/stops.rs` is not run here: without the timer, the one test of it would wait for a statement that takes minutes.

- [ ] **Step 3: The timer**

````diff
diff --git a/crates/tabletist-mcp/src/refusal.rs b/crates/tabletist-mcp/src/refusal.rs
index 8119b9d..22dac06 100644
--- a/crates/tabletist-mcp/src/refusal.rs
+++ b/crates/tabletist-mcp/src/refusal.rs
@@ -145,6 +145,7 @@ impl Refused {
             Error::Agent(refusal) => refusal.into(),
             Error::Timeout => Self::timeout(limits.timeout),
             Error::Cancelled => match why {
+                Some(Why::Timeout) => Self::timeout(limits.timeout),
                 Some(Why::Stopped) => Self::stopped(),
                 // Or the database ended it, and nobody here asked.
                 Some(Why::Client) | None => Self::cancelled(),
diff --git a/crates/tabletist-mcp/src/stop.rs b/crates/tabletist-mcp/src/stop.rs
index f4b5dee..67d67f1 100644
--- a/crates/tabletist-mcp/src/stop.rs
+++ b/crates/tabletist-mcp/src/stop.rs
@@ -9,6 +9,9 @@ use tokio_util::sync::CancellationToken;
 /// Why a call was stopped.
 #[derive(Debug, Clone, Copy, PartialEq, Eq)]
 pub(crate) enum Why {
+    /// It ran past the connection's time, and the database had not ended
+    /// it by itself.
+    Timeout,
     /// The client cancelled the call, or is gone.
     Client,
     /// The person stopped the session or the server.
diff --git a/crates/tabletist-mcp/src/tools.rs b/crates/tabletist-mcp/src/tools.rs
index 7979bb3..3c3b1c7 100644
--- a/crates/tabletist-mcp/src/tools.rs
+++ b/crates/tabletist-mcp/src/tools.rs
@@ -5,9 +5,11 @@
 mod connections;
 mod query;
 
+use std::future::Future;
 use std::time::Duration;
 
 use serde_json::{Map, Value as Json, json};
+use tabletist_db::agent::Limits;
 
 use crate::gateway::{Failure, Gateway, Listed};
 use crate::refusal::Refused;
@@ -76,6 +78,10 @@ pub(crate) struct Done {
     pub note: Note,
 }
 
+/// How long past its connection's time a call is left before this crate
+/// ends it: the database's own timeout comes first, and says more.
+pub(crate) const GRACE: Duration = Duration::from_secs(2);
+
 /// Runs the tool called `name`.
 pub(crate) async fn call<G: Gateway>(
     gateway: &G,
@@ -142,6 +148,20 @@ fn find<G: Gateway>(gateway: &G, name: &str) -> Result<Listed, Refused> {
         .ok_or_else(|| Refused::unknown_connection(name))
 }
 
+/// Awaits `work` to its end, and ends the call when it runs past its
+/// connection's time by [`GRACE`]. The work is never dropped: once the call
+/// is ended it is the gateway that brings it to its end.
+async fn within<T>(stop: &Stop, limits: Limits, work: impl Future<Output = T>) -> T {
+    tokio::pin!(work);
+    tokio::select! {
+        done = &mut work => done,
+        () = tokio::time::sleep(limits.timeout.saturating_add(GRACE)) => {
+            stop.end(Why::Timeout);
+            work.await
+        }
+    }
+}
+
 /// What the work on a connection comes to. A call the person stopped
 /// answers that, whatever it had got to: an agent that was stopped is
 /// given nothing more.
diff --git a/crates/tabletist-mcp/src/tools/query.rs b/crates/tabletist-mcp/src/tools/query.rs
index 9595d50..d0b3f07 100644
--- a/crates/tabletist-mcp/src/tools/query.rs
+++ b/crates/tabletist-mcp/src/tools/query.rs
@@ -3,7 +3,7 @@
 use serde_json::{Map, Value as Json, json};
 use tabletist_db::agent::{self, Param};
 
-use super::{Arguments, Done, Invalid, Note, Tool, find, kept, schema, settle};
+use super::{Arguments, Done, Invalid, Note, Tool, find, kept, schema, settle, within};
 use crate::answer::{self, MAX_ANSWER_BYTES};
 use crate::gateway::{Gateway, Session};
 use crate::refusal::{Refused, code};
@@ -86,7 +86,7 @@ async fn run<G: Gateway>(
     let session = settle(gateway.open(&listed.id, stop.token()).await, stop, &listed)?;
     note.ran_as = Some(RAN_AS);
     let limits = listed.limits;
-    let answer = session.read(&read, limits).await;
+    let answer = within(stop, limits, session.read(&read, limits)).await;
     let answer = settle(answer, stop, &listed)?;
     let written = answer::write(&answer, MAX_ANSWER_BYTES).map_err(|_| {
         Refused::new(
````

- [ ] **Step 4: Run the tests**

Run: `cargo test --locked -p tabletist-mcp`

Expected: PASS. `tests/stops.rs` takes about five seconds and `tests/tools.rs` about three: they wait on the real clock.

```text
     Running unittests src/lib.rs
test result: ok. 42 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
     Running tests/sessions.rs
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
     Running tests/stops.rs
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
     Running tests/tools.rs
test result: ok. 16 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

- [ ] **Step 5: Commit**

```bash
cargo fmt --all
cargo clippy --locked --workspace --all-targets -- -D warnings
git add crates/tabletist-mcp
git commit -m "End an agent's call that the database did not end"
```

---

### Task 6: `describe_schema`

The tool has two answers. Without `tables` it lists the connection's own schema: names, kinds, and the catalog's rough count of rows. With `tables` it describes each one that is there, and lists the others as not found.

Both are held to the size an answer may have. A list that does not fit says `truncated`. A description that does not fit names the tables it left out, so that an agent asks for them again.

A name is looked for in the connection's own schema first, as it stands, and then, when it has a dot, in the schema before the dot. So `v1.events` is found as a table of that name before it is taken for `events` in a schema `v1`.

**Files:**
- Create: `crates/tabletist-mcp/src/tools/schema.rs`
- Modify: `crates/tabletist-mcp/src/tools.rs`, `crates/tabletist-mcp/tests/tools.rs`

- [ ] **Step 1: Write the failing tests**

`schema.rs` holds its own two tests and nothing else yet. Nothing reads the file until Step 3 names it in `tools.rs`.

````diff
diff --git a/crates/tabletist-mcp/src/tools.rs b/crates/tabletist-mcp/src/tools.rs
index 3c3b1c7..ad9374f 100644
--- a/crates/tabletist-mcp/src/tools.rs
+++ b/crates/tabletist-mcp/src/tools.rs
@@ -244,7 +244,7 @@ mod tests {
     #[test]
     fn the_tools_are_told_as_they_are() {
         let names: Vec<&str> = all().iter().map(|tool| tool.name).collect();
-        assert_eq!(names, ["list_connections", "run_query"]);
+        assert_eq!(names, ["list_connections", "describe_schema", "run_query"]);
         for tool in all() {
             assert_eq!(tool.input["type"], json!("object"), "{}", tool.name);
             assert!(!tool.description.is_empty() && !tool.title.is_empty());
@@ -255,7 +255,7 @@ mod tests {
     fn a_tool_that_is_not_there_is_answered_with_the_ones_that_are() {
         assert_eq!(
             Invalid::UnknownTool.message(),
-            "Unknown tool. Tabletist has list_connections, run_query."
+            "Unknown tool. Tabletist has list_connections, describe_schema, run_query."
         );
     }
 
diff --git a/crates/tabletist-mcp/src/tools/schema.rs b/crates/tabletist-mcp/src/tools/schema.rs
new file mode 100644
index 0000000..bcca36c
--- /dev/null
+++ b/crates/tabletist-mcp/src/tools/schema.rs
@@ -0,0 +1,38 @@
+//! `describe_schema`: the tables of a connection, and what a table is made
+//! of.
+
+#[cfg(test)]
+mod tests {
+    use super::*;
+
+    #[test]
+    fn a_name_is_looked_for_at_home_first_and_then_by_its_schema() {
+        let schemas = ["public".to_owned(), "sales".to_owned()];
+        assert_eq!(
+            places("book_covers", Some("public"), &schemas),
+            [("public", "book_covers")]
+        );
+        assert_eq!(
+            places("sales.customers", Some("public"), &schemas),
+            [("public", "sales.customers"), ("sales", "customers")]
+        );
+        // A dot in a name of the connection's own schema.
+        assert_eq!(
+            places("v1.events", Some("public"), &schemas),
+            [("public", "v1.events")]
+        );
+        assert_eq!(places("book_covers", None, &schemas), []);
+    }
+
+    #[test]
+    fn room_is_taken_until_there_is_no_more() {
+        let mut room = Room::new();
+        let entry = json!("x".repeat(MAX_ANSWER_BYTES / 2 - 200));
+        assert!(room.take(&entry));
+        assert!(room.take(&entry));
+        let left = room.left;
+        assert!(!room.take(&entry));
+        assert_eq!(room.left, left, "what did not fit took nothing");
+        assert!(room.take(&json!("small")));
+    }
+}
diff --git a/crates/tabletist-mcp/tests/tools.rs b/crates/tabletist-mcp/tests/tools.rs
index 7537762..7708d83 100644
--- a/crates/tabletist-mcp/tests/tools.rs
+++ b/crates/tabletist-mcp/tests/tools.rs
@@ -39,7 +39,7 @@ async fn an_agent_is_told_the_tools_and_that_each_only_reads() {
     let agent = Agent::connect(&server).await;
     let tools = agent.client.list_all_tools().await.unwrap();
     let names: Vec<&str> = tools.iter().map(|tool| tool.name.as_ref()).collect();
-    assert_eq!(names, ["list_connections", "run_query"]);
+    assert_eq!(names, ["list_connections", "describe_schema", "run_query"]);
     for tool in &tools {
         let annotations = tool.annotations.as_ref().unwrap();
         assert_eq!(annotations.read_only_hint, Some(true), "{}", tool.name);
@@ -85,7 +85,7 @@ async fn a_call_that_is_no_tools_is_the_protocols_error() {
     // The sentence never repeats what was sent, here a name.
     assert_eq!(
         invalid(&agent, "drop_everything", json!({})).await,
-        "Unknown tool. Tabletist has list_connections, run_query."
+        "Unknown tool. Tabletist has list_connections, describe_schema, run_query."
     );
     assert_eq!(
         invalid(&agent, "list_connections", json!({ "all": true })).await,
@@ -467,3 +467,154 @@ async fn a_second_call_on_a_connection_waits_its_turn_and_then_is_busy() {
     first.close().await;
     second.close().await;
 }
+
+#[tokio::test]
+async fn describe_schema_lists_the_tables_and_views() {
+    let shop = Shop::new();
+    let (server, _log) = server(&shop);
+    let agent = Agent::connect(&server).await;
+    let called = agent
+        .call("describe_schema", json!({ "connection": DEV }))
+        .await;
+    let answer = called.answer();
+    assert_eq!(answer["schema"], json!("main"));
+    let tables: Vec<(&str, &str)> = answer["tables"]
+        .as_array()
+        .unwrap()
+        .iter()
+        .map(|table| {
+            (
+                table["name"].as_str().unwrap(),
+                table["kind"].as_str().unwrap(),
+            )
+        })
+        .collect();
+    assert_eq!(
+        tables,
+        [
+            ("authors", "table"),
+            ("book_covers", "table"),
+            ("books", "table"),
+            ("customer_names", "view"),
+            ("customers", "table"),
+            ("order_items", "table"),
+            ("orders", "table"),
+            ("publishers", "table"),
+            ("recent_orders", "view"),
+        ]
+    );
+    assert!(answer.get("truncated").is_none());
+    assert!(answer.get("other_schemas").is_none(), "SQLite has the one");
+    agent.close().await;
+}
+
+#[tokio::test]
+async fn describe_schema_says_what_a_table_is_made_of() {
+    let shop = Shop::new();
+    let (server, _log) = server(&shop);
+    let agent = Agent::connect(&server).await;
+    let called = agent
+        .call(
+            "describe_schema",
+            json!({ "connection": DEV, "tables": ["order_items", "no_such_table", "orders"] }),
+        )
+        .await;
+    let answer = called.answer();
+    assert_eq!(answer["not_found"], json!(["no_such_table"]));
+    let tables = answer["tables"].as_array().unwrap();
+    assert_eq!(tables.len(), 2);
+    let items = &tables[0];
+    assert_eq!(items["name"], json!("order_items"));
+    assert_eq!(items["kind"], json!("table"));
+    assert!(items.get("schema").is_none(), "its own schema goes unsaid");
+    assert_eq!(items["primary_key"], json!(["id"]));
+    let columns = items["columns"].as_array().unwrap();
+    assert_eq!(columns.len(), 5);
+    assert_eq!(
+        columns[3],
+        json!({ "name": "quantity", "type": "INTEGER", "nullable": false })
+    );
+    let to_orders = items["foreign_keys"]
+        .as_array()
+        .unwrap()
+        .iter()
+        .find(|key| key["references"] == json!("orders"))
+        .unwrap();
+    assert_eq!(
+        to_orders,
+        &json!({
+            "columns": ["order_id"],
+            "references": "orders",
+            "referenced_columns": ["id"],
+            "on_delete": "CASCADE",
+        })
+    );
+    // And from the other side.
+    let orders = &tables[1];
+    assert_eq!(
+        orders["referenced_by"],
+        json!([{ "table": "order_items", "columns": ["order_id"], "on_delete": "CASCADE" }])
+    );
+    agent.close().await;
+}
+
+#[tokio::test]
+async fn describe_schema_takes_so_many_tables_at_once() {
+    let shop = Shop::new();
+    let (server, _log) = server(&shop);
+    let agent = Agent::connect(&server).await;
+    let many: Vec<String> = (0..21).map(|n| format!("t{n}")).collect();
+    let called = agent
+        .call(
+            "describe_schema",
+            json!({ "connection": DEV, "tables": many }),
+        )
+        .await;
+    assert_eq!(called.refused(), "too_large");
+    assert_eq!(
+        called.text,
+        "describe_schema describes at most 20 tables in one call."
+    );
+    // And a name is no longer than a table's can be: what was asked for
+    // comes back in the answer, and goes into the log.
+    let called = agent
+        .call(
+            "describe_schema",
+            json!({ "connection": DEV, "tables": ["books", "x".repeat(1_001)] }),
+        )
+        .await;
+    assert_eq!(called.refused(), "too_large");
+    assert_eq!(called.text, "A table's name is at most 1000 bytes.");
+    let called = agent
+        .call(
+            "describe_schema",
+            json!({ "connection": DEV, "tables": ["books", "x".repeat(1_000)] }),
+        )
+        .await;
+    assert_eq!(called.answer()["not_found"], json!(["x".repeat(1_000)]));
+    agent.close().await;
+}
+
+#[tokio::test]
+async fn describe_schema_takes_a_list_of_names_and_knows_no_connection_that_is_off() {
+    let shop = Shop::new();
+    let (server, _log) = server(&shop);
+    let agent = Agent::connect(&server).await;
+    assert_eq!(
+        invalid(
+            &agent,
+            "describe_schema",
+            json!({ "connection": DEV, "tables": [1] })
+        )
+        .await,
+        "describe_schema: `tables` is a list of strings."
+    );
+    let called = agent
+        .call(
+            "describe_schema",
+            json!({ "connection": "Bookshop · production" }),
+        )
+        .await;
+    assert_eq!(called.refused(), "unknown_connection");
+    agent.close().await;
+}
````

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test --locked -p tabletist-mcp --lib tools::tests::the_tools`, then `cargo test --locked -p tabletist-mcp --test tools describe_schema`

Expected: FAIL, both. The tool is not among those an agent is told, and a call of it is a call of no tool:

```text
test tools::tests::the_tools_are_told_as_they_are ... FAILED
  left: ["list_connections", "run_query"]
 right: ["list_connections", "describe_schema", "run_query"]
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 41 filtered out
```

```text
test describe_schema_says_what_a_table_is_made_of ... FAILED
test describe_schema_takes_a_list_of_names_and_knows_no_connection_that_is_off ... FAILED
test describe_schema_takes_so_many_tables_at_once ... FAILED
test describe_schema_lists_the_tables_and_views ... FAILED
  left: "Unknown tool. Tabletist has list_connections, run_query."
 right: "describe_schema: `tables` is a list of strings."
test result: FAILED. 0 passed; 4 failed; 0 ignored; 0 measured; 16 filtered out
```

- [ ] **Step 3: The tool**

````diff
diff --git a/crates/tabletist-mcp/src/tools.rs b/crates/tabletist-mcp/src/tools.rs
index ad9374f..01902a0 100644
--- a/crates/tabletist-mcp/src/tools.rs
+++ b/crates/tabletist-mcp/src/tools.rs
@@ -4,6 +4,7 @@
 
 mod connections;
 mod query;
+mod schema;
 
 use std::future::Future;
 use std::time::Duration;
@@ -27,7 +28,7 @@ pub struct Tool {
 
 /// Every tool. Each of them only reads.
 pub fn all() -> Vec<Tool> {
-    vec![connections::tool(), query::tool()]
+    vec![connections::tool(), schema::tool(), query::tool()]
 }
 
 /// A call that is not one of a tool's: the protocol's error, not a tool's
@@ -91,6 +92,7 @@ pub(crate) async fn call<G: Gateway>(
 ) -> Result<Done, Invalid> {
     match name {
         connections::NAME => connections::call(gateway, arguments),
+        schema::NAME => schema::call(gateway, stop, arguments).await,
         query::NAME => query::call(gateway, stop, arguments).await,
         _ => Err(Invalid::UnknownTool),
     }
diff --git a/crates/tabletist-mcp/src/tools/schema.rs b/crates/tabletist-mcp/src/tools/schema.rs
index bcca36c..b59407e 100644
--- a/crates/tabletist-mcp/src/tools/schema.rs
+++ b/crates/tabletist-mcp/src/tools/schema.rs
@@ -1,6 +1,396 @@
 //! `describe_schema`: the tables of a connection, and what a table is made
 //! of.
 
+use std::collections::HashMap;
+
+use serde_json::{Map, Value as Json, json};
+use tabletist_db::{Error, ObjectInfo, ObjectKind, ObjectRef, Structure};
+
+use super::{Arguments, Done, Invalid, Note, Tool, find, kept, schema, settle, within};
+use crate::answer::{self, MAX_ANSWER_BYTES};
+use crate::gateway::{Failure, Gateway, Schemas, Session};
+use crate::refusal::{Refused, code};
+use crate::stop::Stop;
+
+pub const NAME: &str = "describe_schema";
+
+/// The most tables one call describes.
+pub const MAX_TABLES: usize = 20;
+
+/// The most bytes of a name an agent asks for: more than any of the three
+/// databases lets a table be called by, bar SQLite, which has no limit.
+pub const MAX_NAME_BYTES: usize = 1_000;
+
+pub fn tool() -> Tool {
+    Tool {
+        name: NAME,
+        title: "Describe the schema",
+        description: "Without `tables`: the names of the tables and views of a connection, \
+                      with a rough count of each one's rows, and the names of its other \
+                      schemas. With `tables` (at most 20 names): each one's columns with their \
+                      types, its primary key, its foreign keys and the keys that point at it, \
+                      and its indexes. Write a table of another schema as schema.table.",
+        input: schema(
+            json!({
+                "connection": {
+                    "type": "string",
+                    "description": "The connection's name, from list_connections.",
+                },
+                "tables": {
+                    "type": "array",
+                    "description": "The tables to describe. Leave it out to list every table.",
+                    "items": { "type": "string" },
+                },
+            }),
+            &["connection"],
+        ),
+    }
+}
+
+pub async fn call<G: Gateway>(
+    gateway: &G,
+    stop: &Stop,
+    arguments: &Map<String, Json>,
+) -> Result<Done, Invalid> {
+    let arguments = Arguments::new(NAME, arguments, &["connection", "tables"])?;
+    let connection = arguments.text("connection")?;
+    let tables = match arguments.list("tables")? {
+        None => None,
+        Some(given) => Some(
+            given
+                .iter()
+                .map(|name| name.as_str().map(str::to_owned))
+                .collect::<Option<Vec<String>>>()
+                .ok_or_else(|| {
+                    Invalid::Arguments(format!("{NAME}: `tables` is a list of strings."))
+                })?,
+        ),
+    };
+    let mut note = Note {
+        request: tables.as_ref().map(|tables| kept(&tables.join(", "))),
+        ..Note::default()
+    };
+    let answer = run(gateway, stop, connection, tables.as_deref(), &mut note).await;
+    Ok(Done { answer, note })
+}
+
+async fn run<G: Gateway>(
+    gateway: &G,
+    stop: &Stop,
+    connection: &str,
+    tables: Option<&[String]>,
+    note: &mut Note,
+) -> Result<Map<String, Json>, Refused> {
+    let listed = find(gateway, connection)?;
+    note.connection = Some((listed.id.clone(), listed.name.clone()));
+    let asked = tables.unwrap_or_default();
+    if asked.len() > MAX_TABLES {
+        return Err(Refused::new(
+            code::TOO_LARGE,
+            format!("{NAME} describes at most {MAX_TABLES} tables in one call."),
+        ));
+    }
+    if asked.iter().any(|name| name.len() > MAX_NAME_BYTES) {
+        return Err(Refused::new(
+            code::TOO_LARGE,
+            format!("A table's name is at most {MAX_NAME_BYTES} bytes."),
+        ));
+    }
+    let session = settle(gateway.open(&listed.id, stop.token()).await, stop, &listed)?;
+    let work = async {
+        match tables {
+            None => list(&session).await,
+            Some(tables) => describe(&session, stop, tables).await,
+        }
+    };
+    let described = settle(within(stop, listed.limits, work).await, stop, &listed)?;
+    note.rows = Some(described.tables);
+    note.columns = described.columns;
+    Ok(described.json)
+}
+
+/// An answer, with what the log says of it.
+struct Described {
+    json: Map<String, Json>,
+    tables: u64,
+    columns: Option<u64>,
+}
+
+/// The tables and views of the connection's own schema, by name, and the
+/// names of its other schemas.
+async fn list<S: Session>(session: &S) -> Result<Described, Failure> {
+    let Schemas { names, home } = session.schemas().await?;
+    let objects = match &home {
+        Some(home) => session.objects(home).await?,
+        None => Vec::new(),
+    };
+    let mut room = Room::new();
+    let mut truncated = false;
+    let mut json = Map::new();
+    if let Some(home) = &home
+        && room.take(&json!(home))
+    {
+        json.insert("schema".into(), json!(home));
+    }
+    // The other schemas first, in a quarter of the room at most: a server
+    // can have thousands, and the tables are what was asked for.
+    let mut for_schemas = Room {
+        left: room.left / 4,
+    };
+    let mut others = Vec::new();
+    for name in names.iter().filter(|name| Some(*name) != home.as_ref()) {
+        let name = json!(name);
+        if !for_schemas.take(&name) {
+            truncated = true;
+            break;
+        }
+        others.push(name);
+    }
+    room.left -= room.left / 4 - for_schemas.left;
+    let mut tables = Vec::new();
+    for object in &objects {
+        let mut entry = Map::new();
+        entry.insert("name".into(), json!(object.name));
+        entry.insert("kind".into(), json!(kind(object.kind)));
+        if let Some(rows) = object.estimated_rows {
+            entry.insert("rows_estimate".into(), json!(rows));
+        }
+        let entry = Json::Object(entry);
+        if !room.take(&entry) {
+            truncated = true;
+            break;
+        }
+        tables.push(entry);
+    }
+    let listed = tables.len() as u64;
+    json.insert("tables".into(), Json::Array(tables));
+    if !others.is_empty() {
+        json.insert("other_schemas".into(), Json::Array(others));
+    }
+    if truncated {
+        json.insert("truncated".into(), json!(true));
+    }
+    Ok(Described {
+        json,
+        tables: listed,
+        columns: None,
+    })
+}
+
+/// What each of `asked` is made of. A name that is no table of the
+/// connection is listed as not found, and the others are described all the
+/// same. Tables that do not fit the answer are named as left out.
+async fn describe<S: Session>(
+    session: &S,
+    stop: &Stop,
+    asked: &[String],
+) -> Result<Described, Failure> {
+    let Schemas { names, home } = session.schemas().await?;
+    let mut room = Room::new();
+    // Each name comes back at most once more: as not found, or as left out.
+    for name in asked {
+        room.take(&json!(name));
+    }
+    // Each schema's tables, read once.
+    let mut listings: HashMap<String, Vec<ObjectInfo>> = HashMap::new();
+    let mut tables = Vec::new();
+    let (mut not_found, mut left_out) = (Vec::new(), Vec::new());
+    let (mut columns, mut full) = (0, false);
+    for name in asked {
+        if full {
+            left_out.push(name);
+            continue;
+        }
+        // Twenty tables are a hundred catalog queries. A call that was
+        // ended stops between two tables, whatever the gateway does.
+        if stop.token().is_cancelled() {
+            return Err(Error::Cancelled.into());
+        }
+        let mut found = None;
+        for (schema, table) in places(name, home.as_deref(), &names) {
+            if !listings.contains_key(schema) {
+                listings.insert(schema.to_owned(), session.objects(schema).await?);
+            }
+            if let Some(object) = listings[schema].iter().find(|object| object.name == table) {
+                found = Some((ObjectRef::new(schema, table), object.kind));
+                break;
+            }
+        }
+        let Some((object, object_kind)) = found else {
+            not_found.push(name);
+            continue;
+        };
+        let structure = session.describe(&object).await?;
+        let foreign = home.as_deref() != Some(object.schema.as_str());
+        let entry = table(&object, object_kind, foreign, &structure);
+        if room.take(&entry) {
+            tables.push(entry);
+            columns += structure.columns.len() as u64;
+        } else {
+            full = true;
+            left_out.push(name);
+        }
+    }
+    let mut json = Map::new();
+    let described = tables.len() as u64;
+    json.insert("tables".into(), Json::Array(tables));
+    if !not_found.is_empty() {
+        json.insert("not_found".into(), json!(not_found));
+    }
+    if !left_out.is_empty() {
+        json.insert("truncated".into(), json!(true));
+        json.insert("left_out".into(), json!(left_out));
+    }
+    Ok(Described {
+        json,
+        tables: described,
+        columns: Some(columns),
+    })
+}
+
+/// Where a name an agent wrote may be: in the connection's own schema as
+/// it stands, and, when it has a dot, in the schema before the dot.
+fn places<'a>(
+    name: &'a str,
+    home: Option<&'a str>,
+    schemas: &'a [String],
+) -> Vec<(&'a str, &'a str)> {
+    let mut places = Vec::new();
+    if let Some(home) = home {
+        places.push((home, name));
+    }
+    if let Some((schema, table)) = name.split_once('.')
+        && schemas.iter().any(|known| known == schema)
+    {
+        places.push((schema, table));
+    }
+    places
+}
+
+/// What is left of the size an answer may have.
+struct Room {
+    left: usize,
+}
+
+impl Room {
+    /// The size, less the answer's own words: its keys and its brackets.
+    fn new() -> Self {
+        Self {
+            left: MAX_ANSWER_BYTES - 200,
+        }
+    }
+
+    /// Takes the room `value` needs, with the comma after it, and says
+    /// whether there was that much.
+    fn take(&mut self, value: &Json) -> bool {
+        let needs = value.to_string().len() + 1;
+        if needs > self.left {
+            return false;
+        }
+        self.left -= needs;
+        true
+    }
+}
+
+fn kind(kind: ObjectKind) -> &'static str {
+    match kind {
+        ObjectKind::Table => "table",
+        ObjectKind::View => "view",
+        ObjectKind::MaterializedView => "materialized view",
+    }
+}
+
+fn table(
+    object: &ObjectRef,
+    object_kind: ObjectKind,
+    foreign: bool,
+    structure: &Structure,
+) -> Json {
+    let mut entry = Map::new();
+    entry.insert("name".into(), json!(object.name));
+    if foreign {
+        entry.insert("schema".into(), json!(object.schema));
+    }
+    entry.insert("kind".into(), json!(kind(object_kind)));
+    let columns: Vec<Json> = structure
+        .columns
+        .iter()
+        .map(|column| {
+            let mut entry = Map::new();
+            entry.insert("name".into(), json!(column.name));
+            entry.insert("type".into(), json!(column.type_name));
+            entry.insert("nullable".into(), json!(column.nullable));
+            // A default and a comment are as long as their author liked.
+            if let Some(default) = &column.default {
+                entry.insert("default".into(), answer::text(default));
+            }
+            if let Some(values) = &column.allowed_values {
+                entry.insert("values".into(), json!(values));
+            }
+            if column.generated {
+                entry.insert("generated".into(), json!(true));
+            }
+            if let Some(comment) = &column.comment {
+                entry.insert("comment".into(), answer::text(comment));
+            }
+            Json::Object(entry)
+        })
+        .collect();
+    entry.insert("columns".into(), Json::Array(columns));
+    if !structure.primary_key.is_empty() {
+        entry.insert("primary_key".into(), json!(structure.primary_key));
+    }
+    let named = |schema: &str, table: &str| match schema == object.schema {
+        true => table.to_owned(),
+        false => format!("{schema}.{table}"),
+    };
+    if !structure.foreign_keys.is_empty() {
+        let keys: Vec<Json> = structure
+            .foreign_keys
+            .iter()
+            .map(|key| {
+                json!({
+                    "columns": key.columns,
+                    "references": named(&key.ref_schema, &key.ref_table),
+                    "referenced_columns": key.ref_columns,
+                    "on_delete": key.on_delete,
+                })
+            })
+            .collect();
+        entry.insert("foreign_keys".into(), Json::Array(keys));
+    }
+    if !structure.referenced_by.is_empty() {
+        let keys: Vec<Json> = structure
+            .referenced_by
+            .iter()
+            .map(|key| {
+                json!({
+                    "table": named(&key.schema, &key.table),
+                    "columns": key.columns,
+                    "on_delete": key.on_delete,
+                })
+            })
+            .collect();
+        entry.insert("referenced_by".into(), Json::Array(keys));
+    }
+    let indexes: Vec<Json> = structure
+        .indexes
+        .iter()
+        .filter(|index| !index.primary)
+        .map(|index| {
+            json!({
+                "name": index.name,
+                "columns": index.columns,
+                "unique": index.unique,
+            })
+        })
+        .collect();
+    if !indexes.is_empty() {
+        entry.insert("indexes".into(), Json::Array(indexes));
+    }
+    Json::Object(entry)
+}
+
 #[cfg(test)]
 mod tests {
     use super::*;
````

- [ ] **Step 4: Run the tests**

Run: `cargo test --locked -p tabletist-mcp`

Expected: PASS.

```text
     Running unittests src/lib.rs
test result: ok. 44 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
     Running tests/sessions.rs
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
     Running tests/stops.rs
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
     Running tests/tools.rs
test result: ok. 20 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

- [ ] **Step 5: Commit**

```bash
cargo fmt --all
cargo clippy --locked --workspace --all-targets -- -D warnings
git add crates/tabletist-mcp
git commit -m "Describe a connection's tables to an agent"
```

---

### Task 7: What never leaves, and the rules that keep it so

Two test files that pin what Tasks 2 to 6 built, and the two places a later change is held to it.

- **`tests/log.rs`** (the master plan's F.4): each tool and each way a call ends is an entry with exactly these fields, and a refusal is an entry with its code, whatever refused the call. And no row's value is in the log: a value no statement spells out is put into the Bookshop, every tool is called on it, a database's error is made to quote it, and every byte of the log's files is searched for it.
- **`tests/secrets.rs`** (F.5): two connections that cannot be opened, whose driver errors name their files. Every tool is called on them, with good and bad arguments. Then every byte the server wrote to the agent, every entry of the log and every line any crate wrote to the app's log is searched for where the files are. And every line but the SDK's and the parser's is searched for what the agent sent. A second test reads the crate's own sources for the names of what opens a database.
- The test reads the app's log as the app would get it, so the tests build `tracing` with its `log` feature, as the app does ("What the SDK writes", above). `Cargo.lock` gains the one line.
- **`tests/engines.rs`** reads the new crate too. It finds a `Dialect` or a `Driver` that is compared (`==`, `!=`, `matches!`) where it should be matched. Nothing finds a `_` arm: the crate's one `match` on a `Dialect`, in `tools/connections.rs`, names each.
- **`AGENTS.md`** gets the crate's rules, where the review reads them.

These tests pass as they are written: the behaviour is there. Step 3 shows that each can fail.

The rule on the app's log is written into `AGENTS.md` now, one run before the code that keeps it: the second run cannot link the server without meeting it.

**Files:**
- Create: `crates/tabletist-mcp/tests/log.rs`, `crates/tabletist-mcp/tests/secrets.rs`
- Modify: `crates/tabletist-mcp/Cargo.toml`, `Cargo.lock`, `tests/engines.rs`, `AGENTS.md`

- [ ] **Step 1: The tests and the rules**

````diff
diff --git a/AGENTS.md b/AGENTS.md
index e0d057f..48ee86a 100644
--- a/AGENTS.md
+++ b/AGENTS.md
@@ -14,6 +14,18 @@ MySQL, SQLite) on egui/eframe and fastframe. The design lives in
 - Database, network, and disk work runs on the backend runtime
   (`src/backend.rs`), never on the UI thread.
 - `crates/tabletist-db` has no UI dependencies.
+- `crates/tabletist-mcp`, the MCP server agents use, has none either. It
+  reaches a database only through its `Gateway`, which lists the connections
+  an agent may name and opens one by its id, and never names `ConnectSpec`,
+  `Secrets` or `HostKeys`: its `tests/secrets.rs` reads its sources. Only
+  its `protocol` module names the SDK's types.
+- What an agent sent (a statement, a value, a name) goes into the log of
+  calls and nowhere else: not into the app's log, and not into a protocol
+  error. And nothing a database holds goes into either log: its error's
+  words can quote a row, so the log of calls keeps the error's code. The
+  SDK (`rmcp`, and `tracing` for its spans) and `sqlparser` write what a
+  client sent to the `log` crate themselves, so the app's log takes no line
+  of theirs.
 - Code that depends on the database engine matches on `Driver` or `Dialect`
   and names every variant: no `==`, `!=`, `matches!` or `_` arm. A new engine
   then fails to compile wherever nobody has decided for it.
diff --git a/Cargo.lock b/Cargo.lock
index ad28f87..f7573f9 100644
--- a/Cargo.lock
+++ b/Cargo.lock
@@ -5757,6 +5757,7 @@ dependencies = [
  "tempfile",
  "tokio",
  "tokio-util",
+ "tracing",
  "uuid",
 ]
 
diff --git a/crates/tabletist-mcp/Cargo.toml b/crates/tabletist-mcp/Cargo.toml
index 67e45b2..969c8f9 100644
--- a/crates/tabletist-mcp/Cargo.toml
+++ b/crates/tabletist-mcp/Cargo.toml
@@ -49,3 +49,8 @@ rmcp = { version = "3.5", default-features = false, features = ["client"] }
 tokio = { version = "1", features = ["rt-multi-thread", "macros", "time", "test-util"] }
 # A directory of its own for each test's files.
 tempfile = "3"
+# The app is built with this crate's `log` feature on (calloop turns it on,
+# on Linux), and with it what the SDK traces goes to the app's log. On here
+# too, so that the test which reads the app's log reads what the app would
+# write, on every platform.
+tracing = { version = "0.1", features = ["log"] }
diff --git a/crates/tabletist-mcp/tests/log.rs b/crates/tabletist-mcp/tests/log.rs
new file mode 100644
index 0000000..e54868c
--- /dev/null
+++ b/crates/tabletist-mcp/tests/log.rs
@@ -0,0 +1,309 @@
+//! The log: one entry for each call, with what was asked and what came of
+//! it, and nothing a database holds.
+
+// `allow-unwrap-in-tests` does not cover helpers in an integration-test crate.
+#![allow(clippy::unwrap_used)]
+
+mod support;
+
+use serde_json::json;
+use support::{Agent, Shop, entries, server};
+use tabletist_mcp::Entry;
+
+const DEV: &str = "Bookshop · dev";
+
+/// What every entry of these tests has.
+fn entry(tool: &str, result: &str) -> Entry {
+    Entry {
+        agent: "Claude Code".into(),
+        project: Some("tabletist-repo".into()),
+        transport: "stdio".into(),
+        tool: tool.into(),
+        result: result.into(),
+        ..Entry::default()
+    }
+}
+
+/// An entry without what differs from run to run, which is checked apart.
+fn steady(mut entry: Entry) -> Entry {
+    assert!(entry.at_ms > 1_700_000_000_000, "{}", entry.at_ms);
+    assert!(entry.took_ms.is_some_and(|took| took >= 0));
+    assert!(!entry.session.is_empty());
+    (entry.at_ms, entry.took_ms, entry.bytes) = (0, None, None);
+    entry.session.clear();
+    entry
+}
+
+#[tokio::test]
+async fn every_call_is_an_entry_with_what_was_asked_and_what_came_of_it() {
+    let shop = Shop::new();
+    let (server, log) = server(&shop);
+    let agent = Agent::connect(&server).await;
+    let on_dev = Some(format!("id:{DEV}"));
+    agent.call("list_connections", json!({})).await;
+    agent
+        .call("describe_schema", json!({ "connection": DEV }))
+        .await;
+    agent
+        .call(
+            "describe_schema",
+            json!({ "connection": DEV, "tables": ["book_covers", "publishers"] }),
+        )
+        .await;
+    let answered = agent
+        .call(
+            "run_query",
+            json!({
+                "connection": DEV,
+                "sql": "  SELECT id, kind FROM book_covers WHERE book_id = ? ORDER BY id;  ",
+                "params": [1],
+            }),
+        )
+        .await;
+    agent
+        .call(
+            "run_query",
+            json!({ "connection": DEV, "sql": "DELETE FROM book_covers WHERE id = 9" }),
+        )
+        .await;
+    agent
+        .call(
+            "run_query",
+            json!({ "connection": "Payroll", "sql": "SELECT * FROM salaries" }),
+        )
+        .await;
+    agent
+        .call(
+            "run_query",
+            json!({ "connection": DEV, "sql": "SELECT nmae FROM authors" }),
+        )
+        .await;
+    agent.close().await;
+
+    let read = entries(&log).await;
+    let sessions: Vec<&str> = read.iter().map(|entry| entry.session.as_str()).collect();
+    assert!(sessions.iter().all(|session| *session == sessions[0]));
+    let bytes = read[3].bytes;
+    assert_eq!(bytes, Some(answered.text.len() as i64));
+    let rows = answered.answer()["row_count"].as_i64();
+    assert!(rows.is_some_and(|rows| rows > 0));
+    let read: Vec<Entry> = read.into_iter().map(steady).collect();
+    let expected = [
+        Entry {
+            rows: Some(1),
+            ..entry("list_connections", "ok")
+        },
+        Entry {
+            connection_id: on_dev.clone(),
+            connection: Some(DEV.into()),
+            rows: Some(9),
+            ..entry("describe_schema", "ok")
+        },
+        Entry {
+            connection_id: on_dev.clone(),
+            connection: Some(DEV.into()),
+            request: Some("book_covers, publishers".into()),
+            rows: Some(2),
+            columns: Some(7),
+            ..entry("describe_schema", "ok")
+        },
+        Entry {
+            connection_id: on_dev.clone(),
+            connection: Some(DEV.into()),
+            // As the agent sent it.
+            sql: Some("  SELECT id, kind FROM book_covers WHERE book_id = ? ORDER BY id;  ".into()),
+            params: Some("[1]".into()),
+            ran_as: Some("read-only transaction".into()),
+            rows,
+            matched: rows,
+            columns: Some(2),
+            ..entry("run_query", "ok")
+        },
+        Entry {
+            connection_id: on_dev.clone(),
+            connection: Some(DEV.into()),
+            sql: Some("DELETE FROM book_covers WHERE id = 9".into()),
+            code: Some("not_read_only".into()),
+            message: Some("run_query only runs reads, and this is a DELETE.".into()),
+            ..entry("run_query", "refused")
+        },
+        Entry {
+            // A name that is no connection's is in the sentence alone.
+            sql: Some("SELECT * FROM salaries".into()),
+            code: Some("unknown_connection".into()),
+            message: Some("No connection named Payroll. list_connections has the names.".into()),
+            ..entry("run_query", "refused")
+        },
+        Entry {
+            connection_id: on_dev,
+            connection: Some(DEV.into()),
+            sql: Some("SELECT nmae FROM authors".into()),
+            code: Some("query_failed".into()),
+            // The database's words are the agent's, and can quote a row.
+            message: Some("The database refused the statement.".into()),
+            ran_as: Some("read-only transaction".into()),
+            ..entry("run_query", "error")
+        },
+    ];
+    assert_eq!(read.len(), expected.len());
+    for (read, expected) in read.iter().zip(&expected) {
+        assert_eq!(read, expected);
+    }
+}
+
+#[tokio::test]
+async fn a_refusal_is_an_entry_with_its_code_whatever_refused_the_call() {
+    let shop = Shop::new();
+    shop.list(
+        "Bookshop · gone",
+        &shop.path("no-such-file.db"),
+        support::LIMITS,
+    );
+    let (server, log) = server(&shop);
+    let agent = Agent::connect(&server).await;
+    let many: Vec<String> = (0..21).map(|n| format!("t{n}")).collect();
+    let calls = [
+        (
+            "run_query",
+            json!({ "connection": "Bookshop · gone", "sql": "SELECT 1" }),
+        ),
+        (
+            "describe_schema",
+            json!({ "connection": DEV, "tables": many }),
+        ),
+        (
+            "run_query",
+            json!({ "connection": DEV, "sql": "SELECT ?", "params": [{ "a": 1 }] }),
+        ),
+    ];
+    for (tool, arguments) in calls {
+        assert!(agent.call(tool, arguments).await.is_error, "{tool}");
+    }
+    agent.close().await;
+    let read: Vec<(String, Option<String>, Option<String>)> = entries(&log)
+        .await
+        .into_iter()
+        .map(|entry| (entry.result, entry.code, entry.message))
+        .collect();
+    let entry = |result: &str, code: &str, message: &str| {
+        (
+            result.to_owned(),
+            Some(code.to_owned()),
+            Some(message.to_owned()),
+        )
+    };
+    assert_eq!(
+        read,
+        [
+            entry(
+                "error",
+                "connection_unavailable",
+                "Tabletist could not open Bookshop · gone. Open it in Tabletist to see why.",
+            ),
+            entry(
+                "refused",
+                "too_large",
+                "describe_schema describes at most 20 tables in one call.",
+            ),
+            entry(
+                "refused",
+                "invalid_params",
+                "Parameter 1 is an object. A parameter is a string, a number, true, false or null.",
+            ),
+        ]
+    );
+}
+
+#[tokio::test]
+async fn an_answer_that_was_cut_does_not_say_how_many_rows_there_were() {
+    let shop = Shop::new();
+    let few = tabletist_db::agent::Limits {
+        rows: 3,
+        ..support::LIMITS
+    };
+    shop.list("Bookshop · few", &shop.bookshop(), few);
+    let (server, log) = server(&shop);
+    let agent = Agent::connect(&server).await;
+    agent
+        .call(
+            "run_query",
+            json!({ "connection": "Bookshop · few", "sql": "SELECT id FROM books" }),
+        )
+        .await;
+    agent.close().await;
+    let read = entries(&log).await;
+    assert_eq!((read[0].rows, read[0].matched), (Some(3), None));
+}
+
+#[tokio::test]
+async fn the_log_holds_nothing_a_database_holds() {
+    // A value no statement of the test spells out.
+    let marker = ["zq", "marker", "7f3a"].join("-");
+    let shop = Shop::new();
+    shop.change(&format!(
+        "INSERT INTO customers (id, name, email) VALUES (900, '{marker}', '{marker}@example.com');
+         ALTER TABLE customers ADD COLUMN tier TEXT DEFAULT '{marker}';"
+    ));
+    let (server, log) = server(&shop);
+    let agent = Agent::connect(&server).await;
+    let calls = [
+        ("list_connections", json!({})),
+        ("describe_schema", json!({ "connection": DEV })),
+        (
+            "describe_schema",
+            json!({ "connection": DEV, "tables": ["customers"] }),
+        ),
+        (
+            "run_query",
+            json!({ "connection": DEV, "sql": "SELECT * FROM customers" }),
+        ),
+        (
+            "run_query",
+            json!({ "connection": DEV, "sql": "SELECT name FROM customers WHERE id = ?", "params": [900] }),
+        ),
+        (
+            "run_query",
+            json!({ "connection": DEV, "sql": "SELECT * FROM customer_names" }),
+        ),
+        (
+            "run_query",
+            json!({ "connection": DEV, "sql": "EXPLAIN QUERY PLAN SELECT * FROM customers" }),
+        ),
+    ];
+    for (tool, arguments) in calls {
+        assert!(!agent.call(tool, arguments).await.is_error, "{tool}");
+    }
+    // A database's own error can quote a row: SQLite says which JSON path
+    // it could not read, and here the path is a customer's name. The agent
+    // may read that. The log keeps the error without its words.
+    let failed = agent
+        .call(
+            "run_query",
+            json!({ "connection": DEV, "sql": "SELECT json_extract('{}', name) FROM customers WHERE id = 900" }),
+        )
+        .await;
+    assert_eq!(failed.refused(), "query_failed");
+    assert!(failed.text.contains(marker.as_str()), "{}", failed.text);
+    // The agent was sent the value: it is the log that must not have it.
+    let sent = agent.received();
+    assert!(sent.matches(marker.as_str()).count() >= 5, "{sent}");
+    agent.close().await;
+    assert_eq!(entries(&log).await.len(), 8);
+    drop((server, log));
+
+    let mut files = 0;
+    for file in std::fs::read_dir(shop.path("")).unwrap() {
+        let path = file.unwrap().path();
+        let name = path.file_name().unwrap().to_string_lossy().into_owned();
+        if !name.starts_with("agents.sqlite3") {
+            continue;
+        }
+        files += 1;
+        let bytes = std::fs::read(&path).unwrap();
+        let found = bytes
+            .windows(marker.len())
+            .any(|window| window == marker.as_bytes());
+        assert!(!found, "{name} holds a row's value");
+    }
+    assert!(files >= 1);
+}
diff --git a/crates/tabletist-mcp/tests/secrets.rs b/crates/tabletist-mcp/tests/secrets.rs
new file mode 100644
index 0000000..bab93e9
--- /dev/null
+++ b/crates/tabletist-mcp/tests/secrets.rs
@@ -0,0 +1,249 @@
+//! Where a database is and what opens it never leaves the app: not to an
+//! agent, not into the log of calls, and not into the app's own log.
+
+// `allow-unwrap-in-tests` does not cover helpers in an integration-test crate.
+#![allow(clippy::unwrap_used)]
+
+mod support;
+
+use std::path::{Path, PathBuf};
+use std::sync::{Mutex, Once};
+
+use serde_json::json;
+use support::{Agent, LIMITS, Shop, entries, request, server};
+
+/// The app's log, caught: every record any crate writes while a test runs.
+static LINES: Mutex<Vec<Line>> = Mutex::new(Vec::new());
+
+#[derive(Debug, Clone)]
+struct Line {
+    level: log::Level,
+    target: String,
+    text: String,
+}
+
+struct Caught;
+
+impl log::Log for Caught {
+    fn enabled(&self, _metadata: &log::Metadata) -> bool {
+        true
+    }
+
+    fn log(&self, record: &log::Record) {
+        LINES.lock().unwrap().push(Line {
+            level: record.level(),
+            target: record.target().to_owned(),
+            text: record.args().to_string(),
+        });
+    }
+
+    fn flush(&self) {}
+}
+
+fn catch_the_log() {
+    static ONCE: Once = Once::new();
+    ONCE.call_once(|| {
+        log::set_logger(&Caught).unwrap();
+        log::set_max_level(log::LevelFilter::Trace);
+    });
+}
+
+/// Whether a line is the SDK's, or the parser's the check reads with.
+/// Both write what a client sent: the SDK a client's name and its
+/// notifications from `info` up and every request whole at `debug`, the
+/// parser each word of a statement. So the app's log takes no line of
+/// theirs. That is the app's filter to keep (`src/entrypoint.rs`), from the
+/// change that links this crate into the app.
+fn not_the_apps_to_write(line: &Line) -> bool {
+    ["rmcp", "sqlparser", "tracing"]
+        .iter()
+        .any(|theirs| line.target.starts_with(theirs))
+}
+
+#[tokio::test]
+async fn nothing_says_where_a_database_is() {
+    catch_the_log();
+    let shop = Shop::new();
+    // Two that cannot be opened: no such file, and a file that is no
+    // database. The driver's own error names the path of each.
+    let nowhere = shop.path("zq-nowhere-5e1c.db");
+    let garbage = shop.path("zq-garbage-5e1c.db");
+    std::fs::write(&garbage, "not a database, whatever its name says").unwrap();
+    shop.list("Bookshop · gone", &nowhere, LIMITS);
+    shop.list("Bookshop · garbage", &garbage, LIMITS);
+    let (server, log) = server(&shop);
+    // A client's name is its own to choose, and so is every word it sends.
+    let agent = Agent::connect_as(&server, support::client("zq-client-5e1c", None)).await;
+
+    let statement = "SELECT name FROM authors WHERE name <> 'zq-literal-5e1c'";
+    let mut refused = Vec::new();
+    for connection in [
+        "Bookshop · dev",
+        "Bookshop · gone",
+        "Bookshop · garbage",
+        "Payroll",
+    ] {
+        let calls = [
+            ("list_connections", json!({})),
+            ("describe_schema", json!({ "connection": connection })),
+            (
+                "describe_schema",
+                json!({ "connection": connection, "tables": ["authors", "nope"] }),
+            ),
+            (
+                "run_query",
+                json!({ "connection": connection, "sql": statement }),
+            ),
+            (
+                "run_query",
+                json!({ "connection": connection, "sql": "SELECT ?", "params": ["zq-value-5e1c"] }),
+            ),
+            (
+                "run_query",
+                json!({ "connection": connection, "sql": "SELECT nmae FROM authors" }),
+            ),
+            (
+                "run_query",
+                json!({ "connection": connection, "sql": "DROP TABLE authors" }),
+            ),
+        ];
+        for (tool, arguments) in calls {
+            let called = agent.call(tool, arguments).await;
+            if called.is_error {
+                refused.push((connection, called.refused().to_owned(), called.text.clone()));
+            }
+        }
+        // And calls that are no tool's.
+        for (tool, arguments) in [
+            (
+                "run_query",
+                json!({ "connection": connection, "sql": [statement] }),
+            ),
+            ("open_file", json!({ "path": "anywhere" })),
+        ] {
+            assert!(
+                agent
+                    .client
+                    .call_tool(request(tool, arguments))
+                    .await
+                    .is_err()
+            );
+        }
+    }
+    // The two that cannot be opened say one thing, and the same thing.
+    for name in ["Bookshop · gone", "Bookshop · garbage"] {
+        let said: Vec<&(&str, String, String)> = refused
+            .iter()
+            .filter(|(connection, code, _)| *connection == name && code == "connection_unavailable")
+            .collect();
+        assert_eq!(said.len(), 5, "{refused:?}");
+        for (_, _, text) in said {
+            assert_eq!(
+                text,
+                &format!("Tabletist could not open {name}. Open it in Tabletist to see why.")
+            );
+        }
+    }
+
+    let sent = agent.received();
+    agent.close().await;
+    let logged = format!("{:?}", entries(&log).await);
+    let lines = LINES.lock().unwrap().clone();
+    // The log was caught: this crate's line for a call is in it.
+    let ours: Vec<&Line> = lines
+        .iter()
+        .filter(|line| !not_the_apps_to_write(line))
+        .collect();
+    let a_call = "agent call: run_query refused (not_allowed) in ";
+    assert!(ours.iter().any(|line| line.text.starts_with(a_call)));
+
+    // Where the files are: the directory, and each file's own name.
+    let directory = shop
+        .path("")
+        .to_string_lossy()
+        .trim_end_matches('/')
+        .to_owned();
+    let private = [
+        directory.as_str(),
+        "zq-nowhere-5e1c",
+        "zq-garbage-5e1c",
+        "bookshop.db",
+    ];
+    for word in private {
+        assert!(!sent.contains(word), "an agent was sent {word}");
+        assert!(!logged.contains(word), "the log of calls holds {word}");
+        for line in &lines {
+            assert!(
+                !line.text.contains(word),
+                "the app's log holds {word}: {line:?}"
+            );
+        }
+    }
+    // What an agent sent is in the log of calls, on purpose, and in no
+    // line the app writes: not at any level, and from no crate.
+    assert!(logged.contains("zq-literal-5e1c") && logged.contains("zq-value-5e1c"));
+    assert!(logged.contains("zq-client-5e1c"));
+    let sent_words = [
+        "zq-literal-5e1c",
+        "zq-value-5e1c",
+        "zq-client-5e1c",
+        "SELECT",
+        "authors",
+        "Payroll",
+    ];
+    for line in &ours {
+        for word in sent_words {
+            assert!(
+                !line.text.contains(word),
+                "the app's log holds {word}: {line:?}"
+            );
+        }
+    }
+    // Which is why the SDK's lines are none of the app's: it writes who
+    // connected at `info`, and at `debug` the statement.
+    let theirs = |level: log::Level, word: &str| {
+        lines.iter().any(|line| {
+            line.target.starts_with("rmcp") && line.level == level && line.text.contains(word)
+        })
+    };
+    assert!(theirs(log::Level::Info, "zq-client-5e1c"));
+    assert!(theirs(log::Level::Debug, "zq-literal-5e1c"));
+}
+
+/// Every Rust file under `dir`.
+fn sources(dir: &Path, into: &mut Vec<PathBuf>) {
+    for entry in std::fs::read_dir(dir).unwrap() {
+        let path = entry.unwrap().path();
+        if path.is_dir() {
+            sources(&path, into);
+        } else if path.extension().is_some_and(|extension| extension == "rs") {
+            into.push(path);
+        }
+    }
+}
+
+#[test]
+fn the_crate_never_names_what_opens_a_database() {
+    let mut files = Vec::new();
+    sources(
+        &Path::new(env!("CARGO_MANIFEST_DIR")).join("src"),
+        &mut files,
+    );
+    assert!(files.len() >= 8, "{files:?}");
+    for file in files {
+        let text = std::fs::read_to_string(&file).unwrap();
+        for name in [
+            "ConnectSpec",
+            "Secrets",
+            "HostKeys",
+            "connect_with",
+            "Connection::connect",
+        ] {
+            assert!(
+                !text.contains(name),
+                "{} names {name}: the server reaches a database through its gateway alone",
+                file.display()
+            );
+        }
+    }
+}
diff --git a/tests/engines.rs b/tests/engines.rs
index fa13e5e..fdc9a83 100644
--- a/tests/engines.rs
+++ b/tests/engines.rs
@@ -2,8 +2,8 @@
 
 use std::path::{Path, PathBuf};
 
-/// Every Rust file of the app and of the database layer, by its path from
-/// the repository root, with its text.
+/// Every Rust file of the app, of the database layer and of the MCP
+/// server, by its path from the repository root, with its text.
 fn sources() -> Vec<(String, String)> {
     fn walk(dir: &Path, into: &mut Vec<PathBuf>) {
         for entry in std::fs::read_dir(dir).expect("read a source directory") {
@@ -17,7 +17,7 @@ fn sources() -> Vec<(String, String)> {
     }
     let root = Path::new(env!("CARGO_MANIFEST_DIR"));
     let mut paths = Vec::new();
-    for dir in ["src", "crates/tabletist-db/src"] {
+    for dir in ["src", "crates/tabletist-db/src", "crates/tabletist-mcp/src"] {
         let before = paths.len();
         walk(&root.join(dir), &mut paths);
         assert!(paths.len() > before, "no sources under {dir}");
````

- [ ] **Step 2: Run them**

Run: `cargo test --locked -p tabletist-mcp --test log --test secrets`, then `cargo test --locked --test engines`

Expected: PASS.

```text
     Running tests/log.rs
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
     Running tests/secrets.rs
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

```text
     Running tests/engines.rs
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

- [ ] **Step 3: See each of them fail**

Break one thing, run, and put it back. Three times.

Each time: change the one line, run `cargo test --locked -p tabletist-mcp --test log --test secrets --no-fail-fast`, and then `git checkout crates/tabletist-mcp/src`.

**A failure to open that says why.** In `src/tools.rs`, in `settle`:

```rust
// from
        (Err(failure), why) => Err(Refused::failed(failure, &listed.name, listed.limits, why)),
// to
        (Err(failure), _) => Err(Refused::new("query_failed", format!("{failure:?}"))),
```

Expected: `nothing_says_where_a_database_is` fails, since the two connections that cannot be opened no longer say the one sentence (`left: 0`, `right: 5`). Three tests of the log fail with it, all but the one of an answer that was cut: a database's words are in the log again, and a failure to open is no longer the entry it was.

**A database's words in the log.** In `src/refusal.rs`, in `failed`:

```rust
// from
                    logged: Some(format!("The database refused the statement{coded}.")),
// to
                    logged: None,
```

Expected: `the_log_holds_nothing_a_database_holds` fails with `agents.sqlite3 holds a row's value`, and `every_call_is_an_entry_with_what_was_asked_and_what_came_of_it` with it. `tests/secrets.rs` passes.

**What an agent sent in the app's log.** In `src/server.rs`, in `call`:

```rust
// from
            "agent call: {name} {result}{} in {} ms",
// to
            "agent call: {name} {result}{} in {} ms: {arguments:?}",
```

Expected: `nothing_says_where_a_database_is` fails with `the app's log holds authors`, and shows the line. `tests/log.rs` passes.

Then `git diff --stat crates/tabletist-mcp/src` shows nothing.

- [ ] **Step 4: Commit**

```bash
cargo fmt --all
cargo clippy --locked --workspace --all-targets -- -D warnings
git add Cargo.lock AGENTS.md tests/engines.rs crates/tabletist-mcp
git commit -m "Pin what an agent's call leaves behind, and what it never does"
```

---

### Task 8: The master plan says what was built

The master plan named the tools and left what they take and answer to this run. It also has a few things this run's draft showed otherwise: where the gateway's sessions are built, what `rmcp` brings, what the SDK does when a client closes its end, what of a database's error the log keeps, what else writes to the app's log. Each is corrected where it stands, with a note of its own.

**Files:**
- Modify: `docs/superpowers/plans/2026-10-13-mcp-server.md`

- [ ] **Step 1: Correct it**

````diff
diff --git a/docs/superpowers/plans/2026-10-13-mcp-server.md b/docs/superpowers/plans/2026-10-13-mcp-server.md
index f328d5f..c178056 100644
--- a/docs/superpowers/plans/2026-10-13-mcp-server.md
+++ b/docs/superpowers/plans/2026-10-13-mcp-server.md
@@ -24,6 +24,12 @@ list at the end, and the sections they change say so. With 5, 6, 15 and 16
 before them, no question marked as one Phase 1 waits for is open. Phase 1c
 has two of its own (8 and 13). G cuts 1b into five runs.
 
+**Built on 2026-10-13: Phase 1b's first run** (`2026-10-13-mcp-server-1b-1-server.md`), the
+server with nothing to start it. This plan named the tools and left open
+what each takes and answers: that run's plan says. A.1, A.4, A.5, B.5, B.8,
+C.1, F.1 and G are corrected where its draft showed otherwise, each where it
+changed.
+
 Each phase below gets its own plan under `docs/superpowers/plans/` before any
 code is written, as every batch of work in this repository does.
 
@@ -147,6 +153,16 @@ have no session at all. So the `Gateway`, implemented in the backend beside
 - An approved write gets a writable session of its own, opened when the
   person approves and closed when the write ends (B.7).
 
+**The trait, as built** (Phase 1b's first run). `Gateway::connections()`
+lists the connections agents may use. `Gateway::open(id, stop)` gives a
+`Session` for one call, with `schemas`, `objects`, `describe` and `read`,
+and cancelling `stop` ends whatever the session is doing. An agent names a
+connection, the tools find that name among `connections()`, and the session
+is opened by the connection's id, which is also what the log keeps: the
+name is the agent's word for it and can change. One call at a time, `busy`
+and the idle close are the implementation's to keep, behind `open`. The first run has the trait, and for its tests a gateway over SQLite
+files. The backend's own is the second run's.
+
 **Secrets.** The `Worker` owns the `Keyring` (`src/backend.rs:1567`). The
 gateway reads `connection/{id}/password` and `connection/{id}/ssh-secret`
 (`src/secrets.rs:18-25`) through it, inside the backend, when it opens a
@@ -310,7 +326,7 @@ HTTP agents with **Remove**, and says which is which (question 17).
 | Tool schemas | `Tool { name, title, description, input_schema, output_schema, annotations }`, each schema a JSON object. Written by hand with `serde_json::json!` for four tools. No macro (`macros` off) | The same JSON |
 | Cancellation | `RequestContext::ct`, a token cancelled when `notifications/cancelled` arrives | To write |
 | Progress | `peer.notify_progress(..)` when the request carried a progress token | To write |
-| Cost | New crates. Stdio: `rmcp`, `schemars` and `pastey` and their few dependencies; `tracing`, `chrono`, `tokio-util`, `futures`, `thiserror` and `uuid` are in `Cargo.lock` already. HTTP: `hyper`, `hyper-util`, `http`, `http-body`, `http-body-util`, `httparse`, `tower-service`, `sse-stream` and `tokio-stream`, none of which is in the tree today | No new crate for stdio. hyper and friends for HTTP all the same |
+| Cost | New crates. Stdio: `rmcp`, `schemars`, `schemars_derive`, `serde_derive_internals`, `pastey`, `dyn-clone`, `ref-cast` and `ref-cast-impl` (counted in Phase 1b's first run), and `tokio-stream` for the tests; `tracing`, `chrono`, `tokio-util`, `futures`, `thiserror` and `uuid` are in `Cargo.lock` already. HTTP: `hyper`, `hyper-util`, `http`, `http-body`, `http-body-util`, `httparse`, `tower-service`, `sse-stream` and `tokio-stream`, none of which is in the tree today | No new crate for stdio. hyper and friends for HTTP all the same |
 | Risk | A release every week or two (3.4.0 on 2026-09-15, 3.5.1 on 2026-10-05). Apache-2.0. Needs Rust 1.88; the workspace is on 1.98 | Ours to keep up with a protocol that changed three times in a year |
 
 Why rmcp: the protocol moves faster than this app should have to follow, and
@@ -323,11 +339,36 @@ and `transport-async-rw` in Phase 1b, `transport-streamable-http-server` in
 Phase 1d. The `auth` (OAuth), `client` and `macros` features stay off; the
 `client` feature is a dev-dependency for the test harness.
 
+In Phase 1b's first run the feature is `server` alone, which turns
+`transport-async-rw` on itself. The tests also build `tracing` with its
+`log` feature, as the app's build does, so that they see what the SDK
+writes to the app's log (C.1).
+
 **Tool annotations.** `readOnlyHint: true` on `list_connections`,
 `describe_schema` and `run_query`. `destructiveHint: true` on
 `propose_write`. They are hints for the client's own prompts. Nothing here
 relies on them.
 
+**What the tools take and answer** was left to Phase 1b's first run. Its
+plan has the table (`2026-10-13-mcp-server-1b-1-server.md`, "What this run settles"). In
+short:
+
+- `list_connections()`: for each connection its `name`, `engine`,
+  `parameters` (`$1, $2` or `?`), `access`, `rows` and `timeout_s`. The
+  engine and the placeholder are not on the board: an agent cannot write a
+  statement without them.
+- `describe_schema(connection, tables?)`: without `tables`, the names of the
+  tables and views of the connection's own schema. With them (at most 20),
+  what each is made of.
+- `run_query(connection, sql, params?)`: `columns`, `rows` as arrays,
+  `row_count`, `truncated`, `truncated_by`, `took_ms`.
+- An answer is its JSON as text, once, with no structured content beside
+  it: a client that hands a model both would hand it the rows twice, and
+  the transport would carry three times the size. A refusal has both, as
+  B.8 says: it is small.
+- No tool declares an output schema.
+- An argument a tool does not have is refused, not ignored.
+
 **The long wait in `propose_write`.** The call stays open until the person
 answers or five minutes pass:
 
@@ -368,6 +409,13 @@ struct AgentSession {
 - The agent's name is whatever the client says, in `initialize` or in a
   request's `_meta`. It is a label, not an identity. Only an HTTP token's
   name is something the person chose.
+- Built in Phase 1b's first run: the id (a UUID, since the log keeps it
+  from one start of the app to the next), the transport, the label (the
+  client's title, else its name, without control characters, at most 64
+  characters), the project, when it connected, its count of calls, and the
+  stop. The project is given by whoever hands the transport over. Asking the
+  client for its roots is left for later. `last_call_at`, `connections` and
+  `state` come with the tab that shows them, in the fourth run.
 
 **Where a session is** (added on 2026-10-13, for the Agents tab). The tab
 covers one connection, and a session belongs to none: the agent names a
@@ -409,6 +457,23 @@ connection in each call.
   or the stream ends (HTTP).
 - The agent's read session on the database is kept for the next agent,
   unless a cleanup failed, in which case it is closed, as the backend does.
+- A call the person stopped answers `server_stopped` whatever it had got
+  to, an answer too, and whatever else ended it besides: its timer, or the
+  SDK's own cancel when the session closes. The person's stop wins from
+  wherever it was given, the server's as a session's.
+- **A client that closes its end** (found in the first run's draft). The SDK
+  goes on answering for five seconds after a client closed its writing end,
+  which is right for one that sent its calls and waits, and it cancels
+  nothing. When it has let the transport go, the server ends what still
+  runs. A write that fails ends the session's calls at once: nobody will
+  read an answer.
+- **A client that reads nothing.** A session that was stopped is closed all
+  the same. Its calls are given five seconds to answer, and three seconds
+  after that the server takes the transport from the SDK, which would wait
+  on it for good.
+- **What a session may take.** A message of more than a megabyte ends the
+  session, since the SDK reads a line into memory however long it is. A
+  session runs sixteen calls at once, and the next is refused (`busy`).
 
 "Stop agents here" stops each session that is here, as "Stop session" does,
 and withdraws every write waiting on this connection. A session that also
@@ -813,6 +878,20 @@ written: Phase 1b, not 1a as this plan first had it (changed on 2026-10-13).
 - An answer is at most 256 kB as JSON. Past that, rows are dropped from the
   end and the answer says it was cut by size.
 
+**As built** (Phase 1b's first run). A text that is cut is
+`{"cut": <its first 2,000 characters>, "characters": <its length>}`, and a
+binary value is `{"binary_bytes": <its size>}`: a database's value is never
+an object, so neither can be taken for data. The size is 256,000 bytes, of
+an answer that is sent once (A.4). The columns count: an answer whose
+columns alone pass the size is refused (`too_large`), since two hundred
+columns with names of two thousand letters are 400 kB without a row, from a
+statement of under three kilobytes.
+`describe_schema` is held to the size as well, the names it was asked for
+among it. **"N of M" is not built yet**: the
+count is a change to the three drivers, a run of its own in `tabletist-db`
+before the fourth run of 1b shows the number. Until then an answer that was
+cut says so, and not of how many.
+
 **Settings.** Rows per answer: 100, 500 (default), 1,000 or 5,000. Statement
 timeout: 5, 10 (default), 30 or 60 seconds. `MAX_SQL_LIMIT` is 10,000 for
 the editor (`src/settings.rs:399-401`); agents get half of that at most.
@@ -949,7 +1028,7 @@ The codes are stable. The sentences may be reworded.
 | `connection_unavailable` | Tabletist could not open Bookshop · staging. Open it in Tabletist to see why. | The password is asked for each time, the SSH host is not trusted, the keyring is locked, the connect failed. The reason is never sent: it can name a host or a user |
 | `too_many_statements` | Send one statement per call. | B.2 |
 | `not_parsed` | Tabletist could not read this statement, so it was not run. | B.2 |
-| `not_read_only` | run_query only runs reads. Use propose_write for DELETE FROM book_covers WHERE id = 9. | A write, `INTO`, `FOR UPDATE` |
+| `not_read_only` | run_query only runs reads. Use propose_write for DELETE FROM book_covers WHERE id = 9. | A write, `INTO`, `FOR UPDATE`. Until Phase 2 has `propose_write` the sentence ends at what the statement is ("run_query only runs reads, and this is a DELETE."): the pointer is the server's to add, not the check's |
 | `not_allowed` | CREATE TABLE is not available to agents. | DDL, `COPY`, `SET`, `DO`, transaction control, upserts, `RETURNING` and the rest |
 | `not_a_write` | propose_write takes one INSERT, UPDATE or DELETE. Use run_query for a SELECT. | |
 | `write_not_allowed` | Bookshop · staging is read-only for agents. | `propose_write` on a Read connection |
@@ -957,12 +1036,12 @@ The codes are stable. The sentences may be reworded.
 | `system_catalog` | The system catalogs are not available on this connection. Use describe_schema. | B.3 |
 | `masked_column_used` | orders.email is masked. It can be selected, not filtered, joined, sorted or computed on. | B.4 |
 | `function_not_allowed` | pg_sleep is not available to agents. | B.2 |
-| `invalid_params` | The statement has 2 parameters and 1 was sent. | Count, or a value that is not a string, number, boolean or null |
-| `too_large` | The statement is larger than 100 kB. | |
+| `invalid_params` | The statement has 2 parameters and 1 was sent. | Count, or a value that is not a string, number, boolean or null, or a whole number past 64 bits signed |
+| `too_large` | The statement is larger than 100 kB. | Also more than 20 tables in one `describe_schema`, a table's name of more than 1,000 bytes, and an answer whose columns alone pass its size |
 | `timeout` | The query was stopped after 10 s, this connection's limit. | The statement timeout |
-| `busy` | Another agent's query is running on this connection. Try again. | A.1 |
+| `busy` | Another agent's query is running on this connection. Try again. | A.1. Also a session's seventeenth call at once: "Too many calls at once. Wait for an answer before the next call." |
 | `rate_limited` | Too many calls. Try again in 20 s. | E.5 |
-| `query_failed` | The database's own error, with its code. | A syntax error, a missing column. Without the detail and hint lines on a connection that hides or masks something |
+| `query_failed` | The database's own error, with its code. | A syntax error, a missing column. Without the detail and hint lines on a connection that hides or masks something. At most 2,000 characters, as every sentence is. The log keeps the code and not the words (C.1) |
 | `denied` | The person's note, or "The request was denied." | B.7 |
 | `timed_out` | No answer in 5 minutes. Nothing ran. | B.7 |
 | `count_changed` | B.7 | |
@@ -978,7 +1057,7 @@ Protocol and HTTP answers, for calls that never reach a tool:
 
 | Case | JSON-RPC | HTTP |
 |---|---|---|
-| An unknown tool, a missing argument, an argument of the wrong type | -32602 | 200 |
+| An unknown tool, a missing argument, an argument of the wrong type, an argument the tool does not have | -32602 | 200 |
 | An unknown method | -32601 | 200 |
 | Not JSON | -32700 | 400 |
 | No token, or an unknown one | | 401 |
@@ -986,6 +1065,9 @@ Protocol and HTTP answers, for calls that never reach a tool:
 | A wrong path, method, content type or size | | 404, 405, 415, 413 |
 | The server is stopping | | 503 |
 
+A protocol error's sentence never repeats what the agent sent: the SDK
+writes each one to the app's log (C.1).
+
 ## C. Data and settings
 
 ### C.1 Storage
@@ -1096,7 +1178,7 @@ CREATE TABLE calls (
   result        TEXT    NOT NULL,   -- ok | refused | error | timeout | waiting |
                                     -- approved | denied | timed_out | cancelled | stopped
   code          TEXT,
-  message       TEXT,               -- the refusal's sentence, or the database's message
+  message       TEXT,               -- the refusal's sentence; a database's error by its code
   note          TEXT,               -- the person's, on a denial
   ran_as        TEXT,               -- "read-only transaction" | "one transaction, committed" | …
   rows          INTEGER,            -- returned to the agent, or affected
@@ -1132,12 +1214,51 @@ CREATE INDEX calls_result     ON calls (result, at_ms);
 - **Export.** The rows that pass the current filters, as CSV or as JSON
   Lines, through the save dialog the app already has (`rfd`), written on the
   backend runtime.
+- **As built** (Phase 1b's first run): the file with the whole table, an
+  entry for each call a tool answered or refused, and `forget_before`. A
+  protocol error is no entry. A call whose entry cannot be written is
+  answered all the same, with an error in the app's log: Phase 2 decides
+  that again for a write. The schedule of retention comes with what starts
+  the server (the second run), the filters, the search and the export with
+  the Log view (the fourth).
+- **`message` is not the database's message** (corrected in the first run).
+  A database quotes what it could not read, and that can be a row's value:
+  SQLite's "bad JSON path" names the path, which a statement can take from
+  a column. The agent may read the row, so its sentence has the words. The
+  log holds no row, so for a database's error it keeps "The database
+  refused the statement (code 42703)." and nothing else. F.4's search of
+  the file found this.
+- **A log that is only locked is not a broken one.** A file is kept aside
+  when it is no database, a damaged one, or of another schema. One that
+  could not be looked into is an error, and stays where it is. What the
+  server does without its log is the second run's to say.
+- The file's mode is set whether the file was there or not.
 
 **The app's own log is another thing.** `tabletist.log` never holds SQL
 (`AGENTS.md`, and the editor spec's "The SQL text is never logged"). That
 stays true. The MCP code logs a tool's name, a code and a duration there,
 nothing more.
 
+**The SDK writes to the app's log itself** (found in Phase 1b's first
+run). `rmcp` traces through the `tracing` crate, and the app is built with
+that crate's `log` feature on (through `calloop`, on Linux), which sends
+what is traced to the app's log. What it writes is what a client sent:
+
+- at `info`, what a client calls itself, every notification whole (a
+  cancel's reason, a progress message), and each cancel;
+- at `warn`, every protocol error with the request's id, which is the
+  client's to choose, and for an unknown method its name;
+- at `debug`, every request whole: the statement and its values.
+
+`sqlparser`, which the check reads a statement with, writes each word at
+`debug`, and `tracing` writes lines of its own for the SDK's spans. So the
+app's log takes no line of the three. The second run, which links the
+server into the app, turns them off in the app's filter
+(`src/entrypoint.rs`), with a test that a session leaves nothing an agent
+sent in `tabletist.log`. The first run's `tests/secrets.rs` holds every
+other crate's lines to that, and shows what the SDK's hold. `AGENTS.md` has
+the rule.
+
 ### C.2 Migration, and what agents cannot change
 
 - A connection without an `agents` object reads as Off (`#[serde(default)]`,
@@ -1587,6 +1708,9 @@ both.
 
 - The harness starts the server with a `Gateway` over the SQLite fixture, a
   scripted approver and a clock it controls, and gives the test a client.
+  The first run of 1b has the gateway and the client
+  (`crates/tabletist-mcp/tests/support/`). The approver and the clock come
+  with Phase 2, which is the first to wait for a person.
 - **In process:** rmcp's client (a dev-dependency) over `tokio::io::duplex`.
   Most tests use this.
 - **Stdio, end to end:** the built binary, `tabletist mcp --endpoint
@@ -1751,7 +1875,7 @@ is off until the person turns it on.
 | Part | Runs | State on 2026-10-13 |
 |---|---|---|
 | 1a. The enforcement core | 3 | Done: pull requests 122, 123 and 124 |
-| 1b. The server over stdio, with its interface | 5, below | Next. Nothing it waits for is open |
+| 1b. The server over stdio, with its interface | 5, below | The first is built (`2026-10-13-mcp-server-1b-1-server.md`). Nothing the others wait for is open |
 | 1b on Windows: the way in | 1 | After 1b (question 14) |
 | 1c. Hidden tables | to be cut in its plan | Waits for questions 8 and 13 |
 | 1d. HTTP with tokens | 1 | After stdio (question 9) |
@@ -1817,8 +1941,8 @@ can see what agents do and stop them.
 
 | Run | What it builds | What a user can do after it | Tests |
 |---|---|---|---|
-| **1b-1. The server, with nothing to start it** | The `tabletist-mcp` crate: the server on `rmcp`, the gateway to the connections, `list_connections`, `describe_schema` and `run_query` over `agent::check_read` and `Connection::run_agent_read`, the size cap on an answer (B.5), the refusal codes (B.8), the log's store (C.1) | Nothing. No setting, no transport and no command reaches it | F.1 in process, F.4 but its interface, F.5 in process |
-| **1b-2. Turning it on from the files, and stdio** | The `[mcp]` keys in `settings.toml`. `[connections.agents]` with access, rows and timeout, which makes the connections file version 4. `Command::Mcp` and `Event::Mcp`, and the server started and stopped with the setting. The endpoint, the key and the relay, `tabletist mcp`, on Linux and macOS | With `enabled = true` and an access written by hand, an agent connects over stdio and reads. Nothing in the app shows it yet, and nothing in the app turns it on | F.1 over stdio, F.5 over stdio, the settings and connections files, `tests/cli.rs` |
+| **1b-1. The server, with nothing to start it** | The `tabletist-mcp` crate: the server on `rmcp`, the gateway's trait, `list_connections`, `describe_schema` and `run_query` over `agent::check_read` and `Connection::run_agent_read`, the size cap on an answer (B.5), the refusal codes (B.8), the log's store (C.1) | Nothing. No setting, no transport and no command reaches it | F.1 in process, F.4 but its interface, F.5 in process |
+| **1b-2. Turning it on from the files, and stdio** | The `[mcp]` keys in `settings.toml`. `[connections.agents]` with access, rows and timeout, which makes the connections file version 4. `Command::Mcp` and `Event::Mcp`, the backend's gateway with its own sessions on each database, and the server started and stopped with the setting. The schedule of the log's retention, and the SDK held out of the app's log. The endpoint, the key and the relay, `tabletist mcp`, on Linux and macOS | With `enabled = true` and an access written by hand, an agent connects over stdio and reads. Nothing in the app shows it yet, and nothing in the app turns it on | F.1 over stdio, F.5 over stdio, the settings and connections files, `tests/cli.rs` |
 | **1b-3. The Agents tab's frame** | The third kind of tab. The sidebar entry with its five states. The ways in: the key, `ga`, `:agents`, quick open. The three empty states, with **Turn on** and **Copy**. The keymap and the help | Open the tab in any state, see which state the connection is in, turn the server on from the tab | F.7: the entry's states, the ways in, one tab for a window, the empty states, nothing opening or focusing by itself |
 | **1b-4. What agents are doing** | `App::agents`. The sessions that are here and "Also:". The feed. **Stop session** and **Stop agents here**. The Log view as a list for this connection, with All connections. The notification for an agent that connects | See each agent and each call as it happens, stop one agent or all on a connection, read the log | F.7: one connection, the stops, closing the tab, the log's default. F.4's interface |
 | **1b-5. Letting agents in** | Settings › Agents, with the switch between panes. The Connections list's Agents column, its menu and Omarchy's box. The connection editor's Agents section, with **Allow agents…** and **Show Agents tab** pointing at them. The website: a new guide, and the reference pages | Everything Phase 1 draws, without a file edited by hand | The interface tests of Settings, Connections and the editor. The keymap. `docs/bin/check` |
@@ -1835,12 +1959,18 @@ can see what agents do and stop them.
   file. The website says nothing of Agents until the fifth.
 - **Questions the runs meet.** Four are open and not marked as ones to
   wait for. Each is built as this plan has it unless it is answered before
-  its run: what the log keeps of a call's values (18) and the name an agent
-  sees for a connection (19) in the first, the notification for a new agent
-  (17) in the fourth, "Stop all agents" (11) in the fifth.
-- **Sizes.** The first and the fifth are the largest. The first is the one
-  with the risk: `rmcp`, the gateway's own sessions on each database, and
-  the first log.
+  its run: what the log keeps of a call's values (18) in the first, which
+  built it as planned, in full: up to 100 kB of a call's statement and as
+  much of its values, cut there. The name an agent sees for a connection (19)
+  in the second, with the gateway that names the connections. The
+  notification for a new agent (17) in the fourth, "Stop all agents" (11) in
+  the fifth.
+- **Sizes.** The first had the risk of `rmcp` and of the first log. The
+  gateway's own sessions on each database went to the second run with the
+  backend's gateway, which makes the second the largest: its plan may cut it
+  in two, the gateway and the files, then the endpoint and the relay.
+- **"N of M"** (B.5) is a run of its own in `tabletist-db`, at any time
+  before the fourth.
 
 **1b on Windows: the way in.** One run, after 1b (question 14): the named
 pipe, the relay from the GUI binary or a console target if that fails
````

- [ ] **Step 2: Check it**

Run: `grep -c $'\u2014' docs/superpowers/plans/2026-10-13-mcp-server.md` (an em dash, written so that this plan holds none)

Expected: `0`.

- [ ] **Step 3: Commit**

```bash
git add docs/superpowers/plans/2026-10-13-mcp-server.md
git commit -m "Say in the MCP plan what its server's first run built"
```

---

### After the last task

- [ ] **The whole of it, once more**

```bash
cargo fmt --all --check
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo test --locked --workspace --all-targets
RUSTDOCFLAGS='-D warnings' cargo doc --locked --workspace --no-deps
git diff --stat origin/main -- src
```

Expected: all clean, and every test passes. On 2026-10-13, with both servers up: 23 test binaries, 3,342 passed, 0 failed, 22 ignored.

The last command shows nothing: the app is as it was.

Then run the workspace's tests once with the servers, as `AGENTS.md` has it.

- [ ] **Open the pull request.** `packaging.yml` starts with it, since `Cargo.toml` and `Cargo.lock` changed. The description says what was only compiled here (macOS, Windows), and that nothing starts the server.

## Self-review

- **Against the master plan's table for 1b-1** (G): the crate on `rmcp` (Task 3), the gateway's trait (Task 3), `list_connections` (3), `run_query` (4) and `describe_schema` (6) over `agent::check_read` and the gateway's `read`, the size cap (4, and 6 for the schema), the refusal codes (3 and 4), the log's store (2). Not there, and said above: the backend's gateway, which the table's words "the gateway to the connections" could be read to mean. Task 8 corrects the table.
- **Against F.1, in process.** The SDK's client over `tokio::io::duplex` (`tests/support`), and both lifecycles (`tests/sessions.rs`). The stdio and HTTP parts of F.1 are the second run's and Phase 1d's, and so is F.1's "same tool tests over all three, from one list": the tests here are bound to the one transport there is.
- **Against F.4.** An entry is asserted for each tool and for these outcomes: answered, refused by the check, unknown connection, the database's error, a connection that cannot be opened, `too_large`, `invalid_params` (`tests/log.rs`), timeout, cancelled, stopped (`tests/stops.rs`). The two kinds of `busy` are asserted as answers and not as entries: they pass through the same code as every refusal. No row data, by a search of the file's bytes, a quoted row in an error among it. A file that cannot be read is set aside (`src/log.rs`). Not here: retention on a paused clock (the schedule is the second run's; `forget_before` is tested), export (the fourth run), `propose_write`'s entry (Phase 2), and the connections file with `agents` (the second run).
- **Against F.5.** Every tool, good and bad arguments, connections that fail to open. The transport's bytes, the log's entries and every line written to the `log` crate are searched. A wrong password and an unreachable host need a server and the backend's gateway: the second run. Here the two failures are SQLite's, whose errors name a path. The static check reads the crate's sources.
- **Reviewed before it was written.** A reviewer was given the draft and asked to break it, with tests of its own. It found twelve defects, each with a test that showed it, and all twelve are fixed in what is above, each with a test: a stop of the whole server answered `cancelled`; a timer or the SDK's cancel could outrank the person's stop; a database's error put a row's value into the log, and was sent at any length; the SDK's own lines put a client's words into the app's log; a stop never closed a session whose client read nothing; a line without an end took 200 MB, and calls ran without bound; columns, the names asked of `describe_schema` and the log's `request` were outside the size cap, and an answer went over the transport three times; a log that was only locked was set aside; an existing log kept its permissions; a serving that was dropped left its session open; a blank title hid a client's name; and a client that closed its writing end was answered `cancelled`. What it tried that held: `unknown_connection` for a connection that is Off, the one sentence for a failure to open, the cap on rows, and arguments 120 levels deep.
- **Left open, and said above.** A client that floods and reads nothing costs a few hundred bytes a call. `Gateway::open` is the implementation's to bound: the server's timer starts when it has the session.
- **Names.** `Gateway`, `Session`, `Listed`, `Schemas`, `Failure`, `Refused`, `code`, `Stop`, `Why`, `Tool`, `Invalid`, `Note`, `Done`, `Arguments`, `Reply`, `Live`, `Server`, `Origin`, `Transport`, `SessionInfo`, `Log`, `Entry`, `Opened`, `LogError`, `Watched`, `Handler` are spelled the same in every task. `Why::Timeout`, `within` and `GRACE` arrive in Task 5, and `Stop::token` in Task 4, each with its first caller: before that the compiler would call it dead.
- **Every commit stands.** Each task's state was built and tested as a whole. After Task 3 an agent can list connections and nothing else. After Task 4 it can read. `describe_schema` comes after the stops, in Task 6: it is the simpler tool, and nothing waits for it.

## After the run

Run on 2026-10-13, inline, one commit for each task. Every step showed what it says above, the three breakages of Task 7 among them, and the files the tasks changed are byte for byte the draft's. The last state passed the workspace's tests with PostgreSQL and MySQL up: 23 test binaries, 3,342 passed, none failed.

One step was wrong as written, and is corrected above: the last check compared `src` with `main`, and a worktree's own `main` can be months behind. It is `origin/main` that the app's sources are the same as.

## After the pull request's review

The review found nothing to stop the merge, and one sentence that said too much: the crate's own description had "Every call is written to the Log", where a call that is no tool's is the protocol's error and no entry, as "What this run settles" has it and `tests/tools.rs` asserts. The sentence in `src/lib.rs` now says which calls. The diffs above have it as it was.
