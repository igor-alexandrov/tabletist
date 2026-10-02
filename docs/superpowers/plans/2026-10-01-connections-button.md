# Connections button Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Put a Connections button at the start of the workspace's connection bar, left of the current connection, that opens the saved connections in a new tab.

**Architecture:** The connection bar is painted by hand in `src/ui/workspace.rs`: `mac_bar` for the macOS and standard looks, `terminal_bar` for Omarchy. Each gets one more hand-placed control before what it draws today, an `ui.interact` rect announced as a button, which pushes the existing `Action::NewConnTab` (what Cmd/Ctrl+O already does). No new state, no new action.

**Tech Stack:** Rust 2024, egui (crmne fork), fastframe-icons. Headless UI tests through `src/testing.rs` (AccessKit).

**Design:** the user's Design canvas (the macOS table view, the Windows draft and the Omarchy half-width tile artboards). It is not in the repository and is not to be added. The values this plan needs are written out below.

---

## Ground rules (read first)

- Work in `/home/igor/Work/tabletist/.claude/worktrees/connections-burger-icon-5dda42`, branch `claude/connections-burger-icon-5dda42`.
- `cargo` through mise is broken here: always run `~/.cargo/bin/cargo`. Never point `CARGO_TARGET_DIR` at `/tmp`.
- **Never commit or push unless the user asks.** Each task ends with a "commit point": stop there and report. If the user has asked to commit: one topic per commit, signed (`git commit -S`). If signing fails, ask before committing unsigned. Never set `SSH_AUTH_SOCK` in git commands. End messages with `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.
- No em dashes anywhere (code, comments, docs, commit messages). Use a full stop, comma, colon or parentheses.
- Tests are behavioural (the button exists, it does its job, it does not sit on its neighbours). Never a design or pixel conformance test: do not assert the design's sizes, offsets or colours.
- Test fixtures use neutral names only (`Fixture`, `Bar check`). Never Safari Portal names.
- Views draw text only through `TextRole`s and the `widgets` helpers; never name a font or size.
- Views do not mutate application state: push an `Action`.
- Comment density: short doc comments on items saying what and why, matching the surrounding code.

Full check commands (used in the last task, and handy any time):

```bash
~/.cargo/bin/cargo fmt --all --check
~/.cargo/bin/cargo clippy --locked --workspace --all-targets -- -D warnings
~/.cargo/bin/cargo clippy --locked --workspace --all-targets --features shots -- -D warnings
~/.cargo/bin/cargo test --locked --workspace --all-targets
RUSTDOCFLAGS='-D warnings' ~/.cargo/bin/cargo doc --locked --workspace --no-deps
```

## What the design says

macOS and Windows artboards, first in the bar's row (the row starts 16 in, items 12 apart):

```html
<button aria-label="Connections" title="Connections · ⌘O"
  style="width: 34px; height: 32px; border-radius: 8px;
         background: rgba(255,255,255,0.7); border: 1px solid {{headerLine}}; color: #4d4c48">
  <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.9">
    <rect x="4" y="3" width="16" height="7" rx="1.5"/><rect x="4" y="14" width="16" height="7" rx="1.5"/>
    <path d="M8 6.5h.01M8 17.5h.01"/></svg></button>
```

The Windows artboard is the same with `title="Connections · Ctrl+O"`.

Omarchy artboards, first in the bar's row (the row starts 12 in, items 12 apart):

```html
<button aria-label="Connections (ctrl+shift+c)" title="Connections · ctrl+shift+c"
  style="height: 24px; padding: 0 8px; border: 1px solid {{connLine}}; border-radius: 3px; color: {{t.fg}}">≡ conn</button>
```

How the design's names map to the code:

| Design | Code |
|---|---|
| `headerLine`, `connLine` | `env.bar_border()` (already `rim` in `mac_bar`, `connection_line` in `terminal_bar`) |
| `rgba(255,255,255,0.7)` | `face(0.7)`, the closure `mac_bar` already has |
| `#4d4c48` | `palette.secondary` (as Disconnect's icon) |
| `border-radius: 8px` | `CornerRadius::same(look.radius)`, as the crumb beside it |
| `t.fg` | `palette.text` |
| 13 px mono | `TextRole::OBody` |

## Decisions (where the design and the app differ)

1. **What a click does.** The design has one window per connection and a Connections window. The app has connection tabs and a picker tab, and Cmd/Ctrl+O already opens the picker in a new tab (`Action::NewConnTab`). The button does the same. The connection it was clicked on stays open in its tab.
2. **The shortcut stays Cmd/Ctrl+O on every look.** The Omarchy artboard names `ctrl+shift+c`; the app binds no such key, so the tooltip names the key that works (`connections · ctrl+o`). The artboard's right-hand `ctrl+shift+c conn` hint is not added: that corner holds `ctrl+shift+w disconnect`.
3. **The icon is Lucide's `server`**, the drawing the design's glyph follows (two stacked boxes, a dot in each). The shared fastframe set does not have it, so it joins Tabletist's own Lucide icons in `assets/icons/` (ISC, covered by the `LICENSE.txt` there).
4. **Hover.** The design draws none. The macOS face goes opaque and its icon to `palette.text`; the terminal box's line takes the connection's colour (`env.base()`).
5. **Out of scope:** the rest of the artboards' bar (the two-line connection box, the other connections beside it). Only the button is added; what the bar draws today moves right to make room.

## File map

| File | Change |
|---|---|
| `assets/icons/server.svg` | New: Lucide `server` |
| `src/theme.rs` | `Icon::Server` |
| `src/ui/workspace.rs` | `connections_hint`; the button in `mac_bar` and in `terminal_bar`; a small tests module |
| `src/ui/mod.rs` | Headless tests: the button opens the picker, keeps clear of the connection; the layout test asks for it |
| `docs/superpowers/specs/2026-09-27-tabletist-design.md` | 5.3 names the button |

---

## Task 1: The macOS and standard bars get the Connections button

**Files:**
- Create: `assets/icons/server.svg`
- Modify: `src/theme.rs` (the `fastframe_icons::icons!` list, near line 689)
- Modify: `src/ui/workspace.rs` (`mac_bar`, lines 277-345; a new `connections_hint` after `label_copy`, near line 568; a tests module at the end)
- Test: `src/ui/mod.rs` (tests module, after `the_top_bar_disconnect_button_returns_to_the_picker`, near line 684)

- [ ] **Step 1: Write the failing test**

In `src/ui/mod.rs`, inside `mod tests`, right after `the_top_bar_disconnect_button_returns_to_the_picker`:

```rust
    #[test]
    fn the_top_bar_connections_button_opens_the_picker_in_a_new_tab() {
        // Task 2 drops this filter.
        for look in crate::theme::Look::ALL
            .into_iter()
            .filter(|look| !look.terminal)
        {
            let mut harness = Harness::new();
            harness.set_look(look);
            let tab = harness.connect_fake();
            harness.click("Connections");
            assert_eq!(harness.app.tabs.len(), 2, "{}", look.name);
            assert!(
                matches!(
                    harness.app.active_tab().content,
                    crate::model::ConnTabContent::Picker(_)
                ),
                "{}",
                look.name
            );
            // The connection stays open in its own tab.
            assert!(harness.app.workspace(tab).is_some(), "{}", look.name);
        }
    }
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `~/.cargo/bin/cargo test --locked -p tabletist --lib the_top_bar_connections_button`
Expected: FAIL, panicking with `nothing labelled "Connections"` (the first build of the worktree takes a few minutes).

- [ ] **Step 3: Add the icon**

Create `assets/icons/server.svg` (Lucide `server`, in the shape of its neighbours):

```svg
<svg
  xmlns="http://www.w3.org/2000/svg"
  width="24"
  height="24"
  viewBox="0 0 24 24"
  fill="none"
  stroke="#ffffff"
  stroke-width="2"
  stroke-linecap="round"
  stroke-linejoin="round"
>
  <rect width="20" height="8" x="2" y="2" rx="2" ry="2" />
  <rect width="20" height="8" x="2" y="14" rx="2" ry="2" />
  <line x1="6" x2="6.01" y1="6" y2="6" />
  <line x1="6" x2="6.01" y1="18" y2="18" />
</svg>
```

In `src/theme.rs`, in the `fastframe_icons::icons!` list, between `PanelRight` and `Table`:

```rust
        PanelRight => "panel-right",
        Server => "server",
        Table => "table-2",
```

- [ ] **Step 4: Add the tooltip helper**

In `src/ui/workspace.rs`, after `label_copy`:

```rust
/// The Connections button's tooltip: its name and the key that does the
/// same, as the look spells both.
fn connections_hint(look: &Look, locale: crate::i18n::Locale) -> String {
    look.label(&format!(
        "{} · {}O",
        gettext(locale, "Connections"),
        look.command_key()
    ))
}
```

And at the end of the file:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_connections_hint_names_the_key_as_the_look_spells_it() {
        let locale = crate::i18n::Locale::default();
        assert_eq!(connections_hint(&Look::macos(), locale), "Connections · ⌘O");
        assert_eq!(
            connections_hint(&Look::standard(), locale),
            "Connections · Ctrl+O"
        );
        assert_eq!(
            connections_hint(&Look::omarchy(), locale),
            "connections · ctrl+o"
        );
    }
}
```

- [ ] **Step 5: Draw the button in `mac_bar`**

In `src/ui/workspace.rs`, change `mac_bar`'s doc comment to:

```rust
/// macOS: the Connections button, a box with the name, host and database
/// (a pop-up of the other databases), the read-only pill, and Disconnect.
```

Right after `let hair = Stroke::new(widgets::hairline(ui), rim);` insert:

```rust
    let corner = CornerRadius::same(look.radius);
    // Connections, 16 in: the way to the saved connections, a face of the
    // bar's own under the server icon.
    let connections = Rect::from_min_size(
        pos2(rect.left() + 16.0, center - 16.0),
        vec2(34.0, 32.0),
    );
    {
        let label = gettext(locale, "Connections");
        let response = ui.interact(connections, ui.id().with("connections"), Sense::click());
        response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, &label));
        let (fill, tint) = if response.hovered() {
            (face(1.0), palette.text)
        } else {
            (face(0.7), palette.secondary)
        };
        ui.painter().rect_filled(connections, corner, fill);
        ui.painter()
            .rect_stroke(connections, corner, hair, StrokeKind::Inside);
        Icon::Server.image(tint, 16.0).paint_at(
            ui,
            Rect::from_center_size(connections.center(), vec2(16.0, 16.0)),
        );
        if response
            .on_hover_text(connections_hint(look, locale))
            .clicked()
        {
            actions.push(Action::NewConnTab);
        }
    }
```

The crumb now starts 12 after the button. Change the crumb's comment and rect:

```rust
    // The crumb, 12 on: name, host, "/", database, and the pop-up's
    // chevrons, 8 apart and 10 in from its edges.
```

```rust
    let crumb = Rect::from_min_size(
        pos2(connections.right() + 12.0, center - 16.0),
        vec2(width, 32.0),
    );
```

Delete the crumb's own `let corner = CornerRadius::same(look.radius);` (the line just before `ui.painter().rect_filled(crumb, corner, face(0.7));`): the one added above serves both.

- [ ] **Step 6: Run the tests to verify they pass**

Run: `~/.cargo/bin/cargo test --locked -p tabletist --lib connections`
Expected: PASS, `the_top_bar_connections_button_opens_the_picker_in_a_new_tab` and `the_connections_hint_names_the_key_as_the_look_spells_it` among them.

- [ ] **Step 7: Commit point**

Stop and report. If asked to commit:

```bash
git add assets/icons/server.svg src/theme.rs src/ui/workspace.rs src/ui/mod.rs
git commit -S -m "Lead the connection bar with a Connections button

It opens the saved connections in a new tab, as Cmd/Ctrl+O does.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

## Task 2: The Omarchy bar gets `≡ conn`

**Files:**
- Modify: `src/ui/workspace.rs` (`terminal_bar`, lines 468-487 before Task 1's insertions)
- Test: `src/ui/mod.rs` (the test from Task 1, one new test after it, and `every_look_lays_out_at_small_and_large_sizes` near line 129)

- [ ] **Step 1: Widen the test to every look and add the layout tests**

In `src/ui/mod.rs`, in `the_top_bar_connections_button_opens_the_picker_in_a_new_tab`, replace the filtered loop header and its comment with:

```rust
        for look in crate::theme::Look::ALL {
```

After that test add:

```rust
    #[test]
    fn the_connections_button_keeps_clear_of_the_connection() {
        for look in crate::theme::Look::ALL {
            let mut harness = Harness::with_size(egui::vec2(720.0, 480.0));
            harness.set_look(look);
            let tab = harness.connect_fake();
            harness.app.workspace_mut(tab).unwrap().name = "Bar check".into();
            let tree = harness.settle();
            let button =
                crate::testing::bounds(&tree, "Connections", egui::accesskit::Role::Button)
                    .unwrap_or_else(|| panic!("Connections missing in {}", look.name));
            let name = crate::testing::bounds(&tree, "Bar check", egui::accesskit::Role::Label)
                .unwrap_or_else(|| panic!("the name is missing in {}", look.name));
            let disconnect =
                crate::testing::bounds(&tree, "Disconnect", egui::accesskit::Role::Button)
                    .unwrap_or_else(|| panic!("Disconnect missing in {}", look.name));
            let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, harness.size);
            assert!(screen.contains_rect(button), "{}", look.name);
            // It leads the bar: before the connection's name, off Disconnect.
            assert!(button.right() < name.left(), "{}", look.name);
            assert!(!button.intersects(disconnect), "{}", look.name);
        }
    }
```

In `every_look_lays_out_at_small_and_large_sizes`, add the button to the first list of required nodes:

```rust
                for (label, role) in [
                    ("Connections", egui::accesskit::Role::Button),
                    ("Disconnect", egui::accesskit::Role::Button),
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `~/.cargo/bin/cargo test --locked -p tabletist --lib -- connections_button every_look_lays_out`
Expected: FAIL in the `omarchy` look: `nothing labelled "Connections"`, `Connections missing in omarchy`.

- [ ] **Step 3: Draw the button in `terminal_bar`**

In `src/ui/workspace.rs`, change `terminal_bar`'s doc comment to:

```rust
/// Omarchy: the Connections button, the environment badge, the name, where
/// it points, read-only, and the key that closes the connection.
```

Replace

```rust
    // Twelve in and twelve apart, as the design's row.
    let mut x = rect.left() + 12.0;
    x += env_badge(ui, x, center, info.env, env, Badge::Tracked, look) + 12.0;
```

with

```rust
    // Twelve in and twelve apart, as the design's row.
    let mut x = rect.left() + 12.0;
    // Connections first: a boxed word, eight in from its line, that opens
    // the saved connections.
    {
        let label = gettext(locale, "Connections");
        let text = Text::one(
            look,
            TextRole::OBody,
            &format!("≡ {}", gettext(locale, "conn")),
            palette.text,
        )
        .layout(ui.ctx());
        let button = Rect::from_min_size(
            pos2(x, center - 12.0),
            vec2(text.width() + 16.0 + 2.0, 24.0),
        );
        let response = ui.interact(button, ui.id().with("connections"), Sense::click());
        response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, &label));
        let line = if response.hovered() {
            env.base()
        } else {
            connection_line
        };
        ui.painter().rect_stroke(
            button,
            CornerRadius::same(3),
            Stroke::new(1.0, line),
            StrokeKind::Inside,
        );
        text.paint_left(ui.painter(), button.left() + 9.0, center);
        if response
            .on_hover_text(connections_hint(look, locale))
            .clicked()
        {
            actions.push(Action::NewConnTab);
        }
        x = button.right() + 12.0;
    }
    x += env_badge(ui, x, center, info.env, env, Badge::Tracked, look) + 12.0;
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `~/.cargo/bin/cargo test --locked -p tabletist --lib -- connections every_look_lays_out`
Expected: every matched test passes (the filter also matches a few older tests), in every look.

- [ ] **Step 5: Commit point**

Stop and report. If asked to commit:

```bash
git add src/ui/workspace.rs src/ui/mod.rs
git commit -S -m "Give the Omarchy bar its Connections button

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

## Task 3: The spec names the button, and everything passes

**Files:**
- Modify: `docs/superpowers/specs/2026-09-27-tabletist-design.md:405-406`

- [ ] **Step 1: Update section 5.3**

Replace

```markdown
- `+` or Cmd/Ctrl+O opens a new tab with the **picker**. The app starts with
  one picker tab.
```

with

```markdown
- `+`, the connection bar's **Connections** button, or Cmd/Ctrl+O opens a new
  tab with the **picker**. The app starts with one picker tab.
```

- [ ] **Step 2: Run every check**

```bash
~/.cargo/bin/cargo fmt --all --check
~/.cargo/bin/cargo clippy --locked --workspace --all-targets -- -D warnings
~/.cargo/bin/cargo clippy --locked --workspace --all-targets --features shots -- -D warnings
~/.cargo/bin/cargo test --locked --workspace --all-targets
RUSTDOCFLAGS='-D warnings' ~/.cargo/bin/cargo doc --locked --workspace --no-deps
```

Expected: all five exit 0; the database integration tests print "skipped" without their servers. If `fmt` reports a diff, run `~/.cargo/bin/cargo fmt --all` and rerun.

- [ ] **Step 3: Look at it once, by eye**

Render the workspace scene in both looks and compare the bar with the artboards by hand. The PNGs stay local: never commit or push them.

```bash
~/.cargo/bin/cargo test --locked --features shots --lib shots -- --ignored
```

It needs a GPU (wgpu) and writes `target/shots/workspace-<look>-<light|dark>.png` (gitignored). Without a GPU, say so in the report instead. Check: `≡` draws as a glyph, the button is the first thing in the bar, on the bar's line, and the crumb (macOS) or the environment badge (Omarchy) follows it 12 on.

- [ ] **Step 4: Commit point**

Stop and report. If asked to commit:

```bash
git add docs/superpowers/specs/2026-09-27-tabletist-design.md
git commit -S -m "Name the Connections button in the design's connection tabs

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```
