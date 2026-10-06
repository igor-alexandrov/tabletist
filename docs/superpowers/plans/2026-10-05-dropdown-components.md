# Dropdowns and menus as the Components sheet draws them Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Draw every dropdown and every menu in the app as the design's Components sheet draws them, in the macOS look and in the Omarchy look.

**Architecture:** Today the app draws menus four ways: egui's `ComboBox`, `widgets::popup_menu`, and three menus built by hand from `egui::Popup` and buttons. All of them take egui's default menu: a filled row for the choice in use, 6 pt between rows, the look's button corners. One new module, `src/ui/menu.rs`, owns the panel, the row and the dropdown field, and every call site moves to it. What the menus do stays as it is, with one exception: a dropdown's menu now closes on a pick that no pointer made, and gives the keyboard back to the dropdown.

**Tech Stack:** Rust 2024, egui (crmne fork, `egui::Popup`), fastframe-icons. Headless UI tests through `src/testing.rs` (AccessKit tree, painted fills and text).

**Design:** the user's Design canvas, the artboards "macOS – Components, light", "macOS – Components, dark" and "Omarchy – Components". It is not in the repository and is not to be added. The values this plan needs are written out below.

**Size:** three code tasks and a by-eye check, about an hour inline. Each code task leaves the app working and the checks green, so the run can stop after any of them. Task 1 alone fixes the menus in the report (the SQL editor's Limit and Timeout, and the sidebar's schema).

---

## Ground rules (read first)

- Work in `/home/igor/Work/tabletist/.claude/worktrees/dropdown-component-compliance-ec4446`, branch `claude/dropdown-component-compliance-ec4446`.
- `cargo` through mise is broken here: always run `~/.cargo/bin/cargo`. Never point `CARGO_TARGET_DIR` at `/tmp`.
- The shell is zsh: quote globs, and write `echo '---'`, never `echo ===`.
- **Never commit or push unless the user asks.** Each task ends with a "commit point": stop there and report. If the user has asked to commit: one topic per commit, signed (`git commit -S`). If signing fails, ask before committing unsigned. Never set `SSH_AUTH_SOCK` in git commands. End messages with `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.
- No em dashes anywhere (code, comments, docs, commit messages). Use a full stop, comma, colon or parentheses.
- Tests are behavioural: what a screen reader is told, that the choice in use is not filled, that the pointer and the keyboard light a row with the palette's own colour, that rows touch, that a pick closes the menu. Never a design or pixel conformance test: do not assert the design's sizes (28, 5, 4) or its hex colours.
- Design material and screenshots are never committed. Screenshots use the Bookshop demo data only.
- Views draw text only through `TextRole`s and the `widgets` helpers; never name a font or a size.
- Focus is drawn in `src/ui/focus.rs`. A widget that shows the keyboard by itself says so with `focus::hint(.., Ring::Own)`.
- Views do not mutate application state apart from the text or value a control is editing: push an `Action`.
- Comment density: short doc comments on items saying what and why, matching the surrounding code.
- Apply each diff below with the Edit tool, hunk by hunk. The diffs were generated from a draft that passed every check; the line numbers in their headers are the draft's and are right for a tree at the task before.

Full check commands (each task ends with them):

```bash
~/.cargo/bin/cargo fmt --all --check
~/.cargo/bin/cargo clippy --locked --workspace --all-targets -- -D warnings
~/.cargo/bin/cargo clippy --locked --workspace --all-targets --features shots -- -D warnings
~/.cargo/bin/cargo test --locked --workspace --all-targets
RUSTDOCFLAGS='-D warnings' ~/.cargo/bin/cargo doc --locked --workspace --no-deps
```

## What the design says

### macOS (the same in light and dark, colours aside)

The dropdown, in the sheet's "Fields" section: "same box, ▾ at the end".

| | Light | Dark |
|---|---|---|
| Box | 30 high, padding 0 10, corners 7, 1 px border | the same |
| Fill | `#ffffff` | `#262624` |
| Border | `#dcdad4` | `#44433f` |
| Border, hover | `#c9c7c1` | `#55534e` |
| Focus | accent border and a 3 px halo, as a text field | the same |
| `▾` | 10 px, `#6b6a65`, at the right end | `#a3a19b` |

The menu, with the sheet's "Menu rules": "Items 28 high, radius 5, 4px inset. Hover and keyboard highlight look the same. The current value gets a ✓. Disabled items stay visible. Type-ahead jumps to matches."

```html
<div class="pop" style="width: 230px; padding: 4px">
<div class="mi"><span style="width: 14px"></span>disable</div>
<div class="mi" style="background: #e4ebfb; color: #1e3f9e"><span style="width: 14px"></span>verify-ca<span class="mk">hover / keyboard</span></div>
<div class="mi"><span style="width: 14px">✓</span><b style="font-weight: 500">verify-full</b><span class="mk">selected</span></div>
</div>
```

```css
.pop{background:#ffffff;border:1px solid #dcdad4;border-radius:8px;box-shadow:0 8px 24px rgba(28,28,26,0.14)}
.mi{display:flex;align-items:center;gap:6px;height:28px;padding:0 8px;border-radius:5px}
.mk{margin-left:auto;font-size:11px;color:#9a9892}
```

Dark: panel `#262624` with border `#44433f`; the lit row `#2b3550` with text `#a9c1fa`.

### Omarchy

The sheet has no dropdown box of its own: a form's choice is a cycle value (`‹ verify-full ›`, which the settings screen already draws) and every list of choices is a picker: "pickers float over the pane with an accent border. j/k or ctrl+n/p move, enter picks, esc closes. Typing filters; matches are highlighted in accent."

```html
<div style="width: 240px; border: 1px solid {{t.accent}}; border-radius: 3px; background: {{t.dark}}">
<span class="pk">  disable</span>
<span class="pk" style="background: {{t.sel}}"><span style="color: {{t.accent}}">▌</span>verify-ca</span>
<span class="pk"><span style="color: {{t.green}}">✓</span> verify-full</span>
</div>
```

```css
.pk{display:flex;align-items:center;padding:1px 10px;white-space:pre}
```

A choice shown in a box (the connection sheet's group) is an outline: `height: 30px; padding: 0 8px; border: 1px solid {{t.border}}; border-radius: 3px`, the value, then a muted `▾` at the right.

### How the design's names map to the code

| Design | Code |
|---|---|
| `#ffffff` / `#262624` (panel, dropdown fill) | `palette.overlay` (panel), `palette.window` (dropdown) |
| `#dcdad4` / `#44433f` | `palette.border` |
| `#c9c7c1` / `#55534e` | `palette.border.lerp_to_gamma(palette.text, 0.1)` (exact in light) |
| `#e4ebfb` / `#2b3550`, `t.sel` | `palette.selection` |
| `#1e3f9e` / `#a9c1fa` | `palette.accent_hover` |
| `#6b6a65` / `#a3a19b`, `t.muted` | `palette.dim` |
| `rgba(28,28,26,0.14)` | `palette.shadow.gamma_multiply(0.5)` (the light palette's shadow is `rgba(28,28,26,0.28)`) |
| `t.accent`, `t.dark`, `t.border`, `t.green` | `palette.accent`, `palette.panel`, `palette.outline`, `palette.success` |
| weight 500 | `TextRole::UiBodyStrong` (and `MonoGroup` for `MonoSecondary`) |
| `✓` on macOS | `Icon::Check` at 12 |

## What the app does today

| Where | File | Drawn with |
|---|---|---|
| Filter bar: column, operator | `src/ui/filter_bar.rs` | `widgets::popup_button` (egui `ComboBox`) |
| Settings: Rows per page | `src/ui/settings/sheet.rs` | `widgets::popup_button` |
| Connection sheet: SSL mode, Authentication | `src/ui/connect_dialog/sheet.rs` | its own `select` (egui `ComboBox`) |
| SQL editor: Limit, Timeout | `src/ui/sql_editor.rs` | `widgets::popup_menu` |
| Sidebar: schema | `src/ui/sidebar.rs` | `widgets::popup_menu` |
| Connection bar: database | `src/ui/workspace.rs` | `egui::Popup::menu` and `Button::selectable` |
| Connection sheet: hosts from `~/.ssh/config` | `src/ui/connect_dialog/mod.rs` | `egui::Popup::menu` and `widgets::button` |
| Connections: a row's context menu and its More menu | `src/ui/picker.rs` | `Response::context_menu`, `egui::Popup::menu` |

What all of them get wrong against the sheet: the choice in use is filled (the accent at 35 %) and has no check mark; rows are 6 pt apart; rows take the look's button corners (8 on macOS); the panel has a 6 pt margin, the window's border colour and a harder shadow; in the Omarchy look the panel takes a dialog's 2 pt border. On macOS a dropdown is a raised button with a shadow and up and down chevrons where the sheet draws a field with one `▾`.

Not dropdowns, and left alone: the completion list (`src/ui/sql_complete.rs`) and the cell editor's popover follow their own artboards; the terminal settings screen's cycle values already follow the sheet.

## Decisions in this plan (say so if any should go the other way)

1. **The macOS "raised pop-up button" goes.** `Look::raised_popups`, `widgets::popup_button` and its up and down chevrons were a platform-looks decision; the Components sheet draws a field with `▾`. The flag has no other use and is removed.
2. **A dropdown takes the sheet's box wherever it stands.** In the filter bar the value field beside it keeps what it has today (the surface tone, no border): text fields are their own drift from the sheet and are not part of this change. In the connection sheet the dropdown takes the sheet's own field fill, which in the dark palette is `surface`, so it matches the fields beside it.
3. **Control heights stay the look's.** The sheet's dropdown is 30 high; the app's controls are 28 on macOS (`Look::control_height`) and the settings artboard draws its dropdown 28 high. Changing the height of every control is not a dropdown fix.
4. **The check mark's column is for choices only.** A menu of actions (a connection's Duplicate and Delete, the `~/.ssh/config` hosts) starts its words at the row's padding on macOS. In the Omarchy look every row has the column: the cursor `▌` needs it.
5. **Omarchy menus are pickers without the filter line.** The border, the cursor and the mark are drawn; typing to filter is behaviour no menu has today and is left for its own change. So is the macOS "type-ahead jumps to matches".
6. **Nothing is built that no menu has:** disabled rows, separators, submenus.
7. **An Omarchy dropdown is the sheet's outline** (1 px `outline`, corners 3, a muted `▾`), though the filter bar's text fields beside it are square with a brighter border today. Same reason as 2.
8. **Windows takes the macOS menu and dropdown.** Its artboard is a draft with no Components sheet.
9. **A dropdown's menu closes on a pick that no pointer made**, and the keyboard is back on the dropdown after it or after Escape. The combo boxes (filter bar, connection sheet) stayed open after a pick by key or by a screen reader, where the SQL editor's menus closed. The three menus of Task 3 keep what they do today: they close on a pointer's click only (see "Left for later").
10. **What opens a menu keeps its own drawing** where it is not a dropdown field: the SQL editor's Limit and Timeout buttons, the sidebar's schema heading and the connection bar's chip (with its up and down chevrons) follow their own artboards. Only their menus change. Do not "fix" them.

## File structure

- Create `src/ui/menu.rs`: the menu's panel (`frame`), its row (`Item`), the two ways a menu opens (`under`, `context`), a menu of choices by index (`choices`, with `Choice`), a choice bound to a value (`value`), and the dropdown field (`Dropdown`). Its tests.
- `src/ui/mod.rs`: registers the module.
- `src/ui/widgets.rs`: loses `MenuChoice`, `popup_menu`, `popup_button`, `paint_chevrons` and the pop-up button's test.
- `src/theme.rs`: loses `Look::raised_popups`.
- `src/ui/sidebar.rs`, `src/ui/sql_editor.rs`, `src/ui/filter_bar.rs`, `src/ui/settings/sheet.rs`, `src/ui/connect_dialog/sheet.rs`, `src/ui/connect_dialog/mod.rs`, `src/ui/workspace.rs`, `src/ui/picker.rs`: their menus move to `menu`.

---

### Task 1: The menu's panel and row, and the menus of choices

Moves `widgets::popup_menu` to `menu::choices`, drawn by the new panel and row. After it the SQL editor's Limit and Timeout menus and the sidebar's schema menu follow the sheet.

**Files:**
- Create: `src/ui/menu.rs`
- Modify: `src/ui/mod.rs`, `src/ui/widgets.rs`, `src/ui/sidebar.rs`, `src/ui/sql_editor.rs`
- Test: `src/ui/menu.rs` (its `tests` module)

- [ ] **Step 1: Write the failing tests**

Create `src/ui/menu.rs` with the module's doc comment and its tests. The tests drive the SQL editor's Limit menu through the app, so they run against the menu as it is today.

```rust
//! Menus, and the dropdowns that open them: one panel and one kind of row
//! for every menu in the app, as the designs' Components sheet draws them.
//!
//! macOS and Windows: a panel with a 1 pt border, corners of 8 and a soft
//! shadow, its rows 4 in from its edge, 28 high with corners of 5. The row
//! under the pointer or the keyboard takes the selection's tint and the
//! strong accent. The choice in use takes a check mark and a heavier face,
//! never a fill.
//!
//! The terminal: a picker. A 1 pt accent border with corners of 3 round
//! rows a line high; `▌` marks the row under the pointer or the keyboard
//! and a green `✓` the choice in use.

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use egui::accesskit::Role;
    use egui::{Key, Modifiers};

    use crate::model::Action;
    use crate::testing::{Harness, bounds, node};
    use crate::theme::Look;
    use crate::ui::tests::focused_name;

    /// A SQL editor with its Limit menu open: three choices, the second in
    /// use.
    fn limit_menu(look: Look) -> Harness {
        let mut harness = Harness::new();
        harness.set_look(look);
        let tab = harness.connect_fake();
        harness.app.apply(Action::NewSqlTab(tab));
        harness.click("Limit");
        harness
    }

    fn row(harness: &mut Harness, name: &str) -> egui::Rect {
        let tree = harness.settle();
        bounds(&tree, name, Role::Button).unwrap_or_else(|| panic!("no row {name}"))
    }

    /// Gives the keyboard to the row named `name`, as a screen reader does.
    fn give_keyboard(harness: &mut Harness, name: &str) {
        let tree = harness.settle();
        let target = node(&tree, name, Role::Button).unwrap_or_else(|| panic!("no row {name}"));
        harness.frame(vec![egui::Event::AccessKitActionRequest(
            egui::accesskit::ActionRequest {
                target_tree: egui::accesskit::TreeId::ROOT,
                target_node: target,
                action: egui::accesskit::Action::Focus,
                data: None,
            },
        )]);
        harness.settle();
    }

    /// What the last frame filled the row at `rect` with, if anything.
    fn fill(harness: &Harness, rect: egui::Rect) -> Option<egui::Color32> {
        harness
            .fills
            .iter()
            .find(|(filled, _)| {
                (filled.center() - rect.center()).length() < 1.0
                    && (filled.height() - rect.height()).abs() < 1.0
            })
            .map(|(_, color)| *color)
    }

    #[test]
    fn the_choice_in_use_is_marked_and_not_filled() {
        for look in Look::ALL {
            let mut harness = limit_menu(look);
            let tree = harness.settle();
            let chosen = |name: &str| {
                let id = node(&tree, name, Role::Button).unwrap();
                let (_, node) = tree.nodes.iter().find(|(n, _)| *n == id).unwrap();
                node.toggled() == Some(egui::accesskit::Toggled::True)
            };
            assert!(chosen("Limit 1,000"), "{}", look.name);
            assert!(!chosen("Limit 100"), "{}", look.name);
            let current = row(&mut harness, "Limit 1,000");
            assert_eq!(fill(&harness, current), None, "{}", look.name);
        }
    }

    #[test]
    fn a_menus_rows_touch_and_are_as_tall_as_each_other() {
        for look in Look::ALL {
            let mut harness = limit_menu(look);
            let rows =
                ["Limit 100", "Limit 1,000", "Limit 10,000"].map(|name| row(&mut harness, name));
            for pair in rows.windows(2) {
                assert!(
                    (pair[0].bottom() - pair[1].top()).abs() < 0.5,
                    "{pair:?} in {}",
                    look.name
                );
                assert!(
                    (pair[0].height() - pair[1].height()).abs() < 0.5,
                    "{pair:?} in {}",
                    look.name
                );
                assert!(
                    (pair[0].width() - pair[1].width()).abs() < 0.5,
                    "{pair:?} in {}",
                    look.name
                );
            }
        }
    }

    #[test]
    fn the_pointer_and_the_keyboard_light_a_row_the_same_way() {
        for look in Look::ALL {
            let mut harness = limit_menu(look);
            let selection = harness.app.palette.selection;
            let first = row(&mut harness, "Limit 100");
            assert_eq!(fill(&harness, first), None, "{}", look.name);
            harness.frame(vec![egui::Event::PointerMoved(first.center())]);
            harness.settle();
            assert_eq!(
                fill(&harness, first),
                Some(selection),
                "the row under the pointer in {}",
                look.name
            );
            harness.frame(vec![egui::Event::PointerGone]);
            harness.settle();
            assert_eq!(fill(&harness, first), None, "{}", look.name);
            // A screen reader puts the keyboard on a row, and a key moves
            // it: only then is it shown.
            give_keyboard(&mut harness, "Limit 100");
            harness.press(Key::ArrowDown, Modifiers::NONE);
            let name = focused_name(&harness.settle());
            assert_eq!(name, "Limit 1,000", "{}", look.name);
            let lit = row(&mut harness, &name);
            assert_eq!(
                fill(&harness, lit),
                Some(selection),
                "the row with the keyboard in {}",
                look.name
            );
        }
    }
}
```

Register the module in `src/ui/mod.rs`:

```diff
diff --git a/src/ui/mod.rs b/src/ui/mod.rs
--- a/src/ui/mod.rs
+++ b/src/ui/mod.rs
@@ -18,6 +18,7 @@ pub mod help;
 pub mod host_key_prompt;
 pub mod json_view;
 pub mod keys;
+pub mod menu;
 pub mod object_tabs;
 pub mod password_prompt;
 pub mod pending_bar;
```

- [ ] **Step 2: Run the tests to see them fail**

Run: `~/.cargo/bin/cargo test --locked --lib ui::menu`

Expected: 3 failed.
- `the_choice_in_use_is_marked_and_not_filled`: `left: Some(#2B_36_55_58)`, `right: None` (the choice in use is filled).
- `a_menus_rows_touch_and_are_as_tall_as_each_other`: two rectangles 6 apart (`[1047.0 142.0] - [1150.0 168.0]`, then `[1047.0 174.0] ...`).
- `the_pointer_and_the_keyboard_light_a_row_the_same_way`: "the row under the pointer in standard", `left: Some(#3A_39_36_FF)`, `right: Some(#2B_35_50_FF)` (the hover fill is not the selection's).

- [ ] **Step 3: Write the panel, the row and the menus**

In `src/ui/menu.rs`, between the doc comment and `#[cfg(test)]`:

```rust
use egui::{CornerRadius, Rect, Response, Sense, Stroke, Ui, WidgetInfo, WidgetType, pos2, vec2};

use crate::theme::{Icon, Look, Palette};
use crate::typography::{Text, TextRole};
use crate::ui::focus::{self, Ring};
use crate::ui::widgets;

/// How far a menu opens from what it hangs under.
const GAP: f32 = 4.0;
/// The room between a row's words and the note at its right.
const DETAIL_GAP: f32 = 16.0;

/// The terminal's mark of the choice in use. The second for a face without
/// the first: the look's font is the desktop's own.
const CURRENT: &[&str] = &["✓", "√"];

/// A row's measures in a look.
struct Shape {
    height: f32,
    corner: u8,
    /// How far in the row's first mark or word starts, and its last ends.
    inset: f32,
    /// The column a choice's mark sits in, and the room after it.
    gutter: f32,
    gap: f32,
}

impl Shape {
    fn of(ui: &Ui, look: &Look) -> Self {
        if look.terminal {
            let role = TextRole::OBody;
            let width = |text| role.width(ui.ctx(), look.faces, text);
            Self {
                // A line of text, 1 above and 1 below.
                height: role.row_height(ui.ctx(), look.faces) + 2.0,
                corner: 0,
                inset: 10.0,
                gutter: width("▌"),
                gap: width(" "),
            }
        } else {
            Self {
                height: 28.0,
                corner: 5,
                inset: 8.0,
                gutter: 14.0,
                gap: 6.0,
            }
        }
    }
}

/// The panel a menu's rows sit in.
pub fn frame(look: &Look, palette: &Palette) -> egui::Frame {
    if look.terminal {
        egui::Frame::new()
            .fill(palette.panel)
            .stroke(Stroke::new(1.0, palette.accent))
            .corner_radius(3)
    } else {
        egui::Frame::new()
            .fill(palette.overlay)
            .stroke(Stroke::new(1.0, palette.border))
            .corner_radius(8)
            .inner_margin(4)
            .shadow(egui::epaint::Shadow {
                offset: [0, 8],
                blur: 24,
                spread: 0,
                // Half a dialog's: a menu stands a step off the window.
                color: palette.shadow.gamma_multiply(0.5),
            })
    }
}

/// The heavier face the choice in use is written in.
fn strong(role: TextRole) -> TextRole {
    match role {
        TextRole::UiBody => TextRole::UiBodyStrong,
        TextRole::MonoSecondary => TextRole::MonoGroup,
        other => other,
    }
}

/// One row of a menu.
pub struct Item<'a> {
    text: &'a str,
    name: Option<&'a str>,
    current: Option<bool>,
    role: Option<TextRole>,
    detail: Option<&'a str>,
}

impl<'a> Item<'a> {
    /// A row that does something.
    pub fn action(text: &'a str) -> Self {
        Self {
            text,
            name: None,
            current: None,
            role: None,
            detail: None,
        }
    }

    /// One of a set of choices: `current` is the one in use.
    pub fn choice(text: &'a str, current: bool) -> Self {
        Self {
            current: Some(current),
            ..Self::action(text)
        }
    }

    /// Its name for screen readers, when it is not what it reads.
    pub fn name(mut self, name: &'a str) -> Self {
        self.name = Some(name);
        self
    }

    /// The role it reads in, when not the look's body.
    pub fn role(mut self, role: TextRole) -> Self {
        self.role = Some(role);
        self
    }

    /// A muted note at the row's right.
    pub fn detail(mut self, detail: &'a str) -> Self {
        self.detail = Some(detail);
        self
    }

    /// Draws the row across the menu. A click on it closes the menu by
    /// itself only when a pointer made it: see [`choices`].
    pub fn show(self, ui: &mut Ui, look: &Look, palette: &Palette) -> Response {
        let shape = Shape::of(ui, look);
        let current = self.current == Some(true);
        let role = self.role.unwrap_or_else(|| widgets::body(look));
        let role = if current && !look.terminal {
            strong(role)
        } else {
            role
        };
        // The terminal's cursor is a mark too: every row has its column.
        let marked = self.current.is_some() || look.terminal;
        let lead = shape.inset
            + if marked {
                shape.gutter + shape.gap
            } else {
                0.0
            };
        let note = widgets::secondary(look);
        let measure = |role: TextRole, text| role.width(ui.ctx(), look.faces, text);
        let noted = self
            .detail
            .map_or(0.0, |detail| DETAIL_GAP + measure(note, detail));
        let wanted = lead + measure(role, self.text) + noted + shape.inset;
        let (rect, response) = ui.allocate_at_least(vec2(wanted, shape.height), Sense::click());
        let name = self.name.unwrap_or(self.text);
        response.widget_info(|| match self.current {
            Some(current) => WidgetInfo::selected(WidgetType::Button, true, current, name),
            None => WidgetInfo::labeled(WidgetType::Button, true, name),
        });
        // The pointer and the keyboard light a row the same way.
        let lit = response.hovered() || focus::shown(&response);
        focus::hint(ui, &response, rect, Ring::Own);
        if !ui.is_rect_visible(rect) {
            return response;
        }
        let center = rect.center().y;
        let painter = ui.painter();
        if lit {
            painter.rect_filled(rect, CornerRadius::same(shape.corner), palette.selection);
        }
        let ink = if lit && !look.terminal {
            palette.accent_hover
        } else {
            palette.text
        };
        let mark = rect.left() + shape.inset;
        if look.terminal {
            let glyph = if current {
                super::workspace::drawable(ui, TextRole::OBody, look, CURRENT)
                    .map(|glyph| (glyph, palette.success))
            } else if lit {
                Some(("▌", palette.accent))
            } else {
                None
            };
            if let Some((glyph, color)) = glyph {
                Text::one(look, TextRole::OBody, glyph, color)
                    .layout(ui.ctx())
                    .paint_left(painter, mark, center);
            }
        } else if current {
            let place =
                Rect::from_center_size(pos2(mark + shape.gutter / 2.0, center), vec2(12.0, 12.0));
            Icon::Check.image(ink, 12.0).paint_at(ui, place);
        }
        Text::one(look, role, self.text, ink)
            .layout(ui.ctx())
            .paint_left(painter, rect.left() + lead, center);
        if let Some(detail) = self.detail {
            Text::one(look, note, detail, palette.dim)
                .layout(ui.ctx())
                .paint_right(painter, rect.right() - shape.inset, center);
        }
        response
    }
}

/// A menu's rows: no room between them, at least `width` across, and a
/// scroll bar once they are taller than the look lets a menu grow.
fn rows<R>(ui: &mut Ui, width: f32, add: impl FnOnce(&mut Ui) -> R) -> R {
    ui.spacing_mut().item_spacing = egui::Vec2::ZERO;
    ui.set_min_width(width);
    egui::ScrollArea::vertical()
        .max_height(ui.spacing().combo_height)
        .show(ui, add)
        .inner
}

/// The menu `button` opens when clicked: `add`'s rows under it, the panel
/// at least `min_width` wide. `add` runs only while the menu is open.
pub fn under<R>(
    button: &Response,
    min_width: f32,
    look: &Look,
    palette: &Palette,
    add: impl FnOnce(&mut Ui) -> R,
) -> Option<R> {
    let frame = frame(look, palette);
    let inside = min_width - frame.total_margin().sum().x;
    egui::Popup::menu(button)
        .frame(frame)
        .gap(GAP)
        .show(|ui| rows(ui, inside.max(0.0), add))
        .map(|shown| shown.inner)
}

/// The menu a secondary click on `target` opens, at the pointer.
pub fn context<R>(
    target: &Response,
    look: &Look,
    palette: &Palette,
    add: impl FnOnce(&mut Ui) -> R,
) -> Option<R> {
    egui::Popup::context_menu(target)
        .frame(frame(look, palette))
        .show(|ui| rows(ui, 0.0, add))
        .map(|shown| shown.inner)
}

/// One choice of [`choices`].
pub struct Choice {
    /// What the choice reads.
    pub text: String,
    /// Its name for screen readers, when it is not what it reads (the
    /// terminal look's lower case).
    pub name: Option<String>,
    /// The choice in use.
    pub selected: bool,
}

/// The menu that `button` opens when clicked: `choices` under it, at least
/// `min_width` wide. Returns the index of the choice picked this frame.
/// `choices` is asked for only while the menu is open.
///
/// egui closes a menu on a pointer's click; a pick by key or by a screen
/// reader closes it here. After a pick or Escape the keyboard is back on
/// `button`, where it was before the menu opened.
pub fn choices(
    button: &Response,
    min_width: f32,
    look: &Look,
    palette: &Palette,
    choices: impl FnOnce() -> Vec<Choice>,
) -> Option<usize> {
    let mut picked = None;
    let open = under(button, min_width, look, palette, |ui| {
        for (index, choice) in choices().into_iter().enumerate() {
            let item = Item::choice(&choice.text, choice.selected);
            let item = match &choice.name {
                Some(name) => item.name(name),
                None => item,
            };
            if item.show(ui, look, palette).clicked() {
                picked = Some(index);
                ui.close();
            }
        }
    });
    let escaped = || {
        button
            .ctx
            .input(|input| input.key_pressed(egui::Key::Escape))
    };
    if open.is_some() && (picked.is_some() || escaped()) {
        button.request_focus();
    }
    picked
}

/// A choice that reads `text` and sets `value` to `own` when picked. The
/// menu closes on the pick, whatever made it.
pub fn value<T: PartialEq>(
    ui: &mut Ui,
    value: &mut T,
    own: T,
    text: &str,
    look: &Look,
    palette: &Palette,
) -> Response {
    let response = Item::choice(text, *value == own).show(ui, look, palette);
    if response.clicked() {
        *value = own;
        ui.close();
    }
    response
}
```

Notes for whoever reads this cold:
- `Popup::menu` lays its rows out top down and justified, so `allocate_at_least` hands each row the menu's whole width; `wanted` is only what the row needs.
- `ui.close()` inside the scroll area closes the popup: egui walks up to the nearest closable container.
- `super::workspace::drawable` already exists (`pub(super)`): it returns the first of the marks the font has, because the terminal look draws in the desktop's own monospace face.

- [ ] **Step 4: Move the callers, and take the old menu out of `widgets`**

```diff
diff --git a/src/ui/widgets.rs b/src/ui/widgets.rs
--- a/src/ui/widgets.rs
+++ b/src/ui/widgets.rs
@@ -449,58 +449,6 @@ pub fn popup_button<R>(
     inner.inner
 }
 
-/// One choice of a [`popup_menu`].
-pub struct MenuChoice {
-    /// What the choice reads.
-    pub text: String,
-    /// Its name for screen readers, when it is not what it reads (the
-    /// terminal look's lower case).
-    pub name: Option<String>,
-    /// The choice in use.
-    pub selected: bool,
-}
-
-/// The menu that `button` opens when clicked: `choices` under it, at least
-/// `min_width` wide. Returns the index of the choice picked this frame.
-/// `choices` is asked for only while the menu is open.
-///
-/// egui closes a menu on a pointer's click; a pick by key or by a screen
-/// reader closes it here. After a pick or Escape the keyboard is back on
-/// `button`, where it was before the menu opened.
-pub fn popup_menu(
-    button: &Response,
-    min_width: f32,
-    look: &Look,
-    choices: impl FnOnce() -> Vec<MenuChoice>,
-) -> Option<usize> {
-    let mut picked = None;
-    let open = egui::Popup::menu(button).show(|ui| {
-        ui.set_min_width(min_width);
-        for (index, choice) in choices().into_iter().enumerate() {
-            let text = galley(ui, &choice.text, Color32::PLACEHOLDER, look);
-            let response = ui.add(egui::Button::selectable(choice.selected, text));
-            if let Some(name) = &choice.name {
-                response.widget_info(|| {
-                    WidgetInfo::selected(WidgetType::Button, true, choice.selected, name)
-                });
-            }
-            if response.clicked() {
-                picked = Some(index);
-                ui.close();
-            }
-        }
-    });
-    let escaped = || {
-        button
-            .ctx
-            .input(|input| input.key_pressed(egui::Key::Escape))
-    };
-    if open.is_some() && (picked.is_some() || escaped()) {
-        button.request_focus();
-    }
-    picked
-}
-
 /// Up and down chevrons, centred in `rect`: the mark of a macOS pop-up
 /// button.
 fn paint_chevrons(painter: &egui::Painter, rect: Rect, color: Color32) {
```

```diff
diff --git a/src/ui/sidebar.rs b/src/ui/sidebar.rs
--- a/src/ui/sidebar.rs
+++ b/src/ui/sidebar.rs
@@ -12,6 +12,7 @@ use crate::theme::{Icon, Look, Palette};
 use crate::typography::{Text, TextRole};
 use crate::ui::focus;
 use crate::ui::format::display_safe;
+use crate::ui::menu;
 use crate::ui::widgets::{self, ButtonSpec, icon_button};
 
 /// The sidebar's width when it opens, per look: the design's 264 and 248
@@ -576,10 +577,10 @@ fn schema_header(
                 .image(palette.secondary, 10.0)
                 .paint_at(ui, Rect::from_center_size(glyph, vec2(10.0, 10.0)));
         }
-        let picked = widgets::popup_menu(&response, 160.0, look, || {
+        let picked = menu::choices(&response, 160.0, look, palette, || {
             schemas
                 .iter()
-                .map(|other| widgets::MenuChoice {
+                .map(|other| menu::Choice {
                     text: display_safe(other).into_owned(),
                     name: None,
                     selected: Some(other.as_str()) == shown,
```

```diff
diff --git a/src/ui/sql_editor.rs b/src/ui/sql_editor.rs
--- a/src/ui/sql_editor.rs
+++ b/src/ui/sql_editor.rs
@@ -14,7 +14,8 @@ use crate::settings::Settings;
 use crate::theme::{Icon, Look, Palette};
 use crate::typography::{Text, TextRole};
 use crate::ui::format;
-use crate::ui::widgets::{self, ButtonSpec, MenuChoice};
+use crate::ui::menu;
+use crate::ui::widgets::{self, ButtonSpec};
 
 /// Left and right padding of the toolbar.
 const SIDE: f32 = 16.0;
@@ -381,7 +382,7 @@ fn menus(
         || {
             Settings::SQL_LIMITS
                 .iter()
-                .map(|choice| MenuChoice {
+                .map(|choice| menu::Choice {
                     text: limit_text(*choice, false, look, locale),
                     name: Some(limit_name(*choice, locale)),
                     selected: *choice == bar.limit,
@@ -407,7 +408,7 @@ fn menus(
         || {
             Settings::SQL_TIMEOUTS
                 .iter()
-                .map(|choice| MenuChoice {
+                .map(|choice| menu::Choice {
                     text: timeout_text(*choice, false, look, locale),
                     name: Some(timeout_name(*choice, locale)),
                     selected: *choice == bar.secs,
@@ -752,7 +753,7 @@ fn menu(
     label: &MenuLabel,
     shape: MenuShape,
     (look, palette): (&Look, &Palette),
-    choices: impl FnOnce() -> Vec<MenuChoice>,
+    choices: impl FnOnce() -> Vec<menu::Choice>,
 ) -> Option<usize> {
     let response = ui.interact(rect, ui.id().with(("menu", &label.name)), Sense::click());
     response.widget_info(|| {
@@ -812,7 +813,7 @@ fn menu(
     } else {
         response
     };
-    widgets::popup_menu(&response, rect.width(), look, choices)
+    menu::choices(&response, rect.width(), look, palette, choices)
 }
 
 /// Whether the last run ended and ran something. A run that failed as a
```

- [ ] **Step 5: Run the tests to see them pass**

Run: `~/.cargo/bin/cargo fmt --all && ~/.cargo/bin/cargo test --locked --lib ui::menu`

Expected: `test result: ok. 3 passed`.

- [ ] **Step 6: Run the full checks**

Run the five commands under "Full check commands". Expected: no output from fmt, clippy and doc; the library's tests end `test result: ok. 1680 passed; 0 failed; 1 ignored`.

- [ ] **Step 7: Commit point**

Stop and report. If asked to commit:

```bash
git add src/ui/menu.rs src/ui/mod.rs src/ui/widgets.rs src/ui/sidebar.rs src/ui/sql_editor.rs
git commit -S -m "Draw menus of choices as the Components sheet draws them" -m "One panel and one row for a menu, in src/ui/menu.rs: the choice in use takes a check mark and never a fill, the pointer and the keyboard light a row the same way, and rows touch. The SQL editor's Limit and Timeout menus and the sidebar's schema menu use it." -m "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 2: The dropdown

A field with `▾` that opens a menu of choices, replacing egui's `ComboBox` in the filter bar, the settings sheet and the connection sheet.

**Files:**
- Modify: `src/ui/menu.rs`, `src/ui/widgets.rs`, `src/theme.rs`, `src/ui/filter_bar.rs`, `src/ui/settings/sheet.rs`, `src/ui/connect_dialog/sheet.rs`, `src/ui/connect_dialog/mod.rs`
- Test: `src/ui/menu.rs`

- [ ] **Step 1: Write the failing tests**

Add both to the end of the `tests` module in `src/ui/menu.rs`. The first is the pop-up button's test from `widgets`, for the dropdown.

```rust
    #[test]
    fn a_dropdown_keeps_its_name_and_value_in_every_look() {
        for look in Look::ALL {
            let mut harness = Harness::new();
            harness.set_look(look);
            let palette = harness.app.palette;
            let tree = harness.frame_with(|ui| {
                let dropdown = super::Dropdown::new("database", "bookshop_test", 160.0);
                let response = dropdown.show(ui, &look, &palette, |_| {}).response;
                assert_eq!(response.rect.width(), 160.0, "{}", look.name);
                response.widget_info(|| {
                    let mut info =
                        egui::WidgetInfo::labeled(egui::WidgetType::ComboBox, true, "Database");
                    info.current_text_value = Some("bookshop_test".into());
                    info
                });
            });
            let id =
                node(&tree, "Database", Role::ComboBox).unwrap_or_else(|| panic!("{}", look.name));
            let (_, node) = tree.nodes.iter().find(|(n, _)| *n == id).unwrap();
            assert_eq!(node.value(), Some("bookshop_test"), "{}", look.name);
            assert_eq!(harness.painted_color("bookshop_test"), Some(palette.text));
        }
    }

    #[test]
    fn a_pick_in_a_dropdown_sets_the_value_and_closes_its_menu() {
        for look in Look::ALL {
            let mut harness = Harness::new();
            harness.set_look(look);
            let (tab, id) = harness.editable();
            harness.press(Key::F, Modifiers::COMMAND);
            harness.click("Filter operator");
            assert!(harness.has("contains"), "the menu opens in {}", look.name);
            harness.click("contains");
            assert!(
                !harness.has("starts with"),
                "the menu stays open in {}",
                look.name
            );
            // The row that had the keyboard is gone: the dropdown has it.
            assert_eq!(
                focused_name(&harness.settle()),
                "Filter operator",
                "{}",
                look.name
            );
            let workspace = harness.app.workspace(tab).unwrap();
            let rows = &workspace.object_tab(id).unwrap().filter.rows;
            assert_eq!(
                rows[0].op,
                tabletist_db::FilterOp::Contains,
                "{}",
                look.name
            );
        }
    }
```

- [ ] **Step 2: Run the tests to see them fail**

Run: `~/.cargo/bin/cargo test --locked --lib ui::menu`

Expected: it does not compile: `Dropdown` is not found in `super`. (With the first test left out, the second fails with "the menu stays open in standard": a combo box stays open after a pick that no pointer made, and the keyboard is not on it.)

- [ ] **Step 3: Write the dropdown**

In `src/ui/menu.rs` the imports gain `Color32` and `StrokeKind`:

```rust
use egui::{
    Color32, CornerRadius, Rect, Response, Sense, Stroke, StrokeKind, Ui, WidgetInfo, WidgetType,
    pos2, vec2,
};
```

and after `value`, before `#[cfg(test)]`:

```rust
/// A dropdown: a field that reads the choice in use, with `▾` at its end,
/// and opens a menu of the choices under it.
pub struct Dropdown<'a> {
    id: egui::Id,
    text: &'a str,
    width: f32,
    height: Option<f32>,
    fill: Option<Color32>,
}

impl<'a> Dropdown<'a> {
    /// A dropdown `width` wide that reads `text`. `salt` tells it from the
    /// others in its `Ui`.
    pub fn new(salt: impl egui::AsIdSalt, text: &'a str, width: f32) -> Self {
        Self {
            id: egui::Id::new(salt),
            text,
            width,
            height: None,
            fill: None,
        }
    }

    /// As tall as the fields beside it, where they are not the look's
    /// controls' height.
    pub fn height(mut self, height: f32) -> Self {
        self.height = Some(height);
        self
    }

    /// The fill of the fields beside it, where it is not the window's.
    pub fn fill(mut self, fill: Color32) -> Self {
        self.fill = Some(fill);
        self
    }

    /// Draws the field, and `add`'s rows in its menu while that is open.
    /// It is announced as a combo box whose value is what it reads: name
    /// it with a `widget_info` of the caller's own, or `labelled_by`.
    pub fn show<R>(
        self,
        ui: &mut Ui,
        look: &Look,
        palette: &Palette,
        add: impl FnOnce(&mut Ui) -> R,
    ) -> egui::InnerResponse<Option<R>> {
        let height = self.height.unwrap_or(look.control_height);
        let (_, rect) = ui.allocate_space(vec2(self.width, height));
        let response = ui.interact(rect, ui.id().with(self.id), Sense::click());
        response.widget_info(|| {
            let mut info = WidgetInfo::new(WidgetType::ComboBox);
            info.enabled = ui.is_enabled();
            info.current_text_value = Some(self.text.to_owned());
            info
        });
        // The terminal: an outline with corners of 3. Elsewhere a field's
        // box, corners one tighter than a button's.
        let (fill, border, radius, line) = if look.terminal {
            (Color32::TRANSPARENT, palette.outline, 3, 1.0)
        } else {
            (
                self.fill.unwrap_or(palette.window),
                palette.border,
                look.radius.saturating_sub(1),
                widgets::hairline(ui),
            )
        };
        if ui.is_rect_visible(rect) {
            let open =
                egui::Popup::is_id_open(ui.ctx(), egui::Popup::default_response_id(&response));
            // A step toward the text under the pointer, and while open.
            let border = if response.hovered() || open {
                border.lerp_to_gamma(palette.text, 0.1)
            } else {
                border
            };
            let corner = CornerRadius::same(radius);
            let painter = ui.painter();
            painter.rect(
                rect,
                corner,
                fill,
                Stroke::new(line, border),
                StrokeKind::Inside,
            );
            // The border, then a field's padding: 8 in the terminal, 10.
            let inset = if look.terminal { 9.0 } else { 11.0 };
            let center = rect.center().y;
            let mark = if look.terminal {
                let mark = Text::one(look, widgets::body(look), "▾", palette.dim).layout(ui.ctx());
                let width = mark.paint_right(painter, rect.right() - inset, center);
                rect.right() - inset - width
            } else {
                let place = Rect::from_center_size(
                    pos2(rect.right() - inset - 5.0, center),
                    vec2(10.0, 10.0),
                );
                Icon::ChevronDown
                    .image(palette.dim, 10.0)
                    .paint_at(ui, place);
                place.left()
            };
            // What does not fit ends at the mark.
            let words = Rect::from_min_max(
                pos2(rect.left() + inset, rect.top()),
                pos2(mark - 6.0, rect.bottom()),
            );
            Text::one(look, widgets::body(look), self.text, palette.text)
                .layout(ui.ctx())
                .paint_left(
                    &painter.with_clip_rect(words.intersect(ui.clip_rect())),
                    words.left(),
                    center,
                );
        }
        focus::hint(ui, &response, rect, Ring::Field { radius });
        let inner = under(&response, rect.width(), look, palette, add);
        // The menu took the keyboard down its rows. Once a key or a screen
        // reader closes it, by a pick or by Escape, the keyboard is back on
        // the field, as after [`choices`]. A pointer that closed it put the
        // keyboard where it clicked.
        let popup = egui::Popup::default_response_id(&response);
        let (escaped, clicked) = ui.input(|input| {
            (
                input.key_pressed(egui::Key::Escape),
                input.pointer.any_click(),
            )
        });
        let closed = !egui::Popup::is_id_open(ui.ctx(), popup);
        if inner.is_some() && (escaped || (closed && !clicked)) {
            response.request_focus();
        }
        egui::InnerResponse::new(inner, response)
    }
}
```

- [ ] **Step 4: Move the filter bar**

```diff
diff --git a/src/ui/filter_bar.rs b/src/ui/filter_bar.rs
--- a/src/ui/filter_bar.rs
+++ b/src/ui/filter_bar.rs
@@ -8,6 +8,7 @@ use crate::i18n::gettext;
 use crate::model::{Action, ConnTabId, TabId};
 use crate::theme::Icon;
 use crate::ui::format::display_safe;
+use crate::ui::menu;
 use crate::ui::widgets::{self, icon_button};
 
 pub const FILTER_OPS: [FilterOp; 11] = [
@@ -86,26 +87,22 @@ pub fn show(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId, object_tab: TabId)
     for index in 0..count {
         let row = &mut bar.rows[index];
         ui.horizontal(|ui| {
-            let combo = crate::ui::widgets::popup_button(
-                ui,
-                egui::ComboBox::from_id_salt(("filter-column", tab.0, object_tab.0, index))
-                    .selected_text(widgets::galley(
-                        ui,
-                        &display_safe(&row.column),
-                        egui::Color32::PLACEHOLDER,
-                        &look,
-                    ))
-                    .width(160.0),
-                &look,
-                &palette,
-                |ui| {
-                    for column in &columns {
-                        let name = display_safe(column);
-                        let text = widgets::galley(ui, &name, egui::Color32::PLACEHOLDER, &look);
-                        ui.selectable_value(&mut row.column, column.clone(), text);
-                    }
-                },
-            );
+            let shown = display_safe(&row.column).into_owned();
+            let combo =
+                menu::Dropdown::new(("filter-column", tab.0, object_tab.0, index), &shown, 160.0)
+                    .show(ui, &look, &palette, |ui| {
+                        for column in &columns {
+                            let name = display_safe(column);
+                            menu::value(
+                                ui,
+                                &mut row.column,
+                                column.clone(),
+                                &name,
+                                &look,
+                                &palette,
+                            );
+                        }
+                    });
             // No visible label in the row: name it for screen readers,
             // keeping its selection as the value.
             let selected = Some(display_safe(&row.column).into_owned());
@@ -118,26 +115,17 @@ pub fn show(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId, object_tab: TabId)
                 info.current_text_value = selected.clone();
                 info
             });
-            let combo = crate::ui::widgets::popup_button(
-                ui,
-                egui::ComboBox::from_id_salt(("filter-op", tab.0, object_tab.0, index))
-                    .selected_text(widgets::galley(
-                        ui,
-                        &gettext(locale, op_label(row.op)),
-                        egui::Color32::PLACEHOLDER,
-                        &look,
-                    ))
-                    .width(110.0),
-                &look,
-                &palette,
-                |ui| {
-                    for op in FILTER_OPS {
-                        let label = gettext(locale, op_label(op));
-                        let text = widgets::galley(ui, &label, egui::Color32::PLACEHOLDER, &look);
-                        ui.selectable_value(&mut row.op, op, text);
-                    }
-                },
-            );
+            let combo = menu::Dropdown::new(
+                ("filter-op", tab.0, object_tab.0, index),
+                &gettext(locale, op_label(row.op)),
+                110.0,
+            )
+            .show(ui, &look, &palette, |ui| {
+                for op in FILTER_OPS {
+                    let label = gettext(locale, op_label(op));
+                    menu::value(ui, &mut row.op, op, &label, &look, &palette);
+                }
+            });
             // No visible label in the row: name it for screen readers,
             // keeping its selection as the value.
             let selected = Some(gettext(locale, op_label(row.op)).into_owned());
```

- [ ] **Step 5: Move the settings sheet**

```diff
diff --git a/src/ui/settings/sheet.rs b/src/ui/settings/sheet.rs
--- a/src/ui/settings/sheet.rs
+++ b/src/ui/settings/sheet.rs
@@ -13,6 +13,7 @@ use crate::theme::{Look, Palette};
 use crate::typography::{Laid, Text, TextRole};
 use crate::ui::focus::{self, Ring};
 use crate::ui::format::group_digits;
+use crate::ui::menu;
 use crate::ui::value_tags::slot_colors;
 use crate::ui::widgets::{self, Segment};
 
@@ -349,23 +350,19 @@ fn control(
     let name = skin.say(label(option));
     match option {
         OptionId::PageSize => {
-            let written = |ui: &Ui, size: u32| {
-                let text = group_digits(u64::from(size));
-                widgets::galley(ui, &text, egui::Color32::PLACEHOLDER, look)
-            };
-            let combo = egui::ComboBox::from_id_salt("page-size")
-                .width(MENU)
-                .selected_text(written(ui, settings.page_size));
-            let menu = widgets::popup_button(ui, combo, look, palette, |ui| {
+            let written = |size: u32| group_digits(u64::from(size));
+            let size = written(settings.page_size);
+            let dropdown = menu::Dropdown::new("page-size", &size, MENU);
+            let menu = dropdown.show(ui, look, palette, |ui| {
                 for size in page_sizes(settings.page_size) {
                     let chosen = size == settings.page_size;
-                    let entry = ui.add(egui::Button::selectable(chosen, written(ui, size)));
                     // Named as the SQL editor's Limit menu names its
                     // entries: what it sets, and to what.
-                    let entry_name = format!("{name} {}", group_digits(u64::from(size)));
-                    entry.widget_info(|| {
-                        WidgetInfo::selected(WidgetType::Button, true, chosen, &entry_name)
-                    });
+                    let text = written(size);
+                    let entry_name = format!("{name} {text}");
+                    let entry = menu::Item::choice(&text, chosen)
+                        .name(&entry_name)
+                        .show(ui, look, palette);
                     if entry.clicked() {
                         actions.push(Action::SetOption(OptionValue::PageSize(size)));
                         // egui closes the menu on a pointer's click; a
```

- [ ] **Step 6: Move the connection sheet**

```diff
diff --git a/src/ui/connect_dialog/sheet.rs b/src/ui/connect_dialog/sheet.rs
--- a/src/ui/connect_dialog/sheet.rs
+++ b/src/ui/connect_dialog/sheet.rs
@@ -12,6 +12,7 @@ use crate::i18n::gettext;
 use crate::model::{Action, ConnectionForm, SshAuthKind};
 use crate::theme::{self, Icon};
 use crate::typography::{Text, TextRole};
+use crate::ui::menu;
 use crate::ui::widgets::{self, ButtonSpec};
 
 use super::choice::{Choice, Group, choose, driver_choice, environment_choice};
@@ -181,40 +182,19 @@ fn keyring_box(
     );
 }
 
-/// macOS: a list of choices drawn as the designs draw one, a field with a
-/// mark at its right: the look's own up and down chevrons, or one pointing
-/// down. It relies on [`super::style_controls`] having set the fields'
-/// fill, border and height on `ui`: it takes them as they are.
-fn select<R>(
-    ui: &mut Ui,
-    combo: egui::ComboBox,
+/// macOS: a dropdown `width` wide that reads `text`, as tall as the
+/// dialog's fields and filled as they are. It relies on
+/// [`super::style_controls`] having set that fill on `ui`.
+fn select<'a>(
+    ui: &Ui,
+    salt: &'static str,
+    text: &'a str,
+    width: f32,
     skin: &Skin,
-    contents: impl FnOnce(&mut Ui) -> R,
-) -> egui::InnerResponse<Option<R>> {
-    let icon = if skin.look.raised_popups {
-        Icon::ChevronsUpDown
-    } else {
-        Icon::ChevronDown
-    };
-    ui.scope(|ui| {
-        // The fields' fill while it rests. Under the pointer, and with
-        // the keyboard on it, it keeps the look's own, which its menu's
-        // rows share.
-        let fill = ui.visuals().extreme_bg_color;
-        let states = &mut ui.visuals_mut().widgets;
-        states.inactive.weak_bg_fill = fill;
-        states.open.weak_bg_fill = fill;
-        // Its text starts where a field's does.
-        ui.spacing_mut().button_padding.x = f32::from(skin.text_inset());
-        combo
-            .icon(move |ui, rect, visuals, _open| {
-                let place = Rect::from_center_size(rect.center(), vec2(14.0, 14.0));
-                icon.image(visuals.fg_stroke.color, 14.0)
-                    .paint_at(ui, place);
-            })
-            .show_ui(ui, contents)
-    })
-    .inner
+) -> menu::Dropdown<'a> {
+    menu::Dropdown::new(salt, text, width)
+        .height(skin.field_height())
+        .fill(ui.visuals().extreme_bg_color)
 }
 
 /// macOS: a group of fields in a hairline box, its heading set into the
@@ -411,19 +391,13 @@ fn security(ui: &mut Ui, form: &mut ConnectionForm, skin: &Skin, actions: &mut V
             if column == 0 {
                 let width = ui.available_width();
                 labelled(ui, &skin.say("SSL mode"), small, skin, |ui| {
-                    select(
-                        ui,
-                        egui::ComboBox::from_id_salt("tls-mode")
-                            .width(width)
-                            .selected_text(tls_label(form.tls)),
-                        skin,
-                        |ui| {
+                    select(ui, "tls-mode", tls_label(form.tls), width, skin)
+                        .show(ui, look, palette, |ui| {
                             for (mode, label) in TLS_MODES {
-                                ui.selectable_value(&mut form.tls, mode, label);
+                                menu::value(ui, &mut form.tls, mode, label, look, palette);
                             }
-                        },
-                    )
-                    .response
+                        })
+                        .response
                 });
             } else {
                 let label = widgets::label(
@@ -510,23 +484,21 @@ fn ssh_fields(ui: &mut Ui, form: &mut ConnectionForm, skin: &Skin, actions: &mut
             let width = ui.available_width();
             let name = gettext(skin.locale, "Authentication");
             let value = gettext(skin.locale, form.ssh_auth.label());
-            let response = select(
-                ui,
-                egui::ComboBox::from_id_salt("ssh-auth")
-                    .width(width)
-                    .selected_text(&*value),
-                skin,
-                |ui| {
+            let response = select(ui, "ssh-auth", &value, width, skin)
+                .show(ui, skin.look, skin.palette, |ui| {
                     for kind in SshAuthKind::ALL {
-                        ui.selectable_value(
+                        let label = gettext(skin.locale, kind.label());
+                        menu::value(
+                            ui,
                             &mut form.ssh_auth,
                             kind,
-                            gettext(skin.locale, kind.label()),
+                            &label,
+                            skin.look,
+                            skin.palette,
                         );
                     }
-                },
-            )
-            .response;
+                })
+                .response;
             response.widget_info(|| {
                 let mut info = WidgetInfo::labeled(WidgetType::ComboBox, true, &*name);
                 info.current_text_value = Some(value.to_string());
```

```diff
diff --git a/src/ui/connect_dialog/mod.rs b/src/ui/connect_dialog/mod.rs
--- a/src/ui/connect_dialog/mod.rs
+++ b/src/ui/connect_dialog/mod.rs
@@ -419,8 +419,9 @@ fn frame(skin: &Skin) -> egui::Frame {
     }
 }
 
-/// Fields and pop-up buttons as the designs draw them: boxes with a border
-/// on the field colour, and no spacing but what the layout adds.
+/// Fields as the designs draw them: boxes with a border on the field
+/// colour, and no spacing but what the layout adds. A dropdown takes the
+/// fill from here.
 fn style_controls(ui: &mut Ui, skin: &Skin) {
     let Skin { look, palette, .. } = *skin;
     let (fill, border, radius, width) = if look.terminal {
```

- [ ] **Step 7: Take the pop-up button out of `widgets` and the look**

```diff
diff --git a/src/ui/widgets.rs b/src/ui/widgets.rs
--- a/src/ui/widgets.rs
+++ b/src/ui/widgets.rs
@@ -374,9 +374,9 @@ pub fn track_fill(palette: &Palette) -> Color32 {
     palette.surface
 }
 
-/// The fill of something raised off its bar (a raised tab, a pop-up
-/// button): the window colour, which is lighter than the bar in a light
-/// palette. In a dark palette the window is darker than the bar and would
+/// The fill of something raised off its bar (a raised tab, the list of
+/// completions): the window colour, which is lighter than the bar in a
+/// light palette. In a dark palette the window is darker than the bar and would
 /// read as a slot, so it takes the lightest surface instead.
 pub fn raised_fill(palette: &Palette) -> Color32 {
     if palette.dark {
@@ -406,72 +406,6 @@ pub fn active_tab_fill(look: &Look, palette: &Palette) -> Color32 {
     }
 }
 
-/// A pop-up button: `combo` with `contents` as its menu. With
-/// `look.raised_popups` it is drawn as macOS draws one: raised off its
-/// background with a soft shadow and a hairline, and marked with up and
-/// down chevrons. Other looks keep egui's combo box.
-pub fn popup_button<R>(
-    ui: &mut Ui,
-    combo: egui::ComboBox,
-    look: &Look,
-    palette: &Palette,
-    contents: impl FnOnce(&mut Ui) -> R,
-) -> egui::InnerResponse<Option<R>> {
-    if !look.raised_popups {
-        return combo.show_ui(ui, contents);
-    }
-    let shadow = ui.painter().add(egui::Shape::Noop);
-    let fill = raised_fill(palette);
-    let rim = Stroke::new(1.0, palette.text.gamma_multiply(0.1));
-    let inner = ui.scope(|ui| {
-        let widgets = &mut ui.visuals_mut().widgets;
-        for widget in [
-            &mut widgets.inactive,
-            &mut widgets.hovered,
-            &mut widgets.open,
-        ] {
-            widget.weak_bg_fill = fill;
-            widget.bg_stroke = rim;
-        }
-        widgets.active.bg_stroke = rim;
-        combo
-            .icon(|ui, rect, visuals, _open| {
-                paint_chevrons(ui.painter(), rect, visuals.fg_stroke.color);
-            })
-            .show_ui(ui, contents)
-    });
-    let rect = inner.inner.response.rect;
-    if ui.is_rect_visible(rect) {
-        let corner = ui.visuals().widgets.inactive.corner_radius;
-        ui.painter()
-            .set(shadow, raised_shadow(palette).as_shape(rect, corner));
-    }
-    inner.inner
-}
-
-/// Up and down chevrons, centred in `rect`: the mark of a macOS pop-up
-/// button.
-fn paint_chevrons(painter: &egui::Painter, rect: Rect, color: Color32) {
-    let center = rect.center();
-    let half_width = rect.width() * 0.25;
-    let rise = half_width * 0.75;
-    let gap = rect.height() * 0.1;
-    let stroke = Stroke::new(1.5, color);
-    // Up, then down: the wings sit `gap` off the centre, the tip `rise`
-    // further out.
-    for direction in [-1.0, 1.0] {
-        let wings = center.y + direction * gap;
-        painter.line(
-            vec![
-                egui::pos2(center.x - half_width, wings),
-                egui::pos2(center.x, wings + direction * rise),
-                egui::pos2(center.x + half_width, wings),
-            ],
-            stroke,
-        );
-    }
-}
-
 /// `text` in the body role as a galley for a widget egui draws (a combo
 /// box's value, a menu item, a field's hint).
 pub fn galley(ui: &Ui, text: &str, color: Color32, look: &Look) -> std::sync::Arc<egui::Galley> {
@@ -1953,32 +1887,6 @@ mod tests {
         }
     }
 
-    #[test]
-    fn pop_up_buttons_keep_their_name_and_value_in_every_look() {
-        for look in crate::theme::Look::ALL {
-            let mut harness = crate::testing::Harness::new();
-            harness.set_look(look);
-            let palette = harness.app.palette;
-            let tree = harness.frame_with(|ui| {
-                let mut chosen = "bookshop_test".to_owned();
-                let combo = egui::ComboBox::from_id_salt("database").selected_text(&chosen);
-                let response = super::popup_button(ui, combo, &look, &palette, |ui| {
-                    ui.selectable_value(&mut chosen, "postgres".into(), "postgres");
-                })
-                .response;
-                response.widget_info(|| {
-                    let mut info = WidgetInfo::labeled(WidgetType::ComboBox, true, "Database");
-                    info.current_text_value = Some("bookshop_test".into());
-                    info
-                });
-            });
-            let id = crate::testing::node(&tree, "Database", egui::accesskit::Role::ComboBox)
-                .unwrap_or_else(|| panic!("{}", look.name));
-            let (_, node) = tree.nodes.iter().find(|(n, _)| *n == id).unwrap();
-            assert_eq!(node.value(), Some("bookshop_test"), "{}", look.name);
-        }
-    }
-
     #[test]
     fn omarchy_dialogs_use_a_scrim_and_an_accent_border() {
         use crate::theme::{Look, Palette};
```

```diff
diff --git a/src/theme.rs b/src/theme.rs
--- a/src/theme.rs
+++ b/src/theme.rs
@@ -378,9 +378,6 @@ pub struct Look {
     /// the content by 1 px lines. The sidebar and the row panel always are:
     /// one tone off the content is too faint to mark a pane's edge.
     pub panel_separators: bool,
-    /// Pop-up buttons (combo boxes) stand off their background with a
-    /// raised fill and a soft shadow, and show up and down chevrons.
-    pub raised_popups: bool,
     pub data_font: DataFont,
     pub faces: Faces,
     pub primary: PrimaryStyle,
@@ -409,7 +406,6 @@ impl Look {
             capsule_search: false,
             sidebar_tinted: true,
             panel_separators: true,
-            raised_popups: false,
             data_font: DataFont::Proportional,
             faces: Faces::Inter,
             primary: PrimaryStyle::Accent,
@@ -435,7 +431,6 @@ impl Look {
             capsule_search: false,
             sidebar_tinted: true,
             panel_separators: true,
-            raised_popups: true,
             data_font: DataFont::Monospace,
             faces: Faces::Plex,
             primary: PrimaryStyle::Ink,
@@ -461,7 +456,6 @@ impl Look {
             capsule_search: false,
             sidebar_tinted: false,
             panel_separators: true,
-            raised_popups: false,
             data_font: DataFont::Monospace,
             faces: Faces::Terminal,
             primary: PrimaryStyle::Outline,
```

- [ ] **Step 8: Run the tests to see them pass**

Run: `~/.cargo/bin/cargo fmt --all && ~/.cargo/bin/cargo test --locked --lib ui::menu`

Expected: `test result: ok. 5 passed`.

- [ ] **Step 9: Run the full checks**

Expected: no output from fmt, clippy and doc; the library's tests end `test result: ok. 1681 passed; 0 failed; 1 ignored` (two tests added, the pop-up button's one moved here).

- [ ] **Step 10: Commit point**

Stop and report. If asked to commit:

```bash
git add src/ui/menu.rs src/ui/widgets.rs src/theme.rs src/ui/filter_bar.rs src/ui/settings/sheet.rs src/ui/connect_dialog/sheet.rs src/ui/connect_dialog/mod.rs
git commit -S -m "Draw dropdowns as fields that open a menu" -m "A dropdown is a field with one chevron at its end, as the Components sheet draws it, in place of the raised pop-up button with up and down chevrons. The filter bar, the settings sheet and the connection sheet use it. Their menus close on a pick that no pointer made, and the keyboard is back on the dropdown." -m "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 3: The menus built by hand

The connection bar's database menu, the connection sheet's `~/.ssh/config` hosts, and a saved connection's two menus.

**Files:**
- Modify: `src/ui/workspace.rs`, `src/ui/connect_dialog/mod.rs`, `src/ui/picker.rs`
- Test: `src/ui/menu.rs`

- [ ] **Step 1: Write the failing test**

Add after `the_pointer_and_the_keyboard_light_a_row_the_same_way` in `src/ui/menu.rs`:

```rust
    #[test]
    fn the_database_menus_rows_touch() {
        for look in Look::ALL {
            let mut harness = Harness::new();
            harness.set_look(look);
            let tab = harness.connect_fake();
            let workspace = harness.app.workspace_mut(tab).unwrap();
            workspace.spec.database = "tabletist".into();
            workspace.databases.value = Some(vec!["tabletist".into(), "postgres".into()]);
            harness.click("Database");
            let rows = ["tabletist", "postgres"].map(|name| row(&mut harness, name));
            assert!(
                (rows[0].bottom() - rows[1].top()).abs() < 0.5,
                "{rows:?} in {}",
                look.name
            );
        }
    }
```

- [ ] **Step 2: Run the test to see it fail**

Run: `~/.cargo/bin/cargo test --locked --lib ui::menu::tests::the_database`

Expected: FAILED, two rectangles 6 apart (`[69.0 58.0] - [211.2 84.0]`, then `[69.0 90.0] ...`) "in standard".

- [ ] **Step 3: Move the database menu**

```diff
diff --git a/src/ui/workspace.rs b/src/ui/workspace.rs
--- a/src/ui/workspace.rs
+++ b/src/ui/workspace.rs
@@ -11,6 +11,7 @@ use crate::theme::{Icon, Look, Palette};
 use crate::typography::{Text, TextRole};
 use crate::ui::focus::{self, Region};
 use crate::ui::format::display_safe;
+use crate::ui::menu;
 use crate::ui::states;
 use crate::ui::widgets::{self, ButtonSpec};
 
@@ -816,29 +817,16 @@ fn database_menu(
     response: &egui::Response,
     tab: ConnTabId,
     info: &BarInfo,
-    look: &Look,
+    (look, palette): (&Look, &Palette),
     actions: &mut Vec<Action>,
 ) {
     let role = TextRole::pick(look, TextRole::MonoSecondary, TextRole::OSecondary);
-    egui::Popup::menu(response).show(|ui| {
-        ui.set_min_width(response.rect.width());
+    menu::under(response, response.rect.width(), look, palette, |ui| {
         for database in &info.databases {
             // Database names come from the server: nothing hidden.
-            let text = Text::one(
-                look,
-                role,
-                &display_safe(database),
-                egui::Color32::PLACEHOLDER,
-            )
-            .layout(ui.ctx());
-            if ui
-                .add(egui::Button::selectable(
-                    *database == info.database,
-                    text.galley,
-                ))
-                .clicked()
-                && *database != info.database
-            {
+            let name = display_safe(database);
+            let item = menu::Item::choice(&name, *database == info.database).role(role);
+            if item.show(ui, look, palette).clicked() && *database != info.database {
                 actions.push(Action::SwitchDatabase {
                     tab,
                     database: database.clone(),
@@ -1153,7 +1141,7 @@ fn mac_bar(
                     ui,
                     Rect::from_center_size(pos2(rect.right() - 16.0, center), vec2(12.0, 12.0)),
                 );
-                database_menu(&response, tab, info, look, actions);
+                database_menu(&response, tab, info, (look, palette), actions);
             }
         } else if response.clicked() {
             actions.push(Action::ActivateConnTab(chip.tab));
@@ -1420,7 +1408,7 @@ fn terminal_bar(
                     center,
                     Text::one(look, TextRole::OBody, "▾", palette.dim),
                 );
-                database_menu(&response, tab, info, look, actions);
+                database_menu(&response, tab, info, (look, palette), actions);
             }
         } else if response.clicked() {
             actions.push(Action::ActivateConnTab(chip.tab));
```

- [ ] **Step 4: Move the hosts' menu**

```diff
diff --git a/src/ui/connect_dialog/mod.rs b/src/ui/connect_dialog/mod.rs
--- a/src/ui/connect_dialog/mod.rs
+++ b/src/ui/connect_dialog/mod.rs
@@ -20,6 +20,7 @@ use crate::i18n::{Locale, gettext};
 use crate::model::{Action, ConnectionForm, Dialog, SshAuthKind, SshHints, TestState};
 use crate::theme::{self, Faces, Icon, Look, Palette};
 use crate::typography::{Text, TextRole};
+use crate::ui::menu;
 use crate::ui::widgets::{self, ButtonSpec};
 
 use sheet::{body, footer, header};
@@ -797,20 +798,20 @@ fn ssh_host_field(
                 return;
             }
             let button = spec().show(ui, height, look, palette).on_hover_text(&*name);
-            egui::Popup::menu(&button).show(|ui| {
+            menu::under(&button, 0.0, look, palette, |ui| {
                 for host in hosts {
-                    ui.horizontal(|ui| {
-                        if widgets::button(ui, &host.alias, look).clicked() {
-                            actions.push(Action::PickSshHost(host.alias.clone()));
-                            // A menu closes by itself on a pointer's click
-                            // only: the keyboard and a screen reader pick
-                            // too.
-                            ui.close();
-                        }
-                        if let Some(name) = &host.config.host_name {
-                            widgets::label(ui, widgets::secondary(look), name, palette.dim, look);
-                        }
-                    });
+                    let item = menu::Item::action(&host.alias);
+                    let item = match &host.config.host_name {
+                        Some(name) => item.detail(name),
+                        None => item,
+                    };
+                    if item.show(ui, look, palette).clicked() {
+                        actions.push(Action::PickSshHost(host.alias.clone()));
+                        // A menu closes by itself on a pointer's click
+                        // only: the keyboard and a screen reader pick
+                        // too.
+                        ui.close();
+                    }
                 }
             });
         },
```

- [ ] **Step 5: Move a saved connection's menus**

```diff
diff --git a/src/ui/picker.rs b/src/ui/picker.rs
--- a/src/ui/picker.rs
+++ b/src/ui/picker.rs
@@ -9,6 +9,7 @@ use crate::i18n::gettext;
 use crate::model::{Action, ConnTabContent};
 use crate::theme::{self, Icon, Look, Palette};
 use crate::typography::{Text, TextRole};
+use crate::ui::menu;
 use crate::ui::states;
 use crate::ui::widgets::{self, ButtonSpec};
 
@@ -630,7 +631,7 @@ fn row_response(
     rect: Rect,
     connection: &SavedConnection,
     opening: Opening,
-    look: &Look,
+    (look, palette): (&Look, &Palette),
     actions: &mut Vec<Action>,
 ) -> egui::Response {
     let tab = opening.tab;
@@ -660,10 +661,12 @@ fn row_response(
         });
     }
     let id = connection.id.clone();
-    response.context_menu(|ui| {
+    menu::context(&response, look, palette, |ui| {
         let locale = crate::i18n::Locale::default();
         let item = |ui: &mut egui::Ui, text: &'static str| {
-            widgets::button(ui, &gettext(locale, text), look).clicked()
+            menu::Item::action(&gettext(locale, text))
+                .show(ui, look, palette)
+                .clicked()
         };
         // An open connection shows, and can be opened once more.
         if opening.open.is_some() {
@@ -734,7 +737,7 @@ fn mac_row(
         pos2(card.left(), rect.top()),
         pos2(card.right(), rect.bottom()),
     );
-    let response = row_response(ui, rect, connection, opening, look, actions);
+    let response = row_response(ui, rect, connection, opening, (look, palette), actions);
     let is_selected = selected == Some(&connection.id);
     let rows = (card.height() / height).round() as usize;
     // The card's inner corners: its 10 less its border.
@@ -960,13 +963,17 @@ fn mac_row(
         more.hidden_at(ui, more_place)
     };
     let id = connection.id.clone();
-    egui::Popup::menu(&response).show(|ui| {
+    menu::under(&response, 0.0, look, palette, |ui| {
         let duplicate = gettext(locale, "Duplicate");
-        if widgets::button(ui, &duplicate, look).clicked() {
+        if menu::Item::action(&duplicate)
+            .show(ui, look, palette)
+            .clicked()
+        {
             actions.push(Action::DuplicateConnection(id.clone()));
         }
         let delete = format!("{} {}", gettext(locale, "Delete"), connection.name);
-        if widgets::button(ui, &gettext(locale, "Delete"), look)
+        if menu::Item::action(&gettext(locale, "Delete"))
+            .show(ui, look, palette)
             .on_hover_text(&delete)
             .clicked()
         {
@@ -996,7 +1003,7 @@ fn terminal_row(
     } = skin;
     let (rect, _) =
         ui.allocate_exact_size(vec2(ui.available_width(), row_height(look)), Sense::hover());
-    let response = row_response(ui, rect, connection, opening, look, actions);
+    let response = row_response(ui, rect, connection, opening, (look, palette), actions);
     let is_selected = selected == Some(&connection.id);
     let center = rect.center().y;
     if is_selected {
```

- [ ] **Step 6: Check nothing draws a menu by itself any more**

Run: `grep -rn -E 'Popup::(menu|context_menu)|\.context_menu\(|ComboBox::|Button::selectable|selectable_value' src --include='*.rs' | grep -v '^src/ui/menu.rs'`

Expected: no output.

- [ ] **Step 7: Run the tests to see them pass**

Run: `~/.cargo/bin/cargo fmt --all && ~/.cargo/bin/cargo test --locked --lib ui::menu`

Expected: `test result: ok. 6 passed`.

- [ ] **Step 8: Run the full checks**

Expected: no output from fmt, clippy and doc; the library's tests end `test result: ok. 1682 passed; 0 failed; 1 ignored`.

- [ ] **Step 9: Commit point**

Stop and report. If asked to commit:

```bash
git add src/ui/menu.rs src/ui/workspace.rs src/ui/connect_dialog/mod.rs src/ui/picker.rs
git commit -S -m "Draw the remaining menus with the shared panel and row" -m "The connection bar's database menu, the hosts from ~/.ssh/config and a saved connection's menus were built by hand from egui's popup and buttons. They are menu rows now, so every menu in the app is drawn in one place." -m "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 4: Compare with the sheet by eye

Nothing here is committed. It needs a GPU for wgpu, which this machine has.

- [ ] **Step 1: Add throwaway scenes**

Append to `src/shots.rs`. Run none of the full checks while they are there (clippy rejects an item after the tests module):

```rust
#[test]
#[ignore = "renders with wgpu; run with --features shots -- --ignored"]
fn dropdown_shots() {
    let key = |harness: &mut Harness| {
        harness.press(egui::Key::ArrowDown, egui::Modifiers::NONE);
    };
    both("dd-sql-limit", |harness| {
        sql_editor(harness);
        harness.click("Limit");
        key(harness);
    });
    both("dd-filter", |harness| {
        workspace(harness);
        harness.press(egui::Key::F, egui::Modifiers::COMMAND);
        harness.click("Filter operator");
        key(harness);
    });
    both("dd-settings", |harness| {
        harness.app.apply(Action::ShowSettings);
        harness.click("Rows per page");
    });
    both("dd-dialog", |harness| {
        edit_production(harness);
        let tree = harness.settle();
        let combo = tree
            .nodes
            .iter()
            .find(|(_, node)| node.role() == egui::accesskit::Role::ComboBox)
            .map(|(id, _)| *id);
        if let Some(target) = combo {
            harness.frame(vec![egui::Event::AccessKitActionRequest(
                egui::accesskit::ActionRequest {
                    target_tree: egui::accesskit::TreeId::ROOT,
                    target_node: target,
                    action: egui::accesskit::Action::Click,
                    data: None,
                },
            )]);
            key(harness);
        }
    });
    both("dd-hover", |harness| {
        sql_editor(harness);
        harness.click("Timeout");
        let tree = harness.settle();
        let row = crate::testing::bounds(&tree, "Timeout 10 s", egui::accesskit::Role::Button)
            .or_else(|| crate::testing::bounds(&tree, "No timeout", egui::accesskit::Role::Button));
        if let Some(row) = row {
            harness.frame(vec![egui::Event::PointerMoved(row.center())]);
        }
    });
    both("dd-schema", |harness| {
        workspace(harness);
        harness.click("Schema");
    });
    both("dd-database", |harness| {
        workspace(harness);
        harness.click("Database");
    });
}
```

- [ ] **Step 2: Render**

Run: `~/.cargo/bin/cargo test --locked --features shots --lib shots::dropdown_shots -- --ignored`

Expected: `test result: ok. 1 passed`, and 42 files `target/shots/dd-*.png` (seven scenes, three looks, light and dark).

- [ ] **Step 3: Look**

Read `target/shots/dd-hover-macos-light.png`, `dd-hover-macos-dark.png`, `dd-hover-omarchy-dark.png`, `dd-filter-macos-light.png`, `dd-filter-omarchy-dark.png`, `dd-settings-macos-light.png` and `dd-dialog-macos-light.png` beside "What the design says". Check: a check mark and a heavier face on the choice in use, with no fill; the selection's tint and the strong accent on the row under the pointer; rows that touch, 4 in from a hairline panel with a soft shadow; in the Omarchy look an accent border, `▌` on the row under the pointer and a green mark on the choice in use; a dropdown that is a bordered field with one chevron at its end.

- [ ] **Step 4: Remove the scenes**

Run: `git checkout -- src/shots.rs && git status --short`

Expected: `src/shots.rs` is not listed. The PNGs stay under `target/` (ignored); never add them to a commit or a pull request.

- [ ] **Step 5: Hand the window checks to the user**

These sessions have no display. Report these as not run, for the user to try in the real window on macOS and on Omarchy:
- Each menu opens under its button and closes on a click outside, on Escape and on a pick.
- Arrow keys move down a menu's rows, and Enter picks the lit one.
- A long list (the filter bar's columns on a wide table) scrolls inside its menu.
- The connection sheet's SSL mode menu, near the window's bottom edge, opens where it fits.

## Left for later

- The database menu and a saved connection's menus stay open after a pick by key or by a screen reader (they close on a pointer's click). They did before this change too.
- Typing in an open menu: the Omarchy picker's filter line and the macOS type-ahead.
- Text fields against the sheet (the filter bar's value field has no border and takes the surface tone).
- The 30 pt control height of the sheet against the look's 28.

## What the review found

Two things the plan's `Item` left out, found by the review of the pull request and fixed after Task 3, each with a test in `src/ui/menu.rs`:

- **A row's note was drawn and not announced.** The hosts from `~/.ssh/config` read their HostName beside the alias. Before, it was a label of its own that a screen reader heard. `Item::show` now sets it as the row's description (`a_rows_note_is_its_description_for_screen_readers`).
- **The keyboard could go to a row the menu had scrolled away.** A menu scrolls once its rows are taller than `combo_height`, and egui does not bring a widget into view when it takes the keyboard. `Item::show` scrolls the row into view when it gains the keyboard (`the_keyboard_brings_a_row_that_is_scrolled_away_into_view`). The combo boxes before this change did not do it either.
