# Search in the Keyboard shortcuts dialog Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use rubypowers:subagent-driven-development (recommended) or rubypowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A search field at the top of the `?` dialog that cuts the list to the commands whose name, group or keys match what is typed, or whose keys are the chord pressed in it (issue #300, item 3).

**Architecture:** The Settings → Keyboard pane's matcher moves out of `src/ui/settings/keyboard.rs` into a new `src/ui/key_search.rs`, and learns to match typed key text against the keys as the look writes them. Both the pane and the dialog read it. The dialog's search lives in the model (`Dialog::Help(HelpSearch)`), changed by two actions, so headless tests can drive it.

**Tech Stack:** Rust, egui (headless `Harness` in `src/testing.rs`), the keymap in `crates/tabletist-core/src/keymap.rs`.

**Design:** the canvas' Settings page, "Keyboard shortcuts dialog (?)": `MacHelpSearch`, `WindowsHelpSearch`, `OmarchyHelpSearch`. Approved "as drawn" on 2026-11-11, without the Scope-column option (`MacHelpSearchScope`). Its "your call" items are taken as drawn: typed keys match the Keys column (in Settings too), matched letters are marked, the dialog keeps its opened size while typing, the Windows field shows focus as an accent line under it, and Omarchy ends in a key-hint line whose esc says clear or close.

## Where the build departs from the canvas

| The canvas draws | What is built | Why |
|---|---|---|
| macOS field with 7 px corners | The look's search field (`widgets::search_field`, a capsule on macOS) | The same field as Settings → Keyboard and the sidebar; one search field per look. |
| Omarchy footer "type filter · ctrl+key find it · esc clear" | The words "type to filter, or press keys to find them" as a hint with no key, then "esc clear" / "esc close" | `widgets::Hint` keys must come from the keymap, and `tests/keys.rs` refuses a key spelled by hand ("ctrl+key"). |
| Rows on the boards | What `help::rows` lists | The boards' rows were filled in by hand. |

Nothing else departs. Name these in the message that offers the PR.

## What was decided

- One matcher, `key_search::Search`, for the pane and the dialog. A row stays when:
  - a chord was pressed and the row has a key that is that chord (`same`, or `clash` for a range of digits), as the pane does today; and
  - words were typed and they are in the command's name or its group's heading (any case), **or** they are one of the row's keys as written (any case; `+`, `-` and spaces alike), **or** they spell a key (`spell_typed`) that the row has.
- `spell_typed` reads what a person types: `cmd`/`command`/`⌘`, `ctrl`/`control`/`⌃`, `alt`/`opt`/`option`/`⌥`, `shift`/`⇧`, joined by `+`, `-` or spaces, the key last. So on macOS "cmd+r", "cmd r" and "⌘r" find ⌘R; on Windows "ctrl+r" finds Ctrl+R by its written text already.
- Pressing a chord in the field (Cmd, Ctrl or Alt held) finds it, in place of typing it, with Settings' `find_chord`: it moves to `key_search` with `held`. A find empties the typed text; typing drops the find.
- Esc, read before the field draws: with text or a find, `HelpSearch(String::new())` (both cleared, the field keeps the keyboard); with nothing, `CloseDialog`.
- The field takes the keyboard on the dialog's first frame (egui temp data, dropped by `help::fold` when the dialog is gone). It does not lock it: Tab still reaches the buttons.
- The list area keeps the height of the full list (capped by the window) whatever the search leaves.
- Names are marked with `start::marked`, as Quick open marks them; a key cap that the typed text matched is marked whole.
- Omarchy: the field is `widgets::filter_field` (the `/` line) with the count at its right end, 0 in the danger colour.
- No results: "No commands match “{words}”" and "Esc clears the search" (Omarchy: lower case, as the look writes labels).

## Files

- Modify `crates/tabletist-core/src/keymap.rs`: `Keymap::caps`, `Keymap::command_keys`, `spell_typed`, tests.
- Create `src/ui/key_search.rs`: `Search`, `find_folded`, `held`, `find_chord`, tests.
- Modify `src/ui/settings/keyboard.rs`: `listed` reads `Search`; `held`, `find_chord` go.
- Modify `src/ui/settings/keyboard_sheet.rs`: `key_search::find_chord`.
- Modify `crates/tabletist-core/src/model.rs`: `HelpSearch`, `Dialog::Help(HelpSearch)`, `Action::HelpSearch`, `Action::HelpFind`.
- Modify `src/app.rs`: the two actions; `Dialog::Help` → `Dialog::Help(_)` / `Dialog::Help(HelpSearch::default())` there and in `src/app/updates.rs`, `src/ui/mod.rs`, `src/ui/dialog_focus_tests.rs`, `src/ui/start_tests.rs`, `src/menu/tests.rs`.
- Modify `src/ui/help.rs`: `listed`, the field, the found line, the empty line, marks, the kept height, Esc, Omarchy's count and hints, `fold`.
- Modify `src/ui/mod.rs`: `mod key_search;`, `help::show(…, search)`, `help::fold`, `#[cfg(test)] mod help_tests;`.
- Create `src/ui/help_tests.rs`.

---

### Task 1: The keymap writes a command's caps and reads typed keys

**Files:** Modify `crates/tabletist-core/src/keymap.rs` (`Keymap::label` ~3464; tests module at the end).

- [ ] **Step 1: Write the failing tests** in keymap.rs's tests module:

```rust
#[test]
fn typed_keys_read_as_a_person_types_them() {
    let read = |text| spell_typed(text);
    let cmd_r = spell("cmd+r").unwrap();
    for text in ["cmd+r", "cmd r", "Cmd-R", "command+r", "⌘r", "⌘R"] {
        let typed = read(text).unwrap_or_else(|| panic!("{text}"));
        assert!(same(Layout::Mac, &typed, &cmd_r), "{text}");
    }
    let shifted = spell("cmd+shift+f").unwrap();
    for text in ["shift+cmd+f", "⇧⌘F", "cmd+shift+F"] {
        assert!(same(Layout::Mac, &read(text).unwrap(), &shifted), "{text}");
    }
    assert!(same(Layout::Mac, &read("opt+e").unwrap(), &spell("alt+e").unwrap()));
    assert!(same(Layout::Mac, &read("control+r").unwrap(), &spell("ctrl+r").unwrap()));
    // A key alone keeps its case: Omarchy's R is not its r.
    assert!(same(Layout::Omarchy, &read("R").unwrap(), &spell("R").unwrap()));
    assert!(read("row").is_none());
    assert!(read("").is_none());
}

#[test]
fn a_commands_caps_are_its_label_one_by_one() {
    let keymap = Keymap::default();
    for layout in [Layout::Mac, Layout::Windows, Layout::Omarchy] {
        for command in Command::ALL {
            let caps: Vec<String> = keymap.caps(layout, *command).iter().map(|c| c.to_string()).collect();
            assert_eq!(caps.join(", "), keymap.label(layout, *command).as_str(), "{command:?}");
        }
    }
    assert!(keymap.command_keys(Layout::Mac, Command::Reload).contains(&"cmd+r"));
}
```

(Use the keymap constructor the tests module already uses if `Keymap::default()` is not it.)

- [ ] **Step 2: Run them, expect a compile failure** (`spell_typed`, `caps`, `command_keys` missing):
  `CARGO_TARGET_DIR=$PWD/target ~/.cargo/bin/cargo test --locked -p tabletist-core keymap::tests::typed_keys`

- [ ] **Step 3: Implement.** Split `label` into `caps` (the `all` vector as `Vec<Written>`) and `label` (`Written(caps.join(", "))`). Add beside `keys_of`:

```rust
/// A command's keys in `layout`, every line of it, as spelled: what a
/// key pressed or typed to find it is compared with.
pub fn command_keys(&self, layout: Layout, command: Command) -> Vec<&'static str> {
    let mut keys: Vec<&'static str> = Vec::new();
    for binding in self.of(command) {
        for chord in binding.bound(layout).chords {
            if chord.kind != Kind::Via && !keys.contains(&chord.keys) {
                keys.push(chord.keys);
            }
        }
    }
    keys
}
```

and near `spells`:

```rust
/// What a person typed as a key, read as a spelling: `cmd`, `command`
/// or `⌘`; `ctrl`, `control` or `⌃`; `alt`, `opt`, `option` or `⌥`;
/// `shift` or `⇧`; joined by `+`, `-` or spaces, the key last, in any
/// case. A key alone keeps its case. None for what reads as no key.
pub fn spell_typed(text: &str) -> Option<Spelled> {
    let mut expanded = String::new();
    for char in text.trim().chars() {
        match char {
            '⌘' => expanded.push_str(" cmd "),
            '⌃' => expanded.push_str(" ctrl "),
            '⌥' => expanded.push_str(" alt "),
            '⇧' => expanded.push_str(" shift "),
            other => expanded.push(other),
        }
    }
    let parts: Vec<&str> = expanded
        .split(|c: char| c == '+' || c == '-' || c.is_whitespace())
        .filter(|part| !part.is_empty())
        .collect();
    let (key, held) = parts.split_last()?;
    let mut mods = [false; 4];
    for word in held {
        let at = match word.to_lowercase().as_str() {
            "cmd" | "command" => 0,
            "ctrl" | "control" => 1,
            "alt" | "opt" | "option" => 2,
            "shift" => 3,
            _ => return None,
        };
        mods[at] = true;
    }
    let key = if held.is_empty() && key.chars().count() == 1 {
        (*key).to_owned()
    } else {
        key.to_lowercase()
    };
    let mut spelled = String::new();
    for (held, word) in mods.iter().zip(["cmd+", "ctrl+", "alt+", "shift+"]) {
        if *held {
            spelled.push_str(word);
        }
    }
    spelled.push_str(&key);
    spell_keys(&spelled).ok()
}
```

- [ ] **Step 4: Run the keymap tests, expect PASS:** `… cargo test --locked -p tabletist-core keymap`
- [ ] **Step 5: Commit** "Write a command's key caps one by one, and read keys as a person types them".

### Task 2: One matcher for Settings and the dialog

**Files:** Create `src/ui/key_search.rs`; modify `src/ui/mod.rs` (`mod key_search;`), `src/ui/settings/keyboard.rs`, `src/ui/settings/keyboard_sheet.rs:218`.

- [ ] **Step 1: Write the failing tests** at the end of `key_search.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn words_are_found_in_any_case_where_they_are() {
        assert_eq!(find_folded("Add row above", "ROW"), Some(4..7));
        assert_eq!(find_folded("Größe ändern", "ÄN"), Some(7..10));
        assert_eq!(find_folded("Reload", "x"), None);
    }

    #[test]
    fn a_command_stays_by_its_name_its_group_or_its_keys() {
        let mac = Layout::Mac;
        let reload = (&["cmd+r"][..], &["⌘R"][..]);
        let keeps = |text: &str, found| {
            Search::new(text, found).keeps(mac, "Reload", "Navigation", reload.0, reload.1)
        };
        assert!(keeps("rel", None));
        assert!(keeps("navig", None));
        assert!(keeps("⌘r", None));
        assert!(keeps("cmd+r", None));
        assert!(keeps("cmd r", None));
        assert!(!keeps("cmd+t", None));
        assert!(!keeps("row", None));
        assert!(keeps("", Some("cmd+r")));
        assert!(!keeps("", Some("cmd+t")));
        // Windows writes its keys as words: they are matched as written.
        let win = Search::new("ctrl-r", None);
        assert!(win.keeps(Layout::Windows, "Reload", "Navigation", &["cmd+r"], &["Ctrl+R"]));
        // A range of digits holds each of them.
        let digit = Search::new("", Some("cmd+3"));
        assert!(digit.keeps(mac, "Go to connection window", "Window", &["cmd+1..9"], &["⌘1…9"]));
    }

    #[test]
    fn the_keys_typed_mark_their_cap() {
        let search = Search::new("cmd+r", None);
        assert_eq!(search.cap_hit(Layout::Mac, &["cmd+r"], &["⌘R"]), Some(0));
        assert_eq!(Search::new("rel", None).cap_hit(Layout::Mac, &["cmd+r"], &["⌘R"]), None);
        assert_eq!(Search::new("rel", None).name_spans("Reload"), vec![0..3]);
    }
}
```

(If `cmd+1..9` is not how the keymap spells the range, use the spelling `Command::GoToConnection`'s binding has.)

- [ ] **Step 2: Run, expect compile failure:** `… cargo test --locked -p tabletist --lib ui::key_search`
- [ ] **Step 3: Implement** `src/ui/key_search.rs`:

```rust
//! Finding commands by what is typed or pressed: the Keyboard settings'
//! list and the keyboard shortcuts dialog (`?`) read this one search.

use std::ops::Range;

use egui::{Event, Key, Modifiers};

use crate::keymap::{Layout, Mods, Spelled, clash, intern, same, spell, spell_typed, spelling};

/// A search: the words typed, the key they spell if they spell one, and
/// a chord pressed to find what it does.
pub(crate) struct Search {
    words: String,
    typed: Option<Spelled>,
    found: Option<Spelled>,
}

impl Search {
    pub(crate) fn new(text: &str, found: Option<&'static str>) -> Self {
        let words = text.trim().to_owned();
        Search {
            typed: spell_typed(&words),
            words,
            found: found.and_then(|keys| spell(keys).ok()),
        }
    }

    /// Whether a command with this name, group heading, keys (as spelled)
    /// and caps (as the look writes them) stays in the list.
    pub(crate) fn keeps(&self, layout: Layout, name: &str, heading: &str, keys: &[&'static str], caps: &[&str]) -> bool {
        if let Some(found) = &self.found
            && !keys.iter().filter_map(|keys| spell(keys).ok()).any(|keys| is(layout, &keys, found))
        {
            return false;
        }
        self.words.is_empty()
            || find_folded(name, &self.words).is_some()
            || find_folded(heading, &self.words).is_some()
            || self.cap_hit(layout, keys, caps).is_some()
    }

    /// Which of `caps` the words name: one written as they are typed, or
    /// the key they spell.
    pub(crate) fn cap_hit(&self, layout: Layout, keys: &[&'static str], caps: &[&str]) -> Option<usize> {
        if self.words.is_empty() {
            return None;
        }
        let typed = fold(&self.words);
        if let Some(at) = caps.iter().position(|cap| fold(cap) == typed) {
            return Some(at);
        }
        let spelled = self.typed.as_ref()?;
        let key = keys.iter().copied().find(|keys| spell(keys).ok().is_some_and(|keys| is(layout, &keys, spelled)))?;
        let written = crate::keymap::key_written(layout, key);
        caps.iter().position(|cap| cap.contains(written.as_str())).or(Some(0))
    }

    /// Where the words are in a command's name, to mark.
    pub(crate) fn name_spans(&self, name: &str) -> Vec<Range<usize>> {
        if self.words.is_empty() { return Vec::new(); }
        find_folded(name, &self.words).into_iter().collect()
    }
}

/// Whether `keys` is the key `wanted`: a range of digits holds each digit.
fn is(layout: Layout, keys: &Spelled, wanted: &Spelled) -> bool {
    match keys {
        Spelled::Digits(_) => clash(layout, keys, wanted),
        _ => same(layout, keys, wanted),
    }
}

/// A key's text without case, its `+`, `-` and spaces alike.
fn fold(text: &str) -> String { /* lower case; each run of '+', '-', whitespace → '+' */ }

/// Where `needle` is in `hay`, in any case: the byte range of `hay`.
pub(crate) fn find_folded(hay: &str, needle: &str) -> Option<Range<usize>> {
    // For each char boundary of `hay`, walk its chars lower-cased against
    // the needle's lower-cased chars; the end is the boundary after the
    // last char used.
}

// `held` and `find_chord`, moved from settings/keyboard.rs unchanged
// (`pub(crate)`), with `clipboard_key` left where `record` uses it.
```

In `settings/keyboard.rs` `listed`: replace the words and found blocks with

```rust
let search = crate::ui::key_search::Search::new(&page.search, page.found);
…
let name = gettext(locale, row.command.info().name);
let heading = gettext(locale, group.name(layout));
let caps = keymap.row_keys(layout, row);
let caps: Vec<&str> = caps.iter().map(|cap| cap.as_str()).collect();
if !search.keeps(layout, &name, &heading, &keymap.keys_of(layout, row), &caps) {
    continue;
}
```

and import `held` from `crate::ui::key_search` for `record`. In `keyboard_sheet.rs:218` call `crate::ui::key_search::find_chord`.

- [ ] **Step 4: Run** the new tests and the pane's (`… cargo test --locked -p tabletist --lib ui::key_search ui::keyboard_tests`), expect PASS.
- [ ] **Step 5: Commit** "Find commands for Settings with one search, which reads typed keys too".

### Task 3: The dialog's search in the model

**Files:** `crates/tabletist-core/src/model.rs` (`Dialog` ~2844, `Action` ~1062), `src/app.rs` (~2690), and every `Dialog::Help`.

- [ ] **Step 1: Write the failing test** in `src/ui/help_tests.rs` (registered in `src/ui/mod.rs` under `#[cfg(test)]`):

```rust
fn search(harness: &Harness) -> (String, Option<&'static str>) {
    match &harness.app.dialog {
        Some(Dialog::Help(help)) => (help.search.clone(), help.found),
        other => panic!("{other:?}"),
    }
}

#[test]
fn the_search_is_the_dialogs_own_and_a_find_drops_the_words() {
    let mut harness = Harness::new();
    harness.app.apply(Action::ShowHelp);
    harness.app.apply(Action::HelpSearch("row".into()));
    assert_eq!(search(&harness), ("row".into(), None));
    harness.app.apply(Action::HelpFind(Some("cmd+r")));
    assert_eq!(search(&harness), (String::new(), Some("cmd+r")));
    harness.app.apply(Action::HelpSearch("re".into()));
    assert_eq!(search(&harness), ("re".into(), None));
    harness.app.apply(Action::CloseDialog);
    harness.app.apply(Action::ShowHelp);
    assert_eq!(search(&harness), (String::new(), None));
}
```

- [ ] **Step 2: Run, expect compile failure.**
- [ ] **Step 3: Implement.** In model.rs:

```rust
/// The keyboard shortcuts dialog's search.
#[derive(Debug, Default)]
pub struct HelpSearch {
    /// What its field holds.
    pub search: String,
    /// A chord pressed in the field, to list what it does.
    pub found: Option<&'static str>,
}
```

`Dialog::Help(HelpSearch)`; actions `HelpSearch(String)` ("The shortcuts dialog's search: what its field holds; a find is dropped") and `HelpFind(Option<&'static str>)` ("List the commands a chord is a key of; the words are dropped"). In `App::apply`:

```rust
Action::HelpSearch(text) => {
    if let Some(Dialog::Help(help)) = &mut self.dialog {
        help.search = text;
        help.found = None;
    }
}
Action::HelpFind(keys) => {
    if let Some(Dialog::Help(help)) = &mut self.dialog {
        help.found = keys;
        help.search.clear();
    }
}
```

`ShowHelp` makes `Dialog::Help(HelpSearch::default())`. Elsewhere: `sed -i 's/Some(Dialog::Help)/Some(Dialog::Help(_))/g'` on the files listed above, then fix the remaining constructors and `None | Some(Dialog::Help)` patterns by hand.

- [ ] **Step 4: Run** the test and `cargo build --locked --all-targets`; expect PASS.
- [ ] **Step 5: Commit** "Keep the shortcuts dialog's search in the model".

### Task 4: The dialog lists what the search keeps

**Files:** `src/ui/help.rs`, `src/ui/mod.rs:256`.

- [ ] **Step 1: Write the failing tests** (help_tests.rs):

```rust
fn listed(harness: &Harness, text: &str, found: Option<&'static str>) -> Vec<&'static str> {
    let layout = harness.app.layout();
    let search = crate::model::HelpSearch { search: text.into(), found };
    crate::ui::help::listed(&harness.app.keymap, layout, Locale::default(), &search)
        .into_iter()
        .map(|row| row.command.info().name)
        .collect()
}

#[test]
fn words_keys_and_chords_cut_the_list_in_every_look() {
    for (look, typed, chord) in [
        (Look::macos(), "cmd+r", "cmd+r"),
        (Look::standard(), "ctrl+r", "cmd+r"),
        (Look::omarchy(), "R", "R"),
    ] {
        let mut harness = Harness::new();
        harness.set_look(look);
        let all = listed(&harness, "", None);
        assert_eq!(all.len(), crate::ui::help::rows(&harness.app.keymap, harness.app.layout()).len());
        let rows = listed(&harness, "row", None);
        assert!(rows.contains(&"Add row") && !rows.contains(&"Reload"), "{}: {rows:?}", look.name);
        assert!(listed(&harness, "SQL EDITOR", None).contains(&"Run statement"), "{}", look.name);
        assert!(listed(&harness, typed, None).contains(&"Reload"), "{}", look.name);
        assert!(listed(&harness, "", Some(chord)).contains(&"Reload"), "{}", look.name);
        assert!(listed(&harness, "zzz", None).is_empty(), "{}", look.name);
    }
}
```

- [ ] **Step 2: Run, expect compile failure** (`listed` missing).
- [ ] **Step 3: Implement** in help.rs:

```rust
/// A command the dialog lists: its group, its keys as the look writes
/// them, which of its caps the typed keys named, and where the words are
/// in its name.
pub(crate) struct Listed {
    pub group: Group,
    pub command: Command,
    pub keys: Written,
    pub cap: Option<usize>,
    pub spans: Vec<std::ops::Range<usize>>,
}

pub(crate) fn listed(keymap: &Keymap, layout: Layout, locale: Locale, search: &HelpSearch) -> Vec<Listed> {
    let finder = Search::new(&search.search, search.found);
    rows(keymap, layout)
        .into_iter()
        .filter_map(|(group, command, keys)| {
            let name = gettext(locale, command.info().name);
            let heading = gettext(locale, heading_of(group));
            let caps = keymap.caps(layout, command);
            let caps: Vec<&str> = caps.iter().map(|cap| cap.as_str()).collect();
            let spelled = keymap.command_keys(layout, command);
            finder.keeps(layout, &name, &heading, &spelled, &caps).then(|| Listed {
                group,
                command,
                cap: finder.cap_hit(layout, &spelled, &caps),
                spans: finder.name_spans(&name),
                keys,
            })
        })
        .collect()
}
```

with `heading_of(group)` the `GROUPS` lookup `show` already does. Omarchy writes names through `look.label`, which only lower-cases: matching in any case makes the spans the same.

- [ ] **Step 4: Run, expect PASS.**
- [ ] **Step 5: Commit** "List in the shortcuts dialog what its search keeps".

### Task 5: The field, the found line, the empty line, marks and Esc

**Files:** `src/ui/help.rs` (`show`), `src/ui/mod.rs`.

- [ ] **Step 1: Write the failing tests** (help_tests.rs), each over `Look::ALL`:

```rust
/// The dialog open over the start screen in `look`, settled.
fn opened(look: Look) -> Harness {
    let mut harness = Harness::new();
    harness.set_look(look);
    harness.app.apply(Action::ShowHelp);
    harness.settle();
    harness
}

#[test]
fn the_field_has_the_keyboard_as_the_dialog_opens() // focused_name == the field's name ("Search commands or press a shortcut", Omarchy's words lower case)

#[test]
fn typed_text_goes_to_the_field_not_to_its_commands() // type "?" then "R" (Omarchy) / "?" (others): search == "?R"/"?", dialog still Help, no other dialog

#[test]
fn a_chord_pressed_in_the_field_finds_its_commands() // macOS ⌘R (Modifiers::MAC_CMD|COMMAND), Windows Ctrl+R (Modifiers::CTRL|COMMAND), Omarchy ctrl+r: found set, the line "Commands bound to …" shown, the dialog still open, nothing reloaded

#[test]
fn no_results_say_so() // HelpSearch("zzz"): harness.has("No commands match “zzz”") (Omarchy lower case)

#[test]
fn esc_clears_the_search_then_closes() // type "row"; Esc → search empty, still Help, the field still focused; Esc → dialog None. Same after a find.

#[test]
fn the_list_keeps_its_height_while_searching() // the dialog's rect (accesskit Dialog node bounds) is as tall with "zzz" as with ""
```

Write them out in full with the harness helpers: `harness.frame(vec![Event::Text(..)])`, `harness.press(Key, Modifiers)`, `super::tests::focused_name(&harness.settle())`, `harness.has(..)`.

- [ ] **Step 2: Run, expect FAIL** (no field).
- [ ] **Step 3: Implement** `show(ctx, paint, keymap, search: &HelpSearch) -> Vec<Action>`:
  1. `let listed = listed(…)`, `let all = rows(…)` (widths and the kept height come from `all`).
  2. Inside the modal, before the field: Esc (`input.consume_key(Modifiers::NONE, Key::Escape)`) → `HelpSearch(String::new())` when `!search.search.is_empty() || search.found.is_some()`, else `CloseDialog`. Then, when the field has focus, `key_search::find_chord(ctx, layout)` → `HelpFind(Some(keys))`.
  3. The field: Omarchy `widgets::filter_field(…, FieldStyle { fill: None, boxed: true, role: TextRole::OBody }, …)` with hint `look.label("Search commands or press a shortcut")`, and the count painted right-aligned inside it (`palette.danger` at 0, `palette.dim` else) while text is typed; other looks `widgets::search_field` + `widgets::add_search`, full width, `.id(field_id())`. A change → `HelpSearch(text)`. On the dialog's first frame (`ctx.data` temp flag at `field_id().with("opened")`, absent) `request_focus()` and set the flag; `pub fn fold(ctx)` removes it, called from `ui/mod.rs` when the dialog is not Help (as `about::fold`).
  4. Windows (`!look.terminal && !look.capsule_search`): while the field has focus, a 2 pt `palette.accent` line along its bottom inside the border.
  5. A find: "Commands bound to {keys}" (`key_written`) in `TextRole::Secondary`, then a "Show all" link → `HelpFind(None)`.
  6. The list: the existing `ScrollArea`, `auto_shrink([false, false])`, `max_height` = the full list's height (rows × (row height + 6) + headings) capped by the window. Rows from `listed`; the name drawn with `start::marked(look, &name, &spans, (body, text, UiBodySemibold/OGroup, accent strong))`; the keys likewise with the hit cap's span in `keys` marked. Nothing listed: centred "No commands match “{words}”" and "Esc clears the search", `look.label`led.
  7. Omarchy, under the buttons: `widgets::key_hints` with `(key_written(layout, "esc"), "clear" | "close", true)` and `(Written::default(), "type to filter, or press keys to find them", true)`.
- [ ] **Step 4: Run** help_tests, dialog_focus_tests, keyboard_tests, start_tests, menu tests: expect PASS.
- [ ] **Step 5: Look at it by eye** with a throwaway screenshot scene in the scratchpad (never committed), in the three looks: opened, "row", ⌘R found, "zzz".
- [ ] **Step 6: Commit** "Search the keyboard shortcuts dialog by name, group or keys (#300)".

### Task 6: Checks

- [ ] `~/.cargo/bin/cargo fmt --all`
- [ ] `CARGO_TARGET_DIR=$PWD/target ~/.cargo/bin/cargo clippy --locked --workspace --all-targets -- -D warnings`
- [ ] `CARGO_TARGET_DIR=$PWD/target ~/.cargo/bin/cargo test --locked -p tabletist --all-targets` (and `-p tabletist-core` for Task 1's tests)
- [ ] Commit any fmt changes. Ask the user before pushing. The PR says "Part of #300", never "Fixes".
