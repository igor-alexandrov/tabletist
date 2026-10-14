# MCP server, Phase 1b, run 2b: turning it on from the files, and stdio. Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** With `enabled = true` under `[mcp]` in `settings.toml` and a connection's `access = "read"` written by hand, an agent that starts `tabletist mcp` reaches the running app over stdio and reads. Nothing in the app shows it yet, and nothing in the app turns it on.

**Architecture:** `settings.toml` gains `[mcp] enabled` and `log_days`. When the app starts it tells the backend what agents may use, and starts the server where the settings say; an edit of the file starts and stops it. The backend's server listens on a Unix socket in a directory private to the user (`$XDG_RUNTIME_DIR/tabletist` on Linux, the state directory on macOS), with a key beside it that is new each time the server starts. `tabletist mcp`, a subcommand of the same binary, connects to the socket, sends one line with the key, and from then on copies bytes both ways and understands none of them. On macOS an agent's read of the keychain is told to ask the person nothing.

**Tech Stack:** Rust 2024. `tokio`'s `net`, `io-util` and `io-std` features, and `security-framework` on macOS, all already in `Cargo.lock`: nothing new is built.

---

## Where this sits

Phase 1b of `docs/superpowers/plans/2026-10-13-mcp-server.md` is six runs (section G), of which two are merged: the server crate (`2026-10-13-mcp-server-1b-1-server.md`) and the backend's gateway (`2026-10-14-mcp-server-1b-2a-gateway.md`).

1. Done: the server, with nothing to start it.
2. Done: 2a, the backend's gateway. Agents in `connections.toml`, the gateway, the server in the backend, the app's log.
3. **This run, 2b: turning it on from the files, and stdio.** The `[mcp]` settings, the app that starts the server and sends it what agents may use, the endpoint, the key and the relay, the keychain on macOS, the website's reference for the files.
4. to 6. The Agents tab, what agents are doing, letting agents in.

After this run a person who edits both files by hand can let an agent read. Nothing a person does in the app reaches the server: no control turns it on, and nothing shows that it runs. Windows builds and tests all of it and serves nothing: its way in is a run of its own (question 14).

## Before you start

- Work on a branch of `main`. One commit for each task, in the order below. Every commit passes `cargo fmt --all --check`, `cargo clippy --locked --workspace --all-targets -- -D warnings` and `cargo test --locked --workspace --all-targets`.
- On Igor's machine `cargo` is `~/.cargo/bin/cargo` (the mise shim fails), and a build in a worktree needs `CARGO_TARGET_DIR` set to that worktree's `target/`.
- Read `AGENTS.md`. Three of its rules shape this code: nothing an agent sent goes into the app's log; `unsafe` is forbidden, so the socket's modes are set through `std`; and there are no em dashes.
- The diffs below apply with `git apply`. Run `cargo fmt --all` before each commit: the code here is already formatted.
- **`Cargo.lock` is in Task 3's diff**: one line in the `tabletist` package's list of dependencies, and nothing else. Every command has `--locked`. A build without it would also move six unrelated `windows-sys` pins: `git checkout Cargo.lock` and apply the diff again.
- **The servers.** Nothing in this run needs PostgreSQL or MySQL. The full run at the end prints "skipped" for their tests without the variables; with them:

  ```bash
  export TABLETIST_TEST_PG_URL=postgres://tabletist:tabletist@localhost:55432/tabletist
  export TABLETIST_TEST_MYSQL_URL=mysql://tabletist:tabletist@localhost:53306/tabletist
  ```

## What was run before this plan was written

Every task's state was built, passed `cargo fmt --check` and clippy with `-D warnings`, and its tests failed and then passed as each step says. The last state passed `cargo test --locked --workspace --all-targets` (3,395 tests, none failed), `RUSTDOCFLAGS='-D warnings' cargo doc --locked --workspace --no-deps` and `docs/bin/check`. Not run:

- **macOS and Windows.** CI runs every test there. The code that is theirs alone (the keychain's quiet read, the long-path fallback of the endpoint on macOS, and on Windows the paths that serve nothing) was type-checked with clippy for `aarch64-apple-darwin` and `x86_64-pc-windows-msvc` in an extract, not built whole: the workspace does not cross-compile on Igor's machine. Task 3's test runs only on macOS.
- **A locked keychain.** No test can lock one. The test is of the answer the keychain gives a read it was told to ask nothing for.
- **A real agent.** The relay is tested end to end through the built binary, with the protocol's lines written by the test. Claude Code and Cursor are tried by hand after the merge (the after-run note says how it went).

## What this run settles

| Question | Decision | Why |
|---|---|---|
| The server's settings | `[mcp] enabled` (false) and `log_days` (30, from 1 to 3,650, else brought into range), written in every file as every key is | C.1. `listen` comes with HTTP (1d) and `notify` with the notification for a new agent (the fourth run): a key that does nothing would be written into every file |
| Who starts the server | `App::serve_agents`, once the window is up, and only there: not in the demo, whose profile and keyring are throwaway, and not in tests. From then on `apply_settings` starts and stops it, from the app or from an edit of the file | A.1. A test that builds an `App` must not bind a socket |
| What agents may use | `Command::Agents(Policy)` with `agentable(&connections)` and the trusted host keys, at `serve_agents`, after every save of the connections and after a host key is trusted. Sent while the server is off too: the backend keeps it | A.1. Every change to a connection is saved, so a save is where the policy changes |
| `log_days` changed while the server runs | `AgentsCommand::LogDays`: the log's from its next chore, and no session stops | Restarting the server for it would end every agent's session |
| Where the app is reached | `AppDirs::agents_dir`: `/run/user/<id>/tabletist` on Linux, the user's id found as the owner of `/proc/self`, else the state directory. On macOS, where a socket's path may have 103 bytes, `.tabletist` in the home directory when the state directory's is longer | A.2. Found by the draft's review: MCP clients start a server with `HOME`, `PATH` and a few others, without `XDG_RUNTIME_DIR` or `TMPDIR`, so a relay that read them looked in another directory than the app |
| The socket and the key | `mcp.sock` (0600) and `mcp.key` (0600, a new key each start) in that directory, made 0700. One whose mode the user cannot change is not theirs, and is refused. **The key is beside the socket**, not in the state directory: the relay finds both in one place, and a test points it at another with `--endpoint`. The key is written before the socket is made | A.2. Agreed on 2026-10-14. The order was found by the draft's review: a relay that connected the moment the socket was there read the key an ended start had left, 50 times in 50 |
| What the relay says first | One line of JSON: the key, its version, its working directory. **The app answers with one line**, `"ok"` or a refusal with its reason. Each line may have 16 KiB and must come within five seconds | A.2 had the relay's line alone. Agreed on 2026-10-14: a wrong key, or too many agents, reaches the agent as the answer to its first request and not as a closed pipe |
| The key's check | Compared whole, in a time that does not depend on how much of it was right | A.3's rule for tokens |
| A socket already there | One that answers is another Tabletist's: the server does not start, and the app says so, before it opens the log. One that does not was left by a Tabletist that ended, and is replaced. Anything else there is in the way, and left alone | Two windows of the app on one account. Found by the draft's review: a second Tabletist of another version opened the log first, and set aside the one the first had open |
| When the server stops | The socket and the key are removed, each only while it is the one the server made: a later start may have put its own in place. A relay still saying its line is told the server is not running | Found by the draft's review: one that was saying it as the server stopped was answered "ok" by a server that served nothing |
| How many agents | 32 at once, counted once they have sent the key. One more is answered "Too many agents are connected". At most 64 connections that have not said their line yet: the next waits in the system's queue | A program of the user's that opens sessions without end runs into it, and the databases do not. Found by the draft's review: 32 connections that said nothing took every agent's place, and 2,000 more held 4,076 descriptors |
| `tabletist mcp` | A subcommand, which branches off before the logger, the settings and the window. It writes nothing to stdout but the protocol's bytes, opens no log file and shows no dialog. It tries the socket for two seconds | A.2 |
| When it cannot reach the app | It answers the agent's first request with a JSON-RPC error ("Tabletist is not running, or agents are off in it. Open Tabletist, turn agents on, then reconnect."), writes the same to stderr, and exits 1. A refusal from the app is passed on the same way | A.2 and question 10. The wording names no control: the app has none to name until the fifth run |
| When either side ends | The agent's end of stdin ends what the relay sends; the app still answers what it was asked. The app's end ends the relay, with 0 | A.2 |
| What the app says | A start that failed ("another Tabletist is serving agents already", a log that could not be made) is a notice, and so is a log kept aside, and on Windows agents turned on. Started and stopped say nothing. Nothing a relay sent goes into the app's log, its version neither: any program of the user's can send it | G's "nothing in the app shows it yet" is of what agents do. A person who turned the server on in the file and does not get it must be told |
| The keychain on macOS | An agent's read tells the keychain to ask nothing (`SecKeychain::disable_user_interaction`) until it ends. A locked keychain, and an item that would ask whether Tabletist may read it, answer `errSecInteractionNotAllowed`, which is `Locked` | A.1, open since the first part. The switch is the process's: the keyring thread, the only one that uses the keychain, does one job at a time |
| Windows | Everything builds and is tested there. `endpoint::AVAILABLE` is false: the app starts no server and says so, and `tabletist mcp` answers that agents cannot reach Tabletist on this system yet | Question 14 |
| The website | The reference page has the two keys, `agents.sqlite3`, `mcp.sock` and `mcp.key`, and `tabletist mcp` | G's row for this run. Agreed on 2026-10-14, against "the website says nothing of Agents until the fifth": the guide waits for the fifth |

### What this run leaves out

- **The check of changes made outside the app** (C.2). Agreed on 2026-10-14 to come with the first control that turns agents on, the third run's **Turn on**. Until then the files are the only way, and a check that refused what was changed outside the app would refuse the only way there is. So a program that can edit both files can turn agents on for itself, as E says of any program with a shell.
- **HTTP**, `listen`, tokens (1d). **`notify`** and the notification for a new agent (the fourth run).
- **Windows' way in**: the named pipe and the relay from a GUI binary (a run of its own, question 14).
- **The Agents tab, Settings › Agents, the connection editor's section** (the third to fifth runs). Nothing in the app shows that the server runs.
- **A relay that waits for the app**, and `--launch` (question 10).

## File structure

| File | What changes |
|---|---|
| `src/settings.rs` | `mcp_enabled` and `mcp_log_days`, the `[mcp]` table |
| `src/paths.rs` | `AppDirs::runtime`, `agents_dir`, `agents_log_file` |
| `src/endpoint.rs` | New. What the relay and the app say to each other first: the hello, the answer, the key, the project's name |
| `src/secrets.rs` | On macOS the keychain is told to ask nothing during an agent's read |
| `src/backend/agents/listen.rs` | New. The socket, the key, and the hello each relay must send |
| `src/backend/agents.rs` | `Start` listens where it is told, `LogDays` |
| `src/relay.rs` | New. `tabletist mcp` |
| `src/entrypoint.rs` | The `mcp` subcommand, before the logger and the window. The app serves agents |
| `src/app.rs` | `serve_agents`: what agents may use, sent on every change; the server started and stopped with the settings; what could not be done, said |
| `src/lib.rs` | The two new modules |
| `tests/cli.rs` | `tabletist mcp` end to end, with and without the app |
| `Cargo.toml`, `Cargo.lock` | `tokio`'s features, `security-framework` on macOS, each explained |
| `docs/_reference/settings-and-files.md` | The `[mcp]` keys, the new files, `tabletist mcp` |
| `docs/superpowers/plans/2026-10-13-mcp-server.md` | What this run built, and where it went otherwise than planned |

---

### Task 1: The `[mcp]` keys in `settings.toml`

`Settings` gains `mcp_enabled` (false) and `mcp_log_days` (30), under an `[mcp]` table the canonical text writes after `[editor]`. A value that is no flag is no yes, and the days are brought into the range from 1 to 3,650 as the other numbers are. Nothing reads them yet: Task 6 does.

**Files:**
- Modify: `src/settings.rs`

- [ ] **Step 1: Write the failing tests**

````diff
diff --git a/src/settings.rs b/src/settings.rs
index c6c4594..8ae061f 100644
--- a/src/settings.rs
+++ b/src/settings.rs
@@ -713,6 +713,8 @@ mod tests {
         assert_eq!(settings.custom_theme, None);
         assert_eq!(settings.sql_limit, 1_000);
         assert_eq!(settings.sql_timeout_secs, Some(30));
+        assert!(!settings.mcp_enabled);
+        assert_eq!(settings.mcp_log_days, 30);
         assert_eq!(
             settings.sql_timeout(),
             Some(std::time::Duration::from_secs(30))
@@ -745,6 +747,10 @@ show_system_schemas = false
 [editor]
 sql_limit        = 1000
 sql_timeout_secs = 30  # 0 waits forever
+
+[mcp]
+enabled  = false
+log_days = 30
 "
         );
     }
@@ -791,6 +797,8 @@ sql_timeout_secs = 30  # 0 waits forever
             custom_theme: Some("My \"Nord\".json".into()),
             sql_limit: 100,
             sql_timeout_secs: None,
+            mcp_enabled: true,
+            mcp_log_days: 7,
         };
         let text = settings.to_toml();
         let loaded = Settings::from_toml(&text);
@@ -815,6 +823,8 @@ sql_timeout_secs = 30  # 0 waits forever
                 (Key::ShowSystemSchemas, 9),
                 (Key::SqlLimit, 12),
                 (Key::SqlTimeoutSecs, 13),
+                (Key::McpEnabled, 16),
+                (Key::McpLogDays, 17),
             ]
         );
     }
@@ -912,6 +922,26 @@ sql_timeout_secs = 30  # 0 waits forever
         assert_eq!(loaded.settings.sql_timeout_secs, Some(1));
         let loaded = Settings::from_toml("[editor]\nsql_timeout_secs = 0\n");
         assert_eq!(loaded.settings.sql_timeout(), None);
+        // The log of agents' calls keeps a call a day at least, and ten
+        // years at most.
+        let loaded = Settings::from_toml("[mcp]\nlog_days = 0\n");
+        assert_eq!(loaded.settings.mcp_log_days, Settings::MIN_MCP_LOG_DAYS);
+        let loaded = Settings::from_toml("[mcp]\nlog_days = 100000\n");
+        assert_eq!(loaded.settings.mcp_log_days, Settings::MAX_MCP_LOG_DAYS);
+    }
+
+    #[test]
+    fn agents_are_turned_on_in_the_file_and_nothing_else_turns_them_on() {
+        let loaded = Settings::from_toml("[mcp]\nenabled = true\nlog_days = 90\n");
+        assert_eq!(loaded.invalid, Vec::<usize>::new());
+        assert!(loaded.settings.mcp_enabled);
+        assert_eq!(loaded.settings.mcp_log_days, 90);
+        // A value that is no flag is no yes.
+        for text in ["[mcp]\nenabled = \"true\"\n", "[mcp]\nenabled = 1\n"] {
+            let loaded = Settings::from_toml(text);
+            assert_eq!(loaded.invalid, vec![2], "{text}");
+            assert!(!loaded.settings.mcp_enabled, "{text}");
+        }
     }
 
     #[test]
````

- [ ] **Step 2: Run them to see that they do not build**

Run: `cargo test --locked --lib settings::`

Expected: it does not build. The tests name what is not there yet:

```text
error[E0609]: no field `mcp_enabled` on type `settings::Settings`
error[E0599]: no variant, associated function, or constant named `McpEnabled` found for enum `settings::Key` in the current scope
error[E0599]: no associated function or constant named `MIN_MCP_LOG_DAYS` found for struct `settings::Settings` in the current scope
error: could not compile `tabletist` (lib test) due to 13 previous errors
```

- [ ] **Step 3: The two keys**

````diff
diff --git a/src/settings.rs b/src/settings.rs
index 8ae061f..f65cf06 100644
--- a/src/settings.rs
+++ b/src/settings.rs
@@ -59,6 +59,11 @@ pub struct Settings {
     pub sql_limit: u32,
     /// Seconds before a SQL editor run is cancelled; `None` waits forever.
     pub sql_timeout_secs: Option<u32>,
+    /// The MCP server agents use the saved connections through. Off until
+    /// the person turns it on: nothing listens before.
+    pub mcp_enabled: bool,
+    /// Days the log of agents' calls keeps a call.
+    pub mcp_log_days: u32,
 }
 
 /// A key of the settings file.
@@ -71,12 +76,14 @@ pub enum Key {
     ShowSystemSchemas,
     SqlLimit,
     SqlTimeoutSecs,
+    McpEnabled,
+    McpLogDays,
     Theme,
 }
 
 impl Key {
     /// In the order the file has them: a table's keys together.
-    pub const ALL: [Key; 8] = [
+    pub const ALL: [Key; 10] = [
         Self::PageSize,
         Self::Timestamps,
         Self::GroupDigits,
@@ -84,6 +91,8 @@ impl Key {
         Self::ShowSystemSchemas,
         Self::SqlLimit,
         Self::SqlTimeoutSecs,
+        Self::McpEnabled,
+        Self::McpLogDays,
         Self::Theme,
     ];
 
@@ -97,6 +106,8 @@ impl Key {
             Self::ShowSystemSchemas => ("sidebar", "show_system_schemas"),
             Self::SqlLimit => ("editor", "sql_limit"),
             Self::SqlTimeoutSecs => ("editor", "sql_timeout_secs"),
+            Self::McpEnabled => ("mcp", "enabled"),
+            Self::McpLogDays => ("mcp", "log_days"),
             Self::Theme => ("appearance", "theme"),
         }
     }
@@ -384,6 +395,10 @@ impl Settings {
     /// The timeouts (seconds) the SQL editor's Timeout menu offers; `None`
     /// is no timeout.
     pub const SQL_TIMEOUTS: [Option<u32>; 5] = [Some(10), Some(30), Some(60), Some(300), None];
+    /// Days the log of agents' calls keeps a call, at least and at most: a
+    /// day, and ten years.
+    pub const MIN_MCP_LOG_DAYS: u32 = 1;
+    pub const MAX_MCP_LOG_DAYS: u32 = 3_650;
 
     /// The SQL editor's timeout as a duration; `None` waits forever.
     pub fn sql_timeout(&self) -> Option<std::time::Duration> {
@@ -470,6 +485,8 @@ impl Settings {
             Key::SqlLimit => self.sql_limit.to_string(),
             // TOML has no null: no timeout is no seconds.
             Key::SqlTimeoutSecs => self.sql_timeout_secs.unwrap_or(0).to_string(),
+            Key::McpEnabled => self.mcp_enabled.to_string(),
+            Key::McpLogDays => self.mcp_log_days.to_string(),
             Key::Theme => toml_string(self.custom_theme.as_deref()?),
         })
     }
@@ -515,6 +532,9 @@ impl Settings {
             .clamp(Self::MIN_PAGE_SIZE, Self::MAX_PAGE_SIZE);
         self.sql_limit = Self::valid_sql_limit(self.sql_limit);
         self.sql_timeout_secs = Self::valid_sql_timeout(self.sql_timeout_secs);
+        self.mcp_log_days = self
+            .mcp_log_days
+            .clamp(Self::MIN_MCP_LOG_DAYS, Self::MAX_MCP_LOG_DAYS);
         self
     }
 
@@ -540,6 +560,8 @@ impl Settings {
             Key::ShowSystemSchemas => flag.map(|flag| self.show_system_schemas = flag),
             Key::SqlLimit => number.map(|number| self.sql_limit = number),
             Key::SqlTimeoutSecs => number.map(|number| self.sql_timeout_secs = Some(number)),
+            Key::McpEnabled => flag.map(|flag| self.mcp_enabled = flag),
+            Key::McpLogDays => number.map(|number| self.mcp_log_days = number),
             Key::Theme => text.map(|name| self.custom_theme = Some(name.to_owned())),
         }
         .is_some()
@@ -694,6 +716,8 @@ impl Default for Settings {
             custom_theme: None,
             sql_limit: 1_000,
             sql_timeout_secs: Some(30),
+            mcp_enabled: false,
+            mcp_log_days: 30,
         }
     }
 }
````

- [ ] **Step 4: Run the tests**

Run: `cargo test --locked --lib settings::`

Expected: PASS.

```text
test result: ok. 52 passed; 0 failed; 0 ignored; 0 measured; 2387 filtered out
```

- [ ] **Step 5: Commit**

```bash
cargo fmt --all
cargo clippy --locked --workspace --all-targets -- -D warnings
git add src/settings.rs
git commit -m "Keep whether agents may reach the app, and how long their log keeps a call"
```

---

### Task 2: Where the relay finds the app, and what they say first

- `src/paths.rs`: `AppDirs::runtime`, Linux's `/run/user/<id>` found by the user's id (the owner of `/proc/self`), never by a variable an agent may leave out of the relay's environment. `agents_dir`, the endpoint's directory, and `agents_log_file`.
- `src/endpoint.rs`, new: the names of the socket and the key, the relay's `Hello` and the app's `Answer`, each a line of JSON, read a byte at a time so that nothing past it is taken. The key, compared whole. The name of a session's project, from the relay's working directory. `NOT_RUNNING`, what an agent is told when the app is not there.

Nothing uses them yet: Tasks 4 and 5 do.

**Files:**
- Create: `src/endpoint.rs`
- Modify: `src/paths.rs`, `src/lib.rs`

- [ ] **Step 1: Write the failing tests**

````diff
diff --git a/src/endpoint.rs b/src/endpoint.rs
new file mode 100644
index 0000000..eee2101
--- /dev/null
+++ b/src/endpoint.rs
@@ -0,0 +1,88 @@
+//! How `tabletist mcp` reaches the running app: a socket in a directory
+//! private to the user (`AppDirs::agents_dir`), and a key beside it that
+//! the relay sends first.
+//!
+//! The relay sends one line, a [`Hello`], and the app answers one line, an
+//! [`Answer`]. After that the bytes are the agent's and the server's, and
+//! the relay understands none of them: the protocol lives in the app, so an
+//! old relay keeps working with a newer one.
+
+#[cfg(test)]
+mod tests {
+    use super::*;
+
+    #[test]
+    fn a_key_is_long_new_each_time_and_compared_whole() {
+        let key = new_key();
+        assert_eq!(key.len(), 64);
+        assert!(key.chars().all(|c| c.is_ascii_hexdigit()), "{key}");
+        assert_ne!(key, new_key());
+        assert!(same_key(&key, &key));
+        assert!(!same_key(&key[..63], &key));
+        assert!(!same_key(&format!("{key}0"), &key));
+        let mut other = key.clone().into_bytes();
+        other[63] = if other[63] == b'0' { b'1' } else { b'0' };
+        assert!(!same_key(&String::from_utf8(other).unwrap(), &key));
+        assert!(!same_key("", &key));
+    }
+
+    #[test]
+    fn a_project_is_the_last_part_of_the_directory_and_nothing_odd() {
+        assert_eq!(
+            project("/home/ada/src/bookshop").as_deref(),
+            Some("bookshop")
+        );
+        assert_eq!(project("/").as_deref(), None);
+        assert_eq!(project("").as_deref(), None);
+        assert_eq!(
+            project("/home/ada/book\u{1b}[2Jshop").as_deref(),
+            Some("book[2Jshop")
+        );
+        assert_eq!(
+            project(&format!("/x/{}", "a".repeat(100))).unwrap().len(),
+            64
+        );
+    }
+
+    #[test]
+    fn the_lines_are_json_and_an_answer_names_its_kind() {
+        let hello = Hello {
+            key: "k".into(),
+            version: "1.0.0".into(),
+            cwd: Some("/home/ada".into()),
+        };
+        let line = line_of(&hello);
+        assert!(line.ends_with(b"\n"));
+        assert_eq!(
+            serde_json::from_slice::<Hello>(&line[..line.len() - 1]).unwrap(),
+            hello
+        );
+        assert_eq!(line_of(&Answer::Ok), b"\"ok\"\n");
+        assert_eq!(
+            line_of(&Answer::Refused("no".into())),
+            b"{\"refused\":\"no\"}\n"
+        );
+        // A newer relay may say more: what this one does not know is
+        // passed over.
+        let newer: Hello =
+            serde_json::from_str(r#"{"key":"k","version":"9","cwd":null,"shell":"zsh"}"#).unwrap();
+        assert_eq!(newer.cwd, None);
+    }
+
+    #[tokio::test]
+    async fn a_line_is_read_to_its_end_and_no_further() {
+        let mut from: &[u8] = b"{\"key\":1}\nafter";
+        let line = read_line(&mut from, MAX_LINE).await.unwrap();
+        assert_eq!(line.as_deref(), Some(&b"{\"key\":1}"[..]));
+        assert_eq!(from, b"after");
+        let mut empty: &[u8] = b"";
+        assert_eq!(read_line(&mut empty, MAX_LINE).await.unwrap(), None);
+        let mut cut: &[u8] = b"no end";
+        assert!(read_line(&mut cut, MAX_LINE).await.is_err());
+        let long = vec![b'a'; 11];
+        let mut long: &[u8] = &long;
+        assert!(read_line(&mut long, 10).await.is_err());
+        let mut fits: &[u8] = b"aaaaaaaaaa\n";
+        assert_eq!(read_line(&mut fits, 10).await.unwrap().unwrap().len(), 10);
+    }
+}
diff --git a/src/lib.rs b/src/lib.rs
index 0cbb159..61f25c9 100644
--- a/src/lib.rs
+++ b/src/lib.rs
@@ -5,6 +5,7 @@ pub mod backend;
 pub mod completion;
 pub mod connections;
 pub mod edit;
+pub mod endpoint;
 pub mod entrypoint;
 pub mod env;
 pub mod i18n;
diff --git a/src/paths.rs b/src/paths.rs
index 59a0baf..41c46ea 100644
--- a/src/paths.rs
+++ b/src/paths.rs
@@ -157,6 +157,17 @@ mod tests {
         assert_eq!(dirs.labels_file(), PathBuf::from("/root/state/labels.json"));
         assert_eq!(dirs.log_file(), PathBuf::from("/root/state/tabletist.log"));
         assert_eq!(dirs.panic_log(), PathBuf::from("/root/state/panic.log"));
+        assert_eq!(
+            dirs.agents_log_file(),
+            PathBuf::from("/root/state/agents.sqlite3")
+        );
+        // Without a runtime directory, with the state.
+        assert_eq!(dirs.agents_dir(), PathBuf::from("/root/state"));
+        let dirs = AppDirs {
+            runtime: Some(PathBuf::from("/run/user/1000")),
+            ..dirs
+        };
+        assert_eq!(dirs.agents_dir(), PathBuf::from("/run/user/1000/tabletist"));
     }
 
     #[test]
@@ -175,6 +186,19 @@ mod tests {
         assert!(dirs.state.is_dir());
     }
 
+    /// The relay and the app find one directory, whatever the agent left
+    /// out of the relay's environment.
+    #[cfg(target_os = "linux")]
+    #[test]
+    fn the_runtime_directory_is_the_users_own_and_no_variables() {
+        use std::os::unix::fs::MetadataExt;
+        if let Some(dir) = runtime_dir() {
+            let user = std::fs::metadata("/proc/self").unwrap().uid();
+            assert_eq!(dir, PathBuf::from(format!("/run/user/{user}")));
+        }
+        assert_eq!(AppDirs::discover().runtime, runtime_dir());
+    }
+
     #[test]
     fn discovered_directories_are_named_for_the_app() {
         let dirs = AppDirs::discover();
````

- [ ] **Step 2: Run them to see that they do not build**

Run: `cargo test --locked --lib endpoint::`

Expected: it does not build. The tests name what is not there yet:

```text
error[E0422]: cannot find struct, variant or union type `Hello` in this scope
error[E0425]: cannot find type `Hello` in this scope
error[E0425]: cannot find function `new_key` in this scope
error[E0599]: no method named `agents_dir` found for struct `paths::AppDirs` in the current scope
error[E0560]: struct `paths::AppDirs` has no field named `runtime`
error: could not compile `tabletist` (lib test) due to 35 previous errors; 1 warning emitted
```

- [ ] **Step 3: The paths and the lines**

````diff
diff --git a/src/endpoint.rs b/src/endpoint.rs
index eee2101..3c43ac2 100644
--- a/src/endpoint.rs
+++ b/src/endpoint.rs
@@ -7,6 +7,125 @@
 //! the relay understands none of them: the protocol lives in the app, so an
 //! old relay keeps working with a newer one.
 
+use std::time::Duration;
+
+use serde::{Deserialize, Serialize};
+use tokio::io::{AsyncRead, AsyncReadExt};
+
+/// Whether agents can reach the app on this system. Not on Windows yet: its
+/// named pipe and its relay are a run of their own.
+pub const AVAILABLE: bool = cfg!(unix);
+
+/// What the agent is told when the app is not there to reach, or stops as
+/// the relay says its line.
+pub const NOT_RUNNING: &str = "Tabletist is not running, or agents are off in it. Open Tabletist, turn agents on, then reconnect.";
+
+/// The socket's name in the directory.
+pub const SOCKET: &str = "mcp.sock";
+
+/// The key's name in the directory: new each time the server starts.
+pub const KEY: &str = "mcp.key";
+
+/// The longest path of a socket macOS takes (its `sun_path` holds 104
+/// bytes, the last of them the end of the string).
+pub const MAX_SOCKET_PATH: usize = 103;
+
+/// The longest line of a [`Hello`] or an [`Answer`]: a working directory
+/// fits many times over.
+pub const MAX_LINE: usize = 16 * 1024;
+
+/// How long either side waits for the other's line.
+pub const LINE_TIME: Duration = Duration::from_secs(5);
+
+/// What the relay says first.
+#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
+pub struct Hello {
+    /// What the app wrote to [`KEY`] when its server started.
+    pub key: String,
+    /// The relay's version, for the app's log.
+    pub version: String,
+    /// The directory the agent started the relay in: its last part names
+    /// the session's project for the person.
+    pub cwd: Option<String>,
+}
+
+/// What the app answers.
+#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
+#[serde(rename_all = "snake_case")]
+pub enum Answer {
+    /// The session begins with the next byte.
+    Ok,
+    /// It does not, and why, for the person: the relay passes it on to the
+    /// agent.
+    Refused(String),
+}
+
+/// A key: 244 random bits, as two version 4 UUIDs' worth of hex.
+pub fn new_key() -> String {
+    format!(
+        "{}{}",
+        uuid::Uuid::new_v4().simple(),
+        uuid::Uuid::new_v4().simple()
+    )
+}
+
+/// Whether `given` is `key`, in a time that does not tell how much of it
+/// was right.
+pub fn same_key(given: &str, key: &str) -> bool {
+    let (given, key) = (given.as_bytes(), key.as_bytes());
+    given.len() == key.len()
+        && given
+            .iter()
+            .zip(key)
+            .fold(0u8, |differ, (a, b)| differ | (a ^ b))
+            == 0
+}
+
+/// The name of a session's project: the last part of the directory the
+/// agent started in, without control characters, at most 64 characters.
+pub fn project(cwd: &str) -> Option<String> {
+    let last = std::path::Path::new(cwd).file_name()?.to_string_lossy();
+    let name: String = last.chars().filter(|c| !c.is_control()).take(64).collect();
+    (!name.is_empty()).then_some(name)
+}
+
+/// Reads one line of at most `max` bytes, a byte at a time: nothing past
+/// its end is taken from `from`, which then carries the session. `None` at
+/// the end of the stream before any byte, and an error for a line too long
+/// or cut short.
+pub async fn read_line<R: AsyncRead + Unpin>(
+    from: &mut R,
+    max: usize,
+) -> std::io::Result<Option<Vec<u8>>> {
+    let mut line = Vec::new();
+    loop {
+        let mut byte = [0u8; 1];
+        if from.read(&mut byte).await? == 0 {
+            return match line.is_empty() {
+                true => Ok(None),
+                false => Err(std::io::ErrorKind::UnexpectedEof.into()),
+            };
+        }
+        if byte[0] == b'\n' {
+            return Ok(Some(line));
+        }
+        if line.len() == max {
+            return Err(std::io::Error::new(
+                std::io::ErrorKind::InvalidData,
+                "the line is too long",
+            ));
+        }
+        line.push(byte[0]);
+    }
+}
+
+/// `value` as one line of JSON, with its end.
+pub fn line_of<T: Serialize>(value: &T) -> Vec<u8> {
+    let mut line = serde_json::to_vec(value).unwrap_or_default();
+    line.push(b'\n');
+    line
+}
+
 #[cfg(test)]
 mod tests {
     use super::*;
diff --git a/src/paths.rs b/src/paths.rs
index 41c46ea..ec94a2b 100644
--- a/src/paths.rs
+++ b/src/paths.rs
@@ -17,6 +17,10 @@ pub struct AppDirs {
     /// network, and Windows makes several system calls. None of that
     /// belongs in a frame.
     pub home: Option<PathBuf>,
+    /// The user's runtime directory, where there is one (Linux's
+    /// `/run/user/<id>`): what lives as long as the session, the agents'
+    /// endpoint. Elsewhere, and in tests, that is in `state`.
+    pub runtime: Option<PathBuf>,
 }
 
 impl AppDirs {
@@ -24,6 +28,7 @@ impl AppDirs {
     /// `~/.local/state/tabletist` on Linux.
     pub fn discover() -> Self {
         let home = BaseDirs::new().map(|dirs| dirs.home_dir().to_path_buf());
+        let runtime = runtime_dir();
         match ProjectDirs::from("dev", "tabletist", "Tabletist") {
             Some(project) => Self {
                 config: project.config_dir().to_path_buf(),
@@ -32,6 +37,7 @@ impl AppDirs {
                     .map(Path::to_path_buf)
                     .unwrap_or_else(|| project.data_local_dir().to_path_buf()),
                 home,
+                runtime,
             },
             None => Self::at(
                 &std::env::current_dir()
@@ -48,6 +54,7 @@ impl AppDirs {
             config: root.join("config"),
             state: root.join("state"),
             home: None,
+            runtime: None,
         }
     }
 
@@ -109,6 +116,38 @@ impl AppDirs {
         self.state.join("connections.moved")
     }
 
+    /// The log of every call agents made, kept for the days the settings
+    /// say (`[mcp] log_days`).
+    pub fn agents_log_file(&self) -> PathBuf {
+        self.state.join("agents.sqlite3")
+    }
+
+    /// Where `tabletist mcp` reaches the running app: the directory of its
+    /// socket, `mcp.sock`, and of the key a relay sends first, `mcp.key`.
+    /// Private to the user: made 0700 by the app before it listens.
+    /// `tabletist` in the runtime directory where there is one, else the
+    /// state directory. On macOS a socket's path may have 103 bytes, which
+    /// a long home directory can pass: then `.tabletist` in the home
+    /// directory, which is shorter by 45.
+    ///
+    /// The relay finds it from what an agent starts it with, which may be
+    /// no more than `HOME`, `PATH` and a few others: so nothing here comes
+    /// from `XDG_RUNTIME_DIR` or `TMPDIR`.
+    pub fn agents_dir(&self) -> PathBuf {
+        let dir = match &self.runtime {
+            Some(runtime) => runtime.join("tabletist"),
+            None => self.state.clone(),
+        };
+        #[cfg(target_os = "macos")]
+        if let Some(home) = &self.home
+            && dir.join(crate::endpoint::SOCKET).as_os_str().len()
+                > crate::endpoint::MAX_SOCKET_PATH
+        {
+            return home.join(".tabletist");
+        }
+        dir
+    }
+
     pub fn log_file(&self) -> PathBuf {
         self.state.join("tabletist.log")
     }
@@ -118,6 +157,23 @@ impl AppDirs {
     }
 }
 
+/// `/run/user/<id>`, where the system made it for this user: found by the
+/// user's id, which is the owner of the process's own `/proc` entry.
+#[cfg(target_os = "linux")]
+fn runtime_dir() -> Option<PathBuf> {
+    use std::os::unix::fs::MetadataExt;
+    let user = std::fs::metadata("/proc/self").ok()?.uid();
+    let dir = PathBuf::from(format!("/run/user/{user}"));
+    let found = std::fs::metadata(&dir).ok()?;
+    (found.is_dir() && found.uid() == user).then_some(dir)
+}
+
+/// None but on Linux.
+#[cfg(not(target_os = "linux"))]
+fn runtime_dir() -> Option<PathBuf> {
+    None
+}
+
 #[cfg(test)]
 mod tests {
     use super::*;
````

- [ ] **Step 4: Run the tests**

Run: `cargo test --locked --lib endpoint::`, then `cargo test --locked --lib paths::`

Expected: PASS.

```text
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 2440 filtered out
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 2439 filtered out
```

- [ ] **Step 5: Commit**

```bash
cargo fmt --all
cargo clippy --locked --workspace --all-targets -- -D warnings
git add src/endpoint.rs src/paths.rs src/lib.rs
git commit -m "Say where tabletist mcp finds the app, and what the two say first"
```

---

### Task 3: The keychain on macOS asks nothing for an agent

Before anything can reach the server (Task 4), an agent's read of a password must bring up no prompt on any system. Linux and Windows have that since the first part. On macOS `read_asking_nothing` tells the keychain to ask nothing while it reads (`SecKeychain::disable_user_interaction`, a guard that gives it back when dropped). A locked keychain, and an item that would ask whether Tabletist may read it, answer `errSecInteractionNotAllowed`, which `quiet_error` makes `Locked`. The gateway refuses either.

`security-framework` is the crate the keychain store is built on, at the version and with the features it builds: one line in `Cargo.lock`, and nothing built that was not.

**The test runs on macOS only**, in CI: on Linux and Windows the step's runs show the module's tests as they were. The macOS code was type-checked with clippy for `aarch64-apple-darwin` in an extract before this plan was written.

**Files:**
- Modify: `src/secrets.rs`, `Cargo.toml`, `Cargo.lock`

- [ ] **Step 1: Write the test**

````diff
diff --git a/Cargo.lock b/Cargo.lock
index c08036b..1f0794c 100644
--- a/Cargo.lock
+++ b/Cargo.lock
@@ -5695,6 +5695,7 @@ dependencies = [
  "objc2-foundation 0.3.2",
  "rfd",
  "secret-service",
+ "security-framework",
  "serde",
  "serde_json",
  "skrifa",
diff --git a/Cargo.toml b/Cargo.toml
index 929e716..8e98db6 100644
--- a/Cargo.toml
+++ b/Cargo.toml
@@ -119,6 +119,11 @@ skrifa = { version = "0.44", default-features = false, features = ["std"] }
 
 [target.'cfg(target_os = "macos")'.dependencies]
 apple-native-keyring-store = { version = "1.0.2", features = ["keychain"] }
+# The keychain store above is built on it. To tell the keychain to ask the
+# person nothing while it reads for an agent (`secrets::NativeStore::
+# read_quietly`), which the store has no call for. The version and the
+# features the store builds: nothing new is built.
+security-framework = "3.7"
 # The unified title bar the tabs share with the window buttons.
 objc2 = "0.6"
 objc2-foundation = { version = "0.3", default-features = false, features = ["std", "NSArray", "NSGeometry"] }
diff --git a/src/secrets.rs b/src/secrets.rs
index b6ca6ee..364064f 100644
--- a/src/secrets.rs
+++ b/src/secrets.rs
@@ -324,6 +324,21 @@ mod tests {
         );
     }
 
+    /// What the keychain answers a read it was told to ask nothing for,
+    /// where it would have had to: locked, which the gateway refuses.
+    #[cfg(target_os = "macos")]
+    #[test]
+    fn a_keychain_that_would_have_to_ask_is_locked_to_a_quiet_read() {
+        let would_ask = security_framework::base::Error::from_code(-25308);
+        let error = keyring_core::Error::PlatformFailure(Box::new(would_ask));
+        assert_eq!(quiet_error(&error), Some(SecretError::Locked));
+        // Any other failure is the store's, as a read says it.
+        let other = security_framework::base::Error::from_code(-25300);
+        let error = keyring_core::Error::PlatformFailure(Box::new(other));
+        assert_eq!(quiet_error(&error), None);
+        assert_eq!(quiet_error(&keyring_core::Error::NoEntry), None);
+    }
+
     #[tokio::test]
     async fn the_memory_keyring_stores_reads_and_deletes() {
         let keyring = Keyring::memory();
````

- [ ] **Step 2: The quiet read**

````diff
diff --git a/src/secrets.rs b/src/secrets.rs
index 364064f..1e2fae0 100644
--- a/src/secrets.rs
+++ b/src/secrets.rs
@@ -101,6 +101,29 @@ fn native_error(error: keyring_core::Error) -> SecretError {
 }
 
 impl NativeStore {
+    /// A read that the keychain is told to ask nothing for: for the
+    /// process, until it ends. The keyring thread is the only one here that
+    /// uses the keychain, and it does one job at a time, so no read that a
+    /// person is there for falls in between.
+    #[cfg(target_os = "macos")]
+    fn read_asking_nothing(&mut self, account: &str) -> Result<Option<String>, SecretError> {
+        use security_framework::os::macos::keychain::SecKeychain;
+        let entry = self.entry(account)?;
+        let _quiet =
+            SecKeychain::disable_user_interaction().map_err(|_| SecretError::Unavailable)?;
+        match entry.get_password() {
+            Ok(secret) => Ok(Some(secret)),
+            Err(keyring_core::Error::NoEntry) => Ok(None),
+            Err(error) => Err(quiet_error(&error).unwrap_or_else(|| native_error(error))),
+        }
+    }
+
+    /// Elsewhere the read asks nothing as it is, or was looked into first.
+    #[cfg(not(target_os = "macos"))]
+    fn read_asking_nothing(&mut self, account: &str) -> Result<Option<String>, SecretError> {
+        self.read(account)
+    }
+
     fn entry(&mut self, account: &str) -> Result<keyring_core::Entry, SecretError> {
         if self.store.is_none() {
             #[cfg(target_os = "linux")]
@@ -134,9 +157,9 @@ impl SecretStore for NativeStore {
     /// that was not found is not read either: should the search look for
     /// it otherwise than the keyring crate keeps it, the cost is a
     /// password not found, never a prompt. Windows asks nothing of a read.
-    /// On macOS a locked keychain is asked to be unlocked by the system,
-    /// and no call of the keyring crate says not to: that is not kept from
-    /// happening here.
+    /// On macOS the keychain is told to ask nothing while it reads: a
+    /// locked one, and an item that would ask whether Tabletist may read
+    /// it, answer that they would have to, and are not read.
     fn read_quietly(&mut self, account: &str) -> Result<Option<String>, SecretError> {
         #[cfg(target_os = "linux")]
         match linux_item(account)? {
@@ -144,7 +167,7 @@ impl SecretStore for NativeStore {
             Item::Locked => return Err(SecretError::Locked),
             Item::Missing => return Ok(None),
         }
-        self.read(account)
+        self.read_asking_nothing(account)
     }
 
     fn write(&mut self, account: &str, secret: &str) -> Result<(), SecretError> {
@@ -161,6 +184,24 @@ impl SecretStore for NativeStore {
     }
 }
 
+/// What a quiet read's error says, where it is the keychain's answer that
+/// it would have had to ask: `errSecInteractionNotAllowed`. That is a
+/// locked keychain, or an item that would ask whether Tabletist may read it
+/// (after an update the system does not know for the same app). Either is
+/// `Locked` here: the keychain is locked to agents until the person has
+/// answered it once, from the app.
+#[cfg(target_os = "macos")]
+fn quiet_error(error: &keyring_core::Error) -> Option<SecretError> {
+    const INTERACTION_NOT_ALLOWED: i32 = -25308;
+    let keyring_core::Error::PlatformFailure(inner) = error else {
+        return None;
+    };
+    inner
+        .downcast_ref::<security_framework::base::Error>()
+        .filter(|inner| inner.code() == INTERACTION_NOT_ALLOWED)
+        .map(|_| SecretError::Locked)
+}
+
 /// How the Secret Service holds an account's item.
 #[cfg(target_os = "linux")]
 #[derive(Debug, Clone, Copy, PartialEq, Eq)]
````

- [ ] **Step 3: Run the tests**

Run: `cargo test --locked --lib secrets::`

Expected: PASS, the module's tests as they were: on Linux and Windows the new one is not built. (The one ignored is the native keyring's, which needs a desktop.)

```text
test result: ok. 6 passed; 0 failed; 1 ignored; 0 measured; 2437 filtered out
```

On macOS the count has one more, the new test, which CI runs.

- [ ] **Step 4: Commit**

```bash
cargo fmt --all
cargo clippy --locked --workspace --all-targets -- -D warnings
git add src/secrets.rs Cargo.toml Cargo.lock
git commit -m "Tell the keychain on macOS to ask nothing while it reads for an agent"
```

---

### Task 4: The server listens for `tabletist mcp`

`src/backend/agents/listen.rs`, new, for Unix:

- **`listen`** makes the directory 0700 (one whose mode the user cannot change is not theirs, and is refused), takes a socket that answers for another Tabletist's and one that does not for an ended one's, writes a new key, and only then makes the socket, 0600. **`Files`** removes the socket and the key when the server stops, each only while it is the one this start made.
- **`accept`** takes a connection once one of 64 places among the greetings is free. **`greet`** reads the relay's line for five seconds at most, checks the key, counts the session against 32 once it has the key, answers, and hands the stream to `Server::serve`. A stop ends what comes before the session, with the answer that the server is not running. Nothing the relay sent goes into the app's log.

`src/backend/agents.rs`: `Start` gets the directory to listen in (none for the gateway's tests, and on Windows, where nothing listens yet), and looks for another Tabletist there before it opens the log. `LogDays` changes the log's days without a restart.

**Files:**
- Create: `src/backend/agents/listen.rs`
- Modify: `src/backend/agents.rs`, `Cargo.toml`

- [ ] **Step 1: Write the failing tests**

````diff
diff --git a/Cargo.toml b/Cargo.toml
index 8e98db6..dc379de 100644
--- a/Cargo.toml
+++ b/Cargo.toml
@@ -77,8 +77,10 @@ tabletist-mcp = { path = "crates/tabletist-mcp" }
 # over. In the tree through tabletist-mcp, which hands one to every call.
 tokio-util = "0.7"
 # The backend runtime: one OS thread running tokio, where every database
-# call happens (src/backend.rs).
-tokio = { version = "1", features = ["rt-multi-thread", "sync", "macros", "time"] }
+# call happens (src/backend.rs). `net` and `io-util` for the socket agents
+# reach the app through (src/backend/agents/listen.rs), which tabletist-db
+# turns on already: nothing new is built.
+tokio = { version = "1", features = ["rt-multi-thread", "sync", "macros", "time", "net", "io-util"] }
 # Native file dialogs (portal on Linux), built on the UI thread and awaited
 # on the backend runtime.
 rfd = "0.17"
diff --git a/src/backend/agents.rs b/src/backend/agents.rs
index 4c8aec8..aa290de 100644
--- a/src/backend/agents.rs
+++ b/src/backend/agents.rs
@@ -1327,6 +1327,7 @@ mod tests {
         backend.send(Command::Agents(AgentsCommand::Start {
             log: path.clone(),
             log_days: 30,
+            endpoint: None,
         }));
         assert_eq!(
             match backend.wait(WAIT) {
@@ -1363,6 +1364,7 @@ mod tests {
         backend.send(Command::Agents(AgentsCommand::Start {
             log: dir.path().join("no-such-folder").join("agents.sqlite3"),
             log_days: 30,
+            endpoint: None,
         }));
         match backend.wait(WAIT) {
             Some(Event::Agents(AgentsEvent::Failed(said))) => {
@@ -1375,6 +1377,33 @@ mod tests {
         }
     }
 
+    /// Another Tabletist serves agents: this one's server does not start,
+    /// and does not open the log the other has open, which one of another
+    /// version would set aside.
+    #[cfg(unix)]
+    #[test]
+    fn a_start_where_another_tabletist_serves_agents_leaves_its_log_alone() {
+        let dir = tempfile::tempdir().unwrap();
+        let at = dir.path().join("run");
+        std::fs::create_dir(&at).unwrap();
+        let _another =
+            std::os::unix::net::UnixListener::bind(at.join(crate::endpoint::SOCKET)).unwrap();
+        let path = dir.path().join("agents.sqlite3");
+        let mut backend = Backend::start_with(Waker::default(), Keyring::memory());
+        backend.send(Command::Agents(AgentsCommand::Start {
+            log: path.clone(),
+            log_days: 30,
+            endpoint: Some(at),
+        }));
+        match backend.wait(WAIT) {
+            Some(Event::Agents(AgentsEvent::Failed(said))) => {
+                assert!(said.contains("another Tabletist"), "{said}");
+            }
+            other => panic!("{other:?}"),
+        }
+        assert!(!path.exists());
+    }
+
     #[test]
     fn starts_and_stops_one_after_another_leave_the_last_one_standing() {
         let dir = tempfile::tempdir().unwrap();
@@ -1384,6 +1413,7 @@ mod tests {
             Command::Agents(AgentsCommand::Start {
                 log: path.clone(),
                 log_days: 30,
+                endpoint: None,
             })
         };
         backend.send(start());
diff --git a/src/backend/agents/listen.rs b/src/backend/agents/listen.rs
new file mode 100644
index 0000000..32106ce
--- /dev/null
+++ b/src/backend/agents/listen.rs
@@ -0,0 +1,216 @@
+//! The server's endpoint: the socket `tabletist mcp` connects to, and the
+//! key it must send first.
+
+#[cfg(test)]
+mod tests {
+    use super::*;
+    use crate::secrets::Keyring;
+
+    fn server(dir: &Path) -> Server<AppGateway> {
+        let log = tabletist_mcp::Log::open(&dir.join("agents.sqlite3"))
+            .unwrap()
+            .log;
+        Server::new(AppGateway::new(Keyring::memory()), log, "test")
+    }
+
+    /// Says `line` to the endpoint in `dir` as a relay does, and reads the
+    /// app's answer.
+    async fn say(dir: &Path, line: &[u8]) -> (UnixStream, Answer) {
+        let mut stream = UnixStream::connect(dir.join(endpoint::SOCKET))
+            .await
+            .unwrap();
+        stream.write_all(line).await.unwrap();
+        let answer = endpoint::read_line(&mut stream, endpoint::MAX_LINE)
+            .await
+            .unwrap()
+            .unwrap();
+        (stream, serde_json::from_slice(&answer).unwrap())
+    }
+
+    fn hello(key: &str) -> Vec<u8> {
+        endpoint::line_of(&Hello {
+            key: key.into(),
+            version: "test".into(),
+            cwd: Some("/home/ada/src/bookshop".into()),
+        })
+    }
+
+    fn mode(path: &Path) -> u32 {
+        std::fs::symlink_metadata(path)
+            .unwrap()
+            .permissions()
+            .mode()
+            & 0o777
+    }
+
+    #[tokio::test]
+    async fn a_relay_with_the_key_is_served_and_one_without_it_is_not() {
+        let dir = tempfile::tempdir().unwrap();
+        let at = dir.path().join("run");
+        let server = server(dir.path());
+        let listening = listen(&at, server.clone()).unwrap();
+        // Private to the user, the folder and what is in it.
+        assert_eq!(mode(&at), 0o700);
+        assert_eq!(mode(&at.join(endpoint::SOCKET)), 0o600);
+        assert_eq!(mode(&at.join(endpoint::KEY)), 0o600);
+        let key = std::fs::read_to_string(at.join(endpoint::KEY)).unwrap();
+        let (stream, answer) = say(&at, &hello(&key)).await;
+        assert_eq!(answer, Answer::Ok);
+        // From then on the stream is the session's, named for its project.
+        let (read, mut write) = tokio::io::split(stream);
+        let initialize = serde_json::json!({
+            "jsonrpc": "2.0",
+            "id": 0,
+            "method": "initialize",
+            "params": {
+                "protocolVersion": "2025-11-25",
+                "capabilities": {},
+                "clientInfo": { "name": "claude-code", "version": "1" },
+            },
+        });
+        write
+            .write_all(&endpoint::line_of(&initialize))
+            .await
+            .unwrap();
+        let mut lines = tokio::io::AsyncBufReadExt::lines(tokio::io::BufReader::new(read));
+        let answered: serde_json::Value =
+            serde_json::from_str(&lines.next_line().await.unwrap().unwrap()).unwrap();
+        assert!(answered["result"].is_object(), "{answered}");
+        let sessions = server.open_sessions();
+        assert_eq!(sessions.len(), 1);
+        assert_eq!(sessions[0].project.as_deref(), Some("bookshop"));
+        // A key that is not the server's, and a line that is no hello.
+        for line in [hello("not-the-key"), b"hello\n".to_vec()] {
+            let (_, answer) = say(&at, &line).await;
+            assert!(matches!(answer, Answer::Refused(_)), "{answer:?}");
+        }
+        assert_eq!(server.open_sessions().len(), 1);
+        // Stopped: the session ends, and the socket and the key are gone.
+        server.stop();
+        drop(listening);
+        assert_eq!(lines.next_line().await.unwrap(), None);
+        assert!(!at.join(endpoint::SOCKET).exists());
+        assert!(!at.join(endpoint::KEY).exists());
+    }
+
+    #[tokio::test]
+    async fn a_socket_that_answers_is_another_tabletists_and_one_that_does_not_is_old() {
+        let dir = tempfile::tempdir().unwrap();
+        let at = dir.path().join("run");
+        let first = listen(&at, server(dir.path())).unwrap();
+        let said = listen(&at, server(dir.path())).err().unwrap();
+        assert!(said.contains("another Tabletist"), "{said}");
+        let key = std::fs::read_to_string(at.join(endpoint::KEY)).unwrap();
+        // The first is as it was.
+        let (_, answer) = say(&at, &hello(&key)).await;
+        assert_eq!(answer, Answer::Ok);
+        drop(first);
+        // A socket left by a Tabletist that ended: nothing answers it.
+        drop(std::os::unix::net::UnixListener::bind(at.join(endpoint::SOCKET)).unwrap());
+        let _second = listen(&at, server(dir.path())).unwrap();
+        let key = std::fs::read_to_string(at.join(endpoint::KEY)).unwrap();
+        let (_, answer) = say(&at, &hello(&key)).await;
+        assert_eq!(answer, Answer::Ok);
+    }
+
+    #[tokio::test]
+    async fn what_is_not_a_socket_is_not_removed_nor_what_another_put_in_its_place() {
+        let dir = tempfile::tempdir().unwrap();
+        let at = dir.path().join("run");
+        std::fs::create_dir_all(&at).unwrap();
+        std::fs::write(at.join(endpoint::SOCKET), "mine").unwrap();
+        let said = listen(&at, server(dir.path())).err().unwrap();
+        assert!(said.contains("in the way"), "{said}");
+        assert_eq!(
+            std::fs::read_to_string(at.join(endpoint::SOCKET)).unwrap(),
+            "mine"
+        );
+        std::fs::remove_file(at.join(endpoint::SOCKET)).unwrap();
+        let listening = listen(&at, server(dir.path())).unwrap();
+        // Another socket where it was: the stop leaves it there.
+        std::fs::remove_file(at.join(endpoint::SOCKET)).unwrap();
+        let other = std::os::unix::net::UnixListener::bind(at.join(endpoint::SOCKET)).unwrap();
+        drop(listening);
+        assert!(at.join(endpoint::SOCKET).exists());
+        drop(other);
+    }
+
+    #[tokio::test]
+    async fn connections_that_say_nothing_take_no_agents_place() {
+        let dir = tempfile::tempdir().unwrap();
+        let at = dir.path().join("run");
+        let _listening = listen(&at, server(dir.path())).unwrap();
+        let mut silent = Vec::new();
+        for _ in 0..MAX_SESSIONS + 8 {
+            silent.push(
+                UnixStream::connect(at.join(endpoint::SOCKET))
+                    .await
+                    .unwrap(),
+            );
+        }
+        let key = std::fs::read_to_string(at.join(endpoint::KEY)).unwrap();
+        let (_, answer) = say(&at, &hello(&key)).await;
+        assert_eq!(answer, Answer::Ok);
+    }
+
+    #[tokio::test]
+    async fn a_relay_greeting_as_the_server_stops_is_told_it_is_not_running() {
+        let dir = tempfile::tempdir().unwrap();
+        let at = dir.path().join("run");
+        let listening = listen(&at, server(dir.path())).unwrap();
+        let mut stream = UnixStream::connect(at.join(endpoint::SOCKET))
+            .await
+            .unwrap();
+        // Taken, and waiting for its hello.
+        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
+        drop(listening);
+        let line = endpoint::read_line(&mut stream, endpoint::MAX_LINE)
+            .await
+            .unwrap()
+            .unwrap();
+        assert_eq!(
+            serde_json::from_slice::<Answer>(&line).unwrap(),
+            Answer::Refused(endpoint::NOT_RUNNING.into())
+        );
+    }
+
+    #[tokio::test]
+    async fn a_key_left_by_an_ended_start_is_replaced_before_anyone_can_connect() {
+        let dir = tempfile::tempdir().unwrap();
+        let at = dir.path().join("run");
+        std::fs::create_dir_all(&at).unwrap();
+        std::fs::write(at.join(endpoint::KEY), "an ended start's").unwrap();
+        // A socket that is in the way: nothing listens, and the key that
+        // is there stays the one that was.
+        std::fs::create_dir(at.join(endpoint::SOCKET)).unwrap();
+        assert!(listen(&at, server(dir.path())).is_err());
+        assert_eq!(
+            std::fs::read_to_string(at.join(endpoint::KEY)).unwrap(),
+            "an ended start's"
+        );
+        std::fs::remove_dir(at.join(endpoint::SOCKET)).unwrap();
+        let listening = listen(&at, server(dir.path())).unwrap();
+        let key = std::fs::read_to_string(at.join(endpoint::KEY)).unwrap();
+        assert_ne!(key, "an ended start's");
+        // A key a later start wrote is that start's: the stop leaves it.
+        std::fs::write(at.join(endpoint::KEY), "a later start's").unwrap();
+        drop(listening);
+        assert_eq!(
+            std::fs::read_to_string(at.join(endpoint::KEY)).unwrap(),
+            "a later start's"
+        );
+    }
+
+    #[test]
+    fn a_session_past_the_most_is_counted_and_refused() {
+        let open = Arc::new(AtomicUsize::new(0));
+        let held: Vec<Held> = (0..MAX_SESSIONS).map(|_| Held::take(&open)).collect();
+        assert!(held.iter().all(|held| held.1));
+        let one_more = Held::take(&open);
+        assert!(!one_more.1);
+        drop(one_more);
+        drop(held);
+        assert_eq!(open.load(Ordering::SeqCst), 0);
+        assert!(Held::take(&open).1);
+    }
+}
````

- [ ] **Step 2: Run them to see that they do not build**

Run: `cargo test --locked --lib backend::agents`

Expected: it does not build. The start the tests send names what is not there yet. (The endpoint's own tests are in a module `agents.rs` declares in Step 3.)

```text
error[E0559]: variant `agents::AgentsCommand::Start` has no field named `endpoint`
error: could not compile `tabletist` (lib test) due to 4 previous errors
```

- [ ] **Step 3: The endpoint**

````diff
diff --git a/src/backend/agents.rs b/src/backend/agents.rs
index aa290de..a99ed03 100644
--- a/src/backend/agents.rs
+++ b/src/backend/agents.rs
@@ -8,6 +8,7 @@
 
 use std::collections::HashMap;
 use std::path::PathBuf;
+use std::sync::atomic::AtomicU32;
 use std::sync::{Arc, Mutex};
 use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
 
@@ -26,6 +27,9 @@ use crate::env::Platform;
 use crate::model::SecretKind;
 use crate::secrets::{Keyring, password_account, ssh_account};
 
+#[cfg(unix)]
+mod listen;
+
 /// A connection agents may use, as the server is given it: what opens it,
 /// and what an agent calls it. Its password is not here: the gateway reads
 /// it from the keyring when it opens the connection, as a tab does.
@@ -123,9 +127,17 @@ pub enum AgentsCommand {
         host_keys: HostKeys,
     },
     /// Starts the server, with its log at `log`, which keeps calls for
-    /// `log_days` days. Answered with [`AgentsEvent::Started`] or
+    /// `log_days` days, listening in `endpoint` for `tabletist mcp`
+    /// (`AppDirs::agents_dir`). Without one nothing reaches it, which the
+    /// tests of the gateway want. Answered with [`AgentsEvent::Started`] or
     /// [`AgentsEvent::Failed`].
-    Start { log: PathBuf, log_days: u32 },
+    Start {
+        log: PathBuf,
+        log_days: u32,
+        endpoint: Option<PathBuf>,
+    },
+    /// The log keeps calls for these days from now on.
+    LogDays(u32),
     /// Stops it: every session ends, and the connections are closed.
     /// Answered with [`AgentsEvent::Stopped`].
     Stop,
@@ -138,8 +150,8 @@ pub enum AgentsEvent {
     Started {
         kept_aside: Option<PathBuf>,
     },
-    /// It could not start: its log could not be opened. Why, for the
-    /// person.
+    /// It could not start: its log could not be opened, or it could not
+    /// listen. Why, for the person.
     Failed(String),
     Stopped,
 }
@@ -162,6 +174,8 @@ const CONNECT_TIME: Duration = Duration::from_secs(20);
 pub(super) struct Running {
     server: tabletist_mcp::Server<AppGateway>,
     chores: tokio::task::JoinHandle<()>,
+    #[cfg(unix)]
+    listening: Option<listen::Listening>,
 }
 
 /// The server's part of the worker: the gateway, whatever the server is
@@ -175,6 +189,8 @@ pub(super) struct Agents {
     /// Held by a start from the opening of its log to its end: two starts
     /// must not open the one file at once.
     starting: Arc<tokio::sync::Mutex<()>>,
+    /// The days the log keeps a call, which its chores read each time.
+    log_days: Arc<AtomicU32>,
 }
 
 impl Agents {
@@ -184,6 +200,7 @@ impl Agents {
             running: Arc::default(),
             turn: Arc::default(),
             starting: Arc::default(),
+            log_days: Arc::default(),
         }
     }
 
@@ -193,7 +210,18 @@ impl Agents {
                 connections,
                 host_keys,
             } => self.gateway.set_policy(connections, host_keys),
-            AgentsCommand::Start { log, log_days } => {
+            AgentsCommand::LogDays(days) => {
+                self.log_days
+                    .store(days, std::sync::atomic::Ordering::SeqCst);
+            }
+            AgentsCommand::Start {
+                log,
+                log_days,
+                endpoint,
+            } => {
+                self.log_days
+                    .store(log_days, std::sync::atomic::Ordering::SeqCst);
+                let log_days = Arc::clone(&self.log_days);
                 let turn = {
                     let mut turn = lock(&self.turn);
                     *turn += 1;
@@ -210,6 +238,16 @@ impl Agents {
                     if overtaken() {
                         return;
                     }
+                    // Before the log: another Tabletist's server has it open,
+                    // and one of another version would set it aside.
+                    if lock(&running).is_none()
+                        && endpoint.as_deref().is_some_and(served_by_another)
+                    {
+                        log::error!("the agent server did not start: another one serves agents");
+                        let said = "agents could not reach it: another Tabletist is serving agents already";
+                        outbox.emit(super::Event::Agents(AgentsEvent::Failed(said.into())));
+                        return;
+                    }
                     let opened = tokio::task::spawn_blocking(move || Log::open(&log)).await;
                     let failed = match opened {
                         Ok(Ok(opened)) => Ok(opened),
@@ -247,8 +285,26 @@ impl Agents {
                     if let Some(before) = running.take() {
                         before.stop();
                     }
+                    // Once the server before it has let its socket go.
+                    let listening = match listening(endpoint.as_deref(), &server) {
+                        Ok(listening) => listening,
+                        Err(error) => {
+                            log::error!("the agent server did not start: {error}");
+                            server.stop();
+                            let said = format!("agents could not reach it: {error}");
+                            outbox.emit(super::Event::Agents(AgentsEvent::Failed(said)));
+                            return;
+                        }
+                    };
+                    #[cfg(not(unix))]
+                    let () = listening;
                     let chores = tokio::spawn(chores(opened.log, gateway, log_days));
-                    *running = Some(Running { server, chores });
+                    *running = Some(Running {
+                        server,
+                        chores,
+                        #[cfg(unix)]
+                        listening,
+                    });
                     log::info!("the agent server started");
                     // Said under the lock a stop takes, so that no Stopped
                     // is said before it.
@@ -272,20 +328,60 @@ impl Agents {
 }
 
 impl Running {
+    /// Stops the server, then the endpoint, whose socket and key go with
+    /// it.
     fn stop(self) {
         self.server.stop();
         self.chores.abort();
+        #[cfg(unix)]
+        drop(self.listening);
+    }
+}
+
+/// Listens in `endpoint` for `server`, where there is one.
+#[cfg(unix)]
+fn listening(
+    endpoint: Option<&std::path::Path>,
+    server: &tabletist_mcp::Server<AppGateway>,
+) -> Result<Option<listen::Listening>, String> {
+    endpoint
+        .map(|dir| listen::listen(dir, server.clone()))
+        .transpose()
+}
+
+/// Whether another Tabletist serves agents in `dir`.
+#[cfg(unix)]
+fn served_by_another(dir: &std::path::Path) -> bool {
+    listen::served_by_another(dir)
+}
+
+/// Nothing serves agents here yet.
+#[cfg(not(unix))]
+fn served_by_another(_dir: &std::path::Path) -> bool {
+    false
+}
+
+/// Nothing listens here yet (`endpoint::AVAILABLE`).
+#[cfg(not(unix))]
+fn listening(
+    endpoint: Option<&std::path::Path>,
+    _server: &tabletist_mcp::Server<AppGateway>,
+) -> Result<(), String> {
+    match endpoint {
+        Some(_) => Err("agents cannot reach Tabletist on this system yet".into()),
+        None => Ok(()),
     }
 }
 
 /// What the server does from time to time while it runs: closes the
 /// connections no agent has used for a while, and forgets calls older than
 /// the log keeps. The first time as it starts.
-async fn chores(log: Log, gateway: AppGateway, log_days: u32) {
-    let keep = Duration::from_secs(u64::from(log_days.max(1)) * 24 * 60 * 60);
+async fn chores(log: Log, gateway: AppGateway, log_days: Arc<AtomicU32>) {
     let mut forgot: Option<Instant> = None;
     loop {
         if forgot.is_none_or(|at| at.elapsed() >= RETENTION_EVERY) {
+            let days = log_days.load(std::sync::atomic::Ordering::SeqCst).max(1);
+            let keep = Duration::from_secs(u64::from(days) * 24 * 60 * 60);
             let before = SystemTime::now()
                 .checked_sub(keep)
                 .and_then(|before| before.duration_since(UNIX_EPOCH).ok())
diff --git a/src/backend/agents/listen.rs b/src/backend/agents/listen.rs
index 32106ce..79dd827 100644
--- a/src/backend/agents/listen.rs
+++ b/src/backend/agents/listen.rs
@@ -1,6 +1,255 @@
 //! The server's endpoint: the socket `tabletist mcp` connects to, and the
 //! key it must send first.
 
+use std::os::unix::fs::{FileTypeExt, MetadataExt, PermissionsExt};
+use std::path::{Path, PathBuf};
+use std::sync::Arc;
+use std::sync::atomic::{AtomicUsize, Ordering};
+
+use tabletist_mcp::{Origin, Server, Transport};
+use tokio::io::AsyncWriteExt;
+use tokio::net::{UnixListener, UnixStream};
+use tokio::sync::{OwnedSemaphorePermit, Semaphore};
+use tokio_util::sync::CancellationToken;
+
+use super::AppGateway;
+use crate::endpoint::{self, Answer, Hello};
+
+/// The most agents connected at once. One more is refused: a program of
+/// the user's that opens sessions without end runs into it, and the
+/// connections to the databases do not.
+const MAX_SESSIONS: usize = 32;
+
+/// The most connections that have not said their [`Hello`] yet. The next
+/// waits in the system's queue, and holds nothing of the app's: connections
+/// that never say anything cannot use up its descriptors, nor the sessions
+/// of the agents that have the key.
+const MAX_GREETINGS: usize = 64;
+
+/// The endpoint while the server listens on it. Dropped, it listens no
+/// more, and its socket and key are gone.
+pub(super) struct Listening {
+    stop: CancellationToken,
+    _files: Files,
+}
+
+impl Drop for Listening {
+    fn drop(&mut self) {
+        self.stop.cancel();
+    }
+}
+
+/// The socket and the key the server made, removed when it stops: each
+/// only while it is the one it made, which a later start may have put
+/// another in the place of.
+struct Files {
+    socket: Option<(PathBuf, (u64, u64))>,
+    key: (PathBuf, String),
+}
+
+impl Drop for Files {
+    fn drop(&mut self) {
+        if let Some((socket, made)) = &self.socket
+            && std::fs::symlink_metadata(socket).is_ok_and(|now| (now.dev(), now.ino()) == *made)
+        {
+            let _ = std::fs::remove_file(socket);
+        }
+        let (file, key) = &self.key;
+        if std::fs::read_to_string(file).is_ok_and(|there| there == *key) {
+            let _ = std::fs::remove_file(file);
+        }
+    }
+}
+
+/// Whether a socket in `dir` answers: another Tabletist is serving its
+/// agents there.
+pub(super) fn served_by_another(dir: &Path) -> bool {
+    let socket = dir.join(endpoint::SOCKET);
+    std::fs::symlink_metadata(&socket).is_ok_and(|found| found.file_type().is_socket())
+        && std::os::unix::net::UnixStream::connect(&socket).is_ok()
+}
+
+/// Listens in `dir` for `server`: makes the directory private, a new key,
+/// and the socket. Why it could not, for the person.
+pub(super) fn listen(dir: &Path, server: Server<AppGateway>) -> Result<Listening, String> {
+    let shown = dir.display();
+    crate::util::create_private_dir(dir)
+        .map_err(|error| format!("could not make the folder {shown}: {error}"))?;
+    // Only its owner can change a folder's mode: one that someone else made
+    // there first is refused, not used.
+    std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o700))
+        .map_err(|error| format!("the folder {shown} is not yours to use: {error}"))?;
+    let socket = dir.join(endpoint::SOCKET);
+    match std::fs::symlink_metadata(&socket) {
+        Ok(found) if found.file_type().is_socket() => {
+            // A socket that answers is another Tabletist's, serving its
+            // agents. One that does not was left by a Tabletist that ended.
+            if served_by_another(dir) {
+                return Err("another Tabletist is serving agents already".into());
+            }
+            std::fs::remove_file(&socket).map_err(|error| {
+                format!("could not remove an old {}: {error}", socket.display())
+            })?;
+        }
+        Ok(_) => return Err(format!("{} is in the way", socket.display())),
+        Err(_) => {}
+    }
+    // The key before the socket: a relay that connects the moment it can
+    // reads the key of this start, not one an ended start left.
+    let key = endpoint::new_key();
+    let key_file = dir.join(endpoint::KEY);
+    let mut files = Files {
+        socket: None,
+        key: (key_file.clone(), key.clone()),
+    };
+    crate::util::write_atomic(&key_file, key.as_bytes())
+        .map_err(|error| format!("could not write {}: {error}", key_file.display()))?;
+    let listener = UnixListener::bind(&socket)
+        .map_err(|error| format!("could not listen at {}: {error}", socket.display()))?;
+    let made = std::fs::symlink_metadata(&socket)
+        .map(|made| (made.dev(), made.ino()))
+        .map_err(|error| format!("could not look at {}: {error}", socket.display()))?;
+    files.socket = Some((socket.clone(), made));
+    std::fs::set_permissions(&socket, std::fs::Permissions::from_mode(0o600))
+        .map_err(|error| format!("could not make {} private: {error}", socket.display()))?;
+    let stop = CancellationToken::new();
+    tokio::spawn(accept(listener, Arc::new(key), server, stop.clone()));
+    Ok(Listening {
+        stop,
+        _files: files,
+    })
+}
+
+/// Takes each relay that connects, until `stop`.
+async fn accept(
+    listener: UnixListener,
+    key: Arc<String>,
+    server: Server<AppGateway>,
+    stop: CancellationToken,
+) {
+    let open = Arc::new(AtomicUsize::new(0));
+    let greetings = Arc::new(Semaphore::new(MAX_GREETINGS));
+    loop {
+        // A place among the greetings before the connection is taken.
+        let greeting = tokio::select! {
+            biased;
+            () = stop.cancelled() => return,
+            permit = Arc::clone(&greetings).acquire_owned() => match permit {
+                Ok(permit) => permit,
+                Err(_) => return,
+            },
+        };
+        let stream = tokio::select! {
+            biased;
+            () = stop.cancelled() => return,
+            accepted = listener.accept() => match accepted {
+                Ok((stream, _)) => stream,
+                Err(error) => {
+                    log::warn!("the agent server could not take a connection: {error}");
+                    // Out of descriptors, most likely: not again at once.
+                    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
+                    continue;
+                }
+            },
+        };
+        let key = Arc::clone(&key);
+        let server = server.clone();
+        let open = Arc::clone(&open);
+        let stop = stop.clone();
+        tokio::spawn(async move {
+            greet(stream, &key, &server, &open, greeting, &stop).await;
+        });
+    }
+}
+
+/// One of the sessions counted against [`MAX_SESSIONS`], while it is open.
+struct Held(Arc<AtomicUsize>, bool);
+
+impl Held {
+    fn take(open: &Arc<AtomicUsize>) -> Self {
+        let before = open.fetch_add(1, Ordering::SeqCst);
+        Self(Arc::clone(open), before < MAX_SESSIONS)
+    }
+}
+
+impl Drop for Held {
+    fn drop(&mut self) {
+        self.0.fetch_sub(1, Ordering::SeqCst);
+    }
+}
+
+/// Reads the relay's [`Hello`], answers it, and serves the session. A stop
+/// ends what comes before the session; the session is ended by the
+/// server's own stop, which lets its calls end and go into the log. Nothing
+/// the relay sent goes into the app's log: any program of the user's can
+/// send it.
+async fn greet(
+    mut stream: UnixStream,
+    key: &str,
+    server: &Server<AppGateway>,
+    open: &Arc<AtomicUsize>,
+    greeting: OwnedSemaphorePermit,
+    stop: &CancellationToken,
+) {
+    let refuse = async |stream: &mut UnixStream, said: &str| {
+        let line = endpoint::line_of(&Answer::Refused(said.to_owned()));
+        let _ = stream.write_all(&line).await;
+        let _ = stream.shutdown().await;
+    };
+    let read = tokio::select! {
+        biased;
+        () = stop.cancelled() => None,
+        read = tokio::time::timeout(
+            endpoint::LINE_TIME,
+            endpoint::read_line(&mut stream, endpoint::MAX_LINE),
+        ) => Some(read),
+    };
+    drop(greeting);
+    let Some(read) = read else {
+        return refuse(&mut stream, endpoint::NOT_RUNNING).await;
+    };
+    let hello = match read {
+        Ok(Ok(Some(line))) => serde_json::from_slice::<Hello>(&line).ok(),
+        _ => None,
+    };
+    let Some(hello) = hello else {
+        log::warn!("a connection to the agent server said nothing it could read");
+        return refuse(&mut stream, "Tabletist could not read what the relay sent.").await;
+    };
+    if !endpoint::same_key(&hello.key, key) {
+        log::warn!("a relay sent a key that is not the agent server's");
+        return refuse(
+            &mut stream,
+            "The relay's key is not the one Tabletist made: start the agent's MCP server again.",
+        )
+        .await;
+    }
+    // Counted once it has the key: who has none takes no agent's place.
+    let held = Held::take(open);
+    if !held.1 {
+        log::warn!("an agent was refused: {MAX_SESSIONS} are connected");
+        return refuse(&mut stream, "Too many agents are connected to Tabletist.").await;
+    }
+    if stop.is_cancelled() {
+        return refuse(&mut stream, endpoint::NOT_RUNNING).await;
+    }
+    if stream
+        .write_all(&endpoint::line_of(&Answer::Ok))
+        .await
+        .is_err()
+    {
+        return;
+    }
+    let origin = Origin {
+        transport: Transport::Stdio,
+        project: hello.cwd.as_deref().and_then(endpoint::project),
+    };
+    // Boxed: the session's future is deep, too deep for the compiler to
+    // lay out inside this one.
+    Box::pin(server.serve(stream, origin)).await;
+    drop(held);
+}
+
 #[cfg(test)]
 mod tests {
     use super::*;
````

- [ ] **Step 4: Run the tests**

Run: `cargo test --locked --lib backend::agents`

Expected: PASS, in about two seconds: the gateway's tests as they were, the start where another Tabletist serves, and the endpoint's seven.

```text
test result: ok. 27 passed; 0 failed; 0 ignored; 0 measured; 2425 filtered out
```

- [ ] **Step 5: Commit**

```bash
cargo fmt --all
cargo clippy --locked --workspace --all-targets -- -D warnings
git add src/backend Cargo.toml
git commit -m "Listen for tabletist mcp on a socket of the user's own, behind a key"
```

---

### Task 5: `tabletist mcp`

`src/relay.rs`, new: the relay. It tries the socket for two seconds, reads the key, says its line, and on `"ok"` copies stdin to the app and the app to stdout. The agent's end of stdin ends what it sends; the app's end ends it. When it cannot reach the app, or the app refuses it, it answers the agent's first request with a JSON-RPC error, writes the same to stderr, and exits 1.

`src/entrypoint.rs`: `mcp` is a subcommand, with a hidden `--endpoint` for tests. `run` parses the command line and branches to the relay before the logger, the settings and the window; `main` takes what it parsed.

`tests/cli.rs` runs the built binary against the app's backend, serving a Bookshop file whose path holds a marker: an agent lists the connections and reads, the database's error comes back, the log has the three calls under the project's name, and nothing on stdout or stderr holds the marker. Then a wrong key, and a server stopped. And with no app at all.

**Files:**
- Create: `src/relay.rs`
- Modify: `src/entrypoint.rs`, `src/lib.rs`, `tests/cli.rs`, `Cargo.toml`

- [ ] **Step 1: Write the failing tests**

````diff
diff --git a/Cargo.toml b/Cargo.toml
index dc379de..a86ea22 100644
--- a/Cargo.toml
+++ b/Cargo.toml
@@ -77,10 +77,11 @@ tabletist-mcp = { path = "crates/tabletist-mcp" }
 # over. In the tree through tabletist-mcp, which hands one to every call.
 tokio-util = "0.7"
 # The backend runtime: one OS thread running tokio, where every database
-# call happens (src/backend.rs). `net` and `io-util` for the socket agents
-# reach the app through (src/backend/agents/listen.rs), which tabletist-db
-# turns on already: nothing new is built.
-tokio = { version = "1", features = ["rt-multi-thread", "sync", "macros", "time", "net", "io-util"] }
+# call happens (src/backend.rs). `net` for the socket agents reach the app
+# through, `io-util` and `io-std` for `tabletist mcp`, which copies its
+# stdin and stdout to it (src/relay.rs). tabletist-db turns the first two
+# on already: nothing new is built.
+tokio = { version = "1", features = ["rt-multi-thread", "sync", "macros", "time", "net", "io-util", "io-std"] }
 # Native file dialogs (portal on Linux), built on the UI thread and awaited
 # on the backend runtime.
 rfd = "0.17"
diff --git a/src/lib.rs b/src/lib.rs
index 61f25c9..42223a4 100644
--- a/src/lib.rs
+++ b/src/lib.rs
@@ -17,6 +17,7 @@ pub mod labels;
 pub mod macos;
 pub mod model;
 pub mod paths;
+pub mod relay;
 pub mod review;
 pub mod secrets;
 pub mod settings;
diff --git a/src/relay.rs b/src/relay.rs
new file mode 100644
index 0000000..2bc21d0
--- /dev/null
+++ b/src/relay.rs
@@ -0,0 +1,36 @@
+//! `tabletist mcp`: what an agent starts to use Tabletist's connections. It
+//! reaches the running app at its endpoint (`crate::endpoint`), and from
+//! then on copies bytes both ways and understands none of them.
+//!
+//! It writes nothing to stdout but the protocol's bytes, opens no log file
+//! (the app's would be made anew, and lose its lines), and shows no
+//! dialog. Why it stops goes to stderr, and to the agent as the answer to
+//! its first request.
+
+#[cfg(test)]
+mod tests {
+    use super::*;
+
+    #[test]
+    fn only_a_request_is_answered_and_with_its_own_id() {
+        assert_eq!(
+            request_id(br#"{"jsonrpc":"2.0","id":7,"method":"initialize","params":{}}"#),
+            Some(serde_json::json!(7))
+        );
+        assert_eq!(
+            request_id(br#"{"jsonrpc":"2.0","id":"a","method":"tools/list"}"#),
+            Some(serde_json::json!("a"))
+        );
+        assert_eq!(
+            request_id(br#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#),
+            None
+        );
+        assert_eq!(request_id(br#"{"jsonrpc":"2.0","id":1,"result":{}}"#), None);
+        assert_eq!(request_id(b"not json"), None);
+        let line = refusal(serde_json::json!(7), NOT_RUNNING);
+        let answer: serde_json::Value = serde_json::from_slice(&line).unwrap();
+        assert_eq!(answer["id"], 7);
+        assert_eq!(answer["error"]["message"], NOT_RUNNING);
+        assert!(line.ends_with(b"\n"));
+    }
+}
````

- [ ] **Step 2: Run them to see that they do not build**

Run: `cargo test --locked --lib relay::`

Expected: it does not build. The tests name what is not there yet:

```text
error[E0425]: cannot find value `NOT_RUNNING` in this scope
error[E0425]: cannot find function `request_id` in this scope
error[E0425]: cannot find function `refusal` in this scope
error: could not compile `tabletist` (lib test) due to 8 previous errors; 1 warning emitted
```

- [ ] **Step 3: The relay, and the tests of the binary**

````diff
diff --git a/src/entrypoint.rs b/src/entrypoint.rs
index 4d39b29..1c50271 100644
--- a/src/entrypoint.rs
+++ b/src/entrypoint.rs
@@ -2,12 +2,14 @@
 
 use std::path::PathBuf;
 
-use clap::Parser;
+use clap::{Parser, Subcommand};
 
 /// Command-line flags.
 #[derive(Debug, Parser)]
 #[command(name = "tabletist", version, about = "A native database client")]
 pub struct Cli {
+    #[command(subcommand)]
+    pub command: Option<Sub>,
     /// Log at debug level.
     #[arg(long)]
     pub verbose: bool,
@@ -22,6 +24,17 @@ pub struct Cli {
     pub demo_size: Option<[f32; 2]>,
 }
 
+/// What `tabletist` can be asked to do besides opening its window.
+#[derive(Debug, Subcommand)]
+pub enum Sub {
+    /// Connect an agent to the running Tabletist, over stdin and stdout.
+    Mcp {
+        /// The folder of the app's socket and key, for tests.
+        #[arg(long, hide = true)]
+        endpoint: Option<PathBuf>,
+    },
+}
+
 fn parse_size(text: &str) -> Result<[f32; 2], String> {
     let (width, height) = text
         .split_once('x')
@@ -46,7 +59,13 @@ pub const APP_ID: &str = "dev.tabletist.Tabletist";
 /// build on Windows has no console, and an app launched from a menu has no
 /// terminal, so stderr alone would be a silent exit.
 pub fn run() -> std::process::ExitCode {
-    match main() {
+    let cli = Cli::parse();
+    // Before the logger, the settings and the window: the relay writes
+    // nothing but the protocol to stdout and shows no dialog.
+    if let Some(Sub::Mcp { endpoint }) = cli.command {
+        return crate::relay::main(endpoint);
+    }
+    match main(cli) {
         Ok(()) => std::process::ExitCode::SUCCESS,
         Err(error) => {
             report_startup_error(&error);
@@ -78,8 +97,7 @@ fn report_startup_error(error: &anyhow::Error) {
         .show();
 }
 
-pub fn main() -> anyhow::Result<()> {
-    let cli = Cli::parse();
+pub fn main(cli: Cli) -> anyhow::Result<()> {
     // Demo runs use a throwaway profile: nothing touches the user's files.
     // It lives until the window closes and is removed when `main` returns.
     let demo_root = if cli.demo {
diff --git a/src/relay.rs b/src/relay.rs
index 2bc21d0..26ddcad 100644
--- a/src/relay.rs
+++ b/src/relay.rs
@@ -7,6 +7,189 @@
 //! dialog. Why it stops goes to stderr, and to the agent as the answer to
 //! its first request.
 
+use std::path::{Path, PathBuf};
+use std::process::ExitCode;
+use std::time::Duration;
+
+use crate::endpoint::{self, NOT_RUNNING};
+
+/// How long the relay tries the endpoint: the app may be starting.
+#[cfg(unix)]
+const REACH_TIME: Duration = Duration::from_secs(2);
+
+/// How long it waits for the agent's first request, to answer why it
+/// stops.
+const FIRST_REQUEST_TIME: Duration = Duration::from_secs(10);
+
+/// The longest line of the agent's it reads to find that request.
+const MAX_REQUEST: usize = 1024 * 1024;
+
+/// Runs the relay to its end. `endpoint` is the directory to reach, for
+/// tests: the app's own (`AppDirs::agents_dir`) without it.
+pub fn main(endpoint: Option<PathBuf>) -> ExitCode {
+    let dir = endpoint.unwrap_or_else(|| crate::paths::AppDirs::discover().agents_dir());
+    let runtime = match tokio::runtime::Builder::new_current_thread()
+        .enable_all()
+        .build()
+    {
+        Ok(runtime) => runtime,
+        Err(error) => {
+            say(&format!("could not start: {error}"));
+            return ExitCode::FAILURE;
+        }
+    };
+    let code = runtime.block_on(relay(&dir));
+    // A read of stdin still waits on its thread: it is not waited for.
+    runtime.shutdown_background();
+    code
+}
+
+async fn relay(dir: &Path) -> ExitCode {
+    match reach(dir).await {
+        Ok(stream) => pump(stream).await,
+        Err(said) => {
+            stop_with(&said).await;
+            ExitCode::FAILURE
+        }
+    }
+}
+
+/// The app's socket, once it has taken the key. Why not, for the agent.
+#[cfg(unix)]
+async fn reach(dir: &Path) -> Result<tokio::net::UnixStream, String> {
+    use tokio::io::AsyncWriteExt;
+    let socket = dir.join(endpoint::SOCKET);
+    let deadline = tokio::time::Instant::now() + REACH_TIME;
+    let mut stream = loop {
+        match tokio::net::UnixStream::connect(&socket).await {
+            Ok(stream) => break stream,
+            Err(_) if tokio::time::Instant::now() < deadline => {
+                tokio::time::sleep(Duration::from_millis(100)).await;
+            }
+            Err(_) => return Err(NOT_RUNNING.into()),
+        }
+    };
+    let key = std::fs::read_to_string(dir.join(endpoint::KEY)).map_err(|_| NOT_RUNNING)?;
+    let hello = endpoint::Hello {
+        key: key.trim().to_owned(),
+        version: env!("CARGO_PKG_VERSION").to_owned(),
+        cwd: std::env::current_dir()
+            .ok()
+            .map(|cwd| cwd.to_string_lossy().into_owned()),
+    };
+    stream
+        .write_all(&endpoint::line_of(&hello))
+        .await
+        .map_err(|_| NOT_RUNNING)?;
+    let answer = tokio::time::timeout(
+        endpoint::LINE_TIME,
+        endpoint::read_line(&mut stream, endpoint::MAX_LINE),
+    )
+    .await;
+    let answer = match answer {
+        Ok(Ok(Some(line))) => serde_json::from_slice::<endpoint::Answer>(&line).ok(),
+        _ => None,
+    };
+    match answer {
+        Some(endpoint::Answer::Ok) => Ok(stream),
+        Some(endpoint::Answer::Refused(said)) => Err(said),
+        None => Err("Tabletist did not answer the relay.".into()),
+    }
+}
+
+/// Nothing to reach here yet (`endpoint::AVAILABLE`).
+#[cfg(not(unix))]
+async fn reach(_dir: &Path) -> Result<std::convert::Infallible, String> {
+    Err("Agents cannot reach Tabletist on this system yet.".into())
+}
+
+/// Copies the agent's bytes to the app and the app's to the agent, until
+/// the app ends the session. The agent's end only ends what it sends: the
+/// app still answers what it was asked.
+#[cfg(unix)]
+async fn pump(stream: tokio::net::UnixStream) -> ExitCode {
+    use tokio::io::AsyncWriteExt;
+    let (mut from_app, mut to_app) = stream.into_split();
+    let up = async {
+        let mut stdin = tokio::io::stdin();
+        let _ = tokio::io::copy(&mut stdin, &mut to_app).await;
+        let _ = to_app.shutdown().await;
+        std::future::pending::<()>().await;
+    };
+    let down = async {
+        let mut stdout = tokio::io::stdout();
+        let copied = tokio::io::copy(&mut from_app, &mut stdout).await;
+        let flushed = stdout.flush().await;
+        copied.is_ok() && flushed.is_ok()
+    };
+    let done = tokio::select! {
+        done = down => done,
+        () = up => unreachable!(),
+    };
+    match done {
+        true => ExitCode::SUCCESS,
+        false => ExitCode::FAILURE,
+    }
+}
+
+#[cfg(not(unix))]
+async fn pump(stream: std::convert::Infallible) -> ExitCode {
+    match stream {}
+}
+
+/// Says why the relay stops: on stderr, and as the error that answers the
+/// agent's first request, which is what a client shows. A client that asks
+/// nothing in time is told on stderr alone.
+async fn stop_with(said: &str) {
+    use tokio::io::AsyncWriteExt;
+    say(said);
+    // Nothing is copied after this: what the buffer takes past a line is
+    // the agent's and goes nowhere.
+    let mut stdin = tokio::io::BufReader::new(tokio::io::stdin());
+    let found = tokio::time::timeout(FIRST_REQUEST_TIME, async {
+        loop {
+            match endpoint::read_line(&mut stdin, MAX_REQUEST).await {
+                Ok(Some(line)) => {
+                    if let Some(id) = request_id(&line) {
+                        return Some(id);
+                    }
+                }
+                _ => return None,
+            }
+        }
+    })
+    .await;
+    if let Ok(Some(id)) = found {
+        let mut stdout = tokio::io::stdout();
+        let _ = stdout.write_all(&refusal(id, said)).await;
+        let _ = stdout.flush().await;
+    }
+}
+
+/// Writes `said` to stderr. Nothing comes of a stderr that is closed: the
+/// agent is still answered.
+fn say(said: &str) {
+    use std::io::Write;
+    let _ = writeln!(std::io::stderr(), "tabletist mcp: {said}");
+}
+
+/// The id of a request, where `line` is one: a notification has none, and
+/// is not answered.
+fn request_id(line: &[u8]) -> Option<serde_json::Value> {
+    let message: serde_json::Value = serde_json::from_slice(line).ok()?;
+    message.get("method")?;
+    message.get("id").cloned()
+}
+
+/// The error that answers request `id` with `said`, as a line.
+fn refusal(id: serde_json::Value, said: &str) -> Vec<u8> {
+    endpoint::line_of(&serde_json::json!({
+        "jsonrpc": "2.0",
+        "id": id,
+        "error": { "code": -32000, "message": said },
+    }))
+}
+
 #[cfg(test)]
 mod tests {
     use super::*;
diff --git a/tests/cli.rs b/tests/cli.rs
index 0ef307d..2270060 100644
--- a/tests/cli.rs
+++ b/tests/cli.rs
@@ -1,5 +1,8 @@
 //! Runs the built binary, the way a launcher or a packager would.
 
+// `allow-unwrap-in-tests` does not cover helpers in an integration-test crate.
+#![allow(clippy::unwrap_used)]
+
 use std::process::Command;
 
 fn tabletist() -> Command {
@@ -31,3 +34,277 @@ fn demo_flags_require_demo() {
         "--demo-shot without --demo must be rejected"
     );
 }
+
+/// `tabletist mcp`, the relay an agent starts, against the app's backend
+/// serving agents in a folder of the test's own.
+#[cfg(unix)]
+mod mcp {
+    use std::io::{BufRead, BufReader, Read, Write};
+    use std::path::{Path, PathBuf};
+    use std::process::{Child, Stdio};
+    use std::sync::mpsc;
+    use std::time::{Duration, Instant};
+
+    use serde_json::{Value, json};
+    use tabletist::backend::{
+        AgentsCommand, AgentsEvent, Backend, Command, Event, Waker, agentable,
+    };
+    use tabletist::connections::{
+        AgentAccess, Agents, ConnectionId, PasswordMode, SavedConnection,
+    };
+    use tabletist::env::Environment;
+    use tabletist::secrets::Keyring;
+    use tabletist_db::{ConnectSpec, HostKeys};
+
+    use super::tabletist;
+
+    const WAIT: Duration = Duration::from_secs(20);
+
+    /// In the database's path, which nothing an agent is sent may hold.
+    const MARKER: &str = "PRIVATE-7f3a";
+
+    fn relay(endpoint: &Path, cwd: &Path) -> Child {
+        tabletist()
+            .arg("mcp")
+            .arg("--endpoint")
+            .arg(endpoint)
+            .current_dir(cwd)
+            .stdin(Stdio::piped())
+            .stdout(Stdio::piped())
+            .stderr(Stdio::piped())
+            .spawn()
+            .expect("run tabletist mcp")
+    }
+
+    fn line(message: &Value) -> Vec<u8> {
+        let mut line = serde_json::to_vec(message).unwrap();
+        line.push(b'\n');
+        line
+    }
+
+    fn initialize(id: u64) -> Value {
+        json!({
+            "jsonrpc": "2.0",
+            "id": id,
+            "method": "initialize",
+            "params": {
+                "protocolVersion": "2025-11-25",
+                "capabilities": {},
+                "clientInfo": { "name": "claude-code", "version": "1" },
+            },
+        })
+    }
+
+    fn call(id: u64, name: &str, arguments: Value) -> Value {
+        json!({
+            "jsonrpc": "2.0",
+            "id": id,
+            "method": "tools/call",
+            "params": { "name": name, "arguments": arguments },
+        })
+    }
+
+    /// The lines of the child's stdout as they come, read on a thread of
+    /// their own so that a test that waits cannot wait for good.
+    fn lines_of(child: &mut Child) -> mpsc::Receiver<String> {
+        let stdout = child.stdout.take().unwrap();
+        let (send, lines) = mpsc::channel();
+        std::thread::spawn(move || {
+            for line in BufReader::new(stdout).lines() {
+                let Ok(line) = line else { return };
+                if send.send(line).is_err() {
+                    return;
+                }
+            }
+        });
+        lines
+    }
+
+    fn stderr_of(child: &mut Child) -> String {
+        let mut stderr = String::new();
+        child
+            .stderr
+            .take()
+            .unwrap()
+            .read_to_string(&mut stderr)
+            .unwrap();
+        stderr
+    }
+
+    /// Waits for the child to end, at most [`WAIT`].
+    fn ended(child: &mut Child) -> std::process::ExitStatus {
+        let deadline = Instant::now() + WAIT;
+        loop {
+            if let Some(status) = child.try_wait().unwrap() {
+                return status;
+            }
+            assert!(Instant::now() < deadline, "tabletist mcp did not end");
+            std::thread::sleep(Duration::from_millis(20));
+        }
+    }
+
+    fn agents_said(backend: &mut Backend) -> AgentsEvent {
+        let deadline = Instant::now() + WAIT;
+        loop {
+            for event in backend.poll() {
+                if let Event::Agents(event) = event {
+                    return event;
+                }
+            }
+            assert!(
+                Instant::now() < deadline,
+                "the backend said nothing of agents"
+            );
+            std::thread::sleep(Duration::from_millis(20));
+        }
+    }
+
+    /// Asks the relay that answers the agent's first request with why it
+    /// stopped, and returns that and what stderr says.
+    fn refused(endpoint: &Path, cwd: &Path) -> (String, String) {
+        let mut child = relay(endpoint, cwd);
+        child
+            .stdin
+            .take()
+            .unwrap()
+            .write_all(&line(&initialize(7)))
+            .unwrap();
+        let lines = lines_of(&mut child);
+        let status = ended(&mut child);
+        assert_eq!(status.code(), Some(1));
+        let said: Vec<String> = lines.try_iter().collect();
+        assert_eq!(said.len(), 1, "one line and nothing else: {said:?}");
+        let answer: Value = serde_json::from_str(&said[0]).unwrap();
+        assert_eq!(answer["id"], 7);
+        let message = answer["error"]["message"].as_str().unwrap().to_owned();
+        (message, stderr_of(&mut child))
+    }
+
+    #[test]
+    fn without_the_app_the_first_request_is_answered_with_why_and_it_ends() {
+        let dir = tempfile::tempdir().unwrap();
+        let (message, stderr) = refused(&dir.path().join("run"), dir.path());
+        assert_eq!(message, tabletist::endpoint::NOT_RUNNING);
+        assert!(
+            stderr.contains(tabletist::endpoint::NOT_RUNNING),
+            "{stderr}"
+        );
+    }
+
+    #[test]
+    fn an_agent_reads_through_the_relay_and_nothing_private_comes_back() {
+        let dir = tempfile::tempdir().unwrap();
+        let database = dir.path().join(format!("{MARKER}.db"));
+        tabletist_db::fixtures::write_sqlite_bookshop(&database).unwrap();
+        let connection = SavedConnection {
+            id: ConnectionId("shop".into()),
+            name: "Bookshop".into(),
+            environment: Environment::Dev,
+            read_only: None,
+            password: PasswordMode::None,
+            ssh_secret: PasswordMode::None,
+            spec: ConnectSpec::sqlite(database),
+            agents: Agents {
+                access: AgentAccess::Read,
+                ..Agents::default()
+            },
+        };
+        let endpoint: PathBuf = dir.path().join("run");
+        let log = dir.path().join("agents.sqlite3");
+        let mut backend = Backend::start_with(Waker::default(), Keyring::memory());
+        backend.send(Command::Agents(AgentsCommand::Policy {
+            connections: agentable(&[connection]),
+            host_keys: HostKeys::default(),
+        }));
+        backend.send(Command::Agents(AgentsCommand::Start {
+            log: log.clone(),
+            log_days: 30,
+            endpoint: Some(endpoint.clone()),
+        }));
+        assert_eq!(
+            agents_said(&mut backend),
+            AgentsEvent::Started { kept_aside: None }
+        );
+        let project = dir.path().join("bookshop-repo");
+        std::fs::create_dir(&project).unwrap();
+        let mut child = relay(&endpoint, &project);
+        let mut stdin = child.stdin.take().unwrap();
+        let lines = lines_of(&mut child);
+        let mut seen = String::new();
+        let mut answer = |message: Value, answered: bool| {
+            stdin.write_all(&line(&message)).unwrap();
+            if !answered {
+                return Value::Null;
+            }
+            let said = lines.recv_timeout(WAIT).expect("an answer");
+            seen.push_str(&said);
+            serde_json::from_str::<Value>(&said).unwrap()
+        };
+        assert!(answer(initialize(0), true)["result"].is_object());
+        // A notification, which nothing answers.
+        let initialized = json!({ "jsonrpc": "2.0", "method": "notifications/initialized" });
+        answer(initialized, false);
+        let listed = answer(call(1, "list_connections", json!({})), true);
+        let text = listed["result"]["content"][0]["text"]
+            .as_str()
+            .unwrap()
+            .to_owned();
+        assert!(text.contains("\"name\":\"Bookshop · dev\""), "{text}");
+        let ran = answer(
+            call(
+                2,
+                "run_query",
+                json!({ "connection": "Bookshop · dev", "sql": "SELECT count(*) FROM books" }),
+            ),
+            true,
+        );
+        let text = ran["result"]["content"][0]["text"]
+            .as_str()
+            .unwrap()
+            .to_owned();
+        assert!(text.contains("\"rows\":[[12]]"), "{text}");
+        // A table that is not there: the database's error comes back.
+        let failed = answer(
+            call(
+                3,
+                "run_query",
+                json!({ "connection": "Bookshop · dev", "sql": "SELECT * FROM no_such_table" }),
+            ),
+            true,
+        );
+        assert_eq!(failed["result"]["isError"], true, "{failed}");
+        // The agent's end ends the session, and the relay with it.
+        drop(stdin);
+        assert!(ended(&mut child).success());
+        let stderr = stderr_of(&mut child);
+        assert!(!seen.contains(MARKER), "{seen}");
+        assert!(!stderr.contains(MARKER), "{stderr}");
+        // The log has the calls, under the project's name.
+        let runtime = tokio::runtime::Runtime::new().unwrap();
+        let entries = runtime.block_on(async {
+            tabletist_mcp::Log::open(&log)
+                .unwrap()
+                .log
+                .latest(10)
+                .await
+                .unwrap()
+        });
+        assert_eq!(entries.len(), 3);
+        assert!(
+            entries
+                .iter()
+                .all(|entry| entry.project.as_deref() == Some("bookshop-repo")),
+            "{entries:?}"
+        );
+        // A key that is not the one the app made.
+        std::fs::write(endpoint.join("mcp.key"), "0".repeat(64)).unwrap();
+        let (message, _) = refused(&endpoint, &project);
+        assert!(message.contains("key"), "{message}");
+        // Stopped, nothing answers: as if the app were not running.
+        backend.send(Command::Agents(AgentsCommand::Stop));
+        assert_eq!(agents_said(&mut backend), AgentsEvent::Stopped);
+        assert!(!endpoint.join("mcp.sock").exists());
+        let (message, _) = refused(&endpoint, &project);
+        assert_eq!(message, tabletist::endpoint::NOT_RUNNING);
+    }
+}
````

- [ ] **Step 4: Run the tests**

Run: `cargo test --locked --lib relay::`, then `cargo test --locked --test cli`

Expected: PASS. The binary's four take about two seconds: the relay tries the socket that is not there for two.

```text
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 2452 filtered out
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

- [ ] **Step 5: Commit**

```bash
cargo fmt --all
cargo clippy --locked --workspace --all-targets -- -D warnings
git add src/relay.rs src/entrypoint.rs src/lib.rs tests/cli.rs Cargo.toml
git commit -m "Connect an agent to the running app with tabletist mcp"
```

---

### Task 6: The app serves agents

`App::serve_agents`, called by the entrypoint once the window is up and not for a demo, tells the backend what agents may use and starts the server where the settings say. From then on:

- `send_agents_policy` after every save of the connections and every host key trusted;
- `apply_settings` starts and stops the server, and sends `LogDays`, whether the change came from the app or from the file;
- `Event::Agents` says, as a notice, a start that failed and a log kept aside. On Windows, agents turned on say that they cannot reach Tabletist there yet.

**Files:**
- Modify: `src/app.rs`, `src/entrypoint.rs`

- [ ] **Step 1: Write the failing tests**

````diff
diff --git a/src/app.rs b/src/app.rs
index a4dced6..da3caf5 100644
--- a/src/app.rs
+++ b/src/app.rs
@@ -5278,6 +5278,121 @@ mod tests {
         }
     }
 
+    /// What the app asked the backend to do with the agent server.
+    fn agents_sent(app: &App) -> Vec<&AgentsCommand> {
+        app.backend
+            .sent
+            .iter()
+            .filter_map(|command| match command {
+                Command::Agents(agents) => Some(agents),
+                _ => None,
+            })
+            .collect()
+    }
+
+    #[test]
+    fn the_settings_start_and_stop_the_agent_server_once_the_app_serves_it() {
+        let (mut app, dir) = app();
+        let dirs = AppDirs::at(dir.path());
+        // Not before: tests and the demo never start it.
+        app.change_settings(|settings| settings.mcp_enabled = true);
+        assert!(agents_sent(&app).is_empty());
+        app.serve_agents();
+        let start = |app: &App, days: u32| {
+            agents_sent(app).iter().any(|sent| {
+                matches!(sent, AgentsCommand::Start { log, log_days, endpoint }
+                    if *log == dirs.agents_log_file()
+                        && *log_days == days
+                        && endpoint.as_deref() == Some(dirs.agents_dir().as_path()))
+            })
+        };
+        // What agents may use comes first: none of the connections, here.
+        assert!(matches!(
+            agents_sent(&app).first(),
+            Some(AgentsCommand::Policy { connections, .. }) if connections.is_empty()
+        ));
+        // Not on a system it cannot be reached on, which says so.
+        assert_eq!(start(&app, 30), crate::endpoint::AVAILABLE);
+        assert_eq!(app.notice.is_some(), !crate::endpoint::AVAILABLE);
+        app.backend.sent.clear();
+        app.change_settings(|settings| settings.mcp_log_days = 90);
+        assert!(matches!(
+            agents_sent(&app)[..],
+            [AgentsCommand::LogDays(90)]
+        ));
+        app.backend.sent.clear();
+        app.change_settings(|settings| settings.mcp_enabled = false);
+        assert!(matches!(agents_sent(&app)[..], [AgentsCommand::Stop]));
+        // Off, the days change nothing it would be told.
+        app.backend.sent.clear();
+        app.change_settings(|settings| settings.mcp_log_days = 7);
+        assert!(agents_sent(&app).is_empty());
+        // And an edit of the file turns it on as the app does.
+        let text = Settings {
+            mcp_enabled: true,
+            ..app.settings.clone()
+        }
+        .to_toml();
+        app.apply_event(Event::SettingsFile { text, own: false });
+        assert!(app.settings.mcp_enabled);
+        assert_eq!(start(&app, 7), crate::endpoint::AVAILABLE);
+    }
+
+    #[test]
+    fn agents_are_told_of_each_change_to_what_they_may_use() {
+        let (mut app, _dir) = app();
+        let policy = |app: &App| match agents_sent(app).last() {
+            Some(AgentsCommand::Policy {
+                connections,
+                host_keys,
+            }) => (
+                connections
+                    .iter()
+                    .map(|connection| connection.name.clone())
+                    .collect::<Vec<_>>(),
+                host_keys.clone(),
+            ),
+            other => panic!("{other:?}"),
+        };
+        app.serve_agents();
+        assert_eq!(policy(&app).0, Vec::<String>::new());
+        // A connection agents may read, saved in the dialog.
+        app.apply(Action::NewConnection);
+        form(&mut app).name = "Disk".into();
+        form(&mut app).sqlite_path = "/tmp/disk.db".into();
+        form(&mut app).agents.access = crate::connections::AgentAccess::Read;
+        app.apply(Action::SaveConnection { connect: false });
+        assert_eq!(policy(&app).0, ["Disk · local"]);
+        // A host key trusted.
+        app.trust("bastion", 22, "SHA256:F1");
+        assert_eq!(policy(&app).1.fingerprint("bastion", 22), Some("SHA256:F1"));
+    }
+
+    #[test]
+    fn what_the_agent_server_could_not_do_is_said() {
+        let (mut app, _dir) = app();
+        app.apply_event(Event::Agents(AgentsEvent::Started { kept_aside: None }));
+        app.apply_event(Event::Agents(AgentsEvent::Stopped));
+        assert_eq!(app.notice, None);
+        app.apply_event(Event::Agents(AgentsEvent::Failed(
+            "another Tabletist is serving agents already".into(),
+        )));
+        assert_eq!(
+            app.notice.as_deref(),
+            Some("Agents could not be served: another Tabletist is serving agents already.")
+        );
+        app.apply_event(Event::Agents(AgentsEvent::Started {
+            kept_aside: Some("/state/agents.sqlite3.bad".into()),
+        }));
+        assert!(
+            app.notice
+                .as_deref()
+                .is_some_and(|notice| notice.contains("/state/agents.sqlite3.bad")),
+            "{:?}",
+            app.notice
+        );
+    }
+
     fn settings_saves(app: &App) -> Vec<&Settings> {
         app.backend
             .sent
````

- [ ] **Step 2: Run them to see that they do not build**

Run: `cargo test --locked --lib -- app::tests::the_settings_start app::tests::agents_are_told app::tests::what_the_agent_server`

Expected: it does not build. The tests name what is not there yet:

```text
error[E0425]: cannot find type `AgentsCommand` in this scope
error[E0433]: cannot find type `AgentsCommand` in this scope
error[E0433]: cannot find type `AgentsEvent` in this scope
error[E0599]: no method named `serve_agents` found for struct `app::App` in the current scope
error: could not compile `tabletist` (lib test) due to 12 previous errors
```

- [ ] **Step 3: The app's side**

````diff
diff --git a/src/app.rs b/src/app.rs
index da3caf5..ec51ad6 100644
--- a/src/app.rs
+++ b/src/app.rs
@@ -8,7 +8,9 @@ use tabletist_db::{
     Driver, Error, FilterOp, HostKeys, ObjectKind, ObjectRef, Secrets, Sort, SortDir, SshStage,
 };
 
-use crate::backend::{CancelReason, Command, Event, Opened, RequestId, StateFile};
+use crate::backend::{
+    AgentsCommand, AgentsEvent, CancelReason, Command, Event, Opened, RequestId, StateFile,
+};
 use crate::completion::Need;
 use crate::connections::{PasswordMode, SavedConnection};
 use crate::edit::EditorPlace;
@@ -123,6 +125,9 @@ pub struct App {
     /// The window is closing: it was asked to, and no tab holds edits any
     /// more, or the user gave them up.
     pub closing: bool,
+    /// The agent server is the app's to start and stop
+    /// ([`App::serve_agents`]): not in tests, nor in the demo.
+    serves_agents: bool,
     next_id: u64,
 }
 
@@ -183,6 +188,7 @@ impl App {
             lookup_delay: crate::key_picker::DELAY,
             lookups: Vec::new(),
             closing: false,
+            serves_agents: false,
             next_id: 1,
         };
         let tab = app.picker_tab();
@@ -2796,9 +2802,22 @@ impl App {
                 };
                 self.apply_settings(settings);
             }
-            // Nothing in the app starts the agent server yet: that comes
-            // with its settings. The backend logs what happens to it.
-            Event::Agents(_) => {}
+            // What the person turned on in the file and does not get is
+            // said. The backend logs the rest.
+            Event::Agents(event) => match event {
+                AgentsEvent::Started { kept_aside: None } | AgentsEvent::Stopped => {}
+                AgentsEvent::Started {
+                    kept_aside: Some(path),
+                } => {
+                    self.notice = Some(format!(
+                        "The log of agents' calls could not be read. It was kept aside as {}, and a new one begun.",
+                        path.display()
+                    ));
+                }
+                AgentsEvent::Failed(said) => {
+                    self.notice = Some(format!("Agents could not be served: {said}."));
+                }
+            },
             Event::SettingsFileOpened { with, result } => {
                 if let Err(error) = result {
                     let path = self.dirs.settings_file();
@@ -2959,6 +2978,7 @@ impl App {
             path: self.dirs.known_hosts_file(),
             file: StateFile::KnownHosts(self.host_keys.clone()),
         });
+        self.send_agents_policy();
     }
 
     /// Remembers that `tab`'s saved connection connected now.
@@ -3000,6 +3020,47 @@ impl App {
             path: self.dirs.connections_file(),
             file: StateFile::Connections(self.connections.clone()),
         });
+        // Every change to a connection is saved: what agents may use
+        // changes with it.
+        self.send_agents_policy();
+    }
+
+    /// Takes the agent server on: tells the backend what agents may use,
+    /// and starts the server where the settings say. From then on the
+    /// settings start and stop it. Called once, as the window opens.
+    pub fn serve_agents(&mut self) {
+        self.serves_agents = true;
+        self.send_agents_policy();
+        if self.settings.mcp_enabled {
+            self.start_agents();
+        }
+    }
+
+    /// Tells the backend which connections agents may use, and the SSH host
+    /// keys the person trusts. Sent while the server is off too: it keeps
+    /// them for when it starts.
+    fn send_agents_policy(&mut self) {
+        if !self.serves_agents {
+            return;
+        }
+        let connections = crate::backend::agentable(&self.connections.connections);
+        self.backend.send(Command::Agents(AgentsCommand::Policy {
+            connections,
+            host_keys: self.host_keys.clone(),
+        }));
+    }
+
+    fn start_agents(&mut self) {
+        if !crate::endpoint::AVAILABLE {
+            log::warn!("agents are turned on, and cannot reach Tabletist on this system yet");
+            self.notice = Some("Agents cannot reach Tabletist on this system yet.".into());
+            return;
+        }
+        self.backend.send(Command::Agents(AgentsCommand::Start {
+            log: self.dirs.agents_log_file(),
+            log_days: self.settings.mcp_log_days,
+            endpoint: Some(self.dirs.agents_dir()),
+        }));
     }
 
     /// Sends the settings to the backend to be written. Only
@@ -3089,6 +3150,18 @@ impl App {
             // Reading a theme needs the window: `logic` has it.
             self.theme_changed = true;
         }
+        if self.serves_agents {
+            match (old.mcp_enabled, self.settings.mcp_enabled) {
+                (false, true) => self.start_agents(),
+                (true, false) => self.backend.send(Command::Agents(AgentsCommand::Stop)),
+                (true, true) if old.mcp_log_days != self.settings.mcp_log_days => {
+                    let days = self.settings.mcp_log_days;
+                    self.backend
+                        .send(Command::Agents(AgentsCommand::LogDays(days)));
+                }
+                _ => {}
+            }
+        }
     }
 
     /// Fetches again, at the settings' page size, every table that shows a
diff --git a/src/entrypoint.rs b/src/entrypoint.rs
index 1c50271..f14caf8 100644
--- a/src/entrypoint.rs
+++ b/src/entrypoint.rs
@@ -136,6 +136,10 @@ pub fn main(cli: Cli) -> anyhow::Result<()> {
             );
             let mut app = App::new(dirs, settings, backend);
             app.attach(&cc.egui_ctx, !demo);
+            // Not for a demo: its profile and its keyring are throwaway.
+            if !demo {
+                app.serve_agents();
+            }
             if demo {
                 demo_setup(&mut app);
             }
````

- [ ] **Step 4: Run the tests**

Run: the same, then `cargo test --locked --lib app::`

Expected: PASS.

```text
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 2453 filtered out
test result: ok. 391 passed; 0 failed; 0 ignored; 0 measured; 2065 filtered out
```

- [ ] **Step 5: Commit**

```bash
cargo fmt --all
cargo clippy --locked --workspace --all-targets -- -D warnings
git add src/app.rs src/entrypoint.rs
git commit -m "Start and stop the agent server with the settings, and tell it what agents may use"
```

---

### Task 7: The website's reference, and the master plan

- `docs/_reference/settings-and-files.md`: the two `[mcp]` keys, `agents.sqlite3`, `mcp.sock` and `mcp.key` and where they are, `tabletist mcp`, and that `[connections.agents]` is used while agents are on.
- The master plan: what this run built, and the four departures agreed on 2026-10-14.

**Files:**
- Modify: `docs/_reference/settings-and-files.md`, `docs/superpowers/plans/2026-10-13-mcp-server.md`

- [ ] **Step 1: Correct both**

````diff
diff --git a/docs/_reference/settings-and-files.md b/docs/_reference/settings-and-files.md
index b071542..297e958 100644
--- a/docs/_reference/settings-and-files.md
+++ b/docs/_reference/settings-and-files.md
@@ -41,6 +41,10 @@ show_system_schemas = false
 [editor]
 sql_limit = 1000
 sql_timeout_secs = 30
+
+[mcp]
+enabled = false
+log_days = 30
 ```
 
 | Key | Values | Default |
@@ -53,8 +57,11 @@ sql_timeout_secs = 30
 | `editor.sql_limit` | 1 to 10000 rows for each statement | `1000` |
 | `editor.sql_timeout_secs` | Seconds. `0` waits forever | `30` |
 | `appearance.theme` | The name of a palette file in the `themes` folder | Not set |
+| `mcp.enabled` | `true` lets agents reach the connections you open to them, through `tabletist mcp`. Linux and macOS | `false` |
+| `mcp.log_days` | 1 to 3650 days. How long the log of agents' calls keeps a call | `30` |
 
-`show_system_schemas` and `theme` are set only in the file. The SQL
+`show_system_schemas`, `theme` and the two `mcp` keys are set only in the
+file. The SQL
 editor's **Limit** and **Timeout** menus set the two `editor` keys.
 
 A line that cannot be read is ignored, keeps its default, and is reported
@@ -83,6 +90,13 @@ On Linux, `XDG_CONFIG_HOME` and `XDG_STATE_HOME` move the two folders.
 | `connections.moved` | State | A note that your connections are kept in `connections.toml` |
 | `labels.json` | State | The column you chose to label each table's rows in a foreign key's list, by saved connection and, on PostgreSQL, by database |
 | `tabletist.log`, `panic.log` | State | Logs |
+| `agents.sqlite3` | State | Every call agents made: the tool, the statement and its values, and how it ended. Never a row |
+| `mcp.sock`, `mcp.key` | Runtime, else State | Where `tabletist mcp` reaches the app, and the key it must send. There while agents are on |
+
+The runtime folder is `/run/user/<your user id>/tabletist` on Linux, where
+the system makes `/run/user` for you. macOS has none, and uses the state
+folder, or `.tabletist` in your home folder when the state folder's path is
+too long for a socket.
 
 Passwords are kept in the system keyring, never in these files.
 [Read about passwords]({% link _reference/connections.md %}#passwords).
@@ -148,8 +162,8 @@ What agents may do on a connection is its `[connections.agents]` table:
 and `timeout_secs`, how long a statement may run (1 to 60). Without the
 table, or a key of it, a connection is `off`, with 500 rows and 10 seconds.
 A file in which a connection has the table says `version = 4`, and one in
-which none has it stays `version = 3`. Nothing in Tabletist uses the table
-yet: it is read and kept.
+which none has it stays `version = 3`. Agents use the table only while
+`[mcp] enabled` is `true` in `settings.toml`.
 
 Edit the file while Tabletist is closed. Tabletist writes the whole file
 again when you change a connection, so a comment you add is not kept.
@@ -188,6 +202,7 @@ read once more, remove `connections.moved` as well.
 | `--verbose` | Logs at debug level |
 | `--version` | Prints the version |
 | `--help` | Lists the options |
+| `mcp` | Connects an agent to the running Tabletist, over its stdin and stdout. An agent starts it. When Tabletist is not running, or agents are off in it, it says so to the agent and ends |
 
 `--help` also lists `--demo-shot`, which saves a picture of the demo's
 window to a file, and `--demo-size`, which sets that window's size.
diff --git a/docs/superpowers/plans/2026-10-13-mcp-server.md b/docs/superpowers/plans/2026-10-13-mcp-server.md
index dbc0b58..f6e2a74 100644
--- a/docs/superpowers/plans/2026-10-13-mcp-server.md
+++ b/docs/superpowers/plans/2026-10-13-mcp-server.md
@@ -35,6 +35,13 @@ the backend's gateway, and agents in `connections.toml`. G cuts the second
 run in two, as it said it might. A.1, C.1, F.4, G and question 19 are
 corrected where its draft showed otherwise.
 
+**Built on 2026-10-14: the second part of Phase 1b's second run** (`2026-10-14-mcp-server-1b-2b-stdio.md`):
+the `[mcp]` settings, the endpoint, the key and the relay, and the keychain
+on macOS. Four departures were agreed the same day, each where it stands:
+the key beside the socket, the app's answer to the relay's first line
+(A.2), the website's reference for the files (G), and the check of changes
+made outside the app moved to the first control that turns agents on (C.2).
+
 Each phase below gets its own plan under `docs/superpowers/plans/` before any
 code is written, as every batch of work in this repository does.
 
@@ -177,9 +184,13 @@ What its draft's review showed it must also do:
   brings up a prompt to unlock it: on Linux the Secret Service would ask
   for a collection whose item a read finds locked, so the item is looked
   for first and a locked one is not read (`SecretStore::read_quietly`).
-  Windows asks nothing of a read. **On macOS a locked keychain is asked to
-  be unlocked by the system, and the keyring crate has no call that says
-  not to**: open for the second part, which makes the server reachable.
+  Windows asks nothing of a read. On macOS the keychain is told to ask
+  nothing while it reads for an agent (`SecKeychain::
+  disable_user_interaction`, in `security-framework`, which the keychain
+  store is built on): a locked keychain, and an item that would ask whether
+  Tabletist may read it, answer that they would have to, and are not read.
+  The switch is the process's, and the keyring thread, the only one that
+  uses the keychain, does one job at a time (`2026-10-14-mcp-server-1b-2b-stdio.md`).
 - **Look again after a wait.** A call that waited for the connection, or
   for its connect, takes the connection as it is once it has it: turned
   Off meanwhile, it runs nothing; changed, it is opened anew.
@@ -248,6 +259,30 @@ and `eframe::run_native` (`src/entrypoint.rs:81-161`):
 So the protocol lives in one place, the app, and an old relay keeps working
 with a newer app.
 
+**As built** (`2026-10-14-mcp-server-1b-2b-stdio.md`). Agreed on 2026-10-14, where it went otherwise:
+
+- **The key is beside the socket**, `mcp.key` in the endpoint's directory
+  (`AppDirs::agents_dir`), not in the state directory: the relay finds both
+  in one place, and a test points it at another with `--endpoint`. On
+  Linux the directory is `/run/user/<id>/tabletist`, found by the user's
+  id and not by `XDG_RUNTIME_DIR`, which an agent may leave out of the
+  relay's environment; else the state directory. It is made 0700, and one
+  that is not the user's to change is refused. The key is written before
+  the socket is made, so that a relay never reads an ended start's.
+- **The app answers the relay's line** with one of its own, `"ok"` or a
+  refusal with its reason, so that a wrong key, or too many agents, reaches
+  the agent as the answer to its first request and not as a closed pipe.
+- The relay's line is JSON: the key, its version, and its working
+  directory, whose last part names the session's project. Each line may
+  have 16 KiB and must come within five seconds.
+- A socket that answers is another Tabletist's: the server does not start,
+  and says so. One that does not answer was left by a Tabletist that ended,
+  and is replaced. The socket and the key are removed when the server
+  stops, the socket only while it is the one the server made.
+- At most 32 agents at once, counted once they have the key, and at most
+  64 connections waiting to say their line. A second Tabletist that finds
+  the socket answering does not start, and does not open the log either.
+
 **When the app is not running.** A clear error, and no launch:
 
 - The relay tries the endpoint for two seconds (the app may be starting).
@@ -1188,6 +1223,11 @@ notify   = true
   edit in an editor reaches `App::apply_settings` (`src/app.rs:2894-2921`),
   which starts or stops the server. See C.2 for the one edit that is not
   applied as it stands.
+- **As built** (`2026-10-14-mcp-server-1b-2b-stdio.md`): `enabled` and `log_days` (1 to 3,650, else
+  brought into range). `listen` comes with HTTP (1d) and `notify` with the
+  notification for a new agent (the fourth run): a key that does nothing
+  would be written into every file. A change of `log_days` while the
+  server runs is the log's from then on, and stops no session.
 
 **HTTP tokens: `agents.json`** in the state directory, 0600, through
 `util::write_atomic`: `{ "id", "name", "sha256", "created" }` for each. Not
@@ -1331,6 +1371,12 @@ It also catches a synced dotfile and a restored backup. It does not stop a
 program that edits the remembered copy as well. Writes still wait for the
 person either way.
 
+**Moved on 2026-10-14: built with the first control that turns agents on**
+(the third run's **Turn on**). Until then the files are the only way to
+turn agents on, and a check that refused changes made outside the app would
+refuse the only one there is. So after Phase 1b's second run a program that
+can edit both files can turn agents on for itself, as E says of a shell.
+
 ### C.3 Hidden tables and masked columns when the schema changes
 
 Entries are names: `customers`, `sales.customers`, `orders.email`. A schema
@@ -1916,7 +1962,7 @@ is off until the person turns it on.
 | Part | Runs | State on 2026-10-13 |
 |---|---|---|
 | 1a. The enforcement core | 3 | Done: pull requests 122, 123 and 124 |
-| 1b. The server over stdio, with its interface | 5, below | The first is built (`2026-10-13-mcp-server-1b-1-server.md`). Nothing the others wait for is open |
+| 1b. The server over stdio, with its interface | 6, below | The first two are built (`2026-10-13-mcp-server-1b-1-server.md`, `2026-10-14-mcp-server-1b-2a-gateway.md` and `2026-10-14-mcp-server-1b-2b-stdio.md`). Nothing the others wait for is open |
 | 1b on Windows: the way in | 1 | After 1b (question 14) |
 | 1c. Hidden tables | to be cut in its plan | Waits for questions 8 and 13 |
 | 1d. HTTP with tokens | 1 | After stdio (question 9) |
@@ -1984,7 +2030,7 @@ can see what agents do and stop them.
 |---|---|---|---|
 | **1b-1. The server, with nothing to start it** | The `tabletist-mcp` crate: the server on `rmcp`, the gateway's trait, `list_connections`, `describe_schema` and `run_query` over `agent::check_read` and `Connection::run_agent_read`, the size cap on an answer (B.5), the refusal codes (B.8), the log's store (C.1) | Nothing. No setting, no transport and no command reaches it | F.1 in process, F.4 but its interface, F.5 in process |
 | **1b-2a. The backend's gateway** (built: `2026-10-14-mcp-server-1b-2a-gateway.md`) | `[connections.agents]` with access, rows and timeout, which makes the connections file version 4. `Command::Agents` and `Event::Agents`, the backend's gateway with its own sessions on each database, and the server started and stopped by the backend, with the log's retention. The SDK held out of the app's log | Nothing: no setting starts the server, and no transport reaches it | The connections file, the gateway on SQLite, PostgreSQL and MySQL, the backend's start and stop, the log's filter |
-| **1b-2b. Turning it on from the files, and stdio** | The `[mcp]` keys in `settings.toml`, and the app that starts and stops the server with them and sends the backend what agents may use. The endpoint, the key and the relay, `tabletist mcp`, on Linux and macOS. The keychain on macOS (A.1). The website's reference for the files | With `enabled = true` and an access written by hand, an agent connects over stdio and reads. Nothing in the app shows it yet, and nothing in the app turns it on | F.1 over stdio, F.5 over stdio, the settings file, `tests/cli.rs` |
+| **1b-2b. Turning it on from the files, and stdio** (built: `2026-10-14-mcp-server-1b-2b-stdio.md`) | The `[mcp]` keys in `settings.toml`, and the app that starts and stops the server with them and sends the backend what agents may use. The endpoint, the key and the relay, `tabletist mcp`, on Linux and macOS. The keychain on macOS (A.1). The website's reference for the files | With `enabled = true` and an access written by hand, an agent connects over stdio and reads. Nothing in the app shows it yet, and nothing in the app turns it on | F.1 over stdio, F.5 over stdio, the settings file, `tests/cli.rs` |
 | **1b-3. The Agents tab's frame** | The third kind of tab. The sidebar entry with its five states. The ways in: the key, `ga`, `:agents`, quick open. The three empty states, with **Turn on** and **Copy**. The keymap and the help | Open the tab in any state, see which state the connection is in, turn the server on from the tab | F.7: the entry's states, the ways in, one tab for a window, the empty states, nothing opening or focusing by itself |
 | **1b-4. What agents are doing** | `App::agents`. The sessions that are here and "Also:". The feed. **Stop session** and **Stop agents here**. The Log view as a list for this connection, with All connections. The notification for an agent that connects | See each agent and each call as it happens, stop one agent or all on a connection, read the log | F.7: one connection, the stops, closing the tab, the log's default. F.4's interface |
 | **1b-5. Letting agents in** | Settings › Agents, with the switch between panes. The Connections list's Agents column, its menu and Omarchy's box. The connection editor's Agents section, with **Allow agents…** and **Show Agents tab** pointing at them. The website: a new guide, and the reference pages | Everything Phase 1 draws, without a file edited by hand | The interface tests of Settings, Connections and the editor. The keymap. `docs/bin/check` |
@@ -1995,10 +2041,12 @@ can see what agents do and stop them.
   **Settings…** come in the fifth, with the editor's section and the pane
   they open. Omarchy's keys for them come with them.
 - **What may be released between runs.** After any of them. After the
-  second the server can be turned on only from a file, off by default and
-  not documented. After the third and fourth a person can turn it on and
+  second the server can be turned on only from a file, off by default. The
+  website's reference describes the files and `tabletist mcp` from then on
+  (agreed on 2026-10-14): a person who finds the keys in their file can
+  read what they do. After the third and fourth a person can turn it on and
   watch it, and still cannot open a connection to agents without editing a
-  file. The website says nothing of Agents until the fifth.
+  file. The website's guide to Agents waits for the fifth.
 - **Questions the runs meet.** Four are open and not marked as ones to
   wait for. Each is built as this plan has it unless it is answered before
   its run: what the log keeps of a call's values (18) in the first, which
````

- [ ] **Step 2: Check them**

Run: `docs/bin/check`, and `grep -c $'\u2014' docs/superpowers/plans/2026-10-13-mcp-server.md docs/_reference/settings-and-files.md` (an em dash, written so that this plan holds none)

Expected: the check ends `HTML-Proofer finished successfully.`, and both counts are `0`.

- [ ] **Step 3: Commit**

```bash
git add docs/_reference/settings-and-files.md docs/superpowers/plans/2026-10-13-mcp-server.md
git commit -m "Say what the [mcp] keys and tabletist mcp do, and what the run built"
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

Expected: all clean, and every test passes. On 2026-10-14, with both servers up: 23 test binaries, 3,395 passed, none failed, 22 ignored.

- [ ] **By hand, on Linux**, with a copy of the Bookshop file as a saved connection set to `access = "read"` and `[mcp] enabled = true`: add `tabletist mcp` to Claude Code (`claude mcp add tabletist -- tabletist mcp`), ask it to list the connections and count the books, then turn `enabled` off in the file and see the next call answered that Tabletist is not running.

- [ ] **Open the pull request.** `packaging.yml` starts with it, since `Cargo.toml` and `Cargo.lock` changed. The description says what was only type-checked here (macOS, Windows), and that the keychain's test runs on macOS alone.

## Self-review

- **Against the master plan's row for this run** (G): the `[mcp]` keys (Task 1), the app that starts and stops the server and sends it what agents may use (Task 6), the endpoint, the key and the relay on Linux and macOS (Tasks 2, 4 and 5), the keychain on macOS (Task 3), the website's reference for the files (Task 7). `listen` and `notify` are left to the runs that use them, and the master plan says so.
- **Against A.2.** The endpoint for each platform but Windows, 0700 and 0600, a random key that is new each start and that the relay sends first, the relay before the logger and the window with nothing on stdout but the protocol, two seconds of tries, the error that answers the first request and exits 1, no launch. Where it went otherwise, agreed: the key beside the socket, and the app's answer to the relay's line.
- **Against A.1 and E.** No prompt for an agent on any system: Linux since the first part, macOS here. A program of the user's that reaches the socket without the key is refused, takes no agent's place, and cannot fill the app's descriptors. Another user cannot reach the socket.
- **Against C.1 and C.2.** The two keys with their defaults and ranges. C.2's check is moved to the first control that turns agents on, agreed, and the plan says what that leaves open until then.
- **Reviewed before it was written.** A reviewer was given the draft and asked to break it, with tests of its own. It found these, each fixed above, with a test where one can be written: a relay that connected the moment the socket was there read an ended start's key (50 times in 50); a relay started with the few variables an MCP client passes looked in another directory than the app; the version a relay sent went into the app's log as it was, so that any program could write lines there; a second Tabletist of another version set aside the log the first had open; connections that said nothing took every agent's place and could hold thousands of descriptors; a relay greeting as the server stopped was answered "ok"; and on Windows agents turned on said nothing, a closed stderr could panic the relay, and the keychain's answer covers more than a locked keychain, which `quiet_error` now says. What held: the inode check, the modes and the refusal of a directory someone else made, the hello's bounds, the key's comparison, the turns of `Start`, `Stop` and `LogDays`, the policy sent on every path through `save_connections` and on `trust`, the settings' round trip, the relay's stdout and its ends, the Windows paths, and the keychain switch's single thread.
- **Names.** `mcp_enabled`, `mcp_log_days`, `agents_dir`, `agents_log_file`, `Hello`, `Answer`, `AVAILABLE`, `NOT_RUNNING`, `serve_agents`, `send_agents_policy`, `served_by_another`, `read_asking_nothing`, `quiet_error`, `LogDays` are spelled the same in every task.
- **Every commit stands.** After Task 1 the keys are read and written, and nothing acts on them. After Task 2 the paths and the lines are there, and nothing uses them. After Task 3 every system's quiet read asks nothing. After Task 4 the backend can listen, and the app sends it no directory. After Task 5 `tabletist mcp` reaches a backend that listens, and the app starts none. After Task 6 the settings start the server.

## After the run

Run on 2026-10-14, inline, as written: every diff applied as it stands, every step's run said what the step says, one commit for each task. The whole of it at the end, with both servers up: fmt and clippy clean, 23 test binaries, 3,395 passed, none failed, 22 ignored, and the documentation without a warning.

Not done here, and left for the review of the pull request:

- **The by-hand check with Claude Code** needs the app's window, and no session here has a display. It is Igor's to run once, as "After the last task" says.
- **macOS and Windows** run in CI alone. Task 3's test is the first that runs on macOS only.

## After the pull request's review

The review found that a start which failed was said twice over: the backend began its reason with "agents could not reach it: ", and the app put "Agents could not be served: " before that. The backend now says the reason alone (`endpoint::ANOTHER_SERVES`, or what `listen` said), and the app's test is of the reason the backend sends.
