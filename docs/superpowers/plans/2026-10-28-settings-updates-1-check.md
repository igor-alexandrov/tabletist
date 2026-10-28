# Settings › Updates, run 1: the check Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Tabletist asks its GitHub releases whether a newer version is out, on launch when a check is due, at the interval while it runs, and whenever the person asks, and Settings › Updates says what it found in both looks: the status card (not checked yet, checking, up to date, available with the first four notes, no build for this computer, managed elsewhere with the command to copy, couldn't check), **Check for updates** (every day, every week, only when I ask), the privacy note, the dot on Updates in the navigation, and in the Omarchy look the status line's "0.4.0 available · :update" once a day and the keys `c`, `n`, `S` and `y`. An update's relaunch is confirmed after the first frame (step 1's Part B), so the release this ships in can be updated by run 2's. Nothing downloads yet.

**Architecture:** `src/update.rs` configures fastframe-update (bundle id, the release signing key, AppImages) over a ureq transport of our own, and holds what the app knows of updates (`Updates`, `Phase`, `due`). Under it: `check` (one GET of `releases/latest` with a 10 s limit, sending only the User-Agent, Accept and Host), `release` (the tag, the first four notes, the asset for this computer, by the names releases already use), `seen` (`updates.json` in the state folder: the last check that worked, the release found, when the status line last told of it) and `installed` (fastframe-update's detection as the card says it, with the command). The backend runs the check, the detection and the pager on blocking threads (`Command::Updates`, `Event::Updates`); `App::tick_updates` schedules the check each frame, waiting while a query runs. `updates.check` and `updates.skip` are keys of `settings.toml` under `[updates]`, written only where they differ from the defaults. The pane is `SettingsPane::Updates`, drawn by `src/ui/settings/updates.rs` (the sheet) and `src/ui/settings/updates_terminal.rs` (Omarchy) with the settings window's rows, keys and live reload.

**Tech Stack:** Rust, egui (crmne fork) with AccessKit, fastframe-update (crmne/fastframe tag v0.13.0), ureq 3.4 on rustls, serde_json, headless tests through `src/testing.rs` and a fake release server on a local port.

**Spec:** none in the repository: the prompt "build Settings › Updates" and the decisions below stand in for one. Step 1's plan, `docs/superpowers/plans/2026-10-21-automatic-updates-1-releases-and-groundwork.md`, says how fastframe-update installs an update and the asset names it expects.

---

## Before Task 1

Commit this plan:

```bash
git add docs/superpowers/plans/2026-10-28-settings-updates-1-check.md
git commit -m "Plan Settings › Updates, run 1: the check"
```

## Where this sits

The prompt asked for one pull request; you chose fastframe-update as the foundation and three runs, a pull request each:

- **Run 1 (this plan):** the check, the pane in both looks, install detection, and step 1's Part B (the receipt). A branch of `main` at `9dc4d68f`.
- **Run 2:** Download and install (`U`, `:update`'s confirm line), verification, the Ready and Work in progress states, Restart when done, the start screen's "0.2.0 is ready · Restart", the progress bar.
- **Run 3:** going back: the kept copy, the footer's **Go back to 0.1.0…**, its question, and the start screen's offer after a crash. You chose to re-download the previous release, verified, rather than keep a copy beside the app.

Step 1's Part A (the signed `checksums.txt`, the marker files, the AppImage's name) shipped in v0.3.0. Its Part B was never built; it is Task 1 here, because the version that installs an update is the old one, and the one it installs must confirm its receipt within a minute or be rolled back. So this run's release has to be out before run 2's.

## Decisions and departures, for a yes or a no before code

The first four are the prompt's open questions; the plan is written with the recommendation of each.

| # | The prompt or the boards | Built (recommended) | Why, and the other way |
|---|---|---|---|
| 1 | Open question 1: minisign, one `.minisig` asset per file | The signed `checksums.txt` releases carry since v0.3.0: `checksums.txt.sig`, a raw 64-byte Ed25519 signature, checked against `assets/update-public-key.hex`, then each file's SHA-256 in it. The release workflow adds nothing | Step 1 built and tests it (`tests/update_release.rs`), and fastframe-update verifies exactly this. One signature covers every file; minisign would add a format and a tool to the release job. Run 1 downloads nothing, so the verifying itself is run 2's |
| 2 | Open question 2: `tabletist-<version>-macos-universal.zip`, `tabletist-<version>-linux-x86_64.tar.gz` | The names releases already use, and the updater downloads: `tabletist-v<v>-macos-universal.dmg`, `tabletist-v<v>-<x86_64\|aarch64>-unknown-linux-gnu.tar.gz`, `tabletist-<v>-<x86_64\|aarch64>.AppImage`, and on Windows `tabletist-v<v>-<x86_64\|aarch64>-pc-windows-msvc-setup.exe` or `.zip` (`release::asset_name`) | The prompt says to match existing names. The card's size ("31.2 MB") is that file's |
| 3 | Open question 3: keep the Homebrew detection | Kept: fastframe-update's (`Caskroom/tabletist` under the prefix), with `brew upgrade --cask tabletist` | It costs nothing, as the prompt says |
| 4 | Open question 4: `U`, `c`, `n`, `S` in the pane and `:update` | `c` check now (and try again), `n` release notes and `S` skip are bound to the Updates pane (`When::UpdatesPane`) and read before the window's keys there, so the Editor pane's `n` is untouched; `U` is held there too, not built (`.reserved()`), for run 2. `y` stays the settings window's copy (`When::SettingsScreen`), which on this pane copies the update's command and on Agents the agent's. `:update` works from any Omarchy tab. The keymap's own tests find no other command on them | |
| 5 | Available: **Download and install**, Release notes on GitHub, Skip this version | **Release notes on GitHub** as the main button (the page has the downloads), and Skip this version. The Omarchy key line has no `U` | Downloading is run 2's. A Download button that does nothing until then would be a lie |
| 6 | `:update` checks, then downloads after "install 0.2.0 and restart? y/n" | `:update` opens Settings › Updates and checks there, as the macOS menu's Check for Updates… does | The confirm line comes with the download, in run 2 |
| 7 | "A newer version equal to `updates.skip` isn't offered" | Not offered by a check of the schedule's. A check the person asks for (Check now, the menu, `c`, `:update`) offers it, until the next check | The Go back question says "0.2.0 won't be offered again until you check". The other way: never offer it; Check now would then say up to date while a newer version is out |
| 8 | The states board's nine states | Run 1's five of them (Up to date, Checking, Available, Managed elsewhere, Couldn't check), and two the boards do not draw: **Not checked yet** (no check has worked, and none is under way: "Tabletist 0.3.0", "Not checked yet", **Check now** and What's new in 0.3.0) and **No build for this computer** ("0.4.0 has no build for this computer yet", from the prompt's Assets, with Release notes on GitHub as a link and Skip this version). Couldn't check before any check worked says "No check has worked yet" where the board has the last one's time | A fresh profile with "Only when I ask" has checked nothing; Up to date would be untrue |
| 8a | (a check that fails while a version found before is kept) | A check the person asked for that fails shows Couldn't check, with Try again, over the version; one of the schedule's that fails leaves the version shown. The dot stays either way | The person who asked is told; a laptop offline overnight does not lose the offer. The other way: every failure over the offer |
| 8b | Signals "while an update is Available or Ready" | The dot (and Omarchy's `●`) wherever a newer version is known, whatever the card shows meanwhile (Checking, Couldn't check), and also for No build and Managed elsewhere. The status line tells of it in the Omarchy look once a day, whether a check found it now or a check before this launch did; the macOS look has no status line, and notes nothing as told | A version out is news on every install; Managed elsewhere's command is the way to it. The other way: only where this copy can take it |
| 9 | Install detection's table | As the table, through fastframe-update's detection, and: `sudo pacman -Syu` (the prompt writes `pacman -Syu`), `yay -Syu` or `paru -Syu` where one is on the `PATH`; `sudo apt update && sudo apt upgrade`, `sudo dnf upgrade`, `flatpak update`, `sudo snap refresh`; Nix and other system folders with no command; a Mac copy run from the disk image says "Move Tabletist to Applications, then open it from there to update it" | pacman needs root. An AUR helper's update covers the AUR package, which pacman's does not. The disk image is read-only, and macOS runs a quarantined app from a random path |
| 10 | "Itself: anything else" | A Linux or Windows copy updates itself only where it is a release's: the archive's `tabletist-portable.txt` beside it, the installer's folder, or an AppImage. Any other copy (one built from source and copied by hand) says "Tabletist can't replace itself here" | fastframe-update's rule: replacing a binary nobody marked as replaceable could replace someone's build |
| 11 | "Nothing else is sent" | The User-Agent, `Accept: application/vnd.github+json` and `Host`, no other header; a test reads them on the wire. A redirect is followed only to `https://api.github.com/` (a repository that moved), at most three times; any other is a failed check | HTTP needs Host; GitHub asks for its Accept. Neither says anything of the person, and nothing goes to another host |
| 12 | Omarchy board: `check for updates ‹ daily ›` hint `last checked 09:12 · c check now · or weekly · never` | `last checked 09:12 · c check now`, as the prompt has it | The prompt wins; the row has no room for both beside the file pane |
| 13 | Omarchy board: the block's title "0.4.0 available" | The board's, lower case, for Available and No build; the card's title lower case for the others. The line under it ends with the system and processor (`you have 0.3.0 · released today · 31.2 mb · linux x86_64`) | The Omarchy pane's own voice; the size is that build's. The other way: leave the system out |
| 13a | Checking: **Cancel** | The sheet's Cancel. The Omarchy block has no key for it while it checks | The check gives up within 10 s, and `esc` leaves the pane. The other way: a key, which the board does not draw |
| 14 | Out of scope: Windows updates | Windows checks and shows the pane (the sheet), and has **Help › Check for Updates…**, since every command the quick open runs is in a menu there. Its privacy note says "this version, Windows and the processor" (its User-Agent names no system version), and its line under the title "signed" without "and notarized". Run 2 will offer no download on Windows | The settings window and the menu are one for both platforms; a Windows copy hears of a release and gets its page. The other way: hide the pane on Windows |
| 15 | Footer: "0.1.0 is kept until 0.2.0 has started once · Go back to 0.1.0…", then "Also in the menu: Tabletist › Check for Updates…" | The second line only, and only on macOS, where it is the whole footer as the prompt says ("shows only"): the settings file's path, Reveal, Export… and Reset to defaults are not on this pane there. Windows and Omarchy keep the window's usual footer | Going back is run 3's. Windows has the item in Help, and Omarchy no menu. The other way: the line beside the usual links |
| 16 | `n`: release notes in `$PAGER` | The notes (the release body whole) written to the state folder's `release-notes-<version>.md` and shown with `${PAGER:-less}` in a new terminal by `xdg-terminal-exec`; where none starts, the release page opens | Tabletist is a window, and `$PAGER` needs a terminal; `xdg-terminal-exec` is how Omarchy opens one |
| 17 | Accessibility | The card is a region named "Update status"; its title is announced politely as it changes, an error once and assertively; the notes are a list named "What's new"; the dot is named "Update available". The progress bar and the Work in progress list are run 2's | |
| 18 | Tests 6 to 9 (download, verification, install, restart) | Run 2's and run 3's. This run has tests 1 to 5, 10, 11, and 12 for `c`, `n`, `S`, `y` and `:update` | Test 10's detection by path (the Caskroom, `~/.cargo/bin`, `/usr`, `/nix/store`, owned by root, not writable) is fastframe-update's, with its own tests; this run tests what the card makes of each answer, one real detection (a cargo binary) and the Homebrew and self cards. Test 12's `n` is tested to the backend's command and from its "no terminal" answer to the page; starting the terminal itself is not |

The prompt's other rules hold as written: drafts and pre-releases are ignored (by `releases/latest` and again by the reader), a failed check does not count as checked and the next scheduled one retries an interval later, the state lives in `updates.json`, no error shows outside the card and its signals, and no UI copy has an em dash. Strings go through `gettext`, English being the app's only catalog. Every test uses the Bookshop data or the release fixture; none reaches GitHub, and none checks a colour, a size or a picture.

A reviewer given the draft tried to break it with tests of its own. It found eleven defects, which the draft now fixes. Each fix comes with a test that fails without it:

- **Cancel did not hold.** Cancel on a scheduled check left the schedule due, so the next frame started the check again, and the card said Checking at once. Cancel now holds the schedule's next check for an interval, as a failure does (`a_cancelled_scheduled_check_waits_an_interval`, and `cancel_on_the_card_holds` with the frames' own ticks).
- **A clock that had been ahead stopped the checks.** A `last_checked` or failure time after now (a clock once a year ahead) held every scheduled check back for a year and a day. The status line's once a day had the same flaw. A time after now counts as now (`a_last_check_in_the_future_counts_as_now`).
- **A failed check the person asked for went unseen.** With a version found before, Check now that failed showed the version again, with no error and no Try again. It now shows Couldn't check over the version; the schedule's failures leave the version shown (row 8a; `a_failed_check_asked_for_is_shown_over_a_version_found_before`).
- **A stalled body was not a timeout.** An answer whose body stalled past the limit read as no connection: ureq wraps its timeout inside an I/O error. It is now a timeout (`a_body_that_stalls_is_a_timeout`).
- **Redirects went to any host.** The check followed a redirect anywhere, http included, with its User-Agent. It now follows one only to `https://api.github.com/` (row 11; `a_redirect_to_another_host_is_not_followed`).
- **Check now during a scheduled check was dropped.** The scheduled check's answer then did not offer the version the person skipped, though they had asked. The press now makes the check under way one they asked for (`check_now_during_a_scheduled_check_counts_as_asked`).
- **The dot went away while a check ran.** The dot on Updates disappeared during every check. It now follows the version known, not the card (`the_dot_stays_while_a_check_runs`).
- **Notes alike merged for a screen reader.** Two notes with the same text were one list item. The items are keyed by their place (`two_notes_alike_are_two_items`).
- **An error was announced twice.** The title and the error line were both assertive. The title is polite in every state, in both looks (`an_error_is_told_once`).
- **A bracket before a link garbled a note.** `[breaking] Rename the key ([#12](…))` lost text. A link's words are taken only where no other bracket comes between (`a_bracket_before_a_link_is_kept_as_written`).
- **Code in a note was read as Markdown.** Its contents were parsed, so `` `__init__` `` turned bold. Code is now kept as written (`code_is_its_text`).

What held, among what it tried:
- An answer to an old check is dropped.
- Changing the setting to never while a check runs does no harm.
- A failed scheduled check retries an interval later and leaves `last_checked` as it was.
- A running query delays the check.
- `updates.skip` takes a `v`; an empty value clears it; invalid values are marked and keep the old value on live reload.
- Pre-releases, drafts, `-rc` tags, `nightly` and newer-than-skip all behave.
- No version with a `/` can reach the notes file's name.
- Notes with nested bullets, CRLF or non-ASCII text, and bodies over 1 MB, are handled.
- Exactly Host, Accept and User-Agent go out.
- The asset names match fastframe-update's.
- `c`, `n` and `S` act only on the Updates pane; repeats are ignored, and `y` passes through when there is no command.
- `:update` works from a connection tab.
- `intercept` handles odd arguments, and the receipt is confirmed where fastframe's migration notes say.
- Nothing in a test or a demo checks on its own.

A second reviewer read this plan against its diffs and the prompt. Besides the cancel and failed-check defects above, which it found too, the plan and the draft now take in these:

- **The status line spoke too rarely.** It spoke only when a scheduled check found the version. It now tells of any version offered, found now or before this launch, once a day, in the Omarchy look only. The macOS look notes nothing as told (row 8b; `the_status_line_tells_of_a_version_kept_from_before`, `the_macos_look_tells_nothing`).
- **The launch check could save the wrong download.** It could answer before the installation was detected, so an AppImage or an installer copy kept the archive's download. The scheduled check now waits for the detection (`the_scheduled_check_waits_to_know_how_this_copy_was_installed`).
- **Windows said "the macOS version" and "notarized".** It has its own words now (row 14).
- **Rows the table lacked:**
  - the dot and status line for No build and Managed elsewhere (8b);
  - no Cancel in the Omarchy look (13a);
  - the macOS footer losing its links (15);
  - the system and processor on Omarchy's line (13);
  - what tests 10 and 12 lean on (18).
- **Prose that was wrong:**
  - the count of states, and what Not checked yet and No build show (8);
  - which keys are the pane's (4, and Task 3's intro);
  - Windows file names (2);
  - the `base64` lines in `Cargo.lock`;
  - where Task 2's tests are;
  - the docs' "sixth pane".
- **Checks by hand:** they now say how to see Available without a newer release on GitHub.
- **Leftovers:** two leftover lines (`let _ = …`) are gone.

## Files

| File | Task | What |
|---|---|---|
| `Cargo.toml`, `Cargo.lock` | 1 | fastframe-update, ureq: new entries, and the three `"base64"` dependency lines that become `"base64 0.22.1"` beside the new 0.23; no other version moves |
| `src/lib.rs`, `src/update.rs` (new) | 1, 2 | The updater's configuration and transport; then `Updates`, `Phase`, `due` |
| `src/update/check.rs` (new) | 1 | The GET, its failures, the User-Agent; the fake server the tests share |
| `src/update/release.rs` (new) | 1 | The release as GitHub says it, the offer, the notes, the asset for this computer |
| `src/update/seen.rs` (new) | 1 | `updates.json` |
| `src/update/installed.rs` (new) | 1 | Who updates this copy, and the command |
| `src/entrypoint.rs` | 1, 2 | `fastframe_update::intercept` before clap, the receipt after the first frame; then `watch_updates` |
| `tests/cli.rs`, `tests/fixtures/update/latest.json` (new) | 1 | The flags taken out; the release the fake server gives |
| `AGENTS.md` | 1, 2 | The Updates section: the contract with later releases, then the schedule |
| `src/settings.rs` | 2, 3 | `UpdateCheck`, `updates.check`, `updates.skip`; then the option and its value |
| `src/backend.rs`, `src/backend/updates.rs` (new) | 2 | The check, the detection and the pager on the backend; `StateFile::Updates` |
| `src/app.rs`, `src/app/updates.rs` (new) | 2, 3 | The schedule, what a check's answer does, the status line; then `ShowUpdates` |
| `src/model.rs` | 2, 3 | The actions; then `SettingsPane::Updates` |
| `src/paths.rs`, `src/ui/toast.rs`, `src/promo/live.rs` | 2 | `updates.json` and the notes file; `say_for`; the promo's match |
| `src/keymap.rs`, `src/menu.rs`, `src/app/{commands,editing,menu}.rs` | 3 | `:update`, the pane's keys, Check for Updates… in the menus and quick open |
| `src/ui/settings/{mod,sheet,terminal}.rs`, `src/ui/settings/updates.rs` (new), `src/ui/settings/updates_terminal.rs` (new) | 3 | The pane in both looks, the dot, the row's print, the keys |
| `src/theme.rs`, `assets/icons/download.svg` (new) | 3 | Lucide's download icon |
| `src/ui/update_settings_tests.rs` (new), `src/ui/mod.rs` | 3 | Behaviour, headless |
| `docs/_reference/settings-and-files.md`, `docs/_guide/{omarchy,macos,download}.md` | 3 | The pane, the keys, the files, the menu item |

The tests run as before: `cargo test --locked --workspace --all-targets`; no server is needed by this run's. Use `~/.cargo/bin/cargo` where the mise shim fails.

---

### Task 1: The check, and the receipt

**Files:** see the table: `Cargo.toml`, `Cargo.lock`, `src/lib.rs`, `src/update.rs`, `src/update/{check,release,seen,installed}.rs`, `src/entrypoint.rs`, `tests/cli.rs`, `tests/fixtures/update/latest.json`, `AGENTS.md`.

What GitHub says, read without the app: `check::latest` sends one GET, follows a redirect only to the API's own host, and reads at most 1 MB of answer, a stalled body being a timeout as a stalled answer is; `release::found` makes an `Offer` of a newer release (its notes, its date, the asset for this computer and its size) or says it is up to date; `Seen` is the state file; `Installed::of` turns fastframe-update's detection into what the card says. `entrypoint::run` hands the arguments to `fastframe_update::intercept` before clap reads them: `--apply-update` makes the process an update's helper, and `--update-receipt` and `--update-error` are taken out (the error goes to the log). The receipt is confirmed on a thread of its own after the first frame. The new modules carry their tests; the check's run against a server on a local port that answers once with the fixture.

- [ ] **Step 1: Write the failing tests**

`tests/cli.rs`:

```diff
diff --git a/tests/cli.rs b/tests/cli.rs
index 67831148..99f5cbb0 100644
--- a/tests/cli.rs
+++ b/tests/cli.rs
@@ -35,6 +35,49 @@ fn demo_flags_require_demo() {
     );
 }
 
+/// An update's helper relaunches the app with `--update-receipt` (or, after
+/// a rollback, `--update-error`) beside the arguments it was started with.
+/// Those flags are taken out before clap reads the rest; clap alone would
+/// refuse them.
+#[test]
+fn update_flags_are_taken_out_before_the_arguments_are_read() {
+    for flags in [
+        [
+            "--update-receipt",
+            "/missing/.tabletist-update-0000000000000000/handoff.json",
+        ],
+        [
+            "--update-error",
+            "The update could not start. The previous version has been restored.",
+        ],
+    ] {
+        let output = tabletist()
+            .args(flags)
+            .arg("--version")
+            .output()
+            .expect("run tabletist");
+        assert!(output.status.success(), "{flags:?}: {output:?}");
+        assert_eq!(
+            String::from_utf8_lossy(&output.stdout).trim(),
+            format!("tabletist {}", env!("CARGO_PKG_VERSION"))
+        );
+    }
+}
+
+/// `--apply-update <job>` makes the process the helper, before anything
+/// else: with no job to read it fails and exits, and never opens a window.
+#[test]
+fn the_helper_flag_runs_the_helper_and_nothing_else() {
+    let output = tabletist()
+        .args([
+            "--apply-update",
+            "/missing/.tabletist-update-0000000000000000/handoff.json",
+        ])
+        .output()
+        .expect("run tabletist");
+    assert_eq!(output.status.code(), Some(1), "{output:?}");
+}
+
 /// A release build on Windows is a GUI program (`src/main.rs`), and the
 /// relay is that same program: CI runs the relay's tests below on it too,
 /// and this says that they ran on what a person installs.
```

`tests/fixtures/update/latest.json`:

```diff
diff --git a/tests/fixtures/update/latest.json b/tests/fixtures/update/latest.json
new file mode 100644
index 00000000..e99a2170
--- /dev/null
+++ b/tests/fixtures/update/latest.json
@@ -0,0 +1,24 @@
+{
+  "url": "https://api.github.com/repos/igor-alexandrov/tabletist/releases/1",
+  "html_url": "https://github.com/igor-alexandrov/tabletist/releases/tag/v0.4.0",
+  "tag_name": "v0.4.0",
+  "name": "Tabletist v0.4.0",
+  "draft": false,
+  "prerelease": false,
+  "published_at": "2026-10-24T03:16:13Z",
+  "body": "## What's new\n\n- **Edit rows** in the grid and the inspector, with a pending bar and one transaction per save\n- **Insert and delete rows**, with checks before anything is sent\n- **Set to**: NULL, DEFAULT and common values in one key\n- **Date and time picker**, **DDL view** and **query history**\n- Settings › Updates\n",
+  "assets": [
+    { "name": "checksums.txt", "size": 1043 },
+    { "name": "checksums.txt.sig", "size": 64 },
+    { "name": "tabletist-0.4.0-aarch64.AppImage", "size": 33100000 },
+    { "name": "tabletist-0.4.0-x86_64.AppImage", "size": 32400000 },
+    { "name": "tabletist-v0.4.0-aarch64-pc-windows-msvc-setup.exe", "size": 12900000 },
+    { "name": "tabletist-v0.4.0-aarch64-pc-windows-msvc.zip", "size": 13800000 },
+    { "name": "tabletist-v0.4.0-aarch64-unknown-linux-gnu.tar.gz", "size": 28900000 },
+    { "name": "tabletist-v0.4.0-macos-universal.dmg", "size": 31200000 },
+    { "name": "tabletist-v0.4.0-source.tar.gz", "size": 2100000 },
+    { "name": "tabletist-v0.4.0-x86_64-pc-windows-msvc-setup.exe", "size": 13100000 },
+    { "name": "tabletist-v0.4.0-x86_64-pc-windows-msvc.zip", "size": 14000000 },
+    { "name": "tabletist-v0.4.0-x86_64-unknown-linux-gnu.tar.gz", "size": 29800000 }
+  ]
+}
```

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test --locked --test cli`
Expected: `test result: FAILED. 5 passed; 2 failed`, clap refusing the flags:

```
error: unexpected argument '--apply-update' found
error: unexpected argument '--update-receipt' found
```

- [ ] **Step 3: Write the code**

The `Cargo.lock` diff is the whole change: add its entries by hand (and rename the three `"base64"` lines to `"base64 0.22.1"`, as a second base64 asks) and check with `--locked`; a re-resolve moves unrelated `windows-sys` pins.

`AGENTS.md`:

```diff
diff --git a/AGENTS.md b/AGENTS.md
index 02118e32..7569727a 100644
--- a/AGENTS.md
+++ b/AGENTS.md
@@ -242,3 +242,29 @@ source AUR package, and builds and runs the AppImage, whenever `packaging/`
 changes. `packaging/linux/appimage.sh` pins appimagetool and the AppImage
 runtime by version and SHA-256; move them together. Locally,
 `packaging/arch/render.sh` fills a PKGBUILD from a release's checksums.
+
+## Updates
+
+`src/update.rs` configures fastframe-update, which installs an update, and
+holds the check: one GET of `releases/latest` that sends
+`Tabletist/<version> (<system>; <chip>)` and no other header of the
+person's (`src/update/check.rs` tests what goes on the wire). Tests never
+reach GitHub; they serve fixture JSON from a local port
+(`tests/fixtures/update/latest.json`).
+
+The version that installs an update is the old one, and its helper runs
+the new one, so these hold for every release, and a change to any of them
+needs a release that accepts both ways first:
+
+- `tabletist --version` prints `tabletist <version>`, and
+  `--update-receipt <job>`, `--update-error <message>` and
+  `--apply-update <job>` are handled by `fastframe_update::intercept` before
+  clap reads the command line (`tests/cli.rs`).
+- The receipt is confirmed after the first frame (`entrypoint.rs`).
+- The release's file names, the markers (`packaging/tabletist-portable.txt`
+  in the archives, `packaging/windows/tabletist-installer.txt` installed
+  beside the app) and `checksums.txt.sig` stay as `release.yml` makes them
+  (`tests/update_release.rs`).
+- `assets/update-public-key.hex` changes only through
+  `additional_publisher_keys`: ship the next key there, keep signing with
+  the old one, and switch once installed copies trust the next.
```

`Cargo.lock`:

```diff
diff --git a/Cargo.lock b/Cargo.lock
index 0b1aa9bf..64c2a912 100644
--- a/Cargo.lock
+++ b/Cargo.lock
@@ -550,6 +550,12 @@ version = "0.22.1"
 source = "registry+https://github.com/rust-lang/crates.io-index"
 checksum = "72b3254f16251a8381aa12e40e3c4d2f0199f8c6508fbecb9d91f575e0fbb8c6"
 
+[[package]]
+name = "base64"
+version = "0.23.1"
+source = "registry+https://github.com/rust-lang/crates.io-index"
+checksum = "ac07cdecf99051d9a5238b80f35af32cdeba5b336e55d957b318b50137e18da5"
+
 [[package]]
 name = "base64ct"
 version = "1.8.3"
@@ -1861,6 +1867,14 @@ version = "0.1.9"
 source = "registry+https://github.com/rust-lang/crates.io-index"
 checksum = "7360491ce676a36bf9bb3c56c1aa791658183a54d2744120f27285738d90465a"
 
+[[package]]
+name = "fastframe-dirs"
+version = "0.13.0"
+source = "git+https://github.com/crmne/fastframe?tag=v0.13.0#d5e1a99702f39fda6e252b7acb5582c60becf545"
+dependencies = [
+ "directories",
+]
+
 [[package]]
 name = "fastframe-fonts"
 version = "0.13.0"
@@ -1925,6 +1939,21 @@ dependencies = [
  "serde_json",
 ]
 
+[[package]]
+name = "fastframe-update"
+version = "0.13.0"
+source = "git+https://github.com/crmne/fastframe?tag=v0.13.0#d5e1a99702f39fda6e252b7acb5582c60becf545"
+dependencies = [
+ "anyhow",
+ "fastframe-dirs",
+ "ring",
+ "serde",
+ "serde_json",
+ "sha2 0.11.0",
+ "url",
+ "windows-sys 0.60.2",
+]
+
 [[package]]
 name = "fastrand"
 version = "2.5.0"
@@ -2486,6 +2515,22 @@ dependencies = [
  "digest 0.11.3",
 ]
 
+[[package]]
+name = "http"
+version = "1.5.0"
+source = "registry+https://github.com/rust-lang/crates.io-index"
+checksum = "918d3568bebf352712bc2ef3d46a8bcf1a75b373be6539de198e9105cbbf9ce0"
+dependencies = [
+ "bytes",
+ "itoa",
+]
+
+[[package]]
+name = "httparse"
+version = "1.10.1"
+source = "registry+https://github.com/rust-lang/crates.io-index"
+checksum = "6dbf3de79e51f3d586ab4cb9d5c3e2c14aa28ed23d180cf89b4df0454a69cc87"
+
 [[package]]
 name = "hybrid-array"
 version = "0.4.15"
@@ -3307,7 +3352,7 @@ version = "0.37.3"
 source = "registry+https://github.com/rust-lang/crates.io-index"
 checksum = "0f27695f286b461da077b8c2f72f47feaa04ce3c3f9c0976257410e90e21208a"
 dependencies = [
- "base64",
+ "base64 0.22.1",
  "bitflags 2.13.2",
  "btoi",
  "byteorder",
@@ -4398,7 +4443,7 @@ version = "0.6.12"
 source = "registry+https://github.com/rust-lang/crates.io-index"
 checksum = "08808e3c483c46e999108051c78334f473d5adb59d78bb80a1268c7e6aa6c514"
 dependencies = [
- "base64",
+ "base64 0.22.1",
  "byteorder",
  "bytes",
  "fallible-iterator 0.2.0",
@@ -5740,6 +5785,7 @@ dependencies = [
  "fastframe-log",
  "fastframe-text",
  "fastframe-theme",
+ "fastframe-update",
  "hmac",
  "image",
  "jiff",
@@ -5767,6 +5813,7 @@ dependencies = [
  "tokio",
  "tokio-util",
  "toml",
+ "ureq",
  "uuid",
  "windows-native-keyring-store",
  "winit",
@@ -6275,6 +6322,34 @@ version = "0.9.0"
 source = "registry+https://github.com/rust-lang/crates.io-index"
 checksum = "8ecb6da28b8a351d773b68d5825ac39017e680750f980f3a1a85cd8dd28a47c1"
 
+[[package]]
+name = "ureq"
+version = "3.4.2"
+source = "registry+https://github.com/rust-lang/crates.io-index"
+checksum = "9a7ac20be9b7726e0bbdbf974c059676d9acb1cd414961f570a4e8231cacd7fc"
+dependencies = [
+ "base64 0.23.1",
+ "log",
+ "percent-encoding",
+ "rustls",
+ "rustls-pki-types",
+ "ureq-proto",
+ "utf8-zero",
+ "webpki-roots",
+]
+
+[[package]]
+name = "ureq-proto"
+version = "0.6.5"
+source = "registry+https://github.com/rust-lang/crates.io-index"
+checksum = "841583fc36089d7d59419568ae157f43a5a03a2e5a81ba87c77d1596ef46e922"
+dependencies = [
+ "base64 0.23.1",
+ "http",
+ "httparse",
+ "log",
+]
+
 [[package]]
 name = "url"
 version = "2.5.8"
@@ -6293,7 +6368,7 @@ version = "0.45.1"
 source = "registry+https://github.com/rust-lang/crates.io-index"
 checksum = "80be9b06fbae3b8b303400ab20778c80bbaf338f563afe567cf3c9eea17b47ef"
 dependencies = [
- "base64",
+ "base64 0.22.1",
  "data-url",
  "flate2",
  "imagesize",
@@ -6309,6 +6384,12 @@ dependencies = [
  "xmlwriter",
 ]
 
+[[package]]
+name = "utf8-zero"
+version = "0.8.1"
+source = "registry+https://github.com/rust-lang/crates.io-index"
+checksum = "b8c0a043c9540bae7c578c88f91dda8bd82e59ae27c21baca69c8b191aaf5a6e"
+
 [[package]]
 name = "utf8_iter"
 version = "1.0.4"
```

`Cargo.toml`:

```diff
diff --git a/Cargo.toml b/Cargo.toml
index 20f27441..ff9029b3 100644
--- a/Cargo.toml
+++ b/Cargo.toml
@@ -75,11 +75,22 @@ fastframe-theme = { git = "https://github.com/crmne/fastframe", tag = "v0.13.0"
 fastframe-i18n = { git = "https://github.com/crmne/fastframe", tag = "v0.13.0" }
 # Log file for bug reports and a panic log without payloads.
 fastframe-log = { git = "https://github.com/crmne/fastframe", tag = "v0.13.0" }
+# Self-update from GitHub releases (src/update.rs): detects how this copy
+# was installed, checks the signature on checksums.txt, hands an update to
+# a helper that installs it and rolls back if the new version does not
+# start.
+fastframe-update = { git = "https://github.com/crmne/fastframe", tag = "v0.13.0" }
 # The database layer (no UI dependencies).
 tabletist-db = { path = "crates/tabletist-db" }
 # The MCP server agents reach the saved connections through. The backend
 # runs it, and is its gateway to the databases (src/backend/agents.rs).
 tabletist-mcp = { path = "crates/tabletist-mcp" }
+# The check for updates and the updater's HTTP client: blocking GETs to
+# GitHub, on rustls with ring and Mozilla's roots (webpki-roots), all
+# already in the tree. Adds ureq, ureq-proto, http, httparse, utf8-zero
+# and base64 0.23 (beside the tree's 0.22); reqwest would add hyper, h2
+# and tower. No gzip: release files are compressed already.
+ureq = { version = "3.4", default-features = false, features = ["rustls"] }
 # The menu bar on macOS and Windows, built with muda from `src/menu.rs`'s
 # model. Empty elsewhere.
 tabletist-menubar = { path = "crates/tabletist-menubar" }
```

`src/entrypoint.rs`:

```diff
diff --git a/src/entrypoint.rs b/src/entrypoint.rs
index 27ae081e..a26a7dee 100644
--- a/src/entrypoint.rs
+++ b/src/entrypoint.rs
@@ -59,13 +59,18 @@ pub const APP_ID: &str = "dev.tabletist.Tabletist";
 /// build on Windows has no console, and an app launched from a menu has no
 /// terminal, so stderr alone would be a silent exit.
 pub fn run() -> std::process::ExitCode {
-    let cli = Cli::parse();
+    // First of all: `--apply-update <job>` makes this process an update's
+    // helper, which installs it and exits. Otherwise the receipt or the
+    // error an update's relaunch carries is taken out of the arguments
+    // before clap reads them (src/update.rs).
+    let launch = fastframe_update::intercept(&crate::update::CONFIG);
+    let cli = Cli::parse_from(&launch.arguments);
     // Before the logger, the settings and the window: the relay writes
     // nothing but the protocol to stdout and shows no dialog.
     if let Some(Sub::Mcp { endpoint }) = cli.command {
         return crate::relay::main(endpoint);
     }
-    match main(cli) {
+    match main(cli, launch) {
         Ok(()) => std::process::ExitCode::SUCCESS,
         Err(error) => {
             report_startup_error(&error);
@@ -97,7 +102,7 @@ fn report_startup_error(error: &anyhow::Error) {
         .show();
 }
 
-pub fn main(cli: Cli) -> anyhow::Result<()> {
+pub fn main(cli: Cli, launch: fastframe_update::Launch) -> anyhow::Result<()> {
     // Demo runs use a throwaway profile: their settings, connections and
     // secrets are never the user's. Like any start they take the desktop's
     // theme and font (and, installed from a package, set up its Omarchy
@@ -124,6 +129,12 @@ pub fn main(cli: Cli) -> anyhow::Result<()> {
         logging = logging.file(dirs.log_file()).panic_log(dirs.panic_log());
     }
     logging.init()?;
+    // The helper put the previous version back: this one. The next run
+    // shows it in the window; until then the log has it.
+    if let Some(message) = &launch.error {
+        log::error!("{message}");
+    }
+    let update_receipt = launch.receipt;
 
     let settings = Settings::load(&dirs);
     let demo = cli.demo;
@@ -157,6 +168,7 @@ pub fn main(cli: Cli) -> anyhow::Result<()> {
                 app,
                 shot,
                 demo,
+                update_receipt,
                 #[cfg(target_os = "macos")]
                 title_bar: None,
                 menu,
@@ -293,6 +305,10 @@ struct Window {
     app: App,
     shot: Option<Shot>,
     demo: bool,
+    /// Set when an update's helper started this version. Confirmed after
+    /// the first frame: until then the helper stands ready to put the
+    /// previous version back.
+    update_receipt: Option<fastframe_update::Receipt>,
     /// macOS: the window whose unified title bar the tabs share, once it
     /// exists.
     #[cfg(target_os = "macos")]
@@ -389,6 +405,15 @@ impl eframe::App for Window {
             }
         }
         self.app.frame_ui(ui);
+        // The window is up: tell the update's helper this version started,
+        // so it keeps it. Off the interface thread, as the receipt asks.
+        if let Some(receipt) = self.update_receipt.take() {
+            std::thread::spawn(move || {
+                if let Err(error) = receipt.acknowledge() {
+                    log::error!("could not confirm the update: {error:#}");
+                }
+            });
+        }
         #[cfg(target_os = "macos")]
         if let Some(title_bar) = &self.title_bar {
             let zoom = ui.ctx().zoom_factor();
```

`src/lib.rs`:

```diff
diff --git a/src/lib.rs b/src/lib.rs
index 67fe386e..ec7ed9c2 100644
--- a/src/lib.rs
+++ b/src/lib.rs
@@ -40,6 +40,7 @@ pub mod testing;
 pub mod theme;
 pub mod typography;
 pub mod ui;
+pub mod update;
 pub mod util;
 pub mod vim;
 pub mod zones;
```

`src/update.rs`:

```diff
diff --git a/src/update.rs b/src/update.rs
new file mode 100644
index 00000000..e4882b8f
--- /dev/null
+++ b/src/update.rs
@@ -0,0 +1,133 @@
+//! Updates from Tabletist's GitHub releases: the check Settings › Updates
+//! shows (`check`, `release`), what it found as the state folder keeps it
+//! (`seen`), how this copy was installed (`installed`), and the updater
+//! that installs one, fastframe-update.
+//!
+//! What a later release's helper relies on from this build, which must not
+//! change: `tabletist --version` prints `tabletist <version>`;
+//! `--update-receipt <job>` and `--update-error <message>` are taken out
+//! before clap reads the command line ([`fastframe_update::intercept`] in
+//! `entrypoint::run`); and the receipt is confirmed once the window is up.
+//! The release side of the contract (asset names, markers, the signed
+//! checksums) is in `release.yml` and `tests/update_release.rs`.
+
+pub mod check;
+pub mod installed;
+pub mod release;
+pub mod seen;
+
+use std::time::Duration;
+
+use fastframe_update::{MacConfig, Request, Response, Transport, UpdateConfig, Updater};
+
+pub use check::Failed;
+pub use installed::{Installed, Managed};
+pub use release::{Found, Offer};
+pub use seen::Seen;
+
+/// The version that runs.
+pub const CURRENT: &str = env!("CARGO_PKG_VERSION");
+
+/// What the updater knows about Tabletist.
+pub const CONFIG: UpdateConfig = UpdateConfig {
+    macos: MacConfig {
+        bundle_ids: &["dev.tabletist.Tabletist"],
+        // The slug: Contents/MacOS/tabletist.
+        executable_names: &[],
+        legacy_bundle_names: &[],
+    },
+    // Releases sign checksums.txt (packaging/sign-checksums.sh); nothing is
+    // downloaded from a release whose signature does not verify.
+    publisher_key: Some(include_str!("../assets/update-public-key.hex")),
+    // Releases list tabletist-<version>-<arch>.AppImage in the signed
+    // checksums, so an AppImage replaces itself with the next one.
+    appimage: true,
+    ..UpdateConfig::new(
+        "igor-alexandrov/tabletist",
+        "Tabletist",
+        "tabletist",
+        CURRENT,
+    )
+};
+
+/// An updater, on an HTTP client of its own. Its calls block: run them on
+/// a thread of their own, never on the interface thread.
+pub fn updater() -> Updater {
+    Updater::new(CONFIG, UreqTransport::new())
+}
+
+/// The release page of the version that runs: What's new in it.
+pub fn this_release() -> String {
+    format!("https://github.com/igor-alexandrov/tabletist/releases/tag/v{CURRENT}")
+}
+
+/// The updater's HTTP client: one GET at a time, redirects handed back
+/// rather than followed (the updater follows them, to GitHub's hosts
+/// only), any status returned as an answer, 15 seconds to connect and 15
+/// minutes for a whole download, as fastframe-update asks of a transport.
+/// It says what the check says of this copy, and nothing else.
+struct UreqTransport {
+    agent: ureq::Agent,
+    user_agent: String,
+}
+
+impl UreqTransport {
+    fn new() -> Self {
+        let config = ureq::Agent::config_builder()
+            .max_redirects(0)
+            .http_status_as_error(false)
+            .timeout_connect(Some(Duration::from_secs(15)))
+            .timeout_global(Some(Duration::from_secs(15 * 60)))
+            .build();
+        Self {
+            agent: config.into(),
+            user_agent: check::user_agent(CURRENT, check::macos_version().as_deref()),
+        }
+    }
+}
+
+impl Transport for UreqTransport {
+    fn get(&self, request: &Request<'_>) -> anyhow::Result<Response> {
+        let response = self
+            .agent
+            .get(request.url)
+            .header("Accept", request.accept)
+            .header("User-Agent", &self.user_agent)
+            .call()?;
+        let location = response
+            .headers()
+            .get("location")
+            .and_then(|value| value.to_str().ok())
+            .map(str::to_owned);
+        Ok(Response {
+            status: response.status().as_u16(),
+            location,
+            body: Box::new(response.into_body().into_reader()),
+        })
+    }
+}
+
+#[cfg(test)]
+mod tests {
+    use super::*;
+
+    #[test]
+    fn the_configuration_is_valid() {
+        CONFIG.validate().unwrap();
+        assert_eq!(CONFIG.current_version, env!("CARGO_PKG_VERSION"));
+    }
+
+    /// Without a key the updater trusts checksums.txt as GitHub serves it;
+    /// with one, a release must be signed (packaging/sign-checksums.sh).
+    #[test]
+    fn updates_need_the_release_signature() {
+        assert!(CONFIG.publisher_key.is_some());
+    }
+
+    /// The release names its AppImages for the updater (release.yml), so
+    /// an AppImage may replace itself.
+    #[test]
+    fn appimages_update_themselves() {
+        const { assert!(CONFIG.appimage) };
+    }
+}
```

`src/update/check.rs`:

```diff
diff --git a/src/update/check.rs b/src/update/check.rs
new file mode 100644
index 00000000..7831239d
--- /dev/null
+++ b/src/update/check.rs
@@ -0,0 +1,383 @@
+//! The check: one GET of the latest release, which says nothing of the
+//! person but this version, the system and the processor.
+
+use std::io::Read;
+use std::time::Duration;
+
+use super::release::Latest;
+
+/// Where the check asks.
+pub const LATEST: &str = "https://api.github.com/repos/igor-alexandrov/tabletist/releases/latest";
+
+/// What the check card says it is asking, while it asks.
+pub const ASKED: &str = "github.com/igor-alexandrov/tabletist/releases";
+
+/// How long a check waits for its answer, all told.
+pub const TIMEOUT: Duration = Duration::from_secs(10);
+
+/// The largest answer read: a release's notes and its files' names.
+const LIMIT: u64 = 1024 * 1024;
+
+/// Where a redirect may lead: the API itself, as for a repository that
+/// moved. Any other is not followed.
+const API: &str = "https://api.github.com/";
+
+/// How many of those redirects are followed.
+const REDIRECTS: usize = 3;
+
+/// Why a check found nothing.
+#[derive(Clone, Debug, PartialEq, Eq)]
+pub enum Failed {
+    /// No answer within this long.
+    TimedOut(Duration),
+    /// No connection to GitHub: offline, or its name did not resolve.
+    Unreachable,
+    /// GitHub answered with this status.
+    Status(u16),
+    /// An answer that was no release.
+    Unreadable,
+}
+
+impl Failed {
+    /// What the card says of it, under its title. English: the caller
+    /// translates, with `{secs}` and `{status}` put in after.
+    pub fn said(&self) -> &'static str {
+        match self {
+            Self::TimedOut(_) => {
+                "Couldn't reach github.com: the connection timed out after {secs} s."
+            }
+            Self::Unreachable => "Couldn't reach github.com. Check the connection to the internet.",
+            Self::Status(403 | 429) => {
+                "GitHub is limiting checks from this network. Try again later."
+            }
+            Self::Status(_) => "GitHub answered with an error ({status}).",
+            Self::Unreadable => "GitHub's answer wasn't a release.",
+        }
+    }
+
+    /// [`Failed::said`], with its numbers in.
+    pub fn with_numbers(&self, said: &str) -> String {
+        let secs = match self {
+            Self::TimedOut(after) => after.as_secs(),
+            Self::Unreachable | Self::Status(_) | Self::Unreadable => 0,
+        };
+        let status = match self {
+            Self::Status(status) => *status,
+            Self::TimedOut(_) | Self::Unreachable | Self::Unreadable => 0,
+        };
+        said.replace("{secs}", &secs.to_string())
+            .replace("{status}", &status.to_string())
+    }
+}
+
+/// Asks `url` for the latest release, sending `user_agent` and nothing
+/// else of the person: no cookie, no id. Blocks for up to `timeout`.
+pub fn latest(url: &str, user_agent: &str, timeout: Duration) -> Result<Latest, Failed> {
+    let agent: ureq::Agent = ureq::Agent::config_builder()
+        .http_status_as_error(false)
+        // Followed here, and only to the API's own host.
+        .max_redirects(0)
+        .timeout_global(Some(timeout))
+        .build()
+        .into();
+    let mut url = url.to_owned();
+    let mut redirects = 0;
+    let response = loop {
+        let response = agent
+            .get(&url)
+            .header("Accept", "application/vnd.github+json")
+            .header("User-Agent", user_agent)
+            .call()
+            .map_err(|error| match error {
+                ureq::Error::Timeout(_) => Failed::TimedOut(timeout),
+                _ => Failed::Unreachable,
+            })?;
+        let status = response.status().as_u16();
+        let location = response
+            .headers()
+            .get("location")
+            .and_then(|value| value.to_str().ok())
+            .filter(|location| location.starts_with(API));
+        match (status, location) {
+            (200, _) => break response,
+            (301 | 302 | 307 | 308, Some(location)) if redirects < REDIRECTS => {
+                url = location.to_owned();
+                redirects += 1;
+            }
+            _ => return Err(Failed::Status(status)),
+        }
+    };
+    let mut body = Vec::new();
+    response
+        .into_body()
+        .into_reader()
+        .take(LIMIT)
+        .read_to_end(&mut body)
+        .map_err(|error| {
+            // ureq says a timeout in the body as its own error, inside.
+            let inner = error
+                .get_ref()
+                .and_then(|inner| inner.downcast_ref::<ureq::Error>());
+            match (error.kind(), inner) {
+                (std::io::ErrorKind::TimedOut, _) | (_, Some(ureq::Error::Timeout(_))) => {
+                    Failed::TimedOut(timeout)
+                }
+                _ => Failed::Unreachable,
+            }
+        })?;
+    let latest: Latest = serde_json::from_slice(&body).map_err(|_| Failed::Unreadable)?;
+    if latest.tag_name.is_empty() {
+        return Err(Failed::Unreadable);
+    }
+    Ok(latest)
+}
+
+/// What the check says of this copy: `Tabletist/0.3.0 (macOS 15.1;
+/// arm64)`, `Tabletist/0.3.0 (Linux; x86_64)`. `macos` is the system's
+/// version on a Mac.
+pub fn user_agent(version: &str, macos: Option<&str>) -> String {
+    let chip = match std::env::consts::ARCH {
+        // As Apple names the chip.
+        "aarch64" if cfg!(target_os = "macos") => "arm64",
+        arch => arch,
+    };
+    let system = if cfg!(target_os = "macos") {
+        match macos {
+            Some(version) => format!("macOS {version}"),
+            None => "macOS".to_owned(),
+        }
+    } else if cfg!(windows) {
+        "Windows".to_owned()
+    } else if cfg!(target_os = "linux") {
+        "Linux".to_owned()
+    } else {
+        std::env::consts::OS.to_owned()
+    };
+    format!("Tabletist/{version} ({system}; {chip})")
+}
+
+/// The Mac's system version (`15.1`), asked of `sw_vers` once. None
+/// elsewhere, or where it says nothing.
+pub fn macos_version() -> Option<String> {
+    if !cfg!(target_os = "macos") {
+        return None;
+    }
+    static VERSION: std::sync::OnceLock<Option<String>> = std::sync::OnceLock::new();
+    VERSION
+        .get_or_init(|| {
+            let output = std::process::Command::new("/usr/bin/sw_vers")
+                .arg("-productVersion")
+                .output()
+                .ok()?;
+            let version = String::from_utf8(output.stdout).ok()?.trim().to_owned();
+            (!version.is_empty()).then_some(version)
+        })
+        .clone()
+}
+
+#[cfg(test)]
+pub(crate) mod tests {
+    use std::io::{BufRead, BufReader, Write};
+    use std::net::TcpListener;
+    use std::thread::JoinHandle;
+
+    use super::*;
+
+    /// The latest release as the fake server gives it: Bookshop's notes
+    /// are the app's own.
+    pub(crate) const RELEASE: &str = include_str!("../../tests/fixtures/update/latest.json");
+
+    /// Answers one request on a local port with `answer`, after `wait`,
+    /// and gives back the request's lines, the request line first.
+    pub(crate) fn serve_once(answer: String, wait: Duration) -> (String, JoinHandle<Vec<String>>) {
+        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
+        let url = format!(
+            "http://{}/repos/igor-alexandrov/tabletist/releases/latest",
+            listener.local_addr().unwrap()
+        );
+        let server = std::thread::spawn(move || {
+            let (stream, _) = listener.accept().unwrap();
+            let mut reader = BufReader::new(stream.try_clone().unwrap());
+            let mut head = Vec::new();
+            loop {
+                let mut line = String::new();
+                reader.read_line(&mut line).unwrap();
+                if line == "\r\n" || line.is_empty() {
+                    break;
+                }
+                head.push(line.trim_end().to_owned());
+            }
+            std::thread::sleep(wait);
+            // The client may have given up by now.
+            let _ = (&stream).write_all(answer.as_bytes());
+            head
+        });
+        (url, server)
+    }
+
+    pub(crate) fn ok(body: &str) -> String {
+        format!(
+            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
+            body.len()
+        )
+    }
+
+    #[test]
+    fn the_check_reads_the_latest_release() {
+        let (url, server) = serve_once(ok(RELEASE), Duration::ZERO);
+        let latest = latest(&url, "Tabletist/0.3.0 (Linux; x86_64)", TIMEOUT).unwrap();
+        server.join().unwrap();
+        assert_eq!(latest.tag_name, "v0.4.0");
+        assert!(!latest.assets.is_empty());
+    }
+
+    /// Only the user agent says anything: no cookie, no id, no other
+    /// header of the client's.
+    #[test]
+    fn the_check_sends_the_user_agent_and_nothing_else() {
+        let (url, server) = serve_once(ok(RELEASE), Duration::ZERO);
+        latest(&url, "Tabletist/0.3.0 (Linux; x86_64)", TIMEOUT).unwrap();
+        let head = server.join().unwrap();
+        assert_eq!(
+            head[0],
+            "GET /repos/igor-alexandrov/tabletist/releases/latest HTTP/1.1"
+        );
+        let mut names: Vec<String> = head[1..]
+            .iter()
+            .map(|line| line.split(':').next().unwrap().to_ascii_lowercase())
+            .collect();
+        names.sort();
+        assert_eq!(names, ["accept", "host", "user-agent"], "{head:?}");
+        let sent = |line: &str| head.iter().any(|sent| sent.eq_ignore_ascii_case(line));
+        assert!(
+            sent("user-agent: Tabletist/0.3.0 (Linux; x86_64)"),
+            "{head:?}"
+        );
+        assert!(sent("accept: application/vnd.github+json"), "{head:?}");
+    }
+
+    #[test]
+    fn no_answer_in_time_is_a_timeout() {
+        let (url, server) = serve_once(ok(RELEASE), Duration::from_millis(1_500));
+        let wait = Duration::from_millis(200);
+        assert_eq!(
+            latest(&url, "Tabletist/0.3.0", wait),
+            Err(Failed::TimedOut(wait))
+        );
+        server.join().unwrap();
+    }
+
+    #[test]
+    fn no_server_is_unreachable() {
+        // A port nothing listens on.
+        let port = TcpListener::bind("127.0.0.1:0")
+            .unwrap()
+            .local_addr()
+            .unwrap()
+            .port();
+        let url = format!("http://127.0.0.1:{port}/releases/latest");
+        assert_eq!(
+            latest(&url, "Tabletist/0.3.0", TIMEOUT),
+            Err(Failed::Unreachable)
+        );
+    }
+
+    #[test]
+    fn an_error_status_or_no_release_fails_the_check() {
+        let (url, server) = serve_once(
+            "HTTP/1.1 404 Not Found\r\nContent-Length: 9\r\nConnection: close\r\n\r\nnot found"
+                .into(),
+            Duration::ZERO,
+        );
+        assert_eq!(
+            latest(&url, "Tabletist/0.3.0", TIMEOUT),
+            Err(Failed::Status(404))
+        );
+        server.join().unwrap();
+        let (url, server) = serve_once(ok("{\"message\":\"hi\"}"), Duration::ZERO);
+        assert_eq!(
+            latest(&url, "Tabletist/0.3.0", TIMEOUT),
+            Err(Failed::Unreadable)
+        );
+        server.join().unwrap();
+    }
+
+    #[test]
+    fn the_user_agent_names_the_version_the_system_and_the_chip() {
+        let agent = user_agent("0.3.0", Some("15.1"));
+        if cfg!(target_os = "linux") {
+            assert_eq!(
+                agent,
+                format!("Tabletist/0.3.0 (Linux; {})", std::env::consts::ARCH)
+            );
+        }
+        if cfg!(target_os = "macos") {
+            assert!(
+                agent.starts_with("Tabletist/0.3.0 (macOS 15.1; "),
+                "{agent}"
+            );
+        }
+    }
+
+    #[test]
+    fn what_a_failure_says_has_its_numbers() {
+        let failed = Failed::TimedOut(TIMEOUT);
+        assert_eq!(
+            failed.with_numbers(failed.said()),
+            "Couldn't reach github.com: the connection timed out after 10 s."
+        );
+        let failed = Failed::Status(502);
+        assert_eq!(
+            failed.with_numbers(failed.said()),
+            "GitHub answered with an error (502)."
+        );
+    }
+
+    /// A redirect to a host that is not the API's is not followed: the
+    /// request and its User-Agent go to GitHub's API only.
+    #[test]
+    fn a_redirect_to_another_host_is_not_followed() {
+        let (elsewhere, _other) = serve_once(ok(RELEASE), Duration::ZERO);
+        // The same machine by another name: another host.
+        let elsewhere = elsewhere.replace("127.0.0.1", "localhost");
+        let (url, server) = serve_once(
+            format!(
+                "HTTP/1.1 302 Found\r\nLocation: {elsewhere}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
+            ),
+            Duration::ZERO,
+        );
+        let result = latest(&url, "Tabletist/0.3.0 (Linux; x86_64)", TIMEOUT);
+        server.join().unwrap();
+        assert!(
+            result.is_err(),
+            "followed the redirect to {elsewhere}: {result:?}"
+        );
+    }
+
+    /// Headers in time, then the body stalls: still a timeout, said as one.
+    #[test]
+    fn a_body_that_stalls_is_a_timeout() {
+        use std::io::Write as _;
+        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
+        let url = format!("http://{}/releases/latest", listener.local_addr().unwrap());
+        let server = std::thread::spawn(move || {
+            let (mut stream, _) = listener.accept().unwrap();
+            let mut buf = [0u8; 4096];
+            let _ = std::io::Read::read(&mut stream, &mut buf);
+            let _ = stream.write_all(
+                b"HTTP/1.1 200 OK\r\nContent-Length: 100000\r\nConnection: close\r\n\r\n{\"tag",
+            );
+            std::thread::sleep(Duration::from_millis(1_500));
+        });
+        let wait = Duration::from_millis(300);
+        let started = std::time::Instant::now();
+        let result = latest(&url, "Tabletist/0.3.0", wait);
+        assert!(
+            started.elapsed() < Duration::from_secs(1),
+            "{:?}",
+            started.elapsed()
+        );
+        assert_eq!(result, Err(Failed::TimedOut(wait)));
+        server.join().unwrap();
+    }
+}
```

`src/update/installed.rs`:

```diff
diff --git a/src/update/installed.rs b/src/update/installed.rs
new file mode 100644
index 00000000..130d1f2b
--- /dev/null
+++ b/src/update/installed.rs
@@ -0,0 +1,215 @@
+//! How this copy was installed, and so who updates it: itself, or the
+//! tool that installed it, with the command that does it.
+
+use fastframe_update::{Kind, PackageManager, Unsupported};
+
+/// Who updates this copy.
+#[derive(Clone, Debug, PartialEq, Eq)]
+pub enum Installed {
+    /// It replaces itself: a copy from the download page.
+    Itself(Kind),
+    /// Something else does, or nothing can: the pane checks and says so.
+    Elsewhere(Managed),
+}
+
+/// What updates a copy that does not update itself, as the card says it.
+#[derive(Clone, Debug, PartialEq, Eq)]
+pub struct Managed {
+    /// The line under the card's title. English: the caller translates.
+    pub said: &'static str,
+    /// What to run, where it is known.
+    pub command: Option<&'static str>,
+}
+
+impl Installed {
+    /// What detecting the installation came to. `found` says whether a
+    /// program is on the `PATH`: an AUR helper's update covers the AUR's
+    /// packages, which pacman's does not.
+    pub fn of(detected: Result<Kind, &Unsupported>, found: impl Fn(&str) -> bool) -> Self {
+        let reason = match detected {
+            Ok(kind) => return Self::Itself(kind),
+            Err(reason) => reason,
+        };
+        let managed = |said, command| Self::Elsewhere(Managed { said, command });
+        let package = "Your package manager updates Tabletist";
+        match reason {
+            Unsupported::Homebrew => managed(
+                "Installed with Homebrew, so Homebrew updates it",
+                Some("brew upgrade --cask tabletist"),
+            ),
+            Unsupported::Cargo => managed(
+                "Installed with cargo, so cargo updates it",
+                Some("cargo install tabletist --locked"),
+            ),
+            Unsupported::SystemPackage(PackageManager::Pacman) => {
+                let command = ["yay", "paru"]
+                    .into_iter()
+                    .find(|helper| found(helper))
+                    .map_or("sudo pacman -Syu", |helper| match helper {
+                        "paru" => "paru -Syu",
+                        _ => "yay -Syu",
+                    });
+                managed(package, Some(command))
+            }
+            Unsupported::SystemPackage(PackageManager::Apt) => {
+                managed(package, Some("sudo apt update && sudo apt upgrade"))
+            }
+            Unsupported::SystemPackage(PackageManager::Dnf) => {
+                managed(package, Some("sudo dnf upgrade"))
+            }
+            Unsupported::Flatpak => managed(package, Some("flatpak update")),
+            Unsupported::Snap => managed(package, Some("sudo snap refresh")),
+            Unsupported::Nix | Unsupported::SystemDirectory => managed(package, None),
+            Unsupported::MoveToApplications => managed(
+                "Move Tabletist to Applications, then open it from there to update it",
+                None,
+            ),
+            Unsupported::NotPortable
+            | Unsupported::NotWritable
+            | Unsupported::ForeignBundle
+            | Unsupported::Platform
+            | Unsupported::Unavailable(_) => managed("Tabletist can't replace itself here", None),
+            // A reason of a later fastframe-update's.
+            _ => managed("Tabletist can't replace itself here", None),
+        }
+    }
+
+    /// Asks fastframe-update how the running copy was installed: it reads
+    /// the disk and may ask the package manager, so off the interface
+    /// thread, once per launch.
+    pub fn detect() -> Self {
+        let detected = super::updater().installation();
+        Self::of(
+            detected.as_ref().map(|installation| installation.kind),
+            on_path,
+        )
+    }
+
+    /// The command to copy, where something else updates this copy.
+    pub fn command(&self) -> Option<&'static str> {
+        match self {
+            Self::Itself(_) => None,
+            Self::Elsewhere(managed) => managed.command,
+        }
+    }
+
+    /// The kind, where it updates itself.
+    pub fn kind(&self) -> Option<Kind> {
+        match self {
+            Self::Itself(kind) => Some(*kind),
+            Self::Elsewhere(_) => None,
+        }
+    }
+}
+
+/// Whether a program of this name is in a folder of the `PATH`.
+fn on_path(name: &str) -> bool {
+    std::env::var_os("PATH").is_some_and(|folders| {
+        std::env::split_paths(&folders).any(|folder| folder.join(name).is_file())
+    })
+}
+
+#[cfg(test)]
+mod tests {
+    use super::*;
+
+    fn none(_: &str) -> bool {
+        false
+    }
+
+    fn command(
+        detected: Result<Kind, &Unsupported>,
+        found: impl Fn(&str) -> bool,
+    ) -> Option<&'static str> {
+        Installed::of(detected, found).command()
+    }
+
+    #[test]
+    fn homebrew_and_cargo_say_their_own_command() {
+        let homebrew = Installed::of(Err(&Unsupported::Homebrew), none);
+        assert_eq!(
+            homebrew,
+            Installed::Elsewhere(Managed {
+                said: "Installed with Homebrew, so Homebrew updates it",
+                command: Some("brew upgrade --cask tabletist"),
+            })
+        );
+        assert_eq!(
+            command(Err(&Unsupported::Cargo), none),
+            Some("cargo install tabletist --locked")
+        );
+    }
+
+    #[test]
+    fn a_package_manager_updates_it_with_its_command_where_known() {
+        let pacman = Unsupported::SystemPackage(PackageManager::Pacman);
+        assert_eq!(command(Err(&pacman), none), Some("sudo pacman -Syu"));
+        // An AUR helper's update covers the AUR's packages too.
+        assert_eq!(
+            command(Err(&pacman), |name| name == "yay"),
+            Some("yay -Syu")
+        );
+        assert_eq!(
+            command(Err(&pacman), |name| name == "paru"),
+            Some("paru -Syu")
+        );
+        let apt = Unsupported::SystemPackage(PackageManager::Apt);
+        assert_eq!(
+            command(Err(&apt), none),
+            Some("sudo apt update && sudo apt upgrade")
+        );
+        for unknown in [Unsupported::Nix, Unsupported::SystemDirectory] {
+            let installed = Installed::of(Err(&unknown), none);
+            assert_eq!(
+                installed,
+                Installed::Elsewhere(Managed {
+                    said: "Your package manager updates Tabletist",
+                    command: None,
+                }),
+                "{unknown:?}"
+            );
+        }
+    }
+
+    #[test]
+    fn a_copy_it_cannot_write_to_or_does_not_know_cannot_replace_itself() {
+        for reason in [
+            Unsupported::NotWritable,
+            Unsupported::NotPortable,
+            Unsupported::Unavailable("no".into()),
+        ] {
+            assert_eq!(
+                Installed::of(Err(&reason), none),
+                Installed::Elsewhere(Managed {
+                    said: "Tabletist can't replace itself here",
+                    command: None,
+                }),
+                "{reason:?}"
+            );
+        }
+    }
+
+    #[test]
+    fn a_copy_from_the_download_page_updates_itself() {
+        for kind in [Kind::Portable, Kind::MacBundle, Kind::AppImage] {
+            let installed = Installed::of(Ok(kind), none);
+            assert_eq!(installed, Installed::Itself(kind));
+            assert_eq!(installed.command(), None);
+            assert_eq!(installed.kind(), Some(kind));
+        }
+    }
+
+    /// The detection itself, on a file of the shape `cargo install`
+    /// leaves: `~/.cargo/bin/tabletist`.
+    #[cfg(unix)]
+    #[test]
+    fn a_binary_cargo_installed_is_cargos_to_update() {
+        let home = tempfile::tempdir().unwrap();
+        let bin = home.path().join(".cargo/bin");
+        std::fs::create_dir_all(&bin).unwrap();
+        let executable = bin.join("tabletist");
+        std::fs::write(&executable, b"").unwrap();
+        let detected = super::super::updater().installation_at(&executable);
+        assert_eq!(detected, Err(Unsupported::Cargo));
+    }
+}
```

`src/update/release.rs`:

```diff
diff --git a/src/update/release.rs b/src/update/release.rs
new file mode 100644
index 00000000..20bf85af
--- /dev/null
+++ b/src/update/release.rs
@@ -0,0 +1,426 @@
+//! The latest release as GitHub's API says it, and what Settings ›
+//! Updates makes of it: whether it is newer than this version, what it
+//! says is new, and the download this computer would take.
+
+use serde::{Deserialize, Serialize};
+
+/// The fields of `GET /repos/<owner>/<name>/releases/latest` the pane
+/// reads. Anything else in the answer is left.
+#[derive(Clone, Debug, Default, Deserialize, PartialEq, Eq)]
+pub struct Latest {
+    #[serde(default)]
+    pub tag_name: String,
+    #[serde(default)]
+    pub html_url: String,
+    #[serde(default)]
+    pub draft: bool,
+    #[serde(default)]
+    pub prerelease: bool,
+    /// The release's notes, in Markdown.
+    #[serde(default)]
+    pub body: Option<String>,
+    /// When it was published, as RFC 3339.
+    #[serde(default)]
+    pub published_at: Option<String>,
+    #[serde(default)]
+    pub assets: Vec<Asset>,
+}
+
+/// A file of a release.
+#[derive(Clone, Debug, Default, Deserialize, PartialEq, Eq)]
+pub struct Asset {
+    #[serde(default)]
+    pub name: String,
+    /// In bytes.
+    #[serde(default)]
+    pub size: u64,
+}
+
+/// A release, as the pane and the state file keep it.
+#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
+pub struct Offer {
+    /// Its version, without the tag's `v`: `0.4.0`.
+    pub version: String,
+    /// Its page, with the notes and every download.
+    pub url: String,
+    /// The first [`NOTES`] points of its notes, each as Markdown.
+    #[serde(default)]
+    pub notes: Vec<String>,
+    /// All of its notes, as Markdown: what the pager shows.
+    #[serde(default)]
+    pub body: String,
+    /// When it was published, in seconds since the Unix epoch.
+    #[serde(default)]
+    pub published: Option<u64>,
+    /// The download this computer would take, and its size in bytes.
+    /// None where the release has no build for it.
+    #[serde(default)]
+    pub download: Option<(String, u64)>,
+}
+
+/// How many points of a release's notes the pane shows.
+pub const NOTES: usize = 4;
+
+/// What a check found, against the version that runs.
+#[derive(Clone, Debug, PartialEq, Eq)]
+pub enum Found {
+    /// A newer release, to offer.
+    Newer(Offer),
+    /// Nothing newer, or only the version the person said to skip.
+    UpToDate,
+}
+
+/// The version a tag names: `v0.4.0` and `0.4.0` are `0.4.0`.
+pub fn version_of(tag: &str) -> &str {
+    tag.strip_prefix('v').unwrap_or(tag)
+}
+
+/// What `latest` is to the version `current`: newer, or not. A draft or a
+/// pre-release is never offered (GitHub's latest release is neither, but
+/// a feed could say so). A release equal to `skip` is not offered while
+/// `skip` holds; one newer than it is.
+pub fn found(
+    latest: &Latest,
+    current: &str,
+    skip: Option<&str>,
+    os: Os,
+    kind: Option<Kind>,
+) -> Found {
+    if latest.draft || latest.prerelease {
+        return Found::UpToDate;
+    }
+    let version = version_of(&latest.tag_name);
+    if !fastframe_update::is_newer(version, current) || skip == Some(version) {
+        return Found::UpToDate;
+    }
+    Found::Newer(offer(latest, os, kind))
+}
+
+/// `latest` as the pane keeps it, with the download for `os` and `kind`.
+pub fn offer(latest: &Latest, os: Os, kind: Option<Kind>) -> Offer {
+    let version = version_of(&latest.tag_name).to_owned();
+    let download = asset_name(&version, os, kind).and_then(|name| {
+        latest
+            .assets
+            .iter()
+            .find(|asset| asset.name == name)
+            .map(|asset| (asset.name.clone(), asset.size))
+    });
+    Offer {
+        url: latest.html_url.clone(),
+        notes: notes(latest.body.as_deref().unwrap_or_default()),
+        body: latest.body.clone().unwrap_or_default(),
+        published: latest
+            .published_at
+            .as_deref()
+            .and_then(|at| at.parse::<jiff::Timestamp>().ok())
+            .and_then(|at| u64::try_from(at.as_second()).ok()),
+        download,
+        version,
+    }
+}
+
+/// The first [`NOTES`] points of a release's notes, each without its
+/// bullet: a line that starts with `- `, `* ` or `+ `, at any depth.
+pub fn notes(body: &str) -> Vec<String> {
+    body.lines()
+        .map(str::trim_start)
+        .filter_map(|line| {
+            ["- ", "* ", "+ "]
+                .iter()
+                .find_map(|bullet| line.strip_prefix(bullet))
+        })
+        .map(str::trim)
+        .filter(|point| !point.is_empty())
+        .take(NOTES)
+        .map(str::to_owned)
+        .collect()
+}
+
+/// A point of the notes in runs of text, each strong or not: `**bold**`
+/// and `__bold__` are strong, a link is its words, code is its text.
+pub fn runs(point: &str) -> Vec<(String, bool)> {
+    let mut runs: Vec<(String, bool)> = Vec::new();
+    let mut strong = false;
+    let mut text = String::new();
+    let mut rest = point;
+    let flush = |runs: &mut Vec<(String, bool)>, text: &mut String, strong: bool| {
+        if !text.is_empty() {
+            runs.push((std::mem::take(text), strong));
+        }
+    };
+    while let Some(next) = rest.chars().next() {
+        if rest.starts_with("**") || rest.starts_with("__") {
+            flush(&mut runs, &mut text, strong);
+            strong = !strong;
+            rest = &rest[2..];
+            continue;
+        }
+        match next {
+            // Code is its text, as written.
+            '`' if let Some(end) = rest[1..].find('`') => {
+                text.push_str(&rest[1..=end]);
+                rest = &rest[end + 2..];
+            }
+            '`' => rest = &rest[1..],
+            // `[words](url)`: the words, where no other bracket comes
+            // between.
+            '[' if let Some(close) = rest.find("](")
+                && !rest[1..close].contains(['[', ']'])
+                && let Some(end) = rest[close..].find(')') =>
+            {
+                text.push_str(&rest[1..close]);
+                rest = &rest[close + end + 1..];
+            }
+            _ => {
+                text.push(next);
+                rest = &rest[next.len_utf8()..];
+            }
+        }
+    }
+    flush(&mut runs, &mut text, strong);
+    runs
+}
+
+/// The systems a release builds for.
+#[derive(Clone, Copy, Debug, PartialEq, Eq)]
+pub enum Os {
+    Linux,
+    MacOs,
+    Windows,
+    /// One no release builds for.
+    Other,
+}
+
+impl Os {
+    /// The one this runs on.
+    pub fn current() -> Self {
+        if cfg!(target_os = "linux") {
+            Self::Linux
+        } else if cfg!(target_os = "macos") {
+            Self::MacOs
+        } else if cfg!(windows) {
+            Self::Windows
+        } else {
+            Self::Other
+        }
+    }
+}
+
+/// How this copy was installed, where it updates itself, which picks its
+/// download.
+pub use fastframe_update::Kind;
+
+/// The name of the download a release has for `os` on this processor,
+/// for a copy installed as `kind` (None while that is not known yet: the
+/// archive). As `release.yml` names them, and as fastframe-update
+/// downloads them.
+pub fn asset_name(version: &str, os: Os, kind: Option<Kind>) -> Option<String> {
+    asset_name_on(version, os, std::env::consts::ARCH, kind)
+}
+
+fn asset_name_on(version: &str, os: Os, arch: &str, kind: Option<Kind>) -> Option<String> {
+    let arch = match arch {
+        "x86_64" | "aarch64" => arch,
+        _ if os == Os::MacOs => arch,
+        _ => return None,
+    };
+    match (os, kind) {
+        (Os::MacOs, _) => Some(format!("tabletist-v{version}-macos-universal.dmg")),
+        (Os::Linux, Some(Kind::AppImage)) => Some(format!("tabletist-{version}-{arch}.AppImage")),
+        (Os::Linux, _) => Some(format!(
+            "tabletist-v{version}-{arch}-unknown-linux-gnu.tar.gz"
+        )),
+        (Os::Windows, Some(Kind::WindowsInstaller)) => Some(format!(
+            "tabletist-v{version}-{arch}-pc-windows-msvc-setup.exe"
+        )),
+        (Os::Windows, _) => Some(format!("tabletist-v{version}-{arch}-pc-windows-msvc.zip")),
+        (Os::Other, _) => None,
+    }
+}
+
+/// A size as the pane writes it: `31.2 MB`.
+pub fn megabytes(bytes: u64) -> String {
+    // Shown to a tenth: the precision of f64 is no matter here.
+    #[allow(clippy::cast_precision_loss)]
+    let megabytes = bytes as f64 / 1_000_000.0;
+    format!("{megabytes:.1} MB")
+}
+
+#[cfg(test)]
+mod tests {
+    use super::*;
+
+    fn latest(tag: &str) -> Latest {
+        Latest {
+            tag_name: tag.into(),
+            html_url: format!("https://github.com/igor-alexandrov/tabletist/releases/tag/{tag}"),
+            body: Some("## What's new\n\n- **Edit rows** in the grid\n- one\n* two\n  - three\n- four\n- five".into()),
+            published_at: Some("2026-10-24T03:16:13Z".into()),
+            assets: vec![
+                Asset {
+                    name: "tabletist-v0.4.0-x86_64-unknown-linux-gnu.tar.gz".into(),
+                    size: 31_200_000,
+                },
+                Asset {
+                    name: "tabletist-v0.4.0-macos-universal.dmg".into(),
+                    size: 41_000_000,
+                },
+            ],
+            ..Latest::default()
+        }
+    }
+
+    #[test]
+    fn a_newer_tag_is_offered_and_an_equal_or_older_one_is_not() {
+        let linux = |tag: &str, current: &str| found(&latest(tag), current, None, Os::Linux, None);
+        assert!(matches!(linux("v0.4.0", "0.3.0"), Found::Newer(_)));
+        // Without the `v` too.
+        assert!(matches!(linux("0.4.0", "0.3.0"), Found::Newer(_)));
+        assert_eq!(linux("v0.3.0", "0.3.0"), Found::UpToDate);
+        assert_eq!(linux("v0.2.2", "0.3.0"), Found::UpToDate);
+        assert!(matches!(linux("v0.10.0", "0.9.9"), Found::Newer(_)));
+        // A tag that is no version.
+        assert_eq!(linux("nightly", "0.3.0"), Found::UpToDate);
+    }
+
+    #[test]
+    fn drafts_and_pre_releases_are_never_offered() {
+        let mut draft = latest("v0.4.0");
+        draft.draft = true;
+        assert_eq!(
+            found(&draft, "0.3.0", None, Os::Linux, None),
+            Found::UpToDate
+        );
+        let mut pre = latest("v0.4.0");
+        pre.prerelease = true;
+        assert_eq!(found(&pre, "0.3.0", None, Os::Linux, None), Found::UpToDate);
+        // A pre-release's tag, unflagged, is no newer stable version.
+        assert_eq!(
+            found(&latest("v0.4.0-rc.1"), "0.3.0", None, Os::Linux, None),
+            Found::UpToDate
+        );
+    }
+
+    #[test]
+    fn a_skipped_version_is_not_offered_and_a_newer_one_is() {
+        let skip = Some("0.4.0");
+        assert_eq!(
+            found(&latest("v0.4.0"), "0.3.0", skip, Os::Linux, None),
+            Found::UpToDate
+        );
+        assert!(matches!(
+            found(&latest("v0.4.1"), "0.3.0", skip, Os::Linux, None),
+            Found::Newer(_)
+        ));
+    }
+
+    #[test]
+    fn an_offer_has_the_first_four_points_its_date_and_this_computers_download() {
+        let Found::Newer(offer) = found(&latest("v0.4.0"), "0.3.0", None, Os::Linux, None) else {
+            panic!("not offered");
+        };
+        assert_eq!(offer.version, "0.4.0");
+        assert_eq!(
+            offer.notes,
+            ["**Edit rows** in the grid", "one", "two", "three"]
+        );
+        assert_eq!(offer.published, Some(1_792_811_773));
+        if std::env::consts::ARCH == "x86_64" {
+            assert_eq!(
+                offer.download,
+                Some((
+                    "tabletist-v0.4.0-x86_64-unknown-linux-gnu.tar.gz".into(),
+                    31_200_000
+                ))
+            );
+        }
+        let mac = offer_on_mac();
+        assert_eq!(
+            mac.download,
+            Some(("tabletist-v0.4.0-macos-universal.dmg".into(), 41_000_000))
+        );
+        // Windows has no build in this release.
+        assert_eq!(
+            super::offer(&latest("v0.4.0"), Os::Windows, None).download,
+            None
+        );
+    }
+
+    fn offer_on_mac() -> Offer {
+        super::offer(&latest("v0.4.0"), Os::MacOs, Some(Kind::MacBundle))
+    }
+
+    #[test]
+    fn each_system_and_processor_takes_its_own_download() {
+        let name = |os, arch, kind| asset_name_on("0.4.0", os, arch, kind);
+        assert_eq!(
+            name(Os::Linux, "x86_64", Some(Kind::Portable)).as_deref(),
+            Some("tabletist-v0.4.0-x86_64-unknown-linux-gnu.tar.gz")
+        );
+        assert_eq!(
+            name(Os::Linux, "aarch64", None).as_deref(),
+            Some("tabletist-v0.4.0-aarch64-unknown-linux-gnu.tar.gz")
+        );
+        assert_eq!(
+            name(Os::Linux, "x86_64", Some(Kind::AppImage)).as_deref(),
+            Some("tabletist-0.4.0-x86_64.AppImage")
+        );
+        // One universal disk image, whichever chip.
+        for arch in ["aarch64", "x86_64"] {
+            assert_eq!(
+                name(Os::MacOs, arch, Some(Kind::MacBundle)).as_deref(),
+                Some("tabletist-v0.4.0-macos-universal.dmg")
+            );
+        }
+        assert_eq!(
+            name(Os::Windows, "x86_64", Some(Kind::WindowsInstaller)).as_deref(),
+            Some("tabletist-v0.4.0-x86_64-pc-windows-msvc-setup.exe")
+        );
+        assert_eq!(
+            name(Os::Windows, "aarch64", Some(Kind::Portable)).as_deref(),
+            Some("tabletist-v0.4.0-aarch64-pc-windows-msvc.zip")
+        );
+        // No release builds for these.
+        assert_eq!(name(Os::Linux, "riscv64", None), None);
+        assert_eq!(name(Os::Other, "x86_64", None), None);
+    }
+
+    #[test]
+    fn a_point_is_read_in_runs_of_strong_and_plain_text() {
+        assert_eq!(
+            runs("**Edit rows** in the grid, `NULL` and [the docs](https://x)"),
+            [
+                ("Edit rows".to_owned(), true),
+                (" in the grid, NULL and the docs".to_owned(), false),
+            ]
+        );
+        assert_eq!(runs("plain"), [("plain".to_owned(), false)]);
+    }
+
+    #[test]
+    fn a_size_is_written_in_megabytes_to_a_tenth() {
+        assert_eq!(megabytes(31_200_000), "31.2 MB");
+        assert_eq!(megabytes(29_849_999), "29.8 MB");
+    }
+
+    /// A bracket before a link is kept as written, and the link's words
+    /// are its own.
+    #[test]
+    fn a_bracket_before_a_link_is_kept_as_written() {
+        let text: String = runs("[breaking] Rename the key ([#12](https://x/12))")
+            .into_iter()
+            .map(|(run, _)| run)
+            .collect();
+        assert_eq!(text, "[breaking] Rename the key (#12)");
+    }
+
+    /// Code is its text: `__init__` in backticks is not strong.
+    #[test]
+    fn code_is_its_text() {
+        assert_eq!(
+            runs("Fix `__init__` handling"),
+            [("Fix __init__ handling".to_owned(), false)]
+        );
+    }
+}
```

`src/update/seen.rs`:

```diff
diff --git a/src/update/seen.rs b/src/update/seen.rs
new file mode 100644
index 00000000..9144210a
--- /dev/null
+++ b/src/update/seen.rs
@@ -0,0 +1,97 @@
+//! What the checks found, kept in the state folder (`updates.json`), not
+//! in the settings: when the last one that worked ran, the newest version
+//! seen and its notes, and when the status line last told of it.
+
+use serde::{Deserialize, Serialize};
+
+use super::release::Offer;
+
+/// The file's contents. Every field may be missing: a file of an older or
+/// a newer version reads as far as it goes.
+#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
+pub struct Seen {
+    /// When the last check that got an answer ran, in seconds since the
+    /// Unix epoch. A failed check leaves it.
+    #[serde(default)]
+    pub last_checked: Option<u64>,
+    /// The newest version a check found, newer than the version that ran
+    /// then or not.
+    #[serde(default)]
+    pub latest_seen: Option<String>,
+    /// That release, with its notes, while it is newer than this version.
+    #[serde(default)]
+    pub offer: Option<Offer>,
+    /// When the terminal look's status line last said a version is
+    /// available: at most once a day.
+    #[serde(default)]
+    pub announced: Option<u64>,
+}
+
+/// How often the status line tells of a version available, at most.
+pub const ANNOUNCE_EVERY: u64 = 24 * 60 * 60;
+
+impl Seen {
+    /// Reads `path`. A missing file is no check yet; one that does not
+    /// parse is kept aside, as every state file is.
+    pub fn load(path: &std::path::Path) -> Self {
+        crate::util::load_json(path)
+    }
+
+    /// The release to offer, when the file holds one newer than `current`.
+    pub fn offered(&self, current: &str) -> Option<&Offer> {
+        self.offer
+            .as_ref()
+            .filter(|offer| fastframe_update::is_newer(&offer.version, current))
+    }
+
+    /// Whether the status line may tell of a version at `now`. A time
+    /// after `now` (a clock that was ahead) does not hold it back.
+    pub fn may_announce(&self, now: u64) -> bool {
+        self.announced
+            .is_none_or(|then| then > now || now - then >= ANNOUNCE_EVERY)
+    }
+}
+
+#[cfg(test)]
+mod tests {
+    use super::*;
+
+    #[test]
+    fn an_offer_older_than_this_version_is_not_offered_again() {
+        let seen = Seen {
+            offer: Some(Offer {
+                version: "0.4.0".into(),
+                ..Offer::default()
+            }),
+            ..Seen::default()
+        };
+        assert!(seen.offered("0.3.0").is_some());
+        assert!(seen.offered("0.4.0").is_none());
+    }
+
+    #[test]
+    fn the_file_reads_as_far_as_it_goes() {
+        let dir = tempfile::tempdir().unwrap();
+        let path = dir.path().join("updates.json");
+        std::fs::write(&path, r#"{"last_checked": 1792811773, "later": true}"#).unwrap();
+        let seen = Seen::load(&path);
+        assert_eq!(seen.last_checked, Some(1_792_811_773));
+        assert_eq!(seen.offer, None);
+        // No file: no check yet.
+        assert_eq!(
+            Seen::load(&dir.path().join("missing.json")),
+            Seen::default()
+        );
+    }
+
+    #[test]
+    fn the_status_line_tells_at_most_once_a_day() {
+        let seen = Seen {
+            announced: Some(1_000),
+            ..Seen::default()
+        };
+        assert!(!seen.may_announce(1_000 + ANNOUNCE_EVERY - 1));
+        assert!(seen.may_announce(1_000 + ANNOUNCE_EVERY));
+        assert!(Seen::default().may_announce(0));
+    }
+}
```

- [ ] **Step 4: Run the checks**

Run: `cargo fmt --all --check && cargo clippy --locked --workspace --all-targets -- -D warnings && RUSTDOCFLAGS='-D warnings' cargo doc --locked --workspace --no-deps && cargo test --locked --workspace --all-targets`
Expected: clean; the library's tests `test result: ok. 3554 passed; 0 failed; 17 ignored`, `tests/cli.rs` 7 passed.

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml Cargo.lock AGENTS.md src/lib.rs src/entrypoint.rs src/update.rs src/update tests/cli.rs tests/fixtures/update/latest.json
git commit -m "Read the latest release, and confirm an update's relaunch"
```

---

### Task 2: When the app checks

**Files:** `src/settings.rs`, `src/update.rs`, `src/backend.rs`, `src/backend/updates.rs`, `src/app.rs`, `src/app/updates.rs`, `src/model.rs`, `src/paths.rs`, `src/ui/toast.rs`, `src/promo/live.rs`, `src/entrypoint.rs`, `AGENTS.md`.

`updates.check` (`daily`, `weekly`, `never`) and `updates.skip` (a version, with its `v` or without) are keys of `settings.toml`; they have no option row yet. `due` says how long until the next scheduled check, from the last that worked or the last scheduled one that failed, whichever came later. `App::tick_updates` runs each frame: where a check is due and no query runs it asks the backend, and it asks for a frame when the next is due. Only `App::watch_updates`, which the window calls outside a demo, turns the schedule on, so no test harness checks on its own. A check's answer is kept in `updates.json` (`StateFile::Updates`); a newer release found by a scheduled check, where the status line has not told of one for a day, is said there for 8 s in the Omarchy look (`toast::say_for`). The scheduled check also waits for the backend to say how this copy was installed (`UpdatesEvent::Detected`, within the launch), so the offer keeps the download of this copy's kind. Cancel drops the check's answer and, as a failure does, makes the schedule's next check wait an interval; Check now while a scheduled check runs makes that check one the person asked for. In the Omarchy look the status line tells of any version offered (found now or before this launch), once a day; the macOS look has none. `CheckForUpdates`, `CancelUpdateCheck`, `SkipUpdate` and `ShowReleaseNotes` are the actions Task 3's pane and keys push. Step 1 shows the tests of `src/settings.rs` and `src/update.rs`; `src/app/updates.rs` carries its own (the schedule, the detection, a failure, cancel, Check now during a check, skip, the status line, the pager).

- [ ] **Step 1: Write the failing tests**

`src/settings.rs`:

```diff
diff --git a/src/settings.rs b/src/settings.rs
index 7a7d5549..382d4936 100644
--- a/src/settings.rs
+++ b/src/settings.rs
@@ -3320,6 +3320,8 @@ mod tests {
             column_types: false,
             table_list: TableList::Flat,
             env_bar: EnvBar::Label,
+            update_check: UpdateCheck::Weekly,
+            update_skip: Some("0.4.0".into()),
             keys: crate::keymap::Overrides::default(),
         }
     }
@@ -3399,6 +3401,10 @@ table_list = \"flat\"  # tree | flat
 
 [appearance.env]
 bar = \"label\"  # tint | label
+
+[updates]
+check = \"weekly\"  # daily | weekly | never
+skip = \"0.4.0\"  # a version to stop offering
 "
             )
         );
@@ -3499,6 +3505,8 @@ bar = \"label\"  # tint | label
                 (Key::ColumnTypes, 59),
                 (Key::TableList, 60),
                 (Key::EnvBar, 63),
+                (Key::UpdatesCheck, 66),
+                (Key::UpdatesSkip, 67),
             ]
         );
     }
```

`src/update.rs`:

```diff
diff --git a/src/update.rs b/src/update.rs
index e4882b8f..2b8578fa 100644
--- a/src/update.rs
+++ b/src/update.rs
@@ -130,4 +130,107 @@ mod tests {
     fn appimages_update_themselves() {
         const { assert!(CONFIG.appimage) };
     }
+
+    const DAY: u64 = 24 * 60 * 60;
+
+    #[test]
+    fn a_daily_or_weekly_check_is_due_an_interval_after_the_last() {
+        let checked = |at| Seen {
+            last_checked: Some(at),
+            ..Seen::default()
+        };
+        let now = 100 * DAY;
+        // Never checked: due at once.
+        assert_eq!(
+            due(UpdateCheck::Daily, &Seen::default(), None, now),
+            Some(0)
+        );
+        assert_eq!(
+            due(UpdateCheck::Daily, &checked(now - 2 * DAY), None, now),
+            Some(0)
+        );
+        assert_eq!(
+            due(UpdateCheck::Daily, &checked(now - 3_600), None, now),
+            Some(DAY - 3_600)
+        );
+        assert_eq!(
+            due(UpdateCheck::Weekly, &checked(now - 2 * DAY), None, now),
+            Some(5 * DAY)
+        );
+        assert_eq!(
+            due(UpdateCheck::Weekly, &checked(now - 8 * DAY), None, now),
+            Some(0)
+        );
+        // Only when asked.
+        assert_eq!(due(UpdateCheck::Never, &Seen::default(), None, now), None);
+    }
+
+    /// A failed check is no check: the next scheduled one waits an
+    /// interval from the failure, and the last check that worked stays.
+    #[test]
+    fn a_check_that_failed_retries_at_the_next_interval() {
+        let seen = Seen {
+            last_checked: Some(0),
+            ..Seen::default()
+        };
+        let now = 10 * DAY;
+        assert_eq!(
+            due(UpdateCheck::Daily, &seen, Some(now - 60), now),
+            Some(DAY - 60)
+        );
+    }
+
+    fn offered(version: &str) -> Seen {
+        Seen {
+            offer: Some(Offer {
+                version: version.into(),
+                ..Offer::default()
+            }),
+            ..Seen::default()
+        }
+    }
+
+    #[test]
+    fn the_skipped_version_is_offered_only_after_a_check_that_was_asked_for() {
+        let updates = Updates {
+            seen: offered("99.0.0"),
+            ..Updates::default()
+        };
+        assert!(updates.offer(None).is_some());
+        assert!(updates.offer(Some("99.0.0")).is_none());
+        assert!(updates.offer(Some("98.0.0")).is_some());
+        let asked = Updates {
+            asked_offer: updates.seen.offer.clone(),
+            ..updates
+        };
+        assert!(asked.offer(Some("99.0.0")).is_some());
+        // Nothing older than this version, ever.
+        let old = Updates {
+            seen: offered("0.0.1"),
+            ..Updates::default()
+        };
+        assert!(old.offer(None).is_none());
+    }
+
+    /// A clock that was ahead once (a dead RTC battery, a VM) leaves the
+    /// last check "in the future": it counts as now, so the next check
+    /// waits an interval at most.
+    #[test]
+    fn a_last_check_in_the_future_counts_as_now() {
+        let now = 100 * DAY;
+        let seen = Seen {
+            last_checked: Some(now + 365 * DAY),
+            ..Seen::default()
+        };
+        let wait = due(UpdateCheck::Daily, &seen, None, now).unwrap();
+        assert!(wait <= DAY, "the next check waits {} days", wait / DAY);
+        let wait = due(
+            UpdateCheck::Daily,
+            &Seen::default(),
+            Some(now + 365 * DAY),
+            now,
+        )
+        .unwrap();
+        assert!(wait <= DAY, "after a failure: waits {} days", wait / DAY);
+    }
 }
```

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test --locked --lib --no-run`
Expected: no build, 28 errors, such as:

```
error[E0425]: cannot find function `due` in this scope
error[E0422]: cannot find struct, variant or union type `Updates` in this scope
error[E0560]: struct `settings::Settings` has no field named `update_check`
```

- [ ] **Step 3: Write the code**

`AGENTS.md`:

```diff
diff --git a/AGENTS.md b/AGENTS.md
index 7569727a..e67165aa 100644
--- a/AGENTS.md
+++ b/AGENTS.md
@@ -245,12 +245,14 @@ runtime by version and SHA-256; move them together. Locally,
 
 ## Updates
 
-`src/update.rs` configures fastframe-update, which installs an update, and
-holds the check: one GET of `releases/latest` that sends
-`Tabletist/<version> (<system>; <chip>)` and no other header of the
-person's (`src/update/check.rs` tests what goes on the wire). Tests never
-reach GitHub; they serve fixture JSON from a local port
-(`tests/fixtures/update/latest.json`).
+`src/update.rs` holds the check Settings › Updates shows and configures
+fastframe-update, which installs an update. The check is one GET of
+`releases/latest` that sends `Tabletist/<version> (<system>; <chip>)` and
+no other header of the person's: `src/update/check.rs` tests what goes on
+the wire. Tests never reach GitHub; they serve fixture JSON from a local
+port (`tests/fixtures/update/latest.json`). A test harness never checks on
+its own: only `App::watch_updates`, which the window calls outside a demo,
+turns the schedule on.
 
 The version that installs an update is the old one, and its helper runs
 the new one, so these hold for every release, and a change to any of them
```

`src/app.rs`:

```diff
diff --git a/src/app.rs b/src/app.rs
index f2938639..f0c958f2 100644
--- a/src/app.rs
+++ b/src/app.rs
@@ -39,6 +39,7 @@ mod menu;
 mod quick_open;
 #[cfg(test)]
 mod startup_tests;
+mod updates;
 
 pub use commands::{About, RUNNABLE, Runnable, TabName};
 pub use keyboard::{CHEAT_SHEET, cheat_sheet, keys_list};
@@ -267,6 +268,19 @@ pub struct App {
     /// which nothing in the person's files lifts. It is applied where the
     /// app tells the backend what to do, never written into those files.
     pub managed: crate::managed::Managed,
+    /// What the checks for updates found, and where the one under way
+    /// stands.
+    pub updates: crate::update::Updates,
+    /// Whether checks run on their own ([`App::watch_updates`]): not in
+    /// tests, nor in the demo.
+    pub updates_scheduled: bool,
+    /// Where a check asks: GitHub, or a test's server.
+    pub updates_url: String,
+    /// A version the status line is to tell of, by the next frame.
+    pub update_announcement: Option<String>,
+    /// A page the next frame opens in the browser: a release's, where its
+    /// notes could not be shown in a terminal.
+    pub url_to_open: Option<String>,
     next_id: u64,
 }
 
@@ -333,6 +347,10 @@ impl App {
         let (table_lists, table_lists_unread) = load_table_lists(&dirs.table_lists_file());
         let session = crate::session::Session::load(&dirs.session_file());
         let recents = crate::recents::Recents::load(&dirs.recents_file());
+        let updates = crate::update::Updates {
+            seen: crate::update::Seen::load(&dirs.updates_file()),
+            ..Default::default()
+        };
         let writes_session = loaded_connections.problem.is_none();
         // An installed data font is read once the window exists (`attach`).
         let look = crate::theme::Look::for_platform().with_text(&settings, false);
@@ -408,6 +426,11 @@ impl App {
             agents_shown,
             agents_held: Vec::new(),
             managed,
+            updates,
+            updates_scheduled: false,
+            updates_url: crate::update::check::LATEST.to_owned(),
+            update_announcement: None,
+            url_to_open: None,
             next_id: 1,
         };
         app.refresh_keymap();
@@ -2520,6 +2543,10 @@ impl App {
                     dialog.reveal = false;
                 }
             }
+            Action::CheckForUpdates
+            | Action::CancelUpdateCheck
+            | Action::SkipUpdate
+            | Action::ShowReleaseNotes => self.update_action(action),
             Action::ChooseAgentSetup(agent) => {
                 if let Some(Dialog::Settings(dialog)) = &mut self.dialog {
                     dialog.agent = agent;
@@ -4644,6 +4671,7 @@ impl App {
                 result,
             } => self.written(session, request, result),
             Event::History(event) => self.history_event(event),
+            Event::Updates(event) => self.updates_event(event),
             Event::SqlRan {
                 session,
                 request,
@@ -7889,6 +7917,10 @@ impl App {
         if let Some(wait) = self.close_idle(std::time::Instant::now()) {
             ctx.request_repaint_after(wait);
         }
+        if let Some(wait) = self.tick_updates() {
+            ctx.request_repaint_after(wait);
+        }
+        self.say_updates(ctx);
     }
 
     /// Closes the session of each connection left unused for longer than
```

`src/app/updates.rs`:

```diff
diff --git a/src/app/updates.rs b/src/app/updates.rs
new file mode 100644
index 00000000..c494083e
--- /dev/null
+++ b/src/app/updates.rs
@@ -0,0 +1,590 @@
+//! Updates, as the app keeps them: when a check runs (on launch where one
+//! is due, at the interval while the app is open, and when the person
+//! asks), what it found, and what the window says of it. The request is
+//! the backend's ([`crate::backend::UpdatesCommand`]); what the checks
+//! found is kept in the state folder (`updates.json`).
+
+use std::time::Duration;
+
+use super::App;
+use crate::backend::{Command, StateFile, UpdatesCommand, UpdatesEvent};
+use crate::model::{Action, ConnTabContent};
+use crate::update::{Failed, Found, Phase, release};
+
+/// What the terminal look's status line says of a version available,
+/// `{version}` for it, for [`ANNOUNCED_FOR`].
+pub const ANNOUNCEMENT: &str = "{version} available · :update";
+
+/// How long the status line says it.
+pub const ANNOUNCED_FOR: f64 = 8.0;
+
+impl App {
+    /// The time now, in seconds since the Unix epoch: the stopped clock in
+    /// a test.
+    pub(crate) fn now_secs(&self) -> u64 {
+        let now = self.stopped_clock.unwrap_or_else(jiff::Timestamp::now);
+        u64::try_from(now.as_second()).unwrap_or_default()
+    }
+
+    /// Starts what updates need once the window is up: how this copy was
+    /// installed is worked out, and the scheduled checks run from here on.
+    /// Not for a demo, whose profile is throwaway: it checks only when
+    /// asked.
+    pub fn watch_updates(&mut self) {
+        self.updates_scheduled = true;
+        self.backend.send(Command::Updates(UpdatesCommand::Detect));
+    }
+
+    /// Whether a query runs in a window: a scheduled check waits for it.
+    fn query_runs(&self) -> bool {
+        self.tabs.iter().any(|tab| match &tab.content {
+            ConnTabContent::Workspace(workspace) => workspace.waits(),
+            ConnTabContent::Picker(_) => false,
+        })
+    }
+
+    /// Runs the scheduled check where it is due, unless a query runs: then
+    /// it runs once none does. Says how long until the next is due, for a
+    /// frame to be asked for then.
+    pub fn tick_updates(&mut self) -> Option<Duration> {
+        if !self.updates_scheduled {
+            return None;
+        }
+        let now = self.now_secs();
+        self.announce_update(now);
+        // The asset an offer keeps is this copy's kind's: the check waits
+        // for the detection, which answers within the launch.
+        if self.updates.checking() || self.updates.installed.is_none() {
+            return None;
+        }
+        let due = crate::update::due(
+            self.settings.update_check,
+            &self.updates.seen,
+            self.updates.tried,
+            now,
+        )?;
+        if due > 0 {
+            return Some(Duration::from_secs(due));
+        }
+        if self.query_runs() {
+            // Looked at again in a second: the query's answer asks for a
+            // frame too.
+            return Some(Duration::from_secs(1));
+        }
+        self.start_update_check(false);
+        None
+    }
+
+    /// Asks GitHub for the latest release. `asked_for` where the person
+    /// asked: that check offers a version the settings skip.
+    fn start_update_check(&mut self, asked_for: bool) {
+        let asked = self.next_id();
+        self.updates.phase = Phase::Checking { asked, asked_for };
+        self.backend.send(Command::Updates(UpdatesCommand::Check {
+            asked,
+            url: self.updates_url.clone(),
+            timeout: crate::update::check::TIMEOUT,
+        }));
+    }
+
+    /// The actions of Settings › Updates.
+    pub(super) fn update_action(&mut self, action: Action) {
+        match action {
+            // One at a time: a check under way is answered first, as one
+            // the person asked for.
+            Action::CheckForUpdates => match &mut self.updates.phase {
+                Phase::Checking { asked_for, .. } => *asked_for = true,
+                Phase::Idle | Phase::Failed { .. } => self.start_update_check(true),
+            },
+            // The schedule's next check waits an interval from it, as
+            // from one that failed.
+            Action::CancelUpdateCheck => {
+                if self.updates.checking() {
+                    self.updates.phase = Phase::Idle;
+                    self.updates.tried = Some(self.now_secs());
+                }
+            }
+            Action::SkipUpdate => {
+                let skip = self.settings.update_skip.as_deref();
+                let Some(version) = self.updates.offer(skip).map(|offer| offer.version.clone())
+                else {
+                    return;
+                };
+                self.updates.asked_offer = None;
+                self.change_settings(|settings| settings.update_skip = Some(version));
+            }
+            Action::ShowReleaseNotes => {
+                let skip = self.settings.update_skip.as_deref();
+                let Some(offer) = self.updates.offer(skip).cloned() else {
+                    return;
+                };
+                self.backend
+                    .send(Command::Updates(UpdatesCommand::ShowNotes {
+                        path: self.dirs.release_notes_file(&offer.version),
+                        text: notes_text(&offer),
+                        url: offer.url,
+                    }));
+            }
+            _ => {}
+        }
+    }
+
+    pub(super) fn updates_event(&mut self, event: UpdatesEvent) {
+        match event {
+            UpdatesEvent::Detected(installed) => self.updates.installed = Some(installed),
+            UpdatesEvent::NotesUnshown { url } => self.url_to_open = Some(url),
+            UpdatesEvent::Checked { asked, result } => {
+                let Phase::Checking {
+                    asked: waited,
+                    asked_for,
+                } = self.updates.phase
+                else {
+                    return;
+                };
+                if waited != asked {
+                    return;
+                }
+                let now = self.now_secs();
+                match result {
+                    Ok(latest) => self.checked(&latest, asked_for, now),
+                    Err(failed) => self.check_failed(failed, asked_for, now),
+                }
+            }
+        }
+    }
+
+    /// A check that got its answer: kept as the last check, with the
+    /// release it found where that is newer.
+    fn checked(&mut self, latest: &release::Latest, asked_for: bool, now: u64) {
+        self.updates.phase = Phase::Idle;
+        self.updates.tried = None;
+        let kind = self.updates.installed.as_ref().and_then(|i| i.kind());
+        let os = release::Os::current();
+        let current = crate::update::CURRENT;
+        let skip = self.settings.update_skip.as_deref();
+        let seen = &mut self.updates.seen;
+        seen.last_checked = Some(now);
+        seen.latest_seen = Some(release::version_of(&latest.tag_name).to_owned());
+        // Kept whether it is skipped or not: the person may ask again.
+        let found = release::found(latest, current, None, os, kind);
+        seen.offer = match &found {
+            Found::Newer(offer) => Some(offer.clone()),
+            Found::UpToDate => None,
+        };
+        self.updates.asked_offer = match &found {
+            Found::Newer(offer) if asked_for && skip == Some(offer.version.as_str()) => {
+                Some(offer.clone())
+            }
+            Found::Newer(_) | Found::UpToDate => None,
+        };
+        self.save_updates();
+    }
+
+    /// In the terminal look, the status line tells of a version offered,
+    /// whether a check found it now or one before: at most once a day.
+    fn announce_update(&mut self, now: u64) {
+        if !self.look.terminal || !self.updates.seen.may_announce(now) {
+            return;
+        }
+        let skip = self.settings.update_skip.as_deref();
+        let Some(version) = self.updates.offer(skip).map(|offer| offer.version.clone()) else {
+            return;
+        };
+        self.updates.seen.announced = Some(now);
+        self.update_announcement = Some(version);
+        self.save_updates();
+    }
+
+    /// A check that got no answer: it does not count as checked, and a
+    /// scheduled one is tried again an interval later.
+    fn check_failed(&mut self, failed: Failed, asked_for: bool, now: u64) {
+        if !asked_for {
+            self.updates.tried = Some(now);
+        }
+        self.updates.phase = Phase::Failed { failed, asked_for };
+    }
+
+    /// Saves what the checks found, on the backend.
+    fn save_updates(&mut self) {
+        self.backend.send(Command::Save {
+            path: self.dirs.updates_file(),
+            file: StateFile::Updates(self.updates.seen.clone()),
+        });
+    }
+
+    /// What the window says once, outside the app's state: the status
+    /// line's line of a version available (terminal look), and a page to
+    /// open. Taken by the frame that says it.
+    pub(crate) fn say_updates(&mut self, ctx: &egui::Context) {
+        if let Some(version) = self.update_announcement.take()
+            && self.look.terminal
+        {
+            let said =
+                crate::i18n::gettext(self.locale, ANNOUNCEMENT).replace("{version}", &version);
+            crate::ui::toast::say_for(ctx, &said, ANNOUNCED_FOR);
+        }
+        if let Some(url) = self.url_to_open.take() {
+            ctx.open_url(egui::OpenUrl::new_tab(url));
+        }
+    }
+}
+
+/// A release's notes as the pager shows them: its version and page over
+/// the points the card shows.
+fn notes_text(offer: &crate::update::Offer) -> String {
+    let mut text = format!("# Tabletist {}\n\n{}\n\n", offer.version, offer.url);
+    if offer.body.trim().is_empty() {
+        for point in &offer.notes {
+            text.push_str(&format!("- {point}\n"));
+        }
+    } else {
+        text.push_str(offer.body.trim());
+        text.push('\n');
+    }
+    text
+}
+
+#[cfg(test)]
+mod tests {
+    use super::*;
+    use crate::backend::Event;
+    use crate::settings::UpdateCheck;
+    use crate::testing::Harness;
+    use crate::update::check::Failed;
+
+    const DAY: u64 = 24 * 60 * 60;
+    /// 2026-10-27 09:12 UTC.
+    const NOW: i64 = 1_793_092_320;
+
+    fn harness(check: UpdateCheck, last_checked: Option<u64>) -> Harness {
+        let mut harness = Harness::new();
+        harness.app.stopped_clock = Some(jiff::Timestamp::from_second(NOW).unwrap());
+        harness.app.settings.update_check = check;
+        harness.app.updates.seen.last_checked = last_checked;
+        harness.app.updates_scheduled = true;
+        harness.app.updates.installed = Some(crate::update::Installed::Itself(
+            crate::update::release::Kind::Portable,
+        ));
+        harness.app.backend.sent.clear();
+        harness
+    }
+
+    fn checks(harness: &Harness) -> Vec<u64> {
+        harness
+            .app
+            .backend
+            .sent
+            .iter()
+            .filter_map(|sent| match sent {
+                Command::Updates(UpdatesCommand::Check { asked, .. }) => Some(*asked),
+                _ => None,
+            })
+            .collect()
+    }
+
+    fn now() -> u64 {
+        u64::try_from(NOW).unwrap()
+    }
+
+    fn latest(tag: &str) -> release::Latest {
+        serde_json::from_str(&crate::update::check::tests::RELEASE.replace("v0.4.0", tag)).unwrap()
+    }
+
+    fn answer(harness: &mut Harness, result: Result<release::Latest, Failed>) {
+        let asked = *checks(harness).last().expect("a check");
+        harness
+            .app
+            .apply(Action::Backend(Event::Updates(UpdatesEvent::Checked {
+                asked,
+                result,
+            })));
+    }
+
+    #[test]
+    fn a_daily_check_runs_on_launch_only_when_it_is_due() {
+        let mut due = harness(UpdateCheck::Daily, Some(now() - 2 * DAY));
+        due.app.tick_updates();
+        assert_eq!(checks(&due).len(), 1);
+        // Not twice while it runs.
+        due.app.tick_updates();
+        assert_eq!(checks(&due).len(), 1);
+
+        let mut early = harness(UpdateCheck::Daily, Some(now() - 3_600));
+        let wait = early.app.tick_updates();
+        assert!(checks(&early).is_empty());
+        assert_eq!(wait, Some(Duration::from_secs(DAY - 3_600)));
+    }
+
+    #[test]
+    fn a_weekly_check_waits_a_week() {
+        let mut harness = harness(UpdateCheck::Weekly, Some(now() - 2 * DAY));
+        harness.app.tick_updates();
+        assert!(checks(&harness).is_empty());
+        harness.app.updates.seen.last_checked = Some(now() - 8 * DAY);
+        harness.app.tick_updates();
+        assert_eq!(checks(&harness).len(), 1);
+    }
+
+    #[test]
+    fn only_when_i_ask_checks_only_when_asked() {
+        let mut harness = harness(UpdateCheck::Never, None);
+        assert_eq!(harness.app.tick_updates(), None);
+        assert!(checks(&harness).is_empty());
+        harness.app.apply(Action::CheckForUpdates);
+        assert_eq!(checks(&harness).len(), 1);
+    }
+
+    /// A demo, and the tests, ask GitHub nothing until told to.
+    #[test]
+    fn nothing_is_checked_before_the_window_watches_for_updates() {
+        let mut harness = harness(UpdateCheck::Daily, None);
+        harness.app.updates_scheduled = false;
+        harness.app.tick_updates();
+        assert!(checks(&harness).is_empty());
+    }
+
+    #[test]
+    fn a_scheduled_check_waits_for_a_running_query() {
+        let mut harness = harness(UpdateCheck::Daily, None);
+        let tab = harness.connect_fake();
+        let sql = harness.add_sql_tab(tab);
+        let workspace = harness.app.workspace_mut(tab).unwrap();
+        let editor = workspace
+            .tabs
+            .iter_mut()
+            .filter_map(|open| open.as_sql_mut())
+            .find(|open| open.id == sql)
+            .unwrap();
+        editor.text = "SELECT * FROM book_covers".into();
+        harness.app.apply(Action::RunSql {
+            tab,
+            sql_tab: sql,
+            all: true,
+        });
+        harness
+            .app
+            .backend
+            .sent
+            .retain(|sent| matches!(sent, Command::RunSql { .. }));
+        assert!(harness.app.query_runs());
+        assert_eq!(harness.app.tick_updates(), Some(Duration::from_secs(1)));
+        assert!(checks(&harness).is_empty());
+        // The query ends: the check runs right after.
+        let outcome = crate::testing::script_outcome(vec![crate::testing::rows_outcome(3)]);
+        harness.answer_sql(Ok(outcome), None);
+        assert!(!harness.app.query_runs());
+        harness.app.tick_updates();
+        assert_eq!(checks(&harness).len(), 1);
+    }
+
+    #[test]
+    fn a_check_that_finds_a_newer_release_keeps_it_and_when_it_ran() {
+        let mut harness = harness(UpdateCheck::Daily, None);
+        harness.app.tick_updates();
+        answer(&mut harness, Ok(latest("v99.0.0")));
+        let updates = &harness.app.updates;
+        assert_eq!(updates.phase, Phase::Idle);
+        assert_eq!(updates.seen.last_checked, Some(now()));
+        assert_eq!(updates.seen.latest_seen.as_deref(), Some("99.0.0"));
+        let offer = updates.offer(None).expect("offered");
+        assert_eq!(offer.version, "99.0.0");
+        assert_eq!(offer.notes.len(), 4);
+        // Saved in the state folder.
+        let saved = harness.app.backend.sent.iter().any(|sent| {
+            matches!(sent, Command::Save { path, file: StateFile::Updates(seen) }
+                if *path == harness.app.dirs.updates_file() && seen.last_checked == Some(now()))
+        });
+        assert!(saved);
+    }
+
+    #[test]
+    fn a_timeout_or_no_connection_fails_the_check_and_is_no_check() {
+        for failed in [
+            Failed::TimedOut(Duration::from_secs(10)),
+            Failed::Unreachable,
+        ] {
+            let before = now() - 2 * DAY;
+            let mut harness = harness(UpdateCheck::Daily, Some(before));
+            harness.app.tick_updates();
+            answer(&mut harness, Err(failed.clone()));
+            assert_eq!(
+                harness.app.updates.phase,
+                Phase::Failed {
+                    failed,
+                    asked_for: false
+                }
+            );
+            assert_eq!(harness.app.updates.seen.last_checked, Some(before));
+            // Tried again at the next interval, not at once.
+            assert_eq!(harness.app.tick_updates(), Some(Duration::from_secs(DAY)));
+            assert_eq!(checks(&harness).len(), 1);
+        }
+    }
+
+    #[test]
+    fn a_cancelled_check_drops_what_it_brings() {
+        let mut harness = harness(UpdateCheck::Never, None);
+        harness.app.apply(Action::CheckForUpdates);
+        harness.app.apply(Action::CancelUpdateCheck);
+        answer(&mut harness, Ok(latest("v99.0.0")));
+        assert_eq!(harness.app.updates.phase, Phase::Idle);
+        assert_eq!(harness.app.updates.seen.last_checked, None);
+        assert!(harness.app.updates.offer(None).is_none());
+    }
+
+    #[test]
+    fn skip_this_version_hides_it_until_a_check_is_asked_for() {
+        let mut harness = harness(UpdateCheck::Daily, None);
+        harness.app.tick_updates();
+        answer(&mut harness, Ok(latest("v99.0.0")));
+        harness.app.apply(Action::SkipUpdate);
+        assert_eq!(harness.app.settings.update_skip.as_deref(), Some("99.0.0"));
+        assert!(
+            harness
+                .app
+                .settings_file
+                .text
+                .contains("[updates]\nskip = \"99.0.0\"")
+        );
+        let skip = harness.app.settings.update_skip.clone();
+        assert!(harness.app.updates.offer(skip.as_deref()).is_none());
+        // A scheduled check finds it again, and does not offer it.
+        harness.app.updates.seen.last_checked = Some(now() - 2 * DAY);
+        harness.app.tick_updates();
+        answer(&mut harness, Ok(latest("v99.0.0")));
+        assert!(harness.app.updates.offer(skip.as_deref()).is_none());
+        // A newer one is offered.
+        harness.app.updates.seen.last_checked = Some(now() - 2 * DAY);
+        harness.app.tick_updates();
+        answer(&mut harness, Ok(latest("v99.0.1")));
+        assert!(harness.app.updates.offer(skip.as_deref()).is_some());
+        // A check asked for offers the skipped one.
+        harness.app.apply(Action::CheckForUpdates);
+        answer(&mut harness, Ok(latest("v99.0.0")));
+        assert_eq!(
+            harness
+                .app
+                .updates
+                .offer(skip.as_deref())
+                .map(|offer| offer.version.as_str()),
+            Some("99.0.0")
+        );
+    }
+
+    #[test]
+    fn the_status_line_tells_of_a_version_at_most_once_a_day() {
+        let mut harness = harness(UpdateCheck::Daily, None);
+        harness.set_look(crate::theme::Look::omarchy());
+        harness.app.tick_updates();
+        answer(&mut harness, Ok(latest("v99.0.0")));
+        harness.app.tick_updates();
+        assert_eq!(
+            harness.app.update_announcement.take().as_deref(),
+            Some("99.0.0")
+        );
+        assert_eq!(harness.app.updates.seen.announced, Some(now()));
+        // A day has not passed.
+        harness.app.updates.seen.last_checked = Some(now() - 2 * DAY);
+        harness.app.tick_updates();
+        answer(&mut harness, Ok(latest("v99.0.0")));
+        harness.app.tick_updates();
+        assert_eq!(harness.app.update_announcement, None);
+    }
+
+    /// A version found before this launch is told of too, and a check
+    /// the person asked for counts as any other.
+    #[test]
+    fn the_status_line_tells_of_a_version_kept_from_before() {
+        let mut harness = harness(UpdateCheck::Weekly, Some(now() - DAY));
+        harness.set_look(crate::theme::Look::omarchy());
+        harness.app.updates.seen.offer = Some(release::offer(
+            &latest("v99.0.0"),
+            release::Os::current(),
+            None,
+        ));
+        harness.app.updates.seen.announced = Some(now() - 2 * DAY);
+        harness.app.tick_updates();
+        assert!(checks(&harness).is_empty());
+        assert_eq!(harness.app.update_announcement.as_deref(), Some("99.0.0"));
+    }
+
+    /// The macOS look has no status line: nothing is told, and nothing
+    /// noted as told.
+    #[test]
+    fn the_macos_look_tells_nothing() {
+        let mut harness = harness(UpdateCheck::Daily, None);
+        harness.set_look(crate::theme::Look::macos());
+        harness.app.tick_updates();
+        answer(&mut harness, Ok(latest("v99.0.0")));
+        harness.app.tick_updates();
+        assert_eq!(harness.app.update_announcement, None);
+        assert_eq!(harness.app.updates.seen.announced, None);
+    }
+
+    /// The check waits for the detection, so the offer keeps the asset of
+    /// this copy's kind.
+    #[test]
+    fn the_scheduled_check_waits_to_know_how_this_copy_was_installed() {
+        let mut harness = harness(UpdateCheck::Daily, None);
+        harness.app.updates.installed = None;
+        harness.app.tick_updates();
+        assert!(checks(&harness).is_empty());
+        harness
+            .app
+            .apply(Action::Backend(Event::Updates(UpdatesEvent::Detected(
+                crate::update::Installed::Itself(release::Kind::Portable),
+            ))));
+        harness.app.tick_updates();
+        assert_eq!(checks(&harness).len(), 1);
+    }
+
+    #[test]
+    fn the_notes_go_to_the_pager_whole() {
+        let mut harness = harness(UpdateCheck::Never, None);
+        harness.app.apply(Action::CheckForUpdates);
+        answer(&mut harness, Ok(latest("v99.0.0")));
+        harness.app.apply(Action::ShowReleaseNotes);
+        let shown = harness.app.backend.sent.iter().find_map(|sent| match sent {
+            Command::Updates(UpdatesCommand::ShowNotes { path, text, url }) => {
+                Some((path.clone(), text.clone(), url.clone()))
+            }
+            _ => None,
+        });
+        let (path, text, url) = shown.expect("shown");
+        assert_eq!(path, harness.app.dirs.release_notes_file("99.0.0"));
+        assert!(text.starts_with("# Tabletist 99.0.0\n"), "{text}");
+        // All five points, not the card's four.
+        assert!(text.contains("- Settings › Updates"), "{text}");
+        assert!(url.ends_with("/v99.0.0"), "{url}");
+        // No terminal: the page opens.
+        harness.app.apply(Action::Backend(Event::Updates(
+            UpdatesEvent::NotesUnshown { url: url.clone() },
+        )));
+        assert_eq!(harness.app.url_to_open.as_deref(), Some(url.as_str()));
+    }
+
+    /// Cancel holds: the schedule's next check waits an interval.
+    #[test]
+    fn a_cancelled_scheduled_check_waits_an_interval() {
+        let mut harness = harness(UpdateCheck::Daily, None);
+        harness.app.tick_updates();
+        assert_eq!(checks(&harness).len(), 1);
+        harness.app.apply(Action::CancelUpdateCheck);
+        assert!(!harness.app.updates.checking());
+        assert_eq!(harness.app.tick_updates(), Some(Duration::from_secs(DAY)));
+        assert_eq!(checks(&harness).len(), 1);
+    }
+
+    /// Check now while a scheduled check runs makes it one asked for: it
+    /// offers the version skipped.
+    #[test]
+    fn check_now_during_a_scheduled_check_counts_as_asked() {
+        let mut harness = harness(UpdateCheck::Daily, None);
+        harness.app.settings.update_skip = Some("99.0.0".into());
+        harness.app.tick_updates();
+        harness.app.apply(Action::CheckForUpdates);
+        assert_eq!(checks(&harness).len(), 1);
+        answer(&mut harness, Ok(latest("v99.0.0")));
+        assert!(harness.app.updates.offer(Some("99.0.0")).is_some());
+    }
+}
```

`src/backend.rs`:

```diff
diff --git a/src/backend.rs b/src/backend.rs
index 10e3c5d2..ce9a8736 100644
--- a/src/backend.rs
+++ b/src/backend.rs
@@ -20,11 +20,13 @@ use crate::secrets::{Keyring, SecretString};
 
 mod agents;
 mod history;
+mod updates;
 
 pub use agents::{
     Agentable, AgentsCommand, AgentsEvent, AppGateway, LOG_PAGE, agent_names, agentable,
 };
 pub use history::{HistoryCommand, HistoryEvent};
+pub use updates::{UpdatesCommand, UpdatesEvent};
 
 /// Wakes the UI from any thread. The default does nothing (tests).
 #[derive(Clone)]
@@ -241,6 +243,8 @@ pub enum Command {
     Agents(AgentsCommand),
     /// The query history's file.
     History(HistoryCommand),
+    /// The check for updates.
+    Updates(UpdatesCommand),
 }
 
 /// The contents of a state file the backend saves.
@@ -264,6 +268,8 @@ pub enum StateFile {
     Session(crate::session::Session),
     /// What was opened lately on each connection.
     Recents(crate::recents::Recents),
+    /// What the checks for updates found.
+    Updates(crate::update::Seen),
 }
 
 impl StateFile {
@@ -281,6 +287,7 @@ impl StateFile {
             Self::TableLists(lists) => crate::util::save_json(path, lists),
             Self::Session(session) => session.save(path),
             Self::Recents(recents) => recents.save(path),
+            Self::Updates(seen) => crate::util::save_json(path, seen),
         }
     }
 }
@@ -441,6 +448,8 @@ pub enum Event {
     Agents(AgentsEvent),
     /// What came of a [`HistoryCommand`].
     History(HistoryEvent),
+    /// What came of an [`UpdatesCommand`].
+    Updates(UpdatesEvent),
 }
 
 /// What the settings file was handed to.
@@ -2103,6 +2112,7 @@ impl Worker {
             }
             Command::Agents(command) => self.agents.handle(command, &self.outbox),
             Command::History(command) => self.history.handle(command, &self.outbox),
+            Command::Updates(command) => updates::handle(command, &self.outbox),
             query => {
                 let session = session_of(&query);
                 match self.sessions.get(&session) {
@@ -2158,7 +2168,8 @@ fn session_of(command: &Command) -> SessionId {
         | Command::ListFonts
         | Command::Flush { .. }
         | Command::Agents(_)
-        | Command::History(_) => SessionId(0),
+        | Command::History(_)
+        | Command::Updates(_) => SessionId(0),
     }
 }
 
@@ -2194,7 +2205,8 @@ fn request_of(command: &Command) -> Option<RequestId> {
         | Command::ListFonts
         | Command::Flush { .. }
         | Command::Agents(_)
-        | Command::History(_) => None,
+        | Command::History(_)
+        | Command::Updates(_) => None,
     }
 }
 
@@ -2298,7 +2310,8 @@ fn fail(outbox: &Outbox, command: Command, error: Error) {
         | Command::ListFonts
         | Command::Flush { .. }
         | Command::Agents(_)
-        | Command::History(_) => return,
+        | Command::History(_)
+        | Command::Updates(_) => return,
     };
     outbox.emit(event);
 }
@@ -2612,7 +2625,8 @@ async fn run_session(
             | Command::ListFonts
             | Command::Flush { .. }
             | Command::Agents(_)
-            | Command::History(_) => None,
+            | Command::History(_)
+            | Command::Updates(_) => None,
         };
         end.command = None;
         {
```

`src/backend/updates.rs`:

```diff
diff --git a/src/backend/updates.rs b/src/backend/updates.rs
new file mode 100644
index 00000000..271ccefe
--- /dev/null
+++ b/src/backend/updates.rs
@@ -0,0 +1,106 @@
+//! The check for updates and how this copy was installed, on the
+//! backend's blocking pool: each is a request to GitHub, or a look at the
+//! disk and the package manager, which a frame never waits for.
+
+use std::path::PathBuf;
+
+use crate::update::check::Failed;
+use crate::update::release::Latest;
+
+/// What the UI asks of the updater.
+#[derive(Debug)]
+pub enum UpdatesCommand {
+    /// Asks `url` for the latest release, waiting `timeout` at most.
+    /// Answered with [`UpdatesEvent::Checked`].
+    Check {
+        asked: u64,
+        url: String,
+        timeout: std::time::Duration,
+    },
+    /// Works out how this copy was installed. Answered with
+    /// [`UpdatesEvent::Detected`].
+    Detect,
+    /// Shows `text` (a release's notes) in `$PAGER` in a terminal, written
+    /// to `path` first. Answered with [`UpdatesEvent::NotesUnshown`] where
+    /// no terminal could be started.
+    ShowNotes {
+        path: PathBuf,
+        text: String,
+        url: String,
+    },
+}
+
+/// What came of an [`UpdatesCommand`].
+#[derive(Debug)]
+pub enum UpdatesEvent {
+    Checked {
+        asked: u64,
+        result: Result<Latest, Failed>,
+    },
+    Detected(crate::update::Installed),
+    /// The notes could not be shown in a terminal: their page is opened
+    /// instead.
+    NotesUnshown {
+        url: String,
+    },
+}
+
+pub fn handle(command: UpdatesCommand, outbox: &super::Outbox) {
+    let outbox = outbox.clone();
+    match command {
+        UpdatesCommand::Check {
+            asked,
+            url,
+            timeout,
+        } => {
+            tokio::task::spawn_blocking(move || {
+                let agent = crate::update::check::user_agent(
+                    crate::update::CURRENT,
+                    crate::update::check::macos_version().as_deref(),
+                );
+                let result = crate::update::check::latest(&url, &agent, timeout);
+                if let Err(failed) = &result {
+                    log::info!("the check for updates failed: {failed:?}");
+                }
+                outbox.emit(super::Event::Updates(UpdatesEvent::Checked {
+                    asked,
+                    result,
+                }));
+            });
+        }
+        UpdatesCommand::Detect => {
+            tokio::task::spawn_blocking(move || {
+                let installed = crate::update::Installed::detect();
+                outbox.emit(super::Event::Updates(UpdatesEvent::Detected(installed)));
+            });
+        }
+        UpdatesCommand::ShowNotes { path, text, url } => {
+            tokio::task::spawn_blocking(move || {
+                let shown = crate::util::write_atomic(&path, text.as_bytes())
+                    .and_then(|()| start_pager(&path));
+                if let Err(error) = shown {
+                    log::info!("could not show the release notes in a terminal: {error}");
+                    outbox.emit(super::Event::Updates(UpdatesEvent::NotesUnshown { url }));
+                }
+            });
+        }
+    }
+}
+
+/// Starts `$PAGER` (else `less`) on `path` in a terminal of the desktop's:
+/// `xdg-terminal-exec`, which Omarchy sets up. Fails where there is none.
+fn start_pager(path: &std::path::Path) -> std::io::Result<()> {
+    let pager = std::env::var("PAGER")
+        .ok()
+        .filter(|pager| !pager.trim().is_empty())
+        .unwrap_or_else(|| "less".to_owned());
+    let mut command = std::process::Command::new("xdg-terminal-exec");
+    // The pager as the shell would run it: `$PAGER` may hold its flags.
+    command
+        .arg("sh")
+        .arg("-c")
+        .arg(format!("{pager} \"$1\""))
+        .arg("sh")
+        .arg(path);
+    super::start("xdg-terminal-exec".to_owned(), command)
+}
```

`src/entrypoint.rs`:

```diff
diff --git a/src/entrypoint.rs b/src/entrypoint.rs
index a26a7dee..b32eb6a0 100644
--- a/src/entrypoint.rs
+++ b/src/entrypoint.rs
@@ -157,6 +157,9 @@ pub fn main(cli: Cli, launch: fastframe_update::Launch) -> anyhow::Result<()> {
             }
             if demo {
                 demo_setup(&mut app);
+            } else {
+                // Not for a demo either: it checks only when asked.
+                app.watch_updates();
             }
             // The menu bar is up by now: winit builds its own before the
             // window, and this one takes its place.
```

`src/model.rs`:

```diff
diff --git a/src/model.rs b/src/model.rs
index add01c97..73550cb0 100644
--- a/src/model.rs
+++ b/src/model.rs
@@ -990,6 +990,17 @@ pub enum Action {
     SettingShown,
     /// Show how this agent is connected, in Settings › Agents.
     ChooseAgentSetup(crate::endpoint::Agent),
+    /// Ask GitHub for a newer release now: Check now, Try again, the
+    /// menu's Check for Updates…, `c`, `:update`. A version the settings
+    /// skip is offered by a check asked for.
+    CheckForUpdates,
+    /// Drop the check under way: what it brings is not read.
+    CancelUpdateCheck,
+    /// Stop offering the version offered: `updates.skip`.
+    SkipUpdate,
+    /// Show the notes of the version offered in `$PAGER`, in a terminal;
+    /// where there is none to start, open its page.
+    ShowReleaseNotes,
     /// Close Settings and show the Agents tab on this view: the window's in
     /// front, or with Connections in front the last window's.
     ShowAgentsFromSettings(AgentsView),
```

`src/paths.rs`:

```diff
diff --git a/src/paths.rs b/src/paths.rs
index 826dc0a0..c3322331 100644
--- a/src/paths.rs
+++ b/src/paths.rs
@@ -112,6 +112,17 @@ impl AppDirs {
         self.state.join("labels.json")
     }
 
+    /// What the checks for updates found: when the last one ran, and the
+    /// newest release with its notes. Of this computer, and no setting.
+    pub fn updates_file(&self) -> PathBuf {
+        self.state.join("updates.json")
+    }
+
+    /// A release's notes, written for the pager to show.
+    pub fn release_notes_file(&self, version: &str) -> PathBuf {
+        self.state.join(format!("release-notes-{version}.md"))
+    }
+
     /// How each connection lists its tables where its user chose: tree or
     /// flat, by its id. Of this computer, and no setting: the setting is
     /// the default for a connection with no choice.
```

`src/promo/live.rs`:

```diff
diff --git a/src/promo/live.rs b/src/promo/live.rs
index e6392555..ef718394 100644
--- a/src/promo/live.rs
+++ b/src/promo/live.rs
@@ -223,6 +223,7 @@ impl Live {
                 | Command::ListFonts
                 | Command::Flush { .. }
                 | Command::Agents(_)
+                | Command::Updates(_)
                 | Command::History(
                     HistoryCommand::Clear { .. }
                     | HistoryCommand::Trim { .. }
```

`src/settings.rs`:

```diff
diff --git a/src/settings.rs b/src/settings.rs
index 382d4936..bcda3fd0 100644
--- a/src/settings.rs
+++ b/src/settings.rs
@@ -444,6 +444,44 @@ impl From<Option<String>> for Theme {
     }
 }
 
+/// How often Tabletist asks GitHub for a newer release.
+#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
+pub enum UpdateCheck {
+    #[default]
+    Daily,
+    Weekly,
+    /// Only when the person asks: Check now, the menu's Check for
+    /// Updates…, `c` or `:update`.
+    Never,
+}
+
+impl UpdateCheck {
+    /// Every choice, as the menu lists them.
+    pub const ALL: [Self; 3] = [Self::Daily, Self::Weekly, Self::Never];
+
+    /// As the file says it.
+    pub fn name(self) -> &'static str {
+        match self {
+            Self::Daily => "daily",
+            Self::Weekly => "weekly",
+            Self::Never => "never",
+        }
+    }
+
+    pub fn from_name(name: &str) -> Option<Self> {
+        Self::ALL.into_iter().find(|choice| choice.name() == name)
+    }
+}
+
+/// Whether `text` is a release's version: three numbers, `0.4.0`.
+fn is_version(text: &str) -> bool {
+    let parts: Vec<&str> = text.split('.').collect();
+    parts.len() == 3
+        && parts
+            .iter()
+            .all(|part| !part.is_empty() && part.bytes().all(|byte| byte.is_ascii_digit()))
+}
+
 /// How long a connection may go unused before its session is closed.
 #[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize)]
 pub enum Idle {
@@ -696,6 +734,13 @@ pub struct Settings {
     pub table_list: TableList,
     /// How the connection bar shows a connection's environment.
     pub env_bar: EnvBar,
+    /// How often a check for a newer release runs on its own.
+    #[serde(skip)]
+    pub update_check: UpdateCheck,
+    /// A version the person said to skip: no check offers it, but one they
+    /// ask for. A later version is offered.
+    #[serde(skip)]
+    pub update_skip: Option<String>,
     /// The person's changes to the keymap: `[keys.<scope>]` and
     /// `[keys.unbind]`. As the file says them: whether the keymap can take
     /// each is the look's to say ([`crate::keymap::Keymap::with`]).
@@ -751,13 +796,15 @@ pub enum Key {
     ColumnTypes,
     TableList,
     EnvBar,
+    UpdatesCheck,
+    UpdatesSkip,
 }
 
 impl Key {
     /// In the order the file has them: a table's keys together.
     /// An old key comes before the one that took its place: the new one
     /// is read over it.
-    pub const ALL: [Key; 41] = [
+    pub const ALL: [Key; 43] = [
         Self::OnLaunch,
         Self::RestoreTabs,
         Self::PageSize,
@@ -799,6 +846,8 @@ impl Key {
         Self::ColumnTypes,
         Self::TableList,
         Self::EnvBar,
+        Self::UpdatesCheck,
+        Self::UpdatesSkip,
     ];
 
     /// The key's table and its name there.
@@ -845,6 +894,8 @@ impl Key {
             Self::ColumnTypes => ("appearance.grid", "column_types"),
             Self::TableList => ("appearance.grid", "table_list"),
             Self::EnvBar => ("appearance.env", "bar"),
+            Self::UpdatesCheck => ("updates", "check"),
+            Self::UpdatesSkip => ("updates", "skip"),
         }
     }
 
@@ -880,6 +931,8 @@ impl Key {
             Self::RowHeight => Some("compact | regular | roomy"),
             Self::TableList => Some("tree | flat"),
             Self::EnvBar => Some("tint | label"),
+            Self::UpdatesCheck => Some("daily | weekly | never"),
+            Self::UpdatesSkip => Some("a version to stop offering"),
             Self::InterfaceSize => Some("small | default | large"),
             // The fonts of the look the platform draws, as the theme's.
             // Windows draws its data in Inter, whatever the key says.
@@ -2419,6 +2472,8 @@ impl Settings {
             Key::ColumnTypes => self.column_types.to_string(),
             Key::TableList => toml_string(self.table_list.name()),
             Key::EnvBar => toml_string(self.env_bar.name()),
+            Key::UpdatesCheck => toml_string(self.update_check.name()),
+            Key::UpdatesSkip => toml_string(self.update_skip.as_deref()?),
         })
     }
 
@@ -2769,6 +2824,16 @@ impl Settings {
             Key::EnvBar => text
                 .and_then(EnvBar::from_name)
                 .map(|bar| self.env_bar = bar),
+            Key::UpdatesCheck => text
+                .and_then(UpdateCheck::from_name)
+                .map(|check| self.update_check = check),
+            // A version, with its `v` or without; an empty one skips none.
+            Key::UpdatesSkip => text
+                .map(|text| crate::update::release::version_of(text.trim()))
+                .filter(|version| version.is_empty() || is_version(version))
+                .map(|version| {
+                    self.update_skip = (!version.is_empty()).then(|| version.to_owned());
+                }),
         }
         .is_some()
     }
@@ -2939,6 +3004,8 @@ impl Settings {
             Key::ColumnTypes => self.column_types = other.column_types,
             Key::TableList => self.table_list = other.table_list,
             Key::EnvBar => self.env_bar = other.env_bar,
+            Key::UpdatesCheck => self.update_check = other.update_check,
+            Key::UpdatesSkip => self.update_skip = other.update_skip.clone(),
         }
     }
 
@@ -3023,6 +3090,8 @@ impl Default for Settings {
             column_types: true,
             table_list: TableList::Tree,
             env_bar: EnvBar::Tint,
+            update_check: UpdateCheck::Daily,
+            update_skip: None,
             keys: crate::keymap::Overrides::default(),
         }
     }
```

`src/ui/toast.rs`:

```diff
diff --git a/src/ui/toast.rs b/src/ui/toast.rs
index ab8528a5..44dd053c 100644
--- a/src/ui/toast.rs
+++ b/src/ui/toast.rs
@@ -15,11 +15,12 @@ pub const SECONDS: f64 = 4.0;
 /// How far above the window's bottom edge it floats.
 const LIFT: f32 = 16.0;
 
-/// What is being said, and since when, in egui's seconds.
+/// What is being said, since when in egui's seconds, and for how long.
 #[derive(Clone)]
 struct Said {
     text: String,
     when: f64,
+    seconds: f64,
 }
 
 fn id() -> Id {
@@ -29,9 +30,16 @@ fn id() -> Id {
 /// Says `text`, already in the user's language, for [`SECONDS`]. It takes
 /// the place of what was being said.
 pub fn say(ctx: &egui::Context, text: &str) {
+    say_for(ctx, text, SECONDS);
+}
+
+/// [`say`], for `seconds`: what is told of rather than what was done (an
+/// update available) stays longer.
+pub fn say_for(ctx: &egui::Context, text: &str, seconds: f64) {
     let said = Said {
         text: text.to_owned(),
         when: ctx.input(|input| input.time),
+        seconds,
     };
     ctx.data_mut(|data| data.insert_temp(id(), said));
     ctx.request_repaint();
@@ -41,7 +49,7 @@ pub fn say(ctx: &egui::Context, text: &str) {
 /// moment ends: it goes without waiting for the pointer to move.
 pub fn said(ctx: &egui::Context) -> Option<String> {
     let said: Said = ctx.data(|data| data.get_temp(id()))?;
-    let left = SECONDS - (ctx.input(|input| input.time) - said.when);
+    let left = said.seconds - (ctx.input(|input| input.time) - said.when);
     if left <= 0.0 {
         ctx.data_mut(|data| data.remove::<Said>(id()));
         return None;
```

`src/update.rs`:

```diff
diff --git a/src/update.rs b/src/update.rs
index 2b8578fa..fc776865 100644
--- a/src/update.rs
+++ b/src/update.rs
@@ -25,6 +25,8 @@ pub use installed::{Installed, Managed};
 pub use release::{Found, Offer};
 pub use seen::Seen;
 
+use crate::settings::UpdateCheck;
+
 /// The version that runs.
 pub const CURRENT: &str = env!("CARGO_PKG_VERSION");
 
@@ -107,6 +109,80 @@ impl Transport for UreqTransport {
     }
 }
 
+/// How long `check` waits between scheduled checks; None where checks
+/// are only asked for.
+pub fn interval(check: UpdateCheck) -> Option<u64> {
+    match check {
+        UpdateCheck::Daily => Some(24 * 60 * 60),
+        UpdateCheck::Weekly => Some(7 * 24 * 60 * 60),
+        UpdateCheck::Never => None,
+    }
+}
+
+/// How long from `now` until the next scheduled check, in seconds: 0
+/// when it is due. An interval from the last check that worked, or from
+/// the last that failed where that came later. None with `"never"`. A
+/// time after `now` (a clock that was ahead) counts as now.
+pub fn due(check: UpdateCheck, seen: &Seen, tried: Option<u64>, now: u64) -> Option<u64> {
+    let every = interval(check)?;
+    let since = seen.last_checked.max(tried).map(|since| since.min(now));
+    Some(since.map_or(0, |since| (since + every).saturating_sub(now)))
+}
+
+/// Where the check stands while the app runs.
+#[derive(Clone, Debug, Default, PartialEq, Eq)]
+pub enum Phase {
+    /// Nothing asked.
+    #[default]
+    Idle,
+    /// Asked, and not answered yet: answers to any other ask are dropped.
+    Checking {
+        asked: u64,
+        /// Whether the person asked for it, which offers a version they
+        /// said to skip.
+        asked_for: bool,
+    },
+    /// The last check found nothing, for this reason. One the person
+    /// asked for is shown over a version found before.
+    Failed { failed: Failed, asked_for: bool },
+}
+
+/// What Settings › Updates and the signals elsewhere know.
+#[derive(Clone, Debug, Default)]
+pub struct Updates {
+    /// The state file's.
+    pub seen: Seen,
+    pub phase: Phase,
+    /// How this copy was installed, once the backend has looked.
+    pub installed: Option<Installed>,
+    /// When the last scheduled check that failed ran: the next waits an
+    /// interval from it.
+    pub tried: Option<u64>,
+    /// A version found by a check the person asked for, though they had
+    /// said to skip it: offered until the next check.
+    pub asked_offer: Option<Offer>,
+}
+
+impl Updates {
+    /// The release to offer now, if any: newer than this version, and not
+    /// the one the settings skip unless a check that was asked for found
+    /// it.
+    pub fn offer(&self, skip: Option<&str>) -> Option<&Offer> {
+        self.asked_offer
+            .as_ref()
+            .or_else(|| {
+                self.seen
+                    .offered(CURRENT)
+                    .filter(|offer| skip != Some(offer.version.as_str()))
+            })
+            .filter(|offer| fastframe_update::is_newer(&offer.version, CURRENT))
+    }
+
+    pub fn checking(&self) -> bool {
+        matches!(self.phase, Phase::Checking { .. })
+    }
+}
+
 #[cfg(test)]
 mod tests {
     use super::*;
```

- [ ] **Step 4: Run the checks**

Run: `cargo fmt --all --check && cargo clippy --locked --workspace --all-targets -- -D warnings && cargo clippy --locked --workspace --all-targets --features shots -- -D warnings && RUSTDOCFLAGS='-D warnings' cargo doc --locked --workspace --no-deps && cargo test --locked --workspace --all-targets`
Expected: clean; the library's tests `test result: ok. 3574 passed; 0 failed; 17 ignored`. The `shots` build is the one that reads `src/promo/live.rs`.

- [ ] **Step 5: Commit**

```bash
git add AGENTS.md src/settings.rs src/update.rs src/backend.rs src/backend/updates.rs src/app.rs src/app/updates.rs src/model.rs src/paths.rs src/ui/toast.rs src/promo/live.rs src/entrypoint.rs
git commit -m "Check for updates on launch and at the interval, and keep what was found"
```

---

### Task 3: Settings › Updates in both looks

**Files:** `src/settings.rs`, `src/model.rs`, `src/app.rs`, `src/app/{commands,editing,menu}.rs`, `src/keymap.rs`, `src/menu.rs`, `src/theme.rs`, `assets/icons/download.svg`, `src/ui/settings/{mod,sheet,terminal}.rs`, `src/ui/settings/updates.rs`, `src/ui/settings/updates_terminal.rs`, `src/ui/update_settings_tests.rs`, `src/ui/mod.rs`, the docs.

`SettingsPane::Updates` is listed last in both looks, with one option row, `OptionId::UpdatesCheck` (a menu on the sheet, `‹ ›` in Omarchy). Its print is the time of the last check with a **Check now** link (`sheet::Print`, drawn after the row's text: it does not describe the control). `updates::Card::of` picks the card's state from `App::updates` and the installation; the sheet draws it as a region with an icon tile, a title announced as it changes, the notes as a list, the command box with Copy, and its buttons; `updates_terminal` draws the Omarchy block, heading and note with the settings rows' painter. The navigation's dot is drawn wherever `Updates::offer` has a version, whatever the card shows. A check the person asked for that failed shows over a version found before (`Phase::Failed { asked_for }`). `c`, `n` and `S` are bound to `When::UpdatesPane` and read before the window's keys there (`settings::asked`); `y` stays the settings window's copy, which on this pane copies the update's command. `ShowUpdates` (the menu, quick open, `:update`) shows the pane and checks.

- [ ] **Step 1: Write the failing tests**

`src/settings.rs`:

```diff
diff --git a/src/settings.rs b/src/settings.rs
index bcda3fd0..94af0f02 100644
--- a/src/settings.rs
+++ b/src/settings.rs
@@ -4701,6 +4701,7 @@ skip = \"0.4.0\"  # a version to stop offering
                 Key::McpEnabled,
                 Key::McpLogDays,
                 Key::McpNotify,
+                Key::UpdatesCheck,
             ]
         );
         // Each pane has its own, and every one is on a pane.
@@ -4709,6 +4710,7 @@ skip = \"0.4.0\"  # a version to stop offering
             &OptionId::APPEARANCE[..],
             &OptionId::EDITOR[..],
             &OptionId::AGENTS[..],
+            &OptionId::UPDATES[..],
         ]
         .concat();
         // The sections of the General and Appearance panes hold their
```

`src/ui/mod.rs`:

```diff
diff --git a/src/ui/mod.rs b/src/ui/mod.rs
index 6c7f03bf..af86bd56 100644
--- a/src/ui/mod.rs
+++ b/src/ui/mod.rs
@@ -117,6 +117,8 @@ mod theme_settings_tests;
 pub mod toast;
 #[cfg(test)]
 mod typing_settings_tests;
+#[cfg(test)]
+mod update_settings_tests;
 pub mod value_tags;
 #[cfg(test)]
 mod vim_tests;
@@ -6937,7 +6939,7 @@ value_tags   = true
     }
 
     /// A key that `keys::letters` reads as the character it types.
-    pub(super) fn type_key(harness: &mut Harness, key: Key, text: &str) {
+    pub(crate) fn type_key(harness: &mut Harness, key: Key, text: &str) {
         harness.settle();
         harness.frame(vec![
             crate::testing::key(key, Modifiers::NONE),
```

`src/ui/update_settings_tests.rs`:

```diff
diff --git a/src/ui/update_settings_tests.rs b/src/ui/update_settings_tests.rs
new file mode 100644
index 00000000..d7dbe80e
--- /dev/null
+++ b/src/ui/update_settings_tests.rs
@@ -0,0 +1,565 @@
+//! Settings › Updates, in both looks: the pane, the status card in each of
+//! its states, its signals elsewhere, how often a check runs, and the
+//! terminal look's keys and `:update`.
+
+use egui::accesskit::{Live, Role};
+use egui::{Key, Modifiers};
+
+use crate::backend::{Command, UpdatesCommand};
+use crate::model::{Action, Dialog, SettingsPane};
+use crate::settings::UpdateCheck;
+use crate::testing::{Harness, node};
+use crate::theme::Look;
+use crate::update::Phase;
+use crate::update::check::Failed;
+use crate::update::installed::{Installed, Managed};
+use crate::update::release::{self, Kind, Latest, Offer, Os};
+
+fn looks() -> [Look; 2] {
+    [Look::macos(), Look::omarchy()]
+}
+
+/// 2026-10-27 09:12 UTC.
+const NOW: i64 = 1_793_092_320;
+
+/// A release far newer than this version, as the fake server gives it.
+fn offer() -> Offer {
+    let json = crate::update::check::tests::RELEASE
+        .replace("v0.4.0", "v99.0.0")
+        .replace("0.4.0", "99.0.0");
+    let latest: Latest = serde_json::from_str(&json).unwrap();
+    let mut offer = release::offer(&latest, Os::current(), Some(Kind::Portable));
+    offer
+        .download
+        .get_or_insert(("tabletist-v99.0.0.tar.gz".into(), 31_200_000));
+    offer
+}
+
+fn harness(look: Look) -> Harness {
+    let mut harness = Harness::new();
+    harness.set_look(look);
+    harness.app.stopped_clock = Some(jiff::Timestamp::from_second(NOW).unwrap());
+    harness
+}
+
+/// Settings, on its Updates pane.
+fn on_the_pane(harness: &mut Harness) {
+    harness
+        .app
+        .apply(Action::ShowSettingsPane(SettingsPane::Updates));
+    harness.settle();
+}
+
+fn pane(harness: &Harness) -> Option<SettingsPane> {
+    match &harness.app.dialog {
+        Some(Dialog::Settings(dialog)) => Some(dialog.pane),
+        _ => None,
+    }
+}
+
+/// The words as `look` writes them.
+fn said(look: &Look, words: &str) -> String {
+    look.label(words)
+}
+
+/// A capital typed: its key with Shift, and the letter.
+fn capital(harness: &mut Harness, key: Key) {
+    harness.settle();
+    let letter = format!("{key:?}");
+    harness.frame(vec![
+        crate::testing::key(key, Modifiers::SHIFT),
+        egui::Event::Text(letter),
+    ]);
+    harness.frame(vec![crate::testing::release(key, Modifiers::SHIFT)]);
+    harness.settle();
+}
+
+fn checks(harness: &Harness) -> usize {
+    harness
+        .app
+        .backend
+        .sent
+        .iter()
+        .filter(|sent| matches!(sent, Command::Updates(UpdatesCommand::Check { .. })))
+        .count()
+}
+
+#[test]
+fn settings_lists_an_updates_pane_last() {
+    for look in looks() {
+        let mut harness = harness(look);
+        harness.app.apply(Action::ShowSettings);
+        harness.settle();
+        let listed = SettingsPane::listed();
+        assert_eq!(listed.last(), Some(&SettingsPane::Updates));
+        if look.terminal {
+            for _ in 1..listed.len() {
+                harness.press(Key::L, Modifiers::CTRL);
+            }
+        } else {
+            harness.click("Updates");
+        }
+        harness.settle();
+        assert_eq!(pane(&harness), Some(SettingsPane::Updates), "{}", look.name);
+        // The row's control, named by the option in either look.
+        assert!(harness.has("Check for updates"), "{}", look.name);
+    }
+}
+
+#[test]
+fn the_card_says_where_updates_stand_in_a_region_of_its_own() {
+    for look in looks() {
+        let mut harness = harness(look);
+        on_the_pane(&mut harness);
+        let tree = harness.settle();
+        assert!(node(&tree, &said(&look, "Update status"), Role::Region).is_some());
+        let current = crate::update::CURRENT;
+        assert!(
+            harness.has(&said(&look, &format!("Tabletist {current}"))),
+            "{}",
+            look.name
+        );
+
+        // Up to date, checked today.
+        harness.app.updates.seen.last_checked = Some(u64::try_from(NOW).unwrap() - 60);
+        let title = said(&look, &format!("Tabletist {current} is up to date"));
+        assert!(harness.has(&title), "{}", look.name);
+        let tree = harness.settle();
+        let id = node(&tree, &title, Role::Label).expect("the title");
+        let title_node = tree.nodes.iter().find(|(at, _)| *at == id).unwrap();
+        assert_eq!(title_node.1.live(), Some(Live::Polite), "{}", look.name);
+
+        // Checking, with what it asks.
+        harness.app.apply(Action::CheckForUpdates);
+        assert!(
+            harness.has(&said(&look, "Checking for updates…")),
+            "{}",
+            look.name
+        );
+        assert!(harness.has(crate::update::check::ASKED), "{}", look.name);
+        if !look.terminal {
+            harness.click("Cancel");
+            harness.settle();
+            assert!(!harness.app.updates.checking());
+        }
+
+        // Couldn't check: the error told at once.
+        harness.app.updates.phase = Phase::Failed {
+            failed: Failed::TimedOut(std::time::Duration::from_secs(10)),
+            asked_for: true,
+        };
+        let tree = harness.settle();
+        assert!(
+            harness.has(&said(&look, "Couldn't check for updates")),
+            "{}: {:?}",
+            look.name,
+            crate::testing::labels(&tree)
+        );
+        let error = said(
+            &look,
+            "Couldn't reach github.com: the connection timed out after 10 s.",
+        );
+        let tree = harness.settle();
+        let id = node(&tree, &error, Role::Label).expect("the error");
+        let error_node = tree.nodes.iter().find(|(at, _)| *at == id).unwrap();
+        assert_eq!(error_node.1.live(), Some(Live::Assertive), "{}", look.name);
+    }
+}
+
+#[test]
+fn an_update_available_shows_its_notes_and_a_dot_on_updates() {
+    for look in looks() {
+        let mut harness = harness(look);
+        harness.app.updates.installed = Some(Installed::Itself(Kind::Portable));
+        on_the_pane(&mut harness);
+        let dot = "Update available";
+        assert!(!harness.has(dot), "{}", look.name);
+        harness.app.updates.seen.offer = Some(offer());
+        // The terminal look's block says it as its board does.
+        let title = match look.terminal {
+            true => "99.0.0 available",
+            false => "Tabletist 99.0.0 is available",
+        };
+        assert!(harness.has(title), "{}", look.name);
+        assert!(harness.has(dot), "{}", look.name);
+        // Four of the five points.
+        if look.terminal {
+            assert!(harness.has("- date and time picker, ddl view and query history"));
+            assert!(!harness.has("- settings › updates"));
+        } else {
+            let tree = harness.settle();
+            let list = node(&tree, "What's new", Role::List).expect("the notes");
+            let items = tree
+                .nodes
+                .iter()
+                .filter(|(_, node)| node.role() == Role::ListItem)
+                .count();
+            assert_eq!(items, 4);
+            let _ = list;
+            assert!(harness.has("Date and time picker, DDL view and query history"));
+        }
+        // You have this version, the day it came out, and the download's
+        // size.
+        let offer = offer();
+        let size = release::megabytes(offer.download.unwrap().1);
+        let then = std::time::UNIX_EPOCH + std::time::Duration::from_secs(offer.published.unwrap());
+        let now =
+            std::time::UNIX_EPOCH + std::time::Duration::from_secs(u64::try_from(NOW).unwrap());
+        let day = match crate::clock::day_of(then, now) {
+            crate::clock::Day::Today => "today".to_owned(),
+            crate::clock::Day::Yesterday => "yesterday".to_owned(),
+            crate::clock::Day::On(day) => day,
+        };
+        let sub = format!(
+            "You have {} · released {day} · {size}",
+            crate::update::CURRENT
+        );
+        if look.terminal {
+            let sub = format!(
+                "{} · {} {}",
+                look.label(&sub),
+                std::env::consts::OS,
+                std::env::consts::ARCH
+            );
+            assert!(harness.has(&sub), "{}: {sub}", look.name);
+        } else {
+            assert!(harness.has(&sub), "{}: {sub}", look.name);
+        }
+    }
+}
+
+#[test]
+fn release_notes_on_github_opens_the_page_and_skip_stops_offering_it() {
+    let look = Look::macos();
+    let mut harness = harness(look);
+    harness.app.updates.installed = Some(Installed::Itself(Kind::Portable));
+    harness.app.updates.seen.offer = Some(offer());
+    on_the_pane(&mut harness);
+    harness.click("Release notes on GitHub");
+    harness.settle();
+    assert_eq!(
+        harness.opened.as_deref(),
+        Some("https://github.com/igor-alexandrov/tabletist/releases/tag/v99.0.0")
+    );
+    harness.click("Skip this version");
+    harness.settle();
+    assert_eq!(harness.app.settings.update_skip.as_deref(), Some("99.0.0"));
+    assert!(!harness.has("Tabletist 99.0.0 is available"));
+    assert!(!harness.has("Update available"));
+}
+
+#[test]
+fn a_release_with_no_build_here_offers_its_page_only() {
+    for look in looks() {
+        let mut harness = harness(look);
+        harness.app.updates.installed = Some(Installed::Itself(Kind::Portable));
+        let mut offer = offer();
+        offer.download = None;
+        harness.app.updates.seen.offer = Some(offer);
+        on_the_pane(&mut harness);
+        let said_here = said(&look, "99.0.0 has no build for this computer yet");
+        assert!(harness.has(&said_here), "{}", look.name);
+    }
+}
+
+#[test]
+fn a_copy_something_else_installed_says_what_updates_it_and_copies_the_command() {
+    for look in looks() {
+        let mut harness = harness(look);
+        harness.app.updates.installed = Some(Installed::Elsewhere(Managed {
+            said: "Installed with Homebrew, so Homebrew updates it",
+            command: Some("brew upgrade --cask tabletist"),
+        }));
+        harness.app.updates.seen.offer = Some(offer());
+        on_the_pane(&mut harness);
+        assert!(
+            harness.has(&said(&look, "99.0.0 is available")),
+            "{}",
+            look.name
+        );
+        let managed = said(&look, "Installed with Homebrew, so Homebrew updates it");
+        assert!(harness.has(&managed), "{}", look.name);
+        if look.terminal {
+            assert!(harness.has("brew upgrade --cask tabletist"));
+            crate::ui::tests::type_key(&mut harness, Key::Y, "y");
+        } else {
+            assert!(harness.has("brew upgrade --cask tabletist"));
+            // No download offered: something else installs it.
+            assert!(!harness.has("Skip this version"));
+            harness.click("Copy");
+        }
+        harness.settle();
+        assert_eq!(
+            harness.copied.as_deref(),
+            Some("brew upgrade --cask tabletist"),
+            "{}",
+            look.name
+        );
+    }
+}
+
+#[test]
+fn check_now_and_try_again_check() {
+    let mut harness = harness(Look::macos());
+    harness.app.updates.seen.last_checked = Some(u64::try_from(NOW).unwrap() - 3_600);
+    on_the_pane(&mut harness);
+    // In the computer's zone, as the clock writes an hour.
+    let then =
+        std::time::UNIX_EPOCH + std::time::Duration::from_secs(u64::try_from(NOW).unwrap() - 3_600);
+    let hour = crate::clock::hour(then);
+    assert!(harness.has(&format!("Last checked today at {hour}")));
+    // The card's button, and the row's link: either checks.
+    harness.click("Check now");
+    harness.settle();
+    assert_eq!(checks(&harness), 1);
+    harness.app.updates.phase = Phase::Failed {
+        failed: Failed::Unreachable,
+        asked_for: true,
+    };
+    harness.click("Try again");
+    harness.settle();
+    assert_eq!(checks(&harness), 2);
+}
+
+#[test]
+fn how_often_a_check_runs_is_set_and_written() {
+    for look in looks() {
+        let mut harness = harness(look);
+        on_the_pane(&mut harness);
+        if look.terminal {
+            crate::ui::tests::type_key(&mut harness, Key::L, "l");
+        } else {
+            harness.click("Check for updates");
+            harness.click("Check for updates Every week");
+        }
+        harness.settle();
+        assert_eq!(
+            harness.app.settings.update_check,
+            UpdateCheck::Weekly,
+            "{}",
+            look.name
+        );
+        let text = &harness.app.settings_file.text;
+        assert!(
+            text.contains("[updates]\ncheck = \"weekly\"  # daily | weekly | never"),
+            "{}: {text}",
+            look.name
+        );
+        // Only when I ask, the end: no further.
+        if look.terminal {
+            crate::ui::tests::type_key(&mut harness, Key::L, "l");
+            crate::ui::tests::type_key(&mut harness, Key::L, "l");
+            assert_eq!(harness.app.settings.update_check, UpdateCheck::Never);
+            // R puts back the default, which the file leaves out.
+            capital(&mut harness, Key::R);
+            assert_eq!(harness.app.settings.update_check, UpdateCheck::Daily);
+            assert!(!harness.app.settings_file.text.contains("[updates]"));
+        }
+    }
+}
+
+#[test]
+fn the_pane_says_what_the_check_sends() {
+    for look in looks() {
+        let mut harness = harness(look);
+        on_the_pane(&mut harness);
+        let words = if look.terminal {
+            "the check asks github for the latest release and sends this version, os and cpu, nothing else."
+        } else if cfg!(windows) {
+            "The check asks GitHub for the latest release. It sends this version, Windows and the processor, nothing else: no ID, no usage data, no database details."
+        } else {
+            "The check asks GitHub for the latest release. It sends this version, the macOS version and the chip, nothing else: no ID, no usage data, no database details."
+        };
+        assert!(harness.has(words), "{}", look.name);
+    }
+}
+
+#[test]
+fn the_terminal_keys_check_show_the_notes_and_skip() {
+    let look = Look::omarchy();
+    let mut harness = harness(look);
+    harness.app.updates.installed = Some(Installed::Itself(Kind::Portable));
+    on_the_pane(&mut harness);
+    crate::ui::tests::type_key(&mut harness, Key::C, "c");
+    assert_eq!(checks(&harness), 1);
+    harness.app.apply(Action::CancelUpdateCheck);
+    harness.app.updates.seen.offer = Some(offer());
+    // The block's keys, as the keymap writes them.
+    assert!(harness.has("n release notes  S skip 99.0.0"));
+    crate::ui::tests::type_key(&mut harness, Key::N, "n");
+    let shown = harness
+        .app
+        .backend
+        .sent
+        .iter()
+        .any(|sent| matches!(sent, Command::Updates(UpdatesCommand::ShowNotes { .. })));
+    assert!(shown);
+    capital(&mut harness, Key::S);
+    assert_eq!(harness.app.settings.update_skip.as_deref(), Some("99.0.0"));
+    // Off this pane, `n` is the history's, and `c` nothing.
+    harness.press(Key::H, Modifiers::CTRL);
+    harness.settle();
+    let before = checks(&harness);
+    crate::ui::tests::type_key(&mut harness, Key::C, "c");
+    assert_eq!(checks(&harness), before);
+}
+
+#[test]
+fn update_on_the_prompt_shows_the_pane_and_checks() {
+    let mut harness = harness(Look::omarchy());
+    let tab = harness.connect_fake();
+    harness.app.workspace_mut(tab).unwrap().command = Some("update".into());
+    harness.app.apply(Action::RunCommand(tab));
+    harness.settle();
+    assert_eq!(pane(&harness), Some(SettingsPane::Updates));
+    assert_eq!(checks(&harness), 1);
+}
+
+#[test]
+fn the_status_line_tells_of_a_version_for_eight_seconds() {
+    let mut harness = harness(Look::omarchy());
+    let _tab = harness.connect_fake();
+    harness.app.update_announcement = Some("99.0.0".into());
+    let ctx = harness.ctx.clone();
+    harness.app.say_updates(&ctx);
+    assert_eq!(
+        crate::ui::toast::said(&harness.ctx).as_deref(),
+        Some("99.0.0 available · :update")
+    );
+    // macOS and Windows say nothing of it but the dot.
+    let mut mac = Harness::new();
+    mac.set_look(Look::macos());
+    mac.app.update_announcement = Some("99.0.0".into());
+    let ctx = mac.ctx.clone();
+    mac.app.say_updates(&ctx);
+    assert_eq!(crate::ui::toast::said(&mac.ctx), None);
+}
+
+#[test]
+fn the_settings_file_sets_the_check_live_and_marks_a_bad_value() {
+    let mut harness = harness(Look::omarchy());
+    let text = "[updates]\ncheck = \"never\"\nskip = \"v0.9.0\"\n".to_owned();
+    let file = crate::backend::Event::SettingsFile { text, own: false };
+    harness.app.apply(Action::Backend(file));
+    assert_eq!(harness.app.settings.update_check, UpdateCheck::Never);
+    assert_eq!(harness.app.settings.update_skip.as_deref(), Some("0.9.0"));
+    assert!(harness.app.settings_file.invalid.is_empty());
+    let text = "[updates]\ncheck = \"hourly\"\nskip = \"soon\"\n".to_owned();
+    let file = crate::backend::Event::SettingsFile { text, own: false };
+    harness.app.apply(Action::Backend(file));
+    // Each bad line is marked, and leaves the value as it was.
+    assert_eq!(harness.app.settings_file.invalid, vec![2, 3]);
+    assert_eq!(harness.app.settings.update_check, UpdateCheck::Never);
+    assert_eq!(harness.app.settings.update_skip.as_deref(), Some("0.9.0"));
+}
+
+/// Two notes alike are two points of the list a screen reader hears.
+#[test]
+fn two_notes_alike_are_two_items() {
+    let mut harness = harness(Look::macos());
+    harness.app.updates.installed = Some(Installed::Itself(Kind::Portable));
+    let mut offer = offer();
+    offer.notes = vec![
+        "Fixes".into(),
+        "Fixes".into(),
+        "Faster grid".into(),
+        "Dark mode".into(),
+    ];
+    harness.app.updates.seen.offer = Some(offer);
+    on_the_pane(&mut harness);
+    let tree = harness.settle();
+    let items = tree
+        .nodes
+        .iter()
+        .filter(|(_, node)| node.role() == Role::ListItem)
+        .count();
+    assert_eq!(items, 4, "{:?}", crate::testing::labels(&tree));
+}
+
+/// While a check runs, the version found before is still available: the
+/// dot on Updates stays.
+#[test]
+fn the_dot_stays_while_a_check_runs() {
+    for look in looks() {
+        let mut harness = harness(look);
+        harness.app.updates.installed = Some(Installed::Itself(Kind::Portable));
+        harness.app.updates.seen.offer = Some(offer());
+        on_the_pane(&mut harness);
+        assert!(harness.has("Update available"), "{}", look.name);
+        harness.app.apply(Action::CheckForUpdates);
+        harness.settle();
+        assert!(harness.app.updates.checking());
+        assert!(
+            harness.has("Update available"),
+            "{}: the dot went while a check runs",
+            look.name
+        );
+    }
+}
+
+/// An error is told once, assertively, by its own line; the title
+/// politely.
+#[test]
+fn an_error_is_told_once() {
+    for look in looks() {
+        let mut harness = harness(look);
+        on_the_pane(&mut harness);
+        harness.app.updates.phase = Phase::Failed {
+            failed: Failed::Unreachable,
+            asked_for: true,
+        };
+        let tree = harness.settle();
+        let assertive: Vec<_> = tree
+            .nodes
+            .iter()
+            .filter(|(_, node)| node.live() == Some(Live::Assertive))
+            .map(|(_, node)| node.label().or(node.value()).map(str::to_owned))
+            .collect();
+        assert_eq!(assertive.len(), 1, "{}: {assertive:?}", look.name);
+    }
+}
+
+/// Cancel holds as the person sees it, with the frames' own ticks: the
+/// schedule's check does not start again.
+#[test]
+fn cancel_on_the_card_holds() {
+    let mut harness = harness(Look::macos());
+    harness.app.settings.update_check = UpdateCheck::Daily;
+    harness.app.updates_scheduled = true;
+    harness.app.updates.installed = Some(Installed::Itself(Kind::Portable));
+    on_the_pane(&mut harness);
+    harness.app.logic(&harness.ctx.clone());
+    harness.settle();
+    assert!(harness.has("Checking for updates…"));
+    harness.click("Cancel");
+    harness.app.logic(&harness.ctx.clone());
+    harness.settle();
+    assert!(!harness.has("Checking for updates…"));
+}
+
+/// A check the person asked for that fails is shown, though a version
+/// found before is kept, and the dot still tells of it. One of the
+/// schedule's that fails leaves the version shown.
+#[test]
+fn a_failed_check_asked_for_is_shown_over_a_version_found_before() {
+    for look in looks() {
+        let mut harness = harness(look);
+        harness.app.updates.installed = Some(Installed::Itself(Kind::Portable));
+        harness.app.updates.seen.offer = Some(offer());
+        on_the_pane(&mut harness);
+        harness.app.updates.phase = Phase::Failed {
+            failed: Failed::Unreachable,
+            asked_for: true,
+        };
+        harness.settle();
+        let error = said(&look, "Couldn't check for updates");
+        assert!(harness.has(&error), "{}", look.name);
+        assert!(harness.has("Update available"), "{}", look.name);
+        harness.app.updates.phase = Phase::Failed {
+            failed: Failed::Unreachable,
+            asked_for: false,
+        };
+        harness.settle();
+        assert!(!harness.has(&error), "{}", look.name);
+    }
+}
```

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test --locked --lib --no-run`
Expected: no build, 5 errors, such as:

```
error[E0599]: no variant, associated function, or constant named `Updates` found for enum `SettingsPane` in the current scope
error[E0599]: no variant, associated function, or constant named `UPDATES` found for enum `settings::OptionId` in the current scope
```

- [ ] **Step 3: Write the code**

`assets/icons/download.svg`:

```diff
diff --git a/assets/icons/download.svg b/assets/icons/download.svg
new file mode 100644
index 00000000..eaad42a6
--- /dev/null
+++ b/assets/icons/download.svg
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
+  <path d="M12 15V3" />
+  <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" />
+  <path d="m7 10 5 5 5-5" />
+</svg>
```

`docs/_guide/download.md`:

````diff
diff --git a/docs/_guide/download.md b/docs/_guide/download.md
index adb39a45..8b8eee77 100644
--- a/docs/_guide/download.md
+++ b/docs/_guide/download.md
@@ -88,8 +88,10 @@ the binary in `/usr/local/bin` instead, which needs root:
 sudo install -Dm755 tabletist /usr/local/bin/tabletist
 ```
 
-To update, install the new binary the same way. To remove Tabletist, delete
-the three files.
+To update, install the new binary the same way: **Settings › Updates**
+(`:update` in the Omarchy look) checks GitHub for a newer version every
+day and says when there is one. To remove Tabletist, delete the three
+files.
 
 The Linux binaries are built on Ubuntu 24.04 and need glibc 2.39 or newer
 (Ubuntu 24.04, Debian 13, Fedora 40 or later). On an older system,
````

`docs/_guide/macos.md`:

```diff
diff --git a/docs/_guide/macos.md b/docs/_guide/macos.md
index 8fe11926..578289f1 100644
--- a/docs/_guide/macos.md
+++ b/docs/_guide/macos.md
@@ -47,7 +47,9 @@ launch, are on the
   **Query**, **Window** and **Help** list the commands quick open runs,
   each with its key; **Tabletist → About Tabletist** shows the version, a
   link to the source code and the licenses of the fonts it bundles, and
-  **Tabletist → Settings…** opens Settings. **View → History** shows the
+  **Tabletist → Settings…** opens Settings. **Tabletist → Check for
+  Updates…** opens Settings › Updates and asks GitHub for a newer
+  version. **View → History** shows the
   window's [History tab]({% link _guide/sql-editor.md %}#history), and
   **View → Quick Open** opens quick open. A menu greys the commands that
   do not apply now, while it is open. Its keys do what they do where you
```

`docs/_guide/omarchy.md`:

```diff
diff --git a/docs/_guide/omarchy.md b/docs/_guide/omarchy.md
index 059fac17..b8d5d838 100644
--- a/docs/_guide/omarchy.md
+++ b/docs/_guide/omarchy.md
@@ -49,6 +49,7 @@ second Esc gives the keyboard back.
 | `:keys default` | Open every key in force in your editor, as `settings.toml` spells them, commented: a file to copy lines from, written again each time and never read |
 | `:keys cheatsheet` | Save the keys in force as a Markdown cheat sheet, where you say |
 | `:set calendar`, `:set nocalendar` | Show or hide the [calendar]({% link _guide/editing-data.md %}#the-calendar) under a date's editor, saved as its switch in Settings is |
+| `:update` | Show Settings › Updates, and check for a newer version there |
 | `?` | The list of keys |
 
 Ctrl+P opens quick open from any pane: a picker at the top of the
@@ -357,9 +358,12 @@ stopped a run, `t` runs it again without one. Times read "45s", "1m" and
 | `j` `k` | Move |
 | `h` `l` | Change the option |
 | Space | Toggle the option |
-| Ctrl+H, Ctrl+L | The pane before or after: General, Appearance, Editor, Keyboard, Agents |
+| Ctrl+H, Ctrl+L | The pane before or after: General, Appearance, Editor, Keyboard, Agents, Updates |
 | Enter | On the Agents pane: show the Agents tab, Connections or the log, from the status, access or log row. On the Editor pane's keep row: show the History tab |
-| `y` | On the Agents pane: copy the command that connects Claude Code |
+| `y` | On the Agents pane: copy the command that connects Claude Code. On the Updates pane, where your package manager, Homebrew or cargo updates Tabletist: copy its command |
+| `c` | On the Updates pane: check for a newer version now |
+| `n` | On the Updates pane, while a version is offered: its release notes in `$PAGER` (`less` without one), in a terminal; where none can be started, its page in the browser |
+| `S` | On the Updates pane: skip the version offered |
 | Ctrl+E | Open `settings.toml` in your editor |
 | `R` | Reset the option. The agent server's switch is left as it is |
 | `D`, `n` | On the Editor pane, once keep is set to don't keep: delete the history kept, or keep it |
```

`docs/_reference/settings-and-files.md`:

````diff
diff --git a/docs/_reference/settings-and-files.md b/docs/_reference/settings-and-files.md
index bb034c7c..25465f65 100644
--- a/docs/_reference/settings-and-files.md
+++ b/docs/_reference/settings-and-files.md
@@ -94,6 +94,35 @@ hold, and shows how to connect an agent:
 | **Keep the activity log** | 7, 30, 90 or 365 days | 30 days |
 | **Notify when an agent connects** | On or off | On |
 
+The last pane, **Updates**, says whether a newer version is out, and sets
+how often Tabletist asks. Its card shows one of: not checked yet; up to date (with when
+the last check ran); checking; a version available, with what is new in
+it (the first four points of its release notes), its size and **Release
+notes on GitHub** (its page has the downloads), and **Skip this
+version**; that a release has no build for this computer yet; that
+another tool updates this copy (Homebrew, cargo or your package manager),
+with the command to run and **Copy**; or that the check failed, and why.
+While a version is offered, Updates has a dot in the window's list of
+panes (Omarchy: a `●` after `updates`), and the Omarchy look's status
+line says `0.4.0 available · :update` for 8 seconds, once a day at most.
+Nothing opens over your work. Tabletist does not download or install the
+update itself yet.
+
+| Option | Choices | Default |
+| --- | --- | --- |
+| **Check for updates** | Every day, Every week or Only when I ask. A check runs at launch when the last one that worked is older than that, then again at that interval while Tabletist is open, and waits for a running query to end. **Check now** under it (Omarchy: `c`) checks whatever it says, and so do **Tabletist › Check for Updates…** on macOS, **Help › Check for Updates…** on Windows and `:update` in the Omarchy look | Every day |
+
+The check is one request to
+`https://api.github.com/repos/igor-alexandrov/tabletist/releases/latest`,
+which sends a user agent of this version, the system and the processor
+(`Tabletist/0.3.0 (macOS 15.1; arm64)`, `Tabletist/0.3.0 (Linux; x86_64)`)
+and nothing else: no cookie, no ID, nothing of your databases. It gives
+up after 10 seconds. A check that fails does not count as a check: a
+scheduled one tries again an interval later. Drafts and pre-releases are
+never offered. **Skip this version** (Omarchy: `S`) sets `updates.skip`:
+that version is not offered by a scheduled check, and a newer one is. A
+check you ask for offers it anyway.
+
 On macOS and Windows the window shows where the settings file is, and can
 reveal it in your file manager, export it, or reset the options of the
 pane it shows (on the Editor pane, every key of the `[editor.…]` tables). On the Agents pane, a reset leaves the server's switch as
@@ -166,6 +195,9 @@ table_list = "tree"
 
 [appearance.env]
 bar = "tint"
+
+[updates]
+check = "daily"
 ```
 
 | Key | Values | Default |
@@ -209,6 +241,8 @@ bar = "tint"
 | `mcp.enabled` | `true` lets agents reach the connections you open to them, through `tabletist mcp` | `false` |
 | `mcp.log_days` | 1 to 3650 days. How long the log of agents' calls keeps a call | `30` |
 | `mcp.notify` | `true` tells you when an agent connects: a notification on Linux, the Dock icon bouncing on macOS, the taskbar button flashing on Windows | `true` |
+| `updates.check` | `"daily"`, `"weekly"` or `"never"`. `"never"` checks only when you ask | `"daily"` |
+| `updates.skip` | A version, `"0.4.0"` (a `v` before it is read too): the one **Skip this version** stops offering. An empty string skips none | Not set |
 
 `show_system_schemas` is set only in the file, and so is a pin by a
 name or a path other than the Omarchy theme of the moment, and a font by
@@ -349,6 +383,8 @@ An administrator's policy, `managed.toml`, is in a third place of its own:
 | `agents.sqlite3` | State | Every call agents made: the tool, the statement and its values, and how it ended. Never a row |
 | `history.sqlite3` | State | The query history, by saved connection: each statement run from the SQL editor, a grid's save (its whole transaction, with the values you typed written in) and a filter or sort you set, with when, how long, its rows and how it ended, and a filter's table, filter and sort. An agent's run is kept as its call's place in `agents.sqlite3`, its tool, rows, time and result, never what it sent, its name included: the History tab reads that from `agents.sqlite3`. Never a result's rows. Kept to `editor.history.keep` per connection, and written on its own: a statement never waits for it |
 | `agents_shown.json` | State | What agents may do as Tabletist last showed it: whether the server is on, and each connection's access, hidden tables and masked columns |
+| `updates.json` | State | What the checks for updates found: when the last one that worked ran, the newest version seen, that release's page and notes while it is newer than this version, and when the Omarchy status line last told of it |
+| `release-notes-<version>.md` | State | A release's notes, written for `$PAGER` when `n` shows them in the Omarchy look |
 | `mcp.sock`, `mcp.key`, `mcp.lock` | Runtime, else State | Where `tabletist mcp` reaches the app, the key each proves it holds before either says more, and the lock the app holds while it listens. The socket and the key are there while agents are on |
 | `mcp.pipe` | State | On Windows, in the socket's place: the name of the pipe `tabletist mcp` reaches the app by, new each time agents are turned on |
 
````

`src/app.rs`:

```diff
diff --git a/src/app.rs b/src/app.rs
index f0c958f2..87db19a8 100644
--- a/src/app.rs
+++ b/src/app.rs
@@ -2543,6 +2543,15 @@ impl App {
                     dialog.reveal = false;
                 }
             }
+            Action::ShowUpdates => {
+                self.apply(Action::ShowSettingsPane(
+                    crate::model::SettingsPane::Updates,
+                ));
+                if matches!(&self.dialog, Some(Dialog::Settings(dialog)) if dialog.pane == crate::model::SettingsPane::Updates)
+                {
+                    self.apply(Action::CheckForUpdates);
+                }
+            }
             Action::CheckForUpdates
             | Action::CancelUpdateCheck
             | Action::SkipUpdate
```

`src/app/commands.rs`:

```diff
diff --git a/src/app/commands.rs b/src/app/commands.rs
index 2d5a5aa1..befe10aa 100644
--- a/src/app/commands.rs
+++ b/src/app/commands.rs
@@ -79,7 +79,7 @@ impl Scripts {
 /// review, which the key shows and hides by turns; and its Agents shows
 /// the Agents tab, which the key does but where History has an agent's
 /// statement picked (there it shows that call).
-pub const RUNNABLE: [Command; 29] = [
+pub const RUNNABLE: [Command; 30] = [
     Command::NewSqlTab,
     Command::SwitchDatabase,
     Command::ShowAgents,
@@ -93,6 +93,7 @@ pub const RUNNABLE: [Command; 29] = [
     Command::NewConnection,
     Command::ToggleSidebar,
     Command::Settings,
+    Command::CheckForUpdates,
     Command::Help,
     Command::LargerText,
     Command::SmallerText,
@@ -409,6 +410,11 @@ impl App {
                 vec![Action::NewConnection],
             ),
             Command::Settings => ("Settings", About::Nothing, vec![Action::ShowSettings]),
+            Command::CheckForUpdates => (
+                "Check for Updates…",
+                About::Nothing,
+                vec![Action::ShowUpdates],
+            ),
             Command::Help => ("Keyboard shortcuts", About::Nothing, vec![Action::ShowHelp]),
             // Each where it changes the size: not at the end of the steps,
             // nor back to the look's own where it is that already.
```

`src/app/editing.rs`:

```diff
diff --git a/src/app/editing.rs b/src/app/editing.rs
index 88907be9..ac931902 100644
--- a/src/app/editing.rs
+++ b/src/app/editing.rs
@@ -1941,6 +1941,8 @@ impl App {
             // The keys in force: the full list in the editor, or a cheat
             // sheet where the person says. Neither touches the settings.
             (Typed::Command(Command::KeysDefault), _) => Action::KeysDefault,
+            // Settings › Updates, checking.
+            (Typed::Command(Command::CheckForUpdates), _) => Action::ShowUpdates,
             (Typed::Command(Command::KeysCheatsheet), _) => Action::ExportCheatSheet,
             // The calendar under a date's editor, saved as its switch in
             // Settings saves it.
```

`src/app/menu.rs`:

```diff
diff --git a/src/app/menu.rs b/src/app/menu.rs
index 86265e3d..1d0bdd75 100644
--- a/src/app/menu.rs
+++ b/src/app/menu.rs
@@ -67,7 +67,8 @@ impl App {
     /// it). `keys::handle` does not run under a dialog, so a greyed
     /// Settings would lose `⌘,` there.
     fn clickable_now(&self, command: Command) -> bool {
-        self.key_reaches(command) || (command == Command::Settings && self.settings_may_open())
+        let opens_settings = matches!(command, Command::Settings | Command::CheckForUpdates);
+        self.key_reaches(command) || (opens_settings && self.settings_may_open())
     }
 
     /// Whether a press of `command`'s key is read now: always without a
```

`src/keymap.rs`:

```diff
diff --git a/src/keymap.rs b/src/keymap.rs
index c66720f0..90bd9df4 100644
--- a/src/keymap.rs
+++ b/src/keymap.rs
@@ -227,6 +227,9 @@ pub enum When {
     ConnectionForm,
     /// The Settings screen.
     SettingsScreen,
+    /// The Settings screen shows its Updates pane, where the history's
+    /// question never stands: its keys are that pane's.
+    UpdatesPane,
     /// Quick open.
     QuickOpen,
     /// The question before a save to production.
@@ -440,6 +443,7 @@ commands! {
     GoToConnectionWindow = "go_to_connection_window",
     SwitchDatabase = "switch_database",
     Settings = "settings",
+    CheckForUpdates = "check_for_updates",
     LargerText = "larger_text",
     SmallerText = "smaller_text",
     DefaultTextSize = "default_text_size",
@@ -676,6 +680,10 @@ commands! {
     SettingsCopyCommand = "settings_copy_command",
     SettingsDeleteHistory = "settings_delete_history",
     SettingsKeepHistory = "settings_keep_history",
+    SettingsCheckNow = "settings_check_now",
+    SettingsReleaseNotes = "settings_release_notes",
+    SettingsSkipVersion = "settings_skip_version",
+    SettingsInstallUpdate = "settings_install_update",
     SettingsUnbind = "settings_unbind",
     SettingsFilter = "settings_filter",
     SettingsFindKey = "settings_find_key",
@@ -756,6 +764,7 @@ impl Command {
             }
             C::SwitchDatabase => info("Switch database", "Switch database", Window),
             C::Settings => info("Settings", "Settings", Window),
+            C::CheckForUpdates => info("Check for Updates…", "", Window),
             C::LargerText => info("Larger text", "", Window),
             C::SmallerText => info("Smaller text", "", Window),
             C::DefaultTextSize => info("Default text size", "", Window),
@@ -1067,11 +1076,27 @@ impl Command {
                 Prompts,
             ),
             C::SettingsOpenRow => info("Open what the settings' row shows", "", Prompts),
-            C::SettingsCopyCommand => agents(info(
-                "Copy the command that connects an agent, in the settings",
+            C::SettingsCopyCommand => info(
+                "Copy the command the settings show: an agent's, or the update's",
                 "",
                 Prompts,
-            )),
+            ),
+            C::SettingsCheckNow => info("Check for updates now, in the settings", "", Prompts),
+            C::SettingsReleaseNotes => info(
+                "Read the release notes of the version offered, in the settings",
+                "",
+                Prompts,
+            ),
+            C::SettingsSkipVersion => {
+                info("Skip the version offered, in the settings", "", Prompts)
+            }
+            // Downloading and installing come in the next run.
+            C::SettingsInstallUpdate => info(
+                "Download and install the version offered, in the settings",
+                "",
+                Prompts,
+            )
+            .reserved(),
             C::SettingsDeleteHistory => info(
                 "Delete the query history kept, as the settings ask",
                 "",
@@ -1234,6 +1259,8 @@ pub const BINDINGS: &[Binding] = {
             keys(&[k("ctrl+shift+1..9")]),
         ),
         bind(C::SwitchDatabase, GLOBAL, NONE, keys(&[k("ctrl+shift+d")])),
+        // macOS's Tabletist menu has it; Omarchy's prompt.
+        bind(C::CheckForUpdates, GLOBAL, NONE, keys(&[k(":update")])),
         bind(
             C::Settings,
             GLOBAL,
@@ -2342,6 +2369,11 @@ pub const BINDINGS: &[Binding] = {
         bind(C::SettingsCopyCommand, PROMPT, NONE, keys(&[k("y")])).when(When::SettingsScreen),
         bind(C::SettingsDeleteHistory, PROMPT, NONE, keys(&[k("D")])).when(When::SettingsScreen),
         bind(C::SettingsKeepHistory, PROMPT, NONE, keys(&[k("n")])).when(When::SettingsScreen),
+        // The Updates pane's own.
+        bind(C::SettingsCheckNow, PROMPT, NONE, keys(&[k("c")])).when(When::UpdatesPane),
+        bind(C::SettingsReleaseNotes, PROMPT, NONE, keys(&[k("n")])).when(When::UpdatesPane),
+        bind(C::SettingsSkipVersion, PROMPT, NONE, keys(&[k("S")])).when(When::UpdatesPane),
+        bind(C::SettingsInstallUpdate, PROMPT, NONE, keys(&[k("U")])).when(When::UpdatesPane),
         // The Keyboard pane's own: Enter records the row (the screen's
         // Enter), `R` resets it, and the arrows' letters move as on any
         // pane.
```

`src/menu.rs`:

```diff
diff --git a/src/menu.rs b/src/menu.rs
index 17f61dad..344ec3f3 100644
--- a/src/menu.rs
+++ b/src/menu.rs
@@ -76,6 +76,7 @@ pub fn bar(platform: Platform) -> Vec<Menu> {
             role: Role::Plain,
             items: vec![
                 Item::About,
+                Run(C::CheckForUpdates),
                 Sep,
                 Run(C::Settings),
                 Sep,
@@ -180,7 +181,8 @@ pub fn bar(platform: Platform) -> Vec<Menu> {
     });
     let mut help = vec![Run(C::Help)];
     if !mac {
-        help.extend([Sep, Item::About]);
+        // macOS has it under About, in the Tabletist menu.
+        help.extend([Sep, Run(C::CheckForUpdates), Item::About]);
     }
     menus.push(Menu {
         title: "Help",
@@ -203,6 +205,7 @@ pub fn title(item: Item, on: bool) -> &'static str {
             C::CloseConnection => "Close Connection",
             C::CloseTab => "Close Tab",
             C::Settings => "Settings…",
+            C::CheckForUpdates => "Check for Updates…",
             C::ToggleSidebar if on => "Hide Sidebar",
             C::ToggleSidebar => "Show Sidebar",
             C::ToggleInspector if on => "Hide Row Panel",
```

`src/model.rs`:

```diff
diff --git a/src/model.rs b/src/model.rs
index 73550cb0..cf3eff06 100644
--- a/src/model.rs
+++ b/src/model.rs
@@ -994,6 +994,9 @@ pub enum Action {
     /// menu's Check for Updates…, `c`, `:update`. A version the settings
     /// skip is offered by a check asked for.
     CheckForUpdates,
+    /// Show Settings › Updates and check there now: the menu's Check for
+    /// Updates…, quick open's, `:update`.
+    ShowUpdates,
     /// Drop the check under way: what it brings is not read.
     CancelUpdateCheck,
     /// Stop offering the version offered: `updates.skip`.
@@ -3012,6 +3015,8 @@ pub enum SettingsPane {
     Keyboard,
     /// How agents reach the app. Listed where they can.
     Agents,
+    /// Whether a newer version is out, and how often to ask.
+    Updates,
 }
 
 impl SettingsPane {
@@ -3024,6 +3029,7 @@ impl SettingsPane {
                 Self::Editor,
                 Self::Keyboard,
                 Self::Agents,
+                Self::Updates,
             ]
         } else {
             &[
@@ -3031,6 +3037,7 @@ impl SettingsPane {
                 Self::Appearance,
                 Self::Editor,
                 Self::Keyboard,
+                Self::Updates,
             ]
         }
     }
@@ -3043,15 +3050,22 @@ impl SettingsPane {
             Self::Editor => "Editor",
             Self::Keyboard => "Keyboard",
             Self::Agents => "Agents",
+            Self::Updates => "Updates",
         }
     }
 
     /// The pane whose rows hold `option`.
     pub fn of(option: crate::settings::OptionId) -> Self {
-        [Self::General, Self::Appearance, Self::Editor, Self::Agents]
-            .into_iter()
-            .find(|pane| pane.options().contains(&option))
-            .unwrap_or_default()
+        [
+            Self::General,
+            Self::Appearance,
+            Self::Editor,
+            Self::Agents,
+            Self::Updates,
+        ]
+        .into_iter()
+        .find(|pane| pane.options().contains(&option))
+        .unwrap_or_default()
     }
 
     /// The sections the sheet (macOS, Windows) shows the pane's options
@@ -3079,7 +3093,7 @@ impl SettingsPane {
                 ("Format · {format}", &OptionId::FORMAT),
                 ("History", &OptionId::HISTORY),
             ],
-            Self::Keyboard | Self::Agents => &[],
+            Self::Keyboard | Self::Agents | Self::Updates => &[],
         }
     }
 
@@ -3089,6 +3103,7 @@ impl SettingsPane {
     pub fn sheet_options(self) -> Vec<crate::settings::OptionId> {
         match self {
             Self::Agents => crate::settings::OptionId::AGENTS.to_vec(),
+            Self::Updates => crate::settings::OptionId::UPDATES.to_vec(),
             Self::Keyboard => Vec::new(),
             Self::General | Self::Appearance | Self::Editor => self
                 .sections()
@@ -3106,6 +3121,7 @@ impl SettingsPane {
             Self::Editor => &crate::settings::OptionId::EDITOR,
             Self::Keyboard => &[],
             Self::Agents => &crate::settings::OptionId::AGENTS,
+            Self::Updates => &crate::settings::OptionId::UPDATES,
         }
     }
 
@@ -3122,6 +3138,7 @@ impl SettingsPane {
             // Reset all… of its own, which asks first.
             Self::Keyboard => &[],
             Self::Agents => &[OptionId::LogDays, OptionId::AgentsNotify],
+            Self::Updates => &OptionId::UPDATES,
         }
     }
 
@@ -3178,6 +3195,8 @@ impl SettingsPane {
                 SettingsRow::Option(OptionId::LogDays),
                 SettingsRow::Option(OptionId::AgentsNotify),
             ],
+            // The status over it is no row: its keys are the pane's.
+            Self::Updates => &[SettingsRow::Option(OptionId::UpdatesCheck)],
         }
     }
 }
```

`src/settings.rs`:

```diff
diff --git a/src/settings.rs b/src/settings.rs
index 94af0f02..c5dd31cd 100644
--- a/src/settings.rs
+++ b/src/settings.rs
@@ -1017,12 +1017,14 @@ pub enum OptionId {
     Commas,
     /// Queries the history keeps: `editor.history.keep`.
     HistoryKeep,
+    /// How often a check for a newer release runs: `updates.check`.
+    UpdatesCheck,
 }
 
 impl OptionId {
     /// Every option, the General pane's, the Appearance pane's, the Editor
-    /// pane's and then the Agents pane's.
-    pub const ALL: [OptionId; 38] = [
+    /// pane's, the Agents pane's and then the Updates pane's.
+    pub const ALL: [OptionId; 39] = [
         Self::OnLaunch,
         Self::RestoreTabs,
         Self::PageSize,
@@ -1061,6 +1063,7 @@ impl OptionId {
         Self::AgentsOn,
         Self::LogDays,
         Self::AgentsNotify,
+        Self::UpdatesCheck,
     ];
     /// The General pane's, in the order it has them.
     pub const GENERAL: [OptionId; 11] = [
@@ -1172,6 +1175,9 @@ impl OptionId {
     pub const HISTORY: [OptionId; 1] = [Self::HistoryKeep];
     /// The Agents pane's, in the order it has them.
     pub const AGENTS: [OptionId; 3] = [Self::AgentsOn, Self::LogDays, Self::AgentsNotify];
+    /// The Updates pane's. The version to skip has no row: its card's
+    /// Skip this version sets it.
+    pub const UPDATES: [OptionId; 1] = [Self::UpdatesCheck];
 
     /// The key of the file the option is stored under.
     pub fn key(self) -> Key {
@@ -1214,6 +1220,7 @@ impl OptionId {
             Self::Keywords => Key::Keywords,
             Self::Commas => Key::Commas,
             Self::HistoryKeep => Key::HistoryKeep,
+            Self::UpdatesCheck => Key::UpdatesCheck,
         }
     }
 
@@ -1274,6 +1281,7 @@ pub enum OptionValue {
     Commas(Commas),
     /// 0 keeps none.
     HistoryKeep(u32),
+    UpdatesCheck(UpdateCheck),
 }
 
 impl OptionValue {
@@ -1342,6 +1350,7 @@ impl OptionValue {
             Self::Keywords(_) => OptionId::Keywords,
             Self::Commas(_) => OptionId::Commas,
             Self::HistoryKeep(_) => OptionId::HistoryKeep,
+            Self::UpdatesCheck(_) => OptionId::UpdatesCheck,
         }
     }
 
@@ -1397,6 +1406,7 @@ impl OptionValue {
             Self::Keywords(case) => settings.keywords = case,
             Self::Commas(commas) => settings.commas = commas,
             Self::HistoryKeep(keep) => settings.history_keep = keep.min(Settings::MAX_HISTORY),
+            Self::UpdatesCheck(check) => settings.update_check = check,
         }
     }
 }
@@ -2056,6 +2066,7 @@ impl Settings {
             OptionId::Keywords => OptionValue::Keywords(self.keywords),
             OptionId::Commas => OptionValue::Commas(self.commas),
             OptionId::HistoryKeep => OptionValue::HistoryKeep(self.history_keep),
+            OptionId::UpdatesCheck => OptionValue::UpdatesCheck(self.update_check),
         }
     }
 
@@ -2264,6 +2275,11 @@ impl Settings {
                 },
                 forward,
             )),
+            OptionId::UpdatesCheck => OptionValue::UpdatesCheck(stepped_among(
+                &UpdateCheck::ALL,
+                self.update_check,
+                forward,
+            )),
         }
     }
 
@@ -2423,7 +2439,8 @@ impl Settings {
             | OptionId::Autocomplete
             | OptionId::LineNumbers
             | OptionId::Keywords
-            | OptionId::HistoryKeep => None,
+            | OptionId::HistoryKeep
+            | OptionId::UpdatesCheck => None,
         }
     }
 
```

`src/theme.rs`:

```diff
diff --git a/src/theme.rs b/src/theme.rs
index e31036be..b7c348d0 100644
--- a/src/theme.rs
+++ b/src/theme.rs
@@ -960,6 +960,7 @@ fastframe_icons::icons! {
         ChevronsUpDown => "chevrons-up-down",
         Code => "code",
         Database => "database",
+        Download => "download",
         Funnel => "funnel",
         Image => "image",
         KeyRound => "key-round",
```

`src/ui/settings/mod.rs`:

```diff
diff --git a/src/ui/settings/mod.rs b/src/ui/settings/mod.rs
index a22ca181..924f9b2e 100644
--- a/src/ui/settings/mod.rs
+++ b/src/ui/settings/mod.rs
@@ -16,6 +16,8 @@ mod editor_preview;
 mod preview;
 mod sheet;
 mod terminal;
+mod updates;
+mod updates_terminal;
 
 use std::borrow::Cow;
 
@@ -27,7 +29,7 @@ use crate::model::{Action, AgentsView, Dialog, SettingsDialog, SettingsPane, Set
 use crate::settings::{
     Autocomplete, Commas, EditorKeys, EnvBar, Idle, InterfaceSize, KeywordCase, Launch,
     LineNumbers, Mode, NewTab, OptionId, OptionValue, RowHeight, RunScope, Settings, TableList,
-    Theme, ThemeChoice, Timestamps, Zone,
+    Theme, ThemeChoice, Timestamps, UpdateCheck, Zone,
 };
 
 /// The label of an option's row. English: the caller translates, and the
@@ -73,6 +75,7 @@ pub(crate) fn label(option: OptionId) -> &'static str {
         OptionId::Keywords => "Keywords",
         OptionId::Commas => "Commas",
         OptionId::HistoryKeep => "Keep query history",
+        OptionId::UpdatesCheck => "Check for updates",
     }
 }
 
@@ -142,7 +145,8 @@ pub(super) fn terminal_label(option: OptionId) -> &'static str {
         | OptionId::HighlightLine
         | OptionId::Wrap
         | OptionId::Keywords
-        | OptionId::Commas => label(option),
+        | OptionId::Commas
+        | OptionId::UpdatesCheck => label(option),
     }
 }
 
@@ -177,7 +181,9 @@ pub(super) fn small_print(option: OptionId) -> Option<&'static str> {
         OptionId::HistoryKeep => Some(
             "Per connection, on this computer: SQL editor, grid saves, filters and agent runs. Results are never stored.",
         ),
-        OptionId::Timeout
+        // When the last check ran, with Check now: the pane writes it.
+        OptionId::UpdatesCheck
+        | OptionId::Timeout
         | OptionId::RunScope
         | OptionId::EditorKeys
         | OptionId::Indent
@@ -289,6 +295,16 @@ pub(super) fn idle_text(locale: Locale, choice: Idle) -> String {
     ngettext(locale, one, many, count).replace("{n}", &count.to_string())
 }
 
+/// How often a check for updates runs, as the window writes it.
+pub(super) fn check_text(locale: Locale, choice: UpdateCheck) -> String {
+    let words = match choice {
+        UpdateCheck::Daily => "Every day",
+        UpdateCheck::Weekly => "Every week",
+        UpdateCheck::Never => "Only when I ask",
+    };
+    gettext(locale, words).into_owned()
+}
+
 /// The days the log keeps a call, as the window writes them.
 pub(super) fn days(locale: Locale, days: u32) -> String {
     ngettext(locale, "{n} day", "{n} days", days).replace("{n}", &days.to_string())
@@ -458,7 +474,7 @@ pub(super) fn choices(
             (Word("off"), OptionValue::AgentsOn(false)),
             (Word("on"), OptionValue::AgentsOn(true)),
         ],
-        OptionId::LogDays | OptionId::IdleDisconnect => vec![
+        OptionId::LogDays | OptionId::IdleDisconnect | OptionId::UpdatesCheck => vec![
             (Word("fewer"), settings.stepped(option, false)),
             (Word("more"), settings.stepped(option, true)),
         ],
@@ -648,6 +664,10 @@ enum Asked {
     Copy,
     /// Answer the history's question: delete what it kept, or keep it.
     History(bool),
+    /// The Updates pane's: check now, the notes, skip the version offered.
+    CheckNow,
+    Notes,
+    Skip,
 }
 
 /// What `event` asks of the screen, if it is a press of one of its keys,
@@ -657,10 +677,26 @@ enum Asked {
 /// workspace: the screen's plain keys but its close. Its letters, the
 /// chord that opens the file and the key that closes are the screen's
 /// wherever the keyboard is.
-fn asked(keymap: &Keymap, event: &egui::Event, held: egui::Modifiers, free: bool) -> Option<Asked> {
+fn asked(
+    keymap: &Keymap,
+    event: &egui::Event,
+    held: egui::Modifiers,
+    (free, pane): (bool, SettingsPane),
+) -> Option<Asked> {
     // The screen is Omarchy's.
     let layout = Layout::Omarchy;
-    let commands = [
+    // The Updates pane's own come first there, and are read nowhere else:
+    // its `n` is the notes, which the history's question has elsewhere.
+    let updates = [
+        Command::SettingsCheckNow,
+        Command::SettingsReleaseNotes,
+        Command::SettingsSkipVersion,
+    ];
+    let updates = match pane {
+        SettingsPane::Updates => &updates[..],
+        _ => &[],
+    };
+    let commands = updates.iter().copied().chain([
         Command::SettingsOpenFile,
         Command::SettingsReset,
         Command::SettingsClose,
@@ -672,7 +708,7 @@ fn asked(keymap: &Keymap, event: &egui::Event, held: egui::Modifiers, free: bool
         Command::SettingsCopyCommand,
         Command::SettingsDeleteHistory,
         Command::SettingsKeepHistory,
-    ];
+    ]);
     // A letter is read as the key that types it, with Shift for its
     // capital and nothing else held.
     let typed = match event {
@@ -721,6 +757,9 @@ fn asked(keymap: &Keymap, event: &egui::Event, held: egui::Modifiers, free: bool
                 Command::SettingsCopyCommand => Asked::Copy,
                 Command::SettingsDeleteHistory => Asked::History(true),
                 Command::SettingsKeepHistory => Asked::History(false),
+                Command::SettingsCheckNow => Asked::CheckNow,
+                Command::SettingsReleaseNotes => Asked::Notes,
+                Command::SettingsSkipVersion => Asked::Skip,
                 _ => Asked::Flip,
             });
         }
@@ -755,10 +794,11 @@ fn keys(
     let keymap = app.keymap.clone();
     // Copied once the input is let go: the clipboard is the context's.
     let mut copy = false;
+    let mut update_command = None;
     ctx.input_mut(|input| {
         let held = input.modifiers;
         input.events.retain(|event| {
-            let Some(asked) = asked(&keymap, event, held, free) else {
+            let Some(asked) = asked(&keymap, event, held, (free, pane)) else {
                 return true;
             };
             if preview && !matches!(asked, Asked::Edit | Asked::Pane { .. }) {
@@ -805,11 +845,35 @@ fn keys(
                         actions.extend(opens);
                     }
                 }
-                Asked::Copy => {
-                    if pane != SettingsPane::Agents {
-                        return true;
+                Asked::Copy => match pane {
+                    SettingsPane::Agents => copy |= !repeat,
+                    // The command that updates a copy something else
+                    // installed, where it has one.
+                    SettingsPane::Updates => match updates::managed_command(app) {
+                        Some(command) => {
+                            if !repeat {
+                                update_command = Some(command);
+                            }
+                        }
+                        None => return true,
+                    },
+                    _ => return true,
+                },
+                // Fresh presses only: a held key would ask again and again.
+                Asked::CheckNow => {
+                    if !repeat {
+                        actions.push(Action::CheckForUpdates);
+                    }
+                }
+                Asked::Notes => {
+                    if !repeat {
+                        actions.push(Action::ShowReleaseNotes);
+                    }
+                }
+                Asked::Skip => {
+                    if !repeat {
+                        actions.push(Action::SkipUpdate);
                     }
-                    copy |= !repeat;
                 }
                 // Only while the question stands.
                 Asked::History(delete) => {
@@ -866,6 +930,9 @@ fn keys(
     if copy {
         crate::ui::agents::copy_command(ctx, app.locale);
     }
+    if let Some(command) = update_command {
+        updates::copy_command(ctx, command, app.locale);
+    }
 }
 
 /// The editor preview's text field, for the tests.
```

`src/ui/settings/sheet.rs`:

```diff
diff --git a/src/ui/settings/sheet.rs b/src/ui/settings/sheet.rs
index e705fbce..b332c1d4 100644
--- a/src/ui/settings/sheet.rs
+++ b/src/ui/settings/sheet.rs
@@ -13,7 +13,7 @@ use crate::model::{Action, SettingsDialog, SettingsPane};
 use crate::settings::{
     Autocomplete, Commas, EditorKeys, EnvBar, FontSizes, Idle, Indent, InterfaceSize, KeywordCase,
     Launch, LineNumbers, Mode, NewTab, OptionId, OptionValue, RowHeight, RunScope, Settings,
-    TableList, ThemeChoice, Timestamps, Zone,
+    TableList, ThemeChoice, Timestamps, UpdateCheck, Zone,
 };
 use crate::theme::{DataFace, Look, Palette, Typeface};
 use crate::typography::{Laid, Text, TextRole};
@@ -174,7 +174,8 @@ pub(super) fn show(
             // laid out after it: its fill waits here, under its words, for
             // that height.
             let fill = ui.painter().add(egui::Shape::Noop);
-            nav(ui, room.min, dialog.pane, &skin, actions);
+            let offered = super::updates::offered(app);
+            nav(ui, (room.min, offered), dialog.pane, &skin, actions);
             // No bottom of its own: its rows end where `content` is told to
             // end them. The sheet's bottom is the one the frame before left
             // it, and rows held to that would get their height back a
@@ -251,6 +252,9 @@ pub(super) fn show(
                 SettingsPane::Agents => {
                     super::agents::content(&mut pane, app, dialog.agent, limit, &skin, actions);
                 }
+                SettingsPane::Updates => {
+                    super::updates::content(&mut pane, app, limit, &skin, actions);
+                }
             }
             let rows = pane.min_rect().bottom();
             // Lower than the rows' end, unless a window too short for even
@@ -305,10 +309,11 @@ pub(super) fn show(
 }
 
 /// The window's name and its panes, down the left from `corner`, the one
-/// shown filled: a click shows another.
+/// shown filled: a click shows another. Updates has a dot while a newer
+/// version is `offered`.
 fn nav(
     ui: &mut Ui,
-    corner: egui::Pos2,
+    (corner, offered): (egui::Pos2, bool),
     shown: SettingsPane,
     skin: &Skin,
     actions: &mut Vec<Action>,
@@ -357,6 +362,13 @@ fn nav(
             item.center().y,
             Text::one(look, role, &name, color),
         );
+        if pane == SettingsPane::Updates && offered {
+            let dot =
+                Rect::from_center_size(pos2(item.right() - 12.0, item.center().y), vec2(7.0, 7.0));
+            ui.painter()
+                .circle_filled(dot.center(), 3.5, palette.accent);
+            widgets::announce(ui, dot, &skin.say("Update available"));
+        }
         // An item of a list: the ring is inside its fill, as a row's is.
         focus::hint(ui, &button, item, Ring::Inset { radius: 5 });
     }
@@ -669,6 +681,43 @@ pub(super) fn row(
     skin: &Skin,
     actions: &mut Vec<Action>,
     after: Option<After<'_>>,
+) {
+    row_of(
+        ui,
+        (option, ruled, column),
+        settings,
+        skin,
+        actions,
+        (after, None),
+    );
+}
+
+/// What a row says under its label where that is not its small print but
+/// changes as the app runs, and the link at its end.
+pub(super) struct Print<'a> {
+    pub(super) text: &'a str,
+    pub(super) link: Option<(&'a str, Action)>,
+}
+
+/// [`row`], with `print` under its label in place of its small print.
+pub(super) fn row_with(
+    ui: &mut Ui,
+    place: (OptionId, bool, f32),
+    settings: &Settings,
+    skin: &Skin,
+    actions: &mut Vec<Action>,
+    print: Print<'_>,
+) {
+    row_of(ui, place, settings, skin, actions, (None, Some(print)));
+}
+
+fn row_of(
+    ui: &mut Ui,
+    (option, ruled, column): (OptionId, bool, f32),
+    settings: &Settings,
+    skin: &Skin,
+    actions: &mut Vec<Action>,
+    (after, shown): (Option<After<'_>>, Option<Print<'_>>),
 ) {
     let Skin { look, palette, .. } = *skin;
     // Wrapped in its column too: a narrow sheet takes from the labels'
@@ -690,13 +739,18 @@ pub(super) fn row(
                 .replace("{name}", missing);
             Some((said, palette.warning))
         }
-        None => small_print(option).map(|print| (skin.say(print), palette.dim)),
+        None => match &shown {
+            Some(shown) => Some((shown.text.to_owned(), palette.dim)),
+            None => small_print(option).map(|print| (skin.say(print), palette.dim)),
+        },
     };
     // What the row says under its label, or beside its control where it
     // says nothing under it, is what its controls are for: a screen reader
-    // reads it as their description.
+    // reads it as their description. What changes as the app runs (when
+    // the last check ran) describes nothing: it is read where it stands.
     let description = print
         .as_ref()
+        .filter(|_| shown.is_none())
         .map(|(print, _)| print.clone())
         .or_else(|| super::helper(option).map(|words| skin.say(words)));
     let print = print.map(|(print, color)| {
@@ -715,10 +769,32 @@ pub(super) fn row(
     }
     let middle = rect.top() + ROW_PAD + inner / 2.0;
     let mut top = middle - words / 2.0;
+    let mut end = None;
     for line in std::iter::once(name).chain(print) {
         write(ui, rect.left(), top + line.height() / 2.0, &line);
+        end = Some(pos2(rect.left() + line.width(), top + line.height() / 2.0));
         top += line.height() + 2.0;
     }
+    // The link at the end of what changes: `· Check now`.
+    if let (Some(end), Some((text, action))) = (end, shown.and_then(|shown| shown.link)) {
+        let role = TextRole::Secondary;
+        let dot = Text::one(look, role, " · ", palette.dim).layout(ui.ctx());
+        write(ui, end.x, end.y, &dot);
+        let at = end.x + dot.width();
+        let width = role.width(ui.ctx(), look.faces, text).ceil();
+        let link = link(
+            ui,
+            ui.id().with(("row-link", label(option))),
+            pos2(at, end.y),
+            width,
+            text,
+            palette.accent,
+            skin,
+        );
+        if pressed(ui, &link) {
+            actions.push(action);
+        }
+    }
     // The control and its hint, on the row's middle. Each row's controls
     // have ids of their own: two segmented controls in one `Ui` would
     // share their arrow keys.
@@ -859,7 +935,9 @@ fn control_height(option: OptionId, look: &Look) -> f32 {
         | OptionId::AgentsNotify
         | OptionId::Zebra
         | OptionId::ColumnTypes => SWITCH,
-        OptionId::LogDays | OptionId::IdleDisconnect => look.control_height,
+        OptionId::LogDays | OptionId::IdleDisconnect | OptionId::UpdatesCheck => {
+            look.control_height
+        }
         // The Theme section is tiles, drawn on their own: no row's.
         OptionId::Theme => 0.0,
     }
@@ -1052,6 +1130,25 @@ fn control(
                 info
             });
         }
+        OptionId::UpdatesCheck => {
+            let current = settings.update_check;
+            let choices = UpdateCheck::ALL.into_iter().map(|choice| {
+                (
+                    super::check_text(skin.locale, choice),
+                    choice == current,
+                    OptionValue::UpdatesCheck(choice),
+                )
+            });
+            let shown = super::check_text(skin.locale, current);
+            choice_menu(
+                ui,
+                ("update-check", &name, description),
+                &shown,
+                choices,
+                skin,
+                actions,
+            );
+        }
         OptionId::Tz => zone_menu(ui, (&name, description), settings, skin, actions),
         OptionId::OnLaunch => launch_menu(ui, (&name, description), settings, skin, actions),
         OptionId::Timestamps => {
@@ -1507,6 +1604,15 @@ fn hint(ui: &Ui, at: egui::Pos2, right: f32, option: OptionId, settings: &Settin
             &skin.say("A selection always wins"),
             palette.dim,
         ),
+        // The other two choices.
+        OptionId::UpdatesCheck => {
+            let others = match settings.update_check {
+                UpdateCheck::Daily => "Or: every week · only when I ask",
+                UpdateCheck::Weekly => "Or: every day · only when I ask",
+                UpdateCheck::Never => "Or: every day · every week",
+            };
+            Text::one(look, TextRole::Secondary, &skin.say(others), palette.dim)
+        }
         // What the Appearance pane's rows say is their small print, and its
         // tiles have their own line.
         OptionId::Theme
@@ -1682,9 +1788,10 @@ fn reset_question(pane: SettingsPane) -> &'static str {
     match pane {
         SettingsPane::General => "Reset all General settings to their defaults?",
         SettingsPane::Editor => "Reset all Editor settings to their defaults?",
-        SettingsPane::Appearance | SettingsPane::Keyboard | SettingsPane::Agents => {
-            "Reset every option on this tab?"
-        }
+        SettingsPane::Appearance
+        | SettingsPane::Keyboard
+        | SettingsPane::Agents
+        | SettingsPane::Updates => "Reset every option on this tab?",
     }
 }
 
@@ -1739,6 +1846,13 @@ fn footer(
         keyboard_footer(ui, (left, right, middle), app, skin, actions);
         return;
     }
+    // Where else a check is asked for: the menu bar's item, on macOS.
+    if shown_pane == SettingsPane::Updates && cfg!(target_os = "macos") && asking.is_none() {
+        let said = skin.say("Also in the menu: Tabletist › Check for Updates…");
+        let text = Text::one(look, role, &said, palette.secondary).layout(ui.ctx());
+        write(ui, right - text.width(), middle, &text);
+        return;
+    }
     // The right side first: what it leaves is the path's.
     let parts = if let Some(pane) = asking {
         vec![
```

`src/ui/settings/terminal.rs`:

```diff
diff --git a/src/ui/settings/terminal.rs b/src/ui/settings/terminal.rs
index 23ce1dcb..8c91493d 100644
--- a/src/ui/settings/terminal.rs
+++ b/src/ui/settings/terminal.rs
@@ -31,7 +31,7 @@ const NOTICE_SIDE: f32 = 14.0;
 const NAV: f32 = 220.0;
 /// An option's row, and how far its words stand in from its sides.
 const ROW: f32 = 30.0;
-const ROW_SIDE: f32 = 16.0;
+pub(super) const ROW_SIDE: f32 = 16.0;
 /// The label's column and the value's at their widest; the hint has what
 /// is left.
 const LABEL: f32 = 280.0;
@@ -66,6 +66,9 @@ pub(super) struct Skin<'a> {
     pub(super) desktop: Desktop<'a>,
     /// What the font's row says of the font drawn.
     pub(super) font: FontShown<'a>,
+    /// When the last check for updates that worked ran, as the Updates
+    /// pane's row says it: `last checked 09:12`.
+    pub(super) checked: Option<&'a str>,
 }
 
 /// What fontconfig gave for the font the settings name, which the font's
@@ -160,6 +163,7 @@ pub(super) fn show(
     let (pane, row) = (dialog.pane, dialog.row);
     let settings = super::shown(app);
     let launches = super::launch_choices(&app.connections.connections);
+    let checked = super::updates::checked_text(app, true);
     let skin = Skin {
         look: &app.look,
         palette: &app.palette,
@@ -171,6 +175,7 @@ pub(super) fn show(
         launches: &launches,
         desktop: Desktop::of(app),
         font: FontShown::of(app),
+        checked: checked.as_deref(),
     };
     let screen = ctx.content_rect();
     let id = egui::Id::new("settings");
@@ -197,7 +202,8 @@ pub(super) fn show(
             let (body, _) =
                 ui.allocate_exact_size(vec2(ui.available_width(), height), Sense::hover());
             let (side, place) = body.split_left_right_at_x((body.left() + NAV).min(body.right()));
-            nav(ui, side, pane, &skin, actions);
+            let offered = super::updates::offered(app);
+            nav(ui, (side, offered), pane, &skin, actions);
             // The rows scroll when the window is short, so the header and
             // the footer stay on screen. They start past the nav's rule.
             let place = place.with_min_x((place.left() + 1.0).min(place.right()));
@@ -226,6 +232,9 @@ pub(super) fn show(
                         super::keyboard_terminal::rows(ui, app, &dialog.keyboard, &skin, actions);
                     }
                     SettingsPane::Agents => agent_rows(ui, app, &settings, row, &skin, actions),
+                    SettingsPane::Updates => {
+                        super::updates_terminal::rows(ui, app, &settings, row, &skin, actions);
+                    }
                 });
             if let Some(beside) = beside {
                 let shown = match pane {
@@ -380,8 +389,15 @@ fn mark(ui: &Ui, rect: Rect, palette: &Palette) {
 }
 
 /// The panes, down the left, the one shown marked: a click shows another.
-/// Under them, where there are two, the key that switches.
-fn nav(ui: &mut Ui, rect: Rect, shown: SettingsPane, skin: &Skin, actions: &mut Vec<Action>) {
+/// Under them, where there are two, the key that switches. Updates has a
+/// `●` while a newer version is `offered`.
+fn nav(
+    ui: &mut Ui,
+    (rect, offered): (Rect, bool),
+    shown: SettingsPane,
+    skin: &Skin,
+    actions: &mut Vec<Action>,
+) {
     let Skin {
         look,
         palette,
@@ -411,12 +427,20 @@ fn nav(ui: &mut Ui, rect: Rect, shown: SettingsPane, skin: &Skin, actions: &mut
         } else {
             palette.dim
         };
-        widgets::paint_label(
+        let width = widgets::paint_label(
             ui,
             item.left() + 14.0,
             item.center().y,
             Text::one(look, TextRole::OBody, &skin.say(pane.name()), color),
         );
+        if pane == SettingsPane::Updates && offered {
+            let at = item.left() + 14.0 + width;
+            let dot = Text::one(look, TextRole::OBody, " ●", palette.accent).layout(ui.ctx());
+            dot.paint_left(ui.painter(), at, item.center().y);
+            let place =
+                Rect::from_center_size(pos2(at + dot.width() / 2.0, item.center().y), dot.size());
+            widgets::announce(ui, place, &gettext(locale, "Update available"));
+        }
     }
     if listed.len() > 1 {
         let switch = key(ui, look, Command::SettingsPane);
@@ -430,7 +454,7 @@ fn nav(ui: &mut Ui, rect: Rect, shown: SettingsPane, skin: &Skin, actions: &mut
 /// A group's heading over its rows: 12 over it and 4 under, after `top`
 /// at the pane's top or between two groups. A whole number of points, so
 /// the rows' fills start on a pixel.
-fn heading(ui: &mut Ui, text: &'static str, top: f32, skin: &Skin) {
+pub(super) fn heading(ui: &mut Ui, text: &'static str, top: f32, skin: &Skin) {
     let Skin { look, palette, .. } = *skin;
     let role = TextRole::OCaption;
     let line = role.row_height(ui.ctx(), look.faces);
@@ -1016,7 +1040,12 @@ fn note(ui: &mut Ui, skin: &Skin) {
 /// The widest label, value and hint `options` have: the hints of every
 /// value, not only the ones shown, so that the columns stay where they are
 /// when a value changes.
-fn widths(ui: &Ui, options: &[OptionId], settings: &Settings, skin: &Skin) -> (f32, f32, f32) {
+pub(super) fn widths(
+    ui: &Ui,
+    options: &[OptionId],
+    settings: &Settings,
+    skin: &Skin,
+) -> (f32, f32, f32) {
     let role = TextRole::OBody;
     let width = |text: &str| role.width(ui.ctx(), skin.look.faces, text);
     let labels = options
@@ -1051,7 +1080,7 @@ fn widths(ui: &Ui, options: &[OptionId], settings: &Settings, skin: &Skin) -> (f
 /// `LABEL` and `VALUE` where the widest hint has room after them. Rows with
 /// less (the file is beside them) take from the space each column has past
 /// its widest words, from both alike, before a hint is left out.
-fn columns(
+pub(super) fn columns(
     ui: &Ui,
     room: f32,
     (labels, values, hints): (f32, f32, f32),
@@ -1088,7 +1117,8 @@ fn value_width(ui: &Ui, option: OptionId, settings: &Settings, skin: &Skin) -> f
         | OptionId::RowLimit
         | OptionId::Timeout
         | OptionId::Indent
-        | OptionId::HistoryKeep => width(&format!("‹ {} ›", stepped_text(option, settings, skin))),
+        | OptionId::HistoryKeep
+        | OptionId::UpdatesCheck => width(&format!("‹ {} ›", stepped_text(option, settings, skin))),
         OptionId::Font => width(&font_value(settings, skin)),
         OptionId::Timestamps
         | OptionId::GroupDigits
@@ -1164,6 +1194,8 @@ fn stepped_text(option: OptionId, settings: &Settings, skin: &Skin) -> String {
                 .say("{n} per connection")
                 .replace("{n}", &keep.to_string()),
         },
+        // As the file writes it: `daily`.
+        OptionId::UpdatesCheck => settings.update_check.name().to_owned(),
         // A connection by its name as it is saved, and where it points:
         // three of one name are told apart.
         OptionId::OnLaunch => {
@@ -1293,7 +1325,7 @@ fn text_row(
 
 /// One option's row: its label, its value where a click sets it, and the
 /// hint that shows what the value does.
-fn option_row(
+pub(super) fn option_row(
     ui: &mut Ui,
     (index, option): (usize, OptionId),
     on_cursor: bool,
@@ -1365,7 +1397,8 @@ fn option_row(
         | OptionId::RowLimit
         | OptionId::Timeout
         | OptionId::Indent
-        | OptionId::HistoryKeep => stepper(ui, cell, option, settings, skin, actions),
+        | OptionId::HistoryKeep
+        | OptionId::UpdatesCheck => stepper(ui, cell, option, settings, skin, actions),
         // Nothing to step to: a name is set in the file.
         OptionId::Font => {
             let value = Text::one(
@@ -1649,6 +1682,22 @@ fn hint(ui: &Ui, option: OptionId, settings: &Settings, skin: &Skin) -> Text {
         OptionId::HistoryKeep => {
             dim(&skin.say("per connection · sql, saves, filters, agents · never results"))
         }
+        // When the last check ran, and the key that checks. Not the
+        // board's "or weekly · never" before them: beside the file the
+        // hint has no room for all of it, and the chevrons say there are
+        // other values.
+        OptionId::UpdatesCheck => {
+            let mut text = Text::new(look);
+            if let Some(checked) = skin.checked {
+                text = text.add(role, &format!("{checked} · "), palette.dim);
+            }
+            text.add(
+                role,
+                &key(ui, look, Command::SettingsCheckNow),
+                palette.accent,
+            )
+            .add(role, &format!(" {}", skin.say("check now")), palette.dim)
+        }
         OptionId::Timeout
         | OptionId::Indent
         | OptionId::ClosePairs
@@ -2049,6 +2098,6 @@ fn segments(
 }
 
 /// A key of the screen, as the keymap writes it for the look.
-fn key(ui: &Ui, look: &Look, command: Command) -> crate::keymap::Written {
+pub(super) fn key(ui: &Ui, look: &Look, command: Command) -> crate::keymap::Written {
     crate::ui::keys::written(ui.ctx(), look, command)
 }
```

`src/ui/settings/updates.rs`:

```diff
diff --git a/src/ui/settings/updates.rs b/src/ui/settings/updates.rs
new file mode 100644
index 00000000..dcf59d03
--- /dev/null
+++ b/src/ui/settings/updates.rs
@@ -0,0 +1,645 @@
+//! Settings › Updates: the status card that says where updates stand,
+//! how often a check runs, and what the check sends. What the card says
+//! is worked out here for both looks (`Card`); the sheet's drawing is
+//! here too, the terminal look's in `updates_terminal.rs`.
+
+use std::time::{Duration, SystemTime};
+
+use egui::accesskit::{Live, Role};
+use egui::{CornerRadius, Rect, Sense, Stroke, StrokeKind, Ui, pos2, vec2};
+
+use crate::app::App;
+use crate::clock::Day;
+use crate::i18n::{Locale, gettext};
+use crate::model::Action;
+use crate::settings::OptionId;
+use crate::theme::Icon;
+use crate::typography::{Laid, Text, TextRole};
+use crate::ui::widgets::{self, ButtonSpec};
+use crate::update::{Failed, Installed, Managed, Offer, Phase, release};
+
+use super::sheet::Skin;
+
+/// Where updates stand, one state at a time, as the status card shows it.
+#[derive(Clone, Copy, Debug)]
+pub(super) enum Card<'a> {
+    /// No check has worked yet, and none runs.
+    NotChecked,
+    Checking,
+    UpToDate {
+        checked: u64,
+    },
+    /// A newer version this copy can take.
+    Available(&'a Offer),
+    /// A newer version with no download for this computer.
+    NoBuild(&'a Offer),
+    /// A newer version that something else installs.
+    Elsewhere(&'a Offer, &'a Managed),
+    Failed {
+        failed: &'a Failed,
+        last: Option<u64>,
+    },
+}
+
+impl<'a> Card<'a> {
+    pub(super) fn of(app: &'a App) -> Self {
+        let updates = &app.updates;
+        if updates.checking() {
+            return Self::Checking;
+        }
+        let last = updates.seen.last_checked;
+        if let Phase::Failed {
+            failed,
+            asked_for: true,
+        } = &updates.phase
+        {
+            return Self::Failed { failed, last };
+        }
+        if let Some(offer) = updates.offer(app.settings.update_skip.as_deref()) {
+            return match &updates.installed {
+                Some(Installed::Elsewhere(managed)) => Self::Elsewhere(offer, managed),
+                _ if offer.download.is_none() => Self::NoBuild(offer),
+                Some(Installed::Itself(_)) | None => Self::Available(offer),
+            };
+        }
+        match (&updates.phase, last) {
+            (Phase::Failed { failed, .. }, last) => Self::Failed { failed, last },
+            (Phase::Idle | Phase::Checking { .. }, Some(checked)) => Self::UpToDate { checked },
+            (Phase::Idle | Phase::Checking { .. }, None) => Self::NotChecked,
+        }
+    }
+
+    /// The card's title, translated: what a screen reader is told as it
+    /// changes.
+    pub(super) fn title(self, locale: Locale) -> String {
+        let current = crate::update::CURRENT;
+        let (words, version) = match self {
+            Self::NotChecked => ("Tabletist {version}", current),
+            Self::Checking => ("Checking for updates…", current),
+            Self::UpToDate { .. } => ("Tabletist {version} is up to date", current),
+            Self::Available(offer) | Self::NoBuild(offer) => {
+                ("Tabletist {version} is available", offer.version.as_str())
+            }
+            Self::Elsewhere(offer, _) => ("{version} is available", offer.version.as_str()),
+            Self::Failed { .. } => ("Couldn't check for updates", current),
+        };
+        gettext(locale, words).replace("{version}", version)
+    }
+}
+
+/// Whether a newer version is offered, whatever the card says while a
+/// check runs or after one failed: the dot on Updates in the nav.
+pub(super) fn offered(app: &App) -> bool {
+    app.updates
+        .offer(app.settings.update_skip.as_deref())
+        .is_some()
+}
+
+/// The time `at` (seconds since the Unix epoch) as the card says it, by
+/// the clock as it is `now`: `today at 09:12`, `yesterday at 09:05`, `on
+/// Oct 21 at 09:05`. Translated.
+pub(super) fn when(locale: Locale, at: u64, now: u64) -> String {
+    let (at, now) = (time_of(at), time_of(now));
+    let hour = crate::clock::hour(at);
+    let (words, day) = match crate::clock::day_of(at, now) {
+        Day::Today => ("today at {time}", String::new()),
+        Day::Yesterday => ("yesterday at {time}", String::new()),
+        Day::On(day) => ("on {day} at {time}", day),
+    };
+    gettext(locale, words)
+        .replace("{time}", &hour)
+        .replace("{day}", &day)
+}
+
+/// The day a release came out, as the card says it: `released today`.
+pub(super) fn released(locale: Locale, at: u64, now: u64) -> String {
+    let (words, day) = match crate::clock::day_of(time_of(at), time_of(now)) {
+        Day::Today => ("released today", String::new()),
+        Day::Yesterday => ("released yesterday", String::new()),
+        Day::On(day) => ("released {day}", day),
+    };
+    gettext(locale, words).replace("{day}", &day)
+}
+
+fn time_of(secs: u64) -> SystemTime {
+    SystemTime::UNIX_EPOCH + Duration::from_secs(secs)
+}
+
+/// When the last check that worked ran, as the row under the card says
+/// it: `Last checked today at 09:12`, or in the terminal look `last
+/// checked 09:12`. None before any has.
+pub(crate) fn checked_text(app: &App, terminal: bool) -> Option<String> {
+    let at = app.updates.seen.last_checked?;
+    let now = app.now_secs();
+    if terminal {
+        let at = time_of(at);
+        let time = match crate::clock::day_of(at, time_of(now)) {
+            Day::Today => crate::clock::hour(at),
+            Day::Yesterday | Day::On(_) => crate::clock::day(at),
+        };
+        let said = gettext(app.locale, "last checked {time}").replace("{time}", &time);
+        return Some(app.look.label(&said));
+    }
+    Some(gettext(app.locale, "Last checked {when}").replace("{when}", &when(app.locale, at, now)))
+}
+
+/// What the pane says under its title: on Windows no release is
+/// notarized.
+const ABOUT: &str = if cfg!(windows) {
+    "New versions come from Tabletist's GitHub releases, signed. Nothing installs without asking."
+} else {
+    "New versions come from Tabletist's GitHub releases, signed and notarized. Nothing installs without asking."
+};
+
+/// What the check sends, word for word as the pane says it: on Windows
+/// the User-Agent names the system, not its version.
+pub(super) const PRIVACY: &str = if cfg!(windows) {
+    "The check asks GitHub for the latest release. It sends this version, Windows and the processor, nothing else: no ID, no usage data, no database details."
+} else {
+    "The check asks GitHub for the latest release. It sends this version, the macOS version and the chip, nothing else: no ID, no usage data, no database details."
+};
+
+/// The space inside the card, at its sides and over and under.
+const PAD: f32 = 16.0;
+/// The icon's tile, and the gap after it.
+const TILE: f32 = 40.0;
+const TILE_GAP: f32 = 14.0;
+/// Between the card's lines.
+const LINE_GAP: f32 = 6.0;
+
+/// The pane, from the top of `ui` down.
+pub(super) fn content(ui: &mut Ui, app: &App, limit: f32, skin: &Skin, actions: &mut Vec<Action>) {
+    let Skin { look, palette, .. } = *skin;
+    let width = ui.available_width();
+    widgets::label(
+        ui,
+        TextRole::DialogTitle,
+        &skin.say("Updates"),
+        palette.text,
+        look,
+    );
+    ui.add_space(2.0);
+    Text::one(look, TextRole::UiBody, &skin.say(ABOUT), palette.dim)
+        .wrap(width)
+        .layout(ui.ctx())
+        .label(ui);
+    ui.add_space(10.0);
+    let room = (limit - ui.cursor().top()).max(56.0);
+    egui::ScrollArea::vertical()
+        .id_salt("updates")
+        .max_height(room)
+        .auto_shrink([false, true])
+        .show(ui, |ui| {
+            card(ui, app, skin, actions);
+            ui.add_space(14.0);
+            widgets::section_label(&skin.say("Automatic"), look, palette)
+                .layout(ui.ctx())
+                .label(ui);
+            ui.add_space(2.0);
+            check_row(ui, app, skin, actions);
+            ui.add_space(18.0);
+            tip(ui, skin);
+        });
+}
+
+/// The status card: an icon, the title, what it says under it, and its
+/// actions. A region named Update status, whose title a screen reader is
+/// told of as it changes, and an error at once.
+fn card(ui: &mut Ui, app: &App, skin: &Skin, actions: &mut Vec<Action>) {
+    let Skin {
+        look,
+        palette,
+        locale,
+        ..
+    } = *skin;
+    let shown = Card::of(app);
+    let now = app.now_secs();
+    let width = ui.available_width();
+    let words = (width - 2.0 * PAD - TILE - TILE_GAP).max(0.0);
+    let title = shown.title(locale);
+    // What it says under the title, then its notes or its box.
+    let sub = match shown {
+        Card::NotChecked => skin.say("Not checked yet"),
+        Card::Checking => crate::update::check::ASKED.to_owned(),
+        Card::UpToDate { checked } => {
+            gettext(locale, "Checked {when}").replace("{when}", &when(locale, checked, now))
+        }
+        Card::Available(offer) | Card::NoBuild(offer) => {
+            let mut parts = vec![
+                gettext(locale, "You have {version}").replace("{version}", crate::update::CURRENT),
+            ];
+            if let Some(at) = offer.published {
+                parts.push(released(locale, at, now));
+            }
+            if let Some((_, size)) = offer.download {
+                parts.push(release::megabytes(size));
+            }
+            parts.join(" · ")
+        }
+        Card::Elsewhere(_, managed) => skin.say(managed.said),
+        Card::Failed { last, .. } => match last {
+            Some(at) => gettext(locale, "Last successful check: {when}")
+                .replace("{when}", &when(locale, at, now)),
+            None => skin.say("No check has worked yet"),
+        },
+    };
+    let title_laid = Text::one(look, TextRole::StateTitle, &title, palette.text)
+        .wrap(words)
+        .layout(ui.ctx());
+    let sub_laid = Text::one(look, TextRole::Secondary, &sub, palette.dim)
+        .wrap(words)
+        .layout(ui.ctx());
+    let notes: Vec<Laid> = match shown {
+        Card::Available(offer) => offer
+            .notes
+            .iter()
+            .map(|point| note(point, words - 18.0, skin).layout(ui.ctx()))
+            .collect(),
+        _ => Vec::new(),
+    };
+    // The box under the line: an error, the release with no build, or the
+    // command another tool updates with.
+    let boxed = match shown {
+        Card::Failed { failed, .. } => {
+            let said = failed.with_numbers(&gettext(locale, failed.said()));
+            Some((said, palette.danger))
+        }
+        Card::NoBuild(offer) => Some((
+            gettext(locale, "{version} has no build for this computer yet")
+                .replace("{version}", &offer.version),
+            palette.warning,
+        )),
+        _ => None,
+    };
+    let boxed_laid = boxed.as_ref().map(|(said, color)| {
+        Text::one(look, TextRole::Secondary, said, *color)
+            .wrap((words - 20.0).max(0.0))
+            .layout(ui.ctx())
+    });
+    let command = match shown {
+        Card::Elsewhere(_, managed) => managed.command,
+        _ => None,
+    };
+    let line = TextRole::Secondary.row_height(ui.ctx(), look.faces);
+    let mut inner = title_laid.height() + LINE_GAP + sub_laid.height();
+    for laid in &notes {
+        inner += 2.0 + laid.height();
+    }
+    if let Some(laid) = &boxed_laid {
+        inner += LINE_GAP + laid.height() + 16.0;
+    }
+    if command.is_some() {
+        inner += LINE_GAP + look.control_height;
+    }
+    let has_actions = command.is_none();
+    if has_actions {
+        inner += LINE_GAP + 4.0 + look.control_height;
+    }
+    let height = (inner.max(TILE) + 2.0 * PAD).ceil();
+    let (rect, _) = ui.allocate_exact_size(vec2(width, height), Sense::hover());
+    let corner = CornerRadius::same(look.radius.saturating_add(4));
+    ui.painter().rect_filled(rect, corner, palette.window);
+    ui.painter().rect_stroke(
+        rect,
+        corner,
+        Stroke::new(1.0, palette.surface),
+        StrokeKind::Inside,
+    );
+    // The region, named; what is in it is its own.
+    let region_id = ui.id().with("update-status");
+    let region = ui.interact(rect, region_id, Sense::hover());
+    let named = skin.say("Update status");
+    region.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Other, true, &named));
+    ui.ctx().accesskit_node_builder(region_id, |node| {
+        node.set_role(Role::Region);
+    });
+    let inside = egui::UiBuilder::new()
+        .accessibility_parent(region_id)
+        .max_rect(rect);
+    let mut inside = ui.new_child(inside);
+    let ui = &mut inside;
+
+    // The icon in its tile.
+    let tile = Rect::from_min_size(pos2(rect.left() + PAD, rect.top() + PAD), vec2(TILE, TILE));
+    let (icon, tint) = match shown {
+        Card::NotChecked => (Some(Icon::Info), palette.secondary),
+        Card::Checking => (None, palette.dim),
+        Card::UpToDate { .. } => (Some(Icon::CircleCheck), palette.success),
+        Card::Available(_) | Card::NoBuild(_) => (Some(Icon::Download), palette.accent),
+        Card::Elsewhere(..) => (Some(Icon::Download), palette.secondary),
+        Card::Failed { .. } => (Some(Icon::CircleAlert), palette.danger),
+    };
+    let fill = match shown {
+        Card::Checking | Card::Elsewhere(..) | Card::NotChecked => palette.surface,
+        _ => tint.gamma_multiply(0.14),
+    };
+    ui.painter().rect_filled(tile, CornerRadius::same(10), fill);
+    match icon {
+        Some(icon) => icon
+            .image(tint, 22.0)
+            .paint_at(ui, Rect::from_center_size(tile.center(), vec2(22.0, 22.0))),
+        None => crate::ui::states::spinner(
+            ui,
+            Rect::from_center_size(tile.center(), vec2(16.0, 16.0)),
+            palette.accent,
+            palette,
+        ),
+    }
+
+    let left = tile.right() + TILE_GAP;
+    let mut y = rect.top() + PAD;
+    title_laid.paint(ui.painter(), pos2(left, y));
+    // Told politely as it changes; an error is told by its own line.
+    announce_live(
+        ui,
+        Rect::from_min_size(pos2(left, y), title_laid.size()),
+        (&title, "title"),
+        Live::Polite,
+    );
+    y += title_laid.height() + LINE_GAP;
+    paint(ui, &sub_laid, pos2(left, y));
+    y += sub_laid.height();
+    if !notes.is_empty() {
+        y += 2.0;
+        let list_id = ui.id().with("update-notes");
+        let list_rect = Rect::from_min_size(
+            pos2(left, y),
+            vec2(words, notes.iter().map(|laid| laid.height() + 2.0).sum()),
+        );
+        let list = ui.interact(list_rect, list_id, Sense::hover());
+        let named = skin.say("What's new");
+        list.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Other, true, &named));
+        ui.ctx().accesskit_node_builder(list_id, |node| {
+            node.set_role(Role::List);
+        });
+        for (index, laid) in notes.iter().enumerate() {
+            let dot = pos2(left + 6.0, y + laid.height().min(line) / 2.0);
+            ui.painter().circle_filled(dot, 2.0, palette.secondary);
+            laid.paint(ui.painter(), pos2(left + 18.0, y));
+            let item = ui.interact(
+                Rect::from_min_size(pos2(left + 18.0, y), laid.size()),
+                list_id.with(index),
+                Sense::hover(),
+            );
+            let text = laid.galley.text().to_owned();
+            item.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Label, true, &text));
+            ui.ctx().accesskit_node_builder(item.id, |node| {
+                node.set_role(Role::ListItem);
+            });
+            y += laid.height() + 2.0;
+        }
+    }
+    if let Some(laid) = &boxed_laid {
+        y += LINE_GAP;
+        let color = boxed.as_ref().map_or(palette.danger, |(_, color)| *color);
+        let band = Rect::from_min_size(pos2(left, y), vec2(words, laid.height() + 16.0));
+        ui.painter()
+            .rect_filled(band, CornerRadius::same(8), color.gamma_multiply(0.1));
+        ui.painter().rect_stroke(
+            band,
+            CornerRadius::same(8),
+            Stroke::new(1.0, color.gamma_multiply(0.35)),
+            StrokeKind::Inside,
+        );
+        laid.paint(ui.painter(), pos2(left + 10.0, y + 8.0));
+        // Told once, at once, where it is an error.
+        let said = laid.galley.text().to_owned();
+        match shown {
+            Card::Failed { .. } => announce_live(ui, band, (&said, "error"), Live::Assertive),
+            _ => widgets::announce(ui, band, &said),
+        }
+        y = band.bottom();
+    }
+    if let Some(command) = command {
+        y += LINE_GAP;
+        command_box(
+            ui,
+            Rect::from_min_size(pos2(left, y), vec2(words, look.control_height)),
+            command,
+            skin,
+        );
+    }
+    if has_actions {
+        y += LINE_GAP + 4.0;
+        let place = Rect::from_min_size(pos2(left, y), vec2(words, look.control_height));
+        let centred = egui::Layout::left_to_right(egui::Align::Center);
+        let builder = egui::UiBuilder::new()
+            .id_salt("update-actions")
+            .max_rect(place);
+        let mut row = ui.new_child(builder.layout(centred));
+        row.spacing_mut().item_spacing.x = 8.0;
+        card_actions(&mut row, shown, skin, actions);
+    }
+}
+
+/// A point of the notes, its strong runs strong.
+fn note(point: &str, width: f32, skin: &Skin) -> Text {
+    let Skin { look, palette, .. } = *skin;
+    let mut text = Text::new(look);
+    for (run, strong) in release::runs(point) {
+        let role = if strong {
+            TextRole::UiBodyStrong
+        } else {
+            TextRole::UiBody
+        };
+        let color = if strong {
+            palette.text
+        } else {
+            palette.secondary
+        };
+        text = text.add(role, &run, color);
+    }
+    text.wrap(width.max(0.0))
+}
+
+/// The card's buttons and links, by its state.
+fn card_actions(ui: &mut Ui, shown: Card<'_>, skin: &Skin, actions: &mut Vec<Action>) {
+    let Skin {
+        look,
+        palette,
+        locale,
+        ..
+    } = *skin;
+    let height = look.control_height;
+    let current = crate::update::CURRENT;
+    let whats_new = gettext(locale, "What's new in {version}").replace("{version}", current);
+    let on_github = skin.say("Release notes on GitHub");
+    let open = |ui: &Ui, url: &str| ui.ctx().open_url(egui::OpenUrl::new_tab(url));
+    match shown {
+        Card::NotChecked | Card::UpToDate { .. } => {
+            if ButtonSpec::new(&skin.say("Check now"))
+                .show(ui, height, look, palette)
+                .clicked()
+            {
+                actions.push(Action::CheckForUpdates);
+            }
+            if ButtonSpec::new(&whats_new)
+                .link()
+                .show(ui, height, look, palette)
+                .clicked()
+            {
+                open(ui, &crate::update::this_release());
+            }
+        }
+        Card::Checking => {
+            if ButtonSpec::new(&skin.say("Cancel"))
+                .show(ui, height, look, palette)
+                .clicked()
+            {
+                actions.push(Action::CancelUpdateCheck);
+            }
+        }
+        Card::Available(offer) | Card::NoBuild(offer) => {
+            // Downloading comes with the next step: the page has the
+            // download until then.
+            let notes = ButtonSpec::new(&on_github);
+            let notes = if matches!(shown, Card::Available(_)) {
+                notes.primary()
+            } else {
+                notes.link()
+            };
+            if notes.show(ui, height, look, palette).clicked() {
+                open(ui, &offer.url);
+            }
+            let skip = skin.say("Skip this version");
+            if ButtonSpec::new(&skip)
+                .link()
+                .show(ui, height, look, palette)
+                .clicked()
+            {
+                actions.push(Action::SkipUpdate);
+            }
+        }
+        // The command and its Copy stand in the actions' place.
+        Card::Elsewhere(offer, _) => {
+            if ButtonSpec::new(&on_github)
+                .link()
+                .show(ui, height, look, palette)
+                .clicked()
+            {
+                open(ui, &offer.url);
+            }
+        }
+        Card::Failed { .. } => {
+            if ButtonSpec::new(&skin.say("Try again"))
+                .show(ui, height, look, palette)
+                .clicked()
+            {
+                actions.push(Action::CheckForUpdates);
+            }
+        }
+    }
+}
+
+/// The command another tool updates with, in the code's face, with Copy
+/// at its right.
+fn command_box(ui: &mut Ui, rect: Rect, command: &str, skin: &Skin) {
+    let Skin {
+        look,
+        palette,
+        locale,
+        ..
+    } = *skin;
+    let corner = CornerRadius::same(look.radius);
+    ui.painter().rect_filled(rect, corner, palette.panel);
+    ui.painter().rect_stroke(
+        rect,
+        corner,
+        Stroke::new(1.0, palette.surface),
+        StrokeKind::Inside,
+    );
+    let mono = TextRole::MonoSecondary;
+    widgets::paint_text(
+        ui,
+        rect.left() + 10.0,
+        rect.center().y,
+        Text::one(look, mono, command, palette.text),
+    );
+    widgets::announce(ui, rect, command);
+    let label = gettext(locale, "Copy");
+    let copy = ButtonSpec::new(&label).link();
+    let width = copy.width(ui, look);
+    let at = Rect::from_min_size(
+        pos2(rect.right() - width - 4.0, rect.top()),
+        vec2(width, rect.height()),
+    );
+    if copy.show_at(ui, at, look, palette).clicked() {
+        copy_command(ui.ctx(), command, locale);
+    }
+}
+
+/// Puts the command that updates this copy on the clipboard, and says so.
+pub(super) fn copy_command(ctx: &egui::Context, command: &str, locale: Locale) {
+    ctx.copy_text(command.to_owned());
+    crate::ui::toast::say(ctx, &gettext(locale, "Copied the command"));
+}
+
+/// The command another tool updates this copy with, where it has one.
+pub(super) fn managed_command(app: &App) -> Option<&'static str> {
+    match Card::of(app) {
+        Card::Elsewhere(_, managed) => managed.command,
+        _ => None,
+    }
+}
+
+/// The row of how often a check runs: its label over when the last check
+/// ran, with Check now; the menu; and the other choices.
+fn check_row(ui: &mut Ui, app: &App, skin: &Skin, actions: &mut Vec<Action>) {
+    let settings = super::shown(app);
+    let column = super::sheet::label_column(ui, &OptionId::UPDATES, skin);
+    let print = checked_text(app, false).unwrap_or_else(|| skin.say("Not checked yet"));
+    super::sheet::row_with(
+        ui,
+        (OptionId::UpdatesCheck, false, column),
+        &settings,
+        skin,
+        actions,
+        super::sheet::Print {
+            text: &print,
+            link: Some((&skin.say("Check now"), Action::CheckForUpdates)),
+        },
+    );
+}
+
+/// What the check sends, in a box of its own.
+fn tip(ui: &mut Ui, skin: &Skin) {
+    let Skin { look, palette, .. } = *skin;
+    let width = ui.available_width();
+    let said = Text::one(
+        look,
+        TextRole::Secondary,
+        &skin.say(PRIVACY),
+        palette.secondary,
+    )
+    .wrap((width - 24.0).max(0.0))
+    .layout(ui.ctx());
+    let (rect, _) = ui.allocate_exact_size(vec2(width, said.height() + 20.0), Sense::hover());
+    let corner = CornerRadius::same(8);
+    ui.painter().rect_filled(rect, corner, palette.panel);
+    ui.painter().rect_stroke(
+        rect,
+        corner,
+        Stroke::new(1.0, palette.outline),
+        StrokeKind::Inside,
+    );
+    paint(ui, &said, pos2(rect.left() + 12.0, rect.top() + 10.0));
+}
+
+/// Paints `text` with its top left at `at`, and names it for screen
+/// readers.
+fn paint(ui: &Ui, text: &Laid, at: egui::Pos2) {
+    text.paint(ui.painter(), at);
+    widgets::announce(ui, Rect::from_min_size(at, text.size()), text.galley.text());
+}
+
+/// A label node over `rect` that a screen reader reads out as it changes:
+/// one node for `which` of the card's lines whatever it says, so that a
+/// change is a change and not a new node.
+pub(super) fn announce_live(ui: &Ui, rect: Rect, (text, which): (&str, &str), live: Live) {
+    let id = ui.id().with(("announce-live", which));
+    let response = ui.interact(rect, id, Sense::hover());
+    response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Label, true, text));
+    ui.ctx().accesskit_node_builder(response.id, |node| {
+        node.set_live(live);
+    });
+}
```

`src/ui/settings/updates_terminal.rs`:

```diff
diff --git a/src/ui/settings/updates_terminal.rs b/src/ui/settings/updates_terminal.rs
new file mode 100644
index 00000000..4f906fcc
--- /dev/null
+++ b/src/ui/settings/updates_terminal.rs
@@ -0,0 +1,321 @@
+//! Settings › Updates as the terminal look draws it: the status block
+//! with its keys, the row of how often a check runs, and what a package
+//! manager's copy does, with what the check sends.
+
+use egui::accesskit::Live;
+use egui::{CornerRadius, Rect, Sense, Stroke, StrokeKind, Ui, pos2, vec2};
+
+use crate::app::App;
+use crate::i18n::gettext;
+use crate::keymap::Command;
+use crate::model::Action;
+use crate::settings::{OptionId, Settings};
+use crate::typography::{Text, TextRole};
+use crate::ui::widgets;
+use crate::update::release;
+
+use super::terminal::{ROW_SIDE, Skin, columns, heading, key, option_row, widths};
+use super::updates::{Card, announce_live, released, when};
+
+/// What the note under the row says, word for word as the board has it.
+const NOTE: &str = "installed with a package manager (pacman, an AUR helper, nix, cargo install)? then that tool updates tabletist: this pane only checks and shows the command, for example";
+const NOTE_COMMAND: &str = "cargo install tabletist --locked";
+const PRIVACY: &str = "the check asks github for the latest release and sends this version, os and cpu, nothing else.";
+
+/// The pane: the block, then its one row under `automatic`, then the
+/// note. The cursor's row is the pane's only one.
+pub(super) fn rows(
+    ui: &mut Ui,
+    app: &App,
+    settings: &Settings,
+    cursor: usize,
+    skin: &Skin,
+    actions: &mut Vec<Action>,
+) {
+    block(ui, app, skin);
+    heading(ui, "Automatic", 8.0, skin);
+    let room = ui.available_width() - 2.0 * ROW_SIDE;
+    let columns = columns(
+        ui,
+        room,
+        widths(ui, &OptionId::UPDATES, settings, skin),
+        skin,
+    );
+    option_row(
+        ui,
+        (0, OptionId::UpdatesCheck),
+        cursor == 0,
+        columns,
+        settings,
+        skin,
+        actions,
+    );
+    ui.add_space(14.0);
+    note(ui, skin);
+}
+
+/// One line of the block: its pieces, each its text and colour.
+type Line = Vec<(String, egui::Color32)>;
+
+/// The status block: what the card of the other looks says, a line at a
+/// time, with the pane's keys that act on it.
+fn block(ui: &mut Ui, app: &App, skin: &Skin) {
+    let Skin {
+        look,
+        palette,
+        locale,
+        ..
+    } = *skin;
+    let shown = Card::of(app);
+    let now = app.now_secs();
+    let current = crate::update::CURRENT;
+    // As the board writes it here: `0.4.0 available`.
+    let title = match shown {
+        Card::Available(offer) | Card::NoBuild(offer) => {
+            gettext(locale, "{version} available").replace("{version}", &offer.version)
+        }
+        _ => look.label(&shown.title(locale)),
+    };
+    let title_color = match shown {
+        Card::Failed { .. } => palette.danger,
+        Card::UpToDate { .. } => palette.success,
+        _ => palette.accent,
+    };
+    let say = |text: &'static str| skin.say(text);
+    let mut sub: Vec<String> = Vec::new();
+    let mut lines: Vec<Line> = Vec::new();
+    match shown {
+        Card::NotChecked => sub.push(say("not checked yet")),
+        Card::Checking => sub.push(crate::update::check::ASKED.to_owned()),
+        Card::UpToDate { checked } => sub.push(look.label(
+            &gettext(locale, "Checked {when}").replace("{when}", &when(locale, checked, now)),
+        )),
+        Card::Available(offer) | Card::NoBuild(offer) => {
+            sub.push(
+                look.label(&gettext(locale, "You have {version}").replace("{version}", current)),
+            );
+            if let Some(at) = offer.published {
+                sub.push(look.label(&released(locale, at, now)));
+            }
+            if let Some((_, size)) = offer.download {
+                sub.push(look.label(&release::megabytes(size)));
+            }
+            sub.push(format!(
+                "{} {}",
+                std::env::consts::OS,
+                std::env::consts::ARCH
+            ));
+            for point in &offer.notes {
+                let plain: String = release::runs(point)
+                    .into_iter()
+                    .map(|(run, _)| run)
+                    .collect();
+                lines.push(vec![(format!("  - {}", look.label(&plain)), palette.text)]);
+            }
+            if matches!(shown, Card::NoBuild(_)) {
+                let said = gettext(locale, "{version} has no build for this computer yet")
+                    .replace("{version}", &offer.version);
+                lines.push(vec![(look.label(&said), palette.warning)]);
+            }
+        }
+        Card::Elsewhere(_, managed) => {
+            sub.push(say(managed.said));
+            if let Some(command) = managed.command {
+                lines.push(vec![(format!("  {command}"), palette.text)]);
+            }
+        }
+        Card::Failed { failed, last } => {
+            sub.push(match last {
+                Some(at) => look.label(
+                    &gettext(locale, "Last successful check: {when}")
+                        .replace("{when}", &when(locale, at, now)),
+                ),
+                None => say("no check has worked yet"),
+            });
+            let said = failed.with_numbers(&gettext(locale, failed.said()));
+            lines.push(vec![(look.label(&said), palette.danger)]);
+        }
+    }
+    // The pane's keys that act on what the block shows.
+    let keys: Vec<(Command, String)> = match shown {
+        Card::NotChecked | Card::UpToDate { .. } => {
+            vec![(Command::SettingsCheckNow, say("check now"))]
+        }
+        Card::Failed { .. } => vec![(Command::SettingsCheckNow, say("try again"))],
+        Card::Checking => Vec::new(),
+        Card::Available(offer) | Card::NoBuild(offer) => vec![
+            (Command::SettingsReleaseNotes, say("release notes")),
+            (
+                Command::SettingsSkipVersion,
+                format!("{} {}", say("skip"), offer.version),
+            ),
+        ],
+        Card::Elsewhere(_, managed) => match managed.command {
+            Some(_) => vec![
+                (Command::SettingsCopyCommand, say("copy")),
+                (Command::SettingsReleaseNotes, say("release notes")),
+            ],
+            None => vec![(Command::SettingsReleaseNotes, say("release notes"))],
+        },
+    };
+    let mut keyed: Line = Vec::new();
+    for (index, (command, what)) in keys.iter().enumerate() {
+        if index > 0 {
+            keyed.push(("  ".to_owned(), palette.dim));
+        }
+        keyed.push((key(ui, look, *command).as_str().to_owned(), palette.accent));
+        keyed.push((format!(" {what}"), palette.text));
+    }
+
+    let role = TextRole::OBody;
+    let line = (role.row_height(ui.ctx(), look.faces) * 1.35).round();
+    let count = 1 + lines.len() + usize::from(!keyed.is_empty());
+    let height = count as f32 * line + 20.0 + if keyed.is_empty() { 0.0 } else { 6.0 };
+    let (rect, _) =
+        ui.allocate_exact_size(vec2(ui.available_width(), height + 14.0), Sense::hover());
+    let band = Rect::from_min_max(
+        pos2(rect.left() + ROW_SIDE, rect.top() + 8.0),
+        pos2(
+            (rect.right() - ROW_SIDE).max(rect.left() + ROW_SIDE),
+            rect.bottom() - 6.0,
+        ),
+    );
+    ui.painter()
+        .rect_filled(band, CornerRadius::ZERO, palette.panel);
+    ui.painter().rect_stroke(
+        band,
+        CornerRadius::ZERO,
+        Stroke::new(1.0, palette.accent),
+        StrokeKind::Inside,
+    );
+    // The region, named; its lines are its own.
+    let region_id = ui.id().with("update-status");
+    let region = ui.interact(band, region_id, Sense::hover());
+    let named = skin.say("Update status");
+    region.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Other, true, &named));
+    ui.ctx().accesskit_node_builder(region_id, |node| {
+        node.set_role(egui::accesskit::Role::Region);
+    });
+    let inside = egui::UiBuilder::new()
+        .accessibility_parent(region_id)
+        .max_rect(band);
+    let mut inside = ui.new_child(inside);
+    let ui = &mut inside;
+    let left = band.left() + 16.0;
+    let right = band.right() - 16.0;
+    let mut y = band.top() + 10.0 + line / 2.0;
+    // A line is cut at the block's edge, and read whole.
+    let ctx = ui.ctx().clone();
+    let cut = |text: &str, room: f32| {
+        crate::ui::grid::ellipsize(text, room.max(0.0), false, |text| {
+            role.width(&ctx, look.faces, text)
+        })
+        .into_owned()
+    };
+    // The title, strong, and what it says beside it.
+    let strong = Text::one(look, TextRole::OBody, &title, title_color).layout(ui.ctx());
+    strong.paint_left(ui.painter(), left, y);
+    // Told politely as it changes; an error is told by its own line.
+    announce_live(
+        ui,
+        Rect::from_center_size(pos2(left + strong.width() / 2.0, y), strong.size()),
+        (&title, "title"),
+        Live::Polite,
+    );
+    if !sub.is_empty() {
+        let said = format!("  {}", sub.join(" · "));
+        let shown_sub = cut(&said, right - left - strong.width());
+        let width = widgets::paint_text(
+            ui,
+            left + strong.width(),
+            y,
+            Text::one(look, role, &shown_sub, palette.dim),
+        );
+        widgets::announce(
+            ui,
+            Rect::from_min_size(
+                pos2(left + strong.width(), y - line / 2.0),
+                vec2(width.max(1.0), line),
+            ),
+            said.trim(),
+        );
+    }
+    for pieces in lines.iter().chain((!keyed.is_empty()).then_some(&keyed)) {
+        y += line;
+        if std::ptr::eq(pieces, &keyed) {
+            y += 6.0;
+        }
+        let mut text = Text::new(look);
+        for (piece, color) in pieces {
+            // A line of one piece is a note or what is said: cut to fit.
+            let piece = match pieces.len() {
+                1 => cut(piece, right - left),
+                _ => piece.clone(),
+            };
+            text = text.add(role, &piece, *color);
+        }
+        let laid = text.layout(ui.ctx());
+        laid.paint_left(ui.painter(), left, y);
+        let said: String = pieces.iter().map(|(piece, _)| piece.as_str()).collect();
+        let place = Rect::from_center_size(
+            pos2(left + laid.width() / 2.0, y),
+            vec2(laid.width().max(1.0), line),
+        );
+        let error = matches!(shown, Card::Failed { .. })
+            && pieces
+                .first()
+                .is_some_and(|(_, color)| *color == palette.danger);
+        if error {
+            announce_live(ui, place, (said.trim(), "error"), Live::Assertive);
+        } else {
+            widgets::announce(ui, place, said.trim());
+        }
+    }
+}
+
+/// What a package manager's copy does, and what the check sends, in a
+/// box under the row.
+fn note(ui: &mut Ui, skin: &Skin) {
+    let Skin { look, palette, .. } = *skin;
+    let role = TextRole::OBody;
+    let width = (ui.available_width() - 2.0 * ROW_SIDE - 24.0).max(0.0);
+    let first = Text::new(look)
+        .add(role, &skin.say(NOTE), palette.dim)
+        .space(role, " ")
+        .add(role, NOTE_COMMAND, palette.text)
+        .add(role, ".", palette.dim)
+        .wrap(width)
+        .layout(ui.ctx());
+    let second = Text::one(look, role, &skin.say(PRIVACY), palette.dim)
+        .wrap(width)
+        .layout(ui.ctx());
+    let height = first.height() + second.height() + 20.0;
+    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), height), Sense::hover());
+    let band = Rect::from_min_max(
+        pos2(rect.left() + ROW_SIDE, rect.top()),
+        pos2(
+            (rect.right() - ROW_SIDE).max(rect.left() + ROW_SIDE),
+            rect.bottom(),
+        ),
+    );
+    ui.painter().rect_stroke(
+        band,
+        CornerRadius::ZERO,
+        Stroke::new(1.0, palette.outline),
+        StrokeKind::Inside,
+    );
+    let at = pos2(band.left() + 12.0, band.top() + 10.0);
+    first.paint(ui.painter(), at);
+    widgets::announce(
+        ui,
+        Rect::from_min_size(at, first.size()),
+        first.galley.text(),
+    );
+    let at = pos2(at.x, at.y + first.height());
+    second.paint(ui.painter(), at);
+    widgets::announce(
+        ui,
+        Rect::from_min_size(at, second.size()),
+        second.galley.text(),
+    );
+}
```

- [ ] **Step 4: Run the checks**

Run: `cargo fmt --all --check && cargo clippy --locked --workspace --all-targets -- -D warnings && cargo clippy --locked --workspace --all-targets --features shots -- -D warnings && RUSTDOCFLAGS='-D warnings' cargo doc --locked --workspace --no-deps && cargo test --locked --workspace --all-targets`, then `cd docs && bin/check`.
Expected: clean; the library's tests `test result: ok. 3592 passed; 0 failed; 17 ignored`, `update_settings_tests` 18 of them; `tests/keys.rs` passes.

- [ ] **Step 5: Commit**

```bash
git add assets/icons/download.svg docs/_guide/download.md docs/_guide/macos.md docs/_guide/omarchy.md docs/_reference/settings-and-files.md src/app.rs src/app/commands.rs src/app/editing.rs src/app/menu.rs src/keymap.rs src/menu.rs src/model.rs src/settings.rs src/theme.rs src/ui/mod.rs src/ui/settings/mod.rs src/ui/settings/sheet.rs src/ui/settings/terminal.rs src/ui/settings/updates.rs src/ui/settings/updates_terminal.rs src/ui/update_settings_tests.rs
git commit -m "Show Settings › Updates in both looks, with Check for Updates… and :update"
```

---

## After Task 3

Push the branch and open the pull request. Its description has what the prompt's Delivery asks of this run: a summary; the check (the request, what it sends, the schedule, the 10 s limit, what a failure does); install detection and the commands shown; what the release workflow must add (nothing: the signed checksums and the asset names it already makes, decisions 1 and 2); the open questions and the answers chosen (1 to 4); verification, install and going back as runs 2 and 3, with step 1's Part B (the receipt) in this one; and the departures above. No screenshot.

## Checks by hand

Off the record, on the demo database (Bookshop) and in both looks: open Settings › Updates with no `updates.json` and "Only when I ask" (Not checked yet), press Check now (Checking, then most likely Up to date from the live GitHub), turn the network off and press it again (Couldn't check, Try again). For Available, quit and write an offer newer than this version into the state folder's `updates.json` (`{"last_checked": <now>, "offer": {"version": "9.9.9", "url": "https://github.com/igor-alexandrov/tabletist/releases", "notes": ["A point"], "body": "- A point", "download": ["x", 31200000]}}`): it is offered at launch. In the Omarchy look press `c`, `n` (a terminal with the notes), `S`, and `:update` from a SQL tab; the status line tells of 9.9.9 once. On macOS, Tabletist › Check for Updates…. Compare with the boards by eye: `macOS – Settings, Updates`, `… states` and `Omarchy – Settings, Updates`. Nothing of it is a test.
