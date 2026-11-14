# Interface scale (issue #286, part 1) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use rubypowers:subagent-driven-development (recommended) or rubypowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace macOS's and Windows's **Interface size** (Small 12,
Default 13, Large 14) with an **Interface scale** that zooms the whole
window from 90% to 250%, step it with ⌘= ⌘- ⌘0 (Ctrl on Windows and
Linux) and View › Larger, Smaller, Actual Size, and widen the Omarchy
look's one font size from 9–24 to 9–32 with `ctrl+0` back to 13.

**Architecture:** The core's `settings` gains `UiScale` (percent, one of
eight) as `appearance.ui_scale`, which replaces the key
`appearance.interface_size` the way `editor.run.limit` replaced
`editor.sql_limit` (`Key::replaced`); `Loaded::migrated` names the keys
read from their old key alone, which the app writes once at its start.
`App::zoom_factor` returns the scale on macOS and Windows (the terminal
look keeps its font size over 13), and `App::logic` hands it to egui's
`set_zoom_factor`, which multiplies the display's own pixels per point.
The interface's text roles keep the design's sizes (`Faces` loses its
interface size); the data roles keep the data font size, which the zoom
multiplies. The size keys (`LargerText`, `SmallerText`,
`DefaultTextSize`, ids kept) step the scale or the terminal look's size
through one `App::resize`, which says the result in the toast or the
status line.

**Tech Stack:** Rust, egui 0.36 (crmne fork) with AccessKit; headless
tests through `src/testing.rs`; Jekyll for the website.

**Spec:** the issue #286 prompt (Interface scale, part 1), with the
boards `macOS – Settings, Appearance`, `macOS – Interface scale` and
`Omarchy – Settings, Appearance` of the design canvas, read again on
2026-11-08.

---

## Before Task 1

```bash
git add docs/superpowers/plans/2026-11-08-interface-scale.md
git commit -m "Plan the interface scale (#286, part 1)"
```

## Where this sits

One pull request, linking #286, as the prompt asks. Base: `main` at
`a7eb9dc2` (0.5.0). The whole run was built and checked first; each
task below is a state of that draft that builds and passes on its own
(kept in the session's scratchpad as `states/s1`, `s2`, `s3`).

## Decided while building

Where the prompt, the boards and the code disagree. To put to Igor in
the pull request:

| # | The prompt or board | Built | Why, and the other way |
|---|---|---|---|
| B1 | Migrate `appearance.ui_size` `12` / `13` / `14` | Migrate `appearance.interface_size` `"small"` / `"default"` / `"large"` to `0.9` / `1.0` / `1.1` | No version wrote `ui_size`: the file has always said `interface_size`. Written as `ui_scale` at the first start that reads it, the old line taken out; `"default"` leaves no line, as every default |
| B2 | An invalid `ui_scale` falls back to 1.0 and is reported like other invalid settings | At a start: 1.0, its line reported. While the file is edited live: the line reported and the scale kept as it was | Every key keeps its last good value on a live edit (`settings-and-files.md`); the scale is no exception. The other way: 100% on a bad edit too |
| B3 | The keys "in the Global scope" | They stay in the keymap's `ANYWHERE` (Global, and under a dialog) | The prompt also asks them to work in Settings, which is a dialog; Global alone would not reach it |
| B4 | View › Larger, Smaller, Actual Size | Those names; the commands keep their ids (`larger_text`, `smaller_text`, `default_text_size`) and are titled Larger, Smaller, Actual size | A `[keys]` line someone wrote for them keeps working |
| B5 | Board: `⌘= larger · ⌘- smaller · ⌘0 back to 100%` beside the menu | As drawn on macOS. Windows's Ctrl words leave no room in the sheet's row (291 points for 211): `Ctrl+= / Ctrl+- / Ctrl+0` there | A row's hint is drawn whole or not at all |
| B6 | Board: the menu 210 wide | 200 | At 210 the macOS hint is 4 points short of the row's end, and would not show |
| B7 | Board: the menu's `12 px text` notes in a column | Each note follows its percentage | `menu::Item::note` is the menu's way to add words; a column would be a new kind of entry |
| B8 | The toast `Interface 150% · 20 px text · ⌘0 resets` (the board has no dot before `⌘0`) | The prompt's words, the key from the keymap (`Ctrl+0` on Windows) | The prompt wins |
| B9 | Omarchy: the status line says `font size 18 · ctrl+0 resets` | Said wherever the window is: the connections screen and a connection's start screen now show what the toast would say in place of their note (`secrets: keyring`, `no table open`) for its 4 s | Those two lines never showed it, so the keys that work "anywhere" were silent there |
| B10 | The dropdown read as "Interface scale", its options "125 percent, 16 pixel text" | So, and the closed menu's value reads the same words | The shown `125% · 16 px text` would be read as symbols |
| B11 | Snap to whole physical pixels where egui allows | egui already rounds text, rects and line segments to physical pixels (its tessellation options, on by default); a test keeps them on. A 1-point line is 1.25 px wide at 125% | egui has no hairline that stays one pixel under a zoom |
| B12 | Bounds: never refuse a scale | Nothing refuses: the window's minimum (720 × 480 pixels) stays as it is, and at 250% it is 288 × 192 points, where the panels scroll | |
| B13 | (not in the prompt) | Three fixes the new layout checks found: Settings › Editor's history menu cut `Last 1,000 queries` short at every scale (a choice menu is now as wide as its widest choice); Tab coming round to the Theme tiles left them scrolled out of sight; the grid's selected row left the view when the grid's room shrank | The prompt's accessibility points ask for both of the last two |
| B14 | Layout checks "without pixels" | Two checks on egui's output: no piece of text wider or taller than the place it is clipped to, where that place ends inside what holds it (a cut at a scrolling pane's or the window's edge is a scroll's); and no control's own label drawn past the control's AccessKit bounds. Each view is scrolled through with the wheel. Window: 1920 × 1080 pixels, 768 × 432 points at 250%, 780 × 439 at Omarchy 32 | The Omarchy screens scroll by their cursor, not the wheel, and paint every row: their first view holds them all |

Open questions, with the prompt's defaults: **250% is the top step**
(33 px text, the reporter's 32 covered); **the size keys move from the
data font to the interface scale**, and the data font size has none;
**Omarchy's size goes to 32**.

How the zoom combines with the system's scale: egui draws at
`native_pixels_per_point × zoom_factor`. A Retina Mac (2 pixels a point)
at 125% draws 2.5 pixels a point; the issue's 1920 × 1080 laptop at 100%
system scale and 150% draws 1.5. Layouts are laid out in points, of which
the window has fewer as the zoom grows: they keep their shape and scroll.

## Files

| File | Task | What |
|---|---|---|
| `crates/tabletist-core/src/settings.rs` | 1 | `UiScale`, `Key::UiScale` replacing `Key::OldInterfaceSize`, `OptionId::InterfaceScale`, `OptionValue::InterfaceScale`, `Loaded::migrated`, the terminal look's sizes to 32, the notes `0.9 \| … \| 2.5` and (Linux) `9 to 32` |
| `crates/tabletist-core/src/theme.rs`, `src/typography.rs` | 1 | `Faces` without the interface's size; the interface's roles at the design's sizes |
| `crates/tabletist-core/src/keymap.rs`, `model.rs` | 1 | `ctrl+0` for `DefaultTextSize` in the Omarchy column (out of `ANY_LOOK`); the three titles |
| `src/app.rs`, `src/app/commands.rs`, `src/menu.rs` | 1 | `zoom_factor`, `resize` and its words, the migration at the start; quick open's rows; the View menu's names |
| `src/ui/settings/mod.rs`, `sheet.rs`, `terminal.rs` | 1 | The Interface scale menu and its hint, the data font's words; Omarchy's `9 to 32` and its hint |
| `src/ui/picker.rs`, `src/ui/start.rs` | 1 | The terminal look's said, on those two lines |
| `src/ui/text_settings_tests.rs`, `preview_settings_tests.rs`, `quick_open_tests.rs`, `settings_access_tests.rs` | 1 | The tests that named Interface size and the data size keys, and the new ones |
| `src/testing.rs`, `src/ui/scale_layout_tests.rs`, `src/ui/mod.rs` | 2 | Each text's clip rect; the largest-window checks |
| `src/ui/grid.rs`, `src/ui/settings/sheet.rs` | 2 | The selected row kept in view; the tiles scrolled to; choice menus as wide as their choices |
| `docs/_reference/settings-and-files.md`, `keyboard-shortcuts.md`, `docs/_guide/macos.md`, `omarchy.md` | 3 | The setting, the key, the migration, the keys |

Every commit ends on `~/.cargo/bin/cargo fmt --all --check`,
`~/.cargo/bin/cargo clippy --locked --workspace --all-targets -- -D warnings`,
`~/.cargo/bin/cargo test --locked --workspace --all-targets` and
`RUSTDOCFLAGS='-D warnings' ~/.cargo/bin/cargo doc --locked --workspace
--no-deps`; Task 3 also on `cd docs && bin/check`.

---

### Task 1: The interface scale, Omarchy's 9 to 32, and the keys

**Files:** every file of Task 1 in the table.

`UiScale` holds a percent: `ALL` (90, 100, 110, 125, 150, 175, 200,
250), `factor()`, `text_px()` (13 × the scale to the nearest pixel, in
integers: 12, 13, 14, 16, 20, 23, 26, 33), `name()` as the file writes it,
`of(f64)` (one of the eight or none), `stepped(larger)` (stopping at the
ends). The Settings sheet draws a `menu::Dropdown` named Interface scale
over `100% · 13 px text` with an entry a scale, its note `13 px text`,
`default` at 100%; its hint is the three keys from the keymap. `App::resize`
(the keys, the View menu and quick open) changes the scale or the terminal
look's size with `change_settings` (saved at once) and sets `app.said`;
a change made in Settings goes through `SetOption` and says nothing.

- [ ] **Step 1: Write the tests.** `settings.rs`:
  `the_interface_scale_steps_through_the_menu_and_stops_at_its_ends`
  (each zoom factor and `text_px`, against `(13 × scale).round()`),
  `the_interface_scale_is_one_of_the_menus_and_reads_back_as_written`
  (each scale round trip; `1.3`, `3`, `"large"`, `nan` ignored by their
  line at 1.0), `an_older_interface_size_is_read_as_its_scale_and_goes_when_written`
  (12, 13, 14 as small, default, large; `migrated`; the old line gone),
  and the font size range to 32. `app.rs`:
  `an_older_interface_size_is_written_as_its_scale_once_at_the_start`,
  `the_interface_scale_reloads_live_and_one_it_cannot_be_is_told`.
  `typography.rs`: `the_data_is_sized_apart_and_the_interface_keeps_the_designs_sizes`.
  `text_settings_tests.rs`: `each_interface_scale_sets_the_zoom_factor_to_its_own`
  (and on a display of 2 pixels a point), `the_data_font_size_multiplies_with_the_scale`
  (12.5 at 150% is 18.75), `the_mac_keys_step_through_the_scales_stop_at_the_ends_and_write_it`
  (macOS's Cmd, Windows's Ctrl), `a_key_that_changes_the_scale_says_so_in_the_toast`,
  `the_scale_keys_work_under_the_settings_window_which_shows_it_unsaid`,
  `the_terminal_keys_step_the_size_anywhere_and_stop_at_its_ends` (9, 32,
  `ctrl+0`), `a_terminal_key_that_changes_the_size_says_so_on_the_status_line`,
  `the_sheet_sets_the_interface_scale_and_the_data_font`,
  `changing_the_scale_keeps_the_keyboard_where_it_was`; the quick open,
  preview and access tests that named the old size and words.
- [ ] **Step 2: Run them, and see them fail.**
  Run: `~/.cargo/bin/cargo test --locked --workspace --all-targets`
  Expected: the core's tests fail to compile, 39 errors, led by
  `error[E0433]: cannot find type `UiScale` in this scope`,
  `error[E0609]: no field `ui_scale` on type `settings::Settings`` and
  `error[E0599]: no method named `migrated` found for struct `settings::Loaded``.
- [ ] **Step 3: Write the code** (state `s1`).
- [ ] **Step 4: Run the checks.** Expected: fmt and clippy clean; `tabletist` 2868 passed, 16
  ignored; `tabletist-core` 1043 passed, 1 ignored; every other suite as
  on `main`; the docs build with `-D warnings`.
- [ ] **Step 5: Commit.**
  `git commit -m "Scale the whole window by an interface scale, and give Omarchy's text 9 to 32"`

### Task 2: Nothing is cut off at the largest

**Files:** `src/testing.rs`, `src/ui/scale_layout_tests.rs`,
`src/ui/mod.rs`, `src/ui/grid.rs`, `src/ui/settings/sheet.rs`.

`Harness::clipped_text` keeps each painted text with its clip rect. The
checks (B14) run the table view and its sidebar, every Settings pane, the
connection dialog and the export sheet, in each look, at 250% and at
Omarchy's 32; Tab walks the Appearance pane at 250% with every control it
reaches in sight; the grid's selected row stays in sight when the scale
grows. The grid reveals its selection when its room changes size
(`grid::show`); the Theme tiles scroll into view when they gain the
keyboard; `choice_menu` is as wide as its widest choice.

- [ ] **Step 1: Write the tests** (`src/testing.rs`'s `clipped_text`,
  `scale_layout_tests.rs` and its `mod` line).
- [ ] **Step 2: Run them, and see them fail.**
  Run: `~/.cargo/bin/cargo test --locked -p tabletist --lib scale_layout_tests`
  Expected: 6 passed, 4 failed:
  `every_mac_settings_pane_cuts_nothing_off_at_250_percent` and
  `every_windows_settings_pane_cuts_nothing_off_at_250_percent`
  (`macos Editor, scrolled 1 times: cut off: "Last 1,000 queries" …`),
  `tab_reaches_every_control_of_the_settings_in_sight_at_250_percent`
  (`"System" has the keyboard at [[278.0 -271.6] - [410.0 -164.7]]`),
  `the_selected_row_stays_in_sight_when_the_scale_changes`
  (`row 20 gone`).
- [ ] **Step 3: Write the code** (state `s2`).
- [ ] **Step 4: Run the checks.** Expected: fmt and clippy clean; `tabletist` 2878 passed, 16
  ignored (the ten new ones in under 10 s); the docs build.
- [ ] **Step 5: Commit.**
  `git commit -m "Keep everything in reach at the largest scale: the selected row, the theme tiles, a menu's longest choice"`

### Task 3: The website

**Files:** `docs/_reference/settings-and-files.md`,
`docs/_reference/keyboard-shortcuts.md`, `docs/_guide/macos.md`,
`docs/_guide/omarchy.md`.

- [ ] **Step 1: Write the pages** (state `s3`): the Interface scale's
  row, its key in the file and the migration, the keys' new meaning on
  macOS and Windows, Omarchy's 9 to 32 and `ctrl+0`.
- [ ] **Step 2: Check them.** `cd docs && bin/check`. Expected:
  `HTML-Proofer finished successfully.`
- [ ] **Step 3: Commit.**
  `git commit -m "Say the interface scale and its keys on the website"`

### After Task 3

- [ ] Push, and open the pull request linking #286: the summary, how the
  zoom combines with the system's scale, the migration, the keys and the
  keymap check (the keymap's generated tests: no two commands share a
  key where both are read; `ctrl+0` was free in every Omarchy scope), the
  open questions and their defaults, and B1 to B14. No screenshot.
