# MCP server, Phase 3a, run 3: the masked columns' setting. Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A connection's `[connections.agents]` table can name the columns whose values agents get as `•••` (`masked = ["orders.email"]`). The app passes them to the server, which enforces them since runs 1a to 2. A change made outside the app that masks less is held until the person keeps it or puts it back, as a table hidden less is. The website's reference describes the key.

**Architecture:** `Agents` (`src/connections.rs`) gains `masked: Vec<ColumnName>`. `src/connections/file.rs` reads and writes the list through `ColumnName`'s `FromStr` and `Display`, and writes version 6 where a connection masks a column. `src/agents_shown.rs` keeps each connection's masked columns as last shown; `Shown::compare` holds a change that masks less (`Raised::Masked`, decided by `masks_no_less`) and takes one that masks more. `App::agents_masked` gives what is in force, which `send_agents_policy` sends, a copy takes and **Put back** writes back. The backend's `Agentable` carries them to the gateway's `Listed`.

**Tech Stack:** Rust 2024, `serde` and `toml` for the file, `serde_json` for `agents_shown.json`.

---

## Where this sits

Phase 3a of `docs/superpowers/plans/2026-10-13-mcp-server.md`, masked columns, in five runs. Runs 1a, 1b and 2 (pull requests 150, 152 and 153) built the checks and the server's side, with the app passing no masked column. This run, 3, is the setting, as 1c-2b was for hidden tables. Run 4 is the editor's Masked columns control and the website's guide: until then the file is the only way to set it.

## What the plan asks, and what is built

| The plan | Built | Why |
|---|---|---|
| C.1: `"masked": ["orders.email", "orders.phone"]` in a connection's settings for agents | `masked = [...]` in `[connections.agents]`, as `hidden` and `only` are | The connections moved to `connections.toml` (pull request 125) |
| C.2: a change made outside the app that lets agents do more is held | One that masks less is held (`Raised::Masked`), with the sentence a table's has: "agents may read more of …" | The same answer to the person: agents may read what they could not |
| Not in the plan | While one is held, what was shown is in force with what the file says now | A column the change also masks is masked at once, as a change that masks more is taken |
| Not in the plan | The file is version 6 where a column is masked | A Tabletist of version 5 would refuse the key, and one of an older one never reads the file |

## Before you start

- Work on a branch of `main`. One commit for each task, in the order below. Every commit passes `cargo fmt --all --check`, `cargo clippy --locked --workspace --all-targets -- -D warnings` and `cargo test --locked --workspace --all-targets`.
- On Igor's machine `cargo` is `~/.cargo/bin/cargo` (the mise shim fails), and a build in a worktree needs `CARGO_TARGET_DIR` set to that worktree's `target/`.
- Read `AGENTS.md`. The Agents part shows only where agents can reach the app (`cfg(any(unix, windows))`). The diffs below apply with `git apply`. Run `cargo fmt --all` before each commit: the code here is already formatted.

## What was run before this plan was written

Every task's state was built here, passed `cargo fmt --check` and clippy with `-D warnings`, and its tests failed and then passed as each step says. The last state passed `cargo test --locked --workspace --all-targets` with both test servers (3,570 tests, none failed), `RUSTDOCFLAGS='-D warnings' cargo doc --locked --workspace --no-deps` and `docs/bin/check`. Not run: CI.

## What this run settles

| Question | Decision | Why |
|---|---|---|
| Where the setting is | `masked = [...]` in a connection's `[connections.agents]` table | Beside `hidden` and `only` |
| The file's version | 6 where a connection masks a column | A Tabletist that would answer a masked column's values as they are never reads the file |
| What a change made outside that masks less is | An entry taken out, or one for every schema made one schema's | An entry without a schema is that column in every schema |
| What is in force while one is held | What was shown with what the file says now | A column the change also masks is masked at once, as a change that masks more is taken |
| A shown entry that does not read | Every change against it is held, and the connection is Off to agents meanwhile; **Put back** writes the entries that read, and they are what was shown from then on | What it masked cannot be said, so it cannot be kept masked (found by the review) |
| What the held bar says | "agents may read more of …", as for a table, and a connection once | One answer to the person: agents may read what they could not (found by the review) |
| The editor | Carries them as they are; its save keeps what the file says | Its control is run 4's |

### What this run leaves out

- The editor's Masked columns control, its **Add column…** and its mark, and the website's guide: run 4. Until then the editor carries a connection's masked columns as they are, and its save keeps what the file says.

## File structure

| File | What changes |
|---|---|
| `src/connections.rs` | `Agents::masked` |
| `src/connections/file.rs` | `masked = [...]`, read and written; version 6 |
| `src/agents_shown.rs` | `Shown::masked`, `masked_of`, `Raised::Masked`, `masks_no_less` |
| `src/model.rs` | `ConnectionForm::agents_masked`, carried to the save |
| `src/app.rs` | `App::agents_masked`; the editor, a copy and **Put back**; the policy |
| `src/ui/mod.rs` | The held bar's sentence for a masked change |
| `src/backend/agents.rs` | `Agentable::masked`, to the gateway's `Listed` |
| `docs/_reference/settings-and-files.md` | The key, the version, and what is held |
| `docs/superpowers/plans/2026-10-13-mcp-server.md` | B.4 as built for 3a-3; G |

---

### Task 1: The setting, read, written and held

- `Agents::masked`: the columns whose values agents get masked; none to start with.
- `connections.toml`: `masked = ["orders.email", "sales.orders.email"]` in `[connections.agents]`, each entry read by `ColumnName`'s `FromStr`, an entry that does not read refused by its place and never by its words, and written back as the person writes it. A file with a masked column is version 6; one that hides a table and masks none stays 5.
- `agents_shown.json` keeps each connection's masked columns as last shown (`Shown::masked`, read by `masked_of`). `Shown::compare` holds a change that masks less, an entry taken out or one for every schema made one schema's (`Raised::Masked`, decided by `masks_no_less`), and takes one that masks more; the same in another case or order is no change. A shown entry that does not read holds every change.
- `App::agents_masked`: what is in force, its own, and where a change that masks less is held what was shown with what the file says. While a change is held against a shown entry that does not read, the connection is Off to agents. A copy takes what is in force; **Put back** writes what was shown back and takes it for what was shown; **Keep** takes the file's. The editor carries a connection's masked columns as they are (`ConnectionForm::agents_masked`) and its save keeps what the file says. The held bar says "agents may read more of …", once for a connection that hides and masks less.

**Files:**
- Modify: `src/connections.rs`, `src/connections/file.rs`, `src/agents_shown.rs`, `src/model.rs`, `src/app.rs`, `src/ui/mod.rs`

- [ ] **Step 1: Write the failing test**

````diff
diff --git a/src/agents_shown.rs b/src/agents_shown.rs
index a6965c8..58f3831 100644
--- a/src/agents_shown.rs
+++ b/src/agents_shown.rs
@@ -388,6 +388,73 @@ mod tests {
         );
     }
 
+    fn masking(id: &str, masked: &[&str]) -> SavedConnection {
+        let mut saved = saved(id, AgentAccess::Read);
+        saved.agents.masked = masked.iter().map(|entry| entry.parse().unwrap()).collect();
+        saved
+    }
+
+    #[test]
+    fn a_change_that_masks_less_is_held_and_one_that_masks_more_is_taken() {
+        let id = || ConnectionId("shop".into());
+        let mut shown = Shown::default();
+        shown.keep(&[Raised::Access {
+            id: id(),
+            to: AgentAccess::Read,
+        }]);
+        // More, from the file: taken at once, and remembered.
+        let (raised, lowered) = shown.compare(false, &[masking("shop", &["orders.email"])]);
+        assert_eq!(raised, []);
+        assert!(lowered);
+        assert_eq!(shown.masked["shop"], ["orders.email"]);
+        // The same in another case is no change.
+        let (raised, lowered) = shown.compare(false, &[masking("shop", &["Orders.EMAIL"])]);
+        assert_eq!((raised, lowered), (Vec::new(), false));
+        // Another entry: more.
+        let (raised, _) = shown.compare(false, &[masking("shop", &["orders.email", "phone.x"])]);
+        assert_eq!(raised, []);
+        // Less: held, with what the file says now.
+        let (raised, lowered) = shown.compare(false, &[masking("shop", &["phone.x"])]);
+        assert_eq!(
+            raised,
+            [Raised::Masked {
+                id: id(),
+                to: vec!["phone.x".parse().unwrap()],
+            }]
+        );
+        assert!(!lowered);
+        // So is a schema's entry in place of one for every schema.
+        shown.keep(&[Raised::Masked {
+            id: id(),
+            to: vec!["orders.email".parse().unwrap()],
+        }]);
+        let (raised, _) = shown.compare(false, &[masking("shop", &["sales.orders.email"])]);
+        assert_eq!(raised.len(), 1);
+        // Kept: what was shown is the file's.
+        shown.keep(&raised);
+        assert_eq!(shown.masked["shop"], ["sales.orders.email"]);
+        assert_eq!(shown.compare(false, &[masking("shop", &[])]).0.len(), 1);
+    }
+
+    #[test]
+    fn a_shown_mask_that_does_not_read_holds_every_change() {
+        let mut shown = Shown::default();
+        shown.keep(&[Raised::Access {
+            id: ConnectionId("shop".into()),
+            to: AgentAccess::Read,
+        }]);
+        shown
+            .masked
+            .insert("shop".into(), vec!["orders.email".into(), "email".into()]);
+        let was = shown.masked_of(&ConnectionId("shop".into()));
+        assert_eq!(was.columns, vec!["orders.email".parse().unwrap()]);
+        assert!(was.unread);
+        // Even the readable entry again, with more.
+        let (raised, _) =
+            shown.compare(false, &[masking("shop", &["orders.email", "orders.phone"])]);
+        assert_eq!(raised.len(), 1);
+    }
+
     #[test]
     fn a_shown_list_that_does_not_read_hides_every_table() {
         let mut shown = Shown::default();
diff --git a/src/app.rs b/src/app.rs
index d2e9268..a4bf1cc 100644
--- a/src/app.rs
+++ b/src/app.rs
@@ -7183,6 +7183,144 @@ mod tests {
         );
     }
 
+    #[cfg(any(unix, windows))]
+    #[test]
+    fn a_change_outside_that_masks_less_is_held_and_the_editor_keeps_the_files_masks() {
+        use crate::agents_shown::Raised;
+        use crate::connections::AgentAccess;
+        use tabletist_db::agent::ColumnName;
+        let (mut app, _dir) = app();
+        let conn = with_saved(&mut app);
+        app.serve_agents();
+        app.apply(Action::SetAgentAccess {
+            conn: conn.clone(),
+            access: AgentAccess::Read,
+        });
+        let columns = |written: &[&str]| -> Vec<ColumnName> {
+            written.iter().map(|entry| entry.parse().unwrap()).collect()
+        };
+        // What is in force, which the policy sends.
+        let sent = |app: &App| app.agents_masked(app.connections.get(&conn).unwrap());
+        let file_says = |app: &mut App, masked: Vec<ColumnName>| {
+            let mut saved = app.connections.get(&conn).unwrap().clone();
+            saved.agents.masked = masked;
+            app.connections.upsert(saved);
+            app.check_agents();
+            app.send_agents_policy();
+        };
+        let email = columns(&["orders.email"]);
+        // Masking more, from the file: taken at once.
+        file_says(&mut app, email.clone());
+        assert!(app.agents_held.is_empty());
+        assert_eq!(sent(&app), email);
+        // Masking less: held, and what was shown is in force with what the
+        // file says now.
+        file_says(&mut app, columns(&["orders.phone"]));
+        assert!(matches!(
+            app.agents_held.as_slice(),
+            [Raised::Masked { .. }]
+        ));
+        assert_eq!(sent(&app), columns(&["orders.phone", "orders.email"]));
+        let said = crate::ui::agents_held_said(&app).unwrap();
+        assert!(
+            said.contains("agents may read more of Local · dev"),
+            "{said}"
+        );
+        // A copy made meanwhile masks what is in force.
+        app.apply(Action::DuplicateConnection(conn.clone()));
+        let copy = &app.connections.connections[1];
+        assert_eq!(
+            copy.agents.masked,
+            columns(&["orders.phone", "orders.email"])
+        );
+        let copy = copy.id.clone();
+        app.apply(Action::DeleteConnection(copy));
+        // Put back: the file masks what was shown again.
+        app.apply(Action::PutBackAgentChanges(app.agents_held.clone()));
+        assert!(app.agents_held.is_empty());
+        assert_eq!(app.connections.get(&conn).unwrap().agents.masked, email);
+        // Kept: in force.
+        file_says(&mut app, Vec::new());
+        app.apply(Action::KeepAgentChanges(app.agents_held.clone()));
+        assert!(app.agents_held.is_empty());
+        assert_eq!(sent(&app), Vec::<ColumnName>::new());
+        // The editor has no control for them: its save keeps what the file
+        // says when it saves.
+        app.apply(Action::EditConnection(conn.clone()));
+        file_says(&mut app, email.clone());
+        form(&mut app).name = "Local shop".into();
+        app.apply(Action::SaveConnection { connect: false });
+        let saved = app.connections.get(&conn).unwrap();
+        assert_eq!(
+            (saved.name.as_str(), &saved.agents.masked),
+            ("Local shop", &email)
+        );
+    }
+
+    #[cfg(any(unix, windows))]
+    #[test]
+    fn a_connection_that_hides_and_masks_less_is_said_once() {
+        use crate::connections::AgentAccess;
+        use tabletist_db::agent::Tables;
+        let (mut app, _dir) = app();
+        let conn = with_saved(&mut app);
+        app.serve_agents();
+        app.apply(Action::SetAgentAccess {
+            conn: conn.clone(),
+            access: AgentAccess::Read,
+        });
+        let file_says = |app: &mut App, tables: Tables, masked: &[&str]| {
+            let mut saved = app.connections.get(&conn).unwrap().clone();
+            saved.agents.tables = tables;
+            saved.agents.masked = masked.iter().map(|entry| entry.parse().unwrap()).collect();
+            app.connections.upsert(saved);
+            app.check_agents();
+        };
+        file_says(
+            &mut app,
+            Tables::AllExcept(vec!["customers".parse().unwrap()]),
+            &["orders.email"],
+        );
+        file_says(&mut app, Tables::All, &[]);
+        assert_eq!(app.agents_held.len(), 2);
+        let said = crate::ui::agents_held_said(&app).unwrap();
+        assert_eq!(said.matches("Local · dev").count(), 1, "{said}");
+    }
+
+    #[cfg(any(unix, windows))]
+    #[test]
+    fn a_held_change_against_a_shown_mask_that_does_not_read_is_off_until_answered() {
+        use crate::agents_shown::Raised;
+        use crate::connections::AgentAccess;
+        let (mut app, _dir) = app();
+        let conn = with_saved(&mut app);
+        app.serve_agents();
+        app.apply(Action::SetAgentAccess {
+            conn: conn.clone(),
+            access: AgentAccess::Read,
+        });
+        let mut saved = app.connections.get(&conn).unwrap().clone();
+        saved.agents.masked = vec!["orders.email".parse().unwrap()];
+        app.connections.upsert(saved);
+        app.check_agents();
+        // A hand writes an entry that does not read into what was shown.
+        app.agents_shown
+            .masked
+            .insert(conn.0.clone(), vec!["orders.email".into(), "bad".into()]);
+        app.check_agents();
+        assert!(matches!(
+            app.agents_held.as_slice(),
+            [Raised::Masked { .. }]
+        ));
+        let saved = app.connections.get(&conn).unwrap();
+        assert_eq!(app.agents_access(saved), AgentAccess::Off);
+        // Put back ends it: what was shown is what was written.
+        app.apply(Action::PutBackAgentChanges(app.agents_held.clone()));
+        assert!(app.agents_held.is_empty(), "{:?}", app.agents_held);
+        let saved = app.connections.get(&conn).unwrap();
+        assert_eq!(app.agents_access(saved), AgentAccess::Read);
+    }
+
     #[test]
     fn agents_are_told_of_each_change_to_what_they_may_use() {
         let (mut app, _dir) = app();
diff --git a/src/connections/file.rs b/src/connections/file.rs
index 737fa43..4be0b18 100644
--- a/src/connections/file.rs
+++ b/src/connections/file.rs
@@ -903,8 +903,8 @@ auth = "key-file"
         let said = |text: &str| read(text).unwrap_err().to_string();
         assert!(said("").contains("missing field `version`"), "{}", said(""));
         assert_eq!(
-            said("version = 6\n"),
-            "it is version 6 of the file, written by a newer Tabletist; this one reads version 5"
+            said("version = 7\n"),
+            "it is version 7 of the file, written by a newer Tabletist; this one reads version 6"
         );
         assert_eq!(
             said("version = 2\n"),
@@ -1055,6 +1055,54 @@ hidden       = ["customers", "\"odd.schema\".orders"]
         assert!(write(&connections).unwrap().contains("\nversion = 4\n"));
     }
 
+    #[test]
+    fn the_columns_agents_get_masked_make_the_file_version_six() {
+        let mut connections = with_agents(AgentAccess::Read, 500, 10);
+        connections[2].agents.masked = vec![
+            "orders.email".parse().unwrap(),
+            "\"odd.schema\".orders.phone".parse().unwrap(),
+        ];
+        let text = write(&connections).unwrap();
+        assert!(text.contains("\nversion = 6\n"), "{text}");
+        assert!(
+            text.ends_with(
+                r#"timeout_secs = 10
+masked       = ["orders.email", "\"odd.schema\".orders.phone"]
+"#
+            ),
+            "{text}"
+        );
+        assert_eq!(read(&text).unwrap().connections, connections);
+        // With hidden tables, still version 6; without masked columns, 5.
+        connections[2].agents.tables = Tables::AllExcept(vec!["customers".parse().unwrap()]);
+        let text = write(&connections).unwrap();
+        assert!(text.contains("\nversion = 6\n"), "{text}");
+        assert_eq!(read(&text).unwrap().connections, connections);
+        connections[2].agents.masked = Vec::new();
+        let five = write(&connections).unwrap();
+        assert!(five.contains("\nversion = 5\n"), "{five}");
+        // An empty list masks nothing, and an entry that does not read is
+        // refused by its place, never by its words.
+        let with = |line: &str| {
+            five.replace(
+                "hidden       = [\"customers\"]",
+                &format!("hidden       = [\"customers\"]\n{line}"),
+            )
+        };
+        assert_eq!(read(&with("masked = []")).unwrap().connections, connections);
+        assert_eq!(
+            read(&with("masked = [\"orders.email\", \"email\"]"))
+                .unwrap_err()
+                .to_string(),
+            "connection \"Bookshop on disk\": agents' `masked` entry 2 has no table: write the \
+             table, a dot, and the column"
+        );
+        let secret = read(&with("masked = [\"hunter2\\\"x.y\"]"))
+            .unwrap_err()
+            .to_string();
+        assert!(!secret.contains("hunter2"), "{secret}");
+    }
+
     #[test]
     fn every_connection_has_an_id_of_its_own() {
         let entry = |id: &str, name: &str| {
````

- [ ] **Step 2: Run it to see that it does not build**

Run: `cargo test --locked --lib -- masks_less masked_make_the_file`

Expected: it does not build:

```text
error[E0609]: no field `masked` on type `connections::Agents`
error[E0599]: no variant named `Masked` found for enum `agents_shown::Raised`
error[E0599]: no method named `agents_masked` found for reference `&app::App` in the current scope
error: could not compile `tabletist` (lib test) due to 19 previous errors
```

- [ ] **Step 3: The setting**

````diff
diff --git a/src/agents_shown.rs b/src/agents_shown.rs
index 58f3831..725e5eb 100644
--- a/src/agents_shown.rs
+++ b/src/agents_shown.rs
@@ -1,5 +1,6 @@
 //! What the app last showed the person of what agents may do: whether the
-//! server is on, and each connection's access and the tables it hides. A change made outside the
+//! server is on, and each connection's access, the tables it hides and the
+//! columns it masks. A change made outside the
 //! app (a hand, a program, a synced dotfile, a backup) that lets agents do
 //! more than that is held until the person keeps it or puts it back; one
 //! that lets them do less is taken at once.
@@ -13,7 +14,7 @@ use std::path::Path;
 
 use serde::{Deserialize, Serialize};
 
-use tabletist_db::agent::{TableName, Tables};
+use tabletist_db::agent::{ColumnName, TableName, Tables};
 
 use crate::connections::{AgentAccess, ConnectionId, SavedConnection};
 
@@ -28,6 +29,19 @@ pub struct Shown {
     /// The tables each connection hides, by its id. A connection not here
     /// hid none.
     pub tables: BTreeMap<String, ShownTables>,
+    /// The columns each connection masks, by its id, each entry as the
+    /// person writes it. A connection not here masked none.
+    pub masked: BTreeMap<String, Vec<String>>,
+}
+
+/// The columns a connection was shown to mask.
+#[derive(Debug, Clone, Default, PartialEq, Eq)]
+pub struct ShownMasked {
+    /// The entries that read.
+    pub columns: Vec<ColumnName>,
+    /// Whether an entry did not read, which only a hand could write: what
+    /// it masked cannot be said, so a change against it is held.
+    pub unread: bool,
 }
 
 /// The tables a connection hid, each entry as the person writes it.
@@ -47,6 +61,11 @@ pub enum Raised {
     Access { id: ConnectionId, to: AgentAccess },
     /// A connection hides less: `to`.
     Tables { id: ConnectionId, to: Tables },
+    /// A connection masks less: `to`.
+    Masked {
+        id: ConnectionId,
+        to: Vec<ColumnName>,
+    },
 }
 
 impl Shown {
@@ -87,6 +106,23 @@ impl Shown {
         }
     }
 
+    /// The columns `id` was shown to mask.
+    pub fn masked_of(&self, id: &ConnectionId) -> ShownMasked {
+        let written = self
+            .masked
+            .get(&id.0)
+            .map(Vec::as_slice)
+            .unwrap_or_default();
+        let columns: Vec<ColumnName> = written
+            .iter()
+            .filter_map(|entry| entry.parse().ok())
+            .collect();
+        ShownMasked {
+            unread: columns.len() != written.len(),
+            columns,
+        }
+    }
+
     /// What the files say now, `enabled` and `connections`, against what
     /// was shown: the changes that let agents do more, to be held. What
     /// lets them do less is taken into `self` at once; whether anything
@@ -135,6 +171,22 @@ impl Shown {
                 }),
             }
         }
+        for saved in connections {
+            let (was, now) = (self.masked_of(&saved.id), &saved.agents.masked);
+            if !was.unread && same_columns(now, &was.columns) {
+                continue;
+            }
+            match masks_no_less(now, &was) {
+                true => {
+                    self.set_masked(&saved.id, now);
+                    lowered = true;
+                }
+                false => raised.push(Raised::Masked {
+                    id: saved.id.clone(),
+                    to: now.clone(),
+                }),
+            }
+        }
         // A connection deleted since lets agents do nothing. What it hid is
         // kept: a start that read no connection at all (a file kept aside,
         // or one of a newer version) deleted nothing, and when it comes
@@ -153,6 +205,19 @@ impl Shown {
                 Raised::Server => self.enabled = true,
                 Raised::Access { id, to } => self.set(id, *to),
                 Raised::Tables { id, to } => self.set_tables(id, to),
+                Raised::Masked { id, to } => self.set_masked(id, to),
+            }
+        }
+    }
+
+    fn set_masked(&mut self, id: &ConnectionId, masked: &[ColumnName]) {
+        match masked.is_empty() {
+            true => {
+                self.masked.remove(&id.0);
+            }
+            false => {
+                let written = masked.iter().map(ToString::to_string).collect();
+                self.masked.insert(id.0.clone(), written);
             }
         }
     }
@@ -186,6 +251,50 @@ impl Shown {
     }
 }
 
+/// Whether `a` and `b` mask the same columns, in any order and case.
+fn same_columns(a: &[ColumnName], b: &[ColumnName]) -> bool {
+    let within = |a: &[ColumnName], b: &[ColumnName]| {
+        a.iter()
+            .all(|entry| b.iter().any(|other| same_column(entry, other)))
+    };
+    within(a, b) && within(b, a)
+}
+
+/// Whether two entries are one, without regard to case.
+fn same_column(a: &ColumnName, b: &ColumnName) -> bool {
+    let same = |a: &str, b: &str| a.to_lowercase() == b.to_lowercase();
+    same(&a.table, &b.table)
+        && same(&a.column, &b.column)
+        && match (&a.schema, &b.schema) {
+            (None, None) => true,
+            (Some(a), Some(b)) => same(a, b),
+            _ => false,
+        }
+}
+
+/// Whether agents get masked under `now` every column they did under
+/// `was`: each entry `was` had is covered by one `now` has. An entry
+/// without a schema is that column in every schema, so it covers one with
+/// a schema, and not the other way round. A shown entry that did not read
+/// masked what nobody can say: nothing then masks no less.
+fn masks_no_less(now: &[ColumnName], was: &ShownMasked) -> bool {
+    let same = |a: &str, b: &str| a.to_lowercase() == b.to_lowercase();
+    let covers = |wide: &ColumnName, narrow: &ColumnName| {
+        same(&wide.table, &narrow.table)
+            && same(&wide.column, &narrow.column)
+            && match (&wide.schema, &narrow.schema) {
+                (None, _) => true,
+                (Some(wide), Some(narrow)) => same(wide, narrow),
+                (Some(_), None) => false,
+            }
+    };
+    !was.unread
+        && was
+            .columns
+            .iter()
+            .all(|masked| now.iter().any(|still| covers(still, masked)))
+}
+
 /// `tables`, an empty "All except" list as "All".
 fn normal(tables: Tables) -> Tables {
     match tables {
diff --git a/src/app.rs b/src/app.rs
index a4bf1cc..052442a 100644
--- a/src/app.rs
+++ b/src/app.rs
@@ -2034,6 +2034,7 @@ impl App {
                     let tables = self.agents_tables(saved);
                     form.agents_tables = crate::model::TablesChoice::of(&tables);
                     form.agents_tables_opened = tables;
+                    form.agents_masked = self.agents_masked(saved);
                     self.dialog = Some(Dialog::Connection(Box::new(form)));
                     self.list_ssh_hosts();
                 }
@@ -2078,17 +2079,19 @@ impl App {
                 }
             }
             Action::DuplicateConnection(id) => {
-                // The copy hides what is in force: where a change made
-                // outside that hides less is held, what was shown.
+                // The copy hides and masks what is in force: where a change
+                // made outside that hides or masks less is held, what was
+                // shown.
                 let in_force = self
                     .connections
                     .get(&id)
-                    .map(|saved| self.agents_tables(saved));
+                    .map(|saved| (self.agents_tables(saved), self.agents_masked(saved)));
                 if let Some(copy) = self.connections.duplicate(&id) {
-                    if let (Some(tables), Some(mut saved)) =
+                    if let (Some((tables, masked)), Some(mut saved)) =
                         (in_force, self.connections.get(&copy).cloned())
                     {
                         saved.agents.tables = tables;
+                        saved.agents.masked = masked;
                         self.connections.upsert(saved);
                     }
                     self.save_connections();
@@ -3365,9 +3368,19 @@ impl App {
     }
 
     /// What agents may do on `saved` now: its access, or what was shown
-    /// where a change made outside the app is held.
+    /// where a change made outside the app is held. A change of its masked
+    /// columns held against a shown entry that does not read leaves it Off
+    /// meanwhile: what that entry masked cannot be said, so it cannot be
+    /// kept masked.
     pub fn agents_access(&self, saved: &SavedConnection) -> crate::connections::AgentAccess {
         use crate::agents_shown::Raised;
+        let masked = self
+            .agents_held
+            .iter()
+            .any(|raise| matches!(raise, Raised::Masked { id, .. } if *id == saved.id));
+        if masked && self.agents_shown.masked_of(&saved.id).unread {
+            return crate::connections::AgentAccess::Off;
+        }
         let held = self
             .agents_held
             .iter()
@@ -3392,6 +3405,27 @@ impl App {
         }
     }
 
+    /// The columns agents get masked on `saved` now: its own, and where a
+    /// change made outside the app that masks less is held, what was shown
+    /// with them, so that a column that change also masks is masked at
+    /// once.
+    pub fn agents_masked(&self, saved: &SavedConnection) -> Vec<tabletist_db::agent::ColumnName> {
+        use crate::agents_shown::Raised;
+        let held = self
+            .agents_held
+            .iter()
+            .any(|raise| matches!(raise, Raised::Masked { id, .. } if *id == saved.id));
+        let mut masked = saved.agents.masked.clone();
+        if held {
+            for shown in self.agents_shown.masked_of(&saved.id).columns {
+                if !masked.contains(&shown) {
+                    masked.push(shown);
+                }
+            }
+        }
+        masked
+    }
+
     /// Compares what the files let agents do with what was last shown:
     /// what lets them do more is held, what lets them do less is taken and
     /// remembered. Where agents cannot reach the app nothing is held.
@@ -3458,15 +3492,28 @@ impl App {
         }
         let mut changed = false;
         for raise in back {
-            let (id, access) = match raise {
+            let id = match &raise {
                 Raised::Server => continue,
-                Raised::Access { id, .. } => (id, true),
-                Raised::Tables { id, .. } => (id, false),
+                Raised::Access { id, .. }
+                | Raised::Tables { id, .. }
+                | Raised::Masked { id, .. } => id.clone(),
             };
             if let Some(mut saved) = self.connections.get(&id).cloned() {
-                match access {
-                    true => saved.agents.access = self.agents_shown.access_of(&id),
-                    false => saved.agents.tables = self.agents_shown.tables_of(&id),
+                match raise {
+                    Raised::Server => {}
+                    Raised::Access { .. } => saved.agents.access = self.agents_shown.access_of(&id),
+                    Raised::Tables { .. } => saved.agents.tables = self.agents_shown.tables_of(&id),
+                    // What was shown that does not read cannot be written
+                    // back: the rest is, and is what was shown from now on.
+                    Raised::Masked { .. } => {
+                        let columns = self.agents_shown.masked_of(&id).columns;
+                        saved.agents.masked = columns.clone();
+                        self.agents_shown.keep(&[Raised::Masked {
+                            id: id.clone(),
+                            to: columns,
+                        }]);
+                        self.save_agents_shown();
+                    }
                 }
                 self.connections.upsert(saved);
                 changed = true;
@@ -3770,7 +3817,10 @@ impl App {
         // made outside the app stays held, and one put back stays back.
         // The tables likewise: those the person changed are their own and
         // shown, wider or narrower; the same ones keep what the file says.
+        // The masked columns, which the editor does not change yet, keep
+        // what the file says.
         if let Some(was) = self.connections.get(&saved.id) {
+            saved.agents.masked = was.agents.masked.clone();
             let same = crate::model::TablesChoice::of(&saved.agents.tables)
                 == crate::model::TablesChoice::of(&tables_opened);
             if same {
diff --git a/src/connections.rs b/src/connections.rs
index 0c7e6f4..a7469d8 100644
--- a/src/connections.rs
+++ b/src/connections.rs
@@ -70,6 +70,9 @@ pub struct Agents {
     /// The tables agents may not read: all may be read, all but a list,
     /// or only a list. An "All except" list that is empty is "All".
     pub tables: tabletist_db::agent::Tables,
+    /// The columns whose values agents get as `•••`, and may select but
+    /// not filter, join, sort or compute on.
+    pub masked: Vec<tabletist_db::agent::ColumnName>,
 }
 
 /// The most rows an agent's answer may be given: half of what the SQL
@@ -86,6 +89,7 @@ impl Default for Agents {
             rows: 500,
             timeout_secs: 10,
             tables: tabletist_db::agent::Tables::All,
+            masked: Vec::new(),
         }
     }
 }
diff --git a/src/connections/file.rs b/src/connections/file.rs
index 4be0b18..5194ee3 100644
--- a/src/connections/file.rs
+++ b/src/connections/file.rs
@@ -11,7 +11,7 @@ use std::collections::HashSet;
 use std::path::{Path, PathBuf};
 
 use serde::Deserialize;
-use tabletist_db::agent::{TableName, Tables};
+use tabletist_db::agent::{ColumnName, TableName, Tables};
 use tabletist_db::{ConnectSpec, Driver, SshAuth, SshSpec, TlsMode};
 
 use super::{
@@ -22,12 +22,16 @@ use crate::env::Environment;
 use crate::util::toml_string;
 
 /// The file's format. 3 is the first that is TOML: 1 and 2 were
-/// `connections.json`. 4 has a connection's settings for agents, and 5 the
-/// tables they may not read. A file is written as the oldest that holds
-/// what it has: a Tabletist that does not know a key refuses only a file it
-/// would lose it from, and one that would read agents' tables as all of
-/// them never reads the file.
-pub(super) const VERSION: u32 = 5;
+/// `connections.json`. 4 has a connection's settings for agents, 5 the
+/// tables they may not read, and 6 the columns they get masked. A file is
+/// written as the oldest that holds what it has: a Tabletist that does not
+/// know a key refuses only a file it would lose it from, and one that
+/// would read agents' tables as all of them, or a masked column's values
+/// as they are, never reads the file.
+pub(super) const VERSION: u32 = 6;
+
+/// The version that has agents' tables and not their masked columns.
+const TABLES_VERSION: u32 = 5;
 
 /// The version that has agents' settings and not their tables.
 const AGENTS_VERSION: u32 = 4;
@@ -124,6 +128,8 @@ struct AgentsTable {
     hidden: Option<Vec<String>>,
     /// The tables agents may read, and no others.
     only: Option<Vec<String>>,
+    /// The columns whose values agents get masked.
+    masked: Option<Vec<String>>,
 }
 
 #[derive(Deserialize)]
@@ -258,18 +264,22 @@ impl AgentsTable {
         let starts = Agents::default();
         // An entry by its place in the list: a value that does not read
         // is not repeated, as no other in this file is.
-        let entries = |key: &str, written: Vec<String>| {
+        fn entries<T: std::str::FromStr<Err = String>>(
+            connection: &str,
+            key: &str,
+            written: Vec<String>,
+        ) -> Result<Vec<T>, String> {
             written
                 .iter()
                 .enumerate()
                 .map(|(at, entry)| {
-                    entry.parse::<TableName>().map_err(|why| {
+                    entry.parse::<T>().map_err(|why| {
                         let place = at + 1;
                         format!("connection {connection:?}: agents' `{key}` entry {place} {why}")
                     })
                 })
-                .collect::<Result<Vec<_>, String>>()
-        };
+                .collect()
+        }
         let tables = match (self.hidden, self.only) {
             (Some(_), Some(_)) => {
                 return Err(format!(
@@ -277,8 +287,10 @@ impl AgentsTable {
                 ));
             }
             (Some(hidden), None) if hidden.is_empty() => Tables::All,
-            (Some(hidden), None) => Tables::AllExcept(entries("hidden", hidden)?),
-            (None, Some(only)) => Tables::Only(entries("only", only)?),
+            (Some(hidden), None) => {
+                Tables::AllExcept(entries::<TableName>(connection, "hidden", hidden)?)
+            }
+            (None, Some(only)) => Tables::Only(entries::<TableName>(connection, "only", only)?),
             (None, None) => Tables::All,
         };
         let agents = Agents {
@@ -286,6 +298,10 @@ impl AgentsTable {
             rows: self.rows.unwrap_or(starts.rows),
             timeout_secs: self.timeout_secs.unwrap_or(starts.timeout_secs),
             tables,
+            masked: match self.masked {
+                Some(masked) => entries::<ColumnName>(connection, "masked", masked)?,
+                None => Vec::new(),
+            },
         };
         if !(1..=MAX_AGENT_ROWS).contains(&agents.rows) {
             return Err(format!(
@@ -390,14 +406,18 @@ fn tls(mode: TlsMode) -> &'static str {
 /// except a server's own (host, port, user, database, TLS), which a person
 /// looks for.
 pub(super) fn write(connections: &[SavedConnection]) -> std::io::Result<String> {
+    let masks = connections
+        .iter()
+        .any(|saved| !saved.agents.masked.is_empty());
     let hides = connections
         .iter()
         .any(|saved| saved.agents.tables.hides_anything());
     let agents = connections.iter().any(|saved| !saved.agents.is_default());
-    let version = match (hides, agents) {
-        (true, _) => VERSION,
-        (false, true) => AGENTS_VERSION,
-        (false, false) => FIRST_VERSION,
+    let version = match (masks, hides, agents) {
+        (true, _, _) => VERSION,
+        (false, true, _) => TABLES_VERSION,
+        (false, false, true) => AGENTS_VERSION,
+        (false, false, false) => FIRST_VERSION,
     };
     let mut out = format!("{HEADER}version = {version}\n");
     for connection in connections {
@@ -457,6 +477,9 @@ pub(super) fn write(connections: &[SavedConnection]) -> std::io::Result<String>
                 Tables::AllExcept(names) => lines.bare("hidden", entries(names)),
                 Tables::Only(names) => lines.bare("only", entries(names)),
             }
+            if !connection.agents.masked.is_empty() {
+                lines.bare("masked", entries(&connection.agents.masked));
+            }
             out.push_str("\n[connections.agents]\n");
             lines.write(&mut out);
         }
@@ -464,8 +487,9 @@ pub(super) fn write(connections: &[SavedConnection]) -> std::io::Result<String>
     Ok(out)
 }
 
-/// Tables as a list of TOML strings, each written as the person writes it.
-fn entries(names: &[TableName]) -> String {
+/// Tables or columns as a list of TOML strings, each written as the person
+/// writes it.
+fn entries<T: std::fmt::Display>(names: &[T]) -> String {
     let written: Vec<String> = names
         .iter()
         .map(|name| toml_string(&name.to_string()))
diff --git a/src/model.rs b/src/model.rs
index 9458560..357801d 100644
--- a/src/model.rs
+++ b/src/model.rs
@@ -1294,6 +1294,9 @@ pub struct ConnectionForm {
     /// The tables in force when the dialog opened: others are the
     /// person's own, and the same keep what the file says.
     pub agents_tables_opened: tabletist_db::agent::Tables,
+    /// The masked columns in force when the dialog opened, carried to the
+    /// save as they are: the editor has no control for them yet.
+    pub agents_masked: Vec<tabletist_db::agent::ColumnName>,
     /// The name being typed after Add table…, while its field is open.
     pub agents_table_typed: Option<String>,
     /// Why the name typed does not read, said under the names.
@@ -1427,6 +1430,7 @@ impl Default for ConnectionForm {
             agents_opened: crate::connections::AgentAccess::Off,
             agents_tables: TablesChoice::default(),
             agents_tables_opened: tabletist_db::agent::Tables::All,
+            agents_masked: Vec::new(),
             agents_table_typed: None,
             agents_table_error: None,
             agents_table_focus: None,
@@ -1579,6 +1583,7 @@ impl ConnectionForm {
             agents_opened: saved.agents.access,
             agents_tables: TablesChoice::of(&saved.agents.tables),
             agents_tables_opened: saved.agents.tables.clone(),
+            agents_masked: saved.agents.masked.clone(),
             agents_table_typed: None,
             agents_table_error: None,
             agents_table_focus: None,
@@ -1906,6 +1911,7 @@ impl ConnectionForm {
             rows,
             timeout_secs,
             tables: self.agents_tables.tables(),
+            masked: self.agents_masked.clone(),
         })
     }
 }
diff --git a/src/ui/mod.rs b/src/ui/mod.rs
index 1159b8b..3795efd 100644
--- a/src/ui/mod.rs
+++ b/src/ui/mod.rs
@@ -206,13 +206,21 @@ pub fn agents_held_said(app: &App) -> Option<String> {
                 let name = called(id);
                 crate::i18n::gettext(locale, "agents may read {name}").replace("{name}", &name)
             }
-            Raised::Tables { id, .. } => {
+            Raised::Tables { id, .. } | Raised::Masked { id, .. } => {
                 let name = called(id);
                 crate::i18n::gettext(locale, "agents may read more of {name}")
                     .replace("{name}", &name)
             }
         })
         .collect();
+    // A connection that hides and masks less is said once.
+    let mut said: Vec<String> = Vec::new();
+    for part in parts {
+        if !said.contains(&part) {
+            said.push(part);
+        }
+    }
+    let parts = said;
     let changes = match parts.as_slice() {
         [one] => one.clone(),
         // A catalog may join them otherwise, and in another order.
````

- [ ] **Step 4: Run the tests**

Run: `cargo test --locked --lib -- masks_less masked_make_the_file does_not_read_is_off said_once`

Expected: PASS.

```text
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 2557 filtered out
```

- [ ] **Step 5: Commit**

```bash
cargo fmt --all
cargo clippy --locked --workspace --all-targets -- -D warnings
git add src/connections.rs src/connections/file.rs src/agents_shown.rs src/model.rs src/app.rs src/ui/mod.rs
git commit -m "Keep the columns agents get masked in connections.toml, and hold a change that masks less"
```

---

### Task 2: The app passes them to the server

- `send_agents_policy` sends what is in force (`App::agents_masked`); the backend's `Agentable::masked` carries it to the gateway's `Listed::masked`, which the server has used since run 2.

**Files:**
- Modify: `src/app.rs`, `src/backend/agents.rs`

- [ ] **Step 1: Write the failing test**

````diff
diff --git a/src/app.rs b/src/app.rs
index 052442a..de9cf0f 100644
--- a/src/app.rs
+++ b/src/app.rs
@@ -7305,6 +7305,12 @@ mod tests {
             (saved.name.as_str(), &saved.agents.masked),
             ("Local shop", &email)
         );
+        // And the server is sent them.
+        let sent_masked = match agents_sent(&app).last() {
+            Some(AgentsCommand::Policy { connections, .. }) => connections[0].masked.clone(),
+            other => panic!("{other:?}"),
+        };
+        assert_eq!(sent_masked, email);
     }
 
     #[cfg(any(unix, windows))]
diff --git a/src/backend/agents.rs b/src/backend/agents.rs
index 6d89bae..8ed65fd 100644
--- a/src/backend/agents.rs
+++ b/src/backend/agents.rs
@@ -1127,8 +1127,13 @@ mod tests {
         let mut shop = saved("shop", "Bookshop", Environment::Dev, spec);
         let customers = Tables::AllExcept(vec!["customers".parse().unwrap()]);
         shop.agents.tables = customers.clone();
+        shop.agents.masked = vec!["orders.email".parse().unwrap()];
         let gateway = gateway(shop);
         assert_eq!(gateway.connections()[0].tables, customers);
+        assert_eq!(
+            gateway.connections()[0].masked,
+            vec!["orders.email".parse().unwrap()]
+        );
         let stop = CancellationToken::new();
         let session = gateway.open("shop", &stop).await.unwrap();
         let read = check_read(Dialect::Sqlite, "SELECT * FROM customers", &[]).unwrap();
````

- [ ] **Step 2: Run it to see that it does not build**

Run: `cargo test --locked --lib -- masks_less_is_held_and_the_editor the_gateway_lists_what`

Expected: it does not build:

```text
error[E0609]: no field `masked` on type `Agentable`
error: could not compile `tabletist` (lib test) due to 1 previous error
```

- [ ] **Step 3: To the server**

````diff
diff --git a/src/app.rs b/src/app.rs
index de9cf0f..561bacd 100644
--- a/src/app.rs
+++ b/src/app.rs
@@ -3575,6 +3575,7 @@ impl App {
                 let mut now = saved.clone();
                 now.agents.access = self.agents_access(saved);
                 now.agents.tables = self.agents_tables(saved);
+                now.agents.masked = self.agents_masked(saved);
                 now
             })
             .collect();
diff --git a/src/backend/agents.rs b/src/backend/agents.rs
index 8ed65fd..0367275 100644
--- a/src/backend/agents.rs
+++ b/src/backend/agents.rs
@@ -47,6 +47,8 @@ pub struct Agentable {
     pub limits: Limits,
     /// What agents may not read of it.
     pub tables: Tables,
+    /// The columns whose values agents get masked.
+    pub masked: Vec<ColumnName>,
 }
 
 /// The connections of `saved` that agents may use, in its order. One that
@@ -78,6 +80,7 @@ pub fn agentable(saved: &[SavedConnection]) -> Vec<Agentable> {
                 timeout: Duration::from_secs(u64::from(saved.agents.timeout_secs)),
             },
             tables: saved.agents.tables.clone(),
+            masked: saved.agents.masked.clone(),
         })
         .collect()
 }
@@ -658,8 +661,7 @@ impl Gateway for AppGateway {
                 dialect: agentable.spec.driver.dialect(),
                 limits: agentable.limits,
                 tables: agentable.tables.clone(),
-                // Masked columns are read from the setting in the next run.
-                masked: Vec::new(),
+                masked: agentable.masked.clone(),
             })
             .collect()
     }
````

- [ ] **Step 4: Run the tests**

Run: `cargo test --locked --lib -- masks_less_is_held_and_the_editor the_gateway_lists_what`

Expected: PASS.

```text
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 2563 filtered out
```

- [ ] **Step 5: Commit**

```bash
cargo fmt --all
cargo clippy --locked --workspace --all-targets -- -D warnings
git add src/app.rs src/backend/agents.rs
git commit -m "Pass each connection's masked columns to the agent server"
```

---

### Task 3: The reference page

- `docs/_reference/settings-and-files.md`: the `masked` key, what a masked column is to an agent, version 6, what a change made outside that masks less is held for, and what `agents_shown.json` keeps.

**Files:**
- Modify: `docs/_reference/settings-and-files.md`

- [ ] **Step 1: The page**

````diff
diff --git a/docs/_reference/settings-and-files.md b/docs/_reference/settings-and-files.md
index 3e49552..984f943 100644
--- a/docs/_reference/settings-and-files.md
+++ b/docs/_reference/settings-and-files.md
@@ -103,7 +103,7 @@ On Linux, `XDG_CONFIG_HOME` and `XDG_STATE_HOME` move the two folders.
 | `labels.json` | State | The column you chose to label each table's rows in a foreign key's list, by saved connection and, on PostgreSQL, by database |
 | `tabletist.log`, `panic.log` | State | Logs |
 | `agents.sqlite3` | State | Every call agents made: the tool, the statement and its values, and how it ended. Never a row |
-| `agents_shown.json` | State | What agents may do as Tabletist last showed it: whether the server is on, and each connection's access |
+| `agents_shown.json` | State | What agents may do as Tabletist last showed it: whether the server is on, and each connection's access, hidden tables and masked columns |
 | `mcp.sock`, `mcp.key`, `mcp.lock` | Runtime, else State | Where `tabletist mcp` reaches the app, the key each proves it holds before either says more, and the lock the app holds while it listens. The socket and the key are there while agents are on |
 | `mcp.pipe` | State | On Windows, in the socket's place: the name of the pipe `tabletist mcp` reaches the app by, new each time agents are turned on |
 
@@ -201,9 +201,31 @@ system catalogs either (`information_schema`, `pg_catalog`, `mysql`,
 or dropped, agents may not use the connection at all until you take the
 name out of the list or put the right one in.
 
-A file in which a connection hides a table says `version = 5`, one in
-which a connection has the table and hides nothing says `version = 4`,
-and one in which none has it stays `version = 3`. Agents use the table only while
+`masked` names the columns whose values agents get as `•••`, every value,
+an empty one too:
+
+```toml
+[connections.agents]
+access = "read"
+masked = ["orders.email", "sales.customers.phone"]
+```
+
+An entry is a table and a column, or a schema, a table and a column, each
+part written as in `hidden`. One without a schema is that column of the
+table of that name in every schema. Agents may select a masked column as
+it is, and may not filter, join, sort, group or compute on it, nor on any
+column of the same name in any table. A view that reads one is refused,
+and so is any view over its table on MySQL. Its partitions and child
+tables have it masked too, and so does a generated column of its table.
+If a column that `masked` names is renamed or dropped, agents may not use
+the connection at all until you take it out of the list or put the right
+one in.
+
+A file in which a connection masks a column says `version = 6`, one in
+which a connection hides a table and masks none says `version = 5`, one
+in which a connection has the table and neither hides nor masks says
+`version = 4`, and one in which none has it stays `version = 3`. Agents
+use the table only while
 `[mcp] enabled` is `true` in `settings.toml`. **Stop agents here**, in a
 connection's Agents tab, stops the agents working on it and sets its
 `access` to `off`, so that an agent that connects again does not see it.
@@ -212,14 +234,18 @@ connection's Agents tab, stops the agents working on it and sets its
 
 A change to either file that lets agents do more than Tabletist last
 showed you is not applied at once: turning `[mcp] enabled` on, a
-connection's `access` up from `off`, or a connection hiding less: a name
+connection's `access` up from `off`, a connection hiding less (a name
 taken out of `hidden` or put into `only`, or a `hidden` list made an
-`only` one. Tabletist says what
+`only` one), or a connection masking less: a name taken out of `masked`,
+or one that named a column in every schema made to name it in one.
+Tabletist says what
 changed in a bar under the top of the window, and until you answer,
 agents may do what they could before. **Keep** applies the change. **Put
 back** undoes it in the files: it turns `[mcp] enabled` off, sets the
-connection's `access` back to `off`, or puts its tables back as they were,
-and leaves your other settings and connections as they are. A change that
+connection's `access` back to `off`, or puts its tables or its masked
+columns back as they were, and leaves your other settings and connections
+as they are. While a name taken out of `masked` is held, a column put into
+it at the same time is masked at once. A change that
 lets agents do less applies at once.
 
 Turning agents on in Tabletist itself, with **Turn on** in a connection's
````

- [ ] **Step 2: Check it**

Run: `docs/bin/check` and `grep -c "$(printf '\342\200\224')" docs/_reference/settings-and-files.md` (a count of em dashes)

Expected: `HTML-Proofer finished successfully.`, and no em dash (`0`).

- [ ] **Step 3: Commit**

```bash
git add docs/_reference/settings-and-files.md
git commit -m "Say on the website how connections.toml masks agents' columns"
```

---

### Task 4: The master plan

- B.4 as built for 3a-3; G's row and 3a's run 3: built.

**Files:**
- Modify: `docs/superpowers/plans/2026-10-13-mcp-server.md`

- [ ] **Step 1: The edits**

````diff
diff --git a/docs/superpowers/plans/2026-10-13-mcp-server.md b/docs/superpowers/plans/2026-10-13-mcp-server.md
index b026fd3..1141cf2 100644
--- a/docs/superpowers/plans/2026-10-13-mcp-server.md
+++ b/docs/superpowers/plans/2026-10-13-mcp-server.md
@@ -1183,6 +1183,27 @@ C.1 gave it from the start.
   the column is computed from the masked one is not asked: it errs toward
   masking.
 
+**As built in 3a's run 3** (`2026-10-17-mcp-server-3a-3-setting.md`): the
+setting, as 1c-2b's for the tables. A connection's `[connections.agents]`
+table takes `masked = ["orders.email"]`, read through `ColumnName` and
+written back as the person writes it; a file with a masked column is
+version 6, so that a Tabletist that would answer its values as they are
+never reads it. `agents_shown.json` keeps each connection's masked
+columns as last shown. A change made outside the app that masks less (an
+entry taken out, or one with a schema in place of one for every schema)
+is held (`Raised::Masked`), with the sentence a table's has ("agents may
+read more of …"); one that masks more is taken. While one is held, what
+was shown is in force with what the file says now, so that a column the
+change also masks is masked at once; **Put back** writes what was shown
+back. A shown entry that does not read, which only a hand could write,
+holds every change, and while one is held the connection is Off to
+agents, since what that entry masked cannot be kept masked; **Put back**
+writes the entries that read, and they are what was shown from then on
+(found by the review). A connection that hides and masks less is said
+once in the bar. The editor has no control for them until run 4: its
+save keeps what the file says. The app's gateway passes them to the
+server, which has used them since run 2.
+
 **The rule.** It applies to any statement that reads a table with a masked
 column.
 
@@ -2619,7 +2640,7 @@ is off until the person turns it on.
 | 1b on Windows: the way in | 1 | Built (`2026-10-15-mcp-server-1b-6-windows.md`) |
 | 1c. Hidden tables | 6, below | Built (`2026-10-15-mcp-server-1c-1-checks.md`, `2026-10-15-mcp-server-1c-2a-server.md`, `2026-10-16-mcp-server-1c-2b-setting.md`, `2026-10-16-mcp-server-1c-3a-tables-control.md`, `2026-10-16-mcp-server-1c-3b-table-list.md`) |
 | 1d. HTTP with tokens | 1 | Dropped for now (2026-10-15) |
-| 3a. Masked columns | 5, below | Runs 1a, 1b and 2 built (`2026-10-17-mcp-server-3a-1a-masked-rule.md`, `2026-10-17-mcp-server-3a-1b-masked-values.md`, `2026-10-17-mcp-server-3a-2-server.md`): the rule, its stops, the engines' fences, the values and the server's side. Run 3, the setting in `connections.toml`, is next (order changed on 2026-10-17) |
+| 3a. Masked columns | 5, below | Runs 1a to 3 built (`2026-10-17-mcp-server-3a-1a-masked-rule.md`, `2026-10-17-mcp-server-3a-1b-masked-values.md`, `2026-10-17-mcp-server-3a-2-server.md`, `2026-10-17-mcp-server-3a-3-setting.md`): the rule, its stops, the engines' fences, the values, the server's side and the setting. Run 4, the editor's Masked columns control and the website's guide, is next (order changed on 2026-10-17) |
 | 3b. The Log view, in full | Not cut yet | After 3a |
 | 2. Approved writes | Not cut yet | Last, after 3b (moved on 2026-10-17) |
 
@@ -2816,7 +2837,8 @@ pull request:
 2. The server's side: the setting read, `list_connections`' masked
    columns, `describe_schema`'s mark, and `masked: true` in an answer
    (`2026-10-17-mcp-server-3a-2-server.md`, built).
-3. The setting in `connections.toml`, as 1c-2b's for tables.
+3. The setting in `connections.toml`, as 1c-2b's for tables
+   (`2026-10-17-mcp-server-3a-3-setting.md`, built).
 4. The editor's Masked columns control with **Add column…** and its mark,
    and the website's guide.
 
````

- [ ] **Step 2: Check the whole of it**

Run: `cargo fmt --all --check`, clippy as above, `RUSTDOCFLAGS='-D warnings' cargo doc --locked --workspace --no-deps`, the count of em dashes in the master plan, and `cargo test --locked --workspace --all-targets` with both servers

Expected: fmt and clippy clean, the documentation builds without a warning, no em dash (`0`), and:

```text
3570 passed, 0 failed, 22 ignored
```

- [ ] **Step 3: Commit**

```bash
git add docs/superpowers/plans/2026-10-13-mcp-server.md
git commit -m "Say in the MCP plan that the masked columns' setting is built"
```

## Self-review

- **Against the design.** G's run 3 ("the setting in `connections.toml`, as 1c-2b's for tables"), C.1's `masked` and C.2's holding of a change made outside the app, with the departures in "What the plan asks, and what is built".
- **A review before the plan.** A reviewer drove the draft through the app's tests and found no change made outside that masks less and is not held, and none whose value in force lets a masked column through. It found a held change against a shown entry that does not read, which **Put back** could not end and which left that entry's columns in force in no form; the held bar saying a connection twice where it hides and masks less; the reference page leaving out an entry made one schema's and what `agents_shown.json` keeps; and the policy the app sends, and the gateway's list, untested. All fixed, with a test each.
- **Placeholders.** None: every step has its diff or its command and what it prints.
- **Names.** `Agents::masked`, `Shown::masked`, `masked_of`, `ShownMasked`, `Raised::Masked`, `masks_no_less`, `App::agents_masked`, `ConnectionForm::agents_masked` and `Agentable::masked` are the same in every task that uses them.
