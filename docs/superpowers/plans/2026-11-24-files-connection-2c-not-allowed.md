# Files connection 2c: the macOS "not allowed" state

> **For agentic workers:** REQUIRED SUB-SKILL: Use rubypowers:subagent-driven-development (recommended) or rubypowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** When macOS's privacy settings haven't let Tabletist open a Files connection's folder, the Test and the connection's window say so. They use the prompt's words ("Can't read ~/Data/bookshop-exports. macOS hasn't allowed Tabletist to open this folder.") and offer **Open Privacy & Security**, which opens System Settings at Privacy & Security › Files and Folders.

**Architecture:**
- **Driver** (`files.rs`): when macOS's privacy settings refuse a folder or file, they do it with `EPERM`. A file's own permissions refuse with `EACCES`. Only on macOS, the scan tells the two apart and says the refusal in its own words. Those words can be read back (`files::refused_by_macos`, `files::refused_file_by_macos`).
- **The scan opens each file it keeps:** a file's size can be read where its contents can't. That's true of a file's own permissions, and of macOS's privacy settings for a file in a folder they guard. So a file that can be looked at but not read is caught there, not later in DuckDB.
- **UI:** `src/ui/not_allowed.rs` turns that message into the state's words and button.
  - The connection dialog shows them as a danger banner in place of the failed Test's line, and its found line says them, naming the file where it is one of a list's.
  - A Test that reads past a file macOS refused (a list's file, or one in an allowed folder) keeps its partial banner. That banner now says macOS's words, translated, and puts Open Privacy & Security before Skip.
  - The window's connect-failure card takes them for its title and sentence, with the button first.
- **System Settings:** the button sends `Command::OpenPrivacySettings`. On macOS the backend runs `open` with System Settings' link: `open` hands it to System Settings, where a browser wouldn't.

**Tech Stack:** Rust, egui (`egui_kittest` headless tests), gettext catalogs (`assets/i18n`).

**Base:** `main` at `6c201bef`, branch `claude/files-connection-2c-not-allowed`. 2b-2 (#382) is merged.

---

## What was decided

On 2026-11-24 Igor picked these, with the recommended option each time:

| Question | Decision |
| --- | --- |
| Security-scoped bookmarks | **None.** Tabletist's macOS build isn't sandboxed (Developer ID, hardened runtime, no App Sandbox), so macOS keeps the Files and Folders grant itself, and a security-scoped bookmark would add no access. The PR says why; bookmarks come back if the app is ever sandboxed. |
| Opening the connection when macOS refuses | The connect-failure card has the same words and the same button, first and primary, beside Retry. |

Canvas: the dialog's banner is drawn in "macOS – Connection, Files, Test and list" ("Test · failure"). The connect card and a Test past a refused listed file in both looks, and the dialog in the Omarchy look, are drawn in **"macOS – Files, not allowed"** and **"Omarchy – Files, not allowed"** (canvas version 213).

### Where this plan does less than the prompt or the canvas

| The prompt or canvas | What is built | Why |
| --- | --- | --- |
| "On macOS, also store a security-scoped bookmark for the folder or each file, so access survives a restart. If the bookmark can't be resolved, show the 'not allowed' state." | No bookmarks. The "not allowed" state shows when macOS refuses the folder or a file. | Decided above: unsandboxed, macOS keeps the grant. |
| "Test · failure" draws the banner under the footer, beside the missing-folder state | The banner sits over the footer, where the partial Test's banner is, and the footer's status says nothing. | One place for the dialog's banners. The canvas board shows two states in one panel. |
| The refusal of a folder | The refusal of a listed file says "this file" ("macOS hasn't allowed Tabletist to open this file."). | A Files list can hold a file from a protected folder. |
| — | Only refusals the scan sees are recognized: listing the folder, and looking at and opening each file. A refusal that comes only later, in DuckDB's own read, shows DuckDB's error. | Can't be tested here: macOS's privacy settings only act on a Mac. A hand test on macOS checks it (see "After the tasks"). |
| "Open Privacy & Security" | The button keeps the prompt's words on every macOS version. The link opens the pane on each, but macOS 11 and 12 call it "Security & Privacy". | The prompt's words. The app's minimum is macOS 11. |
| System Settings › Privacy & Security › Files and Folders | The button always opens Files and Folders. A place that only Full Disk Access unlocks (such as `~/Library/Mail`) refuses with the same `EPERM`, and Tabletist isn't listed under Files and Folders for it. | The refusal doesn't say which list guards it. Full Disk Access sits one row away in the same pane. |
| "macOS hasn't allowed Tabletist" | A build run as a bare binary from a terminal (the Homebrew formula) is credited by macOS to the terminal app, not to Tabletist. | macOS's own attribution. The app bundle is credited to Tabletist. |
| — | A SQLite connection to a file in a guarded folder still shows the generic card. | Out of scope: the prompt asks for the Files type. |

The new words have no AI, sparkle or confidence wording, and the UI copy has no em dashes. They're translated into Russian and Brazilian Portuguese, using the names macOS gives the pane in each language ("Конфиденциальность и безопасность", "Privacidade e Segurança"); the Portuguese wants a native review.

## Files

| File | Change |
| --- | --- |
| `crates/tabletist-db/src/files.rs` | `Refused`, the refusal's words for `EPERM` on macOS (`not_allowed`), `refused_by_macos`, `refused_file_by_macos`; the scan opens each file it keeps (`readable`) |
| `crates/tabletist-db/tests/files_check.rs` | A listed file that can be looked at and not read is said by the scan |
| `crates/tabletist-core/src/backend.rs`, `src/promo/live.rs` | `Command::OpenPrivacySettings`, `open` on macOS |
| `crates/tabletist-core/src/model.rs`, `src/app.rs` | `Action::OpenPrivacySettings` |
| `src/ui/not_allowed.rs` (new), `src/ui/mod.rs` | The state's words, its banner and button |
| `src/ui/connect_dialog/mod.rs` | The banner in place of the failed Test's line (`Shown`) |
| `src/ui/connect_dialog/files.rs` | The found line in the banner's words |
| `src/ui/partial.rs` | The partial banner's words and Open Privacy & Security for a file macOS refused |
| `src/ui/banner.rs` | An action with no key starts where a key would, in the terminal look |
| `src/ui/workspace.rs` | The connect-failure card's words and first button |
| `src/ui/not_allowed_tests.rs` (new) | Headless tests of the dialog and the card |
| `assets/i18n/*.po`, `*.pot` | The four new messages |
| `docs/_reference/connections.md` | The state |

Every commit passes these:
- `cargo fmt --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- the headless tests
- `RUSTDOCFLAGS='-D warnings' cargo doc --workspace --no-deps`
- `sh assets/i18n/update.sh --check`

Run cargo as `~/.cargo/bin/cargo` with `CARGO_TARGET_DIR=$PWD/target`. For the server tests, point `TABLETIST_TEST_PG_URL` and `TABLETIST_TEST_MYSQL_URL` at the test servers when they're up (`ss -ltn` shows 55432 and 53306).

The macOS-only code (`open_privacy_settings`) doesn't build here. Type-check it for `aarch64-apple-darwin` in a scratch crate under `target/` (see Task 2, Step 5). Commits are signed: one for this plan, then one per task.

---

### Task 1: The scan says macOS's refusal in its own words, and opens each file

**Files:**
- Modify: `crates/tabletist-db/src/files.rs`
- Test: `crates/tabletist-db/tests/files_check.rs`

- [ ] **Step 1: Write the failing tests**

The unit tests go at the end of `files.rs`'s `tests` module; the integration test makes a listed file unreadable with mode `000`.

````diff
diff --git a/crates/tabletist-db/src/files.rs b/crates/tabletist-db/src/files.rs
--- a/crates/tabletist-db/src/files.rs
+++ b/crates/tabletist-db/src/files.rs
@@ -931,4 +931,55 @@
         let folder = Source::Folder("/data/bookshop-exports".into());
         assert_eq!(folder.title(), "bookshop-exports");
     }
-}
+
+    /// On macOS a refusal of its privacy settings (`EPERM`) is said as
+    /// macOS's, of the folder or of a file; a file's own permissions
+    /// (`EACCES`) are not, and elsewhere neither is.
+    #[test]
+    fn a_refusal_of_macos_privacy_settings_is_said_as_its() {
+        let eperm = std::io::Error::from_raw_os_error(1);
+        let eacces = std::io::Error::from_raw_os_error(13);
+        assert_eq!(
+            not_allowed_on(&eperm, Refused::Folder, true),
+            "macOS hasn't allowed Tabletist to open this folder"
+        );
+        assert_eq!(
+            not_allowed_on(&eperm, Refused::File, true),
+            "macOS hasn't allowed Tabletist to open this file"
+        );
+        assert_eq!(
+            not_allowed_on(&eacces, Refused::Folder, true),
+            "not allowed to read it"
+        );
+        assert_eq!(
+            not_allowed_on(&eperm, Refused::Folder, false),
+            "not allowed to read it"
+        );
+    }
+
+    /// The path and what macOS refused, read back from the message; any
+    /// other message is not one.
+    #[test]
+    fn a_refusal_of_macos_is_read_back_from_its_message() {
+        assert_eq!(
+            refused_by_macos(
+                "Can't open ~/Data/bookshop-exports: macOS hasn't allowed Tabletist to open this folder"
+            ),
+            Some(("~/Data/bookshop-exports", Refused::Folder))
+        );
+        assert_eq!(
+            refused_by_macos(
+                "Can't open ~/Documents/orders.csv: macOS hasn't allowed Tabletist to open this file"
+            ),
+            Some(("~/Documents/orders.csv", Refused::File))
+        );
+        assert_eq!(
+            refused_by_macos("Can't open ~/Data/bookshop-exports: no such folder"),
+            None
+        );
+        assert_eq!(
+            refused_by_macos("Can't open ~/Data/bookshop-exports: not allowed to read it"),
+            None
+        );
+    }
+}
diff --git a/crates/tabletist-db/tests/files_check.rs b/crates/tabletist-db/tests/files_check.rs
--- a/crates/tabletist-db/tests/files_check.rs
+++ b/crates/tabletist-db/tests/files_check.rs
@@ -441,3 +441,50 @@
     assert!(message.contains("returns.csv"), "{message}");
     assert_eq!(named.file, None);
 }
+
+/// A listed file that may be looked at and not read (its own permissions,
+/// or macOS's privacy settings for a file in a folder they guard) is said
+/// by the scan: a Test reads the others and says it, and connecting fails
+/// on it.
+#[cfg(unix)]
+#[tokio::test]
+async fn a_listed_file_that_cannot_be_read_is_said_by_the_scan() {
+    use std::os::unix::fs::PermissionsExt;
+    let dir = exports();
+    let shut = dir.path().join("returns.csv");
+    let spec = ConnectSpec::files(Source::Files(vec![
+        shut.clone(),
+        dir.path().join("orders.csv"),
+    ]));
+    std::fs::set_permissions(&shut, std::fs::Permissions::from_mode(0o000)).unwrap();
+    let readable = std::fs::File::open(&shut).is_ok();
+    let connected = Connection::connect(&spec, &Secrets::default()).await.err();
+    let tested = Connection::open_files_to_test(&spec).await;
+    let read = match &tested {
+        Ok(connection) => Some(connection.read_files().await.unwrap().unwrap()),
+        Err(_) => None,
+    };
+    std::fs::set_permissions(&shut, std::fs::Permissions::from_mode(0o644)).unwrap();
+    if readable {
+        // Run as root, which may read anything.
+        return;
+    }
+    let error = connected.unwrap();
+    assert!(
+        error
+            .to_string()
+            .contains("returns.csv: not allowed to read it"),
+        "{error}"
+    );
+    let read = read.unwrap_or_else(|| panic!("{:?}", tested.err()));
+    assert_eq!(
+        read.failed,
+        [Failure {
+            file: "returns.csv".into(),
+            path: shut,
+            format: Format::Csv,
+            why: Why::Other("not allowed to read it".into()),
+        }]
+    );
+    assert_eq!(read.checks, ["orders"]);
+}
````

- [ ] **Step 2: Run them to see them fail**

Run: `~/.cargo/bin/cargo test -q -p tabletist-db --lib refus`

Expected: the tests don't compile: `not_allowed_on`, `Refused` and `refused_by_macos` aren't there yet.

```text
error[E0433]: cannot find type `Refused` in this scope
    |
error[E0425]: cannot find function `not_allowed_on` in this scope
error[E0425]: cannot find function `refused_by_macos` in this scope
error: could not compile `tabletist-db` (lib test) due to 14 previous errors
```

Run: `~/.cargo/bin/cargo test -q -p tabletist-db --test files_check cannot_be_read`

Expected: `a_listed_file_that_cannot_be_read_is_said_by_the_scan` fails. The scan only looks at the file, so connecting fails later, in DuckDB, with other words. (Run as root, which may read anything, it returns early.)

```text
failures:
---- a_listed_file_that_cannot_be_read_is_said_by_the_scan stdout ----
thread 'a_listed_file_that_cannot_be_read_is_said_by_the_scan' (4157364) panicked at crates/tabletist-db/tests/files_check.rs:473:5:
    a_listed_file_that_cannot_be_read_is_said_by_the_scan
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 13 filtered out; finished in 0.08s
error: test failed, to rerun pass `-p tabletist-db --test files_check`
```

- [ ] **Step 3: Tell macOS's refusal from a file's own permissions**

````diff
diff --git a/crates/tabletist-db/src/files.rs b/crates/tabletist-db/src/files.rs
--- a/crates/tabletist-db/src/files.rs
+++ b/crates/tabletist-db/src/files.rs
@@ -409,9 +409,13 @@
                     continue;
                 }
                 // A link to a file counts as the file; a folder never, nor
-                // a broken link. A file that cannot be looked at is said.
+                // a broken link. A file that cannot be looked at, or read,
+                // is said.
                 match std::fs::metadata(&path) {
-                    Ok(meta) if meta.is_file() => entries.push((path, format, meta.len())),
+                    Ok(meta) if meta.is_file() => match readable(&path) {
+                        Ok(()) => entries.push((path, format, meta.len())),
+                        Err(error) => unlooked(&path, &error)?,
+                    },
                     Ok(_) => {}
                     Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                     Err(error) => unlooked(&path, &error)?,
@@ -429,7 +433,10 @@
                     )));
                 };
                 match std::fs::metadata(path) {
-                    Ok(meta) if meta.is_file() => entries.push((path.clone(), format, meta.len())),
+                    Ok(meta) if meta.is_file() => match readable(path) {
+                        Ok(()) => entries.push((path.clone(), format, meta.len())),
+                        Err(error) => unlooked(path, &error)?,
+                    },
                     Ok(_) => missing.push(path.clone()),
                     Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                         missing.push(path.clone());
@@ -472,12 +479,19 @@
     (name, path.to_owned())
 }
 
+/// Whether `path` may be read, which looking at it does not say: a file's
+/// own permissions refuse its contents and not its size, and so do macOS's
+/// privacy settings for a file in a folder they guard.
+fn readable(path: &Path) -> std::io::Result<()> {
+    std::fs::File::open(path).map(drop)
+}
+
 /// Why a folder cannot be read, in the dialog's words.
 fn folder_error(folder: &Path, error: &std::io::Error) -> Error {
     let why = match error.kind() {
         std::io::ErrorKind::NotFound => "no such folder".to_owned(),
         std::io::ErrorKind::NotADirectory => "not a folder".to_owned(),
-        std::io::ErrorKind::PermissionDenied => "not allowed to read it".to_owned(),
+        std::io::ErrorKind::PermissionDenied => not_allowed(error, Refused::Folder),
         _ => error.to_string(),
     };
     Error::Connect(format!("Can't open {}: {why}", shown(folder)))
@@ -492,9 +506,59 @@
 /// Why a file cannot be looked at, in the dialog's words.
 fn file_why(error: &std::io::Error) -> String {
     match error.kind() {
-        std::io::ErrorKind::PermissionDenied => "not allowed to read it".to_owned(),
+        std::io::ErrorKind::PermissionDenied => not_allowed(error, Refused::File),
         _ => error.to_string(),
     }
+}
+
+/// What macOS's privacy settings (Files and Folders, Full Disk Access)
+/// did not let Tabletist open: a Files connection's folder, or one of its
+/// files.
+#[derive(Debug, Clone, Copy, PartialEq, Eq)]
+pub enum Refused {
+    Folder,
+    File,
+}
+
+/// What a refusal of macOS's privacy settings says after the path.
+const FOLDER_REFUSED: &str = "macOS hasn't allowed Tabletist to open this folder";
+const FILE_REFUSED: &str = "macOS hasn't allowed Tabletist to open this file";
+
+/// Why `what` may not be read, its error a refusal: macOS's privacy
+/// settings where they refused it, its own permissions anywhere else.
+fn not_allowed(error: &std::io::Error, what: Refused) -> String {
+    not_allowed_on(error, what, cfg!(target_os = "macos"))
+}
+
+/// [`not_allowed`] on macOS or elsewhere. macOS's privacy settings refuse
+/// with `EPERM`, where a file's own permissions refuse with `EACCES`.
+fn not_allowed_on(error: &std::io::Error, what: Refused, macos: bool) -> String {
+    const EPERM: i32 = 1;
+    match (macos && error.raw_os_error() == Some(EPERM), what) {
+        (true, Refused::Folder) => FOLDER_REFUSED.to_owned(),
+        (true, Refused::File) => FILE_REFUSED.to_owned(),
+        (false, _) => "not allowed to read it".to_owned(),
+    }
+}
+
+/// Whether `why`, a file's [`Why::Other`], is macOS's privacy settings
+/// refusing Tabletist that file: a Test goes past such a file, and says
+/// so.
+pub fn refused_file_by_macos(why: &str) -> bool {
+    why == FILE_REFUSED
+}
+
+/// The folder or file a message of [`scan`]'s says macOS's privacy
+/// settings did not let Tabletist open, as the message shows it, and which
+/// it is. `None` for any other message.
+pub fn refused_by_macos(message: &str) -> Option<(&str, Refused)> {
+    let rest = message.strip_prefix("Can't open ")?;
+    [
+        (FOLDER_REFUSED, Refused::Folder),
+        (FILE_REFUSED, Refused::File),
+    ]
+    .into_iter()
+    .find_map(|(why, what)| Some((rest.strip_suffix(why)?.strip_suffix(": ")?, what)))
 }
 
 /// The name a file's stem becomes: lowercased, with anything outside
````

- [ ] **Step 4: Run them to see them pass**

Run: `~/.cargo/bin/cargo test -q -p tabletist-db --lib refus`

Expected: `a_refusal_of_macos_privacy_settings_is_said_as_its` and `a_refusal_of_macos_is_read_back_from_its_message` pass, with the others the filter picks. `~/.cargo/bin/cargo test -q -p tabletist-db --test files_check` passes too.

- [ ] **Step 5: Run the full check**

Expected: all pass; 5,657 passed, 0 failed.

- [ ] **Step 6: Commit**

```bash
git add crates/tabletist-db/src/files.rs crates/tabletist-db/tests/files_check.rs
git commit -m "Say a Files folder macOS's privacy settings refused in its own words, and open each file the scan keeps"
```

### Task 2: The dialog's banner and Open Privacy & Security

**Files:**
- Create: `src/ui/not_allowed.rs`, `src/ui/not_allowed_tests.rs`
- Modify: `crates/tabletist-core/src/backend.rs`, `src/promo/live.rs`, `crates/tabletist-core/src/model.rs`, `src/app.rs`, `src/ui/mod.rs`, `src/ui/connect_dialog/mod.rs`, `src/ui/connect_dialog/files.rs`, `src/ui/partial.rs`, `src/ui/banner.rs`, `assets/i18n/ru.po`, `assets/i18n/pt-BR.po`, `assets/i18n/tabletist.pot`, `docs/_reference/connections.md`

- [ ] **Step 1: Write the failing tests**

The helpers open a new Files connection's dialog over the Bookshop's exports, and answer its Test.

````diff
diff --git a/src/ui/mod.rs b/src/ui/mod.rs
--- a/src/ui/mod.rs
+++ b/src/ui/mod.rs
@@ -113,6 +113,8 @@
 #[cfg(test)]
 mod look_switch_tests;
 pub mod menu;
+#[cfg(test)]
+mod not_allowed_tests;
 pub mod object_tabs;
 pub mod pane_hints;
 #[cfg(test)]
diff --git a/src/ui/not_allowed_tests.rs b/src/ui/not_allowed_tests.rs
new file mode 100644
--- /dev/null
+++ b/src/ui/not_allowed_tests.rs
@@ -0,0 +1,248 @@
+//! A Files connection's folder that macOS's privacy settings did not let
+//! Tabletist open: the dialog's Test and the window's connect card say so
+//! in the boards' words, and offer Open Privacy & Security, which opens
+//! System Settings at Files and Folders.
+
+use std::path::PathBuf;
+
+use egui::accesskit::Role;
+use tabletist_db::Error;
+use tabletist_db::files::{Format, Found, Scan};
+
+use crate::backend::{Command, Event};
+use crate::model::{Action, ConnectionForm, Dialog, FilesFound, PickTarget, TestState};
+use crate::testing::{Harness, node};
+use crate::theme::Look;
+
+fn looks() -> [Look; 2] {
+    [Look::macos(), Look::omarchy()]
+}
+
+/// With capitals, which the path keeps in the terminal look's lower case.
+const FOLDER: &str = "/data/Bookshop-Exports";
+
+/// What the backend says of the folder macOS's privacy settings refused.
+fn refused() -> String {
+    format!("Can't open {FOLDER}: macOS hasn't allowed Tabletist to open this folder")
+}
+
+fn form(harness: &Harness) -> &ConnectionForm {
+    match &harness.app.dialog {
+        Some(Dialog::Connection(form)) => form,
+        _ => panic!("no connection dialog"),
+    }
+}
+
+/// A new Files connection's dialog in `look`, its folder the Bookshop's
+/// exports, read.
+fn exports(look: Look) -> Harness {
+    let mut harness = Harness::new();
+    harness.set_look(look);
+    harness.app.apply(Action::NewConnection);
+    harness.settle();
+    harness.click(&look.label("Files"));
+    harness.app.apply(Action::PickFolder);
+    let request = form(&harness).pick_request.expect("a pick in flight");
+    assert_eq!(form(&harness).pick_target, PickTarget::Folder);
+    harness.app.apply(Action::Backend(Event::FilePicked {
+        request,
+        path: Some(FOLDER.into()),
+    }));
+    let FilesFound::Reading(request) = form(&harness).files_found else {
+        panic!("no read of the source");
+    };
+    harness.app.apply(Action::Backend(Event::SourceRead {
+        request,
+        result: Ok(Scan {
+            files: vec![Found {
+                path: PathBuf::from(FOLDER).join("orders.csv"),
+                format: Format::Csv,
+                size: 1_000,
+                name: "orders".into(),
+            }],
+            missing: Vec::new(),
+            unreadable: Vec::new(),
+        }),
+    }));
+    harness.settle();
+    harness
+}
+
+/// Runs the dialog's Test and answers it with `error`.
+fn tested(harness: &mut Harness, error: Error) {
+    harness.app.apply(Action::TestConnection);
+    let TestState::Running(request) = form(harness).test else {
+        panic!("no test: {:?}", form(harness).test);
+    };
+    harness.app.apply(Action::Backend(Event::FilesTested {
+        request,
+        result: Err(error),
+    }));
+    harness.settle();
+}
+
+/// The banner's and the card's first words in `look`: the path keeps its
+/// case.
+fn lead(look: Look) -> String {
+    look.label("Can't read {path}.").replace("{path}", FOLDER)
+}
+
+/// What the banner says in `look`, as a screen reader reads it.
+fn said(look: Look) -> String {
+    format!(
+        "{} {}",
+        lead(look),
+        look.label("macOS hasn't allowed Tabletist to open this folder.")
+    )
+}
+
+fn privacy(look: Look) -> String {
+    look.label("Open Privacy & Security")
+}
+
+fn opened_privacy(harness: &Harness) -> bool {
+    harness
+        .app
+        .backend
+        .sent
+        .iter()
+        .any(|command| matches!(command, Command::OpenPrivacySettings))
+}
+
+#[test]
+fn a_test_macos_refused_says_so_and_opens_privacy_and_security() {
+    for look in looks() {
+        let mut harness = exports(look);
+        tested(&mut harness, Error::Connect(refused()));
+        let tree = harness.settle();
+        let labels = crate::testing::labels(&tree);
+        assert!(
+            node(&tree, &said(look), Role::Alert).is_some(),
+            "{}: {labels:?}",
+            look.name
+        );
+        // The banner says it: not the backend's words again over it.
+        assert!(
+            !labels.iter().any(|label| label.contains(&refused())),
+            "{}: {labels:?}",
+            look.name
+        );
+        assert!(!opened_privacy(&harness));
+        harness.click(&privacy(look));
+        assert!(opened_privacy(&harness), "{}", look.name);
+    }
+}
+
+/// A folder that is gone, or that its own permissions refuse, is not
+/// macOS's to allow: no way to Privacy & Security.
+#[test]
+fn other_failures_offer_no_privacy_and_security() {
+    for why in ["no such folder", "not allowed to read it"] {
+        let mut harness = exports(Look::macos());
+        tested(
+            &mut harness,
+            Error::Connect(format!("Can't open {FOLDER}: {why}")),
+        );
+        let tree = harness.settle();
+        assert!(
+            node(&tree, &privacy(Look::macos()), Role::Button).is_none(),
+            "{why}"
+        );
+    }
+}
+
+/// The found line under Source says it in the banner's words.
+#[test]
+fn the_found_line_says_macos_refused_the_folder() {
+    let mut harness = exports(Look::macos());
+    harness.app.apply(Action::ReadSource);
+    let FilesFound::Reading(request) = form(&harness).files_found else {
+        panic!("no read of the source");
+    };
+    harness.app.apply(Action::Backend(Event::SourceRead {
+        request,
+        result: Err(refused()),
+    }));
+    let tree = harness.settle();
+    let labels = crate::testing::labels(&tree);
+    assert!(
+        labels
+            .iter()
+            .any(|label| label == "macOS hasn't allowed Tabletist to open this folder."),
+        "{labels:?}"
+    );
+}
+
+/// A Test that read past a file macOS's privacy settings refused: the
+/// partial banner says so in its words, with Open Privacy & Security
+/// first and Skip.
+#[test]
+fn a_test_past_a_file_macos_refused_offers_privacy_and_security() {
+    use tabletist_db::FilesRead;
+    use tabletist_db::files::{Failure, Why};
+    for look in looks() {
+        let mut harness = exports(look);
+        harness.app.apply(Action::TestConnection);
+        let TestState::Running(request) = form(&harness).test else {
+            panic!("no test: {:?}", form(&harness).test);
+        };
+        harness.app.apply(Action::Backend(Event::FilesTested {
+            request,
+            result: Ok(FilesRead {
+                files: 1,
+                bytes: 1_000,
+                failed: vec![Failure {
+                    file: "orders.csv".into(),
+                    path: PathBuf::from(FOLDER).join("orders.csv"),
+                    format: Format::Csv,
+                    why: Why::Other("macOS hasn't allowed Tabletist to open this file".into()),
+                }],
+                checks: Vec::new(),
+            }),
+        }));
+        let tree = harness.settle();
+        let labels = crate::testing::labels(&tree);
+        let said = match look.terminal {
+            true => "! orders.csv macos hasn't allowed tabletist to open this file",
+            false => {
+                "orders.csv couldn't be read. macOS hasn't allowed Tabletist to open this file."
+            }
+        };
+        assert!(
+            node(&tree, said, Role::Alert).is_some(),
+            "{}: {labels:?}",
+            look.name
+        );
+        assert!(
+            node(&tree, &look.label("Skip orders.csv"), Role::Button).is_some(),
+            "{}: {labels:?}",
+            look.name
+        );
+        harness.click(&privacy(look));
+        assert!(opened_privacy(&harness), "{}", look.name);
+        // Privacy & Security mends nothing in the form.
+        assert!(form(&harness).files_csv.overrides.is_empty());
+    }
+}
+
+/// A list's found line names the file macOS refused.
+#[test]
+fn the_found_line_names_a_listed_file_macos_refused() {
+    let mut harness = exports(Look::macos());
+    harness.app.apply(Action::ReadSource);
+    let FilesFound::Reading(request) = form(&harness).files_found else {
+        panic!("no read of the source");
+    };
+    harness.app.apply(Action::Backend(Event::SourceRead {
+        request,
+        result: Err(format!(
+            "Can't open {FOLDER}/orders.csv: macOS hasn't allowed Tabletist to open this file"
+        )),
+    }));
+    let tree = harness.settle();
+    let labels = crate::testing::labels(&tree);
+    let line = format!(
+        "Can't read {FOLDER}/orders.csv. macOS hasn't allowed Tabletist to open this file."
+    );
+    assert!(labels.contains(&line), "{labels:?}");
+}
````

- [ ] **Step 2: Run them to see them fail**

Run: `~/.cargo/bin/cargo test -q -p tabletist not_allowed`

Expected: the tests don't compile: there is no `Command::OpenPrivacySettings` yet.

```text
error[E0599]: no variant, associated function, or constant named `OpenPrivacySettings` found for enum `tabletist_core::backend::Command` in the current scope
    |
error: could not compile `tabletist` (lib test) due to 1 previous error
```

- [ ] **Step 3: The command, the action, the words and the banner**

````diff
diff --git a/crates/tabletist-core/src/backend.rs b/crates/tabletist-core/src/backend.rs
--- a/crates/tabletist-core/src/backend.rs
+++ b/crates/tabletist-core/src/backend.rs
@@ -291,6 +291,10 @@
     Reveal {
         path: PathBuf,
     },
+    /// Opens System Settings at Privacy & Security, Files and Folders, on
+    /// macOS, where a Files connection's folder may be allowed; nothing
+    /// elsewhere. Nothing is answered: a failure is logged.
+    OpenPrivacySettings,
     /// A desktop notification of `summary` over `body`, with no action:
     /// on Linux over D-Bus (what mako shows on Omarchy); nothing elsewhere,
     /// where the window asks for attention instead. Nothing is answered.
@@ -2053,6 +2057,28 @@
     }
 }
 
+/// Where System Settings shows which apps may open the files in which
+/// folders.
+#[cfg(target_os = "macos")]
+const PRIVACY_FILES_AND_FOLDERS: &str =
+    "x-apple.systempreferences:com.apple.preference.security?Privacy_FilesAndFolders";
+
+/// Opens System Settings at Privacy & Security, Files and Folders, with
+/// `open`, which hands the link to System Settings, not to the browser.
+/// Nothing to open elsewhere.
+fn open_privacy_settings() -> std::io::Result<()> {
+    #[cfg(target_os = "macos")]
+    {
+        let status = std::process::Command::new("open")
+            .arg(PRIVACY_FILES_AND_FOLDERS)
+            .status()?;
+        if !status.success() {
+            return Err(std::io::Error::other(format!("open exited with {status}")));
+        }
+    }
+    Ok(())
+}
+
 /// The program that shows where `path` is kept on this system, and what
 /// it is given. On Windows the one argument is explorer's command line as
 /// it must read, and is given to it as it is ([`start_reveal`]).
@@ -2629,6 +2655,13 @@
                     }
                 });
             }
+            Command::OpenPrivacySettings => {
+                tokio::task::spawn_blocking(|| {
+                    if let Err(error) = open_privacy_settings() {
+                        log::warn!("could not open Privacy & Security: {error}");
+                    }
+                });
+            }
             Command::ListFolder {
                 asked,
                 typed,
@@ -2726,6 +2759,7 @@
         | Command::Export(_)
         | Command::CancelExport(_)
         | Command::Reveal { .. }
+        | Command::OpenPrivacySettings
         | Command::Notify { .. }
         | Command::ListFolder { .. }
         | Command::Agents(_)
@@ -2795,6 +2829,7 @@
         | Command::Export(_)
         | Command::CancelExport(_)
         | Command::Reveal { .. }
+        | Command::OpenPrivacySettings
         | Command::Notify { .. }
         | Command::ListFolder { .. }
         | Command::Agents(_)
@@ -2941,6 +2976,7 @@
         | Command::Export(_)
         | Command::CancelExport(_)
         | Command::Reveal { .. }
+        | Command::OpenPrivacySettings
         | Command::Notify { .. }
         | Command::ListFolder { .. }
         | Command::Agents(_)
@@ -3376,6 +3412,7 @@
             | Command::Export(_)
             | Command::CancelExport(_)
             | Command::Reveal { .. }
+            | Command::OpenPrivacySettings
             | Command::Notify { .. }
             | Command::ListFolder { .. }
             | Command::Agents(_)
diff --git a/crates/tabletist-core/src/model.rs b/crates/tabletist-core/src/model.rs
--- a/crates/tabletist-core/src/model.rs
+++ b/crates/tabletist-core/src/model.rs
@@ -119,6 +119,10 @@
     /// One of the partial Test's fixes for the first file it could not
     /// read: saved as an override in the form, and the Test run again.
     FixFile(FileFix),
+    /// Open System Settings where macOS lets an app open a folder's files
+    /// (Privacy & Security, Files and Folders): from the "not allowed"
+    /// state of a Files connection, in its dialog or its window.
+    OpenPrivacySettings,
     /// One of the partial banner's fixes for the file a Files table's rows
     /// could not be read from: saved as an override in the connection, and
     /// the session opened again with it. Skip closes the tab.
diff --git a/docs/_reference/connections.md b/docs/_reference/connections.md
--- a/docs/_reference/connections.md
+++ b/docs/_reference/connections.md
@@ -156,6 +156,14 @@
   stay and the table reads again. **Skip** closes the table's tab, and
   the table leaves the sidebar. Reset under Reading CSV in the
   connection's dialog takes a fix off.
+- On macOS, a folder that macOS's privacy settings have not let
+  Tabletist open (one in Documents, Desktop or Downloads, or on an
+  external disk, until you allow it) makes the Test fail with "Can't read
+  ~/Data/bookshop-exports. macOS hasn't allowed Tabletist to open this
+  folder." and **Open Privacy & Security**, which opens System Settings
+  at Privacy & Security › Files and Folders. Opening the connection says
+  the same on its window, with the same button first. macOS keeps what
+  you allow there, so Tabletist keeps no bookmark of the folder.
 - The SQL editor runs DuckDB's SQL, and the Structure view shows each
   file's path, size and format beside its columns, and what it is read
   with.
diff --git a/src/app.rs b/src/app.rs
--- a/src/app.rs
+++ b/src/app.rs
@@ -3589,6 +3589,7 @@
                     self.apply(Action::TestConnection);
                 }
             }
+            Action::OpenPrivacySettings => self.backend.send(Command::OpenPrivacySettings),
             Action::FixFile(fix) => {
                 let Some(Dialog::Connection(form)) = &mut self.dialog else {
                     return;
diff --git a/src/promo/live.rs b/src/promo/live.rs
--- a/src/promo/live.rs
+++ b/src/promo/live.rs
@@ -269,6 +269,7 @@
                 | Command::Export(_)
                 | Command::CancelExport(_)
                 | Command::Reveal { .. }
+                | Command::OpenPrivacySettings
                 | Command::Notify { .. }
                 | Command::LoadSecret { .. }
                 | Command::StoreSecret { .. }
diff --git a/src/ui/banner.rs b/src/ui/banner.rs
--- a/src/ui/banner.rs
+++ b/src/ui/banner.rs
@@ -116,7 +116,16 @@
     /// The width of an action in the terminal look: its key, a space, its
     /// words. No wider than the row `width`.
     fn key_width(ctx: &egui::Context, (key, words): &(Laid, Laid), width: f32, look: &Look) -> f32 {
-        (key.width() + space(ctx, look, " ") + words.width()).min(width)
+        (key.width() + Self::after_key(ctx, key, look) + words.width()).min(width)
+    }
+
+    /// The space between an action's key and its words: none where it has
+    /// no key, so its words start where a key would.
+    fn after_key(ctx: &egui::Context, key: &Laid, look: &Look) -> f32 {
+        match key.width() > 0.0 {
+            true => space(ctx, look, " "),
+            false => 0.0,
+        }
     }
 
     /// Where each action goes, from the left of a row `width` wide, `wide`
@@ -230,7 +239,6 @@
             sentence.paint(ui.painter(), pos2(left, top));
             let line = TextRole::OBody.row_height(ui.ctx(), look.faces);
             let below = top + sentence.height();
-            let space = space(ui.ctx(), look, " ");
             for (at, ((place, (key, words)), action)) in self
                 .key_places(ui.ctx(), inner, look, palette)
                 .into_iter()
@@ -247,7 +255,8 @@
                         .rect_filled(place, CornerRadius::ZERO, palette.selection);
                 }
                 key.paint_left(ui.painter(), x, y);
-                words.paint_left(ui.painter(), x + key.width() + space, y);
+                let after = Self::after_key(ui.ctx(), &key, look);
+                words.paint_left(ui.painter(), x + key.width() + after, y);
                 crate::ui::focus::hint(ui, &response, place, crate::ui::focus::Ring::Bar);
                 if response.clicked() {
                     pressed = Some(at);
diff --git a/src/ui/connect_dialog/files.rs b/src/ui/connect_dialog/files.rs
--- a/src/ui/connect_dialog/files.rs
+++ b/src/ui/connect_dialog/files.rs
@@ -54,13 +54,23 @@
 }
 
 /// Why the Source cannot be read, as the field says it: a folder that is
-/// gone in the board's words (translated), anything else as the backend
-/// said it.
+/// gone, or that macOS's privacy settings refused, in the boards' words
+/// (translated), anything else as the backend said it.
 fn trouble(found: &FilesFound, skin: &Skin) -> Option<String> {
     match found {
         FilesFound::Failed(why) if why.ends_with(": no such folder") => Some(skin.say(GONE)),
-        // A path in it keeps its case, in either look.
-        FilesFound::Failed(why) => Some(why.clone()),
+        // macOS's privacy settings in the banner's words, naming the file
+        // where it is one of a list's; anything else as it is, a path in it
+        // keeping its case in either look.
+        FilesFound::Failed(why) => Some(
+            match crate::ui::not_allowed::NotAllowed::of(why, skin.look, skin.locale) {
+                Some(refused) if refused.what == tabletist_db::files::Refused::File => {
+                    format!("{} {}", refused.lead, refused.text)
+                }
+                Some(refused) => refused.text,
+                None => why.clone(),
+            },
+        ),
         FilesFound::Unread | FilesFound::Reading(_) | FilesFound::Read(_) => None,
     }
 }
diff --git a/src/ui/connect_dialog/mod.rs b/src/ui/connect_dialog/mod.rs
--- a/src/ui/connect_dialog/mod.rs
+++ b/src/ui/connect_dialog/mod.rs
@@ -1173,9 +1173,38 @@
     cap: f32,
     /// An unknown SSH host key: the button that trusts it follows.
     untrusted: bool,
-    /// A Files connection's partial Test: the banner of the first file it
-    /// could not read, with its fixes, under the notices, and its height.
-    partial: Option<(crate::ui::partial::Partial, f32)>,
+    /// A Files connection's banner under the notices, and its height: the
+    /// partial Test's of the first file it could not read, with its fixes,
+    /// or the "not allowed" state's, with the way to Privacy & Security.
+    banner: Option<(Shown, f32)>,
+}
+
+/// The banner the messages show: a Test that read some files, or one that
+/// macOS's privacy settings refused. Never both: one Test passed, the
+/// other failed.
+enum Shown {
+    Partial(crate::ui::partial::Partial),
+    NotAllowed(crate::ui::not_allowed::NotAllowed),
+}
+
+impl Shown {
+    fn banner(&self) -> crate::ui::banner::Banner<'_> {
+        match self {
+            Self::Partial(partial) => partial.banner(),
+            Self::NotAllowed(refused) => refused.banner(),
+        }
+    }
+
+    /// What pressing the banner's action at `at` asks for.
+    fn action(&self, at: usize) -> Option<Action> {
+        match self {
+            Self::Partial(partial) if partial.opens_privacy(at) => {
+                Some(Action::OpenPrivacySettings)
+            }
+            Self::Partial(partial) => partial.fix(at).map(Action::FixFile),
+            Self::NotAllowed(_) => Some(Action::OpenPrivacySettings),
+        }
+    }
 }
 
 impl Messages {
@@ -1189,9 +1218,16 @@
             texts.push((message.into(), palette.danger));
         }
         let mut untrusted = false;
+        let mut refused = None;
         match &form.test {
             TestState::Idle | TestState::Running(_) | TestState::Passed => {}
-            TestState::Failed(message) => texts.push((message.into(), palette.danger)),
+            // macOS's refusal is the banner's to say, with its button.
+            TestState::Failed(message) => {
+                match crate::ui::not_allowed::NotAllowed::of(message, look, skin.locale) {
+                    Some(not_allowed) => refused = Some(not_allowed),
+                    None => texts.push((message.into(), palette.danger)),
+                }
+            }
             TestState::Untrusted {
                 host, fingerprint, ..
             } => {
@@ -1220,22 +1256,24 @@
                 .collect()
         };
         let partial = form.first_failure().map(|failure| {
-            let partial = crate::ui::partial::of(
+            Shown::Partial(crate::ui::partial::of(
                 ctx,
                 failure,
                 skin.look,
                 skin.locale,
                 &crate::ui::partial::FORM_KEYS,
-            );
+            ))
+        });
+        let banner = partial.or(refused.map(Shown::NotAllowed)).map(|shown| {
             let width = width - 2.0 * Self::side(skin);
-            let height = partial.banner().height(ctx, width, look, skin.palette);
-            (partial, height)
+            let height = shown.banner().height(ctx, width, look, skin.palette);
+            (shown, height)
         });
         let mut messages = Self {
             notices: lay(wrap),
             cap,
             untrusted,
-            partial,
+            banner,
         };
         // Texts that will scroll leave room for the scroll bar.
         if messages.texts_height(skin) > cap {
@@ -1285,7 +1323,7 @@
     /// The height the messages take in the dialog: nothing when nothing
     /// went wrong.
     fn height(&self, skin: &Skin) -> f32 {
-        if self.notices.is_empty() && self.partial.is_none() {
+        if self.notices.is_empty() && self.banner.is_none() {
             return 0.0;
         }
         let button = if self.untrusted {
@@ -1293,7 +1331,7 @@
         } else {
             0.0
         };
-        let banner = match (&self.partial, self.notices.is_empty()) {
+        let banner = match (&self.banner, self.notices.is_empty()) {
             (Some((_, height)), true) => *height,
             (Some((_, height)), false) => Self::GAP + height,
             (None, _) => 0.0,
@@ -1306,7 +1344,7 @@
     }
 
     fn show(self, ui: &mut Ui, skin: &Skin, actions: &mut Vec<Action>) {
-        if self.notices.is_empty() && self.partial.is_none() {
+        if self.notices.is_empty() && self.banner.is_none() {
             return;
         }
         let noticed = !self.notices.is_empty();
@@ -1347,17 +1385,17 @@
                             }
                         });
                 }
-                // The partial Test's banner, over the footer, with its fixes.
-                if let Some((partial, _)) = &self.partial {
+                // The banner over the footer, with its actions.
+                if let Some((shown, _)) = &self.banner {
                     if noticed {
                         ui.add_space(Self::GAP);
                     }
-                    if let Some(fix) = partial
+                    if let Some(action) = shown
                         .banner()
                         .show(ui, look, palette)
-                        .and_then(|at| partial.fix(at))
+                        .and_then(|at| shown.action(at))
                     {
-                        actions.push(Action::FixFile(fix));
+                        actions.push(action);
                     }
                 }
                 // The way out of an unknown host key stays in reach however
diff --git a/src/ui/mod.rs b/src/ui/mod.rs
--- a/src/ui/mod.rs
+++ b/src/ui/mod.rs
@@ -113,6 +113,7 @@
 #[cfg(test)]
 mod look_switch_tests;
 pub mod menu;
+pub mod not_allowed;
 #[cfg(test)]
 mod not_allowed_tests;
 pub mod object_tabs;
diff --git a/src/ui/not_allowed.rs b/src/ui/not_allowed.rs
new file mode 100644
--- /dev/null
+++ b/src/ui/not_allowed.rs
@@ -0,0 +1,65 @@
+//! A Files connection that macOS's privacy settings did not let Tabletist
+//! open: what the connection dialog's banner and the window's connect card
+//! say of it, and the button that opens System Settings where the folder
+//! may be allowed (Privacy & Security, Files and Folders).
+
+use tabletist_db::files::Refused;
+
+use crate::i18n::{Locale, fill, gettext};
+use crate::theme::Look;
+use crate::ui::banner::{Action, Banner};
+use crate::ui::states::Tone;
+
+/// The "not allowed" state's words, made for one frame.
+pub struct NotAllowed {
+    /// "Can't read ~/Data/bookshop-exports."
+    pub lead: String,
+    /// "macOS hasn't allowed Tabletist to open this folder."
+    pub text: String,
+    /// "Open Privacy & Security".
+    pub button: String,
+    /// What macOS refused: the folder, or one of its files.
+    pub what: Refused,
+}
+
+impl NotAllowed {
+    /// The state the backend's `message` says, in `look` and `locale`:
+    /// `None` where it is not macOS's privacy settings that refused.
+    pub fn of(message: &str, look: &Look, locale: Locale) -> Option<Self> {
+        let (path, what) = tabletist_db::files::refused_by_macos(message)?;
+        // The path is put in after the look's case: it keeps its own.
+        let lead = fill(
+            &look.label(&gettext(locale, "Can't read {path}.")),
+            &[("{path}", path)],
+        );
+        let text = look.label(&match what {
+            Refused::Folder => gettext(
+                locale,
+                "macOS hasn't allowed Tabletist to open this folder.",
+            ),
+            Refused::File => gettext(locale, "macOS hasn't allowed Tabletist to open this file."),
+        });
+        let button = look.label(&gettext(locale, "Open Privacy & Security"));
+        Some(Self {
+            lead,
+            text,
+            button,
+            what,
+        })
+    }
+
+    /// The danger banner, its one action the button.
+    pub fn banner(&self) -> Banner<'_> {
+        Banner {
+            tone: Tone::Danger,
+            lead: &self.lead,
+            text: &self.text,
+            actions: vec![Action {
+                label: &self.button,
+                key: None,
+                quiet: false,
+            }],
+            salt: "not-allowed",
+        }
+    }
+}
diff --git a/src/ui/partial.rs b/src/ui/partial.rs
--- a/src/ui/partial.rs
+++ b/src/ui/partial.rs
@@ -50,6 +50,9 @@
 pub struct Partial {
     lead: String,
     text: String,
+    /// "Open Privacy & Security", first, where macOS's privacy settings
+    /// refused the file.
+    privacy: Option<String>,
     fixes: Vec<Fix>,
 }
 
@@ -67,13 +70,18 @@
             lead: &self.lead,
             text: &self.text,
             actions: self
-                .fixes
+                .privacy
                 .iter()
-                .map(|fix| Action {
+                .map(|label| Action {
+                    label,
+                    key: None,
+                    quiet: false,
+                })
+                .chain(self.fixes.iter().map(|fix| Action {
                     label: &fix.label,
                     key: fix.key.clone(),
                     quiet: fix.quiet,
-                })
+                }))
                 .collect(),
             salt: "partial",
         }
@@ -81,7 +89,13 @@
 
     /// The fix of the action at `at`, as the banner counts them.
     pub fn fix(&self, at: usize) -> Option<FileFix> {
-        self.fixes.get(at).map(|fix| fix.fix)
+        let first = usize::from(self.privacy.is_some());
+        self.fixes.get(at.checked_sub(first)?).map(|fix| fix.fix)
+    }
+
+    /// Whether the action at `at` opens Privacy & Security.
+    pub fn opens_privacy(&self, at: usize) -> bool {
+        self.privacy.is_some() && at == 0
     }
 }
 
@@ -192,7 +206,20 @@
         key: key(keys.skip),
         quiet: true,
     });
-    Partial { lead, text, fixes }
+    // macOS's privacy settings refused the file: where it may be allowed.
+    let privacy = match &failure.why {
+        Why::Other(why) if tabletist_db::files::refused_file_by_macos(why) => Some(
+            skin.look
+                .label(&gettext(skin.locale, "Open Privacy & Security")),
+        ),
+        _ => None,
+    };
+    Partial {
+        lead,
+        text,
+        privacy,
+        fixes,
+    }
 }
 
 /// What the banner says of why the file could not be read.
@@ -248,6 +275,12 @@
             ),
             &[("line", &crate::i18n::number(skin.locale, *line))],
         ),
+        Why::Other(message) if tabletist_db::files::refused_file_by_macos(message) => {
+            skin.look.label(&gettext(
+                skin.locale,
+                "macOS hasn't allowed Tabletist to open this file.",
+            ))
+        }
         Why::Other(message) => message.clone(),
     }
 }
````

- [ ] **Step 4: Bring the catalogs up to date and translate the four messages**

Run `sh assets/i18n/update.sh`, then fill in their `msgstr`s:

| msgid | ru | pt-BR |
| --- | --- | --- |
| Can't read {path}. | Не удалось прочитать {path}. | Não foi possível ler {path}. |
| macOS hasn't allowed Tabletist to open this folder. | macOS не разрешила Tabletist открыть эту папку. | O macOS não permitiu que o Tabletist abrisse esta pasta. |
| macOS hasn't allowed Tabletist to open this file. | macOS не разрешила Tabletist открыть этот файл. | O macOS não permitiu que o Tabletist abrisse este arquivo. |
| Open Privacy & Security | Открыть «Конфиденциальность и безопасность» | Abrir Privacidade e Segurança |

Run `sh assets/i18n/update.sh` again. Then `sh assets/i18n/update.sh --check` and `sh assets/i18n/update_test.sh` pass, with every entry translated.

- [ ] **Step 5: Type-check the macOS code**

Copy the block from `/// Where System Settings shows` down to `reveal_command`'s doc comment out of `backend.rs`. Put it in a scratch crate under `target/` with `#![deny(warnings)]` and a `pub fn call()` that calls `open_privacy_settings`. Run `cargo check --target aarch64-apple-darwin` and `--target x86_64-pc-windows-msvc`; both pass. Don't commit the scratch crate.

- [ ] **Step 6: Run them to see them pass**

Run: `~/.cargo/bin/cargo test -q -p tabletist not_allowed`

Expected: `test result: ok. 5 passed`. The partial Test's banner is otherwise unchanged: `~/.cargo/bin/cargo test -q -p tabletist partial_test` passes.

- [ ] **Step 7: Run the full check**

Expected: all pass; 5,662 passed, 0 failed.

- [ ] **Step 8: Commit**

```bash
git add -A src crates assets/i18n docs/_reference/connections.md
git commit -m "Say in the Files dialog that macOS hasn't allowed the folder, with Open Privacy & Security"
```

### Task 3: The connect-failure card

**Files:**
- Modify: `src/ui/workspace.rs`
- Test: `src/ui/not_allowed_tests.rs`

- [ ] **Step 1: Write the failing test**

````diff
diff --git a/src/ui/not_allowed_tests.rs b/src/ui/not_allowed_tests.rs
--- a/src/ui/not_allowed_tests.rs
+++ b/src/ui/not_allowed_tests.rs
@@ -6,12 +6,14 @@
 use std::path::PathBuf;
 
 use egui::accesskit::Role;
-use tabletist_db::Error;
-use tabletist_db::files::{Format, Found, Scan};
+use tabletist_db::files::{Format, Found, Scan, Source};
+use tabletist_db::{ConnectSpec, Error};
 
 use crate::backend::{Command, Event};
+use crate::connections::{ConnectionId, PasswordMode, SavedConnection};
+use crate::env::Environment;
 use crate::model::{Action, ConnectionForm, Dialog, FilesFound, PickTarget, TestState};
-use crate::testing::{Harness, node};
+use crate::testing::{Harness, last_sent, node};
 use crate::theme::Look;
 
 fn looks() -> [Look; 2] {
@@ -246,3 +248,58 @@
     );
     assert!(labels.contains(&line), "{labels:?}");
 }
+
+/// The saved Bookshop exports connection, over its folder.
+fn saved_exports() -> SavedConnection {
+    SavedConnection {
+        id: ConnectionId::new(),
+        name: "Bookshop exports".into(),
+        group: None,
+        environment: Environment::Local,
+        read_only: Some(true),
+        password: PasswordMode::None,
+        ssh_secret: PasswordMode::None,
+        spec: ConnectSpec::files(Source::Folder(FOLDER.into())),
+        agents: Default::default(),
+    }
+}
+
+#[test]
+fn a_connect_macos_refused_says_so_with_privacy_and_security_first() {
+    for look in looks() {
+        let mut harness = Harness::new();
+        harness.set_look(look);
+        let saved = saved_exports();
+        let conn = saved.id.clone();
+        harness.app.connections.upsert(saved);
+        let tab = harness.app.active_tab_id();
+        harness.app.apply(Action::Connect { tab, conn });
+        let Command::Connect {
+            session, request, ..
+        } = *last_sent(&harness.app)
+        else {
+            panic!("expected Connect");
+        };
+        harness.app.apply(Action::Backend(Event::ConnectFailed {
+            session,
+            request,
+            error: Error::Connect(refused()),
+        }));
+        let tree = harness.settle();
+        let labels = crate::testing::labels(&tree);
+        let title = lead(look);
+        assert!(labels.contains(&title), "{}: {labels:?}", look.name);
+        assert!(
+            labels
+                .iter()
+                .any(|label| *label
+                    == look.label("macOS hasn't allowed Tabletist to open this folder.")),
+            "{}: {labels:?}",
+            look.name
+        );
+        // The card's buttons are named as the macOS look shows them.
+        assert!(node(&tree, "Retry", Role::Button).is_some());
+        harness.click("Open Privacy & Security");
+        assert!(opened_privacy(&harness), "{}", look.name);
+    }
+}
````

- [ ] **Step 2: Run it to see it fail**

Run: `~/.cargo/bin/cargo test -q -p tabletist not_allowed`

Expected: `a_connect_macos_refused_says_so_with_privacy_and_security_first` fails. The card still reads "Can't open /data/Bookshop-Exports" and has no Open Privacy & Security. Task 2's five pass.

```text
failures:
---- ui::not_allowed_tests::a_connect_macos_refused_says_so_with_privacy_and_security_first stdout ----
thread 'ui::not_allowed_tests::a_connect_macos_refused_says_so_with_privacy_and_security_first' (884306) panicked at src/ui/not_allowed_tests.rs:291:9:
    ui::not_allowed_tests::a_connect_macos_refused_says_so_with_privacy_and_security_first
test result: FAILED. 5 passed; 1 failed; 0 ignored; 0 measured; 3124 filtered out; finished in 0.05s
error: test failed, to rerun pass `-p tabletist --lib`
```

- [ ] **Step 3: The card's words and its first button**

````diff
diff --git a/src/ui/workspace.rs b/src/ui/workspace.rs
--- a/src/ui/workspace.rs
+++ b/src/ui/workspace.rs
@@ -711,10 +711,24 @@
         SessionStatus::Disconnected(error) => Some(error),
         _ => None,
     };
-    let (icon, title) = failure_title(error, &workspace.spec, say);
-    let sentence = error
-        .map(|error| crate::format::describe_error(locale, error))
-        .unwrap_or_default();
+    // A Files connection's folder macOS's privacy settings refused: the
+    // dialog's words, and the way to where it may be allowed first.
+    let refused = match error {
+        Some(Error::Connect(message)) => {
+            crate::ui::not_allowed::NotAllowed::of(message, &look, locale)
+        }
+        _ => None,
+    };
+    let (icon, title) = match &refused {
+        Some(refused) => (Icon::Lock, refused.lead.clone()),
+        None => failure_title(error, &workspace.spec, say),
+    };
+    let sentence = match &refused {
+        Some(refused) => refused.text.clone(),
+        None => error
+            .map(|error| crate::format::describe_error(locale, error))
+            .unwrap_or_default(),
+    };
     let raw = error.map(ToString::to_string).unwrap_or_default();
     // The settings are what is wrong: changing them comes first.
     let edit_first = matches!(
@@ -766,7 +780,7 @@
             .layout(ui.ctx())
             .label(&mut ui);
     }
-    let (mut retry, mut edit) = (false, false);
+    let (mut retry, mut edit, mut privacy) = (false, false, false);
     let details = format!("{title}\n{raw}");
     ui.horizontal(|ui| {
         let height = states::button_height(&look);
@@ -779,7 +793,16 @@
         fn lead(button: ButtonSpec<'_>, first: bool) -> ButtonSpec<'_> {
             if first { button.primary() } else { button }
         }
-        let retry_button = lead(states::button(&again, &look).label("Retry"), !edit_first);
+        if let Some(refused) = &refused {
+            let open = states::button(&refused.button, &look)
+                .label("Open Privacy & Security")
+                .primary();
+            privacy = open.show(ui, height, &look, &palette).clicked();
+        }
+        let retry_button = lead(
+            states::button(&again, &look).label("Retry"),
+            !edit_first && refused.is_none(),
+        );
         let edit_button = lead(
             states::button(&change, &look).label("Edit connection"),
             edit_first,
@@ -812,6 +835,9 @@
             .label(&mut ui);
     }
     let mut actions = Vec::new();
+    if privacy {
+        actions.push(Action::OpenPrivacySettings);
+    }
     if retry {
         actions.push(Action::Reconnect(tab));
     }
````

- [ ] **Step 4: Run them to see them pass**

Run: `~/.cargo/bin/cargo test -q -p tabletist not_allowed`

Expected: `test result: ok. 6 passed`.

- [ ] **Step 5: Run the full check**

Expected: all pass; 5,663 passed, 0 failed.

- [ ] **Step 6: Commit**

```bash
git add src/ui/workspace.rs src/ui/not_allowed_tests.rs
git commit -m "Say on a Files window's connect card that macOS hasn't allowed the folder, Open Privacy & Security first"
```

### After the tasks

- [ ] **Hand test on a Mac**, with the app bundle (not a bare binary). Point a Files connection at a folder in `~/Documents`. Turn Tabletist off under System Settings › Privacy & Security › Files and Folders › Documents Folder, or deny the prompt. Then quit Tabletist and open it again: a folder picked with Choose… stays open to it until it quits. Or type the path instead of picking it. Then:
  1. Test it: the banner shows, and Open Privacy & Security opens that pane.
  2. Allow it there, then Retry: it reads.
  3. Repeat for a Files list holding one file from `~/Documents` and one from elsewhere. The Test reads the other, and its partial banner says "orders.csv couldn't be read. macOS hasn't allowed Tabletist to open this file." with Open Privacy & Security and Skip. Connecting from the list shows the card, "this file".
  4. Allow it, quit, and open Tabletist again: the connection still reads with no bookmark. This settles the "no bookmarks" decision. If it doesn't, stop and raise it.
  5. Compare with the canvas boards by eye.

  Any screenshot stays local, or is blurred.
- [ ] Push the branch and open a pull request against `main`. Its description lists:
  - the state, and where it shows;
  - why there are no bookmarks;
  - the departures above;
  - the canvas boards;
  - that the hand test on a Mac is still to do, if it is;
  - that the Portuguese wants a native review.
