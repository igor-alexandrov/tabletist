# Translations, run 1: the Language setting and Russian: Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use rubypowers:subagent-driven-development (recommended) or rubypowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Settings › General gets a Language row, System or a language. Russian is the first catalog. It goes with the tooling a translator needs and a contributor guide, and the strings the code glues from pieces become whole phrases, so Russian reads right.

**Architecture:** Four tasks, one PR, a commit each. The tasks are, in order:
1. **Whole phrases:** whole-phrase messages in the views; the English drawn stays the same.
2. **Catalog and tooling:** `assets/i18n/update.sh`, the template, `ru.po` and `Locale::Russian`, with a CI job.
3. **The setting:** `[interface] language` in settings.toml. `App::locale` follows it, and the menu bar is described again when it changes.
4. **The Language row:** the row on both looks, and the docs.

**Tech Stack:**
- Rust and egui 0.36.
- fastframe-i18n v0.16.1: `gettext`, `ngettext`, `LanguageTag`, `system_languages`, and catalogs compiled by `build.rs`.
- GNU gettext 0.24 or later: `xgettext`, `msgmerge`, `msgfmt`.
- The app's `testing::Harness`.

**Spec:** `docs/superpowers/specs/2026-11-17-translations-1-russian-design.md`.

**Base:** `main` at 628ab8b4. Work on this worktree's branch, `claude/design-agent-init-75b678`. If main moves before the PR, rebase onto it, never a merge of main. Then run `assets/i18n/update.sh` again, because main's new strings belong in the template: the counts this plan gives change with them.

---

## Decided with Igor (2026-11-17)

| What | The canvas or the reading | This run builds |
|---|---|---|
| First languages | Russian, then Português (Brasil) and 简体中文 | Russian only; the menu lists a language once its catalog ships |
| System | The system's first language only | With a catalog, that language; without one, English, never the next language (`Language::resolve`) |
| No catalog for the system language | The value reads "English", not "System (English)" | No System entry in the menu, ✓ on English; the file keeps `system`, so a later catalog is followed |
| Omarchy's list | Enter opens a filterable list | h/l step, R resets; the list comes with the later languages |
| Reset to defaults | Not drawn | General's reset leaves the language, as Appearance's leaves the look |
| Pieced-together strings | | Rewritten as whole phrases in this run (task 1) |
| What stays English | Keys, `:` commands, SQL, names, data, types, settings.toml, server errors, environment names | As drawn |
| The website | Only the app is translated | The guide pages stay English |

**For Igor's check:** the Russian wording. All 720 messages are in `assets/i18n/ru.po` (task 2, then task 4). These are the entries the draft was least sure of:
- «Чтение» for the access level Read
- «Сбросить» for Discard
- «Системный ({language})»
- «{language}: перевода пока нет»
- the abbreviations «подкл.», «Стр.» and «стлб»

**Not in this run (run 2):** strings handed to `gettext` in a variable, which no extractor reads, stay English in Russian until run 2. They include:
- the settings panes' names and row labels ("General", "On launch")
- the keymap's descriptions, and so the menu bar's items, and its Query, Window and Help menus (only File, Edit and View have Russian titles in this run)
- the Omarchy hint words drawn through `skin.say`
- words behind local `say` closures, among them the review's SQL comments (`-- row id 2 · blocked: fix kind first`)
- the English written in `cell_editor`, `sql_complete` and `date_line`
- a few words task 1 left alone ("Copied", "Query", "Row", "pending")

Run 2 also covers interface numbers and dates in the language's format.

## Files

| File | Task | What changes |
|---|---|---|
| `crates/tabletist-core/src/selection.rs`, `src/ui/{agents,conflict_prompt,copy_as,data_view,ddl,host_key_prompt,object_tabs,password_prompt,pending_bar,picker,quick_open,review,row_panel,sql_editor,start,workspace,write_prompts}.rs`, `src/ui/connect_dialog/mod.rs` | 1 | Whole-phrase messages |
| `src/app.rs`, `src/ui/settings/{mod,sheet,terminal}.rs` | 1, then 3 or 4 | Whole phrases (1); the locale (3); the Language row (4) |
| `crates/tabletist-core/src/i18n.rs` | 1, 2 | `around` (1); `Locale::Russian`, `first_system_language`, `endonym`, catalog tests (2) |
| `assets/i18n/update.sh`, `tabletist.pot`, `ru.po`; `.gitkeep` removed | 2, 4 | The tooling and the catalog; task 4 adds the row's nine messages |
| `.github/workflows/ci.yml`, `AGENTS.md` | 2 | The `translations` job; the rule for messages |
| `crates/tabletist-core/src/settings.rs` | 3, 4 | `Language` and its key (3); `OptionId::Language` and stepping (4) |
| `src/menu/bar.rs`, `src/menu/tests.rs` | 3 | Built again when the language changes |
| `crates/tabletist-core/src/model.rs` | 4 | The row first in General; Reset leaves it |
| `src/ui/language_tests.rs` (new), `src/ui/mod.rs`, `src/ui/settings_access_tests.rs` | 4 | The row's tests; existing tests one row down |
| `docs/_reference/translating.md` (new), `docs/_reference/settings-and-files.md`, `docs/_guide/omarchy.md` | 4 | The contributor guide and the reference |

Every task's checks, the same each time:

```bash
export CARGO_TARGET_DIR=$PWD/target
cargo fmt --all --check
cargo clippy --locked --workspace --all-targets -- -D warnings
RUSTDOCFLAGS='-D warnings' cargo doc --locked --workspace --no-deps
cargo test --locked --workspace --all-targets
```

From task 2 on, also `assets/i18n/update.sh --check`, which must print `720 translated messages.` (`711` after task 2 and task 3).

---

### Task 1: Whole phrases for the strings glued from pieces

Today many messages are glued: a count, "rows", "in" and a table's name. Translated piece by piece, Russian breaks («1 файла», «2 новая»). Each phrase becomes one message with named placeholders, and a count goes through `ngettext`. The English drawn is byte for byte what it was, so the existing tests are this task's tests and none changes.

Where a placeholder is drawn in a style of its own (the connection's name in strong weight, after "Agents on"), `i18n::around` splits the translated phrase at it. Where a phrase takes two values and one of them is a name, `i18n::fill` puts them in in one pass, so a table called `{columns}` is written as it is. Where the terminal look lower-cases words, the rewritten phrases mostly lower-case the translated phrase before the names go in, as the old code did; quick open's "no objects match" lower-cases after, as before. Two contexts part a word's meanings: the row panel's lone `pgettext("row panel title", "row")` (also used as a plural elsewhere, which xgettext warns about), and the Agents key hint's `pgettext("key hint", "open")`, apart from the picker's tag «открыто». `is required` is reworded into whole phrases.

**Files:** Modify `crates/tabletist-core/src/i18n.rs`; Modify `crates/tabletist-core/src/selection.rs`; Modify `src/app.rs`; Modify `src/ui/agents.rs`; Modify `src/ui/conflict_prompt.rs`; Modify `src/ui/connect_dialog/mod.rs`; Modify `src/ui/copy_as.rs`; Modify `src/ui/data_view.rs`; Modify `src/ui/ddl.rs`; Modify `src/ui/host_key_prompt.rs`; Modify `src/ui/object_tabs.rs`; Modify `src/ui/password_prompt.rs`; Modify `src/ui/pending_bar.rs`; Modify `src/ui/picker.rs`; Modify `src/ui/quick_open.rs`; Modify `src/ui/review.rs`; Modify `src/ui/row_panel.rs`; Modify `src/ui/settings/mod.rs`; Modify `src/ui/settings/sheet.rs`; Modify `src/ui/settings/terminal.rs`; Modify `src/ui/sql_editor.rs`; Modify `src/ui/start.rs`; Modify `src/ui/workspace.rs`; Modify `src/ui/write_prompts.rs`

- [ ] **Step 1: Check what xgettext warns about today**

```bash
mkdir -p target
git ls-files 'src/*.rs' 'crates/*.rs' > target/POTFILES
xgettext --language=Rust --from-code=UTF-8 --keyword= --keyword=gettext:2 --keyword=ngettext:2,3 --keyword=pgettext:2c,3 --files-from=target/POTFILES --output=target/t.pot
```

Expected: two warnings, `msgid 'is required' is used without plural and with plural` and the same for `'row'`.

- [ ] **Step 2: Apply the change**

```diff
--- a/crates/tabletist-core/src/i18n.rs
+++ b/crates/tabletist-core/src/i18n.rs
@@ -10,6 +10,37 @@
 pub fn counted(locale: Locale, count: usize, one: &'static str, many: &'static str) -> String {
     let plural = u32::try_from(count).unwrap_or(u32::MAX);
     format!("{count} {}", ngettext(locale, one, many, plural))
+}
+
+/// `text` either side of its `placeholder`: what is before it and what is
+/// after, for a phrase whose placeholder is drawn in a style of its own. A
+/// text that lost the placeholder (a translation's slip) is all before.
+pub fn around<'a>(text: &'a str, placeholder: &str) -> (&'a str, &'a str) {
+    text.split_once(placeholder).unwrap_or((text, ""))
+}
+
+/// `text` with each of its placeholders (`{table}`) put in from `values`,
+/// all in one pass: a value that holds a placeholder's name (a table called
+/// `{columns}`) is put in as it is, never filled itself.
+pub fn fill(text: &str, values: &[(&str, &str)]) -> String {
+    let mut out = String::with_capacity(text.len());
+    let mut rest = text;
+    while let Some(start) = rest.find('{') {
+        out.push_str(&rest[..start]);
+        let at = &rest[start..];
+        match values.iter().find(|(name, _)| at.starts_with(name)) {
+            Some((name, value)) => {
+                out.push_str(value);
+                rest = &at[name.len()..];
+            }
+            None => {
+                out.push('{');
+                rest = &at[1..];
+            }
+        }
+    }
+    out.push_str(rest);
+    out
 }
 
 #[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
```
```diff
--- a/crates/tabletist-core/src/selection.rs
+++ b/crates/tabletist-core/src/selection.rs
@@ -437,14 +437,16 @@
 /// What a screen reader is told of a range: "4 rows by 2 columns
 /// selected", or "3 rows selected" for whole rows.
 pub fn said(locale: crate::i18n::Locale, (rows, cols): (usize, usize), whole_rows: bool) -> String {
+    if whole_rows {
+        let plural = u32::try_from(rows).unwrap_or(u32::MAX);
+        return crate::i18n::ngettext(locale, "{n} row selected", "{n} rows selected", plural)
+            .replace("{n}", &rows.to_string());
+    }
     let rows = crate::i18n::counted(locale, rows, "row", "rows");
-    let selected = crate::i18n::gettext(locale, "selected");
-    if whole_rows {
-        return format!("{rows} {selected}");
-    }
     let cols = crate::i18n::counted(locale, cols, "column", "columns");
-    let by = crate::i18n::gettext(locale, "by");
-    format!("{rows} {by} {cols} {selected}")
+    crate::i18n::gettext(locale, "{rows} by {cols} selected")
+        .replace("{rows}", &rows)
+        .replace("{cols}", &cols)
 }
 
 #[cfg(test)]
```
```diff
--- a/src/app.rs
+++ b/src/app.rs
@@ -5215,7 +5215,8 @@
                 // takes the statements over.
                 let ran = sql.in_flight.as_ref().map(|run| {
                     let base = crate::history::Entry {
-                        detail: format!("{} {}", gettext(locale, "Query"), sql.number),
+                        detail: gettext(locale, "Query {n}")
+                            .replace("{n}", &sql.number.to_string()),
                         read_only: run.mode == tabletist_db::ScriptMode::ReadOnly,
                         at_ms: base.at_ms.saturating_sub(
                             i64::try_from(run.started.elapsed().as_millis()).unwrap_or_default(),
@@ -9503,18 +9504,16 @@
         Copied::Text { statements, .. } => {
             let size = (copying.rows, copying.cols);
             let what = crate::selection::size_words(locale, size, copying.whole_rows);
-            let copied = gettext(locale, "Copied");
             match how {
                 How::ColumnNames => {
                     let names = counted(locale, copying.cols, "column name", "column names");
-                    format!("{copied} {names}")
-                }
-                How::Plain => format!("{copied} {what} {} TSV", gettext(locale, "as")),
+                    gettext(locale, "Copied {what}").replace("{what}", &names)
+                }
+                How::Plain => gettext(locale, "Copied {what} as TSV").replace("{what}", &what),
                 How::As(format) => {
-                    let said = format!(
-                        "{copied} {what} {} {}",
-                        gettext(locale, "as"),
-                        format.name()
+                    let said = crate::i18n::fill(
+                        &gettext(locale, "Copied {what} as {format}"),
+                        &[("{what}", &what), ("{format}", format.name())],
                     );
                     if format == crate::export::Format::SqlInsert {
                         let made = counted(locale, *statements, "statement", "statements");
```
```diff
--- a/src/ui/agents.rs
+++ b/src/ui/agents.rs
@@ -226,26 +226,29 @@
                 }
                 left = child.min_rect().right().max(place.right()) + 12.0;
             }
-            let on = look.label(&gettext(locale, "Agents on"));
+            // The words either side of the connection's name, which is
+            // drawn in its own weight.
+            let on = look.label(&gettext(locale, "Agents on {connection}"));
+            let (before, after) = crate::i18n::around(&on, "{connection}");
             // A connection's name gives way at its end to a narrow tab.
             let strong = TextRole::pick(&look, TextRole::UiBodyStrong, TextRole::OBody);
             let body = widgets::body(&look);
-            let around = body.width(ui.ctx(), look.faces, &format!("{on}  · {access}"));
+            let around = body.width(ui.ctx(), look.faces, &format!("{before}{after} · {access}"));
             let room = (right - left - around).max(0.0);
             let name = crate::ui::grid::ellipsize(&connection, room, false, |text| {
                 strong.width(ui.ctx(), look.faces, text)
             });
             let line = Text::new(&look)
-                .add(widgets::body(&look), &on, palette.secondary)
-                .space(widgets::body(&look), " ")
+                .add(widgets::body(&look), before, palette.secondary)
                 .add(strong, &name, palette.text)
                 .add(
                     widgets::body(&look),
-                    &format!(" · {access}"),
+                    &format!("{after} · {access}"),
                     palette.secondary,
                 );
             widgets::paint_text(ui, left, full.center().y, line);
-            widgets::announce(ui, full, &format!("{on} {connection} · {access}"));
+            let said = on.replace("{connection}", &connection);
+            widgets::announce(ui, full, &format!("{said} · {access}"));
         });
     if !look.terminal {
         status_bar(ui, tab, (&here, &show_key), (&look, &palette, locale));
@@ -318,7 +321,8 @@
                     }
                 }
                 AgentsHere::OffHere => {
-                    let title = format!("{} {connection}", gettext(locale, "Agents can't use"));
+                    let title = gettext(locale, "Agents can't use {connection}")
+                        .replace("{connection}", &connection);
                     let notice = states::Notice {
                         icon: Icon::SquareTerminal,
                         title: &title,
@@ -431,7 +435,7 @@
         AgentsHere::ServerOff => gettext(locale, "Server: off"),
         _ => gettext(locale, "Server: stdio"),
     };
-    let shows = format!("{} {}", key.as_str(), gettext(locale, "shows this tab"));
+    let shows = gettext(locale, "{key} shows this tab").replace("{key}", key.as_str());
     egui::Panel::bottom(Id::new(("agents-status", tab.0)))
         .exact_size(STATUS)
         .resizable(false)
@@ -488,7 +492,8 @@
         text.job_mut().halign = align;
         text.layout(&ctx)
     };
-    let title = format!("{} {connection}", gettext(locale, "No agents connected to"));
+    let title =
+        gettext(locale, "No agents connected to {connection}").replace("{connection}", connection);
     let text = gettext(
         locale,
         "Agents like Claude Code can read this database. Connect one with this command, then ask it about {name}.",
@@ -641,7 +646,10 @@
     /// "Cursor on Bookshop · staging", or "Cursor · no connection yet".
     fn said(&self, locale: crate::i18n::Locale) -> String {
         match &self.on {
-            Some((_, name)) => format!("{} {} {name}", self.agent, gettext(locale, "on")),
+            Some((_, name)) => crate::i18n::fill(
+                &gettext(locale, "{agent} on {connection}"),
+                &[("{agent}", &self.agent), ("{connection}", name)],
+            ),
             None => format!("{} · {}", self.agent, gettext(locale, "no connection yet")),
         }
     }
@@ -782,7 +790,7 @@
         let small = widgets::secondary(look);
         let mut said = format!("{agent} · {facts}");
         if let Some(runs_as) = runs_as {
-            let runs = format!("{} {runs_as}", gettext(locale, "Runs as"));
+            let runs = gettext(locale, "Runs as {environment}").replace("{environment}", runs_as);
             let laid = Text::one(look, small, &runs, palette.secondary).layout(ui.ctx());
             laid.paint_left(ui.painter(), right - laid.width(), line.center().y);
             right -= laid.width() + 16.0;
@@ -820,11 +828,10 @@
     if let Some(project) = &session.project {
         facts.push(project.clone());
     }
-    facts.push(format!(
-        "{} {}",
-        gettext(locale, "connected"),
-        crate::clock::hour(session.connected_at)
-    ));
+    facts.push(
+        gettext(locale, "connected {time}")
+            .replace("{time}", &crate::clock::hour(session.connected_at)),
+    );
     facts.push(
         ngettext(
             locale,
@@ -870,7 +877,8 @@
                 widgets::hline(ui, line.x_range(), line.bottom() - 0.5, palette.outline);
                 let mut facts = session_facts(session, locale);
                 if let Some(runs_as) = runs_as {
-                    facts = format!("{facts} · {} {runs_as}", gettext(locale, "runs as"));
+                    let runs = gettext(locale, "runs as {environment}");
+                    facts = format!("{facts} · {}", runs.replace("{environment}", runs_as));
                 }
                 let agent = look.label(&crate::model::agent_name(session));
                 let strong = TextRole::pick(look, TextRole::UiBodyStrong, TextRole::OBody);
@@ -933,7 +941,7 @@
     show_key: &crate::keymap::Written,
     (look, palette, locale): (&Look, &Palette, crate::i18n::Locale),
 ) {
-    let shows = format!("{} {}", show_key.as_str(), gettext(locale, "shows"));
+    let shows = gettext(locale, "{key} shows").replace("{key}", show_key.as_str());
     // A narrow window gives the calls the room the list would take.
     let nav = NAV.min(ui.available_width() * 0.35);
     egui::Panel::left(Id::new(("agents-sessions", tab.0)))
@@ -1875,17 +1883,16 @@
         // "filters  agent cursor · result refused · search “select”".
         let mut said = Vec::new();
         if let Some(agent) = &log.agent {
-            said.push(format!("{} {}", gettext(locale, "agent"), agent));
+            said.push(gettext(locale, "agent {name}").replace("{name}", agent));
         }
         if let Some(result) = &log.result {
-            said.push(format!(
-                "{} {}",
-                gettext(locale, "result"),
-                result_word(result, locale)
-            ));
+            said.push(
+                gettext(locale, "result {result}")
+                    .replace("{result}", &result_word(result, locale)),
+            );
         }
         if !log.search.is_empty() {
-            said.push(format!("{} “{}”", gettext(locale, "search"), log.search));
+            said.push(gettext(locale, "search “{text}”").replace("{text}", &log.search));
         }
         let line = Rect::from_min_size(pos2(rect.left(), top), vec2(rect.width(), FILTER_ROW));
         let words = look.label(&said.join(" · "));
@@ -2123,7 +2130,7 @@
                         count("{n} column", "{n} columns", columns)
                     );
                 }
-                said.push(format!("{returned} {}", gettext(locale, "returned")));
+                said.push(gettext(locale, "{what} returned").replace("{what}", &returned));
             }
             (_, None) => {}
         }
```
```diff
--- a/src/ui/conflict_prompt.rs
+++ b/src/ui/conflict_prompt.rs
@@ -161,8 +161,11 @@
     let table = format::display_safe(&object.object.name);
     let mut place = format!("{} · {table}", workspace.name);
     if of > 1 {
-        let word = look.label(&gettext(locale, "of"));
-        place.push_str(&format!(" · {} {word} {of}", at + 1));
+        let which = look
+            .label(&gettext(locale, "{at} of {count}"))
+            .replace("{at}", &(at + 1).to_string())
+            .replace("{count}", &of.to_string());
+        place.push_str(&format!(" · {which}"));
     }
     place
 }
```
```diff
--- a/src/ui/connect_dialog/mod.rs
+++ b/src/ui/connect_dialog/mod.rs
@@ -17,7 +17,7 @@
 
 use crate::connections::PasswordMode;
 use crate::env::{EnvColors, Environment, Platform, env_colors};
-use crate::i18n::{Locale, gettext};
+use crate::i18n::{Locale, gettext, ngettext};
 use crate::keymap::{Command, Keymap, Scope};
 use crate::model::{
     Action, ConnectionForm, FilesKind, LoadedTables, SshAuthKind, SshHints, TestState,
@@ -1116,9 +1116,12 @@
             TestState::Untrusted {
                 host, fingerprint, ..
             } => {
-                let text = format!(
-                    "{} {host}: {fingerprint}",
-                    gettext(skin.locale, "Unknown SSH host key for")
+                let text = crate::i18n::fill(
+                    &gettext(
+                        skin.locale,
+                        "Unknown SSH host key for {host}: {fingerprint}",
+                    ),
+                    &[("{host}", host), ("{fingerprint}", fingerprint)],
                 );
                 texts.push((text.into(), palette.warning));
                 untrusted = true;
@@ -1281,15 +1284,17 @@
             let mut text = match read {
                 // What a Files connection's Test read: "Read 3 files ·
                 // 2.4 GB".
-                Some(read) => format!(
-                    "{} {} · {}",
-                    gettext(skin.locale, "Read"),
-                    match read.files {
-                        1 => gettext(skin.locale, "1 file").into_owned(),
-                        files => format!("{files} {}", gettext(skin.locale, "files")),
-                    },
-                    tabletist_db::files::size_text(read.bytes)
-                ),
+                Some(read) => {
+                    let plural = u32::try_from(read.files).unwrap_or(u32::MAX);
+                    ngettext(
+                        skin.locale,
+                        "Read {n} file · {size}",
+                        "Read {n} files · {size}",
+                        plural,
+                    )
+                    .replace("{n}", &read.files.to_string())
+                    .replace("{size}", &tabletist_db::files::size_text(read.bytes))
+                }
                 None => format!("{} · {}", gettext(skin.locale, "Connected"), driver.label()),
             };
             if let Some(took) = form.test_took {
```
```diff
--- a/src/ui/copy_as.rs
+++ b/src/ui/copy_as.rs
@@ -65,7 +65,7 @@
         CopyHow::ColumnNames => gettext(locale, "Column names").into_owned(),
     };
     if row {
-        format!("{} {name}", gettext(locale, "Copy row as"))
+        gettext(locale, "Copy row as {format}").replace("{format}", &name)
     } else {
         name
     }
```
```diff
--- a/src/ui/data_view.rs
+++ b/src/ui/data_view.rs
@@ -71,12 +71,14 @@
     let cells = object.range.as_ref().is_some_and(|range| !range.rows());
     if let Some(size) = object.selection_size().filter(|_| cells) {
         let size = crate::selection::size_words(locale, size, false);
-        return Some(format!("{size} {}", gettext(locale, "selected")));
+        return Some(gettext(locale, "{size} selected").replace("{size}", &size));
     }
     // Several rows: how many, whatever kind each is.
     let rows = object.selected_rows().len();
     if rows > 1 {
-        return Some(format!("{rows} {}", gettext(locale, "rows selected")));
+        let plural = u32::try_from(rows).unwrap_or(u32::MAX);
+        let said = ngettext(locale, "{n} row selected", "{n} rows selected", plural);
+        return Some(said.replace("{n}", &rows.to_string()));
     }
     let Some(id) = crate::edit::new_id(cell.row) else {
         return Some(gettext(locale, "1 row selected").into_owned());
@@ -88,8 +90,11 @@
         return Some(new.into_owned());
     }
     let plural = u32::try_from(missing).unwrap_or(u32::MAX);
-    let fields = ngettext(locale, "required field", "required fields", plural);
-    Some(format!("{new} · {missing} {fields}"))
+    let fields = ngettext(locale, "{n} required field", "{n} required fields", plural);
+    Some(format!(
+        "{new} · {}",
+        fields.replace("{n}", &missing.to_string())
+    ))
 }
 
 /// The parts of "13 rows · 1 to delete · 6 columns · public", as far as
@@ -115,7 +120,10 @@
     });
     // The rows to add, beside the table's own: "13 rows + 1 new".
     let added = object.edits.added.len();
-    let new = (added > 0).then(|| format!("{added} {}", gettext(locale, "new")));
+    let new = (added > 0).then(|| {
+        let plural = u32::try_from(added).unwrap_or(u32::MAX);
+        ngettext(locale, "{n} new", "{n} new", plural).replace("{n}", &added.to_string())
+    });
     match (rows, new) {
         (Some(rows), Some(new)) => parts.push(format!("{rows} + {new}")),
         (Some(one), None) | (None, Some(one)) => parts.push(one),
@@ -124,7 +132,9 @@
     // The rows to delete, beside them: "13 rows · 1 to delete".
     let deleted = object.edits.deleted.len();
     if deleted > 0 {
-        parts.push(format!("{deleted} {}", gettext(locale, "to delete")));
+        let plural = u32::try_from(deleted).unwrap_or(u32::MAX);
+        let to_delete = ngettext(locale, "{n} to delete", "{n} to delete", plural);
+        parts.push(to_delete.replace("{n}", &deleted.to_string()));
     }
     if let Some(page) = page {
         let count = page.columns.len();
@@ -659,7 +669,7 @@
     let measure = |text: &str, role: TextRole| role.width(ui.ctx(), look.faces, text);
     if look.terminal {
         // One bordered line: "sort created_at ↑", 8 at its sides.
-        let text = format!("{} {column}", gettext(locale, "sort"));
+        let text = gettext(locale, "sort {column}").replace("{column}", &column);
         let role = TextRole::OBody;
         let height = line(ui, role, look) + 2.0 + 2.0;
         let width = measure(&text, role) + 16.0 + 2.0;
@@ -1211,11 +1221,7 @@
         .map(|saved| super::pending_bar::written_text(saved, locale));
     let timing = written.or_else(|| {
         page.map(|page| {
-            format!(
-                "{} {}",
-                gettext(locale, "Query"),
-                format::elapsed(page.elapsed)
-            )
+            gettext(locale, "Query {elapsed}").replace("{elapsed}", &format::elapsed(page.elapsed))
         })
     });
     let unordered = page.is_some_and(|page| !page.ordered_by_key) && object.query.sort.is_empty();
@@ -1655,17 +1661,22 @@
     // The last row that was marked for deletion or restored, or what the
     // last save deleted: said to a screen reader.
     let told = match (&object.edits.told, &object.edits.saved) {
-        (Some(Told::Marked { row, deleted }), _) => Some(format!(
-            "{} {row} {} {deleted} {}",
-            gettext(locale, "Row"),
-            gettext(locale, "marked for deletion."),
-            gettext(locale, "to delete.")
-        )),
-        (Some(Told::Restored { row }), _) => Some(format!(
-            "{} {row} {}",
-            gettext(locale, "Row"),
-            gettext(locale, "restored.")
-        )),
+        (Some(Told::Marked { row, deleted }), _) => {
+            let plural = u32::try_from(*deleted).unwrap_or(u32::MAX);
+            let said = ngettext(
+                locale,
+                "Row {row} marked for deletion. {n} to delete.",
+                "Row {row} marked for deletion. {n} to delete.",
+                plural,
+            );
+            Some(
+                said.replace("{n}", &deleted.to_string())
+                    .replace("{row}", row),
+            )
+        }
+        (Some(Told::Restored { row }), _) => {
+            Some(gettext(locale, "Row {row} restored.").replace("{row}", row))
+        }
         (
             Some(Told::MarkedRows {
                 marked,
@@ -1678,7 +1689,9 @@
             let mut parts = Vec::new();
             if *marked > 0 {
                 parts.push(super::pending_bar::marked_text(*marked, *skipped, locale));
-                parts.push(format!("{deleted} {}", gettext(locale, "to delete.")));
+                let plural = u32::try_from(*deleted).unwrap_or(u32::MAX);
+                let to_delete = ngettext(locale, "{n} to delete.", "{n} to delete.", plural);
+                parts.push(to_delete.replace("{n}", &deleted.to_string()));
             }
             if *dropped > 0 {
                 let rows = super::pending_bar::counted(
@@ -1707,7 +1720,7 @@
         let whole_rows = mode == crate::selection::Visual::Rows;
         let size = object.selection_size().unwrap_or((1, 1));
         let held = crate::selection::said(locale, size, whole_rows);
-        format!("{} {held}.", gettext(locale, "Visual."))
+        gettext(locale, "Visual. {selection}.").replace("{selection}", &held)
     });
     // The tab itself from here on: the field on a cell edits the text its
     // editor holds, beside the page the grid reads. Nothing else of it is
@@ -2539,7 +2552,7 @@
                 Some(loaded) => {
                     cell.mark = if saving { Mark::Saving } else { Mark::Pending };
                     let was = format::cell_text(loaded);
-                    let was = format!("{} {was}", gettext(locale, "was"));
+                    let was = gettext(locale, "was {value}").replace("{value}", &was);
                     // A pending DEFAULT says what it will come to.
                     cell.hint = Some(match default_hint(&pending.new, locale) {
                         Some(default) => format!("{was}\n{default}"),
@@ -2563,10 +2576,9 @@
                     message
                 } else {
                     format!(
-                        "{message}\n{} · {} {}",
+                        "{message}\n{} · {}",
                         gettext(locale, "Checked before saving"),
-                        self.undo_key,
-                        gettext(locale, "reverts")
+                        gettext(locale, "{key} reverts").replace("{key}", &self.undo_key)
                     )
                 });
             }
@@ -2673,11 +2685,10 @@
     else {
         return None;
     };
-    Some(format!(
-        "{} {}",
-        format::capped(expression),
-        gettext(locale, "from DEFAULT")
-    ))
+    Some(
+        gettext(locale, "{expression} from DEFAULT")
+            .replace("{expression}", &format::capped(expression)),
+    )
 }
 
 /// What the database said of a statement that failed, with its code: a
```
```diff
--- a/src/ui/ddl.rs
+++ b/src/ui/ddl.rs
@@ -16,7 +16,7 @@
 use tabletist_db::{Ddl, Driver, ObjectKind};
 
 use crate::format;
-use crate::i18n::{Locale, gettext};
+use crate::i18n::{Locale, gettext, ngettext};
 use crate::keymap::Command;
 use crate::model::{Action, ConnTabId, ObjectTab, TabId};
 use crate::style::Icon;
@@ -247,17 +247,32 @@
 /// how long ago the definition arrived.
 pub fn read_ago(waited: Duration, locale: Locale) -> String {
     let seconds = waited.as_secs();
-    let ago = |count: u64, unit: &str| {
-        gettext(locale, "read {count} {unit} ago")
-            .replace("{count}", &count.to_string())
-            .replace("{unit}", unit)
+    let plural = |count: u64| u32::try_from(count).unwrap_or(u32::MAX);
+    // A whole phrase for each unit, so a translation can spell its unit
+    // out and say it for the count.
+    let (count, said) = match seconds {
+        0 => return gettext(locale, "read just now").into_owned(),
+        1..60 => (
+            seconds,
+            ngettext(locale, "read {n} s ago", "read {n} s ago", plural(seconds)),
+        ),
+        60..3600 => {
+            let minutes = seconds / 60;
+            let said = ngettext(
+                locale,
+                "read {n} min ago",
+                "read {n} min ago",
+                plural(minutes),
+            );
+            (minutes, said)
+        }
+        _ => {
+            let hours = seconds / 3600;
+            let said = ngettext(locale, "read {n} h ago", "read {n} h ago", plural(hours));
+            (hours, said)
+        }
     };
-    match seconds {
-        0 => gettext(locale, "read just now").into_owned(),
-        1..60 => ago(seconds, "s"),
-        60..3600 => ago(seconds / 60, "min"),
-        _ => ago(seconds / 3600, "h"),
-    }
+    said.replace("{n}", &count.to_string())
 }
 
 /// The terminal header's summary in the DDL view: "ddl · read-only ·
```
```diff
--- a/src/ui/host_key_prompt.rs
+++ b/src/ui/host_key_prompt.rs
@@ -24,16 +24,13 @@
             &look,
         );
         ui.add_space(4.0);
-        let message = format!(
-            "{}:{} {}",
-            prompt.host,
-            prompt.port,
-            gettext(
-                locale,
-                "has a key this computer has not seen before. \
-                 Compare its fingerprint with the server's before trusting it."
-            )
-        );
+        let message = gettext(
+            locale,
+            "{host}:{port} has a key this computer has not seen before. \
+             Compare its fingerprint with the server's before trusting it.",
+        )
+        .replace("{port}", &prompt.port.to_string())
+        .replace("{host}", &prompt.host);
         let width = ui.available_width();
         Text::one(&look, widgets::body(&look), &message, palette.secondary)
             .wrap(width)
```
```diff
--- a/src/ui/object_tabs.rs
+++ b/src/ui/object_tabs.rs
@@ -66,7 +66,7 @@
             }
             model::Tab::Sql(sql) => (
                 sql.id,
-                format!("{} {}", gettext(locale, "Query"), sql.number),
+                gettext(locale, "Query {n}").replace("{n}", &sql.number.to_string()),
                 true,
                 Kind::Sql,
                 false,
@@ -165,12 +165,11 @@
                                     focus::claim(ui, focus::Region::Tabs, &response);
                                 }
                                 let label = if *unsaved {
-                                    let (tab, unsaved) =
-                                        (gettext(locale, "tab"), gettext(locale, "unsaved"));
-                                    format!("{name} {tab}, {unsaved}")
+                                    gettext(locale, "{name} tab, unsaved")
                                 } else {
-                                    format!("{name} {}", gettext(locale, "tab"))
+                                    gettext(locale, "{name} tab")
                                 };
+                                let label = label.replace("{name}", name);
                                 response.widget_info(|| {
                                     WidgetInfo::selected(
                                         WidgetType::Button,
```
```diff
--- a/src/ui/password_prompt.rs
+++ b/src/ui/password_prompt.rs
@@ -16,15 +16,12 @@
         ctx,
         |ui| {
             ui.set_width(360.0);
-            let title = format!(
-                "{} {}",
-                match prompt.kind {
-                    SecretKind::Database => gettext(locale, "Password for"),
-                    SecretKind::SshPassword => gettext(locale, "SSH password for"),
-                    SecretKind::SshPassphrase => gettext(locale, "Key passphrase for"),
-                },
-                prompt.name
-            );
+            let title = match prompt.kind {
+                SecretKind::Database => gettext(locale, "Password for {connection}"),
+                SecretKind::SshPassword => gettext(locale, "SSH password for {connection}"),
+                SecretKind::SshPassphrase => gettext(locale, "Key passphrase for {connection}"),
+            };
+            let title = title.replace("{connection}", &prompt.name);
             let title = widgets::label(
                 ui,
                 widgets::dialog_title(&look),
```
```diff
--- a/src/ui/pending_bar.rs
+++ b/src/ui/pending_bar.rs
@@ -51,7 +51,14 @@
     if skipped == 0 {
         return format!("{rows}.");
     }
-    format!("{rows}. {skipped} {}", gettext(locale, "can't be deleted."))
+    let plural = u32::try_from(skipped).unwrap_or(u32::MAX);
+    let left = ngettext(
+        locale,
+        "{n} can't be deleted.",
+        "{n} can't be deleted.",
+        plural,
+    );
+    format!("{rows}. {}", left.replace("{n}", &skipped.to_string()))
 }
 
 /// How many foreign keys name no row: "1 value names no row". What a save
@@ -73,16 +80,21 @@
         parts.push(if short {
             changes
         } else {
-            format!(
-                "{changes} {} {}",
-                gettext(locale, "in"),
-                counted(locale, counts.rows, "row", "rows")
+            let plural = u32::try_from(counts.rows).unwrap_or(u32::MAX);
+            ngettext(
+                locale,
+                "{changes} in {n} row",
+                "{changes} in {n} rows",
+                plural,
             )
+            .replace("{n}", &counts.rows.to_string())
+            .replace("{changes}", &changes)
         });
     }
     if counts.deleted > 0 {
-        let to_delete = gettext(locale, "to delete");
-        parts.push(format!("{} {to_delete}", counts.deleted));
+        let plural = u32::try_from(counts.deleted).unwrap_or(u32::MAX);
+        let to_delete = ngettext(locale, "{n} to delete", "{n} to delete", plural);
+        parts.push(to_delete.replace("{n}", &counts.deleted.to_string()));
     }
     if doubts > 0 && !short {
         parts.push(doubts_text(doubts, locale));
@@ -150,20 +162,17 @@
     own: impl Fn(&str) -> String,
 ) -> String {
     let plural = u32::try_from(holder.rows).unwrap_or(u32::MAX);
-    let points = ngettext(
+    let said = ngettext(
         locale,
-        "still points at this row",
-        "still point at this row",
+        "{n} row in {table} still points at this row",
+        "{n} rows in {table} still point at this row",
         plural,
     );
-    format!(
-        "{} {} {} {} {}",
-        holder.rows,
-        own(&ngettext(locale, "row", "rows", plural)),
-        own(&gettext(locale, "in")),
-        format::display_safe(&holder.table.name),
-        own(&points)
-    )
+    // The words are the app's own, and the placeholders are lower case
+    // already: `own` leaves them for the count and the table's own name.
+    own(&said)
+        .replace("{n}", &holder.rows.to_string())
+        .replace("{table}", &format::display_safe(&holder.table.name))
 }
 
 /// What `note` says, the app's own words put through `own`: the values of
@@ -179,15 +188,20 @@
     match note {
         Note::Conflict { row, gone, others } => {
             let what = if *gone {
-                say("no longer exists on the server.")
+                gettext(locale, "Row {row} no longer exists on the server.")
             } else {
-                say("changed on the server.")
-            };
-            let mut text = format!("{} {} {what} {nothing}", say("Row"), row_name(object, *row));
+                gettext(locale, "Row {row} changed on the server.")
+            };
+            // The row's key is the table's own: put in after `own`.
+            let what = own(&what).replace("{row}", &row_name(object, *row));
+            let mut text = format!("{what} {nothing}");
             if *others > 0 {
                 let plural = u32::try_from(*others).unwrap_or(u32::MAX);
-                let more = own(&ngettext(locale, "more row too.", "more rows too.", plural));
-                text.push_str(&format!(" {others} {more}"));
+                let more = ngettext(locale, "{n} more row too.", "{n} more rows too.", plural);
+                text.push_str(&format!(
+                    " {}",
+                    own(&more).replace("{n}", &others.to_string())
+                ));
             }
             text
         }
@@ -263,7 +277,7 @@
         ));
     }
     parts.push(format::elapsed(saved.elapsed));
-    format!("{} {}", gettext(locale, "written"), parts.join(" · "))
+    gettext(locale, "written {what}").replace("{what}", &parts.join(" · "))
 }
 
 /// Why Save cannot be pressed, as its tooltip says it, and the terminal's
@@ -273,11 +287,13 @@
         SaveBlock::Saving => gettext(locale, "Saving…").into_owned(),
         SaveBlock::ToFix => {
             let plural = u32::try_from(to_fix).unwrap_or(u32::MAX);
-            format!(
-                "{} {to_fix} {}",
-                gettext(locale, "Fix"),
-                ngettext(locale, "value to save", "values to save", plural)
+            ngettext(
+                locale,
+                "Fix {n} value to save",
+                "Fix {n} values to save",
+                plural,
             )
+            .replace("{n}", &to_fix.to_string())
         }
         SaveBlock::Required => gettext(locale, "Fill required fields to save").into_owned(),
         SaveBlock::Disconnected => gettext(locale, "Not connected").into_owned(),
@@ -366,12 +382,17 @@
     // What the tab's new rows still need: the column, or how many.
     let needs = match lacking.as_slice() {
         [] => None,
-        [one] => Some(format!("{one} {}", gettext(locale, "is required"))),
-        more => Some(format!(
-            "{} {}",
-            more.len(),
-            gettext(locale, "values are required")
-        )),
+        [one] => Some(gettext(locale, "{column} is required").replace("{column}", one)),
+        more => {
+            let plural = u32::try_from(more.len()).unwrap_or(u32::MAX);
+            let said = ngettext(
+                locale,
+                "{n} value is required",
+                "{n} values are required",
+                plural,
+            );
+            Some(said.replace("{n}", &more.len().to_string()))
+        }
     };
     let mut actions = Vec::new();
     egui::Panel::bottom(Id::new(("pending-bar", tab.0, id.0)))
@@ -413,8 +434,11 @@
             let short = pending.then(|| counts_text(counts, doubts, true, locale));
             // What stands against a save, in one place: the values to fix,
             // and what a new row still needs.
-            let fix = (counts.to_fix > 0)
-                .then(|| format!("{} {}", counts.to_fix, gettext(locale, "to fix")));
+            let fix = (counts.to_fix > 0).then(|| {
+                let plural = u32::try_from(counts.to_fix).unwrap_or(u32::MAX);
+                ngettext(locale, "{n} to fix", "{n} to fix", plural)
+                    .replace("{n}", &counts.to_fix.to_string())
+            });
             // Nothing is to fix, and a new row needs a value: no mistake
             // to mark. A dot and the words, as the design has it.
             let asked = fix.is_none() && needs.is_some();
```
```diff
--- a/src/ui/picker.rs
+++ b/src/ui/picker.rs
@@ -1466,7 +1466,7 @@
         Text::one(
             look,
             small,
-            &format!("{} {when}", gettext(locale, "Last used")),
+            &gettext(locale, "Last used {when}").replace("{when}", &when),
             palette.dim,
         ),
     );
@@ -1517,11 +1517,8 @@
     if response.clicked() {
         actions.push(Action::EditConnection(connection.id.clone()));
     }
-    let more_label = format!(
-        "{} {}",
-        gettext(locale, "More actions for"),
-        connection.name
-    );
+    let more_label =
+        gettext(locale, "More actions for {connection}").replace("{connection}", &connection.name);
     let more = ButtonSpec::new("")
         .icon(Icon::Ellipsis)
         .gap(0.0)
@@ -1656,9 +1653,9 @@
     let (agents, agents_color) = match opening.agents {
         Some(access) => (
             format!(
-                " · {} {}",
-                gettext(locale, "agents"),
-                agents_level(access, locale).to_lowercase()
+                " · {}",
+                gettext(locale, "agents {level}")
+                    .replace("{level}", &agents_level(access, locale).to_lowercase())
             ),
             match access {
                 crate::connections::AgentAccess::Off => palette.dim,
```
```diff
--- a/src/ui/quick_open.rs
+++ b/src/ui/quick_open.rs
@@ -1135,19 +1135,23 @@
         widgets::announce(ui, rect, &said);
         return;
     }
-    let place = if everywhere {
-        gettext(locale, "open connections").into_owned()
-    } else if look.terminal {
-        skin.windows
-            .get(&skin.here)
-            .map(|window| display_safe(&window.database).into_owned())
-            .unwrap_or_default()
+    let said = if everywhere {
+        gettext(locale, "No objects match “{query}” in open connections")
+            .replace("{query}", &display_safe(query))
     } else {
-        skin.window_name(skin.here)
+        let place = if look.terminal {
+            skin.windows
+                .get(&skin.here)
+                .map(|window| display_safe(&window.database).into_owned())
+                .unwrap_or_default()
+        } else {
+            skin.window_name(skin.here)
+        };
+        crate::i18n::fill(
+            &gettext(locale, "No objects match “{query}” in {place}"),
+            &[("{query}", &display_safe(query)), ("{place}", &place)],
+        )
     };
-    let said = gettext(locale, "No objects match “{query}” in {place}")
-        .replace("{query}", &display_safe(query))
-        .replace("{place}", &place);
     let said = look.label(&said);
     let link = gettext(locale, "Search all open connections");
     // Export a table… picks one of this connection's: no other to search.
```
```diff
--- a/src/ui/review.rs
+++ b/src/ui/review.rs
@@ -98,12 +98,16 @@
         ),
         Line::NewRequired(columns) => {
             let plural = u32::try_from(columns.len()).unwrap_or(u32::MAX);
+            let blocked = ngettext(
+                locale,
+                "blocked: {columns} is required",
+                "blocked: {columns} are required",
+                plural,
+            );
             format!(
-                "-- {} · {} {} {}",
+                "-- {} · {}",
                 say("new row"),
-                say("blocked:"),
-                columns.join(", "),
-                ngettext(locale, "is required", "are required", plural)
+                blocked.replace("{columns}", &columns.join(", "))
             )
         }
         Line::NewRefused(reason) => {
@@ -125,35 +129,42 @@
         Line::Also(Effect::Deletes { table, rows }) => {
             let what = match rows {
                 Some(rows) => {
-                    let rows = usize::try_from(*rows).unwrap_or(usize::MAX);
-                    counted(locale, rows, "row", "rows")
+                    let plural = u32::try_from(*rows).unwrap_or(u32::MAX);
+                    ngettext(
+                        locale,
+                        "also deletes {n} row in {table}",
+                        "also deletes {n} rows in {table}",
+                        plural,
+                    )
+                    .replace("{n}", &rows.to_string())
                 }
-                None => say("rows").into_owned(),
+                None => gettext(locale, "also deletes rows in {table}").into_owned(),
             };
-            format!(
-                "-- {} {what} {} {table} (ON DELETE CASCADE)",
-                say("also deletes"),
-                say("in")
-            )
+            format!("-- {} (ON DELETE CASCADE)", what.replace("{table}", table))
         }
         Line::Also(Effect::Clears {
             table,
             columns,
             rows,
         }) => {
-            let rows = match rows {
+            let what = match rows {
                 Some(rows) => {
                     let plural = u32::try_from(*rows).unwrap_or(u32::MAX);
-                    format!("{rows} {table} {}", ngettext(locale, "row", "rows", plural))
+                    ngettext(
+                        locale,
+                        "also clears {columns} in {n} {table} row",
+                        "also clears {columns} in {n} {table} rows",
+                        plural,
+                    )
+                    .replace("{n}", &rows.to_string())
                 }
-                None => format!("{table} {}", say("rows")),
+                None => gettext(locale, "also clears {columns} in {table} rows").into_owned(),
             };
-            format!(
-                "-- {} {} {} {rows} (ON DELETE SET NULL)",
-                say("also clears"),
-                columns.join(", "),
-                say("in")
-            )
+            let what = crate::i18n::fill(
+                &what,
+                &[("{table}", table), ("{columns}", &columns.join(", "))],
+            );
+            format!("-- {what} (ON DELETE SET NULL)")
         }
         Line::Sql(_) => return None,
     })
```
```diff
--- a/src/ui/row_panel.rs
+++ b/src/ui/row_panel.rs
@@ -11,7 +11,7 @@
 
 use crate::edit::{Editor, Lock, State, Table, Unset};
 use crate::format;
-use crate::i18n::gettext;
+use crate::i18n::{gettext, ngettext, pgettext};
 use crate::keymap::Command;
 use crate::model::{Action, CellPos, ConnTabId, RowFields, Tab, TabId, Workspace};
 use crate::style::Icon;
@@ -353,7 +353,7 @@
         Tab::Sql(sql) => {
             let (columns, rows, _) = sql.shown_rows()?;
             Some(Source {
-                name: format!("{} {}", gettext(locale, "Query"), sql.number),
+                name: gettext(locale, "Query {n}").replace("{n}", &sql.number.to_string()),
                 columns,
                 rows,
                 // The cell only while its row shows: under the Messages
@@ -656,7 +656,10 @@
                 let mut x = header.left() + side;
                 let (word, ink) = match fresh {
                     Some(_) => (format!("+ {}", gettext(locale, "new row")), green.bar),
-                    None => (gettext(locale, "row").into_owned(), palette.text),
+                    None => (
+                        pgettext(locale, "row panel title", "row").into_owned(),
+                        palette.text,
+                    ),
                 };
                 let word = Text::one(&look, TextRole::OGroup, &word, ink);
                 x += widgets::paint_label(ui, x, y, word) + 10.0;
@@ -795,10 +798,11 @@
             } else {
                 let title = match (&key_column, &key_value, &fresh) {
                     (_, _, Some(_)) => gettext(locale, "New row").into_owned(),
-                    (Some(key), Some(value), None) => {
-                        format!("{} · {key} {value}", gettext(locale, "Row"))
-                    }
-                    _ => format!("{} {number}", gettext(locale, "Row")),
+                    (Some(key), Some(value), None) => crate::i18n::fill(
+                        &gettext(locale, "Row · {key} {value}"),
+                        &[("{key}", key), ("{value}", value)],
+                    ),
+                    _ => gettext(locale, "Row {n}").replace("{n}", &number.to_string()),
                 };
                 // The title over the table's name, centred in the 52.
                 let (title_role, sub_role) = (TextRole::UiBodySemibold, TextRole::FieldLabel);
@@ -1481,7 +1485,9 @@
     };
     ui.add_space(if look.terminal { 2.0 } else { 3.0 });
     let role = caption(look);
-    let text = format!("{} {}", look.label(&gettext(locale, "was")), pending.was);
+    let text = look
+        .label(&gettext(locale, "was {value}"))
+        .replace("{value}", &pending.was);
     let width = ui.available_width();
     if look.terminal {
         // Its key reverts: `u`.
@@ -1646,7 +1652,7 @@
     };
     // A screen reader hears the mark as a word.
     let name = if pending {
-        format!("{text}, {}", gettext(locale, "pending"))
+        gettext(locale, "{value}, pending").replace("{value}", &text)
     } else {
         text.clone()
     };
@@ -1675,8 +1681,10 @@
         },
     );
     if let Some(pending) = pending_cell.filter(|_| pending) {
-        let from = gettext(locale, "changed from");
-        said = format!("{said}, {from} {}", pending.was);
+        said = crate::i18n::fill(
+            &gettext(locale, "{value}, changed from {old}"),
+            &[("{value}", &said), ("{old}", &pending.was)],
+        );
     }
     if let Some(words) = &info.doubt {
         said = format!("{said}, {words}");
@@ -1694,7 +1702,7 @@
     // the terminal look's has its hints: there it is said under the value.
     let was = pending_cell
         .filter(|_| pending && !look.terminal && doc.is_none())
-        .map(|cell| format!("{} {}", gettext(locale, "was"), cell.was));
+        .map(|cell| gettext(locale, "was {value}").replace("{value}", &cell.was));
     let revert = gettext(locale, "revert");
     let revert_width = measure(" · ") + measure(&revert);
     // The label gives way to it, down to a few words of what the cell was.
@@ -2762,11 +2770,8 @@
             ("gd open".to_owned(), target.clone())
         } else {
             (
-                format!(
-                    "{} {} →",
-                    gettext(locale, "Open"),
-                    singular(&format::display_safe(&target.name))
-                ),
+                gettext(locale, "Open {table} →")
+                    .replace("{table}", &singular(&format::display_safe(&target.name))),
                 target.clone(),
             )
         }
@@ -2858,11 +2863,8 @@
                 let link = Text::one(look, small, &text, palette.accent)
                     .layout(ui.ctx())
                     .label_sense(ui, Sense::click());
-                let name = format!(
-                    "{} {}",
-                    gettext(locale, "Open"),
-                    format::display_safe(&target.name)
-                );
+                let name = gettext(locale, "Open {table}")
+                    .replace("{table}", &format::display_safe(&target.name));
                 link.widget_info(|| WidgetInfo::labeled(WidgetType::Link, true, &name));
                 if link.clicked() {
                     actions.push(Action::FollowForeignKey {
@@ -2995,11 +2997,15 @@
         }
         let more = array.items.len() - shown;
         if more > 0 {
-            let rest = format!(
-                "… {} {}",
-                format::group_digits(more as u64),
-                look.label(&gettext(locale, "more elements"))
-            );
+            let plural = u32::try_from(more).unwrap_or(u32::MAX);
+            let rest = look
+                .label(&ngettext(
+                    locale,
+                    "… {n} more elements",
+                    "… {n} more elements",
+                    plural,
+                ))
+                .replace("{n}", &format::group_digits(more as u64));
             Text::one(look, role, &rest, palette.faint)
                 .layout(ui.ctx())
                 .label(ui);
@@ -3116,7 +3122,7 @@
     // The large editor's own key for it: Omarchy's `=` is a SQL tab's.
     let scope = crate::keymap::Scope::CellEditor;
     let chord = crate::ui::keys::written_in(ui.ctx(), look, Command::Format, scope);
-    let label = format!("{} {chord}", gettext(locale, "Format"));
+    let label = gettext(locale, "Format {key}").replace("{key}", &chord);
     let role = TextRole::FieldLabel;
     let size = vec2(
         role.width(ui.ctx(), look.faces, &label),
@@ -3416,11 +3422,10 @@
     let measure = |text: &str| note_role.width(ui.ctx(), look.faces, text);
     // And, as the design's hint ends, the keys that copy and delete.
     let key = |command| crate::ui::keys::written(ui.ctx(), look, command);
-    let click = gettext(locale, "Click a value or press");
-    let edit = gettext(locale, "to edit");
+    let edit = gettext(locale, "Click a value or press {key} to edit")
+        .replace("{key}", &key(Command::OpenInspector));
     let said = format!(
-        "{click} {} {edit} · {} · {}",
-        key(Command::OpenInspector),
+        "{edit} · {} · {}",
         key(Command::DuplicateRow),
         key(Command::DeleteRow)
     );
```
```diff
--- a/src/ui/settings/mod.rs
+++ b/src/ui/settings/mod.rs
@@ -241,7 +241,7 @@
 pub(super) fn zone_text(locale: Locale, zone: &Zone, system: Option<&str>) -> String {
     match zone {
         Zone::System => match system {
-            Some(name) => format!("{} · {name}", gettext(locale, "System")),
+            Some(name) => gettext(locale, "System · {zone}").replace("{zone}", name),
             None => gettext(locale, "System").into_owned(),
         },
         Zone::Utc => "UTC".to_owned(),
```
```diff
--- a/src/ui/settings/sheet.rs
+++ b/src/ui/settings/sheet.rs
@@ -1955,11 +1955,13 @@
     let lines = u32::try_from(count).unwrap_or(u32::MAX);
     let said = ngettext(
         skin.locale,
-        "line in the file could not be read and was ignored",
-        "lines in the file could not be read and were ignored",
+        "{n} line in the file could not be read and was ignored: {lines}",
+        "{n} lines in the file could not be read and were ignored: {lines}",
         lines,
     );
-    let said = format!("{count} {said}: {}", line_numbers(skin.locale, ignored));
+    let said = said
+        .replace("{n}", &count.to_string())
+        .replace("{lines}", &line_numbers(skin.locale, ignored));
     let text = Text::one(skin.look, TextRole::Secondary, &said, skin.palette.warning);
     Some(text.wrap(width).layout(ui.ctx()))
 }
```
```diff
--- a/src/ui/settings/terminal.rs
+++ b/src/ui/settings/terminal.rs
@@ -1282,7 +1282,7 @@
         // As the artboard writes it: `system (america/new_york)`.
         OptionId::Tz => skin.look.label(&match (&settings.tz, skin.system) {
             (crate::settings::Zone::System, Some(name)) => {
-                format!("{} ({name})", gettext(skin.locale, "System"))
+                gettext(skin.locale, "System ({zone})").replace("{zone}", name)
             }
             (zone, system) => super::zone_text(skin.locale, zone, system),
         }),
```
```diff
--- a/src/ui/sql_editor.rs
+++ b/src/ui/sql_editor.rs
@@ -8,7 +8,7 @@
 };
 
 use crate::format;
-use crate::i18n::{Locale, gettext};
+use crate::i18n::{Locale, gettext, ngettext};
 use crate::keymap::{Command, Keymap, Scope, Written};
 use crate::model::{Action, ConnTabId, NoWrites, RunMode, SqlTab, TabId, Workspace};
 use crate::settings::Settings;
@@ -252,8 +252,12 @@
         (true, _, 0) => format!("{minutes}m"),
         (true, _, _) => format!("{minutes}m {rest}s"),
         (false, 0, _) => format!("{secs} s"),
-        (false, _, 0) => format!("{minutes} {}", gettext(locale, "min")),
-        (false, _, _) => format!("{minutes} {} {rest} s", gettext(locale, "min")),
+        (false, _, 0) => {
+            ngettext(locale, "{n} min", "{n} min", minutes).replace("{n}", &minutes.to_string())
+        }
+        (false, _, _) => gettext(locale, "{minutes} min {seconds} s")
+            .replace("{minutes}", &minutes.to_string())
+            .replace("{seconds}", &rest.to_string()),
     }
 }
 
@@ -361,7 +365,9 @@
         return Vec::new();
     };
     let (mode, writes) = (workspace.run_mode(sql), workspace.sql_writes());
-    let title = look.label(&format!("{} {}", gettext(locale, "Query"), sql.number));
+    let title = look
+        .label(&gettext(locale, "Query {n}"))
+        .replace("{n}", &sql.number.to_string());
     let limit = sql.limit;
     let secs = sql
         .timeout
@@ -1252,11 +1258,9 @@
         return;
     };
     let (line, column) = sql.line_col();
-    let cursor = format!(
-        "{} {line}, {} {column}",
-        gettext(locale, "Ln"),
-        gettext(locale, "Col")
-    );
+    let cursor = gettext(locale, "Ln {line}, Col {column}")
+        .replace("{line}", &line.to_string())
+        .replace("{column}", &column.to_string());
     let summary = run_summary(sql, locale);
     let note = end_note(sql, locale);
     let version = workspace.server_version.value.clone();
@@ -1348,7 +1352,11 @@
 pub fn status_summary(workspace: &Workspace, locale: Locale) -> Option<String> {
     let sql = workspace.active_sql_tab()?;
     let (line, column) = sql.line_col();
-    let mut parts = vec![format!("{} {line}:{column}", gettext(locale, "ln"))];
+    let mut parts = vec![
+        gettext(locale, "ln {line}:{column}")
+            .replace("{line}", &line.to_string())
+            .replace("{column}", &column.to_string()),
+    ];
     parts.extend(run_summary(sql, locale));
     if let Some(run) = sql.last_run().filter(|_| ran(sql)) {
         parts.push(gettext(locale, end_words(run)).into_owned());
```
```diff
--- a/src/ui/start.rs
+++ b/src/ui/start.rs
@@ -11,7 +11,7 @@
 
 use crate::env::{Platform, env_colors};
 use crate::format::display_safe;
-use crate::i18n::{Locale, gettext};
+use crate::i18n::{Locale, gettext, ngettext};
 use crate::keymap::{Command, Keymap, Layout, Scope, Written};
 use crate::model::{Action, ConnTabId, Found, Pane, StartRecent, Workspace};
 use crate::style::Icon;
@@ -710,7 +710,9 @@
         } else {
             rows.to_string()
         };
-        format!("≈ {count} {}", look.label(&gettext(locale, "rows")))
+        let plural = u32::try_from(rows).unwrap_or(u32::MAX);
+        look.label(&ngettext(locale, "≈ {n} rows", "≈ {n} rows", plural))
+            .replace("{n}", &count)
     }
 }
 
```
```diff
--- a/src/ui/workspace.rs
+++ b/src/ui/workspace.rs
@@ -6,7 +6,7 @@
 
 use crate::app::App;
 use crate::format::display_safe;
-use crate::i18n::gettext;
+use crate::i18n::{gettext, pgettext};
 use crate::keymap::Command;
 use crate::model::{Action, ConnTabId, ObjectView, SessionStatus, TabId};
 use crate::style::Icon;
@@ -1199,12 +1199,12 @@
 
 /// What a click on another connection's chip is announced as.
 fn switch_label(chip: &Chip, look: &Look, locale: crate::i18n::Locale) -> String {
-    format!(
-        "{} {} · {}",
-        gettext(locale, "Switch to"),
-        chip.name,
-        chip.env.label(crate::env::Platform::of(look))
-    )
+    gettext(locale, "Switch to {connection} · {environment}")
+        .replace(
+            "{environment}",
+            chip.env.label(crate::env::Platform::of(look)),
+        )
+        .replace("{connection}", &chip.name)
 }
 
 /// Announces the bar's own chip: the database switcher.
@@ -1491,7 +1491,7 @@
     if let Some(host) = &info.ssh_host {
         pills.push((
             None,
-            format!("{} {host}", gettext(locale, "via SSH")),
+            gettext(locale, "via SSH {host}").replace("{host}", host),
             palette.secondary,
         ));
     }
@@ -1761,7 +1761,7 @@
     }
     if let Some(host) = &info.ssh_host {
         tags.push((
-            format!("{} {host}", gettext(locale, "via SSH")),
+            gettext(locale, "via SSH {host}").replace("{host}", host),
             palette.dim,
         ));
     }
@@ -3259,7 +3259,7 @@
                 gettext(locale, "this connection"),
                 gettext(locale, "filter"),
                 gettext(locale, "details"),
-                gettext(locale, "open"),
+                pgettext(locale, "key hint", "open"),
                 gettext(locale, "export"),
             ];
             match agents_log {
```
```diff
--- a/src/ui/write_prompts.rs
+++ b/src/ui/write_prompts.rs
@@ -105,6 +105,61 @@
         },
         Held::CloseWindow => "closing the window",
         Held::Nothing => "discarding",
+    }
+}
+
+/// The leave prompt's title, Save's or Discard's, by what waits on the
+/// answer: "Save {changes} before closing the tab?". Each a whole phrase,
+/// so a translation can put what waits in the case its words ask for.
+fn before_title(held: &Held, save: bool, locale: Locale) -> std::borrow::Cow<'static, str> {
+    let either = |save_title, discard_title| if save { save_title } else { discard_title };
+    match held {
+        Held::Action(action) => match **action {
+            Action::CloseTab { .. } => either(
+                gettext(locale, "Save {changes} before closing the tab?"),
+                gettext(locale, "Discard {changes} before closing the tab?"),
+            ),
+            Action::Refresh(_) | Action::RetryRows { .. } | Action::RetryStructure { .. } => {
+                either(
+                    gettext(locale, "Save {changes} before reloading?"),
+                    gettext(locale, "Discard {changes} before reloading?"),
+                )
+            }
+            Action::NextPage { .. } | Action::PrevPage { .. } => either(
+                gettext(locale, "Save {changes} before changing the page?"),
+                gettext(locale, "Discard {changes} before changing the page?"),
+            ),
+            Action::SortBy { .. } | Action::ClearSort { .. } => either(
+                gettext(locale, "Save {changes} before sorting?"),
+                gettext(locale, "Discard {changes} before sorting?"),
+            ),
+            Action::ApplyFilters { .. }
+            | Action::ClearFilters { .. }
+            | Action::DropFilter { .. }
+            | Action::FollowForeignKey { .. }
+            | Action::OpenFiltered { .. } => either(
+                gettext(locale, "Save {changes} before filtering?"),
+                gettext(locale, "Discard {changes} before filtering?"),
+            ),
+            Action::SwitchDatabase { .. } => either(
+                gettext(locale, "Save {changes} before switching database?"),
+                gettext(locale, "Discard {changes} before switching database?"),
+            ),
+            Action::CloseConnTab(_) | Action::Disconnect(_) | Action::Connect { .. } => either(
+                gettext(locale, "Save {changes} before disconnecting?"),
+                gettext(locale, "Discard {changes} before disconnecting?"),
+            ),
+            _ => either(
+                gettext(locale, "Save {changes} before going on?"),
+                gettext(locale, "Discard {changes} before going on?"),
+            ),
+        },
+        Held::CloseWindow => either(
+            gettext(locale, "Save {changes} before closing the window?"),
+            gettext(locale, "Discard {changes} before closing the window?"),
+        ),
+        // Asked for no other step: the prompt only drops the changes.
+        Held::Nothing => gettext(locale, "Discard {changes}?"),
     }
 }
 
@@ -227,28 +282,21 @@
     let title = match (prompt.can_save, prompt.tabs.len()) {
         // The changes themselves were asked to be dropped: nothing waits
         // on the answer to be named.
-        _ if matches!(prompt.held, Held::Nothing) => {
-            format!("{} {changes}?", gettext(locale, "Discard"))
-        }
-        (true, _) => format!(
-            "{} {changes} {} {}?",
-            gettext(locale, "Save"),
-            gettext(locale, "before"),
-            gettext(locale, doing(&prompt.held, false))
-        ),
-        (false, 0 | 1) => format!(
-            "{} {changes} {} {}?",
-            gettext(locale, "Discard"),
-            gettext(locale, "before"),
-            gettext(locale, doing(&prompt.held, false))
-        ),
-        (false, tabs) => format!(
-            "{} {changes} {} {tabs} {}?",
-            gettext(locale, "Discard"),
-            gettext(locale, "in"),
-            gettext(locale, "tabs")
-        ),
+        _ if matches!(prompt.held, Held::Nothing) => gettext(locale, "Discard {changes}?"),
+        (true, _) => before_title(&prompt.held, true, locale),
+        (false, 0 | 1) => before_title(&prompt.held, false, locale),
+        (false, tabs) => {
+            let plural = u32::try_from(tabs).unwrap_or(u32::MAX);
+            let title = ngettext(
+                locale,
+                "Discard {changes} in {n} tab?",
+                "Discard {changes} in {n} tabs?",
+                plural,
+            );
+            title.replace("{n}", &tabs.to_string()).into()
+        }
     };
+    let title = title.replace("{changes}", &changes);
     let plural = u32::try_from(prompt.changes).unwrap_or(u32::MAX);
     let text = if prompt.saving {
         gettext(locale, SAVING)
@@ -538,7 +586,10 @@
     let what = match parts.as_slice() {
         [] => String::new(),
         [one] => one.clone(),
-        [first @ .., last] => format!("{} {} {last}", first.join(", "), gettext(locale, "and")),
+        [first @ .., last] => crate::i18n::fill(
+            &gettext(locale, "{list} and {last}"),
+            &[("{list}", &first.join(", ")), ("{last}", last)],
+        ),
     };
     let facts = Facts {
         connection: format!(
@@ -552,11 +603,10 @@
         rows: if prompt.rows == 0 && (prompt.added > 0 || prompt.deleted > 0) {
             table.clone()
         } else {
-            format!(
-                "{} {} {table}",
-                counted(locale, prompt.rows, "row", "rows"),
-                gettext(locale, "in")
-            )
+            let plural = u32::try_from(prompt.rows).unwrap_or(u32::MAX);
+            ngettext(locale, "{n} row in {table}", "{n} rows in {table}", plural)
+                .replace("{n}", &prompt.rows.to_string())
+                .replace("{table}", &table)
         },
         columns,
         tag: workspace.environment.label(crate::env::Platform::of(&look)),
@@ -714,12 +764,8 @@
         palette,
         locale,
     } = skin;
-    let title = format!(
-        "{} {} {}?",
-        gettext(locale, "Save"),
-        facts.changes,
-        gettext(locale, "to production")
-    );
+    let title =
+        gettext(locale, "Save {changes} to production?").replace("{changes}", &facts.changes);
     let about = format!("{} · {}", facts.connection, facts.rows);
     let (frame, radius) = skin.frame();
     let mut copy = false;
```

- [ ] **Step 3: Run the extraction again and the checks**

The xgettext command of Step 1 prints nothing. Every check passes, and no test changed.

- [ ] **Step 4: Commit**

```bash
git add -A crates src
git commit -m "Make each glued message one whole phrase"
```

---

### Task 2: The catalog tooling and the Russian catalog

`assets/i18n/update.sh` extracts the template from every `.rs` file under `src/` and `crates/` that git does not ignore, a new one not yet added among them. References name the file and not the line, so moving code changes nothing. `counted(locale, n, one, many)` is a keyword beside `gettext`, `ngettext` and `pgettext`, so its words are plurals of the catalog: that brings in `filter`, `new row` and `column`, which the views also say alone, so those lone uses take the context `label`. The script merges the template into each catalog without fuzzy matching, drops what the code no longer has, and checks each with `msgfmt`. `--check` changes nothing and fails on a stale template or catalog, or on any xgettext warning, and shows xgettext's own errors when it fails. CI runs it on macOS, because Ubuntu's gettext 0.21 can't read Rust.

`Locale::Russian` takes its catalog from the module `build.rs` compiles from `ru.po` (`ru::Translator`). `first_system_language` is the first of `fastframe_i18n::system_languages()`, and never a later one. The C locale (`C`, `POSIX`, `C.UTF-8`) names no language, so it gives `None` (`language_of`). `endonym` names about forty languages in themselves, for the line that says one is not translated yet; a Chinese tag's script decides before its region. The tests of task 1's `fill` come with this task's test module.

**Files:** Modify `.github/workflows/ci.yml`; Modify `AGENTS.md`; Create `assets/i18n/ru.po`; Create `assets/i18n/tabletist.pot`; Create `assets/i18n/update.sh`; Modify `crates/tabletist-core/src/i18n.rs`; Modify `src/ui/agents.rs`; Modify `src/ui/history.rs`; Modify `src/ui/row_panel.rs`; Modify `src/ui/sql_complete.rs`; Modify `src/ui/workspace.rs`; Delete `assets/i18n/.gitkeep`

- [ ] **Step 1: Write the failing tests**

```diff
--- a/crates/tabletist-core/src/i18n.rs
+++ b/crates/tabletist-core/src/i18n.rs
@@ -56,3 +56,147 @@
         }
     }
 }
+
+#[cfg(test)]
+mod tests {
+    use super::*;
+
+    fn tag(text: &str) -> LanguageTag {
+        LanguageTag::parse(text).expect("a tag")
+    }
+
+    #[test]
+    fn a_tag_names_a_shipped_language_by_its_language_alone() {
+        assert_eq!(Locale::from_tag(&tag("ru")), Some(Locale::Russian));
+        assert_eq!(Locale::from_tag(&tag("ru-RU")), Some(Locale::Russian));
+        assert_eq!(Locale::from_tag(&tag("ru_UA.UTF-8")), Some(Locale::Russian));
+        assert_eq!(Locale::from_tag(&tag("en_GB.UTF-8")), Some(Locale::English));
+        assert_eq!(Locale::from_tag(&tag("fr-FR")), None);
+        assert_eq!(Locale::from_tag(&tag("C")), None);
+    }
+
+    #[test]
+    fn russian_has_a_catalog_and_english_none() {
+        use fastframe_i18n::Locale as _;
+        assert!(Locale::English.catalog().is_none());
+        assert!(Locale::Russian.catalog().is_some());
+        assert_eq!(gettext(Locale::Russian, "Settings"), "Настройки");
+        assert_eq!(gettext(Locale::English, "Settings"), "Settings");
+    }
+
+    #[test]
+    fn russian_plurals_take_three_forms() {
+        let rows = |count| ngettext(Locale::Russian, "{n} row", "{n} rows", count);
+        assert_eq!(rows(1), "{n} строка");
+        assert_eq!(rows(3), "{n} строки");
+        assert_eq!(rows(5), "{n} строк");
+        assert_eq!(rows(21), "{n} строка");
+        assert_eq!(rows(11), "{n} строк");
+        assert_eq!(
+            ngettext(Locale::English, "{n} row", "{n} rows", 1),
+            "{n} row"
+        );
+    }
+
+    /// Each entry of a catalog's text: its msgid with its plural, and its
+    /// translations. Enough of the PO format for the catalogs here: a
+    /// field's strings on its line and the lines after it.
+    fn entries(text: &str) -> Vec<(Vec<String>, Vec<String>)> {
+        let mut entries = Vec::new();
+        let (mut ids, mut strs): (Vec<String>, Vec<String>) = (Vec::new(), Vec::new());
+        let mut field: Option<bool> = None;
+        let quoted = |line: &str| -> String {
+            let inner = line.trim();
+            let inner = inner
+                .strip_prefix('"')
+                .and_then(|rest| rest.strip_suffix('"'));
+            inner.unwrap_or_default().replace("\\\"", "\"")
+        };
+        for line in text.lines() {
+            let line = line.trim_end();
+            if line.starts_with("msgid") {
+                if field == Some(false) && line.starts_with("msgid ") {
+                    entries.push((std::mem::take(&mut ids), std::mem::take(&mut strs)));
+                }
+                field = Some(true);
+                let (_, rest) = line.split_once(' ').unwrap_or_default();
+                ids.push(quoted(rest));
+            } else if line.starts_with("msgstr") {
+                field = Some(false);
+                let (_, rest) = line.split_once(' ').unwrap_or_default();
+                strs.push(quoted(rest));
+            } else if line.starts_with('"') {
+                let target = if field == Some(true) {
+                    &mut ids
+                } else {
+                    &mut strs
+                };
+                if let Some(last) = target.last_mut() {
+                    last.push_str(&quoted(line));
+                }
+            }
+        }
+        entries.push((ids, strs));
+        entries
+    }
+
+    /// The `{…}` placeholders of `text`, sorted.
+    fn placeholders(text: &str) -> Vec<&str> {
+        let mut found: Vec<&str> = text
+            .match_indices('{')
+            .filter_map(|(start, _)| {
+                let end = text[start..].find('}')?;
+                Some(&text[start..=start + end])
+            })
+            .collect();
+        found.sort_unstable();
+        found
+    }
+
+    #[test]
+    fn every_russian_translation_keeps_its_placeholders() {
+        let text = include_str!("../../../assets/i18n/ru.po");
+        let entries = entries(text);
+        assert!(entries.len() > 600, "{} entries", entries.len());
+        for (ids, strs) in entries {
+            let Some(id) = ids.first().filter(|id| !id.is_empty()) else {
+                continue;
+            };
+            let wanted = placeholders(id);
+            for translated in strs.iter().filter(|text| !text.is_empty()) {
+                assert_eq!(placeholders(translated), wanted, "{id:?} → {translated:?}");
+            }
+        }
+    }
+
+    #[test]
+    fn the_c_locale_is_no_language() {
+        for none in ["C", "POSIX", "C.UTF-8", "c", ""] {
+            assert_eq!(language_of(none), None, "{none:?}");
+        }
+        assert_eq!(language_of("ru_RU.UTF-8"), Some(tag("ru-RU")));
+    }
+
+    #[test]
+    fn placeholders_are_filled_in_one_pass() {
+        let filled = fill(
+            "also clears {columns} in {table} rows",
+            &[("{table}", "{columns}"), ("{columns}", "a")],
+        );
+        assert_eq!(filled, "also clears a in {columns} rows");
+        assert_eq!(fill("{n} of {total} {", &[("{n}", "2")]), "2 of {total} {");
+    }
+
+    #[test]
+    fn a_language_is_named_in_itself() {
+        assert_eq!(endonym(&tag("fr_FR.UTF-8")), Some("Français"));
+        assert_eq!(endonym(&tag("de")), Some("Deutsch"));
+        assert_eq!(endonym(&tag("pt-BR")), Some("Português (Brasil)"));
+        assert_eq!(endonym(&tag("zh-Hant-TW")), Some("繁體中文"));
+        assert_eq!(endonym(&tag("zh-CN")), Some("简体中文"));
+        assert_eq!(endonym(&tag("zh-Hans-TW")), Some("简体中文"));
+        assert_eq!(endonym(&tag("zh-CHT")), Some("繁體中文"));
+        assert_eq!(endonym(&tag("zh-CHS")), Some("简体中文"));
+        assert_eq!(endonym(&tag("xx")), None);
+    }
+}
```

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test -p tabletist-core --lib i18n::`
Expected: it doesn't compile: `no variant or associated item named Russian found for enum Locale`, and `LanguageTag`, `Locale::from_tag`, `language_of` and `endonym` are not found.

- [ ] **Step 3: Write the code, the script, the job and the rule**

```diff
--- a/.github/workflows/ci.yml
+++ b/.github/workflows/ci.yml
@@ -42,6 +42,19 @@
       - run: cargo doc --locked --workspace --no-deps
         env:
           RUSTDOCFLAGS: -D warnings
+
+  # The translation template and every catalog are in step with the code.
+  # Ubuntu's gettext is older than 0.24, the first to read Rust; Homebrew's
+  # is newer.
+  translations:
+    runs-on: macos-latest
+    timeout-minutes: 10
+    steps:
+      - uses: actions/checkout@v4
+      - run: |
+          brew install gettext
+          echo "$(brew --prefix gettext)/bin" >> "$GITHUB_PATH"
+      - run: assets/i18n/update.sh --check
 
   test:
     name: test (${{ matrix.os }})
```
```diff
--- a/AGENTS.md
+++ b/AGENTS.md
@@ -98,6 +98,14 @@
 (CI's `mariadb` job). A test that expects what only one of them does asks the server's version.
 The native keyring test is `#[ignore]`d; run it by hand on a desktop session.
 
+A change that adds, changes or takes out a user-facing string runs
+`assets/i18n/update.sh` (GNU gettext 0.24 or later), which writes the
+template and merges it into every catalog; CI's `translations` job runs it
+with `--check`. Each message is a whole phrase with named placeholders
+(`{n}`, `{table}`), and a count goes through `ngettext`: never a sentence
+glued from translated pieces. A string `gettext` is handed in a variable is
+not extracted. A new message shows in English until its catalog has it.
+
 Views draw text only through `TextRole`s (`src/typography.rs`); a view never
 names a font, a family or a size.
 
```
```diff
--- /dev/null
+++ b/assets/i18n/update.sh
@@ -0,0 +1,92 @@
+#!/usr/bin/env bash
+# Extracts Tabletist's translatable messages into assets/i18n/tabletist.pot,
+# merges the template into every catalog (assets/i18n/*.po), and checks each
+# catalog with msgfmt.
+#
+# Run from anywhere in the repository after changing a user-facing string.
+# Needs GNU gettext 0.24 or later, which reads Rust (Homebrew's gettext,
+# Arch's, Fedora 42's). Ordinary Cargo builds need no gettext tools.
+#
+#   assets/i18n/update.sh           update the template and the catalogs
+#   assets/i18n/update.sh --check   change nothing; fail if either is stale
+#
+# References name the file and not the line, so moving code changes
+# nothing here: only a message added, changed or taken out does.
+set -euo pipefail
+
+mode=update
+case ${1:-} in
+    '') ;;
+    --check) mode=check ;;
+    *) echo "usage: $0 [--check]" >&2; exit 2 ;;
+esac
+
+root=$(git rev-parse --show-toplevel)
+cd "$root"
+dir=assets/i18n
+template=$dir/tabletist.pot
+
+version=$(xgettext --version | sed -n '1s/.* //p')
+major=${version%%.*}
+minor=${version#*.}; minor=${minor%%.*}
+if (( major == 0 && minor < 24 )); then
+    echo "xgettext $version cannot read Rust; gettext 0.24 or later is needed" >&2
+    exit 2
+fi
+
+scratch=$(mktemp -d)
+trap 'rm -rf "$scratch"' EXIT
+
+# Every Rust file but the ignored ones, a new one not yet added among them.
+git ls-files --cached --others --exclude-standard 'src/*.rs' 'crates/*.rs' \
+    | LC_ALL=C sort -u > "$scratch/files"
+# `counted(locale, n, "row", "rows")` (crate::i18n) is a plural too.
+xgettext --language=Rust --from-code=UTF-8 \
+    --keyword= --keyword=gettext:2 --keyword=ngettext:2,3 --keyword=pgettext:2c,3 \
+    --keyword=counted:3,4 \
+    --add-comments=Translators: --add-location=file --sort-by-file \
+    --flag=ngettext:2:rust-format --flag=ngettext:3:rust-format \
+    --package-name=Tabletist --copyright-holder="Tabletist contributors" \
+    --msgid-bugs-address="https://github.com/igor-alexandrov/tabletist/issues" \
+    --files-from="$scratch/files" --output="$scratch/tabletist.pot" 2> "$scratch/warnings" \
+    || { cat "$scratch/warnings" >&2; exit 1; }
+# A message used both alone and as a plural, among others, is a warning
+# xgettext does not fail on: here it fails.
+if [[ -s $scratch/warnings ]]; then
+    cat "$scratch/warnings" >&2
+    exit 1
+fi
+
+# The extraction and merge dates are the only headers that change on
+# every run.
+undated() { sed -e '/^"POT-Creation-Date:/d' -e '/^"PO-Revision-Date:/d' "$1"; }
+
+stale=0
+if [[ $mode == check ]]; then
+    if ! diff -u <(undated "$template") <(undated "$scratch/tabletist.pot"); then
+        echo "$template is out of date: run $dir/update.sh" >&2
+        stale=1
+    fi
+else
+    cp "$scratch/tabletist.pot" "$template"
+fi
+
+for catalog in "$dir"/*.po; do
+    [[ -e $catalog ]] || continue
+    merged=$scratch/$(basename "$catalog")
+    # A message the code no longer has is dropped, not kept as obsolete.
+    msgmerge --quiet --no-fuzzy-matching --add-location=file --sort-by-file \
+        --output-file="$merged.all" "$catalog" "$scratch/tabletist.pot"
+    msgattrib --no-obsolete --add-location=file --sort-by-file \
+        --output-file="$merged" "$merged.all"
+    if [[ $mode == check ]]; then
+        if ! diff -u <(undated "$catalog") <(undated "$merged"); then
+            echo "$catalog is out of date: run $dir/update.sh" >&2
+            stale=1
+        fi
+    else
+        cp "$merged" "$catalog"
+    fi
+    msgfmt --check --statistics --output-file=/dev/null "$catalog"
+done
+exit "$stale"
```
```diff
--- a/crates/tabletist-core/src/i18n.rs
+++ b/crates/tabletist-core/src/i18n.rs
@@ -1,8 +1,9 @@
-//! Bundled gettext catalogs. English is the source language and, for now, the
-//! only one. Every user-facing string goes through `gettext` so translations
-//! can be added later without touching the views.
-
-pub use fastframe_i18n::{gettext, ngettext, pgettext};
+//! Bundled gettext catalogs. English is the source language; every other
+//! language the app ships has a catalog in `assets/i18n` (`ru.po`), which
+//! the build compiles. Every user-facing string goes through `gettext`, and
+//! a string a catalog lacks shows in English.
+
+pub use fastframe_i18n::{LanguageTag, gettext, ngettext, pgettext};
 
 include!(concat!(env!("OUT_DIR"), "/catalogs.rs"));
 
@@ -43,18 +44,126 @@
     out
 }
 
-#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
+/// A language the interface is drawn in.
+#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
 pub enum Locale {
     #[default]
     English,
+    Russian,
+}
+
+impl Locale {
+    /// Every language the app ships, in the order the language menu lists
+    /// them.
+    pub const ALL: [Locale; 2] = [Self::English, Self::Russian];
+
+    /// The tag the settings file writes: `en`, `ru`.
+    pub fn tag(self) -> &'static str {
+        match self {
+            Self::English => "en",
+            Self::Russian => "ru",
+        }
+    }
+
+    /// The language's name in itself, as the menu lists it: never
+    /// translated, so a person finds theirs whatever the window shows.
+    pub fn native_name(self) -> &'static str {
+        match self {
+            Self::English => "English",
+            Self::Russian => "Русский",
+        }
+    }
+
+    /// The language `tag` names, where the app ships it: its language
+    /// subtag alone decides (`ru-RU`, `ru_UA.UTF-8` are Russian).
+    pub fn from_tag(tag: &LanguageTag) -> Option<Self> {
+        Self::ALL
+            .into_iter()
+            .find(|locale| locale.tag() == tag.language)
+    }
 }
 
 impl fastframe_i18n::Locale for Locale {
     fn catalog(self) -> Option<&'static dyn fastframe_i18n::Translator> {
         match self {
             Self::English => None,
-        }
-    }
+            Self::Russian => Some(&ru::Translator),
+        }
+    }
+}
+
+/// The system's first language, as the system lists them for the app:
+/// the one "System" follows. Only the first counts: a later one never
+/// stands in for it. `None` when the system lists none, or its first is
+/// no language: one that does not parse, or the C locale (`C`, `POSIX`,
+/// `C.UTF-8`), which sets none.
+pub fn first_system_language() -> Option<LanguageTag> {
+    fastframe_i18n::system_languages()
+        .first()
+        .and_then(|tag| language_of(tag))
+}
+
+/// The language `tag` names, as [`first_system_language`] reads it: `None`
+/// for the C locale and for what does not parse.
+pub fn language_of(tag: &str) -> Option<LanguageTag> {
+    LanguageTag::parse(tag).filter(|tag| !matches!(tag.language.as_str(), "c" | "posix"))
+}
+
+/// A language's name in itself, for the line that says it is not
+/// translated yet: `Français`, `Deutsch`. `None` for a language not in the
+/// list, which the line names by its tag.
+pub fn endonym(tag: &LanguageTag) -> Option<&'static str> {
+    let name = match (tag.language.as_str(), tag.region.as_deref()) {
+        ("pt", Some("br")) => "Português (Brasil)",
+        // The script says it where it is written; else the region does,
+        // Windows' legacy `zh-CHT` and `zh-CHS` among them.
+        ("zh", _) if tag.script.as_deref() == Some("hant") => "繁體中文",
+        ("zh", _) if tag.script.as_deref() == Some("hans") => "简体中文",
+        ("zh", Some("tw" | "hk" | "mo" | "cht")) => "繁體中文",
+        ("zh", _) => "简体中文",
+        ("ar", _) => "العربية",
+        ("bg", _) => "Български",
+        ("ca", _) => "Català",
+        ("cs", _) => "Čeština",
+        ("da", _) => "Dansk",
+        ("de", _) => "Deutsch",
+        ("el", _) => "Ελληνικά",
+        ("en", _) => "English",
+        ("es", _) => "Español",
+        ("et", _) => "Eesti",
+        ("fa", _) => "فارسی",
+        ("fi", _) => "Suomi",
+        ("fr", _) => "Français",
+        ("he", _) => "עברית",
+        ("hi", _) => "हिन्दी",
+        ("hr", _) => "Hrvatski",
+        ("hu", _) => "Magyar",
+        ("id", _) => "Bahasa Indonesia",
+        ("it", _) => "Italiano",
+        ("ja", _) => "日本語",
+        ("ka", _) => "ქართული",
+        ("kk", _) => "Қазақ тілі",
+        ("ko", _) => "한국어",
+        ("lt", _) => "Lietuvių",
+        ("lv", _) => "Latviešu",
+        ("ms", _) => "Bahasa Melayu",
+        ("nb" | "no" | "nn", _) => "Norsk",
+        ("nl", _) => "Nederlands",
+        ("pl", _) => "Polski",
+        ("pt", _) => "Português",
+        ("ro", _) => "Română",
+        ("ru", _) => "Русский",
+        ("sk", _) => "Slovenčina",
+        ("sl", _) => "Slovenščina",
+        ("sr", _) => "Српски",
+        ("sv", _) => "Svenska",
+        ("th", _) => "ไทย",
+        ("tr", _) => "Türkçe",
+        ("uk", _) => "Українська",
+        ("vi", _) => "Tiếng Việt",
+        _ => return None,
+    };
+    Some(name)
 }
 
 #[cfg(test)]
```
```diff
--- a/src/ui/agents.rs
+++ b/src/ui/agents.rs
@@ -7,7 +7,7 @@
 use egui::{Frame, Id, Margin, Rect, pos2, vec2};
 
 use crate::connections::{SavedConnection, SavedConnections};
-use crate::i18n::{gettext, ngettext};
+use crate::i18n::{gettext, ngettext, pgettext};
 use crate::keymap::Keymap;
 use crate::model::{Action, AgentsHere, AgentsView, ConnTabId, Workspace};
 use crate::style::Icon;
@@ -1145,7 +1145,7 @@
             .add(body, keys.filter.as_str(), palette.accent)
             .add(
                 body,
-                &format!(" {}", look.label(&gettext(locale, "filter"))),
+                &format!(" {}", look.label(&pgettext(locale, "label", "filter"))),
                 palette.secondary,
             );
         if log.total > 0 {
```
```diff
--- a/src/ui/history.rs
+++ b/src/ui/history.rs
@@ -8,7 +8,7 @@
 use egui::{Frame, Id, Rect, pos2, vec2};
 
 use crate::history::{Entry, Outcome, Source};
-use crate::i18n::{Locale, gettext, ngettext};
+use crate::i18n::{Locale, gettext, ngettext, pgettext};
 use crate::keymap::Keymap;
 use crate::model::{Action, ConnTabId, HistoryFilterSet, HistoryRow, HistoryTab, Workspace};
 use crate::style::Icon;
@@ -481,7 +481,7 @@
                     .id(search_id(self.tab)),
             );
             ui.ctx().accesskit_node_builder(field.id, |node| {
-                node.set_label(look.label(&gettext(locale, "filter")).as_str());
+                node.set_label(look.label(&pgettext(locale, "label", "filter")).as_str());
             });
             let seen_id = Id::new(("history-line-seen", self.tab.0));
             let seen: Option<u64> = ui.data(|data| data.get_temp(seen_id));
```
```diff
--- a/src/ui/row_panel.rs
+++ b/src/ui/row_panel.rs
@@ -655,7 +655,10 @@
                 let role = TextRole::OBody;
                 let mut x = header.left() + side;
                 let (word, ink) = match fresh {
-                    Some(_) => (format!("+ {}", gettext(locale, "new row")), green.bar),
+                    Some(_) => (
+                        format!("+ {}", pgettext(locale, "label", "new row")),
+                        green.bar,
+                    ),
                     None => (
                         pgettext(locale, "row panel title", "row").into_owned(),
                         palette.text,
```
```diff
--- a/src/ui/sql_complete.rs
+++ b/src/ui/sql_complete.rs
@@ -12,7 +12,7 @@
 };
 
 use crate::completion::{Candidate, Kind};
-use crate::i18n::{Locale, gettext};
+use crate::i18n::{Locale, gettext, pgettext};
 use crate::model::Completion;
 use crate::theme::{Look, Palette};
 use crate::typography::{Text, TextRole};
@@ -178,7 +178,7 @@
         Kind::Table => gettext(locale, "table"),
         Kind::View => gettext(locale, "view"),
         Kind::MaterializedView => gettext(locale, "materialized view"),
-        Kind::Column => gettext(locale, "column"),
+        Kind::Column => pgettext(locale, "label", "column"),
     }
 }
 
```
```diff
--- a/src/ui/workspace.rs
+++ b/src/ui/workspace.rs
@@ -3075,7 +3075,7 @@
                 table_hints.push((&edit_key, &*edit, true));
             }
             // A new row is opened, where the table takes one.
-            let new_row = gettext(locale, "new row");
+            let new_row = pgettext(locale, "label", "new row");
             if editing.can_add {
                 table_hints.push((&add_key, &*new_row, true));
             }
@@ -3257,7 +3257,7 @@
                 gettext(locale, "calls"),
                 gettext(locale, "all connections"),
                 gettext(locale, "this connection"),
-                gettext(locale, "filter"),
+                pgettext(locale, "label", "filter"),
                 gettext(locale, "details"),
                 pgettext(locale, "key hint", "open"),
                 gettext(locale, "export"),
```

Make the script executable, and remove the placeholder that kept the empty folder:

```bash
chmod +x assets/i18n/update.sh
git rm assets/i18n/.gitkeep
```

- [ ] **Step 4: Make the template and the catalog**

```bash
assets/i18n/update.sh
cp docs/superpowers/plans/2026-11-18-translations-1-russian.ru.po assets/i18n/ru.po
assets/i18n/update.sh --check
```

`update.sh` writes `assets/i18n/tabletist.pot` (711 messages, 74 of them plural). The Russian catalog drafted for this plan is beside it, as `2026-11-18-translations-1-russian.ru.po`, already merged with that template. The check prints `711 translated messages.` and exits 0. Its header carries `Language: ru` and Russian's three plural forms:

```po
"Plural-Forms: nplurals=3; plural=(n%10==1 && n%100!=11 ? 0 : n%10>=2 && "
"n%10<=4 && (n%100<10 || n%100>=20) ? 1 : 2);\n"
```

- [ ] **Step 5: Run the tests and the checks**

Run: `cargo test -p tabletist-core --lib i18n::`
Expected: seven passed: `a_tag_names_a_shipped_language_by_its_language_alone`, `russian_has_a_catalog_and_english_none`, `russian_plurals_take_three_forms`, `every_russian_translation_keeps_its_placeholders`, `the_c_locale_is_no_language`, `placeholders_are_filled_in_one_pass` and `a_language_is_named_in_itself`. Then every check passes.

- [ ] **Step 6: Commit**

```bash
git add -A assets/i18n crates/tabletist-core/src/i18n.rs .github/workflows/ci.yml AGENTS.md src/ui
git commit -m "Add the Russian catalog and the script that keeps catalogs in step"
```

---

### Task 3: The language in settings.toml, followed live

`Language { System, Fixed(Locale) }` is `interface.language`: `system`, `en` or `ru`, in any case, and `ru-RU` reads as `ru`. A language with no catalog (`fr`) is a line that cannot be read, kept and shown red, as an unknown zone is. `resolve` gives the window's locale from the system's first language. `choices` and `shown` are the menu's entries and the one it marks. `App::system_language` is read once at start (`None` in tests), and `apply_settings`, which every change from the app or the file goes through, sets `App::locale` again. The menu bar's `Keyed::rekey` now also answers a change of language, so `Bar::sync` describes the bar again in the new words. A rebuild muda refuses is asked once.

**Files:** Modify `crates/tabletist-core/src/settings.rs`; Modify `src/app.rs`; Modify `src/menu/bar.rs`; Modify `src/menu/tests.rs`

- [ ] **Step 1: Write the failing tests**

```diff
--- a/crates/tabletist-core/src/settings.rs
+++ b/crates/tabletist-core/src/settings.rs
@@ -3366,6 +3366,87 @@
         assert!(!loaded.settings.confirm_close_edits);
     }
 
+    fn tag(text: &str) -> LanguageTag {
+        LanguageTag::parse(text).expect("a tag")
+    }
+
+    #[test]
+    fn the_language_is_read_by_its_tag_and_anything_else_is_a_wrong_line() {
+        let read = |value: &str| {
+            let text = format!("[interface]\nlanguage = \"{value}\"\n");
+            Settings::from_toml(&text)
+        };
+        assert_eq!(read("system").settings.language, Language::System);
+        assert_eq!(read("SYSTEM").settings.language, Language::System);
+        assert_eq!(
+            read("en").settings.language,
+            Language::Fixed(Locale::English)
+        );
+        assert_eq!(
+            read("ru").settings.language,
+            Language::Fixed(Locale::Russian)
+        );
+        assert_eq!(
+            read("RU").settings.language,
+            Language::Fixed(Locale::Russian)
+        );
+        assert_eq!(
+            read("ru-RU").settings.language,
+            Language::Fixed(Locale::Russian)
+        );
+        // A language the app has no catalog for is no value, and the last
+        // good one stays.
+        let last = Settings {
+            language: Language::Fixed(Locale::Russian),
+            ..Settings::default()
+        };
+        for wrong in ["fr", "xx", ""] {
+            let text = format!("[interface]\nlanguage = \"{wrong}\"\n");
+            let loaded = Settings::from_toml_over(&text, &last);
+            assert_eq!(loaded.invalid, [2], "{wrong:?}");
+            assert_eq!(loaded.settings.language, Language::Fixed(Locale::Russian));
+        }
+        // Written as its tag, and the default not at all.
+        let russian = Settings {
+            language: Language::Fixed(Locale::Russian),
+            ..Settings::default()
+        };
+        assert!(russian.to_toml().contains("[interface]\nlanguage = \"ru\""));
+        assert!(!Settings::default().to_toml().contains("language"));
+    }
+
+    #[test]
+    fn system_follows_the_first_language_only_and_else_english() {
+        let system = Language::System;
+        assert_eq!(system.resolve(Some(&tag("ru-RU"))), Locale::Russian);
+        assert_eq!(system.resolve(Some(&tag("en_GB.UTF-8"))), Locale::English);
+        assert_eq!(system.resolve(Some(&tag("fr-FR"))), Locale::English);
+        assert_eq!(system.resolve(None), Locale::English);
+        let fixed = Language::Fixed(Locale::Russian);
+        assert_eq!(fixed.resolve(Some(&tag("fr-FR"))), Locale::Russian);
+        assert_eq!(fixed.resolve(None), Locale::Russian);
+    }
+
+    #[test]
+    fn the_menu_lists_system_only_where_its_language_has_a_catalog() {
+        let english = Language::Fixed(Locale::English);
+        let russian = Language::Fixed(Locale::Russian);
+        assert_eq!(
+            Language::choices(Some(&tag("ru-RU"))),
+            [Language::System, english, russian]
+        );
+        assert_eq!(
+            Language::choices(Some(&tag("en-US"))),
+            [Language::System, english, russian]
+        );
+        assert_eq!(Language::choices(Some(&tag("fr-FR"))), [english, russian]);
+        assert_eq!(Language::choices(None), [english, russian]);
+        // Where System is no entry, it is shown as English.
+        assert_eq!(Language::System.shown(Some(&tag("fr-FR"))), english);
+        assert_eq!(Language::System.shown(Some(&tag("ru"))), Language::System);
+        assert_eq!(russian.shown(Some(&tag("fr-FR"))), russian);
+    }
+
     fn chosen(id: &str, name: &str) -> Launch {
         Launch::Connection {
             id: crate::connections::ConnectionId(id.into()),
@@ -3555,6 +3636,7 @@
     /// Every key away from its default.
     fn all_set() -> Settings {
         Settings {
+            language: Language::Fixed(Locale::Russian),
             on_launch: Launch::Connection {
                 id: crate::connections::ConnectionId("6a0c1f0e-5a52-4b7b-9c56-3f0c8f2f7a11".into()),
                 name: "Bookshop".into(),
@@ -3620,6 +3702,9 @@
             format!(
                 "\
 # written by tabletist, safe to edit by hand
+[interface]
+language = \"ru\"  # system | \"en\" | \"ru\"
+
 [startup]
 on_launch = \"conn:6a0c1f0e-5a52-4b7b-9c56-3f0c8f2f7a11\"  # Bookshop
 restore_tabs = false
@@ -3748,48 +3833,49 @@
         assert_eq!(
             loaded.lines,
             vec![
-                (Key::OnLaunch, 3),
-                (Key::RestoreTabs, 4),
-                (Key::PageSize, 7),
-                (Key::Timestamps, 8),
-                (Key::Tz, 9),
-                (Key::GroupDigits, 10),
-                (Key::ValueTags, 11),
-                (Key::DatePicker, 12),
-                (Key::ProductionReadOnly, 15),
-                (Key::ConfirmCloseEdits, 16),
-                (Key::IdleDisconnect, 17),
-                (Key::ShowSystemSchemas, 20),
-                (Key::NewTab, 23),
-                (Key::SqlLimit, 24),
-                (Key::SqlTimeoutSecs, 25),
-                (Key::RunScope, 26),
-                (Key::ConfirmUnsafeWrites, 27),
-                (Key::EditorKeys, 30),
-                (Key::Indent, 31),
-                (Key::Autocomplete, 32),
-                (Key::ClosePairs, 33),
-                (Key::LineNumbers, 34),
-                (Key::HighlightLine, 35),
-                (Key::Wrap, 36),
-                (Key::Keywords, 39),
-                (Key::Commas, 40),
-                (Key::HistoryKeep, 43),
-                (Key::McpEnabled, 46),
-                (Key::McpLogDays, 47),
-                (Key::McpNotify, 48),
-                (Key::Look, 51),
-                (Key::Theme, 52),
-                (Key::UiScale, 53),
-                (Key::Font, 54),
-                (Key::FontSize, 55),
-                (Key::RowHeight, 58),
-                (Key::Zebra, 59),
-                (Key::ColumnTypes, 60),
-                (Key::TableList, 61),
-                (Key::EnvBar, 64),
-                (Key::UpdatesCheck, 67),
-                (Key::UpdatesSkip, 68),
+                (Key::Language, 3),
+                (Key::OnLaunch, 6),
+                (Key::RestoreTabs, 7),
+                (Key::PageSize, 10),
+                (Key::Timestamps, 11),
+                (Key::Tz, 12),
+                (Key::GroupDigits, 13),
+                (Key::ValueTags, 14),
+                (Key::DatePicker, 15),
+                (Key::ProductionReadOnly, 18),
+                (Key::ConfirmCloseEdits, 19),
+                (Key::IdleDisconnect, 20),
+                (Key::ShowSystemSchemas, 23),
+                (Key::NewTab, 26),
+                (Key::SqlLimit, 27),
+                (Key::SqlTimeoutSecs, 28),
+                (Key::RunScope, 29),
+                (Key::ConfirmUnsafeWrites, 30),
+                (Key::EditorKeys, 33),
+                (Key::Indent, 34),
+                (Key::Autocomplete, 35),
+                (Key::ClosePairs, 36),
+                (Key::LineNumbers, 37),
+                (Key::HighlightLine, 38),
+                (Key::Wrap, 39),
+                (Key::Keywords, 42),
+                (Key::Commas, 43),
+                (Key::HistoryKeep, 46),
+                (Key::McpEnabled, 49),
+                (Key::McpLogDays, 50),
+                (Key::McpNotify, 51),
+                (Key::Look, 54),
+                (Key::Theme, 55),
+                (Key::UiScale, 56),
+                (Key::Font, 57),
+                (Key::FontSize, 58),
+                (Key::RowHeight, 61),
+                (Key::Zebra, 62),
+                (Key::ColumnTypes, 63),
+                (Key::TableList, 64),
+                (Key::EnvBar, 67),
+                (Key::UpdatesCheck, 70),
+                (Key::UpdatesSkip, 71),
             ]
         );
     }
```
```diff
--- a/src/menu/tests.rs
+++ b/src/menu/tests.rs
@@ -1074,6 +1074,30 @@
 }
 
 #[test]
+fn the_bar_is_built_again_in_a_language_chosen_in_settings() {
+    use crate::i18n::Locale;
+    use crate::settings::Language;
+    let mut harness = Harness::new();
+    let Described { items, menus, .. } = describe(Platform::MacOs, &harness.app);
+    let mut keyed = Keyed::new(&items, &harness.app);
+    assert!(menus.iter().any(|menu| menu.title == "View"));
+    harness
+        .app
+        .change_settings(|settings| settings.language = Language::Fixed(Locale::Russian));
+    assert_eq!(harness.app.locale, Locale::Russian);
+    // The keys are as they were, but the words are not: built again.
+    let keys = keyed
+        .rekey(&items, &harness.app)
+        .expect("the language changed");
+    assert_eq!(keys, keyed.keys);
+    // Asked once.
+    assert_eq!(keyed.rekey(&items, &harness.app), None);
+    let Described { menus, .. } = describe(Platform::MacOs, &harness.app);
+    let titles: Vec<&str> = menus.iter().map(|menu| menu.title.as_str()).collect();
+    assert!(titles.contains(&"Вид"), "{titles:?}");
+}
+
+#[test]
 fn run_all_is_enabled_as_each_edit_leaves_the_script() {
     let (mut harness, tab) = sql_editor();
     let run_all = Item::Command(Command::RunAll);
```

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test -p tabletist-core --lib settings::` and `cargo test -p tabletist --lib menu::`
Expected: neither compiles: `cannot find type Language`, `no variant Key::Language`, `Settings` has no field `language`, and `App` none named `system_language`.

- [ ] **Step 3: Write the code**

```diff
--- a/crates/tabletist-core/src/settings.rs
+++ b/crates/tabletist-core/src/settings.rs
@@ -4,6 +4,7 @@
 
 use serde::Deserialize;
 
+use crate::i18n::{LanguageTag, Locale};
 use crate::keymap::Layout;
 use crate::paths::AppDirs;
 use crate::util::toml_string;
@@ -38,6 +39,77 @@
         [Self::Second, Self::Full]
             .into_iter()
             .find(|choice| choice.name() == name)
+    }
+}
+
+/// The language the interface is drawn in.
+#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
+pub enum Language {
+    /// The system's first language where the app has a catalog for it,
+    /// else English.
+    #[default]
+    System,
+    /// One the person chose.
+    Fixed(Locale),
+}
+
+impl Language {
+    /// As the file says it: `system`, `en`, `ru`.
+    pub fn name(self) -> &'static str {
+        match self {
+            Self::System => "system",
+            Self::Fixed(locale) => locale.tag(),
+        }
+    }
+
+    /// The language the file names, where the app ships it: `system` in
+    /// any case, or a tag whose language has a catalog (`ru`, `RU`,
+    /// `ru-RU`). Anything else is no value: the line is shown as wrong and
+    /// left.
+    pub fn from_name(name: &str) -> Option<Self> {
+        if name.eq_ignore_ascii_case("system") {
+            return Some(Self::System);
+        }
+        LanguageTag::parse(name)
+            .as_ref()
+            .and_then(Locale::from_tag)
+            .map(Self::Fixed)
+    }
+
+    /// The language the window is drawn in, where `system` is the system's
+    /// first language: only that one counts, and with no catalog for it
+    /// the window is English.
+    pub fn resolve(self, system: Option<&LanguageTag>) -> Locale {
+        match self {
+            Self::Fixed(locale) => locale,
+            Self::System => system.and_then(Locale::from_tag).unwrap_or_default(),
+        }
+    }
+
+    /// Whether the system's first language has a catalog, so that System
+    /// is an entry of its own.
+    pub fn system_listed(system: Option<&LanguageTag>) -> bool {
+        system.and_then(Locale::from_tag).is_some()
+    }
+
+    /// The language menu's entries, in order: System where the system's
+    /// first language has a catalog, then each language the app ships.
+    pub fn choices(system: Option<&LanguageTag>) -> Vec<Self> {
+        let system = Self::system_listed(system).then_some(Self::System);
+        system
+            .into_iter()
+            .chain(Locale::ALL.into_iter().map(Self::Fixed))
+            .collect()
+    }
+
+    /// The entry the menu marks: this one, but System as English where
+    /// System is no entry (the system's language has no catalog). The
+    /// file keeps `system`, so the app follows a catalog that ships later.
+    pub fn shown(self, system: Option<&LanguageTag>) -> Self {
+        match self {
+            Self::System if !Self::system_listed(system) => Self::Fixed(Locale::English),
+            other => other,
+        }
     }
 }
 
@@ -703,6 +775,10 @@
 #[derive(Clone, Debug, PartialEq, Deserialize)]
 #[serde(default)]
 pub struct Settings {
+    /// The language the interface is drawn in. Not in an older version's
+    /// `settings.json`, which had none.
+    #[serde(skip)]
+    pub language: Language,
     /// What a launch opens: the windows open at the last quit, the
     /// saved connections, or one of them. Not in an older version's
     /// `settings.json`, which had none.
@@ -849,6 +925,7 @@
 /// A key of the settings file.
 #[derive(Clone, Copy, Debug, PartialEq, Eq)]
 pub enum Key {
+    Language,
     OnLaunch,
     RestoreTabs,
     PageSize,
@@ -906,7 +983,8 @@
     /// In the order the file has them: a table's keys together.
     /// An old key comes before the one that took its place: the new one
     /// is read over it.
-    pub const ALL: [Key; 45] = [
+    pub const ALL: [Key; 46] = [
+        Self::Language,
         Self::OnLaunch,
         Self::RestoreTabs,
         Self::PageSize,
@@ -957,6 +1035,7 @@
     /// The key's table and its name there.
     pub fn path(self) -> (&'static str, &'static str) {
         match self {
+            Self::Language => ("interface", "language"),
             Self::OnLaunch => ("startup", "on_launch"),
             Self::RestoreTabs => ("startup", "restore_tabs"),
             Self::PageSize => ("data", "page_size"),
@@ -1020,6 +1099,7 @@
     /// value: see [`Settings::note`].
     fn note(self) -> Option<&'static str> {
         match self {
+            Self::Language => Some("system | \"en\" | \"ru\""),
             Self::OnLaunch => Some("restore | connections | \"conn:<id>\""),
             Self::Timestamps => Some("second | full"),
             Self::Tz => Some("system | utc | \"Area/City\""),
@@ -2619,6 +2699,7 @@
     /// out: a theme that is not set, and stripes left to the look.
     fn written(&self, key: Key) -> Option<String> {
         Some(match key {
+            Key::Language => toml_string(self.language.name()),
             Key::OnLaunch => toml_string(&self.on_launch.name()),
             Key::RestoreTabs => self.restore_tabs.to_string(),
             Key::PageSize => self.page_size.to_string(),
@@ -2939,6 +3020,9 @@
             float.parse::<f64>().ok()
         });
         match key {
+            Key::Language => text
+                .and_then(Language::from_name)
+                .map(|language| self.language = language),
             Key::OnLaunch => text
                 .and_then(Launch::from_name)
                 .map(|launch| self.on_launch = launch),
@@ -3169,6 +3253,7 @@
     /// Takes `key`'s value from `other`.
     fn keep(&mut self, key: Key, other: &Settings) {
         match key {
+            Key::Language => self.language = other.language,
             Key::OnLaunch => self.on_launch = other.on_launch.clone(),
             Key::RestoreTabs => self.restore_tabs = other.restore_tabs,
             Key::PageSize => self.page_size = other.page_size,
@@ -3258,6 +3343,7 @@
 impl Default for Settings {
     fn default() -> Self {
         Self {
+            language: Language::System,
             on_launch: Launch::Restore,
             restore_tabs: true,
             page_size: Self::DEFAULT_PAGE_SIZE,
```
```diff
--- a/src/app.rs
+++ b/src/app.rs
@@ -135,6 +135,11 @@
     /// The last named zone the settings had this session: the Omarchy
     /// screen's steps reach it after a step off it.
     pub named_zone: Option<String>,
+    /// The system's first language as the app found it at its start:
+    /// what the System language of the settings follows.
+    pub system_language: Option<crate::i18n::LanguageTag>,
+    /// The language the window is drawn in: the settings' language,
+    /// resolved against [`App::system_language`].
     pub locale: Locale,
     pub palette: Palette,
     pub look: crate::theme::Look,
@@ -448,6 +453,8 @@
         // An installed data font is read once the window exists (`attach`).
         let look = crate::theme::Look::chosen(settings.look, crate::update::release::Os::current())
             .with_text(&settings, false);
+        let system_language = system_language();
+        let locale = settings.language.resolve(system_language.as_ref());
         let mut app = Self {
             dirs,
             settings,
@@ -458,7 +465,8 @@
             stopped_clock: None,
             named_zone,
             zones: None,
-            locale: Locale::default(),
+            system_language,
+            locale,
             palette: Palette::dark(),
             look,
             keymap: std::sync::Arc::default(),
@@ -6051,6 +6059,12 @@
     pub fn apply_settings(&mut self, new: Settings) {
         let was_on = self.agents_on();
         let old = std::mem::replace(&mut self.settings, new);
+        // A language of its own or the system's: the window is drawn in it
+        // from the next frame, and the menu bar is described again then.
+        self.locale = self
+            .settings
+            .language
+            .resolve(self.system_language.as_ref());
         // The keys: the text they were read from is the file's now.
         self.refresh_keymap();
         let terminal = self.look.terminal;
@@ -6613,6 +6627,16 @@
         fastframe_i18n::system_languages(),
     );
     crate::dates::first_weekday(region.as_deref())
+}
+
+/// The system's first language ([`crate::i18n::first_system_language`]).
+/// None in a test, whatever the machine's, so what tests show is English:
+/// a test of the System language sets `App::system_language`.
+fn system_language() -> Option<crate::i18n::LanguageTag> {
+    if cfg!(test) {
+        return None;
+    }
+    crate::i18n::first_system_language()
 }
 
 /// The computer's time zone, or UTC where it cannot be found. Tests have
```
```diff
--- a/src/menu/bar.rs
+++ b/src/menu/bar.rs
@@ -1,7 +1,7 @@
 //! The menu bar attached to the window, kept in step with the app: each
 //! frame it takes what was chosen, and after each frame it shows what is
 //! enabled and, when the keymap or the look changes, is built again with
-//! the new keys.
+//! the new keys, and when the language changes, with the new words.
 //!
 //! What it decides is in plain functions and types (`describe`, `route`,
 //! `changes`, `Keyed`, `Feed`), tested headlessly in `tests.rs`; [`Bar`]
@@ -13,7 +13,7 @@
 use tabletist_menubar::{Entry, MenuBar, Shortcut, Via};
 
 use crate::app::{App, ItemState};
-use crate::i18n::gettext;
+use crate::i18n::{Locale, gettext};
 use crate::keymap::{EditItem, Keymap, Layout, MenuKey};
 
 use super::{Handling, Item, Platform, Replay, bar, handling, key_of, title};
@@ -159,11 +159,13 @@
     changes
 }
 
-/// The keys the bar was built with, and the keymap and look they came
-/// from: what decides when the bar is built again.
+/// The keys the bar was built with, the keymap and look they came from,
+/// and the language its words are in: what decides when the bar is built
+/// again.
 pub(super) struct Keyed {
     keymap: Arc<Keymap>,
     layout: Layout,
+    locale: Locale,
     /// The key each item was built with.
     pub keys: HashMap<Id, Option<MenuKey>>,
 }
@@ -174,29 +176,33 @@
         Self {
             keymap: app.keymap.clone(),
             layout: app.layout(),
+            locale: app.locale,
             keys: keys_of(items, app),
         }
     }
 
     /// The keys to build the bar again with, where `app`'s keymap or look
-    /// changed and with it an item's key; None where the bar's still
-    /// hold. Every settings change makes a new keymap (`refresh_keymap`),
-    /// so the pointer only says the keys may have changed. The keymap and
-    /// look are recorded either way, so a rebuild muda refuses is asked
-    /// once; the keys are the caller's to keep once the bar is built.
+    /// changed and with it an item's key, or where its language changed;
+    /// None where the bar's keys and words still hold. Every settings
+    /// change makes a new keymap (`refresh_keymap`), so the pointer only
+    /// says the keys may have changed. The keymap, look and language are
+    /// recorded either way, so a rebuild muda refuses is asked once; the
+    /// keys are the caller's to keep once the bar is built.
     pub fn rekey(
         &mut self,
         items: &HashMap<Id, Item>,
         app: &App,
     ) -> Option<HashMap<Id, Option<MenuKey>>> {
         let layout = app.layout();
-        if Arc::ptr_eq(&self.keymap, &app.keymap) && layout == self.layout {
+        let reworded = app.locale != self.locale;
+        if !reworded && Arc::ptr_eq(&self.keymap, &app.keymap) && layout == self.layout {
             return None;
         }
         self.keymap = app.keymap.clone();
         self.layout = layout;
+        self.locale = app.locale;
         let keys = keys_of(items, app);
-        (keys != self.keys).then_some(keys)
+        (reworded || keys != self.keys).then_some(keys)
     }
 }
 
```

- [ ] **Step 4: Run the tests and the checks**

Run: `cargo test -p tabletist-core --lib settings::` and `cargo test -p tabletist --lib menu::`
Expected: they pass, among them `the_language_is_read_by_its_tag_and_anything_else_is_a_wrong_line`, `system_follows_the_first_language_only_and_else_english`, `the_menu_lists_system_only_where_its_language_has_a_catalog`, `each_key_is_found_on_its_line` and `the_bar_is_built_again_in_a_language_chosen_in_settings`. `assets/i18n/update.sh --check` still prints `711 translated messages.`: this task adds no message. Then every check passes.

- [ ] **Step 5: Commit**

```bash
git add crates/tabletist-core/src/settings.rs src/app.rs src/menu
git commit -m "Read the interface's language from settings.toml and follow it live"
```

---

### Task 4: The Language row in Settings › General

`OptionId::Language` is General's first option, in a section of its own.

**Classic sheet (macOS, Windows):**
- **Dropdown:** `language_menu`, like `launch_menu`. System stands apart from the languages, and each language is written in its own name, with its name in the window's language as the entry's detail.
- **Link:** "Help translate" opens `https://igor.works/tabletist/translating/`.
- **Label:** in any language but English, the English word follows in grey: «Язык · Language».
- **Small print:** it says "Français is not translated yet" where System stands for English. It is left out of the label column's width, as History's is, so the Numbers row keeps its hint.

**Omarchy screen:**
- **Value:** the row steps with h/l through `Settings::stepped_language` and stops at either end. A step that stays leaves System as it is.
- **Hint:** "from $LANG · help translate: assets/i18n", with `$LANG` and the folder put in after the look lower-cases.

**Model:** `SettingsPane::reset_options` leaves General's first option out.

**Strings:** the row's messages are said in full, because `label` and `small_print` hand theirs to `gettext` in a variable, which xgettext does not read. Seven are in `settings::language_words`, `language_text`, `language_aside` and `language_print`. The Omarchy hint's two, `from {variable}` and `help translate: {folder}`, are in `terminal.rs`.

**Files:** Modify `assets/i18n/ru.po`; Modify `assets/i18n/tabletist.pot`; Modify `crates/tabletist-core/src/model.rs`; Modify `crates/tabletist-core/src/settings.rs`; Modify `docs/_guide/omarchy.md`; Modify `docs/_reference/settings-and-files.md`; Create `docs/_reference/translating.md`; Create `src/ui/language_tests.rs`; Modify `src/ui/mod.rs`; Modify `src/ui/settings/mod.rs`; Modify `src/ui/settings/sheet.rs`; Modify `src/ui/settings/terminal.rs`; Modify `src/ui/settings_access_tests.rs`

- [ ] **Step 1: Write the failing tests**

The row's tests are the new `src/ui/language_tests.rs` in the diff below. Existing tests that open on the first row, or count General's rows, move down one row: `settings_screen_in` and `settings_cursor` count the Language section with Startup, and the access test lists the row's hint. The core's step and sections tests come with them:

```diff
--- a/crates/tabletist-core/src/settings.rs
+++ b/crates/tabletist-core/src/settings.rs
@@ -3533,6 +3533,55 @@
         assert_eq!(russian.shown(Some(&tag("fr-FR"))), russian);
     }
 
+    #[test]
+    fn a_step_moves_through_the_menu_and_stops_at_its_ends() {
+        let at = |language| Settings {
+            language,
+            ..Settings::default()
+        };
+        let english = Language::Fixed(Locale::English);
+        let russian = Language::Fixed(Locale::Russian);
+        let step = |language, system: Option<&LanguageTag>, forward| {
+            at(language).stepped_language(system, forward)
+        };
+        let ru = tag("ru-RU");
+        assert_eq!(
+            step(Language::System, Some(&ru), true),
+            OptionValue::Language(english)
+        );
+        assert_eq!(
+            step(english, Some(&ru), true),
+            OptionValue::Language(russian)
+        );
+        assert_eq!(
+            step(russian, Some(&ru), true),
+            OptionValue::Language(russian)
+        );
+        assert_eq!(
+            step(english, Some(&ru), false),
+            OptionValue::Language(Language::System)
+        );
+        assert_eq!(
+            step(Language::System, Some(&ru), false),
+            OptionValue::Language(Language::System)
+        );
+        // Shown as English, System steps on to Russian, and back stays
+        // System rather than writing English in its place.
+        let fr = tag("fr-FR");
+        assert_eq!(
+            step(Language::System, Some(&fr), true),
+            OptionValue::Language(russian)
+        );
+        assert_eq!(
+            step(Language::System, Some(&fr), false),
+            OptionValue::Language(Language::System)
+        );
+        assert_eq!(
+            step(russian, Some(&fr), false),
+            OptionValue::Language(english)
+        );
+    }
+
     fn chosen(id: &str, name: &str) -> Launch {
         Launch::Connection {
             id: crate::connections::ConnectionId(id.into()),
@@ -5048,6 +5097,7 @@
         assert_eq!(
             keys,
             vec![
+                Key::Language,
                 Key::OnLaunch,
                 Key::RestoreTabs,
                 Key::PageSize,
@@ -5102,6 +5152,7 @@
         // The sections of the General and Appearance panes hold their
         // options, in order.
         let sections = [
+            &OptionId::LANGUAGE[..],
             &OptionId::STARTUP[..],
             &OptionId::DATA[..],
             &OptionId::SAFETY[..],
```
```diff
--- /dev/null
+++ b/src/ui/language_tests.rs
@@ -0,0 +1,201 @@
+//! The interface's language (Settings › General, Language), in both looks:
+//! what the row says for the system's first language, what choosing one
+//! does to the window and the file, and what Reset to defaults leaves.
+//!
+//! Only what is shown, said and written is checked. No colour, size or
+//! picture of the design is.
+
+use egui::{Key, Modifiers};
+
+use crate::i18n::{LanguageTag, Locale};
+use crate::model::{Action, SettingsPane, SettingsRow};
+use crate::settings::{Language, OptionId, OptionValue};
+use crate::testing::Harness;
+use crate::theme::Look;
+
+fn tag(text: &str) -> LanguageTag {
+    LanguageTag::parse(text).expect("a tag")
+}
+
+/// Settings open on General in `look`, the system's first language
+/// `system`, in a window tall enough for every row.
+fn general(look: Look, system: Option<&str>) -> Harness {
+    let mut harness = Harness::with_size(egui::vec2(1920.0, 2400.0));
+    harness.set_look(look);
+    harness.app.system_language = system.map(tag);
+    // As the app would at its start: the window in the System language.
+    let settings = harness.app.settings.clone();
+    harness.app.apply_settings(settings);
+    harness.app.apply(Action::ShowSettings);
+    harness
+        .app
+        .apply(Action::ShowSettingsPane(SettingsPane::General));
+    harness.settle();
+    harness
+}
+
+fn painted(harness: &Harness, text: &str) -> bool {
+    harness.painted_color(text).is_some()
+}
+
+#[test]
+fn the_language_is_general_s_first_row_in_both_looks() {
+    assert_eq!(
+        SettingsPane::General.rows().first(),
+        Some(&SettingsRow::Option(OptionId::Language))
+    );
+    assert_eq!(
+        SettingsPane::General.sheet_options().first(),
+        Some(&OptionId::Language)
+    );
+}
+
+#[test]
+fn system_says_the_language_it_stands_for() {
+    let harness = general(Look::macos(), Some("en-US"));
+    assert!(painted(&harness, "System (English)"));
+    assert!(painted(
+        &harness,
+        "Menus, labels and messages; data and SQL stay as they are"
+    ));
+    assert!(painted(&harness, "Help translate"));
+    // In English the label has no English word beside it.
+    assert!(painted(&harness, "Language"));
+    assert!(!painted(&harness, "Language · Language"));
+    // A Russian system: the window is Russian, and System says so.
+    let harness = general(Look::macos(), Some("ru-RU"));
+    assert_eq!(harness.app.locale, Locale::Russian);
+    assert!(painted(&harness, "Системный (Русский)"));
+}
+
+#[test]
+fn a_system_language_with_no_catalog_reads_english_and_says_so() {
+    let harness = general(Look::macos(), Some("fr_FR.UTF-8"));
+    assert_eq!(harness.app.locale, Locale::English);
+    assert_eq!(harness.app.settings.language, Language::System);
+    assert!(painted(&harness, "English"));
+    assert!(!painted(&harness, "System (English)"));
+    assert!(painted(&harness, "Français is not translated yet"));
+    // A language with no name in the list is named by its tag.
+    let harness = general(Look::macos(), Some("xx-YY"));
+    assert!(painted(&harness, "xx is not translated yet"));
+}
+
+#[test]
+fn choosing_russian_redraws_the_window_and_writes_the_file() {
+    let mut harness = general(Look::macos(), Some("en-US"));
+    harness
+        .app
+        .apply(Action::SetOption(OptionValue::Language(Language::Fixed(
+            Locale::Russian,
+        ))));
+    harness.settle();
+    assert_eq!(harness.app.locale, Locale::Russian);
+    // The sheet's title, from the catalog. (The panes' names are handed to
+    // `gettext` in a variable, which no extractor reads yet.)
+    assert!(painted(&harness, "Настройки"));
+    // The English word stays beside the label: the way back.
+    assert!(painted(&harness, "Язык · Language"));
+    assert!(painted(&harness, "Русский"));
+    assert!(
+        harness
+            .app
+            .settings_file
+            .text
+            .contains("[interface]\nlanguage = \"ru\"")
+    );
+    // And back to English.
+    harness
+        .app
+        .apply(Action::SetOption(OptionValue::Language(Language::Fixed(
+            Locale::English,
+        ))));
+    harness.settle();
+    assert_eq!(harness.app.locale, Locale::English);
+    assert!(painted(&harness, "Settings"));
+}
+
+#[test]
+fn reset_to_defaults_leaves_the_language() {
+    let mut harness = general(Look::macos(), Some("en-US"));
+    harness
+        .app
+        .apply(Action::SetOption(OptionValue::Language(Language::Fixed(
+            Locale::Russian,
+        ))));
+    harness
+        .app
+        .apply(Action::SetOption(OptionValue::PageSize(500)));
+    harness.app.apply(Action::ResetSettings);
+    harness.app.apply(Action::ConfirmResetSettings(true));
+    harness.settle();
+    assert_eq!(harness.app.settings.page_size, 1000);
+    assert_eq!(
+        harness.app.settings.language,
+        Language::Fixed(Locale::Russian)
+    );
+    assert_eq!(harness.app.locale, Locale::Russian);
+}
+
+/// The Omarchy screen's cursor on the Language row: the first.
+fn omarchy(system: Option<&str>) -> Harness {
+    general(Look::omarchy(), system)
+}
+
+#[test]
+fn the_omarchy_row_steps_through_the_languages_and_r_puts_back_system() {
+    let mut harness = omarchy(Some("en-US"));
+    // On the cursor's row, between its chevrons.
+    assert!(painted(&harness, "‹ system (english) ›"));
+    assert!(painted(
+        &harness,
+        "from $LANG · help translate: assets/i18n"
+    ));
+    harness.press(Key::L, Modifiers::NONE);
+    assert_eq!(
+        harness.app.settings.language,
+        Language::Fixed(Locale::English)
+    );
+    harness.press(Key::L, Modifiers::NONE);
+    assert_eq!(
+        harness.app.settings.language,
+        Language::Fixed(Locale::Russian)
+    );
+    assert_eq!(harness.app.locale, Locale::Russian);
+    assert!(painted(&harness, "‹ русский ›"));
+    // The last: it stays.
+    harness.press(Key::L, Modifiers::NONE);
+    assert_eq!(
+        harness.app.settings.language,
+        Language::Fixed(Locale::Russian)
+    );
+    harness.press(Key::H, Modifiers::NONE);
+    assert_eq!(
+        harness.app.settings.language,
+        Language::Fixed(Locale::English)
+    );
+    harness.press(Key::R, Modifiers::SHIFT);
+    assert_eq!(harness.app.settings.language, Language::System);
+}
+
+#[test]
+fn the_omarchy_row_says_a_system_language_is_not_translated() {
+    let mut harness = omarchy(Some("fr_FR.UTF-8"));
+    assert!(painted(&harness, "‹ english ›"));
+    assert!(painted(
+        &harness,
+        "français is not translated yet · help translate: assets/i18n"
+    ));
+    // Shown as English, System steps on to Russian; back from there is
+    // English, written.
+    harness.press(Key::L, Modifiers::NONE);
+    assert_eq!(
+        harness.app.settings.language,
+        Language::Fixed(Locale::Russian)
+    );
+    harness.press(Key::H, Modifiers::NONE);
+    assert_eq!(
+        harness.app.settings.language,
+        Language::Fixed(Locale::English)
+    );
+}
```
```diff
--- a/src/ui/mod.rs
+++ b/src/ui/mod.rs
@@ -105,6 +105,8 @@
 #[cfg(test)]
 mod label_tests;
 #[cfg(test)]
+mod language_tests;
+#[cfg(test)]
 mod look_switch_tests;
 pub mod menu;
 pub mod object_tabs;
@@ -1372,12 +1374,14 @@
     }
 
     /// The cursor on the first Data row, Rows per page: the rows of the
-    /// Startup group are over it, and have tests of their own.
+    /// Language and Startup groups are over it, and have tests of their own.
     fn settings_screen_in(mut harness: Harness) -> Harness {
         harness.set_look(crate::theme::Look::omarchy());
         harness.press(egui::Key::Comma, egui::Modifiers::COMMAND);
         assert!(settings_open(&harness));
-        for _ in 0..crate::settings::OptionId::STARTUP.len() {
+        for _ in
+            0..crate::settings::OptionId::LANGUAGE.len() + crate::settings::OptionId::STARTUP.len()
+        {
             harness.press(egui::Key::J, egui::Modifiers::NONE);
         }
         harness
@@ -1415,7 +1419,10 @@
     /// Data group: the tests of the rows over it read [`settings_row`].
     fn settings_cursor(harness: &Harness) -> usize {
         settings_row(harness)
-            .checked_sub(crate::settings::OptionId::STARTUP.len())
+            .checked_sub(
+                crate::settings::OptionId::LANGUAGE.len()
+                    + crate::settings::OptionId::STARTUP.len(),
+            )
             .expect("the cursor is under the Startup group")
     }
 
@@ -1616,6 +1623,7 @@
             for text in [
                 "Settings",
                 "General",
+                "Language",
                 "How Tabletist starts, shows data and protects your work. Changes apply right away.",
                 "Rows per page",
                 "Table view; the SQL editor has its own limit",
@@ -1770,7 +1778,9 @@
         ] {
             assert!(harness.painted_color(text).is_some(), "{text}");
         }
-        // The cursor opens on the first.
+        // The cursor opens on Language, the first row: one down is the first
+        // of the group.
+        harness.press(egui::Key::J, egui::Modifiers::NONE);
         assert!(harness.painted_color("▌on launch").is_some());
         assert!(harness.painted_color("‹ restore last session ›").is_some());
         harness.press(egui::Key::L, egui::Modifiers::NONE);
@@ -1895,7 +1905,9 @@
             })
             .expect("the row");
         // From Rows per page, where the screen's tests start.
-        for _ in crate::settings::OptionId::STARTUP.len()..row {
+        for _ in crate::settings::OptionId::LANGUAGE.len()
+            + crate::settings::OptionId::STARTUP.len()..row
+        {
             harness.press(egui::Key::J, egui::Modifiers::NONE);
         }
         assert!(
@@ -2119,7 +2131,7 @@
             // the first of them.
             let stored = harness.painted_rect("Stored in").expect("the footer");
             assert!(stored.bottom() < size.y, "{stored:?}");
-            let row = harness.painted_rect("On launch").expect("a row");
+            let row = harness.painted_rect("Language").expect("a row");
             assert!(row.bottom() < stored.top(), "{row:?} {stored:?}");
             harness.click("Export…");
             assert_eq!(harness.app.backend.saves.len(), 1);
@@ -2742,7 +2754,7 @@
         let mut harness = Harness::with_size(egui::vec2(1000.0, 700.0));
         harness.set_look(crate::theme::Look::omarchy());
         harness.press(egui::Key::Comma, egui::Modifiers::COMMAND);
-        assert!(harness.painted_color("▌on launch").is_some());
+        assert!(harness.painted_color("▌language").is_some());
         assert!(harness.painted_color("[data]").is_none());
     }
 
```
```diff
--- a/src/ui/settings_access_tests.rs
+++ b/src/ui/settings_access_tests.rs
@@ -282,6 +282,10 @@
         SettingsPane::General,
         &[
             (
+                OptionId::Language,
+                Some("from $LANG · help translate: assets/i18n"),
+            ),
+            (
                 OptionId::OnLaunch,
                 Some("or connections · a chosen connection"),
             ),
```

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test -p tabletist --lib -- ui::language_tests ui::tests ui::settings_access_tests` and `cargo test -p tabletist-core --lib settings::`
Expected: neither compiles: `no variant OptionId::Language`, `no variant OptionValue::Language`, `no associated item OptionId::LANGUAGE` and `no method stepped_language`.

- [ ] **Step 3: Write the code and the docs**

```diff
--- a/crates/tabletist-core/src/model.rs
+++ b/crates/tabletist-core/src/model.rs
@@ -3778,6 +3778,7 @@
         use crate::settings::OptionId;
         match self {
             Self::General => &[
+                ("Language", &OptionId::LANGUAGE),
                 ("Startup", &OptionId::STARTUP),
                 ("Data", &OptionId::DATA),
                 ("Safety", &OptionId::SAFETY),
@@ -3832,13 +3833,18 @@
 
     /// What Reset to defaults puts back: the pane's options but the
     /// server's switch, which turned off would end every agent's session
-    /// for a question about defaults, and but the look (Appearance's first),
-    /// which would change the whole window under the person. The Editor pane's are all of its
+    /// for a question about defaults, and but the language (General's
+    /// first) and the look (Appearance's first), each of which would change
+    /// the whole window under the person. The Editor pane's are all of its
     /// keys, the ones it has no row for yet among them.
     pub fn reset_options(self) -> &'static [crate::settings::OptionId] {
         use crate::settings::OptionId;
         match self {
-            Self::General => &OptionId::GENERAL,
+            // But the language (General's first), which would change the
+            // whole window's words under the person, as the look would.
+            Self::General => OptionId::GENERAL
+                .split_first()
+                .map_or(&[], |(_, rest)| rest),
             // But the look: a question about defaults would change the
             // whole window under the person.
             Self::Appearance => OptionId::APPEARANCE
@@ -3857,6 +3863,7 @@
         use crate::settings::OptionId;
         match self {
             Self::General => &[
+                SettingsRow::Option(OptionId::Language),
                 SettingsRow::Option(OptionId::OnLaunch),
                 SettingsRow::Option(OptionId::RestoreTabs),
                 SettingsRow::Option(OptionId::PageSize),
```
```diff
--- a/crates/tabletist-core/src/settings.rs
+++ b/crates/tabletist-core/src/settings.rs
@@ -1157,6 +1157,8 @@
 /// An option the Settings window shows: one row of it.
 #[derive(Clone, Copy, Debug, PartialEq, Eq)]
 pub enum OptionId {
+    /// The interface's language: `interface.language`.
+    Language,
     /// What a launch opens: `startup.on_launch`.
     OnLaunch,
     /// Whether a connection reopens its tables: `startup.restore_tabs`.
@@ -1228,7 +1230,8 @@
 impl OptionId {
     /// Every option, the General pane's, the Appearance pane's, the Editor
     /// pane's, the Agents pane's and then the Updates pane's.
-    pub const ALL: [OptionId; 40] = [
+    pub const ALL: [OptionId; 41] = [
+        Self::Language,
         Self::OnLaunch,
         Self::RestoreTabs,
         Self::PageSize,
@@ -1271,7 +1274,8 @@
         Self::UpdatesCheck,
     ];
     /// The General pane's, in the order it has them.
-    pub const GENERAL: [OptionId; 11] = [
+    pub const GENERAL: [OptionId; 12] = [
+        Self::Language,
         Self::OnLaunch,
         Self::RestoreTabs,
         Self::PageSize,
@@ -1284,7 +1288,9 @@
         Self::ConfirmCloseEdits,
         Self::IdleDisconnect,
     ];
-    /// The General pane's sections, in order: Startup, Data and Safety.
+    /// The General pane's sections, in order: Language, Startup, Data and
+    /// Safety.
+    pub const LANGUAGE: [OptionId; 1] = [Self::Language];
     pub const STARTUP: [OptionId; 2] = [Self::OnLaunch, Self::RestoreTabs];
     pub const DATA: [OptionId; 6] = [
         Self::PageSize,
@@ -1391,6 +1397,7 @@
     /// The key of the file the option is stored under.
     pub fn key(self) -> Key {
         match self {
+            Self::Language => Key::Language,
             Self::OnLaunch => Key::OnLaunch,
             Self::RestoreTabs => Key::RestoreTabs,
             Self::PageSize => Key::PageSize,
@@ -1443,6 +1450,7 @@
 /// An option with a value: what a key, a click or a reset sets.
 #[derive(Clone, Debug, PartialEq, Eq)]
 pub enum OptionValue {
+    Language(Language),
     OnLaunch(Launch),
     RestoreTabs(bool),
     PageSize(u32),
@@ -1530,6 +1538,7 @@
 
     pub fn option(&self) -> OptionId {
         match self {
+            Self::Language(_) => OptionId::Language,
             Self::OnLaunch(_) => OptionId::OnLaunch,
             Self::RestoreTabs(_) => OptionId::RestoreTabs,
             Self::PageSize(_) => OptionId::PageSize,
@@ -1576,6 +1585,7 @@
     /// Puts the value in `settings`.
     pub fn set(&self, settings: &mut Settings) {
         match self.clone() {
+            Self::Language(language) => settings.language = language,
             Self::OnLaunch(launch) => settings.on_launch = launch,
             Self::RestoreTabs(on) => settings.restore_tabs = on,
             Self::PageSize(size) => settings.page_size = size,
@@ -2269,6 +2279,7 @@
     /// What `option` is set to.
     pub fn value(&self, option: OptionId) -> OptionValue {
         match option {
+            OptionId::Language => OptionValue::Language(self.language),
             OptionId::OnLaunch => OptionValue::OnLaunch(self.on_launch.clone()),
             OptionId::RestoreTabs => OptionValue::RestoreTabs(self.restore_tabs),
             OptionId::PageSize => OptionValue::PageSize(self.page_size),
@@ -2356,6 +2367,9 @@
             // System, UTC, and the zone the file names, where it names
             // one: any other is set in the file.
             OptionId::Tz => self.stepped_zone(None, forward),
+            // Without the system's language System is no entry: English
+            // and the languages after it.
+            OptionId::Language => self.stepped_language(None, forward),
             OptionId::Timestamps => OptionValue::Timestamps(if forward {
                 Timestamps::Full
             } else {
@@ -2520,6 +2534,26 @@
         }
     }
 
+    /// The language one step from this one among the menu's entries
+    /// ([`Language::choices`]), where `system` is the system's first
+    /// language. It stops at either end.
+    pub fn stepped_language(&self, system: Option<&LanguageTag>, forward: bool) -> OptionValue {
+        let choices = Language::choices(system);
+        let shown = self.language.shown(system);
+        let at = choices
+            .iter()
+            .position(|choice| *choice == shown)
+            .unwrap_or(0);
+        let to = if forward {
+            (at + 1).min(choices.len() - 1)
+        } else {
+            at.saturating_sub(1)
+        };
+        // A step that stays leaves System as it is, not English in its place.
+        let language = if to == at { self.language } else { choices[to] };
+        OptionValue::Language(language)
+    }
+
     /// The zone one step from this one: System, UTC, then a named zone,
     /// the one set or else `named` (the last the session had), so that a
     /// step off it can come back. Any other zone is set in the file.
@@ -2613,9 +2647,11 @@
     /// The other value of an option that has two; `None` for one with more.
     pub fn flipped(&self, option: OptionId) -> Option<OptionValue> {
         match option {
-            OptionId::PageSize | OptionId::Tz | OptionId::IdleDisconnect | OptionId::OnLaunch => {
-                None
-            }
+            OptionId::PageSize
+            | OptionId::Tz
+            | OptionId::IdleDisconnect
+            | OptionId::OnLaunch
+            | OptionId::Language => None,
             OptionId::Timestamps => Some(OptionValue::Timestamps(match self.timestamps {
                 Timestamps::Second => Timestamps::Full,
                 Timestamps::Full => Timestamps::Second,
```
```diff
--- a/docs/_guide/omarchy.md
+++ b/docs/_guide/omarchy.md
@@ -390,7 +390,7 @@
 | Keys | Do |
 | --- | --- |
 | `j` `k` | Move |
-| `h` `l` | Change the option |
+| `h` `l` | Change the option. On the language row: System, English, then Русский |
 | Space | Toggle the option |
 | Ctrl+H, Ctrl+L | The pane before or after: General, Appearance, Editor, Keyboard, Agents, Updates |
 | Enter | On the Agents pane: show the Agents tab, Connections or the log, from the status, access or log row. On the Editor pane's keep row: show the History tab |
```
````diff
--- a/docs/_reference/settings-and-files.md
+++ b/docs/_reference/settings-and-files.md
@@ -12,6 +12,7 @@
 
 | Option | Choices | Default |
 | --- | --- | --- |
+| **Language** | System, English or Русский. Menus, labels and messages change at once; data, SQL, keys and `:` commands stay as they are. System follows the first language of your system (macOS's Language & Region, `$LANG` and its kin on Linux, Windows's display language), where Tabletist has a translation of it; for any other language Tabletist is in English, the menu shows English, and the line under the label says the language is not translated yet. **Help translate** opens [Translating](../translating/). **Reset to defaults** leaves the language as it is | System |
 | **On launch** | Restore last session, Show Connections, or a saved connection, each listed with its server and database. Restore opens again the connection windows open when Tabletist last quit, in their order, each connecting and asking for what it needs; with nothing kept, it shows Connections. If the chosen connection was deleted outside Tabletist, Tabletist shows Connections, says so once, and sets this back to Show Connections; deleting it in Tabletist sets it back at once. `--demo` starts as it always does | Restore last session |
 | **Restore tabs and filters** | On or off. On, a connection that opens, at launch or from Connections, opens again the table and view tabs its window had when it last closed on that database (or SQLite file), each with its sort and filters; the one in front loads, and each other when you first show it. SQL editors are not opened again. Off, a connection opens with its tables in the sidebar and no tab | On |
 | **Rows per page** | 100, 500, 1,000, 5,000 or 10,000. A table takes a new size when it next loads, not while you page through it | 1,000 |
@@ -210,6 +211,9 @@
 they follow the look):
 
 ```toml
+[interface]
+language = "system"
+
 [startup]
 on_launch = "restore"
 restore_tabs = true
@@ -271,6 +275,7 @@
 
 | Key | Values | Default |
 | --- | --- | --- |
+| `interface.language` | `"system"`, `"en"` or `"ru"`, in any case. `"system"` follows the system's first language where Tabletist has a translation of it, and is English otherwise; a later language in the system's list is never used in its place. A language Tabletist has no translation of is a line that cannot be read | `"system"` |
 | `startup.on_launch` | `"restore"`, `"connections"` or `"conn:<id>"`, a saved connection's `id` from `connections.toml`. Tabletist writes the connection's name in a comment beside it, `on_launch = "conn:6a0c…"  # Bookshop`, and replaces whatever comment is beside it each time it sets the key, and again when that connection is renamed | `"restore"` |
 | `startup.restore_tabs` | `true` or `false`. Whether a connection opens again the table tabs, sorts and filters its window last had, kept in `session.json`. The row panel comes back as the window last left it either way | `true` |
 | `data.page_size` | 10 to 10000. A table takes a new size when it next loads (opened, refreshed, sorted, filtered or reconnected), not while you page through it | `1000` |
````
````diff
--- /dev/null
+++ b/docs/_reference/translating.md
@@ -0,0 +1,93 @@
+---
+title: Translating
+description: How Tabletist's interface is translated, and how to add or improve a language.
+nav_order: 4
+---
+
+Tabletist's interface is in English and Русский. Each language is one plain
+text file, a gettext catalog, in
+[`assets/i18n`](https://github.com/igor-alexandrov/tabletist/tree/main/assets/i18n):
+`ru.po` for Russian. Every string the interface draws is looked up there,
+and one the catalog lacks shows in English, so a catalog can be finished a
+part at a time.
+
+Only the app is translated. This website and its guide stay in English.
+
+## Improving a language
+
+Open the language's `.po` file in a text editor or in
+[Poedit](https://poedit.net). Each entry is an English message (`msgid`)
+and its translation (`msgstr`):
+
+```po
+#: src/ui/settings/mod.rs
+msgid "Help translate"
+msgstr "Помочь с переводом"
+```
+
+The `#:` line says which file uses the message: read the code around it
+when a short message could mean more than one thing.
+
+To try a change, build Tabletist ([Build from Source](../build-from-source/))
+and choose the language in Settings › General, or set it in
+[settings.toml](../settings-and-files/#the-settings-file):
+
+```toml
+[interface]
+language = "ru"
+```
+
+Then open a pull request with the file.
+
+## Adding a language
+
+Open an [issue](https://github.com/igor-alexandrov/tabletist/issues) first:
+a new language also needs a few lines of code, which name it in the
+language menu and tie it to its catalog.
+Then start its catalog from the template:
+
+```sh
+msginit --no-translator --locale=pt_BR.UTF-8 \
+  --input=assets/i18n/tabletist.pot --output-file=assets/i18n/pt-BR.po
+```
+
+## What to keep as it is
+
+- Keys and chords (`⌘S`, `Ctrl+Shift+P`, `j/k`, `ctrl+b`), and `:`
+  commands (`:w`, `:set calendar`). What a key does is translated; the key
+  is not.
+- SQL and its keywords, names of tables, columns and types, and data.
+- Product names: Tabletist, Omarchy, PostgreSQL, MySQL, SQLite, Redis,
+  DuckDB, MCP.
+- Environment names: dev, staging, production, DEV, PROD.
+- Placeholders in braces, `{n}`, `{name}`, `{table}`, exactly as written:
+  the app puts a number or a name in their place. Their order in your
+  sentence is yours to choose.
+- A leading or trailing space, and a trailing colon or `…`.
+
+The Omarchy look writes every label in lower case, so write ordinary
+sentence case.
+
+## Plurals
+
+A message with a number has an English singular (`msgid`) and plural
+(`msgid_plural`), and as many translations as the language has forms,
+which the catalog's `Plural-Forms` header counts. Russian has three:
+
+```po
+msgid "{n} row"
+msgid_plural "{n} rows"
+msgstr[0] "{n} строка"
+msgstr[1] "{n} строки"
+msgstr[2] "{n} строк"
+```
+
+## After a change to the code
+
+When a change adds, changes or takes out a message, run
+`assets/i18n/update.sh` from the repository. It writes the template,
+`assets/i18n/tabletist.pot`, adds the new messages to every catalog
+untranslated, and checks each catalog. It needs GNU gettext 0.24 or
+later, which reads Rust: `brew install gettext` on macOS, or your
+distribution's `gettext` where it is that new. CI runs it with `--check`
+and fails when the template or a catalog is out of date.
````
```diff
--- a/src/ui/settings/mod.rs
+++ b/src/ui/settings/mod.rs
@@ -23,19 +23,20 @@
 
 use crate::app::App;
 use crate::connections::SavedConnection;
-use crate::i18n::{Locale, gettext, ngettext};
+use crate::i18n::{LanguageTag, Locale, gettext, ngettext};
 use crate::keymap::{Command, Keymap, Layout, Scope, Spelled, Stroke};
 use crate::model::{Action, AgentsView, Dialog, SettingsDialog, SettingsPane, SettingsRow};
 use crate::settings::{
-    Autocomplete, Commas, EditorKeys, EnvBar, Idle, KeywordCase, Launch, LineNumbers, LookChoice,
-    Mode, NewTab, OptionId, OptionValue, RowHeight, RunScope, Settings, TableList, Theme,
-    ThemeChoice, Timestamps, UpdateCheck, Zone,
+    Autocomplete, Commas, EditorKeys, EnvBar, Idle, KeywordCase, Language, Launch, LineNumbers,
+    LookChoice, Mode, NewTab, OptionId, OptionValue, RowHeight, RunScope, Settings, TableList,
+    Theme, ThemeChoice, Timestamps, UpdateCheck, Zone,
 };
 
 /// The label of an option's row. English: the caller translates, and the
 /// terminal look lower-cases. Quick open lists the rows by it.
 pub(crate) fn label(option: OptionId) -> &'static str {
     match option {
+        OptionId::Language => "Language",
         OptionId::OnLaunch => "On launch",
         OptionId::RestoreTabs => "Restore tabs and filters",
         OptionId::PageSize => "Rows per page",
@@ -148,6 +149,7 @@
         | OptionId::Wrap
         | OptionId::Keywords
         | OptionId::Commas
+        | OptionId::Language
         | OptionId::UpdatesCheck => label(option),
     }
 }
@@ -158,6 +160,7 @@
         // As the board writes it, though hiding columns is not built: what
         // the option will cover once it is.
         OptionId::RestoreTabs => Some("Per connection, including sort and hidden columns"),
+        OptionId::Language => Some(LANGUAGE_PRINT),
         // The board's hint: beside the menu it has no room in the sheet.
         OptionId::OnLaunch => Some("Or: show Connections · open a chosen connection"),
         OptionId::PageSize => Some("Table view; the SQL editor has its own limit"),
@@ -234,6 +237,79 @@
     }
 }
 
+/// The contributor guide, which "Help translate" opens.
+pub(super) const TRANSLATING: &str = "https://igor.works/tabletist/translating/";
+
+/// The Language row's small print. English, as [`small_print`]'s.
+const LANGUAGE_PRINT: &str = "Menus, labels and messages; data and SQL stay as they are";
+
+/// The words of the Language row, translated: its label, the section's
+/// heading, and the link to the contributor guide. Said here in full so
+/// the template has them: [`label`] hands its words to `gettext` in a
+/// variable, which no extractor reads.
+pub(super) fn language_words(locale: Locale) -> [String; 3] {
+    [
+        gettext(locale, "Language").into_owned(),
+        gettext(
+            locale,
+            "Menus, labels and messages; data and SQL stay as they are",
+        )
+        .into_owned(),
+        gettext(locale, "Help translate").into_owned(),
+    ]
+}
+
+/// The English word beside the Language row's label in any other
+/// language: the way back for someone who chose one they cannot read.
+/// Never translated.
+pub(super) fn language_aside_label(locale: Locale) -> Option<&'static str> {
+    (locale != Locale::English).then_some("Language")
+}
+
+/// A language as the menu and the row write it: in its own name, and
+/// System with the language it is drawn in (`System (Русский)`). Where
+/// the system's first language (`system`) has no catalog, System is
+/// written as English, the language it stands for.
+pub(super) fn language_text(
+    locale: Locale,
+    language: Language,
+    system: Option<&LanguageTag>,
+) -> String {
+    match language.shown(system) {
+        Language::System => gettext(locale, "System ({language})")
+            .replace("{language}", language.resolve(system).native_name()),
+        Language::Fixed(fixed) => fixed.native_name().to_owned(),
+    }
+}
+
+/// A language's name in the window's language, which the menu writes
+/// beside its own: none for the window's own language.
+pub(super) fn language_aside(locale: Locale, of: Locale) -> Option<String> {
+    let name = match of {
+        Locale::English => gettext(locale, "English"),
+        Locale::Russian => gettext(locale, "Russian"),
+    };
+    (of != locale).then(|| name.into_owned())
+}
+
+/// What the Language row says under its label: that the system's first
+/// language is not translated yet, where System stands for it in English,
+/// and else its small print. A language with no name in the list is
+/// named by its tag.
+pub(super) fn language_print(
+    locale: Locale,
+    language: Language,
+    system: Option<&LanguageTag>,
+) -> String {
+    match system {
+        Some(tag) if language == Language::System && Locale::from_tag(tag).is_none() => {
+            let name = crate::i18n::endonym(tag).unwrap_or(&tag.language);
+            gettext(locale, "{language} is not translated yet").replace("{language}", name)
+        }
+        _ => language_words(locale)[1].clone(),
+    }
+}
+
 /// A zone as the window writes it: the computer's with its own name
 /// beside it where it has one (`System · America/New_York`), UTC, or the
 /// zone's name. `system` is the computer's zone's name. English: the
@@ -415,6 +491,7 @@
         palette: &app.palette,
         locale: app.locale,
         system: app.system_zone.iana_name(),
+        language: app.system_language.as_ref(),
         zones: app.zones.as_deref().map_or(&[], crate::zones::Zones::names),
         connections: &app.connections.connections,
         fonts: sheet::DataFonts {
@@ -460,6 +537,7 @@
         window: app.agents_window().is_some(),
         system: app.system_zone.iana_name(),
         named: app.named_zone.as_deref(),
+        language: app.system_language.as_ref(),
         connections: &app.connections.connections,
         launches,
         desktop: desktop_shown(app),
@@ -553,11 +631,15 @@
 pub(super) fn choices(
     option: OptionId,
     settings: &Settings,
-    named: Option<&str>,
+    (named, language): (Option<&str>, Option<&LanguageTag>),
     launches: &[Launch],
 ) -> Vec<(Said, OptionValue)> {
     use Said::{Key, Sample, Word};
     match option {
+        OptionId::Language => vec![
+            (Word("fewer"), settings.stepped_language(language, false)),
+            (Word("more"), settings.stepped_language(language, true)),
+        ],
         OptionId::OnLaunch => vec![
             (Word("fewer"), settings.stepped_launch(launches, false)),
             (Word("more"), settings.stepped_launch(launches, true)),
@@ -1071,6 +1153,9 @@
                     }
                     let value = match asked {
                         Asked::Reset => Some(option.default_value()),
+                        Asked::Step { forward } if option == OptionId::Language => {
+                            Some(settings.stepped_language(app.system_language.as_ref(), forward))
+                        }
                         Asked::Step { forward } if option == OptionId::Tz => {
                             Some(settings.stepped_zone(app.named_zone.as_deref(), forward))
                         }
```
```diff
--- a/src/ui/settings/sheet.rs
+++ b/src/ui/settings/sheet.rs
@@ -93,6 +93,8 @@
     pub(super) locale: Locale,
     /// The computer's zone's name, which the System zone writes beside it.
     pub(super) system: Option<&'a str>,
+    /// The system's first language, which the System language follows.
+    pub(super) language: Option<&'a crate::i18n::LanguageTag>,
     /// The zone database's names, which the zone menu lists: none until
     /// the backend has read it.
     pub(super) zones: &'a [String],
@@ -779,12 +781,13 @@
 /// from the labels, and the small print wraps.
 pub(super) fn label_column(ui: &Ui, options: &[OptionId], skin: &Skin) -> f32 {
     let role = TextRole::Secondary;
-    // Keep query history's small print, the longest, wraps under its
-    // label: the column it would make leaves the other rows' hints no room.
+    // Keep query history's and the language's small print, the longest,
+    // wrap under their labels: the column either would make leaves the
+    // other rows' hints no room.
     let widest = options
         .iter()
         .copied()
-        .filter(|option| *option != OptionId::HistoryKeep)
+        .filter(|option| !matches!(option, OptionId::HistoryKeep | OptionId::Language))
         .filter_map(small_print)
         .map(|print| role.width(ui.ctx(), skin.look.faces, &skin.say(print)))
         .fold(LABEL, f32::max);
@@ -859,7 +862,14 @@
         TextRole::UiBody,
         &said(ui.ctx(), look, skin.locale, label(option)),
         palette.text,
-    )
+    );
+    // «Язык · Language»: the English word, dim, in any other language.
+    let name = match super::language_aside_label(skin.locale) {
+        Some(english) if option == OptionId::Language => {
+            name.add(TextRole::UiBody, &format!(" · {english}"), palette.dim)
+        }
+        _ => name,
+    }
     .wrap(column)
     .layout(ui.ctx());
     // A data font that is not installed is said in place of the small
@@ -873,6 +883,12 @@
         }
         None => match &shown {
             Some(shown) => Some((shown.text.to_owned(), palette.dim)),
+            // That the system's language is not translated yet, where it
+            // is: else the row's small print.
+            None if option == OptionId::Language => Some((
+                super::language_print(skin.locale, settings.language, skin.language),
+                palette.dim,
+            )),
             None => small_print(option).map(|print| (skin.say(print), palette.dim)),
         },
     };
@@ -1039,7 +1055,9 @@
 /// How tall `option`'s control is: its row is at least as tall.
 fn control_height(option: OptionId, look: &Look) -> f32 {
     match option {
-        OptionId::PageSize | OptionId::Tz | OptionId::OnLaunch => look.control_height,
+        OptionId::PageSize | OptionId::Tz | OptionId::OnLaunch | OptionId::Language => {
+            look.control_height
+        }
         // The track has 2 over its segments and under them.
         OptionId::Timestamps | OptionId::GroupDigits => SEGMENT + 4.0,
         OptionId::Look | OptionId::RowHeight | OptionId::TableList | OptionId::EnvBar => {
@@ -1284,6 +1302,10 @@
         }
         OptionId::Tz => zone_menu(ui, (&name, description), settings, skin, actions),
         OptionId::OnLaunch => launch_menu(ui, (&name, description), settings, skin, actions),
+        OptionId::Language => {
+            language_menu(ui, (&name, description), settings, skin, actions);
+            help_translate(ui, skin);
+        }
         OptionId::Timestamps => {
             let words = [skin.say("To the second"), skin.say("Full precision")];
             let selected = usize::from(settings.timestamps == Timestamps::Full);
@@ -1689,6 +1711,76 @@
     });
 }
 
+/// The language's menu: System where the system's first language has a
+/// catalog, then each language in its own name, with its name in the
+/// window's language beside it.
+fn language_menu(
+    ui: &mut Ui,
+    (name, description): (&str, Option<&str>),
+    settings: &Settings,
+    skin: &Skin,
+    actions: &mut Vec<Action>,
+) {
+    let Skin { look, palette, .. } = *skin;
+    let system = skin.language;
+    let shown = super::language_text(skin.locale, settings.language, system);
+    let current = settings.language.shown(system);
+    let dropdown = menu::Dropdown::new("language", &shown, ZONE_MENU).description(description);
+    let menu = dropdown.show(ui, look, palette, |ui| {
+        for language in crate::settings::Language::choices(system) {
+            let text = super::language_text(skin.locale, language, system);
+            let aside = match language {
+                crate::settings::Language::System => None,
+                crate::settings::Language::Fixed(locale) => {
+                    super::language_aside(skin.locale, locale)
+                }
+            };
+            let entry_name = format!("{name} {text}");
+            let mut entry = menu::Item::choice(&text, language == current).name(&entry_name);
+            if let Some(aside) = &aside {
+                entry = entry.detail(aside);
+            }
+            if entry.show(ui, look, palette).clicked() {
+                actions.push(Action::SetOption(OptionValue::Language(language)));
+                ui.close();
+            }
+            // System stands apart from the languages.
+            if language == crate::settings::Language::System {
+                menu::separator(ui, palette);
+            }
+        }
+    });
+    menu.response.widget_info(|| {
+        let mut info = WidgetInfo::labeled(WidgetType::ComboBox, true, name);
+        info.current_text_value = Some(shown.clone());
+        info
+    });
+}
+
+/// "Help translate" after the language's menu: the contributor guide, in
+/// the browser.
+fn help_translate(ui: &mut Ui, skin: &Skin) {
+    let Skin { palette, look, .. } = *skin;
+    let text = super::language_words(skin.locale)[2].clone();
+    ui.add_space(HINT_GAP);
+    let role = TextRole::Secondary;
+    let width = role.width(ui.ctx(), look.faces, &text).ceil();
+    let (place, _) = ui.allocate_exact_size(vec2(width, LINK), Sense::hover());
+    let link = link(
+        ui,
+        ui.id().with("help-translate"),
+        place.left_center(),
+        width,
+        &text,
+        palette.accent,
+        skin,
+    );
+    if pressed(ui, &link) {
+        ui.ctx()
+            .open_url(egui::OpenUrl::new_tab(super::TRANSLATING));
+    }
+}
+
 /// The zone's menu: the computer's zone and UTC, then every zone of the
 /// database, which a search field over them narrows. What was typed there
 /// is the field's own, kept in egui's memory while the menu is open.
@@ -1755,7 +1847,7 @@
 fn hint(ui: &Ui, at: egui::Pos2, right: f32, option: OptionId, settings: &Settings, skin: &Skin) {
     let Skin { look, palette, .. } = *skin;
     let text = match option {
-        OptionId::PageSize | OptionId::Tz => return,
+        OptionId::PageSize | OptionId::Tz | OptionId::Language => return,
         OptionId::Timestamps => Text::one(
             look,
             TextRole::MonoSecondary,
```
```diff
--- a/src/ui/settings/terminal.rs
+++ b/src/ui/settings/terminal.rs
@@ -59,6 +59,8 @@
     pub(super) system: Option<&'a str>,
     /// The last named zone the session had, which the zone's steps reach.
     pub(super) named: Option<&'a str>,
+    /// The system's first language, which the System language follows.
+    pub(super) language: Option<&'a crate::i18n::LanguageTag>,
     /// The saved connections, and what On launch steps through.
     pub(super) connections: &'a [crate::connections::SavedConnection],
     pub(super) launches: &'a [crate::settings::Launch],
@@ -570,7 +572,8 @@
     );
     let mut index = 0;
     for (heading_text, top, options) in [
-        ("Startup", 8.0, &OptionId::STARTUP[..]),
+        ("Language", 8.0, &OptionId::LANGUAGE[..]),
+        ("Startup", 12.0, &OptionId::STARTUP[..]),
         ("Data", 12.0, &OptionId::DATA[..]),
         ("Safety", 12.0, &OptionId::SAFETY[..]),
     ] {
@@ -1154,7 +1157,14 @@
         .fold(0.0, f32::max);
     let hints = options
         .iter()
-        .flat_map(|option| choices(*option, settings, skin.named, skin.launches))
+        .flat_map(|option| {
+            choices(
+                *option,
+                settings,
+                (skin.named, skin.language),
+                skin.launches,
+            )
+        })
         .map(|(_, value)| {
             let mut changed = settings.clone();
             value.set(&mut changed);
@@ -1218,6 +1228,7 @@
         | OptionId::Tz
         | OptionId::IdleDisconnect
         | OptionId::OnLaunch
+        | OptionId::Language
         | OptionId::FontSize
         | OptionId::RowLimit
         | OptionId::Timeout
@@ -1246,7 +1257,7 @@
         | OptionId::LineNumbers
         | OptionId::Keywords
         | OptionId::Commas => {
-            let words = choices(option, settings, skin.named, skin.launches);
+            let words = choices(option, settings, (skin.named, skin.language), skin.launches);
             let between = width(" ") * words.len().saturating_sub(1) as f32;
             let segments = words.into_iter().map(|(word, _)| segment(ui, word, skin));
             segments.sum::<f32>() + between
@@ -1279,6 +1290,12 @@
             Idle::Never => skin.say("Never"),
             choice => choice.name().to_owned(),
         },
+        // As the artboard writes it: `system (русский)`, `english`.
+        OptionId::Language => skin.look.label(&super::language_text(
+            skin.locale,
+            settings.language,
+            skin.language,
+        )),
         // As the artboard writes it: `system (america/new_york)`.
         OptionId::Tz => skin.look.label(&match (&settings.tz, skin.system) {
             (crate::settings::Zone::System, Some(name)) => {
@@ -1473,6 +1490,7 @@
                 | OptionId::Tz
                 | OptionId::IdleDisconnect
                 | OptionId::OnLaunch
+                | OptionId::Language
                 | OptionId::FontSize
                 | OptionId::RowLimit
                 | OptionId::Timeout
@@ -1506,6 +1524,7 @@
         | OptionId::Tz
         | OptionId::IdleDisconnect
         | OptionId::OnLaunch
+        | OptionId::Language
         | OptionId::FontSize
         | OptionId::RowLimit
         | OptionId::Timeout
@@ -1600,9 +1619,12 @@
     let y = cell.row.center().y;
     let value = stepped_text(option, settings, skin);
     let current = settings.value(option);
-    let Ok([fewer, more]) =
-        <[_; 2]>::try_from(choices(option, settings, skin.named, skin.launches))
-    else {
+    let Ok([fewer, more]) = <[_; 2]>::try_from(choices(
+        option,
+        settings,
+        (skin.named, skin.language),
+        skin.launches,
+    )) else {
         return;
     };
     let text = if cell.on_cursor {
@@ -1682,7 +1704,7 @@
     );
     // One mark stands for both values, and a click on it flips it: the
     // value it is not set to is the button on top.
-    let mut values = choices(option, settings, skin.named, skin.launches);
+    let mut values = choices(option, settings, (skin.named, skin.language), skin.launches);
     values.sort_by_key(|(_, value)| *value != current);
     let mut group = radio_group(ui, cell, option, skin);
     for (word, value) in values {
@@ -1700,6 +1722,26 @@
     let role = TextRole::OBody;
     let dim = |text: &str| Text::one(look, role, text, palette.dim);
     match option {
+        OptionId::Language => {
+            // Where the system's first language is not translated, said
+            // first: `français is not translated yet · help translate: …`.
+            let missing = skin
+                .language
+                .filter(|tag| {
+                    settings.language == crate::settings::Language::System
+                        && Locale::from_tag(tag).is_none()
+                })
+                .map(|_| super::language_print(skin.locale, settings.language, skin.language));
+            // The variable's name keeps its case.
+            let from =
+                missing.unwrap_or_else(|| gettext(skin.locale, "from {variable}").into_owned());
+            let help = gettext(skin.locale, "help translate: {folder}");
+            let said = look
+                .label(&format!("{from} · {help}"))
+                .replace("{variable}", "$LANG")
+                .replace("{folder}", "assets/i18n");
+            dim(&said)
+        }
         OptionId::PageSize => dim(&skin.say("table view")),
         OptionId::Tz => dim(&skin.say("sent with offset")),
         OptionId::Timestamps => dim(sample_timestamp(settings.timestamps)),
@@ -2203,7 +2245,7 @@
     let gap = role.width(ui.ctx(), look.faces, " ");
     let mut group = radio_group(ui, cell, option, skin);
     let mut left = cell.left;
-    for (word, value) in choices(option, settings, skin.named, skin.launches) {
+    for (word, value) in choices(option, settings, (skin.named, skin.language), skin.launches) {
         let text = skin.written(ui.ctx(), word);
         let width = segment(ui, word, skin);
         let place = Rect::from_min_size(
```

- [ ] **Step 4: Add the row's messages to the catalog**

```bash
assets/i18n/update.sh
```

It adds 9 untranslated messages to `ru.po`. Fill them in as below, then run `assets/i18n/update.sh --check`. Expected: `720 translated messages.`

| msgid | msgstr |
| --- | --- |
| `Language` | `Язык` |
| `Menus, labels and messages; data and SQL stay as they are` | `Меню, подписи и сообщения; данные и SQL остаются как есть` |
| `Help translate` | `Помочь с переводом` |
| `System ({language})` | `Системный ({language})` |
| `English` | `Английский` |
| `Russian` | `Русский` |
| `{language} is not translated yet` | `{language}: перевода пока нет` |
| `from {variable}` | `из {variable}` |
| `help translate: {folder}` | `помочь с переводом: {folder}` |

- [ ] **Step 5: Run the tests and the checks**

Run: `cargo test -p tabletist --lib -- ui::language_tests ui::tests ui::settings_access_tests` and `cargo test -p tabletist-core --lib settings::`
Expected: they pass, the core's `a_step_moves_through_the_menu_and_stops_at_its_ends` among them. The language tests are `the_language_is_general_s_first_row_in_both_looks`, `system_says_the_language_it_stands_for`, `a_system_language_with_no_catalog_reads_english_and_says_so`, `choosing_russian_redraws_the_window_and_writes_the_file`, `reset_to_defaults_leaves_the_language`, `the_omarchy_row_steps_through_the_languages_and_r_puts_back_system` and `the_omarchy_row_says_a_system_language_is_not_translated`. Then every check passes.

- [ ] **Step 6: Check it by hand**

```bash
cargo run -- --demo
```

- **Mac:** open Settings (⌘,) and choose Русский in the Language menu. The sheet's title becomes «Настройки», the row reads «Язык · Language», and the menu bar's File, Edit and View menus read «Файл», «Изменить» and «Вид» (its items, and the Query, Window and Help menus, stay English until run 2). Choose English again.
- **Omarchy:** in the Omarchy look (`appearance.look = "omarchy"`), the row's `‹ system (english) ›` steps with `l` to `english`, then `русский`, and R puts it back.

Keep any screenshot local.

- [ ] **Step 7: Commit**

```bash
git add -A crates src assets/i18n docs/_reference docs/_guide
git commit -m "Choose the interface's language in Settings › General"
```

---

## After the tasks

- Open the PR against `main` with the spec and this plan, which are committed first and on their own.
- Two things to watch in CI:
  - The new `translations` job, which runs on macOS and installs `gettext` from Homebrew.
  - The Windows test job, because the sheet's dropdown is shared with Windows.
