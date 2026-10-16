# MCP server, Phase 1c, run 2b: hidden tables in connections.toml. Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A connection's `[connections.agents]` table can say which tables agents may not read (`hidden = [...]`) or the only ones they may (`only = [...]`). The app passes that to the server, which enforces it since 1c-2a. A change made outside the app that hides less is held until the person keeps it or puts it back, as a raised access is. The website's reference describes the keys.

**Architecture:** `Agents` (`src/connections.rs`) gains `tables: tabletist_db::agent::Tables`. `src/connections/file.rs` reads and writes the two lists through `TableName`'s new `FromStr`, and writes version 5 where a connection hides something. `src/agents_shown.rs` keeps each connection's tables as last shown; `Shown::compare` holds a change that hides less (`Raised::Tables`, decided by `hides_no_less`) and takes one that hides more. `App::agents_tables` gives what is in force, which `send_agents_policy` sends and **Put back** writes back. The backend's `Agentable` carries the tables to the gateway's `Listed`.

**Tech Stack:** Rust 2024, `serde` and `toml` for the file, egui for the held bar's sentence.

---

## Where this sits

Phase 1c of `docs/superpowers/plans/2026-10-13-mcp-server.md`, hidden tables. 1c-1 (pull request 144) built the checks and 1c-2a (pull request 146) the server's side, with the app passing "All". This run, 1c-2b, is the setting. 1c-3 is the editor's Tables control, its hint, and the website's guide: until then the file is the only way to set it.

## Before you start

- Work on a branch of `main`. One commit for each task, in the order below. Every commit passes `cargo fmt --all --check`, `cargo clippy --locked --workspace --all-targets -- -D warnings` and `cargo test --locked --workspace --all-targets`.
- On Igor's machine `cargo` is `~/.cargo/bin/cargo` (the mise shim fails), and a build in a worktree needs `CARGO_TARGET_DIR` set to that worktree's `target/`.
- Read `AGENTS.md`: a value the connections file could not read is never repeated in what is said of it, and the website's pages are checked with `docs/bin/check`.
- The diffs below apply with `git apply`. Run `cargo fmt --all` before each commit: the code here is already formatted.

## What was run before this plan was written

Every task's state was built here, passed `cargo fmt --check` and clippy with `-D warnings`, and its tests failed and then passed as each step says. The last state passed `cargo test --locked --workspace --all-targets` with both test servers (3,528 tests, none failed), `RUSTDOCFLAGS='-D warnings' cargo doc --locked --workspace --no-deps` and `docs/bin/check`. Not run: CI, and Windows and macOS (the held bar's tests are `cfg(any(unix, windows))`, which CI runs on all three).

## What this run settles

| Question | Decision | Why |
|---|---|---|
| How the file says it | `hidden = [...]` or `only = [...]`, not both, in `[connections.agents]` | The list says which it is, so no `tables` key is needed beside it, as C.1's JSON had. Both at once has no meaning, and is refused |
| An empty list | `hidden = []` hides nothing; `only = []` hides every table | "All except nothing" is "All". "Only" a list hides what it does not name, even when it names nothing |
| How an entry is written | `customers`, `sales.customers`; a part with a dot or a double quote in double quotes, the quote doubled | As the person writes a name, and as `list_connections` already tells it (1c-2a) |
| An entry that does not read | Refused by its place in the list (`hidden` entry 2), without its words | No value the file could not read is repeated (`AGENTS.md`) |
| The file's version | 5 where a connection hides something, 4 where it has agents' settings and hides nothing, 3 where none has any | A Tabletist that does not know the lists would read every table as readable: it must not read such a file. One that holds no list loses nothing to an older one |
| A change made outside that hides less | Held, as a raised access is; what was shown is in force until the person answers | C.2: "a table unhidden" is on its list of what makes agents more able |
| When a change hides less | Every table the old setting hid is hidden still, entry by entry: one without a schema covers that name in every schema, and not the other way round. "Only" to "All except" always hides less, and so does "All except" to "Only" a list that names anything | The comparison must not let `sales.customers` go readable through an "Only `customers`" list. And an "Only" list reads a named table's partitions and shadow tables, and a hidden table renamed by its new name, which no name tells (found by the review) |
| What was shown, where the file says nothing of a connection | Kept, so that a list that hides less when it comes back is held | A start that read no connection, from a file kept aside or of a newer version, deleted nothing (found by the review) |
| A shown entry that does not read | The list is taken for hiding every table | Only a hand could write one, and what it hid cannot be told (found by the review) |
| The editor, which has no control for the tables yet | Its save keeps what the file says at that moment | As it does for an access it did not change: a change made outside stays held, and one put back stays back |
| A duplicated connection | Keeps the tables in force, the shown ones where a change is held | It starts Off for agents, and keeping what was hidden errs toward hiding. The file's own, while a change that hides less is held, would have given the copy what the person did not keep (found by the review) |

### What this run leaves out

- The editor's Tables control, its hint, an entry marked "not in this database", and the website's guide to hiding tables: 1c-3. Until then the editor keeps what the file says.
- Masked columns (Phase 3a).

## File structure

| File | What changes |
|---|---|
| `crates/tabletist-db/src/agent/hidden.rs` | `FromStr` for `TableName`, the inverse of its `Display` |
| `src/connections.rs` | `Agents::tables`; `Agents` is no longer `Copy` |
| `src/connections/file.rs` | `hidden` and `only`; version 5 |
| `src/model.rs` | The editor's `agents_to_saved` carries the tables it opened with |
| `src/agents_shown.rs` | `Shown::tables`, `ShownTables`, `Raised::Tables`, `hides_no_less` |
| `src/app.rs` | `agents_tables`; the policy, **Put back** and the editor's save |
| `src/ui/mod.rs` | The held bar's sentence for a connection that hides less |
| `src/backend/agents.rs` | `Agentable::tables`, to the gateway's `Listed` |
| `docs/_reference/settings-and-files.md` | The keys, version 5, and the hold |
| `docs/superpowers/plans/2026-10-13-mcp-server.md` | What this run built |

---

### Task 1: The setting, read, written and held

- `TableName` reads what its `Display` writes: `customers`, `sales.customers`, a part in double quotes with a quote doubled. Its error says what is wrong without the entry's words.
- `Agents::tables`, `All` for a connection that never had any. `Agents` is `Clone` and no longer `Copy`.
- `connections.toml`: `hidden = [...]` is "All except", `only = [...]` is "Only", both is an error, `hidden = []` hides nothing, `only = []` hides every table. An entry that does not read is refused by its place in the list. A file in which a connection hides something is version 5; one that has agents' settings and hides nothing stays 4.
- `agents_shown.json` keeps each connection's tables as last shown. A change that hides less is held, `Raised::Tables`; one that hides more, or the same in other words, is taken. `hides_no_less` compares by what each entry covers: one without a schema covers that name in every schema.
- `App::agents_tables` is what is in force: the shown tables while a raise is held. The policy sends it, **Put back** writes it back to the file, and the held bar says "agents may read more of Bookshop · dev".
- The editor has no control for the tables: its form carries the tables it opened with, and its save keeps what the file says at that moment.

**Files:**
- Modify: `crates/tabletist-db/src/agent/hidden.rs`, `src/connections.rs`, `src/connections/file.rs`, `src/model.rs`, `src/agents_shown.rs`, `src/app.rs`, `src/ui/mod.rs`

- [ ] **Step 1: Write the failing tests**

````diff
diff --git a/crates/tabletist-db/src/agent/hidden.rs b/crates/tabletist-db/src/agent/hidden.rs
index e38f03c..6d8baa1 100644
--- a/crates/tabletist-db/src/agent/hidden.rs
+++ b/crates/tabletist-db/src/agent/hidden.rs
@@ -1421,6 +1421,26 @@ mod tests {
         assert_eq!(only.check_settings(), Ok(()));
     }
 
+    #[test]
+    fn an_entry_is_read_as_it_is_written() {
+        for written in [
+            "customers",
+            "sales.customers",
+            "\"odd.schema\".\"say \"\"hi\"\"\"",
+            "\"a.b\"",
+        ] {
+            let read: TableName = written.parse().unwrap();
+            assert_eq!(read.to_string(), written);
+        }
+        assert_eq!(
+            "\"sales\".customers".parse::<TableName>(),
+            Ok(table(Some("sales"), "customers"))
+        );
+        for wrong in ["", "a.", ".b", "a.b.c", "\"a", "a\"b", "\"a\"b"] {
+            assert!(wrong.parse::<TableName>().is_err(), "{wrong:?}");
+        }
+    }
+
     #[test]
     fn an_entry_is_written_as_the_person_writes_it() {
         assert_eq!(table(None, "customers").to_string(), "customers");
diff --git a/src/agents_shown.rs b/src/agents_shown.rs
index c147ed8..415c3f6 100644
--- a/src/agents_shown.rs
+++ b/src/agents_shown.rs
@@ -163,6 +163,129 @@ mod tests {
         assert_eq!(shown, Shown::default());
     }
 
+    fn hiding(id: &str, tables: Tables) -> SavedConnection {
+        let mut saved = saved(id, AgentAccess::Read);
+        saved.agents.tables = tables;
+        saved
+    }
+
+    fn names(written: &[&str]) -> Vec<TableName> {
+        written.iter().map(|entry| entry.parse().unwrap()).collect()
+    }
+
+    #[test]
+    fn a_change_that_hides_less_is_held_and_one_that_hides_more_is_taken() {
+        let id = || ConnectionId("shop".into());
+        let mut shown = Shown::default();
+        shown.keep(&[Raised::Access {
+            id: id(),
+            to: AgentAccess::Read,
+        }]);
+        // Hiding a table where none was: taken.
+        let customers = Tables::AllExcept(names(&["customers"]));
+        let (raised, lowered) = shown.compare(false, &[hiding("shop", customers.clone())]);
+        assert_eq!((raised, lowered), (Vec::new(), true));
+        assert_eq!(shown.tables_of(&id()), customers);
+        // Each way to read more is held, with what it would read.
+        for more in [
+            Tables::All,
+            Tables::AllExcept(Vec::new()),
+            Tables::AllExcept(names(&["orders"])),
+            Tables::AllExcept(names(&["sales.customers"])),
+            Tables::Only(names(&["customers"])),
+            Tables::Only(names(&["books", "public.customers"])),
+            // A partition of a hidden table goes with its parent, and a
+            // hidden table renamed reads by its new name: a name cannot
+            // tell either from a table that was readable.
+            Tables::Only(names(&["books"])),
+        ] {
+            let mut copy = shown.clone();
+            let (raised, lowered) = copy.compare(false, &[hiding("shop", more.clone())]);
+            assert!(!lowered, "{more:?}");
+            assert_eq!(raised.len(), 1, "{more:?}");
+            assert!(
+                matches!(&raised[0], Raised::Tables { id: held, .. } if *held == id()),
+                "{more:?}"
+            );
+            assert_eq!(copy.tables_of(&id()), customers, "what was shown holds");
+        }
+        // Each way to read less, or the same in other words, is taken.
+        for less in [
+            Tables::AllExcept(names(&["customers", "orders"])),
+            Tables::AllExcept(names(&["CUSTOMERS"])),
+            Tables::Only(Vec::new()),
+        ] {
+            let mut copy = shown.clone();
+            let (raised, _) = copy.compare(false, &[hiding("shop", less.clone())]);
+            assert_eq!(raised, [], "{less:?}");
+            assert_eq!(copy.tables_of(&id()), less, "{less:?}");
+        }
+        // From an "Only" list, adding a table is held; taking one is not.
+        let mut only = Shown::default();
+        only.keep(&[
+            Raised::Access {
+                id: id(),
+                to: AgentAccess::Read,
+            },
+            Raised::Tables {
+                id: id(),
+                to: Tables::Only(names(&["books", "orders"])),
+            },
+        ]);
+        let wider = Tables::Only(names(&["books", "orders", "customers"]));
+        let (raised, _) = only.clone().compare(false, &[hiding("shop", wider)]);
+        assert_eq!(raised.len(), 1);
+        let narrower = Tables::Only(names(&["books"]));
+        let (raised, _) = only.clone().compare(false, &[hiding("shop", narrower)]);
+        assert_eq!(raised, []);
+        let except = Tables::AllExcept(names(&["customers"]));
+        let (raised, _) = only.compare(false, &[hiding("shop", except)]);
+        assert_eq!(raised.len(), 1, "every table not named was hidden");
+    }
+
+    #[test]
+    fn what_a_connection_hid_is_kept_while_it_is_not_there() {
+        let id = || ConnectionId("shop".into());
+        let mut shown = Shown::default();
+        shown.keep(&[
+            Raised::Access {
+                id: id(),
+                to: AgentAccess::Read,
+            },
+            Raised::Tables {
+                id: id(),
+                to: Tables::AllExcept(names(&["customers"])),
+            },
+        ]);
+        // A start that read no connection, then the file again without
+        // the list: held.
+        shown.compare(false, &[]);
+        let (raised, _) = shown.compare(false, &[hiding("shop", Tables::All)]);
+        assert!(
+            raised
+                .iter()
+                .any(|raise| matches!(raise, Raised::Tables { .. })),
+            "{raised:?}"
+        );
+    }
+
+    #[test]
+    fn a_shown_list_that_does_not_read_hides_every_table() {
+        let mut shown = Shown::default();
+        shown.tables.insert(
+            "shop".into(),
+            ShownTables::Hidden(vec!["\"cust.omers".into()]),
+        );
+        let id = ConnectionId("shop".into());
+        assert_eq!(shown.tables_of(&id), Tables::Only(Vec::new()));
+        shown.keep(&[Raised::Access {
+            id: id.clone(),
+            to: AgentAccess::Read,
+        }]);
+        let (raised, _) = shown.compare(false, &[hiding("shop", Tables::All)]);
+        assert_eq!(raised.len(), 1, "{raised:?}");
+    }
+
     #[test]
     fn a_connection_deleted_since_is_forgotten() {
         let mut shown = Shown::default();
@@ -194,8 +317,15 @@ mod tests {
         ]);
         shown.save(&path).unwrap();
         assert_eq!(Shown::load(&path), shown);
+        shown.keep(&[Raised::Tables {
+            id: ConnectionId("shop".into()),
+            to: Tables::AllExcept(names(&["customers", "\"odd.name\""])),
+        }]);
+        shown.save(&path).unwrap();
+        assert_eq!(Shown::load(&path), shown);
         let text = std::fs::read_to_string(&path).unwrap();
         assert!(text.contains("\"shop\": \"read\""), "{text}");
+        assert!(text.contains("\"hidden\""), "{text}");
         std::fs::write(&path, "{ not json").unwrap();
         assert_eq!(Shown::load(&path), Shown::default());
     }
diff --git a/src/app.rs b/src/app.rs
index 3a103fe..8efec54 100644
--- a/src/app.rs
+++ b/src/app.rs
@@ -6861,7 +6861,7 @@ mod tests {
         );
         form(&mut app).agents_timeout = "30".into();
         app.apply(Action::SaveConnection { connect: false });
-        let agents = app.connections.get(&conn).unwrap().agents;
+        let agents = app.connections.get(&conn).unwrap().agents.clone();
         assert_eq!((agents.rows, agents.timeout_secs), (250, 30));
         // A new connection given Read in the dialog is shown, not held.
         app.apply(Action::NewConnection);
@@ -6880,6 +6880,75 @@ mod tests {
         );
     }
 
+    #[cfg(any(unix, windows))]
+    #[test]
+    fn a_change_outside_that_hides_less_is_held_and_the_editor_keeps_the_files_tables() {
+        use crate::agents_shown::Raised;
+        use crate::connections::AgentAccess;
+        use tabletist_db::agent::Tables;
+        let (mut app, _dir) = app();
+        let conn = with_saved(&mut app);
+        app.serve_agents();
+        app.apply(Action::SetAgentAccess {
+            conn: conn.clone(),
+            access: AgentAccess::Read,
+        });
+        // What is in force, which the policy sends.
+        let sent = |app: &App| app.agents_tables(app.connections.get(&conn).unwrap());
+        let file_says = |app: &mut App, tables: Tables| {
+            let mut saved = app.connections.get(&conn).unwrap().clone();
+            saved.agents.tables = tables;
+            app.connections.upsert(saved);
+            app.check_agents();
+            app.send_agents_policy();
+        };
+        let customers = Tables::AllExcept(vec!["customers".parse().unwrap()]);
+        // Hiding more, from the file: taken at once.
+        file_says(&mut app, customers.clone());
+        assert!(app.agents_held.is_empty());
+        assert_eq!(sent(&app), customers);
+        // Hiding less: held, and what was shown is in force.
+        file_says(&mut app, Tables::All);
+        assert!(matches!(
+            app.agents_held.as_slice(),
+            [Raised::Tables { .. }]
+        ));
+        assert_eq!(sent(&app), customers);
+        let said = crate::ui::agents_held_said(&app).unwrap();
+        assert!(
+            said.contains("agents may read more of Local · dev"),
+            "{said}"
+        );
+        // A copy made meanwhile hides what is in force, not what is held.
+        app.apply(Action::DuplicateConnection(conn.clone()));
+        let copy = &app.connections.connections[1];
+        assert_ne!(copy.id, conn);
+        assert_eq!(copy.agents.tables, customers);
+        let copy = copy.id.clone();
+        app.apply(Action::DeleteConnection(copy));
+        // Put back: the file hides it again.
+        app.apply(Action::PutBackAgentChanges(app.agents_held.clone()));
+        assert!(app.agents_held.is_empty());
+        assert_eq!(app.connections.get(&conn).unwrap().agents.tables, customers);
+        // Kept: in force.
+        file_says(&mut app, Tables::All);
+        app.apply(Action::KeepAgentChanges(app.agents_held.clone()));
+        assert!(app.agents_held.is_empty());
+        assert_eq!(sent(&app), Tables::All);
+        // The editor has no control for the tables: its save keeps what
+        // the file says when it saves.
+        app.apply(Action::EditConnection(conn.clone()));
+        let only = Tables::Only(vec!["books".parse().unwrap()]);
+        file_says(&mut app, only.clone());
+        form(&mut app).name = "Local shop".into();
+        app.apply(Action::SaveConnection { connect: false });
+        let saved = app.connections.get(&conn).unwrap();
+        assert_eq!(
+            (saved.name.as_str(), &saved.agents.tables),
+            ("Local shop", &only)
+        );
+    }
+
     #[test]
     fn agents_are_told_of_each_change_to_what_they_may_use() {
         let (mut app, _dir) = app();
diff --git a/src/connections.rs b/src/connections.rs
index 5305c86..b13420e 100644
--- a/src/connections.rs
+++ b/src/connections.rs
@@ -1064,10 +1064,11 @@ mod tests {
             access: AgentAccess::Read,
             rows: 1_000,
             timeout_secs: 30,
+            ..Agents::default()
         };
         store.upsert(original.clone());
         let copy = store.duplicate(&original.id).unwrap();
-        let copied = store.get(&copy).unwrap().agents;
+        let copied = store.get(&copy).unwrap().agents.clone();
         assert_eq!(copied.access, AgentAccess::Off);
         assert_eq!((copied.rows, copied.timeout_secs), (1_000, 30));
         assert_eq!(store.get(&original.id).unwrap().agents, original.agents);
diff --git a/src/connections/file.rs b/src/connections/file.rs
index 97439e4..1359e4b 100644
--- a/src/connections/file.rs
+++ b/src/connections/file.rs
@@ -847,8 +847,8 @@ auth = "key-file"
         let said = |text: &str| read(text).unwrap_err().to_string();
         assert!(said("").contains("missing field `version`"), "{}", said(""));
         assert_eq!(
-            said("version = 5\n"),
-            "it is version 5 of the file, written by a newer Tabletist; this one reads version 4"
+            said("version = 6\n"),
+            "it is version 6 of the file, written by a newer Tabletist; this one reads version 5"
         );
         assert_eq!(
             said("version = 2\n"),
@@ -862,6 +862,7 @@ auth = "key-file"
             access,
             rows,
             timeout_secs,
+            ..Agents::default()
         };
         connections
     }
@@ -915,7 +916,7 @@ timeout_secs = 30
             "{}\n[connections.agents]\naccess = \"read\"\n",
             write(&bookshop()).unwrap()
         );
-        let agents = read(&text).unwrap().connections[2].agents;
+        let agents = read(&text).unwrap().connections[2].agents.clone();
         assert_eq!(
             agents,
             Agents {
@@ -953,6 +954,49 @@ timeout_secs = 30
         assert!(!access.contains("read_and_write"), "{access}");
         let key = said("rowz = 9");
         assert!(key.contains("unknown field `rowz`"), "{key}");
+        // The tables: one list or the other, and each entry a table.
+        assert_eq!(
+            said("hidden = [\"customers\"]\nonly = [\"books\"]"),
+            "connection \"Bookshop on disk\": agents have `hidden` or `only`, not both"
+        );
+        let entry = said("hidden = [\"customers\", \"a.b.c\"]");
+        assert_eq!(
+            entry,
+            "connection \"Bookshop on disk\": agents' `hidden` entry 2 is a table, or a schema \
+             and a table, with one dot between"
+        );
+        let secret = said("hidden = [\"hunter2\\\"x\"]");
+        assert!(!secret.contains("hunter2"), "{secret}");
+    }
+
+    #[test]
+    fn the_tables_agents_may_not_read_make_the_file_version_five() {
+        let mut connections = with_agents(AgentAccess::Read, 500, 10);
+        connections[2].agents.tables = Tables::AllExcept(vec![
+            "customers".parse().unwrap(),
+            "\"odd.schema\".orders".parse().unwrap(),
+        ]);
+        let text = write(&connections).unwrap();
+        assert!(text.contains("\nversion = 5\n"), "{text}");
+        assert!(
+            text.ends_with(
+                r#"timeout_secs = 10
+hidden       = ["customers", "\"odd.schema\".orders"]
+"#
+            ),
+            "{text}"
+        );
+        assert_eq!(read(&text).unwrap().connections, connections);
+        // Only a list: the rest is hidden, and an empty one hides it all.
+        connections[2].agents.tables = Tables::Only(Vec::new());
+        let text = write(&connections).unwrap();
+        assert!(text.contains("only         = []"), "{text}");
+        assert_eq!(read(&text).unwrap().connections, connections);
+        // An empty "hidden" hides nothing: the file is version 4.
+        let four = text.replace("only         = []", "hidden = []");
+        connections[2].agents.tables = Tables::All;
+        assert_eq!(read(&four).unwrap().connections, connections);
+        assert!(write(&connections).unwrap().contains("\nversion = 4\n"));
     }
 
     #[test]
diff --git a/src/model.rs b/src/model.rs
index aee0299..51dde45 100644
--- a/src/model.rs
+++ b/src/model.rs
@@ -5256,6 +5256,7 @@ mod tests {
             access: crate::connections::AgentAccess::Read,
             rows: 1_000,
             timeout_secs: 30,
+            ..crate::connections::Agents::default()
         };
         let mut edited = ConnectionForm::from_saved(&saved);
         edited.name = "Production".into();
````

- [ ] **Step 2: Run them to see that they do not build**

Run: `cargo test --locked --lib -- connections:: agents_shown`

Expected: it does not build. The tests name what is not there yet; among the errors:

```text
error[E0609]: no field `tables` on type `connections::Agents`
error[E0599]: no method named `tables_of` found for struct `agents_shown::Shown` in the current scope
error[E0599]: no variant named `Tables` found for enum `agents_shown::Raised`
error[E0433]: cannot find type `ShownTables` in this scope
error[E0599]: no method named `agents_tables` found for reference `&app::App` in the current scope
error[E0277]: the trait bound `TableName: FromStr` is not satisfied
error: could not compile `tabletist` (lib test) due to 48 previous errors
```

- [ ] **Step 3: The tables, in the file and in what was shown**

````diff
diff --git a/crates/tabletist-db/src/agent/hidden.rs b/crates/tabletist-db/src/agent/hidden.rs
index 6d8baa1..4ca325f 100644
--- a/crates/tabletist-db/src/agent/hidden.rs
+++ b/crates/tabletist-db/src/agent/hidden.rs
@@ -63,6 +63,69 @@ impl std::fmt::Display for TableName {
     }
 }
 
+/// An entry as the person writes it, as [`TableName`]'s `Display` writes it
+/// back: a name, or a schema and a name with a dot between, a part in
+/// double quotes where it holds a dot or a quote. The error says what is
+/// wrong without the entry's words, for whoever gives it a place.
+impl std::str::FromStr for TableName {
+    type Err = String;
+
+    fn from_str(written: &str) -> Result<Self, String> {
+        let mut parts: Vec<String> = Vec::new();
+        let mut chars = written.chars().peekable();
+        loop {
+            let mut part = String::new();
+            if chars.peek() == Some(&'"') {
+                chars.next();
+                loop {
+                    match chars.next() {
+                        Some('"') if chars.peek() == Some(&'"') => {
+                            chars.next();
+                            part.push('"');
+                        }
+                        Some('"') => break,
+                        Some(c) => part.push(c),
+                        None => return Err("has a quote that is not closed".into()),
+                    }
+                }
+            } else {
+                while let Some(&c) = chars.peek()
+                    && c != '.'
+                {
+                    if c == '"' {
+                        return Err(
+                            "has a quote inside a name: put the whole name in quotes".into()
+                        );
+                    }
+                    part.push(c);
+                    chars.next();
+                }
+            }
+            if part.is_empty() {
+                return Err("has a name that is empty".into());
+            }
+            parts.push(part);
+            match chars.next() {
+                None => break,
+                Some('.') => {}
+                Some(_) => {
+                    return Err("goes on after a quoted name: a dot comes next".into());
+                }
+            }
+        }
+        match <[String; 2]>::try_from(parts) {
+            Ok([schema, name]) => Ok(Self {
+                schema: Some(schema),
+                name,
+            }),
+            Err(parts) => match <[String; 1]>::try_from(parts) {
+                Ok([name]) => Ok(Self { schema: None, name }),
+                Err(_) => Err("is a table, or a schema and a table, with one dot between".into()),
+            },
+        }
+    }
+}
+
 impl TableName {
     /// Whether this names the relation `name` of `schema`.
     fn names(&self, schema: Option<&str>, name: &str) -> bool {
diff --git a/src/agents_shown.rs b/src/agents_shown.rs
index 415c3f6..a6965c8 100644
--- a/src/agents_shown.rs
+++ b/src/agents_shown.rs
@@ -1,5 +1,5 @@
 //! What the app last showed the person of what agents may do: whether the
-//! server is on, and each connection's access. A change made outside the
+//! server is on, and each connection's access and the tables it hides. A change made outside the
 //! app (a hand, a program, a synced dotfile, a backup) that lets agents do
 //! more than that is held until the person keeps it or puts it back; one
 //! that lets them do less is taken at once.
@@ -13,6 +13,8 @@ use std::path::Path;
 
 use serde::{Deserialize, Serialize};
 
+use tabletist_db::agent::{TableName, Tables};
+
 use crate::connections::{AgentAccess, ConnectionId, SavedConnection};
 
 /// What agents may do, as the person last saw it.
@@ -23,6 +25,17 @@ pub struct Shown {
     pub enabled: bool,
     /// Each connection's access, by its id. A connection not here was Off.
     pub access: BTreeMap<String, AgentAccess>,
+    /// The tables each connection hides, by its id. A connection not here
+    /// hid none.
+    pub tables: BTreeMap<String, ShownTables>,
+}
+
+/// The tables a connection hid, each entry as the person writes it.
+#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
+#[serde(rename_all = "snake_case")]
+pub enum ShownTables {
+    Hidden(Vec<String>),
+    Only(Vec<String>),
 }
 
 /// A change that lets agents do more than the person was shown.
@@ -32,6 +45,8 @@ pub enum Raised {
     Server,
     /// A connection's access went up to `to`.
     Access { id: ConnectionId, to: AgentAccess },
+    /// A connection hides less: `to`.
+    Tables { id: ConnectionId, to: Tables },
 }
 
 impl Shown {
@@ -48,6 +63,30 @@ impl Shown {
         self.access.get(&id.0).copied().unwrap_or_default()
     }
 
+    /// The tables `id` was shown to hide. An entry the file holds that
+    /// does not read, which only a hand could write, cannot say what it
+    /// hid: an "All except" list with one is taken for hiding every table,
+    /// and an "Only" list leaves it out. Either way a change against it is
+    /// held, and what is in force meanwhile hides no less than was shown.
+    pub fn tables_of(&self, id: &ConnectionId) -> Tables {
+        let read = |written: &[String]| -> Option<Vec<TableName>> {
+            written.iter().map(|entry| entry.parse().ok()).collect()
+        };
+        match self.tables.get(&id.0) {
+            None => Tables::All,
+            Some(ShownTables::Hidden(written)) => match read(written) {
+                Some(names) => normal(Tables::AllExcept(names)),
+                None => Tables::Only(Vec::new()),
+            },
+            Some(ShownTables::Only(written)) => Tables::Only(
+                written
+                    .iter()
+                    .filter_map(|entry| entry.parse().ok())
+                    .collect(),
+            ),
+        }
+    }
+
     /// What the files say now, `enabled` and `connections`, against what
     /// was shown: the changes that let agents do more, to be held. What
     /// lets them do less is taken into `self` at once; whether anything
@@ -78,8 +117,28 @@ impl Shown {
                 self.set(&saved.id, now);
                 lowered = true;
             }
+            let (was, now) = (
+                self.tables_of(&saved.id),
+                normal(saved.agents.tables.clone()),
+            );
+            if now == was {
+                continue;
+            }
+            match hides_no_less(&now, &was) {
+                true => {
+                    self.set_tables(&saved.id, &now);
+                    lowered = true;
+                }
+                false => raised.push(Raised::Tables {
+                    id: saved.id.clone(),
+                    to: now,
+                }),
+            }
         }
-        // A connection deleted since lets agents do nothing.
+        // A connection deleted since lets agents do nothing. What it hid is
+        // kept: a start that read no connection at all (a file kept aside,
+        // or one of a newer version) deleted nothing, and when it comes
+        // back, a list that hides less is held.
         let before = self.access.len();
         self.access
             .retain(|id, _| connections.iter().any(|saved| saved.id.0 == *id));
@@ -93,6 +152,24 @@ impl Shown {
             match raise {
                 Raised::Server => self.enabled = true,
                 Raised::Access { id, to } => self.set(id, *to),
+                Raised::Tables { id, to } => self.set_tables(id, to),
+            }
+        }
+    }
+
+    fn set_tables(&mut self, id: &ConnectionId, tables: &Tables) {
+        let written = |names: &[TableName]| names.iter().map(ToString::to_string).collect();
+        match normal(tables.clone()) {
+            Tables::All => {
+                self.tables.remove(&id.0);
+            }
+            Tables::AllExcept(names) => {
+                self.tables
+                    .insert(id.0.clone(), ShownTables::Hidden(written(&names)));
+            }
+            Tables::Only(names) => {
+                self.tables
+                    .insert(id.0.clone(), ShownTables::Only(written(&names)));
             }
         }
     }
@@ -109,6 +186,48 @@ impl Shown {
     }
 }
 
+/// `tables`, an empty "All except" list as "All".
+fn normal(tables: Tables) -> Tables {
+    match tables {
+        Tables::AllExcept(names) if names.is_empty() => Tables::All,
+        tables => tables,
+    }
+}
+
+/// Whether agents may read under `now` nothing they could not under `was`:
+/// every table `was` hid is hidden still. An entry without a schema is that
+/// name in every schema, so it covers one with a schema, and not the other
+/// way round. Names are compared without regard to case.
+fn hides_no_less(now: &Tables, was: &Tables) -> bool {
+    let same = |a: &str, b: &str| a.to_lowercase() == b.to_lowercase();
+    // Every table `narrow` names, `wide` names too.
+    let covers = |wide: &TableName, narrow: &TableName| {
+        same(&wide.name, &narrow.name)
+            && match (&wide.schema, &narrow.schema) {
+                (None, _) => true,
+                (Some(wide), Some(narrow)) => same(wide, narrow),
+                (Some(_), None) => false,
+            }
+    };
+    match (was, now) {
+        (Tables::All, _) => true,
+        (_, Tables::All) => false,
+        (Tables::AllExcept(was), Tables::AllExcept(now)) => was
+            .iter()
+            .all(|hidden| now.iter().any(|still| covers(still, hidden))),
+        (Tables::Only(was), Tables::Only(now)) => now
+            .iter()
+            .all(|readable| was.iter().any(|could| covers(could, readable))),
+        // What a list names is not all an "Only" list reads: a partition
+        // or a shadow table goes with the table it is part of, which a
+        // name cannot tell, and a renamed hidden table is readable by its
+        // new name. Only a list that names nothing hides no less.
+        (Tables::AllExcept(_), Tables::Only(now)) => now.is_empty(),
+        // Every table that is not named was hidden, and is readable now.
+        (Tables::Only(_), Tables::AllExcept(_)) => false,
+    }
+}
+
 #[cfg(test)]
 mod tests {
     use super::*;
diff --git a/src/app.rs b/src/app.rs
index 8efec54..78dffec 100644
--- a/src/app.rs
+++ b/src/app.rs
@@ -2003,7 +2003,19 @@ impl App {
                 }
             }
             Action::DuplicateConnection(id) => {
-                if self.connections.duplicate(&id).is_some() {
+                // The copy hides what is in force: where a change made
+                // outside that hides less is held, what was shown.
+                let in_force = self
+                    .connections
+                    .get(&id)
+                    .map(|saved| self.agents_tables(saved));
+                if let Some(copy) = self.connections.duplicate(&id) {
+                    if let (Some(tables), Some(mut saved)) =
+                        (in_force, self.connections.get(&copy).cloned())
+                    {
+                        saved.agents.tables = tables;
+                        self.connections.upsert(saved);
+                    }
                     self.save_connections();
                 }
             }
@@ -3291,6 +3303,20 @@ impl App {
         }
     }
 
+    /// The tables agents may not read on `saved` now: its own, or what was
+    /// shown where a change made outside the app that hides less is held.
+    pub fn agents_tables(&self, saved: &SavedConnection) -> tabletist_db::agent::Tables {
+        use crate::agents_shown::Raised;
+        let held = self
+            .agents_held
+            .iter()
+            .any(|raise| matches!(raise, Raised::Tables { id, .. } if *id == saved.id));
+        match held {
+            true => self.agents_shown.tables_of(&saved.id),
+            false => saved.agents.tables.clone(),
+        }
+    }
+
     /// Compares what the files let agents do with what was last shown:
     /// what lets them do more is held, what lets them do less is taken and
     /// remembered. Where agents cannot reach the app nothing is held.
@@ -3357,11 +3383,16 @@ impl App {
         }
         let mut changed = false;
         for raise in back {
-            let Raised::Access { id, .. } = raise else {
-                continue;
+            let (id, access) = match raise {
+                Raised::Server => continue,
+                Raised::Access { id, .. } => (id, true),
+                Raised::Tables { id, .. } => (id, false),
             };
             if let Some(mut saved) = self.connections.get(&id).cloned() {
-                saved.agents.access = self.agents_shown.access_of(&id);
+                match access {
+                    true => saved.agents.access = self.agents_shown.access_of(&id),
+                    false => saved.agents.tables = self.agents_shown.tables_of(&id),
+                }
                 self.connections.upsert(saved);
                 changed = true;
             }
@@ -3421,6 +3452,7 @@ impl App {
             .map(|saved| {
                 let mut now = saved.clone();
                 now.agents.access = self.agents_access(saved);
+                now.agents.tables = self.agents_tables(saved);
                 now
             })
             .collect();
@@ -3624,6 +3656,11 @@ impl App {
         // their own, and one they raised is shown, never held. The access
         // the dialog opened with keeps what the file says now: a change
         // made outside the app stays held, and one put back stays back.
+        // The tables have no control in the dialog yet: they are what the
+        // file says now, held or not.
+        if let Some(was) = self.connections.get(&saved.id) {
+            saved.agents.tables = was.agents.tables.clone();
+        }
         match self.connections.get(&saved.id) {
             Some(was) if saved.agents.access == opened => {
                 saved.agents.access = was.agents.access;
diff --git a/src/connections.rs b/src/connections.rs
index b13420e..0c7e6f4 100644
--- a/src/connections.rs
+++ b/src/connections.rs
@@ -60,14 +60,16 @@ impl AgentAccess {
 
 /// A connection's settings for agents. A connection that was never given
 /// any is Off, with the limits every connection starts with.
-#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
-#[serde(default)]
+#[derive(Debug, Clone, PartialEq, Eq)]
 pub struct Agents {
     pub access: AgentAccess,
     /// The most rows of an answer.
     pub rows: u32,
     /// How long a statement of an agent's may run.
     pub timeout_secs: u32,
+    /// The tables agents may not read: all may be read, all but a list,
+    /// or only a list. An "All except" list that is empty is "All".
+    pub tables: tabletist_db::agent::Tables,
 }
 
 /// The most rows an agent's answer may be given: half of what the SQL
@@ -83,6 +85,7 @@ impl Default for Agents {
             access: AgentAccess::Off,
             rows: 500,
             timeout_secs: 10,
+            tables: tabletist_db::agent::Tables::All,
         }
     }
 }
@@ -111,7 +114,7 @@ pub struct SavedConnection {
     pub spec: ConnectSpec,
     /// What agents may do on it. The JSON of older versions has none, and
     /// reads as Off.
-    #[serde(default, skip_serializing_if = "Agents::is_default")]
+    #[serde(skip)]
     pub agents: Agents,
 }
 
diff --git a/src/connections/file.rs b/src/connections/file.rs
index 1359e4b..566fdf1 100644
--- a/src/connections/file.rs
+++ b/src/connections/file.rs
@@ -11,6 +11,7 @@ use std::collections::HashSet;
 use std::path::{Path, PathBuf};
 
 use serde::Deserialize;
+use tabletist_db::agent::{TableName, Tables};
 use tabletist_db::{ConnectSpec, Driver, SshAuth, SshSpec, TlsMode};
 
 use super::{
@@ -21,10 +22,15 @@ use crate::env::Environment;
 use crate::util::toml_string;
 
 /// The file's format. 3 is the first that is TOML: 1 and 2 were
-/// `connections.json`. 4 has a connection's settings for agents, and a file
-/// that has none is written as 3: a Tabletist that does not know them
-/// refuses only a file it would lose them from.
-pub(super) const VERSION: u32 = 4;
+/// `connections.json`. 4 has a connection's settings for agents, and 5 the
+/// tables they may not read. A file is written as the oldest that holds
+/// what it has: a Tabletist that does not know a key refuses only a file it
+/// would lose it from, and one that would read agents' tables as all of
+/// them never reads the file.
+pub(super) const VERSION: u32 = 5;
+
+/// The version that has agents' settings and not their tables.
+const AGENTS_VERSION: u32 = 4;
 
 /// The first version of this file.
 const FIRST_VERSION: u32 = 3;
@@ -114,6 +120,10 @@ struct AgentsTable {
     access: AgentAccess,
     rows: Option<u32>,
     timeout_secs: Option<u32>,
+    /// The tables agents may not read, every other being readable.
+    hidden: Option<Vec<String>>,
+    /// The tables agents may read, and no others.
+    only: Option<Vec<String>>,
 }
 
 #[derive(Deserialize)]
@@ -246,10 +256,36 @@ impl Entry {
 impl AgentsTable {
     fn agents(self, connection: &str) -> Result<Agents, String> {
         let starts = Agents::default();
+        // An entry by its place in the list: a value that does not read
+        // is not repeated, as no other in this file is.
+        let entries = |key: &str, written: Vec<String>| {
+            written
+                .iter()
+                .enumerate()
+                .map(|(at, entry)| {
+                    entry.parse::<TableName>().map_err(|why| {
+                        let place = at + 1;
+                        format!("connection {connection:?}: agents' `{key}` entry {place} {why}")
+                    })
+                })
+                .collect::<Result<Vec<_>, String>>()
+        };
+        let tables = match (self.hidden, self.only) {
+            (Some(_), Some(_)) => {
+                return Err(format!(
+                    "connection {connection:?}: agents have `hidden` or `only`, not both"
+                ));
+            }
+            (Some(hidden), None) if hidden.is_empty() => Tables::All,
+            (Some(hidden), None) => Tables::AllExcept(entries("hidden", hidden)?),
+            (None, Some(only)) => Tables::Only(entries("only", only)?),
+            (None, None) => Tables::All,
+        };
         let agents = Agents {
             access: self.access,
             rows: self.rows.unwrap_or(starts.rows),
             timeout_secs: self.timeout_secs.unwrap_or(starts.timeout_secs),
+            tables,
         };
         if !(1..=MAX_AGENT_ROWS).contains(&agents.rows) {
             return Err(format!(
@@ -354,9 +390,14 @@ fn tls(mode: TlsMode) -> &'static str {
 /// except a server's own (host, port, user, database, TLS), which a person
 /// looks for.
 pub(super) fn write(connections: &[SavedConnection]) -> std::io::Result<String> {
-    let version = match connections.iter().any(|saved| !saved.agents.is_default()) {
-        true => VERSION,
-        false => FIRST_VERSION,
+    let hides = connections
+        .iter()
+        .any(|saved| saved.agents.tables.hides_anything());
+    let agents = connections.iter().any(|saved| !saved.agents.is_default());
+    let version = match (hides, agents) {
+        (true, _) => VERSION,
+        (false, true) => AGENTS_VERSION,
+        (false, false) => FIRST_VERSION,
     };
     let mut out = format!("{HEADER}version = {version}\n");
     for connection in connections {
@@ -410,6 +451,12 @@ pub(super) fn write(connections: &[SavedConnection]) -> std::io::Result<String>
             lines.text("access", connection.agents.access.key());
             lines.bare("rows", connection.agents.rows);
             lines.bare("timeout_secs", connection.agents.timeout_secs);
+            match &connection.agents.tables {
+                Tables::All => {}
+                Tables::AllExcept(names) if names.is_empty() => {}
+                Tables::AllExcept(names) => lines.bare("hidden", entries(names)),
+                Tables::Only(names) => lines.bare("only", entries(names)),
+            }
             out.push_str("\n[connections.agents]\n");
             lines.write(&mut out);
         }
@@ -417,6 +464,15 @@ pub(super) fn write(connections: &[SavedConnection]) -> std::io::Result<String>
     Ok(out)
 }
 
+/// Tables as a list of TOML strings, each written as the person writes it.
+fn entries(names: &[TableName]) -> String {
+    let written: Vec<String> = names
+        .iter()
+        .map(|name| toml_string(&name.to_string()))
+        .collect();
+    format!("[{}]", written.join(", "))
+}
+
 /// A connection's SSH tunnel, as a table of its own.
 fn tunnel(ssh: &SshSpec, out: &mut String) -> std::io::Result<()> {
     let mut lines = Lines::default();
diff --git a/src/model.rs b/src/model.rs
index 51dde45..2b350a5 100644
--- a/src/model.rs
+++ b/src/model.rs
@@ -1288,7 +1288,7 @@ impl ConnectionForm {
             name: saved.name.clone(),
             environment: Some(saved.environment),
             read_only: saved.read_only,
-            agents: saved.agents,
+            agents: saved.agents.clone(),
             agents_opened: saved.agents.access,
             agents_rows: saved.agents.rows.to_string(),
             agents_timeout: saved.agents.timeout_secs.to_string(),
@@ -1570,10 +1570,12 @@ impl ConnectionForm {
                     "Statement timeout is a number of seconds from 1 to {MAX_AGENT_TIMEOUT_SECS}."
                 )
             })?;
+        // The tables have no field here yet: the ones the form opened with.
         Ok(crate::connections::Agents {
             access: self.agents.access,
             rows,
             timeout_secs,
+            tables: self.agents.tables.clone(),
         })
     }
 }
diff --git a/src/ui/mod.rs b/src/ui/mod.rs
index f43ceb1..1159b8b 100644
--- a/src/ui/mod.rs
+++ b/src/ui/mod.rs
@@ -188,6 +188,13 @@ pub fn agents_held_said(app: &App) -> Option<String> {
     let locale = app.locale;
     let saved = &app.connections.connections;
     let names = crate::backend::agent_names(saved);
+    let called = |id: &crate::connections::ConnectionId| {
+        saved
+            .iter()
+            .zip(&names)
+            .find(|(saved, _)| saved.id == *id)
+            .map_or(id.0.clone(), |(_, name)| name.clone())
+    };
     let parts: Vec<String> = app
         .agents_held
         .iter()
@@ -196,12 +203,13 @@ pub fn agents_held_said(app: &App) -> Option<String> {
                 crate::i18n::gettext(locale, "the agent server was turned on").into_owned()
             }
             Raised::Access { id, .. } => {
-                let name = saved
-                    .iter()
-                    .zip(&names)
-                    .find(|(saved, _)| saved.id == *id)
-                    .map_or(id.0.as_str(), |(_, name)| name.as_str());
-                crate::i18n::gettext(locale, "agents may read {name}").replace("{name}", name)
+                let name = called(id);
+                crate::i18n::gettext(locale, "agents may read {name}").replace("{name}", &name)
+            }
+            Raised::Tables { id, .. } => {
+                let name = called(id);
+                crate::i18n::gettext(locale, "agents may read more of {name}")
+                    .replace("{name}", &name)
             }
         })
         .collect();
````

- [ ] **Step 4: Run the tests**

Run: `cargo test --locked -p tabletist-db --lib an_entry`, then `cargo test --locked --lib -- connections:: agents_shown a_change_outside_that_hides_less`

Expected: PASS.

```text
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 482 filtered out
test result: ok. 47 passed; 0 failed; 0 ignored; 0 measured; 2505 filtered out
```

- [ ] **Step 5: Commit**

```bash
cargo fmt --all
cargo clippy --locked --workspace --all-targets -- -D warnings
git add crates/tabletist-db/src/agent/hidden.rs src
git commit -m "Keep the tables agents may not read in connections.toml, and hold a change that hides less"
```

---

### Task 2: The app passes them to the server

- `Agentable` carries the connection's tables, and the gateway's `Listed` gives them to the server, which since 1c-2a refuses what they hide and leaves it out of what it says of the schema.

**Files:**
- Modify: `src/backend/agents.rs`

- [ ] **Step 1: Write the failing test**

````diff
diff --git a/src/backend/agents.rs b/src/backend/agents.rs
index 8899b1c..ec4eaaa 100644
--- a/src/backend/agents.rs
+++ b/src/backend/agents.rs
@@ -1109,6 +1109,41 @@ mod tests {
         assert_eq!(std::ptr::from_ref(again.connection().unwrap()), opened);
     }
 
+    #[tokio::test]
+    async fn the_gateway_lists_what_a_connection_hides_and_its_reads_keep_to_it() {
+        let dir = tempfile::tempdir().unwrap();
+        let spec = ConnectSpec::sqlite(bookshop(dir.path(), "bookshop.db"));
+        let mut shop = saved("shop", "Bookshop", Environment::Dev, spec);
+        let customers = Tables::AllExcept(vec!["customers".parse().unwrap()]);
+        shop.agents.tables = customers.clone();
+        let gateway = gateway(shop);
+        assert_eq!(gateway.connections()[0].tables, customers);
+        let stop = CancellationToken::new();
+        let session = gateway.open("shop", &stop).await.unwrap();
+        let read = check_read(Dialect::Sqlite, "SELECT * FROM customers", &[]).unwrap();
+        let refused = session
+            .read(
+                &read,
+                Limits {
+                    rows: 500,
+                    timeout: Duration::from_secs(10),
+                },
+                &customers,
+            )
+            .await;
+        assert!(
+            matches!(
+                refused,
+                Err(Failure::Database(tabletist_db::Error::Agent(
+                    tabletist_db::agent::Refusal::TableHidden { .. }
+                )))
+            ),
+            "{refused:?}"
+        );
+        let hidden = session.hidden(&customers).await.unwrap();
+        assert!(hidden.hides("main", "customers") && !hidden.hides("main", "books"));
+    }
+
     #[tokio::test]
     async fn a_connection_that_needs_a_person_is_not_opened() {
         let dir = tempfile::tempdir().unwrap();
````

- [ ] **Step 2: Run it to see it fail**

Run: `cargo test --locked --lib the_gateway_lists_what`

Expected: FAIL. The gateway still lists every connection as hiding nothing:

```text
test backend::agents::tests::the_gateway_lists_what_a_connection_hides_and_its_reads_keep_to_it ... FAILED
  left: All
 right: AllExcept([TableName { schema: None, name: "customers" }])
```

- [ ] **Step 3: The tables through the gateway**

````diff
diff --git a/src/backend/agents.rs b/src/backend/agents.rs
index ec4eaaa..ecc3aed 100644
--- a/src/backend/agents.rs
+++ b/src/backend/agents.rs
@@ -45,6 +45,8 @@ pub struct Agentable {
     pub password: PasswordMode,
     pub ssh_secret: PasswordMode,
     pub limits: Limits,
+    /// What agents may not read of it.
+    pub tables: Tables,
 }
 
 /// The connections of `saved` that agents may use, in its order. One that
@@ -75,6 +77,7 @@ pub fn agentable(saved: &[SavedConnection]) -> Vec<Agentable> {
                 rows: saved.agents.rows,
                 timeout: Duration::from_secs(u64::from(saved.agents.timeout_secs)),
             },
+            tables: saved.agents.tables.clone(),
         })
         .collect()
 }
@@ -654,9 +657,7 @@ impl Gateway for AppGateway {
                 name: agentable.name.clone(),
                 dialect: agentable.spec.driver.dialect(),
                 limits: agentable.limits,
-                // What the connection hides comes with its setting in
-                // connections.toml (1c-2b): until then, nothing.
-                tables: Tables::All,
+                tables: agentable.tables.clone(),
             })
             .collect()
     }
````

- [ ] **Step 4: Run the tests**

Run: `cargo test --locked --lib backend::agents`

Expected: PASS.

```text
test result: ok. 30 passed; 0 failed; 0 ignored; 0 measured; 2523 filtered out
```

- [ ] **Step 5: Commit**

```bash
cargo fmt --all
cargo clippy --locked --workspace --all-targets -- -D warnings
git add src/backend/agents.rs
git commit -m "Give the server the tables each connection hides"
```

---

### Task 3: The reference page

- `docs/_reference/settings-and-files.md`: the two lists, how an entry is written, what is hidden with a table, the system catalogs, a renamed table, version 5, and that a change that hides less is held.

**Files:**
- Modify: `docs/_reference/settings-and-files.md`

- [ ] **Step 1: The page**

````diff
diff --git a/docs/_reference/settings-and-files.md b/docs/_reference/settings-and-files.md
index 17f9516..3e49552 100644
--- a/docs/_reference/settings-and-files.md
+++ b/docs/_reference/settings-and-files.md
@@ -179,8 +179,31 @@ What agents may do on a connection is its `[connections.agents]` table:
 `access` (`off` or `read`), `rows`, the most rows of an answer (1 to 5000),
 and `timeout_secs`, how long a statement may run (1 to 60). Without the
 table, or a key of it, a connection is `off`, with 500 rows and 10 seconds.
-A file in which a connection has the table says `version = 4`, and one in
-which none has it stays `version = 3`. Agents use the table only while
+
+The table can also say which tables agents may not read, with one of two
+lists:
+
+```toml
+[connections.agents]
+access = "read"
+hidden = ["customers", "sales.orders"]
+```
+
+`hidden` names the tables agents may not read, and every other one they
+may. `only` names the tables they may read, and no others, the ones made
+later included: `only = []` hides every table. A name without a schema is
+that table in every schema, and a name with a dot or a double quote in it
+is written in double quotes, with the quote doubled: `sales."q1.orders"`.
+A view over a hidden table is hidden with it, and so are a partition and a
+child table of one. While anything is hidden, agents may not read the
+system catalogs either (`information_schema`, `pg_catalog`, `mysql`,
+`sqlite_schema` and the rest). If a table that `hidden` names is renamed
+or dropped, agents may not use the connection at all until you take the
+name out of the list or put the right one in.
+
+A file in which a connection hides a table says `version = 5`, one in
+which a connection has the table and hides nothing says `version = 4`,
+and one in which none has it stays `version = 3`. Agents use the table only while
 `[mcp] enabled` is `true` in `settings.toml`. **Stop agents here**, in a
 connection's Agents tab, stops the agents working on it and sets its
 `access` to `off`, so that an agent that connects again does not see it.
@@ -188,13 +211,16 @@ connection's Agents tab, stops the agents working on it and sets its
 ### Agent access changed outside Tabletist
 
 A change to either file that lets agents do more than Tabletist last
-showed you is not applied at once: turning `[mcp] enabled` on, or a
-connection's `access` up from `off`. Tabletist says what changed in a bar
-under the top of the window, and until you answer, agents may do what they
-could before. **Keep** applies the change. **Put back** undoes it in the
-files: it turns `[mcp] enabled` off, or sets the connection's `access`
-back to `off`, and leaves your other settings and connections as they
-are. A change that lets agents do less applies at once.
+showed you is not applied at once: turning `[mcp] enabled` on, a
+connection's `access` up from `off`, or a connection hiding less: a name
+taken out of `hidden` or put into `only`, or a `hidden` list made an
+`only` one. Tabletist says what
+changed in a bar under the top of the window, and until you answer,
+agents may do what they could before. **Keep** applies the change. **Put
+back** undoes it in the files: it turns `[mcp] enabled` off, sets the
+connection's `access` back to `off`, or puts its tables back as they were,
+and leaves your other settings and connections as they are. A change that
+lets agents do less applies at once.
 
 Turning agents on in Tabletist itself, with **Turn on** in a connection's
 Agents tab or the switch in **Settings › Agents**, is never held. Tabletist keeps what it showed you in
````

- [ ] **Step 2: Check it**

Run: `docs/bin/check`, `grep -c "$(printf '\342\200\224')" docs/_reference/settings-and-files.md` (a count of em dashes) and `cargo test --locked --lib the_websites_example`, which reads the page's example file

Expected: the site checks clean, no em dash (`0`), and the page's example reads:

```text
HTML-Proofer finished successfully.
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 2552 filtered out
```

- [ ] **Step 3: Commit**

```bash
git add docs/_reference/settings-and-files.md
git commit -m "Say in the reference how a connection hides tables from agents"
```

---

### Task 4: The master plan

- C.1 and C.2 as built; G's row for 1c.

**Files:**
- Modify: `docs/superpowers/plans/2026-10-13-mcp-server.md`

- [ ] **Step 1: The edits**

````diff
diff --git a/docs/superpowers/plans/2026-10-13-mcp-server.md b/docs/superpowers/plans/2026-10-13-mcp-server.md
index 209336b..f001daf 100644
--- a/docs/superpowers/plans/2026-10-13-mcp-server.md
+++ b/docs/superpowers/plans/2026-10-13-mcp-server.md
@@ -1333,6 +1333,18 @@ an agent's settings are. Where they are kept changes:
   The table is written whenever anything in it is not the default, not only
   when access is on (`Agents::is_off` below would drop it): a stop loses
   neither the level nor, from 1c on, the hidden tables.
+- **As built** (`2026-10-16-mcp-server-1c-2b-setting.md`): the tables are `hidden = [...]` or `only = [...]`
+  in the same table, not both, and not the `tables` key the JSON below
+  has: a list says which it is. An entry is `customers` or
+  `sales.customers`, a part with a dot or a double quote in double quotes,
+  the quote doubled. `hidden = []` hides nothing; `only = []` hides every
+  table. A file in which a connection hides something is version 5, so
+  that a Tabletist that would read every table as readable never reads
+  it; one that has agents' settings and hides nothing stays 4. An entry
+  that does not read is refused by its place in the list, not its words,
+  as every value of the file is. The editor has no control for the tables
+  yet (1c-3): its save keeps what the file says then, as it does for an
+  access it did not change.
 
 **Per connection: `connections.json`.** `SavedConnection`
 (`src/connections.rs:35-50`) gains one field:
@@ -1551,6 +1563,22 @@ its second part (1b-3b), right after **Turn on** (`2026-10-14-mcp-server-1b-3a-a
 - What the app last showed is `agents_shown.json` in the state directory:
   whether the server is on, and each connection's access. `listen` and the
   lists come with what they are for (1d, 1c, 3a).
+  **As built for 1c** (`2026-10-16-mcp-server-1c-2b-setting.md`): each connection's tables are there too, a
+  connection without an entry hiding none. A change that hides less is
+  held (`Raised::Tables`, "agents may read more of Bookshop · dev"), and
+  what was shown is in force until the person answers; one that hides
+  more, or the same in other words, is taken. An entry without a schema
+  is that table in every schema: it covers one with a schema, not the
+  other way round, so a change from `sales.customers` to `customers` in an
+  "Only" list is held. From "Only" to "All except" is always held: every
+  table the list did not name was hidden. So is "All except" to "Only" a
+  list, found by this run's review: an "Only" list reads the partitions
+  and shadow tables of what it names, and a hidden table renamed by its
+  new name, and no name says which. **Put back** writes the shown tables
+  back to the file. What a connection hid is kept while it is not in the
+  file, as after a start that read no connection; a shown entry that does
+  not read hides every table; a copy of a connection whose change is held
+  takes what is in force.
 - **A file that is not there, or cannot be read, shows nothing.** So the
   first start of the version that checks holds whatever was turned on by
   hand, once, and a person who set agents up through the files keeps it
@@ -1578,7 +1606,8 @@ its second part (1b-3b), right after **Turn on** (`2026-10-14-mcp-server-1b-3a-a
 - Only the server and each connection's access are compared. A Read
   connection pointed at another server by hand is not held: an agent
   with the person's shell gains little by it, and the settings and
-  connection files are the person's to edit.
+  connection files are the person's to edit. From 1c-2b (`2026-10-16-mcp-server-1c-2b-setting.md`) each
+  connection's tables are compared too.
 
 ### C.3 Hidden tables and masked columns when the schema changes
 
@@ -2369,7 +2398,7 @@ is off until the person turns it on.
 | 1a. The enforcement core | 3 | Done: pull requests 122, 123 and 124 |
 | 1b. The server over stdio, with its interface | 9, below | Built (`2026-10-13-mcp-server-1b-1-server.md`, `2026-10-14-mcp-server-1b-2a-gateway.md`, `2026-10-14-mcp-server-1b-2b-stdio.md`, `2026-10-14-mcp-server-1b-3a-agents-tab.md`, `2026-10-14-mcp-server-1b-3b-changes-outside.md`, `2026-10-14-mcp-server-1b-4a-activity.md`, `2026-10-14-mcp-server-1b-4b-log.md`, `2026-10-15-mcp-server-1b-5a-access.md` and `2026-10-15-mcp-server-1b-5b-settings.md`) |
 | 1b on Windows: the way in | 1 | Built (`2026-10-15-mcp-server-1b-6-windows.md`) |
-| 1c. Hidden tables | 4, below | The checks and the server's side built (`2026-10-15-mcp-server-1c-1-checks.md`, `2026-10-15-mcp-server-1c-2a-server.md`). The setting is next |
+| 1c. Hidden tables | 4, below | The checks, the server's side and the setting built (`2026-10-15-mcp-server-1c-1-checks.md`, `2026-10-15-mcp-server-1c-2a-server.md`, `2026-10-16-mcp-server-1c-2b-setting.md`). The editor's Tables control is next |
 | 1d. HTTP with tokens | 1 | Dropped for now (2026-10-15) |
 
 The move of the connections to `connections.toml` came between 1a and 1b
@@ -2491,7 +2520,7 @@ build; no console target.
 - The second cut in two, the same day, the server first: 1c-2a, the
   server's side with nothing to set it (`2026-10-15-mcp-server-1c-2a-server.md`); 1c-2b, the keys in
   `connections.toml`, the app passing them, changes made outside the app
-  held, and the reference page for the keys.
+  held, and the reference page for the keys (`2026-10-16-mcp-server-1c-2b-setting.md`).
 - Files: `crates/tabletist-db/src/catalog.rs`, `pg.rs`, `mysql.rs`,
   `sqlite.rs` (what views read, routines, partitions), `agent/` (names,
   dependencies, the plan check, the system catalogs), `tabletist-sqlite-ffi`
````

- [ ] **Step 2: Check the whole of it**

Run: `cargo fmt --all --check`, clippy as above, `RUSTDOCFLAGS='-D warnings' cargo doc --locked --workspace --no-deps`, the count of em dashes in the master plan, and `cargo test --locked --workspace --all-targets` with both servers

Expected: fmt and clippy clean, the documentation builds without a warning, no em dash (`0`), and:

```text
3528 passed, 0 failed, 22 ignored
```

- [ ] **Step 3: Commit**

```bash
git add docs/superpowers/plans/2026-10-13-mcp-server.md
git commit -m "Say in the MCP plan what Phase 1c's run 2b built"
```

## Self-review

- **Against the master plan.** C.1's per-connection lists (Task 1, as `hidden` and `only`), C.2's "a table unhidden" held (Task 1), C.3's "Only" hiding tables made later (the server's since 1c-2a, and the reference page), G's 1c-2b (Tasks 1 to 3).
- **A review before the plan.** A reviewer ran the draft and found five ways a narrower setting could go wider without the person's answer, each proved with a test (one of them on the PostgreSQL Bookshop): a change from hiding a partition to "Only" its parent, and from hiding a table later renamed to "Only" the new name, both taken where they read what was hidden; a copy of a connection whose change was held, which took the file's wider list; a start that read no connection, which forgot every connection's shown tables; and a shown entry that did not read, which widened what was shown. Fixed: "All except" to "Only" a list is held, what a connection hid is kept while it is not in the file, a copy takes what is in force, and an unreadable shown list hides every table, each with a test. It found sound: every other pair of `hides_no_less`, every path that writes the file or sends the backend a policy, an `agents_shown.json` from before this run, and every entry of up to four characters over a list of hard ones round-tripping through the file.
- **Left, and said.** The editor's Tables control and the guide are 1c-3's. A person who switches a list from `hidden` to `only` in the file gets the bar, also where the new list reads nothing that was hidden: no name can say so.
- **Placeholders.** None: every step has its diff or its command and what it prints.
- **Names.** `Agents::tables`, `ShownTables`, `Raised::Tables`, `tables_of`, `hides_no_less`, `agents_tables` and `Agentable::tables` are the same in every task that uses them.
