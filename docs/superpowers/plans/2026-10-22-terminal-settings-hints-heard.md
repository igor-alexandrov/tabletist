# The terminal Settings screen's hints, heard. Implementation Plan

**Goal:** The Omarchy (terminal) look's Settings screen paints each row's
hint in its third column (`src/ui/settings/terminal.rs`, `hint`,
`option_row`, `text_row`), but a screen reader never hears it. PR #177
described the terminal look's choices by macOS's small print
(`super::helper`), which is not what this screen writes: "1 · 1.5 · 2
lines", "second header line", "default; t toggles", "PROD always tinted",
"now: jetbrains mono, bundled", "“Fira Code” not found · now: …" are never
read. Make each row's controls carry the hint it paints as their AccessKit
description, and announce a warning when it appears.

**Tech stack:** Rust 2024, egui 0.36 (the fork) with AccessKit. No new
dependency. One pull request.

## What changes

1. **The description is the painted hint.** `option_row` lays its hint out
   before anything else and keeps its text. That text, where it is not
   empty, describes the row's button and every choice the row draws: the
   radio buttons of `segments` and `check` (through `choice`) and the
   stepper's two buttons. `Cell` carries it to them (`choice` takes the
   `Cell` in place of its index, so it keeps its seven arguments).
   `describe` takes the words, not the option: `super::helper` is no
   longer read here (macOS's sheet still reads it).
   - The words are the hint's as painted, in the look's case: "Keep the
     hint's text the one painted". A key in a hint ("default; t toggles")
     is the keymap's, as painted.
   - A row whose hint is empty (the zebra, the gutter's switches, the
     timeout) has no description, as before.
   - A hint that is left out for want of room is still the row's
     description: what it says is still true, and nothing else says it.
2. **Rows that are no option** (`text_row`, the Agents pane's status,
   access and other agents). The row's button is described by its hint.
   "Cursor, others" has no button (the cursor never stops on it): its
   hint is announced as a label beside its value, as the value is,
   painted or not (the command it follows can take its room).
3. **Warnings are announced when they appear.** The font's "“{name}” not
   found · now: …" and the theme's "“{name}” not found · following …" are
   the rows' warnings (painted in `palette.warning`). While one stands, it
   is a polite live region too (`widgets::announce_polite`, over a point
   at the hint's left, as the data view says its counts), so a screen
   reader reads it out as it appears. Its node's id is its words, so it is
   read again only when they change.

## Tests (headless, AccessKit tree)

In `src/ui/settings_access_tests.rs`:

- `editor_settings_tests.rs`'s run scope test expects the terminal
  look's hint, "a selection always wins", in place of macOS's words.
- `every_rows_controls_are_described_by_its_small_print` becomes macOS
  only: the terminal look's description is no longer the small print, by
  design. The next tests cover that look. Its controls are now found by
  their names with the keys written in, so the run scope's ("{run}
  runs") is checked too, by the words beside its control.
- One test per pane in the Omarchy look (General, Appearance, Editor,
  Agents): every control the pane's option rows have is described by the
  hint the row paints, a table of option → words, and a row with no hint
  has no description. Every option the pane shows is in its table, so a
  row added later fails until it says what it is described by.
- Agents: the status and access rows' buttons are described by their
  hints; "Cursor, others"'s hint is a label.
Beside the tests that already paint each warning:

- `text_settings_tests.rs`: a missing font's row is described by the
  warning and a polite live node says it; found, the row is described by
  what fontconfig drew and nothing is read out.
- `theme_settings_tests.rs`: a pin found nowhere describes the theme's
  row and both its choices, and is read out; followed again, it is not.

## Checks

    cargo fmt --all --check
    cargo clippy --locked --workspace --all-targets -- -D warnings
    cargo test --locked --workspace --all-targets
    RUSTDOCFLAGS='-D warnings' cargo doc --locked --workspace --no-deps

Linux only; the change is in look-independent egui code, so macOS and
Windows are only compiled by CI.

## Departures

None from the brief.
