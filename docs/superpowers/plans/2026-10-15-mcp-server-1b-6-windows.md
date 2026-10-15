# MCP server, Phase 1b, run 6: Windows' way in. Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Agents reach Tabletist on Windows as they do on Linux and macOS: the app listens on a named pipe, `tabletist mcp` reaches it from the installed GUI program, the command Settings copies names the installed program as Windows' shells read it, a new agent flashes the taskbar button, and everything of Agents shows. On every system, the app and the relay now prove to each other that each holds the key, and neither sends it.

**Architecture:** `src/backend/agents/listen.rs` keeps one lock, one key and one greeting for both endpoints; `accept` is generic over a private `Endpoint` trait, which the Unix socket's listener and, on Windows, a `Pipe` (the instance that waits, and the name the next one takes) implement. Each start makes a pipe of a new random name (`endpoint::new_pipe`), with `first_pipe_instance` and `reject_remote_clients`, and writes the name to `mcp.pipe` beside `mcp.key`. The greeting is a challenge and an answer each way (`endpoint::prove`, an HMAC-SHA-256 under the key): the relay tells nothing, not its working directory nor the agent's requests, to what cannot prove the key. The relay (`src/relay.rs`) reads the pipe's name, checks it (`endpoint::is_pipe`), and copies bytes as before; a pipe has no half-close, so there the end of stdin ends the session. `endpoint::AVAILABLE` turns everything of Agents on where it is `true`, on Windows from Task 3.

**Tech Stack:** Rust 2024, tokio's named pipes, `hmac` and `sha2` (in the tree through russh: two lines of `Cargo.lock`, nothing new built).

---

## Where this sits

Phase 1b of `docs/superpowers/plans/2026-10-13-mcp-server.md` is built on Linux and macOS (section G, nine runs). This run is G's "1b on Windows: the way in", the last of Phase 1b (question 14: Windows follows by one run).

## Before you start

- Work on a branch of `main`. One commit for each task, in the order below. Every commit passes `cargo fmt --all --check`, `cargo clippy --locked --workspace --all-targets -- -D warnings` and `cargo test --locked --workspace --all-targets`.
- On Igor's machine `cargo` is `~/.cargo/bin/cargo` (the mise shim fails), and a build in a worktree needs `CARGO_TARGET_DIR` set to that worktree's `target/`.
- Read `AGENTS.md`: no `unsafe` (the workspace forbids it), and a key is spelled in the keymap and nowhere else.
- The diffs below apply with `git apply`. Run `cargo fmt --all` before each commit: the code here is already formatted.
- **Windows is not built here.** The workspace does not cross-compile on Linux (ring's build script). Its Windows code runs on CI's `windows-latest` runner, the pull request's and before it the draft's (below).
- Task 1 adds `hmac` and `sha2` to `Cargo.toml`. Their versions are the ones russh builds: `Cargo.lock` gains the two names in `tabletist`'s list and nothing else. Check with `cargo metadata --locked`.

## What was run before this plan was written

Every task's state was built on Linux, passed `cargo fmt --check` and clippy with `-D warnings`, and its tests failed and then passed as each step says. The last state passed `cargo test --locked --workspace --all-targets` (3,487 tests, none failed), `RUSTDOCFLAGS='-D warnings' cargo doc --locked --workspace --no-deps` and `docs/bin/check`.

The draft was pushed to a scratch branch and CI was run on it by hand (agreed on 2026-10-15), three times: the first found one fault, a test that asked too soon whether the old pipe was gone; the second passed every job, with the relay's tests on Windows' release build in 18 minutes; the third, on the tree this plan ends in, passed every job: on Windows the whole workspace (the lib's 2,499 tests among them, `relay::tests::what_cannot_prove_it_holds_the_key_is_told_nothing` and `backend::agents::listen::tests::each_start_names_a_new_pipe_and_a_stop_leaves_a_later_starts_name` with them), then `a_release_build_on_windows_is_a_gui_program` and the three `mcp::` tests on the release build, in a job of 38 minutes; macOS and Linux all of theirs. The scratch branch is deleted. Not run: the states before the last on Windows, and an agent by hand on Windows.

## What this run settles

| Question | Decision | Why |
|---|---|---|
| How the plan's Windows code is checked | A scratch branch with the draft, and CI run on it by hand, then deleted | Agreed on 2026-10-15. Nothing builds for Windows on Linux here |
| The pipe's name | New and random for each start, written to `mcp.pipe` beside the key, after the one an ended start left is removed. The relay takes only a name of the form `new_pipe` makes | Agreed on 2026-10-15. A fixed name per user can be made first by another account. A name read back is never another computer's pipe |
| Who proves what | Each side proves it holds the key with an HMAC-SHA-256 of the other's challenge, its side's name in it, before it says anything else; the key is never sent. On every system | Found by the review: pipe names can be listed, and a name the app no longer holds, as after a crash, can be made by another account. A relay that sent the key, or an agent's requests, to it would have handed over both |
| The relay from the GUI binary | Kept: it works. CI's Windows job runs the relay's tests on a release build, and a test reads that build to be sure it is a GUI program | Agreed on 2026-10-15. The master plan's fallback, a console target, is not needed |
| The end of the agent's stdin on Windows | It ends the relay and the session | A pipe cannot be closed one way. A client closes stdin once it has its answers |
| The command on Windows | The installed program by its whole path, in double quotes where it has more than letters, digits and `\/._-:` | The installer puts it in `%LOCALAPPDATA%\Programs\Tabletist`, which is not on the `PATH`. Double quotes keep a space, a comma, a bracket and an apostrophe for the Command Prompt and PowerShell alike |
| A new agent on Windows | The taskbar button flashes | The master plan's row. A toast needs an app identity the installer does not register |
| Another account that floods the pipe | Left: it can keep agents out while it does, and never gets the key | Only the pipe's own access list would keep it out, and that takes `unsafe` code |
| CI's time | The test job may take 60 minutes | Windows builds the app twice: its job took 32 minutes with a warm cache |

### What this run leaves out

- An access list of the pipe's own, which would keep other accounts that are logged in at the same time from opening it: it takes `unsafe` code. Until then such an account can keep agents from reaching the app, and never gets the key.
- HTTP (1d), and anything else of Phase 1c, 2 or 3.

## File structure

| File | What changes |
|---|---|
| `src/endpoint.rs` | The greeting's lines (`Hello` with a challenge, `Answer::Prove`, `Reply`), `prove`, `new_nonce`, `NOT_PROVEN`; the pipe's name (`PIPE`, `new_pipe`, `is_pipe`); `shell_word` by system (`posix_word`, `windows_word`); `AVAILABLE` |
| `src/relay.rs` | The greeting; the pipe on Windows; `pump` over `tokio::io::split` |
| `src/backend/agents/listen.rs` | The pipe beside the socket, through the `Endpoint` trait; the greeting; `mcp.pipe` |
| `src/backend/agents.rs` | The endpoint on Windows too |
| `Cargo.toml`, `Cargo.lock` | `hmac` and `sha2` |
| `tests/cli.rs` | The relay's tests on Windows; a release build on Windows is a GUI program |
| `.github/workflows/ci.yml` | The relay's tests on Windows' release build; a longer limit for the job |
| `src/app.rs`, `src/settings.rs`, `src/ui/agents.rs`, `src/ui/agents_tests.rs`, `src/ui/mod.rs`, `src/ui/settings/agents.rs` | The taskbar's flash; the tests of Agents on Windows too; Credential Manager in Settings › Agents |
| `docs/_guide/agents.md`, `docs/_reference/*.md` | Windows |
| `docs/superpowers/plans/2026-10-13-mcp-server.md` | What this run built |

---

### Task 1: Reaching the app on Windows, each side proving it holds the key

- The greeting, on every system: the relay sends `Hello { version, challenge }`; the app answers `Answer::Prove { proof, challenge }`, its `prove(key, Side::App, …)` of the relay's challenge and a challenge of its own; the relay reads the key then, checks the proof, and stops with `NOT_PROVEN` where it is wrong, having said nothing more; else it sends `Reply { proof, cwd }`; the app checks it and answers `Answer::Ok` or refuses. A session is counted once its relay has proved the key.
- The pipe, on Windows: `listen_on_pipe` removes an `mcp.pipe` an ended start left, writes the new key, makes the first instance of a new pipe (`first_pipe_instance`, `reject_remote_clients`), then writes its name to `mcp.pipe`. `Pipe::next` waits for a relay, puts the next instance in place, and gives the connected one to `greet`. On a stop the key and the name go while they are this start's; the pipe goes with the task that holds its waiting instance, a moment later.
- The relay on Windows reads `mcp.pipe`, takes only a name `is_pipe` accepts, and tries for two seconds, as on a socket. `pump` copies over `tokio::io::split`; on Windows the end of stdin ends the relay.
- `tests/cli.rs` runs the relay's tests on Windows too, and checks that Windows' release build is a GUI program; CI's Windows job runs them on that build.

**Files:**
- Modify: `src/endpoint.rs`, `src/relay.rs`, `src/backend/agents/listen.rs`, `src/backend/agents.rs`, `Cargo.toml`, `Cargo.lock`, `tests/cli.rs`, `.github/workflows/ci.yml`

- [ ] **Step 1: Write the failing tests**

````diff
diff --git a/.github/workflows/ci.yml b/.github/workflows/ci.yml
index fdd40f5..0d28f8b 100644
--- a/.github/workflows/ci.yml
+++ b/.github/workflows/ci.yml
@@ -41,7 +41,9 @@ jobs:
   test:
     name: test (${{ matrix.os }})
     runs-on: ${{ matrix.os }}
-    timeout-minutes: 45
+    # Windows builds the app a second time, in release, for the relay's
+    # step: about 30 minutes with a warm cache.
+    timeout-minutes: 60
     strategy:
       fail-fast: false
       matrix:
@@ -56,6 +58,13 @@ jobs:
           sudo apt-get install -y libxkbcommon-dev libwayland-dev libgl1-mesa-dev
       - uses: Swatinem/rust-cache@v2
       - run: cargo test --locked --workspace --all-targets
+      # The relay is the app's own program, which a release build on
+      # Windows makes a GUI program: an agent must reach the app through
+      # that one too.
+      - name: The relay from the GUI build (Windows)
+        if: runner.os == 'Windows'
+        timeout-minutes: 30
+        run: cargo test --locked --release --test cli -- "mcp::" a_release_build_on_windows_is_a_gui_program
       - name: Native keyring round trip
         if: runner.os == 'Windows' || runner.os == 'macOS'
         timeout-minutes: 2
diff --git a/Cargo.lock b/Cargo.lock
index a945232..9879d52 100644
--- a/Cargo.lock
+++ b/Cargo.lock
@@ -5686,6 +5686,7 @@ dependencies = [
  "fastframe-log",
  "fastframe-text",
  "fastframe-theme",
+ "hmac",
  "image",
  "jiff",
  "keyring-core",
@@ -5699,6 +5700,7 @@ dependencies = [
  "security-framework",
  "serde",
  "serde_json",
+ "sha2 0.11.0",
  "skrifa",
  "tabletist-appkit",
  "tabletist-db",
diff --git a/Cargo.toml b/Cargo.toml
index cb9092d..1a84f5a 100644
--- a/Cargo.toml
+++ b/Cargo.toml
@@ -87,6 +87,11 @@ tokio = { version = "1", features = ["rt-multi-thread", "sync", "macros", "time"
 rfd = "0.17"
 # Stable ids for saved connections, so renaming one keeps its identity.
 uuid = { version = "1", features = ["v4"] }
+# The proofs the app and `tabletist mcp` give each other that each holds
+# the key, before either says anything else (src/endpoint.rs). In the tree
+# through russh, on every system: nothing new is built.
+hmac = { version = "0.13", default-features = false }
+sha2 = { version = "0.11", default-features = false }
 
 # One protected-store API for saved passwords; each platform's native
 # provider below. Passwords never go into connections.toml.
diff --git a/src/backend/agents.rs b/src/backend/agents.rs
index 540a3d9..0b8c7eb 100644
--- a/src/backend/agents.rs
+++ b/src/backend/agents.rs
@@ -1605,7 +1605,7 @@ mod tests {
     /// Another Tabletist serves agents: this one's server does not start,
     /// and does not open the log the other has open, which one of another
     /// version would set aside.
-    #[cfg(unix)]
+    #[cfg(any(unix, windows))]
     #[test]
     fn a_start_where_another_tabletist_serves_agents_leaves_its_log_alone() {
         let dir = tempfile::tempdir().unwrap();
diff --git a/src/backend/agents/listen.rs b/src/backend/agents/listen.rs
index b34c8e3..6e2c9bb 100644
--- a/src/backend/agents/listen.rs
+++ b/src/backend/agents/listen.rs
@@ -294,28 +294,87 @@ mod tests {
         Server::new(AppGateway::new(Keyring::memory()), log, "test")
     }
 
-    /// Says `line` to the endpoint in `dir` as a relay does, and reads the
-    /// app's answer.
-    async fn say(dir: &Path, line: &[u8]) -> (UnixStream, Answer) {
-        let mut stream = UnixStream::connect(dir.join(endpoint::SOCKET))
+    /// What a relay connects with.
+    #[cfg(unix)]
+    type Relay = tokio::net::UnixStream;
+    #[cfg(windows)]
+    type Relay = tokio::net::windows::named_pipe::NamedPipeClient;
+
+    /// Connects to the endpoint in `dir` as a relay does.
+    #[cfg(unix)]
+    async fn connect(dir: &Path) -> Relay {
+        tokio::net::UnixStream::connect(dir.join(endpoint::SOCKET))
             .await
-            .unwrap();
-        stream.write_all(line).await.unwrap();
-        let answer = endpoint::read_line(&mut stream, endpoint::MAX_LINE)
+            .unwrap()
+    }
+
+    /// Connects to the endpoint in `dir` as a relay does: by the name the
+    /// server wrote, again while its instances are busy.
+    #[cfg(windows)]
+    async fn connect(dir: &Path) -> Relay {
+        let name = std::fs::read_to_string(dir.join(endpoint::PIPE)).unwrap();
+        assert!(endpoint::is_pipe(&name), "{name}");
+        let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(5);
+        loop {
+            match tokio::net::windows::named_pipe::ClientOptions::new().open(&name) {
+                Ok(client) => return client,
+                Err(error) if tokio::time::Instant::now() < deadline => {
+                    let _ = error;
+                    tokio::time::sleep(std::time::Duration::from_millis(10)).await;
+                }
+                Err(error) => panic!("{error}"),
+            }
+        }
+    }
+
+    /// The app's next line.
+    async fn answer(stream: &mut Relay) -> Answer {
+        let line = endpoint::read_line(stream, endpoint::MAX_LINE)
             .await
             .unwrap()
             .unwrap();
-        (stream, serde_json::from_slice(&answer).unwrap())
+        serde_json::from_slice(&line).unwrap()
     }
 
-    fn hello(key: &str) -> Vec<u8> {
-        endpoint::line_of(&Hello {
-            key: key.into(),
+    /// Says `line` to the endpoint in `dir`, and reads the app's answer.
+    async fn say_line(dir: &Path, line: &[u8]) -> (Relay, Answer) {
+        let mut stream = connect(dir).await;
+        stream.write_all(line).await.unwrap();
+        let answer = answer(&mut stream).await;
+        (stream, answer)
+    }
+
+    /// Greets the endpoint in `dir` as a relay does, as one that holds
+    /// `key`, and reads the app's last answer. An app that lets it in has
+    /// proved it holds that key.
+    async fn say(dir: &Path, key: &str) -> (Relay, Answer) {
+        let challenge = endpoint::new_nonce();
+        let hello = Hello {
             version: "test".into(),
+            challenge: challenge.clone(),
+        };
+        let (mut stream, first) = say_line(dir, &endpoint::line_of(&hello)).await;
+        let Answer::Prove {
+            proof,
+            challenge: theirs,
+        } = first
+        else {
+            return (stream, first);
+        };
+        let proven = endpoint::same_key(&proof, &endpoint::prove(key, Side::App, &challenge));
+        let reply = Reply {
+            proof: endpoint::prove(key, Side::Relay, &theirs),
             cwd: Some("/home/ada/src/bookshop".into()),
-        })
+        };
+        stream.write_all(&endpoint::line_of(&reply)).await.unwrap();
+        let last = answer(&mut stream).await;
+        if last == Answer::Ok {
+            assert!(proven, "the app let a relay in without proving its key");
+        }
+        (stream, last)
     }
 
+    #[cfg(unix)]
     fn mode(path: &Path) -> u32 {
         std::fs::symlink_metadata(path)
             .unwrap()
@@ -331,11 +390,14 @@ mod tests {
         let server = server(dir.path());
         let listening = listen(&at, server.clone()).unwrap();
         // Private to the user, the folder and what is in it.
-        assert_eq!(mode(&at), 0o700);
-        assert_eq!(mode(&at.join(endpoint::SOCKET)), 0o600);
-        assert_eq!(mode(&at.join(endpoint::KEY)), 0o600);
+        #[cfg(unix)]
+        {
+            assert_eq!(mode(&at), 0o700);
+            assert_eq!(mode(&at.join(endpoint::SOCKET)), 0o600);
+            assert_eq!(mode(&at.join(endpoint::KEY)), 0o600);
+        }
         let key = std::fs::read_to_string(at.join(endpoint::KEY)).unwrap();
-        let (stream, answer) = say(&at, &hello(&key)).await;
+        let (stream, answer) = say(&at, &key).await;
         assert_eq!(answer, Answer::Ok);
         // From then on the stream is the session's, named for its project.
         let (read, mut write) = tokio::io::split(stream);
@@ -361,17 +423,19 @@ mod tests {
         assert_eq!(sessions.len(), 1);
         assert_eq!(sessions[0].project.as_deref(), Some("bookshop"));
         // A key that is not the server's, and a line that is no hello.
-        for line in [hello("not-the-key"), b"hello\n".to_vec()] {
-            let (_, answer) = say(&at, &line).await;
-            assert!(matches!(answer, Answer::Refused(_)), "{answer:?}");
-        }
+        let (_, answer) = say(&at, "not-the-key").await;
+        assert!(matches!(answer, Answer::Refused(_)), "{answer:?}");
+        let (_, answer) = say_line(&at, b"hello\n").await;
+        assert!(matches!(answer, Answer::Refused(_)), "{answer:?}");
         assert_eq!(server.open_sessions().len(), 1);
-        // Stopped: the session ends, and the socket and the key are gone.
+        // Stopped: the session ends, and the socket, the pipe's name and
+        // the key are gone.
         server.stop();
         drop(listening);
         assert_eq!(lines.next_line().await.unwrap(), None);
-        assert!(!at.join(endpoint::SOCKET).exists());
-        assert!(!at.join(endpoint::KEY).exists());
+        for file in [endpoint::SOCKET, endpoint::PIPE, endpoint::KEY] {
+            assert!(!at.join(file).exists(), "{file}");
+        }
     }
 
     #[tokio::test]
@@ -389,18 +453,20 @@ mod tests {
             std::fs::read_to_string(at.join(endpoint::KEY)).unwrap(),
             key
         );
-        let (_, answer) = say(&at, &hello(&key)).await;
+        let (_, answer) = say(&at, &key).await;
         assert_eq!(answer, Answer::Ok);
         drop(first);
         assert!(!served_by_another(&at));
         // A socket left by a Tabletist that ended: nothing answers it.
+        #[cfg(unix)]
         drop(std::os::unix::net::UnixListener::bind(at.join(endpoint::SOCKET)).unwrap());
         let _second = listen(&at, server(dir.path())).unwrap();
         let key = std::fs::read_to_string(at.join(endpoint::KEY)).unwrap();
-        let (_, answer) = say(&at, &hello(&key)).await;
+        let (_, answer) = say(&at, &key).await;
         assert_eq!(answer, Answer::Ok);
     }
 
+    #[cfg(unix)]
     #[tokio::test]
     async fn what_is_not_a_socket_is_not_removed_nor_what_another_put_in_its_place() {
         let dir = tempfile::tempdir().unwrap();
@@ -430,14 +496,10 @@ mod tests {
         let _listening = listen(&at, server(dir.path())).unwrap();
         let mut silent = Vec::new();
         for _ in 0..MAX_SESSIONS + 8 {
-            silent.push(
-                UnixStream::connect(at.join(endpoint::SOCKET))
-                    .await
-                    .unwrap(),
-            );
+            silent.push(connect(&at).await);
         }
         let key = std::fs::read_to_string(at.join(endpoint::KEY)).unwrap();
-        let (_, answer) = say(&at, &hello(&key)).await;
+        let (_, answer) = say(&at, &key).await;
         assert_eq!(answer, Answer::Ok);
     }
 
@@ -446,9 +508,7 @@ mod tests {
         let dir = tempfile::tempdir().unwrap();
         let at = dir.path().join("run");
         let listening = listen(&at, server(dir.path())).unwrap();
-        let mut stream = UnixStream::connect(at.join(endpoint::SOCKET))
-            .await
-            .unwrap();
+        let mut stream = connect(&at).await;
         // Taken, and waiting for its hello.
         tokio::time::sleep(std::time::Duration::from_millis(50)).await;
         drop(listening);
@@ -462,6 +522,7 @@ mod tests {
         );
     }
 
+    #[cfg(unix)]
     #[tokio::test]
     async fn a_key_left_by_an_ended_start_is_replaced_before_anyone_can_connect() {
         let dir = tempfile::tempdir().unwrap();
@@ -489,6 +550,49 @@ mod tests {
         );
     }
 
+    #[cfg(windows)]
+    #[tokio::test]
+    async fn each_start_names_a_new_pipe_and_a_stop_leaves_a_later_starts_name() {
+        let dir = tempfile::tempdir().unwrap();
+        let at = dir.path().join("run");
+        std::fs::create_dir_all(&at).unwrap();
+        // What an ended start left: replaced before anyone can connect.
+        std::fs::write(at.join(endpoint::PIPE), "an ended start's").unwrap();
+        std::fs::write(at.join(endpoint::KEY), "an ended start's").unwrap();
+        let first = listen(&at, server(dir.path())).unwrap();
+        let named = std::fs::read_to_string(at.join(endpoint::PIPE)).unwrap();
+        assert!(endpoint::is_pipe(&named), "{named}");
+        let key = std::fs::read_to_string(at.join(endpoint::KEY)).unwrap();
+        let (_, answer) = say(&at, &key).await;
+        assert_eq!(answer, Answer::Ok);
+        drop(first);
+        assert!(!at.join(endpoint::PIPE).exists());
+        // The next start's pipe has a name of its own.
+        let second = listen(&at, server(dir.path())).unwrap();
+        let renamed = std::fs::read_to_string(at.join(endpoint::PIPE)).unwrap();
+        assert_ne!(renamed, named);
+        // The pipe of a server that stopped goes with it, once the task that
+        // held its waiting instance has ended: a moment after the stop.
+        let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(2);
+        while tokio::net::windows::named_pipe::ClientOptions::new()
+            .open(&named)
+            .is_ok()
+        {
+            assert!(
+                tokio::time::Instant::now() < deadline,
+                "the old pipe is still there"
+            );
+            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
+        }
+        // A name a later start wrote is that start's: the stop leaves it.
+        std::fs::write(at.join(endpoint::PIPE), "a later start's").unwrap();
+        drop(second);
+        assert_eq!(
+            std::fs::read_to_string(at.join(endpoint::PIPE)).unwrap(),
+            "a later start's"
+        );
+    }
+
     #[test]
     fn a_session_past_the_most_is_counted_and_refused() {
         let open = Arc::new(AtomicUsize::new(0));
diff --git a/src/endpoint.rs b/src/endpoint.rs
index f672771..cbd8e36 100644
--- a/src/endpoint.rs
+++ b/src/endpoint.rs
@@ -326,9 +326,8 @@ mod tests {
     #[test]
     fn the_lines_are_json_and_an_answer_names_its_kind() {
         let hello = Hello {
-            key: "k".into(),
             version: "1.0.0".into(),
-            cwd: Some("/home/ada".into()),
+            challenge: new_nonce(),
         };
         let line = line_of(&hello);
         assert!(line.ends_with(b"\n"));
@@ -341,13 +340,51 @@ mod tests {
             line_of(&Answer::Refused("no".into())),
             b"{\"refused\":\"no\"}\n"
         );
+        assert_eq!(
+            line_of(&Answer::Prove {
+                proof: "p".into(),
+                challenge: "c".into()
+            }),
+            b"{\"prove\":{\"proof\":\"p\",\"challenge\":\"c\"}}\n"
+        );
         // A newer relay may say more: what this one does not know is
         // passed over.
-        let newer: Hello =
-            serde_json::from_str(r#"{"key":"k","version":"9","cwd":null,"shell":"zsh"}"#).unwrap();
+        let newer: Reply =
+            serde_json::from_str(r#"{"proof":"p","cwd":null,"shell":"zsh"}"#).unwrap();
         assert_eq!(newer.cwd, None);
     }
 
+    #[test]
+    fn a_proof_is_the_keys_and_the_sides_and_the_challenges() {
+        let key = new_key();
+        let challenge = new_nonce();
+        let proof = prove(&key, Side::App, &challenge);
+        // HMAC-SHA-256, as hex.
+        assert_eq!(proof.len(), 64);
+        assert!(proof.bytes().all(|b| b.is_ascii_hexdigit()), "{proof}");
+        assert_eq!(prove(&key, Side::App, &challenge), proof);
+        // Another key, another side or another challenge: another proof.
+        // The app's cannot be handed back as the relay's.
+        assert_ne!(prove(&new_key(), Side::App, &challenge), proof);
+        assert_ne!(prove(&key, Side::Relay, &challenge), proof);
+        assert_ne!(prove(&key, Side::App, &new_nonce()), proof);
+        // RFC 4231's second case, under the app's side's name: the HMAC is
+        // the standard one.
+        use hmac::{KeyInit, Mac};
+        let mut mac = hmac::Hmac::<sha2::Sha256>::new_from_slice(b"Jefe").unwrap();
+        mac.update(b"what do ya want for nothing?");
+        let standard: String = mac
+            .finalize()
+            .into_bytes()
+            .iter()
+            .map(|byte| format!("{byte:02x}"))
+            .collect();
+        assert_eq!(
+            standard,
+            "5bdcc146bf60754e6a042426089575c75a003f089d2739839dec58b964ec3843"
+        );
+    }
+
     #[test]
     fn the_command_starts_this_program_and_quotes_its_path_where_it_must() {
         use std::path::Path;
@@ -362,6 +399,7 @@ mod tests {
             relay_for(Some(bundle), None),
             "/Applications/Tabletist.app/Contents/MacOS/tabletist mcp"
         );
+        // Quoted where it must be, as this system's shell reads it.
         let spaced = Path::new("/Users/ada/My Apps/Tabletist's/tabletist");
         assert_eq!(
             relay_for(Some(spaced), None),
@@ -375,6 +413,25 @@ mod tests {
         );
     }
 
+    #[test]
+    fn a_pipes_name_is_new_each_time_and_one_read_back_is_checked() {
+        let name = new_pipe();
+        assert!(name.starts_with(r"\\.\pipe\tabletist-mcp-"), "{name}");
+        assert!(is_pipe(&name));
+        assert_ne!(name, new_pipe());
+        // Nothing else is taken for one: another computer's pipe least of
+        // all, which the key would be sent to.
+        for other in [
+            r"\\server\pipe\tabletist-mcp-0123456789abcdef0123456789abcdef",
+            r"\\.\pipe\tabletist-mcp-",
+            r"\\.\pipe\tabletist-mcp-0123456789abcdef0123456789abcdeg",
+            r"\\.\pipe\tabletist-mcp-0123456789abcdef0123456789abcdef0",
+            "",
+        ] {
+            assert!(!is_pipe(other), "{other}");
+        }
+    }
+
     #[test]
     fn the_command_is_ready_once_it_is_worked_out() {
         let command = agent_command();
diff --git a/src/relay.rs b/src/relay.rs
index 26ddcad..e69fe54 100644
--- a/src/relay.rs
+++ b/src/relay.rs
@@ -194,6 +194,64 @@ fn refusal(id: serde_json::Value, said: &str) -> Vec<u8> {
 mod tests {
     use super::*;
 
+    /// Something at the endpoint in `dir` that is not the app: it does not
+    /// hold the key in `dir`. Returns what it reads after the relay's
+    /// hello, once the relay has stopped.
+    #[cfg(any(unix, windows))]
+    async fn impostor(dir: &Path) -> tokio::task::JoinHandle<Vec<u8>> {
+        use tokio::io::{AsyncReadExt, AsyncWriteExt};
+        std::fs::create_dir_all(dir).unwrap();
+        std::fs::write(dir.join(endpoint::KEY), endpoint::new_key()).unwrap();
+        #[cfg(unix)]
+        let listener = tokio::net::UnixListener::bind(dir.join(endpoint::SOCKET)).unwrap();
+        // A pipe of the name a stopped app left in its file, which another
+        // account can make once the app's is gone.
+        #[cfg(windows)]
+        let pipe = {
+            let name = endpoint::new_pipe();
+            std::fs::write(dir.join(endpoint::PIPE), &name).unwrap();
+            tokio::net::windows::named_pipe::ServerOptions::new()
+                .create(&name)
+                .unwrap()
+        };
+        tokio::spawn(async move {
+            #[cfg(unix)]
+            let (mut stream, _) = listener.accept().await.unwrap();
+            #[cfg(windows)]
+            let mut stream = {
+                pipe.connect().await.unwrap();
+                pipe
+            };
+            let hello = endpoint::read_line(&mut stream, endpoint::MAX_LINE)
+                .await
+                .unwrap()
+                .unwrap();
+            let hello: endpoint::Hello = serde_json::from_slice(&hello).unwrap();
+            let guess = endpoint::Answer::Prove {
+                proof: endpoint::prove(&endpoint::new_key(), Side::App, &hello.challenge),
+                challenge: endpoint::new_nonce(),
+            };
+            stream.write_all(&endpoint::line_of(&guess)).await.unwrap();
+            let mut rest = Vec::new();
+            let _ = stream.read_to_end(&mut rest).await;
+            rest
+        })
+    }
+
+    #[cfg(any(unix, windows))]
+    #[tokio::test]
+    async fn what_cannot_prove_it_holds_the_key_is_told_nothing() {
+        let dir = tempfile::tempdir().unwrap();
+        let at = dir.path().join("run");
+        let told = impostor(&at).await;
+        assert_eq!(
+            reach(&at).await.err().as_deref(),
+            Some(endpoint::NOT_PROVEN)
+        );
+        // No proof of the relay's, no working directory, no request.
+        assert_eq!(told.await.unwrap(), Vec::<u8>::new());
+    }
+
     #[test]
     fn only_a_request_is_answered_and_with_its_own_id() {
         assert_eq!(
diff --git a/tests/cli.rs b/tests/cli.rs
index 42a82cb..6783114 100644
--- a/tests/cli.rs
+++ b/tests/cli.rs
@@ -35,9 +35,28 @@ fn demo_flags_require_demo() {
     );
 }
 
+/// A release build on Windows is a GUI program (`src/main.rs`), and the
+/// relay is that same program: CI runs the relay's tests below on it too,
+/// and this says that they ran on what a person installs.
+#[cfg(all(windows, not(debug_assertions)))]
+#[test]
+fn a_release_build_on_windows_is_a_gui_program() {
+    let exe = std::fs::read(env!("CARGO_BIN_EXE_tabletist")).unwrap();
+    let field = |at: usize, len: usize| &exe[at..at + len];
+    // Where the PE header starts, from the DOS header.
+    let pe = u32::from_le_bytes(field(0x3c, 4).try_into().unwrap()) as usize;
+    assert_eq!(field(pe, 4), b"PE\0\0");
+    // The optional header follows the signature and the 20 bytes of the
+    // file header; its subsystem is 68 bytes in, for 32 and 64 bits alike.
+    let subsystem = u16::from_le_bytes(field(pe + 24 + 68, 2).try_into().unwrap());
+    // IMAGE_SUBSYSTEM_WINDOWS_GUI.
+    assert_eq!(subsystem, 2);
+}
+
 /// `tabletist mcp`, the relay an agent starts, against the app's backend
-/// serving agents in a folder of the test's own.
-#[cfg(unix)]
+/// serving agents in a folder of the test's own: through a socket on Linux
+/// and macOS, a named pipe on Windows.
+#[cfg(any(unix, windows))]
 mod mcp {
     use std::io::{BufRead, BufReader, Read, Write};
     use std::path::{Path, PathBuf};
@@ -414,7 +433,10 @@ mod mcp {
         // Stopped, nothing answers: as if the app were not running.
         backend.send(Command::Agents(AgentsCommand::Stop));
         assert_eq!(agents_said(&mut backend), AgentsEvent::Stopped);
-        assert!(!endpoint.join("mcp.sock").exists());
+        // The key is the test's own now, and stays.
+        for file in ["mcp.sock", "mcp.pipe"] {
+            assert!(!endpoint.join(file).exists(), "{file}");
+        }
         let (message, _) = refused(&endpoint, &project);
         assert_eq!(message, tabletist::endpoint::NOT_RUNNING);
     }
````

- [ ] **Step 2: Run them to see that they do not build**

Run: `cargo test --locked --lib -- endpoint:: relay:: backend::agents::listen`

Expected: it does not build. The tests name what is not there yet; among the errors:

```text
error[E0425]: cannot find function `new_nonce` in module `endpoint`
error[E0425]: cannot find function `prove` in module `endpoint`
error[E0422]: cannot find struct, variant or union type `Reply` in this scope
error[E0425]: cannot find value `NOT_PROVEN` in module `endpoint`
error[E0560]: struct `endpoint::Hello` has no field named `challenge`
error[E0599]: no variant named `Prove` found for enum `endpoint::Answer`
error[E0425]: cannot find function `new_pipe` in this scope
error[E0425]: cannot find function `is_pipe` in this scope
error: could not compile `tabletist` (lib test) due to 35 previous errors
```

- [ ] **Step 3: The greeting and the pipe**

````diff
diff --git a/src/backend/agents.rs b/src/backend/agents.rs
index 0b8c7eb..986f136 100644
--- a/src/backend/agents.rs
+++ b/src/backend/agents.rs
@@ -27,7 +27,7 @@ use crate::env::Platform;
 use crate::model::SecretKind;
 use crate::secrets::{Keyring, password_account, ssh_account};
 
-#[cfg(unix)]
+#[cfg(any(unix, windows))]
 mod listen;
 #[cfg(target_os = "linux")]
 mod notify;
@@ -219,7 +219,7 @@ pub(super) struct Running {
     watching: tokio::task::JoinHandle<()>,
     /// Tells the app of each call as it ends.
     telling: tokio::task::JoinHandle<()>,
-    #[cfg(unix)]
+    #[cfg(any(unix, windows))]
     listening: Option<listen::Listening>,
 }
 
@@ -369,7 +369,7 @@ impl Agents {
                             return;
                         }
                     };
-                    #[cfg(not(unix))]
+                    #[cfg(not(any(unix, windows)))]
                     let () = listening;
                     let chores = tokio::spawn(chores(opened.log, gateway, log_days));
                     let watching = tokio::spawn(sessions(server.clone(), changes, outbox.clone()));
@@ -379,7 +379,7 @@ impl Agents {
                         chores,
                         watching,
                         telling,
-                        #[cfg(unix)]
+                        #[cfg(any(unix, windows))]
                         listening,
                     });
                     log::info!("the agent server started");
@@ -412,13 +412,13 @@ impl Running {
         self.chores.abort();
         self.watching.abort();
         self.telling.abort();
-        #[cfg(unix)]
+        #[cfg(any(unix, windows))]
         drop(self.listening);
     }
 }
 
 /// Listens in `endpoint` for `server`, where there is one.
-#[cfg(unix)]
+#[cfg(any(unix, windows))]
 fn listening(
     endpoint: Option<&std::path::Path>,
     server: &tabletist_mcp::Server<AppGateway>,
@@ -429,25 +429,25 @@ fn listening(
 }
 
 /// Whether another Tabletist serves agents in `dir`.
-#[cfg(unix)]
+#[cfg(any(unix, windows))]
 fn served_by_another(dir: &std::path::Path) -> bool {
     listen::served_by_another(dir)
 }
 
-/// Nothing serves agents here yet.
-#[cfg(not(unix))]
+/// Nothing serves agents here (`endpoint::AVAILABLE`).
+#[cfg(not(any(unix, windows)))]
 fn served_by_another(_dir: &std::path::Path) -> bool {
     false
 }
 
-/// Nothing listens here yet (`endpoint::AVAILABLE`).
-#[cfg(not(unix))]
+/// Nothing listens here (`endpoint::AVAILABLE`).
+#[cfg(not(any(unix, windows)))]
 fn listening(
     endpoint: Option<&std::path::Path>,
     _server: &tabletist_mcp::Server<AppGateway>,
 ) -> Result<(), String> {
     match endpoint {
-        Some(_) => Err("agents cannot reach Tabletist on this system yet".into()),
+        Some(_) => Err("agents cannot reach Tabletist on this system".into()),
         None => Ok(()),
     }
 }
diff --git a/src/backend/agents/listen.rs b/src/backend/agents/listen.rs
index 6e2c9bb..6066493 100644
--- a/src/backend/agents/listen.rs
+++ b/src/backend/agents/listen.rs
@@ -1,19 +1,23 @@
-//! The server's endpoint: the socket `tabletist mcp` connects to, and the
-//! key it must send first.
+//! The server's endpoint: the socket `tabletist mcp` connects to on Linux
+//! and macOS, the named pipe on Windows, and the key it must send first.
 
+#[cfg(unix)]
 use std::os::unix::fs::{FileTypeExt, MetadataExt, PermissionsExt};
 use std::path::{Path, PathBuf};
 use std::sync::Arc;
 use std::sync::atomic::{AtomicUsize, Ordering};
 
 use tabletist_mcp::{Origin, Server, Transport};
-use tokio::io::AsyncWriteExt;
-use tokio::net::{UnixListener, UnixStream};
+use tokio::io::{AsyncRead, AsyncWrite, AsyncWriteExt};
+#[cfg(unix)]
+use tokio::net::UnixListener;
+#[cfg(windows)]
+use tokio::net::windows::named_pipe::{NamedPipeServer, ServerOptions};
 use tokio::sync::{OwnedSemaphorePermit, Semaphore};
 use tokio_util::sync::CancellationToken;
 
 use super::AppGateway;
-use crate::endpoint::{self, Answer, Hello};
+use crate::endpoint::{self, Answer, Hello, Reply, Side};
 
 /// The most agents connected at once. One more is refused: a program of
 /// the user's that opens sessions without end runs into it, and the
@@ -41,37 +45,48 @@ impl Drop for Listening {
 
 /// The socket and the key the server made, removed when it stops: each
 /// only while it is the one it made, which a later start may have put
-/// another in the place of. The directory's lock is let go after them.
+/// another in the place of. On Windows the file that names the pipe, in
+/// the socket's place: the pipe goes with its last instance. The
+/// directory's lock is let go after them.
 struct Files {
+    #[cfg(unix)]
     socket: Option<(PathBuf, (u64, u64))>,
+    #[cfg(windows)]
+    pipe: Option<(PathBuf, String)>,
     key: (PathBuf, String),
     _locked: std::fs::File,
 }
 
 impl Drop for Files {
     fn drop(&mut self) {
+        #[cfg(unix)]
         if let Some((socket, made)) = &self.socket
             && std::fs::symlink_metadata(socket).is_ok_and(|now| (now.dev(), now.ino()) == *made)
         {
             let _ = std::fs::remove_file(socket);
         }
-        let (file, key) = &self.key;
-        if std::fs::read_to_string(file).is_ok_and(|there| there == *key) {
-            let _ = std::fs::remove_file(file);
+        #[cfg(windows)]
+        if let Some(named) = &self.pipe {
+            remove_if_still(named);
         }
+        remove_if_still(&self.key);
+    }
+}
+
+/// Removes `file` while it holds what this server wrote in it.
+fn remove_if_still((file, wrote): &(PathBuf, String)) {
+    if std::fs::read_to_string(file).is_ok_and(|there| there == *wrote) {
+        let _ = std::fs::remove_file(file);
     }
 }
 
 /// The directory's lock, opened, and made where there is none.
 fn lock_file(dir: &Path) -> std::io::Result<std::fs::File> {
-    use std::os::unix::fs::OpenOptionsExt;
-    std::fs::OpenOptions::new()
-        .read(true)
-        .write(true)
-        .create(true)
-        .truncate(false)
-        .mode(0o600)
-        .open(dir.join(endpoint::LOCK))
+    let mut options = std::fs::OpenOptions::new();
+    options.read(true).write(true).create(true).truncate(false);
+    #[cfg(unix)]
+    std::os::unix::fs::OpenOptionsExt::mode(&mut options, 0o600);
+    options.open(dir.join(endpoint::LOCK))
 }
 
 /// Whether another Tabletist is serving its agents in `dir`: it holds the
@@ -84,13 +99,16 @@ pub(super) fn served_by_another(dir: &Path) -> bool {
 }
 
 /// Listens in `dir` for `server`: makes the directory private, a new key,
-/// and the socket. Why it could not, for the person.
+/// and the socket, or on Windows the pipe. Why it could not, for the
+/// person.
 pub(super) fn listen(dir: &Path, server: Server<AppGateway>) -> Result<Listening, String> {
     let shown = dir.display();
     crate::util::create_private_dir(dir)
         .map_err(|error| format!("could not make the folder {shown}: {error}"))?;
     // Only its owner can change a folder's mode: one that someone else made
-    // there first is refused, not used.
+    // there first is refused, not used. On Windows the folder is in the
+    // user's own profile, which only they may change.
+    #[cfg(unix)]
     std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o700))
         .map_err(|error| format!("the folder {shown} is not yours to use: {error}"))?;
     // Held for as long as it listens: a second start stops here, before
@@ -111,6 +129,38 @@ pub(super) fn listen(dir: &Path, server: Server<AppGateway>) -> Result<Listening
             ));
         }
     }
+    #[cfg(unix)]
+    return listen_on_socket(dir, locked, server);
+    #[cfg(windows)]
+    return listen_on_pipe(dir, locked, server);
+}
+
+/// The key, new for this start, written before anything is listened on: a
+/// relay that connects the moment it can reads the key of this start, not
+/// one an ended start left.
+fn new_key(dir: &Path, locked: std::fs::File) -> Result<(Files, String), String> {
+    let key = endpoint::new_key();
+    let key_file = dir.join(endpoint::KEY);
+    let files = Files {
+        #[cfg(unix)]
+        socket: None,
+        #[cfg(windows)]
+        pipe: None,
+        key: (key_file.clone(), key.clone()),
+        _locked: locked,
+    };
+    crate::util::write_atomic(&key_file, key.as_bytes())
+        .map_err(|error| format!("could not write {}: {error}", key_file.display()))?;
+    Ok((files, key))
+}
+
+/// Listens on the socket in `dir`, whose lock is `locked`.
+#[cfg(unix)]
+fn listen_on_socket(
+    dir: &Path,
+    locked: std::fs::File,
+    server: Server<AppGateway>,
+) -> Result<Listening, String> {
     let socket = dir.join(endpoint::SOCKET);
     match std::fs::symlink_metadata(&socket) {
         // With the lock held, a socket there was left by a Tabletist that
@@ -123,17 +173,7 @@ pub(super) fn listen(dir: &Path, server: Server<AppGateway>) -> Result<Listening
         Ok(_) => return Err(format!("{} is in the way", socket.display())),
         Err(_) => {}
     }
-    // The key before the socket: a relay that connects the moment it can
-    // reads the key of this start, not one an ended start left.
-    let key = endpoint::new_key();
-    let key_file = dir.join(endpoint::KEY);
-    let mut files = Files {
-        socket: None,
-        key: (key_file.clone(), key.clone()),
-        _locked: locked,
-    };
-    crate::util::write_atomic(&key_file, key.as_bytes())
-        .map_err(|error| format!("could not write {}: {error}", key_file.display()))?;
+    let (mut files, key) = new_key(dir, locked)?;
     let listener = UnixListener::bind(&socket)
         .map_err(|error| format!("could not listen at {}: {error}", socket.display()))?;
     let made = std::fs::symlink_metadata(&socket)
@@ -150,9 +190,99 @@ pub(super) fn listen(dir: &Path, server: Server<AppGateway>) -> Result<Listening
     })
 }
 
+/// Listens on a pipe of a new name, which it writes in `dir`, whose lock is
+/// `locked`. The first instance is this server's own, or it does not
+/// listen; no other computer's relay is taken.
+#[cfg(windows)]
+fn listen_on_pipe(
+    dir: &Path,
+    locked: std::fs::File,
+    server: Server<AppGateway>,
+) -> Result<Listening, String> {
+    // The name an ended start left goes before the new key is written: a
+    // relay that read it would find, at best, a pipe another account made
+    // of that name since.
+    let pipe_file = dir.join(endpoint::PIPE);
+    match std::fs::remove_file(&pipe_file) {
+        Err(error) if error.kind() != std::io::ErrorKind::NotFound => {
+            return Err(format!(
+                "could not remove an old {}: {error}",
+                pipe_file.display()
+            ));
+        }
+        _ => {}
+    }
+    let (mut files, key) = new_key(dir, locked)?;
+    let name = endpoint::new_pipe();
+    let first = ServerOptions::new()
+        .first_pipe_instance(true)
+        .reject_remote_clients(true)
+        .create(&name)
+        .map_err(|error| format!("could not listen at {name}: {error}"))?;
+    // Named once it is there: a relay that reads the name finds the pipe.
+    crate::util::write_atomic(&pipe_file, name.as_bytes())
+        .map_err(|error| format!("could not write {}: {error}", pipe_file.display()))?;
+    files.pipe = Some((pipe_file, name.clone()));
+    let stop = CancellationToken::new();
+    tokio::spawn(accept(
+        Pipe {
+            waiting: first,
+            name,
+        },
+        Arc::new(key),
+        server,
+        stop.clone(),
+    ));
+    Ok(Listening {
+        stop,
+        _files: files,
+    })
+}
+
+/// Where relays connect: a socket's listener, or a pipe's instance that
+/// waits for one.
+trait Endpoint: Send + 'static {
+    type Stream: AsyncRead + AsyncWrite + Unpin + Send + 'static;
+    /// The next relay that connects.
+    async fn next(&mut self) -> std::io::Result<Self::Stream>;
+}
+
+#[cfg(unix)]
+impl Endpoint for UnixListener {
+    type Stream = tokio::net::UnixStream;
+
+    async fn next(&mut self) -> std::io::Result<Self::Stream> {
+        self.accept().await.map(|(stream, _)| stream)
+    }
+}
+
+/// The pipe: the instance a relay connects to next, and its name, which
+/// the next instance takes.
+#[cfg(windows)]
+struct Pipe {
+    waiting: NamedPipeServer,
+    name: String,
+}
+
+#[cfg(windows)]
+impl Endpoint for Pipe {
+    type Stream = NamedPipeServer;
+
+    /// The instance a relay connected to; another waits in its place. One
+    /// that could not be connected is put aside for another.
+    async fn next(&mut self) -> std::io::Result<Self::Stream> {
+        let connected = self.waiting.connect().await;
+        let next = ServerOptions::new()
+            .reject_remote_clients(true)
+            .create(&self.name)?;
+        let taken = std::mem::replace(&mut self.waiting, next);
+        connected.map(|()| taken)
+    }
+}
+
 /// Takes each relay that connects, until `stop`.
-async fn accept(
-    listener: UnixListener,
+async fn accept<E: Endpoint>(
+    mut endpoint: E,
     key: Arc<String>,
     server: Server<AppGateway>,
     stop: CancellationToken,
@@ -172,8 +302,8 @@ async fn accept(
         let stream = tokio::select! {
             biased;
             () = stop.cancelled() => return,
-            accepted = listener.accept() => match accepted {
-                Ok((stream, _)) => stream,
+            accepted = endpoint.next() => match accepted {
+                Ok(stream) => stream,
                 Err(error) => {
                     log::warn!("the agent server could not take a connection: {error}");
                     // Out of descriptors, most likely: not again at once.
@@ -213,29 +343,31 @@ impl Drop for Held {
 /// server's own stop, which lets its calls end and go into the log. Nothing
 /// the relay sent goes into the app's log: any program of the user's can
 /// send it.
-async fn greet(
-    mut stream: UnixStream,
+async fn greet<S: AsyncRead + AsyncWrite + Unpin + Send + 'static>(
+    mut stream: S,
     key: &str,
     server: &Server<AppGateway>,
     open: &Arc<AtomicUsize>,
     greeting: OwnedSemaphorePermit,
     stop: &CancellationToken,
 ) {
-    let refuse = async |stream: &mut UnixStream, said: &str| {
+    let refuse = async |stream: &mut S, said: &str| {
         let line = endpoint::line_of(&Answer::Refused(said.to_owned()));
         let _ = stream.write_all(&line).await;
         let _ = stream.shutdown().await;
     };
-    let read = tokio::select! {
-        biased;
-        () = stop.cancelled() => None,
-        read = tokio::time::timeout(
-            endpoint::LINE_TIME,
-            endpoint::read_line(&mut stream, endpoint::MAX_LINE),
-        ) => Some(read),
+    // The relay's line, within its time and until a stop.
+    let line = async |stream: &mut S| {
+        tokio::select! {
+            biased;
+            () = stop.cancelled() => None,
+            read = tokio::time::timeout(
+                endpoint::LINE_TIME,
+                endpoint::read_line(stream, endpoint::MAX_LINE),
+            ) => Some(read),
+        }
     };
-    drop(greeting);
-    let Some(read) = read else {
+    let Some(read) = line(&mut stream).await else {
         return refuse(&mut stream, endpoint::NOT_RUNNING).await;
     };
     let hello = match read {
@@ -248,14 +380,34 @@ async fn greet(
         log::warn!("a connection to the agent server said nothing it could read");
         return refuse(&mut stream, "Tabletist could not read what the relay sent.").await;
     };
-    if !endpoint::same_key(&hello.key, key) {
-        log::warn!("a relay sent a key that is not the agent server's");
+    // The app's proof first: a relay tells nothing to what cannot give it.
+    let challenge = endpoint::new_nonce();
+    let prove = Answer::Prove {
+        proof: endpoint::prove(key, Side::App, &hello.challenge),
+        challenge: challenge.clone(),
+    };
+    if stream.write_all(&endpoint::line_of(&prove)).await.is_err() {
+        return;
+    }
+    let Some(read) = line(&mut stream).await else {
+        return refuse(&mut stream, endpoint::NOT_RUNNING).await;
+    };
+    drop(greeting);
+    let reply = match read {
+        Ok(Ok(Some(line))) => serde_json::from_slice::<Reply>(&line).ok(),
+        _ => None,
+    };
+    let reply = reply.filter(|reply| {
+        endpoint::same_key(&reply.proof, &endpoint::prove(key, Side::Relay, &challenge))
+    });
+    let Some(reply) = reply else {
+        log::warn!("a relay did not prove it holds the agent server's key");
         return refuse(
             &mut stream,
             "The relay's key is not the one Tabletist made: start the agent's MCP server again.",
         )
         .await;
-    }
+    };
     // Counted once it has the key: who has none takes no agent's place.
     let held = Held::take(open);
     if !held.1 {
@@ -274,7 +426,7 @@ async fn greet(
     }
     let origin = Origin {
         transport: Transport::Stdio,
-        project: hello.cwd.as_deref().and_then(endpoint::project),
+        project: reply.cwd.as_deref().and_then(endpoint::project),
     };
     // Boxed: the session's future is deep, too deep for the compiler to
     // lay out inside this one.
diff --git a/src/endpoint.rs b/src/endpoint.rs
index cbd8e36..253d61d 100644
--- a/src/endpoint.rs
+++ b/src/endpoint.rs
@@ -2,10 +2,16 @@
 //! private to the user (`AppDirs::agents_dir`), and a key beside it that
 //! the relay sends first.
 //!
-//! The relay sends one line, a [`Hello`], and the app answers one line, an
-//! [`Answer`]. After that the bytes are the agent's and the server's, and
-//! the relay understands none of them: the protocol lives in the app, so an
-//! old relay keeps working with a newer one.
+//! The relay sends one line, a [`Hello`] with a challenge. The app answers
+//! with its proof that it holds the key, and a challenge of its own
+//! ([`Answer::Prove`]); the relay checks it before it says anything else,
+//! and sends its own proof ([`Reply`]); the app answers [`Answer::Ok`].
+//! Neither sends the key, so what does not hold it learns nothing: not the
+//! key from a relay that reached something else at the endpoint, which on
+//! Windows another account can make once the app's pipe is gone, nor an
+//! agent's requests. After that the bytes are the agent's and the
+//! server's, and the relay understands none of them: the protocol lives in
+//! the app.
 
 use std::time::Duration;
 
@@ -20,6 +26,11 @@ pub const AVAILABLE: bool = cfg!(unix);
 /// the relay says its line.
 pub const NOT_RUNNING: &str = "Tabletist is not running, or agents are off in it. Open Tabletist, turn agents on, then reconnect.";
 
+/// What the agent is told when what answered at the endpoint could not
+/// prove it holds the key: not this Tabletist, or a key file that changed
+/// since. It was told nothing.
+pub const NOT_PROVEN: &str = "What answered at Tabletist's endpoint did not prove it holds Tabletist's key, and was told nothing. Open Tabletist, then start the agent's MCP server again.";
+
 /// Why a server does not start where another Tabletist serves agents
 /// already: the person is told it after the app's own words.
 pub const ANOTHER_SERVES: &str = "another Tabletist is serving agents already";
@@ -36,6 +47,15 @@ pub const LOCK: &str = "mcp.lock";
 /// The key's name in the directory: new each time the server starts.
 pub const KEY: &str = "mcp.key";
 
+/// On Windows, the name of the file in the directory that holds the pipe's
+/// name, which is new each time the server starts: no other account can
+/// know it before the server makes the pipe, and so none can make one of
+/// that name first and be sent the key.
+pub const PIPE: &str = "mcp.pipe";
+
+/// How every pipe's name starts. The rest is [`new_pipe`]'s hex.
+pub const PIPE_PREFIX: &str = r"\\.\pipe\tabletist-mcp-";
+
 /// The longest path of a socket macOS takes (its `sun_path` holds 104
 /// bytes, the last of them the end of the string).
 pub const MAX_SOCKET_PATH: usize = 103;
@@ -50,19 +70,20 @@ pub const LINE_TIME: Duration = Duration::from_secs(5);
 /// What the relay says first.
 #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
 pub struct Hello {
-    /// What the app wrote to [`KEY`] when its server started.
-    pub key: String,
     /// The relay's version, for the app's log.
     pub version: String,
-    /// The directory the agent started the relay in: its last part names
-    /// the session's project for the person.
-    pub cwd: Option<String>,
+    /// What the app proves it holds the key with ([`new_nonce`]).
+    pub challenge: String,
 }
 
 /// What the app answers.
 #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
 #[serde(rename_all = "snake_case")]
 pub enum Answer {
+    /// The app holds the key: `proof` is [`prove`]'s for the relay's
+    /// challenge, from the app's [`Side`]. The relay proves it holds the
+    /// key too, for `challenge`.
+    Prove { proof: String, challenge: String },
     /// The session begins with the next byte.
     Ok,
     /// It does not, and why, for the person: the relay passes it on to the
@@ -70,6 +91,48 @@ pub enum Answer {
     Refused(String),
 }
 
+/// The relay's proof that it holds the key, for the app's challenge, and
+/// what it tells only an app that has proved it holds the key too.
+#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
+pub struct Reply {
+    pub proof: String,
+    /// The directory the agent started the relay in: its last part names
+    /// the session's project for the person.
+    pub cwd: Option<String>,
+}
+
+/// Who proves: each side's proof is its own, so neither can hand back the
+/// other's.
+#[derive(Debug, Clone, Copy, PartialEq, Eq)]
+pub enum Side {
+    App,
+    Relay,
+}
+
+/// A new challenge: 122 random bits, as hex.
+pub fn new_nonce() -> String {
+    uuid::Uuid::new_v4().simple().to_string()
+}
+
+/// The proof that `side` holds `key`, for `challenge`: an HMAC-SHA-256 of
+/// the side's name and the challenge under the key, as hex.
+pub fn prove(key: &str, side: Side, challenge: &str) -> String {
+    use hmac::{KeyInit, Mac};
+    let mut mac = hmac::Hmac::<sha2::Sha256>::new_from_slice(key.as_bytes())
+        .expect("HMAC takes a key of any length");
+    let name: &[u8] = match side {
+        Side::App => b"tabletist app\n",
+        Side::Relay => b"tabletist relay\n",
+    };
+    mac.update(name);
+    mac.update(challenge.as_bytes());
+    mac.finalize()
+        .into_bytes()
+        .iter()
+        .map(|byte| format!("{byte:02x}"))
+        .collect()
+}
+
 /// A key: 244 random bits, as two version 4 UUIDs' worth of hex.
 pub fn new_key() -> String {
     format!(
@@ -79,6 +142,18 @@ pub fn new_key() -> String {
     )
 }
 
+/// A new name for the server's pipe.
+pub fn new_pipe() -> String {
+    format!("{PIPE_PREFIX}{}", uuid::Uuid::new_v4().simple())
+}
+
+/// Whether `name`, as read from [`PIPE`], is a name [`new_pipe`] makes: a
+/// pipe on this computer, never a path to another one's.
+pub fn is_pipe(name: &str) -> bool {
+    name.strip_prefix(PIPE_PREFIX)
+        .is_some_and(|rest| rest.len() == 32 && rest.bytes().all(|b| b.is_ascii_hexdigit()))
+}
+
 /// Whether `given` is `key`, in a time that does not tell how much of it
 /// was right.
 pub fn same_key(given: &str, key: &str) -> bool {
diff --git a/src/relay.rs b/src/relay.rs
index e69fe54..ac3492c 100644
--- a/src/relay.rs
+++ b/src/relay.rs
@@ -11,10 +11,10 @@ use std::path::{Path, PathBuf};
 use std::process::ExitCode;
 use std::time::Duration;
 
-use crate::endpoint::{self, NOT_RUNNING};
+use crate::endpoint::{self, NOT_RUNNING, Side};
 
 /// How long the relay tries the endpoint: the app may be starting.
-#[cfg(unix)]
+#[cfg(any(unix, windows))]
 const REACH_TIME: Duration = Duration::from_secs(2);
 
 /// How long it waits for the agent's first request, to answer why it
@@ -54,14 +54,44 @@ async fn relay(dir: &Path) -> ExitCode {
     }
 }
 
-/// The app's socket, once it has taken the key. Why not, for the agent.
+/// The app's socket.
 #[cfg(unix)]
-async fn reach(dir: &Path) -> Result<tokio::net::UnixStream, String> {
+type Stream = tokio::net::UnixStream;
+
+/// The app's pipe.
+#[cfg(windows)]
+type Stream = tokio::net::windows::named_pipe::NamedPipeClient;
+
+/// Connects to the app's socket in `dir`.
+#[cfg(unix)]
+async fn connect(dir: &Path) -> std::io::Result<Stream> {
+    tokio::net::UnixStream::connect(dir.join(endpoint::SOCKET)).await
+}
+
+/// Connects to the app's pipe, by the name its server wrote in `dir`. A
+/// name that is not one the app makes is not tried: the key would go to
+/// whatever it names.
+#[cfg(windows)]
+async fn connect(dir: &Path) -> std::io::Result<Stream> {
+    let name = std::fs::read_to_string(dir.join(endpoint::PIPE))?;
+    let name = name.trim();
+    if !endpoint::is_pipe(name) {
+        return Err(std::io::Error::new(
+            std::io::ErrorKind::InvalidData,
+            "not a pipe of Tabletist's",
+        ));
+    }
+    // Busy while the server makes its next instance: tried again.
+    tokio::net::windows::named_pipe::ClientOptions::new().open(name)
+}
+
+/// The app's endpoint, once it has taken the key. Why not, for the agent.
+#[cfg(any(unix, windows))]
+async fn reach(dir: &Path) -> Result<Stream, String> {
     use tokio::io::AsyncWriteExt;
-    let socket = dir.join(endpoint::SOCKET);
     let deadline = tokio::time::Instant::now() + REACH_TIME;
     let mut stream = loop {
-        match tokio::net::UnixStream::connect(&socket).await {
+        match connect(dir).await {
             Ok(stream) => break stream,
             Err(_) if tokio::time::Instant::now() < deadline => {
                 tokio::time::sleep(Duration::from_millis(100)).await;
@@ -69,52 +99,85 @@ async fn reach(dir: &Path) -> Result<tokio::net::UnixStream, String> {
             Err(_) => return Err(NOT_RUNNING.into()),
         }
     };
-    let key = std::fs::read_to_string(dir.join(endpoint::KEY)).map_err(|_| NOT_RUNNING)?;
+    let challenge = endpoint::new_nonce();
     let hello = endpoint::Hello {
-        key: key.trim().to_owned(),
         version: env!("CARGO_PKG_VERSION").to_owned(),
-        cwd: std::env::current_dir()
-            .ok()
-            .map(|cwd| cwd.to_string_lossy().into_owned()),
+        challenge: challenge.clone(),
     };
     stream
         .write_all(&endpoint::line_of(&hello))
         .await
         .map_err(|_| NOT_RUNNING)?;
-    let answer = tokio::time::timeout(
-        endpoint::LINE_TIME,
-        endpoint::read_line(&mut stream, endpoint::MAX_LINE),
-    )
-    .await;
-    let answer = match answer {
-        Ok(Ok(Some(line))) => serde_json::from_slice::<endpoint::Answer>(&line).ok(),
-        _ => None,
+    let (proof, theirs) = match answer(&mut stream).await {
+        Some(endpoint::Answer::Prove { proof, challenge }) => (proof, challenge),
+        Some(endpoint::Answer::Refused(said)) => return Err(said),
+        Some(endpoint::Answer::Ok) | None => return Err(NO_ANSWER.into()),
     };
-    match answer {
+    // Read now, the key of the start that answers. What cannot prove it
+    // holds it is told nothing: not the relay's proof, nor the agent's
+    // first request.
+    let key = std::fs::read_to_string(dir.join(endpoint::KEY)).map_err(|_| NOT_RUNNING)?;
+    let key = key.trim();
+    if !endpoint::same_key(&proof, &endpoint::prove(key, Side::App, &challenge)) {
+        return Err(endpoint::NOT_PROVEN.into());
+    }
+    let reply = endpoint::Reply {
+        proof: endpoint::prove(key, Side::Relay, &theirs),
+        cwd: std::env::current_dir()
+            .ok()
+            .map(|cwd| cwd.to_string_lossy().into_owned()),
+    };
+    stream
+        .write_all(&endpoint::line_of(&reply))
+        .await
+        .map_err(|_| NOT_RUNNING)?;
+    match answer(&mut stream).await {
         Some(endpoint::Answer::Ok) => Ok(stream),
         Some(endpoint::Answer::Refused(said)) => Err(said),
-        None => Err("Tabletist did not answer the relay.".into()),
+        Some(endpoint::Answer::Prove { .. }) | None => Err(NO_ANSWER.into()),
     }
 }
 
-/// Nothing to reach here yet (`endpoint::AVAILABLE`).
-#[cfg(not(unix))]
+/// What the relay says when the app says nothing it can read.
+#[cfg(any(unix, windows))]
+const NO_ANSWER: &str = "Tabletist did not answer the relay.";
+
+/// The app's next line, within its time.
+#[cfg(any(unix, windows))]
+async fn answer(stream: &mut Stream) -> Option<endpoint::Answer> {
+    let read = tokio::time::timeout(
+        endpoint::LINE_TIME,
+        endpoint::read_line(stream, endpoint::MAX_LINE),
+    )
+    .await;
+    match read {
+        Ok(Ok(Some(line))) => serde_json::from_slice(&line).ok(),
+        _ => None,
+    }
+}
+
+/// Nothing to reach here (`endpoint::AVAILABLE`).
+#[cfg(not(any(unix, windows)))]
 async fn reach(_dir: &Path) -> Result<std::convert::Infallible, String> {
-    Err("Agents cannot reach Tabletist on this system yet.".into())
+    Err("Agents cannot reach Tabletist on this system.".into())
 }
 
 /// Copies the agent's bytes to the app and the app's to the agent, until
-/// the app ends the session. The agent's end only ends what it sends: the
-/// app still answers what it was asked.
-#[cfg(unix)]
-async fn pump(stream: tokio::net::UnixStream) -> ExitCode {
+/// the app ends the session. On a socket the agent's end only ends what
+/// it sends: the app still answers what it was asked. A pipe cannot be
+/// closed one way: there the agent's end ends the session, which a client
+/// does once it has its answers.
+#[cfg(any(unix, windows))]
+async fn pump(stream: Stream) -> ExitCode {
     use tokio::io::AsyncWriteExt;
-    let (mut from_app, mut to_app) = stream.into_split();
+    let (mut from_app, mut to_app) = tokio::io::split(stream);
     let up = async {
         let mut stdin = tokio::io::stdin();
         let _ = tokio::io::copy(&mut stdin, &mut to_app).await;
         let _ = to_app.shutdown().await;
-        std::future::pending::<()>().await;
+        if cfg!(unix) {
+            std::future::pending::<()>().await;
+        }
     };
     let down = async {
         let mut stdout = tokio::io::stdout();
@@ -124,7 +187,7 @@ async fn pump(stream: tokio::net::UnixStream) -> ExitCode {
     };
     let done = tokio::select! {
         done = down => done,
-        () = up => unreachable!(),
+        () = up => true,
     };
     match done {
         true => ExitCode::SUCCESS,
@@ -132,7 +195,7 @@ async fn pump(stream: tokio::net::UnixStream) -> ExitCode {
     }
 }
 
-#[cfg(not(unix))]
+#[cfg(not(any(unix, windows)))]
 async fn pump(stream: std::convert::Infallible) -> ExitCode {
     match stream {}
 }
````

- [ ] **Step 4: Run the tests**

Run: the same, then `cargo test --locked --test cli`

Expected: PASS. On Linux, the Windows tests are not built; CI's Windows job runs them (below).

```text
test result: ok. 18 passed; 0 failed; 0 ignored; 0 measured; 2519 filtered out
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

- [ ] **Step 5: Commit**

```bash
cargo fmt --all
cargo clippy --locked --workspace --all-targets -- -D warnings
git add Cargo.toml Cargo.lock src tests .github/workflows/ci.yml
git commit -m "Reach the app on Windows by a named pipe, each side proving it holds the key"
```

---

### Task 2: The command each agent is told, on Windows

- `shell_word` quotes as the system's shell reads it back: `posix_word` as before, and on Windows `windows_word`, which leaves a path of letters, digits and `\/._-:` as it is and puts any other in double quotes. The installed program is `%LOCALAPPDATA%\Programs\Tabletist\tabletist.exe` (`packaging/windows/tabletist.iss`), not on the `PATH`, so the command names it by its whole path. Cursor's entry has it as JSON writes it.

**Files:**
- Modify: `src/endpoint.rs`

- [ ] **Step 1: Write the failing test**

````diff
diff --git a/src/endpoint.rs b/src/endpoint.rs
index 253d61d..12121c3 100644
--- a/src/endpoint.rs
+++ b/src/endpoint.rs
@@ -478,7 +478,7 @@ mod tests {
         let spaced = Path::new("/Users/ada/My Apps/Tabletist's/tabletist");
         assert_eq!(
             relay_for(Some(spaced), None),
-            "'/Users/ada/My Apps/Tabletist'\\''s/tabletist' mcp"
+            format!("{} mcp", shell_word(&spaced.to_string_lossy()))
         );
         // Not known at all: the name.
         assert_eq!(relay_for(None, None), "tabletist mcp");
@@ -488,6 +488,31 @@ mod tests {
         );
     }
 
+    #[test]
+    fn a_path_is_quoted_as_each_kind_of_shell_reads_it() {
+        assert_eq!(posix_word("/usr/bin/tabletist"), "/usr/bin/tabletist");
+        assert_eq!(
+            posix_word("/Users/ada/My Apps/Tabletist's/tabletist"),
+            "'/Users/ada/My Apps/Tabletist'\\''s/tabletist'"
+        );
+        // A backslash is a POSIX shell's own.
+        assert_eq!(posix_word(r"a\b"), r"'a\b'");
+        assert_eq!(
+            windows_word(r"C:\Tools\tabletist.exe"),
+            r"C:\Tools\tabletist.exe"
+        );
+        assert_eq!(
+            windows_word(r"C:\Users\Ada Lovelace\AppData\Local\Programs\Tabletist\tabletist.exe"),
+            r#""C:\Users\Ada Lovelace\AppData\Local\Programs\Tabletist\tabletist.exe""#
+        );
+        // A comma, which PowerShell reads as a list, and brackets.
+        assert_eq!(windows_word(r"C:\a,b\t.exe"), r#""C:\a,b\t.exe""#);
+        assert_eq!(
+            windows_word(r"C:\Program Files (x86)\t.exe"),
+            r#""C:\Program Files (x86)\t.exe""#
+        );
+    }
+
     #[test]
     fn a_pipes_name_is_new_each_time_and_one_read_back_is_checked() {
         let name = new_pipe();
````

- [ ] **Step 2: Run it to see that it does not build**

Run: `cargo test --locked --lib endpoint::tests::a_path_is_quoted`

Expected: it does not build:

```text
error[E0425]: cannot find function `posix_word` in this scope
error[E0425]: cannot find function `windows_word` in this scope
error: could not compile `tabletist` (lib test) due to 7 previous errors
```

- [ ] **Step 3: The quoting**

````diff
diff --git a/src/endpoint.rs b/src/endpoint.rs
index 12121c3..ac66d46 100644
--- a/src/endpoint.rs
+++ b/src/endpoint.rs
@@ -337,19 +337,48 @@ fn relay_for(exe: Option<&std::path::Path>, on_path: Option<&std::path::Path>) -
     relay_of(&program_for(exe, on_path))
 }
 
-/// `word` as a POSIX shell reads it back: as it is when nothing in it is
-/// special, else in single quotes.
+/// `word` as this system's shell reads it back: a POSIX shell's, or on
+/// Windows the Command Prompt's and PowerShell's.
 fn shell_word(word: &str) -> String {
-    let plain = !word.is_empty()
+    if cfg!(windows) {
+        windows_word(word)
+    } else {
+        posix_word(word)
+    }
+}
+
+/// Whether `word` is not empty and is made of letters, digits and
+/// `others` alone.
+fn plain_word(word: &str, others: &str) -> bool {
+    !word.is_empty()
         && word
             .chars()
-            .all(|c| c.is_ascii_alphanumeric() || "/._-+:@%,=".contains(c));
-    match plain {
+            .all(|c| c.is_ascii_alphanumeric() || others.contains(c))
+}
+
+/// `word` as a POSIX shell reads it back: as it is when nothing in it is
+/// special, else in single quotes.
+fn posix_word(word: &str) -> String {
+    match plain_word(word, "/._-+:@%,=") {
         true => word.to_owned(),
         false => format!("'{}'", word.replace('\'', "'\\''")),
     }
 }
 
+/// `word` as the Command Prompt and PowerShell read it back, as an
+/// argument: as it is when it is made of what neither reads as anything
+/// else, else in double quotes, which keep a space, a comma, a bracket and
+/// an apostrophe as they are. A path on Windows holds no double quote. In
+/// double quotes PowerShell still expands `$` and a backtick, and the
+/// Command Prompt `%NAME%`: a path that holds them is not quoted against
+/// that, and an installed Tabletist's holds none.
+fn windows_word(word: &str) -> String {
+    match plain_word(word, "\\/._-:") {
+        true => word.to_owned(),
+        false => format!("\"{word}\""),
+    }
+}
+
 /// `value` as one line of JSON, with its end.
 pub fn line_of<T: Serialize>(value: &T) -> Vec<u8> {
     let mut line = serde_json::to_vec(value).unwrap_or_default();
````

- [ ] **Step 4: Run the tests**

Run: `cargo test --locked --lib endpoint::`

Expected: PASS.

```text
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 2528 filtered out
```

- [ ] **Step 5: Commit**

```bash
cargo fmt --all
cargo clippy --locked --workspace --all-targets -- -D warnings
git add src/endpoint.rs
git commit -m "Quote the program's path in the command as Windows' shells read it"
```

---

### Task 3: Agents on Windows

- `endpoint::AVAILABLE` is `cfg!(any(unix, windows))`: the sidebar's entry, the tab, the keys, Settings › Agents, the Connections column and the editor's section show on Windows, and the server starts there.
- A new agent flashes the taskbar button (`ViewportCommand::RequestUserAttention`), as it bounces the Dock on macOS.
- The tests of Agents that were `cfg(unix)` because nothing of them showed on Windows are `cfg(any(unix, windows))`; the one that held that nothing shows is `cfg(not(any(unix, windows)))`.
- Settings › Agents says passwords stay in Credential Manager.

This task turns on what Tasks 1 and 2 and the runs before built and test. On Linux its tests pass before it and after it; on Windows it is CI's Windows job that runs them, which ran them on the draft.

**Files:**
- Modify: `src/endpoint.rs`, `src/app.rs`, `src/settings.rs`, `src/ui/agents.rs`, `src/ui/agents_tests.rs`, `src/ui/mod.rs`, `src/ui/settings/agents.rs`

- [ ] **Step 1: Turn it on**

````diff
diff --git a/src/app.rs b/src/app.rs
index 330b84f..3e1dcb4 100644
--- a/src/app.rs
+++ b/src/app.rs
@@ -120,7 +120,7 @@ pub struct App {
     /// The window title last sent, so it is sent only when it changes.
     window_title: String,
     /// What the next frame asks of the window: to come forward, or the
-    /// Dock's bounce.
+    /// Dock's bounce or the taskbar's flash.
     pub window_asks: Vec<egui::ViewportCommand>,
     /// SQL editors closed since the last frame: what egui keeps for each
     /// (its undo history holds copies of the script) is dropped once a
@@ -4008,8 +4008,8 @@ impl App {
     }
 
     /// Tells the person of each session that opened with its agent named,
-    /// once: a notification on Linux, the Dock's bounce on macOS, while
-    /// `[mcp] notify` is on.
+    /// once: a notification on Linux, the Dock's bounce on macOS, the
+    /// taskbar button's flash on Windows, while `[mcp] notify` is on.
     fn tell_of_sessions(&mut self, open: &[tabletist_mcp::SessionInfo]) {
         self.agents
             .told
@@ -4023,7 +4023,7 @@ impl App {
         if new.is_empty() || !self.settings.mcp_notify {
             return;
         }
-        if cfg!(target_os = "macos") {
+        if cfg!(any(target_os = "macos", windows)) {
             self.window_asks
                 .push(egui::ViewportCommand::RequestUserAttention(
                     egui::UserAttentionType::Informational,
@@ -6115,7 +6115,7 @@ mod tests {
 
     /// Quick open has no Agents tab to offer where agents cannot reach the
     /// app.
-    #[cfg(not(unix))]
+    #[cfg(not(any(unix, windows)))]
     #[test]
     fn quick_open_offers_no_agents_where_they_cannot_reach_the_app() {
         let (app, _dir) = app();
@@ -6180,7 +6180,7 @@ mod tests {
 
     /// What the files say beside an app that serves agents: the server on,
     /// and a connection agents may read, which nobody in the app turned on.
-    #[cfg(unix)]
+    #[cfg(any(unix, windows))]
     fn changed_outside(app: &mut App) -> crate::connections::ConnectionId {
         let mut saved = crate::testing::fixture_connection();
         saved.name = "Shop".into();
@@ -6192,7 +6192,7 @@ mod tests {
         id
     }
 
-    #[cfg(unix)]
+    #[cfg(any(unix, windows))]
     fn agents_shown_saved(app: &App) -> Option<crate::agents_shown::Shown> {
         app.backend
             .sent
@@ -6207,7 +6207,7 @@ mod tests {
             })
     }
 
-    #[cfg(unix)]
+    #[cfg(any(unix, windows))]
     #[test]
     fn a_change_made_outside_the_app_is_held_until_the_person_keeps_it() {
         use crate::agents_shown::Raised;
@@ -6264,7 +6264,7 @@ mod tests {
         assert!(app.agents_held.is_empty());
     }
 
-    #[cfg(unix)]
+    #[cfg(any(unix, windows))]
     #[test]
     fn the_bar_names_every_change_in_one_sentence() {
         let (mut app, _dir) = app();
@@ -6285,7 +6285,7 @@ mod tests {
         );
     }
 
-    #[cfg(unix)]
+    #[cfg(any(unix, windows))]
     #[test]
     fn put_back_writes_what_the_person_was_shown_into_the_files() {
         let (mut app, dir) = app();
@@ -6319,7 +6319,7 @@ mod tests {
         );
     }
 
-    #[cfg(unix)]
+    #[cfg(any(unix, windows))]
     #[test]
     fn what_the_app_itself_turns_on_is_never_held() {
         let (mut app, _dir) = app();
@@ -6335,7 +6335,7 @@ mod tests {
         assert_eq!(app.agents_held.len(), 1, "the connection is still held");
     }
 
-    #[cfg(unix)]
+    #[cfg(any(unix, windows))]
     #[test]
     fn keep_and_put_back_answer_only_what_the_bar_said() {
         use crate::agents_shown::Raised;
@@ -6390,7 +6390,7 @@ mod tests {
         assert_eq!(app.agents_held, [Raised::Server]);
     }
 
-    #[cfg(unix)]
+    #[cfg(any(unix, windows))]
     #[test]
     fn an_edit_of_something_else_keeps_nothing_that_is_held() {
         use crate::connections::AgentAccess;
@@ -6519,7 +6519,7 @@ mod tests {
         assert!(app.agents_held.is_empty(), "a lowering is not held");
     }
 
-    #[cfg(unix)]
+    #[cfg(any(unix, windows))]
     #[test]
     fn show_brings_the_window_of_the_connection_an_agent_works_on_forward() {
         let (mut app, _dir) = app();
@@ -6600,7 +6600,7 @@ mod tests {
             .unwrap()
     }
 
-    #[cfg(unix)]
+    #[cfg(any(unix, windows))]
     #[test]
     fn the_log_is_read_a_page_at_a_time_for_this_connection_or_for_all() {
         use crate::model::AgentsView;
@@ -6654,7 +6654,7 @@ mod tests {
         assert!(log.all);
     }
 
-    #[cfg(unix)]
+    #[cfg(any(unix, windows))]
     #[test]
     fn a_call_on_the_connection_reads_the_log_again() {
         let (mut app, _dir) = app();
@@ -6695,7 +6695,7 @@ mod tests {
         assert_ne!(log_asked(&app).unwrap().2, asked);
     }
 
-    #[cfg(unix)]
+    #[cfg(any(unix, windows))]
     #[test]
     fn an_agent_that_connects_is_told_of_once_and_review_shows_the_tab() {
         let (mut app, _dir) = app();
@@ -6727,7 +6727,7 @@ mod tests {
                 .contains(&egui::ViewportCommand::RequestUserAttention(
                     egui::UserAttentionType::Informational
                 )),
-            cfg!(target_os = "macos")
+            cfg!(any(target_os = "macos", windows))
         );
         // Off in the settings: nobody is told.
         app.settings.mcp_notify = false;
@@ -6750,7 +6750,7 @@ mod tests {
         assert!(app.workspace(app.active_tab_id()).is_none());
     }
 
-    #[cfg(unix)]
+    #[cfg(any(unix, windows))]
     #[test]
     fn the_lists_access_is_the_persons_own_raised_or_lowered() {
         use crate::connections::AgentAccess;
@@ -6812,7 +6812,7 @@ mod tests {
         ));
     }
 
-    #[cfg(unix)]
+    #[cfg(any(unix, windows))]
     #[test]
     fn the_editors_access_is_the_persons_own_and_allow_agents_chooses_read() {
         use crate::connections::AgentAccess;
diff --git a/src/endpoint.rs b/src/endpoint.rs
index ac66d46..40826ee 100644
--- a/src/endpoint.rs
+++ b/src/endpoint.rs
@@ -18,9 +18,9 @@ use std::time::Duration;
 use serde::{Deserialize, Serialize};
 use tokio::io::{AsyncRead, AsyncReadExt};
 
-/// Whether agents can reach the app on this system. Not on Windows yet: its
-/// named pipe and its relay are a run of their own.
-pub const AVAILABLE: bool = cfg!(unix);
+/// Whether agents can reach the app on this system: through a socket on
+/// Linux and macOS, a named pipe on Windows.
+pub const AVAILABLE: bool = cfg!(any(unix, windows));
 
 /// What the agent is told when the app is not there to reach, or stops as
 /// the relay says its line.
diff --git a/src/settings.rs b/src/settings.rs
index 13385ff..3ad616a 100644
--- a/src/settings.rs
+++ b/src/settings.rs
@@ -64,9 +64,9 @@ pub struct Settings {
     pub mcp_enabled: bool,
     /// Days the log of agents' calls keeps a call.
     pub mcp_log_days: u32,
-    /// Whether an agent that connects is told of: a notification, or on
-    /// macOS the Dock's bounce. The Agents entry says who is here either
-    /// way.
+    /// Whether an agent that connects is told of: a notification, on
+    /// macOS the Dock's bounce, on Windows the taskbar button's flash. The
+    /// Agents entry says who is here either way.
     pub mcp_notify: bool,
 }
 
diff --git a/src/ui/agents.rs b/src/ui/agents.rs
index 72c82e0..dc209da 100644
--- a/src/ui/agents.rs
+++ b/src/ui/agents.rs
@@ -1516,7 +1516,7 @@ fn ended(
 mod tests {
     /// Where agents cannot reach the app, nothing of them shows: no entry,
     /// and no tab.
-    #[cfg(not(unix))]
+    #[cfg(not(any(unix, windows)))]
     #[test]
     fn nothing_of_agents_shows_where_they_cannot_reach_the_app() {
         let mut harness = crate::testing::Harness::new();
diff --git a/src/ui/agents_tests.rs b/src/ui/agents_tests.rs
index b8dbfd3..763a200 100644
--- a/src/ui/agents_tests.rs
+++ b/src/ui/agents_tests.rs
@@ -400,7 +400,7 @@ fn more_sessions_than_the_tab_has_room_for_scroll_to_the_last() {
 
 /// What the files say, changed outside the app: the server on, and agents
 /// may read `tab`'s connection.
-#[cfg(unix)]
+#[cfg(any(unix, windows))]
 fn changed_outside(harness: &mut Harness, tab: ConnTabId) {
     let conn = harness.app.workspace(tab).unwrap().conn_id.clone();
     let mut saved = harness.app.connections.get(&conn).unwrap().clone();
@@ -411,7 +411,7 @@ fn changed_outside(harness: &mut Harness, tab: ConnTabId) {
     harness.settle();
 }
 
-#[cfg(unix)]
+#[cfg(any(unix, windows))]
 #[test]
 fn the_held_bar_says_what_changed_and_its_buttons_answer_it() {
     for look in looks() {
@@ -450,7 +450,7 @@ fn the_held_bar_says_what_changed_and_its_buttons_answer_it() {
     }
 }
 
-#[cfg(unix)]
+#[cfg(any(unix, windows))]
 #[test]
 fn while_a_change_is_held_the_tab_says_what_is_in_force() {
     use crate::agents_shown::Raised;
@@ -986,7 +986,7 @@ fn a_call_selected_with_j_is_kept_in_sight() {
 }
 
 /// The Connections list with one saved connection, Shop · dev, selected.
-#[cfg(unix)]
+#[cfg(any(unix, windows))]
 fn on_the_list(look: Look) -> (Harness, crate::connections::ConnectionId) {
     let mut harness = Harness::new();
     harness.set_look(look);
@@ -1003,12 +1003,12 @@ fn on_the_list(look: Look) -> (Harness, crate::connections::ConnectionId) {
     (harness, conn)
 }
 
-#[cfg(unix)]
+#[cfg(any(unix, windows))]
 fn access_of(harness: &Harness, conn: &crate::connections::ConnectionId) -> AgentAccess {
     harness.app.connections.get(conn).unwrap().agents.access
 }
 
-#[cfg(unix)]
+#[cfg(any(unix, windows))]
 #[test]
 fn the_list_says_what_agents_may_do_and_its_menu_changes_it() {
     let (mut harness, conn) = on_the_list(Look::macos());
@@ -1030,7 +1030,7 @@ fn the_list_says_what_agents_may_do_and_its_menu_changes_it() {
     assert!(harness.has("Access through Tabletist (MCP)"));
 }
 
-#[cfg(unix)]
+#[cfg(any(unix, windows))]
 #[test]
 fn the_lists_menu_shows_the_agents_tab() {
     let (mut harness, conn) = on_the_list(Look::macos());
@@ -1046,7 +1046,7 @@ fn the_lists_menu_shows_the_agents_tab() {
 
 /// Types `text` over what the editor's field labelled `label` holds: the
 /// field is found by the label that names it, as a screen reader finds it.
-#[cfg(unix)]
+#[cfg(any(unix, windows))]
 fn type_over(harness: &mut Harness, label: &str, text: &str) {
     use egui::accesskit::Role;
     let tree = harness.settle();
@@ -1072,7 +1072,7 @@ fn type_over(harness: &mut Harness, label: &str, text: &str) {
     harness.settle();
 }
 
-#[cfg(unix)]
+#[cfg(any(unix, windows))]
 #[test]
 fn omarchys_box_sets_what_agents_may_do_from_the_list() {
     let (mut harness, conn) = on_the_list(Look::omarchy());
@@ -1112,7 +1112,7 @@ fn omarchys_box_sets_what_agents_may_do_from_the_list() {
     assert!(workspace.agents_in_front());
 }
 
-#[cfg(unix)]
+#[cfg(any(unix, windows))]
 #[test]
 fn the_editors_agents_part_sets_access_rows_and_timeout() {
     for look in looks() {
@@ -1147,7 +1147,7 @@ fn the_editors_agents_part_sets_access_rows_and_timeout() {
     }
 }
 
-#[cfg(unix)]
+#[cfg(any(unix, windows))]
 #[test]
 fn allow_agents_opens_the_editor_with_read_chosen() {
     for look in looks() {
@@ -1183,7 +1183,7 @@ fn allow_agents_opens_the_editor_with_read_chosen() {
     }
 }
 
-#[cfg(unix)]
+#[cfg(any(unix, windows))]
 #[test]
 fn the_editor_opens_at_its_agents_part() {
     for look in looks() {
@@ -1236,7 +1236,7 @@ fn the_editor_opens_at_its_agents_part() {
     }
 }
 
-#[cfg(unix)]
+#[cfg(any(unix, windows))]
 #[test]
 fn the_limits_are_in_sight_when_the_editor_opens_at_them() {
     for (look, size) in [(Look::macos(), 600.0), (Look::omarchy(), 700.0)] {
@@ -1268,7 +1268,7 @@ fn the_limits_are_in_sight_when_the_editor_opens_at_them() {
     }
 }
 
-#[cfg(unix)]
+#[cfg(any(unix, windows))]
 #[test]
 fn an_enter_held_or_shifted_in_the_box_does_nothing_behind_it() {
     use egui::{Key, Modifiers};
@@ -1298,7 +1298,7 @@ fn an_enter_held_or_shifted_in_the_box_does_nothing_behind_it() {
     );
 }
 
-#[cfg(unix)]
+#[cfg(any(unix, windows))]
 #[test]
 fn no_allow_agents_is_offered_for_a_connection_saved_no_more() {
     let (mut harness, tab) = connected(Look::omarchy());
@@ -1318,7 +1318,7 @@ fn no_allow_agents_is_offered_for_a_connection_saved_no_more() {
     assert!(!offered(&harness));
 }
 
-#[cfg(unix)]
+#[cfg(any(unix, windows))]
 #[test]
 fn the_agents_column_and_hint_step_back_where_the_window_has_no_room() {
     // macOS at 1024: Last used stays clear of the row's buttons.
diff --git a/src/ui/mod.rs b/src/ui/mod.rs
index b60867f..dcfddd5 100644
--- a/src/ui/mod.rs
+++ b/src/ui/mod.rs
@@ -7,11 +7,11 @@ pub mod agents;
 // Where agents can reach the app: elsewhere nothing of them shows, which
 // the last test of `agents` holds.
 #[cfg(test)]
-#[cfg(unix)]
+#[cfg(any(unix, windows))]
 mod agents_tests;
 // Settings › Agents, which is listed where agents can reach the app.
 #[cfg(test)]
-#[cfg(unix)]
+#[cfg(any(unix, windows))]
 mod agent_settings_tests;
 pub mod cell_editor;
 #[cfg(test)]
diff --git a/src/ui/settings/agents.rs b/src/ui/settings/agents.rs
index 9d2bbed..2b10aad 100644
--- a/src/ui/settings/agents.rs
+++ b/src/ui/settings/agents.rs
@@ -92,6 +92,8 @@ fn title(ui: &mut Ui, on: bool, skin: &Skin, actions: &mut Vec<Action>) {
     .layout(ui.ctx());
     let said = if cfg!(target_os = "macos") {
         "Let coding agents query your databases through Tabletist (MCP). They use your saved connections; passwords stay in the Keychain."
+    } else if cfg!(windows) {
+        "Let coding agents query your databases through Tabletist (MCP). They use your saved connections; passwords stay in Credential Manager."
     } else {
         "Let coding agents query your databases through Tabletist (MCP). They use your saved connections; passwords stay in the keyring."
     };
````

- [ ] **Step 2: Run the tests**

Run: `cargo test --locked --lib ui::agents_tests`, `cargo test --locked --lib ui::agent_settings_tests` and `cargo test --locked --lib app::`

Expected: PASS, as before this task on Linux.

```text
test result: ok. 34 passed; 0 failed; 0 ignored; 0 measured; 2504 filtered out
test result: ok. 14 passed; 0 failed; 0 ignored; 0 measured; 2524 filtered out
test result: ok. 406 passed; 0 failed; 0 ignored; 0 measured; 2132 filtered out
```

- [ ] **Step 3: Commit**

```bash
cargo fmt --all
cargo clippy --locked --workspace --all-targets -- -D warnings
git add src
git commit -m "Show agents on Windows, and flash the taskbar for a new one"
```

---

### Task 4: The website

- `docs/_guide/agents.md`: Windows among the systems, the installed program's path in the command, Ctrl+Alt+A, the taskbar's flash.
- `docs/_reference/settings-and-files.md`: the Agents pane on every system, `mcp.pipe`, the key each side proves it holds, the notification on Windows.
- `docs/_reference/connections.md`, `keyboard-shortcuts.md`: no "(Linux and macOS)".

**Files:**
- Modify: `docs/_guide/agents.md`, `docs/_reference/connections.md`, `docs/_reference/keyboard-shortcuts.md`, `docs/_reference/settings-and-files.md`

- [ ] **Step 1: The pages**

````diff
diff --git a/docs/_guide/agents.md b/docs/_guide/agents.md
index 522eacc..ca33e5d 100644
--- a/docs/_guide/agents.md
+++ b/docs/_guide/agents.md
@@ -10,7 +10,7 @@ Protocol (MCP), and Tabletist runs the query on your saved connection,
 with that connection's own login and its limits. Passwords stay in the
 system keyring and never reach the agent.
 
-Agents work on Linux and macOS. On Windows nothing of them shows yet.
+Agents work on Linux, macOS and Windows.
 
 ## What an agent can do
 
@@ -74,8 +74,10 @@ agent, then **Copy** what it takes:
 
 Where `tabletist` on your `PATH` is not the app you run, the command names
 the app by its whole path instead. On macOS that is the program inside the
-app: `/Applications/Tabletist.app/Contents/MacOS/tabletist mcp`. Copy gives
-the one your Tabletist needs.
+app: `/Applications/Tabletist.app/Contents/MacOS/tabletist mcp`. On
+Windows it is where the installer put it, in quotes where the path has a
+space: `"C:\Users\Ada Lovelace\AppData\Local\Programs\Tabletist\tabletist.exe" mcp`.
+Copy gives the one your Tabletist needs.
 
 In the Omarchy look, `y` on the Agents pane copies Claude Code's command.
 
@@ -98,10 +100,10 @@ section.
 
 Each connection window has an Agents tab for its connection. It opens
 from the **Agents** entry at the bottom of the sidebar, from Cmd+Option+A
-on macOS or `ga` and `:agents` in the Omarchy look, from quick open on
-macOS, from Connections, and from **Show Agents tab** in **Settings ›
-Agents**. Nothing
-an agent does opens it for you.
+on macOS, Ctrl+Alt+A on Windows, or `ga` and `:agents` in the Omarchy
+look, from quick open on macOS and Windows, from Connections, and from
+**Show Agents tab** in **Settings › Agents**. Nothing an agent does opens
+it for you.
 
 The tab shows:
 
@@ -120,7 +122,8 @@ long it took. Rows themselves are never logged. **Settings › Agents ›
 Keep the activity log** says for how many days a call is kept.
 
 When an agent connects, Tabletist tells you: a notification on Linux,
-whose **Review** shows the Agents tab, and the Dock icon bouncing on macOS.
+whose **Review** shows the Agents tab, the Dock icon bouncing on macOS,
+and the taskbar button flashing on Windows.
 **Notify when an agent connects** turns it off.
 
 ## Changes made outside Tabletist
diff --git a/docs/_reference/connections.md b/docs/_reference/connections.md
index 2807b82..baadbdd 100644
--- a/docs/_reference/connections.md
+++ b/docs/_reference/connections.md
@@ -34,7 +34,7 @@ already open, and Shift+Enter opens it again.
 | **SSL mode**, **CA certificate** | See [TLS](#tls) |
 | **Connect through SSH tunnel** | See [SSH tunnels](#ssh-tunnels) |
 | **Open read-only** | See [Open read-only](#open-read-only) |
-| **Agents** | See [Agents](#agents) (Linux and macOS) |
+| **Agents** | See [Agents](#agents) |
 
 **Test** checks the connection without saving it.
 
diff --git a/docs/_reference/keyboard-shortcuts.md b/docs/_reference/keyboard-shortcuts.md
index 018394c..420bff7 100644
--- a/docs/_reference/keyboard-shortcuts.md
+++ b/docs/_reference/keyboard-shortcuts.md
@@ -50,7 +50,7 @@ In the connection dialog:
 | --- | --- |
 | Mod+P | Quick open. In it, Up and Down move, Enter opens and Esc closes |
 | Mod+B | Show or hide the sidebar |
-| Mod+Alt+A | Show the Agents tab (Linux and macOS) |
+| Mod+Alt+A | Show the Agents tab |
 | Mod+W | Close the tab |
 | Mod+Shift+[ and ] | Previous and next tab |
 | F6, Shift+F6 | Next and previous part of the window |
diff --git a/docs/_reference/settings-and-files.md b/docs/_reference/settings-and-files.md
index 010642b..17f9516 100644
--- a/docs/_reference/settings-and-files.md
+++ b/docs/_reference/settings-and-files.md
@@ -17,7 +17,7 @@ Cmd/Ctrl+, opens Settings. Changes apply right away.
 | **Numbers** | `1240.50` or `1,240.50`. Grouping is for display only: a copy gives the raw value | `1240.50` |
 | **Value tags** | Colors for enum, CHECK and boolean columns | On |
 
-On Linux and macOS a second pane, **Agents**, sets what the `[mcp]` keys
+A second pane, **Agents**, sets what the `[mcp]` keys
 hold, and shows how to connect an agent:
 [Agents]({% link _guide/agents.md %}) is the guide.
 
@@ -69,9 +69,9 @@ notify = true
 | `editor.sql_limit` | 1 to 10000 rows for each statement | `1000` |
 | `editor.sql_timeout_secs` | Seconds. `0` waits forever | `30` |
 | `appearance.theme` | The name of a palette file in the `themes` folder | Not set |
-| `mcp.enabled` | `true` lets agents reach the connections you open to them, through `tabletist mcp`. Linux and macOS | `false` |
+| `mcp.enabled` | `true` lets agents reach the connections you open to them, through `tabletist mcp` | `false` |
 | `mcp.log_days` | 1 to 3650 days. How long the log of agents' calls keeps a call | `30` |
-| `mcp.notify` | `true` tells you when an agent connects: a notification on Linux, the Dock icon bouncing on macOS | `true` |
+| `mcp.notify` | `true` tells you when an agent connects: a notification on Linux, the Dock icon bouncing on macOS, the taskbar button flashing on Windows | `true` |
 
 `show_system_schemas` and `theme` are set only in the file. The SQL
 editor's **Limit** and **Timeout** menus set the two `editor` keys.
@@ -104,14 +104,17 @@ On Linux, `XDG_CONFIG_HOME` and `XDG_STATE_HOME` move the two folders.
 | `tabletist.log`, `panic.log` | State | Logs |
 | `agents.sqlite3` | State | Every call agents made: the tool, the statement and its values, and how it ended. Never a row |
 | `agents_shown.json` | State | What agents may do as Tabletist last showed it: whether the server is on, and each connection's access |
-| `mcp.sock`, `mcp.key`, `mcp.lock` | Runtime, else State | Where `tabletist mcp` reaches the app, the key it must send, and the lock the app holds while it listens. The first two are there while agents are on |
+| `mcp.sock`, `mcp.key`, `mcp.lock` | Runtime, else State | Where `tabletist mcp` reaches the app, the key each proves it holds before either says more, and the lock the app holds while it listens. The socket and the key are there while agents are on |
+| `mcp.pipe` | State | On Windows, in the socket's place: the name of the pipe `tabletist mcp` reaches the app by, new each time agents are turned on |
 
 The runtime folder is `/run/user/<your user id>/tabletist` on Linux, where
 the system makes `/run/user` for you, and `~/.local/state/tabletist` where
 it does not, whatever `XDG_STATE_HOME` says: an agent may start
 `tabletist mcp` without your variables. macOS has none, and uses the state
 folder, or `.tabletist` in your home folder when the state folder's path is
-too long for a socket.
+too long for a socket. Windows has none either, and uses the state folder:
+the app listens on a named pipe, whose name is new each time agents are
+turned on, and writes the name there for `tabletist mcp` to find.
 
 Passwords are kept in the system keyring, never in these files.
 [Read about passwords]({% link _reference/connections.md %}#passwords).
````

- [ ] **Step 2: Check them**

Run: `docs/bin/check`, and `grep -c "$(printf '\342\200\224')" docs/_guide/agents.md docs/_reference/connections.md docs/_reference/keyboard-shortcuts.md docs/_reference/settings-and-files.md` (an em dash, written in octal so that this plan holds none)

Expected: the check ends `HTML-Proofer finished successfully.`, and each page counts `0`.

- [ ] **Step 3: Commit**

```bash
git add docs/_guide docs/_reference
git commit -m "Say on the website that agents reach Tabletist on Windows"
```

---

### Task 5: The master plan says what was built

**Files:**
- Modify: `docs/superpowers/plans/2026-10-13-mcp-server.md`

- [ ] **Step 1: Correct it**

````diff
diff --git a/docs/superpowers/plans/2026-10-13-mcp-server.md b/docs/superpowers/plans/2026-10-13-mcp-server.md
index c458f33..1bf3b7a 100644
--- a/docs/superpowers/plans/2026-10-13-mcp-server.md
+++ b/docs/superpowers/plans/2026-10-13-mcp-server.md
@@ -332,6 +332,32 @@ sidebar entry, no pane in Settings, no command in the keymap or in quick
 open. The code is the same on all three and its tests run on all three, so
 the look does not fall behind: what waits is the way in.
 
+**As built on Windows** (`2026-10-15-mcp-server-1b-6-windows.md`). Agreed on 2026-10-15, where it went
+otherwise:
+
+- **The pipe's name is new each time the server starts**, a random one
+  under `\\.\pipe\tabletist-mcp-`, and the server writes it to
+  `mcp.pipe` beside `mcp.key` in the state directory, which is the
+  user's own. The fixed name per user that this section planned could be
+  made first by another account, and the relay would send it the key. The
+  relay takes only a name of that form from the file: never another
+  computer's pipe. The first instance is made with `first_pipe_instance`
+  and every one with `reject_remote_clients`; the key still guards the
+  pipe from the other accounts its default access list lets open it.
+- **A pipe cannot be closed one way.** On Windows the end of the agent's
+  stdin ends the relay and the session, where a socket lets the app still
+  answer what it was asked. A client closes stdin once it has its
+  answers.
+- **The relay from the GUI binary works**, and CI holds it: the Windows
+  job also runs the relay's tests on a release build, whose binary a test
+  reads to be sure it is a GUI program. No console target was needed.
+- The command for Claude Code names the installed program by its whole
+  path, in double quotes where it has a space
+  (`%LOCALAPPDATA%\Programs\Tabletist\tabletist.exe`), as the Command
+  Prompt and PowerShell read it. Cursor's entry has the path as JSON
+  writes it.
+- A new agent flashes the taskbar button, as the Dock bounces on macOS.
+
 ### A.3 HTTP
 
 `http://127.0.0.1:7433/mcp`, the Streamable HTTP transport.
@@ -2178,8 +2204,8 @@ is off until the person turns it on.
 | Part | Runs | State on 2026-10-13 |
 |---|---|---|
 | 1a. The enforcement core | 3 | Done: pull requests 122, 123 and 124 |
-| 1b. The server over stdio, with its interface | 9, below | Built on Linux and macOS (`2026-10-13-mcp-server-1b-1-server.md`, `2026-10-14-mcp-server-1b-2a-gateway.md`, `2026-10-14-mcp-server-1b-2b-stdio.md`, `2026-10-14-mcp-server-1b-3a-agents-tab.md`, `2026-10-14-mcp-server-1b-3b-changes-outside.md`, `2026-10-14-mcp-server-1b-4a-activity.md`, `2026-10-14-mcp-server-1b-4b-log.md`, `2026-10-15-mcp-server-1b-5a-access.md` and `2026-10-15-mcp-server-1b-5b-settings.md`). Windows' way in is next |
-| 1b on Windows: the way in | 1 | After 1b (question 14) |
+| 1b. The server over stdio, with its interface | 9, below | Built (`2026-10-13-mcp-server-1b-1-server.md`, `2026-10-14-mcp-server-1b-2a-gateway.md`, `2026-10-14-mcp-server-1b-2b-stdio.md`, `2026-10-14-mcp-server-1b-3a-agents-tab.md`, `2026-10-14-mcp-server-1b-3b-changes-outside.md`, `2026-10-14-mcp-server-1b-4a-activity.md`, `2026-10-14-mcp-server-1b-4b-log.md`, `2026-10-15-mcp-server-1b-5a-access.md` and `2026-10-15-mcp-server-1b-5b-settings.md`) |
+| 1b on Windows: the way in | 1 | Built (`2026-10-15-mcp-server-1b-6-windows.md`) |
 | 1c. Hidden tables | to be cut in its plan | Waits for questions 8 and 13 |
 | 1d. HTTP with tokens | 1 | After stdio (question 9) |
 
@@ -2287,6 +2313,8 @@ can see what agents do and stop them.
 pipe, the relay from the GUI binary or a console target if that fails
 (A.2), its test on the Windows CI runner, the taskbar's flash for a
 notification, and the function that says the server is available there.
+Built (`2026-10-15-mcp-server-1b-6-windows.md`): the relay from the GUI binary, tested on CI's release
+build; no console target.
 
 **1c. Hidden tables.**
 
````

- [ ] **Step 2: Check it**

Run: `grep -c "$(printf '\342\200\224')" docs/superpowers/plans/2026-10-13-mcp-server.md`

Expected: `0`.

- [ ] **Step 3: Commit**

```bash
git add docs/superpowers/plans/2026-10-13-mcp-server.md
git commit -m "Say in the MCP plan what its run for Windows built"
```

---

### After the last task

- [ ] **The whole of it, once more**, with both servers' variables set:

```bash
cargo fmt --all --check
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo test --locked --workspace --all-targets
RUSTDOCFLAGS='-D warnings' cargo doc --locked --workspace --no-deps
docs/bin/check
```

Expected: all clean, and every test passes. On 2026-10-15 on Linux, with both servers up: 23 test binaries, 3,487 passed, none failed, 22 ignored. The website's check ends `HTML-Proofer finished successfully.`

- [ ] **Open the pull request**, and read its Windows job: `test (windows-latest)` runs the relay's tests in its own step on the release build (`The relay from the GUI build (Windows)`), where `a_release_build_on_windows_is_a_gui_program` and the three `mcp::` tests say `ok`.

- [ ] **By hand, on Windows**: install the pull request's build, turn agents on in Settings › Agents, copy the command into a terminal where Claude Code is installed, and ask the agent about a connection set to Read. Then quit Tabletist and start the agent again: it says Tabletist is not running.

## Self-review

- **Against the master plan.** G's row for Windows: the named pipe (Task 1), the relay from the GUI binary and its test on the Windows CI runner (Task 1, the CI step and the PE test), the taskbar's flash (Task 3), and the function that says the server is available there (Task 3, `AVAILABLE`). A.2's Windows row: `first_pipe_instance` and `reject_remote_clients` are kept; the fixed name is not (above).
- **A review before the plan.** A reviewer read the draft, type-checked its Windows code in an extract, and found:
  - that another account could take a pipe name the app no longer held and be sent the key and an agent's requests: fixed, each side proves the key and neither sends it, with a test that puts such an impostor at the endpoint (a socket on Linux and macOS, a pipe of the left name on Windows) and fails when the relay does not check;
  - that a start wrote the new key while an ended start's pipe name was still there: fixed, the name goes first;
  - that the old pipe could still be opened right after a stop: it goes with the task that holds it, a moment later, and the Windows test waits for it (CI's first run caught it too);
  - that the quoting's comment claimed more than it did, and two lines of the website: fixed.
  It found sound: the async trait's futures are `Send` for both endpoints, mio turns a broken pipe into the end of the stream, a refusal still reaches the relay, `is_pipe` keeps out another computer's pipe, `SECURITY_IDENTIFICATION` keeps a server from acting as the relay's user, and the CI step's filter picks the four tests.
- **Left, and said.** Another account logged in at the same time can flood the pipe: it keeps agents out while it does and gets nothing. On Windows the end of stdin ends the session at once. Starting a GUI program that opens no window may show the busy cursor for a moment, each time an agent starts the relay.
- **Placeholders.** None: every step has its diff or its command and what it prints. Task 3's step runs on Linux what passes there before it and after it; its proof on Windows is CI's.
- **Names.** `Hello`, `Answer::Prove`, `Reply`, `Side`, `prove`, `new_nonce`, `NOT_PROVEN`, `PIPE`, `new_pipe`, `is_pipe`, `posix_word`, `windows_word`, `Endpoint`, `Pipe` are the same in every task that uses them.
