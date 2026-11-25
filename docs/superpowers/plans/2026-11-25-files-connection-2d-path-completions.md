# Files connection 2d: path completions in the Omarchy look

> **For agentic workers:** REQUIRED SUB-SKILL: Use rubypowers:subagent-driven-development (recommended) or rubypowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** In the Omarchy look, typing a path in a Files connection's Source lists the names it starts under the line, as the prompt asks: "Typing a path offers completions under the field, picked with `ctrl+n/p` and `enter`. Tab still moves to the next field." The folder field lists folders. A new **add** line under the file list lists folders and the files a Files connection reads, and adds the file taken.

**Architecture:**
- **What is listed** (`crates/tabletist-core/src/source_names.rs`, new): `Keep` says which of a folder's names a line offers: every name (the export float's save to line, as before), folders only (the folder field), or folders and the files a Files connection reads (the add line). `ask` reads the folder to list off a typed line, and only for a line with a root (`/` or `~`). `Command::ListFolder` gains `keep`, and the backend filters the folder's names with it before `export_sheet::completions` keeps the first eight.
- **The form** (`ConnectionForm`): the add line's text, the names listed (`files_names`, an `export_sheet::PathList`), the listing on its way (one at a time), and whether Esc put the names away. `Action::SourceNames` changes them; `src/app/source_names.rs` asks, takes and adds, as `src/app/export.rs` does for the save to line. `Event::FolderListed` goes to the form while the connection dialog is open, and to the export float otherwise.
- **The view:**
  - `src/ui/path_list.rs` (new) draws the names under a line and keeps the keyboard on the line. The export float's save to line moves onto it, unchanged.
  - `connect_dialog/files.rs` puts it under the folder field and the new add line.
  - `connect_dialog/mod.rs` reads `ctrl+n`/`ctrl+p` (and Down/Up), Enter and Esc before the form draws, so they come before the form's own Enter and Esc. They are the keys the save to line already has (`PathNext`, `PathPrevious`, `PathPick`, `ClosePathList`).

**Tech Stack:** Rust, egui (`egui_kittest` headless tests), gettext catalogs (`assets/i18n`).

**Base:** `main` at `f20d6cf1`, branch `claude/files-connection-2d-path-completions`. 2c (#385) is merged.

---

## What was decided

On 2026-11-25 Igor picked these, with the recommended option each time:

| Question | Decision |
| --- | --- |
| The Files kind has no field to type a path in | **An add line** under the file list: a path typed there lists folders and the files a Files connection reads, and Enter on a file adds it. The Files board's note asks for typing there. |
| What the folder field lists, and when the folder is read | **Folders only.** Enter on one fills it with its `/` and lists its own folders. The folder is read when the line is left (Tab), as it is today. |

Canvas: **"Omarchy – Files, path completions"** is new: the folder field typing, a folder taken, the line left, the add line typing, a file added, and the keys. The add line is also drawn on "Omarchy – Connection, Files, details and Test" ("source · individual files"). The list looks like the save to line's, on "Omarchy – Components, additions for Export" (canvas version 215). The macOS look doesn't change: it keeps Choose… and Add files….

### Where this plan does more, or less, than the prompt or the canvas

| The prompt or canvas | What is built | Why |
| --- | --- | --- |
| "Typing a path offers completions under the field" | Only a line that starts with `/` or `~` (a root on Windows) lists names. `orders` or `Data/ord` lists nothing. | The connection would read a relative path from wherever the app was started. Drawn on the new board ("folder · left with tab"). |
| "picked with `ctrl+n/p` and `enter`" | Down and Up move too. Esc closes the list, and a second Esc cancels the dialog. A click on a name takes it. | The save to line's keys and clicks, which are bound in the same scope. |
| — | A file the list holds already isn't offered in the add line. | Adding it again would do nothing. Drawn on the new board. |
| — | Enter in the add line with nothing listed adds the path as typed, if it names a file from `/` or `~`. A path that isn't there shows in the list as missing, as a moved file does. A folder's path (ending in `/`) or one with no root adds nothing, and the line keeps it. | Typing a whole path should add it. A held Enter after a folder was taken must not add the folder as a file. Drawn on the new board. |
| — | A file the list holds is found however its path is written (`./`, `/` or `\`, `~`): it isn't added twice. | The chooser and a typed path may spell one file two ways. |
| — | Up and Down in the add line move the file list's cursor while no names are listed, as they do anywhere in the form. While names are listed they move in the names. | The form's arrows, unchanged. |
| — | The footer stays as it is while a list shows. | The board keeps it. |

The new word is "Add" (the add line's label). It has no AI, sparkle or confidence wording, and the UI copy has no em dashes. It's translated into Russian and Brazilian Portuguese; the Portuguese wants a native review.

## Files

| File | Change |
| --- | --- |
| `crates/tabletist-core/src/source_names.rs` (new), `crates/tabletist-core/src/lib.rs`, `src/lib.rs` | `Keep`, `ask`, `moved` |
| `crates/tabletist-core/src/backend.rs`, `src/app/export.rs` | `Command::ListFolder`'s `keep`, and the filter |
| `crates/tabletist-core/src/model.rs` | The form's add line and names, `Action::SourceNames`, `source_line`, `lit_source_name`, `lists_file`, `listed_at`; `set_aside` drops a listing |
| `crates/tabletist-core/src/keymap.rs` | The names' keys say they serve the Files lines too (comments) |
| `src/app.rs`, `src/app/source_names.rs` (new) | The reducers: ask, take, add; `FolderListed` to the form |
| `src/ui/path_list.rs` (new), `src/ui/export.rs`, `src/ui/mod.rs` | The names under a line, shared with the save to line |
| `src/ui/connect_dialog/files.rs`, `src/ui/connect_dialog/mod.rs` | The folder field's names, the add line, the list's keys |
| `src/ui/source_names_tests.rs` (new) | Headless tests of both lines |
| `assets/i18n/*.po`, `*.pot` | "Add" |
| `docs/_reference/connections.md`, `docs/_guide/omarchy.md` | The completions and their keys |

Every commit passes these:
- `cargo fmt --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- the headless tests
- `RUSTDOCFLAGS='-D warnings' cargo doc --workspace --no-deps`
- `sh assets/i18n/update.sh --check`

Run cargo as `~/.cargo/bin/cargo` with `CARGO_TARGET_DIR=$PWD/target`. For the server tests, point `TABLETIST_TEST_PG_URL` and `TABLETIST_TEST_MYSQL_URL` at the test servers when they're up (`ss -ltn` shows 55432 and 53306). Commits are signed: one for this plan, then one per task.

---

### Task 1: Which names a line offers

**Files:**
- Create: `crates/tabletist-core/src/source_names.rs`
- Modify: `crates/tabletist-core/src/lib.rs`, `src/lib.rs`, `crates/tabletist-core/src/backend.rs`, `src/app/export.rs`

- [ ] **Step 1: Write the failing tests**

The module starts with its tests. The backend's test sends `keep` with each listing, and a new one lists a folder of the Bookshop's returns both ways.

````diff
diff --git a/crates/tabletist-core/src/backend.rs b/crates/tabletist-core/src/backend.rs
--- a/crates/tabletist-core/src/backend.rs
+++ b/crates/tabletist-core/src/backend.rs
@@ -4120,12 +4120,13 @@
             std::fs::write(dir.path().join(name), "").unwrap();
         }
         let mut backend = Backend::start_with(Waker::default(), Keyring::memory());
-        let mut ask = |start: &str, folder: PathBuf| {
+        let mut ask_keeping = |start: &str, folder: PathBuf, keep| {
             backend.send(Command::ListFolder {
                 asked: 7,
                 typed: format!("~/x/{start}"),
                 folder,
                 start: start.to_owned(),
+                keep,
             });
             match backend.wait(WAIT) {
                 Some(Event::FolderListed {
@@ -4141,6 +4142,9 @@
                 }
                 other => panic!("expected the names, got {other:?}"),
             }
+        };
+        let mut ask = |start: &str, folder: PathBuf| {
+            ask_keeping(start, folder, crate::source_names::Keep::All)
         };
         let owned = |names: &[(&str, bool)]| {
             names
@@ -4173,6 +4177,42 @@
                 owned(&[("linked", true), ("link.csv", false)])
             );
         }
+    }
+
+    /// A Files connection's lines: its folder field lists folders, its
+    /// add line folders and the files a Files connection reads.
+    #[test]
+    fn a_files_line_lists_only_what_it_takes() {
+        use crate::source_names::Keep;
+        let dir = tempfile::tempdir().unwrap();
+        std::fs::create_dir(dir.path().join("returns-2025")).unwrap();
+        for name in ["returns.csv", "returns.xlsx", "returns-notes.txt"] {
+            std::fs::write(dir.path().join(name), "").unwrap();
+        }
+        let mut backend = Backend::start_with(Waker::default(), Keyring::memory());
+        let mut ask = |keep| {
+            backend.send(Command::ListFolder {
+                asked: 8,
+                typed: "/data/ret".into(),
+                folder: dir.path().to_path_buf(),
+                start: "ret".into(),
+                keep,
+            });
+            match backend.wait(WAIT) {
+                Some(Event::FolderListed {
+                    asked: 8, names, ..
+                }) => names
+                    .into_iter()
+                    .map(|listed| listed.name)
+                    .collect::<Vec<_>>(),
+                other => panic!("expected the names, got {other:?}"),
+            }
+        };
+        assert_eq!(ask(Keep::Folders), ["returns-2025"]);
+        assert_eq!(
+            ask(Keep::Readable),
+            ["returns-2025", "returns.csv", "returns.xlsx"]
+        );
     }
 
     #[test]
diff --git a/crates/tabletist-core/src/lib.rs b/crates/tabletist-core/src/lib.rs
--- a/crates/tabletist-core/src/lib.rs
+++ b/crates/tabletist-core/src/lib.rs
@@ -38,6 +38,7 @@
 pub mod selection;
 pub mod session;
 pub mod settings;
+pub mod source_names;
 pub mod theme;
 pub mod update;
 pub mod util;
diff --git a/crates/tabletist-core/src/source_names.rs b/crates/tabletist-core/src/source_names.rs
new file mode 100644
--- /dev/null
+++ b/crates/tabletist-core/src/source_names.rs
@@ -0,0 +1,79 @@
+//! The names a Files connection's path lines offer while a path is typed,
+//! in the terminal look: under the folder field, its folders; under the
+//! file list's add line, its folders and the files a Files connection
+//! reads. They are listed as the export float's save to line lists its
+//! own ([`crate::export_sheet::completions`]). No disk is read here.
+
+
+#[cfg(test)]
+mod tests {
+    use super::*;
+    use crate::export_sheet::Listed;
+
+    #[test]
+    fn a_line_asks_for_its_folder_when_it_is_absolute() {
+        let home = Some(Path::new("/home/reader"));
+        assert_eq!(
+            ask("/data/book", home),
+            Some((PathBuf::from("/data/"), "book".to_owned()))
+        );
+        assert_eq!(
+            ask("~/Data/bookshop-exports/", home),
+            Some((
+                PathBuf::from("/home/reader/Data/bookshop-exports"),
+                String::new()
+            ))
+        );
+        assert_eq!(
+            ask("  ~/Data/ret ", home),
+            Some((PathBuf::from("/home/reader/Data"), "ret".to_owned()))
+        );
+        // No folder, or not one the connection reads from here.
+        assert_eq!(ask("orders", home), None);
+        assert_eq!(ask("Data/ord", home), None);
+        assert_eq!(ask("~reader/Data/", home), None);
+        assert_eq!(ask("~/Data/", None), None);
+        assert_eq!(ask("", home), None);
+    }
+
+    #[test]
+    fn each_line_keeps_its_own_names() {
+        assert!(Keep::All.keeps("notes.txt", false));
+        assert!(Keep::Folders.keeps("2025", true));
+        assert!(!Keep::Folders.keeps("orders.csv", false));
+        assert!(Keep::Readable.keeps("archive", true));
+        for name in [
+            "orders.csv",
+            "returns.TSV",
+            "customers.parquet",
+            "events.ndjson",
+            "events.jsonl",
+            "authors.json",
+            "stock.xlsx",
+            "shop.duckdb",
+        ] {
+            assert!(Keep::Readable.keeps(name, false), "{name}");
+        }
+        assert!(!Keep::Readable.keeps("returns-notes.txt", false));
+        assert!(!Keep::Readable.keeps("orders", false));
+    }
+
+    #[test]
+    fn the_lit_name_moves_round() {
+        let name = |name: &str| Listed {
+            name: name.into(),
+            folder: false,
+        };
+        let mut list = PathList {
+            line: "/data/ret".into(),
+            names: vec![name("returns.csv"), name("returns.xlsx")],
+            lit: 0,
+        };
+        moved(&mut list, 1);
+        assert_eq!(list.lit, 1);
+        moved(&mut list, 1);
+        assert_eq!(list.lit, 0);
+        moved(&mut list, -1);
+        assert_eq!(list.lit, 1);
+    }
+}
diff --git a/src/lib.rs b/src/lib.rs
--- a/src/lib.rs
+++ b/src/lib.rs
@@ -25,5 +25,5 @@
     agents_shown, backend, clock, completion, connections, dates, desktop_theme, edit, endpoint,
     env, export, export_sheet, font_files, format, history, i18n, json, key_picker, keymap,
     known_hosts, labels, managed, model, paths, recents, relay, review, search, secrets, selection,
-    session, settings, theme, update, util, vim, zones,
+    session, settings, source_names, theme, update, util, vim, zones,
 };
````

- [ ] **Step 2: Run them to see them fail**

Run: `~/.cargo/bin/cargo test -q -p tabletist-core --lib source_names`

Expected: the tests don't compile: `Keep`, `ask` and `moved` aren't there yet, nor is `ListFolder`'s `keep`.

```text
error[E0432]: unresolved import `crate::source_names::Keep`
error[E0433]: cannot find `Keep` in `source_names`
error[E0433]: cannot find type `Path` in this scope
error[E0433]: cannot find type `PathBuf` in this scope
error[E0422]: cannot find struct, variant or union type `PathList` in this scope
error[E0425]: cannot find function `moved` in this scope
error[E0559]: variant `backend::Command::ListFolder` has no field named `keep`
error[E0425]: cannot find function `ask` in this scope
error[E0433]: cannot find type `Keep` in this scope
error: could not compile `tabletist-core` (lib test) due to 27 previous errors; 1 warning emitted
```

- [ ] **Step 3: `Keep`, `ask` and the filter**

````diff
diff --git a/crates/tabletist-core/src/backend.rs b/crates/tabletist-core/src/backend.rs
--- a/crates/tabletist-core/src/backend.rs
+++ b/crates/tabletist-core/src/backend.rs
@@ -302,16 +302,20 @@
         summary: String,
         body: String,
     },
-    /// The names of `folder` that start with `start`, for the export
-    /// float's path line `typed` (spec 4, S15): read off the UI thread, a
-    /// slow or network disk taking what it takes. Answered with
-    /// [`Event::FolderListed`], no names where the folder cannot be read.
+    /// The names of `folder` that start with `start`, for a path line
+    /// `typed`: the export float's (spec 4, S15) or a Files connection's in
+    /// the terminal look. Read off the UI thread, a slow or network disk
+    /// taking what it takes. Answered with [`Event::FolderListed`], no
+    /// names where the folder cannot be read.
     ListFolder {
-        /// The sheet's (`ExportSheet::asked`).
+        /// The sheet's (`ExportSheet::asked`), or the connection form's
+        /// listing (`ConnectionForm::files_listing`).
         asked: u64,
         typed: String,
         folder: PathBuf,
         start: String,
+        /// The names the line offers.
+        keep: crate::source_names::Keep,
     },
     /// Signals `done` once every save sent before it is on disk.
     Flush {
@@ -2667,10 +2671,11 @@
                 typed,
                 folder,
                 start,
+                keep,
             } => {
                 let outbox = self.outbox.clone();
                 tokio::task::spawn_blocking(move || {
-                    let names = list_folder(&folder, &start);
+                    let names = list_folder(&folder, &start, keep);
                     outbox.emit(Event::FolderListed {
                         asked,
                         typed,
@@ -2769,9 +2774,14 @@
 }
 
 /// The names of `folder` the path line offers for `start` (see
-/// [`crate::export_sheet::completions`]); none where it cannot be read. A
-/// name that is not UTF-8 is left out: the line could not write it.
-fn list_folder(folder: &Path, start: &str) -> Vec<crate::export_sheet::Listed> {
+/// [`crate::export_sheet::completions`]), of those `keep` keeps; none
+/// where it cannot be read. A name that is not UTF-8 is left out: the
+/// line could not write it.
+fn list_folder(
+    folder: &Path,
+    start: &str,
+    keep: crate::source_names::Keep,
+) -> Vec<crate::export_sheet::Listed> {
     let Ok(entries) = std::fs::read_dir(folder) else {
         return Vec::new();
     };
@@ -2784,7 +2794,8 @@
             .is_ok_and(|kind| kind.is_dir() || (kind.is_symlink() && entry.path().is_dir()));
         Some((name, folder))
     });
-    crate::export_sheet::completions(names, start)
+    let kept = names.filter(|(name, folder)| keep.keeps(name, *folder));
+    crate::export_sheet::completions(kept, start)
 }
 
 /// The requests a queued command answers: one, or none, or the two of an
diff --git a/crates/tabletist-core/src/source_names.rs b/crates/tabletist-core/src/source_names.rs
--- a/crates/tabletist-core/src/source_names.rs
+++ b/crates/tabletist-core/src/source_names.rs
@@ -4,6 +4,57 @@
 //! reads. They are listed as the export float's save to line lists its
 //! own ([`crate::export_sheet::completions`]). No disk is read here.
 
+use std::path::{MAIN_SEPARATOR, Path, PathBuf};
+
+use crate::export_sheet::PathList;
+
+/// Which of a folder's names a path line offers.
+#[derive(Debug, Clone, Copy, PartialEq, Eq)]
+pub enum Keep {
+    /// Every name: the export float's save to line.
+    All,
+    /// Folders only: a Files connection's folder field.
+    Folders,
+    /// Folders, and the files a Files connection reads: the add line.
+    Readable,
+}
+
+impl Keep {
+    /// Whether the name, a folder's or a file's, is offered.
+    pub fn keeps(self, name: &str, folder: bool) -> bool {
+        match self {
+            Self::All => true,
+            Self::Folders => folder,
+            Self::Readable => folder || tabletist_db::files::Format::of(Path::new(name)).is_some(),
+        }
+    }
+}
+
+/// What a Files path line `typed` asks to list: the folder its last part
+/// is in, `~` being `home`, and that part. `None` for a line with no
+/// folder, or one with no root (`Data/ord`): the connection would read it
+/// from wherever the app was started. A root is enough, with no drive
+/// (`/data` on Windows): the connection reads it on the current drive.
+pub fn ask(typed: &str, home: Option<&Path>) -> Option<(PathBuf, String)> {
+    let typed = typed.trim();
+    let at = typed.rfind(['/', MAIN_SEPARATOR])? + 1;
+    let (folder, start) = typed.split_at(at);
+    let folder = match folder.strip_prefix('~') {
+        Some(rest) if rest.starts_with(['/', MAIN_SEPARATOR]) => home?.join(&rest[1..]),
+        Some(_) => return None,
+        None => PathBuf::from(folder),
+    };
+    folder.has_root().then(|| (folder, start.to_owned()))
+}
+
+/// `list`'s lit name moved by `by`, round.
+pub fn moved(list: &mut PathList, by: isize) {
+    if list.names.is_empty() {
+        return;
+    }
+    let lit = list.lit.min(list.names.len() - 1) as isize;
+    list.lit = (lit + by).rem_euclid(list.names.len() as isize) as usize;
+}
 
 #[cfg(test)]
 mod tests {
diff --git a/src/app/export.rs b/src/app/export.rs
--- a/src/app/export.rs
+++ b/src/app/export.rs
@@ -696,6 +696,7 @@
             typed: sheet.path.clone(),
             folder,
             start,
+            keep: crate::source_names::Keep::All,
         };
         self.backend.send(command);
         if let Some(Dialog::Export(sheet)) = &mut self.dialog {
````

- [ ] **Step 4: Run them to see them pass**

Run: `~/.cargo/bin/cargo test -q -p tabletist-core --lib source_names`, then `~/.cargo/bin/cargo test -q -p tabletist-core --lib folder`.

Expected: `a_line_asks_for_its_folder_when_it_is_absolute`, `each_line_keeps_its_own_names` and `the_lit_name_moves_round` pass. So do `a_files_line_lists_only_what_it_takes` and `a_folder_is_listed_off_the_ui_thread`. The save to line's tests are unchanged: `~/.cargo/bin/cargo test -q -p tabletist --lib export_path` passes.

- [ ] **Step 5: Run the full check**

Expected: all pass; 5,671 passed, 0 failed.

- [ ] **Step 6: Commit**

```bash
git add crates/tabletist-core/src/source_names.rs crates/tabletist-core/src/lib.rs src/lib.rs crates/tabletist-core/src/backend.rs src/app/export.rs
git commit -m "Let a folder listing keep only folders, or the files a Files connection reads"
```

### Task 2: The folder field lists its folders

**Files:**
- Create: `src/app/source_names.rs`, `src/ui/path_list.rs`, `src/ui/source_names_tests.rs`
- Modify: `crates/tabletist-core/src/model.rs`, `crates/tabletist-core/src/keymap.rs`, `src/app.rs`, `src/ui/export.rs`, `src/ui/mod.rs`, `src/ui/connect_dialog/mod.rs`, `src/ui/connect_dialog/files.rs`

The reducers cover both lines here. The add line's view and its tests come in Task 3. A form set aside while its Delete is asked about (`set_aside`) drops the listing it waited for, as it drops its file dialog: its answer is dropped while the form is held, and the form would otherwise wait for it for good.

- [ ] **Step 1: Write the failing tests**

The helpers open a new Files connection's dialog in the Omarchy look, the keyboard on its path line and the home folder `/home/reader`. They answer each listing as the backend would.

````diff
diff --git a/src/ui/mod.rs b/src/ui/mod.rs
--- a/src/ui/mod.rs
+++ b/src/ui/mod.rs
@@ -146,6 +146,8 @@
 #[cfg(test)]
 mod settings_access_tests;
 pub mod sidebar;
+#[cfg(test)]
+mod source_names_tests;
 pub mod sql_complete;
 pub mod sql_editor;
 pub mod sql_results;
diff --git a/src/ui/source_names_tests.rs b/src/ui/source_names_tests.rs
new file mode 100644
--- /dev/null
+++ b/src/ui/source_names_tests.rs
@@ -0,0 +1,268 @@
+//! A Files connection's path lines in the terminal look (the "Omarchy –
+//! Files, path completions" board): typing a path lists the names it
+//! starts under the line, `ctrl+n`/`ctrl+p` move in them, Enter takes
+//! one and Esc puts them away. The folder field lists folders; the add
+//! line under the file list lists folders and the files a Files
+//! connection reads, and adds the file taken.
+//!
+//! The backend is a recording one: the names a folder holds are given, as
+//! its answer would give them. No picture of the design is compared.
+
+use std::path::PathBuf;
+
+use egui::{Key, Modifiers};
+
+use crate::backend::{Command, Event};
+use crate::export_sheet::Listed;
+use crate::model::{Action, ConnectionForm, Dialog, FilesKind};
+use crate::source_names::Keep;
+use crate::testing::Harness;
+use crate::theme::Look;
+
+fn form(harness: &Harness) -> &ConnectionForm {
+    match &harness.app.dialog {
+        Some(Dialog::Connection(form)) => form,
+        other => panic!("no connection dialog: {:?}", other.is_some()),
+    }
+}
+
+/// A new Files connection's dialog in the terminal look, its `kind` of
+/// Source chosen and the keyboard on its path line; the home folder the
+/// test's own.
+fn dialog(kind: FilesKind) -> Harness {
+    let mut harness = Harness::new();
+    harness.set_look(Look::omarchy());
+    harness.app.dirs.home = Some(PathBuf::from("/home/reader"));
+    harness.app.apply(Action::NewConnection);
+    harness.settle();
+    harness.click(&Look::omarchy().label("Files"));
+    harness.app.apply(Action::SetFilesKind(kind));
+    harness.settle();
+    let id = crate::ui::connect_dialog::source_line_id(kind);
+    harness.ctx.memory_mut(|memory| memory.request_focus(id));
+    harness.settle();
+    harness
+}
+
+/// The line, as typed into its field: what it held goes first.
+fn type_line(harness: &mut Harness, line: &str) {
+    harness.frame(vec![
+        crate::testing::key(Key::A, Modifiers::COMMAND),
+        egui::Event::Text(line.into()),
+    ]);
+    harness.settle();
+}
+
+/// The folder listings asked so far: the line each was for, its folder,
+/// its last part and the names it keeps.
+fn asks(harness: &Harness) -> Vec<(String, PathBuf, String, Keep)> {
+    harness
+        .app
+        .backend
+        .sent
+        .iter()
+        .filter_map(|command| match command {
+            Command::ListFolder {
+                typed,
+                folder,
+                start,
+                keep,
+                ..
+            } => Some((typed.clone(), folder.clone(), start.clone(), *keep)),
+            _ => None,
+        })
+        .collect()
+}
+
+/// The backend's answer for the listing on its way, of the line `typed`.
+fn answer(harness: &mut Harness, typed: &str, names: &[(&str, bool)]) {
+    let asked = form(harness).files_listing.expect("a listing on its way");
+    let names = names
+        .iter()
+        .map(|(name, folder)| Listed {
+            name: (*name).to_owned(),
+            folder: *folder,
+        })
+        .collect();
+    harness.app.apply(Action::Backend(Event::FolderListed {
+        asked,
+        typed: typed.to_owned(),
+        names,
+    }));
+    harness.settle();
+}
+
+fn painted(harness: &Harness, text: &str) -> bool {
+    harness.painted.iter().any(|(painted, _)| painted == text)
+}
+
+fn lit(harness: &Harness) -> usize {
+    form(harness)
+        .files_names
+        .as_ref()
+        .expect("names listed")
+        .lit
+}
+
+fn on_line(harness: &Harness) -> bool {
+    let id = crate::ui::connect_dialog::source_line_id(form(harness).files_kind);
+    harness.ctx.memory(|memory| memory.has_focus(id))
+}
+
+#[test]
+fn the_folder_field_lists_the_folders_it_starts() {
+    let mut harness = dialog(FilesKind::Folder);
+    type_line(&mut harness, "~/Data/book");
+    assert_eq!(
+        asks(&harness).last(),
+        Some(&(
+            "~/Data/book".to_owned(),
+            PathBuf::from("/home/reader/Data"),
+            "book".to_owned(),
+            Keep::Folders
+        ))
+    );
+    assert!(
+        form(&harness).files_names.is_none(),
+        "nothing until it answers"
+    );
+    answer(
+        &mut harness,
+        "~/Data/book",
+        &[("bookshop-exports", true), ("books-archive", true)],
+    );
+    assert!(
+        painted(&harness, "bookshop-exports/"),
+        "{:?}",
+        harness.painted
+    );
+    assert!(painted(&harness, "books-archive/"));
+    // Nothing matches: no list.
+    type_line(&mut harness, "~/Data/zz");
+    answer(&mut harness, "~/Data/zz", &[]);
+    assert!(form(&harness).files_names.is_none());
+}
+
+/// A line with no folder, or one not absolute, lists nothing: the
+/// connection would read it from wherever the app was started.
+#[test]
+fn a_line_that_is_not_absolute_lists_nothing() {
+    let mut harness = dialog(FilesKind::Folder);
+    for line in ["book", "Data/book", ""] {
+        type_line(&mut harness, line);
+    }
+    assert!(asks(&harness).is_empty(), "{:?}", asks(&harness));
+}
+
+/// One listing at a time: a line typed meanwhile is listed once the
+/// first answers, and that answer, for a line since changed, is dropped.
+#[test]
+fn an_answer_for_an_old_line_is_dropped() {
+    let mut harness = dialog(FilesKind::Folder);
+    type_line(&mut harness, "/data/b");
+    type_line(&mut harness, "/data/bo");
+    let typed: Vec<String> = asks(&harness).into_iter().map(|ask| ask.0).collect();
+    assert_eq!(typed, ["/data/b"]);
+    answer(&mut harness, "/data/b", &[("backups", true)]);
+    assert!(form(&harness).files_names.is_none(), "the old line's");
+    assert!(!painted(&harness, "backups/"));
+    assert_eq!(
+        asks(&harness).last().map(|ask| ask.0.as_str()),
+        Some("/data/bo"),
+        "then the line as it is"
+    );
+    answer(&mut harness, "/data/bo", &[("bookshop-exports", true)]);
+    assert!(painted(&harness, "bookshop-exports/"));
+}
+
+/// `ctrl+n` and `ctrl+p` (↓ ↑) move in the names, round; Enter takes the
+/// lit folder with its `/` and lists its own folders. The folder is not
+/// read until the line is left.
+#[test]
+fn enter_takes_a_folder_and_lists_its_own() {
+    let mut harness = dialog(FilesKind::Folder);
+    type_line(&mut harness, "~/Data/book");
+    answer(
+        &mut harness,
+        "~/Data/book",
+        &[("books-archive", true), ("bookshop-exports", true)],
+    );
+    harness.press(Key::N, Modifiers::CTRL);
+    assert_eq!(lit(&harness), 1);
+    harness.press(Key::ArrowDown, Modifiers::NONE);
+    assert_eq!(lit(&harness), 0, "round");
+    harness.press(Key::P, Modifiers::CTRL);
+    assert_eq!(lit(&harness), 1);
+    let read = harness.app.backend.sources_read.len();
+    harness.press(Key::Enter, Modifiers::NONE);
+    assert_eq!(form(&harness).files_folder, "~/Data/bookshop-exports/");
+    assert_eq!(
+        asks(&harness).last(),
+        Some(&(
+            "~/Data/bookshop-exports/".to_owned(),
+            PathBuf::from("/home/reader/Data/bookshop-exports"),
+            String::new(),
+            Keep::Folders
+        ))
+    );
+    assert!(on_line(&harness), "the line keeps the keyboard");
+    assert_eq!(harness.app.backend.sources_read.len(), read, "nor is read");
+    assert!(matches!(harness.app.dialog, Some(Dialog::Connection(_))));
+}
+
+/// Esc puts the names away and leaves the dialog open; the next Esc
+/// cancels it. The names stay away until the line changes.
+#[test]
+fn esc_puts_the_names_away_then_cancels() {
+    let mut harness = dialog(FilesKind::Folder);
+    type_line(&mut harness, "/data/book");
+    answer(&mut harness, "/data/book", &[("bookshop-exports", true)]);
+    harness.press(Key::Escape, Modifiers::NONE);
+    assert!(form(&harness).files_names.is_none());
+    assert!(form(&harness).files_names_closed);
+    assert!(on_line(&harness), "the line keeps the keyboard");
+    harness.press(Key::Escape, Modifiers::NONE);
+    assert!(harness.app.dialog.is_none(), "the second Esc cancels");
+}
+
+/// Tab moves to the next field, as always: the names go, and the folder
+/// is read as it was before.
+#[test]
+fn tab_leaves_the_line_and_reads_the_folder() {
+    let mut harness = dialog(FilesKind::Folder);
+    type_line(&mut harness, "/data/book");
+    answer(&mut harness, "/data/book", &[("bookshop-exports", true)]);
+    let read = harness.app.backend.sources_read.len();
+    harness.press(Key::Tab, Modifiers::NONE);
+    harness.settle();
+    assert!(!on_line(&harness));
+    assert!(form(&harness).files_names.is_none());
+    assert_eq!(
+        form(&harness).files_folder,
+        "/data/book",
+        "Tab takes nothing"
+    );
+    let sources = &harness.app.backend.sources_read;
+    assert!(sources.len() > read, "the folder is read");
+    assert_eq!(
+        sources.last(),
+        Some(&tabletist_db::files::Source::Folder("/data/book".into()))
+    );
+}
+
+/// A click on a name takes it, as Enter does, and reads nothing.
+#[test]
+fn a_click_takes_a_name() {
+    let mut harness = dialog(FilesKind::Folder);
+    type_line(&mut harness, "/data/book");
+    answer(
+        &mut harness,
+        "/data/book",
+        &[("books-archive", true), ("bookshop-exports", true)],
+    );
+    let read = harness.app.backend.sources_read.len();
+    harness.click("bookshop-exports/");
+    harness.settle();
+    assert_eq!(form(&harness).files_folder, "/data/bookshop-exports/");
+    assert_eq!(harness.app.backend.sources_read.len(), read);
+}
````

- [ ] **Step 2: Run them to see them fail**

Run: `~/.cargo/bin/cargo test -q -p tabletist --lib source_names`

Expected: the tests don't compile: there's no `source_line_id`, and the form has no names.

```text
error[E0425]: cannot find function `source_line_id` in module `crate::ui::connect_dialog`
    |
error[E0609]: no field `files_listing` on type `&tabletist_core::model::ConnectionForm`
error[E0609]: no field `files_names` on type `&tabletist_core::model::ConnectionForm`
error[E0609]: no field `files_names_closed` on type `&tabletist_core::model::ConnectionForm`
error: could not compile `tabletist` (lib test) due to 10 previous errors
```

- [ ] **Step 3: The form's names, the reducers, the shared list and the keys**

`src/ui/export.rs` loses its own drawing of the save to line's list, and calls `path_list::under` and `path_list::to_line` instead.

````diff
diff --git a/crates/tabletist-core/src/keymap.rs b/crates/tabletist-core/src/keymap.rs
--- a/crates/tabletist-core/src/keymap.rs
+++ b/crates/tabletist-core/src/keymap.rs
@@ -283,8 +283,9 @@
     /// The terminal look's `:export cancel` asks which of the exports
     /// running to cancel.
     CancelExports,
-    /// The float's save to line has the keyboard, with the names it
-    /// starts listed under it.
+    /// A path line has the keyboard, with the names it starts listed
+    /// under it: the float's save to line, or a Files connection's folder
+    /// field or add line in the terminal look.
     PathListed,
     /// A button of a toast has the keyboard (⌘⇧' put it there).
     ToastFocused,
@@ -2263,7 +2264,8 @@
         bind(C::CancelExportPick, PROMPT, NONE, keys(&[k("enter")])).when(When::CancelExports),
         bind(C::CloseCancelExport, PROMPT, NONE, keys(&[k("esc")])).when(When::CancelExports),
         // The float's save to line (spec 4, S15 and S16): the system's
-        // dialog, and the names the line starts.
+        // dialog, and the names the line starts. A Files connection's path
+        // lines take the names' keys too.
         bind(C::ExportPickFile, PROMPT, NONE, keys(&[k("ctrl+o")])).when(When::Export),
         bind(C::PathNext, PROMPT, NONE, keys(&[k("ctrl+n"), k("down")])).when(When::PathListed),
         bind(C::PathPrevious, PROMPT, NONE, keys(&[k("ctrl+p"), k("up")])).when(When::PathListed),
diff --git a/crates/tabletist-core/src/model.rs b/crates/tabletist-core/src/model.rs
--- a/crates/tabletist-core/src/model.rs
+++ b/crates/tabletist-core/src/model.rs
@@ -134,6 +134,9 @@
     /// Read what a Files connection's Source holds again, for the found
     /// line: after its folder is typed.
     ReadSource,
+    /// The names listed under a Files connection's path line, in the
+    /// terminal look: the folder field's, or the file list's add line's.
+    SourceNames(SourceNames),
     /// Put this ~/.ssh/config Host alias in the SSH host field.
     PickSshHost(String),
     /// Ask an open window of the saved connection for the columns of these
@@ -2186,6 +2189,23 @@
     pub files_list: Vec<String>,
     /// Files: the row of the list the keyboard is on, by its place.
     pub files_cursor: Option<usize>,
+    /// Files, terminal look: the line under the list a file's path is
+    /// typed in, to add it.
+    pub files_add: String,
+    /// Files, terminal look: the names listed under the path line being
+    /// typed in (the folder field, or the add line), for the line they
+    /// were listed for.
+    pub files_names: Option<crate::export_sheet::PathList>,
+    /// Files: the listing on its way, by its ask. One at a time: a line
+    /// typed meanwhile is listed once it answers.
+    pub files_listing: Option<u64>,
+    /// Files: the names put away (Escape) for the line as it is, until it
+    /// changes.
+    pub files_names_closed: bool,
+    /// Asks the view, once for each new value, to give the path line the
+    /// keyboard with its cursor at the end: after a name is taken from
+    /// its list.
+    pub files_to_line: u64,
     /// Files: what the Source holds, as the backend last read it, for the
     /// found line and the list's rows.
     pub files_found: FilesFound,
@@ -2228,6 +2248,22 @@
     Folder,
     /// Files added to a Files connection's list.
     Files,
+}
+
+/// A change to the names listed under a Files connection's path line.
+#[derive(Debug, Clone, Copy, PartialEq, Eq)]
+pub enum SourceNames {
+    /// The line was typed in: the names it starts, asked for anew.
+    Typed,
+    /// The lit name moved by, round.
+    Move(isize),
+    /// The name at the place taken, else the lit one: a folder's lists
+    /// its own names next, a file's is added to the list.
+    Take(Option<usize>),
+    /// The names put away until the line changes.
+    Close,
+    /// The add line's Enter with no name listed: the path as typed added.
+    Add,
 }
 
 /// A fix the partial Test offers for a file it could not read.
@@ -2435,6 +2471,11 @@
             files_folder: String::new(),
             files_list: Vec::new(),
             files_cursor: None,
+            files_add: String::new(),
+            files_names: None,
+            files_listing: None,
+            files_names_closed: false,
+            files_to_line: 0,
             files_found: FilesFound::Unread,
             files_tested: None,
             files_check: None,
@@ -2477,6 +2518,10 @@
         }
         self.pick_request = None;
         self.ssh_hosts_request = None;
+        // Nor is a folder's listing answered: the names start anew.
+        self.files_listing = None;
+        self.files_names = None;
+        self.files_names_closed = false;
     }
 
     /// Applies a change made in the Reading CSV section. A Test read with
@@ -2510,6 +2555,39 @@
         self.test_waiting = 0;
         self.files_tested = None;
         self.files_check = None;
+    }
+
+    /// Files, terminal look: the path line names are typed into, as the
+    /// Source's kind has it (the folder field, or the list's add line),
+    /// and the names it offers.
+    pub fn source_line(&self) -> (&str, crate::source_names::Keep) {
+        match self.files_kind {
+            FilesKind::Folder => (&self.files_folder, crate::source_names::Keep::Folders),
+            FilesKind::Files => (&self.files_add, crate::source_names::Keep::Readable),
+        }
+    }
+
+    /// The name lit under the path line, if one is listed for the line as
+    /// it is.
+    pub fn lit_source_name(&self) -> Option<&crate::export_sheet::Listed> {
+        let (line, _) = self.source_line();
+        let list = self.files_names.as_ref().filter(|list| list.line == line)?;
+        list.names
+            .get(list.lit.min(list.names.len().saturating_sub(1)))
+    }
+
+    /// Whether the list holds the file `path` names, however it is
+    /// written (`~` or the home folder's path, `/` or `\`, `./`).
+    pub fn lists_file(&self, path: &str) -> bool {
+        self.listed_at(path).is_some()
+    }
+
+    /// Where the list holds the file `path` names, however it is written.
+    pub fn listed_at(&self, path: &str) -> Option<usize> {
+        let path = tabletist_db::files::expanded(Path::new(path.trim()));
+        self.files_list
+            .iter()
+            .position(|listed| tabletist_db::files::expanded(Path::new(listed.trim())) == path)
     }
 
     /// The first file the last Test could not read, which the partial
@@ -9378,6 +9456,27 @@
         assert!(!before.is_on(&completion_site(""), print("susers")));
     }
 
+    /// A form set aside (its Delete asked about) gets no answer to the
+    /// listing it waited for: once it is back, its path lines list anew.
+    #[test]
+    fn a_form_set_aside_waits_for_no_listing() {
+        let mut form = ConnectionForm {
+            driver: Driver::Files,
+            files_listing: Some(9),
+            files_names: Some(crate::export_sheet::PathList {
+                line: "/data/book".into(),
+                names: Vec::new(),
+                lit: 0,
+            }),
+            files_names_closed: true,
+            ..ConnectionForm::default()
+        };
+        form.set_aside();
+        assert_eq!(form.files_listing, None);
+        assert!(form.files_names.is_none());
+        assert!(!form.files_names_closed);
+    }
+
     /// A fix adds the override for the first file the Test could not read
     /// and forgets the Test, so it runs again.
     #[test]
diff --git a/src/app.rs b/src/app.rs
--- a/src/app.rs
+++ b/src/app.rs
@@ -40,6 +40,7 @@
 mod labels;
 mod menu;
 mod quick_open;
+mod source_names;
 #[cfg(test)]
 mod startup_tests;
 mod updates;
@@ -3525,6 +3526,8 @@
             Action::SetFilesKind(kind) => {
                 if let Some(Dialog::Connection(form)) = &mut self.dialog {
                     form.files_kind = kind;
+                    // The names were the other line's.
+                    form.files_names = None;
                     self.read_form_source();
                 }
             }
@@ -3548,6 +3551,7 @@
                 }
             }
             Action::ReadSource => self.read_form_source(),
+            Action::SourceNames(change) => self.source_names(change),
             Action::PickSshHost(alias) => {
                 if let Some(Dialog::Connection(form)) = &mut self.dialog {
                     form.pick_ssh_host(&alias);
@@ -4606,11 +4610,16 @@
                 self.export_path_answered(job, path, replace)
             }
             Event::Export { job, state } => self.export_state(job, state),
+            // The export float's line, or a Files connection's: one dialog
+            // is open at a time.
             Event::FolderListed {
                 asked,
                 typed,
                 names,
-            } => self.folder_listed(asked, &typed, names),
+            } => match &self.dialog {
+                Some(Dialog::Connection(_)) => self.source_listed(asked, &typed, names),
+                _ => self.folder_listed(asked, &typed, names),
+            },
             Event::Connected {
                 session,
                 request,
@@ -5588,7 +5597,7 @@
 
     /// Has the backend read what the open dialog's Files Source holds, for
     /// its found line. A Source not chosen yet is not read.
-    fn read_form_source(&mut self) {
+    pub(super) fn read_form_source(&mut self) {
         let request = RequestId(self.next_id());
         let Some(Dialog::Connection(form)) = &mut self.dialog else {
             return;
diff --git a/src/app/source_names.rs b/src/app/source_names.rs
new file mode 100644
--- /dev/null
+++ b/src/app/source_names.rs
@@ -0,0 +1,183 @@
+//! The names a Files connection's path lines list as a path is typed, in
+//! the terminal look: the folder field's folders, and the add line's
+//! folders and files. Asked for off this thread, as the export float's
+//! save to line asks for its own (`export.rs`).
+
+use std::path::Path;
+
+use super::App;
+use crate::backend::Command;
+use crate::export_sheet::{Listed, PathList, complete};
+use crate::model::{Dialog, FilesKind, SourceNames};
+use tabletist_db::Driver;
+
+impl App {
+    pub(super) fn source_names(&mut self, change: SourceNames) {
+        let Some(Dialog::Connection(form)) = &mut self.dialog else {
+            return;
+        };
+        match change {
+            SourceNames::Typed => {
+                form.files_names = None;
+                form.files_names_closed = false;
+                self.list_source();
+            }
+            SourceNames::Move(by) => {
+                if let Some(list) = &mut form.files_names {
+                    crate::source_names::moved(list, by);
+                }
+            }
+            SourceNames::Take(at) => self.take_source_name(at),
+            SourceNames::Close => {
+                form.files_names = None;
+                form.files_names_closed = true;
+            }
+            SourceNames::Add => {
+                form.files_names = None;
+                // Enter takes the keyboard from the line: back to it.
+                form.files_to_line += 1;
+                // Only a file's path from a root: not a folder's (a held
+                // Enter after one was taken), nor one the connection would
+                // read from wherever the app was started.
+                let typed = form.files_add.trim();
+                let path = tabletist_db::files::expanded(Path::new(typed));
+                let file = !typed.is_empty()
+                    && !typed.ends_with(['/', std::path::MAIN_SEPARATOR])
+                    && path.has_root();
+                if file {
+                    let typed = std::mem::take(&mut form.files_add);
+                    self.add_typed_file(&typed);
+                }
+            }
+        }
+    }
+
+    /// Asks for the names the path line starts, off this thread: none in
+    /// the macOS look, whose Source has no typed line, nor for a line with
+    /// no folder to list (`source_names::ask`).
+    fn list_source(&mut self) {
+        if !self.look.terminal {
+            return;
+        }
+        let asked = self.next_id();
+        let home = self.dirs.home.clone();
+        let Some(Dialog::Connection(form)) = &mut self.dialog else {
+            return;
+        };
+        // Only a Files connection's Source has path lines.
+        match form.driver {
+            Driver::Files => {}
+            Driver::Postgres | Driver::MySql | Driver::Sqlite | Driver::BigQuery => return,
+        }
+        // One listing at a time: the one on its way is followed by another
+        // for the line as it is then (`source_listed`).
+        if form.files_listing.is_some() {
+            return;
+        }
+        let (line, keep) = form.source_line();
+        let Some((folder, start)) = crate::source_names::ask(line, home.as_deref()) else {
+            return;
+        };
+        let command = Command::ListFolder {
+            asked,
+            typed: line.to_owned(),
+            folder,
+            start,
+            keep,
+        };
+        form.files_listing = Some(asked);
+        self.backend.send(command);
+    }
+
+    /// The names a path line starts: kept for the line they were asked
+    /// for, dropped for one since changed or another form's.
+    pub(super) fn source_listed(&mut self, asked: u64, typed: &str, names: Vec<Listed>) {
+        let Some(Dialog::Connection(form)) = &mut self.dialog else {
+            return;
+        };
+        if form.files_listing != Some(asked) {
+            return;
+        }
+        form.files_listing = None;
+        // The line changed while it was read: the line as it is now.
+        if form.source_line().0 != typed {
+            self.list_source();
+            return;
+        }
+        if form.files_names_closed {
+            return;
+        }
+        // A file the list holds already is not offered again.
+        let names: Vec<Listed> = names
+            .into_iter()
+            .filter(|listed| listed.folder || !form.lists_file(&complete(typed, listed)))
+            .collect();
+        // Nothing matches: no list.
+        form.files_names = (!names.is_empty()).then(|| PathList {
+            line: typed.to_owned(),
+            names,
+            lit: 0,
+        });
+    }
+
+    /// A name of the path line's list taken: the one at `at`, else the lit
+    /// one. A folder's lists its own names next; a file's (the add line
+    /// lists files) is added to the list, and the line is emptied for the
+    /// next.
+    fn take_source_name(&mut self, at: Option<usize>) {
+        let Some(Dialog::Connection(form)) = &mut self.dialog else {
+            return;
+        };
+        let line = form.source_line().0.to_owned();
+        let picked = match at {
+            Some(at) => form
+                .files_names
+                .as_ref()
+                .filter(|list| list.line == line)
+                .and_then(|list| list.names.get(at)),
+            None => form.lit_source_name(),
+        };
+        let Some(picked) = picked.cloned() else {
+            return;
+        };
+        let taken = complete(&line, &picked);
+        form.files_names = None;
+        // Typing goes on at the line's end, not where the part taken was.
+        form.files_to_line += 1;
+        match (picked.folder, form.files_kind) {
+            (true, FilesKind::Folder) => {
+                form.files_folder = taken;
+                self.list_source();
+            }
+            (true, FilesKind::Files) => {
+                form.files_add = taken;
+                self.list_source();
+            }
+            (false, _) => {
+                form.files_add.clear();
+                self.add_typed_file(&taken);
+            }
+        }
+    }
+
+    /// Adds the file `typed` names to the list, once (however it is
+    /// written), after the files already listed, and reads the Source
+    /// again: as the file dialog's
+    /// files are added. A path that is not there is listed all the same,
+    /// and its row says it is missing.
+    fn add_typed_file(&mut self, typed: &str) {
+        let Some(Dialog::Connection(form)) = &mut self.dialog else {
+            return;
+        };
+        let at = match form.listed_at(typed) {
+            Some(at) => at,
+            None => {
+                let path = tabletist_db::files::expanded(Path::new(typed.trim()));
+                form.files_list.push(tabletist_db::files::shown(&path));
+                form.files_list.len() - 1
+            }
+        };
+        form.files_cursor = Some(at);
+        self.read_form_source();
+    }
+}
diff --git a/src/ui/connect_dialog/files.rs b/src/ui/connect_dialog/files.rs
--- a/src/ui/connect_dialog/files.rs
+++ b/src/ui/connect_dialog/files.rs
@@ -7,7 +7,7 @@
 
 use super::choice::{Choice, Group, choose};
 use super::{Skin, field_with_button, input};
-use crate::model::{Action, ConnectionForm, FilesFound, FilesKind};
+use crate::model::{Action, ConnectionForm, FilesFound, FilesKind, SourceNames};
 use crate::style::Icon;
 use crate::typography::{Text, TextRole};
 use crate::ui::{file_list, widgets};
@@ -288,9 +288,51 @@
     }
 }
 
-/// The terminal look: the "source" section's rows. The folder is typed
-/// or picked (`ctrl+o`); the list's rows are moved through with the
-/// arrows, `a` adds files and `x` takes the row off.
+/// The path line names are typed into for the Source's `kind`, in the
+/// terminal look: the folder field, or the list's add line. Each its own,
+/// so neither takes the other's text back with undo.
+pub(crate) fn source_line_id(kind: FilesKind) -> egui::Id {
+    match kind {
+        FilesKind::Folder => egui::Id::new("files-folder-line"),
+        FilesKind::Files => egui::Id::new("files-add-line"),
+    }
+}
+
+/// The names listed under the path line, over the dialog.
+fn names_id() -> egui::Id {
+    egui::Id::new("files-source-names")
+}
+
+/// The names under the path line `field` (the "Omarchy – Files, path
+/// completions" board), and what they ask for.
+fn names(
+    ui: &Ui,
+    field: &egui::Response,
+    form: &ConnectionForm,
+    skin: &Skin,
+    actions: &mut Vec<Action>,
+) -> crate::ui::path_list::Asked {
+    let asked = crate::ui::path_list::under(
+        ui,
+        field,
+        form.source_line().0,
+        form.files_names.as_ref(),
+        names_id(),
+        (skin.look, skin.palette),
+    );
+    if asked.close {
+        actions.push(Action::SourceNames(SourceNames::Close));
+    }
+    if let Some(at) = asked.taken {
+        actions.push(Action::SourceNames(SourceNames::Take(Some(at))));
+    }
+    asked
+}
+
+/// The terminal look: the "source" section's rows. The folder is typed,
+/// its folders listed under it as it is, or picked (`ctrl+o`); the list's
+/// rows are moved through with the arrows, `a` adds files and `x` takes
+/// the row off.
 pub(super) fn terminal(
     ui: &mut Ui,
     form: &mut ConnectionForm,
@@ -323,13 +365,22 @@
                     role,
                     skin,
                 );
+                let id = source_line_id(FilesKind::Folder);
+                crate::ui::path_list::to_line(ui.ctx(), id, form.files_to_line, &form.files_folder);
                 let field = ui
                     .add(
                         input(ui, &mut form.files_folder, role, skin.field_height(), skin)
+                            .id(id)
                             .hint_text(hint),
                     )
                     .labelled_by(label);
-                if field.lost_focus() {
+                if field.changed() {
+                    actions.push(Action::SourceNames(SourceNames::Typed));
+                }
+                let asked = names(ui, &field, form, skin, actions);
+                // A click on a name is no leaving: the folder is read when
+                // the keyboard goes on.
+                if field.lost_focus() && !asked.on_list {
                     actions.push(Action::ReadSource);
                 }
             });
diff --git a/src/ui/connect_dialog/mod.rs b/src/ui/connect_dialog/mod.rs
--- a/src/ui/connect_dialog/mod.rs
+++ b/src/ui/connect_dialog/mod.rs
@@ -6,6 +6,8 @@
 mod agents;
 mod choice;
 mod files;
+#[cfg(test)]
+pub(crate) use files::source_line_id;
 mod group;
 mod reading;
 mod sheet;
@@ -22,7 +24,7 @@
 use crate::i18n::{Locale, Msgid, fill, gettext, msgid, ngettext};
 use crate::keymap::{Command, Keymap, Scope};
 use crate::model::{
-    Action, ConnectionForm, FilesKind, LoadedTables, SshAuthKind, SshHints, TestState,
+    Action, ConnectionForm, FilesKind, LoadedTables, SourceNames, SshAuthKind, SshHints, TestState,
 };
 use crate::style::Icon;
 use crate::theme::{self, Look, Palette};
@@ -349,6 +351,31 @@
     let focus_name = form.focus_name;
     form.focus_name &= measuring;
     let mut actions: Vec<Action> = wanted.into_iter().collect();
+    // The names under a Files path line, while they show: their keys come
+    // before the form's Enter and Esc, and are not the line's to move its
+    // cursor with. Enter held from the press that took a name takes
+    // nothing more.
+    let on_line =
+        ctx.memory(|memory| memory.focused()) == Some(files::source_line_id(form.files_kind));
+    if look.terminal && is_files(form) && on_line && form.lit_source_name().is_some() {
+        ctx.input_mut(|input| {
+            let pick = on(input, Command::PathPick);
+            if pick.count > 0 {
+                crate::ui::keys::take_enter(input);
+                if pick.fresh {
+                    actions.push(Action::SourceNames(SourceNames::Take(None)));
+                }
+            } else if on(input, Command::ClosePathList).count > 0 {
+                actions.push(Action::SourceNames(SourceNames::Close));
+            } else {
+                let step = on(input, Command::PathNext).count as isize
+                    - on(input, Command::PathPrevious).count as isize;
+                if step != 0 {
+                    actions.push(Action::SourceNames(SourceNames::Move(step)));
+                }
+            }
+        });
+    }
     // The dialog's own keys are taken before anything draws: a focused
     // button would read the chord that saves and connects as a press of
     // itself. An open list keeps them, as it keeps Escape. A press of its
diff --git a/src/ui/export.rs b/src/ui/export.rs
--- a/src/ui/export.rs
+++ b/src/ui/export.rs
@@ -696,50 +696,21 @@
                 response.rect.center().y,
                 hint(),
             );
-            // Where the list is drawn (where egui put it, as a small window
-            // may move it up): a press on it takes the keyboard from the
-            // line for the moment of a click, a pointer only resting there
-            // does not keep it.
-            let list_rect = sheet.listed.as_ref().map(|list| {
-                let height = TextRole::OBody.row_height(ui.ctx(), look.faces) + 4.0;
-                let below = Rect::from_min_size(
-                    response.rect.left_bottom(),
-                    vec2(response.rect.width(), height * list.names.len() as f32),
-                );
-                ui.ctx()
-                    .memory(|memory| memory.area_rect(list_id(sheet)))
-                    .unwrap_or(below)
-            });
-            let clicking = ui
-                .ctx()
-                .input(|input| input.pointer.any_down() || input.pointer.any_released());
-            let pointer_on_list = clicking
-                && list_rect.is_some_and(|rect| {
-                    ui.ctx()
-                        .pointer_latest_pos()
-                        .is_some_and(|pos| rect.contains(pos))
-                });
             // The keyboard gone from the line: its list goes too, until the
             // next key typed there (plan 4b, B9).
-            if !response.has_focus() && !pointer_on_list && sheet.listed.is_some() {
+            let asked = crate::ui::path_list::under(
+                ui,
+                &response,
+                &sheet.path,
+                sheet.listed.as_ref(),
+                list_id(sheet),
+                (look, palette),
+            );
+            if asked.close {
                 change(actions, ExportChange::ClosePathList);
             }
-            if (response.has_focus() || pointer_on_list) && sheet.lit_name().is_some() {
-                // Esc and ↑ ↓ are the list's, not egui's to move or drop
-                // the keyboard with (`keys`).
-                let filter = egui::EventFilter {
-                    horizontal_arrows: true,
-                    vertical_arrows: true,
-                    escape: true,
-                    ..Default::default()
-                };
-                ui.ctx()
-                    .memory_mut(|memory| memory.set_focus_lock_filter(response.id, filter));
-                if path_list(ui.ctx(), sheet, response.rect, skin, actions) {
-                    // Back to the line, for a folder's names next.
-                    ui.ctx()
-                        .memory_mut(|memory| memory.request_focus(response.id));
-                }
+            if let Some(at) = asked.taken {
+                actions.push(Action::PickPath(Some(at)));
             }
             if sheet.exists {
                 widgets::label(
@@ -763,93 +734,7 @@
 /// time the app asks (`ExportSheet::to_line`): a name taken from the list,
 /// `ctrl+o`'s answer.
 fn take_line(ctx: &egui::Context, id: Id, sheet: &ExportSheet) {
-    let seen = id.with("to-line");
-    let asked = ctx.data_mut(|data| {
-        let before = data.get_temp::<u64>(seen).unwrap_or(0);
-        data.insert_temp(seen, sheet.to_line);
-        before != sheet.to_line
-    });
-    if !asked {
-        return;
-    }
-    let mut state = egui::text_edit::TextEditState::load(ctx, id).unwrap_or_default();
-    let end = egui::text::CCursor::new(sheet.path.chars().count());
-    state
-        .cursor
-        .set_char_range(Some(egui::text::CCursorRange::one(end)));
-    state.store(ctx, id);
-    ctx.memory_mut(|memory| memory.request_focus(id));
-}
-
-/// The names the path line starts, under the line `field` (spec 4, S15;
-/// the component board's path line): the lit one on the selection's fill,
-/// the rest muted, in an area of their own over the float, as the float's
-/// rows scroll and clip. A click takes one; returns whether one did.
-fn path_list(
-    ctx: &egui::Context,
-    sheet: &ExportSheet,
-    field: Rect,
-    skin: Skin<'_>,
-    actions: &mut Vec<Action>,
-) -> bool {
-    let Skin { look, palette, .. } = skin;
-    let Some(list) = sheet.listed.as_ref() else {
-        return false;
-    };
-    let lit = list.lit.min(list.names.len().saturating_sub(1));
-    let height = TextRole::OBody.row_height(ctx, look.faces) + 4.0;
-    let mut taken = false;
-    egui::Area::new(list_id(sheet))
-        .order(egui::Order::Tooltip)
-        .fixed_pos(field.left_bottom())
-        .show(ctx, |ui| {
-            ui.spacing_mut().item_spacing = egui::Vec2::ZERO;
-            for (at, listed) in list.names.iter().enumerate() {
-                // No keyboard of its own: the line keeps it.
-                let (row, response) =
-                    ui.allocate_exact_size(vec2(field.width(), height), Sense::CLICK);
-                let name = match listed.folder {
-                    true => format!("{}/", listed.name),
-                    false => listed.name.clone(),
-                };
-                let ink = match at == lit {
-                    true => {
-                        ui.painter().rect_filled(row, 0, palette.selection);
-                        palette.text
-                    }
-                    false => {
-                        ui.painter().rect_filled(row, 0, palette.panel);
-                        palette.dim
-                    }
-                };
-                widgets::paint_text(
-                    ui,
-                    row.left() + 8.0,
-                    row.center().y,
-                    Text::one(look, TextRole::OBody, &name, ink),
-                );
-                response.widget_info(|| {
-                    egui::WidgetInfo::selected(
-                        egui::WidgetType::SelectableLabel,
-                        true,
-                        at == lit,
-                        &name,
-                    )
-                });
-                if response.clicked() {
-                    actions.push(Action::PickPath(Some(at)));
-                    taken = true;
-                }
-            }
-            let outline = ui.min_rect();
-            ui.painter().rect_stroke(
-                outline,
-                0,
-                egui::Stroke::new(1.0, palette.outline),
-                egui::StrokeKind::Inside,
-            );
-        });
-    taken
+    crate::ui::path_list::to_line(ctx, id, sheet.to_line, &sheet.path);
 }
 
 /// The notes under the choices, always in sight above the footer: the
diff --git a/src/ui/mod.rs b/src/ui/mod.rs
--- a/src/ui/mod.rs
+++ b/src/ui/mod.rs
@@ -126,6 +126,7 @@
 #[cfg(test)]
 mod partial_test_tests;
 pub mod password_prompt;
+pub mod path_list;
 pub mod pending_bar;
 pub mod picker;
 #[cfg(test)]
diff --git a/src/ui/path_list.rs b/src/ui/path_list.rs
new file mode 100644
--- /dev/null
+++ b/src/ui/path_list.rs
@@ -0,0 +1,178 @@
+//! The names listed under a path line in the terminal look, as a path is
+//! typed: the export float's save to line (spec 4, S15; the component
+//! board's path line) and a Files connection's folder field and add line
+//! (the "Omarchy – Files, path completions" board). The lit name on the
+//! selection's fill, the rest muted, in an area of its own over the
+//! dialog, as the dialog's rows scroll and clip. The line keeps the
+//! keyboard: the dialog reads the list's keys before it draws.
+
+use egui::{Id, Rect, Response, Sense, Ui, vec2};
+
+use crate::export_sheet::PathList;
+use crate::theme::{Look, Palette};
+use crate::typography::{Text, TextRole};
+use crate::ui::widgets;
+
+/// What the list under a path line asks for, this frame.
+#[derive(Default)]
+pub struct Asked {
+    /// The keyboard left the line: the list goes too, until the next key
+    /// typed there.
+    pub close: bool,
+    /// The name clicked, by its place.
+    pub taken: Option<usize>,
+    /// A press on the list, which takes the keyboard from the line for the
+    /// moment of a click: no leaving of the line.
+    pub on_list: bool,
+}
+
+/// The list under the path line `field`, which holds `line`: `list` while
+/// it is the line's, in the area `id`. A click on a name takes it and
+/// gives the line the keyboard back.
+pub fn under(
+    ui: &Ui,
+    field: &Response,
+    line: &str,
+    list: Option<&PathList>,
+    id: Id,
+    skin: (&Look, &Palette),
+) -> Asked {
+    let (look, _) = skin;
+    let mut asked = Asked::default();
+    let height = row_height(ui.ctx(), look);
+    // Where the list is drawn (where egui put it, as a small window may
+    // move it up): a press on it takes the keyboard from the line for the
+    // moment of a click, a pointer only resting there does not keep it.
+    let list_rect = list.map(|list| {
+        let below = Rect::from_min_size(
+            field.rect.left_bottom(),
+            vec2(field.rect.width(), height * list.names.len() as f32),
+        );
+        ui.ctx()
+            .memory(|memory| memory.area_rect(id))
+            .unwrap_or(below)
+    });
+    let clicking = ui
+        .ctx()
+        .input(|input| input.pointer.any_down() || input.pointer.any_released());
+    let pointer_on_list = clicking
+        && list_rect.is_some_and(|rect| {
+            ui.ctx()
+                .pointer_latest_pos()
+                .is_some_and(|pos| rect.contains(pos))
+        });
+    asked.on_list = pointer_on_list;
+    if !field.has_focus() && !pointer_on_list && list.is_some() {
+        asked.close = true;
+    }
+    let shown = list.filter(|list| list.line == line && !list.names.is_empty());
+    if let Some(list) = shown
+        && (field.has_focus() || pointer_on_list)
+    {
+        // Esc and ↑ ↓ are the list's, not egui's to move or drop the
+        // keyboard with.
+        let filter = egui::EventFilter {
+            horizontal_arrows: true,
+            vertical_arrows: true,
+            escape: true,
+            ..Default::default()
+        };
+        ui.ctx()
+            .memory_mut(|memory| memory.set_focus_lock_filter(field.id, filter));
+        asked.taken = names(ui.ctx(), id, list, field.rect, skin);
+        if asked.taken.is_some() {
+            // Back to the line, for a folder's names next.
+            ui.ctx().memory_mut(|memory| memory.request_focus(field.id));
+        }
+    }
+    asked
+}
+
+/// Gives the line `id` the keyboard, its cursor at the end of `text`, once
+/// for each new `asked`: after a name is taken from its list. Nought is
+/// never asked, so a dialog opened anew takes no keyboard to the line.
+pub fn to_line(ctx: &egui::Context, id: Id, asked: u64, text: &str) {
+    let seen = id.with("to-line");
+    let new = ctx.data_mut(|data| {
+        let before = data.get_temp::<u64>(seen).unwrap_or(0);
+        data.insert_temp(seen, asked);
+        before != asked
+    });
+    if !new || asked == 0 {
+        return;
+    }
+    let mut state = egui::text_edit::TextEditState::load(ctx, id).unwrap_or_default();
+    let end = egui::text::CCursor::new(text.chars().count());
+    state
+        .cursor
+        .set_char_range(Some(egui::text::CCursorRange::one(end)));
+    state.store(ctx, id);
+    ctx.memory_mut(|memory| memory.request_focus(id));
+}
+
+fn row_height(ctx: &egui::Context, look: &Look) -> f32 {
+    TextRole::OBody.row_height(ctx, look.faces) + 4.0
+}
+
+/// The names, under `field`: the one clicked, if one was.
+fn names(
+    ctx: &egui::Context,
+    id: Id,
+    list: &PathList,
+    field: Rect,
+    (look, palette): (&Look, &Palette),
+) -> Option<usize> {
+    let lit = list.lit.min(list.names.len().saturating_sub(1));
+    let height = row_height(ctx, look);
+    let mut taken = None;
+    egui::Area::new(id)
+        .order(egui::Order::Tooltip)
+        .fixed_pos(field.left_bottom())
+        .show(ctx, |ui: &mut Ui| {
+            ui.spacing_mut().item_spacing = egui::Vec2::ZERO;
+            for (at, listed) in list.names.iter().enumerate() {
+                // No keyboard of its own: the line keeps it.
+                let (row, response) =
+                    ui.allocate_exact_size(vec2(field.width(), height), Sense::CLICK);
+                let name = match listed.folder {
+                    true => format!("{}/", listed.name),
+                    false => listed.name.clone(),
+                };
+                let ink = match at == lit {
+                    true => {
+                        ui.painter().rect_filled(row, 0, palette.selection);
+                        palette.text
+                    }
+                    false => {
+                        ui.painter().rect_filled(row, 0, palette.panel);
+                        palette.dim
+                    }
+                };
+                widgets::paint_text(
+                    ui,
+                    row.left() + 8.0,
+                    row.center().y,
+                    Text::one(look, TextRole::OBody, &name, ink),
+                );
+                response.widget_info(|| {
+                    egui::WidgetInfo::selected(
+                        egui::WidgetType::SelectableLabel,
+                        true,
+                        at == lit,
+                        &name,
+                    )
+                });
+                if response.clicked() {
+                    taken = Some(at);
+                }
+            }
+            let outline = ui.min_rect();
+            ui.painter().rect_stroke(
+                outline,
+                0,
+                egui::Stroke::new(1.0, palette.outline),
+                egui::StrokeKind::Inside,
+            );
+        });
+    taken
+}
````

- [ ] **Step 4: Run them to see them pass**

Run: `~/.cargo/bin/cargo test -q -p tabletist --lib source_names`

Expected: `test result: ok. 7 passed`. `~/.cargo/bin/cargo test -q -p tabletist-core --lib set_aside` passes too, `a_form_set_aside_waits_for_no_listing` among them. The save to line is unchanged: `~/.cargo/bin/cargo test -q -p tabletist --lib export_path` passes.

- [ ] **Step 5: Run the full check**

Expected: all pass; 5,679 passed, 0 failed.

- [ ] **Step 6: Commit**

```bash
git add -A src crates
git commit -m "List the folders a typed Files folder starts, in the Omarchy look"
```

### Task 3: The add line under the file list

**Files:**
- Modify: `src/ui/connect_dialog/files.rs`, `assets/i18n/ru.po`, `assets/i18n/pt-BR.po`, `assets/i18n/tabletist.pot`, `docs/_reference/connections.md`, `docs/_guide/omarchy.md`
- Test: `src/ui/source_names_tests.rs`

- [ ] **Step 1: Write the failing tests**

````diff
diff --git a/src/ui/source_names_tests.rs b/src/ui/source_names_tests.rs
--- a/src/ui/source_names_tests.rs
+++ b/src/ui/source_names_tests.rs
@@ -266,3 +266,190 @@
     assert_eq!(form(&harness).files_folder, "/data/bookshop-exports/");
     assert_eq!(harness.app.backend.sources_read.len(), read);
 }
+
+/// The add line lists folders and the files a Files connection reads;
+/// Enter on a file adds it to the list, empties the line and reads the
+/// Source again, the keyboard staying on the line for the next.
+#[test]
+fn the_add_line_adds_the_file_taken() {
+    let mut harness = dialog(FilesKind::Files);
+    type_line(&mut harness, "/data/bookshop-exports/ret");
+    assert_eq!(
+        asks(&harness).last(),
+        Some(&(
+            "/data/bookshop-exports/ret".to_owned(),
+            PathBuf::from("/data/bookshop-exports/"),
+            "ret".to_owned(),
+            Keep::Readable
+        ))
+    );
+    answer(
+        &mut harness,
+        "/data/bookshop-exports/ret",
+        &[("returns.csv", false), ("returns.xlsx", false)],
+    );
+    assert!(painted(&harness, "returns.csv"));
+    let read = harness.app.backend.sources_read.len();
+    harness.press(Key::Enter, Modifiers::NONE);
+    harness.settle();
+    let open = form(&harness);
+    assert_eq!(open.files_list, ["/data/bookshop-exports/returns.csv"]);
+    assert_eq!(open.files_cursor, Some(0));
+    assert!(open.files_add.is_empty());
+    assert!(open.files_names.is_none());
+    assert_eq!(harness.app.backend.sources_read.len(), read + 1);
+    assert!(on_line(&harness), "the line keeps the keyboard");
+}
+
+/// A file the list holds already is not offered again; its folder is.
+#[test]
+fn a_listed_file_is_not_offered_again() {
+    let mut harness = dialog(FilesKind::Files);
+    if let Some(Dialog::Connection(form)) = &mut harness.app.dialog {
+        form.files_list = vec!["/data/bookshop-exports/orders.csv".into()];
+    }
+    type_line(&mut harness, "/data/bookshop-exports/ord");
+    answer(
+        &mut harness,
+        "/data/bookshop-exports/ord",
+        &[("orders-2025", true), ("orders.csv", false)],
+    );
+    let names: Vec<&str> = form(&harness)
+        .files_names
+        .as_ref()
+        .expect("names listed")
+        .names
+        .iter()
+        .map(|listed| listed.name.as_str())
+        .collect();
+    assert_eq!(names, ["orders-2025"]);
+}
+
+/// Enter with no name listed adds the path as typed: one that is not
+/// there is listed all the same, its row saying it is missing.
+/// (`enter_adds_only_a_files_path` has what it does not add.)
+#[test]
+fn enter_with_no_names_adds_the_path_as_typed() {
+    let mut harness = dialog(FilesKind::Files);
+    type_line(&mut harness, "/data/bookshop-exports/events.ndjson");
+    answer(&mut harness, "/data/bookshop-exports/events.ndjson", &[]);
+    harness.press(Key::Enter, Modifiers::NONE);
+    harness.settle();
+    let open = form(&harness);
+    assert_eq!(open.files_list, ["/data/bookshop-exports/events.ndjson"]);
+    assert!(open.files_add.is_empty());
+    assert!(on_line(&harness), "the line keeps the keyboard");
+    assert!(matches!(harness.app.dialog, Some(Dialog::Connection(_))));
+}
+
+/// In the add line `a` and `x` type: they add and remove files only
+/// where no line has the keyboard. Each comes as a keyboard sends it, the
+/// key and then its text.
+#[test]
+fn letters_in_the_add_line_are_typed() {
+    let mut harness = dialog(FilesKind::Files);
+    if let Some(Dialog::Connection(form)) = &mut harness.app.dialog {
+        form.files_list = vec!["/data/bookshop-exports/orders.csv".into()];
+    }
+    for (key, text) in [(Key::A, "a"), (Key::X, "x")] {
+        harness.frame(vec![
+            crate::testing::key(key, Modifiers::NONE),
+            egui::Event::Text(text.into()),
+            crate::testing::release(key, Modifiers::NONE),
+        ]);
+    }
+    harness.settle();
+    let open = form(&harness);
+    assert_eq!(open.files_add, "ax");
+    assert_eq!(open.files_list.len(), 1, "x removed nothing");
+    assert!(open.pick_request.is_none(), "a opened no picker");
+}
+
+/// Enter in the add line adds only a file's path from a root: not a
+/// folder's, which ends in its `/`, nor a path with no root, which the
+/// connection would read from wherever the app was started. The line
+/// keeps its text and the keyboard.
+#[test]
+fn enter_adds_only_a_files_path() {
+    let mut harness = dialog(FilesKind::Files);
+    for line in ["/data/bookshop-exports/", "orders.csv"] {
+        type_line(&mut harness, line);
+        if form(&harness).files_listing.is_some() {
+            answer(&mut harness, line, &[]);
+        }
+        harness.press(Key::Enter, Modifiers::NONE);
+        harness.settle();
+        let open = form(&harness);
+        assert!(open.files_list.is_empty(), "{line}: {:?}", open.files_list);
+        assert_eq!(open.files_add, line);
+        assert!(on_line(&harness), "{line}: the line keeps the keyboard");
+    }
+}
+
+/// Enter held after it took a folder takes nothing more, though the
+/// folder lists nothing: the folder is not added as a file.
+#[test]
+fn enter_held_after_a_folder_adds_nothing() {
+    let mut harness = dialog(FilesKind::Files);
+    type_line(&mut harness, "/data/bookshop-exports/ret");
+    answer(
+        &mut harness,
+        "/data/bookshop-exports/ret",
+        &[("returns-2025", true)],
+    );
+    let enter = || crate::testing::key(Key::Enter, Modifiers::NONE);
+    harness.frame(vec![enter()]);
+    harness.settle();
+    assert_eq!(
+        form(&harness).files_add,
+        "/data/bookshop-exports/returns-2025/"
+    );
+    answer(&mut harness, "/data/bookshop-exports/returns-2025/", &[]);
+    // Still held: egui sends the key down again, as a repeat.
+    harness.frame(vec![enter()]);
+    harness.frame(vec![crate::testing::release(Key::Enter, Modifiers::NONE)]);
+    harness.settle();
+    let open = form(&harness);
+    assert!(open.files_list.is_empty(), "{:?}", open.files_list);
+    assert_eq!(open.files_add, "/data/bookshop-exports/returns-2025/");
+}
+
+/// While names are listed under the add line, Down moves in them, not
+/// the file list's cursor above it.
+#[test]
+fn down_moves_in_the_names_not_the_list() {
+    let mut harness = dialog(FilesKind::Files);
+    if let Some(Dialog::Connection(form)) = &mut harness.app.dialog {
+        form.files_list = vec![
+            "/data/bookshop-exports/orders.csv".into(),
+            "/data/bookshop-exports/customers.parquet".into(),
+        ];
+        form.files_cursor = Some(0);
+    }
+    type_line(&mut harness, "/data/bookshop-exports/ret");
+    answer(
+        &mut harness,
+        "/data/bookshop-exports/ret",
+        &[("returns.csv", false), ("returns.xlsx", false)],
+    );
+    harness.press(Key::ArrowDown, Modifiers::NONE);
+    assert_eq!(lit(&harness), 1);
+    assert_eq!(form(&harness).files_cursor, Some(0));
+}
+
+/// A file is found in the list however its path is written: typed again
+/// with a `./` in it, it is not added a second time.
+#[test]
+fn a_file_typed_another_way_is_not_added_twice() {
+    let mut harness = dialog(FilesKind::Files);
+    if let Some(Dialog::Connection(form)) = &mut harness.app.dialog {
+        form.files_list = vec!["/data/bookshop-exports/orders.csv".into()];
+    }
+    type_line(&mut harness, "/data/bookshop-exports/./orders.csv");
+    answer(&mut harness, "/data/bookshop-exports/./orders.csv", &[]);
+    harness.press(Key::Enter, Modifiers::NONE);
+    harness.settle();
+    let open = form(&harness);
+    assert_eq!(open.files_list, ["/data/bookshop-exports/orders.csv"]);
+    assert_eq!(open.files_cursor, Some(0));
+}
````

- [ ] **Step 2: Run them to see them fail**

Run: `~/.cargo/bin/cargo test -q -p tabletist --lib source_names`

Expected: the eight new tests fail: there's no add line to type in, so nothing is listed or added (and with no line, `a` opens the file chooser). Task 2's seven pass.

```text
failures:
---- ui::source_names_tests::enter_with_no_names_adds_the_path_as_typed stdout ----
thread 'ui::source_names_tests::enter_with_no_names_adds_the_path_as_typed' (1237169) panicked at src/ui/source_names_tests.rs:79:45:
---- ui::source_names_tests::the_add_line_adds_the_file_taken stdout ----
thread 'ui::source_names_tests::the_add_line_adds_the_file_taken' (1237176) panicked at src/ui/source_names_tests.rs:277:5:
  left: None
---- ui::source_names_tests::down_moves_in_the_names_not_the_list stdout ----
thread 'ui::source_names_tests::down_moves_in_the_names_not_the_list' (1237164) panicked at src/ui/source_names_tests.rs:79:45:
---- ui::source_names_tests::a_file_typed_another_way_is_not_added_twice stdout ----
thread 'ui::source_names_tests::a_file_typed_another_way_is_not_added_twice' (1237160) panicked at src/ui/source_names_tests.rs:79:45:
---- ui::source_names_tests::a_listed_file_is_not_offered_again stdout ----
thread 'ui::source_names_tests::a_listed_file_is_not_offered_again' (1237162) panicked at src/ui/source_names_tests.rs:79:45:
---- ui::source_names_tests::letters_in_the_add_line_are_typed stdout ----
thread 'ui::source_names_tests::letters_in_the_add_line_are_typed' (1237173) panicked at src/ui/source_names_tests.rs:363:5:
  left: ""
---- ui::source_names_tests::enter_held_after_a_folder_adds_nothing stdout ----
thread 'ui::source_names_tests::enter_held_after_a_folder_adds_nothing' (1237166) panicked at src/ui/source_names_tests.rs:79:45:
---- ui::source_names_tests::enter_adds_only_a_files_path stdout ----
thread 'ui::source_names_tests::enter_adds_only_a_files_path' (1237165) panicked at src/ui/source_names_tests.rs:384:9:
    ui::source_names_tests::a_file_typed_another_way_is_not_added_twice
    ui::source_names_tests::a_listed_file_is_not_offered_again
    ui::source_names_tests::down_moves_in_the_names_not_the_list
    ui::source_names_tests::enter_adds_only_a_files_path
    ui::source_names_tests::enter_held_after_a_folder_adds_nothing
    ui::source_names_tests::enter_with_no_names_adds_the_path_as_typed
    ui::source_names_tests::letters_in_the_add_line_are_typed
    ui::source_names_tests::the_add_line_adds_the_file_taken
test result: FAILED. 7 passed; 8 failed; 0 ignored; 0 measured; 3132 filtered out; finished in 0.04s
error: test failed, to rerun pass `-p tabletist --lib`
```

- [ ] **Step 3: The add line, and the docs**

````diff
diff --git a/docs/_guide/omarchy.md b/docs/_guide/omarchy.md
--- a/docs/_guide/omarchy.md
+++ b/docs/_guide/omarchy.md
@@ -103,6 +103,7 @@
 | Up, Down | In add table…'s or add column…'s list: move. Enter adds the one chosen, or in add column…'s puts a table chosen in the field for its columns, and Esc closes the field. In a Files connection's list of files: move |
 | Enter, Space | Choose the choice the cursor is on: a type, an environment, folder or files. On a closed section's heading (a Files connection's reading csv or agents): open it, and on an open one, close it. On group: open its list of groups, where Up and Down move and Enter picks; new group… turns it into a field for a new group's name |
 | Ctrl+O | A Files connection: pick its folder, or add files to its list, in the file chooser |
+| Ctrl+N, Ctrl+P | A Files connection's folder, or the add line under its list of files, while the names a typed path starts are listed under it: move (Down and Up too). Enter takes one: a folder lists its own names, a file is added to the list. Esc closes the list. In the add line with nothing listed, Enter adds the path as typed, if it names a file from `/` or `~` |
 | `a` | A Files connection's list: add files |
 | `x` | A Files connection's list: take the file under the cursor off the list. The file stays on disk |
 | `R` | A Files connection's reading csv: take the override the cursor is on off. Other overrides still apply |
diff --git a/docs/_reference/connections.md b/docs/_reference/connections.md
--- a/docs/_reference/connections.md
+++ b/docs/_reference/connections.md
@@ -101,6 +101,15 @@
   are left out, and a file added later shows the next time the connection
   opens. **Files** is a list you add files to (**Add files…**); × takes a
   file off the list and never off the disk.
+- In the Omarchy look the folder is typed, or picked with Ctrl+O, and the
+  list has an **add** line under it for a file's path. While you type a
+  path that starts with `/` or `~`, the names of its folder that start
+  with its last part are listed under the line: folders for the folder,
+  and folders and the files a Files connection reads for the add line.
+  Ctrl+N and Ctrl+P (or Down and Up) move, Enter takes one (a folder
+  lists its own names next, a file is added), and Esc closes the list.
+  Tab moves on and never completes. Enter in the add line with nothing
+  listed adds the path as typed, if it names a file from `/` or `~`.
 - Under the source, Tabletist says what it found: `4 files · 2.5 GB · CSV,
   Parquet, NDJSON`, and the tables they become.
 - A table's name is the file's name without its extension, in lower case,
diff --git a/src/ui/connect_dialog/files.rs b/src/ui/connect_dialog/files.rs
--- a/src/ui/connect_dialog/files.rs
+++ b/src/ui/connect_dialog/files.rs
@@ -332,7 +332,7 @@
 /// The terminal look: the "source" section's rows. The folder is typed,
 /// its folders listed under it as it is, or picked (`ctrl+o`); the list's
 /// rows are moved through with the arrows, `a` adds files and `x` takes
-/// the row off.
+/// the row off, and a file's path is typed in the add line under it.
 pub(super) fn terminal(
     ui: &mut Ui,
     form: &mut ConnectionForm,
@@ -434,6 +434,7 @@
                     },
                 );
             }
+            add_line(ui, form, skin, actions);
         }
     }
     let (text, color) = match trouble(&form.files_found, skin) {
@@ -448,6 +449,42 @@
                 .label(ui);
         });
     }
+}
+
+/// The line under the list a file's path is typed in: its folders and
+/// the files a Files connection reads listed under it as it is, Enter on
+/// one taking it, and Enter with none listed adding the path as typed.
+fn add_line(ui: &mut Ui, form: &mut ConnectionForm, skin: &Skin, actions: &mut Vec<Action>) {
+    let look = skin.look;
+    let role = TextRole::OBody;
+    let pick = crate::ui::keys::written(ui.ctx(), look, crate::keymap::Command::FormPick);
+    super::terminal::terminal_row(ui, &skin.say(msgid("Add")), skin, |ui, label| {
+        let hint = super::hint(
+            ui,
+            &format!("{} {}", pick.as_str(), skin.say(msgid("pick"))),
+            role,
+            skin,
+        );
+        let id = source_line_id(FilesKind::Files);
+        crate::ui::path_list::to_line(ui.ctx(), id, form.files_to_line, &form.files_add);
+        let field = ui
+            .add(
+                input(ui, &mut form.files_add, role, skin.field_height(), skin)
+                    .id(id)
+                    .hint_text(hint),
+            )
+            .labelled_by(label);
+        if field.changed() {
+            actions.push(Action::SourceNames(SourceNames::Typed));
+        }
+        let asked = names(ui, &field, form, skin, actions);
+        // Enter with no name listed (the list's Enter is the dialog's,
+        // read before it draws): the path as typed.
+        let entered = ui.input(|input| input.key_pressed(egui::Key::Enter));
+        if field.lost_focus() && entered && !asked.on_list {
+            actions.push(Action::SourceNames(SourceNames::Add));
+        }
+    });
 }
 
 #[cfg(test)]
````

- [ ] **Step 4: Bring the catalogs up to date and translate "Add"**

Run `sh assets/i18n/update.sh`, then fill in its `msgstr`:

| msgid | ru | pt-BR |
| --- | --- | --- |
| Add | Добавить | Adicionar |

Run `sh assets/i18n/update.sh` again. Then `sh assets/i18n/update.sh --check` and `sh assets/i18n/update_test.sh` pass, with every entry translated.

- [ ] **Step 5: Run them to see them pass**

Run: `~/.cargo/bin/cargo test -q -p tabletist --lib source_names`

Expected: `test result: ok. 15 passed`.

- [ ] **Step 6: Run the full check**

Expected: all pass; 5,687 passed, 0 failed.

- [ ] **Step 7: Commit**

```bash
git add src/ui/connect_dialog/files.rs src/ui/source_names_tests.rs assets/i18n docs/_reference/connections.md docs/_guide/omarchy.md
git commit -m "Add a Files list's file by typing its path, its folders and files listed as it is typed"
```

### After the tasks

- [ ] **Hand test**, in the Omarchy look on the Bookshop's exports:
  1. New connection, Files, folder: type `~/Data/book`. `bookshop-exports/` is listed; `ctrl+n`, `ctrl+p` and Enter take it, and its own folders follow. Tab reads the folder, and the found line says its four files.
  2. Files: type `~/Data/bookshop-exports/ret` in the add line. `returns.csv` is listed, and no file a Files connection doesn't read (put a `returns-notes.txt` beside it to see). Enter adds it, and the line is ready for the next.
  3. Esc closes a list and leaves the dialog open; Esc again cancels it.
  4. Compare with "Omarchy – Files, path completions" by eye.

  Any screenshot stays local, or is blurred.
- [ ] Push the branch and open a pull request against `main`. Its description lists:
  - what is listed, and where;
  - the departures above;
  - the canvas boards;
  - that the Portuguese wants a native review.
