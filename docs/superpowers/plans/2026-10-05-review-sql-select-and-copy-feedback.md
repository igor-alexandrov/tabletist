# Selectable Review SQL and Copy Feedback Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** The lines of Review SQL can be selected and copied, a button shows that it is held down, and Copy SQL says that it copied.

**Architecture:** A review's line becomes text that egui's label selection handles, named by its place in the review so a selection survives scrolling. `ButtonSpec` gains the Pressed fill the design's Components give a secondary and a quiet button. A new view module, `src/ui/toast.rs`, keeps one line and the time it was said in egui's frame data: macOS and Windows float it over the bottom of the window, the terminal look says it in its status line.

**Tech Stack:** Rust, egui (the crmne fork of 0.36: `egui::text_selection::LabelSelectionState`), the headless UI harness in `src/testing.rs`.

---

## Before you start

- **All three tasks were built as a draft and checked** before this plan was written, then taken out of the tree again. The diffs below are that draft, one commit per task, each applying cleanly on the one before. The tree after task 1 alone and the tree after all three each passed `cargo fmt --all --check`, `cargo clippy --locked --workspace --all-targets -- -D warnings` and the tests (1679 and 1683 of the app's library; the whole workspace after task 3); `cargo doc` with `-D warnings` was run on the whole. Base: `16d0fc1` (`main`).
- **Not checked:** nothing was looked at in a window (these sessions have no display). The by-hand checks are listed at the end, for the product owner.
- Cargo is `~/.cargo/bin/cargo`. Never a cargo target dir under `/tmp`.
- House rules (`AGENTS.md`): no em dashes; comments say why, in the surrounding code's voice; a view never names a font and never paints a focus ring; every user-visible string goes through `gettext`; a regression test for every behaviour change.
- The design is the source of truth for how it looks. It is not in the repository and is never copied into it; what this plan takes from its Components artboards is written out below. No test compares a screen with the design.
- Apply a diff with `git apply` from the repository root (paths have no `a/` prefix: `git apply -p0`), or make the edits by hand. Where the code moved since `16d0fc1`, the code wins: say so in your report.
- Commit after every task. Subjects are plain sentences, each ending with:

      Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>

## What the design's Components say

- **Buttons** have six states: Default, Hover, Pressed, Focused, Disabled, Loading. A Secondary button is filled a step darker held down than under the pointer (light: white, then a warm off-white, then a warmer grey; dark: the same steps lighter). Quiet has no border and the same three steps. Omarchy's button held down has the selection's fill inside a 1 px accent line.
- **Toast:** bottom centre, 4 s. One line ("Copied 3 rows as TSV") in the window's colour on the text colour, 12.5 px, padding 8 by 12, radius 9, a soft shadow below.
- **Omarchy** has no toast: "in-app: a one-line message in the status line for 4 s".

## What this plan decides

1. **A selection is copied as it reads, a cut value cut.** The spec had the lines unselectable for this reason: a value over 60 characters is shown as its beginning and `…`, and a pasted `'aaa…'` would be stored as it is. This plan reverses that rule, as asked. Copy SQL stays the way to the whole statements. egui's label selection gives no hook to put the whole value in place of the cut one, and a text field over the whole review would lay out every line of a large set, take the keyboard from the grid, and still need a map from shown to whole text. **This is the decision to confirm before execution.**
2. **The same lines are selectable in the production confirmation.** It draws them with the same function.
3. **A line is named by its place in the review.** egui keys a selection to a widget id. Rows in a `show_rows` scroll area get ids by their place in view, so the selection would jump to another line on scroll.
4. **A selection whose ends scroll out of view is dropped.** That is egui's own rule for a selection it cannot see both ends of.
5. **With a selection in the lines, `Mod+C` copies the selection even while the grid has the keyboard.** The grid's own copy runs first and egui's replaces it at the end of the frame, as it already does for the row panel's JSON.
6. **The pressed fill goes to every secondary and quiet button,** not to Copy SQL alone: it is one drawing function, and the Components give the state to the kind. Pressed is `palette.surface` on macOS and Windows; on Omarchy `palette.selection` with the accent's line. A link button (accent text) gets the same fill as a quiet one. The design's Quiet is written in the accent colour; the app's quiet button stays in the secondary colour (the Review SQL plan's decision 25).
7. **Copy feedback is the toast, not a change of the button's name.** "Copied SQL", four seconds. The toast has no action ("Undo" in the artboard): a copy has nothing to undo. Its text is the body role (13 px, the nearest role to 12.5).
8. **The toast is the frame's own,** kept in egui's data as the panel's place and its page turns are: no `Action`, nothing in `App`. One toast at a time; a second takes the first's place.
9. **Omarchy says it in the status line's `said` slot,** after everything else that slot says (a lock, a save's outcome, a refused command). Only a table tab's line has that slot, and only a table tab has a review.
10. **The production sheet's Copy SQL says it too,** over the sheet.

## Files

- `src/typography.rs`: `Laid::select_left`, the selectable twin of `paint_left`.
- `src/ui/review.rs`: `rows` draws lines that can be selected; `COPIED_SQL`; the panel's copy says so.
- `src/testing.rs`: a frame of a widget on its own records what it copied.
- `src/ui/widgets.rs`: the pressed fills in `ButtonSpec::show_at`.
- `src/ui/toast.rs` (new): `say`, `said`, `show`.
- `src/ui/mod.rs`: the module, `toast::show` after the dialogs, the tests.
- `src/ui/keys.rs`, `src/ui/write_prompts.rs`: the other two places that copy a review.
- `src/ui/workspace.rs`: the status line says what the toast says.
- `docs/superpowers/specs/2026-10-03-value-editing-core-design.md`: the rules as built.

---

### Task 1: The lines of a review can be selected

**Files:**
- Modify: `src/typography.rs` (after `Laid::paint_left`)
- Modify: `src/ui/review.rs` (`rows`, and its tests)
- Modify: `src/testing.rs` (`Harness::frame_with_events`)
- Modify: `src/ui/write_prompts.rs` (the doc comment of `statements`)
- Modify: `docs/superpowers/specs/2026-10-03-value-editing-core-design.md`
- Test: `src/ui/review.rs`, `src/ui/mod.rs`

- [ ] **Step 1: Write the failing tests**

````diff
diff --git src/testing.rs src/testing.rs
--- src/testing.rs
+++ src/testing.rs
@@ -146,6 +146,11 @@ impl Harness {
         });
         output.textures_delta.clear();
         self.collect(&output);
+        for command in &output.platform_output.commands {
+            if let egui::OutputCommand::CopyText(text) = command {
+                self.copied = Some(text.clone());
+            }
+        }
         output
             .platform_output
             .accesskit_update
diff --git src/ui/mod.rs src/ui/mod.rs
--- src/ui/mod.rs
+++ src/ui/mod.rs
@@ -16883,6 +16883,43 @@ mod tests {
         }
     }
 
+    #[test]
+    fn the_reviews_lines_are_selected_and_copied_as_they_read() {
+        for look in Look::ALL {
+            let (mut harness, tab, id) = editable_in(look);
+            select(&mut harness, tab, id, (0, 1));
+            make_pending(&mut harness, tab, id, (1, 1), &"x".repeat(100));
+            review(&mut harness, tab, id);
+            // The statement's three lines: the one with the cut value is
+            // the second, and in a narrow panel runs past its edge.
+            let mut texts = harness.painted.iter().map(|(text, _)| text);
+            let set = texts
+                .find(|text| text.starts_with("   SET"))
+                .cloned()
+                .expect("the line that sets");
+            assert!(set.ends_with("x…'"), "{}: {set}", look.name);
+            let from = harness.painted_rect(BOB[2]).expect(BOB[2]).left_center();
+            let to = harness.painted_rect(BOB[4]).expect(BOB[4]).right_center();
+            harness.copied = None;
+            drag(&mut harness, from, to + egui::vec2(2.0, 0.0));
+            assert_eq!(
+                harness.copied, None,
+                "{}: selecting copies nothing",
+                look.name
+            );
+            harness.frame(vec![egui::Event::Copy]);
+            // The selection, not the grid's cell, though the grid has the
+            // keyboard; and a cut value as it is shown, with its `…`.
+            assert_eq!(
+                harness.copied.as_deref(),
+                Some(format!("{}\n{set}\n{}", BOB[2], BOB[4]).as_str()),
+                "{}",
+                look.name
+            );
+            assert!(painted(&harness, &set), "{}: the lines stay", look.name);
+        }
+    }
+
     #[test]
     fn the_lines_wear_the_sql_editors_colours() {
         for look in desktop_looks() {
diff --git src/ui/review.rs src/ui/review.rs
--- src/ui/review.rs
+++ src/ui/review.rs
@@ -803,6 +816,59 @@ mod tests {
         }
     }
 
+    #[test]
+    fn a_selection_stays_on_its_line_while_the_lines_scroll() {
+        let review = of(
+            Dialect::Sqlite,
+            &changes(&[2, 4], "bob@example.com"),
+            &[],
+            Values::Shown,
+        );
+        let lines = &review.lines;
+        let update = r#"UPDATE "main"."users""#;
+        for look in Look::ALL {
+            let mut harness = Harness::new();
+            harness.set_look(look);
+            let palette = harness.app.palette;
+            // Four of the ten lines in view, from the line `top`.
+            let draw = |top: f32| {
+                move |ui: &mut egui::Ui| {
+                    let height = row_height(ui.ctx(), &look);
+                    ui.spacing_mut().item_spacing = egui::Vec2::ZERO;
+                    egui::ScrollArea::both()
+                        .max_height(4.0 * height)
+                        .vertical_scroll_offset(top * height)
+                        .show_rows(ui, height, lines.len(), |ui, range| {
+                            rows(ui, lines, range, &look, &palette, Locale::English);
+                        });
+                }
+            };
+            harness.frame_with(draw(0.0));
+            harness.frame_with(draw(0.0));
+            // A double click selects the word under it: the third line's
+            // first.
+            let at = harness.painted_rect(update).expect(update).left_center() + vec2(10.0, 0.0);
+            let button = |pressed| egui::Event::PointerButton {
+                pos: at,
+                button: egui::PointerButton::Primary,
+                pressed,
+                modifiers: egui::Modifiers::NONE,
+            };
+            harness.frame_with_events(vec![egui::Event::PointerMoved(at)], draw(0.0));
+            for pressed in [true, false, true, false] {
+                harness.frame_with_events(vec![button(pressed)], draw(0.0));
+            }
+            harness.frame_with_events(vec![egui::Event::Copy], draw(0.0));
+            assert_eq!(harness.copied.as_deref(), Some("UPDATE"), "{}", look.name);
+            // Two lines up, the line is the first in view where it was the
+            // third: the selection is still its word, not the third row's.
+            harness.copied = None;
+            harness.frame_with(draw(2.0));
+            harness.frame_with_events(vec![egui::Event::Copy], draw(2.0));
+            assert_eq!(harness.copied.as_deref(), Some("UPDATE"), "{}", look.name);
+        }
+    }
+
     #[test]
     fn the_clipboards_text_says_what_it_is_and_holds_the_whole_statements() {
         let long = "x".repeat(100);
````

- [ ] **Step 2: Run them to see them fail**

Run: `~/.cargo/bin/cargo test --locked --lib -- a_selection_stays_on_its_line the_reviews_lines_are_selected`
Expected: both FAIL. Nothing in a line is selected, so the first copies nothing (`left: None`), and the second copies the grid's cell instead of the lines.

- [ ] **Step 3: Make the lines selectable, and say so in the spec**

````diff
diff --git docs/superpowers/specs/2026-10-03-value-editing-core-design.md docs/superpowers/specs/2026-10-03-value-editing-core-design.md
--- docs/superpowers/specs/2026-10-03-value-editing-core-design.md
+++ docs/superpowers/specs/2026-10-03-value-editing-core-design.md
@@ -858,9 +858,14 @@ confirmation.
   a NUL (`('aaa…')`, `('aaa'…)`). The value in a check's comment is cut
   the same way. That is display only: what runs, and what is copied, hold
   the whole value.
-- **The lines cannot be selected.** What is shown is cut, and a copy of it
-  would be pasted as it is. **Copy SQL** puts the whole statements on the
-  clipboard instead (see "What a copied text is").
+- **The lines can be selected,** as a label's text is: a drag selects
+  across lines, a double click a word, and `Mod+C` copies the selection.
+  It is copied as it reads: a value that is shown cut is copied cut, with
+  its `…`, and nothing says what the text is. **Copy SQL** puts the whole
+  statements on the clipboard (see "What a copied text is"). A line is
+  named by its place in the review, so a selection stays on its line while
+  the lines scroll; one whose ends scroll out of view is dropped. The lines
+  take no Tab stop.
 - **The review is the tab's, and goes with the set.** Each table tab has
   its own (`Edits::reviewing`, `Edits::review`). It stays open while
   another tab shows, through a save that runs, fails or conflicts, and
@@ -927,8 +932,8 @@ Only when the workspace's environment is production
 (`Environment::confirms_writes()`), every Save first asks, with every
 statement it would send on screen. The statements are the review of the
 set the confirmation was made with (`WritePrompt::review`), and both looks
-draw them as Review SQL does: the same lines, colours and cut, not
-selectable, in a box that scrolls both ways. The box is as tall as its
+draw them as Review SQL does: the same lines, colours and cut, selectable
+as there, in a box that scrolls both ways. The box is as tall as its
 lines, to twelve of them, and lower in a low window, so the question and
 its answers stay on screen with them.
 
diff --git src/typography.rs src/typography.rs
--- src/typography.rs
+++ src/typography.rs
@@ -571,6 +571,21 @@ impl Laid {
         self.width()
     }
 
+    /// [`Self::paint_left`] for text that can be selected, as a label's
+    /// can: `response` is the widget the text is of, and what the pointer
+    /// presses and drags to select. A selection runs on into the next such
+    /// text, and is copied as it reads.
+    pub fn select_left(&self, ui: &Ui, response: &Response, x: f32, y: f32) {
+        egui::text_selection::LabelSelectionState::label_text_selection(
+            ui,
+            response,
+            egui::pos2(x, y - self.middle()),
+            self.galley.clone(),
+            Color32::PLACEHOLDER,
+            egui::Stroke::NONE,
+        );
+    }
+
     /// Paints with the right edge at `right`, centred on `y`. Returns the width.
     pub fn paint_right(&self, painter: &Painter, right: f32, y: f32) -> f32 {
         self.paint(painter, egui::pos2(right - self.width(), y - self.middle()));
diff --git src/ui/review.rs src/ui/review.rs
--- src/ui/review.rs
+++ src/ui/review.rs
@@ -153,18 +153,31 @@ pub fn rows(
     locale: Locale,
 ) {
     let height = row_height(ui.ctx(), look);
-    for line in lines.get(range).unwrap_or_default() {
+    // What a label that can be selected senses: a press, and a drag where
+    // the pointer is not a finger, which scrolls. Never the keyboard: Tab
+    // passes the lines by.
+    let sense = if ui.input(|input| input.has_touch_screen()) {
+        Sense::CLICK
+    } else {
+        Sense::CLICK | Sense::DRAG
+    };
+    let first = range.start;
+    for (index, line) in lines.get(range).unwrap_or_default().iter().enumerate() {
         let laid = line_text(line, look, palette, locale).layout(ui.ctx());
         // As wide as the line, so a long one scrolls and is never cut by
         // its row; and no narrower than the place.
         let room = Some(ui.available_width()).filter(|room| room.is_finite());
         let width = room.map_or(laid.width(), |room| room.max(laid.width()));
-        // Painted, not a label that can be selected: what is shown is cut,
-        // and a copy of it would be pasted as it is.
-        let (rect, response) = ui.allocate_exact_size(vec2(width, height), Sense::hover());
+        let (_, rect) = ui.allocate_space(vec2(width, height));
+        // Named by its place in the review, not by its place among the
+        // rows in view: a selection stays on its line while the lines
+        // scroll under it.
+        let response = ui.interact(rect, ui.id().with(("line", first + index)), sense);
         response.widget_info(|| WidgetInfo::labeled(WidgetType::Label, true, laid.galley.text()));
         if ui.is_rect_visible(rect) {
-            laid.paint_left(ui.painter(), rect.left(), rect.center().y);
+            // Selected as it is shown: a value that is cut is copied cut,
+            // with its `…`. Copy SQL gives the whole.
+            laid.select_left(ui, &response, rect.left(), rect.center().y);
         }
     }
 }
diff --git src/ui/write_prompts.rs src/ui/write_prompts.rs
--- src/ui/write_prompts.rs
+++ src/ui/write_prompts.rs
@@ -572,8 +572,8 @@ fn confirm_write(app: &mut App, ctx: &egui::Context) {
 /// one to a row in the code face, in a bordered box as tall as they are, to
 /// at most [`review::MAX_ROWS`] of them. It scrolls both ways: a line
 /// longer than the box is cut by it, never wrapped into what could read as
-/// another. The lines are painted and cannot be selected: a value is shown
-/// cut, and a copy of it would be pasted as it is. `rest` is what the
+/// another. The lines can be selected, and a selection is copied as it
+/// reads: a value that is shown cut is copied cut. `rest` is what the
 /// prompt takes of the window besides them: in a low window they stand
 /// lower, so the question and its answers stay on screen with them.
 /// `pages` moves them, by as many pages of the rows in view: Page Down and
````

- [ ] **Step 4: Run the tests and the checks**

Run: `~/.cargo/bin/cargo test --locked --lib -- a_selection_stays_on_its_line the_reviews_lines_are_selected`
Expected: PASS.

Run: `~/.cargo/bin/cargo fmt --all --check && ~/.cargo/bin/cargo clippy --locked --workspace --all-targets -- -D warnings && ~/.cargo/bin/cargo test --locked --workspace --all-targets`
Expected: clean, and every test passes (1679 in the app's library).

- [ ] **Step 5: Commit**

Subject: `Let the lines of Review SQL be selected and copied`

### Task 2: A button shows that it is held down

**Files:**
- Modify: `src/ui/widgets.rs` (`ButtonSpec::show_at`, and its tests)

- [ ] **Step 1: Write the failing test**

````diff
diff --git src/ui/widgets.rs src/ui/widgets.rs
--- src/ui/widgets.rs
+++ src/ui/widgets.rs
@@ -2181,6 +2200,80 @@ mod tests {
         }
     }
 
+    #[test]
+    fn a_button_held_down_is_filled_a_step_past_its_hover() {
+        use crate::testing::Harness;
+        for look in crate::theme::Look::ALL {
+            let said = look.name;
+            let mut harness = Harness::new();
+            harness.set_look(look);
+            let palette = harness.app.palette;
+            let (quiet, plain) = (
+                Rect::from_min_size(pos2(40.0, 40.0), vec2(160.0, 32.0)),
+                Rect::from_min_size(pos2(40.0, 100.0), vec2(160.0, 32.0)),
+            );
+            let draw = |harness: &mut Harness, events: Vec<egui::Event>| {
+                harness.frame_with_events(events, |ui| {
+                    let button = ButtonSpec::new("Copy SQL").quiet();
+                    button.show_at(ui, quiet, &look, &palette);
+                    ButtonSpec::new("Open in SQL").show_at(ui, plain, &look, &palette);
+                });
+            };
+            let fill = |harness: &Harness, place: Rect| {
+                let mut fills = harness.fills.iter();
+                fills
+                    .find(|(rect, _)| *rect == place)
+                    .map(|(_, fill)| *fill)
+            };
+            let line = |harness: &Harness, place: Rect| {
+                let mut outlines = harness.outlines.iter();
+                let found = outlines.find(|(rect, _)| *rect == place);
+                found.map(|(_, stroke)| stroke.color)
+            };
+            // What a press does to each: the bordered one, and the quiet
+            // one, which has no fill at rest and never a line.
+            let held = if look.terminal {
+                palette.selection
+            } else {
+                palette.surface
+            };
+            for (place, at_rest) in [(plain, true), (quiet, false)] {
+                let button = |pressed| egui::Event::PointerButton {
+                    pos: place.center(),
+                    button: egui::PointerButton::Primary,
+                    pressed,
+                    modifiers: egui::Modifiers::NONE,
+                };
+                draw(
+                    &mut harness,
+                    vec![egui::Event::PointerMoved(place.center())],
+                );
+                draw(&mut harness, Vec::new());
+                let hovered = fill(&harness, place).expect("a fill under the pointer");
+                assert_ne!(hovered, held, "{said}");
+                draw(&mut harness, vec![button(true)]);
+                draw(&mut harness, Vec::new());
+                assert_eq!(fill(&harness, place), Some(held), "{said}");
+                assert_eq!(line(&harness, quiet), None, "{said}");
+                if look.terminal && at_rest {
+                    assert_eq!(line(&harness, plain), Some(palette.accent), "{said}");
+                }
+                // Let go, it is under the pointer again; and with the
+                // pointer away, as it was.
+                draw(&mut harness, vec![button(false)]);
+                draw(&mut harness, Vec::new());
+                assert_eq!(fill(&harness, place), Some(hovered), "{said}");
+                draw(
+                    &mut harness,
+                    vec![egui::Event::PointerMoved(pos2(600.0, 400.0))],
+                );
+                draw(&mut harness, Vec::new());
+                let rest = fill(&harness, place).filter(|fill| *fill != Color32::TRANSPARENT);
+                assert_eq!(rest.is_some(), at_rest && !look.terminal, "{said}");
+            }
+        }
+    }
+
     #[test]
     fn space_and_enter_flip_a_toggle_that_has_the_keyboard() {
         use crate::testing::Harness;
````

- [ ] **Step 2: Run it to see it fail**

Run: `~/.cargo/bin/cargo test --locked --lib -- a_button_held_down`
Expected: FAIL. Held down, the button has the fill it has under the pointer.

- [ ] **Step 3: Give the secondary and the quiet button their pressed fill**

````diff
diff --git src/ui/widgets.rs src/ui/widgets.rs
--- src/ui/widgets.rs
+++ src/ui/widgets.rs
@@ -1535,7 +1535,11 @@ impl<'a> ButtonSpec<'a> {
                 palette.dim,
             ),
             (ButtonKind::Secondary, false) => (
-                if hovered {
+                // A step further from the window while it is held down
+                // than under the pointer.
+                if pressed {
+                    palette.surface
+                } else if hovered {
                     palette.panel
                 } else {
                     palette.window
@@ -1545,12 +1549,22 @@ impl<'a> ButtonSpec<'a> {
                 palette.dim,
             ),
             (ButtonKind::Secondary, true) => (
-                if hovered {
+                // Held down, the selection's fill inside the accent's line.
+                if pressed {
+                    palette.selection
+                } else if hovered {
                     palette.text.gamma_multiply(0.08)
                 } else {
                     Color32::TRANSPARENT
                 },
-                Some(Stroke::new(1.0, palette.outline)),
+                Some(Stroke::new(
+                    1.0,
+                    if pressed {
+                        palette.accent
+                    } else {
+                        palette.outline
+                    },
+                )),
                 palette.text,
                 palette.dim,
             ),
@@ -1571,8 +1585,13 @@ impl<'a> ButtonSpec<'a> {
         };
         let (fill, border, text) =
             if (self.quiet || self.link) && self.kind == ButtonKind::Secondary {
-                // Under the pointer, the fill its look gives a secondary button.
-                let fill = if hovered { fill } else { Color32::TRANSPARENT };
+                // Under the pointer and held down, the fills its look gives
+                // a secondary button.
+                let fill = if hovered || pressed {
+                    fill
+                } else {
+                    Color32::TRANSPARENT
+                };
                 // A link is told from a quiet button by its colour alone.
                 let text = if self.link {
                     palette.accent
````

- [ ] **Step 4: Run the test and the checks**

Run: `~/.cargo/bin/cargo test --locked --lib -- a_button_held_down`
Expected: PASS. Then the three checks of task 1, step 4: clean, 1680 tests.

- [ ] **Step 5: Commit**

Subject: `Fill a secondary and a quiet button while it is held down`

### Task 3: Copy SQL says that it copied

**Files:**
- Create: `src/ui/toast.rs`
- Modify: `src/ui/mod.rs` (the module list, `show`, the tests)
- Modify: `src/ui/review.rs` (`COPIED_SQL`, `show`)
- Modify: `src/ui/keys.rs` (`letters`, the `Y` of the terminal look)
- Modify: `src/ui/write_prompts.rs` (`confirm_write`)
- Modify: `src/ui/workspace.rs` (`status_line`)
- Modify: `docs/superpowers/specs/2026-10-03-value-editing-core-design.md`
- Test: `src/ui/mod.rs`

- [ ] **Step 1: Write the failing tests**

````diff
diff --git src/ui/mod.rs src/ui/mod.rs
--- src/ui/mod.rs
+++ src/ui/mod.rs
@@ -16920,6 +16923,113 @@ mod tests {
         }
     }
 
+    #[test]
+    fn copy_sql_says_so_in_a_toast_for_four_seconds() {
+        for look in desktop_looks() {
+            let (mut harness, tab, id) = editable_in(look);
+            let palette = harness.app.palette;
+            make_pending(&mut harness, tab, id, (1, 1), "bob@example.com");
+            review(&mut harness, tab, id);
+            assert!(!painted(&harness, "Copied SQL"), "{}", look.name);
+            harness.copied = None;
+            harness.click("Copy SQL");
+            assert!(harness.copied.is_some(), "{}", look.name);
+            // The toast, once it has faded in: the window's colour on the
+            // text colour, centred over the bottom of the window, clear of
+            // its edge.
+            harness.finish_animations();
+            assert!(
+                painted_in(&harness, "Copied SQL", palette.window),
+                "{}",
+                look.name
+            );
+            let said = harness.painted_rect("Copied SQL").unwrap();
+            let window = harness.ctx.content_rect();
+            assert!(
+                (said.center().x - window.center().x).abs() <= 1.0,
+                "{}",
+                look.name
+            );
+            assert!(said.bottom() < window.bottom(), "{}", look.name);
+            assert!(said.top() > window.bottom() - 60.0, "{}", look.name);
+            let behind = harness.fills.iter().any(|(rect, fill)| {
+                *fill == palette.text && rect.contains_rect(said) && rect.width() < 200.0
+            });
+            assert!(behind, "{}: {:?}", look.name, harness.fills);
+            // The button is as it was, and still copies.
+            assert_eq!(pressable(&mut harness, "Copy SQL").len(), 1);
+            // It asks for the frame that takes it away, and four seconds
+            // later, at sixty frames a second, it is gone.
+            assert!(
+                harness.repaint_after <= std::time::Duration::from_secs(4),
+                "{}: {:?}",
+                look.name,
+                harness.repaint_after
+            );
+            for _ in 0..170 {
+                harness.frame(Vec::new());
+            }
+            assert!(painted(&harness, "Copied SQL"), "{}", look.name);
+            for _ in 0..80 {
+                harness.frame(Vec::new());
+            }
+            assert!(!painted(&harness, "Copied SQL"), "{}", look.name);
+            assert!(painted(&harness, DRAWER), "{}", look.name);
+        }
+    }
+
+    #[test]
+    fn copy_sql_in_the_confirmation_says_so_over_the_sheet() {
+        for look in desktop_looks() {
+            let (mut harness, _tab, _id, _lines) = confirming(look);
+            harness.copied = None;
+            click_dialog(&mut harness, "Copy SQL");
+            assert!(harness.copied.is_some(), "{}", look.name);
+            harness.finish_animations();
+            assert!(
+                painted_in(&harness, "Copied SQL", harness.app.palette.window),
+                "{}",
+                look.name
+            );
+            // The sheet is still up, and still asks.
+            assert_eq!(pressable(&mut harness, "Save to production").len(), 1);
+        }
+    }
+
+    #[test]
+    fn the_terminals_status_line_says_that_the_sql_was_copied() {
+        for with_key in [false, true] {
+            let (mut harness, tab, id) = normal_mode((0, 1));
+            make_pending(&mut harness, tab, id, (1, 1), "bob@example.com");
+            review(&mut harness, tab, id);
+            assert!(!painted(&harness, "copied sql"), "{with_key}");
+            harness.copied = None;
+            if with_key {
+                type_key(&mut harness, Key::Y, "Y");
+            } else {
+                harness.click("Copy SQL");
+            }
+            let copied = harness.copied.clone().expect("the text was copied");
+            assert!(copied.starts_with("-- What Tabletist runs"), "{with_key}");
+            // In the status line, under the panel: no toast floats here.
+            let said = harness.painted_rect("copied sql").expect("copied sql");
+            let panel = drawn(&harness).expect("the panel");
+            assert!(panel.rect.bottom() <= said.top(), "{with_key}");
+            let texts = harness
+                .painted
+                .iter()
+                .filter(|(text, _)| text == "copied sql");
+            assert_eq!(texts.count(), 1, "{with_key}");
+            // What is pending stays on the line beside it.
+            assert!(painted(&harness, "1 pending · 1 row"), "{with_key}");
+            assert!(painted(&harness, "Y copy sql"), "{with_key}");
+            for _ in 0..260 {
+                harness.frame(Vec::new());
+            }
+            assert!(!painted(&harness, "copied sql"), "{with_key}");
+        }
+    }
+
     #[test]
     fn the_lines_wear_the_sql_editors_colours() {
         for look in desktop_looks() {
````

- [ ] **Step 2: Run them to see them fail**

Run: `~/.cargo/bin/cargo test --locked --lib -- copy_sql_says_so copy_sql_in_the_confirmation the_terminals_status_line_says`
Expected: all three FAIL. Nothing paints "Copied SQL" or "copied sql".

- [ ] **Step 3: Add the toast and say it from the three places that copy a review**

````diff
diff --git docs/superpowers/specs/2026-10-03-value-editing-core-design.md docs/superpowers/specs/2026-10-03-value-editing-core-design.md
--- docs/superpowers/specs/2026-10-03-value-editing-core-design.md
+++ docs/superpowers/specs/2026-10-03-value-editing-core-design.md
@@ -866,6 +866,13 @@ confirmation.
   named by its place in the review, so a selection stays on its line while
   the lines scroll; one whose ends scroll out of view is dropped. The lines
   take no Tab stop.
+- **Copy SQL says that it copied.** On macOS and Windows a toast, "Copied
+  SQL", floats over the bottom of the window, centred, for four seconds:
+  the window's colour on the text colour, over a dialog too. Omarchy's
+  status line says `copied sql` for as long, where it has nothing of a
+  lock or of a save to say. A second copy starts the four seconds again.
+  The toast is the frame's own (`src/ui/toast.rs`): it takes neither the
+  pointer nor the keyboard, and nothing of the app's state knows of it.
 - **The review is the tab's, and goes with the set.** Each table tab has
   its own (`Edits::reviewing`, `Edits::review`). It stays open while
   another tab shows, through a save that runs, fails or conflicts, and
diff --git src/ui/keys.rs src/ui/keys.rs
--- src/ui/keys.rs
+++ src/ui/keys.rs
@@ -1311,6 +1311,12 @@ fn letters(app: &mut App, ctx: &egui::Context, actions: &mut Vec<Action>) {
         };
         if let Some(text) = text {
             ctx.copy_text(text);
+            // The cell's `y` shows in the grid; the review's `Y` shows
+            // nowhere, so the status line says it.
+            if sql.is_some() {
+                let said = crate::i18n::gettext(app.locale, crate::ui::review::COPIED_SQL);
+                crate::ui::toast::say(ctx, &said);
+            }
         }
     }
     if pressed(Key::G) {
diff --git src/ui/mod.rs src/ui/mod.rs
--- src/ui/mod.rs
+++ src/ui/mod.rs
@@ -34,6 +34,7 @@ pub mod sql_text;
 pub mod states;
 pub mod structure;
 pub mod terminal_dialog;
+pub mod toast;
 pub mod value_tags;
 pub mod widgets;
 pub mod workspace;
@@ -66,6 +67,8 @@ pub fn show(app: &mut App, ui: &mut egui::Ui) {
     settings::show(app, &ui.ctx().clone());
     write_prompts::show(app, &ui.ctx().clone());
     conflict_prompt::show(app, &ui.ctx().clone());
+    // Last: it says what was done, whatever is on screen by now.
+    toast::show(app, ui.ctx());
 }
 
 /// A problem worth the user's attention that belongs to no one tab (a
diff --git src/ui/review.rs src/ui/review.rs
--- src/ui/review.rs
+++ src/ui/review.rs
@@ -19,6 +19,7 @@ use crate::typography::{Text, TextRole};
 use crate::ui::pending_bar::counted;
 use crate::ui::sql_text;
 use crate::ui::states::Tone;
+use crate::ui::toast;
 use crate::ui::widgets::{self, ButtonSpec};
 
 /// The most lines a place shows before it scrolls.
@@ -44,6 +45,9 @@ const FOOT: f32 = 28.0;
 /// its status line keeps it.
 const GAP: f32 = 18.0;
 
+/// What the app says once a review is on the clipboard.
+pub const COPIED_SQL: &str = "Copied SQL";
+
 /// What a copied review opens with, after the comment's dashes: pasted
 /// elsewhere, nothing checks a row and nothing wraps a transaction.
 const COPIED: &str = "What Tabletist runs to save these changes, in one transaction. \
@@ -347,6 +351,7 @@ pub fn show(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId, id: TabId) {
         && let Some(text) = copy_text(app, tab, id)
     {
         ui.ctx().copy_text(text);
+        toast::say(ui.ctx(), &gettext(locale, COPIED_SQL));
     }
     let placed = Placed {
         tab,
diff --git src/ui/toast.rs src/ui/toast.rs
new file mode 100644
--- /dev/null
+++ src/ui/toast.rs
@@ -0,0 +1,84 @@
+//! A toast: one line that says what was just done ("Copied SQL"), for a
+//! moment and then no longer. On macOS and Windows it floats over the
+//! bottom of the window, centred; the terminal look says it in its status
+//! line. It is the frame's own, as a tooltip is: nothing of the app's
+//! state, and nothing a save or a reload has to know of.
+
+use egui::{Align2, Area, Frame, Id, Margin, Order, vec2};
+
+use crate::app::App;
+use crate::ui::widgets;
+
+/// How long a toast is said, in seconds.
+pub const SECONDS: f64 = 4.0;
+
+/// How far above the window's bottom edge it floats.
+const LIFT: f32 = 16.0;
+
+/// What is being said, and since when, in egui's seconds.
+#[derive(Clone)]
+struct Said {
+    text: String,
+    when: f64,
+}
+
+fn id() -> Id {
+    Id::new("toast")
+}
+
+/// Says `text`, already in the user's language, for [`SECONDS`]. It takes
+/// the place of what was being said.
+pub fn say(ctx: &egui::Context, text: &str) {
+    let said = Said {
+        text: text.to_owned(),
+        when: ctx.input(|input| input.time),
+    };
+    ctx.data_mut(|data| data.insert_temp(id(), said));
+    ctx.request_repaint();
+}
+
+/// What is being said. While something is, a frame is asked for when its
+/// moment ends: it goes without waiting for the pointer to move.
+pub fn said(ctx: &egui::Context) -> Option<String> {
+    let said: Said = ctx.data(|data| data.get_temp(id()))?;
+    let left = SECONDS - (ctx.input(|input| input.time) - said.when);
+    if left <= 0.0 {
+        ctx.data_mut(|data| data.remove::<Said>(id()));
+        return None;
+    }
+    ctx.request_repaint_after(std::time::Duration::from_secs_f64(left));
+    Some(said.text)
+}
+
+/// The toast of macOS and Windows, over everything else in the window, a
+/// dialog too: the text colour filled, the window's colour written on it,
+/// as the primary button is. It takes neither the pointer nor the
+/// keyboard. The terminal look draws none: its status line says it.
+pub fn show(app: &App, ctx: &egui::Context) {
+    if app.look.terminal {
+        return;
+    }
+    let Some(text) = said(ctx) else {
+        return;
+    };
+    let (look, palette) = (&app.look, &app.palette);
+    Area::new(id())
+        .order(Order::Tooltip)
+        .anchor(Align2::CENTER_BOTTOM, vec2(0.0, -LIFT))
+        .interactable(false)
+        .show(ctx, |ui| {
+            Frame::new()
+                .fill(palette.text)
+                .corner_radius(9)
+                .inner_margin(Margin::symmetric(12, 8))
+                .shadow(egui::Shadow {
+                    offset: [0, 8],
+                    blur: 24,
+                    spread: 0,
+                    color: palette.shadow,
+                })
+                .show(ui, |ui| {
+                    widgets::label(ui, widgets::body(look), &text, palette.window, look);
+                });
+        });
+}
diff --git src/ui/workspace.rs src/ui/workspace.rs
--- src/ui/workspace.rs
+++ src/ui/workspace.rs
@@ -1883,7 +1883,17 @@ fn status_line(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId) {
             })
         })
         .unwrap_or_default();
-    let editing = editing_status(app, tab);
+    let mut editing = editing_status(app, tab);
+    // What was just done (a copy of the review's SQL), where the line has
+    // nothing of a lock or of a save to say: this look floats no toast.
+    if editing.said.is_none() {
+        editing.said = super::toast::said(ui.ctx()).map(|text| Said {
+            mark: None,
+            text: look.label(&text),
+            tail: None,
+            color: palette.text,
+        });
+    }
     egui::Panel::bottom(egui::Id::new(("status-line", tab.0)))
         .exact_size(31.0)
         .resizable(false)
diff --git src/ui/write_prompts.rs src/ui/write_prompts.rs
--- src/ui/write_prompts.rs
+++ src/ui/write_prompts.rs
@@ -562,6 +562,7 @@ fn confirm_write(app: &mut App, ctx: &egui::Context) {
         // one in front, nor, were it to change, the one that was read.
         let whole = crate::review::of(dialect, &prompt.changeset, &[], Values::Whole);
         ctx.copy_text(review::text(&whole, locale));
+        crate::ui::toast::say(ctx, &gettext(locale, review::COPIED_SQL));
     }
     if ripe && !early {
         app.actions.extend(actions);
````

- [ ] **Step 4: Run the tests and the checks**

Run: `~/.cargo/bin/cargo test --locked --lib -- copy_sql_says_so copy_sql_in_the_confirmation the_terminals_status_line_says`
Expected: PASS.

Run: `~/.cargo/bin/cargo fmt --all --check && ~/.cargo/bin/cargo clippy --locked --workspace --all-targets -- -D warnings && ~/.cargo/bin/cargo test --locked --workspace --all-targets && RUSTDOCFLAGS='-D warnings' ~/.cargo/bin/cargo doc --locked --workspace --no-deps`
Expected: clean, and every test passes (1683 in the app's library).

- [ ] **Step 5: Commit**

Subject: `Say in a toast that Copy SQL copied`

---

## By hand, in a window (the product owner)

No session here can open the window. With a pending change and Review SQL open:

- Drag across two lines of the drawer: the selection is drawn in the selection colour, the pointer is a text cursor over the lines, and `Mod+C` pastes what was selected. Double click a word.
- Scroll a long review with the wheel while part of it is selected.
- Hold the mouse down on Copy SQL, on Discard all and on Review SQL: each is filled darker than under the pointer.
- Press Copy SQL: the toast shows at the bottom centre, clear of the pending bar's buttons, and goes after four seconds. The same over the production sheet.
- Omarchy: `Y` and a click on `Y copy sql` put `copied sql` in the status line for four seconds.

## What this plan leaves for later

- A toast for the grid's own copies ("Copied 3 rows as TSV"), and a toast with an action.
- The status line of a SQL tab says no toast: nothing there copies a review.
- The accent colour of the design's Quiet button.
- Selecting the whole of a cut value. It would need the selection's range, which egui's label selection keeps to itself.
