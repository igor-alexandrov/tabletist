# The Type row as a radio grid: Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use rubypowers:subagent-driven-development (recommended) or rubypowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** The connection dialog's Type row holds eight types and more: a grid of radio buttons, four to a row on macOS and three on Omarchy, in place of the segmented track and the line of words that run out of room.

**Architecture:** One run, one PR. `choice.rs` gains `Group::Grid` and a `radio_grid` drawer beside `radio_group`; only `driver_choice` uses it, and the environment, SSL mode and SSH auth choices keep `radio_group` as they are. On macOS the grid is one Tab stop whose arrows choose (`focus::segment` learns ↑ and ↓ as `focus::grid_segment`); on Omarchy each type stays a Tab stop that Enter or Space picks. The macOS hint moves from beside the control to under it. Changing type is the same `Action::SetDriver` as today.

**Tech Stack:** Rust, egui 0.36 (accesskit roles, `EventFilter` focus locks), the app's `testing::Harness`.

**Base:** PR #247 (`claude/tabletist-files-connection-ba64f6`) is still open and this work needs its Files type. Branch from that PR's head; open the PR once #247 is merged, rebased onto main (never a merge of main).

---

## Why

Measured with the app's fonts and `radio_group`'s spacing (the options page: https://claude.ai/artifact/31TBrfeSPTzHV4vCd4VtX7, option B is the one picked):

| Types | macOS track (row is 660) | Omarchy words (column is about 576) |
|---|---|---|
| 5, today | 358 | 370 |
| 6, with Redis | 415 | 431 |
| 8 | 594 | 631, over by 55 |
| 9 | 679, over by 19 | 724, over by 148 |

Today every type is also its own Tab stop on macOS, and the Files hint (325 wide) already wraps under the five-type track.

## Departures from the canvas: all six agreed by Igor on 2026-11-17

The canvas (`https://claude.ai/artifact/4FtP3GnsXevWfpBsotjVb6`) draws Type as a segmented track (macOS) and boxed words (Omarchy) on every connection artboard: `MacConnectionEdit`, `MacConnectionEditNext`, `MacConnectionSqlite`, `MacConnectionFiles`, `MacConnectionRedis` and their Omarchy boards. The grid departs from them here:

1. **The Type row is a radio grid in both looks.** macOS uses the Components Radio (a 16 circle, the chosen one filled in the accent with a white dot) in rows 28 high and 2 apart, at most four columns 12 apart. Omarchy uses the Components Radio (`(•)` in the accent, `( )` muted) in rows 23 high, at most three columns 14 apart. The canvas has the radio, never a grid of them.
2. **The macOS hint moves under the grid,** 6 below it, where the canvas has it beside the track.
3. **macOS: Type becomes one Tab stop.** The chosen radio takes Tab, and ← → ↑ ↓ move and choose, as the segmented controls' arrows do. ↑ and ↓ are new keys. Today each type is a Tab stop.
4. **Omarchy: each type stays a Tab stop and Enter or Space picks,** as the canvas's choice row says. The keyboard's cursor is the Components Radio's cursor state: the selection's tone under the cell and the ▌ bar at its left (`Ring::Bar`), where the words today draw the outer ring.
5. **The Omarchy "type" label is centred on the grid's height,** as every Omarchy row centres its label. With six types it sits between the two rows.
6. **Fewer columns in a narrow window.** A column is never narrower than 150 (macOS) or 170 (Omarchy), so a dialog squeezed by a window under 748 (macOS) or 828 (Omarchy) wide drops to three or two columns.

Igor said yes to all six on 2026-11-17. A follow-up updates the canvas boards to match; the plan does not touch the canvas.

## Files

- Modify `src/ui/focus.rs`: `segment` becomes a call to the new `grid_segment`, which adds ↑ and ↓ when the choices are in rows.
- Modify `src/ui/connect_dialog/choice.rs`: `GridShape`, `Group::Grid`, `radio_grid`, `driver_height`; `driver_choice` uses the grid.
- Modify `src/ui/connect_dialog/sheet.rs`: the hint goes under the grid.
- Modify `src/ui/connect_dialog/terminal.rs`: the Type row is as tall as the grid.
- Modify `src/ui/files_tests.rs`: the keyboard tests and the renamed row test.
- Modify `docs/_reference/connections.md`: the Type row's line.

Commands below run from the worktree root with:

```bash
export CARGO_TARGET_DIR=$PWD/target
```

---

### Task 1: macOS, one Tab stop and the arrows across rows

**Files:**
- Modify: `src/ui/focus.rs:278-330`
- Modify: `src/ui/connect_dialog/choice.rs`
- Modify: `src/ui/connect_dialog/sheet.rs:306-330`
- Test: `src/ui/files_tests.rs`

- [ ] **Step 1: Write the failing test**

Add to `src/ui/files_tests.rs`, after `the_type_is_a_radio_group_of_four_on_a_row_of_its_own`:

```rust
/// The label of what has the keyboard.
fn focused_label(harness: &mut Harness) -> Option<String> {
    let id = harness.ctx.memory(|memory| memory.focused())?;
    let tree = harness.settle();
    tree.nodes
        .iter()
        .find(|(node, _)| *node == id.accesskit_id())
        .and_then(|(_, node)| node.label().map(str::to_owned))
}

/// macOS: the types are four to a row, one Tab stop: the chosen one. The
/// arrows choose across a row and down a column, and Tab leaves the grid.
#[test]
fn the_mac_type_grid_is_one_tab_stop_and_the_arrows_choose() {
    let mut harness = Harness::new();
    harness.set_look(Look::macos());
    // A new connection gives the name the keyboard on its first frame.
    harness.app.apply(Action::NewConnection);
    form_mut(&mut harness).driver = Driver::Postgres;
    harness.settle();
    // Back from the name is the chosen type.
    harness.press(Key::Tab, Modifiers::SHIFT);
    assert_eq!(focused_label(&mut harness).as_deref(), Some("PostgreSQL"));
    harness.press(Key::ArrowLeft, Modifiers::NONE);
    assert_eq!(form(&harness).driver, Driver::Sqlite);
    assert_eq!(focused_label(&mut harness).as_deref(), Some("SQLite"));
    // SQLite, PostgreSQL, MySQL, BigQuery on the first row; Files under SQLite.
    harness.press(Key::ArrowDown, Modifiers::NONE);
    assert_eq!(form(&harness).driver, Driver::Files);
    assert_eq!(focused_label(&mut harness).as_deref(), Some("Files"));
    // Nothing under Files: ↓ stays.
    harness.press(Key::ArrowDown, Modifiers::NONE);
    assert_eq!(form(&harness).driver, Driver::Files);
    harness.press(Key::ArrowUp, Modifiers::NONE);
    assert_eq!(form(&harness).driver, Driver::Sqlite);
    // One Tab leaves the grid: the arrows no longer choose.
    harness.press(Key::Tab, Modifiers::NONE);
    harness.press(Key::ArrowRight, Modifiers::NONE);
    assert_eq!(form(&harness).driver, Driver::Sqlite);
}
```

- [ ] **Step 2: Run it to see it fail**

Run: `~/.cargo/bin/cargo test -p tabletist the_mac_type_grid_is_one_tab_stop -- --nocapture`
Expected: FAIL. Today Shift+Tab from Name lands on Files (each type is its own Tab stop), so the second assertion fails with `left: Some("Files"), right: Some("PostgreSQL")`.

- [ ] **Step 3: `focus::grid_segment`**

Replace `segment` in `src/ui/focus.rs` (lines 278-330) with:

```rust
/// One of the `count` choices of a segmented control, the one at `index`:
/// the chosen one is the control's only Tab stop, and with the keyboard on
/// it ← and → choose its neighbours. Returns its response and the choice
/// the arrows made, if any; the keyboard follows that choice once the
/// caller has made it the chosen one.
pub fn segment(
    ui: &Ui,
    rect: Rect,
    id: Id,
    group: Id,
    (index, count): (usize, usize),
    chosen: bool,
) -> (Response, Option<usize>) {
    grid_segment(ui, rect, id, group, (index, count, count), chosen)
}

/// [`segment`] for choices `columns` to a row: ↑ and ↓ also choose the one
/// above and below. Where there is none, the choice stays.
pub fn grid_segment(
    ui: &Ui,
    rect: Rect,
    id: Id,
    group: Id,
    (index, count, columns): (usize, usize, usize),
    chosen: bool,
) -> (Response, Option<usize>) {
    let sense = if chosen { Sense::click() } else { Sense::CLICK };
    let response = ui.interact(rect, id, sense);
    let now = ui.ctx().cumulative_frame_nr();
    // The choice the arrows made, and when: taken up within two frames or
    // not at all (the caller may have refused it).
    let moved: Option<(usize, u64)> = ui.data(|data| data.get_temp(group));
    if let Some((target, when)) = moved
        && chosen
        && target == index
    {
        if now <= when + 2 {
            response.request_focus();
        }
        ui.data_mut(|data| data.remove::<(usize, u64)>(group));
    }
    let mut picked = None;
    if response.has_focus() {
        // A single row leaves ↑ and ↓ to whatever holds it.
        let rows = columns < count;
        let arrows = EventFilter {
            horizontal_arrows: true,
            vertical_arrows: rows,
            ..Default::default()
        };
        ui.memory_mut(|memory| memory.set_focus_lock_filter(response.id, arrows));
        let (right, left, down, up) = ui.input_mut(|input| {
            (
                input.consume_key(Modifiers::NONE, Key::ArrowRight),
                input.consume_key(Modifiers::NONE, Key::ArrowLeft),
                rows && input.consume_key(Modifiers::NONE, Key::ArrowDown),
                rows && input.consume_key(Modifiers::NONE, Key::ArrowUp),
            )
        });
        let next = match (right, left, down, up) {
            (true, false, _, _) => (index + 1).min(count.saturating_sub(1)),
            (false, true, _, _) => index.saturating_sub(1),
            (_, _, true, false) if index + columns < count => index + columns,
            (_, _, false, true) => index.checked_sub(columns).unwrap_or(index),
            _ => index,
        };
        if next != index {
            picked = Some(next);
            ui.data_mut(|data| data.insert_temp(group, (next, now)));
        }
    }
    (response, picked)
}
```

- [ ] **Step 4: The grid in `choice.rs`**

In `src/ui/connect_dialog/choice.rs`, add `Grid` to `Group` and give it a height of its own:

```rust
/// How a [`radio_group`] draws.
#[derive(Clone, Copy, PartialEq)]
pub(super) enum Group {
    /// macOS: a sunken track, the chosen segment raised off it. Segments
    /// are this tall.
    Track(f32),
    /// The terminal look's environments: bordered buttons in each one's
    /// colour, the chosen one filled.
    Buttons,
    /// The terminal look's other choices: words in a line, the chosen one
    /// boxed in the accent.
    Words,
    /// Radio buttons in rows across the width given (see [`radio_grid`]):
    /// the type, which grows past what a line holds.
    Grid,
}

impl Group {
    /// How tall the choices are. A grid's height depends on how many
    /// choices there are and the width: [`GridShape::height`].
    pub(super) fn height(self) -> f32 {
        match self {
            Self::Track(height) => height,
            Self::Buttons => 26.0,
            // A line of the look's text (17), 2 above and below it, and
            // the 1 pt box round the chosen word.
            Self::Words => 23.0,
            Self::Grid => 28.0,
        }
    }
}
```

After the `DOT_GAP` constant, add the grid's shape:

```rust
/// How a [`radio_grid`] lays out in a look: at most `columns` to a row,
/// none narrower than `min`, `gap` apart; rows `row` tall, `row_gap` apart.
struct GridShape {
    columns: usize,
    min: f32,
    gap: f32,
    row: f32,
    row_gap: f32,
}

/// macOS: the Components Radio, four to a row in the dialog's 660.
const MAC_GRID: GridShape = GridShape {
    columns: 4,
    min: 150.0,
    gap: 12.0,
    row: 28.0,
    row_gap: 2.0,
};

/// The terminal look: `(•) name`, three to a row in the value column's 576.
const TERMINAL_GRID: GridShape = GridShape {
    columns: 3,
    min: 170.0,
    gap: 14.0,
    row: 23.0,
    row_gap: 0.0,
};

impl GridShape {
    fn of(skin: &Skin) -> &'static GridShape {
        if skin.look.terminal {
            &TERMINAL_GRID
        } else {
            &MAC_GRID
        }
    }

    /// The columns `width` holds: fewer in a dialog a narrow window
    /// squeezes.
    fn columns(&self, width: f32) -> usize {
        (((width + self.gap) / (self.min + self.gap)).floor() as usize).clamp(1, self.columns)
    }

    /// How tall `count` choices stand across `width`.
    fn height(&self, count: usize, width: f32) -> f32 {
        let rows = count.div_ceil(self.columns(width)).max(1);
        rows as f32 * self.row + (rows - 1) as f32 * self.row_gap
    }
}

/// How tall the type's grid stands across `width`: the terminal row it
/// sits on is as tall.
pub(super) fn driver_height(width: f32, skin: &Skin) -> f32 {
    GridShape::of(skin).height(DRIVERS.len(), width)
}
```

In `choose`, send a grid to its own drawer. Replace the two calls of `radio_group(ui, salt, &choices, chosen, group, skin)` in `choose` with `draw(ui)`, defined before the `match`:

```rust
    let draw = |ui: &mut Ui| match group {
        Group::Grid => radio_grid(ui, salt, &choices, chosen, skin),
        Group::Track(_) | Group::Buttons | Group::Words => {
            radio_group(ui, salt, &choices, chosen, group, skin)
        }
    };
    let clicked = match name {
        None => draw(ui),
        // The choices are drawn in a Ui of their own, whose node is their
        // parent in the accessibility tree: a radio group with the name.
        Some(name) => {
            ui.scope_builder(egui::UiBuilder::new().id_salt(salt), |ui| {
                ui.ctx().accesskit_node_builder(ui.unique_id(), |node| {
                    node.set_role(egui::accesskit::Role::RadioGroup);
                    node.set_label(name);
                });
                draw(ui)
            })
            .inner
        }
    };
```

`radio_group` is never given `Group::Grid`: in each of its three `match group` (the padding, the corner, the colour) add the arm `Group::Grid => unreachable!("a grid is drawn by radio_grid"),`.

Add `radio_grid` after `radio_group`:

```rust
/// Choices as radio buttons in rows across the width the Ui has; returns
/// the one clicked or, on macOS, the one the arrows chose. Each is
/// announced as a radio button by its own text. On macOS the chosen one is
/// the grid's only Tab stop and the arrows move in the grid; in the
/// terminal look each is a Tab stop that Enter or Space picks.
fn radio_grid(
    ui: &mut Ui,
    salt: &str,
    choices: &[Choice<'_>],
    chosen: usize,
    skin: &Skin,
) -> Option<usize> {
    let Skin { look, palette, .. } = *skin;
    let shape = GridShape::of(skin);
    let width = ui.available_width();
    let columns = shape.columns(width);
    let cell_width = (width - shape.gap * (columns - 1) as f32) / columns as f32;
    let size = vec2(width, shape.height(choices.len(), width));
    let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
    let painter = ui.painter().clone();
    let enabled = ui.is_enabled();
    let group = ui.id().with((salt, "grid"));
    let mut clicked = None;
    for (index, choice) in choices.iter().enumerate() {
        let (row, column) = (index / columns, index % columns);
        let cell = Rect::from_min_size(
            pos2(
                rect.left() + column as f32 * (cell_width + shape.gap),
                rect.top() + row as f32 * (shape.row + shape.row_gap),
            ),
            vec2(cell_width, shape.row),
        );
        let active = index == chosen;
        let id = ui.id().with((salt, index));
        let (response, arrow) = if look.terminal {
            (ui.interact(cell, id, Sense::click()), None)
        } else {
            crate::ui::focus::grid_segment(
                ui,
                cell,
                id,
                group,
                (index, choices.len(), columns),
                active,
            )
        };
        let can = enabled && choice.off.is_none();
        response.widget_info(|| {
            WidgetInfo::selected(WidgetType::RadioButton, can, active, choice.label)
        });
        if can && response.clicked() {
            clicked = Some(index);
        }
        if can {
            clicked = arrow.or(clicked);
        }
        let center = cell.center().y;
        if look.terminal {
            // The Components Radio: (•) in the accent, ( ) muted; the
            // keyboard's cursor is the selection's tone and the ▌ bar.
            let cursor = crate::ui::focus::shown(&response);
            if cursor {
                painter.rect_filled(cell, CornerRadius::ZERO, palette.selection);
            }
            let (mark, mark_ink) = if active {
                ("(•) ", palette.accent)
            } else {
                ("( ) ", palette.dim)
            };
            let name_ink = if active || (can && response.hovered()) {
                palette.text
            } else {
                palette.dim
            };
            let role = TextRole::OBody;
            let x = cell.left() + 8.0;
            let x = x + Text::one(look, role, mark, mark_ink)
                .layout(ui.ctx())
                .paint_left(&painter, x, center);
            Text::one(look, role, choice.label, name_ink)
                .layout(ui.ctx())
                .paint_left(&painter, x, center);
            crate::ui::focus::hint(ui, &response, cell, crate::ui::focus::Ring::Bar);
        } else {
            // The Components Radio: a 16 circle, the chosen one filled in
            // the accent round a dot.
            let mark = Rect::from_center_size(pos2(cell.left() + 8.0, center), vec2(16.0, 16.0));
            if active {
                painter.circle_filled(mark.center(), 8.0, palette.accent);
                painter.circle_filled(mark.center(), 3.0, palette.on_accent);
            } else {
                let edge = if can && response.hovered() {
                    palette.border.lerp_to_gamma(palette.text, 0.2)
                } else {
                    palette.border
                };
                painter.circle_filled(mark.center(), 8.0, palette.window);
                painter.circle_stroke(mark.center(), 7.5, Stroke::new(1.0, edge));
            }
            crate::ui::focus::hint(
                ui,
                &response,
                mark,
                crate::ui::focus::Ring::Outer { radius: 8 },
            );
            let role = if active {
                TextRole::UiBodyStrong
            } else {
                TextRole::UiBody
            };
            let ink = if can { palette.text } else { palette.dim };
            Text::one(look, role, choice.label, ink)
                .layout(ui.ctx())
                .paint_left(&painter, cell.left() + 24.0, center);
        }
    }
    clicked
}
```

In `driver_choice`, draw the grid in both looks and fix its doc comment:

```rust
/// The driver choice: every type as a radio button in a grid, under
/// `heading`.
pub(super) fn driver_choice(
    ui: &mut Ui,
    form: &ConnectionForm,
    heading: &str,
    skin: &Skin,
    actions: &mut Vec<Action>,
) {
    let labels = DRIVERS.map(|driver| skin.look.label(driver.label()));
    let options: Vec<(Driver, Choice<'_>)> = DRIVERS
        .iter()
        .zip(&labels)
        .map(|(driver, label)| (*driver, Choice::plain(label)))
        .collect();
    let clicked = choose(
        ui,
        "driver",
        Some(heading),
        &options,
        form.driver,
        Group::Grid,
        skin,
    );
    if let Some(driver) = clicked
        && driver != form.driver
    {
        actions.push(Action::SetDriver(driver));
    }
}
```

- [ ] **Step 5: The macOS hint under the grid**

In `src/ui/connect_dialog/sheet.rs`, replace the doc comment of `identity` and the `ui.horizontal_wrapped` block that holds `driver_choice` (lines 306-330):

```rust
/// The kind of database as a grid of radio buttons on rows of their own,
/// with what it changes under them; then the name, then the environment.
```

```rust
    widgets::label(ui, TextRole::FormLabel, &heading, palette.secondary, look);
    ui.add_space(5.0);
    driver_choice(ui, form, &heading, skin, actions);
    ui.add_space(6.0);
    widgets::label(
        ui,
        TextRole::Secondary,
        &skin.say(super::type_hint(form.driver)),
        palette.dim,
        look,
    );
    ui.add_space(18.0);
```

- [ ] **Step 6: Run the test**

Run: `~/.cargo/bin/cargo test -p tabletist the_mac_type_grid_is_one_tab_stop -- --nocapture`
Expected: PASS.

- [ ] **Step 7: Run the segmented controls' tests, which share `focus::segment`**

Run: `~/.cargo/bin/cargo test -p tabletist segment`
Run: `~/.cargo/bin/cargo test -p tabletist files_tests`
Expected: PASS. ↑ and ↓ on a single row are not consumed, so the data view switch and the SQL editor's mode switch behave as before.

- [ ] **Step 8: Commit**

```bash
git add src/ui/focus.rs src/ui/connect_dialog/choice.rs src/ui/connect_dialog/sheet.rs src/ui/files_tests.rs
git commit -m "Draw the connection's type as a radio grid, one Tab stop on macOS"
```

On "agent refused operation", 1Password is locked: leave the changes staged, say so, and wait for Igor.

### Task 2: Omarchy, the grid's height and its keys

**Files:**
- Modify: `src/ui/connect_dialog/terminal.rs:320-326`
- Test: `src/ui/files_tests.rs`

- [ ] **Step 1: Write the test**

Add to `src/ui/files_tests.rs`:

```rust
/// Omarchy: every type is a Tab stop and Enter or Space picks it; the
/// arrows and h/l choose nothing.
#[test]
fn the_omarchy_type_grid_takes_tab_and_enter_or_space() {
    let look = Look::omarchy();
    let mut harness = Harness::new();
    harness.set_look(look);
    harness.app.apply(Action::NewConnection);
    form_mut(&mut harness).driver = Driver::Postgres;
    harness.settle();
    // A new connection gives the name the keyboard. Back from it: the last
    // type, then the one before it.
    harness.press(Key::Tab, Modifiers::SHIFT);
    assert_eq!(focused_label(&mut harness), Some(look.label("Files")));
    harness.press(Key::Tab, Modifiers::SHIFT);
    assert_eq!(focused_label(&mut harness), Some(look.label("BigQuery")));
    harness.press(Key::ArrowLeft, Modifiers::NONE);
    harness.press(Key::H, Modifiers::NONE);
    assert_eq!(form(&harness).driver, Driver::Postgres);
    harness.press(Key::Space, Modifiers::NONE);
    assert_eq!(form(&harness).driver, Driver::BigQuery);
    harness.press(Key::Tab, Modifiers::NONE);
    harness.press(Key::Enter, Modifiers::NONE);
    assert_eq!(form(&harness).driver, Driver::Files);
}
```

- [ ] **Step 2: Run it**

Run: `~/.cargo/bin/cargo test -p tabletist the_omarchy_type_grid_takes_tab -- --nocapture`
Expected: PASS already: the keys are today's. The test pins them through the change. What Step 3 fixes is layout, which no test pins (no design checks in tests): after Task 1 the Omarchy row is still `Group::Words.height()` (23) tall, so the grid's second row is drawn under Name's field.

- [ ] **Step 3: The row as tall as the grid**

In `src/ui/connect_dialog/terminal.rs`, import `driver_height` beside `driver_choice`:

```rust
use super::choice::{Choice, Group, choose, driver_choice, driver_height, environment_choice};
```

and replace the Type row (lines 320-326):

```rust
            // The type first: it changes every row after the name. Its
            // grid is as tall as its rows, in the value column's width.
            let heading = skin.say("Type");
            let height = driver_height(ui.available_width() - 164.0, skin);
            terminal_row_of(ui, &heading, height, skin, |ui, _| {
                driver_choice(ui, form, &heading, skin, actions);
            });
```

164 is `terminal_row_of`'s label column (150) and its gap (14).

- [ ] **Step 4: Run the tests**

Run: `~/.cargo/bin/cargo test -p tabletist the_omarchy_type_grid_takes_tab -- --nocapture`
Run: `~/.cargo/bin/cargo test -p tabletist files_tests`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/ui/connect_dialog/terminal.rs src/ui/files_tests.rs
git commit -m "Give the Omarchy type row the grid's height, and pin its keys"
```

### Task 3: The row test, the reference

**Files:**
- Modify: `src/ui/files_tests.rs:128-160`
- Modify: `docs/_reference/connections.md:39`

- [ ] **Step 1: Rename the row test and name every type**

In `src/ui/files_tests.rs`, rename `the_type_is_a_radio_group_of_four_on_a_row_of_its_own` to `the_type_is_a_radio_group_above_the_name`, and make its loop name all five:

```rust
        for name in ["SQLite", "PostgreSQL", "MySQL", "BigQuery", "Files"] {
```

- [ ] **Step 2: The reference**

In `docs/_reference/connections.md`, the **Type** row's text becomes:

```markdown
| **Type** | SQLite, PostgreSQL, MySQL, BigQuery or Files, as radio buttons in rows above the name. On macOS the type is one Tab stop and the arrow keys choose; on Omarchy each type is a Tab stop and Enter or Space picks it. MySQL is also for [MariaDB](#mariadb). Changing it keeps the name, the environment and whatever was typed for the other types until the dialog closes. Saving a connection you have used under another type asks first |
```

- [ ] **Step 3: Run the tests**

Run: `~/.cargo/bin/cargo test -p tabletist files_tests`
Expected: PASS.

- [ ] **Step 4: Commit**

```bash
git add src/ui/files_tests.rs docs/_reference/connections.md
git commit -m "Say in the reference how the Type row's keys work"
```

### Task 4: The whole suite, then by hand

- [ ] **Step 1: Format, lint, test**

Run: `~/.cargo/bin/cargo fmt --check`
Run: `~/.cargo/bin/cargo clippy --all-targets -- -D warnings`
Run: `~/.cargo/bin/cargo test -p tabletist` (in the background: it runs for minutes)
Expected: all PASS. A test that tabbed through the macOS types one by one now reaches Name after one Tab: fix it to the new order, it is the change agreed in departure 3.

- [ ] **Step 2: By hand, on the Bookshop demo**

Run the app on the Bookshop demo database in both looks and check, without screenshots leaving the machine:
- macOS light and dark: New connection shows SQLite, PostgreSQL, MySQL, BigQuery on one row and Files under SQLite; the hint is under the grid and follows the type; Shift+Tab from Name lands on the chosen type; the arrows choose and the form below changes; Tab goes to Name.
- A window under 748 wide (macOS) or 828 (Omarchy): fewer columns, nothing cut off.
- Omarchy: two rows of three and two; Tab walks the types; Space and Enter pick; ← → and h l do nothing; the ▌ bar shows on the type that has the keyboard.
- Edit the saved Bookshop production connection, change its type and back: the host, user and database typed for PostgreSQL are still there.
- The Environment row is unchanged in both looks.

- [ ] **Step 3: Pull request**

Once #247 is merged: rebase onto main, push, and open the PR. Its body lists the six departures and that Igor agreed to all six on 2026-11-17.
