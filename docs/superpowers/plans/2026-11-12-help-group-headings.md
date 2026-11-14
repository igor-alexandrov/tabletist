# Group headings in the keyboard shortcuts dialog Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use rubypowers:subagent-driven-development (recommended) or rubypowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make the group headings of the keyboard shortcuts dialog (`?`) stand apart from its rows, and close the gap between a command's keys and its name (issue #300, item 2).

**Architecture:** `src/ui/help.rs` stops drawing the list as one `egui::Grid` whose headings are rows of the keys column. Each group becomes a scope with a `RowGroup` accessibility node named by its heading, as Settings › Keyboard's `group_rows` does: a heading band in `widgets::section_label` (30 pt, 12 pt less for the list's first), then its rows. A row is the command's name at the left and its keys right-aligned at the right edge, with Settings' hairline under it (none in Omarchy). After the scroll area, the heading of the group the list is scrolled into is painted pinned over the list's top, with the next heading pushing it out. Omarchy's heading has a rule running on to the right edge.

**Tech Stack:** Rust, egui (`ScrollArea`, `UiBuilder`, accesskit), the crate's headless `testing::Harness`.

**Design:** the Tabletist Designs canvas, Settings page, "Group headings in ? (#300, item 2)": `MacHelpHeadings`, `WindowsHelpHeadings`, `OmarchyHelpHeadings`. Approved: A (section labels with room above) + B (names first, keys right-aligned) + the pinned heading; no divider; Omarchy's rule after the heading. Design material is not committed.

---

## File structure

- Modify: `src/ui/help.rs`, the dialog: width and height of the list, `group_rows`, `row_line`, `heading_label`, `pin`; `heading` becomes `pub(crate)` for the tests.
- Modify: `src/ui/help_tests.rs`, new tests over the harness's painted text rects and the accessibility tree.

## Measures

```rust
/// Space between a command's name and its keys, at the least.
const GAP: f32 = 24.0;
/// Space between the rows of the list.
const ROW_GAP: f32 = 6.0;
/// A group's heading band, Settings › Keyboard's: its name near the foot,
/// room above it.
const HEADING: f32 = 30.0;
/// The room above a heading that the list's first one goes without.
const ABOVE: f32 = 12.0;
/// The band the list's first heading takes, and the one pinned at its top.
const PINNED: f32 = HEADING - ABOVE;
/// Where a heading's name centres, above its band's foot.
const NAME_ABOVE_FOOT: f32 = 8.0;
/// Room at the list's right for the scroll bar, which floats over it.
const BAR_ROOM: f32 = 12.0;
```

The list's whole height: `rows × (body row height + ROW_GAP) + groups × HEADING − ABOVE`. Its width: the widest `name + GAP + keys` of the look's rows (or the widest heading), plus `BAR_ROOM`, no wider than the window leaves.

### Task 1: Tests for headings, rows and the pin

**Files:**
- Modify: `src/ui/help.rs:45` (`fn heading` → `pub(crate) fn heading`)
- Test: `src/ui/help_tests.rs`

- [ ] **Step 1: Write the failing tests**

Helpers, after `close_button_top`:

```rust
/// A group's heading as the dialog paints it: capitals, or Omarchy's
/// lower case.
fn painted_heading(look: &Look, group: Group) -> String {
    let name = crate::ui::help::heading(group);
    if look.terminal {
        name.to_lowercase()
    } else {
        name.to_uppercase()
    }
}

/// What the list shows, top to bottom: each heading and each command's
/// name painted inside the list's clip, with where it stands.
fn shown(harness: &mut Harness, rows: &[(Group, Command)]) -> Vec<(Shown, Rect)> {
    harness.settle();
    let look = harness.app.look;
    let mut shown: Vec<(Shown, Rect)> = harness
        .clipped_text
        .iter()
        .filter(|(_, rect, clip)| clip.contains_rect(*rect))
        .filter_map(|(text, rect, _)| {
            let heading = rows
                .iter()
                .find(|(group, _)| painted_heading(&look, *group) == *text)
                .map(|(group, _)| Shown::Heading(*group));
            let name = rows
                .iter()
                .find(|(_, command)| command.info().name == text)
                .map(|(group, _)| Shown::Row(*group));
            heading.or(name).map(|shown| (shown, *rect))
        })
        .collect();
    shown.sort_by(|a, b| a.1.top().total_cmp(&b.1.top()));
    shown
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Shown {
    Heading(Group),
    Row(Group),
}

/// The groups and commands the dialog lists in its look.
fn grouped(harness: &Harness) -> Vec<(Group, Command)> {
    let layout = harness.app.layout();
    crate::ui::help::rows(&harness.app.keymap, layout)
        .into_iter()
        .map(|(group, command, _)| (group, command))
        .collect()
}
```

Tests:

```rust
#[test]
fn each_group_has_room_above_its_heading_but_the_first() {
    for look in Look::ALL {
        let name = look.name;
        let mut harness = opened(look);
        let rows = grouped(&harness);
        let shown = shown(&mut harness, &rows);
        assert!(matches!(shown[0].0, Shown::Heading(_)), "{name}: {shown:?}");
        let first_row = shown.iter().find(|(at, _)| matches!(at, Shown::Row(_))).unwrap();
        // A heading starts where the names start: across the list.
        assert!((shown[0].1.left() - first_row.1.left()).abs() < 1.0, "{name}");
        let mut row_gaps = Vec::new();
        let mut heading_gaps = Vec::new();
        for pair in shown.windows(2) {
            let gap = pair[1].1.top() - pair[0].1.bottom();
            match pair[1].0 {
                Shown::Heading(_) => heading_gaps.push(gap),
                Shown::Row(_) => row_gaps.push(gap),
            }
        }
        assert!(!heading_gaps.is_empty(), "{name}: one group shown");
        let widest_row_gap = row_gaps.iter().copied().fold(0.0_f32, f32::max);
        for gap in heading_gaps {
            assert!(gap >= widest_row_gap + 10.0, "{name}: {gap} after {widest_row_gap}");
        }
        // Each group is a row group named by its heading.
        let tree = harness.settle();
        for (group, _) in &rows {
            let said = look.label(crate::ui::help::heading(*group));
            assert!(crate::testing::node(&tree, &said, Role::RowGroup).is_some(), "{name}: {said}");
        }
    }
}

#[test]
fn keys_stand_at_the_right_after_their_names() {
    for look in Look::ALL {
        let name = look.name;
        let mut harness = opened(look);
        harness.settle();
        let layout = harness.app.layout();
        let rows = crate::ui::help::rows(&harness.app.keymap, layout);
        let mut rights = Vec::new();
        for (_, command, keys) in &rows {
            let visible = |text: &str| {
                harness
                    .clipped_text
                    .iter()
                    .find(|(painted, rect, clip)| painted == text && clip.contains_rect(*rect))
                    .map(|(_, rect, _)| *rect)
            };
            let (Some(what), Some(keys)) = (visible(command.info().name), visible(keys.as_str())) else {
                continue;
            };
            assert!(keys.left() >= what.right() + 20.0, "{name}: {command:?}");
            rights.push(keys.right());
        }
        assert!(rights.len() > 5, "{name}: {rights:?}");
        let (least, most) = rights.iter().fold((f32::MAX, f32::MIN), |(a, b), r| (a.min(*r), b.max(*r)));
        assert!(most - least < 1.0, "{name}: {rights:?}");
    }
}

#[test]
fn a_heading_stays_on_top_while_its_group_scrolls() {
    for look in Look::ALL {
        let name = look.name;
        let mut harness = opened(look);
        let rows = grouped(&harness);
        let before = shown(&mut harness, &rows);
        let (Shown::Heading(first), top) = before[0] else { panic!("{name}: {before:?}") };
        // Not pinned until the list moves: the first heading shows once.
        let once = before.iter().filter(|(at, _)| *at == Shown::Heading(first)).count();
        assert_eq!(once, 1, "{name}");
        for _ in 0..3 {
            harness.frame(vec![
                Event::PointerMoved(top.center()),
                Event::MouseWheel {
                    unit: egui::MouseWheelUnit::Point,
                    delta: egui::vec2(0.0, -150.0),
                    modifiers: Modifiers::NONE,
                    phase: egui::TouchPhase::Move,
                },
            ]);
        }
        for _ in 0..5 {
            harness.finish_animations();
        }
        let after = shown(&mut harness, &rows);
        // The heading at the list's top is the group of the rows under it.
        let pinned = after
            .iter()
            .position(|(at, rect)| matches!(at, Shown::Heading(_)) && (rect.top() - top.top()).abs() < 1.0)
            .unwrap_or_else(|| panic!("{name}: nothing pinned: {after:?}"));
        let Shown::Heading(group) = after[pinned].0 else { unreachable!() };
        let next_row = after.iter().find(|(at, rect)| {
            matches!(at, Shown::Row(_)) && rect.top() >= after[pinned].1.bottom()
        });
        assert_eq!(next_row.map(|(at, _)| *at), Some(Shown::Row(group)), "{name}: {after:?}");
        // It scrolled: the list's first rows have gone.
        let first_row = rows[0].1.info().name;
        assert!(
            !harness.clipped_text.iter().any(|(text, rect, clip)| text == first_row && clip.contains_rect(*rect) && rect.top() > after[pinned].1.bottom()),
            "{name}"
        );
    }
}

#[test]
fn a_search_keeps_the_headings_of_the_groups_it_keeps() {
    for look in Look::ALL {
        let name = look.name;
        let mut harness = opened(look);
        let rows = grouped(&harness);
        type_text(&mut harness, "new");
        let kept = listed(&harness, "new", None);
        let mut groups: Vec<Group> = rows
            .iter()
            .filter(|(_, command)| kept.contains(command))
            .map(|(group, _)| *group)
            .collect();
        groups.dedup();
        assert!(groups.len() >= 2, "{name}: {groups:?}");
        let shown = shown(&mut harness, &rows);
        let headings: Vec<Group> = shown
            .iter()
            .filter_map(|(at, _)| match at {
                Shown::Heading(group) => Some(*group),
                Shown::Row(_) => None,
            })
            .collect();
        assert_eq!(headings, groups, "{name}");
        // Each heading stands above its own rows.
        let mut under = None;
        for (at, _) in &shown {
            match at {
                Shown::Heading(group) => under = Some(*group),
                Shown::Row(group) => assert_eq!(under, Some(*group), "{name}"),
            }
        }
    }
}
```

Imports: `use egui::Rect;` and `use crate::keymap::{Command, Group};`.

- [ ] **Step 2: Run them to see them fail**

Run: `CARGO_TARGET_DIR=$PWD/target ~/.cargo/bin/cargo test --locked -p tabletist --lib help_tests`
Expected: the four new tests fail (headings in the keys column, no room above, keys left of names, nothing pinned); the old ones pass.

### Task 2: Headings, rows and the pin

**Files:**
- Modify: `src/ui/help.rs`

- [ ] **Step 1: Measure the list for B**

In `show`, replace the keys/what widths with the widest row and heading:

```rust
let mut natural = 0.0_f32;
for (_, command, keys) in &all {
    let keys = Text::one(&look, widgets::code(&look), keys, palette.text);
    let what = Text::one(&look, widgets::body(&look), &gettext(locale, command.info().name), palette.text);
    natural = natural.max(widgets::measure(ui, what) + GAP + widgets::measure(ui, keys));
}
for (group, _) in GROUPS {
    let name = look.label(&gettext(locale, heading(group)));
    natural = natural.max(widgets::measure(ui, widgets::section_label(&name, &look, &palette)));
}
let natural = natural.ceil();
let width = (natural + BAR_ROOM).min(screen.width() - 80.0).max(120.0);
```

and the height:

```rust
let full = all.len() as f32 * line + groups as f32 * HEADING - ABOVE;
```

- [ ] **Step 2: Draw the groups in place of the grid**

```rust
let output = egui::ScrollArea::both()
    .max_height(height)
    .auto_shrink([false, false])
    .show(&mut list, |ui| {
        if listed.is_empty() {
            nothing_listed(ui, paint, search);
            return Vec::new();
        }
        listed
            .chunk_by(|a, b| a.group == b.group)
            .enumerate()
            .map(|(at, rows)| group_rows(ui, paint, rows, natural, at == 0))
            .collect()
    });
pin(&list, paint, output.inner_rect, &output.inner, natural);
```

with:

```rust
/// Where a group's heading stands in the list as drawn.
struct Placed {
    group: Group,
    left: f32,
    /// The foot of its band.
    foot: f32,
}

/// A group: its heading, then its rows, as one row group named by the
/// heading, as Settings › Keyboard has them.
fn group_rows(ui: &mut egui::Ui, paint: Paint, rows: &[Listed], width: f32, first: bool) -> Placed {
    let Paint { locale, look, .. } = paint;
    let group = rows[0].group;
    let name = look.label(&gettext(locale, heading(group)));
    let builder = egui::UiBuilder::new().id_salt(("help-group", heading(group)));
    ui.scope_builder(builder, |ui| {
        ui.ctx().accesskit_node_builder(ui.unique_id(), |node| {
            node.set_role(egui::accesskit::Role::RowGroup);
            node.set_label(name.as_str());
        });
        let band = if first { PINNED } else { HEADING };
        let (rect, _) = ui.allocate_exact_size(vec2(width, band), Sense::hover());
        // Painted only: the row group carries the name for a screen reader.
        heading_label(ui, ui.painter(), rect, &name, paint);
        for row in rows {
            row_line(ui, paint, row, width);
        }
        Placed { group, left: rect.left(), foot: rect.bottom() }
    })
    .inner
}
```

`heading_label` paints the section label centred `NAME_ABOVE_FOOT` above the band's foot and, in Omarchy, a rule from 10 pt after it to the band's right edge in `palette.border`. `row_line` allocates `(width, line)`, lays the marked name out left to right and the marked keys right to left in a child `Ui` on that rect, and in macOS and Windows draws `widgets::hline` at the row's foot in `palette.surface`.

- [ ] **Step 3: Pin the heading**

```rust
/// Pins the heading of the group the list is scrolled into over the
/// list's top, the rows scrolling under it; the next heading pushes it out.
fn pin(ui: &egui::Ui, paint: Paint, view: Rect, placed: &[Placed], width: f32) {
    let Some(at) = placed.iter().rposition(|group| group.foot - PINNED < view.top()) else {
        return;
    };
    let mut foot = view.top() + PINNED;
    if let Some(next) = placed.get(at + 1) {
        foot = foot.min(next.foot - PINNED);
    }
    let painter = ui.painter_at(view);
    let band = Rect::from_x_y_ranges(view.x_range(), (foot - PINNED)..=foot);
    painter.rect_filled(band, CornerRadius::ZERO, paint.palette.overlay);
    let name = paint.look.label(&gettext(paint.locale, heading(placed[at].group)));
    let left = placed[at].left;
    heading_label(ui, &painter, Rect::from_x_y_ranges(left..=left + width, band.y_range()), &name, paint);
    if !paint.look.terminal {
        let y = painter.round_to_pixel_center(foot - 0.5);
        painter.hline(view.x_range(), y, Stroke::new(1.0 / ui.pixels_per_point(), paint.palette.border));
    }
}
```

- [ ] **Step 4: Run the tests**

Run: `CARGO_TARGET_DIR=$PWD/target ~/.cargo/bin/cargo test --locked -p tabletist --lib help_tests`
Expected: all pass, the old ones too (search, Esc, the kept size).

### Task 3: Checks and commit

- [ ] **Step 1:** `~/.cargo/bin/cargo fmt`
- [ ] **Step 2:** `CARGO_TARGET_DIR=$PWD/target ~/.cargo/bin/cargo clippy --locked --workspace --all-targets -- -D warnings`
- [ ] **Step 3:** `CARGO_TARGET_DIR=$PWD/target ~/.cargo/bin/cargo test --locked -p tabletist --all-targets`
- [ ] **Step 4:** Commit the plan, `help.rs` and `help_tests.rs`. Ask before pushing; the PR is part of #300 and must not close it ("Part of #300", never "Closes").
