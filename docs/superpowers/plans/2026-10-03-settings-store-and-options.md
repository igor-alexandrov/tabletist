# Settings: the store and the options Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Settings live in `settings.toml`, read tolerantly and written in one canonical form, with an older `settings.json` carried over once; and three new options (timestamps, number grouping, value tags) change what the grids show.

**Architecture:** `src/settings.rs` gains a canonical writer (`to_toml`) and a tolerant reader (`from_toml`, on `toml::de::DeTable::parse_recoverable`) that returns the settings together with the file's text, the lines it ignored and the line of each key. `Settings::load` picks the source (TOML, the old JSON, or defaults) and writes no settings; `App::new` writes the TOML once when the JSON was the source. The three options are read where cells are built: a workspace starts with the timestamps option, `data_view::cell` groups numbers, and the three views drop value tags when the option is off.

**Tech Stack:** Rust 2024, egui (crmne fork, 0.36), `toml` 1.1 (parser only). Headless UI tests through `src/testing.rs`.

**Spec:** `docs/superpowers/specs/2026-10-03-settings-general-design.md`. This plan is step 1 of its four ("The store and the options"). There is no window in this step: the options are set by editing the file and take effect at the next start. Steps 2 to 4 (live reload, the Omarchy screen, the macOS and Windows window) get their own plans.

---

## Ground rules (read first)

- Work in `/home/igor/Work/tabletist/.claude/worktrees/settings-general-tab-957dcf`, branch `claude/settings-general-tab-957dcf`.
- `cargo` through mise is broken here: always run `~/.cargo/bin/cargo`. Never point `CARGO_TARGET_DIR` at `/tmp`.
- Every `cargo` command takes `--locked`. `Cargo.lock` is edited by hand in Task 4 and nowhere else; if any command wants to change it, stop and report.
- One topic per commit, signed: plain `git commit -S`. Never set `SSH_AUTH_SOCK` in a git command. If signing fails with "agent refused operation", 1Password has locked again: do **not** commit unsigned. Stage the task's files, run `git write-tree`, note the tree id and the commit message in your report, and go on to the next task. End every message with `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.
- Never push.
- No em dashes anywhere (code, comments, docs, commit messages). Use a full stop, comma, colon or parentheses.
- Tests are behavioural. Never a design or pixel conformance test. Design material is never committed.
- Test fixtures use neutral names only (`Fixture`, `Bookshop`, `users`, `editions`).
- Views do not mutate application state; they read `app.settings` and push `Action`s.
- Comments say why, in the voice of the code around them. Match the file you are in.
- `clippy::unwrap_used` warns outside tests: use `?`, `let else`, or `unwrap_or`.

## File structure

| File | Change |
|---|---|
| `src/ui/format.rs` | `group_number`, sharing its digit grouping with `group_digits`. |
| `src/settings.rs` | `Timestamps`, three new fields, `Key`, `Source`, `Loaded`, `to_toml`, `from_toml`, `load(dirs)`, `save` as TOML. `version` goes. |
| `src/paths.rs` | `settings_file()` is `settings.toml`; `legacy_settings_file()` is the JSON. |
| `src/util.rs` | `keep_aside` becomes `pub`. |
| `src/backend.rs` | The settings save test reads TOML. |
| `src/entrypoint.rs` | Loads with `Settings::load(&dirs)`. |
| `src/app.rs` | `App::new` takes a `Loaded`; writes the TOML once after a JSON source; a workspace starts with the timestamps option. |
| `src/testing.rs` | The harness passes `Settings::default().into()`. |
| `src/ui/value_tags.rs` | `Tags::when`. |
| `src/ui/data_view.rs` | `Shown`; `cell` and `plain_cell` take it; grouping; `is_key`; tags follow the option. |
| `src/ui/sql_results.rs` | `Place` carries the two options to the grid. |
| `src/ui/row_panel.rs` | Tags follow the option. |
| `src/ui/mod.rs` | UI tests. |
| `Cargo.toml`, `Cargo.lock` | `toml` as a direct dependency. |

---

### Task 1: Group a number's digits

**Files:**
- Modify: `src/ui/format.rs` (`group_digits` near line 676, tests in `mod tests` near line 831)

- [ ] **Step 1: Write the failing tests**

Add to `mod tests` in `src/ui/format.rs`:

```rust
    #[test]
    fn a_number_groups_its_integer_digits_in_threes() {
        use super::group_number;
        assert_eq!(group_number("1240.50"), "1,240.50");
        assert_eq!(group_number("-9100000"), "-9,100,000");
        assert_eq!(group_number("1234567.891011"), "1,234,567.891011");
        // More digits than any integer type holds: it is text all the way.
        assert_eq!(
            group_number("123456789012345678901234567890"),
            "123,456,789,012,345,678,901,234,567,890"
        );
    }

    #[test]
    fn what_is_not_a_plain_number_is_left_as_it_is() {
        use super::group_number;
        use std::borrow::Cow;
        for text in [
            "999", "-999", "0.5", "1e21", "NaN", "inf", "-inf", "$1240.50", "1,240", "1240.",
            ".5", "", "-", "1.2.3", "１２３４", " 1240",
        ] {
            assert!(
                matches!(group_number(text), Cow::Borrowed(same) if same == text),
                "{text:?}"
            );
        }
    }
```

- [ ] **Step 2: Run them to see them fail**

Run: `~/.cargo/bin/cargo test --locked -p tabletist --lib ui::format::tests::a_number_groups`
Expected: does not compile, "cannot find function `group_number`".

- [ ] **Step 3: Implement**

Replace `group_digits` in `src/ui/format.rs` with:

```rust
/// `1234567` as `1,234,567`.
pub fn group_digits(number: u64) -> String {
    grouped(&number.to_string())
}

/// A run of digits with a comma between every three, counted from the end.
fn grouped(digits: &str) -> String {
    let mut out = String::with_capacity(digits.len() + digits.len() / 3);
    for (index, digit) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index).is_multiple_of(3) {
            out.push(',');
        }
        out.push(digit);
    }
    out
}

/// A plain decimal number with its integer digits in threes: `1240.50` as
/// `1,240.50`. Anything else (an exponent, `NaN`, a currency sign) comes
/// back as it is, and so does a number with nothing to group. The fraction
/// is never touched.
pub fn group_number(text: &str) -> Cow<'_, str> {
    let (sign, rest) = match text.strip_prefix('-') {
        Some(rest) => ("-", rest),
        None => ("", text),
    };
    let (whole, fraction) = match rest.split_once('.') {
        Some((whole, fraction)) => (whole, Some(fraction)),
        None => (rest, None),
    };
    let digits = |part: &str| !part.is_empty() && part.bytes().all(|b| b.is_ascii_digit());
    if !digits(whole) || fraction.is_some_and(|fraction| !digits(fraction)) || whole.len() <= 3 {
        return Cow::Borrowed(text);
    }
    let mut out = format!("{sign}{}", grouped(whole));
    if let Some(fraction) = fraction {
        out.push('.');
        out.push_str(fraction);
    }
    Cow::Owned(out)
}
```

- [ ] **Step 4: Run the tests**

Run: `~/.cargo/bin/cargo test --locked -p tabletist --lib ui::format::`
Expected: PASS, including the existing `group_digits` assertions.

- [ ] **Step 5: Commit**

```bash
git add src/ui/format.rs
git commit -S -m "Group a number's digits in threes

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 2: The three options in `Settings`

**Files:**
- Modify: `src/settings.rs`

- [ ] **Step 1: Write the failing tests**

In `src/settings.rs`'s `mod tests`, replace `defaults_match_the_spec` with:

```rust
    #[test]
    fn defaults_match_the_spec() {
        let settings = Settings::default();
        assert_eq!(settings.page_size, 300);
        assert_eq!(settings.timestamps, Timestamps::Second);
        assert!(!settings.group_digits);
        assert!(settings.value_tags);
        assert!(!settings.show_system_schemas);
        assert_eq!(settings.custom_theme, None);
        assert_eq!(settings.sql_limit, 1_000);
        assert_eq!(settings.sql_timeout_secs, Some(30));
        assert_eq!(
            settings.sql_timeout(),
            Some(std::time::Duration::from_secs(30))
        );
    }

    #[test]
    fn an_older_file_with_a_version_gets_the_new_defaults() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        std::fs::write(&path, br#"{"version": 1, "page_size": 100}"#).unwrap();
        let settings = Settings::load(&path);
        assert_eq!(settings.page_size, 100);
        assert_eq!(settings.timestamps, Timestamps::Second);
        assert!(!settings.group_digits);
        assert!(settings.value_tags);
    }

    #[test]
    fn a_timestamps_choice_has_a_name() {
        for choice in [Timestamps::Second, Timestamps::Full] {
            assert_eq!(Timestamps::from_name(choice.name()), Some(choice));
        }
        assert_eq!(Timestamps::from_name("minute"), None);
    }
```

- [ ] **Step 2: Run them to see them fail**

Run: `~/.cargo/bin/cargo test --locked -p tabletist --lib settings::`
Expected: does not compile, "cannot find type `Timestamps`".

- [ ] **Step 3: Implement**

In `src/settings.rs`, above `Settings`:

```rust
/// How much of a timestamp the grid shows.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Timestamps {
    /// `2026-01-12 09:14:03`.
    #[default]
    Second,
    /// With the fraction the server sent.
    Full,
}

impl Timestamps {
    /// As the file says it.
    pub fn name(self) -> &'static str {
        match self {
            Self::Second => "second",
            Self::Full => "full",
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        [Self::Second, Self::Full]
            .into_iter()
            .find(|choice| choice.name() == name)
    }
}
```

In `Settings`: delete the `version` field and its comment, and add after `page_size`:

```rust
    /// Timestamps in grid cells: to the second, or as the server sent them.
    pub timestamps: Timestamps,
    /// Numbers in grid cells with their integer digits in threes. Display
    /// only: a copy gives the value as it is.
    pub group_digits: bool,
    /// Enum, CHECK and boolean values drawn as coloured tags.
    pub value_tags: bool,
```

In `impl Settings`: delete `CURRENT_VERSION`, and change `load` to share its clamping:

```rust
    /// The settings with every number in its range.
    fn validated(mut self) -> Self {
        self.page_size = self
            .page_size
            .clamp(Self::MIN_PAGE_SIZE, Self::MAX_PAGE_SIZE);
        self.sql_limit = Self::valid_sql_limit(self.sql_limit);
        self.sql_timeout_secs = Self::valid_sql_timeout(self.sql_timeout_secs);
        self
    }

    pub fn load(path: &Path) -> Self {
        crate::util::load_json::<Settings>(path).validated()
    }
```

In `Default for Settings`: delete the `version` line and add:

```rust
            timestamps: Timestamps::Second,
            group_digits: false,
            value_tags: true,
```

An old file's `"version"` key is now an unknown key, which serde ignores.

- [ ] **Step 4: Run the tests**

Run: `~/.cargo/bin/cargo test --locked -p tabletist --lib settings::`
Expected: PASS (every test of the module).

Run: `~/.cargo/bin/cargo check --locked --workspace --all-targets`
Expected: no errors (nothing else names `version`).

- [ ] **Step 5: Commit**

```bash
git add src/settings.rs
git commit -S -m "Give the settings their timestamps, grouping and value tag options

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 3: The canonical text

**Files:**
- Modify: `src/settings.rs`

- [ ] **Step 1: Write the failing tests**

Add to `mod tests`:

```rust
    #[test]
    fn the_defaults_are_written_as_the_canonical_text() {
        assert_eq!(
            Settings::default().to_toml(),
            "\
# written by tabletist, safe to edit by hand
[data]
page_size    = 300
timestamps   = \"second\"  # second | full
group_digits = false
value_tags   = true

[sidebar]
show_system_schemas = false

[editor]
sql_limit        = 1000
sql_timeout_secs = 30  # 0 waits forever
"
        );
    }

    #[test]
    fn a_theme_gets_its_table_and_no_timeout_is_zero() {
        let settings = Settings {
            custom_theme: Some("My \"Nord\".json".into()),
            sql_timeout_secs: None,
            timestamps: Timestamps::Full,
            ..Settings::default()
        };
        let text = settings.to_toml();
        assert!(text.contains("timestamps   = \"full\"  # second | full\n"), "{text}");
        assert!(text.contains("sql_timeout_secs = 0  # 0 waits forever\n"), "{text}");
        assert!(
            text.ends_with("\n[appearance]\ntheme = \"My \\\"Nord\\\".json\"\n"),
            "{text}"
        );
    }

    #[test]
    fn every_key_has_its_own_place_in_the_file() {
        let mut paths: Vec<_> = Key::ALL.iter().map(|key| key.path()).collect();
        paths.sort_unstable();
        paths.dedup();
        assert_eq!(paths.len(), Key::ALL.len());
    }
```

- [ ] **Step 2: Run them to see them fail**

Run: `~/.cargo/bin/cargo test --locked -p tabletist --lib settings::`
Expected: does not compile, "no method named `to_toml`".

- [ ] **Step 3: Implement**

In `src/settings.rs`, above `impl Settings`:

```rust
/// A key of the settings file.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Key {
    PageSize,
    Timestamps,
    GroupDigits,
    ValueTags,
    ShowSystemSchemas,
    SqlLimit,
    SqlTimeoutSecs,
    Theme,
}

impl Key {
    /// In the order the file has them: a table's keys together.
    pub const ALL: [Key; 8] = [
        Self::PageSize,
        Self::Timestamps,
        Self::GroupDigits,
        Self::ValueTags,
        Self::ShowSystemSchemas,
        Self::SqlLimit,
        Self::SqlTimeoutSecs,
        Self::Theme,
    ];

    /// The key's table and its name there.
    pub fn path(self) -> (&'static str, &'static str) {
        match self {
            Self::PageSize => ("data", "page_size"),
            Self::Timestamps => ("data", "timestamps"),
            Self::GroupDigits => ("data", "group_digits"),
            Self::ValueTags => ("data", "value_tags"),
            Self::ShowSystemSchemas => ("sidebar", "show_system_schemas"),
            Self::SqlLimit => ("editor", "sql_limit"),
            Self::SqlTimeoutSecs => ("editor", "sql_timeout_secs"),
            Self::Theme => ("appearance", "theme"),
        }
    }

    /// What the file says beside the key: the values of a closed set, or
    /// what a value that reads oddly means.
    fn note(self) -> Option<&'static str> {
        match self {
            Self::Timestamps => Some("second | full"),
            Self::SqlTimeoutSecs => Some("0 waits forever"),
            _ => None,
        }
    }
}

/// The file's first line.
const HEADER: &str = "# written by tabletist, safe to edit by hand\n";

/// `text` as a TOML basic string.
fn quote(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    out.push('"');
    for character in text.chars() {
        match character {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            control if control.is_control() => {
                out.push_str(&format!("\\u{:04X}", u32::from(control)));
            }
            other => out.push(other),
        }
    }
    out.push('"');
    out
}
```

In `impl Settings`:

```rust
    /// `key`'s value as TOML writes it. `None` for a key the file leaves
    /// out: a theme that is not set.
    fn literal(&self, key: Key) -> Option<String> {
        Some(match key {
            Key::PageSize => self.page_size.to_string(),
            Key::Timestamps => quote(self.timestamps.name()),
            Key::GroupDigits => self.group_digits.to_string(),
            Key::ValueTags => self.value_tags.to_string(),
            Key::ShowSystemSchemas => self.show_system_schemas.to_string(),
            Key::SqlLimit => self.sql_limit.to_string(),
            // TOML has no null: no timeout is no seconds.
            Key::SqlTimeoutSecs => self.sql_timeout_secs.unwrap_or(0).to_string(),
            Key::Theme => quote(self.custom_theme.as_deref()?),
        })
    }

    /// The file's text: the one form the app writes, the same every time.
    /// Tables in a fixed order, a table's keys aligned, and a note beside
    /// the keys that have one.
    pub fn to_toml(&self) -> String {
        let written: Vec<(Key, String)> = Key::ALL
            .into_iter()
            .filter_map(|key| Some((key, self.literal(key)?)))
            .collect();
        let mut out = String::from(HEADER);
        let mut open = None;
        for (key, literal) in &written {
            let (table, name) = key.path();
            if open != Some(table) {
                if open.is_some() {
                    out.push('\n');
                }
                out.push_str(&format!("[{table}]\n"));
                open = Some(table);
            }
            let width = written
                .iter()
                .filter(|(other, _)| other.path().0 == table)
                .map(|(other, _)| other.path().1.len())
                .max()
                .unwrap_or(0);
            out.push_str(&format!("{name:width$} = {literal}"));
            if let Some(note) = key.note() {
                out.push_str(&format!("  # {note}"));
            }
            out.push('\n');
        }
        out
    }
```

- [ ] **Step 4: Run the tests**

Run: `~/.cargo/bin/cargo test --locked -p tabletist --lib settings::`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/settings.rs
git commit -S -m "Write the settings as one canonical TOML text

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 4: Read the text, tolerantly

**Files:**
- Modify: `Cargo.toml`, `Cargo.lock`, `src/settings.rs`

- [ ] **Step 1: Add the dependency**

In `Cargo.toml`, after the `tempfile = "3"` line of `[dependencies]`:

```toml
# Reads settings.toml: `DeTable::parse_recoverable` keeps what it could read
# of a file with a bad line and says where each key and each error is. The
# app writes the file itself (src/settings.rs), so only the parser is built:
# the default `display` feature would bring in toml_writer.
toml = { version = "1.1", default-features = false, features = ["parse"] }
```

`toml` 1.1.6 is already in `Cargo.lock` (through `egui_kittest`). Add it to the app's own list by hand: in `Cargo.lock`, find the `[[package]]` whose `name = "tabletist"` and put ` "toml",` between ` "tokio",` and ` "uuid",` in its `dependencies`. Change nothing else in the lock.

Run: `~/.cargo/bin/cargo check --locked -p tabletist`
Expected: compiles. If cargo says the lock file needs to be updated, the hand edit is wrong: fix it, do not drop `--locked`.

Run: `git diff --stat Cargo.lock`
Expected: `1 file changed, 1 insertion(+)`.

- [ ] **Step 2: Write the failing tests**

Add to `mod tests` in `src/settings.rs`:

```rust
    #[test]
    fn what_is_written_reads_back_the_same() {
        let settings = Settings {
            page_size: 500,
            timestamps: Timestamps::Full,
            group_digits: true,
            value_tags: false,
            show_system_schemas: true,
            custom_theme: Some("My \"Nord\".json".into()),
            sql_limit: 100,
            sql_timeout_secs: None,
        };
        let text = settings.to_toml();
        let loaded = Settings::from_toml(&text);
        assert_eq!(loaded.settings, settings);
        assert_eq!(loaded.text, text);
        assert_eq!(loaded.invalid, Vec::<usize>::new());
        assert_eq!(loaded.source, Source::Toml);
        // Writing it again changes nothing.
        assert_eq!(loaded.settings.to_toml(), text);
    }

    #[test]
    fn each_key_is_found_on_its_line() {
        let loaded = Settings::from_toml(&Settings::default().to_toml());
        assert_eq!(
            loaded.lines,
            vec![
                (Key::PageSize, 3),
                (Key::Timestamps, 4),
                (Key::GroupDigits, 5),
                (Key::ValueTags, 6),
                (Key::ShowSystemSchemas, 9),
                (Key::SqlLimit, 12),
                (Key::SqlTimeoutSecs, 13),
            ]
        );
    }

    #[test]
    fn a_missing_key_keeps_its_default_and_an_unknown_one_is_ignored() {
        let loaded = Settings::from_toml(
            "[data]\npage_size = 100\nfrom_the_future = true\n\n[tomorrow]\nkey = 1\n",
        );
        assert_eq!(
            loaded.settings,
            Settings {
                page_size: 100,
                ..Settings::default()
            }
        );
        assert_eq!(loaded.invalid, Vec::<usize>::new());
        assert_eq!(loaded.lines, vec![(Key::PageSize, 2)]);
    }

    #[test]
    fn a_value_the_key_cannot_have_is_ignored_by_its_line() {
        let loaded = Settings::from_toml(
            "[data]\npage_size = \"lots\"\ntimestamps = \"minute\"\ngroup_digits = \"yes\"\nvalue_tags = false\n",
        );
        assert_eq!(loaded.invalid, vec![2, 3, 4]);
        assert_eq!(
            loaded.settings,
            Settings {
                value_tags: false,
                ..Settings::default()
            }
        );
        assert_eq!(loaded.lines, vec![(Key::ValueTags, 5)]);
    }

    #[test]
    fn a_line_that_is_not_toml_is_ignored_and_the_rest_applies() {
        let text = "[data]\npage_size = 100\nthis is not toml\ngroup_digits = true\n\n\
                    [editor]\nsql_limit = 100\n";
        let loaded = Settings::from_toml(text);
        assert_eq!(loaded.invalid, vec![3]);
        assert_eq!(loaded.settings.page_size, 100);
        // The lines and the tables under the bad one are still read.
        assert!(loaded.settings.group_digits);
        assert_eq!(loaded.settings.sql_limit, 100);
        assert_eq!(
            loaded.lines,
            vec![(Key::PageSize, 2), (Key::GroupDigits, 4), (Key::SqlLimit, 7)]
        );
        // What the pane will show is the file, not what was parsed.
        assert_eq!(loaded.text, text);
    }

    #[test]
    fn a_line_the_parser_rejects_is_not_applied_on_a_guess() {
        // Each of these the parser reports and still gives a value for.
        let loaded = Settings::from_toml(
            "[data]\ntimestamps = full\nvalue_tags = False\n\n[appearance]\ntheme = Nord.json\n",
        );
        assert_eq!(loaded.invalid, vec![2, 3, 6]);
        assert_eq!(loaded.settings, Settings::default());
        assert!(loaded.lines.is_empty());
    }

    #[test]
    fn a_key_given_twice_keeps_its_first_value() {
        let loaded = Settings::from_toml("[data]\npage_size = 100\npage_size = 500\n");
        assert_eq!(loaded.invalid, vec![3]);
        assert_eq!(loaded.settings.page_size, 100);
        assert_eq!(loaded.lines, vec![(Key::PageSize, 2)]);
    }

    #[test]
    fn numbers_out_of_range_are_clamped_and_not_invalid() {
        let loaded = Settings::from_toml(
            "[data]\npage_size = 0\n\n[editor]\nsql_limit = 999999999\nsql_timeout_secs = 0\n",
        );
        assert_eq!(loaded.invalid, Vec::<usize>::new());
        assert_eq!(loaded.settings.page_size, Settings::MIN_PAGE_SIZE);
        assert_eq!(loaded.settings.sql_limit, Settings::MAX_SQL_LIMIT);
        assert_eq!(loaded.settings.sql_timeout_secs, None);
        let loaded = Settings::from_toml("[data]\npage_size = -5\n");
        assert_eq!(loaded.invalid, Vec::<usize>::new());
        assert_eq!(loaded.settings.page_size, Settings::MIN_PAGE_SIZE);
        let loaded = Settings::from_toml("[data]\npage_size = 999999999\n");
        assert_eq!(loaded.settings.page_size, Settings::MAX_PAGE_SIZE);
        // The editor's own floor, and a timeout that is one.
        let loaded = Settings::from_toml("[editor]\nsql_limit = 0\nsql_timeout_secs = 1\n");
        assert_eq!(loaded.settings.sql_limit, 1);
        assert_eq!(loaded.settings.sql_timeout_secs, Some(1));
        let loaded = Settings::from_toml("[editor]\nsql_timeout_secs = 0\n");
        assert_eq!(loaded.settings.sql_timeout(), None);
    }

    #[test]
    fn an_empty_text_is_the_defaults() {
        let loaded = Settings::from_toml("");
        assert_eq!(loaded.settings, Settings::default());
        assert_eq!(loaded.invalid, Vec::<usize>::new());
        assert!(loaded.lines.is_empty());
    }

    #[test]
    fn settings_from_no_toml_file_carry_the_text_that_would_be_written() {
        let settings = Settings {
            page_size: 500,
            ..Settings::default()
        };
        let loaded = Loaded::of(settings.clone(), Source::Json);
        assert_eq!(loaded.source, Source::Json);
        assert_eq!(loaded.text, settings.to_toml());
        assert_eq!(loaded.lines.first(), Some(&(Key::PageSize, 3)));
        let defaults: Loaded = Settings::default().into();
        assert_eq!(defaults.source, Source::Defaults);
        assert_eq!(defaults.settings, Settings::default());
    }
```

- [ ] **Step 3: Run them to see them fail**

Run: `~/.cargo/bin/cargo test --locked -p tabletist --lib settings::`
Expected: does not compile, "no function `from_toml`", "cannot find type `Loaded`".

- [ ] **Step 4: Implement**

In `src/settings.rs`, above `impl Settings`:

```rust
/// Where the settings a start has came from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Source {
    /// `settings.toml`.
    Toml,
    /// An older version's `settings.json`: the TOML is still to be written.
    Json,
    /// No file.
    Defaults,
}

/// The settings as a file gave them, and what the file looked like.
#[derive(Clone, Debug, PartialEq)]
pub struct Loaded {
    pub settings: Settings,
    /// The file's text as read. With no TOML file, what would be written.
    pub text: String,
    /// The lines that were ignored, counted from 1, each once, in order.
    pub invalid: Vec<usize>,
    /// The line each key was read from, in the order of [`Key::ALL`].
    pub lines: Vec<(Key, usize)>,
    pub source: Source,
}

impl Loaded {
    /// Settings that came from no TOML file.
    pub fn of(settings: Settings, source: Source) -> Self {
        let text = settings.to_toml();
        let lines = Settings::from_toml(&text).lines;
        Self {
            settings,
            text,
            invalid: Vec::new(),
            lines,
            source,
        }
    }
}

impl From<Settings> for Loaded {
    fn from(settings: Settings) -> Self {
        Self::of(settings, Source::Defaults)
    }
}

/// The line `offset` is on in `text`, counted from 1.
fn line_of(text: &str, offset: usize) -> usize {
    let before = &text.as_bytes()[..offset.min(text.len())];
    before.iter().filter(|byte| **byte == b'\n').count() + 1
}

/// `text` with the lines numbered in `lines` (from 1) emptied. Their line
/// breaks stay, so every other line keeps its number.
fn without_lines(text: &str, lines: &[usize]) -> String {
    text.split_inclusive('\n')
        .enumerate()
        .map(|(index, line)| {
            if !lines.contains(&(index + 1)) {
                line
            } else if line.ends_with('\n') {
                "\n"
            } else {
                ""
            }
        })
        .collect()
}
```

In `impl Settings`:

```rust
    /// Takes `key`'s value from the file. False when it is no value the key
    /// can have: the setting stays as it was.
    fn take(&mut self, key: Key, value: &toml::de::DeValue<'_>) -> bool {
        // Any integer is a number here: `validated` brings it into range.
        let number = value
            .as_integer()
            .and_then(|integer| i64::from_str_radix(integer.as_str(), integer.radix()).ok())
            .and_then(|number| u32::try_from(number.clamp(0, i64::from(u32::MAX))).ok());
        let flag = value.as_bool();
        let text = value.as_str();
        match key {
            Key::PageSize => number.map(|number| self.page_size = number),
            Key::Timestamps => text
                .and_then(Timestamps::from_name)
                .map(|choice| self.timestamps = choice),
            Key::GroupDigits => flag.map(|flag| self.group_digits = flag),
            Key::ValueTags => flag.map(|flag| self.value_tags = flag),
            Key::ShowSystemSchemas => flag.map(|flag| self.show_system_schemas = flag),
            Key::SqlLimit => number.map(|number| self.sql_limit = number),
            Key::SqlTimeoutSecs => number.map(|number| self.sql_timeout_secs = Some(number)),
            Key::Theme => text.map(|name| self.custom_theme = Some(name.to_owned())),
        }
        .is_some()
    }

    /// Reads the file's text, keeping everything it can. A line that is not
    /// TOML, and a value its key cannot have, are ignored and remembered by
    /// line; the key keeps its default. A key or a table this version does
    /// not know is ignored without a word: a newer one may have written it.
    pub fn from_toml(text: &str) -> Loaded {
        // The parser recovers from a bad line in two ways that are not
        // "ignored": it stops reading at some (a key with no `=`), and
        // reads past others with a guess at the value (`full` unquoted).
        // So a line it rejects is emptied and the text read again, until
        // it rejects nothing more.
        let mut working = text.to_owned();
        let mut invalid: Vec<usize> = Vec::new();
        loop {
            let (_, errors) = toml::de::DeTable::parse_recoverable(&working);
            let mut rejected = Vec::new();
            for error in &errors {
                match error.span() {
                    Some(span) => rejected.push(line_of(&working, span.start)),
                    None => log::warn!("settings: {}", error.message()),
                }
            }
            // A line already emptied has nothing left to reject: an error
            // that still names one (the end of the text) ends the loop.
            rejected.retain(|line| !invalid.contains(line));
            rejected.sort_unstable();
            rejected.dedup();
            if rejected.is_empty() {
                break;
            }
            working = without_lines(&working, &rejected);
            invalid.extend(rejected);
        }
        let (document, _) = toml::de::DeTable::parse_recoverable(&working);
        let document = document.into_inner();
        let mut settings = Settings::default();
        let mut lines = Vec::new();
        for key in Key::ALL {
            let (table, name) = key.path();
            let Some((found, value)) = document
                .get(table)
                .and_then(|table| table.get_ref().as_table())
                .and_then(|table| table.get_key_value(name))
            else {
                continue;
            };
            // Emptied lines kept their breaks: a line has the same number
            // in `working` as in `text`.
            let line = line_of(&working, found.span().start);
            if settings.take(key, value.get_ref()) {
                lines.push((key, line));
            } else {
                invalid.push(line);
            }
        }
        invalid.sort_unstable();
        invalid.dedup();
        Loaded {
            settings: settings.validated(),
            text: text.to_owned(),
            invalid,
            lines,
            source: Source::Toml,
        }
    }
```

Notes for the implementer:
- `toml::de::DeTable` is `Map<Spanned<DeString>, Spanned<DeValue>>`. `get` and `get_key_value` take `&str` (`Spanned<Cow<str>>: Borrow<str>`). `Spanned::span()` is a byte range, `get_ref()` the value. `DeValue::as_integer()` gives a `DeInteger` whose `as_str()` and `radix()` feed `from_str_radix`. `Error::span()` is an `Option`.
- Why the loop: `parse_recoverable` gives up on the rest of the document after a line with a key and no `=` (`toml`'s `de/parser/document.rs` breaks out of its event loop), and for `timestamps = full` or `value_tags = False` it reports an error and still returns a value. Emptying each rejected line and parsing again makes "ignored" true in both cases. The loop ends because every pass empties at least one more line. A value that never closes is the one case that costs more than its own line: for a multi-line string left open (`theme = """Nord`) the parser blames the end of the text, a place no line has, and still returns a guess. An error at or past the end of the text therefore blames the last line that is not blank, so the lines under the opening one are emptied from the bottom up until it goes, and the guess is never applied (test: `a_string_left_open_is_not_applied_and_costs_the_lines_under_it`). An unclosed `[` or `{` costs its own line when only blank lines and comments follow, and the lines under it otherwise; they are all marked, and nothing wrong is applied, since no key takes an array or a table. Blank, here, is TOML's blank (spaces and tabs): a stray carriage return or a no-break space on the last line is the bad line itself, not one to skip. A table header the parser rejects also costs the keys under it, without a mark for them: they are read into the table above, where this version does not know them.
- If one of the three bad-line tests reports other line numbers than it expects, print what `parse_recoverable` returned for that text (the errors' spans and messages) before changing anything. Fix the reader so the tests' claims hold (one bad line costs that line and nothing else); do not weaken a test without saying why.

- [ ] **Step 5: Run the tests**

Run: `~/.cargo/bin/cargo test --locked -p tabletist --lib settings::`
Expected: PASS.

Run: `~/.cargo/bin/cargo clippy --locked --workspace --all-targets -- -D warnings`
Expected: no warnings.

- [ ] **Step 6: Commit**

```bash
git add Cargo.toml Cargo.lock src/settings.rs
git commit -S -m "Read the settings text and keep what it can of a damaged one

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 5: The file is `settings.toml`

> **After review.** The listings in Tasks 4 and 5 are as first written. What was built differs where the reviews found something: an error at the end of the text blames the last line TOML does not call blank (`last_line`); `Loaded::of` brings the settings into range itself, so the JSON arm of `load` does not; a `settings.toml` that cannot be read is kept aside like one that is not UTF-8; and `take`'s comment says an integer an `i64` cannot hold is no value. `src/settings.rs` is the record.

**Files:**
- Modify: `src/paths.rs`, `src/util.rs`, `src/settings.rs`, `src/backend.rs` (test near line 3090), `src/entrypoint.rs:111`, `src/app.rs`

After this task `Settings::load` takes the directories and returns a `Loaded`, and `save` writes TOML.

- [ ] **Step 1: Write the failing tests**

In `src/paths.rs`, in `files_live_in_their_directories`, replace the `settings_file` assertion with:

```rust
        assert_eq!(
            dirs.settings_file(),
            PathBuf::from("/root/config/settings.toml")
        );
        assert_eq!(
            dirs.legacy_settings_file(),
            PathBuf::from("/root/config/settings.json")
        );
```

In `src/settings.rs`'s `mod tests`, **delete** these tests (their behaviour is covered by Task 4's TOML tests and the ones below): `an_older_file_with_a_version_gets_the_new_defaults`, `settings_round_trip`, `older_files_with_missing_and_unknown_fields_still_load`, `older_files_get_the_sql_defaults`, `the_sql_limit_is_clamped`, `the_sql_limit_has_its_own_ceiling`, `a_timeout_of_no_seconds_loads_as_no_timeout`, `no_timeout_is_written_as_null_and_the_old_keys_stay`, `damaged_settings_are_kept_aside_and_defaults_used`, `page_size_is_clamped_to_a_sane_range`. Then add:

```rust
    fn dirs() -> (crate::paths::AppDirs, tempfile::TempDir) {
        let root = tempfile::tempdir().unwrap();
        let dirs = crate::paths::AppDirs::at(root.path());
        dirs.ensure().unwrap();
        (dirs, root)
    }

    #[test]
    fn no_file_gives_the_defaults_and_writes_nothing() {
        let (dirs, _root) = dirs();
        let loaded = Settings::load(&dirs);
        assert_eq!(loaded.settings, Settings::default());
        assert_eq!(loaded.source, Source::Defaults);
        assert_eq!(loaded.text, Settings::default().to_toml());
        assert!(!dirs.settings_file().exists());
    }

    #[test]
    fn what_is_saved_is_loaded() {
        let (dirs, _root) = dirs();
        let settings = Settings {
            page_size: 500,
            group_digits: true,
            sql_timeout_secs: None,
            ..Settings::default()
        };
        settings.save(&dirs.settings_file()).unwrap();
        let loaded = Settings::load(&dirs);
        assert_eq!(loaded.settings, settings);
        assert_eq!(loaded.source, Source::Toml);
        assert_eq!(loaded.text, settings.to_toml());
    }

    #[test]
    fn only_the_old_json_is_read_and_left_as_it_was() {
        let (dirs, _root) = dirs();
        let json = br#"{"version": 1, "page_size": 100, "show_system_schemas": true,
            "custom_theme": "Nord.json", "sql_limit": 100, "sql_timeout_secs": null,
            "from_the_future": true}"#;
        std::fs::write(dirs.legacy_settings_file(), json).unwrap();
        let loaded = Settings::load(&dirs);
        let expected = Settings {
            page_size: 100,
            show_system_schemas: true,
            custom_theme: Some("Nord.json".into()),
            sql_limit: 100,
            sql_timeout_secs: None,
            ..Settings::default()
        };
        assert_eq!(loaded.settings, expected);
        assert_eq!(loaded.source, Source::Json);
        assert_eq!(loaded.text, expected.to_toml());
        // Reading writes nothing, and an older Tabletist still finds its file.
        assert!(!dirs.settings_file().exists());
        assert_eq!(std::fs::read(dirs.legacy_settings_file()).unwrap(), json);
    }

    #[test]
    fn the_old_json_is_brought_into_range_as_before() {
        let (dirs, _root) = dirs();
        std::fs::write(
            dirs.legacy_settings_file(),
            br#"{"page_size": 0, "sql_limit": 999999999, "sql_timeout_secs": 0}"#,
        )
        .unwrap();
        let settings = Settings::load(&dirs).settings;
        assert_eq!(settings.page_size, Settings::MIN_PAGE_SIZE);
        assert_eq!(settings.sql_limit, Settings::MAX_SQL_LIMIT);
        assert_eq!(settings.sql_timeout_secs, None);
        assert!(
            Settings::SQL_LIMITS
                .iter()
                .all(|limit| (1..=Settings::MAX_SQL_LIMIT).contains(limit)),
            "every choice of the Limit menu loads as it was saved"
        );
    }

    #[test]
    fn with_both_files_the_toml_is_the_one_read() {
        let (dirs, _root) = dirs();
        std::fs::write(dirs.legacy_settings_file(), br#"{"page_size": 100}"#).unwrap();
        std::fs::write(dirs.settings_file(), "[data]\npage_size = 500\n").unwrap();
        let loaded = Settings::load(&dirs);
        assert_eq!(loaded.settings.page_size, 500);
        assert_eq!(loaded.source, Source::Toml);
        assert_eq!(loaded.text, "[data]\npage_size = 500\n");
    }

    #[test]
    fn a_damaged_old_json_is_kept_aside_and_the_defaults_used() {
        let (dirs, _root) = dirs();
        std::fs::write(dirs.legacy_settings_file(), br#"{"page_size": "lots"}"#).unwrap();
        let loaded = Settings::load(&dirs);
        assert_eq!(loaded.settings, Settings::default());
        assert_eq!(loaded.source, Source::Json);
        assert!(dirs.config.join("settings.json.bad").exists());
    }

    #[test]
    fn a_file_that_is_not_text_is_kept_aside_and_the_defaults_used() {
        let (dirs, _root) = dirs();
        std::fs::write(dirs.settings_file(), [0xff, 0xfe, 0x00]).unwrap();
        let loaded = Settings::load(&dirs);
        assert_eq!(loaded.settings, Settings::default());
        assert_eq!(loaded.source, Source::Defaults);
        assert!(dirs.config.join("settings.toml.bad").exists());
        assert!(!dirs.settings_file().exists());
    }

    #[test]
    fn a_file_with_a_bad_line_is_left_as_the_user_wrote_it() {
        let (dirs, _root) = dirs();
        let text = "[data]\npage_size = 100\ngroup_digits = \"yes\"\n";
        std::fs::write(dirs.settings_file(), text).unwrap();
        let loaded = Settings::load(&dirs);
        assert_eq!(loaded.invalid, vec![3]);
        assert_eq!(loaded.settings.page_size, 100);
        assert_eq!(std::fs::read_to_string(dirs.settings_file()).unwrap(), text);
    }
```

In `src/backend.rs`, in `settings_are_saved_as_a_state_file`, change the path to `"settings.toml"` and the last assertion to:

```rust
        let text = std::fs::read_to_string(&path).unwrap();
        assert_eq!(text, settings.to_toml());
        assert_eq!(crate::settings::Settings::from_toml(&text).settings, settings);
```

- [ ] **Step 2: Run them to see them fail**

Run: `~/.cargo/bin/cargo test --locked -p tabletist --lib settings::`
Expected: does not compile, "no method named `legacy_settings_file`", mismatched types for `Settings::load(&dirs)`.

- [ ] **Step 3: Implement**

`src/paths.rs`, replace `settings_file`:

```rust
    pub fn settings_file(&self) -> PathBuf {
        self.config.join("settings.toml")
    }

    /// Where versions before the TOML file kept the settings. Read once,
    /// when there is no `settings.toml`, and never written.
    pub fn legacy_settings_file(&self) -> PathBuf {
        self.config.join("settings.json")
    }
```

`src/util.rs`: change `fn keep_aside(` to `pub fn keep_aside(`.

`src/settings.rs`:

- The module comment becomes `//! User settings, stored as `settings.toml` in the config directory.`
- The doc comment of `Settings` becomes:

```rust
/// Everything the user can set. A key the file lacks takes its default and
/// one this version does not know is ignored, so older and newer files both
/// load. The serde derive reads the `settings.json` of versions before the
/// TOML file.
```

- Change the derive of `Settings` to `#[derive(Clone, Debug, PartialEq, Deserialize)]` and of `Timestamps` to `#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize)]`, and the import to `use serde::Deserialize;`.
- Add `use crate::paths::AppDirs;`.
- Replace `load` and `save` in `impl Settings`:

```rust
    /// The settings a start has: from `settings.toml`, else from an older
    /// version's `settings.json`, else the defaults. It only reads; what
    /// came from the JSON is written as TOML by the app (see [`Source`]).
    pub fn load(dirs: &AppDirs) -> Loaded {
        let path = dirs.settings_file();
        match std::fs::read(&path) {
            Ok(bytes) => match String::from_utf8(bytes) {
                Ok(text) => {
                    let loaded = Self::from_toml(&text);
                    // Until a window shows them, the log is where a typo is told.
                    for line in &loaded.invalid {
                        log::warn!(
                            "{}: line {line} could not be read and is ignored",
                            path.display()
                        );
                    }
                    loaded
                }
                Err(error) => {
                    // Not text at all: kept aside like a damaged JSON file,
                    // so the next save cannot replace it.
                    crate::util::keep_aside(&path, &error.to_string());
                    Self::default().into()
                }
            },
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                let legacy = dirs.legacy_settings_file();
                if legacy.exists() {
                    let settings = crate::util::load_json::<Settings>(&legacy).validated();
                    Loaded::of(settings, Source::Json)
                } else {
                    Self::default().into()
                }
            }
            Err(error) => {
                log::warn!("could not read {} ({error}); using defaults", path.display());
                Self::default().into()
            }
        }
    }

    /// Writes the canonical text, atomically.
    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        crate::util::write_atomic(path, self.to_toml().as_bytes())
    }
```

`src/entrypoint.rs:111`: `let settings = Settings::load(&dirs).settings;` (the `Loaded` reaches the app in Task 6).

`src/app.rs`: the comment of `save_settings` stays; nothing else changes here in this task.

- [ ] **Step 4: Run the tests**

Cargo takes one filter at a time:

```bash
~/.cargo/bin/cargo test --locked -p tabletist --lib settings::
~/.cargo/bin/cargo test --locked -p tabletist --lib paths::
~/.cargo/bin/cargo test --locked -p tabletist --lib settings_are_saved_as_a_state_file
```

Expected: PASS.

Run: `~/.cargo/bin/cargo clippy --locked --workspace --all-targets -- -D warnings`
Expected: no warnings (no unused `Serialize` import, no unused `save_json` complaint: `save_json` is still used by the connection and host key stores).

- [ ] **Step 5: Commit**

```bash
git add src/paths.rs src/util.rs src/settings.rs src/backend.rs src/entrypoint.rs
git commit -S -m "Keep the settings in settings.toml

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 6: The app writes the TOML once after reading the JSON

**Files:**
- Modify: `src/app.rs` (`App::new` near line 102, tests near line 3225 and 7876), `src/entrypoint.rs:111-123`, `src/testing.rs:668`

- [ ] **Step 1: Write the failing test**

In `src/app.rs`'s `mod tests`, after `an_older_connections_file_is_saved_upgraded_by_the_backend`:

```rust
    #[test]
    fn settings_read_from_the_old_json_are_written_as_toml_once() {
        use crate::settings::{Loaded, Source};
        let dir = tempfile::tempdir().unwrap();
        let dirs = AppDirs::at(dir.path());
        let path = dirs.settings_file();
        let saves = |app: &App| {
            app.backend
                .sent
                .iter()
                .filter(|command| {
                    matches!(
                        command,
                        Command::Save { path: to, file: StateFile::Settings(_) } if *to == path
                    )
                })
                .count()
        };
        let settings = Settings {
            page_size: 100,
            ..Settings::default()
        };
        let app = App::new(
            dirs.clone(),
            Loaded::of(settings.clone(), Source::Json),
            Backend::recording(),
        );
        assert_eq!(app.settings, settings);
        assert_eq!(saves(&app), 1);
        assert!(!path.exists(), "the UI thread writes nothing");
        // A TOML file, or none, is not written at a start.
        for source in [Source::Toml, Source::Defaults] {
            let app = App::new(
                dirs.clone(),
                Loaded::of(settings.clone(), source),
                Backend::recording(),
            );
            assert_eq!(saves(&app), 0, "{source:?}");
        }
    }
```

- [ ] **Step 2: Run it to see it fail**

Run: `~/.cargo/bin/cargo test --locked -p tabletist --lib settings_read_from_the_old_json`
Expected: does not compile, mismatched types (`App::new` takes `Settings`).

- [ ] **Step 3: Implement**

`src/app.rs`:

- Change the import to `use crate::settings::{Loaded, Settings, Source};`.
- `App::new`:

```rust
    pub fn new(dirs: AppDirs, loaded: Loaded, backend: Backend) -> Self {
        let Loaded {
            settings, source, ..
        } = loaded;
```

  and, beside the connections upgrade at the end of `new`:

```rust
        // An older file is written in this version once, off the UI thread.
        if upgraded {
            app.save_connections();
        }
        // So are settings that were read from the old settings.json.
        if source == Source::Json {
            app.save_settings();
        }
        app
```

- Every other caller passes a `Loaded`. In `src/app.rs` tests (near lines 3227, 3241, 3262, 7876) and `src/testing.rs:668`, `Settings::default()` becomes `Settings::default().into()`.
- `src/entrypoint.rs`: line 111 becomes `let settings = Settings::load(&dirs);` (the name stays; it is now the `Loaded` that `App::new(dirs, settings, backend)` takes).

If the `use` in the test above shadows the new top-level import, drop the test's own `use` line.

- [ ] **Step 4: Run the tests**

Run: `~/.cargo/bin/cargo test --locked -p tabletist --lib app::tests::`
Expected: PASS.

Run: `~/.cargo/bin/cargo check --locked --workspace --all-targets`
Expected: no errors.

- [ ] **Step 5: Commit**

```bash
git add src/app.rs src/entrypoint.rs src/testing.rs
git commit -S -m "Write settings.toml once from the old settings.json

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 7: A workspace starts with the timestamps option

**Files:**
- Modify: `src/app.rs` (`open_workspace` near line 1360)
- Test: `src/ui/mod.rs` (after `the_grid_shows_timestamps_to_the_second_until_asked_for_more`, near line 376)

- [ ] **Step 1: Write the failing test**

```rust
    #[test]
    fn a_workspace_starts_with_the_timestamps_the_settings_ask_for() {
        let mut harness = Harness::new();
        harness.app.settings.timestamps = crate::settings::Timestamps::Full;
        let tab = harness.connect_fake();
        assert!(harness.app.workspace(tab).unwrap().full_precision);
        harness.click("users");
        let mut page = crate::testing::page(1, false);
        page.columns[1].kind = tabletist_db::ValueKind::Temporal;
        page.rows[0][1] = tabletist_db::Value::Text("2026-01-12 09:14:03.482915".into());
        harness.answer_rows(page);
        harness.settle();
        assert!(
            harness
                .painted_color("2026-01-12 09:14:03.482915")
                .is_some()
        );
        // The grid's own link still switches this workspace.
        harness
            .app
            .apply(crate::model::Action::ToggleFullPrecision(tab));
        assert!(!harness.app.workspace(tab).unwrap().full_precision);
    }
```

- [ ] **Step 2: Run it to see it fail**

Run: `~/.cargo/bin/cargo test --locked -p tabletist --lib a_workspace_starts_with_the_timestamps`
Expected: FAIL at the first `assert!` (`full_precision` is false).

- [ ] **Step 3: Implement**

`src/app.rs`, `open_workspace`:

```rust
    fn open_workspace(&mut self, tab: ConnTabId, saved: SavedConnection, typed: Secrets) {
        let session = SessionId(self.next_id());
        let request = RequestId(self.next_id());
        // Where the option puts it; the grid's link switches it from there.
        let full_precision = self.settings.timestamps == crate::settings::Timestamps::Full;
        let Some(entry) = self.tabs.iter_mut().find(|t| t.id == tab) else {
            return;
        };
        let mut workspace = Workspace::new(session, request, saved, typed);
        workspace.full_precision = full_precision;
        entry.content = ConnTabContent::Workspace(Box::new(workspace));
    }
```

- [ ] **Step 4: Run the tests**

Run: `~/.cargo/bin/cargo test --locked -p tabletist --lib timestamps`
Expected: PASS (the new test and `the_grid_shows_timestamps_to_the_second_until_asked_for_more`).

- [ ] **Step 5: Commit**

```bash
git add src/app.rs src/ui/mod.rs
git commit -S -m "Start a workspace with the timestamps the settings ask for

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 8: Numbers in threes

**Files:**
- Modify: `src/ui/data_view.rs` (`show` near line 1055, `cell` and `plain_cell` near lines 1313 to 1415, tests near lines 1631, 1660, 1703, 1762), `src/ui/sql_results.rs` (`Place` near line 193, `draw` near line 215, `results` near lines 1044 and 1155)
- Test: `src/ui/data_view.rs`, `src/ui/mod.rs`

`cell` and `plain_cell` take one more thing to know. Their last parameter `full_precision: bool` becomes `shown: Shown`.

- [ ] **Step 1: Write the failing tests**

In `src/ui/data_view.rs`'s `mod tests`:

```rust
    #[test]
    fn a_number_is_grouped_only_when_asked() {
        let ctx = context();
        let look = Look::macos();
        let text = |value: &Value, kind, shown| {
            plain_cell(&ctx, value, &meta("", kind), &look, shown)
                .text
                .into_owned()
        };
        let grouped = Shown {
            grouped: true,
            ..Shown::default()
        };
        let amount = Value::Text("1240.50".into());
        assert_eq!(text(&Value::Int(1_234_567), ValueKind::Numeric, grouped), "1,234,567");
        assert_eq!(text(&amount, ValueKind::Numeric, grouped), "1,240.50");
        assert_eq!(text(&Value::Int(1_234_567), ValueKind::Numeric, Shown::default()), "1234567");
        // Digits that are not a number's: a text column's stay as they are.
        assert_eq!(text(&Value::Text("1234567".into()), ValueKind::Text, grouped), "1234567");
    }

    #[test]
    fn a_key_is_a_column_of_the_primary_key_or_of_a_foreign_one() {
        let structure = tabletist_db::Structure {
            primary_key: vec!["id".into()],
            foreign_keys: vec![tabletist_db::ForeignKeyInfo {
                columns: vec!["publisher_id".into()],
                ..Default::default()
            }],
            ..Default::default()
        };
        assert!(is_key("id", Some(&structure)));
        assert!(is_key("publisher_id", Some(&structure)));
        assert!(!is_key("price", Some(&structure)));
        // Not described yet: nothing is known to be a key.
        assert!(!is_key("id", None));
    }
```

If `ForeignKeyInfo` has no `Default`, build it with all its fields instead (see `crates/tabletist-db/src/catalog.rs:66`); do not add a derive to the database crate for a test.

In `src/ui/mod.rs`, after the test of Task 7:

```rust
    #[test]
    fn numbers_are_grouped_when_the_settings_say_so_but_keys_never_are() {
        let mut harness = Harness::new();
        harness.app.settings.group_digits = true;
        let tab = harness.connect_fake();
        harness.click("users");
        let mut page = crate::testing::page(1, false);
        page.rows[0][0] = tabletist_db::Value::Int(1_234_567);
        page.columns[1].name = "amount".into();
        page.columns[1].kind = tabletist_db::ValueKind::Numeric;
        page.rows[0][1] = tabletist_db::Value::Text("1240.50".into());
        harness.answer_rows(page);
        harness.settle();
        // Not described yet: no column is known to be a key.
        assert!(harness.painted_color("1,234,567").is_some());
        assert!(harness.painted_color("1,240.50").is_some());
        harness.answer_structure(tabletist_db::Structure {
            primary_key: vec!["id".into()],
            ..Default::default()
        });
        harness.settle();
        assert!(harness.painted_color("1234567").is_some());
        assert!(harness.painted_color("1,234,567").is_none());
        assert!(harness.painted_color("1,240.50").is_some());
        // A copy gives the value as it is.
        let object_tab = harness.app.workspace(tab).unwrap().active_tab.unwrap();
        harness.app.apply(crate::model::Action::SelectCell {
            tab,
            id: object_tab,
            cell: crate::model::CellPos { row: 0, col: 1 },
        });
        harness.app.workspace_mut(tab).unwrap().pane = crate::model::Pane::Grid;
        harness.copy(false);
        assert_eq!(harness.copied.as_deref(), Some("1240.50"));
    }

    #[test]
    fn numbers_are_plain_when_the_settings_do_not_group() {
        let mut harness = Harness::new();
        harness.connect_fake();
        harness.click("users");
        let mut page = crate::testing::page(1, false);
        page.rows[0][0] = tabletist_db::Value::Int(1_234_567);
        harness.answer_rows(page);
        harness.settle();
        assert!(harness.painted_color("1234567").is_some());
    }
```

Add, beside the existing SQL result tests of `src/ui/mod.rs` (they use `with_sql_outcome`, near line 1755):

```rust
    #[test]
    fn a_result_groups_every_number_when_the_settings_say_so() {
        let mut harness = Harness::new();
        harness.app.settings.group_digits = true;
        let tab = harness.connect_fake();
        let mut page = crate::testing::page(1, false);
        page.rows[0][0] = tabletist_db::Value::Int(1_234_567);
        with_sql_outcome(
            &mut harness,
            tab,
            tabletist_db::StatementOutcome::Rows {
                columns: page.columns,
                rows: page.rows,
                truncated: false,
            },
        );
        harness.settle();
        // No structure to name a key: `id` is grouped here.
        assert!(harness.painted_color("1,234,567").is_some());
    }
```

If `harness.copy(false)` does not copy the selected cell with the data grid focused, read how the existing copy tests set it up (`grep -n "harness.copy(" src/`) and follow them; keep the assertion.

- [ ] **Step 2: Run them to see them fail**

Run: `~/.cargo/bin/cargo test --locked -p tabletist --lib a_number_is_grouped_only_when_asked`
Expected: does not compile, "cannot find struct `Shown`", "cannot find function `is_key`".

- [ ] **Step 3: Implement**

`src/ui/data_view.rs`, above `cell`:

```rust
/// How a cell writes its value out.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Shown {
    /// Timestamps with the fraction the server sent.
    pub full_precision: bool,
    /// A number's integer digits in threes.
    pub grouped: bool,
}

/// Whether `name` is a column of the table's primary key or of one of its
/// foreign keys: a number that names a row, which grouping would only make
/// harder to read. Nothing is, until the table is described.
fn is_key(name: &str, structure: Option<&tabletist_db::Structure>) -> bool {
    structure.is_some_and(|structure| {
        structure.primary_key.iter().any(|column| column == name)
            || structure
                .foreign_keys
                .iter()
                .any(|foreign| foreign.columns.iter().any(|column| column == name))
    })
}
```

`cell`: the parameter `full_precision: bool` becomes `shown: Shown`, and its last line `plain_cell(ctx, value, column, look, shown)`.

`plain_cell`: the parameter becomes `shown: Shown`; its doc comment's "a timestamp to the second unless `full_precision`" becomes "a timestamp to the second and a number in threes as `shown` asks"; and its ending becomes:

```rust
    let text = if kind == ValueKind::Temporal && !shown.full_precision {
        match format::to_the_second(&text) {
            std::borrow::Cow::Borrowed(_) => text,
            std::borrow::Cow::Owned(short) => short.into(),
        }
    } else if kind == ValueKind::Numeric && shown.grouped {
        match format::group_number(&text) {
            std::borrow::Cow::Borrowed(_) => text,
            std::borrow::Cow::Owned(grouped) => grouped.into(),
        }
    } else {
        text
    };
    styled(text, Style::Plain)
```

`show` in `src/ui/data_view.rs`: read the option beside the look (`let group_digits = app.settings.group_digits;` with `locale`, `palette`, `look` at the top), and where the tags are made:

```rust
        // Grouping is for amounts: a key reads as the name it is.
        let shown: Vec<Shown> = page
            .columns
            .iter()
            .map(|column| Shown {
                full_precision,
                grouped: group_digits && !is_key(&column.name, structure),
            })
            .collect();
```

and in the grid's closure the last argument of `cell` is `shown[col]`.

Existing tests in `src/ui/data_view.rs` that pass a `bool` to `cell` or `plain_cell` (near lines 1631, 1660, 1703, 1762): `false` becomes `Shown::default()`, and the one that passes a variable `full` passes `Shown { full_precision: full, ..Shown::default() }`.

`src/ui/sql_results.rs`:

- `Place` gains, after `full_precision`:

```rust
    /// Numbers in threes, as the settings ask.
    grouped: bool,
```

- `draw` fills it: `grouped: app.settings.group_digits,`.
- `results` takes `grouped` out of `*place` beside `full_precision`, and the grid's closure passes:

```rust
                data_view::Shown {
                    full_precision,
                    // A result has no key to leave alone.
                    grouped,
                },
```

  as the last argument of `data_view::cell`.

Any other place that builds a `Place` or calls `data_view::cell` is found by the compiler; give each the same treatment.

- [ ] **Step 4: Run the tests**

```bash
~/.cargo/bin/cargo test --locked -p tabletist --lib ui::data_view::
~/.cargo/bin/cargo test --locked -p tabletist --lib numbers_are_
~/.cargo/bin/cargo test --locked -p tabletist --lib a_result_groups_every_number
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/ui/data_view.rs src/ui/sql_results.rs src/ui/mod.rs
git commit -S -m "Group the numbers in a grid when the settings ask for it

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 9: Value tags can be turned off

**Files:**
- Modify: `src/ui/value_tags.rs`, `src/ui/data_view.rs` (near line 1103), `src/ui/row_panel.rs` (near lines 233 and 345), `src/ui/sql_results.rs` (`Place`, `draw`, near line 1141)
- Test: `src/ui/value_tags.rs`, `src/ui/mod.rs`

- [ ] **Step 1: Write the failing tests**

In `src/ui/value_tags.rs`'s `mod tests`:

```rust
    #[test]
    fn tags_that_are_turned_off_are_none() {
        let allowed = ["print".to_owned(), "ebook".to_owned()];
        assert_eq!(Tags::Values(&allowed).when(true), Tags::Values(&allowed));
        assert_eq!(Tags::Values(&allowed).when(false), Tags::None);
        assert_eq!(Tags::Bool.when(false), Tags::None);
    }
```

In `src/ui/mod.rs`, after the tests of Task 8:

```rust
    /// The colours a table's grid paints `true` and a plain text cell in,
    /// with the row selected so the row panel shows the value too: every
    /// `true` the frame painted, then the plain cell.
    fn true_and_plain(value_tags: bool) -> (Vec<egui::Color32>, egui::Color32) {
        let mut harness = Harness::new();
        harness.app.settings.value_tags = value_tags;
        let tab = harness.connect_fake();
        harness.click("users");
        let mut page = crate::testing::page(1, false);
        page.columns[2].name = "active".into();
        page.columns[2].kind = tabletist_db::ValueKind::Bool;
        page.rows[0][2] = tabletist_db::Value::Bool(true);
        harness.answer_rows(page);
        let object_tab = harness.app.workspace(tab).unwrap().active_tab.unwrap();
        harness.app.apply(crate::model::Action::SelectCell {
            tab,
            id: object_tab,
            cell: crate::model::CellPos { row: 0, col: 1 },
        });
        harness.settle();
        let trues = harness
            .painted
            .iter()
            .filter(|(text, _)| text == "true")
            .map(|(_, color)| *color)
            .collect();
        let plain = harness
            .painted_color("user1@example.com")
            .expect("the email cell");
        (trues, plain)
    }

    #[test]
    fn value_tags_colour_a_boolean_until_the_settings_turn_them_off() {
        let (on, plain) = true_and_plain(true);
        assert!(!on.is_empty());
        assert!(on.iter().any(|color| *color != plain), "a tag has its colour");
        let (off, plain) = true_and_plain(false);
        // The grid's cell and the row panel's field.
        assert!(off.len() >= 2, "{off:?}");
        assert!(off.iter().all(|color| *color == plain), "{off:?}");
    }

    #[test]
    fn a_result_draws_a_boolean_plain_when_value_tags_are_off() {
        let painted = |value_tags: bool| {
            let mut harness = Harness::new();
            harness.app.settings.value_tags = value_tags;
            let tab = harness.connect_fake();
            let mut page = crate::testing::page(1, false);
            page.columns[2].kind = tabletist_db::ValueKind::Bool;
            page.rows[0][2] = tabletist_db::Value::Bool(true);
            with_sql_outcome(
                &mut harness,
                tab,
                tabletist_db::StatementOutcome::Rows {
                    columns: page.columns,
                    rows: page.rows,
                    truncated: false,
                },
            );
            harness.settle();
            (
                harness.painted_color("true").expect("the boolean cell"),
                harness.painted_color("user1@example.com").expect("the email cell"),
            )
        };
        let (tag, plain) = painted(true);
        assert_ne!(tag, plain);
        let (flat, plain) = painted(false);
        assert_eq!(flat, plain);
    }
```

These compare colours the frame painted with each other, never with a colour of the design. If the selected row paints its cells in a selection colour, so that the plain email cell and a plain `true` differ for a reason that is not a tag, select nothing in the grid test (drop the `SelectCell`) and assert on the grid alone (`off.len() >= 1`); then cover the row panel by asserting that with tags off the panel's `true` has the colour of the panel's email value. Read what `harness.painted` holds for the frame before deciding, and say in your report which form you kept.

- [ ] **Step 2: Run them to see them fail**

Run: `~/.cargo/bin/cargo test --locked -p tabletist --lib tags_that_are_turned_off`
Expected: does not compile, "no method named `when`".

- [ ] **Step 3: Implement**

`src/ui/value_tags.rs`, in `impl<'a> Tags<'a>`:

```rust
    /// These tags when the settings draw value tags (`on`), else none.
    pub fn when(self, on: bool) -> Self {
        if on { self } else { Self::None }
    }
```

`src/ui/data_view.rs`, `show`: read `let value_tags = app.settings.value_tags;` at the top, and:

```rust
        let tags: Vec<_> = crate::ui::value_tags::Tags::of_page(page, structure)
            .into_iter()
            .map(|tags| tags.when(value_tags))
            .collect();
```

`src/ui/row_panel.rs`, `show`: read `let value_tags = app.settings.value_tags;` with `locale`, `palette`, `look` at the top, and:

```rust
                .map(|column| crate::ui::value_tags::Tags::of(column, structure).when(value_tags))
```

`src/ui/sql_results.rs`: `Place` gains

```rust
    /// Booleans as tags, as the settings ask.
    value_tags: bool,
```

`draw` fills it with `app.settings.value_tags`, `results` takes it out of `*place`, and:

```rust
        .map(|column| Tags::of(column, None).when(value_tags))
```

- [ ] **Step 4: Run the tests**

```bash
~/.cargo/bin/cargo test --locked -p tabletist --lib ui::value_tags::
~/.cargo/bin/cargo test --locked -p tabletist --lib value_tags_colour_a_boolean
~/.cargo/bin/cargo test --locked -p tabletist --lib a_result_draws_a_boolean_plain
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/ui/value_tags.rs src/ui/data_view.rs src/ui/row_panel.rs src/ui/sql_results.rs src/ui/mod.rs
git commit -S -m "Let the settings turn value tags off

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 10: The whole suite, and the file by hand

**Files:** none, unless a check fails.

- [ ] **Step 1: Run every check of `AGENTS.md`**

```bash
~/.cargo/bin/cargo fmt --all --check
~/.cargo/bin/cargo clippy --locked --workspace --all-targets -- -D warnings
~/.cargo/bin/cargo test --locked --workspace --all-targets
RUSTDOCFLAGS='-D warnings' ~/.cargo/bin/cargo doc --locked --workspace --no-deps
```

Expected: all four pass. The database integration tests print "skipped" without their servers; that is expected here. A failure is fixed in the task it belongs to, with its own commit.

- [ ] **Step 2: Confirm the lock file changed by one line**

Run: `git diff main --stat -- Cargo.lock`
Expected: `1 file changed, 1 insertion(+)`.

- [ ] **Step 3: See the migration and the options work against real files**

The app reads `$XDG_CONFIG_HOME/tabletist` on Linux. In a scratch directory (never `/tmp`: use the session's scratchpad or `target/`):

```bash
mkdir -p target/settings-check/tabletist
printf '{"version": 1, "page_size": 100, "sql_limit": 100}' > target/settings-check/tabletist/settings.json
~/.cargo/bin/cargo build --locked
XDG_CONFIG_HOME="$PWD/target/settings-check" XDG_STATE_HOME="$PWD/target/settings-check/state" timeout 8 target/debug/tabletist --verbose; true
cat target/settings-check/tabletist/settings.toml
cat target/settings-check/tabletist/settings.json
```

Expected: `settings.toml` exists with `page_size    = 100` and `sql_limit        = 100` in the canonical layout; `settings.json` is unchanged. If no display is available the window cannot open: say so in the report instead of claiming this step, and rely on the tests of Tasks 5 and 6.

Then check the log names a bad line:

```bash
printf '[data]\ngroup_digits = "yes"\n' > target/settings-check/tabletist/settings.toml
XDG_CONFIG_HOME="$PWD/target/settings-check" XDG_STATE_HOME="$PWD/target/settings-check/state" timeout 8 target/debug/tabletist --verbose 2>&1 | grep "line 2"; true
rm -rf target/settings-check
```

Expected: a warning naming `settings.toml` and "line 2 could not be read and is ignored" (on stderr, or in `target/settings-check/state/tabletist/tabletist.log` if the log goes to the file only).

- [ ] **Step 4: Report**

Say what passed, what was only compiled (macOS and Windows: this step has no platform code, and CI runs all three), which commits exist, and which are waiting for a signature (tree ids and messages).
