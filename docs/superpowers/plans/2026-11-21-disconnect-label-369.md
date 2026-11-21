# Close, not Disconnect, for a connection that is down (#369) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use rubypowers:subagent-driven-development (recommended) or rubypowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** When a connection is lost, cancelled or closed while idle, the
button at the connection bar's end says **Close** (it closes the tab, as
Disconnect does now); it says **Disconnect** while the connection is
connecting or connected. Fixes igor-alexandrov/tabletist#369 ("It says
disconnected, but right says disconnect").

**Architecture:** `bar_info` (`src/ui/workspace.rs`) gains `closes: bool`,
true unless the session is `Connected` or `Connecting`. Both bar builders
read it: macOS and Windows' `mac_bar` swaps the label and icon, Omarchy's
`terminal_bar` swaps the note's word and the button's name. Both still push
`Action::Disconnect(tab)`, so what a click does is unchanged. Task 2 does
the same for the lost-connection strip (`banner`). One run, one PR.

**Tech Stack:** Rust, egui, the headless `Harness` (`src/testing.rs`),
gettext catalogs in `assets/i18n/`.

---

## What the design draws, and what this builds

The canvas (re-read for this plan) draws the macOS connection bar only
while connected: the log-in icon and **Disconnect** at its end (Main,
MacTwoConnections). The Omarchy boards (Omarchy, OmarchyTwoConnections)
draw no way out at the bar's end; the app's note (the Close connection
key, then "disconnect") is its own. The "Connection lost mid-session" board (MacErrors 6) draws the strip
while it reconnects: **Reconnect now** and **Disconnect**. No board draws a
bar or a strip of a connection that is down, so the states below are new,
and each is put to Igor for a yes:

| Where | The design draws | This builds | Why |
| --- | --- | --- | --- |
| Bar's button, connection down (macOS and Windows looks) | nothing for this state; connected: log-in icon + Disconnect | **x icon** + **Close** | the issue's word; the x is the app's close sign already (the export toast's close, a filter chip's remove), where the log-in arrow says "leave a session" |
| Omarchy note, connection down | no note in any state (the app has `<key> disconnect`) | `<key> close` | the same word, same key (`CloseConnection`) |
| Lost-connection strip, lost or cancelled (Task 2) | nothing; reconnecting: Reconnect now / Disconnect | **Reconnect** / **Close** | otherwise the strip under the bar still says Disconnect for a connection that is gone; while it reconnects it keeps Disconnect, as drawn |

No new message: `Close` («Закрыть», «Fechar») and `close` («закрыть»,
«fechar») are in all three catalogs already. `assets/i18n/update.sh` only
adds `src/ui/workspace.rs` to `Close`'s references (and moves the
POT-Creation-Date).

Out of scope: what a click does (it is `Action::Disconnect` as now), the
chips (#366 is changing them), Esc on a failed connect (#188).

## Files

- Modify `src/ui/workspace.rs`: `BarInfo.closes`, `bar_info`, `mac_bar`,
  `terminal_bar` (Task 1); `banner` (Task 2).
- Modify `src/ui/mod.rs` tests: two new bar tests and a helper (Task 1);
  one new strip test (Task 2); the two lost-connection tests that count
  the bar's and the strip's buttons (both tasks).
- Modify `assets/i18n/tabletist.pot`, `ru.po`, `pt-BR.po` (written by
  `update.sh`, Task 1).

### Task 1: The bar says Close while its connection is down

**Files:**
- Modify: `src/ui/workspace.rs` (`BarInfo`, `bar_info`, `mac_bar`, `terminal_bar`)
- Test: `src/ui/mod.rs` (new `bar_button`, `bar_says`,
  `the_bar_says_close_while_its_connection_is_down`,
  `the_bars_close_closes_the_tab_of_a_lost_connection`; changed
  `the_lost_strip_keeps_its_buttons_inside_the_window`,
  `a_lost_connection_can_be_left`)
- Modify: `assets/i18n/tabletist.pot`, `ru.po`, `pt-BR.po` (by `update.sh`)

The two new tests run in every look (`Look::ALL`: Windows, macOS, Omarchy). `bar_button` finds the bar's own button by its row (the
Connections button's), so a strip's button of the same name never answers
for it. In Omarchy `bar_says` also reads the painted note, `<key> close` or
`<key> disconnect`, with the key from the keymap. The two lost-connection
tests counted two Disconnect buttons (the bar's and the strip's); after
this task the strip's is the only one.

- [ ] **Step 1: Write the failing tests**

```diff
diff --git a/src/ui/mod.rs b/src/ui/mod.rs
--- a/src/ui/mod.rs
+++ b/src/ui/mod.rs
@@ -767,11 +767,11 @@
                 }
                 let tree = harness.settle();
                 let window = egui::Rect::from_min_size(egui::Pos2::ZERO, harness.size);
-                // The connection bar has a Disconnect of its own: the
-                // strip's is the lower one.
+                // The connection bar's button says Close: the strip's
+                // Disconnect is the only one.
                 let reconnect = buttons_named(&tree, "Reconnect");
                 let disconnect = buttons_named(&tree, "Disconnect");
-                assert_eq!((reconnect.len(), disconnect.len()), (1, 2), "{}", look.name);
+                assert_eq!((reconnect.len(), disconnect.len()), (1, 1), "{}", look.name);
                 if long {
                     // The error is there in full, before the buttons.
                     let said = error.to_string();
@@ -780,7 +780,7 @@
                     assert!(text.right() <= reconnect[0].1.left(), "{}", look.name);
                 }
                 for (name, (_, button)) in
-                    [("Reconnect", reconnect[0]), ("Disconnect", disconnect[1])]
+                    [("Reconnect", reconnect[0]), ("Disconnect", disconnect[0])]
                 {
                     assert!(
                         window.contains_rect(button),
@@ -4507,6 +4507,105 @@
             ));
             assert!(harness.has(&look.label("Disconnected")), "{}", look.name);
             assert!(!harness.has("fixture.db"), "{}", look.name);
+        }
+    }
+
+    /// The button at the connection bar's end named `word`, if it is:
+    /// the one on the Connections button's row, apart from a
+    /// lost-connection strip's.
+    fn bar_button(
+        tree: &egui::accesskit::TreeUpdate,
+        word: &str,
+    ) -> Option<egui::accesskit::NodeId> {
+        let row = crate::testing::bounds(tree, "Connections", egui::accesskit::Role::Button)
+            .expect("the Connections button")
+            .center()
+            .y;
+        buttons_named(tree, word)
+            .into_iter()
+            .find(|(_, button)| (button.center().y - row).abs() < 2.0)
+            .map(|(id, _)| id)
+    }
+
+    /// Whether the button at the connection bar's end is `word` ("Close"
+    /// or "Disconnect"), as is the terminal look's note.
+    fn bar_says(harness: &mut Harness, word: &str) -> bool {
+        let tree = harness.settle();
+        let named = bar_button(&tree, word).is_some();
+        if !harness.app.look.terminal {
+            return named;
+        }
+        let key = harness.app.keymap.key(
+            harness.app.layout(),
+            crate::keymap::Command::CloseConnection,
+        );
+        let note = format!("{key} {}", word.to_lowercase());
+        named && harness.painted_rect(&note).is_some()
+    }
+
+    #[test]
+    fn the_bar_says_close_while_its_connection_is_down() {
+        use crate::model::SessionStatus;
+        for look in crate::theme::Look::ALL {
+            let mut harness = Harness::new();
+            harness.set_look(look);
+            let tab = harness.connect_fake();
+            // Connected, there is a session to end.
+            assert!(bar_says(&mut harness, "Disconnect"), "{}", look.name);
+            assert!(!bar_says(&mut harness, "Close"), "{}", look.name);
+            let session = harness.app.workspace(tab).unwrap().session;
+            harness.app.apply(crate::model::Action::Backend(
+                crate::backend::Event::Disconnected {
+                    session,
+                    error: tabletist_db::Error::ConnectionLost("server went away".into()),
+                },
+            ));
+            // Lost, cancelled or closed while idle, it closes the tab,
+            // and says so.
+            assert!(bar_says(&mut harness, "Close"), "lost in {}", look.name);
+            for (name, status) in [
+                ("cancelled", SessionStatus::Cancelled),
+                ("idle", SessionStatus::Idle),
+            ] {
+                harness.app.workspace_mut(tab).unwrap().status = status;
+                assert!(bar_says(&mut harness, "Close"), "{name} in {}", look.name);
+                assert!(!bar_says(&mut harness, "Disconnect"), "{}", look.name);
+            }
+            // Connecting again, it disconnects again.
+            harness.app.workspace_mut(tab).unwrap().status = SessionStatus::Connecting {
+                request: crate::backend::RequestId(0),
+            };
+            assert!(bar_says(&mut harness, "Disconnect"), "{}", look.name);
+        }
+    }
+
+    #[test]
+    fn the_bars_close_closes_the_tab_of_a_lost_connection() {
+        use egui::accesskit;
+        for look in crate::theme::Look::ALL {
+            let mut harness = Harness::new();
+            harness.set_look(look);
+            let tab = lost(&mut harness);
+            let tree = harness.settle();
+            let close = bar_button(&tree, "Close").expect("the bar's Close");
+            harness.frame(vec![egui::Event::AccessKitActionRequest(
+                accesskit::ActionRequest {
+                    target_tree: accesskit::TreeId::ROOT,
+                    target_node: close,
+                    action: accesskit::Action::Click,
+                    data: None,
+                },
+            )]);
+            harness.settle();
+            assert!(harness.app.workspace(tab).is_none(), "{}", look.name);
+            assert!(
+                matches!(
+                    harness.app.active_tab().content,
+                    crate::model::ConnTabContent::Picker(_)
+                ),
+                "{}",
+                look.name
+            );
         }
     }
 
@@ -12004,15 +12103,15 @@
             let mut harness = Harness::new();
             harness.set_look(look);
             let tab = lost(&mut harness);
-            // The connection bar has a Disconnect of its own: the strip's
-            // is the lower one.
+            // The connection bar's button says Close: the strip's
+            // Disconnect is the only one.
             let tree = harness.settle();
             let buttons = buttons_named(&tree, "Disconnect");
-            assert_eq!(buttons.len(), 2, "{}", look.name);
+            assert_eq!(buttons.len(), 1, "{}", look.name);
             harness.frame(vec![egui::Event::AccessKitActionRequest(
                 accesskit::ActionRequest {
                     target_tree: accesskit::TreeId::ROOT,
-                    target_node: buttons[1].0,
+                    target_node: buttons[0].0,
                     action: accesskit::Action::Click,
                     data: None,
                 },
```

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test --locked -p tabletist --lib -- ui::tests::the_bar ui::tests::a_lost_connection ui::tests::the_lost_strip ui::tests::every_look ui::tests::the_connections_button ui::tests::many_chips`

Expected: `13 passed; 4 failed`:
`the_bar_says_close_while_its_connection_is_down` (the bar says
Disconnect while lost), `the_bars_close_closes_the_tab_of_a_lost_connection`
("the bar's Close" is missing), and
`the_lost_strip_keeps_its_buttons_inside_the_window` and
`a_lost_connection_can_be_left` (two Disconnect buttons, one expected).

- [ ] **Step 3: Build it**

```diff
diff --git a/src/ui/workspace.rs b/src/ui/workspace.rs
--- a/src/ui/workspace.rs
+++ b/src/ui/workspace.rs
@@ -836,6 +836,10 @@
     /// one, as the look writes them.
     connections_key: crate::keymap::Written,
     close_key: crate::keymap::Written,
+    /// Whether the connection is down (lost, cancelled, closed while
+    /// idle): the button at the bar's end says Close then, as it closes
+    /// the tab, and Disconnect while there is a session to end.
+    closes: bool,
 }
 
 /// How a status reads: fine, or a warning.
@@ -916,6 +920,10 @@
         chips: chips(app, tab),
         connections_key: app.keymap.key(app.layout(), Command::Connections),
         close_key: app.keymap.key(app.layout(), Command::CloseConnection),
+        closes: !matches!(
+            workspace.status,
+            SessionStatus::Connected | SessionStatus::Connecting { .. }
+        ),
     })
 }
 
@@ -1435,7 +1443,13 @@
         }
     }
     // Disconnect, 12 in from the right: an icon and its label, 6 apart.
-    let label = gettext(locale, "Disconnect");
+    // A connection that is down has no session to end: its button closes
+    // the tab, and says so.
+    let (label, icon) = if info.closes {
+        (gettext(locale, "Close"), Icon::X)
+    } else {
+        (gettext(locale, "Disconnect"), Icon::LogIn)
+    };
     let text = Text::one(look, TextRole::UiBody, &label, palette.secondary).layout(ui.ctx());
     let width = 10.0 + 14.0 + 6.0 + text.width() + 10.0;
     let button = Rect::from_min_size(
@@ -1448,7 +1462,7 @@
         ui.painter()
             .rect_filled(button, CornerRadius::same(6), face(0.7));
     }
-    Icon::LogIn.image(palette.secondary, 14.0).paint_at(
+    icon.image(palette.secondary, 14.0).paint_at(
         ui,
         Rect::from_center_size(pos2(button.left() + 17.0, center), vec2(14.0, 14.0)),
     );
@@ -1704,8 +1718,14 @@
         }
         x = button.right() + 12.0;
     }
-    // The way out, a muted note that also answers a click.
-    let note = format!("{} {}", info.close_key, gettext(locale, "disconnect"));
+    // The way out, a muted note that also answers a click: it closes the
+    // tab of a connection that is down, and says so.
+    let (word, label) = if info.closes {
+        (msgid("close"), msgid("Close"))
+    } else {
+        (msgid("disconnect"), msgid("Disconnect"))
+    };
+    let note = format!("{} {}", info.close_key, word.say(locale));
     let note = note.as_str();
     let width = widgets::measure(ui, label_copy(look, TextRole::OSecondary, note));
     let left = rect.right() - 12.0 - width;
@@ -1714,7 +1734,7 @@
         vec2(width + 8.0, rect.height()),
     );
     let response = ui.interact(hit, ui.id().with("disconnect"), Sense::click());
-    let label = gettext(locale, "Disconnect");
+    let label = label.say(locale);
     response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, &label));
     let color = if response.hovered() {
         palette.text
```

The note's word and the button's name are `msgid`s drawn with `.say`,
since they are chosen before they are drawn (AGENTS.md).

- [ ] **Step 4: Update the catalogs**

Run: `assets/i18n/update.sh`

Expected: no new message (each catalog still says `2379 translated
messages`); the template and both catalogs gain one reference and a new
POT-Creation-Date:

```diff
diff --git a/assets/i18n/tabletist.pot b/assets/i18n/tabletist.pot
--- a/assets/i18n/tabletist.pot
+++ b/assets/i18n/tabletist.pot
@@ -8,7 +8,7 @@
 msgstr ""
 "Project-Id-Version: Tabletist\n"
 "Report-Msgid-Bugs-To: https://github.com/igor-alexandrov/tabletist/issues\n"
-"POT-Creation-Date: 2026-11-19 23:02-0500\n"
+"POT-Creation-Date: 2026-11-21 09:45-0500\n"
 "PO-Revision-Date: YEAR-MO-DA HO:MI+ZONE\n"
 "Last-Translator: FULL NAME <EMAIL@ADDRESS>\n"
 "Language-Team: LANGUAGE <LL@li.org>\n"
@@ -2810,7 +2810,7 @@
 msgstr ""
 
 #: src/ui/about.rs src/ui/connect_dialog/sheet.rs src/ui/help.rs
-#: src/ui/object_tabs.rs src/ui/settings/terminal.rs
+#: src/ui/object_tabs.rs src/ui/settings/terminal.rs src/ui/workspace.rs
 msgid "Close"
 msgstr ""
 
```

(`ru.po` and `pt-BR.po` change the same way: the reference line and the
date.)

- [ ] **Step 5: Run the tests again**

Run: the command of Step 2.
Expected: `17 passed; 0 failed`.

- [ ] **Step 6: Commit**

```bash
git add src/ui/workspace.rs src/ui/mod.rs assets/i18n/tabletist.pot assets/i18n/ru.po assets/i18n/pt-BR.po
git commit -S -m "Say Close, not Disconnect, at the bar's end while its connection is down"
```

### Task 2: The lost-connection strip says Close too

Only with Igor's yes to the third row of the departures table; without it
the strip keeps Disconnect and this task is left out.

**Files:**
- Modify: `src/ui/workspace.rs` (`banner`)
- Test: `src/ui/mod.rs` (new
  `the_strips_way_out_says_close_while_lost_and_disconnect_while_it_reconnects`;
  `the_lost_strip_keeps_its_buttons_inside_the_window` and
  `a_lost_connection_can_be_left` count two Close buttons, the strip's the
  lower)

- [ ] **Step 1: Write the failing tests**

```diff
diff --git a/src/ui/mod.rs b/src/ui/mod.rs
--- a/src/ui/mod.rs
+++ b/src/ui/mod.rs
@@ -767,11 +767,11 @@
                 }
                 let tree = harness.settle();
                 let window = egui::Rect::from_min_size(egui::Pos2::ZERO, harness.size);
-                // The connection bar's button says Close: the strip's
-                // Disconnect is the only one.
+                // The connection bar has a Close of its own: the strip's
+                // is the lower one.
                 let reconnect = buttons_named(&tree, "Reconnect");
-                let disconnect = buttons_named(&tree, "Disconnect");
-                assert_eq!((reconnect.len(), disconnect.len()), (1, 1), "{}", look.name);
+                let close = buttons_named(&tree, "Close");
+                assert_eq!((reconnect.len(), close.len()), (1, 2), "{}", look.name);
                 if long {
                     // The error is there in full, before the buttons.
                     let said = error.to_string();
@@ -779,9 +779,7 @@
                     let text = text.expect("the exact error");
                     assert!(text.right() <= reconnect[0].1.left(), "{}", look.name);
                 }
-                for (name, (_, button)) in
-                    [("Reconnect", reconnect[0]), ("Disconnect", disconnect[0])]
-                {
+                for (name, (_, button)) in [("Reconnect", reconnect[0]), ("Close", close[1])] {
                     assert!(
                         window.contains_rect(button),
                         "{name} at {button:?} in {} (long error: {long})",
@@ -12097,21 +12095,44 @@
     }
 
     #[test]
+    fn the_strips_way_out_says_close_while_lost_and_disconnect_while_it_reconnects() {
+        for look in crate::theme::Look::ALL {
+            let mut harness = Harness::new();
+            harness.set_look(look);
+            lost(&mut harness);
+            // Lost, the strip's way out closes the tab, as the bar's does.
+            let tree = harness.settle();
+            assert_eq!(buttons_named(&tree, "Close").len(), 2, "{}", look.name);
+            assert!(
+                buttons_named(&tree, "Disconnect").is_empty(),
+                "{}",
+                look.name
+            );
+            // Reconnecting, there is a session to end again: both say
+            // Disconnect.
+            harness.click("Reconnect");
+            let tree = harness.settle();
+            assert_eq!(buttons_named(&tree, "Disconnect").len(), 2, "{}", look.name);
+            assert!(buttons_named(&tree, "Close").is_empty(), "{}", look.name);
+        }
+    }
+
+    #[test]
     fn a_lost_connection_can_be_left() {
         use egui::accesskit;
         for look in crate::theme::Look::ALL {
             let mut harness = Harness::new();
             harness.set_look(look);
             let tab = lost(&mut harness);
-            // The connection bar's button says Close: the strip's
-            // Disconnect is the only one.
+            // The connection bar has a Close of its own: the strip's is
+            // the lower one.
             let tree = harness.settle();
-            let buttons = buttons_named(&tree, "Disconnect");
-            assert_eq!(buttons.len(), 1, "{}", look.name);
+            let buttons = buttons_named(&tree, "Close");
+            assert_eq!(buttons.len(), 2, "{}", look.name);
             harness.frame(vec![egui::Event::AccessKitActionRequest(
                 accesskit::ActionRequest {
                     target_tree: accesskit::TreeId::ROOT,
-                    target_node: buttons[0].0,
+                    target_node: buttons[1].0,
                     action: accesskit::Action::Click,
                     data: None,
                 },
```

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test --locked -p tabletist --lib -- ui::tests::the_bar ui::tests::a_lost_connection ui::tests::the_strips ui::tests::the_lost_strip ui::tests::every_look ui::tests::the_connections_button ui::tests::many_chips`

Expected: `17 passed; 3 failed`: the new strip test (one Close, two
expected), `the_lost_strip_keeps_its_buttons_inside_the_window` and
`a_lost_connection_can_be_left` (likewise).

- [ ] **Step 3: Build it**

```diff
diff --git a/src/ui/workspace.rs b/src/ui/workspace.rs
--- a/src/ui/workspace.rs
+++ b/src/ui/workspace.rs
@@ -3966,8 +3966,9 @@
 }
 
 /// The strip over a tab whose connection was lost: what happened, the way
-/// back (Reconnect) and the way out (Disconnect). What was on screen stays
-/// under it. While a reconnect is on its way the strip says so.
+/// back (Reconnect) and the way out (Close, or Disconnect while it
+/// reconnects). What was on screen stays under it. While a reconnect is
+/// on its way the strip says so.
 fn banner(
     ui: &mut egui::Ui,
     paint: super::Paint,
@@ -4035,11 +4036,15 @@
             ui.horizontal(|ui| {
                 ui.spacing_mut().item_spacing.x = 12.0;
                 let height = if look.terminal { 24.0 } else { 28.0 };
-                let (out, back) = (say(msgid("Disconnect")), say(msgid("Reconnect")));
-                let leave = states::button(&out, &look)
-                    .label("Disconnect")
-                    .salt("lost")
-                    .quiet();
+                // Reconnecting, there is a session to end; lost or
+                // cancelled, the way out closes the tab, as the bar's does.
+                let (out, name) = if reconnecting {
+                    (msgid("Disconnect"), "Disconnect")
+                } else {
+                    (msgid("Close"), "Close")
+                };
+                let (out, back) = (say(out), say(msgid("Reconnect")));
+                let leave = states::button(&out, &look).label(name).salt("lost").quiet();
                 let again =
                     (!reconnecting).then(|| states::button(&back, &look).label("Reconnect"));
                 // The buttons keep their room at the right, 8 apart and 12
```

No catalog changes: `Close` is referenced from `workspace.rs` already
(Task 1). `assets/i18n/update.sh --check` passes.

- [ ] **Step 4: Run the tests again**

Run: the command of Step 2.
Expected: `20 passed; 0 failed`.

- [ ] **Step 5: Commit**

```bash
git add src/ui/workspace.rs src/ui/mod.rs
git commit -S -m "Say Close in the lost-connection strip while there is no session to end"
```

## Done

- [ ] `cargo fmt --all --check`
- [ ] `cargo clippy --locked --workspace --all-targets -- -D warnings`
- [ ] `cargo test --locked --workspace --all-targets` (the translation
      tests among them: every catalog entry translated)
- [ ] `RUSTDOCFLAGS='-D warnings' cargo doc --locked --workspace --no-deps`
- [ ] `assets/i18n/update.sh --check`
- [ ] Signed commits (the plan, then one per task), a PR that says
      "Closes #369" and lists the three departures above.
