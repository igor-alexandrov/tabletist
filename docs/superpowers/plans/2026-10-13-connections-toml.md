# Saved connections in `connections.toml`. Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Saved connections are kept in `connections.toml`, a file a person can read and write, where they were kept in `connections.json`. What an older version saved is carried over at the first start, and nothing a user set is lost.

**Architecture:** A new module, `connections/file.rs`, is the file as text: one writer, which gives the same text for the same connections, and one strict reader. `SavedConnections::load` reads the TOML, else the older JSON, else nothing, and says what the app is to write. When each connection last connected moves to a file of its own in the state directory, so the file a person edits is written only when a connection changes.

**Tech Stack:** Rust 2024, the `toml` crate with its `serde` feature (no new crate), `serde`.

---

## Where this sits

The MCP server's master plan (`docs/superpowers/plans/2026-10-13-mcp-server.md`) asked, as its open question 5, whether an agent's settings for a connection go into `connections.json` or into a new `connections.toml`, which is what the design's boards draw. The answer, given on 2026-10-13: TOML, and as a run of its own before Phase 1b, so that the settings land in the file once.

This run moves the file and adds nothing to it. An agent's settings are Phase 1b's.

## Before you start

- Work on a branch of `main`. One commit for each task, in the order below. Every commit passes `cargo fmt --all --check`, `cargo clippy --locked --workspace --all-targets -- -D warnings` and `cargo test --locked --workspace --all-targets`.
- On Igor's machine `cargo` is `~/.cargo/bin/cargo` (the mise shim fails), and a build in a worktree needs `CARGO_TARGET_DIR` set to that worktree's `target/`.
- Read `AGENTS.md`. Three of its rules shape this code: a `match` on `Driver` names every variant, only `src/env.rs` matches on an `Environment`, and there are no em dashes in comments or messages.
- The diffs below apply with `git apply`. They are fenced with four backticks, because one of them holds a page of the website with fences of three. Run `cargo fmt --all` before each commit: the code here is already formatted, so it should change nothing.
- `Cargo.lock` does not change. It already lists everything `toml`'s `serde` feature needs: `cargo check --locked` passes with the feature named in `Cargo.toml` and the lock file as it is.
- Task 3 builds the website: `docs/bin/check` needs Ruby and the site's gems (`bundle install` in `docs/`).

## What was run before this plan was written

Each task's state was built, passed `cargo fmt --check` and clippy with `-D warnings` for the whole workspace, and its tests failed and then passed as each step says. The last state also passed `cargo doc` with warnings denied, the workspace's tests, and `docs/bin/check`. On Linux only: the CI matrix runs the same tests on macOS and Windows.

Not done: the app was not started by hand on a real profile. The start-up path is tested through `App::new` with a recording backend, as the move of the settings to TOML was.

## The file

```toml
# Tabletist's saved connections. No password is kept here: those are in the
# system's keyring.
#
# Edit it while Tabletist is closed. A connection without an `id` line gets
# one at the next start. Tabletist writes the whole file again when a
# connection changes, and keeps no comment but these.
version = 3

[[connections]]
id          = "6a0c1f0e-5a52-4b7b-9c56-3f0c8f2f7a11"
name        = "Bookshop production"
environment = "production"
read_only   = true
password    = "keyring"
ssh_secret  = "ask"
driver      = "postgres"
host        = "db.example.com"
port        = 5432
user        = "app"
database    = "bookshop_production"
tls         = "verify-full"
ca_file     = "/etc/ssl/bookshop-ca.pem"

[connections.ssh]
host = "bastion.example.com"
port = 2222
user = "deploy"
auth = "key-file"
key  = "/home/ada/.ssh/id_ed25519"

[[connections]]
id          = "0b7d6c1e-9f0a-4c1d-8a43-1d2b7c5e9f02"
name        = "Bookshop on disk"
environment = "local"
driver      = "sqlite"
sqlite_path = "/home/ada/bookshop.db"
```

## What this run settles

| Question | Decision | Why |
|---|---|---|
| How a connection is found | By its `id`, in a `[[connections]]` table of its own | The boards draw `[connections."Bookshop · dev".agents]`, keyed by name. Two connections may have one name: they form a group in the picker (`src/ui/picker.rs`). The keyring's entries and the label choices are kept by id already |
| The shape of an entry | The spec's keys (`driver`, `host`, …) beside the connection's own (`name`, `environment`, …), and the tunnel as `[connections.ssh]` | The JSON nested a `spec` object, because it was a derived type written out. A person writing a connection does not think of a spec. Phase 1b's `[connections.agents]` sits beside `[connections.ssh]` |
| Who writes the file | The app, by hand, in one form: the same text for the same connections, a table's keys aligned | As `settings.toml` is written (`Settings::to_toml`). The `toml` crate's writer would be a new crate (`toml_writer`), which `Cargo.toml` already says it avoids |
| Who reads it | `toml::from_str` into types of the file's own (`File`, `Entry`, `Tunnel`), with `deny_unknown_fields` | Not into `ConnectSpec`'s derived form, which stays what it is for the older JSON. The file's schema is in one place, and a key the app does not know is an error |
| A key the app does not know | An error, with its line. Nothing of the file is used | In a file a person edits, `read_onyl = true` must not pass for a connection that may write. A reader that skips what it does not know cannot tell a typo from a newer version's key |
| A file that cannot be read | Kept beside itself, whole, as `connections.toml.bad`. The start has no connections and says why, with the line | What `util::keep_aside` already does for a damaged file. Using the connections that could be read would lose the others at the next save |
| A key that is left out | Reads as its default: no environment, no password, the driver's own port, `prefer` for TLS | So a connection written by hand is five lines |
| What the writer leaves out | A key whose value is what a missing key reads as. Except a server's host, port, user, database and TLS, which are always written | A person looks for those. A SQLite connection is its path |
| An `id` that is left out | The app gives the connection one, and the file is written once so that it keeps it | Nobody should have to make up a UUID to add a connection by hand |
| Two connections with one id | An error | A block copied by hand has its original's id. The app cannot know which of the two the keyring's password belongs to |
| The file's version | 3. Versions 1 and 2 were the JSON | A newer file is refused like any the app cannot read |
| The older file | Read at a start that finds no `connections.toml`, never written, never removed | As `settings.json` was treated. It stays as the user's own copy of what was there before |
| When a connection last connected | `last_used.json`, in the state directory | The JSON kept it beside the connections, so every connect wrote the whole file (`App::mark_used`). It is of this computer, and no setting |
| A comment a person adds | Not kept. The header says so | Keeping one needs a parser that keeps the text (`toml_edit`, a new crate). `settings.toml` does not keep them either |
| An edit while the app runs | Overwritten at the next change of a connection. The header says to edit while Tabletist is closed | The file is not watched, as the JSON was not |
| `known_hosts.json`, `labels.json` | Stay JSON | Nobody writes them by hand |
| A path that is not UTF-8 | The save fails and says which path | TOML is text. The JSON could not hold one either |

### What changes for someone who downgrades

The reader is strict, so an older Tabletist than the one that wrote a file refuses a key it does not know, where the JSON was read past it. Once Phase 1b adds `[connections.agents]`, a Tabletist from before it will keep such a file aside as `connections.toml.bad` and start with no connections, until the newer one is back or the file is renamed. Nothing is lost, and no setting is silently dropped: that is the trade this run makes for catching a mistyped key.

A Tabletist from before this run does not know `connections.toml` at all. It reads the `connections.json` that was left in place, as it was on the day of the move.

### What this run leaves out

- **An agent's settings.** `[connections.agents]` is Phase 1b's.
- **Watching the file**, and showing it beside the connections in the Omarchy look as `settings.toml` is shown beside the options. Phase 1b's editor needs both, and its plan decides them.
- **Reading a file that is wrong in one place.** `settings.toml` is read past a bad line. A connection is not a setting with a default to fall back on.
- **A notice for a damaged `connections.json`.** It is kept aside without a word, as before. Only the new file's problem is said, since only that one is written by hand.

## File structure

| File | What changes |
|---|---|
| `src/connections/file.rs` | New. The file as text: `write`, `read`, and the file's own types |
| `src/connections.rs` | `SavedConnections::load` reads the TOML, else the JSON, and says what is to be written (`Loaded`). `save` writes TOML. The store loses its `version` |
| `src/paths.rs` | `connections_file` is the TOML. `legacy_connections_file`, `last_used_file` |
| `src/backend.rs` | `StateFile::LastUsed` |
| `src/app.rs` | The start says a problem, and has both files written when they came from the JSON. A connect saves the times and not the connections |
| `src/env.rs` | `Environment::key`: the word a file keeps it as |
| `src/util.rs`, `src/settings.rs` | `toml_string` moves from the settings to the helpers, for both writers |
| `Cargo.toml` | `toml` with `serde` |
| `docs/_reference/settings-and-files.md`, `docs/_guide/macos.md` | The file, its keys, and what happens to the old one |

---

### Task 1: Two helpers

What the file's writer needs from elsewhere. `quote`, which writes a TOML string for `settings.toml`, moves to `util` as `toml_string`. And an environment gets the word a file keeps it as: only `src/env.rs` may match on an `Environment` (`environments_are_matched_only_in_this_module`), so the writer asks it.

**Files:**
- Modify: `src/env.rs`, `src/util.rs`, `src/settings.rs`

- [ ] **Step 1: Write the failing test**

````diff
diff --git a/src/env.rs b/src/env.rs
index 1673880..f43fbed 100644
--- a/src/env.rs
+++ b/src/env.rs
@@ -405,6 +405,15 @@ fn omarchy(env: Environment, palette: &Palette) -> EnvColors {
 #[cfg(test)]
 mod tests {
     use super::*;
+
+    #[test]
+    fn an_environment_is_read_back_from_the_word_a_file_keeps_it_as() {
+        for environment in Environment::ALL {
+            let written = format!("\"{}\"", environment.key());
+            let read: Environment = serde_json::from_str(&written).unwrap();
+            assert_eq!(read, environment);
+        }
+    }
     use crate::theme::contrast;
 
     fn hex(value: u32) -> Color32 {
````

- [ ] **Step 2: Run it to see it fail**

Run: `cargo test --locked --lib env::tests::an_environment_is_read_back`

Expected: it does not compile.

```
error[E0599]: no method named `key` found for enum `env::Environment` in the current scope
```

- [ ] **Step 3: The word, and the helper where both writers reach it**

````diff
diff --git a/src/env.rs b/src/env.rs
index f43fbed..4956598 100644
--- a/src/env.rs
+++ b/src/env.rs
@@ -50,6 +50,18 @@ impl Environment {
         if local { Self::Local } else { Self::None }
     }
 
+    /// The word a file keeps the environment as, and is read back from:
+    /// its name in lower case.
+    pub fn key(self) -> &'static str {
+        match self {
+            Self::Local => "local",
+            Self::Dev => "dev",
+            Self::Staging => "staging",
+            Self::Production => "production",
+            Self::None => "none",
+        }
+    }
+
     /// Whether a connection in this environment is read-only unless its
     /// user says otherwise.
     pub fn read_only_by_default(self) -> bool {
diff --git a/src/settings.rs b/src/settings.rs
index 320772c..c6c4594 100644
--- a/src/settings.rs
+++ b/src/settings.rs
@@ -5,6 +5,7 @@ use std::path::Path;
 use serde::Deserialize;
 
 use crate::paths::AppDirs;
+use crate::util::toml_string;
 
 /// How much of a timestamp the grid shows.
 #[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize)]
@@ -186,27 +187,6 @@ const HEADER: &str = "# written by tabletist, safe to edit by hand\n";
 const MAX_LINES: usize = 1_000;
 const MAX_BYTES: usize = 64 * 1024;
 
-/// `text` as a TOML basic string.
-fn quote(text: &str) -> String {
-    let mut out = String::with_capacity(text.len() + 2);
-    out.push('"');
-    for character in text.chars() {
-        match character {
-            '"' => out.push_str("\\\""),
-            '\\' => out.push_str("\\\\"),
-            '\n' => out.push_str("\\n"),
-            '\r' => out.push_str("\\r"),
-            '\t' => out.push_str("\\t"),
-            control if control.is_control() => {
-                out.push_str(&format!("\\u{:04X}", u32::from(control)));
-            }
-            other => out.push(other),
-        }
-    }
-    out.push('"');
-    out
-}
-
 /// Where a start's settings came from.
 #[derive(Clone, Copy, Debug, PartialEq, Eq)]
 pub enum Source {
@@ -483,14 +463,14 @@ impl Settings {
     fn literal(&self, key: Key) -> Option<String> {
         Some(match key {
             Key::PageSize => self.page_size.to_string(),
-            Key::Timestamps => quote(self.timestamps.name()),
+            Key::Timestamps => toml_string(self.timestamps.name()),
             Key::GroupDigits => self.group_digits.to_string(),
             Key::ValueTags => self.value_tags.to_string(),
             Key::ShowSystemSchemas => self.show_system_schemas.to_string(),
             Key::SqlLimit => self.sql_limit.to_string(),
             // TOML has no null: no timeout is no seconds.
             Key::SqlTimeoutSecs => self.sql_timeout_secs.unwrap_or(0).to_string(),
-            Key::Theme => quote(self.custom_theme.as_deref()?),
+            Key::Theme => toml_string(self.custom_theme.as_deref()?),
         })
     }
 
diff --git a/src/util.rs b/src/util.rs
index 39d249c..c13b473 100644
--- a/src/util.rs
+++ b/src/util.rs
@@ -191,6 +191,28 @@ fn put_atomic(path: &Path, bytes: &[u8], replace: bool) -> std::io::Result<bool>
     Ok(true)
 }
 
+/// `text` as a TOML basic string: what the app writes into the files it
+/// writes as TOML itself.
+pub fn toml_string(text: &str) -> String {
+    let mut out = String::with_capacity(text.len() + 2);
+    out.push('"');
+    for character in text.chars() {
+        match character {
+            '"' => out.push_str("\\\""),
+            '\\' => out.push_str("\\\\"),
+            '\n' => out.push_str("\\n"),
+            '\r' => out.push_str("\\r"),
+            '\t' => out.push_str("\\t"),
+            control if control.is_control() => {
+                out.push_str(&format!("\\u{:04X}", u32::from(control)));
+            }
+            other => out.push(other),
+        }
+    }
+    out.push('"');
+    out
+}
+
 /// Saves `value` as pretty JSON, atomically.
 pub fn save_json<T: Serialize>(path: &Path, value: &T) -> std::io::Result<()> {
     let bytes = serde_json::to_vec_pretty(value).map_err(std::io::Error::other)?;
````

- [ ] **Step 4: Run the tests**

Run: `cargo test --locked --lib env::`, then `cargo test --locked --lib settings::`

Expected: `15 passed` for `env::` and `51 passed` for `settings::`, whose tests pin the text of `settings.toml` and pass unchanged: the helper moved and writes what it wrote.

- [ ] **Step 5: Commit**

```bash
cargo fmt --all
cargo clippy --locked --workspace --all-targets -- -D warnings
git add src/env.rs src/util.rs src/settings.rs
git commit -m "Give the writers of TOML one string helper, and an environment its word"
```

---

### Task 2: The file, and the store that keeps it

One task and one commit: the format has no caller until the store reads and writes it, and the store's new `load` changes what the app's start calls.

`connections/file.rs` is pure: text in, connections out, and back. `connections.rs` does the files: `load` takes the app's directories and gives a `Loaded`, which holds the store and what the caller is to do (`save`, `save_last_used`, `problem`). `App::new` puts the problem in the notice and sends the saves to the backend, as it sends the settings that came from `settings.json`.

The store's `version` field goes: nothing read it, and the writer knows the file's version. `StoredConnections`, the reader of the JSON, loses its own with it: every JSON file is written as TOML now, whichever of its two versions it was.

**Files:**
- Create: `src/connections/file.rs`
- Modify: `src/connections.rs`, `src/paths.rs`, `src/backend.rs`, `src/app.rs`, `Cargo.toml`
- Modify (a comment each): `crates/tabletist-db/Cargo.toml`, `crates/tabletist-db/src/spec.rs`

- [ ] **Step 1: Write the failing tests**

The store's tests are rewritten for the two files. In `app.rs`, the test of an older file saved by the backend becomes the test of the move, and two are new: a file that cannot be read is said at the start, and a connect writes the times and not the connections. The format's own tests come with its file in Step 3.

````diff
diff --git a/src/app.rs b/src/app.rs
index 1511459..c12fbc9 100644
--- a/src/app.rs
+++ b/src/app.rs
@@ -4402,7 +4402,7 @@ mod tests {
         use crate::labels::{LabelChoice, Place};
         let dir = tempfile::tempdir().unwrap();
         let dirs = AppDirs::at(dir.path());
-        let connections = dirs.connections_file();
+        let connections = dirs.legacy_connections_file();
         std::fs::create_dir_all(connections.parent().unwrap()).unwrap();
         let fixture = include_str!("../tests/fixtures/connections-v1.json");
         std::fs::write(&connections, fixture).unwrap();
@@ -4451,39 +4451,101 @@ mod tests {
         assert_eq!(again.labels, app.labels);
     }
 
+    /// How many times `app` had the backend save `path`.
+    fn saves_of(app: &App, path: &std::path::Path) -> usize {
+        app.backend
+            .sent
+            .iter()
+            .filter(|command| matches!(command, Command::Save { path: to, .. } if to == path))
+            .count()
+    }
+
     #[test]
-    fn an_older_connections_file_is_saved_upgraded_by_the_backend() {
+    fn connections_read_from_the_old_json_are_written_as_toml_once() {
+        let dir = tempfile::tempdir().unwrap();
+        let dirs = AppDirs::at(dir.path());
+        let (old, new, used) = (
+            dirs.legacy_connections_file(),
+            dirs.connections_file(),
+            dirs.last_used_file(),
+        );
+        std::fs::create_dir_all(old.parent().unwrap()).unwrap();
+        let fixture = include_str!("../tests/fixtures/connections-v1.json");
+        std::fs::write(&old, fixture).unwrap();
+        let start = || {
+            App::new(
+                AppDirs::at(dir.path()),
+                Settings::default().into(),
+                Backend::recording(),
+            )
+        };
+        let app = start();
+        assert_eq!(app.notice, None);
+        assert_eq!(app.connections.connections.len(), 14);
+        // Both files are the backend's to write: the connections, and
+        // when each last connected, which the old file kept with them.
+        assert_eq!((saves_of(&app, &new), saves_of(&app, &used)), (1, 1));
+        assert!(!new.exists(), "the UI thread writes nothing");
+        assert_eq!(saves_of(&app, &old), 0);
+        // Written, they are what the next start reads, and nothing is
+        // saved again. The old file is as it was.
+        app.connections.save(&new).unwrap();
+        std::fs::create_dir_all(used.parent().unwrap()).unwrap();
+        crate::util::save_json(&used, &app.connections.last_used).unwrap();
+        let again = start();
+        assert_eq!(again.connections, app.connections);
+        assert_eq!((saves_of(&again, &new), saves_of(&again, &used)), (0, 0));
+        assert_eq!(std::fs::read_to_string(&old).unwrap(), fixture);
+    }
+
+    #[test]
+    fn a_connections_file_that_cannot_be_read_is_said_at_the_start() {
         let dir = tempfile::tempdir().unwrap();
         let path = AppDirs::at(dir.path()).connections_file();
         std::fs::create_dir_all(path.parent().unwrap()).unwrap();
-        std::fs::write(&path, include_str!("../tests/fixtures/connections-v1.json")).unwrap();
+        std::fs::write(&path, "version = 3\n\n[[connections]]\nname = 1\n").unwrap();
         let app = App::new(
             AppDirs::at(dir.path()),
             Settings::default().into(),
             Backend::recording(),
         );
-        let saves = |app: &App| {
-            app.backend
-                .sent
-                .iter()
-                .filter(|command| matches!(command, Command::Save { path: to, .. } if *to == path))
-                .count()
-        };
-        assert_eq!(saves(&app), 1);
+        assert!(app.connections.connections.is_empty());
+        let notice = app.notice.clone().unwrap();
         assert!(
-            std::fs::read_to_string(&path)
-                .unwrap()
-                .contains("\"color\""),
-            "the UI thread writes nothing"
+            notice.starts_with("connections.toml could not be read (line 4: "),
+            "{notice}"
         );
-        // A current file is not saved again.
-        app.connections.save(&path).unwrap();
-        let app = App::new(
-            AppDirs::at(dir.path()),
-            Settings::default().into(),
-            Backend::recording(),
-        );
-        assert_eq!(saves(&app), 0);
+        // Kept aside, and nothing written in its place.
+        assert!(path.with_file_name("connections.toml.bad").exists());
+        assert_eq!(saves_of(&app, &path), 0);
+    }
+
+    /// A connect is written to the state, and leaves the file of
+    /// connections alone.
+    #[test]
+    fn a_connect_writes_when_it_was_and_not_the_connections() {
+        let (mut app, dir) = app();
+        let dirs = AppDirs::at(dir.path());
+        let (connections, used) = (dirs.connections_file(), dirs.last_used_file());
+        let (tab, session, request) = connect(&mut app);
+        let conn = app.workspace(tab).unwrap().conn_id.clone();
+        let before = saves_of(&app, &connections);
+        assert_eq!(saves_of(&app, &used), 0);
+        app.apply(Action::Backend(Event::Connected {
+            session,
+            request,
+            driver: Driver::Sqlite,
+            encrypted: false,
+            access: crate::testing::asked_access(&app),
+        }));
+        assert!(app.connections.last_used(&conn).is_some());
+        assert_eq!(saves_of(&app, &used), 1);
+        assert_eq!(saves_of(&app, &connections), before);
+        // A deleted connection's time goes with it, in both files.
+        app.apply(Action::DeleteConnection(conn.clone()));
+        assert_eq!(app.connections.last_used(&conn), None);
+        assert_eq!(saves_of(&app, &used), 2);
+        assert_eq!(saves_of(&app, &connections), before + 1);
     }
 
     #[test]
@@ -5945,8 +6007,7 @@ mod tests {
         app.apply(Action::DeleteConnection(conn.clone()));
         assert!(app.connections.get(&conn).is_none());
         assert_eq!(app.workspace(tab).unwrap().name, "Local");
-        let stored =
-            crate::connections::SavedConnections::load(&AppDirs::at(dir.path()).connections_file());
+        let stored = crate::connections::SavedConnections::load(&AppDirs::at(dir.path())).store;
         assert!(stored.get(&conn).is_none());
     }
 
diff --git a/src/backend.rs b/src/backend.rs
index 7280b2b..d58fda6 100644
--- a/src/backend.rs
+++ b/src/backend.rs
@@ -4383,7 +4383,8 @@ mod tests {
     #[test]
     fn a_burst_of_saves_leaves_the_newest_on_disk() {
         let dir = tempfile::tempdir().unwrap();
-        let path = dir.path().join("config").join("connections.json");
+        let dirs = crate::paths::AppDirs::at(dir.path());
+        let path = dirs.connections_file();
         let mut backend = Backend::start_with(Waker::default(), Keyring::memory());
         for n in 0..50 {
             backend.send(Command::Save {
@@ -4392,7 +4393,7 @@ mod tests {
             });
         }
         backend.flush(WAIT);
-        let stored = SavedConnections::load(&path);
+        let stored = SavedConnections::load(&dirs).store;
         assert_eq!(stored.connections[0].name, "49");
         let mut saved = 0;
         while let Some(Event::Saved { result, .. }) = backend.wait(Duration::from_millis(100)) {
diff --git a/src/connections.rs b/src/connections.rs
index 2b4b5a8..20def7b 100644
--- a/src/connections.rs
+++ b/src/connections.rs
@@ -422,8 +422,14 @@ mod tests {
         let stored: StoredConnections = serde_json::from_str(text).unwrap();
         let (store, _) = stored.upgrade();
         assert!(store.last_used.is_empty());
-        let saved = serde_json::to_string(&store).unwrap();
-        assert!(!saved.contains("last_used"), "{saved}");
+    }
+
+    /// The directories of a test, and the folder they are under.
+    fn dirs() -> (AppDirs, tempfile::TempDir) {
+        let dir = tempfile::tempdir().unwrap();
+        let dirs = AppDirs::at(dir.path());
+        dirs.ensure().unwrap();
+        (dirs, dir)
     }
 
     fn saved(name: &str, file: &str) -> SavedConnection {
@@ -440,29 +446,134 @@ mod tests {
 
     #[test]
     fn connections_round_trip_through_the_file() {
-        let dir = tempfile::tempdir().unwrap();
-        let path = dir.path().join("connections.json");
+        let (dirs, _dir) = dirs();
         let mut store = SavedConnections::default();
         store.upsert(SavedConnection {
             environment: Environment::Production,
             ..saved("Prod", "/prod.db")
         });
-        store.save(&path).unwrap();
-        let loaded = SavedConnections::load(&path);
-        assert_eq!(loaded.connections, store.connections);
-        let text = std::fs::read_to_string(&path).unwrap();
-        assert!(text.contains("\"environment\": \"production\""), "{text}");
-        assert!(text.contains("\"version\": 2"), "{text}");
-        assert!(!text.contains("color"), "{text}");
+        store.save(&dirs.connections_file()).unwrap();
+        let loaded = SavedConnections::load(&dirs);
+        assert_eq!(loaded.store, store);
+        // Read as it was written: nothing to write again, nothing wrong.
+        assert!(!loaded.save && !loaded.save_last_used);
+        assert_eq!(loaded.problem, None);
+        let text = std::fs::read_to_string(dirs.connections_file()).unwrap();
+        assert!(text.contains("environment = \"production\""), "{text}");
+        assert!(text.contains("version = 3"), "{text}");
+    }
+
+    /// When a connection last connected is the state's, and comes back
+    /// with the connections though it is no part of their file.
+    #[test]
+    fn the_times_of_last_use_are_kept_apart_from_the_connections() {
+        let (dirs, _dir) = dirs();
+        let mut store = SavedConnections::default();
+        let prod = saved("Prod", "/prod.db");
+        store.upsert(prod.clone());
+        store.mark_used(&prod.id, 1_790_683_200);
+        store.save(&dirs.connections_file()).unwrap();
+        let text = std::fs::read_to_string(dirs.connections_file()).unwrap();
+        assert!(!text.contains("last_used"), "{text}");
+        assert!(!text.contains("1790683200"), "{text}");
+        // Not written by a save of the connections.
+        assert!(!dirs.last_used_file().exists());
+        assert_eq!(
+            SavedConnections::load(&dirs).store.last_used(&prod.id),
+            None
+        );
+        crate::util::save_json(&dirs.last_used_file(), &store.last_used).unwrap();
+        let loaded = SavedConnections::load(&dirs);
+        assert_eq!(loaded.store.last_used(&prod.id), Some(1_790_683_200));
+        assert_eq!(loaded.store, store);
     }
 
     #[test]
-    fn a_damaged_store_is_kept_aside() {
-        let dir = tempfile::tempdir().unwrap();
-        let path = dir.path().join("connections.json");
+    fn a_file_that_cannot_be_read_is_kept_aside_whole_and_said() {
+        let (dirs, _dir) = dirs();
+        let path = dirs.connections_file();
+        let good = saved("Prod", "/prod.db");
+        let mut store = SavedConnections::default();
+        store.upsert(good);
+        store.save(&path).unwrap();
+        // One key that is no key: nothing of the file is used.
+        let text = std::fs::read_to_string(&path).unwrap();
+        let text = text.replace("driver ", "read_onyl = true\ndriver ");
+        std::fs::write(&path, &text).unwrap();
+        let loaded = SavedConnections::load(&dirs);
+        assert!(loaded.store.connections.is_empty());
+        assert!(!loaded.save && !loaded.save_last_used);
+        let problem = loaded.problem.unwrap();
+        assert!(
+            problem.starts_with(
+                "connections.toml could not be read (line 13: unknown field `read_onyl`"
+            ),
+            "{problem}"
+        );
+        assert!(
+            problem.ends_with(
+                "It was kept beside itself as connections.toml.bad, and no connection was loaded."
+            ),
+            "{problem}"
+        );
+        // The file is whole where it was kept, and gone from its place:
+        // the next save writes a new one and replaces nothing.
+        let aside = path.with_file_name("connections.toml.bad");
+        assert_eq!(std::fs::read_to_string(aside).unwrap(), text);
+        assert!(!path.exists());
+        // Bytes that are no text are kept aside the same way.
+        std::fs::write(&path, [0xC3, 0x28]).unwrap();
+        let loaded = SavedConnections::load(&dirs);
+        assert!(loaded.problem.is_some());
+        assert!(path.with_file_name("connections.toml.bad.1").exists());
+    }
+
+    #[test]
+    fn a_damaged_older_file_is_kept_aside() {
+        let (dirs, _dir) = dirs();
+        let path = dirs.legacy_connections_file();
         std::fs::write(&path, "[not json").unwrap();
-        assert!(SavedConnections::load(&path).connections.is_empty());
-        assert!(dir.path().join("connections.json.bad").exists());
+        let loaded = SavedConnections::load(&dirs);
+        assert!(loaded.store.connections.is_empty());
+        assert!(path.with_file_name("connections.json.bad").exists());
+    }
+
+    /// The new file wins: once it is there the old one is not read, even
+    /// with other connections in it.
+    #[test]
+    fn the_older_file_is_not_read_beside_the_new_one() {
+        let (dirs, _dir) = dirs();
+        let fixture = include_str!("../tests/fixtures/connections-v1.json");
+        std::fs::write(dirs.legacy_connections_file(), fixture).unwrap();
+        let mut store = SavedConnections::default();
+        store.upsert(saved("Only", "/only.db"));
+        store.save(&dirs.connections_file()).unwrap();
+        let loaded = SavedConnections::load(&dirs);
+        assert_eq!(loaded.store, store);
+        assert!(!loaded.save && !loaded.save_last_used);
+    }
+
+    /// A connection written into the file by hand needs no id: it gets
+    /// one, and the file is to be written so that it keeps it.
+    #[test]
+    fn a_connection_without_an_id_gets_one_that_is_saved() {
+        let (dirs, _dir) = dirs();
+        let text = "version = 3\n\n[[connections]]\nname = \"By hand\"\ndriver = \"sqlite\"\n\
+                    sqlite_path = \"/by-hand.db\"\n";
+        std::fs::write(dirs.connections_file(), text).unwrap();
+        let loaded = SavedConnections::load(&dirs);
+        assert!(loaded.save);
+        assert_eq!(loaded.problem, None);
+        let [connection] = loaded.store.connections.as_slice() else {
+            panic!("{:?}", loaded.store);
+        };
+        assert_eq!(connection.name, "By hand");
+        assert!(!connection.id.0.is_empty());
+        // Saved, it reads back with the same id and nothing left to write.
+        loaded.store.save(&dirs.connections_file()).unwrap();
+        let again = SavedConnections::load(&dirs);
+        assert_eq!(again.store, loaded.store);
+        assert!(!again.save);
     }
 
     #[test]
@@ -561,7 +672,6 @@ mod tests {
         ] {
             assert_eq!(environment(id), expected, "{id}");
         }
-        assert_eq!(store.version, VERSION);
         assert_eq!(
             store.last_used(&ConnectionId("red".into())),
             Some(1_790_683_200)
@@ -602,32 +712,38 @@ mod tests {
         );
     }
 
+    /// The first start of this version: what `connections.json` kept is
+    /// read, both files are to be written, and written they hold all of
+    /// it. The old file is left as it was.
     #[test]
-    fn an_upgraded_file_saved_again_has_nothing_left_to_work_out() {
-        let dir = tempfile::tempdir().unwrap();
-        let path = dir.path().join("connections.json");
-        std::fs::write(&path, include_str!("../tests/fixtures/connections-v1.json")).unwrap();
-        let (first, old) = SavedConnections::load_upgrading(&path);
-        assert!(old);
-        first.save(&path).unwrap();
-        let text = std::fs::read_to_string(&path).unwrap();
-        assert!(text.contains("\"version\": 2"), "{text}");
+    fn an_older_file_is_read_once_and_written_as_the_new_ones() {
+        let (dirs, _dir) = dirs();
+        let fixture = include_str!("../tests/fixtures/connections-v1.json");
+        std::fs::write(dirs.legacy_connections_file(), fixture).unwrap();
+        let first = SavedConnections::load(&dirs);
+        assert!(first.save && first.save_last_used);
+        assert_eq!(first.problem, None);
+        assert_eq!(first.store, self::fixture().0);
+        // What the app then has the backend write.
+        first.store.save(&dirs.connections_file()).unwrap();
+        crate::util::save_json(&dirs.last_used_file(), &first.store.last_used).unwrap();
+        let text = std::fs::read_to_string(dirs.connections_file()).unwrap();
+        assert!(text.contains("version = 3"), "{text}");
         assert!(!text.contains("color"), "{text}");
         assert!(!text.contains("\"test\""), "{text}");
-        let stored: StoredConnections = serde_json::from_str(&text).unwrap();
-        let (again, notes) = stored.upgrade();
-        assert!(notes.is_empty(), "{notes:#?}");
-        assert_eq!(again, first);
-        assert!(!SavedConnections::load_upgrading(&path).1);
+        let again = SavedConnections::load(&dirs);
+        assert_eq!(again.store, first.store);
+        assert!(!again.save && !again.save_last_used);
+        let kept = std::fs::read_to_string(dirs.legacy_connections_file()).unwrap();
+        assert_eq!(kept, fixture);
     }
 
     #[test]
-    fn a_missing_file_needs_no_upgrade() {
-        let dir = tempfile::tempdir().unwrap();
-        let path = dir.path().join("connections.json");
-        let (store, old) = SavedConnections::load_upgrading(&path);
-        assert!(store.connections.is_empty());
-        assert!(!old);
+    fn nothing_on_disk_is_no_connections_and_nothing_to_write() {
+        let (dirs, _dir) = dirs();
+        assert_eq!(SavedConnections::load(&dirs), Loaded::default());
+        assert!(!dirs.connections_file().exists());
+        assert!(!dirs.last_used_file().exists());
     }
 
     #[test]
diff --git a/src/paths.rs b/src/paths.rs
index 8c709bf..b3fdd91 100644
--- a/src/paths.rs
+++ b/src/paths.rs
@@ -113,8 +113,16 @@ mod tests {
         );
         assert_eq!(
             dirs.connections_file(),
+            PathBuf::from("/root/config/connections.toml")
+        );
+        assert_eq!(
+            dirs.legacy_connections_file(),
             PathBuf::from("/root/config/connections.json")
         );
+        assert_eq!(
+            dirs.last_used_file(),
+            PathBuf::from("/root/state/last_used.json")
+        );
         assert_eq!(
             dirs.known_hosts_file(),
             PathBuf::from("/root/config/known_hosts.json")
````

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test --locked --lib connections::`

Expected: it does not compile. The first of 47 errors:

```
error[E0433]: cannot find type `Loaded` in this scope
error[E0599]: no method named `legacy_connections_file` found for struct `paths::AppDirs` in the current scope
error[E0599]: no method named `last_used_file` found for struct `paths::AppDirs` in the current scope
error[E0609]: no field `store` on type `connections::SavedConnections`
```

- [ ] **Step 3: The format, the store, and the start**

````diff
diff --git a/Cargo.toml b/Cargo.toml
index df9cfa1..7dc1046 100644
--- a/Cargo.toml
+++ b/Cargo.toml
@@ -80,17 +80,19 @@ rfd = "0.17"
 uuid = { version = "1", features = ["v4"] }
 
 # One protected-store API for saved passwords; each platform's native
-# provider below. Passwords never go into connections.json.
+# provider below. Passwords never go into connections.toml.
 keyring-core = "1"
 # Settings and state files are written through a uniquely named temporary
 # file created exclusively (0600 on Unix) and renamed into place
 # (src/util.rs), and --demo runs in a private, self-removing directory.
 tempfile = "3"
 # Reads settings.toml: `DeTable::parse_recoverable` keeps what it could read
-# of a file with a bad line and says where each key and each error is. The
-# app writes the file itself (src/settings.rs), so only the parser is built:
-# the default `display` feature would bring in toml_writer.
-toml = { version = "1.1", default-features = false, features = ["parse"] }
+# of a file with a bad line and says where each key and each error is. And
+# reads connections.toml, strictly, into its own types (`serde`, which adds
+# no crate: serde is in the tree). The app writes both files itself
+# (src/settings.rs, src/connections/file.rs), so no writer is built: the
+# default `display` feature would bring in toml_writer.
+toml = { version = "1.1", default-features = false, features = ["parse", "serde"] }
 # Tells the backend when settings.toml changes on disk, so an edit made in
 # an editor reaches the running app (src/backend.rs). The version and the
 # features fastframe-theme already brings in for following the desktop's
diff --git a/crates/tabletist-db/Cargo.toml b/crates/tabletist-db/Cargo.toml
index a0cbf80..f83a4d9 100644
--- a/crates/tabletist-db/Cargo.toml
+++ b/crates/tabletist-db/Cargo.toml
@@ -12,7 +12,7 @@ workspace = true
 [dependencies]
 # Typed errors the app shows to the user as-is.
 thiserror = "2"
-# Connection specs are saved in connections.json.
+# Connection specs were saved in connections.json, which is still read.
 serde = { version = "1", features = ["derive"] }
 # Adapters are async; SQLite's blocking calls run on spawn_blocking.
 tokio = { version = "1", features = ["rt", "sync", "net", "io-util"] }
diff --git a/crates/tabletist-db/src/spec.rs b/crates/tabletist-db/src/spec.rs
index 18493c8..cb7b847 100644
--- a/crates/tabletist-db/src/spec.rs
+++ b/crates/tabletist-db/src/spec.rs
@@ -93,7 +93,9 @@ pub struct SshSpec {
     pub auth: SshAuth,
 }
 
-/// Everything needed to connect except secrets. Saved in connections.json.
+/// Everything needed to connect except secrets. Saved in the app's
+/// connections.toml, in a form of the app's own: what is derived here
+/// reads the connections.json of older versions.
 #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
 #[serde(default)]
 pub struct ConnectSpec {
diff --git a/src/app.rs b/src/app.rs
index c12fbc9..f919635 100644
--- a/src/app.rs
+++ b/src/app.rs
@@ -130,7 +130,8 @@ impl App {
     pub fn new(dirs: AppDirs, loaded: Loaded, backend: Backend) -> Self {
         let source = loaded.source;
         let (settings, settings_file) = loaded.into_parts();
-        let (connections, upgraded) = SavedConnections::load_upgrading(&dirs.connections_file());
+        let loaded_connections = SavedConnections::load(&dirs);
+        let connections = loaded_connections.store;
         let (host_keys, host_keys_error) = match crate::known_hosts::load(&dirs.known_hosts_file())
         {
             Ok(keys) => (keys, None),
@@ -168,7 +169,7 @@ impl App {
             dialog: None,
             pending_secrets: HashMap::new(),
             pending_stores: HashMap::new(),
-            notice: None,
+            notice: loaded_connections.problem,
             host_keys,
             host_keys_error,
             titlebar: TitleBar::default(),
@@ -184,10 +185,15 @@ impl App {
         };
         let tab = app.picker_tab();
         app.tabs.push(tab);
-        // An older file is written in this version once, off the UI thread.
-        if upgraded {
+        // What an older version kept in connections.json is written as
+        // this one keeps it, once, off the UI thread. So is a file in
+        // which a connection had no id yet.
+        if loaded_connections.save {
             app.save_connections();
         }
+        if loaded_connections.save_last_used {
+            app.save_last_used();
+        }
         if adopted {
             app.save_labels();
         }
@@ -1774,6 +1780,7 @@ impl App {
             Action::DeleteConnection(id) => {
                 if let Some(removed) = self.connections.remove(&id) {
                     self.save_connections();
+                    self.save_last_used();
                     // Its label choices go with it. A tab that is still
                     // open on it falls back to Automatic.
                     if self.labels.forget(&id) {
@@ -2963,7 +2970,7 @@ impl App {
             return;
         }
         self.connections.mark_used(&conn, crate::util::now_secs());
-        self.save_connections();
+        self.save_last_used();
     }
 
     /// Saves the label choices on the backend, as the connections are.
@@ -2974,6 +2981,15 @@ impl App {
         });
     }
 
+    /// Saves when each connection last connected, on the backend. A file
+    /// of its own: a connect does not write the file of connections.
+    fn save_last_used(&mut self) {
+        self.backend.send(Command::Save {
+            path: self.dirs.last_used_file(),
+            file: StateFile::LastUsed(self.connections.last_used.clone()),
+        });
+    }
+
     /// Saves the connections on the backend (writing syncs the disk, which
     /// can stall a frame).
     fn save_connections(&mut self) {
diff --git a/src/backend.rs b/src/backend.rs
index d58fda6..e3e189c 100644
--- a/src/backend.rs
+++ b/src/backend.rs
@@ -190,6 +190,8 @@ pub enum Command {
 #[derive(Debug, Clone, PartialEq)]
 pub enum StateFile {
     Connections(SavedConnections),
+    /// When each saved connection last connected, by its id.
+    LastUsed(std::collections::BTreeMap<String, u64>),
     KnownHosts(HostKeys),
     Settings(crate::settings::Settings),
     Labels(crate::labels::Labels),
@@ -199,6 +201,7 @@ impl StateFile {
     fn save(&self, path: &std::path::Path) -> std::io::Result<()> {
         match self {
             Self::Connections(connections) => connections.save(path),
+            Self::LastUsed(times) => crate::util::save_json(path, times),
             Self::KnownHosts(keys) => crate::known_hosts::save(path, keys),
             Self::Settings(settings) => settings.save(path),
             Self::Labels(labels) => labels.save(path),
diff --git a/src/connections.rs b/src/connections.rs
index 20def7b..bc65cd0 100644
--- a/src/connections.rs
+++ b/src/connections.rs
@@ -1,12 +1,17 @@
-//! Saved connections, stored as `connections.json`. Specs only: passwords
-//! live in the OS keyring, never in this file.
+//! Saved connections, stored as `connections.toml` (see `file`). Specs
+//! only: passwords live in the OS keyring, never in this file. When each
+//! connection last connected is kept apart, with the state.
 
+use std::collections::BTreeMap;
 use std::path::Path;
 
 use serde::{Deserialize, Serialize};
 use tabletist_db::ConnectSpec;
 
 use crate::env::Environment;
+use crate::paths::AppDirs;
+
+mod file;
 
 #[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
 pub struct ConnectionId(pub String);
@@ -75,56 +80,44 @@ pub fn access(read_only: Option<bool>, environment: Environment) -> tabletist_db
     }
 }
 
-/// The file's format. 2 dropped the colour a connection used to keep
-/// beside its environment, and the `test` environment.
-pub const VERSION: u32 = 2;
+/// The format of `connections.toml`. 1 and 2 were `connections.json`: 2
+/// dropped the colour a connection used to keep beside its environment,
+/// and the `test` environment. 3 is the first that is TOML.
+pub const VERSION: u32 = file::VERSION;
 
-#[derive(Debug, Clone, PartialEq, Serialize)]
+#[derive(Debug, Clone, Default, PartialEq)]
 pub struct SavedConnections {
-    pub version: u32,
     pub connections: Vec<SavedConnection>,
     /// When each connection last connected, in seconds since the Unix
-    /// epoch, by id. Kept beside the connections so they stay as saved.
-    #[serde(skip_serializing_if = "std::collections::BTreeMap::is_empty")]
-    pub last_used: std::collections::BTreeMap<String, u64>,
+    /// epoch, by id. Kept in a file of its own (`AppDirs::last_used_file`).
+    pub last_used: BTreeMap<String, u64>,
 }
 
-impl Default for SavedConnections {
-    fn default() -> Self {
-        Self {
-            version: VERSION,
-            connections: Vec::new(),
-            last_used: std::collections::BTreeMap::new(),
-        }
-    }
+/// What a start has of the saved connections.
+#[derive(Debug, Default, PartialEq)]
+pub struct Loaded {
+    pub store: SavedConnections,
+    /// Whether `connections.toml` is to be written: the connections came
+    /// from an older version's `connections.json`, or one of them had no
+    /// id in the file and was given one.
+    pub save: bool,
+    /// Whether the times of last use are to be written: they came from
+    /// `connections.json`, which kept them.
+    pub save_last_used: bool,
+    /// Why `connections.toml` could not be read, as the user is told. The
+    /// file was kept aside and the store is empty.
+    pub problem: Option<String>,
 }
 
-/// The file as any version wrote it.
-#[derive(Debug, Deserialize)]
+/// `connections.json` as either of its versions wrote it.
+#[derive(Debug, Default, Deserialize)]
 #[serde(default)]
 struct StoredConnections {
-    /// Files from before the field existed are version 1.
-    #[serde(default = "first_version")]
-    version: u32,
     connections: Vec<StoredConnection>,
-    last_used: std::collections::BTreeMap<String, u64>,
+    last_used: BTreeMap<String, u64>,
 }
 
-fn first_version() -> u32 {
-    1
-}
-
-impl Default for StoredConnections {
-    fn default() -> Self {
-        Self {
-            version: VERSION,
-            connections: Vec::new(),
-            last_used: std::collections::BTreeMap::new(),
-        }
-    }
-}
-
-/// A connection as any version wrote it.
+/// A connection as either version of `connections.json` wrote it.
 #[derive(Debug, Deserialize)]
 struct StoredConnection {
     id: ConnectionId,
@@ -258,7 +251,6 @@ impl StoredConnections {
             })
             .collect();
         let store = SavedConnections {
-            version: VERSION,
             connections,
             last_used: self.last_used,
         };
@@ -267,26 +259,74 @@ impl StoredConnections {
 }
 
 impl SavedConnections {
-    /// Reads the store. An older file is upgraded in memory, each
-    /// connection whose environment is worked out logged.
-    pub fn load(path: &Path) -> Self {
-        Self::load_upgrading(path).0
+    /// The connections a start has: from `connections.toml`, else from an
+    /// older version's `connections.json`, else none. It writes nothing:
+    /// what is to be written, the caller saves on the backend, once.
+    ///
+    /// A `connections.toml` that cannot be read is kept aside, whole, and
+    /// the start has no connections: a file that is wrong somewhere is not
+    /// used in part, so the next save cannot lose what was not read.
+    pub fn load(dirs: &AppDirs) -> Loaded {
+        let path = dirs.connections_file();
+        let text = match std::fs::read(&path) {
+            Ok(bytes) => String::from_utf8(bytes).map_err(|error| error.to_string()),
+            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
+                return Self::load_legacy(&dirs.legacy_connections_file());
+            }
+            Err(error) => Err(error.to_string()),
+        };
+        match text.and_then(|text| file::read(&text).map_err(|problem| problem.to_string())) {
+            Ok(read) => Loaded {
+                store: Self {
+                    connections: read.connections,
+                    last_used: crate::util::load_json(&dirs.last_used_file()),
+                },
+                save: read.named,
+                ..Loaded::default()
+            },
+            Err(problem) => {
+                crate::util::keep_aside(&path, &problem);
+                let name = |path: &Path| {
+                    path.file_name()
+                        .map(|name| name.to_string_lossy().into_owned())
+                        .unwrap_or_default()
+                };
+                Loaded {
+                    problem: Some(format!(
+                        "{} could not be read ({problem}). It was kept beside itself as \
+                         {}.bad, and no connection was loaded.",
+                        name(&path),
+                        name(&path)
+                    )),
+                    ..Loaded::default()
+                }
+            }
+        }
     }
 
-    /// [`SavedConnections::load`], and whether the file was an older
-    /// version: the caller saves it (on the backend) so it is upgraded once.
-    pub fn load_upgrading(path: &Path) -> (Self, bool) {
+    /// What `connections.json` holds, each connection whose environment
+    /// is worked out logged. A file that is there is written as TOML by
+    /// the caller, and is left as it is.
+    fn load_legacy(path: &Path) -> Loaded {
+        let there = path.exists();
         let stored: StoredConnections = crate::util::load_json(path);
-        let old = stored.version < VERSION;
         let (store, notes) = stored.upgrade();
         for note in &notes {
             log::info!("{note}");
         }
-        (store, old)
+        Loaded {
+            save: there,
+            save_last_used: there && !store.last_used.is_empty(),
+            store,
+            problem: None,
+        }
     }
 
+    /// Writes `connections.toml`: the connections, in the one form the app
+    /// writes, atomically.
     pub fn save(&self, path: &Path) -> std::io::Result<()> {
-        crate::util::save_json(path, self)
+        let text = file::write(&self.connections)?;
+        crate::util::write_atomic(path, text.as_bytes())
     }
 
     pub fn get(&self, id: &ConnectionId) -> Option<&SavedConnection> {
diff --git a/src/connections/file.rs b/src/connections/file.rs
new file mode 100644
index 0000000..1b1d7af
--- /dev/null
+++ b/src/connections/file.rs
@@ -0,0 +1,794 @@
+//! `connections.toml` as text: the one form the app writes, and what it
+//! reads.
+//!
+//! The app writes the file itself, as it writes `settings.toml`: the same
+//! text for the same connections, a key on each line. It reads the file
+//! strictly. A key it does not know is an error and not a key it skips: in
+//! a file a person edits, `read_onyl = true` must not pass for a
+//! connection that may write.
+
+use std::collections::HashSet;
+use std::path::{Path, PathBuf};
+
+use serde::Deserialize;
+use tabletist_db::{ConnectSpec, Driver, SshAuth, SshSpec, TlsMode};
+
+use super::{ConnectionId, PasswordMode, SavedConnection};
+use crate::env::Environment;
+use crate::util::toml_string;
+
+/// The file's format. 3 is the first that is TOML: 1 and 2 were
+/// `connections.json`.
+pub(super) const VERSION: u32 = 3;
+
+const HEADER: &str = "\
+# Tabletist's saved connections. No password is kept here: those are in the
+# system's keyring.
+#
+# Edit it while Tabletist is closed. A connection without an `id` line gets
+# one at the next start. Tabletist writes the whole file again when a
+# connection changes, and keeps no comment but these.
+";
+
+/// What the file holds, read from its text.
+#[derive(Debug, PartialEq)]
+pub(super) struct Read {
+    pub connections: Vec<SavedConnection>,
+    /// Whether a connection had no id and was given one: the file is to
+    /// be written again, so that it keeps the id.
+    pub named: bool,
+}
+
+/// Why a file's text could not be read: the line, where the parser gives
+/// one, and what is wrong there.
+#[derive(Debug, PartialEq, Eq)]
+pub(super) struct Problem {
+    pub line: Option<usize>,
+    pub what: String,
+}
+
+impl std::fmt::Display for Problem {
+    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
+        match self.line {
+            Some(line) => write!(f, "line {line}: {}", self.what),
+            None => f.write_str(&self.what),
+        }
+    }
+}
+
+#[derive(Deserialize)]
+#[serde(deny_unknown_fields)]
+struct File {
+    version: u32,
+    #[serde(default)]
+    connections: Vec<Entry>,
+}
+
+/// A connection as the file has it: its spec's keys beside its own, and
+/// the SSH tunnel as a table of its own.
+#[derive(Deserialize)]
+#[serde(deny_unknown_fields)]
+struct Entry {
+    id: Option<String>,
+    name: String,
+    #[serde(default)]
+    environment: Environment,
+    read_only: Option<bool>,
+    #[serde(default)]
+    password: PasswordMode,
+    #[serde(default)]
+    ssh_secret: PasswordMode,
+    driver: Driver,
+    #[serde(default)]
+    host: String,
+    port: Option<u16>,
+    #[serde(default)]
+    user: String,
+    #[serde(default)]
+    database: String,
+    sqlite_path: Option<PathBuf>,
+    #[serde(default)]
+    tls: TlsMode,
+    ca_file: Option<PathBuf>,
+    ssh: Option<Tunnel>,
+}
+
+#[derive(Deserialize)]
+#[serde(deny_unknown_fields)]
+struct Tunnel {
+    host: String,
+    port: Option<u16>,
+    #[serde(default)]
+    user: String,
+    auth: Auth,
+    /// The key file, for `auth = "key-file"` and no other.
+    key: Option<PathBuf>,
+}
+
+#[derive(Deserialize, Clone, Copy, PartialEq, Eq)]
+#[serde(rename_all = "kebab-case")]
+enum Auth {
+    Password,
+    KeyFile,
+    Agent,
+}
+
+/// The line `offset` is on in `text`, counted from 1.
+fn line_of(text: &str, offset: usize) -> usize {
+    let before = &text.as_bytes()[..offset.min(text.len())];
+    before.iter().filter(|byte| **byte == b'\n').count() + 1
+}
+
+/// Reads the file's text. Any error is the whole file's: nothing of a file
+/// that is wrong somewhere is used, so nothing of it is lost to a save.
+pub(super) fn read(text: &str) -> Result<Read, Problem> {
+    let file: File = toml::from_str(text).map_err(|error| Problem {
+        line: error.span().map(|span| line_of(text, span.start)),
+        what: error.message().to_owned(),
+    })?;
+    let whole = |what: String| Problem { line: None, what };
+    if file.version > VERSION {
+        return Err(whole(format!(
+            "it is version {} of the file, written by a newer Tabletist; this one reads \
+             version {VERSION}",
+            file.version
+        )));
+    }
+    if file.version < VERSION {
+        return Err(whole(format!(
+            "it says version {}, and the first version of this file is {VERSION}",
+            file.version
+        )));
+    }
+    let mut seen = HashSet::new();
+    let mut named = false;
+    let mut connections = Vec::with_capacity(file.connections.len());
+    for entry in file.connections {
+        let connection = entry.connection(&mut named).map_err(&whole)?;
+        if !seen.insert(connection.id.clone()) {
+            return Err(whole(format!(
+                "two connections have the id {:?} (the second is {:?}); remove the line from \
+                 one of them and it gets a new one",
+                connection.id.0, connection.name
+            )));
+        }
+        connections.push(connection);
+    }
+    Ok(Read { connections, named })
+}
+
+impl Entry {
+    /// The connection the entry describes. `named` is set when it had no
+    /// id and was given one.
+    fn connection(self, named: &mut bool) -> Result<SavedConnection, String> {
+        let id = match self.id {
+            Some(id) if !id.trim().is_empty() => ConnectionId(id),
+            Some(_) => return Err(format!("connection {:?} has an empty id", self.name)),
+            None => {
+                *named = true;
+                ConnectionId::new()
+            }
+        };
+        let ssh = match self.ssh {
+            Some(tunnel) => Some(tunnel.spec(&self.name)?),
+            None => None,
+        };
+        Ok(SavedConnection {
+            id,
+            name: self.name,
+            environment: self.environment,
+            read_only: self.read_only,
+            password: self.password,
+            ssh_secret: self.ssh_secret,
+            spec: ConnectSpec {
+                driver: self.driver,
+                host: self.host,
+                port: self.port.unwrap_or(self.driver.default_port()),
+                user: self.user,
+                database: self.database,
+                sqlite_path: self.sqlite_path,
+                tls: self.tls,
+                ca_file: self.ca_file,
+                ssh,
+            },
+        })
+    }
+}
+
+impl Tunnel {
+    fn spec(self, connection: &str) -> Result<SshSpec, String> {
+        let auth = match (self.auth, self.key) {
+            (Auth::KeyFile, key) => SshAuth::KeyFile {
+                // No key named takes `~/.ssh/config`'s IdentityFile.
+                path: key.unwrap_or_default(),
+            },
+            (Auth::Password, None) => SshAuth::Password,
+            (Auth::Agent, None) => SshAuth::Agent,
+            (Auth::Password | Auth::Agent, Some(_)) => {
+                return Err(format!(
+                    "connection {connection:?}: an SSH `key` goes with `auth = \"key-file\"`"
+                ));
+            }
+        };
+        Ok(SshSpec {
+            host: self.host,
+            port: self.port,
+            user: self.user,
+            auth,
+        })
+    }
+}
+
+/// A table's lines as they are written: its keys aligned.
+#[derive(Default)]
+struct Lines(Vec<(&'static str, String)>);
+
+impl Lines {
+    fn text(&mut self, key: &'static str, value: &str) {
+        self.0.push((key, toml_string(value)));
+    }
+
+    fn bare(&mut self, key: &'static str, value: impl std::fmt::Display) {
+        self.0.push((key, value.to_string()));
+    }
+
+    /// A path, which the file can hold only as text.
+    fn path(&mut self, key: &'static str, path: &Path) -> std::io::Result<()> {
+        let Some(text) = path.to_str() else {
+            return Err(std::io::Error::other(format!(
+                "{} is not valid UTF-8, so it cannot be saved",
+                path.display()
+            )));
+        };
+        self.text(key, text);
+        Ok(())
+    }
+
+    fn write(&self, out: &mut String) {
+        let width = self.0.iter().map(|(key, _)| key.len()).max().unwrap_or(0);
+        for (key, value) in &self.0 {
+            out.push_str(&format!("{key:width$} = {value}\n"));
+        }
+    }
+}
+
+fn password(mode: PasswordMode) -> &'static str {
+    match mode {
+        PasswordMode::None => "none",
+        PasswordMode::Keyring => "keyring",
+        PasswordMode::Ask => "ask",
+    }
+}
+
+fn driver(driver: Driver) -> &'static str {
+    match driver {
+        Driver::Postgres => "postgres",
+        Driver::MySql => "mysql",
+        Driver::Sqlite => "sqlite",
+    }
+}
+
+fn tls(mode: TlsMode) -> &'static str {
+    match mode {
+        TlsMode::Disable => "disable",
+        TlsMode::Prefer => "prefer",
+        TlsMode::Require => "require",
+        TlsMode::VerifyCa => "verify-ca",
+        TlsMode::VerifyFull => "verify-full",
+    }
+}
+
+/// The file's text for `connections`: the one form the app writes. A key
+/// is left out where the reader gives the same value for a missing one,
+/// except a server's own (host, port, user, database, TLS), which a person
+/// looks for.
+pub(super) fn write(connections: &[SavedConnection]) -> std::io::Result<String> {
+    let mut out = format!("{HEADER}version = {VERSION}\n");
+    for connection in connections {
+        let spec = &connection.spec;
+        let server = match spec.driver {
+            Driver::Postgres | Driver::MySql => true,
+            Driver::Sqlite => false,
+        };
+        let mut lines = Lines::default();
+        lines.text("id", &connection.id.0);
+        lines.text("name", &connection.name);
+        lines.text("environment", connection.environment.key());
+        if let Some(read_only) = connection.read_only {
+            lines.bare("read_only", read_only);
+        }
+        if server || connection.password != PasswordMode::None {
+            lines.text("password", password(connection.password));
+        }
+        if spec.ssh.is_some() || connection.ssh_secret != PasswordMode::None {
+            lines.text("ssh_secret", password(connection.ssh_secret));
+        }
+        lines.text("driver", driver(spec.driver));
+        if let Some(path) = &spec.sqlite_path {
+            lines.path("sqlite_path", path)?;
+        }
+        if server || !spec.host.is_empty() {
+            lines.text("host", &spec.host);
+        }
+        if server || spec.port != spec.driver.default_port() {
+            lines.bare("port", spec.port);
+        }
+        if server || !spec.user.is_empty() {
+            lines.text("user", &spec.user);
+        }
+        if server || !spec.database.is_empty() {
+            lines.text("database", &spec.database);
+        }
+        if server || spec.tls != TlsMode::default() {
+            lines.text("tls", tls(spec.tls));
+        }
+        if let Some(path) = &spec.ca_file {
+            lines.path("ca_file", path)?;
+        }
+        out.push_str("\n[[connections]]\n");
+        lines.write(&mut out);
+        let Some(ssh) = &spec.ssh else {
+            continue;
+        };
+        let mut lines = Lines::default();
+        lines.text("host", &ssh.host);
+        if let Some(port) = ssh.port {
+            lines.bare("port", port);
+        }
+        if !ssh.user.is_empty() {
+            lines.text("user", &ssh.user);
+        }
+        match &ssh.auth {
+            SshAuth::Password => lines.text("auth", "password"),
+            SshAuth::Agent => lines.text("auth", "agent"),
+            SshAuth::KeyFile { path } => {
+                lines.text("auth", "key-file");
+                if !path.as_os_str().is_empty() {
+                    lines.path("key", path)?;
+                }
+            }
+        }
+        out.push_str("\n[connections.ssh]\n");
+        lines.write(&mut out);
+    }
+    Ok(out)
+}
+
+#[cfg(test)]
+mod tests {
+    use super::*;
+
+    fn saved(id: &str, name: &str, spec: ConnectSpec) -> SavedConnection {
+        SavedConnection {
+            id: ConnectionId(id.into()),
+            name: name.into(),
+            environment: Environment::None,
+            read_only: None,
+            password: PasswordMode::None,
+            ssh_secret: PasswordMode::None,
+            spec,
+        }
+    }
+
+    fn server(driver: Driver, host: &str, database: &str) -> ConnectSpec {
+        ConnectSpec {
+            driver,
+            host: host.into(),
+            port: driver.default_port(),
+            user: "app".into(),
+            database: database.into(),
+            ..ConnectSpec::default()
+        }
+    }
+
+    /// The three kinds of connection, with everything a file can say of
+    /// the first.
+    fn bookshop() -> Vec<SavedConnection> {
+        let production = SavedConnection {
+            environment: Environment::Production,
+            read_only: Some(true),
+            password: PasswordMode::Keyring,
+            ssh_secret: PasswordMode::Ask,
+            spec: ConnectSpec {
+                tls: TlsMode::VerifyFull,
+                ca_file: Some("/etc/ssl/bookshop-ca.pem".into()),
+                ssh: Some(SshSpec {
+                    host: "bastion.example.com".into(),
+                    port: Some(2222),
+                    user: "deploy".into(),
+                    auth: SshAuth::KeyFile {
+                        path: "/home/ada/.ssh/id_ed25519".into(),
+                    },
+                }),
+                ..server(Driver::Postgres, "db.example.com", "bookshop_production")
+            },
+            ..saved("prod", "Bookshop production", ConnectSpec::default())
+        };
+        let preview = SavedConnection {
+            environment: Environment::Staging,
+            password: PasswordMode::Ask,
+            ..saved(
+                "preview",
+                "Bookshop preview",
+                server(Driver::MySql, "preview.example.com", "bookshop_preview"),
+            )
+        };
+        let file = SavedConnection {
+            environment: Environment::Local,
+            ..saved(
+                "file",
+                "Bookshop on disk",
+                ConnectSpec::sqlite("/home/ada/bookshop.db"),
+            )
+        };
+        vec![production, preview, file]
+    }
+
+    #[test]
+    fn the_file_is_written_in_one_form() {
+        let expected = format!(
+            "{HEADER}{}",
+            r#"version = 3
+
+[[connections]]
+id          = "prod"
+name        = "Bookshop production"
+environment = "production"
+read_only   = true
+password    = "keyring"
+ssh_secret  = "ask"
+driver      = "postgres"
+host        = "db.example.com"
+port        = 5432
+user        = "app"
+database    = "bookshop_production"
+tls         = "verify-full"
+ca_file     = "/etc/ssl/bookshop-ca.pem"
+
+[connections.ssh]
+host = "bastion.example.com"
+port = 2222
+user = "deploy"
+auth = "key-file"
+key  = "/home/ada/.ssh/id_ed25519"
+
+[[connections]]
+id          = "preview"
+name        = "Bookshop preview"
+environment = "staging"
+password    = "ask"
+driver      = "mysql"
+host        = "preview.example.com"
+port        = 3306
+user        = "app"
+database    = "bookshop_preview"
+tls         = "prefer"
+
+[[connections]]
+id          = "file"
+name        = "Bookshop on disk"
+environment = "local"
+driver      = "sqlite"
+sqlite_path = "/home/ada/bookshop.db"
+"#
+        );
+        assert_eq!(write(&bookshop()).unwrap(), expected);
+        // No connection is a file all the same, with its version.
+        assert_eq!(write(&[]).unwrap(), format!("{HEADER}version = 3\n"));
+    }
+
+    #[test]
+    fn what_is_written_is_read_back_as_it_was() {
+        let mut all = bookshop();
+        let tunnel = |auth: SshAuth| SshSpec {
+            host: "bastion".into(),
+            port: None,
+            user: String::new(),
+            auth,
+        };
+        // What the writer leaves out where it can: each must come back.
+        let odd = [
+            // A file with a server's keys set, each away from what a
+            // missing one reads as.
+            ConnectSpec {
+                host: "elsewhere".into(),
+                port: 7,
+                user: "someone".into(),
+                database: "something".into(),
+                tls: TlsMode::Disable,
+                ..ConnectSpec::sqlite("/odd.db")
+            },
+            // A server with nothing filled in.
+            ConnectSpec {
+                host: String::new(),
+                port: 0,
+                user: String::new(),
+                ..server(Driver::Postgres, "", "")
+            },
+            ConnectSpec {
+                ssh: Some(tunnel(SshAuth::Password)),
+                ..server(Driver::MySql, "db", "shop")
+            },
+            ConnectSpec {
+                ssh: Some(tunnel(SshAuth::Agent)),
+                ..server(Driver::Postgres, "db", "shop")
+            },
+            // A key file left to `~/.ssh/config`.
+            ConnectSpec {
+                ssh: Some(tunnel(SshAuth::KeyFile {
+                    path: PathBuf::new(),
+                })),
+                ..server(Driver::Postgres, "db", "shop")
+            },
+            // A path as Windows writes one.
+            ConnectSpec::sqlite(r"C:\Users\ada\My Databases\bookshop.db"),
+        ];
+        for (index, spec) in odd.into_iter().enumerate() {
+            all.push(saved(&format!("odd-{index}"), "Odd", spec));
+        }
+        // Text that needs care in a TOML string.
+        for (index, name) in [
+            "Zoë's \"shop\"",
+            r"back\slash",
+            "two\nlines\tand a tab",
+            "a bell \u{7} and a null \u{0}",
+            "# not a comment",
+            "",
+            "  spaces  ",
+            "[[connections]]",
+        ]
+        .into_iter()
+        .enumerate()
+        {
+            all.push(saved(
+                &format!("name-{index}"),
+                name,
+                ConnectSpec::sqlite("/x.db"),
+            ));
+        }
+        // Each choice a connection can carry beside its spec.
+        for (index, (read_only, password, ssh_secret)) in [
+            (Some(false), PasswordMode::None, PasswordMode::Keyring),
+            (Some(true), PasswordMode::Ask, PasswordMode::None),
+            (None, PasswordMode::Keyring, PasswordMode::Ask),
+        ]
+        .into_iter()
+        .enumerate()
+        {
+            for environment in Environment::ALL {
+                all.push(SavedConnection {
+                    environment,
+                    read_only,
+                    password,
+                    ssh_secret,
+                    ..saved(
+                        &format!("choice-{index}-{}", environment.key()),
+                        "Choices",
+                        ConnectSpec::sqlite("/x.db"),
+                    )
+                });
+            }
+        }
+        let text = write(&all).unwrap();
+        let read = read(&text).unwrap_or_else(|problem| panic!("{problem}\n{text}"));
+        assert!(!read.named);
+        assert_eq!(read.connections.len(), all.len());
+        for (read, written) in read.connections.iter().zip(&all) {
+            assert_eq!(read, written, "{text}");
+        }
+        // And written again it is the same text.
+        assert_eq!(write(&read.connections).unwrap(), text);
+    }
+
+    /// What a person writes: the keys that matter, and no more.
+    #[test]
+    fn a_key_that_is_left_out_reads_as_its_default() {
+        let text = r#"
+version = 3
+
+[[connections]]
+id = "one"
+name = "By hand"
+driver = "postgres"
+host = "db.example.com"
+database = "bookshop"
+
+[[connections]]
+id = "two"
+name = "Also by hand"
+driver = "mysql"
+
+[connections.ssh]
+host = "bastion"
+auth = "key-file"
+"#;
+        let read = read(text).unwrap();
+        let [one, two] = read.connections.as_slice() else {
+            panic!("{read:?}");
+        };
+        assert_eq!(
+            *one,
+            saved(
+                "one",
+                "By hand",
+                ConnectSpec {
+                    user: String::new(),
+                    ..server(Driver::Postgres, "db.example.com", "bookshop")
+                }
+            )
+        );
+        assert_eq!(two.spec.port, 3306);
+        assert_eq!(two.spec.host, "");
+        assert_eq!(
+            two.spec.ssh,
+            Some(SshSpec {
+                host: "bastion".into(),
+                port: None,
+                user: String::new(),
+                auth: SshAuth::KeyFile {
+                    path: PathBuf::new()
+                },
+            })
+        );
+        // A file with its version and nothing else has no connections.
+        assert_eq!(read_ok("version = 3\n"), Vec::new());
+    }
+
+    fn read_ok(text: &str) -> Vec<SavedConnection> {
+        read(text).unwrap().connections
+    }
+
+    /// The problem with a file that is `version = 3` and then `rest`.
+    fn problem(rest: &str) -> Problem {
+        read(&format!("version = 3\n{rest}")).unwrap_err()
+    }
+
+    #[test]
+    fn a_key_the_file_does_not_have_is_an_error_at_its_line() {
+        let entry = "\n[[connections]]\nid = \"a\"\nname = \"A\"\ndriver = \"sqlite\"\n";
+        for (rest, line, what) in [
+            // A key spelled wrong must not pass for one left out.
+            (
+                format!("{entry}read_onyl = true\n"),
+                7,
+                "unknown field `read_onyl`",
+            ),
+            (
+                format!("{entry}\n[connections.agents]\naccess = \"read\"\n"),
+                8,
+                "unknown field `agents`",
+            ),
+            (
+                format!(
+                    "{entry}\n[connections.ssh]\nhost = \"b\"\nauth = \"agent\"\nkeyfile = \"k\"\n"
+                ),
+                11,
+                "unknown field `keyfile`",
+            ),
+            (
+                "\nlast_used = 1\n".to_owned(),
+                3,
+                "unknown field `last_used`",
+            ),
+            // A value that is none of the key's.
+            (
+                entry.replace("\"sqlite\"", "\"oracle\""),
+                6,
+                "unknown variant `oracle`",
+            ),
+            (
+                format!("{entry}environment = \"test\"\n"),
+                7,
+                "unknown variant `test`",
+            ),
+            (format!("{entry}tls = \"on\"\n"), 7, "unknown variant `on`"),
+            (
+                format!("{entry}password = \"hunter2\"\n"),
+                7,
+                "unknown variant `hunter2`",
+            ),
+            (format!("{entry}port = 70000\n"), 7, "invalid value"),
+            (format!("{entry}read_only = \"yes\"\n"), 7, "invalid type"),
+            // A key that must be there.
+            (
+                "\n[[connections]]\nid = \"a\"\ndriver = \"sqlite\"\n".to_owned(),
+                3,
+                "missing field `name`",
+            ),
+            (
+                "\n[[connections]]\nid = \"a\"\nname = \"A\"\n".to_owned(),
+                3,
+                "missing field `driver`",
+            ),
+            // Text that is no TOML.
+            ("\n[[connections]\n".to_owned(), 3, ""),
+        ] {
+            let problem = problem(&rest);
+            assert_eq!(problem.line, Some(line), "{rest}: {problem}");
+            assert!(problem.what.contains(what), "{rest}: {problem}");
+            assert!(
+                problem.to_string().starts_with(&format!("line {line}: ")),
+                "{problem}"
+            );
+        }
+    }
+
+    #[test]
+    fn the_file_says_which_version_it_is() {
+        let said = |text: &str| read(text).unwrap_err().to_string();
+        assert!(said("").contains("missing field `version`"), "{}", said(""));
+        assert_eq!(
+            said("version = 4\n"),
+            "it is version 4 of the file, written by a newer Tabletist; this one reads version 3"
+        );
+        assert_eq!(
+            said("version = 2\n"),
+            "it says version 2, and the first version of this file is 3"
+        );
+    }
+
+    #[test]
+    fn every_connection_has_an_id_of_its_own() {
+        let entry = |id: &str, name: &str| {
+            format!("\n[[connections]]\n{id}name = \"{name}\"\ndriver = \"sqlite\"\n")
+        };
+        // Two with one id, as a block copied by hand has.
+        let copied = problem(&format!(
+            "{}{}",
+            entry("id = \"a\"\n", "First"),
+            entry("id = \"a\"\n", "Second")
+        ));
+        assert_eq!(
+            copied.to_string(),
+            "two connections have the id \"a\" (the second is \"Second\"); remove the line \
+             from one of them and it gets a new one"
+        );
+        let empty = problem(&entry("id = \" \"\n", "Blank"));
+        assert_eq!(empty.to_string(), "connection \"Blank\" has an empty id");
+        // None at all: each gets its own, and the file is to be written.
+        let text = format!("version = 3\n{}{}", entry("", "First"), entry("", "Second"));
+        let read = read(&text).unwrap();
+        assert!(read.named);
+        let [first, second] = read.connections.as_slice() else {
+            panic!("{read:?}");
+        };
+        assert_ne!(first.id, second.id);
+        assert_eq!(first.id.0.len(), 36, "{:?}", first.id);
+    }
+
+    #[test]
+    fn an_ssh_key_goes_with_a_key_file_and_nothing_else() {
+        let tunnel = |auth: &str| {
+            format!(
+                "\n[[connections]]\nid = \"a\"\nname = \"Tunnelled\"\ndriver = \"postgres\"\n\n\
+                 [connections.ssh]\nhost = \"bastion\"\nauth = \"{auth}\"\nkey = \"/id\"\n"
+            )
+        };
+        for auth in ["password", "agent"] {
+            assert_eq!(
+                problem(&tunnel(auth)).to_string(),
+                "connection \"Tunnelled\": an SSH `key` goes with `auth = \"key-file\"`",
+                "{auth}"
+            );
+        }
+        let read = read(&format!("version = 3\n{}", tunnel("key-file"))).unwrap();
+        let ssh = read.connections[0].spec.ssh.clone().unwrap();
+        assert_eq!(ssh.auth, SshAuth::KeyFile { path: "/id".into() });
+    }
+
+    /// TOML is text, and so is JSON, which could not hold such a path
+    /// either: the save fails and says which path.
+    #[cfg(unix)]
+    #[test]
+    fn a_path_that_is_no_text_is_not_written() {
+        use std::os::unix::ffi::OsStringExt;
+        let path = PathBuf::from(std::ffi::OsString::from_vec(b"/caf\xC3\x28.db".to_vec()));
+        let error = write(&[saved("a", "A", ConnectSpec::sqlite(path))]).unwrap_err();
+        assert!(
+            error
+                .to_string()
+                .ends_with("is not valid UTF-8, so it cannot be saved"),
+            "{error}"
+        );
+    }
+}
diff --git a/src/paths.rs b/src/paths.rs
index b3fdd91..42c8653 100644
--- a/src/paths.rs
+++ b/src/paths.rs
@@ -70,6 +70,13 @@ impl AppDirs {
     }
 
     pub fn connections_file(&self) -> PathBuf {
+        self.config.join("connections.toml")
+    }
+
+    /// Where versions before the TOML file kept the connections, and when
+    /// each last connected. Read at a start that finds no
+    /// `connections.toml`, and never written.
+    pub fn legacy_connections_file(&self) -> PathBuf {
         self.config.join("connections.json")
     }
 
@@ -87,6 +94,13 @@ impl AppDirs {
         self.state.join("labels.json")
     }
 
+    /// When each saved connection last connected. Of this computer, and
+    /// written at every connect: kept with the state, so that the file of
+    /// connections is written only when one of them changes.
+    pub fn last_used_file(&self) -> PathBuf {
+        self.state.join("last_used.json")
+    }
+
     pub fn log_file(&self) -> PathBuf {
         self.state.join("tabletist.log")
     }
````

- [ ] **Step 4: Run the tests**

Run: `cargo check --locked`, which shows that `Cargo.lock` needs no change. Then `cargo test --locked --lib connections::`, then `cargo test --locked --lib`

Expected: `27 passed` for `connections::` (19 of the store's, 8 of the format's), then `2381 passed; 0 failed; 20 ignored` for the whole library.

- [ ] **Step 5: Commit**

```bash
cargo fmt --all
cargo clippy --locked --workspace --all-targets -- -D warnings
git diff --stat Cargo.lock
git add Cargo.toml crates/tabletist-db/Cargo.toml crates/tabletist-db/src/spec.rs src
git commit -m "Keep saved connections in connections.toml"
```

`git diff --stat Cargo.lock` prints nothing.

---

### Task 3: The website

The page of files gains the connections file: an example, its keys, what a missing key reads as, how strictly the file is read, and what became of the old one. A test reads the example off the page and holds it to the code: it is a file the app reads, and its first connection is written by the app just as the page shows.

**Files:**
- Modify: `docs/_reference/settings-and-files.md`, `docs/_guide/macos.md`
- Test: `src/connections/file.rs`

- [ ] **Step 1: Write the failing test**

````diff
diff --git a/src/connections/file.rs b/src/connections/file.rs
index 1b1d7af..e3dffae 100644
--- a/src/connections/file.rs
+++ b/src/connections/file.rs
@@ -776,6 +776,28 @@ auth = "key-file"
         assert_eq!(ssh.auth, SshAuth::KeyFile { path: "/id".into() });
     }
 
+    /// The example on the website's page of files is a file the app reads,
+    /// and its first connection is one the app writes just so.
+    #[test]
+    fn the_websites_example_is_read_and_written_as_it_shows() {
+        // Windows checks the page out with CRLF line ends.
+        let page =
+            include_str!("../../docs/_reference/settings-and-files.md").replace("\r\n", "\n");
+        let (_, section) = page.split_once("## The connections file").unwrap();
+        let (_, block) = section.split_once("```toml\n").unwrap();
+        let (example, _) = block.split_once("```").unwrap();
+        let read = read(example).unwrap();
+        // The second connection is shown without an id.
+        assert!(read.named);
+        let [production, on_disk] = read.connections.as_slice() else {
+            panic!("{read:?}");
+        };
+        assert_eq!(on_disk.name, "Bookshop on disk");
+        let written = write(std::slice::from_ref(production)).unwrap();
+        let (shown, _) = example.split_once("\n[[connections]]\nname").unwrap();
+        assert_eq!(written, format!("{HEADER}{shown}"));
+    }
+
     /// TOML is text, and so is JSON, which could not hold such a path
     /// either: the save fails and says which path.
     #[cfg(unix)]
````

- [ ] **Step 2: Run it to see it fail**

Run: `cargo test --locked --lib connections::file::tests::the_websites`

Expected: FAIL. The page has no such section yet:

```
called `Option::unwrap()` on a `None` value
test result: FAILED. 0 passed; 1 failed
```

- [ ] **Step 3: Write the page**

````diff
diff --git a/docs/_guide/macos.md b/docs/_guide/macos.md
index 7b62585..cb9a945 100644
--- a/docs/_guide/macos.md
+++ b/docs/_guide/macos.md
@@ -186,7 +186,7 @@ Everything Tabletist keeps is in one folder:
 ~/Library/Application Support/dev.tabletist.Tabletist
 ```
 
-It holds `settings.toml`, `connections.json`, `known_hosts.json`, the
+It holds `settings.toml`, `connections.toml`, `known_hosts.json`, the
 `themes` folder and the logs. **Settings** shows where the settings file
 is, and **Reveal in Finder** takes you there.
 
diff --git a/docs/_reference/settings-and-files.md b/docs/_reference/settings-and-files.md
index e352c28..d90d842 100644
--- a/docs/_reference/settings-and-files.md
+++ b/docs/_reference/settings-and-files.md
@@ -1,6 +1,6 @@
 ---
 title: Settings and Files
-description: The Settings window, every key of settings.toml, where Tabletist keeps its files, and its command-line options.
+description: The Settings window, every key of settings.toml and of connections.toml, where Tabletist keeps its files, and its command-line options.
 nav_order: 2
 ---
 
@@ -76,15 +76,87 @@ On Linux, `XDG_CONFIG_HOME` and `XDG_STATE_HOME` move the two folders.
 | File | Folder | Holds |
 | --- | --- | --- |
 | `settings.toml` | Settings | The options above |
-| `connections.json` | Settings | Your saved connections. It never holds a password |
+| `connections.toml` | Settings | Your saved connections. It never holds a password |
 | `known_hosts.json` | Settings | The SSH host keys you trusted |
 | `themes/` | Settings | Palette files |
+| `last_used.json` | State | When each saved connection last connected |
 | `labels.json` | State | The column you chose to label each table's rows in a foreign key's list, by saved connection and, on PostgreSQL, by database |
 | `tabletist.log`, `panic.log` | State | Logs |
 
 Passwords are kept in the system keyring, never in these files.
 [Read about passwords]({% link _reference/connections.md %}#passwords).
 
+## The connections file
+
+Saved connections are kept in `connections.toml`, one `[[connections]]`
+table for each.
+
+```toml
+version = 3
+
+[[connections]]
+id          = "6a0c1f0e-5a52-4b7b-9c56-3f0c8f2f7a11"
+name        = "Bookshop production"
+environment = "production"
+read_only   = true
+password    = "keyring"
+ssh_secret  = "ask"
+driver      = "postgres"
+host        = "db.example.com"
+port        = 5432
+user        = "app"
+database    = "bookshop_production"
+tls         = "verify-full"
+ca_file     = "/etc/ssl/bookshop-ca.pem"
+
+[connections.ssh]
+host = "bastion.example.com"
+port = 2222
+user = "deploy"
+auth = "key-file"
+key  = "/home/ada/.ssh/id_ed25519"
+
+[[connections]]
+name        = "Bookshop on disk"
+environment = "local"
+driver      = "sqlite"
+sqlite_path = "/home/ada/bookshop.db"
+```
+
+| Key | Values | Without it |
+| --- | --- | --- |
+| `id` | Any text that no other connection has | Tabletist gives the connection one at the next start |
+| `name` | Text | Required |
+| `environment` | `local`, `dev`, `staging`, `production`, `none` | `none` |
+| `read_only` | `true`, `false` | As the environment has it: read-only on production |
+| `password` | `none`, `keyring`, `ask` | `none` |
+| `ssh_secret` | `none`, `keyring`, `ask` | `none` |
+| `driver` | `postgres`, `mysql`, `sqlite` | Required |
+| `host`, `user`, `database` | Text | Empty |
+| `port` | A number up to 65535 | 5432 for PostgreSQL, 3306 for MySQL |
+| `tls` | `disable`, `prefer`, `require`, `verify-ca`, `verify-full` | `prefer` |
+| `ca_file` | The path of a certificate file | None |
+| `sqlite_path` | The path of a SQLite file | None |
+
+A tunnel is the connection's `[connections.ssh]` table: `host`, and
+optionally `port` and `user`, `auth` (`password`, `key-file` or `agent`),
+and with `key-file` the `key` to use.
+
+Edit the file while Tabletist is closed. Tabletist writes the whole file
+again when you change a connection, so a comment you add is not kept.
+
+Tabletist reads the file strictly. A key it does not know, or a value a key
+cannot have, is an error and not something it skips: a line such as
+`read_onyl = true` must not pass for a connection that may write. When the
+file cannot be read, Tabletist starts with no connections, says which line
+is wrong, and keeps the file beside itself as `connections.toml.bad`. Fix
+the line and rename the file back.
+
+Versions before this one kept the connections in `connections.json`.
+Tabletist reads that file once, at the first start that finds no
+`connections.toml`, writes the two files above from it, and leaves it
+where it is.
+
 ## Command-line options
 
 | Option | Does |
````

- [ ] **Step 4: Run the test, and build the site**

Run: `cargo test --locked --lib connections::file::tests::the_websites`, then `docs/bin/check`

Expected: `1 passed`, and `docs/bin/check` ends with `HTML-Proofer finished successfully.`

- [ ] **Step 5: Commit**

```bash
cargo fmt --all
git add docs/_reference/settings-and-files.md docs/_guide/macos.md src/connections/file.rs
git commit -m "Say on the website how connections.toml is written and read"
```

---

### Task 4: The whole workspace, and the pull request

**Files:** none.

- [ ] **Step 1: Run every check `AGENTS.md` names**

```bash
cargo fmt --all --check
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo test --locked --workspace --all-targets
RUSTDOCFLAGS='-D warnings' cargo doc --locked --workspace --no-deps
```

Expected: all four pass. The library has `2382 passed; 0 failed; 20 ignored`.

- [ ] **Step 2: Open the pull request**

Its description says:

- that connections move to `connections.toml`, with the example, and that the first start after the update writes it from `connections.json`, which is left where it is;
- that the file is read strictly, what happens to one that cannot be read, and what that means for a downgrade;
- that when a connection last connected is now `last_used.json` in the state directory;
- that an `id` may be left out of a connection written by hand;
- what is not done: the app was not started by hand on a real profile, and the file is not watched;
- that this is the answer to open question 5 of the MCP server's plan, and that Phase 1b's `[connections.agents]` builds on it.

The website changes with it: Task 3.

## Self-review

- **Nothing a user set is lost.** `an_older_file_is_read_once_and_written_as_the_new_ones` loads the version 1 fixture (fourteen connections, with colours, the `test` environment, tunnels and a time of last use), writes both new files and reads them back equal. `what_is_written_is_read_back_as_it_was` does the same for every key the writer may leave out, each kind of tunnel, a Windows path, names with quotes, newlines and control characters, and every environment with every choice.
- **The three choices agreed before the plan.** A connection is found by its id (`every_connection_has_an_id_of_its_own`). The times of last use are in the state directory (`the_times_of_last_use_are_kept_apart_from_the_connections`, `a_connect_writes_when_it_was_and_not_the_connections`). Comments are not kept, and the file's header says so.
- **What goes beyond them, and is the user's to refuse.** The reader is strict, with what that means for a downgrade. An `id` may be left out. A file that cannot be read is said in the notice at the start. Each is in "What this run settles", with its reason.
- **Names.** `Loaded { store, save, save_last_used, problem }`, `file::read`, `file::write`, `file::Read { connections, named }`, `file::Problem`, `AppDirs::legacy_connections_file`, `AppDirs::last_used_file`, `StateFile::LastUsed`, `Environment::key` and `util::toml_string` are spelled the same in every task.
- **Every commit stands.** After Task 1 nothing a user sees changed. After Task 2 the app reads and writes the new files, and the website still names the old one, for one commit. After Task 3 it does not.
- **Not reviewed by a second reader.** The draft was not given to a separate reviewer before this plan was written, as the second run of Phase 1a was. The pull request's review is the first.

## After the run

Run on 2026-10-13, inline, one commit for each task. Every step showed what it says above, `Cargo.lock` did not change, and the files the tasks changed are byte for byte the draft's.

## After the pull request's review

One commit beyond the tasks above, from the review of the pull request. The diffs in the tasks do not have it. Each of the four was shown by a test that failed first, but for the last, which is about names.

- **The times of last use were lost after a file that could not be read.** The start left them unread with the connections, so the next connect wrote `last_used.json` without them. They are read whatever becomes of `connections.toml`: they are another file's, and whole.
- **A damaged `connections.json` was abandoned.** The start had an empty `connections.toml` written for it, after which the older file was never read again, mended or not. Nothing is written for an older file that cannot be read now, and it is said in the notice as the new file's problem is. That is the notice "What this run leaves out" said it would not give.
- **The notice named a file that was not always the one.** A file is kept as `.bad.1` when `.bad` is taken, and is not moved at all where its link cannot be followed. `util::keep_aside` now says what it did, and the notice says that: the name the file has, or that it is still in its place and a save would replace it.
- **Two tests still saved `connections.json`.** They save `connections.toml` now, and the one that saves through a symbolic link saves `last_used.json` too.
