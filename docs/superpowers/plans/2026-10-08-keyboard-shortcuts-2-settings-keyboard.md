# Keyboard Shortcuts, Run 2: Settings → Keyboard Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** The user can change a key: by hand in `settings.toml` under `[keys.<scope>]`, or on a Keyboard page of the Settings window in every look, with a taken key reported and swapped on request.

**Architecture:** The user's changes are data beside the defaults (`keymap::Overrides`, a field of `Settings`), read and written by the settings store like every other key. `Keymap::with(layout, &overrides)` lays them over `BINDINGS` and says which lines it refused and why; the conflict rule is the one the keymap's own test already holds the defaults to, moved out of the tests. The Keyboard page only asks pure functions (`rebind`, `unbind`, `reset`) for the next `Overrides` and pushes one action.

**Tech Stack:** Rust 1.98, egui (the fork), `toml` (parse only; the app writes its own text), the headless `testing::Harness`.

**Spec:** `docs/superpowers/specs/2026-10-08-keyboard-shortcuts-design.md`, section 4 and test 9. Run 1's plan: `2026-10-08-keyboard-shortcuts-1-keymap.md`.

**Design:** the canvas Artifact `https://claude.ai/artifact/4FtP3GnsXevWfpBsotjVb6`, boards `project/MacSettingsKeyboard.dc.html` and `project/OmarchySettingsKeyboard.dc.html` (read them with the Artifact tool; nothing of them is committed).

---

## Departures and open decisions

Each row is built as the "Built" column says unless the user says otherwise before the run.

| # | Canvas or brief | Built | Why |
|---|---|---|---|
| D1 | Any key for any command (the canvas' file gives a typed letter's command a chord: `"ctrl+d" = "set_default"`) | **A key keeps its kind.** A command takes a chord where it has a chord, typed letters where it has letters, a `:` line where it has one, a range of digits where it has one. A line of another kind is refused with its reason. A built command with no key in the look gets none there | Chords are read in one place of the handler and typed letters in another (`letters`, 540 lines, one character after another). One way in for every command is a rework of the handler of its own. Say so to have it first: it adds a sitting |
| D2 | Rows of the settled table that hold several commands (`Undo / redo one cell`) | Listed by command, each with its own name (`Undo one cell`, `Redo one cell`), as the `?` list does. A row with one command has the table's name exactly. A command with keys in two places (`Edit cell`: grid, inspector) has a line for each | One line records one key in one place |
| D3 | Every listed command can be recorded | Listed and dimmed, not recorded: the six whose key says which way (`Move by row`, `Move by column`, `Move by page`, `Move by field`, `Move in the sidebar`, `Move in the connections`); `Help` and `Leave the editor`; Settings on the Mac; and every key that is read only while something else shows (the completion list, the large editor, the pending SQL, the inspector's brackets from the rows, an answer of the assistant) | The handler reads the direction from the spelling. `?` is named in the cell editor's own rule and Esc out of the SQL editor is egui's. ⌘, is the system menu item's. A key that holds only while a thing shows can be the same key as another's in its scope, which `"keys" = "command"` cannot say twice |
| D4 | `[keys.editor]` on the canvas | `[keys.sql-editor]`: the brief's scope ids, `Scope::id()` | The brief settles the scope names |
| D5 | `# :keys default writes the full list` (where to is not said) | `:keys default` writes `keys.default.toml` beside `settings.toml`: every key of the look that can be changed, in the file's own form, to copy lines from. The notice line says where. Fixed keys and a prompt's are not in it | Written into `settings.toml` it would make "only changes from the defaults live here" untrue and pin every key to today's defaults |
| D6 | Mac: search field, `All / Changed` switch, `Export cheat sheet`, the note about vim keys. Omarchy: `/` filter | Left out | New features beside the brief's section 4. The changed dot (Mac) is built |
| D7 | Nav lists Appearance, Editor, AI assistant, Updates | Nav lists General and Keyboard | A page that is not there is not offered |
| D8 | Omarchy footer of the page says `h/l change` | Not on this page: `j/k move`, `enter record`, `dd unbind`, `ctrl+e open file`, `R reset`, `esc close` | Nothing on the page steps left or right |
| D9 | Recording (the canvas shows the state, not how keys end) | Omarchy: a chord is taken at once; typed letters gather (`g`, then `gd`) and Enter takes them, Backspace takes one back. Enter, Esc and Backspace alone cannot be recorded there. On a sheet the first chord is taken, and Esc alone cannot be recorded. Any of them can be written in the file | Nothing else tells `g` from the start of `gd`, and Esc is how a recording is given up |
| D10 | Canvas file comment has two lines | The app writes `# only changes from the defaults live here` above the first `[keys]` table and not the `:keys default` line | The settings writer does not know the look, and the Mac has no `:` prompt |
| D11 | The Mac's keys are stored (the brief names only Omarchy's file) | In the same `settings.toml`, the same tables. The lines are read as the keys of the look the app runs in; a change made on the page writes the file without the lines that look refuses | One store, and a machine has one look |
| D12 | A line with two keys (`Edit cell`: ↩ and F2) | A recorded key stands in for every key of the line that is pressed its way: both chords go. `:w` beside a chord stays. Two chords for one command are written in the file | The canvas shows one shortcut to a line |
| D13 | Swap | Both lines change in every scope they hold in (`Set DEFAULT` is the grid's and the inspector's) | A line is the keymap's line |
| D14 | Omarchy has `dd unbind` and `R reset` | The sheet has `Reset all…` only, as its board does. A single line is put back by recording its default | The Mac board shows no control for either |
| D15 | "List every command in the table" | The keys of prompts and dialogs are not on the page, `Confirm production write` among them | They are the prompt's own letters and are not rebound |
| D16 | (not on the canvas) | On the Mac ⌘H, ⌥⌘H and ⌘Q are refused: the system's menu has them. On Omarchy vim's `ctrl+w`, `ctrl+h`, `ctrl+u`, `ctrl+r` are refused where text is typed | The key would never arrive, or would take the text's own |

Not in this run: the website's key pages and README (Run 3), the PR.

## What a line of the file means

```toml
# only changes from the defaults live here
[keys.grid]
"D"  = "delete_row"
"dd" = "set_default"

[keys.unbind]
grid = ["gd"]
```

- `[keys.<scope>]`, `"<keys>" = "<command>"`: in that scope the command has **exactly** the keys its lines give it there. A command with no line has its defaults. Keys are spelled as the keymap spells them (`ctrl+shift+f`, `yy p`, `:w`, `alt+1..9`); commands by `Command::id()` (`delete_row`).
- `[keys.unbind]`, `<scope> = ["<keys>", …]`: takes a default key away from the command that has it there. This is how a command comes to have no key.
- The lines apply to the look the app runs in. In memory they are kept by scope (the order of `Scope::ALL`), then as written: "the later line" below is the later in that order.
- One key names one command in a scope: TOML has a key once in a table.
- A line is ignored, and drawn red in the file pane, when: the keys do not read; the command or its scope is not one; the command cannot be rebound, or its key in that scope is read only while something else shows (D3), or it is not that scope's; the keys are not of a kind the command takes there (D1; `Reload`, `Find any object` and `Tree or flat list` take one letter); on Omarchy they are one of vim's insert keys where text is typed, or name `cmd`; on the Mac they are the system menu's (D16); another command has them where both could be read (the later line of two gives way, a line gives way to a default; a chord of the sidebar is also held against the chords of the grid, a SQL editor and its result that are read from anywhere in the tab); an unbind names a key nobody has.

## Files

| File | Responsibility |
|---|---|
| `src/keymap.rs` (modify) | `Command::id/from_id/rebindable`, `Scope::from_id/name`, `Press`, `spelling()`, `spell(&str)`, F-keys, the conflict rule out of the tests (`clash`, `Entry`, `Keymap::entries/conflicts`), interning, four new keys of the Settings screen, `KeysDefault` built |
| `src/keymap/overrides.rs` (create) | `Overrides`, `Bind`, `Unbind`, `Refused`, `Why`; `Keymap::with`; `rebind`, `unbind`, `reset`, `full`; `lines()` of the Keyboard page |
| `src/settings.rs` (modify) | `Settings.keys`; reading `[keys.*]`, writing it, the lines the keys are on |
| `src/app.rs` (modify) | the keymap rebuilt from the settings and the look; refused lines marked; the page's actions |
| `src/app/editing.rs` (modify) | `:keys default` |
| `src/model.rs` (modify) | `SettingsPage`, `KeysPage`, `Rebinding`, the actions |
| `src/backend.rs`, `src/paths.rs` (modify) | `StateFile::Text`, `AppDirs::default_keys_file()` |
| `src/ui/settings/mod.rs` (modify) | pages, the nav's keys, routing keys to the recorder |
| `src/ui/settings/record.rs` (create) | a frame's events as recorded keys |
| `src/ui/settings/keyboard_terminal.rs` (create) | the Omarchy page |
| `src/ui/settings/keyboard_sheet.rs` (create) | the Mac and Windows page |
| `src/ui/settings/terminal.rs`, `sheet.rs`, `file_pane.rs` (modify) | nav with two pages; a quoted key coloured; the pane marks a line by number |
| `docs/_reference/settings-and-files.md` (modify) | the `[keys]` tables, with a sample the app takes |
| `AGENTS.md` (modify) | where a changed key lives |

`tests/keys.rs` stays as it is: nothing outside `src/keymap.rs` spells a key. `src/keymap/overrides.rs` must not either (it builds vim's keys from `Key` values, and the recorder asks `keymap::spelling`).

## Checks

After every task, before its commit:

```bash
/tmp/claude-1000/-home-igor-Work-tabletist--claude-worktrees-tabletist-keyboard-shortcuts-fcc089/eea5abc5-d9f8-45e1-8992-94bdc1329f0e/scratchpad/checks.sh
```

It runs `~/.cargo/bin/cargo fmt --all --check`, clippy with `-D warnings` (workspace, all targets, and the `shots` feature), the tests, and the docs build. If the script is gone, run those five by hand (Run 1's plan lists them). Commits are signed: on "agent refused operation" stage the change, say so, and wait. Chain `git add … && git commit …`. No em dashes anywhere. Bookshop names only in fixtures.

## Sittings

- **A (tasks 1 to 5):** the file works by hand: a `[keys]` line changes the key live, a bad one is red.
- **B (tasks 6 to 8):** pages in the Settings window, the Omarchy Keyboard page, `:keys default`. Test 9 passes on Omarchy.
- **C (tasks 9 and 10):** the Mac and Windows page, the docs. Test 9 passes in every look.

Stop after each sitting and say what was built.

---

### Task 1: Names, kinds and spellings the file needs

**Files:** Modify `src/keymap.rs`.

- [ ] **Step 1: Write the failing tests** (in `mod tests` of `src/keymap.rs`)

```rust
#[test]
fn a_command_is_named_in_the_file_by_its_id() {
    assert_eq!(Command::DeleteRow.id(), "delete_row");
    assert_eq!(Command::SetDefault.id(), "set_default");
    assert_eq!(Command::RunAll.id(), "run_all");
    assert_eq!(Command::GoToConnectionWindow.id(), "go_to_connection_window");
    for command in Command::ALL {
        assert_eq!(Command::from_id(command.id()), Some(*command));
    }
    assert_eq!(Command::from_id("no_such_command"), None);
    for scope in Scope::ALL {
        assert_eq!(Scope::from_id(scope.id()), Some(scope));
    }
    assert_eq!(Scope::from_id("editor"), None);
}

#[test]
fn a_chord_is_spelled_back_as_the_keymap_spells_it() {
    for binding in BINDINGS {
        for bound in [&binding.mac, &binding.omarchy] {
            for chord in bound.chords {
                if let Ok(Spelled::Strokes(strokes)) = spell(chord.keys)
                    && let [Stroke::Key { mods, key }] = strokes.as_slice()
                {
                    assert_eq!(spelling(*mods, *key).as_deref(), Some(chord.keys));
                }
            }
        }
    }
    let mods = Mods { ctrl: true, shift: true, ..Mods::default() };
    assert_eq!(spelling(mods, Key::F5).as_deref(), Some("ctrl+shift+f5"));
    assert_eq!(spell("f5").map(|spelled| spelled.press()), Ok(Press::Chord));
}

#[test]
fn a_spelling_is_pressed_as_a_chord_a_range_typed_letters_or_a_line() {
    let press = |keys| spell(keys).expect("reads").press();
    assert_eq!(press("ctrl+s"), Press::Chord);
    assert_eq!(press("enter"), Press::Chord);
    assert_eq!(press("alt+1..9"), Press::Digits);
    assert_eq!(press("yy p"), Press::Typed);
    assert_eq!(press("D"), Press::Typed);
    assert_eq!(press(":w"), Press::Line);
}

#[test]
fn the_keys_that_say_which_way_and_a_prompts_are_not_rebound() {
    for layout in Layout::ALL {
        assert!(!Command::MoveRow.rebindable(layout));
        assert!(!Command::MoveInTree.rebindable(layout));
        assert!(!Command::SettingsClose.rebindable(layout));
        // `?` is named in the cell editor's own rule, and Esc out of the
        // SQL editor is the text field's.
        assert!(!Command::Help.rebindable(layout));
        assert!(!Command::LeaveEditor.rebindable(layout));
        assert!(Command::DeleteRow.rebindable(layout));
    }
    // The Mac's menu has the key of its Settings item.
    assert!(!Command::Settings.rebindable(Layout::Mac));
    assert!(Command::Settings.rebindable(Layout::Omarchy));
}

#[test]
fn a_command_has_one_line_in_a_scope() {
    // What `"keys" = "command"` under `[keys.<scope>]` stands over.
    for (index, a) in BINDINGS.iter().enumerate() {
        for b in &BINDINGS[index + 1..] {
            let shared = a.scopes.iter().any(|scope| b.scopes.contains(scope));
            assert!(a.command != b.command || !shared, "{:?}", a.command);
        }
    }
}
```

- [ ] **Step 2: Run them**

Run: `~/.cargo/bin/cargo test --locked --lib keymap::tests`
Expected: does not compile (`id`, `from_id`, `spelling`, `Press`, `rebindable`, `Key::F5` spelling).

- [ ] **Step 3: Implement**

`spell` takes any text, so a line of the file is read without being kept. Change its signature to `pub fn spell(keys: &str) -> Result<Spelled, Unread>`, `Spelled::Ex(&'static str)` to `Spelled::Ex(String)` (`Ok(Spelled::Ex(line.to_owned()))`), `written(layout, keys: &str)`, and in `Keymap::ex` compare `name == line || (number && name == "N")`. In the tests' `clash`, `Ex` compares as before. `grep -n "Spelled::Ex" -r src` finds the rest (only `src/keymap.rs` today).

Add to `NAMED`, after `f2` and `f6` are replaced by the whole row:

```rust
    ("f1", Key::F1), ("f2", Key::F2), ("f3", Key::F3), ("f4", Key::F4),
    ("f5", Key::F5), ("f6", Key::F6), ("f7", Key::F7), ("f8", Key::F8),
    ("f9", Key::F9), ("f10", Key::F10), ("f11", Key::F11), ("f12", Key::F12),
```

(one to a line as rustfmt will have them). Then:

```rust
/// How a spelling is pressed. A key that is rebound keeps this: chords
/// are read in one place and typed characters in another.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Press {
    /// One key, with what is held.
    Chord,
    /// A digit of `1..9`, with what is held.
    Digits,
    /// Characters, typed one after another.
    Typed,
    /// A line of the `:` prompt.
    Line,
}

impl Spelled {
    pub fn press(&self) -> Press {
        match self {
            Spelled::Ex(_) => Press::Line,
            Spelled::Digits(_) => Press::Digits,
            Spelled::Strokes(strokes) => {
                let typed = strokes
                    .iter()
                    .all(|stroke| matches!(stroke, Stroke::Text(_)));
                if typed { Press::Typed } else { Press::Chord }
            }
        }
    }
}

/// One key with what is held, spelled as the keymap spells it: the
/// modifiers in the keymap's order, then the key. `None` for a key no
/// spelling names.
pub fn spelling(mods: Mods, key: Key) -> Option<String> {
    let name = match (named_word(key), key_mark(key)) {
        (Some(word), _) => word.to_owned(),
        (None, Some(mark)) => mark.to_string(),
        (None, None) => {
            let name = key.name();
            let mut chars = name.chars();
            match (chars.next(), chars.next()) {
                (Some(char), None) if char.is_ascii_alphanumeric() => {
                    char.to_ascii_lowercase().to_string()
                }
                _ => return None,
            }
        }
    };
    let mut out = String::new();
    for (held, word) in [
        (mods.cmd, "cmd"),
        (mods.ctrl, "ctrl"),
        (mods.alt, "alt"),
        (mods.shift, "shift"),
    ] {
        if held {
            out.push_str(word);
            out.push('+');
        }
    }
    out.push_str(&name);
    Some(out)
}

/// The word a spelling has for `key`, if it has one.
fn named_word(key: Key) -> Option<&'static str> {
    NAMED
        .iter()
        .find(|(_, known)| *known == key)
        .map(|(name, _)| *name)
}
```

Ids (`use std::sync::LazyLock;` at the top):

```rust
impl Command {
    /// The command's name in `settings.toml`: `delete_row`. The variant's
    /// own, so a new command has one without being given it.
    pub fn id(self) -> &'static str {
        static IDS: LazyLock<Vec<String>> = LazyLock::new(|| {
            Command::ALL
                .iter()
                .map(|command| {
                    let mut id = String::new();
                    for char in format!("{command:?}").chars() {
                        if char.is_ascii_uppercase() && !id.is_empty() {
                            id.push('_');
                        }
                        id.push(char.to_ascii_lowercase());
                    }
                    id
                })
                .collect()
        });
        // `ALL` has the variants in the order they are declared.
        &IDS[self as usize]
    }

    pub fn from_id(id: &str) -> Option<Command> {
        Command::ALL
            .iter()
            .copied()
            .find(|command| command.id() == id)
    }

    /// Whether the user can give the command other keys in `layout`. Not
    /// the keys of a prompt, which are its own. Not the ones that say
    /// which way by which key they are: `j` is down because it is `j`.
    /// Not the two that something outside the keymap reads. Not the Mac's
    /// Settings: the system's menu item has that key. (A line that holds
    /// only while something shows is fixed too: `overrides` says so by
    /// the line, not by the command.)
    pub fn rebindable(self, layout: Layout) -> bool {
        use Command as C;
        let by_key = matches!(
            self,
            C::MoveInConnections
                | C::MoveInTree
                | C::MoveRow
                | C::MoveColumn
                | C::MovePage
                | C::MoveField
        );
        // Read by something that is not asked through the keymap: `?` is
        // the one letter a selected cell does not start an edit with, and
        // Esc leaves the SQL editor inside egui's text field.
        let elsewhere = matches!(self, C::Help | C::LeaveEditor);
        let menus = layout == Layout::Mac && self == C::Settings;
        self.info().group != Group::Prompts && !by_key && !elsewhere && !menus
    }
}

impl Scope {
    pub fn from_id(id: &str) -> Option<Scope> {
        Scope::ALL.into_iter().find(|scope| scope.id() == id)
    }

    /// The scope as the Keyboard page of a sheet names it. English: the
    /// page translates.
    pub fn name(self) -> &'static str {
        match self {
            Self::Global => "Global",
            Self::Connections => "Connections",
            Self::Sidebar => "Sidebar",
            Self::Grid => "Grid",
            Self::CellEditor => "Cell editor",
            Self::Inspector => "Inspector",
            Self::SqlEditor => "SQL editor",
            Self::Results => "Results",
            Self::Prompt => "Prompt",
        }
    }
}
```

Before relying on `by_key`, confirm it is every command whose handler looks at the spelling: `grep -n '"up" =>\|"k" |\|== .k.\|matches!(keys' src/ui/keys.rs src/ui/*.rs src/ui/settings/mod.rs`. What it finds beyond the six must be in `Group::Prompts` (quick open, the Settings screen) or read the spelling only to tell an arrow from another key of the same command (`completion_keys`). Anything else joins `by_key`, with a line in D3.

- [ ] **Step 4: Run the tests**

Run: `~/.cargo/bin/cargo test --locked --lib keymap::tests`
Expected: PASS, the old ones too.

- [ ] **Step 5: Checks, then commit**

```bash
git add src/keymap.rs && git commit -m "Name a command and spell a chord as the settings file will

A line of the file names a command by an id of its own and a key by the
keymap's spelling. A spelling is read from any text now, and says how it
is pressed: a key that is rebound keeps that.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 2: The conflict rule, out of the tests

**Files:** Modify `src/keymap.rs`.

The rule `no_two_commands_share_a_key_where_both_could_be_read` holds the defaults to is the one a rebind is checked by. It moves from `mod tests` to the module, unchanged in what it says.

- [ ] **Step 1: Move, do not rewrite.** Cut `presses`, `clash`, `Entry` and `entries` out of `mod tests`. `presses` and `clash` become functions of the module, `pub(super)` so the overrides beside it can ask them; `Entry` becomes `pub struct Entry` with public fields and `#[derive(Clone, Debug)]`; `entries` becomes `impl Keymap { pub fn entries(&self, layout: Layout) -> Vec<Entry> }` (its `.expect("a spelling that reads")` becomes `let Ok(spelled) = spell(chord.keys) else { continue };`: a user's line is checked before it gets here, and a panic is not the answer to a slip). Add:

```rust
impl Entry {
    /// Whether this key and `other` could be read in the same moment.
    pub fn beside(&self, other: &Entry) -> bool {
        self.scope.alongside(other.scope)
            && self.when == other.when
            && self.mode.overlaps(other.mode)
    }
}

impl Keymap {
    /// The pairs of keys that two commands would both answer in `layout`,
    /// the one that is earlier in [`Keymap::entries`] first.
    pub fn conflicts(&self, layout: Layout) -> Vec<(Entry, Entry)> {
        let entries = self.entries(layout);
        let mut found = Vec::new();
        for (index, a) in entries.iter().enumerate() {
            for b in &entries[index + 1..] {
                if a.command != b.command && a.beside(b) && clash(layout, &a.spelled, &b.spelled) {
                    found.push((a.clone(), b.clone()));
                }
            }
        }
        found
    }
}
```

- [ ] **Step 2: The test asks the module.** `no_two_commands_share_a_key_where_both_could_be_read` becomes:

```rust
#[test]
fn no_two_commands_share_a_key_where_both_could_be_read() {
    let keymap = Keymap::default();
    for layout in Layout::ALL {
        let conflicts = keymap.conflicts(layout);
        assert!(conflicts.is_empty(), "{layout:?}: {conflicts:#?}");
    }
}
```

The other tests that used `entries(&keymap, layout)` call `keymap.entries(layout)`.

- [ ] **Step 3: Prove the rule still bites.** Add:

```rust
#[test]
fn a_key_given_to_two_commands_is_found() {
    let mut keymap = Keymap::default();
    // Run all, on the key of Run statement.
    let run_all = keymap
        .bindings
        .iter_mut()
        .find(|binding| binding.command == Command::RunAll)
        .expect("Run all");
    // A literal: a call of a `const fn` is not kept for `'static` here.
    run_all.omarchy = Bound {
        chords: &[Chord {
            keys: "ctrl+enter",
            kind: Kind::Own,
        }],
        mode: Mode::Any,
        ..NONE
    };
    let conflicts = keymap.conflicts(Layout::Omarchy);
    assert!(conflicts.iter().any(|(a, b)| {
        [a.command, b.command].contains(&Command::RunAll)
            && [a.command, b.command].contains(&Command::RunStatement)
    }));
    assert!(keymap.conflicts(Layout::Mac).is_empty());
}
```

- [ ] **Step 4: Run** `~/.cargo/bin/cargo test --locked --lib keymap::tests`. Expected: PASS.

- [ ] **Step 5: Checks, then commit**

```bash
git add src/keymap.rs && git commit -m "Ask the keymap which keys two commands would both answer

The rule the defaults are held to is the one a rebind is checked by, so
it is the module's now and the test asks it.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 3: The user's lines over the defaults

**Files:** Create `src/keymap/overrides.rs`. Modify `src/keymap.rs` (`mod overrides; pub use overrides::{…};`, derives, interning).

- [ ] **Step 1: Write the failing tests** (`#[cfg(test)] mod tests` at the foot of `src/keymap/overrides.rs`)

```rust
use super::*;
use crate::keymap::{Command, Keymap, Layout, Scope};

fn bind(scope: Scope, keys: &str, command: Command) -> Bind {
    Bind { scope, keys: keys.to_owned(), command }
}

fn over(binds: Vec<Bind>) -> Overrides {
    Overrides { binds, unbinds: Vec::new() }
}

/// The keys `command` answers in `scope`, as spelled.
fn keys(keymap: &Keymap, layout: Layout, command: Command, scope: Scope) -> Vec<&'static str> {
    keymap
        .of(command)
        .filter(|binding| binding.scopes.contains(&scope))
        .flat_map(|binding| binding.bound(layout).chords.iter())
        .filter(|chord| chord.kind != crate::keymap::Kind::Via)
        .map(|chord| chord.keys)
        .collect()
}

#[test]
fn no_lines_leave_the_defaults() {
    for layout in Layout::ALL {
        let (keymap, refused) = Keymap::with(layout, &Overrides::default());
        assert!(refused.is_empty());
        assert_eq!(keymap.bindings().len(), Keymap::default().bindings().len());
    }
}

#[test]
fn a_command_has_exactly_the_keys_its_lines_give_it_in_that_scope() {
    let lines = over(vec![bind(Scope::Grid, "X", Command::SetNull)]);
    let (keymap, refused) = Keymap::with(Layout::Omarchy, &lines);
    assert!(refused.is_empty(), "{refused:?}");
    assert_eq!(keys(&keymap, Layout::Omarchy, Command::SetNull, Scope::Grid), ["X"]);
    // The inspector's is another scope of the same line of the keymap.
    assert_eq!(keys(&keymap, Layout::Omarchy, Command::SetNull, Scope::Inspector), ["x"]);
    // The Mac's column is not the look's.
    assert_eq!(keys(&keymap, Layout::Mac, Command::SetNull, Scope::Grid), ["cmd+backspace"]);
    assert!(keymap.conflicts(Layout::Omarchy).is_empty());
}

#[test]
fn two_lines_that_exchange_two_keys_are_both_taken() {
    let lines = over(vec![
        bind(Scope::Grid, "D", Command::DeleteRow),
        bind(Scope::Grid, "dd", Command::SetDefault),
    ]);
    let (keymap, refused) = Keymap::with(Layout::Omarchy, &lines);
    assert!(refused.is_empty(), "{refused:?}");
    assert_eq!(keys(&keymap, Layout::Omarchy, Command::DeleteRow, Scope::Grid), ["D"]);
    assert_eq!(keys(&keymap, Layout::Omarchy, Command::SetDefault, Scope::Grid), ["dd"]);
}

#[test]
fn a_line_on_another_commands_key_gives_way_and_says_whose() {
    let lines = over(vec![bind(Scope::Grid, "D", Command::DeleteRow)]);
    let (keymap, refused) = Keymap::with(Layout::Omarchy, &lines);
    assert_eq!(
        refused,
        [Refused { line: Line::Bind(0), why: Why::Taken { by: Command::SetDefault, scope: Scope::Grid } }]
    );
    assert_eq!(keys(&keymap, Layout::Omarchy, Command::DeleteRow, Scope::Grid), ["dd"]);
    // A key that another's begins with is taken too.
    let lines = over(vec![bind(Scope::Grid, "g", Command::SetNull)]);
    let (_, refused) = Keymap::with(Layout::Omarchy, &lines);
    assert!(matches!(refused[..], [Refused { why: Why::Taken { .. }, .. }]), "{refused:?}");
    // And one of a scope that is read alongside: the global one.
    let lines = over(vec![bind(Scope::SqlEditor, "ctrl+t", Command::RunAll)]);
    let (_, refused) = Keymap::with(Layout::Omarchy, &lines);
    assert_eq!(
        refused[0].why,
        Why::Taken { by: Command::NewSqlTab, scope: Scope::Global }
    );
}

#[test]
fn of_two_lines_on_one_key_the_later_gives_way() {
    let lines = over(vec![
        bind(Scope::Grid, "ctrl+g", Command::FilterBar),
        bind(Scope::Grid, "ctrl+g", Command::ToggleInspector),
    ]);
    let (_, refused) = Keymap::with(Layout::Omarchy, &lines);
    assert_eq!(
        refused,
        [Refused { line: Line::Bind(1), why: Why::Taken { by: Command::FilterBar, scope: Scope::Grid } }]
    );
}

#[test]
fn a_line_that_cannot_be_is_refused_with_its_reason() {
    let why = |layout, scope, keys: &str, command| {
        let (_, refused) = Keymap::with(layout, &over(vec![bind(scope, keys, command)]));
        refused.first().map(|refused| refused.why)
    };
    use Layout::{Mac, Omarchy};
    assert_eq!(why(Omarchy, Scope::Grid, "ctrl+", Command::DeleteRow), Some(Why::Unread));
    assert_eq!(why(Omarchy, Scope::Grid, "J", Command::MoveRow), Some(Why::Fixed));
    assert_eq!(why(Mac, Scope::Global, "cmd+;", Command::Settings), Some(Why::Fixed));
    assert_eq!(why(Omarchy, Scope::Prompt, "q", Command::SettingsClose), Some(Why::Fixed));
    assert_eq!(why(Omarchy, Scope::Sidebar, "X", Command::DeleteRow), Some(Why::Elsewhere));
    // Typed letters stay typed letters, a chord a chord.
    assert_eq!(why(Omarchy, Scope::Grid, "ctrl+d", Command::DeleteRow), Some(Why::Kind));
    assert_eq!(why(Mac, Scope::Grid, "X", Command::DeleteRow), Some(Why::Kind));
    assert_eq!(why(Omarchy, Scope::Grid, "ctrl+alt+w", Command::SaveChanges), None);
    assert_eq!(why(Omarchy, Scope::Grid, ":x", Command::SaveChanges), None);
    // Built, and with no key in this look: nothing there reads one.
    assert_eq!(why(Omarchy, Scope::Grid, "ctrl+y", Command::CopyRow), Some(Why::Kind));
    // Not built: the key is only claimed, and a chord can be.
    assert_eq!(why(Mac, Scope::SqlEditor, "cmd+alt+x", Command::ExplainAnalyze), None);
    // Vim's own, where text is typed.
    assert_eq!(why(Omarchy, Scope::SqlEditor, "ctrl+r", Command::RunAll), Some(Why::Vims));
    // Out of a text field it is nobody's but the keymap's.
    assert_eq!(why(Omarchy, Scope::Global, "ctrl+u", Command::CloseTab), None);
    assert_eq!(why(Omarchy, Scope::Global, "cmd+t", Command::NewSqlTab), Some(Why::Unread));
    // Read only while the pending SQL shows: the panel's own.
    assert_eq!(why(Omarchy, Scope::Grid, "q", Command::CloseReview), Some(Why::Fixed));
    assert_eq!(why(Mac, Scope::SqlEditor, "cmd+j", Command::NextCompletion), Some(Why::Fixed));
    // The inspector's bracket is its own there, and fixed from the rows.
    assert_eq!(why(Omarchy, Scope::Inspector, "{", Command::PreviousRow), None);
    assert_eq!(why(Omarchy, Scope::Grid, "{", Command::PreviousRow), Some(Why::Fixed));
    // Read a character at a time: one letter.
    assert_eq!(why(Omarchy, Scope::Sidebar, "gt", Command::ToggleTree), Some(Why::Kind));
    assert_eq!(why(Omarchy, Scope::Sidebar, "T", Command::ToggleTree), None);
    // The system's menu has these on the Mac, and only there.
    assert_eq!(why(Mac, Scope::Global, "cmd+q", Command::NewSqlTab), Some(Why::Menus));
    assert_eq!(why(Mac, Scope::Global, "cmd+alt+h", Command::NewSqlTab), Some(Why::Menus));
    assert_eq!(why(Layout::Windows, Scope::Global, "ctrl+q", Command::NewSqlTab), None);
}

#[test]
fn a_chord_of_the_sidebar_gives_way_to_one_the_grid_reads_from_anywhere() {
    // The filter bar's chord is read with the keyboard on the tree too.
    let lines = over(vec![bind(Scope::Sidebar, "cmd+f", Command::OpenObject)]);
    let (_, refused) = Keymap::with(Layout::Mac, &lines);
    assert_eq!(
        refused,
        [Refused { line: Line::Bind(0), why: Why::Taken { by: Command::FilterBar, scope: Scope::Grid } }]
    );
    // So is a SQL editor's, whose tab can be in front of the tree.
    let lines = over(vec![bind(Scope::Sidebar, "cmd+enter", Command::OpenObject)]);
    let (_, refused) = Keymap::with(Layout::Mac, &lines);
    assert_eq!(
        refused[0].why,
        Why::Taken { by: Command::RunStatement, scope: Scope::SqlEditor }
    );
    // One the grid reads only from its rows is free on the tree.
    let lines = over(vec![bind(Scope::Sidebar, "f2", Command::OpenObject)]);
    assert!(Keymap::with(Layout::Mac, &lines).1.is_empty());
}

#[test]
fn an_unbound_key_is_nobodys() {
    let lines = Overrides {
        binds: Vec::new(),
        unbinds: vec![
            Unbind { scope: Scope::Grid, keys: "gd".to_owned() },
            Unbind { scope: Scope::Grid, keys: "zz".to_owned() },
            Unbind { scope: Scope::Grid, keys: "j".to_owned() },
        ],
    };
    let (keymap, refused) = Keymap::with(Layout::Omarchy, &lines);
    assert!(keys(&keymap, Layout::Omarchy, Command::OpenReferencedRow, Scope::Grid).is_empty());
    assert_eq!(
        refused,
        [
            Refused { line: Line::Unbind(1), why: Why::Nobody },
            Refused { line: Line::Unbind(2), why: Why::Fixed },
        ]
    );
    // Free now, it can be another's.
    let lines = Overrides {
        binds: vec![bind(Scope::Grid, "gd", Command::SetNull)],
        unbinds: vec![Unbind { scope: Scope::Grid, keys: "gd".to_owned() }],
    };
    assert!(Keymap::with(Layout::Omarchy, &lines).1.is_empty());
}

#[test]
fn a_key_written_beside_a_command_goes_when_its_own_command_loses_it() {
    // `ctrl+l` is written beside Open inspector because it is Pane to
    // the right's.
    let lines = over(vec![bind(Scope::Global, "ctrl+alt+l", Command::PaneRight)]);
    let (keymap, refused) = Keymap::with(Layout::Omarchy, &lines);
    assert!(refused.is_empty(), "{refused:?}");
    let label = keymap.label(Layout::Omarchy, Command::OpenInspector);
    assert!(!label.contains("ctrl+l"), "{label}");
}

#[test]
fn windows_reads_ctrl_for_the_macs_cmd() {
    let lines = over(vec![bind(Scope::Global, "ctrl+shift+t", Command::NewSqlTab)]);
    let (keymap, refused) = Keymap::with(Layout::Windows, &lines);
    assert!(refused.is_empty(), "{refused:?}");
    assert_eq!(keymap.label(Layout::Windows, Command::NewSqlTab), "Ctrl+Shift+T");
    // Ctrl+T is the default there, spelled `cmd+t`: unbinding it by the
    // name it has on that keyboard finds it.
    let lines = Overrides {
        binds: Vec::new(),
        unbinds: vec![Unbind { scope: Scope::Global, keys: "ctrl+t".to_owned() }],
    };
    let (keymap, refused) = Keymap::with(Layout::Windows, &lines);
    assert!(refused.is_empty(), "{refused:?}");
    assert!(keymap.label(Layout::Windows, Command::NewSqlTab).is_empty());
}
```

If a key these tests bind is not free in the tree as it stands (`ctrl+g`, `ctrl+alt+w`, `ctrl+alt+l`, `ctrl+alt+x`, `ctrl+y`), take another that is and say so in the commit; the assertions are about the reasons, not those keys. `"Ctrl+Shift+T"` is as `written(Layout::Windows, …)` writes it: read `written_mods` and match it.

- [ ] **Step 2: Run them.** `~/.cargo/bin/cargo test --locked --lib keymap::overrides`. Expected: does not compile.

- [ ] **Step 3: Implement.**

In `src/keymap.rs`: `mod overrides;` and `pub use overrides::{Bind, Line, Overrides, Refused, Unbind, Why};`. Derive `Hash` on `Kind` and `Chord`, `PartialEq` on `Binding`, and add the two pools:

```rust
/// A spelling the user wrote, kept for as long as the app runs: the
/// keymap's spellings are `'static`, and a user's stands among them. Each
/// one is kept once, however often the file is read; a settings file is a
/// few kilobytes of them at most.
pub(super) fn kept(keys: &str) -> &'static str {
    static POOL: LazyLock<Mutex<HashSet<&'static str>>> = LazyLock::new(Mutex::default);
    let mut pool = POOL.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    if let Some(known) = pool.get(keys) {
        return known;
    }
    let kept: &'static str = Box::leak(keys.to_owned().into_boxed_str());
    pool.insert(kept);
    kept
}

/// The keys of a line the user changed, kept as [`kept`] keeps a spelling.
pub(super) fn kept_chords(chords: Vec<Chord>) -> &'static [Chord] {
    static POOL: LazyLock<Mutex<HashSet<&'static [Chord]>>> = LazyLock::new(Mutex::default);
    let mut pool = POOL.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    if let Some(known) = pool.get(chords.as_slice()) {
        return known;
    }
    let kept: &'static [Chord] = Box::leak(chords.into_boxed_slice());
    pool.insert(kept);
    kept
}

/// The one-scope slice of `scope`.
pub(super) fn only(scope: Scope) -> &'static [Scope] {
    match scope {
        Scope::Global => GLOBAL,
        Scope::Connections => CONNECTIONS,
        Scope::Sidebar => SIDEBAR,
        Scope::Grid => GRID,
        Scope::CellEditor => CELL_EDITOR,
        Scope::Inspector => INSPECTOR,
        Scope::SqlEditor => SQL_EDITOR,
        Scope::Results => RESULTS,
        Scope::Prompt => PROMPT,
    }
}
```

(`use std::collections::HashSet; use std::sync::{LazyLock, Mutex};`. A child module reaches its parent's private items, so `overrides.rs` uses `super::{k, kept, kept_chords, only, clash, spell, BINDINGS, …}`; `k` needs no `pub`.)

`src/keymap/overrides.rs`:

```rust
//! What the user changed of the keymap: the lines of `[keys.<scope>]` and
//! `[keys.unbind]` in `settings.toml`, laid over the defaults.
//!
//! A command named in a scope has exactly the keys its lines give it
//! there; a command with no line has its defaults. A line that cannot
//! stand is left out and said, with its reason: the file pane draws it
//! red, the Keyboard page says the reason where it was asked for.

use egui::Key;

use super::{
    BINDINGS, Binding, Chord, Command, Entry, Keymap, Kind, Layout, Mode, Mods, Press, Scope,
    Spelled, Stroke, clash, k, kept, kept_chords, only, spell,
};

/// `"keys" = "command"` under `[keys.<scope>]`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Bind {
    pub scope: Scope,
    pub keys: String,
    pub command: Command,
}

/// One key of `<scope> = [..]` under `[keys.unbind]`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Unbind {
    pub scope: Scope,
    pub keys: String,
}

/// The user's lines, in the file's order.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Overrides {
    pub binds: Vec<Bind>,
    pub unbinds: Vec<Unbind>,
}

impl Overrides {
    pub fn is_empty(&self) -> bool {
        self.binds.is_empty() && self.unbinds.is_empty()
    }
}

/// A line of [`Overrides`], by where it is in its list.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Line {
    Bind(usize),
    Unbind(usize),
}

/// Why a line does not stand.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Why {
    /// The keys are no spelling, or name a key the look has not.
    Unread,
    /// The command's keys are not the user's to change.
    Fixed,
    /// The command is not that scope's.
    Elsewhere,
    /// The command takes no key pressed this way there.
    Kind,
    /// Vim's own key where text is typed.
    Vims,
    /// A key the Mac's menu bar has: it never arrives.
    Menus,
    /// `by` has the keys in `scope`, where both could be read.
    Taken { by: Command, scope: Scope },
    /// No command has the key that was to be unbound.
    Nobody,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Refused {
    pub line: Line,
    pub why: Why,
}

/// The default line of `command` that holds in `scope`. One at most: the
/// keymap's tests see to it.
fn line_of(command: Command, scope: Scope) -> Option<&'static Binding> {
    BINDINGS
        .iter()
        .find(|binding| binding.command == command && binding.scopes.contains(&scope))
}

/// How the keys `line` has in `layout` are pressed: what its command can
/// take there. A command that is not built answers nothing yet, so where
/// it has no key a chord can be claimed for it.
fn takes(layout: Layout, line: &Binding) -> Vec<Press> {
    let mut takes: Vec<Press> = Vec::new();
    for chord in line.bound(layout).chords {
        if !its_own(line, chord) {
            continue;
        }
        if let Ok(spelled) = spell(chord.keys)
            && !takes.contains(&spelled.press())
        {
            takes.push(spelled.press());
        }
    }
    if takes.is_empty() && !line.command.info().built {
        takes.push(Press::Chord);
    }
    takes
}

/// Whether `spelled` is one of the keys vim has in insert mode, which the
/// text keeps wherever it is typed.
fn vims(spelled: &Spelled) -> bool {
    let Spelled::Strokes(strokes) = spelled else {
        return false;
    };
    let ctrl = Mods { ctrl: true, ..Mods::default() };
    strokes.iter().any(|stroke| {
        matches!(
            stroke,
            Stroke::Key { mods, key: Key::W | Key::H | Key::U | Key::R } if *mods == ctrl
        )
    })
}

/// Whether `spelled` is a key of the Mac's menu bar, which takes it
/// before the window sees it: Hide, Hide Others, Quit, and Settings.
fn menus(spelled: &Spelled) -> bool {
    let Spelled::Strokes(strokes) = spelled else {
        return false;
    };
    let cmd = Mods { cmd: true, ..Mods::default() };
    let cmd_alt = Mods { alt: true, ..cmd };
    strokes.iter().any(|stroke| match stroke {
        Stroke::Key { mods, key } => {
            (*mods == cmd && matches!(key, Key::H | Key::Q | Key::Comma))
                || (*mods == cmd_alt && *key == Key::H)
        }
        Stroke::Text(_) => false,
    })
}

/// Whether the user can change the keys of `line`: its command's can be,
/// and the line is not one that holds only while something else shows.
/// Such a line can share its key with another of its scope (`esc` closes
/// the pending SQL, the inspector and a note, each while it shows), which
/// the file, with one command to a key, could not write back.
pub(super) fn rebindable(layout: Layout, line: &Binding) -> bool {
    line.command.rebindable(layout) && line.when == super::When::Always
}

/// Whether `spelled` names the Mac's command key.
fn names_cmd(spelled: &Spelled) -> bool {
    match spelled {
        Spelled::Digits(mods) => mods.cmd,
        Spelled::Strokes(strokes) => strokes
            .iter()
            .any(|stroke| matches!(stroke, Stroke::Key { mods, .. } if mods.cmd)),
        Spelled::Ex(_) => false,
    }
}

impl Bind {
    /// Whether the line can stand at all, before it is laid beside the
    /// others.
    fn check(&self, layout: Layout) -> Result<(), Why> {
        let spelled = spell(&self.keys).map_err(|_| Why::Unread)?;
        // Omarchy has no key for `cmd` but Super, which Hyprland owns.
        if layout == Layout::Omarchy && names_cmd(&spelled) {
            return Err(Why::Unread);
        }
        if !self.command.rebindable(layout) {
            return Err(Why::Fixed);
        }
        let line = line_of(self.command, self.scope).ok_or(Why::Elsewhere)?;
        if !rebindable(layout, line) {
            return Err(Why::Fixed);
        }
        if !takes(layout, line).contains(&spelled.press()) {
            return Err(Why::Kind);
        }
        // Read a character at a time, before the letters of several are.
        let one_letter = matches!(
            self.command,
            Command::Reload | Command::FindObject | Command::ToggleTree
        );
        if one_letter && matches!(&spelled, Spelled::Strokes(strokes) if spelled.press() == Press::Typed && strokes.len() > 1)
        {
            return Err(Why::Kind);
        }
        if layout == Layout::Mac && menus(&spelled) {
            return Err(Why::Menus);
        }
        let insert = line.mode(self.scope, layout, &spelled).overlaps(Mode::Insert);
        if layout == Layout::Omarchy && insert && vims(&spelled) {
            return Err(Why::Vims);
        }
        Ok(())
    }
}

/// Whether two spellings are the same presses in `layout`: Windows has
/// one Ctrl where the Mac has two keys.
fn same(layout: Layout, a: &str, b: &str) -> bool {
    match (spell(a), spell(b)) {
        (Ok(Spelled::Ex(a)), Ok(Spelled::Ex(b))) => a == b,
        (Ok(a), Ok(b)) => a.press() == b.press() && presses(layout, &a) == presses(layout, &b),
        _ => false,
    }
}

/// Whether `chord` is a key of `binding`'s command to change: not one
/// written beside it that is another's, and not one only claimed for a
/// command that answers its others (`:wq` in the large editor), which is
/// offered nowhere.
fn its_own(binding: &Binding, chord: &Chord) -> bool {
    match chord.kind {
        Kind::Own => true,
        Kind::Via => false,
        Kind::Claimed => !binding.command.info().built,
    }
}

/// The defaults with the lines that are `live` laid over them: every line
/// of the keymap apart by scope, so one scope's keys can differ.
fn build(layout: Layout, overrides: &Overrides, live: &[bool], unbound: &[bool]) -> Keymap {
    let mut bindings: Vec<Binding> = BINDINGS
        .iter()
        .flat_map(|binding| {
            binding.scopes.iter().map(|scope| Binding {
                scopes: only(*scope),
                ..*binding
            })
        })
        .collect();
    let set = |binding: &mut Binding, chords: Vec<Chord>| {
        let bound = match layout {
            Layout::Mac | Layout::Windows => &mut binding.mac,
            Layout::Omarchy => &mut binding.omarchy,
        };
        if bound.chords != chords.as_slice() {
            bound.chords = kept_chords(chords);
            // What the keys were written as together is not these keys.
            bound.shown = "";
        }
    };
    for (unbind, _) in overrides.unbinds.iter().zip(unbound).filter(|(_, live)| **live) {
        for binding in &mut bindings {
            if binding.scopes != only(unbind.scope) || !rebindable(layout, binding) {
                continue;
            }
            let chords = binding.bound(layout).chords;
            let left: Vec<Chord> = chords
                .iter()
                .copied()
                .filter(|chord| chord.kind == Kind::Via || !same(layout, chord.keys, &unbind.keys))
                .collect();
            set(binding, left);
        }
    }
    for binding in &mut bindings {
        let scope = binding.scopes[0];
        let lines: Vec<&Bind> = overrides
            .binds
            .iter()
            .zip(live)
            .filter(|(bind, live)| **live && bind.scope == scope && bind.command == binding.command)
            .map(|(bind, _)| bind)
            .collect();
        if lines.is_empty() {
            continue;
        }
        // The keys written beside the command that are another's stay
        // where they were, before its own or after.
        let old = binding.bound(layout).chords;
        let via_first = old.first().is_some_and(|chord| chord.kind == Kind::Via);
        let vias = old.iter().copied().filter(|chord| chord.kind == Kind::Via);
        let own = lines.iter().map(|bind| k(kept(&bind.keys)));
        let chords: Vec<Chord> = if via_first {
            vias.chain(own).collect()
        } else {
            own.chain(vias).collect()
        };
        set(binding, chords);
    }
    // A key written beside a command is another command's own, in a scope
    // read alongside: one that nobody has there any more is not written.
    let owned: Vec<(Scope, &'static str)> = bindings
        .iter()
        .flat_map(|binding| {
            let scope = binding.scopes[0];
            let chords = binding.bound(layout).chords.iter();
            chords
                .filter(|chord| chord.kind != Kind::Via)
                .map(move |chord| (scope, chord.keys))
        })
        .collect();
    for binding in &mut bindings {
        let scope = binding.scopes[0];
        let chords = binding.bound(layout).chords;
        let left: Vec<Chord> = chords
            .iter()
            .copied()
            .filter(|chord| {
                chord.kind != Kind::Via
                    || owned
                        .iter()
                        .any(|(other, keys)| scope.alongside(*other) && *keys == chord.keys)
            })
            .collect();
        set(binding, left);
    }
    Keymap { bindings }
}

impl Keymap {
    /// The keymap `layout` has with the user's lines over the defaults,
    /// and the lines that do not stand, each with its reason, in the
    /// order they were found. A line on a key another command has gives
    /// way: to a default, and to a line before it.
    pub fn with(layout: Layout, overrides: &Overrides) -> (Keymap, Vec<Refused>) {
        if overrides.is_empty() {
            return (Keymap::default(), Vec::new());
        }
        let mut refused = Vec::new();
        let mut live: Vec<bool> = Vec::new();
        for (index, bind) in overrides.binds.iter().enumerate() {
            let checked = bind.check(layout);
            if let Err(why) = checked {
                refused.push(Refused { line: Line::Bind(index), why });
            }
            live.push(checked.is_ok());
        }
        let mut unbound: Vec<bool> = Vec::new();
        for (index, unbind) in overrides.unbinds.iter().enumerate() {
            let holders: Vec<&Binding> = BINDINGS
                .iter()
                .filter(|binding| binding.scopes.contains(&unbind.scope))
                .filter(|binding| {
                    binding.bound(layout).chords.iter().any(|chord| {
                        chord.kind != Kind::Via && same(layout, chord.keys, &unbind.keys)
                    })
                })
                .collect();
            let why = if holders.is_empty() {
                Some(Why::Nobody)
            } else if holders.iter().all(|binding| !rebindable(layout, binding)) {
                Some(Why::Fixed)
            } else {
                None
            };
            if let Some(why) = why {
                refused.push(Refused { line: Line::Unbind(index), why });
            }
            unbound.push(why.is_none());
        }
        // A line that gives way can hand its command back a default that
        // another line is on: so until nothing gives way any more. Each
        // turn takes a line out, so there are no more turns than lines.
        loop {
            let keymap = build(layout, overrides, &live, &unbound);
            let Some((index, why)) = gives_way(layout, &keymap, overrides, &live) else {
                return (keymap, refused);
            };
            live[index] = false;
            refused.push(Refused { line: Line::Bind(index), why });
        }
    }
}

/// The line that must give way in `keymap`, if one must: of the first two
/// keys that two commands would both answer, the user's line, and of two
/// of those the later.
fn gives_way(
    layout: Layout,
    keymap: &Keymap,
    overrides: &Overrides,
    live: &[bool],
) -> Option<(usize, Why)> {
    let line = |entry: &Entry| {
        overrides.binds.iter().enumerate().position(|(index, bind)| {
            live[index]
                && bind.scope == entry.scope
                && bind.command == entry.command
                && kept(&bind.keys) == entry.keys
        })
    };
    let taken = |by: &Entry| Why::Taken { by: by.command, scope: by.scope };
    for (a, b) in keymap.conflicts(layout) {
        match (line(&a), line(&b)) {
            (Some(first), Some(second)) if first > second => return Some((first, taken(&b))),
            (_, Some(second)) => return Some((second, taken(&a))),
            (Some(first), None) => return Some((first, taken(&b))),
            // Two defaults: the keymap's own tests would have failed.
            (None, None) => {}
        }
    }
    // The chords of a tab that are read from anywhere in it (the grid's,
    // a SQL editor's, its result's) are read with the keyboard on the
    // tree too, though the tree's scope is never read with theirs
    // otherwise: a chord of the sidebar gives way to them. So it does to
    // the chord that cancels a query, which is asked for while one runs
    // wherever the keys are.
    let entries = keymap.entries(layout);
    for sidebar in entries.iter().filter(|entry| entry.scope == Scope::Sidebar) {
        let Some(index) = line(sidebar) else {
            continue;
        };
        if sidebar.spelled.press() != Press::Chord {
            continue;
        }
        let grids = entries.iter().find(|entry| {
            matches!(entry.scope, Scope::Grid | Scope::SqlEditor | Scope::Results)
                && (entry.mode == Mode::Any || entry.command == Command::CancelQuery)
                && entry.command != sidebar.command
                && clash(layout, &entry.spelled, &sidebar.spelled)
        });
        if let Some(grids) = grids {
            return Some((index, taken(grids)));
        }
    }
    None
}
```

`its_own` is the one place that says which chords a line of the file stands over: use it in `takes`, in the unbinding (`build` and `with`), and in Task 4's `keys_in` and `full`, in place of the `chord.kind != Kind::Via` the drafts here have. A claimed key of a built command is neither listed nor counted, and goes when the command gets lines of its own. Add `presses` to the `use super::{..}` list. The sidebar rule rests on what `src/ui/keys.rs` does today: the `.any()` chords of the tab in front are asked for whatever part of it has the keys (lines 522 to 541, 569, 598 to 655), and the chord that cancels a query by what is in front (608). If the handler changes, the rule follows it.

- [ ] **Step 4: Run** `~/.cargo/bin/cargo test --locked --lib keymap`. Expected: PASS.

- [ ] **Step 5: Checks, then commit**

```bash
git add src/keymap.rs src/keymap/overrides.rs && git commit -m "Lay the user's keys over the defaults, and say which do not stand

A command named in a scope has exactly the keys its lines give it there.
A line gives way when it cannot be read, names a command whose keys are
fixed, is pressed another way than the command is read, is one of vim's
insert keys, or is on a key another command has where both could be read.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 4: Rebind, swap, unbind, reset, and the page's lines

**Files:** Modify `src/keymap/overrides.rs`, `src/keymap.rs` (re-exports).

Pure functions from the lines the user has to the lines they will have. The Keyboard page calls nothing else.

- [ ] **Step 1: Write the failing tests** (same `mod tests`)

```rust
fn line(layout: Layout, command: Command, scope: Scope) -> PageLine {
    lines(&Keymap::default(), layout)
        .into_iter()
        .find(|line| line.command == command && line.scopes.contains(&scope))
        .expect("the command's line")
}

#[test]
fn a_free_key_is_bound_in_every_scope_of_the_line() {
    // Set NULL is one line for the grid and the inspector.
    let set_null = line(Layout::Omarchy, Command::SetNull, Scope::Grid);
    let Outcome::Bound(lines) = rebind(Layout::Omarchy, &Overrides::default(), &set_null, "X")
    else {
        panic!("X is free");
    };
    assert_eq!(
        lines.binds,
        [bind(Scope::Grid, "X", Command::SetNull), bind(Scope::Inspector, "X", Command::SetNull)]
    );
    assert!(Keymap::with(Layout::Omarchy, &lines).1.is_empty());
}

#[test]
fn a_recorded_key_stands_in_for_the_keys_pressed_its_way_and_keeps_the_others() {
    // Save changes is `:w` and `ctrl+s`: a chord replaces the chord.
    let save = line(Layout::Omarchy, Command::SaveChanges, Scope::Grid);
    let Outcome::Bound(lines) = rebind(Layout::Omarchy, &Overrides::default(), &save, "ctrl+alt+s")
    else {
        panic!("free");
    };
    assert_eq!(
        lines.binds,
        [
            bind(Scope::Grid, ":w", Command::SaveChanges),
            bind(Scope::Grid, "ctrl+alt+s", Command::SaveChanges),
        ]
    );
}

#[test]
fn a_taken_key_says_whose_it_is_and_offers_the_exchange() {
    let delete = line(Layout::Omarchy, Command::DeleteRow, Scope::Grid);
    let Outcome::Taken { by, scope, swap } =
        rebind(Layout::Omarchy, &Overrides::default(), &delete, "D")
    else {
        panic!("D is Set DEFAULT's");
    };
    assert_eq!((by, scope), (Command::SetDefault, Scope::Grid));
    let swap = swap.expect("the two can change places");
    let (keymap, refused) = Keymap::with(Layout::Omarchy, &swap);
    assert!(refused.is_empty(), "{refused:?}");
    assert_eq!(keymap.label(Layout::Omarchy, Command::DeleteRow), "D");
    // Set DEFAULT is the grid's and the inspector's: `dd` in both.
    assert_eq!(keymap.label(Layout::Omarchy, Command::SetDefault), "dd");
}

#[test]
fn a_command_with_no_key_takes_the_others_and_leaves_it_none() {
    // The canvas' own case: Explain analyze on the key of Compare with
    // previous run.
    let explain = line(Layout::Mac, Command::ExplainAnalyze, Scope::SqlEditor);
    let Outcome::Taken { by, swap, .. } =
        rebind(Layout::Mac, &Overrides::default(), &explain, "cmd+alt+e")
    else {
        panic!("taken");
    };
    assert_eq!(by, Command::ComparePreviousRun);
    let (keymap, refused) = Keymap::with(Layout::Mac, &swap.expect("a swap"));
    assert!(refused.is_empty(), "{refused:?}");
    assert_eq!(keymap.label(Layout::Mac, Command::ExplainAnalyze), "⌥⌘E");
    assert!(keymap.label(Layout::Mac, Command::ComparePreviousRun).is_empty());
}

#[test]
fn an_exchange_that_would_not_stand_is_not_offered() {
    // Switch tab is a range of digits: a chord cannot be its key.
    let new_tab = line(Layout::Omarchy, Command::NewSqlTab, Scope::Global);
    let Outcome::Taken { by, swap, .. } =
        rebind(Layout::Omarchy, &Overrides::default(), &new_tab, "alt+3")
    else {
        panic!("taken");
    };
    assert_eq!(by, Command::SwitchTab);
    assert!(swap.is_none());
}

#[test]
fn a_key_that_cannot_be_the_lines_is_refused_with_its_reason() {
    let delete = line(Layout::Omarchy, Command::DeleteRow, Scope::Grid);
    let refused = |keys| rebind(Layout::Omarchy, &Overrides::default(), &delete, keys);
    assert_eq!(refused("ctrl+d"), Outcome::Refused(Why::Kind));
    let rows = line(Layout::Omarchy, Command::MoveRow, Scope::Grid);
    assert!(!rows.rebindable);
    assert_eq!(
        rebind(Layout::Omarchy, &Overrides::default(), &rows, "J"),
        Outcome::Refused(Why::Fixed)
    );
    // A line read only while its panel shows is listed, and fixed.
    let review = line(Layout::Omarchy, Command::CloseReview, Scope::Grid);
    assert!(!review.rebindable);
}

#[test]
fn a_chord_on_a_digit_is_the_range_on_a_line_of_digits() {
    let tabs = line(Layout::Omarchy, Command::SwitchTab, Scope::Global);
    let Outcome::Bound(lines) = rebind(Layout::Omarchy, &Overrides::default(), &tabs, "ctrl+alt+3")
    else {
        panic!("free");
    };
    assert_eq!(lines.binds, [bind(Scope::Global, "ctrl+alt+1..9", Command::SwitchTab)]);
    assert_eq!(
        rebind(Layout::Omarchy, &Overrides::default(), &tabs, "ctrl+alt+x"),
        Outcome::Refused(Why::Kind)
    );
}

#[test]
fn a_reset_that_hands_back_a_taken_key_leaves_no_line_that_is_refused() {
    let delete = line(Layout::Omarchy, Command::DeleteRow, Scope::Grid);
    let Outcome::Taken { swap: Some(swapped), .. } =
        rebind(Layout::Omarchy, &Overrides::default(), &delete, "D")
    else {
        panic!("a swap");
    };
    let back = reset(Layout::Omarchy, &swapped, &delete);
    let (keymap, refused) = Keymap::with(Layout::Omarchy, &back);
    assert!(refused.is_empty(), "{refused:?}");
    assert_eq!(keymap.label(Layout::Omarchy, Command::DeleteRow), "dd");
}

#[test]
fn unbinding_leaves_the_line_no_key_and_resetting_gives_the_defaults_back() {
    let gd = line(Layout::Omarchy, Command::OpenReferencedRow, Scope::Grid);
    let none = unbind(Layout::Omarchy, &Overrides::default(), &gd);
    assert_eq!(none.unbinds, [Unbind { scope: Scope::Grid, keys: "gd".to_owned() }]);
    let (keymap, refused) = Keymap::with(Layout::Omarchy, &none);
    assert!(refused.is_empty());
    assert!(keymap.label(Layout::Omarchy, Command::OpenReferencedRow).is_empty());
    assert_eq!(reset(Layout::Omarchy, &none, &gd), Overrides::default());
    // A rebound line, unbound: its own lines go and its defaults too.
    let Outcome::Bound(moved) = rebind(Layout::Omarchy, &Overrides::default(), &gd, "gr") else {
        panic!("free");
    };
    let none = unbind(Layout::Omarchy, &moved, &gd);
    assert!(none.binds.is_empty());
    assert!(Keymap::with(Layout::Omarchy, &none).0.label(Layout::Omarchy, Command::OpenReferencedRow).is_empty());
    assert_eq!(reset(Layout::Omarchy, &moved, &gd), Overrides::default());
}

#[test]
fn a_line_that_was_refused_is_not_carried_into_the_next_lines() {
    let stale = over(vec![bind(Scope::Grid, "D", Command::DeleteRow)]);
    let gd = line(Layout::Omarchy, Command::OpenReferencedRow, Scope::Grid);
    let Outcome::Bound(lines) = rebind(Layout::Omarchy, &stale, &gd, "gr") else {
        panic!("free");
    };
    assert_eq!(lines.binds, [bind(Scope::Grid, "gr", Command::OpenReferencedRow)]);
}

#[test]
fn the_full_list_read_back_is_the_defaults() {
    for layout in Layout::ALL {
        let all = full(layout);
        let (keymap, refused) = Keymap::with(layout, &all);
        assert!(refused.is_empty(), "{layout:?}: {refused:?}");
        let defaults = Keymap::default();
        for command in Command::ALL {
            assert_eq!(
                keymap.label(layout, *command),
                defaults.label(layout, *command),
                "{layout:?}: {command:?}"
            );
        }
    }
}

#[test]
fn the_page_lists_every_command_of_the_settled_table_in_the_looks_groups() {
    use crate::keymap::Group;
    for layout in Layout::ALL {
        let lines = lines(&Keymap::default(), layout);
        for command in Command::ALL {
            let info = command.info();
            let listed = lines.iter().any(|line| line.command == *command);
            if info.group == Group::Prompts {
                assert!(!listed, "{command:?}");
            } else if !info.row.is_empty() {
                assert!(listed, "{layout:?}: {command:?}");
            }
        }
        // A row of the table with one command is named as the table has it.
        let named = |name: &str| lines.iter().any(|line| line.command.info().name == name);
        for row in ["Connections", "Find any object", "Run statement", "Set NULL"] {
            assert!(named(row), "{row}");
        }
        // Omarchy has no group of its own for the assistant.
        let groups: Vec<Group> = lines.iter().map(|line| line.group).collect();
        assert_eq!(groups.contains(&Group::Ai), layout != Layout::Omarchy);
        assert!(groups.is_sorted_by_key(|group| GROUPS.iter().position(|known| known == group)));
    }
    let changed = over(vec![bind(Scope::Grid, "X", Command::SetNull)]);
    let (keymap, _) = Keymap::with(Layout::Omarchy, &changed);
    let lines = lines(&keymap, Layout::Omarchy);
    let set_null = lines.iter().find(|line| line.command == Command::SetNull).expect("listed");
    assert!(set_null.changed);
    assert!(set_null.keys.contains(&"X") && set_null.keys.contains(&"x"), "{:?}", set_null.keys);
    assert!(lines.iter().filter(|line| line.changed).count() == 1);
}
```

(`is_sorted_by_key` on a `Vec` needs Rust 1.82: fine. If the assertion is clumsier than it is worth, compare against the positions collected and sorted.)

- [ ] **Step 2: Run them.** Expected: does not compile.

- [ ] **Step 3: Implement**, in `src/keymap/overrides.rs`:

```rust
/// The groups of the Keyboard page, in its order.
pub const GROUPS: [Group; 5] = [
    Group::Window,
    Group::Navigation,
    Group::Editing,
    Group::Sql,
    Group::Ai,
];

/// One line of the Keyboard page: a line of the keymap, as it is now.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PageLine {
    pub command: Command,
    /// The group the page puts it under in this look.
    pub group: Group,
    /// The scopes of the keymap's own line: a key recorded here is the
    /// command's in all of them.
    pub scopes: &'static [Scope],
    /// The command's keys now, in those scopes, as spelled: none that is
    /// another command's written beside it.
    pub keys: Vec<&'static str>,
    /// Whether they are other than the defaults.
    pub changed: bool,
    /// Whether the user can change them: not a fixed command's, and not
    /// a line that holds only while something else shows.
    pub rebindable: bool,
}

/// The keys `command` has in `scopes` of `keymap`, each once.
fn keys_in(keymap: &Keymap, layout: Layout, command: Command, scopes: &[Scope]) -> Vec<&'static str> {
    let mut keys = Vec::new();
    for binding in keymap.of(command) {
        if !binding.scopes.iter().any(|scope| scopes.contains(scope)) {
            continue;
        }
        for chord in binding.bound(layout).chords {
            if chord.kind != Kind::Via && !keys.contains(&chord.keys) {
                keys.push(chord.keys);
            }
        }
    }
    keys
}

/// The lines of the Keyboard page in `layout`: every command that is not a
/// prompt's and has a key in the look or a row of the settled table, by
/// the lines the defaults have, under the look's groups. Omarchy lists
/// the assistant's with the SQL editor's, as its design does.
pub fn lines(keymap: &Keymap, layout: Layout) -> Vec<PageLine> {
    let defaults = Keymap::default();
    let mut lines = Vec::new();
    for group in GROUPS {
        if layout == Layout::Omarchy && group == Group::Ai {
            continue;
        }
        for binding in BINDINGS {
            let info = binding.command.info();
            let here = info.group == group
                || (layout == Layout::Omarchy && group == Group::Sql && info.group == Group::Ai);
            if !here {
                continue;
            }
            let keys = keys_in(keymap, layout, binding.command, binding.scopes);
            let default = keys_in(&defaults, layout, binding.command, binding.scopes);
            if keys.is_empty() && default.is_empty() && info.row.is_empty() {
                continue;
            }
            // Per scope: a line whose scopes differ shows every key once,
            // and is changed if one scope's keys are.
            let changed = binding.scopes.iter().any(|scope| {
                keys_in(keymap, layout, binding.command, &[*scope])
                    != keys_in(&defaults, layout, binding.command, &[*scope])
            });
            lines.push(PageLine {
                command: binding.command,
                group,
                scopes: binding.scopes,
                keys,
                changed,
                rebindable: rebindable(layout, binding),
            });
        }
    }
    lines
}

/// What recording keys on a line comes to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Outcome {
    /// The keys are the line's: these are the user's lines now.
    Bound(Overrides),
    /// `by` has them in `scope`. `swap` are the lines that give `by` the
    /// keys this line had, where that can stand.
    Taken {
        by: Command,
        scope: Scope,
        swap: Option<Overrides>,
    },
    Refused(Why),
}

/// `current` without the lines that do not stand in `layout`: what the
/// app writes when the page changes a key, as every change made in the
/// app leaves the file without the lines it had ignored.
fn standing(layout: Layout, current: &Overrides) -> Overrides {
    let (_, refused) = Keymap::with(layout, current);
    let out = |line: Line| refused.iter().any(|refused| refused.line == line);
    Overrides {
        binds: current
            .binds
            .iter()
            .enumerate()
            .filter(|(index, _)| !out(Line::Bind(*index)))
            .map(|(_, bind)| bind.clone())
            .collect(),
        unbinds: current
            .unbinds
            .iter()
            .enumerate()
            .filter(|(index, _)| !out(Line::Unbind(*index)))
            .map(|(_, unbind)| unbind.clone())
            .collect(),
    }
}

/// `lines` with `command` having exactly `keys` in each of `scopes`. No
/// keys: its defaults there, with every one of them unbound.
fn set(
    layout: Layout,
    lines: &Overrides,
    command: Command,
    scopes: &[Scope],
    keys: &[String],
) -> Overrides {
    let mut out = cleared(layout, lines, command, scopes);
    for scope in scopes {
        if keys.is_empty() {
            for default in keys_in(&Keymap::default(), layout, command, &[*scope]) {
                out.unbinds.push(Unbind { scope: *scope, keys: default.to_owned() });
            }
        }
        for keys in keys {
            out.binds.push(Bind { scope: *scope, keys: keys.clone(), command });
        }
    }
    out
}

/// `lines` without what they say of `command` in `scopes`: its own lines,
/// and the unbinding of its default keys.
fn cleared(layout: Layout, lines: &Overrides, command: Command, scopes: &[Scope]) -> Overrides {
    let mut out = lines.clone();
    out.binds
        .retain(|bind| !(bind.command == command && scopes.contains(&bind.scope)));
    out.unbinds.retain(|unbind| {
        let defaults = keys_in(&Keymap::default(), layout, command, &[unbind.scope]);
        !(scopes.contains(&unbind.scope)
            && defaults.iter().any(|default| same(layout, default, &unbind.keys)))
    });
    out
}

/// `keys`, recorded on `line`, over the lines the user has. They stand
/// in for the line's keys that are pressed the same way; the others stay
/// (`:w` beside a new chord). On a line whose key is a range of digits a
/// chord on a digit is the range with what was held.
pub fn rebind(layout: Layout, current: &Overrides, line: &PageLine, keys: &str) -> Outcome {
    if !line.rebindable {
        return Outcome::Refused(Why::Fixed);
    }
    let ranged = ranged(line, keys);
    let keys = ranged.as_deref().unwrap_or(keys);
    let Ok(spelled) = spell(keys) else {
        return Outcome::Refused(Why::Unread);
    };
    let current = standing(layout, current);
    let (now, _) = Keymap::with(layout, &current);
    // What the line would have: its keys of other kinds, then the new one.
    let replaced = |command: Command, scopes: &[Scope], new: Option<&str>, press: Press| {
        let mut keys: Vec<String> = keys_in(&now, layout, command, scopes)
            .into_iter()
            .filter(|old| spell(old).is_ok_and(|old| old.press() != press))
            .map(str::to_owned)
            .collect();
        keys.extend(new.map(str::to_owned));
        keys
    };
    let press = spelled.press();
    let wanted = replaced(line.command, line.scopes, Some(keys), press);
    let bound = set(layout, &current, line.command, line.scopes, &wanted);
    let (_, refused) = Keymap::with(layout, &bound);
    let Some(why) = refused.first().map(|refused| refused.why) else {
        return Outcome::Bound(bound);
    };
    let Why::Taken { by, scope } = why else {
        return Outcome::Refused(why);
    };
    // The exchange: `by` gets the key this line had that is pressed the
    // same way, its first; with none, `by` is left without.
    let had = keys_in(&now, layout, line.command, line.scopes)
        .into_iter()
        .find(|old| spell(old).is_ok_and(|old| old.press() == press));
    let swap = line_of(by, scope).and_then(|theirs| {
        let theirs_now: Vec<String> = keys_in(&now, layout, by, theirs.scopes)
            .into_iter()
            .filter(|old| !spell(old).is_ok_and(|old| clash(layout, &old, &spelled)))
            .map(str::to_owned)
            .chain(had.map(str::to_owned))
            .collect();
        let swapped = set(layout, &bound, by, theirs.scopes, &theirs_now);
        Keymap::with(layout, &swapped).1.is_empty().then_some(swapped)
    });
    Outcome::Taken { by, scope, swap }
}

/// `keys` as a range of digits, where `line` has one and `keys` is a chord
/// on a digit: nobody presses nine keys to record one.
fn ranged(line: &PageLine, keys: &str) -> Option<String> {
    let ranges = line
        .keys
        .iter()
        .any(|old| spell(old).is_ok_and(|old| old.press() == Press::Digits));
    let Ok(Spelled::Strokes(strokes)) = spell(keys) else {
        return None;
    };
    match strokes.as_slice() {
        [Stroke::Key { mods, key }] if ranges && key.name().chars().all(|char| char.is_ascii_digit()) => {
            // `ctrl+shift+1`, with the digit made the range.
            let one = super::spelling(*mods, *key)?;
            let held = &one[..one.len() - key.name().len()];
            Some(format!("{held}1..9"))
        }
        _ => None,
    }
}

/// The user's lines with `line` left no key.
pub fn unbind(layout: Layout, current: &Overrides, line: &PageLine) -> Overrides {
    set(layout, &standing(layout, current), line.command, line.scopes, &[])
}

/// The user's lines with `line` at its defaults again.
pub fn reset(layout: Layout, current: &Overrides, line: &PageLine) -> Overrides {
    let cleared = cleared(layout, &standing(layout, current), line.command, line.scopes);
    // A default that comes back can be on a key a line of the user's
    // took meanwhile: that line gives way, and is not written.
    standing(layout, &cleared)
}

/// Every key of `layout` that can be changed, as lines, by scope as the
/// file has them: what `:keys default` writes.
pub fn full(layout: Layout) -> Overrides {
    let mut all = Overrides::default();
    for scope in Scope::ALL {
        for binding in BINDINGS {
            if !binding.scopes.contains(&scope) || !rebindable(layout, binding) {
                continue;
            }
            for chord in binding.bound(layout).chords {
                if its_own(binding, chord) {
                    all.binds.push(Bind {
                        scope,
                        keys: chord.keys.to_owned(),
                        command: binding.command,
                    });
                }
            }
        }
    }
    all
}
```

Three things to get right while the tests drive this:

1. `set` with `keys` equal to the command's defaults must come out as `cleared` (no lines): `reset(rebind(…))` and the "changed" dot depend on it. Compare with `keys_in(&Keymap::default(), …)` per scope before pushing.
2. In `rebind`, `refused.first()` is this rebind's own line: `current` is `standing`, so nothing in it was refused before, and a new line is the later one beside any older line and gives way to it (`Taken { by: the older line's command }`). Do not sort `bound` before asking: `set` appends, so the new line is the later one whatever its scope, and it is the one that gives way. `Bound` and `swap` are returned only with nothing refused, where the order of their lines changes nothing, and the store puts them in the file's order when it writes them (Task 5).
3. `the_full_list_read_back_is_the_defaults` compares labels; a line with `shown` (`j/k`) is a fixed command and is not in `full`, so `build` leaves it alone. If a rebindable line has `shown`, `set` in `build` must not clear it when the chords are the defaults: the `!=` there sees to it.

Re-export from `src/keymap.rs`: `pub use overrides::{GROUPS as PAGE_GROUPS, Outcome, PageLine, full, lines as page_lines, rebind, reset, unbind};` and use those names from outside.

- [ ] **Step 4: Run** `~/.cargo/bin/cargo test --locked --lib keymap`. Expected: PASS.

- [ ] **Step 5: Checks, then commit**

```bash
git add src/keymap.rs src/keymap/overrides.rs && git commit -m "Work out the lines a recorded key, an unbind and a reset leave

A recorded key stands in for the line's keys that are pressed its way.
On a key another command has it says whose, with the lines that would
exchange the two where those stand.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 5: The settings file holds the lines, and the app reads keys from them

**Files:** Modify `src/settings.rs`, `src/app.rs`, `src/testing.rs`, `src/ui/settings/file_pane.rs`.

- [ ] **Step 1: Write the failing tests.**

In `src/settings.rs` `mod tests`:

```rust
#[test]
fn the_keys_tables_are_read_in_the_files_order_and_written_back_the_same() {
    use crate::keymap::{Bind, Command, Overrides, Scope, Unbind};
    let text = "\
[data]
page_size = 500

[keys.sql-editor]
\"f5\" = \"run_all\"

[keys.grid]
\"dd\" = \"set_default\"
\"D\" = \"delete_row\"

[keys.unbind]
grid = [\"gd\", \"gs\"]
";
    let loaded = Settings::from_toml(text);
    assert!(loaded.invalid.is_empty(), "{:?}", loaded.invalid);
    assert_eq!(
        loaded.settings.keys,
        Overrides {
            // By scope, then as written: the order the app writes.
            binds: vec![
                Bind { scope: Scope::Grid, keys: "dd".into(), command: Command::SetDefault },
                Bind { scope: Scope::Grid, keys: "D".into(), command: Command::DeleteRow },
                Bind { scope: Scope::SqlEditor, keys: "f5".into(), command: Command::RunAll },
            ],
            unbinds: vec![
                Unbind { scope: Scope::Grid, keys: "gd".into() },
                Unbind { scope: Scope::Grid, keys: "gs".into() },
            ],
        }
    );
    assert_eq!(loaded.bind_lines, [8, 9, 5]);
    assert_eq!(loaded.unbind_lines, [12, 12]);
    // The app's own text: tables by scope, a table's keys aligned.
    let written = loaded.settings.to_toml();
    assert!(written.ends_with(
        "\n# only changes from the defaults live here\n\
         [keys.grid]\n\
         \"dd\" = \"set_default\"\n\
         \"D\"  = \"delete_row\"\n\
         \n\
         [keys.sql-editor]\n\
         \"f5\" = \"run_all\"\n\
         \n\
         [keys.unbind]\n\
         grid = [\"gd\", \"gs\"]\n"
    ), "{written}");
    let again = Settings::from_toml(&written);
    assert_eq!(again.settings.keys, loaded.settings.keys);
    // Lines made in the app, in whatever order, are held as the file
    // has them: a line's number is its own.
    let mut shuffled = loaded.settings.clone();
    // Across scopes: within one, the list's order is the file's.
    shuffled.keys.binds.rotate_left(2);
    let of = Loaded::of(shuffled, Source::Toml);
    assert_eq!(of.settings.keys, loaded.settings.keys);
    assert_eq!(of.bind_lines.len(), 3);
    assert!(of.bind_lines.is_sorted());
    assert_eq!(again.settings.to_toml(), written);
    // No keys, no tables: the file of someone who changed none is as it was.
    assert!(!Settings::default().to_toml().contains("keys"));
}

#[test]
fn a_keys_line_that_is_no_key_or_no_command_is_ignored_by_its_line() {
    let text = "\
[keys.grid]
\"ctrl+\" = \"delete_row\"
\"X\" = \"no_such_command\"
\"Y\" = 3
\"Z\" = \"set_null\"

[keys.unbind]
grid = \"gd\"
nowhere = [\"gd\"]

[keys.editor]
\"f5\" = \"run_all\"
";
    let loaded = Settings::from_toml(text);
    assert_eq!(loaded.invalid, [2, 3, 4, 8]);
    assert_eq!(loaded.settings.keys.binds.len(), 1);
    assert!(loaded.settings.keys.unbinds.is_empty());
    // A scope this version does not know is ignored without a word, as a
    // table it does not know is.
    assert_eq!(loaded.bind_lines, [5]);
}

#[test]
fn a_key_that_needs_quoting_is_written_so_that_it_reads_back() {
    use crate::keymap::{Bind, Command, Scope};
    let mut settings = Settings::default();
    settings.keys.binds.push(Bind {
        scope: Scope::Grid,
        keys: "\"+p".into(),
        command: Command::PasteNewRows,
    });
    let again = Settings::from_toml(&settings.to_toml());
    assert!(again.invalid.is_empty());
    assert_eq!(again.settings.keys, settings.keys);
}
```

In `src/app.rs` `mod tests` (beside `the_settings_file_is_watched_through_the_backend`; build the app the way those tests do):

```rust
#[test]
fn a_keys_line_in_the_file_changes_the_key_and_a_taken_one_is_marked() {
    use crate::keymap::{Command, Layout};
    let mut app = /* as the neighbouring tests make one */;
    app.look = crate::theme::Look::omarchy();
    let text = "[keys.grid]\n\"X\" = \"set_null\"\n\"D\" = \"delete_row\"\n".to_owned();
    app.apply(Action::Backend(Event::SettingsFile { text, own: false }));
    assert_eq!(app.keymap.label(Layout::Omarchy, Command::SetNull), "X, x");
    // `D` is Set DEFAULT's: the line is ignored, and the file says so.
    assert_eq!(app.keymap.label(Layout::Omarchy, Command::DeleteRow), "dd");
    assert_eq!(app.settings_file.invalid, [3]);
    assert_eq!(app.keys_refused.len(), 1);
    // Another look reads the same lines its own way.
    app.look = crate::theme::Look::macos();
    app.keymap_for_look();
    assert_eq!(app.keys_refused.len(), 2, "typed letters are not the Mac's");
    // The file without them: the defaults again.
    app.apply(Action::Backend(Event::SettingsFile { text: String::new(), own: false }));
    assert!(app.keys_refused.is_empty());
    assert_eq!(app.keymap.label(Layout::Mac, Command::SetNull), "⌘⌫");
}
```

(`"X, x"`: Set NULL is one line for the grid and the inspector, and only the grid's key changed. An event reaches the app as `Action::Backend`, as at `src/app.rs:4890`.)

In `src/ui/mod.rs` `mod tests`, the behaviour end to end:

```rust
#[test]
fn a_key_changed_in_the_file_is_the_one_that_answers() {
    let mut harness = Harness::new();
    harness.set_look(crate::theme::Look::omarchy());
    let (tab, id) = harness.editable();
    let text = "[keys.grid]\n\"X\" = \"set_null\"\n".to_owned();
    harness.app.apply(Action::Backend(crate::backend::Event::SettingsFile { text, own: false }));
    let pending = |harness: &Harness| {
        let object = harness.app.workspace(tab).unwrap().object_tab(id).unwrap();
        object.edits.pending()
    };
    // The old key is nobody's now.
    harness.press(egui::Key::X, egui::Modifiers::NONE);
    assert!(!pending(&harness));
    harness.press(egui::Key::X, egui::Modifiers::SHIFT);
    assert!(pending(&harness), "X sets the cell NULL");
}
```

(Select a nullable cell first if `editable()` leaves none selected: do what `x_sets_null…` in the tree does before its `x`.)

In `src/ui/settings/file_pane.rs` `mod tests`:

```rust
#[test]
fn a_quoted_key_is_a_key() {
    assert_eq!(
        parts("\"ctrl+d\" = \"set_default\"  # was D"),
        [
            ("\"ctrl+d\" = ", Part::Key),
            ("\"set_default\"", Part::String),
            ("  ", Part::Plain),
            ("# was D", Part::Comment),
        ]
    );
    // A quote left open is no key.
    assert_eq!(parts("\"ctrl+d = 1"), [("\"ctrl+d = 1", Part::Plain)]);
    // Its `=` is the one after the closing quote.
    assert_eq!(parts("\"=\" = \"format\"")[0], ("\"=\" = ", Part::Key));
}
```

- [ ] **Step 2: Run them.** Expected: do not compile.

- [ ] **Step 3: Implement the store** (`src/settings.rs`).

- `Settings` gets `#[serde(skip)] pub keys: crate::keymap::Overrides` (the old JSON has none), default empty.
- `Loaded` and `SettingsFile` get `pub bind_lines: Vec<usize>` and `pub unbind_lines: Vec<usize>`: the line of each of `settings.keys.binds` and `.unbinds`, in the same order. `Loaded::of` re-reads its text as it does for `lines`, and takes **the keys themselves** from that re-read as well as their lines (`settings.keys = reread.settings.keys`): the app's lists come in whatever order `rebind` appended them, and the file has them by scope. `into_parts` carries the lines.
- Reading, at the end of `from_toml` before `invalid.sort_unstable()`:

```rust
        let mut binds: Vec<(usize, crate::keymap::Bind)> = Vec::new();
        let mut unbinds: Vec<(usize, usize, crate::keymap::Unbind)> = Vec::new();
        let keys = document.get("keys").and_then(|keys| keys.get_ref().as_table());
        for (table, lines) in keys.into_iter().flatten() {
            let at = line_of(&working, table.span().start);
            let Some(lines) = lines.get_ref().as_table() else {
                invalid.push(at);
                continue;
            };
            for (name, value) in lines {
                let line = line_of(&working, name.span().start);
                if table.get_ref() == "unbind" {
                    // `<scope> = ["keys", ..]`. A scope this version does
                    // not know is a newer one's.
                    let Some(scope) = crate::keymap::Scope::from_id(name.get_ref()) else {
                        continue;
                    };
                    let listed: Option<Vec<String>> = value.get_ref().as_array().and_then(|keys| {
                        keys.iter()
                            .map(|key| key.get_ref().as_str().map(str::to_owned))
                            .collect()
                    });
                    match listed {
                        Some(listed) if listed.iter().all(|keys| crate::keymap::spell(keys).is_ok()) => {
                            for (index, keys) in listed.into_iter().enumerate() {
                                unbinds.push((line, index, crate::keymap::Unbind { scope, keys }));
                            }
                        }
                        _ => invalid.push(line),
                    }
                    continue;
                }
                let Some(scope) = crate::keymap::Scope::from_id(table.get_ref()) else {
                    continue;
                };
                let command = value
                    .get_ref()
                    .as_str()
                    .and_then(crate::keymap::Command::from_id);
                match command {
                    Some(command) if crate::keymap::spell(name.get_ref()).is_ok() => {
                        let keys = name.get_ref().to_string();
                        binds.push((line, crate::keymap::Bind { scope, keys, command }));
                    }
                    _ => invalid.push(line),
                }
            }
        }
        // By scope, then in the file's order, whatever order the parser's
        // table has: the order the app writes them in, so a line of the
        // list and its line of the text keep their places together.
        let place = |scope: crate::keymap::Scope| {
            crate::keymap::Scope::ALL.iter().position(|known| *known == scope)
        };
        binds.sort_by_key(|(line, bind)| (place(bind.scope), *line));
        unbinds.sort_by_key(|(line, index, unbind)| (place(unbind.scope), *line, *index));
        let bind_lines = binds.iter().map(|(line, _)| *line).collect();
        let unbind_lines = unbinds.iter().map(|(line, _, _)| *line).collect();
        settings.keys = crate::keymap::Overrides {
            binds: binds.into_iter().map(|(_, bind)| bind).collect(),
            unbinds: unbinds.into_iter().map(|(_, _, unbind)| unbind).collect(),
        };
```

  The `toml::de` names (`get_ref`, `as_table`, `as_array`, how a `DeTable` iterates, whether a key is a `Spanned<Cow<str>>`) are as the reader above this already uses them: follow it, and where an accessor has another name, the compiler says which. Two lines of one table on one line of text cannot be (TOML), so `(line)` orders binds; an inline table `keys.grid = { .. }` puts several on one line, which keeps their order by the stable sort.

- Writing: `pub fn keys_toml(keys: &crate::keymap::Overrides) -> String` (public: `:keys default` writes the same form), called at the end of `to_toml` when `!self.keys.is_empty()` after `out.push('\n'); out.push_str("# only changes from the defaults live here\n");`. For each scope in `Scope::ALL` order with binds: `[keys.<id>]`, then its lines in the list's order as `{quoted:width$} = {command}` with `quoted = quote(&bind.keys)`, `width` the widest quoted key of the table in **characters** (`chars().count()`; pad by hand, `{:width$}` counts characters too), a blank line between tables. Then `[keys.unbind]` with one line per scope in `Scope::ALL` order: `{id:width$} = ["a", "b"]`. Test 1 is the form.

- [ ] **Step 4: Implement the pane's quoted key** (`src/ui/settings/file_pane.rs`, in `spans`): before the bare-key rule, when the line's first non-blank character is `"`, find the closing quote as the value's is found (a backslash escapes the next character); if it closes and what follows it, after blanks, is `=`, the key runs to that `=` and the blanks after it, and the value is read as before. Factor the "where a quoted string closes" walk into one function used by both.

- [ ] **Step 5: Implement the app** (`src/app.rs`).

```rust
    /// The lines of the settings' keys that do not stand in the look, and
    /// why: the Keyboard page and the file pane say so.
    pub keys_refused: Vec<crate::keymap::Refused>,
    /// The look and the lines `keymap` was made from.
    keymap_of: Option<(crate::keymap::Layout, crate::keymap::Overrides)>,
```

```rust
    /// Makes the keymap the look and the settings' keys give, where either
    /// changed since it was made, and marks the lines that do not stand
    /// as ignored in the file. Called before a frame reads a key: the look
    /// is set at the start, and by the tests.
    pub fn keymap_for_look(&mut self) {
        let layout = self.layout();
        let current = (layout, self.settings.keys.clone());
        if self.keymap_of.as_ref() != Some(&current) {
            let (keymap, refused) = crate::keymap::Keymap::with(layout, &self.settings.keys);
            self.keymap = std::sync::Arc::new(keymap);
            self.keys_refused = refused;
            self.keymap_of = Some(current);
        }
        let file = &mut self.settings_file;
        for refused in &self.keys_refused {
            let line = match refused.line {
                crate::keymap::Line::Bind(index) => file.bind_lines.get(index),
                crate::keymap::Line::Unbind(index) => file.unbind_lines.get(index),
            };
            if let Some(line) = line.filter(|line| !file.invalid.contains(line)) {
                file.invalid.push(*line);
            }
        }
        file.invalid.sort_unstable();
    }
```

Call it: at the end of `App::new` (after the settings are loaded), at the end of `apply_settings`, and in `frame_ui` right before `crate::ui::keys::publish`. `Harness::set_look` calls it after setting the look. The clone per frame is of a list that is empty for nearly everyone; if a profile ever shows it, compare before cloning.

`apply_settings` runs after `self.settings_file` is replaced, in both `Event::SettingsFile` and `change_settings` (`src/app.rs:2547` and `2770`), so the marks land on the new file's lines.

`Reload` is one line for the grid and the sidebar, and its chord is asked for in the grid's scope alone (`src/ui/keys.rs:598`). Ask in `Scope::Grid` as now, and in `Scope::Sidebar` only while the tree has the keys (`tree_arrows`) and the grid's brought no press, so a line of `[keys.sidebar]` alone is read there and nowhere else: asked for with the keys on the rows, a sidebar line on F2 would reload where F2 edits a cell. Test, beside the reload tests of `src/ui/mod.rs`, in the standard look: the file `[keys.sidebar]` `"ctrl+shift+y" = "reload"`, the press, and the reload asked for as those tests see it asked; with no such line, one press of the default reloads once, not twice; and with the keys on the rows the sidebar's line does nothing.

- [ ] **Step 6: Run** the three test groups, then everything. Expected: PASS. Existing tests that compare `SettingsFile` or `Loaded` literally get the two new empty fields.

- [ ] **Step 7: Checks, then commit**

```bash
git add src/settings.rs src/app.rs src/testing.rs src/ui/mod.rs src/ui/keys.rs src/ui/settings/file_pane.rs && git commit -m "Read the user's keys from settings.toml, live

[keys.<scope>] gives a command its keys there and [keys.unbind] takes a
default away. The keymap is made again when the file or the look
changes, and a line that does not stand is ignored and marked in the
file like any other the reader could not use.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

**End of sitting A.** Say what was built and what to try by hand: add `[keys.grid]` `"X" = "set_null"` to `~/.config/tabletist/settings.toml` with the app open.

---

### Task 6: Two pages in the Settings window

**Files:** Modify `src/model.rs`, `src/app.rs`, `src/keymap.rs`, `src/ui/settings/{mod,terminal,sheet}.rs`, `src/ui/mod.rs` (tests).

- [ ] **Step 1: Write the failing tests** (`src/ui/mod.rs` `mod tests`, beside the settings tests)

```rust
fn settings_page(harness: &Harness) -> crate::model::SettingsPage {
    match &harness.app.dialog {
        Some(crate::model::Dialog::Settings(dialog)) => dialog.page,
        _ => panic!("the Settings window is not open"),
    }
}

#[test]
fn the_settings_screen_goes_to_the_keyboard_page_from_its_nav() {
    use crate::model::SettingsPage;
    let mut harness = settings_screen();
    assert_eq!(settings_page(&harness), SettingsPage::General);
    assert!(harness.has("keyboard"), "the nav names the page");
    // ctrl+h is the nav's pane, j the next page, ctrl+l the page itself.
    harness.press(egui::Key::H, egui::Modifiers::CTRL);
    harness.press(egui::Key::J, egui::Modifiers::NONE);
    assert_eq!(settings_page(&harness), SettingsPage::Keyboard);
    assert_eq!(settings_cursor(&harness), 0, "j moved the page, not the option");
    harness.press(egui::Key::J, egui::Modifiers::NONE);
    assert_eq!(settings_page(&harness), SettingsPage::Keyboard, "the last page");
    harness.press(egui::Key::L, egui::Modifiers::CTRL);
    // In the page again, j is its cursor's.
    harness.press(egui::Key::J, egui::Modifiers::NONE);
    assert_eq!(keys_cursor(&harness), 1);
    assert!(harness.has("connections"), "a command of the page");
    // A click on the nav goes there too.
    let general = harness.painted_rect("general").expect("the nav's item");
    click_at(&mut harness, general.center());
    assert_eq!(settings_page(&harness), SettingsPage::General);
    // Esc closes from the nav as from a page.
    harness.press(egui::Key::H, egui::Modifiers::CTRL);
    harness.press(egui::Key::Escape, egui::Modifiers::NONE);
    assert!(harness.app.dialog.is_none());
}

#[test]
fn the_settings_sheet_goes_to_the_keyboard_page_from_its_nav() {
    use crate::model::SettingsPage;
    for look in SHEET_LOOKS {
        let mut harness = settings_sheet(look());
        harness.click("Keyboard");
        assert_eq!(settings_page(&harness), SettingsPage::Keyboard, "{}", look().name);
        assert!(harness.has("Run statement"));
        assert!(!harness.has("Rows per page"));
        harness.click("General");
        assert!(harness.has("Rows per page"));
    }
}
```

`keys_cursor` reads `dialog.keys.row`. If the nav's "general" is also the header's word, find the nav's by its place (`painted_rect` returns the first: pick the one left of `NAV`), as the tests there tell two texts apart.

- [ ] **Step 2: Run them.** Expected: do not compile.

- [ ] **Step 3: The model** (`src/model.rs`).

```rust
/// A page of the Settings window.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SettingsPage {
    #[default]
    General,
    Keyboard,
}

impl SettingsPage {
    pub const ALL: [SettingsPage; 2] = [Self::General, Self::Keyboard];

    /// English: whoever writes it translates it.
    pub fn name(self) -> &'static str {
        match self {
            Self::General => "General",
            Self::Keyboard => "Keyboard",
        }
    }
}

/// The Keyboard page: where its cursor is and what it is asking.
#[derive(Debug, Default)]
pub struct KeysPage {
    /// The cursor's line, as an index into `keymap::page_lines`.
    pub row: usize,
    /// The letters so far of one of the screen's own keys of several
    /// (`dd`).
    pub waiting: String,
    pub state: Rebinding,
}

/// What the Keyboard page is in the middle of.
#[derive(Debug, Default, PartialEq, Eq)]
pub enum Rebinding {
    #[default]
    Idle,
    /// The cursor's line waits for its keys: `typed` are the letters so
    /// far.
    Recording { typed: String },
    /// The keys recorded are another command's. `swap` exchanges the two.
    Taken {
        keys: String,
        by: crate::keymap::Command,
        swap: Option<crate::keymap::Overrides>,
    },
    /// The keys recorded cannot be the line's.
    Refused { keys: String, why: crate::keymap::Why },
    /// Reset all was chosen and waits for its answer.
    ResettingAll,
}
```

`SettingsDialog` gets `pub page: SettingsPage`, `pub in_nav: bool` ("the terminal screen's keys are on the list of pages"), `pub keys: KeysPage`. Actions: `ShowSettingsPage(SettingsPage)`, `FocusSettingsNav(bool)`, `MoveSettingsPage(isize)`, `MoveKeysRow(isize)`, `SelectKeysRow(usize)`.

`App::apply`: `ShowSettingsPage` sets the page, `in_nav = false`, and `keys.state = Idle`; `MoveSettingsPage` steps within `SettingsPage::ALL` (clamped) keeping `in_nav`; `FocusSettingsNav` sets it; `MoveKeysRow`/`SelectKeysRow` clamp to `keymap::page_lines(&self.keymap, self.layout()).len() - 1` and are ignored unless `state == Idle`. A page change also ends `resetting`.

- [ ] **Step 4: The keymap** gets the screen's new keys (after `SettingsToggle`, all `PROMPT`, `.when(When::SettingsScreen)`, `Group::Prompts`, `row: ""`):

| Command | Name | Omarchy keys |
|---|---|---|
| `SettingsToPages` | "To the list of pages" | `ctrl+h` |
| `SettingsToPage` | "To the page" | `ctrl+l` |
| `SettingsRecord` | "Record a key" | `enter` |
| `SettingsUnbind` | "Leave the command no key" | `dd` |
| `SettingsSwap` | "Exchange the two keys" | `s` |

Two commands for the two ways, not one whose spelling says which: `tests/keys.rs` lets no key be named outside the keymap, and a comparison with a spelling is one. The hint writes them together (`Keymap::together` gives `ctrl+h/l`).

Mac column `NONE` for all five (the sheet has buttons). `every_command_has_a_name_of_its_own` and the conflict test must stay green: nothing else of `When::SettingsScreen` is `s`, `d`, `enter`, `ctrl+h` or `ctrl+l` (the vim rule exempts `Scope::Prompt`).

- [ ] **Step 5: The terminal screen** (`src/ui/settings/mod.rs`, `terminal.rs`).

- `asked` learns `Command::SettingsToPages` and `Command::SettingsToPage` (`Asked::Pane { nav: command == Command::SettingsToPages }`, read through `press_of` like the other chords, wherever the keyboard is).
- `keys()`: with `in_nav`, `Asked::Move(by)` pushes `Action::MoveSettingsPage(by)` and every other option key (`Reset`, `Step`, `Flip`) is left alone; `Asked::Pane { nav }` pushes `FocusSettingsNav(nav)`. Out of the nav on the General page nothing changes. On the Keyboard page `keys()` hands over to `keyboard_terminal::keys` (Task 7); until then, `Move` pushes `MoveKeysRow`, and `Reset`, `Step` and `Flip` are left alone there: they would change General's options, unseen.
- `terminal::nav` draws both pages from `SettingsPage::ALL`, the shown one with `mark`, each a click target (`ui.interact(item, …, Sense::click())` with `WidgetInfo::selected(WidgetType::Button, …)` named as written) that pushes `ShowSettingsPage`. With `in_nav`, the mark's bar is the accent and the name is `palette.text`; out of it the shown page's name is `palette.text` and the others `palette.dim` (as the canvas has them). At the nav's foot, 12 above its bottom edge, the hint `ctrl+h/l pane` from `keys::written_together(ctx, look, &[Command::SettingsToPages, Command::SettingsToPage])` and `skin.say("pane")`, in the secondary role, dim.
- `terminal::header` writes the shown page's name where it wrote `General`.
- `terminal::show` draws `rows(...)` for General; for Keyboard a placeholder call `keyboard_terminal::show` that Task 7 fills (in this task: the lines' names, one to a row, so the test's `has("connections")` holds).

- [ ] **Step 6: The sheet** (`sheet.rs`): `nav` draws an item per page as it draws `General` now (selected: the fill and `accent_hover`; not selected: no fill, `palette.text`, `UiBody`), each pushing `ShowSettingsPage`. `content` switches on the page; for Keyboard it calls `keyboard_sheet::show` (Task 9; in this task the title "Keyboard" and the lines' names in a scroll area under it). The footer is General's on General; on Keyboard it is the path and the reveal link only (Export and Reset to defaults are the options'), with the ignored-lines warning as before.

- [ ] **Step 7: Run** the two tests and the settings tests. Expected: PASS. `the_letters_of_the_terminal_screen_do_nothing_in_the_sheet` and `the_settings_cursor_stays_among_the_options` must pass untouched.

- [ ] **Step 8: Checks, then commit**

```bash
git add src/model.rs src/app.rs src/keymap.rs src/ui/settings src/ui/mod.rs && git commit -m "Give the Settings window a second page, and a nav that goes to it

The terminal screen's keys move between its nav and its page; the
sheet's nav is clicked. The Keyboard page lists the commands so far.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 7: The Omarchy Keyboard page (the brief's test 9, first half)

**Files:** Create `src/ui/settings/record.rs`, `src/ui/settings/keyboard_terminal.rs`. Modify `src/ui/settings/{mod,terminal,file_pane}.rs`, `src/ui/keys.rs` (`typed_chars` becomes `pub(crate)`), `src/model.rs`, `src/app.rs`, `src/ui/mod.rs` (tests).

- [ ] **Step 1: Write the failing tests.**

`src/ui/settings/record.rs` `mod tests`:

```rust
use super::*;
use crate::keymap::Layout;
use egui::{Key, Modifiers};

fn press(key: Key, modifiers: Modifiers) -> egui::Event {
    crate::testing::key(key, modifiers)
}

#[test]
fn a_chord_is_recorded_as_the_keymap_spells_it() {
    let none = Modifiers::NONE;
    let ctrl_shift = Modifiers::CTRL | Modifiers::SHIFT;
    // What is held comes with the press: a test's press, and a window's,
    // can come before the frame's own modifiers know of it.
    assert_eq!(
        chord(Layout::Omarchy, &press(Key::F, ctrl_shift), none),
        Some("ctrl+shift+f".to_owned())
    );
    // The Mac's command key is `cmd`; Windows and Omarchy have Ctrl.
    let cmd = Modifiers::MAC_CMD | Modifiers::COMMAND;
    assert_eq!(chord(Layout::Mac, &press(Key::K, cmd), none), Some("cmd+k".to_owned()));
    let ctrl = Modifiers::CTRL | Modifiers::COMMAND;
    assert_eq!(chord(Layout::Windows, &press(Key::K, ctrl), none), Some("ctrl+k".to_owned()));
    // A key that types nothing is a chord with nothing held.
    assert_eq!(chord(Layout::Mac, &press(Key::F5, none), none), Some("f5".to_owned()));
    // A letter alone is typed, not a chord.
    assert_eq!(chord(Layout::Omarchy, &press(Key::D, Modifiers::SHIFT), none), None);
    // A release records nothing.
    assert_eq!(chord(Layout::Mac, &crate::testing::release(Key::K, cmd), none), None);
}

#[test]
fn the_windows_rewritten_chords_are_recorded_as_pressed() {
    // The command key with C comes as a copy, whatever else is held, and
    // only the frame says what was.
    let held = Modifiers::CTRL | Modifiers::COMMAND | Modifiers::SHIFT;
    assert_eq!(chord(Layout::Omarchy, &egui::Event::Copy, held), Some("ctrl+shift+c".to_owned()));
    // A digit with Shift comes as what it types, which no spelling
    // names: the key pressed is the digit.
    let mut shifted = press(Key::Exclamationmark, held);
    if let egui::Event::Key { physical_key, .. } = &mut shifted {
        *physical_key = Some(Key::Num1);
    }
    assert_eq!(chord(Layout::Omarchy, &shifted, Modifiers::NONE), Some("ctrl+shift+1".to_owned()));
    // A key the layout names is the one recorded, wherever it sits: the
    // handler matches the key, not its place.
    let mut moved = press(Key::Z, Modifiers::CTRL);
    if let egui::Event::Key { physical_key, .. } = &mut moved {
        *physical_key = Some(Key::Y);
    }
    assert_eq!(chord(Layout::Omarchy, &moved, Modifiers::NONE), Some("ctrl+z".to_owned()));
}
```

`src/ui/mod.rs` `mod tests`:

```rust
/// The Settings screen on its Keyboard page, the cursor on `command`.
fn keys_screen_on(command: crate::keymap::Command) -> Harness {
    let mut harness = settings_screen();
    harness.app.apply(Action::ShowSettingsPage(crate::model::SettingsPage::Keyboard));
    let lines = crate::keymap::page_lines(&harness.app.keymap, crate::keymap::Layout::Omarchy);
    let row = lines.iter().position(|line| line.command == command).expect("listed");
    harness.app.apply(Action::SelectKeysRow(row));
    harness.settle();
    harness
}

#[test]
fn rebinding_to_a_taken_key_says_whose_it_is_and_swaps_on_request() {
    use crate::keymap::{Command, Layout};
    let mut harness = keys_screen_on(Command::DeleteRow);
    assert!(harness.has("dd"));
    harness.press(egui::Key::Enter, egui::Modifiers::NONE);
    assert!(harness.has("press keys…"));
    // The screen's own letters are the recording's now.
    let cursor = keys_cursor(&harness);
    harness.press(egui::Key::J, egui::Modifiers::NONE);
    assert_eq!(keys_cursor(&harness), cursor, "j is a letter of the key, not a move");
    harness.press(egui::Key::Backspace, egui::Modifiers::NONE);
    harness.press(egui::Key::D, egui::Modifiers::SHIFT);
    harness.press(egui::Key::Enter, egui::Modifiers::NONE);
    // Taken: whose, and how to exchange.
    assert!(harness.has("D is set default"), "{:?}", crate::testing::labels(&harness.settle()));
    assert!(harness.has("[s] swap"));
    assert!(harness.app.settings.keys.is_empty(), "nothing is bound yet");
    harness.press(egui::Key::S, egui::Modifiers::NONE);
    let keymap = harness.app.keymap.clone();
    assert_eq!(keymap.label(Layout::Omarchy, Command::DeleteRow), "D");
    assert_eq!(keymap.label(Layout::Omarchy, Command::SetDefault), "dd");
    assert_eq!(settings_saves(&harness), 1);
    assert!(harness.app.settings_file.text.contains("[keys.grid]\n\"D\"  = \"delete_row\"\n\"dd\" = \"set_default\"\n"));
    // The page shows the new keys, and the screen is still open.
    assert!(settings_open(&harness));
    assert!(!harness.has("press keys…"));
}

#[test]
fn escape_gives_up_a_recording_and_a_taken_key_without_closing_the_screen() {
    use crate::keymap::Command;
    let mut harness = keys_screen_on(Command::DeleteRow);
    harness.press(egui::Key::Enter, egui::Modifiers::NONE);
    harness.press(egui::Key::Escape, egui::Modifiers::NONE);
    assert!(settings_open(&harness));
    assert!(!harness.has("press keys…"));
    harness.press(egui::Key::Enter, egui::Modifiers::NONE);
    harness.press(egui::Key::D, egui::Modifiers::SHIFT);
    harness.press(egui::Key::Enter, egui::Modifiers::NONE);
    harness.press(egui::Key::Escape, egui::Modifiers::NONE);
    assert!(settings_open(&harness));
    assert!(harness.app.settings.keys.is_empty());
    harness.press(egui::Key::Escape, egui::Modifiers::NONE);
    assert!(harness.app.dialog.is_none());
}

#[test]
fn a_free_key_is_bound_at_once_and_the_new_key_answers_in_the_grid() {
    use crate::keymap::{Command, Layout};
    let mut harness = keys_screen_on(Command::OpenReferencedRow);
    harness.press(egui::Key::Enter, egui::Modifiers::NONE);
    harness.press(egui::Key::G, egui::Modifiers::NONE);
    harness.press(egui::Key::R, egui::Modifiers::NONE);
    assert!(harness.has("gr"), "the letters so far are shown");
    harness.press(egui::Key::Backspace, egui::Modifiers::NONE);
    harness.press(egui::Key::T, egui::Modifiers::NONE);
    harness.press(egui::Key::Enter, egui::Modifiers::NONE);
    assert_eq!(harness.app.keymap.label(Layout::Omarchy, Command::OpenReferencedRow), "gt");
    // A chord is taken as it is pressed.
    let mut harness = keys_screen_on(Command::NewSqlTab);
    harness.press(egui::Key::Enter, egui::Modifiers::NONE);
    harness.press(egui::Key::T, egui::Modifiers::CTRL | egui::Modifiers::ALT);
    assert_eq!(harness.app.keymap.label(Layout::Omarchy, Command::NewSqlTab), "ctrl+alt+t");
    harness.press(egui::Key::Escape, egui::Modifiers::NONE);
    let tab = harness.connect_fake();
    let tabs = |harness: &Harness| harness.app.workspace(tab).unwrap().sql_tabs().count();
    let before = tabs(&harness);
    harness.press(egui::Key::T, egui::Modifiers::CTRL);
    assert_eq!(tabs(&harness), before, "the old key is nobody's");
    harness.press(egui::Key::T, egui::Modifiers::CTRL | egui::Modifiers::ALT);
    assert_eq!(tabs(&harness), before + 1);
}

#[test]
fn a_key_of_the_wrong_kind_and_a_fixed_line_say_why() {
    use crate::keymap::Command;
    let mut harness = keys_screen_on(Command::DeleteRow);
    harness.press(egui::Key::Enter, egui::Modifiers::NONE);
    harness.press(egui::Key::D, egui::Modifiers::CTRL);
    assert!(harness.has("delete row is typed here, not a chord"));
    harness.press(egui::Key::Escape, egui::Modifiers::NONE);
    // A line whose keys say which way is not recorded.
    let mut harness = keys_screen_on(Command::MoveRow);
    harness.press(egui::Key::Enter, egui::Modifiers::NONE);
    assert!(!harness.has("press keys…"));
    assert!(harness.app.settings.keys.is_empty());
}

#[test]
fn dd_leaves_the_line_no_key_and_capital_r_gives_its_default_back() {
    use crate::keymap::{Command, Layout};
    let mut harness = keys_screen_on(Command::OpenReferencedRow);
    harness.press(egui::Key::D, egui::Modifiers::NONE);
    assert!(harness.app.settings.keys.is_empty(), "one d is half the key");
    harness.press(egui::Key::D, egui::Modifiers::NONE);
    assert!(harness.app.keymap.label(Layout::Omarchy, Command::OpenReferencedRow).is_empty());
    assert!(harness.app.settings_file.text.contains("[keys.unbind]\ngrid = [\"gd\"]\n"));
    harness.press(egui::Key::R, egui::Modifiers::SHIFT);
    assert_eq!(harness.app.keymap.label(Layout::Omarchy, Command::OpenReferencedRow), "gd");
    assert!(harness.app.settings.keys.is_empty());
}

#[test]
fn the_file_beside_the_keyboard_page_marks_the_cursors_line_and_a_refused_one() {
    let mut harness = settings_screen();
    let text = "[keys.grid]\n\"X\" = \"set_null\"\n\"D\" = \"delete_row\"\n".to_owned();
    harness.app.apply(Action::Backend(crate::backend::Event::SettingsFile { text, own: false }));
    harness.app.apply(Action::ShowSettingsPage(crate::model::SettingsPage::Keyboard));
    harness.settle();
    // The refused line is in the colour of an ignored one.
    let danger = harness.app.palette.danger;
    // An ignored line is painted whole, in one colour.
    assert_eq!(harness.painted_color("\"D\" = \"delete_row\""), Some(danger));
    assert_ne!(harness.painted_color("\"X\" = "), Some(danger));
}
```

The test names of the two halves of the brief's test 9 are `rebinding_to_a_taken_key_says_whose_it_is_and_swaps_on_request` (this one, Omarchy) and Task 9's `…_in_a_sheet`. Read how the pane's tests look a painted line up (`the_settings_screen_shows_the_file_beside_the_options`) and assert the same way. `Harness::has` reads the names of AccessKit nodes, not what was painted (`src/testing.rs:265`): `widgets::paint_text` makes no node, `widgets::paint_label` and `announce` do (`src/ui/widgets.rs:757`). So the sentence (`D is set default`), each hint (`[s] swap`), `press keys…`, the letters so far and a line's keys are each written with `paint_label`. The key goes into the sentence after the look has lower-cased the words, as `in_editor` puts `$EDITOR` in (`terminal.rs:645`), or `D` would be `d`. Count the SQL tabs as the tests of `ctrl+t` in the tree count them.

- [ ] **Step 2: Run them.** Expected: do not compile.

- [ ] **Step 3: The recorder** (`src/ui/settings/record.rs`).

```rust
//! A frame's presses as the keys someone is recording on the Keyboard
//! page: a chord as the keymap spells it, or the characters typed.

use egui::{Event, Key, Modifiers};

use crate::keymap::{Layout, Mods};

/// What is held, as a spelling names it in `layout`: the Mac tells its
/// command key from Control, and the others have Ctrl for both.
fn mods(layout: Layout, held: Modifiers) -> Mods {
    match layout {
        Layout::Mac => Mods {
            cmd: held.mac_cmd || held.command,
            ctrl: held.ctrl,
            alt: held.alt,
            shift: held.shift,
        },
        Layout::Windows | Layout::Omarchy => Mods {
            cmd: false,
            ctrl: held.ctrl || held.command,
            alt: held.alt,
            shift: held.shift,
        },
    }
}

/// The chord `event` is, if it is the press of one: a key with Ctrl, Alt
/// or the command key held, or a key that types nothing. A letter alone
/// is typed, and read as text. `frame` is what the frame holds: a key
/// event says what was held with it, a clipboard event does not.
pub(super) fn chord(layout: Layout, event: &Event, frame: Modifiers) -> Option<String> {
    let clipboard = |key| crate::keymap::spelling(mods(layout, frame), key);
    match event {
        // The window turns the command key with C, X and V into these.
        Event::Copy => clipboard(Key::C),
        Event::Cut => clipboard(Key::X),
        Event::Paste(_) => clipboard(Key::V),
        Event::Key {
            key,
            physical_key,
            pressed: true,
            repeat: false,
            modifiers,
            ..
        } => {
            let mods = mods(layout, *modifiers);
            let held = mods.cmd || mods.ctrl || mods.alt;
            let types = crate::keymap::char_of(*key, false).is_some() || *key == Key::Space;
            if types && !held {
                return None;
            }
            // The key the layout names, as the handler matches it. With
            // Shift a digit or a mark comes as what it types, which no
            // spelling names: then the key by its place.
            crate::keymap::spelling(mods, *key)
                .or_else(|| crate::keymap::spelling(mods, (*physical_key)?))
        }
        _ => None,
    }
}
```

(Make `mods` the mirror of `held_is` in `src/ui/keys.rs`, and the choice of modifiers that of `press_of` there, lines 162 to 178: the event's own for a key, the frame's for a clipboard event. `Harness::press` sends no `ModifiersChanged`, so a test's chord has its modifiers on the event alone.)

- [ ] **Step 4: Actions and the app.** `Action::RecordKeys` (the cursor's line starts recording, if it is `rebindable` and the state is `Idle`), `Action::KeysTyped(String)` (the letters so far, while recording), `Action::KeysWaiting(String)` (the letters so far of one of the page's own keys, `KeysPage.waiting`), `Action::KeysRecorded(String)`, `Action::SwapKeys`, `Action::CancelKeys`, `Action::UnbindKeys`, `Action::ResetKeys`, `Action::SetKeys(crate::keymap::Overrides)`.

```rust
            Action::KeysRecorded(keys) => {
                let layout = self.layout();
                let lines = crate::keymap::page_lines(&self.keymap, layout);
                let Some(Dialog::Settings(dialog)) = &mut self.dialog else {
                    return;
                };
                let Some(line) = lines.get(dialog.keys.row) else {
                    return;
                };
                use crate::keymap::Outcome;
                use crate::model::Rebinding;
                match crate::keymap::rebind(layout, &self.settings.keys, line, &keys) {
                    Outcome::Bound(lines) => {
                        dialog.keys.state = Rebinding::Idle;
                        self.change_settings(|settings| settings.keys = lines);
                    }
                    Outcome::Taken { by, swap, .. } => {
                        dialog.keys.state = Rebinding::Taken { keys, by, swap };
                    }
                    Outcome::Refused(why) => {
                        dialog.keys.state = Rebinding::Refused { keys, why };
                    }
                }
            }
```

`SwapKeys` takes the `swap` out of a `Taken` state and applies it the same way; `UnbindKeys` and `ResetKeys` apply `keymap::unbind` / `keymap::reset` to the cursor's line when it is `rebindable` and the state is `Idle`; `CancelKeys` sets `Idle`; `SetKeys` is `change_settings(|settings| settings.keys = keys)`. `change_settings` already writes nothing when nothing changed, so a rebind to the key the line has saves nothing. (`self.dialog` is borrowed across `change_settings` in the sketch: take the row and set the state in two steps, as the borrow checker asks.)

- [ ] **Step 5: The page's keys** (`src/ui/settings/keyboard_terminal.rs`, `pub(super) fn keys(app, ctx, dialog state, actions)`, called from `mod.rs` `keys()` when the page is Keyboard and the nav has not the keys).

- `Recording { typed }`: every key and text event of the frame is taken from the input (nothing under reads them). In the frame's order: a bare `Escape` → `CancelKeys`; a bare `Enter` → `KeysRecorded(typed)` when `typed` is not empty; a bare `Backspace` → `KeysTyped(typed without its last char)`; `record::chord(Layout::Omarchy, event, held)` → `KeysRecorded(chord)`; otherwise the frame's typed characters (`crate::ui::keys::typed_chars(ctx)`, made `pub(crate)`: it reads text events and, for a test's bare press, the character the key types) are appended → `KeysTyped`. Spaces between letters are not recorded; `yy p` is written in the file.
- `Taken { swap, .. }`: `SettingsSwap`'s key → `SwapKeys` when `swap` is some; `SettingsClose`'s → `CancelKeys`. Everything else is swallowed.
- `Refused`: `SettingsClose`'s key or `SettingsRecord`'s → `CancelKeys`; swallowed otherwise.
- `Idle`: `SettingsMove` → `MoveKeysRow`; `SettingsRecord` → `RecordKeys`; `SettingsReset` → `ResetKeys`; `SettingsOpenFile` → `EditSettingsFile`; `SettingsToPages` → nav; `SettingsClose` → `CloseDialog`. The letters of several (`dd`) go through the keymap's own reader: append the frame's typed characters to `KeysPage.waiting` one at a time and ask `app.keymap.typed(Layout::Omarchy, Scope::Prompt, &|when| when == When::SettingsScreen, &waiting)`: `Typed::Command(Command::SettingsUnbind)` → `UnbindKeys`; `Typed::Waiting` keeps the letters (`Action::KeysWaiting`); anything else clears them and is read as the single key it is. A key that types nothing clears the wait.

The single-letter matching in `mod.rs` `asked()` is reused for the single keys: give it the list of commands to look for as a parameter rather than copying it.

- [ ] **Step 6: Draw the page** (`keyboard_terminal::show(ui, pane, app, dialog, skin, actions)`), by the canvas board `OmarchySettingsKeyboard.dc.html` (read it; check the Components board for a state it does not show):

- A header row in the secondary role, dim: `scope`, `keys`, `action`. Columns: scope 110, keys 300, action the rest, side padding as `ROW_SIDE`.
- Under each group a heading as General's (`osec`): `window`, `navigate`, `edit`, `sql` (`skin.say` of "Window", "Navigate", "Edit", "SQL").
- A row per line, `ROW` tall: the scopes' ids joined with a space, dim; the keys as `keymap::written(Layout::Omarchy, keys)` joined by two spaces, in the accent, or `none` dim when the line has none; the command's name through `skin.say` (so lower case). A line that is not `rebindable` is dim throughout. The cursor's row has `mark` and its name is written with the `▌` as General's rows are. A click selects the row (`SelectKeysRow`); a click on the selected row's keys records.
- `Recording`: the cursor row's keys column says `press keys…` dim, then the letters so far in the warning colour.
- `Taken`: the keys column shows the recorded keys in the warning colour, and right-aligned in the row `{keys} is {by's name} · [s] swap  [esc] cancel` (the bracketed keys from `keymap.key(...).bracketed()` of `SettingsSwap` and `SettingsClose`; without `[s] swap` when `swap` is none). The sentence: `gettext("{keys} is {command}")`.
- `Refused`: the same place says why, then `[esc] cancel`:

| Why | Sentence (English; lower-cased by the look) |
|---|---|
| `Kind`, the line has typed keys | `{command} is typed here, not a chord` (one letter was wanted: `{command} takes one letter here`) |
| `Kind`, the line has chords | `{command} takes a chord here` |
| `Kind`, the line has a range of digits | `{command} takes a digit with keys held` |
| `Kind`, the line has only a `:` line | `{command} is a line of the prompt: change it in the file` |
| `Kind`, the line has no key | `{command} has no key in this look` |
| `Vims` | `{keys} is vim's own where text is typed` |
| `Menus` | `{keys} is the system menu's` |
| `Unread` | `{keys} is no key here` |
| `Fixed`, `Elsewhere`, `Nobody` | `{command} keeps its keys` |

- Under the list, the note from the canvas: `super is never bound: hyprland owns it · insert-mode keys follow vim`, dim, secondary role.
- The rows scroll in a `ScrollArea` as General's do; the cursor's row is scrolled into view when the cursor moved (`ui.scroll_to_rect(row, None)` in the frame the cursor changed, not every frame, or the wheel would fight it).
- The footer (`terminal::footer`) takes the page: on Keyboard its hints are `SettingsMove` move, `SettingsRecord` record, `SettingsUnbind` unbind, `SettingsOpenFile` open file in $EDITOR, `SettingsReset` reset, `SettingsClose` close; while recording: `SettingsRecord` take, `SettingsClose` cancel. Each hint that does one thing is its button too, as now.
- The file pane: `file_pane::show` takes `mark: Option<usize>` (a line number) where it took the option. General passes the option's line as it worked it out before; Keyboard passes the line of the cursor line's first `Bind` in `settings_file.bind_lines` (the binds of `(command, scope in line.scopes)`), or none. If the pane scrolls to the option's line today, it scrolls to this one the same way.

- [ ] **Step 7: Look at it.** Add a scene `omarchy-settings-keyboard` to `src/shots.rs` (the page with the cursor on Delete row, recording, `D` typed and taken), render the scenes (`~/.cargo/bin/cargo test --locked --features shots --lib shots::shots -- --ignored`), read `target/shots/omarchy-settings-keyboard.png` beside the canvas board, and mend what differs in spacing, colour and wording. The image stays local; nothing compares it.

- [ ] **Step 8: Run** the tests of this task, then all. Expected: PASS.

- [ ] **Step 9: Checks, then commit**

```bash
git add src/model.rs src/app.rs src/shots.rs src/ui/keys.rs src/ui/mod.rs src/ui/settings && git commit -m "Record a key on the Omarchy Keyboard page, and swap a taken one

enter records on the cursor's line: a chord is taken as pressed, letters
when enter ends them. A key another command has is said with whose it
is, and s exchanges the two. dd leaves the line no key, R gives its
default back. The file beside the page marks the line.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 8: `:keys default`

**Files:** Modify `src/keymap.rs` (`KeysDefault` built), `src/app/editing.rs`, `src/app.rs`, `src/model.rs`, `src/backend.rs`, `src/paths.rs`.

- [ ] **Step 1: Write the failing test** (`src/app.rs` `mod tests`, beside the `:w` tests; open the prompt as they do)

```rust
#[test]
fn keys_default_writes_the_looks_full_list_beside_the_settings() {
    let (mut app, tab) = /* an Omarchy app with a workspace, as the `:w` tests make one */;
    run_line(&mut app, tab, "keys default");
    let Some(Command::Save { path, file: StateFile::Text(text) }) = app.backend.sent.last() else {
        panic!("a file is written: {:?}", app.backend.sent.last());
    };
    assert_eq!(path, &app.dirs.default_keys_file());
    assert!(text.starts_with("# the default keys of the omarchy look\n"));
    assert!(text.contains("[keys.grid]\n"));
    assert!(text.contains("\"dd\""));
    // What it wrote is the keymap: read back as the user's lines, it
    // changes nothing and none is refused.
    let lines = crate::settings::Settings::from_toml(text).settings.keys;
    let (_, refused) = crate::keymap::Keymap::with(crate::keymap::Layout::Omarchy, &lines);
    assert!(refused.is_empty());
    // By scope, as the file has them, and no key twice in a table.
    assert!(crate::settings::Settings::from_toml(text).invalid.is_empty());
    assert_eq!(lines, crate::keymap::full(crate::keymap::Layout::Omarchy));
    assert!(app.notice.as_deref().is_some_and(|notice| notice.contains("keys.default.toml")));
    // The settings are as they were.
    assert!(app.settings.keys.is_empty());
    let workspace = app.workspace(tab).unwrap();
    assert!(workspace.command_error.is_none());
}
```

Also update `a_line_of_the_prompt_is_read_in_its_scope` (`src/keymap.rs`, about line 2650), which reads `:keys default` as claimed today.

- [ ] **Step 2: Run it.** Expected: does not compile (`StateFile::Text`, `default_keys_file`).

- [ ] **Step 3: Implement.**

- `src/keymap.rs`: `C::KeysDefault => info("Write the full list of keys", "", Navigation)` without `.reserved()`.
- `src/paths.rs`: `pub fn default_keys_file(&self) -> PathBuf { self.config.join("keys.default.toml") }`, with a comment: written by `:keys default`, never read.
- `src/backend.rs`: `StateFile::Text(String)`, saved with `crate::util::write_atomic(path, text.as_bytes())`. The match at `backend.rs:819` that knows the settings file's own writes has nothing to do for it: give the arm the compiler asks for.
- `src/app/editing.rs` `run_command`: the arm `(Typed::Command(Command::KeysDefault), _) => Action::WriteDefaultKeys`, before the fallthrough that refuses.
- `src/app.rs`:

```rust
            Action::WriteDefaultKeys => {
                let layout = self.layout();
                let path = self.dirs.default_keys_file();
                let mut text = format!(
                    "# the default keys of the {} look\n\
                     # copy a line into settings.toml to change it\n",
                    self.look.name.to_lowercase()
                );
                text.push_str(&crate::settings::keys_toml(&crate::keymap::full(layout)));
                self.notice = Some(format!("The default keys are in {}.", path.display()));
                self.backend.send(Command::Save {
                    path,
                    file: StateFile::Text(text),
                });
            }
```

  (`self.look.name` for Omarchy: check what it is and make the test's first line match. A save that fails replaces the notice with the failure, through the event every save answers with.)

- [ ] **Step 4: Run** the test, then all. **Step 5: Checks, then commit**

```bash
git add src/keymap.rs src/paths.rs src/backend.rs src/model.rs src/app.rs src/app/editing.rs && git commit -m "Write the look's default keys with :keys default

Into keys.default.toml beside the settings, in the form settings.toml
takes them, to copy lines from. The settings stay the user's changes.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

**End of sitting B.** By hand, for the user: record a chord and letters on the Omarchy page, `ctrl+shift+c` and `ctrl+shift+1` among them (the window rewrites both), the swap, `dd`, `R`, `:keys default`.

---

### Task 9: The Mac and Windows Keyboard page (test 9, second half)

**Files:** Create `src/ui/settings/keyboard_sheet.rs`. Modify `src/ui/settings/{mod,sheet}.rs`, `src/app.rs`, `src/model.rs`, `src/shots.rs`, `src/ui/mod.rs` (tests).

- [ ] **Step 1: Write the failing tests** (`src/ui/mod.rs` `mod tests`)

```rust
/// The Settings sheet of `look` on its Keyboard page.
fn keys_sheet(look: crate::theme::Look) -> Harness {
    let mut harness = settings_sheet(look);
    harness.click("Keyboard");
    harness
}

#[test]
fn rebinding_to_a_taken_key_says_whose_it_is_and_swaps_on_request_in_a_sheet() {
    use crate::keymap::Command;
    for look in SHEET_LOOKS {
        let look = look();
        let layout = crate::keymap::Layout::of(&look);
        let mut harness = keys_sheet(look);
        let label = |harness: &Harness, command| harness.app.keymap.label(layout, command);
        let (statement, all) = (
            label(&harness, Command::RunStatement).to_string(),
            label(&harness, Command::RunAll).to_string(),
        );
        // A click on the shortcut records.
        harness.click("Shortcut for Run all, SQL editor");
        assert!(harness.has("Press keys…"), "{}", look.name);
        harness.press(egui::Key::Enter, egui::Modifiers::COMMAND);
        let warning = format!("{statement} is already “Run statement”.");
        assert!(harness.has(&warning), "{}: {:?}", look.name, crate::testing::labels(&harness.settle()));
        assert!(harness.app.settings.keys.is_empty());
        harness.click("Swap");
        assert_eq!(label(&harness, Command::RunAll), statement.as_str());
        assert_eq!(label(&harness, Command::RunStatement), all.as_str());
        assert_eq!(settings_saves(&harness), 1);
        assert!(!harness.has(&warning));
        // The line says it is changed.
        assert!(harness.has("Run all, changed from default"));
    }
}

#[test]
fn escape_and_cancel_give_up_a_recording_in_a_sheet_without_closing_it() {
    for look in SHEET_LOOKS {
        let mut harness = keys_sheet(look());
        harness.click("Shortcut for Run all, SQL editor");
        harness.press(egui::Key::Escape, egui::Modifiers::NONE);
        assert!(settings_open(&harness));
        assert!(!harness.has("Press keys…"));
        harness.click("Shortcut for Run all, SQL editor");
        harness.press(egui::Key::Enter, egui::Modifiers::COMMAND);
        harness.click("Cancel");
        assert!(harness.app.settings.keys.is_empty());
        assert!(settings_open(&harness));
        harness.press(egui::Key::Escape, egui::Modifiers::NONE);
        assert!(harness.app.dialog.is_none());
    }
}

#[test]
fn a_free_chord_recorded_in_a_sheet_is_the_one_that_answers() {
    use crate::keymap::Command;
    let look = crate::theme::Look::macos();
    let mut harness = keys_sheet(look);
    harness.click("Shortcut for New SQL editor tab, Global");
    let held = egui::Modifiers::COMMAND | egui::Modifiers::MAC_CMD | egui::Modifiers::ALT;
    harness.press(egui::Key::T, held);
    assert_eq!(harness.app.keymap.label(crate::keymap::Layout::Mac, Command::NewSqlTab), "⌥⌘T");
    // A line whose keys are fixed has nothing to click.
    assert!(!harness.has("Shortcut for Move by row, Grid, Results"));
    assert!(!harness.has("Shortcut for Settings, Global"));
    // A command with keys in two places has a shortcut in each.
    assert!(harness.has("Shortcut for Edit cell, Grid"));
    assert!(harness.has("Shortcut for Edit cell, Inspector"));
}

#[test]
fn reset_all_asks_and_then_gives_every_default_back() {
    for look in SHEET_LOOKS {
        let mut harness = keys_sheet(look());
        harness.click("Shortcut for New SQL editor tab, Global");
        harness.press(egui::Key::F9, egui::Modifiers::NONE);
        assert!(!harness.app.settings.keys.is_empty());
        harness.click("Reset all…");
        assert!(harness.has("Reset every shortcut to its default?"));
        harness.click("Cancel");
        assert!(!harness.app.settings.keys.is_empty());
        harness.click("Reset all…");
        harness.click("Reset");
        assert!(harness.app.settings.keys.is_empty());
        // The options are not the keys': they stay.
        assert_eq!(harness.app.settings, crate::settings::Settings::default());
    }
}
```

(How the Mac's command key is held in a headless press: as Run 1's Mac tests hold it. The recorder reads a key's modifiers from its own event, so `Harness::press` is enough.)

- [ ] **Step 2: Run them.** Expected: FAIL (`Shortcut for Run all` is not found).

- [ ] **Step 3: Keys while recording** (`src/ui/settings/mod.rs`): before `sheet::show`, when the page is Keyboard and the state is not `Idle`, take the frame's key events:
  - `Recording`: a bare `Escape` → `CancelKeys`; `record::chord(layout, event, held)` → `KeysRecorded`; a character typed with nothing held but Shift → `KeysRecorded(that character)` (no line of a sheet is typed: the line answers "takes a chord here").
  - `Taken`, `Refused`, `ResettingAll`: a bare `Escape` → `CancelKeys`.
  The sheet's own Escape (`sheet::show`, after the modal) must not also close it: it runs only when the state was `Idle` at the start of the frame, as it gives way to `resetting` today.

- [ ] **Step 4: Draw the page** (`keyboard_sheet::show`), by the canvas board `MacSettingsKeyboard.dc.html` and its Components board for the chip and the warning row:

- Title `Keyboard` (`DialogTitle`), under it `Click a shortcut to record a new one. The same keys can do different things in different scopes.` dim, and at the right of that block the button `Reset all…` (a secondary `ButtonSpec`), enabled when the settings have keys. With `ResettingAll` the block's right side asks instead: `Reset every shortcut to its default?` with `Reset` (danger) and `Cancel`, as the General footer asks about its options. Actions `ResetAllKeys` → state `ResettingAll`; `ConfirmResetAllKeys(true)` → `SetKeys(Overrides::default())`; either answer → `Idle`.
- A header row `Command`, `Scope`, `Shortcut` (the section-label role), then in a `ScrollArea` limited as General's rows are: a heading per group (`help::GROUPS`' words: reuse them, do not write the words twice) and a row per line, 34 tall with a hairline under it: the command's name (`gettext` of `info().name`), with a 6 pt dot in the accent after it when `changed`; the scopes' names (`Scope::name`, translated, joined by `, `), dim; the keys at the right as chips, each `keymap::written(layout, keys)` (the chip the `?` list and the buttons' shortcuts paint; none: a dim `none`).
- A `rebindable` line's shortcut cell is a button: `WidgetInfo` named `Shortcut for {name}, {the scopes' names}` (a command with keys in two places has two lines, and a name must find one), a focus ring as the sheet's links have, a click → `SelectKeysRow(index)` then `RecordKeys`. The command's name cell is labelled `"{name}, changed from default"` for a screen reader when changed (and the dot has that hover text). A fixed line's cell is no button and is dim.
- `Recording` on a line: its cell shows `Press keys…` dim in a field outline (the accent ring of a focused input).
- `Taken`: a warning row under the line (the warning fill, a warning glyph): `{keys as written} is already “{by's name}”.`, then the links `Swap` (when `swap` is some) and `Cancel`, with `· esc` after Cancel written from the keymap (`keys::written(ctx, look, Command::SettingsClose)`). `Refused`: the same row with Task 7's sentence for the reason, sentence case, and `Cancel` only.

- [ ] **Step 5: Look at it.** Scenes `macos-settings-keyboard` and `standard-settings-keyboard` in `src/shots.rs` (a line recording and taken, one changed); render, read beside the canvas, mend. Local only.

- [ ] **Step 6: Run** the tests, then all. **Step 7: Checks, then commit**

```bash
git add src/model.rs src/app.rs src/shots.rs src/ui/mod.rs src/ui/settings && git commit -m "Record a shortcut on the sheet's Keyboard page, and swap a taken one

A click on a shortcut records the next chord. One another command has
is said under the line, with Swap and Cancel; Reset all asks first.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 10: The docs' sample, and where a changed key lives

**Files:** Modify `docs/_reference/settings-and-files.md`, `AGENTS.md`. Create `tests/docs_keys.rs`.

- [ ] **Step 1: Write the failing test** (`tests/docs_keys.rs`)

```rust
//! The keys the settings page of the website tells people to write are
//! keys the app takes: a sample that binds vim's redo, or a command's
//! key of the wrong kind, would be ignored by the app it documents.

use tabletist::keymap::{Keymap, Layout};
use tabletist::settings::Settings;

#[test]
fn the_sample_keys_of_the_settings_page_are_lines_the_app_takes() {
    let page = include_str!("../docs/_reference/settings-and-files.md");
    let samples: Vec<&str> = page
        .split("```toml\n")
        .skip(1)
        .filter_map(|rest| rest.split("```").next())
        .filter(|block| block.contains("[keys."))
        .collect();
    assert!(!samples.is_empty(), "the page shows how keys are written");
    for sample in samples {
        let loaded = Settings::from_toml(sample);
        assert!(loaded.invalid.is_empty(), "{sample}\n{:?}", loaded.invalid);
        assert!(!loaded.settings.keys.is_empty());
        // The sample is Omarchy's: its keys are that look's.
        let (_, refused) = Keymap::with(Layout::Omarchy, &loaded.settings.keys);
        assert!(refused.is_empty(), "{sample}\n{refused:?}");
        assert!(!sample.contains("\"ctrl+r\""), "ctrl+r is redo");
    }
}
```

(The crate's library name and what it exports: as `tests/keys.rs` or another integration test reaches it. If the library does not export `keymap` and `settings`, put the test in `src/settings.rs` `mod tests` with `include_str!("../docs/_reference/settings-and-files.md")` instead.)

- [ ] **Step 2: Run it.** Expected: FAIL, the page shows no keys.

- [ ] **Step 3: Write the section** in `docs/_reference/settings-and-files.md`, after the options' keys, in the page's voice (plain English, no em dashes): a heading `Keys`; that Settings → Keyboard changes a key and that this is where it is kept; the sample below; the rules under "What a line of the file means" above, as a short list (exactly the keys of its lines; unbind; a key keeps its kind; the keys whose letter says which way are fixed; `cmd`/Super and vim's insert keys on Omarchy; a taken key is ignored and shown red); the scope names; that `:keys default` writes every default to `keys.default.toml` beside the file.

```toml
# only changes from the defaults live here
[keys.grid]
"D"  = "delete_row"
"dd" = "set_default"

[keys.sql-editor]
"f5" = "run_all"

[keys.unbind]
grid = ["gd"]
```

Also add `keys.default.toml` to the page's table of files. The key pages (`keyboard-shortcuts.md`, the two guides) are Run 3's.

- [ ] **Step 4: `AGENTS.md`**, in the paragraph on where a key is spelled, one more sentence each: the user's changes are `Settings.keys`, laid over `BINDINGS` by `Keymap::with`, and a new command's id in the file is its variant's name, so renaming a variant renames it for everyone's file; a command whose handler reads the spelling of its key belongs in `Command::rebindable`'s list.

- [ ] **Step 5: Run** the test, then all. **Step 6: Checks, then commit**

```bash
git add docs/_reference/settings-and-files.md AGENTS.md tests/docs_keys.rs && git commit -m "Document the keys tables, with a sample the app takes

The sample is checked against the keymap: it cannot name vim's redo or a
key of the wrong kind without a test saying so.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

**End of sitting C and of Run 2.** For the user to try by hand, on the Mac above all (nothing here has run on one): recording with ⌘ held, ⌘C / ⌘V as recorded chords, the swap, Reset all, and that ⌘, still opens Settings from the menu.

---

## What is checked, and by what

| Brief | Test |
|---|---|
| Section 4: lists every command of the table, grouped as the designs | `the_page_lists_every_command_of_the_settled_table_in_the_looks_groups`, the two nav tests |
| Rebinding checks the same scope and scopes active alongside | `a_line_on_another_commands_key_gives_way_and_says_whose` (same scope, a prefix, the global scope), on the rule of `no_two_commands_share_a_key_where_both_could_be_read` |
| Offers Swap; test 9 | `rebinding_to_a_taken_key_says_whose_it_is_and_swaps_on_request` (Omarchy) and `…_in_a_sheet` (Mac, Windows); `a_taken_key_says_whose_it_is_and_offers_the_exchange` |
| Omarchy writes overrides under `[keys.<scope>]` | `the_keys_tables_are_read_in_the_files_order_and_written_back_the_same`, the `settings_file.text` assertions of test 9 |
| `:keys default` dumps the full map | `keys_default_writes_the_looks_full_list_beside_the_settings`, `the_full_list_read_back_is_the_defaults` |
| The sample does not bind `ctrl+r` | `the_sample_keys_of_the_settings_page_are_lines_the_app_takes`; `Why::Vims` in `a_line_that_cannot_be_is_refused_with_its_reason` |
| A changed key is the one that answers | `a_key_changed_in_the_file_is_the_one_that_answers`, `a_free_key_is_bound_at_once_and_the_new_key_answers_in_the_grid` |
