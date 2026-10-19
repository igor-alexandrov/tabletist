# Settings: accessible controls. Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Every Settings pane's controls are heard as the Appearance prompt asks, in both looks: a segmented control is a radio group named by its option, its segments radio buttons, and every control's AccessKit description is its row's small print (the helper text under its label).

**Architecture:** The shared widgets take what a screen reader should hear of the control as a whole. `widgets::segmented` takes an `Option<widgets::Group>` (the group's name and description): with one, its segments are drawn in a `Ui` of their own whose node is a `RadioGroup` with that name; with or without one, each segment is a `RadioButton`, marked chosen, and carries the description. `widgets::toggle` takes an `Option<&str>` description after its name, and `menu::Dropdown` a `.description(Option<&str>)` builder. `sheet::row` works out the small print it writes (or the missing font's warning, which stands in its place) and hands it to `sheet::control`, which passes it to whichever widget the row has. The Agents pane's own controls (the server's switch, the agent's segments) pass the words written beside them. In the terminal look, `terminal::describe` gives the option's small print (in the case of the names, not the look's lower case) to each choice and to the row's button, and the two-value rows (`segments`, `check`) put their radio buttons in a `RadioGroup` named by the option.

**Tech Stack:** Rust 2024, egui 0.36 (the crmne fork) with AccessKit; tests through `src/testing.rs`.

---

## Where this sits

Split off from Settings › Appearance step 1 (`2026-10-17-settings-appearance-1-grid.md`), which listed both points as departures ("Every control's description is its helper text", "Segmented controls are radio groups") because they change widgets the General and Agents panes share. The Appearance pane is on `main` (steps 1 to 5), so its rows are covered here with General's and Agents'. This plan is a branch of `main` at `e2f2eaf`.

## What this run settles

| Question | Decision | Why |
|---|---|---|
| Where the description sits on a segmented control | On each segment (the radio button the keyboard is on) | VoiceOver and Orca read the help of the focused element; a group's is not read when a button in it takes focus |
| The data font's row when the font is not installed | Its description is the warning the row prints in place of the small print | The description is what the row says under its label; the warning is what it says then |
| The Font row, which has two menus (face and size) | Both menus carry the row's small print | Both are the row's controls, and "Grid cells, SQL editor, JSON" is true of both |
| Segmented controls outside Settings (the sidebar's tree and flat, the Agents window's Activity and Log, the Agents pane's agent) | Their segments become radio buttons too; a group only where the caller names one | One widget, one role. The sidebar and the Agents window pass no group: nothing beside them names the choice |
| The terminal look's row button | Gets the option's small print as its description, as its choices do | It is the control Tab and the cursor land on first |
| The Theme row (tiles on macOS, segments in the terminal look) | Unchanged on macOS (already a radio group); in the terminal look a radio group like the other two-value rows | It has no small print, so no description |

## Task 1: the shared widgets

**Files:** `src/ui/widgets.rs`, `src/ui/menu.rs`, callers in `src/ui/sidebar.rs`, `src/ui/agents.rs`, `src/ui/settings/agents.rs`, `src/ui/settings/sheet.rs`.

- [x] Add `pub struct Group<'a> { pub name: &'a str, pub description: Option<&'a str> }` beside `Segment`.
- [x] `segmented(ui, segments, selected, segment, group: Option<Group<'_>>, look, palette)`: the whole control drawn inside `ui.scope_builder` with the salt `"segmented"` (it allocates its own space, so the parent's layout is as before); with a group, the scope's node gets `Role::RadioGroup` and the name. Each segment: `WidgetInfo::selected(WidgetType::RadioButton, ...)`, then `set_description` where there is one.
- [x] `toggle(ui, on, name, description: Option<&str>, palette)`: the description on its node.
- [x] `Dropdown::description(Option<&'a str>)`; `show` sets it on the field's node. A caller's later `widget_info` keeps it (egui's fill does not clear a description).
- [x] Every existing caller passes `None` for now; `cargo check`.

## Task 2: the sheet's rows and the Agents pane

**Files:** `src/ui/settings/sheet.rs`, `src/ui/settings/agents.rs`.

- [x] `row` keeps the said small print (`String`) and passes `Option<&str>` to `control`.
- [x] `control` hands it to every dropdown (page size, zone, launch, idle, log days, font, size), every switch, and `segments`, which builds `Group { name: option, description }`.
- [x] `font_menu` and `size_menu` take the description.
- [x] Agents pane: `title`'s switch is described by the words under the title; `connect`'s segments are a group named "Connect an agent", described by the chosen agent's hint.

## Task 3: the terminal look

**Files:** `src/ui/settings/terminal.rs`.

- [x] `describe` sets `small_print(option)`, translated, as a button's description; `choice` calls it for every choice (values and steps).
- [x] `option_row` sets it on the row's button.
- [x] `segments` and `check` draw their choices in `ui.new_child` over the row, whose node is a `RadioGroup` named by `terminal_label(option)`; `new_child` leaves the parent's layout alone, which the terminal's rows (all painted at places, nothing allocated) need.

## Task 4: tests

**Files:** `src/ui/settings_access_tests.rs` (new), `src/ui/mod.rs` (the module).

- [x] For both looks and every pane: each option with small print has every control of its row (combo box, checkbox, radio button, or the terminal look's buttons) described by it, and no control of a row without small print has a description.
- [x] For both looks: every segmented option (Timestamps, Numbers, Interface size, Row height, Table list, Environment bar) is a `RadioGroup` named by the option whose children are `RadioButton`s, one of them chosen.
- [x] The Agents pane: "Connect an agent" is a radio group; the server's switch has the words under the title as its description.
- [x] The data font's row with a font not installed: both menus are described by the warning.
- [x] The terminal look's switch rows (Value tags) are radio groups too.
- [x] The sidebar's tree and flat switch and the Agents window's Activity and Log still switch, now as radio buttons: the existing tests, run whole.

## Task 5: checks and the pull request

- [x] `cargo fmt --all --check`, `cargo clippy --locked --workspace --all-targets -- -D warnings`, `cargo test --locked --workspace --all-targets`.
- [x] One pull request: a summary, the decisions above, platform coverage (built and tested on Linux; the change is platform-neutral egui code).
