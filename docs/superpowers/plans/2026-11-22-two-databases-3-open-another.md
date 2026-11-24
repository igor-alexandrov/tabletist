# Two databases of one connection, run 3: the picker's "Open another": Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use rubypowers:subagent-driven-development (recommended) or rubypowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** In the connection picker, an open connection's row menu reads "Open another" (was "Connect again") with its key on the row while it is lit, and the footer names the key, so a second window of a connection can be found from the picker. (igor-alexandrov/tabletist#366, run 3 of 3.)

**Architecture:** Three tasks, one PR, a commit each:
1. **The row menu:** "Open another" in place of "Connect again", its key (`Command::ConnectAgain`, as the look writes it) at the row's right while it is lit, through the menu row's `hint` that run 1 added, with no words beside it.
2. **The footer:** "⇧↩ open another" after "↩ connect" (Windows and Linux "Shift+Enter open another"), and Omarchy's "shift+enter another" after "enter connect / show".
3. **The docs:** the reference's Connections page and the three key tables say what Shift+Enter does now.

**Tech Stack:** Rust and egui in `src/ui/picker.rs` (the picker), `src/ui/menu.rs` (menu rows), the headless tests in `src/ui/two_databases_tests.rs` and `src/ui/mod.rs`, and the gettext catalogs in `assets/i18n/`.

**Spec:** `docs/superpowers/specs/2026-11-21-two-databases-design.md`: decision 2 (picker: the rename and the footer hint), decision 11 (the key on the lit menu row, from the canvas), and run 3 under Runs.

**Base:** main at 8af8170c (run 2, #376, merged). Work on `claude/two-databases-366-run3`.

---

## What this run builds

| The canvas and the spec say | This run builds |
|---|---|
| Right-click "Connect again" is renamed "Open another" | `msgid("Open another")` in `row_response`'s menu; ru "Открыть ещё окно", pt-BR "Abrir outra janela" |
| The lit "Open another" row shows `⇧↩` at its right (`MacTwoDatabases`, board 4) | `menu::Item::hint("", again)`: the key alone, on the lit row only, as the database menu's hint is; `menu.rs` lets the words be empty |
| macOS footer: "↩ connect ⇧↩ open another ⌘E edit ⌘D duplicate ⌫ delete" | `format!("{again} open another")` after connect; the footer's words are English in every locale, as the others are |
| Omarchy footer: "j/k move · enter connect / show · shift+enter another · e edit …" | `(&again, pmsgid("key hint", "another"))` after connect / show; ru "ещё окно", pt-BR "outra janela". A narrow window drops hints from the end, as before |
| It opens on the saved database; the row is unchanged | Unchanged: Open another is `Opening::connect`, a connect in the picker's tab, as Connect again was |

What the item and the footer do is not new: Shift+Enter and the menu item opened an open connection once more before this run. Only their words and the key's hints are new. The footer names Shift+Enter whatever row is selected, as it names the other keys; on a connection that is not open it connects, as Enter does.

## Files

| File | Task | What changes |
|---|---|---|
| `src/ui/picker.rs` | 1, 2 | `RowSkin::again`; `row_response` takes the skin; the menu's Open another (1); the footer's hint (2) |
| `src/ui/menu.rs` | 1 | `Item::hint` with no words draws the key alone |
| `src/ui/two_databases_tests.rs` | 1 | The row menu's tests |
| `src/ui/mod.rs` | 2 | `the_pickers_footer_names_the_keys_the_keymap_has` names the new hint |
| `assets/i18n/tabletist.pot`, `assets/i18n/ru.po`, `assets/i18n/pt-BR.po` | 1, 2 | "Open another" (1), "another" in the key hint context (2) |
| `docs/_reference/connections.md`, `docs/_reference/keyboard-shortcuts.md`, `docs/_guide/macos.md`, `docs/_guide/omarchy.md` | 3 | The docs |

## Checks

Each code task ends with these, all passing:

```bash
~/.cargo/bin/cargo fmt --all --check
~/.cargo/bin/cargo clippy --locked --workspace --all-targets -- -D warnings
~/.cargo/bin/cargo test --locked --workspace --all-targets
RUSTDOCFLAGS='-D warnings' ~/.cargo/bin/cargo doc --locked --workspace --no-deps
assets/i18n/update.sh --check
```

Use `~/.cargo/bin/cargo`: the mise shim fails here. Keep `CARGO_TARGET_DIR` unset (the worktree's `target/`), never under `/tmp`. After a message is added or changed, run `assets/i18n/update.sh` (no flag) and fill the new entries' `msgstr` in `ru.po` and `pt-BR.po` by hand, as the diffs below do; the pt-BR words want a native speaker's review, as the rest of that catalog does. The catalogs' `POT-Creation-Date` line reads the time you run it, not the one in the diffs.

---

### Task 1: The row menu's "Open another"

**Files:**
- Modify: `src/ui/picker.rs`, `src/ui/menu.rs`, `assets/i18n/tabletist.pot`, `assets/i18n/ru.po`, `assets/i18n/pt-BR.po`
- Test: `src/ui/two_databases_tests.rs`

What it holds: the menu of an open connection's row reads Show, Open another, Edit…, Duplicate, Delete (then the Agents part, where agents can reach the app). Open another connects in the picker's tab, on the saved database, as Connect again did. While the row is lit, by the pointer or the keyboard, its right shows the key that does the same from the list (`⇧↩`, `Shift+Enter`, Omarchy `shift+enter` in the accent), written from the keymap. Its room is kept whether it is lit or not, so the menu keeps its width. A screen reader hears the key as the row's description.

The key comes to the row through `RowSkin`, which both rows (the macOS and Windows card, the terminal line) pass on to `row_response` in place of the `(look, palette, locale)` they passed before.

- [ ] **Step 1: Write the failing tests**

```diff
diff --git a/src/ui/two_databases_tests.rs b/src/ui/two_databases_tests.rs
--- a/src/ui/two_databases_tests.rs
+++ b/src/ui/two_databases_tests.rs
@@ -590,3 +590,89 @@
     let second = harness.app.active_tab_id();
     assert_eq!(database(&harness, second), "bookshop_test");
 }
+
+/// The picker in `look` with the fixture connection open in a window and
+/// its row selected. Returns the picker's tab and the open window.
+fn picker_with_it_open(look: Look) -> (Harness, ConnTabId, ConnTabId) {
+    let mut harness = Harness::new();
+    harness.set_look(look);
+    let open = harness.connect_fake();
+    harness.app.apply(Action::ShowConnections);
+    let picker = harness.app.active_tab_id();
+    let conn = Some(harness.app.workspace(open).unwrap().conn_id.clone());
+    harness
+        .app
+        .apply(Action::SelectConnection { tab: picker, conn });
+    harness.settle();
+    (harness, picker, open)
+}
+
+/// The windows on `open`'s connection, `open` among them.
+fn windows_on(harness: &Harness, open: ConnTabId) -> usize {
+    let conn = &harness.app.workspace(open).unwrap().conn_id;
+    let tabs = harness.app.tabs.iter();
+    tabs.filter(|tab| {
+        let workspace = harness.app.workspace(tab.id);
+        workspace.is_some_and(|workspace| &workspace.conn_id == conn)
+    })
+    .count()
+}
+
+/// Right-clicks the picker's row of the fixture connection, as a mouse
+/// does, and returns where its menu's row of `name` is.
+fn row_menu(harness: &mut Harness, name: &str) -> egui::Rect {
+    let tree = harness.settle();
+    let at = bounds(&tree, "Fixture", Role::Button)
+        .unwrap_or_else(|| panic!("no row: {:?}", labels(&tree)))
+        .center();
+    let press = |pressed| egui::Event::PointerButton {
+        pos: at,
+        button: egui::PointerButton::Secondary,
+        pressed,
+        modifiers: Modifiers::NONE,
+    };
+    harness.frame(vec![egui::Event::PointerMoved(at)]);
+    harness.frame(vec![press(true)]);
+    harness.frame(vec![press(false)]);
+    let tree = harness.settle();
+    bounds(&tree, name, Role::Button)
+        .unwrap_or_else(|| panic!("no {name:?} in the menu: {:?}", labels(&tree)))
+}
+
+#[test]
+fn open_another_in_an_open_connections_menu_opens_one_more_window() {
+    for look in [Look::macos(), Look::omarchy()] {
+        let (mut harness, picker, open) = picker_with_it_open(look);
+        row_menu(&mut harness, "Open another");
+        assert!(!harness.has("Connect again"), "{}", look.name);
+        assert_eq!(windows_on(&harness, open), 1, "{}", look.name);
+        harness.click("Open another");
+        // The picker's tab connects, on the saved database, beside the
+        // window that had it open.
+        assert_eq!(windows_on(&harness, open), 2, "{}", look.name);
+        assert!(harness.app.workspace(picker).is_some(), "{}", look.name);
+        assert!(harness.app.workspace(open).is_some(), "{}", look.name);
+    }
+}
+
+#[test]
+fn open_another_names_its_key_while_it_is_lit() {
+    for look in Look::ALL {
+        let (mut harness, ..) = picker_with_it_open(look);
+        let command = crate::keymap::Command::ConnectAgain;
+        let key = crate::ui::keys::written(&harness.ctx, &look, command);
+        let key = key.as_str().to_owned();
+        let show = row_menu(&mut harness, "Show");
+        harness.frame(vec![egui::Event::PointerMoved(show.center())]);
+        harness.settle();
+        assert_eq!(harness.painted_rect(&key), None, "{}: Show lit", look.name);
+        let tree = harness.settle();
+        let another = bounds(&tree, "Open another", Role::Button).unwrap();
+        harness.frame(vec![egui::Event::PointerMoved(another.center())]);
+        harness.settle();
+        let hint = harness
+            .painted_rect(&key)
+            .unwrap_or_else(|| panic!("{}: no {key:?}", look.name));
+        assert!(another.contains(hint.center()), "{}: {hint:?}", look.name);
+    }
+}
```

The menu opens where the pointer is, over the row: the second test opens it once and moves the pointer from Show to Open another, never right-clicking twice.

- [ ] **Step 2: Run them to see them fail**

Run: `~/.cargo/bin/cargo test --locked -p tabletist --lib two_databases_tests::open_another`

Expected: both fail, the menu still reading "Connect again":

```
test ui::two_databases_tests::open_another_in_an_open_connections_menu_opens_one_more_window ... FAILED
test ui::two_databases_tests::open_another_names_its_key_while_it_is_lit ... FAILED
thread 'ui::two_databases_tests::open_another_in_an_open_connections_menu_opens_one_more_window' panicked at src/ui/two_databases_tests.rs:639:28:
no "Open another" in the menu: ["Tables, columns and limits…", "Show Agents tab", "Search connections", "Agents", "Delete", "Local", "New connection", "Show", "/tmp/", "fixture.db", "Local", "open", "Show Fixture", "More actions for Fixture", "No SSH", "SQLite", "Edit Fixture", "Connect again", "", "Agents Off", "Edit…", "Agents Read", "Off", "Last used just now", "Agents", "Duplicate", "Agents Off", "Fixture"]
thread 'ui::two_databases_tests::open_another_names_its_key_while_it_is_lit' panicked at src/ui/two_databases_tests.rs:670:67:
test result: FAILED. 0 passed; 2 failed; 0 ignored; 0 measured; 3092 filtered out; finished in …
```

- [ ] **Step 3: Write the code**

```diff
diff --git a/src/ui/picker.rs b/src/ui/picker.rs
--- a/src/ui/picker.rs
+++ b/src/ui/picker.rs
@@ -384,6 +384,7 @@
         key(Command::FilterConnections),
     );
     let agent_access = key(Command::ChangeAgentAccess);
+    let again = key(Command::ConnectAgain);
     let answers = (
         key(Command::ConfirmDeleteConnection),
         key(Command::KeepConnection),
@@ -635,6 +636,7 @@
                         locale,
                         now,
                         side,
+                        again: &again,
                     };
                     if look.terminal {
                         for connection in &group.connections {
@@ -768,6 +770,9 @@
     locale: crate::i18n::Locale,
     now: u64,
     side: f32,
+    /// The key that opens an open connection once more, as the look
+    /// writes it: the row menu's Open another names it.
+    again: &'a crate::keymap::Written,
 }
 
 /// Where a row's connection opens: the picker's tab, or the tab that has
@@ -924,9 +929,16 @@
     rect: Rect,
     connection: &SavedConnection,
     opening: Opening,
-    (look, palette, locale): (&Look, &Palette, crate::i18n::Locale),
+    skin: RowSkin<'_>,
     actions: &mut Vec<Action>,
 ) -> egui::Response {
+    let RowSkin {
+        look,
+        palette,
+        locale,
+        again,
+        ..
+    } = skin;
     let tab = opening.tab;
     let response = ui.interact(
         rect,
@@ -960,12 +972,18 @@
                 .show(ui, look, palette)
                 .clicked()
         };
-        // An open connection shows, and can be opened once more.
+        // An open connection shows, and can be opened once more: its key
+        // on the row while it is lit, as the footer names it.
         if opening.open.is_some() {
             if item(ui, msgid("Show")) {
                 actions.push(opening.choose(connection));
             }
-            if item(ui, msgid("Connect again")) {
+            let another = msgid("Open another").say(locale);
+            if menu::Item::action(&another)
+                .hint("", again)
+                .show(ui, look, palette)
+                .clicked()
+            {
                 actions.push(opening.connect(connection));
             }
         } else if item(ui, msgid("Connect")) {
@@ -1219,14 +1237,7 @@
         pos2(card.left(), rect.top()),
         pos2(card.right(), rect.bottom()),
     );
-    let response = row_response(
-        ui,
-        rect,
-        connection,
-        opening,
-        (look, palette, locale),
-        actions,
-    );
+    let response = row_response(ui, rect, connection, opening, skin, actions);
     let is_selected = selected == Some(&connection.id);
     let rows = (card.height() / height).round() as usize;
     // The card's inner corners: its 10 less its border.
@@ -1545,17 +1556,11 @@
         locale,
         now,
         side,
+        ..
     } = skin;
     let (rect, _) =
         ui.allocate_exact_size(vec2(ui.available_width(), row_height(look)), Sense::hover());
-    let response = row_response(
-        ui,
-        rect,
-        connection,
-        opening,
-        (look, palette, locale),
-        actions,
-    );
+    let response = row_response(ui, rect, connection, opening, skin, actions);
     let is_selected = selected == Some(&connection.id);
     let center = rect.center().y;
     if is_selected {
@@ -1708,7 +1713,7 @@
         };
         assert!(connects(closed.choose(&connection)));
         // Open: choosing it shows that tab, and it can still connect again
-        // (the row menu's Connect again, and Shift+Enter).
+        // (the row menu's Open another, and Shift+Enter).
         let opened = Opening {
             tab: picker,
             open: Some(open),
diff --git a/src/ui/menu.rs b/src/ui/menu.rs
--- a/src/ui/menu.rs
+++ b/src/ui/menu.rs
@@ -160,7 +160,9 @@
     /// `words` and the `key` that does it, as the look writes them
     /// (`New window ⌘↩`, the terminal's `ctrl+enter new window` with its
     /// key in the accent). Give it to every row of the menu: each keeps
-    /// its room, so the menu keeps its width as the light moves.
+    /// its room, so the menu keeps its width as the light moves. `words`
+    /// are empty where the row's own say what the key does (`Open another
+    /// ⇧↩`): the key goes alone.
     pub fn hint(mut self, words: &'a str, key: &'a crate::keymap::Written) -> Self {
         self.hint = Some((words, key));
         self
@@ -224,7 +226,14 @@
             .detail
             .map_or(0.0, |detail| DETAIL_GAP + measure(note, detail));
         let hinted = self.hint.map_or(0.0, |(words, key)| {
-            DETAIL_GAP + measure(note, words) + shape.gap + measure(note, key.as_str())
+            // The words, where there are any, and the gap between them and
+            // the key.
+            let words = if words.is_empty() {
+                0.0
+            } else {
+                measure(note, words) + shape.gap
+            };
+            DETAIL_GAP + words + measure(note, key.as_str())
         });
         let own = measure(role, self.text);
         let after = self
@@ -256,7 +265,7 @@
             });
         }
         if let Some((words, key)) = self.hint {
-            let said = format!("{words} {}", key.as_str());
+            let said = format!("{words} {}", key.as_str()).trim_start().to_owned();
             ui.ctx().accesskit_node_builder(response.id, |node| {
                 node.set_description(said);
             });
@@ -334,12 +343,17 @@
             let right = rect.right() - shape.inset;
             if look.terminal {
                 // The key leads, in the accent, as the footers write keys.
+                let alone = words.is_empty();
                 let words = Text::one(look, note, words, palette.dim).layout(ui.ctx());
-                let width = words.width();
+                let width = if alone {
+                    0.0
+                } else {
+                    words.width() + shape.gap
+                };
                 words.paint_right(painter, right, center);
                 Text::one(look, note, key, palette.accent)
                     .layout(ui.ctx())
-                    .paint_right(painter, right - width - shape.gap, center);
+                    .paint_right(painter, right - width, center);
             } else {
                 let key = Text::one(look, note, key, palette.dim).layout(ui.ctx());
                 let width = key.width();
diff --git a/assets/i18n/tabletist.pot b/assets/i18n/tabletist.pot
--- a/assets/i18n/tabletist.pot
+++ b/assets/i18n/tabletist.pot
@@ -8,7 +8,7 @@
 msgstr ""
 "Project-Id-Version: Tabletist\n"
 "Report-Msgid-Bugs-To: https://github.com/igor-alexandrov/tabletist/issues\n"
-"POT-Creation-Date: 2026-11-21 09:54-0500\n"
+"POT-Creation-Date: 2026-11-22 21:36-0500\n"
 "PO-Revision-Date: YEAR-MO-DA HO:MI+ZONE\n"
 "Last-Translator: FULL NAME <EMAIL@ADDRESS>\n"
 "Language-Team: LANGUAGE <LL@li.org>\n"
@@ -6955,7 +6955,7 @@
 msgstr ""
 
 #: src/ui/picker.rs
-msgid "Connect again"
+msgid "Open another"
 msgstr ""
 
 #: src/ui/picker.rs
diff --git a/assets/i18n/ru.po b/assets/i18n/ru.po
--- a/assets/i18n/ru.po
+++ b/assets/i18n/ru.po
@@ -7,7 +7,7 @@
 msgstr ""
 "Project-Id-Version: Tabletist\n"
 "Report-Msgid-Bugs-To: https://github.com/igor-alexandrov/tabletist/issues\n"
-"POT-Creation-Date: 2026-11-21 09:54-0500\n"
+"POT-Creation-Date: 2026-11-22 21:36-0500\n"
 "PO-Revision-Date: 2026-11-17 22:38-0500\n"
 "Last-Translator: Automatically generated\n"
 "Language-Team: none\n"
@@ -7093,10 +7093,6 @@
 "данных, чтобы начать работу."
 
 #: src/ui/picker.rs
-msgid "Connect again"
-msgstr "Подключиться снова"
-
-#: src/ui/picker.rs
 #, rust-format
 msgid "Connect to {name}"
 msgstr "Подключиться к {name}"
@@ -7144,6 +7140,10 @@
 #: src/ui/picker.rs
 msgid "No joins"
 msgstr "Без соединений"
+
+#: src/ui/picker.rs
+msgid "Open another"
+msgstr "Открыть ещё окно"
 
 #: src/ui/picker.rs
 msgid "Passwords are stored in the macOS Keychain"
diff --git a/assets/i18n/pt-BR.po b/assets/i18n/pt-BR.po
--- a/assets/i18n/pt-BR.po
+++ b/assets/i18n/pt-BR.po
@@ -7,7 +7,7 @@
 msgstr ""
 "Project-Id-Version: Tabletist\n"
 "Report-Msgid-Bugs-To: https://github.com/igor-alexandrov/tabletist/issues\n"
-"POT-Creation-Date: 2026-11-21 09:54-0500\n"
+"POT-Creation-Date: 2026-11-22 21:36-0500\n"
 "PO-Revision-Date: 2026-11-19 16:11-0500\n"
 "Last-Translator: Automatically generated\n"
 "Language-Team: none\n"
@@ -7025,10 +7025,6 @@
 "arquivos de dados, para começar a navegar."
 
 #: src/ui/picker.rs
-msgid "Connect again"
-msgstr "Conectar novamente"
-
-#: src/ui/picker.rs
 #, rust-format
 msgid "Connect to {name}"
 msgstr "Conectar a {name}"
@@ -7078,6 +7074,10 @@
 #: src/ui/picker.rs
 msgid "No joins"
 msgstr "Sem joins"
+
+#: src/ui/picker.rs
+msgid "Open another"
+msgstr "Abrir outra janela"
 
 #: src/ui/picker.rs
 msgid "Passwords are stored in the macOS Keychain"
```

- [ ] **Step 4: Run the tests, then the checks**

Run: `~/.cargo/bin/cargo test --locked -p tabletist --lib -- two_databases_tests picker`

Expected: every test passes, the two new ones among them. Then every check under Checks passes; the app's library ends `test result: ok. 3078 passed; 0 failed; 16 ignored; 0 measured; 0 filtered out; finished in …`.

- [ ] **Step 5: Commit**

```bash
git add src/ui/picker.rs src/ui/menu.rs src/ui/two_databases_tests.rs assets/i18n
git commit -m "Call the picker row menu's second connect Open another, with its key (#366)"
```

---

### Task 2: The footer names Shift+Enter

**Files:**
- Modify: `src/ui/picker.rs`, `assets/i18n/tabletist.pot`, `assets/i18n/ru.po`, `assets/i18n/pt-BR.po`
- Test: `src/ui/mod.rs`

What it holds: the footer names the key after connect in every look: "⇧↩ open another" on macOS, "Shift+Enter open another" on Windows and Linux, and "shift+enter another" in Omarchy, written from the keymap as the others are. In the terminal look, where hints that do not fit beside the keyring's note give way from the end, the new one comes third, so a narrow window loses filter and the others first, as the canvas orders them.

- [ ] **Step 1: Write the failing test**

```diff
diff --git a/src/ui/mod.rs b/src/ui/mod.rs
--- a/src/ui/mod.rs
+++ b/src/ui/mod.rs
@@ -3877,6 +3877,7 @@
         let omarchy = painted(Look::omarchy());
         for hint in [
             "j/k move",
+            "shift+enter another",
             "yy p duplicate",
             "dd delete",
             "/ filter",
@@ -3888,11 +3889,22 @@
             );
         }
         let mac = painted(Look::macos());
-        for hint in ["↩ connect", "⌘E edit", "⌘D duplicate", "⌘⌫ delete"] {
+        for hint in [
+            "↩ connect",
+            "⇧↩ open another",
+            "⌘E edit",
+            "⌘D duplicate",
+            "⌘⌫ delete",
+        ] {
             assert!(mac.iter().any(|text| text == hint), "{hint}: {mac:?}");
         }
         let standard = painted(Look::standard());
-        for hint in ["Enter connect", "Ctrl+E edit", "Ctrl+Backspace delete"] {
+        for hint in [
+            "Enter connect",
+            "Shift+Enter open another",
+            "Ctrl+E edit",
+            "Ctrl+Backspace delete",
+        ] {
             assert!(
                 standard.iter().any(|text| text == hint),
                 "{hint}: {standard:?}"
```

- [ ] **Step 2: Run it to see it fail**

Run: `~/.cargo/bin/cargo test --locked -p tabletist --lib the_pickers_footer_names_the_keys_the_keymap_has`

Expected: it fails, no footer naming the key:

```
test ui::tests::the_pickers_footer_names_the_keys_the_keymap_has ... FAILED
thread 'ui::tests::the_pickers_footer_names_the_keys_the_keymap_has' panicked at src/ui/mod.rs:3886:13:
shift+enter another: ["connections", "1", "+ new", "n", "filter connections", "", "/", "j/k move", "enter connect / show", "e edit", "n new", "yy p duplicate", "dd delete", "/ filter", "a agent access", "secrets: keyring", "▾", "Local", "1", "▌", "PROD", "Bookshop", "Bookshop.db · local · no ssh · agents off", "never"]
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 3093 filtered out; finished in …
```

- [ ] **Step 3: Write the code**

```diff
diff --git a/src/ui/picker.rs b/src/ui/picker.rs
--- a/src/ui/picker.rs
+++ b/src/ui/picker.rs
@@ -481,6 +481,7 @@
             let words = [
                 msgid("move"),
                 msgid("connect / show"),
+                pmsgid("key hint", "another"),
                 msgid("edit"),
                 msgid("new"),
                 msgid("duplicate"),
@@ -492,15 +493,16 @@
             let mut hints = vec![
                 (&moves, &*words[0], true),
                 (&connect, &*words[1], true),
-                (&edit, &*words[2], true),
-                (&new_key, &*words[3], true),
-                (&duplicate, &*words[4], true),
-                (&delete, &*words[5], true),
-                (&filter, &*words[6], true),
+                (&again, &*words[2], true),
+                (&edit, &*words[3], true),
+                (&new_key, &*words[4], true),
+                (&duplicate, &*words[5], true),
+                (&delete, &*words[6], true),
+                (&filter, &*words[7], true),
             ];
             // Where agents can reach the app.
             if crate::endpoint::AVAILABLE {
-                hints.push((&agent_access, &*words[7], true));
+                hints.push((&agent_access, &*words[8], true));
             }
             // What does not fit beside the keyring's note gives way, from the
             // end: the window may be as narrow as 720.
@@ -524,6 +526,7 @@
         } else {
             let hints = [
                 format!("{connect} connect"),
+                format!("{again} open another"),
                 format!("{edit} edit"),
                 format!("{duplicate} duplicate"),
                 format!("{delete} delete"),
diff --git a/assets/i18n/tabletist.pot b/assets/i18n/tabletist.pot
--- a/assets/i18n/tabletist.pot
+++ b/assets/i18n/tabletist.pot
@@ -8,7 +8,7 @@
 msgstr ""
 "Project-Id-Version: Tabletist\n"
 "Report-Msgid-Bugs-To: https://github.com/igor-alexandrov/tabletist/issues\n"
-"POT-Creation-Date: 2026-11-22 21:36-0500\n"
+"POT-Creation-Date: 2026-11-22 21:38-0500\n"
 "PO-Revision-Date: YEAR-MO-DA HO:MI+ZONE\n"
 "Last-Translator: FULL NAME <EMAIL@ADDRESS>\n"
 "Language-Team: LANGUAGE <LL@li.org>\n"
@@ -6907,6 +6907,11 @@
 msgid "connect / show"
 msgstr ""
 
+#: src/ui/picker.rs
+msgctxt "key hint"
+msgid "another"
+msgstr ""
+
 #: src/ui/picker.rs src/ui/row_panel.rs
 msgid "duplicate"
 msgstr ""
diff --git a/assets/i18n/ru.po b/assets/i18n/ru.po
--- a/assets/i18n/ru.po
+++ b/assets/i18n/ru.po
@@ -7,7 +7,7 @@
 msgstr ""
 "Project-Id-Version: Tabletist\n"
 "Report-Msgid-Bugs-To: https://github.com/igor-alexandrov/tabletist/issues\n"
-"POT-Creation-Date: 2026-11-22 21:36-0500\n"
+"POT-Creation-Date: 2026-11-22 21:38-0500\n"
 "PO-Revision-Date: 2026-11-17 22:38-0500\n"
 "Last-Translator: Automatically generated\n"
 "Language-Team: none\n"
@@ -7188,6 +7188,11 @@
 msgstr "агенты: {level}"
 
 #: src/ui/picker.rs
+msgctxt "key hint"
+msgid "another"
+msgstr "ещё окно"
+
+#: src/ui/picker.rs
 msgid "connect / show"
 msgstr "подключиться / показать"
 
diff --git a/assets/i18n/pt-BR.po b/assets/i18n/pt-BR.po
--- a/assets/i18n/pt-BR.po
+++ b/assets/i18n/pt-BR.po
@@ -7,7 +7,7 @@
 msgstr ""
 "Project-Id-Version: Tabletist\n"
 "Report-Msgid-Bugs-To: https://github.com/igor-alexandrov/tabletist/issues\n"
-"POT-Creation-Date: 2026-11-22 21:36-0500\n"
+"POT-Creation-Date: 2026-11-22 21:38-0500\n"
 "PO-Revision-Date: 2026-11-19 16:11-0500\n"
 "Last-Translator: Automatically generated\n"
 "Language-Team: none\n"
@@ -7122,6 +7122,11 @@
 msgstr "agentes: {level}"
 
 #: src/ui/picker.rs
+msgctxt "key hint"
+msgid "another"
+msgstr "outra janela"
+
+#: src/ui/picker.rs
 msgid "connect / show"
 msgstr "conectar / mostrar"
 
```

- [ ] **Step 4: Run the tests, then the checks**

Run: `~/.cargo/bin/cargo test --locked -p tabletist --lib -- the_pickers_footer two_databases_tests agents_tests::the_agents_column`

Expected: they pass; the last is the Omarchy footer at 720 wide, whose hints still stay clear of the keyring's note. Then every check under Checks passes; the app's library ends `test result: ok. 3078 passed; 0 failed; 16 ignored; 0 measured; 0 filtered out; finished in …`.

- [ ] **Step 5: Try it**

By eye, in a throwaway scene in `src/shots.rs` (never committed, nor its pictures): the Bookshop open in a window, the picker with its row right-clicked and the pointer on Open another, in the three looks, light and dark:

```bash
~/.cargo/bin/cargo test --locked --features shots --lib shots::<the scene> -- --ignored
```

The PNGs land in `target/shots/`. The lit row reads "Open another" with the key at its right, and the footer has the new hint after connect, as `MacTwoDatabases` and `OmarchyTwoDatabases` draw them. Take the scene out and delete the pictures.

- [ ] **Step 6: Commit**

```bash
git add src/ui/picker.rs src/ui/mod.rs assets/i18n
git commit -m "Name Shift+Enter in the picker's footer as opening another window (#366)"
```

---

### Task 3: The docs

**Files:**
- Modify: `docs/_reference/connections.md`, `docs/_reference/keyboard-shortcuts.md`, `docs/_guide/macos.md`, `docs/_guide/omarchy.md`

- [ ] **Step 1: Write them**

```diff
diff --git a/docs/_reference/connections.md b/docs/_reference/connections.md
--- a/docs/_reference/connections.md
+++ b/docs/_reference/connections.md
@@ -23,7 +23,8 @@
 written in another case is the same group.
 
 A connection can be open more than once. Enter shows a connection that is
-already open, and Shift+Enter opens it again.
+already open, and Shift+Enter, or **Open another** in its row's menu,
+opens one more window of it, on its saved database.
 
 Cmd/Ctrl+Backspace (`dd` in the Omarchy look), **Delete** in a row's menu,
 or **Delete** in the connection's form asks at the bottom of the picker
diff --git a/docs/_reference/keyboard-shortcuts.md b/docs/_reference/keyboard-shortcuts.md
--- a/docs/_reference/keyboard-shortcuts.md
+++ b/docs/_reference/keyboard-shortcuts.md
@@ -68,7 +68,7 @@
 | Keys | Do |
 | --- | --- |
 | Arrows, Enter | Pick a connection. Mod+Up and Mod+Down go to the first and the last |
-| Shift+Enter | Open it again |
+| Shift+Enter | Open one more window of it |
 | Mod+N | New connection |
 | Mod+E | Edit |
 | Mod+D | Duplicate |
diff --git a/docs/_guide/macos.md b/docs/_guide/macos.md
--- a/docs/_guide/macos.md
+++ b/docs/_guide/macos.md
@@ -161,7 +161,7 @@
 | Arrows | Move |
 | Cmd+Up, Cmd+Down | The first and the last connection |
 | Return | Connect, or show the connection if it is open already |
-| Shift+Return | Open it again |
+| Shift+Return | Open one more window of it |
 | Cmd+N | New connection |
 | Cmd+E | Edit |
 | Cmd+D | Duplicate |
diff --git a/docs/_guide/omarchy.md b/docs/_guide/omarchy.md
--- a/docs/_guide/omarchy.md
+++ b/docs/_guide/omarchy.md
@@ -79,7 +79,7 @@
 | --- | --- |
 | `j` `k`, arrows | Move |
 | Enter | Connect, or show the connection if it is open already |
-| Shift+Enter | Open it again |
+| Shift+Enter | Open one more window of it |
 | `n` | New connection |
 | `e` | Edit |
 | `yy` then `p` | Duplicate |
```

- [ ] **Step 2: Check the site**

Run: `cd docs && bundle install && bin/check`

Expected: the site builds and its checks pass.

- [ ] **Step 3: Commit**

```bash
git add docs/_reference docs/_guide
git commit -m "Say that Shift+Enter and Open another open one more window (#366)"
```

---

## After the run

- **The canvas:** the picker's own boards get the footer's new hint with the code: `MacConnections` ("⇧↩ open another" after "↩ connect") and `OmarchyConnections` ("shift+enter another" after "enter connect / go to window"). `MacTwoDatabases` and `OmarchyTwoDatabases` already draw run 3. The canvas is an Artifact and is never committed; re-read its `canvas.json` before publishing.
- Open the PR for the three commits with "Closes #366": this is the last run.
