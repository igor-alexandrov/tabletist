# Settings › Updates, run 2: download and install Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** From Settings › Updates, Tabletist downloads the version offered when you choose Download and install (Omarchy `U`, or `:update` after a confirm line), shows it downloading, verifies it, and when you choose Restart now installs it and relaunches into the same windows, waiting first for any unsaved edits, saves and running queries.

**Architecture:** fastframe-update does the download, the verification and the swap: `Updater::download` on the backend's blocking pool, with progress events. Our own `Transport` adds cancel, waiting while offline and resuming. `Updater::handoff` starts the crate's helper, and the app quits. The app adds:
- the card's new states;
- the work-in-progress check, from the edits, saves and queries it already tracks;
- a one-time relaunch file for the SQL editors' text;
- a hidden `--relaunch` flag that restores the last session whatever the launch setting says.

**Tech Stack:** Rust, egui/eframe, AccessKit, fastframe-update v0.13.0, ureq 3.

---

## Before Task 1

Commit this plan:

```bash
git add docs/superpowers/plans/2026-10-28-settings-updates-2-install.md
git commit -m "Plan Settings › Updates, run 2: download and install"
```

## Where this sits

Run 1 (#240, merged at `420ca4b5`) built the check, the pane in both looks, install detection and the receipt. This is run 2, a branch of `main` at `420ca4b5`. Run 3 is going back: the footer's Go back, its question, the start screen's offer after a crash, and the `--update-error` message after the crate's automatic rollback.

## Decisions and departures, for a yes or a no before code

The first rows follow from what fastframe-update can and cannot do. The rest fill gaps in the prompt and the boards.

| # | The prompt or the boards | Built (recommended) | Why, and the other way |
|---|---|---|---|
| 1 | "Pauses when offline and resumes; resume after a dropped connection" (test 6) | In our `Transport`. The download's body reader reconnects after a dropped connection with `Range: bytes=<n>-`, every 2 s for up to 10 minutes, and goes on where it stopped. While it waits, the card says "Waiting for the network · 18.4 of 31.2 MB". After 10 minutes it stops: Couldn't download (row 6) | fastframe-update has no resume, pause or Range. To the crate, the reconnects look like one unbroken stream, and it still checks the whole file's hash. The other way: a dropped connection ends the download |
| 2 | "Can be cancelled" | Cancel (sheet) and `ctrl+c` (Omarchy, as the components' progress row has it) set a flag the reader checks. The read fails, and the crate deletes its folder | The crate's progress callback cannot stop it |
| 3 | "Partial files live in the app's cache directory" | In the crate's staging folder beside the app (`.tabletist-update-*`, mode 0700). It is deleted on cancel or failure, and swept by the next download | The swap is a rename, which needs the same disk. The crate does not make it configurable |
| 4 | "Runs at low priority" | Its own thread on the backend's blocking pool, network-bound. No priority is set | Nothing in std sets a thread's priority on all three systems. A frame never waits for it |
| 5 | Verification: release signature, macOS `codesign` and Team ID | All are the crate's: the signed `checksums.txt` before any byte is kept, then the SHA-256, then the `--version` probe; on macOS the bundle identity, `codesign --verify --deep --strict`, the Team ID and Gatekeeper | Its tests cover them. Nothing is installed from a file that failed, and the crate offers no way to try |
| 6 | Verification failed, "Download again", "Report on GitHub" | The crate's errors carry no type, so the app sorts their messages. Signature, checksum, hash, probe, wrong-version, other-publisher, app-bundle and macOS approval failures give **Verification failed** (the card's title reads "The 0.2.0 download didn't verify"), with the error box saying what failed and "The file was deleted." A dropped download that does not come back, a server error or the folder not writable give **Couldn't download 0.2.0**, a state the boards do not draw: Couldn't check's card with "Nothing was installed. Your copy is unchanged.", the error, and **Try again**. Report on GitHub opens a new issue on github.com with the version, the system and the error filled in. Nothing is sent until the person submits it | A network failure is not a bad file. The other way: Verification failed for every error |
| 7 | Downloading: "18.4 of 31.2 MB · about 20 s left", a progress bar | The sub line with the time left from the last 5 s' speed, left out for the first 2 s. The bar is the board's 6 pt bar. For a screen reader it is a progress indicator, 0 to the size in MB, with its value as "18.4 of 31.2 MB". After the last byte the line says "Verifying…" (the crate checks then, for up to a few seconds) | The crate reports each MiB and the end |
| 8 | Ready, Later, "quitting doesn't install it" | Ready holds the crate's `Prepared` on the backend. Later keeps it. A quit drops it (its folder is swept by the next download), so the next launch shows Available again | `Prepared` cannot be saved. Installing on quit is out of scope |
| 9 | Work in progress: unsaved edits, **open transactions**, running queries | The card's title is "Finish your work first". It lists each window's (connection tab's) unsaved edits per table ("Bookshop · dev: 3 unsaved edits in book_covers"), saves running ("saving book_covers"), write runs in an SQL editor ("a write running in Query 2", the nearest Tabletist has to an open transaction), other running queries ("query running for 1 min 12 s"), and tables being read ("loading book_covers"). The list is a list for a screen reader | Tabletist opens no transaction that outlives a run: each run and each save is its own. The other way: build open transactions first |
| 10 | Restart when done; "nothing is ever discarded" | The list is read again every frame. When it is empty, the restart goes ahead. While it waits, the card's primary button is **Don't restart**, which goes back to Ready | The board has no way to change your mind. The other way: no button, and closing the pane does not cancel it |
| 11 | Show them | Closes Settings, brings the window forward, and makes the first connection tab with work active, on the table or SQL tab that holds it | The board does not say |
| 12 | Restore windows, tabs, filters and SQL editor text "as Restart last session does" | The helper relaunches with the launch's arguments plus `--relaunch` (hidden), which restores the last session whatever **On launch** says. Before the handoff, each window's open SQL editors (their number, text, mode and place among the tabs, and the tab in front) are written to `relaunch.json` in the state folder. The relaunched copy reads and deletes the file once, and reopens them. Normal quits are unchanged | Your choice: the session keeps no SQL tabs today, and their text is work |
| 13 | Start screen line "0.2.0 is ready · Restart" (no board draws it) | On the macOS start screen, where the "something is waiting" lines go (between the search field and the recents), muted: "0.2.0 is ready · Restart", where Restart is a link. Its click is Restart now, so with work open it shows Settings › Updates and Work in progress. Not in the Omarchy look | The Ready card's note asks for it. Its place is inferred |
| 14 | Omarchy keys: `U` download and install, "Ready: restart now"; the board's key line "U update now" | The board's words. Available: `U update now`. Downloading: the components' progress row, for example `⠋ [███████░░░░░] 58% · 18.4 of 31.2 mb · about 20 s left · ctrl+c cancel` (12 cells). Ready: `U restart now`. Work in progress: the list as `- ` lines and `U restart when done`. Verification failed, Couldn't download and Couldn't install: `U download again`. Omarchy has no Report on GitHub | No board draws these states in Omarchy. They are built from its block and its components |
| 15 | `:update`: "checks, then downloads and installs after a confirm line ('install 0.2.0 and restart? y/n')" | `:update` opens the pane and checks (run 1). Where the answer is a version this copy can install, the settings footer asks "install 0.2.0 and restart? [y] yes [N] no", in the components' confirm style. `y` downloads, then restarts once Ready, through the work check. `U` on the pane asks nothing | The confirm line stands where the history question does, in the settings footer, since `:update` shows the pane |
| 16 | Windows | No download on Windows, as run 1 said (its row 14): the card offers the release page | Windows updates are out of scope |
| 17 | Tests 6 to 9 | 6: our transport against a local server (resume after a dropped connection, waiting while offline, cancel). 7 and 8: the crate's own tests. A test binary is no installation the crate can update, and its fakes are private, so the app tests which state each error gives and what it hands the crate. 9: the work list, Restart when done and Don't restart. The relaunch file round-trips | The other way: a packaged build in CI, for a later run |
| 18 | Ready: **Later** | Keeps Ready and closes Settings. The dot and the start screen line stay | Your choice: Later is "not now" |
| 19 | (the relaunch and **Reopen tabs**) | The relaunch reopens the tables too, even with Reopen tabs off | Your choice: an update's restart leaves you where you were, as the prompt asks |
| 20 | (a handoff that fails) | **Couldn't install 0.2.0**, Couldn't check's card with the error and **Try again**: for a helper that did not start, for the helper's answer read too late to quit in time (`QUIT_IN_TIME`), and for work still open when the helper gives up (row 22) | Your choice: no board draws it, and without it a failed handoff shows nothing |
| 21 | (offline when the download starts) | Waits as a drop partway through does: "Waiting for the network" for up to 10 minutes, then Couldn't download | Your choice: "pauses when offline and resumes" from the first byte |
| 22 | (work opened during the handoff's up to 10 s) | The helper is then ready and waits a minute for Tabletist to quit. The card shows Finish your work first with the list, armed: Tabletist restarts as soon as the list is empty within 55 s of the helper's answer (`HELPER_WAITS`, a little short of its minute). After that the card says Couldn't install, which is then true. A quit is begun only within 50 s of the answer (`QUIT_IN_TIME`), so it lands inside the minute. `relaunch.json` is deleted whenever a handoff does not end in the quit | Your choice: a quit inside the minute would install, so the card must not say nothing was installed until it is so. The other way: refuse edits, runs and saves while installing |
| 23 | (the armed Restart when done; no board draws it) | Under Finish your work first: "Tabletist restarts once this is done:", the list, and **Don't restart**. Omarchy: `U don't restart` | Your choice |
| 24 | `:update`: "the same Work in progress check in the status line" | The check shows in the pane's block, which `:update` opens (row 15), not in the workspace status line | Settings covers the status line while it is open

The prompt's other rules hold as written:
- Nothing is discarded to restart.
- Nothing installs on quit. The one exception is row 22's minute, which the card says.
- No error shows outside the card and its signals.
- No UI copy has an em dash.
- Strings go through `gettext`.
- Every test uses the Bookshop data, a release fixture or a server on 127.0.0.1. None reaches GitHub, and none checks a colour, a size or a picture.

## What the reviews found

A draft of this plan's code was built first, then reviewed twice:
- one reviewer tried to break it with tests of its own;
- the other read it against rows 1 to 17, the prompt and the boards.

Every defect below is fixed in the tasks, with a test that fails without the fix. Rows 18 to 24 are your answers to the questions the draft and the reviews raised.

- **A connection that went quiet blocked the download.** Wi-Fi gone with the socket still open blocked the read for up to 15 minutes. Cancel and Waiting did nothing, and a second download could run beside it. Each body is now read on a thread of its own, and the reader wakes every 50 ms. Eight seconds with no byte counts as a dropped connection (`a_cancel_ends_a_read_of_a_connection_gone_quiet`, `a_connection_gone_quiet_waits_for_the_network`).
- **A reconnect that dropped at once looped.** It sent 2931 requests in 5 s and never gave up. The give-up clock now starts at the drop and stops only when bytes come again, and every reconnect after the first waits (`a_reconnect_that_drops_at_once_is_paced_and_gives_up`).
- **Offline at the start failed at once.** It now waits as a drop does (row 21; `a_download_that_starts_offline_waits_for_the_network`).
- **A reconnect could lose the start of its redirects.** It then went to GitHub's signed address, which expires. It now goes back to the first address whatever form the redirect took, and follows redirects only over https (or plain http to 127.0.0.1, for the tests) to hosts the first chain went through (`a_reconnect_starts_from_the_first_address_whatever_form_the_redirect_takes`, `a_reconnect_follows_redirects_over_https_to_the_hosts_it_went_through`).
- **A cancel that came after the file verified left it on disk** (`a_cancel_drops_a_download_that_verified_meanwhile`).
- **Restart when done waited for the next input.** It goes ahead in the frame the work ends (`restart_when_done_goes_ahead_in_the_frame_the_work_ends`). It also waits while another dialog is open, rather than dropping it (`restart_when_done_waits_for_a_dialog_to_close`).
- **The relaunch mixed up SQL editors.** Two windows of one connection got each other's editors back, and every editor came back last, behind the first tab. Each window now gets its own, each editor in its place, in its mode, with the tab that was in front (`each_window_of_one_connection_gets_its_own_editors_back`, `a_relaunch_puts_each_sql_editor_back_in_its_place`, `the_relaunch_keeps_each_editor_s_place_and_mode_and_the_tab_in_front`).
- **Work opened during the handoff.** The card said nothing was installed while a quit still would install (row 22; `work_opened_while_the_helper_got_ready_is_waited_for`, `work_still_open_when_the_helper_gives_up_is_couldnt_install`, `dont_restart_while_the_helper_waits_is_ready_until_it_gives_up`, `ready_after_the_handoff_says_a_quit_installs_it`).
- **The helper's answer waited for a frame.** With the window hidden, the quit could come after the helper had given up. `logic` now takes it. An answer read too late quits nothing and says so (`the_helper_s_answer_is_taken_with_the_window_hidden`, `a_handoff_read_too_late_quits_nothing`).
- **`relaunch.json` stayed behind when no quit came.** It holds the SQL text. It is now written by the backend in the install itself, and deleted whenever the handoff does not end in a quit (`forget_relaunch_deletes_the_file`, `a_helper_that_did_not_start_leaves_the_app_open`).
- **The work list missed tables being read.** That is a page, a count, a structure or a DDL (`a_table_being_read_is_work`). A list that empties goes back to Ready rather than say "would interrupt:" over nothing (`a_list_that_empties_is_ready_again`, `a_work_list_that_empties_shows_ready_again`).
- **Skip this version acted while the version downloaded or was ready** (`skip_is_ignored_once_the_version_downloads`, `s_skips_nothing_while_an_update_is_ready`).
- **Smaller fixes:**
  - "1 min 12 s" now goes through `gettext` (`a_running_query_says_how_long_it_has_run`).
  - The Omarchy progress row and the work box take the boards' colours.
  - A backend test now points at 127.0.0.1, not github.com.
  - `restart_when_done_becomes_dont_restart_and_show_them_shows_the_work` and `u_downloads_then_restarts_in_the_terminal_look` now check what their names say.
  - `macos.md` no longer reads as if the restart happens on its own.

What held, among what the reviewers tried:
- **The transport:**
  - a 206 at the wrong byte is refused;
  - a 200 after a Range has what was read skipped;
  - the bytes the crate hashes are the file's, in order;
  - a plain request's body does not resume.
- **The states:**
  - stale progress, waiting and download answers are dropped;
  - Restart now during a download is refused;
  - no check runs during a download or while Ready;
  - Don't restart and Later keep Ready.
- **Quitting:**
  - `closing` bypasses the Leave question;
  - `on_exit` flushes in 5 s at most;
  - nothing else holds the process.
- **The relaunch:**
  - a corrupt or missing `relaunch.json` reads as empty;
  - `--relaunch` with no windows opens the picker;
  - the file is never read without `--relaunch`;
  - `--demo` turns it off.
- **Keys:**
  - `U` and `ctrl+c` act on the Updates pane only;
  - `ctrl+c` as the window really sends it (a copy event) cancels;
  - a held `y` answers the question once;
  - Esc answers no and leaves the pane open.
- **Accessibility:**
  - the progress indicator is 0 to 31.2 with 18.4 and "18.4 of 31.2 MB";
  - the work list's items are the list's, through one container;
  - failures are told assertively, once.
- **Platforms:** Windows downloads nothing. `transport.rs`, `download.rs` and `relaunch.rs` type-check and pass clippy for `x86_64-pc-windows-msvc` and `aarch64-apple-darwin` in a scratch crate; the workspace itself does not cross-compile here.

## Files

| File | Task | What |
|---|---|---|
| `src/update/transport.rs` | 1 (new) | The updater's HTTP client, moved out of `src/update.rs`. Each download shares a `Download` with the backend: its cancel flag and its waiting flag. A body is read on a pump thread, and is resumed with `Range` after a drop or 8 s of quiet. A reconnect starts from the first address of its redirects. `Patience`: every 2 s, for up to 10 minutes |
| `src/update/download.rs` | 1 (new), 2 | `Failure::of`, which sorts the crate's errors into Verification failed and Couldn't download; what each failure says; the time left (the last 5 s' speed, nothing in the first 2 s); the Report on GitHub address |
| `src/backend/updates.rs` | 1, 2 | `UpdatesCommand::{Download, CancelDownload, Install, ForgetRelaunch}` and `UpdatesEvent::{Progress, Waiting, Downloaded, Handed}`. The backend holds the download under way and the verified `Prepared`, and writes `relaunch.json` inside the install |
| `src/backend.rs` (1), `src/update.rs` (1, 2, 3) | 1, 2, 3 | The backend's update state. `update::Install` (Idle, Downloading, Ready, Held, Restarting, Failed), `DOWNLOADS` (not on Windows), `HELPER_WAITS` and `QUIT_IN_TIME` |
| `src/update/relaunch.rs` | 2 (new) | `relaunch.json`: each window's SQL editors, their places, modes and the tab in front. It is read once and deleted |
| `src/app/updates.rs` | 1, 2, 3 | The actions (download, cancel, restart now, later, restart when done, don't restart, show them, report on GitHub), the events, `work_in_progress()`, the restart and the handoff's minute |
| `src/app.rs`, `src/app/editing.rs`, `src/model.rs` (2, 3), `src/entrypoint.rs`, `src/paths.rs` (2) | 2, 3 | `--relaunch` (hidden), and the session restored whatever On launch and Reopen tabs say. The SQL editors are put back. The helper's answer is taken in `logic`. `restart` runs after the actions |
| `src/app/startup_tests.rs`, `tests/cli.rs` | 2 | The relaunch's restore, and the flag |
| `AGENTS.md` | 2 | `--relaunch` joins the update contract |
| `src/ui/settings/updates.rs`, `src/ui/settings/updates_terminal.rs` | 3 | The new card states in both looks, the progress bar (a progress indicator in MB), and the work list as a list |
| `src/ui/settings/mod.rs`, `src/ui/settings/terminal.rs`, `src/keymap.rs` | 3 | `U` bound, `ctrl+c` on the Updates pane, and `:update`'s question in the settings footer (`When::UpdateQuestion`) |
| `src/ui/start.rs` | 3 | The macOS start screen line |
| `src/update/release.rs`, `src/theme.rs`, `assets/icons/triangle-alert.svg` | 3 | `megabytes_figure` and `megabytes_value`, the size in MB for the bar; the warning icon (Lucide) |
| `src/ui/update_settings_tests.rs` | 3 | The pane's tests |
| `docs/_reference/settings-and-files.md`, `docs/_guide/{download,macos,omarchy}.md` | 3 | The pages `AGENTS.md` ties to this |

### Task 1: The download on the backend

**Files:** `src/update/transport.rs` (new), `src/update/download.rs` (new), `src/update.rs`, `src/backend.rs`, `src/backend/updates.rs`, `src/app/updates.rs`.

`Updater::download` runs on the backend's blocking pool:
- its progress comes back as `UpdatesEvent::Progress`;
- `Waiting` comes when the transport waits for the network;
- `Downloaded` answers `Ok`, with the verified download held on the backend as `Prepared`, or a sorted `Failure`.

`CancelDownload` sets the download's cancel flag and drops a `Prepared` that came meanwhile. `Install` hands the held `Prepared` to the crate's helper and answers `Handed`.

The transport's tests run against a server on 127.0.0.1. `download.rs`'s tests, of the sorting, the time left and the issue address, need no server. Both are in the new files, with the code. Step 1 has the backend's own three tests.

- [ ] **Step 1: Write the failing tests**

`src/backend/updates.rs`:

```diff
diff --git a/src/backend/updates.rs b/src/backend/updates.rs
index 8a56850b..05d52589 100644
--- a/src/backend/updates.rs
+++ b/src/backend/updates.rs
@@ -179,3 +179,75 @@ mod tests {
         assert!(launched(command, LAUNCH_FAILS_WITHIN).is_err());
     }
 }
+#[cfg(test)]
+mod handed {
+    use super::*;
+    use crate::backend::{Backend, Command, Event, Waker};
+
+    /// The first update event the backend sends within a few seconds.
+    fn answer(backend: &mut Backend) -> UpdatesEvent {
+        let deadline = Instant::now() + Duration::from_secs(20);
+        while Instant::now() < deadline {
+            for event in backend.poll() {
+                if let Event::Updates(event) = event {
+                    return event;
+                }
+            }
+            std::thread::sleep(Duration::from_millis(10));
+        }
+        panic!("no answer");
+    }
+
+    #[test]
+    fn install_with_no_download_held_hands_nothing_over() {
+        let mut backend = Backend::start_with(Waker::default(), crate::secrets::Keyring::memory());
+        backend.send(Command::Updates(UpdatesCommand::Install {
+            arguments: vec!["--relaunch".into()],
+        }));
+        assert!(matches!(answer(&mut backend), UpdatesEvent::Handed(Err(_))));
+    }
+
+    /// A test binary is no installation fastframe-update can replace: it
+    /// asks nothing of the network, says Couldn't download, and holds
+    /// nothing to install.
+    #[test]
+    fn a_copy_that_cannot_replace_itself_downloads_nothing() {
+        let mut backend = Backend::start_with(Waker::default(), crate::secrets::Keyring::memory());
+        backend.send(Command::Updates(UpdatesCommand::Download {
+            asked: 7,
+            version: "99.0.0".into(),
+            // Never asked: the copy is refused first.
+            url: "http://127.0.0.1:9/releases/tag/v99.0.0".into(),
+        }));
+        match answer(&mut backend) {
+            UpdatesEvent::Downloaded {
+                asked: 7,
+                result: Err(Failure::Undone(_)),
+            } => {}
+            other => panic!("{other:?}"),
+        }
+        backend.send(Command::Updates(UpdatesCommand::Install {
+            arguments: Vec::new(),
+        }));
+        assert!(matches!(answer(&mut backend), UpdatesEvent::Handed(Err(_))));
+    }
+
+    /// A download that verified as the cancel came is not left held: the
+    /// next Install hands nothing over.
+    #[tokio::test]
+    async fn a_cancel_drops_a_download_that_verified_meanwhile() {
+        let (events, _received) = std::sync::mpsc::channel();
+        let outbox = crate::backend::Outbox {
+            events,
+            waker: Waker::default(),
+        };
+        let mut updates = Updates::default();
+        let installation = fastframe_update::Installation {
+            executable: std::path::PathBuf::from("/opt/tabletist/tabletist"),
+            kind: fastframe_update::Kind::Portable,
+        };
+        *updates.held.lock().unwrap() = Some(Prepared::sample(installation, "99.0.0"));
+        updates.handle(UpdatesCommand::CancelDownload, &outbox);
+        assert!(updates.held.lock().unwrap().is_none());
+    }
+}
```

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test --locked --workspace --all-targets --no-run`
Expected: no build, 12 errors, such as:

```
error[E0433]: cannot find type `Instant` in this scope
error[E0433]: cannot find type `Updates` in this scope
```

- [ ] **Step 3: Write the code**

`src/app/updates.rs`:

```diff
diff --git a/src/app/updates.rs b/src/app/updates.rs
index ea7594ad..3762b701 100644
--- a/src/app/updates.rs
+++ b/src/app/updates.rs
@@ -140,6 +140,11 @@ impl App {
                 }
             }
             UpdatesEvent::NotesUnshown { url } => self.url_to_open = Some(url),
+            // Nothing asks for a download yet.
+            UpdatesEvent::Progress { .. }
+            | UpdatesEvent::Waiting { .. }
+            | UpdatesEvent::Downloaded { .. }
+            | UpdatesEvent::Handed(_) => {}
             UpdatesEvent::Checked { asked, result } => {
                 let Phase::Checking {
                     asked: waited,
```

`src/backend.rs`:

```diff
diff --git a/src/backend.rs b/src/backend.rs
index ce9a8736..7870cbba 100644
--- a/src/backend.rs
+++ b/src/backend.rs
@@ -1760,6 +1760,8 @@ struct Worker {
     /// The MCP server, its gateway and what agents may use.
     agents: agents::Agents,
     history: history::History,
+    /// The download under way, and the one that verified.
+    updates: updates::Updates,
     #[cfg(test)]
     watched: Watched,
 }
@@ -1779,6 +1781,7 @@ impl Worker {
             waiting: HashMap::new(),
             settings_watch: None,
             history: history::History::default(),
+            updates: updates::Updates::default(),
             #[cfg(test)]
             watched: Watched::default(),
         };
@@ -2112,7 +2115,7 @@ impl Worker {
             }
             Command::Agents(command) => self.agents.handle(command, &self.outbox),
             Command::History(command) => self.history.handle(command, &self.outbox),
-            Command::Updates(command) => updates::handle(command, &self.outbox),
+            Command::Updates(command) => self.updates.handle(command, &self.outbox),
             query => {
                 let session = session_of(&query);
                 match self.sessions.get(&session) {
```

`src/backend/updates.rs`:

```diff
diff --git a/src/backend/updates.rs b/src/backend/updates.rs
index 05d52589..48b1b7ad 100644
--- a/src/backend/updates.rs
+++ b/src/backend/updates.rs
@@ -1,12 +1,20 @@
-//! The check for updates and how this copy was installed, on the
-//! backend's blocking pool: each is a request to GitHub, or a look at the
-//! disk and the package manager, which a frame never waits for.
+//! The check for updates, how this copy was installed, and the download
+//! and install of an update, on the backend's blocking pool: each is a
+//! request to GitHub, or a look at the disk and the package manager,
+//! which a frame never waits for. A download that verified waits here,
+//! for Restart now: fastframe-update's `Prepared` cannot be saved, so a
+//! quit drops it.
 
 use std::path::PathBuf;
-use std::time::Duration;
+use std::sync::{Arc, Mutex, PoisonError};
+use std::time::{Duration, Instant};
+
+use fastframe_update::{Prepared, Release};
 
 use crate::update::check::Failed;
+use crate::update::download::Failure;
 use crate::update::release::Latest;
+use crate::update::transport::{Download, UreqTransport};
 
 /// What the UI asks of the updater.
 #[derive(Debug)]
@@ -29,6 +37,23 @@ pub enum UpdatesCommand {
         text: String,
         url: String,
     },
+    /// Downloads and verifies `version`, whose release page is `url`,
+    /// dropping any other under way and any held. Told with
+    /// [`UpdatesEvent::Progress`] and [`UpdatesEvent::Waiting`], and
+    /// answered with [`UpdatesEvent::Downloaded`]; a download that verified
+    /// is held for [`UpdatesCommand::Install`].
+    Download {
+        asked: u64,
+        version: String,
+        url: String,
+    },
+    /// Stops the download under way: fastframe-update deletes what it
+    /// wrote. Nothing answers it.
+    CancelDownload,
+    /// Hands the download held to fastframe-update's helper, which
+    /// installs it once the app quits and starts it with `arguments`.
+    /// Answered with [`UpdatesEvent::Handed`].
+    Install { arguments: Vec<String> },
 }
 
 /// What came of an [`UpdatesCommand`].
@@ -44,9 +69,150 @@ pub enum UpdatesEvent {
     NotesUnshown {
         url: String,
     },
+    /// The download `asked` has `received` of `total` bytes, `after` this
+    /// long: told each megabyte and at the end.
+    Progress {
+        asked: u64,
+        received: u64,
+        total: u64,
+        after: Duration,
+    },
+    /// The download `asked` waits for the network, or goes on.
+    Waiting {
+        asked: u64,
+        waiting: bool,
+    },
+    /// The download `asked` verified, and is held, or failed. A cancelled
+    /// one is not answered.
+    Downloaded {
+        asked: u64,
+        result: Result<(), Failure>,
+    },
+    /// The helper stands ready, and the app is to quit now; or it did not
+    /// start, in fastframe-update's words. The download held is gone
+    /// either way.
+    Handed(Result<(), String>),
 }
 
-pub fn handle(command: UpdatesCommand, outbox: &super::Outbox) {
+/// What the backend keeps of updates between commands.
+#[derive(Default)]
+pub struct Updates {
+    /// The download under way, by its ask, and what its body shares.
+    downloading: Option<(u64, Arc<Download>)>,
+    /// The download that verified, until it is installed or another
+    /// starts.
+    held: Arc<Mutex<Option<Prepared>>>,
+}
+
+impl Updates {
+    pub fn handle(&mut self, command: UpdatesCommand, outbox: &super::Outbox) {
+        match command {
+            UpdatesCommand::Download {
+                asked,
+                version,
+                url,
+            } => self.download(asked, Release { version, url }, outbox),
+            // A download that verified as the cancel came is held by
+            // now: it goes too.
+            UpdatesCommand::CancelDownload => {
+                if let Some((_, download)) = self.downloading.take() {
+                    download.cancel();
+                }
+                let held = self
+                    .held
+                    .lock()
+                    .unwrap_or_else(PoisonError::into_inner)
+                    .take();
+                if let Some(held) = held {
+                    tokio::task::spawn_blocking(move || held.discard());
+                }
+            }
+            UpdatesCommand::Install { arguments } => {
+                let held = self
+                    .held
+                    .lock()
+                    .unwrap_or_else(PoisonError::into_inner)
+                    .take();
+                let outbox = outbox.clone();
+                tokio::task::spawn_blocking(move || {
+                    let handed = match held {
+                        Some(prepared) => crate::update::updater()
+                            .handoff(prepared, arguments)
+                            .map_err(|error| format!("{error:#}")),
+                        None => Err("no update is ready to install".to_owned()),
+                    };
+                    if let Err(error) = &handed {
+                        log::error!("could not hand the update over: {error}");
+                    }
+                    outbox.emit(super::Event::Updates(UpdatesEvent::Handed(handed)));
+                });
+            }
+            command => handle(command, outbox),
+        }
+    }
+
+    /// Starts downloading `release` on a thread of the blocking pool,
+    /// in place of any download under way or held.
+    fn download(&mut self, asked: u64, release: Release, outbox: &super::Outbox) {
+        if let Some((_, download)) = self.downloading.take() {
+            download.cancel();
+        }
+        let told = outbox.clone();
+        let download = Arc::new(Download::new(move |waiting| {
+            told.emit(super::Event::Updates(UpdatesEvent::Waiting {
+                asked,
+                waiting,
+            }));
+        }));
+        self.downloading = Some((asked, Arc::clone(&download)));
+        let held = Arc::clone(&self.held);
+        let outbox = outbox.clone();
+        tokio::task::spawn_blocking(move || {
+            // The next download's folder takes the place of this one's.
+            let before = held.lock().unwrap_or_else(PoisonError::into_inner).take();
+            if let Some(before) = before {
+                before.discard();
+            }
+            let updater = fastframe_update::Updater::new(
+                crate::update::CONFIG,
+                UreqTransport::downloading(Arc::clone(&download)),
+            );
+            let started = Instant::now();
+            let result = updater.download(&release, |received, total| {
+                outbox.emit(super::Event::Updates(UpdatesEvent::Progress {
+                    asked,
+                    received,
+                    total,
+                    after: started.elapsed(),
+                }));
+            });
+            let result = match (result, download.cancelled()) {
+                (Ok(prepared), true) => {
+                    prepared.discard();
+                    return;
+                }
+                (Err(error), true) => {
+                    log::info!("the update download stopped: {error:#}");
+                    return;
+                }
+                (Ok(prepared), false) => {
+                    *held.lock().unwrap_or_else(PoisonError::into_inner) = Some(prepared);
+                    Ok(())
+                }
+                (Err(error), false) => {
+                    log::error!("the update download failed: {error:#}");
+                    Err(Failure::of(&error))
+                }
+            };
+            outbox.emit(super::Event::Updates(UpdatesEvent::Downloaded {
+                asked,
+                result,
+            }));
+        });
+    }
+}
+
+fn handle(command: UpdatesCommand, outbox: &super::Outbox) {
     let outbox = outbox.clone();
     match command {
         UpdatesCommand::Check {
@@ -85,6 +251,10 @@ pub fn handle(command: UpdatesCommand, outbox: &super::Outbox) {
                 }
             });
         }
+        // Kept between commands: [`Updates::handle`]'s.
+        UpdatesCommand::Download { .. }
+        | UpdatesCommand::CancelDownload
+        | UpdatesCommand::Install { .. } => {}
     }
 }
 
@@ -179,6 +349,7 @@ mod tests {
         assert!(launched(command, LAUNCH_FAILS_WITHIN).is_err());
     }
 }
+
 #[cfg(test)]
 mod handed {
     use super::*;
```

`src/update.rs`:

```diff
diff --git a/src/update.rs b/src/update.rs
index 7c58f4ff..1baeadc0 100644
--- a/src/update.rs
+++ b/src/update.rs
@@ -1,7 +1,8 @@
 //! Updates from Tabletist's GitHub releases: the check Settings › Updates
 //! shows (`check`, `release`), what it found as the state folder keeps it
 //! (`seen`), how this copy was installed (`installed`), and the updater
-//! that installs one, fastframe-update.
+//! that installs one, fastframe-update, over a client of our own
+//! (`transport`), with what the card makes of its download (`download`).
 //!
 //! What a later release's helper relies on from this build, which must not
 //! change: `tabletist --version` prints `tabletist <version>`;
@@ -12,13 +13,13 @@
 //! checksums) is in `release.yml` and `tests/update_release.rs`.
 
 pub mod check;
+pub mod download;
 pub mod installed;
 pub mod release;
 pub mod seen;
+pub mod transport;
 
-use std::time::Duration;
-
-use fastframe_update::{MacConfig, Request, Response, Transport, UpdateConfig, Updater};
+use fastframe_update::{MacConfig, UpdateConfig, Updater};
 
 pub use check::Failed;
 pub use installed::{Installed, Managed};
@@ -55,7 +56,7 @@ pub const CONFIG: UpdateConfig = UpdateConfig {
 /// An updater, on an HTTP client of its own. Its calls block: run them on
 /// a thread of their own, never on the interface thread.
 pub fn updater() -> Updater {
-    Updater::new(CONFIG, UreqTransport::new())
+    Updater::new(CONFIG, transport::UreqTransport::new())
 }
 
 /// The release page of the version that runs: What's new in it.
@@ -63,52 +64,6 @@ pub fn this_release() -> String {
     format!("https://github.com/igor-alexandrov/tabletist/releases/tag/v{CURRENT}")
 }
 
-/// The updater's HTTP client: one GET at a time, redirects handed back
-/// rather than followed (the updater follows them, to GitHub's hosts
-/// only), any status returned as an answer, 15 seconds to connect and 15
-/// minutes for a whole download, as fastframe-update asks of a transport.
-/// It says what the check says of this copy, and nothing else.
-struct UreqTransport {
-    agent: ureq::Agent,
-    user_agent: String,
-}
-
-impl UreqTransport {
-    fn new() -> Self {
-        let config = ureq::Agent::config_builder()
-            .max_redirects(0)
-            .http_status_as_error(false)
-            .timeout_connect(Some(Duration::from_secs(15)))
-            .timeout_global(Some(Duration::from_secs(15 * 60)))
-            .build();
-        Self {
-            agent: config.into(),
-            user_agent: check::user_agent(CURRENT, check::macos_version().as_deref()),
-        }
-    }
-}
-
-impl Transport for UreqTransport {
-    fn get(&self, request: &Request<'_>) -> anyhow::Result<Response> {
-        let response = self
-            .agent
-            .get(request.url)
-            .header("Accept", request.accept)
-            .header("User-Agent", &self.user_agent)
-            .call()?;
-        let location = response
-            .headers()
-            .get("location")
-            .and_then(|value| value.to_str().ok())
-            .map(str::to_owned);
-        Ok(Response {
-            status: response.status().as_u16(),
-            location,
-            body: Box::new(response.into_body().into_reader()),
-        })
-    }
-}
-
 /// How long `check` waits between scheduled checks; None where checks
 /// are only asked for.
 pub fn interval(check: UpdateCheck) -> Option<u64> {
```

`src/update/download.rs`:

```diff
diff --git a/src/update/download.rs b/src/update/download.rs
new file mode 100644
index 00000000..14d5b04f
--- /dev/null
+++ b/src/update/download.rs
@@ -0,0 +1,447 @@
+//! What the card makes of a download: why one gave nothing to install,
+//! sorted from fastframe-update's words, how long it has left, and the
+//! issue a failure can be reported with.
+
+use std::time::Duration;
+
+/// Why a download gave no update to install.
+#[derive(Clone, Debug, PartialEq, Eq)]
+pub enum Failure {
+    /// Verification failed: the file is not what the release signed. It
+    /// was deleted.
+    Unverified(Unverified),
+    /// Couldn't download: the file never came whole, or could not be
+    /// kept beside the app.
+    Undone(Undone),
+    /// Couldn't install: the helper that swaps the app did not start.
+    /// fastframe-update's words.
+    NotHanded(String),
+}
+
+/// What did not verify.
+#[derive(Clone, Copy, Debug, PartialEq, Eq)]
+pub enum Unverified {
+    /// The signature of the release's checksums.
+    Signature,
+    /// The checksums themselves: missing, twice, or of the wrong size.
+    Checksums,
+    /// The file's own hash.
+    Hash,
+    /// The new version says another version than the release.
+    Version,
+    /// The new version did not start, or not in time.
+    Startup,
+    /// macOS: what was downloaded is no Tabletist app.
+    Bundle,
+    /// macOS: signed by another developer.
+    Publisher,
+    /// macOS: its signature did not check, or Gatekeeper refused it.
+    Approval,
+}
+
+/// Why the file never came whole.
+#[derive(Clone, Debug, PartialEq, Eq)]
+pub enum Undone {
+    /// The connection dropped and did not come back in time.
+    Lost,
+    /// No answer at the start.
+    Unreachable,
+    /// The server answered with this status.
+    Status(u16),
+    /// The folder beside the app takes no files.
+    NotWritable,
+    /// The release is not the one checked.
+    Changed,
+    /// Something else, in fastframe-update's words.
+    Other(String),
+}
+
+impl Failure {
+    /// The failure an error of `Updater::download` says, from its words:
+    /// fastframe-update's errors carry no type.
+    pub fn of(error: &anyhow::Error) -> Self {
+        Self::of_words(&format!("{error:#}"))
+    }
+
+    fn of_words(words: &str) -> Self {
+        let has = |part: &str| words.contains(part);
+        let unverified = if has("signed update metadata size") || has("checksum") {
+            Some(Unverified::Checksums)
+        } else if has("publisher signature") {
+            Some(Unverified::Signature)
+        } else if has("The download couldn't be verified") {
+            Some(Unverified::Hash)
+        } else if has("app bundle") {
+            Some(Unverified::Bundle)
+        } else if has("has the wrong version") {
+            Some(Unverified::Version)
+        } else if has("startup check") || has("cannot run on this computer") {
+            Some(Unverified::Startup)
+        } else if has("different publisher") {
+            Some(Unverified::Publisher)
+        } else if has("could not approve")
+            || has("app signature")
+            || has("signing identity")
+            || has("disk image")
+        {
+            Some(Unverified::Approval)
+        } else {
+            None
+        };
+        if let Some(unverified) = unverified {
+            return Self::Unverified(unverified);
+        }
+        let status = words
+            .split("The update server answered ")
+            .nth(1)
+            .and_then(|rest| {
+                let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
+                digits.parse().ok()
+            });
+        Self::Undone(if let Some(status) = status {
+            Undone::Status(status)
+        } else if has(super::transport::LOST) || has("was interrupted") {
+            Undone::Lost
+        } else if has(super::transport::UNREACHABLE) {
+            Undone::Unreachable
+        } else if has("Cannot write to the installation directory") {
+            Undone::NotWritable
+        } else if has("The release changed") {
+            Undone::Changed
+        } else {
+            Undone::Other(words.to_owned())
+        })
+    }
+
+    /// What the card's box says of it, in English, `{status}` and
+    /// `{error}` put in by [`Failure::with_parts`]: the caller translates
+    /// first. Verification failed's box then says the file was deleted.
+    pub fn said(&self) -> &'static str {
+        match self {
+            Self::Unverified(unverified) => match unverified {
+                Unverified::Signature => "The signature doesn't match Tabletist's release key.",
+                Unverified::Checksums => "The release's checksums couldn't be read.",
+                Unverified::Hash => "The file doesn't match its checksum in the release.",
+                Unverified::Version => "The file holds another version than the release.",
+                Unverified::Startup => "The new version didn't pass its startup check.",
+                Unverified::Bundle => "The download isn't a Tabletist app.",
+                Unverified::Publisher => "The app was signed by another developer.",
+                Unverified::Approval => "macOS couldn't check the app's signature.",
+            },
+            Self::Undone(undone) => match undone {
+                Undone::Lost => "The connection was lost and didn't come back.",
+                Undone::Unreachable => {
+                    "Couldn't reach github.com. Check the connection to the internet."
+                }
+                Undone::Status(_) => "GitHub answered with an error ({status}).",
+                Undone::NotWritable => "Tabletist can't write to the folder it is in.",
+                Undone::Changed => "The release changed. Check for updates again.",
+                Undone::Other(_) => "{error}",
+            },
+            Self::NotHanded(_) => "The installer didn't start: {error}",
+        }
+    }
+
+    /// [`Failure::said`], translated, with its parts in.
+    pub fn with_parts(&self, said: &str) -> String {
+        let status = match self {
+            Self::Undone(Undone::Status(status)) => status.to_string(),
+            _ => String::new(),
+        };
+        let error = match self {
+            Self::Undone(Undone::Other(error)) | Self::NotHanded(error) => error.as_str(),
+            _ => "",
+        };
+        said.replace("{status}", &status).replace("{error}", error)
+    }
+
+    /// Whether the file was refused: Verification failed.
+    pub fn unverified(&self) -> bool {
+        matches!(self, Self::Unverified(_))
+    }
+}
+
+/// How long a download has left at the speed of its last [`WINDOW`],
+/// from `samples` of it (the time since it started, and the bytes it had
+/// then, oldest first), out of `total` bytes. None for its first
+/// [`SETTLING`], before there are two samples, and while nothing came.
+pub fn time_left(samples: &[(Duration, u64)], total: u64) -> Option<Duration> {
+    let &(now, received) = samples.last()?;
+    if now < SETTLING {
+        return None;
+    }
+    // The newest sample at least a window old, else the oldest: progress
+    // comes a megabyte at a time, which a slow network takes longer for.
+    let &(then, before) = samples
+        .iter()
+        .rev()
+        .find(|(at, _)| now.saturating_sub(*at) >= WINDOW)
+        .or(samples.first())?;
+    let took = now.checked_sub(then).filter(|took| !took.is_zero())?;
+    let came = received.checked_sub(before).filter(|came| *came > 0)?;
+    // Seconds and bytes: the precision of f64 is no matter here.
+    #[allow(clippy::cast_precision_loss)]
+    let speed = came as f64 / took.as_secs_f64();
+    #[allow(clippy::cast_precision_loss)]
+    let left = total.saturating_sub(received) as f64 / speed;
+    Some(Duration::from_secs_f64(left.min(1e9)))
+}
+
+/// The time [`time_left`] reads the speed over.
+pub const WINDOW: Duration = Duration::from_secs(5);
+
+/// How long a download runs before [`time_left`] says anything.
+pub const SETTLING: Duration = Duration::from_secs(2);
+
+/// The samples worth keeping for [`time_left`]: those of the last
+/// [`WINDOW`], and the newest before it.
+pub fn keep_window(samples: &mut Vec<(Duration, u64)>) {
+    let Some(&(now, _)) = samples.last() else {
+        return;
+    };
+    let old = samples
+        .iter()
+        .rposition(|(at, _)| now.saturating_sub(*at) >= WINDOW);
+    if let Some(old) = old {
+        samples.drain(..old);
+    }
+}
+
+/// A time left as the card says it, English with `{n}` for the caller to
+/// translate and fill: under a minute to the next 5 seconds, then to the
+/// next minute.
+pub fn left_words(left: Duration) -> (&'static str, u64) {
+    let secs = left.as_secs() + u64::from(left.subsec_nanos() > 0);
+    let fives = secs.div_ceil(5).max(1) * 5;
+    match fives {
+        0..60 => ("about {n} s left", fives),
+        _ => ("about {n} min left", secs.div_ceil(60)),
+    }
+}
+
+/// A new issue on GitHub about `failure`, the download of `version` on
+/// this computer, its title and text filled in: nothing is sent until the
+/// person submits it there.
+pub fn issue_url(version: &str, failure: &Failure, said: &str) -> String {
+    let kind = match failure {
+        Failure::Unverified(_) => "didn't verify",
+        Failure::Undone(_) => "didn't download",
+        Failure::NotHanded(_) => "didn't install",
+    };
+    let title = format!("Update to {version} {kind}");
+    let body = format!(
+        "Tabletist {current} on {os} {arch} could not update to {version}.\n\n{said}\n",
+        current = super::CURRENT,
+        os = std::env::consts::OS,
+        arch = std::env::consts::ARCH,
+    );
+    format!(
+        "https://github.com/igor-alexandrov/tabletist/issues/new?title={}&body={}",
+        encoded(&title),
+        encoded(&body)
+    )
+}
+
+/// `text` as a part of a query: every byte but the unreserved ones
+/// percent-encoded.
+fn encoded(text: &str) -> String {
+    let mut out = String::with_capacity(text.len());
+    for byte in text.bytes() {
+        match byte {
+            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
+                out.push(char::from(byte));
+            }
+            _ => out.push_str(&format!("%{byte:02X}")),
+        }
+    }
+    out
+}
+
+#[cfg(test)]
+mod tests {
+    use super::*;
+
+    fn sorted(words: &str) -> Failure {
+        Failure::of(&anyhow::anyhow!(words.to_owned()))
+    }
+
+    /// Each of fastframe-update's verification failures is Verification
+    /// failed.
+    #[test]
+    fn a_file_that_did_not_verify_is_verification_failed() {
+        for (words, what) in [
+            ("Invalid update publisher signature", Unverified::Signature),
+            (
+                "The update publisher signature could not be verified",
+                Unverified::Signature,
+            ),
+            ("Duplicate checksum for the update", Unverified::Checksums),
+            ("Invalid update checksum", Unverified::Checksums),
+            (
+                "The release is missing the update checksum",
+                Unverified::Checksums,
+            ),
+            ("Invalid signed update metadata size", Unverified::Checksums),
+            (
+                "The download couldn't be verified. Try downloading it again.",
+                Unverified::Hash,
+            ),
+            (
+                "The downloaded app has the wrong version",
+                Unverified::Version,
+            ),
+            (
+                "The downloaded app cannot run on this computer",
+                Unverified::Startup,
+            ),
+            (
+                "The downloaded app failed its startup check",
+                Unverified::Startup,
+            ),
+            (
+                "The downloaded app did not answer its startup check",
+                Unverified::Startup,
+            ),
+            (
+                "The download is not a Tabletist app bundle",
+                Unverified::Bundle,
+            ),
+            ("The app bundle has the wrong version", Unverified::Bundle),
+            (
+                "The update was signed by a different publisher",
+                Unverified::Publisher,
+            ),
+            (
+                "macOS could not approve this update for launch",
+                Unverified::Approval,
+            ),
+            (
+                "The app signature could not be verified",
+                Unverified::Approval,
+            ),
+            ("Cannot read the app signing identity", Unverified::Approval),
+            (
+                "macOS could not open the downloaded disk image",
+                Unverified::Approval,
+            ),
+        ] {
+            assert_eq!(sorted(words), Failure::Unverified(what), "{words}");
+            assert!(sorted(words).unverified(), "{words}");
+        }
+    }
+
+    /// A network that went, a server's error or a folder that takes no
+    /// files is Couldn't download: the file was never bad.
+    #[test]
+    fn a_file_that_never_came_whole_is_couldnt_download() {
+        for (words, why) in [
+            (super::super::transport::LOST, Undone::Lost),
+            ("The update download was interrupted", Undone::Lost),
+            ("The update server answered 503", Undone::Status(503)),
+            (
+                "The update server could not be reached: io: Connection refused",
+                Undone::Unreachable,
+            ),
+            (
+                "Cannot write to the installation directory: Permission denied",
+                Undone::NotWritable,
+            ),
+            (
+                "The release changed. Check for updates again.",
+                Undone::Changed,
+            ),
+            (
+                "Update redirect is not allowed",
+                Undone::Other("Update redirect is not allowed".into()),
+            ),
+        ] {
+            assert_eq!(sorted(words), Failure::Undone(why), "{words}");
+            assert!(!sorted(words).unverified(), "{words}");
+        }
+    }
+
+    /// The error's whole chain is read: the crate wraps an I/O error of
+    /// the body in context of its own.
+    #[test]
+    fn the_words_are_read_down_the_chain() {
+        let error = anyhow::Error::new(std::io::Error::other(super::super::transport::LOST))
+            .context("Could not download the update");
+        assert_eq!(Failure::of(&error), Failure::Undone(Undone::Lost));
+    }
+
+    #[test]
+    fn what_a_failure_says_has_its_parts() {
+        let status = Failure::Undone(Undone::Status(502));
+        assert_eq!(
+            status.with_parts(status.said()),
+            "GitHub answered with an error (502)."
+        );
+        let other = Failure::Undone(Undone::Other("Update redirect is not allowed".into()));
+        assert_eq!(
+            other.with_parts(other.said()),
+            "Update redirect is not allowed"
+        );
+    }
+
+    const MB: u64 = 1_000_000;
+
+    fn secs(secs: u64) -> Duration {
+        Duration::from_secs(secs)
+    }
+
+    #[test]
+    fn the_time_left_is_read_from_the_last_five_seconds() {
+        // 1 MB a second for 10 s, then 2 MB a second for 5.
+        let mut samples: Vec<(Duration, u64)> = (0..=10).map(|at| (secs(at), at * MB)).collect();
+        samples.extend((11..=15).map(|at| (secs(at), 10 * MB + (at - 10) * 2 * MB)));
+        let left = time_left(&samples, 40 * MB).unwrap();
+        // 20 MB to go at 2 MB a second.
+        assert_eq!(left.as_secs(), 10);
+        // Not in the first two seconds.
+        assert_eq!(time_left(&[(secs(0), 0), (secs(1), 3 * MB)], 40 * MB), None);
+        // Nothing came: no telling.
+        assert_eq!(time_left(&[(secs(0), 0), (secs(4), 0)], 40 * MB), None);
+    }
+
+    #[test]
+    fn a_slow_download_reads_its_speed_from_the_sample_before_the_window() {
+        // A megabyte every 8 s.
+        let samples = [(secs(0), 0), (secs(8), MB), (secs(16), 2 * MB)];
+        let left = time_left(&samples, 4 * MB).unwrap();
+        assert_eq!(left.as_secs(), 16);
+    }
+
+    #[test]
+    fn only_the_samples_of_the_window_and_one_before_are_kept() {
+        let mut samples: Vec<(Duration, u64)> = (0..=10).map(|at| (secs(at), at * MB)).collect();
+        keep_window(&mut samples);
+        assert_eq!(samples.first(), Some(&(secs(5), 5 * MB)));
+        assert_eq!(samples.len(), 6);
+    }
+
+    #[test]
+    fn the_time_left_is_said_to_five_seconds_then_to_the_minute() {
+        assert_eq!(
+            left_words(Duration::from_millis(400)),
+            ("about {n} s left", 5)
+        );
+        assert_eq!(left_words(secs(18)), ("about {n} s left", 20));
+        assert_eq!(left_words(secs(20)), ("about {n} s left", 20));
+        assert_eq!(left_words(secs(54)), ("about {n} s left", 55));
+        assert_eq!(left_words(secs(59)), ("about {n} min left", 1));
+        assert_eq!(left_words(secs(61)), ("about {n} min left", 2));
+    }
+
+    #[test]
+    fn the_issue_names_the_versions_the_system_and_the_error() {
+        let failure = Failure::Unverified(Unverified::Signature);
+        let url = issue_url("99.0.0", &failure, failure.said());
+        assert!(
+            url.starts_with("https://github.com/igor-alexandrov/tabletist/issues/new?title=Update%20to%2099.0.0%20didn%27t%20verify&body="),
+            "{url}"
+        );
+        assert!(url.contains("signature%20doesn%27t%20match"), "{url}");
+        assert!(url.contains(&encoded(std::env::consts::OS)), "{url}");
+        assert!(!url.contains(' '), "{url}");
+    }
+}
```

`src/update/transport.rs`:

```diff
diff --git a/src/update/transport.rs b/src/update/transport.rs
new file mode 100644
index 00000000..e21c8ea9
--- /dev/null
+++ b/src/update/transport.rs
@@ -0,0 +1,1225 @@
+//! The updater's HTTP client: one GET at a time, as fastframe-update asks
+//! of a transport. For a download it waits while the network is away,
+//! from the first request on; its bodies go on where they stopped when the
+//! connection drops or goes quiet, with `Range`; and everything fails at
+//! once when the download is cancelled. fastframe-update has none of
+//! that, and to it the reconnects are one unbroken stream, whose whole
+//! hash it still checks.
+
+use std::io::{self, Read};
+use std::sync::atomic::{AtomicBool, Ordering};
+use std::sync::{Arc, Mutex, PoisonError, mpsc};
+use std::time::{Duration, Instant};
+
+use fastframe_update::{Request, Response, Transport};
+
+/// What the body says when the connection did not come back in time: the
+/// app says Couldn't download (`download::Failure`).
+pub const LOST: &str = "The download's connection was lost and didn't come back";
+
+/// What a request says that got no answer: no network, or no server.
+pub const UNREACHABLE: &str = "The update server could not be reached";
+
+/// What the body says once the download is cancelled.
+pub const CANCELLED: &str = "The download was cancelled";
+
+/// How often a dropped download connects again, and for how long, and
+/// how long a connection may go quiet before it counts as dropped.
+#[derive(Clone, Copy, Debug, PartialEq, Eq)]
+pub struct Patience {
+    pub retry: Duration,
+    pub give_up: Duration,
+    pub quiet: Duration,
+}
+
+/// Every 2 seconds, for up to 10 minutes; a connection quiet for 8
+/// seconds has lost its network, as when Wi-Fi goes without the socket
+/// closing.
+pub const PATIENCE: Patience = Patience {
+    retry: Duration::from_secs(2),
+    give_up: Duration::from_secs(10 * 60),
+    quiet: Duration::from_secs(8),
+};
+
+/// How often a body waiting for its connection looks at its cancel.
+const TICK: Duration = Duration::from_millis(50);
+
+/// What a download shares with whoever started it: its cancel, and
+/// whether it waits for the network, which it tells as it changes.
+pub struct Download {
+    cancelled: AtomicBool,
+    waiting: AtomicBool,
+    told: Box<dyn Fn(bool) + Send + Sync>,
+    patience: Patience,
+}
+
+impl std::fmt::Debug for Download {
+    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
+        formatter
+            .debug_struct("Download")
+            .field("cancelled", &self.cancelled())
+            .field("waiting", &self.waiting())
+            .field("patience", &self.patience)
+            .finish_non_exhaustive()
+    }
+}
+
+impl Download {
+    /// A download that calls `told` each time it starts or stops waiting.
+    pub fn new(told: impl Fn(bool) + Send + Sync + 'static) -> Self {
+        Self {
+            cancelled: AtomicBool::new(false),
+            waiting: AtomicBool::new(false),
+            told: Box::new(told),
+            patience: PATIENCE,
+        }
+    }
+
+    /// The same, connecting again by `patience`: a test's is short.
+    pub fn with_patience(mut self, patience: Patience) -> Self {
+        self.patience = patience;
+        self
+    }
+
+    /// Stops it: the body's next read fails, and fastframe-update deletes
+    /// what it wrote.
+    pub fn cancel(&self) {
+        self.cancelled.store(true, Ordering::SeqCst);
+    }
+
+    pub fn cancelled(&self) -> bool {
+        self.cancelled.load(Ordering::SeqCst)
+    }
+
+    /// Whether it waits for the network to come back.
+    pub fn waiting(&self) -> bool {
+        self.waiting.load(Ordering::SeqCst)
+    }
+
+    fn wait(&self, waiting: bool) {
+        if self.waiting.swap(waiting, Ordering::SeqCst) != waiting {
+            (self.told)(waiting);
+        }
+    }
+
+    fn stop_if_cancelled(&self) -> io::Result<()> {
+        match self.cancelled() {
+            true => Err(io::Error::other(CANCELLED)),
+            false => Ok(()),
+        }
+    }
+
+    /// Sleeps `time`, a slice at a time, and stops early once cancelled.
+    fn pause(&self, time: Duration) -> io::Result<()> {
+        let until = Instant::now() + time;
+        loop {
+            self.stop_if_cancelled()?;
+            let left = until.saturating_duration_since(Instant::now());
+            if left.is_zero() {
+                return Ok(());
+            }
+            std::thread::sleep(left.min(Duration::from_millis(50)));
+        }
+    }
+}
+
+/// The client itself: the agent, and what it says of this copy.
+struct Client {
+    agent: ureq::Agent,
+    user_agent: String,
+}
+
+/// A redirect the updater was given: where its chain of redirects began,
+/// and the hosts on the way, which the updater allowed. The updater asks
+/// for where it leads next, in its own form of the address.
+#[derive(Clone, Debug)]
+struct Chain {
+    start: String,
+    hosts: Vec<String>,
+}
+
+/// The updater's HTTP client: redirects handed back rather than followed
+/// (the updater follows them, to GitHub's hosts only), any status returned
+/// as an answer, 15 seconds to connect and 15 minutes for a whole answer,
+/// as fastframe-update asks of a transport. It says what the check says of
+/// this copy, and nothing else.
+pub struct UreqTransport {
+    client: Arc<Client>,
+    /// The download it is for, where it is for one: its bodies resume.
+    download: Option<Arc<Download>>,
+    /// The last redirect handed back.
+    chain: Mutex<Option<Chain>>,
+}
+
+impl UreqTransport {
+    /// For the updater's calls that download nothing: a body that drops
+    /// is an error, as for any client.
+    pub fn new() -> Self {
+        let user_agent =
+            super::check::user_agent(super::CURRENT, super::check::macos_version().as_deref());
+        Self::with_user_agent(user_agent, None)
+    }
+
+    /// For `download`: its bodies go on where they stopped.
+    pub fn downloading(download: Arc<Download>) -> Self {
+        let user_agent =
+            super::check::user_agent(super::CURRENT, super::check::macos_version().as_deref());
+        Self::with_user_agent(user_agent, Some(download))
+    }
+
+    fn with_user_agent(user_agent: String, download: Option<Arc<Download>>) -> Self {
+        let config = ureq::Agent::config_builder()
+            .max_redirects(0)
+            .http_status_as_error(false)
+            .timeout_connect(Some(Duration::from_secs(15)))
+            .timeout_recv_response(Some(Duration::from_secs(30)))
+            .timeout_global(Some(Duration::from_secs(15 * 60)))
+            .build();
+        Self {
+            client: Arc::new(Client {
+                agent: config.into(),
+                user_agent,
+            }),
+            download,
+            chain: Mutex::new(None),
+        }
+    }
+}
+
+impl Default for UreqTransport {
+    fn default() -> Self {
+        Self::new()
+    }
+}
+
+impl UreqTransport {
+    /// Sends `request`. For a download, no answer is waited out: asked
+    /// again every `retry`, saying it waits, until `give_up` has passed.
+    fn call(&self, request: &Request<'_>) -> anyhow::Result<ureq::http::Response<ureq::Body>> {
+        let since = Instant::now();
+        loop {
+            let answer = self
+                .client
+                .agent
+                .get(request.url)
+                .header("Accept", request.accept)
+                .header("User-Agent", &self.client.user_agent)
+                .call();
+            let Some(download) = &self.download else {
+                return answer.map_err(|error| anyhow::Error::new(error).context(UNREACHABLE));
+            };
+            match answer {
+                Ok(response) => {
+                    download.wait(false);
+                    return Ok(response);
+                }
+                Err(error) if since.elapsed() >= download.patience.give_up => {
+                    download.wait(false);
+                    return Err(anyhow::Error::new(error).context(UNREACHABLE));
+                }
+                Err(error) => {
+                    log::info!("the update server did not answer: {error}");
+                    download.wait(true);
+                    download.pause(download.patience.retry)?;
+                }
+            }
+        }
+    }
+}
+
+impl Transport for UreqTransport {
+    fn get(&self, request: &Request<'_>) -> anyhow::Result<Response> {
+        let response = self.call(request)?;
+        let status = response.status().as_u16();
+        let location = header(&response, "location");
+        // Where this request's chain began: a reconnect starts there, for
+        // a redirect's address may be signed for a few minutes only. The
+        // updater follows a redirect with its very next request.
+        let mut chain = self.chain.lock().unwrap_or_else(PoisonError::into_inner);
+        let (start, mut hosts) = match chain.take() {
+            Some(chain) => (chain.start, chain.hosts),
+            None => (request.url.to_owned(), Vec::new()),
+        };
+        hosts.extend(host(request.url));
+        if (300..400).contains(&status)
+            && let Some(location) = &location
+        {
+            *chain = Some(Chain { start, hosts });
+            drop(chain);
+            return Ok(Response {
+                status,
+                location: Some(location.clone()),
+                body: Box::new(io::empty()),
+            });
+        }
+        drop(chain);
+        let download = match (&self.download, status) {
+            (Some(download), 200..=299) => Arc::clone(download),
+            _ => {
+                return Ok(Response {
+                    status,
+                    location,
+                    body: Box::new(response.into_body().into_reader()),
+                });
+            }
+        };
+        let length = response.body().content_length();
+        let body = Body {
+            pieces: Some(pump(Box::new(response.into_body().into_reader()))?),
+            held: Vec::new(),
+            skip: 0,
+            read: 0,
+            length,
+            heard: Instant::now(),
+            lost: None,
+            tries: 0,
+            again: Again {
+                client: Arc::clone(&self.client),
+                url: start,
+                accept: request.accept.to_owned(),
+                hosts,
+            },
+            download,
+        };
+        Ok(Response {
+            status,
+            location: None,
+            body: Box::new(body),
+        })
+    }
+}
+
+/// A header's value, where it is text.
+fn header<B>(response: &ureq::http::Response<B>, name: &str) -> Option<String> {
+    response
+        .headers()
+        .get(name)
+        .and_then(|value| value.to_str().ok())
+        .map(str::to_owned)
+}
+
+/// The host of `url`, with its port: `github.com`, `127.0.0.1:8123`.
+fn host(url: &str) -> Option<String> {
+    let (_, rest) = url.split_once("://")?;
+    let end = rest.find(['/', '?', '#']).unwrap_or(rest.len());
+    Some(rest[..end].to_ascii_lowercase())
+}
+
+/// Whether a reconnect may follow a redirect to `url`: to a host the
+/// updater's own redirects went through, over HTTPS (plain HTTP on this
+/// computer only, as for a local feed), with no user in it.
+fn may_follow(url: &str, hosts: &[String]) -> bool {
+    let Some(host) = host(url) else {
+        return false;
+    };
+    let secure = match url.split_once("://").map(|(scheme, _)| scheme) {
+        Some(scheme) if scheme.eq_ignore_ascii_case("https") => true,
+        Some(scheme) if scheme.eq_ignore_ascii_case("http") => {
+            host.starts_with("127.0.0.1") || host.starts_with("[::1]")
+        }
+        _ => false,
+    };
+    secure && !host.contains('@') && hosts.contains(&host)
+}
+
+/// `location` as an address of its own, where it is one relative to
+/// `base`: GitHub's are absolute, a server's may not be.
+fn absolute(base: &str, location: &str) -> String {
+    if location.contains("://") {
+        return location.to_owned();
+    }
+    let scheme = base.split_once("://").map_or("https", |(scheme, _)| scheme);
+    if let Some(rest) = location.strip_prefix("//") {
+        return format!("{scheme}://{rest}");
+    }
+    let origin = match host(base) {
+        Some(host) => format!("{scheme}://{host}"),
+        None => return location.to_owned(),
+    };
+    if location.starts_with('/') {
+        return format!("{origin}{location}");
+    }
+    let path = &base[origin.len().min(base.len())..];
+    let path = path.split(['?', '#']).next().unwrap_or_default();
+    let folder = path.rfind('/').map_or("/", |at| &path[..=at]);
+    format!("{origin}{folder}{location}")
+}
+
+/// How a download asks again for the rest of its file.
+struct Again {
+    client: Arc<Client>,
+    /// Where the chain of redirects began.
+    url: String,
+    accept: String,
+    /// The hosts its redirects may go to: those the updater allowed.
+    hosts: Vec<String>,
+}
+
+/// Why asking again gave nothing to read.
+enum Missed {
+    /// No answer: the network is away, or the server is. Asked again.
+    Offline(io::Error),
+    /// An answer that is no part of the file: asking again would get the
+    /// same.
+    Refused(io::Error),
+}
+
+impl Again {
+    /// Asks for the file from byte `from` on, following redirects to the
+    /// hosts the updater allowed. Gives the answer's body, and how many of
+    /// its bytes were read already: a server that ignores the range sends
+    /// the whole file again.
+    fn ask(&self, from: u64) -> Result<(Box<dyn Read + Send>, u64), Missed> {
+        let refused = |text: String| Missed::Refused(io::Error::other(text));
+        let mut url = self.url.clone();
+        for _ in 0..=5 {
+            let response = self
+                .client
+                .agent
+                .get(&url)
+                .header("Accept", &self.accept)
+                .header("User-Agent", &self.client.user_agent)
+                .header("Range", &format!("bytes={from}-"))
+                .call()
+                .map_err(|error| Missed::Offline(io::Error::other(error)))?;
+            let status = response.status().as_u16();
+            match status {
+                206 => {
+                    let starts = header(&response, "content-range")
+                        .and_then(|range| range_start(&range))
+                        .unwrap_or_default();
+                    if starts != from {
+                        return Err(refused(format!(
+                            "The update server sent the download from byte {starts}, not {from}"
+                        )));
+                    }
+                    return Ok((Box::new(response.into_body().into_reader()), 0));
+                }
+                200 => return Ok((Box::new(response.into_body().into_reader()), from)),
+                300..=399 => {
+                    let Some(location) = header(&response, "location") else {
+                        return Err(refused(
+                            "The update server sent a redirect without a location".into(),
+                        ));
+                    };
+                    let next = absolute(&url, &location);
+                    if !may_follow(&next, &self.hosts) {
+                        return Err(refused("Update redirect is not allowed".into()));
+                    }
+                    url = next;
+                }
+                // The same words as fastframe-update's.
+                status => return Err(refused(format!("The update server answered {status}"))),
+            }
+        }
+        Err(refused(
+            "The update server redirected too many times".into(),
+        ))
+    }
+}
+
+/// Where `bytes 18400-31199/31200` starts.
+fn range_start(range: &str) -> Option<u64> {
+    let range = range.trim().strip_prefix("bytes")?.trim_start();
+    let (start, _) = range.split_once('-')?;
+    start.trim().parse().ok()
+}
+
+/// A piece of a connection's body as its pump read it: empty at its end.
+type Piece = io::Result<Vec<u8>>;
+
+/// Reads `reader` on a thread of its own into a short queue, so that a
+/// body never blocks on a connection gone quiet. The thread ends with the
+/// reader, or once the body stops listening: a connection left quiet ends
+/// it at ureq's own timeout.
+fn pump(mut reader: Box<dyn Read + Send>) -> io::Result<mpsc::Receiver<Piece>> {
+    let (send, pieces) = mpsc::sync_channel(4);
+    std::thread::Builder::new()
+        .name("tabletist-update-body".into())
+        .spawn(move || {
+            let mut buffer = vec![0; 64 * 1024];
+            loop {
+                let piece = match reader.read(&mut buffer) {
+                    Ok(read) => Ok(buffer[..read].to_vec()),
+                    Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
+                    Err(error) => Err(error),
+                };
+                let more = matches!(&piece, Ok(bytes) if !bytes.is_empty());
+                if send.send(piece).is_err() || !more {
+                    return;
+                }
+            }
+        })?;
+    Ok(pieces)
+}
+
+/// A download's body: what the first answer sends, then, where its
+/// connection drops or goes quiet, the rest of the file from where it
+/// stopped.
+struct Body {
+    /// The connection's body, as its pump reads it.
+    pieces: Option<mpsc::Receiver<Piece>>,
+    /// What came and was not handed on yet.
+    held: Vec<u8>,
+    /// How many bytes the connection sends again before the rest: a
+    /// server that ignored the range.
+    skip: u64,
+    /// The bytes handed on so far.
+    read: u64,
+    /// The whole file's size, where the first answer said it.
+    length: Option<u64>,
+    /// When the connection last sent anything.
+    heard: Instant,
+    /// Since when no new byte of the file has come: from a drop until
+    /// bytes come again. The download gives up `give_up` after it.
+    lost: Option<Instant>,
+    /// The reconnects since new bytes last came: each after the first
+    /// waits `retry` first.
+    tries: u32,
+    again: Again,
+    download: Arc<Download>,
+}
+
+impl Read for Body {
+    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
+        loop {
+            self.download.stop_if_cancelled()?;
+            if !self.held.is_empty() {
+                let count = buffer.len().min(self.held.len());
+                buffer[..count].copy_from_slice(&self.held[..count]);
+                self.held.drain(..count);
+                self.read += count as u64;
+                return Ok(count);
+            }
+            let Some(pieces) = &self.pieces else {
+                self.resume()?;
+                continue;
+            };
+            match pieces.recv_timeout(TICK) {
+                // Short of the size it said: the connection closed early.
+                Ok(Ok(bytes)) if bytes.is_empty() => {
+                    if !self.length.is_some_and(|length| self.read < length) {
+                        return Ok(0);
+                    }
+                    self.dropped("it closed early");
+                }
+                Ok(Ok(bytes)) => self.came(&bytes),
+                Ok(Err(error)) => self.dropped(&error.to_string()),
+                Err(mpsc::RecvTimeoutError::Timeout) => {
+                    if self.heard.elapsed() >= self.download.patience.quiet {
+                        self.dropped("it went quiet");
+                        self.download.wait(true);
+                    }
+                }
+                Err(mpsc::RecvTimeoutError::Disconnected) => self.dropped("its reader ended"),
+            }
+        }
+    }
+}
+
+impl Body {
+    /// Bytes the connection sent: what it sent again is dropped, and the
+    /// rest held to hand on. New bytes end a wait.
+    fn came(&mut self, bytes: &[u8]) {
+        self.heard = Instant::now();
+        let again = usize::try_from(self.skip)
+            .unwrap_or(usize::MAX)
+            .min(bytes.len());
+        self.skip -= again as u64;
+        if again < bytes.len() {
+            self.held.extend_from_slice(&bytes[again..]);
+            self.lost = None;
+            self.tries = 0;
+            self.download.wait(false);
+        }
+    }
+
+    /// The connection is gone, for `why`: the next read asks again.
+    fn dropped(&mut self, why: &str) {
+        log::info!(
+            "the update download dropped after {} bytes: {why}",
+            self.read
+        );
+        self.pieces = None;
+        self.lost.get_or_insert_with(Instant::now);
+    }
+
+    /// Asks for the rest: at once after bytes came, otherwise after
+    /// `retry`, waiting meanwhile, until `give_up` has passed since the
+    /// file last came.
+    fn resume(&mut self) -> io::Result<()> {
+        let patience = self.download.patience;
+        loop {
+            self.download.stop_if_cancelled()?;
+            let lost = *self.lost.get_or_insert_with(Instant::now);
+            if lost.elapsed() >= patience.give_up {
+                self.download.wait(false);
+                log::info!("the update download gave up waiting");
+                return Err(io::Error::other(LOST));
+            }
+            if self.tries > 0 {
+                self.download.wait(true);
+                self.download.pause(patience.retry)?;
+            }
+            self.tries += 1;
+            match self.again.ask(self.read) {
+                Ok((reader, skip)) => {
+                    self.pieces = Some(pump(reader)?);
+                    self.skip = skip;
+                    self.heard = Instant::now();
+                    return Ok(());
+                }
+                Err(Missed::Refused(error)) => {
+                    self.download.wait(false);
+                    return Err(error);
+                }
+                Err(Missed::Offline(error)) => {
+                    log::info!("the update server did not answer: {error}");
+                }
+            }
+        }
+    }
+}
+
+#[cfg(test)]
+mod tests {
+    use std::io::{BufRead, BufReader, Write};
+    use std::net::{TcpListener, TcpStream};
+    use std::sync::mpsc;
+
+    use super::*;
+
+    /// The file the fake server serves: a release's bytes, each its place.
+    fn file() -> Vec<u8> {
+        (0..200_000u32).map(|at| (at % 251) as u8).collect()
+    }
+
+    /// What a request asked: its request line and its headers, lower case.
+    #[derive(Clone, Debug)]
+    struct Asked {
+        line: String,
+        headers: Vec<(String, String)>,
+    }
+
+    impl Asked {
+        fn range(&self) -> Option<u64> {
+            self.headers
+                .iter()
+                .find(|(name, _)| name == "range")
+                .and_then(|(_, value)| value.strip_prefix("bytes="))
+                .and_then(|value| value.trim_end_matches('-').parse().ok())
+        }
+    }
+
+    fn read_request(stream: &TcpStream) -> Asked {
+        let mut reader = BufReader::new(stream.try_clone().unwrap());
+        let mut line = String::new();
+        reader.read_line(&mut line).unwrap();
+        let mut headers = Vec::new();
+        loop {
+            let mut header = String::new();
+            reader.read_line(&mut header).unwrap();
+            if header == "\r\n" || header.is_empty() {
+                break;
+            }
+            if let Some((name, value)) = header.trim_end().split_once(':') {
+                headers.push((name.to_ascii_lowercase(), value.trim().to_owned()));
+            }
+        }
+        Asked {
+            line: line.trim_end().to_owned(),
+            headers,
+        }
+    }
+
+    /// How the fake server answers one connection.
+    enum Answer {
+        /// The file from `from` on, as a 206 where `ranged`, closed after
+        /// `send` bytes where it is given.
+        File {
+            from: u64,
+            send: Option<usize>,
+            ranged: bool,
+        },
+        /// A connection closed with no answer: the network is away.
+        Nothing,
+        /// The file slowly, a chunk at a time, until the client stops
+        /// reading.
+        Slowly,
+    }
+
+    /// Serves `answers`, one connection each, on a local port, and gives
+    /// back the address and what each request asked.
+    fn serve(answers: Vec<Answer>) -> (String, mpsc::Receiver<Asked>) {
+        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
+        let url = format!(
+            "http://{}/download/v99.0.0/tabletist.tar.gz",
+            listener.local_addr().unwrap()
+        );
+        let (asked, heard) = mpsc::channel();
+        std::thread::spawn(move || {
+            let file = file();
+            for answer in answers {
+                let Ok((mut stream, _)) = listener.accept() else {
+                    return;
+                };
+                let request = read_request(&stream);
+                let _ = asked.send(request);
+                match answer {
+                    Answer::Nothing => drop(stream),
+                    Answer::File { from, send, ranged } => {
+                        let from = usize::try_from(from).unwrap();
+                        let rest = &file[from..];
+                        let head = match ranged {
+                            true => format!(
+                                "HTTP/1.1 206 Partial Content\r\nContent-Length: {}\r\nContent-Range: bytes {from}-{}/{}\r\nConnection: close\r\n\r\n",
+                                rest.len(),
+                                file.len() - 1,
+                                file.len()
+                            ),
+                            false => format!(
+                                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
+                                rest.len()
+                            ),
+                        };
+                        let _ = stream.write_all(head.as_bytes());
+                        let sent = send.unwrap_or(rest.len()).min(rest.len());
+                        let _ = stream.write_all(&rest[..sent]);
+                        let _ = stream.flush();
+                        // A cut answer: closed short of its length.
+                        let _ = stream.shutdown(std::net::Shutdown::Both);
+                    }
+                    Answer::Slowly => {
+                        let head = format!(
+                            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
+                            file.len()
+                        );
+                        let _ = stream.write_all(head.as_bytes());
+                        for chunk in file.chunks(1_000) {
+                            if stream.write_all(chunk).is_err() {
+                                break;
+                            }
+                            std::thread::sleep(Duration::from_millis(20));
+                        }
+                    }
+                }
+            }
+        });
+        (url, heard)
+    }
+
+    const QUICK: Patience = Patience {
+        retry: Duration::from_millis(50),
+        give_up: Duration::from_secs(20),
+        quiet: Duration::from_secs(20),
+    };
+
+    /// A download's transport, and each time it said it waited or not.
+    fn downloading(patience: Patience) -> (UreqTransport, Arc<Download>, mpsc::Receiver<bool>) {
+        let (told, heard) = mpsc::channel();
+        let download = Download::new(move |waiting| {
+            let _ = told.send(waiting);
+        })
+        .with_patience(patience);
+        let download = Arc::new(download);
+        let transport = UreqTransport::with_user_agent(
+            "Tabletist/0.3.0 (Linux; x86_64)".into(),
+            Some(Arc::clone(&download)),
+        );
+        (transport, download, heard)
+    }
+
+    fn get(transport: &UreqTransport, url: &str) -> Response {
+        transport
+            .get(&Request {
+                url,
+                accept: "application/octet-stream",
+                user_agent: "ignored",
+            })
+            .unwrap()
+    }
+
+    fn read_all(response: Response) -> io::Result<Vec<u8>> {
+        let mut body = response.body;
+        let mut read = Vec::new();
+        body.read_to_end(&mut read)?;
+        Ok(read)
+    }
+
+    #[test]
+    fn a_download_resumes_where_its_connection_dropped() {
+        let (url, asked) = serve(vec![
+            Answer::File {
+                from: 0,
+                send: Some(70_000),
+                ranged: false,
+            },
+            Answer::File {
+                from: 70_000,
+                send: None,
+                ranged: true,
+            },
+        ]);
+        let (transport, download, waited) = downloading(QUICK);
+        let read = read_all(get(&transport, &url)).unwrap();
+        assert_eq!(read.len(), file().len());
+        assert!(read == file(), "the bytes differ");
+        let first = asked.recv().unwrap();
+        assert_eq!(first.range(), None);
+        let again = asked.recv().unwrap();
+        assert_eq!(again.range(), Some(70_000), "{again:?}");
+        assert!(
+            again.line.starts_with("GET /download/v99.0.0/"),
+            "{again:?}"
+        );
+        // The client says only what the first request said, and the range.
+        let mut names: Vec<&str> = again
+            .headers
+            .iter()
+            .map(|(name, _)| name.as_str())
+            .collect();
+        names.sort_unstable();
+        assert_eq!(names, ["accept", "host", "range", "user-agent"]);
+        // Back at once: it never waited.
+        assert!(waited.try_recv().is_err());
+        assert!(!download.waiting());
+    }
+
+    #[test]
+    fn a_download_waits_while_the_network_is_away_then_goes_on() {
+        let (url, _asked) = serve(vec![
+            Answer::File {
+                from: 0,
+                send: Some(50_000),
+                ranged: false,
+            },
+            Answer::Nothing,
+            Answer::Nothing,
+            Answer::Nothing,
+            Answer::File {
+                from: 50_000,
+                send: None,
+                ranged: true,
+            },
+        ]);
+        let (transport, download, waited) = downloading(QUICK);
+        let read = read_all(get(&transport, &url)).unwrap();
+        assert!(read == file(), "the bytes differ");
+        // It said it waited, then that it went on.
+        assert_eq!(waited.try_iter().collect::<Vec<_>>(), [true, false]);
+        assert!(!download.waiting());
+    }
+
+    #[test]
+    fn a_download_that_does_not_come_back_in_time_is_lost() {
+        let (url, _asked) = serve(vec![
+            Answer::File {
+                from: 0,
+                send: Some(50_000),
+                ranged: false,
+            },
+            Answer::Nothing,
+            Answer::Nothing,
+            Answer::Nothing,
+            Answer::Nothing,
+            Answer::Nothing,
+            Answer::Nothing,
+        ]);
+        let patience = Patience {
+            retry: Duration::from_millis(20),
+            give_up: Duration::from_millis(100),
+            quiet: Duration::from_secs(20),
+        };
+        let (transport, download, _waited) = downloading(patience);
+        let error = read_all(get(&transport, &url)).unwrap_err();
+        assert_eq!(error.to_string(), LOST);
+        assert!(!download.waiting());
+    }
+
+    #[test]
+    fn a_server_that_ignores_the_range_has_what_was_read_skipped() {
+        let (url, asked) = serve(vec![
+            Answer::File {
+                from: 0,
+                send: Some(120_000),
+                ranged: false,
+            },
+            // The whole file, as a 200.
+            Answer::File {
+                from: 0,
+                send: None,
+                ranged: false,
+            },
+        ]);
+        let (transport, _download, _waited) = downloading(QUICK);
+        let read = read_all(get(&transport, &url)).unwrap();
+        assert_eq!(read.len(), file().len());
+        assert!(read == file(), "the bytes differ");
+        let _ = asked.recv().unwrap();
+        assert_eq!(asked.recv().unwrap().range(), Some(120_000));
+    }
+
+    #[test]
+    fn a_cancel_fails_the_next_read() {
+        let (url, _asked) = serve(vec![Answer::Slowly]);
+        let (transport, download, _waited) = downloading(QUICK);
+        let mut body = get(&transport, &url).body;
+        let mut buffer = [0; 512];
+        assert!(body.read(&mut buffer).unwrap() > 0);
+        download.cancel();
+        let error = body.read(&mut buffer).unwrap_err();
+        assert_eq!(error.to_string(), CANCELLED);
+    }
+
+    /// While it waits for the network, a cancel ends the wait.
+    #[test]
+    fn a_cancel_ends_the_wait_for_the_network() {
+        let (url, _asked) = serve(vec![
+            Answer::File {
+                from: 0,
+                send: Some(10_000),
+                ranged: false,
+            },
+            Answer::Nothing,
+            Answer::Nothing,
+            Answer::Nothing,
+            Answer::Nothing,
+            Answer::Nothing,
+            Answer::Nothing,
+            Answer::Nothing,
+            Answer::Nothing,
+        ]);
+        let patience = Patience {
+            retry: Duration::from_secs(5),
+            give_up: Duration::from_secs(60),
+            quiet: Duration::from_secs(20),
+        };
+        let (transport, download, waited) = downloading(patience);
+        let body = get(&transport, &url).body;
+        let reading = std::thread::spawn(move || read_all(Response::ok(body)));
+        assert_eq!(waited.recv_timeout(Duration::from_secs(20)), Ok(true));
+        let started = Instant::now();
+        download.cancel();
+        let error = reading.join().unwrap().unwrap_err();
+        assert_eq!(error.to_string(), CANCELLED);
+        assert!(
+            started.elapsed() < Duration::from_secs(2),
+            "{:?}",
+            started.elapsed()
+        );
+    }
+
+    /// The calls that download nothing are as any client's: a body that
+    /// drops is an error.
+    #[test]
+    fn a_body_of_another_call_does_not_resume() {
+        let (url, _asked) = serve(vec![Answer::File {
+            from: 0,
+            send: Some(10_000),
+            ranged: false,
+        }]);
+        let transport =
+            UreqTransport::with_user_agent("Tabletist/0.3.0 (Linux; x86_64)".into(), None);
+        assert!(read_all(get(&transport, &url)).is_err());
+    }
+
+    /// A reconnect starts where the redirect chain began, for the address
+    /// a redirect gives may be signed for a few minutes only.
+    #[test]
+    fn a_reconnect_starts_from_the_first_address_of_its_redirects() {
+        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
+        let base = format!("http://{}", listener.local_addr().unwrap());
+        let (asked, heard) = mpsc::channel();
+        std::thread::spawn(move || {
+            let file = file();
+            for round in 0..4 {
+                let Ok((mut stream, _)) = listener.accept() else {
+                    return;
+                };
+                let request = read_request(&stream);
+                let _ = asked.send(request.line.clone());
+                let signed = request.line.contains("/signed");
+                let head = match (signed, request_range(&request)) {
+                    (false, _) => format!(
+                        "HTTP/1.1 302 Found\r\nLocation: /signed?round={round}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
+                    ),
+                    (true, None) => format!(
+                        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
+                        file.len()
+                    ),
+                    (true, Some(from)) => format!(
+                        "HTTP/1.1 206 Partial Content\r\nContent-Length: {}\r\nContent-Range: bytes {from}-{}/{}\r\nConnection: close\r\n\r\n",
+                        file.len() - from,
+                        file.len() - 1,
+                        file.len()
+                    ),
+                };
+                let _ = stream.write_all(head.as_bytes());
+                match (signed, request_range(&request)) {
+                    (true, None) => {
+                        let _ = stream.write_all(&file[..30_000]);
+                    }
+                    (true, Some(from)) => {
+                        let _ = stream.write_all(&file[from..]);
+                    }
+                    (false, _) => {}
+                }
+                let _ = stream.flush();
+                let _ = stream.shutdown(std::net::Shutdown::Both);
+            }
+        });
+        let (transport, _download, _waited) = downloading(QUICK);
+        let first = get(
+            &transport,
+            &format!("{base}/download/v99.0.0/tabletist.tar.gz"),
+        );
+        assert_eq!(first.status, 302);
+        let target = absolute(
+            &format!("{base}/download/v99.0.0/tabletist.tar.gz"),
+            first.location.as_deref().unwrap(),
+        );
+        let read = read_all(get(&transport, &target)).unwrap();
+        assert!(read == file(), "the bytes differ");
+        let lines: Vec<String> = heard.try_iter().collect();
+        assert_eq!(
+            lines,
+            [
+                "GET /download/v99.0.0/tabletist.tar.gz HTTP/1.1",
+                "GET /signed?round=0 HTTP/1.1",
+                "GET /download/v99.0.0/tabletist.tar.gz HTTP/1.1",
+                "GET /signed?round=2 HTTP/1.1",
+            ]
+        );
+    }
+
+    fn request_range(asked: &Asked) -> Option<usize> {
+        asked.range().map(|from| usize::try_from(from).unwrap())
+    }
+
+    #[test]
+    fn a_redirect_is_made_absolute_against_its_request() {
+        let base = "https://github.com/o/r/releases/download/v1/file.tar.gz";
+        assert_eq!(
+            absolute(base, "https://release-assets.githubusercontent.com/x?sig=1"),
+            "https://release-assets.githubusercontent.com/x?sig=1"
+        );
+        assert_eq!(absolute(base, "/o/r/other"), "https://github.com/o/r/other");
+        assert_eq!(
+            absolute(base, "next.tar.gz"),
+            "https://github.com/o/r/releases/download/v1/next.tar.gz"
+        );
+        assert_eq!(
+            absolute(base, "//objects.githubusercontent.com/y"),
+            "https://objects.githubusercontent.com/y"
+        );
+        assert_eq!(
+            host("http://127.0.0.1:8123/a?b").as_deref(),
+            Some("127.0.0.1:8123")
+        );
+        assert_eq!(range_start("bytes 18400-31199/31200"), Some(18_400));
+        assert_eq!(range_start("items 1-2"), None);
+    }
+
+    /// Serves every connection, the n-th as `answer` says, with the
+    /// request line and the range it asked, until the test ends.
+    fn serve_each(
+        answer: impl Fn(usize, &str, Option<u64>, &mut TcpStream) + Send + 'static,
+    ) -> (
+        String,
+        Arc<std::sync::atomic::AtomicUsize>,
+        mpsc::Receiver<String>,
+    ) {
+        use std::sync::atomic::AtomicUsize;
+        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
+        let base = format!("http://{}", listener.local_addr().unwrap());
+        let count = Arc::new(AtomicUsize::new(0));
+        let counted = Arc::clone(&count);
+        let (asked, heard) = mpsc::channel();
+        std::thread::spawn(move || {
+            for stream in listener.incoming() {
+                let Ok(mut stream) = stream else {
+                    return;
+                };
+                let n = counted.fetch_add(1, Ordering::SeqCst);
+                let request = read_request(&stream);
+                let _ = asked.send(request.line.clone());
+                answer(n, &request.line, request.range(), &mut stream);
+            }
+        });
+        (base, count, heard)
+    }
+
+    /// The head of a 200 for the whole file, or of a 206 from `from`.
+    fn head(from: Option<u64>) -> String {
+        let length = file().len();
+        match from {
+            None => {
+                format!("HTTP/1.1 200 OK\r\nContent-Length: {length}\r\nConnection: close\r\n\r\n")
+            }
+            Some(from) => format!(
+                "HTTP/1.1 206 Partial Content\r\nContent-Length: {}\r\nContent-Range: bytes {from}-{}/{length}\r\nConnection: close\r\n\r\n",
+                length as u64 - from,
+                length - 1
+            ),
+        }
+    }
+
+    /// 50 kB of the file, then nothing, and no close: the network went
+    /// with the socket open, as when Wi-Fi drops.
+    fn goes_quiet(_: usize, _: &str, _: Option<u64>, stream: &mut TcpStream) {
+        let _ = stream.write_all(head(None).as_bytes());
+        let _ = stream.write_all(&file()[..50_000]);
+        let _ = stream.flush();
+        std::thread::sleep(Duration::from_secs(20));
+    }
+
+    /// Reads `body` on a thread of its own: what came, or the error.
+    fn read_aside(mut body: Box<dyn Read + Send>) -> mpsc::Receiver<Result<Vec<u8>, String>> {
+        let (done, finished) = mpsc::channel();
+        std::thread::spawn(move || {
+            let mut read = Vec::new();
+            let result = body.read_to_end(&mut read).map(|_| read);
+            let _ = done.send(result.map_err(|error| error.to_string()));
+        });
+        finished
+    }
+
+    #[test]
+    fn a_cancel_ends_a_read_of_a_connection_gone_quiet() {
+        let (base, _, _) = serve_each(goes_quiet);
+        let (transport, download, _waited) = downloading(PATIENCE);
+        let mut body = get(&transport, &format!("{base}/file")).body;
+        let mut buffer = vec![0; 64 * 1024];
+        let mut got = 0;
+        while got < 50_000 {
+            got += body.read(&mut buffer).unwrap();
+        }
+        let finished = read_aside(body);
+        std::thread::sleep(Duration::from_millis(200));
+        download.cancel();
+        let result = finished.recv_timeout(Duration::from_secs(2));
+        assert_eq!(result, Ok(Err(CANCELLED.to_owned())));
+    }
+
+    #[test]
+    fn a_connection_gone_quiet_waits_for_the_network() {
+        let (base, _, _) = serve_each(goes_quiet);
+        let patience = Patience {
+            quiet: Duration::from_millis(300),
+            ..PATIENCE
+        };
+        let (transport, download, waited) = downloading(patience);
+        let _finished = read_aside(get(&transport, &format!("{base}/file")).body);
+        assert_eq!(waited.recv_timeout(Duration::from_secs(10)), Ok(true));
+        download.cancel();
+        // The reconnect sends again what came: no new byte, so it still
+        // waits.
+        assert!(download.waiting());
+        // The default notices well before a person gives up on it.
+        const { assert!(PATIENCE.quiet.as_secs() < 10) };
+    }
+
+    /// A server that answers each reconnect with its head, then drops it:
+    /// the reconnects are paced, and the download gives up in time.
+    #[test]
+    fn a_reconnect_that_drops_at_once_is_paced_and_gives_up() {
+        let (base, count, _) = serve_each(|n, _, range, stream| {
+            if n == 0 {
+                let _ = stream.write_all(head(None).as_bytes());
+                let _ = stream.write_all(&file()[..70_000]);
+            } else {
+                let _ = stream.write_all(head(Some(range.unwrap_or(0))).as_bytes());
+            }
+            let _ = stream.flush();
+            let _ = stream.shutdown(std::net::Shutdown::Both);
+        });
+        let patience = Patience {
+            retry: Duration::from_millis(100),
+            give_up: Duration::from_secs(1),
+            quiet: Duration::from_secs(20),
+        };
+        let (transport, download, _waited) = downloading(patience);
+        let finished = read_aside(get(&transport, &format!("{base}/file")).body);
+        let result = finished.recv_timeout(Duration::from_secs(5));
+        download.cancel();
+        assert_eq!(result, Ok(Err(LOST.to_owned())));
+        let asked = count.load(Ordering::SeqCst);
+        assert!(asked < 20, "asked {asked} times in a second");
+    }
+
+    /// The updater asks for a redirect's target in its own form of the
+    /// address (`url` resolves `..`): the reconnect still starts where the
+    /// chain began.
+    #[test]
+    fn a_reconnect_starts_from_the_first_address_whatever_form_the_redirect_takes() {
+        let (base, _, asked) = serve_each(|n, line, range, stream| {
+            if line.contains("/download/") {
+                let _ = stream.write_all(
+                    b"HTTP/1.1 302 Found\r\nLocation: /signed/../blob?sig=1\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
+                );
+            } else {
+                let from = range.unwrap_or(0);
+                let rest = &file()[usize::try_from(from).unwrap()..];
+                let _ = stream.write_all(head(range).as_bytes());
+                let send = if n == 1 { 70_000 } else { rest.len() };
+                let _ = stream.write_all(&rest[..send]);
+            }
+            let _ = stream.flush();
+            let _ = stream.shutdown(std::net::Shutdown::Both);
+        });
+        let (transport, _download, _waited) = downloading(QUICK);
+        let first = get(
+            &transport,
+            &format!("{base}/download/v99.0.0/tabletist.tar.gz"),
+        );
+        assert_eq!(first.status, 302);
+        let read = read_all(get(&transport, &format!("{base}/blob?sig=1"))).unwrap();
+        assert!(read == file(), "the bytes differ");
+        let lines: Vec<String> = asked.try_iter().collect();
+        assert_eq!(
+            lines.get(2).map(String::as_str),
+            Some("GET /download/v99.0.0/tabletist.tar.gz HTTP/1.1"),
+            "{lines:?}"
+        );
+    }
+
+    /// No network when the download starts: it waits as after a drop.
+    #[test]
+    fn a_download_that_starts_offline_waits_for_the_network() {
+        let (base, _, _) = serve_each(|n, _, _, stream| {
+            if n >= 2 {
+                let _ = stream.write_all(head(None).as_bytes());
+                let _ = stream.write_all(&file());
+                let _ = stream.flush();
+            }
+            let _ = stream.shutdown(std::net::Shutdown::Both);
+        });
+        let (transport, download, waited) = downloading(QUICK);
+        let read = read_all(get(&transport, &format!("{base}/file"))).unwrap();
+        assert!(read == file(), "the bytes differ");
+        assert_eq!(waited.try_iter().collect::<Vec<_>>(), [true, false]);
+        assert!(!download.waiting());
+    }
+
+    #[test]
+    fn a_reconnect_follows_redirects_over_https_to_the_hosts_it_went_through() {
+        let hosts = vec![
+            "github.com".to_owned(),
+            "release-assets.githubusercontent.com".to_owned(),
+            "127.0.0.1:8123".to_owned(),
+        ];
+        assert!(may_follow(
+            "https://release-assets.githubusercontent.com/x?sig=1",
+            &hosts
+        ));
+        assert!(may_follow("http://127.0.0.1:8123/blob", &hosts));
+        for refused in [
+            "http://github.com/o/r/file",
+            "https://user@github.com/o/r/file",
+            "https://example.com/file",
+            "ftp://github.com/file",
+        ] {
+            assert!(!may_follow(refused, &hosts), "{refused}");
+        }
+    }
+}
```

- [ ] **Step 4: Run the checks**

Run: `cargo fmt --all --check && cargo clippy --locked --workspace --all-targets -- -D warnings && cargo clippy --locked --workspace --all-targets --features shots -- -D warnings && RUSTDOCFLAGS='-D warnings' cargo doc --locked --workspace --no-deps && cargo test --locked --workspace --all-targets`.
Expected: clean; the library's tests `test result: ok. 3630 passed; 0 failed; 17 ignored`.

- [ ] **Step 5: Commit**

```bash
git add src/app/updates.rs src/backend.rs src/backend/updates.rs src/update.rs src/update/download.rs src/update/transport.rs
git commit -m "Download an update on the backend, resuming after a drop and waiting while offline"
```

### Task 2: The app downloads, waits for work and restarts

**Files:** `src/update.rs`, `src/update/download.rs`, `src/update/relaunch.rs` (new), `src/app.rs`, `src/app/editing.rs`, `src/app/updates.rs`, `src/app/startup_tests.rs`, `src/backend/updates.rs`, `src/entrypoint.rs`, `src/model.rs`, `src/paths.rs`, `tests/cli.rs`, `AGENTS.md`.

`update::Install` is where the download and the restart stand.

**The download.** Download and install sends `Download` and moves to Downloading. Downloading counts the bytes and the speed for the time left, and says when it waits or verifies. Its answer is Ready, or Failed with the sorted failure.

**Restart now** reads `work_in_progress()`: each window's unsaved edits per table, saves, write runs, other SQL runs and tables being read, named as the window's title names it.
- With none, it sends `Install` with the launch's arguments plus `--relaunch`, and the open SQL editors for `relaunch.json`. It quits when the helper answers.
- With some, it is Held. Restart when done arms it, and it goes ahead in the frame the list empties (`tick_restart`, after the actions). Don't restart goes back to Ready.

**The helper's minute (row 22).** Work opened while the helper got ready is waited for until `HELPER_WAITS` (55 s); then it is Couldn't install. The helper's answer is taken in `logic`, so a hidden window quits in time. An answer read after `QUIT_IN_TIME` (50 s) quits nothing.

**The relaunched copy.** `--relaunch` restores the last session whatever On launch and Reopen tabs say, and puts each window's SQL editors back in their places. It reads and deletes the file once, at the start of the launch.

Windows downloads nothing (row 16): its tests that need a download say so and return.

- [ ] **Step 1: Write the failing tests**

`src/app/startup_tests.rs`:

```diff
diff --git a/src/app/startup_tests.rs b/src/app/startup_tests.rs
index ab386758..d5a5e57d 100644
--- a/src/app/startup_tests.rs
+++ b/src/app/startup_tests.rs
@@ -1149,3 +1149,170 @@ fn a_launch_restores_the_picker_after_a_window_and_in_front() {
     assert_eq!(windows(&app), vec![Some(kept), None]);
     assert_eq!(app.active, 1);
 }
+
+/// A copy an update's helper started opens the windows of the restart,
+/// whatever the launch setting and the tables' setting say, and their SQL
+/// editors with their text, and deletes what kept them.
+#[test]
+fn a_relaunch_after_an_update_restores_the_windows_and_their_sql_editors() {
+    use crate::update::relaunch::{Editor, Relaunch, Window};
+    let dir = tempfile::tempdir().unwrap();
+    let mut app = app_in(dir.path());
+    let conn = bookshop(&mut app, "Bookshop");
+    let tab = open_window(&mut app, &conn);
+    books(&mut app, tab, "book_covers");
+    app.save_session();
+    app.apply(Action::SetOption(crate::settings::OptionValue::OnLaunch(
+        crate::settings::Launch::Connections,
+    )));
+    app.change_settings(|settings| settings.restore_tabs = false);
+    write_files(&app);
+    written(&app)
+        .unwrap()
+        .save(&app.dirs.session_file())
+        .unwrap();
+    let kept = Relaunch {
+        windows: vec![Window {
+            connection: conn.0.clone(),
+            editors: vec![Editor {
+                number: 3,
+                text: "SELECT * FROM book_covers".into(),
+                writes: false,
+                at: 1,
+            }],
+            front: None,
+        }],
+    };
+    let path = app.dirs.relaunch_file();
+    crate::util::save_json(&path, &kept).unwrap();
+    // A launch of its own shows Connections, as the setting says.
+    assert_eq!(windows(&start(dir.path())), vec![None]);
+    let dirs = AppDirs::at(dir.path());
+    let settings = Settings::load(&dirs);
+    let app = App::relaunched(dirs, settings, Backend::recording());
+    assert_eq!(windows(&app), vec![Some(conn)]);
+    let workspace = app.workspace(app.tabs[0].id).unwrap();
+    assert_eq!(workspace.object_tabs().count(), 1);
+    let editors: Vec<(u32, &str)> = workspace
+        .tabs
+        .iter()
+        .filter_map(|open| open.as_sql())
+        .map(|sql| (sql.number, sql.text.as_str()))
+        .collect();
+    assert_eq!(editors, [(3, "SELECT * FROM book_covers")]);
+    // The next new one counts on.
+    assert_eq!(workspace.next_query, 4);
+    assert!(!path.exists());
+}
+
+/// The SQL editors of `app`'s windows, by window, as their text.
+fn sql_texts(app: &App) -> Vec<Vec<String>> {
+    app.tabs
+        .iter()
+        .map(|tab| match &tab.content {
+            ConnTabContent::Workspace(workspace) => workspace
+                .tabs
+                .iter()
+                .filter_map(|open| open.as_sql())
+                .map(|sql| sql.text.clone())
+                .collect(),
+            ConnTabContent::Picker(_) => Vec::new(),
+        })
+        .collect()
+}
+
+/// A SQL editor holding `text`, in front of `tab`'s window.
+fn add_sql(app: &mut App, tab: ConnTabId, text: &str) -> TabId {
+    let id = TabId(app.next_id());
+    let workspace = app.workspace_mut(tab).unwrap();
+    let sql = workspace.push_sql_tab(id, 1_000, None);
+    sql.text = text.into();
+    workspace.active_tab = Some(id);
+    id
+}
+
+/// Restart now in `app`, then a relaunch over the files it leaves: the
+/// relaunch file the backend writes, and the session the quit saves.
+fn restart_and_relaunch(mut app: App, dir: &std::path::Path) -> App {
+    app.updates.install = crate::update::Install::Ready {
+        version: "99.0.0".into(),
+        handed: None,
+    };
+    app.apply(Action::RestartToUpdate);
+    let kept = app
+        .backend
+        .sent
+        .iter()
+        .find_map(|sent| match sent {
+            Command::Updates(crate::backend::UpdatesCommand::Install { relaunch, .. }) => {
+                Some(relaunch.clone())
+            }
+            _ => None,
+        })
+        .expect("handed over");
+    app.save_session();
+    write_files(&app);
+    written(&app)
+        .unwrap()
+        .save(&app.dirs.session_file())
+        .unwrap();
+    crate::util::save_json(&app.dirs.relaunch_file(), &kept).unwrap();
+    let dirs = AppDirs::at(dir);
+    let settings = Settings::load(&dirs);
+    App::relaunched(dirs, settings, Backend::recording())
+}
+
+/// One connection in two windows, the second in front, which connects
+/// first: each window gets its own editors back.
+#[test]
+fn each_window_of_one_connection_gets_its_own_editors_back() {
+    let dir = tempfile::tempdir().unwrap();
+    let mut app = app_in(dir.path());
+    let conn = bookshop(&mut app, "Bookshop");
+    let first = open_window(&mut app, &conn);
+    add_sql(&mut app, first, "SELECT 'first window'");
+    let second = open_window(&mut app, &conn);
+    add_sql(&mut app, second, "SELECT 'second window'");
+    app.active = app.tab_index(second).unwrap();
+    let app = restart_and_relaunch(app, dir.path());
+    assert_eq!(
+        sql_texts(&app),
+        [
+            vec!["SELECT 'first window'"],
+            vec!["SELECT 'second window'"]
+        ]
+    );
+}
+
+/// A SQL editor in front between two tables is in front again, between
+/// them, in its mode.
+#[test]
+fn a_relaunch_puts_each_sql_editor_back_in_its_place() {
+    let dir = tempfile::tempdir().unwrap();
+    let mut app = app_in(dir.path());
+    let conn = bookshop(&mut app, "Bookshop");
+    let tab = open_window(&mut app, &conn);
+    books(&mut app, tab, "book_covers");
+    let sql = add_sql(&mut app, tab, "SELECT 2");
+    if let Some(editor) = app.sql_tab_mut(tab, sql) {
+        editor.mode = RunMode::ReadWrite;
+    }
+    books(&mut app, tab, "publishers");
+    app.workspace_mut(tab).unwrap().active_tab = Some(sql);
+    let app = restart_and_relaunch(app, dir.path());
+    let workspace = app.workspace(app.tabs[0].id).unwrap();
+    let kinds: Vec<bool> = workspace
+        .tabs
+        .iter()
+        .map(|open| open.as_sql().is_some())
+        .collect();
+    assert_eq!(kinds, [false, true, false]);
+    let front = workspace
+        .active_tab
+        .and_then(|id| workspace.tabs.iter().find(|open| open.id() == id))
+        .and_then(|open| open.as_sql());
+    assert_eq!(
+        front.map(|sql| (sql.text.as_str(), sql.mode)),
+        Some(("SELECT 2", RunMode::ReadWrite))
+    );
+}
```

`src/app/updates.rs`:

```diff
diff --git a/src/app/updates.rs b/src/app/updates.rs
index 3762b701..af217cd1 100644
--- a/src/app/updates.rs
+++ b/src/app/updates.rs
@@ -694,4 +694,835 @@ mod tests {
         answer(&mut harness, Ok(latest("v99.0.0")));
         assert!(harness.app.updates.offer(Some("99.0.0")).is_some());
     }
+
+    // The download and the restart.
+
+    /// A harness whose copy replaces itself, offered 99.0.0 with its build.
+    fn offered() -> Harness {
+        let mut harness = harness(UpdateCheck::Never, Some(now()));
+        let mut offer = release::offer(&latest("v99.0.0"), release::Os::current(), None);
+        offer
+            .download
+            .get_or_insert(("tabletist-v99.0.0.tar.gz".into(), 31_200_000));
+        harness.app.updates.seen.offer = Some(offer);
+        harness
+    }
+
+    fn downloads(harness: &Harness) -> Vec<(u64, String)> {
+        harness
+            .app
+            .backend
+            .sent
+            .iter()
+            .filter_map(|sent| match sent {
+                Command::Updates(UpdatesCommand::Download { asked, version, .. }) => {
+                    Some((*asked, version.clone()))
+                }
+                _ => None,
+            })
+            .collect()
+    }
+
+    fn installs(harness: &Harness) -> Vec<Vec<String>> {
+        harness
+            .app
+            .backend
+            .sent
+            .iter()
+            .filter_map(|sent| match sent {
+                Command::Updates(UpdatesCommand::Install { arguments, .. }) => {
+                    Some(arguments.clone())
+                }
+                _ => None,
+            })
+            .collect()
+    }
+
+    /// What the last Install keeps for the next copy.
+    fn kept(harness: &Harness) -> crate::update::relaunch::Relaunch {
+        harness
+            .app
+            .backend
+            .sent
+            .iter()
+            .rev()
+            .find_map(|sent| match sent {
+                Command::Updates(UpdatesCommand::Install { relaunch, path, .. })
+                    if *path == harness.app.dirs.relaunch_file() =>
+                {
+                    Some(relaunch.clone())
+                }
+                _ => None,
+            })
+            .expect("an Install")
+    }
+
+    fn forgot(harness: &Harness) -> bool {
+        harness.app.backend.sent.iter().any(|sent| {
+            matches!(sent, Command::Updates(UpdatesCommand::ForgetRelaunch { path })
+                if *path == harness.app.dirs.relaunch_file())
+        })
+    }
+
+    fn handed(harness: &mut Harness, result: Result<(), String>, ago: Duration) {
+        let at = std::time::Instant::now().checked_sub(ago).unwrap();
+        event(harness, UpdatesEvent::Handed { result, at });
+    }
+
+    fn ready_state() -> Install {
+        Install::Ready {
+            version: "99.0.0".into(),
+            handed: None,
+        }
+    }
+
+    fn event(harness: &mut Harness, event: UpdatesEvent) {
+        harness.app.apply(Action::Backend(Event::Updates(event)));
+    }
+
+    /// Downloads 99.0.0 and says it verified: Ready. Windows downloads
+    /// nothing: it is put there.
+    fn ready(harness: &mut Harness) {
+        if !crate::update::DOWNLOADS {
+            harness.app.updates.install = ready_state();
+            return;
+        }
+        harness.app.apply(Action::DownloadUpdate);
+        let (asked, _) = downloads(harness)[0].clone();
+        event(
+            harness,
+            UpdatesEvent::Downloaded {
+                asked,
+                result: Ok(()),
+            },
+        );
+        assert_eq!(harness.app.updates.install, ready_state());
+    }
+
+    #[test]
+    fn download_and_install_downloads_the_version_offered() {
+        // Windows downloads nothing (row 16).
+        if !crate::update::DOWNLOADS {
+            return;
+        }
+        let mut harness = offered();
+        harness.app.apply(Action::DownloadUpdate);
+        let sent = downloads(&harness);
+        assert_eq!(sent.len(), 1);
+        assert_eq!(sent[0].1, "99.0.0");
+        let Install::Downloading(downloading) = &harness.app.updates.install else {
+            panic!("{:?}", harness.app.updates.install);
+        };
+        // The size its release gives the build.
+        let size = harness
+            .app
+            .updates
+            .seen
+            .offer
+            .as_ref()
+            .unwrap()
+            .download
+            .as_ref()
+            .unwrap()
+            .1;
+        assert_eq!(downloading.total, size);
+        // Once at a time.
+        harness.app.apply(Action::DownloadUpdate);
+        assert_eq!(downloads(&harness).len(), 1);
+    }
+
+    #[test]
+    fn a_copy_that_cannot_install_downloads_nothing() {
+        // Something else installed it.
+        let mut elsewhere = offered();
+        elsewhere.app.updates.installed = Some(crate::update::Installed::Elsewhere(
+            crate::update::Managed {
+                said: "Installed with Homebrew, so Homebrew updates it",
+                command: Some("brew upgrade --cask tabletist"),
+            },
+        ));
+        elsewhere.app.apply(Action::DownloadUpdate);
+        assert!(downloads(&elsewhere).is_empty());
+        // The release has no build for this computer.
+        let mut no_build = offered();
+        if let Some(offer) = &mut no_build.app.updates.seen.offer {
+            offer.download = None;
+        }
+        no_build.app.apply(Action::DownloadUpdate);
+        assert!(downloads(&no_build).is_empty());
+        // Not known yet how this copy was installed.
+        let mut unknown = offered();
+        unknown.app.updates.installed = None;
+        unknown.app.apply(Action::DownloadUpdate);
+        assert!(downloads(&unknown).is_empty());
+    }
+
+    #[test]
+    fn the_download_counts_its_bytes_and_the_time_left() {
+        // Windows downloads nothing (row 16).
+        if !crate::update::DOWNLOADS {
+            return;
+        }
+        let mut harness = offered();
+        harness.app.apply(Action::DownloadUpdate);
+        let (asked, _) = downloads(&harness)[0].clone();
+        for (secs, received) in [(1, 2_000_000), (3, 6_000_000), (6, 12_000_000)] {
+            event(
+                &mut harness,
+                UpdatesEvent::Progress {
+                    asked,
+                    received,
+                    total: 31_200_000,
+                    after: Duration::from_secs(secs),
+                },
+            );
+        }
+        // An answer to another download is dropped.
+        event(
+            &mut harness,
+            UpdatesEvent::Progress {
+                asked: asked + 100,
+                received: 30_000_000,
+                total: 31_200_000,
+                after: Duration::from_secs(7),
+            },
+        );
+        let Install::Downloading(downloading) = &harness.app.updates.install else {
+            panic!("downloading");
+        };
+        assert_eq!(downloading.received, 12_000_000);
+        // 2 MB a second over the last five seconds, 19.2 MB to go.
+        assert_eq!(downloading.time_left().map(|left| left.as_secs()), Some(9));
+        assert!(!downloading.verifying());
+        event(
+            &mut harness,
+            UpdatesEvent::Progress {
+                asked,
+                received: 31_200_000,
+                total: 31_200_000,
+                after: Duration::from_secs(16),
+            },
+        );
+        let Install::Downloading(downloading) = &harness.app.updates.install else {
+            panic!("downloading");
+        };
+        assert!(downloading.verifying());
+    }
+
+    #[test]
+    fn the_download_says_when_it_waits_for_the_network() {
+        // Windows downloads nothing (row 16).
+        if !crate::update::DOWNLOADS {
+            return;
+        }
+        let mut harness = offered();
+        harness.app.apply(Action::DownloadUpdate);
+        let (asked, _) = downloads(&harness)[0].clone();
+        event(
+            &mut harness,
+            UpdatesEvent::Waiting {
+                asked,
+                waiting: true,
+            },
+        );
+        assert!(matches!(
+            &harness.app.updates.install,
+            Install::Downloading(downloading) if downloading.waiting
+        ));
+        event(
+            &mut harness,
+            UpdatesEvent::Waiting {
+                asked,
+                waiting: false,
+            },
+        );
+        assert!(matches!(
+            &harness.app.updates.install,
+            Install::Downloading(downloading) if !downloading.waiting
+        ));
+    }
+
+    #[test]
+    fn a_download_that_failed_says_why_and_offers_another() {
+        // Windows downloads nothing (row 16).
+        if !crate::update::DOWNLOADS {
+            return;
+        }
+        let failure = Failure::Unverified(download::Unverified::Signature);
+        let mut harness = offered();
+        harness.app.apply(Action::DownloadUpdate);
+        let (asked, _) = downloads(&harness)[0].clone();
+        event(
+            &mut harness,
+            UpdatesEvent::Downloaded {
+                asked,
+                result: Err(failure.clone()),
+            },
+        );
+        assert_eq!(
+            harness.app.updates.install,
+            Install::Failed {
+                version: "99.0.0".into(),
+                failure
+            }
+        );
+        // Download again.
+        harness.app.apply(Action::DownloadUpdate);
+        assert_eq!(downloads(&harness).len(), 2);
+    }
+
+    #[test]
+    fn report_on_github_opens_an_issue_filled_in() {
+        // Windows downloads nothing (row 16).
+        if !crate::update::DOWNLOADS {
+            return;
+        }
+        let mut harness = offered();
+        harness.app.apply(Action::DownloadUpdate);
+        let (asked, _) = downloads(&harness)[0].clone();
+        event(
+            &mut harness,
+            UpdatesEvent::Downloaded {
+                asked,
+                result: Err(Failure::Unverified(download::Unverified::Hash)),
+            },
+        );
+        harness.app.apply(Action::ReportUpdateFailure);
+        let opened = harness.app.url_to_open.clone().expect("a page to open");
+        assert!(
+            opened.starts_with("https://github.com/igor-alexandrov/tabletist/issues/new?"),
+            "{opened}"
+        );
+        assert!(opened.contains("99.0.0"), "{opened}");
+    }
+
+    #[test]
+    fn cancel_stops_the_download_and_drops_its_answer() {
+        // Windows downloads nothing (row 16).
+        if !crate::update::DOWNLOADS {
+            return;
+        }
+        let mut harness = offered();
+        harness.app.apply(Action::DownloadUpdate);
+        let (asked, _) = downloads(&harness)[0].clone();
+        harness.app.apply(Action::CancelDownload);
+        assert_eq!(harness.app.updates.install, Install::Idle);
+        let cancelled = harness
+            .app
+            .backend
+            .sent
+            .iter()
+            .any(|sent| matches!(sent, Command::Updates(UpdatesCommand::CancelDownload)));
+        assert!(cancelled);
+        event(
+            &mut harness,
+            UpdatesEvent::Downloaded {
+                asked,
+                result: Ok(()),
+            },
+        );
+        assert_eq!(harness.app.updates.install, Install::Idle);
+    }
+
+    /// No check runs while a download runs or waits to be installed:
+    /// the card shows that.
+    #[test]
+    fn no_check_runs_while_a_download_is_under_way_or_ready() {
+        // Windows downloads nothing (row 16).
+        if !crate::update::DOWNLOADS {
+            return;
+        }
+        let mut harness = offered();
+        harness.app.settings.update_check = UpdateCheck::Daily;
+        harness.app.updates.seen.last_checked = Some(now() - 2 * DAY);
+        harness.app.apply(Action::DownloadUpdate);
+        harness.app.tick_updates();
+        harness.app.apply(Action::CheckForUpdates);
+        assert!(checks(&harness).is_empty());
+    }
+
+    #[test]
+    fn restart_now_with_nothing_open_hands_the_update_over_and_quits() {
+        let mut harness = offered();
+        harness.app.launch_arguments = vec!["--verbose".into()];
+        let tab = harness.connect_fake();
+        let sql = harness.add_sql_tab(tab);
+        harness.app.sql_tab_mut(tab, sql).unwrap().text = "SELECT * FROM book_covers".into();
+        ready(&mut harness);
+        harness.app.apply(Action::RestartToUpdate);
+        // The launch's arguments, then the relaunch's flag.
+        assert_eq!(installs(&harness), [vec!["--verbose", RELAUNCH]]);
+        // The SQL editors go with it, for the next copy, in front.
+        let kept = kept(&harness);
+        assert_eq!(kept.windows.len(), 1);
+        assert_eq!(kept.windows[0].editors[0].text, "SELECT * FROM book_covers");
+        assert!(!harness.app.closing);
+        handed(&mut harness, Ok(()), Duration::ZERO);
+        assert!(harness.app.closing);
+        assert!(!forgot(&harness));
+    }
+
+    /// The helper that did not start leaves this copy running, and says
+    /// why.
+    #[test]
+    fn a_helper_that_did_not_start_leaves_the_app_open() {
+        let mut harness = offered();
+        ready(&mut harness);
+        harness.app.apply(Action::RestartToUpdate);
+        handed(&mut harness, Err("no helper".into()), Duration::ZERO);
+        assert!(!harness.app.closing);
+        // No next copy reads the relaunch file: it goes.
+        assert!(forgot(&harness));
+        assert_eq!(
+            harness.app.updates.install,
+            Install::Failed {
+                version: "99.0.0".into(),
+                failure: Failure::NotHanded("no helper".into())
+            }
+        );
+    }
+
+    /// The relaunch's flag is not given twice to a copy started with it.
+    #[test]
+    fn a_relaunched_copy_restarts_with_the_flag_once() {
+        let mut harness = offered();
+        harness.app.launch_arguments = vec![RELAUNCH.into()];
+        ready(&mut harness);
+        harness.app.apply(Action::RestartToUpdate);
+        assert_eq!(installs(&harness), [vec![RELAUNCH]]);
+    }
+
+    /// `kind` of the first row of `book_covers` changed, and not saved.
+    fn unsaved_edit(harness: &mut Harness) -> (crate::model::ConnTabId, crate::model::TabId) {
+        let (tab, id) = harness.book_covers();
+        harness.app.workspace_mut(tab).unwrap().name = "Bookshop".into();
+        harness.app.apply(Action::EditCell {
+            tab,
+            id,
+            cell: crate::model::CellPos { row: 0, col: 2 },
+            start: crate::model::EditStart::Replace("ebook".into()),
+        });
+        harness.app.apply(Action::CommitEdit {
+            tab,
+            id,
+            then: crate::model::Advance::Stay,
+        });
+        (tab, id)
+    }
+
+    #[test]
+    fn restart_now_with_unsaved_edits_lists_them_and_waits() {
+        let mut harness = offered();
+        let (tab, id) = unsaved_edit(&mut harness);
+        ready(&mut harness);
+        harness.app.apply(Action::RestartToUpdate);
+        assert!(installs(&harness).is_empty());
+        assert_eq!(
+            harness.app.updates.install,
+            Install::Held {
+                version: "99.0.0".into(),
+                when_done: false,
+                handed: None,
+            }
+        );
+        assert_eq!(
+            harness.app.work_in_progress(),
+            [Work {
+                tab,
+                id,
+                window: "Bookshop · dev".into(),
+                what: Pending::Edits {
+                    count: 1,
+                    table: "book_covers".into()
+                },
+            }]
+        );
+        // Shown where the list is.
+        assert!(matches!(
+            &harness.app.dialog,
+            Some(Dialog::Settings(dialog)) if dialog.pane == SettingsPane::Updates
+        ));
+        // Nothing was dropped.
+        let edits = &harness
+            .app
+            .workspace(tab)
+            .unwrap()
+            .object_tab(id)
+            .unwrap()
+            .edits;
+        assert!(edits.holds());
+    }
+
+    #[test]
+    fn restart_when_done_restarts_once_the_work_is_saved() {
+        let mut harness = offered();
+        let (tab, id) = unsaved_edit(&mut harness);
+        ready(&mut harness);
+        harness.app.apply(Action::RestartToUpdate);
+        harness.app.apply(Action::RestartWhenDone);
+        assert!(installs(&harness).is_empty());
+        // The save runs: still work.
+        harness.app.apply(Action::WriteEdits { tab, id });
+        assert_eq!(
+            harness.app.work_in_progress()[0].what,
+            Pending::Saving {
+                table: "book_covers".into()
+            }
+        );
+        harness.app.tick_updates();
+        assert!(installs(&harness).is_empty());
+        harness.answer_written(Ok(tabletist_db::WriteOutcome::Written {
+            inserted: Vec::new(),
+            rows: Vec::new(),
+            elapsed: Duration::ZERO,
+            gone: Vec::new(),
+        }));
+        // The table reads its page again after the save: work too.
+        assert_eq!(
+            harness
+                .app
+                .work_in_progress()
+                .iter()
+                .map(|work| work.what.clone())
+                .collect::<Vec<_>>(),
+            [Pending::Loading {
+                table: "book_covers".into()
+            }]
+        );
+        harness.app.tick_updates();
+        assert!(installs(&harness).is_empty());
+        harness.answer_rows(crate::testing::book_covers_page(3));
+        assert!(harness.app.work_in_progress().is_empty());
+        harness.app.tick_updates();
+        assert_eq!(installs(&harness).len(), 1);
+    }
+
+    #[test]
+    fn dont_restart_goes_back_to_ready() {
+        let mut harness = offered();
+        let (tab, id) = unsaved_edit(&mut harness);
+        ready(&mut harness);
+        harness.app.apply(Action::RestartToUpdate);
+        harness.app.apply(Action::RestartWhenDone);
+        harness.app.apply(Action::DontRestart);
+        harness.app.apply(Action::DiscardEdits { tab, id });
+        harness.app.tick_updates();
+        assert!(installs(&harness).is_empty());
+        assert_eq!(harness.app.updates.install, ready_state());
+    }
+
+    #[test]
+    fn show_them_brings_the_window_with_work_forward_on_its_tab() {
+        let mut harness = offered();
+        let (tab, id) = unsaved_edit(&mut harness);
+        // Another tab in front, then another window.
+        let sql = harness.add_sql_tab(tab);
+        harness.app.apply(Action::ActivateTab { tab, id: sql });
+        harness.app.apply(Action::ShowConnections);
+        ready(&mut harness);
+        harness.app.apply(Action::RestartToUpdate);
+        harness.app.window_asks.clear();
+        harness.app.apply(Action::ShowWork);
+        assert!(harness.app.dialog.is_none());
+        assert_eq!(harness.app.active_tab_id(), tab);
+        assert_eq!(harness.app.workspace(tab).unwrap().active_tab, Some(id));
+        assert_eq!(harness.app.window_asks, [egui::ViewportCommand::Focus]);
+    }
+
+    #[test]
+    fn a_run_of_the_sql_editor_is_work_and_a_write_is_named_so() {
+        for write in [false, true] {
+            let mut harness = offered();
+            let tab = harness.connect_fake_as(!write);
+            harness.app.workspace_mut(tab).unwrap().name = "Bookshop".into();
+            let sql = harness.add_sql_tab(tab);
+            let editor = harness.app.sql_tab_mut(tab, sql).unwrap();
+            editor.text = match write {
+                true => "UPDATE book_covers SET kind = 'print' WHERE id = 1".into(),
+                false => "SELECT * FROM book_covers".into(),
+            };
+            if write {
+                editor.mode = crate::model::RunMode::ReadWrite;
+            }
+            let number = editor.number;
+            harness.app.apply(Action::RunSql {
+                tab,
+                sql_tab: sql,
+                all: true,
+            });
+            let work = harness.app.work_in_progress();
+            assert_eq!(work.len(), 1, "{work:?}");
+            match write {
+                true => assert_eq!(work[0].what, Pending::Writing { query: number }),
+                false => assert!(
+                    matches!(work[0].what, Pending::Running { query, .. } if query == number)
+                ),
+            }
+            assert_eq!(work[0].window, "Bookshop · dev");
+        }
+    }
+
+    /// Work opened while the helper got ready is never dropped: the card
+    /// waits for it, armed, while the helper does, and the quit comes as
+    /// soon as it is done.
+    #[test]
+    fn work_opened_while_the_helper_got_ready_is_waited_for() {
+        let mut harness = offered();
+        ready(&mut harness);
+        harness.app.apply(Action::RestartToUpdate);
+        let (tab, id) = unsaved_edit(&mut harness);
+        handed(&mut harness, Ok(()), Duration::ZERO);
+        assert!(!harness.app.closing);
+        assert!(matches!(
+            harness.app.updates.install,
+            Install::Held {
+                when_done: true,
+                handed: Some(_),
+                ..
+            }
+        ));
+        harness.app.apply(Action::DiscardEdits { tab, id });
+        harness.app.apply_actions();
+        assert!(harness.app.closing);
+        // The helper has it: nothing is handed over again.
+        assert_eq!(installs(&harness).len(), 1);
+    }
+
+    /// Past the helper's minute nothing installs: Couldn't install says
+    /// so, and the relaunch file goes.
+    #[test]
+    fn work_still_open_when_the_helper_gives_up_is_couldnt_install() {
+        let mut harness = offered();
+        ready(&mut harness);
+        harness.app.apply(Action::RestartToUpdate);
+        unsaved_edit(&mut harness);
+        handed(&mut harness, Ok(()), Duration::from_secs(30));
+        harness.app.tick_updates();
+        assert!(matches!(harness.app.updates.install, Install::Held { .. }));
+        let late = std::time::Instant::now()
+            .checked_sub(crate::update::HELPER_WAITS)
+            .unwrap();
+        if let Install::Held { handed, .. } = &mut harness.app.updates.install {
+            *handed = Some(late);
+        }
+        harness.app.tick_updates();
+        assert!(matches!(
+            harness.app.updates.install,
+            Install::Failed {
+                failure: Failure::WorkBegan,
+                ..
+            }
+        ));
+        assert!(forgot(&harness));
+        assert!(!harness.app.closing);
+    }
+
+    /// Don't restart while the helper waits goes back to Ready, which a
+    /// quit in that minute still installs; then the minute ends it.
+    #[test]
+    fn dont_restart_while_the_helper_waits_is_ready_until_it_gives_up() {
+        let mut harness = offered();
+        ready(&mut harness);
+        harness.app.apply(Action::RestartToUpdate);
+        unsaved_edit(&mut harness);
+        handed(&mut harness, Ok(()), Duration::ZERO);
+        harness.app.apply(Action::DontRestart);
+        assert!(matches!(
+            harness.app.updates.install,
+            Install::Ready {
+                handed: Some(_),
+                ..
+            }
+        ));
+        if let Install::Ready { handed, .. } = &mut harness.app.updates.install {
+            *handed = std::time::Instant::now().checked_sub(crate::update::HELPER_WAITS);
+        }
+        harness.app.tick_updates();
+        assert!(matches!(
+            harness.app.updates.install,
+            Install::Failed {
+                failure: Failure::WorkBegan,
+                ..
+            }
+        ));
+        assert!(forgot(&harness));
+    }
+
+    /// An answer read too late to quit within the helper's minute (the
+    /// window was hidden) installs nothing, and says so.
+    #[test]
+    fn a_handoff_read_too_late_quits_nothing() {
+        let mut harness = offered();
+        ready(&mut harness);
+        harness.app.apply(Action::RestartToUpdate);
+        handed(&mut harness, Ok(()), crate::update::QUIT_IN_TIME);
+        assert!(!harness.app.closing);
+        assert!(matches!(
+            harness.app.updates.install,
+            Install::Failed {
+                failure: Failure::TooLate,
+                ..
+            }
+        ));
+        assert!(forgot(&harness));
+    }
+
+    /// With the window hidden no frame runs, only `logic`: it takes the
+    /// helper's answer and quits.
+    #[test]
+    fn the_helper_s_answer_is_taken_with_the_window_hidden() {
+        let mut harness = offered();
+        ready(&mut harness);
+        harness.app.apply(Action::RestartToUpdate);
+        harness
+            .app
+            .backend
+            .inject(Event::Updates(UpdatesEvent::Handed {
+                result: Ok(()),
+                at: std::time::Instant::now(),
+            }));
+        let ctx = harness.ctx.clone();
+        harness.app.logic(&ctx);
+        assert!(harness.app.closing);
+    }
+
+    /// Restart when done goes ahead in the frame the work ends in: no
+    /// input is waited for.
+    #[test]
+    fn restart_when_done_goes_ahead_in_the_frame_the_work_ends() {
+        let mut harness = offered();
+        let tab = harness.connect_fake();
+        let sql = harness.add_sql_tab(tab);
+        harness.app.sql_tab_mut(tab, sql).unwrap().text = "SELECT 1".into();
+        harness.app.apply(Action::RunSql {
+            tab,
+            sql_tab: sql,
+            all: true,
+        });
+        ready(&mut harness);
+        harness.app.apply(Action::RestartToUpdate);
+        harness.app.apply(Action::RestartWhenDone);
+        assert!(installs(&harness).is_empty());
+        let (session, request) = harness
+            .app
+            .backend
+            .sent
+            .iter()
+            .rev()
+            .find_map(|sent| match sent {
+                Command::RunSql {
+                    session, request, ..
+                } => Some((*session, *request)),
+                _ => None,
+            })
+            .expect("a run");
+        harness.app.backend.inject(Event::SqlRan {
+            session,
+            request,
+            result: Err(tabletist_db::Error::query("stopped")),
+            cancel: None,
+        });
+        harness.frame(Vec::new());
+        assert_eq!(installs(&harness).len(), 1);
+    }
+
+    /// A dialog open over the window (here the shortcuts) would go with
+    /// the quit: Restart when done waits for it.
+    #[test]
+    fn restart_when_done_waits_for_a_dialog_to_close() {
+        let mut harness = offered();
+        let (tab, id) = unsaved_edit(&mut harness);
+        ready(&mut harness);
+        harness.app.apply(Action::RestartToUpdate);
+        harness.app.apply(Action::RestartWhenDone);
+        harness.app.apply(Action::DiscardEdits { tab, id });
+        harness.app.dialog = Some(Dialog::Help);
+        harness.app.apply_actions();
+        assert!(installs(&harness).is_empty());
+        harness.app.dialog = None;
+        harness.app.apply_actions();
+        assert_eq!(installs(&harness).len(), 1);
+    }
+
+    /// Work that went before Restart when done was chosen leaves nothing
+    /// to wait for: Ready again.
+    #[test]
+    fn a_list_that_empties_is_ready_again() {
+        let mut harness = offered();
+        let (tab, id) = unsaved_edit(&mut harness);
+        ready(&mut harness);
+        harness.app.apply(Action::RestartToUpdate);
+        harness.app.apply(Action::DiscardEdits { tab, id });
+        harness.app.apply_actions();
+        assert_eq!(harness.app.updates.install, ready_state());
+        assert!(installs(&harness).is_empty());
+    }
+
+    /// Skip this version is the card's only while it offers the version:
+    /// one that downloads, waits to install or failed is never skipped.
+    #[test]
+    fn skip_is_ignored_once_the_version_downloads() {
+        let mut harness = offered();
+        ready(&mut harness);
+        harness.app.apply(Action::SkipUpdate);
+        assert_eq!(harness.app.settings.update_skip, None);
+    }
+
+    /// A table's read running is work too: a restart would drop it.
+    #[test]
+    fn a_table_being_read_is_work() {
+        let mut harness = offered();
+        let tab = harness.connect_fake();
+        harness.app.workspace_mut(tab).unwrap().name = "Bookshop".into();
+        harness.app.apply(Action::OpenObject {
+            tab,
+            object: tabletist_db::ObjectRef::new("main", "book_covers"),
+            kind: tabletist_db::ObjectKind::Table,
+            pin: true,
+        });
+        let work = harness.app.work_in_progress();
+        assert_eq!(
+            work.iter()
+                .map(|work| work.what.clone())
+                .collect::<Vec<_>>(),
+            [Pending::Loading {
+                table: "book_covers".into()
+            }]
+        );
+    }
+
+    /// Each editor keeps its place among the tables, its mode, and which
+    /// tab was in front.
+    #[test]
+    fn the_relaunch_keeps_each_editor_s_place_and_mode_and_the_tab_in_front() {
+        let mut harness = offered();
+        let (tab, _) = harness.book_covers();
+        let sql = harness.add_sql_tab(tab);
+        harness.app.sql_tab_mut(tab, sql).unwrap().mode = crate::model::RunMode::ReadWrite;
+        harness.app.apply(Action::OpenObject {
+            tab,
+            object: tabletist_db::ObjectRef::new("main", "publishers"),
+            kind: tabletist_db::ObjectKind::Table,
+            pin: true,
+        });
+        harness.app.apply(Action::ActivateTab { tab, id: sql });
+        let kept = harness.app.kept_for_relaunch();
+        let window = &kept.windows[0];
+        assert_eq!(window.front, Some(1));
+        assert_eq!((window.editors[0].at, window.editors[0].writes), (1, true));
+    }
+
+    /// Later keeps it ready, and closes Settings.
+    #[test]
+    fn later_keeps_the_update_ready() {
+        let mut harness = offered();
+        harness
+            .app
+            .apply(Action::ShowSettingsPane(SettingsPane::Updates));
+        ready(&mut harness);
+        harness.app.apply(Action::UpdateLater);
+        assert!(harness.app.dialog.is_none());
+        assert!(matches!(harness.app.updates.install, Install::Ready { .. }));
+        assert!(installs(&harness).is_empty());
+    }
 }
```

`src/backend/updates.rs`:

```diff
diff --git a/src/backend/updates.rs b/src/backend/updates.rs
index 48b1b7ad..8819d0e6 100644
--- a/src/backend/updates.rs
+++ b/src/backend/updates.rs
@@ -369,13 +369,40 @@ mod handed {
         panic!("no answer");
     }
 
+    /// Nothing held: nothing is handed over, and no relaunch file is
+    /// left for a next copy that will not come.
     #[test]
     fn install_with_no_download_held_hands_nothing_over() {
+        let dir = tempfile::tempdir().unwrap();
+        let path = dir.path().join("relaunch.json");
+        std::fs::write(&path, "{}").unwrap();
         let mut backend = Backend::start_with(Waker::default(), crate::secrets::Keyring::memory());
         backend.send(Command::Updates(UpdatesCommand::Install {
             arguments: vec!["--relaunch".into()],
+            relaunch: crate::update::relaunch::Relaunch::default(),
+            path: path.clone(),
         }));
-        assert!(matches!(answer(&mut backend), UpdatesEvent::Handed(Err(_))));
+        assert!(matches!(
+            answer(&mut backend),
+            UpdatesEvent::Handed { result: Err(_), .. }
+        ));
+        assert!(!path.exists());
+    }
+
+    #[test]
+    fn forget_relaunch_deletes_the_file() {
+        let dir = tempfile::tempdir().unwrap();
+        let path = dir.path().join("relaunch.json");
+        std::fs::write(&path, "{}").unwrap();
+        let mut backend = Backend::start_with(Waker::default(), crate::secrets::Keyring::memory());
+        backend.send(Command::Updates(UpdatesCommand::ForgetRelaunch {
+            path: path.clone(),
+        }));
+        let deadline = Instant::now() + Duration::from_secs(20);
+        while path.exists() && Instant::now() < deadline {
+            std::thread::sleep(Duration::from_millis(10));
+        }
+        assert!(!path.exists());
     }
 
     /// A test binary is no installation fastframe-update can replace: it
@@ -397,10 +424,16 @@ mod handed {
             } => {}
             other => panic!("{other:?}"),
         }
+        let dir = tempfile::tempdir().unwrap();
         backend.send(Command::Updates(UpdatesCommand::Install {
             arguments: Vec::new(),
+            relaunch: crate::update::relaunch::Relaunch::default(),
+            path: dir.path().join("relaunch.json"),
         }));
-        assert!(matches!(answer(&mut backend), UpdatesEvent::Handed(Err(_))));
+        assert!(matches!(
+            answer(&mut backend),
+            UpdatesEvent::Handed { result: Err(_), .. }
+        ));
     }
 
     /// A download that verified as the cancel came is not left held: the
```

`tests/cli.rs`:

```diff
diff --git a/tests/cli.rs b/tests/cli.rs
index 99f5cbb0..9d6f0445 100644
--- a/tests/cli.rs
+++ b/tests/cli.rs
@@ -64,6 +64,20 @@ fn update_flags_are_taken_out_before_the_arguments_are_read() {
     }
 }
 
+/// The helper starts the new version with `--relaunch` after the launch's
+/// own arguments, and that version reads it: every release accepts it,
+/// and `--help` does not list it.
+#[test]
+fn the_relaunch_flag_is_read_and_not_listed() {
+    let output = tabletist()
+        .args(["--relaunch", "--version"])
+        .output()
+        .expect("run tabletist");
+    assert!(output.status.success(), "{output:?}");
+    let help = tabletist().arg("--help").output().expect("run tabletist");
+    assert!(!String::from_utf8_lossy(&help.stdout).contains("--relaunch"));
+}
+
 /// `--apply-update <job>` makes the process the helper, before anything
 /// else: with no job to read it fails and exits, and never opens a window.
 #[test]
```

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test --locked --workspace --all-targets --no-run`
Expected: no build, 152 errors, such as:

```
error[E0432]: unresolved import `crate::update::relaunch`
error[E0433]: cannot find `Install` in `update`
```

- [ ] **Step 3: Write the code**

`AGENTS.md`:

```diff
diff --git a/AGENTS.md b/AGENTS.md
index 17b16ced..02e56a3b 100644
--- a/AGENTS.md
+++ b/AGENTS.md
@@ -264,6 +264,10 @@ needs a release that accepts both ways first:
   clap reads the command line (`tests/cli.rs`).
 - The receipt is confirmed once the first frame is painted, on the second
   (`entrypoint.rs`): eframe paints a frame after its `ui` returns.
+- The helper starts the new version with the old one's arguments and
+  `--relaunch`, a hidden flag every release accepts: it reopens the windows
+  of the restart, and the SQL editors `relaunch.json` kept
+  (`App::relaunched`).
 - The release's file names, the markers (`packaging/tabletist-portable.txt`
   in the archives, `packaging/windows/tabletist-installer.txt` installed
   beside the app) and `checksums.txt.sig` stay as `release.yml` makes them
```

`src/app.rs`:

```diff
diff --git a/src/app.rs b/src/app.rs
index 87db19a8..1113664c 100644
--- a/src/app.rs
+++ b/src/app.rs
@@ -281,6 +281,15 @@ pub struct App {
     /// A page the next frame opens in the browser: a release's, where its
     /// notes could not be shown in a terminal.
     pub url_to_open: Option<String>,
+    /// The command line this copy was started with, without the program's
+    /// name: an update's helper starts the new version with it.
+    pub launch_arguments: Vec<String>,
+    /// What a restart to update kept, while the relaunched windows open:
+    /// each window's own, by its tab.
+    relaunch_windows: HashMap<ConnTabId, crate::update::relaunch::Window>,
+    /// The SQL editors a restart to update kept, while the relaunched
+    /// windows open: each takes its own.
+    relaunch: Option<crate::update::relaunch::Relaunch>,
     next_id: u64,
 }
 
@@ -315,6 +324,18 @@ fn load_table_lists(path: &std::path::Path) -> (TableLists, Unread) {
 
 impl App {
     pub fn new(dirs: AppDirs, loaded: Loaded, backend: Backend) -> Self {
+        Self::opened(dirs, loaded, backend, false)
+    }
+
+    /// [`App::new`], for the copy an update's helper started after a
+    /// restart to install it (`--relaunch`): the windows open at the quit
+    /// come back whatever the launch setting says, with their tables and
+    /// the SQL editors `relaunch.json` kept.
+    pub fn relaunched(dirs: AppDirs, loaded: Loaded, backend: Backend) -> Self {
+        Self::opened(dirs, loaded, backend, true)
+    }
+
+    fn opened(dirs: AppDirs, loaded: Loaded, backend: Backend, relaunched: bool) -> Self {
         let source = loaded.source;
         let (settings, settings_file) = loaded.into_parts();
         let loaded_connections = SavedConnections::load(&dirs);
@@ -431,10 +452,13 @@ impl App {
             updates_url: crate::update::check::LATEST.to_owned(),
             update_announcement: None,
             url_to_open: None,
+            launch_arguments: Vec::new(),
+            relaunch: None,
+            relaunch_windows: HashMap::new(),
             next_id: 1,
         };
         app.refresh_keymap();
-        app.launch();
+        app.launch(relaunched);
         // What an older version kept in connections.json is written as
         // this one keeps it, once, off the UI thread. So is a file in
         // which a connection had no id yet.
@@ -867,6 +891,8 @@ impl App {
         self.read_reopened();
         self.format_rows();
         self.make_reviews();
+        // Restart when done goes ahead in the frame the work ended in.
+        self.tick_restart();
     }
 
     /// Formats the row each open row panel shows, once per selection or
@@ -2555,7 +2581,15 @@ impl App {
             Action::CheckForUpdates
             | Action::CancelUpdateCheck
             | Action::SkipUpdate
-            | Action::ShowReleaseNotes => self.update_action(action),
+            | Action::ShowReleaseNotes
+            | Action::DownloadUpdate
+            | Action::CancelDownload
+            | Action::RestartToUpdate
+            | Action::RestartWhenDone
+            | Action::DontRestart
+            | Action::UpdateLater
+            | Action::ShowWork
+            | Action::ReportUpdateFailure => self.update_action(action),
             Action::ChooseAgentSetup(agent) => {
                 if let Some(Dialog::Settings(dialog)) = &mut self.dialog {
                     dialog.agent = agent;
@@ -3297,9 +3331,21 @@ impl App {
     /// picker, so that the next launch does what the menu says; with
     /// connections that could not be read, nothing is known to be
     /// deleted, and the option stays. Whatever opens nothing opens the
-    /// picker.
-    fn launch(&mut self) {
-        match self.settings.on_launch.clone() {
+    /// picker. A relaunch after an update opens the last windows,
+    /// whatever the settings say, as [`App::relaunched`] says.
+    fn launch(&mut self, relaunched: bool) {
+        if relaunched {
+            let path = self.dirs.relaunch_file();
+            self.relaunch = Some(crate::update::relaunch::Relaunch::take(&path));
+            self.open_last_session();
+            self.relaunch = None;
+            self.relaunch_windows.clear();
+        }
+        let on_launch = match relaunched {
+            true => crate::settings::Launch::Connections,
+            false => self.settings.on_launch.clone(),
+        };
+        match on_launch {
             crate::settings::Launch::Restore => self.open_last_session(),
             crate::settings::Launch::Connections => {}
             crate::settings::Launch::Connection { id, .. } => {
@@ -3359,6 +3405,13 @@ impl App {
                 }
             };
             let tab = self.picker_tab();
+            // What a restart to update kept of this window: windows of
+            // one connection take theirs in the bar's order.
+            if let (Some(relaunch), Some(saved)) = (&mut self.relaunch, &saved)
+                && let Some(window) = relaunch.window_of(&saved.id.0)
+            {
+                self.relaunch_windows.insert(tab.id, window);
+            }
             windows.push((tab.id, saved));
             self.tabs.push(tab);
             if index <= self.session.active {
@@ -3520,9 +3573,11 @@ impl App {
         // are of the database it opens on.
         let kept = self.session.connections.get(&saved.id.0).cloned();
         let row_panel = kept.as_ref().is_none_or(|kept| kept.row_panel);
-        let tables = kept.filter(|kept| {
-            self.settings.restore_tabs && kept.database == crate::session::database_of(&saved.spec)
-        });
+        // A relaunch after an update puts back what the restart took away,
+        // whatever the settings reopen.
+        let restore = self.settings.restore_tabs || self.relaunch.is_some();
+        let tables = kept
+            .filter(|kept| restore && kept.database == crate::session::database_of(&saved.spec));
         let Some(entry) = self.tabs.iter_mut().find(|t| t.id == tab) else {
             return;
         };
@@ -3535,6 +3590,9 @@ impl App {
         if let Some(kept) = tables {
             self.reopen_tabs(tab, &kept);
         }
+        if let Some(window) = self.relaunch_windows.remove(&tab) {
+            self.reopen_editors(tab, window);
+        }
         if replaced {
             self.save_session();
         }
@@ -3571,6 +3629,45 @@ impl App {
         workspace.active_tab = active;
     }
 
+    /// Puts the SQL editors a restart to update kept in `tab`'s window,
+    /// each in its place among the tables, with its number, its text and
+    /// its mode, and the tab that was in front in front again. The new
+    /// ones count on from the last.
+    fn reopen_editors(&mut self, tab: ConnTabId, window: crate::update::relaunch::Window) {
+        let (limit, timeout) = (self.settings.sql_limit, self.settings.sql_timeout());
+        let mut editors = window.editors;
+        editors.sort_by_key(|editor| editor.at);
+        let ids: Vec<TabId> = editors.iter().map(|_| TabId(self.next_id())).collect();
+        let Some(workspace) = self.workspace_mut(tab) else {
+            return;
+        };
+        for (editor, id) in editors.into_iter().zip(ids) {
+            let sql = workspace.push_sql_tab(id, limit, timeout);
+            sql.mode = match editor.writes {
+                true => RunMode::ReadWrite,
+                false => RunMode::ReadOnly,
+            };
+            sql.number = editor.number;
+            sql.text = editor.text;
+            workspace.next_query = workspace.next_query.max(editor.number.saturating_add(1));
+            // Pushed at the end: moved to its place.
+            if let Some(pushed) = workspace.tabs.pop() {
+                let at = editor.at.min(workspace.tabs.len());
+                workspace.tabs.insert(at, pushed);
+            }
+        }
+        let front = window
+            .front
+            .and_then(|at| workspace.tabs.get(at))
+            .map(Tab::id);
+        if let Some(front) = front {
+            workspace.active_tab = Some(front);
+        }
+        if workspace.active_tab.is_none() {
+            workspace.active_tab = workspace.tabs.first().map(Tab::id);
+        }
+    }
+
     /// Reads the table in front of each window that was reopened and not
     /// read yet: its structure and its first page, as when it opened. A
     /// connect under way reads it once it is up (`App::wake_to_read`).
@@ -7926,6 +8023,12 @@ impl App {
         if let Some(wait) = self.close_idle(std::time::Instant::now()) {
             ctx.request_repaint_after(wait);
         }
+        // The helper's answer, and the quit it asks for, while the
+        // window is hidden too: the helper waits a minute at most.
+        self.take_handoff();
+        if self.closing {
+            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
+        }
         if let Some(wait) = self.tick_updates() {
             ctx.request_repaint_after(wait);
         }
```

`src/app/editing.rs`:

```diff
diff --git a/src/app/editing.rs b/src/app/editing.rs
index ac931902..7f383e73 100644
--- a/src/app/editing.rs
+++ b/src/app/editing.rs
@@ -387,7 +387,7 @@ impl App {
     /// typed back to what it loaded is one fewer: closing the editor
     /// takes it out of the set. An open editor with nothing pending yet
     /// still counts as one: a tab that holds edits never counts as none.
-    fn unwritten(&self, tab: ConnTabId, id: TabId) -> usize {
+    pub(super) fn unwritten(&self, tab: ConnTabId, id: TabId) -> usize {
         let Some(edits) = self
             .workspace(tab)
             .and_then(|workspace| workspace.object_tab(id))
```

`src/app/updates.rs`:

```diff
diff --git a/src/app/updates.rs b/src/app/updates.rs
index af217cd1..833246f8 100644
--- a/src/app/updates.rs
+++ b/src/app/updates.rs
@@ -1,15 +1,21 @@
 //! Updates, as the app keeps them: when a check runs (on launch where one
 //! is due, at the interval while the app is open, and when the person
-//! asks), what it found, and what the window says of it. The request is
-//! the backend's ([`crate::backend::UpdatesCommand`]); what the checks
-//! found is kept in the state folder (`updates.json`).
+//! asks), what it found, and what the window says of it; then the
+//! download of the version offered, and the restart that installs it once
+//! no window has work open. The requests are the backend's
+//! ([`crate::backend::UpdatesCommand`]); what the checks found is kept in
+//! the state folder (`updates.json`).
 
 use std::time::Duration;
 
 use super::App;
 use crate::backend::{Command, StateFile, UpdatesCommand, UpdatesEvent};
-use crate::model::{Action, ConnTabContent};
-use crate::update::{Failed, Found, Phase, release};
+use crate::model::{Action, ConnTabContent, ConnTabId, Dialog, SettingsPane, Tab, TabId};
+use crate::update::{Downloading, Failed, Failure, Found, Install, Phase, download, release};
+
+/// The flag an update's helper starts the new version with, after the
+/// launch's own: it reopens the windows of the quit (`entrypoint.rs`).
+pub const RELAUNCH: &str = "--relaunch";
 
 /// What the terminal look's status line says of a version available,
 /// `{version}` for it and `{key}` for Check for Updates' key as the keymap
@@ -20,6 +26,92 @@ pub const ANNOUNCEMENT_KEYLESS: &str = "{version} available";
 /// How long the status line says it.
 pub const ANNOUNCED_FOR: f64 = 8.0;
 
+/// What a restart would interrupt: a line of the Work in progress card.
+#[derive(Clone, Debug, PartialEq, Eq)]
+pub struct Work {
+    /// The window that holds it, and its tab.
+    pub tab: ConnTabId,
+    pub id: TabId,
+    /// The window as the card names it: `Bookshop · dev`.
+    pub window: String,
+    pub what: Pending,
+}
+
+/// What is pending in a window.
+#[derive(Clone, Debug, PartialEq, Eq)]
+pub enum Pending {
+    /// Unsaved edits in a table: how many, and its name.
+    Edits { count: usize, table: String },
+    /// A save running in a table.
+    Saving { table: String },
+    /// A run of a SQL editor sent to write: the nearest Tabletist has to
+    /// an open transaction. Query 2.
+    Writing { query: u32 },
+    /// Another run of a SQL editor, and how long it has gone.
+    Running { query: u32, secs: u64 },
+    /// A table's page, count or structure being read.
+    Loading { table: String },
+}
+
+impl App {
+    /// What a restart would interrupt now, window by window in the bar's
+    /// order: each table's unsaved edits, its save or its read, then each
+    /// SQL editor's run. Nothing is ever dropped to restart.
+    pub fn work_in_progress(&self) -> Vec<Work> {
+        let platform = crate::env::Platform::of(&self.look);
+        let mut work = Vec::new();
+        for (tab, workspace) in self.open_connections() {
+            // As the window's title names it.
+            let environment = workspace.environment.label(platform);
+            let window = format!("{} · {environment}", workspace.name);
+            for object in workspace.object_tabs() {
+                let table = crate::ui::format::object_title(
+                    &object.object,
+                    workspace.name_is_shared(&object.object),
+                );
+                let what = if object.edits.saving.is_some() {
+                    Pending::Saving { table }
+                } else if object.edits.holds() {
+                    Pending::Edits {
+                        count: self.unwritten(tab, object.id),
+                        table,
+                    }
+                } else if object.pending().next().is_some() {
+                    Pending::Loading { table }
+                } else {
+                    continue;
+                };
+                work.push(Work {
+                    tab,
+                    id: object.id,
+                    window: window.clone(),
+                    what,
+                });
+            }
+            for sql in workspace.tabs.iter().filter_map(|open| open.as_sql()) {
+                let what = if sql.is_writing() {
+                    Pending::Writing { query: sql.number }
+                } else if sql.is_running() {
+                    let secs = sql.running_for().unwrap_or_default().as_secs();
+                    Pending::Running {
+                        query: sql.number,
+                        secs,
+                    }
+                } else {
+                    continue;
+                };
+                work.push(Work {
+                    tab,
+                    id: sql.id,
+                    window: window.clone(),
+                    what,
+                });
+            }
+        }
+        work
+    }
+}
+
 impl App {
     /// The time now, in seconds since the Unix epoch: the stopped clock in
     /// a test.
@@ -49,14 +141,24 @@ impl App {
     /// it runs once none does. Says how long until the next is due, for a
     /// frame to be asked for then.
     pub fn tick_updates(&mut self) -> Option<Duration> {
+        self.tick_restart();
+        // The helper waits for the quit a minute at most: looked at each
+        // second until it is over.
+        if self.updates.install.handed().is_some() {
+            return Some(Duration::from_secs(1));
+        }
         if !self.updates_scheduled {
             return None;
         }
         let now = self.now_secs();
         self.announce_update(now);
         // The asset an offer keeps is this copy's kind's: the check waits
-        // for the detection, which answers within the launch.
-        if self.updates.checking() || self.updates.installed.is_none() {
+        // for the detection, which answers within the launch. None runs
+        // while a download is under way or ready.
+        if self.updates.checking()
+            || self.updates.installed.is_none()
+            || self.updates.install.busy()
+        {
             return None;
         }
         let due = crate::update::due(
@@ -93,11 +195,56 @@ impl App {
     pub(super) fn update_action(&mut self, action: Action) {
         match action {
             // One at a time: a check under way is answered first, as one
-            // the person asked for.
-            Action::CheckForUpdates => match &mut self.updates.phase {
-                Phase::Checking { asked_for, .. } => *asked_for = true,
-                Phase::Idle | Phase::Failed { .. } => self.start_update_check(true),
-            },
+            // the person asked for. None while a download is under way or
+            // ready, which the card shows; one after a download that
+            // failed shows what it finds.
+            Action::CheckForUpdates => {
+                if self.updates.install.busy() {
+                    return;
+                }
+                self.updates.install = Install::Idle;
+                match &mut self.updates.phase {
+                    Phase::Checking { asked_for, .. } => *asked_for = true,
+                    Phase::Idle | Phase::Failed { .. } => self.start_update_check(true),
+                }
+            }
+            Action::DownloadUpdate => self.download_update(),
+            Action::CancelDownload => {
+                if let Install::Downloading(_) = self.updates.install {
+                    self.updates.install = Install::Idle;
+                    self.backend
+                        .send(Command::Updates(UpdatesCommand::CancelDownload));
+                }
+            }
+            Action::RestartToUpdate => self.restart_to_update(),
+            Action::RestartWhenDone => {
+                if let Install::Held { when_done, .. } = &mut self.updates.install {
+                    *when_done = true;
+                }
+                self.tick_restart();
+            }
+            // Back to Ready. Where the helper waits already, a quit in its
+            // minute still installs: Ready says so, then the minute ends it.
+            Action::DontRestart => {
+                if let Install::Held {
+                    version, handed, ..
+                } = &self.updates.install
+                {
+                    self.updates.install = Install::Ready {
+                        version: version.clone(),
+                        handed: *handed,
+                    };
+                }
+            }
+            // Ready stays: the next Restart now installs it.
+            Action::UpdateLater => self.close_settings(),
+            Action::ShowWork => self.show_work(),
+            Action::ReportUpdateFailure => {
+                if let Install::Failed { version, failure } = &self.updates.install {
+                    let said = failure.with_parts(failure.said());
+                    self.url_to_open = Some(download::issue_url(version, failure, &said));
+                }
+            }
             // The schedule's next check waits an interval from it, as
             // from one that failed.
             Action::CancelUpdateCheck => {
@@ -106,7 +253,12 @@ impl App {
                     self.updates.tried = Some(self.now_secs());
                 }
             }
+            // Only what the card offers to skip: never a version that
+            // downloads, waits to install or failed to.
             Action::SkipUpdate => {
+                if self.updates.install != Install::Idle {
+                    return;
+                }
                 let skip = self.settings.update_skip.as_deref();
                 let Some(version) = self.updates.offer(skip).map(|offer| offer.version.clone())
                 else {
@@ -140,11 +292,41 @@ impl App {
                 }
             }
             UpdatesEvent::NotesUnshown { url } => self.url_to_open = Some(url),
-            // Nothing asks for a download yet.
-            UpdatesEvent::Progress { .. }
-            | UpdatesEvent::Waiting { .. }
-            | UpdatesEvent::Downloaded { .. }
-            | UpdatesEvent::Handed(_) => {}
+            UpdatesEvent::Progress {
+                asked,
+                received,
+                total,
+                after,
+            } => {
+                if let Some(downloading) = self.downloading(asked) {
+                    downloading.progressed(received, total, after);
+                }
+            }
+            UpdatesEvent::Waiting { asked, waiting } => {
+                if let Some(downloading) = self.downloading(asked) {
+                    downloading.waiting = waiting;
+                }
+            }
+            UpdatesEvent::Downloaded { asked, result } => {
+                let Some(downloading) = self.downloading(asked) else {
+                    return;
+                };
+                let version = downloading.version.clone();
+                let then_restart = downloading.then_restart;
+                match result {
+                    Ok(()) => {
+                        self.updates.install = Install::Ready {
+                            version,
+                            handed: None,
+                        };
+                        if then_restart {
+                            self.restart_to_update();
+                        }
+                    }
+                    Err(failure) => self.updates.install = Install::Failed { version, failure },
+                }
+            }
+            UpdatesEvent::Handed { result, at } => self.handed(result, at),
             UpdatesEvent::Checked { asked, result } => {
                 let Phase::Checking {
                     asked: waited,
@@ -171,6 +353,254 @@ impl App {
         }
     }
 
+    /// The download under way, where it is the one `asked`: an answer to
+    /// any other is dropped.
+    fn downloading(&mut self, asked: u64) -> Option<&mut Downloading> {
+        match &mut self.updates.install {
+            Install::Downloading(downloading) if downloading.asked == asked => Some(downloading),
+            _ => None,
+        }
+    }
+
+    /// Starts downloading the version offered, where this copy can
+    /// install it and no download is under way or ready.
+    fn download_update(&mut self) {
+        if self.updates.install.busy() {
+            return;
+        }
+        let skip = self.settings.update_skip.as_deref();
+        let Some(offer) = self.updates.installable(skip) else {
+            return;
+        };
+        let version = offer.version.clone();
+        let url = offer.url.clone();
+        let total = offer.download.as_ref().map_or(0, |(_, size)| *size);
+        let asked = self.next_id();
+        self.updates.install =
+            Install::Downloading(Downloading::new(asked, version.clone(), total));
+        self.backend
+            .send(Command::Updates(UpdatesCommand::Download {
+                asked,
+                version,
+                url,
+            }));
+    }
+
+    /// Whether the app may quit for the restart now: no window has work
+    /// open, and no dialog but Settings is up, which a quit would drop.
+    fn may_restart(&self) -> bool {
+        matches!(self.dialog, None | Some(Dialog::Settings(_)))
+            && self.work_in_progress().is_empty()
+    }
+
+    /// Restart now: the restart goes ahead where it may. Otherwise the
+    /// card lists the work, in Settings › Updates, and waits for Restart
+    /// when done.
+    fn restart_to_update(&mut self) {
+        let (version, handed) = match &self.updates.install {
+            Install::Ready { version, handed }
+            | Install::Held {
+                version, handed, ..
+            } => (version.clone(), *handed),
+            _ => return,
+        };
+        if self.may_restart() {
+            self.go_restart(version, handed);
+            return;
+        }
+        if let Install::Ready { .. } = self.updates.install {
+            self.updates.install = Install::Held {
+                version,
+                when_done: false,
+                handed,
+            };
+        }
+        let shown = matches!(
+            &self.dialog,
+            Some(Dialog::Settings(dialog)) if dialog.pane == SettingsPane::Updates
+        );
+        if !shown {
+            self.apply(Action::ShowSettingsPane(SettingsPane::Updates));
+        }
+    }
+
+    /// The restart, now that nothing is in its way: handed over, or, where
+    /// the helper waits already, the quit it waits for.
+    fn go_restart(&mut self, version: String, handed: Option<std::time::Instant>) {
+        match handed {
+            Some(_) => self.closing = true,
+            None => self.restart(version),
+        }
+    }
+
+    /// Read after every frame's actions, and in `logic`: Restart when done
+    /// goes ahead once nothing is listed; a list that emptied without it
+    /// is Ready again; and the helper's minute, where it waits, ends in
+    /// Couldn't install.
+    pub(super) fn tick_restart(&mut self) {
+        let (version, when_done, handed) = match &self.updates.install {
+            Install::Held {
+                version,
+                when_done,
+                handed,
+            } => (version.clone(), *when_done, *handed),
+            Install::Ready {
+                version,
+                handed: Some(at),
+            } => (version.clone(), false, Some(*at)),
+            _ => return,
+        };
+        if handed.is_some_and(|at| at.elapsed() >= crate::update::HELPER_WAITS) {
+            self.not_installed(version, Failure::WorkBegan);
+            return;
+        }
+        let Install::Held { .. } = self.updates.install else {
+            return;
+        };
+        let empty = self.work_in_progress().is_empty();
+        match when_done {
+            true if self.may_restart() => self.go_restart(version, handed),
+            true => {}
+            false if empty => {
+                self.updates.install = Install::Ready { version, handed };
+            }
+            false => {}
+        }
+    }
+
+    /// The helper's answer: it stands ready, and the app quits now, unless
+    /// work was opened meanwhile; then the card waits for it, armed, while
+    /// the helper does. An answer read too late to quit in time (the
+    /// window hidden) installs nothing.
+    fn handed(&mut self, result: Result<(), String>, at: std::time::Instant) {
+        let Install::Restarting { version } = &self.updates.install else {
+            return;
+        };
+        let version = version.clone();
+        match result {
+            Err(error) => self.not_installed(version, Failure::NotHanded(error)),
+            Ok(()) if at.elapsed() >= crate::update::QUIT_IN_TIME => {
+                self.not_installed(version, Failure::TooLate);
+            }
+            Ok(()) if self.may_restart() => self.closing = true,
+            Ok(()) => {
+                self.updates.install = Install::Held {
+                    version,
+                    when_done: true,
+                    handed: Some(at),
+                };
+            }
+        }
+    }
+
+    /// Couldn't install: the relaunch file goes, for no next copy comes.
+    fn not_installed(&mut self, version: String, failure: Failure) {
+        self.updates.install = Install::Failed { version, failure };
+        self.backend
+            .send(Command::Updates(UpdatesCommand::ForgetRelaunch {
+                path: self.dirs.relaunch_file(),
+            }));
+    }
+
+    /// The helper's answer, taken in `logic` too: with the window hidden
+    /// no frame reads the backend, and the helper waits a minute at most.
+    /// The other events wait for the frame, in their order.
+    pub(super) fn take_handoff(&mut self) {
+        if !matches!(self.updates.install, Install::Restarting { .. }) {
+            return;
+        }
+        self.poll_backend();
+        let (handed, rest): (Vec<Action>, Vec<Action>) = std::mem::take(&mut self.actions)
+            .into_iter()
+            .partition(|action| {
+                matches!(
+                    action,
+                    Action::Backend(crate::backend::Event::Updates(UpdatesEvent::Handed { .. }))
+                )
+            });
+        self.actions = rest;
+        for action in handed {
+            self.apply(action);
+        }
+    }
+
+    /// Keeps the SQL editors for the next copy, and hands the download to
+    /// fastframe-update's helper, which starts that copy with this one's
+    /// arguments and [`RELAUNCH`]. The app quits once the helper stands
+    /// ready.
+    fn restart(&mut self, version: String) {
+        let mut arguments: Vec<String> = self
+            .launch_arguments
+            .iter()
+            .filter(|argument| *argument != RELAUNCH)
+            .cloned()
+            .collect();
+        arguments.push(RELAUNCH.to_owned());
+        self.backend.send(Command::Updates(UpdatesCommand::Install {
+            arguments,
+            relaunch: self.kept_for_relaunch(),
+            path: self.dirs.relaunch_file(),
+        }));
+        self.updates.install = Install::Restarting { version };
+    }
+
+    /// Each window's SQL editors, in the bar's order, each with its place
+    /// among the window's tables and editors, and which of those was in
+    /// front: what the session does not keep.
+    pub(crate) fn kept_for_relaunch(&self) -> crate::update::relaunch::Relaunch {
+        use crate::update::relaunch::{Editor, Relaunch, Window};
+        let windows = self
+            .open_connections()
+            .map(|(_, workspace)| {
+                // The tabs a relaunch puts back: the tables and editors.
+                let kept: Vec<&Tab> = workspace
+                    .tabs
+                    .iter()
+                    .filter(|open| open.as_object().is_some() || open.as_sql().is_some())
+                    .collect();
+                Window {
+                    connection: workspace.conn_id.0.clone(),
+                    editors: kept
+                        .iter()
+                        .enumerate()
+                        .filter_map(|(at, open)| Some((at, open.as_sql()?)))
+                        .map(|(at, sql)| Editor {
+                            number: sql.number,
+                            text: sql.text.clone(),
+                            writes: sql.mode == crate::model::RunMode::ReadWrite,
+                            at,
+                        })
+                        .collect(),
+                    front: kept
+                        .iter()
+                        .position(|open| Some(open.id()) == workspace.active_tab),
+                }
+            })
+            .collect();
+        Relaunch { windows }
+    }
+
+    /// Show them: Settings closes, and the first window with work comes
+    /// forward, on the tab that holds it.
+    fn show_work(&mut self) {
+        let Some(first) = self.work_in_progress().into_iter().next() else {
+            return;
+        };
+        self.close_settings();
+        self.window_asks.push(egui::ViewportCommand::Focus);
+        self.apply(Action::ActivateConnTab(first.tab));
+        self.apply(Action::ActivateTab {
+            tab: first.tab,
+            id: first.id,
+        });
+    }
+
+    fn close_settings(&mut self) {
+        if let Some(Dialog::Settings(_)) = self.dialog {
+            self.apply(Action::CloseDialog);
+        }
+    }
+
     /// A check that got its answer: kept as the last check, with the
     /// release it found where that is newer.
     fn checked(&mut self, latest: &release::Latest, asked_for: bool, now: u64) {
```

`src/backend/updates.rs`:

```diff
diff --git a/src/backend/updates.rs b/src/backend/updates.rs
index 8819d0e6..ca2d04ef 100644
--- a/src/backend/updates.rs
+++ b/src/backend/updates.rs
@@ -50,10 +50,18 @@ pub enum UpdatesCommand {
     /// Stops the download under way: fastframe-update deletes what it
     /// wrote. Nothing answers it.
     CancelDownload,
-    /// Hands the download held to fastframe-update's helper, which
-    /// installs it once the app quits and starts it with `arguments`.
-    /// Answered with [`UpdatesEvent::Handed`].
-    Install { arguments: Vec<String> },
+    /// Writes `relaunch` to `path`, then hands the download held to
+    /// fastframe-update's helper, which installs it once the app quits
+    /// and starts it with `arguments`. The file goes again where the
+    /// helper does not start. Answered with [`UpdatesEvent::Handed`].
+    Install {
+        arguments: Vec<String>,
+        relaunch: crate::update::relaunch::Relaunch,
+        path: PathBuf,
+    },
+    /// Deletes the relaunch file at `path`: the handoff did not end in
+    /// the quit.
+    ForgetRelaunch { path: PathBuf },
 }
 
 /// What came of an [`UpdatesCommand`].
@@ -88,10 +96,13 @@ pub enum UpdatesEvent {
         asked: u64,
         result: Result<(), Failure>,
     },
-    /// The helper stands ready, and the app is to quit now; or it did not
-    /// start, in fastframe-update's words. The download held is gone
-    /// either way.
-    Handed(Result<(), String>),
+    /// The helper stands ready since `at`, and the app is to quit now; or
+    /// it did not start, in fastframe-update's words. The download held is
+    /// gone either way.
+    Handed {
+        result: Result<(), String>,
+        at: Instant,
+    },
 }
 
 /// What the backend keeps of updates between commands.
@@ -127,7 +138,11 @@ impl Updates {
                     tokio::task::spawn_blocking(move || held.discard());
                 }
             }
-            UpdatesCommand::Install { arguments } => {
+            UpdatesCommand::Install {
+                arguments,
+                relaunch,
+                path,
+            } => {
                 let held = self
                     .held
                     .lock()
@@ -136,17 +151,25 @@ impl Updates {
                 let outbox = outbox.clone();
                 tokio::task::spawn_blocking(move || {
                     let handed = match held {
-                        Some(prepared) => crate::update::updater()
-                            .handoff(prepared, arguments)
+                        Some(prepared) => crate::util::save_json(&path, &relaunch)
+                            .map_err(anyhow::Error::from)
+                            .and_then(|()| crate::update::updater().handoff(prepared, arguments))
                             .map_err(|error| format!("{error:#}")),
                         None => Err("no update is ready to install".to_owned()),
                     };
                     if let Err(error) = &handed {
                         log::error!("could not hand the update over: {error}");
+                        forget(&path);
                     }
-                    outbox.emit(super::Event::Updates(UpdatesEvent::Handed(handed)));
+                    outbox.emit(super::Event::Updates(UpdatesEvent::Handed {
+                        result: handed,
+                        at: Instant::now(),
+                    }));
                 });
             }
+            UpdatesCommand::ForgetRelaunch { path } => {
+                tokio::task::spawn_blocking(move || forget(&path));
+            }
             command => handle(command, outbox),
         }
     }
@@ -212,6 +235,15 @@ impl Updates {
     }
 }
 
+/// Deletes the relaunch file at `path`, where there is one.
+fn forget(path: &std::path::Path) {
+    if let Err(error) = std::fs::remove_file(path)
+        && error.kind() != std::io::ErrorKind::NotFound
+    {
+        log::warn!("could not delete {}: {error}", path.display());
+    }
+}
+
 fn handle(command: UpdatesCommand, outbox: &super::Outbox) {
     let outbox = outbox.clone();
     match command {
@@ -254,7 +286,8 @@ fn handle(command: UpdatesCommand, outbox: &super::Outbox) {
         // Kept between commands: [`Updates::handle`]'s.
         UpdatesCommand::Download { .. }
         | UpdatesCommand::CancelDownload
-        | UpdatesCommand::Install { .. } => {}
+        | UpdatesCommand::Install { .. }
+        | UpdatesCommand::ForgetRelaunch { .. } => {}
     }
 }
 
```

`src/entrypoint.rs`:

```diff
diff --git a/src/entrypoint.rs b/src/entrypoint.rs
index d3f1d656..699b1d73 100644
--- a/src/entrypoint.rs
+++ b/src/entrypoint.rs
@@ -22,6 +22,11 @@ pub struct Cli {
     /// With --demo: the window's inner size, as WIDTHxHEIGHT.
     #[arg(long, requires = "demo", value_parser = parse_size)]
     pub demo_size: Option<[f32; 2]>,
+    /// Started by an update's helper after a restart to install it: the
+    /// windows of the last quit come back, whatever the launch setting
+    /// says, with their SQL editors. Every later release accepts it.
+    #[arg(long, hide = true)]
+    pub relaunch: bool,
 }
 
 /// What `tabletist` can be asked to do besides opening its window.
@@ -135,6 +140,15 @@ pub fn main(cli: Cli, launch: fastframe_update::Launch) -> anyhow::Result<()> {
         log::error!("{message}");
     }
     let update_receipt = launch.receipt;
+    // What the helper starts the next version with, but the program's
+    // name.
+    let launch_arguments: Vec<String> = launch
+        .arguments
+        .iter()
+        .skip(1)
+        .map(|argument| argument.to_string_lossy().into_owned())
+        .collect();
+    let relaunched = cli.relaunch && !cli.demo;
 
     let settings = Settings::load(&dirs);
     let demo = cli.demo;
@@ -149,7 +163,11 @@ pub fn main(cli: Cli, launch: fastframe_update::Launch) -> anyhow::Result<()> {
                 crate::backend::Waker::new(move || repaint.request_repaint()),
                 keyring_for(demo),
             );
-            let mut app = App::new(dirs, settings, backend);
+            let mut app = match relaunched {
+                true => App::relaunched(dirs, settings, backend),
+                false => App::new(dirs, settings, backend),
+            };
+            app.launch_arguments = launch_arguments;
             app.attach(&cc.egui_ctx, follow_desktop);
             // Not for a demo: its profile and its keyring are throwaway.
             if !demo {
```

`src/model.rs`:

```diff
diff --git a/src/model.rs b/src/model.rs
index cf3eff06..70c5ea22 100644
--- a/src/model.rs
+++ b/src/model.rs
@@ -1004,6 +1004,27 @@ pub enum Action {
     /// Show the notes of the version offered in `$PAGER`, in a terminal;
     /// where there is none to start, open its page.
     ShowReleaseNotes,
+    /// Download and verify the version offered, where this copy can
+    /// install it: Download and install, Download again, `U`.
+    DownloadUpdate,
+    /// Stop the download under way: Cancel, `ctrl+c`.
+    CancelDownload,
+    /// Install the download that verified and restart into it, once no
+    /// window has work open: Restart now, the start screen's Restart,
+    /// `U`. With work open the card lists it instead.
+    RestartToUpdate,
+    /// Restart as soon as the work listed is done.
+    RestartWhenDone,
+    /// Go back to Ready from waiting for the work: Don't restart.
+    DontRestart,
+    /// Keep the download for later, and close Settings.
+    UpdateLater,
+    /// Bring the first window with work forward, on the tab that holds
+    /// it: Show them.
+    ShowWork,
+    /// Open a new issue on GitHub about the download that failed, filled
+    /// in: Report on GitHub.
+    ReportUpdateFailure,
     /// Close Settings and show the Agents tab on this view: the window's in
     /// front, or with Connections in front the last window's.
     ShowAgentsFromSettings(AgentsView),
```

`src/paths.rs`:

```diff
diff --git a/src/paths.rs b/src/paths.rs
index c3322331..2a87f278 100644
--- a/src/paths.rs
+++ b/src/paths.rs
@@ -118,6 +118,13 @@ impl AppDirs {
         self.state.join("updates.json")
     }
 
+    /// The SQL editors open when a restart to install an update began,
+    /// for the copy it starts: written before the restart, and deleted by
+    /// that copy once read.
+    pub fn relaunch_file(&self) -> PathBuf {
+        self.state.join("relaunch.json")
+    }
+
     /// A release's notes, written for the pager to show.
     pub fn release_notes_file(&self, version: &str) -> PathBuf {
         self.state.join(format!("release-notes-{version}.md"))
```

`src/update.rs`:

```diff
diff --git a/src/update.rs b/src/update.rs
index 1baeadc0..356c2bad 100644
--- a/src/update.rs
+++ b/src/update.rs
@@ -8,20 +8,26 @@
 //! change: `tabletist --version` prints `tabletist <version>`;
 //! `--update-receipt <job>` and `--update-error <message>` are taken out
 //! before clap reads the command line ([`fastframe_update::intercept`] in
-//! `entrypoint::run`); and the receipt is confirmed once the window is up.
+//! `entrypoint::run`); the receipt is confirmed once the window is up; and
+//! `--relaunch`, which the helper adds to the old copy's arguments, is
+//! read.
 //! The release side of the contract (asset names, markers, the signed
 //! checksums) is in `release.yml` and `tests/update_release.rs`.
 
 pub mod check;
 pub mod download;
 pub mod installed;
+pub mod relaunch;
 pub mod release;
 pub mod seen;
 pub mod transport;
 
+use std::time::{Duration, Instant};
+
 use fastframe_update::{MacConfig, UpdateConfig, Updater};
 
 pub use check::Failed;
+pub use download::Failure;
 pub use installed::{Installed, Managed};
 pub use release::{Found, Offer};
 pub use seen::Seen;
@@ -61,9 +67,18 @@ pub fn updater() -> Updater {
 
 /// The release page of the version that runs: What's new in it.
 pub fn this_release() -> String {
-    format!("https://github.com/igor-alexandrov/tabletist/releases/tag/v{CURRENT}")
+    release_page(CURRENT)
 }
 
+/// The release page of `version`.
+pub fn release_page(version: &str) -> String {
+    format!("https://github.com/igor-alexandrov/tabletist/releases/tag/v{version}")
+}
+
+/// Whether this build downloads updates: not on Windows, whose copies
+/// get the release page.
+pub const DOWNLOADS: bool = !cfg!(windows);
+
 /// How long `check` waits between scheduled checks; None where checks
 /// are only asked for.
 pub fn interval(check: UpdateCheck) -> Option<u64> {
@@ -107,6 +122,131 @@ pub enum Phase {
     Failed { failed: Failed, asked_for: bool },
 }
 
+/// Where a download, and then its install, stands while the app runs.
+#[derive(Clone, Debug, Default, PartialEq, Eq)]
+pub enum Install {
+    /// No download asked for, or the last was cancelled.
+    #[default]
+    Idle,
+    /// Downloading, then verifying once the last byte came.
+    Downloading(Downloading),
+    /// Downloaded and verified: the backend holds it for Restart now.
+    /// Later keeps it so; a quit drops it. Or, `handed`, given to the
+    /// helper already, which then installs it at the next quit within
+    /// [`HELPER_WAITS`] of that time.
+    Ready {
+        version: String,
+        handed: Option<Instant>,
+    },
+    /// Restart now found work open, which the card lists. `when_done`:
+    /// Restart when done was chosen, and the restart goes ahead once
+    /// nothing is listed. `handed` as for `Ready`.
+    Held {
+        version: String,
+        when_done: bool,
+        handed: Option<Instant>,
+    },
+    /// Handed to the helper: the app quits once it stands ready.
+    Restarting { version: String },
+    /// The download gave nothing to install.
+    Failed { version: String, failure: Failure },
+}
+
+/// A download under way.
+#[derive(Clone, Debug, PartialEq, Eq)]
+pub struct Downloading {
+    /// Answers to any other ask are dropped.
+    pub asked: u64,
+    pub version: String,
+    pub received: u64,
+    /// The file's size, as its release says.
+    pub total: u64,
+    /// It waits for the network to come back.
+    pub waiting: bool,
+    /// Its progress lately, for the time left: the time since it started,
+    /// and the bytes it had then.
+    pub samples: Vec<(Duration, u64)>,
+    /// Restart once it is ready: `:update`'s yes.
+    pub then_restart: bool,
+}
+
+impl Downloading {
+    pub fn new(asked: u64, version: String, total: u64) -> Self {
+        Self {
+            asked,
+            version,
+            received: 0,
+            total,
+            waiting: false,
+            samples: vec![(Duration::ZERO, 0)],
+            then_restart: false,
+        }
+    }
+
+    /// The last byte came: fastframe-update checks the file now.
+    pub fn verifying(&self) -> bool {
+        self.total > 0 && self.received >= self.total
+    }
+
+    /// How long it has left, where that can be told.
+    pub fn time_left(&self) -> Option<Duration> {
+        download::time_left(&self.samples, self.total)
+    }
+
+    /// Notes `received` of `total`, `after` this long.
+    pub fn progressed(&mut self, received: u64, total: u64, after: Duration) {
+        self.received = received;
+        self.total = total;
+        self.samples.push((after, received));
+        download::keep_window(&mut self.samples);
+    }
+}
+
+/// How long fastframe-update's helper waits for the app to quit once it
+/// stands ready, a little short of its minute: after that it has given up,
+/// and nothing installs.
+pub const HELPER_WAITS: Duration = Duration::from_secs(55);
+
+/// How late a quit may still begin after the helper said it stands
+/// ready: the app takes a few seconds to go.
+pub const QUIT_IN_TIME: Duration = Duration::from_secs(50);
+
+impl Install {
+    /// When the helper said it stands ready, where it waits for the quit.
+    pub fn handed(&self) -> Option<Instant> {
+        match self {
+            Self::Ready { handed, .. } | Self::Held { handed, .. } => *handed,
+            Self::Idle | Self::Downloading(_) | Self::Restarting { .. } | Self::Failed { .. } => {
+                None
+            }
+        }
+    }
+
+    /// The version it is about, but while idle.
+    pub fn version(&self) -> Option<&str> {
+        match self {
+            Self::Idle => None,
+            Self::Downloading(downloading) => Some(&downloading.version),
+            Self::Ready { version, .. }
+            | Self::Held { version, .. }
+            | Self::Restarting { version }
+            | Self::Failed { version, .. } => Some(version),
+        }
+    }
+
+    /// Whether a download runs, or one is ready: no check runs meanwhile,
+    /// and no other download starts.
+    pub fn busy(&self) -> bool {
+        match self {
+            Self::Downloading(_)
+            | Self::Ready { .. }
+            | Self::Held { .. }
+            | Self::Restarting { .. } => true,
+            Self::Idle | Self::Failed { .. } => false,
+        }
+    }
+}
+
 /// What Settings › Updates and the signals elsewhere know.
 #[derive(Clone, Debug, Default)]
 pub struct Updates {
@@ -124,6 +264,8 @@ pub struct Updates {
     /// A check's answer that came before the detection did: taken in once
     /// it does, so the offer keeps this copy's kind's download.
     pub held: Option<(u64, Result<release::Latest, Failed>)>,
+    /// Where the download of the version offered stands.
+    pub install: Install,
 }
 
 impl Updates {
@@ -144,6 +286,15 @@ impl Updates {
     pub fn checking(&self) -> bool {
         matches!(self.phase, Phase::Checking { .. })
     }
+
+    /// The version offered, where this copy can download and install it:
+    /// it replaces itself, the release has its build, and this is no
+    /// Windows build.
+    pub fn installable(&self, skip: Option<&str>) -> Option<&Offer> {
+        let itself = matches!(self.installed, Some(Installed::Itself(_)));
+        self.offer(skip)
+            .filter(|offer| DOWNLOADS && itself && offer.download.is_some())
+    }
 }
 
 #[cfg(test)]
```

`src/update/download.rs`:

```diff
diff --git a/src/update/download.rs b/src/update/download.rs
index 14d5b04f..894ee171 100644
--- a/src/update/download.rs
+++ b/src/update/download.rs
@@ -16,6 +16,12 @@ pub enum Failure {
     /// Couldn't install: the helper that swaps the app did not start.
     /// fastframe-update's words.
     NotHanded(String),
+    /// Couldn't install: work was opened while the helper got ready, and
+    /// was not done before the helper gave up.
+    WorkBegan,
+    /// Couldn't install: the helper's answer came when it was too late to
+    /// quit in time (the window was hidden).
+    TooLate,
 }
 
 /// What did not verify.
@@ -139,6 +145,10 @@ impl Failure {
                 Undone::Other(_) => "{error}",
             },
             Self::NotHanded(_) => "The installer didn't start: {error}",
+            Self::WorkBegan => {
+                "Work was still open when the installer stopped waiting, so Tabletist didn't restart."
+            }
+            Self::TooLate => "Tabletist couldn't quit in time, so the installer stopped waiting.",
         }
     }
 
@@ -226,7 +236,7 @@ pub fn issue_url(version: &str, failure: &Failure, said: &str) -> String {
     let kind = match failure {
         Failure::Unverified(_) => "didn't verify",
         Failure::Undone(_) => "didn't download",
-        Failure::NotHanded(_) => "didn't install",
+        Failure::NotHanded(_) | Failure::WorkBegan | Failure::TooLate => "didn't install",
     };
     let title = format!("Update to {version} {kind}");
     let body = format!(
```

`src/update/relaunch.rs`:

```diff
diff --git a/src/update/relaunch.rs b/src/update/relaunch.rs
new file mode 100644
index 00000000..bc534484
--- /dev/null
+++ b/src/update/relaunch.rs
@@ -0,0 +1,141 @@
+//! What a restart to install an update keeps that the session does not:
+//! each window's open SQL editors, by their title's number, with their
+//! text, their mode and their place among the window's tabs, and which
+//! tab was in front. Written to `relaunch.json` in the state folder as the
+//! handoff begins, and read by the copy the helper starts (`--relaunch`),
+//! which deletes it then. Deleted too where the handoff does not end in
+//! the quit. A quit of any other kind writes none.
+
+use std::path::Path;
+
+use serde::{Deserialize, Serialize};
+
+/// The file.
+#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
+pub struct Relaunch {
+    /// The windows, in the order the bar had them.
+    #[serde(default)]
+    pub windows: Vec<Window>,
+}
+
+/// One window's SQL editors, in the order its tabs had them.
+#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
+pub struct Window {
+    /// The saved connection's id.
+    pub connection: String,
+    #[serde(default)]
+    pub editors: Vec<Editor>,
+    /// The tab in front, by its place among the window's tables and
+    /// editors.
+    #[serde(default, skip_serializing_if = "Option::is_none")]
+    pub front: Option<usize>,
+}
+
+/// A SQL editor: Query 2, its text, whether its runs may write, and its
+/// place among the window's tables and editors.
+#[derive(Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
+pub struct Editor {
+    pub number: u32,
+    #[serde(default)]
+    pub text: String,
+    #[serde(default)]
+    pub writes: bool,
+    #[serde(default)]
+    pub at: usize,
+}
+
+// Hand-written so the SQL text never reaches logs or panic messages.
+impl std::fmt::Debug for Editor {
+    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
+        formatter
+            .debug_struct("Editor")
+            .field("number", &self.number)
+            .field("text", &format_args!("{} bytes", self.text.len()))
+            .field("writes", &self.writes)
+            .field("at", &self.at)
+            .finish()
+    }
+}
+
+impl Relaunch {
+    /// The file at `path`, deleted once read: it is for one launch. An
+    /// empty one where there is none, or it cannot be read.
+    pub fn take(path: &Path) -> Self {
+        let read = crate::util::load_json(path);
+        if let Err(error) = std::fs::remove_file(path)
+            && error.kind() != std::io::ErrorKind::NotFound
+        {
+            log::warn!("could not delete {}: {error}", path.display());
+        }
+        read
+    }
+
+    /// The first window of `connection` not taken yet, taken: windows
+    /// asked for in the bar's order each get their own, where a
+    /// connection is open in two.
+    pub fn window_of(&mut self, connection: &str) -> Option<Window> {
+        let at = self
+            .windows
+            .iter()
+            .position(|window| window.connection == connection)?;
+        Some(self.windows.remove(at))
+    }
+}
+
+#[cfg(test)]
+mod tests {
+    use super::*;
+
+    fn kept() -> Relaunch {
+        Relaunch {
+            windows: vec![
+                Window {
+                    connection: "bookshop".into(),
+                    editors: vec![Editor {
+                        number: 2,
+                        text: "SELECT * FROM book_covers".into(),
+                        ..Editor::default()
+                    }],
+                    front: None,
+                },
+                Window {
+                    connection: "bookshop".into(),
+                    editors: vec![Editor {
+                        number: 1,
+                        text: "SELECT 1".into(),
+                        ..Editor::default()
+                    }],
+                    front: None,
+                },
+            ],
+        }
+    }
+
+    #[test]
+    fn the_file_is_read_once_and_deleted() {
+        let dir = tempfile::tempdir().unwrap();
+        let path = dir.path().join("relaunch.json");
+        crate::util::save_json(&path, &kept()).unwrap();
+        assert_eq!(Relaunch::take(&path), kept());
+        assert!(!path.exists());
+        assert_eq!(Relaunch::take(&path), Relaunch::default());
+    }
+
+    #[test]
+    fn each_window_of_a_connection_takes_its_own_editors() {
+        let mut kept = kept();
+        let number = |window: Option<Window>| window.map(|window| window.editors[0].number);
+        assert_eq!(number(kept.window_of("bookshop")), Some(2));
+        assert_eq!(number(kept.window_of("bookshop")), Some(1));
+        assert!(kept.window_of("bookshop").is_none());
+    }
+
+    /// A file a newer version wrote, with more in it, is read for what
+    /// this one knows.
+    #[test]
+    fn what_this_version_does_not_know_is_left() {
+        let text = r#"{"windows":[{"connection":"bookshop","editors":[{"number":3,"text":"SELECT 2","mode":"write"}],"front":1}],"version":2}"#;
+        let read: Relaunch = serde_json::from_str(text).unwrap();
+        assert_eq!(read.windows[0].editors[0].number, 3);
+    }
+}
```

- [ ] **Step 4: Run the checks**

Run: the checks of Task 1's Step 4.
Expected: clean; the library's tests `test result: ok. 3665 passed; 0 failed; 17 ignored`; `tests/cli.rs` passes.

- [ ] **Step 5: Commit**

```bash
git add AGENTS.md src/app.rs src/app/editing.rs src/app/startup_tests.rs src/app/updates.rs src/backend/updates.rs src/entrypoint.rs src/model.rs src/paths.rs src/update.rs src/update/download.rs src/update/relaunch.rs tests/cli.rs
git commit -m "Download and restart into an update, waiting for work and keeping the SQL editors"
```

### Task 3: The card's new states in both looks, the keys and the start screen

**Files:** `src/ui/settings/updates.rs`, `src/ui/settings/updates_terminal.rs`, `src/ui/settings/mod.rs`, `src/ui/settings/terminal.rs`, `src/ui/start.rs`, `src/ui/update_settings_tests.rs`, `src/keymap.rs`, `src/app.rs`, `src/app/editing.rs`, `src/app/updates.rs`, `src/model.rs`, `src/theme.rs`, `src/update.rs`, `src/update/release.rs`, `assets/icons/triangle-alert.svg`, the docs.

**The sheet's card** gets the new states, with the boards' copy:
- Downloading, with the bar, which is a progress indicator in MB;
- Ready;
- Work in progress, whose list is a list;
- Verification failed;
- Couldn't download, Couldn't install and Installing….

Download and install is Available's main button where this copy installs it. Release notes on GitHub stays the main button where it does not.

**The Omarchy block** has its key lines (row 14) and the components' progress row. `U`, bound but reserved in run 1, now acts. `ctrl+c` cancels the download on the Updates pane, only while one runs; otherwise the key is left. `:update`'s question is in the settings footer, with `y` and `n`, `N` or Esc (`When::UpdateQuestion`); `S` acts only where Skip is offered.

**The macOS start screen** says "0.2.0 is ready · Restart" while Ready.

- [ ] **Step 1: Write the failing tests**

`src/ui/settings/updates.rs`:

```diff
diff --git a/src/ui/settings/updates.rs b/src/ui/settings/updates.rs
index fe12b2f2..4bf2fab0 100644
--- a/src/ui/settings/updates.rs
+++ b/src/ui/settings/updates.rs
@@ -653,3 +653,26 @@ pub(super) fn announce_live(ui: &Ui, rect: Rect, (text, which): (&str, &str), li
         node.set_live(live);
     });
 }
+#[cfg(test)]
+mod tests {
+    use super::*;
+
+    fn running(secs: u64) -> String {
+        let work = Work {
+            tab: crate::model::ConnTabId(1),
+            id: crate::model::TabId(2),
+            window: "Bookshop · dev".into(),
+            what: Pending::Running { query: 2, secs },
+        };
+        work_said(&work, Locale::English)
+    }
+
+    /// A query a restart would stop, with how long it has run, each unit
+    /// through the catalog.
+    #[test]
+    fn a_running_query_says_how_long_it_has_run() {
+        assert_eq!(running(12), "query running for 12 s");
+        assert_eq!(running(72), "query running for 1 min 12 s");
+        assert_eq!(running(120), "query running for 2 min");
+    }
+}
```

`src/ui/update_settings_tests.rs`:

```diff
diff --git a/src/ui/update_settings_tests.rs b/src/ui/update_settings_tests.rs
index e2a0ce58..2a55f071 100644
--- a/src/ui/update_settings_tests.rs
+++ b/src/ui/update_settings_tests.rs
@@ -10,10 +10,11 @@ use crate::model::{Action, Dialog, SettingsPane};
 use crate::settings::UpdateCheck;
 use crate::testing::{Harness, node};
 use crate::theme::Look;
-use crate::update::Phase;
 use crate::update::check::Failed;
+use crate::update::download::{Failure, Undone, Unverified};
 use crate::update::installed::{Installed, Managed};
 use crate::update::release::{self, Kind, Latest, Offer, Os};
+use crate::update::{Downloading, Install, Phase};
 
 fn looks() -> [Look; 2] {
     [Look::macos(), Look::omarchy()]
@@ -462,8 +463,13 @@ fn the_terminal_keys_check_show_the_notes_and_skip() {
     assert_eq!(checks(&harness), 1);
     harness.app.apply(Action::CancelUpdateCheck);
     harness.app.updates.seen.offer = Some(offer());
-    // The block's keys, as the keymap writes them.
-    assert!(harness.has("n release notes  S skip 99.0.0"));
+    // The block's keys, as the keymap writes them: `U` where this copy
+    // downloads, which Windows does not.
+    let keys = match crate::update::DOWNLOADS {
+        true => "U update now  n release notes  S skip 99.0.0",
+        false => "n release notes  S skip 99.0.0",
+    };
+    assert!(harness.has(keys));
     crate::ui::tests::type_key(&mut harness, Key::N, "n");
     let shown = harness
         .app
@@ -667,3 +673,559 @@ fn a_failed_check_asked_for_is_shown_over_a_version_found_before() {
         assert!(!harness.has(&error), "{}", look.name);
     }
 }
+
+// The download and the restart.
+
+/// A harness offered 99.0.0, which this copy installs itself.
+fn installs(look: Look) -> Harness {
+    let mut harness = harness(look);
+    harness.app.updates.installed = Some(Installed::Itself(Kind::Portable));
+    harness.app.updates.seen.offer = Some(offer());
+    harness
+}
+
+fn downloads(harness: &Harness) -> usize {
+    harness
+        .app
+        .backend
+        .sent
+        .iter()
+        .filter(|sent| matches!(sent, Command::Updates(UpdatesCommand::Download { .. })))
+        .count()
+}
+
+/// 18.4 of 31.2 MB, at 2 MB a second for the last five seconds.
+fn downloading() -> Downloading {
+    let mut downloading = Downloading::new(7, "99.0.0".into(), 31_200_000);
+    for (secs, received) in [(4, 8_400_000), (9, 18_400_000)] {
+        downloading.progressed(received, 31_200_000, std::time::Duration::from_secs(secs));
+    }
+    downloading
+}
+
+fn ready() -> Install {
+    Install::Ready {
+        version: "99.0.0".into(),
+        handed: None,
+    }
+}
+
+#[test]
+fn available_offers_download_and_install_where_this_copy_installs_it() {
+    let mut harness = installs(Look::macos());
+    on_the_pane(&mut harness);
+    if !crate::update::DOWNLOADS {
+        // Windows: the release page, as before.
+        assert!(!harness.has("Download and install"));
+        return;
+    }
+    // The page is a link beside it now.
+    assert!(harness.has("Release notes on GitHub"));
+    harness.click("Download and install");
+    harness.settle();
+    assert_eq!(downloads(&harness), 1);
+    assert!(harness.has("Downloading 99.0.0…"));
+}
+
+#[test]
+fn a_copy_that_does_not_install_it_offers_the_page_alone() {
+    for installed in [
+        Installed::Elsewhere(Managed {
+            said: "Tabletist can't replace itself here",
+            command: None,
+        }),
+        Installed::Itself(Kind::Portable),
+    ] {
+        let mut harness = harness(Look::macos());
+        let mut offer = offer();
+        if matches!(installed, Installed::Itself(_)) {
+            // A release with no build for this computer.
+            offer.download = None;
+        }
+        harness.app.updates.installed = Some(installed);
+        harness.app.updates.seen.offer = Some(offer);
+        on_the_pane(&mut harness);
+        assert!(!harness.has("Download and install"));
+        assert!(harness.has("Release notes on GitHub"));
+    }
+}
+
+#[test]
+fn downloading_shows_how_much_came_and_the_time_left() {
+    for look in looks() {
+        let mut harness = installs(look);
+        harness.app.updates.install = Install::Downloading(downloading());
+        on_the_pane(&mut harness);
+        let title = said(&look, "Downloading 99.0.0…");
+        assert!(harness.has(&title), "{}", look.name);
+        let line = said(&look, "18.4 of 31.2 MB · about 10 s left");
+        let tree = harness.settle();
+        assert!(
+            crate::testing::labels(&tree)
+                .iter()
+                .any(|label| label.contains(&line)),
+            "{}: {:?}",
+            look.name,
+            crate::testing::labels(&tree)
+        );
+    }
+}
+
+/// The bar is a progress indicator, from 0 to the size in megabytes.
+#[test]
+fn the_progress_bar_tells_a_screen_reader_what_came_in_megabytes() {
+    for look in looks() {
+        let mut harness = installs(look);
+        harness.app.updates.install = Install::Downloading(downloading());
+        on_the_pane(&mut harness);
+        let tree = harness.settle();
+        let (_, bar) = tree
+            .nodes
+            .iter()
+            .find(|(_, node)| node.role() == Role::ProgressIndicator)
+            .unwrap_or_else(|| panic!("{}: no progress indicator", look.name));
+        assert_eq!(bar.min_numeric_value(), Some(0.0), "{}", look.name);
+        assert_eq!(bar.max_numeric_value(), Some(31.2), "{}", look.name);
+        assert_eq!(bar.numeric_value(), Some(18.4), "{}", look.name);
+        assert_eq!(bar.value(), Some("18.4 of 31.2 MB"), "{}", look.name);
+    }
+}
+
+#[test]
+fn a_download_says_when_it_waits_for_the_network_and_when_it_verifies() {
+    for look in looks() {
+        let mut harness = installs(look);
+        let mut waiting = downloading();
+        waiting.waiting = true;
+        harness.app.updates.install = Install::Downloading(waiting);
+        on_the_pane(&mut harness);
+        let line = said(&look, "Waiting for the network · 18.4 of 31.2 MB");
+        let has = |harness: &mut Harness, part: &str| {
+            let tree = harness.settle();
+            crate::testing::labels(&tree)
+                .iter()
+                .any(|label| label.contains(part))
+        };
+        assert!(has(&mut harness, &line), "{}", look.name);
+        let mut verifying = downloading();
+        verifying.progressed(31_200_000, 31_200_000, std::time::Duration::from_secs(14));
+        harness.app.updates.install = Install::Downloading(verifying);
+        assert!(
+            has(&mut harness, &said(&look, "Verifying…")),
+            "{}",
+            look.name
+        );
+    }
+}
+
+#[test]
+fn cancel_and_ctrl_c_stop_the_download() {
+    for look in looks() {
+        let mut harness = installs(look);
+        harness.app.updates.install = Install::Downloading(downloading());
+        on_the_pane(&mut harness);
+        if look.terminal {
+            assert!(harness.has(&said(&look, "Downloading 99.0.0…")));
+            harness.press(Key::C, Modifiers::CTRL);
+        } else {
+            harness.click("Cancel");
+        }
+        harness.settle();
+        assert_eq!(harness.app.updates.install, Install::Idle, "{}", look.name);
+        let cancelled = harness
+            .app
+            .backend
+            .sent
+            .iter()
+            .any(|sent| matches!(sent, Command::Updates(UpdatesCommand::CancelDownload)));
+        assert!(cancelled, "{}", look.name);
+    }
+}
+
+#[test]
+fn ready_offers_restart_now_and_later() {
+    let mut harness = installs(Look::macos());
+    harness.app.updates.install = ready();
+    on_the_pane(&mut harness);
+    assert!(harness.has("99.0.0 is ready to install"));
+    assert!(harness.has("Signature checked · restart to finish"));
+    harness.click("Later");
+    harness.settle();
+    assert!(harness.app.dialog.is_none());
+    assert_eq!(harness.app.updates.install, ready());
+    on_the_pane(&mut harness);
+    harness.click("Restart now");
+    harness.settle();
+    assert!(matches!(
+        harness.app.updates.install,
+        Install::Restarting { .. }
+    ));
+}
+
+/// The start screen on macOS has one line of it, and its Restart is
+/// Restart now. The Omarchy look has none.
+#[test]
+fn the_start_screen_tells_of_an_update_ready() {
+    for look in looks() {
+        let mut harness = installs(look);
+        let _tab = harness.connect_fake();
+        harness.app.updates.install = ready();
+        harness.settle();
+        let line = "99.0.0 is ready ·";
+        assert_eq!(harness.has(line), !look.terminal, "{}", look.name);
+        if !look.terminal {
+            harness.click("Restart");
+            harness.settle();
+            assert!(matches!(
+                harness.app.updates.install,
+                Install::Restarting { .. }
+            ));
+        }
+    }
+}
+
+/// `book_covers` with a cell changed, on a window named Bookshop.
+fn with_an_unsaved_edit(harness: &mut Harness) {
+    let (tab, id) = harness.book_covers();
+    harness.app.workspace_mut(tab).unwrap().name = "Bookshop".into();
+    harness.app.apply(Action::EditCell {
+        tab,
+        id,
+        cell: crate::model::CellPos { row: 0, col: 2 },
+        start: crate::model::EditStart::Replace("ebook".into()),
+    });
+    harness.app.apply(Action::CommitEdit {
+        tab,
+        id,
+        then: crate::model::Advance::Stay,
+    });
+}
+
+/// The items of the work list a screen reader hears.
+fn work_items(tree: &egui::accesskit::TreeUpdate, look: &Look) -> Vec<String> {
+    let node_of = |wanted: egui::accesskit::NodeId| {
+        tree.nodes
+            .iter()
+            .find(|(id, _)| *id == wanted)
+            .map(|(_, node)| node)
+            .expect("a node of the tree")
+    };
+    let list = node_of(node(tree, &said(look, "Work in progress"), Role::List).expect("the list"));
+    let mut items = Vec::new();
+    let mut open: Vec<egui::accesskit::NodeId> = list.children().to_vec();
+    while let Some(child) = open.pop() {
+        let child = node_of(child);
+        match child.role() {
+            Role::ListItem => {
+                let said = child.label().or(child.value()).unwrap_or_default();
+                items.push(said.to_owned());
+            }
+            _ => open.extend(child.children()),
+        }
+    }
+    items
+}
+
+#[test]
+fn work_in_progress_lists_what_a_restart_would_interrupt() {
+    for look in looks() {
+        let mut harness = installs(look);
+        with_an_unsaved_edit(&mut harness);
+        harness.app.updates.install = ready();
+        harness.app.apply(Action::RestartToUpdate);
+        harness.settle();
+        assert_eq!(pane(&harness), Some(SettingsPane::Updates), "{}", look.name);
+        assert!(
+            harness.has(&said(&look, "Finish your work first")),
+            "{}",
+            look.name
+        );
+        let tree = harness.settle();
+        // As the window's title names it: the terminal's tag is `DEV`.
+        let dev = crate::env::Environment::Dev.label(crate::env::Platform::of(&look));
+        assert_eq!(
+            work_items(&tree, &look),
+            [format!("Bookshop · {dev}: 1 unsaved edit in book_covers")],
+            "{}",
+            look.name
+        );
+    }
+}
+
+#[test]
+fn restart_when_done_becomes_dont_restart_and_show_them_shows_the_work() {
+    let mut harness = installs(Look::macos());
+    with_an_unsaved_edit(&mut harness);
+    let (tab, id) = {
+        let work = harness.app.work_in_progress();
+        (work[0].tab, work[0].id)
+    };
+    // Another window in front, on its picker.
+    harness.app.apply(Action::ShowConnections);
+    harness.app.updates.install = ready();
+    harness.app.apply(Action::RestartToUpdate);
+    harness.click("Restart when done");
+    harness.settle();
+    assert!(harness.has("Tabletist restarts once this is done:"));
+    harness.click("Don't restart");
+    harness.settle();
+    assert_eq!(harness.app.updates.install, ready());
+    harness.app.apply(Action::RestartToUpdate);
+    harness.click("Show them");
+    harness.settle();
+    assert!(harness.app.dialog.is_none());
+    // The window with the edit is in front, on its table.
+    assert_eq!(harness.app.active_tab_id(), tab);
+    assert_eq!(harness.app.workspace(tab).unwrap().active_tab, Some(id));
+}
+
+#[test]
+fn a_download_that_did_not_verify_says_what_failed_once() {
+    for look in looks() {
+        let mut harness = installs(look);
+        harness.app.updates.install = Install::Failed {
+            version: "99.0.0".into(),
+            failure: Failure::Unverified(Unverified::Signature),
+        };
+        on_the_pane(&mut harness);
+        assert!(
+            harness.has(&said(&look, "The 99.0.0 download didn't verify")),
+            "{}",
+            look.name
+        );
+        let error = said(
+            &look,
+            "The signature doesn't match Tabletist's release key. The file was deleted.",
+        );
+        let tree = harness.settle();
+        let id = node(&tree, &error, Role::Label).expect("the error");
+        let told = tree.nodes.iter().find(|(at, _)| *at == id).unwrap();
+        assert_eq!(told.1.live(), Some(Live::Assertive), "{}", look.name);
+        if !look.terminal {
+            assert!(harness.has("Nothing was installed. Your copy is unchanged."));
+            harness.click("Report on GitHub");
+            harness.settle();
+            let opened = harness.app.url_to_open.take().unwrap_or_default();
+            assert!(
+                opened.starts_with("https://github.com/igor-alexandrov/tabletist/issues/new?"),
+                "{opened}"
+            );
+            harness.click("Download again");
+            harness.settle();
+            // Windows downloads nothing (row 16).
+            assert_eq!(downloads(&harness), usize::from(crate::update::DOWNLOADS));
+        }
+    }
+}
+
+/// A network that went is no bad file: Couldn't download, and Try again.
+#[test]
+fn a_download_that_never_came_whole_is_couldnt_download() {
+    let mut harness = installs(Look::macos());
+    harness.app.updates.install = Install::Failed {
+        version: "99.0.0".into(),
+        failure: Failure::Undone(Undone::Lost),
+    };
+    on_the_pane(&mut harness);
+    assert!(harness.has("Couldn't download 99.0.0"));
+    assert!(harness.has("The connection was lost and didn't come back."));
+    assert!(!harness.has("Report on GitHub"));
+    harness.click("Try again");
+    harness.settle();
+    // Windows downloads nothing (row 16).
+    assert_eq!(downloads(&harness), usize::from(crate::update::DOWNLOADS));
+}
+
+/// `U` does what the block's key line says: update now, restart now,
+/// restart when done.
+#[test]
+fn u_downloads_then_restarts_in_the_terminal_look() {
+    if !crate::update::DOWNLOADS {
+        return;
+    }
+    let mut harness = installs(Look::omarchy());
+    with_an_unsaved_edit(&mut harness);
+    on_the_pane(&mut harness);
+    assert!(harness.has("U update now  n release notes  S skip 99.0.0"));
+    capital(&mut harness, Key::U);
+    assert_eq!(downloads(&harness), 1);
+    harness.app.updates.install = ready();
+    assert!(harness.has("U restart now"));
+    capital(&mut harness, Key::U);
+    // The edit holds it.
+    assert!(harness.has("U restart when done"));
+    let tree = harness.settle();
+    assert_eq!(
+        work_items(&tree, &Look::omarchy()),
+        ["Bookshop · DEV: 1 unsaved edit in book_covers"]
+    );
+    capital(&mut harness, Key::U);
+    assert!(matches!(
+        harness.app.updates.install,
+        Install::Held {
+            when_done: true,
+            ..
+        }
+    ));
+    // The edit goes: the restart does.
+    let work = harness.app.work_in_progress();
+    let (tab, id) = (work[0].tab, work[0].id);
+    harness.app.apply(Action::DiscardEdits { tab, id });
+    harness.settle();
+    let handed = harness
+        .app
+        .backend
+        .sent
+        .iter()
+        .any(|sent| matches!(sent, Command::Updates(UpdatesCommand::Install { .. })));
+    assert!(handed);
+    assert!(matches!(
+        harness.app.updates.install,
+        Install::Restarting { .. }
+    ));
+}
+
+/// `S` is the block's only while it offers the version: with the update
+/// ready it is left, and the version still installs.
+#[test]
+fn s_skips_nothing_while_an_update_is_ready() {
+    let mut harness = installs(Look::omarchy());
+    harness.app.updates.install = ready();
+    on_the_pane(&mut harness);
+    assert!(!harness.has("S skip 99.0.0"));
+    capital(&mut harness, Key::S);
+    assert_eq!(harness.app.settings.update_skip, None);
+    assert_eq!(harness.app.updates.install, ready());
+}
+
+/// The work listed went before Restart when done was chosen: the card
+/// is Ready again, and says nothing would be interrupted.
+#[test]
+fn a_work_list_that_empties_shows_ready_again() {
+    let mut harness = installs(Look::macos());
+    with_an_unsaved_edit(&mut harness);
+    harness.app.updates.install = ready();
+    harness.app.apply(Action::RestartToUpdate);
+    assert!(harness.has("Restart now would interrupt:"));
+    let work = harness.app.work_in_progress();
+    let (tab, id) = (work[0].tab, work[0].id);
+    harness.app.apply(Action::DiscardEdits { tab, id });
+    harness.settle();
+    assert!(!harness.has("Restart now would interrupt:"));
+    assert!(harness.has("99.0.0 is ready to install"));
+}
+
+/// The window turns ctrl+c into a copy on Linux and Windows: that too
+/// cancels the download.
+#[test]
+fn ctrl_c_as_the_window_sends_it_cancels_the_download() {
+    let mut harness = installs(Look::omarchy());
+    harness.app.updates.install = Install::Downloading(downloading());
+    on_the_pane(&mut harness);
+    harness.frame(vec![
+        egui::Event::ModifiersChanged(Modifiers::CTRL),
+        egui::Event::Copy,
+    ]);
+    harness.frame(vec![egui::Event::ModifiersChanged(Modifiers::NONE)]);
+    harness.settle();
+    assert_eq!(harness.app.updates.install, Install::Idle);
+}
+
+/// Where the helper waits already, Ready says a quit installs it.
+#[test]
+fn ready_after_the_handoff_says_a_quit_installs_it() {
+    let mut harness = installs(Look::macos());
+    harness.app.updates.install = Install::Ready {
+        version: "99.0.0".into(),
+        handed: Some(std::time::Instant::now()),
+    };
+    on_the_pane(&mut harness);
+    assert!(harness.has("Signature checked · quitting Tabletist now installs it"));
+}
+
+/// `:update` checks; a version this copy installs is asked about in the
+/// footer, and `y` downloads it, to restart once it is ready.
+#[test]
+fn update_on_the_prompt_asks_before_installing() {
+    if !crate::update::DOWNLOADS {
+        return;
+    }
+    let mut harness = harness(Look::omarchy());
+    harness.app.updates.installed = Some(Installed::Itself(Kind::Portable));
+    let tab = harness.connect_fake();
+    harness.app.workspace_mut(tab).unwrap().command = Some("update".into());
+    harness.app.apply(Action::RunCommand(tab));
+    harness.settle();
+    let asked = harness
+        .app
+        .backend
+        .sent
+        .iter()
+        .rev()
+        .find_map(|sent| match sent {
+            Command::Updates(UpdatesCommand::Check { asked, .. }) => Some(*asked),
+            _ => None,
+        })
+        .expect("a check");
+    let json = crate::update::check::tests::RELEASE.replace("v0.4.0", "v99.0.0");
+    let mut latest: Latest = serde_json::from_str(&json).unwrap();
+    // A build of this computer's, whatever it is.
+    let name = release::asset_name("99.0.0", Os::current(), Some(Kind::Portable))
+        .unwrap_or_else(|| "tabletist.tar.gz".into());
+    latest.assets.push(release::Asset {
+        name,
+        size: 31_200_000,
+    });
+    harness
+        .app
+        .apply(Action::Backend(crate::backend::Event::Updates(
+            crate::backend::UpdatesEvent::Checked {
+                asked,
+                result: Ok(latest),
+            },
+        )));
+    assert!(harness.has("install 99.0.0 and restart?"));
+    assert!(harness.has("[y] yes"));
+    assert!(harness.has("[N] no"));
+    crate::ui::tests::type_key(&mut harness, Key::Y, "y");
+    assert_eq!(downloads(&harness), 1);
+    assert!(matches!(
+        &harness.app.updates.install,
+        Install::Downloading(downloading) if downloading.then_restart
+    ));
+    assert!(!harness.has("install 99.0.0 and restart?"));
+}
+
+#[test]
+fn no_to_the_question_installs_nothing() {
+    if !crate::update::DOWNLOADS {
+        return;
+    }
+    for (key, text) in [(Key::N, Some("n")), (Key::Escape, None)] {
+        let mut harness = installs(Look::omarchy());
+        harness.app.updates.install = ready();
+        let tab = harness.connect_fake();
+        harness.app.workspace_mut(tab).unwrap().command = Some("update".into());
+        harness.app.apply(Action::RunCommand(tab));
+        assert!(harness.has("install 99.0.0 and restart?"));
+        match text {
+            Some(text) => crate::ui::tests::type_key(&mut harness, key, text),
+            None => harness.press(key, Modifiers::NONE),
+        }
+        harness.settle();
+        assert_eq!(harness.app.updates.install, ready(), "{key:?}");
+        // The question went, and the pane stays.
+        assert_eq!(pane(&harness), Some(SettingsPane::Updates), "{key:?}");
+        assert!(!harness.has("install 99.0.0 and restart?"), "{key:?}");
+    }
+}
+
+/// The dot on Updates tells of an update ready too.
+#[test]
+fn the_dot_tells_of_an_update_ready() {
+    let mut harness = installs(Look::macos());
+    harness.app.settings.update_skip = Some("99.0.0".into());
+    harness.app.updates.install = ready();
+    on_the_pane(&mut harness);
+    assert!(harness.has("Update available"));
+}
```

`src/update/release.rs`:

```diff
diff --git a/src/update/release.rs b/src/update/release.rs
index 20bf85af..f1863aa6 100644
--- a/src/update/release.rs
+++ b/src/update/release.rs
@@ -400,6 +400,7 @@ mod tests {
 
     #[test]
     fn a_size_is_written_in_megabytes_to_a_tenth() {
+        assert_eq!(megabytes_figure(18_400_000), "18.4");
         assert_eq!(megabytes(31_200_000), "31.2 MB");
         assert_eq!(megabytes(29_849_999), "29.8 MB");
     }
```

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test --locked --workspace --all-targets --no-run`
Expected: no build, 4 errors, such as:

```
error[E0422]: cannot find struct, variant or union type `Work` in this scope
error[E0433]: cannot find type `Pending` in this scope
```

- [ ] **Step 3: Write the code**

`assets/icons/triangle-alert.svg`:

```diff
diff --git a/assets/icons/triangle-alert.svg b/assets/icons/triangle-alert.svg
new file mode 100644
index 00000000..9f04ae20
--- /dev/null
+++ b/assets/icons/triangle-alert.svg
@@ -0,0 +1,15 @@
+<svg
+  xmlns="http://www.w3.org/2000/svg"
+  width="24"
+  height="24"
+  viewBox="0 0 24 24"
+  fill="none"
+  stroke="#ffffff"
+  stroke-width="2"
+  stroke-linecap="round"
+  stroke-linejoin="round"
+>
+  <path d="m21.73 18-8-14a2 2 0 0 0-3.48 0l-8 14A2 2 0 0 0 4 21h16a2 2 0 0 0 1.73-3" />
+  <path d="M12 9v4" />
+  <path d="M12 17h.01" />
+</svg>
```

`docs/_guide/download.md`:

```diff
diff --git a/docs/_guide/download.md b/docs/_guide/download.md
index 76fbf8c5..5a256e07 100644
--- a/docs/_guide/download.md
+++ b/docs/_guide/download.md
@@ -91,7 +91,11 @@ sudo install -Dm755 tabletist /usr/local/bin/tabletist
 To update, install the new binary the same way: **Settings › Updates**
 (`:update` in the Omarchy look) checks GitHub for a newer version, every
 day unless you choose every week or only when you ask, and says when
-there is one. To remove Tabletist, delete the three files.
+there is one. An AppImage, or `tabletist` run from the unpacked archive's
+folder (beside the archive's `tabletist-portable.txt`), installs it
+itself with **Download and install** (`U`) once you choose to restart; a
+binary copied elsewhere, as above, is installed again the same way. To
+remove Tabletist, delete the three files.
 
 The Linux binaries are built on Ubuntu 24.04 and need glibc 2.39 or newer
 (Ubuntu 24.04, Debian 13, Fedora 40 or later). On an older system,
```

`docs/_guide/macos.md`:

```diff
diff --git a/docs/_guide/macos.md b/docs/_guide/macos.md
index 578289f1..d746a751 100644
--- a/docs/_guide/macos.md
+++ b/docs/_guide/macos.md
@@ -49,7 +49,10 @@ launch, are on the
   link to the source code and the licenses of the fonts it bundles, and
   **Tabletist → Settings…** opens Settings. **Tabletist → Check for
   Updates…** opens Settings › Updates and asks GitHub for a newer
-  version. **View → History** shows the
+  version. **Download and install** there downloads and checks it; once
+  it is ready, the start screen says so too, and **Restart now** installs
+  it, or, with work open, **Restart when done** does once that work is
+  done. **View → History** shows the
   window's [History tab]({% link _guide/sql-editor.md %}#history), and
   **View → Quick Open** opens quick open. A menu greys the commands that
   do not apply now, while it is open. Its keys do what they do where you
```

`docs/_guide/omarchy.md`:

```diff
diff --git a/docs/_guide/omarchy.md b/docs/_guide/omarchy.md
index b8d5d838..d4d448f3 100644
--- a/docs/_guide/omarchy.md
+++ b/docs/_guide/omarchy.md
@@ -49,7 +49,7 @@ second Esc gives the keyboard back.
 | `:keys default` | Open every key in force in your editor, as `settings.toml` spells them, commented: a file to copy lines from, written again each time and never read |
 | `:keys cheatsheet` | Save the keys in force as a Markdown cheat sheet, where you say |
 | `:set calendar`, `:set nocalendar` | Show or hide the [calendar]({% link _guide/editing-data.md %}#the-calendar) under a date's editor, saved as its switch in Settings is |
-| `:update` | Show Settings › Updates, and check for a newer version there |
+| `:update` | Show Settings › Updates, and check for a newer version there. Where Tabletist can install it, the footer asks `install 0.4.0 and restart?`: `y` downloads it and restarts once it is ready and no window has work open; `n`, `N` or Esc does not |
 | `?` | The list of keys |
 
 Ctrl+P opens quick open from any pane: a picker at the top of the
@@ -364,6 +364,8 @@ stopped a run, `t` runs it again without one. Times read "45s", "1m" and
 | `c` | On the Updates pane: check for a newer version now |
 | `n` | On the Updates pane, while a version is offered: its release notes in `$PAGER` (`less` without one), in a terminal; where none can be started, its page in the browser |
 | `S` | On the Updates pane: skip the version offered |
+| `U` | On the Updates pane: download and install the version offered; once it is ready, restart into it; with work open, restart when it is done, or, pressed again, don't; after a download or an install that failed, download it again |
+| Ctrl+C | On the Updates pane: cancel the download |
 | Ctrl+E | Open `settings.toml` in your editor |
 | `R` | Reset the option. The agent server's switch is left as it is |
 | `D`, `n` | On the Editor pane, once keep is set to don't keep: delete the history kept, or keep it |
```

`docs/_reference/settings-and-files.md`:

```diff
diff --git a/docs/_reference/settings-and-files.md b/docs/_reference/settings-and-files.md
index 25465f65..2bb2ff45 100644
--- a/docs/_reference/settings-and-files.md
+++ b/docs/_reference/settings-and-files.md
@@ -97,16 +97,70 @@ hold, and shows how to connect an agent:
 The last pane, **Updates**, says whether a newer version is out, and sets
 how often Tabletist asks. Its card shows one of: not checked yet; up to date (with when
 the last check ran); checking; a version available, with what is new in
-it (the first four points of its release notes), its size and **Release
-notes on GitHub** (its page has the downloads), and **Skip this
-version**; that a release has no build for this computer yet; that
-another tool updates this copy (Homebrew, cargo or your package manager),
-with the command to run and **Copy**; or that the check failed, and why.
-While a version is offered, Updates has a dot in the window's list of
-panes (Omarchy: a `●` after `updates`), and the Omarchy look's status
-line says `0.4.0 available · :update` for 8 seconds, once a day at most.
-Nothing opens over your work. Tabletist does not download or install the
-update itself yet.
+it (the first four points of its release notes), its size, **Download
+and install**, **Release notes on GitHub** and **Skip this version**;
+that a release has no build for this computer yet; that another tool
+updates this copy (Homebrew, cargo or your package manager), with the
+command to run and **Copy**; or that the check failed, and why. Where
+Tabletist cannot replace itself (another tool's copy, a release with no
+build for this computer, and Windows, which updates from the release
+page), **Release notes on GitHub** takes the place of **Download and
+install**. While a version is offered, Updates has a dot in the window's
+list of panes (Omarchy: a `●` after `updates`), and the Omarchy look's
+status line says `0.4.0 available · :update` for 8 seconds, once a day
+at most. Nothing opens over your work.
+
+**Download and install** (Omarchy: `U`) downloads the version offered
+beside the app, in a folder of its own (`.tabletist-update-…`, deleted on
+a cancel or a failure, and by the next download), and the card shows how
+much came and about how long is left, with **Cancel** (Omarchy: Ctrl+C).
+When the network goes, or is not there as the download starts, the card
+says **Waiting for the network** and the download tries again every 2
+seconds for 10 minutes, then goes on where it stopped; a connection that
+sends nothing for 8 seconds counts as gone. Before anything is
+kept it checks the release's signed `checksums.txt`, then the file's
+checksum and that the new version starts; on macOS also that it is
+Tabletist, signed by its developer, and approved by Gatekeeper. A file
+that fails is deleted and nothing is installed: **The 0.4.0 download
+didn't verify** says what failed, with **Download again** and **Report on
+GitHub**, which opens a new issue with the versions, the system and the
+error filled in (nothing is sent until you submit it there). A download
+that never came whole says **Couldn't download 0.4.0**, with **Try
+again**.
+
+Once it verified, the card says **0.4.0 is ready to install**, with
+**Restart now** (Omarchy: `U`) and **Later**, which closes Settings and
+keeps it ready; on macOS the start screen says `0.4.0 is ready ·
+Restart`. It installs only when you choose Restart now: a quit does not
+install it, and the next launch offers it again. Restart now first looks
+at every window, and the restart waits for what it would interrupt:
+unsaved edits in a table, a save running, a table's page, count or
+structure being read, a write running in a SQL editor, and any other
+query an editor runs. With any of them, or another dialog open, the card
+says **Finish your work first** and lists them (`Bookshop · dev: 3
+unsaved edits in book_covers`, `Bookshop · dev: query running for 1 min
+12 s`), with **Restart when done** (Omarchy: `U`), which restarts as soon
+as the list is empty, and **Show them**, which closes Settings and
+brings the first window with work forward, on its tab. Nothing is
+dropped to restart. Once Restart when done is chosen, the card says
+`Tabletist restarts once this is done:`, and **Don't restart** (Omarchy:
+`U` again) takes it back.
+
+While the restart begins, the card says **Installing 0.4.0…**. Tabletist
+writes `relaunch.json`, hands the update to its installer, quits, and
+the new version takes its place and opens the windows you had, with
+their tables, filters and SQL editors, whatever **On launch** and
+**Reopen tabs** say. The installer waits a minute for Tabletist to quit.
+Work you open in the seconds before it is ready is waited for as above
+while that minute lasts, and a quit inside it still installs the update;
+after it, nothing installs, and the card says **Couldn't install 0.4.0**,
+with **Try again**. It says so too where the installer did not start.
+`relaunch.json` is then deleted, as it is by the new version once read.
+
+`:update` in the Omarchy look shows the pane and checks; where it finds a
+version Tabletist can install, the footer asks `install 0.4.0 and
+restart? [y] yes [N] no`. `y` downloads it and restarts once it is ready,
+with the same look for work first; `n`, `N` or Esc leaves it offered.
 
 | Option | Choices | Default |
 | --- | --- | --- |
@@ -385,6 +439,7 @@ An administrator's policy, `managed.toml`, is in a third place of its own:
 | `agents_shown.json` | State | What agents may do as Tabletist last showed it: whether the server is on, and each connection's access, hidden tables and masked columns |
 | `updates.json` | State | What the checks for updates found: when the last one that worked ran, the newest version seen, that release's page and notes while it is newer than this version, and when the Omarchy status line last told of it |
 | `release-notes-<version>.md` | State | A release's notes, written for `$PAGER` when `n` shows them in the Omarchy look |
+| `relaunch.json` | State | Each window's SQL editors (their number, text, mode and place among the tabs) and the tab in front, written as Restart now hands an update to its installer, for the new version to reopen. That version deletes it once read, and so does Tabletist where the installer does not start or gives up |
 | `mcp.sock`, `mcp.key`, `mcp.lock` | Runtime, else State | Where `tabletist mcp` reaches the app, the key each proves it holds before either says more, and the lock the app holds while it listens. The socket and the key are there while agents are on |
 | `mcp.pipe` | State | On Windows, in the socket's place: the name of the pipe `tabletist mcp` reaches the app by, new each time agents are turned on |
 
```

`src/app.rs`:

```diff
diff --git a/src/app.rs b/src/app.rs
index 1113664c..f578b1a9 100644
--- a/src/app.rs
+++ b/src/app.rs
@@ -48,6 +48,7 @@ pub use quick_open::{
     Found, GROUP_SHOWN, QuickCommand, QuickHeading, QuickItem, QuickList, QuickRow, StartRow,
     setting_path,
 };
+pub use updates::{Pending, Work};
 
 /// What a keyring read is for; each names the exact request it serves, so
 /// a late answer for an earlier attempt is dropped.
@@ -2589,7 +2590,9 @@ impl App {
             | Action::DontRestart
             | Action::UpdateLater
             | Action::ShowWork
-            | Action::ReportUpdateFailure => self.update_action(action),
+            | Action::ReportUpdateFailure
+            | Action::UpdateNow
+            | Action::AnswerUpdateQuestion(_) => self.update_action(action),
             Action::ChooseAgentSetup(agent) => {
                 if let Some(Dialog::Settings(dialog)) = &mut self.dialog {
                     dialog.agent = agent;
```

`src/app/editing.rs`:

```diff
diff --git a/src/app/editing.rs b/src/app/editing.rs
index 7f383e73..c6d17c2d 100644
--- a/src/app/editing.rs
+++ b/src/app/editing.rs
@@ -1941,8 +1941,8 @@ impl App {
             // The keys in force: the full list in the editor, or a cheat
             // sheet where the person says. Neither touches the settings.
             (Typed::Command(Command::KeysDefault), _) => Action::KeysDefault,
-            // Settings › Updates, checking.
-            (Typed::Command(Command::CheckForUpdates), _) => Action::ShowUpdates,
+            // Settings › Updates, checking, then asking to install.
+            (Typed::Command(Command::CheckForUpdates), _) => Action::UpdateNow,
             (Typed::Command(Command::KeysCheatsheet), _) => Action::ExportCheatSheet,
             // The calendar under a date's editor, saved as its switch in
             // Settings saves it.
```

`src/app/updates.rs`:

```diff
diff --git a/src/app/updates.rs b/src/app/updates.rs
index 833246f8..d22491b9 100644
--- a/src/app/updates.rs
+++ b/src/app/updates.rs
@@ -239,6 +239,8 @@ impl App {
             // Ready stays: the next Restart now installs it.
             Action::UpdateLater => self.close_settings(),
             Action::ShowWork => self.show_work(),
+            Action::UpdateNow => self.update_now(),
+            Action::AnswerUpdateQuestion(yes) => self.answer_update_question(yes),
             Action::ReportUpdateFailure => {
                 if let Install::Failed { version, failure } = &self.updates.install {
                     let said = failure.with_parts(failure.said());
@@ -248,6 +250,7 @@ impl App {
             // The schedule's next check waits an interval from it, as
             // from one that failed.
             Action::CancelUpdateCheck => {
+                self.updates.ask_after_check = false;
                 if self.updates.checking() {
                     self.updates.phase = Phase::Idle;
                     self.updates.tried = Some(self.now_secs());
@@ -386,6 +389,61 @@ impl App {
             }));
     }
 
+    /// `:update`: the pane, checking. In the terminal look a version this
+    /// copy can install is asked about in the footer: the one downloading
+    /// or ready at once, else the one the check finds.
+    fn update_now(&mut self) {
+        self.apply(Action::ShowUpdates);
+        if !self.look.terminal {
+            return;
+        }
+        let version = match &self.updates.install {
+            Install::Downloading(downloading) => Some(downloading.version.clone()),
+            Install::Ready { version, .. } => Some(version.clone()),
+            Install::Held { .. } | Install::Restarting { .. } => None,
+            Install::Idle | Install::Failed { .. } => {
+                self.updates.ask_after_check = self.updates.checking();
+                None
+            }
+        };
+        if let Some(version) = version {
+            self.ask_to_install(version);
+        }
+    }
+
+    /// Asks in the Updates pane's footer whether to install `version` and
+    /// restart.
+    fn ask_to_install(&mut self, version: String) {
+        if let Some(Dialog::Settings(dialog)) = &mut self.dialog
+            && dialog.pane == SettingsPane::Updates
+        {
+            dialog.update_question = Some(version);
+        }
+    }
+
+    /// The footer's answer: yes downloads, then restarts once the
+    /// download is ready, through the same check for work as Restart now.
+    fn answer_update_question(&mut self, yes: bool) {
+        let asked = match &mut self.dialog {
+            Some(Dialog::Settings(dialog)) => dialog.update_question.take(),
+            _ => None,
+        };
+        if asked.is_none() || !yes {
+            return;
+        }
+        if let Install::Idle | Install::Failed { .. } = self.updates.install {
+            self.download_update();
+        }
+        match &mut self.updates.install {
+            Install::Downloading(downloading) => downloading.then_restart = true,
+            Install::Ready { .. } => self.restart_to_update(),
+            Install::Idle
+            | Install::Held { .. }
+            | Install::Restarting { .. }
+            | Install::Failed { .. } => {}
+        }
+    }
+
     /// Whether the app may quit for the restart now: no window has work
     /// open, and no dialog but Settings is up, which a quit would drop.
     fn may_restart(&self) -> bool {
@@ -626,6 +684,13 @@ impl App {
             Found::Newer(_) | Found::UpToDate => None,
         };
         self.save_updates();
+        if std::mem::take(&mut self.updates.ask_after_check) {
+            let skip = self.settings.update_skip.as_deref();
+            if let Some(offer) = self.updates.installable(skip) {
+                let version = offer.version.clone();
+                self.ask_to_install(version);
+            }
+        }
     }
 
     /// In the terminal look, the status line tells of a version offered,
@@ -646,6 +711,7 @@ impl App {
     /// A check that got no answer: it does not count as checked, and the
     /// schedule tries again an interval later, whoever asked.
     fn check_failed(&mut self, failed: Failed, asked_for: bool, now: u64) {
+        self.updates.ask_after_check = false;
         self.updates.tried = Some(now);
         self.updates.phase = Phase::Failed { failed, asked_for };
     }
```

`src/keymap.rs`:

```diff
diff --git a/src/keymap.rs b/src/keymap.rs
index 90bd9df4..80f80279 100644
--- a/src/keymap.rs
+++ b/src/keymap.rs
@@ -242,6 +242,9 @@ pub enum When {
     Leaving,
     /// The question before a run that changes every row of a table.
     EveryRow,
+    /// The settings footer asks whether to install an update and restart,
+    /// as `:update` has it.
+    UpdateQuestion,
     /// The cursor's row is a new one, not saved yet: the row panel is its
     /// form.
     NewRow,
@@ -684,6 +687,9 @@ commands! {
     SettingsReleaseNotes = "settings_release_notes",
     SettingsSkipVersion = "settings_skip_version",
     SettingsInstallUpdate = "settings_install_update",
+    SettingsCancelDownload = "settings_cancel_download",
+    SettingsInstallYes = "settings_install_yes",
+    SettingsInstallNo = "settings_install_no",
     SettingsUnbind = "settings_unbind",
     SettingsFilter = "settings_filter",
     SettingsFindKey = "settings_find_key",
@@ -1090,13 +1096,20 @@ impl Command {
             C::SettingsSkipVersion => {
                 info("Skip the version offered, in the settings", "", Prompts)
             }
-            // Downloading and installing come in the next run.
             C::SettingsInstallUpdate => info(
-                "Download and install the version offered, in the settings",
+                "Download and install the version offered, or restart into it, in the settings",
                 "",
                 Prompts,
-            )
-            .reserved(),
+            ),
+            C::SettingsCancelDownload => {
+                info("Cancel the update's download, in the settings", "", Prompts)
+            }
+            C::SettingsInstallYes => info(
+                "Install the update and restart, as :update asks",
+                "",
+                Prompts,
+            ),
+            C::SettingsInstallNo => info("Keep this version, as :update asks", "", Prompts),
             C::SettingsDeleteHistory => info(
                 "Delete the query history kept, as the settings ask",
                 "",
@@ -2374,6 +2387,29 @@ pub const BINDINGS: &[Binding] = {
         bind(C::SettingsReleaseNotes, PROMPT, NONE, keys(&[k("n")])).when(When::UpdatesPane),
         bind(C::SettingsSkipVersion, PROMPT, NONE, keys(&[k("S")])).when(When::UpdatesPane),
         bind(C::SettingsInstallUpdate, PROMPT, NONE, keys(&[k("U")])).when(When::UpdatesPane),
+        bind(
+            C::SettingsCancelDownload,
+            PROMPT,
+            NONE,
+            keys(&[k("ctrl+c")]),
+        )
+        .when(When::UpdatesPane),
+        // `:update`'s question, as the components' confirm row has it:
+        // `N` the answer Esc gives.
+        bind(
+            C::SettingsInstallYes,
+            PROMPT,
+            NONE,
+            keys(&[k("y"), k("Y")]).shown("y"),
+        )
+        .when(When::UpdateQuestion),
+        bind(
+            C::SettingsInstallNo,
+            PROMPT,
+            NONE,
+            keys(&[k("n"), k("N"), k("esc")]).shown("N"),
+        )
+        .when(When::UpdateQuestion),
         // The Keyboard pane's own: Enter records the row (the screen's
         // Enter), `R` resets it, and the arrows' letters move as on any
         // pane.
```

`src/model.rs`:

```diff
diff --git a/src/model.rs b/src/model.rs
index 70c5ea22..f6b1f1e3 100644
--- a/src/model.rs
+++ b/src/model.rs
@@ -995,8 +995,15 @@ pub enum Action {
     /// skip is offered by a check asked for.
     CheckForUpdates,
     /// Show Settings › Updates and check there now: the menu's Check for
-    /// Updates…, quick open's, `:update`.
+    /// Updates…, quick open's.
     ShowUpdates,
+    /// `:update`: [`Action::ShowUpdates`], and in the terminal look, where
+    /// the check finds a version this copy can install, the footer asks
+    /// whether to install it and restart.
+    UpdateNow,
+    /// The answer to that question: download it, then restart once it is
+    /// ready, as Restart now does; or not.
+    AnswerUpdateQuestion(bool),
     /// Drop the check under way: what it brings is not read.
     CancelUpdateCheck,
     /// Stop offering the version offered: `updates.skip`.
@@ -2939,6 +2946,9 @@ pub struct SettingsDialog {
     /// A question of the query history's waits for its answer, where the
     /// footer asks it as it asks Reset's.
     pub history_question: Option<HistoryQuestion>,
+    /// `:update` asks whether to install this version and restart: the
+    /// Updates pane's footer asks it, as it asks the history's.
+    pub update_question: Option<String>,
     /// The agent Settings › Agents says how to connect.
     pub agent: crate::endpoint::Agent,
     /// The option quick open opened the window at: the sheet draws its row
```

`src/theme.rs`:

```diff
diff --git a/src/theme.rs b/src/theme.rs
index b7c348d0..ea8e17c6 100644
--- a/src/theme.rs
+++ b/src/theme.rs
@@ -972,6 +972,7 @@ fastframe_icons::icons! {
         ShieldAlert => "shield-alert",
         SquareTerminal => "square-terminal",
         Table => "table-2",
+        TriangleAlert => "triangle-alert",
         WifiOff => "wifi-off",
     }
 }
```

`src/ui/settings/mod.rs`:

```diff
diff --git a/src/ui/settings/mod.rs b/src/ui/settings/mod.rs
index 924f9b2e..6955b895 100644
--- a/src/ui/settings/mod.rs
+++ b/src/ui/settings/mod.rs
@@ -664,10 +664,15 @@ enum Asked {
     Copy,
     /// Answer the history's question: delete what it kept, or keep it.
     History(bool),
-    /// The Updates pane's: check now, the notes, skip the version offered.
+    /// The Updates pane's: check now, the notes, skip the version offered,
+    /// `U` (download, restart), and cancel the download.
     CheckNow,
     Notes,
     Skip,
+    Install,
+    CancelDownload,
+    /// Answer `:update`'s question: install and restart, or not.
+    Answer(bool),
 }
 
 /// What `event` asks of the screen, if it is a press of one of its keys,
@@ -681,19 +686,26 @@ fn asked(
     keymap: &Keymap,
     event: &egui::Event,
     held: egui::Modifiers,
-    (free, pane): (bool, SettingsPane),
+    (free, pane, question): (bool, SettingsPane, bool),
 ) -> Option<Asked> {
     // The screen is Omarchy's.
     let layout = Layout::Omarchy;
     // The Updates pane's own come first there, and are read nowhere else:
     // its `n` is the notes, which the history's question has elsewhere.
+    // While `:update` asks, its answers come before them: Esc and `n`
+    // say no.
     let updates = [
+        Command::SettingsInstallYes,
+        Command::SettingsInstallNo,
         Command::SettingsCheckNow,
         Command::SettingsReleaseNotes,
         Command::SettingsSkipVersion,
+        Command::SettingsInstallUpdate,
+        Command::SettingsCancelDownload,
     ];
-    let updates = match pane {
-        SettingsPane::Updates => &updates[..],
+    let updates = match (pane, question) {
+        (SettingsPane::Updates, true) => &updates[..],
+        (SettingsPane::Updates, false) => &updates[2..],
         _ => &[],
     };
     let commands = updates.iter().copied().chain([
@@ -760,6 +772,10 @@ fn asked(
                 Command::SettingsCheckNow => Asked::CheckNow,
                 Command::SettingsReleaseNotes => Asked::Notes,
                 Command::SettingsSkipVersion => Asked::Skip,
+                Command::SettingsInstallUpdate => Asked::Install,
+                Command::SettingsCancelDownload => Asked::CancelDownload,
+                Command::SettingsInstallYes => Asked::Answer(true),
+                Command::SettingsInstallNo => Asked::Answer(false),
                 _ => Asked::Flip,
             });
         }
@@ -798,7 +814,8 @@ fn keys(
     ctx.input_mut(|input| {
         let held = input.modifiers;
         input.events.retain(|event| {
-            let Some(asked) = asked(&keymap, event, held, (free, pane)) else {
+            let question = pane == SettingsPane::Updates && dialog.update_question.is_some();
+            let Some(asked) = asked(&keymap, event, held, (free, pane, question)) else {
                 return true;
             };
             if preview && !matches!(asked, Asked::Edit | Asked::Pane { .. }) {
@@ -870,11 +887,37 @@ fn keys(
                         actions.push(Action::ShowReleaseNotes);
                     }
                 }
+                // Only where the block offers it: a version that downloads
+                // or waits to install is not skipped, and the key is left.
                 Asked::Skip => {
+                    if !updates::skips(app) {
+                        return true;
+                    }
                     if !repeat {
                         actions.push(Action::SkipUpdate);
                     }
                 }
+                // What the card offers: a press with nothing to do is
+                // left.
+                Asked::Install => match updates::install_action(app) {
+                    Some(action) => {
+                        if !repeat {
+                            actions.push(action);
+                        }
+                    }
+                    None => return true,
+                },
+                Asked::CancelDownload => {
+                    if !matches!(app.updates.install, crate::update::Install::Downloading(_)) {
+                        return true;
+                    }
+                    actions.push(Action::CancelDownload);
+                }
+                Asked::Answer(yes) => {
+                    if !repeat {
+                        actions.push(Action::AnswerUpdateQuestion(yes));
+                    }
+                }
                 // Only while the question stands.
                 Asked::History(delete) => {
                     if dialog.history_question.is_none() || repeat {
```

`src/ui/settings/terminal.rs`:

```diff
diff --git a/src/ui/settings/terminal.rs b/src/ui/settings/terminal.rs
index 7364cc56..36b23d66 100644
--- a/src/ui/settings/terminal.rs
+++ b/src/ui/settings/terminal.rs
@@ -251,12 +251,19 @@ pub(super) fn show(
                 Some(crate::model::Dialog::Settings(dialog)) => dialog.history_question,
                 _ => None,
             };
-            match question {
-                Some(question) => history_question(ui, question, &skin, actions),
-                None if pane == SettingsPane::Keyboard => {
+            let update = match &app.dialog {
+                Some(crate::model::Dialog::Settings(dialog)) if pane == SettingsPane::Updates => {
+                    dialog.update_question.clone()
+                }
+                _ => None,
+            };
+            match (question, update) {
+                (Some(question), _) => history_question(ui, question, &skin, actions),
+                (None, Some(version)) => update_question(ui, &version, &skin, actions),
+                (None, None) if pane == SettingsPane::Keyboard => {
                     keyboard_footer(ui, app, &dialog.keyboard, &skin, actions);
                 }
-                None => footer(ui, pane, option, opens, &settings, &skin, actions),
+                (None, None) => footer(ui, pane, option, opens, &settings, &skin, actions),
             }
         });
 }
@@ -309,6 +316,60 @@ fn history_question(
     }
 }
 
+/// The footer while `:update` asks whether to install `version` and
+/// restart, as the components' confirm row has it: the question on the
+/// sunken ground under a red rule, then `[y] yes  [N] no`. Each hint is
+/// its button too.
+fn update_question(ui: &mut Ui, version: &str, skin: &Skin, actions: &mut Vec<Action>) {
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
+    widgets::hline(ui, rect.x_range(), rect.top() + 0.5, palette.danger);
+    let y = rect.top() + 1.0 + FOOTER / 2.0;
+    let role = TextRole::OSecondary;
+    let asks = skin
+        .say("install {version} and restart?")
+        .replace("{version}", version);
+    let mut left = rect.left() + 12.0;
+    left += widgets::paint_label(ui, left, y, Text::one(look, role, &asks, palette.text)) + 18.0;
+    for (command, word, name, yes) in [
+        (
+            Command::SettingsInstallYes,
+            "yes",
+            "Install and restart",
+            true,
+        ),
+        (Command::SettingsInstallNo, "no", "Cancel", false),
+    ] {
+        let written = key(ui, look, command).bracketed();
+        let word = skin.say(word);
+        let hint = Text::new(look)
+            .add(role, &written, palette.accent)
+            .space(role, " ")
+            .add(role, &word, palette.secondary);
+        let width = widgets::paint_text(ui, left, y, hint);
+        let place = Rect::from_min_max(
+            pos2(left, rect.top() + 5.0),
+            pos2(left + width, rect.bottom() - 4.0),
+        );
+        widgets::announce(ui, place, &format!("{} {word}", written.as_str()));
+        left += width + 16.0;
+        if ButtonSpec::new(&gettext(locale, name))
+            .hidden_at(ui, place)
+            .clicked()
+        {
+            actions.push(Action::AnswerUpdateQuestion(yes));
+        }
+    }
+}
+
 /// One tinted line: the title and the pane it shows, and at the right
 /// what a change does and the key that opens the screen.
 fn header(ui: &mut Ui, pane: SettingsPane, skin: &Skin) {
```

`src/ui/settings/updates.rs`:

```diff
diff --git a/src/ui/settings/updates.rs b/src/ui/settings/updates.rs
index 4bf2fab0..8687dff2 100644
--- a/src/ui/settings/updates.rs
+++ b/src/ui/settings/updates.rs
@@ -1,22 +1,25 @@
 //! Settings › Updates: the status card that says where updates stand,
-//! how often a check runs, and what the check sends. What the card says
-//! is worked out here for both looks (`Card`); the sheet's drawing is
-//! here too, the terminal look's in `updates_terminal.rs`.
+//! from the check to the download and the restart that installs it, how
+//! often a check runs, and what the check sends. What the card says is
+//! worked out here for both looks (`Card`); the sheet's drawing is here
+//! too, the terminal look's in `updates_terminal.rs`.
 
 use std::time::{Duration, SystemTime};
 
 use egui::accesskit::{Live, Role};
 use egui::{CornerRadius, Rect, Sense, Stroke, StrokeKind, Ui, pos2, vec2};
 
-use crate::app::App;
+use crate::app::{App, Pending, Work};
 use crate::clock::Day;
-use crate::i18n::{Locale, gettext};
+use crate::i18n::{Locale, gettext, ngettext};
 use crate::model::Action;
 use crate::settings::OptionId;
 use crate::theme::Icon;
 use crate::typography::{Laid, Text, TextRole};
 use crate::ui::widgets::{self, ButtonSpec};
-use crate::update::{Failed, Installed, Managed, Offer, Phase, release};
+use crate::update::{
+    Downloading, Failed, Failure, Install, Installed, Managed, Offer, Phase, download, release,
+};
 
 use super::sheet::Skin;
 
@@ -39,11 +42,42 @@ pub(super) enum Card<'a> {
         failed: &'a Failed,
         last: Option<u64>,
     },
+    /// Downloading the version offered, then verifying it.
+    Downloading(&'a Downloading),
+    /// Downloaded and verified, for Restart now.
+    Ready(&'a str),
+    /// Restart now found work open: Work in progress, which lists it.
+    /// `when_done`: Restart when done was chosen.
+    Held {
+        when_done: bool,
+    },
+    /// Handed to the helper: the app quits in a moment.
+    Restarting(&'a str),
+    /// A download that gave nothing to install: Verification failed,
+    /// Couldn't download, or Couldn't install.
+    NotInstalled {
+        version: &'a str,
+        failure: &'a Failure,
+    },
 }
 
 impl<'a> Card<'a> {
     pub(super) fn of(app: &'a App) -> Self {
         let updates = &app.updates;
+        match &updates.install {
+            Install::Downloading(downloading) => return Self::Downloading(downloading),
+            Install::Ready { version, .. } => return Self::Ready(version),
+            Install::Held { when_done, .. } => {
+                return Self::Held {
+                    when_done: *when_done,
+                };
+            }
+            Install::Restarting { version } => return Self::Restarting(version),
+            Install::Failed { version, failure } => {
+                return Self::NotInstalled { version, failure };
+            }
+            Install::Idle => {}
+        }
         if updates.checking() {
             return Self::Checking;
         }
@@ -82,6 +116,22 @@ impl<'a> Card<'a> {
             }
             Self::Elsewhere(offer, _) => ("{version} is available", offer.version.as_str()),
             Self::Failed { .. } => ("Couldn't check for updates", current),
+            Self::Downloading(downloading) => {
+                ("Downloading {version}…", downloading.version.as_str())
+            }
+            Self::Ready(version) => ("{version} is ready to install", version),
+            Self::Held { .. } => ("Finish your work first", current),
+            Self::Restarting(version) => ("Installing {version}…", version),
+            Self::NotInstalled { version, failure } => {
+                let words = match failure {
+                    Failure::Unverified(_) => "The {version} download didn't verify",
+                    Failure::Undone(_) => "Couldn't download {version}",
+                    Failure::NotHanded(_) | Failure::WorkBegan | Failure::TooLate => {
+                        "Couldn't install {version}"
+                    }
+                };
+                (words, version)
+            }
         };
         gettext(locale, words).replace("{version}", version)
     }
@@ -90,11 +140,166 @@ impl<'a> Card<'a> {
 /// Whether a newer version is offered, whatever the card says while a
 /// check runs or after one failed: the dot on Updates in the nav.
 pub(super) fn offered(app: &App) -> bool {
+    let ready = matches!(
+        app.updates.install,
+        Install::Ready { .. } | Install::Held { .. }
+    );
+    ready
+        || app
+            .updates
+            .offer(app.settings.update_skip.as_deref())
+            .is_some()
+}
+
+/// Whether the version offered is one this copy downloads and installs
+/// itself: Download and install, and `U`.
+pub(super) fn installs(app: &App) -> bool {
     app.updates
-        .offer(app.settings.update_skip.as_deref())
+        .installable(app.settings.update_skip.as_deref())
         .is_some()
 }
 
+/// Whether the card offers to skip the version: `S`.
+pub(super) fn skips(app: &App) -> bool {
+    matches!(Card::of(app), Card::Available(_) | Card::NoBuild(_))
+}
+
+/// What `U` does on the pane, by the card: download the version offered,
+/// restart into the one ready, restart once the work listed is done or
+/// not, download again.
+pub(super) fn install_action(app: &App) -> Option<Action> {
+    match Card::of(app) {
+        Card::Available(_) if installs(app) => Some(Action::DownloadUpdate),
+        Card::Ready(_) => Some(Action::RestartToUpdate),
+        Card::Held {
+            when_done: false, ..
+        } => Some(Action::RestartWhenDone),
+        Card::Held {
+            when_done: true, ..
+        } => Some(Action::DontRestart),
+        Card::NotInstalled { .. } => Some(Action::DownloadUpdate),
+        _ => None,
+    }
+}
+
+/// How much of a download came, as the card says it: `18.4 of 31.2 MB`.
+pub(super) fn amount(downloading: &Downloading, locale: Locale) -> String {
+    gettext(locale, "{part} of {whole}")
+        .replace("{part}", &release::megabytes_figure(downloading.received))
+        .replace("{whole}", &release::megabytes(downloading.total))
+}
+
+/// What the line under Downloading's title says: how much came and the
+/// time left, that it waits for the network, or, once the last byte
+/// came, that the file is being verified.
+pub(super) fn downloading_said(downloading: &Downloading, locale: Locale) -> String {
+    if downloading.verifying() {
+        return gettext(locale, "Verifying…").into_owned();
+    }
+    let amount = amount(downloading, locale);
+    if downloading.waiting {
+        return format!("{} · {amount}", gettext(locale, "Waiting for the network"));
+    }
+    match downloading.time_left() {
+        Some(left) => {
+            let (words, count) = download::left_words(left);
+            let left = gettext(locale, words).replace("{n}", &count.to_string());
+            format!("{amount} · {left}")
+        }
+        None => amount,
+    }
+}
+
+/// What the box of a download that gave nothing says: what failed and,
+/// where the file was refused, that it was deleted.
+pub(super) fn failure_said(failure: &Failure, locale: Locale) -> String {
+    let said = failure.with_parts(&gettext(locale, failure.said()));
+    match failure.unverified() {
+        true => format!("{said} {}", gettext(locale, "The file was deleted.")),
+        false => said,
+    }
+}
+
+/// What the line under Ready's title says: where the helper waits for
+/// the quit already, that a quit installs it.
+pub(super) fn ready_said(app: &App) -> &'static str {
+    match app.updates.install.handed() {
+        Some(_) => "Signature checked · quitting Tabletist now installs it",
+        None => "Signature checked · restart to finish",
+    }
+}
+
+/// What a line of Work in progress says is pending, after its window's
+/// name: `3 unsaved edits in book_covers`.
+pub(super) fn work_said(work: &Work, locale: Locale) -> String {
+    match &work.what {
+        Pending::Edits { count, table } => {
+            let plural = u32::try_from(*count).unwrap_or(u32::MAX);
+            ngettext(
+                locale,
+                "{n} unsaved edit in {table}",
+                "{n} unsaved edits in {table}",
+                plural,
+            )
+            .replace("{n}", &count.to_string())
+            .replace("{table}", table)
+        }
+        Pending::Saving { table } => gettext(locale, "saving {table}").replace("{table}", table),
+        Pending::Writing { query } => {
+            gettext(locale, "a write running in Query {n}").replace("{n}", &query.to_string())
+        }
+        Pending::Running { secs, .. } => {
+            let (minutes, seconds) = ((secs / 60).to_string(), (secs % 60).to_string());
+            let time = match (secs / 60, secs % 60) {
+                (0, _) => gettext(locale, "{s} s").replace("{s}", &seconds),
+                (_, 0) => gettext(locale, "{m} min").replace("{m}", &minutes),
+                _ => gettext(locale, "{m} min {s} s")
+                    .replace("{m}", &minutes)
+                    .replace("{s}", &seconds),
+            };
+            gettext(locale, "query running for {time}").replace("{time}", &time)
+        }
+        Pending::Loading { table } => gettext(locale, "loading {table}").replace("{table}", table),
+    }
+}
+
+/// The download's bar: its share filled, and for a screen reader a
+/// progress indicator from 0 to the size in megabytes, its value what
+/// came, said as `18.4 of 31.2 MB`.
+pub(super) fn progress_bar(ui: &Ui, rect: Rect, downloading: &Downloading, skin: &Skin) {
+    let Skin {
+        palette, locale, ..
+    } = *skin;
+    let corner = CornerRadius::same(3);
+    ui.painter().rect_filled(rect, corner, palette.surface);
+    let share = match downloading.total {
+        0 => 0.0,
+        total => (release::megabytes_value(downloading.received) / release::megabytes_value(total))
+            .clamp(0.0, 1.0),
+    };
+    #[allow(clippy::cast_possible_truncation)]
+    let filled = Rect::from_min_size(rect.min, vec2(rect.width() * share as f32, rect.height()));
+    ui.painter().rect_filled(filled, corner, palette.accent);
+    progress_node(ui, rect, downloading, locale);
+}
+
+/// The progress indicator a screen reader is told of over `rect`.
+pub(super) fn progress_node(ui: &Ui, rect: Rect, downloading: &Downloading, locale: Locale) {
+    let id = ui.id().with("update-progress");
+    let response = ui.interact(rect, id, Sense::hover());
+    let name = gettext(locale, "Download progress");
+    response.widget_info(|| {
+        egui::WidgetInfo::labeled(egui::WidgetType::ProgressIndicator, true, &name)
+    });
+    let value = amount(downloading, locale);
+    ui.ctx().accesskit_node_builder(response.id, |node| {
+        node.set_min_numeric_value(0.0);
+        node.set_max_numeric_value(release::megabytes_value(downloading.total));
+        node.set_numeric_value(release::megabytes_value(downloading.received));
+        node.set_value(value);
+    });
+}
+
 /// The time `at` (seconds since the Unix epoch) as the card says it, by
 /// the clock as it is `now`: `today at 09:12`, `yesterday at 09:05`, `on
 /// Oct 21 at 09:05`. Translated.
@@ -170,6 +375,8 @@ const TILE: f32 = 40.0;
 const TILE_GAP: f32 = 14.0;
 /// Between the card's lines.
 const LINE_GAP: f32 = 6.0;
+/// The download's bar.
+const BAR: f32 = 6.0;
 
 /// The pane, from the top of `ui` down.
 pub(super) fn content(ui: &mut Ui, app: &App, limit: f32, skin: &Skin, actions: &mut Vec<Action>) {
@@ -246,6 +453,16 @@ fn card(ui: &mut Ui, app: &App, skin: &Skin, actions: &mut Vec<Action>) {
                 .replace("{when}", &when(locale, at, now)),
             None => skin.say("No check has worked yet"),
         },
+        Card::Downloading(downloading) => downloading_said(downloading, locale),
+        Card::Ready(_) => skin.say(ready_said(app)),
+        Card::Held {
+            when_done: false, ..
+        } => skin.say("Restart now would interrupt:"),
+        Card::Held {
+            when_done: true, ..
+        } => skin.say("Tabletist restarts once this is done:"),
+        Card::Restarting(_) => skin.say("Tabletist restarts in a moment"),
+        Card::NotInstalled { .. } => skin.say("Nothing was installed. Your copy is unchanged."),
     };
     let title_laid = Text::one(look, TextRole::StateTitle, &title, palette.text)
         .wrap(words)
@@ -268,6 +485,7 @@ fn card(ui: &mut Ui, app: &App, skin: &Skin, actions: &mut Vec<Action>) {
             let said = failed.with_numbers(&gettext(locale, failed.said()));
             Some((said, palette.danger))
         }
+        Card::NotInstalled { failure, .. } => Some((failure_said(failure, locale), palette.danger)),
         Card::NoBuild(offer) => Some((
             gettext(locale, "{version} has no build for this computer yet")
                 .replace("{version}", &offer.version),
@@ -284,6 +502,20 @@ fn card(ui: &mut Ui, app: &App, skin: &Skin, actions: &mut Vec<Action>) {
         Card::Elsewhere(_, managed) => managed.command,
         _ => None,
     };
+    // What Restart now would interrupt, a line each, in a box under the
+    // line.
+    let work: Vec<Laid> = match shown {
+        Card::Held { .. } => app
+            .work_in_progress()
+            .iter()
+            .map(|work| work_line(work, words - 20.0, skin).layout(ui.ctx()))
+            .collect(),
+        _ => Vec::new(),
+    };
+    let downloading = match shown {
+        Card::Downloading(downloading) => Some(downloading),
+        _ => None,
+    };
     let line = TextRole::Secondary.row_height(ui.ctx(), look.faces);
     let mut inner = title_laid.height() + LINE_GAP + sub_laid.height();
     for laid in &notes {
@@ -295,7 +527,13 @@ fn card(ui: &mut Ui, app: &App, skin: &Skin, actions: &mut Vec<Action>) {
     if command.is_some() {
         inner += LINE_GAP + look.control_height;
     }
-    let has_actions = command.is_none();
+    if downloading.is_some() {
+        inner += LINE_GAP + BAR;
+    }
+    if !work.is_empty() {
+        inner += LINE_GAP + work.iter().map(|laid| laid.height() + 2.0).sum::<f32>() + 14.0;
+    }
+    let has_actions = command.is_none() && !matches!(shown, Card::Restarting(_));
     if has_actions {
         inner += LINE_GAP + 4.0 + look.control_height;
     }
@@ -332,9 +570,19 @@ fn card(ui: &mut Ui, app: &App, skin: &Skin, actions: &mut Vec<Action>) {
         Card::Available(_) | Card::NoBuild(_) => (Some(Icon::Download), palette.accent),
         Card::Elsewhere(..) => (Some(Icon::Download), palette.secondary),
         Card::Failed { .. } => (Some(Icon::CircleAlert), palette.danger),
+        Card::Downloading(_) => (Some(Icon::Download), palette.accent),
+        Card::Ready(_) => (Some(Icon::RefreshCw), palette.accent),
+        Card::Held { .. } => (Some(Icon::TriangleAlert), palette.warning),
+        Card::Restarting(_) => (None, palette.dim),
+        Card::NotInstalled { failure, .. } if failure.unverified() => {
+            (Some(Icon::TriangleAlert), palette.danger)
+        }
+        Card::NotInstalled { .. } => (Some(Icon::CircleAlert), palette.danger),
     };
     let fill = match shown {
-        Card::Checking | Card::Elsewhere(..) | Card::NotChecked => palette.surface,
+        Card::Checking | Card::Elsewhere(..) | Card::NotChecked | Card::Restarting(_) => {
+            palette.surface
+        }
         _ => tint.gamma_multiply(0.14),
     };
     ui.painter().rect_filled(tile, CornerRadius::same(10), fill);
@@ -363,6 +611,16 @@ fn card(ui: &mut Ui, app: &App, skin: &Skin, actions: &mut Vec<Action>) {
     y += title_laid.height() + LINE_GAP;
     paint(ui, &sub_laid, pos2(left, y));
     y += sub_laid.height();
+    if let Some(downloading) = downloading {
+        y += LINE_GAP;
+        let bar = Rect::from_min_size(pos2(left, y), vec2(words, BAR));
+        progress_bar(ui, bar, downloading, skin);
+        y = bar.bottom();
+    }
+    if !work.is_empty() {
+        y += LINE_GAP;
+        y = work_box(ui, (left, y, words), &work, skin);
+    }
     if !notes.is_empty() {
         y += 2.0;
         let list_id = ui.id().with("update-notes");
@@ -415,7 +673,9 @@ fn card(ui: &mut Ui, app: &App, skin: &Skin, actions: &mut Vec<Action>) {
         // Told once, at once, where it is an error.
         let said = laid.galley.text().to_owned();
         match shown {
-            Card::Failed { .. } => announce_live(ui, band, (&said, "error"), Live::Assertive),
+            Card::Failed { .. } | Card::NotInstalled { .. } => {
+                announce_live(ui, band, (&said, "error"), Live::Assertive);
+            }
             _ => widgets::announce(ui, band, &said),
         }
         y = band.bottom();
@@ -438,7 +698,7 @@ fn card(ui: &mut Ui, app: &App, skin: &Skin, actions: &mut Vec<Action>) {
             .max_rect(place);
         let mut row = ui.new_child(builder.layout(centred));
         row.spacing_mut().item_spacing.x = 8.0;
-        card_actions(&mut row, shown, skin, actions);
+        card_actions(&mut row, (shown, installs(app)), skin, actions);
     }
 }
 
@@ -462,8 +722,80 @@ fn note(point: &str, width: f32, skin: &Skin) -> Text {
     text.wrap(width.max(0.0))
 }
 
-/// The card's buttons and links, by its state.
-fn card_actions(ui: &mut Ui, shown: Card<'_>, skin: &Skin, actions: &mut Vec<Action>) {
+/// A line of Work in progress: its window's name strong, then what is
+/// pending there.
+fn work_line(work: &Work, width: f32, skin: &Skin) -> Text {
+    let Skin {
+        look,
+        palette,
+        locale,
+        ..
+    } = *skin;
+    // In the box's warning ink, as the board has it.
+    Text::new(look)
+        .add(TextRole::UiBodyStrong, &work.window, palette.warning)
+        .add(
+            TextRole::UiBody,
+            &format!(": {}", work_said(work, locale)),
+            palette.warning,
+        )
+        .wrap(width.max(0.0))
+}
+
+/// The box of Work in progress at `left`, `top`, `width` wide: a list a
+/// screen reader is told of, an item a line. Returns where it ends.
+fn work_box(ui: &mut Ui, (left, top, width): (f32, f32, f32), work: &[Laid], skin: &Skin) -> f32 {
+    let Skin { palette, .. } = *skin;
+    let height = work.iter().map(|laid| laid.height() + 2.0).sum::<f32>() + 14.0;
+    let band = Rect::from_min_size(pos2(left, top), vec2(width, height));
+    let color = palette.warning;
+    ui.painter()
+        .rect_filled(band, CornerRadius::same(8), color.gamma_multiply(0.1));
+    ui.painter().rect_stroke(
+        band,
+        CornerRadius::same(8),
+        Stroke::new(1.0, color.gamma_multiply(0.35)),
+        StrokeKind::Inside,
+    );
+    let list_id = ui.id().with("update-work");
+    let list = ui.interact(band, list_id, Sense::hover());
+    let named = skin.say("Work in progress");
+    list.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Other, true, &named));
+    ui.ctx().accesskit_node_builder(list_id, |node| {
+        node.set_role(Role::List);
+    });
+    let items = ui.new_child(
+        egui::UiBuilder::new()
+            .max_rect(band)
+            .accessibility_parent(list_id),
+    );
+    let mut y = top + 8.0;
+    for (index, laid) in work.iter().enumerate() {
+        let at = pos2(left + 10.0, y);
+        laid.paint(ui.painter(), at);
+        let item = items.interact(
+            Rect::from_min_size(at, laid.size()),
+            list_id.with(index),
+            Sense::hover(),
+        );
+        let text = laid.galley.text().to_owned();
+        item.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Label, true, &text));
+        ui.ctx().accesskit_node_builder(item.id, |node| {
+            node.set_role(Role::ListItem);
+        });
+        y += laid.height() + 2.0;
+    }
+    band.bottom()
+}
+
+/// The card's buttons and links, by its state, and whether this copy
+/// installs the version offered itself.
+fn card_actions(
+    ui: &mut Ui,
+    (shown, installs): (Card<'_>, bool),
+    skin: &Skin,
+    actions: &mut Vec<Action>,
+) {
     let Skin {
         look,
         palette,
@@ -500,10 +832,19 @@ fn card_actions(ui: &mut Ui, shown: Card<'_>, skin: &Skin, actions: &mut Vec<Act
             }
         }
         Card::Available(offer) | Card::NoBuild(offer) => {
-            // Downloading comes with the next step: the page has the
-            // download until then.
+            let installs = installs && matches!(shown, Card::Available(_));
+            if installs
+                && ButtonSpec::new(&skin.say("Download and install"))
+                    .primary()
+                    .show(ui, height, look, palette)
+                    .clicked()
+            {
+                actions.push(Action::DownloadUpdate);
+            }
+            // Where this copy does not install it (Windows, another
+            // tool's copy, no build), the page has the download.
             let notes = ButtonSpec::new(&on_github);
-            let notes = if matches!(shown, Card::Available(_)) {
+            let notes = if matches!(shown, Card::Available(_)) && !installs {
                 notes.primary()
             } else {
                 notes.link()
@@ -538,6 +879,70 @@ fn card_actions(ui: &mut Ui, shown: Card<'_>, skin: &Skin, actions: &mut Vec<Act
                 actions.push(Action::CheckForUpdates);
             }
         }
+        Card::Downloading(_) => {
+            if ButtonSpec::new(&skin.say("Cancel"))
+                .show(ui, height, look, palette)
+                .clicked()
+            {
+                actions.push(Action::CancelDownload);
+            }
+        }
+        Card::Ready(_) => {
+            let restart = skin.say("Restart now");
+            if ButtonSpec::new(&restart)
+                .primary()
+                .show(ui, height, look, palette)
+                .clicked()
+            {
+                actions.push(Action::RestartToUpdate);
+            }
+            if ButtonSpec::new(&skin.say("Later"))
+                .show(ui, height, look, palette)
+                .clicked()
+            {
+                actions.push(Action::UpdateLater);
+            }
+        }
+        Card::Held { when_done, .. } => {
+            let (words, action) = match when_done {
+                false => ("Restart when done", Action::RestartWhenDone),
+                true => ("Don't restart", Action::DontRestart),
+            };
+            if ButtonSpec::new(&skin.say(words))
+                .primary()
+                .show(ui, height, look, palette)
+                .clicked()
+            {
+                actions.push(action);
+            }
+            if ButtonSpec::new(&skin.say("Show them"))
+                .show(ui, height, look, palette)
+                .clicked()
+            {
+                actions.push(Action::ShowWork);
+            }
+        }
+        Card::Restarting(_) => {}
+        Card::NotInstalled { failure, .. } => {
+            let again = match failure.unverified() {
+                true => "Download again",
+                false => "Try again",
+            };
+            if ButtonSpec::new(&skin.say(again))
+                .show(ui, height, look, palette)
+                .clicked()
+            {
+                actions.push(Action::DownloadUpdate);
+            }
+            if failure.unverified()
+                && ButtonSpec::new(&skin.say("Report on GitHub"))
+                    .link()
+                    .show(ui, height, look, palette)
+                    .clicked()
+            {
+                actions.push(Action::ReportUpdateFailure);
+            }
+        }
     }
 }
 
@@ -653,6 +1058,7 @@ pub(super) fn announce_live(ui: &Ui, rect: Rect, (text, which): (&str, &str), li
         node.set_live(live);
     });
 }
+
 #[cfg(test)]
 mod tests {
     use super::*;
```

`src/ui/settings/updates_terminal.rs`:

```diff
diff --git a/src/ui/settings/updates_terminal.rs b/src/ui/settings/updates_terminal.rs
index 4f906fcc..ca50fefe 100644
--- a/src/ui/settings/updates_terminal.rs
+++ b/src/ui/settings/updates_terminal.rs
@@ -1,6 +1,7 @@
 //! Settings › Updates as the terminal look draws it: the status block
-//! with its keys, the row of how often a check runs, and what a package
-//! manager's copy does, with what the check sends.
+//! with its keys (a download's progress row among its lines, and the work
+//! a restart would interrupt), the row of how often a check runs, and
+//! what a package manager's copy does, with what the check sends.
 
 use egui::accesskit::Live;
 use egui::{CornerRadius, Rect, Sense, Stroke, StrokeKind, Ui, pos2, vec2};
@@ -15,7 +16,10 @@ use crate::ui::widgets;
 use crate::update::release;
 
 use super::terminal::{ROW_SIDE, Skin, columns, heading, key, option_row, widths};
-use super::updates::{Card, announce_live, released, when};
+use super::updates::{
+    Card, announce_live, downloading_said, failure_said, installs, progress_node, released, when,
+    work_said,
+};
 
 /// What the note under the row says, word for word as the board has it.
 const NOTE: &str = "installed with a package manager (pacman, an AUR helper, nix, cargo install)? then that tool updates tabletist: this pane only checks and shows the command, for example";
@@ -57,6 +61,57 @@ pub(super) fn rows(
 /// One line of the block: its pieces, each its text and colour.
 type Line = Vec<(String, egui::Color32)>;
 
+/// The spinner of the components' progress row, a frame at a time.
+const SPINNER: [&str; 10] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
+
+/// The cells of the progress row's bar.
+const CELLS: usize = 12;
+
+/// The components' progress row: the spinner, the bar of cells and the
+/// share, then what the card's line says, and the key that cancels.
+fn progress_row(ui: &Ui, downloading: &crate::update::Downloading, skin: &Skin) -> Line {
+    let Skin {
+        look,
+        palette,
+        locale,
+        ..
+    } = *skin;
+    let time = ui.input(|input| input.time);
+    // Ten frames a second, as long as it shows.
+    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
+    let frame = SPINNER[(time * 10.0) as usize % SPINNER.len()];
+    ui.ctx()
+        .request_repaint_after(std::time::Duration::from_millis(100));
+    let share = match downloading.total {
+        0 => 0.0,
+        total => (release::megabytes_value(downloading.received) / release::megabytes_value(total))
+            .clamp(0.0, 1.0),
+    };
+    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
+    let filled = ((share * CELLS as f64).round() as usize).min(CELLS);
+    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
+    let percent = (share * 100.0).floor() as u32;
+    let said = look.label(&downloading_said(downloading, locale));
+    vec![
+        // The components' progress row: the spinner in accent, the
+        // bracket in the text's colour, the filled cells in accent and
+        // the rest muted, and what it says and its key muted.
+        (format!("  {frame}"), palette.accent),
+        (" [".to_owned(), palette.text),
+        ("█".repeat(filled), palette.accent),
+        ("░".repeat(CELLS - filled), palette.dim),
+        (format!("] {percent}%"), palette.text),
+        (format!(" · {said} · "), palette.dim),
+        (
+            key(ui, look, Command::SettingsCancelDownload)
+                .as_str()
+                .to_owned(),
+            palette.dim,
+        ),
+        (format!(" {}", skin.say("cancel")), palette.dim),
+    ]
+}
+
 /// The status block: what the card of the other looks says, a line at a
 /// time, with the pane's keys that act on it.
 fn block(ui: &mut Ui, app: &App, skin: &Skin) {
@@ -77,13 +132,18 @@ fn block(ui: &mut Ui, app: &App, skin: &Skin) {
         _ => look.label(&shown.title(locale)),
     };
     let title_color = match shown {
-        Card::Failed { .. } => palette.danger,
+        Card::Failed { .. } | Card::NotInstalled { .. } => palette.danger,
         Card::UpToDate { .. } => palette.success,
+        Card::Held { .. } => palette.warning,
         _ => palette.accent,
     };
     let say = |text: &'static str| skin.say(text);
     let mut sub: Vec<String> = Vec::new();
     let mut lines: Vec<Line> = Vec::new();
+    // Which of `lines` are the progress row, and which the work a restart
+    // would interrupt: a list for a screen reader.
+    let mut progress = None;
+    let mut work = 0..0;
     match shown {
         Card::NotChecked => sub.push(say("not checked yet")),
         Card::Checking => sub.push(crate::update::check::ASKED.to_owned()),
@@ -135,6 +195,34 @@ fn block(ui: &mut Ui, app: &App, skin: &Skin) {
             let said = failed.with_numbers(&gettext(locale, failed.said()));
             lines.push(vec![(look.label(&said), palette.danger)]);
         }
+        Card::Downloading(downloading) => {
+            progress = Some((lines.len(), downloading));
+            lines.push(progress_row(ui, downloading, skin));
+        }
+        Card::Ready(_) => sub.push(say(super::updates::ready_said(app))),
+        Card::Held { when_done, .. } => {
+            sub.push(say(match when_done {
+                false => "restart now would interrupt:",
+                true => "tabletist restarts once this is done:",
+            }));
+            let listed = app.work_in_progress();
+            let start = lines.len();
+            for item in &listed {
+                lines.push(vec![
+                    (format!("  - {}", item.window), palette.text),
+                    (format!(": {}", work_said(item, locale)), palette.text),
+                ]);
+            }
+            work = start..lines.len();
+        }
+        Card::Restarting(_) => sub.push(say("tabletist restarts in a moment")),
+        Card::NotInstalled { failure, .. } => {
+            sub.push(say("nothing was installed. your copy is unchanged."));
+            lines.push(vec![(
+                look.label(&failure_said(failure, locale)),
+                palette.danger,
+            )]);
+        }
     }
     // The pane's keys that act on what the block shows.
     let keys: Vec<(Command, String)> = match shown {
@@ -143,13 +231,31 @@ fn block(ui: &mut Ui, app: &App, skin: &Skin) {
         }
         Card::Failed { .. } => vec![(Command::SettingsCheckNow, say("try again"))],
         Card::Checking => Vec::new(),
-        Card::Available(offer) | Card::NoBuild(offer) => vec![
-            (Command::SettingsReleaseNotes, say("release notes")),
-            (
+        Card::Available(offer) | Card::NoBuild(offer) => {
+            let mut keys = Vec::new();
+            if matches!(shown, Card::Available(_)) && installs(app) {
+                keys.push((Command::SettingsInstallUpdate, say("update now")));
+            }
+            keys.push((Command::SettingsReleaseNotes, say("release notes")));
+            keys.push((
                 Command::SettingsSkipVersion,
                 format!("{} {}", say("skip"), offer.version),
-            ),
-        ],
+            ));
+            keys
+        }
+        // The progress row has its key.
+        Card::Downloading(_) | Card::Restarting(_) => Vec::new(),
+        Card::Ready(_) => vec![(Command::SettingsInstallUpdate, say("restart now"))],
+        Card::Held { when_done, .. } => vec![(
+            Command::SettingsInstallUpdate,
+            say(match when_done {
+                false => "restart when done",
+                true => "don't restart",
+            }),
+        )],
+        Card::NotInstalled { .. } => {
+            vec![(Command::SettingsInstallUpdate, say("download again"))]
+        }
         Card::Elsewhere(_, managed) => match managed.command {
             Some(_) => vec![
                 (Command::SettingsCopyCommand, say("copy")),
@@ -240,7 +346,33 @@ fn block(ui: &mut Ui, app: &App, skin: &Skin) {
             said.trim(),
         );
     }
-    for pieces in lines.iter().chain((!keyed.is_empty()).then_some(&keyed)) {
+    // The work a restart would interrupt is a list: its items are the
+    // list's children, not the block's.
+    let items = (!work.is_empty()).then(|| {
+        let top = y + line / 2.0 + line * work.start as f32;
+        let rect = Rect::from_min_max(
+            pos2(left, top),
+            pos2(right.max(left), top + line * work.len() as f32),
+        );
+        let list_id = ui.id().with("update-work");
+        let list = ui.interact(rect, list_id, Sense::hover());
+        let named = skin.say("Work in progress");
+        list.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Other, true, &named));
+        ui.ctx().accesskit_node_builder(list_id, |node| {
+            node.set_role(egui::accesskit::Role::List);
+        });
+        let items = ui.new_child(
+            egui::UiBuilder::new()
+                .max_rect(rect)
+                .accessibility_parent(list_id),
+        );
+        (list_id, items)
+    });
+    for (index, pieces) in lines
+        .iter()
+        .chain((!keyed.is_empty()).then_some(&keyed))
+        .enumerate()
+    {
         y += line;
         if std::ptr::eq(pieces, &keyed) {
             y += 6.0;
@@ -261,11 +393,31 @@ fn block(ui: &mut Ui, app: &App, skin: &Skin) {
             pos2(left + laid.width() / 2.0, y),
             vec2(laid.width().max(1.0), line),
         );
-        let error = matches!(shown, Card::Failed { .. })
+        let error = matches!(shown, Card::Failed { .. } | Card::NotInstalled { .. })
             && pieces
                 .first()
                 .is_some_and(|(_, color)| *color == palette.danger);
-        if error {
+        if let Some((at, downloading)) = progress
+            && at == index
+        {
+            // Read without the spinner and the bar's cells, which the
+            // progress indicator says.
+            let row: String = pieces[4..]
+                .iter()
+                .map(|(piece, _)| piece.as_str())
+                .collect();
+            widgets::announce(ui, place, row.trim_start_matches("] ").trim());
+            progress_node(ui, place, downloading, locale);
+        } else if let Some((list_id, items)) = &items
+            && work.contains(&index)
+        {
+            let item = items.interact(place, list_id.with(index), Sense::hover());
+            let said = said.trim().trim_start_matches("- ").to_owned();
+            item.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Label, true, &said));
+            ui.ctx().accesskit_node_builder(item.id, |node| {
+                node.set_role(egui::accesskit::Role::ListItem);
+            });
+        } else if error {
             announce_live(ui, place, (said.trim(), "error"), Live::Assertive);
         } else {
             widgets::announce(ui, place, said.trim());
```

`src/ui/start.rs`:

```diff
diff --git a/src/ui/start.rs b/src/ui/start.rs
index c8302b33..eeb4dbf8 100644
--- a/src/ui/start.rs
+++ b/src/ui/start.rs
@@ -214,6 +214,11 @@ pub fn show(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId) {
         })
         .unwrap_or_default();
     let searching = !query.trim().is_empty();
+    // An update downloaded and verified, on macOS: Restart installs it.
+    let ready = match &app.updates.install {
+        crate::update::Install::Ready { version, .. } if !look.terminal => Some(version.clone()),
+        _ => None,
+    };
     let skin = Skin {
         look: &look,
         palette: &palette,
@@ -277,6 +282,10 @@ pub fn show(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId) {
                     search_footer(&mut ui, &keys, skin);
                 }
             } else {
+                if let Some(version) = &ready {
+                    ready_line(&mut ui, version, skin, &mut actions);
+                    ui.add_space(gap(&look));
+                }
                 lists(&mut ui, &shown, &keys, skin, &mut claimed, &mut actions);
                 ui.add_space(gap(&look));
                 starts(&mut ui, &shown, &keys, skin, &mut claimed, &mut actions);
@@ -292,6 +301,33 @@ pub fn show(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId) {
     app.actions.extend(actions);
 }
 
+/// What waits where the start screen tells of something waiting, between
+/// the search field and the recents: `0.2.0 is ready · Restart`, muted,
+/// with Restart a link that does what Restart now does.
+fn ready_line(ui: &mut egui::Ui, version: &str, skin: Skin<'_>, actions: &mut Vec<Action>) {
+    let (look, palette, locale) = (skin.look, skin.palette, skin.locale);
+    let height = look.control_height;
+    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), height), Sense::hover());
+    let said = gettext(locale, "{version} is ready").replace("{version}", version);
+    let role = widgets::secondary(look);
+    let y = rect.center().y;
+    let mut x = rect.left();
+    x += widgets::paint_label(
+        ui,
+        x,
+        y,
+        Text::one(look, role, &format!("{said} ·"), palette.dim),
+    );
+    x += 4.0;
+    let restart = gettext(locale, "Restart");
+    let button = ButtonSpec::new(&restart).link();
+    let width = button.width(ui, look);
+    let place = Rect::from_min_size(pos2(x, rect.top()), vec2(width, height));
+    if button.show_at(ui, place, look, palette).clicked() {
+        actions.push(Action::RestartToUpdate);
+    }
+}
+
 /// The words after the database's name: `Read-only` on a production
 /// connection that is, the server as it names itself (or its kind, until
 /// it has), and how many tables and views the shown schema has.
```

`src/update.rs`:

```diff
diff --git a/src/update.rs b/src/update.rs
index 356c2bad..9c88e045 100644
--- a/src/update.rs
+++ b/src/update.rs
@@ -266,6 +266,9 @@ pub struct Updates {
     pub held: Option<(u64, Result<release::Latest, Failed>)>,
     /// Where the download of the version offered stands.
     pub install: Install,
+    /// `:update` waits for its check, to ask whether to install what it
+    /// finds.
+    pub ask_after_check: bool,
 }
 
 impl Updates {
```

`src/update/release.rs`:

```diff
diff --git a/src/update/release.rs b/src/update/release.rs
index f1863aa6..94756e6b 100644
--- a/src/update/release.rs
+++ b/src/update/release.rs
@@ -239,12 +239,22 @@ fn asset_name_on(version: &str, os: Os, arch: &str, kind: Option<Kind>) -> Optio
     }
 }
 
-/// A size as the pane writes it: `31.2 MB`.
-pub fn megabytes(bytes: u64) -> String {
-    // Shown to a tenth: the precision of f64 is no matter here.
+/// A size in megabytes to a tenth, without its unit: `18.4`.
+pub fn megabytes_figure(bytes: u64) -> String {
+    format!("{:.1}", megabytes_value(bytes))
+}
+
+/// A size in megabytes: what a screen reader is told of a download.
+pub fn megabytes_value(bytes: u64) -> f64 {
+    // Told to a tenth: the precision of f64 is no matter here.
     #[allow(clippy::cast_precision_loss)]
     let megabytes = bytes as f64 / 1_000_000.0;
-    format!("{megabytes:.1} MB")
+    megabytes
+}
+
+/// A size as the pane writes it: `31.2 MB`.
+pub fn megabytes(bytes: u64) -> String {
+    format!("{} MB", megabytes_figure(bytes))
 }
 
 #[cfg(test)]
```

- [ ] **Step 4: Run the checks**

Run: the checks of Task 1's Step 4, then `cd docs && bin/check`.
Expected: clean; the library's tests `test result: ok. 3686 passed; 0 failed; 17 ignored`; `tests/keys.rs` passes.

- [ ] **Step 5: Commit**

```bash
git add assets/icons/triangle-alert.svg docs/_guide/download.md docs/_guide/macos.md docs/_guide/omarchy.md docs/_reference/settings-and-files.md src/app.rs src/app/editing.rs src/app/updates.rs src/keymap.rs src/model.rs src/theme.rs src/ui/settings/mod.rs src/ui/settings/terminal.rs src/ui/settings/updates.rs src/ui/settings/updates_terminal.rs src/ui/start.rs src/ui/update_settings_tests.rs src/update.rs src/update/release.rs
git commit -m "Show the download, Ready and Work in progress in both looks, with U and :update's question"
```

---

## After Task 3

Push the branch and open the pull request. Its description has what the prompt's Delivery asks of this run:
- a summary;
- the download (what it fetches, resume, waiting while offline, cancel, where the partial files go);
- verification per platform (the crate's checks, row 5) and what a failure says (row 6);
- install per platform (the crate's helper, its minute, the relaunch with `--relaunch`);
- the work check;
- that going back is run 3;
- that the release workflow adds nothing;
- the departures above, rows 1 to 24.

No screenshot.

## Checks by hand

Off the record, on the demo database (Bookshop) and in both looks. None of this is a test.

1. **A release to install.** Run 1's hand check writes an offer into `updates.json`, but a download needs a real, signed release. `CONFIG` names this repository and its publisher key. So either use a throwaway build whose `CONFIG` names a fork and a key of your own, with that fork's release signed by it (`packaging/sign-checksums.sh`), or wait for a real release newer than the build. Run a portable Linux build with `tabletist-portable.txt` beside it.
2. **The download.** Download and install. Watch the bar, then turn the network off (Waiting for the network) and on again (it goes on). Then Cancel.
3. **The restart.** Download again, and with an unsaved edit in `book_covers` press Restart now (Work in progress). Then Restart when done, Save, and see it restart into the new version with the same windows, tables and SQL editors.
4. **Omarchy.** Press `U`, `ctrl+c`, and `:update` then `y`.
5. **macOS.** The start screen line.
6. **The boards.** Compare by eye with `macOS – Settings, Updates, states`.
