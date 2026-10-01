# Connection Dialog Design Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Redraw the new/edit connection dialog as the two "Edit connection" artboards show it (macOS and Omarchy), without losing anything the dialog does today.

**Architecture:** `src/ui/connect_dialog.rs` is rewritten around a small `Skin` (look, palette, the chosen environment's colours).
- macOS and the standard look draw a 700 pt sheet: a stripe in the environment's colour, a header with Parameters and URL tabs, labelled fields in "Server" and "Security" groups, a read-only note, and a footer with the buttons.
- The terminal look (Omarchy) draws a 780 pt two-column form: a tinted title line, label and field per row under headings, and its keys in the footer.
- Both share one set of painters in the same file: `radio_group` (segmented choices), `row` (grid columns), `input`, `field_with_button`, `messages`, `paint_status`.
- The form model gains what the design needs and nothing more: a URL mode, the time a passed Test took, and a file picker for the CA certificate.

**Tech Stack:** Rust 2024, egui/eframe 0.36 (crmne fork), fastframe v0.1.7, AccessKit headless tests (`src/testing.rs`), egui_kittest + wgpu screenshots for comparing by eye.

**Spec:** the artboards "Edit connection, macOS" (1440 x 900) and "Edit connection, Omarchy" (960 x 1040) on the user's Design canvas. The canvas is a private Claude Artifact and is never committed. What this plan needs from it is written down in "The design" below.

## Global Constraints

- Follow `AGENTS.md`. Views draw text only through `TextRole`s; a view never names a font, a family or a size.
- Views push `Action`s; `App::apply` applies them. The dialog may change the form's own fields directly, as it does today (text, toggles, choices). Anything with an effect outside the form goes through an `Action`.
- Never log passwords, passphrases or URLs with secrets.
- Never use em dashes in code, comments, docs or commit messages.
- No design conformance tests, of any kind: no pixel snapshots, no metric or colour assertions made to prove the design is matched, no role-versus-spec copies. Tests check that the dialog works. The design is compared by eye in Task 9, with screenshots that stay in `target/shots/` (gitignored). No design material is committed.
- Screenshots and fixtures use the neutral Bookshop demo data only.
- `unsafe_code = "forbid"`; clippy `-D warnings`; no new `allow`.
- Checks before every commit:
  - `~/.cargo/bin/cargo fmt --all --check`
  - `~/.cargo/bin/cargo clippy --locked --workspace --all-targets -- -D warnings`
  - `~/.cargo/bin/cargo test --locked --workspace --all-targets`
  - `RUSTDOCFLAGS='-D warnings' ~/.cargo/bin/cargo doc --locked --workspace --no-deps`
- Use `~/.cargo/bin/cargo`: the `cargo` on the path is a mise shim that fails in this checkout.
- Commit messages: an imperative sentence as the subject, a short body, ending with `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`. Do not set `SSH_AUTH_SOCK` in git commands.
- Work on the branch `claude/connection-modal-design-752062` in this worktree.

## The design

Both artboards show the same connection being edited: production, behind an SSH bastion, with a Test that passed.

### macOS (also the standard look)

| Part | As drawn |
|---|---|
| Sheet | 700 wide, corners 14, no border, shadow `0 24 64` in the palette's shadow, on a dimmed window |
| Stripe | 5 pt along the top in the environment's colour, following the corners |
| Header | padding 16/20/12: the title (17 semibold) over "Name · environment" (13, dim); at the right a segmented "Parameters / URL" (segments 26 tall) and a 30 pt Close button, 12 apart |
| Body | padding 4/20/18, blocks 18 apart |
| Identity | two columns 1.4fr and 1fr, 12 apart: labels 12 medium in the secondary colour, 5 above 34 pt fields |
| Environment | label, 6, then a segmented control (segments 28 tall, 12 of padding, track padding 3, track radius 8, segment radius 6); the chosen segment is raised white with a soft shadow, its text medium in the environment's text colour after an 8 pt dot in the environment's colour; a 12 pt hint follows, 12 on |
| Groups | "Server" and "Security": a hairline box, corners 10, padding 14/14/12, its heading (12 semibold) set into the top edge; fields 32 tall, labels 12 regular, rows 10 apart, columns 12 apart |
| Server | Host beside Port (96 wide); Database; User; Password beside a "Keychain" check box |
| Security | "SSL mode" beside "CA certificate" (a path and a "Choose…" button, 6 apart); a "Connect through SSH tunnel" check box; the tunnel's fields 22 in, 10 apart |
| Safety | a box tinted with the environment's colour, corners 10, padding 12/14: a check box, "Open read-only" (13 medium) over a 12 pt explanation |
| Footer | padding 14/20 on the panel colour above a hairline: "Delete" in the danger colour at the left; at the right the Test's status (12 pt, success colour, a check mark), then Test, Cancel and Save, 32 tall, 10 apart, Save filled in ink |
| Fields | white, a hairline border in the border colour, corners 7; values that go to the server in monospace 12.5 |

### Omarchy (the terminal look)

| Part | As drawn |
|---|---|
| Dialog | 780 wide, corners 6, a 2 pt border in the environment's colour, the window colour inside, over a scrim |
| Header | 40 tall, the panel colour mixed 16% with the environment's colour, a 1 pt rule below: "edit connection" (14 bold), 12, "Name · environment" (muted); at the right "u paste url" (12 pt, the key in the text colour) |
| Body | padding 16/18: a 150 pt label column (muted), 14, then the field; rows 8 apart; fields 30 tall on the panel colour with a 1 pt border, corners 3 |
| Environment | buttons 26 tall, 10 of padding, 6 apart, each bordered and written in its environment's colour; the chosen one filled with that colour, dark bold text, a dot before it |
| Headings | "server", "security", "safety": bold over a 1 pt rule, 8 above and below |
| Rows | name, environment; host : port (port 80 wide), database, user, password with a "keyring" check; ssl mode as words (the chosen one boxed in the accent), ca cert, ssh tunnel with an `[x]`; read-only with an `[x]` in the environment's colour |
| Footer | 40 tall on the panel colour above a 1 pt rule: the Test's status at the left; at the right, 16 apart, "tab next", "ctrl+t test", "ctrl+s save" (its key in the accent), "esc cancel" |

Everything is lower case in the terminal look (`Look::label`).

## Where the design and the app differ

The artboards show one PostgreSQL connection. The app also has SQLite and MySQL, three ways to log in over SSH, and five TLS modes; and it has no groups and no writable sessions yet. Each difference is settled here. Review these before executing: they are product decisions.

| The design | The app | What this plan does |
|---|---|---|
| A "Group" list beside Name | No groups: the picker groups by shared name, then Local and Remote | No Group field. The column beside Name holds "Type": SQLite, PostgreSQL, MySQL as a segmented control (the design shows no driver choice). Adding real groups is its own change. |
| No driver choice | Three drivers | "Type", as above. SQLite replaces the Server and Security groups with one "Database" group holding the file and "Choose…". |
| Environment: Local, Dev, Staging, Production, None | Also has Test | Local, Dev, Test, Staging, Production, None, in that order. `Environment::name()` says "Dev", as the design does. |
| "Sets the connection color and safety defaults" | No safety defaults | The hint says "Sets the connection color". |
| No colour field | A separate Color list | Dropped. The environment sets the colour (it already did on choosing). Nothing but this dialog read `ColorTag` any more. |
| Password with a "Keychain" check box | Three modes: keyring, ask every time, no password | The box: checked is the keyring, unchecked asks every time. "No password" needs no control: a checked box with nothing typed and nothing saved already saves as no password (`ConnectionForm::to_saved`), and a connection saved that way opens checked (Task 6), so a password typed later is kept. The box is named "Keychain" on macOS and "Keyring" elsewhere. |
| "SSL mode": disable, prefer, require, verify-full | Five modes, labelled "Off", "Prefer (not verified)"… under "TLS" | "SSL mode" with the libpq names the picker already uses: disable, prefer, require, verify-ca, verify-full. The interception warning stays, under the mode. |
| "CA certificate": a path and "Choose…" | A text field, shown only in the modes that use it, no picker | Always shown, greyed out in disable and prefer, still typed into, with a new "Choose…" that opens a file dialog. Its placeholder says what an empty field means in the mode, as today. |
| SSH: host, user, key in one line | Host, port, user, three login methods, key file, a secret and how it is kept | macOS: one line of host, port, user and the login method, named by placeholders; then the key file with "Choose…" when the method is a key; then the secret with the keyring box unless the method is the agent. Terminal: a row each. |
| "Open read-only", on by default for production | Every session is read-only in this version | Shown, checked and locked: "Blocks every write from this app. Always on in this version." It becomes a real setting when editing arrives. |
| Parameters / URL tabs (macOS), "u paste url" (Omarchy) | A "Paste URL" field with a "Fill" button at the bottom of the form | macOS: the URL tab shows the URL field alone, with "Fill"; a URL that fills the form returns to Parameters. Terminal: `u` (when no field has the keyboard) or a click on the hint shows a "url" row at the top; Enter fills. |
| "Delete" in the footer | Delete lives in the picker | Added to the macOS footer when editing: deletes the connection and closes the dialog (as the picker's Delete, without asking). The terminal look keeps `dd` in the picker, as its design does. |
| "Connected · PostgreSQL 17 · 42 ms" | "Connection works." | "Connected · PostgreSQL · 42 ms". The time is measured in the app. The server's version is not known to a Test and is left out. |
| One primary button: Save | Save, and Save & Connect (primary) | Both stay. Editing: Save is primary, as the design. A new connection: Save & Connect is primary. The other one sits before it as a plain button. Terminal: "ctrl+s save" and "ctrl+enter connect". |
| Errors are not drawn | A validation message, a failed Test, an untrusted SSH host key with "Trust and test" | Under the fields: a tinted box on macOS, plain coloured lines in the terminal look. Only a running or passed Test shows in the footer. |
| Centred | Centred while opening, then the top edge stays put | The dialog opens where a server's form would be centred (it reserves 815 pt, 615 in the terminal look), so choosing PostgreSQL does not send the form into a scroll area. The edge stays put from then on. |
| Omarchy: a shadow under the dialog | The terminal look draws dialogs with a 2 pt border and no shadow (`DialogStyle::AccentBorder`) | No shadow, as every other dialog in that look. |

New keys in the dialog, in every look: Mod+S saves, Mod+T tests, Mod+Enter saves and connects.

## What changed while executing

The plan was executed task by task with a spec review and a code quality review after each. The reviews changed these things; the code is the record, this list says where it left the plan's text.

- **Task 4.** The Test's clock starts when the Test is sent to the server (a keyring read is not counted), and only a Test that passed keeps its time.
- **Task 5.** A test that a file pick overtaken by another fills nothing.
- **Task 6.** An "ask every time" connection is tested to keep asking; both `from_saved` mappings name every mode.
- **Task 7, behaviour.**
  - A URL typed in the URL field and not yet filled is used by Save, Save & Connect and Test, which stop when it does not parse. A URL that has filled the form is emptied from the field. In the terminal look the URL also fills the form when the keyboard leaves the field.
  - What went wrong (a field, a failed Test, an SSH host key to trust) is drawn between the fields and the footer, always on screen, not under the fields.
  - The dialog is measured on an unseen frame before it is first painted, and reaches its height in one frame.
  - The dialog's Mod keys are taken before the dialog draws and ignore key repeats; Mod+T does nothing while a Test runs.
  - The terminal footer drops key hints from the left when the status needs the room; a dropped hint keeps its button.
  - The header cuts a long name with an ellipsis. The terminal read-only row wraps in a narrow window.
  - Choices are radio groups named by their heading. The SSH secret's keyring box is named apart from the password's. Fields named by placeholders keep their password role.
- **Task 7, structure.** The view is the directory `src/ui/connect_dialog/` (`mod.rs`, `choice.rs`, `sheet.rs`, `terminal.rs`), not one file. `widgets::toggle` and `ColorTag::label` lost their last caller and were removed. `Harness::press` releases the key it pressed. The password prompt's field is named by its title.
- **Task 9.** The comparison fixed how these are drawn: terminal fields on their row's line and rows as tall as their content; a plain tick for the status; the SSL mode and Authentication lists flat like fields; text insets; the safety note's tone; contrast of the terminal environment buttons on light palettes; no stripe for a connection with no environment. The room the dialog keeps when it opens is 815 pt (615 in the terminal look), the measured height of a server's form.

Left as they are, for a later decision: the macOS check boxes keep the look's outlined box and body-weight labels (the design shows a filled box and a smaller "Keychain" label); the terminal look's backdrop does not dim the window behind a dialog (the design dims it); the Group field and a real read-only setting are not built.

## Rebased onto main

The branch was built on an older `main`. While it was, `main` gained `src/env.rs` (five environments, their colours from `env_colors` only, no colour tags), an environment that follows where the connection points until one is chosen, a stored read-only setting, and the Host aliases of `~/.ssh/config` in the old dialog. The branch was rebuilt on top of that, step by step, and reviewed again. Where this plan's text and the code now differ:

- **Task 2** is gone: `main` already offers the environments in the dialog's order. There is no Test environment.
- **Colours** come from `env_colors` accessors, not from the view: the stripe, the terminal look's border and environment-coloured text are `base()`; the terminal title line and the read-only note's tint are `bar_bg()`; a chosen choice takes `badge_fg()` (and `badge_bg()` in the terminal look). The dialog keeps its own names for the choices ("Local", "Dev", "Staging", "Production", "None").
- **Environment** is chosen by a click and follows the host until then. A connection with no environment has no stripe on the sheet; in the terminal look it takes the muted border and a faintly tinted title line, as the connection bar does.
- **Read-only** is stored now, with the environment's default. The dialog still shows it locked: "Every connection is read-only in 0.1.0" on the sheet, "always on in 0.1.0" in the terminal look, where the row has to fit one line.
- **SSH config hosts**: the "Hosts from ~/.ssh/config" button, the hints from the config as placeholders, the host name line and the ProxyJump and ProxyCommand warnings are in both layouts. A hint longer than its field is cut with an ellipsis.
- **History**: the view is one commit (it holds what were the view, its follow-ups, the split into a directory and the visual fixes); the harness's key release and the password prompt's field name come before it.

## File Structure

| File | Change |
|---|---|
| `src/typography.rs` | Two roles, `FormLabel` and `Legend`. The `legacy` module goes (Task 7). |
| `src/theme.rs` | The re-export of the legacy helpers goes (Task 7). |
| `src/connections.rs` | `Environment::ALL` in the dialog's order; `name()` says "Dev". |
| `src/ui/widgets.rs` | `field`: a single-line field of a given height. |
| `src/model.rs` | `ConnectionForm::{url_mode, test_started, test_took}`, `PickTarget::CaFile`, `Action::PickCaFile`; `from_saved` opens a passwordless connection in the keyring mode; the verify-ca message names the modes as the dialog does. |
| `crates/tabletist-db/src/tls.rs` | The same message, as the driver says it. |
| `docs/superpowers/specs/2026-09-27-tabletist-design.md` | Section 5.4 describes the dialog as it now is. |
| `src/app.rs` | `apply_url` leaves the URL mode; the Test is timed; a picked CA file fills its field. |
| `src/backend.rs` | `Backend::pick_ca_file`. |
| `src/ui/connect_dialog.rs` | Rewritten. |
| `src/ui/mod.rs` | The dialog's headless tests. |
| `src/ui/keys.rs` | The shortcut table lists the dialog's keys. |
| `src/shots.rs` | The dialog scenes, for comparing by eye. |

## Review Focus

1. **A short window.** The form is taller than before. Expected: the footer stays on screen at 720 x 480 in every look; the fields scroll. Pinned by `every_look_lays_out_at_small_and_large_sizes`.
2. **Screen readers.** The segmented choices, the terminal check marks and the fields named by placeholders are custom drawn. Expected: every one has a role and a name. Pinned by `every_interactive_node_is_named` and the tests that click by name.
3. **Secrets.** The keyring box replaces a three-way list. Expected: a saved password is never sent to another server and "ask every time" still works. The model tests in `src/model.rs` and `src/app.rs` are untouched and must pass unchanged.
4. **Keys.** Mod+T opens a connection tab outside the dialog and tests inside it. Expected: no shortcut acts behind the dialog (`shortcuts_are_ignored_while_the_dialog_is_open` passes unchanged).

---

### Task 1: Text roles for the dialog

The design's CSS has two styles no role covers: the labels over Name and Environment (12 medium) and the group headings (12 semibold). Roles are a table, not behaviour: this task has no test (a test that repeats the table is a role-versus-spec copy, which this repository does not write).

**Files:**
- Modify: `src/typography.rs`

- [ ] **Step 1: Add the two roles**

In `pub enum TextRole`, after `InspectorValue`:

```rust
    /// Values in the inspector.
    InspectorValue,
    /// Labels over the connection dialog's name and environment.
    FormLabel,
    /// The connection dialog's group headings (Server, Security).
    Legend,
```

In `TextRole::ALL`, change the length to 36 and add the two after `Self::InspectorValue`:

```rust
    pub const ALL: [TextRole; 36] = [
```

```rust
        Self::InspectorValue,
        Self::FormLabel,
        Self::Legend,
        Self::OBody,
```

In `TextRole::id`, after the `InspectorValue` arm:

```rust
            Self::FormLabel => "form-label",
            Self::Legend => "legend",
```

In `TextRole::spec`, after the `InspectorValue` arm:

```rust
            Self::FormLabel => style(Sans, 500, 12.0),
            Self::Legend => style(Sans, 600, 12.0),
```

- [ ] **Step 2: Run the checks**

Run: `~/.cargo/bin/cargo clippy --locked --workspace --all-targets -- -D warnings && ~/.cargo/bin/cargo test --locked --lib typography`
Expected: no warnings; the typography tests pass.

- [ ] **Step 3: Commit**

```bash
git add src/typography.rs
git commit -m "Add the connection dialog's label and heading roles

The dialog's design labels its name and environment in 12 medium and
heads its groups in 12 semibold; no role had either.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

### Task 2: Environments in the dialog's order

**Files:**
- Modify: `src/connections.rs`

- [ ] **Step 1: Write the failing test**

In `mod tests` of `src/connections.rs`:

```rust
    #[test]
    fn the_dialog_offers_environments_from_local_to_production_then_none() {
        let names: Vec<&str> = Environment::ALL
            .iter()
            .map(|environment| environment.name())
            .collect();
        assert_eq!(
            names,
            ["Local", "Dev", "Test", "Staging", "Production", "None"]
        );
    }
```

- [ ] **Step 2: Run it to see it fail**

Run: `~/.cargo/bin/cargo test --locked --lib the_dialog_offers_environments`
Expected: FAIL. The left side is `["None", "Local", "Development", "Test", "Staging", "Production"]`.

- [ ] **Step 3: Reorder and rename**

```rust
    /// In the order the dialog offers them.
    pub const ALL: [Environment; 6] = [
        Self::Local,
        Self::Dev,
        Self::Test,
        Self::Staging,
        Self::Production,
        Self::None,
    ];
```

In `Environment::name`:

```rust
            Self::Dev => "Dev",
```

- [ ] **Step 4: Run the tests**

Run: `~/.cargo/bin/cargo test --locked --lib`
Expected: PASS, all of them (the old dialog lists the environments in the new order and nothing else depends on it).

- [ ] **Step 5: Commit**

```bash
git add src/connections.rs
git commit -m "Offer environments from local to production

The order the connection dialog's design lists them in, with None last
and Dev by its short name.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

### Task 3: A field of a given height

The dialog's fields are 30, 32 and 34 pt tall, whatever the look's control height is. `widgets::padded` builds a field of `look.control_height`; it learns to take the height.

**Files:**
- Modify: `src/ui/widgets.rs`

- [ ] **Step 1: Write the failing test**

In `mod tests` of `src/ui/widgets.rs`, after `single_line_fields_are_as_tall_as_the_looks_controls`:

```rust
    #[test]
    fn a_field_is_as_tall_as_it_is_asked_to_be() {
        for look in crate::theme::Look::ALL {
            let mut harness = crate::testing::Harness::new();
            harness.set_look(look);
            let mut text = String::new();
            for height in [30.0, 34.0] {
                let mut drawn = 0.0;
                harness.frame_with(|ui| {
                    let role = super::body(&look);
                    drawn = ui
                        .add(super::field(ui, &mut text, &look, role, height, 10))
                        .rect
                        .height();
                });
                assert!(
                    (drawn - height).abs() <= 0.5,
                    "{}: {drawn} for {height}",
                    look.name
                );
            }
        }
    }
```

- [ ] **Step 2: Run it to see it fail**

Run: `~/.cargo/bin/cargo test --locked --lib a_field_is_as_tall`
Expected: FAIL to compile: `cannot find function `field` in module `super``.

- [ ] **Step 3: Add `field`, and give `padded` the height**

Replace `single_in` and `padded` (near the top of the file) with:

```rust
/// [`single`] drawn in `role`.
pub fn single_in<'a>(
    ui: &Ui,
    text: &'a mut String,
    look: &Look,
    role: TextRole,
) -> egui::TextEdit<'a> {
    field(ui, text, look, role, look.control_height, 8)
}

/// A single-line field `height` tall in `role`, with `pad` points at each
/// side of the text: the connection dialog's fields, which the designs
/// draw taller than the look's controls.
pub fn field<'a>(
    ui: &Ui,
    text: &'a mut String,
    look: &Look,
    role: TextRole,
    height: f32,
    pad: i8,
) -> egui::TextEdit<'a> {
    padded(ui, text, height, role.font_id(look.faces), [pad, pad])
}

/// A field `height` tall with `left` and `right` points of padding. egui
/// ignores the height of `TextEdit::min_size`, so the height comes from
/// padding the text above and below, measured for `font`.
fn padded<'a>(
    ui: &Ui,
    text: &'a mut String,
    height: f32,
    font: egui::FontId,
    [left, right]: [i8; 2],
) -> egui::TextEdit<'a> {
    let line = ui.fonts_mut(|fonts| fonts.row_height(&font)) + ui.spacing().extra_text_line_spacing;
    let padding = (height - line).max(0.0);
    let top = (padding / 2.0).floor() as i8;
    let bottom = (padding - f32::from(top)).round() as i8;
    egui::TextEdit::singleline(text)
        .font(font)
        .margin(egui::Margin {
            left,
            right,
            top,
            bottom,
        })
        .vertical_align(egui::Align::Center)
}
```

In `search_field`, the last line becomes:

```rust
    padded(ui, text, look.control_height, font, [left, right]).hint_text(hint)
```

- [ ] **Step 4: Run the tests**

Run: `~/.cargo/bin/cargo test --locked --lib widgets`
Expected: PASS, including `single_line_fields_are_as_tall_as_the_looks_controls` and `search_fields_are_as_tall_as_the_looks_controls` unchanged.

- [ ] **Step 5: Commit**

```bash
git add src/ui/widgets.rs
git commit -m "Let a field say how tall it is

The connection dialog's design draws fields taller than the look's
controls. Fields of the look's height are built from the same code.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

### Task 4: The form knows its URL mode and how long a Test took

**Files:**
- Modify: `src/model.rs` (`ConnectionForm`)
- Modify: `src/app.rs` (`Action::TestConnection`, `Event::Tested`, `apply_url`)

- [ ] **Step 1: Write the failing tests**

In `mod tests` of `src/app.rs`, after `a_pasted_url_fills_the_form_or_explains_why_not`:

```rust
    #[test]
    fn a_url_that_fills_the_form_leaves_the_url_field() {
        let (mut app, _dir) = app();
        app.apply(Action::NewConnection);
        form(&mut app).url_mode = true;
        form(&mut app).url = "not a url".into();
        app.apply(Action::ApplyUrl);
        assert!(form(&mut app).message.is_some());
        assert!(
            form(&mut app).url_mode,
            "a URL that does not parse stays to be fixed"
        );
        form(&mut app).url = "postgres://me@db.example.com/app".into();
        app.apply(Action::ApplyUrl);
        assert!(!form(&mut app).url_mode);
        assert_eq!(form(&mut app).host, "db.example.com");
    }

    #[test]
    fn a_test_that_passes_says_how_long_it_took() {
        let (mut app, _dir) = app();
        app.apply(Action::NewConnection);
        form(&mut app).url = "postgres://me@localhost/app".into();
        app.apply(Action::ApplyUrl);
        app.apply(Action::TestConnection);
        let request = match form(&mut app).test {
            TestState::Running(request) => request,
            ref other => panic!("{other:?}"),
        };
        assert!(form(&mut app).test_started.is_some());
        app.apply(Action::Backend(Event::Tested {
            request,
            result: Ok(()),
        }));
        assert_eq!(form(&mut app).test, TestState::Passed);
        assert!(form(&mut app).test_took.is_some());
        // The next Test forgets that time until it finishes itself.
        app.apply(Action::TestConnection);
        assert!(form(&mut app).test_took.is_none());
    }
```

- [ ] **Step 2: Run them to see them fail**

Run: `~/.cargo/bin/cargo test --locked --lib a_url_that_fills`, then `~/.cargo/bin/cargo test --locked --lib a_test_that_passes`
Expected: FAIL to compile: no field `url_mode`, `test_started`, `test_took` on `ConnectionForm`.

- [ ] **Step 3: Add the fields**

In `pub struct ConnectionForm` (`src/model.rs`), replace the `url` field's lines and add after `test`:

```rust
    /// Text in the URL field.
    pub url: String,
    /// The URL field is showing: the dialog's URL tab, the terminal look's
    /// `u`.
    pub url_mode: bool,
```

```rust
    pub test: TestState,
    /// When the running Test was sent to the server.
    pub test_started: Option<std::time::Instant>,
    /// How long the Test that passed took.
    pub test_took: Option<std::time::Duration>,
```

In `impl Default for ConnectionForm`:

```rust
            url: String::new(),
            url_mode: false,
            message: None,
            test: TestState::Idle,
            test_started: None,
            test_took: None,
```

- [ ] **Step 4: Time the Test and leave the URL mode**

In `src/app.rs`, `Action::TestConnection`, where the Test starts, it has no clock yet:

```rust
                form.message = None;
                form.test = TestState::Running(request);
                form.test_started = None;
                form.test_took = None;
```

The clock starts where `Command::Test` is sent, at both places (the one in `Action::TestConnection` when no saved secret has to load, and the one after the last saved secret arrives), so a keyring prompt is not counted:

```rust
                    form.test_started = Some(std::time::Instant::now());
```

In `Event::Tested`, before `form.test = match (result, tested) {`, only a Test that passed keeps its time:

```rust
                    let took = form.test_started.take().map(|started| started.elapsed());
                    form.test_took = if result.is_ok() { took } else { None };
```

(Revised during execution, after code review: the plan first started the clock when the Test began, which counted keyring reads. The tests grew with it: a SQLite step in the URL test, a failed Test in the timing test, and `the_test_clock_starts_when_the_test_is_sent_not_while_the_keyring_is_read`.)

In `fn apply_url`, both arms end by clearing the message. Each also leaves the URL mode:

```rust
            form.message = None;
            form.url_mode = false;
```

- [ ] **Step 5: Run the tests**

Run: `~/.cargo/bin/cargo test --locked --lib`
Expected: PASS, all of them.

- [ ] **Step 6: Commit**

```bash
git add src/model.rs src/app.rs
git commit -m "Time a connection test and track the dialog's URL mode

The dialog's design says how long a passed test took, and shows the URL
field as a mode of its own that a filled form leaves.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

### Task 5: Choose the CA certificate from a file dialog

**Files:**
- Modify: `src/model.rs` (`Action`, `PickTarget`)
- Modify: `src/app.rs` (`App::apply`, `Event::FilePicked`)
- Modify: `src/backend.rs` (`Backend`)

- [ ] **Step 1: Write the failing test**

In `mod tests` of `src/app.rs`, after `a_picked_file_fills_the_path_and_a_cancelled_pick_changes_nothing`:

```rust
    #[test]
    fn a_picked_ca_certificate_fills_its_field() {
        let (mut app, _dir) = app();
        app.apply(Action::NewConnection);
        app.apply(Action::PickCaFile);
        let request = form(&mut app).pick_request.expect("a pick is in flight");
        assert_eq!(form(&mut app).pick_target, PickTarget::CaFile);
        app.apply(Action::Backend(Event::FilePicked {
            request,
            path: Some("/etc/ssl/ca.pem".into()),
        }));
        assert_eq!(form(&mut app).ca_file, "/etc/ssl/ca.pem");
        assert_eq!(form(&mut app).sqlite_path, "", "only the CA file changes");
    }
```

- [ ] **Step 2: Run it to see it fail**

Run: `~/.cargo/bin/cargo test --locked --lib a_picked_ca_certificate`
Expected: FAIL to compile: no variant `PickCaFile`, no variant `CaFile`.

- [ ] **Step 3: Add the action and the target**

In `pub enum Action` (`src/model.rs`), after `PickKeyFile`:

```rust
    /// Open the native file dialog for the CA certificate.
    PickCaFile,
```

`PickTarget`:

```rust
pub enum PickTarget {
    #[default]
    Sqlite,
    KeyFile,
    CaFile,
}
```

- [ ] **Step 4: Ask for the file**

In `src/backend.rs`, after `pick_key_file`:

```rust
    /// Asks for the CA certificate a server's certificate is checked against.
    pub fn pick_ca_file(&mut self, request: RequestId) {
        let Some(runtime) = &self.runtime else {
            return;
        };
        let dialog = rfd::AsyncFileDialog::new()
            .set_title("Choose a CA certificate")
            .add_filter("Certificates", &["pem", "crt", "cer"])
            .add_filter("All files", &["*"])
            .pick_file();
        let outbox = self.outbox.clone();
        runtime.spawn(async move {
            let path = dialog.await.map(|file| file.path().to_path_buf());
            outbox.emit(Event::FilePicked { request, path });
        });
    }
```

In `App::apply` (`src/app.rs`), after the `Action::PickKeyFile` arm:

```rust
            Action::PickCaFile => {
                let request = RequestId(self.next_id());
                if let Some(Dialog::Connection(form)) = &mut self.dialog {
                    form.pick_request = Some(request);
                    form.pick_target = PickTarget::CaFile;
                    self.backend.pick_ca_file(request);
                }
            }
```

In `Event::FilePicked`, after the `PickTarget::KeyFile` arm:

```rust
                        (Some(path), PickTarget::CaFile) => {
                            form.ca_file = path.display().to_string();
                        }
```

- [ ] **Step 5: Run the tests**

Run: `~/.cargo/bin/cargo test --locked --lib`
Expected: PASS, all of them.

- [ ] **Step 6: Commit**

```bash
git add src/model.rs src/app.rs src/backend.rs
git commit -m "Pick the CA certificate from a file dialog

The connection dialog's design gives the CA certificate a Choose button,
as the SQLite file and the SSH key have.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

### Task 6: A connection saved without a password opens ready to take one

The new dialog shows the password's mode as a check box: saved in the keyring, or asked for every time. Today a connection saved with "No password" opens in that third mode, and a password typed into it would be dropped on save. It opens in the keyring mode instead. Nothing is lost: with nothing typed and nothing saved, `ConnectionForm::to_saved` still saves "no password".

**Files:**
- Modify: `src/model.rs` (`ConnectionForm::from_saved`)
- Test: `src/app.rs` (`mod tests`)

- [ ] **Step 1: Write the failing test**

In `mod tests` of `src/app.rs`, after `saving_without_retyping_keeps_the_keyring_password`:

```rust
    #[test]
    fn a_password_typed_into_a_connection_saved_without_one_is_stored() {
        let (mut app, _dir) = app();
        let conn = postgres_saved(&mut app, PasswordMode::None);
        app.apply(Action::EditConnection(conn.clone()));
        assert_eq!(form(&mut app).password_mode, PasswordMode::Keyring);
        // Saved again untouched, it still has no password.
        app.apply(Action::SaveConnection { connect: false });
        assert_eq!(
            app.connections.get(&conn).unwrap().password,
            PasswordMode::None
        );
        assert!(sent_secrets(&app).is_empty());
        app.apply(Action::EditConnection(conn.clone()));
        form(&mut app).password = "pw".into();
        app.apply(Action::SaveConnection { connect: false });
        assert_eq!(
            app.connections.get(&conn).unwrap().password,
            PasswordMode::Keyring
        );
        assert!(app.backend.sent.iter().any(|c| matches!(
            c,
            Command::StoreSecret { secret: Some(SecretString(p)), .. } if p == "pw"
        )));
    }
```

- [ ] **Step 2: Run it to see it fail**

Run: `~/.cargo/bin/cargo test --locked --lib a_password_typed_into`
Expected: FAIL: `left: None, right: Keyring` on the first assertion.

- [ ] **Step 3: Open such a connection in the keyring mode**

In `ConnectionForm::from_saved` (`src/model.rs`), `password_mode: saved.password,` becomes:

```rust
            // The dialog's check box has two states. A connection saved
            // without a password opens checked, and still saves without one
            // unless one is typed (see `to_saved`).
            password_mode: match saved.password {
                PasswordMode::Ask => PasswordMode::Ask,
                _ => PasswordMode::Keyring,
            },
```

- [ ] **Step 4: Run the tests**

Run: `~/.cargo/bin/cargo test --locked --lib`
Expected: PASS, all of them.

- [ ] **Step 5: Commit**

```bash
git add src/model.rs src/app.rs
git commit -m "Open a passwordless connection ready to take a password

A connection saved without a password opened in a mode that dropped a
password typed into it unless the mode was changed first. It opens in
the keyring mode, and still saves without a password when none is typed.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

### Task 7: The dialog

The view is replaced whole. Its tests come first: the ones that name what changed are ported, and each new behaviour gets one.

**Files:**
- Modify: `src/ui/mod.rs` (`mod tests`)
- Replace: `src/ui/connect_dialog.rs`
- Modify: `src/typography.rs`, `src/theme.rs` (the legacy helpers go)
- Modify: `src/model.rs`, `crates/tabletist-db/src/tls.rs` (the verify-ca message)

- [ ] **Step 1: Port the tests that name what changed**

All in `mod tests` of `src/ui/mod.rs`.

In `every_look_lays_out_at_small_and_large_sizes`, the dialog's part becomes:

```rust
                // The connection dialog keeps its buttons on screen; the
                // PostgreSQL form with the SSH tunnel open is the tallest.
                harness.press(Key::N, Modifiers::COMMAND);
                harness.click(&look.label("PostgreSQL"));
                harness.click("Connect through SSH tunnel");
```

Replace `escape_with_the_color_list_open_keeps_the_dialog` with:

```rust
    #[test]
    fn escape_with_a_list_open_keeps_the_dialog() {
        let mut harness = Harness::new();
        harness.press(Key::N, Modifiers::COMMAND);
        harness.click("PostgreSQL");
        let tree = harness.settle();
        let combo = tree
            .nodes
            .iter()
            .find(|(_, node)| node.role() == egui::accesskit::Role::ComboBox)
            .map(|(id, _)| *id)
            .expect("the SSL mode list");
        harness.frame(vec![egui::Event::AccessKitActionRequest(
            egui::accesskit::ActionRequest {
                target_tree: egui::accesskit::TreeId::ROOT,
                target_node: combo,
                action: egui::accesskit::Action::Click,
                data: None,
            },
        )]);
        harness.press(Key::Escape, Modifiers::NONE);
        assert!(
            harness.app.dialog.is_some(),
            "Escape closes only the open list"
        );
        harness.press(Key::Escape, Modifiers::NONE);
        assert!(harness.app.dialog.is_none());
    }
```

Replace `choosing_an_environment_colours_the_connection` with:

```rust
    #[test]
    fn choosing_an_environment_colours_the_connection() {
        let mut harness = Harness::new();
        harness.press(Key::N, Modifiers::COMMAND);
        harness.click("Production");
        let form = form(&harness);
        assert_eq!(
            form.environment,
            crate::connections::Environment::Production
        );
        assert_eq!(form.color, crate::connections::ColorTag::Red);
    }
```

Replace `paste_url_suggests_the_chosen_drivers_url` with:

```rust
    #[test]
    fn the_url_field_suggests_the_chosen_drivers_url() {
        let mut harness = Harness::new();
        harness.press(Key::N, Modifiers::COMMAND);
        let placeholders = |harness: &mut Harness| -> Vec<String> {
            harness
                .settle()
                .nodes
                .iter()
                .filter_map(|(_, node)| node.placeholder().map(str::to_owned))
                .collect()
        };
        harness.click("URL");
        assert!(placeholders(&mut harness).contains(&"sqlite:///path/to/file.db".to_owned()));
        harness.click("Parameters");
        harness.click("PostgreSQL");
        harness.click("URL");
        assert!(placeholders(&mut harness).contains(&"postgres://user@host/db".to_owned()));
    }
```

In `choosing_postgres_shows_its_fields` and `choosing_mysql_shows_the_server_fields`, the labels become:

```rust
        for label in ["Host", "Port", "User", "Password", "Database", "SSL mode"] {
```

Replace `a_ca_file_used_by_require_stays_visible` with:

```rust
    /// The field named by the "CA certificate" label beside it.
    fn ca_field(tree: &egui::accesskit::TreeUpdate) -> &egui::accesskit::Node {
        let (label, _) = tree
            .nodes
            .iter()
            .find(|(_, node)| node.value() == Some("CA certificate"))
            .expect("CA certificate label");
        &tree
            .nodes
            .iter()
            .find(|(_, node)| {
                node.role() == egui::accesskit::Role::TextInput
                    && node.labelled_by().contains(label)
            })
            .expect("CA certificate field")
            .1
    }

    #[test]
    fn the_ca_certificate_is_offered_where_it_is_checked() {
        let mut harness = Harness::new();
        harness.press(Key::N, Modifiers::COMMAND);
        harness.click("PostgreSQL");
        let offered = |harness: &mut Harness, tls| {
            if let Some(crate::model::Dialog::Connection(form)) = &mut harness.app.dialog {
                form.tls = tls;
            }
            let tree = harness.settle();
            !ca_field(&tree).is_disabled()
        };
        assert!(!offered(&mut harness, tabletist_db::TlsMode::Prefer));
        // `require` checks against a CA file too, as libpq does.
        assert!(offered(&mut harness, tabletist_db::TlsMode::Require));
        assert!(offered(&mut harness, tabletist_db::TlsMode::VerifyFull));
    }
```

In `only_verify_full_offers_the_system_certificates`, the `ca_hint` closure becomes:

```rust
        let ca_hint = |harness: &mut Harness, tls| {
            match &mut harness.app.dialog {
                Some(crate::model::Dialog::Connection(form)) => form.tls = tls,
                other => panic!("{other:?}"),
            }
            let tree = harness.settle();
            ca_field(&tree).placeholder().map(str::to_owned)
        };
```

Replace `the_ssh_section_shows_the_fields_for_each_method` with:

```rust
    #[test]
    fn the_ssh_tunnel_shows_the_fields_for_each_method() {
        let mut harness = Harness::new();
        harness.press(Key::N, Modifiers::COMMAND);
        harness.click("PostgreSQL");
        assert!(!harness.has("SSH host"));
        harness.click("Connect through SSH tunnel");
        for label in [
            "SSH host",
            "SSH port",
            "SSH user",
            "Authentication",
            "SSH password",
        ] {
            assert!(harness.has(label), "{label}");
        }
        assert!(!harness.has("Passphrase"));
        match &mut harness.app.dialog {
            Some(crate::model::Dialog::Connection(form)) => {
                form.ssh_auth = crate::model::SshAuthKind::KeyFile
            }
            other => panic!("{other:?}"),
        }
        harness.settle();
        assert!(harness.has("Key file"));
        assert!(harness.has("Passphrase"));
        assert!(!harness.has("SSH password"));
    }
```

Replace `sqlite_has_no_ssh_section` with:

```rust
    #[test]
    fn sqlite_has_no_ssh_tunnel() {
        let mut harness = Harness::new();
        harness.press(Key::N, Modifiers::COMMAND);
        assert!(!harness.has("Connect through SSH tunnel"));
    }
```

In `scenes()` (used by `every_interactive_node_is_named`), the connection dialog's scene becomes:

```rust
        let mut harness = Harness::new();
        harness.press(Key::N, Modifiers::COMMAND);
        harness.click("PostgreSQL");
        harness.click("Connect through SSH tunnel");
        scenes.push(("connection dialog", harness.settle()));
```

- [ ] **Step 2: Write the tests for what is new**

After `the_ssh_tunnel_shows_the_fields_for_each_method`:

```rust
    #[test]
    fn the_dialog_draws_each_looks_own_form() {
        // The terminal look: lower-case labels, a row each, keys for buttons.
        let mut harness = Harness::new();
        harness.set_look(crate::theme::Look::omarchy());
        harness.press(Key::N, Modifiers::COMMAND);
        harness.click("postgresql");
        for label in [
            "new connection",
            "host : port",
            "ssl mode",
            "ca cert",
            "read-only",
        ] {
            assert!(harness.has(label), "{label}");
        }
        // Screen readers still get each field and button by its name.
        for label in ["Host", "Port", "Test", "Save", "Save & Connect", "Cancel"] {
            assert!(harness.has(label), "{label}");
        }
        // Elsewhere: labelled fields in groups.
        let mut harness = Harness::new();
        harness.press(Key::N, Modifiers::COMMAND);
        harness.click("PostgreSQL");
        for label in ["Server", "Security", "SSL mode", "CA certificate"] {
            assert!(harness.has(label), "{label}");
        }
    }

    #[test]
    fn the_dialog_says_the_connection_opens_read_only() {
        for look in crate::theme::Look::ALL {
            let mut harness = Harness::new();
            harness.set_look(look);
            harness.press(Key::N, Modifiers::COMMAND);
            let tree = harness.settle();
            assert!(
                crate::testing::node(&tree, "Open read-only", egui::accesskit::Role::CheckBox)
                    .is_some(),
                "{}",
                look.name
            );
        }
    }

    #[test]
    fn an_edit_can_delete_its_connection_and_a_new_one_cannot() {
        let mut harness = Harness::new();
        let id = add_saved(&mut harness, "Shop");
        harness
            .app
            .apply(crate::model::Action::EditConnection(id.clone()));
        assert!(harness.has("Edit connection"));
        harness.click("Delete Shop");
        assert!(harness.app.dialog.is_none());
        assert!(harness.app.connections.get(&id).is_none());
        harness.press(Key::N, Modifiers::COMMAND);
        let tree = harness.settle();
        let deletes: Vec<String> = crate::testing::labels(&tree)
            .into_iter()
            .filter(|label| label.starts_with("Delete"))
            .collect();
        assert!(deletes.is_empty(), "{deletes:?}");
    }

    #[test]
    fn a_test_that_passes_says_what_it_reached() {
        let mut harness = Harness::new();
        harness.press(Key::N, Modifiers::COMMAND);
        harness.click("PostgreSQL");
        if let Some(crate::model::Dialog::Connection(form)) = &mut harness.app.dialog {
            form.user = "me".into();
        }
        harness.click("Test");
        assert!(harness.has("Testing…"));
        let request = match &form(&harness).test {
            crate::model::TestState::Running(request) => *request,
            other => panic!("{other:?}"),
        };
        harness.app.apply(crate::model::Action::Backend(
            crate::backend::Event::Tested {
                request,
                result: Ok(()),
            },
        ));
        let tree = harness.settle();
        let said = crate::testing::labels(&tree);
        assert!(
            said.iter()
                .any(|label| label.starts_with("Connected · PostgreSQL · ")),
            "{said:?}"
        );
    }

    #[test]
    fn the_keyring_box_saves_the_password_or_asks_every_time() {
        use crate::connections::PasswordMode;
        let mut harness = Harness::new();
        harness.press(Key::N, Modifiers::COMMAND);
        harness.click("PostgreSQL");
        assert_eq!(form(&harness).password_mode, PasswordMode::Keyring);
        harness.click("Keyring");
        assert_eq!(form(&harness).password_mode, PasswordMode::Ask);
        harness.click("Keyring");
        assert_eq!(form(&harness).password_mode, PasswordMode::Keyring);
    }

    #[test]
    fn the_url_tab_fills_the_parameters_and_returns_to_them() {
        let mut harness = Harness::new();
        harness.press(Key::N, Modifiers::COMMAND);
        harness.click("URL");
        assert!(
            !harness.has("Environment"),
            "the URL tab shows the URL alone"
        );
        if let Some(crate::model::Dialog::Connection(form)) = &mut harness.app.dialog {
            form.url = "postgres://me@db.example.com/app".into();
        }
        harness.click("Fill");
        assert!(!form(&harness).url_mode);
        assert_eq!(form(&harness).host, "db.example.com");
        assert!(harness.has("Environment"));
    }

    #[test]
    fn choose_asks_for_a_ca_certificate() {
        let mut harness = Harness::new();
        harness.press(Key::N, Modifiers::COMMAND);
        harness.click("PostgreSQL");
        if let Some(crate::model::Dialog::Connection(form)) = &mut harness.app.dialog {
            form.tls = tabletist_db::TlsMode::VerifyFull;
        }
        harness.click("Choose a CA certificate");
        assert_eq!(form(&harness).pick_target, crate::model::PickTarget::CaFile);
        assert!(form(&harness).pick_request.is_some());
    }

    #[test]
    fn the_dialogs_keys_test_and_save() {
        let mut harness = Harness::new();
        harness.press(Key::N, Modifiers::COMMAND);
        harness.click("PostgreSQL");
        if let Some(crate::model::Dialog::Connection(form)) = &mut harness.app.dialog {
            form.name = "Shop".into();
            form.user = "me".into();
        }
        let before = harness.app.backend.sent.len();
        harness.press(Key::T, Modifiers::COMMAND);
        assert!(
            harness.app.backend.sent[before..]
                .iter()
                .any(|command| matches!(command, Command::Test { .. })),
            "Mod+T tests"
        );
        assert_eq!(harness.app.tabs.len(), 1, "and opens no tab behind the dialog");
        harness.press(Key::S, Modifiers::COMMAND);
        assert!(harness.app.dialog.is_none(), "Mod+S saves");
        assert_eq!(harness.app.connections.connections.len(), 1);
    }

    #[test]
    fn u_shows_the_url_field_in_the_terminal_look() {
        let mut harness = Harness::new();
        harness.set_look(crate::theme::Look::omarchy());
        let id = add_saved(&mut harness, "Shop");
        harness
            .app
            .apply(crate::model::Action::EditConnection(id));
        harness.press(Key::U, Modifiers::NONE);
        assert!(form(&harness).url_mode);
        assert!(harness.has("url"));
        // A new connection opens with the keyboard in Name: there, u is a
        // letter.
        let mut harness = Harness::new();
        harness.set_look(crate::theme::Look::omarchy());
        harness.press(Key::N, Modifiers::COMMAND);
        harness.press(Key::U, Modifiers::NONE);
        assert!(!form(&harness).url_mode);
    }
```

- [ ] **Step 3: Run them to see them fail**

Run: `~/.cargo/bin/cargo test --locked --lib ui::tests`
Expected: FAIL. The ported tests fail on the old labels ("nothing labelled \"Connect through SSH tunnel\"", "SSL mode"), the new ones on what the old dialog lacks ("nothing labelled \"URL\"", "nothing labelled \"Delete Shop\""). `the_connection_dialog_keeps_its_place_as_it_grows`, `shortcuts_are_ignored_while_the_dialog_is_open`, `a_new_connection_starts_in_name_and_escape_closes_it_while_typing` and `the_dialog_warns_when_a_remote_password_is_not_protected` are not touched and still pass.

- [ ] **Step 4: Replace the view**

Replace the whole of `src/ui/connect_dialog.rs` with:

```rust
//! The connection dialog: new or edit, any driver, with an optional SSH
//! tunnel. macOS draws a sheet of labelled fields in groups under the
//! environment's colour; the terminal look a two-column form with its keys
//! in the footer.

use egui::{
    Color32, CornerRadius, Rect, Response, Sense, Stroke, StrokeKind, Ui, WidgetInfo, WidgetType,
    pos2, vec2,
};
use tabletist_db::{Driver, TlsMode};

use crate::app::App;
use crate::connections::{Environment, PasswordMode};
use crate::i18n::{Locale, gettext};
use crate::model::{Action, ConnectionForm, Dialog, SshAuthKind, TestState};
use crate::theme::{self, EnvColors, Faces, Icon, Look, Palette};
use crate::typography::{Text, TextRole};
use crate::ui::widgets::{self, ButtonSpec};

/// The TLS modes by their libpq names, as the picker says them.
const TLS_MODES: [(TlsMode, &str); 5] = [
    (TlsMode::Disable, "disable"),
    (TlsMode::Prefer, "prefer"),
    (TlsMode::Require, "require"),
    (TlsMode::VerifyCa, "verify-ca"),
    (TlsMode::VerifyFull, "verify-full"),
];

const DRIVERS: [Driver; 3] = [Driver::Sqlite, Driver::Postgres, Driver::MySql];

const INTERCEPT_WARNING: &str = "The password can be intercepted on the network. \
                                 To prevent it, verify the certificate and host, \
                                 or use an SSH tunnel.";

/// The stripe in the environment's colour along the macOS dialog's top.
const STRIPE: f32 = 5.0;

fn tls_label(mode: TlsMode) -> &'static str {
    TLS_MODES
        .iter()
        .find(|(m, _)| *m == mode)
        .map_or("", |(_, label)| label)
}

/// Where the dialog's top edge settled once it opened (see [`Placement`]).
fn placement_id() -> egui::Id {
    egui::Id::new("connection-dialog-placement")
}

/// Set for one frame after the URL field appears: it takes the keyboard.
fn focus_url_id() -> egui::Id {
    egui::Id::new("connection-dialog-focus-url")
}

/// The dialog's place. egui centres a modal on its size, so a dialog that
/// grows (PostgreSQL's fields, the SSH section, the TLS warning) would move
/// under the pointer. It opens where a server's form would be centred, so
/// the short SQLite form and that one share a top edge; then the edge stays
/// put and the dialog grows downwards.
#[derive(Clone, Copy, Default)]
struct Placement {
    /// Frames shown so far (sizes settle on the second).
    frames: u8,
    top: Option<f32>,
}

/// The height the dialog keeps room for when it opens: a server's form
/// without a tunnel.
fn reserve(look: &Look) -> f32 {
    if look.terminal { 650.0 } else { 760.0 }
}

/// Where the top edge of a dialog `height` tall goes on a screen
/// `screen` tall.
fn top_edge(screen: f32, height: f32) -> f32 {
    ((screen - height) / 2.0).max(24.0)
}

/// How the dialog draws: the look, the palette, and the environment that
/// colours it.
#[derive(Clone, Copy)]
struct Skin<'a> {
    look: &'a Look,
    palette: &'a Palette,
    locale: Locale,
    environment: Environment,
    env: EnvColors,
    /// The dialog's own background.
    fill: Color32,
}

impl<'a> Skin<'a> {
    fn new(look: &'a Look, palette: &'a Palette, locale: Locale, environment: Environment) -> Self {
        Self {
            look,
            palette,
            locale,
            environment,
            env: theme::env_colors(environment, palette),
            fill: if look.terminal {
                palette.window
            } else {
                palette.overlay
            },
        }
    }

    /// `text` translated, in the look's case.
    fn say(&self, text: &'static str) -> String {
        self.look.label(&gettext(self.locale, text))
    }

    /// The colour of the terminal look's border: the environment's, or the
    /// accent every other dialog takes when there is none.
    fn edge(&self) -> Color32 {
        if self.environment == Environment::None {
            self.palette.accent
        } else {
            self.env.color
        }
    }

    /// The dialog's border: the terminal look's 2 pt edge, a hairline
    /// where a dark dialog would merge with a dark window, none on light.
    fn stroke(&self) -> Stroke {
        if self.look.terminal {
            Stroke::new(2.0, self.edge())
        } else if self.palette.dark {
            Stroke::new(1.0, self.palette.outline)
        } else {
            Stroke::NONE
        }
    }

    /// The dialog's corners, and those of what fills it edge to edge.
    fn radius(&self) -> u8 {
        if self.look.terminal {
            6
        } else {
            self.look.dialog_radius + 2
        }
    }

    fn inner_radius(&self) -> u8 {
        self.radius().saturating_sub(self.stroke().width as u8)
    }

    /// Rules between the dialog's parts and round its groups.
    fn rule(&self) -> Color32 {
        if self.look.terminal {
            self.palette.outline
        } else {
            self.palette.surface_hover
        }
    }

    /// The footer's fill: the panel tone, or the window's where the dialog
    /// itself has the panel's.
    fn footer(&self) -> Color32 {
        if self.fill == self.palette.panel {
            self.palette.window
        } else {
            self.palette.panel
        }
    }

    /// The environment's colour as text on the dialog.
    fn env_ink(&self, colors: &EnvColors) -> Color32 {
        if self.palette.dark {
            colors.color
        } else {
            colors.badge_text
        }
    }

    /// Where saved passwords go, as this platform calls it.
    fn keyring(&self) -> &'static str {
        if self.look.faces == Faces::Plex {
            "Keychain"
        } else {
            "Keyring"
        }
    }

    fn saved_hint(&self) -> &'static str {
        if self.look.faces == Faces::Plex {
            "Saved in the keychain"
        } else {
            "Saved in the keyring"
        }
    }

    /// The height of a field: the terminal's 30, macOS's 32 (34 for the
    /// name).
    fn field_height(&self) -> f32 {
        if self.look.terminal { 30.0 } else { 32.0 }
    }

    /// Monospace for what goes to the server: hosts, names, paths.
    fn value_role(&self) -> TextRole {
        TextRole::pick(self.look, TextRole::GridCell, TextRole::OBody)
    }
}

pub fn show(app: &mut App, ctx: &egui::Context) {
    let locale = app.locale;
    let palette = app.palette;
    let look = app.look;
    let Some(Dialog::Connection(form)) = &mut app.dialog else {
        ctx.data_mut(|data| data.remove::<Placement>(placement_id()));
        return;
    };
    let placement: Placement = ctx
        .data(|data| data.get_temp(placement_id()))
        .unwrap_or_default();
    let focus_name = std::mem::take(&mut form.focus_name);
    let mut actions = Vec::new();
    let skin = Skin::new(&look, &palette, locale, form.environment);
    let screen = ctx.content_rect();
    let width: f32 = if look.terminal { 780.0 } else { 700.0 };
    let width = width.min(screen.width() - 48.0).max(320.0);
    // A modal: nothing behind it can be clicked while it is open.
    let id = egui::Id::new("connection-dialog");
    let top = placement
        .top
        .unwrap_or_else(|| top_edge(screen.height(), reserve(&look)));
    let modal = widgets::modal(id, &look, &palette)
        .frame(frame(&skin))
        .area(egui::Modal::default_area(id).anchor(egui::Align2::CENTER_TOP, vec2(0.0, top)));
    let modal = modal.show(ctx, |ui| {
        ui.set_width(width);
        style_controls(ui, &skin);
        if look.terminal {
            terminal_header(ui, form, &skin);
        } else {
            header(ui, form, &skin, &mut actions);
        }
        // The fields scroll when the window is short, so the footer stays
        // on screen.
        let chrome = if look.terminal { 86.0 } else { 140.0 };
        let room = (screen.bottom() - top - chrome - 24.0).max(120.0);
        egui::ScrollArea::vertical()
            .max_height(room)
            .auto_shrink([false, true])
            .show(ui, |ui| {
                if look.terminal {
                    terminal_body(ui, form, &skin, focus_name, &mut actions);
                } else {
                    body(ui, form, &skin, focus_name, &mut actions);
                }
            });
        if look.terminal {
            terminal_footer(ui, form, &skin, &mut actions);
        } else {
            footer(ui, form, &skin, &mut actions);
        }
    });
    // A form taller than the room kept for it (a tunnel's fields) is
    // centred on its own height once that is known; the edge stays put
    // from then on.
    if placement.top.is_none() {
        let frames = placement.frames.saturating_add(1);
        let top = (frames >= 2).then(|| {
            let height = modal.response.rect.height().max(reserve(&look));
            top_edge(screen.height(), height)
        });
        ctx.data_mut(|data| data.insert_temp(placement_id(), Placement { frames, top }));
    }
    if modal.is_top_modal && !modal.any_popup_open {
        let typing = ctx.text_edit_focused();
        let toggle_url = ctx.input_mut(|input| {
            use egui::{Key, Modifiers};
            if input.consume_key(Modifiers::COMMAND, Key::Enter) {
                actions.push(Action::SaveConnection { connect: true });
            }
            if input.consume_key(Modifiers::COMMAND, Key::S) {
                actions.push(Action::SaveConnection { connect: false });
            }
            if input.consume_key(Modifiers::COMMAND, Key::T) {
                actions.push(Action::TestConnection);
            }
            // Escape closes the dialog, unless it is closing an open list
            // first.
            if input.consume_key(Modifiers::NONE, Key::Escape) {
                actions.push(Action::CloseDialog);
            }
            // The terminal look's `u`, when it is not a letter being typed.
            look.terminal && !typing && input.consume_key(Modifiers::NONE, Key::U)
        });
        // After the input is released: showing the field writes to egui's
        // memory, which sits behind the same lock.
        if toggle_url {
            show_url(ctx, form, !form.url_mode);
        }
    }
    app.actions.extend(actions);
}

/// Shows or hides the URL field; shown, it takes the keyboard.
fn show_url(ctx: &egui::Context, form: &mut ConnectionForm, shown: bool) {
    form.url_mode = shown;
    if shown {
        ctx.data_mut(|data| data.insert_temp(focus_url_id(), true));
    }
}

/// The dialog's frame: its parts fill it edge to edge, so it has no margin
/// of its own.
fn frame(skin: &Skin) -> egui::Frame {
    let frame = egui::Frame::new()
        .fill(skin.fill)
        .corner_radius(CornerRadius::same(skin.radius()))
        .stroke(skin.stroke());
    if skin.look.terminal {
        frame
    } else {
        frame.shadow(egui::epaint::Shadow {
            offset: [0, 24],
            blur: 64,
            spread: 0,
            color: skin.palette.shadow,
        })
    }
}

/// Fields and pop-up buttons as the designs draw them: boxes with a border
/// on the field colour, and no spacing but what the layout adds.
fn style_controls(ui: &mut Ui, skin: &Skin) {
    let Skin { look, palette, .. } = *skin;
    let (fill, border, radius, width) = if look.terminal {
        (palette.panel, palette.outline, 3, 1.0)
    } else {
        let fill = if palette.dark {
            palette.surface
        } else {
            palette.window
        };
        (
            fill,
            palette.border,
            look.radius.saturating_sub(1),
            widgets::hairline(ui),
        )
    };
    let visuals = ui.visuals_mut();
    visuals.extreme_bg_color = fill;
    for state in [
        &mut visuals.widgets.inactive,
        &mut visuals.widgets.hovered,
        &mut visuals.widgets.active,
        &mut visuals.widgets.open,
    ] {
        state.corner_radius = CornerRadius::same(radius);
        state.bg_stroke = Stroke::new(width, border);
    }
    ui.spacing_mut().item_spacing = egui::Vec2::ZERO;
    ui.spacing_mut().interact_size.y = skin.field_height();
}

/// The top `height` points of `rect` with its top corners rounded at
/// `radius`: a stripe that follows the dialog's corners, which a rounded
/// rectangle that thin cannot.
fn top_cap(rect: Rect, radius: f32, height: f32) -> Vec<egui::Pos2> {
    let bottom = rect.top() + height;
    let radius = radius.max(0.0);
    let from = if radius > 0.0 {
        ((radius - height).max(0.0) / radius).asin()
    } else {
        std::f32::consts::FRAC_PI_2
    };
    // From the stripe's lower edge up to the top: how far in from the
    // side, how far down from the top.
    const STEPS: usize = 8;
    let arc: Vec<(f32, f32)> = (0..=STEPS)
        .map(|step| {
            let angle = from + (std::f32::consts::FRAC_PI_2 - from) * step as f32 / STEPS as f32;
            (radius - radius * angle.cos(), radius - radius * angle.sin())
        })
        .collect();
    let mut points = Vec::with_capacity(2 * arc.len() + 2);
    if height > radius {
        points.push(pos2(rect.left(), bottom));
    }
    points.extend(
        arc.iter()
            .map(|(dx, dy)| pos2(rect.left() + dx, rect.top() + dy)),
    );
    points.extend(
        arc.iter()
            .rev()
            .map(|(dx, dy)| pos2(rect.right() - dx, rect.top() + dy)),
    );
    if height > radius {
        points.push(pos2(rect.right(), bottom));
    }
    points
}

fn title(form: &ConnectionForm) -> &'static str {
    if form.editing.is_some() {
        "Edit connection"
    } else {
        "New connection"
    }
}

/// What the header says under (or after) the title: the connection's name
/// and environment, or its kind until it has either.
fn subtitle(form: &ConnectionForm) -> String {
    let name = form.name.trim();
    let mut parts = Vec::new();
    if !name.is_empty() {
        parts.push(name);
    }
    if form.environment != Environment::None {
        parts.push(form.environment.label(false));
    }
    if parts.is_empty() {
        parts.push(form.driver.label());
    }
    parts.join(" · ")
}

/// macOS: the environment's stripe, the title over the connection's name,
/// the Parameters and URL tabs, and Close.
fn header(ui: &mut Ui, form: &mut ConnectionForm, skin: &Skin, actions: &mut Vec<Action>) {
    let Skin { look, palette, .. } = *skin;
    let title = Text::one(
        look,
        TextRole::DialogTitle,
        &skin.say(title(form)),
        palette.text,
    )
    .layout(ui.ctx());
    let subtitle = Text::one(look, TextRole::UiBody, &subtitle(form), palette.dim).layout(ui.ctx());
    // 16 above, the two lines 2 apart, 12 below.
    let block = title.height() + 2.0 + subtitle.height();
    let height = STRIPE + 16.0 + block + 12.0;
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), height), Sense::hover());
    ui.painter().add(egui::Shape::convex_polygon(
        top_cap(rect, f32::from(skin.inner_radius()), STRIPE),
        skin.env.color,
        Stroke::NONE,
    ));
    let top = rect.top() + STRIPE + 16.0;
    let left = rect.left() + 20.0;
    for (text, y) in [(&title, top), (&subtitle, top + title.height() + 2.0)] {
        text.paint(ui.painter(), pos2(left, y));
        widgets::announce(
            ui,
            Rect::from_min_size(pos2(left, y), text.size()),
            text.galley.text(),
        );
    }
    // Close at the right, the tabs 12 before it, on the block's centre.
    let controls = Rect::from_min_max(
        pos2(left + title.width().max(subtitle.width()) + 12.0, top),
        pos2(rect.right() - 20.0, top + block),
    );
    let mut child = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(controls)
            .layout(egui::Layout::right_to_left(egui::Align::Center)),
    );
    child.spacing_mut().item_spacing.x = 12.0;
    let close = gettext(skin.locale, "Close");
    if widgets::icon_button_sized(
        &mut child,
        Icon::X,
        &close,
        vec2(30.0, 30.0),
        14.0,
        look,
        palette,
    )
    .clicked()
    {
        actions.push(Action::CloseDialog);
    }
    let parameters = gettext(skin.locale, "Parameters");
    let url = gettext(skin.locale, "URL");
    let tabs = [Choice::plain(&parameters), Choice::plain(&url)];
    let chosen = usize::from(form.url_mode);
    if let Some(index) = radio_group(&mut child, "input", &tabs, chosen, Group::Track(26.0), skin) {
        show_url(ui.ctx(), form, index == 1);
    }
}

/// The terminal look: one tinted line with the title, the connection, and
/// the key that shows the URL field.
fn terminal_header(ui: &mut Ui, form: &mut ConnectionForm, skin: &Skin) {
    let Skin { look, palette, .. } = *skin;
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 41.0), Sense::hover());
    let fill = if skin.environment == Environment::None {
        palette.panel
    } else {
        theme::mix(palette.panel, skin.env.color, 0.16)
    };
    let radius = skin.inner_radius();
    ui.painter().rect_filled(
        rect,
        CornerRadius {
            nw: radius,
            ne: radius,
            sw: 0,
            se: 0,
        },
        fill,
    );
    widgets::hline(ui, rect.x_range(), rect.bottom() - 0.5, palette.outline);
    let y = rect.top() + 20.0;
    let mut x = rect.left() + 14.0;
    x += widgets::paint_label(
        ui,
        x,
        y,
        Text::one(
            look,
            TextRole::OScreenTitle,
            &skin.say(title(form)),
            palette.text,
        ),
    ) + 12.0;
    widgets::paint_label(
        ui,
        x,
        y,
        Text::one(look, TextRole::OBody, &subtitle(form), palette.dim),
    );
    let label = skin.say("Paste URL");
    let hint = [("u", label.as_str(), true)];
    let width = widgets::key_hints_width(ui, &hint, 0.0, look, palette);
    let place = Rect::from_min_size(
        pos2(rect.right() - 14.0 - width, rect.top()),
        vec2(width, 40.0),
    );
    widgets::key_hints(ui, (place.left(), y), &hint, 0.0, look, palette);
    if ButtonSpec::new("Paste URL").hidden_at(ui, place).clicked() {
        show_url(ui.ctx(), form, !form.url_mode);
    }
}

/// One choice of a [`radio_group`].
#[derive(Clone, Copy)]
struct Choice<'a> {
    label: &'a str,
    /// An environment's colours, when the choice is one.
    tint: Option<EnvColors>,
}

impl<'a> Choice<'a> {
    fn plain(label: &'a str) -> Self {
        Self { label, tint: None }
    }
}

/// How a [`radio_group`] draws.
#[derive(Clone, Copy, PartialEq)]
enum Group {
    /// macOS: a sunken track, the chosen segment raised off it. Segments
    /// are this tall.
    Track(f32),
    /// The terminal look's environments: bordered buttons in each one's
    /// colour, the chosen one filled.
    Buttons,
    /// The terminal look's other choices: words in a line, the chosen one
    /// boxed in the accent.
    Words,
}

/// The dot before a chosen environment, and the gap after it.
const DOT: f32 = 8.0;
const DOT_GAP: f32 = 6.0;

/// Choices of which one is chosen; returns the one clicked. Each is
/// announced as a radio button by its label.
fn radio_group(
    ui: &mut Ui,
    salt: &str,
    choices: &[Choice<'_>],
    chosen: usize,
    group: Group,
    skin: &Skin,
) -> Option<usize> {
    let Skin { look, palette, .. } = *skin;
    let idle_role = widgets::body(look);
    let chosen_role = TextRole::pick(look, TextRole::UiBodyStrong, TextRole::OGroup);
    let (pad, gap, inset, height) = match group {
        Group::Track(height) => (12.0, 0.0, 3.0, height),
        Group::Buttons => (10.0, 6.0, 0.0, 26.0),
        Group::Words => (8.0, 6.0, 0.0, 22.0),
    };
    let role = |index: usize| {
        if index == chosen && group != Group::Words {
            chosen_role
        } else {
            idle_role
        }
    };
    let dotted =
        |index: usize| index == chosen && group != Group::Words && choices[index].tint.is_some();
    let widths: Vec<f32> = choices
        .iter()
        .enumerate()
        .map(|(index, choice)| {
            let text = role(index).width(ui.ctx(), look.faces, choice.label);
            let dot = if dotted(index) { DOT + DOT_GAP } else { 0.0 };
            (text + dot + 2.0 * pad).ceil()
        })
        .collect();
    let size = vec2(
        widths.iter().sum::<f32>() + gap * choices.len().saturating_sub(1) as f32 + 2.0 * inset,
        height + 2.0 * inset,
    );
    let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
    let enabled = ui.is_enabled();
    let painter = ui.painter().clone();
    if let Group::Track(_) = group {
        painter.rect_filled(rect, CornerRadius::same(look.radius), palette.surface);
    }
    let corner = CornerRadius::same(match group {
        Group::Track(_) => look.radius.saturating_sub(2),
        Group::Buttons | Group::Words => 3,
    });
    let mut clicked = None;
    let mut left = rect.left() + inset;
    for (index, choice) in choices.iter().enumerate() {
        let cell = Rect::from_min_size(pos2(left, rect.top() + inset), vec2(widths[index], height));
        left += widths[index] + gap;
        let active = index == chosen;
        let response = ui.interact(cell, ui.id().with((salt, index)), Sense::click());
        response.widget_info(|| {
            WidgetInfo::selected(WidgetType::RadioButton, enabled, active, choice.label)
        });
        if response.clicked() {
            clicked = Some(index);
        }
        let hovered = response.hovered() && enabled;
        let tint = choice.tint.as_ref();
        let color = match group {
            Group::Track(_) => {
                if active {
                    painter.add(
                        egui::epaint::Shadow {
                            offset: [0, 1],
                            blur: 2,
                            spread: 0,
                            color: Color32::from_black_alpha(20),
                        }
                        .as_shape(cell, corner),
                    );
                    painter.rect_filled(cell, corner, widgets::raised_fill(palette));
                    tint.map_or(palette.text, |colors| skin.env_ink(colors))
                } else if hovered {
                    palette.text
                } else {
                    palette.secondary
                }
            }
            Group::Buttons => {
                let ink = tint.map_or(palette.text, |colors| colors.color);
                if active {
                    painter.rect_filled(cell, corner, ink);
                    palette.panel
                } else {
                    if hovered {
                        painter.rect_filled(cell, corner, palette.text.gamma_multiply(0.08));
                    }
                    painter.rect_stroke(
                        cell,
                        corner,
                        Stroke::new(1.0, palette.outline),
                        StrokeKind::Inside,
                    );
                    ink
                }
            }
            Group::Words => {
                if active {
                    painter.rect_stroke(
                        cell,
                        corner,
                        Stroke::new(1.0, palette.accent),
                        StrokeKind::Inside,
                    );
                    palette.accent
                } else if hovered {
                    palette.text
                } else {
                    palette.dim
                }
            }
        };
        if response.has_focus() {
            painter.rect_stroke(
                cell.expand(1.0),
                corner,
                widgets::primary_focus_ring(palette),
                StrokeKind::Outside,
            );
        }
        let mut x = cell.left() + pad;
        if dotted(index) {
            let dot = match (group, tint) {
                (Group::Track(_), Some(colors)) => colors.color,
                _ => color,
            };
            painter.circle_filled(pos2(x + DOT / 2.0, cell.center().y), DOT / 2.0, dot);
            x += DOT + DOT_GAP;
        }
        Text::one(look, role(index), choice.label, color)
            .layout(ui.ctx())
            .paint_left(&painter, x, cell.center().y);
    }
    clicked
}

/// The driver choice: SQLite, PostgreSQL or MySQL.
fn driver_choice(ui: &mut Ui, form: &ConnectionForm, skin: &Skin, actions: &mut Vec<Action>) {
    let labels = DRIVERS.map(|driver| skin.look.label(driver.label()));
    let choices: Vec<Choice<'_>> = labels.iter().map(|label| Choice::plain(label)).collect();
    let chosen = DRIVERS
        .iter()
        .position(|driver| *driver == form.driver)
        .unwrap_or(0);
    let group = if skin.look.terminal {
        Group::Words
    } else {
        Group::Track(28.0)
    };
    if let Some(index) = radio_group(ui, "driver", &choices, chosen, group, skin)
        && DRIVERS[index] != form.driver
    {
        actions.push(Action::SetDriver(DRIVERS[index]));
    }
}

/// The environment choice. It is what the badges say (PROD), and it gives
/// the connection its colour.
fn environment_choice(ui: &mut Ui, form: &mut ConnectionForm, skin: &Skin) {
    let labels = Environment::ALL.map(|environment| skin.say(environment.name()));
    let choices: Vec<Choice<'_>> = Environment::ALL
        .iter()
        .zip(&labels)
        .map(|(environment, label)| Choice {
            label,
            tint: Some(theme::env_colors(*environment, skin.palette)),
        })
        .collect();
    let chosen = Environment::ALL
        .iter()
        .position(|environment| *environment == form.environment)
        .unwrap_or(0);
    let group = if skin.look.terminal {
        Group::Buttons
    } else {
        Group::Track(28.0)
    };
    if let Some(index) = radio_group(ui, "environment", &choices, chosen, group, skin) {
        form.environment = Environment::ALL[index];
        form.color = form.environment.color();
    }
}

/// A column of a [`row`]: a share of the free width, or a fixed width.
#[derive(Clone, Copy)]
enum Track {
    Fr(f32),
    Px(f32),
}

/// The widths `tracks` take in `total`, `gap` apart.
fn track_widths(total: f32, gap: f32, tracks: &[Track]) -> Vec<f32> {
    let fixed: f32 = tracks
        .iter()
        .map(|track| match track {
            Track::Px(width) => *width,
            Track::Fr(_) => 0.0,
        })
        .sum();
    let shares: f32 = tracks
        .iter()
        .map(|track| match track {
            Track::Fr(share) => *share,
            Track::Px(_) => 0.0,
        })
        .sum();
    let free = (total - fixed - gap * tracks.len().saturating_sub(1) as f32).max(0.0);
    tracks
        .iter()
        .map(|track| match track {
            Track::Px(width) => *width,
            Track::Fr(share) => (free * share / shares.max(f32::EPSILON)).floor(),
        })
        .collect()
}

/// Cells side by side across the available width, their tops in line.
fn row(ui: &mut Ui, gap: f32, tracks: &[Track], mut cell: impl FnMut(&mut Ui, usize)) {
    let widths = track_widths(ui.available_width(), gap, tracks);
    ui.horizontal_top(|ui| {
        ui.spacing_mut().item_spacing.x = gap;
        for (index, width) in widths.into_iter().enumerate() {
            ui.allocate_ui_with_layout(
                vec2(width, 0.0),
                egui::Layout::top_down(egui::Align::Min),
                |ui| {
                    ui.set_width(width);
                    cell(ui, index);
                },
            );
        }
    });
}

/// A field filling its cell, `height` tall.
fn input<'t>(
    ui: &Ui,
    text: &'t mut String,
    role: TextRole,
    height: f32,
    skin: &Skin,
) -> egui::TextEdit<'t> {
    let pad = if skin.look.terminal { 8 } else { 10 };
    widgets::field(ui, text, skin.look, role, height, pad).desired_width(f32::INFINITY)
}

/// A field's placeholder, in the field's role.
fn hint(ui: &Ui, text: &str, role: TextRole, skin: &Skin) -> std::sync::Arc<egui::Galley> {
    Text::one(skin.look, role, text, skin.palette.faint)
        .layout(ui.ctx())
        .galley
}

/// macOS: a label over what `add` draws, 5 apart, naming it.
fn labelled(
    ui: &mut Ui,
    text: &str,
    role: TextRole,
    skin: &Skin,
    add: impl FnOnce(&mut Ui) -> Response,
) -> Response {
    let label = widgets::label(ui, role, text, skin.palette.secondary, skin.look);
    ui.add_space(5.0);
    add(ui).labelled_by(label.id)
}

/// A field and the button that picks a file for it, 6 apart (8 in the
/// terminal look). Returns the field, and whether the button was clicked.
fn field_with_button(
    ui: &mut Ui,
    button: &str,
    name: &'static str,
    skin: &Skin,
    field: impl FnOnce(&mut Ui) -> Response,
) -> (Response, bool) {
    let Skin { look, palette, .. } = *skin;
    let height = skin.field_height();
    let name = gettext(skin.locale, name);
    let spec = || {
        ButtonSpec::new(button)
            .label(&name)
            .padding(10.0)
            .radius(if look.terminal {
                3
            } else {
                look.radius.saturating_sub(1)
            })
    };
    let gap = if look.terminal { 8.0 } else { 6.0 };
    let width = spec().width(ui, look);
    let mut field = Some(field);
    let mut output = None;
    let mut clicked = false;
    row(
        ui,
        gap,
        &[Track::Fr(1.0), Track::Px(width)],
        |ui, column| {
            if column == 0 {
                if let Some(field) = field.take() {
                    output = Some(field(ui));
                }
            } else {
                clicked = spec().show(ui, height, look, palette).clicked();
            }
        },
    );
    let response = output.unwrap_or_else(|| ui.response());
    (response, clicked)
}

/// The password's mode as a check box: saved in the keyring, or asked for
/// on every connect.
fn keeps(mode: PasswordMode) -> bool {
    mode != PasswordMode::Ask
}

fn set_keeps(mode: &mut PasswordMode, keep: bool) {
    *mode = if keep {
        PasswordMode::Keyring
    } else {
        PasswordMode::Ask
    };
}

/// macOS: the "Keychain" check box beside a secret, on the field's line.
fn keyring_box(ui: &mut Ui, mode: &mut PasswordMode, labelled: bool, skin: &Skin) {
    let Skin { look, palette, .. } = *skin;
    if labelled {
        let line = TextRole::Secondary.row_height(ui.ctx(), look.faces);
        ui.add_space(line + 5.0);
    }
    ui.allocate_ui_with_layout(
        vec2(ui.available_width(), skin.field_height()),
        egui::Layout::left_to_right(egui::Align::Center),
        |ui| {
            let mut keep = keeps(*mode);
            let name = gettext(skin.locale, skin.keyring());
            let hover = gettext(
                skin.locale,
                "Saved for the next connect. Off, it is asked for every time.",
            );
            if widgets::checkbox(ui, &mut keep, &name, look, palette)
                .on_hover_text(hover)
                .changed()
            {
                set_keeps(mode, keep);
            }
        },
    );
}

/// macOS: a group of fields in a hairline box, its heading set into the
/// top edge.
fn fieldset(ui: &mut Ui, legend: &str, skin: &Skin, add: impl FnOnce(&mut Ui)) {
    let Skin { look, palette, .. } = *skin;
    let legend = Text::one(look, TextRole::Legend, legend, palette.secondary).layout(ui.ctx());
    let half = legend.height() / 2.0;
    ui.add_space(half);
    let border = ui.painter().add(egui::Shape::Noop);
    let rect = egui::Frame::new()
        .inner_margin(egui::Margin {
            left: 15,
            right: 15,
            // The heading's lower half, then the group's 14.
            top: (half + 15.0).round() as i8,
            bottom: 13,
        })
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            add(ui);
        })
        .response
        .rect;
    ui.painter().set(
        border,
        egui::epaint::RectShape::stroke(
            rect,
            CornerRadius::same(10),
            Stroke::new(widgets::hairline(ui), skin.rule()),
            StrokeKind::Inside,
        ),
    );
    // The heading breaks the edge: 6 of the dialog's colour at each side.
    let gap = Rect::from_min_size(
        pos2(rect.left() + 15.0, rect.top() - half),
        vec2(legend.width() + 12.0, legend.height()),
    );
    ui.painter().rect_filled(gap, CornerRadius::ZERO, skin.fill);
    legend.paint(ui.painter(), pos2(gap.left() + 6.0, gap.top()));
    widgets::announce(ui, gap, legend.galley.text());
}

/// macOS: the dialog's body, between the header and the footer.
fn body(
    ui: &mut Ui,
    form: &mut ConnectionForm,
    skin: &Skin,
    focus_name: bool,
    actions: &mut Vec<Action>,
) {
    egui::Frame::new()
        .inner_margin(egui::Margin {
            left: 20,
            right: 20,
            top: 4,
            bottom: 18,
        })
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            if form.url_mode {
                url_pane(ui, form, skin, actions);
            } else {
                identity(ui, form, skin, focus_name, actions);
                ui.add_space(18.0);
                if form.driver == Driver::Sqlite {
                    fieldset(ui, &skin.say("Database"), skin, |ui| {
                        file_field(ui, form, skin, actions);
                    });
                } else {
                    fieldset(ui, &skin.say("Server"), skin, |ui| {
                        server(ui, form, skin);
                    });
                    ui.add_space(18.0);
                    fieldset(ui, &skin.say("Security"), skin, |ui| {
                        security(ui, form, skin, actions);
                    });
                }
                ui.add_space(18.0);
                safety(ui, skin);
            }
            messages(ui, form, skin, actions);
        });
}

/// The name beside the kind of database, then the environment.
fn identity(
    ui: &mut Ui,
    form: &mut ConnectionForm,
    skin: &Skin,
    focus_name: bool,
    actions: &mut Vec<Action>,
) {
    let Skin { look, palette, .. } = *skin;
    row(ui, 12.0, &[Track::Fr(1.4), Track::Fr(1.0)], |ui, column| {
        if column == 0 {
            let name = labelled(ui, &skin.say("Name"), TextRole::FormLabel, skin, |ui| {
                ui.add(input(ui, &mut form.name, TextRole::UiBody, 34.0, skin))
            });
            if focus_name {
                name.request_focus();
            }
        } else {
            widgets::label(
                ui,
                TextRole::FormLabel,
                &skin.say("Type"),
                palette.secondary,
                look,
            );
            ui.add_space(5.0);
            driver_choice(ui, form, skin, actions);
        }
    });
    ui.add_space(18.0);
    widgets::label(
        ui,
        TextRole::FormLabel,
        &skin.say("Environment"),
        palette.secondary,
        look,
    );
    ui.add_space(6.0);
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 12.0;
        environment_choice(ui, form, skin);
        widgets::label(
            ui,
            TextRole::Secondary,
            &skin.say("Sets the connection color"),
            palette.dim,
            look,
        );
    });
}

/// SQLite: the file, and the button that picks one.
fn file_field(ui: &mut Ui, form: &mut ConnectionForm, skin: &Skin, actions: &mut Vec<Action>) {
    let role = skin.value_role();
    let height = skin.field_height();
    let choose = skin.say("Choose…");
    let mut pick = |ui: &mut Ui, label: Option<egui::Id>| {
        let (field, clicked) =
            field_with_button(ui, &choose, "Choose a database file", skin, |ui| {
                ui.add(input(ui, &mut form.sqlite_path, role, height, skin))
            });
        if clicked {
            actions.push(Action::PickSqliteFile);
        }
        match label {
            Some(label) => field.labelled_by(label),
            None => field,
        }
    };
    if skin.look.terminal {
        terminal_row(ui, &skin.say("File"), skin, |ui, label| {
            pick(ui, Some(label));
        });
    } else {
        let label = widgets::label(
            ui,
            TextRole::Secondary,
            &skin.say("File"),
            skin.palette.secondary,
            skin.look,
        );
        ui.add_space(5.0);
        pick(ui, Some(label.id));
    }
}

/// macOS: where the server is and who logs in.
fn server(ui: &mut Ui, form: &mut ConnectionForm, skin: &Skin) {
    let role = skin.value_role();
    let small = TextRole::Secondary;
    let height = skin.field_height();
    let columns = [Track::Fr(1.0), Track::Px(96.0)];
    row(ui, 12.0, &columns, |ui, column| {
        if column == 0 {
            labelled(ui, &skin.say("Host"), small, skin, |ui| {
                ui.add(input(ui, &mut form.host, role, height, skin))
            });
        } else {
            labelled(ui, &skin.say("Port"), small, skin, |ui| {
                ui.add(input(ui, &mut form.port, role, height, skin))
            });
        }
    });
    ui.add_space(10.0);
    let same = skin.say("Same as the user");
    labelled(ui, &skin.say("Database"), small, skin, |ui| {
        let hint = hint(ui, &same, role, skin);
        ui.add(input(ui, &mut form.database, role, height, skin).hint_text(hint))
    });
    ui.add_space(10.0);
    row(ui, 12.0, &columns, |ui, column| {
        if column == 0 {
            labelled(ui, &skin.say("User"), small, skin, |ui| {
                ui.add(input(ui, &mut form.user, role, height, skin))
            });
        }
    });
    ui.add_space(10.0);
    let saved = if form.password_is_saved() {
        skin.say(skin.saved_hint())
    } else {
        String::new()
    };
    row(ui, 12.0, &columns, |ui, column| {
        if column == 0 {
            labelled(ui, &skin.say("Password"), small, skin, |ui| {
                let hint = hint(ui, &saved, TextRole::UiBody, skin);
                ui.add(
                    input(ui, &mut form.password, TextRole::UiBody, height, skin)
                        .password(true)
                        .hint_text(hint),
                )
            });
        } else {
            keyring_box(ui, &mut form.password_mode, true, skin);
        }
    });
}

/// Whether the TLS mode checks the server against a CA file. `require`
/// uses one too (as verify-ca, like libpq).
fn uses_ca_file(mode: TlsMode) -> bool {
    matches!(
        mode,
        TlsMode::Require | TlsMode::VerifyCa | TlsMode::VerifyFull
    )
}

/// What an empty CA file means in each mode. verify-ca refuses the system
/// roots: they vouch for any public certificate.
fn ca_hint(mode: TlsMode) -> &'static str {
    match mode {
        TlsMode::Require => "None (not checked)",
        TlsMode::VerifyCa => "Required",
        TlsMode::VerifyFull => "System certificates",
        TlsMode::Disable | TlsMode::Prefer => "Not used",
    }
}

/// The CA certificate's field and its button, greyed out in the modes that
/// check nothing.
fn ca_field(
    ui: &mut Ui,
    form: &mut ConnectionForm,
    label: egui::Id,
    skin: &Skin,
    actions: &mut Vec<Action>,
) {
    let role = TextRole::pick(skin.look, TextRole::MonoSecondary, TextRole::OBody);
    let height = skin.field_height();
    let placeholder = skin.say(ca_hint(form.tls));
    ui.add_enabled_ui(uses_ca_file(form.tls), |ui| {
        let (field, clicked) = field_with_button(
            ui,
            &skin.say("Choose…"),
            "Choose a CA certificate",
            skin,
            |ui| {
                let hint = hint(ui, &placeholder, role, skin);
                ui.add(input(ui, &mut form.ca_file, role, height, skin).hint_text(hint))
            },
        );
        field.labelled_by(label);
        if clicked {
            actions.push(Action::PickCaFile);
        }
    });
}

/// The warning under the TLS mode when it leaves the password readable.
fn intercept_warning(ui: &mut Ui, skin: &Skin) {
    let role = widgets::secondary(skin.look);
    Text::one(
        skin.look,
        role,
        &gettext(skin.locale, INTERCEPT_WARNING),
        skin.palette.warning,
    )
    .wrap(ui.available_width())
    .layout(ui.ctx())
    .label(ui);
}

/// macOS: TLS, then the SSH tunnel.
fn security(ui: &mut Ui, form: &mut ConnectionForm, skin: &Skin, actions: &mut Vec<Action>) {
    let Skin { look, palette, .. } = *skin;
    let small = TextRole::Secondary;
    row(ui, 12.0, &[Track::Fr(1.0), Track::Fr(1.0)], |ui, column| {
        if column == 0 {
            let width = ui.available_width();
            labelled(ui, &skin.say("SSL mode"), small, skin, |ui| {
                widgets::popup_button(
                    ui,
                    egui::ComboBox::from_id_salt("tls-mode")
                        .width(width)
                        .selected_text(tls_label(form.tls)),
                    look,
                    palette,
                    |ui| {
                        for (mode, label) in TLS_MODES {
                            ui.selectable_value(&mut form.tls, mode, label);
                        }
                    },
                )
                .response
            });
        } else {
            let label = widgets::label(
                ui,
                small,
                &skin.say("CA certificate"),
                palette.secondary,
                look,
            );
            ui.add_space(5.0);
            ca_field(ui, form, label.id, skin, actions);
        }
    });
    if form.password_can_be_intercepted() {
        ui.add_space(10.0);
        intercept_warning(ui, skin);
    }
    ui.add_space(12.0);
    widgets::checkbox(
        ui,
        &mut form.ssh,
        &skin.say("Connect through SSH tunnel"),
        look,
        palette,
    );
    if !form.ssh {
        return;
    }
    ui.add_space(10.0);
    // The tunnel's fields sit under the box's label, 22 in.
    egui::Frame::new()
        .inner_margin(egui::Margin {
            left: 22,
            ..egui::Margin::ZERO
        })
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ssh_fields(ui, form, skin, actions);
        });
}

/// What the SSH secret is, for the chosen login.
fn ssh_secret_name(auth: SshAuthKind) -> &'static str {
    if auth == SshAuthKind::Password {
        "SSH password"
    } else {
        "Passphrase"
    }
}

/// macOS: the tunnel's host, port, user and login in one line, named by
/// their placeholders, then what the login needs.
fn ssh_fields(ui: &mut Ui, form: &mut ConnectionForm, skin: &Skin, actions: &mut Vec<Action>) {
    let Skin { look, palette, .. } = *skin;
    let role = TextRole::MonoSecondary;
    let height = skin.field_height();
    let named = |response: Response, name: &'static str| {
        let name = gettext(skin.locale, name);
        response.widget_info(|| WidgetInfo::labeled(WidgetType::TextEdit, true, &*name));
        response
    };
    let columns = [
        Track::Fr(1.2),
        Track::Px(64.0),
        Track::Fr(0.8),
        Track::Fr(1.0),
    ];
    row(ui, 10.0, &columns, |ui, column| match column {
        0 => {
            let hint = hint(ui, &skin.say("SSH host"), role, skin);
            let field = ui.add(input(ui, &mut form.ssh_host, role, height, skin).hint_text(hint));
            named(field, "SSH host");
        }
        1 => {
            let hint = hint(ui, &skin.say("Port"), role, skin);
            let field = ui.add(input(ui, &mut form.ssh_port, role, height, skin).hint_text(hint));
            named(field, "SSH port");
        }
        2 => {
            let hint = hint(ui, &skin.say("User"), role, skin);
            let field = ui.add(input(ui, &mut form.ssh_user, role, height, skin).hint_text(hint));
            named(field, "SSH user");
        }
        _ => {
            let width = ui.available_width();
            let name = gettext(skin.locale, "Authentication");
            let value = gettext(skin.locale, form.ssh_auth.label());
            let response = widgets::popup_button(
                ui,
                egui::ComboBox::from_id_salt("ssh-auth")
                    .width(width)
                    .selected_text(&*value),
                look,
                palette,
                |ui| {
                    for kind in SshAuthKind::ALL {
                        ui.selectable_value(
                            &mut form.ssh_auth,
                            kind,
                            gettext(skin.locale, kind.label()),
                        );
                    }
                },
            )
            .response;
            response.widget_info(|| {
                let mut info = WidgetInfo::labeled(WidgetType::ComboBox, true, &*name);
                info.current_text_value = Some(value.to_string());
                info
            });
        }
    });
    if form.ssh_auth == SshAuthKind::KeyFile {
        ui.add_space(10.0);
        let (field, clicked) =
            field_with_button(ui, &skin.say("Choose…"), "Choose a key file", skin, |ui| {
                let hint = hint(ui, &skin.say("Key file"), role, skin);
                ui.add(input(ui, &mut form.ssh_key_file, role, height, skin).hint_text(hint))
            });
        named(field, "Key file");
        if clicked {
            actions.push(Action::PickKeyFile);
        }
    }
    if form.ssh_auth != SshAuthKind::Agent {
        ui.add_space(10.0);
        let name = ssh_secret_name(form.ssh_auth);
        let placeholder = if form.ssh_secret_is_saved() {
            format!("{} · {}", skin.say(name), skin.say(skin.saved_hint()))
        } else {
            skin.say(name)
        };
        row(
            ui,
            12.0,
            &[Track::Fr(1.0), Track::Px(96.0)],
            |ui, column| {
                if column == 0 {
                    let hint = hint(ui, &placeholder, TextRole::UiBody, skin);
                    let field = ui.add(
                        input(ui, &mut form.ssh_secret, TextRole::UiBody, height, skin)
                            .password(true)
                            .hint_text(hint),
                    );
                    named(field, name);
                } else {
                    keyring_box(ui, &mut form.ssh_secret_mode, false, skin);
                }
            },
        );
    }
}

/// A 14 pt check mark that cannot be cleared: what always holds.
fn locked_check(ui: &mut Ui, name: &str, skin: &Skin) {
    // Three below the line's top, as the design sets the box.
    let (rect, response) = ui.allocate_exact_size(vec2(14.0, 17.0), Sense::hover());
    response.widget_info(|| WidgetInfo::selected(WidgetType::Checkbox, false, true, name));
    let mark = Rect::from_min_size(rect.min + vec2(0.0, 3.0), vec2(14.0, 14.0));
    let painter = ui.painter();
    painter.rect_filled(
        mark,
        CornerRadius::same(skin.look.radius.min(4)),
        skin.env.color,
    );
    painter.line(
        vec![
            mark.min + vec2(3.2, 7.4),
            mark.min + vec2(5.8, 10.0),
            mark.min + vec2(10.8, 4.4),
        ],
        Stroke::new(1.6, skin.fill),
    );
}

/// macOS: the read-only promise, tinted with the environment's colour.
/// Every session is read-only in this version, so the box is locked.
fn safety(ui: &mut Ui, skin: &Skin) {
    let Skin { look, palette, .. } = *skin;
    let amount = if palette.dark { 0.16 } else { 0.1 };
    let title = skin.say("Open read-only");
    egui::Frame::new()
        .fill(theme::mix(skin.fill, skin.env.color, amount))
        .corner_radius(CornerRadius::same(10))
        .inner_margin(egui::Margin::symmetric(14, 12))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal_top(|ui| {
                ui.spacing_mut().item_spacing.x = 10.0;
                locked_check(ui, &title, skin);
                ui.vertical(|ui| {
                    widgets::label(ui, TextRole::UiBodyStrong, &title, palette.text, look);
                    ui.add_space(2.0);
                    widgets::label(
                        ui,
                        TextRole::Secondary,
                        &skin.say("Blocks every write from this app. Always on in this version."),
                        palette.secondary,
                        look,
                    );
                });
            });
        });
}

/// macOS: the URL tab, one field that fills the parameters.
fn url_pane(ui: &mut Ui, form: &mut ConnectionForm, skin: &Skin, actions: &mut Vec<Action>) {
    let Skin { look, palette, .. } = *skin;
    let label = widgets::label(
        ui,
        TextRole::FormLabel,
        &skin.say("Connection URL"),
        palette.secondary,
        look,
    );
    ui.add_space(5.0);
    let (field, clicked) = field_with_button(ui, &skin.say("Fill"), "Fill", skin, |ui| {
        url_field(ui, form, skin)
    });
    let field = field.labelled_by(label.id);
    let entered = field.lost_focus() && ui.input(|input| input.key_pressed(egui::Key::Enter));
    if clicked || entered {
        actions.push(Action::ApplyUrl);
    }
    ui.add_space(6.0);
    widgets::label(
        ui,
        TextRole::Secondary,
        &skin.say("Fills the parameters. A password in the URL moves to the password field."),
        palette.dim,
        look,
    );
}

/// The URL field, with the chosen driver's URL as its placeholder. It
/// takes the keyboard on the frame it appears.
fn url_field(ui: &mut Ui, form: &mut ConnectionForm, skin: &Skin) -> Response {
    let role = skin.value_role();
    let example = match form.driver {
        Driver::Sqlite => "sqlite:///path/to/file.db",
        Driver::MySql => "mysql://user@host/db",
        Driver::Postgres => "postgres://user@host/db",
    };
    let hint = hint(ui, example, role, skin);
    let field = ui.add(input(ui, &mut form.url, role, skin.field_height(), skin).hint_text(hint));
    let focus = ui
        .ctx()
        .data_mut(|data| data.remove_temp::<bool>(focus_url_id()))
        .unwrap_or(false);
    if focus {
        field.request_focus();
    }
    field
}

/// What went wrong, under the fields: a field that does not describe a
/// connection, a failed Test, or an SSH host key to trust first.
fn messages(ui: &mut Ui, form: &ConnectionForm, skin: &Skin, actions: &mut Vec<Action>) {
    let Skin { look, palette, .. } = *skin;
    let gap = if look.terminal { 8.0 } else { 18.0 };
    let notice = |ui: &mut Ui, text: &str, color: Color32| {
        ui.add_space(gap);
        let role = widgets::body(look);
        if look.terminal {
            Text::one(look, role, text, color)
                .wrap(ui.available_width())
                .layout(ui.ctx())
                .label(ui);
        } else {
            egui::Frame::new()
                .fill(theme::mix(skin.fill, color, 0.08))
                .corner_radius(CornerRadius::same(10))
                .inner_margin(egui::Margin::symmetric(14, 10))
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    Text::one(look, role, text, color)
                        .wrap(ui.available_width())
                        .layout(ui.ctx())
                        .label(ui);
                });
        }
    };
    if let Some(message) = &form.message {
        notice(ui, message, palette.danger);
    }
    match &form.test {
        TestState::Idle | TestState::Running(_) | TestState::Passed => {}
        TestState::Failed(message) => notice(ui, message, palette.danger),
        TestState::Untrusted {
            host, fingerprint, ..
        } => {
            let text = format!(
                "{} {host}: {fingerprint}",
                gettext(skin.locale, "Unknown SSH host key for")
            );
            notice(ui, &text, palette.warning);
            ui.add_space(8.0);
            let trust = skin.say("Trust and test");
            if ButtonSpec::new(&trust)
                .label(&gettext(skin.locale, "Trust and test"))
                .show(ui, skin.field_height(), look, palette)
                .clicked()
            {
                actions.push(Action::TrustTestHostKey);
            }
        }
    }
}

/// What the footer says about the last Test: that it is running, or what
/// it reached and how long that took.
enum Status {
    Running(String),
    Passed(String),
}

fn status(form: &ConnectionForm, skin: &Skin) -> Option<Status> {
    match form.test {
        TestState::Running(_) => Some(Status::Running(skin.say("Testing…"))),
        TestState::Passed => {
            let driver = form
                .test_spec
                .as_ref()
                .map_or(form.driver, |spec| spec.driver);
            let mut text = format!("{} · {}", gettext(skin.locale, "Connected"), driver.label());
            if let Some(took) = form.test_took {
                text = format!("{text} · {}", crate::ui::format::elapsed(took));
            }
            Some(Status::Passed(skin.look.label(&text)))
        }
        TestState::Idle | TestState::Failed(_) | TestState::Untrusted { .. } => None,
    }
}

/// Paints the Test's status with its right edge at `right` (or its left at
/// `left`), centred on `y`.
fn paint_status(ui: &mut Ui, form: &ConnectionForm, x: Result<f32, f32>, y: f32, skin: &Skin) {
    let Skin { look, palette, .. } = *skin;
    let Some(status) = status(form, skin) else {
        return;
    };
    let role = widgets::secondary(look);
    let (text, color) = match &status {
        Status::Running(text) => (text, palette.dim),
        Status::Passed(text) => (text, palette.success),
    };
    let laid = Text::one(look, role, text, color).layout(ui.ctx());
    let mark = 12.0;
    let left = match x {
        Ok(left) => left,
        Err(right) => right - laid.width() - mark - 6.0,
    };
    let place = Rect::from_center_size(pos2(left + mark / 2.0, y), vec2(mark, mark));
    match status {
        Status::Running(_) => {
            ui.put(place, egui::Spinner::new().size(mark).color(color));
        }
        Status::Passed(_) => Icon::CircleCheck.image(color, mark).paint_at(ui, place),
    }
    let x = left + mark + 6.0;
    laid.paint_left(ui.painter(), x, y);
    widgets::announce(
        ui,
        Rect::from_min_size(pos2(x, y - laid.height() / 2.0), laid.size()),
        laid.galley.text(),
    );
}

/// macOS: Delete at the left; the Test's status, Test, Cancel and the two
/// ways to save at the right. Saving is the main thing an edit does, and
/// connecting the main thing a new connection does.
fn footer(ui: &mut Ui, form: &ConnectionForm, skin: &Skin, actions: &mut Vec<Action>) {
    let Skin { look, palette, .. } = *skin;
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 61.0), Sense::hover());
    let radius = skin.inner_radius();
    ui.painter().rect_filled(
        rect,
        CornerRadius {
            nw: 0,
            ne: 0,
            sw: radius,
            se: radius,
        },
        skin.footer(),
    );
    widgets::hline(ui, rect.x_range(), rect.top() + 0.5, skin.rule());
    let y = rect.top() + 1.0 + 30.0;
    let editing = form.editing.is_some();
    let save = gettext(skin.locale, "Save");
    let connect = gettext(skin.locale, "Save & Connect");
    let test = gettext(skin.locale, "Test");
    let cancel = gettext(skin.locale, "Cancel");
    let delete = gettext(skin.locale, "Delete");
    let button = |text, primary: bool| {
        let spec = ButtonSpec::new(text);
        if primary {
            spec.primary().padding(16.0)
        } else {
            spec
        }
    };
    // Right to left, 10 apart: the primary, the other way to save, Cancel
    // and Test.
    let (first, second) = if editing {
        (&save, &connect)
    } else {
        (&connect, &save)
    };
    let specs = [
        (&test, false, Action::TestConnection),
        (&cancel, false, Action::CloseDialog),
        (second, false, Action::SaveConnection { connect: editing }),
        (first, true, Action::SaveConnection { connect: !editing }),
    ];
    let widths: Vec<f32> = specs
        .iter()
        .map(|(text, primary, _)| button(text, *primary).width(ui, look))
        .collect();
    let total = widths.iter().sum::<f32>() + 10.0 * (specs.len() - 1) as f32;
    let mut left = rect.right() - 20.0 - total;
    let buttons = left;
    // Delete first: the keyboard reaches the buttons left to right.
    if let Some(id) = &form.editing {
        let laid = Text::one(look, widgets::body(look), &delete, palette.danger).layout(ui.ctx());
        let place =
            Rect::from_min_size(pos2(rect.left() + 20.0, y - 16.0), vec2(laid.width(), 32.0));
        let name = format!("{} {}", delete, form.name.trim());
        let response = ui.interact(place, ui.id().with("delete"), Sense::click());
        response.widget_info(|| WidgetInfo::labeled(WidgetType::Button, true, &name));
        laid.paint_left(ui.painter(), place.left(), y);
        if response.hovered() || response.has_focus() {
            let under = y + laid.height() / 2.0;
            ui.painter()
                .hline(place.x_range(), under, Stroke::new(1.0, palette.danger));
        }
        if response.clicked() {
            actions.push(Action::DeleteConnection(id.clone()));
            actions.push(Action::CloseDialog);
        }
    }
    paint_status(ui, form, Err(buttons - 10.0), y, skin);
    for ((text, primary, action), width) in specs.into_iter().zip(widths) {
        let place = Rect::from_min_size(pos2(left, y - 16.0), vec2(width, 32.0));
        left += width + 10.0;
        if button(text, primary)
            .show_at(ui, place, look, palette)
            .clicked()
        {
            actions.push(action);
        }
    }
}

/// The terminal look: the Test's status at the left, the keys at the
/// right. Each key's hint is its button too.
fn terminal_footer(ui: &mut Ui, form: &ConnectionForm, skin: &Skin, actions: &mut Vec<Action>) {
    let Skin { look, palette, .. } = *skin;
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 41.0), Sense::hover());
    let radius = skin.inner_radius();
    ui.painter().rect_filled(
        rect,
        CornerRadius {
            nw: 0,
            ne: 0,
            sw: radius,
            se: radius,
        },
        palette.panel,
    );
    widgets::hline(ui, rect.x_range(), rect.top() + 0.5, palette.outline);
    let y = rect.top() + 1.0 + 20.0;
    paint_status(ui, form, Ok(rect.left() + 14.0), y, skin);
    let role = widgets::secondary(look);
    // The key, what it does, the button it stands for, and what that does.
    let keys = [
        ("tab", "next", None),
        ("ctrl+t", "test", Some(("Test", Action::TestConnection))),
        (
            "ctrl+s",
            "save",
            Some(("Save", Action::SaveConnection { connect: false })),
        ),
        (
            "ctrl+enter",
            "connect",
            Some(("Save & Connect", Action::SaveConnection { connect: true })),
        ),
        ("esc", "cancel", Some(("Cancel", Action::CloseDialog))),
    ];
    let hint = |key: &str, label: &str| {
        // Saving is what the dialog is for: its key takes the accent.
        let color = if key == "ctrl+s" {
            palette.accent
        } else {
            palette.text
        };
        Text::new(look)
            .add(role, key, color)
            .space(role, " ")
            .add(role, label, palette.dim)
    };
    let widths: Vec<f32> = keys
        .iter()
        .map(|(key, label, _)| widgets::measure(ui, hint(key, label)))
        .collect();
    let total = widths.iter().sum::<f32>() + 16.0 * (keys.len() - 1) as f32;
    let mut left = rect.right() - 14.0 - total;
    for ((key, label, button), width) in keys.into_iter().zip(widths) {
        widgets::paint_text(ui, left, y, hint(key, label));
        let place = Rect::from_min_max(
            pos2(left, rect.top() + 1.0),
            pos2(left + width, rect.bottom()),
        );
        left += width + 16.0;
        if let Some((name, action)) = button
            && ButtonSpec::new(name).hidden_at(ui, place).clicked()
        {
            actions.push(action);
        }
    }
}

/// The terminal look: a label in the 150 pt column, then what `add` draws
/// on the same line, 8 below the row before it.
fn terminal_row(ui: &mut Ui, label: &str, skin: &Skin, add: impl FnOnce(&mut Ui, egui::Id)) {
    let Skin { look, palette, .. } = *skin;
    let width = ui.available_width();
    let height = skin.field_height();
    let center = egui::Layout::left_to_right(egui::Align::Center);
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 14.0;
        let label = ui
            .allocate_ui_with_layout(vec2(150.0, height), center, |ui| {
                ui.set_min_size(vec2(150.0, height));
                widgets::label(ui, TextRole::OBody, label, palette.dim, look)
            })
            .inner;
        ui.allocate_ui_with_layout(vec2(width - 164.0, height), center, |ui| {
            ui.set_min_size(vec2(width - 164.0, height));
            ui.spacing_mut().item_spacing.x = 8.0;
            add(ui, label.id);
        });
    });
    ui.add_space(8.0);
}

/// A cell `width` wide on a terminal row, for a field that would take the
/// whole line otherwise.
fn cell(ui: &mut Ui, width: f32, add: impl FnOnce(&mut Ui) -> Response) -> Response {
    ui.allocate_ui_with_layout(
        vec2(width, 0.0),
        egui::Layout::top_down(egui::Align::Min),
        |ui| {
            ui.set_width(width);
            add(ui)
        },
    )
    .inner
}

/// The terminal look: a heading over a rule, 8 below the rows before it.
fn terminal_section(ui: &mut Ui, text: &str, skin: &Skin) {
    let Skin { look, palette, .. } = *skin;
    ui.add_space(8.0);
    widgets::label(ui, TextRole::OGroup, text, palette.text, look);
    ui.add_space(4.0);
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 1.0), Sense::hover());
    widgets::hline(ui, rect.x_range(), rect.center().y, palette.outline);
    ui.add_space(8.0);
}

/// The terminal look's check box: `[x]` and what it turns on. `mark` is
/// the colour of a set mark; `None` locks it.
fn terminal_check(
    ui: &mut Ui,
    checked: Option<&mut bool>,
    text: &str,
    name: &str,
    mark: Color32,
    skin: &Skin,
) -> Response {
    let Skin { look, palette, .. } = *skin;
    let role = TextRole::OBody;
    let on = checked.as_deref().copied().unwrap_or(true);
    let locked = checked.is_none();
    let (glyph, color) = if on {
        ("[x]", mark)
    } else {
        ("[ ]", palette.dim)
    };
    let mut laid = Text::new(look).add(role, glyph, color);
    if !text.is_empty() {
        laid =
            laid.space(role, " ")
                .add(role, text, if locked { palette.text } else { palette.dim });
    }
    let laid = laid.layout(ui.ctx());
    let sense = if locked {
        Sense::hover()
    } else {
        Sense::click()
    };
    let (rect, mut response) = ui.allocate_exact_size(laid.size(), sense);
    response.widget_info(|| WidgetInfo::selected(WidgetType::Checkbox, !locked, on, name));
    laid.paint(ui.painter(), rect.min);
    if response.has_focus() {
        ui.painter().rect_stroke(
            rect.expand(2.0),
            CornerRadius::same(3),
            Stroke::new(1.0, palette.accent),
            StrokeKind::Outside,
        );
    }
    if response.clicked()
        && let Some(checked) = checked
    {
        *checked = !*checked;
        response.mark_changed();
    }
    response
}

/// The terminal look's "keyring" check after a secret; returns its width.
fn terminal_keyring_width(ui: &Ui, skin: &Skin) -> f32 {
    let text = format!("[x] {}", skin.say(skin.keyring()));
    TextRole::OBody.width(ui.ctx(), skin.look.faces, &text)
}

fn terminal_keyring(ui: &mut Ui, mode: &mut PasswordMode, name: &'static str, skin: &Skin) {
    let mut keep = keeps(*mode);
    let text = skin.say(skin.keyring());
    let name = gettext(skin.locale, name);
    if terminal_check(
        ui,
        Some(&mut keep),
        &text,
        &name,
        skin.palette.success,
        skin,
    )
    .changed()
    {
        set_keeps(mode, keep);
    }
}

/// The terminal look's body: label and field, a line each, under headings.
fn terminal_body(
    ui: &mut Ui,
    form: &mut ConnectionForm,
    skin: &Skin,
    focus_name: bool,
    actions: &mut Vec<Action>,
) {
    let Skin { look, palette, .. } = *skin;
    let role = TextRole::OBody;
    let height = skin.field_height();
    egui::Frame::new()
        .inner_margin(egui::Margin {
            left: 18,
            right: 18,
            top: 16,
            // The last row's 8, and 8 more.
            bottom: 8,
        })
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            if form.url_mode {
                terminal_row(ui, &skin.say("URL"), skin, |ui, label| {
                    let field = url_field(ui, form, skin).labelled_by(label);
                    if field.lost_focus() && ui.input(|input| input.key_pressed(egui::Key::Enter)) {
                        actions.push(Action::ApplyUrl);
                    }
                });
            }
            terminal_row(ui, &skin.say("Name"), skin, |ui, label| {
                let name = ui
                    .add(input(ui, &mut form.name, role, height, skin))
                    .labelled_by(label);
                if focus_name {
                    name.request_focus();
                }
            });
            terminal_row(ui, &skin.say("Type"), skin, |ui, _| {
                driver_choice(ui, form, skin, actions);
            });
            terminal_row(ui, &skin.say("Environment"), skin, |ui, _| {
                environment_choice(ui, form, skin);
            });
            if form.driver == Driver::Sqlite {
                terminal_section(ui, &skin.say("Database"), skin);
                file_field(ui, form, skin, actions);
            } else {
                terminal_section(ui, &skin.say("Server"), skin);
                terminal_server(ui, form, skin);
                terminal_section(ui, &skin.say("Security"), skin);
                terminal_security(ui, form, skin, actions);
            }
            terminal_section(ui, &skin.say("Safety"), skin);
            terminal_row(ui, &skin.say("Read-only"), skin, |ui, _| {
                let name = gettext(skin.locale, "Open read-only");
                terminal_check(
                    ui,
                    None,
                    &skin.say("Block every write from this app"),
                    &name,
                    skin.env.color,
                    skin,
                );
                widgets::label(
                    ui,
                    role,
                    &format!("· {}", skin.say("Always on in this version")),
                    palette.dim,
                    look,
                );
            });
            messages(ui, form, skin, actions);
        });
}

/// The terminal look: the host and port on one line, then the database,
/// the user and the password.
fn terminal_server(ui: &mut Ui, form: &mut ConnectionForm, skin: &Skin) {
    let role = TextRole::OBody;
    let height = skin.field_height();
    let named = |response: Response, name: &'static str| {
        let name = gettext(skin.locale, name);
        response.widget_info(|| WidgetInfo::labeled(WidgetType::TextEdit, true, &*name));
    };
    terminal_row(ui, &skin.say("Host : Port"), skin, |ui, _| {
        let host = ui.available_width() - 88.0;
        named(
            cell(ui, host, |ui| {
                ui.add(input(ui, &mut form.host, role, height, skin))
            }),
            "Host",
        );
        named(
            cell(ui, 80.0, |ui| {
                ui.add(input(ui, &mut form.port, role, height, skin))
            }),
            "Port",
        );
    });
    terminal_row(ui, &skin.say("Database"), skin, |ui, label| {
        let hint = hint(ui, &skin.say("Same as the user"), role, skin);
        ui.add(input(ui, &mut form.database, role, height, skin).hint_text(hint))
            .labelled_by(label);
    });
    terminal_row(ui, &skin.say("User"), skin, |ui, label| {
        ui.add(input(ui, &mut form.user, role, height, skin))
            .labelled_by(label);
    });
    let saved = if form.password_is_saved() {
        skin.say(skin.saved_hint())
    } else {
        String::new()
    };
    terminal_row(ui, &skin.say("Password"), skin, |ui, label| {
        let field = ui.available_width() - terminal_keyring_width(ui, skin) - 10.0;
        ui.spacing_mut().item_spacing.x = 10.0;
        cell(ui, field, |ui| {
            let hint = hint(ui, &saved, role, skin);
            ui.add(
                input(ui, &mut form.password, role, height, skin)
                    .password(true)
                    .hint_text(hint),
            )
        })
        .labelled_by(label);
        terminal_keyring(ui, &mut form.password_mode, "Save the password", skin);
    });
}

/// The terminal look: the TLS mode as words, the CA file, and the tunnel.
fn terminal_security(
    ui: &mut Ui,
    form: &mut ConnectionForm,
    skin: &Skin,
    actions: &mut Vec<Action>,
) {
    let role = TextRole::OBody;
    let height = skin.field_height();
    let named = |response: Response, name: &'static str| {
        let name = gettext(skin.locale, name);
        response.widget_info(|| WidgetInfo::labeled(WidgetType::TextEdit, true, &*name));
    };
    terminal_row(ui, &skin.say("SSL mode"), skin, |ui, _| {
        let choices = TLS_MODES.map(|(_, label)| Choice::plain(label));
        let chosen = TLS_MODES
            .iter()
            .position(|(mode, _)| *mode == form.tls)
            .unwrap_or(0);
        if let Some(index) = radio_group(ui, "tls-mode", &choices, chosen, Group::Words, skin) {
            form.tls = TLS_MODES[index].0;
        }
    });
    terminal_row(ui, &skin.say("CA cert"), skin, |ui, label| {
        ca_field(ui, form, label, skin, actions);
    });
    if form.password_can_be_intercepted() {
        intercept_warning(ui, skin);
        ui.add_space(8.0);
    }
    terminal_row(ui, &skin.say("SSH tunnel"), skin, |ui, _| {
        let name = gettext(skin.locale, "Connect through SSH tunnel");
        terminal_check(
            ui,
            Some(&mut form.ssh),
            &skin.say("Connect through SSH"),
            &name,
            skin.palette.success,
            skin,
        );
    });
    if !form.ssh {
        return;
    }
    terminal_row(ui, &skin.say("SSH host : Port"), skin, |ui, _| {
        let host = ui.available_width() - 88.0;
        named(
            cell(ui, host, |ui| {
                ui.add(input(ui, &mut form.ssh_host, role, height, skin))
            }),
            "SSH host",
        );
        named(
            cell(ui, 80.0, |ui| {
                ui.add(input(ui, &mut form.ssh_port, role, height, skin))
            }),
            "SSH port",
        );
    });
    terminal_row(ui, &skin.say("SSH user"), skin, |ui, label| {
        ui.add(input(ui, &mut form.ssh_user, role, height, skin))
            .labelled_by(label);
    });
    terminal_row(ui, &skin.say("Authentication"), skin, |ui, _| {
        let labels = SshAuthKind::ALL.map(|kind| skin.say(kind.label()));
        let choices: Vec<Choice<'_>> = labels.iter().map(|label| Choice::plain(label)).collect();
        let chosen = SshAuthKind::ALL
            .iter()
            .position(|kind| *kind == form.ssh_auth)
            .unwrap_or(0);
        if let Some(index) = radio_group(ui, "ssh-auth", &choices, chosen, Group::Words, skin) {
            form.ssh_auth = SshAuthKind::ALL[index];
        }
    });
    if form.ssh_auth == SshAuthKind::KeyFile {
        terminal_row(ui, &skin.say("Key file"), skin, |ui, label| {
            let (field, clicked) =
                field_with_button(ui, &skin.say("Choose…"), "Choose a key file", skin, |ui| {
                    ui.add(input(ui, &mut form.ssh_key_file, role, height, skin))
                });
            field.labelled_by(label);
            if clicked {
                actions.push(Action::PickKeyFile);
            }
        });
    }
    if form.ssh_auth != SshAuthKind::Agent {
        let name = ssh_secret_name(form.ssh_auth);
        let saved = if form.ssh_secret_is_saved() {
            skin.say(skin.saved_hint())
        } else {
            String::new()
        };
        terminal_row(ui, &skin.say(name), skin, |ui, label| {
            let field = ui.available_width() - terminal_keyring_width(ui, skin) - 10.0;
            ui.spacing_mut().item_spacing.x = 10.0;
            cell(ui, field, |ui| {
                let hint = hint(ui, &saved, role, skin);
                ui.add(
                    input(ui, &mut form.ssh_secret, role, height, skin)
                        .password(true)
                        .hint_text(hint),
                )
            })
            .labelled_by(label);
            terminal_keyring(ui, &mut form.ssh_secret_mode, "Save the SSH secret", skin);
        });
    }
}
```

How it fits together, for whoever changes it next:

- `show` builds the `Skin`, places the modal (`Placement`, `reserve`, `top_edge`), draws header, scrolling body and footer for the look, then takes the dialog's keys.
- `radio_group` draws every one-of-many choice: the tabs, the driver, the environment, and in the terminal look the TLS mode and the SSH login. Each choice is a radio button to screen readers.
- `row` and `Track` are the macOS grid (`1.4fr 1fr`, `1fr 96px`); `terminal_row` is the terminal look's label column and field.
- `style_controls` gives egui's own text fields and pop-up buttons the design's box (fill, border, corners) for the dialog only.
- `top_cap` exists because a 5 pt tall rounded rectangle cannot have 14 pt corners: the stripe is a polygon that follows the sheet's corners.

- [ ] **Step 5: Remove the legacy helpers**

The old dialog was the last user of the pre-role font helpers.

In `src/typography.rs`, delete the module at the end of the file:

```rust
/// The pre-role helpers, kept for the edit connection dialog until it
/// moves to [`TextRole`] (it is out of 0.1.0's scope).
pub mod legacy {
    use egui::FontId;

    /// Dialog titles.
    pub const TEXT_TITLE: f32 = 17.0;

    pub fn semibold(size: f32) -> FontId {
        fastframe_fonts::Weight::SemiBold.font_id(size)
    }
}
```

In `src/theme.rs`, delete:

```rust
/// The edit connection dialog's title font (it keeps the pre-role helpers).
pub use crate::typography::legacy::{TEXT_TITLE, semibold};
```

- [ ] **Step 6: Name the TLS modes in messages as the dialog does**

Two messages still say "Verify certificate" and "Verify certificate and host", the old list's labels. The tests on them only look for "needs a CA file", which stays.

In `ConnectionForm::to_spec` (`src/model.rs`):

```rust
                    return Err(
                        "verify-ca needs a CA file. Choose verify-full to use the system \
                         certificates."
                            .into(),
                    );
```

In `crates/tabletist-db/src/tls.rs`:

```rust
/// Why `verify-ca` without a CA file is refused.
const VERIFY_CA_NEEDS_A_CA_FILE: &str =
    "verify-ca needs a CA file; use verify-full to use the system certificates";
```

- [ ] **Step 7: Run the tests**

Run: `~/.cargo/bin/cargo fmt --all && ~/.cargo/bin/cargo test --locked --lib`
Expected: PASS, all of them. If a test fails, fix the view, not the test, unless the test names something this plan's decisions changed.

Things that have bitten in this codebase before, if a test does fail:
- `harness.click(label)` looks for a Button with that name first, then any node. The picker behind the dialog has a "New connection" button; never assert on that text to prove the dialog is open.
- A plain label's text is its AccessKit value, not its label (`crate::testing::labels` reads both).
- egui lays out on the first frame and settles on the second: read bounds after `harness.settle()`.

- [ ] **Step 8: Run every check**

Run:
```bash
~/.cargo/bin/cargo fmt --all --check
~/.cargo/bin/cargo clippy --locked --workspace --all-targets -- -D warnings
~/.cargo/bin/cargo test --locked --workspace --all-targets
RUSTDOCFLAGS='-D warnings' ~/.cargo/bin/cargo doc --locked --workspace --no-deps
```
Expected: all pass with no warnings.

- [ ] **Step 9: Commit**

```bash
git add src/ui/connect_dialog.rs src/ui/mod.rs src/typography.rs src/theme.rs src/model.rs crates/tabletist-db/src/tls.rs
git commit -m "Draw the connection dialog as designed

macOS draws a sheet under the environment's colour: Parameters and URL
tabs, the fields in Server and Security groups, a read-only note, and
Delete, Test, Cancel and Save in the footer. The terminal look draws a
two-column form with its keys in the footer.

The environment sets the colour, so the colour list is gone. The
password's three modes become a keyring check box. The CA certificate
has a Choose button. A passed test says what it reached and how long
it took. Mod+S saves, Mod+T tests, Mod+Enter saves and connects.

The dialog draws through text roles now; the last pre-role font helpers
go with the old one. Messages name the TLS modes as the dialog does.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

### Task 8: List the dialog's keys

**Files:**
- Modify: `src/ui/keys.rs`

- [ ] **Step 1: Write the failing test**

In `mod tests` of `src/ui/keys.rs`:

```rust
    #[test]
    fn the_shortcut_table_lists_the_connection_dialogs_keys() {
        assert!(
            SHORTCUTS
                .iter()
                .any(|(keys, what)| keys.contains("Mod+S") && what.contains("connection dialog"))
        );
    }
```

- [ ] **Step 2: Run it to see it fail**

Run: `~/.cargo/bin/cargo test --locked --lib the_shortcut_table_lists`
Expected: FAIL: the assertion.

- [ ] **Step 3: Add the row**

In `SHORTCUTS`, after `("Mod+N", "New connection"),`:

```rust
    (
        "Mod+S, Mod+T, Mod+Enter",
        "Save, test, or save and connect in the connection dialog",
    ),
```

- [ ] **Step 4: Run the tests**

Run: `~/.cargo/bin/cargo test --locked --lib keys`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/ui/keys.rs
git commit -m "List the connection dialog's keys with the shortcuts

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

### Task 9: Compare with the design by eye

Screenshots are for looking at, never for asserting on. They are written to `target/shots/` (gitignored) and stay there.

**Files:**
- Modify: `src/shots.rs`
- Modify: `src/ui/connect_dialog.rs` (only what the comparison finds)

- [ ] **Step 1: Stage the artboards' scene**

In `src/shots.rs`, after `fn saved()`:

```rust
/// The Bookshop's production database behind a bastion, open in the
/// connection dialog after a Test that passed: the scene the dialog's
/// mockups show.
fn edit_production(harness: &mut Harness) {
    let (mut spec, _) = ConnectSpec::from_url(
        "postgres://app_readonly@db.example.com:5432/bookshop_production?sslmode=verify-full",
    )
    .unwrap();
    spec.ca_file = Some("ca-bundle.pem".into());
    spec.ssh = Some(tabletist_db::SshSpec {
        host: "bastion.example.com".into(),
        port: 22,
        user: "deploy".into(),
        auth: tabletist_db::SshAuth::KeyFile {
            path: "~/.ssh/id_ed25519".into(),
        },
    });
    // The picker's scene has this connection already: edit that one.
    let id = harness
        .app
        .connections
        .connections
        .iter()
        .find(|connection| connection.spec.database == "bookshop_production")
        .map_or_else(ConnectionId::new, |connection| connection.id.clone());
    harness.app.connections.upsert(SavedConnection {
        id: id.clone(),
        name: "Bookshop".into(),
        color: ColorTag::Red,
        environment: Some(crate::connections::Environment::Production),
        password: PasswordMode::Keyring,
        ssh_secret: PasswordMode::None,
        spec,
    });
    harness.app.apply(Action::EditConnection(id));
    if let Some(crate::model::Dialog::Connection(form)) = &mut harness.app.dialog {
        form.test = crate::model::TestState::Passed;
        form.test_took = Some(Duration::from_millis(42));
    }
}
```

In `fn shots()`, replace the `"dialog"` scene with two:

```rust
    both("dialog", |harness| {
        edit_production(harness);
    });
    both("dialog-new", |harness| {
        harness.press(egui::Key::N, egui::Modifiers::COMMAND);
        let postgres = harness.app.look.label("PostgreSQL");
        harness.click(&postgres);
    });
```

In `mod mock`, add the two artboards to `Screen`. The enum and its list:

```rust
    pub enum Screen {
        MacWorkspace,
        OmarchyWorkspace,
        MacPicker,
        OmarchyPicker,
        MacDialog,
        OmarchyDialog,
    }
```

```rust
        pub const ALL: [Screen; 6] = [
            Self::MacWorkspace,
            Self::OmarchyWorkspace,
            Self::MacPicker,
            Self::OmarchyPicker,
            Self::MacDialog,
            Self::OmarchyDialog,
        ];
```

In `name`:

```rust
                Self::MacDialog => "macos-connection-edit",
                Self::OmarchyDialog => "omarchy-connection-edit",
```

In `look`, `size`, `design_scale` and `palette`, the dialogs join their pickers' arms:

```rust
                Self::MacWorkspace | Self::MacPicker | Self::MacDialog => Look::macos(),
                Self::OmarchyWorkspace | Self::OmarchyPicker | Self::OmarchyDialog => {
                    Look::omarchy()
                }
```

```rust
                Self::MacWorkspace | Self::MacPicker | Self::MacDialog => egui::vec2(1440.0, 900.0),
                Self::OmarchyWorkspace => egui::vec2(1896.0, 1056.0),
                Self::OmarchyPicker | Self::OmarchyDialog => egui::vec2(936.0, 1016.0),
```

```rust
                Self::MacWorkspace | Self::MacPicker | Self::MacDialog => 2000.0 / 1440.0,
                Self::OmarchyWorkspace => 2000.0 / 1920.0,
                Self::OmarchyPicker | Self::OmarchyDialog => 1846.0 / 960.0,
```

```rust
                Self::MacWorkspace | Self::MacPicker | Self::MacDialog => Palette::light(),
                Self::OmarchyWorkspace | Self::OmarchyPicker | Self::OmarchyDialog => {
                    tokyo_night()
                }
```

In `stage`, after the pickers' arm:

```rust
                Self::MacDialog | Self::OmarchyDialog => {
                    pickers(harness);
                    edit_production(harness);
                }
```

If another `match` on `Screen` exists further down the file, the compiler names it: give the dialogs the arm their pickers have.

- [ ] **Step 2: Render**

Run: `~/.cargo/bin/cargo test --locked --features shots --lib shots -- --ignored`
Expected: PASS (it needs a GPU). `target/shots/` holds `dialog-<look>-<light|dark>.png`, `dialog-new-<look>-<light|dark>.png`, `mock-macos-connection-edit.png` and `mock-omarchy-connection-edit.png`.

- [ ] **Step 3: Compare with the artboards**

Open `mock-macos-connection-edit.png` beside "Edit connection, macOS" and `mock-omarchy-connection-edit.png` beside "Edit connection, Omarchy" (the user's Design canvas; ask the user for it if it is not at hand). Go through "The design" above, part by part:

- the sheet's width, corners, shadow or border, and the stripe or tinted title line in the production red;
- the header: title, "Bookshop · production", the tabs and Close, or "u paste url";
- the environment choice: the chosen one's fill, dot and text colour;
- the groups: hairline, heading set into the edge, 10 between rows, 12 between columns, 32 pt fields;
- the fields: fill, border, corners, monospace for what goes to the server;
- the safety note's tint; the footer's fill, rule and buttons; the status in the success colour.

Then look at every `dialog-*.png` for anything broken in the other looks and in the dark palettes: clipped text, a field narrower than its cell, a control off its line, a tint that hides its text.

What is expected to differ from the artboards is in "Where the design and the app differ": Type in place of Group, the Test environment, verify-ca, the tunnel's extra fields, Save & Connect, "Always on in this version", no server version.

- [ ] **Step 4: Fix what differs**

Change `src/ui/connect_dialog.rs` only. After each fix:

Run: `~/.cargo/bin/cargo test --locked --lib ui::tests && ~/.cargo/bin/cargo test --locked --features shots --lib shots -- --ignored`
Expected: PASS; look again.

Stop when the two mock screenshots read as their artboards apart from the expected differences. Do not add a test for any of it.

- [ ] **Step 5: Run every check and commit**

Run the four checks from "Global Constraints".

```bash
git status --short   # only src/shots.rs and, if fixes were needed, src/ui/connect_dialog.rs
git add src/shots.rs src/ui/connect_dialog.rs
git commit -m "Stage the connection dialog's design scene for screenshots

The Bookshop's production connection behind a bastion, with a passed
test: what the dialog's mockups show, for comparing by eye.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

If the comparison changed the view, say what in the commit body.

### Task 10: Finish

- [ ] **Step 1: Run the app once**

Run: `~/.cargo/bin/cargo run --locked`
Open New connection: choose PostgreSQL, an environment, the URL tab, the SSH tunnel; Test against a server that is not there (the error shows under the fields); Escape. This is Linux: say in the report that macOS and Windows were only compiled, and that the macOS look was checked through screenshots.

- [ ] **Step 2: Bring the product spec up to date**

In `docs/superpowers/specs/2026-09-27-tabletist-design.md`, section 5.4, the "Connection dialog" and "Password storage" items become:

```markdown
- Connection dialog: name; driver; environment (local, dev, test, staging,
  production, none), which sets the connection's colour; host, port,
  database, user, password, or SQLite file picker; SSL mode, with a CA
  certificate for the modes that check one; SSH tunnel (host, port, user,
  auth method, password or key file + passphrase); the read-only note; a URL
  that fills the fields; **Test**; **Save**; **Save & Connect**; **Delete**
  when editing. macOS draws it as a sheet of grouped fields, the terminal
  look as a two-column form with its keys in the footer.
- Password storage per secret: saved in the keyring (default) or asked for
  every time. Secrets live only in the keyring, keyed by connection id.
```

Commit it with the plan:

```bash
git add docs/superpowers/specs/2026-09-27-tabletist-design.md docs/superpowers/plans/2026-09-30-connection-dialog-design.md
git commit -m "Describe the connection dialog as it is now

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

- [ ] **Step 3: Report**

List what was built, the decisions in "Where the design and the app differ" that the user should confirm (Group, read-only, Save & Connect, Delete without asking), and the test results. Then use superpowers:finishing-a-development-branch.
