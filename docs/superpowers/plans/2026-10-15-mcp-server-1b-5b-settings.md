# MCP server, Phase 1b, run 5b: Settings › Agents. Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Settings gets its second pane, Agents, in both looks: the server's switch, whether it runs and who is connected, how to connect Claude Code, Cursor or another agent with **Copy**, where access is set with **Open Connections**, what keeps agents within bounds, the log's days with **Show log**, and the notification. The Agents tab's **Settings…** and **Other agents…** open it. The website gets its guide to Agents. With this run Phase 1b is whole on Linux and macOS.

**Architecture:** `SettingsDialog` gains `pane: SettingsPane { General, Agents }` (Agents only where `endpoint::AVAILABLE`) and the agent whose setup it shows. The `[mcp]` keys become options of the window (`OptionId::AgentsOn`, `LogDays`, `AgentsNotify`), so the terminal look's cursor, its keys and the file pane work on them as on General's; its Agents pane has two rows that are no options (`SettingsRow::AgentsStatus`, `AgentsAccess`), and Enter opens what a row shows. The switch shows what is in force (`App::agents_on`), and turning it on goes through `turn_agents_on`, which keeps a change made outside the app instead of writing it again. `endpoint::agent_setup_ready` gives what connects each agent, worked out once off the UI thread as the command is now.

**Tech Stack:** Rust 2024, egui. Nothing new is built.

---

## Where this sits

Phase 1b of `docs/superpowers/plans/2026-10-13-mcp-server.md` is cut into runs (section G). Nine are merged, through the fifth run's first part (letting agents in, per connection).

1. Done: the server, the gateway, the files and stdio, the Agents tab, changes made outside, what agents are doing, the log, letting agents in per connection.
2. **This run, 5b: Settings › Agents**, the last of Phase 1b on Linux and macOS, as one run (agreed on 2026-10-15).
3. Then Windows' way in.

## Before you start

- Work on a branch of `main`. One commit for each task, in the order below. Every commit passes `cargo fmt --all --check`, `cargo clippy --locked --workspace --all-targets -- -D warnings` and `cargo test --locked --workspace --all-targets`.
- On Igor's machine `cargo` is `~/.cargo/bin/cargo` (the mise shim fails), and a build in a worktree needs `CARGO_TARGET_DIR` set to that worktree's `target/`.
- Read `AGENTS.md`: views draw and push `Action`s and `App::apply` changes state; a key is spelled in the keymap and nowhere else (`tests/keys.rs`).
- The diffs below apply with `git apply`. Run `cargo fmt --all` before each commit: the code here is already formatted.
- No `Cargo.toml` or `Cargo.lock` changes. Nothing needs PostgreSQL or MySQL; the full run at the end prints "skipped" for their tests without the variables.

## What was run before this plan was written

Every task's state was built, passed `cargo fmt --check` and clippy with `-D warnings`, and its tests failed and then passed as each step says. The last state passed `cargo test --locked --workspace --all-targets` (3,482 tests, none failed), `RUSTDOCFLAGS='-D warnings' cargo doc --locked --workspace --no-deps` and `docs/bin/check`. Settings › Agents was rendered headless in both looks, light and dark, at 1440 by 900, 1024 by 640 and 1000 by 700, for Claude Code and Cursor, with the server on and off, and looked at; the scene was thrown away. Not run: macOS and Windows, which CI runs; on Windows Settings lists General alone.

## What this run settles

| Question | Decision | Why |
|---|---|---|
| One run or two | One: the pane, the tab's two buttons and the guide | Agreed on 2026-10-15 |
| What "Connect an agent" shows for each agent | Claude Code: its command. Cursor: the entry for its `mcp.json`. Other: the command `tabletist mcp`. Each with **Copy**; Omarchy keeps the board's box, `y`, and "cursor, others" with the command | Agreed on 2026-10-15. The board draws only Claude Code's; tokens for Other come with HTTP (1d) |
| What the switch shows and does | What is in force: off while a change made outside that turned the server on is held. On is `turn_agents_on`, the person's own, which keeps what was held; off ends every session | 1b-3b's rule for the app's own controls. Off is "Stop all agents" (question 11), built as the plan has it |
| The `[mcp]` keys in the window | Options of the window like General's (`OptionId`), so the terminal look's cursor, `h`/`l`, Space, `R` and the file pane's marked line work on them | One way for every option. `OptionId::ALL` is all of them; each pane has its own list |
| Rows of the terminal look's Agents pane | The switch, the status, the access, the log's days and the notification are rows; `enter` shows the Agents tab from the status, Connections from the access, the log from the log's days. The command's box, "cursor, others" and the note are not rows | D.1: "the rows with the cursor, `y` to copy the command, `enter` on the access row to open Connections" |
| Switching panes on Omarchy | `ctrl+h` and `ctrl+l`, said at the nav's foot | The board's nav: "ctrl+h/l pane" |
| A held Enter | The Agents tab's Enter, which shows where an agent elsewhere works, acts on a press, not on what a held one repeats. The Settings screen takes only a press too | Found by the review: Enter held on the status row showed the tab, then went on to another connection's window and connected it |
| Show Agents tab and Show log | They close Settings and show the tab of the window in front, or with Connections in front of the last window open, the way Review does (`App::agents_window`). With no window open, disabled with "Open a connection to see its agents" | D.1, changed on 2026-10-13 |
| Reset to defaults on the Agents pane | Puts back the log's 30 days and the notification, and leaves the server's switch as it is. A Reset asked on one pane is dropped when the pane changes | Found by the review: off, the switch would end every agent's session for a question about defaults. Omarchy's `R` on the switch's own row still puts it back, as it does any option |
| Omarchy's rows in a narrow window | A hint is whole or left out, as on General; the log's row says "enter shows it" only where there is a window to show it in. The columns are measured with the status as it may come to read, so they stay put when the server turns on. The footer's hints give way from the end, all but `esc close` | Found by the review: the columns moved, and at 720 wide "esc close" was off the window |
| The log's days | A menu of 7, 30, 90 and 365 days, with a number set in the file in its place among them; Omarchy steps through the same | The board's "30 days" menu. The file keeps 1 to 3650 |
| What the boards say of writes, HTTP and the agent list | Left out until they are built: "Off or Read", the safety box without its sentence on writes, "Notify when an agent connects", "stdio · this computer only" | A sentence about what the app does not do yet would be wrong |
| The Agents tab's **Settings…** and **Other agents…** | Both open Settings › Agents; Omarchy's `s` in both states | D.4. One key for one place |
| The guide's pictures | None in this run; the Settings pictures keep the nav with General alone | Pictures go to GitHub only with Igor's yes |

### What this run leaves out

- The list of agents in Settings, **Remove**, tokens and "Add an agent…" under Other, and the HTTP address on the status line: HTTP, 1d.
- "Read + approved writes", the safety box's sentence on writes, and a notification for a write: Phase 2.
- The board's other panes (Appearance, Editor, Keyboard, AI assistant, Updates).
- New pictures for the website: the guide has none, and the Settings pictures still show the nav with General alone. Pictures are pushed only with Igor's yes.

## File structure

| File | What changes |
|---|---|
| `src/endpoint.rs` | `Agent` and `agent_setup_ready`: Claude Code's command, Cursor's entry for `mcp.json`, the command for others |
| `src/settings.rs` | `OptionId::AgentsOn`, `LogDays`, `AgentsNotify` and their values; `OptionId::GENERAL` and `AGENTS`; `Settings::LOG_DAYS` |
| `src/model.rs` | `SettingsDialog::pane` and `agent`, `SettingsPane`, `SettingsRow`; `ShowAgentSettings`, `ShowSettingsPane`, `ChooseAgentSetup`, `ShowAgentsFromSettings` |
| `src/app.rs` | The actions; the cursor and Reset by pane; the switch through `turn_agents_on`; `App::agents_window`, shared with Review |
| `src/keymap.rs` | `SettingsPane` (`ctrl+h`/`ctrl+l`), `SettingsOpenRow` (`enter`), `SettingsCopyCommand` (`y`); `ShowAgentSettings` (`s` in the Agents tab) |
| `src/ui/settings/mod.rs`, `sheet.rs`, `terminal.rs` | The pane switch in both looks, the new options' controls, the keys by pane |
| `src/ui/settings/agents.rs` | New: the Agents pane as the sheet draws it |
| `src/ui/agent_settings_tests.rs`, `src/ui/mod.rs` | New: the tests of Settings › Agents |
| `src/ui/agents.rs`, `src/ui/keys.rs`, `src/ui/workspace.rs` | **Settings…** and **Other agents…**, `s`, and its hint |
| `docs/_guide/agents.md` | New: the guide to Agents |
| `docs/_guide/omarchy.md`, `docs/_guide/macos.md`, `docs/_reference/settings-and-files.md` | The keys, the pane's options, the guides' order |
| `docs/superpowers/plans/2026-10-13-mcp-server.md` | What this run built |

---

### Task 1: What connects each agent

- `endpoint::Agent { ClaudeCode, Cursor, Other }`, and `agent_setup_ready(agent)`: Claude Code's command as it is now, Cursor's entry for its `mcp.json` (the program, by its whole path where `tabletist` on the `PATH` is not this one, written as JSON writes it, with `"args": ["mcp"]`), and for others the command `tabletist mcp`.
- All three are worked out once, with the command, by `agent_command` on the backend's runtime as it starts, never on the UI thread. `agent_command_ready` is `agent_setup_ready(Agent::ClaudeCode)`.

**Files:**
- Modify: `src/endpoint.rs`

- [ ] **Step 1: Write the failing tests**

````diff
diff --git a/src/endpoint.rs b/src/endpoint.rs
index 880f8c5..5e92a72 100644
--- a/src/endpoint.rs
+++ b/src/endpoint.rs
@@ -320,6 +320,24 @@ mod tests {
             "{command}"
         );
         assert!(command.ends_with(" mcp"), "{command}");
+        // The others start the same relay.
+        let relay = agent_setup_ready(Agent::Other).unwrap();
+        assert_eq!(command, command_for(relay));
+        let cursor = agent_setup_ready(Agent::Cursor).unwrap();
+        assert!(cursor.contains("\"args\": [\"mcp\"]"), "{cursor}");
+    }
+
+    #[test]
+    fn cursors_entry_starts_the_program_by_its_path_as_json_writes_it() {
+        let entry = cursor_entry("tabletist");
+        let read: serde_json::Value = serde_json::from_str(&entry).unwrap();
+        let server = &read["mcpServers"]["tabletist"];
+        assert_eq!(server["command"], "tabletist");
+        assert_eq!(server["args"], serde_json::json!(["mcp"]));
+        // A path a shell would quote is written as JSON writes it.
+        let spaced = "C:\\Program Files\\Tabletist's \"app\"\\tabletist";
+        let read: serde_json::Value = serde_json::from_str(&cursor_entry(spaced)).unwrap();
+        assert_eq!(read["mcpServers"]["tabletist"]["command"], spaced);
     }
 
     #[tokio::test]
````

- [ ] **Step 2: Run them to see that they do not build**

Run: `cargo test --locked --lib endpoint::`

Expected: it does not build. The tests name what is not there yet:

```text
error[E0433]: cannot find type `Agent` in this scope
error[E0425]: cannot find function `agent_setup_ready` in this scope
error[E0425]: cannot find function `cursor_entry` in this scope
error: could not compile `tabletist` (lib test) due to 6 previous errors
```

- [ ] **Step 3: The setups**

````diff
diff --git a/src/endpoint.rs b/src/endpoint.rs
index 5e92a72..f672771 100644
--- a/src/endpoint.rs
+++ b/src/endpoint.rs
@@ -141,30 +141,73 @@ pub async fn read_line<R: AsyncRead + Unpin>(
     }
 }
 
-/// The command, once worked out.
-static COMMAND: std::sync::OnceLock<String> = std::sync::OnceLock::new();
+/// The agents Settings says how to connect: each is told in its own way.
+#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
+pub enum Agent {
+    /// A command to run once.
+    #[default]
+    ClaudeCode,
+    /// An entry of its `mcp.json`.
+    Cursor,
+    /// Any agent that starts an MCP server over stdio: the command it
+    /// starts.
+    Other,
+}
+
+impl Agent {
+    /// In the order Settings shows them.
+    pub const ALL: [Agent; 3] = [Self::ClaudeCode, Self::Cursor, Self::Other];
+}
+
+/// What connects each agent, once worked out.
+struct Setups {
+    claude_code: String,
+    cursor: String,
+    other: String,
+}
+
+static SETUPS: std::sync::OnceLock<Setups> = std::sync::OnceLock::new();
 
 /// [`agent_command`] where it is worked out already: what the UI reads.
 pub fn agent_command_ready() -> Option<&'static str> {
-    COMMAND.get().map(String::as_str)
+    agent_setup_ready(Agent::ClaudeCode)
+}
+
+/// What connects `agent`, where it is worked out already.
+pub fn agent_setup_ready(agent: Agent) -> Option<&'static str> {
+    let setups = SETUPS.get()?;
+    Some(match agent {
+        Agent::ClaudeCode => &setups.claude_code,
+        Agent::Cursor => &setups.cursor,
+        Agent::Other => &setups.other,
+    })
 }
 
 /// The command that connects Claude Code, the agent the boards name, to
 /// this Tabletist: `claude mcp add tabletist -- tabletist mcp`, with this
 /// program's whole path where `tabletist` on the `PATH` is not this
-/// program (the app bundle on macOS). Worked out once, by looking at the
-/// disk: on the backend's runtime as it starts (`Backend::start_with`),
-/// never on the UI thread, which asks [`agent_command_ready`].
+/// program (the app bundle on macOS). Worked out once, with what connects
+/// the other agents, by looking at the disk: on the backend's runtime as
+/// it starts (`Backend::start_with`), never on the UI thread, which asks
+/// [`agent_setup_ready`].
 pub fn agent_command() -> &'static str {
-    COMMAND.get_or_init(|| {
-        let exe = std::env::current_exe().ok();
-        let on_path = std::env::var_os("PATH").and_then(|path| {
-            std::env::split_paths(&path)
-                .map(|dir| dir.join(EXE))
-                .find(|file| file.is_file())
-        });
-        command_for(&relay_for(exe.as_deref(), on_path.as_deref()))
-    })
+    &SETUPS
+        .get_or_init(|| {
+            let exe = std::env::current_exe().ok();
+            let on_path = std::env::var_os("PATH").and_then(|path| {
+                std::env::split_paths(&path)
+                    .map(|dir| dir.join(EXE))
+                    .find(|file| file.is_file())
+            });
+            let program = program_for(exe.as_deref(), on_path.as_deref());
+            let relay = relay_of(&program);
+            Setups {
+                claude_code: command_for(&relay),
+                cursor: cursor_entry(&program),
+                other: relay,
+            }
+        })
+        .claude_code
 }
 
 /// This program's file name.
@@ -179,10 +222,19 @@ fn command_for(relay: &str) -> String {
     format!("claude mcp add tabletist -- {relay}")
 }
 
-/// How an agent starts the relay: `tabletist mcp` when `on_path`, the
-/// program a shell finds by that name, is `exe`, this program; else
-/// `exe`'s whole path, as a shell reads it.
-fn relay_for(exe: Option<&std::path::Path>, on_path: Option<&std::path::Path>) -> String {
+/// Cursor's entry for Tabletist in its `mcp.json`: `program` started with
+/// `mcp`.
+fn cursor_entry(program: &str) -> String {
+    let command = serde_json::to_string(program).unwrap_or_default();
+    format!(
+        "{{\n  \"mcpServers\": {{\n    \"tabletist\": {{ \"command\": {command}, \"args\": [\"mcp\"] }}\n  }}\n}}"
+    )
+}
+
+/// The program an agent starts as the relay: `tabletist` when `on_path`,
+/// the program a shell finds by that name, is `exe`, this program; else
+/// `exe`'s whole path.
+fn program_for(exe: Option<&std::path::Path>, on_path: Option<&std::path::Path>) -> String {
     let same = |a: &std::path::Path, b: &std::path::Path| match (
         std::fs::canonicalize(a),
         std::fs::canonicalize(b),
@@ -192,12 +244,24 @@ fn relay_for(exe: Option<&std::path::Path>, on_path: Option<&std::path::Path>) -
     };
     match exe {
         Some(exe) if !on_path.is_some_and(|found| same(exe, found)) => {
-            format!("{} mcp", shell_word(&exe.to_string_lossy()))
+            exe.to_string_lossy().into_owned()
         }
-        _ => "tabletist mcp".to_owned(),
+        _ => "tabletist".to_owned(),
     }
 }
 
+/// How a shell starts the relay of `program`.
+fn relay_of(program: &str) -> String {
+    format!("{} mcp", shell_word(program))
+}
+
+/// How an agent starts the relay: `tabletist mcp`, or this program's whole
+/// path as a shell reads it (see [`program_for`]).
+#[cfg(test)]
+fn relay_for(exe: Option<&std::path::Path>, on_path: Option<&std::path::Path>) -> String {
+    relay_of(&program_for(exe, on_path))
+}
+
 /// `word` as a POSIX shell reads it back: as it is when nothing in it is
 /// special, else in single quotes.
 fn shell_word(word: &str) -> String {
````

- [ ] **Step 4: Run the tests**

Run: the same.

Expected: PASS.

```text
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 2512 filtered out
```

- [ ] **Step 5: Commit**

```bash
cargo fmt --all
cargo clippy --locked --workspace --all-targets -- -D warnings
git add src/endpoint.rs
git commit -m "Say how Claude Code, Cursor and other agents connect"
```

---

### Task 2: Settings › Agents

- The settings: `OptionId::AgentsOn`, `LogDays` and `AgentsNotify`, the `[mcp]` keys, with their values; `OptionId::GENERAL` and `OptionId::AGENTS` are the panes' (`ALL` is every option); the log steps through `Settings::LOG_DAYS`, 7, 30, 90 and 365 days.
- The model: `SettingsDialog { pane, row, resetting, agent }`, `SettingsPane::listed()` (General, and Agents where agents can reach the app), `SettingsPane::rows()` for the terminal look's cursor, `SettingsRow` (an option, or the status or access row). `ShowAgentSettings` opens Settings on Agents; `ShowSettingsPane` switches, the cursor back on the first row and a question of Reset dropped; `ChooseAgentSetup`; `ShowAgentsFromSettings(view)` closes Settings and shows the Agents tab of `App::agents_window()` on that view, nothing with no window open.
- The app: the cursor moves among the pane's rows; Reset puts back the options of the pane that asked, but the server's switch (`SettingsPane::reset_options`); `SetOption(AgentsOn(true))` is `turn_agents_on`, the person's own, which keeps a held change and writes nothing when the server is on already. The window shows the switch as `agents_on()` has it.
- The Agents tab's Enter acts on a press, not on what a held one repeats: one held on Settings' status row shows the tab and goes no further (`src/ui/keys.rs`).
- The sheet: the nav's two items; the Agents pane (`src/ui/settings/agents.rs`): the title with the switch, the status box with **Show Agents tab** (disabled with "Open a connection to see its agents" with no window open), Connect an agent with the switch of three, what the agent takes and **Copy**, the Access box with **Open Connections**, How it stays safe, the log's row with its days and **Show log**, and the notification's.
- The terminal look: the nav and its `ctrl+h/l pane`; the Agents pane's rows (the switch, status, the command's box with `y copy`, "cursor, others", access, the log's days, notify, the note); `enter` on the status, access and log rows; `y`; the footer by pane, its hints giving way from the end in a narrow window, all but `esc close`.
- Nothing of it where agents cannot reach the app: Settings lists General alone.

**Files:**
- Create: `src/ui/settings/agents.rs`, `src/ui/agent_settings_tests.rs`
- Modify: `src/settings.rs`, `src/model.rs`, `src/app.rs`, `src/keymap.rs`, `src/ui/keys.rs`, `src/ui/mod.rs`, `src/ui/settings/mod.rs`, `src/ui/settings/sheet.rs`, `src/ui/settings/terminal.rs`

- [ ] **Step 1: Write the failing tests**

````diff
diff --git a/src/app.rs b/src/app.rs
index 9dfb61f..2f0ad24 100644
--- a/src/app.rs
+++ b/src/app.rs
@@ -7151,7 +7151,7 @@ mod tests {
         app.apply(Action::MoveSettingsRow(1));
         assert_eq!(settings_row(&app), Some(1));
         app.apply(Action::MoveSettingsRow(99));
-        assert_eq!(settings_row(&app), Some(OptionId::ALL.len() - 1));
+        assert_eq!(settings_row(&app), Some(OptionId::GENERAL.len() - 1));
         app.apply(Action::SelectSettingsRow(2));
         assert_eq!(settings_row(&app), Some(2));
         app.apply(Action::SelectSettingsRow(99));
diff --git a/src/settings.rs b/src/settings.rs
index c72da64..2cdfff9 100644
--- a/src/settings.rs
+++ b/src/settings.rs
@@ -1366,9 +1366,15 @@ notify   = true
                 Key::PageSize,
                 Key::Timestamps,
                 Key::GroupDigits,
-                Key::ValueTags
+                Key::ValueTags,
+                Key::McpEnabled,
+                Key::McpLogDays,
+                Key::McpNotify,
             ]
         );
+        // Each pane has its own, and every one is on a pane.
+        let panes = [&OptionId::GENERAL[..], &OptionId::AGENTS[..]].concat();
+        assert_eq!(panes, OptionId::ALL);
         let defaults = Settings::default();
         for option in OptionId::ALL {
             assert_eq!(option.default_value(), defaults.value(option));
@@ -1395,6 +1401,26 @@ notify   = true
         assert_eq!(step(9_000, false), OptionValue::PageSize(5_000));
     }
 
+    #[test]
+    fn the_logs_days_step_through_their_choices_and_stop_at_the_ends() {
+        let step = |days: u32, forward: bool| {
+            let settings = Settings {
+                mcp_log_days: days,
+                ..Settings::default()
+            };
+            settings.stepped(OptionId::LogDays, forward)
+        };
+        assert_eq!(step(30, true), OptionValue::LogDays(90));
+        assert_eq!(step(30, false), OptionValue::LogDays(7));
+        assert_eq!(step(7, false), OptionValue::LogDays(7));
+        assert_eq!(step(365, true), OptionValue::LogDays(365));
+        // Days set by hand are between two choices.
+        assert_eq!(step(14, true), OptionValue::LogDays(30));
+        assert_eq!(step(1_000, false), OptionValue::LogDays(365));
+        // Nothing to flip: there are more than two.
+        assert_eq!(Settings::default().flipped(OptionId::LogDays), None);
+    }
+
     #[test]
     fn an_option_of_two_values_steps_to_the_side_it_is_drawn_on() {
         let settings = Settings::default();
diff --git a/src/ui/agent_settings_tests.rs b/src/ui/agent_settings_tests.rs
new file mode 100644
index 0000000..9b56c6d
--- /dev/null
+++ b/src/ui/agent_settings_tests.rs
@@ -0,0 +1,424 @@
+//! Settings › Agents, in both looks: the pane switch, the server's switch
+//! and the log's options, how each agent connects, and the ways from it to
+//! the Agents tab and Connections, and back from the tab.
+
+use crate::connections::AgentAccess;
+use crate::model::{Action, AgentsView, ConnTabId, Dialog, SettingsPane, Tab};
+use crate::testing::Harness;
+use crate::theme::Look;
+
+fn looks() -> [Look; 2] {
+    [Look::macos(), Look::omarchy()]
+}
+
+/// A window connected to the fake connection, in `look`, with agents
+/// allowed to read it.
+fn connected(look: Look) -> (Harness, ConnTabId) {
+    let mut harness = Harness::new();
+    harness.set_look(look);
+    let tab = harness.connect_fake();
+    let conn = harness.app.workspace(tab).unwrap().conn_id.clone();
+    let mut saved = harness.app.connections.get(&conn).unwrap().clone();
+    saved.agents.access = AgentAccess::Read;
+    harness.app.connections.upsert(saved);
+    harness.app.check_agents();
+    let held = harness.app.agents_held.clone();
+    harness.app.apply(Action::KeepAgentChanges(held));
+    harness.settle();
+    (harness, tab)
+}
+
+/// The pane Settings shows, while it is open.
+fn pane(harness: &Harness) -> Option<SettingsPane> {
+    match &harness.app.dialog {
+        Some(Dialog::Settings(dialog)) => Some(dialog.pane),
+        _ => None,
+    }
+}
+
+/// The view of `tab`'s Agents tab, while it is in front.
+fn agents_view(harness: &Harness, tab: ConnTabId) -> Option<AgentsView> {
+    let workspace = harness.app.workspace(tab)?;
+    if !workspace.agents_in_front() {
+        return None;
+    }
+    workspace.tabs.iter().find_map(|open| match open {
+        Tab::Agents(agents) => Some(agents.view),
+        Tab::Object(_) | Tab::Sql(_) => None,
+    })
+}
+
+/// Settings on its Agents pane, the cursor moved down `rows` rows in the
+/// terminal look.
+fn on_the_pane(harness: &mut Harness, rows: usize) {
+    harness.app.apply(Action::ShowAgentSettings);
+    harness.settle();
+    for _ in 0..rows {
+        super::tests::type_key(harness, egui::Key::J, "j");
+    }
+}
+
+#[test]
+fn settings_lists_an_agents_pane_beside_general() {
+    for look in looks() {
+        let mut harness = Harness::new();
+        harness.set_look(look);
+        harness.app.apply(Action::ShowSettings);
+        harness.settle();
+        assert_eq!(pane(&harness), Some(SettingsPane::General), "{}", look.name);
+        assert!(!harness.has("Let agents use Tabletist"), "{}", look.name);
+        if look.terminal {
+            harness.press(egui::Key::L, egui::Modifiers::CTRL);
+        } else {
+            harness.click("Agents");
+        }
+        harness.settle();
+        assert_eq!(pane(&harness), Some(SettingsPane::Agents), "{}", look.name);
+        assert!(harness.has("Let agents use Tabletist"), "{}", look.name);
+        assert!(!harness.has("Rows per page"), "{}", look.name);
+        // And back.
+        if look.terminal {
+            harness.press(egui::Key::H, egui::Modifiers::CTRL);
+        } else {
+            harness.click("General");
+        }
+        harness.settle();
+        assert_eq!(pane(&harness), Some(SettingsPane::General), "{}", look.name);
+        assert!(harness.has("Rows per page"), "{}", look.name);
+    }
+}
+
+#[test]
+fn the_agents_pane_turns_the_server_on_and_sets_the_log_and_the_notification() {
+    for look in looks() {
+        let mut harness = Harness::new();
+        harness.set_look(look);
+        on_the_pane(&mut harness, 0);
+        assert!(!harness.app.settings.mcp_enabled);
+        if look.terminal {
+            harness.press(egui::Key::Space, egui::Modifiers::NONE);
+        } else {
+            harness.click("Let agents use Tabletist");
+        }
+        harness.settle();
+        assert!(harness.app.settings.mcp_enabled, "{}", look.name);
+        assert!(harness.app.agents_on(), "{}", look.name);
+        // The log's days, from 30 to 90.
+        if look.terminal {
+            for _ in 0..3 {
+                super::tests::type_key(&mut harness, egui::Key::J, "j");
+            }
+            super::tests::type_key(&mut harness, egui::Key::L, "l");
+        } else {
+            harness.click("Keep the activity log");
+            harness.click("Keep the activity log 90 days");
+        }
+        harness.settle();
+        assert_eq!(harness.app.settings.mcp_log_days, 90, "{}", look.name);
+        // The notification, off.
+        if look.terminal {
+            super::tests::type_key(&mut harness, egui::Key::J, "j");
+            harness.press(egui::Key::Space, egui::Modifiers::NONE);
+        } else {
+            harness.click("Notify when an agent connects");
+        }
+        harness.settle();
+        assert!(!harness.app.settings.mcp_notify, "{}", look.name);
+        // Off again: what Stop all agents was.
+        if look.terminal {
+            for _ in 0..4 {
+                super::tests::type_key(&mut harness, egui::Key::K, "k");
+            }
+            harness.press(egui::Key::Space, egui::Modifiers::NONE);
+        } else {
+            harness.click("Let agents use Tabletist");
+        }
+        harness.settle();
+        assert!(!harness.app.settings.mcp_enabled, "{}", look.name);
+    }
+}
+
+#[test]
+fn the_servers_switch_shows_what_is_in_force_and_keeps_a_change_held_from_outside() {
+    let mut harness = Harness::new();
+    // The file turned the server on, outside the app: held.
+    harness.app.settings.mcp_enabled = true;
+    harness.app.check_agents();
+    assert!(!harness.app.agents_on());
+    on_the_pane(&mut harness, 0);
+    // The switch says off, as the server is.
+    assert!(harness.has("Off"));
+    harness.click("Let agents use Tabletist");
+    harness.settle();
+    // On is the person's own: what was held is kept, not written again.
+    assert!(harness.app.agents_on());
+    assert!(harness.app.agents_held.is_empty());
+    // On once more changes nothing, and writes nothing.
+    harness.app.backend.sent.clear();
+    harness
+        .app
+        .apply(Action::SetOption(crate::settings::OptionValue::AgentsOn(
+            true,
+        )));
+    assert!(
+        harness.app.backend.sent.is_empty(),
+        "{:?}",
+        harness.app.backend.sent
+    );
+}
+
+#[test]
+fn reset_on_the_agents_pane_puts_back_its_own_options() {
+    let mut harness = Harness::new();
+    harness.app.change_settings(|settings| {
+        settings.page_size = 500;
+        settings.mcp_log_days = 90;
+        settings.mcp_notify = false;
+    });
+    harness.app.apply(Action::TurnAgentsOn);
+    on_the_pane(&mut harness, 0);
+    harness.app.apply(Action::ResetSettings);
+    harness.app.apply(Action::ConfirmResetSettings(true));
+    assert_eq!(harness.app.settings.mcp_log_days, 30);
+    assert!(harness.app.settings.mcp_notify);
+    // The server's switch is not a default to put back: off, it would end
+    // every agent's session.
+    assert!(harness.app.agents_on());
+    // General's are General's.
+    assert_eq!(harness.app.settings.page_size, 500);
+    // A question asked on one pane is not answered on the other.
+    harness.app.apply(Action::ResetSettings);
+    harness
+        .app
+        .apply(Action::ShowSettingsPane(SettingsPane::General));
+    harness.app.apply(Action::ConfirmResetSettings(true));
+    assert_eq!(harness.app.settings.page_size, 500);
+}
+
+#[test]
+fn the_agents_pane_says_how_each_agent_connects_and_copies_it() {
+    // Worked out, as the backend's start does off the UI thread.
+    let command = crate::endpoint::agent_command();
+    let (mut harness, _) = connected(Look::macos());
+    on_the_pane(&mut harness, 0);
+    assert!(harness.has(command));
+    harness.click("Copy");
+    harness.settle();
+    assert_eq!(harness.copied.as_deref(), Some(command));
+    // Cursor's entry for its mcp.json.
+    harness.click("Connect an agent: Cursor");
+    harness.settle();
+    let entry = crate::endpoint::agent_setup_ready(crate::endpoint::Agent::Cursor).unwrap();
+    assert!(harness.has(entry));
+    harness.click("Copy");
+    harness.settle();
+    assert_eq!(harness.copied.as_deref(), Some(entry));
+    assert_eq!(
+        crate::ui::toast::said(&harness.ctx).as_deref(),
+        Some("Copied the entry for mcp.json")
+    );
+    // Any other agent: the command it starts.
+    harness.click("Connect an agent: Other");
+    harness.settle();
+    let relay = crate::endpoint::agent_setup_ready(crate::endpoint::Agent::Other).unwrap();
+    assert!(harness.has(relay));
+
+    // The terminal look: `y` copies Claude Code's command.
+    let (mut harness, _) = connected(Look::omarchy());
+    on_the_pane(&mut harness, 0);
+    super::tests::type_key(&mut harness, egui::Key::Y, "y");
+    assert_eq!(harness.copied.as_deref(), Some(command));
+    // On General it is nothing.
+    harness
+        .app
+        .apply(Action::ShowSettingsPane(SettingsPane::General));
+    harness.settle();
+    harness.copied = None;
+    super::tests::type_key(&mut harness, egui::Key::Y, "y");
+    assert_eq!(harness.copied, None);
+}
+
+#[test]
+fn settings_shows_the_agents_tab_and_its_log_of_the_window_in_front() {
+    for look in looks() {
+        let (mut harness, tab) = connected(look);
+        // The status row, in the terminal look.
+        on_the_pane(&mut harness, 1);
+        if look.terminal {
+            harness.press(egui::Key::Enter, egui::Modifiers::NONE);
+        } else {
+            harness.click("Show Agents tab");
+        }
+        harness.settle();
+        assert!(harness.app.dialog.is_none(), "{}", look.name);
+        assert_eq!(
+            agents_view(&harness, tab),
+            Some(AgentsView::Activity),
+            "{}",
+            look.name
+        );
+        // The log's row.
+        on_the_pane(&mut harness, 3);
+        if look.terminal {
+            harness.press(egui::Key::Enter, egui::Modifiers::NONE);
+        } else {
+            harness.click("Show log");
+        }
+        harness.settle();
+        assert!(harness.app.dialog.is_none(), "{}", look.name);
+        assert_eq!(
+            agents_view(&harness, tab),
+            Some(AgentsView::Log),
+            "{}",
+            look.name
+        );
+    }
+}
+
+#[test]
+fn with_no_window_open_settings_has_no_agents_tab_to_show() {
+    for look in looks() {
+        let mut harness = Harness::new();
+        harness.set_look(look);
+        on_the_pane(&mut harness, 1);
+        if look.terminal {
+            harness.press(egui::Key::Enter, egui::Modifiers::NONE);
+        } else {
+            // Disabled: a click is nothing.
+            harness.click("Show Agents tab");
+        }
+        harness.settle();
+        assert_eq!(pane(&harness), Some(SettingsPane::Agents), "{}", look.name);
+        harness
+            .app
+            .apply(Action::ShowAgentsFromSettings(AgentsView::Log));
+        assert_eq!(pane(&harness), Some(SettingsPane::Agents), "{}", look.name);
+    }
+}
+
+#[test]
+fn settings_opens_connections_where_access_is_set() {
+    for look in looks() {
+        let (mut harness, tab) = connected(look);
+        // The access row, in the terminal look.
+        on_the_pane(&mut harness, 2);
+        if look.terminal {
+            harness.press(egui::Key::Enter, egui::Modifiers::NONE);
+        } else {
+            harness.click("Open Connections");
+        }
+        harness.settle();
+        assert!(harness.app.dialog.is_none(), "{}", look.name);
+        let front = harness.app.active_tab_id();
+        assert_ne!(front, tab, "{}", look.name);
+        assert!(harness.app.workspace(front).is_none(), "{}", look.name);
+    }
+}
+
+#[test]
+fn an_enter_held_on_the_status_row_shows_the_agents_tab_and_goes_no_further() {
+    let (mut harness, tab) = connected(Look::omarchy());
+    harness.app.settings.mcp_enabled = true;
+    harness.app.serve_agents();
+    // An agent works on another connection: the tab's Enter shows its
+    // window.
+    let mut ledger = crate::testing::fixture_connection();
+    ledger.id = crate::connections::ConnectionId("ledger".into());
+    ledger.name = "Ledger".into();
+    harness.app.connections.upsert(ledger);
+    harness
+        .app
+        .agents
+        .sessions
+        .push(tabletist_mcp::SessionInfo {
+            id: "elsewhere".into(),
+            agent: Some("Cursor".into()),
+            project: None,
+            transport: tabletist_mcp::Transport::Stdio,
+            connected_at: std::time::SystemTime::UNIX_EPOCH,
+            calls: 1,
+            connections: vec!["ledger".into()],
+            last_call_at: None,
+        });
+    on_the_pane(&mut harness, 1);
+    // Enter held: its press, then what the window repeats of it.
+    for _ in 0..4 {
+        harness.frame(vec![crate::testing::key(
+            egui::Key::Enter,
+            egui::Modifiers::NONE,
+        )]);
+    }
+    harness.frame(vec![crate::testing::release(
+        egui::Key::Enter,
+        egui::Modifiers::NONE,
+    )]);
+    harness.settle();
+    assert_eq!(
+        harness.app.active_tab_id(),
+        tab,
+        "it went on past the Agents tab"
+    );
+    assert_eq!(agents_view(&harness, tab), Some(AgentsView::Activity));
+}
+
+#[test]
+fn the_log_row_says_enter_shows_the_log_only_where_there_is_a_window() {
+    let shows = |harness: &Harness| {
+        let hint = |text: &String| text.contains("shows it");
+        harness.painted.iter().any(|(text, _)| hint(text))
+    };
+    // Wide enough for the rows' hints beside the file.
+    let wide = || {
+        let mut harness = Harness::with_size(egui::vec2(1440.0, 900.0));
+        harness.set_look(Look::omarchy());
+        harness
+    };
+    let mut harness = wide();
+    on_the_pane(&mut harness, 3);
+    assert!(!shows(&harness));
+    assert!(
+        harness
+            .painted
+            .iter()
+            .any(|(text, _)| text == "on this computer")
+    );
+    let mut harness = wide();
+    harness.connect_fake();
+    on_the_pane(&mut harness, 3);
+    assert!(shows(&harness));
+}
+
+#[test]
+fn the_agents_panes_columns_stay_where_they_are_when_the_server_turns_on() {
+    let mut harness = Harness::with_size(egui::vec2(1440.0, 900.0));
+    harness.set_look(Look::omarchy());
+    on_the_pane(&mut harness, 0);
+    let column = |harness: &Harness| {
+        let found = harness
+            .text_rects
+            .iter()
+            .find(|(text, _)| text == "in connections");
+        found.map(|(_, rect)| rect.left())
+    };
+    let off = column(&harness);
+    assert!(off.is_some());
+    harness.press(egui::Key::Space, egui::Modifiers::NONE);
+    harness.settle();
+    assert!(harness.app.agents_on());
+    assert_eq!(column(&harness), off);
+}
+
+#[test]
+fn the_settings_footer_keeps_close_in_the_narrowest_window() {
+    let mut harness = Harness::with_size(egui::vec2(720.0, 480.0));
+    harness.set_look(Look::omarchy());
+    harness.connect_fake();
+    // The status row: the most hints the Agents pane has.
+    on_the_pane(&mut harness, 1);
+    let close = harness
+        .text_rects
+        .iter()
+        .find(|(text, _)| text.contains("close"))
+        .map(|(_, rect)| *rect);
+    assert!(close.is_some_and(|rect| rect.right() <= 720.0), "{close:?}");
+}
diff --git a/src/ui/mod.rs b/src/ui/mod.rs
index 9e28d82..b60867f 100644
--- a/src/ui/mod.rs
+++ b/src/ui/mod.rs
@@ -9,6 +9,10 @@ pub mod agents;
 #[cfg(test)]
 #[cfg(unix)]
 mod agents_tests;
+// Settings › Agents, which is listed where agents can reach the app.
+#[cfg(test)]
+#[cfg(unix)]
+mod agent_settings_tests;
 pub mod cell_editor;
 #[cfg(test)]
 mod complete_tests;
diff --git a/src/ui/settings/sheet.rs b/src/ui/settings/sheet.rs
index f725ae6..52d256d 100644
--- a/src/ui/settings/sheet.rs
+++ b/src/ui/settings/sheet.rs
@@ -777,6 +777,8 @@ mod tests {
 
     #[test]
     fn a_size_that_is_not_in_the_list_takes_its_place_among_the_others() {
+        assert_eq!(log_days(30), Settings::LOG_DAYS);
+        assert_eq!(log_days(14), [7, 14, 30, 90, 365]);
         assert_eq!(page_sizes(300), Settings::PAGE_SIZES);
         assert_eq!(page_sizes(250), [100, 250, 300, 500, 1_000, 5_000]);
         assert_eq!(page_sizes(9_000), [100, 300, 500, 1_000, 5_000, 9_000]);
````

- [ ] **Step 2: Run them to see that they do not build**

Run: `cargo test --locked --lib -- settings:: ui::agent_settings_tests app::tests::the_settings_cursor`

Expected: it does not build. The tests name what is not there yet; among the errors:

```text
error[E0432]: unresolved import `crate::model::SettingsPane`
error[E0599]: no variant, associated function, or constant named `GENERAL` found for enum `settings::OptionId` in the current scope
error[E0599]: no variant, associated function, or constant named `LogDays` found for enum `settings::OptionId` in the current scope
error[E0609]: no field `pane` on type `&Box<SettingsDialog>`
error[E0599]: no variant, associated function, or constant named `ShowAgentSettings` found for enum `model::Action` in the current scope
error[E0599]: no variant, associated function, or constant named `ShowSettingsPane` found for enum `model::Action` in the current scope
error[E0599]: no associated function or constant named `LOG_DAYS` found for struct `settings::Settings` in the current scope
error: could not compile `tabletist` (lib test) due to 21 previous errors
```

- [ ] **Step 3: The pane**

````diff
diff --git a/src/app.rs b/src/app.rs
index 2f0ad24..330b84f 100644
--- a/src/app.rs
+++ b/src/app.rs
@@ -1608,19 +1608,62 @@ impl App {
                     self.dialog = Some(Dialog::Settings(Box::default()));
                 }
             }
+            Action::ShowAgentSettings => {
+                // From the Agents tab: there is nothing of agents to show
+                // where they cannot reach the app.
+                if crate::endpoint::AVAILABLE && matches!(self.dialog, None | Some(Dialog::Help)) {
+                    self.dialog = Some(Dialog::Settings(Box::new(crate::model::SettingsDialog {
+                        pane: crate::model::SettingsPane::Agents,
+                        ..Default::default()
+                    })));
+                }
+            }
+            Action::ShowSettingsPane(pane) => {
+                if let Some(Dialog::Settings(dialog)) = &mut self.dialog
+                    && crate::model::SettingsPane::listed().contains(&pane)
+                    && dialog.pane != pane
+                {
+                    dialog.pane = pane;
+                    dialog.row = 0;
+                    // The question was about the other pane's options.
+                    dialog.resetting = false;
+                }
+            }
+            Action::ChooseAgentSetup(agent) => {
+                if let Some(Dialog::Settings(dialog)) = &mut self.dialog {
+                    dialog.agent = agent;
+                }
+            }
+            Action::ShowAgentsFromSettings(view) => {
+                if !matches!(self.dialog, Some(Dialog::Settings(_))) {
+                    return;
+                }
+                let Some(tab) = self.agents_window() else {
+                    return;
+                };
+                self.dialog = None;
+                self.apply(Action::ActivateConnTab(tab));
+                self.apply(Action::ShowAgents(tab));
+                self.apply(Action::ShowAgentsView { tab, view });
+            }
             Action::MoveSettingsRow(by) => {
                 if let Some(Dialog::Settings(dialog)) = &mut self.dialog {
-                    let last = crate::settings::OptionId::ALL.len() - 1;
+                    let last = dialog.pane.rows().len() - 1;
                     dialog.row = dialog.row.saturating_add_signed(by).min(last);
                 }
             }
             Action::SelectSettingsRow(row) => {
                 if let Some(Dialog::Settings(dialog)) = &mut self.dialog
-                    && row < crate::settings::OptionId::ALL.len()
+                    && row < dialog.pane.rows().len()
                 {
                     dialog.row = row;
                 }
             }
+            // The server's switch, turned on: the person's own doing, as
+            // Turn on is, which keeps a change made outside that is held.
+            Action::SetOption(crate::settings::OptionValue::AgentsOn(true)) => {
+                self.turn_agents_on();
+            }
             Action::SetOption(value) => self.change_settings(|settings| value.set(settings)),
             Action::EditSettingsFile => {
                 let text = self.offer_settings_text();
@@ -1654,14 +1697,19 @@ impl App {
             Action::ConfirmResetSettings(reset) => {
                 // An answer to a question that was asked.
                 let asked = match &mut self.dialog {
-                    Some(Dialog::Settings(dialog)) => std::mem::take(&mut dialog.resetting),
-                    _ => false,
+                    Some(Dialog::Settings(dialog)) => {
+                        std::mem::take(&mut dialog.resetting).then_some(dialog.pane)
+                    }
+                    _ => None,
                 };
-                if asked && reset {
-                    // The options the window shows. A key it has no control
-                    // for is not the window's to change.
+                if let Some(pane) = asked
+                    && reset
+                {
+                    // The options of the pane that asked. A key it has no
+                    // control for is not the window's to change, nor is
+                    // the server's switch.
                     self.change_settings(|settings| {
-                        for option in crate::settings::OptionId::ALL {
+                        for option in pane.reset_options() {
                             option.default_value().set(settings);
                         }
                     });
@@ -3324,6 +3372,10 @@ impl App {
     /// kept.
     fn turn_agents_on(&mut self) {
         use crate::agents_shown::Raised;
+        // On already, and in force: nothing to write.
+        if self.agents_on() {
+            return;
+        }
         if !self.settings.mcp_enabled {
             self.change_settings(|settings| settings.mcp_enabled = true);
             return;
@@ -3990,20 +4042,24 @@ impl App {
         }
     }
 
+    /// The window whose Agents tab the app's own ways in show (Review,
+    /// Settings): the one in front; with Connections in front, the last
+    /// window there is. None with no window open.
+    pub fn agents_window(&self) -> Option<ConnTabId> {
+        let front = self.active_tab_id();
+        match self.workspace(front) {
+            Some(_) => Some(front),
+            None => self.open_connections().last().map(|(tab, _)| tab),
+        }
+    }
+
     /// The person clicked Review: Tabletist comes forward, on the Agents
     /// tab of the window in front, or on Connections where none is open
     /// (question 22). A new agent is on no connection yet: it is under
     /// "Also:" there.
     fn reviewed(&mut self) {
         self.window_asks.push(egui::ViewportCommand::Focus);
-        // The window in front; with Connections in front, the last window
-        // there is.
-        let front = self.active_tab_id();
-        let tab = match self.workspace(front) {
-            Some(_) => Some(front),
-            None => self.open_connections().last().map(|(tab, _)| tab),
-        };
-        match tab {
+        match self.agents_window() {
             Some(tab) => {
                 self.apply(Action::ActivateConnTab(tab));
                 self.apply(Action::ShowAgents(tab));
diff --git a/src/keymap.rs b/src/keymap.rs
index 670e4ae..090edb2 100644
--- a/src/keymap.rs
+++ b/src/keymap.rs
@@ -491,6 +491,9 @@ commands! {
     SettingsOpenFile,
     SettingsReset,
     SettingsClose,
+    SettingsPane,
+    SettingsOpenRow,
+    SettingsCopyCommand,
     QuickOpenMove,
     QuickOpenPick,
     QuickOpenClose,
@@ -780,6 +783,17 @@ impl Command {
             C::SettingsOpenFile => info("Open the settings file in $EDITOR", "", Prompts),
             C::SettingsReset => info("Reset the option", "", Prompts),
             C::SettingsClose => info("Close the settings", "", Prompts),
+            C::SettingsPane => info(
+                "Show the next or the previous pane of the settings",
+                "",
+                Prompts,
+            ),
+            C::SettingsOpenRow => info("Open what the settings' row shows", "", Prompts),
+            C::SettingsCopyCommand => agents(info(
+                "Copy the command that connects an agent, in the settings",
+                "",
+                Prompts,
+            )),
             C::QuickOpenMove => info("Move in quick open", "", Prompts),
             C::QuickOpenPick => info("Open the one selected", "", Prompts),
             C::QuickOpenClose => info("Close quick open", "", Prompts),
@@ -1710,6 +1724,15 @@ pub const BINDINGS: &[Binding] = {
         bind(C::SettingsToggle, PROMPT, NONE, keys(&[k("space")])).when(When::SettingsScreen),
         bind(C::SettingsOpenFile, PROMPT, NONE, keys(&[k("ctrl+e")])).when(When::SettingsScreen),
         bind(C::SettingsReset, PROMPT, NONE, keys(&[k("R")])).when(When::SettingsScreen),
+        bind(
+            C::SettingsPane,
+            PROMPT,
+            NONE,
+            keys(&[k("ctrl+h"), k("ctrl+l")]).shown("ctrl+h/l"),
+        )
+        .when(When::SettingsScreen),
+        bind(C::SettingsOpenRow, PROMPT, NONE, keys(&[k("enter")])).when(When::SettingsScreen),
+        bind(C::SettingsCopyCommand, PROMPT, NONE, keys(&[k("y")])).when(When::SettingsScreen),
         bind(
             C::SettingsClose,
             PROMPT,
diff --git a/src/model.rs b/src/model.rs
index 7943e35..f9ee8b9 100644
--- a/src/model.rs
+++ b/src/model.rs
@@ -782,9 +782,18 @@ pub enum Action {
     ShowAbout,
     /// Show the Settings window.
     ShowSettings,
-    /// Move the Settings window's cursor by this many options.
+    /// Show the Settings window on its Agents pane: from the Agents tab.
+    ShowAgentSettings,
+    /// Show this pane of the Settings window, its cursor on its first row.
+    ShowSettingsPane(SettingsPane),
+    /// Show how this agent is connected, in Settings › Agents.
+    ChooseAgentSetup(crate::endpoint::Agent),
+    /// Close Settings and show the Agents tab on this view: the window's in
+    /// front, or with Connections in front the last window's.
+    ShowAgentsFromSettings(AgentsView),
+    /// Move the Settings window's cursor by this many rows.
     MoveSettingsRow(isize),
-    /// Put the Settings window's cursor on this option.
+    /// Put the Settings window's cursor on this row of its pane.
     SelectSettingsRow(usize),
     /// Set an option. Applied and saved at once.
     SetOption(crate::settings::OptionValue),
@@ -1743,12 +1752,104 @@ pub struct QuickOpen {
 /// them.
 #[derive(Debug, Default)]
 pub struct SettingsDialog {
-    /// The option the keys act on, as an index into
-    /// [`crate::settings::OptionId::ALL`].
+    /// The pane shown.
+    pub pane: SettingsPane,
+    /// The row the keys act on, as an index into the pane's
+    /// [`SettingsPane::rows`].
     pub row: usize,
     /// Reset to defaults was chosen and waits for its answer: the footer
     /// asks in place of its links.
     pub resetting: bool,
+    /// The agent Settings › Agents says how to connect.
+    pub agent: crate::endpoint::Agent,
+}
+
+/// A pane of the Settings window.
+#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
+pub enum SettingsPane {
+    #[default]
+    General,
+    /// How agents reach the app. Listed where they can.
+    Agents,
+}
+
+impl SettingsPane {
+    /// The panes the window lists, in order.
+    pub fn listed() -> &'static [SettingsPane] {
+        if crate::endpoint::AVAILABLE {
+            &[Self::General, Self::Agents]
+        } else {
+            &[Self::General]
+        }
+    }
+
+    /// The pane's name. English: the caller translates it.
+    pub fn name(self) -> &'static str {
+        match self {
+            Self::General => "General",
+            Self::Agents => "Agents",
+        }
+    }
+
+    /// The pane's options, in order.
+    pub fn options(self) -> &'static [crate::settings::OptionId] {
+        match self {
+            Self::General => &crate::settings::OptionId::GENERAL,
+            Self::Agents => &crate::settings::OptionId::AGENTS,
+        }
+    }
+
+    /// What Reset to defaults puts back: the pane's options but the
+    /// server's switch, which turned off would end every agent's session
+    /// for a question about defaults.
+    pub fn reset_options(self) -> &'static [crate::settings::OptionId] {
+        use crate::settings::OptionId;
+        match self {
+            Self::General => &OptionId::GENERAL,
+            Self::Agents => &[OptionId::LogDays, OptionId::AgentsNotify],
+        }
+    }
+
+    /// The rows the terminal look's cursor moves among, in order.
+    pub fn rows(self) -> &'static [SettingsRow] {
+        use crate::settings::OptionId;
+        match self {
+            Self::General => &[
+                SettingsRow::Option(OptionId::PageSize),
+                SettingsRow::Option(OptionId::Timestamps),
+                SettingsRow::Option(OptionId::GroupDigits),
+                SettingsRow::Option(OptionId::ValueTags),
+            ],
+            Self::Agents => &[
+                SettingsRow::Option(OptionId::AgentsOn),
+                SettingsRow::AgentsStatus,
+                SettingsRow::AgentsAccess,
+                SettingsRow::Option(OptionId::LogDays),
+                SettingsRow::Option(OptionId::AgentsNotify),
+            ],
+        }
+    }
+}
+
+/// A row of the terminal look's Settings screen that the cursor can be on.
+#[derive(Clone, Copy, Debug, PartialEq, Eq)]
+pub enum SettingsRow {
+    /// An option, which the keys change.
+    Option(crate::settings::OptionId),
+    /// Whether agents can connect, and who has: Enter shows the Agents tab.
+    AgentsStatus,
+    /// What agents may do, which is set in Connections: Enter opens them.
+    AgentsAccess,
+}
+
+impl SettingsRow {
+    /// The row's option, if it is one.
+    pub fn option(self) -> Option<crate::settings::OptionId> {
+        match self {
+            Self::Option(option) => Some(option),
+            Self::AgentsStatus | Self::AgentsAccess => None,
+        }
+    }
 }
 
 /// Asks whether to trust an SSH host seen for the first time.
diff --git a/src/settings.rs b/src/settings.rs
index 2cdfff9..13385ff 100644
--- a/src/settings.rs
+++ b/src/settings.rs
@@ -137,16 +137,34 @@ pub enum OptionId {
     Timestamps,
     GroupDigits,
     ValueTags,
+    /// Whether agents can reach the app: `mcp.enabled`.
+    AgentsOn,
+    /// How long the log of agents' calls keeps one: `mcp.log_days`.
+    LogDays,
+    /// Whether a new agent is told of: `mcp.notify`.
+    AgentsNotify,
 }
 
 impl OptionId {
-    /// In the order the window has them.
-    pub const ALL: [OptionId; 4] = [
+    /// Every option, the General pane's and then the Agents pane's.
+    pub const ALL: [OptionId; 7] = [
+        Self::PageSize,
+        Self::Timestamps,
+        Self::GroupDigits,
+        Self::ValueTags,
+        Self::AgentsOn,
+        Self::LogDays,
+        Self::AgentsNotify,
+    ];
+    /// The General pane's, in the order it has them.
+    pub const GENERAL: [OptionId; 4] = [
         Self::PageSize,
         Self::Timestamps,
         Self::GroupDigits,
         Self::ValueTags,
     ];
+    /// The Agents pane's, in the order it has them.
+    pub const AGENTS: [OptionId; 3] = [Self::AgentsOn, Self::LogDays, Self::AgentsNotify];
 
     /// The key of the file the option is stored under.
     pub fn key(self) -> Key {
@@ -155,6 +173,9 @@ impl OptionId {
             Self::Timestamps => Key::Timestamps,
             Self::GroupDigits => Key::GroupDigits,
             Self::ValueTags => Key::ValueTags,
+            Self::AgentsOn => Key::McpEnabled,
+            Self::LogDays => Key::McpLogDays,
+            Self::AgentsNotify => Key::McpNotify,
         }
     }
 
@@ -171,6 +192,9 @@ pub enum OptionValue {
     Timestamps(Timestamps),
     GroupDigits(bool),
     ValueTags(bool),
+    AgentsOn(bool),
+    LogDays(u32),
+    AgentsNotify(bool),
 }
 
 impl OptionValue {
@@ -180,6 +204,9 @@ impl OptionValue {
             Self::Timestamps(_) => OptionId::Timestamps,
             Self::GroupDigits(_) => OptionId::GroupDigits,
             Self::ValueTags(_) => OptionId::ValueTags,
+            Self::AgentsOn(_) => OptionId::AgentsOn,
+            Self::LogDays(_) => OptionId::LogDays,
+            Self::AgentsNotify(_) => OptionId::AgentsNotify,
         }
     }
 
@@ -190,6 +217,9 @@ impl OptionValue {
             Self::Timestamps(choice) => settings.timestamps = choice,
             Self::GroupDigits(on) => settings.group_digits = on,
             Self::ValueTags(on) => settings.value_tags = on,
+            Self::AgentsOn(on) => settings.mcp_enabled = on,
+            Self::LogDays(days) => settings.mcp_log_days = days,
+            Self::AgentsNotify(on) => settings.mcp_notify = on,
         }
     }
 }
@@ -406,6 +436,8 @@ impl Settings {
     /// day, and ten years.
     pub const MIN_MCP_LOG_DAYS: u32 = 1;
     pub const MAX_MCP_LOG_DAYS: u32 = 3_650;
+    /// The days the Settings window offers for the log.
+    pub const LOG_DAYS: [u32; 4] = [7, 30, 90, 365];
 
     /// The SQL editor's timeout as a duration; `None` waits forever.
     pub fn sql_timeout(&self) -> Option<std::time::Duration> {
@@ -435,6 +467,9 @@ impl Settings {
             OptionId::Timestamps => OptionValue::Timestamps(self.timestamps),
             OptionId::GroupDigits => OptionValue::GroupDigits(self.group_digits),
             OptionId::ValueTags => OptionValue::ValueTags(self.value_tags),
+            OptionId::AgentsOn => OptionValue::AgentsOn(self.mcp_enabled),
+            OptionId::LogDays => OptionValue::LogDays(self.mcp_log_days),
+            OptionId::AgentsNotify => OptionValue::AgentsNotify(self.mcp_notify),
         }
     }
 
@@ -464,6 +499,20 @@ impl Settings {
             // Grouped is the left of the two.
             OptionId::GroupDigits => OptionValue::GroupDigits(!forward),
             OptionId::ValueTags => OptionValue::ValueTags(forward),
+            OptionId::AgentsOn => OptionValue::AgentsOn(forward),
+            OptionId::LogDays => {
+                let days = self.mcp_log_days;
+                let next = if forward {
+                    Self::LOG_DAYS.into_iter().find(|choice| *choice > days)
+                } else {
+                    Self::LOG_DAYS
+                        .into_iter()
+                        .rev()
+                        .find(|choice| *choice < days)
+                };
+                OptionValue::LogDays(next.unwrap_or(days))
+            }
+            OptionId::AgentsNotify => OptionValue::AgentsNotify(forward),
         }
     }
 
@@ -477,6 +526,9 @@ impl Settings {
             })),
             OptionId::GroupDigits => Some(OptionValue::GroupDigits(!self.group_digits)),
             OptionId::ValueTags => Some(OptionValue::ValueTags(!self.value_tags)),
+            OptionId::AgentsOn => Some(OptionValue::AgentsOn(!self.mcp_enabled)),
+            OptionId::LogDays => None,
+            OptionId::AgentsNotify => Some(OptionValue::AgentsNotify(!self.mcp_notify)),
         }
     }
 
diff --git a/src/ui/keys.rs b/src/ui/keys.rs
index 10b4eaf..afd068c 100644
--- a/src/ui/keys.rs
+++ b/src/ui/keys.rs
@@ -2201,8 +2201,9 @@ fn letters(
                     true,
                 )
             })
-            .count
-            > 0
+            // A press, not what a held Enter repeats: one held on Settings'
+            // status row shows this tab and goes no further.
+            .fresh
     {
         actions.push(Action::ShowAgentsOn(conn));
     }
diff --git a/src/ui/settings/agents.rs b/src/ui/settings/agents.rs
new file mode 100644
index 0000000..9d2bbed
--- /dev/null
+++ b/src/ui/settings/agents.rs
@@ -0,0 +1,454 @@
+//! Settings › Agents as the sheet draws it: the server's switch beside the
+//! pane's title, whether it runs and who is connected, how to connect an
+//! agent, where access is set, what keeps agents within bounds, and the
+//! log's options.
+
+use egui::{CornerRadius, Rect, Sense, Stroke, StrokeKind, Ui, pos2, vec2};
+
+use crate::app::App;
+use crate::endpoint::Agent;
+use crate::i18n::{gettext, ngettext};
+use crate::keymap::Command;
+use crate::model::{Action, AgentsView};
+use crate::settings::OptionId;
+use crate::theme::Icon;
+use crate::typography::{Laid, Text, TextRole};
+use crate::ui::widgets::{self, ButtonSpec, Segment};
+
+use super::sheet::{After, Skin, label_column, row};
+
+/// What a short window leaves of the pane under its title at the least.
+const ROWS_MIN: f32 = 56.0;
+/// The space inside a box, at its sides and over and under what it holds.
+const PAD: egui::Vec2 = vec2(14.0, 12.0);
+/// A segment's height, and the gap between a group's heading and what
+/// follows.
+const SEGMENT: f32 = 22.0;
+const UNDER_HEADING: f32 = 8.0;
+/// Between the groups.
+const GROUP_GAP: f32 = 16.0;
+/// The switch beside the title, and the gap before it.
+const SWITCH: f32 = 30.0;
+const SWITCH_GAP: f32 = 16.0;
+
+/// What the toast says once an agent's setup is on the clipboard.
+const COPIED_ENTRY: &str = "Copied the entry for mcp.json";
+
+/// The pane, from the top of `ui` down. What is under its title scrolls
+/// when it is taller than `limit` leaves it.
+pub(super) fn content(
+    ui: &mut Ui,
+    app: &App,
+    agent: Agent,
+    limit: f32,
+    skin: &Skin,
+    actions: &mut Vec<Action>,
+) {
+    let settings = super::shown(app);
+    title(ui, settings.mcp_enabled, skin, actions);
+    ui.add_space(14.0);
+    let room = (limit - ui.cursor().top()).max(ROWS_MIN);
+    egui::ScrollArea::vertical()
+        .id_salt("agents")
+        .max_height(room)
+        .auto_shrink([false, true])
+        .show(ui, |ui| {
+            status(ui, app, skin, actions);
+            heading(ui, "Connect an agent", skin);
+            connect(ui, agent, skin, actions);
+            heading(ui, "Access", skin);
+            access(ui, skin, actions);
+            ui.add_space(GROUP_GAP - 4.0);
+            safety(ui, skin);
+            heading(ui, "Log", skin);
+            let column = label_column(ui, &OptionId::AGENTS, skin);
+            let show_log = gettext(skin.locale, "Show log");
+            let reason = gettext(skin.locale, super::NO_WINDOW);
+            let after = After {
+                text: &show_log,
+                disabled: app.agents_window().is_none().then_some(&*reason),
+                action: Action::ShowAgentsFromSettings(AgentsView::Log),
+            };
+            let log = OptionId::LogDays;
+            row(ui, log, true, column, &settings, skin, actions, Some(after));
+            let notify = OptionId::AgentsNotify;
+            row(ui, notify, false, column, &settings, skin, actions, None);
+        });
+}
+
+/// The pane's title and what it is for, with the server's switch at the
+/// right: off is what Stop all agents was (question 11), until it is
+/// turned on again.
+fn title(ui: &mut Ui, on: bool, skin: &Skin, actions: &mut Vec<Action>) {
+    let Skin { look, palette, .. } = *skin;
+    let width = ui.available_width();
+    let words = (width - SWITCH - SWITCH_GAP).max(0.0);
+    let name = Text::one(
+        look,
+        TextRole::DialogTitle,
+        &skin.say("Agents"),
+        palette.text,
+    )
+    .layout(ui.ctx());
+    let said = if cfg!(target_os = "macos") {
+        "Let coding agents query your databases through Tabletist (MCP). They use your saved connections; passwords stay in the Keychain."
+    } else {
+        "Let coding agents query your databases through Tabletist (MCP). They use your saved connections; passwords stay in the keyring."
+    };
+    let what = Text::one(look, TextRole::UiBody, &skin.say(said), palette.dim)
+        .wrap(words)
+        .layout(ui.ctx());
+    let height = name.height() + 2.0 + what.height();
+    let (rect, _) = ui.allocate_exact_size(vec2(width, height), Sense::hover());
+    paint(ui, &name, pos2(rect.left(), rect.top()));
+    paint(
+        ui,
+        &what,
+        pos2(rect.left(), rect.top() + name.height() + 2.0),
+    );
+    let place = Rect::from_min_max(pos2(rect.right() - SWITCH, rect.top()), rect.max);
+    let centred = egui::Layout::right_to_left(egui::Align::Center);
+    let builder = egui::UiBuilder::new().id_salt("switch").max_rect(place);
+    let mut switch = ui.new_child(builder.layout(centred));
+    let named = gettext(skin.locale, super::label(OptionId::AgentsOn));
+    if widgets::toggle(&mut switch, on, &named, palette).clicked() {
+        actions.push(Action::SetOption(crate::settings::OptionValue::AgentsOn(
+            !on,
+        )));
+    }
+}
+
+/// Paints `text` with its top left at `at`, and names it for screen
+/// readers.
+fn paint(ui: &Ui, text: &Laid, at: egui::Pos2) {
+    text.paint(ui.painter(), at);
+    widgets::announce(ui, Rect::from_min_size(at, text.size()), text.galley.text());
+}
+
+/// A group's heading, with the gap over it.
+fn heading(ui: &mut Ui, text: &'static str, skin: &Skin) {
+    ui.add_space(GROUP_GAP);
+    widgets::section_label(&skin.say(text), skin.look, skin.palette)
+        .layout(ui.ctx())
+        .label(ui);
+    ui.add_space(UNDER_HEADING);
+}
+
+/// A box `height` tall across the pane, its fill the panel's where
+/// `filled`, with a hairline round it. Returns where it is.
+fn boxed(ui: &mut Ui, height: f32, filled: bool, skin: &Skin) -> Rect {
+    let Skin { look, palette, .. } = *skin;
+    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), height), Sense::hover());
+    let corner = CornerRadius::same(look.radius);
+    if filled {
+        ui.painter().rect_filled(rect, corner, palette.panel);
+    }
+    ui.painter().rect_stroke(
+        rect,
+        corner,
+        Stroke::new(1.0, palette.surface),
+        StrokeKind::Inside,
+    );
+    rect
+}
+
+/// Whether the server runs, how agents reach it and who is connected, with
+/// Show Agents tab: the window's in front, or with Connections in front the
+/// last window's, disabled with no window open.
+fn status(ui: &mut Ui, app: &App, skin: &Skin, actions: &mut Vec<Action>) {
+    let Skin {
+        look,
+        palette,
+        locale,
+    } = *skin;
+    let role = TextRole::UiBody;
+    let on = app.agents_on();
+    let height = look.control_height + 2.0 * 10.0;
+    let rect = boxed(ui, height, true, skin);
+    let y = rect.center().y;
+    let mut x = rect.left() + PAD.x;
+    let dot = if on { palette.success } else { palette.dim };
+    ui.painter().circle_filled(pos2(x + 4.0, y), 4.0, dot);
+    x += 8.0 + 10.0;
+    let (state, how) = if on {
+        ("Running", "stdio · this computer only")
+    } else {
+        ("Off", "Agents can't connect")
+    };
+    x += widgets::paint_label(
+        ui,
+        x,
+        y,
+        Text::one(look, TextRole::UiBodyStrong, &skin.say(state), palette.text),
+    ) + 10.0;
+    let how = Text::one(look, role, &skin.say(how), palette.dim).layout(ui.ctx());
+    // At the right: the button, and before it who is connected.
+    let label = gettext(locale, "Show Agents tab");
+    let reason = gettext(locale, super::NO_WINDOW);
+    let mut button = ButtonSpec::new(&label);
+    if app.agents_window().is_none() {
+        button = button.disabled(&reason);
+    }
+    let width = button.width(ui, look);
+    let place = Rect::from_center_size(
+        pos2(rect.right() - PAD.x - width / 2.0, y),
+        vec2(width, look.control_height),
+    );
+    if button.show_at(ui, place, look, palette).clicked() {
+        actions.push(Action::ShowAgentsFromSettings(AgentsView::Activity));
+    }
+    let mut right = place.left() - 12.0;
+    if on {
+        let count = app.agents.sessions.len();
+        let (said, color) = match count {
+            0 => (
+                gettext(locale, "No agents connected").into_owned(),
+                palette.dim,
+            ),
+            count => (
+                ngettext(
+                    locale,
+                    "{n} agent connected",
+                    "{n} agents connected",
+                    u32::try_from(count).unwrap_or(u32::MAX),
+                )
+                .replace("{n}", &count.to_string()),
+                palette.success,
+            ),
+        };
+        let text = Text::one(look, TextRole::UiBodyStrong, &said, color).layout(ui.ctx());
+        // Whole or left out, as is how agents reach it.
+        if right - text.width() >= x + how.width() + 12.0 {
+            text.paint_right(ui.painter(), right, y);
+            widgets::announce(
+                ui,
+                Rect::from_center_size(pos2(right - text.width() / 2.0, y), text.size()),
+                &said,
+            );
+            right -= text.width() + 12.0;
+        }
+    }
+    if x + how.width() <= right {
+        how.paint_left(ui.painter(), x, y);
+        widgets::announce(
+            ui,
+            Rect::from_center_size(pos2(x + how.width() / 2.0, y), how.size()),
+            how.galley.text(),
+        );
+    }
+}
+
+/// What the switch's segment for `agent` says.
+fn agent_name(agent: Agent) -> &'static str {
+    match agent {
+        Agent::ClaudeCode => "Claude Code",
+        Agent::Cursor => "Cursor",
+        Agent::Other => "Other",
+    }
+}
+
+/// How `agent` takes what is under the switch.
+fn agent_hint(agent: Agent) -> &'static str {
+    match agent {
+        Agent::ClaudeCode => "Run this once in your project:",
+        Agent::Cursor => {
+            "Add its tabletist entry under mcpServers in ~/.cursor/mcp.json, or in .cursor/mcp.json in a project:"
+        }
+        Agent::Other => "An agent that starts MCP servers over stdio runs this command:",
+    }
+}
+
+/// Puts what connects `agent` on the clipboard, and says so. Nothing while
+/// it is still being worked out, as the app starts.
+fn copy_setup(ctx: &egui::Context, agent: Agent, locale: crate::i18n::Locale) {
+    if let Some(setup) = crate::endpoint::agent_setup_ready(agent) {
+        ctx.copy_text(setup.to_owned());
+        let said = match agent {
+            Agent::Cursor => COPIED_ENTRY,
+            Agent::ClaudeCode | Agent::Other => crate::ui::agents::COPIED_COMMAND,
+        };
+        crate::ui::toast::say(ctx, &gettext(locale, said));
+    }
+}
+
+/// The switch between the agents, what the chosen one takes, and that with
+/// Copy beside it.
+fn connect(ui: &mut Ui, agent: Agent, skin: &Skin, actions: &mut Vec<Action>) {
+    let Skin {
+        look,
+        palette,
+        locale,
+    } = *skin;
+    // The switch, and what the agent takes beside it, wrapped in what the
+    // switch leaves.
+    let words = Agent::ALL.map(|agent| gettext(locale, agent_name(agent)).into_owned());
+    let role = TextRole::FieldLabel;
+    let widest = words
+        .iter()
+        .map(|word| role.width(ui.ctx(), look.faces, word))
+        .fold(0.0, f32::max);
+    let size = vec2((widest + 20.0).ceil(), SEGMENT);
+    let names = words
+        .each_ref()
+        .map(|word| format!("{}: {word}", gettext(locale, "Connect an agent")));
+    let parts: Vec<Segment<'_>> = words
+        .iter()
+        .zip(&names)
+        .map(|(text, name)| Segment::Named { text, name })
+        .collect();
+    let selected = Agent::ALL
+        .iter()
+        .position(|shown| *shown == agent)
+        .unwrap_or(0);
+    let switch = 3.0 * size.x + 4.0;
+    let hint = Text::one(
+        look,
+        TextRole::Secondary,
+        &skin.say(agent_hint(agent)),
+        palette.dim,
+    )
+    .wrap((ui.available_width() - switch - 10.0).max(0.0))
+    .layout(ui.ctx());
+    let height = (SEGMENT + 4.0).max(hint.height());
+    let (line, _) = ui.allocate_exact_size(vec2(ui.available_width(), height), Sense::hover());
+    let place = Rect::from_min_size(line.min, vec2(switch, height));
+    let centred = egui::Layout::left_to_right(egui::Align::Center);
+    let builder = egui::UiBuilder::new().id_salt("agent").max_rect(place);
+    let mut row = ui.new_child(builder.layout(centred));
+    if let Some(picked) = widgets::segmented(&mut row, &parts, selected, size, look, palette) {
+        actions.push(Action::ChooseAgentSetup(Agent::ALL[picked]));
+    }
+    let at = row.min_rect().right() + 10.0;
+    paint(ui, &hint, pos2(at, line.center().y - hint.height() / 2.0));
+    ui.add_space(8.0);
+
+    // What the agent takes, in the code's face, a line of it at a time:
+    // a long path is cut by the box, and copied whole.
+    let setup = crate::endpoint::agent_setup_ready(agent).unwrap_or_else(|| {
+        ui.ctx()
+            .request_repaint_after(std::time::Duration::from_millis(100));
+        "…"
+    });
+    let mono = TextRole::MonoSecondary;
+    let line = (mono.row_height(ui.ctx(), look.faces) * 1.25).round();
+    let lines = setup.lines().count().max(1);
+    let label = gettext(locale, "Copy");
+    let copy = ButtonSpec::new(&label).icon(Icon::Copy);
+    let copy_width = copy.width(ui, look);
+    let height = (lines as f32 * line + 20.0).max(look.control_height);
+    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), height), Sense::hover());
+    let code = rect.with_max_x((rect.right() - copy_width - 8.0).max(rect.left()));
+    let corner = CornerRadius::same(look.radius);
+    ui.painter().rect_filled(code, corner, palette.panel);
+    ui.painter().rect_stroke(
+        code,
+        corner,
+        Stroke::new(1.0, palette.surface),
+        StrokeKind::Inside,
+    );
+    // A line gives way in its middle to a box too narrow for it.
+    let fits = (code.width() - 24.0).max(0.0);
+    let ctx = ui.ctx().clone();
+    for (index, text) in setup.lines().enumerate() {
+        let y = code.top() + 10.0 + (index as f32 + 0.5) * line;
+        let shown =
+            crate::ui::grid::ellipsize(text, fits, true, |text| mono.width(&ctx, look.faces, text));
+        Text::one(look, mono, &shown, palette.text)
+            .layout(ui.ctx())
+            .paint_left(ui.painter(), code.left() + 12.0, y);
+    }
+    widgets::announce(ui, code, setup);
+    let at = Rect::from_min_size(
+        pos2(rect.right() - copy_width, rect.top()),
+        vec2(copy_width, look.control_height),
+    );
+    if copy.show_at(ui, at, look, palette).clicked() {
+        copy_setup(ui.ctx(), agent, locale);
+    }
+    ui.add_space(6.0);
+    widgets::label(
+        ui,
+        TextRole::Secondary,
+        &skin.say("Each agent gets its own name in the Agents tab."),
+        palette.dim,
+        look,
+    );
+}
+
+/// Where access is set: in each connection, with the button that opens
+/// Connections.
+fn access(ui: &mut Ui, skin: &Skin, actions: &mut Vec<Action>) {
+    let Skin {
+        look,
+        palette,
+        locale,
+    } = *skin;
+    let label = gettext(locale, "Open Connections");
+    let key = crate::ui::keys::written(ui.ctx(), look, Command::Connections);
+    let button = ButtonSpec::new(&label).shortcut(&key);
+    let width = button.width(ui, look);
+    let words = (ui.available_width() - 2.0 * PAD.x - width - 12.0).max(0.0);
+    let strong = Text::one(
+        look,
+        TextRole::UiBodyStrong,
+        &skin.say("Set for each connection, in Connections"),
+        palette.text,
+    )
+    .wrap(words)
+    .layout(ui.ctx());
+    let said = Text::one(
+        look,
+        TextRole::Secondary,
+        &skin.say("Every connection has an Agents section: Off or Read, with limits on rows and time. New connections start Off."),
+        palette.dim,
+    )
+    .wrap(words)
+    .layout(ui.ctx());
+    let height = (strong.height() + 2.0 + said.height()).max(look.control_height) + 2.0 * PAD.y;
+    let rect = boxed(ui, height, true, skin);
+    let top = rect.center().y - (strong.height() + 2.0 + said.height()) / 2.0;
+    paint(ui, &strong, pos2(rect.left() + PAD.x, top));
+    paint(
+        ui,
+        &said,
+        pos2(rect.left() + PAD.x, top + strong.height() + 2.0),
+    );
+    let place = Rect::from_center_size(
+        pos2(rect.right() - PAD.x - width / 2.0, rect.center().y),
+        vec2(width, look.control_height),
+    );
+    if button.show_at(ui, place, look, palette).clicked() {
+        actions.push(Action::CloseDialog);
+        actions.push(Action::ShowConnections);
+    }
+}
+
+/// What keeps agents within bounds, in a box of its own.
+fn safety(ui: &mut Ui, skin: &Skin) {
+    let Skin { look, palette, .. } = *skin;
+    let width = (ui.available_width() - 2.0 * PAD.x).max(0.0);
+    let strong = Text::one(
+        look,
+        TextRole::UiBodyStrong,
+        &skin.say("How it stays safe"),
+        palette.text,
+    )
+    .layout(ui.ctx());
+    let said = Text::one(
+        look,
+        TextRole::Secondary,
+        &skin.say("Reads run in a read-only transaction with each connection's limits. No DDL, no multiple statements, no COPY. Agents see what the connection's database user can see, and never more."),
+        palette.secondary,
+    )
+    .wrap(width)
+    .layout(ui.ctx());
+    let height = strong.height() + 6.0 + said.height() + 2.0 * PAD.y;
+    let rect = boxed(ui, height, false, skin);
+    let left = rect.left() + PAD.x;
+    paint(ui, &strong, pos2(left, rect.top() + PAD.y));
+    paint(
+        ui,
+        &said,
+        pos2(left, rect.top() + PAD.y + strong.height() + 6.0),
+    );
+}
diff --git a/src/ui/settings/mod.rs b/src/ui/settings/mod.rs
index dd2f92c..f7ba586 100644
--- a/src/ui/settings/mod.rs
+++ b/src/ui/settings/mod.rs
@@ -1,9 +1,11 @@
-//! The Settings window: every option of the General tab, changed where it
-//! is shown and saved at once. The terminal look draws a screen over the
-//! whole window with its keys in the footer (`terminal.rs`); the other
-//! looks draw a sheet over the dimmed window, with a control for each
-//! option (`sheet.rs`).
+//! The Settings window: the options of its panes, General and, where
+//! agents can reach the app, Agents, each changed where it is shown and
+//! saved at once. The terminal look draws a screen over the whole window
+//! with its keys in the footer (`terminal.rs`); the other looks draw a
+//! sheet over the dimmed window, with a control for each option
+//! (`sheet.rs`, and `agents.rs` for the Agents pane).
 
+mod agents;
 mod file_pane;
 mod sheet;
 mod terminal;
@@ -11,9 +13,9 @@ mod terminal;
 use std::borrow::Cow;
 
 use crate::app::App;
-use crate::i18n::{Locale, gettext};
+use crate::i18n::{Locale, gettext, ngettext};
 use crate::keymap::{Command, Keymap, Layout, Scope, Spelled, Stroke};
-use crate::model::{Action, Dialog};
+use crate::model::{Action, AgentsView, Dialog, SettingsDialog, SettingsPane, SettingsRow};
 use crate::settings::{OptionId, OptionValue, Settings, Timestamps};
 
 /// The label of an option's row. English: the caller translates, and the
@@ -24,6 +26,9 @@ pub(super) fn label(option: OptionId) -> &'static str {
         OptionId::Timestamps => "Timestamps",
         OptionId::GroupDigits => "Numbers",
         OptionId::ValueTags => "Value tags",
+        OptionId::AgentsOn => "Let agents use Tabletist",
+        OptionId::LogDays => "Keep the activity log",
+        OptionId::AgentsNotify => "Notify when an agent connects",
     }
 }
 
@@ -32,10 +37,38 @@ pub(super) fn small_print(option: OptionId) -> Option<&'static str> {
     match option {
         OptionId::PageSize => Some("Table view; the SQL editor has its own limit"),
         OptionId::ValueTags => Some("Colors for enum, CHECK (…IN…) and boolean columns"),
-        OptionId::Timestamps | OptionId::GroupDigits => None,
+        OptionId::LogDays => Some("Every call and its SQL, never the rows. On this computer only"),
+        OptionId::Timestamps
+        | OptionId::GroupDigits
+        | OptionId::AgentsOn
+        | OptionId::AgentsNotify => None,
     }
 }
 
+/// The days the log keeps a call, as the window writes them.
+pub(super) fn days(locale: Locale, days: u32) -> String {
+    ngettext(locale, "{n} day", "{n} days", days).replace("{n}", &days.to_string())
+}
+
+/// Whether an option of on and off is on.
+pub(super) fn is_on(value: OptionValue) -> bool {
+    matches!(
+        value,
+        OptionValue::ValueTags(true)
+            | OptionValue::AgentsOn(true)
+            | OptionValue::AgentsNotify(true)
+    )
+}
+
+/// The settings as the window shows them: the server's switch as it is in
+/// force, off where a change made outside the app that turned it on is
+/// held.
+pub(super) fn shown(app: &App) -> Settings {
+    let mut settings = app.settings.clone();
+    settings.mcp_enabled = app.agents_on();
+    settings
+}
+
 /// A timestamp as the grid shows one at each precision.
 pub(super) fn sample_timestamp(timestamps: Timestamps) -> &'static str {
     match timestamps {
@@ -95,9 +128,40 @@ pub(super) fn choices(option: OptionId, settings: &Settings) -> Vec<(Said, Optio
             (Word("off"), OptionValue::ValueTags(false)),
             (Word("on"), OptionValue::ValueTags(true)),
         ],
+        OptionId::AgentsOn => vec![
+            (Word("off"), OptionValue::AgentsOn(false)),
+            (Word("on"), OptionValue::AgentsOn(true)),
+        ],
+        OptionId::LogDays => vec![
+            (Word("fewer"), settings.stepped(option, false)),
+            (Word("more"), settings.stepped(option, true)),
+        ],
+        OptionId::AgentsNotify => vec![
+            (Word("off"), OptionValue::AgentsNotify(false)),
+            (Word("on"), OptionValue::AgentsNotify(true)),
+        ],
     }
 }
 
+/// What Enter does on `row`, where it does something: the Agents tab, its
+/// log, or Connections. Showing the tab needs a window: with none open,
+/// nothing.
+pub(super) fn opened(app: &App, row: SettingsRow) -> Option<Vec<Action>> {
+    let show = |view| {
+        app.agents_window()
+            .map(|_| vec![Action::ShowAgentsFromSettings(view)])
+    };
+    match row {
+        SettingsRow::AgentsStatus => show(AgentsView::Activity),
+        SettingsRow::Option(OptionId::LogDays) => show(AgentsView::Log),
+        SettingsRow::AgentsAccess => Some(vec![Action::CloseDialog, Action::ShowConnections]),
+        SettingsRow::Option(_) => None,
+    }
+}
+
+/// What a disabled Show Agents tab and Show log say, with no window open.
+pub(super) const NO_WINDOW: &str = "Open a connection to see its agents";
+
 /// What one of the screen's keys asks for.
 #[derive(Clone, Copy, Debug, PartialEq, Eq)]
 enum Asked {
@@ -114,6 +178,14 @@ enum Asked {
     },
     /// Flip the cursor's option, if it has two values.
     Flip,
+    /// Show the next pane (`true`) or the one before.
+    Pane {
+        forward: bool,
+    },
+    /// Open what the cursor's row shows.
+    Open,
+    /// Copy the command that connects an agent.
+    Copy,
 }
 
 /// What `event` asks of the screen, if it is a press of one of its keys,
@@ -133,6 +205,9 @@ fn asked(keymap: &Keymap, event: &egui::Event, held: egui::Modifiers, free: bool
         Command::SettingsMove,
         Command::SettingsChange,
         Command::SettingsToggle,
+        Command::SettingsPane,
+        Command::SettingsOpenRow,
+        Command::SettingsCopyCommand,
     ];
     // A letter is read as the key that types it, with Shift for its
     // capital and nothing else held.
@@ -148,7 +223,11 @@ fn asked(keymap: &Keymap, event: &egui::Event, held: egui::Modifiers, free: bool
         _ => None,
     };
     for command in commands {
-        for keys in keymap.chords_now(layout, command, Scope::Prompt, true) {
+        // A pane's two chords are the one before, then the next.
+        for (nth, keys) in keymap
+            .chords_now(layout, command, Scope::Prompt, true)
+            .enumerate()
+        {
             let pressed = match crate::keymap::spell(keys) {
                 Ok(Spelled::Strokes(strokes)) => match strokes.as_slice() {
                     [Stroke::Text(letter)] => typed == Some(*letter),
@@ -173,6 +252,9 @@ fn asked(keymap: &Keymap, event: &egui::Event, held: egui::Modifiers, free: bool
                 Command::SettingsChange => Asked::Step {
                     forward: matches!(keys, "l" | "right"),
                 },
+                Command::SettingsPane => Asked::Pane { forward: nth == 1 },
+                Command::SettingsOpenRow => Asked::Open,
+                Command::SettingsCopyCommand => Asked::Copy,
                 _ => Asked::Flip,
             });
         }
@@ -188,13 +270,15 @@ fn asked(keymap: &Keymap, event: &egui::Event, held: egui::Modifiers, free: bool
 /// before it left: the cursor where they moved it, an option as they set
 /// it. `j` then `l` changes the row `j` moved to, and `l` twice steps
 /// twice.
-fn keys(app: &App, ctx: &egui::Context, row: usize, actions: &mut Vec<Action>) {
+fn keys(app: &App, ctx: &egui::Context, dialog: &SettingsDialog, actions: &mut Vec<Action>) {
     let free = !super::focus::on_control(ctx);
-    let last = OptionId::ALL.len() - 1;
     // As `App::apply` will have them once it has run the actions so far.
-    let mut cursor = row;
-    let mut settings = app.settings.clone();
+    let mut cursor = dialog.row;
+    let mut pane = dialog.pane;
+    let mut settings = shown(app);
     let keymap = app.keymap.clone();
+    // Copied once the input is let go: the clipboard is the context's.
+    let mut copy = false;
     ctx.input_mut(|input| {
         let held = input.modifiers;
         input.events.retain(|event| {
@@ -212,14 +296,48 @@ fn keys(app: &App, ctx: &egui::Context, row: usize, actions: &mut Vec<Action>) {
                 }
                 Asked::Close => actions.push(Action::CloseDialog),
                 Asked::Move(by) => {
+                    let last = pane.rows().len() - 1;
                     cursor = cursor.saturating_add_signed(by).min(last);
                     actions.push(Action::MoveSettingsRow(by));
                 }
+                Asked::Pane { forward } => {
+                    let listed = SettingsPane::listed();
+                    let at = listed.iter().position(|shown| *shown == pane).unwrap_or(0);
+                    let next = match forward {
+                        true => (at + 1).min(listed.len() - 1),
+                        false => at.saturating_sub(1),
+                    };
+                    if listed[next] != pane {
+                        pane = listed[next];
+                        cursor = 0;
+                        actions.push(Action::ShowSettingsPane(pane));
+                    }
+                }
+                // What the row opens, on a row that opens something: else
+                // the key is left.
+                Asked::Open => {
+                    let opens = pane.rows().get(cursor).and_then(|row| opened(app, *row));
+                    let Some(opens) = opens else {
+                        return true;
+                    };
+                    // A fresh press only: a held Enter would go on to the
+                    // screen it opened.
+                    if !repeat {
+                        actions.extend(opens);
+                    }
+                }
+                Asked::Copy => {
+                    if pane != SettingsPane::Agents {
+                        return true;
+                    }
+                    copy |= !repeat;
+                }
                 Asked::Reset | Asked::Step { .. } | Asked::Flip => {
-                    // A cursor on no row has no option to change: the key
-                    // is left, and the ones above still close the screen
-                    // or move it back onto a row.
-                    let Some(option) = OptionId::ALL.get(cursor).copied() else {
+                    // A cursor on no option has no option to change: the
+                    // key is left, and the ones above still close the
+                    // screen or move it back onto a row.
+                    let row = pane.rows().get(cursor);
+                    let Some(option) = row.and_then(|row| row.option()) else {
                         return true;
                     };
                     let value = match asked {
@@ -236,21 +354,23 @@ fn keys(app: &App, ctx: &egui::Context, row: usize, actions: &mut Vec<Action>) {
             false
         });
     });
+    if copy {
+        crate::ui::agents::copy_command(ctx, app.locale);
+    }
 }
 
 pub fn show(app: &mut App, ctx: &egui::Context) {
     let Some(Dialog::Settings(dialog)) = &app.dialog else {
         return;
     };
-    let (row, resetting) = (dialog.row, dialog.resetting);
     let mut actions = Vec::new();
     if app.look.terminal {
         // The screen's keys act on its cursor. The sheet has none: its
         // controls are reached with Tab.
-        keys(app, ctx, row, &mut actions);
-        terminal::show(app, ctx, row, &mut actions);
+        keys(app, ctx, dialog, &mut actions);
+        terminal::show(app, ctx, dialog, &mut actions);
     } else {
-        sheet::show(app, ctx, resetting, &mut actions);
+        sheet::show(app, ctx, dialog, &mut actions);
     }
     app.actions.extend(actions);
 }
diff --git a/src/ui/settings/sheet.rs b/src/ui/settings/sheet.rs
index 52d256d..fa21017 100644
--- a/src/ui/settings/sheet.rs
+++ b/src/ui/settings/sheet.rs
@@ -1,13 +1,14 @@
 //! The Settings window in the looks that are not a terminal's: a sheet
-//! over the dimmed window, with the nav at its left and the tab's options
-//! in rows, each with a control that changes it. Under the rows, the footer
-//! says where the file is and what can be done with it.
+//! over the dimmed window, with the nav of its panes at its left and the
+//! pane's options in rows, each with a control that changes it (the Agents
+//! pane is `agents.rs`'s). Under the rows, the footer says where the file
+//! is and what can be done with it.
 
 use egui::{CornerRadius, Rect, Sense, Stroke, Ui, WidgetInfo, WidgetType, pos2, vec2};
 
 use crate::app::App;
 use crate::i18n::{Locale, gettext, ngettext};
-use crate::model::Action;
+use crate::model::{Action, SettingsDialog, SettingsPane};
 use crate::settings::{OptionId, OptionValue, Settings, Timestamps};
 use crate::theme::{Look, Palette};
 use crate::typography::{Laid, Text, TextRole};
@@ -59,20 +60,26 @@ const LINK: f32 = 18.0;
 
 /// How the sheet draws: the look, the palette and the language.
 #[derive(Clone, Copy)]
-struct Skin<'a> {
-    look: &'a Look,
-    palette: &'a Palette,
-    locale: Locale,
+pub(super) struct Skin<'a> {
+    pub(super) look: &'a Look,
+    pub(super) palette: &'a Palette,
+    pub(super) locale: Locale,
 }
 
 impl Skin<'_> {
     /// `text` translated.
-    fn say(&self, text: &'static str) -> String {
+    pub(super) fn say(&self, text: &'static str) -> String {
         gettext(self.locale, text).into_owned()
     }
 }
 
-pub(super) fn show(app: &App, ctx: &egui::Context, resetting: bool, actions: &mut Vec<Action>) {
+pub(super) fn show(
+    app: &App,
+    ctx: &egui::Context,
+    dialog: &SettingsDialog,
+    actions: &mut Vec<Action>,
+) {
+    let resetting = dialog.resetting;
     let skin = Skin {
         look: &app.look,
         palette: &app.palette,
@@ -108,7 +115,7 @@ pub(super) fn show(app: &App, ctx: &egui::Context, resetting: bool, actions: &mu
             // laid out after it: its fill waits here, under its words, for
             // that height.
             let fill = ui.painter().add(egui::Shape::Noop);
-            nav(ui, room.min, &skin);
+            nav(ui, room.min, dialog.pane, &skin, actions);
             // No bottom of its own: its rows end where `content` is told to
             // end them. The sheet's bottom is the one the frame before left
             // it, and rows held to that would get their height back a
@@ -125,7 +132,12 @@ pub(super) fn show(app: &App, ctx: &egui::Context, resetting: bool, actions: &mu
             let limit = room.top() + screen.height() - 2.0 * MARGIN - foot;
             let builder = egui::UiBuilder::new().id_salt("content").max_rect(pane);
             let mut pane = ui.new_child(builder.layout(egui::Layout::top_down(egui::Align::Min)));
-            content(&mut pane, &app.settings, limit, &skin, actions);
+            match dialog.pane {
+                SettingsPane::General => content(&mut pane, &app.settings, limit, &skin, actions),
+                SettingsPane::Agents => {
+                    super::agents::content(&mut pane, app, dialog.agent, limit, &skin, actions);
+                }
+            }
             let rows = pane.min_rect().bottom();
             let bottom = rows + foot;
             ui.allocate_rect(room.with_max_y(bottom), Sense::hover());
@@ -163,9 +175,15 @@ pub(super) fn show(app: &App, ctx: &egui::Context, resetting: bool, actions: &mu
     }
 }
 
-/// The window's name and its tabs, down the left from `corner`: the one
-/// tab there is, which is the one shown.
-fn nav(ui: &mut Ui, corner: egui::Pos2, skin: &Skin) {
+/// The window's name and its panes, down the left from `corner`, the one
+/// shown filled: a click shows another.
+fn nav(
+    ui: &mut Ui,
+    corner: egui::Pos2,
+    shown: SettingsPane,
+    skin: &Skin,
+    actions: &mut Vec<Action>,
+) {
     let Skin { look, palette, .. } = *skin;
     let role = TextRole::StateTitle;
     let line = role.row_height(ui.ctx(), look.faces);
@@ -176,24 +194,43 @@ fn nav(ui: &mut Ui, corner: egui::Pos2, skin: &Skin) {
         top + line / 2.0,
         Text::one(look, role, &skin.say("Settings"), palette.text),
     );
-    // Up to the nav's rule at the right.
-    let item = Rect::from_min_size(
-        pos2(corner.x + NAV_PAD.x, top + line + 10.0),
-        vec2(NAV - 1.0 - 2.0 * NAV_PAD.x, NAV_ITEM),
-    );
-    let name = skin.say("General");
-    let button = ui.interact(item, ui.id().with("general"), Sense::click());
-    button.widget_info(|| WidgetInfo::selected(WidgetType::Button, true, true, &name));
-    ui.painter()
-        .rect_filled(item, CornerRadius::same(5), palette.selection);
-    widgets::paint_text(
-        ui,
-        item.left() + 8.0,
-        item.center().y,
-        Text::one(look, TextRole::UiBodyStrong, &name, palette.accent_hover),
-    );
-    // An item of a list: the ring is inside its fill, as a row's is.
-    focus::hint(ui, &button, item, Ring::Inset { radius: 5 });
+    for (index, pane) in SettingsPane::listed().iter().copied().enumerate() {
+        // Up to the nav's rule at the right, 2 apart.
+        let item = Rect::from_min_size(
+            pos2(
+                corner.x + NAV_PAD.x,
+                top + line + 10.0 + index as f32 * (NAV_ITEM + 2.0),
+            ),
+            vec2(NAV - 1.0 - 2.0 * NAV_PAD.x, NAV_ITEM),
+        );
+        let on = pane == shown;
+        let name = skin.say(pane.name());
+        let button = ui.interact(item, ui.id().with(pane.name()), Sense::click());
+        button.widget_info(|| WidgetInfo::selected(WidgetType::Button, true, on, &name));
+        if button.clicked() && !on {
+            actions.push(Action::ShowSettingsPane(pane));
+        }
+        let (fill, role, color) = if on {
+            (
+                palette.selection,
+                TextRole::UiBodyStrong,
+                palette.accent_hover,
+            )
+        } else if button.hovered() {
+            (palette.surface_hover, TextRole::UiBody, palette.text)
+        } else {
+            (egui::Color32::TRANSPARENT, TextRole::UiBody, palette.text)
+        };
+        ui.painter().rect_filled(item, CornerRadius::same(5), fill);
+        widgets::paint_text(
+            ui,
+            item.left() + 8.0,
+            item.center().y,
+            Text::one(look, role, &name, color),
+        );
+        // An item of a list: the ring is inside its fill, as a row's is.
+        focus::hint(ui, &button, item, Ring::Inset { radius: 5 });
+    }
 }
 
 /// The tab: its title, what it is for, and its options under the heading
@@ -221,7 +258,7 @@ fn content(ui: &mut Ui, settings: &Settings, limit: f32, skin: &Skin, actions: &
         .layout(ui.ctx())
         .label(ui);
     ui.add_space(2.0);
-    let column = label_column(ui, skin);
+    let column = label_column(ui, &OptionId::GENERAL, skin);
     let room = (limit - ui.cursor().top()).max(ROWS_MIN);
     egui::ScrollArea::vertical()
         .id_salt("rows")
@@ -230,21 +267,31 @@ fn content(ui: &mut Ui, settings: &Settings, limit: f32, skin: &Skin, actions: &
         .show(ui, |ui| {
             // The last row stands on the footer's rule: one of its own
             // right over that would read as a double line.
-            let last = OptionId::ALL.len() - 1;
-            for (index, option) in OptionId::ALL.into_iter().enumerate() {
-                row(ui, option, index < last, column, settings, skin, actions);
+            let last = OptionId::GENERAL.len() - 1;
+            for (index, option) in OptionId::GENERAL.into_iter().enumerate() {
+                row(
+                    ui,
+                    option,
+                    index < last,
+                    column,
+                    settings,
+                    skin,
+                    actions,
+                    None,
+                );
             }
         });
 }
 
 /// How wide the rows' labels have it: `LABEL`, or what the widest small
-/// print takes in a face that writes it wider, so that it stays on one
-/// line. A sheet too narrow for that and the controls takes from the
-/// labels, and the small print wraps.
-fn label_column(ui: &Ui, skin: &Skin) -> f32 {
+/// print of `options` takes in a face that writes it wider, so that it
+/// stays on one line. A sheet too narrow for that and the controls takes
+/// from the labels, and the small print wraps.
+pub(super) fn label_column(ui: &Ui, options: &[OptionId], skin: &Skin) -> f32 {
     let role = TextRole::Secondary;
-    let widest = OptionId::ALL
-        .into_iter()
+    let widest = options
+        .iter()
+        .copied()
         .filter_map(small_print)
         .map(|print| role.width(ui.ctx(), skin.look.faces, &skin.say(print)))
         .fold(LABEL, f32::max);
@@ -252,10 +299,19 @@ fn label_column(ui: &Ui, skin: &Skin) -> f32 {
     widest.ceil().min(room).max(0.0)
 }
 
+/// A button after a row's control: what it says, why it cannot be pressed
+/// while it cannot, and what a press does.
+pub(super) struct After<'a> {
+    pub(super) text: &'a str,
+    pub(super) disabled: Option<&'a str>,
+    pub(super) action: Action,
+}
+
 /// One option's row: its label over its small print in a column `column`
-/// wide, its control, and the hint that shows what the value does, over a
-/// rule when it is `ruled`.
-fn row(
+/// wide, its control and the button `after` it, and the hint that shows
+/// what the value does, over a rule when it is `ruled`.
+#[allow(clippy::too_many_arguments)] // one row: its place, what it shows, what it pushes
+pub(super) fn row(
     ui: &mut Ui,
     option: OptionId,
     ruled: bool,
@@ -263,6 +319,7 @@ fn row(
     settings: &Settings,
     skin: &Skin,
     actions: &mut Vec<Action>,
+    after: Option<After<'_>>,
 ) {
     let Skin { look, palette, .. } = *skin;
     let name = Text::one(
@@ -304,6 +361,19 @@ fn row(
         .max_rect(place);
     let mut line = ui.new_child(builder.layout(centred));
     control(&mut line, option, settings, skin, actions);
+    if let Some(after) = after {
+        line.add_space(HINT_GAP);
+        let mut button = widgets::ButtonSpec::new(after.text);
+        if let Some(reason) = after.disabled {
+            button = button.disabled(reason);
+        }
+        if button
+            .show(&mut line, look.control_height, look, palette)
+            .clicked()
+        {
+            actions.push(after.action);
+        }
+    }
     // Tab reaches a control of a row that is scrolled out of view, where
     // its ring is cut with it: the row comes to the keyboard. Jump, not
     // animate: the next Tab may come at once.
@@ -333,7 +403,8 @@ fn control_height(option: OptionId, look: &Look) -> f32 {
         OptionId::PageSize => look.control_height,
         // The track has 2 over its segments and under them.
         OptionId::Timestamps | OptionId::GroupDigits => SEGMENT + 4.0,
-        OptionId::ValueTags => SWITCH,
+        OptionId::ValueTags | OptionId::AgentsOn | OptionId::AgentsNotify => SWITCH,
+        OptionId::LogDays => look.control_height,
     }
 }
 
@@ -396,16 +467,50 @@ fn control(
                 actions.push(Action::SetOption(OptionValue::GroupDigits(picked == 0)));
             }
         }
-        OptionId::ValueTags => {
-            if widgets::toggle(ui, settings.value_tags, &name, palette).clicked() {
-                actions.push(Action::SetOption(OptionValue::ValueTags(
-                    !settings.value_tags,
-                )));
+        OptionId::ValueTags | OptionId::AgentsOn | OptionId::AgentsNotify => {
+            let on = super::is_on(settings.value(option));
+            if widgets::toggle(ui, on, &name, palette).clicked()
+                && let Some(flipped) = settings.flipped(option)
+            {
+                actions.push(Action::SetOption(flipped));
             }
         }
+        OptionId::LogDays => {
+            let shown = super::days(skin.locale, settings.mcp_log_days);
+            let dropdown = menu::Dropdown::new("log-days", &shown, MENU);
+            let menu = dropdown.show(ui, look, palette, |ui| {
+                for days in log_days(settings.mcp_log_days) {
+                    let text = super::days(skin.locale, days);
+                    let entry_name = format!("{name} {text}");
+                    let entry = menu::Item::choice(&text, days == settings.mcp_log_days)
+                        .name(&entry_name)
+                        .show(ui, look, palette);
+                    if entry.clicked() {
+                        actions.push(Action::SetOption(OptionValue::LogDays(days)));
+                        ui.close();
+                    }
+                }
+            });
+            menu.response.widget_info(|| {
+                let mut info = WidgetInfo::labeled(WidgetType::ComboBox, true, &name);
+                info.current_text_value = Some(shown.clone());
+                info
+            });
+        }
     }
 }
 
+/// The days the log's menu offers: the list, with the days in use in their
+/// place when the file set others.
+fn log_days(current: u32) -> Vec<u32> {
+    let mut days = Settings::LOG_DAYS.to_vec();
+    if !days.contains(&current) {
+        days.push(current);
+        days.sort_unstable();
+    }
+    days
+}
+
 /// A segmented control of `words` for the option called `option`, the one
 /// at `selected` chosen. Returns the index of a segment picked this frame.
 /// The segments are of one width: the widest word's, with 10 at each side.
@@ -466,6 +571,7 @@ fn hint(ui: &Ui, at: egui::Pos2, right: f32, option: OptionId, settings: &Settin
             palette.dim,
         ),
         OptionId::ValueTags => return tags(ui, at, right, settings.value_tags, skin),
+        OptionId::AgentsOn | OptionId::LogDays | OptionId::AgentsNotify => return,
     };
     let text = text.layout(ui.ctx());
     if at.x + text.width() <= right {
diff --git a/src/ui/settings/terminal.rs b/src/ui/settings/terminal.rs
index ace5bdb..88889b7 100644
--- a/src/ui/settings/terminal.rs
+++ b/src/ui/settings/terminal.rs
@@ -1,6 +1,6 @@
 //! The Settings window as the terminal look draws it: a screen over the
-//! whole window. A header, the app's notice while it has one, a nav of one
-//! item, the options in rows under a cursor, the settings file beside them
+//! whole window. A header, the app's notice while it has one, a nav of its
+//! panes, the pane's rows under a cursor, the settings file beside them
 //! where the window has the room (`file_pane.rs` draws it in the place it
 //! is given here), and the screen's keys in the footer.
 
@@ -8,9 +8,9 @@ use egui::emath::GuiRounding as _;
 use egui::{CornerRadius, Rect, Sense, Stroke, StrokeKind, Ui, WidgetInfo, WidgetType, pos2, vec2};
 
 use crate::app::App;
-use crate::i18n::{Locale, gettext};
+use crate::i18n::{Locale, gettext, ngettext};
 use crate::keymap::Command;
-use crate::model::Action;
+use crate::model::{Action, SettingsDialog, SettingsPane, SettingsRow};
 use crate::settings::{OptionId, OptionValue, Settings};
 use crate::theme::{Look, Palette};
 use crate::typography::{Text, TextRole};
@@ -18,7 +18,7 @@ use crate::ui::focus::{self, Ring};
 use crate::ui::value_tags::terminal_slots;
 use crate::ui::widgets::{self, ButtonSpec};
 
-use super::{Said, choices, file_pane, label, sample_timestamp};
+use super::{Said, choices, file_pane, label, opened, sample_timestamp};
 
 /// The header's and the footer's heights, each without its rule.
 const HEADER: f32 = 40.0;
@@ -50,6 +50,9 @@ pub(super) struct Skin<'a> {
     pub(super) look: &'a Look,
     pub(super) palette: &'a Palette,
     pub(super) locale: Locale,
+    /// Whether a connection window is open: what Enter on the Agents
+    /// pane's status and log rows shows the tab of.
+    pub(super) window: bool,
 }
 
 impl Skin<'_> {
@@ -78,11 +81,19 @@ struct Cell {
     on_cursor: bool,
 }
 
-pub(super) fn show(app: &App, ctx: &egui::Context, row: usize, actions: &mut Vec<Action>) {
+pub(super) fn show(
+    app: &App,
+    ctx: &egui::Context,
+    dialog: &SettingsDialog,
+    actions: &mut Vec<Action>,
+) {
+    let (pane, row) = (dialog.pane, dialog.row);
+    let settings = super::shown(app);
     let skin = Skin {
         look: &app.look,
         palette: &app.palette,
         locale: app.locale,
+        window: app.agents_window().is_some(),
     };
     let screen = ctx.content_rect();
     let id = egui::Id::new("settings");
@@ -102,44 +113,49 @@ pub(super) fn show(app: &App, ctx: &egui::Context, row: usize, actions: &mut Vec
             ui.set_width(screen.width());
             ui.set_height(screen.height());
             ui.spacing_mut().item_spacing = egui::Vec2::ZERO;
-            header(ui, &skin);
+            header(ui, pane, &skin);
             let notice = notice(ui, app, actions);
             // What the header, a notice, the footer and their rules leave.
             let height = (screen.height() - HEADER - notice - FOOTER - 2.0).max(0.0);
             let (body, _) =
                 ui.allocate_exact_size(vec2(ui.available_width(), height), Sense::hover());
-            let (side, pane) = body.split_left_right_at_x((body.left() + NAV).min(body.right()));
-            nav(ui, side, &skin);
+            let (side, place) = body.split_left_right_at_x((body.left() + NAV).min(body.right()));
+            nav(ui, side, pane, &skin, actions);
             // The rows scroll when the window is short, so the header and
             // the footer stay on screen. They start past the nav's rule.
-            let pane = pane.with_min_x((pane.left() + 1.0).min(pane.right()));
-            let option = OptionId::ALL.get(row).copied();
+            let place = place.with_min_x((place.left() + 1.0).min(place.right()));
+            let cursor = pane.rows().get(row).copied();
+            let option = cursor.and_then(SettingsRow::option);
             // The file has the right of the rows, in a window wide enough
             // for both.
-            let (pane, beside) = if screen.width() >= FILE_FROM {
+            let (place, beside) = if screen.width() >= FILE_FROM {
                 let width = FILE.min(screen.width() * FILE_SHARE);
-                let edge = (pane.right() - width).round_to_pixels(ui.pixels_per_point());
-                let (pane, beside) = pane.split_left_right_at_x(edge);
-                (pane, Some(beside))
+                let edge = (place.right() - width).round_to_pixels(ui.pixels_per_point());
+                let (place, beside) = place.split_left_right_at_x(edge);
+                (place, Some(beside))
             } else {
-                (pane, None)
+                (place, None)
             };
-            let mut list = ui.new_child(egui::UiBuilder::new().id_salt("rows").max_rect(pane));
+            let mut list = ui.new_child(egui::UiBuilder::new().id_salt("rows").max_rect(place));
+            // Each pane scrolls on its own.
             egui::ScrollArea::vertical()
+                .id_salt(pane.name())
                 .auto_shrink([false, false])
-                .show(&mut list, |ui| {
-                    rows(ui, &app.settings, row, &skin, actions);
+                .show(&mut list, |ui| match pane {
+                    SettingsPane::General => rows(ui, &settings, row, &skin, actions),
+                    SettingsPane::Agents => agent_rows(ui, app, &settings, row, &skin, actions),
                 });
             if let Some(beside) = beside {
                 file_pane::show(ui, beside, app, option, &skin);
             }
-            footer(ui, option, &app.settings, &skin, actions);
+            let opens = cursor.and_then(|cursor| opened(app, cursor).map(|opens| (cursor, opens)));
+            footer(ui, pane, option, opens, &settings, &skin, actions);
         });
 }
 
-/// One tinted line: the title and the tab it shows, and at the right what
-/// a change does and the key that opens the screen.
-fn header(ui: &mut Ui, skin: &Skin) {
+/// One tinted line: the title and the pane it shows, and at the right
+/// what a change does and the key that opens the screen.
+fn header(ui: &mut Ui, pane: SettingsPane, skin: &Skin) {
     let Skin { look, palette, .. } = *skin;
     let (rect, _) =
         ui.allocate_exact_size(vec2(ui.available_width(), HEADER + 1.0), Sense::hover());
@@ -163,7 +179,7 @@ fn header(ui: &mut Ui, skin: &Skin) {
         ui,
         x,
         y,
-        Text::one(look, TextRole::OBody, &skin.say("General"), palette.dim),
+        Text::one(look, TextRole::OBody, &skin.say(pane.name()), palette.dim),
     );
     let role = widgets::secondary(look);
     let note = format!("{} · ", skin.say("Changes apply right away"));
@@ -216,44 +232,84 @@ fn mark(ui: &Ui, rect: Rect, palette: &Palette) {
     painter.rect_filled(bar, CornerRadius::ZERO, palette.accent);
 }
 
-/// The tabs, down the left: the one there is, which is the one shown.
-fn nav(ui: &Ui, rect: Rect, skin: &Skin) {
-    let Skin { look, palette, .. } = *skin;
+/// The panes, down the left, the one shown marked: a click shows another.
+/// Under them, where there are two, the key that switches.
+fn nav(ui: &mut Ui, rect: Rect, shown: SettingsPane, skin: &Skin, actions: &mut Vec<Action>) {
+    let Skin {
+        look,
+        palette,
+        locale,
+        ..
+    } = *skin;
     widgets::vline(ui, rect.right() + 0.5, rect.y_range(), palette.outline);
-    // The item is its line of text, with 3 above and below.
+    // An item is its line of text, with 3 above and below.
     let line = TextRole::OBody.row_height(ui.ctx(), look.faces);
-    let item = Rect::from_min_size(
-        pos2(rect.left(), rect.top() + 10.0),
-        vec2(rect.width(), line + 6.0),
-    );
-    mark(ui, item, palette);
-    widgets::paint_label(
-        ui,
-        item.left() + 14.0,
-        item.center().y,
-        Text::one(look, TextRole::OBody, &skin.say("General"), palette.text),
-    );
+    let listed = SettingsPane::listed();
+    for (index, pane) in listed.iter().copied().enumerate() {
+        let item = Rect::from_min_size(
+            pos2(rect.left(), rect.top() + 10.0 + index as f32 * (line + 6.0)),
+            vec2(rect.width(), line + 6.0),
+        );
+        let on = pane == shown;
+        let name = gettext(locale, pane.name());
+        let button = ButtonSpec::new(&name).hidden_at(ui, item);
+        button.widget_info(|| WidgetInfo::selected(WidgetType::Button, true, on, &name));
+        focus::hint(ui, &button, item, Ring::Inset { radius: 0 });
+        if button.clicked() && !on {
+            actions.push(Action::ShowSettingsPane(pane));
+        }
+        let color = if on {
+            mark(ui, item, palette);
+            palette.text
+        } else {
+            palette.dim
+        };
+        widgets::paint_label(
+            ui,
+            item.left() + 14.0,
+            item.center().y,
+            Text::one(look, TextRole::OBody, &skin.say(pane.name()), color),
+        );
+    }
+    if listed.len() > 1 {
+        let switch = key(ui, look, Command::SettingsPane);
+        let what = skin.say("pane");
+        let y = rect.bottom() - 10.0 - line / 2.0;
+        let hint = [(&switch, what.as_str(), true)];
+        widgets::key_hints(ui, (rect.left() + 14.0, y), &hint, 0.0, look, palette);
+    }
 }
 
-/// The options, one to a row, under the heading of their group.
-fn rows(ui: &mut Ui, settings: &Settings, cursor: usize, skin: &Skin, actions: &mut Vec<Action>) {
+/// A group's heading over its rows: 12 over it and 4 under, after `top`
+/// at the pane's top or between two groups. A whole number of points, so
+/// the rows' fills start on a pixel.
+fn heading(ui: &mut Ui, text: &'static str, top: f32, skin: &Skin) {
     let Skin { look, palette, .. } = *skin;
     let role = TextRole::OCaption;
-    // 8 at the top, then the heading with 12 over it and 4 under: a whole
-    // number of points, so the rows' fills start on a pixel.
     let line = role.row_height(ui.ctx(), look.faces);
-    let height = (8.0 + 12.0 + line + 4.0).ceil();
+    let height = (top + 12.0 + line + 4.0).ceil();
     let (place, _) = ui.allocate_exact_size(vec2(ui.available_width(), height), Sense::hover());
     // Dim, which `widgets::section_label` is not in this look.
     widgets::paint_label(
         ui,
         place.left() + ROW_SIDE,
-        place.top() + 20.0 + line / 2.0,
-        Text::one(look, role, &skin.say("Data"), palette.dim),
+        place.top() + top + 12.0 + line / 2.0,
+        Text::one(look, role, &skin.say(text), palette.dim),
     );
+}
+
+/// The General pane's options, one to a row, under the heading of their
+/// group.
+fn rows(ui: &mut Ui, settings: &Settings, cursor: usize, skin: &Skin, actions: &mut Vec<Action>) {
+    heading(ui, "Data", 8.0, skin);
     let room = ui.available_width() - 2.0 * ROW_SIDE;
-    let columns = columns(ui, room, settings, skin);
-    for (index, option) in OptionId::ALL.into_iter().enumerate() {
+    let columns = columns(
+        ui,
+        room,
+        widths(ui, &OptionId::GENERAL, settings, skin),
+        skin,
+    );
+    for (index, option) in OptionId::GENERAL.into_iter().enumerate() {
         option_row(
             ui,
             (index, option),
@@ -266,37 +322,326 @@ fn rows(ui: &mut Ui, settings: &Settings, cursor: usize, skin: &Skin, actions: &
     }
 }
 
-/// The widths of the label's column and the value's in rows with `room`
-/// between their sides: `LABEL` and `VALUE` where the widest hint has room
-/// after them. Rows with less (the file is beside them) take from the space
-/// each column has past its widest words, from both alike, before a hint
-/// is left out.
-fn columns(ui: &Ui, room: f32, settings: &Settings, skin: &Skin) -> (f32, f32) {
+/// The Agents pane: the server's switch and its status, how to connect an
+/// agent, where access is set, and the log. Its rows are
+/// [`SettingsPane::rows`]'s, in order; the command, the line for other
+/// agents and the note under them are no rows of the cursor's.
+fn agent_rows(
+    ui: &mut Ui,
+    app: &App,
+    settings: &Settings,
+    cursor: usize,
+    skin: &Skin,
+    actions: &mut Vec<Action>,
+) {
+    let Skin {
+        look,
+        palette,
+        locale,
+        ..
+    } = *skin;
+    let role = TextRole::OBody;
+    let room = ui.available_width() - 2.0 * ROW_SIDE;
+    let other = crate::endpoint::agent_setup_ready(crate::endpoint::Agent::Other).unwrap_or("…");
+    // The rows that are no options', each its label, value and hint: made
+    // again for each use, as a text is laid out once. The command for other
+    // agents gives way in its middle to what `fits`, and is copied whole.
+    let text_of = |ui: &Ui, which: usize, fits: f32| -> (&'static str, Text, Text) {
+        match which {
+            0 => {
+                let (value, hint) = status_text(app.agents_on(), app.agents.sessions.len(), skin);
+                ("Status", value, hint)
+            }
+            1 => {
+                let (value, hint) = access_text(ui, skin);
+                ("Set for each connection", value, hint)
+            }
+            _ => {
+                let ctx = ui.ctx().clone();
+                let shown = crate::ui::grid::ellipsize(other, fits, true, |text| {
+                    role.width(&ctx, look.faces, text)
+                });
+                (
+                    "Cursor, others",
+                    Text::one(look, role, &shown, palette.text),
+                    dim(skin, "same server, stdio"),
+                )
+            }
+        }
+    };
+    // Their labels and values with the options': all of the pane's columns
+    // line up. The command is as long as the path it holds, and lines up
+    // with nothing.
+    let mut widest = widths(ui, &OptionId::AGENTS, settings, skin);
+    for which in 0..2 {
+        let (name, value, hint) = text_of(ui, which, f32::INFINITY);
+        let label = role.width(ui.ctx(), look.faces, &format!("▌{}", skin.say(name)));
+        widest.0 = widest.0.max(label);
+        widest.1 = widest.1.max(value.layout(ui.ctx()).width());
+        widest.2 = widest.2.max(hint.layout(ui.ctx()).width());
+    }
+    // The status as it may come to read, not only as it does: the columns
+    // stay where they are when the server is turned on or off, or agents
+    // come and go.
+    for (on, count) in [(false, 0), (true, 0), (true, 999)] {
+        let (value, hint) = status_text(on, count, skin);
+        widest.1 = widest.1.max(value.layout(ui.ctx()).width());
+        widest.2 = widest.2.max(hint.layout(ui.ctx()).width());
+    }
+    let columns = columns(ui, room, widest, skin);
+    let on = |index: usize| index == cursor;
+    let fits = (room - columns.0).max(0.0);
+    let text = |ui: &mut Ui, index: usize, which: usize, actions: &mut Vec<Action>| {
+        let (name, value, hint) = text_of(ui, which, fits);
+        let on_cursor = index != usize::MAX && on(index);
+        text_row(
+            ui,
+            (index, name),
+            on_cursor,
+            columns,
+            (value, hint),
+            skin,
+            actions,
+        );
+    };
+    heading(ui, "MCP server", 8.0, skin);
+    option_row(
+        ui,
+        (0, OptionId::AgentsOn),
+        on(0),
+        columns,
+        settings,
+        skin,
+        actions,
+    );
+    text(ui, 1, 0, actions);
+
+    heading(ui, "Connect an agent", 12.0, skin);
+    command_box(ui, skin, locale);
+    text(ui, usize::MAX, 2, actions);
+
+    heading(ui, "Access", 12.0, skin);
+    text(ui, 2, 1, actions);
+
+    heading(ui, "Log", 12.0, skin);
+    option_row(
+        ui,
+        (3, OptionId::LogDays),
+        on(3),
+        columns,
+        settings,
+        skin,
+        actions,
+    );
+    option_row(
+        ui,
+        (4, OptionId::AgentsNotify),
+        on(4),
+        columns,
+        settings,
+        skin,
+        actions,
+    );
+
+    ui.add_space(14.0);
+    note(ui, skin);
+}
+
+/// A dim hint.
+fn dim(skin: &Skin, text: &'static str) -> Text {
+    Text::one(
+        skin.look,
+        TextRole::OBody,
+        &skin.say(text),
+        skin.palette.dim,
+    )
+}
+
+/// The status row's value and hint: whether the server runs, with how many
+/// agents are connected, and how they reach it.
+fn status_text(on: bool, count: usize, skin: &Skin) -> (Text, Text) {
+    let Skin {
+        look,
+        palette,
+        locale,
+        ..
+    } = *skin;
+    let role = TextRole::OBody;
+    if !on {
+        let value = Text::one(look, role, &format!("● {}", skin.say("off")), palette.dim);
+        return (value, dim(skin, "agents can't connect"));
+    }
+    let agents = match count {
+        0 => skin.say("no agents"),
+        count => look.label(
+            &ngettext(
+                locale,
+                "{n} agent",
+                "{n} agents",
+                u32::try_from(count).unwrap_or(u32::MAX),
+            )
+            .replace("{n}", &count.to_string()),
+        ),
+    };
+    let value = Text::new(look)
+        .add(role, &format!("● {}", skin.say("running")), palette.success)
+        .add(role, &format!(" · {agents}"), palette.text);
+    (value, dim(skin, "stdio · this computer"))
+}
+
+/// The access row's value and hint: where access is set, and the key
+/// that sets it on a row of Connections.
+fn access_text(ui: &Ui, skin: &Skin) -> (Text, Text) {
+    let Skin { look, palette, .. } = *skin;
+    let role = TextRole::OBody;
+    let change = key(ui, look, Command::ChangeAgentAccess);
+    let value = Text::one(look, role, &skin.say("in connections"), palette.text);
+    let hint = Text::new(look).add(role, &change, palette.accent).add(
+        role,
+        &format!(" {}", skin.say("on a row")),
+        palette.dim,
+    );
+    (value, hint)
+}
+
+/// The command that connects Claude Code, boxed, with the key that copies
+/// it at the box's right.
+fn command_box(ui: &mut Ui, skin: &Skin, locale: Locale) {
+    let Skin { look, palette, .. } = *skin;
+    let role = TextRole::OBody;
+    let line = role.row_height(ui.ctx(), look.faces);
+    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), line + 22.0), Sense::hover());
+    let boxed = Rect::from_min_max(
+        pos2(rect.left() + ROW_SIDE, rect.top() + 4.0),
+        pos2(
+            (rect.right() - ROW_SIDE).max(rect.left() + ROW_SIDE),
+            rect.bottom() - 2.0,
+        ),
+    );
+    ui.painter()
+        .rect_filled(boxed, CornerRadius::ZERO, palette.panel);
+    ui.painter().rect_stroke(
+        boxed,
+        CornerRadius::ZERO,
+        Stroke::new(1.0, palette.outline),
+        StrokeKind::Inside,
+    );
+    let copy = key(ui, look, Command::SettingsCopyCommand);
+    let hint = Text::new(look)
+        .add(role, &copy, palette.accent)
+        .space(role, " ")
+        .add(role, &skin.say("copy"), palette.dim)
+        .layout(ui.ctx());
+    let right = boxed.right() - 12.0;
+    hint.paint_right(ui.painter(), right, boxed.center().y);
+    // The command gives way in its middle to a pane too narrow for it, and
+    // is copied whole.
+    let command = crate::endpoint::agent_command_ready().unwrap_or_else(|| {
+        ui.ctx()
+            .request_repaint_after(std::time::Duration::from_millis(100));
+        "…"
+    });
+    let fits = (right - hint.width() - 16.0 - boxed.left() - 12.0).max(0.0);
+    let ctx = ui.ctx().clone();
+    let shown = crate::ui::grid::ellipsize(command, fits, true, |text| {
+        role.width(&ctx, look.faces, text)
+    });
+    widgets::paint_text(
+        ui,
+        boxed.left() + 12.0,
+        boxed.center().y,
+        Text::one(look, role, &shown, palette.text),
+    );
+    widgets::announce(ui, boxed, command);
+    let name = gettext(locale, "Copy");
+    if ButtonSpec::new(&name)
+        .hidden_at(ui, boxed.with_min_x(right - hint.width()))
+        .clicked()
+    {
+        crate::ui::agents::copy_command(ui.ctx(), locale);
+    }
+}
+
+/// What keeps agents within bounds, in a band with the accent at its left.
+fn note(ui: &mut Ui, skin: &Skin) {
+    let Skin { look, palette, .. } = *skin;
+    let role = TextRole::OBody;
+    let lines = [
+        "reads run in a read-only transaction with each connection's limits",
+        "no ddl, no multiple statements, no copy, no secrets sent",
+    ];
+    let line = role.row_height(ui.ctx(), look.faces);
+    let height = (lines.len() as f32 * line * 1.2 + 20.0).ceil();
+    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), height), Sense::hover());
+    let band = Rect::from_min_max(
+        pos2(rect.left() + ROW_SIDE, rect.top()),
+        pos2(
+            (rect.right() - ROW_SIDE).max(rect.left() + ROW_SIDE),
+            rect.bottom(),
+        ),
+    );
+    ui.painter()
+        .rect_filled(band, CornerRadius::ZERO, palette.panel);
+    let bar = Rect::from_min_size(band.min, vec2(2.0, band.height()));
+    ui.painter()
+        .rect_filled(bar, CornerRadius::ZERO, palette.accent);
+    let width = (band.width() - 24.0).max(0.0);
+    let mut y = band.top() + 10.0;
+    for said in lines {
+        let text = Text::one(look, role, &skin.say(said), palette.dim)
+            .wrap(width)
+            .layout(ui.ctx());
+        text.paint(ui.painter(), pos2(band.left() + 12.0, y));
+        widgets::announce(
+            ui,
+            Rect::from_min_size(pos2(band.left() + 12.0, y), text.size()),
+            text.galley.text(),
+        );
+        y += line * 1.2;
+    }
+}
+
+/// The widest label, value and hint `options` have: the hints of every
+/// value, not only the ones shown, so that the columns stay where they are
+/// when a value changes.
+fn widths(ui: &Ui, options: &[OptionId], settings: &Settings, skin: &Skin) -> (f32, f32, f32) {
     let role = TextRole::OBody;
     let width = |text: &str| role.width(ui.ctx(), skin.look.faces, text);
-    // Two cells stand between a column's widest words and the next column.
-    let gap = width("  ");
-    let labels = OptionId::ALL
-        .into_iter()
-        .map(|option| width(&format!("▌{}", skin.say(label(option)))))
+    let labels = options
+        .iter()
+        .map(|option| width(&format!("▌{}", skin.say(label(*option)))))
         .fold(0.0, f32::max);
-    let values = OptionId::ALL
-        .into_iter()
-        .map(|option| value_width(ui, option, settings, skin))
+    let values = options
+        .iter()
+        .map(|option| value_width(ui, *option, settings, skin))
         .fold(0.0, f32::max);
-    // The widest hint any value has, not only the ones shown: the columns
-    // stay where they are when a value changes.
-    let hints = OptionId::ALL
-        .into_iter()
-        .flat_map(|option| choices(option, settings))
+    let hints = options
+        .iter()
+        .flat_map(|option| choices(*option, settings))
         .map(|(_, value)| {
             let mut changed = settings.clone();
             value.set(&mut changed);
-            hint(value.option(), &changed, skin)
+            hint(ui, value.option(), &changed, skin)
                 .layout(ui.ctx())
                 .width()
         })
         .fold(0.0, f32::max);
+    (labels, values, hints)
+}
+
+/// The widths of the label's column and the value's in rows with `room`
+/// between their sides, from the widest label, value and hint they have:
+/// `LABEL` and `VALUE` where the widest hint has room after them. Rows with
+/// less (the file is beside them) take from the space each column has past
+/// its widest words, from both alike, before a hint is left out.
+fn columns(
+    ui: &Ui,
+    room: f32,
+    (labels, values, hints): (f32, f32, f32),
+    skin: &Skin,
+) -> (f32, f32) {
+    // Two cells stand between a column's widest words and the next column.
+    let gap = TextRole::OBody.width(ui.ctx(), skin.look.faces, "  ");
     let spare = [LABEL - labels - gap, VALUE - values - gap].map(|spare| spare.max(0.0));
     // The shortfall in whole points: a hint that fits by the last fraction
     // of one is not left out for a rounding.
@@ -317,14 +662,27 @@ fn value_width(ui: &Ui, option: OptionId, settings: &Settings, skin: &Skin) -> f
     let width = |text: &str| role.width(ui.ctx(), skin.look.faces, text);
     match option {
         // Between its chevrons, as the cursor's row has it.
-        OptionId::PageSize => width(&format!("‹ {} ›", settings.page_size)),
+        OptionId::PageSize | OptionId::LogDays => {
+            width(&format!("‹ {} ›", stepped_text(option, settings, skin)))
+        }
         OptionId::Timestamps | OptionId::GroupDigits => {
             let words = choices(option, settings);
             let between = width(" ") * words.len().saturating_sub(1) as f32;
             let segments = words.into_iter().map(|(word, _)| segment(ui, word, skin));
             segments.sum::<f32>() + between
         }
-        OptionId::ValueTags => width("[x]"),
+        OptionId::ValueTags | OptionId::AgentsOn | OptionId::AgentsNotify => width("[x]"),
+    }
+}
+
+/// A stepped option's value as its row writes it: the page size, the log's
+/// days.
+fn stepped_text(option: OptionId, settings: &Settings, skin: &Skin) -> String {
+    match option {
+        OptionId::LogDays => skin
+            .look
+            .label(&super::days(skin.locale, settings.mcp_log_days)),
+        _ => settings.page_size.to_string(),
     }
 }
 
@@ -334,6 +692,82 @@ fn segment(ui: &Ui, word: Said, skin: &Skin) -> f32 {
     TextRole::OBody.width(ui.ctx(), skin.look.faces, &text) + 2.0 * SEGMENT_PAD
 }
 
+/// A row's frame: its button, which puts the cursor on it (none for a row
+/// at `usize::MAX`, which the cursor is never on), the cursor's mark, and
+/// its label. Returns the row, and where its words stand between.
+fn row_frame(
+    ui: &mut Ui,
+    (index, name): (usize, &'static str),
+    on_cursor: bool,
+    skin: &Skin,
+    actions: &mut Vec<Action>,
+) -> (Rect, f32, f32, Option<egui::Response>) {
+    let Skin {
+        look,
+        palette,
+        locale,
+        ..
+    } = *skin;
+    let role = TextRole::OBody;
+    let (row, _) = ui.allocate_exact_size(vec2(ui.available_width(), ROW), Sense::hover());
+    let button = (index != usize::MAX).then(|| {
+        let said = gettext(locale, name);
+        let button = ButtonSpec::new(&said).hidden_at(ui, row);
+        // A row of a list: a ring outside it would be cut where the list
+        // ends.
+        focus::hint(ui, &button, row, Ring::Inset { radius: 0 });
+        if button.clicked() {
+            actions.push(Action::SelectSettingsRow(index));
+        }
+        button
+    });
+    if on_cursor {
+        mark(ui, row, palette);
+    }
+    let left = row.left() + ROW_SIDE;
+    let right = (row.right() - ROW_SIDE).max(left);
+    // The cursor's mark stands where a label starts, so the label under
+    // the cursor is a cell further in than the others.
+    let text = skin.say(name);
+    let text = if on_cursor {
+        Text::one(look, role, &format!("▌{text}"), palette.accent)
+    } else {
+        Text::one(look, role, &text, palette.text)
+    };
+    widgets::paint_text(ui, left, row.center().y, text);
+    (row, left, right, button)
+}
+
+/// A row that is no option: its label, what it says, and a hint, whole or
+/// left out.
+fn text_row(
+    ui: &mut Ui,
+    (index, name): (usize, &'static str),
+    on_cursor: bool,
+    (label_width, value_width): (f32, f32),
+    (value, hint): (Text, Text),
+    skin: &Skin,
+    actions: &mut Vec<Action>,
+) {
+    let (row, left, right, _) = row_frame(ui, (index, name), on_cursor, skin, actions);
+    let y = row.center().y;
+    let label_width = label_width.min(right - left);
+    let value_width = value_width.min(right - left - label_width);
+    let value = value.layout(ui.ctx());
+    let at = left + label_width;
+    value.paint_left(ui.painter(), at, y);
+    widgets::announce(
+        ui,
+        Rect::from_min_size(pos2(at, row.top()), vec2(value.width(), ROW)),
+        value.galley.text(),
+    );
+    let hint = hint.layout(ui.ctx());
+    let hint_left = at + value_width.max(value.width());
+    if hint_left + hint.width() <= right {
+        hint.paint_left(ui.painter(), hint_left, y);
+    }
+}
+
 /// One option's row: its label, its value where a click sets it, and the
 /// hint that shows what the value does.
 fn option_row(
@@ -345,45 +779,24 @@ fn option_row(
     skin: &Skin,
     actions: &mut Vec<Action>,
 ) {
-    let Skin {
-        look,
-        palette,
-        locale,
-    } = *skin;
-    let role = TextRole::OBody;
-    let (row, _) = ui.allocate_exact_size(vec2(ui.available_width(), ROW), Sense::hover());
+    let locale = skin.locale;
     // The row's button comes before its values', so that a click on a
     // value is the value's.
-    let name = gettext(locale, label(option));
-    let button = ButtonSpec::new(&name).hidden_at(ui, row);
-    // The page size has no button that is its value: its row says it.
-    if option == OptionId::PageSize {
+    let (row, left, right, button) =
+        row_frame(ui, (index, label(option)), on_cursor, skin, actions);
+    let y = row.center().y;
+    // A stepped option has no button that is its value: its row says it.
+    if let Some(button) = button
+        && matches!(option, OptionId::PageSize | OptionId::LogDays)
+    {
+        let name = gettext(locale, label(option));
+        let value = stepped_text(option, settings, skin);
         button.widget_info(|| {
             let mut info = WidgetInfo::labeled(WidgetType::Button, true, &name);
-            info.current_text_value = Some(settings.page_size.to_string());
+            info.current_text_value = Some(value.clone());
             info
         });
     }
-    // A row of a list: a ring outside it would be cut where the list ends.
-    focus::hint(ui, &button, row, Ring::Inset { radius: 0 });
-    if button.clicked() {
-        actions.push(Action::SelectSettingsRow(index));
-    }
-    if on_cursor {
-        mark(ui, row, palette);
-    }
-    let y = row.center().y;
-    let left = row.left() + ROW_SIDE;
-    let right = (row.right() - ROW_SIDE).max(left);
-    // The cursor's mark stands where a label starts, so the label under
-    // the cursor is a cell further in than the others.
-    let text = skin.say(label(option));
-    let text = if on_cursor {
-        Text::one(look, role, &format!("▌{text}"), palette.accent)
-    } else {
-        Text::one(look, role, &text, palette.text)
-    };
-    widgets::paint_text(ui, left, y, text);
     // A window too narrow for the columns takes from the hint first, then
     // from the value's column.
     let label_width = label_width.min(right - left);
@@ -395,20 +808,294 @@ fn option_row(
         on_cursor,
     };
     match option {
-        OptionId::PageSize => page_size(ui, cell, settings, skin, actions),
+        OptionId::PageSize | OptionId::LogDays => {
+            stepper(ui, cell, option, settings, skin, actions)
+        }
         OptionId::Timestamps | OptionId::GroupDigits => {
             segments(ui, cell, option, settings, skin, actions);
         }
-        OptionId::ValueTags => check(ui, cell, settings, skin, actions),
+        OptionId::ValueTags | OptionId::AgentsOn | OptionId::AgentsNotify => {
+            check(ui, cell, option, settings, skin, actions);
+        }
     }
     // The hint is whole or left out: it tells nothing cut short.
-    let hint = hint(option, settings, skin).layout(ui.ctx());
+    let hint = hint(ui, option, settings, skin).layout(ui.ctx());
     let hint_left = cell.left + value_width;
     if hint_left + hint.width() <= right {
         hint.paint_left(ui.painter(), hint_left, y);
     }
 }
 
+/// A stepped option, the page size or the log's days: its value as it is,
+/// and on the cursor's row between the chevrons that step it, each dim
+/// where there is no value on its side.
+fn stepper(
+    ui: &mut Ui,
+    cell: Cell,
+    option: OptionId,
+    settings: &Settings,
+    skin: &Skin,
+    actions: &mut Vec<Action>,
+) {
+    let Skin { look, palette, .. } = *skin;
+    let role = TextRole::OBody;
+    let y = cell.row.center().y;
+    let value = stepped_text(option, settings, skin);
+    let current = settings.value(option);
+    let [fewer, more] = choices(option, settings)[..] else {
+        return;
+    };
+    let text = if cell.on_cursor {
+        let chevron = |(_, step): (Said, OptionValue)| {
+            if step == current {
+                palette.dim
+            } else {
+                palette.accent
+            }
+        };
+        Text::new(look)
+            .add(role, "‹", chevron(fewer))
+            .space(role, " ")
+            .add(role, &value, palette.text)
+            .space(role, " ")
+            .add(role, "›", chevron(more))
+    } else {
+        Text::one(look, role, &value, palette.text)
+    };
+    widgets::paint_text(ui, cell.left, y, text);
+    // The chevrons' buttons are on every row. On the cursor's row each is
+    // over its chevron, with half the space beside it. On the others, where
+    // no chevron is drawn, each keeps a point at the column's corner: a
+    // click on the value there only moves the cursor, and Tab does not
+    // stop on a point.
+    let places = if cell.on_cursor {
+        let width = |text: &str| role.width(ui.ctx(), look.faces, text);
+        let lead = width("‹ ");
+        let chevron = width("‹");
+        let reach = (lead - chevron) / 2.0;
+        let after = cell.left + lead + width(&value) + (lead - chevron);
+        [cell.left, after].map(|left| {
+            Rect::from_min_max(
+                pos2(left - reach, cell.row.top()),
+                pos2(left + chevron + reach, cell.row.bottom()),
+            )
+        })
+    } else {
+        [0.0, 1.0].map(|along| {
+            Rect::from_min_size(pos2(cell.left + along, cell.row.top()), vec2(1.0, 1.0))
+        })
+    };
+    let press = Press::Step {
+        drawn: cell.on_cursor,
+    };
+    for (place, step) in places.into_iter().zip([fewer, more]) {
+        choice(ui, place, cell.index, step, press, skin, actions);
+    }
+}
+
+/// An option that is on or off, as the look's check.
+fn check(
+    ui: &mut Ui,
+    cell: Cell,
+    option: OptionId,
+    settings: &Settings,
+    skin: &Skin,
+    actions: &mut Vec<Action>,
+) {
+    let Skin { look, palette, .. } = *skin;
+    let current = settings.value(option);
+    let (glyph, color) = if super::is_on(current) {
+        ("[x]", palette.success)
+    } else {
+        ("[ ]", palette.dim)
+    };
+    let text = Text::one(look, TextRole::OBody, glyph, color);
+    let width = widgets::paint_text(ui, cell.left, cell.row.center().y, text);
+    let place = Rect::from_min_max(
+        pos2(cell.left, cell.row.top()),
+        pos2(cell.left + width, cell.row.bottom()),
+    );
+    // One mark stands for both values, and a click on it flips it: the
+    // value it is not set to is the button on top.
+    let mut values = choices(option, settings);
+    values.sort_by_key(|(_, value)| *value != current);
+    for (word, value) in values {
+        let press = Press::Value {
+            set: value == current,
+        };
+        choice(ui, place, cell.index, (word, value), press, skin, actions);
+    }
+}
+
+/// What the row says beside `option`'s value: where it applies, or a
+/// sample of what it does.
+fn hint(ui: &Ui, option: OptionId, settings: &Settings, skin: &Skin) -> Text {
+    let Skin { look, palette, .. } = *skin;
+    let role = TextRole::OBody;
+    let dim = |text: &str| Text::one(look, role, text, palette.dim);
+    match option {
+        OptionId::PageSize => dim(&skin.say("table view")),
+        OptionId::Timestamps => dim(sample_timestamp(settings.timestamps)),
+        OptionId::GroupDigits => dim(&skin.say(if settings.group_digits {
+            "grouping on"
+        } else {
+            "grouping off"
+        })),
+        // Two tags as a grid shows them: in the first two slots' colours,
+        // or as plain values. They are values of a column, not words:
+        // no language has others.
+        OptionId::ValueTags => {
+            let slots = terminal_slots(palette);
+            let [first, second] = if settings.value_tags {
+                [slots[0], slots[1]]
+            } else {
+                [palette.dim; 2]
+            };
+            Text::new(look)
+                .add(role, "print", first)
+                .space(role, "  ")
+                .add(role, "ebook", second)
+        }
+        OptionId::AgentsOn => dim(&skin.say("while tabletist is open")),
+        // The key that shows the log, from this row, where there is a
+        // window to show it in.
+        OptionId::LogDays if !skin.window => dim(&skin.say("on this computer")),
+        OptionId::LogDays => Text::new(look)
+            .add(
+                role,
+                &key(ui, look, Command::SettingsOpenRow),
+                palette.accent,
+            )
+            .add(role, &format!(" {}", skin.say("shows it")), palette.dim),
+        OptionId::AgentsNotify => dim(&skin.say("a desktop notification")),
+    }
+}
+
+/// The button a hint of the footer stands for: its name, and what a
+/// press does, where it does something.
+type Button<'a> = (&'a str, Option<Vec<Action>>);
+
+/// The screen's keys at the left. A hint whose key does one thing is that
+/// thing's button too. On the Agents pane: Enter, on a row that opens
+/// something (`opens`), and the key that copies the command.
+fn footer(
+    ui: &mut Ui,
+    pane: SettingsPane,
+    option: Option<OptionId>,
+    opens: Option<(SettingsRow, Vec<Action>)>,
+    settings: &Settings,
+    skin: &Skin,
+    actions: &mut Vec<Action>,
+) {
+    let Skin {
+        look,
+        palette,
+        locale,
+        ..
+    } = *skin;
+    let (rect, _) =
+        ui.allocate_exact_size(vec2(ui.available_width(), FOOTER + 1.0), Sense::hover());
+    ui.painter()
+        .rect_filled(rect, CornerRadius::ZERO, palette.panel);
+    widgets::hline(ui, rect.x_range(), rect.top() + 0.5, palette.outline);
+    let y = rect.top() + 1.0 + FOOTER / 2.0;
+    // What the keys would do on the cursor's row. Space has nothing to
+    // flip on an option of more than two values.
+    let flip = option
+        .and_then(|option| settings.flipped(option))
+        .map(Action::SetOption);
+    let reset = option.map(|option| Action::SetOption(option.default_value()));
+    let edit = Some(Action::EditSettingsFile);
+    // `$EDITOR` is a name: the look does not lower-case it.
+    let in_editor = skin
+        .say("open file in {editor}")
+        .replace("{editor}", "$EDITOR");
+    // The key, what it does, the button it stands for, and what that does.
+    let mut keys: Vec<(Command, String, Option<Button<'_>>)> = vec![
+        (Command::SettingsMove, skin.say("move"), None),
+        (Command::SettingsChange, skin.say("change"), None),
+        (
+            Command::SettingsToggle,
+            skin.say("toggle"),
+            Some(("Toggle", flip.map(|flip| vec![flip]))),
+        ),
+    ];
+    if pane == SettingsPane::Agents {
+        if let Some((row, opens)) = opens {
+            let (what, name) = match row {
+                SettingsRow::AgentsAccess => ("open connections", "Open Connections"),
+                SettingsRow::Option(OptionId::LogDays) => ("show log", "Show log"),
+                _ => ("show agents tab", "Show Agents tab"),
+            };
+            keys.push((
+                Command::SettingsOpenRow,
+                skin.say(what),
+                Some((name, Some(opens))),
+            ));
+        }
+        keys.push((
+            Command::SettingsCopyCommand,
+            skin.say("copy command"),
+            Some(("Copy the command", None)),
+        ));
+    }
+    keys.extend([
+        (
+            Command::SettingsOpenFile,
+            in_editor,
+            Some(("Open file in editor", edit.map(|edit| vec![edit]))),
+        ),
+        (
+            Command::SettingsReset,
+            skin.say("reset option"),
+            Some(("Reset option", reset.map(|reset| vec![reset]))),
+        ),
+        (
+            Command::SettingsClose,
+            skin.say("close"),
+            Some(("Close", Some(vec![Action::CloseDialog]))),
+        ),
+    ]);
+    // The hints give way from the end where the window has no room for
+    // them, all but the last, which closes: their keys still work.
+    let width_of = |ui: &Ui, (command, what): (Command, &str)| {
+        let key = key(ui, look, command);
+        widgets::key_hints_width(ui, &[(&key, what, true)], 0.0, look, palette)
+    };
+    let room = rect.width() - 24.0;
+    let mut widths: Vec<f32> = keys
+        .iter()
+        .map(|(command, what, _)| width_of(ui, (*command, what)))
+        .collect();
+    while keys.len() > 1 && widths.iter().sum::<f32>() + 16.0 * (keys.len() - 1) as f32 > room {
+        let before_last = keys.len() - 2;
+        keys.remove(before_last);
+        widths.remove(before_last);
+    }
+    let mut left = rect.left() + 12.0;
+    for (command, what, button) in keys {
+        let key = key(ui, look, command);
+        let hint = [(&key, what.as_str(), true)];
+        let width = widgets::key_hints(ui, (left, y), &hint, 0.0, look, palette);
+        // In from the window's edge by a focus ring's reach, or the ring
+        // would be cut there.
+        let place = Rect::from_min_max(
+            pos2(left, rect.top() + 5.0),
+            pos2(left + width, rect.bottom() - 4.0),
+        );
+        left += width + 16.0;
+        if let Some((name, action)) = button {
+            let name = gettext(locale, name);
+            if ButtonSpec::new(&name).hidden_at(ui, place).clicked() {
+                if command == Command::SettingsCopyCommand {
+                    crate::ui::agents::copy_command(ui.ctx(), locale);
+                }
+                actions.extend(action.into_iter().flatten());
+            }
+        }
+    }
+}
+
 /// What a choice's button is to the keyboard and to a screen reader.
 #[derive(Clone, Copy)]
 enum Press {
@@ -450,65 +1137,6 @@ fn choice(
     }
 }
 
-/// The page size: the number as it is, and on the cursor's row between the
-/// chevrons that step it, each dim where there is no size on its side.
-fn page_size(ui: &mut Ui, cell: Cell, settings: &Settings, skin: &Skin, actions: &mut Vec<Action>) {
-    let Skin { look, palette, .. } = *skin;
-    let role = TextRole::OBody;
-    let y = cell.row.center().y;
-    let size = settings.page_size.to_string();
-    let current = settings.value(OptionId::PageSize);
-    let [fewer, more] = choices(OptionId::PageSize, settings)[..] else {
-        return;
-    };
-    let text = if cell.on_cursor {
-        let chevron = |(_, step): (Said, OptionValue)| {
-            if step == current {
-                palette.dim
-            } else {
-                palette.accent
-            }
-        };
-        Text::new(look)
-            .add(role, "‹", chevron(fewer))
-            .space(role, " ")
-            .add(role, &size, palette.text)
-            .space(role, " ")
-            .add(role, "›", chevron(more))
-    } else {
-        Text::one(look, role, &size, palette.text)
-    };
-    widgets::paint_text(ui, cell.left, y, text);
-    // The chevrons' buttons are on every row. On the cursor's row each is
-    // over its chevron, with half the space beside it. On the others, where
-    // no chevron is drawn, each keeps a point at the column's corner: a
-    // click on the number there only moves the cursor, and Tab does not
-    // stop on a point.
-    let places = if cell.on_cursor {
-        let width = |text: &str| role.width(ui.ctx(), look.faces, text);
-        let lead = width("‹ ");
-        let chevron = width("‹");
-        let reach = (lead - chevron) / 2.0;
-        let after = cell.left + lead + width(&size) + (lead - chevron);
-        [cell.left, after].map(|left| {
-            Rect::from_min_max(
-                pos2(left - reach, cell.row.top()),
-                pos2(left + chevron + reach, cell.row.bottom()),
-            )
-        })
-    } else {
-        [0.0, 1.0].map(|along| {
-            Rect::from_min_size(pos2(cell.left + along, cell.row.top()), vec2(1.0, 1.0))
-        })
-    };
-    let press = Press::Step {
-        drawn: cell.on_cursor,
-    };
-    for (place, step) in places.into_iter().zip([fewer, more]) {
-        choice(ui, place, cell.index, step, press, skin, actions);
-    }
-}
-
 /// One of two values as segments, each as tall as its line of text and a
 /// cell from the next: the chosen one filled with the accent on the
 /// cursor's row and outlined on the others, the other one dim.
@@ -555,142 +1183,6 @@ fn segments(
     }
 }
 
-/// An option that is on or off, as the look's check.
-fn check(ui: &mut Ui, cell: Cell, settings: &Settings, skin: &Skin, actions: &mut Vec<Action>) {
-    let Skin { look, palette, .. } = *skin;
-    let current = settings.value(OptionId::ValueTags);
-    let (glyph, color) = if settings.value_tags {
-        ("[x]", palette.success)
-    } else {
-        ("[ ]", palette.dim)
-    };
-    let text = Text::one(look, TextRole::OBody, glyph, color);
-    let width = widgets::paint_text(ui, cell.left, cell.row.center().y, text);
-    let place = Rect::from_min_max(
-        pos2(cell.left, cell.row.top()),
-        pos2(cell.left + width, cell.row.bottom()),
-    );
-    // One mark stands for both values, and a click on it flips it: the
-    // value it is not set to is the button on top.
-    let mut values = choices(OptionId::ValueTags, settings);
-    values.sort_by_key(|(_, value)| *value != current);
-    for (word, value) in values {
-        let press = Press::Value {
-            set: value == current,
-        };
-        choice(ui, place, cell.index, (word, value), press, skin, actions);
-    }
-}
-
-/// What the row says beside `option`'s value: where it applies, or a
-/// sample of what it does.
-fn hint(option: OptionId, settings: &Settings, skin: &Skin) -> Text {
-    let Skin { look, palette, .. } = *skin;
-    let role = TextRole::OBody;
-    let dim = |text: &str| Text::one(look, role, text, palette.dim);
-    match option {
-        OptionId::PageSize => dim(&skin.say("table view")),
-        OptionId::Timestamps => dim(sample_timestamp(settings.timestamps)),
-        OptionId::GroupDigits => dim(&skin.say(if settings.group_digits {
-            "grouping on"
-        } else {
-            "grouping off"
-        })),
-        // Two tags as a grid shows them: in the first two slots' colours,
-        // or as plain values. They are values of a column, not words:
-        // no language has others.
-        OptionId::ValueTags => {
-            let slots = terminal_slots(palette);
-            let [first, second] = if settings.value_tags {
-                [slots[0], slots[1]]
-            } else {
-                [palette.dim; 2]
-            };
-            Text::new(look)
-                .add(role, "print", first)
-                .space(role, "  ")
-                .add(role, "ebook", second)
-        }
-    }
-}
-
-/// The screen's keys at the left. A hint whose key does one thing is that
-/// thing's button too.
-fn footer(
-    ui: &mut Ui,
-    option: Option<OptionId>,
-    settings: &Settings,
-    skin: &Skin,
-    actions: &mut Vec<Action>,
-) {
-    let Skin {
-        look,
-        palette,
-        locale,
-    } = *skin;
-    let (rect, _) =
-        ui.allocate_exact_size(vec2(ui.available_width(), FOOTER + 1.0), Sense::hover());
-    ui.painter()
-        .rect_filled(rect, CornerRadius::ZERO, palette.panel);
-    widgets::hline(ui, rect.x_range(), rect.top() + 0.5, palette.outline);
-    let y = rect.top() + 1.0 + FOOTER / 2.0;
-    // What the keys would do on the cursor's row. Space has nothing to
-    // flip on an option of more than two values.
-    let flip = option
-        .and_then(|option| settings.flipped(option))
-        .map(Action::SetOption);
-    let reset = option.map(|option| Action::SetOption(option.default_value()));
-    let edit = Some(Action::EditSettingsFile);
-    // `$EDITOR` is a name: the look does not lower-case it.
-    let in_editor = skin
-        .say("open file in {editor}")
-        .replace("{editor}", "$EDITOR");
-    // The key, what it does, the button it stands for, and what that does.
-    let keys = [
-        (Command::SettingsMove, skin.say("move"), None),
-        (Command::SettingsChange, skin.say("change"), None),
-        (
-            Command::SettingsToggle,
-            skin.say("toggle"),
-            Some(("Toggle", flip)),
-        ),
-        (
-            Command::SettingsOpenFile,
-            in_editor,
-            Some(("Open file in editor", edit)),
-        ),
-        (
-            Command::SettingsReset,
-            skin.say("reset option"),
-            Some(("Reset option", reset)),
-        ),
-        (
-            Command::SettingsClose,
-            skin.say("close"),
-            Some(("Close", Some(Action::CloseDialog))),
-        ),
-    ];
-    let mut left = rect.left() + 12.0;
-    for (command, what, button) in keys {
-        let key = key(ui, look, command);
-        let hint = [(&key, what.as_str(), true)];
-        let width = widgets::key_hints(ui, (left, y), &hint, 0.0, look, palette);
-        // In from the window's edge by a focus ring's reach, or the ring
-        // would be cut there.
-        let place = Rect::from_min_max(
-            pos2(left, rect.top() + 5.0),
-            pos2(left + width, rect.bottom() - 4.0),
-        );
-        left += width + 16.0;
-        if let Some((name, action)) = button {
-            let name = gettext(locale, name);
-            if ButtonSpec::new(&name).hidden_at(ui, place).clicked() {
-                actions.extend(action);
-            }
-        }
-    }
-}
-
 /// A key of the screen, as the keymap writes it for the look.
 fn key(ui: &Ui, look: &Look, command: Command) -> crate::keymap::Written {
     crate::ui::keys::written(ui.ctx(), look, command)
````

- [ ] **Step 4: Run the tests**

Run: the same, then `cargo test --locked --lib ui::`, `cargo test --locked --lib keymap::` and `cargo test --locked --test keys`

Expected: PASS.

```text
test result: ok. 66 passed; 0 failed; 0 ignored; 0 measured; 2466 filtered out
test result: ok. 1485 passed; 0 failed; 19 ignored; 0 measured; 1028 filtered out
test result: ok. 23 passed; 0 failed; 0 ignored; 0 measured; 2509 filtered out
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

- [ ] **Step 5: Look at it.** Render, by eye, in a throwaway scene in `src/shots.rs`: Settings › Agents in both looks, with the server on and an agent connected, and off; Claude Code and Cursor; at 1440 by 900, and narrow. Remove it before the commit. No picture is committed.

- [ ] **Step 6: Commit**

```bash
cargo fmt --all
cargo clippy --locked --workspace --all-targets -- -D warnings
git add src
git commit -m "Add Settings › Agents: the server's switch, how to connect an agent, the log"
```

---

### Task 3: Settings… and Other agents… in the Agents tab

- The server off: **Settings…** beside **Turn on**. Nobody here: **Other agents…** beside **Show log**. Both open Settings › Agents (`Action::ShowAgentSettings`).
- Omarchy: `s` (`Command::ShowAgentSettings`, in the Agents scope) in both states, in the Activity view, and its hint `s settings` on the bottom line.

**Files:**
- Modify: `src/ui/agents.rs`, `src/keymap.rs`, `src/ui/keys.rs`, `src/ui/workspace.rs`, `src/ui/agent_settings_tests.rs`

- [ ] **Step 1: Write the failing test**

````diff
diff --git a/src/ui/agent_settings_tests.rs b/src/ui/agent_settings_tests.rs
index 9b56c6d..2e00b64 100644
--- a/src/ui/agent_settings_tests.rs
+++ b/src/ui/agent_settings_tests.rs
@@ -315,6 +315,34 @@ fn settings_opens_connections_where_access_is_set() {
     }
 }
 
+#[test]
+fn the_agents_tab_opens_its_settings_where_the_server_is_off_and_where_nobody_is_here() {
+    for look in looks() {
+        let (mut harness, tab) = connected(look);
+        harness.app.apply(Action::ShowAgents(tab));
+        harness.settle();
+        // The server off: Settings…
+        if look.terminal {
+            super::tests::type_key(&mut harness, egui::Key::S, "s");
+        } else {
+            harness.click("Settings…");
+        }
+        harness.settle();
+        assert_eq!(pane(&harness), Some(SettingsPane::Agents), "{}", look.name);
+        harness.app.apply(Action::CloseDialog);
+        // Nobody here: Other agents…
+        harness.app.apply(Action::TurnAgentsOn);
+        harness.settle();
+        if look.terminal {
+            super::tests::type_key(&mut harness, egui::Key::S, "s");
+        } else {
+            harness.click("Other agents…");
+        }
+        harness.settle();
+        assert_eq!(pane(&harness), Some(SettingsPane::Agents), "{}", look.name);
+    }
+}
+
 #[test]
 fn an_enter_held_on_the_status_row_shows_the_agents_tab_and_goes_no_further() {
     let (mut harness, tab) = connected(Look::omarchy());
````

- [ ] **Step 2: Run it to see that it fails**

Run: `cargo test --locked --lib ui::agent_settings_tests::the_agents_tab_opens`

Expected: FAIL. The server-off state has no Settings… yet (the terminal look's `s` does nothing either):

```text
test ui::agent_settings_tests::the_agents_tab_opens_its_settings_where_the_server_is_off_and_where_nobody_is_here ... FAILED
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 2532 filtered out
```

- [ ] **Step 3: The buttons and the key**

````diff
diff --git a/src/keymap.rs b/src/keymap.rs
index 090edb2..4382523 100644
--- a/src/keymap.rs
+++ b/src/keymap.rs
@@ -343,6 +343,7 @@ commands! {
     ShowAgentsLog,
     ToggleAgentsLogScope,
     AllowAgents,
+    ShowAgentSettings,
     ChangeAgentAccess,
     MoveInAgentLevels,
     ShowAgentsFromBox,
@@ -598,6 +599,7 @@ impl Command {
             }
             C::ShowAgentsLog => agents(info("Show the log, or what agents do now", "", Window)),
             C::AllowAgents => agents(info("Allow agents on this connection…", "", Window)),
+            C::ShowAgentSettings => agents(info("Show the agents' settings", "", Window)),
             C::ChangeAgentAccess => agents(info(
                 "Change what agents may do on the connection",
                 "",
@@ -1019,6 +1021,7 @@ pub const BINDINGS: &[Binding] = {
         bind(C::ShowAgentsLog, AGENTS, NONE, keys(&[k("l")])),
         bind(C::ToggleAgentsLogScope, AGENTS, NONE, keys(&[k("L")])),
         bind(C::AllowAgents, AGENTS, NONE, keys(&[k("e")])),
+        bind(C::ShowAgentSettings, AGENTS, NONE, keys(&[k("s")])),
         bind(C::ChangeAgentAccess, CONNECTIONS, NONE, keys(&[k("a")])),
         bind(
             C::MoveInAgentLevels,
diff --git a/src/ui/agents.rs b/src/ui/agents.rs
index 59ec6d9..72c82e0 100644
--- a/src/ui/agents.rs
+++ b/src/ui/agents.rs
@@ -26,6 +26,13 @@ pub fn copy_command(ctx: &egui::Context, locale: crate::i18n::Locale) {
     }
 }
 
+/// What the Nobody here state's buttons ask for.
+enum NoneAsk {
+    /// Other agents…: Settings › Agents.
+    Settings,
+    Log,
+}
+
 /// A segment of the macOS look's Activity and Log switch.
 const SEGMENT: f32 = 64.0;
 
@@ -97,6 +104,9 @@ pub fn show(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId) {
         .keymap
         .key(layout, crate::keymap::Command::CopyAgentCommand);
     let on_key = app.keymap.key(layout, crate::keymap::Command::TurnAgentsOn);
+    let settings_key = app
+        .keymap
+        .key(layout, crate::keymap::Command::ShowAgentSettings);
     let stop_key = app
         .keymap
         .key(layout, crate::keymap::Command::StopAgentSession);
@@ -239,10 +249,16 @@ pub fn show(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId) {
                             "Turn it on to let agents like Claude Code use the connections you allow. It runs only while Tabletist is open, on this computer.",
                         ),
                     };
-                    let label = gettext(locale, "Turn on");
-                    let button = states::key_button(&label, &on_key, &look).primary();
-                    if states::empty(ui, rect, &notice, vec![button], &look, &palette).is_some() {
-                        actions.push(Action::TurnAgentsOn);
+                    let (settings_label, label) =
+                        (gettext(locale, "Settings…"), gettext(locale, "Turn on"));
+                    let buttons = vec![
+                        states::key_button(&settings_label, &settings_key, &look),
+                        states::key_button(&label, &on_key, &look).primary(),
+                    ];
+                    match states::empty(ui, rect, &notice, buttons, &look, &palette) {
+                        Some(0) => actions.push(Action::ShowAgentSettings),
+                        Some(_) => actions.push(Action::TurnAgentsOn),
+                        None => {}
                     }
                 }
                 AgentsHere::OffHere => {
@@ -286,22 +302,30 @@ pub fn show(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId) {
                         .frame(Frame::new().fill(palette.window))
                         .show(ui, |ui| {
                             let rect = ui.max_rect();
-                            let keys = (&copy_key, &log_key);
+                            let keys = (&copy_key, &log_key, &settings_key);
                             let said = (&connection[..], &name[..], &access[..]);
                             let style = (&look, &palette, locale);
-                            if none_connected(ui, rect, said, keys, style) {
-                                let view = AgentsView::Log;
-                                actions.push(Action::ShowAgentsView { tab, view });
+                            match none_connected(ui, rect, said, keys, style) {
+                                Some(NoneAsk::Log) => {
+                                    let view = AgentsView::Log;
+                                    actions.push(Action::ShowAgentsView { tab, view });
+                                }
+                                Some(NoneAsk::Settings) => actions.push(Action::ShowAgentSettings),
+                                None => {}
                             }
                         });
                 }
                 AgentsHere::NoneConnected => {
                     let name = app.workspace(tab).map(|w| w.name.clone()).unwrap_or_default();
-                    let keys = (&copy_key, &log_key);
+                    let keys = (&copy_key, &log_key, &settings_key);
                     let said = (&connection[..], &name[..], &access[..]);
-                    if none_connected(ui, rect, said, keys, (&look, &palette, locale)) {
-                        let view = AgentsView::Log;
-                        actions.push(Action::ShowAgentsView { tab, view });
+                    match none_connected(ui, rect, said, keys, (&look, &palette, locale)) {
+                        Some(NoneAsk::Log) => {
+                            let view = AgentsView::Log;
+                            actions.push(Action::ShowAgentsView { tab, view });
+                        }
+                        Some(NoneAsk::Settings) => actions.push(Action::ShowAgentSettings),
+                        None => {}
                     }
                 }
                 AgentsHere::Connected { .. } if look.terminal => {
@@ -381,16 +405,20 @@ fn status_bar(
 }
 
 /// Nobody here: how to connect an agent, the command with Copy beside it,
-/// and what agents may do on this connection. Centred under an icon tile,
-/// or in the terminal look from the top left without one, as an empty
-/// state is (`states::empty`).
+/// and what agents may do on this connection, with Other agents… and Show
+/// log under them. Centred under an icon tile, or in the terminal look
+/// from the top left without one, as an empty state is (`states::empty`).
 fn none_connected(
     ui: &mut egui::Ui,
     rect: Rect,
     (connection, name, access): (&str, &str, &str),
-    (copy_key, log_key): (&crate::keymap::Written, &crate::keymap::Written),
+    (copy_key, log_key, settings_key): (
+        &crate::keymap::Written,
+        &crate::keymap::Written,
+        &crate::keymap::Written,
+    ),
     (look, palette, locale): (&Look, &Palette, crate::i18n::Locale),
-) -> bool {
+) -> Option<NoneAsk> {
     let ctx = ui.ctx().clone();
     let terminal = look.terminal;
     let room = (rect.width() - 2.0 * states::INSET).max(0.0);
@@ -506,13 +534,27 @@ fn none_connected(
         Rect::from_min_size(pos2(facts_left, y), facts.size()),
         &format!("{on_this}: {access_word} {access}"),
     );
-    // Under them, the calls agents made here before.
+    // Under them, how to connect other agents, and the calls agents made
+    // here before.
     y += facts.height() + 18.0;
-    let label = gettext(locale, "Show log");
+    let (other_label, label) = (
+        gettext(locale, "Other agents…"),
+        gettext(locale, "Show log"),
+    );
+    let other = states::key_button(&other_label, settings_key, look);
     let show = states::key_button(&label, log_key, look);
-    let width = show.width(ui, look);
-    let at = Rect::from_min_size(pos2(left(width), y), vec2(width, height));
-    show.show_at(ui, at, look, palette).clicked()
+    let (other_width, width) = (other.width(ui, look), show.width(ui, look));
+    let both = other_width + 8.0 + width;
+    let at = Rect::from_min_size(pos2(left(both), y), vec2(other_width, height));
+    let mut asked = other
+        .show_at(ui, at, look, palette)
+        .clicked()
+        .then_some(NoneAsk::Settings);
+    let at = Rect::from_min_size(pos2(at.right() + 8.0, y), vec2(width, height));
+    if show.show_at(ui, at, look, palette).clicked() {
+        asked = Some(NoneAsk::Log);
+    }
+    asked
 }
 
 /// A session that works elsewhere, as "Also:" names it: its agent, and
diff --git a/src/ui/keys.rs b/src/ui/keys.rs
index afd068c..6af2dd1 100644
--- a/src/ui/keys.rs
+++ b/src/ui/keys.rs
@@ -2259,6 +2259,20 @@ fn letters(
             (Some(Command::TurnAgentsOn), _) if agents_shows(app, tab, true) => {
                 actions.push(Action::TurnAgentsOn);
             }
+            // Settings… with the server off, Other agents… with nobody
+            // here: both are Settings › Agents.
+            (Some(Command::ShowAgentSettings), _)
+                if agents_view(app, tab) == Some(crate::model::AgentsView::Activity)
+                    && matches!(
+                        app.agents_here(tab),
+                        Some(
+                            crate::model::AgentsHere::ServerOff
+                                | crate::model::AgentsHere::NoneConnected
+                        )
+                    ) =>
+            {
+                actions.push(Action::ShowAgentSettings);
+            }
             (Some(Command::AllowAgents), _)
                 if agents_view(app, tab) == Some(crate::model::AgentsView::Activity)
                     && app.agents_here(tab) == Some(crate::model::AgentsHere::OffHere) =>
diff --git a/src/ui/workspace.rs b/src/ui/workspace.rs
index ce53263..fcd825d 100644
--- a/src/ui/workspace.rs
+++ b/src/ui/workspace.rs
@@ -2029,7 +2029,8 @@ fn status_line(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId) {
     let review_key = key(Command::ReviewSql);
     let (agents_copy_key, agents_on_key) =
         (key(Command::CopyAgentCommand), key(Command::TurnAgentsOn));
-    let agents_allow_key = key(Command::AllowAgents);
+    let (agents_allow_key, agents_settings_key) =
+        (key(Command::AllowAgents), key(Command::ShowAgentSettings));
     let (agents_move_key, agents_stop_key, agents_here_key) = (
         key(Command::MoveInAgentSessions),
         key(Command::StopAgentSession),
@@ -2257,23 +2258,27 @@ fn status_line(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId) {
             }
             let (copy_word, on_word) = (gettext(locale, "copy"), gettext(locale, "turn on"));
             let allow_word = gettext(locale, "allow agents");
+            let settings_word = gettext(locale, "settings");
             let stop_words = [
                 gettext(locale, "sessions"),
                 gettext(locale, "stop session"),
                 gettext(locale, "stop agents here"),
             ];
             let mut agents_hints: Vec<widgets::Hint<'_>> = match &agents {
-                Some(crate::model::AgentsHere::ServerOff) => {
-                    vec![(&agents_on_key, &*on_word, true)]
-                }
+                Some(crate::model::AgentsHere::ServerOff) => vec![
+                    (&agents_on_key, &*on_word, true),
+                    (&agents_settings_key, &*settings_word, true),
+                ],
                 Some(crate::model::AgentsHere::NoneConnected) if agents_listed => vec![
                     (&agents_move_key, &*stop_words[0], true),
                     (&agents_stop_key, &*stop_words[1], true),
                     (&agents_copy_key, &*copy_word, true),
+                    (&agents_settings_key, &*settings_word, true),
+                ],
+                Some(crate::model::AgentsHere::NoneConnected) => vec![
+                    (&agents_copy_key, &*copy_word, true),
+                    (&agents_settings_key, &*settings_word, true),
                 ],
-                Some(crate::model::AgentsHere::NoneConnected) => {
-                    vec![(&agents_copy_key, &*copy_word, true)]
-                }
                 Some(crate::model::AgentsHere::Connected { .. }) => vec![
                     (&agents_move_key, &*stop_words[0], true),
                     (&agents_stop_key, &*stop_words[1], true),
````

- [ ] **Step 4: Run the tests**

Run: `cargo test --locked --lib ui::agent_settings_tests`, then `cargo test --locked --lib ui::agents_tests`, `cargo test --locked --lib keymap::` and `cargo test --locked --test keys`

Expected: PASS.

```text
test result: ok. 13 passed; 0 failed; 0 ignored; 0 measured; 2520 filtered out
test result: ok. 34 passed; 0 failed; 0 ignored; 0 measured; 2499 filtered out
test result: ok. 23 passed; 0 failed; 0 ignored; 0 measured; 2510 filtered out
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

- [ ] **Step 5: Commit**

```bash
cargo fmt --all
cargo clippy --locked --workspace --all-targets -- -D warnings
git add src
git commit -m "Open Settings › Agents from the Agents tab"
```

---

### Task 4: The website's guide to Agents

- `docs/_guide/agents.md`: what an agent can do, turning the server on, connecting Claude Code, Cursor and others, allowing a connection, the Agents tab and its log, the notification, and changes made outside. Sixth among the guides; Omarchy and macOS move down one.
- `docs/_reference/settings-and-files.md`: the Agents pane's options; the `[mcp]` keys are no longer the file's alone; the switch is never held.
- `docs/_guide/omarchy.md`: the Settings screen's `ctrl+h`/`ctrl+l`, `enter` and `y`; the Agents tab's `s`.

**Files:**
- Create: `docs/_guide/agents.md`
- Modify: `docs/_guide/omarchy.md`, `docs/_guide/macos.md`, `docs/_reference/settings-and-files.md`

- [ ] **Step 1: The pages**

````diff
diff --git a/docs/_guide/agents.md b/docs/_guide/agents.md
new file mode 100644
index 0000000..522eacc
--- /dev/null
+++ b/docs/_guide/agents.md
@@ -0,0 +1,134 @@
+---
+title: Agents
+description: Let coding agents such as Claude Code and Cursor read the databases you choose through Tabletist, over MCP, and see and stop what they do.
+nav_order: 6
+---
+
+Tabletist can let coding agents, such as Claude Code and Cursor, read the
+databases you choose. An agent asks Tabletist over the Model Context
+Protocol (MCP), and Tabletist runs the query on your saved connection,
+with that connection's own login and its limits. Passwords stay in the
+system keyring and never reach the agent.
+
+Agents work on Linux and macOS. On Windows nothing of them shows yet.
+
+## What an agent can do
+
+An agent has three tools:
+
+| Tool | What it does |
+| --- | --- |
+| `list_connections` | Lists the connections you allowed agents, with what each is and its limits |
+| `describe_schema` | Names a connection's tables and views, and describes the columns and keys of those it asks about |
+| `run_query` | Runs one `SELECT`, `WITH … SELECT`, `VALUES` or `EXPLAIN` of one, and returns its rows |
+
+Every read runs in a read-only transaction that is rolled back, with the
+connection's limits:
+rows per answer and a statement timeout. Tabletist checks each statement
+before it runs. A statement that would write, lock or change the schema is
+refused, and so are several statements at once and `COPY`. An agent sees
+what the connection's database user can see, and never more.
+
+Agents cannot write in this version.
+
+## Turn it on
+
+The agent server is off until you turn it on, in either place:
+
+- **Settings › Agents**: the **Let agents use Tabletist** switch.
+- A connection's Agents tab, while the server is off: **Turn on**.
+
+The server runs only while Tabletist is open, and only on this computer:
+an agent reaches it through `tabletist mcp`, a small program it starts,
+which talks to the running app. Turning the switch off ends every agent's
+session until you turn it on again.
+
+Turning the server on lets no agent see any connection yet. That is the
+next step.
+
+## Connect an agent
+
+**Settings › Agents** shows how, under **Connect an agent**. Choose the
+agent, then **Copy** what it takes:
+
+- **Claude Code**: a command to run once in your project:
+
+  ```sh
+  claude mcp add tabletist -- tabletist mcp
+  ```
+
+- **Cursor**: an entry under `mcpServers` in `~/.cursor/mcp.json`, or in
+  `.cursor/mcp.json` in a project. A file that has other servers already
+  takes the `tabletist` entry beside them:
+
+  ```json
+  {
+    "mcpServers": {
+      "tabletist": { "command": "tabletist", "args": ["mcp"] }
+    }
+  }
+  ```
+
+- **Other**: any agent that starts an MCP server over stdio runs the
+  command `tabletist mcp`.
+
+Where `tabletist` on your `PATH` is not the app you run, the command names
+the app by its whole path instead. On macOS that is the program inside the
+app: `/Applications/Tabletist.app/Contents/MacOS/tabletist mcp`. Copy gives
+the one your Tabletist needs.
+
+In the Omarchy look, `y` on the Agents pane copies Claude Code's command.
+
+## Allow a connection
+
+Every connection starts with agents **Off**: they do not see it at all.
+**Read** lets them read it. Set it in any of these places:
+
+- In Connections: a row's "…" menu, or `a` on a row in the Omarchy
+  look.
+- In the connection's form: its **Agents** section, which also has the
+  rows per answer and the statement timeout.
+- In the connection's Agents tab, while it says agents can't use it:
+  **Allow agents…**, which opens that section with **Read** chosen.
+
+[Connections]({% link _reference/connections.md %}#agents) describes the
+section.
+
+## See what agents do
+
+Each connection window has an Agents tab for its connection. It opens
+from the **Agents** entry at the bottom of the sidebar, from Cmd+Option+A
+on macOS or `ga` and `:agents` in the Omarchy look, from quick open on
+macOS, from Connections, and from **Show Agents tab** in **Settings ›
+Agents**. Nothing
+an agent does opens it for you.
+
+The tab shows:
+
+- whether agents can reach the connection, and if not, the way to let
+  them;
+- the agents working on it, each with **Stop session**, and the ones
+  working elsewhere;
+- the calls they make, as they make them.
+
+**Stop agents here** ends every session on the connection and sets its
+access to **Off**, so that an agent that connects again does not see it.
+
+The tab's **Log** view lists every call agents made, on this connection or
+on all of them: the tool, the statement, how many rows it returned and how
+long it took. Rows themselves are never logged. **Settings › Agents ›
+Keep the activity log** says for how many days a call is kept.
+
+When an agent connects, Tabletist tells you: a notification on Linux,
+whose **Review** shows the Agents tab, and the Dock icon bouncing on macOS.
+**Notify when an agent connects** turns it off.
+
+## Changes made outside Tabletist
+
+Agents cannot change Tabletist's files through MCP. A program that edits
+`settings.toml` or `connections.toml` to let agents do more is held back
+until you answer: see
+[Settings and Files]({% link _reference/settings-and-files.md %}#agent-access-changed-outside-tabletist).
+
+The Omarchy look's keys for the Agents tab and the Settings screen are in
+[The Omarchy Look]({% link _guide/omarchy.md %}#the-agents-tab).
diff --git a/docs/_guide/macos.md b/docs/_guide/macos.md
index 6bb37ac..125c665 100644
--- a/docs/_guide/macos.md
+++ b/docs/_guide/macos.md
@@ -1,7 +1,7 @@
 ---
 title: Tabletist on macOS
 description: How Tabletist looks and works on a Mac, the keys of the macOS look, the Keychain, and where its files are.
-nav_order: 7
+nav_order: 8
 ---
 
 On a Mac, Tabletist has the macOS look: rounded controls, IBM Plex Sans
diff --git a/docs/_guide/omarchy.md b/docs/_guide/omarchy.md
index 8b476ce..58e9b7b 100644
--- a/docs/_guide/omarchy.md
+++ b/docs/_guide/omarchy.md
@@ -1,7 +1,7 @@
 ---
 title: The Omarchy Look
 description: How Tabletist looks and works on Linux, the keys of the Omarchy look, how it follows the Omarchy theme, and how to use your own colors.
-nav_order: 6
+nav_order: 7
 ---
 
 On Linux, Tabletist takes the Omarchy look: square corners, the keyboard
@@ -111,6 +111,7 @@ key. Nor has copying a whole row.
 | --- | --- |
 | `y` | Copy the command that connects an agent |
 | `o` | Turn the agent server on, while it is off |
+| `s` | Settings › Agents, while the server is off or no agent is here |
 | `e` | Allow agents on this connection, while their access is off |
 | `j` / `k` | Move among the agents' sessions: those here, then those elsewhere |
 | `S` | Stop the selected session, wherever it works |
@@ -248,6 +249,9 @@ offers.
 | `j` `k` | Move |
 | `h` `l` | Change the option |
 | Space | Toggle the option |
+| Ctrl+H, Ctrl+L | The pane before or after: General, Agents |
+| Enter | On the Agents pane: show the Agents tab, Connections or the log, from the status, access or log row |
+| `y` | On the Agents pane: copy the command that connects Claude Code |
 | Ctrl+E | Open `settings.toml` in your editor |
 | `R` | Reset the option |
 | Esc | Close |
diff --git a/docs/_reference/settings-and-files.md b/docs/_reference/settings-and-files.md
index c4dde1c..010642b 100644
--- a/docs/_reference/settings-and-files.md
+++ b/docs/_reference/settings-and-files.md
@@ -17,9 +17,20 @@ Cmd/Ctrl+, opens Settings. Changes apply right away.
 | **Numbers** | `1240.50` or `1,240.50`. Grouping is for display only: a copy gives the raw value | `1240.50` |
 | **Value tags** | Colors for enum, CHECK and boolean columns | On |
 
+On Linux and macOS a second pane, **Agents**, sets what the `[mcp]` keys
+hold, and shows how to connect an agent:
+[Agents]({% link _guide/agents.md %}) is the guide.
+
+| Option | Choices | Default |
+| --- | --- | --- |
+| **Let agents use Tabletist** | On or off. Off ends every agent's session | Off |
+| **Keep the activity log** | 7, 30, 90 or 365 days | 30 days |
+| **Notify when an agent connects** | On or off | On |
+
 On macOS and Windows the window shows where the settings file is, and can
-reveal it in your file manager, export it, or reset every option. In the
-Omarchy look the file is shown beside the options.
+reveal it in your file manager, export it, or reset the options of the
+pane it shows. On the Agents pane, a reset leaves the server's switch as
+it is. In the Omarchy look the file is shown beside the options.
 
 {% include shot.html file="omarchy-settings" alt="The Settings screen in the Omarchy look, with settings.toml beside the options" width="2880" height="1800" %}
 
@@ -62,8 +73,7 @@ notify = true
 | `mcp.log_days` | 1 to 3650 days. How long the log of agents' calls keeps a call | `30` |
 | `mcp.notify` | `true` tells you when an agent connects: a notification on Linux, the Dock icon bouncing on macOS | `true` |
 
-`show_system_schemas`, `theme` and the three `mcp` keys are set only in the
-file. The SQL
+`show_system_schemas` and `theme` are set only in the file. The SQL
 editor's **Limit** and **Timeout** menus set the two `editor` keys.
 
 A line that cannot be read is ignored, keeps its default, and is reported
@@ -184,7 +194,7 @@ back to `off`, and leaves your other settings and connections as they
 are. A change that lets agents do less applies at once.
 
 Turning agents on in Tabletist itself, with **Turn on** in a connection's
-Agents tab, is never held. Tabletist keeps what it showed you in
+Agents tab or the switch in **Settings › Agents**, is never held. Tabletist keeps what it showed you in
 `agents_shown.json`. Without that file, the first time a version with this
 check starts, whatever you turned on by hand is held once.
 
````

- [ ] **Step 2: Check them**

Run: `docs/bin/check`, and `grep -c "$(printf '\342\200\224')" docs/_guide/agents.md docs/_guide/omarchy.md docs/_reference/settings-and-files.md` (an em dash, written in octal so that this plan holds none)

Expected: the check ends `HTML-Proofer finished successfully.`, and each page counts `0`.

- [ ] **Step 3: Commit**

```bash
git add docs/_guide docs/_reference
git commit -m "Add the website's guide to Agents"
```

---

### Task 5: The master plan says what was built

**Files:**
- Modify: `docs/superpowers/plans/2026-10-13-mcp-server.md`

- [ ] **Step 1: Correct it**

````diff
diff --git a/docs/superpowers/plans/2026-10-13-mcp-server.md b/docs/superpowers/plans/2026-10-13-mcp-server.md
index e8486b9..c458f33 100644
--- a/docs/superpowers/plans/2026-10-13-mcp-server.md
+++ b/docs/superpowers/plans/2026-10-13-mcp-server.md
@@ -1483,6 +1483,41 @@ front when Settings opened. With no connection window open both are
 disabled, with the reason beside them ("Open a connection to see its
 agents"). The board does not draw that case.
 
+**As built in the second part of Phase 1b's fifth run** (`2026-10-15-mcp-server-1b-5b-settings.md`). Where
+it draws otherwise than the boards:
+
+- The nav lists General and Agents, and General alone where agents cannot
+  reach the app (Windows). On Omarchy `ctrl+h` and `ctrl+l` switch panes,
+  and the nav's foot says so.
+- The switch is `mcp.enabled` as it is in force: off while a change made
+  outside the app that turned it on is held. Turning it on is the
+  person's own, as **Turn on** is, and keeps what was held. Turning it off
+  ends every session: it is "Stop all agents" (question 11).
+- The status says "stdio · this computer only" until HTTP (1d), and how
+  many agents are connected. **Show Agents tab** and **Show log** close
+  Settings and show the tab of the window in front, or with Connections
+  in front of the last window open, the way Review does.
+- "Connect an agent" switches between Claude Code's command, Cursor's
+  entry for its `mcp.json`, and for Other the command `tabletist mcp`,
+  each with **Copy** (agreed on 2026-10-15). The list of agents and
+  "Removing it here disconnects it at once." come with HTTP (1d).
+  Omarchy draws the board's box with `y`, and "cursor, others" with the
+  command.
+- The Access box says "Off or Read" and the safety box leaves out writes:
+  both until Phase 2. **Open Connections** closes Settings.
+- The log's row offers 7, 30, 90 and 365 days, with a number set in the
+  file in its place among them. Its small print says the log keeps "never
+  the rows", where the board says "what was returned". "Notify when an
+  agent connects" leaves out writes until Phase 2.
+- Omarchy's rows under the cursor are the switch, the status (`enter`
+  shows the Agents tab), the access (`enter` opens Connections, and the
+  hint names `a`), the log's days (`enter` shows the log) and the
+  notification. The command, the line for other agents and the note are
+  not rows. The hints are shorter than the board's, to fit beside the file
+  at 1440 wide.
+- Reset to defaults puts back the options of the pane it is asked on, and
+  on the Agents pane leaves the server's switch as it is.
+
 ### D.2 The Connections list
 
 Boards: `macOS – Connections, agent access`, `Omarchy – Connections, agent
@@ -1768,6 +1803,11 @@ connects. Where it draws otherwise than the boards:
 - The clock is jiff's, in the system's time zone: dates in the log, and
   times in the feed and on a session's line.
 
+**As built in the second part of Phase 1b's fifth run** (`2026-10-15-mcp-server-1b-5b-settings.md`):
+**Settings…** beside **Turn on** with the server off, and **Other
+agents…** beside **Show log** with nobody here, both opening Settings ›
+Agents; on Omarchy `s` in both states, with its hint on the bottom line.
+
 | State | Title | Body | Actions |
 |---|---|---|---|
 | Allowed, no agents yet | No agents connected to Bookshop · dev | "Agents like Claude Code can read this database and propose changes you approve here. Connect one with this command, then ask it about Bookshop." Then the command with **Copy**, and a box "On this connection": access, hidden tables, masked columns, last agent | **Other agents…**, **Show log** |
@@ -2138,7 +2178,7 @@ is off until the person turns it on.
 | Part | Runs | State on 2026-10-13 |
 |---|---|---|
 | 1a. The enforcement core | 3 | Done: pull requests 122, 123 and 124 |
-| 1b. The server over stdio, with its interface | 7, below | The first four and a half are built (`2026-10-13-mcp-server-1b-1-server.md`, `2026-10-14-mcp-server-1b-2a-gateway.md`, `2026-10-14-mcp-server-1b-2b-stdio.md`, `2026-10-14-mcp-server-1b-3a-agents-tab.md`, `2026-10-14-mcp-server-1b-3b-changes-outside.md`, `2026-10-14-mcp-server-1b-4a-activity.md`, `2026-10-14-mcp-server-1b-4b-log.md` and `2026-10-15-mcp-server-1b-5a-access.md`). Nothing the others wait for is open |
+| 1b. The server over stdio, with its interface | 9, below | Built on Linux and macOS (`2026-10-13-mcp-server-1b-1-server.md`, `2026-10-14-mcp-server-1b-2a-gateway.md`, `2026-10-14-mcp-server-1b-2b-stdio.md`, `2026-10-14-mcp-server-1b-3a-agents-tab.md`, `2026-10-14-mcp-server-1b-3b-changes-outside.md`, `2026-10-14-mcp-server-1b-4a-activity.md`, `2026-10-14-mcp-server-1b-4b-log.md`, `2026-10-15-mcp-server-1b-5a-access.md` and `2026-10-15-mcp-server-1b-5b-settings.md`). Windows' way in is next |
 | 1b on Windows: the way in | 1 | After 1b (question 14) |
 | 1c. Hidden tables | to be cut in its plan | Waits for questions 8 and 13 |
 | 1d. HTTP with tokens | 1 | After stdio (question 9) |
@@ -2212,7 +2252,7 @@ can see what agents do and stop them.
 | **1b-4a. What agents are doing** (built: `2026-10-14-mcp-server-1b-4a-activity.md`) | `App::agents`'s feed, from `Server::calls`. The sessions that are here and "Also:". **Stop session** and **Stop agents here** | See each agent and each call as it happens, stop one agent or all on a connection | F.7: one connection, the stops, closing the tab |
 | **1b-4b. The log, and a new agent** (built: `2026-10-14-mcp-server-1b-4b-log.md`) | The Log view as a list for this connection, with All connections, and **Show log**. The notification for an agent that connects | Read the log, and hear of an agent that connects | F.7: the log's default. F.4's interface |
 | **1b-5a. Letting agents in, per connection** (built: `2026-10-15-mcp-server-1b-5a-access.md`) | The Connections list's Agents column, its menu and Omarchy's box. The connection editor's Agents section, with **Allow agents…** in the Agents tab and **Show Agents tab** in the list. The reference pages | A connection is opened to agents, and closed, without a file edited by hand | The interface tests of Connections and the editor. The keymap. `docs/bin/check` |
-| **1b-5b. Settings › Agents** | Settings › Agents, with the switch between panes, **Show Agents tab** and **Show log** there, the Agents tab's **Other agents…** and **Settings…**. The website's guide to Agents | Everything Phase 1 draws | The interface tests of Settings. The keymap. `docs/bin/check` |
+| **1b-5b. Settings › Agents** (built: `2026-10-15-mcp-server-1b-5b-settings.md`) | Settings › Agents, with the switch between panes, **Show Agents tab** and **Show log** there, the Agents tab's **Other agents…** and **Settings…**. The website's guide to Agents | Everything Phase 1 draws | The interface tests of Settings. The keymap. `docs/bin/check` |
 
 - **A button arrives with what it opens.** The empty states of the third
   run have **Turn on** and **Copy** and no other action. **Show log** comes
@@ -2356,7 +2396,8 @@ The ones Phase 1 waits for are marked.
     answers with an error and exits, as planned. No `--launch` flag in
     Phase 1, and no relay that stays up and waits (A.2 says why).
 11. "Stop all agents" switches the server off until it is turned on again.
-    Accepted?
+    Built so in Phase 1b's fifth run (`2026-10-15-mcp-server-1b-5b-settings.md`): it is the switch in
+    Settings › Agents.
 12. Omarchy: `a` and `d` are keys of the Agents tab alone, and in any other
     tab the bottom line ends `ga agents` where the board has `a approve  d
     deny  esc later`. Accepted? The connections screen needs no `:` prompt
````

- [ ] **Step 2: Check it**

Run: `grep -c "$(printf '\342\200\224')" docs/superpowers/plans/2026-10-13-mcp-server.md`

Expected: `0`.

- [ ] **Step 3: Commit**

```bash
git add docs/superpowers/plans/2026-10-13-mcp-server.md
git commit -m "Say in the MCP plan what the second part of its fifth run built"
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

Expected: all clean, and every test passes. On 2026-10-15, with both servers up: 23 test binaries, 3,482 passed, none failed, 22 ignored. The website's check ends `HTML-Proofer finished successfully.`

- [ ] **By hand, on Linux and on macOS**: Settings (Cmd/Ctrl+,), then Agents (Omarchy: `ctrl+l`). Turn the switch on: the status says Running; connect Claude Code with the copied command, and it says 1 agent connected. Switch to Cursor and copy its entry. **Show Agents tab** shows the window in front; **Show log** its log. Turn the switch off: the agent's session ends. In an Agents tab with the server off, **Settings…** (Omarchy: `s`) opens the pane.

- [ ] **Open the pull request.**

## Self-review

- **Against the master plan.** D.1's rows: the pane switch (Task 2), the macOS pane with the toggle, the status row with **Show Agents tab**, "Connect an agent" with the three-way switch and Copy, the Access box with **Open Connections**, "How it stays safe", the two Log rows with **Show log** (Task 2); Omarchy's nav, rows with the cursor, `y`, `enter` on the access row, and the file pane marking the `[mcp]` line of the cursor's option (Task 2); the command as the backend works it out (Task 1). The list of agents under "Connect an agent" is HTTP's (1d), as A.3 has it. D.1's change of 2026-10-13, the disabled buttons with no window open: Task 2. D.4's **Other agents…** and **Settings…** and their `s`: Task 3. D.6's `y` in Settings and `s` in the Agents tab: Tasks 2 and 3. G's guide: Task 4.
- **A review before the plan.** A reviewer tried to break the draft and found six faults, each fixed here with a test that fails without its fix: a held Enter on the status row went past the Agents tab and connected another database; Reset to defaults on the Agents pane turned the server off; the log's row said "enter shows it" with no window to show it in; the Omarchy pane's columns moved when the server turned on; at 720 wide the footer's `esc close` was off the window; the reference still said Reset puts back every option. Three smaller ones are fixed too: the switch turned on when on already wrote `agents_shown.json`, an `allow` had no reason, and the guide named the wrong menu. It found nothing that lets the pane raise agents' access, or keep a change made outside, other than the person turning the switch on.
- **What the review suspected and this run leaves.** Cursor's entry names `tabletist` where it is on the `PATH`, and Cursor started from a desktop may have a shorter `PATH`: the command does the same for Claude Code, and the app's whole path would break an AppImage, whose path changes each run. At 720 wide the macOS look wraps Cursor's hint over several lines; Copy copies the entry whole.
- **The held-change rule.** The switch shows `agents_on()`; on goes through `turn_agents_on`, which keeps only the server's raise, so a raised access stays held; off writes `enabled = false`, which is the put-back and raises nothing. Log days and the notification leave a held server held.
- **Placeholders.** None: every step has its diff or its command and what it prints.
- **Names.** `SettingsPane`, `SettingsRow`, `SettingsDialog::{pane, agent}`, `OptionId::{AgentsOn, LogDays, AgentsNotify, GENERAL, AGENTS}`, `Settings::LOG_DAYS`, `Action::{ShowAgentSettings, ShowSettingsPane, ChooseAgentSetup, ShowAgentsFromSettings}`, `Command::{SettingsPane, SettingsOpenRow, SettingsCopyCommand, ShowAgentSettings}`, `App::agents_window`, `endpoint::{Agent, agent_setup_ready}` are the same in every task that uses them.
