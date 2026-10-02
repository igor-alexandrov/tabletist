# Values, focus and dark mode Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A value that is awkward to show (NULL, an empty or blank string, a line break, an array, a binary value, a long text) says what it is in the grid and in the row panel; whatever has the keyboard shows it, in the form the design gives each kind of control, the tree and each grid are one stop for the Tab key, and F6 moves through the window's parts in the design's order; and the dark palette is the design's, drawn as the row "0.1.0 · Values, focus and dark mode" shows.

**Architecture:** Three parts that can be merged one after another. *Dark mode* retunes `Palette::dark()`, the environments' dark colours and the value tags' dark table. *Values* adds pure helpers to `src/ui/format.rs` (marks, PostgreSQL arrays, binary previews), new cell styles to `src/ui/grid.rs`, and their fields to `src/ui/row_panel.rs`. *Focus* adds one module, `src/ui/focus.rs`, that draws the ring of whatever has the keyboard once per frame; views only hint at the ring's form, and the tree and each grid become one Tab stop whose keys the existing shortcuts handle. Nothing changes in `tabletist-db`; the backend gains one file dialog (saving a binary value).

**Tech Stack:** Rust 2024, egui (crmne fork, 0.36), fastframe-icons, rfd. Headless UI tests through `src/testing.rs` (AccessKit). Local screenshots through `--features shots`.

**Design:** the user's Design canvas, row "0.1.0 · Values, focus and dark mode": the artboards "macOS – Awkward values", "macOS – Keyboard focus" and "macOS – Dark mode", and panes 5 and 6 of "Omarchy – States, values and focus". It is not in the repository and is not to be added. The values this plan needs are written out below.

---

## Ground rules (read first)

- Work in `/home/igor/Work/tabletist/.claude/worktrees/values-focus-dark-mode-44b53e`, branch `claude/values-focus-dark-mode-44b53e`.
- `cargo` through mise is broken here: always run `~/.cargo/bin/cargo`. Never point `CARGO_TARGET_DIR` at `/tmp`.
- **Never commit or push unless the user asks.** Each task ends with a "commit point": stop there and report. If the user has asked to commit: one topic per commit, signed (`git commit -S`). If signing fails, ask before committing unsigned. Never set `SSH_AUTH_SOCK` in git commands. End messages with `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.
- No em dashes anywhere (code, comments, docs, commit messages). Use a full stop, comma, colon or parentheses.
- Tests are behavioural (a cell says what its value is, the Tab key reaches a control and something shows it, a key moves where it should). Never a design or pixel conformance test: do not assert the design's sizes, offsets or colours. Design material (PNGs, the artboards' HTML) is never committed.
- Test fixtures and screenshots use neutral names only (`Fixture`, `Bookshop`, `editions`). Never Safari Portal names or rows.
- Screenshots stay local (`target/shots/`, which git ignores). Never push them.
- Views draw text only through `TextRole`s and the `widgets` helpers; never name a font or a size.
- Views do not mutate application state: push an `Action`. What egui keeps in its own memory (which widget has the keyboard, a scroll offset, what `ui::focus` remembers between frames) is not application state.
- Every sentence of ours goes through `gettext(locale, "...")`. The terminal look writes our words in lower case: `look.label(&gettext(locale, "..."))`. What a database or the user named keeps its case.
- Comment density: short doc comments on items saying what and why, matching the surrounding code.

**How the diffs in this plan were made.** Every task was drafted in this worktree, in the order given, and at each task the tree passed `cargo fmt --check`, both `clippy` runs and `cargo test --locked --lib`. The diffs are the differences between those trees, so they apply in order with `git apply` (save a block to a file in the scratchpad, never in the repository) or by hand. Each task gives its tests first and its code second. Where a diff does not apply because the tree has moved on, make the change it shows by hand and keep the tests.

Full check commands (used in the last task, and handy any time):

```bash
~/.cargo/bin/cargo fmt --all --check
~/.cargo/bin/cargo clippy --locked --workspace --all-targets -- -D warnings
~/.cargo/bin/cargo clippy --locked --workspace --all-targets --features shots -- -D warnings
~/.cargo/bin/cargo test --locked --workspace --all-targets
RUSTDOCFLAGS='-D warnings' ~/.cargo/bin/cargo doc --locked --workspace --no-deps
```

While working on one task, the library's tests alone are quick (about three seconds once built):

```bash
~/.cargo/bin/cargo test --locked --lib
```

Screenshots, for the looks by eye (about half a minute, needs a GPU):

```bash
~/.cargo/bin/cargo test --locked --features shots --lib shots -- --ignored
```

They are written to `target/shots/`. The scenes this plan adds: `mock-macos-workspace-dark.png`, `mock-macos-values.png`, `mock-omarchy-values.png`, `focus-NN-<look>-<light|dark>.png` (the ring NN Tab stops in), `focus-grid-*`, `focus-tree-*` and `values-scrolled-*`.

## What the design says

### Dark mode (artboard "macOS – Dark mode")

The workspace of the "Table view" artboard in a dark palette. The content is a step lighter than the bars round it, as the light content is whiter than its bars.

| Piece | Design |
|---|---|
| Content: grid, object header, filter bar, row panel | `#262624` |
| Sidebar, grid header, status bar, the row panel's footer | `#222220` |
| Tab strip, disabled buttons, minor dividers | `#2d2c29` |
| Segmented track, sort chip, grid row dividers | `#302f2c` |
| Pane borders | `#3a3936`; field and button borders `#44433f` |
| Text | `#ecebe6`; secondary `#c4c2bc`; labels `#a3a19b`; faint `#8a8882` |
| Accent | `#7a9df5`; strong (selected names, a sorted header, links under the pointer) `#a9c1fa` |
| Selected row and sidebar item | `#2b3550` |
| Primary button, the hidden-columns pill | `#ecebe6` with `#1c1c1a` text |
| Key icon | `#e0b25a`; JSON keys `#a9c1fa`, strings `#8fd19e`, numbers `#f0a868` |
| Header | the environment's colour mixed 18% into the content, its line 34% |
| Header faces (Connections, the active chip, Read-only) | white at 6 to 8% |
| Environment pill (fill, text) | local `#372e49` `#b9a0f5`; dev `#27372c` `#7fcf9a`; staging `#483a20` `#e0b25a`; production `#452623` `#f08a82`; none `#2f2e2b` `#c4c2bc` |
| Value tags (fill, text) | tan `#3b3026` `#e3b88c`; blue `#26324a` `#9fbdf0` |

### Awkward values (artboard "macOS – Awkward values", Omarchy pane 5)

"Cells never wrap; the inspector shows the whole value."

| Value | Grid cell | Row panel |
|---|---|---|
| NULL | a small filled chip `NULL` (fill `#f0efeb`, text `#8a8984`, 11 px); with the numbers in a numeric column. Omarchy: the word, muted | the same chip |
| Empty string | `''` in `#9a9892` | `''` and the words "empty string" |
| Whitespace only | a `␣` for each space, in `#9a9892` | the same marks and the words "whitespace only" |
| Line break in text | `↵` in its place, the text on one line | the text on its lines |
| Array | a chip for each element (1 px `#e3e1dc` border, text `#4d4c48`, 11.5 px), `+4` for the ones that do not fit; an empty array `{}` in `#9a9892`. Omarchy: `{en,fr}` as written | `languages · text[] · 2 elements`, then `[1] en`, `[2] fr` |
| Binary | one chip `bytea · 48.2 KB`, never its bytes. Omarchy: `bytea 48.2K`, muted | `cover · bytea · 48.2 KB · looks like JPEG`, a box with the first 24 bytes as hex, twelve a line, then `… 49,334 more bytes · first 24 shown`, and "Save to file…". Omarchy: `cover · bytea · 48.2K · jpeg`, `… 49,334 more` |
| JSON | `{ 46 }` chip and a preview (built already) | a tree (built already) |
| Boolean | a tag: true in the first slot (tan), false in the second (blue). Omarchy: orange and cyan text | |
| Long text | cut with "…" (built already) | `title · text · 2,341 characters`, the first three lines, the last fading out, then "Show all" |

The grid keeps its key column in sight: `id` stays at the left with a 1 px line at its right while the other columns scroll under it. The footer says "Rows 1–7 of 7" and "Columns 1–10 of 40 · id pinned" (Omarchy: `cols 1–7 of 40 · id pinned`).

### Keyboard focus (artboard "macOS – Keyboard focus", Omarchy pane 6)

"Focus is shown only after keyboard input (focus-visible). Two forms: an outer ring on controls, an inset ring inside lists and grids."

| Control | Design |
|---|---|
| Buttons, chips, links | outer ring: a 2 px gap, then 2 px of `#2C55C9`, at the control's radius + 2. The ring stays accent blue on every environment colour; the gap shows the header tint. A link's ring has radius 3 |
| Text field | 1 px accent border and a 3 px halo of the accent at 25% |
| Segmented control | 2 px accent on the segment's own edge. "←/→ move inside the group; Tab leaves it." |
| Sidebar rows, tabs, list rows | inset ring: 2 px accent inside the item. "Focus and selection are separate: focused awards, selected covers." A tab's ring has radius 4 |
| Disabled controls | "stay focusable so the tooltip can be read" |
| Grid | the active cell `#E4EBFB` with an inset 2 px accent ring, its row a lighter `#EEF2FC`. "arrows move the cell · ⇧arrows extend · Space opens the inspector · ⌘C copies the cell" |
| Order | "Tab order: connections → chips → sidebar search → object list → tabs → toolbar → grid → inspector. F6 jumps between these regions." |
| Omarchy | "focused pane gets the accent border" (2 px); the cell in reverse video (accent behind dark text); a selected row in a pane without the keys has a muted bar; a focused button is reversed (`[ retry ]`); "ctrl+h/l moves between panes · hjkl moves the cell" |

How the design's names map to the code:

| Design | Code |
|---|---|
| `#262624` content, `#222220` bars | `palette.window`, `palette.panel` |
| `#302f2c`, `#2d2c29` | `palette.surface`, `palette.surface_hover` (as their light values `#f0efeb`, `#ebe9e4` are used) |
| `#3a3936`, `#44433f` | `palette.outline`, `palette.border` |
| `#8a8984` NULL text, `#9a9892` stand-ins | `palette.faint` |
| `#f0efeb` NULL fill | `palette.surface` |
| `#e3e1dc` chip border, `#4d4c48` chip text | `palette.outline`, `palette.secondary` |
| the ring's `#2C55C9` | `palette.accent` |
| `#E4EBFB` cell, `#EEF2FC` row | `palette.selection`, and `palette.window` mixed 60% towards it |
| NULL chip text, array chip text, binary chip text | `TextRole::JsonChip` (mono 11), `TextRole::ValueTag` (mono 11.5), `TextRole::FieldLabel` (sans 11.5) |
| the hex box | `TextRole::Json` on `palette.panel` with a `palette.surface_hover` border |

## Decisions (where the design and the app differ)

Each of these is a product decision the plan makes so that work can start. Change them here before executing if you disagree.

1. **`␣` and `↵` when the fonts have them, `·` and `¶` otherwise.** IBM Plex and JetBrains Mono, the bundled faces, have neither of the design's marks; a browser (and the artboard) takes them from a system font, and so does the app, whose font fallbacks have them on macOS and on Omarchy. The marks are picked once per set of faces by asking the fonts (`grid::marks`). Tests and screenshots run without system fonts, so they show `·` and `¶`.
2. **A line break is the mark alone.** "Night Train↵Part One", not " ↵ " with spaces the value does not have. A Windows line end is one mark; a tab stays a space.
3. **Booleans are tags.** True takes the first slot and false the second in every look, as the artboards draw them. They were a neutral chip and muted text; the test that said so is rewritten.
4. **Arrays are PostgreSQL's.** A column is an array by its type (`_text` in a result, `text[]` in the catalog; the header says `text[]`). An array inside an array is one chip, written as it is. Text that does not parse as an array stays text.
5. **A binary value is its first 24 bytes.** The row panel's hex dump (4096 bytes, sixteen a line with offsets, behind "Show all") gives way to the design's two lines of twelve. The whole value is one click away: Copy (as `0x…`, as before) and "Save to file…". `hex_dump` and `HEX_LIMIT` and their two tests go with it.
6. **"looks like JPEG" is a guess from the first bytes** (JPEG, PNG, GIF, PDF, ZIP, gzip, WebP, SQLite), and said as one. Nothing is decoded.
7. **"Save to file…" sits under the hex box**, not on the label's line: that line's right end is the copy button's. The file dialog suggests `table-column.ext`, the names cut to letters, digits, `-` and `_`. A write that fails is told by the notice bar, as a failed save of settings is. The terminal look gets the link too, without the design's `[s]` key (`s` is Structure).
8. **"Show all" opens the value in place.** There is no viewer ("open in viewer" is left out). A value is cut to three lines by how it is laid out at the panel's width, not by a count of characters; its label says how long it is once it has more than 120 characters.
9. **JSON is left as it is.** The cell's preview stays the document's first string (the "Table view" artboard's), not `"awards": […`. The tree does not fold big documents to "… 42 more keys", and the rule "over 1 MB open collapsed; over 10 MB show size only" is not built: a document over 256 KB is shown as text, as today. Its label gets no "46 keys · 18 KB" (the label's line holds the fold link).
10. **No column chooser and no digit grouping.** "Columns 10 / 40" is a feature of its own. `1,240.00` would mean guessing which numbers are amounts and which are identifiers; values read as the database writes them.
11. **The key column is pinned when it leads the grid**: the first column, if it is part of the primary key. A SQL result pins nothing. The footer's note shows only while some columns are out of view and the footer has the room; the terminal look says it in its status line. The design's hint "⇧ scroll or ⌥→ to move by column" is left out (there is no move by column).
12. **One painter draws focus.** `ui::focus::paint` rings whatever egui says has the keyboard, so every Tab stop shows, egui's own widgets included. A widget that needs another form than the outer ring says so (`focus::hint`). The six rings drawn by hand today (`primary_focus_ring`) go, with the test of that function; the tests in `focus.rs` take its place.
13. **Focus-visible is the last input's kind.** A key press (or a screen reader moving focus) shows the ring, a pointer press hides it. Text fields keep their caret and accent border either way.
14. **The tree and each grid are one Tab stop.** Their rows take clicks, not the Tab key (they were a stop each). With the keyboard on the tree ("Objects") or a grid ("Rows") the arrows, Enter and Space act on what it shows, as they do when nothing is focused. Tabbing onto a grid selects nothing (a new `Action::GridKeys` only hands it the arrows); the first arrow picks a row. Column headers that sort stay Tab stops.
15. **The arrows act on the tree or the grid only while no control has the keyboard.** With a button focused they used to move the grid behind it; now they are the button's (egui moves focus with them). Three tests that tabbed to rows are rewritten for the one stop.
16. **Drag areas are no Tab stops.** The title bar's and the picker header's drag areas and the column resize handles took the Tab key and did nothing with it; with focus now visible they would show a ring.
17. **A segmented control is one Tab stop** (its chosen segment); ← and → choose its neighbours and the keyboard follows. That is the tree/flat switch and the Data/Structure switch.
18. **F6 and Shift+F6 step through these parts**, where they are on screen: the bar (Connections), the sidebar's filter, the tree, the tabs (the active one), a table's header (its switch), a SQL editor's script, the rows, the row panel (its first button, when it shows a row). "Connections" and "chips" are one part: the bar. The terminal look also steps through its three panes (tree, rows, row panel) with ctrl+l and ctrl+h.
19. **⇧arrows do not extend the selection.** The grid selects one cell.
20. **The terminal look**: a focused button is reversed (accent behind, the window's tone for its text) instead of ringed; everything else is ringed, square. The pane the keys go to has the 2 px accent border while the keyboard is in use. The open object's bar in the tree is muted while the arrows are the grid's.
21. **A disabled button takes the Tab key** and shows its reason as a tooltip while the keyboard is on it; a screen reader gets the reason as the button's description. Enter does not press it. That is a `ButtonSpec` with a reason ("Add row", the row panel's Edit, Duplicate and Delete). An icon button that is disabled for the moment (the footer's page arrows, the row panel's previous and next row) has no reason to read and stays out of the Tab order.
22. **The dark palette is the design's, with these fillings-in.** Dialogs take the content's tone (the artboard shows none). What is raised (a pop-up button, the running card) keeps the lightest surface. egui's own controls get lighter under the pointer in the dark and darker in the light (`Palette::hover_fill`). The design gives two of the eight dark tags (tan, blue); the other six are made the same way (a deep tone of the hue under a light one) and a test keeps each readable on its fill. The environments' five pills are the design's.
23. **Windows follows the macOS drawing** with its own radii and faces, as everywhere else.
24. **The Tab key keeps the order the parts are drawn in; F6 has the design's.** egui gives the Tab key its stops in the order widgets are made, and the workspace makes the row panel before the table's header and rows (a side panel comes before what it sits beside). So on the desktop looks Tab goes bar, sidebar, tabs, row panel, table header, toolbar, footer, filter bar, rows, where the design reads "tabs → toolbar → grid → inspector"; in the terminal look the tabs come before the sidebar. F6 steps through the parts in the design's order whatever the drawing order is. Making Tab follow it too means drawing the workspace in another order, which is not done here.

## File map

| File | Change |
|---|---|
| `src/theme.rs` | `Palette::dark()` to the design; `hover_fill`, `pressed_fill` |
| `src/env.rs` | the dark badges; the bar tint mixed into the content |
| `src/ui/value_tags.rs` | the dark slots; booleans as tags |
| `src/ui/workspace.rs` | the bar's faces in the dark; the columns note in the terminal's status line; the bar as a region; the drag area no Tab stop |
| `src/ui/format.rs` | `Marks`, `cell_line`, `blank_text`, `is_array`, `type_label`, `array_items`, `hex_preview`, `sniff`, `binary_label`, `terse_size`, `save_name`; `FieldText::characters`; `hex_dump` and `HEX_LIMIT` removed |
| `src/ui/grid.rs` | cell styles `Quiet`, `Chip`, `Array`; the NULL chip; `marks`, `null_label`; the grid as one Tab stop; the lit cell; the pinned column; `ColumnsShown` |
| `src/ui/data_view.rs` | `cell` and `plain_cell` take the column; the header's `text[]`; the switch's keys; `grid_id`, `columns_note`, the footer's note |
| `src/ui/row_panel.rs` | label facts; NULL, blank, array, binary and long text fields; "Save to file…"; the panel as a region |
| `src/ui/focus.rs` | **new**: focus-visible, the ring painter and its hints, panes, segments, regions |
| `src/ui/mod.rs` | `pub mod focus;`; the tests of Tab stops, panes, regions |
| `src/app.rs` | `focus::begin_frame` and `focus::paint` round the frame; `Action::GridKeys`, `Action::SaveValue` |
| `src/model.rs` | the two actions |
| `src/backend.rs` | `save_bytes` |
| `src/ui/widgets.rs` | hints in `ButtonSpec`, `segmented`, `filter_field`; the terminal's reversed button; a disabled button's focus; `primary_focus_ring` removed |
| `src/typography.rs` | a clickable text is a link: its ring |
| `src/ui/sidebar.rs` | the tree as one Tab stop; its cursor's ring; the terminal's pane mark and muted bar; regions |
| `src/ui/keys.rs` | keys by where the keyboard is; F6, Shift+F6, ctrl+h, ctrl+l |
| `src/ui/object_tabs.rs`, `quick_open.rs`, `picker.rs`, `sql_results.rs`, `sql_editor.rs`, `sql_text.rs`, `connect_dialog/choice.rs` | hints and regions |
| `src/testing.rs` | `Harness::outlines` |
| `src/shots.rs` | the scenes listed under "Ground rules" |
| `AGENTS.md` | who draws focus |

---

### Task 1: The dark palette

**Files:**
- Modify: `src/theme.rs`

The design's dark colours replace the ones `Palette::dark()` has. The content (`window`) becomes a step lighter than the bars (`panel`), which is the other way round from today, so two things that leaned on the old order move with it: dialogs take the content's tone, and egui's own controls (the buttons of menus, combo boxes) get a fill for "under the pointer" that goes lighter in the dark and darker in the light, where one field did both before.

- [ ] **Step 1: Write the tests**

One new test, `a_control_under_the_pointer_stands_further_off_the_window`. The tests that already guard the palette keep passing and are the ones to watch: `palette_text_meets_contrast` (text, labels, accent, danger and warning at 4.5:1 on every surface), `dialog_controls_stand_off_the_dialog` and `the_default_palettes_are_warm`.

````diff
diff --git a/src/theme.rs b/src/theme.rs
--- a/src/theme.rs
+++ b/src/theme.rs
@@ -950,6 +976,18 @@ mod tests {
         assert!(failures.is_empty(), "{failures:#?}");
     }
 
+    #[test]
+    fn a_control_under_the_pointer_stands_further_off_the_window() {
+        for (theme, palette) in [("light", Palette::light()), ("dark", Palette::dark())] {
+            let off = |color| contrast(color, palette.window);
+            assert!(off(palette.hover_fill()) > off(palette.surface), "{theme}");
+            assert!(
+                off(palette.pressed_fill()) > off(palette.hover_fill()),
+                "{theme}"
+            );
+        }
+    }
+
     #[test]
     fn dialog_controls_stand_off_the_dialog() {
         for (theme, palette) in [("light", Palette::light()), ("dark", Palette::dark())] {
````

- [ ] **Step 2: Run them and see them fail**

Run: `~/.cargo/bin/cargo test --locked --lib`

Expected, before the code below is there:

````text
Does not compile:
error[E0599]: no method named `hover_fill` found for struct `theme::Palette` in the current scope
error[E0599]: no method named `pressed_fill` found for struct `theme::Palette` in the current scope
````

- [ ] **Step 3: Write the code**

````diff
diff --git a/src/theme.rs b/src/theme.rs
--- a/src/theme.rs
+++ b/src/theme.rs
@@ -81,37 +81,63 @@ pub const EXTRA_COLORS: [&str; 10] = [
 ];
 
 impl Palette {
+    /// The macOS design's dark colours. The content is a step lighter than
+    /// the bars round it, as the light content is whiter than its bars.
     pub fn dark() -> Self {
         Self {
             dark: true,
-            window: Color32::from_rgb(0x1f, 0x1e, 0x1d),
-            panel: Color32::from_rgb(0x26, 0x25, 0x23),
-            surface: Color32::from_rgb(0x30, 0x2e, 0x2c),
-            surface_hover: Color32::from_rgb(0x39, 0x37, 0x34),
-            surface_active: Color32::from_rgb(0x43, 0x40, 0x3d),
-            outline: Color32::from_rgb(0x3a, 0x38, 0x35),
-            text: Color32::from_rgb(0xec, 0xeb, 0xe8),
-            secondary: Color32::from_rgb(0xb0, 0xad, 0xa7),
-            dim: Color32::from_rgb(0x9a, 0x97, 0x91),
-            accent: Color32::from_rgb(0x5a, 0x9f, 0xf8),
-            accent_hover: Color32::from_rgb(0x7a, 0xb2, 0xf9),
-            on_accent: Color32::from_rgb(0x0b, 0x13, 0x20),
-            danger: Color32::from_rgb(0xf2, 0x70, 0x7a),
-            warning: Color32::from_rgb(0xea, 0xb3, 0x5a),
-            // The panel tone: fields and buttons (surface) must stand off
-            // a dialog as they do off the sidebar.
-            overlay: Color32::from_rgb(0x26, 0x25, 0x23),
+            window: Color32::from_rgb(0x26, 0x26, 0x24),
+            panel: Color32::from_rgb(0x22, 0x22, 0x20),
+            surface: Color32::from_rgb(0x30, 0x2f, 0x2c),
+            surface_hover: Color32::from_rgb(0x2d, 0x2c, 0x29),
+            // Lighter than the surface: what is raised or pressed comes
+            // forward in the dark.
+            surface_active: Color32::from_rgb(0x3a, 0x39, 0x36),
+            outline: Color32::from_rgb(0x3a, 0x39, 0x36),
+            text: Color32::from_rgb(0xec, 0xeb, 0xe6),
+            secondary: Color32::from_rgb(0xc4, 0xc2, 0xbc),
+            dim: Color32::from_rgb(0xa3, 0xa1, 0x9b),
+            // Accent, danger and warning are read as text: 4.5:1 or better.
+            accent: Color32::from_rgb(0x7a, 0x9d, 0xf5),
+            accent_hover: Color32::from_rgb(0xa9, 0xc1, 0xfa),
+            on_accent: Color32::from_rgb(0x1c, 0x1c, 0x1a),
+            danger: Color32::from_rgb(0xf0, 0x8a, 0x82),
+            warning: Color32::from_rgb(0xe0, 0xb2, 0x5a),
+            // The content's tone, as in the light palette: a dialog is a
+            // piece of content over a dimmed window.
+            overlay: Color32::from_rgb(0x26, 0x26, 0x24),
             shadow: Color32::from_black_alpha(150),
-            faint: Color32::from_rgb(0x85, 0x81, 0x7b),
-            border: Color32::from_rgb(0x48, 0x45, 0x41),
-            selection: Color32::from_rgb(0x28, 0x34, 0x57),
-            success: Color32::from_rgb(0x9e, 0xce, 0x6a),
-            info: Color32::from_rgb(0x7d, 0xcf, 0xff),
-            orange: Color32::from_rgb(0xff, 0x9e, 0x64),
-            magenta: Color32::from_rgb(0xbb, 0x9a, 0xf7),
-            blue: Color32::from_rgb(0x7a, 0xa2, 0xf7),
-            rose: Color32::from_rgb(0xff, 0x7e, 0xb6),
-            olive: Color32::from_rgb(0xb5, 0xbd, 0x68),
+            faint: Color32::from_rgb(0x8a, 0x88, 0x82),
+            border: Color32::from_rgb(0x44, 0x43, 0x3f),
+            selection: Color32::from_rgb(0x2b, 0x35, 0x50),
+            success: Color32::from_rgb(0x8f, 0xd1, 0x9e),
+            info: Color32::from_rgb(0x9f, 0xbd, 0xf0),
+            orange: Color32::from_rgb(0xf0, 0xa8, 0x68),
+            magenta: Color32::from_rgb(0xb9, 0xa0, 0xf5),
+            blue: Color32::from_rgb(0x9f, 0xbd, 0xf0),
+            rose: Color32::from_rgb(0xed, 0xa6, 0xc6),
+            olive: Color32::from_rgb(0xbc, 0xcb, 0x88),
+        }
+    }
+
+    /// The fill of a control under the pointer: a step further from the
+    /// window than the control at rest, so darker on a light palette and
+    /// lighter on a dark one.
+    pub fn hover_fill(&self) -> Color32 {
+        if self.dark {
+            self.surface_active
+        } else {
+            self.surface_hover
+        }
+    }
+
+    /// The fill of a control while it is pressed: a step past
+    /// [`Palette::hover_fill`].
+    pub fn pressed_fill(&self) -> Color32 {
+        if self.dark {
+            self.surface_active.lerp_to_gamma(self.text, 0.08)
+        } else {
+            self.surface_active
         }
     }
 
@@ -600,12 +626,12 @@ fn apply_to_style(style: &mut egui::Style, palette: &Palette, look: &Look) {
     visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0, palette.text);
     visuals.widgets.inactive.bg_fill = palette.surface;
     visuals.widgets.inactive.weak_bg_fill = palette.surface;
-    visuals.widgets.hovered.bg_fill = palette.surface_hover;
-    visuals.widgets.hovered.weak_bg_fill = palette.surface_hover;
-    visuals.widgets.active.bg_fill = palette.surface_active;
-    visuals.widgets.active.weak_bg_fill = palette.surface_active;
-    visuals.widgets.open.bg_fill = palette.surface_hover;
-    visuals.widgets.open.weak_bg_fill = palette.surface_hover;
+    visuals.widgets.hovered.bg_fill = palette.hover_fill();
+    visuals.widgets.hovered.weak_bg_fill = palette.hover_fill();
+    visuals.widgets.active.bg_fill = palette.pressed_fill();
+    visuals.widgets.active.weak_bg_fill = palette.pressed_fill();
+    visuals.widgets.open.bg_fill = palette.hover_fill();
+    visuals.widgets.open.weak_bg_fill = palette.hover_fill();
     if look.bordered_controls {
         // Omarchy's shell: foreground alpha over the background, 1 px borders.
         let fg = palette.text;
````

- [ ] **Step 4: Run the tests**

Run: `~/.cargo/bin/cargo test --locked --lib theme::`
Expected: PASS.

Then all of the library's: `~/.cargo/bin/cargo test --locked --lib`
Expected: `test result: ok. 853 passed; 0 failed; 1 ignored`.

- [ ] **Step 5: Format and lint**

```bash
~/.cargo/bin/cargo fmt --all --check
~/.cargo/bin/cargo clippy --locked --workspace --all-targets -- -D warnings
~/.cargo/bin/cargo clippy --locked --workspace --all-targets --features shots -- -D warnings
```

Expected: no output.

- [ ] **Step 6: Commit point**

Stop and report. If the user has asked for commits:

```bash
git add -A src AGENTS.md
git commit -S -m "Take the dark palette from the design" -m "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

### Task 2: The environments in the dark

**Files:**
- Modify: `src/env.rs`

A dark bar is the environment's colour mixed into the content at 18% (its line at 34%): more than the 12% of a light bar, because a dark tint needs more of the colour to show. The pills take the design's fills and texts instead of a computed lightening, so `readable` goes.

- [ ] **Step 1: Write the tests**

`macos_dark_mode_keeps_the_colour_and_mixes_it_into_the_panel` becomes `..._into_the_content`: the base colour is kept, the bar is the content tinted (not the panel) with its line standing further off the content than the bar does, the pill's text reads on its fill, and what the bar says reads on its tint. No hex and no ratio of the design is asserted.

````diff
diff --git a/src/env.rs b/src/env.rs
--- a/src/env.rs
+++ b/src/env.rs
@@ -260,6 +262,7 @@ fn omarchy(env: Environment, palette: &Palette) -> EnvColors {
 #[cfg(test)]
 mod tests {
     use super::*;
+    use crate::theme::contrast;
 
     fn hex(value: u32) -> Color32 {
         let [_, r, g, b] = value.to_be_bytes();
@@ -286,19 +289,26 @@ mod tests {
     }
 
     #[test]
-    fn macos_dark_mode_keeps_the_colour_and_mixes_it_into_the_panel() {
+    fn macos_dark_mode_keeps_the_colour_and_mixes_it_into_the_content() {
         let dark = Palette::dark();
         for env in Environment::ALL {
             let light = env_colors(env, Platform::Native, &Palette::light());
             let colors = env_colors(env, Platform::Native, &dark);
             assert_eq!(colors.base(), light.base(), "{env:?}");
-            assert_eq!(colors.bar_bg(), mix(dark.panel, colors.base(), 0.12));
-            assert_eq!(colors.bar_border(), mix(dark.panel, colors.base(), 0.28));
-            assert_eq!(colors.badge_bg(), mix(dark.panel, colors.base(), 0.2));
+            // The bar is the content tinted, and its line stands off the
+            // content more than the bar does.
+            let off = |color| contrast(color, dark.window);
+            assert_ne!(colors.bar_bg(), dark.window, "{env:?}");
+            assert_ne!(colors.bar_bg(), dark.panel, "{env:?}");
+            assert!(off(colors.bar_border()) > off(colors.bar_bg()), "{env:?}");
             assert!(
                 contrast(colors.badge_fg(), colors.badge_bg()) >= 4.5,
                 "{env:?}"
             );
+            // What the bar says is read on its tint.
+            for text in [dark.text, dark.secondary] {
+                assert!(contrast(text, colors.bar_bg()) >= 4.5, "{env:?}");
+            }
         }
     }
 
````

- [ ] **Step 2: Run them**

Run: `~/.cargo/bin/cargo test --locked --lib`

Expected, before the code below is there:

````text
These pass already: they guard what the code below keeps true.
test result: ok. 853 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out
````

- [ ] **Step 3: Write the code**

````diff
diff --git a/src/env.rs b/src/env.rs
--- a/src/env.rs
+++ b/src/env.rs
@@ -9,7 +9,7 @@ use egui::Color32;
 use serde::{Deserialize, Serialize};
 use tabletist_db::{ConnectSpec, Driver};
 
-use crate::theme::{Look, Palette, contrast, mix};
+use crate::theme::{Look, Palette, mix};
 
 /// What a connection is. It decides the connection's colour everywhere and
 /// whether it is read-only by default.
@@ -139,7 +139,7 @@ impl EnvColors {
 }
 
 /// An environment's colours on `platform` with `palette`. macOS and Windows
-/// take fixed colours (softened into the panel in dark mode); Linux takes
+/// take fixed colours (mixed into the content in dark mode); Linux takes
 /// the theme's red, yellow, green, magenta and muted, so it follows a
 /// theme change.
 pub fn env_colors(env: Environment, platform: Platform, palette: &Palette) -> EnvColors {
@@ -201,6 +201,19 @@ fn native_table(env: Environment) -> (Color32, Color32, Color32) {
     }
 }
 
+/// The badge's fill and text on a dark palette, as the dark design draws
+/// them: the base colour sunk into the content, its text a light tone of it.
+fn native_dark_badge(env: Environment) -> (Color32, Color32) {
+    let rgb = Color32::from_rgb;
+    match env {
+        Environment::Local => (rgb(0x37, 0x2e, 0x49), rgb(0xb9, 0xa0, 0xf5)),
+        Environment::Dev => (rgb(0x27, 0x37, 0x2c), rgb(0x7f, 0xcf, 0x9a)),
+        Environment::Staging => (rgb(0x48, 0x3a, 0x20), rgb(0xe0, 0xb2, 0x5a)),
+        Environment::Production => (rgb(0x45, 0x26, 0x23), rgb(0xf0, 0x8a, 0x82)),
+        Environment::None => (rgb(0x2f, 0x2e, 0x2b), rgb(0xc4, 0xc2, 0xbc)),
+    }
+}
+
 fn native(env: Environment, palette: &Palette) -> EnvColors {
     let (base, badge_bg, badge_fg) = native_table(env);
     if !palette.dark {
@@ -212,29 +225,18 @@ fn native(env: Environment, palette: &Palette) -> EnvColors {
             bar_border: mix(Color32::WHITE, base, 0.28),
         };
     }
-    // Dark mode keeps the colour but mixes it into the panel, and lightens
-    // the badge text until it reads on its fill.
-    let badge_bg = mix(palette.panel, base, 0.2);
+    // Dark mode keeps the colour and mixes it into the content, a little
+    // stronger than on white: a dark tint needs more of it to show.
+    let (badge_bg, badge_fg) = native_dark_badge(env);
     EnvColors {
         base,
         badge_bg,
-        badge_fg: readable(base, badge_bg),
-        bar_bg: mix(palette.panel, base, 0.12),
-        bar_border: mix(palette.panel, base, 0.28),
+        badge_fg,
+        bar_bg: mix(palette.window, base, 0.18),
+        bar_border: mix(palette.window, base, 0.34),
     }
 }
 
-/// `color` lightened towards white until it has 4.5:1 against `background`.
-fn readable(color: Color32, background: Color32) -> Color32 {
-    let mut color = color;
-    let mut step = 0;
-    while contrast(color, background) < 4.5 && step < 50 {
-        color = color.lerp_to_gamma(Color32::WHITE, 0.04);
-        step += 1;
-    }
-    color
-}
-
 /// The theme key each environment takes, as the Omarchy template maps it
 /// onto the palette (`dark_background` is the panel).
 fn omarchy(env: Environment, palette: &Palette) -> EnvColors {
````

- [ ] **Step 4: Run the tests**

Run: `~/.cargo/bin/cargo test --locked --lib env::`
Expected: PASS.

Then all of the library's: `~/.cargo/bin/cargo test --locked --lib`
Expected: `test result: ok. 853 passed; 0 failed; 1 ignored`.

- [ ] **Step 5: Format and lint**

```bash
~/.cargo/bin/cargo fmt --all --check
~/.cargo/bin/cargo clippy --locked --workspace --all-targets -- -D warnings
~/.cargo/bin/cargo clippy --locked --workspace --all-targets --features shots -- -D warnings
```

Expected: no output.

- [ ] **Step 6: Commit point**

Stop and report. If the user has asked for commits:

```bash
git add -A src AGENTS.md
git commit -S -m "Mix a dark bar's colour into the content" -m "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

### Task 3: The value tags in the dark

**Files:**
- Modify: `src/ui/value_tags.rs`

The desktop looks get a dark table beside the light one: a deep tone of each hue under a light one. The design draws two of the eight (tan `#3b3026` `#e3b88c`, blue `#26324a` `#9fbdf0`); the other six are made the same way.

- [ ] **Step 1: Write the tests**

`a_tag_reads_on_its_fill_and_no_two_slots_look_alike`: in both palettes every slot's text has 4.5:1 on its fill, no fill is the window's or a selected row's colour, and no two slots are the same. It guards the table; that the table is the design's is checked by eye in the next task.

````diff
diff --git a/src/ui/value_tags.rs b/src/ui/value_tags.rs
--- a/src/ui/value_tags.rs
+++ b/src/ui/value_tags.rs
@@ -316,6 +323,30 @@ mod tests {
         }
     }
 
+    #[test]
+    fn a_tag_reads_on_its_fill_and_no_two_slots_look_alike() {
+        for look in [Look::macos(), Look::standard()] {
+            for (theme, palette) in [("light", Palette::light()), ("dark", Palette::dark())] {
+                let slots: Vec<_> = (0..SLOTS)
+                    .map(|slot| slot_colors(slot, &look, &palette))
+                    .collect();
+                for (slot, (text, fill)) in slots.iter().enumerate() {
+                    let fill = fill.expect("a desktop tag is a chip");
+                    let ratio = crate::theme::contrast(*text, fill);
+                    assert!(ratio >= 4.5, "{theme} slot {slot}: {ratio:.2}");
+                    // The chip shows on the content and on a selected row.
+                    for under in [palette.window, palette.selection] {
+                        assert_ne!(fill, under, "{theme} slot {slot}");
+                    }
+                    assert!(
+                        slots[..slot].iter().all(|other| other != &slots[slot]),
+                        "{theme} slot {slot} repeats"
+                    );
+                }
+            }
+        }
+    }
+
     #[test]
     fn terminal_slots_follow_the_theme_without_fills() {
         let mut palette = Palette::dark();
````

- [ ] **Step 2: Run them**

Run: `~/.cargo/bin/cargo test --locked --lib`

Expected, before the code below is there:

````text
These pass already: they guard what the code below keeps true.
test result: ok. 854 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out
````

- [ ] **Step 3: Write the code**

````diff
diff --git a/src/ui/value_tags.rs b/src/ui/value_tags.rs
--- a/src/ui/value_tags.rs
+++ b/src/ui/value_tags.rs
@@ -77,6 +77,19 @@ const FIXED: [(u32, u32); SLOTS] = [
     (0xEEF3E0, 0x4C5A1E), // olive
 ];
 
+/// The same slots on a dark palette: a deep tone of each hue under a light
+/// one.
+const FIXED_DARK: [(u32, u32); SLOTS] = [
+    (0x3B3026, 0xE3B88C), // tan
+    (0x26324A, 0x9FBDF0), // blue
+    (0x332B4A, 0xC3ACF2), // violet
+    (0x1F3A38, 0x8FD3CD), // teal
+    (0x42361C, 0xE6C274), // amber
+    (0x30343A, 0xB9C1CC), // slate
+    (0x45283A, 0xEDA6C6), // rose
+    (0x30381F, 0xBCCB88), // olive
+];
+
 fn hex(rgb: u32) -> Color32 {
     let [_, r, g, b] = rgb.to_be_bytes();
     Color32::from_rgb(r, g, b)
@@ -88,15 +101,9 @@ pub fn slot_colors(slot: usize, look: &Look, palette: &Palette) -> (Color32, Opt
     if look.terminal {
         return (terminal_slots(palette)[slot % SLOTS], None);
     }
-    let (fill, text) = FIXED[slot % SLOTS];
-    let (fill, text) = (hex(fill), hex(text));
-    if palette.dark {
-        // The same hues on a dark window: the light tone as text over a
-        // deep fill of the ink.
-        (fill, Some(palette.window.lerp_to_gamma(text, 0.45)))
-    } else {
-        (text, Some(fill))
-    }
+    let table = if palette.dark { &FIXED_DARK } else { &FIXED };
+    let (fill, text) = table[slot % SLOTS];
+    (hex(text), Some(hex(fill)))
 }
 
 /// The colours a cell styled `style` draws in: its text, and its chip's
````

- [ ] **Step 4: Run the tests**

Run: `~/.cargo/bin/cargo test --locked --lib value_tags`
Expected: PASS.

Then all of the library's: `~/.cargo/bin/cargo test --locked --lib`
Expected: `test result: ok. 854 passed; 0 failed; 1 ignored`.

- [ ] **Step 5: Format and lint**

```bash
~/.cargo/bin/cargo fmt --all --check
~/.cargo/bin/cargo clippy --locked --workspace --all-targets -- -D warnings
~/.cargo/bin/cargo clippy --locked --workspace --all-targets --features shots -- -D warnings
```

Expected: no output.

- [ ] **Step 6: Commit point**

Stop and report. If the user has asked for commits:

```bash
git add -A src AGENTS.md
git commit -S -m "Give value tags a dark table of their own" -m "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

### Task 4: The bar's faces in the dark, and a dark scene

**Files:**
- Modify: `src/ui/workspace.rs`
- Modify: `src/shots.rs`

On a light bar the Connections button, the active chip and the Read-only pill are faces of white at 70 to 100%. On a dark bar they were faces of the window's colour, which reads as holes; the design washes them with white at 6 to 8%. The scene `MacWorkspaceDark` renders the mockup's workspace in the dark palette at the mockup's size, for the look by eye. There is nothing to unit test here: this task ends with a look at the picture.

- [ ] **Step 1: Write the code**

````diff
diff --git a/src/ui/workspace.rs b/src/ui/workspace.rs
--- a/src/ui/workspace.rs
+++ b/src/ui/workspace.rs
@@ -924,14 +924,12 @@ fn mac_bar(
     locale: crate::i18n::Locale,
 ) {
     let center = rect.center().y;
-    // The bar's own rule, and white faces over its tint.
+    // The bar's own rule, and faces over its tint: nearly white on a light
+    // bar, a thin wash of white on a dark one.
     let rim = env.bar_border();
     let face = |alpha: f32| {
-        if palette.dark {
-            palette.window.gamma_multiply(alpha)
-        } else {
-            egui::Color32::WHITE.gamma_multiply(alpha)
-        }
+        let alpha = if palette.dark { alpha * 0.09 } else { alpha };
+        egui::Color32::WHITE.gamma_multiply(alpha)
     };
     let hair = Stroke::new(widgets::hairline(ui), rim);
     let corner = CornerRadius::same(look.radius);
````

- [ ] **Step 2: Run the tests**

Run: `~/.cargo/bin/cargo test --locked --lib ui::env_tests`
Expected: PASS.

Then all of the library's: `~/.cargo/bin/cargo test --locked --lib`
Expected: `test result: ok. 854 passed; 0 failed; 1 ignored`.

- [ ] **Step 3: Add the scenes**

````diff
diff --git a/src/shots.rs b/src/shots.rs
--- a/src/shots.rs
+++ b/src/shots.rs
@@ -723,6 +723,8 @@ mod mock {
     #[derive(Clone, Copy, Debug, PartialEq, Eq)]
     pub enum Screen {
         MacWorkspace,
+        /// The same workspace in the dark palette.
+        MacWorkspaceDark,
         OmarchyWorkspace,
         MacPicker,
         OmarchyPicker,
@@ -731,8 +733,9 @@ mod mock {
     }
 
     impl Screen {
-        pub const ALL: [Screen; 6] = [
+        pub const ALL: [Screen; 7] = [
             Self::MacWorkspace,
+            Self::MacWorkspaceDark,
             Self::OmarchyWorkspace,
             Self::MacPicker,
             Self::OmarchyPicker,
@@ -744,6 +747,7 @@ mod mock {
         pub fn name(self) -> &'static str {
             match self {
                 Self::MacWorkspace => "macos-workspace",
+                Self::MacWorkspaceDark => "macos-workspace-dark",
                 Self::OmarchyWorkspace => "omarchy-workspace",
                 Self::MacPicker => "macos-connections",
                 Self::OmarchyPicker => "omarchy-connections",
@@ -754,7 +758,9 @@ mod mock {
 
         pub fn look(self) -> Look {
             match self {
-                Self::MacWorkspace | Self::MacPicker | Self::MacDialog => Look::macos(),
+                Self::MacWorkspace | Self::MacWorkspaceDark | Self::MacPicker | Self::MacDialog => {
+                    Look::macos()
+                }
                 Self::OmarchyWorkspace | Self::OmarchyPicker | Self::OmarchyDialog => {
                     Look::omarchy()
                 }
@@ -765,7 +771,9 @@ mod mock {
         /// a 10 pt wallpaper margin and Hyprland's 2 pt border.
         pub fn size(self) -> egui::Vec2 {
             match self {
-                Self::MacWorkspace | Self::MacPicker | Self::MacDialog => egui::vec2(1440.0, 900.0),
+                Self::MacWorkspace | Self::MacWorkspaceDark | Self::MacPicker | Self::MacDialog => {
+                    egui::vec2(1440.0, 900.0)
+                }
                 Self::OmarchyWorkspace => egui::vec2(1896.0, 1056.0),
                 Self::OmarchyPicker | Self::OmarchyDialog => egui::vec2(936.0, 1016.0),
             }
@@ -775,7 +783,9 @@ mod mock {
         /// line up with them.
         pub fn design_scale(self) -> f32 {
             match self {
-                Self::MacWorkspace | Self::MacPicker | Self::MacDialog => 2000.0 / 1440.0,
+                Self::MacWorkspace | Self::MacWorkspaceDark | Self::MacPicker | Self::MacDialog => {
+                    2000.0 / 1440.0
+                }
                 Self::OmarchyWorkspace => 2000.0 / 1920.0,
                 Self::OmarchyPicker | Self::OmarchyDialog => 1846.0 / 960.0,
             }
@@ -786,6 +796,7 @@ mod mock {
         pub fn palette(self) -> Palette {
             match self {
                 Self::MacWorkspace | Self::MacPicker | Self::MacDialog => Palette::light(),
+                Self::MacWorkspaceDark => Palette::dark(),
                 Self::OmarchyWorkspace | Self::OmarchyPicker | Self::OmarchyDialog => tokyo_night(),
             }
         }
@@ -794,7 +805,7 @@ mod mock {
         pub fn stage(self, harness: &mut Harness) {
             crate::util::pin_now(Some(NOW));
             match self {
-                Self::MacWorkspace => {
+                Self::MacWorkspace | Self::MacWorkspaceDark => {
                     let tab = workspace(harness);
                     production_beside(harness, tab);
                 }
````

- [ ] **Step 4: Render, and look**

Run: `~/.cargo/bin/cargo test --locked --features shots --lib shots -- --ignored`

`target/shots/mock-macos-workspace-dark.png` beside the artboard "macOS – Dark mode": the content lighter than the sidebar and the grid header; the green-tinted bar with its faint white faces; the selected row and sidebar item in `#2b3550` with light blue names; the primary "SQL Editor" button light with dark text; the JSON in light blue, green and orange. Also open `workspace-macos-dark.png`, `dialog-macos-dark.png` and `picker-macos-dark.png` and check nothing reads as a hole or loses its edge.

Put right what differs, in the code this task wrote; keep the tests.

- [ ] **Step 5: Format and lint**

```bash
~/.cargo/bin/cargo fmt --all --check
~/.cargo/bin/cargo clippy --locked --workspace --all-targets -- -D warnings
~/.cargo/bin/cargo clippy --locked --workspace --all-targets --features shots -- -D warnings
```

Expected: no output.

- [ ] **Step 6: Commit point**

Stop and report. If the user has asked for commits:

```bash
git add -A src AGENTS.md
git commit -S -m "Wash the dark bar's faces with white" -m "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

### Task 5: Words for values with nothing to show

**Files:**
- Modify: `src/ui/format.rs`

The pure helpers the grid and the row panel will draw with, in `src/ui/format.rs`, each with its tests: the marks for blank text and line breaks, PostgreSQL arrays, and what is said of a binary value. Nothing draws with them yet, so the app looks the same after this task, apart from one thing: the row panel shows a binary value as its first 24 bytes (decision 5), and `hex_dump`, `HEX_LIMIT` and their two tests are removed.

- [ ] **Step 1: Write the tests**

New tests: `a_cell_keeps_its_line_breaks_in_sight`, `text_with_nothing_to_see_says_what_it_is`, `the_marks_fall_back_when_a_font_lacks_the_designs`, `arrays_are_told_by_their_type`, `an_arrays_elements_are_read_as_postgres_writes_them`, `a_cut_array_keeps_the_elements_that_are_whole`, `text_that_is_no_array_is_left_alone`, `a_long_line_is_long_too_and_counted_in_characters`, `a_binary_field_is_its_first_bytes`, `a_binary_value_is_guessed_by_how_it_starts`, `a_binary_cell_says_its_type_and_size`. Removed with what they tested: `hex_dumps_have_offsets_and_sixteen_bytes_a_line`, `hex_dumps_stop_at_the_limit`.

````diff
diff --git a/src/ui/format.rs b/src/ui/format.rs
--- a/src/ui/format.rs
+++ b/src/ui/format.rs
@@ -644,6 +901,113 @@ mod tests {
         assert_eq!(cell_text(&text("a\nb\tc\r\nd")), "a b c  d");
     }
 
+    #[test]
+    fn a_cell_keeps_its_line_breaks_in_sight() {
+        let marks = Marks::DESIGN;
+        assert_eq!(
+            cell_line("Night Train\nPart One", marks),
+            "Night Train↵Part One"
+        );
+        // A Windows line end is one break; a tab is a space.
+        assert_eq!(cell_line("a\r\nb\rc\td", marks), "a↵b↵c d");
+        assert_eq!(cell_line("a\nb", Marks::PLAIN), "a¶b");
+        // Other hidden characters are still written out, and plain text is
+        // not copied.
+        assert_eq!(cell_line("a\u{200B}b", marks), "a<U+200B>b");
+        assert!(matches!(cell_line("plain", marks), Cow::Borrowed(_)));
+        let long = "y".repeat(CELL_MAX_CHARS + 10);
+        assert!(cell_line(&long, marks).ends_with('…'));
+    }
+
+    #[test]
+    fn text_with_nothing_to_see_says_what_it_is() {
+        let marks = Marks::DESIGN;
+        assert_eq!(blank_text("", marks).as_deref(), Some("''"));
+        assert_eq!(blank_text("  ", marks).as_deref(), Some("␣␣"));
+        assert_eq!(blank_text(" \t\r\n", marks).as_deref(), Some("␣␣↵"));
+        assert_eq!(blank_text("  ", Marks::PLAIN).as_deref(), Some("··"));
+        // Leading spaces are the text's own: nothing stands in for them.
+        assert_eq!(blank_text("   Leading spaces kept", marks), None);
+        assert_eq!(blank_text("a", marks), None);
+        // A megabyte of spaces is cut like any text, without reading it all.
+        let spaces = " ".repeat(1_000_000);
+        let shown = blank_text(&spaces, marks).unwrap();
+        assert_eq!(shown.chars().count(), CELL_MAX_CHARS + 1);
+        assert!(shown.ends_with('…'));
+    }
+
+    #[test]
+    fn the_marks_fall_back_when_a_font_lacks_the_designs() {
+        assert_eq!(Marks::pick(|_| true), Marks::DESIGN);
+        assert_eq!(Marks::pick(|character| character != '↵'), Marks::PLAIN);
+        assert_eq!(Marks::pick(|_| false), Marks::PLAIN);
+    }
+
+    #[test]
+    fn arrays_are_told_by_their_type() {
+        assert!(is_array("_text", ValueKind::Other));
+        assert!(is_array("int4[]", ValueKind::Other));
+        // A type the app knows is never an array, whatever its name.
+        assert!(!is_array("_text", ValueKind::Text));
+        assert!(!is_array("_", ValueKind::Other));
+        assert!(!is_array("int4range", ValueKind::Other));
+        assert_eq!(type_label("_text", ValueKind::Other), "text[]");
+        assert_eq!(type_label("text[]", ValueKind::Other), "text[]");
+        assert_eq!(type_label("int8", ValueKind::Numeric), "int8");
+    }
+
+    fn items(text: &str) -> Option<(Vec<String>, bool)> {
+        array_items(text).map(|array| {
+            let items = array.items.iter().map(|item| item.to_string()).collect();
+            (items, array.cut)
+        })
+    }
+
+    #[test]
+    fn an_arrays_elements_are_read_as_postgres_writes_them() {
+        let some =
+            |list: &[&str], cut| Some((list.iter().map(|item| (*item).to_owned()).collect(), cut));
+        assert_eq!(items("{en,fr}"), some(&["en", "fr"], false));
+        assert_eq!(items("{}"), some(&[], false));
+        assert_eq!(items("{1}"), some(&["1"], false));
+        // Quotes go, and what they escape stays.
+        assert_eq!(
+            items(r#"{"two words",NULL,"a \"b\" c\\d","x,y}"}"#),
+            some(&["two words", "NULL", r#"a "b" c\d"#, "x,y}"], false)
+        );
+        assert_eq!(items("{\"Ærø\",東京}"), some(&["Ærø", "東京"], false));
+        // An array in an array stays as it is written.
+        assert_eq!(
+            items(r#"{{1,2},{"a}",b}}"#),
+            some(&["{1,2}", r#"{"a}",b}"#], false)
+        );
+    }
+
+    #[test]
+    fn a_cut_array_keeps_the_elements_that_are_whole() {
+        let some = |list: &[&str]| {
+            Some((
+                list.iter()
+                    .map(|item| (*item).to_owned())
+                    .collect::<Vec<_>>(),
+                true,
+            ))
+        };
+        assert_eq!(items("{en,fr,d…"), some(&["en", "fr"]));
+        assert_eq!(items("{en,fr,"), some(&["en", "fr"]));
+        assert_eq!(items("{en,fr"), some(&["en"]));
+        assert_eq!(items(r#"{en,"fr…"#), some(&["en"]));
+        assert_eq!(items("{{1,2},{3…"), some(&["{1,2}"]));
+        assert_eq!(items("{"), some(&[]));
+    }
+
+    #[test]
+    fn text_that_is_no_array_is_left_alone() {
+        for text in ["", "en,fr", "[0:1]={a,b}", "{a}b", "{,a}", "{a,,b}", "{a}}"] {
+            assert_eq!(items(text), None, "{text:?}");
+        }
+    }
+
     #[test]
     fn short_text_is_not_copied() {
         let value = text("borrowed");
@@ -684,23 +1048,63 @@ mod tests {
             (short.short.as_str(), short.full, short.size.as_str()),
             ("hi", None, "2 B")
         );
+        assert_eq!(short.characters, Some(2));
+        assert_eq!(field_text(&Value::Int(42)).characters, None);
     }
 
     #[test]
-    fn hex_dumps_have_offsets_and_sixteen_bytes_a_line() {
-        let bytes: Vec<u8> = (0u8..20).collect();
+    fn a_long_line_is_long_too_and_counted_in_characters() {
+        let title = "é".repeat(5_000);
+        let field = field_text(&text(&title));
+        // Its start, broken into lines the panel can lay out.
+        let start = field.short.chars().filter(|c| *c != '\n').count();
+        assert_eq!(start, COLLAPSE_CHARS);
+        assert_eq!(field.characters, Some(5_000));
+        assert!(field.full.is_some());
+    }
+
+    #[test]
+    fn a_binary_field_is_its_first_bytes() {
+        let jpeg: Vec<u8> = [0xff, 0xd8, 0xff, 0xe0, 0x00, 0x10]
+            .into_iter()
+            .chain(std::iter::repeat_n(0xab, 49_352))
+            .collect();
+        let field = field_text(&Value::Bytes(jpeg.clone().into()));
         assert_eq!(
-            hex_dump(&bytes),
-            "00000000  00 01 02 03 04 05 06 07 08 09 0a 0b 0c 0d 0e 0f\n00000010  10 11 12 13"
+            field.short,
+            "ff d8 ff e0 00 10 ab ab ab ab ab ab\nab ab ab ab ab ab ab ab ab ab ab ab"
         );
+        assert_eq!((field.full, field.size.as_str()), (None, "48.2 KB"));
+        assert_eq!(hex_preview(&[0x00, 0x0f]), "00 0f");
+        assert_eq!(hex_preview(&[]), "");
+        // A huge value is never read past its first bytes.
+        let started = std::time::Instant::now();
+        let huge = Value::Bytes(vec![0xab; 10 * 1024 * 1024].into());
+        assert_eq!(field_text(&huge).short.len(), HEX_PREVIEW * 3 - 1);
+        assert!(started.elapsed() < Duration::from_secs(1));
     }
 
     #[test]
-    fn hex_dumps_stop_at_the_limit() {
-        let full = full_text(&Value::Bytes(vec![0xab; 10 * 1024 * 1024].into())).into_owned();
-        assert!(full.starts_with("10.0 MB\n"));
-        assert!(full.ends_with('…'));
-        assert!(full.lines().count() <= HEX_LIMIT / 16 + 3);
+    fn a_binary_value_is_guessed_by_how_it_starts() {
+        assert_eq!(sniff(&[0xff, 0xd8, 0xff, 0xe0]), Some("JPEG"));
+        assert_eq!(sniff(b"\x89PNG\r\n\x1a\n...."), Some("PNG"));
+        assert_eq!(sniff(b"GIF89a.."), Some("GIF"));
+        assert_eq!(sniff(b"%PDF-1.7"), Some("PDF"));
+        assert_eq!(sniff(b"PK\x03\x04"), Some("ZIP"));
+        assert_eq!(sniff(b"RIFF\x10\0\0\0WEBPVP8 "), Some("WebP"));
+        assert_eq!(sniff(b"RIFF\x10\0\0\0WAVE"), None);
+        assert_eq!(sniff(b""), None);
+        assert_eq!(sniff(b"plain text"), None);
+    }
+
+    #[test]
+    fn a_binary_cell_says_its_type_and_size() {
+        assert_eq!(binary_label("bytea", 49_358, false), "bytea · 48.2 KB");
+        assert_eq!(binary_label("bytea", 49_358, true), "bytea 48.2K");
+        assert_eq!(binary_label("BLOB", 0, false), "BLOB · 0 B");
+        assert_eq!(binary_label("BLOB", 0, true), "BLOB 0B");
+        assert_eq!(binary_label("", 3, false), "binary · 3 B");
+        assert_eq!(terse_size(5 * 1024 * 1024), "5.0M");
     }
 
     #[test]
````

- [ ] **Step 2: Run them and see them fail**

Run: `~/.cargo/bin/cargo test --locked --lib`

Expected, before the code below is there:

````text
Does not compile:
error[E0433]: cannot find type `Marks` in this scope
error[E0425]: cannot find value `HEX_PREVIEW` in this scope
error[E0425]: cannot find function `cell_line` in this scope
error[E0425]: cannot find function `blank_text` in this scope
error[E0433]: cannot find type `ValueKind` in this scope
error[E0425]: cannot find function `is_array` in this scope
(and 7 more of the same kind)
````

- [ ] **Step 3: Write the code**

````diff
diff --git a/src/ui/format.rs b/src/ui/format.rs
--- a/src/ui/format.rs
+++ b/src/ui/format.rs
@@ -4,16 +4,17 @@ use std::borrow::Cow;
 use std::fmt::Write as _;
 use std::time::Duration;
 
-use tabletist_db::Value;
+use tabletist_db::{Value, ValueKind};
 
 /// Characters a grid cell shows before cutting the value off.
 pub const CELL_MAX_CHARS: usize = 256;
-/// Bytes of a binary value the row panel dumps as hex.
-pub const HEX_LIMIT: usize = 4096;
-/// Lines of a long value the row panel shows before "Show all".
+/// Bytes of a binary value the row panel shows as hex: two lines of twelve.
+pub const HEX_PREVIEW: usize = 24;
+/// Lines of a long value the row panel lays out before "Show all". It
+/// shows the first three of them (see `row_panel`); this bounds the layout.
 pub const COLLAPSE_LINES: usize = 20;
-/// Characters of a long value the row panel shows before "Show all", so a
-/// huge single-line value is never laid out whole.
+/// Characters of a long value the row panel lays out before "Show all", so
+/// a huge single-line value is never laid out whole.
 pub const COLLAPSE_CHARS: usize = 4_000;
 
 /// One short line for a grid cell.
@@ -28,10 +29,79 @@ pub fn cell_text(value: &Value) -> Cow<'_, str> {
     }
 }
 
-/// Text on one line, as a grid cell shows it: line breaks and tabs become
-/// spaces, hidden characters are written out.
+/// Text on one line: line breaks and tabs become spaces, hidden characters
+/// are written out. For text that is a part of something else (a
+/// document's first string); a cell's own text keeps its breaks in sight
+/// (see [`cell_line`]).
 pub fn one_line(text: &str) -> Cow<'_, str> {
-    bounded_line(text, CELL_MAX_CHARS, true)
+    bounded_line(text, CELL_MAX_CHARS, Breaks::Space)
+}
+
+/// The marks a cell writes where text would show nothing.
+#[derive(Clone, Copy, Debug, PartialEq, Eq)]
+pub struct Marks {
+    /// One for each character of a value that is only whitespace.
+    pub space: char,
+    /// In place of a line break.
+    pub line: char,
+}
+
+impl Marks {
+    /// The design's marks. IBM Plex and JetBrains Mono have neither: they
+    /// come from a font of the system's, as they do in a browser.
+    pub const DESIGN: Self = Self {
+        space: '␣',
+        line: '↵',
+    };
+    /// Marks every bundled face draws, where no font has the design's.
+    pub const PLAIN: Self = Self {
+        space: '·',
+        line: '¶',
+    };
+
+    /// The design's marks when `has_glyph` finds both, else the plain ones.
+    pub fn pick(mut has_glyph: impl FnMut(char) -> bool) -> Self {
+        if has_glyph(Self::DESIGN.space) && has_glyph(Self::DESIGN.line) {
+            Self::DESIGN
+        } else {
+            Self::PLAIN
+        }
+    }
+}
+
+/// A cell's text on one line: a line break becomes `marks.line` (a Windows
+/// line end is one break), a tab a space, hidden characters are written out.
+pub fn cell_line(text: &str, marks: Marks) -> Cow<'_, str> {
+    bounded_line(text, CELL_MAX_CHARS, Breaks::Mark(marks.line))
+}
+
+/// What a cell shows for text with nothing to see: `''` for an empty
+/// string, a mark for each character of one that is only whitespace. `None`
+/// for text that shows as itself.
+pub fn blank_text(text: &str, marks: Marks) -> Option<String> {
+    if text.is_empty() {
+        return Some("''".to_owned());
+    }
+    let mut shown = String::new();
+    let mut characters = text.chars().peekable();
+    let mut count = 0;
+    while let Some(character) = characters.next() {
+        if !character.is_whitespace() {
+            return None;
+        }
+        // A cell's worth is all that is looked at, as for any text.
+        if count == CELL_MAX_CHARS {
+            shown.push('…');
+            break;
+        }
+        count += 1;
+        match character {
+            '\r' if characters.peek() == Some(&'\n') => count -= 1,
+            '\n' | '\r' => shown.push(marks.line),
+            _ => shown.push(marks.space),
+        }
+    }
+    Some(shown)
 }
 
 /// Characters of a name (schema, table, column, database) the UI lays out.
@@ -44,13 +114,13 @@ pub const NAME_MAX_CHARS: usize = 256;
 /// for `users_data`, nor `users\u{200B}` for `users`. Copying keeps the name
 /// as it is.
 pub fn display_safe(name: &str) -> Cow<'_, str> {
-    bounded_line(name, NAME_MAX_CHARS, false)
+    bounded_line(name, NAME_MAX_CHARS, Breaks::Escape)
 }
 
 /// `text` with every hidden character written out, as in `display_safe`,
 /// and not cut: for text already cut to a size.
 pub fn escape_hidden(text: &str) -> Cow<'_, str> {
-    bounded_line(text, usize::MAX, false)
+    bounded_line(text, usize::MAX, Breaks::Escape)
 }
 
 /// An object as its tab and the row panel name it: the name, or
@@ -64,10 +134,21 @@ pub fn object_title(object: &tabletist_db::ObjectRef, qualified: bool) -> String
     }
 }
 
+/// What [`bounded_line`] does with a line break.
+#[derive(Clone, Copy)]
+enum Breaks {
+    /// Written out, as any hidden character (a name has none of its own).
+    Escape,
+    /// A space, and a tab too.
+    Space,
+    /// This mark; a tab is a space.
+    Mark(char),
+}
+
 /// The first `max` characters of `text` with hidden characters written out
-/// and "…" when cut. `spaces` turns line breaks and tabs into spaces instead.
-/// Borrows when there is nothing to change.
-fn bounded_line(text: &str, max: usize, spaces: bool) -> Cow<'_, str> {
+/// and "…" when cut; line breaks and tabs as `breaks` says. Borrows when
+/// there is nothing to change.
+fn bounded_line(text: &str, max: usize, breaks: Breaks) -> Cow<'_, str> {
     // Only the first `max` characters are ever shown, so never look past
     // them: a cell may hold megabytes and is drawn every frame.
     let (end, too_long) = match text.char_indices().nth(max) {
@@ -79,13 +160,15 @@ fn bounded_line(text: &str, max: usize, spaces: bool) -> Cow<'_, str> {
         return Cow::Borrowed(text);
     }
     let mut line = String::with_capacity(head.len() + 16);
-    for character in head.chars() {
-        if spaces && matches!(character, '\n' | '\r' | '\t') {
-            line.push(' ');
-        } else if is_hidden(character) {
-            push_escaped(&mut line, character);
-        } else {
-            line.push(character);
+    let mut characters = head.chars().peekable();
+    while let Some(character) = characters.next() {
+        match (breaks, character) {
+            (Breaks::Space, '\n' | '\r' | '\t') | (Breaks::Mark(_), '\t') => line.push(' '),
+            // A Windows line end is one break: its LF writes the mark.
+            (Breaks::Mark(_), '\r') if characters.peek() == Some(&'\n') => {}
+            (Breaks::Mark(mark), '\n' | '\r') => line.push(mark),
+            _ if is_hidden(character) => push_escaped(&mut line, character),
+            _ => line.push(character),
         }
     }
     if too_long {
@@ -133,6 +216,129 @@ fn push_escaped(out: &mut String, character: char) {
     let _ = write!(out, "<U+{:04X}>", u32::from(character));
 }
 
+/// Whether a column of this type and kind holds PostgreSQL arrays: a result
+/// names their type `_text`, the catalog `text[]`. No other database has
+/// them.
+pub fn is_array(type_name: &str, kind: ValueKind) -> bool {
+    kind == ValueKind::Other
+        && (type_name.ends_with("[]")
+            || type_name
+                .strip_prefix('_')
+                .is_some_and(|element| !element.is_empty()))
+}
+
+/// A column's type as people write it: an array's `_text` is `text[]`.
+pub fn type_label(type_name: &str, kind: ValueKind) -> Cow<'_, str> {
+    match type_name.strip_prefix('_') {
+        Some(element) if is_array(type_name, kind) => Cow::Owned(format!("{element}[]")),
+        _ => Cow::Borrowed(type_name),
+    }
+}
+
+/// A PostgreSQL array's elements.
+#[derive(Debug, Clone, PartialEq)]
+pub struct ArrayItems<'a> {
+    /// Each element as its text: a quoted one without its quotes, an array
+    /// inside the array as it is written.
+    pub items: Vec<Cow<'a, str>>,
+    /// The text ended before the array did (a cell's text is cut): there
+    /// are more elements than these.
+    pub cut: bool,
+}
+
+/// The elements of an array as PostgreSQL writes one: `{en,fr}`,
+/// `{"two words",NULL}`, `{{1,2},{3,4}}`. `None` for text that is no array,
+/// one with its bounds written before it (`[0:1]={a,b}`) included.
+pub fn array_items(text: &str) -> Option<ArrayItems<'_>> {
+    let bytes = text.as_bytes();
+    if bytes.first() != Some(&b'{') {
+        return None;
+    }
+    let mut items = Vec::new();
+    if text == "{}" {
+        return Some(ArrayItems { items, cut: false });
+    }
+    // The delimiters are ASCII, so stepping by bytes never splits a
+    // character, and every slice below starts and ends on one.
+    let mut at = 1;
+    loop {
+        let start = at;
+        let item = match bytes.get(at) {
+            None => return Some(ArrayItems { items, cut: true }),
+            Some(b'"') => {
+                at += 1;
+                let from = at;
+                // Copied only when an escape makes it differ from the text.
+                let mut unescaped: Option<String> = None;
+                loop {
+                    let Some(character) = text[at..].chars().next() else {
+                        return Some(ArrayItems { items, cut: true });
+                    };
+                    at += character.len_utf8();
+                    match character {
+                        '"' => break,
+                        '\\' => {
+                            let Some(next) = text[at..].chars().next() else {
+                                return Some(ArrayItems { items, cut: true });
+                            };
+                            unescaped
+                                .get_or_insert_with(|| text[from..at - 1].to_owned())
+                                .push(next);
+                            at += next.len_utf8();
+                        }
+                        _ => {
+                            if let Some(copy) = &mut unescaped {
+                                copy.push(character);
+                            }
+                        }
+                    }
+                }
+                unescaped.map_or(Cow::Borrowed(&text[from..at - 1]), Cow::Owned)
+            }
+            Some(b'{') => {
+                let (mut depth, mut quoted) = (0, false);
+                loop {
+                    match bytes.get(at) {
+                        None => return Some(ArrayItems { items, cut: true }),
+                        Some(b'\\') if quoted => at += 1,
+                        Some(b'"') => quoted = !quoted,
+                        Some(b'{') if !quoted => depth += 1,
+                        Some(b'}') if !quoted => depth -= 1,
+                        Some(_) => {}
+                    }
+                    at += 1;
+                    if depth == 0 {
+                        break;
+                    }
+                }
+                Cow::Borrowed(&text[start..at.min(text.len())])
+            }
+            Some(_) => {
+                while bytes
+                    .get(at)
+                    .is_some_and(|byte| !matches!(byte, b',' | b'}'))
+                {
+                    at += 1;
+                }
+                if at == bytes.len() {
+                    return Some(ArrayItems { items, cut: true });
+                }
+                if at == start {
+                    return None;
+                }
+                Cow::Borrowed(&text[start..at])
+            }
+        };
+        items.push(item);
+        match bytes.get(at) {
+            Some(b',') => at += 1,
+            Some(b'}') if at + 1 == bytes.len() => return Some(ArrayItems { items, cut: false }),
+            None => return Some(ArrayItems { items, cut: true }),
+            Some(_) => return None,
+        }
+    }
+}
+
 /// The colour a `#rgb`, `#rgba`, `#rrggbb` or `#rrggbbaa` value names, as
 /// red, green, blue and alpha (opaque when the value has none).
 pub fn hex_color(text: &str) -> Option<[u8; 4]> {
@@ -238,21 +444,14 @@ thread_local! {
 }
 
 /// The whole value as the row panel shows it as text. JSON the panel can
-/// parse is drawn as a tree instead (`json_view`). Text is borrowed, not
-/// copied.
+/// parse is drawn as a tree instead (`json_view`), and a binary value as
+/// its first bytes ([`hex_preview`]). Text is borrowed, not copied.
 pub fn full_text(value: &Value) -> Cow<'_, str> {
     #[cfg(test)]
     FULL_TEXTS.with(|count| count.set(count.get() + 1));
     match value {
         Value::Text(text) => Cow::Borrowed(text),
-        Value::Bytes(bytes) => {
-            let shown = &bytes[..bytes.len().min(HEX_LIMIT)];
-            let mut text = format!("{}\n{}", human_size(bytes.len()), hex_dump(shown));
-            if bytes.len() > HEX_LIMIT {
-                text.push_str("\n…");
-            }
-            Cow::Owned(text)
-        }
+        Value::Bytes(bytes) => Cow::Owned(hex_preview(bytes)),
         other => Cow::Owned(plain_text(other)),
     }
 }
@@ -266,19 +465,33 @@ pub struct FieldText {
     pub full: Option<String>,
     /// The value's size, for "Show all".
     pub size: String,
+    /// How many characters a text value has, for its label.
+    pub characters: Option<usize>,
 }
 
 /// Formats a value for the row panel. Slow for a big value (it reads all of
 /// it), so the app calls it once per selected row, not every frame.
 pub fn field_text(value: &Value) -> FieldText {
     let text = full_text(value);
+    // A binary value is its first bytes and nothing to unfold: the whole
+    // of it is a file's worth, and copying or saving gives it.
+    if let Value::Bytes(bytes) = value {
+        return FieldText {
+            short: text.into_owned(),
+            full: None,
+            size: human_size(bytes.len()),
+            characters: None,
+        };
+    }
     let long = text.len() > COLLAPSE_CHARS || text.lines().nth(COLLAPSE_LINES).is_some();
     let size = human_size(text.len());
+    let characters = matches!(value, Value::Text(_)).then(|| text.chars().count());
     if !long {
         return FieldText {
             short: for_display(&text),
             full: None,
             size,
+            characters,
         };
     }
     let start: String = text
@@ -293,24 +506,68 @@ pub fn field_text(value: &Value) -> FieldText {
         short: for_display(&start),
         full: Some(for_display(&text)),
         size,
+        characters,
     }
 }
 
-/// `00000000  00 01 02 ...`, sixteen bytes a line.
-pub fn hex_dump(bytes: &[u8]) -> String {
+/// The first [`HEX_PREVIEW`] bytes as `ff d8 ff ...`, twelve to a line.
+pub fn hex_preview(bytes: &[u8]) -> String {
     let mut out = String::new();
-    for (line, chunk) in bytes.chunks(16).enumerate() {
-        if line > 0 {
-            out.push('\n');
-        }
-        let _ = write!(out, "{:08x} ", line * 16);
-        for byte in chunk {
-            let _ = write!(out, " {byte:02x}");
+    for (index, byte) in bytes.iter().take(HEX_PREVIEW).enumerate() {
+        if index > 0 {
+            out.push(if index % 12 == 0 { '\n' } else { ' ' });
         }
+        let _ = write!(out, "{byte:02x}");
     }
     out
 }
 
+/// What a binary value starts as, by the bytes files of that kind begin
+/// with. A guess, and said as one ("looks like JPEG").
+pub fn sniff(bytes: &[u8]) -> Option<&'static str> {
+    const SIGNS: [(&[u8], &str); 8] = [
+        (b"\xff\xd8\xff", "JPEG"),
+        (b"\x89PNG\r\n\x1a\n", "PNG"),
+        (b"GIF87a", "GIF"),
+        (b"GIF89a", "GIF"),
+        (b"%PDF-", "PDF"),
+        (b"PK\x03\x04", "ZIP"),
+        (b"\x1f\x8b", "gzip"),
+        (b"SQLite format 3\0", "SQLite"),
+    ];
+    if bytes.len() >= 12 && bytes.starts_with(b"RIFF") && &bytes[8..12] == b"WEBP" {
+        return Some("WebP");
+    }
+    SIGNS
+        .iter()
+        .find(|(sign, _)| bytes.starts_with(sign))
+        .map(|(_, name)| *name)
+}
+
+/// What a grid cell says of a binary value: its type and its size, never
+/// its bytes. `bytea · 48.2 KB`, or the terminal's `bytea 48.2K`.
+pub fn binary_label(type_name: &str, bytes: usize, terminal: bool) -> String {
+    let name = if type_name.is_empty() {
+        "binary"
+    } else {
+        type_name
+    };
+    let name = display_safe(name);
+    if terminal {
+        format!("{name} {}", terse_size(bytes))
+    } else {
+        format!("{name} · {}", human_size(bytes))
+    }
+}
+
+/// [`human_size`] as the terminal writes it: `0B`, `48.2K`, `2.1M`.
+pub fn terse_size(bytes: usize) -> String {
+    human_size(bytes)
+        .replace(" KB", "K")
+        .replace(" MB", "M")
+        .replace(" B", "B")
+}
+
 pub fn human_size(bytes: usize) -> String {
     const KB: f64 = 1024.0;
     let size = bytes as f64;
````

- [ ] **Step 4: Run the tests**

Run: `~/.cargo/bin/cargo test --locked --lib ui::format`
Expected: PASS.

Then all of the library's: `~/.cargo/bin/cargo test --locked --lib`
Expected: `test result: ok. 863 passed; 0 failed; 1 ignored`.

- [ ] **Step 5: Format and lint**

```bash
~/.cargo/bin/cargo fmt --all --check
~/.cargo/bin/cargo clippy --locked --workspace --all-targets -- -D warnings
~/.cargo/bin/cargo clippy --locked --workspace --all-targets --features shots -- -D warnings
```

Expected: no output.

- [ ] **Step 6: Commit point**

Stop and report. If the user has asked for commits:

```bash
git add -A src AGENTS.md
git commit -S -m "Find words for values with nothing to show" -m "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

### Task 6: Booleans are tags

**Files:**
- Modify: `src/ui/value_tags.rs`
- Modify: `src/ui/grid.rs`
- Modify: `src/ui/sql_results.rs` (a test)
- Modify: `src/ui/data_view.rs` (a test)

A boolean column is a closed set of two: true takes the first slot and false the second, as the artboards draw them in every look (tan and blue chips on macOS, orange and cyan text in the terminal). `Style::True` and `Style::False` go; `Style::Tag` does their work.

- [ ] **Step 1: Write the tests**

`booleans_are_a_neutral_tag_or_muted` is rewritten as `booleans_are_the_first_two_tags` (the behaviour it pinned is the one this task changes); `a_results_booleans_read_as_a_tables_do` and one line of `a_tag_comes_after_null_and_a_colour_and_before_the_rest` follow.

````diff
diff --git a/src/ui/data_view.rs b/src/ui/data_view.rs
--- a/src/ui/data_view.rs
+++ b/src/ui/data_view.rs
@@ -1548,7 +1548,7 @@ mod tests {
         );
         assert_eq!(
             style(&Value::Bool(true), ValueKind::Bool, &Tags::Bool),
-            (false, Style::True)
+            (false, Style::Tag(0))
         );
     }
 
diff --git a/src/ui/sql_results.rs b/src/ui/sql_results.rs
--- a/src/ui/sql_results.rs
+++ b/src/ui/sql_results.rs
@@ -1344,7 +1344,7 @@ mod tests {
         };
         harness.answer_sql(Ok(script_outcome(vec![flags])), None);
         harness.settle();
-        // A false flag is muted; the same word as text is not a flag.
+        // A false flag is the second tag; the same word as text is no flag.
         let palette = harness.app.palette;
         let colors: Vec<egui::Color32> = harness
             .painted
@@ -1352,7 +1352,8 @@ mod tests {
             .filter(|(piece, _)| piece == "false")
             .map(|(_, color)| *color)
             .collect();
-        assert_eq!(colors, [palette.dim, palette.text]);
+        let (tag, _) = crate::ui::value_tags::slot_colors(1, &harness.app.look, &palette);
+        assert_eq!(colors, [tag, palette.text]);
     }
 
     #[test]
diff --git a/src/ui/value_tags.rs b/src/ui/value_tags.rs
--- a/src/ui/value_tags.rs
+++ b/src/ui/value_tags.rs
@@ -284,25 +283,25 @@ mod tests {
     }
 
     #[test]
-    fn booleans_are_a_neutral_tag_or_muted() {
+    fn booleans_are_the_first_two_tags() {
         let tags = Tags::of(&meta("done", ValueKind::Bool), None);
         assert_eq!(tags, Tags::Bool);
-        assert_eq!(tags.style(&Value::Bool(true)), Some(Style::True));
-        assert_eq!(tags.style(&Value::Bool(false)), Some(Style::False));
-        assert_eq!(tags.style(&Value::Int(1)), Some(Style::True), "SQLite");
+        assert_eq!(tags.style(&Value::Bool(true)), Some(Style::Tag(0)));
+        assert_eq!(tags.style(&Value::Bool(false)), Some(Style::Tag(1)));
+        assert_eq!(tags.style(&Value::Int(1)), Some(Style::Tag(0)), "SQLite");
+        assert_eq!(tags.style(&Value::Int(0)), Some(Style::Tag(1)), "SQLite");
         assert_eq!(tags.style(&Value::Int(2)), None);
+        // Neither reads as an environment: no red, no green.
         for look in Look::ALL {
             for palette in [Palette::light(), Palette::dark()] {
-                let (text, fill) = style_colors(Style::True, &look, &palette);
-                assert_eq!(text, palette.text);
-                assert_eq!(fill.is_some(), !look.terminal);
-                assert_eq!(
-                    style_colors(Style::False, &look, &palette),
-                    (palette.dim, None)
-                );
-                for color in [text, fill.unwrap_or(text)] {
-                    assert_ne!(color, palette.danger, "no red");
-                    assert_ne!(color, palette.success, "no green");
+                for flag in [true, false] {
+                    let style = tags.style(&Value::Bool(flag)).unwrap();
+                    let (text, fill) = style_colors(style, &look, &palette);
+                    assert_eq!(fill.is_some(), !look.terminal);
+                    for color in [text, fill.unwrap_or(text)] {
+                        assert_ne!(color, palette.danger, "no red");
+                        assert_ne!(color, palette.success, "no green");
+                    }
                 }
             }
         }
````

- [ ] **Step 2: Run them and see them fail**

Run: `~/.cargo/bin/cargo test --locked --lib`

Expected, before the code below is there:

````text
test ui::data_view::tests::a_tag_comes_after_null_and_a_colour_and_before_the_rest ... FAILED
test ui::sql_results::tests::a_results_booleans_read_as_a_tables_do ... FAILED
test ui::value_tags::tests::booleans_are_the_first_two_tags ... FAILED
test result: FAILED. 860 passed; 3 failed; 1 ignored; 0 measured; 0 filtered out
````

- [ ] **Step 3: Write the code**

````diff
diff --git a/src/ui/grid.rs b/src/ui/grid.rs
--- a/src/ui/grid.rs
+++ b/src/ui/grid.rs
@@ -64,10 +64,6 @@ pub enum Style {
     /// A value from a column's allowed list, in that palette slot (see
     /// [`crate::ui::value_tags`]).
     Tag(usize),
-    /// A true boolean: a neutral tag.
-    True,
-    /// A false boolean: muted text.
-    False,
     /// A JSON document with this many keys: a `{ n }` chip, then the text.
     Json(usize),
     /// A colour (`#3a7bd5`): a swatch of it, then the text.
@@ -273,8 +269,8 @@ pub fn initial_widths<'a>(
                 .map(|row| {
                     let cell = cell(row, col);
                     let chip = match cell.style {
-                        Style::Plain | Style::False => 0.0,
-                        Style::Tag(_) | Style::True => 16.0,
+                        Style::Plain => 0.0,
+                        Style::Tag(_) => 16.0,
                         Style::Json(_) => 44.0,
                         Style::Color(_) => SWATCH + SWATCH_GAP,
                     };
@@ -787,11 +783,11 @@ fn draw_cell(
         return;
     }
     match content.style {
-        Style::Tag(_) | Style::True | Style::False => {
+        Style::Tag(_) => {
             let (color, fill) = crate::ui::value_tags::style_colors(content.style, look, palette);
             // macOS and Windows: a chip in the value-tag face, 2 above and
-            // below, 6 at the sides. Terminal (and false): the text alone,
-            // in the tag's colour.
+            // below, 6 at the sides. Terminal: the text alone, in the
+            // tag's colour.
             let Some(fill) = fill else {
                 let shown = ellipsize(&content.text, room, false, |text| width(text, role));
                 paint(
diff --git a/src/ui/value_tags.rs b/src/ui/value_tags.rs
--- a/src/ui/value_tags.rs
+++ b/src/ui/value_tags.rs
@@ -1,7 +1,8 @@
 //! Value tags: the colours of values from a closed set. Only three kinds of
 //! column get them: a PostgreSQL enum, a text column whose CHECK constraint
 //! is a plain value list (both from the catalog, never from the rows), and
-//! booleans. Everything else is plain text, however its values repeat.
+//! booleans (true in the first slot, false in the second). Everything else
+//! is plain text, however its values repeat.
 
 use egui::Color32;
 use tabletist_db::{ColumnMeta, Structure, Value, ValueKind};
@@ -60,8 +61,9 @@ impl<'a> Tags<'a> {
     }
 }
 
+/// A boolean's tag: the two values of a closed set, true first.
 fn bool_style(flag: bool) -> Style {
-    if flag { Style::True } else { Style::False }
+    Style::Tag(usize::from(!flag))
 }
 
 /// The desktop looks' fixed slots, (fill, text) on a light palette. They
@@ -111,9 +113,6 @@ pub fn slot_colors(slot: usize, look: &Look, palette: &Palette) -> (Color32, Opt
 pub fn style_colors(style: Style, look: &Look, palette: &Palette) -> (Color32, Option<Color32>) {
     match style {
         Style::Tag(slot) => slot_colors(slot, look, palette),
-        Style::True if look.terminal => (palette.text, None),
-        Style::True => (palette.text, Some(palette.surface)),
-        Style::False => (palette.dim, None),
         Style::Plain | Style::Json(_) | Style::Color(_) => (palette.text, None),
     }
 }
````

- [ ] **Step 4: Run the tests**

Run: `~/.cargo/bin/cargo test --locked --lib value_tags`
Expected: PASS.

Then all of the library's: `~/.cargo/bin/cargo test --locked --lib`
Expected: `test result: ok. 863 passed; 0 failed; 1 ignored`.

- [ ] **Step 5: Format and lint**

```bash
~/.cargo/bin/cargo fmt --all --check
~/.cargo/bin/cargo clippy --locked --workspace --all-targets -- -D warnings
~/.cargo/bin/cargo clippy --locked --workspace --all-targets --features shots -- -D warnings
```

Expected: no output.

- [ ] **Step 6: Commit point**

Stop and report. If the user has asked for commits:

```bash
git add -A src AGENTS.md
git commit -S -m "Draw booleans as the first two tags" -m "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

### Task 7: Cells for awkward values

**Files:**
- Modify: `src/ui/grid.rs`
- Modify: `src/ui/data_view.rs`
- Modify: `src/ui/sql_results.rs`
- Modify: `src/ui/row_panel.rs` (the label's `text[]`)
- Modify: `src/ui/value_tags.rs` (the new styles' colours)
- Modify: `src/shots.rs`

The grid learns three ways to draw a cell (`Style::Quiet` for text that stands for a value, `Style::Chip` for a note about one, `Style::Array` for a chip per element) and draws NULL as a chip on the desktop looks. `data_view::cell` and `plain_cell` take the result column instead of its kind, because an array and a binary value are told by the column's type. The marks are asked of the fonts once (`grid::marks`, decision 1). The scenes `MacValues` and `OmarchyValues` show a table of such values at the mockups' sizes.

- [ ] **Step 1: Write the tests**

New in `src/ui/data_view.rs`: `a_cell_says_what_a_value_with_nothing_to_show_is` and `an_array_and_a_binary_value_are_told_by_their_column`, with a `context()` that has every look's fonts and has drawn a frame (the marks ask the fonts). The two existing cell tests pass a column where they passed a kind.

````diff
diff --git a/src/ui/data_view.rs b/src/ui/data_view.rs
--- a/src/ui/data_view.rs
+++ b/src/ui/data_view.rs
@@ -1452,12 +1480,109 @@ mod tests {
         }
     }
 
+    /// A context with every look's faces that has drawn a frame: its fonts
+    /// are there to ask for the marks. No system fonts, as in every test.
+    fn context() -> egui::Context {
+        let ctx = egui::Context::default();
+        let mut fonts = fastframe_fonts::FontSetup::default()
+            .system_fallbacks(false)
+            .definitions();
+        for look in Look::ALL {
+            crate::typography::configure(&mut fonts, &look, false);
+        }
+        ctx.set_fonts(fonts);
+        let mut output = ctx.run_ui(egui::RawInput::default(), |_| {});
+        output.textures_delta.clear();
+        ctx
+    }
+
+    /// A result column of `kind`, its type named `type_name`.
+    fn meta(type_name: &str, kind: ValueKind) -> tabletist_db::ColumnMeta {
+        tabletist_db::ColumnMeta {
+            name: "column".into(),
+            type_name: type_name.into(),
+            kind,
+        }
+    }
+
+    #[test]
+    fn a_cell_says_what_a_value_with_nothing_to_show_is() {
+        // Without the system's fonts the marks are the plain ones.
+        let ctx = context();
+        let cell = |value: &Value, column: &tabletist_db::ColumnMeta, look: &Look| {
+            let cell = plain_cell(&ctx, value, column, look, false);
+            (cell.text.into_owned(), cell.style)
+        };
+        let text = |text: &str| Value::Text(text.into());
+        let words = meta("text", ValueKind::Text);
+        for look in Look::ALL {
+            let marks = grid::marks(&ctx, &look);
+            let (space, line) = (marks.space, marks.line);
+            assert_eq!(cell(&text(""), &words, &look), ("''".into(), Style::Quiet));
+            assert_eq!(
+                cell(&text("  "), &words, &look),
+                (format!("{space}{space}"), Style::Quiet)
+            );
+            // Text with a line break stays text, the break in sight.
+            assert_eq!(
+                cell(&text("Night Train\nPart One"), &words, &look),
+                (format!("Night Train{line}Part One"), Style::Plain)
+            );
+            assert_eq!(
+                cell(&text("   Leading spaces kept"), &words, &look),
+                ("   Leading spaces kept".into(), Style::Plain)
+            );
+        }
+    }
+
+    #[test]
+    fn an_array_and_a_binary_value_are_told_by_their_column() {
+        let ctx = context();
+        let cell = |value: &Value, column: &tabletist_db::ColumnMeta, look: &Look| {
+            let cell = plain_cell(&ctx, value, column, look, false);
+            (cell.text.into_owned(), cell.style)
+        };
+        let text = |text: &str| Value::Text(text.into());
+        let (mac, terminal) = (Look::macos(), Look::omarchy());
+        let languages = meta("_text", ValueKind::Other);
+        for look in [mac, terminal] {
+            assert_eq!(
+                cell(&text("{en,fr}"), &languages, &look),
+                ("{en,fr}".into(), Style::Array)
+            );
+            assert_eq!(
+                cell(&text("{}"), &languages, &look),
+                ("{}".into(), Style::Quiet)
+            );
+            // What is no array in an array's column is the text it is.
+            assert_eq!(
+                cell(&text("[0:1]={a,b}"), &languages, &look),
+                ("[0:1]={a,b}".into(), Style::Plain)
+            );
+        }
+        // The same text in a column of another type is only text.
+        assert_eq!(
+            cell(&text("{en,fr}"), &meta("int4range", ValueKind::Other), &mac),
+            ("{en,fr}".into(), Style::Plain)
+        );
+        let cover = Value::Bytes(vec![0; 49_358].into());
+        let bytea = meta("bytea", ValueKind::Binary);
+        assert_eq!(
+            cell(&cover, &bytea, &mac),
+            ("bytea · 48.2 KB".into(), Style::Chip)
+        );
+        assert_eq!(
+            cell(&cover, &bytea, &terminal),
+            ("bytea 48.2K".into(), Style::Quiet)
+        );
+    }
+
     #[test]
     fn a_plain_cell_reads_as_its_value_does_in_any_grid() {
-        let ctx = egui::Context::default();
+        let ctx = context();
         let mac = Look::macos();
         let cell = |value: &Value, kind, look: &Look, full| {
-            let cell = plain_cell(&ctx, value, kind, look, full);
+            let cell = plain_cell(&ctx, value, &meta("", kind), look, full);
             (cell.text.into_owned(), cell.null, cell.style)
         };
         assert_eq!(
@@ -1511,12 +1636,12 @@ mod tests {
     #[test]
     fn a_tag_comes_after_null_and_a_colour_and_before_the_rest() {
         use crate::ui::value_tags::Tags;
-        let ctx = egui::Context::default();
+        let ctx = context();
         let look = Look::macos();
         let allowed = ["#fff".to_owned(), "cover".to_owned(), "{}".to_owned()];
         let tags = Tags::Values(&allowed);
         let style = |value: &Value, kind, tags: &Tags<'_>| {
-            let cell = cell(&ctx, value, kind, tags, &look, false);
+            let cell = cell(&ctx, value, &meta("", kind), tags, &look, false);
             (cell.null, cell.style)
         };
         let text = |text: &str| Value::Text(text.into());
````

- [ ] **Step 2: Run them and see them fail**

Run: `~/.cargo/bin/cargo test --locked --lib`

Expected, before the code below is there:

````text
Does not compile:
error[E0425]: cannot find function `marks` in module `grid`
error[E0308]: mismatched types
error[E0599]: no variant, associated function, or constant named `Quiet` found for enum `ui::grid::Style` in the current scope
error[E0599]: no variant, associated function, or constant named `Array` found for enum `ui::grid::Style` in the current scope
error[E0599]: no variant, associated function, or constant named `Chip` found for enum `ui::grid::Style` in the current scope
````

- [ ] **Step 3: Write the code**

````diff
diff --git a/src/ui/data_view.rs b/src/ui/data_view.rs
--- a/src/ui/data_view.rs
+++ b/src/ui/data_view.rs
@@ -932,7 +932,7 @@ pub fn type_line(
     let base = match (kind, type_name) {
         (ValueKind::Temporal, "timestamp") => "timestamp · no tz".to_owned(),
         (ValueKind::Temporal, "timestamptz") => "timestamp · tz".to_owned(),
-        _ => type_name.to_owned(),
+        _ => format::type_label(type_name, kind).into_owned(),
     };
     let line = match (look.terminal, key, target) {
         (true, true, _) => "pk".to_owned(),
@@ -1006,7 +1006,7 @@ pub fn show(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId, object_tab: TabId)
                 cell(
                     &ctx,
                     &page.rows[row][col],
-                    page.columns[col].kind,
+                    &page.columns[col],
                     &tags[col],
                     &look,
                     full_precision,
@@ -1199,7 +1199,7 @@ fn empty_rows(
 pub fn cell<'a>(
     ctx: &egui::Context,
     value: &'a tabletist_db::Value,
-    kind: ValueKind,
+    column: &tabletist_db::ColumnMeta,
     tags: &crate::ui::value_tags::Tags<'_>,
     look: &Look,
     full_precision: bool,
@@ -1213,19 +1213,26 @@ pub fn cell<'a>(
             style,
         };
     }
-    plain_cell(ctx, value, kind, look, full_precision)
+    plain_cell(ctx, value, column, look, full_precision)
 }
 
-/// A cell's text and style when it is no tag: NULL, a colour's swatch, a
-/// document at a glance, a timestamp to the second unless `full_precision`,
-/// and anything else as it reads.
+/// A cell's text and style when it is no tag: NULL, a binary value's type
+/// and size, a colour's swatch, a document at a glance, what stands for
+/// text with nothing to see, an array's elements, a timestamp to the second
+/// unless `full_precision`, and anything else as it reads.
 pub fn plain_cell<'a>(
     ctx: &egui::Context,
     value: &'a tabletist_db::Value,
-    kind: ValueKind,
+    column: &tabletist_db::ColumnMeta,
     look: &Look,
     full_precision: bool,
 ) -> Cell<'a> {
+    let kind = column.kind;
+    let styled = |text: std::borrow::Cow<'a, str>, style| Cell {
+        text,
+        null: false,
+        style,
+    };
     if value.is_null() {
         return Cell {
             text: "NULL".into(),
@@ -1233,12 +1240,19 @@ pub fn plain_cell<'a>(
             style: Style::Plain,
         };
     }
-    if let Some(color) = format::color(value) {
-        return Cell {
-            text: format::cell_text(value),
-            null: false,
-            style: Style::Color(color),
+    if let tabletist_db::Value::Bytes(bytes) = value {
+        // Its type and size, never its bytes: a chip, or the terminal's
+        // muted words.
+        let label = format::binary_label(&column.type_name, bytes.len(), look.terminal);
+        let style = if look.terminal {
+            Style::Quiet
+        } else {
+            Style::Chip
         };
+        return styled(label.into(), style);
+    }
+    if let Some(color) = format::color(value) {
+        return styled(format::cell_text(value), Style::Color(color));
     }
     if let Some(doc) =
         crate::ui::json_view::document(ctx, kind, value, crate::ui::json_view::CELL_MAX)
@@ -1249,13 +1263,31 @@ pub fn plain_cell<'a>(
         } else {
             strings.into_iter().next().unwrap_or_default()
         };
-        return Cell {
-            text: format::one_line(&shown).into_owned().into(),
-            null: false,
-            style: Style::Json(count),
-        };
+        return styled(
+            format::one_line(&shown).into_owned().into(),
+            Style::Json(count),
+        );
     }
-    let text = format::cell_text(value);
+    let text = match value {
+        tabletist_db::Value::Text(text) => {
+            let marks = grid::marks(ctx, look);
+            // An empty string and a blank one say what they are.
+            if let Some(blank) = format::blank_text(text, marks) {
+                return styled(blank.into(), Style::Quiet);
+            }
+            let line = format::cell_line(text, marks);
+            if format::is_array(&column.type_name, kind) {
+                if line == "{}" {
+                    return styled(line, Style::Quiet);
+                }
+                if format::array_items(&line).is_some() {
+                    return styled(line, Style::Array);
+                }
+            }
+            line
+        }
+        other => format::cell_text(other),
+    };
     let text = if kind == ValueKind::Temporal && !full_precision {
         match format::to_the_second(&text) {
             std::borrow::Cow::Borrowed(_) => text,
@@ -1264,11 +1296,7 @@ pub fn plain_cell<'a>(
     } else {
         text
     };
-    Cell {
-        text,
-        null: false,
-        style: Style::Plain,
-    }
+    styled(text, Style::Plain)
 }
 
 /// The error a view shows in place of what `fetch` holds: see
diff --git a/src/ui/grid.rs b/src/ui/grid.rs
--- a/src/ui/grid.rs
+++ b/src/ui/grid.rs
@@ -12,7 +12,7 @@ use tabletist_db::SortDir;
 use crate::model::CellPos;
 use crate::theme::{DataFont, Icon, Look, Palette};
 use crate::typography::{Text, TextRole};
-use crate::ui::format::display_safe;
+use crate::ui::format::{Marks, array_items, display_safe};
 use crate::ui::widgets::virtual_rows;
 
 /// The header's height, per look.
@@ -68,6 +68,16 @@ pub enum Style {
     Json(usize),
     /// A colour (`#3a7bd5`): a swatch of it, then the text.
     Color(egui::Color32),
+    /// Text that stands for what the cell holds rather than being it (`''`
+    /// for an empty string, a mark for each space of a blank one, `{}` for
+    /// an empty array): faint.
+    Quiet,
+    /// What is known of a value the cell does not show (a binary value's
+    /// type and size): one outlined chip.
+    Chip,
+    /// A PostgreSQL array, its text as the database writes it: a chip for
+    /// each element that fits, then `+n` for the rest.
+    Array,
 }
 
 pub struct Cell<'a> {
@@ -96,6 +106,20 @@ pub fn data_role(look: &Look) -> TextRole {
     }
 }
 
+/// The marks `look`'s data face writes where text would show nothing: the
+/// design's when the fonts at hand have them (see [`Marks::pick`]). Found
+/// once for each set of faces, not for every cell.
+pub fn marks(ctx: &egui::Context, look: &Look) -> Marks {
+    let id = Id::new(("cell-marks", look.faces));
+    if let Some(marks) = ctx.data(|data| data.get_temp::<Marks>(id)) {
+        return marks;
+    }
+    let font = data_role(look).font_id(look.faces);
+    let marks = ctx.fonts_mut(|fonts| Marks::pick(|character| fonts.has_glyph(&font, character)));
+    ctx.data_mut(|data| data.insert_temp(id, marks));
+    marks
+}
+
 /// Paints `text` in `role` at `x` (its left edge, or its right with
 /// `right`), centred on `y`.
 #[allow(clippy::too_many_arguments)] // the text, its place, and its look
@@ -269,10 +293,18 @@ pub fn initial_widths<'a>(
                 .map(|row| {
                     let cell = cell(row, col);
                     let chip = match cell.style {
-                        Style::Plain => 0.0,
+                        Style::Plain | Style::Quiet => 0.0,
                         Style::Tag(_) => 16.0,
+                        Style::Chip => 2.0 * CHIP_PAD + 2.0,
                         Style::Json(_) => 44.0,
                         Style::Color(_) => SWATCH + SWATCH_GAP,
+                        // Each element's chip adds its sides and the gap
+                        // to the next.
+                        Style::Array => {
+                            let elements =
+                                array_items(&cell.text).map_or(0, |array| array.items.len());
+                            (2.0 * CHIP_PAD + CHIP_GAP) * elements as f32
+                        }
                     };
                     width(&cell.text) + chip
                 })
@@ -752,7 +784,84 @@ fn draw_header(
     );
 }
 
-/// One cell: its text, tag, JSON chip or colour swatch, cut to fit.
+/// Space inside a chip, at each side of its text.
+const CHIP_PAD: f32 = 5.0;
+/// Space between two chips.
+const CHIP_GAP: f32 = 4.0;
+
+/// How a chip draws: NULL is filled, an array's element and a binary
+/// value's size are outlined.
+#[derive(Clone, Copy)]
+struct ChipSkin {
+    role: TextRole,
+    text: egui::Color32,
+    fill: Option<egui::Color32>,
+    border: Option<egui::Color32>,
+}
+
+impl ChipSkin {
+    /// NULL: quieter than any value.
+    fn null(palette: &Palette) -> Self {
+        Self {
+            role: TextRole::JsonChip,
+            text: palette.faint,
+            fill: Some(palette.surface),
+            border: None,
+        }
+    }
+
+    /// A piece of a value, or a note about one, in `role`.
+    fn outlined(role: TextRole, palette: &Palette) -> Self {
+        Self {
+            role,
+            text: palette.secondary,
+            fill: None,
+            border: Some(palette.outline),
+        }
+    }
+}
+
+/// The width of a chip holding `text`.
+fn chip_width(ui: &Ui, text: &str, role: TextRole, look: &Look) -> f32 {
+    text_width(ui, text, role, look) + 2.0 * CHIP_PAD
+}
+
+/// Paints a chip holding `text`, its left edge at `left`, centred on
+/// `center`.
+fn paint_chip(
+    painter: &egui::Painter,
+    ui: &Ui,
+    text: &str,
+    left: f32,
+    center: f32,
+    skin: ChipSkin,
+    look: &Look,
+) {
+    // The text's line, and room for an outline round it.
+    let line = skin.role.row_height(ui.ctx(), look.faces);
+    let height = line + if skin.border.is_some() { 4.0 } else { 2.0 };
+    let rect = Rect::from_min_size(
+        pos2(left, center - height / 2.0),
+        vec2(chip_width(ui, text, skin.role, look), height),
+    );
+    let corner = CornerRadius::same(4);
+    if let Some(fill) = skin.fill {
+        painter.rect_filled(rect, corner, fill);
+    }
+    if let Some(border) = skin.border {
+        painter.rect_stroke(
+            rect,
+            corner,
+            Stroke::new(crate::ui::widgets::hairline(ui), border),
+            StrokeKind::Inside,
+        );
+    }
+    Text::one(look, skin.role, text, skin.text)
+        .layout(ui.ctx())
+        .paint_center(painter, rect.center());
+}
+
+/// One cell: its text, tag, chips or colour swatch, cut to fit.
 fn draw_cell(
     ui: &Ui,
     painter: &egui::Painter,
@@ -769,20 +878,120 @@ fn draw_cell(
     let center = rect.center().y;
     let room = rect.width() - 2.0 * pad;
     if content.null {
-        paint(
-            &clip,
-            ui,
-            role,
-            "NULL",
-            palette.faint,
-            rect.left() + pad,
-            center,
-            false,
-            look,
-        );
+        // With the numbers in a numeric column, as a value would be.
+        let numeric = column.numeric;
+        if look.terminal {
+            let x = if numeric {
+                rect.right() - pad
+            } else {
+                rect.left() + pad
+            };
+            paint(
+                &clip,
+                ui,
+                role,
+                "NULL",
+                palette.faint,
+                x,
+                center,
+                numeric,
+                look,
+            );
+        } else {
+            let skin = ChipSkin::null(palette);
+            let left = if numeric {
+                rect.right() - pad - chip_width(ui, "NULL", skin.role, look)
+            } else {
+                rect.left() + pad
+            };
+            paint_chip(&clip, ui, "NULL", left, center, skin, look);
+        }
         return;
     }
     match content.style {
+        Style::Quiet => {
+            let shown = ellipsize(&content.text, room, false, |text| width(text, role));
+            paint(
+                &clip,
+                ui,
+                role,
+                &shown,
+                palette.faint,
+                rect.left() + pad,
+                center,
+                false,
+                look,
+            );
+        }
+        Style::Chip => {
+            let skin = ChipSkin::outlined(TextRole::FieldLabel, palette);
+            let shown = ellipsize(&content.text, room - 2.0 * CHIP_PAD, false, |text| {
+                width(text, skin.role)
+            });
+            paint_chip(&clip, ui, &shown, rect.left() + pad, center, skin, look);
+        }
+        Style::Array => {
+            // The terminal writes an array as the database does.
+            let Some(array) = array_items(&content.text).filter(|_| !look.terminal) else {
+                let shown = ellipsize(&content.text, room, false, |text| width(text, role));
+                paint(
+                    &clip,
+                    ui,
+                    role,
+                    &shown,
+                    palette.text,
+                    rect.left() + pad,
+                    center,
+                    false,
+                    look,
+                );
+                return;
+            };
+            let skin = ChipSkin::outlined(TextRole::ValueTag, palette);
+            // What stands for the elements that do not fit: how many, or
+            // "…" when the text was cut and nobody counted.
+            let more = |rest: usize| {
+                if array.cut {
+                    "…".to_owned()
+                } else {
+                    format!("+{rest}")
+                }
+            };
+            let right = rect.right() - pad;
+            let mut left = rect.left() + pad;
+            let mut shown = 0;
+            for (index, item) in array.items.iter().enumerate() {
+                let after = array.items.len() - index - 1;
+                let reserve = if after > 0 || array.cut {
+                    CHIP_GAP + chip_width(ui, &more(after), skin.role, look)
+                } else {
+                    0.0
+                };
+                let room = right - left - reserve;
+                let wide = chip_width(ui, item, skin.role, look);
+                if wide <= room {
+                    paint_chip(&clip, ui, item, left, center, skin, look);
+                    left += wide + CHIP_GAP;
+                } else if index == 0 {
+                    // The first element always shows, cut to its room.
+                    let cut = ellipsize(item, (room - 2.0 * CHIP_PAD).max(0.0), false, |text| {
+                        width(text, skin.role)
+                    });
+                    paint_chip(&clip, ui, &cut, left, center, skin, look);
+                    left += chip_width(ui, &cut, skin.role, look) + CHIP_GAP;
+                } else {
+                    break;
+                }
+                shown += 1;
+                if wide > room {
+                    break;
+                }
+            }
+            let rest = array.items.len() - shown;
+            if rest > 0 || array.cut {
+                paint_chip(&clip, ui, &more(rest), left, center, skin, look);
+            }
+        }
         Style::Tag(_) => {
             let (color, fill) = crate::ui::value_tags::style_colors(content.style, look, palette);
             // macOS and Windows: a chip in the value-tag face, 2 above and
diff --git a/src/ui/row_panel.rs b/src/ui/row_panel.rs
--- a/src/ui/row_panel.rs
+++ b/src/ui/row_panel.rs
@@ -88,7 +88,7 @@ fn label(name: &str, type_name: &str, kind: ValueKind, info: &FieldInfo, look: &
     let type_name = match (kind, type_name) {
         (ValueKind::Temporal, "timestamp") => "timestamp · no tz".to_owned(),
         (ValueKind::Temporal, "timestamptz") => "timestamp · tz".to_owned(),
-        _ => format::display_safe(type_name).into_owned(),
+        _ => format::display_safe(&format::type_label(type_name, kind)).into_owned(),
     };
     parts.push(type_name);
     if info.key {
diff --git a/src/ui/sql_results.rs b/src/ui/sql_results.rs
--- a/src/ui/sql_results.rs
+++ b/src/ui/sql_results.rs
@@ -1156,7 +1156,7 @@ fn results(ui: &mut Ui, run: &SqlRun, place: &Place<'_>, env: &Env<'_>, actions:
             data_view::cell(
                 &ctx,
                 &rows[row][col],
-                columns[col].kind,
+                &columns[col],
                 &tags[col],
                 look,
                 full_precision,
diff --git a/src/ui/value_tags.rs b/src/ui/value_tags.rs
--- a/src/ui/value_tags.rs
+++ b/src/ui/value_tags.rs
@@ -113,7 +113,9 @@ pub fn slot_colors(slot: usize, look: &Look, palette: &Palette) -> (Color32, Opt
 pub fn style_colors(style: Style, look: &Look, palette: &Palette) -> (Color32, Option<Color32>) {
     match style {
         Style::Tag(slot) => slot_colors(slot, look, palette),
-        Style::Plain | Style::Json(_) | Style::Color(_) => (palette.text, None),
+        Style::Quiet => (palette.faint, None),
+        Style::Chip => (palette.secondary, None),
+        Style::Plain | Style::Json(_) | Style::Color(_) | Style::Array => (palette.text, None),
     }
 }
 
````

- [ ] **Step 4: Run the tests**

Run: `~/.cargo/bin/cargo test --locked --lib ui::data_view`
Expected: PASS.

Then all of the library's: `~/.cargo/bin/cargo test --locked --lib`
Expected: `test result: ok. 865 passed; 0 failed; 1 ignored`.

- [ ] **Step 5: Add the scenes**

````diff
diff --git a/src/shots.rs b/src/shots.rs
--- a/src/shots.rs
+++ b/src/shots.rs
@@ -725,6 +725,9 @@ mod mock {
         MacWorkspace,
         /// The same workspace in the dark palette.
         MacWorkspaceDark,
+        /// `book_editions`: values that are awkward to show.
+        MacValues,
+        OmarchyValues,
         OmarchyWorkspace,
         MacPicker,
         OmarchyPicker,
@@ -733,9 +736,11 @@ mod mock {
     }
 
     impl Screen {
-        pub const ALL: [Screen; 7] = [
+        pub const ALL: [Screen; 9] = [
             Self::MacWorkspace,
             Self::MacWorkspaceDark,
+            Self::MacValues,
+            Self::OmarchyValues,
             Self::OmarchyWorkspace,
             Self::MacPicker,
             Self::OmarchyPicker,
@@ -748,6 +753,8 @@ mod mock {
             match self {
                 Self::MacWorkspace => "macos-workspace",
                 Self::MacWorkspaceDark => "macos-workspace-dark",
+                Self::MacValues => "macos-values",
+                Self::OmarchyValues => "omarchy-values",
                 Self::OmarchyWorkspace => "omarchy-workspace",
                 Self::MacPicker => "macos-connections",
                 Self::OmarchyPicker => "omarchy-connections",
@@ -758,12 +765,15 @@ mod mock {
 
         pub fn look(self) -> Look {
             match self {
-                Self::MacWorkspace | Self::MacWorkspaceDark | Self::MacPicker | Self::MacDialog => {
-                    Look::macos()
-                }
-                Self::OmarchyWorkspace | Self::OmarchyPicker | Self::OmarchyDialog => {
-                    Look::omarchy()
-                }
+                Self::MacWorkspace
+                | Self::MacWorkspaceDark
+                | Self::MacValues
+                | Self::MacPicker
+                | Self::MacDialog => Look::macos(),
+                Self::OmarchyWorkspace
+                | Self::OmarchyValues
+                | Self::OmarchyPicker
+                | Self::OmarchyDialog => Look::omarchy(),
             }
         }
 
@@ -771,10 +781,12 @@ mod mock {
         /// a 10 pt wallpaper margin and Hyprland's 2 pt border.
         pub fn size(self) -> egui::Vec2 {
             match self {
-                Self::MacWorkspace | Self::MacWorkspaceDark | Self::MacPicker | Self::MacDialog => {
-                    egui::vec2(1440.0, 900.0)
-                }
-                Self::OmarchyWorkspace => egui::vec2(1896.0, 1056.0),
+                Self::MacWorkspace
+                | Self::MacWorkspaceDark
+                | Self::MacValues
+                | Self::MacPicker
+                | Self::MacDialog => egui::vec2(1440.0, 900.0),
+                Self::OmarchyWorkspace | Self::OmarchyValues => egui::vec2(1896.0, 1056.0),
                 Self::OmarchyPicker | Self::OmarchyDialog => egui::vec2(936.0, 1016.0),
             }
         }
@@ -783,10 +795,12 @@ mod mock {
         /// line up with them.
         pub fn design_scale(self) -> f32 {
             match self {
-                Self::MacWorkspace | Self::MacWorkspaceDark | Self::MacPicker | Self::MacDialog => {
-                    2000.0 / 1440.0
-                }
-                Self::OmarchyWorkspace => 2000.0 / 1920.0,
+                Self::MacWorkspace
+                | Self::MacWorkspaceDark
+                | Self::MacValues
+                | Self::MacPicker
+                | Self::MacDialog => 2000.0 / 1440.0,
+                Self::OmarchyWorkspace | Self::OmarchyValues => 2000.0 / 1920.0,
                 Self::OmarchyPicker | Self::OmarchyDialog => 1846.0 / 960.0,
             }
         }
@@ -795,9 +809,14 @@ mod mock {
         /// Night, as the mockups.
         pub fn palette(self) -> Palette {
             match self {
-                Self::MacWorkspace | Self::MacPicker | Self::MacDialog => Palette::light(),
+                Self::MacWorkspace | Self::MacValues | Self::MacPicker | Self::MacDialog => {
+                    Palette::light()
+                }
                 Self::MacWorkspaceDark => Palette::dark(),
-                Self::OmarchyWorkspace | Self::OmarchyPicker | Self::OmarchyDialog => tokyo_night(),
+                Self::OmarchyWorkspace
+                | Self::OmarchyValues
+                | Self::OmarchyPicker
+                | Self::OmarchyDialog => tokyo_night(),
             }
         }
 
@@ -829,6 +848,23 @@ mod mock {
                         cell: CellPos { row: 0, col: 0 },
                     });
                 }
+                Self::MacValues | Self::OmarchyValues => {
+                    let tab = workspace(harness);
+                    harness.app.apply(Action::OpenObject {
+                        tab,
+                        object: ObjectRef::new("public", "book_editions"),
+                        kind: ObjectKind::Table,
+                        pin: true,
+                    });
+                    harness.answer_structure(editions_structure());
+                    harness.answer_rows(editions());
+                    let object_tab = harness.app.workspace(tab).unwrap().active_tab.unwrap();
+                    harness.app.apply(Action::SelectCell {
+                        tab,
+                        id: object_tab,
+                        cell: CellPos { row: 0, col: 0 },
+                    });
+                }
                 Self::MacPicker | Self::OmarchyPicker => pickers(harness),
                 Self::MacDialog | Self::OmarchyDialog => {
                     pickers(harness);
@@ -1043,6 +1079,188 @@ mod mock {
         }
     }
 
+    /// A document of `keys` keys: an ISBN, a page count, a list and an
+    /// object, then plain numbered ones.
+    fn metadata(keys: usize) -> Value {
+        let mut parts = vec![
+            r#""isbn": "978-1-4028-9462-6""#.to_owned(),
+            r#""pages": 612"#.to_owned(),
+            r#""awards": ["a", "b", "c", "d", "e", "f", "g"]"#.to_owned(),
+            r#""print_runs": {"first": 1200, "second": 800}"#.to_owned(),
+        ];
+        parts.truncate(keys);
+        for index in parts.len()..keys {
+            parts.push(format!(r#""note_{index}": "{index}""#));
+        }
+        Value::Text(format!("{{{}}}", parts.join(", ")).into())
+    }
+
+    /// `bytes` bytes that start as a JPEG does.
+    fn cover(bytes: usize) -> Value {
+        let head = [
+            0xff, 0xd8, 0xff, 0xe0, 0x00, 0x10, 0x4a, 0x46, 0x49, 0x46, 0x00, 0x01, 0x01, 0x00,
+            0x00, 0x48, 0x00, 0x48, 0x00, 0x00, 0xff, 0xdb, 0x00, 0x43,
+        ];
+        let data: Vec<u8> = head
+            .into_iter()
+            .chain(std::iter::repeat(0))
+            .take(bytes)
+            .collect();
+        Value::Bytes(data.into())
+    }
+
+    /// `book_editions`: the values the mockups call awkward.
+    fn editions() -> RowPage {
+        let column = |name: &str, type_name: &str, kind| ColumnMeta {
+            name: name.into(),
+            type_name: type_name.into(),
+            kind,
+        };
+        let text = |value: &str| Value::Text(value.into());
+        let long = "The Lighthouse Keeper's Daughter: A Novel in Three Tides, with an Afterword \
+                    by the Translator. First published in a small run by Harbor Press, later \
+                    reissued with the restored third part and the letters. "
+            .repeat(10);
+        let rows = vec![
+            vec![
+                Value::Int(101),
+                text(&long),
+                text("A novel"),
+                text("hardcover"),
+                Value::Bool(true),
+                text("24.00"),
+                text("{en,fr}"),
+                cover(49_358),
+                metadata(46),
+                text("2025-11-04"),
+            ],
+            vec![
+                Value::Int(102),
+                text("Small Rooms"),
+                text(""),
+                text("paperback"),
+                Value::Bool(true),
+                text("12.50"),
+                text("{en}"),
+                cover(31_744),
+                metadata(12),
+                text("2024-03-19"),
+            ],
+            vec![
+                Value::Int(103),
+                text("Notes from the Night Train\nPart One"),
+                Value::Null,
+                text("ebook"),
+                Value::Bool(false),
+                text("0.00"),
+                text("{}"),
+                Value::Null,
+                Value::Null,
+                text("2023-09-01"),
+            ],
+            vec![
+                Value::Int(104),
+                text("A Field Guide to Paper"),
+                Value::Null,
+                Value::Null,
+                Value::Null,
+                Value::Null,
+                Value::Null,
+                Value::Null,
+                Value::Null,
+                Value::Null,
+            ],
+            vec![
+                Value::Int(105),
+                text("Maps of Imaginary Coasts"),
+                text("Atlas edition"),
+                text("hardcover"),
+                Value::Bool(true),
+                text("1240.00"),
+                text("{en,de,it,fr,es,pt,nl}"),
+                cover(2_202_010),
+                metadata(103),
+                text("2026-01-15"),
+            ],
+            vec![
+                Value::Int(106),
+                text("   Leading spaces kept"),
+                text("  "),
+                text("paperback"),
+                Value::Bool(false),
+                text("9.99"),
+                text("{es}"),
+                cover(0),
+                metadata(0),
+                text("2022-06-30"),
+            ],
+            vec![
+                Value::Int(107),
+                text("Unicode: Ærøskøbing"),
+                text("Translated"),
+                text("ebook"),
+                Value::Bool(true),
+                text("7.00"),
+                text("{da,ja,ar}"),
+                cover(13_005),
+                metadata(8),
+                text("2025-02-14"),
+            ],
+        ];
+        RowPage {
+            columns: vec![
+                column("id", "int8", ValueKind::Numeric),
+                column("title", "text", ValueKind::Text),
+                column("subtitle", "text", ValueKind::Text),
+                column("format", "edition_format", ValueKind::Other),
+                column("in_print", "bool", ValueKind::Bool),
+                column("price", "numeric", ValueKind::Numeric),
+                column("languages", "_text", ValueKind::Other),
+                column("cover", "bytea", ValueKind::Binary),
+                column("metadata", "jsonb", ValueKind::Json),
+                column("published_on", "date", ValueKind::Temporal),
+            ],
+            rows,
+            has_more: false,
+            ordered_by_key: true,
+            elapsed: Duration::from_millis(2),
+        }
+    }
+
+    fn editions_structure() -> Structure {
+        let column = |name: &str, type_name: &str, nullable| ColumnInfo {
+            name: name.into(),
+            type_name: type_name.into(),
+            nullable,
+            default: None,
+            comment: None,
+            allowed_values: None,
+        };
+        let mut format = column("format", "edition_format", true);
+        format.allowed_values = Some(
+            ["hardcover", "paperback", "audiobook", "ebook"]
+                .map(str::to_owned)
+                .to_vec(),
+        );
+        Structure {
+            columns: vec![
+                column("id", "bigint", false),
+                column("title", "text", false),
+                column("subtitle", "text", true),
+                format,
+                column("in_print", "boolean", true),
+                column("price", "numeric(10,2)", true),
+                column("languages", "text[]", true),
+                column("cover", "bytea", true),
+                column("metadata", "jsonb", true),
+                column("published_on", "date", true),
+            ],
+            primary_key: vec!["id".into()],
+            indexes: Vec::new(),
+            foreign_keys: Vec::new(),
+        }
+    }
+
     fn covers_structure() -> Structure {
         let column = |name: &str, type_name: &str, nullable| ColumnInfo {
             name: name.into(),
````

- [ ] **Step 6: Render, and look**

Run: `~/.cargo/bin/cargo test --locked --features shots --lib shots -- --ignored`

`target/shots/mock-macos-values.png` and `mock-omarchy-values.png` beside the artboard "macOS – Awkward values" and pane 5 of the Omarchy one: NULL chips (at the right in `price`), `''`, the blank subtitle's marks, the line break's mark in "Night Train", the tags of `format` and `in_print`. The macOS window is too narrow for the array and binary columns: scroll is not possible in a picture, so render it wide once (in `Screen::size`, give `MacValues` `egui::vec2(2100.0, 600.0)`, look, and put it back) and check the element chips, `+n`, `{}` and `bytea · 48.2 KB`.

Put right what differs, in the code this task wrote; keep the tests.

- [ ] **Step 7: Format and lint**

```bash
~/.cargo/bin/cargo fmt --all --check
~/.cargo/bin/cargo clippy --locked --workspace --all-targets -- -D warnings
~/.cargo/bin/cargo clippy --locked --workspace --all-targets --features shots -- -D warnings
```

Expected: no output.

- [ ] **Step 8: Commit point**

Stop and report. If the user has asked for commits:

```bash
git add -A src AGENTS.md
git commit -S -m "Say what a cell with nothing to show holds" -m "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

### Task 8: The row panel's awkward values

**Files:**
- Modify: `src/ui/row_panel.rs`
- Modify: `src/ui/grid.rs` (`null_label`)

The row panel says of a value what the grid's cell could only hint at. A field's label gains facts (`languages · text[] · 2 elements`, `cover · bytea · 48.2 KB · looks like JPEG`, `title · text · 2,341 characters`); NULL is the grid's chip; an empty or blank string is its stand-in and the words for it; an array is a list under its positions (one of more than 64 KB is text: the panel reads the elements every frame it shows them); a binary value is its first bytes in a box; a value of more than three lines shows three, the last fading out, until "Show all".

- [ ] **Step 1: Write the tests**

New in `src/ui/row_panel.rs`, on a fixture `awkward(look)` (a table open on one row of such values): `a_fields_label_says_what_its_value_does_not_show`, `values_with_nothing_to_see_say_what_they_are`, `a_long_value_shows_its_start_until_asked_for_all`, each in every look, and `an_array_too_long_to_read_every_frame_is_text`.

````diff
diff --git a/src/ui/row_panel.rs b/src/ui/row_panel.rs
--- a/src/ui/row_panel.rs
+++ b/src/ui/row_panel.rs
@@ -1279,4 +1545,145 @@ mod tests {
         assert_eq!(range(400.0), (200.0, 200.0));
         assert_eq!(range(0.0), (0.0, 0.0));
     }
+
+    /// A table open on one row of awkward values, that row in the panel.
+    fn awkward(look: Look) -> crate::testing::Harness {
+        awkward_with(look, "{en,fr}")
+    }
+
+    /// [`awkward`], its array column holding `languages`.
+    fn awkward_with(look: Look, languages: &str) -> crate::testing::Harness {
+        use tabletist_db::{ColumnMeta, ObjectKind, ObjectRef, RowPage};
+        let mut harness = crate::testing::Harness::new();
+        harness.set_look(look);
+        let tab = harness.connect_fake();
+        harness.app.apply(Action::OpenObject {
+            tab,
+            object: ObjectRef::new("main", "editions"),
+            kind: ObjectKind::Table,
+            pin: true,
+        });
+        let column = |name: &str, type_name: &str, kind| ColumnMeta {
+            name: name.into(),
+            type_name: type_name.into(),
+            kind,
+        };
+        let text = |value: &str| Value::Text(value.into());
+        let jpeg: Vec<u8> = [0xff, 0xd8, 0xff, 0xe0]
+            .into_iter()
+            .chain(std::iter::repeat_n(0, 96))
+            .collect();
+        harness.answer_rows(RowPage {
+            columns: vec![
+                column("title", "text", ValueKind::Text),
+                column("subtitle", "text", ValueKind::Text),
+                column("languages", "_text", ValueKind::Other),
+                column("cover", "bytea", ValueKind::Binary),
+                column("notes", "text", ValueKind::Text),
+            ],
+            rows: vec![vec![
+                text(&"A long title. ".repeat(40)),
+                text(""),
+                text(languages),
+                Value::Bytes(jpeg.into()),
+                Value::Null,
+            ]],
+            has_more: false,
+            ordered_by_key: true,
+            elapsed: std::time::Duration::ZERO,
+        });
+        let id = harness.app.workspace(tab).unwrap().active_tab.unwrap();
+        harness.app.apply(Action::SelectCell {
+            tab,
+            id,
+            cell: CellPos { row: 0, col: 0 },
+        });
+        harness.settle();
+        harness
+    }
+
+    #[test]
+    fn a_fields_label_says_what_its_value_does_not_show() {
+        for look in Look::ALL {
+            let mut harness = awkward(look);
+            let labels = crate::testing::labels(&harness.settle());
+            let expected = if look.terminal {
+                [
+                    "title · text · 560 chars",
+                    "languages · text[] · 2 elements",
+                    "cover · bytea · 100B · jpeg",
+                ]
+            } else {
+                [
+                    "title · text · 560 characters",
+                    "languages · text[] · 2 elements",
+                    "cover · bytea · 100 B · looks like JPEG",
+                ]
+            };
+            for label in expected {
+                assert!(
+                    labels.iter().any(|found| found == label),
+                    "{label} in {}: {labels:?}",
+                    look.name
+                );
+            }
+            // A value that shows whole says nothing more.
+            assert!(labels.iter().any(|found| found == "subtitle · text"));
+        }
+    }
+
+    #[test]
+    fn values_with_nothing_to_see_say_what_they_are() {
+        for look in Look::ALL {
+            let mut harness = awkward(look);
+            let labels = crate::testing::labels(&harness.settle());
+            for text in ["''", "empty string", "NULL", "en", "fr"] {
+                assert!(
+                    labels.iter().any(|found| found == text),
+                    "{text} in {}: {labels:?}",
+                    look.name
+                );
+            }
+            // A binary value is its first bytes and a count of the rest.
+            let painted = |text: &str| {
+                harness
+                    .painted
+                    .iter()
+                    .any(|(piece, _)| piece.contains(text))
+            };
+            assert!(painted("ff d8 ff e0 00 00"), "{:?}", harness.painted);
+            assert!(painted("76"), "the bytes past the first 24");
+            // An array's elements stand after their positions.
+            assert!(painted("[1]") && painted("[2]"), "{}", look.name);
+        }
+    }
+
+    #[test]
+    fn an_array_too_long_to_read_every_frame_is_text() {
+        let long = format!("{{{}}}", vec!["element"; 20_000].join(","));
+        let mut harness = awkward_with(Look::macos(), &long);
+        let labels = crate::testing::labels(&harness.settle());
+        // No list and no count of elements: its length, as any long text.
+        assert!(
+            labels.iter().any(|label| {
+                label.starts_with("languages · text[] · ") && label.ends_with(" characters")
+            }),
+            "{labels:?}"
+        );
+        assert!(!labels.iter().any(|label| label.contains("elements")));
+        assert!(labels.iter().any(|label| label.starts_with("Show all")));
+    }
+
+    #[test]
+    fn a_long_value_shows_its_start_until_asked_for_all() {
+        for look in Look::ALL {
+            let mut harness = awkward(look);
+            assert!(harness.has("Show all (560 B)"), "{}", look.name);
+            assert!(!harness.has("Show less"));
+            harness.click("Show all (560 B)");
+            assert!(harness.has("Show less"), "{}", look.name);
+            harness.click("Show less");
+            assert!(harness.has("Show all (560 B)"), "{}", look.name);
+        }
+    }
 }
````

- [ ] **Step 2: Run them and see them fail**

Run: `~/.cargo/bin/cargo test --locked --lib`

Expected, before the code below is there:

````text
test ui::row_panel::tests::a_fields_label_says_what_its_value_does_not_show ... FAILED
test ui::row_panel::tests::a_long_value_shows_its_start_until_asked_for_all ... FAILED
test ui::row_panel::tests::an_array_too_long_to_read_every_frame_is_text ... FAILED
test ui::row_panel::tests::values_with_nothing_to_see_say_what_they_are ... FAILED
test result: FAILED. 865 passed; 4 failed; 1 ignored; 0 measured; 0 filtered out
````

- [ ] **Step 3: Write the code**

````diff
diff --git a/src/ui/grid.rs b/src/ui/grid.rs
--- a/src/ui/grid.rs
+++ b/src/ui/grid.rs
@@ -861,6 +861,36 @@ fn paint_chip(
         .paint_center(painter, rect.center());
 }
 
+/// NULL where a value would be, laid out in `ui`: the grid's chip, or the
+/// terminal's faint word. The row panel's fields use it.
+pub fn null_label(ui: &mut Ui, look: &Look, palette: &Palette) -> egui::Response {
+    let role = data_role(look);
+    if look.terminal {
+        return Text::one(look, role, "NULL", palette.faint)
+            .layout(ui.ctx())
+            .label(ui);
+    }
+    let skin = ChipSkin::null(palette);
+    let size = vec2(
+        chip_width(ui, "NULL", skin.role, look),
+        role.row_height(ui.ctx(), look.faces),
+    );
+    let (rect, response) = ui.allocate_exact_size(size, Sense::hover());
+    response.widget_info(|| WidgetInfo::labeled(WidgetType::Label, true, "NULL"));
+    if ui.is_rect_visible(rect) {
+        paint_chip(
+            ui.painter(),
+            ui,
+            "NULL",
+            rect.left(),
+            rect.center().y,
+            skin,
+            look,
+        );
+    }
+    response
+}
+
 /// One cell: its text, tag, chips or colour swatch, cut to fit.
 fn draw_cell(
     ui: &Ui,
diff --git a/src/ui/row_panel.rs b/src/ui/row_panel.rs
--- a/src/ui/row_panel.rs
+++ b/src/ui/row_panel.rs
@@ -76,10 +76,18 @@ struct FieldInfo {
     target_column: String,
 }
 
-/// A column's label: `id · int8 · primary key`. The terminal's half-width
-/// cells name a timestamp alone, as the design does.
+/// A column's label: `id · int8 · primary key`, then what [`facts`] says of
+/// its value (`cover · bytea · 48.2 KB · looks like JPEG`). The terminal's
+/// half-width cells name a timestamp alone, as the design does.
 /// Names and types come from the server, so nothing hidden in them shows.
-fn label(name: &str, type_name: &str, kind: ValueKind, info: &FieldInfo, look: &Look) -> String {
+fn label(
+    name: &str,
+    type_name: &str,
+    kind: ValueKind,
+    info: &FieldInfo,
+    facts: &[String],
+    look: &Look,
+) -> String {
     let name = format::display_safe(name).into_owned();
     if look.terminal && kind == ValueKind::Temporal {
         return name;
@@ -98,6 +106,7 @@ fn label(name: &str, type_name: &str, kind: ValueKind, info: &FieldInfo, look: &
             "primary key".into()
         });
     }
+    parts.extend(facts.iter().cloned());
     let mut text = parts.join(" · ");
     if let (Some(target), true) = (&info.target, look.terminal) {
         text.push_str(&format!(" → {}", format::display_safe(&target.name)));
@@ -105,6 +114,57 @@ fn label(name: &str, type_name: &str, kind: ValueKind, info: &FieldInfo, look: &
     text
 }
 
+/// A text value longer than this says how long it is in its label.
+const COUNTED_CHARS: usize = 120;
+
+/// The longest array the panel lists: its text is read into elements every
+/// frame it shows. A longer one is text, cut and unfolded as any long text.
+const ARRAY_MAX: usize = 64 * 1024;
+
+/// Whether `text` is an array the panel lists element by element.
+fn listed(text: &str, column: &tabletist_db::ColumnMeta) -> bool {
+    text.len() <= ARRAY_MAX && format::is_array(&column.type_name, column.kind)
+}
+
+/// What a field's label says of its value, after its column's name and
+/// type: how many elements an array has, how big a binary value is and what
+/// it starts as, how long a long text is.
+fn facts(
+    value: &Value,
+    column: &tabletist_db::ColumnMeta,
+    formatted: Option<&format::FieldText>,
+    look: &Look,
+    locale: crate::i18n::Locale,
+) -> Vec<String> {
+    let say = |text: &'static str| look.label(&gettext(locale, text));
+    match value {
+        Value::Bytes(bytes) if look.terminal => std::iter::once(format::terse_size(bytes.len()))
+            .chain(format::sniff(bytes).map(str::to_lowercase))
+            .collect(),
+        Value::Bytes(bytes) => std::iter::once(format::human_size(bytes.len()))
+            .chain(format::sniff(bytes).map(|kind| format!("{} {kind}", say("looks like"))))
+            .collect(),
+        Value::Text(text) if listed(text, column) => format::array_items(text)
+            .map(|array| {
+                let count = array.items.len();
+                let noun = if count == 1 { "element" } else { "elements" };
+                format!("{count} {}", say(noun))
+            })
+            .into_iter()
+            .collect(),
+        Value::Text(_) => formatted
+            .and_then(|field| field.characters)
+            .filter(|count| *count > COUNTED_CHARS)
+            .map(|count| {
+                let noun = if look.terminal { "chars" } else { "characters" };
+                format!("{} {}", format::group_digits(count as u64), say(noun))
+            })
+            .into_iter()
+            .collect(),
+        Value::Null | Value::Bool(_) | Value::Int(_) | Value::Float(_) => Vec::new(),
+    }
+}
+
 /// What the panel shows a row of: a table's page, or a SQL editor's result.
 struct Source<'a> {
     /// The table's name, or "Query 3": under the title in the macOS header.
@@ -758,8 +818,22 @@ fn field(
     // page or another result keeps no folds and nothing expanded.
     let request = texts.and_then(|texts| texts.request);
     let label_role = caption(look);
-    let text = label(&column.name, &column.type_name, column.kind, info, look);
+    let formatted = texts.and_then(|texts| texts.fields.get(col));
     let doc = json_doc(ui.ctx(), value, column.kind);
+    // A document's label shares its line with the document's controls.
+    let facts = if doc.is_some() {
+        Vec::new()
+    } else {
+        facts(value, column, formatted, look, locale)
+    };
+    let text = label(
+        &column.name,
+        &column.type_name,
+        column.kind,
+        info,
+        &facts,
+        look,
+    );
     let width = ui.available_width();
     // A document's label line holds its 26 pt copy button (macOS); a
     // plain label is one line of its text.
@@ -849,14 +923,7 @@ fn field(
         3.0
     });
     if value.is_null() {
-        Text::one(
-            look,
-            crate::ui::grid::data_role(look),
-            "NULL",
-            palette.faint,
-        )
-        .layout(ui.ctx())
-        .label(ui);
+        crate::ui::grid::null_label(ui, look, palette);
         return;
     }
     if let Some(doc) = doc {
@@ -888,11 +955,44 @@ fn field(
         }
         return;
     }
-    let Some(formatted) = texts.and_then(|texts| texts.fields.get(col)) else {
+    let Some(formatted) = formatted else {
         return;
     };
+    let say = |text: &'static str| look.label(&gettext(locale, text));
+    match value {
+        Value::Bytes(bytes) => {
+            binary(ui, bytes, &formatted.short, look, palette, locale);
+            return;
+        }
+        Value::Text(text) => {
+            let marks = crate::ui::grid::marks(ui.ctx(), look);
+            if let Some(shown) = format::blank_text(text, marks) {
+                let note = if text.is_empty() {
+                    say("empty string")
+                } else {
+                    say("whitespace only")
+                };
+                stand_in(ui, &shown, &note, look, palette);
+                return;
+            }
+            if listed(text, column)
+                && let Some(array) = format::array_items(text)
+            {
+                if array.items.is_empty() {
+                    stand_in(ui, "{}", &say("empty array"), look, palette);
+                } else {
+                    elements(ui, &array, look, palette, locale);
+                }
+                return;
+            }
+        }
+        Value::Null | Value::Bool(_) | Value::Int(_) | Value::Float(_) => {}
+    }
     let expanded_id = Id::new(("row-panel-expanded", tab, tab_id, request, row, col));
     let expanded: bool = ui.data(|data| data.get_temp(expanded_id)).unwrap_or(false);
+    // Whether the value takes more lines than the panel shows at first:
+    // known once it is laid out at the width it gets.
+    let mut tall = false;
     let long = formatted.full.is_some();
     let shown = match &formatted.full {
         Some(full) if expanded => full,
@@ -948,17 +1048,37 @@ fn field(
         let room = (ui.available_width() - link).max(24.0);
         ui.allocate_ui(vec2(room, 0.0), |ui| {
             let mut layouter = crate::typography::layouter(look, role, color);
-            ui.add(
-                TextEdit::multiline(&mut shown.as_str())
-                    .font(role.font_id(look.faces))
-                    // Flush with the field name above.
-                    .frame(egui::Frame::NONE)
-                    .margin(egui::Margin::ZERO)
-                    .desired_width(room)
-                    .desired_rows(1)
-                    .layouter(&mut layouter),
-            )
-            .labelled_by(name_id);
+            tall = layouter(ui, &shown.as_str(), room).rows.len() > CLAMP_ROWS;
+            let mut edit = |ui: &mut egui::Ui| {
+                ui.add(
+                    TextEdit::multiline(&mut shown.as_str())
+                        .font(role.font_id(look.faces))
+                        // Flush with the field name above.
+                        .frame(egui::Frame::NONE)
+                        .margin(egui::Margin::ZERO)
+                        .desired_width(room)
+                        .desired_rows(1)
+                        .layouter(&mut layouter),
+                )
+                .labelled_by(name_id);
+            };
+            if tall && !expanded {
+                // The first lines, the last of them fading out (macOS):
+                // "Show all" below gives the rest.
+                let line = line_of(ui, role, look);
+                let size = vec2(room, CLAMP_ROWS as f32 * line);
+                let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
+                let mut child = ui.new_child(egui::UiBuilder::new().max_rect(rect));
+                child.set_clip_rect(rect.intersect(ui.clip_rect()));
+                edit(&mut child);
+                if !look.terminal {
+                    let last =
+                        Rect::from_min_max(pos2(rect.left(), rect.bottom() - line), rect.max);
+                    fade(ui, last, palette.window);
+                }
+            } else {
+                edit(ui);
+            }
         });
         if let Some((text, target)) = follow {
             ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
@@ -982,7 +1102,7 @@ fn field(
             });
         }
     });
-    if long {
+    if long || tall {
         let label = if expanded {
             gettext(locale, "Show less").into_owned()
         } else {
@@ -998,6 +1118,152 @@ fn field(
     }
 }
 
+/// The lines of a long value the panel shows before "Show all".
+const CLAMP_ROWS: usize = 3;
+
+/// Fades `rect` from clear at its top to `color` at its bottom: the last
+/// line of a value cut short runs out into the panel.
+fn fade(ui: &egui::Ui, rect: Rect, color: egui::Color32) {
+    let clear = color.gamma_multiply(0.0);
+    let mut mesh = egui::Mesh::default();
+    mesh.colored_vertex(rect.left_top(), clear);
+    mesh.colored_vertex(rect.right_top(), clear);
+    mesh.colored_vertex(rect.right_bottom(), color);
+    mesh.colored_vertex(rect.left_bottom(), color);
+    mesh.add_triangle(0, 1, 2);
+    mesh.add_triangle(0, 2, 3);
+    ui.painter().add(mesh);
+}
+
+/// A value with nothing to see: what the grid writes for it (`''`, a mark
+/// for each space, `{}`), and in words what that is.
+fn stand_in(ui: &mut egui::Ui, shown: &str, note: &str, look: &Look, palette: &Palette) {
+    ui.horizontal(|ui| {
+        ui.spacing_mut().item_spacing.x = 6.0;
+        let role = crate::ui::grid::data_role(look);
+        Text::one(look, role, shown, palette.faint)
+            .layout(ui.ctx())
+            .label(ui);
+        Text::one(look, widgets::secondary(look), note, palette.secondary)
+            .layout(ui.ctx())
+            .label(ui);
+    });
+}
+
+/// The most elements of an array the panel lists.
+const ELEMENTS_SHOWN: usize = 100;
+
+/// An array's elements, one to a line after their positions (which count
+/// from 1, as PostgreSQL's do).
+fn elements(
+    ui: &mut egui::Ui,
+    array: &format::ArrayItems<'_>,
+    look: &Look,
+    palette: &Palette,
+    locale: crate::i18n::Locale,
+) {
+    let role = TextRole::pick(look, TextRole::MonoSecondary, TextRole::OSecondary);
+    let shown = array.items.len().min(ELEMENTS_SHOWN);
+    // The positions share a column as wide as the last one's.
+    let column = role.width(ui.ctx(), look.faces, &format!("[{shown}]"));
+    let line = line_of(ui, role, look);
+    ui.scope(|ui| {
+        ui.spacing_mut().item_spacing = vec2(8.0, 2.0);
+        for (index, item) in array.items.iter().take(ELEMENTS_SHOWN).enumerate() {
+            ui.horizontal_top(|ui| {
+                let (place, _) = ui.allocate_exact_size(vec2(column, line), Sense::hover());
+                let position = format!("[{}]", index + 1);
+                widgets::paint_text(
+                    ui,
+                    place.left(),
+                    place.center().y,
+                    Text::one(look, role, &position, palette.faint),
+                );
+                Text::one(look, role, &format::display_safe(item), palette.text)
+                    .wrap(ui.available_width())
+                    .layout(ui.ctx())
+                    .label(ui);
+            });
+        }
+        let more = array.items.len() - shown;
+        if more > 0 {
+            let rest = format!(
+                "… {} {}",
+                format::group_digits(more as u64),
+                look.label(&gettext(locale, "more elements"))
+            );
+            Text::one(look, role, &rest, palette.faint)
+                .layout(ui.ctx())
+                .label(ui);
+        }
+    });
+}
+
+/// A binary value: its first bytes, and how many more there are. The whole
+/// of it is a file's worth, which Copy gives.
+fn binary(
+    ui: &mut egui::Ui,
+    bytes: &[u8],
+    preview: &str,
+    look: &Look,
+    palette: &Palette,
+    locale: crate::i18n::Locale,
+) {
+    let say = |text: &'static str| look.label(&gettext(locale, text));
+    if bytes.is_empty() {
+        Text::one(
+            look,
+            widgets::secondary(look),
+            &say("no bytes"),
+            palette.secondary,
+        )
+        .layout(ui.ctx())
+        .label(ui);
+        return;
+    }
+    let role = TextRole::pick(look, TextRole::Json, TextRole::OSecondary);
+    let more = bytes.len().saturating_sub(format::HEX_PREVIEW);
+    let rest = (more > 0).then(|| {
+        let count = format::group_digits(more as u64);
+        if look.terminal {
+            format!("… {count} {}", say("more"))
+        } else {
+            format!(
+                "… {count} {} · {} {} {}",
+                say("more bytes"),
+                say("first"),
+                format::HEX_PREVIEW,
+                say("shown")
+            )
+        }
+    });
+    let lines = |ui: &mut egui::Ui| {
+        ui.spacing_mut().item_spacing.y = 0.0;
+        Text::one(look, role, preview, palette.secondary)
+            .layout(ui.ctx())
+            .label(ui);
+        if let Some(rest) = &rest {
+            Text::one(look, role, rest, palette.faint)
+                .layout(ui.ctx())
+                .label(ui);
+        }
+    };
+    if look.terminal {
+        ui.scope(lines);
+    } else {
+        Frame::new()
+            .fill(palette.panel)
+            // The design's 1 pt border outside 8 and 10 of padding.
+            .stroke(Stroke::new(1.0, palette.surface_hover))
+            .corner_radius(CornerRadius::same(look.radius))
+            .inner_margin(egui::Margin::symmetric(10, 8))
+            .show(ui, |ui| {
+                ui.set_width(ui.available_width());
+                lines(ui);
+            });
+    }
+}
+
 /// `books` becomes `book`, for "Open book →".
 fn singular(table: &str) -> &str {
     table
````

- [ ] **Step 4: Run the tests**

Run: `~/.cargo/bin/cargo test --locked --lib ui::row_panel`
Expected: PASS.

Then all of the library's: `~/.cargo/bin/cargo test --locked --lib`
Expected: `test result: ok. 869 passed; 0 failed; 1 ignored`.

- [ ] **Step 5: Render, and look**

Run: `~/.cargo/bin/cargo test --locked --features shots --lib shots -- --ignored`

`target/shots/mock-macos-values.png` beside the artboard's inspector: the title's three lines and fade, "Show all"; `languages · text[] · 2 elements` over `[1] en`, `[2] fr`; the hex box with its "… 49,334 more bytes · first 24 shown". `mock-omarchy-values.png` for the same in the terminal's half-width cells.

Put right what differs, in the code this task wrote; keep the tests.

- [ ] **Step 6: Format and lint**

```bash
~/.cargo/bin/cargo fmt --all --check
~/.cargo/bin/cargo clippy --locked --workspace --all-targets -- -D warnings
~/.cargo/bin/cargo clippy --locked --workspace --all-targets --features shots -- -D warnings
```

Expected: no output.

- [ ] **Step 7: Commit point**

Stop and report. If the user has asked for commits:

```bash
git add -A src AGENTS.md
git commit -S -m "Show awkward values whole in the row panel" -m "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

### Task 9: One painter for keyboard focus

**Files:**
- Create: `src/ui/focus.rs`
- Modify: `src/ui/mod.rs` (`pub mod focus;`)
- Modify: `src/app.rs`

A new module, `src/ui/focus.rs`, and two calls round the frame in `App::frame_ui`. `begin_frame` notes whether the last input was a key or the pointer; `paint`, once everything is drawn, rings whatever egui says has the keyboard: an outer ring by default, a field's border and halo for a text field, or the form a widget asked for with `hint`. The module also holds what the next tasks wire in: panes (`pane`, `on_control`), segments (`segment`), the terminal's pane border (`pane_border`) and the regions F6 steps through (`region`, `claim`, `step`). It is created whole here, with its tests; nothing but the painter is used yet.

In `src/ui/focus.rs`: `the_keyboard_shows_focus_and_the_pointer_hides_it`, `a_text_field_takes_the_accent_border_and_a_halo`, `focus_the_pointer_gave_is_not_shown`. The file is new, so its tests come with it in one block.

- [ ] **Step 1: Write the code**

````diff
diff --git a/src/app.rs b/src/app.rs
--- a/src/app.rs
+++ b/src/app.rs
@@ -2754,11 +2754,15 @@ impl App {
     pub fn frame_ui(&mut self, ui: &mut egui::Ui) {
         self.poll_backend();
         self.apply_actions();
+        // Before the shortcuts take their keys: what the user works with.
+        crate::ui::focus::begin_frame(ui.ctx());
         // A dialog takes the keyboard: no shortcut acts behind it.
         if self.dialog.is_none() {
             crate::ui::keys::handle(self, ui.ctx());
         }
         crate::ui::show(self, ui);
+        // Over everything drawn: the ring of what has the keyboard.
+        crate::ui::focus::paint(ui.ctx(), &self.look, &self.palette);
         crate::ui::keys::after_frame(ui.ctx());
         self.apply_actions();
         let title = self.window_title();
diff --git a/src/ui/focus.rs b/src/ui/focus.rs
new file mode 100644
--- /dev/null
+++ b/src/ui/focus.rs
@@ -0,0 +1,562 @@
+//! Keyboard focus, shown: a ring round whatever has the keyboard, once the
+//! keyboard (not the pointer) put it there. One painter draws it for every
+//! widget, egui's own included, so no Tab stop is ever without one; a
+//! widget that wants another form than the default says so with [`hint`].
+
+use egui::{Color32, CornerRadius, EventFilter, Id, Key, Modifiers, Rect, Response, Sense, Stroke};
+use egui::{StrokeKind, Ui};
+
+use crate::theme::{Look, Palette};
+
+/// How far off a control its ring starts, and how wide a ring is.
+const GAP: f32 = 2.0;
+const WIDTH: f32 = 2.0;
+/// A field's halo: this wide, the accent at a quarter.
+const HALO: f32 = 3.0;
+/// A field's border.
+const BORDER: f32 = 1.0;
+
+/// How the widget that has the keyboard shows it.
+#[derive(Clone, Copy, Debug, PartialEq, Eq)]
+pub enum Ring {
+    /// Off the control, with a gap that shows what the control sits on:
+    /// buttons, chips, links. The default.
+    Outer { radius: u8 },
+    /// Inside the item's edge, where a ring outside it would be cut by the
+    /// list it scrolls in: rows, tabs, cells.
+    Inset { radius: u8 },
+    /// On the item's edge, with no gap: a segment in its track.
+    Edge { radius: u8 },
+    /// A text field: its border in the accent and a soft halo round it.
+    Field { radius: u8 },
+    /// The widget shows it by itself (a caret in text that has no box, the
+    /// terminal's reversed button).
+    Own,
+}
+
+/// What the focused widget said of its ring this frame.
+#[derive(Clone, Copy)]
+struct Hint {
+    id: Id,
+    rect: Rect,
+    ring: Ring,
+    clip: Rect,
+}
+
+fn visible_id() -> Id {
+    Id::new("focus-visible")
+}
+
+fn hint_id() -> Id {
+    Id::new("focus-hint")
+}
+
+fn pane_id() -> Id {
+    Id::new("focus-pane")
+}
+
+fn regions_id() -> Id {
+    Id::new("focus-regions")
+}
+
+/// The parts of the window F6 steps between, in the order it takes them.
+#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
+pub enum Region {
+    /// The connection bar: Connections, then a chip for each connection.
+    Header,
+    /// The sidebar's filter.
+    Search,
+    /// The object tree.
+    Tree,
+    /// The open tables and editors.
+    Tabs,
+    /// A table's header: the Data and Structure switch.
+    Toolbar,
+    /// A SQL editor's script.
+    Editor,
+    /// The rows.
+    Grid,
+    /// The row panel.
+    Panel,
+}
+
+impl Region {
+    /// The panes the terminal look steps between with ctrl+h and ctrl+l.
+    pub const PANES: [Region; 3] = [Region::Tree, Region::Grid, Region::Panel];
+}
+
+/// Where the regions are, and which one F6 asked for.
+#[derive(Clone, Default)]
+struct Regions {
+    /// The regions drawn so far this frame.
+    drawn: Vec<(Region, Rect)>,
+    /// The ones the frame before drew: what a key pressed now steps among.
+    before: Vec<(Region, Rect)>,
+    /// The region asked for, and in which frame.
+    wanted: Option<(Region, u64)>,
+}
+
+/// Says where `region` is this frame, so F6 can step to it and from it.
+pub fn region(ui: &Ui, region: Region, rect: Rect) {
+    ui.data_mut(|data| {
+        let regions: &mut Regions = data.get_temp_mut_or_default(regions_id());
+        regions.drawn.push((region, rect));
+    });
+}
+
+/// Gives `response`'s widget the keyboard when F6 asked for `region`: call
+/// it for the control a region starts with.
+pub fn claim(ui: &Ui, region: Region, response: &Response) {
+    let asked = ui.data_mut(|data| {
+        let regions: &mut Regions = data.get_temp_mut_or_default(regions_id());
+        let asked = matches!(regions.wanted, Some((wanted, _)) if wanted == region);
+        if asked {
+            regions.wanted = None;
+        }
+        asked
+    });
+    if asked {
+        response.request_focus();
+    }
+}
+
+/// F6: asks for the region after (or, not `forward`, before) the one the
+/// keyboard is in, among the ones on screen and in `among` if given. With
+/// the keyboard nowhere it counts from `from`.
+pub fn step(ctx: &egui::Context, forward: bool, among: Option<&[Region]>, from: Option<Region>) {
+    let at = ctx
+        .memory(|memory| memory.focused())
+        .and_then(|id| ctx.read_response(id))
+        .map(|response| response.rect.center());
+    let now = ctx.cumulative_frame_nr();
+    ctx.data_mut(|data| {
+        let regions: &mut Regions = data.get_temp_mut_or_default(regions_id());
+        let mut stops: Vec<Region> = regions
+            .before
+            .iter()
+            .map(|(region, _)| *region)
+            .filter(|region| among.is_none_or(|among| among.contains(region)))
+            .collect();
+        stops.sort();
+        stops.dedup();
+        // The region the keyboard is in: the smallest one round it.
+        let current = at
+            .and_then(|at| {
+                regions
+                    .before
+                    .iter()
+                    .filter(|(_, rect)| rect.contains(at))
+                    .min_by(|(_, a), (_, b)| a.area().total_cmp(&b.area()))
+                    .map(|(region, _)| *region)
+            })
+            .or(from);
+        let next = match (current, forward) {
+            (Some(current), true) => stops.iter().find(|stop| **stop > current),
+            (Some(current), false) => stops.iter().rev().find(|stop| **stop < current),
+            (None, _) => None,
+        };
+        // Past the last one it starts over.
+        let next = next.or(if forward { stops.first() } else { stops.last() });
+        regions.wanted = next.map(|region| (*region, now));
+    });
+    // The region takes the keyboard when it is next drawn.
+    ctx.request_repaint();
+}
+
+/// Makes `response`'s widget a pane: one Tab stop for a list or a grid
+/// whose keys the app's shortcuts handle. With the keyboard on it Enter,
+/// Space and the arrows act on what it shows: egui neither presses it like
+/// a button nor moves focus off it with the arrows.
+pub fn pane(ui: &Ui, response: &Response) {
+    if !response.has_focus() {
+        return;
+    }
+    let arrows = EventFilter {
+        horizontal_arrows: true,
+        vertical_arrows: true,
+        ..Default::default()
+    };
+    ui.memory_mut(|memory| memory.set_focus_lock_filter(response.id, arrows));
+    ui.data_mut(|data| data.insert_temp(pane_id(), response.id));
+}
+
+/// Whether the keyboard is on a control of its own (a button, a field, a
+/// tab): it takes Enter and Space itself, and the arrows move focus from
+/// it. On a pane, or nowhere, the shortcuts have those keys.
+pub fn on_control(ctx: &egui::Context) -> bool {
+    ctx.memory(|memory| memory.focused())
+        .is_some_and(|id| ctx.data(|data| data.get_temp::<Id>(pane_id())) != Some(id))
+}
+
+/// The terminal look's mark of the pane the keys go to: a 2 pt accent line
+/// round it, inside its edge. The other looks mark the item, not the pane.
+pub fn pane_border(ui: &Ui, rect: Rect, look: &Look, palette: &Palette) {
+    if look.terminal {
+        ui.painter().rect_stroke(
+            rect,
+            CornerRadius::ZERO,
+            Stroke::new(WIDTH, palette.accent),
+            StrokeKind::Inside,
+        );
+    }
+}
+
+/// One of the `count` choices of a segmented control, the one at `index`:
+/// the chosen one is the control's only Tab stop, and with the keyboard on
+/// it ← and → choose its neighbours. Returns its response and the choice
+/// the arrows made, if any; the keyboard follows that choice once the
+/// caller has made it the chosen one.
+pub fn segment(
+    ui: &Ui,
+    rect: Rect,
+    id: Id,
+    group: Id,
+    (index, count): (usize, usize),
+    chosen: bool,
+) -> (Response, Option<usize>) {
+    let sense = if chosen { Sense::click() } else { Sense::CLICK };
+    let response = ui.interact(rect, id, sense);
+    let now = ui.ctx().cumulative_frame_nr();
+    // The choice the arrows made, and when: taken up within two frames or
+    // not at all (the caller may have refused it).
+    let moved: Option<(usize, u64)> = ui.data(|data| data.get_temp(group));
+    if let Some((target, when)) = moved
+        && chosen
+        && target == index
+    {
+        if now <= when + 2 {
+            response.request_focus();
+        }
+        ui.data_mut(|data| data.remove::<(usize, u64)>(group));
+    }
+    let mut picked = None;
+    if response.has_focus() {
+        let arrows = EventFilter {
+            horizontal_arrows: true,
+            ..Default::default()
+        };
+        ui.memory_mut(|memory| memory.set_focus_lock_filter(response.id, arrows));
+        let (right, left) = ui.input_mut(|input| {
+            (
+                input.consume_key(Modifiers::NONE, Key::ArrowRight),
+                input.consume_key(Modifiers::NONE, Key::ArrowLeft),
+            )
+        });
+        let next = match (right, left) {
+            (true, false) => (index + 1).min(count.saturating_sub(1)),
+            (false, true) => index.saturating_sub(1),
+            _ => index,
+        };
+        if next != index {
+            picked = Some(next);
+            ui.data_mut(|data| data.insert_temp(group, (next, now)));
+        }
+    }
+    (response, picked)
+}
+
+/// Notes what this frame's input says of how the user works: a key makes
+/// focus visible, a pointer press hides it. A screen reader that moves
+/// focus counts as a key. Call before anything reads the frame's keys.
+pub fn begin_frame(ctx: &egui::Context) {
+    let keyboard = ctx.input(|input| {
+        input.events.iter().rev().find_map(|event| match event {
+            egui::Event::Key { pressed: true, .. } => Some(true),
+            egui::Event::AccessKitActionRequest(request)
+                if request.action == egui::accesskit::Action::Focus =>
+            {
+                Some(true)
+            }
+            egui::Event::PointerButton { pressed: true, .. } => Some(false),
+            _ => None,
+        })
+    });
+    let now = ctx.cumulative_frame_nr();
+    ctx.data_mut(|data| {
+        if let Some(keyboard) = keyboard {
+            data.insert_temp(visible_id(), keyboard);
+        }
+        let regions: &mut Regions = data.get_temp_mut_or_default(regions_id());
+        regions.before = std::mem::take(&mut regions.drawn);
+        // A region asked for and not drawn since is not there to take it.
+        if regions.wanted.is_some_and(|(_, when)| now > when + 1) {
+            regions.wanted = None;
+        }
+    });
+}
+
+/// Whether focus is shown: the keyboard was used last, not the pointer.
+pub fn visible(ctx: &egui::Context) -> bool {
+    ctx.data(|data| data.get_temp(visible_id()))
+        .unwrap_or(false)
+}
+
+/// Whether `response`'s widget has the keyboard and should show it: for a
+/// widget that draws its own focus ([`Ring::Own`]).
+pub fn shown(response: &Response) -> bool {
+    response.has_focus() && visible(&response.ctx)
+}
+
+/// Says how `response`'s widget shows focus: `ring` round `rect`. Without a
+/// hint a widget gets [`Ring::Outer`] round its own rectangle (a text field
+/// [`Ring::Field`]), at the look's radius.
+pub fn hint(ui: &Ui, response: &Response, rect: Rect, ring: Ring) {
+    if !response.has_focus() {
+        return;
+    }
+    let hint = Hint {
+        id: response.id,
+        rect,
+        ring,
+        clip: ui.clip_rect(),
+    };
+    ui.data_mut(|data| data.insert_temp(hint_id(), hint));
+}
+
+/// Draws the ring of whatever has the keyboard. Call once the frame's
+/// widgets are drawn.
+pub fn paint(ctx: &egui::Context, look: &Look, palette: &Palette) {
+    let Some(id) = ctx.memory(|memory| memory.focused()) else {
+        return;
+    };
+    if !visible(ctx) {
+        return;
+    }
+    let Some(response) = ctx.read_response(id) else {
+        return;
+    };
+    let hint = ctx
+        .data(|data| data.get_temp::<Hint>(hint_id()))
+        .filter(|hint| hint.id == id);
+    let (rect, ring, clip) = match hint {
+        Some(hint) => (hint.rect, hint.ring, hint.clip),
+        None => {
+            let ring = if ctx.text_edit_focused() {
+                Ring::Field {
+                    radius: look.radius,
+                }
+            } else {
+                Ring::Outer {
+                    radius: look.radius,
+                }
+            };
+            (response.rect, ring, Rect::EVERYTHING)
+        }
+    };
+    let painter = ctx.layer_painter(response.layer_id);
+    let accent = Stroke::new(WIDTH, palette.accent);
+    // Concentric with the control: a ring further out is rounder. A
+    // square control keeps a square ring.
+    let round = |radius: u8, out: f32| {
+        if radius == 0 {
+            CornerRadius::ZERO
+        } else {
+            CornerRadius::same(radius.saturating_add(out as u8))
+        }
+    };
+    match ring {
+        Ring::Outer { radius } => {
+            // The ring stands outside the control, so it may pass the edge
+            // of what clips the control by its own reach.
+            painter
+                .with_clip_rect(clip.expand(GAP + WIDTH))
+                .rect_stroke(
+                    rect.expand(GAP),
+                    round(radius, GAP),
+                    accent,
+                    StrokeKind::Outside,
+                );
+        }
+        Ring::Inset { radius } => {
+            painter.with_clip_rect(clip).rect_stroke(
+                rect,
+                CornerRadius::same(radius),
+                accent,
+                StrokeKind::Inside,
+            );
+        }
+        Ring::Edge { radius } => {
+            painter.with_clip_rect(clip.expand(WIDTH)).rect_stroke(
+                rect,
+                CornerRadius::same(radius),
+                accent,
+                StrokeKind::Outside,
+            );
+        }
+        Ring::Field { radius } => {
+            let painter = painter.with_clip_rect(clip.expand(HALO));
+            // The terminal's fields are square and take the border alone.
+            if !look.terminal {
+                painter.rect_stroke(
+                    rect,
+                    round(radius, 0.0),
+                    Stroke::new(HALO, halo(palette)),
+                    StrokeKind::Outside,
+                );
+            }
+            painter.rect_stroke(
+                rect,
+                CornerRadius::same(radius),
+                Stroke::new(BORDER, palette.accent),
+                StrokeKind::Inside,
+            );
+        }
+        Ring::Own => {}
+    }
+}
+
+/// A field's halo: the accent at a quarter.
+pub fn halo(palette: &Palette) -> Color32 {
+    palette.accent.gamma_multiply(0.25)
+}
+
+#[cfg(test)]
+mod tests {
+    use super::*;
+    use egui::{Key, Modifiers, vec2};
+
+    /// One frame of two buttons and a text field, with `events`; the
+    /// second button asks for an inset ring. Returns what was painted.
+    fn frame(
+        ctx: &egui::Context,
+        look: &Look,
+        palette: &Palette,
+        events: Vec<egui::Event>,
+    ) -> Vec<egui::epaint::ClippedShape> {
+        let input = egui::RawInput {
+            events,
+            ..Default::default()
+        };
+        let mut output = ctx.run_ui(input, |ui| {
+            begin_frame(ui.ctx());
+            egui::CentralPanel::default().show(ui, |ui| {
+                let _ = ui.button("First");
+                let (rect, second) = ui.allocate_exact_size(vec2(80.0, 24.0), egui::Sense::click());
+                hint(ui, &second, rect, Ring::Inset { radius: 0 });
+                let mut text = String::new();
+                ui.text_edit_singleline(&mut text);
+            });
+            paint(ui.ctx(), look, palette);
+        });
+        output.textures_delta.clear();
+        output.shapes
+    }
+
+    /// The kinds of the accent outlines in `shapes`, by their width.
+    fn rings(shapes: &[egui::epaint::ClippedShape], palette: &Palette) -> Vec<(StrokeKind, f32)> {
+        shapes
+            .iter()
+            .filter_map(|clipped| match &clipped.shape {
+                egui::Shape::Rect(rect)
+                    if rect.fill == Color32::TRANSPARENT
+                        && (rect.stroke.color == palette.accent
+                            || rect.stroke.color == halo(palette)) =>
+                {
+                    Some((rect.stroke_kind, rect.stroke.width))
+                }
+                _ => None,
+            })
+            .collect()
+    }
+
+    fn context(look: &Look, palette: &Palette) -> egui::Context {
+        let ctx = egui::Context::default();
+        crate::theme::install(&ctx, false, look);
+        crate::theme::apply(&ctx, palette, look);
+        ctx
+    }
+
+    fn tab() -> Vec<egui::Event> {
+        vec![
+            crate::testing::key(Key::Tab, Modifiers::NONE),
+            crate::testing::release(Key::Tab, Modifiers::NONE),
+        ]
+    }
+
+    #[test]
+    fn the_keyboard_shows_focus_and_the_pointer_hides_it() {
+        for look in Look::ALL {
+            for palette in [Palette::light(), Palette::dark()] {
+                let ctx = context(&look, &palette);
+                let draw = |events| rings(&frame(&ctx, &look, &palette, events), &palette);
+                assert_eq!(draw(Vec::new()), [], "{}: nothing focused", look.name);
+                // Tab lands on the first button: the default, an outer ring.
+                draw(tab());
+                assert_eq!(
+                    draw(Vec::new()),
+                    [(StrokeKind::Outside, WIDTH)],
+                    "{}",
+                    look.name
+                );
+                // The second asked for an inset one.
+                draw(tab());
+                assert_eq!(
+                    draw(Vec::new()),
+                    [(StrokeKind::Inside, WIDTH)],
+                    "{}",
+                    look.name
+                );
+                // A press of the pointer elsewhere hides the ring.
+                let press = egui::Event::PointerButton {
+                    pos: egui::pos2(600.0, 500.0),
+                    button: egui::PointerButton::Primary,
+                    pressed: true,
+                    modifiers: Modifiers::NONE,
+                };
+                draw(vec![
+                    egui::Event::PointerMoved(egui::pos2(600.0, 500.0)),
+                    press,
+                ]);
+                assert_eq!(draw(Vec::new()), [], "{}: the pointer hides it", look.name);
+            }
+        }
+    }
+
+    #[test]
+    fn a_text_field_takes_the_accent_border_and_a_halo() {
+        for look in Look::ALL {
+            let palette = Palette::light();
+            let ctx = context(&look, &palette);
+            let draw = |events| rings(&frame(&ctx, &look, &palette, events), &palette);
+            for _ in 0..3 {
+                draw(tab());
+            }
+            let rings = draw(Vec::new());
+            assert!(
+                rings.contains(&(StrokeKind::Inside, BORDER)),
+                "{}: {rings:?}",
+                look.name
+            );
+            assert_eq!(
+                rings.contains(&(StrokeKind::Outside, HALO)),
+                !look.terminal,
+                "{}: {rings:?}",
+                look.name
+            );
+        }
+    }
+
+    #[test]
+    fn focus_the_pointer_gave_is_not_shown() {
+        let (look, palette) = (Look::macos(), Palette::light());
+        let ctx = context(&look, &palette);
+        let draw = |ctx: &egui::Context| {
+            let mut id = Id::NULL;
+            let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
+                begin_frame(ui.ctx());
+                egui::CentralPanel::default().show(ui, |ui| {
+                    id = ui.button("First").id;
+                });
+                paint(ui.ctx(), &look, &palette);
+            });
+            output.textures_delta.clear();
+            (id, rings(&output.shapes, &palette))
+        };
+        let (id, _) = draw(&ctx);
+        // Focus with no key behind it: code asked for it after a click.
+        ctx.memory_mut(|memory| memory.request_focus(id));
+        draw(&ctx);
+        assert_eq!(draw(&ctx).1, []);
+        assert!(!visible(&ctx));
+    }
+}
diff --git a/src/ui/mod.rs b/src/ui/mod.rs
--- a/src/ui/mod.rs
+++ b/src/ui/mod.rs
@@ -7,6 +7,7 @@ pub mod data_view;
 #[cfg(test)]
 mod env_tests;
 pub mod filter_bar;
+pub mod focus;
 pub mod format;
 pub mod grid;
 pub mod help;
````

- [ ] **Step 2: Run the tests**

Run: `~/.cargo/bin/cargo test --locked --lib ui::focus`
Expected: PASS.

Then all of the library's: `~/.cargo/bin/cargo test --locked --lib`
Expected: `test result: ok. 872 passed; 0 failed; 1 ignored`.

- [ ] **Step 3: Format and lint**

```bash
~/.cargo/bin/cargo fmt --all --check
~/.cargo/bin/cargo clippy --locked --workspace --all-targets -- -D warnings
~/.cargo/bin/cargo clippy --locked --workspace --all-targets --features shots -- -D warnings
```

Expected: no output.

- [ ] **Step 4: Commit point**

Stop and report. If the user has asked for commits:

```bash
git add -A src AGENTS.md
git commit -S -m "Ring whatever has the keyboard, in one place" -m "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

### Task 10: Each control's ring

**Files:**
- Modify: `src/ui/widgets.rs`
- Modify: `src/typography.rs`
- Modify: `src/ui/sidebar.rs`, `object_tabs.rs`, `quick_open.rs`, `picker.rs`
- Modify: `src/ui/sql_results.rs`, `sql_editor.rs`, `sql_text.rs`, `connect_dialog/choice.rs`
- Modify: `src/ui/row_panel.rs`, `data_view.rs`
- Modify: `src/testing.rs`
- Modify: `src/ui/mod.rs` (tests)
- Modify: `src/shots.rs`

The six rings drawn by hand go (`primary_focus_ring` and its uses), and each kind of control says which ring is its own: rows of lists and tabs an inset one, a segment one on its edge, a boxed field the field's, a text link a tight one, and text that only shows a caret none. In the terminal look a focused `ButtonSpec` is reversed instead of ringed. `Harness` learns to record outlined rectangles so a test can see a ring.

- [ ] **Step 1: Write the tests**

New in `src/ui/mod.rs`: `every_tab_stop_shows_where_the_keyboard_is` (Tab sixty times through a workspace in each look: at every stop that is not a text field something is drawn in the accent) and `the_pointer_takes_the_ring_away`. Removed: `a_focused_primary_button_draws_a_focus_ring` in `widgets.rs`, which tested the function this task deletes; `focus.rs` tests the ring now.

````diff
diff --git a/src/testing.rs b/src/testing.rs
--- a/src/testing.rs
+++ b/src/testing.rs
@@ -25,6 +25,8 @@ pub struct Harness {
     pub fills: Vec<(egui::Rect, egui::Color32)>,
     /// The colour of every line and outline the last frame drew.
     pub strokes: Vec<egui::Color32>,
+    /// Every rectangle the last frame outlined, and with what.
+    pub outlines: Vec<(egui::Rect, egui::Stroke)>,
     /// How soon the last frame asked to be drawn again: at once is zero.
     pub repaint_after: std::time::Duration,
     #[cfg(feature = "shots")]
@@ -78,9 +80,11 @@ impl Harness {
         self.painted.clear();
         self.fills.clear();
         self.strokes.clear();
+        self.outlines.clear();
         for clipped in &output.shapes {
             collect_text(&clipped.shape, &mut self.painted);
             collect_paint(&clipped.shape, &mut self.fills, &mut self.strokes);
+            collect_outlines(&clipped.shape, &mut self.outlines);
         }
         let viewport = output.viewport_output.get(&egui::ViewportId::ROOT);
         self.viewport_commands = viewport
@@ -247,6 +251,18 @@ fn collect_text(shape: &egui::Shape, into: &mut Vec<(String, egui::Color32)>) {
     }
 }
 
+fn collect_outlines(shape: &egui::Shape, into: &mut Vec<(egui::Rect, egui::Stroke)>) {
+    match shape {
+        egui::Shape::Vec(shapes) => shapes
+            .iter()
+            .for_each(|shape| collect_outlines(shape, into)),
+        egui::Shape::Rect(rect) if rect.stroke.width > 0.0 && rect.stroke.color.a() > 0 => {
+            into.push((rect.rect, rect.stroke));
+        }
+        _ => {}
+    }
+}
+
 fn collect_paint(
     shape: &egui::Shape,
     fills: &mut Vec<(egui::Rect, egui::Color32)>,
@@ -643,6 +659,7 @@ impl Harness {
             painted: Vec::new(),
             fills: Vec::new(),
             strokes: Vec::new(),
+            outlines: Vec::new(),
             repaint_after: std::time::Duration::MAX,
             #[cfg(feature = "shots")]
             renderer: None,
diff --git a/src/ui/mod.rs b/src/ui/mod.rs
--- a/src/ui/mod.rs
+++ b/src/ui/mod.rs
@@ -8286,6 +8286,58 @@ mod tests {
         }
     }
 
+    #[test]
+    fn every_tab_stop_shows_where_the_keyboard_is() {
+        for look in crate::theme::Look::ALL {
+            let (mut harness, _tab) = tree_harness();
+            harness.set_look(look);
+            harness.click("orders");
+            harness.answer_rows(crate::testing::page(3, true));
+            let accent = harness.app.palette.accent;
+            let mut seen = 0;
+            for _ in 0..60 {
+                harness.press(Key::Tab, Modifiers::NONE);
+                let name = focused_name(&harness.settle());
+                // A text field shows its caret, in a box or not.
+                if harness.ctx.memory(|memory| memory.focused().is_none())
+                    || harness.ctx.text_edit_focused()
+                {
+                    continue;
+                }
+                seen += 1;
+                // A ring, a field's border, or the terminal's reversed
+                // button: something is drawn in the accent for it.
+                let ringed = harness
+                    .outlines
+                    .iter()
+                    .any(|(_, stroke)| stroke.color == accent && stroke.width >= 1.0);
+                let reversed =
+                    look.terminal && harness.fills.iter().any(|(_, fill)| *fill == accent);
+                assert!(ringed || reversed, "{name} in {}", look.name);
+            }
+            assert!(seen > 10, "{}: {seen} stops", look.name);
+        }
+    }
+
+    #[test]
+    fn the_pointer_takes_the_ring_away() {
+        let (mut harness, _tab) = tree_harness();
+        harness.set_look(crate::theme::Look::macos());
+        harness.press(Key::Tab, Modifiers::NONE);
+        assert!(crate::ui::focus::visible(&harness.ctx));
+        let at = egui::pos2(900.0, 500.0);
+        harness.frame(vec![
+            egui::Event::PointerMoved(at),
+            egui::Event::PointerButton {
+                pos: at,
+                button: egui::PointerButton::Primary,
+                pressed: true,
+                modifiers: Modifiers::NONE,
+            },
+        ]);
+        assert!(!crate::ui::focus::visible(&harness.ctx));
+    }
+
     #[test]
     fn command_r_refreshes_the_tree_from_the_tree() {
         let (mut harness, _tab) = tree_harness();
diff --git a/src/ui/widgets.rs b/src/ui/widgets.rs
--- a/src/ui/widgets.rs
+++ b/src/ui/widgets.rs
@@ -1566,46 +1568,6 @@ mod tests {
         }
     }
 
-    /// Whether `shapes` hold a rectangle outlined with `stroke`.
-    fn has_ring(shapes: &[egui::epaint::ClippedShape], stroke: Stroke) -> bool {
-        shapes.iter().any(|clipped| match &clipped.shape {
-            egui::Shape::Rect(rect) => {
-                rect.stroke == stroke && rect.stroke_kind == StrokeKind::Outside
-            }
-            _ => false,
-        })
-    }
-
-    #[test]
-    fn a_focused_primary_button_draws_a_focus_ring() {
-        for look in crate::theme::Look::ALL {
-            let palette = crate::theme::Palette::dark();
-            let ctx = egui::Context::default();
-            crate::theme::install(&ctx, false, &look);
-            crate::theme::apply(&ctx, &palette, &look);
-            let frame = |ctx: &egui::Context| {
-                let mut id = egui::Id::NULL;
-                let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
-                    id = super::primary_button(ui, "Apply", &look, &palette).id;
-                });
-                output.textures_delta.clear();
-                (output.shapes, id)
-            };
-            let ring = super::primary_focus_ring(&palette);
-            assert_eq!(ring.width, 2.0);
-            let (shapes, id) = frame(&ctx);
-            assert!(!has_ring(&shapes, ring), "{}: no ring unfocused", look.name);
-            ctx.memory_mut(|memory| memory.request_focus(id));
-            frame(&ctx);
-            let (shapes, _) = frame(&ctx);
-            assert!(
-                has_ring(&shapes, ring),
-                "{}: a ring when focused",
-                look.name
-            );
-        }
-    }
-
     #[test]
     fn search_fields_are_as_tall_as_the_looks_controls() {
         for look in crate::theme::Look::ALL {
````

- [ ] **Step 2: Run them**

Run: `~/.cargo/bin/cargo test --locked --lib`

Expected, before the code below is there:

````text
These pass already: they guard what the code below keeps true.
test result: ok. 873 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out
````

- [ ] **Step 3: Write the code**

````diff
diff --git a/src/typography.rs b/src/typography.rs
--- a/src/typography.rs
+++ b/src/typography.rs
@@ -584,6 +584,15 @@ impl Laid {
         let (rect, response) = ui.allocate_exact_size(self.size(), sense);
         let text = self.galley.text().to_owned();
         response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Label, true, &text));
+        // Text that can be clicked is a link: a tight ring round its words.
+        if sense.senses_click() {
+            crate::ui::focus::hint(
+                ui,
+                &response,
+                rect,
+                crate::ui::focus::Ring::Outer { radius: 3 },
+            );
+        }
         if ui.is_rect_visible(rect) {
             self.paint(ui.painter(), rect.min);
         }
diff --git a/src/ui/connect_dialog/choice.rs b/src/ui/connect_dialog/choice.rs
--- a/src/ui/connect_dialog/choice.rs
+++ b/src/ui/connect_dialog/choice.rs
@@ -226,14 +226,12 @@ fn radio_group(
                 }
             }
         };
-        if response.has_focus() {
-            painter.rect_stroke(
-                cell.expand(1.0),
-                corner,
-                widgets::primary_focus_ring(palette),
-                StrokeKind::Outside,
-            );
-        }
+        crate::ui::focus::hint(
+            ui,
+            &response,
+            cell,
+            crate::ui::focus::Ring::Outer { radius: corner.nw },
+        );
         let mut x = cell.left() + pad;
         if dotted(index) {
             let dot = match (group, tint) {
diff --git a/src/ui/data_view.rs b/src/ui/data_view.rs
--- a/src/ui/data_view.rs
+++ b/src/ui/data_view.rs
@@ -660,6 +660,8 @@ fn where_line(
             .layouter(&mut layouter),
     );
     response.widget_info(|| WidgetInfo::labeled(WidgetType::TextEdit, true, "WHERE"));
+    // A line of the bar, not a box: its caret says where the keyboard is.
+    crate::ui::focus::hint(ui, &response, field, crate::ui::focus::Ring::Own);
     if focus {
         response.request_focus();
     }
diff --git a/src/ui/object_tabs.rs b/src/ui/object_tabs.rs
--- a/src/ui/object_tabs.rs
+++ b/src/ui/object_tabs.rs
@@ -8,6 +8,7 @@ use crate::i18n::gettext;
 use crate::model::{self, Action, ConnTabId, TabId};
 use crate::theme::{Icon, Look, Palette};
 use crate::typography::{Text, TextRole};
+use crate::ui::focus::{self, Ring};
 use crate::ui::widgets::{self, ButtonSpec, icon_button};
 
 /// The strip's height, per look.
@@ -276,6 +277,8 @@ fn mac_tab(
     let label = Text::one(look, role, tab.name, color).layout(ui.ctx());
     let width = mac_width(label.width(), tab.active);
     let (rect, response) = ui.allocate_exact_size(vec2(width, bar.height()), Sense::click());
+    // Inside the tab: the strip scrolls, and a ring outside would be cut.
+    focus::hint(ui, &response, rect, Ring::Inset { radius: 4 });
     let painter = ui.painter();
     if tab.active {
         // White down to the content, over the strip's rule.
@@ -338,6 +341,7 @@ fn terminal_tab(
     let (number_width, name_width) = (measure(&number), measure(&name));
     let width = 12.0 + number_width + 8.0 + name_width + 12.0;
     let (rect, response) = ui.allocate_exact_size(vec2(width, bar.height()), Sense::click());
+    focus::hint(ui, &response, rect, Ring::Inset { radius: 0 });
     let painter = ui.painter();
     if tab.active {
         painter.rect_filled(rect, CornerRadius::ZERO, palette.window);
diff --git a/src/ui/picker.rs b/src/ui/picker.rs
--- a/src/ui/picker.rs
+++ b/src/ui/picker.rs
@@ -636,6 +636,15 @@ fn row_response(
         Sense::click(),
     );
     response.widget_info(|| WidgetInfo::labeled(WidgetType::Button, true, &connection.name));
+    // A row of a list that scrolls: the ring inside it.
+    crate::ui::focus::hint(
+        ui,
+        &response,
+        rect,
+        crate::ui::focus::Ring::Inset {
+            radius: look.radius.saturating_sub(2),
+        },
+    );
     let activated = response.double_clicked()
         || (response.clicked() && !response.clicked_by(egui::PointerButton::Primary));
     if activated {
diff --git a/src/ui/quick_open.rs b/src/ui/quick_open.rs
--- a/src/ui/quick_open.rs
+++ b/src/ui/quick_open.rs
@@ -92,6 +92,14 @@ pub fn show(app: &mut App, ctx: &egui::Context) {
                             &look,
                             &palette,
                         );
+                        crate::ui::focus::hint(
+                            ui,
+                            &response,
+                            crate::ui::widgets::selection_rect(rect, &look),
+                            crate::ui::focus::Ring::Inset {
+                                radius: look.radius.saturating_sub(2),
+                            },
+                        );
                         // Long names end in "…" and show in full on hover.
                         let role = widgets::body(&look);
                         let color = crate::ui::widgets::selection_text(selected, &look, &palette);
diff --git a/src/ui/row_panel.rs b/src/ui/row_panel.rs
--- a/src/ui/row_panel.rs
+++ b/src/ui/row_panel.rs
@@ -1050,17 +1050,21 @@ fn field(
             let mut layouter = crate::typography::layouter(look, role, color);
             tall = layouter(ui, &shown.as_str(), room).rows.len() > CLAMP_ROWS;
             let mut edit = |ui: &mut egui::Ui| {
-                ui.add(
-                    TextEdit::multiline(&mut shown.as_str())
-                        .font(role.font_id(look.faces))
-                        // Flush with the field name above.
-                        .frame(egui::Frame::NONE)
-                        .margin(egui::Margin::ZERO)
-                        .desired_width(room)
-                        .desired_rows(1)
-                        .layouter(&mut layouter),
-                )
-                .labelled_by(name_id);
+                let value = ui
+                    .add(
+                        TextEdit::multiline(&mut shown.as_str())
+                            .font(role.font_id(look.faces))
+                            // Flush with the field name above.
+                            .frame(egui::Frame::NONE)
+                            .margin(egui::Margin::ZERO)
+                            .desired_width(room)
+                            .desired_rows(1)
+                            .layouter(&mut layouter),
+                    )
+                    .labelled_by(name_id);
+                // A value to read and select, not a field: its caret says
+                // where the keyboard is.
+                crate::ui::focus::hint(ui, &value, value.rect, crate::ui::focus::Ring::Own);
             };
             if tall && !expanded {
                 // The first lines, the last of them fading out (macOS):
diff --git a/src/ui/sidebar.rs b/src/ui/sidebar.rs
--- a/src/ui/sidebar.rs
+++ b/src/ui/sidebar.rs
@@ -10,6 +10,7 @@ use crate::i18n::{Locale, gettext};
 use crate::model::{Action, ConnTabId, TreeNode, TreeRow};
 use crate::theme::{Icon, Look, Palette};
 use crate::typography::{Text, TextRole};
+use crate::ui::focus;
 use crate::ui::format::display_safe;
 use crate::ui::widgets::{self, ButtonSpec, icon_button};
 
@@ -426,6 +427,7 @@ fn recent_section(
         });
         let selected = Some(object) == active;
         widgets::selection(ui, row, selected, response.hovered(), look, palette);
+        row_ring(ui, &response, widgets::selection_rect(row, look), look);
         let (role, color) = if selected {
             (TextRole::UiBodyStrong, palette.accent_hover)
         } else {
@@ -618,6 +620,13 @@ struct Marks<'a> {
     cursor: Option<&'a TreeNode>,
 }
 
+/// A row with the keyboard is ringed inside its highlight: a ring outside
+/// it would be cut by the list it scrolls in.
+fn row_ring(ui: &egui::Ui, response: &egui::Response, highlight: Rect, look: &Look) {
+    let radius = look.radius.saturating_sub(2);
+    focus::hint(ui, response, highlight, focus::Ring::Inset { radius });
+}
+
 /// How rows are drawn: the platform look and the colour palette.
 #[derive(Clone, Copy)]
 struct Skin<'a> {
@@ -713,11 +722,15 @@ fn tree_row(
     };
     widgets::selection(ui, band, selected, response.hovered(), look, palette);
     let highlight = widgets::selection_rect(band, look);
-    if marks.cursor == Some(&row.node) {
+    row_ring(ui, &response, highlight, look);
+    // The row the arrows are on, while the keyboard is in use: the ring a
+    // focused row takes. Apart from the open object's fill: one says where
+    // the keys are, the other what is open.
+    if marks.cursor == Some(&row.node) && focus::visible(ui.ctx()) && !response.has_focus() {
         ui.painter().rect_stroke(
-            highlight.shrink(0.5),
+            highlight,
             CornerRadius::same(look.radius.saturating_sub(2)),
-            egui::Stroke::new(1.0, palette.accent),
+            egui::Stroke::new(2.0, palette.accent),
             egui::StrokeKind::Inside,
         );
     }
diff --git a/src/ui/sql_editor.rs b/src/ui/sql_editor.rs
--- a/src/ui/sql_editor.rs
+++ b/src/ui/sql_editor.rs
@@ -799,14 +799,13 @@ fn menu(
             );
         }
     }
-    if response.has_focus() {
-        ui.painter().rect_stroke(
-            rect.expand(1.0),
-            CornerRadius::same(if look.terminal { 0 } else { look.radius }),
-            widgets::primary_focus_ring(palette),
-            StrokeKind::Outside,
-        );
-    }
+    let radius = if look.terminal { 0 } else { look.radius };
+    crate::ui::focus::hint(
+        ui,
+        &response,
+        rect,
+        crate::ui::focus::Ring::Outer { radius },
+    );
     // Where its button reads short, the menu says what it sets on hover.
     let response = if shape.short {
         response.on_hover_text(label.value.as_str())
diff --git a/src/ui/sql_results.rs b/src/ui/sql_results.rs
--- a/src/ui/sql_results.rs
+++ b/src/ui/sql_results.rs
@@ -6,7 +6,7 @@
 
 use std::time::Duration;
 
-use egui::{CornerRadius, Id, Rect, Sense, StrokeKind, Ui, WidgetInfo, WidgetType, pos2, vec2};
+use egui::{CornerRadius, Id, Rect, Sense, Ui, WidgetInfo, WidgetType, pos2, vec2};
 use tabletist_db::{Error, StatementOutcome, ValueKind};
 
 use crate::app::App;
@@ -486,14 +486,8 @@ fn pane_tabs(
                 palette.accent,
             );
         }
-        if response.has_focus() {
-            ui.painter().rect_stroke(
-                hit.expand(1.0),
-                CornerRadius::same(if look.terminal { 0 } else { look.radius }),
-                widgets::primary_focus_ring(palette),
-                StrokeKind::Outside,
-            );
-        }
+        let radius = if look.terminal { 0 } else { look.radius };
+        crate::ui::focus::hint(ui, &response, hit, crate::ui::focus::Ring::Outer { radius });
         if response.clicked() && !selected {
             actions.push(Action::SetResultPane {
                 tab: place.tab,
diff --git a/src/ui/sql_text.rs b/src/ui/sql_text.rs
--- a/src/ui/sql_text.rs
+++ b/src/ui/sql_text.rs
@@ -503,6 +503,13 @@ fn edit(ui: &mut Ui, sql_tab: &mut SqlTab, field: &Field<'_>) -> Edited {
         .desired_rows(rows as usize)
         .layouter(&mut layouter)
         .show(ui);
+    // The script fills its pane: the caret says where the keyboard is.
+    crate::ui::focus::hint(
+        ui,
+        &output.response,
+        output.response.rect,
+        crate::ui::focus::Ring::Own,
+    );
     // The name only: the field keeps the role and the value egui gave it.
     ui.ctx().accesskit_node_builder(field.id, |node| {
         node.set_label(field.name);
diff --git a/src/ui/widgets.rs b/src/ui/widgets.rs
--- a/src/ui/widgets.rs
+++ b/src/ui/widgets.rs
@@ -7,6 +7,7 @@ use egui::{
 
 use crate::theme::{DialogStyle, Icon, Look, Palette, Selection, TabStyle};
 use crate::typography::{Text, TextRole};
+use crate::ui::focus::{self, Ring};
 
 /// The body role in `look`.
 pub fn body(look: &Look) -> TextRole {
@@ -693,15 +694,9 @@ pub fn primary_fill(hovered: bool, focused: bool, pressed: bool, palette: &Palet
     }
 }
 
-/// The ring round a focused primary button: the full accent, 1 pt off the
-/// button so it keeps 3:1 against the background rather than blending into
-/// the fill.
-pub fn primary_focus_ring(palette: &Palette) -> Stroke {
-    Stroke::new(2.0, palette.accent)
-}
-
 /// The one accent-filled button in a dialog or view. Its own fill replaces
-/// egui's state visuals, so it draws hover, press and keyboard focus itself.
+/// egui's state visuals, so it draws hover and press itself; its focus ring
+/// is the one every control gets (see [`crate::ui::focus`]).
 pub fn primary_button(ui: &mut Ui, text: &str, look: &Look, palette: &Palette) -> Response {
     // Last frame's state, read as egui's Button does, picks this frame's fill.
     let state = ui.ctx().read_response(ui.next_auto_id());
@@ -721,18 +716,7 @@ pub fn primary_button(ui: &mut Ui, text: &str, look: &Look, palette: &Palette) -
         // The fill edges itself, not the grey border of other controls.
         button = button.stroke(Stroke::new(1.0, fill));
     }
-    let response = ui.add(button);
-    if response.has_focus() {
-        // One point out, the corners stay concentric (square on Omarchy).
-        let corner = if look.radius == 0 { 0 } else { look.radius + 1 };
-        ui.painter().rect_stroke(
-            response.rect.expand(1.0),
-            CornerRadius::same(corner),
-            primary_focus_ring(palette),
-            StrokeKind::Outside,
-        );
-    }
-    response
+    ui.add(button)
 }
 
 /// A dialog: a soft shadow over a dimmed window, or Omarchy's accent border
@@ -963,6 +947,14 @@ pub fn filter_field(
             .layouter(&mut layouter),
     );
     let focused = response.has_focus();
+    // The box is the field, not the line of text inside it. A bare line
+    // has its caret and its accent mark.
+    let ring = if style.boxed {
+        Ring::Field { radius: corner.nw }
+    } else {
+        Ring::Own
+    };
+    focus::hint(ui, &response, rect, ring);
     let painter = ui.painter();
     if let Some(fill) = style.fill {
         painter.set(
@@ -1075,6 +1067,16 @@ pub fn segmented(
         if response.clicked() {
             clicked = Some(index);
         }
+        // On the segment's own edge: a ring further out would leave the
+        // track.
+        let ring = if look.terminal {
+            Ring::Inset { radius: 0 }
+        } else {
+            Ring::Edge {
+                radius: look.radius.saturating_sub(4),
+            }
+        };
+        focus::hint(ui, &response, cell, ring);
         let active = index == selected;
         if look.terminal && index > 0 {
             vline(ui, cell.left(), cell.y_range(), palette.outline);
@@ -1264,14 +1266,8 @@ impl<'a> ButtonSpec<'a> {
         let name = self.label.unwrap_or(self.text);
         let response = ui.interact(rect, self.id(ui), Sense::click());
         response.widget_info(|| WidgetInfo::labeled(WidgetType::Button, true, name));
-        if response.has_focus() {
-            ui.painter().rect_stroke(
-                rect,
-                CornerRadius::ZERO,
-                primary_focus_ring(&Palette::light()),
-                StrokeKind::Outside,
-            );
-        }
+        // Not drawn, so the ring is all that shows where the keyboard is.
+        focus::hint(ui, &response, rect, Ring::Outer { radius: 0 });
         response
     }
 
@@ -1421,19 +1417,25 @@ impl<'a> ButtonSpec<'a> {
         } else {
             (fill, border, text)
         };
+        // With the keyboard on it the terminal's button is reversed, as a
+        // terminal marks its cursor; the other looks ring it.
+        let reversed = look.terminal && focus::shown(&response);
+        let (fill, border, text, shortcut) = if reversed {
+            (palette.accent, None, palette.window, palette.window)
+        } else {
+            (fill, border, text, shortcut)
+        };
+        let ring = if reversed {
+            Ring::Own
+        } else {
+            Ring::Outer { radius: corner.nw }
+        };
+        focus::hint(ui, &response, rect, ring);
         let painter = ui.painter();
         painter.rect_filled(rect, corner, fill);
         if let Some(border) = border {
             painter.rect_stroke(rect, corner, border, StrokeKind::Inside);
         }
-        if response.has_focus() {
-            painter.rect_stroke(
-                rect.expand(1.0),
-                corner,
-                primary_focus_ring(palette),
-                StrokeKind::Outside,
-            );
-        }
         let content = self.width(ui, look) - 2.0 * self.padding;
         let mut x = if self.justified {
             rect.left() + self.padding
````

- [ ] **Step 4: Run the tests**

Run: `~/.cargo/bin/cargo test --locked --lib ui::`
Expected: PASS.

Then all of the library's: `~/.cargo/bin/cargo test --locked --lib`
Expected: `test result: ok. 873 passed; 0 failed; 1 ignored`.

- [ ] **Step 5: Add the scenes**

````diff
diff --git a/src/shots.rs b/src/shots.rs
--- a/src/shots.rs
+++ b/src/shots.rs
@@ -501,6 +501,16 @@ fn shots() {
                 }),
             }));
     });
+    // Where the keyboard is, a few Tab stops in: the ring each control
+    // takes (a button, a chip, the search field, a row, a tab, a segment).
+    for stop in [1, 2, 4, 6, 9, 12, 16, 20] {
+        both(&format!("focus-{stop:02}"), |harness| {
+            workspace(harness);
+            for _ in 0..stop {
+                harness.press(egui::Key::Tab, egui::Modifiers::NONE);
+            }
+        });
+    }
     both("structure", |harness| {
         let tab = workspace(harness);
         let object_tab = harness.app.workspace(tab).unwrap().active_tab.unwrap();
````

- [ ] **Step 6: Render, and look**

Run: `~/.cargo/bin/cargo test --locked --features shots --lib shots -- --ignored`

`target/shots/focus-02-macos-light.png` (the Connections button: the outer ring and its gap), `focus-04-macos-dark.png` (the chip: the gap shows the bar's tint), `focus-06-macos-light.png` (a sidebar row: the ring inside it), `focus-09-macos-light.png` (an icon button), and the same numbers for `omarchy` and `standard`. Beside the artboard "macOS – Keyboard focus".

Put right what differs, in the code this task wrote; keep the tests.

- [ ] **Step 7: Format and lint**

```bash
~/.cargo/bin/cargo fmt --all --check
~/.cargo/bin/cargo clippy --locked --workspace --all-targets -- -D warnings
~/.cargo/bin/cargo clippy --locked --workspace --all-targets --features shots -- -D warnings
```

Expected: no output.

- [ ] **Step 8: Commit point**

Stop and report. If the user has asked for commits:

```bash
git add -A src AGENTS.md
git commit -S -m "Give each control the focus ring of its kind" -m "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

### Task 11: The tree and the grid are one Tab stop each

**Files:**
- Modify: `src/ui/grid.rs`
- Modify: `src/ui/sidebar.rs`
- Modify: `src/ui/keys.rs`
- Modify: `src/ui/data_view.rs`, `sql_results.rs`
- Modify: `src/ui/widgets.rs` (`segmented`)
- Modify: `src/model.rs`, `src/app.rs` (`Action::GridKeys`)
- Modify: `src/ui/mod.rs` (tests)
- Modify: `src/shots.rs`

The tree and every grid become a pane: one focusable widget over their area ("Objects", "Rows") whose keys the shortcuts in `keys.rs` handle, with rows that take clicks but not the Tab key. With the keyboard on a pane, or nowhere, the arrows, Enter and Space act on what the pane shows; with it on a button or a field they are that control's (`focus::on_control`). The grid lights its selected cell while the arrows are its own and the keyboard is in use: the selection's colour and an inset ring on the cell, a lighter tint on its row; in the terminal look the cell is reversed. Coming to a grid by Tab selects nothing: a new `Action::GridKeys` hands it the arrows. The two switches (tree/flat, Data/Structure) become one stop each, with ← and → inside.

- [ ] **Step 1: Write the tests**

New in `src/ui/mod.rs`: `the_tab_key_gives_the_tree_and_the_grid_the_arrows`, `arrows_leave_the_grid_alone_while_a_button_has_the_keyboard`, `a_switch_is_one_stop_and_the_arrows_choose_in_it`. New in `src/ui/grid.rs`: `the_keyboard_lights_its_cell_and_the_pointer_only_the_row`. Rewritten for the one stop: `tab_reaches_the_main_controls` (expects "Objects" and "Rows" where it expected a row), `opening_the_row_panel_keeps_the_keyboard_on_the_result_row` (now `..._on_the_results_rows`), `tab_passes_a_results_column_headers_by`.

````diff
diff --git a/src/ui/grid.rs b/src/ui/grid.rs
--- a/src/ui/grid.rs
+++ b/src/ui/grid.rs
@@ -1259,6 +1333,7 @@ mod tests {
                     rows,
                     0,
                     None,
+                    false,
                     &palette,
                     &crate::theme::Look::standard(),
                     |row, col| Cell {
@@ -1286,6 +1361,91 @@ mod tests {
         })
     }
 
+    /// What a frame of a three-row grid paints with cell (1, 1) selected:
+    /// its filled rectangles and its outlines.
+    fn painted(
+        ctx: &egui::Context,
+        look: &Look,
+        palette: &Palette,
+        keys: bool,
+        events: Vec<egui::Event>,
+    ) -> Vec<egui::epaint::RectShape> {
+        let input = egui::RawInput {
+            screen_rect: Some(egui::Rect::from_min_size(
+                egui::Pos2::ZERO,
+                egui::vec2(800.0, 400.0),
+            )),
+            events,
+            ..Default::default()
+        };
+        let mut output = ctx.run_ui(input, |ui| {
+            focus::begin_frame(ui.ctx());
+            show(
+                ui,
+                egui::Id::new("grid"),
+                &columns(),
+                3,
+                0,
+                Some(CellPos { row: 1, col: 1 }),
+                keys,
+                palette,
+                look,
+                |row, col| Cell {
+                    text: format!("r{row}c{col}").into(),
+                    null: false,
+                    style: Style::Plain,
+                },
+            );
+        });
+        output.textures_delta.clear();
+        output
+            .shapes
+            .into_iter()
+            .filter_map(|clipped| match clipped.shape {
+                egui::Shape::Rect(rect) => Some(rect),
+                _ => None,
+            })
+            .collect()
+    }
+
+    #[test]
+    fn the_keyboard_lights_its_cell_and_the_pointer_only_the_row() {
+        for look in Look::ALL {
+            let palette = Palette::light();
+            let ctx = egui::Context::default();
+            crate::theme::install(&ctx, false, &look);
+            crate::theme::apply(&ctx, &palette, &look);
+            let key = crate::testing::key(egui::Key::ArrowDown, egui::Modifiers::NONE);
+            // The cell's mark: the selection's colour on the cell alone,
+            // or the terminal's accent block under its text.
+            let lit = |shapes: &[egui::epaint::RectShape]| {
+                shapes.iter().any(|rect| {
+                    let wide = rect.rect.width();
+                    let cell = rect.fill == palette.selection && wide > 20.0 && wide < 700.0;
+                    let block = rect.fill == palette.accent && wide > 20.0;
+                    if look.terminal { block } else { cell }
+                })
+            };
+            let whole_row = |shapes: &[egui::epaint::RectShape]| {
+                shapes
+                    .iter()
+                    .any(|rect| rect.fill == palette.selection && rect.rect.width() >= 700.0)
+            };
+            // No key yet: the row is selected, the cell is not lit.
+            let shapes = painted(&ctx, &look, &palette, true, Vec::new());
+            assert!(!lit(&shapes) && whole_row(&shapes), "{}", look.name);
+            // A key, and the arrows are the grid's: the cell is lit.
+            painted(&ctx, &look, &palette, true, vec![key.clone()]);
+            let shapes = painted(&ctx, &look, &palette, true, Vec::new());
+            assert!(lit(&shapes), "{}", look.name);
+            // The desktop looks move the selection's colour to the cell.
+            assert_eq!(whole_row(&shapes), look.terminal, "{}", look.name);
+            // The arrows are the tree's: the grid shows its row alone.
+            let shapes = painted(&ctx, &look, &palette, false, Vec::new());
+            assert!(!lit(&shapes) && whole_row(&shapes), "{}", look.name);
+        }
+    }
+
     #[test]
     fn columns_are_found_by_x() {
         let widths = [100.0, 50.0, 80.0];
diff --git a/src/ui/mod.rs b/src/ui/mod.rs
--- a/src/ui/mod.rs
+++ b/src/ui/mod.rs
@@ -3535,16 +3535,21 @@ mod tests {
     }
 
     #[test]
-    fn opening_the_row_panel_keeps_the_keyboard_on_the_result_row() {
+    fn opening_the_row_panel_keeps_the_keyboard_on_the_results_rows() {
         for look in crate::theme::Look::ALL {
             let mut harness = Harness::new();
             harness.set_look(look);
             let tab = harness.connect_fake();
-            with_sql_result(&mut harness, tab, 3);
-            focus(&mut harness, "Row 2", egui::accesskit::Role::Button);
-            harness.press(Key::Enter, Modifiers::NONE);
+            let id = with_sql_result(&mut harness, tab, 3);
+            // The keyboard comes to the rows and selects nothing; an arrow
+            // picks a row, which opens the panel.
+            focus(&mut harness, "Rows", egui::accesskit::Role::Group);
+            assert!(!panel_shows(&mut harness), "{}", look.name);
+            harness.press(Key::ArrowDown, Modifiers::NONE);
+            let workspace = harness.app.workspace(tab).unwrap();
+            assert!(workspace.sql_tab(id).unwrap().selection.is_some());
             assert!(panel_shows(&mut harness), "{}", look.name);
-            assert_eq!(focused_name(&harness.settle()), "Row 2", "{}", look.name);
+            assert_eq!(focused_name(&harness.settle()), "Rows", "{}", look.name);
         }
     }
 
@@ -8278,7 +8283,15 @@ mod tests {
             harness.press(Key::Tab, Modifiers::NONE);
             reached.insert(focused_name(&harness.settle()));
         }
-        for expected in ["Filter", "orders", "Count", "Next page", "Refresh objects"] {
+        // The tree and the grid are one stop each: the arrows move in them.
+        for expected in [
+            "Filter",
+            "Objects",
+            "Rows",
+            "Count",
+            "Next page",
+            "Refresh objects",
+        ] {
             assert!(
                 reached.contains(expected),
                 "{expected} not reached: {reached:?}"
@@ -8319,6 +8332,94 @@ mod tests {
         }
     }
 
+    #[test]
+    fn the_tab_key_gives_the_tree_and_the_grid_the_arrows() {
+        use crate::model::Pane;
+        for look in crate::theme::Look::ALL {
+            let (mut harness, tab) = tree_harness();
+            harness.set_look(look);
+            harness.click("orders");
+            harness.answer_rows(crate::testing::page(3, true));
+            let tab_to = |harness: &mut Harness, name: &str| {
+                for _ in 0..60 {
+                    harness.press(Key::Tab, Modifiers::NONE);
+                    if focused_name(&harness.settle()) == name {
+                        return;
+                    }
+                }
+                panic!("{name} is no Tab stop in {}", look.name);
+            };
+            // Onto the tree: its cursor is on the open table, and moves.
+            tab_to(&mut harness, "Objects");
+            let workspace = harness.app.workspace(tab).unwrap();
+            assert_eq!(workspace.pane, Pane::Tree, "{}", look.name);
+            let before = workspace.tree.cursor.clone();
+            assert!(before.is_some(), "{}", look.name);
+            harness.press(Key::ArrowDown, Modifiers::NONE);
+            let workspace = harness.app.workspace(tab).unwrap();
+            assert_ne!(workspace.tree.cursor, before, "{}", look.name);
+            assert_eq!(focused_name(&harness.settle()), "Objects", "{}", look.name);
+            // Onto the grid: its first cell, and the arrows are its own.
+            tab_to(&mut harness, "Rows");
+            let selection = |harness: &Harness| {
+                let workspace = harness.app.workspace(tab).unwrap();
+                workspace.active_object_tab().unwrap().selection
+            };
+            assert_eq!(harness.app.workspace(tab).unwrap().pane, Pane::Grid);
+            assert_eq!(selection(&harness), None, "coming to it selects nothing");
+            harness.press(Key::ArrowDown, Modifiers::NONE);
+            let first = selection(&harness);
+            assert!(first.is_some(), "{}", look.name);
+            harness.press(Key::ArrowDown, Modifiers::NONE);
+            assert_ne!(selection(&harness), first, "{}", look.name);
+            assert_eq!(focused_name(&harness.settle()), "Rows", "{}", look.name);
+        }
+    }
+
+    #[test]
+    fn arrows_leave_the_grid_alone_while_a_button_has_the_keyboard() {
+        let (mut harness, tab) = tree_harness();
+        harness.click("orders");
+        harness.answer_rows(crate::testing::page(3, true));
+        harness.click("Row 1");
+        let selection = |harness: &Harness| {
+            let workspace = harness.app.workspace(tab).unwrap();
+            workspace.active_object_tab().unwrap().selection
+        };
+        let first = selection(&harness);
+        focus(&mut harness, "Add filter", egui::accesskit::Role::Button);
+        harness.press(Key::ArrowDown, Modifiers::NONE);
+        assert_eq!(selection(&harness), first, "the button had the keys");
+    }
+
+    #[test]
+    fn a_switch_is_one_stop_and_the_arrows_choose_in_it() {
+        let mut harness = Harness::new();
+        harness.set_look(crate::theme::Look::macos());
+        let tab = with_page(&mut harness);
+        let view = |harness: &Harness| {
+            let workspace = harness.app.workspace(tab).unwrap();
+            workspace.active_object_tab().unwrap().view
+        };
+        focus(&mut harness, "Data", egui::accesskit::Role::Button);
+        harness.press(Key::ArrowRight, Modifiers::NONE);
+        assert_eq!(view(&harness), crate::model::ObjectView::Structure);
+        assert_eq!(focused_name(&harness.settle()), "Structure");
+        harness.press(Key::ArrowLeft, Modifiers::NONE);
+        assert_eq!(view(&harness), crate::model::ObjectView::Data);
+        assert_eq!(focused_name(&harness.settle()), "Data");
+        // Tab passes the choice not made.
+        let mut stops = std::collections::HashSet::new();
+        for _ in 0..60 {
+            harness.press(Key::Tab, Modifiers::NONE);
+            stops.insert(focused_name(&harness.settle()));
+        }
+        assert!(
+            stops.contains("Data") && !stops.contains("Structure"),
+            "{stops:?}"
+        );
+    }
+
     #[test]
     fn the_pointer_takes_the_ring_away() {
         let (mut harness, _tab) = tree_harness();
diff --git a/src/ui/sql_results.rs b/src/ui/sql_results.rs
--- a/src/ui/sql_results.rs
+++ b/src/ui/sql_results.rs
@@ -2115,8 +2124,9 @@ mod tests {
                 harness.press(Key::Tab, Modifiers::NONE);
                 stops.push(focused(&mut harness));
             }
-            // It goes on to Messages and the rows, not the headers.
-            for stop in ["Results", "Messages", "Row 1", "Row 3"] {
+            // It goes on to Messages and the rows (one stop for all of
+            // them), not the headers.
+            for stop in ["Results", "Messages", "Rows"] {
                 assert!(stops.iter().any(|name| name == stop), "{stop}: {stops:?}");
             }
             for header in ["id", "email", "meta"] {
````

- [ ] **Step 2: Run them and see them fail**

Run: `~/.cargo/bin/cargo test --locked --lib`

Expected, before the code below is there:

````text
Does not compile:
error[E0433]: cannot find module or crate `focus` in this scope
error[E0277]: expected an `FnMut(usize, usize)` closure, found `&theme::Look`
error[E0277]: expected an `FnOnce(usize, usize)` closure, found `theme::Look`
error[E0061]: this function takes 9 arguments but 10 arguments were supplied
````

- [ ] **Step 3: Write the code**

````diff
diff --git a/src/app.rs b/src/app.rs
--- a/src/app.rs
+++ b/src/app.rs
@@ -657,6 +657,11 @@ impl App {
                     self.fetch_rows(tab, object_tab);
                 }
             }
+            Action::GridKeys(tab) => {
+                if let Some(workspace) = self.workspace_mut(tab) {
+                    workspace.pane = Pane::Grid;
+                }
+            }
             Action::SelectCell { tab, id, cell } => {
                 if let Some(object) = self.object_tab_mut(tab, id) {
                     object.selection = Some(cell);
diff --git a/src/model.rs b/src/model.rs
--- a/src/model.rs
+++ b/src/model.rs
@@ -205,6 +205,9 @@ pub enum Action {
         object_tab: TabId,
         column: String,
     },
+    /// The keyboard came to a grid (the Tab key, a screen reader): the
+    /// arrows move in it from now on. Selects nothing.
+    GridKeys(ConnTabId),
     /// Select a cell of an object tab's page or a SQL editor's result.
     SelectCell {
         tab: ConnTabId,
diff --git a/src/ui/data_view.rs b/src/ui/data_view.rs
--- a/src/ui/data_view.rs
+++ b/src/ui/data_view.rs
@@ -13,6 +13,7 @@ use crate::i18n::gettext;
 use crate::model::{Action, ConnTabId, ObjectTab, ObjectView, TabId};
 use crate::theme::{Icon, Look, Palette};
 use crate::typography::{Text, TextRole};
+use crate::ui::focus;
 use crate::ui::format;
 use crate::ui::grid::{self, Cell, Column, Style};
 use crate::ui::states;
@@ -259,15 +260,31 @@ pub fn header(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId, object_tab: TabI
             ui.painter()
                 .rect_filled(track, CornerRadius::same(look.radius), palette.surface);
             x += 3.0;
-            for ((target, label, _), width) in views.iter().zip(widths) {
+            for (index, ((target, label, _), width)) in views.iter().zip(widths).enumerate() {
                 let cell = Rect::from_min_size(pos2(x, track.top() + 3.0), vec2(width, 28.0));
                 x += width;
-                let response =
-                    ui.interact(cell, ui.id().with(("view", label.as_ref())), Sense::click());
                 let selected = view == *target;
+                // One Tab stop for the switch; the arrows choose inside it.
+                let (response, arrow) = focus::segment(
+                    ui,
+                    cell,
+                    ui.id().with(("view", label.as_ref())),
+                    ui.id().with("views"),
+                    (index, views.len()),
+                    selected,
+                );
                 response.widget_info(|| {
                     WidgetInfo::selected(WidgetType::Button, true, selected, label.as_ref())
                 });
+                let radius = look.radius.saturating_sub(2);
+                focus::hint(ui, &response, cell, focus::Ring::Edge { radius });
+                if let Some((chosen, _, _)) = arrow.and_then(|index| views.get(index)) {
+                    actions.push(Action::SetView {
+                        tab,
+                        object_tab,
+                        view: *chosen,
+                    });
+                }
                 if selected {
                     let corner = CornerRadius::same(look.radius.saturating_sub(2));
                     ui.painter().add(
@@ -1002,6 +1019,8 @@ pub fn show(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId, object_tab: TabId)
             page.rows.len(),
             object.query.offset,
             object.selection,
+            // The arrows are the grid's once the user worked in it.
+            workspace.pane == crate::model::Pane::Grid,
             &palette,
             &look,
             |row, col| {
@@ -1022,6 +1041,10 @@ pub fn show(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId, object_tab: TabId)
                 cell,
             });
         }
+        // The Tab key came to the grid: the arrows are its own now.
+        if output.focused {
+            actions.push(Action::GridKeys(tab));
+        }
         if let Some(col) = output.sort_clicked {
             actions.push(Action::SortBy {
                 tab,
diff --git a/src/ui/grid.rs b/src/ui/grid.rs
--- a/src/ui/grid.rs
+++ b/src/ui/grid.rs
@@ -12,6 +12,7 @@ use tabletist_db::SortDir;
 use crate::model::CellPos;
 use crate::theme::{DataFont, Icon, Look, Palette};
 use crate::typography::{Text, TextRole};
+use crate::ui::focus;
 use crate::ui::format::{Marks, array_items, display_safe};
 use crate::ui::widgets::virtual_rows;
 
@@ -90,6 +91,9 @@ pub struct Cell<'a> {
 pub struct GridOutput {
     pub clicked: Option<CellPos>,
     pub sort_clicked: Option<usize>,
+    /// The keyboard came to the grid this frame (the Tab key, a screen
+    /// reader): the arrows should be the grid's.
+    pub focused: bool,
 }
 
 /// How wide `text` is in `role`, in points (laid out once, then cached).
@@ -361,6 +365,8 @@ pub fn show<'a>(
     row_count: usize,
     first_row_number: u64,
     selection: Option<CellPos>,
+    // Whether the arrow keys move in this grid.
+    keys: bool,
     palette: &Palette,
     look: &crate::theme::Look,
     mut cell: impl FnMut(usize, usize) -> Cell<'a>,
@@ -407,6 +413,26 @@ pub fn show<'a>(
     let total = gutter + widths.iter().sum::<f32>();
     let hairline = crate::ui::widgets::hairline(ui);
     let visible = ui.max_rect();
+    // The grid is one Tab stop, not one for each row: with the keyboard on
+    // it the arrows move the selected cell, which shows where they are.
+    let stop = ui.interact(visible, id.with("keys"), Sense::focusable_noninteractive());
+    stop.widget_info(|| WidgetInfo::labeled(WidgetType::Other, true, "Rows"));
+    ui.ctx().accesskit_node_builder(stop.id, |node| {
+        node.set_role(egui::accesskit::Role::Group);
+    });
+    focus::pane(ui, &stop);
+    // The selected cell shows where the keyboard is; with none selected
+    // yet the grid itself does, until an arrow picks one.
+    let ring = if selection.is_some() {
+        focus::Ring::Own
+    } else {
+        focus::Ring::Inset { radius: 0 }
+    };
+    focus::hint(ui, &stop, visible, ring);
+    output.focused = stop.gained_focus();
+    // The cell is lit while the keyboard is in use and its keys come here:
+    // not while a button or a field has them.
+    let lit = keys && focus::visible(ui.ctx()) && !focus::on_control(ui.ctx());
 
     let scroll = egui::ScrollArea::both()
         .id_salt(id)
@@ -434,8 +460,8 @@ pub fn show<'a>(
             }
 
             virtual_rows(ui, row_count, row_height, |ui, row| {
-                let (rect, response) =
-                    ui.allocate_exact_size(vec2(full, row_height), Sense::click());
+                // A row takes a click, not the Tab key: the grid is the stop.
+                let (rect, response) = ui.allocate_exact_size(vec2(full, row_height), Sense::CLICK);
                 let number = first_row_number + row as u64 + 1;
                 let label = format!("Row {number}");
                 response.widget_info(|| WidgetInfo::labeled(WidgetType::Button, true, &label));
@@ -448,13 +474,21 @@ pub fn show<'a>(
                 }
                 let painter = ui.painter().clone();
                 let selected_row = selection.is_some_and(|cell| cell.row == row);
-                if let Some(fill) = row_fill(
-                    selected_row,
-                    response.hovered(),
-                    row % 2 == 1,
-                    look,
-                    palette,
-                ) {
+                // With the keyboard in the grid the cell takes the
+                // selection's colour and its row a lighter tint of it.
+                let lit_row = selected_row && lit && !look.terminal;
+                let fill = if lit_row {
+                    Some(palette.window.lerp_to_gamma(palette.selection, 0.6))
+                } else {
+                    row_fill(
+                        selected_row,
+                        response.hovered(),
+                        row % 2 == 1,
+                        look,
+                        palette,
+                    )
+                };
+                if let Some(fill) = fill {
                     painter.rect_filled(rect, CornerRadius::ZERO, fill);
                 }
                 if !look.terminal {
@@ -470,7 +504,7 @@ pub fn show<'a>(
                             &painter,
                             pos2(rect.left() + GUTTER / 2.0, rect.center().y),
                         );
-                    } else {
+                    } else if !lit_row {
                         let bar = Rect::from_min_size(rect.min, vec2(3.0, rect.height()));
                         painter.rect_filled(bar, CornerRadius::ZERO, palette.accent);
                     }
@@ -484,6 +518,36 @@ pub fn show<'a>(
                         continue;
                     }
                     let content = cell(row, col);
+                    let here = selection == Some(CellPos { row, col });
+                    if here && lit && look.terminal {
+                        // Reverse video, as a terminal marks its cursor:
+                        // the accent behind, the text in the window's tone.
+                        painter.rect_filled(cell_rect, CornerRadius::ZERO, palette.accent);
+                        let reversed = Palette {
+                            text: palette.window,
+                            secondary: palette.window,
+                            dim: palette.window,
+                            faint: palette.window,
+                            ..*palette
+                        };
+                        let plain = Cell {
+                            style: Style::Plain,
+                            ..content
+                        };
+                        draw_cell(
+                            ui,
+                            &painter,
+                            cell_rect,
+                            &columns[col],
+                            &plain,
+                            look,
+                            &reversed,
+                        );
+                        continue;
+                    }
+                    if here && lit {
+                        painter.rect_filled(cell_rect, CornerRadius::ZERO, palette.selection);
+                    }
                     draw_cell(
                         ui,
                         &painter,
@@ -493,9 +557,18 @@ pub fn show<'a>(
                         look,
                         palette,
                     );
-                    // The row is selected; a cell past the first is marked
-                    // too, for the keys that act on one cell.
-                    if selection == Some(CellPos { row, col }) && col > 0 {
+                    if here && lit {
+                        // Inside the cell: nothing the grid scrolls under
+                        // cuts it.
+                        painter.rect_stroke(
+                            cell_rect,
+                            CornerRadius::ZERO,
+                            Stroke::new(2.0, palette.accent),
+                            StrokeKind::Inside,
+                        );
+                    } else if here && col > 0 {
+                        // The row is selected; a cell past the first is
+                        // marked too, for the keys that act on one cell.
                         painter.rect_stroke(
                             cell_rect.shrink(1.0),
                             CornerRadius::same(look.radius.min(3)),
@@ -532,6 +605,7 @@ pub fn show<'a>(
                     (Sense::CLICK, WidgetType::Label)
                 };
                 let response = ui.interact(rect, id.with(("header", col)), sense);
+                focus::hint(ui, &response, rect, focus::Ring::Inset { radius: 0 });
                 // Column names come from the server: nothing hidden in them.
                 let name = display_safe(column.name);
                 response.widget_info(|| WidgetInfo::labeled(kind, true, &*name));
diff --git a/src/ui/keys.rs b/src/ui/keys.rs
--- a/src/ui/keys.rs
+++ b/src/ui/keys.rs
@@ -96,8 +96,10 @@ pub fn handle(app: &mut App, ctx: &egui::Context) {
             .and_then(|workspace| workspace.sql_tab(id))
             .is_some_and(|sql| sql.selected_row().is_some())
     });
-    // Space activates a focused button; it only toggles the panel otherwise.
-    let focused = ctx.memory(|memory| memory.focused().is_some());
+    // Space and Enter press a focused button, and the arrows move focus
+    // from it; with the keyboard on the tree, on a grid or nowhere they act
+    // on what those show.
+    let focused = crate::ui::focus::on_control(ctx);
     let filter_open = object.is_some_and(|(tab, id)| {
         app.workspace(tab)
             .and_then(|workspace| workspace.object_tab(id))
@@ -198,7 +200,7 @@ pub fn handle(app: &mut App, ctx: &egui::Context) {
         }
         key(Modifiers::COMMAND, Key::P, Action::OpenQuickOpen);
         key(Modifiers::COMMAND, Key::B, Action::ToggleSidebar(active));
-        if tree_arrows {
+        if tree_arrows && !focused {
             use crate::model::TreeKey;
             for (pressed, tree_key) in [
                 (Key::ArrowUp, TreeKey::Up),
@@ -217,17 +219,14 @@ pub fn handle(app: &mut App, ctx: &egui::Context) {
                     },
                 );
             }
-            // Enter belongs to a focused button.
-            if !focused {
-                key(
-                    Modifiers::NONE,
-                    Key::Enter,
-                    Action::TreeKey {
-                        tab: active,
-                        key: TreeKey::Enter,
-                    },
-                );
-            }
+            key(
+                Modifiers::NONE,
+                Key::Enter,
+                Action::TreeKey {
+                    tab: active,
+                    key: TreeKey::Enter,
+                },
+            );
         }
         // Paging is a table's: a SQL result has one page.
         if let Some((tab, object_tab)) = object {
@@ -258,7 +257,7 @@ pub fn handle(app: &mut App, ctx: &egui::Context) {
                 );
             }
             key(Modifiers::COMMAND, Key::W, Action::CloseTab { tab, id });
-            if !editing && any_grid && !tree_arrows {
+            if !editing && any_grid && !tree_arrows && !focused {
                 let page = 20;
                 for (pressed, rows, cols) in [
                     (Key::ArrowUp, -1, 0),
@@ -283,7 +282,7 @@ pub fn handle(app: &mut App, ctx: &egui::Context) {
                 }
                 // The row panel shows a table's row, or the selected row
                 // of a SQL result.
-                if (grid || sql_row) && !focused {
+                if grid || sql_row {
                     key(Modifiers::NONE, Key::Space, Action::ToggleRowPanel(tab));
                 }
             }
@@ -409,7 +408,7 @@ fn letters(app: &mut App, ctx: &egui::Context, actions: &mut Vec<Action>) {
             actions.push(Action::MovePickerSelection { tab, step: -1 });
         }
         if let Some(conn) = selected {
-            if ctx.memory(|memory| memory.focused().is_none()) {
+            if !crate::ui::focus::on_control(ctx) {
                 // Shift first: egui ignores an extra Shift when matching.
                 let again = ctx.input_mut(|input| input.consume_key(Modifiers::SHIFT, Key::Enter));
                 if again || pressed(Key::Enter) {
@@ -538,7 +537,7 @@ fn letters(app: &mut App, ctx: &egui::Context, actions: &mut Vec<Action>) {
     // of it is selected (with none its panel has nothing to show).
     if let Some(id) = active.or(sql_row) {
         // Enter belongs to a focused button.
-        let focused = ctx.memory(|memory| memory.focused().is_some());
+        let focused = crate::ui::focus::on_control(ctx);
         if !tree && !panel && !focused && pressed(Key::Enter) {
             actions.push(Action::ToggleRowPanel(tab));
         }
diff --git a/src/ui/sidebar.rs b/src/ui/sidebar.rs
--- a/src/ui/sidebar.rs
+++ b/src/ui/sidebar.rs
@@ -116,6 +116,15 @@ pub fn show(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId) {
         .cursor
         .clone()
         .filter(|_| workspace.pane == crate::model::Pane::Tree);
+    // Where the cursor starts when the Tab key comes to the tree: where it
+    // was, else on the open object's row.
+    let cursor_start = workspace.tree.cursor.clone().or_else(|| {
+        let open = active.as_ref()?;
+        rows.iter()
+            .map(|row| &row.node)
+            .find(|node| matches!(node, TreeNode::Object(object, _) if object == open))
+            .cloned()
+    });
     // After a key moved the cursor, scroll its row into view.
     let reveal = workspace
         .tree
@@ -270,6 +279,31 @@ pub fn show(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId) {
             let footer = if look.terminal { FOOTER } else { SQL_FOOTER };
             let body = ui.available_height() - footer;
             ui.allocate_ui(vec2(full.width(), body.max(0.0)), |ui| {
+                // The tree is one Tab stop, not one for each row: with the
+                // keyboard on it the arrows move its cursor.
+                let area = Rect::from_min_size(ui.cursor().min, vec2(full.width(), body.max(0.0)));
+                let stop = ui.interact(
+                    area,
+                    Id::new(("tree-keys", tab.0)),
+                    Sense::focusable_noninteractive(),
+                );
+                stop.widget_info(|| {
+                    WidgetInfo::labeled(WidgetType::Other, true, gettext(locale, "Objects"))
+                });
+                ui.ctx().accesskit_node_builder(stop.id, |node| {
+                    node.set_role(egui::accesskit::Role::Group);
+                });
+                focus::pane(ui, &stop);
+                focus::hint(ui, &stop, area, focus::Ring::Own);
+                // The cursor it had, the open object's row, or its first.
+                let start = cursor_start
+                    .clone()
+                    .or_else(|| rows.first().map(|row| row.node.clone()));
+                if stop.gained_focus()
+                    && let Some(node) = start
+                {
+                    actions.push(Action::SetTreeCursor { tab, node });
+                }
                 egui::ScrollArea::vertical()
                     .auto_shrink([false, false])
                     .show(ui, |ui| {
@@ -701,9 +735,10 @@ fn tree_row(
         });
         return;
     }
+    // A row takes a click, not the Tab key: the tree is the stop.
     let (rect, response) = ui.allocate_exact_size(
         vec2(ui.available_width(), row_height(row, look)),
-        Sense::click(),
+        Sense::CLICK,
     );
     // Names come from the server: nothing hidden in them.
     let full_name = match &row.node {
@@ -722,11 +757,10 @@ fn tree_row(
     };
     widgets::selection(ui, band, selected, response.hovered(), look, palette);
     let highlight = widgets::selection_rect(band, look);
-    row_ring(ui, &response, highlight, look);
-    // The row the arrows are on, while the keyboard is in use: the ring a
-    // focused row takes. Apart from the open object's fill: one says where
-    // the keys are, the other what is open.
-    if marks.cursor == Some(&row.node) && focus::visible(ui.ctx()) && !response.has_focus() {
+    // The row the arrows are on, while the keyboard is in use and its keys
+    // come to the tree: the ring a focused row takes. Apart from the open
+    // object's fill: one says where the keys are, the other what is open.
+    if marks.cursor == Some(&row.node) && focus::visible(ui.ctx()) && !focus::on_control(ui.ctx()) {
         ui.painter().rect_stroke(
             highlight,
             CornerRadius::same(look.radius.saturating_sub(2)),
diff --git a/src/ui/sql_results.rs b/src/ui/sql_results.rs
--- a/src/ui/sql_results.rs
+++ b/src/ui/sql_results.rs
@@ -197,6 +197,8 @@ struct Place<'a> {
     full_precision: bool,
     /// Whose error codes the results read.
     driver: tabletist_db::Driver,
+    /// Whether the arrow keys move in the result's grid.
+    keys: bool,
 }
 
 fn draw(app: &App, ui: &mut Ui, tab: ConnTabId, id: TabId, actions: &mut Vec<Action>) {
@@ -217,6 +219,7 @@ fn draw(app: &App, ui: &mut Ui, tab: ConnTabId, id: TabId, actions: &mut Vec<Act
         sql,
         full_precision: workspace.full_precision,
         driver: workspace.driver,
+        keys: workspace.pane == crate::model::Pane::Grid,
     };
     let state = state(sql);
     let pane = ui.max_rect();
@@ -1043,6 +1046,7 @@ fn results(ui: &mut Ui, run: &SqlRun, place: &Place<'_>, env: &Env<'_>, actions:
         tab,
         sql,
         full_precision,
+        keys,
         ..
     } = *place;
     let Some((columns, rows, truncated)) = sql.shown_rows() else {
@@ -1144,6 +1148,7 @@ fn results(ui: &mut Ui, run: &SqlRun, place: &Place<'_>, env: &Env<'_>, actions:
         rows.len(),
         0,
         sql.selection,
+        keys,
         palette,
         look,
         |row, col| {
@@ -1164,6 +1169,10 @@ fn results(ui: &mut Ui, run: &SqlRun, place: &Place<'_>, env: &Env<'_>, actions:
             cell,
         });
     }
+    // The Tab key came to the grid: the arrows are its own now.
+    if output.focused {
+        actions.push(Action::GridKeys(tab));
+    }
     if rows.is_empty() {
         let under = Rect::from_min_max(
             pos2(area.left(), area.top() + grid::header_height(look)),
diff --git a/src/ui/widgets.rs b/src/ui/widgets.rs
--- a/src/ui/widgets.rs
+++ b/src/ui/widgets.rs
@@ -1060,13 +1060,22 @@ pub fn segmented(
             Segment::Icon(icon, name) => (*name, Some(*icon), None),
             Segment::Text(text) => (*text, None, Some(*text)),
         };
-        let response = ui.interact(cell, ui.id().with(("segment", name)), Sense::click());
+        // One Tab stop for the control; the arrows choose inside it.
+        let (response, arrow) = focus::segment(
+            ui,
+            cell,
+            ui.id().with(("segment", name)),
+            ui.id().with("segments"),
+            (index, segments.len()),
+            index == selected,
+        );
         response.widget_info(|| {
             WidgetInfo::selected(WidgetType::Button, true, index == selected, name)
         });
         if response.clicked() {
             clicked = Some(index);
         }
+        clicked = arrow.or(clicked);
         // On the segment's own edge: a ring further out would leave the
         // track.
         let ring = if look.terminal {
````

- [ ] **Step 4: Run the tests**

Run: `~/.cargo/bin/cargo test --locked --lib ui::`
Expected: PASS.

Then all of the library's: `~/.cargo/bin/cargo test --locked --lib`
Expected: `test result: ok. 877 passed; 0 failed; 1 ignored`.

- [ ] **Step 5: Add the scenes**

````diff
diff --git a/src/shots.rs b/src/shots.rs
--- a/src/shots.rs
+++ b/src/shots.rs
@@ -511,6 +511,25 @@ fn shots() {
             }
         });
     }
+    // The arrows in the grid: the cell they are on, lit in its row.
+    both("focus-grid", |harness| {
+        workspace(harness);
+        for _ in 0..2 {
+            harness.press(egui::Key::ArrowRight, egui::Modifiers::NONE);
+        }
+    });
+    // The arrows in the tree: its cursor, apart from the open table's row.
+    both("focus-tree", |harness| {
+        let tab = workspace(harness);
+        harness.app.apply(Action::TreeKey {
+            tab,
+            key: crate::model::TreeKey::Home,
+        });
+        harness.app.workspace_mut(tab).unwrap().pane = crate::model::Pane::Tree;
+        for _ in 0..3 {
+            harness.press(egui::Key::ArrowDown, egui::Modifiers::NONE);
+        }
+    });
     both("structure", |harness| {
         let tab = workspace(harness);
         let object_tab = harness.app.workspace(tab).unwrap().active_tab.unwrap();
````

- [ ] **Step 6: Render, and look**

Run: `~/.cargo/bin/cargo test --locked --features shots --lib shots -- --ignored`

`target/shots/focus-grid-macos-light.png` beside the artboard's grid: the row's lighter tint, the cell in the selection's colour with the ring inside it. `focus-tree-macos-light.png`: the ring on the tree's cursor, apart from the open table's fill. `focus-grid-omarchy-dark.png`: the reversed cell.

Put right what differs, in the code this task wrote; keep the tests.

- [ ] **Step 7: Format and lint**

```bash
~/.cargo/bin/cargo fmt --all --check
~/.cargo/bin/cargo clippy --locked --workspace --all-targets -- -D warnings
~/.cargo/bin/cargo clippy --locked --workspace --all-targets --features shots -- -D warnings
```

Expected: no output.

- [ ] **Step 8: Commit point**

Stop and report. If the user has asked for commits:

```bash
git add -A src AGENTS.md
git commit -S -m "Make the tree and each grid one stop for the Tab key" -m "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

### Task 12: The terminal look marks the pane with the keys

**Files:**
- Modify: `src/ui/sidebar.rs`
- Modify: `src/ui/grid.rs`
- Modify: `src/ui/mod.rs` (tests)

Omarchy's pane 6: the pane the keys go to has a 2 pt accent border while the keyboard is in use, and the open table's bar in the tree is muted while the arrows are the grid's ("selected, not focused").

- [ ] **Step 1: Write the tests**

New in `src/ui/mod.rs`: `the_terminal_marks_the_pane_the_keys_go_to` (only the terminal look, and only once a key was pressed) and `the_terminals_open_table_is_marked_quietly_while_the_keys_are_elsewhere`.

````diff
diff --git a/src/ui/mod.rs b/src/ui/mod.rs
--- a/src/ui/mod.rs
+++ b/src/ui/mod.rs
@@ -8420,6 +8420,54 @@ mod tests {
         );
     }
 
+    #[test]
+    fn the_terminal_marks_the_pane_the_keys_go_to() {
+        // An accent line round something as tall as a pane.
+        let marked = |harness: &Harness| {
+            let accent = harness.app.palette.accent;
+            harness
+                .outlines
+                .iter()
+                .any(|(rect, stroke)| stroke.color == accent && rect.height() > 200.0)
+        };
+        for look in crate::theme::Look::ALL {
+            let (mut harness, _tab) = tree_harness();
+            harness.set_look(look);
+            harness.click("orders");
+            harness.answer_rows(crate::testing::page(3, true));
+            // The pointer alone marks nothing.
+            harness.settle();
+            assert!(!marked(&harness), "{}", look.name);
+            // A key: the tree has the arrows, where opening left them.
+            harness.press(Key::ArrowDown, Modifiers::NONE);
+            harness.settle();
+            assert_eq!(marked(&harness), look.terminal, "{}", look.name);
+        }
+    }
+
+    #[test]
+    fn the_terminals_open_table_is_marked_quietly_while_the_keys_are_elsewhere() {
+        let (mut harness, _tab) = tree_harness();
+        harness.set_look(crate::theme::Look::omarchy());
+        harness.click("orders");
+        harness.answer_rows(crate::testing::page(3, true));
+        // The bar at the left of the open table's row, in `color`.
+        let bar = |harness: &Harness, color: egui::Color32| {
+            harness
+                .fills
+                .iter()
+                .any(|(rect, fill)| *fill == color && rect.width() < 4.0 && rect.height() > 10.0)
+        };
+        let (accent, muted) = (harness.app.palette.accent, harness.app.palette.dim);
+        // Opening left the arrows with the tree.
+        harness.settle();
+        assert!(bar(&harness, accent) && !bar(&harness, muted));
+        // A click in the grid takes them there.
+        harness.click("Row 1");
+        harness.settle();
+        assert!(bar(&harness, muted));
+    }
+
     #[test]
     fn the_pointer_takes_the_ring_away() {
         let (mut harness, _tab) = tree_harness();
````

- [ ] **Step 2: Run them and see them fail**

Run: `~/.cargo/bin/cargo test --locked --lib`

Expected, before the code below is there:

````text
test ui::tests::the_terminal_marks_the_pane_the_keys_go_to ... FAILED
test ui::tests::the_terminals_open_table_is_marked_quietly_while_the_keys_are_elsewhere ... FAILED
test result: FAILED. 877 passed; 2 failed; 1 ignored; 0 measured; 0 filtered out
````

- [ ] **Step 3: Write the code**

````diff
diff --git a/src/ui/grid.rs b/src/ui/grid.rs
--- a/src/ui/grid.rs
+++ b/src/ui/grid.rs
@@ -731,6 +731,9 @@ pub fn show<'a>(
         }
     }
 
+    if lit {
+        focus::pane_border(ui, visible, look, palette);
+    }
     ui.data_mut(|data| {
         if keep {
             data.insert_temp(widths_id, widths);
diff --git a/src/ui/sidebar.rs b/src/ui/sidebar.rs
--- a/src/ui/sidebar.rs
+++ b/src/ui/sidebar.rs
@@ -136,6 +136,10 @@ pub fn show(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId) {
         .flatten();
     let reveal_pending = workspace.tree.reveal_cursor;
     let mut actions = Vec::new();
+    // Whether the keys come to the tree and the keyboard is in use: then
+    // its cursor shows, and the terminal look marks the pane.
+    let keys = workspace.pane == crate::model::Pane::Tree;
+    let lit = keys && focus::visible(ui.ctx()) && !focus::on_control(ui.ctx());
 
     // The terminal's sidebar takes the theme's dark background too.
     let fill = if look.sidebar_tinted || look.terminal {
@@ -332,6 +336,8 @@ pub fn show(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId) {
                                 Marks {
                                     active: active.as_ref(),
                                     cursor: cursor.as_ref(),
+                                    keys,
+                                    lit,
                                 },
                                 locale,
                                 Skin {
@@ -349,6 +355,9 @@ pub fn show(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId) {
             } else {
                 sql_button(ui, full, tab, locale, &look, &palette, &mut actions);
             }
+            if lit {
+                focus::pane_border(ui, full, &look, &palette);
+            }
         });
     if reveal_pending && let Some(workspace) = app.workspace_mut(tab) {
         workspace.tree.reveal_cursor = false;
@@ -652,6 +661,10 @@ fn schema_header(
 struct Marks<'a> {
     active: Option<&'a ObjectRef>,
     cursor: Option<&'a TreeNode>,
+    /// The arrows are the tree's.
+    keys: bool,
+    /// And the keyboard is in use: the cursor shows.
+    lit: bool,
 }
 
 /// A row with the keyboard is ringed inside its highlight: a ring outside
@@ -757,10 +770,17 @@ fn tree_row(
     };
     widgets::selection(ui, band, selected, response.hovered(), look, palette);
     let highlight = widgets::selection_rect(band, look);
+    // The terminal's bar on the open object's row is muted while the keys
+    // are elsewhere: selected, not focused.
+    if selected && !marks.keys && look.selection == crate::theme::Selection::Bar {
+        let edge = Rect::from_min_size(band.min, vec2(2.0, band.height()));
+        ui.painter()
+            .rect_filled(edge, CornerRadius::ZERO, palette.dim);
+    }
     // The row the arrows are on, while the keyboard is in use and its keys
     // come to the tree: the ring a focused row takes. Apart from the open
     // object's fill: one says where the keys are, the other what is open.
-    if marks.cursor == Some(&row.node) && focus::visible(ui.ctx()) && !focus::on_control(ui.ctx()) {
+    if marks.cursor == Some(&row.node) && marks.lit {
         ui.painter().rect_stroke(
             highlight,
             CornerRadius::same(look.radius.saturating_sub(2)),
````

- [ ] **Step 4: Run the tests**

Run: `~/.cargo/bin/cargo test --locked --lib ui::tests::the_terminal`
Expected: PASS.

Then all of the library's: `~/.cargo/bin/cargo test --locked --lib`
Expected: `test result: ok. 879 passed; 0 failed; 1 ignored`.

- [ ] **Step 5: Render, and look**

Run: `~/.cargo/bin/cargo test --locked --features shots --lib shots -- --ignored`

`target/shots/focus-grid-omarchy-dark.png` and `focus-tree-omarchy-dark.png` beside pane 6 of the Omarchy artboard: the border round the grid or the sidebar, the muted bar on the open table when the grid has the keys.

Put right what differs, in the code this task wrote; keep the tests.

- [ ] **Step 6: Format and lint**

```bash
~/.cargo/bin/cargo fmt --all --check
~/.cargo/bin/cargo clippy --locked --workspace --all-targets -- -D warnings
~/.cargo/bin/cargo clippy --locked --workspace --all-targets --features shots -- -D warnings
```

Expected: no output.

- [ ] **Step 7: Commit point**

Stop and report. If the user has asked for commits:

```bash
git add -A src AGENTS.md
git commit -S -m "Mark the terminal's pane that has the keys" -m "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

### Task 13: F6 steps through the parts of the window

**Files:**
- Modify: `src/ui/keys.rs`
- Modify: `src/ui/workspace.rs`, `sidebar.rs`, `object_tabs.rs`, `data_view.rs`, `grid.rs`, `sql_text.rs`, `row_panel.rs`, `picker.rs`
- Modify: `src/ui/mod.rs` (tests)

Each part of the window says where it is (`focus::region`) and which control it starts with (`focus::claim`); F6 asks for the part after the one the keyboard is in, Shift+F6 for the one before, and the terminal look steps through its panes alone with ctrl+l and ctrl+h. The drag areas (the title bar, the picker's header, the column resize handles) stop being Tab stops: they did nothing with the keyboard, and would now show a ring.

- [ ] **Step 1: Write the tests**

New in `src/ui/mod.rs`: `f6_steps_through_the_parts_of_the_window` (forward from the tree: the tab, the switch, the rows, Connections, the filter, the tree; and back) and `the_terminal_steps_between_its_panes_with_ctrl_h_and_l`.

````diff
diff --git a/src/ui/mod.rs b/src/ui/mod.rs
--- a/src/ui/mod.rs
+++ b/src/ui/mod.rs
@@ -8420,6 +8420,68 @@ mod tests {
         );
     }
 
+    #[test]
+    fn f6_steps_through_the_parts_of_the_window() {
+        for look in crate::theme::Look::ALL {
+            let (mut harness, _tab) = tree_harness();
+            harness.set_look(look);
+            harness.click("orders");
+            harness.answer_rows(crate::testing::page(3, true));
+            let step = |harness: &mut Harness, modifiers| {
+                harness.press(Key::F6, modifiers);
+                focused_name(&harness.settle())
+            };
+            // From the tree, where opening the table left the arrows.
+            let forward: Vec<String> = (0..6)
+                .map(|_| step(&mut harness, Modifiers::NONE))
+                .collect();
+            assert_eq!(
+                forward,
+                [
+                    "orders tab",
+                    "Data",
+                    "Rows",
+                    "Connections",
+                    "Filter",
+                    "Objects"
+                ],
+                "{}",
+                look.name
+            );
+            // And back the way it came.
+            let back: Vec<String> = (0..3)
+                .map(|_| step(&mut harness, Modifiers::SHIFT))
+                .collect();
+            assert_eq!(back, ["Filter", "Connections", "Rows"], "{}", look.name);
+        }
+    }
+
+    #[test]
+    fn the_terminal_steps_between_its_panes_with_ctrl_h_and_l() {
+        let (mut harness, tab) = tree_harness();
+        harness.set_look(crate::theme::Look::omarchy());
+        harness.click("orders");
+        harness.answer_rows(crate::testing::page(3, true));
+        let pane = |harness: &Harness| harness.app.workspace(tab).unwrap().pane;
+        harness.press(Key::L, Modifiers::CTRL);
+        assert_eq!(focused_name(&harness.settle()), "Rows");
+        assert_eq!(pane(&harness), crate::model::Pane::Grid);
+        harness.press(Key::H, Modifiers::CTRL);
+        assert_eq!(focused_name(&harness.settle()), "Objects");
+        assert_eq!(pane(&harness), crate::model::Pane::Tree);
+        // The other looks leave those keys alone.
+        let (mut harness, tab) = tree_harness();
+        harness.set_look(crate::theme::Look::macos());
+        harness.click("orders");
+        harness.answer_rows(crate::testing::page(3, true));
+        harness.press(Key::L, Modifiers::CTRL);
+        assert_eq!(focused_name(&harness.settle()), "");
+        assert_eq!(
+            harness.app.workspace(tab).unwrap().pane,
+            crate::model::Pane::Tree
+        );
+    }
+
     #[test]
     fn the_terminal_marks_the_pane_the_keys_go_to() {
         // An accent line round something as tall as a pane.
````

- [ ] **Step 2: Run them and see them fail**

Run: `~/.cargo/bin/cargo test --locked --lib`

Expected, before the code below is there:

````text
test ui::tests::f6_steps_through_the_parts_of_the_window ... FAILED
test ui::tests::the_terminal_steps_between_its_panes_with_ctrl_h_and_l ... FAILED
test result: FAILED. 879 passed; 2 failed; 1 ignored; 0 measured; 0 filtered out
````

- [ ] **Step 3: Write the code**

````diff
diff --git a/src/ui/data_view.rs b/src/ui/data_view.rs
--- a/src/ui/data_view.rs
+++ b/src/ui/data_view.rs
@@ -126,6 +126,7 @@ pub fn header(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId, object_tab: TabI
         .frame(Frame::new().fill(palette.window))
         .show(ui, |ui| {
             let rect = ui.max_rect();
+            focus::region(ui, focus::Region::Toolbar, rect);
             let divider = if look.terminal {
                 palette.outline
             } else {
@@ -173,6 +174,9 @@ pub fn header(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId, object_tab: TabI
                     );
                     let response = ui.interact(hit, ui.id().with(("view", *key)), Sense::click());
                     let selected = view == *target;
+                    if selected {
+                        focus::claim(ui, focus::Region::Toolbar, &response);
+                    }
                     let name = views
                         .iter()
                         .find(|(v, ..)| v == target)
@@ -278,6 +282,9 @@ pub fn header(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId, object_tab: TabI
                 });
                 let radius = look.radius.saturating_sub(2);
                 focus::hint(ui, &response, cell, focus::Ring::Edge { radius });
+                if selected {
+                    focus::claim(ui, focus::Region::Toolbar, &response);
+                }
                 if let Some((chosen, _, _)) = arrow.and_then(|index| views.get(index)) {
                     actions.push(Action::SetView {
                         tab,
diff --git a/src/ui/grid.rs b/src/ui/grid.rs
--- a/src/ui/grid.rs
+++ b/src/ui/grid.rs
@@ -429,6 +429,8 @@ pub fn show<'a>(
         focus::Ring::Inset { radius: 0 }
     };
     focus::hint(ui, &stop, visible, ring);
+    focus::region(ui, focus::Region::Grid, visible);
+    focus::claim(ui, focus::Region::Grid, &stop);
     output.focused = stop.gained_focus();
     // The cell is lit while the keyboard is in use and its keys come here:
     // not while a button or a field has them.
@@ -624,7 +626,8 @@ pub fn show<'a>(
                     pos2(rect.right() - HANDLE_WIDTH / 2.0, top),
                     pos2(rect.right() + HANDLE_WIDTH / 2.0, top + header_height),
                 );
-                let drag = ui.interact(handle, id.with(("resize", col)), Sense::drag());
+                // Only the pointer drags it: no stop for the Tab key.
+                let drag = ui.interact(handle, id.with(("resize", col)), Sense::DRAG);
                 if drag.hovered() || drag.dragged() {
                     ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeColumn);
                     painter.vline(
diff --git a/src/ui/keys.rs b/src/ui/keys.rs
--- a/src/ui/keys.rs
+++ b/src/ui/keys.rs
@@ -36,6 +36,7 @@ pub const SHORTCUTS: &[(&str, &str)] = &[
     ("Mod+F", "Filter bar"),
     ("Mod+P", "Quick open"),
     ("Mod+B", "Show or hide the sidebar"),
+    ("F6, Shift+F6", "Next / previous part of the window"),
     ("Mod+Alt+Left / Right", "Previous / next page"),
     ("Mod+.", "Cancel running query"),
     ("Esc", "Cancel connecting"),
@@ -48,7 +49,7 @@ pub const SHORTCUTS: &[(&str, &str)] = &[
     ("Arrows, Home/End, Enter", "Move in the tree"),
     ("Arrows, Page Up/Down, Home/End", "Move in the grid"),
     (
-        "j/k, h/l, [ ], i, Esc, /, y, s, d, gd, za, t, 1…9",
+        "j/k, h/l, Ctrl+H/L, [ ], i, Esc, /, y, s, d, gd, za, t, 1…9",
         "Omarchy: vim keys (shown in the status line)",
     ),
     ("?", "Shortcuts"),
@@ -288,6 +289,34 @@ pub fn handle(app: &mut App, ctx: &egui::Context) {
             }
         }
     });
+    // F6 steps through the parts of the window (the bar, the filter, the
+    // tree, the tabs, the table's header, the rows, the row panel), and
+    // Shift+F6 back. The terminal look steps through its panes alone with
+    // ctrl+l and ctrl+h.
+    if let Some(workspace) = app.workspace(active) {
+        use crate::ui::focus::{self, Region};
+        let from = match workspace.pane {
+            crate::model::Pane::Tree => Region::Tree,
+            crate::model::Pane::Grid => Region::Grid,
+        };
+        // The first that matches: egui ignores an extra Shift.
+        let pressed = |keys: [(Modifiers, Key); 2]| {
+            ctx.input_mut(|input| {
+                keys.into_iter()
+                    .position(|(modifiers, key)| input.consume_key(modifiers, key))
+            })
+        };
+        match pressed([(Modifiers::SHIFT, Key::F6), (Modifiers::NONE, Key::F6)]) {
+            Some(index) => focus::step(ctx, index == 1, None, Some(from)),
+            None if app.look.terminal && !editing => {
+                if let Some(index) = pressed([(Modifiers::CTRL, Key::H), (Modifiers::CTRL, Key::L)])
+                {
+                    focus::step(ctx, index == 1, Some(&Region::PANES), Some(from));
+                }
+            }
+            None => {}
+        }
+    }
     if !editing && grid {
         let (copy, shift) = ctx.input(|input| {
             (
diff --git a/src/ui/object_tabs.rs b/src/ui/object_tabs.rs
--- a/src/ui/object_tabs.rs
+++ b/src/ui/object_tabs.rs
@@ -76,6 +76,7 @@ pub fn show(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId) {
         .frame(Frame::new().fill(fill))
         .show(ui, |ui| {
             let full = ui.max_rect();
+            focus::region(ui, focus::Region::Tabs, full);
             let rule = if look.terminal {
                 palette.outline
             } else {
@@ -125,6 +126,9 @@ pub fn show(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId) {
                                 } else {
                                     mac_tab(ui, &one, bar, &look, &palette)
                                 };
+                                if is_active {
+                                    focus::claim(ui, focus::Region::Tabs, &response);
+                                }
                                 let label = format!("{name} {}", gettext(locale, "tab"));
                                 response.widget_info(|| {
                                     WidgetInfo::selected(
diff --git a/src/ui/picker.rs b/src/ui/picker.rs
--- a/src/ui/picker.rs
+++ b/src/ui/picker.rs
@@ -166,7 +166,11 @@ pub fn show(app: &mut App, ui: &mut egui::Ui) {
 
     // Header.
     let header = Rect::from_min_size(full.min, vec2(full.width(), header_height(&look)));
-    let drag = ui.interact(header, ui.id().with("picker-drag"), Sense::click_and_drag());
+    let drag = ui.interact(
+        header,
+        ui.id().with("picker-drag"),
+        Sense::CLICK | Sense::DRAG,
+    );
     if drag.drag_started() {
         ui.ctx().send_viewport_cmd(egui::ViewportCommand::StartDrag);
     }
diff --git a/src/ui/row_panel.rs b/src/ui/row_panel.rs
--- a/src/ui/row_panel.rs
+++ b/src/ui/row_panel.rs
@@ -13,6 +13,7 @@ use crate::i18n::gettext;
 use crate::model::{Action, CellPos, ConnTabId, RowFields, Tab, TabId, Workspace};
 use crate::theme::{Icon, Look, Palette};
 use crate::typography::{Text, TextRole};
+use crate::ui::focus::{self, Region};
 use crate::ui::format;
 use crate::ui::json_view;
 use crate::ui::widgets;
@@ -314,6 +315,8 @@ pub fn show(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId, id: TabId) {
                 });
                 return;
             };
+            // A part of the window to step to once it has a row to show.
+            focus::region(ui, Region::Panel, full);
             let structure = source.structure;
             let info = |name: &str| {
                 let key =
@@ -428,6 +431,7 @@ pub fn show(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId, id: TabId) {
                 let response = ui.interact(esc, ui.id().with("close"), Sense::click());
                 let close = gettext(locale, "Close the row panel");
                 response.widget_info(|| WidgetInfo::labeled(WidgetType::Button, true, &close));
+                focus::claim(ui, Region::Panel, &response);
                 ui.painter().rect_stroke(
                     esc,
                     CornerRadius::same(3),
@@ -539,7 +543,7 @@ pub fn show(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId, id: TabId) {
                     right -= 34.0;
                     let mut child = ui.new_child(egui::UiBuilder::new().max_rect(place));
                     child.add_enabled_ui(enabled, |ui| {
-                        if widgets::icon_button_sized(
+                        let button = widgets::icon_button_sized(
                             ui,
                             icon,
                             &gettext(locale, label),
@@ -547,9 +551,12 @@ pub fn show(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId, id: TabId) {
                             14.0,
                             &look,
                             &palette,
-                        )
-                        .clicked()
-                        {
+                        );
+                        // The panel starts with its first button.
+                        if icon == Icon::X {
+                            focus::claim(ui, Region::Panel, &button);
+                        }
+                        if button.clicked() {
                             actions.push(action);
                         }
                     });
diff --git a/src/ui/sidebar.rs b/src/ui/sidebar.rs
--- a/src/ui/sidebar.rs
+++ b/src/ui/sidebar.rs
@@ -177,7 +177,7 @@ pub fn show(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId) {
                         boxed: true,
                         role: TextRole::pick(&look, TextRole::UiBody, TextRole::OField),
                     };
-                    widgets::filter_field(
+                    let field = widgets::filter_field(
                         ui,
                         &mut workspace.tree.filter,
                         &hint,
@@ -185,8 +185,10 @@ pub fn show(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId) {
                         style,
                         &look,
                         &palette,
-                    )
-                    .widget_info(|| WidgetInfo::labeled(WidgetType::TextEdit, true, "Filter"));
+                    );
+                    field.widget_info(|| WidgetInfo::labeled(WidgetType::TextEdit, true, "Filter"));
+                    focus::region(ui, focus::Region::Search, field.rect.expand(8.0));
+                    focus::claim(ui, focus::Region::Search, &field);
                 }
             });
             ui.add_space(if look.terminal { 6.0 } else { 8.0 });
@@ -299,6 +301,8 @@ pub fn show(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId) {
                 });
                 focus::pane(ui, &stop);
                 focus::hint(ui, &stop, area, focus::Ring::Own);
+                focus::region(ui, focus::Region::Tree, area);
+                focus::claim(ui, focus::Region::Tree, &stop);
                 // The cursor it had, the open object's row, or its first.
                 let start = cursor_start
                     .clone()
diff --git a/src/ui/sql_text.rs b/src/ui/sql_text.rs
--- a/src/ui/sql_text.rs
+++ b/src/ui/sql_text.rs
@@ -503,6 +503,8 @@ fn edit(ui: &mut Ui, sql_tab: &mut SqlTab, field: &Field<'_>) -> Edited {
         .desired_rows(rows as usize)
         .layouter(&mut layouter)
         .show(ui);
+    crate::ui::focus::region(ui, crate::ui::focus::Region::Editor, output.response.rect);
+    crate::ui::focus::claim(ui, crate::ui::focus::Region::Editor, &output.response);
     // The script fills its pane: the caret says where the keyboard is.
     crate::ui::focus::hint(
         ui,
diff --git a/src/ui/workspace.rs b/src/ui/workspace.rs
--- a/src/ui/workspace.rs
+++ b/src/ui/workspace.rs
@@ -9,6 +9,7 @@ use crate::i18n::gettext;
 use crate::model::{Action, ConnTabId, ObjectView, SessionStatus};
 use crate::theme::{Icon, Look, Palette};
 use crate::typography::{Text, TextRole};
+use crate::ui::focus::{self, Region};
 use crate::ui::format::display_safe;
 use crate::ui::states;
 use crate::ui::widgets::{self, ButtonSpec};
@@ -861,8 +862,10 @@ fn top_bar(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId) {
         .frame(Frame::new().fill(tint))
         .show(ui, |ui| {
             let rect = ui.max_rect();
-            // The empty bar moves the window, as a title bar does.
-            let drag = ui.interact(rect, ui.id().with("drag"), Sense::click_and_drag());
+            // The empty bar moves the window, as a title bar does. Only
+            // the pointer can: it is no stop for the Tab key.
+            let drag = ui.interact(rect, ui.id().with("drag"), Sense::CLICK | Sense::DRAG);
+            focus::region(ui, Region::Header, rect);
             if drag.drag_started() {
                 ui.ctx().send_viewport_cmd(egui::ViewportCommand::StartDrag);
             }
@@ -941,6 +944,7 @@ fn mac_bar(
         let label = gettext(locale, "Connections");
         let response = ui.interact(connections, ui.id().with("connections"), Sense::click());
         response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, &label));
+        focus::claim(ui, Region::Header, &response);
         let (fill, tint) = if response.hovered() {
             (face(1.0), palette.text)
         } else {
@@ -1204,6 +1208,7 @@ fn terminal_bar(
         );
         let response = ui.interact(button, ui.id().with("connections"), Sense::click());
         response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, &label));
+        focus::claim(ui, Region::Header, &response);
         let line = if response.hovered() {
             env.base()
         } else {
````

- [ ] **Step 4: Run the tests**

Run: `~/.cargo/bin/cargo test --locked --lib ui::tests`
Expected: PASS.

Then all of the library's: `~/.cargo/bin/cargo test --locked --lib`
Expected: `test result: ok. 881 passed; 0 failed; 1 ignored`.

- [ ] **Step 5: Format and lint**

```bash
~/.cargo/bin/cargo fmt --all --check
~/.cargo/bin/cargo clippy --locked --workspace --all-targets -- -D warnings
~/.cargo/bin/cargo clippy --locked --workspace --all-targets --features shots -- -D warnings
```

Expected: no output.

- [ ] **Step 6: Commit point**

Stop and report. If the user has asked for commits:

```bash
git add -A src AGENTS.md
git commit -S -m "Step through the window's parts with F6" -m "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

### Task 14: A disabled button still says why

**Files:**
- Modify: `src/ui/widgets.rs`
- Modify: `src/ui/mod.rs` (a test)

A `ButtonSpec` that cannot be pressed takes the Tab key, shows its reason as a tooltip while the keyboard is on it, and gives the reason to a screen reader as its description. Enter does not press it.

- [ ] **Step 1: Write the tests**

New in `src/ui/mod.rs`: `a_button_that_cannot_be_pressed_still_says_why_to_the_keyboard`.

````diff
diff --git a/src/ui/mod.rs b/src/ui/mod.rs
--- a/src/ui/mod.rs
+++ b/src/ui/mod.rs
@@ -8482,6 +8482,43 @@ mod tests {
         );
     }
 
+    #[test]
+    fn a_button_that_cannot_be_pressed_still_says_why_to_the_keyboard() {
+        let mut harness = Harness::new();
+        harness.set_look(crate::theme::Look::macos());
+        let tab = with_page(&mut harness);
+        let mut reached = false;
+        for _ in 0..60 {
+            harness.press(Key::Tab, Modifiers::NONE);
+            let tree = harness.settle();
+            if focused_name(&tree) == "Add row" {
+                reached = true;
+                // Why not, where a screen reader finds it and on screen.
+                let (_, node) = tree.nodes.iter().find(|(id, _)| *id == tree.focus).unwrap();
+                assert_eq!(
+                    node.description(),
+                    Some("Editing arrives in a later version")
+                );
+                assert!(node.is_disabled());
+                assert!(
+                    harness
+                        .painted
+                        .iter()
+                        .any(|(text, _)| text == "Editing arrives in a later version"),
+                    "{:?}",
+                    harness.painted
+                );
+                break;
+            }
+        }
+        assert!(reached, "Add row is a Tab stop");
+        // Enter does not press it.
+        let sent = harness.app.backend.sent.len();
+        harness.press(Key::Enter, Modifiers::NONE);
+        assert_eq!(harness.app.backend.sent.len(), sent);
+        assert!(harness.app.workspace(tab).is_some());
+    }
+
     #[test]
     fn the_terminal_marks_the_pane_the_keys_go_to() {
         // An accent line round something as tall as a pane.
````

- [ ] **Step 2: Run them and see them fail**

Run: `~/.cargo/bin/cargo test --locked --lib`

Expected, before the code below is there:

````text
test ui::tests::a_button_that_cannot_be_pressed_still_says_why_to_the_keyboard ... FAILED
test result: FAILED. 881 passed; 1 failed; 1 ignored; 0 measured; 0 filtered out
````

- [ ] **Step 3: Write the code**

````diff
diff --git a/src/ui/widgets.rs b/src/ui/widgets.rs
--- a/src/ui/widgets.rs
+++ b/src/ui/widgets.rs
@@ -1344,10 +1344,12 @@ impl<'a> ButtonSpec<'a> {
     /// Draws the button in `rect`.
     pub fn show_at(self, ui: &mut Ui, rect: Rect, look: &Look, palette: &Palette) -> Response {
         let enabled = self.kind != ButtonKind::Disabled;
+        // A button that cannot be pressed still takes the Tab key, so the
+        // keyboard can read why.
         let sense = if enabled {
             Sense::click()
         } else {
-            Sense::hover()
+            Sense::focusable_noninteractive()
         };
         let name = self.label.unwrap_or(self.text);
         let response = ui.interact(rect, self.id(ui), sense);
@@ -1470,7 +1472,17 @@ impl<'a> ButtonSpec<'a> {
             }
         }
         match self.reason {
-            Some(reason) => response.on_hover_text(reason),
+            Some(reason) => {
+                // Said to a screen reader, and shown while the keyboard is
+                // on the button as it is under the pointer.
+                ui.ctx().accesskit_node_builder(response.id, |node| {
+                    node.set_description(reason);
+                });
+                if focus::shown(&response) {
+                    response.show_tooltip_text(reason);
+                }
+                response.on_hover_text(reason)
+            }
             None => response,
         }
     }
````

- [ ] **Step 4: Run the tests**

Run: `~/.cargo/bin/cargo test --locked --lib ui::tests::a_button_that_cannot`
Expected: PASS.

Then all of the library's: `~/.cargo/bin/cargo test --locked --lib`
Expected: `test result: ok. 882 passed; 0 failed; 1 ignored`.

- [ ] **Step 5: Format and lint**

```bash
~/.cargo/bin/cargo fmt --all --check
~/.cargo/bin/cargo clippy --locked --workspace --all-targets -- -D warnings
~/.cargo/bin/cargo clippy --locked --workspace --all-targets --features shots -- -D warnings
```

Expected: no output.

- [ ] **Step 6: Commit point**

Stop and report. If the user has asked for commits:

```bash
git add -A src AGENTS.md
git commit -S -m "Let the keyboard read why a button is disabled" -m "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

### Task 15: The key column stays in sight

**Files:**
- Modify: `src/ui/grid.rs`
- Modify: `src/ui/data_view.rs`
- Modify: `src/ui/workspace.rs`
- Modify: `src/ui/mod.rs` (a test)
- Modify: `src/shots.rs`

When the grid's first column is part of the primary key it is pinned: drawn last, where the view begins, over what scrolled under it, with a line at its right. A click over it picks its cell, and a cell revealed by the arrows never hides under it. The grid records which columns were in view (`ColumnsShown`); the footer (and the terminal's status line) says "Columns 1–10 of 40 · id pinned" while some are out of view and there is room.

- [ ] **Step 1: Write the tests**

New in `src/ui/grid.rs`: `a_leading_key_column_stays_in_sight_while_the_others_scroll`, `a_grid_without_a_leading_key_pins_nothing`. New in `src/ui/mod.rs`: `the_status_line_says_which_columns_are_in_view_while_some_are_not`, on a fixture `wide_table`.

````diff
diff --git a/src/ui/grid.rs b/src/ui/grid.rs
--- a/src/ui/grid.rs
+++ b/src/ui/grid.rs
@@ -1452,6 +1602,133 @@ mod tests {
         }
     }
 
+    #[test]
+    fn a_leading_key_column_stays_in_sight_while_the_others_scroll() {
+        let ctx = egui::Context::default();
+        crate::theme::install(&ctx, false, &Look::standard());
+        ctx.enable_accesskit();
+        let id = egui::Id::new("grid");
+        let names: Vec<String> = (0..8).map(|col| format!("column_{col}")).collect();
+        let columns: Vec<Column<'_>> = names
+            .iter()
+            .enumerate()
+            .map(|(col, name)| Column {
+                name,
+                type_line: "int8".into(),
+                numeric: false,
+                sort: None,
+                key: col == 0,
+                flexible: false,
+                sortable: true,
+            })
+            .collect();
+        // One frame of a 320 pt wide grid: its output, where each cell's
+        // text was painted, and the AccessKit tree.
+        let frame = |events: Vec<egui::Event>| {
+            let mut result = GridOutput::default();
+            let input = egui::RawInput {
+                screen_rect: Some(egui::Rect::from_min_size(
+                    egui::Pos2::ZERO,
+                    egui::vec2(320.0, 300.0),
+                )),
+                events,
+                ..Default::default()
+            };
+            let mut output = ctx.run_ui(input, |ui| {
+                result = show(
+                    ui,
+                    id,
+                    &columns,
+                    3,
+                    0,
+                    None,
+                    false,
+                    &Palette::light(),
+                    &Look::standard(),
+                    |row, col| Cell {
+                        text: format!("r{row}c{col}").into(),
+                        null: false,
+                        style: Style::Plain,
+                    },
+                );
+            });
+            output.textures_delta.clear();
+            let texts: Vec<(String, f32)> = output
+                .shapes
+                .iter()
+                .filter_map(|clipped| match &clipped.shape {
+                    egui::Shape::Text(text) if clipped.clip_rect.contains(text.pos) => {
+                        Some((text.galley.text().to_owned(), text.pos.x))
+                    }
+                    _ => None,
+                })
+                .collect();
+            let tree = output.platform_output.accesskit_update.expect("accesskit");
+            (result, texts, tree)
+        };
+        let at = |texts: &[(String, f32)], cell: &str| {
+            texts
+                .iter()
+                .rev()
+                .find(|(text, _)| text == cell)
+                .map(|(_, x)| *x)
+        };
+        frame(Vec::new());
+        let (_, texts, tree) = frame(Vec::new());
+        let shown = columns_shown(&ctx, id).expect("the columns in view");
+        assert_eq!((shown.first, shown.total, shown.pinned), (0, 8, true));
+        assert!(shown.partial(), "{shown:?}");
+        let before = at(&texts, "r0c0").expect("the key");
+        // To the columns out of view: the pill at the header's end.
+        let more = tree
+            .nodes
+            .iter()
+            .find(|(_, node)| {
+                node.label()
+                    .is_some_and(|name| name.ends_with("more columns"))
+            })
+            .map(|(id, _)| *id)
+            .expect("the pill");
+        frame(vec![click(more)]);
+        frame(Vec::new());
+        let (_, texts, _) = frame(Vec::new());
+        let scrolled = columns_shown(&ctx, id).unwrap();
+        assert!(scrolled.last > shown.last, "{scrolled:?}");
+        // The range is of the columns that scrolled into view: the key is
+        // in sight beside it, and some are still out of it.
+        assert!(scrolled.first > 1 && scrolled.pinned, "{scrolled:?}");
+        assert!(scrolled.partial(), "{scrolled:?}");
+        // The key is where it was; the column after it went under it.
+        assert_eq!(at(&texts, "r0c0"), Some(before));
+        assert!(at(&texts, "r0c1").is_none_or(|x| x < before));
+        // A click over the key picks the key's cell, not one under it.
+        let over_key = egui::pos2(before + 4.0, 45.0 + 26.0 * 1.5);
+        let press = |pressed| egui::Event::PointerButton {
+            pos: over_key,
+            button: egui::PointerButton::Primary,
+            pressed,
+            modifiers: egui::Modifiers::NONE,
+        };
+        frame(vec![egui::Event::PointerMoved(over_key), press(true)]);
+        let (output, _, _) = frame(vec![press(false)]);
+        assert_eq!(output.clicked, Some(CellPos { row: 1, col: 0 }));
+    }
+
+    #[test]
+    fn a_grid_without_a_leading_key_pins_nothing() {
+        let ctx = egui::Context::default();
+        crate::theme::install(&ctx, false, &Look::standard());
+        ctx.enable_accesskit();
+        // A result's columns: none is a key.
+        let mut columns = columns_that(false);
+        columns[0].key = false;
+        frame_of(&ctx, &columns, 3, Vec::new());
+        frame_of(&ctx, &columns, 3, Vec::new());
+        let shown = columns_shown(&ctx, egui::Id::new("grid")).expect("the columns in view");
+        assert!(!shown.pinned);
+        assert!(!shown.partial(), "every column fits: {shown:?}");
+    }
+
     #[test]
     fn columns_are_found_by_x() {
         let widths = [100.0, 50.0, 80.0];
diff --git a/src/ui/mod.rs b/src/ui/mod.rs
--- a/src/ui/mod.rs
+++ b/src/ui/mod.rs
@@ -8519,6 +8519,74 @@ mod tests {
         assert!(harness.app.workspace(tab).is_some());
     }
 
+    /// A table of twelve text columns behind a key, open in `harness`.
+    fn wide_table(harness: &mut Harness) {
+        use tabletist_db::{ColumnInfo, ColumnMeta, RowPage, Structure, Value, ValueKind};
+        let tab = harness.connect_fake();
+        harness.app.apply(crate::model::Action::OpenObject {
+            tab,
+            object: tabletist_db::ObjectRef::new("main", "wide"),
+            kind: tabletist_db::ObjectKind::Table,
+            pin: true,
+        });
+        let names: Vec<String> = std::iter::once("id".to_owned())
+            .chain((1..12).map(|index| format!("a_rather_long_column_{index}")))
+            .collect();
+        harness.answer_structure(Structure {
+            columns: names
+                .iter()
+                .map(|name| ColumnInfo {
+                    name: name.clone(),
+                    type_name: "text".into(),
+                    nullable: true,
+                    default: None,
+                    comment: None,
+                    allowed_values: None,
+                })
+                .collect(),
+            primary_key: vec!["id".into()],
+            indexes: Vec::new(),
+            foreign_keys: Vec::new(),
+        });
+        harness.answer_rows(RowPage {
+            columns: names
+                .iter()
+                .map(|name| ColumnMeta {
+                    name: name.clone(),
+                    type_name: "text".into(),
+                    kind: ValueKind::Text,
+                })
+                .collect(),
+            rows: vec![vec![Value::Text("value".into()); 12]; 3],
+            has_more: false,
+            ordered_by_key: true,
+            elapsed: std::time::Duration::ZERO,
+        });
+        harness.settle();
+        harness.settle();
+    }
+
+    #[test]
+    fn the_status_line_says_which_columns_are_in_view_while_some_are_not() {
+        for look in crate::theme::Look::ALL {
+            let said = |harness: &Harness| {
+                harness
+                    .painted
+                    .iter()
+                    .any(|(text, _)| text.contains("of 12 · id pinned"))
+            };
+            let mut harness = Harness::with_size(egui::vec2(1600.0, 600.0));
+            harness.set_look(look);
+            wide_table(&mut harness);
+            assert!(said(&harness), "{}: {:?}", look.name, harness.painted);
+            // With room for every column there is nothing to say.
+            let mut harness = Harness::with_size(egui::vec2(6000.0, 600.0));
+            harness.set_look(look);
+            wide_table(&mut harness);
+            assert!(!said(&harness), "{}", look.name);
+        }
+    }
+
     #[test]
     fn the_terminal_marks_the_pane_the_keys_go_to() {
         // An accent line round something as tall as a pane.
````

- [ ] **Step 2: Run them and see them fail**

Run: `~/.cargo/bin/cargo test --locked --lib`

Expected, before the code below is there:

````text
Does not compile:
error[E0425]: cannot find function `columns_shown` in this scope
````

- [ ] **Step 3: Write the code**

````diff
diff --git a/src/ui/data_view.rs b/src/ui/data_view.rs
--- a/src/ui/data_view.rs
+++ b/src/ui/data_view.rs
@@ -746,6 +746,47 @@ fn dashed_rect(ui: &egui::Ui, rect: Rect, radius: f32, color: egui::Color32) {
     }
 }
 
+/// The id of a table's grid. Full precision widens timestamps: the columns
+/// are fitted again, as a grid of its own.
+fn grid_id(tab: ConnTabId, object_tab: TabId, full_precision: bool) -> Id {
+    Id::new(("grid", tab.0, object_tab.0, full_precision))
+}
+
+/// What a status line says of the columns while some are out of view:
+/// `Columns 1–10 of 40 · id pinned`, or the terminal's `cols 1–7 of 40`.
+/// Nothing while every column shows. It reads what the grid drew last, so
+/// it is a frame behind a scroll.
+pub fn columns_note(
+    ctx: &egui::Context,
+    workspace: &crate::model::Workspace,
+    tab: ConnTabId,
+    object: &ObjectTab,
+    look: &Look,
+    locale: crate::i18n::Locale,
+) -> Option<String> {
+    let id = grid_id(tab, object.id, workspace.full_precision);
+    let shown = grid::columns_shown(ctx, id).filter(grid::ColumnsShown::partial)?;
+    let say = |text: &'static str| look.label(&gettext(locale, text));
+    let noun = if look.terminal { "cols" } else { "Columns" };
+    let mut note = format!(
+        "{} {}–{} {} {}",
+        say(noun),
+        shown.first + 1,
+        shown.last + 1,
+        say("of"),
+        shown.total
+    );
+    let first = object.page().and_then(|page| page.columns.first());
+    if let (true, Some(column)) = (shown.pinned, first) {
+        note.push_str(&format!(
+            " · {} {}",
+            format::display_safe(&column.name),
+            say("pinned")
+        ));
+    }
+    Some(note)
+}
+
 /// The status footer: the page's range and paging, then what is selected
 /// and how long the query took.
 pub fn footer(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId, object_tab: TabId) {
@@ -781,6 +822,10 @@ pub fn footer(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId, object_tab: TabI
     let timing = page.map(|page| format::elapsed(page.elapsed));
     let unordered = page.is_some_and(|page| !page.ordered_by_key) && object.query.sort.is_empty();
     let selected = object.selection.is_some();
+    let columns = app
+        .workspace(tab)
+        .filter(|_| view == ObjectView::Data)
+        .and_then(|workspace| columns_note(ui.ctx(), workspace, tab, object, &look, locale));
     let mut actions = Vec::new();
     egui::Panel::bottom(Id::new(("object-footer", tab.0, object_tab.0)))
         .exact_size(33.0)
@@ -837,6 +882,31 @@ pub fn footer(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId, object_tab: TabI
                     }
                     ui.spacing_mut().item_spacing.x = 16.0;
                     ui.add_space(14.0);
+                    // The columns in view, where the footer has the room: what
+                    // its right end says comes first.
+                    if let Some(columns) = &columns {
+                        let width = |text: &str| {
+                            widgets::secondary(&look).width(ui.ctx(), look.faces, text)
+                        };
+                        let state = if selected {
+                            format!(
+                                "{} · {}",
+                                gettext(locale, "1 row selected"),
+                                gettext(locale, "read-only")
+                            )
+                        } else {
+                            gettext(locale, "read-only").into_owned()
+                        };
+                        let query = timing
+                            .as_ref()
+                            .map(|timing| format!("{} {timing}", gettext(locale, "Query")));
+                        let taken = width(&state)
+                            + query.as_deref().map_or(0.0, |query| 16.0 + width(query))
+                            + 16.0;
+                        if ui.available_width() >= width(columns) + 16.0 + taken {
+                            note(ui, columns, status, &look);
+                        }
+                    }
                     if filtered {
                         note(ui, &gettext(locale, "Filtered"), palette.accent, &look)
                             .on_hover_text(gettext(locale, "Cmd/Ctrl+F edits the filter"));
@@ -1020,8 +1090,7 @@ pub fn show(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId, object_tab: TabId)
         let tags = crate::ui::value_tags::Tags::of_page(page, structure);
         let output = grid::show(
             ui,
-            // Full precision widens timestamps: the columns fit again.
-            Id::new(("grid", tab.0, object_tab.0, full_precision)),
+            grid_id(tab, object_tab, full_precision),
             &columns,
             page.rows.len(),
             object.query.offset,
diff --git a/src/ui/grid.rs b/src/ui/grid.rs
--- a/src/ui/grid.rs
+++ b/src/ui/grid.rs
@@ -96,6 +96,33 @@ pub struct GridOutput {
     pub focused: bool,
 }
 
+/// Which of a grid's columns were in view when it was last drawn.
+#[derive(Clone, Copy, Debug, PartialEq, Eq)]
+pub struct ColumnsShown {
+    /// The first and the last column in view, counted from 0. A pinned
+    /// column counts among them while the column after it is in view; past
+    /// that it is in sight but not of the range.
+    pub first: usize,
+    pub last: usize,
+    pub total: usize,
+    /// The grid's first column stays in sight while the others scroll.
+    pub pinned: bool,
+}
+
+impl ColumnsShown {
+    /// Whether some columns are out of view.
+    pub fn partial(&self) -> bool {
+        let apart = usize::from(self.pinned && self.first > 0);
+        self.last + 1 - self.first + apart < self.total
+    }
+}
+
+/// The columns the grid `id` showed when it was last drawn: what a status
+/// line says of them, a frame later.
+pub fn columns_shown(ctx: &egui::Context, id: Id) -> Option<ColumnsShown> {
+    ctx.data(|data| data.get_temp(id.with("columns-shown")))
+}
+
 /// How wide `text` is in `role`, in points (laid out once, then cached).
 fn text_width(ui: &Ui, text: &str, role: TextRole, look: &Look) -> f32 {
     role.width(ui.ctx(), look.faces, text)
@@ -342,6 +369,7 @@ pub fn forget(ctx: &egui::Context, id: Id) {
             data.remove::<egui::scroll_area::State>(scroll);
         }
         data.remove::<Option<CellPos>>(id.with("last-selection"));
+        data.remove::<ColumnsShown>(id.with("columns-shown"));
         data.remove::<Kept>(id);
     });
 }
@@ -413,6 +441,24 @@ pub fn show<'a>(
     let total = gutter + widths.iter().sum::<f32>();
     let hairline = crate::ui::widgets::hairline(ui);
     let visible = ui.max_rect();
+    // A key column that leads the grid stays in sight: the others scroll
+    // under it, so a row is never read without knowing whose it is.
+    let pinned = columns.len() > 1 && columns[0].key;
+    // Drawn last, over what scrolled under it; the others in their order.
+    let order: Vec<usize> = (usize::from(pinned)..columns.len())
+        .chain(pinned.then_some(0))
+        .collect();
+    // Each column's left edge, from the first one's: found once a frame,
+    // not once a cell. A width dragged this frame moves its neighbours the
+    // next.
+    let lefts: Vec<f32> = widths
+        .iter()
+        .scan(0.0, |edge, width| {
+            let left = *edge;
+            *edge += width;
+            Some(left)
+        })
+        .collect();
     // The grid is one Tab stop, not one for each row: with the keyboard on
     // it the arrows move the selected cell, which shows where they are.
     let stop = ui.interact(visible, id.with("keys"), Sense::focusable_noninteractive());
@@ -445,16 +491,32 @@ pub fn show<'a>(
             let full = total.max(ui.available_width());
             ui.allocate_space(vec2(total, header_height));
 
+            // How far the pinned column stands off its place: as far as
+            // the grid is scrolled sideways.
+            let shift = if pinned {
+                // Under a point it is the clip's own margin, not a scroll.
+                Some(ui.clip_rect().left() - origin.x)
+                    .filter(|shift| *shift >= 1.0)
+                    .unwrap_or(0.0)
+            } else {
+                0.0
+            };
             if let Some(target) = reveal {
                 let col = target.col.min(widths.len().saturating_sub(1));
                 let x = origin.x + gutter + widths[..col].iter().sum::<f32>();
                 let y = origin.y + header_height + target.row as f32 * row_height;
                 // Include the header's height above the row so the sticky
-                // header never covers it.
+                // header never covers it, and the pinned column's width
+                // before a cell that scrolls so that never does either.
+                let cover = if pinned && col > 0 {
+                    gutter + widths[0]
+                } else {
+                    0.0
+                };
                 let rect = Rect::from_min_size(
-                    pos2(x, y - header_height),
+                    pos2(x - cover, y - header_height),
                     vec2(
-                        widths.get(col).copied().unwrap_or(0.0),
+                        widths.get(col).copied().unwrap_or(0.0) + cover,
                         row_height + header_height,
                     ),
                 );
@@ -470,7 +532,15 @@ pub fn show<'a>(
                 if response.clicked() {
                     let col = response
                         .interact_pointer_pos()
-                        .map(|pointer| column_at(&widths, pointer.x - rect.left() - gutter))
+                        .map(|pointer| {
+                            let x = pointer.x - rect.left() - gutter;
+                            // Over the pinned column, whatever is under it.
+                            if pinned && x - shift < widths[0] {
+                                0
+                            } else {
+                                column_at(&widths, x)
+                            }
+                        })
                         .unwrap_or(0);
                     output.clicked = Some(CellPos { row, col });
                 }
@@ -497,25 +567,55 @@ pub fn show<'a>(
                     let y = painter.round_to_pixel_center(rect.bottom() - hairline / 2.0);
                     painter.hline(rect.x_range(), y, Stroke::new(hairline, palette.surface));
                 }
-                if selected_row {
-                    if look.terminal {
-                        // The cursor: a bold accent block in the gutter.
-                        let cursor =
-                            Text::one(look, TextRole::OGroup, "▌", palette.accent).layout(ui.ctx());
-                        cursor.paint_center(
-                            &painter,
-                            pos2(rect.left() + GUTTER / 2.0, rect.center().y),
-                        );
-                    } else if !lit_row {
-                        let bar = Rect::from_min_size(rect.min, vec2(3.0, rect.height()));
-                        painter.rect_filled(bar, CornerRadius::ZERO, palette.accent);
-                    }
-                }
-                let mut x = rect.left() + gutter;
-                for (col, width) in widths.iter().enumerate() {
+                for &col in &order {
+                    let left = rect.left() + gutter + lefts[col];
                     let cell_rect =
-                        Rect::from_min_size(pos2(x, rect.top()), vec2(*width, row_height));
-                    x += width;
+                        Rect::from_min_size(pos2(left, rect.top()), vec2(widths[col], row_height));
+                    // The first column, and the row's mark before it, stand
+                    // where the view begins while it is pinned.
+                    let cell_rect = if col == 0 {
+                        let lead = Rect::from_min_max(
+                            pos2(rect.left() + shift, rect.top()),
+                            pos2(cell_rect.right() + shift, rect.bottom()),
+                        );
+                        if shift > 0.0 {
+                            // Over what scrolled under: the row's own fill.
+                            let under = fill.unwrap_or(palette.window);
+                            painter.rect_filled(lead, CornerRadius::ZERO, under);
+                            if !look.terminal {
+                                let y =
+                                    painter.round_to_pixel_center(rect.bottom() - hairline / 2.0);
+                                painter.hline(
+                                    lead.x_range(),
+                                    y,
+                                    Stroke::new(hairline, palette.surface),
+                                );
+                            }
+                            painter.vline(
+                                lead.right() - hairline / 2.0,
+                                lead.y_range(),
+                                Stroke::new(hairline, palette.outline),
+                            );
+                        }
+                        if selected_row {
+                            if look.terminal {
+                                // The cursor: a bold accent block in the
+                                // gutter.
+                                let cursor = Text::one(look, TextRole::OGroup, "▌", palette.accent)
+                                    .layout(ui.ctx());
+                                cursor.paint_center(
+                                    &painter,
+                                    pos2(lead.left() + GUTTER / 2.0, rect.center().y),
+                                );
+                            } else if !lit_row {
+                                let bar = Rect::from_min_size(lead.min, vec2(3.0, rect.height()));
+                                painter.rect_filled(bar, CornerRadius::ZERO, palette.accent);
+                            }
+                        }
+                        cell_rect.translate(vec2(shift, 0.0))
+                    } else {
+                        cell_rect
+                    };
                     if !ui.is_rect_visible(cell_rect) {
                         continue;
                     }
@@ -595,9 +695,28 @@ pub fn show<'a>(
             painter.rect_filled(header, CornerRadius::ZERO, header_fill);
             let y = painter.round_to_pixel_center(header.bottom() - hairline / 2.0);
             painter.hline(header.x_range(), y, Stroke::new(hairline, palette.outline));
-            let mut x = origin.x + gutter;
-            for (col, column) in columns.iter().enumerate() {
-                let rect = Rect::from_min_size(pos2(x, top), vec2(widths[col], header_height));
+            for &col in &order {
+                let column = &columns[col];
+                let left = origin.x + gutter + lefts[col];
+                let rect = Rect::from_min_size(pos2(left, top), vec2(widths[col], header_height));
+                // The pinned column's header stands with its cells, over
+                // the headers that scrolled under it.
+                let rect = if col == 0 && shift > 0.0 {
+                    let lead = Rect::from_min_max(
+                        pos2(origin.x + shift, top),
+                        pos2(rect.right() + shift, header.bottom()),
+                    );
+                    painter.rect_filled(lead, CornerRadius::ZERO, header_fill);
+                    painter.hline(lead.x_range(), y, Stroke::new(hairline, palette.outline));
+                    painter.vline(
+                        lead.right() - hairline / 2.0,
+                        lead.y_range(),
+                        Stroke::new(hairline, palette.outline),
+                    );
+                    rect.translate(vec2(shift, 0.0))
+                } else {
+                    rect
+                };
                 // A header that sorts nothing still takes the pointer's
                 // clicks, and drops them: a hover-only header would let
                 // them through to a row scrolled under it.
@@ -640,11 +759,42 @@ pub fn show<'a>(
                     widths[col] = (widths[col] + drag.drag_delta().x).max(MIN_WIDTH);
                     keep = true;
                 }
-                x += widths[col];
             }
             origin
         });
 
+    // The columns in view, for the status line to say: the ones that show
+    // any of themselves past the pinned one.
+    {
+        let offset = scroll.state.offset.x;
+        let from = offset + if pinned { gutter + widths[0] } else { 0.0 };
+        let to = offset + scroll.inner_rect.width();
+        let mut edge = gutter;
+        let mut seen: Option<(usize, usize)> = None;
+        for (col, width) in widths.iter().enumerate() {
+            let scrolls = !(pinned && col == 0);
+            if scrolls && edge < to && edge + width > from {
+                seen = Some((seen.map_or(col, |(first, _)| first), col));
+            }
+            edge += width;
+        }
+        // The pinned column leads the range while its neighbour shows.
+        let seen = match seen {
+            Some((1, last)) if pinned => Some((0, last)),
+            None if pinned => Some((0, 0)),
+            seen => seen,
+        };
+        if let Some((first, last)) = seen {
+            let shown = ColumnsShown {
+                first,
+                last,
+                total: widths.len(),
+                pinned,
+            };
+            ui.data_mut(|data| data.insert_temp(id.with("columns-shown"), shown));
+        }
+    }
+
     // Columns the viewport cuts off entirely: a pill at the header's right
     // edge says how many, and scrolls to them.
     if !look.terminal {
diff --git a/src/ui/workspace.rs b/src/ui/workspace.rs
--- a/src/ui/workspace.rs
+++ b/src/ui/workspace.rs
@@ -1547,10 +1547,20 @@ fn status_line(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId) {
                 .map(|count| count.to_string())
                 .or_else(|| (!page.has_more).then(|| last.to_string()))
                 .unwrap_or_else(|| "?".into());
-            Some(format!(
+            let rows = format!(
                 "{first}–{last}/{total} · {}",
                 crate::ui::format::elapsed(page.elapsed)
-            ))
+            );
+            // The columns in view first, while some are out of it.
+            let columns = (object.view == ObjectView::Data)
+                .then(|| {
+                    super::data_view::columns_note(ui.ctx(), workspace, tab, object, &look, locale)
+                })
+                .flatten();
+            Some(match columns {
+                Some(columns) => format!("{columns} · {rows}"),
+                None => rows,
+            })
         })
         .unwrap_or_default();
     egui::Panel::bottom(egui::Id::new(("status-line", tab.0)))
````

- [ ] **Step 4: Run the tests**

Run: `~/.cargo/bin/cargo test --locked --lib ui::`
Expected: PASS.

Then all of the library's: `~/.cargo/bin/cargo test --locked --lib`
Expected: `test result: ok. 885 passed; 0 failed; 1 ignored`.

- [ ] **Step 5: Add the scenes**

````diff
diff --git a/src/shots.rs b/src/shots.rs
--- a/src/shots.rs
+++ b/src/shots.rs
@@ -530,6 +530,31 @@ fn shots() {
             harness.press(egui::Key::ArrowDown, egui::Modifiers::NONE);
         }
     });
+    // A wide table scrolled to its last columns: the key stays in sight.
+    both("values-scrolled", |harness| {
+        let tab = harness.connect_fake();
+        let workspace = harness.app.workspace_mut(tab).unwrap();
+        workspace.name = "Bookshop".into();
+        workspace.driver = Driver::Postgres;
+        harness.app.apply(Action::OpenObject {
+            tab,
+            object: ObjectRef::new("public", "book_images"),
+            kind: ObjectKind::Table,
+            pin: true,
+        });
+        harness.answer_structure(structure());
+        harness.answer_rows(page());
+        // The last column's cell: the grid scrolls to show it.
+        let object_tab = harness.app.workspace(tab).unwrap().active_tab.unwrap();
+        harness.app.apply(Action::SelectCell {
+            tab,
+            id: object_tab,
+            cell: CellPos { row: 4, col: 5 },
+        });
+        harness.app.apply(Action::ToggleRowPanel(tab));
+        // The scroll glides: let it arrive.
+        harness.finish_animations();
+    });
     both("structure", |harness| {
         let tab = workspace(harness);
         let object_tab = harness.app.workspace(tab).unwrap().active_tab.unwrap();
````

- [ ] **Step 6: Render, and look**

Run: `~/.cargo/bin/cargo test --locked --features shots --lib shots -- --ignored`

`target/shots/values-scrolled-macos-light.png`: `id` at the left with its line, `kind` half under it, the last column's cell selected and in view. `values-scrolled-omarchy-dark.png` for the terminal look, its status line saying which columns show.

Put right what differs, in the code this task wrote; keep the tests.

- [ ] **Step 7: Format and lint**

```bash
~/.cargo/bin/cargo fmt --all --check
~/.cargo/bin/cargo clippy --locked --workspace --all-targets -- -D warnings
~/.cargo/bin/cargo clippy --locked --workspace --all-targets --features shots -- -D warnings
```

Expected: no output.

- [ ] **Step 8: Commit point**

Stop and report. If the user has asked for commits:

```bash
git add -A src AGENTS.md
git commit -S -m "Keep the key column in sight while the others scroll" -m "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

### Task 16: Save a binary value to a file

**Files:**
- Modify: `src/ui/format.rs` (`save_name`)
- Modify: `src/model.rs`, `src/app.rs` (`Action::SaveValue`)
- Modify: `src/backend.rs` (`save_bytes`)
- Modify: `src/ui/row_panel.rs`

Under a binary value's hex box, "Save to file…". The view pushes `Action::SaveValue`; the app finds the bytes (the page may have changed since the frame that drew the link: then there is nothing to save) and hands them to the backend, which opens the system's save dialog and writes the file off the UI thread, as the other file dialogs do. A write that fails comes back as the `Event::Saved` the app already turns into a notice. In tests the backend records what it was asked to save (`Backend::saves`).

- [ ] **Step 1: Write the tests**

New in `src/ui/format.rs`: `a_saved_value_is_named_for_where_it_came_from_and_what_it_looks_like`. New in `src/ui/row_panel.rs`: `a_binary_value_is_saved_whole_under_a_name_of_its_own`, `a_value_that_is_gone_is_not_saved`.

````diff
diff --git a/src/ui/format.rs b/src/ui/format.rs
--- a/src/ui/format.rs
+++ b/src/ui/format.rs
@@ -1097,6 +1128,21 @@ mod tests {
         assert_eq!(sniff(b"plain text"), None);
     }
 
+    #[test]
+    fn a_saved_value_is_named_for_where_it_came_from_and_what_it_looks_like() {
+        assert_eq!(
+            save_name("book_covers", "image", &[0xff, 0xd8, 0xff]),
+            "book_covers-image.jpg"
+        );
+        assert_eq!(save_name("files", "body", b"%PDF-1.7"), "files-body.pdf");
+        assert_eq!(save_name("files", "body", b"\x00\x01"), "files-body.bin");
+        // A name from the server never leads out of the folder chosen.
+        assert_eq!(
+            save_name("../../etc", "pass wd\u{202E}", &[]),
+            "______etc-pass_wd_.bin"
+        );
+    }
+
     #[test]
     fn a_binary_cell_says_its_type_and_size() {
         assert_eq!(binary_label("bytea", 49_358, false), "bytea · 48.2 KB");
diff --git a/src/ui/row_panel.rs b/src/ui/row_panel.rs
--- a/src/ui/row_panel.rs
+++ b/src/ui/row_panel.rs
@@ -1669,6 +1683,35 @@ mod tests {
         }
     }
 
+    #[test]
+    fn a_binary_value_is_saved_whole_under_a_name_of_its_own() {
+        for look in Look::ALL {
+            let mut harness = awkward(look);
+            let link = look.label("Save to file…");
+            harness.click(&link);
+            // The dialog is the system's; the backend was asked with the
+            // whole value and a name that says what it is.
+            assert_eq!(
+                harness.app.backend.saves,
+                [("editions-cover.jpg".to_owned(), 100)],
+                "{}",
+                look.name
+            );
+        }
+    }
+
+    #[test]
+    fn a_value_that_is_gone_is_not_saved() {
+        let mut harness = awkward(Look::macos());
+        let tab = harness.app.active_tab_id();
+        let id = harness.app.workspace(tab).unwrap().active_tab.unwrap();
+        // A row the page no longer has, and a column that holds text.
+        for (row, col) in [(7, 3), (0, 0)] {
+            harness.app.apply(Action::SaveValue { tab, id, row, col });
+        }
+        assert!(harness.app.backend.saves.is_empty());
+    }
+
     #[test]
     fn an_array_too_long_to_read_every_frame_is_text() {
         let long = format!("{{{}}}", vec!["element"; 20_000].join(","));
````

- [ ] **Step 2: Run them and see them fail**

Run: `~/.cargo/bin/cargo test --locked --lib`

Expected, before the code below is there:

````text
Does not compile:
error[E0425]: cannot find function `save_name` in this scope
error[E0609]: no field `saves` on type `backend::Backend`
error[E0599]: no variant named `SaveValue` found for enum `model::Action`
````

- [ ] **Step 3: Write the code**

````diff
diff --git a/src/app.rs b/src/app.rs
--- a/src/app.rs
+++ b/src/app.rs
@@ -657,6 +657,34 @@ impl App {
                     self.fetch_rows(tab, object_tab);
                 }
             }
+            Action::SaveValue { tab, id, row, col } => {
+                // The value the row panel showed: it may be gone by now (a
+                // refresh, another page), and then there is nothing to save.
+                let found = self.workspace(tab).and_then(|workspace| {
+                    let (table, columns, rows) = match workspace.tab(id)? {
+                        Tab::Object(object) => {
+                            let page = object.page()?;
+                            (
+                                object.object.name.as_str(),
+                                page.columns.as_slice(),
+                                page.rows.as_slice(),
+                            )
+                        }
+                        Tab::Sql(sql) => {
+                            let (columns, rows, _) = sql.shown_rows()?;
+                            ("query", columns, rows)
+                        }
+                    };
+                    let tabletist_db::Value::Bytes(bytes) = rows.get(row)?.get(col)? else {
+                        return None;
+                    };
+                    let name = crate::ui::format::save_name(table, &columns.get(col)?.name, bytes);
+                    Some((name, bytes.to_vec()))
+                });
+                if let Some((name, bytes)) = found {
+                    self.backend.save_bytes(name, bytes);
+                }
+            }
             Action::GridKeys(tab) => {
                 if let Some(workspace) = self.workspace_mut(tab) {
                     workspace.pane = Pane::Grid;
diff --git a/src/backend.rs b/src/backend.rs
--- a/src/backend.rs
+++ b/src/backend.rs
@@ -283,6 +283,9 @@ pub struct Backend {
     outbox: Outbox,
     #[cfg(test)]
     pub sent: Vec<Command>,
+    /// The names and sizes of the values a test asked to save.
+    #[cfg(test)]
+    pub saves: Vec<(String, usize)>,
     #[cfg(test)]
     watched: Watched,
 }
@@ -343,6 +346,8 @@ impl Backend {
             #[cfg(test)]
             sent: Vec::new(),
             #[cfg(test)]
+            saves: Vec::new(),
+            #[cfg(test)]
             watched,
         }
     }
@@ -362,6 +367,8 @@ impl Backend {
             #[cfg(test)]
             sent: Vec::new(),
             #[cfg(test)]
+            saves: Vec::new(),
+            #[cfg(test)]
             watched: Watched::default(),
         }
     }
@@ -437,6 +444,39 @@ impl Backend {
         });
     }
 
+    /// Asks where to save `bytes` (a binary value), suggesting `name`, and
+    /// writes them there off the UI thread. A dialog closed without a
+    /// choice saves nothing and says nothing; a write that fails is told as
+    /// any failed save is.
+    pub fn save_bytes(&mut self, name: String, bytes: Vec<u8>) {
+        #[cfg(test)]
+        self.saves.push((name.clone(), bytes.len()));
+        let Some(runtime) = &self.runtime else {
+            return;
+        };
+        let dialog = rfd::AsyncFileDialog::new()
+            .set_title("Save value")
+            .set_file_name(name)
+            .save_file();
+        let outbox = self.outbox.clone();
+        runtime.spawn(async move {
+            let Some(file) = dialog.await else {
+                return;
+            };
+            let path = file.path().to_path_buf();
+            let target = path.clone();
+            let written = tokio::task::spawn_blocking(move || std::fs::write(target, bytes)).await;
+            let result = match written {
+                Ok(result) => result.map_err(|error| error.to_string()),
+                Err(error) => Err(error.to_string()),
+            };
+            if let Err(error) = &result {
+                log::error!("could not save {}: {error}", path.display());
+            }
+            outbox.emit(Event::Saved { path, result });
+        });
+    }
+
     /// Reads the Host aliases in ~/.ssh/config off the UI thread.
     pub fn list_ssh_hosts(&mut self, request: RequestId) {
         let Some(runtime) = &self.runtime else {
diff --git a/src/model.rs b/src/model.rs
--- a/src/model.rs
+++ b/src/model.rs
@@ -208,6 +208,14 @@ pub enum Action {
     /// The keyboard came to a grid (the Tab key, a screen reader): the
     /// arrows move in it from now on. Selects nothing.
     GridKeys(ConnTabId),
+    /// Save the binary value in column `col` of row `row`, of an object
+    /// tab's page or a SQL editor's result, to a file the user names.
+    SaveValue {
+        tab: ConnTabId,
+        id: TabId,
+        row: usize,
+        col: usize,
+    },
     /// Select a cell of an object tab's page or a SQL editor's result.
     SelectCell {
         tab: ConnTabId,
diff --git a/src/ui/format.rs b/src/ui/format.rs
--- a/src/ui/format.rs
+++ b/src/ui/format.rs
@@ -544,6 +544,37 @@ pub fn sniff(bytes: &[u8]) -> Option<&'static str> {
         .map(|(_, name)| *name)
 }
 
+/// The name a binary value's file is offered under: its table and column,
+/// and the extension of what it looks like (`covers-image.jpg`), or `.bin`.
+/// Only letters, digits, `-` and `_` of the names are kept: they come from
+/// the server, and a file name must not lead anywhere.
+pub fn save_name(table: &str, column: &str, bytes: &[u8]) -> String {
+    let safe = |name: &str| -> String {
+        name.chars()
+            .take(64)
+            .map(|character| {
+                if character.is_alphanumeric() || matches!(character, '-' | '_') {
+                    character
+                } else {
+                    '_'
+                }
+            })
+            .collect()
+    };
+    let extension = match sniff(bytes) {
+        Some("JPEG") => "jpg",
+        Some("PNG") => "png",
+        Some("GIF") => "gif",
+        Some("PDF") => "pdf",
+        Some("ZIP") => "zip",
+        Some("gzip") => "gz",
+        Some("WebP") => "webp",
+        Some("SQLite") => "sqlite",
+        _ => "bin",
+    };
+    format!("{}-{}.{extension}", safe(table), safe(column))
+}
+
 /// What a grid cell says of a binary value: its type and its size, never
 /// its bytes. `bytea · 48.2 KB`, or the terminal's `bytea 48.2K`.
 pub fn binary_label(type_name: &str, bytes: usize, terminal: bool) -> String {
diff --git a/src/ui/row_panel.rs b/src/ui/row_panel.rs
--- a/src/ui/row_panel.rs
+++ b/src/ui/row_panel.rs
@@ -968,7 +968,14 @@ fn field(
     let say = |text: &'static str| look.label(&gettext(locale, text));
     match value {
         Value::Bytes(bytes) => {
-            binary(ui, bytes, &formatted.short, look, palette, locale);
+            if binary(ui, bytes, &formatted.short, look, palette, locale) {
+                actions.push(Action::SaveValue {
+                    tab,
+                    id: tab_id,
+                    row,
+                    col,
+                });
+            }
             return;
         }
         Value::Text(text) => {
@@ -1210,8 +1217,9 @@ fn elements(
     });
 }
 
-/// A binary value: its first bytes, and how many more there are. The whole
-/// of it is a file's worth, which Copy gives.
+/// A binary value: its first bytes, how many more there are, and a link
+/// that saves the whole of it to a file (Copy gives it as hex). Returns
+/// whether the link was clicked.
 fn binary(
     ui: &mut egui::Ui,
     bytes: &[u8],
@@ -1219,7 +1227,7 @@ fn binary(
     look: &Look,
     palette: &Palette,
     locale: crate::i18n::Locale,
-) {
+) -> bool {
     let say = |text: &'static str| look.label(&gettext(locale, text));
     if bytes.is_empty() {
         Text::one(
@@ -1230,7 +1238,7 @@ fn binary(
         )
         .layout(ui.ctx())
         .label(ui);
-        return;
+        return false;
     }
     let role = TextRole::pick(look, TextRole::Json, TextRole::OSecondary);
     let more = bytes.len().saturating_sub(format::HEX_PREVIEW);
@@ -1273,6 +1281,12 @@ fn binary(
                 lines(ui);
             });
     }
+    let save = say("Save to file…");
+    let link = Text::one(look, widgets::secondary(look), &save, palette.accent)
+        .layout(ui.ctx())
+        .label_sense(ui, Sense::click());
+    link.widget_info(|| WidgetInfo::labeled(WidgetType::Link, true, &save));
+    link.clicked()
 }
 
 /// `books` becomes `book`, for "Open book →".
````

- [ ] **Step 4: Run the tests**

Run: `~/.cargo/bin/cargo test --locked --lib ui::`
Expected: PASS.

Then all of the library's: `~/.cargo/bin/cargo test --locked --lib`
Expected: `test result: ok. 888 passed; 0 failed; 1 ignored`.

- [ ] **Step 5: Render, and look**

Run: `~/.cargo/bin/cargo test --locked --features shots --lib shots -- --ignored`

`target/shots/mock-macos-values.png`: the link under the hex box. Then once by hand, since the dialog is the system's: run the app (`~/.cargo/bin/cargo run`), open a table with a binary column, select a row, click "Save to file…", save, and check the file's size is the value's.

Put right what differs, in the code this task wrote; keep the tests.

- [ ] **Step 6: Format and lint**

```bash
~/.cargo/bin/cargo fmt --all --check
~/.cargo/bin/cargo clippy --locked --workspace --all-targets -- -D warnings
~/.cargo/bin/cargo clippy --locked --workspace --all-targets --features shots -- -D warnings
```

Expected: no output.

- [ ] **Step 7: Commit point**

Stop and report. If the user has asked for commits:

```bash
git add -A src AGENTS.md
git commit -S -m "Save a binary value to a file" -m "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

### Task 17: The agent guide, and the checks

**Files:**
- Modify: `AGENTS.md`

`AGENTS.md` gains the rule this plan introduces, next to the one about text roles: focus is drawn in one place. Then the full checks, and a last look at every picture.

- [ ] **Step 1: Write the code**

````diff
diff --git a/AGENTS.md b/AGENTS.md
--- a/AGENTS.md
+++ b/AGENTS.md
@@ -43,6 +43,11 @@ The native keyring test is `#[ignore]`d; run it by hand on a desktop session.
 Views draw text only through `TextRole`s (`src/typography.rs`); a view never
 names a font, a family or a size.
 
+Keyboard focus is drawn in one place, `src/ui/focus.rs`, for whatever has the
+keyboard. A view never paints a control's focus ring: a widget that needs
+another form than the default says so with `focus::hint`. Only a pane (the
+tree, a grid) marks where its arrows are by itself: its cursor, its cell.
+
 Add a focused regression test for every behaviour change. UI behaviour is
 tested headlessly through `src/testing.rs` (AccessKit tree + events). Do not
 weaken a lint, delete a test, or add an `allow` to make CI green without
````

- [ ] **Step 2: Render, and look**

Run: `~/.cargo/bin/cargo test --locked --features shots --lib shots -- --ignored`

Every scene listed under "Ground rules", in the three looks and both palettes, beside the three artboards and panes 5 and 6 of the Omarchy one. Also the scenes that were there before (`workspace-*`, `picker-*`, `dialog-*`, `sql-*`, `state-*`): nothing should have moved in the light palette, apart from booleans being tags and NULL a chip.

Put right what differs, in the code this task wrote; keep the tests.

- [ ] **Step 3: Run every check**

```bash
~/.cargo/bin/cargo fmt --all --check
~/.cargo/bin/cargo clippy --locked --workspace --all-targets -- -D warnings
~/.cargo/bin/cargo clippy --locked --workspace --all-targets --features shots -- -D warnings
~/.cargo/bin/cargo test --locked --workspace --all-targets
RUSTDOCFLAGS='-D warnings' ~/.cargo/bin/cargo doc --locked --workspace --no-deps
```

Expected: no output from the first three and the last; every `test result` line `ok`.

- [ ] **Step 4: Commit point**

Stop and report. If the user has asked for commits:

```bash
git add -A src AGENTS.md
git commit -S -m "Say in the agent guide who draws focus" -m "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

## What is left for later

Written down so nobody takes their absence for an oversight (see Decisions):

- The column chooser ("Columns 10 / 40"), and moving the view by a column (⌥→).
- Digit grouping of amounts (`1,240.00`).
- JSON in the row panel: big documents opening folded, "… 42 more keys", "over 1 MB open collapsed; over 10 MB show size only", and a label that counts keys and bytes. The cell's preview of a document as its first key (`"awards": […`).
- A viewer for a long value ("open in viewer"), and the terminal's `[enter] view all` and `[s] save` keys.
- Extending the selection with ⇧arrows: the grid selects one cell.
- Resizing a column from the keyboard: the handles are the pointer's.
- Bundling the glyphs of `␣` and `↵`, so they show without a system font that has them.
- The Tab key in the design's order (the row panel after the rows): see decision 24.
- Whole sentences for translators: the notes this plan adds are put together from words ("first", "shown", "of", "pinned"), as the app's other notes are. There are no catalogs yet.
