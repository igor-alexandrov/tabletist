# Settings › Editor, step 3: Format and indent Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make Format and the Tab key follow Settings › Editor: reserved words in upper case, lower case or as typed, a list's commas trailing or leading, a `CASE`'s parts in by the indent, Tab and Shift+Tab putting in and taking out the indent set (on every selected line), and completion offering keywords in the chosen case. Show the Indent row and the Format section in both looks.

**Architecture:** The formatter (`crates/tabletist-db/src/sql/format.rs`) takes a `Style` (`Case`, `Commas`, `case_indent`) through `format_with`; `format` keeps today's style. The editor (`src/ui/sql_text.rs`) builds the style from the settings and formats with it. Tab and Shift+Tab are keymap commands (`IndentLines`, `OutdentLines`) that the editor reads itself, before its field and after the completion list's keys: a Tab with nothing selected becomes the indent's text in its place among the frame's events; a Tab over a selection and Shift+Tab are made on the lines (`indented`), one undo step each, the selection kept as it ran. Completion (`src/completion.rs`) takes the keyword case. The pane's rows come from `OptionId::TYPING`, `TERMINAL_TYPING` and the new `FORMAT`, with the Format key in the section's heading (`settings::said`'s `{format}`).

**Tech Stack:** Rust, egui (crmne fork) with AccessKit, headless tests through `src/testing.rs`.

**Spec:** `docs/superpowers/specs/2026-10-19-settings-editor-design.md`, sections "Steps" item 3 and "Step 3: Format and indent".

---

## Before Task 1

Commit the spec's step 3 section and this plan:

```bash
git add docs/superpowers/specs/2026-10-19-settings-editor-design.md docs/superpowers/plans/2026-10-20-settings-editor-3-format.md
git commit -m "Plan Settings › Editor step 3: Format and indent"
```

## Where this sits

Steps 1 and 2 are merged (#181, #183). The query history (#197) built the History section and most of step 6 since; the Editor pane now has Running queries, Typing and History. This plan is a branch of `main` at `b3a86b29`.

## Departures, for a yes or a no before code

| Asked, or today | Built | Why |
|---|---|---|
| Today: Format sets a `CASE`'s `WHEN` and `ELSE` four in | Two in by default, the indent's default; four with 4 spaces or Tab | "Reformats … with … the indent from Typing" |
| Indent Tab, in Format | Four spaces | A tab cannot keep the river's columns |
| Omarchy today: Format uppercases | Lower case until set: the look's default (the board) | Set by step 1's look defaults, acted on here |
| Today: completion follows the case typed (`sel` gives `select`, `SEL` gives `SELECT`) | The case set, whatever is typed; upper case with Keep | "Keywords are inserted in the chosen keyword case; with Keep, in uppercase" |
| egui today: Tab replaces a selection with a tab character; Shift+Tab takes a tab or four spaces off the cursor's line | Tab: an indent at the cursor, or at the start of every line a selection touches (not blank ones), the selection kept; Shift+Tab: one indent off every line touched | What an editor's Tab does over a selection; "Indent" says no more |
| (where a leading comma goes) | On the river's edge, one column left of the list's items: `SELECT a` then `     , b`; only in a `SELECT`'s and a `WITH`'s list; a comma with a comment after it, on its line or the next, stays at the end of its line | No board draws leading commas; this keeps the river and the comment where it was |
| Omarchy board: "format · = in normal mode" | The same words; `=` is the keymap's, read with the keyboard out of the text, as today | Normal mode is the keyboard out of the text until vim (step 7) |
| "Each is one undo step" | A Tab at the cursor is typed text, and undoes with what is typed beside it; a selection's Tab and Shift+Tab are a step each | Typed in its place among the frame's events, the Tab keeps its order with the text around it |
| (the in-app shortcuts sheet) | Indent and Outdent are not listed there (their row is empty); the website lists Tab and Shift+Tab | The sheet's rows are the Keyboard board's |
| (a selection that starts at a line's start) | Grows to take in the new indent; its moving end stays its moving end | It covers the same text and the indent before it |

A reviewer given the draft broke it with tests of its own, and the draft now fixes what it found:

- Tab or Shift+Tab over a selection made backwards gave it back made forwards, so the next Shift+Arrow moved the wrong end. The moving end stays the moving end now (`a_selection_made_backwards_keeps_its_moving_end`).
- A Tab and text typed in one frame landed in the wrong order (`a  ` for Tab then `a`): the indent waited for the next frame, behind an action. The editor reads Tab and Shift+Tab itself now, before its field: a Tab with nothing selected is the indent's text in its place among the frame's events (`a_tab_and_text_typed_in_one_frame_keep_their_order`). A line edit (a selection's, Shift+Tab's) is made before the frame's other events; text typed before it in the same frame lands after it, as a selection typed over is replaced anyway.

It ran 33 awkward scripts in three dialects through all twelve styles (comments beside commas, MySQL's `--`, nested lists, windows, sigils, CRLF, scripts already in leading style, a selection) and found no token changed, no case moved under Keep, no second pass that changed anything, and the cursor where it should be. Tab is not taken from Settings, a dialog, the grid or an editor without the keyboard, and Omarchy's normal mode leaves it alone.

Left as they are, for the record:

- Tab with an open completion list that has no rows yet (names loading) indents, as Tab put a tab in before.
- `AcceptAi` (Tab, while an assistant's answer is offered) is reserved, not built: when it is, it must be read before the editor's `IndentLines`, as the completion list is.
- On MySQL, `SELECT a, b INTO @x, @y` breaks the `INTO` list as the select list's, leading or trailing, as Format already did.
- A held Tab or Shift+Tab over a selection makes a step of the undo history for each repeat, as a held key types.

A second reviewer read this plan against its diffs and the spec. Its points are in: the random scripts now run in all twelve styles; new tests hold a selection's Tab as one undo step, Tab without the editor's keyboard (Omarchy's normal mode, an editor left with Esc), Shift+Tab under an open completion list, the rows' order in both looks, and completion offered against the case typed; doc comments that still said "uppercase" now say the style's case; the departures above name the Tab at the cursor's undo, the shortcuts sheet and a selection's growth; the docs add `WITH` lists, blank lines and completion's case.

## Files

| File | What it does after this plan |
|---|---|
| `crates/tabletist-db/src/sql/format.rs` | `Style`, `Case`, `Commas`; `format_with`; the layout's case, leading commas and `CASE` indent from the style. |
| `src/settings.rs` | `OptionId::TYPING` and `TERMINAL_TYPING` with Indent, `OptionId::FORMAT`, `EDITOR_LATER` without them. |
| `src/model.rs` | The Omarchy cursor's Editor rows. |
| `src/ui/settings/{mod,sheet,terminal}.rs` | The Format section in both looks, its heading naming the Format key. |
| `src/keymap.rs` | `IndentLines` (Tab) and `OutdentLines` (Shift+Tab) in the SQL editor. |
| `src/ui/sql_text.rs` | `format_style`, `indented`, `indent_script`: Format with the settings' style; Tab and Shift+Tab read before the field, as typed text or line edits, one undo step each. |
| `src/completion.rs`, `src/app.rs` | Keywords in the case set. |
| `src/ui/format_settings_tests.rs` | New: this step's headless tests. |
| `src/ui/mod.rs`, `src/ui/complete_tests.rs`, `src/ui/editor_settings_tests.rs` | Declares the tests; tests that met the old Tab, the old case and step 1's rows. |
| `docs/_reference/settings-and-files.md`, `docs/_reference/keyboard-shortcuts.md`, `docs/_guide/sql-editor.md`, `docs/_guide/macos.md`, `docs/_guide/omarchy.md` | The rows, the keys, Tab and Format. |

Every task ends with the repository's checks passing:

    cargo fmt --all --check
    cargo clippy --locked --workspace --all-targets -- -D warnings
    cargo test --locked --workspace --all-targets
    RUSTDOCFLAGS='-D warnings' cargo doc --locked --workspace --no-deps

(`cargo` is `~/.cargo/bin/cargo` in these sessions.) Commit after each task, signed as usual. If signing says "agent refused operation", 1Password is locked: leave the change staged and say so.

---

### Task 1: The formatter's style

**Files:**
- Modify: `crates/tabletist-db/src/sql/format.rs`

- [ ] **Step 1: Write the failing tests**

```diff
diff --git a/crates/tabletist-db/src/sql/format.rs b/crates/tabletist-db/src/sql/format.rs
--- a/crates/tabletist-db/src/sql/format.rs
+++ b/crates/tabletist-db/src/sql/format.rs
@@ -879,9 +879,19 @@ mod tests {
         dialect: Dialect,
         script: &str,
         selection: Option<Range<usize>>,
+    ) -> (String, Range<usize>) {
+        laid_in(dialect, script, selection, Style::default())
+    }
+
+    /// [`laid`], in `style`.
+    fn laid_in(
+        dialect: Dialect,
+        script: &str,
+        selection: Option<Range<usize>>,
+        style: Style,
     ) -> (String, Range<usize>) {
         let old = tokenize(dialect, script);
-        let Some((region, laid)) = laid_out(dialect, script, &old, selection) else {
+        let Some((region, laid)) = laid_out(dialect, script, &old, selection, style) else {
             return (script.to_owned(), 0..0);
         };
         let result = [&script[..region.start], &laid, &script[region.end..]].concat();
@@ -937,15 +947,43 @@ mod tests {
     /// has nothing to change. Checks what every result owes: the same
     /// tokens, and nothing left for a second pass.
     fn formatted(dialect: Dialect, script: &str) -> String {
-        let (result, _) = laid(dialect, script, None);
-        let checked = format(dialect, script, None, 0).map(|formatted| formatted.text);
+        formatted_in(dialect, script, Style::default())
+    }
+
+    /// [`formatted`], in `style`.
+    fn formatted_in(dialect: Dialect, script: &str, style: Style) -> String {
+        let (result, _) = laid_in(dialect, script, None, style);
+        let checked = format_with(dialect, script, None, 0, style).map(|formatted| formatted.text);
         assert_eq!(checked.as_deref().unwrap_or(script), result, "{script:?}");
         // The second pass is the layout's too: `format` returns `None` as
         // well for a layout its check refused.
-        assert_eq!(laid(dialect, &result, None).0, result, "{script:?}");
+        assert_eq!(
+            laid_in(dialect, &result, None, style).0,
+            result,
+            "{script:?}"
+        );
         result
     }
 
+    /// Every style but the default: each case, each side of the comma, and
+    /// each indent the editor sets (two and four).
+    fn other_styles() -> Vec<Style> {
+        let mut all = Vec::new();
+        for keywords in [Case::Upper, Case::Lower, Case::Keep] {
+            for commas in [Commas::Trailing, Commas::Leading] {
+                for case_indent in [2, 4] {
+                    all.push(Style {
+                        keywords,
+                        commas,
+                        case_indent,
+                    });
+                }
+            }
+        }
+        all.retain(|style| *style != Style::default());
+        all
+    }
+
     fn pg(script: &str) -> String {
         formatted(Dialect::Postgres, script)
     }
@@ -1751,6 +1789,7 @@ mod tests {
             seed ^= seed << 17;
             (seed % below as u64) as usize
         };
+        let styles = other_styles();
         for _ in 0..2_000 {
             let mut script = String::new();
             for _ in 0..next(40) {
@@ -1771,10 +1810,114 @@ mod tests {
                 if !part.is_empty() {
                     assert_eq!(laid(dialect, &result, Some(part)).0, result, "{script:?}");
                 }
+                for style in &styles {
+                    formatted_in(dialect, &script, *style);
+                }
             }
         }
     }
 
+    #[test]
+    fn keywords_are_written_in_the_style_s_case_and_names_never() {
+        let script = "Select Title, count(*) as n from Books where Genre in ('A') group by Title";
+        let style = |keywords| Style {
+            keywords,
+            ..Style::default()
+        };
+        assert_eq!(
+            formatted_in(Dialect::Postgres, script, style(Case::Lower)),
+            lines(&[
+                "select Title,",
+                "       count(*) as n",
+                "  from Books",
+                " where Genre in ('A')",
+                " group by Title",
+            ])
+        );
+        assert_eq!(
+            formatted_in(Dialect::Postgres, script, style(Case::Keep)),
+            lines(&[
+                "Select Title,",
+                "       count(*) as n",
+                "  from Books",
+                " where Genre in ('A')",
+                " group by Title",
+            ])
+        );
+        // A word read as structure is lowercased with the rest.
+        assert_eq!(
+            formatted_in(
+                Dialect::Postgres,
+                "SELECT 1 UNION SELECT 2 EXCEPT SELECT 3",
+                style(Case::Lower)
+            ),
+            lines(&["select 1", " union", "select 2", "except", "select 3"])
+        );
+    }
+
+    #[test]
+    fn leading_commas_stand_on_the_river_s_edge() {
+        let leading = Style {
+            commas: Commas::Leading,
+            ..Style::default()
+        };
+        assert_eq!(
+            formatted_in(
+                Dialect::Postgres,
+                "select p.name as publisher, count(*) as covers from book_covers c \
+                join publishers p on p.id = c.publisher_id group by p.name;",
+                leading
+            ),
+            lines(&[
+                "SELECT p.name AS publisher",
+                "     , count(*) AS covers",
+                "  FROM book_covers c",
+                "  JOIN publishers p ON p.id = c.publisher_id",
+                " GROUP BY p.name;",
+            ])
+        );
+        // A WITH's list too, and the select list of a query in
+        // parentheses at its own river; a comment after a comma keeps the
+        // comma where it is.
+        assert_eq!(
+            formatted_in(
+                Dialect::Postgres,
+                "with a as (select 1, 2), b as (select 3) select x, -- why\n y from a",
+                leading
+            ),
+            lines(&[
+                "  WITH a AS (SELECT 1",
+                "                  , 2)",
+                "     , b AS (SELECT 3)",
+                "SELECT x, -- why",
+                "       y",
+                "  FROM a",
+            ])
+        );
+    }
+
+    #[test]
+    fn a_case_s_parts_stand_in_by_the_style_s_indent() {
+        let two = Style {
+            case_indent: 2,
+            ..Style::default()
+        };
+        assert_eq!(
+            formatted_in(
+                Dialect::Postgres,
+                "select case when stock = 0 then 'out' else 'ok' end from books",
+                two
+            ),
+            lines(&[
+                "SELECT CASE",
+                "         WHEN stock = 0 THEN 'out'",
+                "         ELSE 'ok'",
+                "       END",
+                "  FROM books",
+            ])
+        );
+    }
+
     #[test]
     fn the_check_tells_a_changed_token() {
         let same = |old: &str, new: &str| {
```

- [ ] **Step 2: Run the tests to see them fail**

Run: `cargo test --locked -p tabletist-db --lib sql::format`
Expected: FAIL to compile (24 errors): `sql::format` has no `Style`, `Case`, `Commas` or `format_with`, and `laid_out` takes no style.

- [ ] **Step 3: Give the layout a style**

```diff
diff --git a/crates/tabletist-db/src/sql/format.rs b/crates/tabletist-db/src/sql/format.rs
--- a/crates/tabletist-db/src/sql/format.rs
+++ b/crates/tabletist-db/src/sql/format.rs
@@ -1,7 +1,7 @@
-//! Format: lays a script's queries out in river style and uppercases
-//! reserved words. It works on the tokens and never parses: only
-//! whitespace and the case of keywords change, and a check at the end
-//! holds it to that.
+//! Format: lays a script's queries out in river style and sets the case
+//! of reserved words, as a [`Style`] says. It works on the tokens and
+//! never parses: only whitespace and the case of keywords change, and a
+//! check at the end holds it to that.
 
 use std::borrow::Cow;
 use std::ops::Range;
@@ -18,7 +18,8 @@ pub struct Formatted {
     pub cursor: usize,
 }
 
-/// The keywords Format uppercases: the ones that cannot be an unquoted
+/// The keywords whose case Format sets (upper case in the default style):
+/// the ones that cannot be an unquoted
 /// table name or alias where case would matter. Every one the MySQL
 /// dialect reads as a keyword is reserved in MySQL 5.7, MySQL 8 and
 /// MariaDB (the MySQL tests ask the server they run against); the words
@@ -93,8 +94,9 @@ pub const UPPERCASED: &[&str] = &[
     "WITH",
 ];
 
-/// Keywords that can be names, uppercased where the layout read them as
-/// structure, on the dialects that compare unquoted names without case.
+/// Keywords that can be names, in the style's case where the layout read
+/// them as structure, on the dialects that compare unquoted names without
+/// case.
 const STRUCTURE: &[&str] = &[
     "EXCEPT",
     "FULL",
@@ -125,9 +127,46 @@ const JOIN_LEADERS: &[&str] = &[
 /// The width of the river: `SELECT`.
 const RIVER: usize = 6;
 
-/// How far a `WHEN`, an `ELSE` and what follows them stand in from their
-/// `CASE`.
-const CASE_INDENT: usize = 4;
+/// How Format writes what it lays out.
+#[derive(Clone, Copy, Debug, PartialEq, Eq)]
+pub struct Style {
+    /// The case of the reserved words it would uppercase.
+    pub keywords: Case,
+    /// Where the comma between two items of a list that breaks goes.
+    pub commas: Commas,
+    /// How far a `WHEN`, an `ELSE` and what follows them stand in from
+    /// their `CASE`, in spaces.
+    pub case_indent: usize,
+}
+
+/// Today's style: reserved words in upper case, a comma at the end of its
+/// item's line, a `CASE`'s parts four in.
+impl Default for Style {
+    fn default() -> Self {
+        Self {
+            keywords: Case::Upper,
+            commas: Commas::Trailing,
+            case_indent: 4,
+        }
+    }
+}
+
+/// The case reserved words are written in: upper, lower, or as typed.
+#[derive(Clone, Copy, Debug, PartialEq, Eq)]
+pub enum Case {
+    Upper,
+    Lower,
+    Keep,
+}
+
+/// Where a list's comma goes when the list breaks: at the end of the line
+/// of the item before it, or at the start of the next item's, on the
+/// river's edge.
+#[derive(Clone, Copy, Debug, PartialEq, Eq)]
+pub enum Commas {
+    Trailing,
+    Leading,
+}
 
 /// How deep queries in parentheses are laid out as blocks. The layout
 /// recurses once for each, so a script nested without end (a generated
@@ -136,16 +175,28 @@ const CASE_INDENT: usize = 4;
 const MAX_DEPTH: usize = 64;
 
 /// `script` with the statements `selection` overlaps formatted, or all of
-/// them when it is `None` or empty, and where `cursor` went. All three are
-/// byte offsets. `None` when there is nothing to change.
+/// them when it is `None` or empty, and where `cursor` went, in the
+/// default [`Style`]. All three are byte offsets. `None` when there is
+/// nothing to change.
 pub fn format(
     dialect: Dialect,
     script: &str,
     selection: Option<Range<usize>>,
     cursor: usize,
+) -> Option<Formatted> {
+    format_with(dialect, script, selection, cursor, Style::default())
+}
+
+/// [`format()`], in `style`.
+pub fn format_with(
+    dialect: Dialect,
+    script: &str,
+    selection: Option<Range<usize>>,
+    cursor: usize,
+    style: Style,
 ) -> Option<Formatted> {
     let tokens = tokenize(dialect, script);
-    let (region, laid) = laid_out(dialect, script, &tokens, selection)?;
+    let (region, laid) = laid_out(dialect, script, &tokens, selection, style)?;
     let text = [&script[..region.start], &laid, &script[region.end..]].concat();
     if text == script {
         return None;
@@ -171,6 +222,7 @@ fn laid_out(
     script: &str,
     tokens: &[Token],
     selection: Option<Range<usize>>,
+    style: Style,
 ) -> Option<(Range<usize>, String)> {
     let region = region(script, tokens, selection)?;
     // The indentation of its first line is Format's to set.
@@ -180,6 +232,7 @@ fn laid_out(
     let line = script[..region.start].rfind('\n').map_or(0, |end| end + 1);
     let mut layout = Layout {
         dialect,
+        style,
         items: items(script, tokens, &region),
         at: 0,
         writer: Writer {
@@ -413,6 +466,7 @@ impl Block {
 
 struct Layout<'a> {
     dialect: Dialect,
+    style: Style,
     items: Vec<Item<'a>>,
     /// The item to write next.
     at: usize,
@@ -532,8 +586,20 @@ impl<'a> Layout<'a> {
                 _ if item.is(",") => {
                     let list = block.frames.is_empty()
                         && matches!(block.clause, Clause::Select | Clause::With);
-                    self.put(&mut block, Place::Inline, false);
-                    block.broken = list;
+                    // A leading comma starts the next item's line, on the
+                    // river's edge: not where a comment follows it, which
+                    // would come between the two.
+                    let next = self.items.get(self.at + 1);
+                    let leads = list
+                        && self.style.commas == Commas::Leading
+                        && next.is_some_and(|next| next.kind != TokenKind::Comment);
+                    if leads {
+                        let edge = (block.base + RIVER).saturating_sub(1);
+                        self.put(&mut block, Place::Line(edge), false);
+                    } else {
+                        self.put(&mut block, Place::Inline, false);
+                        block.broken = list;
+                    }
                 }
                 _ => self.put(&mut block, Place::Inline, false),
             }
@@ -574,7 +640,8 @@ impl<'a> Layout<'a> {
         }
         if let Some(Frame::Case(column)) = block.frames.last().copied() {
             if is("WHEN") || is("ELSE") {
-                return self.put(block, Place::Line(column + CASE_INDENT), false);
+                let indent = self.style.case_indent;
+                return self.put(block, Place::Line(column + indent), false);
             }
             if is("END") {
                 block.frames.pop();
@@ -654,7 +721,8 @@ impl<'a> Layout<'a> {
         self.word(end, &["JOIN"]).then_some(end - at + 1)
     }
 
-    /// Whether the keyword at `index`, read as structure, is uppercased
+    /// Whether the keyword at `index`, read as structure, takes the style's
+    /// case
     /// though it can be a name: not on MySQL, where it may be an alias and
     /// an alias's case can matter.
     fn structure(&self, index: usize) -> bool {
@@ -679,7 +747,8 @@ impl<'a> Layout<'a> {
     }
 
     /// Writes the item at `at` and what separates it from the one before.
-    /// `structure`: it is a word read as structure, uppercased whatever
+    /// `structure`: it is a word read as structure, in the style's case
+    /// whatever
     /// the list says.
     fn put(&mut self, block: &mut Block, place: Place, structure: bool) {
         let at = self.at;
@@ -714,20 +783,25 @@ impl<'a> Layout<'a> {
         block.headed = false;
     }
 
-    /// The item's text, uppercased when it is a keyword on the list (or
-    /// `structure` says so) and not a name.
+    /// The item's text, in the style's case when it is a keyword on the
+    /// list (or `structure` says so) and not a name.
     fn cased(&self, index: usize, structure: bool) -> Cow<'a, str> {
         let item = self.items[index];
-        let upper = item.kind == TokenKind::Keyword
+        let reserved = item.kind == TokenKind::Keyword
             && !self.named(index)
             && (structure
                 || UPPERCASED
                     .iter()
                     .any(|word| word.eq_ignore_ascii_case(item.text)));
-        if upper && item.text.bytes().any(|byte| byte.is_ascii_lowercase()) {
-            Cow::Owned(item.text.to_ascii_uppercase())
-        } else {
-            Cow::Borrowed(item.text)
+        let bytes = item.text.bytes();
+        match self.style.keywords {
+            Case::Upper if reserved && bytes.clone().any(|byte| byte.is_ascii_lowercase()) => {
+                Cow::Owned(item.text.to_ascii_uppercase())
+            }
+            Case::Lower if reserved && bytes.clone().any(|byte| byte.is_ascii_uppercase()) => {
+                Cow::Owned(item.text.to_ascii_lowercase())
+            }
+            Case::Upper | Case::Lower | Case::Keep => Cow::Borrowed(item.text),
         }
     }
 
```

- [ ] **Step 4: Run the tests to see them pass**

Run: `cargo test --locked -p tabletist-db --lib sql::format`
Expected: PASS, the random scripts in all twelve styles among them. Then the four checks above.

- [ ] **Step 5: Commit**

```bash
git add crates/tabletist-db/src/sql/format.rs
git commit -m "Let Format write keywords in either case or as typed, commas leading, and a CASE in by an indent"
```

---

### Task 2: The Indent row and the Format section

**Files:**
- Modify: `src/settings.rs`, `src/model.rs`, `src/ui/settings/mod.rs`, `src/ui/settings/sheet.rs`, `src/ui/settings/terminal.rs`
- Create: `src/ui/format_settings_tests.rs`
- Modify (tests): `src/ui/mod.rs`, `src/ui/editor_settings_tests.rs`

- [ ] **Step 1: Write the failing tests**

```diff
diff --git a/src/settings.rs b/src/settings.rs
--- a/src/settings.rs
+++ b/src/settings.rs
@@ -4601,6 +4601,7 @@ bar = \"label\"  # tint | label
             let mut shown = [
                 &OptionId::RUNNING[..],
                 sections,
+                &OptionId::FORMAT[..],
                 &OptionId::HISTORY[..],
                 &OptionId::EDITOR_LATER[..],
             ]
```

```diff
diff --git a/src/ui/editor_settings_tests.rs b/src/ui/editor_settings_tests.rs
--- a/src/ui/editor_settings_tests.rs
+++ b/src/ui/editor_settings_tests.rs
@@ -528,7 +528,7 @@ fn the_sheet_sets_each_editor_option_with_a_click() {
         assert!(shows(&mut harness, words), "{words}");
     }
     // The rows of what is not built yet are not there.
-    for words in ["Keybindings", "Indent", "Wrap", "FORMAT", "Keywords"] {
+    for words in ["Keybindings", "Wrap", "Close brackets and quotes"] {
         assert!(!shows(&mut harness, words), "{words}");
     }
     let saves = settings_saves(&harness);
@@ -658,11 +658,19 @@ fn the_screen_sets_each_editor_option_with_its_keys() {
     // The whole tab, with space.
     type_key(&mut harness, Key::J, "j");
     harness.press(Key::Space, Modifiers::NONE);
-    // Line numbers from relative to absolute; completion on the key.
+    // Line numbers from relative to absolute; four spaces; completion on
+    // the key; keywords from the look's lower case to upper; leading
+    // commas, with space.
     type_key(&mut harness, Key::J, "j");
     type_key(&mut harness, Key::L, "l");
     type_key(&mut harness, Key::J, "j");
     type_key(&mut harness, Key::L, "l");
+    type_key(&mut harness, Key::J, "j");
+    type_key(&mut harness, Key::L, "l");
+    type_key(&mut harness, Key::J, "j");
+    type_key(&mut harness, Key::H, "h");
+    type_key(&mut harness, Key::J, "j");
+    harness.press(Key::Space, Modifiers::NONE);
     harness.settle();
     let settings = &harness.app.settings;
     assert_eq!(settings.new_tab, NewTab::Connection);
@@ -670,8 +678,11 @@ fn the_screen_sets_each_editor_option_with_its_keys() {
     assert_eq!(settings.sql_timeout_secs, Some(15));
     assert_eq!(settings.run_scope, RunScope::All);
     assert_eq!(settings.line_numbers, Some(LineNumbers::Absolute));
+    assert_eq!(settings.indent, crate::settings::Indent::Four);
     assert_eq!(settings.autocomplete, Autocomplete::Manual);
-    assert_eq!(settings_saves(&harness), saves + 6);
+    assert_eq!(settings.keywords, Some(KeywordCase::Upper));
+    assert_eq!(settings.commas, crate::settings::Commas::Leading);
+    assert_eq!(settings_saves(&harness), saves + 9);
     // The file has each of them, and R puts the cursor's back.
     let text = harness.app.settings_file.text.clone();
     assert!(
@@ -681,7 +692,10 @@ fn the_screen_sets_each_editor_option_with_its_keys() {
     assert!(text.contains("autocomplete = \"manual\""), "{text}");
     harness.press(Key::R, Modifiers::SHIFT);
     harness.settle();
-    assert_eq!(harness.app.settings.autocomplete, Autocomplete::Typing);
+    assert_eq!(
+        harness.app.settings.commas,
+        crate::settings::Commas::Trailing
+    );
 }
 
 #[test]
```

```diff
diff --git a/src/ui/format_settings_tests.rs b/src/ui/format_settings_tests.rs
new file mode 100644
--- /dev/null
+++ b/src/ui/format_settings_tests.rs
@@ -0,0 +1,77 @@
+//! Settings › Editor's Format section and indent, in both looks: what
+//! Format writes, what Tab and Shift+Tab put in and take out, the case
+//! completion offers keywords in, and the rows that set them.
+
+use crate::keymap::{Command as Keyed, Scope};
+use crate::model::{Action, SettingsPane};
+use crate::settings::{Commas, Indent, KeywordCase};
+use crate::testing::Harness;
+use crate::theme::Look;
+
+use super::appearance_settings_tests::{settings_saves, shows};
+
+/// Settings on its Editor pane.
+fn on_the_pane(harness: &mut Harness) {
+    harness.app.apply(Action::ShowSettings);
+    harness
+        .app
+        .apply(Action::ShowSettingsPane(SettingsPane::Editor));
+    harness.settle();
+}
+
+#[test]
+fn the_sheet_has_the_indent_and_the_format_section_and_sets_them() {
+    let look = Look::macos();
+    let mut harness = Harness::with_size(egui::vec2(1440.0, 1400.0));
+    harness.set_look(look);
+    on_the_pane(&mut harness);
+    // The heading names the Format key, as the keymap writes it.
+    let key = crate::ui::keys::written_in(&harness.ctx, &look, Keyed::Format, Scope::SqlEditor);
+    let heading = format!("FORMAT · {}", key.as_str()).to_uppercase();
+    for words in [heading.as_str(), "Indent", "Keywords", "Commas"] {
+        assert!(shows(&mut harness, words), "{words}");
+    }
+    // Indent leads the Typing section, as the board has it.
+    let top = |harness: &Harness, words: &str| harness.painted_rect(words).expect(words).top();
+    assert!(top(&harness, "TYPING") < top(&harness, "Indent"));
+    assert!(top(&harness, "Indent") < top(&harness, "Autocomplete"));
+    assert!(top(&harness, "Autocomplete") < top(&harness, "Keywords"));
+    let saves = settings_saves(&harness);
+    harness.click("Indent: Tab");
+    harness.click("Keywords: lowercase");
+    harness.click("Commas: Leading");
+    harness.settle();
+    let settings = &harness.app.settings;
+    assert_eq!(settings.indent, Indent::Tab);
+    assert_eq!(settings.keywords, Some(KeywordCase::Lower));
+    assert_eq!(settings.commas, Commas::Leading);
+    assert_eq!(settings_saves(&harness), saves + 3);
+    // UPPERCASE is the look's own: picked while lower is set, it is set.
+    harness.click("Keywords: UPPERCASE");
+    assert_eq!(harness.app.settings.keywords, Some(KeywordCase::Upper));
+}
+
+#[test]
+fn the_screen_has_the_indent_and_the_format_section() {
+    let look = Look::omarchy();
+    let mut harness = Harness::with_size(egui::vec2(1920.0, 1080.0));
+    harness.set_look(look);
+    on_the_pane(&mut harness);
+    let key = crate::ui::keys::written_in(&harness.ctx, &look, Keyed::Format, Scope::SqlEditor);
+    let heading = format!("format · {} in normal mode", key.as_str());
+    for words in [heading.as_str(), "indent", "2 spaces", "keywords", "commas"] {
+        assert!(shows(&mut harness, words), "{words}");
+    }
+    // As the board orders them: line numbers, indent, autocomplete.
+    let top = |harness: &Harness, words: &str| harness.painted_rect(words).expect(words).top();
+    assert!(top(&harness, "line numbers") < top(&harness, "indent"));
+    assert!(top(&harness, "indent") < top(&harness, "autocomplete"));
+    // The case is the choice: UPPER as it is, lower chosen.
+    assert!(shows(&mut harness, "UPPER"));
+    let tree = harness.settle();
+    let chosen = tree.nodes.iter().any(|(_, node)| {
+        node.label() == Some("Keywords: lower")
+            && node.toggled() == Some(egui::accesskit::Toggled::True)
+    });
+    assert!(chosen, "the look's lower case is the one chosen");
+}
```

```diff
diff --git a/src/ui/mod.rs b/src/ui/mod.rs
--- a/src/ui/mod.rs
+++ b/src/ui/mod.rs
@@ -38,6 +38,8 @@ mod env_tests;
 pub mod filter_bar;
 pub mod focus;
 pub mod format;
+#[cfg(test)]
+mod format_settings_tests;
 pub mod grid;
 pub mod help;
 pub mod history;
```

- [ ] **Step 2: Run the tests to see them fail**

Run: `cargo test --locked --lib -- settings ui::format_settings_tests ui::editor_settings_tests`
Expected: FAIL to compile (1 error): `OptionId` has no `FORMAT`. With it in place, the two pane tests fail (no Indent row, no Format section) and `the_screen_sets_each_editor_option_with_its_keys` steps the wrong rows.

- [ ] **Step 3: Show the rows**

```diff
diff --git a/src/model.rs b/src/model.rs
--- a/src/model.rs
+++ b/src/model.rs
@@ -2792,7 +2792,10 @@ impl SettingsPane {
                 SettingsRow::Option(OptionId::Timeout),
                 SettingsRow::Option(OptionId::RunScope),
                 SettingsRow::Option(OptionId::LineNumbers),
+                SettingsRow::Option(OptionId::Indent),
                 SettingsRow::Option(OptionId::Autocomplete),
+                SettingsRow::Option(OptionId::Keywords),
+                SettingsRow::Option(OptionId::Commas),
                 SettingsRow::Option(OptionId::HistoryKeep),
             ],
             Self::Agents => &[
```

```diff
diff --git a/src/settings.rs b/src/settings.rs
--- a/src/settings.rs
+++ b/src/settings.rs
@@ -1070,24 +1070,29 @@ impl OptionId {
     ];
     /// The Editor pane's options whose features are not built yet: no row
     /// draws them, and their keys are read, kept and reset.
-    pub const EDITOR_LATER: [OptionId; 7] = [
+    pub const EDITOR_LATER: [OptionId; 4] = [
         Self::ConfirmUnsafeWrites,
         Self::EditorKeys,
-        Self::Indent,
         Self::ClosePairs,
         Self::Wrap,
-        Self::Keywords,
-        Self::Commas,
     ];
     /// The Editor pane's Running queries section, as far as its options
     /// are built.
     pub const RUNNING: [OptionId; 4] =
         [Self::NewTab, Self::RowLimit, Self::Timeout, Self::RunScope];
     /// Its Typing section on macOS and Windows.
-    pub const TYPING: [OptionId; 3] = [Self::Autocomplete, Self::LineNumbers, Self::HighlightLine];
+    pub const TYPING: [OptionId; 4] = [
+        Self::Indent,
+        Self::Autocomplete,
+        Self::LineNumbers,
+        Self::HighlightLine,
+    ];
     /// The terminal look's, as its board orders them, with no row for the
     /// cursor's line.
-    pub const TERMINAL_TYPING: [OptionId; 2] = [Self::LineNumbers, Self::Autocomplete];
+    pub const TERMINAL_TYPING: [OptionId; 3] =
+        [Self::LineNumbers, Self::Indent, Self::Autocomplete];
+    /// Its Format section, in both looks.
+    pub const FORMAT: [OptionId; 2] = [Self::Keywords, Self::Commas];
     /// Its History section, in both looks.
     pub const HISTORY: [OptionId; 1] = [Self::HistoryKeep];
     /// The Agents pane's, in the order it has them.
```

```diff
diff --git a/src/ui/settings/mod.rs b/src/ui/settings/mod.rs
--- a/src/ui/settings/mod.rs
+++ b/src/ui/settings/mod.rs
@@ -69,20 +69,26 @@ pub(super) fn label(option: OptionId) -> &'static str {
     }
 }
 
-/// A row's label as the window writes it, translated: with the run key in
-/// place of `{run}`, as the keymap writes it for the look.
+/// A row's label or a section's heading as the window writes it,
+/// translated: with the run key in place of `{run}` and the editor's
+/// Format key in place of `{format}`, as the keymap writes them for the
+/// look.
 pub(super) fn said(
     ctx: &egui::Context,
     look: &crate::theme::Look,
     locale: Locale,
     text: &'static str,
 ) -> String {
-    let said = gettext(locale, text);
-    if !said.contains("{run}") {
-        return said.into_owned();
+    let mut said = gettext(locale, text).into_owned();
+    if said.contains("{run}") {
+        let key = crate::ui::keys::written(ctx, look, Command::RunStatement);
+        said = said.replace("{run}", key.as_str());
     }
-    let key = crate::ui::keys::written(ctx, look, Command::RunStatement);
-    said.replace("{run}", key.as_str())
+    if said.contains("{format}") {
+        let key = crate::ui::keys::written_in(ctx, look, Command::Format, Scope::SqlEditor);
+        said = said.replace("{format}", key.as_str());
+    }
+    said
 }
 
 /// The label of an option's row in the terminal look, which names some
```

```diff
diff --git a/src/ui/settings/sheet.rs b/src/ui/settings/sheet.rs
--- a/src/ui/settings/sheet.rs
+++ b/src/ui/settings/sheet.rs
@@ -238,6 +238,7 @@ pub(super) fn show(
                         &[
                             ("Running queries", &OptionId::RUNNING),
                             ("Typing", &OptionId::TYPING),
+                            ("Format · {format}", &OptionId::FORMAT),
                             ("History", &OptionId::HISTORY),
                         ],
                         false,
@@ -419,7 +420,7 @@ fn content(
     };
     // The first heading stays over the rows as they scroll; the others
     // scroll with their rows, 14 under the last row before them.
-    widgets::section_label(&skin.say(first), look, palette)
+    widgets::section_label(&said(ui.ctx(), look, skin.locale, first), look, palette)
         .layout(ui.ctx())
         .label(ui);
     ui.add_space(2.0);
@@ -441,9 +442,13 @@ fn content(
             for (nth, (heading, options)) in sections.iter().enumerate() {
                 if nth > 0 {
                     ui.add_space(14.0);
-                    widgets::section_label(&skin.say(heading), look, palette)
-                        .layout(ui.ctx())
-                        .label(ui);
+                    widgets::section_label(
+                        &said(ui.ctx(), look, skin.locale, heading),
+                        look,
+                        palette,
+                    )
+                    .layout(ui.ctx())
+                    .label(ui);
                     ui.add_space(2.0);
                 }
                 for option in options.iter().copied() {
```

```diff
diff --git a/src/ui/settings/terminal.rs b/src/ui/settings/terminal.rs
--- a/src/ui/settings/terminal.rs
+++ b/src/ui/settings/terminal.rs
@@ -427,7 +427,7 @@ fn heading(ui: &mut Ui, text: &'static str, top: f32, skin: &Skin) {
         ui,
         place.left() + ROW_SIDE,
         place.top() + top + 12.0 + line / 2.0,
-        Text::one(look, role, &skin.say(text), palette.dim),
+        Text::one(look, role, &skin.label(ui.ctx(), text), palette.dim),
     );
 }
 
@@ -528,6 +528,7 @@ fn editor_rows(
     let shown: Vec<OptionId> = [
         &OptionId::RUNNING[..],
         &OptionId::TERMINAL_TYPING[..],
+        &OptionId::FORMAT[..],
         &OptionId::HISTORY[..],
     ]
     .concat();
@@ -536,6 +537,12 @@ fn editor_rows(
     for (heading_text, top, options) in [
         ("Running queries", 8.0, &OptionId::RUNNING[..]),
         ("Typing", 12.0, &OptionId::TERMINAL_TYPING[..]),
+        // The board's: "format · = in normal mode".
+        (
+            "Format · {format} in normal mode",
+            12.0,
+            &OptionId::FORMAT[..],
+        ),
         ("History", 12.0, &OptionId::HISTORY[..]),
     ] {
         heading(ui, heading_text, top, skin);
```

- [ ] **Step 4: Run the tests to see them pass**

Run: `cargo test --locked --lib -- settings ui::format_settings_tests ui::editor_settings_tests ui::settings_access_tests`
Expected: PASS. Then the four checks above.

- [ ] **Step 5: Compare with the boards by eye**

Not a test, and nothing of it is committed. Append a throwaway `#[test] #[ignore]` scene to `src/shots.rs` that opens Settings › Editor in `Harness::for_shots(egui::vec2(1440.0, 1100.0), 1.0, true, Look::macos())` and in `Harness::for_shots(egui::vec2(1920.0, 1080.0), 1.0, true, Look::omarchy())` and shoots each. Run it with `cargo test --locked --features shots --lib shots::<name> -- --ignored`, read the PNGs in `target/shots/`, and set them beside the two "Settings, Editor" boards' Typing and Format rows. Remove the scene before committing. Keep the PNGs local.

- [ ] **Step 6: Commit**

```bash
git add src/model.rs src/settings.rs src/ui/editor_settings_tests.rs src/ui/format_settings_tests.rs src/ui/mod.rs src/ui/settings/mod.rs src/ui/settings/sheet.rs src/ui/settings/terminal.rs
git commit -m "Show Settings › Editor's Indent row and Format section in both looks"
```

---

### Task 3: Format, Tab and completion follow the settings

**Files:**
- Modify: `src/keymap.rs`, `src/app.rs`, `src/ui/sql_text.rs`, `src/completion.rs`
- Modify (tests): `src/ui/format_settings_tests.rs`, `src/ui/sql_text.rs`, `src/completion.rs`, `src/ui/complete_tests.rs`, `src/ui/mod.rs`

- [ ] **Step 1: Write the failing tests**

```diff
diff --git a/src/completion.rs b/src/completion.rs
--- a/src/completion.rs
+++ b/src/completion.rs
@@ -600,7 +600,7 @@ mod tests {
             columns: &columns,
             ..catalog(Dialect::Postgres, &tree, Some("public"), &schemas)
         };
-        list(&site, &typed, false, &catalog).candidates
+        list(&site, &typed, false, &catalog, KeywordCase::Lower).candidates
     }
 
     fn labels_of(candidates: &[Candidate]) -> Vec<&str> {
@@ -653,7 +653,7 @@ mod tests {
         let (tree, schemas) = shop();
         let (site, typed) = site_at(marked);
         let catalog = catalog(Dialect::Postgres, &tree, bare, &schemas);
-        list(&site, &typed, false, &catalog)
+        list(&site, &typed, false, &catalog, KeywordCase::Lower)
             .candidates
             .into_iter()
             .map(|c| (c.label, c.insert, c.kind))
@@ -743,7 +743,7 @@ mod tests {
         let offered = |marked: &str| {
             let (site, typed) = site_at(marked);
             let catalog = catalog(Dialect::Postgres, &tree, Some("public"), &schemas);
-            let listed = list(&site, &typed, false, &catalog);
+            let listed = list(&site, &typed, false, &catalog, KeywordCase::Lower);
             let rows = listed.candidates.into_iter();
             rows.map(|c| (c.label, c.insert)).collect::<Vec<_>>()
         };
@@ -1152,7 +1152,7 @@ mod tests {
         let (site, typed) = site_at(marked);
         let tree = Tree::default();
         let catalog = catalog(dialect, &tree, None, &[]);
-        let listed = list(&site, &typed, manual, &catalog);
+        let listed = list(&site, &typed, manual, &catalog, KeywordCase::Lower);
         listed.candidates.into_iter().map(|c| c.label).collect()
     }
 
@@ -1177,24 +1177,33 @@ mod tests {
     }
 
     #[test]
-    fn keywords_follow_the_case_that_is_typed() {
-        assert_eq!(keywords_at("sel|", false), ["select"]);
-        assert_eq!(keywords_at("SEL|", false), ["SELECT"]);
-        assert_eq!(keywords_at("Sel|", false), ["SELECT"]);
-        // Nothing typed: upper case, in order, phrases among them.
+    fn keywords_are_offered_in_the_settings_case_whatever_is_typed() {
+        let (site, typed) = site_at("Sel|");
+        let tree = Tree::default();
+        let sqlite = catalog(Dialect::Sqlite, &tree, None, &[]);
+        let labels = |case| -> Vec<String> {
+            let listed = list(&site, &typed, false, &sqlite, case);
+            listed.candidates.into_iter().map(|c| c.label).collect()
+        };
+        assert_eq!(labels(KeywordCase::Upper), ["SELECT"]);
+        assert_eq!(labels(KeywordCase::Lower), ["select"]);
+        // Keep has no case for a word not typed yet: upper, as Format's.
+        assert_eq!(labels(KeywordCase::Keep), ["SELECT"]);
+        assert_eq!(keywords_at("SEL|", false), ["select"]);
+        // Nothing typed: in order, phrases among them.
         let all = keywords_at("|", true);
-        assert_eq!(all[0], "ALL");
-        assert!(all.contains(&"GROUP BY".to_owned()));
+        assert_eq!(all[0], "all");
+        assert!(all.contains(&"group by".to_owned()));
         // Keywords and phrases must fit under KEPT, or the last ones fall
         // out of the list: in every dialect, each with its own words.
         let (site, typed) = site_at("|");
         let tree = Tree::default();
         for dialect in [Dialect::Postgres, Dialect::MySql, Dialect::Sqlite] {
             let catalog = catalog(dialect, &tree, None, &[]);
-            let listed = list(&site, &typed, true, &catalog);
+            let listed = list(&site, &typed, true, &catalog, KeywordCase::Lower);
             assert_eq!(listed.more, 0, "{dialect:?}");
             let last = listed.candidates.last().map(|c| c.label.as_str());
-            assert_eq!(last, Some("WITH"), "{dialect:?}");
+            assert_eq!(last, Some("with"), "{dialect:?}");
         }
     }
 
@@ -1208,7 +1217,7 @@ mod tests {
     fn a_keyword_typed_in_any_case_is_what_is_typed() {
         assert_eq!(
             keywords_at("SELECT * FROM users WHERE Is|", false)[..3],
-            ["IS", "IS NOT NULL", "IS NULL"]
+            ["is", "is not null", "is null"]
         );
         assert!(keyword_row("FROM").is_typed("From"));
         assert!(keyword_row("FROM").is_typed("from"));
@@ -1278,7 +1287,7 @@ mod tests {
         let rows = |marked: &str| {
             let (site, typed) = site_at(marked);
             let catalog = catalog(Dialect::Postgres, &tree, Some("public"), &schemas);
-            let listed = list(&site, &typed, false, &catalog);
+            let listed = list(&site, &typed, false, &catalog, KeywordCase::Lower);
             let only_repeats = listed.only_repeats(&typed);
             let rows = listed.candidates.into_iter();
             let rows = rows.map(|c| (c.is_typed(&typed), c.insert));
@@ -1342,7 +1351,7 @@ mod tests {
         let (site, typed) = site_at("sel|");
         let tree = Tree::default();
         let catalog = catalog(Dialect::Sqlite, &tree, None, &[]);
-        let listed = list(&site, &typed, false, &catalog);
+        let listed = list(&site, &typed, false, &catalog, KeywordCase::Lower);
         assert_eq!(
             listed.candidates,
             [Candidate {
```

```diff
diff --git a/src/ui/complete_tests.rs b/src/ui/complete_tests.rs
--- a/src/ui/complete_tests.rs
+++ b/src/ui/complete_tests.rs
@@ -14,10 +14,17 @@ use crate::model::{Action, Completion, ConnTabId, Fetch, SqlTab, TabId, TextPrin
 use crate::testing::{Harness, key, release};
 use crate::ui::sql_text::TOKENIZED;
 
+/// Keywords offered in lower case, as these tests were written for: the
+/// case typed, lower here, before Settings › Editor set one.
+fn lower_keywords(harness: &mut Harness) {
+    harness.app.settings.keywords = Some(crate::settings::KeywordCase::Lower);
+}
+
 /// A connected fixture workspace showing an empty SQL editor that has the
 /// keyboard.
 fn editor() -> (Harness, ConnTabId) {
     let mut harness = Harness::new();
+    lower_keywords(&mut harness);
     let tab = harness.connect_fake();
     harness.press(Key::T, Modifiers::COMMAND);
     (harness, tab)
@@ -166,7 +173,7 @@ fn a_list_asked_for_by_hand_opens_on_an_empty_word() {
     harness.settle();
     let open = list(&harness, tab).expect("a list");
     assert!(open.manual);
-    assert_eq!(open.candidates[0].label, "ALL");
+    assert_eq!(open.candidates[0].label, "all");
     // It goes on as its word is typed, still by hand.
     type_text(&mut harness, "s");
     assert_eq!(labels(&harness, tab), ["select", "set", "show", "skip"]);
@@ -323,7 +330,7 @@ fn a_list_opened_on_an_empty_word_stays_when_its_word_is_typed_and_deleted() {
     assert_eq!(sql(&harness, tab).text, "");
     // The whole list again, still by hand.
     let open = list(&harness, tab).expect("a list");
-    assert_eq!(open.candidates[0].label, "ALL");
+    assert_eq!(open.candidates[0].label, "all");
     assert!(open.manual);
     assert_eq!(open.typed, "");
 }
@@ -541,7 +548,7 @@ fn ctrl_space_opens_the_list_by_hand() {
     harness.press(Key::Space, ctrl());
     let open = list(&harness, tab).expect("a list");
     assert!(open.manual);
-    assert_eq!(open.candidates[0].label, "ALL");
+    assert_eq!(open.candidates[0].label, "all");
     assert_eq!(sql(&harness, tab).text, "");
 }
 
@@ -1524,7 +1531,7 @@ fn mod_i_opens_the_list_by_hand() {
         harness.press(Key::I, held);
         let open = list(&harness, tab).unwrap_or_else(|| panic!("no list for {held:?}"));
         assert!(open.manual, "{held:?}");
-        assert_eq!(open.candidates[0].label, "ALL", "{held:?}");
+        assert_eq!(open.candidates[0].label, "all", "{held:?}");
         // The editor never saw the key.
         assert_eq!(sql(&harness, tab).text, "", "{held:?}");
         assert!(editor_has_keyboard(&harness, tab), "{held:?}");
@@ -1671,7 +1678,13 @@ fn a_list_with_no_place_closes_once_the_caret_is_in_the_pane() {
         schemas: &[],
         columns: &columns,
     };
-    let listed = crate::completion::list(&site, "se", false, &catalog);
+    let listed = crate::completion::list(
+        &site,
+        "se",
+        false,
+        &catalog,
+        crate::settings::KeywordCase::Lower,
+    );
     assert!(!listed.candidates.is_empty());
     let of = (TextPrint::of(&sql_tab.text), sql_tab.cursor, generation);
     let open = Completion::new(false, of, site, "se".into(), listed, false);
@@ -1685,7 +1698,7 @@ fn a_list_with_no_place_closes_once_the_caret_is_in_the_pane() {
     assert!(list(&harness, tab).is_none());
     assert!(editor_has_keyboard(&harness, tab));
     harness.press(Key::Tab, Modifiers::NONE);
-    assert_eq!(sql(&harness, tab).text, format!("{before}\t"));
+    assert_eq!(sql(&harness, tab).text, format!("{before}  "));
 }
 
 #[test]
@@ -1806,6 +1819,7 @@ fn a_list_that_shrinks_is_drawn_at_once_where_it_stays() {
     // typed word left it a single row, and once that has settled.
     let run = |height: f32, lines: usize| {
         let mut harness = Harness::with_size(egui::vec2(1280.0, height));
+        lower_keywords(&mut harness);
         harness.connect_fake();
         harness.press(Key::T, Modifiers::COMMAND);
         paste(&mut harness, &"\n".repeat(lines));
@@ -1846,6 +1860,8 @@ fn format_leaves_no_list_of_the_script_as_it_was() {
         paste(&mut harness, script);
         type_text(&mut harness, "fr");
         assert_eq!(labels(&harness, tab), ["from"]);
+        // Upper case from here, so that Format has keywords to change.
+        harness.app.settings.keywords = Some(crate::settings::KeywordCase::Upper);
         (harness, tab)
     };
     // An open list is of the script on screen, once a frame is over.
@@ -1865,10 +1881,10 @@ fn format_leaves_no_list_of_the_script_as_it_was() {
     no_stale_list(&harness, tab);
     harness.settle();
     assert!(list(&harness, tab).is_some_and(|list| list.is_of_text("SELECT 1 fr")));
-    assert_eq!(labels(&harness, tab), ["from"]);
+    assert_eq!(labels(&harness, tab), ["FROM"]);
     assert!(editor_has_keyboard(&harness, tab));
     harness.press(Key::Tab, Modifiers::NONE);
-    assert_eq!(sql(&harness, tab).text, "SELECT 1 from");
+    assert_eq!(sql(&harness, tab).text, "SELECT 1 FROM");
 
     // Format moves the word: the list closes.
     let (mut harness, tab) = completing("select a,b ");
@@ -2431,6 +2447,7 @@ fn a_schema_and_a_table_and_a_dot_offer_its_columns() {
 #[test]
 fn a_table_tabs_structure_fills_the_columns() {
     let mut harness = Harness::new();
+    lower_keywords(&mut harness);
     let tab = harness.connect_fake();
     harness.click("users");
     harness.answer_rows(crate::testing::page(5, false));
@@ -2459,6 +2476,7 @@ fn a_table_tabs_structure_fills_the_columns() {
 #[test]
 fn a_table_tabs_structure_that_failed_leaves_its_columns_to_be_asked_for() {
     let mut harness = Harness::new();
+    lower_keywords(&mut harness);
     let tab = harness.connect_fake();
     harness.click("users");
     harness.answer_rows(crate::testing::page(5, false));
```

```diff
diff --git a/src/ui/format_settings_tests.rs b/src/ui/format_settings_tests.rs
--- a/src/ui/format_settings_tests.rs
+++ b/src/ui/format_settings_tests.rs
@@ -2,14 +2,315 @@
 //! Format writes, what Tab and Shift+Tab put in and take out, the case
 //! completion offers keywords in, and the rows that set them.
 
+use egui::{Key, Modifiers};
+
 use crate::keymap::{Command as Keyed, Scope};
-use crate::model::{Action, SettingsPane};
-use crate::settings::{Commas, Indent, KeywordCase};
+use crate::model::{Action, ConnTabId, SettingsPane};
+use crate::settings::{Commas, Indent, KeywordCase, OptionValue};
 use crate::testing::Harness;
 use crate::theme::Look;
 
 use super::appearance_settings_tests::{settings_saves, shows};
 
+const COMMAND_SHIFT: Modifiers = Modifiers::COMMAND.plus(Modifiers::SHIFT);
+
+/// A connected fixture in `look`, with a new SQL editor that has the
+/// keyboard and holds `text`, its cursor at the end.
+fn editor(look: Look, text: &str) -> (Harness, ConnTabId) {
+    let mut harness = Harness::new();
+    harness.set_look(look);
+    let tab = harness.connect_fake();
+    harness.press(Key::T, Modifiers::COMMAND);
+    if !text.is_empty() {
+        harness.frame(vec![egui::Event::Paste(text.into())]);
+    }
+    harness.settle();
+    (harness, tab)
+}
+
+fn text(harness: &Harness, tab: ConnTabId) -> String {
+    let workspace = harness.app.workspace(tab).unwrap();
+    workspace.active_sql_tab().unwrap().text.clone()
+}
+
+/// Formats with the chord of the macOS and Windows looks.
+fn format(harness: &mut Harness) {
+    harness.press(Key::F, COMMAND_SHIFT);
+    harness.settle();
+}
+
+fn set(harness: &mut Harness, value: OptionValue) {
+    harness.app.apply(Action::SetOption(value));
+    harness.settle();
+}
+
+#[test]
+fn format_writes_the_case_the_commas_and_the_indent_set() {
+    let script = "select p.name as publisher, case when c.kind = 'ebook' then 1 else 0 end \
+        as ebook from book_covers c join publishers p on p.id = c.publisher_id";
+    // The defaults of macOS and Windows: upper case, trailing commas, a
+    // CASE's parts two in.
+    let (mut harness, tab) = editor(Look::macos(), script);
+    format(&mut harness);
+    assert_eq!(
+        text(&harness, tab),
+        [
+            "SELECT p.name AS publisher,",
+            "       CASE",
+            "         WHEN c.kind = 'ebook' THEN 1",
+            "         ELSE 0",
+            "       END AS ebook",
+            "  FROM book_covers c",
+            "  JOIN publishers p ON p.id = c.publisher_id",
+        ]
+        .join("\n")
+    );
+    // Lower case, leading commas, four in.
+    let (mut harness, tab) = editor(Look::macos(), script);
+    set(
+        &mut harness,
+        OptionValue::Keywords(Some(KeywordCase::Lower)),
+    );
+    set(&mut harness, OptionValue::Commas(Commas::Leading));
+    set(&mut harness, OptionValue::Indent(Indent::Four));
+    format(&mut harness);
+    assert_eq!(
+        text(&harness, tab),
+        [
+            "select p.name as publisher",
+            "     , case",
+            "           when c.kind = 'ebook' then 1",
+            "           else 0",
+            "       end as ebook",
+            "  from book_covers c",
+            "  join publishers p on p.id = c.publisher_id",
+        ]
+        .join("\n")
+    );
+    // One undo step gives the script back as typed.
+    harness.press(Key::Z, Modifiers::COMMAND);
+    harness.settle();
+    assert_eq!(text(&harness, tab), script);
+}
+
+#[test]
+fn keep_leaves_the_case_as_typed_and_names_strings_and_comments_alone() {
+    let script = "Select Name, 'Select me' -- Select it\nFrom Books where Kind = 'From'";
+    let (mut harness, tab) = editor(Look::macos(), script);
+    set(&mut harness, OptionValue::Keywords(Some(KeywordCase::Keep)));
+    format(&mut harness);
+    assert_eq!(
+        text(&harness, tab),
+        [
+            "Select Name,",
+            "       'Select me' -- Select it",
+            "  From Books",
+            " where Kind = 'From'",
+        ]
+        .join("\n")
+    );
+}
+
+#[test]
+fn the_terminal_look_formats_in_lower_case_until_set() {
+    let (mut harness, tab) = editor(Look::omarchy(), "SELECT 1 FROM t");
+    harness.press(Key::Escape, Modifiers::NONE);
+    super::tests::type_key(&mut harness, Key::Equals, "=");
+    harness.settle();
+    assert_eq!(text(&harness, tab), "select 1\n  from t");
+}
+
+#[test]
+fn tab_puts_in_the_indent_set_and_shift_tab_takes_one_out() {
+    for (indent, unit) in [
+        (Indent::Two, "  "),
+        (Indent::Four, "    "),
+        (Indent::Tab, "\t"),
+    ] {
+        let (mut harness, tab) = editor(Look::macos(), "");
+        set(&mut harness, OptionValue::Indent(indent));
+        harness.press(Key::Tab, Modifiers::NONE);
+        harness.settle();
+        harness.frame(vec![egui::Event::Text("SELECT 1".into())]);
+        harness.settle();
+        assert_eq!(text(&harness, tab), format!("{unit}SELECT 1"), "{indent:?}");
+        harness.press(Key::Tab, Modifiers::SHIFT);
+        harness.settle();
+        assert_eq!(text(&harness, tab), "SELECT 1", "{indent:?}");
+        // Shift+Tab is a step of its own. (A Tab at the cursor is typed
+        // text, and undoes with what is typed beside it.)
+        harness.press(Key::Z, Modifiers::COMMAND);
+        harness.settle();
+        assert_eq!(text(&harness, tab), format!("{unit}SELECT 1"), "{indent:?}");
+    }
+}
+
+#[test]
+fn tab_over_a_selection_indents_its_lines_and_keeps_them_selected() {
+    let (mut harness, tab) = editor(Look::macos(), "SELECT 1\nFROM t\n\nWHERE x");
+    // From the end up over the last three lines.
+    for _ in 0.."FROM t\n\nWHERE x".chars().count() {
+        harness.press(Key::ArrowLeft, Modifiers::SHIFT);
+    }
+    harness.press(Key::Tab, Modifiers::NONE);
+    harness.settle();
+    // The blank line stays blank.
+    assert_eq!(text(&harness, tab), "SELECT 1\n  FROM t\n\n  WHERE x");
+    // One undo takes the lines back, and redo puts them in again.
+    harness.press(Key::Z, Modifiers::COMMAND);
+    harness.settle();
+    assert_eq!(text(&harness, tab), "SELECT 1\nFROM t\n\nWHERE x");
+    harness.press(Key::Z, COMMAND_SHIFT);
+    harness.settle();
+    assert_eq!(text(&harness, tab), "SELECT 1\n  FROM t\n\n  WHERE x");
+    // Still selected: a second Tab indents them again, Shift+Tab out.
+    harness.press(Key::Tab, Modifiers::NONE);
+    harness.settle();
+    assert_eq!(text(&harness, tab), "SELECT 1\n    FROM t\n\n    WHERE x");
+    harness.press(Key::Tab, Modifiers::SHIFT);
+    harness.press(Key::Tab, Modifiers::SHIFT);
+    harness.settle();
+    assert_eq!(text(&harness, tab), "SELECT 1\nFROM t\n\nWHERE x");
+}
+
+/// Puts the editor's selection from `anchor` to `head` (characters), as a
+/// drag from one to the other does.
+fn select(harness: &mut Harness, tab: ConnTabId, anchor: usize, head: usize) {
+    use egui::text::{CCursor, CCursorRange};
+    let workspace = harness.app.workspace(tab).unwrap();
+    let id = workspace.active_sql_tab().unwrap().id;
+    let editor = crate::ui::sql_text::editor_id(tab, id);
+    let mut state = egui::TextEdit::load_state(&harness.ctx, editor).expect("an editor");
+    // `two` puts the moving end at its second.
+    let range = CCursorRange::two(CCursor::new(anchor), CCursor::new(head));
+    state.cursor.set_char_range(Some(range));
+    egui::TextEdit::store_state(&harness.ctx, editor, state);
+    harness.settle();
+}
+
+/// The editor's selection: where it was started, and its moving end.
+fn selection(harness: &Harness, tab: ConnTabId) -> Option<(usize, usize)> {
+    let workspace = harness.app.workspace(tab).unwrap();
+    let id = workspace.active_sql_tab().unwrap().id;
+    let editor = crate::ui::sql_text::editor_id(tab, id);
+    let range = egui::TextEdit::load_state(&harness.ctx, editor)?
+        .cursor
+        .char_range()?;
+    Some((range.secondary.index.0, range.primary.index.0))
+}
+
+#[test]
+fn a_selection_made_backwards_keeps_its_moving_end() {
+    let (mut harness, tab) = editor(Look::macos(), "SELECT 1\nFROM t");
+    select(&mut harness, tab, 15, 9);
+    harness.press(Key::Tab, Modifiers::NONE);
+    harness.settle();
+    assert_eq!(text(&harness, tab), "SELECT 1\n  FROM t");
+    assert_eq!(selection(&harness, tab), Some((17, 9)));
+    harness.press(Key::Tab, Modifiers::SHIFT);
+    harness.settle();
+    assert_eq!(selection(&harness, tab), Some((15, 9)));
+    // Forwards stays forwards.
+    select(&mut harness, tab, 9, 15);
+    harness.press(Key::Tab, Modifiers::NONE);
+    harness.settle();
+    assert_eq!(selection(&harness, tab), Some((9, 17)));
+}
+
+#[test]
+fn a_tab_and_text_typed_in_one_frame_keep_their_order() {
+    for (events, want) in [(vec![true, false], "  a"), (vec![false, true], "a  ")] {
+        let (mut harness, tab) = editor(Look::macos(), "");
+        let frame = events
+            .into_iter()
+            .flat_map(|tab_key| {
+                if tab_key {
+                    vec![
+                        crate::testing::key(Key::Tab, Modifiers::NONE),
+                        crate::testing::release(Key::Tab, Modifiers::NONE),
+                    ]
+                } else {
+                    vec![egui::Event::Text("a".into())]
+                }
+            })
+            .collect();
+        harness.frame(frame);
+        harness.settle();
+        assert_eq!(text(&harness, tab), want);
+    }
+}
+
+#[test]
+fn tab_is_not_the_editor_s_without_the_keyboard() {
+    for look in [Look::macos(), Look::omarchy()] {
+        let (mut harness, tab) = editor(look, "SELECT 1");
+        // Esc gives the keyboard up: Omarchy's normal mode, and the other
+        // looks' editor left.
+        harness.press(Key::Escape, Modifiers::NONE);
+        harness.press(Key::Tab, Modifiers::NONE);
+        harness.press(Key::Tab, Modifiers::SHIFT);
+        harness.settle();
+        assert_eq!(text(&harness, tab), "SELECT 1", "{}", look.name);
+    }
+}
+
+#[test]
+fn shift_tab_under_an_open_list_leaves_no_list_of_the_old_text() {
+    let (mut harness, tab) = editor(Look::macos(), "  ");
+    harness.frame(vec![egui::Event::Text("sel".into())]);
+    harness.settle();
+    let list = |harness: &Harness| {
+        let workspace = harness.app.workspace(tab).unwrap();
+        let sql = workspace.active_sql_tab().unwrap();
+        sql.completion
+            .as_ref()
+            .map(|list| list.is_of_text(&sql.text))
+    };
+    assert_eq!(list(&harness), Some(true));
+    harness.press(Key::Tab, Modifiers::SHIFT);
+    harness.settle();
+    assert_eq!(text(&harness, tab), "sel");
+    assert_ne!(list(&harness), Some(false), "a list of the text as it was");
+}
+
+#[test]
+fn tab_still_accepts_an_open_completion() {
+    let (mut harness, tab) = editor(Look::macos(), "");
+    harness.frame(vec![egui::Event::Text("sel".into())]);
+    harness.settle();
+    harness.press(Key::Tab, Modifiers::NONE);
+    harness.settle();
+    assert_eq!(text(&harness, tab), "SELECT");
+}
+
+#[test]
+fn completion_offers_keywords_in_the_case_set() {
+    // Typed in the other case than the one offered: the case typed is not
+    // what decides.
+    for (case, typed, want) in [
+        (None, "sel", "SELECT"),
+        (Some(KeywordCase::Lower), "SEL", "select"),
+        (Some(KeywordCase::Upper), "sel", "SELECT"),
+        // Keep has no case of its own for a word not typed yet.
+        (Some(KeywordCase::Keep), "sel", "SELECT"),
+    ] {
+        let (mut harness, tab) = editor(Look::macos(), "");
+        set(&mut harness, OptionValue::Keywords(case));
+        harness.frame(vec![egui::Event::Text(typed.into())]);
+        harness.settle();
+        harness.press(Key::Tab, Modifiers::NONE);
+        harness.settle();
+        assert_eq!(text(&harness, tab), want, "{case:?}");
+    }
+    // The terminal look's own is lower case.
+    let (mut harness, tab) = editor(Look::omarchy(), "");
+    harness.frame(vec![egui::Event::Text("SEL".into())]);
+    harness.settle();
+    harness.press(Key::Tab, Modifiers::NONE);
+    harness.settle();
+    assert_eq!(text(&harness, tab), "select");
+}
+
 /// Settings on its Editor pane.
 fn on_the_pane(harness: &mut Harness) {
     harness.app.apply(Action::ShowSettings);
```

```diff
diff --git a/src/ui/mod.rs b/src/ui/mod.rs
--- a/src/ui/mod.rs
+++ b/src/ui/mod.rs
@@ -4820,7 +4820,8 @@ value_tags   = true
             harness.ctx.text_edit_focused(),
             "the keys stay in the editor"
         );
-        assert_eq!(active_sql(&harness, tab).text, "\tSELECT 1");
+        // An indent is two spaces until Settings › Editor says otherwise.
+        assert_eq!(active_sql(&harness, tab).text, "  SELECT 1");
     }
 
     #[test]
@@ -5383,6 +5384,9 @@ value_tags   = true
         // The terminal has the key alone: `=`, once the keyboard is out of
         // the text. Formatting gives it back.
         let (mut harness, tab) = sql_harness(crate::theme::Look::omarchy());
+        // Upper case, so that Format has the keyword to change: the look's
+        // own is lower.
+        harness.app.settings.keywords = Some(crate::settings::KeywordCase::Upper);
         harness.settle();
         type_text(&mut harness, "select 1");
         assert!(!harness.has("Format"));
@@ -5395,6 +5399,8 @@ value_tags   = true
     #[test]
     fn omarchy_formats_with_the_equals_sign_once_the_keyboard_is_out_of_the_text() {
         let (mut harness, tab) = sql_harness(Look::omarchy());
+        // Upper case, so that Format has the keyword to change.
+        harness.app.settings.keywords = Some(crate::settings::KeywordCase::Upper);
         harness.settle();
         type_text(&mut harness, "select 1");
         type_text(&mut harness, "=");
```

```diff
diff --git a/src/ui/sql_text.rs b/src/ui/sql_text.rs
--- a/src/ui/sql_text.rs
+++ b/src/ui/sql_text.rs
@@ -938,6 +938,59 @@ mod tests {
         assert_eq!(byte_offset("", 3), 0);
     }
 
+    #[test]
+    fn an_indent_goes_in_at_the_cursor_or_on_each_line_selected() {
+        let tab = |text: &str, selection: Range<usize>, indent, outdent| {
+            indented(text, selection, indent, outdent)
+        };
+        // No selection: at the cursor, in the middle of a line too.
+        assert_eq!(
+            tab("SELECT 1", 6..6, Indent::Two, false),
+            Some(("SELECT   1".to_owned(), 8..8))
+        );
+        assert_eq!(
+            tab("x", 0..0, Indent::Tab, false),
+            Some(("\tx".to_owned(), 1..1))
+        );
+        // A selection: each line it touches, not one it only reaches the
+        // start of, and no blank one; it covers the same text after.
+        assert_eq!(
+            tab("a\nb\n\nc\nd", 2..7, Indent::Four, false),
+            Some(("a\n    b\n\n    c\nd".to_owned(), 2..15))
+        );
+        // Characters of more than one byte keep their place.
+        assert_eq!(
+            tab("é\nü", 0..5, Indent::Two, false),
+            Some(("  é\n  ü".to_owned(), 0..9))
+        );
+    }
+
+    #[test]
+    fn an_outdent_takes_one_indent_off_each_line_it_touches() {
+        let tab =
+            |text: &str, selection: Range<usize>, indent| indented(text, selection, indent, true);
+        // The cursor's line: up to an indent's width of spaces.
+        assert_eq!(
+            tab("    x", 5..5, Indent::Two),
+            Some(("  x".to_owned(), 3..3))
+        );
+        assert_eq!(tab(" x", 2..2, Indent::Four), Some(("x".to_owned(), 1..1)));
+        // A tab, whatever the indent is.
+        assert_eq!(tab("\tx", 1..1, Indent::Two), Some(("x".to_owned(), 0..0)));
+        // Nothing to take: nothing changes.
+        assert_eq!(tab("x\n  y", 0..1, Indent::Two), None);
+        // A cursor inside the indent stays at the line's new start.
+        assert_eq!(
+            tab("    x", 1..1, Indent::Four),
+            Some(("x".to_owned(), 0..0))
+        );
+        // Every line selected.
+        assert_eq!(
+            tab("  a\n    b\nc", 0..10, Indent::Two),
+            Some(("a\n  b\nc".to_owned(), 0..6))
+        );
+    }
+
     #[test]
     fn the_terminal_gutter_is_relative_to_the_cursors_line() {
         let numbers = |cursor, relative| -> Vec<usize> {
@@ -979,6 +1032,9 @@ mod tests {
                     look: &look,
                     palette: &palette,
                     hold_escape,
+                    style: sql::format::Style::default(),
+                    // A tab character: what Tab put in before the setting.
+                    indent: Indent::Tab,
                 };
                 focused = edit(ui, sql, &field).focused;
             });
```

- [ ] **Step 2: Run the tests to see them fail**

Run: `cargo test --locked --lib -- ui::format_settings_tests ui::sql_text completion ui::complete_tests ui::tests`
Expected: FAIL to compile (34 errors), among them: `sql_text` has no `indented`, its `Field` no `style` or `indent`, and `completion::list` is called with a keyword case it does not take.

- [ ] **Step 3: Format, indent and complete as set**

```diff
diff --git a/src/app.rs b/src/app.rs
--- a/src/app.rs
+++ b/src/app.rs
@@ -6177,7 +6177,8 @@ impl App {
             return changed;
         };
         let catalog = catalog_of(workspace, &schemas);
-        let listed = crate::completion::list(&site, &typed, manual, &catalog);
+        let case = self.settings.keywords_in(self.look.terminal);
+        let listed = crate::completion::list(&site, &typed, manual, &catalog, case);
         let loading = needs.iter().any(|need| workspace.is_loading(need));
         // The generation after asking: asking changed it.
         let of = (print, cursor, workspace.catalog_generation);
```

```diff
diff --git a/src/completion.rs b/src/completion.rs
--- a/src/completion.rs
+++ b/src/completion.rs
@@ -8,6 +8,7 @@ use tabletist_db::complete::{Expects, PHRASES, Site, Source};
 use tabletist_db::{ColumnInfo, Dialect, ObjectInfo, ObjectKind, ObjectRef};
 
 use crate::model::{Fetch, Tree};
+use crate::settings::KeywordCase;
 
 /// How many candidates a list keeps; the rest are only counted. Every
 /// keyword and phrase of a dialect fits, so a list opened by hand on an
@@ -317,14 +318,22 @@ pub fn needs(site: &Site, catalog: &Catalog<'_>) -> Vec<Need> {
 
 /// What `site` offers for the `typed` part of its word, from `catalog`.
 /// `manual` says the list was asked for by hand.
-pub fn list(site: &Site, typed: &str, manual: bool, catalog: &Catalog<'_>) -> Listed {
+/// Keywords are offered in `case`, Settings › Editor's keyword case:
+/// upper case for Keep.
+pub fn list(
+    site: &Site,
+    typed: &str,
+    manual: bool,
+    catalog: &Catalog<'_>,
+    case: KeywordCase,
+) -> Listed {
     #[cfg(test)]
     LISTED.with(|count| count.set(count.get() + 1));
     let dialect = catalog.dialect;
     let mut found: Vec<(Group, Candidate)> = Vec::new();
     match target(site, manual, catalog) {
         Target::Nothing => {}
-        Target::Keywords => keywords(&mut found, typed, dialect),
+        Target::Keywords => keywords(&mut found, typed, dialect, case),
         Target::Columns {
             objects,
             tables: with_tables,
@@ -356,7 +365,7 @@ pub fn list(site: &Site, typed: &str, manual: bool, catalog: &Catalog<'_>) -> Li
                 }
             }
             if with_keywords {
-                keywords(&mut found, typed, dialect);
+                keywords(&mut found, typed, dialect, case);
             }
         }
         Target::Tables => tables(&mut found, site, typed, catalog),
@@ -425,10 +434,10 @@ fn object(
     })
 }
 
-/// Adds the keywords and phrases that start with `typed`.
-fn keywords(found: &mut Vec<(Group, Candidate)>, typed: &str, dialect: Dialect) {
+/// Adds the keywords and phrases that start with `typed`, in `case`.
+fn keywords(found: &mut Vec<(Group, Candidate)>, typed: &str, dialect: Dialect, case: KeywordCase) {
     let words = tabletist_db::sql::keywords(dialect).chain(PHRASES);
-    let words = words.filter_map(|word| keyword(word, typed));
+    let words = words.filter_map(|word| keyword(word, typed, case));
     found.extend(words.map(|candidate| (KEYWORDS, candidate)));
 }
 
@@ -454,15 +463,14 @@ fn name(name: &str, kind: Kind, typed: &str, dialect: Dialect) -> Option<Candida
     })
 }
 
-/// `word` as a candidate when it starts with `typed`, in the case typed:
-/// lower case when every typed letter is lower case, upper case otherwise.
-fn keyword(word: &str, typed: &str) -> Option<Candidate> {
+/// `word` as a candidate when it starts with `typed`, whatever its case,
+/// written in `case`: upper case for Keep, which keeps what Format writes
+/// and has no case of its own to give a word not typed yet.
+fn keyword(word: &str, typed: &str, case: KeywordCase) -> Option<Candidate> {
     let matched = find_ignoring_case(word, typed).filter(|found| found.start == 0)?;
-    let lower = typed.chars().any(char::is_lowercase) && !typed.chars().any(char::is_uppercase);
-    let label = if lower {
-        word.to_lowercase()
-    } else {
-        word.to_owned()
+    let label = match case {
+        KeywordCase::Lower => word.to_lowercase(),
+        KeywordCase::Upper | KeywordCase::Keep => word.to_uppercase(),
     };
     Some(Candidate {
         kind: Kind::Keyword,
```

```diff
diff --git a/src/keymap.rs b/src/keymap.rs
--- a/src/keymap.rs
+++ b/src/keymap.rs
@@ -478,6 +478,8 @@ commands! {
     StepDown,
     Format,
     // SQL editor.
+    IndentLines,
+    OutdentLines,
     RunStatement,
     RunAll,
     Explain,
@@ -823,6 +825,8 @@ impl Command {
             C::StepUp => info("Step up", STEP, Editing).reserved(),
             C::StepDown => info("Step down", STEP, Editing).reserved(),
             C::Format => info("Format SQL or JSON", "Format SQL or JSON", Sql),
+            C::IndentLines => info("Indent", "", Sql),
+            C::OutdentLines => info("Outdent", "", Sql),
             C::RunStatement => info("Run statement", "Run statement", Sql),
             C::RunAll => info("Run all", "Run all", Sql),
             C::Explain => info("Explain", "Explain", Sql).reserved(),
@@ -1682,6 +1686,21 @@ pub const BINDINGS: &[Binding] = {
             keys(&[k("cmd+shift+f")]).any(),
             keys(&[k("=")]),
         ),
+        // In the editor's text: an indent at the cursor, or one on each
+        // line the selection touches; one taken off each line. Settings ›
+        // Editor says what an indent is.
+        bind(
+            C::IndentLines,
+            SQL_EDITOR,
+            keys(&[k("tab")]),
+            keys(&[k("tab")]),
+        ),
+        bind(
+            C::OutdentLines,
+            SQL_EDITOR,
+            keys(&[k("shift+tab")]),
+            keys(&[k("shift+tab")]),
+        ),
         bind(
             C::Format,
             CELL_EDITOR,
```

```diff
diff --git a/src/ui/sql_text.rs b/src/ui/sql_text.rs
--- a/src/ui/sql_text.rs
+++ b/src/ui/sql_text.rs
@@ -16,6 +16,7 @@ use tabletist_db::sql::{self, Statement, Token, TokenKind};
 use crate::app::App;
 use crate::i18n::{Locale, gettext};
 use crate::model::{Action, Completion, ConnTabId, Pane, SqlTab, TabId, TextPrint};
+use crate::settings::Indent;
 use crate::theme::{Look, Palette};
 use crate::typography::{Text, TextRole};
 use crate::ui::widgets;
@@ -339,8 +340,9 @@ fn format_script(ui: &Ui, sql_tab: &mut SqlTab, field: &Field<'_>) -> bool {
         }
         None => (None, sql_tab.cursor),
     };
-    let Some(formatted) = sql::format::format(field.dialect, &sql_tab.text, selection, cursor)
-    else {
+    let formatted =
+        sql::format::format_with(field.dialect, &sql_tab.text, selection, cursor, field.style);
+    let Some(formatted) = formatted else {
         return false;
     };
     let at = |text: &str, byte: usize| CCursorRange::one(CCursor::new(char_index(text, byte)));
@@ -444,6 +446,7 @@ impl Edited {
 pub fn show(app: &mut App, ui: &mut Ui, tab: ConnTabId, id: TabId) {
     let (look, palette, locale) = (app.look, app.palette, app.locale);
     let numbering = app.settings.line_numbers_in(look.terminal);
+    let (style, indent) = (format_style(&app.settings, &look), app.settings.indent);
     let highlight = app.settings.highlight_line;
     let Some(workspace) = app.workspace_mut(tab) else {
         return;
@@ -469,6 +472,8 @@ pub fn show(app: &mut App, ui: &mut Ui, tab: ConnTabId, id: TabId) {
         dialect,
         look: &look,
         palette: &palette,
+        style,
+        indent,
         // An open completion list takes Esc: the editor keeps the
         // keyboard. Not one whose row this frame inserts: Esc is the
         // editor's again.
@@ -689,6 +694,221 @@ struct Field<'a> {
     palette: &'a Palette,
     /// Keep the keyboard on Esc (a completion list is open and takes it).
     hold_escape: bool,
+    /// How Format writes the script.
+    style: sql::format::Style,
+    /// What the Tab key puts in, and takes out with Shift.
+    indent: Indent,
+}
+
+/// How Format writes, as the settings have it in `look`: its keyword case
+/// and commas, and a `CASE`'s parts in by the indent (four spaces for a
+/// tab: the river is set in spaces).
+fn format_style(settings: &crate::settings::Settings, look: &Look) -> sql::format::Style {
+    use crate::settings::{Commas, KeywordCase};
+    use sql::format::Case;
+    sql::format::Style {
+        keywords: match settings.keywords_in(look.terminal) {
+            KeywordCase::Upper => Case::Upper,
+            KeywordCase::Lower => Case::Lower,
+            KeywordCase::Keep => Case::Keep,
+        },
+        commas: match settings.commas {
+            Commas::Trailing => sql::format::Commas::Trailing,
+            Commas::Leading => sql::format::Commas::Leading,
+        },
+        case_indent: indent_width(settings.indent),
+    }
+}
+
+/// What an indent puts in.
+fn indent_unit(indent: Indent) -> &'static str {
+    match indent {
+        Indent::Two => "  ",
+        Indent::Four => "    ",
+        Indent::Tab => "\t",
+    }
+}
+
+/// How many columns an indent is: a tab counts four.
+fn indent_width(indent: Indent) -> usize {
+    match indent {
+        Indent::Two => 2,
+        Indent::Four | Indent::Tab => 4,
+    }
+}
+
+/// `text` with an indent put in or taken out (`outdent`), and the bytes
+/// `selection` covers after it. Without a selection an indent goes in at
+/// the cursor (`selection`'s end); with one, at the start of every line it
+/// touches but blank ones; an outdent takes one indent (a tab, or up to
+/// an indent's width of spaces) off the start of every line the selection
+/// or the cursor touches. A line the selection only reaches the start of
+/// is not touched. `None` when nothing changes.
+fn indented(
+    text: &str,
+    selection: Range<usize>,
+    indent: Indent,
+    outdent: bool,
+) -> Option<(String, Range<usize>)> {
+    let unit = indent_unit(indent);
+    if !outdent && selection.is_empty() {
+        let at = selection.end;
+        let mut out = text.to_owned();
+        out.insert_str(at, unit);
+        let to = at + unit.len();
+        return Some((out, to..to));
+    }
+    // The starts of the lines touched, in order.
+    let first = text[..selection.start].rfind('\n').map_or(0, |at| at + 1);
+    let mut starts = vec![first];
+    for (at, byte) in text.bytes().enumerate().skip(first) {
+        let next = at + 1;
+        let touched = next < selection.end;
+        if byte == b'\n' && touched && next < text.len() {
+            starts.push(next);
+        } else if next >= selection.end {
+            break;
+        }
+    }
+    // Each line's change: where, what goes out, what comes in.
+    let mut edits: Vec<(usize, usize, &str)> = Vec::new();
+    for start in starts {
+        let line = &text[start
+            ..text[start..]
+                .find('\n')
+                .map_or(text.len(), |end| start + end)];
+        if outdent {
+            let cut = if line.starts_with('\t') {
+                1
+            } else {
+                line.bytes()
+                    .take(indent_width(indent))
+                    .take_while(|byte| *byte == b' ')
+                    .count()
+            };
+            if cut > 0 {
+                edits.push((start, cut, ""));
+            }
+        } else if !line.trim().is_empty() {
+            edits.push((start, 0, unit));
+        }
+    }
+    if edits.is_empty() {
+        return None;
+    }
+    // Where an offset of the old text lands in the new one.
+    let moved = |offset: usize| -> usize {
+        let mut to = offset;
+        for (at, cut, put) in &edits {
+            if *at >= offset {
+                break;
+            }
+            let gone = (*cut).min(offset - at);
+            to = to - gone + put.len();
+        }
+        to
+    };
+    let mut out = String::with_capacity(text.len() + edits.len() * unit.len());
+    let mut from = 0;
+    for (at, cut, put) in &edits {
+        out.push_str(&text[from..*at]);
+        out.push_str(put);
+        from = at + cut;
+    }
+    out.push_str(&text[from..]);
+    Some((out, moved(selection.start)..moved(selection.end)))
+}
+
+/// Takes the frame's Tab and Shift+Tab, as the keymap has them in the
+/// editor (`IndentLines`, `OutdentLines`), before the field reads its
+/// events: Tab with nothing selected becomes the indent's text, typed in
+/// its place among the frame's events; Tab over a selection and Shift+Tab
+/// are made here on the lines (`indented`), each one step of the field's
+/// undo history, the selection kept as it ran. Only while the field has
+/// the keyboard: the completion list took its Tab before. Returns whether
+/// the text changed here.
+fn indent_script(ui: &mut Ui, sql_tab: &mut SqlTab, field: &Field<'_>) -> bool {
+    use crate::keymap::{Command, Layout, Scope};
+    if !ui.memory(|memory| memory.has_focus(field.id)) {
+        return false;
+    }
+    let keymap = crate::ui::keys::published(ui.ctx());
+    let layout = Layout::of(field.look);
+    let chords = |command| -> Vec<&'static str> {
+        keymap
+            .chords_now(layout, command, Scope::SqlEditor, false)
+            .collect()
+    };
+    let (indents, outdents) = (chords(Command::IndentLines), chords(Command::OutdentLines));
+    let unit = indent_unit(field.indent);
+    let selected = egui::TextEdit::load_state(ui.ctx(), field.id)
+        .and_then(|state| state.cursor.char_range())
+        .is_some_and(|range| !range.is_empty());
+    // The line edits asked for, in order: `true` to outdent.
+    let mut lines = Vec::new();
+    ui.input_mut(|input| {
+        let held = input.modifiers;
+        let pressed = |event: &egui::Event, chords: &[&'static str]| {
+            chords
+                .iter()
+                .any(|keys| crate::ui::keys::press_of(layout, held, event, keys).is_some())
+        };
+        let events = std::mem::take(&mut input.events);
+        input.events = events
+            .into_iter()
+            .filter_map(|event| {
+                if pressed(&event, &outdents) {
+                    lines.push(true);
+                    None
+                } else if pressed(&event, &indents) {
+                    if selected || !lines.is_empty() {
+                        lines.push(false);
+                        None
+                    } else {
+                        Some(egui::Event::Text(unit.to_owned()))
+                    }
+                } else {
+                    Some(event)
+                }
+            })
+            .collect();
+    });
+    let mut changed = false;
+    for outdent in lines {
+        let mut state = egui::TextEdit::load_state(ui.ctx(), field.id).unwrap_or_default();
+        let range = state.cursor.char_range().unwrap_or_else(|| {
+            CCursorRange::one(CCursor::new(char_index(&sql_tab.text, sql_tab.cursor)))
+        });
+        let [start, end] = range.sorted_cursors();
+        let byte = |cursor: CCursor| byte_offset(&sql_tab.text, cursor.index.0);
+        let selection = byte(start)..byte(end);
+        let Some((text, after)) = indented(&sql_tab.text, selection, field.indent, outdent) else {
+            continue;
+        };
+        let at = |byte: usize| CCursor::new(char_index(&text, byte));
+        // The end that moves stays the one that moved: a selection made
+        // backwards is still one.
+        let backwards = range.primary.index < range.secondary.index;
+        let after_range = match (after.is_empty(), backwards) {
+            (true, _) => CCursorRange::one(at(after.end)),
+            (false, false) => CCursorRange::two(at(after.start), at(after.end)),
+            (false, true) => CCursorRange::two(at(after.end), at(after.start)),
+        };
+        let cursor = if backwards { after.start } else { after.end };
+        let before = (range, std::mem::take(&mut sql_tab.text));
+        let after_edit = (after_range, text);
+        outside_edit(
+            &mut state,
+            ui.input(|input| input.time),
+            &before,
+            &after_edit,
+        );
+        egui::TextEdit::store_state(ui.ctx(), field.id, state);
+        sql_tab.text = after_edit.1;
+        sql_tab.cursor = cursor;
+        changed = true;
+    }
+    changed
 }
 
 /// The text field itself: edits the tab's text and reports its cursor.
@@ -696,7 +916,11 @@ fn edit(ui: &mut Ui, sql_tab: &mut SqlTab, field: &Field<'_>) -> Edited {
     let inserted = insert_completion(ui.ctx(), field.id, sql_tab);
     let focus = std::mem::take(&mut sql_tab.focus_editor);
     // Before the field is drawn, so this frame shows the formatted text.
+    // An indent is an outside edit too: what the field shows, and where
+    // its caret goes, follow as after Format.
+    let indented = indent_script(ui, sql_tab, field);
     let formatted = std::mem::take(&mut sql_tab.format) && format_script(ui, sql_tab, field);
+    let formatted = formatted || indented;
     // The field fills the pane, so a click under the last line lands in
     // it: whole lines, and the rest as bottom padding. (egui sizes a text
     // edit in lines of its font.)
```

- [ ] **Step 4: Run the tests to see them pass**

Run: `cargo test --locked --lib -- ui::format_settings_tests ui::sql_text completion ui::complete_tests ui::tests keymap`
Expected: PASS (the keymap's own tests among them: Tab is also `InsertCompletion`'s, under `When::Completion`, and the completion list's keys are read first, in `src/ui/keys.rs`, before the editor's). Then the four checks above.

- [ ] **Step 5: Commit**

```bash
git add src/app.rs src/completion.rs src/keymap.rs src/ui/complete_tests.rs src/ui/format_settings_tests.rs src/ui/mod.rs src/ui/sql_text.rs
git commit -m "Format, indent with Tab and offer keywords as Settings › Editor says"
```

---

### Task 4: The website

**Files:**
- Modify: `docs/_reference/settings-and-files.md`, `docs/_reference/keyboard-shortcuts.md`, `docs/_guide/sql-editor.md`, `docs/_guide/macos.md`, `docs/_guide/omarchy.md`

- [ ] **Step 1: Say what the rows and the keys do**

```diff
diff --git a/docs/_guide/macos.md b/docs/_guide/macos.md
--- a/docs/_guide/macos.md
+++ b/docs/_guide/macos.md
@@ -198,6 +198,7 @@ column's line, or come to them with Tab or F6.
 | Cmd+Shift+Return | Run all |
 | Cmd+. | Cancel the run |
 | Cmd+Shift+F | Format |
+| Tab, Shift+Tab | Indent, outdent: at the cursor, or every line of the selection |
 | Cmd+Shift+M | Switch between read-only and read-write runs |
 | Cmd+I, Ctrl+Space | Complete |
 
```

```diff
diff --git a/docs/_guide/omarchy.md b/docs/_guide/omarchy.md
--- a/docs/_guide/omarchy.md
+++ b/docs/_guide/omarchy.md
@@ -267,6 +267,7 @@ A save can ask you something first. Each question has its own keys:
 | Ctrl+C | Cancel the run, while one runs. With none running it copies |
 | Esc | Leave the editor: the letters below are read from then on |
 | `=` | Format, with the keyboard out of the text |
+| Tab, Shift+Tab | Indent, outdent, in the text: at the cursor, or every line of the selection |
 | `:ro`, `:rw` | Read-only and read-write runs, with the keyboard out of the text |
 | Ctrl+Space | Complete |
 
```

```diff
diff --git a/docs/_guide/sql-editor.md b/docs/_guide/sql-editor.md
--- a/docs/_guide/sql-editor.md
+++ b/docs/_guide/sql-editor.md
@@ -82,17 +82,26 @@ back.
 ## Format
 
 **Format** (Cmd/Ctrl+Shift+F; in the Omarchy look Esc, then `=`) lays
-queries out in river style and
-uppercases reserved words. It formats the statements in the selection, or
-the whole script when nothing is selected. Only whitespace and the case of
-keywords change.
+queries out in river style, with reserved words in the case
+**Settings › Editor** sets (upper case on macOS and Windows and lower
+case in the Omarchy look until you choose, or as you typed them), the
+commas of a `SELECT`'s or a `WITH`'s list trailing or leading, and a
+`CASE`'s parts in by the indent set. It formats the statements in the selection, or the whole
+script when nothing is selected. Only whitespace and the case of
+keywords change, and one undo takes it back.
+
+Tab puts in an indent (two spaces until you set four or a tab), at the
+start of every line a selection touches when there is one (blank lines
+stay blank), and Shift+Tab takes one off. While the completion list is
+open, Tab accepts its row first.
 
 ## Completion
 
 Keywords, schemas, tables, views and the columns of the tables in your
 statement are offered while you type. Ctrl+Space asks for the list
 anywhere, and so does Cmd/Ctrl+I on macOS and Windows. **Settings ›
-Editor** can have the list open only when you ask, or never.
+Editor** can have the list open only when you ask, or never, and sets
+the case keywords are offered in.
 
 {% include shot.html file="macos-sql-complete" alt="The completion list under a table name being typed, offering the tables that match" %}
 
```

```diff
diff --git a/docs/_reference/keyboard-shortcuts.md b/docs/_reference/keyboard-shortcuts.md
--- a/docs/_reference/keyboard-shortcuts.md
+++ b/docs/_reference/keyboard-shortcuts.md
@@ -130,6 +130,7 @@ These are the keys of the macOS and Windows looks. The Omarchy look has
 | Mod+Shift+Return | Run all |
 | Mod+. | Cancel the run |
 | Mod+Shift+F | Format |
+| Tab, Shift+Tab | Indent, outdent: at the cursor, or every line of the selection |
 | Mod+Shift+M | Switch between read-only and read-write runs |
 | Ctrl+Space, Mod+I | Complete |
 | Up, Down | Move in the completion list |
```

```diff
diff --git a/docs/_reference/settings-and-files.md b/docs/_reference/settings-and-files.md
--- a/docs/_reference/settings-and-files.md
+++ b/docs/_reference/settings-and-files.md
@@ -52,9 +52,12 @@ open SQL editor follows each change at once.
 | **Row limit** | 100, 500, 1,000, 5,000 or 10,000 rows for each statement, or No limit. The rows past the limit are not fetched; nothing is added to your SQL. **Show all** under a cut result fetches the rest | 1,000 |
 | **Timeout** | 5, 15 or 30 seconds, 1 or 5 minutes, or Off. A run still going then is cancelled and rolled back; Cancel works either way. **Run without timeout** runs a stopped run again with none | 30 s |
 | **⌘↩ runs** (Omarchy: ctrl+enter runs) | Statement at cursor, or Whole tab: every statement in order, stopping at the first error. A selection always runs what it selects, whichever is set | Statement at cursor |
-| **Autocomplete** | As you type, On Ctrl+Space (the list opens only when you ask), or Off | As you type |
+| **Indent** | 2 spaces, 4 spaces or Tab: what Tab puts in at the cursor, or at the start of every line a selection touches, and Shift+Tab takes off them; and how far Format sets a `CASE`'s `WHEN` and `ELSE` in (four for Tab: the layout is set in spaces) | 2 spaces |
+| **Autocomplete** | As you type, On Ctrl+Space (the list opens only when you ask), or Off. Keywords are offered in the case **Keywords** sets, upper case with Keep | As you type |
 | **Line numbers** | On or off on macOS and Windows; relative, absolute or off in the Omarchy look. On writes `absolute`; a `relative` set in the file shows as on and stays | On (absolute) on macOS and Windows, relative in the Omarchy look |
 | **Current line highlight** (macOS and Windows) | On or off: the line the cursor is on is filled while the editor has the keyboard. The Omarchy look has no row: it follows the key, set in the file | On |
+| **Keywords** (in Format · Cmd/Ctrl+Shift+F; Omarchy: format · `=` in normal mode) | UPPERCASE, lowercase or Keep: the case Format writes reserved words in. Keep leaves them as you typed them. Names, strings and comments never change | UPPERCASE on macOS and Windows, lowercase in the Omarchy look |
+| **Commas** | Trailing (at the end of an item's line) or Leading (at the start of the next item's, on the edge of the river), in the lists Format breaks: a `SELECT`'s and a `WITH`'s. A comma with a comment after it, on its line or the next, stays at the end of its line | Trailing |
 | **Keep query history** (Omarchy: keep) | The last 100, 1,000 or 10,000 statements of each connection, on this computer, or Don't keep: every statement run from the SQL editor, a grid's save, a filter or sort you set, and agents' runs, as the [History tab]({% link _guide/sql-editor.md %}#history) shows them. Results are never kept. Don't keep records nothing more, and asks whether to delete what is kept (Omarchy: `D` deletes, `n` keeps). **Show History** beside it (Omarchy: Enter) shows the window's History tab | Last 1,000 |
 
 The other keys of the four `[editor.…]` tables are read and kept now, and
@@ -166,14 +169,14 @@ bar = "tint"
 | `editor.run.run` | `"statement"` or `"all"`: what Cmd/Ctrl+Return runs | `"statement"` |
 | `editor.run.confirm_unsafe_writes` | `true` or `false`. Read and kept; not acted on yet | `true` |
 | `editor.typing.keys` | `"standard"` or `"vim"`. Read and kept; not acted on yet | Not set: standard on macOS and Windows, vim in the Omarchy look |
-| `editor.typing.indent` | `2`, `4` or `"tab"`. Read and kept; not acted on yet | `2` |
+| `editor.typing.indent` | `2`, `4` or `"tab"` | `2` |
 | `editor.typing.autocomplete` | `"typing"`, `"manual"` or `"off"` | `"typing"` |
 | `editor.typing.close_pairs` | `true` or `false`. Read and kept; not acted on yet | `true` |
 | `editor.typing.line_numbers` | `"absolute"`, `"relative"` or `"off"` | Not set: absolute on macOS and Windows, relative in the Omarchy look |
 | `editor.typing.highlight_line` | `true` or `false` | `true` |
 | `editor.typing.wrap` | `true` or `false`. Read and kept; not acted on yet | `false` |
-| `editor.format.keywords` | `"upper"`, `"lower"` or `"keep"`. Read and kept; not acted on yet | Not set: upper on macOS and Windows, lower in the Omarchy look |
-| `editor.format.commas` | `"trailing"` or `"leading"`. Read and kept; not acted on yet | `"trailing"` |
+| `editor.format.keywords` | `"upper"`, `"lower"` or `"keep"` | Not set: upper on macOS and Windows, lower in the Omarchy look |
+| `editor.format.commas` | `"trailing"` or `"leading"` | `"trailing"` |
 | `editor.history.keep` | 0 to 100000 statements kept for each connection, from every source; `0` keeps none. A smaller number deletes the oldest past it | `1000` |
 | `appearance.interface_size` | `"small"`, `"default"` or `"large"`, on macOS and Windows | `"default"` |
 | `appearance.font` | On macOS the data's face, `"IBM Plex Mono"`, `"JetBrains Mono"` or the family name of an installed font (`"Menlo"`); `"monospace"` is IBM Plex Mono too, and a name no installed font has is drawn as IBM Plex Mono, which the window says. Windows draws its data in Inter, whatever it says. In the Omarchy look a fontconfig name (`"Fira Code"`); `"monospace"` is the system's. Written once you set it | Not set: IBM Plex Mono, or the system's monospace |
```

- [ ] **Step 2: Build and check the site**

Run: `cd docs && bin/check` where Ruby and the bundle are installed; otherwise leave it to `docs.yml` on the pull request.
Expected: the build and its checks pass.

- [ ] **Step 3: Commit**

```bash
git add docs/_guide/macos.md docs/_guide/omarchy.md docs/_guide/sql-editor.md docs/_reference/keyboard-shortcuts.md docs/_reference/settings-and-files.md
git commit -m "Describe Settings › Editor's Format and indent on the website"
```

---

### Task 5: The pull request

- [ ] Push the branch and open one pull request: a summary, the rows built, the keys added (Tab and Shift+Tab in the editor, from the keymap) and the keymap check, the departures above, and what each look does by default. No screenshot.
