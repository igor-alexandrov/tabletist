# Connection groups, run 1: a group of the person's own: Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use rubypowers:subagent-driven-development (recommended) or rubypowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A saved connection can be put in a group the person names, with the **Group** field the connection form's boards draw (a dropdown of the groups the Connections list shows, and **New group…**). The list shows it under that group, `connections.toml` keeps it, and the list's search finds a connection by it.

**Architecture:** `SavedConnection` gains `group: Option<String>`, written as `group = "…"` after `name` in `connections.toml`, which is version 9 once a connection has one. The grouping rule moves from `src/ui/picker.rs` into the core, as `connections::groups`, because the form needs it too: a connection with a group is under it, one without falls under a name it shares with another such connection, else Local or Remote; titles in another case are one group. The form keeps the choice (`group`) and the name typed after New group… (`group_typed`); its view, `connect_dialog/group.rs`, draws one dropdown for both looks. A save keeps a group only where it is not the one the rule gives anyway (`groups::keep`), so a connection left in its own group follows the rule.

**Tech Stack:**
- Rust and egui 0.36 (the crmne fork).
- `toml` and `serde` for `connections.toml`.
- GNU gettext 1.0 (`assets/i18n/update.sh`).
- The app's `testing::Harness` (AccessKit tree and events).

**Spec:** `docs/superpowers/specs/2026-11-21-connection-groups-design.md`, "Runs", item 1, as brought in line with the decisions below in the commit that adds this plan. Issue #367. The design canvas: **macOS – Connection groups (#367)** and **Omarchy – Connection groups (#367)** on the Connections page, and the two edit-connection boards, which drew Group before this design.

**Base:** this branch, `claude/connection-groups-367-159587`, at bb4bdcc5 (the spec's commit on 55c6f276). `main` has since gained c0eb151c (#365, `docs/_reference/translating.md` only): rebase onto it before Task 1 (never a merge of main). No file of this plan changes there.

---

## Decisions for Igor (made while drafting)

The spec left these open, or the draft found them. Each is built as the first column says; say if one should change.

| What | This plan builds | Why |
| --- | --- | --- |
| **What Group offers** | The groups the list shows, and the one the rule gives this connection (so Remote only where a row, or this connection, is remote), then New group…. The spec's first draft said "named, shared-name, Local, Remote" | The canvas's dropdown: Bookshop, Local, New group…, with no Remote, while no remote connection is outside Bookshop. The two #367 boards were corrected the same way |
| **Search** | Finds the group the person gave, not Local, Remote or a shared name | A shared name is found as the name already. "re" would otherwise find every remote connection |
| **Shared names** | A name is shared with connections that have no group, or that name as their group (in any case; names compared without the spaces round them). Two Bookshops, one put with Clients: the other is alone with its name and goes under Local or Remote | Moving one out of a shared-name group must not leave a group of one behind it, and a Bookshop saved in "Bookshop" while its twin was elsewhere must not split from the twin once it comes back (the draft's review found that split) |
| **Two twins alone** | Two connections that share a name and nothing else: each one's Group offers the name and New group…, not Local or Remote. Typing `Local` after New group… moves one out | What the canvas lists (the groups shown and the connection's own). Offering Local or Remote too would add Remote to the canvas's Bookshop production example. Say if Local or Remote should always be offered |
| **A heading's spelling** | As the first connection in the file writes it: "experiments" and "Experiments" are one group, titled as the first | The spec's "titles compare trimmed and case-insensitively" |
| **Omarchy's Group** | The same dropdown as macOS, a box with `▾`, as the Omarchy edit boards draw it. Its menu says the form's words in lower case (`new group…`) and the groups as written | The canvas. The terminal form's other choices are rows of words, but the boards draw this one as a box |
| **New group…'s field** | It replaces the dropdown, with the keyboard, placeholder "Group name". Esc goes back to the dropdown (the dialog stays). Save saves what is typed; blank is no group | The canvas's "Esc goes back to the list · empty means no group" |
| **Local and Remote** | Stay English, as the list draws them today (they were never marked for translation). A group typed "Local" is the rule's Local | Out of this run. Worth its own small fix: the Russian and Portuguese windows show both in English today |
| **The website's pictures of the form** | Task 4 renders `macos-connection-edit.webp` and `omarchy-connection-edit.webp` again, which now show Group, and asks before they are committed | AGENTS.md: the pages follow user-visible behaviour. A picture goes to GitHub only on Igor's yes |

Not in this run (the spec's runs 2 and 3): the rows' Group section, Omarchy `m` and `R`, renaming a heading, and the Environment view.

## Files

| File | What changes | Task |
| --- | --- | --- |
| `crates/tabletist-core/src/connections.rs` | `SavedConnection::group`; search matches it; `pub mod groups` | 1, 2 |
| `crates/tabletist-core/src/connections/file.rs` | The `group` key, version 9 | 1 |
| `crates/tabletist-core/src/connections/groups.rs` | New: the rule (`Group`, `groups`, `own`, `automatic`, `of`, `LOCAL`, `REMOTE`), the form's (`choices`, `Choices`, `keep`, `kept`), `same`, `stays_on_this_machine` | 2 |
| `src/ui/picker.rs` | Takes `Group` and `groups` from the core; its `is_local` stays, for the TLS label, on the core's `stays_on_this_machine` | 2 |
| `crates/tabletist-core/src/model.rs` | `ConnectionForm::group`, `group_typed`, `focus_group`, `group_choices`, `chosen_group`, `stays_local` | 3 |
| `src/ui/connect_dialog/group.rs` | New: the Group control | 3 |
| `src/ui/connect_dialog/{mod,sheet,terminal,agents}.rs` | `show` takes the choices; Group beside Name (macOS), under it (terminal) | 3 |
| `src/ui/mod.rs`, `src/app.rs` | The choices each frame; a save keeps the group by the rule; `mod group_tests` | 3 |
| `src/ui/group_tests.rs` | New: the harness tests | 3 |
| `src/ui/env_tests.rs` | Its dialog test puts the form in a group, so Group does not read "Local" | 3 |
| `assets/i18n/*` | "Group", "New group…", "Group name" | 3 |
| `docs/_reference/connections.md`, `docs/_reference/settings-and-files.md`, `docs/_guide/omarchy.md` | The picker's groups, the Group field, the `group` key, the form's keys | 4 |
| Each full `SavedConnection { … }` literal, in the files above and ten more (fixtures, tests, the demo's and the promo's connections) | `group: None,` after `name` | 1 |

The checks, for every task (AGENTS.md):

```bash
export CARGO_TARGET_DIR=$PWD/target
cargo fmt --all --check
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo test --locked --workspace --all-targets
RUSTDOCFLAGS='-D warnings' cargo doc --locked --workspace --no-deps
```

`cargo` is `~/.cargo/bin/cargo` in these sessions.

---

### Task 1: A connection's group in `connections.toml`, and search by it

**Files:**
- Modify: `crates/tabletist-core/src/connections.rs` (the field, `search`, two tests)
- Modify: `crates/tabletist-core/src/connections/file.rs` (`Entry`, `connection`, `write`, `VERSION`, a test, the version test)
- Modify: every other file with a full `SavedConnection { … }` literal: `group: None,` after its `name`

- [ ] **Step 1: Write the failing tests.**

````diff
diff --git a/crates/tabletist-core/src/connections.rs b/crates/tabletist-core/src/connections.rs
--- a/crates/tabletist-core/src/connections.rs
+++ b/crates/tabletist-core/src/connections.rs
@@ -1104,6 +1104,37 @@
     }
 
     #[test]
+    fn search_matches_the_group_the_person_gave() {
+        let mut store = SavedConnections::default();
+        store.upsert(SavedConnection {
+            group: Some("Side projects".into()),
+            ..saved("Playground", "/srv/playground.db")
+        });
+        store.upsert(saved("Rails test", "/srv/test.db"));
+        let names = |text: &str| -> Vec<String> {
+            store.search(text).iter().map(|c| c.name.clone()).collect()
+        };
+        assert_eq!(names("side PROJ"), vec!["Playground"]);
+        // The rule's own groups are no words of a connection's.
+        assert!(names("Local").is_empty());
+    }
+
+    #[test]
+    fn a_duplicate_keeps_its_group() {
+        let mut store = SavedConnections::default();
+        let original = SavedConnection {
+            group: Some("Experiments".into()),
+            ..saved("Playground", "/p.db")
+        };
+        store.upsert(original.clone());
+        let copy = store.duplicate(&original.id).unwrap();
+        assert_eq!(
+            store.get(&copy).unwrap().group.as_deref(),
+            Some("Experiments")
+        );
+    }
+
+    #[test]
     fn password_modes_default_to_none_and_round_trip() {
         let old: SavedConnection = serde_json::from_str(
             r#"{"id": "x", "name": "Old", "environment": "none",
diff --git a/crates/tabletist-core/src/connections/file.rs b/crates/tabletist-core/src/connections/file.rs
--- a/crates/tabletist-core/src/connections/file.rs
+++ b/crates/tabletist-core/src/connections/file.rs
@@ -1251,8 +1251,8 @@
         let said = |text: &str| read(text).unwrap_err().to_string();
         assert!(said("").contains("missing field `version`"), "{}", said(""));
         assert_eq!(
-            said("version = 9\n"),
-            "it is version 9 of the file, written by a newer Tabletist; this one reads version 8"
+            said("version = 10\n"),
+            "it is version 10 of the file, written by a newer Tabletist; this one reads version 9"
         );
         assert_eq!(
             said("version = 2\n"),
@@ -1618,6 +1618,54 @@
         // Without one, the file is as it was.
         let others = write(&bookshop()).unwrap();
         assert!(!others.contains("version = 7"), "{others}");
+    }
+
+    /// The group the person put a connection in, after its name, makes the
+    /// file version 9, whatever else it holds.
+    #[test]
+    fn a_group_is_written_after_the_name_in_version_nine() {
+        let mut connections = bookshop();
+        connections[2].group = Some("Experiments".into());
+        let text = write(&connections).unwrap();
+        assert!(text.contains("\nversion = 9\n"), "{text}");
+        assert!(
+            text.ends_with(
+                r#"id          = "file"
+name        = "Bookshop on disk"
+group       = "Experiments"
+environment = "local"
+driver      = "sqlite"
+sqlite_path = "/home/ada/bookshop.db"
+"#
+            ),
+            "{text}"
+        );
+        assert_eq!(read(&text).unwrap().connections, connections);
+        // Without one, the file stays as old as the rest lets it; a blank
+        // one is none.
+        connections[2].group = None;
+        assert!(write(&connections).unwrap().contains("\nversion = 3\n"));
+        connections[2].group = Some("  ".into());
+        let blank = write(&connections).unwrap();
+        assert!(
+            blank.contains("\nversion = 3\n") && !blank.contains("group"),
+            "{blank}"
+        );
+        connections[2].group = None;
+        // A blank group, as a person may leave one, is none; the spaces
+        // round one are not kept.
+        let with = |line: &str| {
+            text.replace(
+                "group       = \"Experiments\"",
+                &format!("group       = {line}"),
+            )
+        };
+        assert_eq!(read(&with("\"  \"")).unwrap().connections, connections);
+        connections[2].group = Some("Side projects".into());
+        assert_eq!(
+            read(&with("\" Side projects \"")).unwrap().connections,
+            connections
+        );
     }
 
     /// How a Files connection reads CSV, as the file writes it: version 8,
````

- [ ] **Step 2: Run them, and see them fail.**

Run: `cargo test --locked -p tabletist-core --lib connections`

Expected: the crate's tests do not compile, as `SavedConnection` has no `group` yet:

```text
error[E0609]: no field `group` on type `connections::SavedConnection`
error[E0560]: struct `connections::SavedConnection` has no field named `group`
error[E0609]: no field `group` on type `&connections::SavedConnection`
error: could not compile `tabletist-core` (lib test) due to 8 previous errors
```

- [ ] **Step 3: Write the code.** The field, with its serde defaults (the older JSON has none); search; the file's key and version; and `group: None,` in each literal that lists every field. A literal with `..` takes it from what it spreads.

````diff
diff --git a/crates/tabletist-core/src/agents_shown.rs b/crates/tabletist-core/src/agents_shown.rs
--- a/crates/tabletist-core/src/agents_shown.rs
+++ b/crates/tabletist-core/src/agents_shown.rs
@@ -346,6 +346,7 @@
         SavedConnection {
             id: ConnectionId(id.into()),
             name: id.into(),
+            group: None,
             environment: crate::env::Environment::Dev,
             read_only: None,
             password: crate::connections::PasswordMode::None,
diff --git a/crates/tabletist-core/src/backend.rs b/crates/tabletist-core/src/backend.rs
--- a/crates/tabletist-core/src/backend.rs
+++ b/crates/tabletist-core/src/backend.rs
@@ -7145,6 +7145,7 @@
         connections.upsert(crate::connections::SavedConnection {
             id: crate::connections::ConnectionId::new(),
             name: name.into(),
+            group: None,
             environment: crate::env::Environment::None,
             read_only: None,
             password: crate::connections::PasswordMode::None,
diff --git a/crates/tabletist-core/src/backend/agents.rs b/crates/tabletist-core/src/backend/agents.rs
--- a/crates/tabletist-core/src/backend/agents.rs
+++ b/crates/tabletist-core/src/backend/agents.rs
@@ -1121,6 +1121,7 @@
         SavedConnection {
             id: ConnectionId(id.into()),
             name: name.into(),
+            group: None,
             environment,
             read_only: Some(false),
             password: PasswordMode::None,
diff --git a/crates/tabletist-core/src/connections.rs b/crates/tabletist-core/src/connections.rs
--- a/crates/tabletist-core/src/connections.rs
+++ b/crates/tabletist-core/src/connections.rs
@@ -104,6 +104,11 @@
 pub struct SavedConnection {
     pub id: ConnectionId,
     pub name: String,
+    /// The group the person put it in, as they wrote it; `None` until
+    /// they do, so the Connections list places it by its rule: under a
+    /// name it shares, else Local or Remote.
+    #[serde(default, skip_serializing_if = "Option::is_none")]
+    pub group: Option<String>,
     /// What the connection is. Its colour everywhere comes from this.
     pub environment: Environment,
     /// Read-only as the user set it; `None` until they do, so the
@@ -310,6 +315,7 @@
         let saved = SavedConnection {
             id: self.id,
             name: self.name,
+            group: None,
             environment,
             read_only: self.read_only,
             password: self.password,
@@ -591,7 +597,9 @@
         Some(new_id)
     }
 
-    /// Connections whose name or summary contains `text`, ignoring case.
+    /// Connections whose name, group or summary contains `text`, ignoring
+    /// case. The group is the one the person gave: Local and Remote are
+    /// no words to find a connection by.
     pub fn search(&self, text: &str) -> Vec<&SavedConnection> {
         let needle = text.trim().to_lowercase();
         self.connections
@@ -599,6 +607,10 @@
             .filter(|connection| {
                 needle.is_empty()
                     || connection.name.to_lowercase().contains(&needle)
+                    || connection
+                        .group
+                        .as_ref()
+                        .is_some_and(|group| group.to_lowercase().contains(&needle))
                     || connection.spec.summary().to_lowercase().contains(&needle)
             })
             .collect()
@@ -695,6 +707,7 @@
         SavedConnection {
             id: ConnectionId::new(),
             name: name.into(),
+            group: None,
             environment: Environment::None,
             read_only: None,
             password: PasswordMode::None,
diff --git a/crates/tabletist-core/src/connections/file.rs b/crates/tabletist-core/src/connections/file.rs
--- a/crates/tabletist-core/src/connections/file.rs
+++ b/crates/tabletist-core/src/connections/file.rs
@@ -30,8 +30,12 @@
 /// written as the oldest that holds what it has: a Tabletist that does not
 /// know a key refuses only a file it would lose it from, and one that
 /// would read agents' tables as all of them, or a masked column's values
-/// as they are, never reads the file.
-pub(super) const VERSION: u32 = 8;
+/// as they are, never reads the file. 9 has the group the person put a
+/// connection in (`group`).
+pub(super) const VERSION: u32 = 9;
+
+/// The version that has how Files connections read CSV and no group.
+const CSV_VERSION: u32 = 8;
 
 /// The version that has Files connections and not how they read CSV.
 const FILES_VERSION: u32 = 7;
@@ -101,6 +105,7 @@
 struct Entry {
     id: Option<String>,
     name: String,
+    group: Option<String>,
     #[serde(default)]
     environment: Environment,
     read_only: Option<bool>,
@@ -438,9 +443,15 @@
                 Reading::default()
             }
         };
+        // A blank group is none: the connection goes by the rule.
+        let group = self
+            .group
+            .map(|group| group.trim().to_owned())
+            .filter(|group| !group.is_empty());
         Ok(SavedConnection {
             id,
             name: self.name,
+            group,
             environment: self.environment,
             read_only: self.read_only,
             password: self.password,
@@ -651,6 +662,16 @@
     !options_are_default(reading) || written(reading).next().is_some()
 }
 
+/// The group the file holds for `connection`: none for a blank one, which
+/// the file could not read back as one.
+fn written_group(connection: &SavedConnection) -> Option<&str> {
+    connection
+        .group
+        .as_deref()
+        .map(str::trim)
+        .filter(|group| !group.is_empty())
+}
+
 fn password(mode: PasswordMode) -> &'static str {
     match mode {
         PasswordMode::None => "none",
@@ -695,13 +716,17 @@
     let csv = connections
         .iter()
         .any(|saved| saved.spec.files.is_some() && reads_csv_its_own_way(&saved.spec.csv));
-    let version = match (csv, files, masks, hides, agents) {
-        (true, _, _, _, _) => VERSION,
-        (false, true, _, _, _) => FILES_VERSION,
-        (false, false, true, _, _) => MASKED_VERSION,
-        (false, false, false, true, _) => TABLES_VERSION,
-        (false, false, false, false, true) => AGENTS_VERSION,
-        (false, false, false, false, false) => FIRST_VERSION,
+    let groups = connections
+        .iter()
+        .any(|saved| written_group(saved).is_some());
+    let version = match (groups, csv, files, masks, hides, agents) {
+        (true, _, _, _, _, _) => VERSION,
+        (false, true, _, _, _, _) => CSV_VERSION,
+        (false, false, true, _, _, _) => FILES_VERSION,
+        (false, false, false, true, _, _) => MASKED_VERSION,
+        (false, false, false, false, true, _) => TABLES_VERSION,
+        (false, false, false, false, false, true) => AGENTS_VERSION,
+        (false, false, false, false, false, false) => FIRST_VERSION,
     };
     let mut out = format!("{HEADER}version = {version}\n");
     for connection in connections {
@@ -713,6 +738,9 @@
         let mut lines = Lines::default();
         lines.text("id", &connection.id.0);
         lines.text("name", &connection.name);
+        if let Some(group) = written_group(connection) {
+            lines.text("group", group);
+        }
         lines.text("environment", connection.environment.key());
         if let Some(read_only) = connection.read_only {
             lines.bare("read_only", read_only);
@@ -854,6 +882,7 @@
         SavedConnection {
             id: ConnectionId(id.into()),
             name: name.into(),
+            group: None,
             environment: Environment::None,
             read_only: None,
             password: PasswordMode::None,
diff --git a/crates/tabletist-core/src/fixtures.rs b/crates/tabletist-core/src/fixtures.rs
--- a/crates/tabletist-core/src/fixtures.rs
+++ b/crates/tabletist-core/src/fixtures.rs
@@ -94,6 +94,7 @@
     SavedConnection {
         id: ConnectionId::new(),
         name: "Fixture".into(),
+        group: None,
         environment: crate::env::Environment::Dev,
         // Read-only, as every connection was when most tests were written.
         // A test of a writable connection says so itself.
diff --git a/crates/tabletist-core/src/model.rs b/crates/tabletist-core/src/model.rs
--- a/crates/tabletist-core/src/model.rs
+++ b/crates/tabletist-core/src/model.rs
@@ -2913,6 +2913,7 @@
         Ok(crate::connections::SavedConnection {
             id: self.editing.clone().unwrap_or_else(ConnectionId::new),
             name: name.to_owned(),
+            group: None,
             environment: self.environment(),
             read_only: self.saved_read_only(),
             password,
@@ -9205,6 +9206,7 @@
         let saved = SavedConnection {
             id: ConnectionId("exports".into()),
             name: "Bookshop exports".into(),
+            group: None,
             environment: crate::env::Environment::Local,
             read_only: Some(true),
             password: crate::connections::PasswordMode::None,
diff --git a/src/app.rs b/src/app.rs
--- a/src/app.rs
+++ b/src/app.rs
@@ -10953,6 +10953,7 @@
         let saved = SavedConnection {
             id: ConnectionId::new(),
             name: "Local".into(),
+            group: None,
             environment: crate::env::Environment::Dev,
             read_only: None,
             password: crate::connections::PasswordMode::None,
@@ -16632,6 +16633,7 @@
         let saved = SavedConnection {
             id: ConnectionId::new(),
             name: "Prod".into(),
+            group: None,
             environment: crate::env::Environment::Production,
             read_only: None,
             password: mode,
@@ -17947,6 +17949,7 @@
         let saved = SavedConnection {
             id: ConnectionId::new(),
             name: "Shop".into(),
+            group: None,
             environment: crate::env::Environment::None,
             read_only: None,
             password: PasswordMode::None,
@@ -18195,6 +18198,7 @@
         let saved = SavedConnection {
             id: ConnectionId::new(),
             name: "Shop".into(),
+            group: None,
             environment: crate::env::Environment::None,
             read_only: None,
             password: PasswordMode::None,
@@ -18305,6 +18309,7 @@
             let saved = SavedConnection {
                 id: ConnectionId::new(),
                 name: "Prod".into(),
+                group: None,
                 environment: crate::env::Environment::Production,
                 read_only: None,
                 password: PasswordMode::None,
diff --git a/src/entrypoint.rs b/src/entrypoint.rs
--- a/src/entrypoint.rs
+++ b/src/entrypoint.rs
@@ -249,6 +249,7 @@
     let saved = crate::connections::SavedConnection {
         id: crate::connections::ConnectionId::new(),
         name: "Demo".into(),
+        group: None,
         environment: crate::env::Environment::Dev,
         read_only: None,
         password: crate::connections::PasswordMode::None,
diff --git a/src/promo/runner.rs b/src/promo/runner.rs
--- a/src/promo/runner.rs
+++ b/src/promo/runner.rs
@@ -175,6 +175,7 @@
             harness.app.connections.upsert(SavedConnection {
                 id: ConnectionId::new(),
                 name: other.name.clone(),
+                group: None,
                 environment: other.environment,
                 read_only: Some(false),
                 password: PasswordMode::None,
@@ -186,6 +187,7 @@
         let saved = SavedConnection {
             id: ConnectionId::new(),
             name: name.into(),
+            group: None,
             environment: Environment::Dev,
             read_only: Some(false),
             password: PasswordMode::None,
diff --git a/src/shots.rs b/src/shots.rs
--- a/src/shots.rs
+++ b/src/shots.rs
@@ -32,6 +32,7 @@
     SavedConnection {
         id: ConnectionId::new(),
         name: "Bookshop".into(),
+        group: None,
         environment: Environment::Dev,
         read_only: None,
         password: PasswordMode::None,
@@ -69,6 +70,7 @@
     harness.app.connections.upsert(SavedConnection {
         id: id.clone(),
         name: "Bookshop".into(),
+        group: None,
         environment: Environment::Production,
         read_only: None,
         password: PasswordMode::Keyring,
@@ -2323,6 +2325,7 @@
             harness.app.connections.upsert(SavedConnection {
                 id: id.clone(),
                 name: name.into(),
+                group: None,
                 environment,
                 read_only: None,
                 password: PasswordMode::None,
diff --git a/src/ui/env_tests.rs b/src/ui/env_tests.rs
--- a/src/ui/env_tests.rs
+++ b/src/ui/env_tests.rs
@@ -131,6 +131,7 @@
                 .upsert(crate::connections::SavedConnection {
                     id: crate::connections::ConnectionId::new(),
                     name: format!("Bookshop {env:?}"),
+                    group: None,
                     environment: env,
                     read_only: None,
                     password: crate::connections::PasswordMode::None,
diff --git a/src/ui/files_tests.rs b/src/ui/files_tests.rs
--- a/src/ui/files_tests.rs
+++ b/src/ui/files_tests.rs
@@ -592,6 +592,7 @@
     SavedConnection {
         id: id.clone(),
         name: "Bookshop".into(),
+        group: None,
         environment: Environment::Dev,
         read_only: None,
         password: PasswordMode::None,
@@ -608,6 +609,7 @@
     let saved = SavedConnection {
         id: ConnectionId::new(),
         name: "Bookshop".into(),
+        group: None,
         environment: Environment::Dev,
         read_only: None,
         password: PasswordMode::None,
@@ -823,6 +825,7 @@
     let saved = SavedConnection {
         id: ConnectionId::new(),
         name: "Bookshop exports".into(),
+        group: None,
         environment: Environment::Local,
         read_only: Some(true),
         password: PasswordMode::None,
@@ -884,6 +887,7 @@
     let sqlite = SavedConnection {
         id: ConnectionId::new(),
         name: "Bookshop on disk".into(),
+        group: None,
         environment: Environment::Local,
         read_only: None,
         password: PasswordMode::None,
diff --git a/src/ui/mod.rs b/src/ui/mod.rs
--- a/src/ui/mod.rs
+++ b/src/ui/mod.rs
@@ -4054,6 +4054,7 @@
         let saved = crate::connections::SavedConnection {
             id: crate::connections::ConnectionId::new(),
             name: name.into(),
+            group: None,
             environment: crate::env::Environment::Production,
             read_only: None,
             password: crate::connections::PasswordMode::None,
@@ -4156,6 +4157,7 @@
                 .upsert(crate::connections::SavedConnection {
                     id: crate::connections::ConnectionId::new(),
                     name: "Bookshop".into(),
+                    group: None,
                     environment: crate::env::Environment::Dev,
                     read_only: None,
                     password: crate::connections::PasswordMode::None,
@@ -4228,6 +4230,7 @@
             .upsert(crate::connections::SavedConnection {
                 id: crate::connections::ConnectionId::new(),
                 name: "Production".into(),
+                group: None,
                 environment: crate::env::Environment::Production,
                 read_only: None,
                 password: crate::connections::PasswordMode::Ask,
@@ -4621,6 +4624,7 @@
             .upsert(crate::connections::SavedConnection {
                 id: crate::connections::ConnectionId::new(),
                 name: "Production".into(),
+                group: None,
                 environment: crate::env::Environment::Production,
                 read_only: None,
                 password: crate::connections::PasswordMode::None,
@@ -9251,6 +9255,7 @@
         let saved = crate::connections::SavedConnection {
             id: crate::connections::ConnectionId::new(),
             name: "Prod".into(),
+            group: None,
             environment: crate::env::Environment::Production,
             read_only: None,
             password: crate::connections::PasswordMode::Ask,
@@ -9284,6 +9289,7 @@
         let saved = crate::connections::SavedConnection {
             id: crate::connections::ConnectionId::new(),
             name: "Prod".into(),
+            group: None,
             environment: crate::env::Environment::None,
             read_only: None,
             password: crate::connections::PasswordMode::Ask,
@@ -9960,6 +9966,7 @@
         let saved = crate::connections::SavedConnection {
             id: crate::connections::ConnectionId::new(),
             name: "Prod".into(),
+            group: None,
             environment: crate::env::Environment::Production,
             read_only: None,
             password: crate::connections::PasswordMode::None,
@@ -10867,6 +10874,7 @@
         let saved = crate::connections::SavedConnection {
             id: crate::connections::ConnectionId::new(),
             name: "Prod".into(),
+            group: None,
             environment: crate::env::Environment::Production,
             read_only: None,
             password: crate::connections::PasswordMode::None,
@@ -10893,6 +10901,7 @@
         let saved = crate::connections::SavedConnection {
             id: crate::connections::ConnectionId::new(),
             name: "Prod".into(),
+            group: None,
             environment: crate::env::Environment::Production,
             read_only: None,
             password: crate::connections::PasswordMode::None,
diff --git a/src/ui/picker.rs b/src/ui/picker.rs
--- a/src/ui/picker.rs
+++ b/src/ui/picker.rs
@@ -1735,6 +1735,7 @@
         let connection = SavedConnection {
             id: ConnectionId::new(),
             name: "Fixture".into(),
+            group: None,
             environment: crate::env::Environment::Dev,
             read_only: None,
             password: crate::connections::PasswordMode::None,
@@ -1775,6 +1776,7 @@
         SavedConnection {
             id: ConnectionId::new(),
             name: "Bookshop exports".into(),
+            group: None,
             environment: crate::env::Environment::Local,
             read_only: Some(true),
             password: crate::connections::PasswordMode::None,
diff --git a/src/ui/reading_csv_tests.rs b/src/ui/reading_csv_tests.rs
--- a/src/ui/reading_csv_tests.rs
+++ b/src/ui/reading_csv_tests.rs
@@ -352,6 +352,7 @@
     let saved = SavedConnection {
         id: ConnectionId::new(),
         name: "Bookshop exports".into(),
+        group: None,
         environment: Environment::Local,
         read_only: Some(true),
         password: PasswordMode::None,
@@ -386,6 +387,7 @@
     let saved = SavedConnection {
         id: ConnectionId::new(),
         name: "Bookshop exports".into(),
+        group: None,
         environment: Environment::Local,
         read_only: Some(true),
         password: PasswordMode::None,
@@ -457,6 +459,7 @@
     let saved = SavedConnection {
         id: ConnectionId::new(),
         name: "Bookshop exports".into(),
+        group: None,
         environment: Environment::Local,
         read_only: Some(true),
         password: PasswordMode::None,
diff --git a/tests/cli.rs b/tests/cli.rs
--- a/tests/cli.rs
+++ b/tests/cli.rs
@@ -303,6 +303,7 @@
         let connection = SavedConnection {
             id: ConnectionId("shop".into()),
             name: "Bookshop".into(),
+            group: None,
             environment: Environment::Dev,
             read_only: None,
             password: PasswordMode::None,
@@ -379,6 +380,7 @@
         let connection = SavedConnection {
             id: ConnectionId("shop".into()),
             name: "Bookshop".into(),
+            group: None,
             environment: Environment::Dev,
             read_only: None,
             password: PasswordMode::None,
````

- [ ] **Step 4: Run the checks.** All four pass: 5,529 tests pass across the workspace.

- [ ] **Step 5: Commit.** "Keep a connection's group in connections.toml, as version 9, and find connections by it"

### Task 2: The grouping rule in the core

**Files:**
- Create: `crates/tabletist-core/src/connections/groups.rs`
- Modify: `crates/tabletist-core/src/connections.rs` (`pub mod groups;`)
- Modify: `src/ui/picker.rs` (its `Group`, `groups` and `is_local` give way to the core's)

- [ ] **Step 1: Write the failing tests:** the module, its tests alone.

````diff
diff --git a/crates/tabletist-core/src/connections.rs b/crates/tabletist-core/src/connections.rs
--- a/crates/tabletist-core/src/connections.rs
+++ b/crates/tabletist-core/src/connections.rs
@@ -12,6 +12,7 @@
 use crate::paths::AppDirs;
 
 pub(crate) mod file;
+pub mod groups;
 
 #[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
 pub struct ConnectionId(pub String);
diff --git a/crates/tabletist-core/src/connections/groups.rs b/crates/tabletist-core/src/connections/groups.rs
new file mode 100644
--- /dev/null
+++ b/crates/tabletist-core/src/connections/groups.rs
@@ -0,0 +1,264 @@
+//! The groups of the Connections list. A connection the person put in a
+//! group is under it. One with none falls under a name it shares with
+//! another such connection, else under Local or Remote by where it
+//! connects. Groups with the same title, in any case, are one.
+
+
+#[cfg(test)]
+mod tests {
+    use tabletist_db::{ConnectSpec, SshAuth, SshSpec};
+
+    use super::*;
+    use crate::connections::{ConnectionId, PasswordMode};
+    use crate::env::Environment;
+
+    fn saved(name: &str, group: Option<&str>, spec: ConnectSpec) -> SavedConnection {
+        SavedConnection {
+            id: ConnectionId::new(),
+            name: name.into(),
+            group: group.map(Into::into),
+            environment: Environment::None,
+            read_only: None,
+            password: PasswordMode::None,
+            ssh_secret: PasswordMode::None,
+            spec,
+            agents: Default::default(),
+        }
+    }
+
+    fn server(host: &str) -> ConnectSpec {
+        ConnectSpec {
+            driver: Driver::Postgres,
+            host: host.into(),
+            port: 5432,
+            ..ConnectSpec::default()
+        }
+    }
+
+    fn file(path: &str) -> ConnectSpec {
+        ConnectSpec::sqlite(path)
+    }
+
+    /// The headings and, under each, its connections' names.
+    fn shown(connections: &[SavedConnection]) -> Vec<(String, Vec<String>)> {
+        let all: Vec<&SavedConnection> = connections.iter().collect();
+        groups(&all)
+            .into_iter()
+            .map(|group| {
+                let names = group.connections.iter().map(|c| c.name.clone()).collect();
+                (group.title, names)
+            })
+            .collect()
+    }
+
+    fn heading(title: &str, names: &[&str]) -> (String, Vec<String>) {
+        (
+            title.to_owned(),
+            names.iter().map(|name| (*name).to_owned()).collect(),
+        )
+    }
+
+    #[test]
+    fn without_groups_a_shared_name_heads_one_and_the_rest_are_local_or_remote() {
+        let tunnelled = ConnectSpec {
+            ssh: Some(SshSpec {
+                host: "bastion.example.com".into(),
+                port: None,
+                user: "deploy".into(),
+                auth: SshAuth::Agent,
+            }),
+            ..server("localhost")
+        };
+        let connections = [
+            saved("Rails test", None, server("localhost")),
+            saved("Bookshop", None, server("localhost")),
+            saved("Reports", None, server("db.example.com")),
+            saved("Bookshop", None, server("db.example.com")),
+            saved("Playground", None, file("/srv/playground.db")),
+            saved("Tunnelled", None, tunnelled),
+        ];
+        assert_eq!(
+            shown(&connections),
+            [
+                heading("Bookshop", &["Bookshop", "Bookshop"]),
+                heading("Local", &["Rails test", "Playground"]),
+                heading("Remote", &["Reports", "Tunnelled"]),
+            ]
+        );
+    }
+
+    #[test]
+    fn a_group_the_person_gave_heads_its_own_among_the_shared_names() {
+        let connections = [
+            saved("Rails test", None, server("localhost")),
+            saved(
+                "Playground",
+                Some("Experiments"),
+                file("/srv/playground.db"),
+            ),
+            saved("Bookshop", None, server("localhost")),
+            saved("Bookshop", None, server("db.example.com")),
+            saved("Reports", Some("  Clients "), server("db.example.com")),
+        ];
+        assert_eq!(
+            shown(&connections),
+            [
+                heading("Bookshop", &["Bookshop", "Bookshop"]),
+                heading("Clients", &["Reports"]),
+                heading("Experiments", &["Playground"]),
+                heading("Local", &["Rails test"]),
+            ]
+        );
+    }
+
+    #[test]
+    fn titles_in_another_case_are_one_group_as_the_first_writes_it() {
+        let connections = [
+            saved("Playground", Some("experiments"), file("/p.db")),
+            saved("Scratch", Some("Experiments"), file("/s.db")),
+            // The person's "local" is the rule's Local.
+            saved("Reports", Some("local"), server("db.example.com")),
+            saved("Rails test", None, server("localhost")),
+        ];
+        assert_eq!(
+            shown(&connections),
+            [
+                heading("experiments", &["Playground", "Scratch"]),
+                heading("local", &["Reports", "Rails test"]),
+            ]
+        );
+    }
+
+    #[test]
+    fn a_connection_in_a_group_shares_its_name_with_none() {
+        // Two Bookshops, one of them put with the clients: the other is
+        // alone with its name, and goes by where it connects.
+        let connections = [
+            saved("Bookshop", Some("Clients"), server("db.example.com")),
+            saved("Bookshop", None, server("localhost")),
+        ];
+        assert_eq!(
+            shown(&connections),
+            [
+                heading("Clients", &["Bookshop"]),
+                heading("Local", &["Bookshop"]),
+            ]
+        );
+        // A blank group is none.
+        let blank = [
+            saved("Bookshop", Some("  "), server("db.example.com")),
+            saved("Bookshop", None, server("localhost")),
+        ];
+        assert_eq!(
+            shown(&blank),
+            [heading("Bookshop", &["Bookshop", "Bookshop"])]
+        );
+    }
+
+    #[test]
+    fn a_namesake_put_in_a_group_of_its_name_still_shares_it() {
+        // One Bookshop saved in "Bookshop" while the other was elsewhere;
+        // that one then has no group: one heading holds both.
+        let connections = [
+            saved("Bookshop", None, server("localhost")),
+            saved("Bookshop", Some("bookshop"), server("db.example.com")),
+        ];
+        assert_eq!(
+            shown(&connections),
+            [heading("Bookshop", &["Bookshop", "Bookshop"])]
+        );
+        // A name read with spaces round it is the same name.
+        let spaced = [
+            saved("Bookshop ", None, server("localhost")),
+            saved("Bookshop", None, server("db.example.com")),
+        ];
+        assert_eq!(shown(&spaced).len(), 1, "{:?}", shown(&spaced));
+    }
+
+    #[test]
+    fn the_choices_are_the_groups_shown_and_the_connections_own() {
+        let connections = [
+            saved("Bookshop", None, server("localhost")),
+            saved("Bookshop", None, server("db.example.com")),
+            saved("Playground", Some("Experiments"), file("/p.db")),
+        ];
+        let all: Vec<&SavedConnection> = connections.iter().collect();
+        // A new connection to a server: Remote is offered, though no row
+        // is there yet.
+        let remote = automatic("Reports", false, all.iter().copied());
+        assert_eq!(remote, "Remote");
+        assert_eq!(
+            choices(&all, &remote),
+            ["Bookshop", "Experiments", "Remote"]
+        );
+        // One more Bookshop falls under the name it shares.
+        assert_eq!(automatic("Bookshop", true, all.iter().copied()), "Bookshop");
+        assert_eq!(choices(&all, "Bookshop"), ["Bookshop", "Experiments"]);
+        // The Playground, edited: without its group it would be Local.
+        let others = all.iter().copied().filter(|c| c.id != connections[2].id);
+        assert_eq!(automatic("Playground", true, others), "Local");
+        assert_eq!(choices(&all, "Local"), ["Bookshop", "Experiments", "Local"]);
+    }
+
+    #[test]
+    fn the_form_offers_the_groups_and_shows_the_one_in_use() {
+        let connections = [
+            saved("Bookshop", None, server("localhost")),
+            saved("Bookshop", None, server("db.example.com")),
+            saved("Playground", Some("experiments"), file("/p.db")),
+        ];
+        // Editing the Playground: its own group as the list writes it, or
+        // Local without one.
+        let editing = Some(&connections[2].id);
+        let offered = Choices::of("Playground", true, editing, &connections);
+        assert_eq!(offered.titles, ["Bookshop", "experiments", "Local"]);
+        assert_eq!(offered.automatic, "Local");
+        assert_eq!(offered.shown(Some("Experiments")), "experiments");
+        assert_eq!(offered.shown(None), "Local");
+        assert_eq!(offered.shown(Some(" ")), "Local");
+        // A group typed that no connection is in yet reads as typed.
+        assert_eq!(offered.shown(Some("Clients")), "Clients");
+    }
+
+    #[test]
+    fn a_connection_keeps_its_group_unless_the_rule_gives_it_anyway() {
+        let mut connections = vec![
+            saved("Bookshop", None, server("db.example.com")),
+            saved("Bookshop", Some("Bookshop"), server("localhost")),
+            saved("Playground", Some("Experiments"), file("/p.db")),
+        ];
+        // The second shares its name with the first: its group is the
+        // rule's own.
+        assert_eq!(keep(&connections[1], &connections), None);
+        assert_eq!(
+            keep(&connections[2], &connections),
+            Some("Experiments".into())
+        );
+        connections[2].group = Some("LOCAL".into());
+        assert_eq!(keep(&connections[2], &connections), None);
+    }
+
+    #[test]
+    fn only_a_group_other_than_the_rules_own_is_kept() {
+        assert_eq!(kept(Some("Clients"), "Local"), Some("Clients".into()));
+        assert_eq!(kept(Some("  Clients  "), "Local"), Some("Clients".into()));
+        assert_eq!(kept(Some("local"), "Local"), None);
+        assert_eq!(kept(Some("Local"), "Remote"), Some("Local".into()));
+        assert_eq!(kept(Some("   "), "Local"), None);
+        assert_eq!(kept(None, "Local"), None);
+    }
+
+    #[test]
+    fn a_file_or_a_local_host_without_a_tunnel_stays_on_this_machine() {
+        assert!(stays_on_this_machine(Driver::Sqlite, "", false));
+        assert!(stays_on_this_machine(Driver::Files, "", false));
+        assert!(stays_on_this_machine(Driver::Postgres, "localhost", false));
+        assert!(!stays_on_this_machine(Driver::Postgres, "localhost", true));
+        assert!(!stays_on_this_machine(
+            Driver::MySql,
+            "db.example.com",
+            false
+        ));
+        assert!(!stays_on_this_machine(Driver::BigQuery, "", false));
+    }
+}
````

- [ ] **Step 2: Run them, and see them fail.**

Run: `cargo test --locked -p tabletist-core --lib connections::groups`

Expected: nothing the tests name is there yet:

```text
error[E0425]: cannot find type `SavedConnection` in this scope
error[E0422]: cannot find struct, variant or union type `SavedConnection` in this scope
error[E0425]: cannot find function `automatic` in this scope
error[E0425]: cannot find function `keep` in this scope
error[E0425]: cannot find function `kept` in this scope
error[E0433]: cannot find type `Driver` in this scope
error[E0425]: cannot find function `groups` in this scope
error[E0425]: cannot find function `choices` in this scope
error[E0433]: cannot find type `Choices` in this scope
error[E0425]: cannot find function `stays_on_this_machine` in this scope
error: could not compile `tabletist-core` (lib test) due to 35 previous errors; 1 warning emitted
```

- [ ] **Step 3: Write the code.** The rule, and the picker taking it: its `Group` and `groups` go, and its `is_local`, which the TLS label still uses, asks the core.

````diff
diff --git a/crates/tabletist-core/src/connections/groups.rs b/crates/tabletist-core/src/connections/groups.rs
--- a/crates/tabletist-core/src/connections/groups.rs
+++ b/crates/tabletist-core/src/connections/groups.rs
@@ -3,6 +3,194 @@
 //! another such connection, else under Local or Remote by where it
 //! connects. Groups with the same title, in any case, are one.
 
+use tabletist_db::Driver;
+
+use super::SavedConnection;
+
+/// The rule's group of connections that stay on this machine.
+pub const LOCAL: &str = "Local";
+
+/// The rule's group of the other connections.
+pub const REMOTE: &str = "Remote";
+
+/// Connections under one heading, in the file's order.
+pub struct Group<'a> {
+    /// As the first of them that names it writes it, or the rule's own.
+    pub title: String,
+    pub connections: Vec<&'a SavedConnection>,
+}
+
+/// The group the person gave `connection`, if any: what they wrote,
+/// without the spaces at its ends.
+pub fn own(connection: &SavedConnection) -> Option<&str> {
+    connection
+        .group
+        .as_deref()
+        .map(str::trim)
+        .filter(|group| !group.is_empty())
+}
+
+/// The group a connection named `name` falls under while it has none of
+/// its own: its name where another of `others` shares it and has no other
+/// group (none, or that name), else Local where it stays on this machine
+/// (`local`), else Remote.
+pub fn automatic<'a>(
+    name: &str,
+    local: bool,
+    others: impl IntoIterator<Item = &'a SavedConnection>,
+) -> String {
+    let name = name.trim();
+    let shared = others
+        .into_iter()
+        .any(|other| other.name.trim() == name && own(other).is_none_or(|group| same(group, name)));
+    match (shared, local) {
+        (true, _) => name.to_owned(),
+        (false, true) => LOCAL.to_owned(),
+        (false, false) => REMOTE.to_owned(),
+    }
+}
+
+/// The group `connection` is under among `connections`, which hold it.
+pub fn of(connection: &SavedConnection, connections: &[&SavedConnection]) -> String {
+    match own(connection) {
+        Some(group) => group.to_owned(),
+        None => automatic(
+            &connection.name,
+            stays_local(&connection.spec),
+            connections
+                .iter()
+                .copied()
+                .filter(|other| other.id != connection.id),
+        ),
+    }
+}
+
+/// `connections` in groups, in the list's order: by title, ignoring case,
+/// with Local and then Remote last.
+pub fn groups<'a>(connections: &[&'a SavedConnection]) -> Vec<Group<'a>> {
+    let mut groups: Vec<Group<'a>> = Vec::new();
+    for connection in connections {
+        let title = of(connection, connections);
+        match groups.iter_mut().find(|group| same(&group.title, &title)) {
+            Some(group) => group.connections.push(connection),
+            None => groups.push(Group {
+                title,
+                connections: vec![connection],
+            }),
+        }
+    }
+    groups.sort_by_key(|group| place(&group.title));
+    groups
+}
+
+/// The groups a connection can be put in: those `connections` are in,
+/// and `automatic`, the one it falls under without one of its own, in the
+/// list's order.
+pub fn choices(connections: &[&SavedConnection], automatic: &str) -> Vec<String> {
+    let mut titles: Vec<String> = groups(connections)
+        .into_iter()
+        .map(|group| group.title)
+        .collect();
+    if !titles.iter().any(|title| same(title, automatic)) {
+        titles.push(automatic.to_owned());
+        titles.sort_by_key(|title| place(title));
+    }
+    titles
+}
+
+/// What the connection form's Group offers: the groups in the list's
+/// order, and the one the connection falls under without its own.
+#[derive(Debug, Clone, Default, PartialEq)]
+pub struct Choices {
+    pub titles: Vec<String>,
+    pub automatic: String,
+}
+
+impl Choices {
+    /// For a connection named `name` (`local` where it stays on this
+    /// machine) among `connections`, the saved ones; `editing` is its id
+    /// when it is one of them.
+    pub fn of(
+        name: &str,
+        local: bool,
+        editing: Option<&super::ConnectionId>,
+        connections: &[SavedConnection],
+    ) -> Self {
+        let others = connections
+            .iter()
+            .filter(|other| Some(&other.id) != editing);
+        let automatic = automatic(name.trim(), local, others);
+        let all: Vec<&SavedConnection> = connections.iter().collect();
+        Self {
+            titles: choices(&all, &automatic),
+            automatic,
+        }
+    }
+
+    /// The title the Group shows for `chosen`, the form's own group: as
+    /// the list writes it, else as chosen, else the rule's.
+    pub fn shown<'a>(&'a self, chosen: Option<&'a str>) -> &'a str {
+        let Some(chosen) = chosen.map(str::trim).filter(|chosen| !chosen.is_empty()) else {
+            return &self.automatic;
+        };
+        self.titles
+            .iter()
+            .find(|title| same(title, chosen))
+            .map_or(chosen, String::as_str)
+    }
+}
+
+/// The group to save for `connection`, given the others saved: its own,
+/// unless that is the group the rule gives it anyway.
+pub fn keep(connection: &SavedConnection, connections: &[SavedConnection]) -> Option<String> {
+    let others = connections.iter().filter(|other| other.id != connection.id);
+    let automatic = automatic(
+        connection.name.trim(),
+        stays_local(&connection.spec),
+        others,
+    );
+    kept(connection.group.as_deref(), &automatic)
+}
+
+/// The group to save for a connection given `chosen`, where `automatic`
+/// is the one it falls under without: none for a blank one, or for the
+/// group the rule gives it anyway, which then follows the rule.
+pub fn kept(chosen: Option<&str>, automatic: &str) -> Option<String> {
+    let chosen = chosen.map(str::trim).filter(|group| !group.is_empty())?;
+    (!same(chosen, automatic)).then(|| chosen.to_owned())
+}
+
+/// Whether two titles name one group.
+pub fn same(one: &str, other: &str) -> bool {
+    one.trim().to_lowercase() == other.trim().to_lowercase()
+}
+
+/// Whether a connection of `driver` to `host` stays on this machine: a
+/// file, or a local host without a tunnel (`tunnel`).
+pub fn stays_on_this_machine(driver: Driver, host: &str, tunnel: bool) -> bool {
+    match driver {
+        Driver::Sqlite | Driver::Files => true,
+        Driver::Postgres | Driver::MySql => !tunnel && crate::model::is_local_host(host),
+        Driver::BigQuery => false,
+    }
+}
+
+fn stays_local(spec: &tabletist_db::ConnectSpec) -> bool {
+    stays_on_this_machine(spec.driver, &spec.host, spec.ssh.is_some())
+}
+
+/// Where a title sorts: the person's and the shared names by their words,
+/// then Local, then Remote.
+fn place(title: &str) -> (u8, String) {
+    let rank = if same(title, LOCAL) {
+        1
+    } else if same(title, REMOTE) {
+        2
+    } else {
+        0
+    };
+    (rank, title.trim().to_lowercase())
+}
 
 #[cfg(test)]
 mod tests {
diff --git a/src/ui/picker.rs b/src/ui/picker.rs
--- a/src/ui/picker.rs
+++ b/src/ui/picker.rs
@@ -55,66 +55,13 @@
         .map_or(laid.height() * 0.8, |glyph| glyph.pos.y)
 }
 
-/// Connections under one heading: those sharing a name, then the local
-/// and the remote rest.
-pub struct Group<'a> {
-    pub title: String,
-    pub connections: Vec<&'a SavedConnection>,
-}
-
-/// `connections` in groups: a name two or more share heads a group of its
-/// own; the others go under Local or Remote by where they connect.
-pub fn groups<'a>(connections: &[&'a SavedConnection]) -> Vec<Group<'a>> {
-    let mut named: Vec<Group<'a>> = Vec::new();
-    let mut local = Vec::new();
-    let mut remote = Vec::new();
-    for connection in connections {
-        let shared = connections
-            .iter()
-            .filter(|other| other.name == connection.name)
-            .count()
-            > 1;
-        if shared {
-            match named
-                .iter_mut()
-                .find(|group| group.title == connection.name)
-            {
-                Some(group) => group.connections.push(connection),
-                None => named.push(Group {
-                    title: connection.name.clone(),
-                    connections: vec![connection],
-                }),
-            }
-        } else if is_local(connection) {
-            local.push(*connection);
-        } else {
-            remote.push(*connection);
-        }
-    }
-    named.sort_by_key(|group| group.title.to_lowercase());
-    for (title, connections) in [("Local", local), ("Remote", remote)] {
-        if !connections.is_empty() {
-            named.push(Group {
-                title: title.into(),
-                connections,
-            });
-        }
-    }
-    named
-}
+pub use crate::connections::groups::{Group, groups};
 
 /// Whether a connection stays on this machine: a file, or a local host
 /// without a tunnel.
 fn is_local(connection: &SavedConnection) -> bool {
-    use tabletist_db::Driver;
     let spec = &connection.spec;
-    match spec.driver {
-        Driver::Sqlite | Driver::Files => true,
-        Driver::Postgres | Driver::MySql => {
-            spec.ssh.is_none() && crate::model::is_local_host(&spec.host)
-        }
-        Driver::BigQuery => false,
-    }
+    crate::connections::groups::stays_on_this_machine(spec.driver, &spec.host, spec.ssh.is_some())
 }
 
 /// The connections the picker lists, in its order: grouped and filtered by
````

- [ ] **Step 4: Run the checks.** All four pass: 5,539 tests pass across the workspace. The picker's tests are unchanged and pass: without groups the rule is today's.

- [ ] **Step 5: Commit.** "Move the Connections list's groups into the core, with a group of the person's own first"

### Task 3: Group in the connection form

**Files:**
- Create: `src/ui/connect_dialog/group.rs`, `src/ui/group_tests.rs`
- Modify: `crates/tabletist-core/src/model.rs`, `src/ui/connect_dialog/mod.rs`, `src/ui/connect_dialog/sheet.rs`, `src/ui/connect_dialog/terminal.rs`, `src/ui/connect_dialog/agents.rs` (`cancel_asked` for the new field), `src/ui/mod.rs`, `src/app.rs`, `src/ui/env_tests.rs`
- Modify: `assets/i18n/tabletist.pot`, `assets/i18n/ru.po`, `assets/i18n/pt-BR.po`

- [ ] **Step 1: Write the failing tests:** the harness tests, which read only the AccessKit tree and what is saved, and the form's unit test.

````diff
diff --git a/crates/tabletist-core/src/model.rs b/crates/tabletist-core/src/model.rs
--- a/crates/tabletist-core/src/model.rs
+++ b/crates/tabletist-core/src/model.rs
@@ -8322,6 +8322,40 @@
     }
 
     #[test]
+    fn the_group_chosen_or_typed_is_saved_and_read_back() {
+        let chosen = ConnectionForm {
+            group: Some("Experiments".into()),
+            ..ssh_form()
+        };
+        let saved = chosen.to_saved().unwrap();
+        assert_eq!(saved.group.as_deref(), Some("Experiments"));
+        assert_eq!(ConnectionForm::from_saved(&saved).group, saved.group);
+        // A name typed after New group… wins over the choice, without the
+        // spaces round it; a blank one leaves the connection to the rule.
+        let typed = |text: &str| {
+            ConnectionForm {
+                group: Some("Experiments".into()),
+                group_typed: Some(text.into()),
+                ..ssh_form()
+            }
+            .to_saved()
+            .unwrap()
+            .group
+        };
+        assert_eq!(typed(" Side projects "), Some("Side projects".into()));
+        assert_eq!(typed("  "), None);
+        assert_eq!(ssh_form().to_saved().unwrap().group, None);
+        // The fields decide where the rule would put it.
+        assert!(!ssh_form().stays_local());
+        let local = ConnectionForm {
+            ssh: false,
+            host: "localhost".into(),
+            ..ssh_form()
+        };
+        assert!(local.stays_local());
+    }
+
+    #[test]
     fn a_new_production_connection_keeps_the_switch_the_settings_gave_it() {
         for on in [true, false] {
             let form = production_form(on);
diff --git a/src/ui/group_tests.rs b/src/ui/group_tests.rs
new file mode 100644
--- /dev/null
+++ b/src/ui/group_tests.rs
@@ -0,0 +1,337 @@
+//! Connection groups in the connection form and the Connections list, in
+//! both looks: Group offers the list's groups and New group…, a connection
+//! given a group is listed under it and stays there after a relaunch, and
+//! the rule's own group, chosen, leaves it to the rule.
+
+use egui::accesskit::{self, Node, NodeId, Role, TreeUpdate};
+use egui::{Key, Modifiers};
+
+use crate::backend::{Command, StateFile};
+use crate::connections::{ConnectionId, PasswordMode, SavedConnection, SavedConnections};
+use crate::env::Environment;
+use crate::model::Action;
+use crate::paths::AppDirs;
+use crate::testing::{Harness, bounds};
+use crate::theme::Look;
+
+fn looks() -> [Look; 2] {
+    [Look::macos(), Look::omarchy()]
+}
+
+fn add(harness: &mut Harness, name: &str, group: Option<&str>, path: &str) -> ConnectionId {
+    let saved = SavedConnection {
+        id: ConnectionId::new(),
+        name: name.into(),
+        group: group.map(Into::into),
+        environment: Environment::Local,
+        read_only: None,
+        password: PasswordMode::None,
+        ssh_secret: PasswordMode::None,
+        spec: tabletist_db::ConnectSpec::sqlite(path),
+        agents: Default::default(),
+    };
+    let id = saved.id.clone();
+    harness.app.connections.upsert(saved);
+    id
+}
+
+/// The Bookshop's demo connections, on files: two that share a name, the
+/// Playground, and the Rails test. Returns the Playground's and the Rails
+/// test's ids.
+fn bookshop(look: Look, playground: Option<&str>) -> (Harness, ConnectionId, ConnectionId) {
+    let mut harness = Harness::new();
+    harness.set_look(look);
+    add(
+        &mut harness,
+        "Bookshop",
+        None,
+        "/srv/bookshop_development.db",
+    );
+    add(&mut harness, "Bookshop", None, "/srv/bookshop_staging.db");
+    let play = add(&mut harness, "Playground", playground, "/srv/playground.db");
+    let test = add(&mut harness, "Rails test", None, "/srv/bookshop_test.db");
+    harness.settle();
+    (harness, play, test)
+}
+
+/// Whether the list shows `row` under the heading `title`: below it, and
+/// above the next heading of `titles`, the list's in order.
+fn listed_under(harness: &mut Harness, titles: &[&str], title: &str, row: &str) -> bool {
+    let tree = harness.settle();
+    let top = |label: &str| bounds(&tree, label, Role::Button).map(|rect| rect.top());
+    let at = titles.iter().position(|each| *each == title).unwrap();
+    let (Some(heading), Some(row)) = (top(title), top(row)) else {
+        return false;
+    };
+    let next = titles.get(at + 1).and_then(|next| top(next));
+    heading < row && next.is_none_or(|next| row < next)
+}
+
+/// The control with `role` named `name` in the look's case: by its own
+/// name, or by the label it is labelled by.
+fn find_control<'t>(
+    tree: &'t TreeUpdate,
+    look: &Look,
+    name: &str,
+    role: Role,
+) -> Option<(NodeId, &'t Node)> {
+    let name = look.label(name);
+    let label = tree
+        .nodes
+        .iter()
+        .find(|(_, node)| {
+            node.role() == Role::Label && node.label().or_else(|| node.value()) == Some(&name)
+        })
+        .map(|(id, _)| *id);
+    tree.nodes
+        .iter()
+        .find(|(_, node)| {
+            node.role() == role
+                && (node.label() == Some(&name)
+                    || (role == Role::Label && node.value() == Some(&name))
+                    || label.is_some_and(|label| node.labelled_by().contains(&label)))
+        })
+        .map(|(id, node)| (*id, node))
+}
+
+/// [`find_control`], which is there.
+fn control<'t>(tree: &'t TreeUpdate, look: &Look, name: &str, role: Role) -> (NodeId, &'t Node) {
+    find_control(tree, look, name, role)
+        .unwrap_or_else(|| panic!("no {name} ({role:?}): {:?}", crate::testing::labels(tree)))
+}
+
+/// Clicks the node `target`, as a screen reader does.
+fn click(harness: &mut Harness, target: NodeId) {
+    harness.frame(vec![egui::Event::AccessKitActionRequest(
+        accesskit::ActionRequest {
+            target_tree: accesskit::TreeId::ROOT,
+            target_node: target,
+            action: accesskit::Action::Click,
+            data: None,
+        },
+    )]);
+    harness.settle();
+}
+
+/// Opens the Group dropdown.
+fn open_group(harness: &mut Harness) {
+    let look = harness.app.look;
+    let tree = harness.settle();
+    let (id, _) = control(&tree, &look, "Group", Role::ComboBox);
+    click(harness, id);
+}
+
+/// The item `name` of the Group dropdown's menu: a button in the menu
+/// under it, as wide as it, not a row of the list behind the dialog.
+fn menu_item(harness: &mut Harness, name: &str) -> Option<NodeId> {
+    let look = harness.app.look;
+    let tree = harness.settle();
+    let dropdown = control(&tree, &look, "Group", Role::ComboBox)
+        .1
+        .bounds()
+        .unwrap();
+    tree.nodes
+        .iter()
+        .find(|(_, node)| {
+            node.role() == Role::Button
+                && node.label() == Some(name)
+                && node.bounds().is_some_and(|item| {
+                    item.y0 >= dropdown.y1 - 1.0
+                        && item.x0 >= dropdown.x0 - 8.0
+                        && item.x1 <= dropdown.x1 + 8.0
+                })
+        })
+        .map(|(id, _)| *id)
+}
+
+fn choose(harness: &mut Harness, name: &str) {
+    let item = menu_item(harness, name).unwrap_or_else(|| panic!("no item {name}"));
+    click(harness, item);
+}
+
+/// The menu's last item: the form's own words, in the look's case, where
+/// the groups are as their connections write them.
+fn new_group(look: &Look) -> String {
+    look.label("New group…")
+}
+
+/// What is typed in Group's field, while New group… has it open.
+fn group_typed(harness: &mut Harness) -> Option<String> {
+    let look = harness.app.look;
+    let tree = harness.settle();
+    let (_, field) = find_control(&tree, &look, "Group", Role::TextInput)?;
+    Some(field.value().unwrap_or_default().to_owned())
+}
+
+/// The value the Group dropdown reads.
+fn group_shown(harness: &mut Harness) -> Option<String> {
+    let look = harness.app.look;
+    let tree = harness.settle();
+    let (_, node) = control(&tree, &look, "Group", Role::ComboBox);
+    node.value().map(str::to_owned)
+}
+
+/// What the app asked the backend to write as `connections.toml` last.
+fn last_saved(harness: &Harness) -> SavedConnections {
+    harness
+        .app
+        .backend
+        .sent
+        .iter()
+        .rev()
+        .find_map(|command| match command {
+            Command::Save {
+                file: StateFile::Connections(store),
+                ..
+            } => Some(store.clone()),
+            _ => None,
+        })
+        .expect("a save of the connections")
+}
+
+#[test]
+fn a_connection_given_a_new_group_in_the_form_is_listed_under_it_after_a_relaunch() {
+    for look in looks() {
+        let (mut harness, play, _) = bookshop(look, None);
+        let titles = ["Bookshop", "Local"];
+        assert!(listed_under(&mut harness, &titles, "Local", "Playground"));
+        harness.app.apply(Action::EditConnection(play.clone()));
+        assert_eq!(group_shown(&mut harness).as_deref(), Some("Local"));
+        open_group(&mut harness);
+        choose(&mut harness, &new_group(&look));
+        // The dropdown is a field now, with the keyboard.
+        harness.frame(vec![egui::Event::Text("Experiments".into())]);
+        assert_eq!(
+            group_typed(&mut harness).as_deref(),
+            Some("Experiments"),
+            "{}",
+            look.name
+        );
+        harness.app.apply(Action::SaveConnection { connect: false });
+        assert!(harness.app.dialog.is_none(), "{}", look.name);
+        let saved = harness.app.connections.get(&play).unwrap();
+        assert_eq!(saved.group.as_deref(), Some("Experiments"));
+        let titles = ["Bookshop", "Experiments", "Local"];
+        assert!(
+            listed_under(&mut harness, &titles, "Experiments", "Playground"),
+            "{}",
+            look.name
+        );
+        assert!(listed_under(&mut harness, &titles, "Local", "Rails test"));
+        // The file the backend writes is read at the next start.
+        let dir = tempfile::tempdir().unwrap();
+        let dirs = AppDirs::at(dir.path());
+        last_saved(&harness).save(&dirs.connections_file()).unwrap();
+        let loaded = SavedConnections::load(&dirs);
+        assert_eq!(loaded.problem, None);
+        let mut relaunched = Harness::new();
+        relaunched.set_look(look);
+        relaunched.app.connections = loaded.store;
+        assert!(
+            listed_under(&mut relaunched, &titles, "Experiments", "Playground"),
+            "{}: after a relaunch",
+            look.name
+        );
+    }
+}
+
+#[test]
+fn a_group_chosen_from_the_dropdown_is_kept_and_the_rules_own_clears_it() {
+    for look in looks() {
+        let (mut harness, _, test) = bookshop(look, Some("Experiments"));
+        harness.app.apply(Action::EditConnection(test.clone()));
+        open_group(&mut harness);
+        // The list's groups, the Rails test's own among them, and New
+        // group… under them. No row is remote: Remote is not offered.
+        for offered in ["Bookshop", "Experiments", "Local", &new_group(&look)] {
+            assert!(
+                menu_item(&mut harness, offered).is_some(),
+                "{}: {offered}",
+                look.name
+            );
+        }
+        assert!(menu_item(&mut harness, "Remote").is_none());
+        choose(&mut harness, "Experiments");
+        assert_eq!(group_shown(&mut harness).as_deref(), Some("Experiments"));
+        harness.app.apply(Action::SaveConnection { connect: false });
+        let saved = harness.app.connections.get(&test).unwrap();
+        assert_eq!(saved.group.as_deref(), Some("Experiments"), "{}", look.name);
+        // Local is the group the rule gives it: chosen, the connection has
+        // none of its own again.
+        harness.app.apply(Action::EditConnection(test.clone()));
+        open_group(&mut harness);
+        choose(&mut harness, "Local");
+        assert_eq!(group_shown(&mut harness).as_deref(), Some("Local"));
+        harness.app.apply(Action::SaveConnection { connect: false });
+        assert_eq!(harness.app.connections.get(&test).unwrap().group, None);
+    }
+}
+
+#[test]
+fn escape_in_the_new_groups_field_goes_back_to_the_dropdown() {
+    for look in looks() {
+        let (mut harness, play, _) = bookshop(look, Some("Experiments"));
+        harness.app.apply(Action::EditConnection(play.clone()));
+        open_group(&mut harness);
+        choose(&mut harness, &new_group(&look));
+        harness.frame(vec![egui::Event::Text("Clients".into())]);
+        harness.press(Key::Escape, Modifiers::NONE);
+        assert!(
+            harness.app.dialog.is_some(),
+            "{}: the dialog stays",
+            look.name
+        );
+        assert_eq!(group_typed(&mut harness), None, "{}", look.name);
+        assert_eq!(group_shown(&mut harness).as_deref(), Some("Experiments"));
+        // The keyboard is back on the dropdown.
+        let focused = harness.ctx.memory(|memory| memory.focused());
+        let tree = harness.settle();
+        let (dropdown, _) = control(&tree, &look, "Group", Role::ComboBox);
+        assert_eq!(
+            focused.map(|id| id.accesskit_id()),
+            Some(dropdown),
+            "{}",
+            look.name
+        );
+        // What was typed is gone: a save keeps the group as it was.
+        harness.app.apply(Action::SaveConnection { connect: false });
+        let saved = harness.app.connections.get(&play).unwrap();
+        assert_eq!(saved.group.as_deref(), Some("Experiments"), "{}", look.name);
+    }
+}
+
+/// macOS: Group beside Name, as the connection form's board draws it, the
+/// name the wider. The terminal: a row of its own between name and
+/// environment.
+#[test]
+fn group_sits_beside_the_name_on_macos_and_under_it_in_the_terminal() {
+    for look in looks() {
+        let mut harness = Harness::new();
+        harness.set_look(look);
+        harness.app.apply(Action::NewConnection);
+        let tree = harness.settle();
+        let name = control(&tree, &look, "Name", Role::TextInput)
+            .1
+            .bounds()
+            .unwrap();
+        let group = control(&tree, &look, "Group", Role::ComboBox)
+            .1
+            .bounds()
+            .unwrap();
+        if look.terminal {
+            assert!(group.y0 >= name.y1, "under the name");
+            // The environment's row, by its label: the buttons' group has
+            // no bounds of its own.
+            let environment = control(&tree, &look, "Environment", Role::Label)
+                .1
+                .bounds()
+                .unwrap();
+            assert!(environment.y0 >= group.y1, "above the environment");
+        } else {
+            assert!((group.y0 - name.y0).abs() < 1.0, "tops in line");
+            assert!(group.x0 > name.x1, "beside it");
+            let ratio = name.width() / group.width();
+            assert!((1.3..1.5).contains(&ratio), "1.4 : 1, not {ratio}");
+        }
+    }
+}
diff --git a/src/ui/mod.rs b/src/ui/mod.rs
--- a/src/ui/mod.rs
+++ b/src/ui/mod.rs
@@ -82,6 +82,8 @@
 pub mod grid;
 #[cfg(test)]
 mod grid_keys_tests;
+#[cfg(test)]
+mod group_tests;
 pub mod help;
 #[cfg(test)]
 mod help_tests;
````

- [ ] **Step 2: Run them, and see them fail.**

Run: `cargo test --locked -p tabletist-core --lib the_group_chosen_or_typed`

Expected: the form has no `group` yet:

```text
error[E0560]: struct `model::ConnectionForm` has no field named `group`
error[E0609]: no field `group` on type `model::ConnectionForm`
error[E0560]: struct `model::ConnectionForm` has no field named `group_typed`
error[E0599]: no method named `stays_local` found for struct `model::ConnectionForm` in the current scope
error: could not compile `tabletist-core` (lib test) due to 6 previous errors
```

Run: `cargo test --locked -p tabletist --lib group_tests`

Expected: four failures, the form has no Group:

```text
test ui::group_tests::group_sits_beside_the_name_on_macos_and_under_it_in_the_terminal ... FAILED
test ui::group_tests::escape_in_the_new_groups_field_goes_back_to_the_dropdown ... FAILED
test ui::group_tests::a_group_chosen_from_the_dropdown_is_kept_and_the_rules_own_clears_it ... FAILED
test ui::group_tests::a_connection_given_a_new_group_in_the_form_is_listed_under_it_after_a_relaunch ... FAILED
thread 'ui::group_tests::group_sits_beside_the_name_on_macos_and_under_it_in_the_terminal' panicked at src/ui/group_tests.rs:100:28:
    no Group (ComboBox): ["Open read-only", "Name", "Access  …
thread 'ui::group_tests::escape_in_the_new_groups_field_goes_back_to_the_dropdown' panicked at src/ui/group_tests.rs:100:28:
    no Group (ComboBox): ["Name", "Edit Bookshop", "All exce …
thread 'ui::group_tests::a_group_chosen_from_the_dropdown_is_kept_and_the_rules_own_clears_it' panicked at src/ui/group_tests.rs:100:28:
    no Group (ComboBox): ["Name", "All except", "Last used n …
thread 'ui::group_tests::a_connection_given_a_new_group_in_the_form_is_listed_under_it_after_a_relaunch' panicked at src/ui/group_tests.rs:100:28:
    no Group (ComboBox): ["Name", "Bookshop", "All except",  …
test result: FAILED. 0 passed; 4 failed; 0 ignored; 0 measured; 3058 filtered out; finished in …
error: test failed, to rerun pass `-p tabletist --lib`
```

- [ ] **Step 3: Write the code.** The form's state; the control; both looks; the choices drawn each frame; the save. `env_tests` puts its form in a group: its check of the environment's colour finds the first text painted "Local", which Group would otherwise be.

````diff
diff --git a/crates/tabletist-core/src/model.rs b/crates/tabletist-core/src/model.rs
--- a/crates/tabletist-core/src/model.rs
+++ b/crates/tabletist-core/src/model.rs
@@ -2033,6 +2033,16 @@
     /// `Some` when editing a saved connection, `None` for a new one.
     pub editing: Option<ConnectionId>,
     pub name: String,
+    /// The group chosen in Group: `None` while the connection goes by the
+    /// Connections list's rule.
+    pub group: Option<String>,
+    /// The name being typed after New group…, while its field is open.
+    pub group_typed: Option<String>,
+    /// Put the keyboard in Group on the next frame: in its field once New
+    /// group… opens it, on the dropdown once Escape closes the field.
+    pub focus_group: bool,
+    /// What Group offers, read again on each frame the dialog draws.
+    pub group_choices: crate::connections::groups::Choices,
     /// The environment the user chose; `None` until they do, while it
     /// follows where the connection points (see
     /// [`ConnectionForm::environment`]).
@@ -2276,6 +2286,10 @@
         Self {
             editing: None,
             name: String::new(),
+            group: None,
+            group_typed: None,
+            focus_group: false,
+            group_choices: Default::default(),
             environment: None,
             read_only: None,
             production_read_only: true,
@@ -2527,6 +2541,10 @@
         Self {
             editing: Some(saved.id.clone()),
             name: saved.name.clone(),
+            group: saved.group.clone(),
+            group_typed: None,
+            focus_group: false,
+            group_choices: Default::default(),
             environment: Some(saved.environment),
             read_only: saved.read_only,
             production_read_only: true,
@@ -2913,7 +2931,7 @@
         Ok(crate::connections::SavedConnection {
             id: self.editing.clone().unwrap_or_else(ConnectionId::new),
             name: name.to_owned(),
-            group: None,
+            group: self.chosen_group(),
             environment: self.environment(),
             read_only: self.saved_read_only(),
             password,
@@ -2921,6 +2939,24 @@
             spec,
             agents,
         })
+    }
+
+    /// The group to save: the one typed after New group…, while its field
+    /// is open, else the one chosen; none where it is blank.
+    pub fn chosen_group(&self) -> Option<String> {
+        self.group_typed
+            .as_deref()
+            .or(self.group.as_deref())
+            .map(str::trim)
+            .filter(|group| !group.is_empty())
+            .map(str::to_owned)
+    }
+
+    /// Whether the connection stays on this machine, as the fields say: a
+    /// file, or a local host without a tunnel. Without a group of its own
+    /// it is then listed under Local.
+    pub fn stays_local(&self) -> bool {
+        crate::connections::groups::stays_on_this_machine(self.driver, &self.host, self.ssh)
     }
 
     /// Takes into the Tables control a name typed and not yet added, as a
diff --git a/src/app.rs b/src/app.rs
--- a/src/app.rs
+++ b/src/app.rs
@@ -6401,6 +6401,9 @@
                 }
             }
         }
+        // A group that is the one the rule gives it anyway is none: the
+        // connection then follows the rule, as its name or server change.
+        saved.group = crate::connections::groups::keep(&saved, &self.connections.connections);
         self.connections.upsert(saved.clone());
         self.save_connections();
         self.rename_launch(&saved);
diff --git a/src/ui/connect_dialog/agents.rs b/src/ui/connect_dialog/agents.rs
--- a/src/ui/connect_dialog/agents.rs
+++ b/src/ui/connect_dialog/agents.rs
@@ -606,7 +606,7 @@
 /// Whether the form's Cancel, as the keymap has it, was pressed while
 /// `field` has the keyboard or has just let it go: the field's own, taken
 /// out of the frame so that the dialog is not closed by it.
-fn cancel_asked(ui: &Ui, field: &egui::Response, skin: &Skin) -> bool {
+pub(super) fn cancel_asked(ui: &Ui, field: &egui::Response, skin: &Skin) -> bool {
     (field.has_focus() || field.lost_focus()) && {
         let keymap = crate::ui::keys::published(ui.ctx());
         let layout = crate::keymap::Layout::of(skin.look);
diff --git a/src/ui/connect_dialog/group.rs b/src/ui/connect_dialog/group.rs
new file mode 100644
--- /dev/null
+++ b/src/ui/connect_dialog/group.rs
@@ -0,0 +1,89 @@
+//! The connection's Group: a dropdown of the groups it can be put in, the
+//! Connections list's, with New group… under them, which turns it into a
+//! field where a new one is named. Escape there goes back to the dropdown.
+
+use egui::{Response, Ui, WidgetInfo, WidgetType, vec2};
+
+use super::{Skin, input};
+use crate::connections::groups;
+use crate::i18n::msgid;
+use crate::model::ConnectionForm;
+use crate::typography::TextRole;
+use crate::ui::menu;
+
+/// Draws Group `width` wide, as tall as Name beside or above it. `name`
+/// is what screen readers call it; the field the caller labels.
+pub(super) fn control(
+    ui: &mut Ui,
+    form: &mut ConnectionForm,
+    name: &str,
+    width: f32,
+    skin: &Skin,
+) -> Response {
+    let Skin { look, palette, .. } = *skin;
+    let focus = std::mem::take(&mut form.focus_group);
+    // As Name is: 34 on macOS, the terminal's fields' 30.
+    let role = TextRole::pick(look, TextRole::UiBody, TextRole::OBody);
+    let height = if look.terminal {
+        skin.field_height()
+    } else {
+        34.0
+    };
+    if let Some(typed) = form.group_typed.as_mut() {
+        let placeholder = super::hint(ui, &skin.say(msgid("Group name")), role, skin);
+        let field = ui.add_sized(
+            vec2(width, height),
+            input(ui, typed, role, height, skin)
+                .id(ui.id().with("group-typed"))
+                .hint_text(placeholder),
+        );
+        if focus {
+            field.request_focus();
+        }
+        if super::agents::cancel_asked(ui, &field, skin) {
+            field.surrender_focus();
+            form.group_typed = None;
+            form.focus_group = true;
+        }
+        return field;
+    }
+    let choices = &form.group_choices;
+    let shown = choices.shown(form.group.as_deref()).to_owned();
+    let titles = choices.titles.clone();
+    let automatic = choices.automatic.clone();
+    let mut chosen = shown.clone();
+    let mut new = false;
+    let dropdown = menu::Dropdown::new("group", &shown, width)
+        .height(height)
+        .fill(ui.visuals().extreme_bg_color);
+    let response = dropdown
+        .show(ui, look, palette, |ui| {
+            for title in &titles {
+                menu::value(ui, &mut chosen, title.clone(), title, look, palette);
+            }
+            menu::separator(ui, palette);
+            let more = skin.say(msgid("New group…"));
+            if menu::Item::action(&more).show(ui, look, palette).clicked() {
+                new = true;
+                ui.close();
+            }
+        })
+        .response;
+    response.widget_info(|| {
+        let mut info = WidgetInfo::labeled(WidgetType::ComboBox, true, name);
+        info.current_text_value = Some(shown.clone());
+        info
+    });
+    if focus {
+        response.request_focus();
+    }
+    if new {
+        form.group_typed = Some(String::new());
+        form.focus_group = true;
+    } else if chosen != shown {
+        // The rule's own group is no group of the connection's: it then
+        // follows the rule.
+        form.group = (!groups::same(&chosen, &automatic)).then_some(chosen);
+    }
+    response
+}
diff --git a/src/ui/connect_dialog/mod.rs b/src/ui/connect_dialog/mod.rs
--- a/src/ui/connect_dialog/mod.rs
+++ b/src/ui/connect_dialog/mod.rs
@@ -6,6 +6,7 @@
 mod agents;
 mod choice;
 mod files;
+mod group;
 mod reading;
 mod sheet;
 mod terminal;
@@ -290,13 +291,15 @@
 /// `loaded` is what an open window of the connection the form edits has
 /// loaded, read again on each frame, as a sidebar loads: `None` where it
 /// lists no names ([`lists_names`]), is a new connection, or no window of
-/// it is open.
+/// it is open. `groups` is what Group offers, read again on each frame
+/// too.
 pub fn show(
     ctx: &egui::Context,
     paint: Paint,
     form: &mut ConnectionForm,
     keymap: &Keymap,
     loaded: Option<LoadedTables>,
+    groups: crate::connections::groups::Choices,
 ) -> Vec<Action> {
     let Paint {
         locale,
@@ -335,6 +338,7 @@
         _ => None,
     };
     form.agents_loaded = loaded;
+    form.group_choices = groups;
     let placement: Placement = ctx
         .data(|data| data.get_temp(placement_id()))
         .unwrap_or_default();
diff --git a/src/ui/connect_dialog/sheet.rs b/src/ui/connect_dialog/sheet.rs
--- a/src/ui/connect_dialog/sheet.rs
+++ b/src/ui/connect_dialog/sheet.rs
@@ -322,16 +322,29 @@
         look,
     );
     ui.add_space(18.0);
-    let name = labelled(
-        ui,
-        &skin.say(msgid("Name")),
-        TextRole::FormLabel,
-        skin,
-        |ui| ui.add(input(ui, &mut form.name, TextRole::UiBody, 34.0, skin)),
-    );
-    if focus_name {
-        name.request_focus();
-    }
+    // The name, and beside it the group the Connections list shows it in.
+    let columns = [Column::Fr(1.4), Column::Fr(1.0)];
+    row(ui, 12.0, &columns, |ui, column| match column {
+        0 => {
+            let name = labelled(
+                ui,
+                &skin.say(msgid("Name")),
+                TextRole::FormLabel,
+                skin,
+                |ui| ui.add(input(ui, &mut form.name, TextRole::UiBody, 34.0, skin)),
+            );
+            if focus_name {
+                name.request_focus();
+            }
+        }
+        _ => {
+            let heading = skin.say(msgid("Group"));
+            let width = ui.available_width();
+            labelled(ui, &heading, TextRole::FormLabel, skin, |ui| {
+                super::group::control(ui, form, &heading, width, skin)
+            });
+        }
+    });
     ui.add_space(18.0);
     let heading = skin.say(msgid("Environment"));
     widgets::label(ui, TextRole::FormLabel, &heading, palette.secondary, look);
diff --git a/src/ui/connect_dialog/terminal.rs b/src/ui/connect_dialog/terminal.rs
--- a/src/ui/connect_dialog/terminal.rs
+++ b/src/ui/connect_dialog/terminal.rs
@@ -341,6 +341,11 @@
                     name.request_focus();
                 }
             });
+            let heading = skin.say(msgid("Group"));
+            terminal_row(ui, &heading, skin, |ui, label| {
+                let width = ui.available_width();
+                super::group::control(ui, form, &heading, width, skin).labelled_by(label);
+            });
             let heading = skin.say(msgid("Environment"));
             terminal_row_of(ui, &heading, Group::Buttons.height(), skin, |ui, _| {
                 environment_choice(ui, &heading, skin, actions);
diff --git a/src/ui/env_tests.rs b/src/ui/env_tests.rs
--- a/src/ui/env_tests.rs
+++ b/src/ui/env_tests.rs
@@ -182,6 +182,9 @@
                 // Named, so the header's line about the connection is not
                 // the environment's name alone.
                 form.name = "Bookshop".into();
+                // In a group of its own, so Group does not read Local,
+                // the name of an environment too.
+                form.group = Some("Bookshop".into());
                 form.environment = Some(env);
                 // The box follows the environment: set, it is filled in
                 // each one's colour.
diff --git a/src/ui/mod.rs b/src/ui/mod.rs
--- a/src/ui/mod.rs
+++ b/src/ui/mod.rs
@@ -247,7 +247,14 @@
         _ => None,
     };
     if let Some(Dialog::Connection(form)) = &mut app.dialog {
-        let asked = connect_dialog::show(&ui.ctx().clone(), paint, form, &app.keymap, loaded);
+        let groups = crate::connections::groups::Choices::of(
+            &form.name,
+            form.stays_local(),
+            form.editing.as_ref(),
+            &app.connections.connections,
+        );
+        let asked =
+            connect_dialog::show(&ui.ctx().clone(), paint, form, &app.keymap, loaded, groups);
         app.actions.extend(asked);
     } else {
         connect_dialog::fold(ui.ctx());
````

Then the catalogs. Run `assets/i18n/update.sh`, which adds the three messages to `tabletist.pot` and, untranslated, to each catalog. Fill them in:

| msgid | `ru.po` | `pt-BR.po` |
| --- | --- | --- |
| `Group` | `Группа` | `Grupo` |
| `New group…` | `Новая группа…` | `Novo grupo…` |
| `Group name` | `Название группы` | `Nome do grupo` |

Run `assets/i18n/update.sh` again: it says `2382 translated messages.` for each catalog, and `assets/i18n/update.sh --check` passes.

- [ ] **Step 4: Run the checks.** All four pass: 5,544 tests pass across the workspace.

- [ ] **Step 5: Look at it.** Render the dialog's screens: `cargo test --locked --features shots --lib shots::mock::mock -- --ignored --exact`, then read `target/shots/mock-macos-connection-edit.png` and `target/shots/mock-omarchy-connection-edit.png`. macOS: Group beside Name, reading Bookshop, as **macOS – Edit connection, read-only switch** draws it. Omarchy: a `group` row under `name` reading Bookshop with `▾`, as **Omarchy – Edit connection, read-only switch** draws it. Nothing is committed from `target/`.

- [ ] **Step 6: Commit.** "Put a connection in a group from its form: the list's groups, or a new one"

### Task 4: The website: the picker's groups, Group, and the `group` key

**Files:**
- Modify: `docs/_reference/connections.md`, `docs/_reference/settings-and-files.md`, `docs/_guide/omarchy.md` (the form's keys: group's list, and Esc in new group…'s field)
- Re-render, on Igor's yes: `assets/screenshots/macos-connection-edit.webp`, `assets/screenshots/omarchy-connection-edit.webp`

- [ ] **Step 1: The pages.**

````diff
diff --git a/docs/_guide/omarchy.md b/docs/_guide/omarchy.md
--- a/docs/_guide/omarchy.md
+++ b/docs/_guide/omarchy.md
@@ -101,12 +101,12 @@
 | Ctrl+Enter | Save and connect |
 | Backspace, Delete | In the empty password field: forget the saved password |
 | Up, Down | In add table…'s or add column…'s list: move. Enter adds the one chosen, or in add column…'s puts a table chosen in the field for its columns, and Esc closes the field. In a Files connection's list of files: move |
-| Enter, Space | Choose the choice the cursor is on: a type, an environment, folder or files. On a closed section's heading (a Files connection's reading csv or agents): open it, and on an open one, close it |
+| Enter, Space | Choose the choice the cursor is on: a type, an environment, folder or files. On a closed section's heading (a Files connection's reading csv or agents): open it, and on an open one, close it. On group: open its list of groups, where Up and Down move and Enter picks; new group… turns it into a field for a new group's name |
 | Ctrl+O | A Files connection: pick its folder, or add files to its list, in the file chooser |
 | `a` | A Files connection's list: add files |
 | `x` | A Files connection's list: take the file under the cursor off the list. The file stays on disk |
 | `R` | A Files connection's reading csv: take the override the cursor is on off. Other overrides still apply |
-| Esc | Cancel |
+| Esc | Cancel. In new group…'s field: back to the list of groups |
 
 {% include shot.html file="omarchy-connection-edit" alt="The connection form in the Omarchy look, for a production database behind an SSH tunnel" width="2200" height="2032" %}
 
diff --git a/docs/_reference/connections.md b/docs/_reference/connections.md
--- a/docs/_reference/connections.md
+++ b/docs/_reference/connections.md
@@ -9,13 +9,18 @@
 Cmd/Ctrl+O (Ctrl+Shift+C in the Omarchy look) opens the picker with your
 saved connections. Each row shows
 the connection's environment and when it was last used, and
-**Find connection…** searches by name or by `user@host:port/database`.
+**Find connection…** searches by name, by the group you gave it, or by
+`user@host:port/database`.
 
 {% include shot.html file="macos-connections" alt="The connection picker: three connections named Bookshop grouped under that name, and two local ones" %}
 
-Connections with exactly the same name form a group under that name, which
-suits one database in several environments. The rest are listed under
-**Local** or **Remote**.
+A connection you put in a group, with **Group** in its form, is listed
+under that group: production and staging, one group per product, or
+whatever suits you. Of the others, those with exactly the same name form
+a group under that name, which suits one database in several
+environments, and the rest are listed under **Local** or **Remote**.
+Groups are listed by name, with Local and Remote last, and a group
+written in another case is the same group.
 
 A connection can be open more than once. Enter shows a connection that is
 already open, and Shift+Enter opens it again.
@@ -37,6 +42,7 @@
 | --- | --- |
 | **Name** | Left empty, it becomes `user@host:port/database`, the file's name or the BigQuery project |
 | **Type** | SQLite, PostgreSQL, MySQL, BigQuery or Files, as radio buttons in rows above the name. In the Omarchy look each type is a Tab stop and Enter or Space picks it; in the other looks the type is one Tab stop and the arrow keys choose. MySQL is also for [MariaDB](#mariadb). Changing it keeps the name, the environment and whatever was typed for the other types until the dialog closes. Saving a connection you have used under another type asks first |
+| **Group** | The group the picker lists it under: one the picker shows, or **New group…**, which turns the box into a field for a new group's name (Esc goes back). A connection put in the group it would be in anyway, the name it shares, Local or Remote, has none of its own, and follows its name and server. See [The picker](#the-picker) |
 | **Environment** | local, dev, staging, production or none. Sets the connection's color |
 | **File** | SQLite only |
 | **Project**, **Key file** | BigQuery only. See [BigQuery](#bigquery) |
diff --git a/docs/_reference/settings-and-files.md b/docs/_reference/settings-and-files.md
--- a/docs/_reference/settings-and-files.md
+++ b/docs/_reference/settings-and-files.md
@@ -540,6 +540,7 @@
 | --- | --- | --- |
 | `id` | Any text that no other connection has | Tabletist gives the connection one at the next start |
 | `name` | Text | Required |
+| `group` | Text: the group the picker lists the connection under. A file that holds one says `version = 9` | The picker's own: a name the connection shares with another that has no group (or that name as its group), else Local or Remote |
 | `environment` | `local`, `dev`, `staging`, `production`, `none` | `none` |
 | `read_only` | `true`, `false` | As the environment has it: read-only on production |
 | `password` | `none`, `keyring`, `ask` | `none` |
````

- [ ] **Step 2: Check the site.** `cd docs && bundle install && bin/check`. The core's `the_websites_example_is_read_and_written_as_it_shows` still passes: the example has no group.

- [ ] **Step 3: The two pictures of the form.** They show the form without Group. Render them again by the website's recipe: add, to `mod mock` of `src/shots.rs`, right before `fn mock()`, a throwaway test that is never committed:

```rust
    #[test]
    #[ignore = "throwaway"]
    fn website() {
        for (screen, size, name) in [
            (Screen::MacDialog, egui::vec2(1440.0, 900.0), "site-l-macos-connection-edit.png"),
            (Screen::OmarchyDialog, egui::vec2(1100.0, 1016.0), "site-l-omarchy-connection-edit.png"),
        ] {
            let mut harness = Harness::for_shots(size, 2.0, true, screen.look());
            harness.app.palette = screen.palette();
            harness.app.apply_style(&harness.ctx);
            screen.stage(&mut harness);
            harness.finish_animations();
            harness.shot(&out(name));
        }
    }
```

Run `cargo test --locked --features shots --lib shots::mock::website -- --ignored --exact`, then `git checkout -- src/shots.rs`. Then, for each look:

```bash
magick target/shots/site-l-macos-connection-edit.png -strip -define webp:lossless=true -define webp:method=6 assets/screenshots/macos-connection-edit.webp
magick target/shots/site-l-omarchy-connection-edit.png -strip -define webp:lossless=true -define webp:method=6 assets/screenshots/omarchy-connection-edit.webp
```

Each new picture has the old one's size (`magick identify`), shows the Bookshop data only, and Group in it. Show both to Igor. Commit them only on his yes; without it, leave them out and say so in the pull request.

- [ ] **Step 4: Commit.** "Say on the website how connections are grouped, and how to give one a group"

---

## After the tasks

- The pull request says what was built, the decisions table above, and that it was checked on Linux only. No `cfg` code is touched, but nothing was compiled for macOS or Windows here: CI's macOS and Windows jobs are the first to.
- The canvas already draws this run. Runs 2 and 3 update their boards with their code, as Igor asked: the design and the code stay in sync.
