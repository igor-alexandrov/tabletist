# Empty, loading and error states Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Every empty area says why it is empty and offers one next step, every wait longer than 300 ms shows progress and a Cancel, and every failure says what failed in plain words with the fix that matches the cause, drawn as the "0.1.0 · Empty, loading and error states" row of the design shows.

**Architecture:** A new module, `src/ui/states.rs`, draws the shared pieces (an empty state, a tinted card, a list of steps, a running card, skeleton rows, a progress line) for every look. The views that own a state (`picker.rs`, `workspace.rs`, `data_view.rs`, `structure.rs`, `sql_results.rs`) only say what to say and push the `Action`s that already exist. The model gains two timestamps (when a fetch and when a connect was sent) so a wait can show its time and stay silent for its first 300 ms. Nothing changes in the backend or in `tabletist-db`.

**Tech Stack:** Rust 2024, egui (crmne fork), fastframe-icons. Headless UI tests through `src/testing.rs` (AccessKit). Local screenshots through `--features shots`.

**Design:** the user's Design canvas, row "0.1.0 · Empty, loading and error states": the artboards "macOS – Empty and loading states", "macOS – Error states" and "Omarchy – States, values and focus" (its panes 1 to 4; panes 5 and 6 belong to the row below). It is not in the repository and is not to be added. The values this plan needs are written out below.

---

## Ground rules (read first)

- Work in `/home/igor/Work/tabletist/.claude/worktrees/empty-loading-error-states-1d793e`, branch `claude/empty-loading-error-states-1d793e`.
- `cargo` through mise is broken here: always run `~/.cargo/bin/cargo`. Never point `CARGO_TARGET_DIR` at `/tmp`.
- **Never commit or push unless the user asks.** Each task ends with a "commit point": stop there and report. If the user has asked to commit: one topic per commit, signed (`git commit -S`). If signing fails, ask before committing unsigned. Never set `SSH_AUTH_SOCK` in git commands. End messages with `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.
- No em dashes anywhere (code, comments, docs, commit messages). Use a full stop, comma, colon or parentheses.
- Tests are behavioural (the state says what it must, its buttons do their job, it stays inside its area). Never a design or pixel conformance test: do not assert the design's sizes, offsets or colours. Design material (PNGs, the artboards' HTML) is never committed.
- Test fixtures and screenshots use neutral names only (`Fixture`, `Bookshop`, `Production`). Never Safari Portal names or rows.
- Screenshots stay local (`target/shots/`, which git ignores). Never push them.
- Views draw text only through `TextRole`s and the `widgets` and `states` helpers; never name a font or a size.
- Views do not mutate application state: push an `Action`. (Copying text to the clipboard through `ui.ctx().copy_text` is not application state; `row_panel.rs` does it too.)
- Every sentence of ours goes through `gettext(locale, "...")`. The terminal look writes our words in lower case: `look.label(&gettext(locale, "..."))`. What a database or the user named (a table, a host, a filter) keeps its case.
- Comment density: short doc comments on items saying what and why, matching the surrounding code.

Task 1's code (`src/ui/states.rs`, the icons, the text role) was compiled and its tests were run while this plan was written; so was Task 2's picker change, which was also rendered and looked at in the macOS and Omarchy looks. The code in Tasks 3 to 9 is written against the same helpers but was not compiled: where it does not build as written, fix it to do what the task's text and tests say, and keep the tests.

Full check commands (used in the last task, and handy any time):

```bash
~/.cargo/bin/cargo fmt --all --check
~/.cargo/bin/cargo clippy --locked --workspace --all-targets -- -D warnings
~/.cargo/bin/cargo clippy --locked --workspace --all-targets --features shots -- -D warnings
~/.cargo/bin/cargo test --locked --workspace --all-targets
RUSTDOCFLAGS='-D warnings' ~/.cargo/bin/cargo doc --locked --workspace --no-deps
```

While working on one task, the UI tests alone are quick (under a second once built):

```bash
~/.cargo/bin/cargo test --locked --lib ui::
```

## What the design says

The macOS artboards share these pieces (13 px body text unless said otherwise):

| Piece | Design |
|---|---|
| Empty state | centred column, 8 between its parts: a 44 by 44 tile (radius 12, fill `#f0efeb`) holding a 20 px stroke icon in `#6b6a65`; a title, 15 px semibold, 6 more above it; a sentence in `#6b6a65`, at most 340 wide, line height 1.5; a row of buttons, 8 apart, 6 to 8 more above it |
| Button | 32 high, 12 at its sides, radius 8, 1 px `#dcdad4` border on white, medium text; the primary one is `#1c1c1a` with white text; the shortcut after the text, 11 px, muted |
| Quiet button | the same without a border, its text `#2c55c9` or `#6b6a65` |
| Spinner | 14 px ring, 2 px, `#dcdad4` with a `#2c55c9` head |
| Error card | 14 of padding, radius 10, fill `#fbe9e7`, 1 px `#f2c9c4` border; an 18 px icon in `#a3231b`, 12 from the text; the title semibold in `#1c1c1a`, the sentence under it (4 apart) in `#6b3a35`, line height 1.5 |
| Warning card and banner | the same with fill `#fbf0d9`, border `#f1dcae`, icon `#8a5a00` |
| Skeleton bar | 10 high, radius 5, fill `#efeee9`, at 70% |

The six empty and loading states (artboard "macOS – Empty and loading states"):

1. **First launch**, no connections saved. Tile with a database icon, "No connections yet", "Add a PostgreSQL server to start browsing. Tabletist stores passwords in the macOS Keychain.", a primary button "+ New connection ⌘N", and under it "Tip: paste a postgres:// URL anywhere in this window". The header has a disabled "Import".
2. **Connecting**, over an SSH tunnel. A 300 wide list of steps, 10 apart, each a 14 px mark, 10, then the text: a green tick "SSH tunnel `bastion.bookshop.example`", a spinner "Opening TLS session" in medium weight with "2.4 s" at the right, then two grey rings "Authenticate as `reader`" and "Load schema" in `#9a9892`. Under the list a button "Cancel esc" and "Times out after 15 s · set per connection".
3. **Empty table.** The column headers stay. Tile with a table icon, "No rows in book_wishlists", "The table exists and is empty. Columns stay visible so you can still read the structure.", a button with a reload icon "Reload ⌘R". Footer: "0 rows", "Query 1 ms".
4. **No matches**, filters exclude every row. The filter chips and the column headers stay. Tile with a funnel icon, "No rows match 2 filters", "book_covers has 13 rows. None has `kind = 'audio'` and `publisher_id = 4`.", buttons "Clear filters" and a quiet "Remove last filter". Footer: "0 of 13 rows".
5. **Query running**, shown after 300 ms. A 2 px line under the header (`#e4ebfb`) with a `#2c55c9` piece 30% of its width; the column headers in `#9a9892`; four rows of skeleton bars at 70%; in the middle a card (white, 1 px `#dcdad4`, radius 10, shadow `0 6px 20px rgba(0,0,0,0.08)`, padding 10 10 10 14, 12 between its parts): a spinner, "Running query… 4.2 s" (the time muted), a 28 high button "Cancel ⌘.". Footer: "Waiting for server", "Timeout 30 s".
6. **Loading more rows**, infinite scroll. Under the last row a 44 high line: spinner, "Loading rows 1,001–2,000…", a "Cancel" link.

The six error states (artboard "macOS – Error states"), each under the connection bar, 28 from the sides:

1. **Authentication failed.** Error card with a lock: "Password rejected for `reader`", "The server refused the password stored in the Keychain. It may have been rotated." A folded "Server message" holding `FATAL: password authentication failed for user "reader"`. Buttons: primary "Edit connection", "Retry", and at the right a quiet "Copy details".
2. **Server unreachable.** Card with a crossed-out wifi icon: "Can't reach db.staging.bookshop.example:5432", "No answer after 15 s. Check VPN, the SSH tunnel, or whether the server is running." A two line checklist ("DNS resolved to 10.20.4.17" ticked, "TCP connect timed out" crossed). Buttons: primary "Retry", "Edit connection".
3. **Certificate problem.** Card with a shield: "Certificate doesn't match the host", "Issued for `*.rds.example.net`, but you connected to `db.bookshop.example`." Issuer, expiry and SHA-256. Buttons: primary "Edit connection", "View certificate". Under them: `There is no "connect anyway". Change SSL mode or host in the connection.`
4. **Query error.** A red marker on the failing line, a wavy underline, the "Messages · 1 error" tab in red with Results, Explain and History beside it, the code as a chip, the message, "line 3, column 9", "Go to line ⌘L", a hint with "Replace".
5. **Write blocked in 0.1.0.** The first line of the statement marked amber. A warning card with a lock: "This version only reads data", "Every query runs in a read-only transaction, so PostgreSQL refused the UPDATE. Nothing changed." Under it in the code face `25006 · cannot execute UPDATE in a read-only transaction`, then "Editing arrives in a later version."
6. **Connection lost mid-session.** A 44 high amber strip (`#fbf0d9`, a `#f1dcae` line under it): an amber spinner, "Connection to Bookshop · dev lost." in semibold, "Reconnecting, attempt 2 of 5…", and at the right "Reconnect now" and a quiet "Disconnect" (28 high). The grid stays under it, its headers in `#9a9892` and its rows at 55%.

The Omarchy artboard says the same "in terminal form: plain words, the server code, one key per next step". No tiles, no cards, no centring: text from the top left, 18 in, in the 13 px mono face. A bold title, a muted sentence, then the next steps as keys (`[n] new connection`). Steps of a connect are lines (`✓ ssh bastion…`, a spinner, `· auth reader`). The write-blocked note is a block with a 3 px yellow bar down its left on a yellow tint, its title in yellow.

How the design's names map to the code:

| Design | Code |
|---|---|
| `#f0efeb` tile, `#efeee9` skeleton | `palette.surface` |
| `#6b6a65` icon | `palette.dim` |
| `#6b6a65` sentence | `palette.secondary` (the app's rule for notes a person reads, see `informational_notes_are_readable_not_dim` in `src/ui/mod.rs`) |
| `#9a9892` steps to come | `palette.faint` |
| `#2c7a4b` tick | `palette.success` |
| `#2c55c9` spinner, progress piece | `palette.accent` |
| `#e4ebfb` progress line | `palette.selection` |
| `#fbe9e7` and `#f2c9c4` | `theme::mix(palette.window, palette.danger, 0.10)` and `0.25` |
| `#fbf0d9` and `#f1dcae` | the same with `palette.warning` |
| radius 12 tile, radius 10 card | `look.radius + 4`, `look.radius + 2` (8 on macOS, 4 on Windows, 0 on Omarchy) |
| 15 px semibold title | a new `TextRole::StateTitle`; `TextRole::OGroup` in the terminal look |
| 13 px body, 12 px small, 12 px mono | `widgets::body`, `widgets::secondary`, `widgets::code` |
| buttons | `widgets::ButtonSpec`, as the picker's and the SQL editor's |
| Omarchy `[n] new connection` | `ButtonSpec::hint()` with its key as the shortcut: the bordered hint button the terminal look already uses ("+ new n", "cancel ctrl+.") |

## Decisions (where the design and the app differ)

Each of these is a product decision the plan makes so that work can start. Change them here before executing if you disagree.

1. **The copy names what the app supports.** The app opens PostgreSQL, MySQL and SQLite, so first launch says "Add a PostgreSQL, MySQL or SQLite database to start browsing." The keyring sentence follows the look, as the picker's footer does ("the macOS Keychain" on macOS, "the system keyring" elsewhere).
2. **No paste tip and no Import.** The app has no paste-to-connect and no import (a URL goes in the connection dialog's URL field). Neither is added, so the tip and the disabled button are left out.
3. **Connecting shows two steps, not four.** The backend reports a connect as one answer, so the list is "Connect to `host:port`" (with "via `bastion`" when tunnelled, "Open `file`" for SQLite) and "Load schema". The running step shows its time. Splitting the first step into tunnel, TLS and login needs `tabletist-db` to report stages: a later batch.
4. **No timeout line.** A connect gives up after a fixed 10 s per stage (the tunnel, then the database); there is no per-connection setting. "Times out after 15 s · set per connection" is left out rather than said wrongly.
5. **Cancel while connecting is Disconnect.** The button and Esc push `Action::Disconnect`: the session is closed and the tab goes back to the picker. Esc acts only while the tab has not shown its content and no dialog is open (a password prompt keeps its own Esc).
6. **A failed connect is a card under the connection bar** (it was a banner). Its title comes from the cause: "Password rejected for `user`", "Can't reach `host:port`", "TLS or certificate problem", "SSH tunnel to `host` failed", "Connection cancelled". Its sentence is the one `format::describe_error` already writes. The button that fixes the cause leads: "Edit connection" for a refused login, a TLS failure and invalid settings, "Retry" otherwise. "Copy details" copies the title and the exact error.
7. **The exact error stays on screen**, under the card, labelled "Details". The design folds it under "Server message"; the banner this replaces kept it visible on purpose ("so keyboards and screen readers get it", test `the_disconnected_banner_shows_the_exact_error_too`), and most of these messages come from the client, not the server.
8. **No DNS and TCP checklist, no certificate details, no "View certificate".** The database layer reports one error text. The TLS card keeps the design's last line: `There is no "connect anyway". Change the TLS mode or the host in the connection.`
9. **A lost connection is an amber strip with Reconnect and Disconnect.** The app does not reconnect by itself (a reconnect may need a password prompt), so there is no "attempt 2 of 5". While a reconnect the user asked for is on its way the strip shows the spinner and "Reconnecting to `name`…". "Edit connection" leaves the strip, as in the design: Disconnect leads back to the picker, where the connection is edited. What was on screen stays, at 55%.
10. **An empty table keeps its column headers** and says "No rows in `name`", "The table exists and is empty." (the design's second sentence explains the design, not the table), with "Reload" (`Action::Refresh`). The button shows no key: Cmd/Ctrl+R refreshes the table only while the grid has the keys, and the tree while the tree has them (as after opening a table from the sidebar), so the design's "⌘R" beside it would often be wrong.
11. **No matches names the filters.** "No rows match the filter" or "No rows match 2 filters", then "`name` has about 13 rows. None matches `kind = 'audio'` and `publisher_id = 4`." The count is the catalog's estimate (hence "about"), left out when unknown. "Clear filters" (was "Clear filter"), and with two or more filters "Remove last filter" (`Action::DropFilter` on the last).
12. **A running table query shows the card after 300 ms**, over skeleton rows when no page is up yet, and over the page that is up when one is (a refresh, the next page). The footer says "Waiting for server" in place of "Loading…" and keeps its small spinner and cancel only for a count. Table queries have no timeout, so the footer's "Timeout 30 s" is left out. In the terminal look the running state is a box in the middle too (square, a line round it), not a line at the top left, so that it reads over a page that is up.
13. **No infinite scroll.** The app pages (Previous and Next). A page on its way shows the same card; state 6 is otherwise not built.
14. **The SQL editor keeps what it has.** The failing line's marker and the Messages pane were built with the SQL editor. This plan adds the write-blocked card to the Results pane and an error count on the Messages tab. Explain, History, "Go to line", the hint's "Replace" and the amber marker on a refused statement's first line are not built.
15. **Windows follows the macOS drawing** with its own radii and faces, as everywhere else.
16. **Not touched:** the sidebar's states (loading schemas, no schemas, no tables or views), the picker's "No connections match", the SQL results' first-run spinner. None is on the artboards.

## File map

| File | Change |
|---|---|
| `src/typography.rs` | a `StateTitle` role (15 px semibold) |
| `src/theme.rs` | four icons: `Check` (shared), `Database`, `ShieldAlert`, `WifiOff` (own) |
| `assets/icons/database.svg`, `shield-alert.svg`, `wifi-off.svg` | new, from Lucide (ISC, as the others there) |
| `src/ui/states.rs` | **new**: the shared drawing and its tests |
| `src/ui/mod.rs` | `pub mod states;`; tests for the picker, connecting, failures, the lost strip, the data view |
| `src/ui/picker.rs` | first launch |
| `src/model.rs` | `Fetch::started` and `running_for`, `Workspace::connect_started`, `opened` and `connecting_for` |
| `src/app.rs` | `send_connect` stamps the connect |
| `src/ui/workspace.rs` | connecting, a failed connect, the lost strip, stale content |
| `src/ui/keys.rs` | Esc cancels a connect |
| `src/ui/data_view.rs` | the empty table, no matches, the running card, the error box, the footer |
| `src/ui/structure.rs` | the running card |
| `src/ui/format.rs` | `refuses_writes` |
| `src/ui/sql_results.rs` | the write-blocked card, the Messages tab's error count |
| `src/shots.rs` | scenes for the local screenshots |

---

### Task 1: The shared drawing (`states.rs`)

**Files:**
- Modify: `src/typography.rs` (the role's four places)
- Modify: `src/theme.rs` (the `icons!` list)
- Create: `assets/icons/database.svg`, `assets/icons/shield-alert.svg`, `assets/icons/wifi-off.svg`
- Create: `src/ui/states.rs`
- Modify: `src/ui/mod.rs` (the module list)

- [ ] **Step 1: Add the text role**

In `src/typography.rs`, four edits. In `pub enum TextRole`, after `Legend,`:

```rust
    /// What an empty area or a failed connection says first.
    StateTitle,
```

In `TextRole::ALL`, change the length to 39 and add the role after `Self::Legend,`:

```rust
    pub const ALL: [TextRole; 39] = [
```

```rust
        Self::Legend,
        Self::StateTitle,
        Self::OBody,
```

In `id()`, after the `Self::Legend` arm:

```rust
            Self::StateTitle => "state-title",
```

In `spec()`, after the `Self::Legend` arm:

```rust
            Self::StateTitle => style(Sans, 600, 15.0),
```

- [ ] **Step 2: Add the icons**

In `src/theme.rs`, in `fastframe_icons::icons!`, add the shared `Check` before `ChevronDown` and the three own icons in their alphabetical places:

```rust
        Check => lucide "check",
        ChevronDown => lucide "chevron-down",
```

```rust
        Code => "code",
        Database => "database",
        Funnel => "funnel",
```

```rust
        Server => "server",
        ShieldAlert => "shield-alert",
        Table => "table-2",
        WifiOff => "wifi-off",
```

Create the three files. Each is Lucide's icon of that name with the stroke set to white, as the files beside them (compare `assets/icons/server.svg`). Every file starts with this header and ends with `</svg>` and a newline:

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
```

`assets/icons/database.svg`, between the header and `</svg>`:

```svg
  <ellipse cx="12" cy="5" rx="9" ry="3" />
  <path d="M3 5V19A9 3 0 0 0 21 19V5" />
  <path d="M3 12A9 3 0 0 0 21 12" />
```

`assets/icons/shield-alert.svg`:

```svg
  <path d="M20 13c0 5-3.5 7.5-7.66 8.95a1 1 0 0 1-.67-.01C7.5 20.5 4 18 4 13V6a1 1 0 0 1 1-1c2 0 4.5-1.2 6.24-2.72a1.17 1.17 0 0 1 1.52 0C14.51 3.81 17 5 19 5a1 1 0 0 1 1 1z" />
  <path d="M12 8v4" />
  <path d="M12 16h.01" />
```

`assets/icons/wifi-off.svg`:

```svg
  <path d="M12 20h.01" />
  <path d="M8.5 16.429a5 5 0 0 1 7 0" />
  <path d="M5 12.859a10 10 0 0 1 5.17-2.69" />
  <path d="M19 12.859a10 10 0 0 0-2.007-1.523" />
  <path d="M2 8.82a15 15 0 0 1 4.177-2.643" />
  <path d="M22 8.82a15 15 0 0 0-11.288-3.764" />
  <path d="m2 2 20 20" />
```

`assets/icons/LICENSE.txt` already covers Lucide's icons: no change.

- [ ] **Step 3: Register the module and write its tests**

In `src/ui/mod.rs`, in the module list, between `sql_text` and `structure`:

```rust
pub mod sql_text;
pub mod states;
pub mod structure;
```

Create `src/ui/states.rs` holding only the module doc, the imports and the tests below (the implementation comes in Step 5):

```rust
//! Empty, loading and error states: what a view shows in place of its
//! content, drawn the same wherever it appears. A view says what to say
//! (an icon, a title, a sentence, the buttons); this module places it.

use std::time::Duration;

use egui::{Align, Color32, CornerRadius, Frame, Margin, Rect, Sense, Stroke, StrokeKind, Ui};
use egui::{pos2, vec2};

use crate::theme::{self, Icon, Look, Palette};
use crate::typography::{Text, TextRole};
use crate::ui::widgets::{self, ButtonSpec};

#[cfg(test)]
mod tests {
    use egui::accesskit::Role;

    use super::*;
    use crate::testing::{Harness, bounds, node};

    fn notice() -> Notice<'static> {
        Notice {
            icon: Icon::Table,
            title: "No rows in fixture",
            text: "The table exists and is empty.",
        }
    }

    #[test]
    fn an_empty_state_says_why_and_offers_its_buttons_in_every_look() {
        for look in Look::ALL {
            let mut harness = Harness::new();
            harness.set_look(look);
            let palette = harness.app.palette;
            let tree = harness.frame_with(|ui| {
                let rect = ui.max_rect();
                let buttons = vec![button("Reload", &look), button("Clear", &look)];
                empty(ui, rect, &notice(), buttons, &look, &palette);
            });
            for label in ["No rows in fixture", "The table exists and is empty."] {
                assert!(node(&tree, label, Role::Label).is_some(), "{}", look.name);
            }
            let reload = bounds(&tree, "Reload", Role::Button).unwrap();
            let clear = bounds(&tree, "Clear", Role::Button).unwrap();
            assert!(reload.right() <= clear.left(), "{}", look.name);
        }
    }

    #[test]
    fn an_empty_state_stays_inside_a_small_area() {
        for look in Look::ALL {
            let mut harness = Harness::new();
            harness.set_look(look);
            let palette = harness.app.palette;
            let area = Rect::from_min_size(pos2(40.0, 60.0), vec2(260.0, 90.0));
            let tree = harness.frame_with(|ui| {
                empty(ui, area, &notice(), Vec::new(), &look, &palette);
            });
            let title = bounds(&tree, "No rows in fixture", Role::Label).unwrap();
            assert!(title.top() >= area.top(), "{}", look.name);
            assert!(title.left() >= area.left(), "{}", look.name);
            assert!(title.right() <= area.right(), "{}", look.name);
        }
    }

    #[test]
    fn a_card_says_what_failed_in_every_look() {
        for look in Look::ALL {
            let mut harness = Harness::new();
            harness.set_look(look);
            let palette = harness.app.palette;
            let tree = harness.frame_with(|ui| {
                let failed = Card {
                    tone: Tone::Danger,
                    icon: Icon::Lock,
                    title: "Password rejected for reader",
                    text: "The server refused the login.",
                };
                card(ui, &failed, &look, &palette);
            });
            for label in [
                "Password rejected for reader",
                "The server refused the login.",
            ] {
                assert!(node(&tree, label, Role::Label).is_some(), "{}", look.name);
            }
        }
    }

    #[test]
    fn steps_are_named_with_what_they_act_on() {
        let look = Look::macos();
        let mut harness = Harness::new();
        harness.set_look(look);
        let palette = harness.app.palette;
        let tree = harness.frame_with(|ui| {
            let rect = ui.max_rect();
            let list = [
                Step {
                    state: StepState::Running(Some(Duration::from_millis(2400))),
                    text: "Connect to",
                    detail: "db.example.com:5432",
                },
                Step {
                    state: StepState::Waiting,
                    text: "Load schema",
                    detail: "",
                },
            ];
            steps(ui, rect, &list, &look, &palette);
        });
        assert!(node(&tree, "Connect to db.example.com:5432", Role::Label).is_some());
        assert!(node(&tree, "Load schema", Role::Label).is_some());
    }

    #[test]
    fn a_wait_names_what_runs_and_its_button_cancels() {
        for look in Look::ALL {
            let mut harness = Harness::new();
            harness.set_look(look);
            let palette = harness.app.palette;
            let tree = harness.frame_with(|ui| {
                let rect = ui.max_rect();
                let cancel = button("Cancel", &look).label("Cancel query");
                let elapsed = Duration::from_millis(4200);
                running(
                    ui,
                    rect,
                    "Running query…",
                    elapsed,
                    Some(cancel),
                    &look,
                    &palette,
                );
            });
            assert!(
                node(&tree, "Running query…", Role::Label).is_some(),
                "{}",
                look.name
            );
            assert!(
                node(&tree, "Cancel query", Role::Button).is_some(),
                "{}",
                look.name
            );
        }
    }

    #[test]
    fn a_wait_shows_once_it_has_lasted() {
        assert!(!lasted(Duration::from_millis(299)));
        assert!(lasted(DELAY));
    }

    #[test]
    fn seconds_read_to_a_tenth() {
        assert_eq!(seconds(Duration::from_millis(4234)), "4.2 s");
        assert_eq!(seconds(Duration::from_millis(300)), "0.3 s");
    }
}
```

- [ ] **Step 4: Run the tests to see them fail**

Run: `~/.cargo/bin/cargo test --locked --lib ui::states`
Expected: does not compile (`cannot find function empty`, `cannot find struct Notice`, and so on).

- [ ] **Step 5: Write the implementation**

Put this between the imports and the test module of `src/ui/states.rs` (the file's doc comment and imports are repeated here so the block is the whole top of the file):

```rust
//! Empty, loading and error states: what a view shows in place of its
//! content, drawn the same wherever it appears. A view says what to say
//! (an icon, a title, a sentence, the buttons); this module places it.

use std::time::Duration;

use egui::{Align, Color32, CornerRadius, Frame, Margin, Rect, Sense, Stroke, StrokeKind, Ui};
use egui::{pos2, vec2};

use crate::theme::{self, Icon, Look, Palette};
use crate::typography::{Text, TextRole};
use crate::ui::widgets::{self, ButtonSpec};

/// A wait shows nothing of its own before this: an answer that comes at
/// once never flashes a spinner.
pub const DELAY: Duration = Duration::from_millis(300);

/// How much of stale content shows through: what a lost connection left
/// on screen stays readable and reads as old.
pub const STALE: f32 = 0.55;

/// The widest a state's sentence runs before it wraps.
const MEASURE: f32 = 340.0;

/// From the edge of its area to a state's text.
pub const INSET: f32 = 18.0;

/// Whether a wait has gone on long enough to show.
pub fn lasted(waited: Duration) -> bool {
    waited >= DELAY
}

/// The height of a state's buttons.
pub fn button_height(look: &Look) -> f32 {
    if look.terminal { 24.0 } else { 32.0 }
}

/// A state's first line: why the area is empty, or what failed.
pub fn title_role(look: &Look) -> TextRole {
    TextRole::pick(look, TextRole::StateTitle, TextRole::OGroup)
}

/// A button of a state: a bordered one, or in the terminal look a hint
/// (the text muted, its key in the text colour).
pub fn button<'a>(text: &'a str, look: &Look) -> ButtonSpec<'a> {
    if look.terminal {
        ButtonSpec::new(text)
            .hint()
            .shortcut_role(TextRole::OBody)
            .padding(11.0)
            .gap(8.0)
    } else {
        ButtonSpec::new(text)
    }
}

/// What an empty area says: why it is empty, in a title and a sentence
/// (which may be empty).
pub struct Notice<'a> {
    pub icon: Icon,
    pub title: &'a str,
    pub text: &'a str,
}

/// `notice` in `rect` with `buttons` under it: centred under an icon tile,
/// or in the terminal look from the top left without one. Returns the
/// index of the button that was clicked.
pub fn empty(
    ui: &mut Ui,
    rect: Rect,
    notice: &Notice<'_>,
    buttons: Vec<ButtonSpec<'_>>,
    look: &Look,
    palette: &Palette,
) -> Option<usize> {
    let ctx = ui.ctx().clone();
    let room = (rect.width() - 2.0 * INSET).max(0.0);
    let measure = if look.terminal {
        room
    } else {
        room.min(MEASURE)
    };
    let align = if look.terminal {
        Align::Min
    } else {
        Align::Center
    };
    let lay = |role: TextRole, text: &str, color: Color32| {
        let mut text = Text::one(look, role, text, color).wrap(measure);
        text.job_mut().halign = align;
        text.layout(&ctx)
    };
    let title = lay(title_role(look), notice.title, palette.text);
    let text =
        (!notice.text.is_empty()).then(|| lay(widgets::body(look), notice.text, palette.secondary));
    let widths: Vec<f32> = buttons
        .iter()
        .map(|button| button.width(ui, look))
        .collect();
    let height = button_height(look);
    // What sits between the pieces: under the tile, the title and the
    // sentence, and between the buttons.
    let (tile, under_tile, under_title, under_text, between) = if look.terminal {
        (0.0, 0.0, 4.0, 8.0, 8.0)
    } else {
        (44.0, 14.0, 8.0, 14.0, 8.0)
    };
    let text_height = text
        .as_ref()
        .map_or(0.0, |text| under_title + text.height());
    let buttons_height = if buttons.is_empty() {
        0.0
    } else {
        under_text + height
    };
    let total = tile + under_tile + title.height() + text_height + buttons_height;
    // The line the pieces hang on, and where they start: the middle of the
    // area, or its top left in the terminal look. Kept under the area's
    // top when there is too little room to centre.
    let (x, mut y) = if look.terminal {
        (rect.left() + INSET, rect.top() + INSET)
    } else {
        let top = rect.center().y - total / 2.0;
        (rect.center().x, top.max(rect.top() + INSET))
    };
    if !look.terminal {
        let tile_rect = Rect::from_center_size(pos2(x, y + tile / 2.0), vec2(tile, tile));
        ui.painter().rect_filled(
            tile_rect,
            CornerRadius::same(look.radius + 4),
            palette.surface,
        );
        notice.icon.image(palette.dim, 20.0).paint_at(
            ui,
            Rect::from_center_size(tile_rect.center(), vec2(20.0, 20.0)),
        );
        y += tile + under_tile;
    }
    // A galley aligned to the centre hangs on its middle, one aligned to
    // the left on its left edge: `x` is that line either way.
    let paint = |laid: &crate::typography::Laid, said: &str, y: f32| {
        laid.paint(ui.painter(), pos2(x, y));
        let left = if look.terminal {
            x
        } else {
            x - laid.width() / 2.0
        };
        widgets::announce(ui, Rect::from_min_size(pos2(left, y), laid.size()), said);
    };
    paint(&title, notice.title, y);
    y += title.height();
    if let Some(text) = &text {
        y += under_title;
        paint(text, notice.text, y);
        y += text.height();
    }
    if buttons.is_empty() {
        return None;
    }
    y += under_text;
    let all: f32 = widths.iter().sum::<f32>() + between * (widths.len() - 1) as f32;
    let mut left = if look.terminal { x } else { x - all / 2.0 };
    let mut clicked = None;
    for (index, (button, width)) in buttons.into_iter().zip(widths).enumerate() {
        let at = Rect::from_min_size(pos2(left, y), vec2(width, height));
        if button.show_at(ui, at, look, palette).clicked() {
            clicked = Some(index);
        }
        left += width + between;
    }
    clicked
}

/// How a card reads: something failed, or something was held back.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Tone {
    Danger,
    Warning,
}

impl Tone {
    pub fn color(self, palette: &Palette) -> Color32 {
        match self {
            Self::Danger => palette.danger,
            Self::Warning => palette.warning,
        }
    }

    /// The fill of a card or a banner in this tone.
    pub fn fill(self, palette: &Palette) -> Color32 {
        theme::mix(palette.window, self.color(palette), 0.10)
    }

    /// The line round a card or under a banner in this tone.
    pub fn line(self, palette: &Palette) -> Color32 {
        theme::mix(palette.window, self.color(palette), 0.25)
    }
}

/// What failed, in plain words: a title and the sentence under it.
pub struct Card<'a> {
    pub tone: Tone,
    pub icon: Icon,
    pub title: &'a str,
    pub text: &'a str,
}

/// `card` across the width `ui` has left: a tinted box with a line round
/// it, or in the terminal look a square one with a bar down its left.
pub fn card(ui: &mut Ui, card: &Card<'_>, look: &Look, palette: &Palette) {
    let tone = card.tone.color(palette);
    let (corner, stroke) = if look.terminal {
        (CornerRadius::ZERO, Stroke::NONE)
    } else {
        (
            CornerRadius::same(look.radius + 2),
            Stroke::new(1.0, card.tone.line(palette)),
        )
    };
    let strong = TextRole::pick(look, TextRole::UiBodySemibold, TextRole::OGroup);
    // The terminal's title takes the tone: it has no icon to carry it.
    let title = if look.terminal { tone } else { palette.text };
    let shown = Frame::new()
        .fill(card.tone.fill(palette))
        .stroke(stroke)
        .corner_radius(corner)
        .inner_margin(Margin::same(14))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal_top(|ui| {
                ui.spacing_mut().item_spacing = vec2(12.0, 0.0);
                if !look.terminal {
                    let (at, _) = ui.allocate_exact_size(vec2(18.0, 18.0), Sense::hover());
                    card.icon.image(tone, 18.0).paint_at(ui, at);
                }
                let width = ui.available_width();
                ui.vertical(|ui| {
                    ui.spacing_mut().item_spacing.y = 4.0;
                    Text::one(look, strong, card.title, title)
                        .wrap(width)
                        .layout(ui.ctx())
                        .label(ui);
                    if !card.text.is_empty() {
                        Text::one(look, widgets::body(look), card.text, palette.secondary)
                            .wrap(width)
                            .layout(ui.ctx())
                            .label(ui);
                    }
                });
            });
        });
    if look.terminal {
        let rect = shown.response.rect;
        let bar = Rect::from_min_size(rect.min, vec2(3.0, rect.height()));
        ui.painter().rect_filled(bar, CornerRadius::ZERO, tone);
    }
}

/// How far a step of a wait has come.
#[derive(Clone, Copy, PartialEq)]
pub enum StepState {
    Done,
    /// Under way, and for how long when that is known.
    Running(Option<Duration>),
    Waiting,
}

/// One step of a wait: what it does, and to what (a host, a user), which
/// reads in the code face.
pub struct Step<'a> {
    pub state: StepState,
    pub text: &'a str,
    pub detail: &'a str,
}

/// The height of one step and the room under it.
fn step_pitch(look: &Look) -> f32 {
    if look.terminal { 21.0 } else { 30.0 }
}

/// The height [`steps`] takes for `count` of them.
pub fn steps_height(count: usize, look: &Look) -> f32 {
    step_pitch(look) * count as f32
}

/// "2.4 s": how long something has been going.
pub fn seconds(elapsed: Duration) -> String {
    format!("{:.1} s", elapsed.as_secs_f64())
}

/// `steps` from the top of `rect`, one per line: a tick for a step that is
/// done, a spinner and the time for the one under way, a ring for those
/// still to come.
pub fn steps(ui: &mut Ui, rect: Rect, steps: &[Step<'_>], look: &Look, palette: &Palette) {
    const MARK: f32 = 14.0;
    let pitch = step_pitch(look);
    let body = widgets::body(look);
    let strong = TextRole::pick(look, TextRole::UiBodyStrong, TextRole::OBody);
    let small = widgets::secondary(look);
    for (index, step) in steps.iter().enumerate() {
        let row = Rect::from_min_size(
            pos2(rect.left(), rect.top() + pitch * index as f32),
            vec2(rect.width(), pitch),
        );
        let center = row.top() + 10.0;
        let mark = Rect::from_center_size(pos2(row.left() + MARK / 2.0, center), vec2(MARK, MARK));
        let (role, color, detail) = match step.state {
            StepState::Done => (body, palette.text, palette.dim),
            StepState::Running(_) => (strong, palette.text, palette.dim),
            StepState::Waiting => (body, palette.faint, palette.faint),
        };
        match step.state {
            StepState::Done => Icon::Check.image(palette.success, MARK).paint_at(ui, mark),
            // The spinner asks for the frames that keep the time going.
            StepState::Running(_) => egui::Spinner::new()
                .size(MARK)
                .color(palette.accent)
                .paint_at(ui, mark),
            StepState::Waiting if look.terminal => {
                ui.painter()
                    .circle_filled(mark.center(), 1.5, palette.faint);
            }
            StepState::Waiting => {
                let ring = Stroke::new(2.0, palette.outline);
                ui.painter()
                    .circle_stroke(mark.center(), MARK / 2.0 - 1.0, ring);
            }
        }
        let mut text = Text::one(look, role, step.text, color);
        if !step.detail.is_empty() {
            text = text
                .space(role, " ")
                .add(widgets::code(look), step.detail, detail);
        }
        widgets::paint_text(ui, row.left() + MARK + 10.0, center, text);
        if let StepState::Running(Some(elapsed)) = step.state {
            let time = Text::one(look, small, &seconds(elapsed), palette.dim);
            widgets::paint_text_right(ui, row.right(), center, time);
        }
        let said = if step.detail.is_empty() {
            step.text.to_owned()
        } else {
            format!("{} {}", step.text, step.detail)
        };
        widgets::announce(ui, row, &said);
    }
}

/// A wait that has lasted: a spinner, what is running and for how long,
/// and the button that cancels it, in a box in the middle of `rect` so it
/// reads over whatever is under it (a page that a refresh will replace).
/// Returns whether the button was clicked.
pub fn running(
    ui: &mut Ui,
    rect: Rect,
    text: &str,
    elapsed: Duration,
    cancel: Option<ButtonSpec<'_>>,
    look: &Look,
    palette: &Palette,
) -> bool {
    let spinner = if look.terminal { 12.0 } else { 14.0 };
    let height = if look.terminal { 24.0 } else { 28.0 };
    let gap = 12.0;
    let role = widgets::body(look);
    let laid = Text::one(look, role, text, palette.text)
        .space(role, " ")
        .add(role, &seconds(elapsed), palette.dim)
        .layout(ui.ctx());
    let button_width = cancel.as_ref().map(|button| button.width(ui, look));
    let content = spinner + gap + laid.width() + button_width.map_or(0.0, |width| gap + width);
    // 14 before the spinner, 10 round the button.
    let card = Rect::from_center_size(rect.center(), vec2(14.0 + content + 10.0, height + 20.0));
    if look.terminal {
        // The terminal's box: square, a line round it, no shadow.
        ui.painter()
            .rect_filled(card, CornerRadius::ZERO, palette.window);
        ui.painter().rect_stroke(
            card,
            CornerRadius::ZERO,
            Stroke::new(1.0, palette.outline),
            StrokeKind::Inside,
        );
    } else {
        let corner = CornerRadius::same(look.radius + 2);
        let shadow = egui::epaint::Shadow {
            offset: [0, 6],
            blur: 20,
            spread: 0,
            color: palette.shadow.gamma_multiply(0.4),
        };
        ui.painter().add(shadow.as_shape(card, corner));
        ui.painter()
            .rect_filled(card, corner, widgets::raised_fill(palette));
        ui.painter().rect_stroke(
            card,
            corner,
            Stroke::new(widgets::hairline(ui), palette.border),
            StrokeKind::Inside,
        );
    }
    let (mut x, center) = (card.left() + 14.0, card.center().y);
    // The spinner asks for the frames that keep the time going.
    egui::Spinner::new()
        .size(spinner)
        .color(palette.accent)
        .paint_at(
            ui,
            Rect::from_center_size(pos2(x + spinner / 2.0, center), vec2(spinner, spinner)),
        );
    x += spinner + gap;
    let width = laid.paint_left(ui.painter(), x, center);
    // Named without the time, which changes every frame.
    let named = Rect::from_min_size(pos2(x, center - 8.0), vec2(width, 16.0));
    widgets::announce(ui, named, text);
    x += width + gap;
    match (cancel, button_width) {
        (Some(button), Some(width)) => {
            let at = Rect::from_min_size(pos2(x, center - height / 2.0), vec2(width, height));
            button.show_at(ui, at, look, palette).clicked()
        }
        _ => false,
    }
}

/// Rows of grey bars where rows are on their way: the shape of a grid
/// without its data. The terminal look draws none.
pub fn skeleton(ui: &Ui, rect: Rect, look: &Look, palette: &Palette) {
    if look.terminal {
        return;
    }
    const BAR: f32 = 10.0;
    const GAP: f32 = 24.0;
    // The widths of a key, a number and a word; the last column takes a
    // share of what is left, a different one on each row.
    let fixed = [32.0, 76.0, 40.0];
    let shares = [1.0, 0.7, 0.85, 0.6];
    let fill = palette.surface.gamma_multiply(0.7);
    let rows = ((rect.height() / look.grid_row) as usize).min(6);
    for row in 0..rows {
        let center = rect.top() + look.grid_row * (row as f32 + 0.5);
        // The fixed bars, then the last one in the room that is left.
        let mut x = rect.left() + 12.0;
        let rest = (rect.right() - 12.0 - x - fixed.iter().sum::<f32>() - 3.0 * GAP).max(0.0);
        for width in fixed.into_iter().chain([rest * shares[row % shares.len()]]) {
            let at = Rect::from_min_size(pos2(x, center - BAR / 2.0), vec2(width, BAR));
            ui.painter().rect_filled(at, CornerRadius::same(5), fill);
            x += width + GAP;
        }
    }
}

/// A 2 pt line across `x` under `y` with a piece of it travelling: work
/// is going on, with no telling how much is left.
pub fn progress(ui: &Ui, x: egui::Rangef, y: f32, palette: &Palette) {
    let track = Rect::from_min_max(pos2(x.min, y), pos2(x.max, y + 2.0));
    ui.painter()
        .rect_filled(track, CornerRadius::ZERO, palette.selection);
    // A piece 30% of the line long crosses it every 1.6 s.
    let time = ui.input(|input| input.time);
    let at = ((time / 1.6).fract() as f32) * 1.3 - 0.3;
    let piece = Rect::from_min_max(
        pos2(x.min + x.span() * at, y),
        pos2(x.min + x.span() * (at + 0.3), y + 2.0),
    );
    ui.painter()
        .rect_filled(piece.intersect(track), CornerRadius::ZERO, palette.accent);
    ui.ctx().request_repaint();
}
```

- [ ] **Step 6: Run the tests to see them pass**

Run: `~/.cargo/bin/cargo fmt --all && ~/.cargo/bin/cargo test --locked --lib ui::states`
Expected: 7 passed.

Run: `~/.cargo/bin/cargo clippy --locked --workspace --all-targets -- -D warnings`
Expected: no warnings (every item of the module is `pub`, so none is dead before the views use it).

- [ ] **Step 7: Commit point**

Stop and report. If the user has asked for commits:

```bash
git add src/typography.rs src/theme.rs assets/icons/database.svg assets/icons/shield-alert.svg assets/icons/wifi-off.svg src/ui/states.rs src/ui/mod.rs
git commit -S -m "Draw empty, loading and error states in one place"
```

---

### Task 2: First launch

**Files:**
- Modify: `src/ui/picker.rs` (imports, and the list's empty branch near line 376)
- Test: `src/ui/mod.rs` (tests `the_picker_shows_its_empty_state` near line 486 and `the_picker_lists_saved_connections_and_filters_them` near line 553)

- [ ] **Step 1: Write the failing test**

In `src/ui/mod.rs`, replace the test `the_picker_shows_its_empty_state` with:

```rust
    #[test]
    fn first_launch_says_why_the_list_is_empty_and_offers_a_connection() {
        use egui::accesskit::{self, Role};
        for look in crate::theme::Look::ALL {
            let mut harness = Harness::new();
            harness.set_look(look);
            let title = look.label("No connections yet");
            assert!(harness.has(&title), "{}", look.name);
            // The header's button and the empty state's: the lower one is
            // the empty state's.
            let tree = harness.settle();
            let top = |node: &accesskit::Node| node.bounds().map_or(0.0, |rect| rect.y0);
            let mut buttons: Vec<_> = tree
                .nodes
                .iter()
                .filter(|(_, node)| {
                    node.label() == Some("New connection") && node.role() == Role::Button
                })
                .collect();
            assert_eq!(buttons.len(), 2, "{}", look.name);
            buttons.sort_by(|(_, a), (_, b)| top(a).total_cmp(&top(b)));
            let lower = buttons[1].0;
            harness.frame(vec![egui::Event::AccessKitActionRequest(
                accesskit::ActionRequest {
                    target_tree: accesskit::TreeId::ROOT,
                    target_node: lower,
                    action: accesskit::Action::Click,
                    data: None,
                },
            )]);
            harness.settle();
            assert!(harness.app.dialog.is_some(), "{}", look.name);
        }
    }
```

In `the_picker_lists_saved_connections_and_filters_them`, replace the line `assert!(!harness.has("No saved connections yet"));` with (written against the look, so the test holds whichever look the harness draws):

```rust
        let title = harness.app.look.label("No connections yet");
        assert!(!harness.has(&title));
```

- [ ] **Step 2: Run the test to see it fail**

Run: `~/.cargo/bin/cargo test --locked --lib ui::tests::first_launch`
Expected: FAIL at `assert!(harness.has(&title))` (the picker still says "No saved connections yet").

- [ ] **Step 3: Draw the empty state**

In `src/ui/picker.rs`, add the import after `use crate::typography::{Text, TextRole};`:

```rust
use crate::ui::states;
```

Replace the start of the list's branch (from `if app.connections.connections.is_empty() || found.is_empty() {` down to the `} else {` before `let now = crate::util::now_secs();`) with:

```rust
    if app.connections.connections.is_empty() {
        // First launch: why the list is empty, and the one next step.
        let keyring = if look.faces == theme::Faces::Plex {
            gettext(locale, "Tabletist stores passwords in the macOS Keychain.")
        } else {
            gettext(locale, "Tabletist stores passwords in the system keyring.")
        };
        let text = format!(
            "{} {keyring}",
            gettext(
                locale,
                "Add a PostgreSQL, MySQL or SQLite database to start browsing."
            )
        );
        let (title, text) = (
            look.label(&gettext(locale, "No connections yet")),
            look.label(&text),
        );
        let notice = states::Notice {
            icon: Icon::Database,
            title: &title,
            text: &text,
        };
        let name = look.label(&gettext(locale, "New connection"));
        let add = states::button(&name, &look)
            .label("New connection")
            .salt("empty");
        let add = if look.terminal {
            add.shortcut("n")
        } else {
            add.primary()
                .icon(Icon::Plus)
                .shortcut(&command_n)
                .padding(14.0)
                .gap(8.0)
        };
        if states::empty(&mut list, body, &notice, vec![add], &look, &palette).is_some() {
            actions.push(Action::NewConnection);
        }
    } else if found.is_empty() {
        list.add_space(40.0);
        list.vertical_centered(|ui| {
            let text = gettext(locale, "No connections match");
            Text::one(&look, widgets::body(&look), &text, palette.secondary)
                .layout(ui.ctx())
                .label(ui);
        });
    } else {
```

`command_n` is the header button's shortcut text, already a local of `show`. The `.salt("empty")` keeps the button's id apart from the header's button of the same name.

- [ ] **Step 4: Run the tests to see them pass**

Run: `~/.cargo/bin/cargo fmt --all && ~/.cargo/bin/cargo test --locked --lib ui::`
Expected: all pass, among them `first_launch_says_why_the_list_is_empty_and_offers_a_connection` and `the_picker_lists_saved_connections_and_filters_them`.

- [ ] **Step 5: Commit point**

```bash
git add src/ui/picker.rs src/ui/mod.rs
git commit -S -m "Say why the picker is empty and offer a connection"
```

---

### Task 3: Time a fetch and a connect

A wait shows its time and stays silent for 300 ms, so the model must know when a request was sent.

**Files:**
- Modify: `src/model.rs` (`Fetch` near line 1047, `Workspace` near line 342 and its `new` near line 1960)
- Modify: `src/app.rs` (`send_connect` near line 1292)
- Test: `src/model.rs` (its `mod tests`)

- [ ] **Step 1: Write the failing tests**

In the `mod tests` of `src/model.rs`:

```rust
    #[test]
    fn a_fetch_times_the_request_in_flight() {
        let mut fetch: Fetch<u32> = Fetch::default();
        assert_eq!(fetch.running_for(), None);
        fetch.start(RequestId(1));
        assert!(fetch.running_for().is_some());
        assert!(fetch.finish(RequestId(1), Ok(7)));
        assert_eq!(fetch.running_for(), None, "nothing is in flight");
    }

    #[test]
    fn a_fetch_dropped_without_an_answer_is_not_timed() {
        let mut fetch: Fetch<u32> = Fetch::default();
        fetch.start(RequestId(1));
        // Dropped without an answer: nothing is in flight any more.
        fetch.pending = None;
        assert_eq!(fetch.running_for(), None);
    }

    #[test]
    fn a_workspace_opens_once_its_schemas_were_listed() {
        let mut workspace = crate::testing::workspace();
        assert!(!workspace.opened());
        assert_eq!(workspace.connecting_for(), None, "nothing was sent yet");
        workspace.connect_started = Some(std::time::Instant::now());
        assert!(workspace.connecting_for().is_some());
        workspace.status = SessionStatus::Connected;
        assert_eq!(workspace.connecting_for(), None, "the connect was answered");
        workspace.tree.schemas.value = Some(Vec::new());
        assert!(workspace.opened());
    }
```

In the `mod tests` of `src/app.rs`, beside `reconnect_opens_a_new_session_for_the_same_tab` (it uses the module's `connect` helper, which opens a tab and returns its ids once `Command::Connect` was sent):

```rust
    #[test]
    fn sending_the_connect_starts_its_clock() {
        let (mut app, _dir) = app();
        let (tab, _, _) = connect(&mut app);
        assert!(app.workspace(tab).unwrap().connect_started.is_some());
        // A reconnect is a new attempt with a clock of its own.
        let sent = app.backend.sent.len();
        app.apply(Action::Reconnect(tab));
        let restarted = app.backend.sent[sent..]
            .iter()
            .any(|command| matches!(command, Command::Connect { .. }));
        assert_eq!(
            app.workspace(tab).unwrap().connect_started.is_some(),
            restarted,
            "the clock runs only once the Connect is sent"
        );
    }
```

(`connect` sends `Command::Connect` and returns the tab's ids; it does not answer it.)

- [ ] **Step 2: Run the tests to see them fail**

Run: `~/.cargo/bin/cargo test --locked --lib model::tests::a_fetch`
Expected: does not compile (`no method named running_for`).

- [ ] **Step 3: Implement**

In `src/model.rs`, `Fetch` gains a field, and `start`, `finish` and a new method use it:

```rust
#[derive(Debug)]
pub struct Fetch<T> {
    pub value: Option<T>,
    pub pending: Option<RequestId>,
    pub error: Option<Error>,
    /// The request whose answer `value` holds.
    pub loaded: Option<RequestId>,
    /// When the pending request was sent, for the time a wait shows.
    pub started: Option<std::time::Instant>,
}
```

Add `started: None,` to `Default::default`. In `start`:

```rust
    pub fn start(&mut self, request: RequestId) {
        self.pending = Some(request);
        self.error = None;
        self.started = Some(std::time::Instant::now());
    }
```

In `finish`, after `self.pending = None;`:

```rust
        self.started = None;
```

After `is_loading`:

```rust
    /// How long the request in flight has been going.
    pub fn running_for(&self) -> Option<Duration> {
        self.pending
            .and(self.started)
            .map(|started| started.elapsed())
    }
```

(`Duration` is already imported in `model.rs`; if the compiler says otherwise, write `std::time::Duration`.)

In `Workspace`, after `pub status: SessionStatus,`:

```rust
    /// When the connect in flight was sent to the backend (after any
    /// password prompt), for the time the tab shows.
    pub connect_started: Option<std::time::Instant>,
```

In `Workspace::new`, after `status: SessionStatus::Connecting { request },`:

```rust
            connect_started: None,
```

In `impl Workspace`, beside its other small accessors:

```rust
    /// Whether the tab has shown its content: its schemas were listed, or
    /// could not be, at least once. Until then the tab shows how
    /// connecting goes.
    pub fn opened(&self) -> bool {
        self.tree.schemas.value.is_some() || self.tree.schemas.error.is_some()
    }

    /// How long the connect in flight has been going, once it was sent.
    pub fn connecting_for(&self) -> Option<Duration> {
        matches!(self.status, SessionStatus::Connecting { .. })
            .then_some(self.connect_started)
            .flatten()
            .map(|started| started.elapsed())
    }
```

In `src/app.rs`, in `send_connect`, after `workspace.secrets = secrets.clone();`:

```rust
        workspace.connect_started = Some(std::time::Instant::now());
```

A new attempt may wait at a password prompt before its Connect is sent, and must not show the time of the attempt before it. In both places that set `workspace.status = SessionStatus::Connecting { request };` for a new attempt (`reconnect` and `ensure_connecting`), add right after that line:

```rust
        workspace.connect_started = None;
```

(In `ensure_connecting` the line sits inside `if let Some(workspace) = self.workspace_mut(tab) {`: indent it to match.)

Search for struct literals that would now miss a field: `grep -rn "Fetch {" src` (none are expected: every `Fetch` is made by `Fetch::default()`).

- [ ] **Step 4: Run the tests to see them pass**

Run: `~/.cargo/bin/cargo fmt --all && ~/.cargo/bin/cargo test --locked --lib model:: && ~/.cargo/bin/cargo test --locked --lib app::`
Expected: all pass.

- [ ] **Step 5: Commit point**

```bash
git add src/model.rs src/app.rs
git commit -S -m "Remember when a fetch and a connect were sent"
```

---

### Task 4: Connecting

A tab that has not shown its content draws how the connect goes in place of the "Connecting…" line: its steps, and a Cancel that also answers Esc.

**Files:**
- Modify: `src/ui/workspace.rs` (`show` near line 47, `banner` near line 854; new `opening` and `connecting`)
- Modify: `src/ui/keys.rs` (`handle` near line 66)
- Test: `src/ui/mod.rs`

- [ ] **Step 1: Write the failing tests**

In the `mod tests` of `src/ui/mod.rs`, after `the_top_bar_disconnect_button_returns_to_the_picker`:

```rust
    #[test]
    fn a_connecting_tab_shows_its_steps_and_cancel_returns_to_the_picker() {
        for look in crate::theme::Look::ALL {
            let mut harness = Harness::new();
            harness.set_look(look);
            add_saved(&mut harness, "Production");
            harness.click("Connect to Production");
            let tab = harness.app.active_tab_id();
            // A SQLite file is opened; a server is connected to.
            let open = format!("{} Production.db", look.label("Open"));
            assert!(harness.has(&open), "{open} in {}", look.name);
            assert!(harness.has(&look.label("Load schema")), "{}", look.name);
            harness.click("Cancel connecting");
            assert!(harness.app.workspace(tab).is_none(), "{}", look.name);
        }
    }

    #[test]
    fn the_first_step_names_the_server_and_its_tunnel() {
        let mut harness = Harness::new();
        harness.set_look(crate::theme::Look::macos());
        add_saved_with_tunnel(&mut harness);
        harness.click("Connect to Prod");
        assert!(harness.has("Connect to db.example.com:5432 via bastion"));
    }

    #[test]
    fn escape_cancels_a_connect_and_does_nothing_once_the_tab_opened() {
        let mut harness = Harness::new();
        add_saved(&mut harness, "Production");
        harness.click("Connect to Production");
        let tab = harness.app.active_tab_id();
        harness.press(Key::Escape, Modifiers::NONE);
        assert!(harness.app.workspace(tab).is_none());

        let mut harness = Harness::new();
        let tab = harness.connect_fake();
        harness.press(Key::Escape, Modifiers::NONE);
        assert!(harness.app.workspace(tab).is_some());
    }
```

`add_saved_with_tunnel` is a helper of the same test module (near line 4927): it saves "Prod", `postgres://me@db.example.com/app` through the SSH host `bastion`.

- [ ] **Step 2: Run the tests to see them fail**

Run: `~/.cargo/bin/cargo test --locked --lib ui::tests::a_connecting_tab`
Expected: FAIL, `nothing labelled "Cancel connecting"` or the `open` assertion.

- [ ] **Step 3: Draw the steps**

In `src/ui/workspace.rs`, extend the imports:

```rust
use crate::ui::states;
use crate::ui::widgets::{self, ButtonSpec};
```

(`ButtonSpec` is used by Task 5; add it then if clippy complains now.)

In `show`, replace the lines from `let connected =` to `return; // still connecting; the banner shows progress` and its closing brace with:

```rust
    let opened = workspace.opened();
    // Never opened and no connect on its way: the connect failed, or its
    // prompt was closed.
    let failed = matches!(
        workspace.status,
        SessionStatus::Disconnected(_) | SessionStatus::Cancelled
    );
    if look.terminal && opened {
        super::object_tabs::show(app, ui, tab);
        status_line(app, ui, tab);
    }
    if !opened {
        if failed {
            // The banner says why, as before. (Task 5 gives it a card.)
            banner(app, ui, tab);
        } else {
            // Nothing of the database to show yet: how connecting goes.
            opening(app, ui, tab);
        }
        return;
    }
    banner(app, ui, tab);
    let Some(workspace) = app.workspace(tab) else {
        return;
    };
```

A tab that failed before it opened keeps the banner in this task, so the tests that read it (`a_disconnected_workspace_offers_reconnect`, `cancelling_the_password_prompt_says_so_and_offers_reconnect`, `escape_cancels_the_password_prompt`) stay green until Task 5 changes what they should read.

Add after `show`:

```rust
/// A tab that has not shown its content yet: how connecting goes. (Task 5
/// adds why it failed.)
fn opening(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId) {
    let fill = app.palette.window;
    egui::CentralPanel::default()
        .frame(Frame::new().fill(fill))
        .show(ui, |ui| connecting(app, ui, tab));
}

/// The steps of a connect, the one under way with its time, and the
/// button that gives up: in the middle of the tab, or in the terminal
/// look from its top left.
fn connecting(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId) {
    let (locale, palette, look) = (app.locale, app.palette, app.look);
    let Some(workspace) = app.workspace(tab) else {
        return;
    };
    let say = |text: &str| look.label(&gettext(locale, text));
    let spec = &workspace.spec;
    let sqlite = spec.driver == tabletist_db::Driver::Sqlite;
    let target = if sqlite {
        spec.summary()
    } else {
        format!("{}:{}", spec.host, spec.port)
    };
    let target = match spec.ssh.as_ref().filter(|_| !sqlite) {
        Some(ssh) => format!("{target} {} {}", say("via"), ssh.host),
        None => target,
    };
    let connected = matches!(workspace.status, SessionStatus::Connected);
    let (reach, load) = (
        say(if sqlite { "Open" } else { "Connect to" }),
        say("Load schema"),
    );
    let list = [
        states::Step {
            state: if connected {
                states::StepState::Done
            } else {
                states::StepState::Running(workspace.connecting_for())
            },
            text: &reach,
            detail: &target,
        },
        states::Step {
            state: if connected {
                states::StepState::Running(workspace.tree.schemas.running_for())
            } else {
                states::StepState::Waiting
            },
            text: &load,
            detail: "",
        },
    ];
    let body = ui.max_rect();
    let height = states::steps_height(list.len(), &look);
    let button_height = states::button_height(&look);
    let name = say("Cancel");
    // Named apart from a password prompt's Cancel, which can be open over it.
    let cancel = states::button(&name, &look)
        .label("Cancel connecting")
        .shortcut("esc");
    let width = cancel.width(ui, &look);
    let room = (body.width() - 2.0 * states::INSET).max(0.0);
    let (steps, button) = if look.terminal {
        let top = body.left_top() + vec2(states::INSET, states::INSET);
        (
            Rect::from_min_size(top, vec2(room, height)),
            Rect::from_min_size(pos2(top.x, top.y + height + 8.0), vec2(width, button_height)),
        )
    } else {
        // The design's 300 wide list with the button 14 under it.
        let block = room.min(300.0);
        let total = height + 14.0 + button_height;
        let top = (body.center().y - total / 2.0).max(body.top() + states::INSET);
        let center = body.center().x;
        (
            Rect::from_min_size(pos2(center - block / 2.0, top), vec2(block, height)),
            Rect::from_min_size(
                pos2(center - width / 2.0, top + height + 14.0),
                vec2(width, button_height),
            ),
        )
    };
    states::steps(ui, steps, &list, &look, &palette);
    if cancel.show_at(ui, button, &look, &palette).clicked() {
        app.actions.push(Action::Disconnect(tab));
    }
}
```

In `banner`, the `SessionStatus::Connecting { .. }` arm still draws "Connecting…": it is now reached only by a tab that had opened (a reconnect). Leave it; Task 6 rewrites `banner`.

- [ ] **Step 4: Let Esc cancel**

In `src/ui/keys.rs`, in `handle`, next to the other `let` lines that read the app before `ctx.input_mut` (after `let in_workspace = ...`):

```rust
    // A tab that is still connecting: Esc gives up, as its Cancel does.
    let opening = app.workspace(active).is_some_and(|workspace| {
        !workspace.opened()
            && matches!(
                workspace.status,
                crate::model::SessionStatus::Connecting { .. }
                    | crate::model::SessionStatus::Connected
            )
    });
```

Inside the `ctx.input_mut` closure, right after the `key(Modifiers::COMMAND | Modifiers::SHIFT, Key::W, Action::CloseConnTab(active));` call:

```rust
        if opening {
            key(Modifiers::NONE, Key::Escape, Action::Disconnect(active));
        }
```

`App::frame_ui` calls `keys::handle` only while no dialog is open, so a password prompt keeps its own Esc.

In `SHORTCUTS`, after the `("Mod+.", "Cancel running query")` line:

```rust
    ("Esc", "Cancel connecting"),
```

- [ ] **Step 5: Run the tests to see them pass**

Run: `~/.cargo/bin/cargo fmt --all && ~/.cargo/bin/cargo test --locked --lib ui::`
Expected: every test passes, the three new ones among them. If a test of the shortcut table counts its rows, update the count.

- [ ] **Step 6: Commit point**

```bash
git add src/ui/workspace.rs src/ui/keys.rs src/ui/mod.rs
git commit -S -m "Show the steps of a connect and let Esc cancel it"
```

---

### Task 5: A connect that failed

A tab that never opened and whose connect failed (or whose password prompt was cancelled) shows a card under the connection bar in place of the red banner.

**Files:**
- Modify: `src/ui/workspace.rs` (`opening`; new `failure_title` and `failure`; its `mod tests`)
- Test: `src/ui/mod.rs` (new tests, and three existing ones whose words change)

- [ ] **Step 1: Write the failing tests**

In the `mod tests` of `src/ui/workspace.rs`:

```rust
    #[test]
    fn a_failed_connects_title_names_what_was_tried() {
        use tabletist_db::{ConnectSpec, Error, SshStage};
        let (mut spec, _) = ConnectSpec::from_url("postgres://reader@db.example.com/app").unwrap();
        let say = |text: &str| text.to_owned();
        let title = |error: Option<&Error>, spec: &ConnectSpec| failure_title(error, spec, say).1;
        assert_eq!(title(None, &spec), "Connection cancelled");
        assert_eq!(
            title(Some(&Error::Auth("no".into())), &spec),
            "Password rejected for reader"
        );
        assert_eq!(
            title(Some(&Error::Connect("refused".into())), &spec),
            "Can't reach db.example.com:5432"
        );
        assert_eq!(
            title(Some(&Error::Timeout), &spec),
            "Can't reach db.example.com:5432"
        );
        assert_eq!(
            title(Some(&Error::Tls("bad certificate".into())), &spec),
            "TLS or certificate problem"
        );
        spec.user.clear();
        assert_eq!(title(Some(&Error::Auth("no".into())), &spec), "Login refused");
        spec.ssh = Some(tabletist_db::SshSpec {
            host: "bastion".into(),
            port: None,
            user: String::new(),
            auth: tabletist_db::SshAuth::KeyFile {
                path: "~/.ssh/id_ed25519".into(),
            },
        });
        let tunnel = Error::Ssh {
            stage: SshStage::Connect,
            message: "timed out".into(),
        };
        assert_eq!(title(Some(&tunnel), &spec), "SSH tunnel to bastion failed");
        let file = ConnectSpec::sqlite("/tmp/shop.db");
        assert_eq!(
            title(Some(&Error::Connect("unable to open".into())), &file),
            "Can't open shop.db"
        );
    }
```

In the `mod tests` of `src/ui/mod.rs`, after the tests of Task 4:

```rust
    /// A saved PostgreSQL connection whose connect fails with `error`.
    fn fail_connect(harness: &mut Harness, error: tabletist_db::Error) -> crate::model::ConnTabId {
        let (spec, _) = tabletist_db::ConnectSpec::from_url("postgres://reader@db.example.com/app")
            .expect("a valid URL");
        harness
            .app
            .connections
            .upsert(crate::connections::SavedConnection {
                id: crate::connections::ConnectionId::new(),
                name: "Production".into(),
                environment: crate::env::Environment::Production,
                read_only: None,
                password: crate::connections::PasswordMode::None,
                ssh_secret: crate::connections::PasswordMode::None,
                spec,
            });
        harness.click("Connect to Production");
        let Command::Connect {
            session, request, ..
        } = *crate::testing::last_sent(&harness.app)
        else {
            panic!("expected Connect");
        };
        harness.app.apply(crate::model::Action::Backend(
            crate::backend::Event::ConnectFailed {
                session,
                request,
                error,
            },
        ));
        harness.app.active_tab_id()
    }

    #[test]
    fn an_unreachable_server_says_which_and_retry_connects_again() {
        let mut harness = Harness::new();
        harness.set_look(crate::theme::Look::macos());
        let refused = tabletist_db::Error::Connect("Connection refused (os error 111)".into());
        let tab = fail_connect(&mut harness, refused);
        assert!(harness.has("Can't reach db.example.com:5432"));
        assert!(harness.has(
            "Could not reach the server. Check the host and port, and that the server is running."
        ));
        // The exact error stays on screen.
        assert!(harness.has("could not connect: Connection refused (os error 111)"));
        harness.click("Retry");
        assert!(matches!(
            harness.app.workspace(tab).unwrap().status,
            crate::model::SessionStatus::Connecting { .. }
        ));
    }

    #[test]
    fn a_refused_login_names_the_user_and_offers_the_connection() {
        let mut harness = Harness::new();
        harness.set_look(crate::theme::Look::macos());
        let refused =
            tabletist_db::Error::Auth("password authentication failed for user \"reader\"".into());
        fail_connect(&mut harness, refused);
        assert!(harness.has("Password rejected for reader"));
        harness.click("Edit connection");
        assert!(matches!(
            harness.app.dialog,
            Some(crate::model::Dialog::Connection(_))
        ));
    }

    #[test]
    fn a_tls_failure_says_there_is_no_way_round_it() {
        let mut harness = Harness::new();
        harness.set_look(crate::theme::Look::macos());
        let failed = tabletist_db::Error::Tls("invalid peer certificate: NotValidForName".into());
        fail_connect(&mut harness, failed);
        assert!(harness.has("TLS or certificate problem"));
        // The plain sentence under it is `describe_error`'s.
        assert!(harness.has(
            "The secure connection failed. Try another TLS mode, or check the certificate."
        ));
        assert!(harness.has(
            "There is no \"connect anyway\". Change the TLS mode or the host in the connection."
        ));
    }

    #[test]
    fn a_failed_connect_offers_its_buttons_in_every_look() {
        use egui::accesskit::Role;
        for look in crate::theme::Look::ALL {
            let mut harness = Harness::new();
            harness.set_look(look);
            fail_connect(&mut harness, tabletist_db::Error::Timeout);
            let tree = harness.settle();
            for name in ["Retry", "Edit connection", "Copy details"] {
                assert!(
                    crate::testing::node(&tree, name, Role::Button).is_some(),
                    "{name} in {}",
                    look.name
                );
            }
        }
    }
```

`Command` is imported further down the test module (`use crate::backend::Command;` near line 735); put these tests below that line.

- [ ] **Step 2: Run the tests to see them fail**

Run: `~/.cargo/bin/cargo test --locked --lib ui::workspace::tests::a_failed_connects_title`
Expected: does not compile (`cannot find function failure_title`).

- [ ] **Step 3: Implement the card**

In `src/ui/workspace.rs`, in `show`, a tab that has not opened now always goes to `opening`. Replace the `if !opened { if failed { ... } else { ... } return; }` block of Task 4, and the `let failed = ...;` above it, with:

```rust
    if !opened {
        // Nothing of the database to show yet: how connecting goes, or
        // why it failed.
        opening(app, ui, tab);
        return;
    }
```

and change `opening` to choose between the two bodies:

```rust
/// A tab that has not shown its content yet: how connecting goes, or why
/// it failed.
fn opening(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId) {
    let fill = app.palette.window;
    let failed = app.workspace(tab).is_some_and(|workspace| {
        matches!(
            workspace.status,
            SessionStatus::Disconnected(_) | SessionStatus::Cancelled
        )
    });
    egui::CentralPanel::default()
        .frame(Frame::new().fill(fill))
        .show(ui, |ui| {
            if failed {
                failure(app, ui, tab);
            } else {
                connecting(app, ui, tab);
            }
        });
}
```

Add after `connecting`:

```rust
/// What a connect that failed says first and the sign beside it; `None`
/// is a prompt the user closed. `say` writes our words as the look does.
fn failure_title(
    error: Option<&tabletist_db::Error>,
    spec: &tabletist_db::ConnectSpec,
    say: impl Fn(&str) -> String,
) -> (Icon, String) {
    use tabletist_db::{Driver, Error, SshStage};
    let Some(error) = error else {
        return (Icon::CircleAlert, say("Connection cancelled"));
    };
    match error {
        Error::Auth(_) if spec.user.is_empty() => (Icon::Lock, say("Login refused")),
        Error::Auth(_) => (
            Icon::Lock,
            format!("{} {}", say("Password rejected for"), spec.user),
        ),
        Error::Connect(_) | Error::Timeout if spec.driver == Driver::Sqlite => (
            Icon::CircleAlert,
            format!("{} {}", say("Can't open"), spec.summary()),
        ),
        Error::Connect(_) | Error::Timeout => (
            Icon::WifiOff,
            format!("{} {}:{}", say("Can't reach"), spec.host, spec.port),
        ),
        Error::Tls(_) => (Icon::ShieldAlert, say("TLS or certificate problem")),
        Error::Ssh {
            stage: SshStage::HostKeyUnknown { .. } | SshStage::HostKeyMismatch { .. },
            ..
        } => (Icon::ShieldAlert, say("SSH host key not trusted")),
        Error::Ssh { .. } => {
            let host = spec.ssh.as_ref().map_or("", |ssh| ssh.host.as_str());
            (
                Icon::WifiOff,
                format!("{} {host} {}", say("SSH tunnel to"), say("failed")),
            )
        }
        Error::InvalidSpec(_) => (
            Icon::CircleAlert,
            say("The connection's settings are not valid"),
        ),
        _ => (Icon::CircleAlert, say("Could not connect")),
    }
}

/// A connect that failed before the tab showed anything: what failed in
/// plain words, the exact error under it, and the buttons, the one that
/// fixes the cause first.
fn failure(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId) {
    use tabletist_db::Error;
    let (locale, palette, look) = (app.locale, app.palette, app.look);
    let Some(workspace) = app.workspace(tab) else {
        return;
    };
    let say = |text: &str| look.label(&gettext(locale, text));
    let error = match &workspace.status {
        SessionStatus::Disconnected(error) => Some(error),
        _ => None,
    };
    let (icon, title) = failure_title(error, &workspace.spec, say);
    let sentence = error
        .map(|error| crate::ui::format::describe_error(locale, error))
        .unwrap_or_default();
    let raw = error.map(ToString::to_string).unwrap_or_default();
    // The settings are what is wrong: changing them comes first.
    let edit_first = matches!(
        error,
        Some(Error::Auth(_) | Error::Tls(_) | Error::InvalidSpec(_))
    );
    let tls = matches!(error, Some(Error::Tls(_)));
    let conn = workspace.conn_id.clone();
    let body = ui.max_rect();
    // The design's 28 at the sides; the card is no wider than 520.
    let side = if look.terminal { states::INSET } else { 28.0 };
    let width = (body.width() - 2.0 * side).clamp(0.0, 520.0);
    let (left, top) = if look.terminal {
        (body.left() + side, body.top() + states::INSET)
    } else {
        (
            body.center().x - width / 2.0,
            body.top() + (body.height() * 0.2).max(24.0),
        )
    };
    let column = Rect::from_min_max(pos2(left, top), pos2(left + width, body.bottom()));
    let mut ui = ui.new_child(
        egui::UiBuilder::new()
            .id_salt("failure")
            .max_rect(column)
            .layout(egui::Layout::top_down(egui::Align::Min)),
    );
    ui.spacing_mut().item_spacing = vec2(8.0, 10.0);
    let card = states::Card {
        tone: states::Tone::Danger,
        icon,
        title: &title,
        text: &sentence,
    };
    states::card(&mut ui, &card, &look, &palette);
    // The exact error when it says more than the sentence: always on
    // screen, so keyboards and screen readers get it.
    if !raw.is_empty() && raw != sentence {
        Text::one(&look, widgets::secondary(&look), &say("Details"), palette.dim)
            .layout(ui.ctx())
            .label(&mut ui);
        Text::one(&look, widgets::code(&look), &raw, palette.secondary)
            .wrap(width)
            .layout(ui.ctx())
            .label(&mut ui);
    }
    let (mut retry, mut edit) = (false, false);
    let details = format!("{title}\n{raw}");
    ui.horizontal(|ui| {
        let height = states::button_height(&look);
        let (again, change, copy) = (say("Retry"), say("Edit connection"), say("Copy details"));
        // The button that fixes the cause is the primary one.
        fn lead<'a>(button: ButtonSpec<'a>, first: bool) -> ButtonSpec<'a> {
            if first { button.primary() } else { button }
        }
        let retry_button = lead(states::button(&again, &look).label("Retry"), !edit_first);
        let edit_button = lead(
            states::button(&change, &look).label("Edit connection"),
            edit_first,
        );
        if edit_first {
            edit = edit_button.show(ui, height, &look, &palette).clicked();
            retry = retry_button.show(ui, height, &look, &palette).clicked();
        } else {
            retry = retry_button.show(ui, height, &look, &palette).clicked();
            edit = edit_button.show(ui, height, &look, &palette).clicked();
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let copy = states::button(&copy, &look).label("Copy details").quiet();
            if copy.show(ui, height, &look, &palette).clicked() {
                ui.ctx().copy_text(details.clone());
            }
        });
    });
    if tls {
        let note = say(
            "There is no \"connect anyway\". Change the TLS mode or the host in the connection.",
        );
        Text::one(&look, widgets::secondary(&look), &note, palette.secondary)
            .wrap(width)
            .layout(ui.ctx())
            .label(&mut ui);
    }
    if retry {
        app.actions.push(Action::Reconnect(tab));
    }
    if edit {
        app.actions.push(Action::EditConnection(conn));
    }
}
```

Notes for whoever builds this:

- `say` is a closure over `Copy` values, so it is `Copy` itself: passing it to `failure_title` and using it afterwards is fine. If the compiler disagrees, pass `&say`.
- `describe_error`'s sentences stay as they are in every look, as today (existing tests assert them word for word). Only the new titles and captions go through `look.label`.
- The buttons' accessible names are fixed English (`"Retry"`), as the picker's "New connection" is, so tests and screen readers get one name in every look.

- [ ] **Step 4: Update the three tests whose words changed**

These tests fail now: a tab that failed before it opened shows the card, not the banner. In `src/ui/mod.rs`:

1. `a_disconnected_workspace_offers_reconnect` (near line 625): the tab is disconnected before it ever connected. Rename it `a_connect_that_was_lost_offers_retry` and replace `assert!(harness.has("Reconnect")); harness.click("Reconnect");` with `assert!(harness.has("Retry")); harness.click("Retry");`.
2. `cancelling_the_password_prompt_says_so_and_offers_reconnect` (near line 647): rename it `cancelling_the_password_prompt_says_so_and_offers_retry`; replace its last three assertions with:

```rust
        let cancelled = harness.app.look.label("Connection cancelled");
        assert!(harness.has(&cancelled));
        assert!(!harness.has("The server refused the login. Check the user and password."));
        assert!(harness.has("Retry"));
```

3. `escape_cancels_the_password_prompt` (near line 4368, ending `assert!(harness.has("Reconnect"));`): change that last line to `assert!(harness.has("Retry"));`.

Do not touch tests that use `connect_fake` and then lose the connection (`the_disconnected_banner_keeps_its_corners_inside_the_window`, `a_disconnected_tab_offers_reconnect_and_edit`, `the_disconnected_banner_shows_the_exact_error_too`): those tabs had opened and still show the banner. Task 6 deals with them.

- [ ] **Step 5: Run the tests to see them pass**

Run: `~/.cargo/bin/cargo fmt --all && ~/.cargo/bin/cargo test --locked --lib ui::`
Expected: all pass. If another test fails on "Reconnect" or "Connecting…", read it: a tab that never opened now says "Retry" and shows steps; a tab that had opened is unchanged until Task 6.

- [ ] **Step 6: Commit point**

```bash
git add src/ui/workspace.rs src/ui/mod.rs
git commit -S -m "Say what a failed connect failed on and lead with its fix"
```

---

### Task 6: A connection lost mid-session

The banner over a tab that had opened becomes the amber strip: what happened, Reconnect and Disconnect. What was on screen stays under it, dimmed.

**Files:**
- Modify: `src/ui/workspace.rs` (`banner`, and the content panel in `show`)
- Test: `src/ui/mod.rs`

- [ ] **Step 1: Write the failing tests**

In the `mod tests` of `src/ui/mod.rs`, replace `a_disconnected_tab_offers_reconnect_and_edit` (near line 6548) with:

```rust
    /// A tab with a page of `users` whose connection is then lost.
    fn lost(harness: &mut Harness) -> crate::model::ConnTabId {
        let tab = harness.connect_fake();
        harness.click("users");
        harness.answer_rows(crate::testing::page(3, false));
        let session = harness.app.workspace(tab).unwrap().session;
        harness.app.apply(crate::model::Action::Backend(
            crate::backend::Event::Disconnected {
                session,
                error: tabletist_db::Error::ConnectionLost("server closed the connection".into()),
            },
        ));
        tab
    }

    #[test]
    fn a_lost_connection_keeps_what_was_on_screen_and_offers_the_way_back() {
        for look in crate::theme::Look::ALL {
            let mut harness = Harness::new();
            harness.set_look(look);
            let tab = lost(&mut harness);
            let lead = format!(
                "{} Fixture · dev {}",
                look.label("Connection to"),
                look.label("lost.")
            );
            assert!(harness.has(&lead), "{lead} in {}", look.name);
            assert!(
                harness.has("the connection was lost: server closed the connection"),
                "{}",
                look.name
            );
            assert!(harness.has("user1@example.com"), "{}", look.name);
            harness.click("Reconnect");
            assert!(matches!(
                harness.app.workspace(tab).unwrap().status,
                crate::model::SessionStatus::Connecting { .. }
            ));
            let again = format!("{} Fixture…", look.label("Reconnecting to"));
            assert!(harness.has(&again), "{again} in {}", look.name);
            assert!(!harness.has("Reconnect"), "{}", look.name);
            assert!(harness.has("user1@example.com"), "{}", look.name);
        }
    }

    #[test]
    fn a_lost_connection_can_be_left() {
        let mut harness = Harness::new();
        let tab = lost(&mut harness);
        harness.click("Disconnect");
        assert!(harness.app.workspace(tab).is_none());
    }
```

Replace `the_disconnected_banner_keeps_its_corners_inside_the_window` (near line 206) with:

```rust
    #[test]
    fn the_lost_strip_keeps_its_buttons_inside_the_window() {
        for look in crate::theme::Look::ALL {
            let mut harness = Harness::with_size(egui::vec2(720.0, 480.0));
            harness.set_look(look);
            let tab = harness.connect_fake();
            let session = harness.app.workspace(tab).unwrap().session;
            harness.app.apply(crate::model::Action::Backend(
                crate::backend::Event::Disconnected {
                    session,
                    error: tabletist_db::Error::ConnectionLost("server went away".into()),
                },
            ));
            let tree = harness.settle();
            let button =
                crate::testing::bounds(&tree, "Reconnect", egui::accesskit::Role::Button).unwrap();
            assert!(button.left() >= 0.0, "{}", look.name);
            assert!(button.right() <= harness.size.x, "{}", look.name);
        }
    }
```

`the_disconnected_banner_shows_the_exact_error_too` (near line 7089) stays as it is and must keep passing: the strip still shows the plain sentence and the exact error for an error that is not a plain loss.

- [ ] **Step 2: Run the tests to see them fail**

Run: `~/.cargo/bin/cargo test --locked --lib ui::tests::a_lost_connection`
Expected: FAIL (the banner says "The connection was lost." and has no strip's lead; "Disconnect" is found, but it is the connection bar's).

- [ ] **Step 3: Rewrite the banner**

In `src/ui/workspace.rs`, replace the whole `banner` function with:

```rust
/// The strip over a tab whose connection was lost: what happened, the way
/// back (Reconnect) and the way out (Disconnect). What was on screen stays
/// under it. While a reconnect is on its way the strip says so.
fn banner(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId) {
    let locale = app.locale;
    let palette = app.palette;
    let look = app.look;
    let Some(workspace) = app.workspace(tab) else {
        return;
    };
    let say = |text: &str| look.label(&gettext(locale, text));
    let name = display_safe(&workspace.name).into_owned();
    // What happened, the plain sentence when it says more, the exact error.
    let (lead, sentence, raw, reconnecting) = match &workspace.status {
        SessionStatus::Connected => return,
        SessionStatus::Connecting { .. } => (
            format!("{} {name}…", say("Reconnecting to")),
            String::new(),
            String::new(),
            true,
        ),
        SessionStatus::Disconnected(error) => {
            let env = match workspace.environment {
                crate::env::Environment::None => String::new(),
                env => format!(" · {}", env.label(crate::env::Platform::Native)),
            };
            // A plain loss is what the lead says already.
            let sentence = if matches!(error, tabletist_db::Error::ConnectionLost(_)) {
                String::new()
            } else {
                crate::ui::format::describe_error(locale, error)
            };
            (
                format!("{} {name}{env} {}", say("Connection to"), say("lost.")),
                sentence,
                error.to_string(),
                false,
            )
        }
        SessionStatus::Cancelled => (
            say("Connection cancelled."),
            String::new(),
            String::new(),
            false,
        ),
    };
    let tone = states::Tone::Warning;
    let mut reconnect = false;
    let mut disconnect = false;
    let shown = Frame::new()
        .fill(tone.fill(&palette))
        .inner_margin(Margin::symmetric(12, 8))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 12.0;
                if reconnecting {
                    // The spinner asks for the frames that keep it turning.
                    ui.add(egui::Spinner::new().size(14.0).color(palette.warning));
                }
                ui.vertical(|ui| {
                    ui.spacing_mut().item_spacing.y = 2.0;
                    let strong = TextRole::pick(&look, TextRole::UiBodySemibold, TextRole::OGroup);
                    Text::one(&look, strong, &lead, palette.text)
                        .layout(ui.ctx())
                        .label(ui);
                    if !sentence.is_empty() {
                        Text::one(&look, widgets::body(&look), &sentence, palette.secondary)
                            .layout(ui.ctx())
                            .label(ui);
                    }
                    // The exact error when it says more (visible, so
                    // keyboards and screen readers get it).
                    if !raw.is_empty() && raw != sentence {
                        Text::one(&look, widgets::secondary(&look), &raw, palette.secondary)
                            .layout(ui.ctx())
                            .label(ui);
                    }
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.spacing_mut().item_spacing.x = 8.0;
                    let height = if look.terminal { 24.0 } else { 28.0 };
                    let (out, back) = (say("Disconnect"), say("Reconnect"));
                    disconnect = states::button(&out, &look)
                        .label("Disconnect")
                        .salt("lost")
                        .quiet()
                        .show(ui, height, &look, &palette)
                        .clicked();
                    if !reconnecting {
                        reconnect = states::button(&back, &look)
                            .label("Reconnect")
                            .show(ui, height, &look, &palette)
                            .clicked();
                    }
                });
            });
        });
    let strip = shown.response.rect;
    widgets::hline(ui, strip.x_range(), strip.bottom() - 0.5, tone.line(&palette));
    if reconnect {
        app.actions.push(Action::Reconnect(tab));
    }
    if disconnect {
        app.actions.push(Action::Disconnect(tab));
    }
}
```

If `Environment` has no `Platform::Native` label for a variant used here, read `src/env.rs` (`Environment::label`): it returns the lower-case names ("dev", "production") for `Platform::Native`.

- [ ] **Step 4: Dim what is stale**

In `show`, where `let active = workspace.active_tab;` is read (after the banner), add:

```rust
    // What a lost connection left on screen reads as old.
    let stale = !matches!(workspace.status, SessionStatus::Connected);
```

and at the top of the closure of the outer `egui::CentralPanel::default().frame(Frame::new().fill(app.palette.window)).show(ui, |ui| {` (the one that holds the object tabs, the editor or the object), as its first statement:

```rust
            if stale {
                ui.multiply_opacity(states::STALE);
            }
```

The sidebar and the connection bar are not dimmed: the tree stays the way to see where one was. No test asserts the dimming (it is how the content looks, not what it does): check it in the screenshots of Task 10.

- [ ] **Step 5: Run the tests to see them pass**

Run: `~/.cargo/bin/cargo fmt --all && ~/.cargo/bin/cargo test --locked --lib ui::`
Expected: all pass, among them the three new tests and `the_disconnected_banner_shows_the_exact_error_too`. If `harness.click("Disconnect")` in an older test now finds two buttons, it clicks the first (the connection bar's), which does the same.

Also update the module's doc comment at the top of `src/ui/workspace.rs`: "the disconnected banner" becomes "what connecting shows, the strip of a lost connection".

- [ ] **Step 6: Commit point**

```bash
git add src/ui/workspace.rs src/ui/mod.rs
git commit -S -m "Keep a lost connection's content on screen under an amber strip"
```

---

### Task 7: An empty table and filters that match nothing

A page without rows keeps its column headers and says under them why it is empty.

**Files:**
- Modify: `src/ui/data_view.rs` (`toolbar` near line 386, `show` near line 940; new `filter_texts` and `empty_rows`)
- Test: `src/ui/mod.rs` (the tests near lines 6690 and 6716, and new ones)

- [ ] **Step 1: Write the failing tests**

In `src/ui/mod.rs`, replace `an_empty_table_says_so` (near line 6716) with:

```rust
    /// The fixture's `users` table, open with a page of no rows.
    fn empty_users(harness: &mut Harness) -> crate::model::ConnTabId {
        let tab = harness.connect_fake();
        harness.app.apply(crate::model::Action::OpenObject {
            tab,
            object: tabletist_db::ObjectRef::new("main", "users"),
            kind: tabletist_db::ObjectKind::Table,
            pin: true,
        });
        harness.answer_rows(crate::testing::page(0, false));
        tab
    }

    #[test]
    fn an_empty_table_keeps_its_columns_and_says_so() {
        for look in crate::theme::Look::ALL {
            let mut harness = Harness::new();
            harness.set_look(look);
            empty_users(&mut harness);
            let title = format!("{} users", look.label("No rows in"));
            assert!(harness.has(&title), "{title} in {}", look.name);
            assert!(
                harness.has(&look.label("The table exists and is empty.")),
                "{}",
                look.name
            );
            // The structure stays readable: the grid's header is there.
            assert!(harness.has("email"), "{}", look.name);
        }
    }

    #[test]
    fn reload_under_an_empty_table_fetches_again_and_takes_the_pointer() {
        let mut harness = Harness::new();
        empty_users(&mut harness);
        let before = fetches(&harness);
        // With the pointer, not through AccessKit: the grid under the
        // button must not take the click.
        let tree = harness.settle();
        let button =
            crate::testing::bounds(&tree, "Reload", egui::accesskit::Role::Button).unwrap();
        click_at(&mut harness, button.center());
        assert_eq!(fetches(&harness), before + 1);
    }

    #[test]
    fn filters_that_match_nothing_are_named_and_the_last_one_can_go() {
        let mut harness = Harness::new();
        harness.set_look(crate::theme::Look::macos());
        let tab = harness.connect_fake();
        harness.click("users");
        harness.answer_rows(crate::testing::page(3, false));
        let id = harness.app.workspace(tab).unwrap().active_tab.unwrap();
        let row = |column: &str, value: &str| crate::model::FilterRow {
            column: column.into(),
            op: tabletist_db::FilterOp::Eq,
            value: value.into(),
        };
        harness
            .app
            .workspace_mut(tab)
            .unwrap()
            .object_tab_mut(id)
            .unwrap()
            .filter
            .rows = vec![row("id", "999"), row("email", "nobody")];
        harness.app.apply(crate::model::Action::ApplyFilters {
            tab,
            object_tab: id,
        });
        harness.answer_rows(crate::testing::page(0, false));
        assert!(harness.has("No rows match 2 filters"));
        // The fixture's estimate for `users`.
        assert!(harness.has(
            "users has about 1,200,000 rows. None matches id = 999 and email = nobody."
        ));
        harness.click("Remove last filter");
        assert!(matches!(
            harness.app.backend.sent.last(),
            Some(Command::FetchRows { query, .. })
                if query.filters.len() == 1 && query.filters[0].column == "id"
        ));
    }
```

`fetches` (near line 737) and `click_at` (near line 1063) are helpers of the same test module. The sentence's filter text must be what the toolbar's chips say: run the test once and, if the chip writes the operator differently (`filter_bar::op_label`), fix the expected string, not the code.

In the test that ends `assert!(harness.has("No rows match the filter")); harness.click("Clear filter");` (near line 6709), change the button's name:

```rust
        let title = harness.app.look.label("No rows match the filter");
        assert!(harness.has(&title));
        harness.click("Clear filters");
```

- [ ] **Step 2: Run the tests to see them fail**

Run: `~/.cargo/bin/cargo test --locked --lib ui::tests::an_empty_table`
Expected: FAIL (`No rows in users` is not on screen: the view says "This table is empty").

- [ ] **Step 3: Share the filters' text**

In `src/ui/data_view.rs`, add above `toolbar`:

```rust
/// The applied filters as the toolbar's chips write them, the raw WHERE
/// last.
fn filter_texts(object: &ObjectTab) -> Vec<String> {
    object
        .query
        .filters
        .iter()
        .map(|filter| {
            let op = crate::ui::filter_bar::op_label(filter.op);
            if matches!(
                filter.op,
                tabletist_db::FilterOp::IsNull | tabletist_db::FilterOp::IsNotNull
            ) {
                format!("{} {op}", filter.column)
            } else {
                format!("{} {op} {}", filter.column, filter.value)
            }
        })
        .chain(object.query.raw_where.clone())
        .collect()
}
```

and in `toolbar` replace the whole `let filters: Vec<String> = object.query.filters.iter()...collect();` statement with:

```rust
    let filters = filter_texts(object);
```

- [ ] **Step 4: Draw the grid for every page, and the state under an empty one**

Add the import `use crate::ui::states;` to `src/ui/data_view.rs`.

In `show`, the branch `} else if let Some(page) = object.page() {` now holds `if page.rows.is_empty() { ...text and Clear filter... } else { ...grid... }`. Replace that inner `if`/`else` so that the grid is drawn for every page and the state follows it:

```rust
    } else if let Some(page) = object.page() {
        // (the code that was in the inner `else`: `structure`, `columns`,
        // `ctx`, `tags`, `grid::show(...)` and the two `if let Some(...)`
        // that push SelectCell and SortBy, unchanged)
        if page.rows.is_empty() {
            // The headers stay: the columns are still worth reading.
            let under = Rect::from_min_max(
                pos2(area.left(), area.top() + grid::header_height(&look)),
                area.max,
            );
            empty_rows(ui, under, object, tab, (&look, &palette, locale), &mut actions);
        }
    } else {
```

with, before the `if let Some(error) = &object.rows.error {` line:

```rust
    let area = ui.max_rect();
```

Add after `show`:

```rust
/// What a page with no rows says under its column headers: that the table
/// is empty, or which filters leave nothing, and the way out of each.
fn empty_rows(
    ui: &mut egui::Ui,
    rect: Rect,
    object: &ObjectTab,
    tab: ConnTabId,
    (look, palette, locale): (&Look, &Palette, crate::i18n::Locale),
    actions: &mut Vec<Action>,
) {
    let say = |text: &str| look.label(&gettext(locale, text));
    let name = format::display_safe(&object.object.name);
    let object_tab = object.id;
    let filters = filter_texts(object);
    if filters.is_empty() {
        let title = format!("{} {name}", say("No rows in"));
        let text = if object.kind == tabletist_db::ObjectKind::Table {
            say("The table exists and is empty.")
        } else {
            say("It returned no rows.")
        };
        let notice = states::Notice {
            icon: Icon::Table,
            title: &title,
            text: &text,
        };
        // No key beside it: Cmd/Ctrl+R refreshes the tree while the tree
        // has the keys.
        let reload = say("Reload");
        let button = states::button(&reload, look).label("Reload");
        let button = if look.terminal {
            button
        } else {
            button.icon(Icon::RefreshCw)
        };
        if states::empty(ui, rect, &notice, vec![button], look, palette).is_some() {
            actions.push(Action::Refresh(tab));
        }
        return;
    }
    let title = if filters.len() == 1 {
        say("No rows match the filter")
    } else {
        format!("{} {} {}", say("No rows match"), filters.len(), say("filters"))
    };
    let none = format!(
        "{} {}.",
        say("None matches"),
        filters.join(&format!(" {} ", say("and")))
    );
    // The catalog's estimate of the whole table, when it has one.
    let text = match object.estimated_rows {
        Some(rows) => format!(
            "{name} {} {} {}. {none}",
            say("has about"),
            format::group_digits(rows),
            say(if rows == 1 { "row" } else { "rows" })
        ),
        None => none,
    };
    let notice = states::Notice {
        icon: Icon::Funnel,
        title: &title,
        text: &text,
    };
    let (clear, last) = (say("Clear filters"), say("Remove last filter"));
    let mut buttons = vec![states::button(&clear, look).label("Clear filters")];
    if filters.len() > 1 {
        buttons.push(
            states::button(&last, look)
                .label("Remove last filter")
                .quiet(),
        );
    }
    match states::empty(ui, rect, &notice, buttons, look, palette) {
        Some(0) => actions.push(Action::ClearFilters { tab, object_tab }),
        // `DropFilter` counts the raw WHERE last, as `filter_texts` does.
        Some(_) => actions.push(Action::DropFilter {
            tab,
            object_tab,
            index: filters.len() - 1,
        }),
        None => {}
    }
}
```

`ObjectTab::estimated_rows` is the table's estimate, not the filtered count (the footer makes the same distinction). If `Locale`'s path differs, use the type `error_box` already takes (`crate::i18n::Locale`).

- [ ] **Step 5: Run the tests to see them pass**

Run: `~/.cargo/bin/cargo fmt --all && ~/.cargo/bin/cargo test --locked --lib ui::`
Expected: all pass. If `reload_under_an_empty_table_fetches_again_and_takes_the_pointer` fails while the button is found, the grid's scroll area is taking the pointer: draw the state in a child `Ui` made after the grid (`ui.new_child(egui::UiBuilder::new().id_salt("empty").max_rect(under))`) and pass that to `empty_rows`.

- [ ] **Step 6: Commit point**

```bash
git add src/ui/data_view.rs src/ui/mod.rs
git commit -S -m "Keep the columns of an empty page and say why it is empty"
```

---

### Task 8: A query on its way, and one that failed

After 300 ms a table's fetch shows the running card with its time and a Cancel; rows that are not there yet are skeleton bars. The error box takes the card's look.

**Files:**
- Modify: `src/ui/data_view.rs` (`show`, `footer` near line 718, `error_box` near line 1132; new `cancel_keys`)
- Modify: `src/ui/structure.rs` (the final `else` of `show`)
- Test: `src/ui/mod.rs`

- [ ] **Step 1: Write the failing tests**

In `src/ui/mod.rs`, replace `a_running_query_shows_a_cancel_button` (near line 812) with:

```rust
    /// Makes the active object tab's fetch look a second old.
    fn age_fetch(harness: &mut Harness, tab: crate::model::ConnTabId) {
        let workspace = harness.app.workspace_mut(tab).unwrap();
        let id = workspace.active_tab.unwrap();
        let object = workspace.object_tab_mut(id).unwrap();
        let earlier = std::time::Instant::now().checked_sub(std::time::Duration::from_secs(1));
        object.rows.started = earlier;
        object.structure.started = earlier;
    }

    #[test]
    fn a_query_that_lasts_says_so_and_can_be_cancelled() {
        for look in crate::theme::Look::ALL {
            let mut harness = Harness::new();
            harness.set_look(look);
            let tab = harness.connect_fake();
            harness.click("users");
            age_fetch(&mut harness, tab);
            assert!(harness.has(&look.label("Running query…")), "{}", look.name);
            harness.click("Cancel query");
            assert!(
                matches!(crate::testing::last_sent(&harness.app), Command::Cancel { .. }),
                "{}",
                look.name
            );
        }
    }

    #[test]
    fn a_refresh_keeps_the_page_under_the_running_card() {
        let mut harness = Harness::new();
        let tab = with_page(&mut harness);
        // Opening from the sidebar leaves the keys with the tree, where
        // Mod+R refreshes the tree: give them to the grid.
        focus_grid(&mut harness, tab);
        harness.press(Key::R, Modifiers::COMMAND);
        age_fetch(&mut harness, tab);
        let running = harness.app.look.label("Running query…");
        assert!(harness.has(&running));
        assert!(harness.has("user1@example.com"));
    }
```

(`checked_sub` returns `None` only when the machine has been up for less than a second; then the test would not see the card. If that ever bites, sleep 350 ms instead.)

`with_page` and `focus_grid` are the module's helpers (near line 901). The test `a_failed_page_shows_the_error_and_retry_refetches` stays as it is and must keep passing.

- [ ] **Step 2: Run the tests to see them fail**

Run: `~/.cargo/bin/cargo test --locked --lib ui::tests::a_query_that_lasts`
Expected: FAIL (`Running query…` is not on screen).

- [ ] **Step 3: Show the wait in the data view**

In `src/ui/data_view.rs`, add:

```rust
/// The keys that cancel a query, as the look writes them: `⌘.`, `Ctrl+.`
/// or the terminal's `ctrl+.`.
pub fn cancel_keys(look: &Look) -> String {
    format!("{}.", look.label(look.command_key()))
}
```

In `show`, replace the final branch

```rust
    } else {
        ui.centered_and_justified(|ui| {
            ui.spinner();
        });
    }
```

with:

```rust
    } else if object.rows.running_for().is_some_and(states::lasted) {
        // Rows on their way and none to show yet: the shape of a grid.
        if !look.terminal {
            states::progress(ui, area.x_range(), area.top(), &palette);
        }
        let rows = Rect::from_min_max(pos2(area.left(), area.top() + 2.0), area.max);
        states::skeleton(ui, rows, &look, &palette);
    }
    // A fetch that has lasted, over whatever is up: a refresh and the next
    // page keep the page they replace on screen.
    if let Some(waited) = object.rows.running_for() {
        if states::lasted(waited) {
            let (text, name, keys) = (
                look.label(&gettext(locale, "Running query…")),
                look.label(&gettext(locale, "Cancel")),
                cancel_keys(&look),
            );
            let cancel = states::button(&name, &look)
                .label("Cancel query")
                .shortcut(&keys);
            if states::running(ui, area, &text, waited, Some(cancel), &look, &palette) {
                actions.push(Action::CancelQuery(tab));
            }
        } else {
            // Come back when the wait is long enough to show.
            ui.ctx().request_repaint_after(states::DELAY - waited);
        }
    }
```

- [ ] **Step 4: Quiet the footer**

In `footer`: the card now says a fetch is running and cancels it, so the footer says only that it waits, and keeps its small spinner and cancel for a count (which has no card).

Replace

```rust
                            if loading {
                                gettext(locale, "Loading…")
                            } else {
                                gettext(locale, "No rows")
                            }
```

with

```rust
                            if fetching {
                                gettext(locale, "Waiting for server")
                            } else {
                                gettext(locale, "No rows")
                            }
```

where, at the top of `footer`, the line `let loading = object.rows.is_loading() || object.structure.is_loading() || object.count.is_loading();` becomes:

```rust
    let fetching = object.rows.is_loading();
```

and in the right-to-left part replace `if loading {` (the cancel icon and `ui.spinner()`) with `if counting {`. `counting` is already a local. If `loading` is used anywhere else in `footer`, read that use and pick `fetching` or `counting` by what it shows.

- [ ] **Step 5: Restyle the error box**

Replace the body of `error_box` (keep its signature and doc comment):

```rust
pub fn error_box(
    ui: &mut egui::Ui,
    error: &tabletist_db::Error,
    look: &crate::theme::Look,
    palette: &crate::theme::Palette,
    locale: crate::i18n::Locale,
    mut retry: impl FnMut(),
) {
    let say = |text: &str| look.label(&gettext(locale, text));
    let text = error.to_string();
    ui.scope(|ui| {
        ui.spacing_mut().item_spacing = vec2(8.0, 10.0);
        let card = states::Card {
            tone: states::Tone::Danger,
            icon: Icon::CircleAlert,
            title: &text,
            text: "",
        };
        states::card(ui, &card, look, palette);
        let width = ui.available_width();
        let mut details = text.clone();
        if let tabletist_db::Error::Query {
            code, detail, hint, ..
        } = error
        {
            for (label, more) in [("Code", code), ("Detail", detail), ("Hint", hint)] {
                if let Some(more) = more {
                    let line = format!("{}: {more}", gettext(locale, label));
                    Text::one(look, widgets::body(look), &line, palette.secondary)
                        .wrap(width)
                        .layout(ui.ctx())
                        .label(ui);
                    details.push('\n');
                    details.push_str(&line);
                }
            }
        }
        ui.horizontal(|ui| {
            let height = states::button_height(look);
            let (again, copy) = (say("Retry"), say("Copy details"));
            let again = states::button(&again, look).label("Retry");
            if again.show(ui, height, look, palette).clicked() {
                retry();
            }
            let copy = states::button(&copy, look).label("Copy details").quiet();
            if copy.show(ui, height, look, palette).clicked() {
                ui.ctx().copy_text(details.clone());
            }
        });
    });
}
```

The card's title is the error's own text, as before (a query error is the server's words). The labels "Code", "Detail" and "Hint" keep their case in every look, as before.

- [ ] **Step 6: Show the wait in the structure view**

In `src/ui/structure.rs`, replace the final

```rust
    } else {
        ui.centered_and_justified(|ui| {
            ui.spinner();
        });
    }
```

with:

```rust
    } else if let Some(waited) = object.structure.running_for() {
        if super::states::lasted(waited) {
            let (text, name, keys) = (
                look.label(&gettext(locale, "Loading structure…")),
                look.label(&gettext(locale, "Cancel")),
                super::data_view::cancel_keys(&look),
            );
            let cancel = super::states::button(&name, &look)
                .label("Cancel query")
                .shortcut(&keys);
            let area = ui.max_rect();
            if super::states::running(ui, area, &text, waited, Some(cancel), &look, &palette) {
                actions.push(Action::CancelQuery(tab));
            }
        } else {
            ui.ctx()
                .request_repaint_after(super::states::DELAY - waited);
        }
    }
```

`Action::CancelQuery` cancels whatever the active tab waits for, the structure included.

- [ ] **Step 7: Run the tests to see them pass**

Run: `~/.cargo/bin/cargo fmt --all && ~/.cargo/bin/cargo test --locked --lib ui::`
Expected: all pass. Tests that clicked the footer's "Cancel query" right after opening a table (search `"Cancel query"` in `src/ui/mod.rs`) need `age_fetch` first; tests that read "Loading…" in the footer now read "Waiting for server". `informational_notes_are_readable_not_dim` clicks "Count" and reads "Counting…": it must still pass.

- [ ] **Step 8: Commit point**

```bash
git add src/ui/data_view.rs src/ui/structure.rs src/ui/mod.rs
git commit -S -m "Show a query that lasts with its time and a Cancel"
```

---

### Task 9: A write the read-only session refused

The SQL editor's Results pane says a refused write as what it is, a limit of this version, in the amber card. The Messages tab counts the errors of the last run.

**Files:**
- Modify: `src/ui/format.rs` (new `refuses_writes`, its test)
- Modify: `src/ui/sql_results.rs` (`Place`, `Head`, `head`, `header`, `pane_tabs`, `draw`, `results`; new `blocked`; its `mod tests`)

- [ ] **Step 1: Write the failing tests**

In the `mod tests` of `src/ui/format.rs`:

```rust
    #[test]
    fn a_refused_write_is_told_from_other_errors() {
        use tabletist_db::{Driver, Error};
        let coded = |code: &str| Error::Query {
            code: Some(code.into()),
            message: "no".into(),
            detail: None,
            hint: None,
        };
        let refused = Error::Refused {
            line: 1,
            what: "COMMIT".into(),
        };
        for driver in [Driver::Postgres, Driver::MySql, Driver::Sqlite] {
            assert!(refuses_writes(&refused, driver));
            assert!(!refuses_writes(&Error::Timeout, driver));
        }
        // SQLSTATE 25006: read_only_sql_transaction.
        assert!(refuses_writes(&coded("25006"), Driver::Postgres));
        assert!(refuses_writes(&coded("25006"), Driver::MySql));
        assert!(!refuses_writes(&coded("42703"), Driver::Postgres));
        // SQLITE_READONLY is 8; its extended codes keep it in the low byte.
        assert!(refuses_writes(&coded("8"), Driver::Sqlite));
        assert!(refuses_writes(&coded("1032"), Driver::Sqlite));
        assert!(!refuses_writes(&coded("1"), Driver::Sqlite));
        // A SQLSTATE whose number ends in the same byte is not SQLite's code.
        assert!(!refuses_writes(&coded("23048"), Driver::Postgres));
    }
```

In the `mod tests` of `src/ui/sql_results.rs` (it has the helpers `editor`, `run`, `show_pane`, `sql`):

```rust
    /// The server's refusal of a write in a read-only transaction.
    fn read_only_refusal() -> StatementOutcome {
        StatementOutcome::Error {
            error: Error::Query {
                code: Some("25006".into()),
                message: "cannot execute UPDATE in a read-only transaction".into(),
                detail: None,
                hint: None,
            },
            position: None,
        }
    }

    #[test]
    fn a_refused_write_reads_as_a_limit_of_this_version() {
        for look in Look::ALL {
            let (mut harness, tab) = editor(look, "UPDATE users SET email = 'x'");
            // The fixture's session is SQLite's: the code is PostgreSQL's.
            harness.app.workspace_mut(tab).unwrap().driver = tabletist_db::Driver::Postgres;
            run(&mut harness);
            harness.answer_sql(Ok(script_outcome(vec![read_only_refusal()])), None);
            show_pane(&mut harness, tab, ResultPane::Results);
            let title = look.label("This version only reads data");
            assert!(harness.has(&title), "{title} in {}", look.name);
            assert!(
                harness.has("25006 · cannot execute UPDATE in a read-only transaction"),
                "{}",
                look.name
            );
        }
    }

    #[test]
    fn another_error_keeps_its_own_words_in_the_results() {
        let look = Look::macos();
        let (mut harness, tab) = editor(look, "SELECT nope");
        run(&mut harness);
        let failed = error_outcome("no such column: nope", None);
        harness.answer_sql(Ok(script_outcome(vec![failed])), None);
        show_pane(&mut harness, tab, ResultPane::Results);
        assert!(!harness.has("This version only reads data"));
    }

    #[test]
    fn the_messages_tab_counts_the_errors_of_the_last_run() {
        let value = |harness: &mut Harness| {
            let tree = harness.settle();
            let id = node(&tree, "Messages", Role::Button).expect("the Messages tab");
            let (_, tab) = tree.nodes.iter().find(|(node, _)| *node == id).unwrap();
            tab.value().map(str::to_owned)
        };
        let (mut harness, _tab) = editor(Look::macos(), "SELECT nope");
        assert_eq!(value(&mut harness), None);
        run(&mut harness);
        let failed = error_outcome("no such column: nope", None);
        harness.answer_sql(Ok(script_outcome(vec![failed])), None);
        assert_eq!(value(&mut harness).as_deref(), Some("1"));
        run(&mut harness);
        harness.answer_sql(Ok(script_outcome(vec![rows_outcome(2)])), None);
        assert_eq!(value(&mut harness), None);
    }
```

- [ ] **Step 2: Run the tests to see them fail**

Run: `~/.cargo/bin/cargo test --locked --lib ui::format::tests::a_refused_write`
Expected: does not compile (`cannot find function refuses_writes`).

- [ ] **Step 3: Tell a refused write apart**

In `src/ui/format.rs`, after `describe_error`:

```rust
/// Whether `error` is the read-only session refusing a write: Tabletist's
/// own guard, or the server's refusal. PostgreSQL and MySQL say SQLSTATE
/// 25006; SQLite says SQLITE_READONLY (8), which its extended codes keep
/// in their low byte.
pub fn refuses_writes(error: &tabletist_db::Error, driver: tabletist_db::Driver) -> bool {
    use tabletist_db::{Driver, Error};
    match error {
        Error::Refused { .. } => true,
        Error::Query {
            code: Some(code), ..
        } => match driver {
            Driver::Sqlite => code.parse::<i32>().is_ok_and(|code| code & 0xff == 8),
            Driver::Postgres | Driver::MySql => code == "25006",
        },
        _ => false,
    }
}
```

(The MySQL driver puts the SQLSTATE in `code` for query errors: `mysql::query_error`.)

- [ ] **Step 4: Draw the card in the Results pane**

In `src/ui/sql_results.rs`, add `use crate::ui::states;` and give `Place` the driver:

```rust
struct Place<'a> {
    tab: ConnTabId,
    sql: &'a SqlTab,
    /// Timestamps in full, as the workspace's tables show them.
    full_precision: bool,
    /// Whose error codes the results read.
    driver: tabletist_db::Driver,
}
```

In `draw`, set it where `place` is built: `driver: workspace.driver,`. Wherever `Place { tab, sql, full_precision }` is destructured (in `results`), add `driver` or `..`.

Add:

```rust
/// A write the read-only session refused: said as what it is, a limit of
/// this version and not a mistake in the statement. The exact error
/// stays under the card.
fn blocked(ui: &mut Ui, rect: Rect, error: &Error, env: &Env<'_>) {
    let Env { look, palette, .. } = *env;
    let inner = rect.shrink2(vec2(16.0, 14.0));
    let mut column = ui.new_child(
        egui::UiBuilder::new()
            .id_salt("blocked")
            .max_rect(inner)
            .layout(egui::Layout::top_down(egui::Align::Min)),
    );
    column.spacing_mut().item_spacing = vec2(8.0, 10.0);
    let title = env.said(|words| words.say("This version only reads data"));
    let text = env.said(|words| {
        words.say(
            "Every query runs in a read-only transaction, so this statement was refused. \
             Nothing changed.",
        )
    });
    let card = states::Card {
        tone: states::Tone::Warning,
        icon: Icon::Lock,
        title: &title.painted,
        text: &text.painted,
    };
    states::card(&mut column, &card, look, palette);
    // The database's own words, after its code when it gave one.
    let raw = match error {
        Error::Query {
            code: Some(code), ..
        } => format!("{code} · {}", error_text(error)),
        other => error_text(other),
    };
    Text::one(look, widgets::code(look), &raw, palette.secondary)
        .wrap(inner.width())
        .layout(column.ctx())
        .label(&mut column);
    let later = env.said(|words| words.say("Editing arrives in a later version."));
    Text::one(look, widgets::secondary(look), &later.painted, palette.secondary)
        .layout(column.ctx())
        .label(&mut column);
}
```

Import `Icon` from `crate::theme` if the file does not have it yet.

In `draw`, the arm `(State::Failed(error), ResultPane::Results)` becomes:

```rust
        (State::Failed(error), ResultPane::Results) => {
            if format::refuses_writes(error, place.driver) {
                blocked(&mut body, rest, error, &env);
            } else {
                note(&body, rest, &whole(error), palette.danger, &env);
            }
        }
```

In `results`, in the branch that handles no shown rows, the `else if let Some(index) = failed {` arm gains a first case:

```rust
        } else if let Some(index) = failed {
            if let StatementOutcome::Error { error, .. } = &run.outcome.results[index].outcome
                && format::refuses_writes(error, place.driver)
            {
                blocked(ui, rect, error, env);
                return;
            }
            // (the statement's line, then what the database said: unchanged)
```

`blocked` takes `&mut Ui`; `results` has `ui: &mut Ui`.

- [ ] **Step 5: Count the errors on the Messages tab**

`Head` gains:

```rust
    /// The statements of the last run that failed (1 for a run that failed
    /// as a whole), beside "Messages".
    errors: usize,
```

and `head` fills it:

```rust
        errors: match state {
            State::Failed(_) => 1,
            State::Ran(run) => run
                .outcome
                .results
                .iter()
                .filter(|result| matches!(result.outcome, StatementOutcome::Error { .. }))
                .count(),
            State::Idle | State::Waiting => 0,
        },
```

`PaneTab` gains `count_color: egui::Color32`. `pane_tabs` takes both counts: its parameter `count: Option<usize>` becomes `(count, errors): (Option<usize>, usize)`, the call in `header` becomes `pane_tabs(ui, (left, center), (head.count, head.errors), place, env, actions)`, and its table becomes:

```rust
    let tabs = [
        (ResultPane::Results, "Results", count, palette.dim),
        (
            ResultPane::Messages,
            "Messages",
            (errors > 0).then_some(errors),
            palette.danger,
        ),
    ]
    .map(|(pane, name, count, count_color)| {
```

with `count_color` stored in the `PaneTab`, and where the count is painted:

```rust
        if let Some(count) = &tab.count {
            text = text.space(plain, " ").add(plain, count, tab.count_color);
        }
```

The count is the tab's value for screen readers, as the Results count is (`info.current_text_value`): nothing more to do.

- [ ] **Step 6: Run the tests to see them pass**

Run: `~/.cargo/bin/cargo fmt --all && ~/.cargo/bin/cargo test --locked --lib ui::`
Expected: all pass. `a_run_that_failed_as_a_whole_shows_its_own_text_and_no_older_rows` covers `Error::Refused` in the Results pane and asserts the error's own text is on screen: the card's raw line is that text, so it keeps passing. A test that asserts the colour of that text (danger) for `Refused` now sees `palette.secondary`: update it, the card carries the tone.

- [ ] **Step 7: Commit point**

```bash
git add src/ui/format.rs src/ui/sql_results.rs
git commit -S -m "Say a refused write as a limit of this version"
```

---

### Task 10: Screenshots, a look by eye, and the checks

**Files:**
- Modify: `src/shots.rs`

- [ ] **Step 1: Add the scenes**

In `src/shots.rs`, in `shots()`, before `both("picker", ...)`:

```rust
    both("state-first-launch", |_| {});
```

After the `state-error` scene, add (the helpers `saved`, `workspace`, `page`, `sql_script` and `run_sql` are in this file; "Bookshop" is the neutral demo data):

```rust
    // A connect on its way, 2.4 s in.
    both("state-connecting", |harness| {
        harness.app.connections.upsert(saved());
        harness.click("Connect to Bookshop");
        let tab = harness.app.active_tab_id();
        harness.app.workspace_mut(tab).unwrap().connect_started =
            std::time::Instant::now().checked_sub(Duration::from_millis(2400));
    });
    // The three ways a connect fails that have a title of their own.
    for (name, error) in [
        (
            "state-connect-auth",
            tabletist_db::Error::Auth("password authentication failed for user \"demo\"".into()),
        ),
        (
            "state-connect-unreachable",
            tabletist_db::Error::Connect("Connection refused (os error 111)".into()),
        ),
        (
            "state-connect-tls",
            tabletist_db::Error::Tls("invalid peer certificate: NotValidForName".into()),
        ),
    ] {
        both(name, |harness| {
            harness.app.connections.upsert(saved());
            harness.click("Connect to Bookshop");
            let (session, request) = match harness.app.backend.sent.last() {
                Some(crate::backend::Command::Connect {
                    session, request, ..
                }) => (*session, *request),
                other => panic!("{other:?}"),
            };
            harness
                .app
                .apply(Action::Backend(crate::backend::Event::ConnectFailed {
                    session,
                    request,
                    error: error.clone(),
                }));
        });
    }
    // A table with no rows and no filter.
    both("state-empty-table", |harness| {
        let tab = workspace(harness);
        harness.app.apply(Action::Refresh(tab));
        let mut empty = page();
        empty.rows.clear();
        harness.answer_rows(empty);
    });
    // A table whose rows are on their way, 4.2 s in.
    both("state-loading", |harness| {
        let tab = workspace(harness);
        harness.app.apply(Action::OpenObject {
            tab,
            object: ObjectRef::new("public", "book_reviews"),
            kind: ObjectKind::Table,
            pin: true,
        });
        let workspace = harness.app.workspace_mut(tab).unwrap();
        let id = workspace.active_tab.unwrap();
        workspace.object_tab_mut(id).unwrap().rows.started =
            std::time::Instant::now().checked_sub(Duration::from_millis(4200));
    });
    // A write the read-only session refused.
    both("sql-blocked", |harness| {
        let tab = sql_script(harness, "UPDATE book_images\n   SET kind = 'ebook'\n WHERE id = 2;");
        run_sql(harness, tab, true);
        let refused = tabletist_db::StatementOutcome::Error {
            error: tabletist_db::Error::Query {
                code: Some("25006".into()),
                message: "cannot execute UPDATE in a read-only transaction".into(),
                detail: None,
                hint: None,
            },
            position: None,
        };
        harness.answer_sql(Ok(crate::testing::script_outcome(vec![refused])), None);
        let sql_tab = harness.app.workspace(tab).unwrap().active_tab.unwrap();
        harness.app.apply(Action::SetResultPane {
            tab,
            sql_tab,
            pane: crate::model::ResultPane::Results,
        });
    });
```

If the Auth scene opens a password prompt (it does when the saved connection asks for a password; `saved()` uses `PasswordMode::None`, so it should not), close it with `harness.press(egui::Key::Escape, egui::Modifiers::NONE)` only if you want the card of a cancelled prompt instead: otherwise leave the scene as it is.

The scenes already there cover the rest: `state-disconnected` (the amber strip over the dimmed grid), `state-empty` (a filter that matches nothing), `state-error` (the error box), `sql-refused`, `sql-error`. Give `state-empty` a second filter so its "Remove last filter" shows: in that scene's `object.filter.rows`, add a second `FilterRow` (`column: "book_id".into(), op: tabletist_db::FilterOp::Eq, value: "4".into()`).

- [ ] **Step 2: Render and look**

```bash
~/.cargo/bin/cargo test --locked --features shots --lib shots::shots -- --ignored
```

Expected: `test shots::shots ... ok`; PNGs in `target/shots/` (ignored by git, never pushed).

Open, for each of the three looks, light and dark: `state-first-launch`, `state-connecting`, `state-connect-auth`, `state-connect-unreachable`, `state-connect-tls`, `state-disconnected`, `state-empty-table`, `state-empty`, `state-loading`, `state-error`, `sql-blocked`, `sql-refused`, `sql-error`. Compare each with the artboard by eye, against "What the design says" above. Check in particular:

- nothing is cut off or overlaps at 1000 by 650, and the texts wrap inside their areas;
- the icons are drawn (a red square means a missing or malformed SVG);
- the tones read in the dark palette too (the card's fill is the tone mixed into the window colour);
- `state-disconnected`: the grid under the strip is dimmed, the sidebar and the bar are not;
- `state-loading`: the progress line sits under the toolbar, the skeleton fills from there, the card is in the middle;
- the terminal look has no tiles, no rounded cards, and lower-case words of ours.

Fix what is off in `src/ui/states.rs` or the view, rerun the tests, render again. Design conformance is checked here, by eye, and nowhere in the tests.

- [ ] **Step 3: Run every check**

```bash
~/.cargo/bin/cargo fmt --all --check
~/.cargo/bin/cargo clippy --locked --workspace --all-targets -- -D warnings
~/.cargo/bin/cargo clippy --locked --workspace --all-targets --features shots -- -D warnings
~/.cargo/bin/cargo test --locked --workspace --all-targets
RUSTDOCFLAGS='-D warnings' ~/.cargo/bin/cargo doc --locked --workspace --no-deps
```

Expected: each ends without errors or warnings; the database integration tests print "skipped" without their servers and pass.

- [ ] **Step 4: Try it for real**

```bash
~/.cargo/bin/cargo run --locked
```

By hand: with no saved connections the picker shows the first-launch state and both New connection buttons open the dialog. Save a PostgreSQL connection to a port nothing listens on (`localhost:1`), connect: the steps show, then the "Can't reach localhost:1" card; Retry tries again, Esc during the steps returns to the picker, Copy details puts the text on the clipboard. Open a SQLite file with an empty table: the headers and "No rows in" show, Reload works with the pointer. Report what was run and on which platform; say plainly what was only compiled.

- [ ] **Step 5: Commit point**

```bash
git add src/shots.rs
git commit -S -m "Stage the empty, loading and error states for screenshots"
```

---

## What is left for later

Written down so nobody takes their absence for an oversight (see Decisions):

- Connect stages (tunnel, TLS, login) reported by `tabletist-db`, and a per-connection timeout.
- Automatic reconnecting with attempts.
- The unreachable card's DNS and TCP checklist, the certificate's details and "View certificate".
- Paste a URL to connect, and Import.
- Infinite scroll.
- The SQL editor's Explain and History tabs, "Go to line", the hint's "Replace", and the amber marker on the first line of a refused write.
