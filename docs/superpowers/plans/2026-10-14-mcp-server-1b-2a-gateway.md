# MCP server, Phase 1b, run 2a: the backend's gateway. Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** The app's backend can run the MCP server of `tabletist-mcp` over the person's saved connections: what agents may do on a connection is kept in `connections.toml`, the backend's own gateway opens a read-only session for each connection an agent uses, and the backend starts and stops the server with its log. Nothing in the app starts it yet, and nothing outside reaches it.

**Architecture:** `SavedConnection` gains `agents` (access Off or Read, rows, timeout), written as a `[connections.agents]` table. A new module of the backend, `backend/agents.rs`, implements `tabletist_mcp::Gateway` over the saved connections: one session of its own per connection, its secrets from the keyring by a read that never asks the person, one call at a time, a stop sent to the database as the backend's other runs are stopped. The worker gains `Command::Agents` and `Event::Agents`, which start and stop the server. Before the server is linked in, the app's log is made to keep out what the SDK writes of an agent's words.

**Tech Stack:** Rust 2024. `tabletist-mcp` (this workspace's), `tokio-util` and `secret-service`, all three already in `Cargo.lock`: nothing new is built.

---

## Where this sits

Phase 1b of `docs/superpowers/plans/2026-10-13-mcp-server.md` is five runs (section G), of which the first, the server crate, is merged. The master plan said the second might be cut in two: it is.

1. Done: the server, with nothing to start it (`2026-10-13-mcp-server-1b-1-server.md`).
2. **This run, 2a: the backend's gateway.** Agents in `connections.toml`, the gateway, the server in the backend, the app's log.
3. 2b: turning it on from the files, and stdio. The `[mcp]` settings, the app that starts the server and sends it what agents may use, the endpoint, the key and the relay.
4. to 6. The Agents tab, what agents are doing, letting agents in.

After this run nothing a person does reaches the server: no setting starts it, the app sends the backend no command for it, and no transport is served. A `[connections.agents]` table written by hand is read and kept, and does nothing yet.

## Before you start

- Work on a branch of `main`. One commit for each task, in the order below. Every commit passes `cargo fmt --all --check`, `cargo clippy --locked --workspace --all-targets -- -D warnings` and `cargo test --locked --workspace --all-targets`.
- On Igor's machine `cargo` is `~/.cargo/bin/cargo` (the mise shim fails), and a build in a worktree needs `CARGO_TARGET_DIR` set to that worktree's `target/`.
- Read `AGENTS.md`. Four of its rules shape this code: a `match` on `Driver` or `Dialect` names every variant; only `src/env.rs` matches on an `Environment`; nothing an agent sent goes into the app's log; and there are no em dashes.
- The diffs below apply with `git apply`. Run `cargo fmt --all` before each commit: the code here is already formatted.
- **`Cargo.lock` is in the diffs** (Tasks 3 and 4): a line or two in the `tabletist` package's list of dependencies, and nothing else. Every command has `--locked`. A build without it would also move six unrelated `windows-sys` pins: `git checkout Cargo.lock` and apply the diff again.
- **The servers.** Task 4 has tests on PostgreSQL and MySQL, which print "skipped" without their variables:

  ```bash
  export TABLETIST_TEST_PG_URL=postgres://tabletist:tabletist@localhost:55432/tabletist
  export TABLETIST_TEST_MYSQL_URL=mysql://tabletist:tabletist@localhost:53306/tabletist
  ```

  On 2026-10-14 both were up on Igor's machine (`ss -ltn | grep -E '55432|53306'`).

## What was run before this plan was written

Every task's state was built, passed `cargo fmt --check` and clippy with `-D warnings`, and its tests failed and then passed as each step says, with both servers up. The last state passed `cargo test --locked --workspace --all-targets` (3,371 tests, none failed) and `RUSTDOCFLAGS='-D warnings' cargo doc --locked --workspace --no-deps`. Not run:

- **macOS and Windows.** CI runs every test there. One thing in this run is a platform's own: the keyring's quiet read (below).
- **A locked keyring on a desktop.** The Linux read that looks before it reads asks the Secret Service over D-Bus, which a headless test cannot. Its tests are of the gateway with a store that is locked, and of the memory store.
- **The server reached from outside.** Nothing serves a transport yet. The gateway is tested under the real server, through lines of the protocol on a transport in the same process.

## What this run settles

| Question | Decision | Why |
|---|---|---|
| Where a connection's agent settings are kept | `SavedConnection::agents`: `access` (`Off`, `Read`), `rows`, `timeout_secs`. A connection without any is Off, with 500 rows and 10 seconds | A.1 and C.1. A field of the connection, which the editor's Agents section will edit (the fifth run) |
| How the file has them | A `[connections.agents]` table under its `[[connections]]`, written only when it is not what a connection starts with. A key left out takes what a connection starts with. `rows` from 1 to 5,000 and `timeout_secs` from 1 to 60, else the file is refused, naming the connection | B.5 has the editor offer 100, 500, 1,000 or 5,000 rows and 5, 10, 30 or 60 seconds. A file a person edits takes what is safe, not only what the editor offers |
| The file's version | 4 where a connection has agent settings, 3 where none has. Both are read | The file is read strictly: a Tabletist that does not know version 4 refuses the file and keeps it aside. Only a file that holds agent settings is refused by an older version, which would otherwise drop them |
| A connection that is duplicated | Its copy starts Off for agents, and keeps the limits | A new connection starts Off (C.2): a person lets agents in on each connection by itself |
| An edit in the connection dialog | Keeps what agents may do on the connection | The dialog has no control for it until the fifth run, and must not turn agents Off by an edit of the host |
| What an agent calls a connection | Its name and environment ("Bookshop · dev"), or its name where its environment is None. Where two saved connections would be called the same, Off or not, each is called that with the first eight letters and digits of its id ("Bookshop · dev (1f0c2a9e)"), or its whole id where that is still another's name | Open question 19 planned " (2)" on the second. The draft's review found that turning the first Off made the second "Bookshop · dev", and an agent's next call reached another database. A name now never comes to mean another one: at worst it means none |
| Which sessions agents use | One of the gateway's own for each connection an agent has used, opened with `Access::ReadOnly` whatever the connection's box says, closed after ten minutes without a call | A.1. A tab's session runs its own commands one at a time, and a connection with no open tab has none |
| One call at a time | A connection's session is held by one call. Another waits up to the connection's timeout, then is `busy`. A call stopped while it waits stops waiting | A.1 |
| A call that waited across a change | Takes the connection as it is once it has its turn, and again once its connect is done: turned Off meanwhile, it runs nothing; changed, the session is opened anew | Found in the draft's review: a call that waited while the connection was turned Off ran on it |
| A session the database lost | Closed when the call lets it go, so the next call opens it again | Found in the draft's review: a session the server had ended answered "connection lost" to every call after |
| A stop | `StopFlag::stop`, then the session's cancel again and again, each time a little later, until the statement ends or its cleanup has begun, as the backend stops a script. The cancels are awaited before the session is let go. The statement's future is never dropped | A.5. A cancel still on its way must not land on the next call's statement |
| A password | Read from the keyring when the session is opened, and given to nothing else. `PasswordMode::Ask` is refused at once, and so is a keyring with nothing saved | A.1: an agent cannot answer a prompt, and a prompt that came up because an agent asked could fish for the person's password |
| The keyring's prompt to unlock | A read for an agent never brings one up: `SecretStore::read_quietly`. On Linux the item is looked for first, which asks nothing, and a locked one is not read. Windows asks nothing of a read | Found in the draft's review: the Secret Service asks the person to unlock a collection whose item a read finds locked. **On macOS a locked keychain is asked to be unlocked by the system, and the keyring crate has no call that says not to.** That is left open, for the second part, which makes the server reachable |
| How long a connect may take | 20 seconds, an SSH tunnel's among it, and a stop ends the wait | The server's own timer starts once the call has its session (`Gateway::open`) |
| The first schema | `model::first_schema`, the app's rule moved out of `App::apply`, which the workspace's tree and the gateway share | The gateway needs the connection's database name, which only the app's side has |
| Starting the server | `AgentsCommand::Start { log, log_days }` opens the log on the blocking pool and starts the server, which a `Stop` stops. One start at a time. A start that a stop or a later start overtook runs nothing and says nothing, and `Started` is said under the lock a stop takes | Found in the draft's review: two starts raced on the log, one was set aside as broken, and the last word could be a `Failed` while the server ran |
| Retention | As the server starts and every six hours, calls older than `log_days` are forgotten. Every minute the sessions idle for ten minutes are closed | C.1 |
| The app's log | Off for `rmcp`, `tracing` and `sqlparser` at every level. A `RUST_LOG` takes the filter's place as a whole, so the logger's rewriter also blanks every line of theirs it lets through | C.1 and `AGENTS.md`: their lines carry what a client sent. Found in the draft's review: a `RUST_LOG` brought them back |

### What this run leaves out

- **The `[mcp]` settings, and the app that sends the backend `Command::Agents`.** The second part. `Event::Agents` reaches the app and is not acted on.
- **The endpoint, the key and the relay.** The second part.
- **The keychain on macOS** (above). The second part, before anything can reach the server.
- **The website.** `_reference/settings-and-files.md` describes `connections.toml`. Its agents table does nothing until the second part, which documents it with the `[mcp]` keys.
- **Names that differ only in case or spacing.** "Shop  · dev", with two spaces, is a name of its own. A hand-edited connection with an empty name is listed as " · dev" or "". The editor keeps a name from being empty.

## File structure

| File | What changes |
|---|---|
| `src/connections.rs` | `Agents`, `AgentAccess`, their limits, `SavedConnection::agents`. A duplicate starts Off |
| `src/connections/file.rs` | Version 4, the `[connections.agents]` table |
| `src/model.rs` | The dialog keeps `agents`. `first_schema` |
| `src/app.rs`, `src/backend.rs`, `src/entrypoint.rs`, `src/shots.rs`, `src/testing.rs`, `src/ui/env_tests.rs`, `src/ui/mod.rs`, `src/ui/picker.rs` | A saved connection built field by field gets its `agents` |
| `src/entrypoint.rs` | The log's filter and rewriter |
| `src/secrets.rs` | `read_quietly`, and the Linux look before the read |
| `src/backend/agents.rs` | New. `Agentable` and `agentable`, `AppGateway` and `AppSession`, and the server in the worker |
| `src/backend.rs` | `Command::Agents`, `Event::Agents`, the worker's field |
| `src/app.rs` | `first_schema`, and the arm for `Event::Agents` |
| `Cargo.toml`, `Cargo.lock` | `tabletist-mcp`, `tokio-util` and `secret-service`, each explained |
| `.github/workflows/ci.yml` | The PostgreSQL and MySQL jobs run the gateway's tests |
| `docs/superpowers/plans/2026-10-13-mcp-server.md` | What this run built, and where it went otherwise than planned |

---

### Task 1: What agents may do on a connection, and its place in the file

A connection gets its settings for agents, Off until a person says otherwise, and `connections.toml` keeps them. One task: the type has no reader until the file reads it, and the file's tests are of the type.

- `src/connections.rs`: `AgentAccess` (`Off`, `Read`), `Agents` (with the limits a connection starts with), `MAX_AGENT_ROWS` and `MAX_AGENT_TIMEOUT_SECS`, and `SavedConnection::agents`, which the JSON of older versions does not have and which reads as Off. A duplicate starts Off.
- `src/connections/file.rs`: `[connections.agents]`, read with the file's strictness and written only where it is not what a connection starts with. Version 4 where a connection has agent settings, 3 where none has. `tunnel` takes the SSH table out of `write`, which now writes two tables after a connection.
- `src/model.rs`: the dialog keeps a connection's `agents` through an edit.
- Every other place that builds a `SavedConnection` field by field gets `agents: Default::default()`: tests and the demo's connection, 26 in all.

**Files:**
- Modify: `src/connections.rs`, `src/connections/file.rs`, `src/model.rs`
- Modify (one line each): `src/app.rs`, `src/backend.rs`, `src/entrypoint.rs`, `src/shots.rs`, `src/testing.rs`, `src/ui/env_tests.rs`, `src/ui/mod.rs`, `src/ui/picker.rs`

- [ ] **Step 1: Write the failing tests**

````diff
diff --git a/src/app.rs b/src/app.rs
index ed47316..8f0cb6e 100644
--- a/src/app.rs
+++ b/src/app.rs
@@ -4880,6 +4880,7 @@ mod tests {
             password: crate::connections::PasswordMode::None,
             ssh_secret: crate::connections::PasswordMode::None,
             spec: ConnectSpec::sqlite("/tmp/local.db"),
+            agents: Default::default(),
         };
         let id = saved.id.clone();
         app.connections.upsert(saved);
@@ -8372,6 +8373,7 @@ mod tests {
             password: mode,
             ssh_secret: crate::connections::PasswordMode::None,
             spec,
+            agents: Default::default(),
         };
         let id = saved.id.clone();
         app.connections.upsert(saved);
@@ -9356,6 +9358,7 @@ mod tests {
             password: PasswordMode::None,
             ssh_secret: crate::connections::PasswordMode::None,
             spec,
+            agents: Default::default(),
         };
         let conn = saved.id.clone();
         app.connections.upsert(saved);
@@ -9464,6 +9467,7 @@ mod tests {
                 password: PasswordMode::None,
                 ssh_secret: PasswordMode::None,
                 spec,
+                agents: Default::default(),
             };
             let id = saved.id.clone();
             app.connections.upsert(saved);
diff --git a/src/backend.rs b/src/backend.rs
index d1c6754..93e44b6 100644
--- a/src/backend.rs
+++ b/src/backend.rs
@@ -4389,6 +4389,7 @@ mod tests {
             password: crate::connections::PasswordMode::None,
             ssh_secret: crate::connections::PasswordMode::None,
             spec: ConnectSpec::sqlite("/tmp/a.db"),
+            agents: Default::default(),
         });
         StateFile::Connections(connections)
     }
diff --git a/src/connections.rs b/src/connections.rs
index 3ea5d30..9ff347e 100644
--- a/src/connections.rs
+++ b/src/connections.rs
@@ -619,6 +619,7 @@ mod tests {
             password: PasswordMode::None,
             ssh_secret: PasswordMode::None,
             spec: ConnectSpec::sqlite(file),
+            agents: Default::default(),
         }
     }
 
@@ -989,6 +990,23 @@ mod tests {
         assert!(store.duplicate(&original.id).is_none());
     }
 
+    #[test]
+    fn a_duplicate_is_off_for_agents_and_keeps_its_limits() {
+        let mut store = SavedConnections::default();
+        let mut original = saved("Bookshop", "/b.db");
+        original.agents = Agents {
+            access: AgentAccess::Read,
+            rows: 1_000,
+            timeout_secs: 30,
+        };
+        store.upsert(original.clone());
+        let copy = store.duplicate(&original.id).unwrap();
+        let copied = store.get(&copy).unwrap().agents;
+        assert_eq!(copied.access, AgentAccess::Off);
+        assert_eq!((copied.rows, copied.timeout_secs), (1_000, 30));
+        assert_eq!(store.get(&original.id).unwrap().agents, original.agents);
+    }
+
     #[test]
     fn search_matches_name_and_summary_case_insensitively() {
         let mut store = SavedConnections::default();
diff --git a/src/connections/file.rs b/src/connections/file.rs
index 4abd210..7c0b355 100644
--- a/src/connections/file.rs
+++ b/src/connections/file.rs
@@ -390,6 +390,7 @@ mod tests {
             password: PasswordMode::None,
             ssh_secret: PasswordMode::None,
             spec,
+            agents: Default::default(),
         }
     }
 
@@ -677,9 +678,9 @@ auth = "key-file"
                 "unknown field `read_onyl`",
             ),
             (
-                format!("{entry}\n[connections.agents]\naccess = \"read\"\n"),
-                8,
-                "unknown field `agents`",
+                format!("{entry}\n[connections.agents]\naccess = \"read\"\nrowz = 9\n"),
+                10,
+                "unknown field `rowz`",
             ),
             (
                 format!(
@@ -781,8 +782,8 @@ auth = "key-file"
         let said = |text: &str| read(text).unwrap_err().to_string();
         assert!(said("").contains("missing field `version`"), "{}", said(""));
         assert_eq!(
-            said("version = 4\n"),
-            "it is version 4 of the file, written by a newer Tabletist; this one reads version 3"
+            said("version = 5\n"),
+            "it is version 5 of the file, written by a newer Tabletist; this one reads version 4"
         );
         assert_eq!(
             said("version = 2\n"),
@@ -790,6 +791,105 @@ auth = "key-file"
         );
     }
 
+    fn with_agents(access: AgentAccess, rows: u32, timeout_secs: u32) -> Vec<SavedConnection> {
+        let mut connections = bookshop();
+        connections[2].agents = Agents {
+            access,
+            rows,
+            timeout_secs,
+        };
+        connections
+    }
+
+    #[test]
+    fn what_agents_may_do_is_a_table_of_its_own_in_version_four() {
+        let connections = with_agents(AgentAccess::Read, 1_000, 30);
+        let text = write(&connections).unwrap();
+        assert!(text.contains("\nversion = 4\n"), "{text}");
+        assert!(
+            text.ends_with(
+                r#"sqlite_path = "/home/ada/bookshop.db"
+
+[connections.agents]
+access       = "read"
+rows         = 1000
+timeout_secs = 30
+"#
+            ),
+            "{text}"
+        );
+        assert_eq!(read(&text).unwrap().connections, connections);
+        // Limits set while access is Off are kept, and so is the table.
+        let off = with_agents(AgentAccess::Off, 100, 10);
+        let text = write(&off).unwrap();
+        assert!(text.contains("access       = \"off\""), "{text}");
+        assert_eq!(read(&text).unwrap().connections, off);
+    }
+
+    #[test]
+    fn a_file_without_agents_is_written_and_read_as_version_three() {
+        let text = write(&bookshop()).unwrap();
+        assert!(
+            text.contains("\nversion = 3\n") && !text.contains("agents"),
+            "{text}"
+        );
+        let read = read(&text).unwrap();
+        assert!(
+            read.connections
+                .iter()
+                .all(|saved| saved.agents == Agents::default())
+        );
+        // And one of version 4 without them is read the same.
+        let four = text.replace("\nversion = 3\n", "\nversion = 4\n");
+        assert_eq!(super::read(&four).unwrap().connections, bookshop());
+    }
+
+    #[test]
+    fn what_agents_may_do_takes_a_connections_start_for_a_key_left_out() {
+        let text = format!(
+            "{}\n[connections.agents]\naccess = \"read\"\n",
+            write(&bookshop()).unwrap()
+        );
+        let agents = read(&text).unwrap().connections[2].agents;
+        assert_eq!(
+            agents,
+            Agents {
+                access: AgentAccess::Read,
+                ..Agents::default()
+            }
+        );
+        // A table that says nothing is Off.
+        let empty = format!("{}\n[connections.agents]\n", write(&bookshop()).unwrap());
+        assert_eq!(
+            read(&empty).unwrap().connections[2].agents,
+            Agents::default()
+        );
+    }
+
+    #[test]
+    fn what_agents_may_do_is_refused_outside_its_limits() {
+        let said = |table: &str| {
+            let text = format!(
+                "{}\n[connections.agents]\n{table}\n",
+                write(&bookshop()).unwrap()
+            );
+            read(&text).unwrap_err().to_string()
+        };
+        let rows = "connection \"Bookshop on disk\": agents' `rows` is from 1 to 5000";
+        assert_eq!(said("rows = 0"), rows);
+        assert_eq!(said("rows = 5001"), rows);
+        assert_eq!(
+            said("timeout_secs = 61"),
+            "connection \"Bookshop on disk\": agents' `timeout_secs` is from 1 to 60"
+        );
+        // An access there is not, and a key there is not, without the value.
+        let access = said("access = \"read_and_write\"");
+        assert!(access.contains("a value the key cannot have"), "{access}");
+        assert!(!access.contains("read_and_write"), "{access}");
+        let key = said("rowz = 9");
+        assert!(key.contains("unknown field `rowz`"), "{key}");
+    }
+
     #[test]
     fn every_connection_has_an_id_of_its_own() {
         let entry = |id: &str, name: &str| {
diff --git a/src/model.rs b/src/model.rs
index 2e80436..0c5e28c 100644
--- a/src/model.rs
+++ b/src/model.rs
@@ -4677,6 +4677,24 @@ mod tests {
         assert!(back.ssh_secret_is_stale(), "another port, another server");
     }
 
+    #[test]
+    fn an_edit_keeps_what_agents_may_do_on_the_connection() {
+        let mut saved = ssh_form().to_saved().unwrap();
+        saved.agents = crate::connections::Agents {
+            access: crate::connections::AgentAccess::Read,
+            rows: 1_000,
+            timeout_secs: 30,
+        };
+        let mut edited = ConnectionForm::from_saved(&saved);
+        edited.name = "Production".into();
+        let again = edited.to_saved().unwrap();
+        assert_eq!(again.name, "Production");
+        assert_eq!(again.agents, saved.agents);
+        // A new connection starts Off.
+        let new = ssh_form().to_saved().unwrap();
+        assert_eq!(new.agents, crate::connections::Agents::default());
+    }
+
     #[test]
     fn a_new_form_leaves_the_ssh_port_to_the_config() {
         assert_eq!(ConnectionForm::default().ssh_port, "");
diff --git a/src/ui/mod.rs b/src/ui/mod.rs
index 5d1f2a7..f221129 100644
--- a/src/ui/mod.rs
+++ b/src/ui/mod.rs
@@ -2872,6 +2872,7 @@ mod tests {
             password: crate::connections::PasswordMode::None,
             ssh_secret: crate::connections::PasswordMode::None,
             spec: tabletist_db::ConnectSpec::sqlite(format!("/tmp/{name}.db")),
+            agents: Default::default(),
         };
         let id = saved.id.clone();
         harness.app.connections.upsert(saved);
@@ -2973,6 +2974,7 @@ mod tests {
                     password: crate::connections::PasswordMode::None,
                     ssh_secret: crate::connections::PasswordMode::None,
                     spec,
+                    agents: Default::default(),
                 });
             let tree = harness.settle();
             // The row may cut the name short, so find it by how it starts.
@@ -3044,6 +3046,7 @@ mod tests {
                 password: crate::connections::PasswordMode::Ask,
                 ssh_secret: crate::connections::PasswordMode::None,
                 spec,
+                agents: Default::default(),
             });
         harness.click("Connect to Production");
         harness.click("Cancel");
@@ -3436,6 +3439,7 @@ mod tests {
                 password: crate::connections::PasswordMode::None,
                 ssh_secret: crate::connections::PasswordMode::None,
                 spec,
+                agents: Default::default(),
             });
         harness.click("Connect to Production");
         let Command::Connect {
@@ -7822,6 +7826,7 @@ mod tests {
             password: crate::connections::PasswordMode::Ask,
             ssh_secret: crate::connections::PasswordMode::None,
             spec,
+            agents: Default::default(),
         };
         let conn = saved.id.clone();
         harness.app.connections.upsert(saved);
@@ -7854,6 +7859,7 @@ mod tests {
             password: crate::connections::PasswordMode::Ask,
             ssh_secret: crate::connections::PasswordMode::None,
             spec,
+            agents: Default::default(),
         };
         let conn = saved.id.clone();
         harness.app.connections.upsert(saved);
@@ -8421,6 +8427,7 @@ mod tests {
             password: crate::connections::PasswordMode::None,
             ssh_secret: crate::connections::PasswordMode::None,
             spec,
+            agents: Default::default(),
         };
         let id = saved.id.clone();
         harness.app.connections.upsert(saved);
@@ -9329,6 +9336,7 @@ mod tests {
             password: crate::connections::PasswordMode::None,
             ssh_secret: crate::connections::PasswordMode::None,
             spec,
+            agents: Default::default(),
         };
         let conn = saved.id.clone();
         harness.app.connections.upsert(saved);
@@ -9354,6 +9362,7 @@ mod tests {
             password: crate::connections::PasswordMode::None,
             ssh_secret: crate::connections::PasswordMode::None,
             spec,
+            agents: Default::default(),
         };
         let conn = saved.id.clone();
         harness.app.connections.upsert(saved);
diff --git a/src/ui/picker.rs b/src/ui/picker.rs
index b9024b5..78e1732 100644
--- a/src/ui/picker.rs
+++ b/src/ui/picker.rs
@@ -1138,6 +1138,7 @@ mod tests {
             password: crate::connections::PasswordMode::None,
             ssh_secret: crate::connections::PasswordMode::None,
             spec: tabletist_db::ConnectSpec::sqlite("/tmp/fixture.db"),
+            agents: Default::default(),
         };
         let picker = crate::model::ConnTabId(1);
         let open = crate::model::ConnTabId(2);
````

- [ ] **Step 2: Run them to see that they do not build**

Run: `cargo test --locked --lib connections::`

Expected: it does not build. The tests name what is not there yet:

```text
error[E0433]: cannot find `AgentAccess` in `connections`
error[E0433]: cannot find `Agents` in `connections`
error[E0560]: struct `connections::SavedConnection` has no field named `agents`
error[E0609]: no field `agents` on type `connections::SavedConnection`
error: could not compile `tabletist` (lib test) due to 44 previous errors
```

- [ ] **Step 3: The settings, their file, and the dialog**

````diff
diff --git a/src/connections.rs b/src/connections.rs
index 9ff347e..99cc1ae 100644
--- a/src/connections.rs
+++ b/src/connections.rs
@@ -36,6 +36,62 @@ pub enum PasswordMode {
     Ask,
 }
 
+/// What agents may do on a connection, through the MCP server.
+#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
+#[serde(rename_all = "snake_case")]
+pub enum AgentAccess {
+    /// Agents do not see the connection at all.
+    #[default]
+    Off,
+    /// Agents may read it, in read-only transactions.
+    Read,
+}
+
+impl AgentAccess {
+    /// The word a file keeps it as.
+    pub fn key(self) -> &'static str {
+        match self {
+            Self::Off => "off",
+            Self::Read => "read",
+        }
+    }
+}
+
+/// A connection's settings for agents. A connection that was never given
+/// any is Off, with the limits every connection starts with.
+#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
+#[serde(default)]
+pub struct Agents {
+    pub access: AgentAccess,
+    /// The most rows of an answer.
+    pub rows: u32,
+    /// How long a statement of an agent's may run.
+    pub timeout_secs: u32,
+}
+
+/// The most rows an agent's answer may be given: half of what the SQL
+/// editor may keep.
+pub const MAX_AGENT_ROWS: u32 = 5_000;
+
+/// The longest an agent's statement may be given to run.
+pub const MAX_AGENT_TIMEOUT_SECS: u32 = 60;
+
+impl Default for Agents {
+    fn default() -> Self {
+        Self {
+            access: AgentAccess::Off,
+            rows: 500,
+            timeout_secs: 10,
+        }
+    }
+}
+
+impl Agents {
+    pub fn is_default(&self) -> bool {
+        *self == Self::default()
+    }
+}
+
 #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
 pub struct SavedConnection {
     pub id: ConnectionId,
@@ -52,6 +108,10 @@ pub struct SavedConnection {
     #[serde(default)]
     pub ssh_secret: PasswordMode,
     pub spec: ConnectSpec,
+    /// What agents may do on it. The JSON of older versions has none, and
+    /// reads as Off.
+    #[serde(default, skip_serializing_if = "Agents::is_default")]
+    pub agents: Agents,
 }
 
 impl SavedConnection {
@@ -82,7 +142,8 @@ pub fn access(read_only: Option<bool>, environment: Environment) -> tabletist_db
 
 /// The format of `connections.toml`. 1 and 2 were `connections.json`: 2
 /// dropped the colour a connection used to keep beside its environment,
-/// and the `test` environment. 3 is the first that is TOML.
+/// and the `test` environment. 3 is the first that is TOML. 4 has what
+/// agents may do on a connection, and a file without that is still 3.
 pub const VERSION: u32 = file::VERSION;
 
 #[derive(Debug, Clone, Default, PartialEq)]
@@ -246,6 +307,7 @@ impl StoredConnection {
             password: self.password,
             ssh_secret: self.ssh_secret,
             spec: self.spec,
+            agents: Default::default(),
         };
         (saved, note)
     }
@@ -513,6 +575,9 @@ impl SavedConnections {
         let mut copy = self.connections[index].clone();
         copy.id = ConnectionId::new();
         copy.name = format!("{} copy", copy.name);
+        // A new connection, which starts Off for agents as every new one
+        // does: the person lets agents in on each connection by itself.
+        copy.agents.access = AgentAccess::Off;
         let new_id = copy.id.clone();
         self.connections.insert(index + 1, copy);
         Some(new_id)
diff --git a/src/connections/file.rs b/src/connections/file.rs
index 7c0b355..97439e4 100644
--- a/src/connections/file.rs
+++ b/src/connections/file.rs
@@ -13,13 +13,21 @@ use std::path::{Path, PathBuf};
 use serde::Deserialize;
 use tabletist_db::{ConnectSpec, Driver, SshAuth, SshSpec, TlsMode};
 
-use super::{ConnectionId, PasswordMode, SavedConnection};
+use super::{
+    AgentAccess, Agents, ConnectionId, MAX_AGENT_ROWS, MAX_AGENT_TIMEOUT_SECS, PasswordMode,
+    SavedConnection,
+};
 use crate::env::Environment;
 use crate::util::toml_string;
 
 /// The file's format. 3 is the first that is TOML: 1 and 2 were
-/// `connections.json`.
-pub(super) const VERSION: u32 = 3;
+/// `connections.json`. 4 has a connection's settings for agents, and a file
+/// that has none is written as 3: a Tabletist that does not know them
+/// refuses only a file it would lose them from.
+pub(super) const VERSION: u32 = 4;
+
+/// The first version of this file.
+const FIRST_VERSION: u32 = 3;
 
 /// The file's name, as the user is told it.
 pub(super) const NAME: &str = "connections.toml";
@@ -94,6 +102,18 @@ struct Entry {
     tls: TlsMode,
     ca_file: Option<PathBuf>,
     ssh: Option<Tunnel>,
+    agents: Option<AgentsTable>,
+}
+
+/// What agents may do on a connection, as the file has it. A key left out
+/// takes what a connection starts with.
+#[derive(Deserialize)]
+#[serde(deny_unknown_fields)]
+struct AgentsTable {
+    #[serde(default)]
+    access: AgentAccess,
+    rows: Option<u32>,
+    timeout_secs: Option<u32>,
 }
 
 #[derive(Deserialize)]
@@ -157,9 +177,9 @@ pub(super) fn read(text: &str) -> Result<Read, Problem> {
             file.version
         )));
     }
-    if file.version < VERSION {
+    if file.version < FIRST_VERSION {
         return Err(whole(format!(
-            "it says version {}, and the first version of this file is {VERSION}",
+            "it says version {}, and the first version of this file is {FIRST_VERSION}",
             file.version
         )));
     }
@@ -196,6 +216,10 @@ impl Entry {
             Some(tunnel) => Some(tunnel.spec(&self.name)?),
             None => None,
         };
+        let agents = match self.agents {
+            Some(table) => table.agents(&self.name)?,
+            None => Agents::default(),
+        };
         Ok(SavedConnection {
             id,
             name: self.name,
@@ -214,10 +238,34 @@ impl Entry {
                 ca_file: self.ca_file,
                 ssh,
             },
+            agents,
         })
     }
 }
 
+impl AgentsTable {
+    fn agents(self, connection: &str) -> Result<Agents, String> {
+        let starts = Agents::default();
+        let agents = Agents {
+            access: self.access,
+            rows: self.rows.unwrap_or(starts.rows),
+            timeout_secs: self.timeout_secs.unwrap_or(starts.timeout_secs),
+        };
+        if !(1..=MAX_AGENT_ROWS).contains(&agents.rows) {
+            return Err(format!(
+                "connection {connection:?}: agents' `rows` is from 1 to {MAX_AGENT_ROWS}"
+            ));
+        }
+        if !(1..=MAX_AGENT_TIMEOUT_SECS).contains(&agents.timeout_secs) {
+            return Err(format!(
+                "connection {connection:?}: agents' `timeout_secs` is from 1 to \
+                 {MAX_AGENT_TIMEOUT_SECS}"
+            ));
+        }
+        Ok(agents)
+    }
+}
+
 impl Tunnel {
     fn spec(self, connection: &str) -> Result<SshSpec, String> {
         let auth = match (self.auth, self.key) {
@@ -306,7 +354,11 @@ fn tls(mode: TlsMode) -> &'static str {
 /// except a server's own (host, port, user, database, TLS), which a person
 /// looks for.
 pub(super) fn write(connections: &[SavedConnection]) -> std::io::Result<String> {
-    let mut out = format!("{HEADER}version = {VERSION}\n");
+    let version = match connections.iter().any(|saved| !saved.agents.is_default()) {
+        true => VERSION,
+        false => FIRST_VERSION,
+    };
+    let mut out = format!("{HEADER}version = {version}\n");
     for connection in connections {
         let spec = &connection.spec;
         let server = match spec.driver {
@@ -350,33 +402,46 @@ pub(super) fn write(connections: &[SavedConnection]) -> std::io::Result<String>
         }
         out.push_str("\n[[connections]]\n");
         lines.write(&mut out);
-        let Some(ssh) = &spec.ssh else {
-            continue;
-        };
-        let mut lines = Lines::default();
-        lines.text("host", &ssh.host);
-        if let Some(port) = ssh.port {
-            lines.bare("port", port);
+        if let Some(ssh) = &spec.ssh {
+            tunnel(ssh, &mut out)?;
         }
-        if !ssh.user.is_empty() {
-            lines.text("user", &ssh.user);
+        if !connection.agents.is_default() {
+            let mut lines = Lines::default();
+            lines.text("access", connection.agents.access.key());
+            lines.bare("rows", connection.agents.rows);
+            lines.bare("timeout_secs", connection.agents.timeout_secs);
+            out.push_str("\n[connections.agents]\n");
+            lines.write(&mut out);
         }
-        match &ssh.auth {
-            SshAuth::Password => lines.text("auth", "password"),
-            SshAuth::Agent => lines.text("auth", "agent"),
-            SshAuth::KeyFile { path } => {
-                lines.text("auth", "key-file");
-                if !path.as_os_str().is_empty() {
-                    lines.path("key", path)?;
-                }
-            }
-        }
-        out.push_str("\n[connections.ssh]\n");
-        lines.write(&mut out);
     }
     Ok(out)
 }
 
+/// A connection's SSH tunnel, as a table of its own.
+fn tunnel(ssh: &SshSpec, out: &mut String) -> std::io::Result<()> {
+    let mut lines = Lines::default();
+    lines.text("host", &ssh.host);
+    if let Some(port) = ssh.port {
+        lines.bare("port", port);
+    }
+    if !ssh.user.is_empty() {
+        lines.text("user", &ssh.user);
+    }
+    match &ssh.auth {
+        SshAuth::Password => lines.text("auth", "password"),
+        SshAuth::Agent => lines.text("auth", "agent"),
+        SshAuth::KeyFile { path } => {
+            lines.text("auth", "key-file");
+            if !path.as_os_str().is_empty() {
+                lines.path("key", path)?;
+            }
+        }
+    }
+    out.push_str("\n[connections.ssh]\n");
+    lines.write(out);
+    Ok(())
+}
+
 #[cfg(test)]
 mod tests {
     use super::*;
diff --git a/src/entrypoint.rs b/src/entrypoint.rs
index 2b5b6c8..20eb69d 100644
--- a/src/entrypoint.rs
+++ b/src/entrypoint.rs
@@ -204,6 +204,7 @@ pub fn demo_setup(app: &mut App) {
         password: crate::connections::PasswordMode::None,
         ssh_secret: crate::connections::PasswordMode::None,
         spec: tabletist_db::ConnectSpec::sqlite(&path),
+        agents: Default::default(),
     };
     let conn = saved.id.clone();
     app.connections.upsert(saved);
diff --git a/src/model.rs b/src/model.rs
index 0c5e28c..477dcfe 100644
--- a/src/model.rs
+++ b/src/model.rs
@@ -922,6 +922,9 @@ pub struct ConnectionForm {
     pub environment: Option<crate::env::Environment>,
     /// Read-only as saved: `None` until the user sets it.
     pub read_only: Option<bool>,
+    /// What agents may do on the connection, as it was saved. The dialog
+    /// has no control for it yet, so an edit keeps it.
+    pub agents: crate::connections::Agents,
     /// Put the keyboard in Name on the next frame (a new connection).
     pub focus_name: bool,
     pub driver: Driver,
@@ -1032,6 +1035,7 @@ impl Default for ConnectionForm {
             name: String::new(),
             environment: None,
             read_only: None,
+            agents: crate::connections::Agents::default(),
             focus_name: true,
             driver: Driver::Sqlite,
             sqlite_path: String::new(),
@@ -1170,6 +1174,7 @@ impl ConnectionForm {
             name: saved.name.clone(),
             environment: Some(saved.environment),
             read_only: saved.read_only,
+            agents: saved.agents,
             focus_name: false,
             driver: spec.driver,
             sqlite_path: spec
@@ -1416,6 +1421,7 @@ impl ConnectionForm {
             password,
             ssh_secret,
             spec,
+            agents: self.agents,
         })
     }
 }
diff --git a/src/shots.rs b/src/shots.rs
index a55b9fc..21da770 100644
--- a/src/shots.rs
+++ b/src/shots.rs
@@ -37,6 +37,7 @@ fn saved() -> SavedConnection {
         password: PasswordMode::None,
         ssh_secret: PasswordMode::None,
         spec,
+        agents: Default::default(),
     }
 }
 
@@ -73,6 +74,7 @@ fn edit_production(harness: &mut Harness) {
         password: PasswordMode::Keyring,
         ssh_secret: PasswordMode::None,
         spec,
+        agents: Default::default(),
     });
     harness.app.apply(Action::EditConnection(id));
     if let Some(crate::model::Dialog::Connection(form)) = &mut harness.app.dialog {
@@ -2304,6 +2306,7 @@ mod mock {
                 password: PasswordMode::None,
                 ssh_secret: PasswordMode::None,
                 spec,
+                agents: Default::default(),
             });
             if let Some(used) = used {
                 harness.app.connections.mark_used(&id, used);
diff --git a/src/testing.rs b/src/testing.rs
index 0505632..a096f1e 100644
--- a/src/testing.rs
+++ b/src/testing.rs
@@ -516,6 +516,7 @@ fn fixture_connection() -> SavedConnection {
         password: crate::connections::PasswordMode::None,
         ssh_secret: crate::connections::PasswordMode::None,
         spec: ConnectSpec::sqlite("/tmp/fixture.db"),
+        agents: Default::default(),
     }
 }
 
diff --git a/src/ui/env_tests.rs b/src/ui/env_tests.rs
index fda9430..60d8902 100644
--- a/src/ui/env_tests.rs
+++ b/src/ui/env_tests.rs
@@ -136,6 +136,7 @@ fn the_connections_list_shows_each_environment_in_its_colours() {
                     password: crate::connections::PasswordMode::None,
                     ssh_secret: crate::connections::PasswordMode::None,
                     spec: tabletist_db::ConnectSpec::sqlite(format!("/tmp/bookshop-{env:?}.db")),
+                    agents: Default::default(),
                 });
         }
         harness.settle();
````

- [ ] **Step 4: Run the tests**

Run: `cargo test --locked --lib connections::`, then `cargo test --locked --lib model::tests::an_edit_keeps`

Expected: PASS.

```text
test result: ok. 39 passed; 0 failed; 0 ignored; 0 measured; 2376 filtered out
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 2414 filtered out
```

- [ ] **Step 5: Commit**

```bash
cargo fmt --all
cargo clippy --locked --workspace --all-targets -- -D warnings
git add src
git commit -m "Keep what agents may do on a connection, Off until a person says"
```

---

### Task 2: The app's log keeps out an agent's words

Before the server is linked into the app (Task 4), its SDK must not write into `tabletist.log`. `rmcp` writes what a client sent from `info` up and every request whole at `debug`, `tracing` writes lines of its own for the SDK's spans, and `sqlparser` writes each word of an agent's statement at `debug`.

- `log_filter` is the filter at both levels, with the three off.
- `without_agents_words` is the logger's rewriter: a line of theirs that a `RUST_LOG` lets through is written as "(a line left out: it can hold what an agent sent)". A `RUST_LOG` takes the filter's place as a whole, and the rewriter is asked of every line whatever the filter was.

**Files:**
- Modify: `src/entrypoint.rs`

- [ ] **Step 1: Write the failing tests**

````diff
diff --git a/src/entrypoint.rs b/src/entrypoint.rs
index 20eb69d..0cfb7cd 100644
--- a/src/entrypoint.rs
+++ b/src/entrypoint.rs
@@ -389,6 +389,50 @@ impl eframe::App for Window {
 mod tests {
     use super::*;
 
+    #[test]
+    fn a_line_that_can_hold_an_agents_words_is_written_without_them() {
+        let line = |target: &str| {
+            let record = log::Record::builder()
+                .target(target)
+                .level(log::Level::Debug)
+                .args(format_args!("received request SELECT email FROM customers"))
+                .build();
+            without_agents_words(&record, "received request SELECT email FROM customers")
+        };
+        for target in [
+            "rmcp",
+            "rmcp::service",
+            "tracing::span",
+            "sqlparser::parser",
+        ] {
+            let written = line(target).expect(target);
+            assert!(!written.contains("SELECT"), "{target}: {written}");
+        }
+        // Every other crate's line is as it was, the app's own among them.
+        for target in [
+            "tabletist",
+            "tabletist_mcp::server",
+            "zbus",
+            "rmcpx",
+            "tracingfoo",
+        ] {
+            assert_eq!(line(target), None, "{target}");
+        }
+    }
+
+    #[test]
+    fn the_log_takes_nothing_of_what_writes_an_agents_words() {
+        for verbose in [false, true] {
+            let filter = log_filter(verbose);
+            for theirs in AGENTS_WORDS {
+                let off = format!("{theirs}=off");
+                assert!(filter.split(',').any(|said| said == off), "{filter}");
+            }
+            // And the app's own crates are as they were.
+            assert!(filter.contains("tabletist="), "{filter}");
+        }
+    }
+
     #[test]
     fn demo_mode_never_uses_the_os_keyring() {
         assert!(!keyring_for(true).is_native());
````

- [ ] **Step 2: Run them to see that they do not build**

Run: `cargo test --locked --lib entrypoint::`

Expected: it does not build:

```text
error[E0425]: cannot find value `AGENTS_WORDS` in this scope
error[E0425]: cannot find function `without_agents_words` in this scope
error[E0425]: cannot find function `log_filter` in this scope
error: could not compile `tabletist` (lib test) due to 3 previous errors
```

- [ ] **Step 3: The filter and the rewriter**

````diff
diff --git a/src/entrypoint.rs b/src/entrypoint.rs
index 0cfb7cd..fc66d38 100644
--- a/src/entrypoint.rs
+++ b/src/entrypoint.rs
@@ -91,13 +91,9 @@ pub fn main() -> anyhow::Result<()> {
         Some(root) => AppDirs::at(root.path()),
         None => AppDirs::discover(),
     };
-    let mut logging = fastframe_log::Logging::new("tabletist", env!("CARGO_PKG_VERSION")).filter(
-        if cli.verbose {
-            "info,tabletist=debug,tabletist_db=debug"
-        } else {
-            "warn,tabletist=info"
-        },
-    );
+    let mut logging = fastframe_log::Logging::new("tabletist", env!("CARGO_PKG_VERSION"))
+        .filter(log_filter(cli.verbose))
+        .redact(without_agents_words);
     // fastframe-log would create the log's folder itself, but with default
     // permissions; made here first, it is private to the user.
     if let Err(error) = dirs.ensure() {
@@ -385,6 +381,39 @@ impl eframe::App for Window {
     }
 }
 
+/// What the app's log takes, at its usual level and at `--verbose`.
+///
+/// Never a line of the MCP server's SDK (`rmcp`, and `tracing`, which
+/// writes lines of its own for the SDK's spans) or of `sqlparser`, which
+/// reads an agent's statement: they write what a client sent, a
+/// statement and its values among it, and `tabletist.log` holds none of
+/// that (`AGENTS.md`). `RUST_LOG`, where it is set, takes the filter's
+/// place as a whole: [`without_agents_words`] is what holds then.
+fn log_filter(verbose: bool) -> &'static str {
+    match verbose {
+        true => "info,tabletist=debug,tabletist_db=debug,rmcp=off,tracing=off,sqlparser=off",
+        false => "warn,tabletist=info,rmcp=off,tracing=off,sqlparser=off",
+    }
+}
+
+/// The crates whose lines can hold what an agent sent.
+const AGENTS_WORDS: [&str; 3] = ["rmcp", "tracing", "sqlparser"];
+
+/// A line of one of [`AGENTS_WORDS`] without its words, for when a
+/// `RUST_LOG` lets it through: that the line was there, and nothing of it.
+fn without_agents_words(
+    record: &log::Record<'_>,
+    _message: &str,
+) -> Option<std::borrow::Cow<'static, str>> {
+    let target = record.target();
+    let theirs = AGENTS_WORDS
+        .iter()
+        .any(|crate_name| target == *crate_name || target.starts_with(&format!("{crate_name}::")));
+    theirs.then_some(std::borrow::Cow::Borrowed(
+        "(a line left out: it can hold what an agent sent)",
+    ))
+}
+
 #[cfg(test)]
 mod tests {
     use super::*;
````

- [ ] **Step 4: Run the tests**

Run: `cargo test --locked --lib entrypoint::`

Expected: PASS.

```text
test result: ok. 12 passed; 0 failed; 0 ignored; 0 measured; 2405 filtered out
```

- [ ] **Step 5: Commit**

```bash
cargo fmt --all
cargo clippy --locked --workspace --all-targets -- -D warnings
git add src/entrypoint.rs
git commit -m "Keep what the MCP server's SDK writes of an agent out of the app's log"
```

---

### Task 3: A read of the keyring that asks the person nothing

The gateway reads a connection's password when an agent's call opens it, with nobody at the screen who asked. On Linux the Secret Service would ask the person to unlock a collection whose item the read finds locked. `SecretStore::read_quietly` is the read that never asks: on Linux it looks for the item first, which asks nothing, and does not read a locked one. Every other store reads as `read` does: the memory store, and Windows, which asks nothing of a read. macOS is the open case (see "What this run settles").

`secret-service` is the crate the Linux keyring store is built on, with the same features: one line in `Cargo.lock`, and nothing built that was not.

**Files:**
- Modify: `src/secrets.rs`, `Cargo.toml`, `Cargo.lock`

- [ ] **Step 1: Write the failing test**

````diff
diff --git a/Cargo.lock b/Cargo.lock
index f7573f9..aecc327 100644
--- a/Cargo.lock
+++ b/Cargo.lock
@@ -5694,6 +5694,7 @@ dependencies = [
  "objc2-app-kit 0.3.2",
  "objc2-foundation 0.3.2",
  "rfd",
+ "secret-service",
  "serde",
  "serde_json",
  "skrifa",
diff --git a/Cargo.toml b/Cargo.toml
index bc1f517..2ebfe2c 100644
--- a/Cargo.toml
+++ b/Cargo.toml
@@ -102,6 +102,11 @@ notify = "8.2"
 [target.'cfg(target_os = "linux")'.dependencies]
 # The desktop's Secret Service (GNOME Keyring, KWallet) over D-Bus.
 zbus-secret-service-keyring-store = { version = "1.0.1", features = ["rt-tokio-crypto-rust"] }
+# The Secret Service itself, which the store above is built on, with the
+# same features: nothing new is built. To ask whether an entry is locked
+# before it is read, which the store's read would answer with a prompt to
+# unlock it (`secrets::linux_locked`), for a read an agent's call made.
+secret-service = { version = "5", features = ["rt-tokio-crypto-rust"] }
 # Checks that the desktop's monospace font parses before egui loads it (egui
 # panics on a font it cannot read). The version epaint already uses.
 skrifa = { version = "0.44", default-features = false, features = ["std"] }
diff --git a/src/secrets.rs b/src/secrets.rs
index 84ed8ab..9800296 100644
--- a/src/secrets.rs
+++ b/src/secrets.rs
@@ -227,6 +227,21 @@ impl Keyring {
 mod tests {
     use super::*;
 
+    #[tokio::test]
+    async fn a_quiet_read_of_a_store_that_never_asks_is_a_read() {
+        let keyring = Keyring::memory();
+        assert_eq!(keyring.read_quietly("a").wait().await, Ok(None));
+        keyring
+            .write("a", SecretString("hunter2".into()))
+            .wait()
+            .await
+            .unwrap();
+        assert_eq!(
+            keyring.read_quietly("a").wait().await,
+            Ok(Some("hunter2".into()))
+        );
+    }
+
     #[tokio::test]
     async fn the_memory_keyring_stores_reads_and_deletes() {
         let keyring = Keyring::memory();
````

- [ ] **Step 2: Run it to see that it does not build**

Run: `cargo test --locked --lib secrets::`

Expected: it does not build:

```text
error[E0599]: no method named `read_quietly` found for struct `secrets::Keyring` in the current scope
error: could not compile `tabletist` (lib test) due to 2 previous errors
```

- [ ] **Step 3: The quiet read**

````diff
diff --git a/src/secrets.rs b/src/secrets.rs
index 9800296..818b2fc 100644
--- a/src/secrets.rs
+++ b/src/secrets.rs
@@ -56,6 +56,12 @@ impl fmt::Display for SecretError {
 
 pub trait SecretStore: Send {
     fn read(&mut self, account: &str) -> Result<Option<String>, SecretError>;
+    /// Reads without asking the person for anything: where the store would
+    /// have to be unlocked first, it answers [`SecretError::Locked`] and
+    /// asks nothing. For a read nobody at the screen asked for.
+    fn read_quietly(&mut self, account: &str) -> Result<Option<String>, SecretError> {
+        self.read(account)
+    }
     fn write(&mut self, account: &str, secret: &str) -> Result<(), SecretError>;
     fn delete(&mut self, account: &str) -> Result<(), SecretError>;
 }
@@ -122,6 +128,20 @@ impl SecretStore for NativeStore {
         }
     }
 
+    /// On Linux the Secret Service asks the person to unlock a collection
+    /// whose item a read finds locked: so the items are looked for first,
+    /// which asks nothing, and a locked one is not read. Windows asks
+    /// nothing of a read. On macOS a locked keychain is asked to be
+    /// unlocked by the system, and no call of the keyring crate says not
+    /// to: that is not kept from happening here.
+    fn read_quietly(&mut self, account: &str) -> Result<Option<String>, SecretError> {
+        #[cfg(target_os = "linux")]
+        if linux_locked(account)? {
+            return Err(SecretError::Locked);
+        }
+        self.read(account)
+    }
+
     fn write(&mut self, account: &str, secret: &str) -> Result<(), SecretError> {
         self.entry(account)?
             .set_password(secret)
@@ -136,6 +156,21 @@ impl SecretStore for NativeStore {
     }
 }
 
+/// Whether the Secret Service holds the account's item in a collection that
+/// is locked. Searching asks the person nothing, and sends no secret.
+#[cfg(target_os = "linux")]
+fn linux_locked(account: &str) -> Result<bool, SecretError> {
+    use secret_service::EncryptionType;
+    use secret_service::blocking::SecretService;
+    let service =
+        SecretService::connect(EncryptionType::Plain).map_err(|_| SecretError::Unavailable)?;
+    // The attributes the keyring crate gives an entry of ours.
+    let found = service
+        .search_items(HashMap::from([("service", SERVICE), ("username", account)]))
+        .map_err(|_| SecretError::Unavailable)?;
+    Ok(!found.locked.is_empty())
+}
+
 type Job = Box<dyn FnOnce(&mut dyn SecretStore) + Send>;
 
 /// A handle to the keyring thread.
@@ -212,6 +247,12 @@ impl Keyring {
         self.run(move |store| store.read(&account))
     }
 
+    /// [`SecretStore::read_quietly`]: a read nobody at the screen asked for.
+    pub fn read_quietly(&self, account: &str) -> Pending<Option<String>> {
+        let account = account.to_owned();
+        self.run(move |store| store.read_quietly(&account))
+    }
+
     pub fn write(&self, account: &str, secret: SecretString) -> Pending<()> {
         let account = account.to_owned();
         self.run(move |store| store.write(&account, &secret.0))
````

- [ ] **Step 4: Run the tests**

Run: `cargo test --locked --lib secrets::`

Expected: PASS. The one ignored is the native keyring's round trip, which runs by hand on a desktop.

```text
test result: ok. 5 passed; 0 failed; 1 ignored; 0 measured; 2412 filtered out
```

- [ ] **Step 5: Commit**

```bash
cargo fmt --all
cargo clippy --locked --workspace --all-targets -- -D warnings
git add src/secrets.rs Cargo.toml Cargo.lock
git commit -m "Read a saved password without a prompt to unlock the keyring"
```

---

### Task 4: The backend's gateway

`backend/agents.rs`, a module of the backend so that it shares the backend's cancels:

- **`agentable`** turns the saved connections into what agents may use, with the names they call them by.
- **`AppGateway`** keeps the policy (the connections and the trusted SSH host keys) and a turn for each connection, which holds its session while it is open. `open` waits for the turn, looks at the policy again, opens the session when there is none or it is not what the connection is now, and looks again once it is open.
- **`connect`** gathers the secrets by the quiet read, refusing `Ask` and a keyring with nothing saved, and opens with `Access::ReadOnly`, for 20 seconds at most. Why it failed goes to the app's log, never to the agent.
- **`AppSession`** holds the turn for one call. `ended` awaits the work and, once the call is stopped, sends the session's cancel again and again, awaiting each before the session is let go. A session the database lost is closed when the call lets it go.
- `close_idle` and `close_all` are for the worker (Task 5).

`model::first_schema` is the app's rule for a connection's first schema, moved out of `App::apply` so that the gateway has the same. CI's PostgreSQL and MySQL jobs run the gateway's tests: the app builds with its GUI, so they install what the test job installs for it.

**Files:**
- Create: `src/backend/agents.rs`
- Modify: `src/backend.rs` (the module), `src/model.rs`, `src/app.rs` (`first_schema`), `Cargo.toml`, `Cargo.lock`, `.github/workflows/ci.yml`

- [ ] **Step 1: Write the failing tests**

The module has its tests and nothing else yet. They are of a gateway over SQLite files of the Bookshop, of one under the real server through lines of the protocol, and of PostgreSQL and MySQL, with their passwords in the keyring.

````diff
diff --git a/.github/workflows/ci.yml b/.github/workflows/ci.yml
index 14a51f9..fdd40f5 100644
--- a/.github/workflows/ci.yml
+++ b/.github/workflows/ci.yml
@@ -91,6 +91,15 @@ jobs:
       - run: cargo test --locked -p tabletist-db --lib --test postgres --test insert_audit --test lookup --test delete_rows --test agent
         env:
           TABLETIST_TEST_PG_URL: postgres://tabletist:tabletist@localhost:55432/tabletist
+      # The app's gateway for the MCP server, which opens its own sessions.
+      # The app builds with its GUI, which wants these.
+      - name: GUI build dependencies
+        run: |
+          sudo apt-get update
+          sudo apt-get install -y libxkbcommon-dev libwayland-dev libgl1-mesa-dev
+      - run: cargo test --locked --lib backend::agents
+        env:
+          TABLETIST_TEST_PG_URL: postgres://tabletist:tabletist@localhost:55432/tabletist
 
   mysql:
     name: mysql integration
@@ -105,6 +114,15 @@ jobs:
       - run: cargo test --locked -p tabletist-db --lib --test mysql --test insert_audit --test lookup --test delete_rows --test agent
         env:
           TABLETIST_TEST_MYSQL_URL: mysql://tabletist:tabletist@localhost:53306/tabletist
+      # The app's gateway for the MCP server, which opens its own sessions.
+      # The app builds with its GUI, which wants these.
+      - name: GUI build dependencies
+        run: |
+          sudo apt-get update
+          sudo apt-get install -y libxkbcommon-dev libwayland-dev libgl1-mesa-dev
+      - run: cargo test --locked --lib backend::agents
+        env:
+          TABLETIST_TEST_MYSQL_URL: mysql://tabletist:tabletist@localhost:53306/tabletist
 
   ssh:
     name: ssh integration
diff --git a/Cargo.lock b/Cargo.lock
index aecc327..c08036b 100644
--- a/Cargo.lock
+++ b/Cargo.lock
@@ -5700,8 +5700,10 @@ dependencies = [
  "skrifa",
  "tabletist-appkit",
  "tabletist-db",
+ "tabletist-mcp",
  "tempfile",
  "tokio",
+ "tokio-util",
  "toml",
  "uuid",
  "windows-native-keyring-store",
diff --git a/Cargo.toml b/Cargo.toml
index 2ebfe2c..929e716 100644
--- a/Cargo.toml
+++ b/Cargo.toml
@@ -70,6 +70,12 @@ fastframe-i18n = { git = "https://github.com/crmne/fastframe", tag = "v0.4.1" }
 fastframe-log = { git = "https://github.com/crmne/fastframe", tag = "v0.4.1" }
 # The database layer (no UI dependencies).
 tabletist-db = { path = "crates/tabletist-db" }
+# The MCP server agents reach the saved connections through. The backend
+# runs it, and is its gateway to the databases (src/backend/agents.rs).
+tabletist-mcp = { path = "crates/tabletist-mcp" }
+# `CancellationToken`: how the server tells its gateway that a call is
+# over. In the tree through tabletist-mcp, which hands one to every call.
+tokio-util = "0.7"
 # The backend runtime: one OS thread running tokio, where every database
 # call happens (src/backend.rs).
 tokio = { version = "1", features = ["rt-multi-thread", "sync", "macros", "time"] }
diff --git a/src/backend.rs b/src/backend.rs
index 93e44b6..7ff6b80 100644
--- a/src/backend.rs
+++ b/src/backend.rs
@@ -18,6 +18,10 @@ use tokio::sync::mpsc as tokio_mpsc;
 use crate::connections::SavedConnections;
 use crate::secrets::{Keyring, SecretString};
 
+mod agents;
+
+pub use agents::{Agentable, AppGateway, agentable};
+
 /// Wakes the UI from any thread. The default does nothing (tests).
 #[derive(Clone)]
 pub struct Waker(Arc<dyn Fn() + Send + Sync>);
diff --git a/src/backend/agents.rs b/src/backend/agents.rs
new file mode 100644
index 0000000..aaa3a29
--- /dev/null
+++ b/src/backend/agents.rs
@@ -0,0 +1,626 @@
+//! The MCP server in the app: the connections agents may use, and the
+//! gateway through which the server reaches them.
+//!
+//! The gateway keeps sessions of its own, one for each connection an agent
+//! has used: a tab's session runs its own commands one at a time, and an
+//! agent's ten-second query must not stall the grid behind it. Every one
+//! is read-only at the database, whatever the connection's own box says.
+
+#[cfg(test)]
+mod tests {
+    use std::path::{Path, PathBuf};
+
+    use tabletist_db::Dialect;
+    use tabletist_db::agent::check_read;
+    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
+
+    use super::*;
+    use crate::connections::Agents as AgentSettings;
+    use crate::env::Environment;
+    use crate::secrets::SecretString;
+
+    fn saved(id: &str, name: &str, environment: Environment, spec: ConnectSpec) -> SavedConnection {
+        SavedConnection {
+            id: ConnectionId(id.into()),
+            name: name.into(),
+            environment,
+            read_only: Some(false),
+            password: PasswordMode::None,
+            ssh_secret: PasswordMode::None,
+            spec,
+            agents: AgentSettings {
+                access: AgentAccess::Read,
+                ..AgentSettings::default()
+            },
+        }
+    }
+
+    /// A new SQLite file of the Bookshop.
+    fn bookshop(dir: &Path, file: &str) -> PathBuf {
+        let path = dir.join(file);
+        tabletist_db::fixtures::write_sqlite_bookshop(&path).unwrap();
+        path
+    }
+
+    /// A gateway over one connection agents may read.
+    fn gateway(connection: SavedConnection) -> AppGateway {
+        let gateway = AppGateway::new(Keyring::memory());
+        gateway.set_policy(agentable(&[connection]), HostKeys::default());
+        gateway
+    }
+
+    async fn read(session: &AppSession, dialect: Dialect, sql: &str) -> Result<Answer, Failure> {
+        let read = check_read(dialect, sql, &[]).unwrap();
+        session
+            .read(
+                &read,
+                Limits {
+                    rows: 500,
+                    timeout: Duration::from_secs(10),
+                },
+            )
+            .await
+    }
+
+    fn the_int(answer: &Answer) -> i64 {
+        match answer.rows[0][0] {
+            tabletist_db::Value::Int(value) => value,
+            ref other => panic!("{other:?}"),
+        }
+    }
+
+    /// A statement that takes minutes: six copies of the Bookshop's forty
+    /// order items, crossed.
+    fn slow() -> String {
+        let tables: Vec<String> = (1..=6).map(|n| format!("order_items t{n}")).collect();
+        let sum: Vec<String> = (1..=6).map(|n| format!("t{n}.quantity")).collect();
+        format!(
+            "SELECT count(*) FROM {} WHERE {} < 0",
+            tables.join(", "),
+            sum.join(" + ")
+        )
+    }
+
+    #[test]
+    fn an_agent_calls_a_connection_by_its_name_and_its_environment() {
+        let file = || ConnectSpec::sqlite("/tmp/bookshop.db");
+        let connections = vec![
+            saved("1f0c2a9e-0001", "Bookshop", Environment::Dev, file()),
+            saved("7b3d55c1-0002", "Bookshop", Environment::Production, file()),
+            saved("c9e81f40-0003", "Scratch", Environment::None, file()),
+        ];
+        let named: Vec<String> = agentable(&connections)
+            .into_iter()
+            .map(|agentable| agentable.name)
+            .collect();
+        assert_eq!(
+            named,
+            ["Bookshop · dev", "Bookshop · production", "Scratch"]
+        );
+        let mut few = connections[2].clone();
+        few.agents.rows = 100;
+        few.agents.timeout_secs = 30;
+        assert_eq!(
+            agentable(&[few])[0].limits,
+            Limits {
+                rows: 100,
+                timeout: Duration::from_secs(30),
+            }
+        );
+    }
+
+    #[test]
+    fn a_name_two_connections_share_never_comes_to_mean_another() {
+        let file = || ConnectSpec::sqlite("/tmp/bookshop.db");
+        let a = saved("aaaa1111-x", "Shop", Environment::Dev, file());
+        let b = saved("bbbb2222-y", "Shop", Environment::Dev, file());
+        let c = saved("cccc3333-z", "Shop", Environment::Dev, file());
+        let names = |saved: &[SavedConnection]| -> Vec<(String, String)> {
+            agentable(saved)
+                .into_iter()
+                .map(|agentable| (agentable.id.0, agentable.name))
+                .collect()
+        };
+        let pair = |id: &str, name: &str| (id.to_owned(), name.to_owned());
+        assert_eq!(
+            names(&[a.clone(), b.clone(), c.clone()]),
+            [
+                pair("aaaa1111-x", "Shop · dev (aaaa1111)"),
+                pair("bbbb2222-y", "Shop · dev (bbbb2222)"),
+                pair("cccc3333-z", "Shop · dev (cccc3333)"),
+            ]
+        );
+        // One turned Off: the others keep their names.
+        let mut off = a.clone();
+        off.agents.access = AgentAccess::Off;
+        assert_eq!(
+            names(&[off, b.clone(), c.clone()]),
+            [
+                pair("bbbb2222-y", "Shop · dev (bbbb2222)"),
+                pair("cccc3333-z", "Shop · dev (cccc3333)"),
+            ]
+        );
+        // One removed: what was shared is no more, and nobody is called
+        // as a removed one was.
+        assert_eq!(
+            names(&[b.clone(), c.clone()]),
+            [
+                pair("bbbb2222-y", "Shop · dev (bbbb2222)"),
+                pair("cccc3333-z", "Shop · dev (cccc3333)"),
+            ]
+        );
+        assert_eq!(
+            names(std::slice::from_ref(&c)),
+            [pair("cccc3333-z", "Shop · dev")]
+        );
+        // A name that would be another's own takes the whole id.
+        let own = saved(
+            "dddd4444-w",
+            "Shop · dev (bbbb2222)",
+            Environment::None,
+            file(),
+        );
+        assert_eq!(
+            names(&[b, c, own]),
+            [
+                pair("bbbb2222-y", "Shop · dev (bbbb2222-y)"),
+                pair("cccc3333-z", "Shop · dev (cccc3333)"),
+                pair("dddd4444-w", "Shop · dev (bbbb2222)"),
+            ]
+        );
+    }
+
+    #[tokio::test]
+    async fn a_call_opens_the_connection_read_only_and_the_next_call_has_it_too() {
+        let dir = tempfile::tempdir().unwrap();
+        let spec = ConnectSpec::sqlite(bookshop(dir.path(), "bookshop.db"));
+        // The connection's own box lets it write: an agent's session never.
+        let gateway = gateway(saved("shop", "Bookshop", Environment::Dev, spec));
+        assert_eq!(gateway.connections()[0].name, "Bookshop · dev");
+        let stop = CancellationToken::new();
+        let first = gateway.open("shop", &stop).await.unwrap();
+        assert_eq!(first.connection().unwrap().access(), Access::ReadOnly);
+        let answer = read(&first, Dialect::Sqlite, "SELECT count(*) FROM books").await;
+        assert_eq!(the_int(&answer.unwrap()), 12);
+        let schemas = first.schemas().await.unwrap();
+        assert_eq!(schemas.home.as_deref(), Some("main"));
+        let opened = std::ptr::from_ref(first.connection().unwrap());
+        drop(first);
+        let again = gateway.open("shop", &stop).await.unwrap();
+        assert_eq!(std::ptr::from_ref(again.connection().unwrap()), opened);
+    }
+
+    #[tokio::test]
+    async fn a_connection_that_needs_a_person_is_not_opened() {
+        let dir = tempfile::tempdir().unwrap();
+        let server = ConnectSpec {
+            driver: tabletist_db::Driver::Postgres,
+            host: "db.example.invalid".into(),
+            port: 5432,
+            user: "app".into(),
+            database: "bookshop".into(),
+            ..ConnectSpec::default()
+        };
+        let mut asks = saved("asks", "Asks", Environment::Dev, server.clone());
+        asks.password = PasswordMode::Ask;
+        let mut kept = saved("kept", "Kept", Environment::Dev, server);
+        kept.password = PasswordMode::Keyring;
+        let gone = saved(
+            "gone",
+            "Gone",
+            Environment::Dev,
+            ConnectSpec::sqlite(dir.path().join("no-such.db")),
+        );
+        let gateway = AppGateway::new(Keyring::memory());
+        gateway.set_policy(agentable(&[asks, kept, gone]), HostKeys::default());
+        let stop = CancellationToken::new();
+        // Asked for each time; nothing in the keyring; no such file.
+        for id in ["asks", "kept", "gone"] {
+            let opened = gateway.open(id, &stop).await;
+            assert!(matches!(opened, Err(Failure::Unavailable)), "{id}");
+        }
+        // And one that is not there at all.
+        assert!(matches!(
+            gateway.open("payroll", &stop).await,
+            Err(Failure::Unavailable)
+        ));
+    }
+
+    /// A keyring that is locked: a read would ask the person to unlock it,
+    /// and says that it was made.
+    struct Locked(Arc<std::sync::atomic::AtomicBool>);
+
+    impl crate::secrets::SecretStore for Locked {
+        fn read(&mut self, _account: &str) -> Result<Option<String>, crate::secrets::SecretError> {
+            self.0.store(true, std::sync::atomic::Ordering::SeqCst);
+            Err(crate::secrets::SecretError::Locked)
+        }
+
+        fn read_quietly(
+            &mut self,
+            _account: &str,
+        ) -> Result<Option<String>, crate::secrets::SecretError> {
+            Err(crate::secrets::SecretError::Locked)
+        }
+
+        fn write(
+            &mut self,
+            _account: &str,
+            _secret: &str,
+        ) -> Result<(), crate::secrets::SecretError> {
+            Err(crate::secrets::SecretError::Locked)
+        }
+
+        fn delete(&mut self, _account: &str) -> Result<(), crate::secrets::SecretError> {
+            Err(crate::secrets::SecretError::Locked)
+        }
+    }
+
+    #[tokio::test]
+    async fn a_locked_keyring_is_not_asked_to_be_unlocked() {
+        let mut kept = saved(
+            "kept",
+            "Kept",
+            Environment::Dev,
+            ConnectSpec {
+                driver: tabletist_db::Driver::Postgres,
+                host: "db.example.invalid".into(),
+                port: 5432,
+                ..ConnectSpec::default()
+            },
+        );
+        kept.password = PasswordMode::Keyring;
+        let asked = Arc::new(std::sync::atomic::AtomicBool::new(false));
+        let gateway = AppGateway::new(Keyring::start(Locked(Arc::clone(&asked))));
+        gateway.set_policy(agentable(&[kept]), HostKeys::default());
+        let opened = gateway.open("kept", &CancellationToken::new()).await;
+        assert!(matches!(opened, Err(Failure::Unavailable)));
+        assert!(
+            !asked.load(std::sync::atomic::Ordering::SeqCst),
+            "a read that would prompt"
+        );
+    }
+
+    #[tokio::test]
+    async fn a_call_that_waited_while_its_connection_was_turned_off_runs_nothing() {
+        let dir = tempfile::tempdir().unwrap();
+        let spec = ConnectSpec::sqlite(bookshop(dir.path(), "bookshop.db"));
+        let connection = saved("shop", "Bookshop", Environment::Dev, spec);
+        let gateway = gateway(connection.clone());
+        let stop = CancellationToken::new();
+        let holding = gateway.open("shop", &stop).await.unwrap();
+        let (waited, ()) = tokio::join!(gateway.open("shop", &stop), async {
+            tokio::time::sleep(Duration::from_millis(100)).await;
+            let mut off = connection;
+            off.agents.access = AgentAccess::Off;
+            gateway.set_policy(agentable(&[off]), HostKeys::default());
+            drop(holding);
+        });
+        assert!(matches!(waited, Err(Failure::Unavailable)));
+    }
+
+    #[tokio::test]
+    async fn a_second_call_waits_its_turn_and_then_is_busy() {
+        let dir = tempfile::tempdir().unwrap();
+        let spec = ConnectSpec::sqlite(bookshop(dir.path(), "bookshop.db"));
+        let mut connection = saved("shop", "Bookshop", Environment::Dev, spec);
+        connection.agents.timeout_secs = 1;
+        let gateway = gateway(connection);
+        let stop = CancellationToken::new();
+        let first = gateway.open("shop", &stop).await.unwrap();
+        let started = Instant::now();
+        let second = gateway.open("shop", &stop).await;
+        assert!(matches!(second, Err(Failure::Busy)));
+        assert!(started.elapsed() >= Duration::from_secs(1));
+        drop(first);
+        assert!(gateway.open("shop", &stop).await.is_ok());
+        // A call that is stopped while it waits stops waiting.
+        let holding = gateway.open("shop", &stop).await.unwrap();
+        let stopped = CancellationToken::new();
+        stopped.cancel();
+        let waiting = gateway.open("shop", &stopped).await;
+        assert!(matches!(waiting, Err(Failure::Database(Error::Cancelled))));
+        drop(holding);
+    }
+
+    #[tokio::test]
+    async fn a_connection_taken_away_or_changed_is_not_used_as_it_was() {
+        let dir = tempfile::tempdir().unwrap();
+        let first = bookshop(dir.path(), "first.db");
+        // Another database altogether: the one the app's demo opens.
+        let second = dir.path().join("second.db");
+        tabletist_db::fixtures::write_sqlite_demo(&second).unwrap();
+        let shop = |path: &Path| {
+            saved(
+                "shop",
+                "Bookshop",
+                Environment::Dev,
+                ConnectSpec::sqlite(path),
+            )
+        };
+        let gateway = gateway(shop(&first));
+        let stop = CancellationToken::new();
+        let session = gateway.open("shop", &stop).await.unwrap();
+        let count = read(&session, Dialect::Sqlite, "SELECT count(*) FROM books").await;
+        assert_eq!(the_int(&count.unwrap()), 12);
+        drop(session);
+        // Pointed at another file: opened again, on that file, which has
+        // no books.
+        gateway.set_policy(agentable(&[shop(&second)]), HostKeys::default());
+        let session = gateway.open("shop", &stop).await.unwrap();
+        let count = read(&session, Dialect::Sqlite, "SELECT count(*) FROM books").await;
+        assert!(
+            matches!(count, Err(Failure::Database(Error::Query { .. }))),
+            "{count:?}"
+        );
+        drop(session);
+        // Turned Off: gone.
+        let mut off = shop(&second);
+        off.agents.access = AgentAccess::Off;
+        gateway.set_policy(agentable(&[off]), HostKeys::default());
+        assert_eq!(gateway.connections(), []);
+        assert!(matches!(
+            gateway.open("shop", &stop).await,
+            Err(Failure::Unavailable)
+        ));
+        assert!(lock(&gateway.inner.turns).is_empty());
+    }
+
+    #[tokio::test]
+    async fn a_stop_ends_a_statement_at_the_database_and_the_session_goes_on() {
+        let dir = tempfile::tempdir().unwrap();
+        let spec = ConnectSpec::sqlite(bookshop(dir.path(), "bookshop.db"));
+        let gateway = gateway(saved("shop", "Bookshop", Environment::Dev, spec));
+        let stop = CancellationToken::new();
+        let session = gateway.open("shop", &stop).await.unwrap();
+        let slow = slow();
+        let started = Instant::now();
+        let (ended, ()) = tokio::join!(read(&session, Dialect::Sqlite, &slow), async {
+            tokio::time::sleep(Duration::from_millis(300)).await;
+            stop.cancel();
+        });
+        assert!(
+            matches!(ended, Err(Failure::Database(Error::Cancelled))),
+            "{ended:?}"
+        );
+        assert!(started.elapsed() < Duration::from_secs(5));
+        drop(session);
+        let next = gateway
+            .open("shop", &CancellationToken::new())
+            .await
+            .unwrap();
+        let count = read(&next, Dialect::Sqlite, "SELECT count(*) FROM books").await;
+        assert_eq!(the_int(&count.unwrap()), 12);
+    }
+
+    #[tokio::test]
+    async fn a_connection_left_idle_is_closed_and_one_in_use_is_not() {
+        let dir = tempfile::tempdir().unwrap();
+        let spec = ConnectSpec::sqlite(bookshop(dir.path(), "bookshop.db"));
+        let gateway = gateway(saved("shop", "Bookshop", Environment::Dev, spec));
+        let stop = CancellationToken::new();
+        let session = gateway.open("shop", &stop).await.unwrap();
+        let turn = Arc::clone(&lock(&gateway.inner.turns)[&ConnectionId("shop".into())]);
+        gateway.close_idle(Duration::ZERO);
+        assert!(session.connection().is_ok(), "in use");
+        drop(session);
+        assert!(turn.try_lock().unwrap().is_some());
+        gateway.close_idle(Duration::from_secs(60));
+        assert!(turn.try_lock().unwrap().is_some(), "not idle so long yet");
+        gateway.close_idle(Duration::ZERO);
+        assert!(turn.try_lock().unwrap().is_none());
+        // And opened again for the next call.
+        assert!(gateway.open("shop", &stop).await.is_ok());
+    }
+
+    /// One line of the protocol, sent, and the answer to it, when it has an
+    /// id.
+    async fn said<R: tokio::io::AsyncBufRead + Unpin, W: tokio::io::AsyncWrite + Unpin>(
+        lines: &mut tokio::io::Lines<R>,
+        write: &mut W,
+        message: serde_json::Value,
+    ) -> Option<serde_json::Value> {
+        write
+            .write_all(format!("{message}\n").as_bytes())
+            .await
+            .unwrap();
+        message.get("id")?;
+        let line = lines.next_line().await.unwrap().unwrap();
+        Some(serde_json::from_str(&line).unwrap())
+    }
+
+    #[tokio::test]
+    async fn the_server_reads_the_bookshop_through_the_apps_gateway() {
+        use serde_json::json;
+        let dir = tempfile::tempdir().unwrap();
+        let spec = ConnectSpec::sqlite(bookshop(dir.path(), "bookshop.db"));
+        let gateway = gateway(saved("shop", "Bookshop", Environment::Dev, spec));
+        let log = tabletist_mcp::Log::open(&dir.path().join("agents.sqlite3"))
+            .unwrap()
+            .log;
+        let server = tabletist_mcp::Server::new(gateway, log.clone(), "test");
+        let (ours, theirs) = tokio::io::duplex(1024 * 1024);
+        let served = tokio::spawn({
+            let server = server.clone();
+            async move {
+                let origin = tabletist_mcp::Origin {
+                    transport: tabletist_mcp::Transport::Stdio,
+                    project: None,
+                };
+                server.serve(theirs, origin).await;
+            }
+        });
+        let (read, mut write) = tokio::io::split(ours);
+        let mut lines = BufReader::new(read).lines();
+        let started = said(
+            &mut lines,
+            &mut write,
+            json!({
+                "jsonrpc": "2.0",
+                "id": 0,
+                "method": "initialize",
+                "params": {
+                    "protocolVersion": "2025-11-25",
+                    "capabilities": {},
+                    "clientInfo": { "name": "claude-code", "version": "1" },
+                },
+            }),
+        )
+        .await;
+        assert!(started.unwrap()["result"].is_object());
+        said(
+            &mut lines,
+            &mut write,
+            json!({ "jsonrpc": "2.0", "method": "notifications/initialized" }),
+        )
+        .await;
+        let call = |id: u64, name: &str, arguments: serde_json::Value| {
+            json!({
+                "jsonrpc": "2.0",
+                "id": id,
+                "method": "tools/call",
+                "params": { "name": name, "arguments": arguments },
+            })
+        };
+        let listed = said(
+            &mut lines,
+            &mut write,
+            call(1, "list_connections", json!({})),
+        )
+        .await;
+        let text = listed.unwrap()["result"]["content"][0]["text"].clone();
+        assert!(
+            text.as_str()
+                .unwrap()
+                .contains("\"name\":\"Bookshop · dev\""),
+            "{text}"
+        );
+        let arguments =
+            json!({ "connection": "Bookshop · dev", "sql": "SELECT count(*) FROM books" });
+        let ran = said(&mut lines, &mut write, call(2, "run_query", arguments)).await;
+        let text = ran.unwrap()["result"]["content"][0]["text"].clone();
+        assert!(text.as_str().unwrap().contains("\"rows\":[[12]]"), "{text}");
+        write.shutdown().await.unwrap();
+        served.await.unwrap();
+        let entries = log.latest(10).await.unwrap();
+        assert_eq!(entries.len(), 2);
+        assert_eq!(entries[0].connection_id.as_deref(), Some("shop"));
+    }
+
+    /// A server's connection from a test URL, its password in the keyring.
+    async fn on_a_server(url_variable: &str) -> Option<(AppGateway, Dialect)> {
+        let Ok(url) = std::env::var(url_variable) else {
+            eprintln!("skipped: {url_variable} is not set");
+            return None;
+        };
+        let (mut spec, secrets) = ConnectSpec::from_url(&url).unwrap();
+        spec.tls = tabletist_db::TlsMode::Disable;
+        let dialect = spec.driver.dialect();
+        let mut connection = saved("server", "Server", Environment::Dev, spec);
+        connection.password = PasswordMode::Keyring;
+        let keyring = Keyring::memory();
+        let password = secrets.password.unwrap_or_default();
+        keyring
+            .write(&password_account(&connection.id), SecretString(password))
+            .wait()
+            .await
+            .unwrap();
+        let gateway = AppGateway::new(keyring);
+        gateway.set_policy(agentable(&[connection]), HostKeys::default());
+        Some((gateway, dialect))
+    }
+
+    async fn the_password_is_the_keyrings_and_a_stop_reaches_the_server(
+        url_variable: &str,
+        slow: &str,
+    ) {
+        let Some((gateway, dialect)) = on_a_server(url_variable).await else {
+            return;
+        };
+        let stop = CancellationToken::new();
+        let session = gateway.open("server", &stop).await.unwrap();
+        let one = read(&session, dialect, "SELECT 1").await.unwrap();
+        assert_eq!(one.rows.len(), 1);
+        let started = Instant::now();
+        let (ended, ()) = tokio::join!(read(&session, dialect, slow), async {
+            tokio::time::sleep(Duration::from_millis(500)).await;
+            stop.cancel();
+        });
+        assert!(
+            matches!(ended, Err(Failure::Database(Error::Cancelled))),
+            "{ended:?}"
+        );
+        assert!(
+            started.elapsed() < Duration::from_secs(8),
+            "{:?}",
+            started.elapsed()
+        );
+        drop(session);
+        let next = gateway
+            .open("server", &CancellationToken::new())
+            .await
+            .unwrap();
+        assert!(read(&next, dialect, "SELECT 1").await.is_ok());
+    }
+
+    #[tokio::test]
+    async fn on_postgres_a_session_that_was_lost_is_opened_again() {
+        let Some((gateway, dialect)) = on_a_server("TABLETIST_TEST_PG_URL").await else {
+            return;
+        };
+        let stop = CancellationToken::new();
+        let session = gateway.open("server", &stop).await.unwrap();
+        let pid = the_int(
+            &read(&session, dialect, "SELECT pg_backend_pid()")
+                .await
+                .unwrap(),
+        );
+        drop(session);
+        // Ended from another session, as a server restart or an admin
+        // would.
+        let url = std::env::var("TABLETIST_TEST_PG_URL").unwrap();
+        let (mut spec, secrets) = ConnectSpec::from_url(&url).unwrap();
+        spec.tls = tabletist_db::TlsMode::Disable;
+        let admin = Connection::connect(&spec, &secrets).await.unwrap();
+        let text = format!("SELECT pg_terminate_backend({pid})");
+        let statements = tabletist_db::sql::statements(dialect, &text);
+        admin
+            .run_script(
+                &statements,
+                1,
+                tabletist_db::ScriptMode::ReadOnly,
+                &StopFlag::new(),
+            )
+            .await
+            .unwrap();
+        let session = gateway.open("server", &stop).await.unwrap();
+        let lost = read(&session, dialect, "SELECT 1").await;
+        assert!(
+            matches!(lost, Err(Failure::Database(ref error)) if error.is_connection_lost()),
+            "{lost:?}"
+        );
+        drop(session);
+        // The next call has a session again.
+        let session = gateway.open("server", &stop).await.unwrap();
+        assert!(read(&session, dialect, "SELECT 1").await.is_ok());
+    }
+
+    #[tokio::test]
+    async fn on_postgres_the_password_is_the_keyrings_and_a_stop_reaches_the_server() {
+        the_password_is_the_keyrings_and_a_stop_reaches_the_server(
+            "TABLETIST_TEST_PG_URL",
+            "SELECT count(*) FROM generate_series(1, 2000000000)",
+        )
+        .await;
+    }
+
+    #[tokio::test]
+    async fn on_mysql_the_password_is_the_keyrings_and_a_stop_reaches_the_server() {
+        the_password_is_the_keyrings_and_a_stop_reaches_the_server(
+            "TABLETIST_TEST_MYSQL_URL",
+            "SELECT count(*) FROM information_schema.columns a, information_schema.columns b, \
+             information_schema.columns c",
+        )
+        .await;
+    }
+}
````

- [ ] **Step 2: Run them to see that they do not build**

Run: `cargo test --locked --lib backend::agents`

Expected: it does not build. The backend declares the module and exports what it does not hold yet:

```text
error[E0432]: unresolved imports `agents::Agentable`, `agents::AppGateway`, `agents::agentable`
error[E0425]: cannot find type `AppGateway` in this scope
error[E0425]: cannot find type `AppSession` in this scope
```

- [ ] **Step 3: The gateway**

````diff
diff --git a/src/app.rs b/src/app.rs
index 8f0cb6e..70b1ac2 100644
--- a/src/app.rs
+++ b/src/app.rs
@@ -2530,22 +2530,12 @@ impl App {
                 }
                 workspace.catalog_changed();
                 let database = workspace.spec.database.clone();
-                let default = workspace.tree.schemas.value.as_ref().and_then(|schemas| {
-                    schemas
-                        .iter()
-                        .find(|schema| !database.is_empty() && **schema == database)
-                        .or_else(|| {
-                            schemas
-                                .iter()
-                                .find(|schema| *schema == "public" || *schema == "main")
-                        })
-                        .or(if schemas.len() == 1 {
-                            schemas.first()
-                        } else {
-                            None
-                        })
-                        .cloned()
-                });
+                let default = workspace
+                    .tree
+                    .schemas
+                    .value
+                    .as_ref()
+                    .and_then(|schemas| crate::model::first_schema(schemas, &database));
                 if let Some(schema) = default {
                     self.expand_schema(tab, &schema);
                 }
diff --git a/src/backend/agents.rs b/src/backend/agents.rs
index aaa3a29..dea1732 100644
--- a/src/backend/agents.rs
+++ b/src/backend/agents.rs
@@ -6,6 +6,436 @@
 //! agent's ten-second query must not stall the grid behind it. Every one
 //! is read-only at the database, whatever the connection's own box says.
 
+use std::collections::HashMap;
+use std::sync::{Arc, Mutex};
+use std::time::{Duration, Instant};
+
+use tabletist_db::agent::{Answer, Limits, Read};
+use tabletist_db::{
+    Access, CancelHandle, ConnectSpec, Connection, Error, HostKeys, ObjectInfo, ObjectRef, Secrets,
+    StopFlag, Structure,
+};
+use tabletist_mcp::{Failure, Gateway, Listed, Schemas, Session};
+use tokio::sync::OwnedMutexGuard;
+use tokio_util::sync::CancellationToken;
+
+use super::{CANCEL_AGAIN, lock, longer, send_cancel};
+use crate::connections::{AgentAccess, ConnectionId, PasswordMode, SavedConnection};
+use crate::env::Platform;
+use crate::model::SecretKind;
+use crate::secrets::{Keyring, password_account, ssh_account};
+
+/// A connection agents may use, as the server is given it: what opens it,
+/// and what an agent calls it. Its password is not here: the gateway reads
+/// it from the keyring when it opens the connection, as a tab does.
+#[derive(Debug, Clone, PartialEq)]
+pub struct Agentable {
+    pub id: ConnectionId,
+    /// What an agent calls it: its name and its environment, with a number
+    /// after it where another connection would be called the same.
+    pub name: String,
+    pub spec: ConnectSpec,
+    pub password: PasswordMode,
+    pub ssh_secret: PasswordMode,
+    pub limits: Limits,
+}
+
+/// The connections of `saved` that agents may use, in its order. One that
+/// is Off is left out: to an agent it does not exist.
+///
+/// An agent calls a connection by its name and its environment. Where two
+/// saved connections would be called the same, Off or not, each is called
+/// that with the start of its id after it, for good: a name an agent was
+/// given never comes to mean another database, when a connection is
+/// added, turned Off or removed. At worst it means none.
+pub fn agentable(saved: &[SavedConnection]) -> Vec<Agentable> {
+    let called = |saved: &SavedConnection| match saved.environment.named(Platform::Native) {
+        Some(environment) => format!("{} · {environment}", saved.name),
+        None => saved.name.clone(),
+    };
+    let mut times: HashMap<String, usize> = HashMap::new();
+    for saved in saved {
+        *times.entry(called(saved)).or_default() += 1;
+    }
+    let shared = |saved: &SavedConnection| times[&called(saved)] > 1;
+    let short: Vec<String> = saved
+        .iter()
+        .map(|saved| match shared(saved) {
+            true => format!("{} ({})", called(saved), short_id(&saved.id, 8)),
+            false => called(saved),
+        })
+        .collect();
+    // A short id can make a name another connection already has: those
+    // take their whole id.
+    let names: Vec<String> = saved
+        .iter()
+        .enumerate()
+        .map(|(index, saved)| {
+            let taken = short
+                .iter()
+                .enumerate()
+                .any(|(other, name)| other != index && *name == short[index]);
+            match taken && shared(saved) {
+                true => format!("{} ({})", called(saved), saved.id.0),
+                false => short[index].clone(),
+            }
+        })
+        .collect();
+    saved
+        .iter()
+        .zip(names)
+        .filter(|(saved, _)| match saved.agents.access {
+            AgentAccess::Off => false,
+            AgentAccess::Read => true,
+        })
+        .map(|(saved, name)| Agentable {
+            id: saved.id.clone(),
+            name,
+            spec: saved.spec.clone(),
+            password: saved.password,
+            ssh_secret: saved.ssh_secret,
+            limits: Limits {
+                rows: saved.agents.rows,
+                timeout: Duration::from_secs(u64::from(saved.agents.timeout_secs)),
+            },
+        })
+        .collect()
+}
+
+/// The first `length` letters and digits of an id.
+fn short_id(id: &ConnectionId, length: usize) -> String {
+    id.0.chars()
+        .filter(char::is_ascii_alphanumeric)
+        .take(length)
+        .collect()
+}
+
+/// How long opening a connection for an agent may take, an SSH tunnel's
+/// among it.
+const CONNECT_TIME: Duration = Duration::from_secs(20);
+
+/// The gateway of the app's MCP server: the saved connections an agent
+/// may use, and a session of its own for each. A clone is the same gateway.
+#[derive(Clone)]
+pub struct AppGateway {
+    inner: Arc<Inner>,
+}
+
+struct Inner {
+    keyring: Keyring,
+    policy: Mutex<Policy>,
+    /// A connection's turn, and its session while it is open.
+    turns: Mutex<HashMap<ConnectionId, Arc<tokio::sync::Mutex<Option<Open>>>>>,
+}
+
+#[derive(Default)]
+struct Policy {
+    connections: Vec<Agentable>,
+    host_keys: HostKeys,
+}
+
+/// A connection's session, open.
+struct Open {
+    connection: Connection,
+    /// What it was opened as: a connection that is not that any more is
+    /// opened again.
+    opened_for: Agentable,
+    used: Instant,
+}
+
+impl AppGateway {
+    pub fn new(keyring: Keyring) -> Self {
+        Self {
+            inner: Arc::new(Inner {
+                keyring,
+                policy: Mutex::default(),
+                turns: Mutex::default(),
+            }),
+        }
+    }
+
+    /// The connections agents may use from now on. One that is not among
+    /// them any more is closed once its call, if it has one, is over.
+    pub fn set_policy(&self, connections: Vec<Agentable>, host_keys: HostKeys) {
+        let ids: Vec<ConnectionId> = connections
+            .iter()
+            .map(|agentable| agentable.id.clone())
+            .collect();
+        *lock(&self.inner.policy) = Policy {
+            connections,
+            host_keys,
+        };
+        lock(&self.inner.turns).retain(|id, _| ids.contains(id));
+    }
+
+    /// Closes every connection no call holds, and lets the others go when
+    /// their calls are over.
+    pub fn close_all(&self) {
+        lock(&self.inner.turns).clear();
+    }
+
+    /// Closes the connections that have had nothing to do for `idle`.
+    pub fn close_idle(&self, idle: Duration) {
+        let turns: Vec<_> = lock(&self.inner.turns).values().cloned().collect();
+        for turn in turns {
+            // One that is in use is not idle.
+            if let Ok(mut open) = turn.try_lock()
+                && open
+                    .as_ref()
+                    .is_some_and(|open| open.used.elapsed() >= idle)
+            {
+                *open = None;
+            }
+        }
+    }
+
+    fn find(&self, id: &str) -> Option<(Agentable, HostKeys)> {
+        let policy = lock(&self.inner.policy);
+        policy
+            .connections
+            .iter()
+            .find(|agentable| agentable.id.0 == id)
+            .map(|agentable| (agentable.clone(), policy.host_keys.clone()))
+    }
+}
+
+impl Gateway for AppGateway {
+    type Session = AppSession;
+
+    fn connections(&self) -> Vec<Listed> {
+        lock(&self.inner.policy)
+            .connections
+            .iter()
+            .map(|agentable| Listed {
+                id: agentable.id.0.clone(),
+                name: agentable.name.clone(),
+                dialect: agentable.spec.driver.dialect(),
+                limits: agentable.limits,
+            })
+            .collect()
+    }
+
+    async fn open(&self, id: &str, stop: &CancellationToken) -> Result<AppSession, Failure> {
+        let (agentable, _) = self.find(id).ok_or(Failure::Unavailable)?;
+        let turn = Arc::clone(
+            lock(&self.inner.turns)
+                .entry(agentable.id.clone())
+                .or_default(),
+        );
+        let mut open = tokio::select! {
+            biased;
+            () = stop.cancelled() => return Err(Error::Cancelled.into()),
+            turn = tokio::time::timeout(agentable.limits.timeout, turn.lock_owned()) => {
+                turn.map_err(|_| Failure::Busy)?
+            }
+        };
+        // The policy may have changed while the call waited: the
+        // connection as it is now, or none.
+        let (agentable, host_keys) = self.find(id).ok_or(Failure::Unavailable)?;
+        // Changed since it was opened: opened again as it is now.
+        if open
+            .as_ref()
+            .is_some_and(|open| open.opened_for != agentable)
+        {
+            *open = None;
+        }
+        if open.is_none() {
+            let connection = tokio::select! {
+                biased;
+                () = stop.cancelled() => return Err(Error::Cancelled.into()),
+                connection = connect(&self.inner.keyring, &agentable, &host_keys) => connection?,
+            };
+            // Turned Off or changed while it opened: not used as it was.
+            if self.find(id).is_none_or(|(now, _)| now != agentable) {
+                return Err(Failure::Unavailable);
+            }
+            *open = Some(Open {
+                connection,
+                opened_for: agentable.clone(),
+                used: Instant::now(),
+            });
+        }
+        let cancel = open
+            .as_ref()
+            .map(|open| open.connection.cancel_handle())
+            .ok_or(Failure::Unavailable)?;
+        Ok(AppSession {
+            open,
+            database: agentable.spec.database.clone(),
+            stop: stop.clone(),
+            cancel,
+            lost: std::sync::atomic::AtomicBool::new(false),
+        })
+    }
+}
+
+/// Opens a connection for agents: read-only, with its secrets from the
+/// keyring. A connection whose password is asked for each time cannot be
+/// opened without the person, and neither can one whose keyring is locked
+/// or whose SSH host is not trusted yet. Why it failed goes to the app's
+/// log, never to the agent: it can name a host or a user.
+async fn connect(
+    keyring: &Keyring,
+    agentable: &Agentable,
+    host_keys: &HostKeys,
+) -> Result<Connection, Failure> {
+    let unavailable = |why: &str| {
+        log::warn!("an agent's call could not open a connection: {why}");
+        Failure::Unavailable
+    };
+    let mut secrets = Secrets::default();
+    let needed = [
+        (
+            SecretKind::Database,
+            agentable.password,
+            password_account(&agentable.id),
+        ),
+        (
+            SecretKind::for_ssh(&agentable.spec).unwrap_or(SecretKind::Database),
+            match SecretKind::for_ssh(&agentable.spec) {
+                Some(_) => agentable.ssh_secret,
+                None => PasswordMode::None,
+            },
+            ssh_account(&agentable.id),
+        ),
+    ];
+    for (kind, mode, account) in needed {
+        match mode {
+            PasswordMode::None => {}
+            PasswordMode::Ask => return Err(unavailable("its password is asked for each time")),
+            // Never a prompt to unlock the keyring: one that came up because
+            // an agent asked could fish for the person's password.
+            PasswordMode::Keyring => match keyring.read_quietly(&account).wait().await {
+                Ok(Some(secret)) => *kind.slot(&mut secrets) = Some(secret),
+                Ok(None) => return Err(unavailable("the keyring has no password for it")),
+                Err(error) => return Err(unavailable(&error.to_string())),
+            },
+        }
+    }
+    let opening = Connection::connect_with(&agentable.spec, &secrets, host_keys, Access::ReadOnly);
+    match tokio::time::timeout(CONNECT_TIME, opening).await {
+        Ok(Ok(connection)) => Ok(connection),
+        Ok(Err(error)) => Err(unavailable(&error.to_string())),
+        Err(_) => Err(unavailable("it took too long to open")),
+    }
+}
+
+/// A connection's session, held by one call.
+pub struct AppSession {
+    open: OwnedMutexGuard<Option<Open>>,
+    database: String,
+    stop: CancellationToken,
+    cancel: CancelHandle,
+    /// Set when the session answered that it is lost or no longer what it
+    /// was opened as: it is closed when the call lets it go, and the next
+    /// call opens it again.
+    lost: std::sync::atomic::AtomicBool,
+}
+
+impl Drop for AppSession {
+    fn drop(&mut self) {
+        if self.lost.load(std::sync::atomic::Ordering::SeqCst) {
+            *self.open = None;
+        } else if let Some(open) = self.open.as_mut() {
+            open.used = Instant::now();
+        }
+    }
+}
+
+impl AppSession {
+    fn connection(&self) -> Result<&Connection, Failure> {
+        self.open
+            .as_ref()
+            .map(|open| &open.connection)
+            .ok_or(Failure::Unavailable)
+    }
+
+    /// Awaits `work` on the session to its end. Once the call is stopped
+    /// the database is told to stop it, as the backend stops a script: the
+    /// session's cancel is sent again and again, each time a little later,
+    /// until the work is over or its cleanup has begun. The work is never
+    /// dropped half way.
+    async fn ended<T>(
+        &self,
+        flag: Option<&StopFlag>,
+        work: impl Future<Output = Result<T, Error>>,
+    ) -> Result<T, Failure> {
+        let done = self.stopped(flag, work).await;
+        if let Err(error) = &done
+            && error.is_connection_lost()
+        {
+            self.lost.store(true, std::sync::atomic::Ordering::SeqCst);
+        }
+        done.map_err(Failure::from)
+    }
+
+    /// The work, and the cancels a stop sends, as [`Self::ended`] says.
+    async fn stopped<T>(
+        &self,
+        flag: Option<&StopFlag>,
+        work: impl Future<Output = Result<T, Error>>,
+    ) -> Result<T, Error> {
+        tokio::pin!(work);
+        tokio::select! {
+            done = &mut work => return done,
+            () = self.stop.cancelled() => {}
+        }
+        if let Some(flag) = flag {
+            flag.stop();
+        }
+        // Each cancel is a task of its own, awaited before the session is
+        // let go: one that is still on its way must not land on the
+        // statement of the next call.
+        let sent = Mutex::new(Vec::new());
+        let cancels = async {
+            let mut pause = CANCEL_AGAIN;
+            loop {
+                if flag.is_some_and(StopFlag::is_finishing) {
+                    std::future::pending::<()>().await;
+                }
+                lock(&sent).push(tokio::spawn(send_cancel(&self.cancel)));
+                tokio::time::sleep(pause).await;
+                pause = longer(pause);
+            }
+        };
+        let done = tokio::select! {
+            done = &mut work => done,
+            () = cancels => unreachable!("the cancels go on until the work is over"),
+        };
+        let sent = std::mem::take(&mut *lock(&sent));
+        for cancel in sent {
+            let _ = cancel.await;
+        }
+        done
+    }
+}
+
+impl Session for AppSession {
+    async fn schemas(&self) -> Result<Schemas, Failure> {
+        let connection = self.connection()?;
+        let names = self.ended(None, connection.list_schemas()).await?;
+        let home = crate::model::first_schema(&names, &self.database);
+        Ok(Schemas { names, home })
+    }
+
+    async fn objects(&self, schema: &str) -> Result<Vec<ObjectInfo>, Failure> {
+        let connection = self.connection()?;
+        self.ended(None, connection.list_objects(schema)).await
+    }
+
+    async fn describe(&self, object: &ObjectRef) -> Result<Structure, Failure> {
+        let connection = self.connection()?;
+        self.ended(None, connection.describe(object)).await
+    }
+
+    async fn read(&self, read: &Read, limits: Limits) -> Result<Answer, Failure> {
+        let connection = self.connection()?;
+        let flag = StopFlag::new();
+        self.ended(Some(&flag), connection.run_agent_read(read, limits, &flag))
+            .await
+    }
+}
+
 #[cfg(test)]
 mod tests {
     use std::path::{Path, PathBuf};
diff --git a/src/model.rs b/src/model.rs
index 477dcfe..f7286c7 100644
--- a/src/model.rs
+++ b/src/model.rs
@@ -911,6 +911,26 @@ pub enum TestState {
     },
 }
 
+/// The schema a connection opens on, of those it lists: the one named as
+/// its database (MySQL's databases are its schemas), else `public` or
+/// `main`, else the only one there is.
+pub fn first_schema(schemas: &[String], database: &str) -> Option<String> {
+    schemas
+        .iter()
+        .find(|schema| !database.is_empty() && **schema == database)
+        .or_else(|| {
+            schemas
+                .iter()
+                .find(|schema| *schema == "public" || *schema == "main")
+        })
+        .or(if schemas.len() == 1 {
+            schemas.first()
+        } else {
+            None
+        })
+        .cloned()
+}
+
 /// The connection dialog's fields while it is open.
 pub struct ConnectionForm {
     /// `Some` when editing a saved connection, `None` for a new one.
````

- [ ] **Step 4: Run the tests**

With both servers' variables set (see "Before you start"). Run: `cargo test --locked --lib backend::agents`, then `cargo test --locked --lib app::tests::connecting_loads_schemas_then_expands_the_default_schema`

Expected: PASS, in about two seconds: a call waits a second for its turn, and the server's own test of the gateway runs a session.

```text
test result: ok. 14 passed; 0 failed; 0 ignored; 0 measured; 2418 filtered out
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 2431 filtered out
```

Without the servers' variables the four tests of PostgreSQL and MySQL print "skipped" and pass, and the count is the same.

- [ ] **Step 5: Commit**

```bash
cargo fmt --all
cargo clippy --locked --workspace --all-targets -- -D warnings
git add src Cargo.toml Cargo.lock .github/workflows/ci.yml
git commit -m "Give the MCP server a gateway to the saved connections, with sessions of its own"
```

---

### Task 5: The backend runs the server

`AgentsCommand` is the backend's word for the server: `Policy` (sent whenever the connections or the host keys change, and kept while the server is off), `Start { log, log_days }` and `Stop`. `AgentsEvent` answers: `Started`, with where a log that could not be read was kept, `Failed`, with why, and `Stopped`.

- `Agents`, the worker's part, holds the gateway, the server while it runs, a count of starts and stops, and the lock one start holds while it opens the log.
- `chores` closes the sessions left idle and, as the server starts and every six hours, forgets old calls.
- `App::apply` takes `Event::Agents` and does nothing with it: nothing in the app starts the server yet.

**Files:**
- Modify: `src/backend/agents.rs`, `src/backend.rs`, `src/app.rs`

- [ ] **Step 1: Write the failing tests**

````diff
diff --git a/src/backend.rs b/src/backend.rs
index 7ff6b80..f002b7e 100644
--- a/src/backend.rs
+++ b/src/backend.rs
@@ -20,7 +20,7 @@ use crate::secrets::{Keyring, SecretString};
 
 mod agents;
 
-pub use agents::{Agentable, AppGateway, agentable};
+pub use agents::{Agentable, AgentsCommand, AgentsEvent, AppGateway, agentable};
 
 /// Wakes the UI from any thread. The default does nothing (tests).
 #[derive(Clone)]
@@ -188,6 +188,8 @@ pub enum Command {
     Flush {
         done: mpsc::Sender<()>,
     },
+    /// The MCP server: what agents may use, and whether it runs.
+    Agents(AgentsCommand),
 }
 
 /// The contents of a state file the backend saves.
@@ -340,6 +342,8 @@ pub enum Event {
         with: Opened,
         result: Result<(), String>,
     },
+    /// What came of an [`AgentsCommand`].
+    Agents(AgentsEvent),
 }
 
 /// What the settings file was handed to.
@@ -1588,6 +1592,8 @@ struct Worker {
     connecting: std::collections::HashSet<SessionId>,
     /// Watches and reads the settings file. Dropping it ends both.
     settings_watch: Option<SettingsReader>,
+    /// The MCP server, its gateway and what agents may use.
+    agents: agents::Agents,
     #[cfg(test)]
     watched: Watched,
 }
@@ -1596,6 +1602,7 @@ impl Worker {
     fn new(outbox: Outbox, keyring: Keyring) -> (Self, tokio_mpsc::UnboundedReceiver<Ready>) {
         let (ready, ready_rx) = tokio_mpsc::unbounded_channel();
         let worker = Self {
+            agents: agents::Agents::new(keyring.clone()),
             outbox,
             keyring,
             saves: Saves::default(),
@@ -1853,6 +1860,7 @@ impl Worker {
                     let _ = done.send(());
                 });
             }
+            Command::Agents(command) => self.agents.handle(command, &self.outbox),
             query => {
                 let session = session_of(&query);
                 match self.sessions.get(&session) {
@@ -1894,7 +1902,8 @@ fn session_of(command: &Command) -> SessionId {
         | Command::WatchSettings { .. }
         | Command::EditSettingsFile { .. }
         | Command::RevealSettingsFile { .. }
-        | Command::Flush { .. } => SessionId(0),
+        | Command::Flush { .. }
+        | Command::Agents(_) => SessionId(0),
     }
 }
 
@@ -1921,7 +1930,8 @@ fn request_of(command: &Command) -> Option<RequestId> {
         | Command::WatchSettings { .. }
         | Command::EditSettingsFile { .. }
         | Command::RevealSettingsFile { .. }
-        | Command::Flush { .. } => None,
+        | Command::Flush { .. }
+        | Command::Agents(_) => None,
     }
 }
 
@@ -2006,7 +2016,8 @@ fn fail(outbox: &Outbox, command: Command, error: Error) {
         | Command::WatchSettings { .. }
         | Command::EditSettingsFile { .. }
         | Command::RevealSettingsFile { .. }
-        | Command::Flush { .. } => return,
+        | Command::Flush { .. }
+        | Command::Agents(_) => return,
     };
     outbox.emit(event);
 }
@@ -2287,7 +2298,8 @@ async fn run_session(
             | Command::WatchSettings { .. }
             | Command::EditSettingsFile { .. }
             | Command::RevealSettingsFile { .. }
-            | Command::Flush { .. } => None,
+            | Command::Flush { .. }
+            | Command::Agents(_) => None,
         };
         end.command = None;
         {
diff --git a/src/backend/agents.rs b/src/backend/agents.rs
index dea1732..80780c5 100644
--- a/src/backend/agents.rs
+++ b/src/backend/agents.rs
@@ -445,10 +445,13 @@ mod tests {
     use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
 
     use super::*;
+    use crate::backend::{Backend, Command, Event, Waker};
     use crate::connections::Agents as AgentSettings;
     use crate::env::Environment;
     use crate::secrets::SecretString;
 
+    const WAIT: Duration = Duration::from_secs(20);
+
     fn saved(id: &str, name: &str, environment: Environment, spec: ConnectSpec) -> SavedConnection {
         SavedConnection {
             id: ConnectionId(id.into()),
@@ -937,6 +940,118 @@ mod tests {
         assert_eq!(entries[0].connection_id.as_deref(), Some("shop"));
     }
 
+    #[test]
+    fn the_backend_starts_and_stops_the_server_and_keeps_its_log_to_its_days() {
+        let dir = tempfile::tempdir().unwrap();
+        let path = dir.path().join("agents.sqlite3");
+        // A call of long ago, and one of now.
+        let runtime = tokio::runtime::Runtime::new().unwrap();
+        runtime.block_on(async {
+            let log = Log::open(&path).unwrap().log;
+            for at_ms in [
+                1_000,
+                i64::try_from(
+                    SystemTime::now()
+                        .duration_since(UNIX_EPOCH)
+                        .unwrap()
+                        .as_millis(),
+                )
+                .unwrap(),
+            ] {
+                let entry = tabletist_mcp::Entry {
+                    at_ms,
+                    session: "s".into(),
+                    agent: "a".into(),
+                    transport: "stdio".into(),
+                    tool: "list_connections".into(),
+                    result: "ok".into(),
+                    ..tabletist_mcp::Entry::default()
+                };
+                log.record(entry).await.unwrap();
+            }
+        });
+        let mut backend = Backend::start_with(Waker::default(), Keyring::memory());
+        backend.send(Command::Agents(AgentsCommand::Start {
+            log: path.clone(),
+            log_days: 30,
+        }));
+        assert_eq!(
+            match backend.wait(WAIT) {
+                Some(Event::Agents(event)) => event,
+                other => panic!("{other:?}"),
+            },
+            AgentsEvent::Started { kept_aside: None }
+        );
+        // Retention runs as the server starts.
+        let deadline = Instant::now() + WAIT;
+        let left = loop {
+            let left = runtime.block_on(async {
+                Log::open(&path)
+                    .unwrap()
+                    .log
+                    .latest(10)
+                    .await
+                    .unwrap()
+                    .len()
+            });
+            if left == 1 || Instant::now() > deadline {
+                break left;
+            }
+            std::thread::sleep(Duration::from_millis(50));
+        };
+        assert_eq!(left, 1, "the call of long ago is forgotten");
+        backend.send(Command::Agents(AgentsCommand::Stop));
+        assert!(matches!(
+            backend.wait(WAIT),
+            Some(Event::Agents(AgentsEvent::Stopped))
+        ));
+        // A log that cannot be made: the server does not start, and says
+        // why.
+        backend.send(Command::Agents(AgentsCommand::Start {
+            log: dir.path().join("no-such-folder").join("agents.sqlite3"),
+            log_days: 30,
+        }));
+        match backend.wait(WAIT) {
+            Some(Event::Agents(AgentsEvent::Failed(said))) => {
+                assert!(
+                    said.starts_with("the log of agents' calls could not be opened"),
+                    "{said}"
+                );
+            }
+            other => panic!("{other:?}"),
+        }
+    }
+
+    #[test]
+    fn starts_and_stops_one_after_another_leave_the_last_one_standing() {
+        let dir = tempfile::tempdir().unwrap();
+        let path = dir.path().join("agents.sqlite3");
+        let mut backend = Backend::start_with(Waker::default(), Keyring::memory());
+        let start = || {
+            Command::Agents(AgentsCommand::Start {
+                log: path.clone(),
+                log_days: 30,
+            })
+        };
+        backend.send(start());
+        backend.send(start());
+        backend.send(Command::Agents(AgentsCommand::Stop));
+        backend.send(start());
+        let mut said = Vec::new();
+        while let Some(Event::Agents(event)) = backend.wait(Duration::from_secs(2)) {
+            said.push(event);
+        }
+        assert_eq!(
+            said,
+            [
+                AgentsEvent::Stopped,
+                AgentsEvent::Started { kept_aside: None }
+            ],
+            "{said:?}"
+        );
+        assert!(!dir.path().join("agents.sqlite3.bad").exists());
+    }
+
     /// A server's connection from a test URL, its password in the keyring.
     async fn on_a_server(url_variable: &str) -> Option<(AppGateway, Dialect)> {
         let Ok(url) = std::env::var(url_variable) else {
````

- [ ] **Step 2: Run them to see that they do not build**

Run: `cargo test --locked --lib backend::agents`

Expected: it does not build:

```text
error[E0432]: unresolved imports `agents::AgentsCommand`, `agents::AgentsEvent`
error[E0433]: cannot find `Agents` in `agents`
error[E0004]: non-exhaustive patterns: `backend::Event::Agents(_)` not covered
error: could not compile `tabletist` (lib test) due to 18 previous errors
```

- [ ] **Step 3: The server in the worker**

````diff
diff --git a/src/app.rs b/src/app.rs
index 70b1ac2..a4dced6 100644
--- a/src/app.rs
+++ b/src/app.rs
@@ -2796,6 +2796,9 @@ impl App {
                 };
                 self.apply_settings(settings);
             }
+            // Nothing in the app starts the agent server yet: that comes
+            // with its settings. The backend logs what happens to it.
+            Event::Agents(_) => {}
             Event::SettingsFileOpened { with, result } => {
                 if let Err(error) = result {
                     let path = self.dirs.settings_file();
diff --git a/src/backend/agents.rs b/src/backend/agents.rs
index 80780c5..db1828e 100644
--- a/src/backend/agents.rs
+++ b/src/backend/agents.rs
@@ -7,19 +7,20 @@
 //! is read-only at the database, whatever the connection's own box says.
 
 use std::collections::HashMap;
+use std::path::PathBuf;
 use std::sync::{Arc, Mutex};
-use std::time::{Duration, Instant};
+use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
 
 use tabletist_db::agent::{Answer, Limits, Read};
 use tabletist_db::{
     Access, CancelHandle, ConnectSpec, Connection, Error, HostKeys, ObjectInfo, ObjectRef, Secrets,
     StopFlag, Structure,
 };
-use tabletist_mcp::{Failure, Gateway, Listed, Schemas, Session};
+use tabletist_mcp::{Failure, Gateway, Listed, Log, Schemas, Session};
 use tokio::sync::OwnedMutexGuard;
 use tokio_util::sync::CancellationToken;
 
-use super::{CANCEL_AGAIN, lock, longer, send_cancel};
+use super::{CANCEL_AGAIN, Outbox, lock, longer, send_cancel};
 use crate::connections::{AgentAccess, ConnectionId, PasswordMode, SavedConnection};
 use crate::env::Platform;
 use crate::model::SecretKind;
@@ -110,10 +111,190 @@ fn short_id(id: &ConnectionId, length: usize) -> String {
         .collect()
 }
 
+/// What the backend is told of the server.
+#[derive(Debug)]
+pub enum AgentsCommand {
+    /// The connections agents may use, and the SSH host keys the user
+    /// trusts. Sent whenever either changes. Kept while the server is off.
+    Policy {
+        connections: Vec<Agentable>,
+        host_keys: HostKeys,
+    },
+    /// Starts the server, with its log at `log`, which keeps calls for
+    /// `log_days` days. Answered with [`AgentsEvent::Started`] or
+    /// [`AgentsEvent::Failed`].
+    Start { log: PathBuf, log_days: u32 },
+    /// Stops it: every session ends, and the connections are closed.
+    /// Answered with [`AgentsEvent::Stopped`].
+    Stop,
+}
+
+/// What the backend tells of the server.
+#[derive(Debug, Clone, PartialEq, Eq)]
+pub enum AgentsEvent {
+    /// It runs. `kept_aside` is where a log that could not be read was put.
+    Started {
+        kept_aside: Option<PathBuf>,
+    },
+    /// It could not start: its log could not be opened. Why, for the
+    /// person.
+    Failed(String),
+    Stopped,
+}
+
+/// How long a connection an agent used is kept open with nothing to do.
+const IDLE: Duration = Duration::from_secs(10 * 60);
+
+/// How often the server's chores are done: closing what is idle, and,
+/// every [`RETENTION_EVERY`], forgetting old calls.
+const CHORES_EVERY: Duration = Duration::from_secs(60);
+
+/// How often the log forgets what is older than it keeps.
+const RETENTION_EVERY: Duration = Duration::from_secs(6 * 60 * 60);
+
 /// How long opening a connection for an agent may take, an SSH tunnel's
 /// among it.
 const CONNECT_TIME: Duration = Duration::from_secs(20);
 
+/// The server in the backend, while it runs.
+pub(super) struct Running {
+    server: tabletist_mcp::Server<AppGateway>,
+    chores: tokio::task::JoinHandle<()>,
+}
+
+/// The server's part of the worker: the gateway, whatever the server is
+/// doing, and the server while it runs.
+pub(super) struct Agents {
+    gateway: AppGateway,
+    running: Arc<Mutex<Option<Running>>>,
+    /// Counted up by each start and stop, so that a start whose log is
+    /// still opening when a stop comes runs nothing, and says nothing.
+    turn: Arc<Mutex<u64>>,
+    /// Held by a start from the opening of its log to its end: two starts
+    /// must not open the one file at once.
+    starting: Arc<tokio::sync::Mutex<()>>,
+}
+
+impl Agents {
+    pub fn new(keyring: Keyring) -> Self {
+        Self {
+            gateway: AppGateway::new(keyring),
+            running: Arc::default(),
+            turn: Arc::default(),
+            starting: Arc::default(),
+        }
+    }
+
+    pub fn handle(&mut self, command: AgentsCommand, outbox: &Outbox) {
+        match command {
+            AgentsCommand::Policy {
+                connections,
+                host_keys,
+            } => self.gateway.set_policy(connections, host_keys),
+            AgentsCommand::Start { log, log_days } => {
+                let turn = {
+                    let mut turn = lock(&self.turn);
+                    *turn += 1;
+                    *turn
+                };
+                let gateway = self.gateway.clone();
+                let running = Arc::clone(&self.running);
+                let current = Arc::clone(&self.turn);
+                let starting = Arc::clone(&self.starting);
+                let outbox = outbox.clone();
+                tokio::spawn(async move {
+                    let _starting = starting.lock().await;
+                    let overtaken = || *lock(&current) != turn;
+                    if overtaken() {
+                        return;
+                    }
+                    let opened = tokio::task::spawn_blocking(move || Log::open(&log)).await;
+                    let opened = match opened {
+                        Ok(Ok(opened)) => opened,
+                        Ok(Err(error)) => {
+                            log::error!("the agent server did not start: {error}");
+                            // A stop or a later start came meanwhile: theirs
+                            // is the last word.
+                            if !overtaken() {
+                                let said = format!(
+                                    "the log of agents' calls could not be opened: {error}"
+                                );
+                                outbox.emit(super::Event::Agents(AgentsEvent::Failed(said)));
+                            }
+                            return;
+                        }
+                        Err(_) => return,
+                    };
+                    let server = tabletist_mcp::Server::new(
+                        gateway.clone(),
+                        opened.log.clone(),
+                        env!("CARGO_PKG_VERSION"),
+                    );
+                    let mut running = lock(&running);
+                    // A stop came while the log was opening.
+                    if overtaken() {
+                        return;
+                    }
+                    if let Some(before) = running.take() {
+                        before.stop();
+                    }
+                    let chores = tokio::spawn(chores(opened.log, gateway, log_days));
+                    *running = Some(Running { server, chores });
+                    log::info!("the agent server started");
+                    // Said under the lock a stop takes, so that no Stopped
+                    // is said before it.
+                    outbox.emit(super::Event::Agents(AgentsEvent::Started {
+                        kept_aside: opened.kept_aside,
+                    }));
+                });
+            }
+            AgentsCommand::Stop => {
+                *lock(&self.turn) += 1;
+                let mut running = lock(&self.running);
+                if let Some(running) = running.take() {
+                    running.stop();
+                }
+                self.gateway.close_all();
+                log::info!("the agent server stopped");
+                outbox.emit(super::Event::Agents(AgentsEvent::Stopped));
+            }
+        }
+    }
+}
+
+impl Running {
+    fn stop(self) {
+        self.server.stop();
+        self.chores.abort();
+    }
+}
+
+/// What the server does from time to time while it runs: closes the
+/// connections no agent has used for a while, and forgets calls older than
+/// the log keeps. The first time as it starts.
+async fn chores(log: Log, gateway: AppGateway, log_days: u32) {
+    let keep = Duration::from_secs(u64::from(log_days.max(1)) * 24 * 60 * 60);
+    let mut forgot: Option<Instant> = None;
+    loop {
+        if forgot.is_none_or(|at| at.elapsed() >= RETENTION_EVERY) {
+            let before = SystemTime::now()
+                .checked_sub(keep)
+                .and_then(|before| before.duration_since(UNIX_EPOCH).ok())
+                .map_or(0, |before| {
+                    i64::try_from(before.as_millis()).unwrap_or(i64::MAX)
+                });
+            match log.forget_before(before).await {
+                Ok(0) => {}
+                Ok(gone) => log::info!("the agents' log forgot {gone} calls"),
+                Err(error) => log::warn!("the agents' log could not forget old calls: {error}"),
+            }
+            forgot = Some(Instant::now());
+        }
+        gateway.close_idle(IDLE);
+        tokio::time::sleep(CHORES_EVERY).await;
+    }
+}
+
 /// The gateway of the app's MCP server: the saved connections an agent
 /// may use, and a session of its own for each. A clone is the same gateway.
 #[derive(Clone)]
````

- [ ] **Step 4: Run the tests**

Run: `cargo test --locked --lib backend::agents`

Expected: PASS.

```text
test result: ok. 16 passed; 0 failed; 0 ignored; 0 measured; 2418 filtered out
```

- [ ] **Step 5: Commit**

```bash
cargo fmt --all
cargo clippy --locked --workspace --all-targets -- -D warnings
git add src
git commit -m "Start and stop the MCP server in the backend, with its log"
```

---

### Task 6: The master plan says what was built

**Files:**
- Modify: `docs/superpowers/plans/2026-10-13-mcp-server.md`

- [ ] **Step 1: Correct it**

````diff
diff --git a/docs/superpowers/plans/2026-10-13-mcp-server.md b/docs/superpowers/plans/2026-10-13-mcp-server.md
index ea2e6fc..0658107 100644
--- a/docs/superpowers/plans/2026-10-13-mcp-server.md
+++ b/docs/superpowers/plans/2026-10-13-mcp-server.md
@@ -30,6 +30,11 @@ what each takes and answers: that run's plan says. A.1, A.4, A.5, B.5, B.8,
 C.1, F.1 and G are corrected where its draft showed otherwise, each where it
 changed.
 
+**Built on 2026-10-14: the first part of Phase 1b's second run** (`2026-10-14-mcp-server-1b-2a-gateway.md`):
+the backend's gateway, and agents in `connections.toml`. G cuts the second
+run in two, as it said it might. A.1, C.1, F.4, G and question 19 are
+corrected where its draft showed otherwise.
+
 Each phase below gets its own plan under `docs/superpowers/plans/` before any
 code is written, as every batch of work in this repository does.
 
@@ -163,6 +168,32 @@ name is the agent's word for it and can change. One call at a time, `busy`
 and the idle close are the implementation's to keep, behind `open`. The first run has the trait, and for its tests a gateway over SQLite
 files. The backend's own is the second run's.
 
+**The backend's gateway, as built** (`2026-10-14-mcp-server-1b-2a-gateway.md`). `AppGateway` keeps a session
+of its own for each connection an agent has used, opened read-only with the
+connection's secrets from the keyring, and hands it to one call at a time.
+What its draft's review showed it must also do:
+
+- **Ask the person nothing.** A read of the keyring for an agent never
+  brings up a prompt to unlock it: on Linux the Secret Service would ask
+  for a collection whose item a read finds locked, so the item is looked
+  for first and a locked one is not read (`SecretStore::read_quietly`).
+  Windows asks nothing of a read. **On macOS a locked keychain is asked to
+  be unlocked by the system, and the keyring crate has no call that says
+  not to**: open for the second part, which makes the server reachable.
+- **Look again after a wait.** A call that waited for the connection, or
+  for its connect, takes the connection as it is once it has it: turned
+  Off meanwhile, it runs nothing; changed, it is opened anew.
+- **Open a lost session again.** A session the database ended answers that
+  it is lost, and is closed when the call lets it go, so the next call
+  opens it again.
+- **Let a stop's cancels land.** The cancels a stop sends are awaited
+  before the session is let go, so that none lands on the next call's
+  statement, as the backend's own are.
+- A connect may take 20 seconds, an SSH tunnel's among it. A session no
+  call has used for ten minutes is closed, looked at every minute.
+- The first schema is the app's own rule (`model::first_schema`), which the
+  workspace's tree and the gateway share.
+
 **Secrets.** The `Worker` owns the `Keyring` (`src/backend.rs:1567`). The
 gateway reads `connection/{id}/password` and `connection/{id}/ssh-secret`
 (`src/secrets.rs:18-25`) through it, inside the backend, when it opens a
@@ -1085,6 +1116,12 @@ an agent's settings are. Where they are kept changes:
   is read strictly, so a Tabletist that does not know the table refuses
   the file and keeps it aside, where the JSON was read past what it did not
   know. Agents are not turned off by a downgrade. They are not read at all.
+  **As built** (`2026-10-14-mcp-server-1b-2a-gateway.md`): a file whose connections have no agent settings
+  is still written as version 3, so that only a file that holds some is
+  refused by an older Tabletist. The table is `access` (`off` or `read`),
+  `rows` (1 to 5,000) and `timeout_secs` (1 to 60), and a key left out
+  takes what a connection starts with. A duplicated connection starts Off
+  for agents, as every new one does.
 - When each connection last connected is in the state directory
   (`last_used.json`), so a connect no longer writes the connections.
 - "Stop agents here" (A.5) writes `access = "off"` and keeps the level that
@@ -1252,10 +1289,12 @@ what is traced to the app's log. What it writes is what a client sent:
 
 `sqlparser`, which the check reads a statement with, writes each word at
 `debug`, and `tracing` writes lines of its own for the SDK's spans. So the
-app's log takes no line of the three. The second run, which links the
-server into the app, turns them off in the app's filter
-(`src/entrypoint.rs`), with a test that a session leaves nothing an agent
-sent in `tabletist.log`. The first run's `tests/secrets.rs` holds every
+app's log takes no line of the three. **As built** (`2026-10-14-mcp-server-1b-2a-gateway.md`), which links the
+server into the app: the app's filter turns the three off
+(`entrypoint::log_filter`), and because a `RUST_LOG` takes that filter's
+place as a whole, the logger's rewriter blanks any line of theirs that a
+`RUST_LOG` lets through (`entrypoint::without_agents_words`), so that the
+line is there and none of its words. The first run's `tests/secrets.rs` holds every
 other crate's lines to that, and shows what the SDK's hold. `AGENTS.md` has
 the rule.
 
@@ -1786,7 +1825,9 @@ runs on tokio's paused clock, which the workspace already uses in tests
 - A file that cannot be opened is set aside and a new one made.
 - `connections.json`: a file with `agents` loads and saves unchanged. The
   version 1 fixture (`tests/fixtures/connections-v1.json`) loads with every
-  connection Off.
+  connection Off. Since the move to TOML that is `connections.toml`'s, in
+  `src/connections/file.rs`: version 3 and 4, a table with keys left out,
+  limits refused past their bounds.
 
 ### F.5 Credentials never appear
 
@@ -1942,7 +1983,8 @@ can see what agents do and stop them.
 | Run | What it builds | What a user can do after it | Tests |
 |---|---|---|---|
 | **1b-1. The server, with nothing to start it** | The `tabletist-mcp` crate: the server on `rmcp`, the gateway's trait, `list_connections`, `describe_schema` and `run_query` over `agent::check_read` and `Connection::run_agent_read`, the size cap on an answer (B.5), the refusal codes (B.8), the log's store (C.1) | Nothing. No setting, no transport and no command reaches it | F.1 in process, F.4 but its interface, F.5 in process |
-| **1b-2. Turning it on from the files, and stdio** | The `[mcp]` keys in `settings.toml`. `[connections.agents]` with access, rows and timeout, which makes the connections file version 4. `Command::Mcp` and `Event::Mcp`, the backend's gateway with its own sessions on each database, and the server started and stopped with the setting. The schedule of the log's retention, and the SDK held out of the app's log. The endpoint, the key and the relay, `tabletist mcp`, on Linux and macOS | With `enabled = true` and an access written by hand, an agent connects over stdio and reads. Nothing in the app shows it yet, and nothing in the app turns it on | F.1 over stdio, F.5 over stdio, the settings and connections files, `tests/cli.rs` |
+| **1b-2a. The backend's gateway** (built: `2026-10-14-mcp-server-1b-2a-gateway.md`) | `[connections.agents]` with access, rows and timeout, which makes the connections file version 4. `Command::Agents` and `Event::Agents`, the backend's gateway with its own sessions on each database, and the server started and stopped by the backend, with the log's retention. The SDK held out of the app's log | Nothing: no setting starts the server, and no transport reaches it | The connections file, the gateway on SQLite, PostgreSQL and MySQL, the backend's start and stop, the log's filter |
+| **1b-2b. Turning it on from the files, and stdio** | The `[mcp]` keys in `settings.toml`, and the app that starts and stops the server with them and sends the backend what agents may use. The endpoint, the key and the relay, `tabletist mcp`, on Linux and macOS. The keychain on macOS (A.1). The website's reference for the files | With `enabled = true` and an access written by hand, an agent connects over stdio and reads. Nothing in the app shows it yet, and nothing in the app turns it on | F.1 over stdio, F.5 over stdio, the settings file, `tests/cli.rs` |
 | **1b-3. The Agents tab's frame** | The third kind of tab. The sidebar entry with its five states. The ways in: the key, `ga`, `:agents`, quick open. The three empty states, with **Turn on** and **Copy**. The keymap and the help | Open the tab in any state, see which state the connection is in, turn the server on from the tab | F.7: the entry's states, the ways in, one tab for a window, the empty states, nothing opening or focusing by itself |
 | **1b-4. What agents are doing** | `App::agents`. The sessions that are here and "Also:". The feed. **Stop session** and **Stop agents here**. The Log view as a list for this connection, with All connections. The notification for an agent that connects | See each agent and each call as it happens, stop one agent or all on a connection, read the log | F.7: one connection, the stops, closing the tab, the log's default. F.4's interface |
 | **1b-5. Letting agents in** | Settings › Agents, with the switch between panes. The Connections list's Agents column, its menu and Omarchy's box. The connection editor's Agents section, with **Allow agents…** and **Show Agents tab** pointing at them. The website: a new guide, and the reference pages | Everything Phase 1 draws, without a file edited by hand | The interface tests of Settings, Connections and the editor. The keymap. `docs/bin/check` |
@@ -1968,8 +2010,9 @@ can see what agents do and stop them.
   the fifth.
 - **Sizes.** The first had the risk of `rmcp` and of the first log. The
   gateway's own sessions on each database went to the second run with the
-  backend's gateway, which makes the second the largest: its plan may cut it
-  in two, the gateway and the files, then the endpoint and the relay.
+  backend's gateway, which made the second the largest: it is cut in two,
+  the gateway and the files (1b-2a), then the settings, the endpoint and
+  the relay (1b-2b).
 - **"N of M"** (B.5) is a run of its own in `tabletist-db`, at any time
   before the fourth.
 
@@ -2109,10 +2152,13 @@ The ones Phase 1 waits for are marked.
     gate for stdio, which has none.
 18. Should the log keep parameters in full? They can hold personal data.
     The alternative keeps their types and lengths.
-19. The agent-facing name of a connection is its name and environment
-    ("Bookshop · dev"). Two visible connections with the same one get
-    " (2)" on the second. Or should the editor refuse to save agent access
-    for a name that is taken?
+19. Settled on 2026-10-14, otherwise than planned (`2026-10-14-mcp-server-1b-2a-gateway.md`): a connection
+    is its name and environment ("Bookshop · dev"). Where two saved
+    connections would be called the same, Off or not, each is called that
+    with the start of its id after it ("Bookshop · dev (1f0c2a9e)"). The
+    planned " (2)" moved: turning the first Off made the second "Bookshop ·
+    dev", and an agent's next call reached another database. A name now
+    never comes to mean another one: at worst it means none.
 20. An `UPDATE` or `DELETE` without a `WHERE`: a warning on the card, as
     planned, or refused outright for agents?
 
````

- [ ] **Step 2: Check it**

Run: `grep -c $'\u2014' docs/superpowers/plans/2026-10-13-mcp-server.md` (an em dash, written so that this plan holds none)

Expected: `0`.

- [ ] **Step 3: Commit**

```bash
git add docs/superpowers/plans/2026-10-13-mcp-server.md
git commit -m "Say in the MCP plan what the first part of its server's second run built"
```

---

### After the last task

- [ ] **The whole of it, once more**, with both servers' variables set:

```bash
cargo fmt --all --check
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo test --locked --workspace --all-targets
RUSTDOCFLAGS='-D warnings' cargo doc --locked --workspace --no-deps
```

Expected: all clean, and every test passes. On 2026-10-14, with both servers up: 23 test binaries, 3,371 passed, none failed, 22 ignored.

- [ ] **Open the pull request.** `packaging.yml` starts with it, since `Cargo.toml` and `Cargo.lock` changed. The description says what was only compiled here (macOS, Windows), that nothing starts the server, and that the keychain on macOS is open.

## Self-review

- **Against the master plan's row for the second run** (G), as cut: `[connections.agents]` and version 4 (Task 1), the backend's gateway with its own sessions on each database (Task 4), the server started and stopped by the backend with the log's retention (Task 5), the SDK held out of the app's log (Task 2). Not here, and said above: the `[mcp]` settings, the app that sends `Command::Agents`, the endpoint, the key, the relay. Task 6 cuts the row in two.
- **Against A.1.** One read session for each connection an agent has used, read-only whatever its box says, one call at a time with `busy` after the connection's timeout, closed after ten idle minutes, when access goes Off, when the connection changes and when the server stops. Secrets from the keyring inside the backend, never sent: `Ask`, a keyring with nothing saved, a locked one and an untrusted SSH host answer `connection_unavailable` through `Failure::Unavailable`.
- **Against A.5.** A stop sends the session's cancel again and again until the driver says it is finishing, awaits the statement to its end, and awaits its cancels.
- **Against C.1.** The table and the version, the log filter, and the rule the review added for a `RUST_LOG`.
- **Question 19** is answered otherwise than planned, and the master plan says why.
- **Reviewed before it was written.** A reviewer was given the draft and both servers and asked to break it, with tests of its own. It found these, each fixed above with a test: a read of a locked keyring would ask to unlock it; a lost session was never opened again; a call that waited across a change of the policy used the old one; a name moved to another database when a connection was turned Off; two starts raced on the log; a `RUST_LOG` brought the SDK's lines back; and the filter's test read the filter and not a line. Two suspicions it could not confirm were closed all the same: a stop's cancels are awaited, and a duplicate starts Off. One finding is open, and said: the keychain on macOS. What held: read-only on every path, `Ask` refused before the keyring is asked, the SSH secret in its slot, no password in a line or a failure, busy and idle, retention's arithmetic, and the file's versions.
- **Names.** `AgentAccess`, `Agents`, `Agentable`, `agentable`, `AppGateway`, `AppSession`, `AgentsCommand`, `AgentsEvent`, `read_quietly`, `log_filter`, `without_agents_words`, `first_schema` are spelled the same in every task.
- **Every commit stands.** After Task 1 a connection has agent settings that nothing reads. After Task 2 the app's log is as it was for every crate it writes. After Task 3 nothing calls the quiet read. After Task 4 the gateway is there and nothing makes one. After Task 5 the backend can run the server, and the app sends it no command for it.

## After the run

Run on 2026-10-14, inline, one commit for each task. Every step showed what it says above, and the files the tasks changed are byte for byte the draft's. The last state passed the workspace's tests with PostgreSQL and MySQL up: 23 test binaries, 3,371 passed, none failed.

## After the pull request's review

The review found four things, each fixed after the run, with a test where one fits. The diffs above have the code as it was.

- **The keyring's quiet read failed open.** It looked for a locked item and otherwise read: had its search looked for the item otherwise than the keyring crate keeps it, it would have found nothing and the read would have asked to unlock. It now reads only an item it found unlocked, and one it did not find is a password not found (`secrets::item`).
- **A turn could be dropped while a call held it.** `set_policy` took a connection that went Off out of the map of turns, and `close_all` emptied it: a connection turned Off and on again, or a stop, while a call ran gave the next call a turn of its own, beside the first. A turn now stays in the map. A session that a change or a stop retired is closed at once when no call holds it, else when its call lets it go (`Inner::keeps`, `Inner::epoch`).
- **Names kept less than the doc said.** "What this run settles" has a name never come to mean another database. An add and then a remove of a connection called the same gives the bare name to the one left. The doc of `agentable` and the master plan's question 19 now say what holds: a name means one database while the connections called the same are the same ones. A test walks the add and the remove.
- **The website's reference** (`_reference/settings-and-files.md`) describes the `[connections.agents]` table and the file's two versions, which "What this run leaves out" had left to the second part: a table written by hand is read and kept now.

The comment on `AppSession::lost` said it is also set when a session is no longer what it was opened as. Only a lost session sets it: it says so now.

A second review found that the epoch of a new session was read once its connect was done: a stop while it opened was missed, and the session stayed open while the server was off. It is read before the connect now.

A third found that a log whose open panicked on the blocking pool left a start without an answer and without a line in the app's log. It is answered `Failed` and logged, as a log that could not be opened is.

A fourth found that `tokio_postgres` writes the statement it prepares or runs, with its values, at debug, which a `RUST_LOG` lets through, and a server's notice at info, which `--verbose` did. Its lines are off in both filters and blanked by the rewriter, as the SDK's are. `mysql_async` writes none: its spans are behind its `tracing` feature, which is not built.

A fifth found that a keyring busy with a tab's read, waiting on the person to unlock it, held an agent's call, and the connection's turn with it, past the connect's 20 seconds. The read was not without end: the keyring's own wait gives up after 20 seconds. But the read and the connect each had their 20, so a call could hold the turn for 40. The whole of the open, the read among it, is now within one `CONNECT_TIME`.
