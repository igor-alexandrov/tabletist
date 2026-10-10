# Keyboard Shortcuts, Run 1: One Keymap Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Every key the app answers to is written once, in `src/keymap.rs`; the keys' handler and everything that names a key (the `?` list, status-line hints, button shortcuts, tooltips) read it from there; the bindings are the ones the brief settled; and tests generated from that data keep it from drifting.

**Architecture:** `src/keymap.rs` holds a table of `Binding`s: a `Command`, the `Scope`s it holds in, what else must be on screen (`When`), and its keys for the Mac's layout and for Omarchy's, each spelled once as text (`cmd+shift+f`, `yy p`, `:w`). Windows has the Mac's keys with Ctrl for Cmd. A spelling is read into presses for the handler (`spell`) and written for a label per layout (`written`: `⇧⌘F`, `Ctrl+Shift+F`, `ctrl+shift+f`). `src/ui/keys.rs` stops naming keys: it asks the keymap whether a command's chord went down, and reads Omarchy's letters and the `:` prompt through `Keymap::typed` and `Keymap::ex`. A command that is not built is in the table with its keys and `built: false`: nothing answers it, and nothing else can have its keys.

**Tech Stack:** Rust 1.98, egui/eframe (the crmne forks). Spec: `docs/superpowers/specs/2026-10-08-keyboard-shortcuts-design.md` (the brief, as given). Design canvas boards read: "macOS – Settings, Keyboard" and "Omarchy – Settings, Keyboard", for the groups and the scope names.

---

## Where the brief and the tree disagree

The brief was read against the tree at `300de6a`. These are the places where the app cannot do what the brief's table asks, or where doing it takes a key away. **Each row needs a yes or a no before code is written.** The "Built" column is what this plan does unless told otherwise.

### What the table asks that the app cannot do yet

| # | The brief asks | Built in this plan | Why |
|---|---|---|---|
| D1 | Tests 5 and 7 press ⌘V, ⇧⌘V, `yy p`, `v y p` and Rollback's ⌥⌘⌫ and see them work | The tests assert the claim: the key reads as that command and as no other, and pressing it changes nothing | Pasting into the grid, duplicating a row, selecting several cells and Commit / Rollback are not built. The brief says not to build them here |
| D2 | Omarchy: `:wq` applies the large editor, `:q!` cancels it | Stays `ctrl+enter` apply, `esc` keep, `ctrl+c` drop. `:wq` and `:q!` are claimed | The floating editor is a text field with no normal mode, so there is nowhere to type a `:` |
| D3 | Omarchy: `=` formats SQL and JSON | In a SQL tab `=` formats once the keyboard is out of the text (`esc`, then `=`; the keyboard goes back to the text). In the large JSON editor `ctrl+shift+f` stays, and `=` is claimed there | The same: neither editor has a normal mode. `esc` out of the large editor closes it |
| D4 | Omarchy: `:ro`, `:rw` (and later `:commit`, `:rollback`, `:history`, `:N`) in the SQL editor | The `:` prompt opens on a SQL tab when the keyboard is out of the text, and takes `ro` and `rw`. The others are claimed: the prompt says "not a command" as it does today | As D3 |
| D5 | Omarchy: `ctrl+c` cancels the running query | It cancels while a query runs. With none running, `ctrl+c` in the SQL text is the field's copy, as now | With no vim mode in the editor there is no other way to copy from it |
| D6 | Omarchy: `v` selects cells, then `y` copies | `v y` typed one after the other copies the cursor's cell. `v p` is claimed | A selection is one cell (`Option<CellPos>`). Bare `y` cannot stay: after `y` nothing could tell it from `yy p` |
| D7 | Omarchy: `ctrl+h/j/k/l` move between panes | `ctrl+h` and `ctrl+l` do, as today. `ctrl+j` and `ctrl+k` are claimed, and so is `ctrl+j` Focus results | The panes are side by side. Moving between a SQL editor and its result by key is not built |
| D8 | Omarchy: `ctrl+shift+d` switches database | Claimed | The database menu opens by click. Opening it by key is small, but it is a new thing: say if you want it in run 1 |
| D9 | The brief names nine scopes | The keys of the dialogs (connection form, Settings screen, quick open, conflict, leaving, the confirmation) are in the keymap under `prompt`, each marked with its dialog | The brief wants every binding in the one module, and has no scope for a dialog |
| D10 | "macOS and Omarchy" | The Windows look has the Mac's keys with Ctrl for Cmd, written `Ctrl+Shift+F` | The app has three looks. Windows had these keys already |
| D11 | Completion on macOS: "⌃Space, ↩ insert, ⇥ next" | Tab still inserts, as ↩ does, on every layout | "⇥ next" can be read as "Tab moves to the next row of the list", which would change what Tab does today. Not changed on a guess |
| D12 | Where the connection names no database (MySQL without one), there is no database name to type | The confirmation asks for the connection's name there | Decision 4 has no word for it |
| D14 | Decision 14: a list is picked from with `ctrl+n/p` or `j/k` and `enter` on Omarchy | The completion list has `ctrl+n/p` (and the arrows), the connection picker `j/k`. Quick open keeps the arrows, and has no key to open it on Omarchy anyway. The foreign key picker of a new row is not built | Nothing is added to a list that works; say if quick open and the picker should take `ctrl+n/p` too |
| D13 | macOS labels show the glyphs ⌃ and ⇥ | Task 1a adds ⌃ to the bundled key font from Noto Sans Symbols (OFL, on this machine). ⇥ is written `Tab` on the Mac. (On 2026-10-08 the plan was told "do it" with no row answered: every row stands as its "Built" column says, and this one, which asked for a font to be chosen, took the way that adds none and is one line to turn round) | The Mac look draws with IBM Plex and a cut of Noto Sans Symbols 2 that holds ⌘ ⇧ ⌥ ⌫ ⌦ ⏎. Neither has ⌃ (U+2303) or ⇥ (U+21E5), so both would be drawn as boxes. ⇥ is in none of the Noto symbol or math fonts; DejaVu Sans has it, under its own licence. The other way out is to write `Tab` on the Mac, as the completion list does today |

### Keys that go

These work today and stop working, because the table gives the key to something else or names another key. The before/after table for the pull request is made from this list.

| Look | Today | After | Why |
|---|---|---|---|
| all | Space shows or hides the row panel | Nothing. ⌘I / `enter` opens it, ⇧⌘R / `ctrl+shift+r` still shows and hides it | Space is Boolean cycle (claimed) |
| macOS | ⌘⌫ deletes a connection | ⌫ | The table. **There is no question before a connection is deleted, today or after** |
| macOS, Windows | ⌘N opens a new connection from anywhere | Only in the connection picker; on a table's rows it adds a row, as now | The table's scope for New connection |
| Omarchy | `ctrl+o` Connections | `ctrl+shift+c` | The table |
| Omarchy | `ctrl+1..9` connection | `ctrl+shift+1..9` | The table |
| Omarchy | `1`..`9` tab | `alt+1..9`. The strip still numbers its tabs: the number is the one to hold Alt with | The table |
| Omarchy | `ctrl+n` new connection anywhere | `n` in the picker | The table; `ctrl+n` is "next in a list" |
| Omarchy | `ctrl+p` quick open | `/` in the sidebar focuses its filter. **Quick open has no key on Omarchy** | The table |
| Omarchy | `ctrl+r` refresh | `R` | `ctrl+r` is vim's redo (claimed) |
| Omarchy | `ctrl+.` cancel, from anywhere | `ctrl+c` (see D5). **A table's load cannot be cancelled by key while a cell is being typed in or a filter has the keyboard**: there `ctrl+c` drops the edit, or copies | The table |
| Omarchy | `ctrl+shift+f` format SQL | `=` (see D3) | Decision 9 |
| Omarchy | `ctrl+shift+m` read-only or read-write | `:ro` / `:rw` | Decision 5 |
| Omarchy | `ctrl+shift+d` review SQL | `:diff` only | `ctrl+shift+d` is Switch database |
| Omarchy | `ctrl+i` focus fields, complete | `enter` / `ctrl+l`, and `ctrl+space` | The table |
| Omarchy | `enter` edits the cell | `enter` opens the inspector; `i`, `cc`, `s` edit | Decision 1 |
| Omarchy | `s` Structure view, `d` Data view | **No key.** The views are clicked | `s` replaces the cell; `d` begins `dd` |
| Omarchy | `y` copies the cell, `ctrl+c` too | `v y` | D6 |
| Omarchy | `ctrl+shift+c` copies the row | **No key** | `ctrl+shift+c` is Connections |
| Omarchy | `[` `]` step rows from the grid | From the inspector's fields only; `j` `k` on the grid | On the grid `]` begins `]e` and `]c` |
| Omarchy | `i` shows the row panel of a result row | `enter` | The table |
| Omarchy | `t` tree or flat list, anywhere | In the sidebar | Scoped, so the grid's letters stay free |
| Omarchy | `yy`, `ctrl+d` duplicate a connection; `ctrl+e`, `ctrl+backspace` | `yy p`; `e`; `dd` | Decision 16, the table |
| Omarchy | `ctrl+w` closes the tab while typing | In normal mode only; while typing it deletes a word | The brief's rule for insert mode |

### Keys that stay though the table has none

The table has no key for these on macOS (a dash, "Review SQL button", "segmented switch"). They work today, collide with nothing, and none of the eighteen decisions takes them away, so this plan **keeps** them, in the keymap like the rest. Say which to drop.

| Key | Does |
|---|---|
| ⌘B | Toggle sidebar |
| `?` | Help, all keys |
| ⇧⌘D | Review pending SQL |
| ⇧⌘M | Read-only or read-write runs |
| ⌘I in the SQL editor | Completion (macOS keeps ⌃Space for input sources) |

Kept, and not in the table at all: close tab, close connection, next and previous connection and tab, the filter bar, pages, F6, `alt+enter` for the large editor, and the keys of the dialogs, in every look; copy row (⇧⌘C) on macOS and Windows; `za` and the WHERE line's `/` on Omarchy.

## Runs

The brief asks for one pull request. It is too much for one sitting, so it is three runs on this one branch, and the pull request is opened at the end of the third.

- **Run 1, this plan:** the keymap, the handler and every label reading from it, the settled bindings, and tests 1 to 8 and 10. Nine tasks, in three sittings: tasks 1, 1a, 2 and 3; tasks 4 and 5; tasks 6 to 8. **Stop after each sitting and say where things stand.**
- **Run 2:** Settings → Keyboard (section 4 of the brief, test 9): overrides in `settings.toml` under `[keys.<scope>]`, the conflict check with Swap, `:keys default`, and the Keyboard page in both looks. The Settings window has one page and no page navigation today, so this is the largest new piece. It gets its own plan.
- **Run 3:** the website's three pages of keys and every other mention, `README.md`, the scenes' screenshots, and the pull request with its description. It gets its own plan.

Until run 3 the website's pages name the old keys. That is in the branch only: nothing is published before the pull request merges.

## Before you start

- Cargo is `~/.cargo/bin/cargo` (the mise shim fails). Never point `CARGO_TARGET_DIR` at `/tmp`.
- The four checks, from `AGENTS.md`. "Run the four checks" below means these, and all four pass after every task:

```bash
~/.cargo/bin/cargo fmt --all --check
~/.cargo/bin/cargo clippy --locked --workspace --all-targets -- -D warnings
~/.cargo/bin/cargo test --locked --workspace --all-targets
RUSTDOCFLAGS='-D warnings' ~/.cargo/bin/cargo doc --locked --workspace --no-deps
```

- `src/shots.rs` is outside those four: it is compiled only with its feature. Task 8 runs `~/.cargo/bin/cargo clippy --locked --features shots --lib --tests -- -D warnings`.
- **What no test here can see.** The tests are headless: they send egui the events they choose. Three things the window does to real keys are described at the top of task 2 (the command key with C, X or V arrives as a clipboard event; a digit or a bracket with Shift arrives as what it types). The code reads what the window really sends and the unit tests send exactly that, but nobody has pressed the keys. Task 8 ends with a list for a person to press.
- **What was run where this plan was written.** Task 1's file was built: `src/keymap.rs` as it stands below compiled, its 18 tests passed, and the four checks passed with it in the tree (1883 tests in the app's suite, 1865 before it). The test that no two commands share a key was checked to fail on a planted clash (`gs` shortened to `g`). The draft was then taken out and the tree left clean at `300de6a`. **Tasks 2 to 8 were written from reading the tree, and nothing in them was compiled.** Their code blocks say what the code should come to; expect to correct a name or a borrow on the first build. Where a block and the compiler disagree the compiler is right, and the task's tests say what must hold.
- `$SCRATCH` in a command below is the session's scratchpad directory. Nothing made there is committed.
- **Databases.** Nothing here touches `crates/tabletist-db`. The app's tests use the fake backend. Nothing needs `docker compose`.
- **The branch.** `claude/tabletist-keyboard-shortcuts-fcc089`, from `300de6a`.
- **Tests that pin keys will change, and that is the work.** About 150 lines of `src/ui/mod.rs`' tests assert a painted key (`"ctrl+b tables"`, `"space inspect"`, `"y copy"`), and many more press a key that moves. Change a test to the new key when the brief moved the key; never delete one to get green. A test whose subject is gone (Space toggling the row panel) is rewritten for what took its place, and the commit says so.
- **Dead code is an error here.** Each task orphans helpers of `src/ui/keys.rs` (the `key` and `command` closures, `typed`, `take_press`, `consume_press`, `is_press`, `NUMBERS`' old users) and imports of the files it touches. Delete each in the task that leaves it unused, with its tests if it had its own (`a_press_matches_its_modifiers_as_each_platform_reports_them` goes with `is_press`, and `what_is_held_is_read_as_the_layout_names_it` holds what it held: say so in the commit). `take_enter` and `drop_repeats` stay: the prompts use them.
- House rules that bite here: no em dashes anywhere; comments explain why, in the surrounding code's voice; a view pushes `Action`s and changes no state; views draw text only through `TextRole`s; every behaviour change has a headless test; do not weaken a lint or delete a test to get green.
- **Design material is never a test and never committed.** Fixtures and scenes use the Bookshop data only. No screenshot goes to GitHub unblurred.

## File structure

| File | What it is for |
|---|---|
| `src/keymap.rs` (new) | The bindings as data, `spell`, `written`, `Keymap` with `chords`, `label`, `key`, `typed`, `ex`, and the tests generated from the data |
| `src/ui/keys.rs` | The handler. Loses `SHORTCUTS`, `Holds`, `shortcuts`, `keys_label` and every `Key::X` it names for a command; gains `held_is`, `presses`, `scopes` and the reading of typed letters through the keymap |
| `src/app.rs` | `App::keymap`, `App::layout()` |
| `src/app/editing.rs` | `run_command` reads the prompt's line through `Keymap::ex` |
| `src/ui/help.rs` | Lists the keymap |
| `src/ui/*.rs` that paint a key | Ask `app.keymap` for the label. The list is in each task |
| `src/ui/widgets.rs`, `src/ui/terminal_dialog.rs`, `src/ui/states.rs` | The three painters of a key take a `keymap::Written` and nothing else (task 7) |
| `src/ui/cell_editor.rs` | Reads its own keys (commit, keep, cancel, apply) from the keymap |
| `src/ui/write_prompts.rs` | The terminal's confirmation takes the database's name |
| `tests/keys.rs` (new) | Reads the sources and fails on a key written outside `src/keymap.rs` (test 10), as `tests/engines.rs` does for engine comparisons |
| `assets/fonts/NotoSansSymbols2-Keys.ttf`, `src/typography/fonts.rs` | The glyphs the Mac's labels need (task 1a) |
| `AGENTS.md` | The line about `SHORTCUTS` |

---

### Task 1: The keymap

**Files:**
- Create: `src/keymap.rs`
- Modify: `src/lib.rs` (the list of modules), `src/app.rs` (the `App` struct near line 70, and `App::new`)

- [ ] **Step 1: Add the module**

In `src/lib.rs`, after `pub mod i18n;`:

```rust
pub mod keymap;
```

- [ ] **Step 2: Write `src/keymap.rs`**

The whole file. It was built and passed as it stands here.

```rust
//! Every key the app answers to, as data: the one place a binding is
//! written. The keys' handler reads its chords from here, and so does
//! everything that names a key: the `?` list, the status line's hints, a
//! button's shortcut, a tooltip, the Keyboard page of the Settings.
//!
//! A binding is spelled once, the way the Omarchy look writes it
//! (`ctrl+shift+c`, `yy p`, `:w`), and each layout writes it its own way
//! from that: `⇧⌘F` on macOS, `Ctrl+Shift+F` on Windows.
//!
//! A command that is not built yet is here all the same, with its keys:
//! they are claimed, so nothing else takes them, and nothing answers them.

use egui::Key;

use crate::theme::{Faces, Look};

/// How a look binds its keys and writes them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Layout {
    /// The macOS look: its own keys, written with glyphs.
    Mac,
    /// The Windows look: the Mac's keys with Ctrl for Cmd, written out.
    Windows,
    /// The Omarchy look: vim keys and the `:` prompt, in lower case.
    Omarchy,
}

impl Layout {
    pub const ALL: [Layout; 3] = [Layout::Mac, Layout::Windows, Layout::Omarchy];

    pub fn of(look: &Look) -> Self {
        if look.terminal {
            Self::Omarchy
        } else if look.faces == Faces::Plex {
            Self::Mac
        } else {
            Self::Windows
        }
    }
}

/// Where a binding holds: what has the keyboard.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Scope {
    /// Everywhere but under a prompt.
    Global,
    /// The connection picker.
    Connections,
    /// The tree of tables.
    Sidebar,
    /// A table's rows.
    Grid,
    /// An open editor on a cell or on a field of the inspector.
    CellEditor,
    /// The fields of the row inspector.
    Inspector,
    /// A SQL editor's tab.
    SqlEditor,
    /// A SQL editor's result.
    Results,
    /// A prompt or a dialog: nothing under it has the keys.
    Prompt,
}

impl Scope {
    pub const ALL: [Scope; 9] = [
        Scope::Global,
        Scope::Connections,
        Scope::Sidebar,
        Scope::Grid,
        Scope::CellEditor,
        Scope::Inspector,
        Scope::SqlEditor,
        Scope::Results,
        Scope::Prompt,
    ];

    /// The scope's name in `settings.toml` (`[keys.<scope>]`).
    pub fn id(self) -> &'static str {
        match self {
            Self::Global => "global",
            Self::Connections => "connections",
            Self::Sidebar => "sidebar",
            Self::Grid => "grid",
            Self::CellEditor => "cell-editor",
            Self::Inspector => "inspector",
            Self::SqlEditor => "sql-editor",
            Self::Results => "results",
            Self::Prompt => "prompt",
        }
    }

    /// Whether a key of `self` and a key of `other` can be read in the same
    /// moment: the same scope, the global one with any but a prompt, an
    /// editor with what it was opened from, a result with its editor's tab.
    pub fn alongside(self, other: Scope) -> bool {
        use Scope::{CellEditor, Global, Grid, Inspector, Prompt, Results, SqlEditor};
        let pair = |a, b| (self == a && other == b) || (self == b && other == a);
        self == other
            || (self == Global && other != Prompt)
            || (other == Global && self != Prompt)
            || pair(Grid, CellEditor)
            || pair(Inspector, CellEditor)
            || pair(SqlEditor, Results)
    }
}

/// What must also be on screen for a binding to hold. A binding with one
/// of these takes its key before the same key of its scope with none: that
/// is no conflict.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum When {
    Always,
    /// A connect is under way.
    Connecting,
    /// The cursor is on the grid's last row.
    LastRow,
    /// The row inspector shows.
    InspectorOpen,
    /// The SQL of the pending changes shows.
    Review,
    /// What the last save came to is still said.
    Note,
    /// The large editor is open.
    LargeEditor,
    /// The completion list is open.
    Completion,
    /// An answer of the assistant is offered.
    Suggestion,
    /// The card of a refused write shows in Messages.
    RefusedWrite,
    /// The connection form.
    ConnectionForm,
    /// The Settings screen.
    SettingsScreen,
    /// Quick open.
    QuickOpen,
    /// The question before a save to production.
    ConfirmWrite,
    /// The question about a row that changed on the server.
    Conflict,
    /// The question about leaving with pending changes.
    Leaving,
}

/// The mode a chord holds in: with the keyboard in a text field (insert),
/// or out of one (normal). Typed letters and `:` commands are normal mode's
/// whatever this says.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    /// By the scope: insert in an editor, normal on rows and lists, any
    /// for the global scope and for prompts.
    Auto,
    Normal,
    Insert,
    Any,
}

impl Mode {
    /// Whether a chord of this mode is read with the keyboard out of a
    /// text field (`normal`) or in one.
    pub fn holds(self, normal: bool) -> bool {
        match self {
            Mode::Auto | Mode::Any => true,
            Mode::Normal => normal,
            Mode::Insert => !normal,
        }
    }

    /// Whether a chord of this mode and one of `other` could be read in
    /// the same moment.
    pub fn overlaps(self, other: Mode) -> bool {
        !matches!(
            (self, other),
            (Mode::Normal, Mode::Insert) | (Mode::Insert, Mode::Normal)
        )
    }
}

/// What a spelling is to the command it stands under.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// The command's own key.
    Own,
    /// Another command's key that ends up here (`ctrl+l` moves a pane to
    /// the right, which from the grid is the inspector). Written beside the
    /// command's keys, and bound where it is that command's own.
    Via,
    /// A key the command will have once what it needs is built. Claimed,
    /// written nowhere, answered by nothing.
    Claimed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Chord {
    pub keys: &'static str,
    pub kind: Kind,
}

const fn k(keys: &'static str) -> Chord {
    Chord {
        keys,
        kind: Kind::Own,
    }
}

const fn via(keys: &'static str) -> Chord {
    Chord {
        keys,
        kind: Kind::Via,
    }
}

const fn claimed(keys: &'static str) -> Chord {
    Chord {
        keys,
        kind: Kind::Claimed,
    }
}

/// A command's keys in one layout.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Bound {
    pub chords: &'static [Chord],
    /// How the keys are written together where that is shorter than one
    /// after the other (`j/k`). Empty: one after the other.
    pub shown: &'static str,
    /// What does the command where no key does (`Review SQL button`).
    pub control: &'static str,
    pub mode: Mode,
}

/// No key, and none meant.
const NONE: Bound = Bound {
    chords: &[],
    shown: "",
    control: "",
    mode: Mode::Auto,
};

const fn keys(chords: &'static [Chord]) -> Bound {
    Bound { chords, ..NONE }
}

const fn control(control: &'static str) -> Bound {
    Bound { control, ..NONE }
}

impl Bound {
    const fn shown(self, shown: &'static str) -> Self {
        Self { shown, ..self }
    }

    const fn normal(self) -> Self {
        Self {
            mode: Mode::Normal,
            ..self
        }
    }

    const fn any(self) -> Self {
        Self {
            mode: Mode::Any,
            ..self
        }
    }
}

/// The groups the Keyboard page lists the commands in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Group {
    Window,
    Navigation,
    Editing,
    Sql,
    Ai,
    /// The keys of a prompt or a dialog: its own, and not rebound.
    Prompts,
}

macro_rules! commands {
    ($($name:ident),* $(,)?) => {
        /// Everything a key can ask for.
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
        pub enum Command {
            $($name),*
        }

        impl Command {
            pub const ALL: &'static [Command] = &[$(Command::$name),*];
        }
    };
}

commands! {
    // Window and connections.
    Connections,
    GoToConnectionWindow,
    SwitchDatabase,
    Settings,
    CloseConnection,
    NextConnection,
    PreviousConnection,
    CancelConnecting,
    NewConnection,
    EditConnection,
    DuplicateConnection,
    DeleteConnection,
    Connect,
    ConnectAgain,
    MoveInConnections,
    FilterConnections,
    // Navigation.
    FindObject,
    NewSqlTab,
    ToggleSidebar,
    PaneLeft,
    PaneDown,
    PaneUp,
    PaneRight,
    NextPart,
    PreviousPart,
    SwitchTab,
    CloseTab,
    PreviousTab,
    NextTab,
    MoveInTree,
    OpenObject,
    ToggleTree,
    Reload,
    FilterBar,
    WhereFilter,
    PreviousPage,
    NextPage,
    Help,
    KeysDefault,
    // Grid and editing.
    MoveRow,
    MoveColumn,
    MovePage,
    OpenInspector,
    ToggleInspector,
    CloseInspector,
    BackToGrid,
    MoveField,
    PreviousRow,
    NextRow,
    FoldDocuments,
    EditCell,
    ReplaceCell,
    CommitDown,
    CommitNext,
    CommitPrevious,
    KeepEdit,
    CancelEdit,
    OpenLargeEditor,
    ApplyLargeEditor,
    KeepLargeEditor,
    CancelLargeEditor,
    OpenInEditor,
    SetNull,
    SetDefault,
    UndoCell,
    RedoCell,
    CopyCells,
    CopyRow,
    PasteCells,
    PasteNewRows,
    AddRow,
    AddRowAbove,
    AddRowAtEnd,
    DuplicateRow,
    DuplicateRowAbove,
    DeleteRow,
    OpenReferencedRow,
    ShowSavedRow,
    ReviewSql,
    NextChange,
    CopyReviewSql,
    CloseReview,
    DismissNote,
    SaveChanges,
    DiscardChanges,
    NextError,
    PreviousError,
    BooleanCycle,
    StepUp,
    StepDown,
    Format,
    // SQL editor.
    RunStatement,
    RunAll,
    Explain,
    ExplainAnalyze,
    ComparePreviousRun,
    CancelQuery,
    FocusResults,
    LeaveEditor,
    Complete,
    InsertCompletion,
    NextCompletion,
    PreviousCompletion,
    CloseCompletion,
    GoToLine,
    History,
    ReadOnlyTab,
    ReadWriteTab,
    ToggleSqlMode,
    Commit,
    Rollback,
    NextPlanTip,
    EditRefusedConnection,
    AllowRefusedWrite,
    // AI assistant.
    AskAi,
    AcceptAi,
    AcceptAiAndRun,
    DiscardAi,
    // Prompts and dialogs.
    ConfirmProductionWrite,
    CancelProductionWrite,
    ScrollStatements,
    LeaveWrite,
    LeaveDiscard,
    LeaveStay,
    ConflictOverwrite,
    ConflictUseServer,
    ConflictKeepMine,
    ConflictDiscard,
    FormPasteUrl,
    FormNextField,
    FormTest,
    FormSave,
    FormSaveAndConnect,
    FormCancel,
    SettingsMove,
    SettingsChange,
    SettingsToggle,
    SettingsOpenFile,
    SettingsReset,
    SettingsClose,
    QuickOpenMove,
    QuickOpenPick,
    QuickOpenClose,
}

/// What is said of a command wherever it is listed.
#[derive(Clone, Copy, Debug)]
pub struct Info {
    /// The command's own name.
    pub name: &'static str,
    /// The row of the settled keymap it belongs to, by the row's name. A
    /// row can hold several commands (`Undo / redo one cell`). Empty: a key
    /// the app had before the keymap was settled, kept beside it.
    pub row: &'static str,
    pub group: Group,
    /// Whether anything answers the command yet.
    pub built: bool,
}

const fn info(name: &'static str, row: &'static str, group: Group) -> Info {
    Info {
        name,
        row,
        group,
        built: true,
    }
}

impl Info {
    const fn reserved(self) -> Self {
        Self {
            built: false,
            ..self
        }
    }
}

impl Command {
    pub fn info(self) -> Info {
        use Command as C;
        use Group::{Ai, Editing, Navigation, Prompts, Sql, Window};
        const PANES: &str = "Move between panes";
        const INSPECTOR_ROWS: &str = "Previous / next row in inspector";
        const COMMIT: &str = "Commit cell and move";
        const UNDO: &str = "Undo / redo one cell";
        const COPY: &str = "Copy / paste cell values";
        const ERRORS: &str = "Next / previous error";
        const LARGE: &str = "Apply / cancel large editor";
        const STEP: &str = "Step number / date part";
        const MODE: &str = "Read-only / read-write tab";
        const ANSWER: &str = "Accept / accept and run / discard AI";
        match self {
            C::Connections => info("Connections", "Connections", Window),
            C::GoToConnectionWindow => {
                info("Go to connection window", "Go to connection window", Window)
            }
            C::SwitchDatabase => info("Switch database", "Switch database", Window).reserved(),
            C::Settings => info("Settings", "Settings", Window),
            C::CloseConnection => info("Close connection", "", Window),
            C::NextConnection => info("Next connection", "", Window),
            C::PreviousConnection => info("Previous connection", "", Window),
            C::CancelConnecting => info("Cancel connecting", "", Window),
            C::NewConnection => info("New connection", "New connection", Window),
            C::EditConnection => info("Edit connection", "Edit connection", Window),
            C::DuplicateConnection => info("Duplicate connection", "Duplicate connection", Window),
            C::DeleteConnection => info("Delete connection", "Delete connection", Window),
            C::Connect => info("Connect / go to window", "Connect / go to window", Window),
            C::ConnectAgain => info("Open the connection again", "", Window),
            C::MoveInConnections => info("Move in the connections", "", Window),
            C::FilterConnections => info("Filter the connections", "", Window),
            C::FindObject => info("Find any object", "Find any object", Navigation),
            C::NewSqlTab => info("New SQL editor tab", "New SQL editor tab", Navigation),
            C::ToggleSidebar => info("Toggle sidebar", "Toggle sidebar", Navigation),
            C::PaneLeft => info("Pane to the left", PANES, Navigation),
            C::PaneDown => info("Pane below", PANES, Navigation).reserved(),
            C::PaneUp => info("Pane above", PANES, Navigation).reserved(),
            C::PaneRight => info("Pane to the right", PANES, Navigation),
            C::NextPart => info("Next part of the window", "", Navigation),
            C::PreviousPart => info("Previous part of the window", "", Navigation),
            C::SwitchTab => info("Switch tab", "Switch tab", Navigation),
            C::CloseTab => info("Close tab", "", Navigation),
            C::PreviousTab => info("Previous tab", "", Navigation),
            C::NextTab => info("Next tab", "", Navigation),
            C::MoveInTree => info("Move in the sidebar", "", Navigation),
            C::OpenObject => info("Open the table or view", "", Navigation),
            C::ToggleTree => info("Tree or flat list", "", Navigation),
            C::Reload => info("Reload", "Reload", Navigation),
            C::FilterBar => info("Filter bar", "", Navigation),
            C::WhereFilter => info("Filter with a WHERE line", "", Navigation),
            C::PreviousPage => info("Previous page", "", Navigation),
            C::NextPage => info("Next page", "", Navigation),
            C::Help => info("Help, all keys", "Help, all keys", Navigation),
            C::KeysDefault => info("Write the full list of keys", "", Navigation).reserved(),
            C::MoveRow => info("Move by row", "", Editing),
            C::MoveColumn => info("Move by column", "", Editing),
            C::MovePage => info("Move by page", "", Editing),
            C::OpenInspector => info(
                "Open inspector / focus fields",
                "Open inspector / focus fields",
                Editing,
            ),
            C::ToggleInspector => info("Show or hide the inspector", "", Editing),
            C::CloseInspector => info("Close the inspector", "", Editing),
            C::BackToGrid => info(
                "Back to grid from inspector",
                "Back to grid from inspector",
                Editing,
            ),
            C::MoveField => info("Move by field", "", Editing),
            C::PreviousRow => info("Previous row in inspector", INSPECTOR_ROWS, Editing),
            C::NextRow => info("Next row in inspector", INSPECTOR_ROWS, Editing),
            C::FoldDocuments => info("Fold the documents", "", Editing),
            C::EditCell => info("Edit cell", "Edit cell", Editing),
            C::ReplaceCell => info("Edit cell from nothing", "Edit cell", Editing),
            C::CommitDown => info("Commit cell and move down", COMMIT, Editing),
            C::CommitNext => info("Commit cell and move to the next", COMMIT, Editing),
            C::CommitPrevious => info("Commit cell and move to the previous", COMMIT, Editing),
            C::KeepEdit => info("Leave the editor and keep the edit", COMMIT, Editing),
            C::CancelEdit => info("Cancel cell edit", "Cancel cell edit", Editing),
            C::OpenLargeEditor => info("Open the large editor", "", Editing),
            C::ApplyLargeEditor => info("Apply large editor", LARGE, Editing),
            C::KeepLargeEditor => info("Leave the large editor and keep the text", "", Editing),
            C::CancelLargeEditor => info("Cancel large editor", LARGE, Editing),
            C::OpenInEditor => {
                info("Open value in $EDITOR", "Open value in $EDITOR", Editing).reserved()
            }
            C::SetNull => info("Set NULL", "Set NULL", Editing),
            C::SetDefault => info("Set DEFAULT", "Set DEFAULT", Editing).reserved(),
            C::UndoCell => info("Undo one cell", UNDO, Editing),
            C::RedoCell => info("Redo one cell", UNDO, Editing).reserved(),
            C::CopyCells => info("Copy cell values", COPY, Editing),
            C::CopyRow => info("Copy row", "", Editing),
            C::PasteCells => info("Paste cell values", COPY, Editing).reserved(),
            C::PasteNewRows => info("Paste as new rows", "Paste as new rows", Editing).reserved(),
            C::AddRow => info("Add row", "Add row", Editing),
            C::AddRowAbove => info("Add row above", "Add row", Editing),
            C::AddRowAtEnd => info("Add row after the last", "Add row", Editing).reserved(),
            C::DuplicateRow => info("Duplicate row", "Duplicate row", Editing).reserved(),
            C::DuplicateRowAbove => {
                info("Duplicate row above", "Duplicate row", Editing).reserved()
            }
            C::DeleteRow => info("Delete row", "Delete row", Editing),
            C::OpenReferencedRow => info("Open referenced row", "Open referenced row", Editing),
            C::ShowSavedRow => info(
                "Show saved row in sorted position",
                "Show saved row in sorted position",
                Editing,
            )
            .reserved(),
            C::ReviewSql => info("Review pending SQL", "Review pending SQL", Editing),
            C::NextChange => info("Next change", "Review pending SQL", Editing).reserved(),
            C::CopyReviewSql => info("Copy the pending SQL", "", Editing),
            C::CloseReview => info("Close the pending SQL", "", Editing),
            C::DismissNote => info("Dismiss what the last save came to", "", Editing),
            C::SaveChanges => info("Save changes", "Save changes", Editing),
            C::DiscardChanges => info("Discard all pending", "Discard all pending", Editing),
            C::NextError => info("Next error", ERRORS, Editing).reserved(),
            C::PreviousError => info("Previous error", ERRORS, Editing).reserved(),
            C::BooleanCycle => info("Boolean cycle", "Boolean cycle", Editing).reserved(),
            C::StepUp => info("Step up", STEP, Editing).reserved(),
            C::StepDown => info("Step down", STEP, Editing).reserved(),
            C::Format => info("Format SQL or JSON", "Format SQL or JSON", Sql),
            C::RunStatement => info("Run statement", "Run statement", Sql),
            C::RunAll => info("Run all", "Run all", Sql),
            C::Explain => info("Explain", "Explain", Sql).reserved(),
            C::ExplainAnalyze => info("Explain analyze", "Explain analyze", Sql).reserved(),
            C::ComparePreviousRun => info(
                "Compare with previous run",
                "Compare with previous run",
                Sql,
            )
            .reserved(),
            C::CancelQuery => info("Cancel running query", "Cancel running query", Sql),
            C::FocusResults => info("Focus results", "Focus results", Sql).reserved(),
            C::LeaveEditor => info("Leave the editor", "", Sql),
            C::Complete => info("Completion", "Completion", Sql),
            C::InsertCompletion => info("Insert the completion", "Completion", Sql),
            C::NextCompletion => info("Next completion", "Completion", Sql),
            C::PreviousCompletion => info("Previous completion", "Completion", Sql),
            C::CloseCompletion => info("Close the completion list", "", Sql),
            C::GoToLine => info("Go to line", "Go to line", Sql).reserved(),
            C::History => info("History", "History", Sql).reserved(),
            C::ReadOnlyTab => info("Read-only tab", MODE, Sql),
            C::ReadWriteTab => info("Read-write tab", MODE, Sql),
            C::ToggleSqlMode => info("Read-only or read-write runs", "", Sql),
            C::Commit => info("Commit", "Commit", Sql).reserved(),
            C::Rollback => info("Rollback", "Rollback", Sql).reserved(),
            C::NextPlanTip => info("Next plan tip", "Next plan tip", Sql).reserved(),
            C::EditRefusedConnection => info("Edit the connection that refused", "", Prompts),
            C::AllowRefusedWrite => info("Allow writes, or run again", "", Prompts),
            C::AskAi => info("Ask AI", "Ask AI", Ai).reserved(),
            C::AcceptAi => info("Accept AI", ANSWER, Ai).reserved(),
            C::AcceptAiAndRun => info("Accept AI and run", ANSWER, Ai).reserved(),
            C::DiscardAi => info("Discard AI", ANSWER, Ai).reserved(),
            C::ConfirmProductionWrite => info(
                "Confirm production write",
                "Confirm production write",
                Prompts,
            ),
            C::CancelProductionWrite => info("Cancel the production write", "", Prompts),
            C::ScrollStatements => info("Scroll the statements", "", Prompts),
            C::LeaveWrite => info("Write and leave", "", Prompts),
            C::LeaveDiscard => info("Discard and leave", "", Prompts),
            C::LeaveStay => info("Stay", "", Prompts),
            C::ConflictOverwrite => info("Overwrite", "", Prompts),
            C::ConflictUseServer => info("Use the server's values", "", Prompts),
            C::ConflictKeepMine => info("Keep mine and reload", "", Prompts),
            C::ConflictDiscard => info("Discard my changes", "", Prompts),
            C::FormPasteUrl => info("Paste a URL", "", Prompts),
            C::FormNextField => info("Next field", "", Prompts),
            C::FormTest => info("Test the connection", "", Prompts),
            C::FormSave => info("Save the connection", "", Prompts),
            C::FormSaveAndConnect => info("Save and connect", "", Prompts),
            C::FormCancel => info("Cancel", "", Prompts),
            C::SettingsMove => info("Move in the settings", "", Prompts),
            C::SettingsChange => info("Change the option", "", Prompts),
            C::SettingsToggle => info("Toggle the option", "", Prompts),
            C::SettingsOpenFile => info("Open the settings file in $EDITOR", "", Prompts),
            C::SettingsReset => info("Reset the option", "", Prompts),
            C::SettingsClose => info("Close the settings", "", Prompts),
            C::QuickOpenMove => info("Move in quick open", "", Prompts),
            C::QuickOpenPick => info("Open the one selected", "", Prompts),
            C::QuickOpenClose => info("Close quick open", "", Prompts),
        }
    }
}

/// One line of the keymap: a command, where it holds, and its keys in the
/// Mac's layout and in Omarchy's. Windows has the Mac's with Ctrl for Cmd.
#[derive(Clone, Copy, Debug)]
pub struct Binding {
    pub command: Command,
    pub scopes: &'static [Scope],
    pub when: When,
    pub mac: Bound,
    pub omarchy: Bound,
}

impl Binding {
    pub fn bound(&self, layout: Layout) -> &Bound {
        match layout {
            Layout::Mac | Layout::Windows => &self.mac,
            Layout::Omarchy => &self.omarchy,
        }
    }
}

impl Binding {
    /// The mode `spelled`, one of this line's keys, holds in where the
    /// line is read in `scope`. Never `Auto`.
    pub fn mode(&self, scope: Scope, layout: Layout, spelled: &Spelled) -> Mode {
        let typed = match spelled {
            Spelled::Ex(_) => true,
            Spelled::Digits(_) => false,
            Spelled::Strokes(strokes) => strokes
                .iter()
                .any(|stroke| matches!(stroke, Stroke::Text(_))),
        };
        // A prompt has no modes: its letters are its own.
        if scope == Scope::Prompt {
            return Mode::Any;
        }
        // Typed letters and the lines of the prompt are normal mode's.
        if typed {
            return Mode::Normal;
        }
        match self.bound(layout).mode {
            Mode::Auto => match scope {
                Scope::CellEditor | Scope::SqlEditor => Mode::Insert,
                Scope::Global | Scope::Prompt => Mode::Any,
                Scope::Connections
                | Scope::Sidebar
                | Scope::Grid
                | Scope::Inspector
                | Scope::Results => Mode::Normal,
            },
            set => set,
        }
    }
}

const fn bind(command: Command, scopes: &'static [Scope], mac: Bound, omarchy: Bound) -> Binding {
    Binding {
        command,
        scopes,
        when: When::Always,
        mac,
        omarchy,
    }
}

impl Binding {
    const fn when(self, when: When) -> Self {
        Self { when, ..self }
    }
}

const GLOBAL: &[Scope] = &[Scope::Global];
const CONNECTIONS: &[Scope] = &[Scope::Connections];
const SIDEBAR: &[Scope] = &[Scope::Sidebar];
const GRID: &[Scope] = &[Scope::Grid];
const CELL_EDITOR: &[Scope] = &[Scope::CellEditor];
const INSPECTOR: &[Scope] = &[Scope::Inspector];
const SQL_EDITOR: &[Scope] = &[Scope::SqlEditor];
const RESULTS: &[Scope] = &[Scope::Results];
const PROMPT: &[Scope] = &[Scope::Prompt];
const ROWS: &[Scope] = &[Scope::Grid, Scope::Results];
const CELLS: &[Scope] = &[Scope::Grid, Scope::Inspector];

/// The keymap. The rows of the settled table first in each part, then the
/// keys the app had beside them.
pub const BINDINGS: &[Binding] = {
    use Command as C;
    &[
        // Window and connections.
        bind(
            C::Connections,
            GLOBAL,
            keys(&[k("cmd+o")]),
            keys(&[k("ctrl+shift+c")]),
        ),
        bind(
            C::GoToConnectionWindow,
            GLOBAL,
            keys(&[k("cmd+1..9")]),
            keys(&[k("ctrl+shift+1..9")]),
        ),
        bind(C::SwitchDatabase, GLOBAL, NONE, keys(&[k("ctrl+shift+d")])),
        bind(
            C::Settings,
            GLOBAL,
            keys(&[k("cmd+,")]),
            keys(&[k("ctrl+,")]),
        ),
        bind(
            C::CloseConnection,
            GLOBAL,
            keys(&[k("cmd+shift+w")]),
            keys(&[k("ctrl+shift+w")]),
        ),
        bind(
            C::NextConnection,
            GLOBAL,
            keys(&[k("ctrl+tab")]),
            keys(&[k("ctrl+tab")]),
        ),
        bind(
            C::PreviousConnection,
            GLOBAL,
            keys(&[k("ctrl+shift+tab")]),
            keys(&[k("ctrl+shift+tab")]),
        ),
        bind(
            C::CancelConnecting,
            GLOBAL,
            keys(&[k("esc")]),
            keys(&[k("esc")]),
        )
        .when(When::Connecting),
        bind(
            C::NewConnection,
            CONNECTIONS,
            keys(&[k("cmd+n")]).any(),
            keys(&[k("n")]),
        ),
        bind(
            C::EditConnection,
            CONNECTIONS,
            keys(&[k("cmd+e")]),
            keys(&[k("e")]),
        ),
        bind(
            C::DuplicateConnection,
            CONNECTIONS,
            keys(&[k("cmd+d")]),
            keys(&[k("yy p")]),
        ),
        bind(
            C::DeleteConnection,
            CONNECTIONS,
            keys(&[k("backspace")]),
            keys(&[k("dd")]),
        ),
        bind(
            C::Connect,
            CONNECTIONS,
            keys(&[k("enter")]),
            keys(&[k("enter")]),
        ),
        bind(
            C::ConnectAgain,
            CONNECTIONS,
            keys(&[k("shift+enter")]),
            keys(&[k("shift+enter")]),
        ),
        bind(
            C::MoveInConnections,
            CONNECTIONS,
            keys(&[k("up"), k("down")]),
            keys(&[k("j"), k("k"), k("up"), k("down")]).shown("j/k"),
        ),
        bind(C::FilterConnections, CONNECTIONS, NONE, keys(&[k("/")])),
        // Navigation.
        bind(C::FindObject, GLOBAL, keys(&[k("cmd+p")]), NONE),
        bind(C::FindObject, SIDEBAR, NONE, keys(&[k("/")])),
        bind(
            C::NewSqlTab,
            GLOBAL,
            keys(&[k("cmd+t")]),
            keys(&[k("ctrl+t")]),
        ),
        bind(
            C::ToggleSidebar,
            GLOBAL,
            keys(&[k("cmd+b")]),
            keys(&[k("ctrl+b")]),
        ),
        bind(C::PaneLeft, GLOBAL, NONE, keys(&[k("ctrl+h")]).normal()),
        bind(C::PaneDown, GLOBAL, NONE, keys(&[k("ctrl+j")]).normal()),
        bind(C::PaneUp, GLOBAL, NONE, keys(&[k("ctrl+k")]).normal()),
        bind(C::PaneRight, GLOBAL, NONE, keys(&[k("ctrl+l")]).normal()),
        bind(C::NextPart, GLOBAL, keys(&[k("f6")]), keys(&[k("f6")])),
        bind(
            C::PreviousPart,
            GLOBAL,
            keys(&[k("shift+f6")]),
            keys(&[k("shift+f6")]),
        ),
        bind(C::SwitchTab, GLOBAL, NONE, keys(&[k("alt+1..9")])),
        bind(
            C::CloseTab,
            GLOBAL,
            keys(&[k("cmd+w")]),
            // In insert mode the chord is vim's: it deletes a word.
            keys(&[k("ctrl+w")]).normal(),
        ),
        bind(
            C::PreviousTab,
            GLOBAL,
            keys(&[k("cmd+shift+[")]),
            keys(&[k("ctrl+shift+[")]),
        ),
        bind(
            C::NextTab,
            GLOBAL,
            keys(&[k("cmd+shift+]")]),
            keys(&[k("ctrl+shift+]")]),
        ),
        bind(
            C::MoveInTree,
            SIDEBAR,
            keys(&[
                k("up"),
                k("down"),
                k("left"),
                k("right"),
                k("home"),
                k("end"),
            ]),
            keys(&[
                k("j"),
                k("k"),
                k("up"),
                k("down"),
                k("left"),
                k("right"),
                k("home"),
                k("end"),
            ])
            .shown("j/k"),
        ),
        bind(
            C::OpenObject,
            SIDEBAR,
            keys(&[k("enter")]),
            keys(&[k("enter")]),
        ),
        bind(C::ToggleTree, SIDEBAR, NONE, keys(&[k("t")])),
        bind(
            C::Reload,
            &[Scope::Grid, Scope::Sidebar],
            keys(&[k("cmd+r")]).any(),
            keys(&[k("R")]),
        ),
        bind(
            C::FilterBar,
            GRID,
            keys(&[k("cmd+f")]).any(),
            keys(&[k("ctrl+f")]).any(),
        ),
        bind(C::WhereFilter, GRID, NONE, keys(&[k("/")])),
        bind(
            C::PreviousPage,
            GRID,
            keys(&[k("cmd+alt+left")]).any(),
            keys(&[k("ctrl+alt+left")]).any(),
        ),
        bind(
            C::NextPage,
            GRID,
            keys(&[k("cmd+alt+right")]).any(),
            keys(&[k("ctrl+alt+right")]).any(),
        ),
        bind(C::Help, GLOBAL, keys(&[k("?")]), keys(&[k("?")])),
        bind(C::KeysDefault, GLOBAL, NONE, keys(&[k(":keys default")])),
        // Grid and editing.
        bind(
            C::MoveRow,
            ROWS,
            keys(&[k("up"), k("down")]),
            keys(&[k("j"), k("k"), k("up"), k("down")]).shown("j/k"),
        ),
        bind(
            C::MoveColumn,
            ROWS,
            keys(&[k("left"), k("right")]),
            keys(&[k("h"), k("l"), k("left"), k("right")]).shown("h/l"),
        ),
        bind(
            C::MovePage,
            ROWS,
            keys(&[k("pgup"), k("pgdn"), k("home"), k("end")]),
            keys(&[k("pgup"), k("pgdn"), k("home"), k("end")]),
        ),
        bind(
            C::OpenInspector,
            GRID,
            keys(&[k("cmd+i")]).any(),
            keys(&[k("enter"), via("ctrl+l")]),
        ),
        bind(C::OpenInspector, RESULTS, NONE, keys(&[k("enter")])),
        bind(
            C::ToggleInspector,
            ROWS,
            keys(&[k("cmd+shift+r")]).any(),
            keys(&[k("ctrl+shift+r")]).any(),
        ),
        bind(C::CloseInspector, ROWS, NONE, keys(&[k("esc")])).when(When::InspectorOpen),
        bind(
            C::BackToGrid,
            INSPECTOR,
            keys(&[k("esc")]),
            keys(&[via("ctrl+h"), k("esc")]),
        ),
        bind(
            C::MoveField,
            INSPECTOR,
            keys(&[k("up"), k("down")]),
            keys(&[k("j"), k("k")]).shown("j/k"),
        ),
        bind(
            C::PreviousRow,
            INSPECTOR,
            control("↑↓ in header"),
            keys(&[k("[")]),
        ),
        bind(
            C::NextRow,
            INSPECTOR,
            control("↑↓ in header"),
            keys(&[k("]")]),
        ),
        bind(
            C::FoldDocuments,
            &[Scope::Grid, Scope::Inspector, Scope::Results],
            NONE,
            keys(&[k("za")]),
        ),
        bind(
            C::EditCell,
            GRID,
            keys(&[k("enter"), k("f2")]),
            keys(&[k("i")]),
        ),
        bind(
            C::EditCell,
            INSPECTOR,
            keys(&[k("enter"), k("f2")]),
            keys(&[k("enter"), k("i")]),
        ),
        bind(C::ReplaceCell, CELLS, NONE, keys(&[k("cc"), k("s")])),
        bind(
            C::CommitDown,
            CELL_EDITOR,
            keys(&[k("enter")]),
            keys(&[k("enter")]),
        ),
        bind(
            C::CommitNext,
            CELL_EDITOR,
            keys(&[k("tab")]),
            keys(&[k("tab")]),
        ),
        bind(
            C::CommitPrevious,
            CELL_EDITOR,
            keys(&[k("shift+tab")]),
            keys(&[k("shift+tab")]),
        ),
        bind(C::KeepEdit, CELL_EDITOR, NONE, keys(&[k("esc")])),
        bind(
            C::CancelEdit,
            CELL_EDITOR,
            keys(&[k("esc")]),
            keys(&[k("ctrl+c")]),
        ),
        bind(
            C::OpenLargeEditor,
            CELL_EDITOR,
            keys(&[k("alt+enter")]),
            keys(&[k("alt+enter")]),
        ),
        bind(
            C::ApplyLargeEditor,
            CELL_EDITOR,
            keys(&[k("cmd+enter")]),
            // `:wq` needs a normal mode in the editor, which it has not.
            keys(&[claimed(":wq"), k("ctrl+enter")]),
        )
        .when(When::LargeEditor),
        bind(C::KeepLargeEditor, CELL_EDITOR, NONE, keys(&[k("esc")])).when(When::LargeEditor),
        bind(
            C::CancelLargeEditor,
            CELL_EDITOR,
            keys(&[k("esc")]),
            keys(&[claimed(":q!"), k("ctrl+c")]),
        )
        .when(When::LargeEditor),
        bind(C::OpenInEditor, CELL_EDITOR, NONE, keys(&[k("ctrl+e")])),
        bind(
            C::SetNull,
            CELLS,
            // Only on a selected cell that is not being edited: in a text
            // field the chord deletes to the start of the line.
            keys(&[k("cmd+backspace")]).normal(),
            keys(&[k("x")]),
        ),
        bind(C::SetDefault, CELLS, keys(&[k("cmd+'")]), keys(&[k("D")])),
        bind(C::UndoCell, CELLS, keys(&[k("cmd+z")]), keys(&[k("u")])),
        bind(
            C::RedoCell,
            CELLS,
            keys(&[k("cmd+shift+z")]),
            keys(&[k("ctrl+r")]).normal(),
        ),
        bind(C::CopyCells, GRID, keys(&[k("cmd+c")]), keys(&[k("v y")])),
        bind(C::CopyRow, GRID, keys(&[k("cmd+shift+c")]), NONE),
        bind(C::PasteCells, GRID, keys(&[k("cmd+v")]), keys(&[k("v p")])),
        bind(
            C::PasteNewRows,
            GRID,
            keys(&[k("cmd+shift+v")]),
            keys(&[k("\"+p"), k("ctrl+shift+v")]),
        ),
        bind(C::AddRow, GRID, keys(&[k("cmd+n")]).any(), keys(&[k("o")])),
        bind(C::AddRowAbove, GRID, NONE, keys(&[k("O")])),
        bind(C::AddRowAtEnd, GRID, keys(&[k("down")]), NONE).when(When::LastRow),
        bind(
            C::DuplicateRow,
            GRID,
            keys(&[k("cmd+d")]),
            keys(&[k("yy p")]),
        ),
        bind(C::DuplicateRowAbove, GRID, NONE, keys(&[k("yy P")])),
        bind(
            C::DeleteRow,
            GRID,
            keys(&[k("backspace"), k("delete")]),
            keys(&[k("dd")]),
        ),
        bind(C::OpenReferencedRow, GRID, NONE, keys(&[k("gd")])),
        bind(C::ShowSavedRow, GRID, NONE, keys(&[k("gs")])),
        bind(
            C::ReviewSql,
            GRID,
            keys(&[k("cmd+shift+d")]).any(),
            keys(&[k(":diff")]),
        ),
        bind(C::NextChange, GRID, NONE, keys(&[k("]c")])),
        bind(C::CopyReviewSql, GRID, NONE, keys(&[k("Y")])).when(When::Review),
        bind(C::CloseReview, GRID, NONE, keys(&[k("esc")])).when(When::Review),
        bind(C::DismissNote, GRID, NONE, keys(&[k("esc")])).when(When::Note),
        bind(
            C::SaveChanges,
            GRID,
            keys(&[k("cmd+s")]).any(),
            keys(&[k(":w"), k("ctrl+s")]).any(),
        ),
        bind(
            C::DiscardChanges,
            GRID,
            keys(&[k("cmd+alt+backspace")]),
            keys(&[k(":e!")]),
        ),
        bind(
            C::NextError,
            GRID,
            keys(&[k("cmd+alt+down")]),
            keys(&[k("]e")]),
        ),
        bind(
            C::PreviousError,
            GRID,
            keys(&[k("cmd+alt+up")]),
            keys(&[k("[e")]),
        ),
        bind(
            C::NextError,
            SQL_EDITOR,
            keys(&[k("cmd+alt+down")]).any(),
            keys(&[k("]e")]),
        ),
        bind(
            C::PreviousError,
            SQL_EDITOR,
            keys(&[k("cmd+alt+up")]).any(),
            keys(&[k("[e")]),
        ),
        bind(
            C::BooleanCycle,
            GRID,
            keys(&[k("space")]),
            keys(&[k("space")]),
        ),
        bind(
            C::StepUp,
            CELL_EDITOR,
            keys(&[k("up")]),
            keys(&[k("ctrl+a")]),
        ),
        bind(
            C::StepDown,
            CELL_EDITOR,
            keys(&[k("down")]),
            keys(&[k("ctrl+x")]),
        ),
        bind(
            C::Format,
            SQL_EDITOR,
            keys(&[k("cmd+shift+f")]).any(),
            keys(&[k("=")]),
        ),
        bind(
            C::Format,
            CELL_EDITOR,
            keys(&[k("cmd+shift+f")]),
            // `=` needs a normal mode in the editor, which it has not.
            keys(&[claimed("="), k("ctrl+shift+f")]),
        )
        .when(When::LargeEditor),
        // SQL editor.
        bind(
            C::RunStatement,
            SQL_EDITOR,
            keys(&[k("cmd+enter")]).any(),
            keys(&[k("ctrl+enter")]).any(),
        ),
        bind(
            C::RunAll,
            SQL_EDITOR,
            keys(&[k("cmd+shift+enter")]).any(),
            keys(&[k("ctrl+shift+enter")]).any(),
        ),
        bind(
            C::Explain,
            SQL_EDITOR,
            keys(&[k("cmd+e")]).any(),
            keys(&[k("ctrl+e")]).any(),
        ),
        bind(C::ExplainAnalyze, SQL_EDITOR, NONE, NONE),
        bind(
            C::ComparePreviousRun,
            SQL_EDITOR,
            keys(&[k("cmd+alt+e")]).any(),
            NONE,
        ),
        bind(
            C::CancelQuery,
            SQL_EDITOR,
            keys(&[k("cmd+.")]).any(),
            // While a query runs. With none running the chord is the text
            // field's own.
            keys(&[k("ctrl+c")]).any(),
        ),
        bind(
            C::CancelQuery,
            GRID,
            keys(&[k("cmd+.")]).any(),
            // Normal mode only: in a cell's editor the chord drops the
            // edit, and in another field it is the copy.
            keys(&[k("ctrl+c")]),
        ),
        bind(C::FocusResults, SQL_EDITOR, NONE, keys(&[k("ctrl+j")])),
        bind(
            C::LeaveEditor,
            SQL_EDITOR,
            keys(&[k("esc")]),
            keys(&[k("esc")]),
        ),
        bind(
            C::Complete,
            SQL_EDITOR,
            keys(&[k("ctrl+space"), k("cmd+i")]),
            keys(&[k("ctrl+space")]),
        ),
        bind(
            C::InsertCompletion,
            SQL_EDITOR,
            keys(&[k("enter"), k("tab")]),
            keys(&[k("tab"), k("enter")]),
        )
        .when(When::Completion),
        bind(
            C::NextCompletion,
            SQL_EDITOR,
            keys(&[k("down")]),
            keys(&[k("ctrl+n"), k("down")]),
        )
        .when(When::Completion),
        bind(
            C::PreviousCompletion,
            SQL_EDITOR,
            keys(&[k("up")]),
            keys(&[k("ctrl+p"), k("up")]),
        )
        .when(When::Completion),
        bind(
            C::CloseCompletion,
            SQL_EDITOR,
            keys(&[k("esc")]),
            keys(&[k("esc")]),
        )
        .when(When::Completion),
        bind(
            C::GoToLine,
            SQL_EDITOR,
            keys(&[k("cmd+l")]),
            keys(&[k(":N")]),
        ),
        bind(C::History, SQL_EDITOR, NONE, keys(&[k(":history")])),
        bind(
            C::ReadOnlyTab,
            SQL_EDITOR,
            control("segmented switch"),
            keys(&[k(":ro")]),
        ),
        bind(
            C::ReadWriteTab,
            SQL_EDITOR,
            control("segmented switch"),
            keys(&[k(":rw")]),
        ),
        bind(
            C::ToggleSqlMode,
            SQL_EDITOR,
            keys(&[k("cmd+shift+m")]).any(),
            NONE,
        ),
        bind(
            C::Commit,
            SQL_EDITOR,
            keys(&[k("cmd+s")]).any(),
            keys(&[k(":commit"), k("ctrl+s")]).any(),
        ),
        bind(
            C::Rollback,
            SQL_EDITOR,
            keys(&[k("cmd+alt+backspace")]).any(),
            keys(&[k(":rollback")]),
        ),
        bind(C::NextPlanTip, RESULTS, NONE, keys(&[k("]t")])),
        bind(C::EditRefusedConnection, SQL_EDITOR, NONE, keys(&[k("e")])).when(When::RefusedWrite),
        bind(C::AllowRefusedWrite, SQL_EDITOR, NONE, keys(&[k("w")])).when(When::RefusedWrite),
        // AI assistant.
        bind(
            C::AskAi,
            SQL_EDITOR,
            keys(&[k("cmd+k")]),
            keys(&[k("ctrl+k")]),
        ),
        bind(
            C::AcceptAi,
            SQL_EDITOR,
            keys(&[k("tab")]),
            keys(&[k("tab")]),
        )
        .when(When::Suggestion),
        bind(
            C::AcceptAiAndRun,
            SQL_EDITOR,
            keys(&[k("cmd+enter")]),
            keys(&[k("ctrl+enter")]),
        )
        .when(When::Suggestion),
        bind(
            C::DiscardAi,
            SQL_EDITOR,
            keys(&[k("esc")]),
            keys(&[k("esc")]),
        )
        .when(When::Suggestion),
        // Prompts and dialogs.
        bind(
            C::ConfirmProductionWrite,
            PROMPT,
            control("dialog button"),
            keys(&[k("enter")]).shown("type the database name, then enter"),
        )
        .when(When::ConfirmWrite),
        bind(
            C::CancelProductionWrite,
            PROMPT,
            keys(&[k("esc")]),
            keys(&[k("esc")]),
        )
        .when(When::ConfirmWrite),
        bind(
            C::ScrollStatements,
            PROMPT,
            keys(&[k("pgup"), k("pgdn")]),
            keys(&[k("pgup"), k("pgdn")]).shown("pgup/pgdn"),
        )
        .when(When::ConfirmWrite),
        bind(C::LeaveWrite, PROMPT, NONE, keys(&[k("w")])).when(When::Leaving),
        bind(C::LeaveDiscard, PROMPT, NONE, keys(&[k("d")])).when(When::Leaving),
        bind(C::LeaveStay, PROMPT, keys(&[k("esc")]), keys(&[k("esc")])).when(When::Leaving),
        bind(C::ConflictOverwrite, PROMPT, NONE, keys(&[k("o")])).when(When::Conflict),
        bind(C::ConflictUseServer, PROMPT, NONE, keys(&[k("s")])).when(When::Conflict),
        bind(
            C::ConflictKeepMine,
            PROMPT,
            keys(&[k("esc")]),
            keys(&[k("k"), k("esc")]),
        )
        .when(When::Conflict),
        bind(C::ConflictDiscard, PROMPT, NONE, keys(&[k("d")])).when(When::Conflict),
        bind(C::FormPasteUrl, PROMPT, NONE, keys(&[k("u")])).when(When::ConnectionForm),
        bind(
            C::FormNextField,
            PROMPT,
            keys(&[k("tab")]),
            keys(&[k("tab")]),
        )
        .when(When::ConnectionForm),
        bind(
            C::FormTest,
            PROMPT,
            keys(&[k("cmd+t")]),
            keys(&[k("ctrl+t")]),
        )
        .when(When::ConnectionForm),
        bind(
            C::FormSave,
            PROMPT,
            keys(&[k("cmd+s")]),
            keys(&[k("ctrl+s")]),
        )
        .when(When::ConnectionForm),
        bind(
            C::FormSaveAndConnect,
            PROMPT,
            keys(&[k("cmd+enter")]),
            keys(&[k("ctrl+enter")]),
        )
        .when(When::ConnectionForm),
        bind(C::FormCancel, PROMPT, keys(&[k("esc")]), keys(&[k("esc")]))
            .when(When::ConnectionForm),
        bind(
            C::SettingsMove,
            PROMPT,
            NONE,
            keys(&[k("j"), k("k"), k("up"), k("down")]).shown("j/k"),
        )
        .when(When::SettingsScreen),
        bind(
            C::SettingsChange,
            PROMPT,
            NONE,
            keys(&[k("h"), k("l"), k("left"), k("right")]).shown("h/l"),
        )
        .when(When::SettingsScreen),
        bind(C::SettingsToggle, PROMPT, NONE, keys(&[k("space")])).when(When::SettingsScreen),
        bind(C::SettingsOpenFile, PROMPT, NONE, keys(&[k("ctrl+e")])).when(When::SettingsScreen),
        bind(C::SettingsReset, PROMPT, NONE, keys(&[k("R")])).when(When::SettingsScreen),
        bind(
            C::SettingsClose,
            PROMPT,
            keys(&[k("esc")]),
            keys(&[k("esc")]),
        )
        .when(When::SettingsScreen),
        bind(
            C::QuickOpenMove,
            PROMPT,
            keys(&[k("up"), k("down")]),
            keys(&[k("up"), k("down")]),
        )
        .when(When::QuickOpen),
        bind(
            C::QuickOpenPick,
            PROMPT,
            keys(&[k("enter")]),
            keys(&[k("enter")]),
        )
        .when(When::QuickOpen),
        bind(
            C::QuickOpenClose,
            PROMPT,
            keys(&[k("esc")]),
            keys(&[k("esc")]),
        )
        .when(When::QuickOpen),
    ]
};

/// The modifiers of a chord, as a spelling names them.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Mods {
    pub cmd: bool,
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
}

impl Mods {
    pub fn none(self) -> bool {
        self == Self::default()
    }
}

/// One press: a key with what is held, or a character as it is typed (a
/// vim letter is its character on every keyboard layout).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stroke {
    Key { mods: Mods, key: Key },
    Text(char),
}

/// A spelling, read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Spelled {
    /// One press, or a few after one another (`yy p`).
    Strokes(Vec<Stroke>),
    /// The digits `1` to `9` with these held (`alt+1..9`).
    Digits(Mods),
    /// A line of the `:` prompt, without its colon.
    Ex(&'static str),
}

/// Why a spelling could not be read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Unread(pub String);

const NAMED: &[(&str, Key)] = &[
    ("enter", Key::Enter),
    ("esc", Key::Escape),
    ("tab", Key::Tab),
    ("space", Key::Space),
    ("backspace", Key::Backspace),
    ("delete", Key::Delete),
    ("up", Key::ArrowUp),
    ("down", Key::ArrowDown),
    ("left", Key::ArrowLeft),
    ("right", Key::ArrowRight),
    ("home", Key::Home),
    ("end", Key::End),
    ("pgup", Key::PageUp),
    ("pgdn", Key::PageDown),
    ("f2", Key::F2),
    ("f6", Key::F6),
];

const MARKS: &[(char, Key)] = &[
    (',', Key::Comma),
    ('.', Key::Period),
    ('\'', Key::Quote),
    ('[', Key::OpenBracket),
    (']', Key::CloseBracket),
    ('/', Key::Slash),
    ('=', Key::Equals),
    ('-', Key::Minus),
    (';', Key::Semicolon),
    ('`', Key::Backtick),
    ('\\', Key::Backslash),
];

fn named(name: &str) -> Option<Key> {
    NAMED
        .iter()
        .find(|(known, _)| *known == name)
        .map(|(_, key)| *key)
}

/// The key a chord names by one character: a letter, a digit or a mark.
fn key_of(name: &str) -> Option<Key> {
    let mut chars = name.chars();
    let (Some(char), None) = (chars.next(), chars.next()) else {
        return None;
    };
    if char.is_ascii_lowercase() || char.is_ascii_digit() {
        return Key::from_name(&char.to_ascii_uppercase().to_string());
    }
    MARKS
        .iter()
        .find(|(mark, _)| *mark == char)
        .map(|(_, key)| *key)
}

/// The character a key types on a US keyboard, with Shift or without: a
/// letter, a digit or a mark. For a key that came with no text (a test's
/// bare press).
pub fn char_of(key: Key, shift: bool) -> Option<char> {
    if let Some(mark) = key_mark(key) {
        return (!shift).then_some(mark);
    }
    let mut chars = key.name().chars();
    let (Some(char), None) = (chars.next(), chars.next()) else {
        return None;
    };
    if char.is_ascii_uppercase() {
        return Some(if shift {
            char
        } else {
            char.to_ascii_lowercase()
        });
    }
    (char.is_ascii_digit() && !shift).then_some(char)
}

/// Reads a spelling. `super` is no modifier here: Hyprland owns it.
pub fn spell(keys: &'static str) -> Result<Spelled, Unread> {
    let unread = |why: &str| Unread(format!("{keys}: {why}"));
    if let Some(line) = keys.strip_prefix(':') {
        if line.is_empty() {
            return Err(unread("a command without a name"));
        }
        return Ok(Spelled::Ex(line));
    }
    if let Some(key) = named(keys) {
        let mods = Mods::default();
        return Ok(Spelled::Strokes(vec![Stroke::Key { mods, key }]));
    }
    let parts: Vec<&str> = keys.split('+').collect();
    let modifier = |part: &str| matches!(part, "cmd" | "ctrl" | "alt" | "shift");
    if parts.len() > 1 && modifier(parts[0]) {
        let (last, held) = parts.split_last().expect("more than one part");
        let mut mods = Mods::default();
        for part in held {
            let flag = match *part {
                "cmd" => &mut mods.cmd,
                "ctrl" => &mut mods.ctrl,
                "alt" => &mut mods.alt,
                "shift" => &mut mods.shift,
                other => return Err(unread(&format!("{other} is no modifier"))),
            };
            if std::mem::replace(flag, true) {
                return Err(unread("a modifier twice"));
            }
        }
        if *last == "1..9" {
            return Ok(Spelled::Digits(mods));
        }
        let key = named(last)
            .or_else(|| key_of(last))
            .ok_or_else(|| unread("no such key"))?;
        return Ok(Spelled::Strokes(vec![Stroke::Key { mods, key }]));
    }
    // A word before a `+` that is no modifier (`super+t`) is a mistake,
    // not five letters: only `"+p` has a `+` among typed characters.
    if parts.len() > 1 && parts.iter().any(|part| part.chars().count() > 1) {
        return Err(unread("no such modifier"));
    }
    // Typed characters, one after another. A space only sets them apart.
    let strokes: Vec<Stroke> = keys
        .chars()
        .filter(|char| *char != ' ')
        .map(Stroke::Text)
        .collect();
    if strokes.is_empty() || keys.chars().any(char::is_control) {
        return Err(unread("nothing to press"));
    }
    Ok(Spelled::Strokes(strokes))
}

fn key_word(key: Key) -> &'static str {
    NAMED
        .iter()
        .find(|(_, known)| *known == key)
        .map_or_else(|| key.name(), |(name, _)| name)
}

fn key_mark(key: Key) -> Option<char> {
    MARKS
        .iter()
        .find(|(_, known)| *known == key)
        .map(|(mark, _)| *mark)
}

/// A key as `layout` writes it.
fn written_key(layout: Layout, key: Key) -> String {
    if let Some(mark) = key_mark(key) {
        return mark.to_string();
    }
    match layout {
        Layout::Omarchy => key_word(key).to_lowercase(),
        Layout::Mac => match key {
            Key::Enter => "↩".to_owned(),
            Key::Backspace => "⌫".to_owned(),
            Key::Delete => "⌦".to_owned(),
            Key::Tab => "⇥".to_owned(),
            Key::Escape => "esc".to_owned(),
            Key::Space => "Space".to_owned(),
            Key::ArrowUp => "↑".to_owned(),
            Key::ArrowDown => "↓".to_owned(),
            Key::ArrowLeft => "←".to_owned(),
            Key::ArrowRight => "→".to_owned(),
            Key::PageUp => "Page Up".to_owned(),
            Key::PageDown => "Page Down".to_owned(),
            Key::Home => "Home".to_owned(),
            Key::End => "End".to_owned(),
            other => other.name().to_owned(),
        },
        Layout::Windows => match key {
            Key::Escape => "Esc".to_owned(),
            Key::ArrowUp => "Up".to_owned(),
            Key::ArrowDown => "Down".to_owned(),
            Key::ArrowLeft => "Left".to_owned(),
            Key::ArrowRight => "Right".to_owned(),
            Key::PageUp => "Page Up".to_owned(),
            Key::PageDown => "Page Down".to_owned(),
            other => other.name().to_owned(),
        },
    }
}

/// What is held, as `layout` writes it before the key.
fn written_mods(layout: Layout, mods: Mods) -> String {
    let mut written = String::new();
    match layout {
        // Apple's order: Control, Option, Shift, Command.
        Layout::Mac => {
            for (held, glyph) in [
                (mods.ctrl, "⌃"),
                (mods.alt, "⌥"),
                (mods.shift, "⇧"),
                (mods.cmd, "⌘"),
            ] {
                if held {
                    written.push_str(glyph);
                }
            }
        }
        Layout::Windows => {
            for (held, word) in [
                (mods.cmd || mods.ctrl, "Ctrl+"),
                (mods.alt, "Alt+"),
                (mods.shift, "Shift+"),
            ] {
                if held {
                    written.push_str(word);
                }
            }
        }
        Layout::Omarchy => {
            for (held, word) in [
                (mods.cmd, "cmd+"),
                (mods.ctrl, "ctrl+"),
                (mods.alt, "alt+"),
                (mods.shift, "shift+"),
            ] {
                if held {
                    written.push_str(word);
                }
            }
        }
    }
    written
}

/// A spelling as `layout` writes it: the one way a key is written there.
pub fn written(layout: Layout, keys: &'static str) -> String {
    let Ok(spelled) = spell(keys) else {
        return keys.to_owned();
    };
    match spelled {
        Spelled::Ex(_) => keys.to_owned(),
        Spelled::Digits(mods) => {
            let range = match layout {
                Layout::Omarchy => "1..9",
                Layout::Mac | Layout::Windows => "1…9",
            };
            format!("{}{range}", written_mods(layout, mods))
        }
        Spelled::Strokes(strokes) => match strokes.as_slice() {
            [Stroke::Key { mods, key }] => {
                format!(
                    "{}{}",
                    written_mods(layout, *mods),
                    written_key(layout, *key)
                )
            }
            // Typed characters are written as they are spelled, the space
            // that sets them apart too.
            _ => keys.to_owned(),
        },
    }
}

/// A key as a layout writes it. Only the keymap makes one, so what paints
/// a key beside a word (a hint, a button's shortcut) can take nothing
/// written by hand.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Written(String);

impl Written {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::ops::Deref for Written {
    type Target = str;

    fn deref(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for Written {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl PartialEq<&str> for Written {
    fn eq(&self, other: &&str) -> bool {
        self.0 == *other
    }
}

/// What typed characters, or a line of the `:` prompt, come to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Typed {
    /// They are this command's keys.
    Command(Command),
    /// They are the keys of something that is not built yet: taken, and
    /// nothing happens.
    Claimed,
    /// They begin a command's keys: the next character says which.
    Waiting,
    /// They are nobody's.
    Nothing,
}

/// The keymap in force: the defaults. What the user rebinds comes to stand
/// over them here.
#[derive(Clone, Debug)]
pub struct Keymap {
    bindings: Vec<Binding>,
}

impl Default for Keymap {
    fn default() -> Self {
        Self {
            bindings: BINDINGS.to_vec(),
        }
    }
}

impl Keymap {
    pub fn bindings(&self) -> &[Binding] {
        &self.bindings
    }

    /// The lines of `command`, in the keymap's order.
    pub fn of(&self, command: Command) -> impl Iterator<Item = &Binding> {
        self.bindings
            .iter()
            .filter(move |binding| binding.command == command)
    }

    /// The spellings that answer `command` in `scope`: its own keys, and
    /// none that is only claimed.
    pub fn chords(
        &self,
        layout: Layout,
        command: Command,
        scope: Scope,
    ) -> impl Iterator<Item = &'static str> + '_ {
        let built = command.info().built;
        self.of(command)
            .filter(move |binding| built && binding.scopes.contains(&scope))
            .flat_map(move |binding| binding.bound(layout).chords.iter())
            .filter(|chord| chord.kind == Kind::Own)
            .map(|chord| chord.keys)
    }

    /// The spellings that answer `command` in `scope` now: with the
    /// keyboard out of a text field (`normal`) or in one. A chord the
    /// keymap gives to normal mode is the field's own while one has it.
    pub fn chords_now(
        &self,
        layout: Layout,
        command: Command,
        scope: Scope,
        normal: bool,
    ) -> impl Iterator<Item = &'static str> + '_ {
        let built = command.info().built;
        self.of(command)
            .filter(move |binding| built && binding.scopes.contains(&scope))
            .flat_map(move |binding| {
                binding
                    .bound(layout)
                    .chords
                    .iter()
                    .map(move |chord| (binding, chord))
            })
            .filter(move |(binding, chord)| {
                chord.kind == Kind::Own
                    && spell(chord.keys)
                        .is_ok_and(|spelled| binding.mode(scope, layout, &spelled).holds(normal))
            })
            .map(|(_, chord)| chord.keys)
    }

    /// Every key of `command` as `layout` writes them, for a list of keys:
    /// each line's keys once, `, ` between them. Empty where it has none.
    pub fn label(&self, layout: Layout, command: Command) -> Written {
        let built = command.info().built;
        let mut all: Vec<String> = Vec::new();
        for binding in self.of(command) {
            let bound = binding.bound(layout);
            let mut line: Vec<String> = Vec::new();
            if bound.shown.is_empty() {
                for chord in bound.chords {
                    // A claimed key of a command that answers its others is
                    // written nowhere: it would be offered and do nothing.
                    if chord.kind != Kind::Claimed || !built {
                        line.push(written(layout, chord.keys));
                    }
                }
            } else {
                line.push(bound.shown.to_owned());
            }
            for keys in line {
                if !all.contains(&keys) {
                    all.push(keys);
                }
            }
        }
        Written(all.join(", "))
    }

    /// The lines that can answer in `scope`: its own and the global ones
    /// (a prompt has only its own), with what `holds` says is on screen.
    fn held<'a>(
        &'a self,
        scope: Scope,
        holds: &'a dyn Fn(When) -> bool,
    ) -> impl Iterator<Item = &'a Binding> {
        self.bindings.iter().filter(move |binding| {
            let global = scope != Scope::Prompt && binding.scopes.contains(&Scope::Global);
            (global || binding.scopes.contains(&scope))
                && (binding.when == When::Always || holds(binding.when))
        })
    }

    /// What `spelled` of a line that holds comes to, where `matches` says
    /// whether it is the keys asked about and `begins` whether they begin
    /// it. A line with a `when` takes its key before one without.
    fn read(
        &self,
        layout: Layout,
        scope: Scope,
        holds: &dyn Fn(When) -> bool,
        matches: &dyn Fn(&Spelled) -> bool,
        begins: &dyn Fn(&Spelled) -> bool,
    ) -> Typed {
        let mut found = Typed::Nothing;
        let mut qualified = false;
        for binding in self.held(scope, holds) {
            for chord in binding.bound(layout).chords {
                let Ok(spelled) = spell(chord.keys) else {
                    continue;
                };
                if chord.kind == Kind::Via {
                    continue;
                }
                let own = binding.when != When::Always;
                let answered = matches!(found, Typed::Command(_) | Typed::Claimed);
                if matches(&spelled) {
                    if answered && qualified && !own {
                        continue;
                    }
                    let answers = binding.command.info().built && chord.kind == Kind::Own;
                    found = if answers {
                        Typed::Command(binding.command)
                    } else {
                        Typed::Claimed
                    };
                    qualified = own;
                } else if begins(&spelled) && !answered {
                    found = Typed::Waiting;
                }
            }
        }
        found
    }

    /// What the characters typed so far (`typed`: `y`, then `yy`, then
    /// `yyp`) come to in `scope`.
    pub fn typed(
        &self,
        layout: Layout,
        scope: Scope,
        holds: &dyn Fn(When) -> bool,
        typed: &str,
    ) -> Typed {
        let letters = |spelled: &Spelled| -> Option<String> {
            let Spelled::Strokes(strokes) = spelled else {
                return None;
            };
            strokes
                .iter()
                .map(|stroke| match stroke {
                    Stroke::Text(char) => Some(*char),
                    Stroke::Key { .. } => None,
                })
                .collect()
        };
        self.read(
            layout,
            scope,
            holds,
            &|spelled| letters(spelled).is_some_and(|letters| letters == typed),
            &|spelled| letters(spelled).is_some_and(|letters| letters.starts_with(typed)),
        )
    }

    /// What a line of the `:` prompt (`line`, without its colon) comes to
    /// in `scope`. A line of digits is the one spelled `:N`.
    pub fn ex(
        &self,
        layout: Layout,
        scope: Scope,
        holds: &dyn Fn(When) -> bool,
        line: &str,
    ) -> Typed {
        let number = !line.is_empty() && line.chars().all(|char| char.is_ascii_digit());
        self.read(
            layout,
            scope,
            holds,
            &|spelled| match spelled {
                Spelled::Ex(name) => *name == line || (number && *name == "N"),
                Spelled::Strokes(_) | Spelled::Digits(_) => false,
            },
            &|_| false,
        )
    }

    /// The first key of `command` as `layout` writes it, for a hint beside
    /// a control. Empty where it has none.
    pub fn key(&self, layout: Layout, command: Command) -> Written {
        let label = self.label(layout, command);
        let first = label.split(", ").next().unwrap_or_default();
        Written(first.to_owned())
    }

    /// The first keys of `commands` written as one hint: `[ ]` for the
    /// previous row and the next, `ctrl+n/p` where they are held alike.
    pub fn together(&self, layout: Layout, commands: &[Command]) -> Written {
        let keys: Vec<Written> = commands
            .iter()
            .map(|command| self.key(layout, *command))
            .filter(|key| !key.is_empty())
            .collect();
        let Some(first) = keys.first() else {
            return Written::default();
        };
        // What is held, up to its last `+`: alike for all, it is written
        // once.
        let held = first.rfind('+').map_or("", |plus| &first[..=plus]);
        let alike = !held.is_empty() && keys.iter().all(|key| key.starts_with(held));
        let written = if alike {
            let rest: Vec<&str> = keys.iter().map(|key| &key[held.len()..]).collect();
            format!("{held}{}", rest.join("/"))
        } else {
            let all: Vec<&str> = keys.iter().map(Written::as_str).collect();
            all.join(" ")
        };
        Written(written)
    }

    /// The key that goes to the `digit`th (from one) of what `command`
    /// has a range of digits for: `⌘2`, `ctrl+shift+2`.
    pub fn digit_label(&self, layout: Layout, command: Command, digit: usize) -> Written {
        let held = self
            .of(command)
            .flat_map(|binding| binding.bound(layout).chords.iter())
            .find_map(|chord| match spell(chord.keys) {
                Ok(Spelled::Digits(mods)) => Some(mods),
                _ => None,
            });
        match held {
            Some(mods) if (1..=9).contains(&digit) => {
                Written(format!("{}{digit}", written_mods(layout, mods)))
            }
            _ => Written::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The presses of a spelling, each digit of a range apart.
    fn presses(layout: Layout, spelled: &Spelled) -> Vec<Vec<Stroke>> {
        // Windows has one Ctrl where the Mac has Cmd and Control.
        let fold = |mods: Mods| match layout {
            Layout::Windows => Mods {
                cmd: false,
                ctrl: mods.cmd || mods.ctrl,
                ..mods
            },
            Layout::Mac | Layout::Omarchy => mods,
        };
        match spelled {
            Spelled::Ex(_) => Vec::new(),
            Spelled::Digits(mods) => (1..=9)
                .map(|digit| {
                    let key = Key::from_name(&digit.to_string()).expect("a digit");
                    vec![Stroke::Key {
                        mods: fold(*mods),
                        key,
                    }]
                })
                .collect(),
            Spelled::Strokes(strokes) => vec![
                strokes
                    .iter()
                    .map(|stroke| match stroke {
                        Stroke::Key { mods, key } => Stroke::Key {
                            mods: fold(*mods),
                            key: *key,
                        },
                        text => *text,
                    })
                    .collect(),
            ],
        }
    }

    /// Whether two spellings would be read as one another: the same line
    /// of the prompt, the same presses, or presses the other begins with
    /// (after `y` nothing can tell `y` from `yy p`).
    fn clash(layout: Layout, a: &Spelled, b: &Spelled) -> bool {
        if let (Spelled::Ex(a), Spelled::Ex(b)) = (a, b) {
            return a == b;
        }
        presses(layout, a).iter().any(|a| {
            presses(layout, b).iter().any(|b| {
                let shared = a.len().min(b.len());
                a[..shared] == b[..shared]
            })
        })
    }

    struct Entry {
        command: Command,
        scope: Scope,
        when: When,
        mode: Mode,
        keys: &'static str,
        spelled: Spelled,
    }

    /// Every key of the keymap that is bound in `layout`, claimed ones and
    /// those of commands not built yet too. Not the ones written beside a
    /// command that are another's.
    fn entries(keymap: &Keymap, layout: Layout) -> Vec<Entry> {
        let mut entries = Vec::new();
        for binding in keymap.bindings() {
            for chord in binding.bound(layout).chords {
                if chord.kind == Kind::Via {
                    continue;
                }
                let spelled = spell(chord.keys).expect("a spelling that reads");
                for scope in binding.scopes {
                    entries.push(Entry {
                        command: binding.command,
                        scope: *scope,
                        when: binding.when,
                        mode: binding.mode(*scope, layout, &spelled),
                        keys: chord.keys,
                        spelled: spelled.clone(),
                    });
                }
            }
        }
        entries
    }

    #[test]
    fn every_spelling_reads() {
        for binding in BINDINGS {
            for bound in [&binding.mac, &binding.omarchy] {
                for chord in bound.chords {
                    assert!(spell(chord.keys).is_ok(), "{:?}", spell(chord.keys));
                }
            }
        }
        for unread in ["super+t", "ctrl+ctrl+t", "ctrl+nosuchkey", ":", ""] {
            assert!(spell(unread).is_err(), "{unread}");
        }
    }

    #[test]
    fn a_key_clashes_with_itself_and_with_what_begins_with_it() {
        let same = |a, b| {
            let (a, b) = (spell(a).unwrap(), spell(b).unwrap());
            Layout::ALL.into_iter().all(|layout| clash(layout, &a, &b))
        };
        assert!(same("ctrl+s", "ctrl+s"));
        assert!(same("y", "yy p"));
        assert!(same("yy p", "y"));
        assert!(same("]", "]e"));
        assert!(same("alt+3", "alt+1..9"));
        assert!(same(":w", ":w"));
        let apart = |a, b| {
            let (a, b) = (spell(a).unwrap(), spell(b).unwrap());
            Layout::ALL.into_iter().all(|layout| !clash(layout, &a, &b))
        };
        assert!(apart("yy p", "yy P"));
        assert!(apart("d", "D"));
        assert!(apart("ctrl+s", "ctrl+shift+s"));
        assert!(apart(":w", ":wq"));
        assert!(apart("]e", "]c"));
        assert!(apart("alt+3", "ctrl+shift+1..9"));
        // Windows has one Ctrl for the Mac's Cmd and Control.
        let (cmd, control) = (spell("cmd+t").unwrap(), spell("ctrl+t").unwrap());
        assert!(clash(Layout::Windows, &cmd, &control));
        assert!(!clash(Layout::Mac, &cmd, &control));
    }

    #[test]
    fn no_two_commands_share_a_key_where_both_could_be_read() {
        let keymap = Keymap::default();
        for layout in Layout::ALL {
            let entries = entries(&keymap, layout);
            for (index, a) in entries.iter().enumerate() {
                for b in &entries[index + 1..] {
                    let together =
                        a.scope.alongside(b.scope) && a.when == b.when && a.mode.overlaps(b.mode);
                    assert!(
                        a.command == b.command
                            || !together
                            || !clash(layout, &a.spelled, &b.spelled),
                        "{layout:?}: {} ({:?}, {}) and {} ({:?}, {})",
                        a.keys,
                        a.command,
                        a.scope.id(),
                        b.keys,
                        b.command,
                        b.scope.id(),
                    );
                }
            }
        }
    }

    #[test]
    fn every_command_has_its_keys_or_none_on_purpose() {
        let keymap = Keymap::default();
        for command in Command::ALL {
            let lines: Vec<&Binding> = keymap.of(*command).collect();
            assert!(!lines.is_empty(), "{command:?} is in no line of the keymap");
            assert!(!command.info().name.is_empty(), "{command:?}");
            for line in lines {
                assert!(!line.scopes.is_empty(), "{command:?}");
                for bound in [&line.mac, &line.omarchy] {
                    // Keys, or what does it in their place, never both.
                    assert!(
                        bound.chords.is_empty() || bound.control.is_empty(),
                        "{command:?}"
                    );
                    assert!(
                        bound.shown.is_empty() || !bound.chords.is_empty(),
                        "{command:?}"
                    );
                }
            }
        }
    }

    #[test]
    fn the_keymap_has_every_row_of_the_settled_table_by_its_name() {
        let mut rows: Vec<&str> = Vec::new();
        for command in Command::ALL {
            let row = command.info().row;
            if !row.is_empty() && !rows.contains(&row) {
                rows.push(row);
            }
        }
        let mut settled = vec![
            "Connections",
            "Go to connection window",
            "Switch database",
            "Settings",
            "Find any object",
            "New SQL editor tab",
            "Toggle sidebar",
            "Move between panes",
            "Switch tab",
            "New connection",
            "Edit connection",
            "Duplicate connection",
            "Delete connection",
            "Connect / go to window",
            "Reload",
            "Open inspector / focus fields",
            "Back to grid from inspector",
            "Previous / next row in inspector",
            "Edit cell",
            "Commit cell and move",
            "Cancel cell edit",
            "Set NULL",
            "Set DEFAULT",
            "Undo / redo one cell",
            "Copy / paste cell values",
            "Paste as new rows",
            "Add row",
            "Duplicate row",
            "Delete row",
            "Open referenced row",
            "Show saved row in sorted position",
            "Review pending SQL",
            "Save changes",
            "Discard all pending",
            "Next / previous error",
            "Open value in $EDITOR",
            "Apply / cancel large editor",
            "Format SQL or JSON",
            "Boolean cycle",
            "Step number / date part",
            "Run statement",
            "Run all",
            "Explain",
            "Explain analyze",
            "Compare with previous run",
            "Cancel running query",
            "Focus results",
            "Completion",
            "Go to line",
            "History",
            "Read-only / read-write tab",
            "Commit",
            "Rollback",
            "Ask AI",
            "Accept / accept and run / discard AI",
            "Next plan tip",
            "Confirm production write",
            "Help, all keys",
        ];
        rows.sort_unstable();
        settled.sort_unstable();
        assert_eq!(rows, settled);
    }

    #[test]
    fn omarchy_binds_no_super_and_nothing_over_vims_insert_keys() {
        let keymap = Keymap::default();
        let ctrl = |key| Stroke::Key {
            mods: Mods {
                ctrl: true,
                ..Mods::default()
            },
            key,
        };
        let vims = [ctrl(Key::W), ctrl(Key::H), ctrl(Key::U), ctrl(Key::R)];
        for entry in entries(&keymap, Layout::Omarchy) {
            let presses = presses(Layout::Omarchy, &entry.spelled);
            for stroke in presses.iter().flatten() {
                // `cmd` is the Mac's. Omarchy has no key for it but Super.
                let cmd = matches!(stroke, Stroke::Key { mods, .. } if mods.cmd);
                assert!(!cmd, "{}: {:?}", entry.keys, entry.command);
                let insert = entry.mode.overlaps(Mode::Insert) && entry.scope != Scope::Prompt;
                assert!(
                    !(insert && vims.contains(stroke)),
                    "{} of {:?} would be taken from the text in insert mode",
                    entry.keys,
                    entry.command
                );
            }
        }
        // The Mac's layout names no Omarchy letters' modifier by mistake.
        for binding in BINDINGS {
            for chord in binding.mac.chords {
                assert!(!chord.keys.contains("super"), "{}", chord.keys);
            }
        }
    }

    #[test]
    fn a_key_written_beside_a_command_is_another_commands_own() {
        let keymap = Keymap::default();
        for layout in Layout::ALL {
            let own = entries(&keymap, layout);
            for binding in keymap.bindings() {
                for chord in binding.bound(layout).chords {
                    if chord.kind != Kind::Via {
                        continue;
                    }
                    let spelled = spell(chord.keys).expect("a spelling that reads");
                    let found = own.iter().any(|entry| {
                        entry.command != binding.command
                            && binding
                                .scopes
                                .iter()
                                .all(|scope| scope.alongside(entry.scope))
                            && clash(layout, &entry.spelled, &spelled)
                    });
                    assert!(found, "{layout:?}: {} of {:?}", chord.keys, binding.command);
                }
            }
        }
    }

    #[test]
    fn each_layout_writes_a_key_its_own_way() {
        let cases = [
            ("cmd+shift+f", "⇧⌘F", "Ctrl+Shift+F"),
            ("cmd+alt+backspace", "⌥⌘⌫", "Ctrl+Alt+Backspace"),
            ("cmd+enter", "⌘↩", "Ctrl+Enter"),
            ("cmd+shift+enter", "⇧⌘↩", "Ctrl+Shift+Enter"),
            ("ctrl+space", "⌃Space", "Ctrl+Space"),
            ("shift+tab", "⇧⇥", "Shift+Tab"),
            ("cmd+alt+down", "⌥⌘↓", "Ctrl+Alt+Down"),
            ("cmd+1..9", "⌘1…9", "Ctrl+1…9"),
            ("cmd+,", "⌘,", "Ctrl+,"),
            ("cmd+'", "⌘'", "Ctrl+'"),
            ("cmd+.", "⌘.", "Ctrl+."),
            ("enter", "↩", "Enter"),
            ("backspace", "⌫", "Backspace"),
            ("tab", "⇥", "Tab"),
            ("esc", "esc", "Esc"),
            ("f2", "F2", "F2"),
            ("?", "?", "?"),
        ];
        for (keys, mac, windows) in cases {
            assert_eq!(written(Layout::Mac, keys), mac, "{keys}");
            assert_eq!(written(Layout::Windows, keys), windows, "{keys}");
        }
    }

    #[test]
    fn omarchy_writes_a_key_as_it_is_spelled() {
        for binding in BINDINGS {
            for chord in binding.omarchy.chords {
                assert_eq!(written(Layout::Omarchy, chord.keys), chord.keys);
            }
        }
        // And never the way another layout or an older label had it.
        let keymap = Keymap::default();
        for command in Command::ALL {
            let label = keymap.label(Layout::Omarchy, *command);
            for wrong in ["Enter", "C-", "Ctrl", "⌘", "↩"] {
                assert!(!label.contains(wrong), "{command:?}: {label}");
            }
        }
    }

    #[test]
    fn a_command_is_labelled_by_its_keys_in_the_layout() {
        let keymap = Keymap::default();
        let label = |layout, command| keymap.label(layout, command);
        assert_eq!(label(Layout::Mac, Command::Connections), "⌘O");
        assert_eq!(label(Layout::Windows, Command::Connections), "Ctrl+O");
        assert_eq!(label(Layout::Omarchy, Command::Connections), "ctrl+shift+c");
        assert_eq!(label(Layout::Omarchy, Command::SaveChanges), ":w, ctrl+s");
        assert_eq!(
            label(Layout::Omarchy, Command::OpenInspector),
            "enter, ctrl+l"
        );
        assert_eq!(label(Layout::Omarchy, Command::MoveRow), "j/k");
        assert_eq!(label(Layout::Mac, Command::EditCell), "↩, F2");
        // A key that is only claimed is not offered: the editor has no
        // normal mode for `:wq` yet.
        assert_eq!(
            label(Layout::Omarchy, Command::ApplyLargeEditor),
            "ctrl+enter"
        );
        // A command that is not built is listed with the keys it will have.
        assert_eq!(label(Layout::Mac, Command::Rollback), "⌥⌘⌫");
        assert_eq!(label(Layout::Omarchy, Command::DuplicateRow), "yy p");
        // None on purpose.
        assert_eq!(label(Layout::Mac, Command::SwitchTab), "");
        assert_eq!(keymap.key(Layout::Omarchy, Command::SaveChanges), ":w");
    }

    #[test]
    fn a_command_that_is_not_built_answers_no_key() {
        let keymap = Keymap::default();
        for command in Command::ALL {
            if command.info().built {
                continue;
            }
            for layout in Layout::ALL {
                for scope in Scope::ALL {
                    let mut chords = keymap.chords(layout, *command, scope);
                    assert_eq!(chords.next(), None, "{command:?}");
                }
            }
        }
        // And one that is built does not answer a key it only claims.
        let mut large = keymap.chords(
            Layout::Omarchy,
            Command::ApplyLargeEditor,
            Scope::CellEditor,
        );
        assert_eq!(large.next(), Some("ctrl+enter"));
        assert_eq!(large.next(), None);
    }

    #[test]
    fn the_settled_keys_are_the_ones_bound() {
        let keymap = Keymap::default();
        let mac = |command| keymap.label(Layout::Mac, command);
        let omarchy = |command| keymap.label(Layout::Omarchy, command);
        // Throwing changes away is one chord on the Mac, and the query's
        // cancel is another.
        assert_eq!(mac(Command::DiscardChanges), "⌥⌘⌫");
        assert_eq!(mac(Command::Rollback), "⌥⌘⌫");
        assert_eq!(mac(Command::CancelQuery), "⌘.");
        assert_eq!(mac(Command::PasteCells), "⌘V");
        assert_eq!(mac(Command::PasteNewRows), "⇧⌘V");
        assert_eq!(mac(Command::SetDefault), "⌘'");
        assert_eq!(mac(Command::NextError), "⌥⌘↓");
        assert_eq!(mac(Command::PreviousError), "⌥⌘↑");
        assert_eq!(mac(Command::Format), "⇧⌘F");
        assert_eq!(mac(Command::Explain), "⌘E");
        assert_eq!(mac(Command::ComparePreviousRun), "⌥⌘E");
        assert_eq!(mac(Command::GoToConnectionWindow), "⌘1…9");
        assert_eq!(omarchy(Command::GoToConnectionWindow), "ctrl+shift+1..9");
        assert_eq!(omarchy(Command::EditCell), "i, enter");
        assert_eq!(omarchy(Command::ReplaceCell), "cc, s");
        assert_eq!(omarchy(Command::PasteNewRows), "\"+p, ctrl+shift+v");
        assert_eq!(omarchy(Command::DuplicateConnection), "yy p");
        assert_eq!(omarchy(Command::DuplicateRow), "yy p");
        assert_eq!(omarchy(Command::DiscardChanges), ":e!");
        assert_eq!(omarchy(Command::ReviewSql), ":diff");
        assert_eq!(omarchy(Command::History), ":history");
        assert_eq!(omarchy(Command::ReadWriteTab), ":rw");
        assert_eq!(omarchy(Command::ReadOnlyTab), ":ro");
        assert_eq!(omarchy(Command::Format), "=, ctrl+shift+f");
        assert_eq!(omarchy(Command::BooleanCycle), "space");
        assert_eq!(omarchy(Command::StepUp), "ctrl+a");
        assert_eq!(omarchy(Command::StepDown), "ctrl+x");
        assert_eq!(omarchy(Command::NextError), "]e");
        assert_eq!(omarchy(Command::Reload), "R");
        assert_eq!(omarchy(Command::Explain), "ctrl+e");
        assert_eq!(omarchy(Command::CancelQuery), "ctrl+c");
    }
    #[test]
    fn typed_characters_are_read_one_after_another() {
        let keymap = Keymap::default();
        let nothing = |_: When| false;
        let omarchy = |scope, typed| keymap.typed(Layout::Omarchy, scope, &nothing, typed);
        use Typed::{Claimed, Nothing, Waiting};
        let command = Typed::Command;
        // A row is duplicated with three letters, and that is not built.
        assert_eq!(omarchy(Scope::Grid, "y"), Waiting);
        assert_eq!(omarchy(Scope::Grid, "yy"), Waiting);
        assert_eq!(omarchy(Scope::Grid, "yyp"), Claimed);
        assert_eq!(omarchy(Scope::Grid, "yyP"), Claimed);
        // A connection is, with the same three.
        assert_eq!(
            omarchy(Scope::Connections, "yyp"),
            command(Command::DuplicateConnection)
        );
        assert_eq!(
            omarchy(Scope::Connections, "dd"),
            command(Command::DeleteConnection)
        );
        assert_eq!(
            omarchy(Scope::Connections, "n"),
            command(Command::NewConnection)
        );
        assert_eq!(omarchy(Scope::Grid, "d"), Waiting);
        assert_eq!(omarchy(Scope::Grid, "dd"), command(Command::DeleteRow));
        assert_eq!(omarchy(Scope::Grid, "i"), command(Command::EditCell));
        assert_eq!(omarchy(Scope::Grid, "s"), command(Command::ReplaceCell));
        assert_eq!(omarchy(Scope::Grid, "cc"), command(Command::ReplaceCell));
        assert_eq!(omarchy(Scope::Grid, "x"), command(Command::SetNull));
        assert_eq!(
            omarchy(Scope::Grid, "gd"),
            command(Command::OpenReferencedRow)
        );
        assert_eq!(omarchy(Scope::Grid, "gs"), Claimed);
        assert_eq!(omarchy(Scope::Grid, "vy"), command(Command::CopyCells));
        assert_eq!(omarchy(Scope::Grid, "vp"), Claimed);
        assert_eq!(omarchy(Scope::Grid, "\"+p"), Claimed);
        assert_eq!(omarchy(Scope::Grid, "D"), Claimed);
        assert_eq!(omarchy(Scope::Grid, "R"), command(Command::Reload));
        assert_eq!(omarchy(Scope::Sidebar, "R"), command(Command::Reload));
        // The slash is the sidebar's filter there and the WHERE line here.
        assert_eq!(omarchy(Scope::Sidebar, "/"), command(Command::FindObject));
        assert_eq!(omarchy(Scope::Grid, "/"), command(Command::WhereFilter));
        // A global letter is read in every scope but a prompt's.
        assert_eq!(omarchy(Scope::Grid, "?"), command(Command::Help));
        assert_eq!(omarchy(Scope::Prompt, "?"), Nothing);
        // The brackets step rows from the inspector, and begin other keys
        // on the grid.
        assert_eq!(omarchy(Scope::Inspector, "]"), command(Command::NextRow));
        assert_eq!(omarchy(Scope::Grid, "]"), Waiting);
        assert_eq!(omarchy(Scope::Grid, "]e"), Claimed);
        assert_eq!(omarchy(Scope::Grid, "q"), Nothing);
        // The letters of the booleans are nobody's.
        assert_eq!(omarchy(Scope::Grid, "f"), Nothing);
        assert_eq!(omarchy(Scope::Grid, "t"), Nothing);
        assert_eq!(omarchy(Scope::Sidebar, "t"), command(Command::ToggleTree));
        // A key that needs something on screen is nobody's without it.
        assert_eq!(omarchy(Scope::Grid, "Y"), Nothing);
        let review = |when: When| when == When::Review;
        assert_eq!(
            keymap.typed(Layout::Omarchy, Scope::Grid, &review, "Y"),
            command(Command::CopyReviewSql)
        );
        let leaving = |when: When| when == When::Leaving;
        assert_eq!(
            keymap.typed(Layout::Omarchy, Scope::Prompt, &leaving, "w"),
            command(Command::LeaveWrite)
        );
        // The other layouts have no letters but the list's.
        for layout in [Layout::Mac, Layout::Windows] {
            assert_eq!(
                keymap.typed(layout, Scope::Grid, &nothing, "?"),
                command(Command::Help)
            );
            assert_eq!(keymap.typed(layout, Scope::Grid, &nothing, "x"), Nothing);
        }
    }

    #[test]
    fn a_line_of_the_prompt_is_read_in_its_scope() {
        let keymap = Keymap::default();
        let nothing = |_: When| false;
        let ex = |scope, line| keymap.ex(Layout::Omarchy, scope, &nothing, line);
        use Typed::{Claimed, Nothing};
        let command = Typed::Command;
        assert_eq!(ex(Scope::Grid, "w"), command(Command::SaveChanges));
        assert_eq!(ex(Scope::Grid, "e!"), command(Command::DiscardChanges));
        assert_eq!(ex(Scope::Grid, "diff"), command(Command::ReviewSql));
        assert_eq!(ex(Scope::SqlEditor, "rw"), command(Command::ReadWriteTab));
        assert_eq!(ex(Scope::SqlEditor, "ro"), command(Command::ReadOnlyTab));
        // Not built yet: claimed.
        for line in ["commit", "rollback", "history", "42"] {
            assert_eq!(ex(Scope::SqlEditor, line), Claimed, "{line}");
        }
        assert_eq!(ex(Scope::Grid, "keys default"), Claimed);
        // Each in its own scope only, and `:w` never anything but a save.
        assert_eq!(ex(Scope::Grid, "rw"), Nothing);
        assert_eq!(ex(Scope::SqlEditor, "w"), Nothing);
        assert_eq!(ex(Scope::Grid, "wq"), Nothing);
        assert_eq!(ex(Scope::Grid, "q!"), Nothing);
        assert_eq!(ex(Scope::Grid, "W"), Nothing);
        assert_eq!(ex(Scope::Grid, ""), Nothing);
        // The floating editor's own, once it has a normal mode.
        let large = |when: When| when == When::LargeEditor;
        assert_eq!(
            keymap.ex(Layout::Omarchy, Scope::CellEditor, &large, "wq"),
            Claimed
        );
        assert_eq!(
            keymap.ex(Layout::Omarchy, Scope::CellEditor, &large, "q!"),
            Claimed
        );
        // The Mac has no prompt.
        assert_eq!(keymap.ex(Layout::Mac, Scope::Grid, &nothing, "w"), Nothing);
    }
    #[test]
    fn every_command_has_a_name_of_its_own() {
        let mut names: Vec<&str> = Vec::new();
        for command in Command::ALL {
            let name = command.info().name;
            assert!(!names.contains(&name), "{name} names two commands");
            names.push(name);
        }
    }

    #[test]
    fn a_key_types_the_character_its_spelling_names() {
        for char in ('a'..='z').chain('0'..='9') {
            let key = key_of(&char.to_string()).expect("a key");
            assert_eq!(char_of(key, false), Some(char));
        }
        for (mark, key) in MARKS {
            assert_eq!(char_of(*key, false), Some(*mark));
        }
        assert_eq!(char_of(Key::R, true), Some('R'));
        // What Shift makes of a digit or a mark depends on the keyboard.
        assert_eq!(char_of(Key::Num1, true), None);
        assert_eq!(char_of(Key::Slash, true), None);
        assert_eq!(char_of(Key::Enter, false), None);
        assert_eq!(char_of(Key::F2, false), None);
    }

    #[test]
    fn a_chord_answers_in_the_mode_the_keymap_gives_it() {
        let keymap = Keymap::default();
        let now = |layout, command, scope, normal| {
            keymap
                .chords_now(layout, command, scope, normal)
                .collect::<Vec<_>>()
        };
        use Layout::{Mac, Omarchy};
        // Omarchy closes a tab in normal mode only: in a text field the
        // chord deletes a word. The Mac's closes it from anywhere.
        assert_eq!(
            now(Omarchy, Command::CloseTab, Scope::Global, true),
            ["ctrl+w"]
        );
        assert!(now(Omarchy, Command::CloseTab, Scope::Global, false).is_empty());
        assert_eq!(now(Mac, Command::CloseTab, Scope::Global, false), ["cmd+w"]);
        // The Mac sets NULL on a cell, not in its editor.
        assert_eq!(
            now(Mac, Command::SetNull, Scope::Grid, true),
            ["cmd+backspace"]
        );
        assert!(now(Mac, Command::SetNull, Scope::Grid, false).is_empty());
        // A save is read while a cell is typed in. Its line of the prompt
        // is not: a colon is text there.
        assert_eq!(
            now(Omarchy, Command::SaveChanges, Scope::Grid, false),
            ["ctrl+s"]
        );
        assert_eq!(
            now(Omarchy, Command::SaveChanges, Scope::Grid, true),
            [":w", "ctrl+s"]
        );
        // Enter opens the inspector from the rows, and is the field's own.
        assert_eq!(
            now(Omarchy, Command::OpenInspector, Scope::Grid, true),
            ["enter"]
        );
        assert!(now(Omarchy, Command::OpenInspector, Scope::Grid, false).is_empty());
        // A run is asked for from the text and from outside it.
        for normal in [true, false] {
            assert_eq!(
                now(Omarchy, Command::RunStatement, Scope::SqlEditor, normal),
                ["ctrl+enter"]
            );
        }
        // What the Mac and Windows read from wherever the keyboard is, as
        // they did before the keymap: a reload, a page, the inspector, a
        // cancel. Omarchy's cancel on a table is normal mode's, because
        // its chord drops the edit of an open editor.
        for command in [
            Command::Reload,
            Command::NextPage,
            Command::ToggleInspector,
            Command::OpenInspector,
            Command::AddRow,
            Command::CancelQuery,
        ] {
            assert_eq!(
                now(Mac, command, Scope::Grid, false).len(),
                1,
                "{command:?}"
            );
        }
        assert!(now(Omarchy, Command::CancelQuery, Scope::Grid, false).is_empty());
        assert_eq!(
            now(Omarchy, Command::CancelQuery, Scope::SqlEditor, false),
            ["ctrl+c"]
        );
        // The panes are normal mode's: in a field Ctrl+H is a backspace.
        assert!(now(Omarchy, Command::PaneLeft, Scope::Global, false).is_empty());
    }

    #[test]
    fn keys_are_written_together_and_for_one_digit() {
        let keymap = Keymap::default();
        let together = |layout, commands: &[Command]| keymap.together(layout, commands);
        assert_eq!(
            together(Layout::Omarchy, &[Command::PreviousRow, Command::NextRow]),
            "[ ]"
        );
        assert_eq!(
            together(
                Layout::Omarchy,
                &[Command::NextCompletion, Command::PreviousCompletion]
            ),
            "ctrl+n/p"
        );
        assert_eq!(
            together(Layout::Omarchy, &[Command::MoveRow, Command::MoveColumn]),
            "j/k h/l"
        );
        assert_eq!(together(Layout::Mac, &[Command::SwitchTab]), "");
        let digit =
            |layout, digit| keymap.digit_label(layout, Command::GoToConnectionWindow, digit);
        assert_eq!(digit(Layout::Mac, 2), "⌘2");
        assert_eq!(digit(Layout::Windows, 2), "Ctrl+2");
        assert_eq!(digit(Layout::Omarchy, 2), "ctrl+shift+2");
        assert_eq!(digit(Layout::Omarchy, 10), "");
        assert_eq!(keymap.digit_label(Layout::Mac, Command::Settings, 2), "");
    }
}
```

- [ ] **Step 3: Run its tests**

Run: `~/.cargo/bin/cargo test --locked --lib keymap::`
Expected: `test result: ok. 18 passed`.

- [ ] **Step 4: See the conflict test fail when it should**

In `BINDINGS`, change `keys(&[k("gs")])` to `keys(&[k("g")])` and run `~/.cargo/bin/cargo test --locked --lib keymap::tests::no_two`.
Expected: FAIL with `Omarchy: gd (OpenReferencedRow, grid) and g (ShowSavedRow, grid)`. Put `gs` back.

- [ ] **Step 5: Give the app its keymap**

In `src/app.rs`, in `pub struct App`, beside `pub look`:

```rust
    /// The keys in force: what the handler answers and what a label names.
    pub keymap: crate::keymap::Keymap,
```

In `App::new`, where the struct is built: `keymap: crate::keymap::Keymap::default(),`. And beside the other small readers of `App`:

```rust
    /// How the look in use binds and writes its keys.
    pub fn layout(&self) -> crate::keymap::Layout {
        crate::keymap::Layout::of(&self.look)
    }
```

- [ ] **Step 6: Run the four checks, then commit**

```bash
git add src/keymap.rs src/lib.rs src/app.rs
git commit -m "Write every key binding once, as data"
```

---

### Task 1a: The glyphs the Mac's keys are written with

**Files:**
- Modify: `assets/fonts/NotoSansSymbols2-Keys.ttf` (a binary: made again, not edited), `assets/fonts/NotoSansSymbols2-OFL.txt` if a second font's notice is needed
- Modify: `src/typography/fonts.rs` (the doc comment at line 31, the test at 263)

The Mac look's faces have ⌘ ⇧ ⌥ ⌫ ⌦ (the key font) and ↩ ↑ ↓ ← → … (Plex). They have no ⌃ and no ⇥ (D13). Until they do, `⌃Space` and `⇥` are boxes.

- [ ] **Step 1: Write the failing test**

In `src/typography/fonts.rs`' tests, beside `macos_draws_the_keys_its_shortcuts_name` (263), and in its way of building a harness:

```rust
    /// Every character the keymap writes for the Mac is one its faces
    /// have: a key added to the keymap cannot come out as a box.
    #[test]
    fn the_mac_has_a_glyph_for_every_key_the_keymap_writes() {
        let harness = mac_harness();
        let keymap = crate::keymap::Keymap::default();
        for role in [
            crate::typography::TextRole::Secondary,
            crate::typography::TextRole::Shortcut,
        ] {
            let font = role.font_id(Faces::Plex);
            for command in crate::keymap::Command::ALL {
                let label = keymap.label(crate::keymap::Layout::Mac, *command);
                assert!(
                    harness.ctx.fonts_mut(|fonts| fonts.has_glyphs(&font, &label)),
                    "{}: {command:?} is written {label}, with a box in it",
                    role.id()
                );
            }
        }
    }
```

(`mac_harness()` stands for the lines the test at 263 builds its harness with.)

Run: `~/.cargo/bin/cargo test --locked --lib typography::fonts::tests::the_mac_has`
Expected: FAIL at the first command written with ⇥ or ⌃ (`CommitNext is written ⇥`).

- [ ] **Step 2: Add ⌃**

Nothing here needs the network but `pip`. In the scratchpad, never in the repository:

```bash
python3 -m venv "$SCRATCH/fonts" && "$SCRATCH/fonts/bin/pip" install fonttools
"$SCRATCH/fonts/bin/pyftsubset" /usr/share/fonts/noto/NotoSansSymbols-Regular.ttf \
  --unicodes=U+2303 --output-file="$SCRATCH/ctrl.ttf"
"$SCRATCH/fonts/bin/pyftmerge" assets/fonts/NotoSansSymbols2-Keys.ttf "$SCRATCH/ctrl.ttf" \
  --output-file="$SCRATCH/keys.ttf"
```

Both are 1000 units to the em, which `pyftmerge` needs. Check the result holds the old six and the new one (`fc-query --format='%{charset}\n' "$SCRATCH/keys.ttf"` prints `21e7 2303 2318 2325-2326 232b 23ce`), then copy it over `assets/fonts/NotoSansSymbols2-Keys.ttf`. Noto Sans Symbols is under the same licence as Noto Sans Symbols 2 (SIL OFL 1.1): say in the doc comment at `fonts.rs:31` that the cut now holds ⌃ from Noto Sans Symbols, and add its copyright line to `assets/fonts/NotoSansSymbols2-OFL.txt` if that file names the font. `macos_key_symbols_stand_on_the_letters_baseline` (311) must still pass: add `⌃` to its list.

- [ ] **Step 3: ⇥, as D13 was answered**

If a font was chosen for ⇥: cut U+21E5 from it and merge it in the same way, with its licence file beside the others in `assets/fonts/`, and its reason in `Cargo.toml`'s or the fonts module's notes as the other faces have.

If the answer was "write Tab": in `src/keymap.rs`, `written_key` for `Layout::Mac` gives `"Tab"` for `Key::Tab`, the test `each_layout_writes_a_key_its_own_way` expects `("shift+tab", "⇧Tab", ...)` and `("tab", "Tab", ...)`, and the brief's list of glyphs is one short. Say so in the pull request.

**If D13 has no answer yet, stop here and ask.** Do not pick a font.

- [ ] **Step 4: Run the test of step 1, the four checks, commit**

```bash
git add assets/fonts src/typography/fonts.rs src/keymap.rs
git commit -m "Give the Mac's faces every glyph its keys are written with"
```

---

### Task 2: Read chords from the keymap; the window's keys and the sidebar's

**Files:**
- Modify: `src/ui/keys.rs` (`handle`, lines 158 to 620; a few letters of `letters`, 1268 to 1601)
- Modify: `src/app.rs` (`App::is_busy`, lifted from the reducer of `Action::CancelQuery` at 1127)
- Modify: `src/ui/workspace.rs` (`chip_card` near 667, the `note` at 1244, `connections_hint` at 1452 and its test at 2477)
- Modify: `src/ui/sidebar.rs` (`HINTS` at 374, `sql_button` at 429, "Refresh objects" at 612, "Refresh" at 265 and 734)
- Modify: `src/ui/object_tabs.rs:256`, `src/ui/data_view.rs` (the tooltip at 1037, `cancel_keys` at 1889), `src/ui/sql_results.rs:539`, `src/ui/structure.rs:191`
- Test: `src/ui/keys.rs` (unit), `src/ui/mod.rs` (headless)

**What changes for the user:** Omarchy's `ctrl+shift+c`, `ctrl+shift+1..9`, `alt+1..9`, `R`, `ctrl+c` to cancel, `/` and `t` in the sidebar; `ctrl+w` in normal mode only; no `ctrl+o`, `ctrl+n`, `ctrl+p`, `ctrl+r`, `ctrl+.`, no bare digits. On macOS and Windows: ⌘N is the picker's and the grid's only, and the name **Reload**; otherwise only the spelling of labels.

**Three things the windowing layer does to keys, which every reader below has to know.** They cannot be seen in a headless test, so each has a test that sends what the real keyboard sends.

1. The command key with C, X or V never arrives as a key: egui-winit sends `Event::Copy`, `Event::Cut` or `Event::Paste` and stops (`egui-winit/src/lib.rs:1021`, `is_copy_command` at 1424), whatever else is held. `ctrl+shift+c` is a copy with Shift held. `presses` reads those events for a chord on one of the three keys.
2. A digit with Shift held is reported as what it types: `!` for 1 on a US keyboard (`Key::Exclamationmark`), and the physical key says `Num1`. `digit_asked` looks at both.
3. A bracket with Shift held is a brace. `same_key` already knows.

- [ ] **Step 1: Write the failing unit tests for reading a chord**

In `src/ui/keys.rs`' tests:

```rust
    #[test]
    fn what_is_held_is_read_as_the_layout_names_it() {
        use crate::keymap::{Layout, Mods};
        let mods = |cmd, ctrl, alt, shift| Mods { cmd, ctrl, alt, shift };
        let cmd = Modifiers::MAC_CMD | Modifiers::COMMAND;
        // As Linux reports Ctrl: both.
        let linux_ctrl = Modifiers::CTRL | Modifiers::COMMAND;
        // Omarchy has one command key. Either name of it is Ctrl.
        for held in [Modifiers::CTRL, Modifiers::COMMAND, linux_ctrl] {
            assert!(held_is(Layout::Omarchy, held, mods(false, true, false, false)));
            assert!(!held_is(Layout::Omarchy, held, Mods::default()));
            assert!(!held_is(Layout::Omarchy, held, mods(false, true, false, true)));
        }
        assert!(held_is(
            Layout::Omarchy,
            linux_ctrl | Modifiers::SHIFT,
            mods(false, true, false, true)
        ));
        // Shift and Alt are exact everywhere: Ctrl+Shift+C is not Ctrl+C.
        assert!(!held_is(
            Layout::Omarchy,
            linux_ctrl | Modifiers::SHIFT,
            mods(false, true, false, false)
        ));
        // The Mac tells Cmd from Control.
        assert!(held_is(Layout::Mac, cmd, mods(true, false, false, false)));
        assert!(!held_is(Layout::Mac, cmd, mods(false, true, false, false)));
        assert!(held_is(Layout::Mac, Modifiers::CTRL, mods(false, true, false, false)));
        assert!(!held_is(Layout::Mac, Modifiers::CTRL, mods(true, false, false, false)));
        // Windows' Ctrl is the command key, and the Control of Ctrl+Space.
        assert!(held_is(Layout::Windows, linux_ctrl, mods(true, false, false, false)));
        assert!(held_is(Layout::Windows, linux_ctrl, mods(false, true, false, false)));
        assert!(held_is(Layout::Mac, Modifiers::NONE, Mods::default()));
    }

    #[test]
    fn a_chord_is_counted_fresh_and_repeated_and_taken() {
        use crate::keymap::Layout;
        let mut input = egui::InputState::default();
        let held = Modifiers::CTRL | Modifiers::COMMAND | Modifiers::SHIFT;
        let repeat = egui::Event::Key {
            key: Key::D,
            physical_key: None,
            pressed: true,
            repeat: true,
            modifiers: held,
        };
        let other = crate::testing::key(Key::D, Modifiers::CTRL | Modifiers::COMMAND);
        input.events = vec![crate::testing::key(Key::D, held), repeat, other.clone()];
        let pressed = presses(&mut input, Layout::Omarchy, "ctrl+shift+d");
        assert_eq!((pressed.fresh, pressed.count), (true, 2));
        // Ctrl+D was another chord, and stays.
        assert_eq!(input.events, vec![other]);
        // A bracket with Shift comes as a brace on a US keyboard.
        input.events = vec![crate::testing::key(Key::OpenCurlyBracket, held)];
        assert!(presses(&mut input, Layout::Omarchy, "ctrl+shift+[").fresh);
    }

    #[test]
    fn the_command_key_with_c_x_or_v_is_read_from_the_clipboard_event() {
        use crate::keymap::Layout;
        // What the window sends for Ctrl+Shift+C: no key, a copy.
        let mut input = egui::InputState::default();
        input.modifiers = Modifiers::CTRL | Modifiers::COMMAND | Modifiers::SHIFT;
        input.events = vec![egui::Event::Copy];
        assert!(!presses(&mut input, Layout::Omarchy, "ctrl+c").fresh, "Shift is held");
        assert_eq!(input.events, vec![egui::Event::Copy], "left for its owner");
        assert!(presses(&mut input, Layout::Omarchy, "ctrl+shift+c").fresh);
        assert!(input.events.is_empty(), "taken");
        // A paste, and a cut.
        input.modifiers = Modifiers::CTRL | Modifiers::COMMAND;
        input.events = vec![egui::Event::Paste("a\tb".into()), egui::Event::Cut];
        assert!(presses(&mut input, Layout::Omarchy, "ctrl+x").fresh);
        assert!(presses(&mut input, Layout::Omarchy, "ctrl+v").fresh);
        // A copy with nothing held (a Copy key, a menu's Copy) is no chord.
        input.modifiers = Modifiers::NONE;
        input.events = vec![egui::Event::Copy];
        assert!(!presses(&mut input, Layout::Omarchy, "ctrl+c").fresh);
        assert!(!presses(&mut input, Layout::Mac, "cmd+c").fresh);
    }

    #[test]
    fn a_digit_is_read_by_the_key_it_is_on_when_shift_changes_what_it_types() {
        use crate::keymap::{Command, Keymap, Layout, Scope};
        let keymap = Keymap::default();
        let held = Modifiers::CTRL | Modifiers::COMMAND | Modifiers::SHIFT;
        let mut input = egui::InputState::default();
        // Ctrl+Shift+1 on a US keyboard: the key is `!`, on the key of 1.
        input.events = vec![egui::Event::Key {
            key: Key::Exclamationmark,
            physical_key: Some(Key::Num1),
            pressed: true,
            repeat: false,
            modifiers: held,
        }];
        let digit = digit_asked(
            &mut input,
            &keymap,
            Layout::Omarchy,
            Command::GoToConnectionWindow,
            Scope::Global,
            true,
        );
        assert_eq!(digit, Some(0));
        assert!(input.events.is_empty());
    }
```

Run: `~/.cargo/bin/cargo test --locked --lib ui::keys::tests::what_is_held`
Expected: does not compile, `held_is`, `presses` and `digit_asked` are not found.

- [ ] **Step 2: Write `held_is`, `presses`, `asked` and `digit_asked`**

In `src/ui/keys.rs`, above `handle`:

```rust
use crate::keymap::{Command, Keymap, Layout, Mods, Scope, Spelled, Stroke};

/// Whether `held` is exactly `mods`, as `layout` reads what is held. Shift
/// and Alt are exact everywhere (`consume_key` lets an extra Shift
/// through, so `ctrl+shift+c` would be read as `ctrl+c`).
pub(crate) fn held_is(layout: Layout, held: Modifiers, mods: Mods) -> bool {
    if held.shift != mods.shift || held.alt != mods.alt {
        return false;
    }
    match layout {
        // One command key, Ctrl. Linux reports it as `ctrl` and `command`
        // both; either names it.
        Layout::Omarchy => !mods.cmd && (held.ctrl || held.command) == mods.ctrl,
        Layout::Mac | Layout::Windows => {
            let pattern = Modifiers {
                alt: mods.alt,
                ctrl: mods.ctrl,
                shift: mods.shift,
                mac_cmd: false,
                command: mods.cmd,
            };
            // As `is_press`: Cmd is held only where the chord names it.
            held.matches_exact(pattern) && (mods.cmd || !held.mac_cmd)
        }
    }
}

/// How a chord went down in a frame.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Presses {
    /// A press of its own: not only what a held key repeats.
    pub fresh: bool,
    /// Every time it went down, repeats too.
    pub count: usize,
}

/// With Shift held a US keyboard reports the brace for a bracket.
fn same_key(named: Key, pressed: Key) -> bool {
    named == pressed
        || matches!(
            (named, pressed),
            (Key::OpenBracket, Key::OpenCurlyBracket) | (Key::CloseBracket, Key::CloseCurlyBracket)
        )
}

/// Whether `event` is what the window sends in place of `key` held with
/// the command key: it turns C, X and V into a copy, a cut and a paste
/// before any key event is made, whatever else is held.
fn clipboard_event(event: &egui::Event, key: Key) -> bool {
    matches!(
        (event, key),
        (egui::Event::Copy, Key::C) | (egui::Event::Cut, Key::X) | (egui::Event::Paste(_), Key::V)
    )
}

/// Takes the presses of the chord spelled `keys` out of the frame and
/// counts them. A spelling that is no single chord (letters, a line of the
/// prompt, a range of digits) is never pressed here.
pub(crate) fn presses(input: &mut egui::InputState, layout: Layout, keys: &'static str) -> Presses {
    let Ok(Spelled::Strokes(strokes)) = crate::keymap::spell(keys) else {
        return Presses::default();
    };
    let [Stroke::Key { mods, key }] = strokes.as_slice() else {
        return Presses::default();
    };
    // A clipboard event says nothing of what was held: the frame does. One
    // with nothing held is a Copy key or a menu's, and no chord.
    let held_now = input.modifiers;
    let command = mods.cmd || mods.ctrl;
    let clipboard = command && held_is(layout, held_now, *mods);
    let mut pressed = Presses::default();
    input.events.retain(|event| match event {
        egui::Event::Key {
            key: down,
            modifiers: held,
            pressed: true,
            repeat,
            ..
        } if same_key(*key, *down) && held_is(layout, *held, *mods) => {
            pressed.fresh |= !repeat;
            pressed.count += 1;
            false
        }
        // A held chord's repeats come as more of the same event, and none
        // says it is a repeat: `held_copy` tells them apart where it
        // matters.
        event if clipboard && clipboard_event(event, *key) => {
            pressed.fresh = true;
            pressed.count += 1;
            false
        }
        _ => true,
    });
    pressed
}

/// How `command` was asked for in `scope` this frame, by any of the chords
/// it has there in this mode (`normal`: no text field has the keyboard).
pub(crate) fn asked(
    input: &mut egui::InputState,
    keymap: &Keymap,
    layout: Layout,
    command: Command,
    scope: Scope,
    normal: bool,
) -> Presses {
    let mut all = Presses::default();
    for keys in keymap.chords_now(layout, command, scope, normal) {
        let pressed = presses(input, layout, keys);
        all.fresh |= pressed.fresh;
        all.count += pressed.count;
    }
    all
}

/// The digit a command bound to a range (`alt+1..9`) was asked with, from
/// nought. With Shift held the key reports what the digit types (`!`), so
/// the key it is on counts too.
pub(crate) fn digit_asked(
    input: &mut egui::InputState,
    keymap: &Keymap,
    layout: Layout,
    command: Command,
    scope: Scope,
    normal: bool,
) -> Option<usize> {
    let mut found = None;
    for keys in keymap.chords_now(layout, command, scope, normal) {
        let Ok(Spelled::Digits(mods)) = crate::keymap::spell(keys) else {
            continue;
        };
        for (index, number) in NUMBERS.into_iter().enumerate() {
            let before = input.events.len();
            input.events.retain(|event| {
                !matches!(event, egui::Event::Key { key, physical_key, modifiers, pressed: true, .. }
                    if (*key == number || *physical_key == Some(number))
                        && held_is(layout, *modifiers, mods))
            });
            if input.events.len() != before {
                found = Some(index);
            }
        }
    }
    found
}
```

The mode is the keymap's (`Keymap::chords_now`, `Binding::mode`): the handler never decides by itself which chord is normal mode's. That is what makes the keymap's own tests (no clash where modes overlap, nothing over vim's insert keys) say something about the app.

Run the four tests of step 1. Expected: PASS.

- [ ] **Step 3: Write the failing headless tests for the window's keys**

In `src/ui/mod.rs`' tests, beside the tests they replace. Read the helpers they use first (`Harness::new`, `set_look`, `press`, `copy` at `testing.rs:174`, `type_key` at 5340, and how the tests around 2171 open a terminal workspace).

```rust
    /// Ctrl as Linux reports it.
    const CTRL: Modifiers = Modifiers::CTRL.plus(Modifiers::COMMAND);

    #[test]
    fn omarchy_opens_the_connections_with_ctrl_shift_c_and_no_longer_with_ctrl_o() {
        let (mut harness, _tab) = terminal_workspace();
        harness.press(Key::O, CTRL);
        assert!(!showing_connections(&harness), "ctrl+o is nobody's");
        // As the window sends it: a copy, with Shift held.
        harness.copy(true);
        assert!(showing_connections(&harness));
        assert_eq!(harness.copied, None, "and no row is copied");
    }

    #[test]
    fn omarchy_goes_to_a_tab_with_alt_and_its_digit_and_not_with_the_digit() {
        let (mut harness, tab) = terminal_workspace_with_two_tabs();
        let first = active_tab(&harness, tab);
        type_key(&mut harness, Key::Num2, "2");
        assert_eq!(active_tab(&harness, tab), first, "a bare digit is nobody's");
        harness.press(Key::Num2, Modifiers::ALT);
        assert_ne!(active_tab(&harness, tab), first);
    }

    #[test]
    fn omarchy_reloads_with_capital_r_and_ctrl_r_reloads_nothing() {
        let (mut harness, tab) = terminal_workspace();
        let before = refreshes(&harness, tab);
        harness.press(Key::R, CTRL);
        assert_eq!(refreshes(&harness, tab), before, "ctrl+r is vim's redo");
        type_key(&mut harness, Key::R, "R");
        assert_eq!(refreshes(&harness, tab), before + 1);
    }

    #[test]
    fn ctrl_w_in_insert_mode_deletes_a_word_and_closes_no_tab() {
        let (mut harness, tab, id) = normal_mode((0, 1));
        type_key(&mut harness, Key::I, "i");
        type_text(&mut harness, " more");
        let tabs = tab_count(&harness, tab);
        harness.press(Key::W, CTRL);
        assert_eq!(tab_count(&harness, tab), tabs, "the tab stays");
        assert!(!editor_text(&harness, tab, id).ends_with("more"));
        // The edit dropped (a tab with one pending asks before it closes),
        // the same chord closes the tab from normal mode.
        drop_the_edit(&mut harness);
        harness.press(Key::W, CTRL);
        assert_eq!(tab_count(&harness, tab), tabs - 1);
    }

    #[test]
    fn the_slash_filters_objects_from_the_sidebar() {
        let (mut harness, tab) = terminal_workspace();
        harness.app.apply(Action::TreeKey { tab, key: TreeKey::Down });
        type_key(&mut harness, Key::Slash, "/");
        assert!(sidebar_filter_focused(&harness), "the sidebar's filter");
        assert!(!harness.app.workspace(tab).unwrap().focus_where);
    }

    #[test]
    fn ctrl_p_opens_quick_open_on_the_mac_and_nothing_on_omarchy() {
        for (look, opens) in [(Look::macos(), true), (Look::omarchy(), false)] {
            let (mut harness, _tab) = workspace_in(look);
            harness.press(Key::P, Modifiers::COMMAND);
            let open = matches!(harness.app.dialog, Some(Dialog::QuickOpen(_)));
            assert_eq!(open, opens, "{}", look.name);
        }
    }

    #[test]
    fn omarchys_ctrl_c_cancels_a_run_and_is_the_copy_with_none_running() {
        let (mut harness, tab) = sql_harness(Look::omarchy());
        start_a_run(&mut harness, tab);
        // An earlier result shows under the run: the key is the tab's, not
        // only the text's.
        harness.copy(false);
        assert!(!running(&harness, tab), "cancelled");
        // With nothing running the copy is left for the field.
        harness.copy(false);
        assert!(!running(&harness, tab));
    }
```

`editor_text` exists (`mod.rs:12037`) and gives an `Option<String>`: compare with `as_deref()`. The helpers named here that do not exist yet (`terminal_workspace`, `showing_connections`, `refreshes`, `tab_count`, `sidebar_filter_focused`, `drop_the_edit`, `start_a_run`, `running`) are small readers and drivers of `harness.app`: write each beside the first test that uses it, from how the tests it sits near do the same (`command_r_refreshes_the_tree_from_the_tree` at 10825 counts refreshes; `ctrl_b_hides_and_shows_the_sidebar` at 2171 opens a terminal workspace; `ctrl_c_drops_the_edit` at 13357 sends the Ctrl+C that drops an edit; `command_period_cancels_a_sql_run` at 4287 starts a run and reads whether it runs).

Run: `~/.cargo/bin/cargo test --locked --lib ui::tests::omarchy`
Expected: FAIL, the old keys still answer.

- [ ] **Step 4: Work out what is in front and what has the keyboard, once, at the top of `handle`**

Two different questions, and the old code answers both by hand in a dozen places. After `in_editor` is known (near line 292), and before the input is read:

```rust
    let layout = app.layout();
    let on_picker = matches!(
        app.active_tab().content,
        crate::model::ConnTabContent::Picker(_)
    );
    // The tab in front, whatever has the keyboard in it: a save, a
    // cancel, a run and the `:` prompt are its own from the tree too.
    let front = if on_picker {
        Scope::Connections
    } else if sql.is_some() {
        Scope::SqlEditor
    } else {
        Scope::Grid
    };
    // What has the keyboard, for the letters: the nearest scope first.
    // `pane` says where the user last worked, not `tree_arrows`, which
    // also sends the arrows to the tree where there is no grid to move in.
    let on_tree = app
        .workspace(active)
        .is_some_and(|workspace| workspace.pane == crate::model::Pane::Tree);
    let scopes: &[Scope] = if on_picker {
        &[Scope::Connections]
    } else if open {
        &[Scope::CellEditor]
    } else if field.is_some() {
        &[Scope::Inspector]
    } else if on_tree {
        &[Scope::Sidebar]
    } else if sql.is_some() && sql_grid {
        &[Scope::Results, Scope::SqlEditor]
    } else if sql.is_some() {
        &[Scope::SqlEditor]
    } else {
        &[Scope::Grid]
    };
    // Normal mode: no text field has the keyboard.
    let normal = !editing;
```

`tree_arrows` stays as it is for the arrows. A SQL tab with the keyboard out of its text and no result yet is `[SqlEditor]`, which is where `=` and `:ro` are read in task 5.

- [ ] **Step 5: Replace the chords of `handle` one by one**

Inside `ctx.input_mut(|input| { ... })`, the closure `key(modifiers, key, action)` goes, and each line that named a key asks the keymap:

```rust
        let keymap = &app.keymap;
        let on = |input: &mut egui::InputState, command: Command, scope: Scope| {
            asked(input, keymap, layout, command, scope, normal)
        };
        if on(input, Command::Connections, Scope::Global).count > 0 {
            actions.push(Action::ShowConnections);
        }
        if on(input, Command::CloseConnection, Scope::Global).count > 0 {
            actions.push(Action::CloseConnTab(active));
        }
        if let Some(index) = digit_asked(
            input,
            keymap,
            layout,
            Command::GoToConnectionWindow,
            Scope::Global,
            normal,
        ) {
            actions.push(Action::ActivateConnection(index));
        }
```

The whole mapping, old line to command. Every guard a line has today stays on it (`in_workspace`, `!on_sql`, `!focused`, `sql_row`): the keymap says which key, not when the command makes sense. And what is read today from wherever the keyboard is stays so: on the Mac and Windows the keymap gives a reload, the pages, the inspector's chords, a new row, a new connection and the cancel to any mode (`.any()` in `BINDINGS`), because `an_editor_under_a_prompt_takes_the_keyboard_back_when_the_prompt_goes` (`mod.rs:15345`) and its like press them with an editor open. If a test that pressed a chord with a field focused stops passing, the fix is the line's mode in `src/keymap.rs`, with the keymap's tests run again, never a `true` passed by hand.

| Was (`keys.rs` line) | Command, scope | Notes |
|---|---|---|
| Esc while connecting (348) | `CancelConnecting`, Global | under `give_up`, a fresh press |
| `COMMAND N` adding a row (354) | `AddRow`, Grid | under `adds`, a fresh press. Omarchy's `o` is a letter: task 4 |
| `COMMAND+SHIFT W` (365) | `CloseConnection`, Global | |
| `COMMAND+SHIFT R` (373) | `ToggleInspector`, Grid or Results by `front` | the `!on_sql \|\| sql_row` guard stays |
| `CTRL+SHIFT Tab`, `CTRL Tab` (380, 385) | `PreviousConnection`, `NextConnection`, Global | |
| `COMMAND T` (388) | `NewSqlTab`, Global | still only `in_workspace` |
| `COMMAND O` (390) | `Connections`, Global | Omarchy's arrives as a copy with Shift held: `presses` reads it |
| `COMMAND N` (391) | `NewConnection`, Connections | only `if on_picker` |
| `COMMAND 1..9` (392) | `GoToConnectionWindow`, Global | `digit_asked` |
| `COMMAND R` (406) | `Reload`, Grid or Sidebar | the `!on_sql` guard stays (`refresh_and_filter_do_nothing_on_a_sql_tab`, 4305, must hold). `RefreshTree` where `tree_arrows`, else `Refresh`, as now. Omarchy's `R` is a letter: step 6 |
| `COMMAND .` (408) | `CancelQuery`, `front` | see below |
| `COMMAND F` (416) | `FilterBar`, Grid | the `!on_sql` guard stays |
| `COMMAND P` (418) | `FindObject`, Global | has no chord on Omarchy, so nothing is read there |
| `COMMAND B` (419) | `ToggleSidebar`, Global | |
| arrows, Home, End, Enter on the tree (422 to 446) | `MoveInTree`, `OpenObject`, Sidebar | under `tree_arrows && !focused`, as now. The arrows keep their `TreeKey`s: for each chord of the command, `presses(input, layout, keys)`, and the spelling (`"up"`, `"home"`) says which `TreeKey`. Omarchy's `j` and `k` stay in `letters` until task 4 |
| `COMMAND+ALT Left / Right` (449) | `PreviousPage`, `NextPage`, Grid | |
| `COMMAND+SHIFT [ ]` (464) | `PreviousTab`, `NextTab`, Global | `same_key` reads the braces |
| `COMMAND W` (476) | `CloseTab`, Global | normal mode's on Omarchy: `chords_now` gives it no chord while a field has the keyboard |
| arrows, PageUp and so on on the grid (477) | `MoveRow`, `MoveColumn`, `MovePage`, Grid or Results by `front` | as the tree's, under the guards at 477 |
| Space (502) | **stays in this task** | task 4 takes it out, with its hint and its tests |
| F6, Shift+F6 (525) | `NextPart`, `PreviousPart`, Global | |
| `CTRL H`, `CTRL L` (528) | `PaneLeft`, `PaneRight`, Global | the `app.look.terminal && !editing` test goes: the keymap has these for Omarchy only, in normal mode. The three arms of the `match` stay |
| `COMMAND ,` (579) | `Settings`, Global | |
| Alt and a digit | `SwitchTab`, Global | new: `digit_asked`, then `Action::ActivateTab` with the tab at that place, as `letters` did for bare digits at 1404 |

The run, format, mode and completion chords (316 to 345) are task 5's; `editing_keys` and `editing_letters` are task 4's; the picker's are task 3's. Leave them as they are in this task.

The cancel. `CancelQuery` is read for `front`, and on Omarchy its chord is `ctrl+c`, which is also the copy of whatever text field has the keyboard. So it is read only while there is something to cancel:

```rust
        // With nothing loading or running, Omarchy's chord is the text
        // field's copy, and the Mac's does nothing either way.
        if app.is_busy(active) && on(input, Command::CancelQuery, front).count > 0 {
            actions.push(Action::CancelQuery(active));
        }
```

`App::is_busy(tab)` is the check the reducer of `Action::CancelQuery` makes at `src/app.rs:1127` before it cancels (the active tab's pending requests): lift it into a method and use it in both places. With a cell's editor open, `chords_now` gives `CancelQuery` no chord in the Grid scope (it is normal mode's there), so `ctrl+c` still drops the edit (`held_copy`, `cell_editor.rs:176`).

The copy of a cell (564 to 577): on the Mac and Windows it is `asked(input, keymap, layout, Command::CopyCells, Scope::Grid, normal)` and `Command::CopyRow` for the one with Shift, both read from `Event::Copy` by `presses`. Read `CopyRow` first; exact Shift means neither can be taken for the other. Omarchy has no chord for either (`v y` is task 4's; until then `y` copies, as now), and its `Event::Copy` on the grid is no longer a copy of the cell.

- [ ] **Step 6: The four letters this task owns**

Omarchy's `R` (Reload), `/` in the sidebar (FindObject), `t` in the sidebar (ToggleTree) and `?` on every layout (Help) are single typed characters. Read just those through the keymap now, and leave everything else in `letters` as it is: the picker's letters move in task 3, the grid's in task 4, and until then their old code and its one-character wait (`pending_id`) are untouched.

At the very top of `letters`, **before** its first return (1276, a frame without a key press): a frame can bring a typed `?` and no key, and nine tests send exactly that (`src/ui/mod.rs` 1034, 2737, 4425, 9120, 9135, 9160, 10351; `src/ui/about.rs` 124, 172), as the block at 602 it replaces read it.

```rust
    // The letters the keymap has been given so far. A character that is
    // one of them is taken from the frame, its key with it; any other is
    // left for the code below.
    let holds = |_: crate::keymap::When| false;
    let mine = [
        Command::Reload,
        Command::FindObject,
        Command::ToggleTree,
        Command::Help,
    ];
    for char in typed_chars(ctx) {
        let read = scopes
            .iter()
            .map(|scope| app.keymap.typed(layout, *scope, &holds, &char.to_string()))
            .find(|read| *read != Typed::Nothing);
        let Some(Typed::Command(command)) = read else {
            continue;
        };
        if !mine.contains(&command) {
            continue;
        }
        take_char(ctx, char);
        match command {
            Command::Reload if on_tree => actions.push(Action::RefreshTree(tab)),
            Command::Reload => actions.push(Action::Refresh(tab)),
            Command::FindObject => {
                use crate::ui::focus::{self, Region};
                focus::step(ctx, true, Some(&[Region::Search]), None);
            }
            Command::ToggleTree => actions.push(Action::ToggleFlatTree(tab)),
            Command::Help => actions.push(Action::ShowHelp),
            _ => {}
        }
    }
```

`letters` takes `scopes`, `layout` and `on_tree` from `handle`. Being first, the block is also above the return for a look that is not the terminal's (1351), so `?` is read on every layout. The block at 602 goes.

```rust
/// The characters this frame typed with nothing held but Shift: the text
/// events where there are any, else the keys that went down, as the
/// characters they type on a US keyboard (a test's bare key press sends
/// no text; the real keyboard sends both).
fn typed_chars(ctx: &egui::Context) -> Vec<char> {
    ctx.input(|input| {
        let held = input.modifiers;
        if held.command || held.ctrl || held.mac_cmd || held.alt {
            return Vec::new();
        }
        let text: Vec<char> = input
            .events
            .iter()
            .filter_map(|event| match event {
                egui::Event::Text(text) => Some(text.chars()),
                _ => None,
            })
            .flatten()
            .collect();
        if !text.is_empty() {
            return text;
        }
        input
            .events
            .iter()
            .filter_map(|event| match event {
                egui::Event::Key {
                    key,
                    pressed: true,
                    modifiers,
                    ..
                } => crate::keymap::char_of(*key, modifiers.shift),
                _ => None,
            })
            .collect()
    })
}

/// Takes a typed character out of the frame, and the key that typed it:
/// what it asked for may open a field, which must not get it again.
fn take_char(ctx: &egui::Context, char: char) {
    ctx.input_mut(|input| {
        input.events.retain(|event| match event {
            egui::Event::Text(text) => !text.chars().eq([char]),
            egui::Event::Key {
                key,
                pressed: true,
                modifiers,
                ..
            } => crate::keymap::char_of(*key, modifiers.shift) != Some(char),
            _ => true,
        });
    });
}
```

Remove from the old code, in this task: the bare digits (1404 to 1410) and `t` (1411 to 1413). `/` on an object tab (1536) stays for the grid; with the tree in front the new block took the character first.

- [ ] **Step 7: The labels this task owns**

Each of these stops building a key from `look.command_key()` or a literal and asks the keymap. `app.keymap.key(app.layout(), Command::X)` is the first key and `label` is all of them; both give a `Written`, which is a `&str` wherever one is wanted for now (task 7 makes the painters take nothing else).

| Where | Was | Becomes |
|---|---|---|
| `workspace.rs:1453` `connections_hint` | `"{Connections} · {cmd}O"` | `key(layout, Command::Connections)`; it takes the keymap. Its test at 2477 expects `Connections · ⌘O`, `Connections · Ctrl+O`, `connections · ctrl+shift+c` |
| `workspace.rs:667` chip card | `"{cmd}{number}"` | `keymap.digit_label(layout, Command::GoToConnectionWindow, chip.number)` |
| `workspace.rs:1244` | `"ctrl+shift+w disconnect"` | `format!("{} {}", key(layout, Command::CloseConnection), "disconnect")` |
| `sidebar.rs:374` `HINTS` | three literal pairs | `[(ToggleSidebar, "hide"), (ToggleTree, "tree"), (OpenObject, "open")]`, keys from the keymap |
| `sidebar.rs:429` | `"{cmd}T"` | `key(layout, Command::NewSqlTab)` |
| `sidebar.rs:612`, `265`, `734` | "Refresh objects", "Refresh" | "Reload" (decision 18). Tests assert "Refresh objects" at `mod.rs` 152 to 156, 9236, 10461; `src/shots.rs:1359` finds the button by "Refresh", and is compiled only with its feature: change it here, task 8 builds it |
| `object_tabs.rs:256` | `.shortcut("ctrl+t")` | `key(layout, Command::NewSqlTab)` |
| `object_tabs.rs:376`, `413` | the tab's number, dim, before its name | stays: it is the N of `alt+N` now |
| `data_view.rs:1037` | "Cmd/Ctrl+F edits the filter" | `"{key} edits the filter"` with `FilterBar`'s key, put into the translated sentence by hand as `settings/terminal.rs:279` does with `{editor}` |
| `data_view.rs:1889` `cancel_keys`, `sql_results.rs:539`, `structure.rs:191` | `"{cmd}."` built twice | one `cancel_keys(app)` that returns `key(layout, Command::CancelQuery)` |

Do not lower-case a key with `look.label`: it would turn `R` into `r`. The keymap writes each layout's case itself.

- [ ] **Step 8: Run the tests of steps 1 and 3, then every test, and mend the ones that pinned a moved key**

Run: `~/.cargo/bin/cargo test --locked --workspace --all-targets`
Expected: the new tests pass. Tests that fail because a key moved are changed to the new key, each in place: `command_r_refreshes_the_tree_from_the_tree` (10825) types `R` on Omarchy; the tests that pick a tab with a digit press Alt with it; `the_connections_hint_names_the_key_as_the_look_spells_it`; the "Refresh objects" assertions; `the_terminal_status_line_is_the_editors_on_a_sql_tab` (6390) still pins `ctrl+. cancel`, which task 5 changes with the rest of that line: leave it. These two must hold unchanged, and say that the scopes are right: `refresh_and_filter_do_nothing_on_a_sql_tab` (4305) and `the_prompt_opens_wherever_the_line_offers_it` (14795). List in the commit message which tests changed and why.

- [ ] **Step 9: Run the four checks, then commit**

```bash
git add -A src
git commit -m "Read the window's keys and the sidebar's from the keymap"
```

---

### Task 3: The connection picker

**Files:**
- Modify: `src/ui/keys.rs` (`letters`, the picker branch, 1294 to 1350)
- Modify: `src/ui/picker.rs` (218 to 236, the footers at 325 to 348, 408)
- Test: `src/ui/mod.rs`

**What changes for the user:** macOS deletes a connection with ⌫ (not ⌘⌫). Omarchy duplicates with `yy p` (not `yy`), and no longer takes `ctrl+e`, `ctrl+d`, `ctrl+backspace`.

- [ ] **Step 1: Write the failing tests**

```rust
    #[test]
    fn omarchy_duplicates_a_connection_with_yy_p_and_not_with_yy() {
        let mut harness = Harness::new();
        harness.set_look(Look::omarchy());
        add_saved(&mut harness, "Bookshop");
        type_key(&mut harness, Key::Y, "y");
        type_key(&mut harness, Key::Y, "y");
        assert_eq!(saved_connections(&harness), 1, "yy only yanks");
        type_key(&mut harness, Key::P, "p");
        assert_eq!(saved_connections(&harness), 2);
        // A bare key press, as the older tests send a letter, is read too.
        for key in [Key::D, Key::D] {
            harness.press(key, Modifiers::NONE);
        }
        assert_eq!(saved_connections(&harness), 1);
    }

    #[test]
    fn the_mac_deletes_a_connection_with_backspace_alone() {
        let mut harness = Harness::new();
        harness.set_look(Look::macos());
        add_saved(&mut harness, "Bookshop");
        harness.press(Key::Backspace, Modifiers::COMMAND);
        assert_eq!(saved_connections(&harness), 1, "not with Cmd any more");
        harness.press(Key::Backspace, Modifiers::NONE);
        assert_eq!(saved_connections(&harness), 0);
    }

    #[test]
    fn a_new_connection_is_the_pickers_key() {
        // From a SQL editor the chord does nothing; on a table's rows it
        // adds a row, as `mod_n_adds_a_row_to_the_table_in_front...` says.
        let (mut harness, _tab) = sql_harness(Look::macos());
        harness.press(Key::N, Modifiers::COMMAND);
        assert!(harness.app.dialog.is_none());
    }
```

`saved_connections` stands for however the tests near `u_shows_the_url_field_in_the_terminal_look` (7632) count saved connections; `add_saved` is theirs. The third test passes after task 2 already: it is here because this is where the picker's keys are settled.

Run them. Expected: the first two FAIL (`yy` duplicates at once; Backspace alone does nothing).

- [ ] **Step 2: Read the picker's keys from the keymap**

In the picker branch of `letters`:

The chords (`Connect`, `ConnectAgain`, `EditConnection`, `DuplicateConnection`, `DeleteConnection`, `MoveInConnections`'s arrows) are read with `asked(..., Scope::Connections, normal)`. `Connect` keeps its guard (`!focus::on_control(ctx)`), and `ConnectAgain` is read before it.

The letters are read with the wait the keymap needs for `yy p`: a `String` of the characters so far, kept between frames under its own id (`typed_id`). The old one-character wait (`pending_id`) is still the grid's until task 4, and the picker stops using it here.

```rust
        let holds = |_: crate::keymap::When| false;
        let mut waiting: String = ctx.data(|data| data.get_temp(typed_id())).unwrap_or_default();
        for char in typed_chars(ctx) {
            waiting.push(char);
            let read = app.keymap.typed(layout, Scope::Connections, &holds, &waiting);
            // Whatever it came to, the picker has no text field under the
            // keys: the character is taken.
            take_char(ctx, char);
            match read {
                Typed::Waiting => continue,
                Typed::Command(command) => picker_asked(command, &selected, tab, actions),
                Typed::Claimed | Typed::Nothing => {}
            }
            waiting.clear();
        }
        ctx.data_mut(|data| data.insert_temp(typed_id(), waiting));
```

`picker_asked` pushes what each letter pushed: `NewConnection`, `EditConnection`, `DuplicateConnection`, `DeleteConnection` (the three that need one only where `selected` is some), `FilterConnections` (`Action::FocusPickerSearch`), `MoveInConnections` (`j` down, `k` up: the last character of `waiting` says which), `Help`. A character that is nobody's after a wait (`y`, then `x`) ends the wait and is not read again as a first key, as now.

Delete the `terminal && pressed(Key::Y)` and `Key::D` blocks with their waiting, and `command(Key::E)`, `command(Key::D)` and the `COMMAND Backspace` line: the keymap has those for the Mac and Windows only. `forget_pending` (1234) clears `typed_id` as well as `pending_id`.

- [ ] **Step 3: The picker's labels**

| Where | Was | Becomes |
|---|---|---|
| `picker.rs:222`, `227`, `236`, `408` | `"{cmd}N"`, `"n"` | `key(layout, Command::NewConnection)` |
| `picker.rs:325` terminal footer | seven literal pairs | `MoveInConnections` "move", `Connect` "connect / show", `EditConnection` "edit", `NewConnection` "new", `DuplicateConnection` "duplicate", `DeleteConnection` "delete", `FilterConnections` "filter" |
| `picker.rs:343` desktop footer | `"↩ connect"`, `"{cmd}E edit"`, `"{cmd}D duplicate"`, `"{cmd}⌫ delete"` | the same four from the keymap: `↩ connect`, `⌘E edit`, `⌘D duplicate`, `⌫ delete` |

- [ ] **Step 4: Run the tests, mend the ones that pressed the old keys, run the four checks, commit**

```bash
git add -A src
git commit -m "Read the connection picker's keys from the keymap"
```

**End of the first sitting. Stop and say where things stand.**

---

### Task 4: The grid, the inspector and the cell's editor

**Files:**
- Modify: `src/ui/keys.rs` (`editing_keys` 868 to 1063, `editing_letters` 1084 to 1217, the rest of `letters` 1354 to 1601, Space at 502)
- Modify: `src/ui/cell_editor.rs` (`ending_keys` 126 to 142, `leaving_keys` 149 to 197, `large_keys` 681 to 694, the band at 905 to 917)
- Modify: `src/app/editing.rs` (`run_command`, 984)
- Modify: `src/ui/workspace.rs` (the status line, 1943 to 1971, 2059 to 2070, 2124 to 2129)
- Modify: `src/ui/row_panel.rs` (504, 521 to 529, 1333 to 1341, 2200, 2408, 2468), `src/ui/data_view.rs` (188, 285, 1793), `src/ui/pending_bar.rs:286`, `src/ui/review.rs` (531, 544)
- Test: `src/ui/mod.rs`, `src/app.rs`

**What changes for the user:** Omarchy: `enter` opens the inspector, `s` replaces the cell, `v y` copies, `[` `]` are the inspector's, the `:` prompt opens on a SQL tab too; no `s` / `d` for the views, no `y`, no `ctrl+i`, no `ctrl+shift+d`. All looks: Space does nothing on the grid.

- [ ] **Step 1: Write the failing tests (the brief's tests 4, 5 and 6)**

```rust
    /// Test 4.
    #[test]
    fn enter_on_a_grid_row_opens_the_inspector_and_i_edits_the_cell() {
        let (mut harness, tab, id) = normal_mode((0, 1));
        harness.press(Key::Enter, Modifiers::NONE);
        assert!(editor(&harness, tab, id).is_none(), "enter edits nothing");
        assert!(harness.app.workspace(tab).unwrap().row_panel);
        assert!(panel_field_focused(&harness, tab, id), "the keyboard is on a field");
        // Back on the grid, `i` is insert mode on the cell.
        harness.press(Key::H, CTRL);
        type_key(&mut harness, Key::I, "i");
        assert!(editor(&harness, tab, id).is_some());
    }

    #[test]
    fn enter_and_i_in_the_inspector_edit_the_focused_field() {
        for (key, text) in [(Key::Enter, None), (Key::I, Some("i"))] {
            let (mut harness, tab, id) = normal_mode((0, 1));
            harness.press(Key::Enter, Modifiers::NONE);
            match text {
                Some(text) => type_key(&mut harness, key, text),
                None => harness.press(key, Modifiers::NONE),
            }
            assert!(editor(&harness, tab, id).is_some(), "{key:?}");
        }
    }

    #[test]
    fn enter_in_a_field_under_the_keys_is_the_fields_own() {
        // The WHERE line, the prompt, a focused button: none of them
        // loses its Enter to the inspector.
        let (mut harness, tab, _id) = normal_mode((0, 1));
        type_key(&mut harness, Key::Colon, ":");
        type_key(&mut harness, Key::W, "w");
        harness.press(Key::Enter, Modifiers::NONE);
        assert!(!harness.app.workspace(tab).unwrap().row_panel);
    }

    #[test]
    fn s_replaces_the_cell_as_cc_does_and_shows_no_structure() {
        let (mut harness, tab, id) = normal_mode((0, 1));
        type_key(&mut harness, Key::S, "s");
        assert_eq!(view(&harness, tab, id), ObjectView::Data);
        assert_eq!(editor_text(&harness, tab, id), "");
    }

    #[test]
    fn v_y_copies_the_cell_and_y_alone_copies_nothing() {
        let (mut harness, _tab, _id) = normal_mode((0, 1));
        type_key(&mut harness, Key::Y, "y");
        assert_eq!(harness.copied, None, "y begins yy p");
        harness.press(Key::Escape, Modifiers::NONE);
        type_key(&mut harness, Key::V, "v");
        type_key(&mut harness, Key::Y, "y");
        assert!(harness.copied.is_some());
    }

    /// Test 5, as far as it can go: the keys are claimed, and nothing
    /// answers them until pasting and duplicating are built.
    #[test]
    fn the_keys_that_paste_and_duplicate_are_claimed_and_change_nothing() {
        use crate::keymap::{Command, Layout};
        let keymap = crate::keymap::Keymap::default();
        assert_eq!(keymap.label(Layout::Mac, Command::PasteCells), "⌘V");
        assert_eq!(keymap.label(Layout::Mac, Command::PasteNewRows), "⇧⌘V");
        assert_eq!(keymap.label(Layout::Omarchy, Command::DuplicateRow), "yy p");
        for command in [Command::PasteCells, Command::PasteNewRows, Command::DuplicateRow] {
            assert!(!command.info().built, "{command:?} is built: test what it does");
        }
        // The Mac: a paste over a selected cell, as the window sends it,
        // without Shift and with.
        let (mut harness, tab, id) = editable_in(Look::macos());
        select(&mut harness, tab, id, (0, 1));
        let before = pending(&harness, tab, id);
        for held in [Modifiers::COMMAND, Modifiers::COMMAND | Modifiers::SHIFT] {
            harness.frame(vec![
                egui::Event::ModifiersChanged(held),
                egui::Event::Paste("a\tb".into()),
            ]);
            harness.frame(vec![egui::Event::ModifiersChanged(Modifiers::NONE)]);
        }
        assert_eq!(pending(&harness, tab, id), before);
        assert!(harness.app.dialog.is_none(), "no preview yet");
        // Omarchy: yy p adds no row.
        let (mut harness, tab, id) = normal_mode((0, 1));
        let rows = row_count(&harness, tab, id);
        for (key, text) in [(Key::Y, "y"), (Key::Y, "y"), (Key::P, "p")] {
            type_key(&mut harness, key, text);
        }
        assert_eq!(row_count(&harness, tab, id), rows);
        assert!(editor(&harness, tab, id).is_none());
    }

    /// Test 6.
    #[test]
    fn mod_backspace_in_an_open_editor_sets_no_null() {
        for look in desktop_looks() {
            let (mut harness, tab, id) = editable_in(look);
            select(&mut harness, tab, id, (0, 1));
            harness.press(Key::Enter, Modifiers::NONE);
            type_text(&mut harness, "abc");
            harness.press(Key::Backspace, Modifiers::COMMAND);
            // The editor is open still, and its cell is no NULL.
            assert!(editor(&harness, tab, id).is_some(), "{}", look.name);
            assert!(!is_null(&harness, tab, id, (0, 1)), "{}", look.name);
        }
    }

    #[test]
    fn space_on_the_grid_is_kept_for_the_booleans_and_toggles_no_panel() {
        for look in Look::ALL {
            let (mut harness, tab, id) = editable_in(look);
            select(&mut harness, tab, id, (0, 1));
            let shown = harness.app.workspace(tab).unwrap().row_panel;
            harness.press(Key::Space, Modifiers::NONE);
            assert_eq!(harness.app.workspace(tab).unwrap().row_panel, shown, "{}", look.name);
        }
    }
```

`editor`, `select`, `pending`, `is_null`, `view`, `row_count`, `panel_field_focused`, `editor_text` stand for the readers the tests around 12078 to 13500 and 21844 already have under their own names (`mod_backspace_sets_null_and_mod_z_reverts` at 12461, `cc_replaces_x_nulls_and_u_reverts` at 13486, `ctrl_l_focuses_the_rows_fields_in_the_terminal_look` at 21844): use theirs.

Run them. Expected: test 6's passes already (the editor returns before the chord is read, `keys.rs:939`, and the keymap gives the chord to normal mode): keep it as the regression test the brief asks for. `enter_in_a_field_under_the_keys...` passes too and must keep passing. The others FAIL.

- [ ] **Step 2: `editing_keys` reads its chords from the keymap**

Each `take_press` / `consume_press` in it becomes `asked(input, keymap, layout, command, scope, normal)` with `scope` being `Scope::Inspector` when `field.is_some()` and `Scope::Grid` otherwise. `normal` is false while the editor's field has the keyboard, and that alone keeps a chord of normal mode from being read there: do not add a second test of your own.

| Was | Command | Where it is read |
|---|---|---|
| `COMMAND S` (884) | `SaveChanges`, Grid | where it is: before the `open` return, any mode |
| `COMMAND+SHIFT D` (906) | `ReviewSql`, Grid | where it is. Omarchy has no chord for it |
| `COMMAND I` (930) | `OpenInspector`, Grid, **the Mac's and Windows' chord** | where it is ("whatever has the keyboard"): the keymap gives `cmd+i` to any mode. Its own `!open` and no-click guards stay, as now |
| Omarchy's `enter` | `OpenInspector`, Grid | **not** at 930. Below the `!(keyboard \|\| field.is_some()) \|\| open` return and the click return (939 to 961), with a fresh press, where Enter is read today (1017): there `keyboard` already says no field, button or tree has the keys. On a field it is `EditCell` (Inspector), so read by `scope` the two never meet |
| `COMMAND+ALT Backspace` (949) | `DiscardChanges`, Grid | |
| `COMMAND Backspace` (972) | `SetNull` | |
| `COMMAND Z` (976) | `UndoCell` | |
| Delete, Backspace (991) | `DeleteRow`, Grid | |
| Enter, F2 (1017) | `EditCell` | |
| Up, Down, Esc on a field (1025 to 1031) | `MoveField`, `BackToGrid`, Inspector | |

`editing_keys` is no longer for the desktop looks only. At its call (585), `keyboard` and `on_field` lose their `!terminal`: the keymap says which layout has which chord. Two things Omarchy must still not get from it stay under `!terminal` inside: typing that starts an edit (1038 to 1058), and `MoveField`'s `j` and `k`, which are letters and are read in step 4.

- [ ] **Step 3: The cell's editor reads its own keys from the keymap**

`src/ui/cell_editor.rs` reads Enter, Tab, Shift+Tab, Alt+Enter, Esc, Ctrl+C and Mod+Enter by hand, and decides by `look.terminal` what Esc does. Give it the keymap and the layout, and ask:

| Was (`cell_editor.rs`) | Command, scope `CellEditor`, `normal: false` |
|---|---|
| Alt+Enter (130) | `OpenLargeEditor` |
| Enter (132) | `CommitDown` |
| Shift+Tab, Tab (134, 136) | `CommitPrevious`, `CommitNext` |
| Esc, the other looks (153) | `CancelEdit` |
| Esc, the terminal (178) | `KeepEdit` |
| Ctrl+C, the terminal (176) | `CancelEdit`, whose chord there is `ctrl+c`: `presses` reads the copy event. `held_copy` in `keys.rs` (639) goes on taking the repeats |
| Mod+Enter in the large editor (688) | `ApplyLargeEditor` |
| Esc, Ctrl+C in the large editor | `CancelLargeEditor`, `KeepLargeEditor`: ask for these two first while the editor is large, then for the two above |

The branches on `look.terminal` in `leaving_keys` go: which command Esc is comes from which command has `esc` on the layout. The order of asking is the order in the table, which is the order the code reads them in today.

- [ ] **Step 4: Omarchy's letters of the grid go through the keymap**

`editing_letters` and the letter half of `letters` (1404 to 1600) are replaced by one reader, as the picker's was in task 3, with the same `String` wait (`typed_id`) and against `scopes`, the nearest first:

```rust
    let holds = |when: crate::keymap::When| on_screen(app, tab, when);
    for char in typed_chars(ctx) {
        waiting.push(char);
        let read = scopes
            .iter()
            .map(|scope| app.keymap.typed(layout, *scope, &holds, &waiting))
            .find(|read| *read != Typed::Nothing)
            .unwrap_or(Typed::Nothing);
        take_char(ctx, char);
        match read {
            Typed::Waiting => continue,
            Typed::Command(command) if alone && !opened => {
                opened |= grid_asked(app, ctx, command, &waiting, actions);
            }
            Typed::Command(_) | Typed::Claimed | Typed::Nothing => {}
        }
        waiting.clear();
    }
```

`on_screen(app, tab, when)` answers each `When` from the app's state: `InspectorOpen`, the row panel shows; `Review`, `object.edits.reviewing`; `Note`, the test at 1393 (`edits.note.is_some() && !edits.holds()`); `RefusedWrite`, `card_key(app, tab).is_some()`; the rest false (a dialog reads its keys itself; `LastRow` and `Suggestion` have nothing built behind them).

`grid_asked` is a `match` on the command that pushes what its letter pushed, and says whether it opened a field or the prompt (after which the frame's typing ends, `opened`):

| Command | Pushes |
|---|---|
| `EditCell` | `edit(EditStart::Value)` as at 1172 |
| `ReplaceCell` | `edit(EditStart::Replace(String::new()))` as at 1176, for `cc` and for `s` |
| `SetNull`, `UndoCell` | as at 1184 and 1188, with `on_cell` |
| `AddRow`, `AddRowAbove` | `Place::Below` / `Place::Above` as at 1202 |
| `DeleteRow` | `Action::DropRow`, where `on_rows` (1569) |
| `OpenReferencedRow` | `Action::FollowSelectedKey` |
| `CopyCells` | `ctx.copy_text(app.copy_text(false))` as at 1549 |
| `CopyReviewSql` | as at 1548, with the toast |
| `WhereFilter` | `Action::FocusWhere(tab)` |
| `MoveRow`, `MoveColumn`, `MoveField`, `MoveInTree` | the `MoveSelection`, `MoveField` and `TreeKey` of `j k h l`: the last character of `waiting` says which way |
| `PreviousRow`, `NextRow` | `MoveSelection` by a row, from the inspector |
| `FoldDocuments` | `Action::FoldDocuments`, where the panel shows (1516) |
| `Reload`, `FindObject`, `ToggleTree`, `Help` | task 2's four: its block goes, they are arms here |

Three chords of this scope are plain keys with a `When`, read with `asked` under the guards they have today (`!left_field`, `!menu_open`, a fresh press, 1382 to 1403 and 1509), and in this order, the first that answers ending it: `CloseReview`, `DismissNote`, `CloseInspector`.

What `editing_letters` guarded stays guarded, as its doc comment (1065 to 1083) says and for its reasons: a letter acts only in a frame that brings nothing else (`alone`, 1122 to 1143: no other key, no pointer button, nothing asked for already), a letter that opens a field ends the frame's typing, and a click or a field taking the keyboard ends a wait (`forget_pending`, which now clears only `typed_id`: `pending_id` and the one-character wait go). Move the comment to the new reader.

The reader sits where task 2's four letters did, before the first return of `letters` and before the one for a look that is not the terminal's: the keymap gives the other layouts no letter but `?`, so nothing else answers there.

The colon. `:` is no line of the keymap; it opens the prompt. It opens wherever a workspace's tab is in front, an object's or a SQL editor's, in normal mode with no dialog: the test at 1489 (`active.is_some()`) becomes "a workspace tab is in front". `the_prompt_opens_wherever_the_line_offers_it` (14795) holds in what it says, but it gets to the Structure view by typing `s` and back with `d`: have it set the view with `Action::SetView` instead. The last assertion of `escape_closes_the_prompt` (14743), "no prompt on a SQL editor", turns round: it opens there now (D4), and task 5 gives it lines to run.

Delete: `s` and `d` setting the view (1539, 1575 to 1598), `pressed(Key::Y)` (1549), `pressed(Key::I)` on a result row (1503), the `[` `]` loop outside the inspector (1458), and Space (502).

- [ ] **Step 5: `run_command` reads its line through the keymap**

So the prompt and the list of keys cannot disagree. In `src/app/editing.rs:984`:

```rust
        // The prompt is Omarchy's, in whatever look a test drives it.
        let layout = crate::keymap::Layout::Omarchy;
        let scope = match table {
            Some(_) => Scope::Grid,
            None => Scope::SqlEditor,
        };
        let line = text.trim();
        if line.is_empty() {
            return;
        }
        let action = match (self.keymap.ex(layout, scope, &|_| false, line), table) {
            (Typed::Command(Command::SaveChanges), Some((id, _))) => Action::WriteEdits { tab, id },
            (Typed::Command(Command::DiscardChanges), Some((id, _))) => {
                Action::DiscardEdits { tab, id }
            }
            (Typed::Command(Command::ReviewSql), Some((id, true))) => Action::ReviewEdits {
                tab,
                id,
                show: true,
            },
            // Nothing to show: the line says so.
            (Typed::Command(Command::ReviewSql), Some((_, false))) => {
                workspace.review_refused = true;
                return;
            }
            // A line of another scope, one that is only claimed, or none.
            _ => {
                workspace.command_error = Some(line.to_owned());
                return;
            }
        };
```

`workspace` is borrowed mutably above this in the function as it stands, and `self.keymap` cannot be read under that borrow: read the line's answer first, from `self.keymap` and a clone of the text, then take the workspace. `the_prompt_runs_w_and_e_bang_and_refuses_the_rest` (`app.rs:12182`) holds through this except for one thing: with a SQL tab in front, `w` and `e!` were silently nothing (the arm at 1015) and are now "not a command", because a SQL tab has lines of its own and these are not among them. Change that assertion, and say so in the commit.

- [ ] **Step 6: The labels of the grid, the inspector and the editor**

| Where | Was | Becomes |
|---|---|---|
| `workspace.rs:1943` to `1971` status line | `j/k`, `h/l`, `space inspect`, `i edit`, `o new row`, `/ filter`, `ctrl+b tables`, `y copy`, `s structure`, `:w write`, `:diff review` | `MoveRow` row, `MoveColumn` col, `OpenInspector` inspect (`enter inspect`), `EditCell` edit, `AddRow` new row, `WhereFilter` filter, `ToggleSidebar` tables, `CopyCells` copy (`v y copy`), `SaveChanges` write, `ReviewSql` review. **No `s structure`** |
| `workspace.rs:2059` insert mode | `esc normal`, `tab next cell` | `KeepEdit`, `CommitNext` |
| `workspace.rs:2124` struck through | `dd delete`, `:w write`, `o new row` | `DeleteRow`, `SaveChanges`, `AddRow` |
| `row_panel.rs:504`, `568` | `esc` | `key(layout, Command::CloseInspector)` |
| `row_panel.rs:521` | `[ ] prev/next`, `ctrl+l focus`, `i edit field` | `keymap.together(layout, &[PreviousRow, NextRow])`, `PaneRight` focus, `EditCell`'s `i` edit field |
| `row_panel.rs:1333` | `za fold · y copy` | `FoldDocuments`, `CopyCells`: `za fold · v y copy` |
| `row_panel.rs:2200` | `⇧⌘F` or `Ctrl+Shift+F` by comparing `command_key()` with `"⌘"` | `key(layout, Command::Format)` |
| `row_panel.rs:2408` | `yy p duplicate`, `dd delete` | `DuplicateRow`, `DeleteRow` |
| `row_panel.rs:2468` | `"{cmd}I"` | `key(layout, Command::OpenInspector)` |
| `data_view.rs:188` | `d data`, `s structure` in the terminal look | the words alone: the letters are gone |
| `data_view.rs:285` | `"{cmd}N"` | `key(layout, Command::AddRow)` |
| `data_view.rs:1793` | `"{cmd}Z reverts"` | `UndoCell` |
| `pending_bar.rs:286` | `"{cmd}S"` | `SaveChanges` |
| `review.rs:531`, `544` | `esc close`, `Y copy sql`, `:w write` | `CloseReview`, `CopyReviewSql`, `SaveChanges` |
| `cell_editor.rs:905` to `917` | `ctrl+enter apply · esc keep`; `{cmd}↩ apply · esc cancel` | terminal: `ApplyLargeEditor` apply, `KeepLargeEditor` keep; others: `ApplyLargeEditor` apply, `CancelLargeEditor` cancel. Windows now reads `Ctrl+Enter apply · Esc cancel` |

`key(layout, Command::EditCell)` is `i` on Omarchy: the keymap lists the grid's line first.

- [ ] **Step 7: Run every test and mend the ones that pinned a moved key**

The ones to expect, by what moved: `i_and_enter_edit_the_cell_and_escape_keeps_the_change` (12983) keeps `i` and loses `enter`; `the_status_line_offers_the_keys_that_edit_and_counts_what_is_pending` (14046), `the_keys_struck_through_are_the_ones_still_to_come` (14928), `the_counts_stay_in_a_window_too_narrow_for_the_keys` (14959) pin the status line; `space_and_ctrl_shift_r_toggle_the_row_panel` (6521) and `space_and_ctrl_shift_r_toggle_a_result_rows_panel` (5309) keep their chord and lose Space; `the_terminal_keys_of_the_row_panel_work_on_a_table_row` (5407) and `..._on_a_result_row` (5351) lose `[` `]` from the grid and `i` on a result; `the_terminal_panel_offers_a_result_row_no_copy_key` (5020); `a_waiting_first_key_ends_where_the_keys_are_not_the_grids` (13595), which now speaks of `typed_id`; every test that pressed `s`, `d`, `y` or a digit in the terminal look; `the_terminal_look_takes_none_of_the_other_looks_editing_keys` (12942), which now holds by the keymap. `the_shortcut_table_*` in `keys.rs` and `the_shortcuts_dialog_lists_each_looks_own_editing_keys` (972) still pass: the old table is task 7's.

- [ ] **Step 8: Run the four checks, then commit**

```bash
git add -A src
git commit -m "Read the grid's keys, the inspector's and the editor's from the keymap"
```

---

### Task 5: The SQL editor and its result

**Files:**
- Modify: `src/ui/keys.rs` (316 to 345, `completion_keys` 742 to 811, the SQL arms of task 4's reader)
- Modify: `src/app/editing.rs` (`run_command`), `src/ui/workspace.rs` (the status line of a SQL tab, 1983 to 1992)
- Modify: `src/ui/sql_editor.rs` (`run_keys` 234, `format_keys` 247, the toolbars), `src/ui/sql_complete.rs` (521, 537), `src/ui/sql_results.rs` (227, 1255, 1440, 1515)
- Test: `src/ui/mod.rs`, `src/ui/sql_editor.rs`, `src/ui/complete_tests.rs`

**What changes for the user:** Omarchy: `=` formats with the keyboard out of the text, `:ro` and `:rw` set the tab's mode; no `ctrl+shift+f`, `ctrl+shift+m`, `ctrl+i` in a SQL tab. Others: nothing but spelling.

- [ ] **Step 1: Write the failing tests (the brief's test 7, and D3, D4)**

```rust
    /// Test 7. Rollback is not built: its chord is claimed, and what the
    /// test can hold is that it reaches nothing else.
    #[test]
    fn rollbacks_chord_cancels_no_query_and_the_cancel_chord_throws_nothing_away() {
        use crate::keymap::{Command, Layout};
        let keymap = crate::keymap::Keymap::default();
        assert_eq!(keymap.label(Layout::Mac, Command::Rollback), "⌥⌘⌫");
        assert_eq!(keymap.label(Layout::Mac, Command::CancelQuery), "⌘.");
        assert!(!Command::Rollback.info().built, "Rollback is built: test what it does");
        // A run in flight in a SQL editor.
        let (mut harness, tab) = sql_harness(Look::macos());
        start_a_run(&mut harness, tab);
        harness.press(Key::Backspace, Modifiers::COMMAND | Modifiers::ALT);
        assert!(running(&harness, tab), "the discard chord cancels nothing");
        harness.press(Key::Period, Modifiers::COMMAND);
        assert!(!running(&harness, tab));
        // On a table with pending changes, the cancel chord keeps them and
        // the discard chord drops them.
        let (mut harness, tab, id) = editable_in(Look::macos());
        make_a_change(&mut harness, tab, id);
        harness.press(Key::Period, Modifiers::COMMAND);
        assert!(pending(&harness, tab, id) > 0);
        harness.press(Key::Backspace, Modifiers::COMMAND | Modifiers::ALT);
        assert_eq!(pending(&harness, tab, id), 0);
    }

    #[test]
    fn omarchy_formats_with_the_equals_sign_once_the_keyboard_is_out_of_the_text() {
        let (mut harness, tab) = sql_harness(Look::omarchy());
        type_text(&mut harness, "select 1");
        type_text(&mut harness, "=");
        assert!(sql_text(&harness, tab).ends_with('='), "typed, in the text");
        harness.press(Key::F, CTRL.plus(Modifiers::SHIFT));
        assert!(sql_text(&harness, tab).starts_with("select"), "no chord formats here");
        harness.press(Key::Escape, Modifiers::NONE);
        type_key(&mut harness, Key::Equals, "=");
        assert!(sql_text(&harness, tab).starts_with("SELECT"), "formatted");
    }

    #[test]
    fn the_prompt_sets_a_sql_tabs_mode_with_ro_and_rw() {
        let (mut harness, tab) = sql_harness(Look::omarchy());
        harness.press(Key::Escape, Modifiers::NONE);
        for (line, mode) in [("rw", RunMode::ReadWrite), ("ro", RunMode::ReadOnly)] {
            type_key(&mut harness, Key::Colon, ":");
            type_text(&mut harness, line);
            harness.press(Key::Enter, Modifiers::NONE);
            assert_eq!(sql_mode(&harness, tab), mode, ":{line}");
        }
        // Not Ctrl+Shift+M, and never Ctrl+W, which is the tab's close.
        harness.press(Key::M, CTRL.plus(Modifiers::SHIFT));
        assert_eq!(sql_mode(&harness, tab), RunMode::ReadOnly);
    }
```

`sql_text`, `sql_mode`, `make_a_change` stand for the readers of `command_shift_f_formats_the_script_and_the_editor_keeps_the_keys` (4100) and `mod_shift_m_switches_the_mode_of_the_editor_on_screen` (`sql_editor.rs:1405`). The fresh tab of `sql_harness` has no result, so after Esc the scope is `[SqlEditor]` (task 2, step 4): these two tests are also the test of that.

Run them. Expected: the first passes already, on the Mac half and on the keymap's lines (task 2 moved the cancel), and is kept as the regression test the brief asks for. The other two FAIL.

- [ ] **Step 2: The chords**

| Was | Command, scope |
|---|---|
| `COMMAND+SHIFT Enter`, `COMMAND Enter` (317) | `RunAll`, `RunStatement`, SqlEditor. Fresh presses only, as now. Exact Shift means the order no longer matters |
| `COMMAND+SHIFT F` (329) | `Format`, SqlEditor for `FormatSql`; `Format`, CellEditor for `FormatEditor` (while `open`). On Omarchy the SQL editor has no chord for it, and the large editor keeps `ctrl+shift+f`. The comment at 326 says the press is taken on every tab so `Mod+F` does not take it: with exact Shift that is no longer needed, and the comment goes |
| `COMMAND+SHIFT M` (342) | `ToggleSqlMode`, SqlEditor. No chord on Omarchy |
| `CTRL Space`, `COMMAND I` (752) | `Complete` |
| Down, Up, `CTRL N`, `CTRL P` (763) | `NextCompletion`, `PreviousCompletion`: the `terminal` column of that table goes, the keymap has `ctrl+n` and `ctrl+p` on Omarchy only. The arrows are still the list's only once it has a row (`list.has_row`) |
| Tab, Enter (797, 804) | `InsertCompletion`. Which key it was still matters (`enter_breaks`): read its two spellings apart with `presses` |
| Esc (808) | `CloseCompletion` |
| Enter on a result row (1500) | `OpenInspector`, Results, with `asked`: `enter` is a key, not a letter. Under the guards it has (`!tree && !panel && !focused`) |
| the card's letter (1526) | `EditRefusedConnection`, `AllowRefusedWrite`, arms of task 4's reader under `When::RefusedWrite`; `card_key` (`sql_results.rs:1440`) returns the command and the action instead of an `egui::Key`, and `Offer::names()` (1255) loses its letter and key. A fresh press only, as the comment at 1520 explains: `typed_chars` must skip a repeat for these |

- [ ] **Step 3: `=`, and the prompt's two lines**

Task 4's reader gets `Format` (push `Action::FormatSql` for the SQL tab in front; it gives the keyboard back to the text, `app.rs:1189`), and the result's `MoveRow` and `MoveColumn` under `Scope::Results`.

The prompt opens on a SQL tab since task 4. In `run_command`, two arms:

```rust
            (Typed::Command(Command::ReadWriteTab), None) => sql_mode(RunMode::ReadWrite),
            (Typed::Command(Command::ReadOnlyTab), None) => sql_mode(RunMode::ReadOnly),
```

where `sql_mode` builds `Action::SetSqlMode { tab, sql_tab, mode }` for the SQL tab in front (`None` there with no SQL tab: then the line is "not a command"). On a production connection `set_sql_mode` already refuses (`NoWrites::Unconfirmed`, `app.rs:3304`); the line then says nothing, as the switch does. A claimed line (`:commit`) is `Typed::Claimed` and falls to "not a command", as any unknown line does today.

- [ ] **Step 4: The labels**

| Where | Was | Becomes |
|---|---|---|
| `sql_editor.rs:234` `run_keys`, `247` `format_keys` | three spellings by comparing `command_key()` | `key(layout, RunStatement)`, `key(layout, RunAll)`, `key(layout, Format)`. The Windows toolbar keeps `Ctrl+Enter`, `Ctrl+Shift+Enter`, `Ctrl+Shift+F` |
| `workspace.rs:1983` SQL status line | `ctrl+enter`, `ctrl+shift+enter`, `ctrl+.`, `esc`, `ctrl+b`, `tab` | `RunStatement`, `RunAll`, `CancelQuery` (`ctrl+c cancel`), `LeaveEditor`, `ToggleSidebar`, `InsertCompletion` |
| `sql_complete.rs:521` | `tab complete · ctrl+n/p move` | `key(layout, InsertCompletion)`, and `keymap.together(layout, &[NextCompletion, PreviousCompletion])` |
| `sql_complete.rs:537` | `↩ Tab insert` | `label(layout, InsertCompletion)` and the word: `↩, ⇥ insert` on the Mac (task 1a gave the faces the glyph, or made it `Tab`) |
| `sql_results.rs:227` | "Run a statement with {keys}" | the same, `key(layout, RunStatement)` |
| `sql_results.rs:1515` | the card's letter | `key(layout, command)` |

- [ ] **Step 5: Run every test, mend the pinned ones, run the four checks, commit**

Expect `the_terminal_status_line_is_the_editors_on_a_sql_tab` (6390, pins `ctrl+. cancel`), `mod_shift_m_switches_the_mode_of_the_editor_on_screen` (looks other than Omarchy only now; its last lines read `SHORTCUTS`, which task 7 deletes: leave them), `command_shift_f_formats_...` (4100, the same), `mod_i_opens_the_list_by_hand` (`complete_tests.rs:1514`, the Mac and Windows only), the footer tests of `complete_tests.rs` (1231 to 1318), `the_cards_letters_are_the_terminal_looks_alone` (`sql_results.rs:2672`).

```bash
git add -A src
git commit -m "Read the SQL editor's keys from the keymap"
```

**End of the second sitting. Stop and say where things stand.**

---

### Task 6: Prompts and dialogs; the production confirmation asks for the database's name

**Files:**
- Modify: `src/ui/write_prompts.rs` (`WORD` at 55, `Facts` 444, `confirm_box` 831 to 1011, the leave box 336 to 417)
- Modify: `src/ui/conflict_prompt.rs` (`KEYS` at 87, 419, 429), `src/ui/connect_dialog/terminal.rs` (29, 79 to 106), `src/ui/connect_dialog/mod.rs` (its Mod+S, Mod+T, Mod+Enter), `src/ui/settings/mod.rs` (`asked` 123), `src/ui/settings/terminal.rs` (172, 649), `src/ui/quick_open.rs` (130), `src/ui/workspace.rs:238`
- Modify: `src/testing.rs` (the fixture's production connection)
- Test: `src/ui/mod.rs`, `src/ui/write_prompts.rs`

- [ ] **Step 1: Write the failing test (the brief's test 8)**

```rust
    /// Test 8.
    #[test]
    fn the_terminal_confirmation_takes_only_its_own_databases_name() {
        // Two production connections open: the confirmation is the one of
        // the tab that asked, whichever is in front.
        let (mut harness, tab, id) = production_in(Look::omarchy(), "bookshop_production");
        open_production(&mut harness, "bookshop_archive");
        make_a_change(&mut harness, tab, id);
        save(&mut harness, tab, id);
        assert!(harness.has("type bookshop_production to confirm"));
        assert!(!harness.has("type bookshop_archive to confirm"));
        for wrong in [
            "write",
            "bookshop_archive",
            "Bookshop_Production",
            "bookshop_production ",
            "bookshop",
        ] {
            set_typed(&mut harness, wrong);
            harness.press(Key::Enter, Modifiers::NONE);
            assert_eq!(writes(&harness), 0, "{wrong:?} confirmed");
            assert!(matches!(harness.app.dialog, Some(Dialog::ConfirmWrite(_))));
        }
        set_typed(&mut harness, "bookshop_production");
        harness.press(Key::Enter, Modifiers::NONE);
        assert_eq!(writes(&harness), 1);
    }
```

`production_in`, `open_production`, `set_typed`, `save`, `writes` stand for what `the_terminal_confirmations_button_cannot_be_pressed_before_the_word` (16835) and `the_prod_box_lists_its_statements_when_another_connection_is_in_front` (19564) use. The fixture's connection is the Bookshop one (`testing.rs:424`): give its production variant the database `bookshop_production`.

Run it. Expected: FAIL, the box asks for `write`.

- [ ] **Step 2: Ask for the name**

In `src/ui/write_prompts.rs`: delete `const WORD`. `Facts` (444) gets the name, read from the prompt's own workspace where `connection` is built (506 to 525), never from the tab in front:

```rust
    /// What is typed to confirm, in the terminal look: the database this
    /// save goes to, as its connection names it. A SQLite file's name; the
    /// connection's own name where it names no database.
    name: String,
```

```rust
        name: {
            let target = super::workspace::target(workspace);
            if target.is_empty() {
                workspace.name.clone()
            } else {
                target
            }
        },
```

(`target` is at `workspace.rs:696`. It is shown through `display_safe` in the connection line; the name compared is the raw one, and the name shown in "type ... to confirm" goes through `display_safe`. If the two differ, which a name with a control character would cause, the box cannot be confirmed by typing: say so in the comment, and leave it. A database named so is not one to write to by reflex.)

In `confirm_box`: `let armed = prompt.typed == facts.name;` and the sentence at 853 becomes `"type {name} to confirm"` with the name put in by hand, as `settings/terminal.rs` does with `{editor}`. Nothing else changes: `enter` confirms once armed, `esc` cancels. The sheet of the other looks is untouched ("dialog button").

Thirteen tests type `write` into the box (16818, 16835, 16891, 19146, 19386, 19453, 19564, 19599, 19702, 19776, 20119, 20252, 20383 in `src/ui/mod.rs`; five in `write_prompts.rs` from 1259; one in `conflict_prompt.rs` at 3291). They share a helper or two: change the helper to type the fixture's database name. `the_terminal_confirmations_button_cannot_be_pressed_before_the_word` types "wri" then "te": make it the name's two halves, and rename it (`..._before_the_name`).

- [ ] **Step 3: Run the four checks, then commit**

```bash
git add src/ui/write_prompts.rs src/ui/mod.rs src/ui/conflict_prompt.rs src/testing.rs
git commit -m "Confirm a production write by typing the database's name"
```

- [ ] **Step 4: The keys and labels of the prompts**

Each dialog reads and writes its keys from its own lines of the keymap (`Scope::Prompt` with its `When`). A dialog has no modes: pass `true` for `normal`, the keymap answers `Mode::Any` for the scope either way.

| Where | Lines |
|---|---|
| `write_prompts.rs:965` to `993` | `ConfirmProductionWrite` (`enter confirm`), `CancelProductionWrite`, `ScrollStatements` (its `shown`, `pgup/pgdn`). `take_enter` (539) stays as it is: it is about every Enter, whatever is held |
| `write_prompts.rs:336`, `407` to `417` | `LeaveWrite`, `LeaveDiscard`, `LeaveStay`; the brackets (`[w]`) stay the box's drawing, around the keymap's key |
| `conflict_prompt.rs:87` `KEYS` | `ConflictOverwrite`, `ConflictUseServer`, `ConflictKeepMine`, `ConflictDiscard`: the array keeps the answer and the word, and takes the letter from the keymap. `ScrollStatements`' `pgup/pgdn` at 419 |
| `connect_dialog/terminal.rs:29`, `79` | `FormPasteUrl`, `FormNextField`, `FormTest`, `FormSave`, `FormSaveAndConnect`, `FormCancel`. `lead: *key == "ctrl+s"` at 106 compares a label: make it `command == Command::FormSave`. The handler of these chords in `connect_dialog/mod.rs` reads them with `asked(..., Scope::Prompt, true)` |
| `settings/mod.rs:123` `asked` | `SettingsMove`, `SettingsChange`, `SettingsToggle`, `SettingsOpenFile`, `SettingsReset`, `SettingsClose`: for each key of the frame, which of these commands has it, by `presses` for the chords and `Keymap::typed` (with `When::SettingsScreen`) for the letters; the last character says which way for `j k h l`. The `free` guard on the arrows and Space stays. Rename the function, it now collides in meaning with `keys::asked` |
| `settings/terminal.rs:172`, `649` | `Settings` (`ctrl+, opens this`), and the six footer hints |
| `quick_open.rs:130` | `QuickOpenMove`, `QuickOpenPick`, `QuickOpenClose` |
| `workspace.rs:238` | `CancelConnecting` |

For a letter of a prompt (`w`, `d`, `o`, `s`, `k`): the box reads `Event::Text` today; have it ask `app.keymap.typed(layout, Scope::Prompt, &|when| when == When::Leaving, text)` and match on the command. Read with the layout in use: in the other looks these lines have no letters, and the boxes there are answered by their buttons, as now.

- [ ] **Step 5: Run every test, mend the pinned ones, the four checks, commit**

```bash
git add -A src
git commit -m "Read the keys of the prompts and dialogs from the keymap"
```

---

### Task 7: The list of keys is the keymap, and no key is written anywhere else

**Files:**
- Modify: `src/ui/help.rs`, `src/ui/keys.rs` (delete `Holds`, `SHORTCUTS`, `shortcuts`, `keys_label` and their tests, 20 to 156 and 1607 to 1826, 1918)
- Modify: `src/ui/widgets.rs` (`Hint` at 789, `key_hints*`, `ButtonSpec::shortcut` at 1315), `src/ui/terminal_dialog.rs` (`Key` at 63), `src/ui/states.rs` (`key_button` at 59), and their callers
- Modify: `src/theme.rs` (`command_key`, 480)
- Create: `tests/keys.rs`
- Modify: `src/keymap.rs` (`Written::bracketed`)
- Modify: `AGENTS.md` (lines 87 to 93), `src/ui/sql_editor.rs:1435` (a test that reads `SHORTCUTS`)
- Test: `src/ui/mod.rs`, `tests/keys.rs`

The brief's test 10 ("every label rendered for a command comes from the keymap") is held three ways, each catching what the others cannot: the painters of a key take only a `keymap::Written`, which only the keymap can make (a type, so it holds by construction, for bare words like `esc` and `dd` too); a test reads the sources for a key spelled by hand in a sentence or a tooltip, where no painter is involved; and the list of keys is tested against the keymap.

- [ ] **Step 1: Write the failing tests**

`tests/keys.rs`, after the pattern of `tests/engines.rs` (read it first: it walks the sources, leaves out test modules, and reports file and line):

```rust
//! No key is written outside the keymap. A label that names a key is made
//! by `src/keymap.rs` from a command, so a key that is rebound, or a
//! decision that moves one, shows everywhere at once. This reads the
//! sources and fails on a key spelled by hand.

use std::path::Path;

/// What a key written by hand looks like: a modifier and its plus, in any
/// of the ways the app has written one, or a glyph of the Mac's keys.
const SPELLED: &[&str] = &[
    "ctrl+", "Ctrl+", "cmd+", "Cmd+", "Cmd/", "alt+", "Alt+", "shift+", "Shift+", "Mod+", "⌘",
    "⇧", "⌥", "⌃", "↩", "⌫", "⌦", "⇥",
];

/// The files that may spell a key: the keymap, and the fonts' module,
/// whose tests name the glyphs the faces must have.
const MAY: &[&str] = &["src/keymap.rs", "src/typography/fonts.rs"];

#[test]
fn no_key_is_spelled_outside_the_keymap() {
    let mut found = Vec::new();
    for file in rust_files(Path::new("src")) {
        let name = file.to_string_lossy().replace('\\', "/");
        if MAY.contains(&name.as_str()) {
            continue;
        }
        let text = std::fs::read_to_string(&file).expect("a source file");
        for (number, line) in code_lines(&text) {
            for literal in string_literals(line) {
                if SPELLED.iter().any(|spelled| literal.contains(spelled)) {
                    found.push(format!("{name}:{number}: {literal}"));
                }
            }
        }
    }
    assert!(found.is_empty(), "keys written by hand:\n{}", found.join("\n"));
}
```

`rust_files`, `code_lines` (the lines outside `#[cfg(test)]` modules and outside comments, with their numbers) and `string_literals` are written here as `tests/engines.rs` writes its own; share nothing between the two files (each integration test is its own crate). `src/ui/complete_tests.rs` and `src/ui/env_tests.rs` are test modules in files of their own: skip a file whose `mod` line in `src/ui/mod.rs` is under `#[cfg(test)]`. A test may spell a key: it says what the user presses and sees.

And the list itself, in `src/ui/mod.rs`:

```rust
    #[test]
    fn the_list_of_keys_is_the_keymap() {
        for look in Look::ALL {
            let mut harness = Harness::new();
            harness.set_look(look);
            type_key(&mut harness, Key::Questionmark, "?");
            let layout = crate::keymap::Layout::of(&look);
            let keymap = harness.app.keymap.clone();
            for command in crate::keymap::Command::ALL {
                let info = command.info();
                let label = keymap.label(layout, *command);
                // Listed where the look has a key for it, and it is built.
                // Each command has a name of its own, so the name finds it.
                let listed = harness.has(info.name);
                assert_eq!(
                    listed,
                    info.built && !label.is_empty(),
                    "{}: {command:?}",
                    look.name
                );
                if listed {
                    assert!(harness.has(label.as_str()), "{}: {label}", look.name);
                }
            }
        }
    }
```

Run: `~/.cargo/bin/cargo test --locked --test keys`
Expected: FAIL, listing `src/ui/keys.rs`' `SHORTCUTS`, `src/theme.rs`' `command_key`, and whatever tasks 2 to 6 missed.

- [ ] **Step 2: The list reads the keymap**

`src/ui/help.rs`: in place of `shortcuts(&look)`, the built commands that have a key in the layout, in the keymap's order, under their group's name as a heading (`Group::Window` "Window and connections", `Navigation` "Navigation", `Editing` "Grid and editing", `Sql` "SQL editor", `Ai` "AI assistant", `Prompts` "Prompts and dialogs": the canvas' groups). The key column is `keymap.label(layout, command)`, the name column `gettext(locale, info.name)`. Two commands of one row of the settled table that are both built are two lines: the list is for finding a key, and each has its own name.

- [ ] **Step 3: The painters of a key take only what the keymap wrote**

Three places paint a key beside a word:

- `widgets::Hint` (`widgets.rs:789`) is `(&str, &str, bool)`: make the key a `&crate::keymap::Written`.
- `ButtonSpec::shortcut(&str)` (`widgets.rs:1315`), and `states::key_button(text, keys, look)` (`states.rs:59`) which calls it: take `&Written`.
- `terminal_dialog::Key { key: &str, .. }` (`terminal_dialog.rs:63`): `&Written`.

Since tasks 2 to 6 every caller holds a `Written` already and passed it as a `&str`. A caller that does not compile after this change is one that was missed: give it its command. Two hints are not a command's key and say so by their own small constructors in `src/keymap.rs`, with a doc comment each: the brackets a box draws round a letter (`[w]`: `Written::bracketed(&self)`), and the checkbox glyphs `[x]` / `[ ]` of the connection form and the Settings screen, which are not keys and are not painted through these three (they are plain text today: leave them).

- [ ] **Step 4: Delete the old table**

From `src/ui/keys.rs`: `Holds`, `ALL`, `DESKTOP`, `TERMINAL`, `SHORTCUTS`, `shortcuts`, `keys_label`, and the tests `the_shortcut_table_*`, `every_look_lists_what_holds_in_all_of_them`, `mod_is_named_for_the_platform`. What each of those tests held is held by `src/keymap.rs`' tests now (the keys of each command per layout: `the_settled_keys_are_the_ones_bound`; that each look lists its own: `the_list_of_keys_is_the_keymap`). **Say in the commit message, test by test, where what it checked is checked now**: `AGENTS.md` forbids deleting a test to get green, and these are replaced, not dropped.

`Look::command_key` (`theme.rs:480`): delete it; nothing calls it after tasks 2 to 6. `Look::label` stays (it lower-cases words, not keys). The last lines of `mod_shift_m_switches_the_mode_of_the_editor_on_screen` (`sql_editor.rs:1435`) read `SHORTCUTS`: they become a line on `keymap.label(Layout::Mac, Command::ToggleSqlMode)`.

`the_shortcuts_dialog_lists_each_looks_own_editing_keys` (`mod.rs:972`) is rewritten on the new labels (`↩, F2`, `⌘⌫`, `i`, `cc, s`).

- [ ] **Step 5: `AGENTS.md`**

Lines 87 to 93, the paragraph about the pages: `_reference/keyboard-shortcuts.md` follows `BINDINGS` in `src/keymap.rs`, written as the Mac and Windows write them; `_guide/macos.md` and `_guide/omarchy.md` each write their layout's column. And a line under Architecture: a key is spelled in `src/keymap.rs` and nowhere else; what paints one takes a `keymap::Written`, and `tests/keys.rs` finds one spelled by hand.

- [ ] **Step 6: Run the tests until `tests/keys.rs` passes, the four checks, commit**

```bash
git add -A src tests AGENTS.md
git commit -m "List the keys from the keymap, and find a key written anywhere else"
```

---

### Task 8: The scenes, and what the eye has to check

**Files:**
- Modify: `src/shots.rs` (whatever names a key or presses one)
- Modify: `README.md` lines 57 to 89 only where a key is named that no longer exists on any look (the pages are run 3's)

- [ ] **Step 1: Build the scenes**

Run: `~/.cargo/bin/cargo clippy --locked --features shots --lib --tests -- -D warnings`
Expected: errors where a scene calls `keys_label`, `command_key`, `run_keys` or a function whose signature took the keymap. Mend each.

- [ ] **Step 2: Render the scenes and read them**

Render the scenes off screen as the shots tests do (they need no display) into the scratchpad, never into `assets/screenshots/`, and look at: the picker's footer in both looks; the table's status line; the row panel's head and foot; the SQL toolbar on the Mac (`⌘↩`, `⇧⌘↩`, `⇧⌘F`); the completion footer on the Mac (the `⇥` glyph must not be a box); the large editor's band; the `?` list in each look. A glyph drawn as a box is a missing glyph: task 1a's test should have caught it.

These pictures are for this check. They are not committed, not compared by a test, and not sent to GitHub.

- [ ] **Step 3: Run the four checks and the shots' clippy, then commit**

```bash
git add -A src README.md
git commit -m "Name the settled keys in the scenes"
```

- [ ] **Step 4: Hand over what only a keyboard can show**

Nothing here can open the app's window. Give the user this list to press, in the Omarchy look and, if they have one to hand, on a Mac:

| Press | Should |
|---|---|
| `ctrl+shift+c` in a workspace | Open the connections, and copy nothing |
| `ctrl+shift+1` and `ctrl+shift+2` with two connections open | Go to each (Shift makes these `!` and `@`) |
| `alt+2` with two tabs | Go to the second tab, and type nothing |
| `ctrl+c` while a long query runs, in a SQL editor and on a table | Cancel it |
| `ctrl+c` on selected text in a SQL editor with nothing running | Copy it |
| `ctrl+w` while typing in a cell | Delete a word, close nothing |
| `yy p` in the picker | Duplicate the connection |
| `v y` on a cell | Copy it |
| `esc`, then `=` in a SQL editor | Format, and give the keyboard back to the text |
| `esc`, then `:rw` | Set the tab read-write |
| On the Mac: ⌃Space and ⌘I in a SQL editor | Open the completion list |
| On the Mac: the `?` list, the picker's footer, the SQL toolbar | Show no box in place of a glyph |

**End of run 1. Stop here.** Say what was built, what was only compiled (the Mac and Windows looks run on Linux in the tests; nothing was tried on a Mac), and offer run 2.

---

## What run 1 leaves for the pull request's description

- The eighteen decisions, from the spec.
- The tests, by the brief's numbers: 1 `keymap::tests::no_two_commands_share_a_key_where_both_could_be_read`; 2 `every_command_has_its_keys_or_none_on_purpose`, `the_keymap_has_every_row_of_the_settled_table_by_its_name`, and for its second half the `Written` type the painters take; 3 `omarchy_binds_no_super_and_nothing_over_vims_insert_keys`, `a_chord_answers_in_the_mode_the_keymap_gives_it` and `ctrl_w_in_insert_mode_deletes_a_word_and_closes_no_tab`; 4 `enter_on_a_grid_row_opens_the_inspector_and_i_edits_the_cell`; 5 `the_keys_that_paste_and_duplicate_are_claimed_and_change_nothing` and `v_y_copies_the_cell_and_y_alone_copies_nothing`; 6 `mod_backspace_in_an_open_editor_sets_no_null`; 7 `rollbacks_chord_cancels_no_query_and_the_cancel_chord_throws_nothing_away`; 8 `the_terminal_confirmation_takes_only_its_own_databases_name`; 9 run 2; 10 the `Written` type, `tests/keys.rs` and `the_list_of_keys_is_the_keymap`.
- The before/after table: "Keys that go" above, as it stands once the open rows are answered.
- Which tests were rewritten because their key moved, from the commit messages.
