# App Header Connections Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Show every open connection as a chip in the connection bar, the app's header, as the "0.1.0 · Multiple connections" artboards draw it, and have the Connections list say which saved connections are open and show them, as "0.1.0 · Connections" does. The connection tab bar goes.

**Architecture:** The connection bar is painted by hand in `src/ui/workspace.rs`: `mac_bar` for the macOS and standard looks, `terminal_bar` for Omarchy. Both get a row of chips between the Connections button and Disconnect, one per open connection, laid out by a small piece of shared arithmetic (`fit`, `place`) that is tested on its own. The bar's own chip is the database switcher it already had; another's pushes the existing `Action::ActivateConnTab`. The app keeps its `tabs: Vec<ConnTab>`: a workspace tab is a chip, and the picker becomes one tab at most, reached by a new `Action::ShowConnections`. `src/ui/conn_tabs.rs` is deleted.

**Tech Stack:** Rust 2024, egui (crmne fork), fastframe. Headless UI tests through `src/testing.rs` (AccessKit).

**Design:** the user's Design canvas, rows "0.1.0 · Multiple connections" (macOS and Omarchy, two connections open) and "0.1.0 · Connections" (the list). The table view artboards draw the same header. The canvas is a private Claude Artifact: it is not in the repository and is not to be added. What this plan needs from it is written out below.

**Where this plan's code comes from:** the change was drafted while exploring, and the tree after each of Tasks 1 to 5 was built and checked (`cargo fmt --check`, `cargo clippy -D warnings`, the library tests: 788 at the end, from 780). The code and diffs below are that draft's. The draft was set aside; the tree starts clean.

---

## Ground rules (read first)

- Work in `/home/igor/Work/tabletist/.claude/worktrees/app-header-connections-820cc6`, branch `claude/app-header-connections-820cc6` (fast-forwarded to `origin/main` at `4237609`).
- `cargo` through mise is broken here: always run `~/.cargo/bin/cargo`. Never point `CARGO_TARGET_DIR` at `/tmp`.
- **Never commit or push unless the user asks.** Each task ends with a "commit point": stop there and report. If the user has asked to commit: one topic per commit, signed (`git commit -S`). If signing fails, ask before committing unsigned. Never set `SSH_AUTH_SOCK` in git commands. End messages with `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.
- No em dashes anywhere (code, comments, docs, commit messages). Use a full stop, comma, colon or parentheses.
- Tests are behavioural (a chip exists, a click switches, chips keep off their neighbours, a colour comes from `env_colors()`). Never a design or pixel conformance test: do not assert the design's sizes, offsets or colours.
- Test fixtures use neutral names only (`Fixture`, `First`, `Second`, the Bookshop demo data). Never Safari Portal names.
- Views draw text only through `TextRole`s and the `widgets` helpers; never name a font or size.
- Views do not mutate application state: push an `Action`.
- Comment density: short doc comments on items saying what and why, matching the surrounding code.
- Diffs below show what to change with three lines of context. Make the edits they show; do not paste a diff over a file. Line numbers are those of the tree as the task before left it.
- Run `~/.cargo/bin/cargo fmt --all` before a task's check: the code below is already formatted, so it should change nothing.

Full check commands (used in the last task, and handy any time):

```bash
~/.cargo/bin/cargo fmt --all --check
~/.cargo/bin/cargo clippy --locked --workspace --all-targets -- -D warnings
~/.cargo/bin/cargo clippy --locked --workspace --all-targets --features shots -- -D warnings
~/.cargo/bin/cargo test --locked --workspace --all-targets
RUSTDOCFLAGS='-D warnings' ~/.cargo/bin/cargo doc --locked --workspace --no-deps
```

## What the design says

### The header, macOS (also the standard look)

One row of chrome, 56 tall under the 3 pt stripe in the environment's colour and over a 1 pt rule (60 in all). The row starts 16 in, its items 12 apart, and ends 12 from the right.

| Part | As drawn |
|---|---|
| Connections button | 34 x 32, as today |
| Another connection's chip | 40 tall, corners 8, a 1 pt line in the bar's rule colour, no fill, text `#4d4c48`. Inside, 12 in: an 8 pt dot in that connection's environment colour, 10, then two lines 2 apart: the name (13, weight 500) with the environment's pill 6 after it (10.5, weight 500, 6 at its sides, corners 8, the environment's tint and text), over the database (mono 12, `#6b6a65`). 12 after. Title: "Switch to Bookshop · dev, ⌘1" |
| This connection's chip | the same on a face of white at 85%, the name in weight 600 and `#1c1c1a`; 10 after the text come the pop-up's chevrons (12), and 10 after them the edge. Title: "This window · click to switch database" |
| Read-only pill, Disconnect | as today |
| Card under another connection's chip | 320 wide: the dot, name and pill with "⌘1" at the right; Host, Database, User, Server ("PostgreSQL 17"), Security ("Local · no SSH"), Connected ("2 h ago · 3 tabs open"); and "Click to switch to this window" |

Dark mode draws the same on the dark bar tint.

### The header, Omarchy

One row, 48 tall over a 1 pt rule (49 in all), 12 in, items 12 apart.

| Part | As drawn |
|---|---|
| `≡ conn` | as today |
| This connection's chip | 38 tall, corners 3, a 1 pt line in the environment's colour, filled with the panel colour. 5 in: the environment's tag (upper case, bold 11.5, tracked, 6 at its sides, corners 2, the environment's colour under the panel colour's text), 8, the name (bold 13) over the database (12, muted), 8, `▾` (muted), 8 |
| Another connection's chip | the same with the window's line colour and no fill; the tag is an outline in its environment's colour with text in that colour; the name is regular weight. Title: "Go to Bookshop · production, ctrl+shift+2" |
| `read-only` tag | as today |
| Right | key hints: `ctrl+shift+c conn · ctrl+shift+1..9 window · ctrl+shift+d db` |

### The Connections list

| Look | As drawn |
|---|---|
| macOS | A saved connection that is open carries an `open` tag after its environment pill (11, 7 at its sides, corners 10, a 1 pt line in a light blue, text in the link blue), and its primary button reads "Show window" where the others read "Connect" |
| Omarchy | `open` in the accent after the name, 10 on. The footer says `enter connect / go to window` |

### How the design's names map to the code

| Design | Code |
|---|---|
| `headerLine`, `connLine` | `env.bar_border()` (`rim` in `mac_bar`, `connection_line` in `terminal_bar`) |
| white at 85% | `face(0.85)`, the closure `mac_bar` already has |
| `#4d4c48`, `#6b6a65`, `#1c1c1a` | `palette.secondary`, `palette.dim`, `palette.text` |
| corners 8 | `CornerRadius::same(look.radius)`, as the Connections button |
| 13 / 500, 13 / 600 | `TextRole::UiBodyStrong`, `TextRole::UiBodySemibold` |
| the pill's 10.5 / 500 | a new `TextRole::ChipEnv` |
| mono 12 | `TextRole::MonoSecondary` |
| `t.dark`, `t.border`, `t.muted`, `t.fg` | `palette.panel`, `palette.outline`, `palette.dim`, `palette.text` |
| bold 13, 13, 12 (Omarchy) | `TextRole::OGroup`, `TextRole::OBody`, `TextRole::OSecondary` |
| the tracked tag | `TextRole::OEnvLabel` (`Badge::Tracked`, whose sides and corners become the chip's: 6 and 2) |

## Decisions (where the design and the app differ)

1. **Windows become tabs.** The design has a window per connection, and a chip brings another window forward. The app has one window: a chip switches it to that connection (`Action::ActivateConnTab`). The tab bar that did this goes, and with it `src/ui/conn_tabs.rs`.
2. **The Connections window becomes one picker tab.** The Connections button and Cmd/Ctrl+O show the picker tab, opening it when there is none (`Action::ShowConnections` replaces `Action::NewConnTab`). Disconnect shows the picker too: alone, the connection's tab becomes it, as today; with the picker already open, the connection's tab closes.
3. **Keys stay Cmd/Ctrl.** Cmd/Ctrl+1..9 switch to an open connection by its place among the chips (they counted every tab, the picker too). Ctrl+Tab still visits every tab. The artboards' `ctrl+shift+c`, `ctrl+shift+1..9` and `ctrl+shift+d` are not bound, so the Omarchy bar's right corner keeps `ctrl+shift+w disconnect`, as decided on 2026-10-01.
4. **"Show window" reads "Show".** The app has no second window to show. Choosing an open connection (the button, Enter, a double click) shows its tab instead of connecting again. Shift+Enter, or "Connect again" in the row's menu, still opens it once more: one saved connection in two tabs stays possible (spec 5.3). For the same reason the Omarchy footer says `enter connect / show`, and a chip is announced as "Switch to Bookshop · dev" in every look (the Omarchy artboard's title says "Go to"); the key is in the card, not the name.
5. **The list's header has no chips,** as its artboard. The way back from the list is Show on an open row, Cmd/Ctrl+1..9, or Ctrl+Tab.
6. **A chip says how its session stands.** The artboards show connected sessions only. While a session is connecting, disconnected or cancelled, the chip's second line says so in place of the database, and the macOS dot takes the status colour the tab's dot had (warning, danger, dim).
7. **The TLS and SSH pills stay,** after Read-only. The artboards show a local connection, which has neither; the app keeps saying how far a remote connection can be trusted.
8. **When the bar runs out of room** (the design has no rule): the pills give way first, from the last; then the widest names are cut with an ellipsis, down to 48 pt of text; then the row slides so the bar's own chip stays whole, and the rest is clipped at the row's ends.
9. **The card shows on every chip, in both looks.** The Omarchy artboard has only a title. The card's "Connected 2 h ago" needs the time a session connected, which the workspace does not keep: `Workspace::connected_at` is added. "Server" says the driver's name until the server's version is known (it is asked for when the first SQL editor opens). The card's key is the one that works (`⌘1`, `Ctrl+1`, `ctrl+1`). The card is egui's tooltip, so it is as wide as its lines rather than 320, has no dot (the chip above it has one), and puts the key after the pill.
10. **Omarchy gets the database switcher.** Its bar only named the database; the chip is drawn as a pop-up, so it opens the same menu as macOS (by click; no key). In both looks the chevrons or `▾` show only when the server has another database to switch to, as the macOS bar's did.
11. **Closing a connection that is not showing** went with the tab's close button and middle click. Switch to it, then Disconnect or Cmd/Ctrl+Shift+W.
12. **Dark macOS faces are not re-tuned.** The chips use the `face()` closure the bar has (the window colour at an alpha in dark mode), as the Connections button does.
13. **Out of scope:** the list header's "Paste URL" button, the list's per-connection `read-only` tag, the Omarchy artboard's overlay picker (`ctrl+shift+c`), the Windows draft artboard, and the README's screenshots.

## File map

| File | Change |
|---|---|
| `src/typography.rs` | `TextRole::ChipEnv` |
| `src/app.rs` | `open_connections`, `tab_showing`, `picker_index`; `ShowConnections`, `ActivateConnection`; Disconnect; `connected_at` is set; tests |
| `src/model.rs` | the two actions; `Workspace::connected_at`; a doc comment |
| `src/ui/workspace.rs` | the bar: chips, their layout, the card, badge styles; unit tests |
| `src/ui/conn_tabs.rs` | deleted |
| `src/ui/mod.rs` | no tab bar; `window_buttons_line`; tests |
| `src/ui/picker.rs` | the title bar inset; `open`, Show, "Connect again"; a first unit test |
| `src/ui/keys.rs` | the two actions; Enter and Shift+Enter in the picker; the shortcut table |
| `src/ui/env_tests.rs` | the chip's colours replace the tab dot's |
| `src/env.rs`, `src/theme.rs`, `src/ui/widgets.rs`, `src/entrypoint.rs`, `src/macos.rs` | the tab bar's name leaves a list and four comments |
| `src/shots.rs` | a second connection in the scenes |
| `docs/superpowers/specs/2026-09-27-tabletist-design.md`, `README.md` | the connection bar replaces the tab bar |

---

## Task 1: The bar draws a chip for each open connection

The tab bar stays for this task: with several tabs open the window shows both. Task 2 removes it.

**Files:**
- Modify: `src/typography.rs` (the role)
- Modify: `src/app.rs` (`open_connections`, above `tab_for_session`, near line 185)
- Modify: `src/ui/workspace.rs` (`bar_height`, everything from `BarInfo` to `terminal_bar`, `Badge` and `env_badge`, the tests module)
- Test: `src/ui/mod.rs` (tests module), `src/ui/env_tests.rs`, `src/ui/workspace.rs`

- [ ] **Step 1: Write the failing tests**

In `src/ui/mod.rs`, inside `mod tests`, just above `fn add_saved`:

```rust
    /// Opens another connection beside the open ones, named `name`: the
    /// picker (Mod+O), then a connect in it. Returns its tab, which shows.
    fn connect_another(harness: &mut Harness, name: &str) -> crate::model::ConnTabId {
        harness.press(Key::O, Modifiers::COMMAND);
        let tab = harness.connect_fake();
        harness.app.workspace_mut(tab).unwrap().name = name.into();
        tab
    }
```

Just above `the_picker_has_a_new_connection_button`:

```rust
    #[test]
    fn the_bar_has_a_chip_for_each_open_connection() {
        for look in crate::theme::Look::ALL {
            let mut harness = Harness::new();
            harness.set_look(look);
            let first = harness.connect_fake();
            harness.app.workspace_mut(first).unwrap().name = "First".into();
            let second = connect_another(&mut harness, "Second");
            let env = crate::env::Environment::Dev.label(crate::env::Platform::of(&look));
            // The second shows: its chip is the database switcher, and
            // reads its name.
            let tree = harness.settle();
            for (label, role) in [
                ("Database", egui::accesskit::Role::ComboBox),
                ("Second", egui::accesskit::Role::Label),
            ] {
                assert!(
                    crate::testing::node(&tree, label, role).is_some(),
                    "{label} missing in {}",
                    look.name
                );
            }
            // The first's chip switches to it, and back.
            harness.click(&format!("Switch to First · {env}"));
            assert_eq!(harness.app.active_tab_id(), first, "{}", look.name);
            harness.click(&format!("Switch to Second · {env}"));
            assert_eq!(harness.app.active_tab_id(), second, "{}", look.name);
        }
    }

    #[test]
    fn a_chip_says_how_its_session_stands_while_it_is_not_connected() {
        for look in crate::theme::Look::ALL {
            let mut harness = Harness::new();
            harness.set_look(look);
            let tab = harness.connect_fake();
            // Connected, it says where it points: the fixture's file.
            assert!(harness.has("fixture.db"), "{}", look.name);
            let session = harness.app.workspace(tab).unwrap().session;
            harness.app.apply(crate::model::Action::Backend(
                crate::backend::Event::Disconnected {
                    session,
                    error: tabletist_db::Error::ConnectionLost("server went away".into()),
                },
            ));
            assert!(harness.has(&look.label("Disconnected")), "{}", look.name);
            assert!(!harness.has("fixture.db"), "{}", look.name);
        }
    }

    #[test]
    fn many_chips_share_the_bar_and_the_one_showing_stays_whole() {
        for look in crate::theme::Look::ALL {
            let mut harness = Harness::with_size(egui::vec2(720.0, 480.0));
            harness.set_look(look);
            harness.connect_fake();
            for number in 2..=8 {
                connect_another(&mut harness, &format!("Connection number {number}"));
            }
            let tree = harness.settle();
            let bounds = |label: &str, role| {
                crate::testing::bounds(&tree, label, role)
                    .unwrap_or_else(|| panic!("{label} missing in {}", look.name))
            };
            let connections = bounds("Connections", egui::accesskit::Role::Button);
            let disconnect = bounds("Disconnect", egui::accesskit::Role::Button);
            // The last one shows: its chip is whole, between the buttons.
            let own = bounds("Database", egui::accesskit::Role::ComboBox);
            assert!(own.left() > connections.right(), "{}", look.name);
            assert!(own.right() < disconnect.left(), "{}", look.name);
            assert!(own.width() > 60.0, "{}: {own:?}", look.name);
            // No other chip reaches over either button.
            let others: Vec<_> = tree
                .nodes
                .iter()
                .filter(|(_, node)| {
                    node.label()
                        .is_some_and(|label| label.starts_with("Switch to"))
                })
                .filter_map(|(_, node)| node.bounds())
                .collect();
            assert!(!others.is_empty(), "{}", look.name);
            for chip in others {
                assert!(chip.x0 as f32 >= connections.right(), "{}", look.name);
                assert!(chip.x1 as f32 <= disconnect.left(), "{}", look.name);
            }
        }
    }
```

In `src/ui/env_tests.rs`, just above `the_connections_list_shows_each_environment_in_its_colours`:

```rust
#[test]
fn a_connections_chip_takes_its_environments_colour() {
    for (look, palette) in setups() {
        for env in Environment::ALL {
            let mut harness = harness(look, palette);
            let tab = harness.connect_fake();
            harness.app.workspace_mut(tab).unwrap().environment = env;
            // A second connection in another environment shows, so the
            // colours found are the first chip's.
            let other = if env == Environment::None {
                Environment::Dev
            } else {
                Environment::None
            };
            harness.press(Key::O, Modifiers::COMMAND);
            let second = harness.connect_fake();
            harness.app.workspace_mut(second).unwrap().environment = other;
            harness.settle();
            let colors = colors(&harness, env);
            let case = format!("{} dark={} {env:?}", look.name, palette.dark);
            // The dot: small and square, where the stripe is neither.
            let dot = |rect: egui::Rect| rect.width() == rect.height() && rect.width() < 16.0;
            // Behind the other: a dot on macOS, an outlined tag in the
            // terminal.
            if look.terminal {
                assert!(harness.strokes.contains(&colors.base()), "outline, {case}");
                assert_eq!(
                    harness.painted_color(label(&harness, env)),
                    Some(colors.base()),
                    "outlined tag, {case}"
                );
            } else {
                assert!(filled(&harness, colors.base(), dot), "behind, {case}");
            }
            // Showing: the dot again, or the solid tag.
            harness.app.apply(Action::ActivateConnTab(tab));
            harness.settle();
            if look.terminal {
                assert!(filled(&harness, colors.badge_bg(), |_| true), "tag, {case}");
                assert_eq!(
                    harness.painted_color(label(&harness, env)),
                    Some(colors.badge_fg()),
                    "tag text, {case}"
                );
            } else {
                assert!(filled(&harness, colors.base(), dot), "showing, {case}");
            }
        }
    }
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `~/.cargo/bin/cargo test --locked -p tabletist --lib -- chip`
Expected: FAIL, all four. `the_bar_has_a_chip_for_each_open_connection` panics with `nothing labelled "Switch to First · dev"`; `a_chip_says_how_its_session_stands_while_it_is_not_connected` fails where it looks for `Disconnected`; `many_chips_share_the_bar_and_the_one_showing_stays_whole` finds no other chip; `a_connections_chip_takes_its_environments_colour` fails on `outline` (the tab's dot passes for the chip's on macOS, so Omarchy is where it fails).

- [ ] **Step 3: Add the pill's text role**

In `src/typography.rs`:

```diff
--- a/src/typography.rs
+++ b/src/typography.rs
@@ -73,6 +73,8 @@
     GroupLabel,
     /// The picker's environment tags.
     EnvTag,
+    /// The environment tag in the connection bar's chips.
+    ChipEnv,
     /// View and trigger tags in the object tree.
     TagSmall,
     /// The database name, the sort chip's column, a connection's database.
@@ -112,7 +114,7 @@
 }
 
 impl TextRole {
-    pub const ALL: [TextRole; 38] = [
+    pub const ALL: [TextRole; 39] = [
         Self::UiBody,
         Self::UiBodyStrong,
         Self::TableTitle,
@@ -129,6 +131,7 @@
         Self::UiBodySemibold,
         Self::GroupLabel,
         Self::EnvTag,
+        Self::ChipEnv,
         Self::TagSmall,
         Self::MonoSecondary,
         Self::MonoGroup,
@@ -172,6 +175,7 @@
             Self::UiBodySemibold => "ui-body-semibold",
             Self::GroupLabel => "group-label",
             Self::EnvTag => "env-tag",
+            Self::ChipEnv => "chip-env",
             Self::TagSmall => "tag-small",
             Self::MonoSecondary => "mono-secondary",
             Self::MonoGroup => "mono-group",
@@ -246,6 +250,7 @@
             Self::UiBodySemibold => style(Sans, 600, 13.0),
             Self::GroupLabel => capitals(Sans, 600, 12.0, 0.06),
             Self::EnvTag => style(Sans, 500, 11.0),
+            Self::ChipEnv => style(Sans, 500, 10.5),
             Self::TagSmall => style(Sans, 400, 10.5),
             Self::MonoSecondary => style(Mono, 400, 12.0),
             Self::MonoGroup => style(Mono, 500, 12.0),
```

- [ ] **Step 4: List the open connections**

In `src/app.rs`, above `tab_for_session`:

```rust
    /// The open connections with their tabs, in the order the header shows
    /// them. The picker is not one of them.
    pub fn open_connections(&self) -> impl Iterator<Item = (ConnTabId, &Workspace)> {
        self.tabs.iter().filter_map(|tab| match &tab.content {
            ConnTabContent::Workspace(workspace) => Some((tab.id, &**workspace)),
            ConnTabContent::Picker(_) => None,
        })
    }
```

- [ ] **Step 5: Give the badges their chip styles**

In `src/ui/workspace.rs`, replace the `Badge` enum and `env_badge` (from `/// How an environment badge draws.` to the end of `env_badge`) with:

```rust
/// How an environment badge draws.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Badge {
    /// macOS: a rounded tint, the label as it is.
    Mac,
    /// macOS, in the connection bar's chips: smaller, six at its sides.
    Chip,
    /// The terminal's solid upper-case tag, letter-spaced (the connection
    /// bar's own chip).
    Tracked,
    /// The same tag as an outline in the environment's colour (the bar's
    /// other chips).
    Outlined,
    /// The terminal's tag without letter spacing (the connections list).
    Plain,
}

impl Badge {
    /// Whose label it shows, the label's role, the room at its sides, and
    /// its corner.
    fn shape(self) -> (crate::env::Platform, TextRole, f32, u8) {
        use crate::env::Platform::{Native, Omarchy};
        match self {
            Self::Mac => (Native, TextRole::EnvTag, 7.0, 10),
            Self::Chip => (Native, TextRole::ChipEnv, 6.0, 8),
            Self::Tracked | Self::Outlined => (Omarchy, TextRole::OEnvLabel, 6.0, 2),
            Self::Plain => (Omarchy, TextRole::OBadge, 7.0, 3),
        }
    }
}

/// The width [`env_badge`] takes.
pub fn env_badge_width(
    ui: &egui::Ui,
    env: crate::env::Environment,
    style: Badge,
    look: &Look,
) -> f32 {
    let (platform, role, pad, _) = style.shape();
    role.width(ui.ctx(), look.faces, env.label(platform)) + 2.0 * pad
}

/// An environment badge at `x`, centred on `y`. Returns its width.
#[allow(clippy::too_many_arguments)] // position, what, and how
pub fn env_badge(
    ui: &egui::Ui,
    x: f32,
    y: f32,
    env: crate::env::Environment,
    colors: &crate::env::EnvColors,
    style: Badge,
    look: &Look,
) -> f32 {
    // One point above and below the text.
    let (platform, role, pad, corner) = style.shape();
    let outlined = style == Badge::Outlined;
    let ink = if outlined {
        colors.base()
    } else {
        colors.badge_fg()
    };
    let laid = Text::one(look, role, env.label(platform), ink).layout(ui.ctx());
    let height = laid.height() + 2.0;
    let rect = Rect::from_min_size(
        pos2(x, y - height / 2.0),
        vec2(laid.width() + 2.0 * pad, height),
    );
    let corner = CornerRadius::same(corner);
    if outlined {
        ui.painter().rect_stroke(
            rect,
            corner,
            Stroke::new(1.0, colors.base()),
            StrokeKind::Inside,
        );
    } else {
        ui.painter().rect_filled(rect, corner, colors.badge_bg());
    }
    laid.paint_left(ui.painter(), x + pad, y);
    rect.width()
}
```

`Badge::Tracked` loses a point at each side and of its corner: it is the chip's tag now, and nothing else draws it.

- [ ] **Step 6: Write the layout's tests**

In `src/ui/workspace.rs`, in `mod tests`, above `the_connections_hint_names_the_key_as_the_look_spells_it`:

```rust
    #[test]
    fn the_rows_room_goes_to_the_chips_before_the_pills() {
        // Two chips, 40 around texts of 100 and 60, and two pills, all 12
        // apart: 252 for the chips, 406 with both pills.
        let chips = [(40.0, 100.0), (40.0, 60.0)];
        let pills = [80.0, 50.0];
        assert_eq!(
            fit(&chips, &pills, 12.0, 406.0),
            Fit {
                pills: 2,
                cap: 100.0
            }
        );
        // The last pill gives way first, then the other.
        assert_eq!(fit(&chips, &pills, 12.0, 405.0).pills, 1);
        assert_eq!(fit(&chips, &pills, 12.0, 300.0).pills, 0);
        // Then the widest text is cut: 20 short leaves it 80.
        let tight = fit(&chips, &pills, 12.0, 232.0);
        assert_eq!(tight.pills, 0);
        assert!((tight.cap - 80.0).abs() < 0.01, "{tight:?}");
        // Under the narrower text, both are cut alike.
        let tighter = fit(&chips, &pills, 12.0, 192.0);
        assert!((tighter.cap - 50.0).abs() < 0.01, "{tighter:?}");
        // Never under the floor.
        assert_eq!(fit(&chips, &pills, 12.0, 10.0).cap, FLOOR);
    }

    #[test]
    fn chips_that_overflow_slide_to_keep_the_bars_own_whole() {
        // Three chips of 90, 12 apart, in a row of 200: the third ends at 394.
        let row = Rect::from_min_max(pos2(100.0, 0.0), pos2(300.0, 40.0));
        let widths = [90.0, 90.0, 90.0];
        let first = place(&widths, Some(0), 12.0, row, 20.0, 40.0);
        assert_eq!(first[0].left(), 100.0, "the first is whole where it is");
        let last = place(&widths, Some(2), 12.0, row, 20.0, 40.0);
        assert_eq!(last[2].right(), 300.0);
        assert_eq!(last[0].left(), 6.0, "the others slide with it");
        assert_eq!(last[2].center().y, 20.0);
        // A row that holds them all slides nothing.
        let wide = Rect::from_min_max(pos2(100.0, 0.0), pos2(500.0, 40.0));
        assert_eq!(
            place(&widths, Some(2), 12.0, wide, 20.0, 40.0)[0].left(),
            100.0
        );
    }
```

They do not compile until Step 7: that is their failing state.

- [ ] **Step 7: What the bar knows, and how it shares its row**

In `src/ui/workspace.rs`, make the bar taller:

```rust
/// The connection bar's height.
pub fn bar_height(look: &Look) -> f32 {
    // macOS: 56 and a 3 pt stripe above, a 1 pt rule below. Omarchy: 48
    // and the rule.
    if look.terminal { 49.0 } else { 60.0 }
}
```

Replace everything from `/// What the connection bar says about one connection.` to the end of `bar_info` (`BarInfo`, `Tone`, `bar_info`) with:

```rust
/// What the connection bar says: the open connections, and more about the
/// one the bar belongs to.
struct BarInfo {
    env: crate::env::Environment,
    database: String,
    databases: Vec<String>,
    tls: Option<(&'static str, Tone)>,
    ssh_host: Option<String>,
    chips: Vec<Chip>,
}

/// How a status reads: fine, or a warning.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Tone {
    Good,
    Warn,
}

/// How a connection's session stands, as its chip shows it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Link {
    Connected,
    Connecting,
    Disconnected,
    Cancelled,
}

/// One open connection in the bar.
struct Chip {
    tab: ConnTabId,
    /// Whether the bar is this connection's own.
    own: bool,
    name: String,
    env: crate::env::Environment,
    /// Under the name: the database (a file's name for SQLite), or how the
    /// session stands while it is not connected.
    line: String,
    link: Link,
}

fn bar_info(app: &App, tab: ConnTabId) -> Option<BarInfo> {
    let workspace = app.workspace(tab)?;
    let spec = &workspace.spec;
    let sqlite = workspace.driver == tabletist_db::Driver::Sqlite;
    // Local traffic never crosses a network, so its TLS says nothing; a
    // remote connection always says how far it can be trusted.
    let remote = !sqlite && !crate::model::is_local_host(&spec.host);
    let tls = remote.then(|| {
        let encrypted =
            matches!(workspace.status, SessionStatus::Connected).then_some(workspace.encrypted);
        let (text, warn) = tls_status(spec.effective_tls(), encrypted);
        (text, if warn { Tone::Warn } else { Tone::Good })
    });
    Some(BarInfo {
        env: workspace.environment,
        database: if sqlite {
            String::new()
        } else {
            spec.database.clone()
        },
        databases: workspace.databases.value.clone().unwrap_or_default(),
        tls,
        ssh_host: spec.ssh.as_ref().map(|ssh| ssh.host.clone()),
        chips: chips(app, tab),
    })
}

/// A chip for each open connection, in the header's order. `own` is the
/// connection whose bar this is.
fn chips(app: &App, own: ConnTabId) -> Vec<Chip> {
    app.open_connections()
        .map(|(tab, workspace)| {
            let (link, state) = match &workspace.status {
                SessionStatus::Connected => (Link::Connected, None),
                SessionStatus::Connecting { .. } => (Link::Connecting, Some("Connecting…")),
                SessionStatus::Disconnected(_) => (Link::Disconnected, Some("Disconnected")),
                SessionStatus::Cancelled => (Link::Cancelled, Some("Cancelled")),
            };
            let line = match state {
                Some(state) => app.look.label(&gettext(app.locale, state)),
                None => target(workspace),
            };
            Chip {
                tab,
                own: tab == own,
                name: workspace.name.clone(),
                env: workspace.environment,
                line,
                link,
            }
        })
        .collect()
}

/// Where a connection points, in a word: its database, or the file's name
/// for SQLite.
fn target(workspace: &crate::model::Workspace) -> String {
    match &workspace.spec.sqlite_path {
        Some(path) => crate::model::file_name(&path.display().to_string()),
        None => workspace.spec.database.clone(),
    }
}

/// The narrowest a chip's text gets. Chips that do not fit even so are
/// clipped at the row's end.
const FLOOR: f32 = 48.0;

/// The widest a chip's text may be so that `columns` take `room` at most:
/// the wider ones are cut to it, the others keep their width. Never under
/// `FLOOR`.
fn text_cap(columns: &[f32], room: f32) -> f32 {
    let widest = columns.iter().copied().fold(0.0, f32::max);
    let used = |cap: f32| columns.iter().map(|width| width.min(cap)).sum::<f32>();
    if used(widest) <= room {
        return widest;
    }
    let (mut fits, mut too_wide) = (FLOOR.min(widest), widest);
    for _ in 0..24 {
        let middle = (fits + too_wide) / 2.0;
        if used(middle) <= room {
            fits = middle;
        } else {
            too_wide = middle;
        }
    }
    fits
}

/// How the bar's row is shared.
#[derive(Debug, PartialEq)]
struct Fit {
    /// How many of the pills show.
    pills: usize,
    /// The widest a chip's text may be.
    cap: f32,
}

/// Shares `room` between the chips and the pills after them, each `gap`
/// from the last. A chip is `(fixed, column)`: what it draws around its
/// text, and its text's own width. The pills give way first, from the
/// last; then the widest texts are cut.
fn fit(chips: &[(f32, f32)], pills: &[f32], gap: f32, room: f32) -> Fit {
    let fixed = chips.iter().map(|(fixed, _)| fixed).sum::<f32>()
        + gap * chips.len().saturating_sub(1) as f32;
    let columns: Vec<f32> = chips.iter().map(|(_, column)| *column).collect();
    let natural = fixed + columns.iter().sum::<f32>();
    let after = |shown: usize| pills[..shown].iter().map(|pill| gap + pill).sum::<f32>();
    let mut shown = pills.len();
    while shown > 0 && natural + after(shown) > room {
        shown -= 1;
    }
    Fit {
        pills: shown,
        cap: text_cap(&columns, room - fixed - after(shown)),
    }
}

/// Where each chip goes in `row`: `widths` laid from its left, `gap`
/// apart, slid left by as much as brings the bar's own chip (`own`) wholly
/// into the row when they overflow it.
fn place(
    widths: &[f32],
    own: Option<usize>,
    gap: f32,
    row: Rect,
    y: f32,
    height: f32,
) -> Vec<Rect> {
    let mut rects = Vec::with_capacity(widths.len());
    let mut x = row.left();
    for width in widths {
        rects.push(Rect::from_min_size(
            pos2(x, y - height / 2.0),
            vec2(*width, height),
        ));
        x += width + gap;
    }
    let over = own
        .and_then(|own| rects.get(own))
        .map_or(0.0, |own| (own.right() - row.right()).max(0.0));
    for rect in &mut rects {
        *rect = rect.translate(vec2(-over, 0.0));
    }
    rects
}

/// The colour that says how a chip's session stands: its environment's
/// while connected.
fn link_color(link: Link, env: &crate::env::EnvColors, palette: &Palette) -> egui::Color32 {
    match link {
        Link::Connected => env.base(),
        Link::Connecting => palette.warning,
        Link::Disconnected => palette.danger,
        Link::Cancelled => palette.dim,
    }
}

/// What a click on another connection's chip is announced as.
fn switch_label(chip: &Chip, look: &Look, locale: crate::i18n::Locale) -> String {
    format!(
        "{} {} · {}",
        gettext(locale, "Switch to"),
        chip.name,
        chip.env.label(crate::env::Platform::of(look))
    )
}

/// Announces the bar's own chip: the database switcher.
fn announce_switcher(
    response: &egui::Response,
    info: &BarInfo,
    switchable: bool,
    locale: crate::i18n::Locale,
) {
    response.widget_info(|| {
        let mut announced = egui::WidgetInfo::labeled(
            egui::WidgetType::ComboBox,
            switchable,
            gettext(locale, "Database"),
        );
        announced.current_text_value = Some(display_safe(&info.database).into_owned());
        announced
    });
}

/// The pop-up of the server's other databases, under the bar's own chip.
fn database_menu(
    response: &egui::Response,
    tab: ConnTabId,
    info: &BarInfo,
    look: &Look,
    actions: &mut Vec<Action>,
) {
    let role = TextRole::pick(look, TextRole::MonoSecondary, TextRole::OSecondary);
    egui::Popup::menu(response).show(|ui| {
        ui.set_min_width(response.rect.width());
        for database in &info.databases {
            // Database names come from the server: nothing hidden.
            let text = Text::one(
                look,
                role,
                &display_safe(database),
                egui::Color32::PLACEHOLDER,
            )
            .layout(ui.ctx());
            if ui
                .add(egui::Button::selectable(
                    *database == info.database,
                    text.galley,
                ))
                .clicked()
                && *database != info.database
            {
                actions.push(Action::SwitchDatabase {
                    tab,
                    database: database.clone(),
                });
            }
        }
    });
}
```

`BarInfo` loses `name` and `host`: the chips carry the names, and the host moves to the card (Task 3).

Change `top_bar`'s doc comment to:

```rust
/// The connection bar: the connection's environment colour behind the open
/// connections, and the way out.
```

The file does not compile until Steps 8 and 9 replace the two painters.

- [ ] **Step 8: Draw the chips in `mac_bar`**

Replace `mac_bar`, doc comment and all, with:

```rust
/// macOS: the Connections button, a chip for each open connection (the
/// bar's own is a pop-up of the server's other databases), the read-only
/// pill, and Disconnect.
#[allow(clippy::too_many_arguments)] // one call site; the pieces are unrelated
fn mac_bar(
    ui: &mut egui::Ui,
    tab: ConnTabId,
    rect: Rect,
    info: &BarInfo,
    env: &crate::env::EnvColors,
    look: &Look,
    palette: &Palette,
    actions: &mut Vec<Action>,
    locale: crate::i18n::Locale,
) {
    let center = rect.center().y;
    // The bar's own rule, and white faces over its tint.
    let rim = env.bar_border();
    let face = |alpha: f32| {
        if palette.dark {
            palette.window.gamma_multiply(alpha)
        } else {
            egui::Color32::WHITE.gamma_multiply(alpha)
        }
    };
    let hair = Stroke::new(widgets::hairline(ui), rim);
    let corner = CornerRadius::same(look.radius);
    // Connections, 16 in: the way to the saved connections, a face of the
    // bar's own under the server icon.
    let connections =
        Rect::from_min_size(pos2(rect.left() + 16.0, center - 16.0), vec2(34.0, 32.0));
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
    // Disconnect, 12 in from the right: an icon and its label, 6 apart.
    let label = gettext(locale, "Disconnect");
    let text = Text::one(look, TextRole::UiBody, &label, palette.secondary).layout(ui.ctx());
    let width = 10.0 + 14.0 + 6.0 + text.width() + 10.0;
    let button = Rect::from_min_size(
        pos2(rect.right() - 12.0 - width, center - 16.0),
        vec2(width, 32.0),
    );
    let response = ui.interact(button, ui.id().with("disconnect"), Sense::click());
    response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, &label));
    if response.hovered() {
        ui.painter()
            .rect_filled(button, CornerRadius::same(6), face(0.7));
    }
    Icon::LogIn.image(palette.secondary, 14.0).paint_at(
        ui,
        Rect::from_center_size(pos2(button.left() + 17.0, center), vec2(14.0, 14.0)),
    );
    text.paint_left(ui.painter(), button.left() + 30.0, center);
    if response.clicked() {
        actions.push(Action::Disconnect(tab));
    }

    // The row between them, 12 from each: the chips, then the pills. What
    // does not fit is cut at the row's end.
    let left = connections.right() + 12.0;
    let row = Rect::from_min_max(
        pos2(left, rect.top()),
        pos2((button.left() - 12.0).max(left), rect.bottom()),
    );
    let mut strip = ui.new_child(egui::UiBuilder::new().id_salt("chips").max_rect(row));
    strip.shrink_clip_rect(row);
    let ui = &mut strip;
    let switchable = info.databases.len() > 1;
    let platform = crate::env::Platform::of(look);
    // A chip: 12, an 8 pt dot, 10, the name and its environment over the
    // database, and 12 (the bar's own: 10, the pop-up's chevrons, 10).
    let name_role = |chip: &Chip| {
        if chip.own {
            TextRole::UiBodySemibold
        } else {
            TextRole::UiBodyStrong
        }
    };
    let line_role = |chip: &Chip| {
        if chip.link == Link::Connected {
            TextRole::MonoSecondary
        } else {
            TextRole::Secondary
        }
    };
    let measured: Vec<(f32, f32)> = info
        .chips
        .iter()
        .map(|chip| {
            let name = name_role(chip).width(ui.ctx(), look.faces, &chip.name);
            let badge = env_badge_width(ui, chip.env, Badge::Chip, look);
            let line = line_role(chip).width(ui.ctx(), look.faces, &display_safe(&chip.line));
            let tail = if chip.own && switchable {
                10.0 + 12.0 + 10.0
            } else {
                12.0
            };
            (30.0 + tail, (name + 6.0 + badge).max(line))
        })
        .collect();
    // Pills after the chips: read-only, then TLS and SSH when remote.
    let mut pills = vec![(
        Some(Icon::Lock),
        gettext(locale, "Read-only").into_owned(),
        palette.secondary,
    )];
    if let Some((text, tone)) = info.tls {
        let color = match tone {
            Tone::Good => palette.success,
            Tone::Warn => palette.warning,
        };
        pills.push((Some(Icon::Lock), gettext(locale, text).into_owned(), color));
    }
    if let Some(host) = &info.ssh_host {
        pills.push((
            None,
            format!("{} {host}", gettext(locale, "via SSH")),
            palette.secondary,
        ));
    }
    let pill_widths: Vec<f32> = pills
        .iter()
        .map(|(icon, text, _)| {
            let icon = if icon.is_some() { 12.0 + 6.0 } else { 0.0 };
            20.0 + icon + TextRole::Secondary.width(ui.ctx(), look.faces, text)
        })
        .collect();
    let shared = fit(&measured, &pill_widths, 12.0, row.width());
    let widths: Vec<f32> = measured
        .iter()
        .map(|(fixed, column)| fixed + column.min(shared.cap))
        .collect();
    let own = info.chips.iter().position(|chip| chip.own);
    let rects = place(&widths, own, 12.0, row, center, 40.0);
    for ((chip, rect), (fixed, _)) in info.chips.iter().zip(&rects).zip(&measured) {
        let hit = rect.intersect(row);
        if !hit.is_positive() {
            continue;
        }
        let colors = crate::env::env_colors(chip.env, platform, palette);
        let response = ui.interact(hit, ui.id().with(("chip", chip.tab.0)), Sense::click());
        let (name_color, fill) = if chip.own {
            announce_switcher(&response, info, switchable, locale);
            (palette.text, Some(face(0.85)))
        } else {
            let label = switch_label(chip, look, locale);
            response
                .widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, &label));
            (palette.secondary, response.hovered().then(|| face(0.5)))
        };
        if let Some(fill) = fill {
            ui.painter().rect_filled(*rect, corner, fill);
        }
        ui.painter()
            .rect_stroke(*rect, corner, hair, StrokeKind::Inside);
        ui.painter().circle_filled(
            pos2(rect.left() + 16.0, center),
            4.0,
            link_color(chip.link, &colors, palette),
        );
        // Two lines 2 apart, centred on the bar's line; what the chip has
        // no room for is cut with an ellipsis.
        let (top, bottom) = (center - 8.0, center + 9.0);
        let x = rect.left() + 30.0;
        let room = rect.width() - fixed;
        let badge = env_badge_width(ui, chip.env, Badge::Chip, look);
        let role = name_role(chip);
        let name = crate::ui::grid::ellipsize(&chip.name, room - 6.0 - badge, false, |text| {
            role.width(ui.ctx(), look.faces, text)
        });
        let name = Text::one(look, role, &name, name_color).layout(ui.ctx());
        name.paint_left(ui.painter(), x, top);
        env_badge(
            ui,
            x + name.width() + 6.0,
            top,
            chip.env,
            &colors,
            Badge::Chip,
            look,
        );
        let role = line_role(chip);
        let line_color = if chip.link == Link::Connected {
            palette.dim
        } else {
            link_color(chip.link, &colors, palette)
        };
        let safe = display_safe(&chip.line);
        let line = crate::ui::grid::ellipsize(&safe, room, false, |text| {
            role.width(ui.ctx(), look.faces, text)
        });
        let line = Text::one(look, role, &line, line_color).layout(ui.ctx());
        line.paint_left(ui.painter(), x, bottom);
        if chip.own {
            // The bar's own connection is read as text too.
            for (laid, y) in [(&name, top), (&line, bottom)] {
                widgets::announce(
                    ui,
                    Rect::from_min_size(pos2(x, y - 8.0), vec2(laid.width().max(1.0), 16.0)),
                    laid.galley.text(),
                );
            }
            if switchable {
                Icon::ChevronsUpDown.image(palette.dim, 12.0).paint_at(
                    ui,
                    Rect::from_center_size(pos2(rect.right() - 16.0, center), vec2(12.0, 12.0)),
                );
                database_menu(&response, tab, info, look, actions);
            }
        } else if response.clicked() {
            actions.push(Action::ActivateConnTab(chip.tab));
        }
    }
    let mut left = rects.last().map_or(row.left(), |last| last.right() + 12.0);
    for ((icon, text, color), width) in pills.iter().zip(&pill_widths).take(shared.pills) {
        let rect = Rect::from_min_size(pos2(left, center - 13.0), vec2(*width, 26.0));
        let corner = CornerRadius::same(13);
        ui.painter().rect_filled(rect, corner, face(0.8));
        ui.painter()
            .rect_stroke(rect, corner, hair, StrokeKind::Inside);
        let mut x = rect.left() + 10.0;
        if let Some(icon) = icon {
            icon.image(*color, 12.0).paint_at(
                ui,
                Rect::from_center_size(pos2(x + 6.0, center), vec2(12.0, 12.0)),
            );
            x += 12.0 + 6.0;
        }
        let label = Text::one(look, TextRole::Secondary, text, *color).layout(ui.ctx());
        label.paint_left(ui.painter(), x, center);
        widgets::announce(
            ui,
            Rect::from_min_size(pos2(x, center - 8.0), vec2(label.width(), 16.0)),
            text,
        );
        left = rect.right() + 12.0;
    }
}
```

What changed: Disconnect is placed before the row so the row knows its room; the crumb (name, host, `/`, database) is the chips; the pills are measured first, so `fit` can drop them; everything in the row draws through a child `Ui` clipped to it.

- [ ] **Step 9: Draw the chips in `terminal_bar`**

Replace `terminal_bar`, doc comment and all, with:

```rust
/// Omarchy: the Connections button, a chip for each open connection (the
/// bar's own is a pop-up of the server's other databases), read-only, and
/// the key that closes the connection.
#[allow(clippy::too_many_arguments)] // one call site; the pieces are unrelated
fn terminal_bar(
    ui: &mut egui::Ui,
    tab: ConnTabId,
    rect: Rect,
    info: &BarInfo,
    env: &crate::env::EnvColors,
    look: &Look,
    palette: &Palette,
    actions: &mut Vec<Action>,
    locale: crate::i18n::Locale,
) {
    let center = rect.center().y;
    let connection_line = env.bar_border();
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
    // The way out, a muted note that also answers a click.
    let note = "ctrl+shift+w disconnect";
    let width = widgets::measure(ui, label_copy(look, TextRole::OSecondary, note));
    let left = rect.right() - 12.0 - width;
    let hit = Rect::from_min_size(
        pos2(left - 4.0, rect.top()),
        vec2(width + 8.0, rect.height()),
    );
    let response = ui.interact(hit, ui.id().with("disconnect"), Sense::click());
    let label = gettext(locale, "Disconnect");
    response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, &label));
    let color = if response.hovered() {
        palette.text
    } else {
        palette.dim
    };
    widgets::paint_text(
        ui,
        left,
        center,
        Text::one(look, TextRole::OSecondary, note, color),
    );
    if response.clicked() {
        actions.push(Action::Disconnect(tab));
    }

    // The row between them: the chips, then the tags. What does not fit is
    // cut at the row's end, 12 before the note.
    let row = Rect::from_min_max(
        pos2(x, rect.top()),
        pos2((hit.left() - 12.0).max(x), rect.bottom()),
    );
    let mut strip = ui.new_child(egui::UiBuilder::new().id_salt("chips").max_rect(row));
    strip.shrink_clip_rect(row);
    let ui = &mut strip;
    let switchable = info.databases.len() > 1;
    let platform = crate::env::Platform::of(look);
    let arrow = widgets::measure(ui, label_copy(look, TextRole::OBody, "▾"));
    // A chip: 5, the environment's tag, 8, the name over the database, and
    // 8 (the bar's own: 8, the pop-up's arrow, 8).
    let name_role = |chip: &Chip| {
        if chip.own {
            TextRole::OGroup
        } else {
            TextRole::OBody
        }
    };
    let badge_style = |chip: &Chip| {
        if chip.own {
            Badge::Tracked
        } else {
            Badge::Outlined
        }
    };
    let small = TextRole::OSecondary;
    let measured: Vec<(f32, f32)> = info
        .chips
        .iter()
        .map(|chip| {
            let badge = env_badge_width(ui, chip.env, badge_style(chip), look);
            let name = name_role(chip).width(ui.ctx(), look.faces, &chip.name);
            let line = small.width(ui.ctx(), look.faces, &display_safe(&chip.line));
            let tail = if chip.own && switchable {
                8.0 + arrow + 8.0
            } else {
                8.0
            };
            (5.0 + badge + 8.0 + tail, name.max(line))
        })
        .collect();
    // Tags after the chips: read-only, then TLS and SSH when remote.
    let mut tags = vec![(gettext(locale, "read-only").into_owned(), palette.text)];
    if let Some((text, tone)) = info.tls {
        let color = match tone {
            Tone::Good => palette.success,
            Tone::Warn => palette.warning,
        };
        tags.push((gettext(locale, text).into_owned(), color));
    }
    if let Some(host) = &info.ssh_host {
        tags.push((
            format!("{} {host}", gettext(locale, "via SSH")),
            palette.dim,
        ));
    }
    let tag_role = TextRole::OCaption;
    let tag_widths: Vec<f32> = tags
        .iter()
        .map(|(text, _)| widgets::measure(ui, label_copy(look, tag_role, text)) + 14.0 + 2.0)
        .collect();
    let shared = fit(&measured, &tag_widths, 12.0, row.width());
    let widths: Vec<f32> = measured
        .iter()
        .map(|(fixed, column)| fixed + column.min(shared.cap))
        .collect();
    let own = info.chips.iter().position(|chip| chip.own);
    let rects = place(&widths, own, 12.0, row, center, 38.0);
    let corner = CornerRadius::same(3);
    for ((chip, rect), (fixed, _)) in info.chips.iter().zip(&rects).zip(&measured) {
        let hit = rect.intersect(row);
        if !hit.is_positive() {
            continue;
        }
        let colors = crate::env::env_colors(chip.env, platform, palette);
        let response = ui.interact(hit, ui.id().with(("chip", chip.tab.0)), Sense::click());
        // The bar's own chip: the panel's colour inside its environment's
        // line. The others: the window's line, the text's when pointed at.
        let line = if chip.own {
            announce_switcher(&response, info, switchable, locale);
            ui.painter().rect_filled(*rect, corner, palette.panel);
            colors.base()
        } else {
            let label = switch_label(chip, look, locale);
            response
                .widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, &label));
            if response.hovered() {
                palette.dim
            } else {
                palette.outline
            }
        };
        ui.painter()
            .rect_stroke(*rect, corner, Stroke::new(1.0, line), StrokeKind::Inside);
        let style = badge_style(chip);
        let badge = env_badge(
            ui,
            rect.left() + 5.0,
            center,
            chip.env,
            &colors,
            style,
            look,
        );
        // A 13 pt name over a 12 pt line, centred on the bar's line; what
        // the chip has no room for is cut with an ellipsis.
        let (top, bottom) = (center - 7.5, center + 8.0);
        let x = rect.left() + 5.0 + badge + 8.0;
        let room = rect.width() - fixed;
        let role = name_role(chip);
        let name = crate::ui::grid::ellipsize(&chip.name, room, false, |text| {
            role.width(ui.ctx(), look.faces, text)
        });
        let name = Text::one(look, role, &name, palette.text).layout(ui.ctx());
        name.paint_left(ui.painter(), x, top);
        let line_color = if chip.link == Link::Connected {
            palette.dim
        } else {
            link_color(chip.link, &colors, palette)
        };
        let safe = display_safe(&chip.line);
        let line = crate::ui::grid::ellipsize(&safe, room, false, |text| {
            small.width(ui.ctx(), look.faces, text)
        });
        let line = Text::one(look, small, &line, line_color).layout(ui.ctx());
        line.paint_left(ui.painter(), x, bottom);
        if chip.own {
            // The bar's own connection is read as text too.
            for (laid, y) in [(&name, top), (&line, bottom)] {
                widgets::announce(
                    ui,
                    Rect::from_min_size(pos2(x, y - 8.0), vec2(laid.width().max(1.0), 16.0)),
                    laid.galley.text(),
                );
            }
            if switchable {
                widgets::paint_text(
                    ui,
                    rect.right() - 8.0 - arrow,
                    center,
                    Text::one(look, TextRole::OBody, "▾", palette.dim),
                );
                database_menu(&response, tab, info, look, actions);
            }
        } else if response.clicked() {
            actions.push(Action::ActivateConnTab(chip.tab));
        }
    }
    let mut x = rects.last().map_or(row.left(), |last| last.right() + 12.0);
    let line = tag_role.row_height(ui.ctx(), look.faces);
    for ((text, color), width) in tags.iter().zip(&tag_widths).take(shared.pills) {
        let rect = Rect::from_min_size(
            pos2(x, center - (line + 2.0) / 2.0),
            vec2(*width, line + 2.0),
        );
        ui.painter().rect_stroke(
            rect,
            CornerRadius::same(3),
            Stroke::new(1.0, connection_line),
            StrokeKind::Inside,
        );
        widgets::paint_label(ui, x + 8.0, center, Text::one(look, tag_role, text, *color));
        x = rect.right() + 12.0;
    }
}
```

- [ ] **Step 10: Point one older test at the bar's line**

The connection's name sits on the chip's upper line now, off the bar's own. In `src/ui/mod.rs`, in `the_mac_window_buttons_centre_on_the_line_of_the_bar_that_leads`, measure the bar by its Connections button:

```diff
-        let bar = middle_of(&tree, "Fixture");
+        let bar = middle_of(&tree, "Connections");
```

- [ ] **Step 11: Run the tests to verify they pass**

Run: `~/.cargo/bin/cargo fmt --all && ~/.cargo/bin/cargo test --locked -p tabletist --lib`
Expected: `test result: ok. 786 passed; 0 failed; 1 ignored` (780 before, two layout tests, three bar tests, one colour test).

Run: `~/.cargo/bin/cargo clippy --locked --workspace --all-targets -- -D warnings`
Expected: no warnings.

- [ ] **Step 12: Commit point**

Stop and report. If asked to commit:

```bash
git add src/typography.rs src/app.rs src/ui/workspace.rs src/ui/mod.rs src/ui/env_tests.rs
git commit -S -m "Show each open connection as a chip in the connection bar

The bar's own chip is its database switcher; another's switches to that
connection. Pills give way, then names shorten, when the row is full.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

## Task 2: One Connections screen, and no tab bar

**Files:**
- Delete: `src/ui/conn_tabs.rs`
- Modify: `src/model.rs`, `src/app.rs`, `src/ui/keys.rs`, `src/ui/workspace.rs`, `src/ui/picker.rs`, `src/ui/mod.rs`
- Modify (names and comments): `src/env.rs`, `src/theme.rs`, `src/ui/widgets.rs`, `src/entrypoint.rs`, `src/macos.rs`
- Test: `src/app.rs`, `src/ui/mod.rs`, `src/ui/keys.rs`, `src/ui/env_tests.rs` (tests modules)

- [ ] **Step 1: Rewrite the tests**

`src/app.rs`, tests module. `connect_another` and two tests are new; four tests open connections where they opened empty picker tabs:

```diff
--- a/src/app.rs
+++ b/src/app.rs
@@ -2901,15 +2920,65 @@
         assert!(matches!(app.tabs[0].content, ConnTabContent::Picker(_)));
     }
 
-    #[test]
-    fn new_tabs_open_at_the_end_and_become_active() {
+    /// Opens another connection: the picker, then a connect in it.
+    fn connect_another(app: &mut App) -> ConnTabId {
+        app.apply(Action::ShowConnections);
+        connect(app).0
+    }
+
+    #[test]
+    fn showing_the_connections_opens_one_picker_tab_and_returns_to_it() {
         let (mut app, _dir) = app();
-        app.apply(Action::NewConnTab);
-        app.apply(Action::NewConnTab);
-        assert_eq!(app.tabs.len(), 3);
-        assert_eq!(app.active, 2);
+        // The app starts on the picker: nothing to open.
+        app.apply(Action::ShowConnections);
+        assert_eq!(app.tabs.len(), 1);
+        let (first, _, _) = connect(&mut app);
+        app.apply(Action::ShowConnections);
+        assert_eq!(app.tabs.len(), 2);
+        assert_eq!(app.active, 1, "a new tab opens at the end and shows");
+        assert!(matches!(
+            app.active_tab().content,
+            ConnTabContent::Picker(_)
+        ));
+        let picker = app.active_tab_id();
+        // From the connection again, the same picker tab shows.
+        app.apply(Action::ActivateConnTab(first));
+        app.apply(Action::ShowConnections);
+        assert_eq!(app.tabs.len(), 2);
+        assert_eq!(app.active_tab_id(), picker);
         let unique: std::collections::HashSet<_> = ids(&app).into_iter().collect();
-        assert_eq!(unique.len(), 3, "tab ids must be unique");
+        assert_eq!(unique.len(), 2, "tab ids must be unique");
+    }
+
+    #[test]
+    fn disconnecting_shows_the_one_picker() {
+        let (mut app, _dir) = app();
+        // Alone, the connection's tab becomes the picker.
+        let (only, session, _) = connect(&mut app);
+        app.apply(Action::Disconnect(only));
+        assert_eq!(ids(&app), [only.0]);
+        assert!(matches!(
+            app.active_tab().content,
+            ConnTabContent::Picker(_)
+        ));
+        assert!(
+            matches!(app.backend.sent.last(), Some(Command::Close { session: s }) if *s == session)
+        );
+        // With the picker open in a tab of its own, the connection's tab
+        // closes and the picker shows.
+        let (first, session, _) = connect(&mut app);
+        let second = connect_another(&mut app);
+        app.apply(Action::ShowConnections);
+        let picker = app.active_tab_id();
+        app.apply(Action::Disconnect(first));
+        assert_eq!(ids(&app), [second.0, picker.0]);
+        assert_eq!(app.active_tab_id(), picker);
+        assert!(
+            app.backend
+                .sent
+                .iter()
+                .any(|command| matches!(command, Command::Close { session: s } if *s == session))
+        );
     }
 
     #[test]
@@ -2925,10 +2994,9 @@
     #[test]
     fn closing_a_tab_before_the_active_one_keeps_the_active_tab() {
         let (mut app, _dir) = app();
-        app.apply(Action::NewConnTab);
-        app.apply(Action::NewConnTab);
-        let active = app.active_tab_id();
-        let first = app.tabs[0].id;
+        let (first, _, _) = connect(&mut app);
+        connect_another(&mut app);
+        let active = connect_another(&mut app);
         app.apply(Action::CloseConnTab(first));
         assert_eq!(app.active_tab_id(), active);
     }
@@ -2936,9 +3004,9 @@
     #[test]
     fn closing_the_active_tab_activates_its_right_neighbour_or_the_new_last() {
         let (mut app, _dir) = app();
-        app.apply(Action::NewConnTab);
-        app.apply(Action::NewConnTab);
-        let [a, b, c] = [app.tabs[0].id, app.tabs[1].id, app.tabs[2].id];
+        let (a, _, _) = connect(&mut app);
+        let b = connect_another(&mut app);
+        let c = connect_another(&mut app);
         app.apply(Action::ActivateConnTab(b));
         app.apply(Action::CloseConnTab(b));
         assert_eq!(app.active_tab_id(), c);
@@ -2955,29 +3023,42 @@
     }
 
     #[test]
-    fn tabs_activate_by_index_and_cycle_with_wrapping() {
+    fn connections_activate_by_position_and_tabs_cycle_with_wrapping() {
         let (mut app, _dir) = app();
-        app.apply(Action::NewConnTab);
-        app.apply(Action::NewConnTab);
-        app.apply(Action::ActivateConnTabIndex(0));
+        let (first, _, _) = connect(&mut app);
+        let second = connect_another(&mut app);
+        // The picker, last, is a tab but not a connection.
+        app.apply(Action::ShowConnections);
+        app.apply(Action::ActivateConnection(0));
+        assert_eq!(app.active_tab_id(), first);
+        app.apply(Action::ActivateConnection(1));
+        assert_eq!(app.active_tab_id(), second);
+        app.apply(Action::ActivateConnection(2));
+        assert_eq!(app.active_tab_id(), second, "the picker has no number");
+        app.apply(Action::ActivateConnection(7));
+        assert_eq!(
+            app.active_tab_id(),
+            second,
+            "a position past the end is ignored"
+        );
+        app.apply(Action::CycleConnTab(1));
+        assert_eq!(app.active, 2, "cycling reaches the picker");
+        app.apply(Action::CycleConnTab(1));
         assert_eq!(app.active, 0);
-        app.apply(Action::ActivateConnTabIndex(7));
-        assert_eq!(app.active, 0, "an index past the end is ignored");
         app.apply(Action::CycleConnTab(-1));
         assert_eq!(app.active, 2);
-        app.apply(Action::CycleConnTab(1));
-        assert_eq!(app.active, 0);
     }
 
     #[test]
     fn queued_actions_are_drained_in_order() {
         let (mut app, _dir) = app();
-        app.actions.push(Action::NewConnTab);
-        app.actions.push(Action::ActivateConnTabIndex(0));
+        let (first, _, _) = connect(&mut app);
+        app.actions.push(Action::ShowConnections);
+        app.actions.push(Action::ActivateConnection(0));
         app.apply_actions();
         assert!(app.actions.is_empty());
         assert_eq!(app.tabs.len(), 2);
-        assert_eq!(app.active, 0);
+        assert_eq!(app.active_tab_id(), first);
     }
 
     use crate::backend::{Command, Event, RequestId, SessionId};
@@ -4296,7 +4377,7 @@
         }));
         assert_eq!(asked(&app), 1);
         // On a picker there is no workspace to open an editor in.
-        app.apply(Action::NewConnTab);
+        app.apply(Action::ShowConnections);
         let picker = app.active_tab_id();
         app.apply(Action::NewSqlTab(picker));
         assert!(app.workspace(picker).is_none());
```

`src/ui/mod.rs`, tests module. The tab bar's own tests go (`one_connection_needs_no_tab_bar`, `the_plus_button_opens_a_tab`, `the_close_button_closes_its_tab`, `the_space_beside_the_mac_window_buttons_stays_when_the_tabs_scroll`); the others say the same of the connection bar:

```diff
--- a/src/ui/mod.rs
+++ b/src/ui/mod.rs
@@ -225,38 +218,6 @@
     }
 
     #[test]
-    fn one_connection_needs_no_tab_bar() {
-        let mut harness = Harness::new();
-        assert!(!harness.has("New tab"));
-        assert!(!harness.has("New connection tab"));
-        harness.press(Key::O, Modifiers::COMMAND);
-        assert!(harness.has("New tab"));
-        assert!(harness.has("New connection tab"));
-    }
-
-    #[test]
-    fn the_plus_button_opens_a_tab() {
-        let mut harness = Harness::new();
-        harness.app.apply(crate::model::Action::NewConnTab);
-        harness.click("New connection tab");
-        assert_eq!(harness.app.tabs.len(), 3);
-        assert_eq!(harness.app.active, 2);
-    }
-
-    #[test]
-    fn the_close_button_closes_its_tab() {
-        let mut harness = Harness::new();
-        let closing = harness.app.active_tab_id();
-        let first = harness.connect_fake();
-        // Named apart from the saved connection the new tab's picker lists.
-        harness.app.workspace_mut(first).unwrap().name = "Tab one".into();
-        harness.app.apply(crate::model::Action::NewConnTab);
-        harness.click("Close Tab one");
-        assert_eq!(harness.app.tabs.len(), 1);
-        assert_ne!(harness.app.tabs[0].id, closing);
-    }
-
-    #[test]
     fn opening_an_object_puts_it_first_in_recent() {
         let mut harness = Harness::new();
         let tab = harness.connect_fake();
@@ -445,16 +406,31 @@
     }
 
     #[test]
-    fn ctrl_o_opens_a_tab() {
-        let mut harness = Harness::new();
+    fn ctrl_o_shows_the_connections() {
+        let mut harness = Harness::new();
+        // Already on the picker: nothing to open.
+        harness.press(Key::O, Modifiers::COMMAND);
+        assert_eq!(harness.app.tabs.len(), 1);
+        let tab = harness.connect_fake();
         harness.press(Key::O, Modifiers::COMMAND);
         assert_eq!(harness.app.tabs.len(), 2);
+        assert!(matches!(
+            harness.app.active_tab().content,
+            crate::model::ConnTabContent::Picker(_)
+        ));
+        // From the connection again, the same picker.
+        harness
+            .app
+            .apply(crate::model::Action::ActivateConnTab(tab));
+        harness.press(Key::O, Modifiers::COMMAND);
+        assert_eq!(harness.app.tabs.len(), 2);
     }
 
     #[test]
     fn ctrl_shift_w_closes_the_connection_tab() {
         let mut harness = Harness::new();
-        harness.app.apply(crate::model::Action::NewConnTab);
+        harness.connect_fake();
+        harness.app.apply(crate::model::Action::ShowConnections);
         let active = harness.app.active_tab_id();
         harness.press(Key::W, Modifiers::COMMAND | Modifiers::SHIFT);
         assert_eq!(harness.app.tabs.len(), 1);
@@ -464,22 +440,29 @@
     #[test]
     fn plain_ctrl_w_does_not_close_the_connection_tab() {
         let mut harness = Harness::new();
-        harness.app.apply(crate::model::Action::NewConnTab);
+        harness.connect_fake();
+        harness.app.apply(crate::model::Action::ShowConnections);
         harness.press(Key::W, Modifiers::COMMAND);
         assert_eq!(harness.app.tabs.len(), 2);
     }
 
     #[test]
-    fn number_shortcuts_and_ctrl_tab_switch_tabs() {
-        let mut harness = Harness::new();
-        harness.app.apply(crate::model::Action::NewConnTab);
-        harness.app.apply(crate::model::Action::NewConnTab);
+    fn number_shortcuts_switch_connections_and_ctrl_tab_every_tab() {
+        let mut harness = Harness::new();
+        let first = harness.connect_fake();
+        let second = connect_another(&mut harness, "Second");
+        harness.app.apply(crate::model::Action::ShowConnections);
         harness.press(Key::Num1, Modifiers::COMMAND);
-        assert_eq!(harness.app.active, 0);
+        assert_eq!(harness.app.active_tab_id(), first);
+        harness.press(Key::Num2, Modifiers::COMMAND);
+        assert_eq!(harness.app.active_tab_id(), second);
+        // The picker has no number, and is one of the tabs Ctrl+Tab visits.
+        harness.press(Key::Num3, Modifiers::COMMAND);
+        assert_eq!(harness.app.active_tab_id(), second);
         harness.press(Key::Tab, Modifiers::CTRL);
+        assert_eq!(harness.app.active, 2);
+        harness.press(Key::Tab, Modifiers::CTRL | Modifiers::SHIFT);
         assert_eq!(harness.app.active, 1);
-        harness.press(Key::Tab, Modifiers::CTRL | Modifiers::SHIFT);
-        assert_eq!(harness.app.active, 0);
     }
 
     #[test]
@@ -1981,13 +1964,13 @@
     #[test]
     fn the_shortcuts_of_any_tab_work_on_a_sql_tab() {
         let (mut harness, tab) = tree_harness();
-        harness.app.apply(crate::model::Action::NewConnTab);
+        harness.app.apply(crate::model::Action::ShowConnections);
         harness.press(Key::Num1, Modifiers::COMMAND);
         assert_eq!(harness.app.active_tab_id(), tab);
         harness.press(Key::T, Modifiers::COMMAND);
         let workspace = harness.app.workspace(tab).unwrap();
         assert!(workspace.active_sql_tab().is_some());
-        harness.press(Key::Num2, Modifiers::COMMAND);
+        harness.press(Key::Tab, Modifiers::CTRL);
         assert_eq!(harness.app.active, 1);
         harness.press(Key::Num1, Modifiers::COMMAND);
         assert_eq!(harness.app.active, 0);
@@ -4101,7 +4084,8 @@
     #[test]
     fn shortcuts_are_ignored_while_the_dialog_is_open() {
         let mut harness = Harness::new();
-        harness.app.apply(crate::model::Action::NewConnTab);
+        harness.connect_fake();
+        harness.app.apply(crate::model::Action::ShowConnections);
         harness.press(Key::N, Modifiers::COMMAND);
         if let Some(crate::model::Dialog::Connection(form)) = &mut harness.app.dialog {
             form.name = "typed".into();
@@ -6456,21 +6440,19 @@
     }
 
     #[test]
-    fn tabs_share_the_title_bar_with_the_mac_window_buttons() {
+    fn several_connections_share_the_title_bar_with_the_mac_window_buttons() {
         let mut harness = Harness::new();
         mac_title_bar(&mut harness);
-        let first = harness.connect_fake();
-        // Named apart from the saved connection the new tab's picker lists.
-        harness.app.workspace_mut(first).unwrap().name = "Tab one".into();
-        harness.app.apply(crate::model::Action::NewConnTab);
+        harness.connect_fake();
+        connect_another(&mut harness, "Second");
         let tree = harness.settle();
-        let tab = bounds_of(&tree, "Tab one");
-        assert!(tab.x0 >= 80.0, "tabs start after the buttons: {tab:?}");
-        let middle = (tab.y0 + tab.y1) / 2.0;
+        let button = bounds_of(&tree, "Connections");
         assert!(
-            (middle - 20.0).abs() < 1.0,
-            "centred on the buttons' line: {tab:?}"
-        );
+            button.x0 >= 80.0,
+            "the bar starts after the buttons: {button:?}"
+        );
+        let chip = bounds_of(&tree, "Switch to Fixture · dev");
+        assert!(chip.x0 > button.x1, "the chips follow: {chip:?}");
     }
 
     /// The middle of the node labelled (or valued) `label`.
@@ -6501,7 +6483,7 @@
         );
 
         // One connection: its bar, under the environment stripe.
-        let first = harness.connect_fake();
+        harness.connect_fake();
         let tree = harness.settle();
         let bar = middle_of(&tree, "Connections");
         assert!(
@@ -6511,34 +6493,40 @@
         );
         assert!(line(&harness) > 20.0, "below the title bar's own middle");
 
-        // Several: the tab bar.
-        harness.app.workspace_mut(first).unwrap().name = "Tab one".into();
-        harness.app.apply(crate::model::Action::NewConnTab);
+        // Several: the same bar, a chip for each.
+        connect_another(&mut harness, "Second");
         let tree = harness.settle();
-        let tab = bounds_of(&tree, "Tab one");
-        let tabs = (tab.y0 + tab.y1) / 2.0;
+        let bar = middle_of(&tree, "Connections");
         assert!(
-            (line(&harness) - tabs).abs() < 1.0,
-            "the tabs at {tabs}, the buttons at {}",
+            (line(&harness) - bar).abs() < 1.0,
+            "the connection bar at {bar}, the buttons at {}",
             line(&harness)
         );
+
+        // The picker again, with the connections open behind it: its header.
+        harness.press(Key::O, Modifiers::COMMAND);
+        let tree = harness.settle();
+        let header = middle_of(&tree, "New connection");
+        assert!(
+            (line(&harness) - header).abs() < 1.0,
+            "the picker's header at {header}, the buttons at {}",
+            line(&harness)
+        );
     }
 
     #[test]
     fn the_mac_title_bar_is_measured_in_window_points_not_zoomed_ones() {
         let mut harness = Harness::new();
         mac_title_bar(&mut harness);
-        let first = harness.connect_fake();
-        // Named apart from the saved connection the new tab's picker lists.
-        harness.app.workspace_mut(first).unwrap().name = "Tab one".into();
-        harness.app.apply(crate::model::Action::NewConnTab);
+        harness.connect_fake();
         harness.ctx.set_zoom_factor(2.0);
         let tree = harness.settle();
-        let tab = bounds_of(&tree, "Tab one");
-        // 80 window points are 40 egui points at 2x zoom.
+        let button = bounds_of(&tree, "Connections");
+        // 80 window points are 40 egui points at 2x zoom: the bar starts
+        // past those, well short of 80.
         assert!(
-            (tab.x0 - 40.0).abs() < 1.0,
-            "tabs start after the buttons: {tab:?}"
+            button.x0 >= 40.0 && button.x0 < 80.0,
+            "the bar starts after the buttons: {button:?}"
         );
     }
 
@@ -6556,48 +6544,6 @@
             .expect("the connection's name");
         assert!(name.x0 >= 80.0, "after the buttons: {name:?}");
         assert!(name.y1 < 40.0, "in the title bar: {name:?}");
-    }
-
-    #[test]
-    fn the_space_beside_the_mac_window_buttons_stays_when_the_tabs_scroll() {
-        let mut harness = Harness::new();
-        mac_title_bar(&mut harness);
-        for _ in 0..12 {
-            harness.app.apply(crate::model::Action::NewConnTab);
-        }
-        let before = bounds_of(&harness.settle(), "New tab");
-        let over = egui::pos2(400.0, 20.0);
-        harness.frame(vec![egui::Event::PointerMoved(over)]);
-        harness.frame(vec![
-            egui::Event::PointerMoved(over),
-            egui::Event::MouseWheel {
-                unit: egui::MouseWheelUnit::Point,
-                delta: egui::vec2(-400.0, 0.0),
-                modifiers: Modifiers::NONE,
-                phase: egui::TouchPhase::Move,
-            },
-        ]);
-        for _ in 0..60 {
-            harness.frame(vec![]);
-        }
-        let after = bounds_of(&harness.settle(), "New tab");
-        assert!(after.x0 < before.x0 - 100.0, "the tabs scrolled: {after:?}");
-
-        // The corner by the buttons still moves the window.
-        let at = egui::pos2(76.0, 20.0);
-        harness.frame(vec![egui::Event::PointerMoved(at)]);
-        harness.frame(vec![egui::Event::PointerButton {
-            pos: at,
-            button: egui::PointerButton::Primary,
-            pressed: true,
-            modifiers: Modifiers::NONE,
-        }]);
-        harness.frame(vec![egui::Event::PointerMoved(at + egui::vec2(30.0, 0.0))]);
-        assert!(
-            harness
-                .viewport_commands
-                .contains(&egui::ViewportCommand::StartDrag)
-        );
     }
 
     #[test]
```

`src/ui/keys.rs`, tests module:

```diff
--- a/src/ui/keys.rs
+++ b/src/ui/keys.rs
@@ -590,9 +590,9 @@
     fn the_shortcut_table_covers_the_spec_map() {
         let descriptions: Vec<&str> = SHORTCUTS.iter().map(|(_, what)| *what).collect();
         for expected in [
-            "New connection tab",
-            "Close connection tab",
-            "Switch connection tab",
+            "Connections",
+            "Close connection",
+            "Switch connection",
             "New connection",
             "New SQL editor",
             "Run statement / run all",
@@ -623,7 +623,7 @@
                 .map(|(keys, _)| *keys)
         };
         assert_eq!(keys("New SQL editor"), Some("Mod+T"));
-        assert_eq!(keys("New connection tab"), Some("Mod+O"));
+        assert_eq!(keys("Connections"), Some("Mod+O"));
         assert_eq!(
             keys("Run statement / run all"),
             Some("Mod+Return, Mod+Shift+Return")
```

`src/ui/env_tests.rs`: delete `a_connected_tabs_dot_is_its_environments_colour` (Task 1's `a_connections_chip_takes_its_environments_colour` says it of the chips).

- [ ] **Step 2: Run the tests to verify they fail**

Run: `~/.cargo/bin/cargo test --locked -p tabletist --lib`
Expected: FAIL to compile: `no variant ... named ShowConnections` and `ActivateConnection` in `model::Action`.

- [ ] **Step 3: The two actions**

`src/model.rs`:

```diff
--- a/src/model.rs
+++ b/src/model.rs
@@ -12,8 +12,8 @@
 #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
 pub struct ConnTabId(pub u64);
 
-/// One tab in the connection tab bar: one connection, or the picker that
-/// chooses one.
+/// One tab: one connection (a chip in the connection bar), or the picker
+/// that chooses one.
 #[derive(Debug)]
 pub struct ConnTab {
     pub id: ConnTabId,
@@ -44,12 +44,14 @@
 /// applies them after the frame is drawn.
 #[derive(Debug)]
 pub enum Action {
-    /// Open a new picker tab and make it active.
-    NewConnTab,
+    /// Cmd/Ctrl+O and the Connections button: show the saved connections,
+    /// in the picker tab there is, or in a new one at the end.
+    ShowConnections,
     CloseConnTab(ConnTabId),
     ActivateConnTab(ConnTabId),
-    /// Cmd/Ctrl+1..9: activate the tab at this position, if there is one.
-    ActivateConnTabIndex(usize),
+    /// Cmd/Ctrl+1..9: activate the open connection at this position, as the
+    /// header numbers them, if there is one.
+    ActivateConnection(usize),
     /// Ctrl+Tab (+1) and Ctrl+Shift+Tab (-1), wrapping around.
     CycleConnTab(isize),
     /// A result from the backend.
```

`src/app.rs` (above its tests). `TitleBar`'s and `titlebar`'s comments name the connection bar; `picker_index` is new:

```diff
--- a/src/app.rs
+++ b/src/app.rs
@@ -48,8 +48,8 @@
     adopt: bool,
 }
 
-/// The native title bar the tab bar shares: its height, and how far the
-/// window's own buttons (the macOS traffic lights) reach from the left.
+/// The native title bar the connection bar shares: its height, and how far
+/// the window's own buttons (the macOS traffic lights) reach from the left.
 #[derive(Debug, Clone, Copy, Default, PartialEq)]
 pub struct TitleBar {
     pub height: f32,
@@ -82,8 +82,8 @@
     pub host_keys: HostKeys,
     /// Why known_hosts.json could not be read; no new host is trusted then.
     pub host_keys_error: Option<String>,
-    /// The window's own title bar the tab bar shares (macOS), measured from
-    /// the window every frame; zero elsewhere.
+    /// The window's own title bar the connection bar shares (macOS),
+    /// measured from the window every frame; zero elsewhere.
     pub titlebar: TitleBar,
     /// The OS theme seen last frame, to notice light/dark switches.
     system_theme: Option<egui::Theme>,
@@ -231,6 +231,14 @@
         self.tabs.iter().position(|tab| tab.id == id)
     }
 
+    /// Where the picker tab is, when one is open. There is never more than
+    /// one: the saved connections are one screen.
+    fn picker_index(&self) -> Option<usize> {
+        self.tabs
+            .iter()
+            .position(|tab| matches!(tab.content, ConnTabContent::Picker(_)))
+    }
+
     /// Applies queued actions until none are left (an action may queue more).
     pub fn apply_actions(&mut self) {
         while !self.actions.is_empty() {
@@ -288,19 +296,23 @@
 
     pub fn apply(&mut self, action: Action) {
         match action {
-            Action::NewConnTab => {
-                let tab = self.picker_tab();
-                self.tabs.push(tab);
-                self.active = self.tabs.len() - 1;
-            }
+            Action::ShowConnections => match self.picker_index() {
+                Some(index) => self.active = index,
+                None => {
+                    let tab = self.picker_tab();
+                    self.tabs.push(tab);
+                    self.active = self.tabs.len() - 1;
+                }
+            },
             Action::CloseConnTab(id) => self.close_tab(id),
             Action::ActivateConnTab(id) => {
                 if let Some(index) = self.tab_index(id) {
                     self.active = index;
                 }
             }
-            Action::ActivateConnTabIndex(index) => {
-                if index < self.tabs.len() {
+            Action::ActivateConnection(position) => {
+                let tab = self.open_connections().nth(position).map(|(tab, _)| tab);
+                if let Some(index) = tab.and_then(|tab| self.tab_index(tab)) {
                     self.active = index;
                 }
             }
@@ -317,10 +329,17 @@
             Action::Disconnect(tab) => {
                 if let Some(workspace) = self.workspace(tab) {
                     let session = workspace.session;
-                    self.backend.send(Command::Close { session });
-                    self.close_editors(tab);
-                    if let Some(entry) = self.tabs.iter_mut().find(|t| t.id == tab) {
-                        entry.content = ConnTabContent::Picker(PickerState::default());
+                    if self.picker_index().is_some() {
+                        // The saved connections have a tab already: this
+                        // one closes, and that one shows.
+                        self.close_tab(tab);
+                        self.apply(Action::ShowConnections);
+                    } else {
+                        self.backend.send(Command::Close { session });
+                        self.close_editors(tab);
+                        if let Some(entry) = self.tabs.iter_mut().find(|t| t.id == tab) {
+                            entry.content = ConnTabContent::Picker(PickerState::default());
+                        }
                     }
                 }
             }
@@ -2085,7 +2104,7 @@
         }
         if connect {
             if !matches!(self.active_tab().content, ConnTabContent::Picker(_)) {
-                self.apply(Action::NewConnTab);
+                self.apply(Action::ShowConnections);
             }
             let tab = self.active_tab_id();
             let mut secrets = Secrets {
```

`src/ui/keys.rs` (above its tests):

```diff
--- a/src/ui/keys.rs
+++ b/src/ui/keys.rs
@@ -19,9 +19,9 @@
 
 /// Every shortcut, for the help dialog. `Mod` is Cmd on macOS, Ctrl elsewhere.
 pub const SHORTCUTS: &[(&str, &str)] = &[
-    ("Mod+O", "New connection tab"),
-    ("Mod+Shift+W", "Close connection tab"),
-    ("Mod+1…9, Ctrl+Tab, Ctrl+Shift+Tab", "Switch connection tab"),
+    ("Mod+O", "Connections"),
+    ("Mod+Shift+W", "Close connection"),
+    ("Mod+1…9, Ctrl+Tab, Ctrl+Shift+Tab", "Switch connection"),
     ("Mod+N", "New connection"),
     (
         "Mod+S, Mod+T, Mod+Enter",
@@ -156,13 +156,13 @@
         if in_workspace {
             key(Modifiers::COMMAND, Key::T, Action::NewSqlTab(active));
         }
-        key(Modifiers::COMMAND, Key::O, Action::NewConnTab);
+        key(Modifiers::COMMAND, Key::O, Action::ShowConnections);
         key(Modifiers::COMMAND, Key::N, Action::NewConnection);
         for (index, number) in NUMBERS.into_iter().enumerate() {
             key(
                 Modifiers::COMMAND,
                 number,
-                Action::ActivateConnTabIndex(index),
+                Action::ActivateConnection(index),
             );
         }
         if !on_sql {
```

- [ ] **Step 4: Remove the tab bar**

```bash
git rm src/ui/conn_tabs.rs
```

`src/ui/mod.rs` (above its tests):

```diff
--- a/src/ui/mod.rs
+++ b/src/ui/mod.rs
@@ -2,7 +2,6 @@
 //! state directly.
 
 pub mod about;
-pub mod conn_tabs;
 pub mod connect_dialog;
 pub mod data_view;
 #[cfg(test)]
@@ -34,10 +33,6 @@
 use crate::model::ConnTabContent;
 
 pub fn show(app: &mut App, ui: &mut egui::Ui) {
-    // One connection needs no tab bar: its own bar leads the window.
-    if app.tabs.len() > 1 {
-        conn_tabs::show(app, ui);
-    }
     notice(app, ui);
     let fill = app.palette.window;
     egui::CentralPanel::default()
@@ -96,9 +91,7 @@
 /// points below the window's top: on the line of the bar that leads the
 /// window, so the buttons and that bar's contents share one line.
 pub fn window_buttons_line(app: &App, zoom: f32) -> f32 {
-    if app.tabs.len() > 1 {
-        conn_tabs::line(app, zoom)
-    } else if matches!(app.active_tab().content, ConnTabContent::Picker(_)) {
+    if matches!(app.active_tab().content, ConnTabContent::Picker(_)) {
         picker::header_line(&app.look)
     } else {
         workspace::bar_line(app, zoom)
```

`src/ui/workspace.rs`. The bar always leads the window, so it is always the macOS title bar; its button shows the connections:

```diff
--- a/src/ui/workspace.rs
+++ b/src/ui/workspace.rs
@@ -22,15 +22,11 @@
 /// The macOS bar's stripe in the environment colour.
 const STRIPE: f32 = 3.0;
 
-/// The bar's height in the window. Alone, the bar is the macOS title bar
-/// and at least as tall as the window has it (window points, which egui's
-/// `zoom` scales away from its own).
+/// The bar's height in the window. The bar is the macOS title bar, and at
+/// least as tall as the window has it (window points, which egui's `zoom`
+/// scales away from its own).
 fn top_bar_height(app: &App, zoom: f32) -> f32 {
-    if app.tabs.len() == 1 {
-        bar_height(&app.look).max(app.titlebar.height / zoom)
-    } else {
-        bar_height(&app.look)
-    }
+    bar_height(&app.look).max(app.titlebar.height / zoom)
 }
 
 /// The line the bar's contents centre on, below its top: under the macOS
@@ -421,15 +417,9 @@
     };
     let env = crate::env::env_colors(info.env, crate::env::Platform::of(&look), &palette);
     let (tint, border) = (env.bar_bg(), env.bar_border());
-    // macOS: with one connection the bar is the title bar, beside the
-    // window buttons.
+    // macOS: the bar is the title bar, beside the window buttons.
     let zoom = ui.ctx().zoom_factor();
-    let alone = app.tabs.len() == 1;
-    let inset = if alone {
-        app.titlebar.inset / zoom
-    } else {
-        0.0
-    };
+    let inset = app.titlebar.inset / zoom;
     let height = top_bar_height(app, zoom);
     let mut actions = Vec::new();
     egui::Panel::top(egui::Id::new(("workspace-top", tab.0)))
@@ -537,7 +527,7 @@
             .on_hover_text(connections_hint(look, locale))
             .clicked()
         {
-            actions.push(Action::NewConnTab);
+            actions.push(Action::ShowConnections);
         }
     }
     // Disconnect, 12 in from the right: an icon and its label, 6 apart.
@@ -797,7 +787,7 @@
             .on_hover_text(connections_hint(look, locale))
             .clicked()
         {
-            actions.push(Action::NewConnTab);
+            actions.push(Action::ShowConnections);
         }
         x = button.right() + 12.0;
     }
```

`src/ui/picker.rs`, the same for the list's header:

```diff
--- a/src/ui/picker.rs
+++ b/src/ui/picker.rs
@@ -152,13 +152,9 @@
     let total = app.connections.connections.len();
     let mut actions = Vec::new();
     let full = ui.max_rect();
-    // macOS: alone, the header is the title bar, beside the window buttons.
+    // macOS: the header is the title bar, beside the window buttons.
     let zoom = ui.ctx().zoom_factor();
-    let inset = if app.tabs.len() == 1 {
-        app.titlebar.inset / zoom
-    } else {
-        0.0
-    };
+    let inset = app.titlebar.inset / zoom;
     let backdrop = if look.terminal {
         palette.window
     } else {
```

- [ ] **Step 5: Take the tab bar's name out of a list and four comments**

```diff
--- a/src/env.rs
+++ b/src/env.rs
@@ -609,7 +609,6 @@
             OWNER,
             "src/ui/workspace.rs",
             "src/ui/picker.rs",
-            "src/ui/conn_tabs.rs",
             "src/ui/connect_dialog/mod.rs",
             "src/ui/env_tests.rs",
         ];
```

```diff
--- a/src/theme.rs
+++ b/src/theme.rs
@@ -329,7 +329,7 @@
     pub name: &'static str,
     /// Controls, tree rows, object tabs, icon-button hover, cards.
     pub radius: u8,
-    /// Connection tabs.
+    /// Menus and the disconnected banner.
     pub tab_radius: u8,
     /// Dialogs and popups.
     pub dialog_radius: u8,
```

```diff
--- a/src/ui/widgets.rs
+++ b/src/ui/widgets.rs
@@ -273,9 +273,8 @@
 /// How far a raised tab sits inside its track.
 const TRACK_INSET: f32 = 2.0;
 
-/// A tab's background: connection tabs (`radius` = `look.tab_radius`) and
-/// object tabs (`look.radius`). Raised tabs sit in a [`TabTrack`], which
-/// fills the inactive ones.
+/// A tab's background: the object tabs, with `radius` = `look.radius`.
+/// Raised tabs sit in a [`TabTrack`], which fills the inactive ones.
 pub fn tab(
     ui: &Ui,
     rect: Rect,
```

```diff
--- a/src/entrypoint.rs
+++ b/src/entrypoint.rs
@@ -228,8 +228,8 @@
             viewport
         }
     };
-    // macOS: no separate title strip. The connection tabs sit in the title
-    // bar next to the window buttons, as in Safari.
+    // macOS: no separate title strip. The connection bar sits in the title
+    // bar next to the window buttons, as Safari's tabs do.
     let viewport = if cfg!(target_os = "macos") {
         viewport
             .with_fullsize_content_view(true)
```

```diff
--- a/src/macos.rs
+++ b/src/macos.rs
@@ -1,10 +1,11 @@
-//! macOS: the tabs share a unified title bar with the window buttons.
+//! macOS: the connection bar shares a unified title bar with the window
+//! buttons.
 //!
 //! The window gets an empty toolbar in the compact style, so AppKit makes
-//! the title bar tall enough for the tab bar (the look of Safari's compact
-//! tabs and Xcode). Every frame reads back the title bar's height and where
-//! the buttons end, so the layout follows the running system (the buttons
-//! grew in macOS 26) and fullscreen, where AppKit hides them.
+//! the title bar tall enough for a bar of its own (the look of Safari's
+//! compact tabs and Xcode). Every frame reads back the title bar's height
+//! and where the buttons end, so the layout follows the running system (the
+//! buttons grew in macOS 26) and fullscreen, where AppKit hides them.
 //!
 //! AppKit centres the buttons in its own title bar, but the bar that leads
 //! the window (the connection bar, the picker's header) can be taller, so
```

- [ ] **Step 6: Run the tests to verify they pass**

Run: `~/.cargo/bin/cargo fmt --all && ~/.cargo/bin/cargo test --locked -p tabletist --lib`
Expected: `test result: ok. 782 passed; 0 failed; 1 ignored`.

Run: `~/.cargo/bin/cargo clippy --locked --workspace --all-targets -- -D warnings`
Expected: no warnings. `grep -rn "conn_tabs\|NewConnTab\|ActivateConnTabIndex" src` finds nothing.

- [ ] **Step 7: Commit point**

Stop and report. If asked to commit:

```bash
git add -A src
git commit -S -m "Keep the saved connections on one screen and drop the tab bar

The connection bar's chips do the tab bar's work. Cmd/Ctrl+O and the
Connections button show the one picker tab; Cmd/Ctrl+1..9 count the open
connections; Disconnect closes the tab when the picker is already open.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

## Task 3: A chip's card

**Files:**
- Modify: `src/model.rs` (`Workspace`), `src/app.rs` (`Event::Connected`), `src/ui/workspace.rs`
- Test: `src/ui/workspace.rs`, `src/app.rs` (tests modules)

- [ ] **Step 1: Write the failing tests**

In `src/ui/workspace.rs`, in `mod tests`, above `the_connections_hint_names_the_key_as_the_look_spells_it`:

```rust
    #[test]
    fn a_chips_card_says_where_the_connection_points_and_how_it_stands() {
        let locale = crate::i18n::Locale::default();
        let look = Look::macos();
        let row = |name: &str, value: &str| (name.to_owned(), value.to_owned());
        // The fixture: a SQLite file, still connecting.
        let mut workspace = crate::testing::workspace();
        assert_eq!(
            card_rows(&workspace, &look, locale, 0),
            [
                row("File", "/tmp/fixture.db"),
                row("Server", "SQLite"),
                row("Security", "Local · no SSH"),
                row("Status", "Connecting…"),
            ]
        );
        // A server behind a bastion, connected two hours ago.
        let now = 1_790_683_200;
        let (mut spec, _) = tabletist_db::ConnectSpec::from_url(
            "postgres://app@db.example.com:5432/bookshop_production?sslmode=verify-full",
        )
        .unwrap();
        spec.ssh = Some(tabletist_db::SshSpec {
            host: "bastion".into(),
            port: Some(22),
            user: "deploy".into(),
            auth: tabletist_db::SshAuth::Agent,
        });
        workspace.spec = spec;
        workspace.driver = tabletist_db::Driver::Postgres;
        workspace.status = SessionStatus::Connected;
        workspace.encrypted = true;
        workspace.connected_at = Some(now - 7_200);
        assert_eq!(
            card_rows(&workspace, &look, locale, now),
            [
                row("Host", "db.example.com:5432"),
                row("Database", "bookshop_production"),
                row("User", "app"),
                row("Server", "PostgreSQL"),
                row("Security", "TLS verified · SSH via bastion"),
                row("Connected", "2 h ago · no tabs open"),
            ]
        );
        // The terminal look says its own words in lower case, never a name.
        let terminal = card_rows(&workspace, &Look::omarchy(), locale, now);
        assert_eq!(terminal[0], row("host", "db.example.com:5432"));
        assert_eq!(
            terminal[4],
            row("security", "tls verified · ssh via bastion")
        );
    }

    #[test]
    fn a_chips_card_says_what_a_click_does() {
        let locale = crate::i18n::Locale::default();
        let chip = |own: bool| Chip {
            tab: ConnTabId(1),
            own,
            name: "Fixture".into(),
            env: crate::env::Environment::Dev,
            line: "fixture.db".into(),
            link: Link::Connected,
            number: 1,
            card: Vec::new(),
        };
        let hint = |own: bool, switchable: bool, look: &Look| {
            card_hint(&chip(own), switchable, look, locale)
        };
        let mac = Look::macos();
        assert_eq!(
            hint(false, false, &mac).as_deref(),
            Some("Click to switch to this connection")
        );
        assert_eq!(
            hint(true, true, &mac).as_deref(),
            Some("Click to switch database")
        );
        // The bar's own chip with one database does nothing: no hint.
        assert_eq!(hint(true, false, &mac), None);
        assert_eq!(
            hint(false, true, &Look::omarchy()).as_deref(),
            Some("click to switch to this connection")
        );
    }
```

In `src/app.rs`, the test of a session connecting also checks the time is kept:

```diff
--- a/src/app.rs
+++ b/src/app.rs
@@ -3118,6 +3119,7 @@
             app.workspace(tab).unwrap().status,
             SessionStatus::Connected
         ));
+        assert!(app.workspace(tab).unwrap().connected_at.is_some());
     }
 
     #[test]
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `~/.cargo/bin/cargo test --locked -p tabletist --lib -- a_chips_card`
Expected: FAIL to compile: `cannot find function card_rows`, `cannot find function card_hint`, `no field connected_at`.

- [ ] **Step 3: Remember when a session connected**

```diff
--- a/src/model.rs
+++ b/src/model.rs
@@ -394,6 +394,9 @@
     pub focus_where: bool,
     /// Fold or unfold the row panel's documents on the next frame (`za`).
     pub fold_documents: Option<TabId>,
+    /// When the session last connected, in seconds since the Unix epoch
+    /// (the connection bar's card says how long ago).
+    pub connected_at: Option<u64>,
 }
 
 /// How many objects the sidebar's Recent section keeps.
@@ -1991,6 +1994,7 @@
             full_precision: false,
             focus_where: false,
             fold_documents: None,
+            connected_at: None,
         }
     }
 
```

```diff
--- a/src/app.rs
+++ b/src/app.rs
@@ -1571,6 +1571,7 @@
                             workspace.status = SessionStatus::Connected;
                             workspace.driver = driver;
                             workspace.encrypted = encrypted;
+                            workspace.connected_at = Some(crate::util::now_secs());
                             Some(tab)
                         }
                         _ => None,
```

- [ ] **Step 4: Build and show the card**

`src/ui/workspace.rs` (above its tests). `Chip` gains its number and the card's lines; `card_rows`, `card_hint` and `chip_card` are new, after `chips`; both painters hang the card on the chip's response:

```diff
--- a/src/ui/workspace.rs
+++ b/src/ui/workspace.rs
@@ -182,6 +182,11 @@
     /// session stands while it is not connected.
     line: String,
     link: Link,
+    /// Its place among the open connections, from 1: Cmd/Ctrl+1 is the
+    /// first.
+    number: usize,
+    /// What its card says: a name and a value per line.
+    card: Vec<(String, String)>,
 }
 
 fn bar_info(app: &App, tab: ConnTabId) -> Option<BarInfo> {
@@ -214,8 +219,10 @@
 /// A chip for each open connection, in the header's order. `own` is the
 /// connection whose bar this is.
 fn chips(app: &App, own: ConnTabId) -> Vec<Chip> {
+    let now = crate::util::now_secs();
     app.open_connections()
-        .map(|(tab, workspace)| {
+        .enumerate()
+        .map(|(index, (tab, workspace))| {
             let (link, state) = match &workspace.status {
                 SessionStatus::Connected => (Link::Connected, None),
                 SessionStatus::Connecting { .. } => (Link::Connecting, Some("Connecting…")),
@@ -233,9 +240,149 @@
                 env: workspace.environment,
                 line,
                 link,
+                number: index + 1,
+                card: card_rows(workspace, &app.look, app.locale, now),
             }
         })
         .collect()
+}
+
+/// What a chip's card says about its connection: where it points, what
+/// serves it, how far it can be trusted, and how it stands.
+fn card_rows(
+    workspace: &crate::model::Workspace,
+    look: &Look,
+    locale: crate::i18n::Locale,
+    now: u64,
+) -> Vec<(String, String)> {
+    let say = |text: &'static str| look.label(&gettext(locale, text));
+    let spec = &workspace.spec;
+    let sqlite = workspace.driver == tabletist_db::Driver::Sqlite;
+    let mut rows = Vec::new();
+    if sqlite {
+        let path = spec
+            .sqlite_path
+            .as_ref()
+            .map(|path| path.display().to_string())
+            .unwrap_or_default();
+        rows.push((say("File"), path));
+    } else {
+        rows.push((say("Host"), format!("{}:{}", spec.host, spec.port)));
+        if !spec.database.is_empty() {
+            rows.push((say("Database"), display_safe(&spec.database).into_owned()));
+        }
+        if !spec.user.is_empty() {
+            rows.push((say("User"), spec.user.clone()));
+        }
+    }
+    let server = workspace
+        .server_version
+        .value
+        .clone()
+        .unwrap_or_else(|| workspace.driver.label().to_owned());
+    rows.push((say("Server"), server));
+    let connected = matches!(workspace.status, SessionStatus::Connected);
+    let remote = !sqlite && !crate::model::is_local_host(&spec.host);
+    let tls = if remote {
+        let encrypted = connected.then_some(workspace.encrypted);
+        say(tls_status(spec.effective_tls(), encrypted).0)
+    } else {
+        say("Local")
+    };
+    let ssh = match &spec.ssh {
+        Some(ssh) => format!("{} {}", say("SSH via"), ssh.host),
+        None => say("no SSH"),
+    };
+    rows.push((say("Security"), format!("{tls} · {ssh}")));
+    let state = match &workspace.status {
+        SessionStatus::Connected => {
+            let tabs = match workspace.tabs.len() {
+                0 => say("no tabs open"),
+                1 => say("1 tab open"),
+                count => format!("{count} {}", say("tabs open")),
+            };
+            let since = crate::connections::when(workspace.connected_at, now);
+            (say("Connected"), format!("{since} · {tabs}"))
+        }
+        SessionStatus::Connecting { .. } => (say("Status"), say("Connecting…")),
+        SessionStatus::Disconnected(_) => (say("Status"), say("Disconnected")),
+        SessionStatus::Cancelled => (say("Status"), say("Cancelled")),
+    };
+    rows.push(state);
+    rows
+}
+
+/// What a click on `chip` does, as its card says it. The bar's own chip
+/// without other databases to switch to does nothing.
+fn card_hint(
+    chip: &Chip,
+    switchable: bool,
+    look: &Look,
+    locale: crate::i18n::Locale,
+) -> Option<String> {
+    let text = if !chip.own {
+        "Click to switch to this connection"
+    } else if switchable {
+        "Click to switch database"
+    } else {
+        return None;
+    };
+    Some(look.label(&gettext(locale, text)))
+}
+
+/// A chip's card, shown while the pointer rests on it: the connection and
+/// its key, what [`card_rows`] says, and what a click does.
+fn chip_card(ui: &mut egui::Ui, chip: &Chip, hint: Option<&str>, look: &Look, palette: &Palette) {
+    let small = widgets::secondary(look);
+    let strong = TextRole::pick(look, TextRole::UiBodySemibold, TextRole::OGroup);
+    let colors = crate::env::env_colors(chip.env, crate::env::Platform::of(look), palette);
+    let badge = if look.terminal {
+        Badge::Tracked
+    } else {
+        Badge::Chip
+    };
+    ui.spacing_mut().item_spacing = vec2(6.0, 6.0);
+    ui.horizontal(|ui| {
+        let name = Text::one(look, strong, &chip.name, palette.text)
+            .layout(ui.ctx())
+            .label(ui);
+        let width = env_badge_width(ui, chip.env, badge, look);
+        let (place, _) = ui.allocate_exact_size(vec2(width, name.rect.height()), Sense::hover());
+        env_badge(
+            ui,
+            place.left(),
+            place.center().y,
+            chip.env,
+            &colors,
+            badge,
+            look,
+        );
+        if chip.number <= 9 {
+            let key = look.label(&format!("{}{}", look.command_key(), chip.number));
+            Text::one(look, small, &key, palette.dim)
+                .layout(ui.ctx())
+                .label(ui);
+        }
+    });
+    egui::Grid::new(("chip-card", chip.tab.0))
+        .num_columns(2)
+        .spacing(vec2(10.0, 6.0))
+        .show(ui, |ui| {
+            for (label, value) in &chip.card {
+                Text::one(look, small, label, palette.dim)
+                    .layout(ui.ctx())
+                    .label(ui);
+                Text::one(look, small, value, palette.text)
+                    .layout(ui.ctx())
+                    .label(ui);
+                ui.end_row();
+            }
+        });
+    if let Some(hint) = hint {
+        Text::one(look, small, hint, palette.dim)
+            .layout(ui.ctx())
+            .label(ui);
+    }
 }
 
 /// Where a connection points, in a word: its database, or the file's name
@@ -636,7 +783,10 @@
             continue;
         }
         let colors = crate::env::env_colors(chip.env, platform, palette);
-        let response = ui.interact(hit, ui.id().with(("chip", chip.tab.0)), Sense::click());
+        let hint = card_hint(chip, switchable, look, locale);
+        let response = ui
+            .interact(hit, ui.id().with(("chip", chip.tab.0)), Sense::click())
+            .on_hover_ui(|ui| chip_card(ui, chip, hint.as_deref(), look, palette));
         let (name_color, fill) = if chip.own {
             announce_switcher(&response, info, switchable, locale);
             (palette.text, Some(face(0.85)))
@@ -895,7 +1045,10 @@
             continue;
         }
         let colors = crate::env::env_colors(chip.env, platform, palette);
-        let response = ui.interact(hit, ui.id().with(("chip", chip.tab.0)), Sense::click());
+        let hint = card_hint(chip, switchable, look, locale);
+        let response = ui
+            .interact(hit, ui.id().with(("chip", chip.tab.0)), Sense::click())
+            .on_hover_ui(|ui| chip_card(ui, chip, hint.as_deref(), look, palette));
         // The bar's own chip: the panel's colour inside its environment's
         // line. The others: the window's line, the text's when pointed at.
         let line = if chip.own {
```

The card is egui's tooltip: it shows while the pointer rests on a chip, and not while the chip's own menu is open.

- [ ] **Step 5: Run the tests to verify they pass**

Run: `~/.cargo/bin/cargo fmt --all && ~/.cargo/bin/cargo test --locked -p tabletist --lib`
Expected: `test result: ok. 784 passed; 0 failed; 1 ignored`.

Run: `~/.cargo/bin/cargo clippy --locked --workspace --all-targets -- -D warnings`
Expected: no warnings.

- [ ] **Step 6: Commit point**

Stop and report. If asked to commit:

```bash
git add src/model.rs src/app.rs src/ui/workspace.rs
git commit -S -m "Say where a connection points in a card under its chip

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

## Task 4: The Connections list shows what is open

**Files:**
- Modify: `src/app.rs` (`tab_showing`, above `tab_for_session`), `src/ui/picker.rs`, `src/ui/keys.rs`
- Test: `src/app.rs`, `src/ui/mod.rs` (tests modules), `src/ui/picker.rs` (a new tests module)

- [ ] **Step 1: Write the failing tests**

`src/app.rs`, tests module:

```diff
--- a/src/app.rs
+++ b/src/app.rs
@@ -2952,6 +2959,23 @@
     }
 
     #[test]
+    fn a_saved_connection_knows_the_tab_that_has_it_open() {
+        let (mut app, _dir) = app();
+        let (tab, _, _) = connect(&mut app);
+        let conn = app.workspace(tab).unwrap().conn_id.clone();
+        assert_eq!(app.tab_showing(&conn), Some(tab));
+        assert_eq!(app.tab_showing(&ConnectionId::new()), None);
+        // Open twice, the first tab is the one that shows.
+        app.apply(Action::ShowConnections);
+        let second = app.active_tab_id();
+        app.apply(Action::Connect {
+            tab: second,
+            conn: conn.clone(),
+        });
+        assert_eq!(app.tab_showing(&conn), Some(tab));
+    }
+
+    #[test]
     fn disconnecting_shows_the_one_picker() {
         let (mut app, _dir) = app();
         // Alone, the connection's tab becomes the picker.
```

`src/ui/mod.rs`, tests module:

```diff
--- a/src/ui/mod.rs
+++ b/src/ui/mod.rs
@@ -807,6 +807,49 @@
     }
 
     #[test]
+    fn the_picker_shows_an_open_connection_instead_of_connecting_again() {
+        for look in crate::theme::Look::ALL {
+            let mut harness = Harness::new();
+            harness.set_look(look);
+            assert!(!harness.has("open"), "{}", look.name);
+            let tab = harness.connect_fake();
+            harness.app.apply(crate::model::Action::ShowConnections);
+            // The saved connection is marked, and its button shows it.
+            assert!(harness.has("open"), "{}", look.name);
+            assert!(!harness.has("Connect to Fixture"), "{}", look.name);
+            let sent = harness.app.backend.sent.len();
+            harness.click("Show Fixture");
+            assert_eq!(harness.app.active_tab_id(), tab, "{}", look.name);
+            assert_eq!(harness.app.backend.sent.len(), sent, "{}", look.name);
+            assert_eq!(harness.app.tabs.len(), 2, "the picker stays for next time");
+        }
+    }
+
+    #[test]
+    fn enter_shows_an_open_connection_and_shift_enter_opens_it_again() {
+        let mut harness = Harness::new();
+        let tab = harness.connect_fake();
+        let conn = harness.app.workspace(tab).unwrap().conn_id.clone();
+        harness.app.apply(crate::model::Action::ShowConnections);
+        let picker = harness.app.active_tab_id();
+        harness.app.apply(crate::model::Action::SelectConnection {
+            tab: picker,
+            conn: Some(conn),
+        });
+        harness.press(Key::Enter, Modifiers::NONE);
+        assert_eq!(harness.app.active_tab_id(), tab);
+        assert_eq!(harness.app.open_connections().count(), 1);
+        harness.app.apply(crate::model::Action::ShowConnections);
+        harness.press(Key::Enter, Modifiers::SHIFT);
+        assert_eq!(
+            harness.app.open_connections().count(),
+            2,
+            "a second session of the same connection"
+        );
+        assert_eq!(harness.app.active_tab_id(), picker, "in the picker's tab");
+    }
+
+    #[test]
     fn the_picker_has_a_new_connection_button() {
         let mut harness = Harness::new();
         harness.click("New connection");
```

`src/ui/picker.rs` has no tests yet. At its end:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn choosing_a_row_shows_an_open_connection_and_connects_another() {
        let connection = SavedConnection {
            id: ConnectionId::new(),
            name: "Fixture".into(),
            environment: crate::env::Environment::Dev,
            read_only: None,
            password: crate::connections::PasswordMode::None,
            ssh_secret: crate::connections::PasswordMode::None,
            spec: tabletist_db::ConnectSpec::sqlite("/tmp/fixture.db"),
        };
        let picker = crate::model::ConnTabId(1);
        let open = crate::model::ConnTabId(2);
        let connects = |action: Action| {
            let Action::Connect { tab, conn } = action else {
                return false;
            };
            tab == picker && conn == connection.id
        };
        // Not open: choosing it connects in the picker's tab.
        let closed = Opening {
            tab: picker,
            open: None,
        };
        assert!(connects(closed.choose(&connection)));
        // Open: choosing it shows that tab, and it can still connect again
        // (the row menu's Connect again, and Shift+Enter).
        let opened = Opening {
            tab: picker,
            open: Some(open),
        };
        assert!(matches!(
            opened.choose(&connection),
            Action::ActivateConnTab(tab) if tab == open
        ));
        assert!(connects(opened.connect(&connection)));
    }
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `~/.cargo/bin/cargo test --locked -p tabletist --lib -- open`
Expected: FAIL to compile: `no method named tab_showing`, `cannot find struct ... Opening`.

- [ ] **Step 3: Find the tab that has a saved connection open**

```diff
--- a/src/app.rs
+++ b/src/app.rs
@@ -189,6 +189,13 @@
             ConnTabContent::Workspace(workspace) => Some((tab.id, &**workspace)),
             ConnTabContent::Picker(_) => None,
         })
+    }
+
+    /// The first tab that has the saved connection `conn` open.
+    pub fn tab_showing(&self, conn: &crate::connections::ConnectionId) -> Option<ConnTabId> {
+        self.open_connections()
+            .find(|(_, workspace)| workspace.conn_id == *conn)
+            .map(|(tab, _)| tab)
     }
 
     pub fn tab_for_session(&self, session: SessionId) -> Option<ConnTabId> {
```

- [ ] **Step 4: Mark and show open connections in the list**

`src/ui/picker.rs` (above its tests). A row is told where its connection opens (`Opening`); choosing it shows an open connection and connects another; the macOS row gets the `open` tag and its button's words from `choose_labels`; the Omarchy row the word and the footer's hint:

```diff
--- a/src/ui/picker.rs
+++ b/src/ui/picker.rs
@@ -312,7 +312,7 @@
     if look.terminal {
         let hints = [
             ("j/k", "move", true),
-            ("enter", "connect", true),
+            ("enter", "connect / show", true),
             ("e", "edit", true),
             ("n", "new", true),
             ("yy", "duplicate", true),
@@ -412,7 +412,10 @@
                                 connection,
                                 &app.connections,
                                 selected.as_ref(),
-                                tab,
+                                Opening {
+                                    tab,
+                                    open: app.tab_showing(&connection.id),
+                                },
                                 skin,
                                 &mut actions,
                             );
@@ -439,7 +442,10 @@
                                 connection,
                                 &app.connections,
                                 selected.as_ref(),
-                                tab,
+                                Opening {
+                                    tab,
+                                    open: app.tab_showing(&connection.id),
+                                },
                                 skin,
                                 &mut actions,
                             );
@@ -539,6 +545,33 @@
     side: f32,
 }
 
+/// Where a row's connection opens: the picker's tab, or the tab that has
+/// it open already.
+#[derive(Clone, Copy)]
+struct Opening {
+    tab: crate::model::ConnTabId,
+    open: Option<crate::model::ConnTabId>,
+}
+
+impl Opening {
+    /// What choosing the row does: an open connection shows, another
+    /// connects in the picker's tab.
+    fn choose(self, connection: &SavedConnection) -> Action {
+        match self.open {
+            Some(open) => Action::ActivateConnTab(open),
+            None => self.connect(connection),
+        }
+    }
+
+    /// Connects in the picker's tab, open already or not.
+    fn connect(self, connection: &SavedConnection) -> Action {
+        Action::Connect {
+            tab: self.tab,
+            conn: connection.id.clone(),
+        }
+    }
+}
+
 /// Where a connection points, as the rows say it: host and port, then the
 /// database (a file's name for SQLite).
 fn target(connection: &SavedConnection) -> (String, String) {
@@ -553,15 +586,17 @@
 }
 
 /// The row's own response, answering a click by selecting and a double
-/// click, Enter or a screen reader by connecting.
+/// click, Enter or a screen reader by choosing it: an open connection
+/// shows, another connects.
 fn row_response(
     ui: &mut egui::Ui,
     rect: Rect,
     connection: &SavedConnection,
-    tab: crate::model::ConnTabId,
+    opening: Opening,
     look: &Look,
     actions: &mut Vec<Action>,
 ) -> egui::Response {
+    let tab = opening.tab;
     let response = ui.interact(
         rect,
         ui.id().with(("connection", &connection.id.0)),
@@ -571,10 +606,7 @@
     let activated = response.double_clicked()
         || (response.clicked() && !response.clicked_by(egui::PointerButton::Primary));
     if activated {
-        actions.push(Action::Connect {
-            tab,
-            conn: connection.id.clone(),
-        });
+        actions.push(opening.choose(connection));
     } else if response.clicked() {
         actions.push(Action::SelectConnection {
             tab,
@@ -587,11 +619,16 @@
         let item = |ui: &mut egui::Ui, text: &'static str| {
             widgets::button(ui, &gettext(locale, text), look).clicked()
         };
-        if item(ui, "Connect") {
-            actions.push(Action::Connect {
-                tab,
-                conn: id.clone(),
-            });
+        // An open connection shows, and can be opened once more.
+        if opening.open.is_some() {
+            if item(ui, "Show") {
+                actions.push(opening.choose(connection));
+            }
+            if item(ui, "Connect again") {
+                actions.push(opening.connect(connection));
+            }
+        } else if item(ui, "Connect") {
+            actions.push(opening.connect(connection));
         }
         if item(ui, "Edit…") {
             actions.push(Action::EditConnection(id.clone()));
@@ -604,6 +641,24 @@
         }
     });
     response
+}
+
+/// The words on the button that chooses a row, and its accessible name:
+/// Show for a connection that is open, Connect for another.
+fn choose_labels(
+    connection: &SavedConnection,
+    opening: Opening,
+    locale: crate::i18n::Locale,
+) -> (String, String) {
+    let (text, label) = if opening.open.is_some() {
+        ("Show", "Show")
+    } else {
+        ("Connect", "Connect to")
+    };
+    (
+        gettext(locale, text).into_owned(),
+        format!("{} {}", gettext(locale, label), connection.name),
+    )
 }
 
 /// macOS: a row of the card, a colour bar at its left edge, columns for
@@ -616,7 +671,7 @@
     connection: &SavedConnection,
     store: &crate::connections::SavedConnections,
     selected: Option<&ConnectionId>,
-    tab: crate::model::ConnTabId,
+    opening: Opening,
     skin: RowSkin<'_>,
     actions: &mut Vec<Action>,
 ) {
@@ -633,7 +688,7 @@
         pos2(card.left(), rect.top()),
         pos2(card.right(), rect.bottom()),
     );
-    let response = row_response(ui, rect, connection, tab, look, actions);
+    let response = row_response(ui, rect, connection, opening, look, actions);
     let is_selected = selected == Some(&connection.id);
     let rows = (card.height() / height).round() as usize;
     // The card's inner corners: its 10 less its border.
@@ -692,7 +747,7 @@
             palette.text,
         ),
     );
-    super::workspace::env_badge(
+    let badge = super::workspace::env_badge(
         ui,
         name_x + name_width + 8.0,
         top,
@@ -701,6 +756,29 @@
         super::workspace::Badge::Mac,
         look,
     );
+    if opening.open.is_some() {
+        // Open in a tab already: an outlined tag in the accent, 8 on.
+        let text = gettext(locale, "open");
+        let laid = Text::one(look, TextRole::Shortcut, &text, palette.accent).layout(ui.ctx());
+        let tag = Rect::from_min_size(
+            pos2(
+                name_x + name_width + 8.0 + badge + 8.0,
+                top - (laid.height() + 2.0) / 2.0,
+            ),
+            vec2(laid.width() + 14.0, laid.height() + 2.0),
+        );
+        ui.painter().rect_stroke(
+            tag,
+            CornerRadius::same(10),
+            Stroke::new(
+                widgets::hairline(ui),
+                palette.window.lerp_to_gamma(palette.accent, 0.45),
+            ),
+            StrokeKind::Inside,
+        );
+        laid.paint_left(ui.painter(), tag.left() + 7.0, top);
+        widgets::announce(ui, tag, &text);
+    }
     let user = &connection.spec.user;
     let second = if user.is_empty() {
         connection.spec.driver.label().to_owned()
@@ -767,12 +845,12 @@
             palette.dim,
         ),
     );
-    // Edit, more, and Connect: on the selected row and under the pointer.
-    // Connect is always there for keyboards and screen readers.
+    // Edit, more, and Connect (Show, for a connection that is open): on
+    // the selected row and under the pointer. Connect is always there for
+    // keyboards and screen readers.
     let shown = is_selected || ui.rect_contains_pointer(rect);
     let y = rect.center().y;
-    let connect_label = format!("{} {}", gettext(locale, "Connect to"), connection.name);
-    let connect_text = gettext(locale, "Connect");
+    let (connect_text, connect_label) = choose_labels(connection, opening, locale);
     // 16 in from the right, 6 apart, 30 tall.
     let connect = ButtonSpec::new(&connect_text)
         .primary()
@@ -791,10 +869,7 @@
         connect.hidden_at(ui, place)
     };
     if response.clicked() {
-        actions.push(Action::Connect {
-            tab,
-            conn: connection.id.clone(),
-        });
+        actions.push(opening.choose(connection));
     }
     let more_place =
         Rect::from_min_size(pos2(place.left() - 6.0 - 30.0, y - 15.0), vec2(30.0, 30.0));
@@ -858,7 +933,7 @@
     connection: &SavedConnection,
     store: &crate::connections::SavedConnections,
     selected: Option<&ConnectionId>,
-    tab: crate::model::ConnTabId,
+    opening: Opening,
     skin: RowSkin<'_>,
     actions: &mut Vec<Action>,
 ) {
@@ -871,7 +946,7 @@
     } = skin;
     let (rect, _) =
         ui.allocate_exact_size(vec2(ui.available_width(), row_height(look)), Sense::hover());
-    let response = row_response(ui, rect, connection, tab, look, actions);
+    let response = row_response(ui, rect, connection, opening, look, actions);
     let is_selected = selected == Some(&connection.id);
     let center = rect.center().y;
     if is_selected {
@@ -901,12 +976,26 @@
     // A 13 pt name and a 12 pt line, 3 apart, centred.
     let top = center - 9.8;
     let bottom = center + 9.8;
-    widgets::paint_text(
+    let name_width = widgets::paint_text(
         ui,
         text_x,
         top,
         Text::one(look, TextRole::OGroup, &connection.name, palette.text),
     );
+    if opening.open.is_some() {
+        // Open in a tab already: the word in the accent, 10 on.
+        widgets::paint_label(
+            ui,
+            text_x + name_width + 10.0,
+            top,
+            Text::one(
+                look,
+                TextRole::OBody,
+                &gettext(locale, "open"),
+                palette.accent,
+            ),
+        );
+    }
     let spec = &connection.spec;
     let mut line = spec.summary();
     if let Some(at) = line.find(" via ") {
@@ -948,18 +1037,16 @@
         center,
         Text::one(look, small, &when, palette.dim),
     );
-    // Connect, for screen readers and the tests; the keys do it here.
-    let connect = format!("{} {}", gettext(locale, "Connect to"), connection.name);
+    // Connect (Show, for a connection that is open), for screen readers
+    // and the tests; the keys do it here.
+    let (_, choose) = choose_labels(connection, opening, locale);
     let hit = Rect::from_min_size(pos2(rect.right() - side - 1.0, rect.top()), vec2(1.0, 1.0));
-    if ButtonSpec::new(&connect)
+    if ButtonSpec::new(&choose)
         .salt(&connection.id.0)
         .hidden_at(ui, hit)
         .clicked()
     {
-        actions.push(Action::Connect {
-            tab,
-            conn: connection.id.clone(),
-        });
-    }
-}
-
+        actions.push(opening.choose(connection));
+    }
+}
+
```

- [ ] **Step 5: Enter shows, Shift+Enter opens again**

`src/ui/keys.rs`:

```diff
--- a/src/ui/keys.rs
+++ b/src/ui/keys.rs
@@ -41,8 +41,8 @@
     ("Space, Mod+Shift+R", "Toggle row panel"),
     ("Mod+C, Mod+Shift+C", "Copy cell / copy row"),
     (
-        "Arrows, Enter, Mod+E, Mod+D, Mod+Backspace",
-        "Pick, edit, duplicate or delete a connection",
+        "Arrows, Enter, Shift+Enter, Mod+E, Mod+D, Mod+Backspace",
+        "Pick, open again, edit, duplicate or delete a connection",
     ),
     ("Arrows, Home/End, Enter", "Move in the tree"),
     ("Arrows, Page Up/Down, Home/End", "Move in the grid"),
@@ -397,11 +397,20 @@
             actions.push(Action::MovePickerSelection { tab, step: -1 });
         }
         if let Some(conn) = selected {
-            if ctx.memory(|memory| memory.focused().is_none()) && pressed(Key::Enter) {
-                actions.push(Action::Connect {
-                    tab,
-                    conn: conn.clone(),
-                });
+            if ctx.memory(|memory| memory.focused().is_none()) {
+                // Shift first: egui ignores an extra Shift when matching.
+                let again = ctx.input_mut(|input| input.consume_key(Modifiers::SHIFT, Key::Enter));
+                if again || pressed(Key::Enter) {
+                    // Enter shows a connection that is open already; with
+                    // Shift it opens once more.
+                    actions.push(match app.tab_showing(&conn).filter(|_| !again) {
+                        Some(open) => Action::ActivateConnTab(open),
+                        None => Action::Connect {
+                            tab,
+                            conn: conn.clone(),
+                        },
+                    });
+                }
             }
             if command(Key::E) || (terminal && pressed(Key::E)) {
                 actions.push(Action::EditConnection(conn.clone()));
```

- [ ] **Step 6: Run the tests to verify they pass**

Run: `~/.cargo/bin/cargo fmt --all && ~/.cargo/bin/cargo test --locked -p tabletist --lib`
Expected: `test result: ok. 788 passed; 0 failed; 1 ignored`.

Run: `~/.cargo/bin/cargo clippy --locked --workspace --all-targets -- -D warnings`
Expected: no warnings.

- [ ] **Step 7: Commit point**

Stop and report. If asked to commit:

```bash
git add src/app.rs src/ui/picker.rs src/ui/keys.rs src/ui/mod.rs
git commit -S -m "Mark open connections in the list and show them when chosen

Shift+Enter, or Connect again in the row's menu, opens one once more.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

## Task 5: Screenshot scenes, and a look by eye

**Files:**
- Modify: `src/shots.rs`

- [ ] **Step 1: Put a second connection in the scenes**

`production_beside` opens Bookshop's production database beside a scene's connection. The mockup scenes (`mock`) get it, as the artboards show two chips; `shots` gets a `workspace-two` scene and a `picker-open` one:

```diff
--- a/src/shots.rs
+++ b/src/shots.rs
@@ -262,6 +262,23 @@
     tab
 }
 
+/// Bookshop's production database open beside the scene's `own`
+/// connection: the header's second chip, as the mockups show it.
+fn production_beside(harness: &mut Harness, own: ConnTabId) {
+    harness.app.apply(Action::ShowConnections);
+    let tab = harness.connect_fake();
+    let (spec, _) = ConnectSpec::from_url(
+        "postgres://app_readonly@db.example.com:5432/bookshop_production?sslmode=verify-full",
+    )
+    .unwrap();
+    let workspace = harness.app.workspace_mut(tab).unwrap();
+    workspace.name = "Bookshop".into();
+    workspace.environment = Environment::Production;
+    workspace.spec = spec;
+    workspace.driver = Driver::Postgres;
+    harness.app.apply(Action::ActivateConnTab(own));
+}
+
 fn both(name: &str, scene: impl Fn(&mut Harness)) {
     for look in crate::theme::Look::ALL {
         for (light, suffix) in [(true, "light"), (false, "dark")] {
@@ -287,6 +304,15 @@
     });
     both("workspace", |harness| {
         workspace(harness);
+    });
+    both("workspace-two", |harness| {
+        let tab = workspace(harness);
+        production_beside(harness, tab);
+    });
+    both("picker-open", |harness| {
+        harness.app.connections.upsert(saved());
+        workspace(harness);
+        harness.app.apply(Action::ShowConnections);
     });
     both("filter", |harness| {
         workspace(harness);
@@ -660,10 +686,12 @@
             crate::util::pin_now(Some(NOW));
             match self {
                 Self::MacWorkspace => {
-                    workspace(harness);
+                    let tab = workspace(harness);
+                    production_beside(harness, tab);
                 }
                 Self::OmarchyWorkspace => {
                     let tab = workspace(harness);
+                    production_beside(harness, tab);
                     let object_tab = harness.app.workspace(tab).unwrap().active_tab.unwrap();
                     let object = harness
                         .app
```

- [ ] **Step 2: Check it builds**

Run: `~/.cargo/bin/cargo clippy --locked --workspace --all-targets --features shots -- -D warnings`
Expected: no warnings.

- [ ] **Step 3: Render and compare by hand**

```bash
~/.cargo/bin/cargo test --locked --features shots --lib shots -- --ignored
```

It needs a GPU (wgpu) and writes `target/shots/*.png` (gitignored). The PNGs stay local: never commit or push them. Without a GPU, say so in the report instead.

Compare with the artboards (without the canvas to hand, with "What the design says" above):
- `mock-macos-workspace.png` and `mock-omarchy-workspace.png` against "Table view" and "Two connections open": the bar's height, the two chips (this connection's on its face, with the chevrons or `▾`; production's beside it), Read-only after them, Disconnect at the right.
- `workspace-two-macos-dark.png`: the same on the dark tint. `workspace-two-standard-light.png`: the standard look's square-ish corners.
- `picker-open-macos-light.png` and `picker-open-omarchy-dark.png`: `open` after the Fixture row's environment.

Fix what is off in `src/ui/workspace.rs` or `src/ui/picker.rs` and rerun Tasks 1 to 4's checks. No test asserts these positions.

- [ ] **Step 4: Commit point**

Stop and report. If asked to commit:

```bash
git add src/shots.rs
git commit -S -m "Open a second connection in the screenshot scenes

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

## Task 6: The spec and the README name the connection bar, and everything passes

**Files:**
- Modify: `docs/superpowers/specs/2026-09-27-tabletist-design.md`
- Modify: `README.md:31`

- [ ] **Step 1: Update the spec**

Other specs that name the tab bar (`2026-09-28-platform-looks-design.md`, `2026-09-30-sql-editor-core-design.md`) say how things were when they were written and are left as they are.

Success criterion 1 (line 24). Replace

```markdown
   open it in a connection tab. Several connection tabs can be open at once.
```

with

```markdown
   open it in a connection tab. Several can be open at once, each a chip in
   the connection bar.
```

The file list (lines 125 and 130). Delete the line

```
    ui/conn_tabs.rs          connection tab bar
```

and replace

```
    ui/workspace.rs          a connected tab: top bar, disconnected banner, body
```

with

```
    ui/workspace.rs          a connected tab: connection bar, disconnected banner, body
```

The layout (5.2). Replace the diagram's first two lines

```
┌ [● prod-db ×] [● staging ×] [○ local.sqlite ×] [+] ───────────────────────────┐  connection tabs
├ top bar: database ▾ │ via SSH host │ TLS verified │ disconnect ───────────────┤
```

with the one line

```
┌ [≡] [● prod-db ▾] [● staging] [○ local.sqlite] │ read-only │ disconnect ──────┐  connection bar
```

Section 5.3. Replace its bullets (from "One tab, one connection" to the "Reconnect" bullet) with

```markdown
- One tab, one connection (one `Session`). Several tabs can be open at once;
  the same saved connection can be opened in more than one tab.
- The window has no tab bar. The **connection bar** leads it and shows every
  open connection as a chip: its environment's colour, its name and
  environment, and its database, or how its session stands while it is not
  connected (connecting, disconnected, cancelled). The bar's own chip opens
  the database switcher; another's switches to that connection. A card
  under a chip says where the connection points and since when.
- When the chips outgrow the bar, the TLS, SSH and read-only pills give way,
  then names shorten, then the row slides to keep the bar's own chip whole.
- The picker is one tab at most. The bar's **Connections** button or
  Cmd/Ctrl+O shows it, opening it when there is none. The app starts with
  one picker tab.
- Cmd/Ctrl+1..9 switch to an open connection by its place in the bar;
  Ctrl+Tab visits every tab, the picker too.
- Closing a tab cancels its running query, closes the connection and the SSH
  tunnel, and removes the tab. **Disconnect** does the same and shows the
  picker.
- When a connection drops, the tab keeps its tree and object tabs and shows a
  "Disconnected: <reason>" banner with **Reconnect**.
```

Section 5.4, the picker's bullet. Replace

```markdown
  and `user@host/db` (or the file name for SQLite). Double-click or Enter
  connects in this tab. New, Edit, Duplicate, Delete.
```

with

```markdown
  and `user@host/db` (or the file name for SQLite). Double-click or Enter
  connects in this tab. A connection that is open already is marked
  **open**, and choosing it shows its tab; Shift+Enter connects once more.
  New, Edit, Duplicate, Delete.
```

The keyboard table (5.10). Replace the three rows

```markdown
| Cmd/Ctrl+O | New connection tab (picker) |
```

```markdown
| Cmd/Ctrl+Shift+W | Close connection tab |
| Cmd/Ctrl+1..9, Ctrl+Tab, Ctrl+Shift+Tab | Switch connection tab |
```

with

```markdown
| Cmd/Ctrl+O | Connections (the picker) |
```

```markdown
| Cmd/Ctrl+Shift+W | Close connection |
| Cmd/Ctrl+1..9, Ctrl+Tab, Ctrl+Shift+Tab | Switch connection (the digits count the open ones) |
```

Platform integration (5.11). Replace

```markdown
- macOS: the connection tabs share a unified title bar with the window
  buttons (`macos.rs`).
```

with

```markdown
- macOS: the connection bar, or the picker's header, shares a unified title
  bar with the window buttons (`macos.rs`).
```

- [ ] **Step 2: Update the README**

Replace

```markdown
  Open more than one and they share a tab bar.
```

with

```markdown
  Open more than one and each is a chip in the window's header: click one
  to switch.
```

The README's screenshots show one connection and are left as they are.

- [ ] **Step 3: Run every check**

```bash
~/.cargo/bin/cargo fmt --all --check
~/.cargo/bin/cargo clippy --locked --workspace --all-targets -- -D warnings
~/.cargo/bin/cargo clippy --locked --workspace --all-targets --features shots -- -D warnings
~/.cargo/bin/cargo test --locked --workspace --all-targets
RUSTDOCFLAGS='-D warnings' ~/.cargo/bin/cargo doc --locked --workspace --no-deps
```

Expected: all five exit 0; the library reports 788 passed; the database integration tests print "skipped" without their servers. If `fmt` reports a diff, run `~/.cargo/bin/cargo fmt --all` and rerun.

- [ ] **Step 4: Commit point**

Stop and report. If asked to commit:

```bash
git add docs/superpowers/specs/2026-09-27-tabletist-design.md README.md
git commit -S -m "Describe the connection bar's chips in the design and the README

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```
