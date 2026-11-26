# Files connection 3b: attached connections in the sidebar, the list and for agents

> **For agentic workers:** REQUIRED SUB-SKILL: Use rubypowers:subagent-driven-development (recommended) or rubypowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** What run 3a left of the prompt's "Join with saved connections": an attached connection's schema shows its environment's badge in the sidebar, the connections list's row says what a Files connection joins, and agents reach an attached connection through the Files connection, only where that connection's own Agents setting lets them read it whole.

**Architecture:**
- **Agents** (`crates/tabletist-core/src/backend/agents.rs`, `attach.rs`; `src/app.rs`):
  - `Agentable` gains `attach: Vec<AttachTo>`: what a Files connection's agents' session attaches.
  - `reaching(attach, connections, saved)` says, for each attachment, what that session attaches for it, or why it leaves it out (`LeftOut::Off`, `LeftOut::Guards`). It reaches one only where the saved connection is among those agents may use (its access Read after the administrator's floor) and its sources, the person's and the administrator's, hide no table and mask no column. One the Join section can't attach at all (deleted, retyped, asked for on every connect, a MySQL one with no database) isn't among them.
  - `reach_attachments` fills `attach` for each Files connection, and `App::send_agents_policy` calls it. The policy's building moves to `App::agents_connections`, which the dialog reads too.
  - The gateway opens a Files connection with `Connection::connect_files` and the attachments' secrets, read from the keyring quietly (`attach::attaching_quietly`): never a prompt from an agent's call. Its rows and timeout are the Files connection's.
  - `AttachSaved` gains the saved connection's `environment`, for the sidebar.
- **The sidebar** (`src/ui/sidebar.rs`, `src/ui/menu.rs`):
  - The schema header draws an attached schema's environment badge after its name.
  - The schema menu lists the connection's own schemas, then a rule and the attached ones, each with its badge on the right. `menu::Choice` gains `badge` and `rule_before`, and `menu::Item` gains `badge()`.
  - A screen reader hears the environment as the header's and the row's description.
- **The connections list** (`src/ui/picker.rs`): `FileRowText::right` holds `Joins`. On macOS the right column says "Joins Bookshop · dev" (the saved name and environment) with "+1", "+2" for the others, or "No joins". The Omarchy line ends with `· joins bookshop_development +1`, by the name SQL calls it. A deleted connection isn't counted.
- **The Files form's Agents part** (`src/ui/connect_dialog/agents.rs`): with Read, a line under the note for each attachment agents don't reach, and why. The form keeps `agents_left_out`, read again each frame from `App::agents_connections`, as `joinable` is.

**Tech Stack:** Rust, egui (`egui_kittest` headless tests), DuckDB (`duckdb` 1.20000.1, the SQLite reader for the agent test), gettext catalogs (`assets/i18n`).

**Base:** `main` at `1307a2fe`, branch `claude/files-connection-3b-attached`. Run 3a (#387) is merged.

---

## What was decided

On 2026-11-26 Igor picked these, with the recommended option each time:

| Question | Decision |
| --- | --- |
| Where an attached schema's environment badge goes | **The header and the menu**: after the schema's name in the sidebar's header while it is shown; in the schema menu, after a rule under the connection's own schemas, each with its badge on the right. |
| The list row's join line with two or more | **The first, then a count**: "Joins Bookshop · dev +1"; Omarchy `joins bookshop_development +1`. |
| Whether the Files form says which attachments agents don't reach | **Name each one left out**, and why, under the Agents part's note. Nothing when agents reach all of them. |

Run 3's own decision stands: agents reach an attachment only where its own setting allows, Read with no hidden tables or masked columns.

Before this run, an agent's session on a Files connection attached nothing: `Connection::connect_with` opens a Files source alone, so a query naming an attached schema failed.

Canvas (canvas version 219): **"macOS – Files, attached connections"** and **"Omarchy – Files, attached connections"** are new. Each draws:
- an attached schema shown in the sidebar, beside the files' own;
- the schema menu;
- the list rows with one join, two, and none;
- the Agents part naming two attachments left out, and with none left out;
- on macOS, what the agent sees.

### Where this plan does more, or less, than the prompt or the canvas

| The prompt or canvas | What is built | Why |
| --- | --- | --- |
| "attached connections show as schemas with their environment badge" | The header and the schema menu, as decided. An attached connection of no environment has no badge. The files' own schemas have none: the connection's own environment is in its title bar as before. | Decided above. Drawn on the new boards. |
| macOS "Joins Bookshop · dev"; Omarchy `joins bookshop_dev` | macOS names the saved connection and its environment, as the list does. Omarchy names it as SQL does, by default its database's name (`bookshop_development`, 3a's departure). "+1", "+2" for the others. A deleted one isn't counted; with nothing else, the row says "No joins". | The boards draw one join each. Decided above. |
| "Agents can query the files and attached connections, read-only" | Only the attachments whose own Agents setting is Read with no hidden tables and no masked columns, the administrator's rules included. The others aren't attached to the agents' session at all, and the Files form's Agents part names each with why. | Run 3's decision. A hidden table or a masked column can't be held to a statement DuckDB runs against an attached database. |
| — | An attachment the person's own open can't attach (its saved connection deleted, of a type that can't be attached, its password asked for on every connect, a MySQL one with no database) is left out of the agents' session without a line in the Agents part: the Join section already says why. | The person's own open fails on it and says so. |
| — | The agents' session reads the attachments' passwords quietly, as for any connection an agent opens: a locked keyring makes the call answer that the connection is unavailable, and why goes to the app's log. | A prompt to unlock the keyring brought up by an agent's call could fish for the person's password. |
| — | The agents' session takes the Files connection's rows per answer and statement timeout, for its attached schemas too. | It is one session, one connection to the agent. |
| — | The Omarchy list line's "joins" goes through the catalogs; the rest of that line (`files · … · 4 files`) is English in every language, as since run 1. | The new words are translated; the old line is left as it is. |

The new words have no AI, sparkle or confidence wording, and the UI copy has no em dashes. They're translated into Russian and Brazilian Portuguese; the Portuguese wants a native review.

## Files

| File | Change |
| --- | --- |
| `crates/tabletist-core/src/backend/attach.rs` | `AttachSaved::environment`; `attaching_quietly` |
| `crates/tabletist-core/src/backend/agents.rs`, `backend.rs` | `Agentable::attach`, `LeftOut`, `reaching`, `reach_attachments`; a Files connection's agents' session attaches them |
| `src/app.rs` | `App::agents_connections`; the policy reaches attachments |
| `src/ui/menu.rs`, `src/ui/sidebar.rs`, `src/ui/pending_bar.rs`, `src/ui/sql_editor.rs`, `tests/environments.rs` | A menu row's badge and rule; the schema header's badge; the two allowed to ask for an environment's colours |
| `src/ui/picker.rs` | `Joins`, the row's join line in both looks |
| `crates/tabletist-core/src/model.rs`, `src/ui/mod.rs`, `src/ui/connect_dialog/agents.rs` | The form's `agents_left_out` and its lines |
| `src/ui/attached_tests.rs` (new), `src/ui/join_tests.rs`, `crates/tabletist-core/src/backend/export.rs` | Headless tests in both looks; the two test literals take `environment` |
| `assets/i18n/*.po`, `*.pot` | The new words |
| `docs/_reference/connections.md`, `docs/_guide/agents.md` | The badges, the joins, agents through attachments |

Every commit passes these, the full check:
- `~/.cargo/bin/cargo fmt --check`
- `~/.cargo/bin/cargo clippy --workspace --all-targets -- -D warnings`
- `~/.cargo/bin/cargo test --workspace --no-fail-fast`, the headless tests (with the server URLs below)
- `RUSTDOCFLAGS='-D warnings' ~/.cargo/bin/cargo doc --workspace --no-deps`
- `sh assets/i18n/update.sh --check`

Run cargo as `~/.cargo/bin/cargo` with `CARGO_TARGET_DIR=$PWD/target`. For the server tests, point `TABLETIST_TEST_PG_URL` and `TABLETIST_TEST_MYSQL_URL` at the test servers when they're up (`ss -ltn` shows 55432 and 53306). The agent test attaches a SQLite file, so DuckDB's SQLite reader is downloaded on the first run if it isn't yet. Commits are signed: one for this plan, then one per task.

---

### Task 1: Agents reach what each attachment lets them read

**Files:**
- Create: `src/ui/attached_tests.rs`
- Modify: `crates/tabletist-core/src/backend/attach.rs`, `agents.rs`, `backend.rs`, `export.rs` (a test literal), `src/app.rs`, `src/ui/mod.rs` (the test module), `src/ui/join_tests.rs` (a test literal)

- [ ] **Step 1: Write the failing tests**

The agents' tests attach the Bookshop four ways to a Files connection: as it is, Off, with a table hidden, and with a column an administrator's rule masks. Agents reach the first alone; two more can't be attached at all and aren't said. Then a real agents' session on the Bookshop's exports, attaching a SQLite Bookshop agents may read and one they may not: it joins a file to the first, and the second isn't in it. The attach module's test reads a password quietly. The app's test checks the policy it gives agents.

````diff
diff --git a/crates/tabletist-core/src/backend/agents.rs b/crates/tabletist-core/src/backend/agents.rs
--- a/crates/tabletist-core/src/backend/agents.rs
+++ b/crates/tabletist-core/src/backend/agents.rs
@@ -2224,4 +2224,187 @@
         )
         .await;
     }
-}
+
+    /// A Files connection attaching the Bookshop four ways: as it is, Off,
+    /// with a table hidden, and with a column an administrator's rule
+    /// masks. And two that can't be attached: one whose password is asked
+    /// for on every connect, and one deleted.
+    fn attaching_four() -> Vec<SavedConnection> {
+        let postgres = |database: &str| ConnectSpec {
+            driver: Driver::Postgres,
+            host: "db.bookshop.test".into(),
+            user: "reader".into(),
+            database: database.into(),
+            ..ConnectSpec::default()
+        };
+        let mut shown = saved(
+            "dev",
+            "Bookshop",
+            Environment::Dev,
+            postgres("bookshop_development"),
+        );
+        shown.password = PasswordMode::Keyring;
+        let mut off = saved(
+            "prod",
+            "Bookshop",
+            Environment::Production,
+            postgres("bookshop_production"),
+        );
+        off.agents.access = AgentAccess::Off;
+        let mut hides = saved(
+            "staging",
+            "Bookshop",
+            Environment::Staging,
+            postgres("bookshop_staging"),
+        );
+        hides.agents.tables = Tables::AllExcept(vec!["customers".parse().unwrap()]);
+        let ruled = saved("ruled", "Reports", Environment::Dev, postgres("reports"));
+        let mut asks = saved("asks", "Shop", Environment::Dev, postgres("shop"));
+        asks.password = PasswordMode::Ask;
+        let mut spec = ConnectSpec::files(tabletist_db::files::Source::Folder(
+            "/data/bookshop-exports".into(),
+        ));
+        let attach = |connection: &str, name: &str| Attach {
+            connection: connection.into(),
+            name: name.into(),
+        };
+        spec.attach = vec![
+            attach("dev", "bookshop_development"),
+            attach("prod", "bookshop_production"),
+            attach("staging", "bookshop_staging"),
+            attach("ruled", "reports"),
+            attach("asks", "shop"),
+            attach("gone", "gone"),
+        ];
+        let files = saved("files", "Bookshop exports", Environment::Local, spec);
+        vec![shown, off, hides, ruled, asks, files]
+    }
+
+    #[test]
+    fn agents_reach_an_attachment_only_where_its_own_setting_lets_them_read_it_whole() {
+        let saved = attaching_four();
+        let mut connections = agentable(&saved);
+        for agentable in &mut connections {
+            if agentable.id.0 == "ruled" {
+                agentable.sources.administrator = vec![tabletist_db::agent::Source {
+                    tables: Tables::All,
+                    masked: vec!["orders.email".parse().unwrap()],
+                }];
+            }
+        }
+        let files = connections
+            .iter()
+            .find(|each| each.id.0 == "files")
+            .unwrap();
+        let reached: Vec<(String, Result<String, LeftOut>)> =
+            reaching(&files.spec.attach, &connections, &saved)
+                .into_iter()
+                .map(|(name, reached)| {
+                    let id = reached.map(|to| to.saved.unwrap().id.0);
+                    (name, id)
+                })
+                .collect();
+        let pair = |name: &str, reached| (name.to_owned(), reached);
+        // The one asked for on every connect and the deleted one can't be
+        // attached at all: the Join section says so, not the Agents part.
+        assert_eq!(
+            reached,
+            [
+                pair("bookshop_development", Ok("dev".to_owned())),
+                pair("bookshop_production", Err(LeftOut::Off)),
+                pair("bookshop_staging", Err(LeftOut::Guards)),
+                pair("reports", Err(LeftOut::Guards)),
+            ]
+        );
+        reach_attachments(&mut connections, &saved);
+        let attached: Vec<(&str, Vec<&str>)> = connections
+            .iter()
+            .map(|each| {
+                let names = each.attach.iter().map(|to| to.name.as_str()).collect();
+                (each.id.0.as_str(), names)
+            })
+            .collect();
+        assert_eq!(
+            attached,
+            [
+                ("dev", vec![]),
+                ("staging", vec![]),
+                ("ruled", vec![]),
+                ("asks", vec![]),
+                ("files", vec!["bookshop_development"]),
+            ]
+        );
+        // What it attaches is the saved connection as agents get it, with
+        // where its password is kept, never the password.
+        let files = connections
+            .iter()
+            .find(|each| each.id.0 == "files")
+            .unwrap();
+        let dev = files.attach[0].saved.as_ref().unwrap();
+        assert_eq!(dev.password, PasswordMode::Keyring);
+        assert_eq!(dev.environment, Environment::Dev);
+        assert_eq!(dev.spec.database, "bookshop_development");
+    }
+
+    #[tokio::test]
+    async fn an_agent_s_files_session_attaches_only_what_agents_may_read() {
+        let dir = tempfile::tempdir().unwrap();
+        let exports = dir.path().join("exports");
+        std::fs::create_dir(&exports).unwrap();
+        tabletist_db::fixtures::write_bookshop_files(&exports).unwrap();
+        let shop = saved(
+            "shop",
+            "Bookshop",
+            Environment::Dev,
+            ConnectSpec::sqlite(bookshop(dir.path(), "shop.sqlite3")),
+        );
+        let mut off = saved(
+            "archive",
+            "Archive",
+            Environment::Production,
+            ConnectSpec::sqlite(bookshop(dir.path(), "archive.sqlite3")),
+        );
+        off.agents.access = AgentAccess::Off;
+        let mut spec = ConnectSpec::files(tabletist_db::files::Source::Folder(exports));
+        spec.attach = vec![
+            Attach {
+                connection: "shop".into(),
+                name: "shop".into(),
+            },
+            Attach {
+                connection: "archive".into(),
+                name: "archive".into(),
+            },
+        ];
+        let files = saved("files", "Bookshop exports", Environment::Local, spec);
+        let all = [shop, off, files];
+        let mut connections = agentable(&all);
+        reach_attachments(&mut connections, &all);
+        let gateway = AppGateway::new(Keyring::memory());
+        gateway.set_policy(connections, HostKeys::default());
+        let stop = CancellationToken::new();
+        let session = gateway.open("files", &stop).await.unwrap();
+        assert_eq!(session.connection().unwrap().access(), Access::ReadOnly);
+        let schemas = session.schemas().await.unwrap();
+        assert_eq!(schemas.names, ["main", "bookshop", "shop"]);
+        // A file's table and the attached database's, in one read.
+        let joined = read(
+            &session,
+            Dialect::DuckDb,
+            "SELECT count(*) FROM bookshop.order_items i JOIN shop.books b ON b.id = i.book_id",
+        )
+        .await;
+        assert!(the_int(&joined.unwrap()) > 0);
+        // The one agents may not read isn't in the session at all.
+        let refused = read(
+            &session,
+            Dialect::DuckDb,
+            "SELECT count(*) FROM archive.books",
+        )
+        .await;
+        assert!(
+            matches!(&refused, Err(Failure::Database(error)) if error.to_string().contains("archive")),
+            "{refused:?}"
+        );
+    }
+}
diff --git a/crates/tabletist-core/src/backend/attach.rs b/crates/tabletist-core/src/backend/attach.rs
--- a/crates/tabletist-core/src/backend/attach.rs
+++ b/crates/tabletist-core/src/backend/attach.rs
@@ -120,6 +120,7 @@
             name: name.into(),
             saved: Some(AttachSaved {
                 id: ConnectionId(format!("id-{name}")),
+                environment: Environment::Dev,
                 spec,
                 password,
                 ssh_secret: PasswordMode::None,
@@ -162,6 +163,27 @@
         assert_eq!(each[0].name, "shop");
         assert_eq!(each[0].secrets.password.as_deref(), Some("s3cret"));
         assert_eq!(each[1].secrets, Secrets::default());
+    }
+
+    /// An agent's session reads the same password, without a prompt.
+    #[tokio::test]
+    async fn an_agent_s_read_is_quiet_and_reads_the_same() {
+        let keyring = Keyring::memory();
+        keyring
+            .write(
+                &password_account(&ConnectionId("id-shop".into())),
+                crate::secrets::SecretString("s3cret".into()),
+            )
+            .wait()
+            .await
+            .unwrap();
+        let each = attaching_quietly(
+            vec![to("shop", postgres(), PasswordMode::Keyring)],
+            &keyring,
+        )
+        .await
+        .unwrap();
+        assert_eq!(each[0].secrets.password.as_deref(), Some("s3cret"));
     }
 
     /// A file needs no SSH secret, whatever tunnel its spec still names.
diff --git a/crates/tabletist-core/src/backend/export.rs b/crates/tabletist-core/src/backend/export.rs
--- a/crates/tabletist-core/src/backend/export.rs
+++ b/crates/tabletist-core/src/backend/export.rs
@@ -869,6 +869,7 @@
                 name: "shop".into(),
                 saved: Some(super::super::attach::AttachSaved {
                     id: crate::connections::ConnectionId("bookshop".into()),
+                    environment: crate::env::Environment::Local,
                     spec: ConnectSpec::sqlite(dir.path().join("bookshop.db")),
                     password: crate::connections::PasswordMode::None,
                     ssh_secret: crate::connections::PasswordMode::None,
diff --git a/src/ui/attached_tests.rs b/src/ui/attached_tests.rs
new file mode 100644
--- /dev/null
+++ b/src/ui/attached_tests.rs
@@ -0,0 +1,88 @@
+//! A Files connection's attached connections outside its dialog, in both
+//! looks (the "Files, attached connections" boards): each one's schema
+//! with its environment's badge in the sidebar, and what agents reach of
+//! them, as the Agents part says and as the policy agents get holds it.
+
+use tabletist_db::ConnectSpec;
+use tabletist_db::agent::Tables;
+use tabletist_db::files::{Attach, Source};
+
+use crate::backend::{AgentsCommand, Command};
+use crate::connections::{AgentAccess, ConnectionId, PasswordMode, SavedConnection};
+use crate::env::Environment;
+use crate::testing::Harness;
+
+fn saved(name: &str, environment: Environment, spec: ConnectSpec) -> SavedConnection {
+    SavedConnection {
+        id: ConnectionId::new(),
+        name: name.into(),
+        group: None,
+        environment,
+        read_only: None,
+        password: PasswordMode::None,
+        ssh_secret: PasswordMode::None,
+        spec,
+        agents: crate::connections::Agents {
+            access: AgentAccess::Read,
+            ..Default::default()
+        },
+    }
+}
+
+/// The Bookshop's SQLite files saved three ways: development, which agents
+/// may read; production, which they may not; staging, whose customers
+/// they may not see. And a Files connection over its exports attaching
+/// all three.
+fn bookshop(harness: &mut Harness) -> ConnectionId {
+    let sqlite = |file: &str| ConnectSpec::sqlite(format!("/data/{file}.sqlite3"));
+    let dev = saved("Bookshop", Environment::Dev, sqlite("development"));
+    let mut production = saved("Bookshop", Environment::Production, sqlite("production"));
+    production.agents.access = AgentAccess::Off;
+    let mut staging = saved("Bookshop", Environment::Staging, sqlite("staging"));
+    staging.agents.tables = Tables::AllExcept(vec!["customers".parse().unwrap()]);
+    let attach = |saved: &SavedConnection, name: &str| Attach {
+        connection: saved.id.0.clone(),
+        name: name.into(),
+    };
+    let mut spec = ConnectSpec::files(Source::Folder("/data/bookshop-exports".into()));
+    spec.attach = vec![
+        attach(&dev, "bookshop_development"),
+        attach(&production, "bookshop_production"),
+        attach(&staging, "bookshop_staging"),
+    ];
+    let files = saved("Bookshop exports", Environment::Local, spec);
+    let id = files.id.clone();
+    for saved in [dev, production, staging, files] {
+        harness.app.connections.upsert(saved);
+    }
+    id
+}
+
+#[test]
+fn agents_get_a_files_connection_with_what_its_attachments_let_them_read() {
+    let mut harness = Harness::new();
+    let id = bookshop(&mut harness);
+    harness.app.serve_agents();
+    let files = harness
+        .app
+        .backend
+        .sent
+        .iter()
+        .rev()
+        .find_map(|command| match command {
+            Command::Agents(AgentsCommand::Policy { connections, .. }) => {
+                connections.iter().find(|agentable| agentable.id == id)
+            }
+            _ => None,
+        })
+        .expect("the Files connection is agents'");
+    let attached: Vec<(&str, Option<Environment>)> = files
+        .attach
+        .iter()
+        .map(|to| {
+            let environment = to.saved.as_ref().map(|saved| saved.environment);
+            (to.name.as_str(), environment)
+        })
+        .collect();
+    assert_eq!(attached, [("bookshop_development", Some(Environment::Dev))]);
+}
diff --git a/src/ui/join_tests.rs b/src/ui/join_tests.rs
--- a/src/ui/join_tests.rs
+++ b/src/ui/join_tests.rs
@@ -378,6 +378,7 @@
                 name: "bookshop_development".into(),
                 saved: Some(AttachSaved {
                     id: dev.clone(),
+                    environment: Environment::Dev,
                     spec: saved.spec.clone(),
                     password: PasswordMode::Keyring,
                     ssh_secret: PasswordMode::None,
diff --git a/src/ui/mod.rs b/src/ui/mod.rs
--- a/src/ui/mod.rs
+++ b/src/ui/mod.rs
@@ -18,6 +18,8 @@
 #[cfg(test)]
 mod appearance_settings_tests;
 pub mod attach_list;
+#[cfg(test)]
+mod attached_tests;
 pub mod banner;
 pub mod calendar;
 #[cfg(test)]
````

- [ ] **Step 2: Run them to see them fail**

Run: `~/.cargo/bin/cargo test -q -p tabletist-core --lib -- agents::tests::agents_reach agents::tests::an_agent_s_files attach::tests`

Expected: they don't compile. `reaching`, `reach_attachments`, `LeftOut`, `attaching_quietly` and `AttachSaved::environment` aren't there yet.

```text
error[E0422]: cannot find struct, variant or union type `Attach` in this scope
error[E0425]: cannot find type `LeftOut` in this scope
error[E0433]: cannot find type `Environment` in this scope
    |
error[E0433]: cannot find type `Driver` in this scope
error[E0425]: cannot find function `reaching` in this scope
error[E0433]: cannot find type `LeftOut` in this scope
error[E0425]: cannot find function `reach_attachments` in this scope
error[E0609]: no field `attach` on type `&Agentable`
error[E0560]: struct `attach::AttachSaved` has no field named `environment`
error[E0425]: cannot find function `attaching_quietly` in this scope
error: could not compile `tabletist-core` (lib test) due to 17 previous errors
```

- [ ] **Step 3: Reach the attachments, and attach them to the agents' session**

````diff
diff --git a/crates/tabletist-core/src/backend.rs b/crates/tabletist-core/src/backend.rs
--- a/crates/tabletist-core/src/backend.rs
+++ b/crates/tabletist-core/src/backend.rs
@@ -25,7 +25,8 @@
 mod updates;
 
 pub use agents::{
-    Agentable, AgentsCommand, AgentsEvent, AppGateway, LOG_PAGE, agent_names, agentable,
+    Agentable, AgentsCommand, AgentsEvent, AppGateway, LOG_PAGE, LeftOut, agent_names, agentable,
+    reach_attachments, reaching,
 };
 pub use history::{HistoryCommand, HistoryEvent};
 pub use updates::{UpdatesCommand, UpdatesEvent};
diff --git a/crates/tabletist-core/src/backend/agents.rs b/crates/tabletist-core/src/backend/agents.rs
--- a/crates/tabletist-core/src/backend/agents.rs
+++ b/crates/tabletist-core/src/backend/agents.rs
@@ -13,14 +13,16 @@
 use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
 
 use tabletist_db::agent::{Answer, Hidden, Limits, Read, Sources};
+use tabletist_db::files::Attach;
 use tabletist_db::{
-    Access, CancelHandle, ConnectSpec, Connection, Error, HostKeys, ObjectInfo, ObjectRef, Secrets,
-    StopFlag, Structure,
+    Access, CancelHandle, ConnectSpec, Connection, Driver, Error, HostKeys, ObjectInfo, ObjectRef,
+    Secrets, StopFlag, Structure,
 };
 use tabletist_mcp::{Failure, Gateway, Listed, Log, Schemas, Session};
 use tokio::sync::OwnedMutexGuard;
 use tokio_util::sync::CancellationToken;
 
+use super::attach::{AttachSaved, AttachTo};
 use super::{CANCEL_AGAIN, Outbox, lock, longer, send_cancel};
 use crate::connections::{AgentAccess, ConnectionId, PasswordMode, SavedConnection};
 use crate::env::Platform;
@@ -49,6 +51,22 @@
     /// get masked: the person's, and each matching rule's of the
     /// administrator's policy.
     pub sources: Sources,
+    /// A Files connection's: the saved connections its agents' session
+    /// attaches, those whose own setting lets agents read them whole (see
+    /// [`reaching`]). Empty for any other connection.
+    pub attach: Vec<AttachTo>,
+}
+
+/// Why agents don't reach a saved connection a Files connection
+/// attaches: its agents' session is opened without it.
+#[derive(Debug, Clone, Copy, PartialEq, Eq)]
+pub enum LeftOut {
+    /// Its own agent access is Off, or the administrator's policy holds
+    /// it Off.
+    Off,
+    /// It hides tables or masks columns, by the person's settings or a
+    /// rule's: an attached database is read with no such check.
+    Guards,
 }
 
 /// The connections of `saved` that agents may use, in its order. One that
@@ -80,8 +98,67 @@
                 timeout: Duration::from_secs(u64::from(saved.agents.timeout_secs)),
             },
             sources: Sources::person(saved.agents.tables.clone(), saved.agents.masked.clone()),
+            attach: Vec::new(),
         })
         .collect()
+}
+
+/// How agents reach each of a Files connection's `attach`: the name SQL
+/// calls it by, and what its agents' session attaches for it, or why that
+/// leaves it out. `connections` are those agents may use ([`agentable`],
+/// with the administrator's sources); `saved` is every saved connection.
+///
+/// One that can't be attached at all is not among them: deleted, of a
+/// type that can't be attached, or one the Join section says can't be
+/// ticked. The person's own open fails on it, and says why.
+pub fn reaching(
+    attach: &[Attach],
+    connections: &[Agentable],
+    saved: &[SavedConnection],
+) -> Vec<(String, Result<AttachTo, LeftOut>)> {
+    let joinable = crate::connections::join::joinable(saved);
+    attach
+        .iter()
+        .filter_map(|each| {
+            let id = ConnectionId(each.connection.clone());
+            let row = joinable
+                .iter()
+                .find(|row| row.id == id && row.blocked.is_none())?;
+            let reached = match connections.iter().find(|agentable| agentable.id == id) {
+                None => Err(LeftOut::Off),
+                Some(agentable) if agentable.sources.guards_anything() => Err(LeftOut::Guards),
+                Some(agentable) => Ok(AttachTo {
+                    name: each.name.clone(),
+                    saved: Some(AttachSaved {
+                        id,
+                        environment: row.environment,
+                        spec: agentable.spec.clone(),
+                        password: agentable.password,
+                        ssh_secret: agentable.ssh_secret,
+                    }),
+                }),
+            };
+            Some((each.name.clone(), reached))
+        })
+        .collect()
+}
+
+/// Gives each Files connection among `connections` what its agents'
+/// session attaches, by [`reaching`].
+pub fn reach_attachments(connections: &mut [Agentable], saved: &[SavedConnection]) {
+    let reached: Vec<Vec<AttachTo>> = connections
+        .iter()
+        .map(|files| match files.spec.driver {
+            Driver::Files => reaching(&files.spec.attach, connections, saved)
+                .into_iter()
+                .filter_map(|(_, reached)| reached.ok())
+                .collect(),
+            Driver::Sqlite | Driver::Postgres | Driver::MySql | Driver::BigQuery => Vec::new(),
+        })
+        .collect();
+    for (files, attach) in connections.iter_mut().zip(reached) {
+        files.attach = attach;
+    }
 }
 
 /// What an agent calls each of `saved`, in its order, Off or not, by the
@@ -930,6 +1007,19 @@
     agentable: &Agentable,
     host_keys: &HostKeys,
 ) -> Result<Connection, Failure> {
+    // A Files connection keeps no secret of its own: those of the saved
+    // connections its agents' session attaches, quietly as the rest.
+    match agentable.spec.driver {
+        Driver::Files => {
+            let attaching = super::attach::attaching_quietly(agentable.attach.clone(), keyring)
+                .await
+                .map_err(|error| unavailable(&error.to_string()))?;
+            return Connection::connect_files(&agentable.spec, &attaching, host_keys, None)
+                .await
+                .map_err(|error| unavailable(&error.to_string()));
+        }
+        Driver::Sqlite | Driver::Postgres | Driver::MySql | Driver::BigQuery => {}
+    }
     let mut secrets = Secrets::default();
     let needed = [
         (
diff --git a/crates/tabletist-core/src/backend/attach.rs b/crates/tabletist-core/src/backend/attach.rs
--- a/crates/tabletist-core/src/backend/attach.rs
+++ b/crates/tabletist-core/src/backend/attach.rs
@@ -7,6 +7,7 @@
 use tabletist_db::{ConnectSpec, Error, Secrets};
 
 use crate::connections::{ConnectionId, PasswordMode, SavedConnections};
+use crate::env::Environment;
 use crate::model::SecretKind;
 use crate::secrets::{Keyring, password_account, ssh_account};
 
@@ -18,11 +19,13 @@
     pub saved: Option<AttachSaved>,
 }
 
-/// What a saved connection to attach is: how it is reached, and where
-/// its secrets are kept.
+/// What a saved connection to attach is: how it is reached, where its
+/// secrets are kept, and its environment, whose badge the sidebar shows on
+/// its schema.
 #[derive(Debug, Clone, PartialEq)]
 pub struct AttachSaved {
     pub id: ConnectionId,
+    pub environment: Environment,
     pub spec: ConnectSpec,
     pub password: PasswordMode,
     pub ssh_secret: PasswordMode,
@@ -39,6 +42,7 @@
                     .get(&ConnectionId(each.connection.clone()))
                     .map(|saved| AttachSaved {
                         id: saved.id.clone(),
+                        environment: saved.environment,
                         spec: saved.spec.clone(),
                         password: saved.password,
                         ssh_secret: saved.ssh_secret,
@@ -53,6 +57,24 @@
 pub(super) async fn attaching(
     attach: Vec<AttachTo>,
     keyring: &Keyring,
+) -> Result<Vec<Attaching>, Error> {
+    read(attach, keyring, false).await
+}
+
+/// [`attaching`] for an agent's session: the keyring is read quietly,
+/// never with a prompt to unlock it, which an agent's call could fish for
+/// the person's password with.
+pub(super) async fn attaching_quietly(
+    attach: Vec<AttachTo>,
+    keyring: &Keyring,
+) -> Result<Vec<Attaching>, Error> {
+    read(attach, keyring, true).await
+}
+
+async fn read(
+    attach: Vec<AttachTo>,
+    keyring: &Keyring,
+    quietly: bool,
 ) -> Result<Vec<Attaching>, Error> {
     let mut each = Vec::with_capacity(attach.len());
     for to in attach {
@@ -95,7 +117,7 @@
                 PasswordMode::Ask => {
                     return Err(cannot("its password is asked for on every connect"));
                 }
-                PasswordMode::Keyring => match keyring.read(&account).wait().await {
+                PasswordMode::Keyring => match read_one(keyring, &account, quietly).await {
                     Ok(Some(secret)) => *kind.slot(&mut secrets) = Some(secret),
                     Ok(None) => return Err(cannot("the keyring has no password for it")),
                     Err(error) => return Err(cannot(&error.to_string())),
@@ -109,6 +131,17 @@
         });
     }
     Ok(each)
+}
+
+async fn read_one(
+    keyring: &Keyring,
+    account: &str,
+    quietly: bool,
+) -> Result<Option<String>, crate::secrets::SecretError> {
+    match quietly {
+        true => keyring.read_quietly(account).wait().await,
+        false => keyring.read(account).wait().await,
+    }
 }
 
 #[cfg(test)]
diff --git a/src/app.rs b/src/app.rs
--- a/src/app.rs
+++ b/src/app.rs
@@ -6238,8 +6238,23 @@
         if !self.serves_agents {
             return;
         }
-        // What a change made outside the app holds back is not sent, and
-        // nothing above the administrator's policy.
+        let (shown, mut connections) = self.agents_connections();
+        // A Files connection's agents' session attaches what each of its
+        // attachments' own settings let agents read.
+        crate::backend::reach_attachments(&mut connections, &shown);
+        self.backend.send(Command::Agents(AgentsCommand::Policy {
+            connections,
+            host_keys: self.host_keys.clone(),
+        }));
+    }
+
+    /// The saved connections as agents get them now, and those agents may
+    /// use, with the administrator's sources: what a change made outside
+    /// the app holds back is not given, and nothing above the
+    /// administrator's policy.
+    pub(crate) fn agents_connections(
+        &self,
+    ) -> (Vec<SavedConnection>, Vec<crate::backend::Agentable>) {
         let home = self.dirs.home.as_deref();
         let shown: Vec<SavedConnection> = self
             .connections
@@ -6260,10 +6275,7 @@
         for agentable in &mut connections {
             agentable.sources.administrator = self.managed.sources(&agentable.spec, home);
         }
-        self.backend.send(Command::Agents(AgentsCommand::Policy {
-            connections,
-            host_keys: self.host_keys.clone(),
-        }));
+        (shown, connections)
     }
 
     fn start_agents(&mut self) {
````

- [ ] **Step 4: Run them to see them pass**

Run: `~/.cargo/bin/cargo test -q -p tabletist-core --lib -- agents::tests::agents_reach agents::tests::an_agent_s_files attach::tests`, then `~/.cargo/bin/cargo test -q -p tabletist --lib -- attached_tests`

Expected: 6 pass, then 1 pass.

- [ ] **Step 5: Run the full check**

Run the five commands of the full check above.

Expected: all pass; 5,727 passed, 0 failed.

- [ ] **Step 6: Commit**

```bash
git add crates/tabletist-core/src/backend crates/tabletist-core/src/backend.rs src/app.rs src/ui/mod.rs src/ui/attached_tests.rs src/ui/join_tests.rs
git commit -m "Let agents reach a Files connection's attachments where each one's own setting lets them read it whole"
```

### Task 2: The sidebar's badges

**Files:**
- Modify: `src/ui/menu.rs`, `src/ui/sidebar.rs`, `src/ui/pending_bar.rs` and `src/ui/sql_editor.rs` (their `menu::Choice` literals), `src/ui/attached_tests.rs`, `tests/environments.rs` (the two new places that ask for an environment's colours)

- [ ] **Step 1: Write the failing test**

In both looks: a Files connection attaching three of the Bookshop's SQLite files is opened through the recording backend, its tree answered. With `main` shown no badge is drawn; with `bookshop_development` shown its badge follows its name. The schema menu lists the attached ones after `main`, each with its badge.

````diff
diff --git a/src/ui/attached_tests.rs b/src/ui/attached_tests.rs
--- a/src/ui/attached_tests.rs
+++ b/src/ui/attached_tests.rs
@@ -3,14 +3,20 @@
 //! with its environment's badge in the sidebar, and what agents reach of
 //! them, as the Agents part says and as the policy agents get holds it.
 
-use tabletist_db::ConnectSpec;
 use tabletist_db::agent::Tables;
 use tabletist_db::files::{Attach, Source};
+use tabletist_db::{ConnectSpec, Driver, ObjectInfo, ObjectKind};
 
-use crate::backend::{AgentsCommand, Command};
+use crate::backend::{AgentsCommand, Command, Event};
 use crate::connections::{AgentAccess, ConnectionId, PasswordMode, SavedConnection};
-use crate::env::Environment;
-use crate::testing::Harness;
+use crate::env::{Environment, Platform};
+use crate::model::Action;
+use crate::testing::{Harness, answer_tree, last_sent};
+use crate::theme::Look;
+
+fn looks() -> [Look; 2] {
+    [Look::macos(), Look::omarchy()]
+}
 
 fn saved(name: &str, environment: Environment, spec: ConnectSpec) -> SavedConnection {
     SavedConnection {
@@ -58,6 +64,103 @@
     id
 }
 
+/// Opens the Files connection `id` in the active tab through the recording
+/// backend, its tree answered: `main` and the three attached schemas.
+fn opened(harness: &mut Harness, id: &ConnectionId) -> crate::model::ConnTabId {
+    let tab = harness.app.active_tab_id();
+    harness.app.apply(Action::Connect {
+        tab,
+        conn: id.clone(),
+    });
+    let Command::Connect {
+        session, request, ..
+    } = *last_sent(&harness.app)
+    else {
+        panic!("expected Connect");
+    };
+    harness.app.apply(Action::Backend(Event::Connected {
+        session,
+        request,
+        driver: Driver::Files,
+        encrypted: false,
+        access: crate::testing::asked_access(&harness.app),
+    }));
+    let schemas = [
+        "main",
+        "bookshop_development",
+        "bookshop_production",
+        "bookshop_staging",
+    ];
+    let orders = ObjectInfo {
+        name: "orders".into(),
+        kind: ObjectKind::Table,
+        estimated_rows: Some(40),
+    };
+    answer_tree(
+        &mut harness.app,
+        schemas.map(str::to_owned).to_vec(),
+        vec![orders],
+    );
+    harness.settle();
+    tab
+}
+
+/// Where the last frame painted `text`, in any case: macOS's section
+/// labels are capitals.
+fn painted(harness: &Harness, text: &str) -> Option<egui::Rect> {
+    harness
+        .text_rects
+        .iter()
+        .find(|(painted, _)| painted.eq_ignore_ascii_case(text))
+        .map(|(_, rect)| *rect)
+}
+
+#[test]
+fn an_attached_schema_shows_its_environment_s_badge() {
+    for look in looks() {
+        let name = look.name;
+        let mut harness = Harness::with_size(egui::vec2(1200.0, 800.0));
+        harness.set_look(look);
+        let id = bookshop(&mut harness);
+        let tab = opened(&mut harness, &id);
+        let label = |environment: Environment| environment.label(Platform::of(&look));
+        // The files' own schema has none.
+        assert!(
+            harness.painted_rect(label(Environment::Dev)).is_none(),
+            "{name}"
+        );
+        harness.app.apply(Action::ShowSchema {
+            tab,
+            schema: "bookshop_development".into(),
+        });
+        harness.settle();
+        let schema = painted(&harness, "bookshop_development")
+            .unwrap_or_else(|| panic!("{name}: the schema is not drawn"));
+        let badge = harness
+            .painted_rect(label(Environment::Dev))
+            .unwrap_or_else(|| panic!("{name}: no badge"));
+        assert!(badge.left() > schema.right(), "{name}");
+        assert!((badge.center().y - schema.center().y).abs() < 2.0, "{name}");
+        // The menu lists the attached schemas after the files' own, each
+        // with its badge.
+        harness.click("Schema");
+        harness.settle();
+        let menu = |text: &str| {
+            harness
+                .painted_rect(text)
+                .unwrap_or_else(|| panic!("{name}: {text} is not in the menu"))
+        };
+        let production = menu("bookshop_production");
+        let badge = menu(label(Environment::Production));
+        assert!(badge.left() > production.right(), "{name}");
+        assert!(menu("main").top() < production.top(), "{name}");
+        assert!(
+            harness.painted_rect(label(Environment::Staging)).is_some(),
+            "{name}"
+        );
+    }
+}
+
 #[test]
 fn agents_get_a_files_connection_with_what_its_attachments_let_them_read() {
     let mut harness = Harness::new();
````

- [ ] **Step 2: Run it to see it fail**

Run: `~/.cargo/bin/cargo test -q -p tabletist --lib -- attached_tests`

Expected: the new test fails: no badge is drawn.

```text
failures:
---- ui::attached_tests::an_attached_schema_shows_its_environment_s_badge stdout ----
thread 'ui::attached_tests::an_attached_schema_shows_its_environment_s_badge' (1577236) panicked at src/ui/attached_tests.rs:141:32:
    ui::attached_tests::an_attached_schema_shows_its_environment_s_badge
test result: FAILED. 1 passed; 1 failed; 0 ignored; 0 measured; 3162 filtered out; finished in 0.02s
error: test failed, to rerun pass `-p tabletist --lib`
```

- [ ] **Step 3: Draw the badge in the header and the menu**

````diff
diff --git a/src/ui/menu.rs b/src/ui/menu.rs
--- a/src/ui/menu.rs
+++ b/src/ui/menu.rs
@@ -111,6 +111,7 @@
     disabled: Option<&'a str>,
     note: Option<&'a str>,
     lit: Option<bool>,
+    badge: Option<crate::env::Environment>,
 }
 
 impl<'a> Item<'a> {
@@ -127,6 +128,7 @@
             disabled: None,
             note: None,
             lit: None,
+            badge: None,
         }
     }
 
@@ -189,6 +191,13 @@
     /// the pointer is over.
     pub fn kept_out(mut self, lit: bool) -> Self {
         self.lit = Some(lit);
+        self
+    }
+
+    /// An environment's badge at the row's right, in the environment's
+    /// colours: the schema of an attached connection.
+    pub fn badge(mut self, environment: crate::env::Environment) -> Self {
+        self.badge = Some(environment);
         self
     }
 
@@ -239,7 +248,14 @@
         let after = self
             .note
             .map_or(0.0, |words| shape.gap + measure(note, words));
-        let wanted = lead + own + after + noted + hinted + shape.inset;
+        let badge_style = match look.terminal {
+            true => super::workspace::Badge::Plain,
+            false => super::workspace::Badge::Mac,
+        };
+        let badged = self.badge.map_or(0.0, |environment| {
+            DETAIL_GAP + super::workspace::env_badge_width(ui, environment, badge_style, look)
+        });
+        let wanted = lead + own + after + noted + hinted + badged + shape.inset;
         // A row that cannot be picked answers no press, and still takes
         // the keyboard, as a button that cannot be pressed does: the
         // keyboard can then read why. In a menu the keyboard never comes
@@ -262,6 +278,11 @@
         if let Some(detail) = self.detail {
             ui.ctx().accesskit_node_builder(response.id, |node| {
                 node.set_description(detail);
+            });
+        }
+        if let Some(environment) = self.badge {
+            ui.ctx().accesskit_node_builder(response.id, |node| {
+                node.set_description(environment.label(crate::env::Platform::Native));
             });
         }
         if let Some((words, key)) = self.hint {
@@ -337,6 +358,13 @@
                 .layout(ui.ctx())
                 .paint_right(painter, rect.right() - shape.inset, center);
         }
+        if let Some(environment) = self.badge {
+            let platform = crate::env::Platform::of(look);
+            let colors = crate::env::env_colors(environment, platform, palette);
+            let width = super::workspace::env_badge_width(ui, environment, badge_style, look);
+            let left = rect.right() - shape.inset - width;
+            super::workspace::env_badge(ui, left, center, environment, &colors, badge_style, look);
+        }
         let hint_shown = self.hint_shown.unwrap_or(lit);
         if let Some((words, key)) = self.hint.filter(|_| hint_shown) {
             let key = key.as_str();
@@ -475,6 +503,10 @@
     /// Why the choice cannot be picked: it is shown all the same, so the
     /// menu says what there is to choose (see [`Item::disabled`]).
     pub disabled: Option<String>,
+    /// An environment's badge at its right (see [`Item::badge`]).
+    pub badge: Option<crate::env::Environment>,
+    /// A rule above it: the choices after it are of another kind.
+    pub rule_before: bool,
 }
 
 /// The menu that `button` opens when clicked: `choices` under it, at least
@@ -518,6 +550,9 @@
     let mut picked = None;
     let open = under(button, min_width, look, palette, |ui| {
         for (index, choice) in choices().into_iter().enumerate() {
+            if choice.rule_before {
+                separator(ui, palette);
+            }
             let item = if marked {
                 Item::choice(&choice.text, choice.selected)
             } else {
@@ -529,6 +564,10 @@
             };
             let item = match &choice.disabled {
                 Some(reason) => item.disabled(reason),
+                None => item,
+            };
+            let item = match choice.badge {
+                Some(environment) => item.badge(environment),
                 None => item,
             };
             if item.show(ui, look, palette).clicked() {
diff --git a/src/ui/pending_bar.rs b/src/ui/pending_bar.rs
--- a/src/ui/pending_bar.rs
+++ b/src/ui/pending_bar.rs
@@ -558,6 +558,8 @@
                     name: None,
                     selected: false,
                     disabled: disabled.map(str::to_owned),
+                    badge: None,
+                    rule_before: false,
                 };
                 let mut rows = Vec::new();
                 if pending {
diff --git a/src/ui/sidebar.rs b/src/ui/sidebar.rs
--- a/src/ui/sidebar.rs
+++ b/src/ui/sidebar.rs
@@ -213,6 +213,18 @@
     let schemas = workspace
         .tree
         .visible_schemas(workspace.driver, show_system);
+    // The schemas a Files tab's session attached, with each one's
+    // environment: those of no environment have no badge.
+    let attached: Vec<(String, crate::env::Environment)> = workspace
+        .attached
+        .iter()
+        .filter_map(|to| {
+            let environment = to.saved.as_ref()?.environment;
+            environment
+                .named(crate::env::Platform::Native)
+                .map(|_| (to.name.clone(), environment))
+        })
+        .collect();
     let shown = workspace.tree.shown_schema(workspace.driver, show_system);
     let flat = workspace.tree.flat;
     let shown_loading = shown
@@ -343,6 +355,7 @@
                 ui,
                 tab,
                 &schemas,
+                &attached,
                 shown.as_deref(),
                 flat,
                 shown_loading,
@@ -860,6 +873,7 @@
     ui: &mut egui::Ui,
     tab: ConnTabId,
     schemas: &[String],
+    attached: &[(String, crate::env::Environment)],
     shown: Option<&str>,
     flat: bool,
     loading: bool,
@@ -886,15 +900,30 @@
     let rect = Rect::from_min_size(pos2(band.left(), band.top() + above), vec2(width, row));
     let left = rect.left() + left_pad;
     let center = rect.center().y;
+    // An attached connection's schema shows its environment's badge.
+    let badge_of = |schema: &str| {
+        attached
+            .iter()
+            .find(|(name, _)| name == schema)
+            .map(|(_, environment)| *environment)
+    };
+    let badge_style = match look.terminal {
+        true => crate::ui::workspace::Badge::Plain,
+        false => crate::ui::workspace::Badge::Mac,
+    };
     // The schema: a menu of the others.
     if let Some(schema) = shown {
         let label = widgets::section_label(&display_safe(schema), look, palette).layout(ui.ctx());
-        let text_width = label.width();
         let (glyph_width, gap) = if look.terminal {
             (8.0, 4.0)
         } else {
             (10.0, 4.0)
         };
+        let badge = badge_of(schema);
+        let badge_width = badge.map_or(0.0, |environment| {
+            6.0 + crate::ui::workspace::env_badge_width(ui, environment, badge_style, look)
+        });
+        let text_width = label.width() + badge_width;
         let hit = Rect::from_min_size(
             pos2(left - 6.0, center - 13.0),
             vec2(6.0 + text_width + gap + glyph_width + 6.0, 26.0),
@@ -906,11 +935,23 @@
             info.current_text_value = Some(display_safe(schema).into_owned());
             info
         });
+        if let Some(environment) = badge {
+            ui.ctx().accesskit_node_builder(response.id, |node| {
+                node.set_description(environment.label(crate::env::Platform::Native));
+            });
+        }
         if response.hovered() && !look.terminal {
             ui.painter()
                 .rect_filled(hit, CornerRadius::same(6), palette.surface);
         }
+        let label_width = label.width();
         label.paint_left(ui.painter(), left, center);
+        if let Some(environment) = badge {
+            let platform = crate::env::Platform::of(look);
+            let colors = crate::env::env_colors(environment, platform, palette);
+            let x = left + label_width + 6.0;
+            crate::ui::workspace::env_badge(ui, x, center, environment, &colors, badge_style, look);
+        }
         let glyph = pos2(left + text_width + gap + glyph_width / 2.0, center);
         if look.terminal {
             let mark = Text::one(look, TextRole::OCaption, "▾", palette.dim).layout(ui.ctx());
@@ -920,14 +961,23 @@
                 .image(palette.secondary, 10.0)
                 .paint_at(ui, Rect::from_center_size(glyph, vec2(10.0, 10.0)));
         }
+        // The connection's own schemas, then after a rule the attached
+        // ones, each with its badge.
         let picked = menu::choices(&response, 160.0, look, palette, || {
             schemas
                 .iter()
-                .map(|other| menu::Choice {
-                    text: display_safe(other).into_owned(),
-                    name: None,
-                    selected: Some(other.as_str()) == shown,
-                    disabled: None,
+                .enumerate()
+                .map(|(index, other)| {
+                    let badge = badge_of(other);
+                    let after_own = index > 0 && badge_of(&schemas[index - 1]).is_none();
+                    menu::Choice {
+                        text: display_safe(other).into_owned(),
+                        name: None,
+                        selected: Some(other.as_str()) == shown,
+                        disabled: None,
+                        badge,
+                        rule_before: badge.is_some() && after_own,
+                    }
                 })
                 .collect()
         });
diff --git a/src/ui/sql_editor.rs b/src/ui/sql_editor.rs
--- a/src/ui/sql_editor.rs
+++ b/src/ui/sql_editor.rs
@@ -485,6 +485,8 @@
                     name: Some(limit_name(*choice, locale)),
                     selected: *choice == bar.limit,
                     disabled: None,
+                    badge: None,
+                    rule_before: false,
                 })
                 .collect()
         },
@@ -513,6 +515,8 @@
                     name: Some(timeout_name(*choice, locale)),
                     selected: *choice == bar.secs,
                     disabled: None,
+                    badge: None,
+                    rule_before: false,
                 })
                 .collect()
         },
diff --git a/tests/environments.rs b/tests/environments.rs
--- a/tests/environments.rs
+++ b/tests/environments.rs
@@ -199,6 +199,10 @@
         // A Files connection's Join section: each saved connection's
         // stripe and badge.
         "src/ui/attach_list.rs",
+        // An attached connection's schema: its badge in the sidebar's
+        // header and in the schema menu's row.
+        "src/ui/sidebar.rs",
+        "src/ui/menu.rs",
     ];
     let mut found = Vec::new();
     for (path, text) in sources() {
````

- [ ] **Step 4: Run it to see it pass**

Run: `~/.cargo/bin/cargo test -q -p tabletist --lib -- attached_tests`

Expected: 2 pass.

- [ ] **Step 5: Run the full check**

Run the five commands of the full check above.

Expected: all pass; 5,728 passed, 0 failed.

- [ ] **Step 6: Commit**

```bash
git add src/ui/menu.rs src/ui/sidebar.rs src/ui/pending_bar.rs src/ui/sql_editor.rs src/ui/attached_tests.rs tests/environments.rs
git commit -m "Show an attached connection's environment badge on its schema in the sidebar"
```

### Task 3: The list row's joins and the Agents part's lines

**Files:**
- Modify: `src/ui/picker.rs`, `crates/tabletist-core/src/model.rs`, `src/ui/mod.rs`, `src/ui/connect_dialog/agents.rs`, `src/ui/attached_tests.rs`, `assets/i18n/*.po`, `assets/i18n/tabletist.pot`, `docs/_reference/connections.md`, `docs/_guide/agents.md`

- [ ] **Step 1: Write the failing tests**

The list row's test: the first connection still saved, by name and environment on macOS and by SQL name in the Omarchy line, "+1" for another, a deleted one not counted, and one of no environment named alone. The Agents part's test, in both looks: editing the Files connection, its Agents part names the production attachment (Off) and the staging one (a table hidden), not development; with agents Off here, nothing.

````diff
diff --git a/src/ui/attached_tests.rs b/src/ui/attached_tests.rs
--- a/src/ui/attached_tests.rs
+++ b/src/ui/attached_tests.rs
@@ -10,7 +10,7 @@
 use crate::backend::{AgentsCommand, Command, Event};
 use crate::connections::{AgentAccess, ConnectionId, PasswordMode, SavedConnection};
 use crate::env::{Environment, Platform};
-use crate::model::Action;
+use crate::model::{Action, Dialog};
 use crate::testing::{Harness, answer_tree, last_sent};
 use crate::theme::Look;
 
@@ -162,6 +162,48 @@
 }
 
 #[test]
+fn the_agents_part_names_the_attachments_agents_don_t_reach() {
+    for look in looks() {
+        let name = look.name;
+        let mut harness = Harness::with_size(egui::vec2(1200.0, 1600.0));
+        harness.set_look(look);
+        let id = bookshop(&mut harness);
+        harness.app.apply(Action::EditConnection(id));
+        if let Some(Dialog::Connection(form)) = &mut harness.app.dialog {
+            form.agents_open = true;
+        }
+        harness.settle();
+        let says = |harness: &mut Harness, text: &str| {
+            let tree = harness.settle();
+            let text = text.to_lowercase();
+            crate::testing::labels(&tree)
+                .iter()
+                .any(|label| label.to_lowercase().contains(&text))
+        };
+        assert!(
+            says(
+                &mut harness,
+                "Agents don't reach bookshop_production: its own agent access is Off."
+            ),
+            "{name}"
+        );
+        assert!(
+            says(
+                &mut harness,
+                "Agents don't reach bookshop_staging: it hides tables or masks columns."
+            ),
+            "{name}"
+        );
+        assert!(!says(&mut harness, "reach bookshop_development"), "{name}");
+        // With agents Off here, nothing of them is said.
+        if let Some(Dialog::Connection(form)) = &mut harness.app.dialog {
+            form.agents.access = AgentAccess::Off;
+        }
+        assert!(!says(&mut harness, "Agents don't reach"), "{name}");
+    }
+}
+
+#[test]
 fn agents_get_a_files_connection_with_what_its_attachments_let_them_read() {
     let mut harness = Harness::new();
     let id = bookshop(&mut harness);
diff --git a/src/ui/picker.rs b/src/ui/picker.rs
--- a/src/ui/picker.rs
+++ b/src/ui/picker.rs
@@ -1779,7 +1779,8 @@
                 "returns.csv",
             ],
         );
-        let text = file_row_text(&folder, Some(&facts)).unwrap();
+        let none = SavedConnections::default();
+        let text = file_row_text(&folder, Some(&facts), &none).unwrap();
         assert_eq!(text.second, "Files · 4 files · 2.5 GB");
         assert_eq!(
             text.middle,
@@ -1790,11 +1791,12 @@
         );
         assert_eq!(
             text.right
-                .map(|(writes, joins)| (writes.english(), joins.english())),
-            Some(("Read-only", "No joins"))
+                .map(|(writes, joins)| (writes.english(), joins.say(crate::i18n::Locale::English))),
+            Some(("Read-only", "No joins".to_owned()))
         );
         assert_eq!(
-            terminal_files_line(&folder, Some(&facts)).as_deref(),
+            terminal_files_line(&folder, Some(&facts), &none, crate::i18n::Locale::English)
+                .as_deref(),
             Some("files · /data/bookshop-exports · 4 files · 2.5 gb")
         );
         // The path keeps its case in the terminal look's line.
@@ -1802,11 +1804,11 @@
             "/srv/Data/Bookshop-Exports".into(),
         )));
         assert_eq!(
-            terminal_files_line(&cased, None).as_deref(),
+            terminal_files_line(&cased, None, &none, crate::i18n::Locale::English).as_deref(),
             Some("files · /srv/Data/Bookshop-Exports")
         );
         // Before the files are read: the type alone.
-        assert_eq!(file_row_text(&folder, None).unwrap().second, "Files");
+        assert_eq!(file_row_text(&folder, None, &none).unwrap().second, "Files");
         // A list: its first files, then their folder.
         let list = saved(tabletist_db::ConnectSpec::files(Source::Files(vec![
             "/data/bookshop-exports/orders.csv".into(),
@@ -1816,7 +1818,7 @@
             "/data/bookshop-exports",
             &["orders.csv", "customers.parquet"],
         );
-        let text = file_row_text(&list, Some(&facts)).unwrap();
+        let text = file_row_text(&list, Some(&facts), &none).unwrap();
         assert_eq!(text.second, "Files · 2 files · 1.2 GB");
         assert_eq!(
             text.middle,
@@ -1832,9 +1834,11 @@
         let sqlite = saved(tabletist_db::ConnectSpec::sqlite(
             "/code/bookshop/storage/development.sqlite3",
         ));
+        let none = SavedConnections::default();
         let text = file_row_text(
             &sqlite,
             Some(&crate::model::SourceFacts::File { size: 12_400_000 }),
+            &none,
         )
         .unwrap();
         assert_eq!(text.second, "SQLite · 12.4 MB");
@@ -1847,9 +1851,68 @@
             )
         );
         assert_eq!(text.right, None);
-        assert_eq!(terminal_files_line(&sqlite, None), None);
+        assert_eq!(
+            terminal_files_line(&sqlite, None, &none, crate::i18n::Locale::English),
+            None
+        );
         // A server's row is its own.
         let (server, _) = tabletist_db::ConnectSpec::from_url("postgres://me@db/app").unwrap();
-        assert_eq!(file_row_text(&saved(server), None), None);
-    }
-}
+        assert_eq!(file_row_text(&saved(server), None, &none), None);
+    }
+
+    #[test]
+    fn a_files_row_says_the_first_connection_it_joins_and_how_many_more() {
+        use tabletist_db::files::{Attach, Source};
+        let mut store = SavedConnections::default();
+        let mut dev = saved(tabletist_db::ConnectSpec::sqlite("/data/bookshop.db"));
+        dev.name = "Bookshop".into();
+        dev.environment = crate::env::Environment::Dev;
+        let mut scratch = saved(tabletist_db::ConnectSpec::sqlite("/data/scratch.db"));
+        scratch.name = "Scratch".into();
+        scratch.environment = crate::env::Environment::None;
+        let attach = |saved: &SavedConnection, name: &str| Attach {
+            connection: saved.id.0.clone(),
+            name: name.into(),
+        };
+        let mut spec =
+            tabletist_db::ConnectSpec::files(Source::Folder("/data/bookshop-exports".into()));
+        // A deleted one isn't counted.
+        spec.attach = vec![
+            Attach {
+                connection: "deleted".into(),
+                name: "gone".into(),
+            },
+            attach(&dev, "bookshop_development"),
+        ];
+        store.upsert(dev.clone());
+        store.upsert(scratch.clone());
+        let files = saved(spec.clone());
+        let said = |files: &SavedConnection| {
+            let text = file_row_text(files, None, &store).unwrap();
+            let line =
+                terminal_files_line(files, None, &store, crate::i18n::Locale::English).unwrap();
+            (
+                text.right.unwrap().1.say(crate::i18n::Locale::English),
+                line,
+            )
+        };
+        assert_eq!(
+            said(&files),
+            (
+                "Joins Bookshop · dev".to_owned(),
+                "files · /data/bookshop-exports · joins bookshop_development".to_owned()
+            )
+        );
+        spec.attach.push(attach(&scratch, "scratch"));
+        assert_eq!(
+            said(&saved(spec.clone())),
+            (
+                "Joins Bookshop · dev +1".to_owned(),
+                "files · /data/bookshop-exports · joins bookshop_development +1".to_owned()
+            )
+        );
+        // One of no environment is named alone.
+        spec.attach = vec![attach(&scratch, "scratch")];
+        assert_eq!(said(&saved(spec)).0, "Joins Scratch");
+    }
+}
````

- [ ] **Step 2: Run them to see them fail**

Run: `~/.cargo/bin/cargo test -q -p tabletist --lib -- attached_tests picker::tests`

Expected: they don't compile: `file_row_text` and `terminal_files_line` take the saved connections and the language now.

```text
error[E0061]: this function takes 2 arguments but 3 arguments were supplied
error[E0308]: mismatched types
error[E0061]: this function takes 2 arguments but 4 arguments were supplied
error: could not compile `tabletist` (lib test) due to 13 previous errors
```

- [ ] **Step 3: Say the joins, and the attachments agents don't reach**

````diff
diff --git a/crates/tabletist-core/src/model.rs b/crates/tabletist-core/src/model.rs
--- a/crates/tabletist-core/src/model.rs
+++ b/crates/tabletist-core/src/model.rs
@@ -2080,6 +2080,10 @@
     /// Files: the saved connections the Join section lists, read again
     /// on each frame the dialog draws.
     pub joinable: Vec<crate::connections::join::Joinable>,
+    /// Files: the saved connections it attaches that agents don't reach,
+    /// by the name SQL calls each, and why. Read again on each frame the
+    /// dialog draws, as the policy the app sends agents says.
+    pub agents_left_out: Vec<(String, crate::backend::LeftOut)>,
     /// The environment the user chose; `None` until they do, while it
     /// follows where the connection points (see
     /// [`ConnectionForm::environment`]).
@@ -2436,6 +2440,7 @@
             focus_group: false,
             group_choices: Default::default(),
             joinable: Vec::new(),
+            agents_left_out: Vec::new(),
             environment: None,
             read_only: None,
             production_read_only: true,
@@ -2774,6 +2779,7 @@
             focus_group: false,
             group_choices: Default::default(),
             joinable: Vec::new(),
+            agents_left_out: Vec::new(),
             environment: Some(saved.environment),
             read_only: saved.read_only,
             production_read_only: true,
diff --git a/docs/_guide/agents.md b/docs/_guide/agents.md
--- a/docs/_guide/agents.md
+++ b/docs/_guide/agents.md
@@ -117,7 +117,10 @@
 
 Every connection starts with agents **Off**: they do not see it at all.
 A new Files connection is the exception: it starts with **Read**, or
-with **Off** on production. **Read** lets agents read a connection. Set
+with **Off** on production. Through a Files connection agents also read
+the saved connections it attaches, but only those whose own access is
+**Read** with no hidden tables and no masked columns. **Read** lets
+agents read a connection. Set
 it in any of these places:
 
 - In Connections: a row's "…" menu, or `a` on a row in the Omarchy
diff --git a/docs/_reference/connections.md b/docs/_reference/connections.md
--- a/docs/_reference/connections.md
+++ b/docs/_reference/connections.md
@@ -137,9 +137,13 @@
   database's (a SQLite file's name without its extension) and you can
   type over it: lowercase letters, digits and `_`, and not a schema the
   connection has already. In the sidebar each attached connection is a
-  schema, listing the tables of its default schema (PostgreSQL's
-  `public`, MySQL's database, SQLite's own); its other schemas are
-  reached in SQL, as `bookshop_development.billing.invoices`. An attached
+  schema with its environment's badge, listing the tables of its default
+  schema (PostgreSQL's `public`, MySQL's database, SQLite's own); its
+  other schemas are reached in SQL, as
+  `bookshop_development.billing.invoices`. The schema menu lists the
+  attached ones after the files' own. In the connections list the row
+  says what it joins: "Joins Bookshop · dev", with "+1" for each more
+  (`joins bookshop_development` in the Omarchy look). An attached
   connection is always read-only. Its password comes from the keyring as
   the Files connection opens, and its SSH tunnel, if it has one, is
   opened then too; a connection whose password is asked for on every
@@ -161,7 +165,11 @@
   another file on the computer, write a file or install anything is
   refused. Agents can query the files, never change them, and cannot
   hide tables or mask columns on a Files connection. Its **Agents**
-  section is closed, and says the access on one line.
+  section is closed, and says the access on one line. Agents reach an
+  attached connection only where its own **Agents** setting is **Read**
+  with no hidden tables and no masked columns (an administrator's rules
+  included): the others are left out of their session, and the section
+  names each one and why.
 - **Test** reads each file's columns and its first rows, never a whole
   file to the end. With **Column types** from all rows, DuckDB reads a
   CSV in full to find its types, which takes longer on a large file.
diff --git a/src/ui/connect_dialog/agents.rs b/src/ui/connect_dialog/agents.rs
--- a/src/ui/connect_dialog/agents.rs
+++ b/src/ui/connect_dialog/agents.rs
@@ -13,6 +13,7 @@
 use super::{Skin, input};
 use tabletist_db::Driver;
 
+use crate::backend::LeftOut;
 use crate::connections::AgentAccess;
 use crate::i18n::{Msgid, msgid};
 use crate::model::{Action, ConnectionForm, TableFocus, TablesMode};
@@ -88,6 +89,28 @@
         Driver::Postgres | Driver::MySql | Driver::Sqlite | Driver::BigQuery => true,
         Driver::Files => false,
     }
+}
+
+/// What a Files form's part says, under its note, of each saved
+/// connection it attaches that agents don't reach, while agents may read
+/// it: their session is opened without it.
+fn left_out(form: &ConnectionForm, skin: &Skin) -> Vec<String> {
+    match form.agents.access {
+        AgentAccess::Off => return Vec::new(),
+        AgentAccess::Read => {}
+    }
+    form.agents_left_out
+        .iter()
+        .map(|(name, why)| {
+            let words = match why {
+                LeftOut::Off => msgid("Agents don't reach {name}: its own agent access is Off."),
+                LeftOut::Guards => {
+                    msgid("Agents don't reach {name}: it hides tables or masks columns.")
+                }
+            };
+            crate::i18n::fill(&skin.say(words), &[("{name}", name)])
+        })
+        .collect()
 }
 
 /// The note under the part's rows.
@@ -1038,6 +1061,10 @@
     });
     ui.add_space(8.0);
     sentence(ui, small, &skin.say(note(form)), skin);
+    for line in left_out(form, skin) {
+        ui.add_space(4.0);
+        sentence(ui, small, &line, skin);
+    }
 }
 
 /// The terminal look: an "agents" section after Safety; on a Files form,
@@ -1121,6 +1148,9 @@
     }
     ui.add_space(4.0);
     sentence(ui, TextRole::OSecondary, &said, skin);
+    for line in left_out(form, skin) {
+        sentence(ui, TextRole::OSecondary, &line, skin);
+    }
 }
 
 /// The terminal look's Tables and Masked rows.
diff --git a/src/ui/mod.rs b/src/ui/mod.rs
--- a/src/ui/mod.rs
+++ b/src/ui/mod.rs
@@ -265,7 +265,29 @@
             .and_then(|conn| app.loaded_tables(conn)),
         _ => None,
     };
+    // What agents reach of a Files form's attachments, by the policy the
+    // app gives them: worked out for a Files form alone.
+    let files_form = match &app.dialog {
+        Some(Dialog::Connection(form)) => match form.driver {
+            tabletist_db::Driver::Files => true,
+            tabletist_db::Driver::Sqlite
+            | tabletist_db::Driver::Postgres
+            | tabletist_db::Driver::MySql
+            | tabletist_db::Driver::BigQuery => false,
+        },
+        _ => false,
+    };
+    let agents_now = files_form.then(|| app.agents_connections());
     if let Some(Dialog::Connection(form)) = &mut app.dialog {
+        form.agents_left_out = match &agents_now {
+            Some((shown, connections)) => {
+                crate::backend::reaching(&form.files_attach, connections, shown)
+                    .into_iter()
+                    .filter_map(|(name, reached)| reached.err().map(|why| (name, why)))
+                    .collect()
+            }
+            None => Vec::new(),
+        };
         let groups = crate::connections::groups::Choices::of(
             &form.name,
             form.stays_local(),
diff --git a/src/ui/picker.rs b/src/ui/picker.rs
--- a/src/ui/picker.rs
+++ b/src/ui/picker.rs
@@ -832,14 +832,79 @@
 pub(crate) struct FileRowText {
     pub second: String,
     pub middle: (String, String),
-    pub right: Option<(Msgid, Msgid)>,
+    pub right: Option<(Msgid, Joins)>,
+}
+
+/// What a Files connection joins, as its list row says it: the first of
+/// the saved connections it attaches that is still saved, and how many
+/// more there are.
+#[derive(Debug, Clone, PartialEq, Eq)]
+pub(crate) enum Joins {
+    None,
+    Some {
+        /// Its name and environment, as the list names it: "Bookshop · dev".
+        first: String,
+        /// The name SQL calls it by: the terminal look's line says that.
+        sql: String,
+        more: usize,
+    },
+}
+
+impl Joins {
+    /// What `spec` attaches, of the connections in `store`.
+    fn of(spec: &tabletist_db::ConnectSpec, store: &SavedConnections) -> Self {
+        let mut saved = spec.attach.iter().filter_map(|each| {
+            let connection = store.get(&ConnectionId(each.connection.clone()))?;
+            Some((each, connection))
+        });
+        let Some((each, connection)) = saved.next() else {
+            return Self::None;
+        };
+        let first = match connection.environment.named(crate::env::Platform::Native) {
+            Some(environment) => format!("{} · {environment}", connection.name),
+            None => connection.name.clone(),
+        };
+        Self::Some {
+            first,
+            sql: each.name.clone(),
+            more: saved.count(),
+        }
+    }
+
+    /// The macOS row's words: "Joins Bookshop · dev +1", or "No joins".
+    pub(crate) fn say(&self, locale: crate::i18n::Locale) -> String {
+        match self {
+            Self::None => msgid("No joins").say(locale).into_owned(),
+            Self::Some { first, more, .. } => joins(first, *more, locale),
+        }
+    }
+
+    /// The terminal look's words after its line: "joins bookshop_dev +1",
+    /// or none.
+    fn terminal(&self, locale: crate::i18n::Locale) -> Option<String> {
+        match self {
+            Self::None => None,
+            Self::Some { sql, more, .. } => Some(joins(sql, *more, locale)),
+        }
+    }
+}
+
+/// "Joins {name}", and "+1", "+2" for the others.
+fn joins(name: &str, more: usize, locale: crate::i18n::Locale) -> String {
+    let name = match more {
+        0 => name.to_owned(),
+        more => format!("{name} +{more}"),
+    };
+    fill(&msgid("Joins {name}").say(locale), &[("{name}", &name)])
 }
 
 /// [`FileRowText`] of `connection`, with what a scan of its files found
-/// (`facts`, which may be on its way). `None` for a server's.
+/// (`facts`, which may be on its way), and the saved connections in
+/// `store`, which name what it joins. `None` for a server's.
 pub(crate) fn file_row_text(
     connection: &SavedConnection,
     facts: Option<&crate::model::SourceFacts>,
+    store: &SavedConnections,
 ) -> Option<FileRowText> {
     use crate::model::SourceFacts;
     use tabletist_db::Driver;
@@ -896,32 +961,40 @@
                 second,
                 middle,
                 // Editing files is not built: every one reads only.
-                right: Some((msgid("Read-only"), msgid("No joins"))),
+                right: Some((msgid("Read-only"), Joins::of(spec, store))),
             })
         }
     }
 }
 
 /// The terminal look's line under a Files connection's name: `files ·
-/// ~/Data/bookshop-exports · 4 files · 2.5 gb`. `None` for any other.
+/// ~/Data/bookshop-exports · 4 files · 2.5 gb · joins bookshop_dev`.
+/// `None` for any other.
 pub(crate) fn terminal_files_line(
     connection: &SavedConnection,
     facts: Option<&crate::model::SourceFacts>,
+    store: &SavedConnections,
+    locale: crate::i18n::Locale,
 ) -> Option<String> {
     use tabletist_db::Driver;
     match connection.spec.driver {
         Driver::Files => {}
         Driver::Postgres | Driver::MySql | Driver::Sqlite | Driver::BigQuery => return None,
     }
-    let text = file_row_text(connection, facts)?;
+    let text = file_row_text(connection, facts, store)?;
     let source = match &connection.spec.files {
         Some(tabletist_db::files::Source::Folder(folder)) => tabletist_db::files::shown(folder),
         Some(source) => source.summary(),
         None => String::new(),
     };
     let counted = text.second.strip_prefix("Files").unwrap_or_default();
-    // The look's case for its words, and the path as it is.
-    Some(format!("files · {source}{}", counted.to_lowercase()))
+    let joins = text
+        .right
+        .and_then(|(_, joins)| joins.terminal(locale))
+        .map(|joins| format!(" · {}", joins.to_lowercase()))
+        .unwrap_or_default();
+    // The look's case for its words, and the path and names as they are.
+    Some(format!("files · {source}{}{joins}", counted.to_lowercase()))
 }
 
 /// The row's own response, answering a click by selecting and a double
@@ -1347,7 +1420,7 @@
         laid.paint_left(ui.painter(), tag.left() + 7.0, top);
         widgets::announce(ui, tag, &text);
     }
-    let files = file_row_text(connection, facts);
+    let files = file_row_text(connection, facts, store);
     let user = &connection.spec.user;
     let second = match &files {
         Some(files) => files.second.clone(),
@@ -1389,7 +1462,7 @@
     }
     // A Files connection says whether it writes and what it joins where a
     // server says its TLS and its tunnel.
-    if let Some((writes, joins)) = files.as_ref().and_then(|files| files.right) {
+    if let Some((writes, joins)) = files.as_ref().and_then(|files| files.right.clone()) {
         let writes = writes.say(locale);
         widgets::paint_label(
             ui,
@@ -1656,7 +1729,7 @@
     // "dev@localhost:5433/bookshop_development · tls verify-full · no ssh",
     // the separators kept with their neighbours as the design writes them.
     // A Files connection's line is its own: what it opens, and how much.
-    let said = match terminal_files_line(connection, facts) {
+    let said = match terminal_files_line(connection, facts, store, locale) {
         Some(files) => Text::new(look).add(small, &files, palette.dim),
         None => Text::new(look)
             .add(small, &format!("{line} ·"), palette.dim)
````

- [ ] **Step 4: Translate the new words**

Run `sh assets/i18n/update.sh`. It adds the three new messages to the template and both catalogs, untranslated. Fill them in:

| msgid | ru | pt-BR |
| --- | --- | --- |
| Joins {name} | Соединяется с {name} | Cruza com {name} |
| Agents don't reach {name}: its own agent access is Off. | Агенты не обращаются к {name}: у него самого доступ агентов выключен. | Os agentes não alcançam {name}: o acesso de agentes dela está desativado. |
| Agents don't reach {name}: it hides tables or masks columns. | Агенты не обращаются к {name}: в нём скрыты таблицы или замаскированы столбцы. | Os agentes não alcançam {name}: ela oculta tabelas ou mascara colunas. |

Run `sh assets/i18n/update.sh` again. Then `sh assets/i18n/update.sh --check` and `sh assets/i18n/update_test.sh` pass, with every entry translated.

- [ ] **Step 5: Run them to see them pass**

Run: `~/.cargo/bin/cargo test -q -p tabletist --lib -- attached_tests picker::tests`

Expected: 7 pass.

- [ ] **Step 6: Run the full check**

Run the five commands of the full check above.

Expected: all pass; 5,730 passed, 0 failed.

- [ ] **Step 7: Commit**

```bash
git add src/ui crates/tabletist-core/src/model.rs assets/i18n docs/_reference/connections.md docs/_guide/agents.md
git commit -m "Say what a Files connection joins in the list, and which attachments agents don't reach"
```

### After the tasks

- [ ] **Hand test**, in both looks, with the Bookshop's exports, the SQLite Bookshop and the test servers' `bookshop` database saved as connections, the server's password in the keyring, and agents on:
  1. A Files connection over the exports attaching the PostgreSQL one (dev, agents Read) and the SQLite file (production, agents Off), each under a name of its own. The list row says "Joins Bookshop · dev +1"; the Omarchy line ends with the PostgreSQL one's SQL name and "+1".
  2. Open it. The schema menu lists `main` and `bookshop` (the exports' `.duckdb` file), a rule, then the two attached, each with its badge. Shown, each has its badge after its name.
  3. Its Agents part says the SQLite one's SQL name: "its own agent access is Off".
  4. From an agent through Tabletist's MCP server: `describe_schema` on the Files connection lists the PostgreSQL one's schema and not the SQLite one's; a join of a file to the first runs; a query naming the second fails.
  5. Compare with "macOS – Files, attached connections" and "Omarchy – Files, attached connections" by eye.

  Any screenshot stays local, or is blurred.
- [ ] Push the branch and open a pull request against `main`. Its description lists:
  - a summary;
  - what agents reach through a Files connection, and how their session attaches it;
  - the new components (the menu row's badge and rule);
  - the decisions and the departures above;
  - the canvas boards;
  - that the Portuguese wants a native review.
