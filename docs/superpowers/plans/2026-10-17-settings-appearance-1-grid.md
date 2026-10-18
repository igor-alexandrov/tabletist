# Settings › Appearance, step 1: Grid and Environment Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add the Appearance pane to Settings in both looks, with its Grid section (row density, alternate rows, column types, table list) and its Environment section (how much the connection bar is tinted), each change applied to every open window at once and written to `settings.toml`.

**Architecture:** Five new keys in `Settings` (`[appearance.grid]`, `[appearance.env]`), read and written by the existing canonical writer. The grids take them through `Look::with_grid`, which gives a look the settings' density, stripes and header lines; the connection bar takes them through `env::bar_colors`. A connection's own tree/flat choice is kept by id in `table_lists.json` in the state directory. The pane plugs into the window's existing machinery (`OptionId`, `OptionValue`, `SettingsPane`), drawn by `sheet.rs` on macOS and `terminal.rs` on Omarchy.

**Tech Stack:** Rust, egui (crmne fork), the `toml` crate's `DeTable`, headless tests through `src/testing.rs`.

**Spec:** `docs/superpowers/specs/2026-10-17-settings-appearance-design.md`. This plan is its step 1. Steps 2 (Theme), 3 (Text and the font size keys) and 4 (Preview) get plans of their own.

---

## What was decided, and where this departs from the prompt and the boards

Asked and answered on 2026-10-17:

| Question | Answer |
|---|---|
| Write only non-default values and keep comments? | No: today's writer, every key in a fixed form, as both boards draw the file. |
| Delivery | In steps; this is step 1. |
| Table list per connection | A state file by connection id (`table_lists.json`), not `connections.toml`. |

Taken from the prompt's own default, for the PR to name: alternate rows are off on macOS and on on Omarchy, as drawn (its open question 1).

Departures this step makes, for a yes or a no before code:

| The prompt or a board asks | Step 1 builds | Why |
|---|---|---|
| macOS subtitle "Theme, type and how dense the grid is. The preview updates as you change things." | The title alone | The pane has no theme, type or preview yet. Step 4 adds the line. |
| Omarchy file pane writes `zebra = true` | `zebra` is written once set | Its default is the look's; the writer does not know the look. A reset takes it out again. |
| Regular · 36, Compact · 28, Roomy · 44 (macOS) | 37, 29, 45 pt rows, labelled as the board labels them | Today's regular row is the board's 36 plus its 1 pt rule; nobody's grid changes on upgrade. |
| (no board for Windows) | Windows draws the sheet too, with the macOS board's labels, and rows of 18, 26, 34 pt (its regular row is 26) | The labels name the design's heights; Windows is a draft look. |
| "Every control's description is its helper text" | Not in this step | The spec says "as General's are", but General's and Agents' controls have no AccessKit description either (the spec is corrected); giving every pane's controls one is a change of `widgets::segmented` and `widgets::toggle`, best done once for all panes. Proposed as a follow-up. |
| "Segmented controls are radio groups" | On Omarchy they are (its choices are radio buttons); the macOS segments stay buttons marked selected | The same `widgets::segmented` change, for General's segments too. In the same follow-up. |
| "Theme", "Text", "Preview" sections | Left out | Steps 2 to 4. |

Found while drafting, fixed here: switching database (`Action::SwitchDatabase`) put the sidebar back to a tree. It now keeps the list as it was (Task 4).

A reviewer given the draft found, and the draft now fixes: a forgotten grid kept its row height and offset, so a grid id come back to jumped to an old row; the loading skeleton was drawn at the regular height; an unsaved connection's toggle was undone by the setting, and a second window on a connection did not follow its toggle; the bar's outlines kept the tint's colour in label mode; `R` and Space in one frame on the stripes row flipped from the wrong value; one value `table_lists.json` did not know cost every connection's choice. Columns are also fitted to their names alone when the types' line is off.

## Files

| File | What it does after this plan |
|---|---|
| `src/settings.rs` | `RowHeight`, `TableList`, `EnvBar`; five `Settings` fields; five `Key`s in nested tables; five `OptionId`s and `OptionValue`s with their steps. |
| `src/theme.rs` | `Look::density`, `Look::zebra`, `Look::type_line`; `Look::with_grid` and `Look::row`. |
| `src/ui/grid.rs` | Rows at `look.row()`, stripes where `look.zebra`, the header one line without `look.type_line`, the top row kept across a change of height. |
| `src/ui/data_view.rs`, `src/ui/sql_results.rs`, `src/ui/states.rs` | Draw their grids in `look.with_grid(&app.settings)`. |
| `src/env.rs`, `src/ui/workspace.rs` | `bar_colors`: the bar's fill and rule as `env_bar` says, production always tinted. |
| `src/paths.rs`, `src/backend.rs`, `src/app.rs` | `table_lists.json`, `StateFile::TableLists`, `App::table_lists` and `App::lists_flat`; the toggle saves, the setting moves the connections without a choice. |
| `src/model.rs` | `SettingsPane::Appearance` between General and Agents. |
| `src/ui/settings/{mod,sheet,terminal}.rs` | The pane's labels, choices and rows in both looks. |
| `src/ui/appearance_settings_tests.rs` | New: the headless tests of this step. |
| `src/ui/mod.rs` | Declares the new test file; its tests share `with_sql_result`. |
| `src/ui/env_tests.rs`, `src/ui/agent_settings_tests.rs` | The bar's test; the Agents pane is one further along now. |
| `docs/_reference/settings-and-files.md`, `docs/_guide/omarchy.md` | The pane, the keys, `table_lists.json`. |

Every task ends with the repository's checks passing:

    cargo fmt --all --check
    cargo clippy --locked --workspace --all-targets -- -D warnings
    cargo test --locked --workspace --all-targets

(`cargo` is `~/.cargo/bin/cargo` in these sessions.) Commit after each task, signed as usual; if signing says "agent refused operation", 1Password is locked: leave the change staged and say so.

---

### Task 1: The keys in the settings file

**Files:**
- Modify: `src/settings.rs`

- [ ] **Step 1: Write the failing tests**

The canonical text gains the two tables; the full round trip and the key lines take the new keys; three new tests read them.

```diff
diff --git a/src/settings.rs b/src/settings.rs
index 3ad616a..8bb225c 100644
--- a/src/settings.rs
+++ b/src/settings.rs
@@ -838,6 +838,14 @@ sql_timeout_secs = 30  # 0 waits forever
 enabled  = false
 log_days = 30
 notify   = true
+
+[appearance.grid]
+row_height   = \"regular\"  # compact | regular | roomy
+column_types = true
+table_list   = \"tree\"  # tree | flat
+
+[appearance.env]
+bar = \"tint\"  # tint | label
 "
         );
     }
@@ -860,7 +868,9 @@ notify   = true
             "{text}"
         );
         assert!(
-            text.ends_with("\n\n[appearance]\ntheme = \"My \\\"Nord\\\".json\"\n"),
+            text.contains(
+                "\n\n[appearance]\ntheme = \"My \\\"Nord\\\".json\"\n\n[appearance.grid]\n"
+            ),
             "{text}"
         );
     }
@@ -887,6 +897,11 @@ notify   = true
             mcp_enabled: true,
             mcp_log_days: 7,
             mcp_notify: false,
+            row_height: RowHeight::Roomy,
+            zebra: Some(false),
+            column_types: false,
+            table_list: TableList::Flat,
+            env_bar: EnvBar::Label,
         };
         let text = settings.to_toml();
         let loaded = Settings::from_toml(&text);
@@ -914,6 +929,10 @@ notify   = true
                 (Key::McpEnabled, 16),
                 (Key::McpLogDays, 17),
                 (Key::McpNotify, 18),
+                (Key::RowHeight, 21),
+                (Key::ColumnTypes, 22),
+                (Key::TableList, 23),
+                (Key::EnvBar, 26),
             ]
         );
     }
@@ -1542,4 +1561,59 @@ notify   = true
         );
         assert_eq!(settings.flipped(OptionId::PageSize), None);
     }
+
+    #[test]
+    fn the_appearance_keys_default_to_todays_grid() {
+        let settings = Settings::default();
+        assert_eq!(settings.row_height, RowHeight::Regular);
+        assert_eq!(settings.zebra, None);
+        assert!(settings.column_types);
+        assert_eq!(settings.table_list, TableList::Tree);
+        assert_eq!(settings.env_bar, EnvBar::Tint);
+        // Stripes left to the look: off on macOS, on in the terminal.
+        assert!(!settings.zebra_in(false));
+        assert!(settings.zebra_in(true));
+        assert!(!settings.to_toml().contains("zebra"));
+    }
+
+    #[test]
+    fn each_appearance_key_is_read_from_its_nested_table() {
+        let loaded = Settings::from_toml(
+            "[appearance.grid]\nrow_height = \"compact\"\nzebra = true\n\
+             column_types = false\ntable_list = \"flat\"\n\n[appearance.env]\nbar = \"label\"\n",
+        );
+        assert_eq!(loaded.invalid, Vec::<usize>::new());
+        assert_eq!(
+            loaded.settings,
+            Settings {
+                row_height: RowHeight::Compact,
+                zebra: Some(true),
+                column_types: false,
+                table_list: TableList::Flat,
+                env_bar: EnvBar::Label,
+                ..Settings::default()
+            }
+        );
+        // Once set, the stripes are written, whatever the look.
+        let text = loaded.settings.to_toml();
+        assert!(text.contains("zebra        = true\n"), "{text}");
+        assert_eq!(Settings::from_toml(&text).settings, loaded.settings);
+        // A table written inline is the same table.
+        let inline = Settings::from_toml("[appearance]\ngrid = { row_height = \"roomy\" }\n");
+        assert_eq!(inline.settings.row_height, RowHeight::Roomy);
+    }
+
+    #[test]
+    fn an_appearance_value_the_key_cannot_have_is_ignored_by_its_line() {
+        let loaded = Settings::from_toml(
+            "[appearance.grid]\nrow_height = \"dense\"\nzebra = \"yes\"\n\
+             column_types = 1\ntable_list = \"grouped\"\n\n[appearance.env]\nbar = \"stripe\"\n",
+        );
+        assert_eq!(loaded.invalid, vec![2, 3, 4, 5, 8]);
+        assert_eq!(loaded.settings, Settings::default());
+        // A key under the wrong table is one this version does not know.
+        let loaded = Settings::from_toml("[appearance]\nrow_height = \"compact\"\n");
+        assert_eq!(loaded.invalid, Vec::<usize>::new());
+        assert_eq!(loaded.settings.row_height, RowHeight::Regular);
+    }
 }
```

- [ ] **Step 2: Run the tests to see them fail**

Run: `cargo test --locked --lib settings::tests`
Expected: the crate's tests do not compile:

```
error[E0560]: struct `settings::Settings` has no field named `row_height`
error[E0433]: cannot find type `RowHeight` in this scope
error[E0560]: struct `settings::Settings` has no field named `zebra`
error[E0560]: struct `settings::Settings` has no field named `column_types`
error[E0560]: struct `settings::Settings` has no field named `table_list`
error[E0560]: struct `settings::Settings` has no field named `env_bar`
...
```

- [ ] **Step 3: Add the keys**

Three closed sets as `Timestamps` is one, five fields, five keys in `[appearance.grid]` and `[appearance.env]`, their literals and their reading. A key in a nested table is found by walking its dotted path. `zebra` is an `Option`: written once it is set.

```diff
diff --git a/src/settings.rs b/src/settings.rs
index 8bb225c..2375e09 100644
--- a/src/settings.rs
+++ b/src/settings.rs
@@ -34,6 +34,101 @@ impl Timestamps {
     }
 }
 
+/// How tall the grid's rows are: the look's own, a step less, or a step
+/// more.
+#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize)]
+#[serde(rename_all = "lowercase")]
+pub enum RowHeight {
+    Compact,
+    #[default]
+    Regular,
+    Roomy,
+}
+
+impl RowHeight {
+    /// In the order the window draws them.
+    pub const ALL: [RowHeight; 3] = [Self::Compact, Self::Regular, Self::Roomy];
+
+    /// As the file says it.
+    pub fn name(self) -> &'static str {
+        match self {
+            Self::Compact => "compact",
+            Self::Regular => "regular",
+            Self::Roomy => "roomy",
+        }
+    }
+
+    pub fn from_name(name: &str) -> Option<Self> {
+        Self::ALL.into_iter().find(|choice| choice.name() == name)
+    }
+}
+
+/// How the sidebar lists a connection's tables when it has no choice of
+/// its own: in groups of a shared prefix, or one list.
+#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, Deserialize)]
+#[serde(rename_all = "lowercase")]
+pub enum TableList {
+    #[default]
+    Tree,
+    Flat,
+}
+
+impl TableList {
+    /// As the file says it.
+    pub fn name(self) -> &'static str {
+        match self {
+            Self::Tree => "tree",
+            Self::Flat => "flat",
+        }
+    }
+
+    pub fn from_name(name: &str) -> Option<Self> {
+        [Self::Tree, Self::Flat]
+            .into_iter()
+            .find(|choice| choice.name() == name)
+    }
+
+    /// Whether the list is flat.
+    pub fn flat(self) -> bool {
+        match self {
+            Self::Tree => false,
+            Self::Flat => true,
+        }
+    }
+
+    /// The list that `flat` says.
+    pub fn of(flat: bool) -> Self {
+        if flat { Self::Flat } else { Self::Tree }
+    }
+}
+
+/// How much of its environment's colour the connection bar takes: the
+/// tint and the stripe or label, or the stripe or label alone. Production
+/// is tinted either way.
+#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize)]
+#[serde(rename_all = "lowercase")]
+pub enum EnvBar {
+    #[default]
+    Tint,
+    Label,
+}
+
+impl EnvBar {
+    /// As the file says it.
+    pub fn name(self) -> &'static str {
+        match self {
+            Self::Tint => "tint",
+            Self::Label => "label",
+        }
+    }
+
+    pub fn from_name(name: &str) -> Option<Self> {
+        [Self::Tint, Self::Label]
+            .into_iter()
+            .find(|choice| choice.name() == name)
+    }
+}
+
 /// Everything the user can set. A key the file lacks takes its default and
 /// one this version does not know is ignored, so older and newer files both
 /// load. The serde derive reads the `settings.json` of versions before the
@@ -68,6 +163,18 @@ pub struct Settings {
     /// macOS the Dock's bounce, on Windows the taskbar button's flash. The
     /// Agents entry says who is here either way.
     pub mcp_notify: bool,
+    /// How tall the grids' rows are.
+    pub row_height: RowHeight,
+    /// Odd rows shaded in the grids. `None` until the user sets it: the
+    /// look's own default, off on macOS and on in the terminal.
+    pub zebra: Option<bool>,
+    /// The header's second line: each column's type.
+    pub column_types: bool,
+    /// How the sidebar lists the tables of a connection with no choice of
+    /// its own.
+    pub table_list: TableList,
+    /// How the connection bar shows a connection's environment.
+    pub env_bar: EnvBar,
 }
 
 /// A key of the settings file.
@@ -84,11 +191,16 @@ pub enum Key {
     McpLogDays,
     McpNotify,
     Theme,
+    RowHeight,
+    Zebra,
+    ColumnTypes,
+    TableList,
+    EnvBar,
 }
 
 impl Key {
     /// In the order the file has them: a table's keys together.
-    pub const ALL: [Key; 11] = [
+    pub const ALL: [Key; 16] = [
         Self::PageSize,
         Self::Timestamps,
         Self::GroupDigits,
@@ -100,6 +212,11 @@ impl Key {
         Self::McpLogDays,
         Self::McpNotify,
         Self::Theme,
+        Self::RowHeight,
+        Self::Zebra,
+        Self::ColumnTypes,
+        Self::TableList,
+        Self::EnvBar,
     ];
 
     /// The key's table and its name there.
@@ -116,6 +233,11 @@ impl Key {
             Self::McpLogDays => ("mcp", "log_days"),
             Self::McpNotify => ("mcp", "notify"),
             Self::Theme => ("appearance", "theme"),
+            Self::RowHeight => ("appearance.grid", "row_height"),
+            Self::Zebra => ("appearance.grid", "zebra"),
+            Self::ColumnTypes => ("appearance.grid", "column_types"),
+            Self::TableList => ("appearance.grid", "table_list"),
+            Self::EnvBar => ("appearance.env", "bar"),
         }
     }
 
@@ -125,6 +247,9 @@ impl Key {
         match self {
             Self::Timestamps => Some("second | full"),
             Self::SqlTimeoutSecs => Some("0 waits forever"),
+            Self::RowHeight => Some("compact | regular | roomy"),
+            Self::TableList => Some("tree | flat"),
+            Self::EnvBar => Some("tint | label"),
             _ => None,
         }
     }
@@ -473,6 +598,12 @@ impl Settings {
         }
     }
 
+    /// Whether odd rows are shaded in a look that shades them by default
+    /// (`terminal`) or not.
+    pub fn zebra_in(&self, terminal: bool) -> bool {
+        self.zebra.unwrap_or(terminal)
+    }
+
     /// The value one step from `option`'s, towards the right (`forward`)
     /// or the left, as the window draws its choices: the next page size
     /// (none past the ends), the segment on that side, a check on to the
@@ -533,7 +664,7 @@ impl Settings {
     }
 
     /// `key`'s value as TOML writes it. `None` for a key the file leaves
-    /// out: a theme that is not set.
+    /// out: a theme that is not set, and stripes left to the look.
     fn literal(&self, key: Key) -> Option<String> {
         Some(match key {
             Key::PageSize => self.page_size.to_string(),
@@ -548,6 +679,11 @@ impl Settings {
             Key::McpLogDays => self.mcp_log_days.to_string(),
             Key::McpNotify => self.mcp_notify.to_string(),
             Key::Theme => toml_string(self.custom_theme.as_deref()?),
+            Key::RowHeight => toml_string(self.row_height.name()),
+            Key::Zebra => self.zebra?.to_string(),
+            Key::ColumnTypes => self.column_types.to_string(),
+            Key::TableList => toml_string(self.table_list.name()),
+            Key::EnvBar => toml_string(self.env_bar.name()),
         })
     }
 
@@ -624,6 +760,17 @@ impl Settings {
             Key::McpLogDays => number.map(|number| self.mcp_log_days = number),
             Key::McpNotify => flag.map(|flag| self.mcp_notify = flag),
             Key::Theme => text.map(|name| self.custom_theme = Some(name.to_owned())),
+            Key::RowHeight => text
+                .and_then(RowHeight::from_name)
+                .map(|height| self.row_height = height),
+            Key::Zebra => flag.map(|flag| self.zebra = Some(flag)),
+            Key::ColumnTypes => flag.map(|flag| self.column_types = flag),
+            Key::TableList => text
+                .and_then(TableList::from_name)
+                .map(|list| self.table_list = list),
+            Key::EnvBar => text
+                .and_then(EnvBar::from_name)
+                .map(|bar| self.env_bar = bar),
         }
         .is_some()
     }
@@ -695,9 +842,18 @@ impl Settings {
         let mut lines = Vec::new();
         for key in Key::ALL {
             let (table, name) = key.path();
-            let Some((found, value)) = document
-                .get(table)
-                .and_then(|table| table.get_ref().as_table())
+            // A table under another is found by its dotted path.
+            let mut parts = table.split('.');
+            let first = parts
+                .next()
+                .and_then(|part| document.get(part))
+                .and_then(|table| table.get_ref().as_table());
+            let Some((found, value)) = parts
+                .fold(first, |table, part| {
+                    table
+                        .and_then(|table| table.get(part))
+                        .and_then(|table| table.get_ref().as_table())
+                })
                 .and_then(|table| table.get_key_value(name))
             else {
                 continue;
@@ -780,6 +936,11 @@ impl Default for Settings {
             mcp_enabled: false,
             mcp_log_days: 30,
             mcp_notify: true,
+            row_height: RowHeight::Regular,
+            zebra: None,
+            column_types: true,
+            table_list: TableList::Tree,
+            env_bar: EnvBar::Tint,
         }
     }
 }
```

- [ ] **Step 4: Run the tests to see them pass**

Run: `cargo test --locked --lib settings::tests`
Expected: PASS. Then the three checks above.

- [ ] **Step 5: Commit**

```bash
git add src/settings.rs
git commit -m "Read and write the grid's and the environment's keys of the Appearance tab"
```

---

### Task 2: The grids at the settings' density, stripes and header lines

**Files:**
- Modify: `src/theme.rs`, `src/ui/grid.rs`, `src/ui/states.rs`, `src/ui/data_view.rs`, `src/ui/sql_results.rs`, `src/ui/mod.rs`
- Create: `src/ui/appearance_settings_tests.rs`

- [ ] **Step 1: Write the failing tests**

A new test file, declared in `src/ui/mod.rs`. `change` sets the settings as an edit of the file does (`App::apply_settings`); the pane comes in Task 5. The top-row test waits for egui's smooth wheel scrolling to stop before it measures. The SQL editor's results are checked with `with_sql_result`, which `src/ui/mod.rs`'s tests now share.

```diff
diff --git a/src/ui/appearance_settings_tests.rs b/src/ui/appearance_settings_tests.rs
new file mode 100644
index 0000000..5fdb9a1
--- /dev/null
+++ b/src/ui/appearance_settings_tests.rs
@@ -0,0 +1,205 @@
+//! Settings › Appearance, in both looks: what each option does to the
+//! grids and the sidebar's list.
+
+use egui::accesskit::Role;
+
+use crate::model::{Action, ConnTabId, TabId};
+use crate::settings::{RowHeight, Settings};
+use crate::testing::{Harness, bounds};
+use crate::theme::Look;
+
+fn looks() -> [Look; 2] {
+    [Look::macos(), Look::omarchy()]
+}
+
+/// The settings changed by `change`, as an edit of the file changes them.
+fn change(harness: &mut Harness, change: impl FnOnce(&mut Settings)) {
+    let mut settings = harness.app.settings.clone();
+    change(&mut settings);
+    harness.app.apply_settings(settings);
+    harness.settle();
+    // A scroll asked for lands two frames later.
+    harness.settle();
+}
+
+/// `book_covers` with `rows` rows, in `look`.
+fn covers(look: Look, rows: usize) -> (Harness, ConnTabId, TabId) {
+    let mut harness = Harness::new();
+    harness.set_look(look);
+    let tab = harness.connect_fake_as(false);
+    harness.app.apply(Action::OpenObject {
+        tab,
+        object: tabletist_db::ObjectRef::new("main", "book_covers"),
+        kind: tabletist_db::ObjectKind::Table,
+        pin: true,
+    });
+    harness.answer_structure(crate::testing::book_covers_structure());
+    harness.answer_rows(crate::testing::book_covers_page(rows));
+    harness.settle();
+    let id = harness.app.workspace(tab).unwrap().active_tab.unwrap();
+    (harness, tab, id)
+}
+
+/// Whether the last frame painted a text that has `words` in it.
+fn shows(harness: &mut Harness, words: &str) -> bool {
+    harness.settle();
+    harness.painted.iter().any(|(text, _)| text.contains(words))
+}
+
+/// The grid's rows as drawn: each row's number and its place.
+fn rows(harness: &mut Harness) -> Vec<(u64, egui::Rect)> {
+    let tree = harness.settle();
+    (1..=500)
+        .filter_map(|number| {
+            bounds(&tree, &format!("Row {number}"), Role::Button).map(|rect| (number, rect))
+        })
+        .collect()
+}
+
+/// The rows drawn on the panel's colour: the stripes.
+fn shaded(harness: &mut Harness) -> Vec<u64> {
+    let panel = harness.app.palette.panel;
+    let rows = rows(harness);
+    rows.into_iter()
+        .filter(|(_, rect)| {
+            harness
+                .fills
+                .iter()
+                .any(|(fill, color)| *color == panel && fill.contains_rect(*rect))
+        })
+        .map(|(number, _)| number)
+        .collect()
+}
+
+/// The grid's own place: its header's top.
+fn grid_top(harness: &mut Harness) -> f32 {
+    let tree = harness.settle();
+    bounds(&tree, "Rows", Role::Group).expect("the grid").top()
+}
+
+#[test]
+fn row_density_sets_the_rows_height_in_each_look() {
+    for look in looks() {
+        let (mut harness, _, _) = covers(look, 3);
+        // With no file: today's rows.
+        let regular = if look.terminal { 30.0 } else { 37.0 };
+        for (_, rect) in rows(&mut harness) {
+            assert_eq!(rect.height(), regular, "{}", look.name);
+        }
+        for (height, expected) in [
+            (RowHeight::Compact, if look.terminal { 20.0 } else { 29.0 }),
+            (RowHeight::Roomy, if look.terminal { 40.0 } else { 45.0 }),
+            (RowHeight::Regular, regular),
+        ] {
+            change(&mut harness, |settings| settings.row_height = height);
+            let rows = rows(&mut harness);
+            assert_eq!(rows.len(), 3, "{}", look.name);
+            for (_, rect) in rows {
+                assert_eq!(rect.height(), expected, "{} {height:?}", look.name);
+            }
+        }
+    }
+}
+
+#[test]
+fn row_density_keeps_the_top_row_where_it_was() {
+    for look in looks() {
+        let (mut harness, _, _) = covers(look, 300);
+        let middle = harness.size / 2.0;
+        for _ in 0..10 {
+            harness.frame(vec![
+                egui::Event::PointerMoved(middle.to_pos2()),
+                egui::Event::MouseWheel {
+                    unit: egui::MouseWheelUnit::Point,
+                    delta: egui::vec2(0.0, -400.0),
+                    modifiers: egui::Modifiers::NONE,
+                    phase: egui::TouchPhase::Move,
+                },
+            ]);
+        }
+        // The wheel scrolls smoothly: until it has stopped.
+        for _ in 0..5 {
+            harness.finish_animations();
+        }
+        // The row at the top: the first one under the sticky header.
+        let under = grid_top(&mut harness) + crate::ui::grid::header_height(&look) + 1.0;
+        let top = |harness: &mut Harness| {
+            rows(harness)
+                .into_iter()
+                .find(|(_, rect)| rect.bottom() > under)
+                .map(|(number, _)| number)
+                .expect("a row at the top")
+        };
+        let before = top(&mut harness);
+        assert!(before > 50, "{} scrolled to {before}", look.name);
+        for height in [RowHeight::Roomy, RowHeight::Compact, RowHeight::Regular] {
+            change(&mut harness, |settings| settings.row_height = height);
+            assert_eq!(top(&mut harness), before, "{} {height:?}", look.name);
+        }
+    }
+}
+
+#[test]
+fn alternate_rows_shade_the_odd_rows_where_they_are_on() {
+    for look in looks() {
+        let (mut harness, _, _) = covers(look, 4);
+        // As drawn: off on macOS, on in the terminal.
+        let default: Vec<u64> = if look.terminal { vec![2, 4] } else { vec![] };
+        assert_eq!(shaded(&mut harness), default, "{}", look.name);
+        change(&mut harness, |settings| settings.zebra = Some(true));
+        assert_eq!(shaded(&mut harness), vec![2, 4], "{}", look.name);
+        change(&mut harness, |settings| settings.zebra = Some(false));
+        assert_eq!(shaded(&mut harness), Vec::<u64>::new(), "{}", look.name);
+    }
+}
+
+#[test]
+fn the_sql_editors_results_take_the_density_and_the_stripes() {
+    for look in looks() {
+        let mut harness = Harness::new();
+        harness.set_look(look);
+        let tab = harness.connect_fake();
+        super::tests::with_sql_result(&mut harness, tab, 4);
+        change(&mut harness, |settings| {
+            settings.row_height = RowHeight::Roomy;
+            settings.zebra = Some(true);
+        });
+        let rows = rows(&mut harness);
+        assert_eq!(rows.len(), 4, "{}", look.name);
+        let roomy = if look.terminal { 40.0 } else { 45.0 };
+        for (_, rect) in rows {
+            assert_eq!(rect.height(), roomy, "{}", look.name);
+        }
+        assert_eq!(shaded(&mut harness), vec![2, 4], "{}", look.name);
+    }
+}
+
+#[test]
+fn without_column_types_the_header_is_one_line() {
+    for look in looks() {
+        let (mut harness, _, _) = covers(look, 3);
+        let header = |harness: &mut Harness| {
+            let top = grid_top(harness);
+            rows(harness)[0].1.top() - top
+        };
+        let near = |height: f32, expected: f32| (height - expected).abs() < 1.0;
+        let two = header(&mut harness);
+        let expected = if look.terminal { 41.0 } else { 45.0 };
+        assert!(near(two, expected), "{} {two}", look.name);
+        // The id column's type, or the key the terminal writes for it.
+        let typed = |harness: &mut Harness| {
+            harness.settle();
+            harness
+                .painted
+                .iter()
+                .any(|(text, _)| text.eq_ignore_ascii_case("integer") || text == "pk")
+        };
+        assert!(typed(&mut harness), "{} {:?}", look.name, harness.painted);
+        change(&mut harness, |settings| settings.column_types = false);
+        let one = header(&mut harness);
+        let expected = if look.terminal { 27.0 } else { 31.0 };
+        assert!(near(one, expected), "{} {one}", look.name);
+        assert!(shows(&mut harness, "publisher_id"), "{}", look.name);
+        assert!(!typed(&mut harness), "{}", look.name);
+    }
+}
diff --git a/src/ui/mod.rs b/src/ui/mod.rs
index 1159b8b..523f526 100644
--- a/src/ui/mod.rs
+++ b/src/ui/mod.rs
@@ -13,6 +13,8 @@ mod agents_tests;
 #[cfg(test)]
 #[cfg(any(unix, windows))]
 mod agent_settings_tests;
+#[cfg(test)]
+mod appearance_settings_tests;
 pub mod cell_editor;
 #[cfg(test)]
 mod complete_tests;
@@ -4067,7 +4069,7 @@ mod tests {
 
     /// Opens a SQL editor in `tab`, runs `SELECT 1` in it and answers with
     /// `rows` rows.
-    fn with_sql_result(
+    pub(super) fn with_sql_result(
         harness: &mut Harness,
         tab: crate::model::ConnTabId,
         rows: usize,
```

- [ ] **Step 2: Run the tests to see them fail**

Run: `cargo test --locked --lib appearance_settings_tests`
Expected: four fail. The top-row test passes before the code, since no row changes its height yet; it is the guard for Step 4's carried offset, and fails without the carry once heights change.

```
test ui::appearance_settings_tests::the_sql_editors_results_take_the_density_and_the_stripes ... FAILED
test ui::appearance_settings_tests::row_density_sets_the_rows_height_in_each_look ... FAILED
test ui::appearance_settings_tests::alternate_rows_shade_the_odd_rows_where_they_are_on ... FAILED
test ui::appearance_settings_tests::without_column_types_the_header_is_one_line ... FAILED
test ui::appearance_settings_tests::row_density_keeps_the_top_row_where_it_was ... ok
test result: FAILED. 1 passed; 4 failed; 0 ignored; 0 measured; 2562 filtered out; finished in 0.24s
```

- [ ] **Step 3: Give the look the grid's settings**

`Look` keeps its regular row in `grid_row` and gains the density, the stripes and the header's second line, with the look's defaults (stripes on Omarchy only, as `row_fill` had them). `Look::row` is the height a grid draws a row at: in lines on the terminal (2/3, 1, 4/3 of the regular row), 8 pt either side elsewhere.

```diff
diff --git a/src/theme.rs b/src/theme.rs
index 37581c6..b630341 100644
--- a/src/theme.rs
+++ b/src/theme.rs
@@ -3,6 +3,8 @@
 
 use egui::{Color32, CornerRadius, Stroke, Vec2};
 
+use crate::settings::RowHeight;
+
 /// A palette file from the themes directory.
 pub type CustomTheme = fastframe_theme::CustomTheme<Palette>;
 /// The palette files, the shared presets, and Omarchy's live palette.
@@ -363,8 +365,17 @@ pub struct Look {
     pub control_height: f32,
     /// Sidebar tree rows.
     pub tree_row: f32,
-    /// Grid rows.
+    /// Grid rows at the regular density: [`Look::row`] is the height a
+    /// grid draws them at.
     pub grid_row: f32,
+    /// How dense the grids are: the Appearance setting, regular until a
+    /// look is given the settings ([`Look::with_grid`]).
+    pub density: RowHeight,
+    /// Odd grid rows shaded: the look's default until it is given the
+    /// settings.
+    pub zebra: bool,
+    /// The grid header's second line, the columns' types.
+    pub type_line: bool,
     /// Omarchy's controls: a faint foreground fill and a 1 px border.
     pub bordered_controls: bool,
     pub selection: Selection,
@@ -399,6 +410,9 @@ impl Look {
             control_height: 26.0,
             tree_row: 24.0,
             grid_row: 26.0,
+            density: RowHeight::Regular,
+            zebra: false,
+            type_line: true,
             bordered_controls: false,
             selection: Selection::Tint,
             tabs: TabStyle::Outlined,
@@ -424,6 +438,9 @@ impl Look {
             control_height: 28.0,
             tree_row: 24.0,
             grid_row: 37.0,
+            density: RowHeight::Regular,
+            zebra: false,
+            type_line: true,
             bordered_controls: false,
             selection: Selection::Pill,
             tabs: TabStyle::Raised,
@@ -449,6 +466,9 @@ impl Look {
             control_height: 24.0,
             tree_row: 24.0,
             grid_row: 30.0,
+            density: RowHeight::Regular,
+            zebra: true,
+            type_line: true,
             bordered_controls: true,
             selection: Selection::Bar,
             tabs: TabStyle::Underline,
@@ -463,6 +483,32 @@ impl Look {
         }
     }
 
+    /// The look with the grids as `settings` have them: how dense their
+    /// rows are, whether odd ones are shaded, and whether the header has
+    /// the types' line.
+    pub fn with_grid(self, settings: &crate::settings::Settings) -> Self {
+        Self {
+            density: settings.row_height,
+            zebra: settings.zebra_in(self.terminal),
+            type_line: settings.column_types,
+            ..self
+        }
+    }
+
+    /// A grid row's height at the look's density. The terminal counts in
+    /// lines, a regular row being one and a half: one or two. The others
+    /// step 8 either side, the regular row's rule kept in each.
+    pub fn row(&self) -> f32 {
+        let regular = self.grid_row;
+        match (self.density, self.terminal) {
+            (RowHeight::Regular, _) => regular,
+            (RowHeight::Compact, true) => regular * 2.0 / 3.0,
+            (RowHeight::Roomy, true) => regular * 4.0 / 3.0,
+            (RowHeight::Compact, false) => regular - 8.0,
+            (RowHeight::Roomy, false) => regular + 8.0,
+        }
+    }
+
     /// The look of the OS this build runs on. Every Linux desktop gets the
     /// Omarchy look.
     pub const fn for_platform() -> Self {
```

- [ ] **Step 4: Draw the grid from the look**

Rows at `look.row()`; stripes where `look.zebra`; the header 31 or 27 pt with the name alone when `!look.type_line`, and the columns fitted to the names alone then. The offset the scroll area last had is kept with the row height: a frame that draws another height scales it, so the same row stays at the top. `forget` drops that record with the rest. The grid's two tests of `initial_widths` pass it the types' line.

```diff
diff --git a/src/ui/grid.rs b/src/ui/grid.rs
index 94330d4..7d2bb03 100644
--- a/src/ui/grid.rs
+++ b/src/ui/grid.rs
@@ -18,10 +18,16 @@ use crate::ui::format::{Marks, array_items, display_safe};
 use crate::ui::states::Tone;
 use crate::ui::widgets::virtual_rows;
 
-/// The header's height, per look.
+/// The header's height, per look: 44 (macOS) or 40 (terminal) with the
+/// types' line under the names, 30 or 26 with the names alone, and the
+/// rule under the header.
 pub fn header_height(look: &crate::theme::Look) -> f32 {
-    // 44 (macOS) or 40 (terminal), and the rule under the header.
-    if look.terminal { 41.0 } else { 45.0 }
+    match (look.terminal, look.type_line) {
+        (true, true) => 41.0,
+        (false, true) => 45.0,
+        (true, false) => 27.0,
+        (false, false) => 31.0,
+    }
 }
 /// The terminal look's gutter, where the selected row's cursor sits.
 const GUTTER: f32 = 22.0;
@@ -383,8 +389,8 @@ pub fn column_at(widths: &[f32], x: f32) -> usize {
 }
 
 /// A data row's background: the selected row's colour wins, then the hover
-/// tint, then (in the terminal look) the zebra stripe on odd rows. `None`
-/// leaves the window colour.
+/// tint, then the zebra stripe on odd rows where the look shades them.
+/// `None` leaves the window colour.
 pub fn row_fill(
     selected: bool,
     hovered: bool,
@@ -396,7 +402,7 @@ pub fn row_fill(
         Some(palette.selection)
     } else if hovered {
         Some(palette.panel.lerp_to_gamma(palette.surface, 0.5))
-    } else if odd && look.terminal {
+    } else if odd && look.zebra {
         Some(palette.panel)
     } else {
         None
@@ -510,7 +516,9 @@ pub fn ellipsize<'t>(
 pub fn initial_widths<'a>(
     columns: &[Column<'_>],
     rows: usize,
-    (pad, key_width): (f32, f32),
+    // The cells' padding, a numeric key's fixed width, and whether the
+    // header has the types' line, which its column is as wide as.
+    (pad, key_width, types): (f32, f32, bool),
     width: &impl Fn(&str) -> f32,
     cell: &mut impl FnMut(usize, usize) -> Cell<'a>,
 ) -> Vec<f32> {
@@ -524,9 +532,12 @@ pub fn initial_widths<'a>(
             let header = if column.key && column.numeric && key_width > 0.0 {
                 key_width
             } else {
-                width(&display_safe(column.name)).max(width(&display_safe(&column.type_line)))
-                    + 2.0 * pad
-                    + 16.0
+                let type_line = if types {
+                    width(&display_safe(&column.type_line))
+                } else {
+                    0.0
+                };
+                width(&display_safe(column.name)).max(type_line) + 2.0 * pad + 16.0
             };
             let widest = (0..rows.min(SAMPLE_ROWS))
                 .map(|row| {
@@ -567,7 +578,8 @@ struct Kept {
 }
 
 /// Drops what egui's memory keeps for the grid `id`: its columns' widths,
-/// the selection it last revealed and where it was scrolled to. For a grid
+/// the selection it last revealed, where it was scrolled to and at what
+/// row height. For a grid
 /// that will not be drawn again (a SQL editor's result, once the next run
 /// has its own grid).
 pub fn forget(ctx: &egui::Context, id: Id) {
@@ -578,6 +590,7 @@ pub fn forget(ctx: &egui::Context, id: Id) {
         }
         data.remove::<Option<CellPos>>(id.with("last-selection"));
         data.remove::<ColumnsShown>(id.with("columns-shown"));
+        data.remove::<(f32, f32)>(id.with("row-height"));
         data.remove::<Kept>(id);
     });
 }
@@ -609,6 +622,7 @@ pub fn remembered(ctx: &egui::Context, id: Id) -> bool {
             || data
                 .get_temp::<Option<CellPos>>(id.with("last-selection"))
                 .is_some()
+            || data.get_temp::<(f32, f32)>(id.with("row-height")).is_some()
     })
 }
 
@@ -631,7 +645,7 @@ pub fn show<'a>(
     mut cell: impl FnMut(usize, usize) -> Cell<'a>,
 ) -> GridOutput {
     let mut output = GridOutput::default();
-    let row_height = look.grid_row;
+    let row_height = look.row();
     let header_height = header_height(look);
     let pad = cell_pad(look);
     let gutter = if look.terminal { GUTTER } else { 0.0 };
@@ -650,7 +664,7 @@ pub fn show<'a>(
         let mut widths = initial_widths(
             columns,
             row_count,
-            (pad, key_width(look)),
+            (pad, key_width(look), look.type_line),
             &width,
             &mut cell,
         );
@@ -718,309 +732,319 @@ pub fn show<'a>(
     let lit =
         keys && focus::visible(ui.ctx()) && (editing.is_some() || !focus::on_control(ui.ctx()));
 
-    let scroll = egui::ScrollArea::both()
+    // Rows of another height keep the same row at the top: the offset,
+    // which is in points, is carried from the height the grid had to this.
+    // The height and the offset the last frame drew them at.
+    let height_id = id.with("row-height");
+    let before = ui.data(|data| data.get_temp::<(f32, f32)>(height_id));
+    let mut area = egui::ScrollArea::both()
         .id_salt(id)
-        .auto_shrink([false, false])
-        .show(ui, |ui| {
-            ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
-            let origin = ui.cursor().min;
-            let full = total.max(ui.available_width());
-            ui.allocate_space(vec2(total, header_height));
+        .auto_shrink([false, false]);
+    if let Some((before, offset)) = before.filter(|(before, _)| *before != row_height) {
+        area = area.vertical_scroll_offset(offset / before * row_height);
+    }
+    let scroll = area.show(ui, |ui| {
+        ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
+        let origin = ui.cursor().min;
+        let full = total.max(ui.available_width());
+        ui.allocate_space(vec2(total, header_height));
 
-            // How far the pinned column stands off its place: as far as
-            // the grid is scrolled sideways.
-            let shift = if pinned {
-                // Under a point it is the clip's own margin, not a scroll.
-                Some(ui.clip_rect().left() - origin.x)
-                    .filter(|shift| *shift >= 1.0)
-                    .unwrap_or(0.0)
+        // How far the pinned column stands off its place: as far as
+        // the grid is scrolled sideways.
+        let shift = if pinned {
+            // Under a point it is the clip's own margin, not a scroll.
+            Some(ui.clip_rect().left() - origin.x)
+                .filter(|shift| *shift >= 1.0)
+                .unwrap_or(0.0)
+        } else {
+            0.0
+        };
+        if let Some(target) = reveal {
+            let col = target.col.min(widths.len().saturating_sub(1));
+            let x = origin.x + gutter + widths[..col].iter().sum::<f32>();
+            let y = origin.y + header_height + target.row as f32 * row_height;
+            // Include the header's height above the row so the sticky
+            // header never covers it, and the pinned column's width
+            // before a cell that scrolls so that never does either.
+            let cover = if pinned && col > 0 {
+                gutter + widths[0]
             } else {
                 0.0
             };
-            if let Some(target) = reveal {
-                let col = target.col.min(widths.len().saturating_sub(1));
-                let x = origin.x + gutter + widths[..col].iter().sum::<f32>();
-                let y = origin.y + header_height + target.row as f32 * row_height;
-                // Include the header's height above the row so the sticky
-                // header never covers it, and the pinned column's width
-                // before a cell that scrolls so that never does either.
-                let cover = if pinned && col > 0 {
-                    gutter + widths[0]
-                } else {
-                    0.0
-                };
-                let rect = Rect::from_min_size(
-                    pos2(x - cover, y - header_height),
-                    vec2(
-                        widths.get(col).copied().unwrap_or(0.0) + cover,
-                        row_height + header_height,
-                    ),
-                );
-                ui.scroll_to_rect(rect, None);
-            }
+            let rect = Rect::from_min_size(
+                pos2(x - cover, y - header_height),
+                vec2(
+                    widths.get(col).copied().unwrap_or(0.0) + cover,
+                    row_height + header_height,
+                ),
+            );
+            ui.scroll_to_rect(rect, None);
+        }
 
-            virtual_rows(ui, row_count, row_height, |ui, row| {
-                // A row takes a click, not the Tab key: the grid is the stop.
-                let (rect, response) = ui.allocate_exact_size(vec2(full, row_height), Sense::CLICK);
-                let Row {
-                    mark,
-                    number,
-                    deleted,
-                    ranged,
-                } = rows(row);
-                let label = match number {
-                    Some(number) if deleted => format!("Row {number}, marked for deletion"),
-                    Some(number) => format!("Row {number}"),
-                    None => "New row, not saved".to_owned(),
-                };
-                response.widget_info(|| WidgetInfo::labeled(WidgetType::Button, true, &label));
-                if ranged {
-                    ui.ctx().accesskit_node_builder(response.id, |node| {
-                        node.set_selected(true);
-                    });
+        virtual_rows(ui, row_count, row_height, |ui, row| {
+            // A row takes a click, not the Tab key: the grid is the stop.
+            let (rect, response) = ui.allocate_exact_size(vec2(full, row_height), Sense::CLICK);
+            let Row {
+                mark,
+                number,
+                deleted,
+                ranged,
+            } = rows(row);
+            let label = match number {
+                Some(number) if deleted => format!("Row {number}, marked for deletion"),
+                Some(number) => format!("Row {number}"),
+                None => "New row, not saved".to_owned(),
+            };
+            response.widget_info(|| WidgetInfo::labeled(WidgetType::Button, true, &label));
+            if ranged {
+                ui.ctx().accesskit_node_builder(response.id, |node| {
+                    node.set_selected(true);
+                });
+            }
+            if response.clicked() || response.double_clicked() {
+                let col = response
+                    .interact_pointer_pos()
+                    .map(|pointer| {
+                        let x = pointer.x - rect.left() - gutter;
+                        // Over the pinned column, whatever is under it.
+                        if pinned && x - shift < widths[0] {
+                            0
+                        } else {
+                            column_at(&widths, x)
+                        }
+                    })
+                    .unwrap_or(0);
+                let cell = Some(CellPos { row, col });
+                if response.clicked() {
+                    output.clicked = cell;
+                    output.shift = ui.input(|input| input.modifiers.shift);
                 }
-                if response.clicked() || response.double_clicked() {
-                    let col = response
-                        .interact_pointer_pos()
-                        .map(|pointer| {
-                            let x = pointer.x - rect.left() - gutter;
-                            // Over the pinned column, whatever is under it.
-                            if pinned && x - shift < widths[0] {
-                                0
-                            } else {
-                                column_at(&widths, x)
-                            }
-                        })
-                        .unwrap_or(0);
-                    let cell = Some(CellPos { row, col });
-                    if response.clicked() {
-                        output.clicked = cell;
-                        output.shift = ui.input(|input| input.modifiers.shift);
-                    }
-                    if response.double_clicked() {
-                        output.double_clicked = cell;
-                    }
+                if response.double_clicked() {
+                    output.double_clicked = cell;
                 }
-                let painter = ui.painter().clone();
-                // A row with pending cells is marked in their tone.
-                let row_tone = match mark {
-                    RowMark::None => None,
-                    RowMark::Changed => Some(Tone::Warning),
-                    RowMark::Trouble | RowMark::Deleted => Some(Tone::Danger),
-                    RowMark::New => Some(Tone::Success),
-                };
-                let new = mark == RowMark::New;
-                // A new row's cell, as a warning knows it: by the row's
-                // having no number among the table's. Its mark is not
-                // steady enough, a cell to fix in it makes it `Trouble`.
-                let held_back = new || number.is_none();
-                // The cursor's row, and with it the rows selected with it.
-                let cursor_row = selection.is_some_and(|cell| cell.row == row);
-                let selected_row = cursor_row || ranged;
-                // With the keyboard in the grid the cell takes the
-                // selection's colour and its row a lighter tint of it.
-                let lit_row = selected_row && lit && !look.terminal;
-                let fresh = crate::env::new_row_colors(crate::env::Platform::of(look), palette);
-                let doomed =
-                    crate::env::deleted_row_colors(crate::env::Platform::of(look), palette);
-                let fill = if new {
-                    // A new row keeps its own fill under the selection
-                    // too: its cell alone takes the selection's colour,
-                    // and says where the keyboard is.
-                    Some(fresh.fill)
-                } else if deleted && !ranged {
-                    // As a new row: the row keeps its fill, and its cell
-                    // alone says where the keyboard is. Not among several
-                    // selected rows: there it takes the selection's fill
-                    // with them, so the range reads as one, and its bar
-                    // and its struck values still say it is marked.
-                    Some(doomed.fill)
-                } else if lit_row {
-                    Some(palette.window.lerp_to_gamma(palette.selection, 0.6))
-                } else {
-                    row_fill(
-                        selected_row,
-                        response.hovered(),
-                        row % 2 == 1,
-                        look,
-                        palette,
-                    )
-                };
-                if let Some(fill) = fill {
-                    painter.rect_filled(rect, CornerRadius::ZERO, fill);
-                }
-                if !look.terminal {
-                    let y = painter.round_to_pixel_center(rect.bottom() - hairline / 2.0);
-                    painter.hline(rect.x_range(), y, Stroke::new(hairline, palette.surface));
-                }
-                for &col in &order {
-                    let left = rect.left() + gutter + lefts[col];
-                    let cell_rect =
-                        Rect::from_min_size(pos2(left, rect.top()), vec2(widths[col], row_height));
-                    // The first column, and the row's mark before it, stand
-                    // where the view begins while it is pinned.
-                    let cell_rect = if col == 0 {
-                        let lead = Rect::from_min_max(
-                            pos2(rect.left() + shift, rect.top()),
-                            pos2(cell_rect.right() + shift, rect.bottom()),
-                        );
-                        if shift > 0.0 {
-                            // Over what scrolled under: the row's own fill.
-                            let under = fill.unwrap_or(palette.window);
-                            painter.rect_filled(lead, CornerRadius::ZERO, under);
-                            if !look.terminal {
-                                let y =
-                                    painter.round_to_pixel_center(rect.bottom() - hairline / 2.0);
-                                painter.hline(
-                                    lead.x_range(),
-                                    y,
-                                    Stroke::new(hairline, palette.surface),
-                                );
-                            }
-                            painter.vline(
-                                lead.right() - hairline / 2.0,
-                                lead.y_range(),
-                                Stroke::new(hairline, palette.outline),
+            }
+            let painter = ui.painter().clone();
+            // A row with pending cells is marked in their tone.
+            let row_tone = match mark {
+                RowMark::None => None,
+                RowMark::Changed => Some(Tone::Warning),
+                RowMark::Trouble | RowMark::Deleted => Some(Tone::Danger),
+                RowMark::New => Some(Tone::Success),
+            };
+            let new = mark == RowMark::New;
+            // A new row's cell, as a warning knows it: by the row's
+            // having no number among the table's. Its mark is not
+            // steady enough, a cell to fix in it makes it `Trouble`.
+            let held_back = new || number.is_none();
+            // The cursor's row, and with it the rows selected with it.
+            let cursor_row = selection.is_some_and(|cell| cell.row == row);
+            let selected_row = cursor_row || ranged;
+            // With the keyboard in the grid the cell takes the
+            // selection's colour and its row a lighter tint of it.
+            let lit_row = selected_row && lit && !look.terminal;
+            let fresh = crate::env::new_row_colors(crate::env::Platform::of(look), palette);
+            let doomed = crate::env::deleted_row_colors(crate::env::Platform::of(look), palette);
+            let fill = if new {
+                // A new row keeps its own fill under the selection
+                // too: its cell alone takes the selection's colour,
+                // and says where the keyboard is.
+                Some(fresh.fill)
+            } else if deleted && !ranged {
+                // As a new row: the row keeps its fill, and its cell
+                // alone says where the keyboard is. Not among several
+                // selected rows: there it takes the selection's fill
+                // with them, so the range reads as one, and its bar
+                // and its struck values still say it is marked.
+                Some(doomed.fill)
+            } else if lit_row {
+                Some(palette.window.lerp_to_gamma(palette.selection, 0.6))
+            } else {
+                row_fill(
+                    selected_row,
+                    response.hovered(),
+                    row % 2 == 1,
+                    look,
+                    palette,
+                )
+            };
+            if let Some(fill) = fill {
+                painter.rect_filled(rect, CornerRadius::ZERO, fill);
+            }
+            if !look.terminal {
+                let y = painter.round_to_pixel_center(rect.bottom() - hairline / 2.0);
+                painter.hline(rect.x_range(), y, Stroke::new(hairline, palette.surface));
+            }
+            for &col in &order {
+                let left = rect.left() + gutter + lefts[col];
+                let cell_rect =
+                    Rect::from_min_size(pos2(left, rect.top()), vec2(widths[col], row_height));
+                // The first column, and the row's mark before it, stand
+                // where the view begins while it is pinned.
+                let cell_rect = if col == 0 {
+                    let lead = Rect::from_min_max(
+                        pos2(rect.left() + shift, rect.top()),
+                        pos2(cell_rect.right() + shift, rect.bottom()),
+                    );
+                    if shift > 0.0 {
+                        // Over what scrolled under: the row's own fill.
+                        let under = fill.unwrap_or(palette.window);
+                        painter.rect_filled(lead, CornerRadius::ZERO, under);
+                        if !look.terminal {
+                            let y = painter.round_to_pixel_center(rect.bottom() - hairline / 2.0);
+                            painter.hline(
+                                lead.x_range(),
+                                y,
+                                Stroke::new(hairline, palette.surface),
                             );
                         }
-                        if look.terminal {
-                            if new {
-                                // The terminal's bar: 2 wide, and as far
-                                // in as the pane's focus ring is wide. The
-                                // ring is drawn over the row's start while
-                                // the grid has the keyboard, and the bar
-                                // stands beside it, as the design has them.
-                                let at = lead.min + vec2(focus::WIDTH, 0.0);
-                                let bar = Rect::from_min_size(at, vec2(2.0, row_height));
-                                painter.rect_filled(bar, CornerRadius::ZERO, fresh.bar);
-                            }
-                            let line = rect.center().y;
-                            if cursor_row {
-                                // The cursor: a bold accent block in the
-                                // gutter.
-                                let cursor = Text::one(look, TextRole::OGroup, "▌", palette.accent)
-                                    .layout(ui.ctx());
-                                cursor
-                                    .paint_center(&painter, pos2(lead.left() + GUTTER / 2.0, line));
-                            }
-                            if let Some(tone) = row_tone {
-                                // Beside the cursor: `~` on a changed row,
-                                // `!` when one of its cells is in trouble,
-                                // `+` on a new one.
-                                let sign = match mark {
-                                    RowMark::Trouble => "!",
-                                    RowMark::New => "+",
-                                    RowMark::Deleted => "-",
-                                    RowMark::None | RowMark::Changed => "~",
-                                };
-                                Text::one(look, TextRole::OGroup, sign, tone.color(palette))
-                                    .layout(ui.ctx())
-                                    .paint_center(
-                                        &painter,
-                                        pos2(lead.left() + GUTTER * 0.75, line),
-                                    );
-                            }
-                        }
-                        cell_rect.translate(vec2(shift, 0.0))
-                    } else {
-                        cell_rect
-                    };
-                    let edited = editing == Some(CellPos { row, col });
-                    if edited {
-                        output.editing_rect = Some(cell_rect);
-                    }
-                    if edited && let Some(editor) = editor.as_deref_mut() {
-                        // The field in place of the cell's text, in view
-                        // or not: it is drawn as long as it is open.
-                        editor(ui, cell_rect);
-                        continue;
-                    }
-                    if !ui.is_rect_visible(cell_rect) {
-                        continue;
-                    }
-                    let content = cell(row, col);
-                    let here = selection == Some(CellPos { row, col });
-                    if content.hint.is_some() || content.note.is_some() {
-                        // Where the cell shows: not the part of it that
-                        // scrolled under the pinned column.
-                        let mut seen = cell_rect;
-                        if pinned && col > 0 {
-                            let lead = rect.left() + shift + gutter + widths[0];
-                            seen.min.x = seen.min.x.max(lead);
-                        }
-                        if seen.is_positive() {
-                            // Hovered and no more: a click is the row's.
-                            let at = ui.interact(seen, id.with(("hint", row, col)), Sense::hover());
-                            if let Some(note) = &content.note {
-                                // It was asked for: said without waiting
-                                // for the pointer.
-                                at.show_tooltip_text(note.as_str());
-                            } else if let Some(hint) = &content.hint {
-                                let _ = at.on_hover_text(hint.as_str());
-                            }
-                        }
-                    }
-                    // What the cell's pending state tints it with. The
-                    // terminal draws a computed column as any other. A row
-                    // that is gone has no tint: nothing of it is pending.
-                    let tone = match content.mark {
-                        // A new row's cell keeps the row's own look: the
-                        // row says it is pending, and the outline below
-                        // says the warning.
-                        Mark::Doubt if held_back => None,
-                        Mark::Pending | Mark::Doubt | Mark::Saving => Some(Tone::Warning),
-                        Mark::Trouble => Some(Tone::Danger),
-                        Mark::Saved => Some(Tone::Success),
-                        Mark::None
-                        | Mark::Locked
-                        | Mark::Gone
-                        | Mark::Deleted
-                        | Mark::Deleting
-                        | Mark::Added
-                        | Mark::Unset => None,
-                    };
-                    let locked = content.mark == Mark::Locked && !look.terminal;
-                    // A cell with a warning keeps its lines under the cursor.
-                    let reversed_cursor = tone.is_none() && content.mark != Mark::Doubt;
-                    if here && lit && look.terminal && reversed_cursor && !edited {
-                        // Reverse video, as a terminal marks its cursor:
-                        // the accent behind, the text in the window's tone.
-                        painter.rect_filled(cell_rect, CornerRadius::ZERO, palette.accent);
-                        let reversed = written_in(palette, palette.window);
-                        let plain = Cell {
-                            style: Style::Plain,
-                            ..content
-                        };
-                        draw_cell(
-                            ui,
-                            &painter,
-                            cell_rect,
-                            &columns[col],
-                            &plain,
-                            look,
-                            &reversed,
+                        painter.vline(
+                            lead.right() - hairline / 2.0,
+                            lead.y_range(),
+                            Stroke::new(hairline, palette.outline),
                         );
-                        continue;
                     }
-                    // A tint stays under the cursor, which is then the
-                    // accent line alone: what is pending reads as pending
-                    // wherever the selection is.
-                    if let Some(tone) = tone {
-                        // What a save just wrote flashes in the colour a
-                        // saved new row does, a changed cell's too.
-                        let tint = match content.mark {
-                            Mark::Saved => fresh.saved,
-                            _ => tone.fill(look, palette),
-                        };
-                        painter.rect_filled(cell_rect, CornerRadius::ZERO, tint);
-                    } else if here && lit {
-                        painter.rect_filled(cell_rect, CornerRadius::ZERO, palette.selection);
-                    } else if locked {
-                        painter.rect_filled(cell_rect, CornerRadius::ZERO, palette.surface);
+                    if look.terminal {
+                        if new {
+                            // The terminal's bar: 2 wide, and as far
+                            // in as the pane's focus ring is wide. The
+                            // ring is drawn over the row's start while
+                            // the grid has the keyboard, and the bar
+                            // stands beside it, as the design has them.
+                            let at = lead.min + vec2(focus::WIDTH, 0.0);
+                            let bar = Rect::from_min_size(at, vec2(2.0, row_height));
+                            painter.rect_filled(bar, CornerRadius::ZERO, fresh.bar);
+                        }
+                        let line = rect.center().y;
+                        if cursor_row {
+                            // The cursor: a bold accent block in the
+                            // gutter.
+                            let cursor = Text::one(look, TextRole::OGroup, "▌", palette.accent)
+                                .layout(ui.ctx());
+                            cursor.paint_center(&painter, pos2(lead.left() + GUTTER / 2.0, line));
+                        }
+                        if let Some(tone) = row_tone {
+                            // Beside the cursor: `~` on a changed row,
+                            // `!` when one of its cells is in trouble,
+                            // `+` on a new one.
+                            let sign = match mark {
+                                RowMark::Trouble => "!",
+                                RowMark::New => "+",
+                                RowMark::Deleted => "-",
+                                RowMark::None | RowMark::Changed => "~",
+                            };
+                            Text::one(look, TextRole::OGroup, sign, tone.color(palette))
+                                .layout(ui.ctx())
+                                .paint_center(&painter, pos2(lead.left() + GUTTER * 0.75, line));
+                        }
                     }
-                    let height = cell_rect.height();
-                    match content.mark {
-                        Mark::Pending | Mark::Saving if !look.terminal => {
+                    cell_rect.translate(vec2(shift, 0.0))
+                } else {
+                    cell_rect
+                };
+                let edited = editing == Some(CellPos { row, col });
+                if edited {
+                    output.editing_rect = Some(cell_rect);
+                }
+                if edited && let Some(editor) = editor.as_deref_mut() {
+                    // The field in place of the cell's text, in view
+                    // or not: it is drawn as long as it is open.
+                    editor(ui, cell_rect);
+                    continue;
+                }
+                if !ui.is_rect_visible(cell_rect) {
+                    continue;
+                }
+                let content = cell(row, col);
+                let here = selection == Some(CellPos { row, col });
+                if content.hint.is_some() || content.note.is_some() {
+                    // Where the cell shows: not the part of it that
+                    // scrolled under the pinned column.
+                    let mut seen = cell_rect;
+                    if pinned && col > 0 {
+                        let lead = rect.left() + shift + gutter + widths[0];
+                        seen.min.x = seen.min.x.max(lead);
+                    }
+                    if seen.is_positive() {
+                        // Hovered and no more: a click is the row's.
+                        let at = ui.interact(seen, id.with(("hint", row, col)), Sense::hover());
+                        if let Some(note) = &content.note {
+                            // It was asked for: said without waiting
+                            // for the pointer.
+                            at.show_tooltip_text(note.as_str());
+                        } else if let Some(hint) = &content.hint {
+                            let _ = at.on_hover_text(hint.as_str());
+                        }
+                    }
+                }
+                // What the cell's pending state tints it with. The
+                // terminal draws a computed column as any other. A row
+                // that is gone has no tint: nothing of it is pending.
+                let tone = match content.mark {
+                    // A new row's cell keeps the row's own look: the
+                    // row says it is pending, and the outline below
+                    // says the warning.
+                    Mark::Doubt if held_back => None,
+                    Mark::Pending | Mark::Doubt | Mark::Saving => Some(Tone::Warning),
+                    Mark::Trouble => Some(Tone::Danger),
+                    Mark::Saved => Some(Tone::Success),
+                    Mark::None
+                    | Mark::Locked
+                    | Mark::Gone
+                    | Mark::Deleted
+                    | Mark::Deleting
+                    | Mark::Added
+                    | Mark::Unset => None,
+                };
+                let locked = content.mark == Mark::Locked && !look.terminal;
+                // A cell with a warning keeps its lines under the cursor.
+                let reversed_cursor = tone.is_none() && content.mark != Mark::Doubt;
+                if here && lit && look.terminal && reversed_cursor && !edited {
+                    // Reverse video, as a terminal marks its cursor:
+                    // the accent behind, the text in the window's tone.
+                    painter.rect_filled(cell_rect, CornerRadius::ZERO, palette.accent);
+                    let reversed = written_in(palette, palette.window);
+                    let plain = Cell {
+                        style: Style::Plain,
+                        ..content
+                    };
+                    draw_cell(
+                        ui,
+                        &painter,
+                        cell_rect,
+                        &columns[col],
+                        &plain,
+                        look,
+                        &reversed,
+                    );
+                    continue;
+                }
+                // A tint stays under the cursor, which is then the
+                // accent line alone: what is pending reads as pending
+                // wherever the selection is.
+                if let Some(tone) = tone {
+                    // What a save just wrote flashes in the colour a
+                    // saved new row does, a changed cell's too.
+                    let tint = match content.mark {
+                        Mark::Saved => fresh.saved,
+                        _ => tone.fill(look, palette),
+                    };
+                    painter.rect_filled(cell_rect, CornerRadius::ZERO, tint);
+                } else if here && lit {
+                    painter.rect_filled(cell_rect, CornerRadius::ZERO, palette.selection);
+                } else if locked {
+                    painter.rect_filled(cell_rect, CornerRadius::ZERO, palette.surface);
+                }
+                let height = cell_rect.height();
+                match content.mark {
+                    Mark::Pending | Mark::Saving if !look.terminal => {
+                        let bar = Rect::from_min_size(cell_rect.min, vec2(2.0, height));
+                        painter.rect_filled(bar, CornerRadius::ZERO, Tone::Warning.color(palette));
+                    }
+                    Mark::Doubt => {
+                        // A pending cell's bar, and an outline in its
+                        // tone, as a cell to fix has one in the
+                        // failure's.
+                        if !look.terminal && !held_back {
                             let bar = Rect::from_min_size(cell_rect.min, vec2(2.0, height));
                             painter.rect_filled(
                                 bar,
@@ -1028,245 +1052,233 @@ pub fn show<'a>(
                                 Tone::Warning.color(palette),
                             );
                         }
-                        Mark::Doubt => {
-                            // A pending cell's bar, and an outline in its
-                            // tone, as a cell to fix has one in the
-                            // failure's.
-                            if !look.terminal && !held_back {
-                                let bar = Rect::from_min_size(cell_rect.min, vec2(2.0, height));
-                                painter.rect_filled(
-                                    bar,
-                                    CornerRadius::ZERO,
-                                    Tone::Warning.color(palette),
-                                );
-                            }
-                            painter.rect_stroke(
-                                cell_rect,
-                                CornerRadius::ZERO,
-                                Stroke::new(1.0, Tone::Warning.color(palette)),
-                                StrokeKind::Inside,
-                            );
-                        }
-                        Mark::Trouble => {
-                            painter.rect_stroke(
-                                cell_rect,
-                                CornerRadius::ZERO,
-                                Stroke::new(1.0, Tone::Danger.color(palette)),
-                                StrokeKind::Inside,
-                            );
-                        }
-                        _ => {}
-                    }
-                    if col == 0 && !look.terminal {
-                        // The row's bar, over what its first cell is filled
-                        // with: of its pending cells' tone, else the
-                        // accent of a selected row whose cell is not lit.
-                        let color = match row_tone {
-                            Some(_) if new => Some(fresh.bar),
-                            Some(_) if deleted => Some(doomed.bar),
-                            Some(tone) => Some(tone.color(palette)),
-                            None => (selected_row && !lit_row).then_some(palette.accent),
-                        };
-                        if let Some(color) = color {
-                            let bar = Rect::from_min_size(cell_rect.min, vec2(3.0, height));
-                            painter.rect_filled(bar, CornerRadius::ZERO, color);
-                        }
-                    }
-                    // The text's colours: dim in a row that is gone, a
-                    // pending cell's tone in the terminal, a changed row's
-                    // on its key, and a computed column a step quieter.
-                    let written = match (tone, row_tone) {
-                        _ if content.mark == Mark::Gone => written_in(palette, palette.dim),
-                        // A row marked for deletion: every value in the
-                        // one tone its style gives, the key's too.
-                        _ if matches!(content.mark, Mark::Deleted | Mark::Deleting) => {
-                            written_in(palette, doomed.text)
-                        }
-                        // A new row's marker in the row's tone; the
-                        // terminal writes it dimmed, as it does what a
-                        // cell will be filled with.
-                        _ if content.mark == Mark::Added && !look.terminal => {
-                            written_in(palette, fresh.marker)
-                        }
-                        _ if matches!(content.mark, Mark::Unset | Mark::Added) => {
-                            written_in(palette, palette.dim)
-                        }
-                        (Some(tone), _) if look.terminal && tone != Tone::Success => {
-                            written_in(palette, tone.color(palette))
-                        }
-                        (None, Some(tone)) if col == 0 && columns[col].key && !look.terminal => {
-                            written_in(palette, tone.color(palette))
-                        }
-                        _ if locked => Palette {
-                            text: palette.secondary,
-                            ..*palette
-                        },
-                        _ => *palette,
-                    };
-                    // A cell being saved keeps room at its right for the
-                    // spinner.
-                    let text_rect = if content.mark == Mark::Saving {
-                        let side = 12.0;
-                        let at = Rect::from_center_size(
-                            pos2(cell_rect.right() - pad - side / 2.0, cell_rect.center().y),
-                            vec2(side, side),
-                        );
-                        crate::ui::states::spinner(ui, at, Tone::Warning.color(palette), palette);
-                        // The text ends 6 short of it.
-                        let right = at.left() - 6.0 + pad;
-                        Rect::from_min_max(cell_rect.min, pos2(right, cell_rect.bottom()))
-                    } else {
-                        cell_rect
-                    };
-                    draw_cell(
-                        ui,
-                        &painter,
-                        text_rect,
-                        &columns[col],
-                        &content,
-                        look,
-                        &written,
-                    );
-                    // A cell edited in an editor of its own keeps its
-                    // value and the cursor's line, lit or not: the line
-                    // says which cell the editor is for.
-                    if (here && lit) || edited {
-                        // Inside the cell: nothing the grid scrolls under
-                        // cuts it.
                         painter.rect_stroke(
                             cell_rect,
                             CornerRadius::ZERO,
-                            Stroke::new(2.0, palette.accent),
-                            StrokeKind::Inside,
-                        );
-                        if content.mark == Mark::Doubt {
-                            // The cursor's line covers the outline of the
-                            // same rect: the warning's goes inside it.
-                            painter.rect_stroke(
-                                cell_rect.shrink(2.0),
-                                CornerRadius::ZERO,
-                                Stroke::new(1.0, Tone::Warning.color(palette)),
-                                StrokeKind::Inside,
-                            );
-                        }
-                    } else if here && (col > 0 || (new && !look.terminal)) {
-                        // The row is selected; a cell past the first is
-                        // marked too, for the keys that act on one cell.
-                        // A new row's first is as well: the row keeps its
-                        // own fill and bar, and nothing else would say it
-                        // is selected. The line stands clear of that bar.
-                        let clear = if col == 0 { 3.0 } else { 0.0 };
-                        let marked = cell_rect.with_min_x(cell_rect.left() + clear);
-                        painter.rect_stroke(
-                            marked.shrink(1.0),
-                            CornerRadius::same(look.radius.min(3)),
-                            Stroke::new(1.5, palette.accent),
+                            Stroke::new(1.0, Tone::Warning.color(palette)),
                             StrokeKind::Inside,
                         );
                     }
+                    Mark::Trouble => {
+                        painter.rect_stroke(
+                            cell_rect,
+                            CornerRadius::ZERO,
+                            Stroke::new(1.0, Tone::Danger.color(palette)),
+                            StrokeKind::Inside,
+                        );
+                    }
+                    _ => {}
                 }
-            });
-
-            // A row scrolled out of view is not built, and its cell's
-            // editor must be drawn all the same to keep the keyboard: where
-            // the cell is, until the grid has scrolled back to it.
-            if let (Some(at), None) = (editing, output.editing_rect)
-                && at.row < row_count
-                && at.col < widths.len()
-            {
-                let place = pos2(
-                    origin.x + gutter + lefts[at.col],
-                    origin.y + header_height + at.row as f32 * row_height,
+                if col == 0 && !look.terminal {
+                    // The row's bar, over what its first cell is filled
+                    // with: of its pending cells' tone, else the
+                    // accent of a selected row whose cell is not lit.
+                    let color = match row_tone {
+                        Some(_) if new => Some(fresh.bar),
+                        Some(_) if deleted => Some(doomed.bar),
+                        Some(tone) => Some(tone.color(palette)),
+                        None => (selected_row && !lit_row).then_some(palette.accent),
+                    };
+                    if let Some(color) = color {
+                        let bar = Rect::from_min_size(cell_rect.min, vec2(3.0, height));
+                        painter.rect_filled(bar, CornerRadius::ZERO, color);
+                    }
+                }
+                // The text's colours: dim in a row that is gone, a
+                // pending cell's tone in the terminal, a changed row's
+                // on its key, and a computed column a step quieter.
+                let written = match (tone, row_tone) {
+                    _ if content.mark == Mark::Gone => written_in(palette, palette.dim),
+                    // A row marked for deletion: every value in the
+                    // one tone its style gives, the key's too.
+                    _ if matches!(content.mark, Mark::Deleted | Mark::Deleting) => {
+                        written_in(palette, doomed.text)
+                    }
+                    // A new row's marker in the row's tone; the
+                    // terminal writes it dimmed, as it does what a
+                    // cell will be filled with.
+                    _ if content.mark == Mark::Added && !look.terminal => {
+                        written_in(palette, fresh.marker)
+                    }
+                    _ if matches!(content.mark, Mark::Unset | Mark::Added) => {
+                        written_in(palette, palette.dim)
+                    }
+                    (Some(tone), _) if look.terminal && tone != Tone::Success => {
+                        written_in(palette, tone.color(palette))
+                    }
+                    (None, Some(tone)) if col == 0 && columns[col].key && !look.terminal => {
+                        written_in(palette, tone.color(palette))
+                    }
+                    _ if locked => Palette {
+                        text: palette.secondary,
+                        ..*palette
+                    },
+                    _ => *palette,
+                };
+                // A cell being saved keeps room at its right for the
+                // spinner.
+                let text_rect = if content.mark == Mark::Saving {
+                    let side = 12.0;
+                    let at = Rect::from_center_size(
+                        pos2(cell_rect.right() - pad - side / 2.0, cell_rect.center().y),
+                        vec2(side, side),
+                    );
+                    crate::ui::states::spinner(ui, at, Tone::Warning.color(palette), palette);
+                    // The text ends 6 short of it.
+                    let right = at.left() - 6.0 + pad;
+                    Rect::from_min_max(cell_rect.min, pos2(right, cell_rect.bottom()))
+                } else {
+                    cell_rect
+                };
+                draw_cell(
+                    ui,
+                    &painter,
+                    text_rect,
+                    &columns[col],
+                    &content,
+                    look,
+                    &written,
                 );
-                let rect = Rect::from_min_size(place, vec2(widths[at.col], row_height));
-                output.editing_rect = Some(rect);
-                if let Some(editor) = editor {
-                    editor(ui, rect);
-                }
-            }
-
-            // The header, painted over the rows at the top of the visible
-            // area so it stays put while rows scroll under it. Its widgets
-            // come after the rows', so they sit on top.
-            let top = ui.clip_rect().top().max(origin.y);
-            let header = Rect::from_min_size(pos2(origin.x, top), vec2(full, header_height));
-            let painter = ui.painter().clone();
-            let header_fill = if look.terminal {
-                palette.window
-            } else {
-                palette.panel
-            };
-            painter.rect_filled(header, CornerRadius::ZERO, header_fill);
-            let y = painter.round_to_pixel_center(header.bottom() - hairline / 2.0);
-            painter.hline(header.x_range(), y, Stroke::new(hairline, palette.outline));
-            for &col in &order {
-                let column = &columns[col];
-                let left = origin.x + gutter + lefts[col];
-                let rect = Rect::from_min_size(pos2(left, top), vec2(widths[col], header_height));
-                // The pinned column's header stands with its cells, over
-                // the headers that scrolled under it.
-                let rect = if col == 0 && shift > 0.0 {
-                    let lead = Rect::from_min_max(
-                        pos2(origin.x + shift, top),
-                        pos2(rect.right() + shift, header.bottom()),
-                    );
-                    painter.rect_filled(lead, CornerRadius::ZERO, header_fill);
-                    painter.hline(lead.x_range(), y, Stroke::new(hairline, palette.outline));
-                    painter.vline(
-                        lead.right() - hairline / 2.0,
-                        lead.y_range(),
-                        Stroke::new(hairline, palette.outline),
-                    );
-                    rect.translate(vec2(shift, 0.0))
-                } else {
-                    rect
-                };
-                // A header that sorts nothing still takes the pointer's
-                // clicks, and drops them: a hover-only header would let
-                // them through to a row scrolled under it.
-                let (sense, kind) = if column.sortable {
-                    (Sense::click(), WidgetType::Button)
-                } else {
-                    (Sense::CLICK, WidgetType::Label)
-                };
-                let response = ui.interact(rect, id.with(("header", col)), sense);
-                focus::hint(ui, &response, rect, focus::Ring::Inset { radius: 0 });
-                // Column names come from the server: nothing hidden in them.
-                let name = display_safe(column.name);
-                response.widget_info(|| WidgetInfo::labeled(kind, true, &*name));
-                if response.clicked() && column.sortable {
-                    output.sort_clicked = Some(col);
-                }
-                if response.hovered() && column.sortable {
-                    painter.rect_filled(
-                        rect.shrink2(vec2(0.0, hairline)),
+                // A cell edited in an editor of its own keeps its
+                // value and the cursor's line, lit or not: the line
+                // says which cell the editor is for.
+                if (here && lit) || edited {
+                    // Inside the cell: nothing the grid scrolls under
+                    // cuts it.
+                    painter.rect_stroke(
+                        cell_rect,
                         CornerRadius::ZERO,
-                        palette.surface,
+                        Stroke::new(2.0, palette.accent),
+                        StrokeKind::Inside,
                     );
-                }
-                draw_header(ui, &painter, rect, column, pad, look, palette);
-                let handle = Rect::from_min_max(
-                    pos2(rect.right() - HANDLE_WIDTH / 2.0, top),
-                    pos2(rect.right() + HANDLE_WIDTH / 2.0, top + header_height),
-                );
-                // Only the pointer drags it: no stop for the Tab key.
-                let drag = ui.interact(handle, id.with(("resize", col)), Sense::DRAG);
-                if drag.hovered() || drag.dragged() {
-                    ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeColumn);
-                    painter.vline(
-                        rect.right(),
-                        header.y_range(),
-                        Stroke::new(hairline, palette.outline),
+                    if content.mark == Mark::Doubt {
+                        // The cursor's line covers the outline of the
+                        // same rect: the warning's goes inside it.
+                        painter.rect_stroke(
+                            cell_rect.shrink(2.0),
+                            CornerRadius::ZERO,
+                            Stroke::new(1.0, Tone::Warning.color(palette)),
+                            StrokeKind::Inside,
+                        );
+                    }
+                } else if here && (col > 0 || (new && !look.terminal)) {
+                    // The row is selected; a cell past the first is
+                    // marked too, for the keys that act on one cell.
+                    // A new row's first is as well: the row keeps its
+                    // own fill and bar, and nothing else would say it
+                    // is selected. The line stands clear of that bar.
+                    let clear = if col == 0 { 3.0 } else { 0.0 };
+                    let marked = cell_rect.with_min_x(cell_rect.left() + clear);
+                    painter.rect_stroke(
+                        marked.shrink(1.0),
+                        CornerRadius::same(look.radius.min(3)),
+                        Stroke::new(1.5, palette.accent),
+                        StrokeKind::Inside,
                     );
                 }
-                if drag.dragged() {
-                    widths[col] = (widths[col] + drag.drag_delta().x).max(MIN_WIDTH);
-                    keep = true;
-                }
             }
-            origin
         });
 
+        // A row scrolled out of view is not built, and its cell's
+        // editor must be drawn all the same to keep the keyboard: where
+        // the cell is, until the grid has scrolled back to it.
+        if let (Some(at), None) = (editing, output.editing_rect)
+            && at.row < row_count
+            && at.col < widths.len()
+        {
+            let place = pos2(
+                origin.x + gutter + lefts[at.col],
+                origin.y + header_height + at.row as f32 * row_height,
+            );
+            let rect = Rect::from_min_size(place, vec2(widths[at.col], row_height));
+            output.editing_rect = Some(rect);
+            if let Some(editor) = editor {
+                editor(ui, rect);
+            }
+        }
+
+        // The header, painted over the rows at the top of the visible
+        // area so it stays put while rows scroll under it. Its widgets
+        // come after the rows', so they sit on top.
+        let top = ui.clip_rect().top().max(origin.y);
+        let header = Rect::from_min_size(pos2(origin.x, top), vec2(full, header_height));
+        let painter = ui.painter().clone();
+        let header_fill = if look.terminal {
+            palette.window
+        } else {
+            palette.panel
+        };
+        painter.rect_filled(header, CornerRadius::ZERO, header_fill);
+        let y = painter.round_to_pixel_center(header.bottom() - hairline / 2.0);
+        painter.hline(header.x_range(), y, Stroke::new(hairline, palette.outline));
+        for &col in &order {
+            let column = &columns[col];
+            let left = origin.x + gutter + lefts[col];
+            let rect = Rect::from_min_size(pos2(left, top), vec2(widths[col], header_height));
+            // The pinned column's header stands with its cells, over
+            // the headers that scrolled under it.
+            let rect = if col == 0 && shift > 0.0 {
+                let lead = Rect::from_min_max(
+                    pos2(origin.x + shift, top),
+                    pos2(rect.right() + shift, header.bottom()),
+                );
+                painter.rect_filled(lead, CornerRadius::ZERO, header_fill);
+                painter.hline(lead.x_range(), y, Stroke::new(hairline, palette.outline));
+                painter.vline(
+                    lead.right() - hairline / 2.0,
+                    lead.y_range(),
+                    Stroke::new(hairline, palette.outline),
+                );
+                rect.translate(vec2(shift, 0.0))
+            } else {
+                rect
+            };
+            // A header that sorts nothing still takes the pointer's
+            // clicks, and drops them: a hover-only header would let
+            // them through to a row scrolled under it.
+            let (sense, kind) = if column.sortable {
+                (Sense::click(), WidgetType::Button)
+            } else {
+                (Sense::CLICK, WidgetType::Label)
+            };
+            let response = ui.interact(rect, id.with(("header", col)), sense);
+            focus::hint(ui, &response, rect, focus::Ring::Inset { radius: 0 });
+            // Column names come from the server: nothing hidden in them.
+            let name = display_safe(column.name);
+            response.widget_info(|| WidgetInfo::labeled(kind, true, &*name));
+            if response.clicked() && column.sortable {
+                output.sort_clicked = Some(col);
+            }
+            if response.hovered() && column.sortable {
+                painter.rect_filled(
+                    rect.shrink2(vec2(0.0, hairline)),
+                    CornerRadius::ZERO,
+                    palette.surface,
+                );
+            }
+            draw_header(ui, &painter, rect, column, pad, look, palette);
+            let handle = Rect::from_min_max(
+                pos2(rect.right() - HANDLE_WIDTH / 2.0, top),
+                pos2(rect.right() + HANDLE_WIDTH / 2.0, top + header_height),
+            );
+            // Only the pointer drags it: no stop for the Tab key.
+            let drag = ui.interact(handle, id.with(("resize", col)), Sense::DRAG);
+            if drag.hovered() || drag.dragged() {
+                ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeColumn);
+                painter.vline(
+                    rect.right(),
+                    header.y_range(),
+                    Stroke::new(hairline, palette.outline),
+                );
+            }
+            if drag.dragged() {
+                widths[col] = (widths[col] + drag.drag_delta().x).max(MIN_WIDTH);
+                keep = true;
+            }
+        }
+        origin
+    });
+
     // Columns the viewport cuts off entirely. Their pill and the fade under
     // it cover the viewport's end, so a column that shows less than the
     // narrowest one before the fade is as much out of sight.
@@ -1392,6 +1404,7 @@ pub fn show<'a>(
             },
         );
     });
+    ui.data_mut(|data| data.insert_temp(height_id, (row_height, scroll.state.offset.y)));
     let viewport = scroll.inner_rect;
     output.body = Some(Rect::from_min_max(
         pos2(viewport.left(), viewport.top() + header_height),
@@ -1415,9 +1428,14 @@ fn draw_header(
     // The name over its type line, the pair centred above the rule.
     let name_role = TextRole::pick(look, TextRole::UiBodySemibold, TextRole::OGroup);
     let type_role = TextRole::pick(look, TextRole::ColumnType, TextRole::OColumnType);
+    // Without the types' line the name is centred alone.
     let (name_line, type_line) = (
         name_role.row_height(ui.ctx(), look.faces),
-        type_role.row_height(ui.ctx(), look.faces),
+        if look.type_line {
+            type_role.row_height(ui.ctx(), look.faces)
+        } else {
+            0.0
+        },
     );
     let top = rect.top() + (rect.height() - 1.0 - name_line - type_line) / 2.0;
     let (name_y, type_y) = (top + name_line / 2.0, top + name_line + type_line / 2.0);
@@ -1519,17 +1537,19 @@ fn draw_header(
             Rect::from_center_size(pos2(name_x + name_width + 9.5, name_y), vec2(11.0, 11.0)),
         );
     }
-    paint(
-        &clip,
-        ui,
-        type_role,
-        &type_text,
-        palette.dim,
-        type_x,
-        type_y,
-        false,
-        look,
-    );
+    if look.type_line {
+        paint(
+            &clip,
+            ui,
+            type_role,
+            &type_text,
+            palette.dim,
+            type_x,
+            type_y,
+            false,
+            look,
+        );
+    }
 }
 
 /// Space inside a chip, at each side of its text.
@@ -3224,7 +3244,7 @@ mod tests {
     #[test]
     fn initial_widths_fit_the_measured_text() {
         let columns = columns();
-        let widths = initial_widths(&columns, 3, (6.0, 0.0), &width, &mut |_, col| Cell {
+        let widths = initial_widths(&columns, 3, (6.0, 0.0, true), &width, &mut |_, col| Cell {
             text: if col == 0 {
                 "1628910071209526786".into()
             } else {
@@ -3239,7 +3259,7 @@ mod tests {
     #[test]
     fn initial_widths_fit_content_within_limits() {
         let columns = columns();
-        let widths = initial_widths(&columns, 3, (6.0, 0.0), &width, &mut |_, col| Cell {
+        let widths = initial_widths(&columns, 3, (6.0, 0.0, true), &width, &mut |_, col| Cell {
             text: if col == 0 {
                 "1".into()
             } else {
diff --git a/src/ui/states.rs b/src/ui/states.rs
index da95f27..8681777 100644
--- a/src/ui/states.rs
+++ b/src/ui/states.rs
@@ -675,9 +675,9 @@ pub fn skeleton(ui: &Ui, rect: Rect, look: &Look, palette: &Palette) {
     let fixed = [32.0, 76.0, 40.0];
     let shares = [1.0, 0.7, 0.85, 0.6];
     let fill = palette.surface.gamma_multiply(0.7);
-    let rows = ((rect.height() / look.grid_row) as usize).min(6);
+    let rows = ((rect.height() / look.row()) as usize).min(6);
     for row in 0..rows {
-        let center = rect.top() + look.grid_row * (row as f32 + 0.5);
+        let center = rect.top() + look.row() * (row as f32 + 0.5);
         // The fixed bars, then the last one in the room that is left.
         let mut x = rect.left() + 12.0;
         let rest = (rect.right() - 12.0 - x - fixed.iter().sum::<f32>() - 3.0 * GAP).max(0.0);
```

- [ ] **Step 5: Draw the two grids, and the loading rows, in the settings' look**

`Fit` gains the types' line: a grid with it on and one with it off are fitted apart.

```diff
diff --git a/src/ui/data_view.rs b/src/ui/data_view.rs
index 18b5f65..551c34e 100644
--- a/src/ui/data_view.rs
+++ b/src/ui/data_view.rs
@@ -870,6 +870,8 @@ pub struct Fit {
     pub grouped: bool,
     /// Values of a closed set as tags, which pad their text.
     pub value_tags: bool,
+    /// The header's types' line, which a column is as wide as.
+    pub column_types: bool,
 }
 
 impl Fit {
@@ -878,6 +880,7 @@ impl Fit {
             full_precision: workspace.full_precision,
             grouped: settings.group_digits,
             value_tags: settings.value_tags,
+            column_types: settings.column_types,
         }
     }
 }
@@ -1343,6 +1346,9 @@ pub fn show(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId, object_tab: TabId)
     let locale = app.locale;
     let palette = app.palette;
     let look = app.look;
+    // The grid's rows as dense and as striped, and its header as tall, as
+    // the settings have them.
+    let grid_look = look.with_grid(&app.settings);
     // An editor that is open has the keyboard, unless a dialog has it.
     let hold = app.dialog.is_none();
     let editor_keys = cell_editor::Keys::of(&app.keymap, app.layout());
@@ -1600,7 +1606,7 @@ pub fn show(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId, object_tab: TabId)
             object.selection.and_then(placed),
             keys,
             &palette,
-            &look,
+            &grid_look,
             &|place| match order.row(place) {
                 Some(row) => match crate::edit::new_id(row) {
                     Some(id) => grid::Row {
@@ -1784,7 +1790,7 @@ pub fn show(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId, object_tab: TabId)
         if order.is_empty() && !lasted {
             // The headers stay: the columns are still worth reading.
             let under = Rect::from_min_max(
-                pos2(area.left(), area.top() + grid::header_height(&look)),
+                pos2(area.left(), area.top() + grid::header_height(&grid_look)),
                 area.max,
             );
             empty_rows(
@@ -1802,7 +1808,7 @@ pub fn show(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId, object_tab: TabId)
             states::progress(ui, area.x_range(), area.top(), &palette);
         }
         let rows = Rect::from_min_max(pos2(area.left(), area.top() + 2.0), area.max);
-        states::skeleton(ui, rows, &look, &palette);
+        states::skeleton(ui, rows, &grid_look, &palette);
     }
     // A fetch that has lasted, over whatever is up: a refresh and the next
     // page keep the page they replace on screen.
diff --git a/src/ui/sql_results.rs b/src/ui/sql_results.rs
index 03a00b1..1d5e19b 100644
--- a/src/ui/sql_results.rs
+++ b/src/ui/sql_results.rs
@@ -193,6 +193,9 @@ struct Place<'a> {
     blocked: Option<Blocked<'a>>,
     /// Whether the arrow keys move in the result's grid.
     keys: bool,
+    /// The look the result's grid is drawn in: the rows as dense, as
+    /// striped and the header as tall as the settings have them.
+    grid: Look,
 }
 
 fn draw(app: &App, ui: &mut Ui, tab: ConnTabId, id: TabId, actions: &mut Vec<Action>) {
@@ -216,6 +219,7 @@ fn draw(app: &App, ui: &mut Ui, tab: ConnTabId, id: TabId, actions: &mut Vec<Act
         driver: workspace.driver,
         blocked: blocked(workspace, sql),
         keys: workspace.pane == crate::model::Pane::Grid,
+        grid: look.with_grid(&app.settings),
     };
     let state = state(sql);
     let pane = ui.max_rect();
@@ -1580,6 +1584,7 @@ fn results(ui: &mut Ui, run: &SqlRun, place: &Place<'_>, env: &Env<'_>, actions:
         sql,
         fit,
         keys,
+        grid: grid_look,
         ..
     } = *place;
     let Some((columns, rows, truncated)) = sql.shown_rows() else {
@@ -1689,7 +1694,7 @@ fn results(ui: &mut Ui, run: &SqlRun, place: &Place<'_>, env: &Env<'_>, actions:
         sql.selection,
         keys,
         palette,
-        look,
+        &grid_look,
         // A result has nothing pending and is never edited: no row is
         // marked, and no cell has an editor.
         &|row| grid::Row {
@@ -1728,7 +1733,7 @@ fn results(ui: &mut Ui, run: &SqlRun, place: &Place<'_>, env: &Env<'_>, actions:
     }
     if rows.is_empty() {
         let under = Rect::from_min_max(
-            pos2(area.left(), area.top() + grid::header_height(look)),
+            pos2(area.left(), area.top() + grid::header_height(&grid_look)),
             area.max,
         );
         let said = env.said(|_| gettext(locale, "No rows").into_owned());
```

- [ ] **Step 6: Run the tests to see them pass**

Run: `cargo test --locked --lib appearance_settings_tests`
Expected: PASS (5 tests). Then the three checks (the grid's own `forget` test now covers the row-height record, through `remembered`).

- [ ] **Step 7: Commit**

```bash
git add src/theme.rs src/ui/grid.rs src/ui/states.rs src/ui/data_view.rs src/ui/sql_results.rs src/ui/mod.rs src/ui/appearance_settings_tests.rs
git commit -m "Draw the grids as dense, as striped and with as many header lines as the settings say"
```

---

### Task 3: The connection bar, tinted or not

**Files:**
- Modify: `src/env.rs`, `src/ui/workspace.rs`, `src/ui/env_tests.rs`

`env_colors()` may be called only from the environment's surfaces (`only_the_environment_surfaces_ask_for_its_colours`), so the tests go in `src/env.rs` and `src/ui/env_tests.rs`.

- [ ] **Step 1: Write the failing tests**

```diff
diff --git a/src/env.rs b/src/env.rs
index 4956598..d4bacc0 100644
--- a/src/env.rs
+++ b/src/env.rs
@@ -871,4 +871,30 @@ mod tests {
         }
         assert!(found.is_empty(), "{}", found.join("\n"));
     }
+
+    #[test]
+    fn stripe_only_leaves_the_bar_untinted_but_production_tinted() {
+        use crate::settings::EnvBar;
+        for look in [Look::macos(), Look::omarchy()] {
+            let platform = Platform::of(&look);
+            let palette = crate::theme::Palette::dark();
+            for env in [
+                Environment::Local,
+                Environment::Dev,
+                Environment::Staging,
+                Environment::Production,
+                Environment::None,
+            ] {
+                let colors = env_colors(env, platform, &palette);
+                let tint = (colors.bar_bg(), colors.bar_border());
+                assert_eq!(bar_colors(env, EnvBar::Tint, platform, &palette), tint);
+                let label = bar_colors(env, EnvBar::Label, platform, &palette);
+                if env == Environment::Production {
+                    assert_eq!(label, tint, "{}", look.name);
+                } else {
+                    assert_eq!(label, (palette.panel, palette.outline), "{env:?}");
+                }
+            }
+        }
+    }
 }
diff --git a/src/ui/env_tests.rs b/src/ui/env_tests.rs
index 60d8902..d3f5b7e 100644
--- a/src/ui/env_tests.rs
+++ b/src/ui/env_tests.rs
@@ -251,3 +251,46 @@ fn the_connection_dialog_takes_the_chosen_environments_colours() {
         }
     }
 }
+
+#[test]
+fn the_connection_bar_follows_the_setting() {
+    use crate::settings::EnvBar;
+    for look in [Look::macos(), Look::omarchy()] {
+        let mut harness = Harness::new();
+        harness.set_look(look);
+        let tab = harness.connect_fake();
+        let env = harness.app.workspace(tab).unwrap().environment;
+        let platform = Platform::of(&look);
+        let colors = env_colors(env, platform, &harness.app.palette);
+        let tint = colors.bar_bg();
+        let wide = |harness: &Harness, color| {
+            let width = harness.size.x - 1.0;
+            harness
+                .fills
+                .iter()
+                .any(|(rect, fill)| *fill == color && rect.width() >= width && rect.top() < 60.0)
+        };
+        // The environment's own colour: the macOS stripe, the terminal's
+        // label.
+        let own = |harness: &Harness| {
+            harness.fills.iter().any(|(rect, fill)| {
+                rect.top() < 60.0 && (*fill == colors.base() || *fill == colors.badge_bg())
+            })
+        };
+        harness.settle();
+        assert!(wide(&harness, tint), "{}", look.name);
+        assert!(own(&harness), "{}", look.name);
+        let mut settings = harness.app.settings.clone();
+        settings.env_bar = EnvBar::Label;
+        harness.app.apply_settings(settings);
+        harness.settle();
+        assert!(!wide(&harness, tint), "{}", look.name);
+        assert!(own(&harness), "{} keeps its stripe or label", look.name);
+        // Nor any line drawn over the tint: they go with the bar's fill.
+        assert!(
+            !harness.strokes.contains(&colors.bar_border()),
+            "{}",
+            look.name
+        );
+    }
+}
```

- [ ] **Step 2: Run the tests to see them fail**

Run: `cargo test --locked --lib -- bar_untinted connection_bar_follows`
Expected:

```
error[E0425]: cannot find function `bar_colors` in this scope
   --> src/env.rs
```

- [ ] **Step 3: Add `bar_colors` and draw the bar with it**

`EnvColors::on_bar` puts the bar's fill and rule into the colours the bar draws with, so the lines over it (the macOS buttons' and chips' hairlines, the Omarchy `≡ conn` box) go with the fill.

```diff
diff --git a/src/env.rs b/src/env.rs
index d4bacc0..b608ac9 100644
--- a/src/env.rs
+++ b/src/env.rs
@@ -170,6 +170,23 @@ impl EnvColors {
     pub fn bar_border(&self) -> Color32 {
         self.bar_border
     }
+
+    /// The colours with the bar's fill and rule as `bar` has them
+    /// ([`bar_colors`]): the lines drawn over the bar go with its fill.
+    pub fn on_bar(
+        self,
+        env: Environment,
+        bar: crate::settings::EnvBar,
+        platform: Platform,
+        palette: &Palette,
+    ) -> Self {
+        let (bar_bg, bar_border) = bar_colors(env, bar, platform, palette);
+        Self {
+            bar_bg,
+            bar_border,
+            ..self
+        }
+    }
 }
 
 /// An environment's colours on `platform` with `palette`. macOS and Windows
@@ -183,6 +200,25 @@ pub fn env_colors(env: Environment, platform: Platform, palette: &Palette) -> En
     }
 }
 
+/// The connection bar's fill and the rule under it, as `bar` says: the
+/// environment's tint and its line, or the panel and its outline with the
+/// stripe or the label alone to say the environment. Production keeps its
+/// tint whatever the setting says.
+pub fn bar_colors(
+    env: Environment,
+    bar: crate::settings::EnvBar,
+    platform: Platform,
+    palette: &Palette,
+) -> (Color32, Color32) {
+    use crate::settings::EnvBar;
+    match (bar, env) {
+        (EnvBar::Tint, _) | (EnvBar::Label, Environment::Production) => {
+            bar_tint(env, platform, palette)
+        }
+        (EnvBar::Label, _) => (palette.panel, palette.outline),
+    }
+}
+
 /// `env`'s bar tint and the line a bar draws over it.
 fn bar_tint(env: Environment, platform: Platform, palette: &Palette) -> (Color32, Color32) {
     let colors = env_colors(env, platform, palette);
diff --git a/src/ui/workspace.rs b/src/ui/workspace.rs
index fcd825d..9d81420 100644
--- a/src/ui/workspace.rs
+++ b/src/ui/workspace.rs
@@ -870,7 +870,11 @@ fn top_bar(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId) {
     let Some(info) = bar_info(app, tab) else {
         return;
     };
-    let env = crate::env::env_colors(info.env, crate::env::Platform::of(&look), &palette);
+    let platform = crate::env::Platform::of(&look);
+    let env = crate::env::env_colors(info.env, platform, &palette);
+    // The bar's fill and rule as the settings have them, and the lines
+    // over it with them.
+    let env = env.on_bar(info.env, app.settings.env_bar, platform, &palette);
     let (tint, border) = (env.bar_bg(), env.bar_border());
     // macOS: the bar is the title bar, beside the window buttons.
     let zoom = ui.ctx().zoom_factor();
```

- [ ] **Step 4: Run the tests to see them pass**

Run: `cargo test --locked --lib -- bar_untinted connection_bar_follows`
Expected: PASS (2 tests). Then the three checks.

- [ ] **Step 5: Commit**

```bash
git add src/env.rs src/ui/workspace.rs src/ui/env_tests.rs
git commit -m "Leave the connection bar untinted where the settings say so, production aside"
```

---

### Task 4: Each connection's table list

**Files:**
- Modify: `src/paths.rs`, `src/backend.rs`, `src/app.rs`, `src/ui/appearance_settings_tests.rs`

- [ ] **Step 1: Write the failing tests**

```diff
diff --git a/src/ui/appearance_settings_tests.rs b/src/ui/appearance_settings_tests.rs
index 5fdb9a1..25b5d8e 100644
--- a/src/ui/appearance_settings_tests.rs
+++ b/src/ui/appearance_settings_tests.rs
@@ -3,8 +3,9 @@
 
 use egui::accesskit::Role;
 
+use crate::backend::{Command, StateFile};
 use crate::model::{Action, ConnTabId, TabId};
-use crate::settings::{RowHeight, Settings};
+use crate::settings::{RowHeight, Settings, TableList};
 use crate::testing::{Harness, bounds};
 use crate::theme::Look;
 
@@ -203,3 +204,144 @@ fn without_column_types_the_header_is_one_line() {
         assert!(!typed(&mut harness), "{}", look.name);
     }
 }
+
+#[test]
+fn the_table_list_default_applies_to_connections_without_a_choice() {
+    let mut harness = Harness::new();
+    let tab = harness.connect_fake();
+    let flat = |harness: &Harness, tab| harness.app.workspace(tab).unwrap().tree.flat;
+    assert!(!flat(&harness, tab), "a tree until set");
+    change(&mut harness, |settings| {
+        settings.table_list = TableList::Flat
+    });
+    assert!(flat(&harness, tab), "the open connection follows");
+    // The sidebar's toggle is the connection's choice, and saved.
+    harness.app.apply(Action::ToggleFlatTree(tab));
+    assert!(!flat(&harness, tab));
+    let conn = harness.app.workspace(tab).unwrap().conn_id.clone();
+    assert_eq!(harness.app.table_lists.get(&conn.0), Some(&TableList::Tree));
+    assert!(harness.app.backend.sent.iter().any(|command| matches!(
+        command,
+        Command::Save { file: StateFile::TableLists(lists), path }
+            if lists.get(&conn.0) == Some(&TableList::Tree)
+                && *path == harness.app.dirs.table_lists_file()
+    )));
+    // The setting no longer moves it, there or in a new window on it.
+    change(&mut harness, |settings| {
+        settings.table_list = TableList::Tree
+    });
+    change(&mut harness, |settings| {
+        settings.table_list = TableList::Flat
+    });
+    assert!(!flat(&harness, tab));
+    harness.app.apply(Action::Connect {
+        tab,
+        conn: conn.clone(),
+    });
+    assert!(!flat(&harness, tab), "opened again");
+    // Another connection, with no choice, takes the setting's.
+    let other = harness.connect_fake();
+    assert!(flat(&harness, other));
+    // Deleting the connection forgets it.
+    harness.app.apply(Action::DeleteConnection(conn.clone()));
+    assert!(!harness.app.table_lists.contains_key(&conn.0));
+}
+
+#[test]
+fn a_saved_table_list_is_read_back_at_a_start() {
+    let dir = tempfile::tempdir().unwrap();
+    let dirs = crate::paths::AppDirs::at(dir.path());
+    dirs.ensure().unwrap();
+    let lists = std::collections::BTreeMap::from([("green".to_owned(), TableList::Flat)]);
+    // As the backend saves it.
+    crate::util::save_json(&dirs.table_lists_file(), &lists).unwrap();
+    let app = crate::app::App::new(
+        dirs,
+        Settings::default().into(),
+        crate::backend::Backend::recording(),
+    );
+    assert_eq!(app.table_lists, lists);
+    assert!(app.lists_flat(&crate::connections::ConnectionId("green".into())));
+    assert!(!app.lists_flat(&crate::connections::ConnectionId("blue".into())));
+}
+
+#[test]
+fn a_connection_open_twice_lists_its_tables_alike() {
+    let mut harness = Harness::new();
+    let first = harness.connect_fake();
+    let conn = harness.app.workspace(first).unwrap().conn_id.clone();
+    harness.app.apply(Action::ShowConnections);
+    let second = harness.app.active_tab_id();
+    harness.app.apply(Action::Connect {
+        tab: second,
+        conn: conn.clone(),
+    });
+    assert_ne!(first, second);
+    harness.app.apply(Action::ToggleFlatTree(first));
+    for tab in [first, second] {
+        assert!(harness.app.workspace(tab).unwrap().tree.flat, "{tab:?}");
+    }
+}
+
+#[test]
+fn a_connection_that_is_not_saved_keeps_its_list_while_the_app_runs() {
+    let mut harness = Harness::new();
+    let tab = harness.connect_fake();
+    let conn = harness.app.workspace(tab).unwrap().conn_id.clone();
+    harness.app.connections.remove(&conn);
+    harness.app.apply(Action::ToggleFlatTree(tab));
+    // Not written: the file is the saved connections'.
+    let written = harness
+        .app
+        .backend
+        .sent
+        .iter()
+        .rev()
+        .find_map(|command| match command {
+            Command::Save {
+                file: StateFile::TableLists(lists),
+                ..
+            } => Some(lists.clone()),
+            _ => None,
+        });
+    assert_eq!(written, Some(std::collections::BTreeMap::new()));
+    // And the setting does not move it.
+    change(&mut harness, |settings| {
+        settings.table_list = TableList::Flat
+    });
+    change(&mut harness, |settings| {
+        settings.table_list = TableList::Tree
+    });
+    assert!(harness.app.workspace(tab).unwrap().tree.flat);
+}
+
+#[test]
+fn a_list_this_version_does_not_know_costs_only_its_connection() {
+    let dir = tempfile::tempdir().unwrap();
+    let dirs = crate::paths::AppDirs::at(dir.path());
+    dirs.ensure().unwrap();
+    std::fs::write(
+        dirs.table_lists_file(),
+        br#"{"green": "flat", "blue": "grouped", "red": 3}"#,
+    )
+    .unwrap();
+    let app = crate::app::App::new(
+        dirs,
+        Settings::default().into(),
+        crate::backend::Backend::recording(),
+    );
+    let expected = std::collections::BTreeMap::from([("green".to_owned(), TableList::Flat)]);
+    assert_eq!(app.table_lists, expected);
+}
+
+#[test]
+fn another_database_lists_its_tables_as_the_one_before() {
+    let mut harness = Harness::new();
+    let tab = harness.connect_fake();
+    harness.app.apply(Action::ToggleFlatTree(tab));
+    harness.app.apply(Action::SwitchDatabase {
+        tab,
+        database: "bookshop_review".into(),
+    });
+    assert!(harness.app.workspace(tab).unwrap().tree.flat);
+}
```

- [ ] **Step 2: Run the tests to see them fail**

Run: `cargo test --locked --lib appearance_settings_tests`
Expected:

```
error[E0609]: no field `table_lists` on type `app::App`
error[E0599]: no variant, associated function, or constant named `TableLists` found for enum `backend::StateFile`
error[E0599]: no method named `table_lists_file` found for struct `paths::AppDirs`
error[E0599]: no method named `lists_flat` found for struct `app::App`
```

- [ ] **Step 3: The file and its save**

```diff
diff --git a/src/backend.rs b/src/backend.rs
index 037a9d6..f794ccb 100644
--- a/src/backend.rs
+++ b/src/backend.rs
@@ -207,6 +207,8 @@ pub enum StateFile {
     Labels(crate::labels::Labels),
     /// What agents may do, as the app last showed it.
     AgentsShown(crate::agents_shown::Shown),
+    /// How each connection lists its tables where its user chose, by id.
+    TableLists(std::collections::BTreeMap<String, crate::settings::TableList>),
 }
 
 impl StateFile {
@@ -221,6 +223,7 @@ impl StateFile {
             Self::Settings(settings) => settings.save(path),
             Self::Labels(labels) => labels.save(path),
             Self::AgentsShown(shown) => shown.save(path),
+            Self::TableLists(lists) => crate::util::save_json(path, lists),
         }
     }
 }
diff --git a/src/paths.rs b/src/paths.rs
index b2647b1..c17d046 100644
--- a/src/paths.rs
+++ b/src/paths.rs
@@ -101,6 +101,13 @@ impl AppDirs {
         self.state.join("labels.json")
     }
 
+    /// How each connection lists its tables where its user chose: tree or
+    /// flat, by its id. Of this computer, and no setting: the setting is
+    /// the default for a connection with no choice.
+    pub fn table_lists_file(&self) -> PathBuf {
+        self.state.join("table_lists.json")
+    }
+
     /// When each saved connection last connected. Of this computer, and
     /// written at every connect: kept with the state, so that the file of
     /// connections is written only when one of them changes.
```

- [ ] **Step 4: The app keeps the choices**

`App::table_lists` is read at the start, value by value: one it does not know is skipped. A workspace opens with its connection's choice, else the setting's. The sidebar's toggle is the connection's choice in every window open on it; it is written for saved connections and held while the app runs for the others. A change of the setting moves every open connection without a choice; deleting a connection forgets it; another database keeps the list as it was.

```diff
diff --git a/src/app.rs b/src/app.rs
index d2e9268..f381046 100644
--- a/src/app.rs
+++ b/src/app.rs
@@ -96,6 +96,9 @@ pub struct App {
     /// The label columns chosen for the tables foreign keys point at, from
     /// labels.json. Read through `App::label_of` only.
     pub labels: crate::labels::Labels,
+    /// How each connection lists its tables where its user chose, by id,
+    /// from table_lists.json. One with no choice takes the setting's.
+    pub table_lists: std::collections::BTreeMap<String, crate::settings::TableList>,
     pub dialog: Option<Dialog>,
     /// Keyring reads in flight, and what each is for.
     pending_secrets: HashMap<RequestId, SecretPurpose>,
@@ -150,6 +153,21 @@ pub struct App {
     next_id: u64,
 }
 
+/// How each connection lists its tables, from `table_lists.json`. A value
+/// this version does not know is skipped, not the file: a newer version
+/// may have written it, and the other connections keep their choices.
+fn load_table_lists(
+    path: &std::path::Path,
+) -> std::collections::BTreeMap<String, crate::settings::TableList> {
+    let read: std::collections::BTreeMap<String, serde_json::Value> = crate::util::load_json(path);
+    read.into_iter()
+        .filter_map(|(id, list)| {
+            let list = crate::settings::TableList::from_name(list.as_str()?)?;
+            Some((id, list))
+        })
+        .collect()
+}
+
 impl App {
     pub fn new(dirs: AppDirs, loaded: Loaded, backend: Backend) -> Self {
         let source = loaded.source;
@@ -176,6 +194,7 @@ impl App {
             Some(crate::labels::database_key(spec.driver, &spec.database))
         });
         let agents_shown = crate::agents_shown::Shown::load(&dirs.agents_shown_file());
+        let table_lists = load_table_lists(&dirs.table_lists_file());
         let mut app = Self {
             dirs,
             settings,
@@ -191,6 +210,7 @@ impl App {
             backend,
             connections,
             labels,
+            table_lists,
             dialog: None,
             pending_secrets: HashMap::new(),
             pending_stores: HashMap::new(),
@@ -660,8 +680,22 @@ impl App {
                 }
             }
             Action::ToggleFlatTree(tab) => {
-                if let Some(workspace) = self.workspace_mut(tab) {
-                    workspace.tree.flat = !workspace.tree.flat;
+                let flipped = self
+                    .workspace(tab)
+                    .map(|workspace| (workspace.conn_id.clone(), !workspace.tree.flat));
+                // The choice is the connection's from now on: the setting no
+                // longer moves it, and its other windows list it so too.
+                if let Some((id, flat)) = flipped {
+                    for entry in &mut self.tabs {
+                        if let ConnTabContent::Workspace(workspace) = &mut entry.content
+                            && workspace.conn_id == id
+                        {
+                            workspace.tree.flat = flat;
+                        }
+                    }
+                    self.table_lists
+                        .insert(id.0, crate::settings::TableList::of(flat));
+                    self.save_table_lists();
                 }
             }
             Action::ToggleSidebar(tab) => {
@@ -2001,7 +2035,10 @@ impl App {
             Action::SwitchDatabase { tab, database } => {
                 if let Some(workspace) = self.workspace_mut(tab) {
                     workspace.spec.database = database;
+                    // The other database's objects, listed as these were.
+                    let flat = workspace.tree.flat;
                     workspace.tree = Tree::default();
+                    workspace.tree.flat = flat;
                     workspace.catalog_changed();
                     // The other database has other objects, but a SQL
                     // editor's text is the user's work: it stays, and so
@@ -2103,6 +2140,10 @@ impl App {
                     if self.labels.forget(&id) {
                         self.save_labels();
                     }
+                    // And its table list.
+                    if self.table_lists.remove(&id.0).is_some() {
+                        self.save_table_lists();
+                    }
                     if removed.password == PasswordMode::Keyring {
                         self.store_secret(&id, false, None, false);
                     }
@@ -2326,11 +2367,13 @@ impl App {
         let request = RequestId(self.next_id());
         // Where the option puts it; the grid's link switches it from there.
         let full_precision = self.settings.timestamps == crate::settings::Timestamps::Full;
+        let flat = self.lists_flat(&saved.id);
         let Some(entry) = self.tabs.iter_mut().find(|t| t.id == tab) else {
             return;
         };
         let mut workspace = Workspace::new(session, request, saved, typed);
         workspace.full_precision = full_precision;
+        workspace.tree.flat = flat;
         entry.content = ConnTabContent::Workspace(Box::new(workspace));
     }
 
@@ -3324,6 +3367,37 @@ impl App {
         });
     }
 
+    /// Whether the connection `id` lists its tables flat: as its user
+    /// chose for it, else as the setting has it.
+    pub fn lists_flat(&self, id: &crate::connections::ConnectionId) -> bool {
+        self.table_lists
+            .get(&id.0)
+            .copied()
+            .unwrap_or(self.settings.table_list)
+            .flat()
+    }
+
+    /// Saves how each saved connection lists its tables, on the backend.
+    /// The choice for one that is not saved is held while the app runs.
+    fn save_table_lists(&mut self) {
+        let saved = |id: &String| {
+            self.connections
+                .connections
+                .iter()
+                .any(|saved| saved.id.0 == *id)
+        };
+        let lists = self
+            .table_lists
+            .iter()
+            .filter(|(id, _)| saved(id))
+            .map(|(id, list)| (id.clone(), *list))
+            .collect();
+        self.backend.send(Command::Save {
+            path: self.dirs.table_lists_file(),
+            file: StateFile::TableLists(lists),
+        });
+    }
+
     /// Saves when each connection last connected, on the backend. A file
     /// of its own: a connect does not write the file of connections.
     fn save_last_used(&mut self) {
@@ -3640,6 +3714,17 @@ impl App {
                 }
             }
         }
+        if old.table_list != self.settings.table_list {
+            // Every open connection with no list of its own follows.
+            let flat = self.settings.table_list.flat();
+            for tab in &mut self.tabs {
+                if let ConnTabContent::Workspace(workspace) = &mut tab.content
+                    && !self.table_lists.contains_key(&workspace.conn_id.0)
+                {
+                    workspace.tree.flat = flat;
+                }
+            }
+        }
         if old.custom_theme != self.settings.custom_theme {
             // Reading a theme needs the window: `logic` has it.
             self.theme_changed = true;
```

- [ ] **Step 5: Run the tests to see them pass**

Run: `cargo test --locked --lib appearance_settings_tests`
Expected: PASS (11 tests). Then the three checks.

- [ ] **Step 6: Commit**

```bash
git add src/paths.rs src/backend.rs src/app.rs src/ui/appearance_settings_tests.rs
git commit -m "Keep each connection's table list, and start the others from the setting"
```

---

### Task 5: The Appearance pane

**Files:**
- Modify: `src/settings.rs`, `src/model.rs`, `src/ui/settings/mod.rs`, `src/ui/settings/sheet.rs`, `src/ui/settings/terminal.rs`, `src/ui/agent_settings_tests.rs`, `src/ui/appearance_settings_tests.rs`

- [ ] **Step 1: Write the failing tests**

The options' steps, the pane in both looks, and the Agents pane one `ctrl+l` further along.

```diff
diff --git a/src/settings.rs b/src/settings.rs
index 2375e09..22ad649 100644
--- a/src/settings.rs
+++ b/src/settings.rs
@@ -1599,13 +1599,26 @@ bar = \"tint\"  # tint | label
                 Key::Timestamps,
                 Key::GroupDigits,
                 Key::ValueTags,
+                Key::RowHeight,
+                Key::Zebra,
+                Key::ColumnTypes,
+                Key::TableList,
+                Key::EnvBar,
                 Key::McpEnabled,
                 Key::McpLogDays,
                 Key::McpNotify,
             ]
         );
         // Each pane has its own, and every one is on a pane.
-        let panes = [&OptionId::GENERAL[..], &OptionId::AGENTS[..]].concat();
+        let panes = [
+            &OptionId::GENERAL[..],
+            &OptionId::APPEARANCE[..],
+            &OptionId::AGENTS[..],
+        ]
+        .concat();
+        // The Appearance pane's sections hold its options, in order.
+        let sections = [&OptionId::GRID[..], &OptionId::ENVIRONMENT[..]].concat();
+        assert_eq!(sections, OptionId::APPEARANCE);
         assert_eq!(panes, OptionId::ALL);
         let defaults = Settings::default();
         for option in OptionId::ALL {
@@ -1777,4 +1790,54 @@ bar = \"tint\"  # tint | label
         assert_eq!(loaded.invalid, Vec::<usize>::new());
         assert_eq!(loaded.settings.row_height, RowHeight::Regular);
     }
+
+    #[test]
+    fn the_appearance_options_step_to_the_side_they_are_drawn_on() {
+        let settings = Settings::default();
+        let height = |settings: &Settings, forward| settings.stepped(OptionId::RowHeight, forward);
+        assert_eq!(
+            height(&settings, false),
+            OptionValue::RowHeight(RowHeight::Compact)
+        );
+        assert_eq!(
+            height(&settings, true),
+            OptionValue::RowHeight(RowHeight::Roomy)
+        );
+        let roomy = Settings {
+            row_height: RowHeight::Roomy,
+            ..Settings::default()
+        };
+        assert_eq!(
+            height(&roomy, true),
+            OptionValue::RowHeight(RowHeight::Roomy)
+        );
+        assert_eq!(settings.flipped(OptionId::RowHeight), None);
+        assert_eq!(
+            settings.stepped(OptionId::TableList, true),
+            OptionValue::TableList(TableList::Flat)
+        );
+        assert_eq!(
+            settings.stepped(OptionId::EnvBar, true),
+            OptionValue::EnvBar(EnvBar::Label)
+        );
+        assert_eq!(
+            settings.flipped(OptionId::EnvBar),
+            Some(OptionValue::EnvBar(EnvBar::Label))
+        );
+        assert_eq!(
+            settings.flipped(OptionId::ColumnTypes),
+            Some(OptionValue::ColumnTypes(false))
+        );
+        // Stripes the window shows on are flipped off.
+        let shown = Settings {
+            zebra: Some(true),
+            ..Settings::default()
+        };
+        assert_eq!(
+            shown.flipped(OptionId::Zebra),
+            Some(OptionValue::Zebra(Some(false)))
+        );
+        // A reset leaves them to the look.
+        assert_eq!(OptionId::Zebra.default_value(), OptionValue::Zebra(None));
+    }
 }
diff --git a/src/ui/agent_settings_tests.rs b/src/ui/agent_settings_tests.rs
index 82b3a4c..a790945 100644
--- a/src/ui/agent_settings_tests.rs
+++ b/src/ui/agent_settings_tests.rs
@@ -59,7 +59,7 @@ fn on_the_pane(harness: &mut Harness, rows: usize) {
 }
 
 #[test]
-fn settings_lists_an_agents_pane_beside_general() {
+fn settings_lists_an_agents_pane_after_appearance() {
     for look in looks() {
         let mut harness = Harness::new();
         harness.set_look(look);
@@ -67,8 +67,10 @@ fn settings_lists_an_agents_pane_beside_general() {
         harness.settle();
         assert_eq!(pane(&harness), Some(SettingsPane::General), "{}", look.name);
         assert!(!harness.has("Let agents use Tabletist"), "{}", look.name);
+        // Past Appearance, the pane between them.
         if look.terminal {
             harness.press(egui::Key::L, egui::Modifiers::CTRL);
+            harness.press(egui::Key::L, egui::Modifiers::CTRL);
         } else {
             harness.click("Agents");
         }
@@ -79,6 +81,7 @@ fn settings_lists_an_agents_pane_beside_general() {
         // And back.
         if look.terminal {
             harness.press(egui::Key::H, egui::Modifiers::CTRL);
+            harness.press(egui::Key::H, egui::Modifiers::CTRL);
         } else {
             harness.click("General");
         }
diff --git a/src/ui/appearance_settings_tests.rs b/src/ui/appearance_settings_tests.rs
index 25b5d8e..0c69c49 100644
--- a/src/ui/appearance_settings_tests.rs
+++ b/src/ui/appearance_settings_tests.rs
@@ -1,11 +1,12 @@
 //! Settings › Appearance, in both looks: what each option does to the
-//! grids and the sidebar's list.
+//! grids and the sidebar's list, and the pane that sets them with a click
+//! or a key and writes them to the file.
 
 use egui::accesskit::Role;
 
 use crate::backend::{Command, StateFile};
-use crate::model::{Action, ConnTabId, TabId};
-use crate::settings::{RowHeight, Settings, TableList};
+use crate::model::{Action, ConnTabId, Dialog, SettingsPane, TabId};
+use crate::settings::{EnvBar, OptionValue, RowHeight, Settings, TableList};
 use crate::testing::{Harness, bounds};
 use crate::theme::Look;
 
@@ -345,3 +346,196 @@ fn another_database_lists_its_tables_as_the_one_before() {
     });
     assert!(harness.app.workspace(tab).unwrap().tree.flat);
 }
+
+/// The pane Settings shows, while it is open.
+fn pane(harness: &Harness) -> Option<SettingsPane> {
+    match &harness.app.dialog {
+        Some(Dialog::Settings(dialog)) => Some(dialog.pane),
+        _ => None,
+    }
+}
+
+/// How many times the settings were saved.
+fn settings_saves(harness: &Harness) -> usize {
+    harness
+        .app
+        .backend
+        .sent
+        .iter()
+        .filter(|command| {
+            matches!(
+                command,
+                Command::Save {
+                    file: StateFile::Settings(_),
+                    ..
+                }
+            )
+        })
+        .count()
+}
+
+/// Settings on its Appearance pane.
+fn on_the_pane(harness: &mut Harness) {
+    harness.app.apply(Action::ShowSettings);
+    harness
+        .app
+        .apply(Action::ShowSettingsPane(SettingsPane::Appearance));
+    harness.settle();
+}
+
+#[test]
+fn settings_lists_an_appearance_pane_after_general() {
+    for look in looks() {
+        let mut harness = Harness::new();
+        harness.set_look(look);
+        harness.app.apply(Action::ShowSettings);
+        harness.settle();
+        if look.terminal {
+            harness.press(egui::Key::L, egui::Modifiers::CTRL);
+        } else {
+            harness.click("Appearance");
+        }
+        harness.settle();
+        assert_eq!(
+            pane(&harness),
+            Some(SettingsPane::Appearance),
+            "{}",
+            look.name
+        );
+        let first = if look.terminal {
+            "Row height"
+        } else {
+            "Row density"
+        };
+        assert!(harness.has(first), "{}", look.name);
+        assert!(!harness.has("Rows per page"), "{}", look.name);
+    }
+}
+
+#[test]
+fn the_sheet_sets_each_appearance_option_with_a_click() {
+    let mut harness = Harness::new();
+    harness.set_look(Look::macos());
+    on_the_pane(&mut harness);
+    // The board's words.
+    for words in [
+        "GRID",
+        "ENVIRONMENT COLOR",
+        "Second line in headers: int8, varchar, jsonb",
+        "Default for new connections; the sidebar toggle overrides it",
+        "Production always uses the full tint",
+    ] {
+        assert!(shows(&mut harness, words), "{words}");
+    }
+    let saves = settings_saves(&harness);
+    harness.click("Row density: Compact · 28");
+    harness.click("Alternate row shading");
+    harness.click("Show column types");
+    harness.click("Table list: Flat");
+    harness.click("Window header: Stripe only");
+    harness.settle();
+    let settings = &harness.app.settings;
+    assert_eq!(settings.row_height, RowHeight::Compact);
+    assert_eq!(settings.zebra, Some(true), "off on macOS until set");
+    assert!(!settings.column_types);
+    assert_eq!(settings.table_list, TableList::Flat);
+    assert_eq!(settings.env_bar, EnvBar::Label);
+    assert_eq!(settings_saves(&harness), saves + 5);
+}
+
+#[test]
+fn the_screen_sets_each_appearance_option_with_its_keys() {
+    let mut harness = Harness::new();
+    harness.set_look(Look::omarchy());
+    on_the_pane(&mut harness);
+    for words in [
+        "grid",
+        "environment",
+        "1 · 1.5 · 2 lines",
+        "second header line",
+        "toggles",
+        "PROD always tinted",
+    ] {
+        assert!(shows(&mut harness, words), "{words}");
+    }
+    let saves = settings_saves(&harness);
+    let key = |harness: &mut Harness, key: egui::Key, text: &str| {
+        super::tests::type_key(harness, key, text);
+    };
+    // Row height: one to the left, compact.
+    key(&mut harness, egui::Key::H, "h");
+    // Alternate rows, on in the terminal until set: space turns it off.
+    key(&mut harness, egui::Key::J, "j");
+    harness.press(egui::Key::Space, egui::Modifiers::NONE);
+    // Column types off, the list flat, the bar's label alone.
+    key(&mut harness, egui::Key::J, "j");
+    harness.press(egui::Key::Space, egui::Modifiers::NONE);
+    key(&mut harness, egui::Key::J, "j");
+    key(&mut harness, egui::Key::L, "l");
+    key(&mut harness, egui::Key::J, "j");
+    key(&mut harness, egui::Key::L, "l");
+    harness.settle();
+    let settings = &harness.app.settings;
+    assert_eq!(settings.row_height, RowHeight::Compact);
+    assert_eq!(settings.zebra, Some(false));
+    assert!(!settings.column_types);
+    assert_eq!(settings.table_list, TableList::Flat);
+    assert_eq!(settings.env_bar, EnvBar::Label);
+    assert_eq!(settings_saves(&harness), saves + 5);
+    // R puts the bar back, and the stripes back to the look's.
+    harness.press(egui::Key::R, egui::Modifiers::SHIFT);
+    for _ in 0..3 {
+        key(&mut harness, egui::Key::K, "k");
+    }
+    harness.press(egui::Key::R, egui::Modifiers::SHIFT);
+    harness.settle();
+    assert_eq!(harness.app.settings.env_bar, EnvBar::Tint);
+    assert_eq!(harness.app.settings.zebra, None);
+}
+
+#[test]
+fn reset_to_defaults_puts_the_appearance_options_back() {
+    let mut harness = Harness::new();
+    harness.set_look(Look::macos());
+    for value in [
+        OptionValue::RowHeight(RowHeight::Roomy),
+        OptionValue::Zebra(Some(true)),
+        OptionValue::ColumnTypes(false),
+        OptionValue::TableList(TableList::Flat),
+        OptionValue::EnvBar(EnvBar::Label),
+        OptionValue::PageSize(500),
+    ] {
+        harness.app.apply(Action::SetOption(value));
+    }
+    on_the_pane(&mut harness);
+    harness.app.apply(Action::ResetSettings);
+    harness.app.apply(Action::ConfirmResetSettings(true));
+    harness.settle();
+    let settings = &harness.app.settings;
+    let defaults = Settings::default();
+    assert_eq!(settings.row_height, defaults.row_height);
+    assert_eq!(settings.zebra, None);
+    assert_eq!(settings.column_types, defaults.column_types);
+    assert_eq!(settings.table_list, defaults.table_list);
+    assert_eq!(settings.env_bar, defaults.env_bar);
+    // Another pane's options are its own.
+    assert_eq!(settings.page_size, 500);
+}
+
+#[test]
+fn a_reset_and_a_toggle_in_one_frame_turn_the_look_s_stripes_off() {
+    let mut harness = Harness::new();
+    harness.set_look(Look::omarchy());
+    on_the_pane(&mut harness);
+    super::tests::type_key(&mut harness, egui::Key::J, "j");
+    change(&mut harness, |settings| settings.zebra = Some(true));
+    // R puts them back to the look's, which shows them; Space then turns
+    // them off.
+    harness.frame(vec![
+        crate::testing::key(egui::Key::R, egui::Modifiers::SHIFT),
+        egui::Event::Text("R".into()),
+        crate::testing::key(egui::Key::Space, egui::Modifiers::NONE),
+    ]);
+    harness.settle();
+    assert_eq!(harness.app.settings.zebra, Some(false));
+}
```

- [ ] **Step 2: Run the tests to see them fail**

Run: `cargo test --locked --lib appearance`
Expected:

```
error[E0599]: no variant, associated function, or constant named `APPEARANCE` found for enum `settings::OptionId`
error[E0599]: no variant, associated function, or constant named `GRID` found for enum `settings::OptionId`
error[E0599]: no variant, associated function, or constant named `ENVIRONMENT` found for enum `settings::OptionId`
error[E0599]: no variant, associated function, or constant named `RowHeight` found for enum `settings::OptionId`
error[E0599]: no variant, associated function, or constant named `RowHeight` found for enum `settings::OptionValue`
...
```

- [ ] **Step 3: The options**

Five `OptionId`s, in `APPEARANCE` and its two sections `GRID` and `ENVIRONMENT`, with their values and steps. A step and a flip go the way the controls are drawn: compact at the left, the tree and the tint at the left.

```diff
diff --git a/src/settings.rs b/src/settings.rs
index 22ad649..04bba94 100644
--- a/src/settings.rs
+++ b/src/settings.rs
@@ -268,15 +268,26 @@ pub enum OptionId {
     LogDays,
     /// Whether a new agent is told of: `mcp.notify`.
     AgentsNotify,
+    RowHeight,
+    Zebra,
+    ColumnTypes,
+    TableList,
+    EnvBar,
 }
 
 impl OptionId {
-    /// Every option, the General pane's and then the Agents pane's.
-    pub const ALL: [OptionId; 7] = [
+    /// Every option, the General pane's, the Appearance pane's and then
+    /// the Agents pane's.
+    pub const ALL: [OptionId; 12] = [
         Self::PageSize,
         Self::Timestamps,
         Self::GroupDigits,
         Self::ValueTags,
+        Self::RowHeight,
+        Self::Zebra,
+        Self::ColumnTypes,
+        Self::TableList,
+        Self::EnvBar,
         Self::AgentsOn,
         Self::LogDays,
         Self::AgentsNotify,
@@ -288,6 +299,22 @@ impl OptionId {
         Self::GroupDigits,
         Self::ValueTags,
     ];
+    /// The Appearance pane's, in the order it has them: its Grid
+    /// section's, then its Environment section's.
+    pub const APPEARANCE: [OptionId; 5] = [
+        Self::RowHeight,
+        Self::Zebra,
+        Self::ColumnTypes,
+        Self::TableList,
+        Self::EnvBar,
+    ];
+    pub const GRID: [OptionId; 4] = [
+        Self::RowHeight,
+        Self::Zebra,
+        Self::ColumnTypes,
+        Self::TableList,
+    ];
+    pub const ENVIRONMENT: [OptionId; 1] = [Self::EnvBar];
     /// The Agents pane's, in the order it has them.
     pub const AGENTS: [OptionId; 3] = [Self::AgentsOn, Self::LogDays, Self::AgentsNotify];
 
@@ -301,6 +328,11 @@ impl OptionId {
             Self::AgentsOn => Key::McpEnabled,
             Self::LogDays => Key::McpLogDays,
             Self::AgentsNotify => Key::McpNotify,
+            Self::RowHeight => Key::RowHeight,
+            Self::Zebra => Key::Zebra,
+            Self::ColumnTypes => Key::ColumnTypes,
+            Self::TableList => Key::TableList,
+            Self::EnvBar => Key::EnvBar,
         }
     }
 
@@ -320,6 +352,12 @@ pub enum OptionValue {
     AgentsOn(bool),
     LogDays(u32),
     AgentsNotify(bool),
+    RowHeight(RowHeight),
+    /// `None` is the look's default.
+    Zebra(Option<bool>),
+    ColumnTypes(bool),
+    TableList(TableList),
+    EnvBar(EnvBar),
 }
 
 impl OptionValue {
@@ -332,6 +370,11 @@ impl OptionValue {
             Self::AgentsOn(_) => OptionId::AgentsOn,
             Self::LogDays(_) => OptionId::LogDays,
             Self::AgentsNotify(_) => OptionId::AgentsNotify,
+            Self::RowHeight(_) => OptionId::RowHeight,
+            Self::Zebra(_) => OptionId::Zebra,
+            Self::ColumnTypes(_) => OptionId::ColumnTypes,
+            Self::TableList(_) => OptionId::TableList,
+            Self::EnvBar(_) => OptionId::EnvBar,
         }
     }
 
@@ -345,6 +388,11 @@ impl OptionValue {
             Self::AgentsOn(on) => settings.mcp_enabled = on,
             Self::LogDays(days) => settings.mcp_log_days = days,
             Self::AgentsNotify(on) => settings.mcp_notify = on,
+            Self::RowHeight(height) => settings.row_height = height,
+            Self::Zebra(on) => settings.zebra = on,
+            Self::ColumnTypes(on) => settings.column_types = on,
+            Self::TableList(list) => settings.table_list = list,
+            Self::EnvBar(bar) => settings.env_bar = bar,
         }
     }
 }
@@ -595,6 +643,11 @@ impl Settings {
             OptionId::AgentsOn => OptionValue::AgentsOn(self.mcp_enabled),
             OptionId::LogDays => OptionValue::LogDays(self.mcp_log_days),
             OptionId::AgentsNotify => OptionValue::AgentsNotify(self.mcp_notify),
+            OptionId::RowHeight => OptionValue::RowHeight(self.row_height),
+            OptionId::Zebra => OptionValue::Zebra(self.zebra),
+            OptionId::ColumnTypes => OptionValue::ColumnTypes(self.column_types),
+            OptionId::TableList => OptionValue::TableList(self.table_list),
+            OptionId::EnvBar => OptionValue::EnvBar(self.env_bar),
         }
     }
 
@@ -644,6 +697,25 @@ impl Settings {
                 OptionValue::LogDays(next.unwrap_or(days))
             }
             OptionId::AgentsNotify => OptionValue::AgentsNotify(forward),
+            OptionId::RowHeight => {
+                let at = RowHeight::ALL
+                    .iter()
+                    .position(|choice| *choice == self.row_height)
+                    .unwrap_or(1);
+                let next = if forward {
+                    (at + 1).min(RowHeight::ALL.len() - 1)
+                } else {
+                    at.saturating_sub(1)
+                };
+                OptionValue::RowHeight(RowHeight::ALL[next])
+            }
+            OptionId::Zebra => OptionValue::Zebra(Some(forward)),
+            OptionId::ColumnTypes => OptionValue::ColumnTypes(forward),
+            // The tree is the left of the two, the tint too.
+            OptionId::TableList => OptionValue::TableList(TableList::of(forward)),
+            OptionId::EnvBar => {
+                OptionValue::EnvBar(if forward { EnvBar::Label } else { EnvBar::Tint })
+            }
         }
     }
 
@@ -660,6 +732,18 @@ impl Settings {
             OptionId::AgentsOn => Some(OptionValue::AgentsOn(!self.mcp_enabled)),
             OptionId::LogDays => None,
             OptionId::AgentsNotify => Some(OptionValue::AgentsNotify(!self.mcp_notify)),
+            OptionId::RowHeight => None,
+            // Unset, it is flipped from off: the window flips the
+            // settings as it shows them, with the look's default in.
+            OptionId::Zebra => Some(OptionValue::Zebra(Some(!self.zebra.unwrap_or(false)))),
+            OptionId::ColumnTypes => Some(OptionValue::ColumnTypes(!self.column_types)),
+            OptionId::TableList => Some(OptionValue::TableList(TableList::of(
+                !self.table_list.flat(),
+            ))),
+            OptionId::EnvBar => Some(OptionValue::EnvBar(match self.env_bar {
+                EnvBar::Tint => EnvBar::Label,
+                EnvBar::Label => EnvBar::Tint,
+            })),
         }
     }
 
```

- [ ] **Step 4: The pane**

`SettingsPane::Appearance`, listed second.

```diff
diff --git a/src/model.rs b/src/model.rs
index 9458560..cb3fdca 100644
--- a/src/model.rs
+++ b/src/model.rs
@@ -2113,6 +2113,8 @@ pub struct SettingsDialog {
 pub enum SettingsPane {
     #[default]
     General,
+    /// How the grid and the connection bar are drawn.
+    Appearance,
     /// How agents reach the app. Listed where they can.
     Agents,
 }
@@ -2121,9 +2123,9 @@ impl SettingsPane {
     /// The panes the window lists, in order.
     pub fn listed() -> &'static [SettingsPane] {
         if crate::endpoint::AVAILABLE {
-            &[Self::General, Self::Agents]
+            &[Self::General, Self::Appearance, Self::Agents]
         } else {
-            &[Self::General]
+            &[Self::General, Self::Appearance]
         }
     }
 
@@ -2131,6 +2133,7 @@ impl SettingsPane {
     pub fn name(self) -> &'static str {
         match self {
             Self::General => "General",
+            Self::Appearance => "Appearance",
             Self::Agents => "Agents",
         }
     }
@@ -2139,6 +2142,7 @@ impl SettingsPane {
     pub fn options(self) -> &'static [crate::settings::OptionId] {
         match self {
             Self::General => &crate::settings::OptionId::GENERAL,
+            Self::Appearance => &crate::settings::OptionId::APPEARANCE,
             Self::Agents => &crate::settings::OptionId::AGENTS,
         }
     }
@@ -2150,6 +2154,7 @@ impl SettingsPane {
         use crate::settings::OptionId;
         match self {
             Self::General => &OptionId::GENERAL,
+            Self::Appearance => &OptionId::APPEARANCE,
             Self::Agents => &[OptionId::LogDays, OptionId::AgentsNotify],
         }
     }
@@ -2164,6 +2169,13 @@ impl SettingsPane {
                 SettingsRow::Option(OptionId::GroupDigits),
                 SettingsRow::Option(OptionId::ValueTags),
             ],
+            Self::Appearance => &[
+                SettingsRow::Option(OptionId::RowHeight),
+                SettingsRow::Option(OptionId::Zebra),
+                SettingsRow::Option(OptionId::ColumnTypes),
+                SettingsRow::Option(OptionId::TableList),
+                SettingsRow::Option(OptionId::EnvBar),
+            ],
             Self::Agents => &[
                 SettingsRow::Option(OptionId::AgentsOn),
                 SettingsRow::AgentsStatus,
```

The labels, small print, choices and the settings as the window shows them. The terminal names four of the options its own way (`terminal_label`). `shown` puts the look's stripes in while `zebra` is unset, so a flip or a step starts from what is drawn; the screen's keys do the same after each key, so `R` then Space in one frame flips what `R` left.

```diff
diff --git a/src/ui/settings/mod.rs b/src/ui/settings/mod.rs
index 5cc6fb7..173c653 100644
--- a/src/ui/settings/mod.rs
+++ b/src/ui/settings/mod.rs
@@ -16,7 +16,7 @@ use crate::app::App;
 use crate::i18n::{Locale, gettext, ngettext};
 use crate::keymap::{Command, Keymap, Layout, Scope, Spelled, Stroke};
 use crate::model::{Action, AgentsView, Dialog, SettingsDialog, SettingsPane, SettingsRow};
-use crate::settings::{OptionId, OptionValue, Settings, Timestamps};
+use crate::settings::{EnvBar, OptionId, OptionValue, RowHeight, Settings, TableList, Timestamps};
 
 /// The label of an option's row. English: the caller translates, and the
 /// terminal look lower-cases.
@@ -29,6 +29,30 @@ pub(super) fn label(option: OptionId) -> &'static str {
         OptionId::AgentsOn => "Let agents use Tabletist",
         OptionId::LogDays => "Keep the activity log",
         OptionId::AgentsNotify => "Notify when an agent connects",
+        OptionId::RowHeight => "Row density",
+        OptionId::Zebra => "Alternate row shading",
+        OptionId::ColumnTypes => "Show column types",
+        OptionId::TableList => "Table list",
+        OptionId::EnvBar => "Window header",
+    }
+}
+
+/// The label of an option's row in the terminal look, which names some of
+/// the Appearance pane's options its own way. English, as [`label`]'s.
+pub(super) fn terminal_label(option: OptionId) -> &'static str {
+    match option {
+        OptionId::RowHeight => "Row height",
+        OptionId::Zebra => "Alternate rows",
+        OptionId::ColumnTypes => "Column types",
+        OptionId::EnvBar => "Connection bar",
+        OptionId::PageSize
+        | OptionId::Timestamps
+        | OptionId::GroupDigits
+        | OptionId::ValueTags
+        | OptionId::AgentsOn
+        | OptionId::LogDays
+        | OptionId::AgentsNotify
+        | OptionId::TableList => label(option),
     }
 }
 
@@ -38,10 +62,15 @@ pub(super) fn small_print(option: OptionId) -> Option<&'static str> {
         OptionId::PageSize => Some("Table view; the SQL editor has its own limit"),
         OptionId::ValueTags => Some("Colors for enum, CHECK (…IN…) and boolean columns"),
         OptionId::LogDays => Some("Every call and its SQL, never the rows. On this computer only"),
+        OptionId::ColumnTypes => Some("Second line in headers: int8, varchar, jsonb"),
+        OptionId::TableList => Some("Default for new connections; the sidebar toggle overrides it"),
+        OptionId::EnvBar => Some("Production always uses the full tint"),
         OptionId::Timestamps
         | OptionId::GroupDigits
         | OptionId::AgentsOn
-        | OptionId::AgentsNotify => None,
+        | OptionId::AgentsNotify
+        | OptionId::RowHeight
+        | OptionId::Zebra => None,
     }
 }
 
@@ -57,15 +86,18 @@ pub(super) fn is_on(value: OptionValue) -> bool {
         OptionValue::ValueTags(true)
             | OptionValue::AgentsOn(true)
             | OptionValue::AgentsNotify(true)
+            | OptionValue::Zebra(Some(true))
+            | OptionValue::ColumnTypes(true)
     )
 }
 
 /// The settings as the window shows them: the server's switch as it is in
 /// force, off where a change made outside the app that turned it on is
-/// held.
+/// held, and the stripes as the look draws them while they are not set.
 pub(super) fn shown(app: &App) -> Settings {
     let mut settings = app.settings.clone();
     settings.mcp_enabled = app.agents_on();
+    settings.zebra = Some(settings.zebra_in(app.look.terminal));
     settings
 }
 
@@ -140,6 +172,27 @@ pub(super) fn choices(option: OptionId, settings: &Settings) -> Vec<(Said, Optio
             (Word("off"), OptionValue::AgentsNotify(false)),
             (Word("on"), OptionValue::AgentsNotify(true)),
         ],
+        OptionId::RowHeight => vec![
+            (Word("compact"), OptionValue::RowHeight(RowHeight::Compact)),
+            (Word("regular"), OptionValue::RowHeight(RowHeight::Regular)),
+            (Word("roomy"), OptionValue::RowHeight(RowHeight::Roomy)),
+        ],
+        OptionId::Zebra => vec![
+            (Word("off"), OptionValue::Zebra(Some(false))),
+            (Word("on"), OptionValue::Zebra(Some(true))),
+        ],
+        OptionId::ColumnTypes => vec![
+            (Word("off"), OptionValue::ColumnTypes(false)),
+            (Word("on"), OptionValue::ColumnTypes(true)),
+        ],
+        OptionId::TableList => vec![
+            (Word("tree"), OptionValue::TableList(TableList::Tree)),
+            (Word("flat"), OptionValue::TableList(TableList::Flat)),
+        ],
+        OptionId::EnvBar => vec![
+            (Word("tint + label"), OptionValue::EnvBar(EnvBar::Tint)),
+            (Word("label only"), OptionValue::EnvBar(EnvBar::Label)),
+        ],
     }
 }
 
@@ -352,6 +405,9 @@ fn keys(app: &App, ctx: &egui::Context, dialog: &SettingsDialog, actions: &mut V
                     };
                     if let Some(value) = value {
                         value.set(&mut settings);
+                        // Stripes put back to the look's are drawn as the
+                        // look draws them: the next key flips those.
+                        settings.zebra = Some(settings.zebra_in(app.look.terminal));
                         actions.push(Action::SetOption(value));
                     }
                 }
```

The sheet: `content` takes a title, its line where it has one, and sections, so General and Appearance share it. `segments` takes any number of words.

```diff
diff --git a/src/ui/settings/sheet.rs b/src/ui/settings/sheet.rs
index 1c26c50..6b3b42e 100644
--- a/src/ui/settings/sheet.rs
+++ b/src/ui/settings/sheet.rs
@@ -9,7 +9,7 @@ use egui::{CornerRadius, Rect, Sense, Stroke, Ui, WidgetInfo, WidgetType, pos2,
 use crate::app::App;
 use crate::i18n::{Locale, gettext, ngettext};
 use crate::model::{Action, SettingsDialog, SettingsPane};
-use crate::settings::{OptionId, OptionValue, Settings, Timestamps};
+use crate::settings::{EnvBar, OptionId, OptionValue, RowHeight, Settings, TableList, Timestamps};
 use crate::theme::{Look, Palette};
 use crate::typography::{Laid, Text, TextRole};
 use crate::ui::focus::{self, Ring};
@@ -138,7 +138,29 @@ pub(super) fn show(
             let builder = egui::UiBuilder::new().id_salt("content").max_rect(pane);
             let mut pane = ui.new_child(builder.layout(egui::Layout::top_down(egui::Align::Min)));
             match dialog.pane {
-                SettingsPane::General => content(&mut pane, &app.settings, limit, &skin, actions),
+                SettingsPane::General => content(
+                    &mut pane,
+                    ("General", Some(GENERAL)),
+                    &[("Data", &OptionId::GENERAL)],
+                    &app.settings,
+                    limit,
+                    &skin,
+                    actions,
+                ),
+                // The board's line under the title names the preview, which
+                // the pane does not have yet: the title stands alone.
+                SettingsPane::Appearance => content(
+                    &mut pane,
+                    ("Appearance", None),
+                    &[
+                        ("Grid", &OptionId::GRID),
+                        ("Environment color", &OptionId::ENVIRONMENT),
+                    ],
+                    &super::shown(app),
+                    limit,
+                    &skin,
+                    actions,
+                ),
                 SettingsPane::Agents => {
                     super::agents::content(&mut pane, app, dialog.agent, limit, &skin, actions);
                 }
@@ -248,32 +270,49 @@ fn nav(
     }
 }
 
-/// The tab: its title, what it is for, and its options under the heading
-/// of their group. The rows end at `limit` at the latest, and scroll when
-/// they are taller than that leaves them.
-fn content(ui: &mut Ui, settings: &Settings, limit: f32, skin: &Skin, actions: &mut Vec<Action>) {
+/// What the General pane says under its title.
+const GENERAL: &str = "How Tabletist shows data. Changes apply right away.";
+
+/// The tab: its title, what it is for where it says, and its options under
+/// the heading of each group. The rows end at `limit` at the latest, and
+/// scroll when they are taller than that leaves them.
+#[allow(clippy::too_many_arguments)] // one pane: what it says, what it shows, what it pushes
+fn content(
+    ui: &mut Ui,
+    (title, about): (&'static str, Option<&'static str>),
+    sections: &[(&'static str, &[OptionId])],
+    settings: &Settings,
+    limit: f32,
+    skin: &Skin,
+    actions: &mut Vec<Action>,
+) {
     let Skin { look, palette, .. } = *skin;
     widgets::label(
         ui,
         TextRole::DialogTitle,
-        &skin.say("General"),
+        &skin.say(title),
         palette.text,
         look,
     );
-    ui.add_space(2.0);
-    widgets::label(
-        ui,
-        TextRole::UiBody,
-        &skin.say("How Tabletist shows data. Changes apply right away."),
-        palette.dim,
-        look,
-    );
+    if let Some(about) = about {
+        ui.add_space(2.0);
+        widgets::label(ui, TextRole::UiBody, &skin.say(about), palette.dim, look);
+    }
     ui.add_space(6.0 + 14.0);
-    widgets::section_label(&skin.say("Data"), look, palette)
+    let Some((first, _)) = sections.first() else {
+        return;
+    };
+    // The first heading stays over the rows as they scroll; the others
+    // scroll with their rows, 14 under the last row before them.
+    widgets::section_label(&skin.say(first), look, palette)
         .layout(ui.ctx())
         .label(ui);
     ui.add_space(2.0);
-    let column = label_column(ui, &OptionId::GENERAL, skin);
+    let options: Vec<OptionId> = sections
+        .iter()
+        .flat_map(|(_, options)| options.iter().copied())
+        .collect();
+    let column = label_column(ui, &options, skin);
     let room = (limit - ui.cursor().top()).max(ROWS_MIN);
     egui::ScrollArea::vertical()
         .id_salt("rows")
@@ -282,18 +321,29 @@ fn content(ui: &mut Ui, settings: &Settings, limit: f32, skin: &Skin, actions: &
         .show(ui, |ui| {
             // The last row stands on the footer's rule: one of its own
             // right over that would read as a double line.
-            let last = OptionId::GENERAL.len() - 1;
-            for (index, option) in OptionId::GENERAL.into_iter().enumerate() {
-                row(
-                    ui,
-                    option,
-                    index < last,
-                    column,
-                    settings,
-                    skin,
-                    actions,
-                    None,
-                );
+            let last = options.len() - 1;
+            let mut index = 0;
+            for (nth, (heading, options)) in sections.iter().enumerate() {
+                if nth > 0 {
+                    ui.add_space(14.0);
+                    widgets::section_label(&skin.say(heading), look, palette)
+                        .layout(ui.ctx())
+                        .label(ui);
+                    ui.add_space(2.0);
+                }
+                for option in options.iter().copied() {
+                    row(
+                        ui,
+                        option,
+                        index < last,
+                        column,
+                        settings,
+                        skin,
+                        actions,
+                        None,
+                    );
+                    index += 1;
+                }
             }
         });
 }
@@ -418,7 +468,12 @@ fn control_height(option: OptionId, look: &Look) -> f32 {
         OptionId::PageSize => look.control_height,
         // The track has 2 over its segments and under them.
         OptionId::Timestamps | OptionId::GroupDigits => SEGMENT + 4.0,
-        OptionId::ValueTags | OptionId::AgentsOn | OptionId::AgentsNotify => SWITCH,
+        OptionId::RowHeight | OptionId::TableList | OptionId::EnvBar => SEGMENT + 4.0,
+        OptionId::ValueTags
+        | OptionId::AgentsOn
+        | OptionId::AgentsNotify
+        | OptionId::Zebra
+        | OptionId::ColumnTypes => SWITCH,
         OptionId::LogDays => look.control_height,
     }
 }
@@ -482,7 +537,44 @@ fn control(
                 actions.push(Action::SetOption(OptionValue::GroupDigits(picked == 0)));
             }
         }
-        OptionId::ValueTags | OptionId::AgentsOn | OptionId::AgentsNotify => {
+        OptionId::RowHeight => {
+            let words = [
+                skin.say("Compact · 28"),
+                skin.say("Regular · 36"),
+                skin.say("Roomy · 44"),
+            ];
+            let selected = RowHeight::ALL
+                .iter()
+                .position(|height| *height == settings.row_height)
+                .unwrap_or(1);
+            if let Some(picked) = segments(ui, &name, &words, selected, skin) {
+                actions.push(Action::SetOption(OptionValue::RowHeight(
+                    RowHeight::ALL[picked],
+                )));
+            }
+        }
+        OptionId::TableList => {
+            let words = [skin.say("Grouped by prefix"), skin.say("Flat")];
+            let selected = usize::from(settings.table_list.flat());
+            if let Some(picked) = segments(ui, &name, &words, selected, skin) {
+                actions.push(Action::SetOption(OptionValue::TableList(TableList::of(
+                    picked == 1,
+                ))));
+            }
+        }
+        OptionId::EnvBar => {
+            let words = [skin.say("Tint + stripe"), skin.say("Stripe only")];
+            let selected = usize::from(settings.env_bar == EnvBar::Label);
+            if let Some(picked) = segments(ui, &name, &words, selected, skin) {
+                let bar = [EnvBar::Tint, EnvBar::Label][picked];
+                actions.push(Action::SetOption(OptionValue::EnvBar(bar)));
+            }
+        }
+        OptionId::ValueTags
+        | OptionId::AgentsOn
+        | OptionId::AgentsNotify
+        | OptionId::Zebra
+        | OptionId::ColumnTypes => {
             let on = super::is_on(settings.value(option));
             if widgets::toggle(ui, on, &name, palette).clicked()
                 && let Some(flipped) = settings.flipped(option)
@@ -532,7 +624,7 @@ fn log_days(current: u32) -> Vec<u32> {
 fn segments(
     ui: &mut Ui,
     option: &str,
-    words: &[String; 2],
+    words: &[String],
     selected: usize,
     skin: &Skin,
 ) -> Option<usize> {
@@ -547,11 +639,15 @@ fn segments(
     // The row's label is beside the control, not a segment's own: each is
     // named by the option and its own word, as the terminal look's
     // choices are.
-    let names = [0, 1].map(|index| format!("{option}: {}", words[index]));
-    let parts = [0, 1].map(|index| Segment::Named {
-        text: &words[index],
-        name: &names[index],
-    });
+    let names: Vec<String> = words
+        .iter()
+        .map(|word| format!("{option}: {word}"))
+        .collect();
+    let parts: Vec<Segment> = words
+        .iter()
+        .zip(&names)
+        .map(|(text, name)| Segment::Named { text, name })
+        .collect();
     widgets::segmented(ui, &parts, selected, size, look, palette)
 }
 
@@ -586,7 +682,15 @@ fn hint(ui: &Ui, at: egui::Pos2, right: f32, option: OptionId, settings: &Settin
             palette.dim,
         ),
         OptionId::ValueTags => return tags(ui, at, right, settings.value_tags, skin),
-        OptionId::AgentsOn | OptionId::LogDays | OptionId::AgentsNotify => return,
+        // What the Appearance pane's rows say is their small print.
+        OptionId::AgentsOn
+        | OptionId::LogDays
+        | OptionId::AgentsNotify
+        | OptionId::RowHeight
+        | OptionId::Zebra
+        | OptionId::ColumnTypes
+        | OptionId::TableList
+        | OptionId::EnvBar => return,
     };
     let text = text.layout(ui.ctx());
     if at.x + text.width() <= right {
```

The screen: `appearance_rows`, and the rows' hints from the board. The table list's hint names the toggle's key through the keymap; the bar's hint writes production as its label does.

```diff
diff --git a/src/ui/settings/terminal.rs b/src/ui/settings/terminal.rs
index 1ef6eb7..cd2ea72 100644
--- a/src/ui/settings/terminal.rs
+++ b/src/ui/settings/terminal.rs
@@ -18,7 +18,7 @@ use crate::ui::focus::{self, Ring};
 use crate::ui::value_tags::terminal_slots;
 use crate::ui::widgets::{self, ButtonSpec};
 
-use super::{Said, choices, file_pane, label, opened, sample_timestamp};
+use super::{Said, choices, file_pane, opened, sample_timestamp, terminal_label};
 
 /// The header's and the footer's heights, each without its rule.
 const HEADER: f32 = 40.0;
@@ -143,6 +143,7 @@ pub(super) fn show(
                 .auto_shrink([false, false])
                 .show(&mut list, |ui| match pane {
                     SettingsPane::General => rows(ui, &settings, row, &skin, actions),
+                    SettingsPane::Appearance => appearance_rows(ui, &settings, row, &skin, actions),
                     SettingsPane::Agents => agent_rows(ui, app, &settings, row, &skin, actions),
                 });
             if let Some(beside) = beside {
@@ -322,6 +323,43 @@ fn rows(ui: &mut Ui, settings: &Settings, cursor: usize, skin: &Skin, actions: &
     }
 }
 
+/// The Appearance pane's options, a row each, under the headings of their
+/// groups. Their columns line up across the groups.
+fn appearance_rows(
+    ui: &mut Ui,
+    settings: &Settings,
+    cursor: usize,
+    skin: &Skin,
+    actions: &mut Vec<Action>,
+) {
+    let room = ui.available_width() - 2.0 * ROW_SIDE;
+    let columns = columns(
+        ui,
+        room,
+        widths(ui, &OptionId::APPEARANCE, settings, skin),
+        skin,
+    );
+    let mut index = 0;
+    for (heading_text, top, options) in [
+        ("Grid", 8.0, &OptionId::GRID[..]),
+        ("Environment", 12.0, &OptionId::ENVIRONMENT[..]),
+    ] {
+        heading(ui, heading_text, top, skin);
+        for option in options.iter().copied() {
+            option_row(
+                ui,
+                (index, option),
+                index == cursor,
+                columns,
+                settings,
+                skin,
+                actions,
+            );
+            index += 1;
+        }
+    }
+}
+
 /// The Agents pane: the server's switch and its status, how to connect an
 /// agent, where access is set, and the log. Its rows are
 /// [`SettingsPane::rows`]'s, in order; the command, the line for other
@@ -609,7 +647,7 @@ fn widths(ui: &Ui, options: &[OptionId], settings: &Settings, skin: &Skin) -> (f
     let width = |text: &str| role.width(ui.ctx(), skin.look.faces, text);
     let labels = options
         .iter()
-        .map(|option| width(&format!("▌{}", skin.say(label(*option)))))
+        .map(|option| width(&format!("▌{}", skin.say(terminal_label(*option)))))
         .fold(0.0, f32::max);
     let values = options
         .iter()
@@ -665,13 +703,21 @@ fn value_width(ui: &Ui, option: OptionId, settings: &Settings, skin: &Skin) -> f
         OptionId::PageSize | OptionId::LogDays => {
             width(&format!("‹ {} ›", stepped_text(option, settings, skin)))
         }
-        OptionId::Timestamps | OptionId::GroupDigits => {
+        OptionId::Timestamps
+        | OptionId::GroupDigits
+        | OptionId::RowHeight
+        | OptionId::TableList
+        | OptionId::EnvBar => {
             let words = choices(option, settings);
             let between = width(" ") * words.len().saturating_sub(1) as f32;
             let segments = words.into_iter().map(|(word, _)| segment(ui, word, skin));
             segments.sum::<f32>() + between
         }
-        OptionId::ValueTags | OptionId::AgentsOn | OptionId::AgentsNotify => width("[x]"),
+        OptionId::ValueTags
+        | OptionId::AgentsOn
+        | OptionId::AgentsNotify
+        | OptionId::Zebra
+        | OptionId::ColumnTypes => width("[x]"),
     }
 }
 
@@ -782,14 +828,19 @@ fn option_row(
     let locale = skin.locale;
     // The row's button comes before its values', so that a click on a
     // value is the value's.
-    let (row, left, right, button) =
-        row_frame(ui, (index, label(option)), on_cursor, skin, actions);
+    let (row, left, right, button) = row_frame(
+        ui,
+        (index, terminal_label(option)),
+        on_cursor,
+        skin,
+        actions,
+    );
     let y = row.center().y;
     // A stepped option has no button that is its value: its row says it.
     if let Some(button) = button
         && matches!(option, OptionId::PageSize | OptionId::LogDays)
     {
-        let name = gettext(locale, label(option));
+        let name = gettext(locale, terminal_label(option));
         let value = stepped_text(option, settings, skin);
         button.widget_info(|| {
             let mut info = WidgetInfo::labeled(WidgetType::Button, true, &name);
@@ -811,10 +862,18 @@ fn option_row(
         OptionId::PageSize | OptionId::LogDays => {
             stepper(ui, cell, option, settings, skin, actions)
         }
-        OptionId::Timestamps | OptionId::GroupDigits => {
+        OptionId::Timestamps
+        | OptionId::GroupDigits
+        | OptionId::RowHeight
+        | OptionId::TableList
+        | OptionId::EnvBar => {
             segments(ui, cell, option, settings, skin, actions);
         }
-        OptionId::ValueTags | OptionId::AgentsOn | OptionId::AgentsNotify => {
+        OptionId::ValueTags
+        | OptionId::AgentsOn
+        | OptionId::AgentsNotify
+        | OptionId::Zebra
+        | OptionId::ColumnTypes => {
             check(ui, cell, option, settings, skin, actions);
         }
     }
@@ -968,6 +1027,26 @@ fn hint(ui: &Ui, option: OptionId, settings: &Settings, skin: &Skin) -> Text {
             )
             .add(role, &format!(" {}", skin.say("shows it")), palette.dim),
         OptionId::AgentsNotify => dim(&skin.say("a desktop notification")),
+        OptionId::RowHeight => dim(&skin.say("1 · 1.5 · 2 lines")),
+        OptionId::Zebra => Text::new(look),
+        OptionId::ColumnTypes => dim(&skin.say("second header line")),
+        // The key that flips the sidebar's list, from the keymap.
+        OptionId::TableList => Text::new(look)
+            .add(role, &format!("{} ", skin.say("default;")), palette.dim)
+            .add(role, &key(ui, look, Command::ToggleTree), palette.accent)
+            .add(role, &format!(" {}", skin.say("toggles")), palette.dim),
+        // Production as the bar's label writes it, in its own case.
+        OptionId::EnvBar => Text::new(look)
+            .add(
+                role,
+                crate::env::Environment::Production.label(crate::env::Platform::Omarchy),
+                palette.dim,
+            )
+            .add(
+                role,
+                &format!(" {}", skin.say("always tinted")),
+                palette.dim,
+            ),
     }
 }
 
@@ -1124,7 +1203,7 @@ fn choice(
 ) {
     let name = format!(
         "{}: {}",
-        gettext(skin.locale, label(value.option())),
+        gettext(skin.locale, terminal_label(value.option())),
         word.name(skin.locale)
     );
     let button = ButtonSpec::new(&name);
```

- [ ] **Step 5: Run the tests to see them pass**

Run: `cargo test --locked --lib appearance`
Expected: PASS (21 tests: 16 in `appearance_settings_tests`, 4 in `settings::tests`, and `settings_lists_an_agents_pane_after_appearance`). Then the three checks.

- [ ] **Step 6: Look at it**

Compare both panes with the "Settings · Appearance" boards by eye: a throwaway scene in `src/shots.rs` that opens Settings on the Appearance pane, rendered with `cargo test --locked --features shots --lib shots::<scene> -- --ignored`, kept local, and removed before the commit. Never commit the scene or its PNGs.

- [ ] **Step 7: Commit**

```bash
git add src/settings.rs src/model.rs src/ui/settings/mod.rs src/ui/settings/sheet.rs src/ui/settings/terminal.rs src/ui/agent_settings_tests.rs src/ui/appearance_settings_tests.rs
git commit -m "Add Settings › Appearance, with its Grid and Environment sections"
```

---

### Task 6: The website

**Files:**
- Modify: `docs/_reference/settings-and-files.md`, `docs/_guide/omarchy.md`

- [ ] **Step 1: Describe the pane, the keys and the file**

````diff
diff --git a/docs/_guide/omarchy.md b/docs/_guide/omarchy.md
index 107597d..ed6c228 100644
--- a/docs/_guide/omarchy.md
+++ b/docs/_guide/omarchy.md
@@ -83,7 +83,7 @@ list instead.
 | `j` `k`, arrows, Home, End | Move |
 | Enter | Open the table or view |
 | `/` | Filter the list |
-| `t` | Switch between tree and flat |
+| `t` | Switch between tree and flat. The connection keeps its choice; Settings › Appearance sets the default |
 | `R` | Reload the list |
 | `ga` | Show the Agents tab |
 | Ctrl+B | Hide the sidebar |
diff --git a/docs/_reference/settings-and-files.md b/docs/_reference/settings-and-files.md
index 3e49552..ac392ee 100644
--- a/docs/_reference/settings-and-files.md
+++ b/docs/_reference/settings-and-files.md
@@ -17,7 +17,18 @@ Cmd/Ctrl+, opens Settings. Changes apply right away.
 | **Numbers** | `1240.50` or `1,240.50`. Grouping is for display only: a copy gives the raw value | `1240.50` |
 | **Value tags** | Colors for enum, CHECK and boolean columns | On |
 
-A second pane, **Agents**, sets what the `[mcp]` keys
+A second pane, **Appearance**, sets how the grids and the connection bar
+are drawn. Each change shows in every open window at once.
+
+| Option | Choices | Default |
+| --- | --- | --- |
+| **Row density** (Omarchy: row height) | Compact, Regular or Roomy. On macOS and Windows a step of 8 points either side of Regular; in the Omarchy look one, one and a half or two lines. The grid keeps the row at its top | Regular |
+| **Alternate row shading** (Omarchy: alternate rows) | On or off, in the table grid and the SQL editor's results | Off on macOS and Windows, on in the Omarchy look |
+| **Show column types** (Omarchy: column types) | On or off: the second line of the grid's header | On |
+| **Table list** | Grouped by prefix (Omarchy: tree) or Flat. The default for a connection whose list you have not switched; the sidebar's switch (Omarchy: `t`) sets the connection's own | Grouped by prefix |
+| **Window header** (Omarchy: connection bar) | Tint + stripe (Omarchy: tint + label), or Stripe only (Omarchy: label only). Production keeps its tint either way | Tint + stripe |
+
+A third pane, **Agents**, sets what the `[mcp]` keys
 hold, and shows how to connect an agent:
 [Agents]({% link _guide/agents.md %}) is the guide.
 
@@ -57,6 +68,14 @@ sql_timeout_secs = 30
 enabled = false
 log_days = 30
 notify = true
+
+[appearance.grid]
+row_height = "regular"
+column_types = true
+table_list = "tree"
+
+[appearance.env]
+bar = "tint"
 ```
 
 | Key | Values | Default |
@@ -69,6 +88,11 @@ notify = true
 | `editor.sql_limit` | 1 to 10000 rows for each statement | `1000` |
 | `editor.sql_timeout_secs` | Seconds. `0` waits forever | `30` |
 | `appearance.theme` | The name of a palette file in the `themes` folder | Not set |
+| `appearance.grid.row_height` | `"compact"`, `"regular"` or `"roomy"` | `"regular"` |
+| `appearance.grid.zebra` | `true` or `false`. Written once you set it; without it, the look's default | Not set: off on macOS and Windows, on in the Omarchy look |
+| `appearance.grid.column_types` | `true` or `false` | `true` |
+| `appearance.grid.table_list` | `"tree"` or `"flat"`: how a connection lists its tables until you switch its own | `"tree"` |
+| `appearance.env.bar` | `"tint"` or `"label"`. Production is tinted either way | `"tint"` |
 | `mcp.enabled` | `true` lets agents reach the connections you open to them, through `tabletist mcp` | `false` |
 | `mcp.log_days` | 1 to 3650 days. How long the log of agents' calls keeps a call | `30` |
 | `mcp.notify` | `true` tells you when an agent connects: a notification on Linux, the Dock icon bouncing on macOS, the taskbar button flashing on Windows | `true` |
@@ -99,6 +123,7 @@ On Linux, `XDG_CONFIG_HOME` and `XDG_STATE_HOME` move the two folders.
 | `known_hosts.json` | Settings | The SSH host keys you trusted |
 | `themes/` | Settings | Palette files |
 | `last_used.json` | State | When each saved connection last connected |
+| `table_lists.json` | State | How each saved connection lists its tables, where you switched it in the sidebar |
 | `connections.moved` | State | A note that your connections are kept in `connections.toml` |
 | `labels.json` | State | The column you chose to label each table's rows in a foreign key's list, by saved connection and, on PostgreSQL, by database |
 | `tabletist.log`, `panic.log` | State | Logs |
````

- [ ] **Step 2: Check the site**

Run: `cd docs && bin/check`
Expected: the build and its checks pass.

- [ ] **Step 3: Commit**

```bash
git add docs/_reference/settings-and-files.md docs/_guide/omarchy.md
git commit -m "Describe Settings › Appearance and table_lists.json on the website"
```

---

## After the plan

Open one pull request for the step. Its description: a summary; the settings built (the tables above); the keys added (`appearance.grid.*`, `appearance.env.bar`, `table_lists.json`); the open questions and the defaults chosen (alternate rows off on macOS and on on Omarchy; no interface size on Omarchy; ⌘= / ⌘- / ⌘0 come with step 3, with egui's zoom off); and the departures table above. No screenshot goes to GitHub unblurred.
