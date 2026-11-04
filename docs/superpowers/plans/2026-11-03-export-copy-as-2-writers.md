# Export and Copy as, run 2: the writers and Copy as Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** The six format writers an export needs (CSV, TSV, JSON, JSON Lines, SQL INSERT, Markdown), each format's options remembered in the state directory, Mod+C and `y` on the TSV writer, and Copy as: Mod+Alt+C and `gy` over the grid's selection, the grid's right-click menu, and the row panel's Copy row ▾, with the clipboard's limit of 10,000 rows and 20 MB.

**Architecture:** `crates/tabletist-core/src/export.rs` holds the formats, their options (`Options`, read from and written to `export_options.json` with what this version cannot read kept), and `Writer`, which writes a format a row at a time (`start`, `row`, `finish`) so runs 3 and 4 can stream through it; `clipboard` writes a selection under the limits. `tabletist_db::Dialect::value_literal` writes SQL INSERT's literals as a save does. The app's `copy_in` reads a grid's selection (a table's or a SQL result's) into a writer; `Action::CopyAs` reduces a copy into `App::to_clipboard` and `App::said`, which the frame takes to the clipboard and the toast. `Dialog::CopyAs` is the menu, drawn and read by `src/ui/copy_as.rs`; the grid draws the right-click menu through `grid::Picked::menu`.

**Tech Stack:** Rust, egui (crmne fork) with AccessKit, headless tests through `src/testing.rs`.

**Spec:** `docs/superpowers/specs/2026-11-03-export-copy-as-2-writers-design.md`. Igor approved its nine departures (D1 to D9) and thirteen decisions (Q1 to Q13) as written on 2026-11-03. Two reviews of the draft (one breaking it with throwaway tests, one reading this plan against its diffs) found what B5 to B14 settle and the defects the draft now fixes.

---

## Before Task 1

Commit the spec and this plan:

```bash
git add docs/superpowers/specs/2026-11-03-export-copy-as-2-writers-design.md docs/superpowers/plans/2026-11-03-export-copy-as-2-writers.md
git commit -m "Plan Export and Copy as, run 2: the writers and Copy as"
```

## Where this sits

Four runs, a pull request each: 1 the selection (merged, PR #248); **2 the writers and Copy as (this plan)**, a branch of `main` at `d5d311e3`; 3 export a table; 4 export a SQL result.

## Decided while building

The spec's departures and decisions stand as approved. Building it settled these, for a yes or a no:

| # | The spec | Built | Why, and the other way |
|---|---|---|---|
| B1 | The menu's keys | Ten commands in the keymap, Prompt scope while `When::CopyAs` holds: the six letters, Next and Previous (Down/Up; Ctrl+N/Ctrl+P and Down/Up in the terminal look), Pick (Enter) and Close (Esc). Settings › Keyboard lists them under Prompts. The menu reads them at the frame's start, before a field under it could take Enter, and gives the keyboard back where it was when it closes | Keys are the keymap's only. The other way: the menu reads Enter and Esc itself, as the agents' box does |
| B2 | Markdown pads numeric columns to the right | The column's name too | As a Markdown formatter aligns a column. The other way: names always to the left |
| B3 | The terminal look's float | The rows as `c  csv`, the lit one marked `▌`, under a title line ("copy as", or "copy row as" over the row panel's row, then the size) and a key line; the letter is not drawn in the accent | `menu::Item` draws a row in one colour. The other way: a row drawn by hand |
| B4 | The status line's feedback, "✓ copied …" | Without the ✓, as "copy sql" and the other copies say it today | The toast's text is what the line says. The other way: a mark for copies |
| B5 | The terminal look's inspector key line `yy copy row · gy copy row as` | Not drawn: the panel's header gives its hints up as the panel narrows, and at the width it opens at these never fit. Settings › Keyboard and the Omarchy guide list both keys | The header is the line the panel has. The other way: a key line at the panel's foot |
| B6 | The grid's right-click | In both looks, as run 1's clicks are. On a row's number it selects that row (one in the selection stays). Its Copy as › opens on hover or click, as egui's submenus do; its letters are drawn, and read in the keyboard menu only. Copy and Copy rows as TSV say nothing, as Mod+C and Mod+Shift+C | A submenu's own keys would need a second reader. The other way: letters in the submenu too |
| B7 | The row's TSV | Copy row as TSV, from Copy row ▾ or the float's `t` over the row panel's row, copies as ⇧⌘C does: no header row, and "Copied 1 row as TSV". The row panel's float has no Column names | It names ⇧⌘C. The other way: TSV's options, header and all |
| B8 | `yy` "unchanged" | `yy` puts the row on the clipboard through the same TSV writer as ⇧⌘C: a NULL empty, a tab `\t`, binary in hex. Over the clipboard's limits the row is yanked and not copied, and the toast says why | One TSV for every copy. The other way: `yy` keeps the old text (a tab as a space, NULL written out) |
| B9 | "Copy row as TSV ⇧⌘C" | Drawn without ⇧⌘C | The menu of actions it is built on has no column for a shortcut. The other way: a menu with one |
| B10 | CSV's Every value quotes "every value but a NULL written Empty" | A NULL written as Text is not quoted either | A quoted `\N` reads back as the text. The other way: quote it |
| B11 | The spec's sketch of `Writer` | `Writer::new(format, options, columns, target)`, then `start`, `row` for each row and `finish`; `Columns` holds no type names (no writer needs them: a pending DEFAULT comes with its cell, as `export::Pending`) | One row at a time is what a stream of batches is made of. The other way: `rows(batch)` |
| B12 | (a byte-order mark on the clipboard) | The clipboard never takes one, whatever CSV's and TSV's option says | It is for Excel opening a file. The other way: as the option says |
| B13 | (a pending DEFAULT or time of the save) | SQL INSERT writes what a save sends: `DEFAULT`, SQLite's default expression, the dialect's clock (`now()`). The other formats write the word the grid shows | A copied INSERT must not store the word. The other way: the loaded value |
| B14 | (Copy as on a new row with no range) | Copies the row, what is not set in it NULL, and in SQL INSERT its column's default, as a save leaves it. Mod+C there still copies nothing, as before | The row is what is selected. The other way: nothing, as Mod+C |

## Files

| File | Task | What |
|---|---|---|
| `crates/tabletist-core/src/export.rs` (new), `export/golden/` (22 new files), `crates/tabletist-core/src/lib.rs`, `src/lib.rs`, `.gitattributes` | 1 | The writers, the options, their tests and golden texts |
| `crates/tabletist-db/src/dialect.rs`, `value.rs`, `lib.rs` | 1 | `Dialect::value_literal`, `is_plain_number` |
| `crates/tabletist-core/src/format.rs` | 1, 2 | `float_text` public (1); `tsv_row` and `tsv_cells` go (2) |
| `crates/tabletist-core/src/paths.rs`, `backend.rs`, `src/app.rs` | 1, 2 | `export_options.json`, `StateFile::ExportOptions`, `App::export` (1); the copies and their reducers (2) |
| `crates/tabletist-core/src/keymap.rs`, `model.rs` | 2 | The commands, `When::CopyAs`; `CopyPick`, `CopyHow`, `CopyAsMenu`, the actions |
| `src/ui/copy_as.rs` (new), `src/ui/copy_as_tests.rs` (new), `src/ui/mod.rs`, `keys.rs`, `grid.rs`, `data_view.rs`, `sql_results.rs`, `row_panel.rs`, `workspace.rs`, `selection_tests.rs` | 2 | The menu, the float, the right-click menu, Copy row ▾, the keys, the key lines, the tests |
| `docs/_guide/{browsing-data,macos,omarchy}.md`, `docs/_reference/keyboard-shortcuts.md` | 2 | The website |

The tests run as before: `cargo test --locked --workspace --all-targets`; no server is needed. Use `~/.cargo/bin/cargo` where the mise shim fails.

---

### Task 1: The six writers and the remembered options

**Files:** `.gitattributes`, `crates/tabletist-core/src/backend.rs`, `crates/tabletist-core/src/export.rs`, `crates/tabletist-core/src/format.rs`, `crates/tabletist-core/src/lib.rs`, `crates/tabletist-core/src/paths.rs`, `crates/tabletist-db/src/dialect.rs`, `crates/tabletist-db/src/lib.rs`, `crates/tabletist-db/src/value.rs`, `src/app.rs`, `src/lib.rs`, `crates/tabletist-core/src/export/golden/` (22 new files)

The writers and their options, with nothing that copies through them yet: `Writer` and `clipboard` in the core, `Dialect::value_literal` in the database layer, the options file and the app's field that holds them.

- [ ] **Step 1: Write the tests**

The writers' tests in `export.rs`'s `tests` module and the golden texts they compare with, and the database layer's three tests. Apply:

```diff
diff --git a/.gitattributes b/.gitattributes
--- a/.gitattributes
+++ b/.gitattributes
@@ -3,3 +3,6 @@
 
 # Expected texts the DDL tests compare byte for byte: LF on every checkout.
 crates/tabletist-db/tests/ddl/*.sql text eol=lf
+
+# Expected texts of the export writers, byte for byte: CRLF and the BOM stay.
+crates/tabletist-core/src/export/golden/** -text
diff --git a/crates/tabletist-core/src/export.rs b/crates/tabletist-core/src/export.rs
new file mode 100644
--- /dev/null
+++ b/crates/tabletist-core/src/export.rs
@@ -0,0 +1,612 @@
+//! The text of an export and of Copy as: six formats, each written with
+//! the options a person last chose for it.
+//!
+//! The values are the database's, never the grid's display: no digit
+//! grouping, no value tags, no display zone. JSON's dates are the one
+//! exception: ISO 8601, with an offset where the value has one. A writer
+//! takes a row at a time, so an export can stream through it; Markdown
+//! alone keeps its rows until the end, to pad its columns.
+//!
+//! The options are kept in the state directory
+//! (`AppDirs::export_options_file`), by format. What the file holds that
+//! this version cannot read is kept and written back.
+
+#[cfg(test)]
+#[allow(clippy::unwrap_used)]
+mod tests {
+    use super::*;
+
+    /// Rows of Bookshop's `reviews`: a NULL, an empty string, a quote, a
+    /// comma, a tab, a newline, a pipe, a backslash, unicode, jsonb, bytea
+    /// and timestamptz.
+    fn reviews() -> (Vec<&'static str>, Vec<ValueKind>, Vec<Vec<Value>>) {
+        use ValueKind::{Binary, Json, Numeric, Temporal, Text};
+        let names = vec![
+            "id",
+            "rating",
+            "title",
+            "body",
+            "metadata",
+            "cover",
+            "created_at",
+        ];
+        let kinds = vec![Numeric, Numeric, Text, Text, Json, Binary, Temporal];
+        let text = |text: &str| Value::Text(text.into());
+        let rows = vec![
+            vec![
+                Value::Int(10),
+                Value::Int(4),
+                text("The Tide Atlas"),
+                text("Maps the coast, one \"harbor\" at a time."),
+                text(r#"{"tags": ["sea", "maps"], "helpful": 12}"#),
+                Value::Bytes(vec![0x89, 0x50, 0x4e, 0x47].into()),
+                text("2026-02-06 08:05:00+00"),
+            ],
+            vec![
+                Value::Int(11),
+                Value::Int(5),
+                text("Harbor Lights"),
+                text("Lovely\tsecond half\nand a pipe | here"),
+                Value::Null,
+                Value::Null,
+                text("2026-02-06 08:06:12.5+00"),
+            ],
+            vec![
+                Value::Int(12),
+                Value::Null,
+                text(""),
+                text("Café Ünïcödé 東京, shelved under C:\\fiction"),
+                text(r#"{"a": {"b": [1, 2.50]}, "z": null}"#),
+                Value::Bytes(Vec::new().into()),
+                text("2026-02-11 12:40:00+05:30"),
+            ],
+        ];
+        (names, kinds, rows)
+    }
+
+    fn written(
+        format: Format,
+        options: &Options,
+        dialect: Dialect,
+        create: Option<&str>,
+    ) -> String {
+        let (names, kinds, rows) = reviews();
+        let object = ObjectRef::new("public", "reviews");
+        let columns = Columns {
+            names,
+            kinds,
+            dialect,
+        };
+        let target = Target::Table {
+            object: &object,
+            create,
+        };
+        let mut writer = Writer::new(format, options, columns, target);
+        let mut out = String::new();
+        writer.start(&mut out);
+        for row in &rows {
+            writer.row(row, &mut out);
+        }
+        writer.finish(&mut out);
+        out
+    }
+
+    /// The text `name` in `export/golden`, byte for byte.
+    fn golden(name: &str) -> String {
+        let path = format!("{}/src/export/golden/{name}", env!("CARGO_MANIFEST_DIR"));
+        std::fs::read_to_string(&path).unwrap_or_else(|error| panic!("{path}: {error}"))
+    }
+
+    fn check(name: &str, format: Format, options: &Options) {
+        check_in(name, format, options, Dialect::Postgres, None);
+    }
+
+    fn check_in(
+        name: &str,
+        format: Format,
+        options: &Options,
+        dialect: Dialect,
+        create: Option<&str>,
+    ) {
+        let text = written(format, options, dialect, create);
+        assert_eq!(text, golden(name), "{name}");
+    }
+
+    fn with(change: impl FnOnce(&mut Options)) -> Options {
+        let mut options = Options::default();
+        change(&mut options);
+        options
+    }
+
+    #[test]
+    fn csv_is_written_as_each_option_says() {
+        check("csv-default.csv", Format::Csv, &Options::default());
+        check(
+            "csv-no-header.csv",
+            Format::Csv,
+            &with(|o| o.csv.header = false),
+        );
+        check(
+            "csv-semicolon.csv",
+            Format::Csv,
+            &with(|o| o.csv.delimiter = ';'),
+        );
+        check(
+            "csv-every-value.csv",
+            Format::Csv,
+            &with(|o| o.csv.quoting = Quoting::Every),
+        );
+        check(
+            "csv-null-text.csv",
+            Format::Csv,
+            &with(|o| o.csv.null_as = NullAs::Text),
+        );
+        check("csv-bom.csv", Format::Csv, &with(|o| o.csv.bom = true));
+        check(
+            "csv-crlf.csv",
+            Format::Csv,
+            &with(|o| o.csv.line_ending = LineEnding::Crlf),
+        );
+        check(
+            "csv-base64.csv",
+            Format::Csv,
+            &with(|o| o.csv.binary = Binary::Base64),
+        );
+    }
+
+    #[test]
+    fn tsv_is_written_as_each_option_says() {
+        check("tsv-default.tsv", Format::Tsv, &Options::default());
+        check(
+            "tsv-no-header.tsv",
+            Format::Tsv,
+            &with(|o| o.tsv.header = false),
+        );
+        check(
+            "tsv-null-text.tsv",
+            Format::Tsv,
+            &with(|o| o.tsv.null_as = NullAs::Text),
+        );
+    }
+
+    #[test]
+    fn json_is_written_as_each_option_says() {
+        check("json-default.json", Format::Json, &Options::default());
+        let strings = with(|o| o.json.values = JsonValues::Strings);
+        check("json-strings.json", Format::Json, &strings);
+        check(
+            "json-one-line.json",
+            Format::Json,
+            &with(|o| o.json.layout = Layout::OneLine),
+        );
+        check(
+            "jsonl-default.jsonl",
+            Format::JsonLines,
+            &Options::default(),
+        );
+    }
+
+    #[test]
+    fn sql_insert_is_written_as_each_option_says() {
+        check("sql-default.sql", Format::SqlInsert, &Options::default());
+        let one = with(|o| o.sql.rows_per_insert = 1);
+        check("sql-one-per-insert.sql", Format::SqlInsert, &one);
+        let create = "CREATE TABLE public.reviews (\n  id bigint PRIMARY KEY\n);\n";
+        let started = with(|o| o.sql.create_table = true);
+        check_in(
+            "sql-create.sql",
+            Format::SqlInsert,
+            &started,
+            Dialect::Postgres,
+            Some(create),
+        );
+        // Asked for, with no definition to hand: none.
+        check_in(
+            "sql-default.sql",
+            Format::SqlInsert,
+            &started,
+            Dialect::Postgres,
+            None,
+        );
+        check_in(
+            "sql-mysql.sql",
+            Format::SqlInsert,
+            &Options::default(),
+            Dialect::MySql,
+            None,
+        );
+        check_in(
+            "sql-sqlite.sql",
+            Format::SqlInsert,
+            &Options::default(),
+            Dialect::Sqlite,
+            None,
+        );
+    }
+
+    #[test]
+    fn markdown_is_written_as_each_option_says() {
+        check("md-default.md", Format::Markdown, &Options::default());
+        check(
+            "md-null-empty.md",
+            Format::Markdown,
+            &with(|o| o.md.null_as = NullAs::Empty),
+        );
+    }
+
+    #[test]
+    fn markdown_cuts_a_long_value_where_asked() {
+        let options = with(|o| o.md.long_values = LongValues::Cut);
+        let columns = Columns {
+            names: vec!["body"],
+            kinds: vec![ValueKind::Text],
+            dialect: Dialect::Postgres,
+        };
+        let mut writer = Writer::new(Format::Markdown, &options, columns, Target::Named("result"));
+        let mut out = String::new();
+        writer.start(&mut out);
+        writer.row(
+            &[Value::Text("é".repeat(MARKDOWN_CUT + 1).into())],
+            &mut out,
+        );
+        writer.finish(&mut out);
+        let cell = out.lines().nth(2).unwrap();
+        assert_eq!(cell, format!("| {}… |", "é".repeat(MARKDOWN_CUT)));
+    }
+
+    #[test]
+    fn a_value_over_256_kib_is_written_whole_in_every_format() {
+        let long = "q".repeat(300 * 1024);
+        let columns = || Columns {
+            names: vec!["body"],
+            kinds: vec![ValueKind::Text],
+            dialect: Dialect::Postgres,
+        };
+        let options = Options::default();
+        for format in Format::ALL {
+            let mut writer = Writer::new(format, &options, columns(), Target::Named("result"));
+            let mut out = String::new();
+            writer.start(&mut out);
+            writer.row(&[Value::Text(long.as_str().into())], &mut out);
+            writer.finish(&mut out);
+            assert!(out.contains(&long), "{format:?}");
+        }
+    }
+
+    #[test]
+    fn rows_in_two_batches_write_what_one_batch_does() {
+        let (names, kinds, rows) = reviews();
+        let options = with(|o| o.sql.rows_per_insert = 2);
+        for format in Format::ALL {
+            let object = ObjectRef::new("public", "reviews");
+            let target = Target::Table {
+                object: &object,
+                create: None,
+            };
+            let columns = Columns {
+                names: names.clone(),
+                kinds: kinds.clone(),
+                dialect: Dialect::Postgres,
+            };
+            let mut writer = Writer::new(format, &options, columns, target);
+            let (mut first, mut second) = (String::new(), String::new());
+            writer.start(&mut first);
+            writer.row(&rows[0], &mut first);
+            for row in &rows[1..] {
+                writer.row(row, &mut second);
+            }
+            writer.finish(&mut second);
+            let whole = written(format, &options, Dialect::Postgres, None);
+            assert_eq!(format!("{first}{second}"), whole, "{format:?}");
+        }
+    }
+
+    #[test]
+    fn json_dates_and_times_are_iso_8601() {
+        assert_eq!(
+            iso_8601("2026-02-06 08:05:00+00"),
+            "2026-02-06T08:05:00+00:00"
+        );
+        assert_eq!(
+            iso_8601("2026-02-06 08:05:00.25-08"),
+            "2026-02-06T08:05:00.25-08:00"
+        );
+        assert_eq!(
+            iso_8601("2026-02-06 08:05:00+05:30"),
+            "2026-02-06T08:05:00+05:30"
+        );
+        assert_eq!(iso_8601("2026-02-06 08:05:00"), "2026-02-06T08:05:00");
+        assert_eq!(
+            iso_8601("2026-02-06 08:05:00+0530"),
+            "2026-02-06T08:05:00+05:30"
+        );
+        assert_eq!(iso_8601("2026-02-06 08:05:00Z"), "2026-02-06T08:05:00Z");
+        // What ISO 8601 has no way to write is left as the database wrote it.
+        for kept in [
+            "2026-02-06",
+            "08:05:00",
+            "1 day 02:00:00",
+            "infinity",
+            "2026-02-06 08:05:00 BC",
+            "0044-03-15 12:00:00+00 BC",
+            "1900-01-01 00:00:00+02:30:17",
+        ] {
+            assert_eq!(iso_8601(kept), kept);
+        }
+    }
+
+    #[test]
+    fn json_numbers_and_documents_keep_every_digit() {
+        let numeric = Value::Text("123456789012345678901234567890.50".into());
+        let written = json_value(
+            Some(&numeric),
+            ValueKind::Numeric,
+            JsonValues::Nested,
+            Binary::Hex,
+            true,
+        );
+        assert_eq!(written, "123456789012345678901234567890.50");
+        let money = Value::Text("$1.00".into());
+        let written = json_value(
+            Some(&money),
+            ValueKind::Numeric,
+            JsonValues::Nested,
+            Binary::Hex,
+            true,
+        );
+        assert_eq!(written, "\"$1.00\"");
+        let broken = Value::Text("{nope".into());
+        let written = json_value(
+            Some(&broken),
+            ValueKind::Json,
+            JsonValues::Nested,
+            Binary::Hex,
+            true,
+        );
+        assert_eq!(written, "\"{nope\"");
+        let nan = Value::Float(f64::NAN);
+        let written = json_value(
+            Some(&nan),
+            ValueKind::Numeric,
+            JsonValues::Nested,
+            Binary::Hex,
+            true,
+        );
+        assert_eq!(written, "\"NaN\"");
+    }
+
+    #[test]
+    fn a_pending_default_is_written_as_a_save_sends_it() {
+        let columns = Columns {
+            names: vec!["id", "kind"],
+            kinds: vec![ValueKind::Numeric, ValueKind::Text],
+            dialect: Dialect::Postgres,
+        };
+        let row = vec![
+            Pending {
+                value: Some(Value::Int(2)),
+                sql: None,
+            },
+            Pending {
+                value: Some(Value::Text("DEFAULT".into())),
+                sql: Some("DEFAULT".into()),
+            },
+        ];
+        let options = Options::default();
+        let target = Target::Named("result");
+        let copied = clipboard(
+            Format::SqlInsert,
+            &options,
+            columns.clone(),
+            target,
+            1,
+            [row.clone()].into_iter(),
+        );
+        let Copied::Text { text, .. } = copied else {
+            panic!("{copied:?}");
+        };
+        assert_eq!(
+            text,
+            "INSERT INTO result (id, kind) VALUES\n  (2, DEFAULT);"
+        );
+        // Elsewhere it is the word the grid shows.
+        let copied = clipboard(Format::Csv, &options, columns, target, 1, [row].into_iter());
+        let Copied::Text { text, .. } = copied else {
+            panic!("{copied:?}");
+        };
+        assert_eq!(text, "id,kind\n2,DEFAULT");
+    }
+
+    #[test]
+    fn markdown_stops_once_past_the_clipboards_limit() {
+        let columns = Columns {
+            names: vec!["body"],
+            kinds: vec![ValueKind::Text],
+            dialect: Dialect::Postgres,
+        };
+        let taken = std::cell::Cell::new(0);
+        let rows = (0..100).map(|_| {
+            taken.set(taken.get() + 1);
+            vec![Value::Text("w".repeat(1 << 20).into())]
+        });
+        let options = Options::default();
+        let copied = clipboard(
+            Format::Markdown,
+            &options,
+            columns,
+            Target::Named("result"),
+            100,
+            rows,
+        );
+        assert_eq!(copied, Copied::TooLarge);
+        assert!(taken.get() <= 21, "{} rows read", taken.get());
+    }
+
+    #[test]
+    fn the_clipboard_takes_no_byte_order_mark() {
+        let columns = Columns {
+            names: vec!["id"],
+            kinds: vec![ValueKind::Numeric],
+            dialect: Dialect::Postgres,
+        };
+        let options = with(|o| o.csv.bom = true);
+        let rows = [vec![Value::Int(1)]].into_iter();
+        let copied = clipboard(
+            Format::Csv,
+            &options,
+            columns,
+            Target::Named("result"),
+            1,
+            rows,
+        );
+        assert_eq!(
+            copied,
+            Copied::Text {
+                text: "id\n1".into(),
+                statements: 0
+            }
+        );
+    }
+
+    #[test]
+    fn base64_pads_as_the_standard_says() {
+        assert_eq!(base64(b""), "");
+        assert_eq!(base64(b"f"), "Zg==");
+        assert_eq!(base64(b"fo"), "Zm8=");
+        assert_eq!(base64(b"foo"), "Zm9v");
+        assert_eq!(base64(b"foob"), "Zm9vYg==");
+    }
+
+    #[test]
+    fn the_clipboard_takes_no_last_line_ending_and_keeps_its_limits() {
+        let columns = || Columns {
+            names: vec!["id"],
+            kinds: vec![ValueKind::Numeric],
+            dialect: Dialect::Postgres,
+        };
+        let options = with(|o| o.tsv.header = false);
+        let rows = (1..=2).map(|id| vec![Value::Int(id)]);
+        let copied = clipboard(
+            Format::Tsv,
+            &options,
+            columns(),
+            Target::Named("result"),
+            2,
+            rows,
+        );
+        assert_eq!(
+            copied,
+            Copied::Text {
+                text: "1\n2".into(),
+                statements: 0
+            }
+        );
+        let many = CLIPBOARD_ROWS + 1;
+        let rows = (0..many).map(|id| vec![Value::Int(id as i64)]);
+        let copied = clipboard(
+            Format::Tsv,
+            &options,
+            columns(),
+            Target::Named("result"),
+            many,
+            rows,
+        );
+        assert_eq!(copied, Copied::TooManyRows(many));
+        let big = Value::Text("w".repeat(CLIPBOARD_BYTES + 1).into());
+        let copied = clipboard(
+            Format::Tsv,
+            &options,
+            columns(),
+            Target::Named("result"),
+            1,
+            [vec![big]].into_iter(),
+        );
+        assert_eq!(copied, Copied::TooLarge);
+    }
+
+    #[test]
+    fn sql_insert_counts_its_statements() {
+        let options = with(|o| o.sql.rows_per_insert = 2);
+        let columns = Columns {
+            names: vec!["id"],
+            kinds: vec![ValueKind::Numeric],
+            dialect: Dialect::Postgres,
+        };
+        let rows = (1..=3).map(|id| vec![Value::Int(id)]);
+        let copied = clipboard(
+            Format::SqlInsert,
+            &options,
+            columns,
+            Target::Named("result"),
+            3,
+            rows,
+        );
+        assert_eq!(
+            copied,
+            Copied::Text {
+                text: "INSERT INTO result (id) VALUES\n  (1),\n  (2);\nINSERT INTO result (id) VALUES\n  (3);".into(),
+                statements: 2
+            }
+        );
+    }
+
+    #[test]
+    fn column_names_are_written_for_a_select_list() {
+        let names = ["rating", "Title", "order"];
+        assert_eq!(
+            column_names(&names, Dialect::Postgres),
+            "rating, \"Title\", \"order\""
+        );
+        assert_eq!(
+            column_names(&names, Dialect::MySql),
+            "rating, Title, `order`"
+        );
+    }
+
+    #[test]
+    fn options_are_kept_by_format_and_read_back() {
+        let dir = tempfile::tempdir().unwrap();
+        let path = dir.path().join("export_options.json");
+        assert_eq!(Options::load(&path), Options::default());
+        let mut options = Options::default();
+        options.csv.delimiter = ';';
+        options.tsv.null_as = NullAs::Text;
+        options.json.layout = Layout::OneLine;
+        options.jsonl.binary = Binary::Base64;
+        options.sql.rows_per_insert = 1;
+        options.md.long_values = LongValues::Cut;
+        crate::util::save_json(&path, &options.to_file()).unwrap();
+        assert_eq!(Options::load(&path), options);
+    }
+
+    #[test]
+    fn what_this_version_cannot_read_is_written_back() {
+        let file: BTreeMap<String, serde_json::Value> = serde_json::from_value(serde_json::json!({
+            "csv": {"delimiter": ";", "quoting": "sometimes", "newer": 1},
+            "xlsx": {"sheet": "one"},
+        }))
+        .unwrap();
+        let mut options = Options::from_file(file);
+        // What reads is taken; what does not, the default.
+        assert_eq!(options.csv.delimiter, ';');
+        assert_eq!(options.csv.quoting, Quoting::WhenNeeded);
+        let written = options.to_file();
+        assert_eq!(written["csv"]["quoting"], "sometimes");
+        assert_eq!(written["csv"]["newer"], 1);
+        assert_eq!(written["xlsx"]["sheet"], "one");
+        // An entry that is no set of options at all is kept as it was.
+        let file =
+            serde_json::from_value(serde_json::json!({"md": "from a newer version"})).unwrap();
+        let mut whole = Options::from_file(file);
+        assert_eq!(whole.md, Markdown::default());
+        assert_eq!(whole.to_file()["md"], "from a newer version");
+        whole.chosen(Format::Markdown);
+        assert_eq!(whole.to_file()["md"]["null_text"], "NULL");
+        // Chosen again, the option this version knows is its own.
+        options.csv.quoting = Quoting::Every;
+        options.chosen(Format::Csv);
+        let written = options.to_file();
+        assert_eq!(written["csv"]["quoting"], "every");
+        assert_eq!(written["csv"]["newer"], 1);
+    }
+}
diff --git a/crates/tabletist-core/src/export/golden/csv-base64.csv b/crates/tabletist-core/src/export/golden/csv-base64.csv
new file mode 100644
--- /dev/null
+++ b/crates/tabletist-core/src/export/golden/csv-base64.csv
@@ -0,0 +1,5 @@
+id,rating,title,body,metadata,cover,created_at
+10,4,The Tide Atlas,"Maps the coast, one ""harbor"" at a time.","{""tags"": [""sea"", ""maps""], ""helpful"": 12}",iVBORw==,2026-02-06 08:05:00+00
+11,5,Harbor Lights,"Lovely	second half
+and a pipe | here",,,2026-02-06 08:06:12.5+00
+12,,,"Café Ünïcödé 東京, shelved under C:\fiction","{""a"": {""b"": [1, 2.50]}, ""z"": null}",,2026-02-11 12:40:00+05:30
diff --git a/crates/tabletist-core/src/export/golden/csv-default.csv b/crates/tabletist-core/src/export/golden/csv-default.csv
new file mode 100644
--- /dev/null
+++ b/crates/tabletist-core/src/export/golden/csv-default.csv
@@ -0,0 +1,5 @@
+id,rating,title,body,metadata,cover,created_at
+10,4,The Tide Atlas,"Maps the coast, one ""harbor"" at a time.","{""tags"": [""sea"", ""maps""], ""helpful"": 12}",0x89504e47,2026-02-06 08:05:00+00
+11,5,Harbor Lights,"Lovely	second half
+and a pipe | here",,,2026-02-06 08:06:12.5+00
+12,,,"Café Ünïcödé 東京, shelved under C:\fiction","{""a"": {""b"": [1, 2.50]}, ""z"": null}",0x,2026-02-11 12:40:00+05:30
diff --git a/crates/tabletist-core/src/export/golden/csv-every-value.csv b/crates/tabletist-core/src/export/golden/csv-every-value.csv
new file mode 100644
--- /dev/null
+++ b/crates/tabletist-core/src/export/golden/csv-every-value.csv
@@ -0,0 +1,5 @@
+"id","rating","title","body","metadata","cover","created_at"
+"10","4","The Tide Atlas","Maps the coast, one ""harbor"" at a time.","{""tags"": [""sea"", ""maps""], ""helpful"": 12}","0x89504e47","2026-02-06 08:05:00+00"
+"11","5","Harbor Lights","Lovely	second half
+and a pipe | here",,,"2026-02-06 08:06:12.5+00"
+"12",,"","Café Ünïcödé 東京, shelved under C:\fiction","{""a"": {""b"": [1, 2.50]}, ""z"": null}","0x","2026-02-11 12:40:00+05:30"
diff --git a/crates/tabletist-core/src/export/golden/csv-no-header.csv b/crates/tabletist-core/src/export/golden/csv-no-header.csv
new file mode 100644
--- /dev/null
+++ b/crates/tabletist-core/src/export/golden/csv-no-header.csv
@@ -0,0 +1,4 @@
+10,4,The Tide Atlas,"Maps the coast, one ""harbor"" at a time.","{""tags"": [""sea"", ""maps""], ""helpful"": 12}",0x89504e47,2026-02-06 08:05:00+00
+11,5,Harbor Lights,"Lovely	second half
+and a pipe | here",,,2026-02-06 08:06:12.5+00
+12,,,"Café Ünïcödé 東京, shelved under C:\fiction","{""a"": {""b"": [1, 2.50]}, ""z"": null}",0x,2026-02-11 12:40:00+05:30
diff --git a/crates/tabletist-core/src/export/golden/csv-null-text.csv b/crates/tabletist-core/src/export/golden/csv-null-text.csv
new file mode 100644
--- /dev/null
+++ b/crates/tabletist-core/src/export/golden/csv-null-text.csv
@@ -0,0 +1,5 @@
+id,rating,title,body,metadata,cover,created_at
+10,4,The Tide Atlas,"Maps the coast, one ""harbor"" at a time.","{""tags"": [""sea"", ""maps""], ""helpful"": 12}",0x89504e47,2026-02-06 08:05:00+00
+11,5,Harbor Lights,"Lovely	second half
+and a pipe | here",\N,\N,2026-02-06 08:06:12.5+00
+12,\N,,"Café Ünïcödé 東京, shelved under C:\fiction","{""a"": {""b"": [1, 2.50]}, ""z"": null}",0x,2026-02-11 12:40:00+05:30
diff --git a/crates/tabletist-core/src/export/golden/csv-semicolon.csv b/crates/tabletist-core/src/export/golden/csv-semicolon.csv
new file mode 100644
--- /dev/null
+++ b/crates/tabletist-core/src/export/golden/csv-semicolon.csv
@@ -0,0 +1,5 @@
+id;rating;title;body;metadata;cover;created_at
+10;4;The Tide Atlas;"Maps the coast, one ""harbor"" at a time.";"{""tags"": [""sea"", ""maps""], ""helpful"": 12}";0x89504e47;2026-02-06 08:05:00+00
+11;5;Harbor Lights;"Lovely	second half
+and a pipe | here";;;2026-02-06 08:06:12.5+00
+12;;;Café Ünïcödé 東京, shelved under C:\fiction;"{""a"": {""b"": [1, 2.50]}, ""z"": null}";0x;2026-02-11 12:40:00+05:30
diff --git a/crates/tabletist-core/src/export/golden/json-default.json b/crates/tabletist-core/src/export/golden/json-default.json
new file mode 100644
--- /dev/null
+++ b/crates/tabletist-core/src/export/golden/json-default.json
@@ -0,0 +1,43 @@
+[
+  {
+    "id": 10,
+    "rating": 4,
+    "title": "The Tide Atlas",
+    "body": "Maps the coast, one \"harbor\" at a time.",
+    "metadata": {
+      "tags": [
+        "sea",
+        "maps"
+      ],
+      "helpful": 12
+    },
+    "cover": "0x89504e47",
+    "created_at": "2026-02-06T08:05:00+00:00"
+  },
+  {
+    "id": 11,
+    "rating": 5,
+    "title": "Harbor Lights",
+    "body": "Lovely\tsecond half\nand a pipe | here",
+    "metadata": null,
+    "cover": null,
+    "created_at": "2026-02-06T08:06:12.5+00:00"
+  },
+  {
+    "id": 12,
+    "rating": null,
+    "title": "",
+    "body": "Café Ünïcödé 東京, shelved under C:\\fiction",
+    "metadata": {
+      "a": {
+        "b": [
+          1,
+          2.50
+        ]
+      },
+      "z": null
+    },
+    "cover": "0x",
+    "created_at": "2026-02-11T12:40:00+05:30"
+  }
+]
diff --git a/crates/tabletist-core/src/export/golden/json-one-line.json b/crates/tabletist-core/src/export/golden/json-one-line.json
new file mode 100644
--- /dev/null
+++ b/crates/tabletist-core/src/export/golden/json-one-line.json
@@ -0,0 +1 @@
+[{"id":10,"rating":4,"title":"The Tide Atlas","body":"Maps the coast, one \"harbor\" at a time.","metadata":{"tags":["sea","maps"],"helpful":12},"cover":"0x89504e47","created_at":"2026-02-06T08:05:00+00:00"},{"id":11,"rating":5,"title":"Harbor Lights","body":"Lovely\tsecond half\nand a pipe | here","metadata":null,"cover":null,"created_at":"2026-02-06T08:06:12.5+00:00"},{"id":12,"rating":null,"title":"","body":"Café Ünïcödé 東京, shelved under C:\\fiction","metadata":{"a":{"b":[1,2.50]},"z":null},"cover":"0x","created_at":"2026-02-11T12:40:00+05:30"}]
diff --git a/crates/tabletist-core/src/export/golden/json-strings.json b/crates/tabletist-core/src/export/golden/json-strings.json
new file mode 100644
--- /dev/null
+++ b/crates/tabletist-core/src/export/golden/json-strings.json
@@ -0,0 +1,29 @@
+[
+  {
+    "id": 10,
+    "rating": 4,
+    "title": "The Tide Atlas",
+    "body": "Maps the coast, one \"harbor\" at a time.",
+    "metadata": "{\"tags\": [\"sea\", \"maps\"], \"helpful\": 12}",
+    "cover": "0x89504e47",
+    "created_at": "2026-02-06T08:05:00+00:00"
+  },
+  {
+    "id": 11,
+    "rating": 5,
+    "title": "Harbor Lights",
+    "body": "Lovely\tsecond half\nand a pipe | here",
+    "metadata": null,
+    "cover": null,
+    "created_at": "2026-02-06T08:06:12.5+00:00"
+  },
+  {
+    "id": 12,
+    "rating": null,
+    "title": "",
+    "body": "Café Ünïcödé 東京, shelved under C:\\fiction",
+    "metadata": "{\"a\": {\"b\": [1, 2.50]}, \"z\": null}",
+    "cover": "0x",
+    "created_at": "2026-02-11T12:40:00+05:30"
+  }
+]
diff --git a/crates/tabletist-core/src/export/golden/jsonl-default.jsonl b/crates/tabletist-core/src/export/golden/jsonl-default.jsonl
new file mode 100644
--- /dev/null
+++ b/crates/tabletist-core/src/export/golden/jsonl-default.jsonl
@@ -0,0 +1,3 @@
+{"id":10,"rating":4,"title":"The Tide Atlas","body":"Maps the coast, one \"harbor\" at a time.","metadata":{"tags":["sea","maps"],"helpful":12},"cover":"0x89504e47","created_at":"2026-02-06T08:05:00+00:00"}
+{"id":11,"rating":5,"title":"Harbor Lights","body":"Lovely\tsecond half\nand a pipe | here","metadata":null,"cover":null,"created_at":"2026-02-06T08:06:12.5+00:00"}
+{"id":12,"rating":null,"title":"","body":"Café Ünïcödé 東京, shelved under C:\\fiction","metadata":{"a":{"b":[1,2.50]},"z":null},"cover":"0x","created_at":"2026-02-11T12:40:00+05:30"}
diff --git a/crates/tabletist-core/src/export/golden/md-default.md b/crates/tabletist-core/src/export/golden/md-default.md
new file mode 100644
--- /dev/null
+++ b/crates/tabletist-core/src/export/golden/md-default.md
@@ -0,0 +1,5 @@
+| id | rating | title          | body                                       | metadata                                 | cover      | created_at                |
+|---:|-------:|----------------|--------------------------------------------|------------------------------------------|------------|---------------------------|
+| 10 |      4 | The Tide Atlas | Maps the coast, one "harbor" at a time.    | {"tags": ["sea", "maps"], "helpful": 12} | 0x89504e47 | 2026-02-06 08:05:00+00    |
+| 11 |      5 | Harbor Lights  | Lovely	second half<br>and a pipe \| here   | NULL                                     | NULL       | 2026-02-06 08:06:12.5+00  |
+| 12 |   NULL |                | Café Ünïcödé 東京, shelved under C:\\fiction | {"a": {"b": [1, 2.50]}, "z": null}       | 0x         | 2026-02-11 12:40:00+05:30 |
diff --git a/crates/tabletist-core/src/export/golden/md-null-empty.md b/crates/tabletist-core/src/export/golden/md-null-empty.md
new file mode 100644
--- /dev/null
+++ b/crates/tabletist-core/src/export/golden/md-null-empty.md
@@ -0,0 +1,5 @@
+| id | rating | title          | body                                       | metadata                                 | cover      | created_at                |
+|---:|-------:|----------------|--------------------------------------------|------------------------------------------|------------|---------------------------|
+| 10 |      4 | The Tide Atlas | Maps the coast, one "harbor" at a time.    | {"tags": ["sea", "maps"], "helpful": 12} | 0x89504e47 | 2026-02-06 08:05:00+00    |
+| 11 |      5 | Harbor Lights  | Lovely	second half<br>and a pipe \| here   |                                          |            | 2026-02-06 08:06:12.5+00  |
+| 12 |        |                | Café Ünïcödé 東京, shelved under C:\\fiction | {"a": {"b": [1, 2.50]}, "z": null}       | 0x         | 2026-02-11 12:40:00+05:30 |
diff --git a/crates/tabletist-core/src/export/golden/sql-create.sql b/crates/tabletist-core/src/export/golden/sql-create.sql
new file mode 100644
--- /dev/null
+++ b/crates/tabletist-core/src/export/golden/sql-create.sql
@@ -0,0 +1,9 @@
+CREATE TABLE public.reviews (
+  id bigint PRIMARY KEY
+);
+
+INSERT INTO public.reviews (id, rating, title, body, metadata, cover, created_at) VALUES
+  (10, 4, 'The Tide Atlas', 'Maps the coast, one "harbor" at a time.', '{"tags": ["sea", "maps"], "helpful": 12}', E'\\x89504e47', '2026-02-06 08:05:00+00'),
+  (11, 5, 'Harbor Lights', 'Lovely	second half
+and a pipe | here', NULL, NULL, '2026-02-06 08:06:12.5+00'),
+  (12, NULL, '', E'Café Ünïcödé 東京, shelved under C:\\fiction', '{"a": {"b": [1, 2.50]}, "z": null}', E'\\x', '2026-02-11 12:40:00+05:30');
diff --git a/crates/tabletist-core/src/export/golden/sql-default.sql b/crates/tabletist-core/src/export/golden/sql-default.sql
new file mode 100644
--- /dev/null
+++ b/crates/tabletist-core/src/export/golden/sql-default.sql
@@ -0,0 +1,5 @@
+INSERT INTO public.reviews (id, rating, title, body, metadata, cover, created_at) VALUES
+  (10, 4, 'The Tide Atlas', 'Maps the coast, one "harbor" at a time.', '{"tags": ["sea", "maps"], "helpful": 12}', E'\\x89504e47', '2026-02-06 08:05:00+00'),
+  (11, 5, 'Harbor Lights', 'Lovely	second half
+and a pipe | here', NULL, NULL, '2026-02-06 08:06:12.5+00'),
+  (12, NULL, '', E'Café Ünïcödé 東京, shelved under C:\\fiction', '{"a": {"b": [1, 2.50]}, "z": null}', E'\\x', '2026-02-11 12:40:00+05:30');
diff --git a/crates/tabletist-core/src/export/golden/sql-mysql.sql b/crates/tabletist-core/src/export/golden/sql-mysql.sql
new file mode 100644
--- /dev/null
+++ b/crates/tabletist-core/src/export/golden/sql-mysql.sql
@@ -0,0 +1,5 @@
+INSERT INTO public.reviews (id, rating, title, body, metadata, cover, created_at) VALUES
+  (10, 4, 'The Tide Atlas', 'Maps the coast, one "harbor" at a time.', '{"tags": ["sea", "maps"], "helpful": 12}', x'89504e47', '2026-02-06 08:05:00+00'),
+  (11, 5, 'Harbor Lights', 'Lovely	second half
+and a pipe | here', NULL, NULL, '2026-02-06 08:06:12.5+00'),
+  (12, NULL, '', 'Café Ünïcödé 東京, shelved under C:\\fiction', '{"a": {"b": [1, 2.50]}, "z": null}', x'', '2026-02-11 12:40:00+05:30');
diff --git a/crates/tabletist-core/src/export/golden/sql-one-per-insert.sql b/crates/tabletist-core/src/export/golden/sql-one-per-insert.sql
new file mode 100644
--- /dev/null
+++ b/crates/tabletist-core/src/export/golden/sql-one-per-insert.sql
@@ -0,0 +1,4 @@
+INSERT INTO public.reviews (id, rating, title, body, metadata, cover, created_at) VALUES (10, 4, 'The Tide Atlas', 'Maps the coast, one "harbor" at a time.', '{"tags": ["sea", "maps"], "helpful": 12}', E'\\x89504e47', '2026-02-06 08:05:00+00');
+INSERT INTO public.reviews (id, rating, title, body, metadata, cover, created_at) VALUES (11, 5, 'Harbor Lights', 'Lovely	second half
+and a pipe | here', NULL, NULL, '2026-02-06 08:06:12.5+00');
+INSERT INTO public.reviews (id, rating, title, body, metadata, cover, created_at) VALUES (12, NULL, '', E'Café Ünïcödé 東京, shelved under C:\\fiction', '{"a": {"b": [1, 2.50]}, "z": null}', E'\\x', '2026-02-11 12:40:00+05:30');
diff --git a/crates/tabletist-core/src/export/golden/sql-sqlite.sql b/crates/tabletist-core/src/export/golden/sql-sqlite.sql
new file mode 100644
--- /dev/null
+++ b/crates/tabletist-core/src/export/golden/sql-sqlite.sql
@@ -0,0 +1,5 @@
+INSERT INTO public.reviews (id, rating, title, body, metadata, cover, created_at) VALUES
+  (10, 4, 'The Tide Atlas', 'Maps the coast, one "harbor" at a time.', '{"tags": ["sea", "maps"], "helpful": 12}', x'89504e47', '2026-02-06 08:05:00+00'),
+  (11, 5, 'Harbor Lights', 'Lovely	second half
+and a pipe | here', NULL, NULL, '2026-02-06 08:06:12.5+00'),
+  (12, NULL, '', 'Café Ünïcödé 東京, shelved under C:\fiction', '{"a": {"b": [1, 2.50]}, "z": null}', x'', '2026-02-11 12:40:00+05:30');
diff --git a/crates/tabletist-core/src/export/golden/tsv-default.tsv b/crates/tabletist-core/src/export/golden/tsv-default.tsv
new file mode 100644
--- /dev/null
+++ b/crates/tabletist-core/src/export/golden/tsv-default.tsv
@@ -0,0 +1,4 @@
+id	rating	title	body	metadata	cover	created_at
+10	4	The Tide Atlas	Maps the coast, one "harbor" at a time.	{"tags": ["sea", "maps"], "helpful": 12}	0x89504e47	2026-02-06 08:05:00+00
+11	5	Harbor Lights	Lovely\tsecond half\nand a pipe | here			2026-02-06 08:06:12.5+00
+12			Café Ünïcödé 東京, shelved under C:\\fiction	{"a": {"b": [1, 2.50]}, "z": null}	0x	2026-02-11 12:40:00+05:30
diff --git a/crates/tabletist-core/src/export/golden/tsv-no-header.tsv b/crates/tabletist-core/src/export/golden/tsv-no-header.tsv
new file mode 100644
--- /dev/null
+++ b/crates/tabletist-core/src/export/golden/tsv-no-header.tsv
@@ -0,0 +1,3 @@
+10	4	The Tide Atlas	Maps the coast, one "harbor" at a time.	{"tags": ["sea", "maps"], "helpful": 12}	0x89504e47	2026-02-06 08:05:00+00
+11	5	Harbor Lights	Lovely\tsecond half\nand a pipe | here			2026-02-06 08:06:12.5+00
+12			Café Ünïcödé 東京, shelved under C:\\fiction	{"a": {"b": [1, 2.50]}, "z": null}	0x	2026-02-11 12:40:00+05:30
diff --git a/crates/tabletist-core/src/export/golden/tsv-null-text.tsv b/crates/tabletist-core/src/export/golden/tsv-null-text.tsv
new file mode 100644
--- /dev/null
+++ b/crates/tabletist-core/src/export/golden/tsv-null-text.tsv
@@ -0,0 +1,4 @@
+id	rating	title	body	metadata	cover	created_at
+10	4	The Tide Atlas	Maps the coast, one "harbor" at a time.	{"tags": ["sea", "maps"], "helpful": 12}	0x89504e47	2026-02-06 08:05:00+00
+11	5	Harbor Lights	Lovely\tsecond half\nand a pipe | here	\N	\N	2026-02-06 08:06:12.5+00
+12	\N		Café Ünïcödé 東京, shelved under C:\\fiction	{"a": {"b": [1, 2.50]}, "z": null}	0x	2026-02-11 12:40:00+05:30
diff --git a/crates/tabletist-core/src/lib.rs b/crates/tabletist-core/src/lib.rs
--- a/crates/tabletist-core/src/lib.rs
+++ b/crates/tabletist-core/src/lib.rs
@@ -14,6 +14,7 @@
 pub mod edit;
 pub mod endpoint;
 pub mod env;
+pub mod export;
 #[cfg(any(test, feature = "testing"))]
 pub mod fixtures;
 pub mod font_files;
diff --git a/crates/tabletist-db/src/dialect.rs b/crates/tabletist-db/src/dialect.rs
--- a/crates/tabletist-db/src/dialect.rs
+++ b/crates/tabletist-db/src/dialect.rs
@@ -1198,6 +1198,74 @@
         SortDir, Value,
     };
 
+    #[test]
+    fn a_pending_default_or_now_is_what_a_save_sends() {
+        let default = NewValue::Default {
+            expression: Some("'print'".into()),
+        };
+        let pg = Dialect::Postgres;
+        assert_eq!(
+            pg.pending_literal(&default, "text").as_deref(),
+            Some("DEFAULT")
+        );
+        assert_eq!(
+            Dialect::Sqlite.pending_literal(&default, "TEXT").as_deref(),
+            Some("('print')")
+        );
+        assert_eq!(
+            pg.pending_literal(&NewValue::Now, "timestamptz").as_deref(),
+            Some("now()")
+        );
+        assert_eq!(
+            pg.pending_literal(&NewValue::Now, "date").as_deref(),
+            Some("CURRENT_DATE")
+        );
+        assert_eq!(pg.pending_literal(&NewValue::Null, "text"), None);
+    }
+
+    #[test]
+    fn a_loaded_value_is_the_literal_a_save_would_write() {
+        use crate::ValueKind::{Numeric, Text};
+        let pg = Dialect::Postgres;
+        assert_eq!(pg.value_literal(&Value::Null, Text), "NULL");
+        assert_eq!(pg.value_literal(&Value::Bool(true), Text), "TRUE");
+        assert_eq!(Dialect::Sqlite.value_literal(&Value::Bool(true), Text), "1");
+        assert_eq!(pg.value_literal(&Value::Int(-4), Numeric), "-4");
+        assert_eq!(pg.value_literal(&Value::Float(4.5), Numeric), "4.5");
+        assert_eq!(pg.value_literal(&Value::Float(f64::NAN), Numeric), "'NaN'");
+        assert_eq!(
+            Dialect::Sqlite.value_literal(&Value::Float(f64::NAN), Numeric),
+            "NULL"
+        );
+        assert_eq!(
+            pg.value_literal(&Value::Text("12.50".into()), Numeric),
+            "12.50"
+        );
+        assert_eq!(
+            pg.value_literal(&Value::Text("$1.00".into()), Numeric),
+            "'$1.00'"
+        );
+        assert_eq!(
+            pg.value_literal(&Value::Text("12.50".into()), Text),
+            "'12.50'"
+        );
+        assert_eq!(
+            pg.value_literal(&Value::Text("it's".into()), Text),
+            "'it''s'"
+        );
+        assert_eq!(
+            pg.value_literal(&Value::Text("a\\b".into()), Text),
+            "E'a\\\\b'"
+        );
+        assert_eq!(
+            Dialect::MySql.value_literal(&Value::Text("a\\b".into()), Text),
+            "'a\\\\b'"
+        );
+        let bytes = Value::Bytes(vec![0x0a, 0xff].into());
+        assert_eq!(pg.value_literal(&bytes, Text), "E'\\\\x0aff'");
+        assert_eq!(Dialect::MySql.value_literal(&bytes, Text), "x'0aff'");
+    }
+
     fn query() -> RowQuery {
         RowQuery::new(ObjectRef::new("public", "users"), 300)
     }
diff --git a/crates/tabletist-db/src/value.rs b/crates/tabletist-db/src/value.rs
--- a/crates/tabletist-db/src/value.rs
+++ b/crates/tabletist-db/src/value.rs
@@ -135,6 +135,25 @@
     use super::*;
 
     #[test]
+    fn plain_numbers_are_told_from_other_text() {
+        for number in [
+            "0",
+            "-1",
+            "12.50",
+            "1e9",
+            "-0.5E-3",
+            "123456789012345678901234567890",
+        ] {
+            assert!(is_plain_number(number), "{number}");
+        }
+        for text in [
+            "", "-", "01", "1.", ".5", "1e", "NaN", "Infinity", "$1.00", "1 ", "+1",
+        ] {
+            assert!(!is_plain_number(text), "{text}");
+        }
+    }
+
+    #[test]
     fn sqlite_declared_types_map_to_kinds() {
         let cases = [
             ("INTEGER", ValueKind::Numeric),
diff --git a/src/lib.rs b/src/lib.rs
--- a/src/lib.rs
+++ b/src/lib.rs
@@ -23,7 +23,7 @@
 pub use tabletist_core::fixtures;
 pub use tabletist_core::{
     agents_shown, backend, clock, completion, connections, dates, desktop_theme, edit, endpoint,
-    env, font_files, format, history, i18n, json, key_picker, keymap, known_hosts, labels, managed,
-    model, paths, recents, relay, review, search, secrets, selection, session, settings, theme,
-    update, util, vim, zones,
+    env, export, font_files, format, history, i18n, json, key_picker, keymap, known_hosts, labels,
+    managed, model, paths, recents, relay, review, search, secrets, selection, session, settings,
+    theme, update, util, vim, zones,
 };
```

Two golden files hold bytes a page of Markdown should not: `csv-bom.csv` is `csv-default.csv` after a byte-order mark, and `csv-crlf.csv` the same text with each row ended by CRLF (the line break inside a quoted value stays LF). Write them from it:

```bash
cd crates/tabletist-core/src/export/golden
python3 -c "import sys; d=open('csv-default.csv','rb').read(); open('csv-bom.csv','wb').write(b'\xef\xbb\xbf'+d); open('csv-crlf.csv','wb').write(d.replace(b'created_at\n',b'created_at\r\n').replace(b'+00\n',b'+00\r\n').replace(b'+05:30\n',b'+05:30\r\n'))"
cd -
```

- [ ] **Step 2: Run them, and see them fail**

Run: `~/.cargo/bin/cargo test -p tabletist-core --lib export:: ; ~/.cargo/bin/cargo test -p tabletist-db --lib -- a_loaded_value a_pending_default plain_numbers`

Expected: both builds fail, naming what does not exist yet: `tabletist-core`'s tests with errors such as ``error[E0425]: cannot find type `Options` in this scope`` and ``cannot find function `clipboard` in this scope``, and `tabletist-db`'s with ``error[E0599]: no method named `value_literal` found for enum `dialect::Dialect` in the current scope``, the same for `pending_literal`, and ``cannot find function `is_plain_number` ``.

- [ ] **Step 3: Write the code**

```diff
diff --git a/crates/tabletist-core/src/backend.rs b/crates/tabletist-core/src/backend.rs
--- a/crates/tabletist-core/src/backend.rs
+++ b/crates/tabletist-core/src/backend.rs
@@ -264,6 +264,9 @@
     /// How each connection lists its tables where its user chose, by id:
     /// a list's name, or a value a newer version wrote, kept as it was.
     TableLists(std::collections::BTreeMap<String, serde_json::Value>),
+    /// The options last chosen for each format of an export, by the
+    /// format's key (`export::Options::to_file`).
+    ExportOptions(std::collections::BTreeMap<String, serde_json::Value>),
     /// The connection tabs open, and each connection's tables.
     Session(crate::session::Session),
     /// What was opened lately on each connection.
@@ -289,6 +292,7 @@
             Self::Labels(labels) => labels.save(path),
             Self::AgentsShown(shown) => shown.save(path),
             Self::TableLists(lists) => crate::util::save_json(path, lists),
+            Self::ExportOptions(options) => crate::util::save_json(path, options),
             Self::Session(session) => session.save(path),
             Self::Recents(recents) => recents.save(path),
             Self::Updates(seen) => crate::util::save_json(path, seen),
@@ -4842,6 +4846,24 @@
     }
 
     #[test]
+    fn export_options_are_saved_as_a_state_file() {
+        let dir = tempfile::tempdir().unwrap();
+        let path = dir.path().join("export_options.json");
+        let mut options = crate::export::Options::default();
+        options.md.long_values = crate::export::LongValues::Cut;
+        let mut backend = Backend::start_with(Waker::default(), Keyring::memory());
+        backend.send(Command::Save {
+            path: path.clone(),
+            file: StateFile::ExportOptions(options.to_file()),
+        });
+        match backend.wait(WAIT) {
+            Some(Event::Saved { result: Ok(()), .. }) => {}
+            other => panic!("expected the save to be written, got {other:?}"),
+        }
+        assert_eq!(crate::export::Options::load(&path), options);
+    }
+
+    #[test]
     fn a_state_file_saved_through_a_symbolic_link_keeps_the_link() {
         if !crate::util::can_symlink() {
             return;
diff --git a/crates/tabletist-core/src/export.rs b/crates/tabletist-core/src/export.rs
--- a/crates/tabletist-core/src/export.rs
+++ b/crates/tabletist-core/src/export.rs
@@ -10,6 +10,1075 @@
 //! The options are kept in the state directory
 //! (`AppDirs::export_options_file`), by format. What the file holds that
 //! this version cannot read is kept and written back.
+
+use std::borrow::Cow;
+use std::collections::BTreeMap;
+use std::fmt::Write as _;
+use std::path::Path;
+
+use serde::de::DeserializeOwned;
+use serde::{Deserialize, Serialize};
+use tabletist_db::{Dialect, ObjectRef, Value, ValueKind};
+
+/// The most rows the clipboard takes; more go to a file.
+pub const CLIPBOARD_ROWS: usize = 10_000;
+/// The most text the clipboard takes, in bytes: 20 MB.
+pub const CLIPBOARD_BYTES: usize = 20 * 1024 * 1024;
+
+/// A format an export writes.
+#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
+pub enum Format {
+    Csv,
+    Tsv,
+    Json,
+    JsonLines,
+    SqlInsert,
+    Markdown,
+}
+
+impl Format {
+    pub const ALL: [Format; 6] = [
+        Format::Csv,
+        Format::Tsv,
+        Format::Json,
+        Format::JsonLines,
+        Format::SqlInsert,
+        Format::Markdown,
+    ];
+
+    /// Its key in the file of options.
+    pub fn key(self) -> &'static str {
+        match self {
+            Self::Csv => "csv",
+            Self::Tsv => "tsv",
+            Self::Json => "json",
+            Self::JsonLines => "jsonl",
+            Self::SqlInsert => "sql",
+            Self::Markdown => "md",
+        }
+    }
+
+    /// Its name, as the menus write it.
+    pub fn name(self) -> &'static str {
+        match self {
+            Self::Csv => "CSV",
+            Self::Tsv => "TSV",
+            Self::Json => "JSON",
+            Self::JsonLines => "JSON Lines",
+            Self::SqlInsert => "SQL INSERT",
+            Self::Markdown => "Markdown",
+        }
+    }
+}
+
+/// What a NULL is written as.
+#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
+#[serde(rename_all = "snake_case")]
+pub enum NullAs {
+    Empty,
+    /// The option's text: `\N`, `NULL`.
+    Text,
+}
+
+/// How bytes are written, where no SQL literal is.
+#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
+#[serde(rename_all = "snake_case")]
+pub enum Binary {
+    /// `0x` and two digits a byte.
+    Hex,
+    Base64,
+}
+
+#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
+#[serde(rename_all = "snake_case")]
+pub enum Quoting {
+    /// A value that holds the delimiter, a quote or a line break.
+    WhenNeeded,
+    /// Every value but a NULL written empty, so a NULL and `""` differ.
+    Every,
+}
+
+#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
+#[serde(rename_all = "snake_case")]
+pub enum LineEnding {
+    Lf,
+    Crlf,
+}
+
+impl LineEnding {
+    fn text(self) -> &'static str {
+        match self {
+            Self::Lf => "\n",
+            Self::Crlf => "\r\n",
+        }
+    }
+}
+
+/// How a json or jsonb value is written in JSON.
+#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
+#[serde(rename_all = "snake_case")]
+pub enum JsonValues {
+    /// As JSON, in the document.
+    Nested,
+    /// As a string holding its text.
+    Strings,
+}
+
+#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
+#[serde(rename_all = "snake_case")]
+pub enum Layout {
+    /// Two spaces deeper with each level.
+    Indented,
+    OneLine,
+}
+
+#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
+#[serde(rename_all = "snake_case")]
+pub enum LongValues {
+    Full,
+    /// At [`MARKDOWN_CUT`] characters, then `…`.
+    Cut,
+}
+
+/// Where Markdown cuts a long value.
+pub const MARKDOWN_CUT: usize = 200;
+/// The widest a Markdown column is padded to; a longer value runs past.
+const MARKDOWN_PAD: usize = 80;
+
+#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
+pub struct Csv {
+    pub header: bool,
+    pub delimiter: char,
+    pub quoting: Quoting,
+    pub null_as: NullAs,
+    pub null_text: String,
+    pub bom: bool,
+    pub line_ending: LineEnding,
+    pub binary: Binary,
+}
+
+impl Default for Csv {
+    fn default() -> Self {
+        Self {
+            header: true,
+            delimiter: ',',
+            quoting: Quoting::WhenNeeded,
+            null_as: NullAs::Empty,
+            null_text: "\\N".into(),
+            bom: false,
+            line_ending: LineEnding::Lf,
+            binary: Binary::Hex,
+        }
+    }
+}
+
+#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
+pub struct Tsv {
+    pub header: bool,
+    pub null_as: NullAs,
+    pub null_text: String,
+    pub bom: bool,
+    pub line_ending: LineEnding,
+    pub binary: Binary,
+}
+
+impl Default for Tsv {
+    fn default() -> Self {
+        Self {
+            header: true,
+            null_as: NullAs::Empty,
+            null_text: "\\N".into(),
+            bom: false,
+            line_ending: LineEnding::Lf,
+            binary: Binary::Hex,
+        }
+    }
+}
+
+#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
+pub struct Json {
+    pub values: JsonValues,
+    pub layout: Layout,
+    pub binary: Binary,
+}
+
+impl Default for Json {
+    fn default() -> Self {
+        Self {
+            values: JsonValues::Nested,
+            layout: Layout::Indented,
+            binary: Binary::Hex,
+        }
+    }
+}
+
+#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
+pub struct JsonLines {
+    pub values: JsonValues,
+    pub binary: Binary,
+}
+
+impl Default for JsonLines {
+    fn default() -> Self {
+        Self {
+            values: JsonValues::Nested,
+            binary: Binary::Hex,
+        }
+    }
+}
+
+#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
+pub struct Sql {
+    /// 1, 100 or 1,000.
+    pub rows_per_insert: usize,
+    /// Start with the table's `CREATE TABLE`, from the DDL view.
+    pub create_table: bool,
+}
+
+impl Default for Sql {
+    fn default() -> Self {
+        Self {
+            rows_per_insert: 100,
+            create_table: false,
+        }
+    }
+}
+
+#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
+pub struct Markdown {
+    pub null_as: NullAs,
+    pub null_text: String,
+    pub long_values: LongValues,
+}
+
+impl Default for Markdown {
+    fn default() -> Self {
+        Self {
+            null_as: NullAs::Text,
+            null_text: "NULL".into(),
+            long_values: LongValues::Full,
+        }
+    }
+}
+
+/// Every format's options, as last chosen.
+#[derive(Debug, Clone, PartialEq, Default)]
+pub struct Options {
+    pub csv: Csv,
+    pub tsv: Tsv,
+    pub json: Json,
+    pub jsonl: JsonLines,
+    pub sql: Sql,
+    pub md: Markdown,
+    /// What the file held of each format that this version could not
+    /// read (an option it does not know, a value it cannot read), by the
+    /// format's key: written back over the options, as the file had it.
+    kept: BTreeMap<String, serde_json::Map<String, serde_json::Value>>,
+    /// The file's other keys, written back as they were.
+    rest: BTreeMap<String, serde_json::Value>,
+}
+
+/// What a format's entry held that this version could not read.
+type Unread = serde_json::Map<String, serde_json::Value>;
+
+impl Options {
+    /// The options in `path`: the defaults where it is missing or does not
+    /// read.
+    pub fn load(path: &Path) -> Self {
+        Self::from_file(crate::util::load_json(path))
+    }
+
+    /// The options a file of them holds.
+    pub fn from_file(mut file: BTreeMap<String, serde_json::Value>) -> Self {
+        let mut kept = BTreeMap::new();
+        let csv = take(&mut file, Format::Csv, &mut kept);
+        let tsv = take(&mut file, Format::Tsv, &mut kept);
+        let json = take(&mut file, Format::Json, &mut kept);
+        let jsonl = take(&mut file, Format::JsonLines, &mut kept);
+        let sql = take(&mut file, Format::SqlInsert, &mut kept);
+        let md = take(&mut file, Format::Markdown, &mut kept);
+        Self {
+            csv,
+            tsv,
+            json,
+            jsonl,
+            sql,
+            md,
+            kept,
+            rest: file,
+        }
+    }
+
+    /// The file of these options.
+    pub fn to_file(&self) -> BTreeMap<String, serde_json::Value> {
+        let mut file = self.rest.clone();
+        let kept = &self.kept;
+        put(&mut file, Format::Csv, &self.csv, kept);
+        put(&mut file, Format::Tsv, &self.tsv, kept);
+        put(&mut file, Format::Json, &self.json, kept);
+        put(&mut file, Format::JsonLines, &self.jsonl, kept);
+        put(&mut file, Format::SqlInsert, &self.sql, kept);
+        put(&mut file, Format::Markdown, &self.md, kept);
+        file
+    }
+
+    /// `format`'s options were chosen again: what was kept of them unread
+    /// gives way to them, and only an option this version does not know
+    /// is still written back.
+    pub fn chosen(&mut self, format: Format) {
+        let known = match format {
+            Format::Csv => keys(&self.csv),
+            Format::Tsv => keys(&self.tsv),
+            Format::Json => keys(&self.json),
+            Format::JsonLines => keys(&self.jsonl),
+            Format::SqlInsert => keys(&self.sql),
+            Format::Markdown => keys(&self.md),
+        };
+        if let Some(unread) = self.kept.get_mut(format.key()) {
+            unread.retain(|key, _| !known.contains(key));
+        }
+        self.rest.remove(format.key());
+    }
+}
+
+/// The names of the options `options` writes.
+fn keys<T: Serialize>(options: &T) -> Vec<String> {
+    match serde_json::to_value(options) {
+        Ok(serde_json::Value::Object(entry)) => entry.keys().cloned().collect(),
+        _ => Vec::new(),
+    }
+}
+
+/// `format`'s options from `file`: each option the entry holds that reads,
+/// the default for the others. What does not read is put in `kept`.
+fn take<T: Serialize + DeserializeOwned + Default>(
+    file: &mut BTreeMap<String, serde_json::Value>,
+    format: Format,
+    kept: &mut BTreeMap<String, Unread>,
+) -> T {
+    let Ok(serde_json::Value::Object(mut merged)) = serde_json::to_value(T::default()) else {
+        return T::default();
+    };
+    let entry = match file.remove(format.key()) {
+        Some(serde_json::Value::Object(entry)) => entry,
+        // An entry this version cannot read at all stays in the file, as
+        // its other keys do, until the format's options are chosen again.
+        Some(other) => {
+            file.insert(format.key().to_owned(), other);
+            return T::default();
+        }
+        None => return T::default(),
+    };
+    let mut unread = Unread::new();
+    for (key, value) in entry {
+        let Some(before) = merged.insert(key.clone(), value.clone()) else {
+            // An option this version does not know.
+            merged.remove(&key);
+            unread.insert(key, value);
+            continue;
+        };
+        let reads = serde_json::from_value::<T>(serde_json::Value::Object(merged.clone())).is_ok();
+        if !reads {
+            merged.insert(key.clone(), before);
+            unread.insert(key, value);
+        }
+    }
+    if !unread.is_empty() {
+        kept.insert(format.key().to_owned(), unread);
+    }
+    serde_json::from_value(serde_json::Value::Object(merged)).unwrap_or_default()
+}
+
+/// Writes `options` into `file` as `format`'s entry, with what was kept of
+/// it over them.
+fn put<T: Serialize>(
+    file: &mut BTreeMap<String, serde_json::Value>,
+    format: Format,
+    options: &T,
+    kept: &BTreeMap<String, Unread>,
+) {
+    let mut entry = match serde_json::to_value(options) {
+        Ok(serde_json::Value::Object(entry)) => entry,
+        _ => Unread::new(),
+    };
+    if let Some(unread) = kept.get(format.key()) {
+        entry.extend(unread.clone());
+    }
+    // Not over an entry kept as the file had it.
+    file.entry(format.key().to_owned())
+        .or_insert(serde_json::Value::Object(entry));
+}
+
+/// The columns a writer writes, in the order written, and the dialect
+/// that writes their names and SQL INSERT's literals.
+#[derive(Debug, Clone)]
+pub struct Columns<'a> {
+    pub names: Vec<&'a str>,
+    pub kinds: Vec<ValueKind>,
+    pub dialect: Dialect,
+}
+
+/// The table SQL INSERT writes into.
+#[derive(Debug, Clone, Copy)]
+pub enum Target<'a> {
+    /// The table the rows are of, schema-qualified, and its `CREATE
+    /// TABLE` to start with, where the options ask for it.
+    Table {
+        object: &'a ObjectRef,
+        create: Option<&'a str>,
+    },
+    /// A name of no table: `result`, for a SQL result's rows.
+    Named(&'a str),
+}
+
+/// A cell a writer reads: `None` is a new row's cell that nothing is set
+/// in, written as a NULL is.
+pub trait Cell {
+    fn value(&self) -> Option<&Value>;
+
+    /// What SQL INSERT writes in its place, where that is no literal of its
+    /// value: a pending DEFAULT, or the time of the save.
+    fn sql(&self) -> Option<&str> {
+        None
+    }
+}
+
+impl Cell for Value {
+    fn value(&self) -> Option<&Value> {
+        Some(self)
+    }
+}
+
+impl Cell for Option<Value> {
+    fn value(&self) -> Option<&Value> {
+        self.as_ref()
+    }
+}
+
+/// A cell of a grid with its pending changes: the value it shows, and for
+/// a pending DEFAULT or time of the save what a save would send.
+#[derive(Debug, Clone, Default, PartialEq)]
+pub struct Pending {
+    pub value: Option<Value>,
+    pub sql: Option<String>,
+}
+
+impl Cell for Pending {
+    fn value(&self) -> Option<&Value> {
+        self.value.as_ref()
+    }
+
+    fn sql(&self) -> Option<&str> {
+        self.sql.as_deref()
+    }
+}
+
+/// Writes one format's text, a row at a time: [`Writer::start`],
+/// [`Writer::row`] for each row, then [`Writer::finish`].
+pub struct Writer<'a> {
+    format: Format,
+    options: &'a Options,
+    columns: Columns<'a>,
+    target: Target<'a>,
+    rows: usize,
+    statements: usize,
+    /// Markdown's cells, as written, kept to be padded at the end.
+    kept: Vec<Vec<String>>,
+    /// How much text those cells hold.
+    kept_bytes: usize,
+}
+
+impl<'a> Writer<'a> {
+    pub fn new(
+        format: Format,
+        options: &'a Options,
+        columns: Columns<'a>,
+        target: Target<'a>,
+    ) -> Self {
+        Self {
+            format,
+            options,
+            columns,
+            target,
+            rows: 0,
+            statements: 0,
+            kept: Vec::new(),
+            kept_bytes: 0,
+        }
+    }
+
+    /// How much text the rows kept for the end hold: Markdown's, which
+    /// [`Writer::finish`] writes.
+    pub fn held(&self) -> usize {
+        self.kept_bytes
+    }
+
+    /// How many INSERT statements were written.
+    pub fn statements(&self) -> usize {
+        self.statements
+    }
+
+    /// What comes before the rows: a byte-order mark, a header row, an
+    /// array's opening, a `CREATE TABLE`.
+    pub fn start(&mut self, out: &mut String) {
+        match self.format {
+            Format::Csv => {
+                let csv = &self.options.csv;
+                if csv.bom {
+                    out.push('\u{feff}');
+                }
+                if csv.header {
+                    let names = self.columns.names.iter();
+                    let cells: Vec<String> = names
+                        .map(|name| csv_quoted(name, csv, csv.quoting == Quoting::Every))
+                        .collect();
+                    line(out, &cells, &csv.delimiter.to_string(), csv.line_ending);
+                }
+            }
+            Format::Tsv => {
+                let tsv = &self.options.tsv;
+                if tsv.bom {
+                    out.push('\u{feff}');
+                }
+                if tsv.header {
+                    let names = self.columns.names.iter();
+                    let cells: Vec<String> = names.map(|name| tsv_escaped(name)).collect();
+                    line(out, &cells, "\t", tsv.line_ending);
+                }
+            }
+            Format::Json => out.push('['),
+            Format::SqlInsert => {
+                let create = match self.target {
+                    Target::Table { create, .. } if self.options.sql.create_table => create,
+                    _ => None,
+                };
+                if let Some(create) = create.map(str::trim_end).filter(|text| !text.is_empty()) {
+                    out.push_str(create);
+                    if !create.ends_with(';') {
+                        out.push(';');
+                    }
+                    out.push_str("\n\n");
+                }
+            }
+            Format::JsonLines | Format::Markdown => {}
+        }
+    }
+
+    /// One row, its cells in the columns' order.
+    pub fn row<C: Cell>(&mut self, cells: &[C], out: &mut String) {
+        let values = || cells.iter().map(Cell::value);
+        match self.format {
+            Format::Csv => {
+                let csv = &self.options.csv;
+                let written: Vec<String> = values().map(|value| csv_cell(value, csv)).collect();
+                line(out, &written, &csv.delimiter.to_string(), csv.line_ending);
+            }
+            Format::Tsv => {
+                let tsv = &self.options.tsv;
+                let written: Vec<String> = values().map(|value| tsv_cell(value, tsv)).collect();
+                line(out, &written, "\t", tsv.line_ending);
+            }
+            Format::Json => {
+                let json = &self.options.json;
+                if self.rows > 0 {
+                    out.push(',');
+                }
+                let indented = json.layout == Layout::Indented;
+                if indented {
+                    out.push_str("\n  ");
+                }
+                let depth = usize::from(indented);
+                self.object(values(), json.values, json.binary, indented, depth, out);
+            }
+            Format::JsonLines => {
+                let jsonl = &self.options.jsonl;
+                self.object(values(), jsonl.values, jsonl.binary, false, 0, out);
+                out.push('\n');
+            }
+            Format::SqlInsert => self.insert(cells, out),
+            Format::Markdown => {
+                let md = &self.options.md;
+                let written: Vec<String> = values().map(|value| markdown_cell(value, md)).collect();
+                // A cell, its padding aside, and the ` | ` after it.
+                self.kept_bytes += written.iter().map(|cell| cell.len() + 3).sum::<usize>();
+                self.kept.push(written);
+            }
+        }
+        self.rows += 1;
+    }
+
+    /// What comes after the rows: an array's end, the last statement's
+    /// `;`, a Markdown table whole.
+    pub fn finish(&mut self, out: &mut String) {
+        match self.format {
+            Format::Json => {
+                if self.rows > 0 && self.options.json.layout == Layout::Indented {
+                    out.push('\n');
+                }
+                out.push_str("]\n");
+            }
+            Format::SqlInsert if self.rows > 0 => out.push_str(";\n"),
+            Format::Markdown => self.markdown(out),
+            _ => {}
+        }
+    }
+
+    /// A row as a JSON object, keys in the columns' order. Indented, its
+    /// members are a line each, `depth` levels in.
+    fn object<'v>(
+        &self,
+        values: impl Iterator<Item = Option<&'v Value>>,
+        nested: JsonValues,
+        binary: Binary,
+        indented: bool,
+        depth: usize,
+        out: &mut String,
+    ) {
+        let inner = "  ".repeat(depth + 1);
+        out.push('{');
+        let mut any = false;
+        for (col, value) in values.enumerate() {
+            if any {
+                out.push(',');
+            }
+            any = true;
+            if indented {
+                out.push('\n');
+                out.push_str(&inner);
+            }
+            let name = self.columns.names.get(col).copied().unwrap_or_default();
+            out.push_str(&json_string(name));
+            out.push(':');
+            if indented {
+                out.push(' ');
+            }
+            let kind = self
+                .columns
+                .kinds
+                .get(col)
+                .copied()
+                .unwrap_or(ValueKind::Other);
+            let written = json_value(value, kind, nested, binary, indented);
+            match (indented, kind == ValueKind::Json) {
+                // A nested document's later lines, as deep as its member.
+                (true, true) => out.push_str(&written.replace('\n', &format!("\n{inner}"))),
+                _ => out.push_str(&written),
+            }
+        }
+        if indented && any {
+            out.push('\n');
+            out.push_str(&"  ".repeat(depth));
+        }
+        out.push('}');
+    }
+
+    /// A row of an INSERT: the statement's head on the first of each run
+    /// of `rows_per_insert` rows.
+    fn insert<C: Cell>(&mut self, cells: &[C], out: &mut String) {
+        let dialect = self.columns.dialect;
+        let per = self.options.sql.rows_per_insert.max(1);
+        let tuple: Vec<String> = cells
+            .iter()
+            .enumerate()
+            .map(|(col, cell)| match cell.sql() {
+                Some(sql) => sql.to_owned(),
+                None => {
+                    let kinds = self.columns.kinds.get(col).copied();
+                    let kind = kinds.unwrap_or(ValueKind::Other);
+                    dialect.value_literal(cell.value().unwrap_or(&Value::Null), kind)
+                }
+            })
+            .collect();
+        let tuple = format!("({})", tuple.join(", "));
+        if self.rows.is_multiple_of(per) {
+            if self.rows > 0 {
+                out.push_str(";\n");
+            }
+            let table = match self.target {
+                Target::Table { object, .. } => {
+                    format!(
+                        "{}.{}",
+                        dialect.ident(&object.schema),
+                        dialect.ident(&object.name)
+                    )
+                }
+                Target::Named(name) => dialect.ident(name).into_owned(),
+            };
+            let names: Vec<_> = self
+                .columns
+                .names
+                .iter()
+                .map(|name| dialect.ident(name))
+                .collect();
+            let _ = write!(out, "INSERT INTO {table} ({}) VALUES", names.join(", "));
+            out.push_str(if per == 1 { " " } else { "\n  " });
+            self.statements += 1;
+        } else {
+            out.push_str(",\n  ");
+        }
+        out.push_str(&tuple);
+    }
+
+    /// The Markdown table: each column padded to its widest cell (to
+    /// [`MARKDOWN_PAD`] characters), numbers to the right.
+    fn markdown(&mut self, out: &mut String) {
+        let header: Vec<String> = self
+            .columns
+            .names
+            .iter()
+            .map(|name| markdown_escaped(name))
+            .collect();
+        let count = |text: &str| text.chars().count();
+        let widths: Vec<usize> = (0..header.len())
+            .map(|col| {
+                let cells = self.kept.iter().filter_map(|row| row.get(col));
+                let widest = cells
+                    .map(|cell| count(cell))
+                    .chain([count(&header[col])])
+                    .max();
+                widest.unwrap_or(0).clamp(1, MARKDOWN_PAD)
+            })
+            .collect();
+        let right = |col: usize| self.columns.kinds.get(col) == Some(&ValueKind::Numeric);
+        let row = |out: &mut String, cells: &[String]| {
+            out.push('|');
+            for (col, width) in widths.iter().enumerate() {
+                let cell = cells.get(col).map_or("", String::as_str);
+                let pad = " ".repeat(width.saturating_sub(count(cell)));
+                if right(col) {
+                    let _ = write!(out, " {pad}{cell} |");
+                } else {
+                    let _ = write!(out, " {cell}{pad} |");
+                }
+            }
+            out.push('\n');
+        };
+        row(out, &header);
+        out.push('|');
+        for (col, width) in widths.iter().enumerate() {
+            if right(col) {
+                let _ = write!(out, "{}:|", "-".repeat(width + 1));
+            } else {
+                let _ = write!(out, "{}|", "-".repeat(width + 2));
+            }
+        }
+        out.push('\n');
+        for cells in std::mem::take(&mut self.kept) {
+            row(out, &cells);
+        }
+    }
+}
+
+/// `cells` as one line, ended as the options say.
+fn line(out: &mut String, cells: &[String], delimiter: &str, ending: LineEnding) {
+    out.push_str(&cells.join(delimiter));
+    out.push_str(ending.text());
+}
+
+/// A value as plain text: what CSV, TSV and Markdown write of it.
+fn plain(value: &Value, binary: Binary) -> Cow<'_, str> {
+    match value {
+        Value::Null => Cow::Borrowed(""),
+        Value::Bool(flag) => Cow::Borrowed(if *flag { "true" } else { "false" }),
+        Value::Int(number) => Cow::Owned(number.to_string()),
+        Value::Float(number) => Cow::Owned(crate::format::float_text(*number)),
+        Value::Text(text) => Cow::Borrowed(text),
+        Value::Bytes(bytes) => Cow::Owned(binary_text(bytes, binary)),
+    }
+}
+
+/// Bytes as `0x` hex, or as Base64.
+pub fn binary_text(bytes: &[u8], binary: Binary) -> String {
+    match binary {
+        Binary::Hex => {
+            let mut hex = String::with_capacity(2 + bytes.len() * 2);
+            hex.push_str("0x");
+            for byte in bytes {
+                let _ = write!(hex, "{byte:02x}");
+            }
+            hex
+        }
+        Binary::Base64 => base64(bytes),
+    }
+}
+
+/// Standard Base64, with its padding.
+fn base64(bytes: &[u8]) -> String {
+    const LETTERS: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
+    let mut text = String::with_capacity(bytes.len().div_ceil(3) * 4);
+    for chunk in bytes.chunks(3) {
+        let at = |index: usize| chunk.get(index).copied().map_or(0, u32::from);
+        let triple = (at(0) << 16) | (at(1) << 8) | at(2);
+        for (index, shift) in [18, 12, 6, 0].into_iter().enumerate() {
+            if index <= chunk.len() {
+                text.push(char::from(LETTERS[((triple >> shift) & 63) as usize]));
+            } else {
+                text.push('=');
+            }
+        }
+    }
+    text
+}
+
+fn csv_cell(value: Option<&Value>, csv: &Csv) -> String {
+    match value {
+        None | Some(Value::Null) => match csv.null_as {
+            NullAs::Empty => String::new(),
+            // Never quoted unless it must be: a quoted one reads as text.
+            NullAs::Text => csv_quoted(&csv.null_text, csv, false),
+        },
+        Some(value) => csv_quoted(
+            &plain(value, csv.binary),
+            csv,
+            csv.quoting == Quoting::Every,
+        ),
+    }
+}
+
+/// `text` in quotes where `always` says or it holds the delimiter, a quote
+/// or a line break, its quotes doubled.
+fn csv_quoted(text: &str, csv: &Csv, always: bool) -> String {
+    let needs = text.contains([csv.delimiter, '"', '\n', '\r']);
+    if always || needs {
+        format!("\"{}\"", text.replace('"', "\"\""))
+    } else {
+        text.to_owned()
+    }
+}
+
+fn tsv_cell(value: Option<&Value>, tsv: &Tsv) -> String {
+    match value {
+        None | Some(Value::Null) => match tsv.null_as {
+            NullAs::Empty => String::new(),
+            NullAs::Text => tsv.null_text.clone(),
+        },
+        Some(value) => tsv_escaped(&plain(value, tsv.binary)),
+    }
+}
+
+/// `text` with a backslash, a tab and a line break written as PostgreSQL's
+/// `COPY` writes them: `\\`, `\t`, `\n`, `\r`.
+fn tsv_escaped(text: &str) -> String {
+    let mut escaped = String::with_capacity(text.len());
+    for character in text.chars() {
+        match character {
+            '\\' => escaped.push_str("\\\\"),
+            '\t' => escaped.push_str("\\t"),
+            '\n' => escaped.push_str("\\n"),
+            '\r' => escaped.push_str("\\r"),
+            other => escaped.push(other),
+        }
+    }
+    escaped
+}
+
+fn markdown_cell(value: Option<&Value>, md: &Markdown) -> String {
+    let text = match value {
+        None | Some(Value::Null) => match md.null_as {
+            NullAs::Empty => return String::new(),
+            NullAs::Text => Cow::Borrowed(md.null_text.as_str()),
+        },
+        Some(value) => plain(value, Binary::Hex),
+    };
+    match md.long_values {
+        LongValues::Cut if text.chars().count() > MARKDOWN_CUT => {
+            let cut: String = text.chars().take(MARKDOWN_CUT).collect();
+            markdown_escaped(&format!("{cut}…"))
+        }
+        _ => markdown_escaped(&text),
+    }
+}
+
+/// `text` on one line of a table: a backslash as `\\`, a pipe as `\|`, a
+/// line break as `<br>`.
+fn markdown_escaped(text: &str) -> String {
+    text.replace('\\', "\\\\")
+        .replace("\r\n", "<br>")
+        .replace(['\n', '\r'], "<br>")
+        .replace('|', "\\|")
+}
+
+/// `text` as a JSON string.
+fn json_string(text: &str) -> String {
+    serde_json::to_string(text).unwrap_or_else(|_| "\"\"".to_owned())
+}
+
+/// A value as JSON, by its column's kind.
+/// A nested document is laid out a member to a line where `indented`, on
+/// one line otherwise.
+fn json_value(
+    value: Option<&Value>,
+    kind: ValueKind,
+    nested: JsonValues,
+    binary: Binary,
+    indented: bool,
+) -> String {
+    let Some(value) = value else {
+        return "null".to_owned();
+    };
+    match value {
+        Value::Null => "null".to_owned(),
+        Value::Bool(flag) => flag.to_string(),
+        Value::Int(number) => number.to_string(),
+        Value::Float(number) if number.is_finite() => crate::format::float_text(*number),
+        Value::Float(number) => json_string(&crate::format::float_text(*number)),
+        Value::Bytes(bytes) => json_string(&binary_text(bytes, binary)),
+        Value::Text(text) => match kind {
+            ValueKind::Json if nested == JsonValues::Nested && is_json(text) => {
+                if indented {
+                    crate::json::pretty(text)
+                } else {
+                    compact(text)
+                }
+            }
+            ValueKind::Numeric if tabletist_db::is_plain_number(text) => text.to_string(),
+            ValueKind::Temporal => json_string(&iso_8601(text)),
+            _ => json_string(text),
+        },
+    }
+}
+
+/// Whether `text` is one JSON document.
+fn is_json(text: &str) -> bool {
+    serde_json::from_str::<serde::de::IgnoredAny>(text).is_ok()
+}
+
+/// A JSON document on one line: the white space between its pieces gone.
+fn compact(text: &str) -> String {
+    crate::json::tokens(text)
+        .into_iter()
+        .filter(|(kind, _)| *kind != crate::json::Kind::Space)
+        .map(|(_, range)| &text[range])
+        .collect()
+}
+
+/// A date and time as ISO 8601: `T` between the two, and an offset as
+/// `+HH:MM`. What is not a date and a time (a date, a time, an interval,
+/// `infinity`) is left as it is.
+pub fn iso_8601(text: &str) -> Cow<'_, str> {
+    let bytes = text.as_bytes();
+    let digits = |range: std::ops::Range<usize>| {
+        bytes
+            .get(range)
+            .is_some_and(|part| part.iter().all(u8::is_ascii_digit))
+    };
+    let date_time = bytes.len() >= 19
+        && digits(0..4)
+        && bytes[4] == b'-'
+        && digits(5..7)
+        && bytes[7] == b'-'
+        && digits(8..10)
+        && matches!(bytes[10], b' ' | b'T')
+        && digits(11..13)
+        && bytes[13] == b':'
+        && digits(14..16)
+        && bytes[16] == b':'
+        && digits(17..19);
+    if !date_time {
+        return Cow::Borrowed(text);
+    }
+    let mut at = 19;
+    if bytes.get(at) == Some(&b'.') {
+        at += 1;
+        while bytes.get(at).is_some_and(u8::is_ascii_digit) {
+            at += 1;
+        }
+    }
+    let (moment, offset) = text.split_at(at);
+    let offset = match offset.as_bytes() {
+        [] => String::new(),
+        [b'Z'] => "Z".to_owned(),
+        // `+05`: an hour alone, as PostgreSQL writes a whole hour.
+        [sign @ (b'+' | b'-'), h1, h2] if h1.is_ascii_digit() && h2.is_ascii_digit() => {
+            format!(
+                "{}{}{}:00",
+                char::from(*sign),
+                char::from(*h1),
+                char::from(*h2)
+            )
+        }
+        // `+05:30`, as it should be.
+        [b'+' | b'-', h1, h2, b':', m1, m2]
+            if [h1, h2, m1, m2].iter().all(|digit| digit.is_ascii_digit()) =>
+        {
+            offset.to_owned()
+        }
+        // `+0530`: the colon put in.
+        [sign @ (b'+' | b'-'), h1, h2, m1, m2]
+            if [h1, h2, m1, m2].iter().all(|digit| digit.is_ascii_digit()) =>
+        {
+            let [sign, h1, h2, m1, m2] = [sign, h1, h2, m1, m2].map(|byte| char::from(*byte));
+            format!("{sign}{h1}{h2}:{m1}{m2}")
+        }
+        // Seconds in the offset, a BC year, anything else after the time:
+        // left as it was, for no ISO 8601 writes it.
+        _ => return Cow::Borrowed(text),
+    };
+    Cow::Owned(format!("{}T{}{offset}", &moment[..10], &moment[11..]))
+}
+
+/// What a copy puts on the clipboard, or why it puts nothing.
+#[derive(Debug, Clone, PartialEq, Eq)]
+pub enum Copied {
+    Text {
+        text: String,
+        /// SQL INSERT's statements.
+        statements: usize,
+    },
+    /// More rows than [`CLIPBOARD_ROWS`].
+    TooManyRows(usize),
+    /// More text than [`CLIPBOARD_BYTES`].
+    TooLarge,
+}
+
+/// `rows` (`count` of them) in `format`, for the clipboard: no byte-order
+/// mark, which a file is for, and no line ending after the last line.
+/// Nothing is written past the clipboard's limits.
+pub fn clipboard<C: Cell>(
+    format: Format,
+    options: &Options,
+    columns: Columns<'_>,
+    target: Target<'_>,
+    count: usize,
+    rows: impl Iterator<Item = Vec<C>>,
+) -> Copied {
+    if count > CLIPBOARD_ROWS {
+        return Copied::TooManyRows(count);
+    }
+    let mut options = options.clone();
+    options.csv.bom = false;
+    options.tsv.bom = false;
+    let mut writer = Writer::new(format, &options, columns, target);
+    let mut text = String::new();
+    writer.start(&mut text);
+    for row in rows {
+        writer.row(&row, &mut text);
+        if text.len() + writer.held() > CLIPBOARD_BYTES {
+            return Copied::TooLarge;
+        }
+    }
+    writer.finish(&mut text);
+    if text.len() > CLIPBOARD_BYTES {
+        return Copied::TooLarge;
+    }
+    let ending = text
+        .strip_suffix("\r\n")
+        .or_else(|| text.strip_suffix('\n'));
+    if let Some(kept) = ending {
+        text.truncate(kept.len());
+    }
+    Copied::Text {
+        statements: writer.statements(),
+        text,
+    }
+}
+
+/// The columns' names on one line, for a `SELECT` list: each as the
+/// dialect writes a name, quoted only where it must be.
+pub fn column_names(names: &[&str], dialect: Dialect) -> String {
+    let names: Vec<_> = names.iter().map(|name| dialect.ident(name)).collect();
+    names.join(", ")
+}
 
 #[cfg(test)]
 #[allow(clippy::unwrap_used)]
diff --git a/crates/tabletist-core/src/format.rs b/crates/tabletist-core/src/format.rs
--- a/crates/tabletist-core/src/format.rs
+++ b/crates/tabletist-core/src/format.rs
@@ -19,7 +19,7 @@
 
 /// A float with an exponent where PostgreSQL writes one: plain digits
 /// would run to a line of zeros that says nothing of the number.
-fn float_text(number: f64) -> String {
+pub fn float_text(number: f64) -> String {
     let size = number.abs();
     if number.is_finite() && size != 0.0 && !(1e-4..1e15).contains(&size) {
         format!("{number:e}")
diff --git a/crates/tabletist-core/src/paths.rs b/crates/tabletist-core/src/paths.rs
--- a/crates/tabletist-core/src/paths.rs
+++ b/crates/tabletist-core/src/paths.rs
@@ -142,6 +142,12 @@
     /// connections is written only when one of them changes.
     pub fn last_used_file(&self) -> PathBuf {
         self.state.join("last_used.json")
+    }
+
+    /// The options last chosen for each format of an export and of Copy
+    /// as. Of this computer, and no setting: written when they change.
+    pub fn export_options_file(&self) -> PathBuf {
+        self.state.join("export_options.json")
     }
 
     /// The connection tabs open at the last quit, and each connection's
diff --git a/crates/tabletist-db/src/dialect.rs b/crates/tabletist-db/src/dialect.rs
--- a/crates/tabletist-db/src/dialect.rs
+++ b/crates/tabletist-db/src/dialect.rs
@@ -543,6 +543,61 @@
             Operand::Bytes(bytes) => Value::Bytes(bytes.as_slice().into()),
         });
         self.placeholder().to_owned()
+    }
+
+    /// A loaded value as a literal of this dialect, for an `INSERT` a
+    /// person copies or exports: what a save would write for it. A
+    /// numeric column's text that reads as a number (PostgreSQL's
+    /// `numeric`, MySQL's `DECIMAL`) is written bare, every digit kept.
+    pub fn value_literal(self, value: &Value, kind: crate::ValueKind) -> String {
+        let operand = match (self, value) {
+            (_, Value::Null) => Operand::Null,
+            (_, Value::Int(number)) => Operand::Int(*number),
+            (Self::Sqlite, Value::Bool(flag)) => Operand::Int(i64::from(*flag)),
+            (Self::Postgres | Self::MySql, Value::Bool(flag)) => {
+                Operand::Raw(if *flag { "TRUE" } else { "FALSE" }.to_owned())
+            }
+            // PostgreSQL reads its words for the numbers no digits write.
+            (Self::Postgres, Value::Float(number)) if !number.is_finite() => {
+                let word = match number {
+                    number if number.is_nan() => "NaN",
+                    number if number.is_sign_positive() => "Infinity",
+                    _ => "-Infinity",
+                };
+                Operand::Text(word.to_owned())
+            }
+            // SQLite keeps a NaN as NULL, and MySQL keeps none.
+            (Self::Sqlite | Self::MySql, Value::Float(number)) if number.is_nan() => Operand::Null,
+            (_, Value::Float(number)) => Operand::Float(*number),
+            (_, Value::Text(text))
+                if kind == crate::ValueKind::Numeric && crate::value::is_plain_number(text) =>
+            {
+                Operand::Raw(text.to_string())
+            }
+            (_, Value::Text(text)) => Operand::Text(text.to_string()),
+            (_, Value::Bytes(bytes)) => Operand::Bytes(bytes.to_vec()),
+        };
+        self.shown(&operand)
+    }
+
+    /// What a save writes in place of a pending DEFAULT or the time of the
+    /// save, in a column of `type_name`: `DEFAULT` (on SQLite the column's
+    /// default, or NULL), or the dialect's clock. `None` for a value.
+    pub fn pending_literal(self, new: &NewValue, type_name: &str) -> Option<String> {
+        let operand = match new {
+            NewValue::Null | NewValue::Text(_) => return None,
+            // As `new_operand` writes it: SQLite has no DEFAULT among VALUES.
+            NewValue::Default { expression } => Operand::Raw(match (self, expression) {
+                (Self::Postgres | Self::MySql, _) => "DEFAULT".to_owned(),
+                (Self::Sqlite, Some(expression)) => format!("({expression})"),
+                (Self::Sqlite, None) => "NULL".to_owned(),
+            }),
+            NewValue::Now => {
+                let kind = temporal(type_name).unwrap_or(Temporal::Timestamp);
+                Operand::Raw(self.now(kind).to_owned())
+            }
+        };
+        Some(self.shown(&operand))
     }
 
     /// A key's loaded value as the operand that finds its row again.
diff --git a/crates/tabletist-db/src/lib.rs b/crates/tabletist-db/src/lib.rs
--- a/crates/tabletist-db/src/lib.rs
+++ b/crates/tabletist-db/src/lib.rs
@@ -56,7 +56,7 @@
 };
 pub use spec::{ConnectSpec, Driver, ParsedUrl, Secrets, SshAuth, SshSpec, TlsMode};
 pub use ssh::HostKeys;
-pub use value::{ColumnMeta, Value, ValueKind, value_from_pg_text};
+pub use value::{ColumnMeta, Value, ValueKind, is_plain_number, value_from_pg_text};
 pub use write::{
     CellChange, ChangeSet, Conflict, InsertValue, MAX_RUN, NewValue, RowChange, RowDelete,
     RowInsert, WriteOutcome, delete_runs,
diff --git a/crates/tabletist-db/src/value.rs b/crates/tabletist-db/src/value.rs
--- a/crates/tabletist-db/src/value.rs
+++ b/crates/tabletist-db/src/value.rs
@@ -120,6 +120,43 @@
         .collect()
 }
 
+/// Whether `text` is a number as JSON and SQL both write one: an optional
+/// minus, digits with no leading zero, a fraction and an exponent.
+/// `NaN`, `Infinity` and `$1.00` are not.
+pub fn is_plain_number(text: &str) -> bool {
+    let bytes = text.as_bytes();
+    let mut at = usize::from(bytes.first() == Some(&b'-'));
+    let digits = |at: &mut usize| {
+        let start = *at;
+        while bytes.get(*at).is_some_and(u8::is_ascii_digit) {
+            *at += 1;
+        }
+        *at - start
+    };
+    let whole = at;
+    match digits(&mut at) {
+        0 => return false,
+        many if many > 1 && bytes[whole] == b'0' => return false,
+        _ => {}
+    }
+    if bytes.get(at) == Some(&b'.') {
+        at += 1;
+        if digits(&mut at) == 0 {
+            return false;
+        }
+    }
+    if matches!(bytes.get(at), Some(b'e' | b'E')) {
+        at += 1;
+        if matches!(bytes.get(at), Some(b'+' | b'-')) {
+            at += 1;
+        }
+        if digits(&mut at) == 0 {
+            return false;
+        }
+    }
+    at == bytes.len()
+}
+
 /// A result column.
 #[derive(Debug, Clone, PartialEq)]
 pub struct ColumnMeta {
diff --git a/src/app.rs b/src/app.rs
--- a/src/app.rs
+++ b/src/app.rs
@@ -152,6 +152,9 @@
     /// id, as the file had them: a newer version may have written them,
     /// and they are written back with the others.
     table_lists_unread: std::collections::BTreeMap<String, serde_json::Value>,
+    /// The options last chosen for each format of an export and of Copy
+    /// as, from export_options.json.
+    pub export: crate::export::Options,
     /// What was open at the last quit, and each connection's tables as
     /// its window last had them, from session.json.
     pub session: crate::session::Session,
@@ -386,6 +389,7 @@
         };
         let (table_lists, table_lists_unread) = load_table_lists(&dirs.table_lists_file());
         let session = crate::session::Session::load(&dirs.session_file());
+        let export = crate::export::Options::load(&dirs.export_options_file());
         let recents = crate::recents::Recents::load(&dirs.recents_file());
         let updates = crate::update::Updates {
             seen: crate::update::Seen::load(&dirs.updates_file()),
@@ -420,6 +424,7 @@
             labels,
             table_lists,
             table_lists_unread,
+            export,
             session,
             writes_session,
             recents,
@@ -5033,6 +5038,14 @@
         self.backend.send(Command::Save {
             path: self.dirs.table_lists_file(),
             file: StateFile::TableLists(lists),
+        });
+    }
+
+    /// Saves the options last chosen for each format, on the backend.
+    pub fn save_export_options(&mut self) {
+        self.backend.send(Command::Save {
+            path: self.dirs.export_options_file(),
+            file: StateFile::ExportOptions(self.export.to_file()),
         });
     }
 
```

- [ ] **Step 4: Run the checks**

Run: `~/.cargo/bin/cargo fmt --check && ~/.cargo/bin/cargo clippy --workspace --all-targets -- -D warnings && ~/.cargo/bin/cargo test --locked --workspace --all-targets`

Expected: every check passes: `fmt` and `clippy` say nothing, and 4,838 tests pass in 30 test binaries, none failing.

- [ ] **Step 5: Commit**

```bash
git add -A crates .gitattributes src/lib.rs src/app.rs
git commit -m "Write the six export formats, and remember each one's options"
```

### Task 2: Copy, Copy as, and the clipboard's limit

**Files:** `crates/tabletist-core/src/format.rs`, `crates/tabletist-core/src/keymap.rs`, `crates/tabletist-core/src/model.rs`, `docs/_guide/browsing-data.md`, `docs/_guide/macos.md`, `docs/_guide/omarchy.md`, `docs/_reference/keyboard-shortcuts.md`, `src/app.rs`, `src/ui/copy_as.rs`, `src/ui/copy_as_tests.rs`, `src/ui/data_view.rs`, `src/ui/grid.rs`, `src/ui/keys.rs`, `src/ui/mod.rs`, `src/ui/row_panel.rs`, `src/ui/selection_tests.rs`, `src/ui/sql_results.rs`, `src/ui/workspace.rs`

Mod+C, Mod+Shift+C, `y` and `yy` copy through the TSV writer; Copy as opens from Mod+Alt+C, `gy`, the grid's right-click and the row panel's Copy row ▾; past 10,000 rows or 20 MB nothing is copied and the toast says why; the key lines and the website say it.

- [ ] **Step 1: Write the tests**

```diff
diff --git a/src/app.rs b/src/app.rs
--- a/src/app.rs
+++ b/src/app.rs
@@ -22643,9 +22643,10 @@
             });
             harness.app.apply(Action::SetNull { tab, id });
             assert_eq!(harness.app.copy_text(false), null);
+            // In a row, as TSV writes a NULL: empty, as a loaded one.
             assert_eq!(
-                harness.app.copy_text(true),
-                null.map(|null| format!("1\tada@example.com\t{null}"))
+                harness.app.copy_text(true).as_deref(),
+                Some("1\tada@example.com\t")
             );
             // Another row's changes are not this row's.
             harness.app.apply(Action::SelectCell {
diff --git a/src/ui/copy_as_tests.rs b/src/ui/copy_as_tests.rs
new file mode 100644
--- /dev/null
+++ b/src/ui/copy_as_tests.rs
@@ -0,0 +1,519 @@
+//! Copy as and the copy of a selection, checked against their spec
+//! (`docs/superpowers/specs/2026-11-03-export-copy-as-2-writers-design.md`:
+//! "test N" below is an entry of its Tests).
+//!
+//! Behaviour and words, on the Bookshop's `book_covers` and on a SQL
+//! result. No picture of the design is compared.
+
+use egui::accesskit::Role;
+use egui::{Key, Modifiers};
+use tabletist_db::Value;
+
+use super::insert_audit_tests::{covers_in, painted, select, type_key};
+use crate::model::{Action, ConnTabId, Dialog, TabId};
+use crate::testing::Harness;
+use crate::theme::Look;
+
+/// Mod+Alt+C, as the window sends it: a copy with Alt held.
+fn copy_as_chord(harness: &mut Harness) {
+    harness.settle();
+    let held = Modifiers {
+        alt: true,
+        ..Modifiers::COMMAND
+    };
+    harness.frame(vec![egui::Event::ModifiersChanged(held), egui::Event::Copy]);
+    harness.frame(vec![egui::Event::ModifiersChanged(Modifiers::NONE)]);
+    harness.settle();
+}
+
+fn menu_open(harness: &Harness) -> bool {
+    matches!(harness.app.dialog, Some(Dialog::CopyAs(_)))
+}
+
+fn said(harness: &Harness) -> Option<String> {
+    crate::ui::toast::said(&harness.ctx)
+}
+
+/// The rectangle of `publisher_id` and `kind` over the first three rows.
+fn rectangle(harness: &mut Harness, tab: ConnTabId, id: TabId) {
+    select(harness, tab, id, (0, 1));
+    if harness.app.look.terminal {
+        type_key(harness, Key::V, "v");
+        type_key(harness, Key::J, "j");
+        type_key(harness, Key::J, "j");
+        type_key(harness, Key::L, "l");
+    } else {
+        harness.press(Key::ArrowDown, Modifiers::SHIFT);
+        harness.press(Key::ArrowDown, Modifiers::SHIFT);
+        harness.press(Key::ArrowRight, Modifiers::SHIFT);
+    }
+}
+
+/// Picks with a letter of the open menu, and returns what was copied.
+fn pick(harness: &mut Harness, key: Key, letter: &str) -> Option<String> {
+    harness.copied = None;
+    type_key(harness, key, letter);
+    harness.copied.clone()
+}
+
+/// A numeric column is right-aligned, its name too.
+const MARKDOWN: &str = "\
+|        publisher_id | kind  |
+|--------------------:|-------|
+| 9100000000000000001 | print |
+| 9100000000000000002 | ebook |
+| 9100000000000000003 | print |";
+
+/// Test 6: Mod+Alt+C opens the menu over a rectangle, and a letter copies
+/// it in that format with the format's options, and says so.
+#[test]
+fn the_keyboard_menu_copies_a_rectangle_in_each_format() {
+    let (mut harness, tab, id) = covers_in(Look::macos());
+    rectangle(&mut harness, tab, id);
+    let sent = harness.app.backend.sent.len();
+    copy_as_chord(&mut harness);
+    assert!(menu_open(&harness));
+    for row in [
+        "CSV",
+        "TSV",
+        "JSON",
+        "Markdown",
+        "SQL INSERT",
+        "Column names",
+    ] {
+        assert!(painted(&harness, row), "{row}: {:?}", harness.painted);
+    }
+    assert_eq!(pick(&mut harness, Key::M, "m").as_deref(), Some(MARKDOWN));
+    assert!(!menu_open(&harness));
+    assert_eq!(
+        said(&harness).as_deref(),
+        Some("Copied 3 rows × 2 columns as Markdown")
+    );
+    let cases = [
+        (
+            Key::C,
+            "c",
+            "publisher_id,kind\n9100000000000000001,print\n9100000000000000002,ebook\n9100000000000000003,print",
+            "Copied 3 rows × 2 columns as CSV",
+        ),
+        (
+            Key::T,
+            "t",
+            "publisher_id\tkind\n9100000000000000001\tprint\n9100000000000000002\tebook\n9100000000000000003\tprint",
+            "Copied 3 rows × 2 columns as TSV",
+        ),
+        (
+            Key::S,
+            "s",
+            "INSERT INTO main.book_covers (publisher_id, kind) VALUES\n  (9100000000000000001, 'print'),\n  (9100000000000000002, 'ebook'),\n  (9100000000000000003, 'print');",
+            "Copied 3 rows × 2 columns as SQL INSERT · 1 statement",
+        ),
+        (Key::N, "n", "publisher_id, kind", "Copied 2 column names"),
+    ];
+    for (key, letter, text, words) in cases {
+        copy_as_chord(&mut harness);
+        assert_eq!(
+            pick(&mut harness, key, letter).as_deref(),
+            Some(text),
+            "{letter}"
+        );
+        assert_eq!(said(&harness).as_deref(), Some(words), "{letter}");
+    }
+    copy_as_chord(&mut harness);
+    let json = pick(&mut harness, Key::J, "j").expect("JSON");
+    assert!(
+        json.starts_with(
+            "[\n  {\n    \"publisher_id\": 9100000000000000001,\n    \"kind\": \"print\"\n  },"
+        ),
+        "{json}"
+    );
+    // Test 8: a copy records nothing, and sends the backend nothing.
+    assert_eq!(harness.app.backend.sent.len(), sent);
+}
+
+/// Test 6: the menu's keys: the arrows and Return pick the lit row, Esc
+/// closes it with nothing copied.
+#[test]
+fn the_menus_arrows_and_return_pick_and_esc_closes() {
+    let (mut harness, tab, id) = covers_in(Look::macos());
+    rectangle(&mut harness, tab, id);
+    copy_as_chord(&mut harness);
+    harness.press(Key::ArrowDown, Modifiers::NONE);
+    harness.press(Key::ArrowDown, Modifiers::NONE);
+    harness.press(Key::ArrowUp, Modifiers::NONE);
+    harness.copied = None;
+    harness.press(Key::Enter, Modifiers::NONE);
+    assert!(!menu_open(&harness));
+    let tsv = harness.copied.clone().expect("TSV");
+    assert!(tsv.starts_with("publisher_id\tkind\n"), "{tsv}");
+    copy_as_chord(&mut harness);
+    assert!(menu_open(&harness));
+    harness.copied = None;
+    harness.press(Key::Escape, Modifiers::NONE);
+    assert!(!menu_open(&harness));
+    assert_eq!(harness.copied, None);
+}
+
+/// Test 6: with no range Copy as takes the cursor's row whole, and says
+/// so in rows.
+#[test]
+fn copy_as_with_no_range_takes_the_cursors_row() {
+    let (mut harness, tab, id) = covers_in(Look::macos());
+    select(&mut harness, tab, id, (1, 2));
+    copy_as_chord(&mut harness);
+    let csv = pick(&mut harness, Key::C, "c").expect("CSV");
+    assert_eq!(
+        csv,
+        "id,publisher_id,kind,image_data,created_at\n2,9100000000000000002,ebook,,2026-01-12 09:14:03"
+    );
+    assert_eq!(said(&harness).as_deref(), Some("Copied 1 row as CSV"));
+}
+
+/// Test 6: the terminal look's `gy` opens the float over a visual block,
+/// a letter copies it, the visual selection is left, and the status line
+/// says it.
+#[test]
+fn gy_opens_the_float_and_a_letter_copies_the_block() {
+    let (mut harness, tab, id) = covers_in(Look::omarchy());
+    rectangle(&mut harness, tab, id);
+    assert!(painted(&harness, "-- VISUAL --"), "{:?}", harness.painted);
+    let offers = |text: &str| {
+        harness
+            .painted
+            .iter()
+            .any(|(painted, _)| painted.contains(text))
+    };
+    assert!(offers("gy copy as"), "{:?}", harness.painted);
+    assert!(offers("y copy tsv"), "{:?}", harness.painted);
+    type_key(&mut harness, Key::G, "g");
+    type_key(&mut harness, Key::Y, "y");
+    assert!(menu_open(&harness));
+    assert!(painted(&harness, "copy as"));
+    assert!(painted(&harness, "3 rows × 2 columns"));
+    assert!(painted(&harness, "m  markdown"), "{:?}", harness.painted);
+    assert_eq!(pick(&mut harness, Key::M, "m").as_deref(), Some(MARKDOWN));
+    assert!(!menu_open(&harness));
+    let workspace = harness.app.workspace(tab).unwrap();
+    assert!(!workspace.object_tab(id).unwrap().visual());
+    assert!(painted(&harness, "copied 3 rows × 2 columns as markdown"));
+}
+
+/// Test 6: `ctrl+n` and `ctrl+p` move in the float, Enter picks.
+#[test]
+fn the_floats_keys_move_and_pick() {
+    let (mut harness, tab, id) = covers_in(Look::omarchy());
+    rectangle(&mut harness, tab, id);
+    type_key(&mut harness, Key::G, "g");
+    type_key(&mut harness, Key::Y, "y");
+    harness.press(Key::N, Modifiers::CTRL);
+    harness.press(Key::N, Modifiers::CTRL);
+    harness.press(Key::P, Modifiers::CTRL);
+    harness.copied = None;
+    harness.press(Key::Enter, Modifiers::NONE);
+    let tsv = harness.copied.clone().expect("TSV");
+    assert!(tsv.starts_with("publisher_id\tkind\n"), "{tsv}");
+}
+
+/// Test 6: a SQL result's selection, as a table's: SQL INSERT writes into
+/// `result`.
+#[test]
+fn a_results_selection_is_copied_into_result() {
+    let mut harness = Harness::new();
+    harness.set_look(Look::macos());
+    let tab = harness.connect_fake();
+    super::tests::with_sql_result(&mut harness, tab, 3);
+    // The pointer on the first row's first cell: the result's grid takes
+    // the keys.
+    let tree = harness.settle();
+    let row = crate::testing::bounds(&tree, "Row 1", Role::Button).expect("Row 1");
+    let pos =
+        row.left_center() + egui::vec2(crate::ui::grid::gutter(&harness.app.look) + 10.0, 0.0);
+    let button = |pressed| egui::Event::PointerButton {
+        pos,
+        button: egui::PointerButton::Primary,
+        pressed,
+        modifiers: Modifiers::NONE,
+    };
+    harness.frame(vec![egui::Event::PointerMoved(pos)]);
+    harness.frame(vec![button(true)]);
+    harness.frame(vec![button(false)]);
+    harness.settle();
+    harness.press(Key::ArrowDown, Modifiers::SHIFT);
+    copy_as_chord(&mut harness);
+    let sql = pick(&mut harness, Key::S, "s").expect("SQL INSERT");
+    assert!(sql.starts_with("INSERT INTO result ("), "{sql}");
+    assert_eq!(sql.lines().count(), 3, "{sql}");
+}
+
+/// Test 6: the row panel's Copy row ▾ copies the cursor's row, whole.
+#[test]
+fn the_row_panels_copy_row_copies_the_row_in_a_format() {
+    let (mut harness, tab, id) = covers_in(Look::macos());
+    rectangle(&mut harness, tab, id);
+    harness.click("Copy row ▾");
+    harness.copied = None;
+    harness.click("Copy row as JSON");
+    let json = harness.copied.clone().expect("JSON");
+    // The cursor's row alone, every column: the range is not what it takes.
+    assert!(json.contains("\"id\": 3"), "{json}");
+    assert!(!json.contains("\"id\": 1"), "{json}");
+    assert!(json.contains("\"image_data\": null"), "{json}");
+    assert_eq!(said(&harness).as_deref(), Some("Copied 1 row as JSON"));
+}
+
+/// Test 6: a secondary click outside the selection selects its cell, and
+/// its menu's Copy as › copies it.
+#[test]
+fn the_grids_menu_copies_the_cell_under_the_pointer() {
+    let (mut harness, tab, id) = covers_in(Look::macos());
+    select(&mut harness, tab, id, (0, 0));
+    let tree = harness.settle();
+    let row = crate::testing::bounds(&tree, "Row 2", Role::Button).expect("Row 2");
+    let gutter = crate::ui::grid::gutter(&harness.app.look);
+    let pos = row.left_center() + egui::vec2(gutter + 10.0, 0.0);
+    let button = |pressed| egui::Event::PointerButton {
+        pos,
+        button: egui::PointerButton::Secondary,
+        pressed,
+        modifiers: Modifiers::NONE,
+    };
+    harness.frame(vec![egui::Event::PointerMoved(pos)]);
+    harness.frame(vec![button(true)]);
+    harness.frame(vec![button(false)]);
+    harness.settle();
+    let workspace = harness.app.workspace(tab).unwrap();
+    let cursor = workspace.object_tab(id).unwrap().selection;
+    assert_eq!(cursor, Some(crate::model::CellPos { row: 1, col: 0 }));
+    assert!(
+        painted(&harness, "Copy rows as TSV"),
+        "{:?}",
+        harness.painted
+    );
+    harness.click("Copy as");
+    harness.copied = None;
+    harness.click("CSV");
+    assert_eq!(
+        harness.copied.as_deref(),
+        Some(
+            "id,publisher_id,kind,image_data,created_at\n2,9100000000000000002,ebook,,2026-01-12 09:14:03"
+        )
+    );
+}
+
+/// The Bookshop's covers with a page of `rows`, the grid holding the keys.
+fn covers_of(page: tabletist_db::RowPage) -> (Harness, ConnTabId, TabId) {
+    let mut harness = Harness::new();
+    harness.set_look(Look::macos());
+    let tab = harness.connect_fake_as(false);
+    harness.app.apply(Action::OpenObject {
+        tab,
+        object: tabletist_db::ObjectRef::new("main", "book_covers"),
+        kind: tabletist_db::ObjectKind::Table,
+        pin: true,
+    });
+    harness.answer_structure(crate::testing::book_covers_structure());
+    harness.answer_rows(page);
+    let id = harness.app.workspace(tab).unwrap().active_tab.unwrap();
+    harness.app.workspace_mut(tab).unwrap().pane = crate::model::Pane::Grid;
+    harness.settle();
+    (harness, tab, id)
+}
+
+/// Test 7: past 10,000 rows nothing is copied, by Copy as or Mod+C, and
+/// the toast says why.
+#[test]
+fn more_than_ten_thousand_rows_copy_nothing() {
+    let (mut harness, tab, id) = covers_of(crate::testing::book_covers_page(10_001));
+    select(&mut harness, tab, id, (0, 0));
+    harness.press(Key::A, Modifiers::COMMAND);
+    harness.copied = None;
+    harness.copy(false);
+    assert_eq!(harness.copied, None);
+    let words = "That's 10,001 rows. Export to a file?";
+    assert_eq!(said(&harness).as_deref(), Some(words));
+    copy_as_chord(&mut harness);
+    assert_eq!(pick(&mut harness, Key::C, "c"), None);
+    assert_eq!(said(&harness).as_deref(), Some(words));
+}
+
+/// Test 7: over 20 MB nothing is copied either.
+#[test]
+fn more_than_twenty_megabytes_copy_nothing() {
+    let mut page = crate::testing::book_covers_page(2);
+    page.rows[1][2] = Value::Text("w".repeat(crate::export::CLIPBOARD_BYTES).into());
+    let (mut harness, tab, id) = covers_of(page);
+    select(&mut harness, tab, id, (0, 0));
+    harness.press(Key::A, Modifiers::COMMAND);
+    copy_as_chord(&mut harness);
+    assert_eq!(pick(&mut harness, Key::C, "c"), None);
+    let words = "That's over 20 MB. Export to a file?";
+    assert_eq!(said(&harness).as_deref(), Some(words));
+    // Mod+C too.
+    harness.copied = None;
+    harness.copy(false);
+    assert_eq!(harness.copied, None);
+    assert_eq!(said(&harness).as_deref(), Some(words));
+}
+
+/// Test 5: Mod+C and `y` write TSV with no header row, its NULL empty.
+#[test]
+fn the_copy_of_a_range_is_tsv_with_no_header() {
+    for look in [Look::macos(), Look::omarchy()] {
+        let (mut harness, tab, id) = covers_in(look);
+        select(&mut harness, tab, id, (0, 2));
+        if look.terminal {
+            type_key(&mut harness, Key::V, "v");
+            type_key(&mut harness, Key::J, "j");
+            type_key(&mut harness, Key::L, "l");
+            harness.copied = None;
+            type_key(&mut harness, Key::Y, "y");
+        } else {
+            harness.press(Key::ArrowDown, Modifiers::SHIFT);
+            harness.press(Key::ArrowRight, Modifiers::SHIFT);
+            harness.copied = None;
+            harness.copy(false);
+        }
+        assert_eq!(
+            harness.copied.as_deref(),
+            Some("print\t\nebook\t"),
+            "{}",
+            look.name
+        );
+    }
+}
+
+/// `g` then `y`, in the terminal look.
+fn gy(harness: &mut Harness) {
+    type_key(harness, Key::G, "g");
+    type_key(harness, Key::Y, "y");
+}
+
+/// A secondary click at `pos`.
+fn secondary(harness: &mut Harness, pos: egui::Pos2) {
+    let button = |pressed| egui::Event::PointerButton {
+        pos,
+        button: egui::PointerButton::Secondary,
+        pressed,
+        modifiers: Modifiers::NONE,
+    };
+    harness.frame(vec![egui::Event::PointerMoved(pos)]);
+    harness.frame(vec![button(true)]);
+    harness.frame(vec![button(false)]);
+    harness.settle();
+}
+
+/// Test 6: a pending DEFAULT is written in SQL INSERT as a save sends it,
+/// never as the text 'DEFAULT'.
+#[test]
+fn a_pending_default_is_copied_as_the_keyword() {
+    let (mut harness, tab, id) = covers_in(Look::macos());
+    select(&mut harness, tab, id, (1, 2));
+    harness.app.apply(Action::SetDefault { tab, id });
+    harness.settle();
+    copy_as_chord(&mut harness);
+    let sql = pick(&mut harness, Key::S, "s").expect("SQL INSERT");
+    // SQLite has no DEFAULT among VALUES: the column's own default.
+    assert!(
+        sql.contains("(2, 9100000000000000002, ('print'), NULL,"),
+        "{sql}"
+    );
+}
+
+/// Test 6: Copy as on a new row with no range copies it, what is not set
+/// in it NULL.
+#[test]
+fn copy_as_on_a_new_row_copies_it() {
+    let (mut harness, tab, id) = covers_in(Look::macos());
+    let new = crate::edit::new_row(super::insert_audit_tests::add(&mut harness, tab, id));
+    super::insert_audit_tests::set(&mut harness, tab, id, (new, 2), "audio");
+    select(&mut harness, tab, id, (new, 2));
+    copy_as_chord(&mut harness);
+    assert!(menu_open(&harness));
+    assert_eq!(
+        pick(&mut harness, Key::C, "c").as_deref(),
+        Some("id,publisher_id,kind,image_data,created_at\n,,audio,,")
+    );
+}
+
+/// Test 6: `gy` in the row panel opens the float on its row: titled for
+/// it, five rows, its TSV as Mod+Shift+C's.
+#[test]
+fn gy_in_the_row_panel_copies_its_row() {
+    let (mut harness, tab, id) = covers_in(Look::omarchy());
+    select(&mut harness, tab, id, (1, 2));
+    harness.press(Key::L, Modifiers::CTRL);
+    assert!(crate::ui::row_panel::focused_field(&harness.ctx, tab, id, 5).is_some());
+    gy(&mut harness);
+    assert!(menu_open(&harness));
+    assert!(painted(&harness, "copy row as"), "{:?}", harness.painted);
+    assert!(!painted(&harness, "n  column names"));
+    // Past the last row the keys stay on it: one up is Markdown.
+    for _ in 0..6 {
+        harness.press(Key::N, Modifiers::CTRL);
+    }
+    harness.press(Key::P, Modifiers::CTRL);
+    harness.copied = None;
+    harness.press(Key::Enter, Modifiers::NONE);
+    let markdown = harness.copied.clone().expect("Markdown");
+    assert!(markdown.starts_with("|"), "{markdown}");
+    let field = crate::ui::row_panel::focused_field(&harness.ctx, tab, id, 5);
+    assert!(field.is_some(), "the field keeps the keyboard");
+    gy(&mut harness);
+    assert!(menu_open(&harness), "{:?}", harness.app.dialog);
+    assert_eq!(
+        pick(&mut harness, Key::T, "t").as_deref(),
+        Some("2\t9100000000000000002\tebook\t\t2026-01-12 09:14:03")
+    );
+    assert!(
+        painted(&harness, "copied 1 row as tsv"),
+        "{:?}",
+        harness.painted
+    );
+}
+
+/// Test 6: the row panel's Copy row as TSV says so.
+#[test]
+fn copy_row_as_tsv_says_so() {
+    let (mut harness, tab, id) = covers_in(Look::macos());
+    select(&mut harness, tab, id, (1, 2));
+    harness.click("Copy row ▾");
+    harness.click("Copy row as TSV");
+    assert_eq!(said(&harness).as_deref(), Some("Copied 1 row as TSV"));
+}
+
+/// Test 6: a secondary click on a row's number selects that row, and its
+/// menu copies it.
+#[test]
+fn the_grids_menu_on_a_row_number_copies_that_row() {
+    let (mut harness, tab, id) = covers_in(Look::macos());
+    select(&mut harness, tab, id, (0, 0));
+    let tree = harness.settle();
+    let row = crate::testing::bounds(&tree, "Row 3", Role::Button).expect("Row 3");
+    secondary(&mut harness, row.left_center() + egui::vec2(4.0, 0.0));
+    harness.click("Copy as");
+    harness.copied = None;
+    harness.click("CSV");
+    let csv = harness.copied.clone().expect("CSV");
+    assert!(
+        csv.ends_with("\n3,9100000000000000003,print,,2026-01-12 09:14:03"),
+        "{csv}"
+    );
+    assert_eq!(csv.lines().count(), 2, "{csv}");
+}
+
+/// Test 6: a secondary click inside the selection keeps it.
+#[test]
+fn the_grids_menu_inside_the_selection_keeps_it() {
+    let (mut harness, tab, id) = covers_in(Look::macos());
+    rectangle(&mut harness, tab, id);
+    let tree = harness.settle();
+    let row = crate::testing::bounds(&tree, "Row 2", Role::Button).expect("Row 2");
+    let kind = harness.painted_rect("ebook").expect("the cell");
+    secondary(&mut harness, egui::pos2(kind.center().x, row.center().y));
+    harness.click("Copy as");
+    harness.click("CSV");
+    assert_eq!(
+        said(&harness).as_deref(),
+        Some("Copied 3 rows × 2 columns as CSV")
+    );
+}
diff --git a/src/ui/mod.rs b/src/ui/mod.rs
--- a/src/ui/mod.rs
+++ b/src/ui/mod.rs
@@ -27,6 +27,8 @@
 mod complete_tests;
 pub mod conflict_prompt;
 pub mod connect_dialog;
+#[cfg(test)]
+mod copy_as_tests;
 pub mod data_view;
 pub mod date_line;
 #[cfg(test)]
@@ -8289,10 +8291,8 @@
         harness.copy(false);
         assert_eq!(harness.copied.as_deref(), Some("user5@example.com"));
         harness.copy(true);
-        assert_eq!(
-            harness.copied.as_deref(),
-            Some("5\tuser5@example.com\tNULL")
-        );
+        // TSV's NULL as: empty by default.
+        assert_eq!(harness.copied.as_deref(), Some("5\tuser5@example.com\t"));
     }
 
     #[test]
diff --git a/src/ui/selection_tests.rs b/src/ui/selection_tests.rs
--- a/src/ui/selection_tests.rs
+++ b/src/ui/selection_tests.rs
@@ -180,7 +180,7 @@
     type_key(&mut harness, Key::V, "v");
     assert!(painted(&harness, "-- VISUAL --"), "{:?}", harness.painted);
     assert!(painted(&harness, "1 row × 1 column"));
-    assert!(painted(&harness, "y copy · esc normal"));
+    assert!(painted(&harness, "y copy tsv · gy copy as · esc normal"));
     type_key(&mut harness, Key::J, "j");
     type_key(&mut harness, Key::L, "l");
     type_key(&mut harness, Key::L, "l");
@@ -391,7 +391,7 @@
     assert!(painted(&harness, "-- VISUAL --"), "{:?}", harness.painted);
     assert!(painted(&harness, "1 row × 1 column"));
     assert!(
-        painted(&harness, "y copy · esc normal"),
+        painted(&harness, "y copy tsv · gy copy as · esc normal"),
         "{:?}",
         harness.painted
     );
```

- [ ] **Step 2: Run them, and see them fail**

Run: `~/.cargo/bin/cargo test -p tabletist --lib -- copy_as_tests copying_takes_the_pending_value arrow_keys_move_the_selection`

Expected: the build fails on the first thing the new tests name: ``error[E0599]: no variant, associated function, or constant named `CopyAs` found for enum `tabletist_core::model::Dialog` ``.

- [ ] **Step 3: Write the code**

```diff
diff --git a/crates/tabletist-core/src/format.rs b/crates/tabletist-core/src/format.rs
--- a/crates/tabletist-core/src/format.rs
+++ b/crates/tabletist-core/src/format.rs
@@ -959,23 +959,6 @@
     }
 }
 
-/// A row as tab-separated values on one line.
-pub fn tsv_row(row: &[Value]) -> String {
-    tsv_cells(row.iter().cloned().map(Some))
-}
-
-/// Cells as one line of TSV, as [`tsv_row`] writes them: a cell with no
-/// value (a new row's that nothing is set in) is empty.
-pub fn tsv_cells(cells: impl Iterator<Item = Option<Value>>) -> String {
-    cells
-        .map(|value| {
-            let text = value.map_or_else(String::new, |value| plain_text(&value));
-            text.replace(['\t', '\n', '\r'], " ")
-        })
-        .collect::<Vec<_>>()
-        .join("\t")
-}
-
 pub fn elapsed(duration: Duration) -> String {
     if duration < Duration::from_secs(1) {
         format!("{} ms", duration.as_millis())
@@ -1397,7 +1380,6 @@
         assert_eq!(cell_text(&key), uuid);
         assert_eq!(plain_text(&key), uuid);
         assert_eq!(full_text(&key), uuid);
-        assert_eq!(tsv_row(&[Value::Int(1), key]), format!("1\t{uuid}"));
         // Any other length stays a blob.
         for len in [15, 17] {
             let blob = Value::Bytes(vec![0xab; len].into());
@@ -1546,12 +1528,6 @@
     }
 
     #[test]
-    fn tsv_rows_keep_one_line_per_row() {
-        let row = vec![Value::Int(1), text("a\tb\nc"), Value::Null];
-        assert_eq!(tsv_row(&row), "1\ta b c\tNULL");
-    }
-
-    #[test]
     fn elapsed_times_read_naturally() {
         assert_eq!(elapsed(Duration::from_millis(84)), "84 ms");
         assert_eq!(elapsed(Duration::from_millis(1234)), "1.2 s");
@@ -1670,7 +1646,6 @@
     fn copying_keeps_hidden_characters() {
         let value = text("users_\u{202E}atad");
         assert_eq!(plain_text(&value), "users_\u{202E}atad");
-        assert_eq!(tsv_row(&[value]), "users_\u{202E}atad");
     }
 
     #[test]
diff --git a/crates/tabletist-core/src/keymap.rs b/crates/tabletist-core/src/keymap.rs
--- a/crates/tabletist-core/src/keymap.rs
+++ b/crates/tabletist-core/src/keymap.rs
@@ -272,6 +272,8 @@
     ClearingHistory,
     /// The Connections list asks whether to delete a connection.
     DeletingConnection,
+    /// The Copy as menu is open: its letters pick a format.
+    CopyAs,
     /// The start screen's search is on: its field has the keyboard. The
     /// keys that move among the matches, open one and clear the search are
     /// read then, and only then.
@@ -562,6 +564,18 @@
     CopyCells = "copy_cells",
     CopySelection = "copy_selection",
     CopyRow = "copy_row",
+    CopyAs = "copy_as",
+    CopyRowAs = "copy_row_as",
+    CopyAsCsv = "copy_as_csv",
+    CopyAsTsv = "copy_as_tsv",
+    CopyAsJson = "copy_as_json",
+    CopyAsMarkdown = "copy_as_markdown",
+    CopyAsSql = "copy_as_sql",
+    CopyColumnNames = "copy_column_names",
+    CopyAsNext = "copy_as_next",
+    CopyAsPrevious = "copy_as_previous",
+    CopyAsPick = "copy_as_pick",
+    CloseCopyAs = "close_copy_as",
     PasteCells = "paste_cells",
     PasteNewRows = "paste_new_rows",
     AddRow = "add_row",
@@ -944,6 +958,18 @@
             C::CopyCells => info("Copy cell values", COPY, Editing),
             C::CopySelection => info("Copy the selection", COPY, Editing),
             C::CopyRow => info("Copy row", "", Editing),
+            C::CopyAs => info("Copy as…", "", Editing),
+            C::CopyRowAs => info("Copy row as…", "", Editing),
+            C::CopyAsCsv => info("Copy as CSV", "", Prompts),
+            C::CopyAsTsv => info("Copy as TSV", "", Prompts),
+            C::CopyAsJson => info("Copy as JSON", "", Prompts),
+            C::CopyAsMarkdown => info("Copy as Markdown", "", Prompts),
+            C::CopyAsSql => info("Copy as SQL INSERT", "", Prompts),
+            C::CopyColumnNames => info("Copy column names", "", Prompts),
+            C::CopyAsNext => info("Next format of Copy as", "", Prompts),
+            C::CopyAsPrevious => info("Previous format of Copy as", "", Prompts),
+            C::CopyAsPick => info("Copy in the highlighted format", "", Prompts),
+            C::CloseCopyAs => info("Close Copy as", "", Prompts),
             C::PasteCells => info("Paste cell values", COPY, Editing).reserved(),
             C::PasteNewRows => info("Paste as new rows", "Paste as new rows", Editing).reserved(),
             C::AddRow => info("Add row", "Add row", Editing),
@@ -1830,6 +1856,39 @@
         ),
         bind(C::CopySelection, ROWS, NONE, keys(&[k("y")])).when(When::Visual),
         bind(C::CopyRow, ROWS, keys(&[k("cmd+shift+c")]), NONE),
+        // Alt with the copy: the window's copy, as Cmd+Shift+C is.
+        bind(C::CopyAs, ROWS, keys(&[k("cmd+alt+c")]), keys(&[k("gy")])),
+        // The macOS look's Copy row ▾ is a button.
+        bind(C::CopyRowAs, INSPECTOR, NONE, keys(&[k("gy")])),
+        // The menu's letters, one a format.
+        bind(C::CopyAsCsv, PROMPT, keys(&[k("c")]), keys(&[k("c")])).when(When::CopyAs),
+        bind(C::CopyAsTsv, PROMPT, keys(&[k("t")]), keys(&[k("t")])).when(When::CopyAs),
+        bind(C::CopyAsJson, PROMPT, keys(&[k("j")]), keys(&[k("j")])).when(When::CopyAs),
+        bind(C::CopyAsMarkdown, PROMPT, keys(&[k("m")]), keys(&[k("m")])).when(When::CopyAs),
+        bind(C::CopyAsSql, PROMPT, keys(&[k("s")]), keys(&[k("s")])).when(When::CopyAs),
+        bind(C::CopyColumnNames, PROMPT, keys(&[k("n")]), keys(&[k("n")])).when(When::CopyAs),
+        bind(
+            C::CopyAsNext,
+            PROMPT,
+            keys(&[k("down")]),
+            keys(&[k("ctrl+n"), k("down")]),
+        )
+        .when(When::CopyAs),
+        bind(
+            C::CopyAsPrevious,
+            PROMPT,
+            keys(&[k("up")]),
+            keys(&[k("ctrl+p"), k("up")]),
+        )
+        .when(When::CopyAs),
+        bind(
+            C::CopyAsPick,
+            PROMPT,
+            keys(&[k("enter")]),
+            keys(&[k("enter")]),
+        )
+        .when(When::CopyAs),
+        bind(C::CloseCopyAs, PROMPT, keys(&[k("esc")]), keys(&[k("esc")])).when(When::CopyAs),
         bind(C::PasteCells, GRID, keys(&[k("cmd+v")]), NONE),
         bind(C::PasteCells, GRID, NONE, keys(&[k("p")]).shown("v p")).when(When::Visual),
         bind(
diff --git a/crates/tabletist-core/src/model.rs b/crates/tabletist-core/src/model.rs
--- a/crates/tabletist-core/src/model.rs
+++ b/crates/tabletist-core/src/model.rs
@@ -348,6 +348,21 @@
     LeaveVisual {
         tab: ConnTabId,
         id: TabId,
+    },
+    /// Open the Copy as menu on the grid in front: over its selection, or
+    /// the row panel's row.
+    OpenCopyAs {
+        tab: ConnTabId,
+        pick: CopyPick,
+    },
+    /// Move the Copy as menu's lit row.
+    MoveInCopyAs(isize),
+    /// Put `pick` of the grid in front on the clipboard, as `how` says,
+    /// and say so. A visual selection is left, as `y` leaves it.
+    CopyAs {
+        tab: ConnTabId,
+        pick: CopyPick,
+        how: CopyHow,
     },
     /// Open the editor on a cell of a table's grid, or say why it cannot
     /// be edited.
@@ -2758,6 +2773,78 @@
     Conflict(Box<ConflictPrompt>),
     /// What agents may do on a connection, in the terminal look's box.
     AgentAccess(Box<AgentAccessBox>),
+    /// The Copy as menu: ⌥⌘C, `gy`, or the row panel's Copy row ▾.
+    CopyAs(Box<CopyAsMenu>),
+}
+
+/// What a copy takes of a grid.
+#[derive(Debug, Clone, Copy, PartialEq, Eq)]
+pub enum CopyPick {
+    /// The selection: a rectangle's cells, or whole rows.
+    Selection,
+    /// Every column of the rows the selection covers.
+    Rows,
+    /// The cursor's row alone, whole: `yy`, and the row panel's.
+    CursorRow,
+}
+
+/// How a copy writes what it takes.
+#[derive(Debug, Clone, Copy, PartialEq, Eq)]
+pub enum CopyHow {
+    /// As ⌘C: TSV with its remembered options and no header row, and one
+    /// cell alone as its text.
+    Plain,
+    /// In a format, with its remembered options.
+    As(crate::export::Format),
+    /// The columns' names, for a `SELECT` list.
+    ColumnNames,
+}
+
+impl CopyHow {
+    /// The rows the Copy as menu offers over `pick`: the row panel's row
+    /// takes no column names, and its TSV is as ⇧⌘C copies it.
+    pub fn menu(pick: CopyPick) -> Vec<(CopyHow, crate::keymap::Command)> {
+        match pick {
+            CopyPick::CursorRow => Self::MENU[..5]
+                .iter()
+                .map(|(how, letter)| (how.of_row(), *letter))
+                .collect(),
+            CopyPick::Selection | CopyPick::Rows => Self::MENU.to_vec(),
+        }
+    }
+
+    /// The copy of a row whole that this is: TSV with no header row, as
+    /// ⇧⌘C copies it.
+    pub fn of_row(self) -> CopyHow {
+        match self {
+            CopyHow::As(crate::export::Format::Tsv) => CopyHow::Plain,
+            other => other,
+        }
+    }
+
+    /// The Copy as menu's rows, in order, with their letters' commands.
+    pub const MENU: [(CopyHow, crate::keymap::Command); 6] = {
+        use crate::export::Format;
+        use crate::keymap::Command;
+        [
+            (CopyHow::As(Format::Csv), Command::CopyAsCsv),
+            (CopyHow::As(Format::Tsv), Command::CopyAsTsv),
+            (CopyHow::As(Format::Json), Command::CopyAsJson),
+            (CopyHow::As(Format::Markdown), Command::CopyAsMarkdown),
+            (CopyHow::As(Format::SqlInsert), Command::CopyAsSql),
+            (CopyHow::ColumnNames, Command::CopyColumnNames),
+        ]
+    };
+}
+
+/// The Copy as menu, while it is open.
+#[derive(Debug, Clone, PartialEq, Eq)]
+pub struct CopyAsMenu {
+    /// The connection tab whose grid it copies from.
+    pub tab: ConnTabId,
+    pub pick: CopyPick,
+    /// The row the keyboard is on, of [`CopyHow::MENU`].
+    pub lit: usize,
 }
 
 /// The terminal look's box of what agents may do on a saved connection.
diff --git a/docs/_guide/browsing-data.md b/docs/_guide/browsing-data.md
--- a/docs/_guide/browsing-data.md
+++ b/docs/_guide/browsing-data.md
@@ -192,7 +192,23 @@
 On macOS and Windows, Cmd/Ctrl+C copies the selection with tabs between
 the values and a line for each row (the selected cell alone, without a
 selection), and Cmd/Ctrl+Shift+C its rows whole. In the Omarchy look `y`
-copies the selection, so `v` then `y` copies the cell.
+copies the selection, so `v` then `y` copies the cell, and `yy` copies the
+cursor's row. A tab, a line break, a carriage return or a backslash inside
+a value is written as `\t`, `\n`, `\r` or `\\`, a NULL is left empty, and
+binary values are written in hex (`0x…`).
+
+**Copy as** copies the selection in another format: CSV, TSV, JSON,
+Markdown or SQL INSERT, or only the names of its columns, ready for a
+`SELECT` list. Open it with Cmd/Ctrl+Alt+C (`gy` in the Omarchy look) and
+pick with one letter: `C`, `T`, `J`, `M`, `S` or `N`. A right-click on a
+cell offers it too, as does **Copy row ▾** at the top of the row panel for
+its row. With no range selected it copies the cursor's row. Values are
+written as the database holds them, never as the grid formats them, and
+SQL INSERT writes into the table, or into `result` for a SQL result. The
+toast says what was copied: "Copied 4 rows × 2 columns as Markdown".
+
+The clipboard takes up to 10,000 rows and 20 MB. A larger selection is not
+copied, and the toast says so. Copying records nothing in the history.
 
 ## Next
 
diff --git a/docs/_guide/macos.md b/docs/_guide/macos.md
--- a/docs/_guide/macos.md
+++ b/docs/_guide/macos.md
@@ -189,6 +189,7 @@
 | Cmd+F | Filter bar |
 | Cmd+C | Copy the selection, or the cell |
 | Cmd+Shift+C | Copy the selected rows |
+| Cmd+Option+C | [Copy as]({% link _guide/browsing-data.md %}#copying): the selection in a format, one letter each |
 | Shift+arrows, Shift-click, drag | Select cells |
 | Click a row's number | Select the row. Shift-click selects through to another, Cmd-click adds or takes out one |
 | Shift+Space | Select the rows of the selected cells |
diff --git a/docs/_guide/omarchy.md b/docs/_guide/omarchy.md
--- a/docs/_guide/omarchy.md
+++ b/docs/_guide/omarchy.md
@@ -251,6 +251,7 @@
 | `v`, Ctrl+V | Start a selection of cells, or a block, on the cursor's cell. The moves stretch it. The status line reads `-- VISUAL --` or `-- VISUAL BLOCK --` and its size |
 | `V` | Start a selection of rows on the cursor's row. `j`, `k` and the other moves extend it. The status line reads `-- VISUAL LINE --` and counts its rows |
 | `y` | In a selection: copy it, with tabs between the values, and leave it |
+| `gy` | [Copy as]({% link _guide/browsing-data.md %}#copying): the selection, or the cursor's row, in a format. One letter picks: `c` csv, `t` tsv, `j` json, `m` markdown, `s` sql insert, `n` column names. Ctrl+N and Ctrl+P move, Enter picks, Esc closes. In the row panel it copies the row |
 | `d` | In a selection of rows: delete its rows, or restore them where all are marked. New rows among them are dropped |
 | Esc, or the key that started it | Leave the selection |
 | Enter, Tab, Shift+Tab | Keep the edit and move down, right or left |
diff --git a/docs/_reference/keyboard-shortcuts.md b/docs/_reference/keyboard-shortcuts.md
--- a/docs/_reference/keyboard-shortcuts.md
+++ b/docs/_reference/keyboard-shortcuts.md
@@ -159,6 +159,7 @@
 | Shift+Space | Select the rows of the selected cells |
 | Mod+A | Select every row loaded |
 | Mod+C, Mod+Shift+C | Copy the selection as tab-separated values, or its rows whole. One cell is copied as it is |
+| Mod+Alt+C | [Copy as]({% link _guide/browsing-data.md %}#copying): the selection in CSV (C), TSV (T), JSON (J), Markdown (M), SQL INSERT (S), or its column names (N). Up and Down move, Return picks, Esc closes |
 | Enter, Tab, Shift+Tab | Keep the edit and move down, right or left |
 | Alt+Enter | Open the large editor. In it, Mod+Enter applies and Esc cancels |
 | Esc | Cancel the edit |
diff --git a/src/app.rs b/src/app.rs
--- a/src/app.rs
+++ b/src/app.rs
@@ -155,6 +155,12 @@
     /// The options last chosen for each format of an export and of Copy
     /// as, from export_options.json.
     pub export: crate::export::Options,
+    /// What a copy the app made puts on the clipboard: the frame takes it
+    /// there (`ui::show`), as it has the clipboard.
+    pub to_clipboard: Option<String>,
+    /// What the toast, or the terminal look's status line, says next: the
+    /// frame says it (`ui::show`).
+    pub said: Option<String>,
     /// What was open at the last quit, and each connection's tables as
     /// its window last had them, from session.json.
     pub session: crate::session::Session,
@@ -425,6 +431,8 @@
             table_lists,
             table_lists_unread,
             export,
+            to_clipboard: None,
+            said: None,
             session,
             writes_session,
             recents,
@@ -2175,6 +2183,47 @@
                     && sql.visual_mode().is_some()
                 {
                     sql.range = None;
+                }
+            }
+            Action::OpenCopyAs { tab, pick } => {
+                // Over a grid with a cell to copy, and no other dialog.
+                if self.dialog.is_none() && self.copy_in(tab, pick, How::ColumnNames).is_some() {
+                    self.dialog = Some(Dialog::CopyAs(Box::new(crate::model::CopyAsMenu {
+                        tab,
+                        pick,
+                        lit: 0,
+                    })));
+                }
+            }
+            Action::MoveInCopyAs(step) => {
+                if let Some(Dialog::CopyAs(menu)) = &mut self.dialog {
+                    let last = crate::model::CopyHow::menu(menu.pick).len() - 1;
+                    menu.lit = menu.lit.saturating_add_signed(step).min(last);
+                }
+            }
+            Action::CopyAs { tab, pick, how } => {
+                if let Some(copying) = self.copy_in(tab, pick, how) {
+                    // The grid menu's Copy and Copy rows as TSV are ⌘C's
+                    // and ⇧⌘C's, which say nothing of a copy made.
+                    let silent = how == How::Plain && pick != Pick::CursorRow;
+                    match copying.copied {
+                        crate::export::Copied::Text { ref text, .. } => {
+                            if !silent {
+                                self.said = Some(copy_said(self.locale, &copying, how));
+                            }
+                            self.to_clipboard = Some(text.clone());
+                        }
+                        _ => self.said = Some(copy_said(self.locale, &copying, how)),
+                    }
+                }
+                // As `y` leaves a visual selection.
+                let workspace = self.workspace(tab);
+                let shown = workspace.and_then(|workspace| {
+                    let object = workspace.active_object_tab().map(|object| object.id);
+                    object.or_else(|| workspace.active_sql_tab().map(|sql| sql.id))
+                });
+                if let Some(id) = shown {
+                    self.apply(Action::LeaveVisual { tab, id });
                 }
             }
             Action::ToggleRowPanel(tab) => {
@@ -7799,47 +7848,93 @@
     /// What a copy takes of the active tab's grid, a table's or a SQL
     /// result's: with a range, the cells it covers as TSV, a line for each
     /// row; with none, or one cell held by `v`, the cursor's cell's text as
-    /// it is. `whole_row` takes every
-    /// column of the rows the selection covers. What is pending reads as
-    /// its new value, and a new row's cell nothing is set in as empty.
+    /// it is. `whole_row` takes every column of the rows the selection
+    /// covers. What is pending reads as its new value, and a new row's cell
+    /// nothing is set in as a NULL. `None` where nothing is copied: no
+    /// cell, or more than the clipboard takes ([`copy_said`] says which).
     pub fn copy_text(&self, whole_row: bool) -> Option<String> {
-        self.copy_of(whole_row, false)
+        let pick = if whole_row {
+            Pick::Rows
+        } else {
+            Pick::Selection
+        };
+        self.copy(pick, How::Plain)?.text()
     }
 
     /// The cursor's row alone, whole, as `yy` yanks it: a range that
     /// stands is not what it puts.
     pub fn yank_text(&self) -> Option<String> {
-        self.copy_of(true, true)
-    }
-
-    fn copy_of(&self, whole_row: bool, cursor_alone: bool) -> Option<String> {
-        use tabletist_db::Value;
-        let workspace = self.workspace(self.active_tab_id())?;
+        self.copy(Pick::CursorRow, How::Plain)?.text()
+    }
+
+    /// A copy of `pick` in the active tab's grid, a table's or a SQL
+    /// result's, `how` says: as ⌘C copies it (TSV with its remembered
+    /// options and no header row), in a format with its remembered
+    /// options, or the columns' names. A copy in a format of no range is
+    /// the cursor's row whole. `None` where the grid has no cell to copy.
+    pub fn copy(&self, pick: Pick, how: How) -> Option<Copying> {
+        self.copy_in(self.active_tab_id(), pick, how)
+    }
+
+    /// [`App::copy`], of the grid in front in `tab`.
+    pub fn copy_in(&self, tab: ConnTabId, pick: Pick, how: How) -> Option<Copying> {
+        let workspace = self.workspace(tab)?;
+        let dialect = workspace.driver.dialect();
         if let Some(object) = workspace.active_object_tab() {
             let mut selected = object.selected()?;
-            if cursor_alone {
+            if pick == Pick::CursorRow {
                 selected.range = None;
             }
             let page = object.page()?;
-            // The cursor's cell or row alone is copied from the page's
-            // rows: a new row is none of them.
-            if selected.range.is_none() && crate::edit::new_id(selected.cursor.row).is_some() {
+            // As ⌘C, the cursor's cell or row alone is copied from the
+            // page's rows: a new row is none of them. A copy in a format
+            // takes a new row too, what is not set in it NULL.
+            let new = crate::edit::new_id(selected.cursor.row).is_some();
+            if how == How::Plain && selected.range.is_none() && new {
                 return None;
             }
+            // What is pending reads as what it shows; a pending DEFAULT or
+            // time of the save is written in SQL INSERT as a save sends it.
             let value = |row: usize, col: usize| match object.edits.cells.get(&(row, col)) {
-                Some(pending) => Some(crate::edit::shown_value(&pending.new)),
-                None => page.rows.get(row)?.get(col).cloned(),
+                Some(pending) => {
+                    let type_name = page.columns.get(col).map_or("", |column| &column.type_name);
+                    crate::export::Pending {
+                        value: Some(crate::edit::shown_value(&pending.new)),
+                        sql: dialect.pending_literal(&pending.new, type_name),
+                    }
+                }
+                None => crate::export::Pending {
+                    value: page.rows.get(row).and_then(|row| row.get(col)).cloned(),
+                    sql: None,
+                },
             };
-            return copied(&selected, page.columns.len(), whole_row, value);
+            let target = crate::export::Target::Table {
+                object: &object.object,
+                create: None,
+            };
+            let source = CopySource {
+                columns: &page.columns,
+                dialect,
+                target,
+            };
+            return copying(&selected, source, pick, how, &self.export, value);
         }
         let sql = workspace.active_sql_tab()?;
         let mut selected = sql.selected()?;
-        if cursor_alone {
+        if pick == Pick::CursorRow {
             selected.range = None;
         }
         let (columns, rows, _) = sql.shown_rows()?;
-        let value = |row: usize, col: usize| -> Option<Value> { rows.get(row)?.get(col).cloned() };
-        copied(&selected, columns.len(), whole_row, value)
+        let value = |row: usize, col: usize| crate::export::Pending {
+            value: rows.get(row).and_then(|row| row.get(col)).cloned(),
+            sql: None,
+        };
+        let source = CopySource {
+            columns,
+            dialect,
+            target: crate::export::Target::Named("result"),
+        };
+        copying(&selected, source, pick, how, &self.export, value)
     }
 
     /// The palette and the look, applied to egui's style, with the caret
@@ -8518,35 +8613,148 @@
     moved as usize
 }
 
-/// What [`App::copy_text`] copies of `selected`, in a grid `width` wide
-/// whose cells `value` reads.
-fn copied(
+pub use crate::model::{CopyHow as How, CopyPick as Pick};
+
+/// What a copy put together, and of how much.
+#[derive(Debug, Clone, PartialEq, Eq)]
+pub struct Copying {
+    pub copied: crate::export::Copied,
+    pub rows: usize,
+    pub cols: usize,
+    /// Whether it took whole rows.
+    pub whole_rows: bool,
+}
+
+impl Copying {
+    /// The text for the clipboard, where there is one.
+    pub fn text(self) -> Option<String> {
+        match self.copied {
+            crate::export::Copied::Text { text, .. } => Some(text),
+            _ => None,
+        }
+    }
+}
+
+/// What the toast says of `copying`, made as `how` says: "Copied 4 rows ×
+/// 2 columns as Markdown", "Copied 12 rows as CSV · 1 statement", or, past
+/// the clipboard's limits, why nothing was copied.
+pub fn copy_said(locale: crate::i18n::Locale, copying: &Copying, how: How) -> String {
+    use crate::export::Copied;
+    use crate::i18n::{counted, gettext};
+    match &copying.copied {
+        Copied::TooManyRows(rows) => gettext(locale, "That's {rows} rows. Export to a file?")
+            .replace("{rows}", &crate::format::group_digits(*rows as u64)),
+        Copied::TooLarge => gettext(locale, "That's over 20 MB. Export to a file?").into_owned(),
+        Copied::Text { statements, .. } => {
+            let size = (copying.rows, copying.cols);
+            let what = crate::selection::size_words(locale, size, copying.whole_rows);
+            let copied = gettext(locale, "Copied");
+            match how {
+                How::ColumnNames => {
+                    let names = counted(locale, copying.cols, "column name", "column names");
+                    format!("{copied} {names}")
+                }
+                How::Plain => format!("{copied} {what} {} TSV", gettext(locale, "as")),
+                How::As(format) => {
+                    let said = format!(
+                        "{copied} {what} {} {}",
+                        gettext(locale, "as"),
+                        format.name()
+                    );
+                    if format == crate::export::Format::SqlInsert {
+                        let made = counted(locale, *statements, "statement", "statements");
+                        format!("{said} · {made}")
+                    } else {
+                        said
+                    }
+                }
+            }
+        }
+    }
+}
+
+/// The grid a copy reads: its columns, its connection's dialect, and the
+/// table SQL INSERT writes into.
+struct CopySource<'a> {
+    columns: &'a [tabletist_db::ColumnMeta],
+    dialect: tabletist_db::Dialect,
+    target: crate::export::Target<'a>,
+}
+
+/// What [`App::copy`] copies of `selected`, in a grid whose cells `value`
+/// reads.
+fn copying(
     selected: &crate::selection::Selection<'_>,
-    width: usize,
-    whole_row: bool,
-    value: impl Fn(usize, usize) -> Option<tabletist_db::Value>,
-) -> Option<String> {
+    source: CopySource<'_>,
+    pick: Pick,
+    how: How,
+    options: &crate::export::Options,
+    value: impl Fn(usize, usize) -> crate::export::Pending,
+) -> Option<Copying> {
+    use crate::export::{Copied, Format};
+    let width = source.columns.len();
     let cursor = selected.cursor;
     // One cell, alone or held by `v`: its text as it is, tabs and lines in
     // it kept.
-    if !whole_row && selected.size(width).is_none_or(|size| size == (1, 1)) {
-        let value = value(cursor.row, cursor.col)?;
-        return Some(crate::format::plain_text(&value));
-    }
-    let (first, last) = if whole_row {
+    let one = selected.size(width).is_none_or(|size| size == (1, 1));
+    if how == How::Plain && pick == Pick::Selection && one {
+        let value = value(cursor.row, cursor.col).value?;
+        let text = crate::format::plain_text(&value);
+        return Some(Copying {
+            copied: Copied::Text {
+                text,
+                statements: 0,
+            },
+            rows: 1,
+            cols: 1,
+            whole_rows: false,
+        });
+    }
+    let whole_rows = pick != Pick::Selection || selected.range.is_none_or(|range| range.rows());
+    let (first, last) = if whole_rows {
         (0, width.checked_sub(1)?)
     } else {
         selected.cols(width)?
     };
-    let lines: Vec<String> = selected
-        .rows()
-        .into_iter()
-        .map(|row| {
-            let cells = (first..=last).map(|col| value(row, col));
-            crate::format::tsv_cells(cells)
+    let columns = &source.columns[first..=last];
+    let names: Vec<&str> = columns.iter().map(|column| column.name.as_str()).collect();
+    let rows = selected.rows();
+    let (count, cols) = (rows.len(), names.len());
+    let done = |copied| {
+        Some(Copying {
+            copied,
+            rows: count,
+            cols,
+            whole_rows,
         })
-        .collect();
-    Some(lines.join("\n"))
+    };
+    let (format, options) = match how {
+        How::ColumnNames => {
+            let text = crate::export::column_names(&names, source.dialect);
+            return done(Copied::Text {
+                text,
+                statements: 0,
+            });
+        }
+        How::As(format) => (format, std::borrow::Cow::Borrowed(options)),
+        How::Plain => {
+            let mut plain = options.clone();
+            plain.tsv.header = false;
+            (Format::Tsv, std::borrow::Cow::Owned(plain))
+        }
+    };
+    let written = crate::export::Columns {
+        names,
+        kinds: columns.iter().map(|column| column.kind).collect(),
+        dialect: source.dialect,
+    };
+    let cells = rows.into_iter().map(|row| {
+        (first..=last)
+            .map(|col| value(row, col))
+            .collect::<Vec<_>>()
+    });
+    let copied = crate::export::clipboard(format, &options, written, source.target, count, cells);
+    done(copied)
 }
 
 /// Where a move by `rows` and `cols` puts the cursor of a SQL result's
diff --git a/src/ui/copy_as.rs b/src/ui/copy_as.rs
new file mode 100644
--- /dev/null
+++ b/src/ui/copy_as.rs
@@ -0,0 +1,326 @@
+//! Copy as: a grid's selection, or the row panel's row, on the clipboard
+//! in a format. ⌥⌘C and `gy` open its menu at the cursor's cell, one
+//! letter a format; a secondary click on a cell opens the grid's menu,
+//! whose Copy as › holds the same rows. The copy itself is the app's
+//! (`Action::CopyAs`): this draws and reads the keys.
+
+use egui::{Area, Id, Order, Rect, Ui, pos2, vec2};
+
+use crate::i18n::gettext;
+use crate::keymap::{Command, Layout, Scope, When};
+use crate::model::{Action, CopyAsMenu, CopyHow, CopyPick};
+use crate::typography::{Text, TextRole};
+use crate::ui::{Paint, menu, widgets};
+
+/// What a row of the grid's menu asks for.
+#[derive(Debug, Clone, Copy, PartialEq, Eq)]
+pub enum Chosen {
+    /// Copy: as ⌘C.
+    Copy,
+    /// Copy rows as TSV: as ⇧⌘C.
+    CopyRows,
+    /// A row of Copy as ›.
+    As(CopyHow),
+}
+
+impl Chosen {
+    /// The copy it asks for, of the grid in front in `tab`.
+    pub fn action(self, tab: crate::model::ConnTabId) -> Action {
+        let (pick, how) = match self {
+            Self::Copy => (CopyPick::Selection, CopyHow::Plain),
+            Self::CopyRows => (CopyPick::Rows, CopyHow::Plain),
+            Self::As(how) => (CopyPick::Selection, how),
+        };
+        Action::CopyAs { tab, pick, how }
+    }
+}
+
+fn anchor_id() -> Id {
+    Id::new("copy-as-anchor")
+}
+
+/// Keeps where the cursor's cell is drawn: the menu opens under it.
+pub fn keep_anchor(ctx: &egui::Context, cell: Rect) {
+    ctx.data_mut(|data| data.insert_temp(anchor_id(), cell));
+}
+
+/// A row's name: the format's ("Markdown"), or "Column names". On the row
+/// panel each reads "Copy row as …".
+pub fn row_name(how: CopyHow, row: bool, locale: crate::i18n::Locale) -> String {
+    let name = match how {
+        CopyHow::As(format) => format.name().to_owned(),
+        CopyHow::Plain => crate::export::Format::Tsv.name().to_owned(),
+        CopyHow::ColumnNames => gettext(locale, "Column names").into_owned(),
+    };
+    if row {
+        format!("{} {name}", gettext(locale, "Copy row as"))
+    } else {
+        name
+    }
+}
+
+/// Draws the Copy as menu, reads its keys, and returns what it asks for.
+/// `size` is what it copies, in words ("4 rows × 2 columns"), for the
+/// terminal look's title.
+/// Reads the menu's keys, before anything else in the frame reads them: a
+/// field under it would take Enter. Returns what they ask for.
+pub fn keys(ctx: &egui::Context, paint: Paint, open: &CopyAsMenu) -> Vec<Action> {
+    keep_keyboard(ctx);
+    let (tab, pick) = (open.tab, open.pick);
+    let rows = CopyHow::menu(pick);
+    let lit = open.lit.min(rows.len() - 1);
+    let copy = |how: CopyHow| vec![Action::CloseDialog, Action::CopyAs { tab, pick, how }];
+    let (keymap, layout) = (crate::ui::keys::published(ctx), Layout::of(&paint.look));
+    // The letters, as typed text, by the keymap.
+    let typed = ctx.input_mut(|input| {
+        let mut typed = Vec::new();
+        input.events.retain(|event| match event {
+            egui::Event::Text(text) => {
+                match crate::ui::keys::prompt_letter(&keymap, layout, When::CopyAs, text) {
+                    Some(command) => {
+                        typed.push(command);
+                        false
+                    }
+                    None => true,
+                }
+            }
+            _ => true,
+        });
+        typed
+    });
+    let picked = typed
+        .iter()
+        .find_map(|command| rows.iter().find(|(_, letter)| letter == command))
+        .map(|(how, _)| *how);
+    let moved = |command: Command| {
+        ctx.input_mut(|input| {
+            crate::ui::keys::asked(input, &keymap, layout, command, Scope::Prompt, true).count
+        })
+    };
+    let step = moved(Command::CopyAsNext) as isize - moved(Command::CopyAsPrevious) as isize;
+    let actions = if let Some(how) = picked {
+        copy(how)
+    } else if moved(Command::CopyAsPick) > 0 {
+        copy(rows[lit].0)
+    } else if moved(Command::CloseCopyAs) > 0 {
+        vec![Action::CloseDialog]
+    } else if step != 0 {
+        vec![Action::MoveInCopyAs(step)]
+    } else {
+        Vec::new()
+    };
+    give_keyboard_back(ctx, &actions);
+    actions
+}
+
+/// Draws the Copy as menu at the cursor's cell, and returns what a click
+/// asks for. `size` is what it copies, in words ("4 rows × 2 columns"),
+/// for the terminal look's title.
+pub fn show(ctx: &egui::Context, paint: Paint, open: &CopyAsMenu, size: &str) -> Vec<Action> {
+    let Paint {
+        locale,
+        palette,
+        look,
+    } = paint;
+    let (tab, pick) = (open.tab, open.pick);
+    let rows = CopyHow::menu(pick);
+    let lit = open.lit.min(rows.len() - 1);
+    // Over the row panel's row it is the row's.
+    let name = if pick == CopyPick::CursorRow {
+        "Copy row as"
+    } else {
+        "Copy as"
+    };
+    let mut actions = Vec::new();
+    let anchor = ctx
+        .data(|data| data.get_temp::<Rect>(anchor_id()))
+        .unwrap_or_else(|| Rect::from_center_size(ctx.content_rect().center(), vec2(0.0, 0.0)));
+    let area = Area::new(Id::new("copy-as"))
+        .order(Order::Foreground)
+        .kind(egui::UiKind::Menu)
+        .fixed_pos(pos2(anchor.left(), anchor.bottom() + 4.0))
+        .show(ctx, |ui| {
+            menu::frame(&look, &palette).show(ui, |ui| {
+                ui.spacing_mut().item_spacing = egui::Vec2::ZERO;
+                ui.set_min_width(if look.terminal { 300.0 } else { 222.0 });
+                if look.terminal {
+                    let title = look.label(&gettext(locale, name));
+                    title_line(ui, paint, &title, size);
+                }
+                for (index, (how, letter)) in rows.iter().enumerate() {
+                    let key = crate::ui::keys::written(ctx, &look, *letter);
+                    let name = row_name(*how, false, locale);
+                    let shown = if look.terminal {
+                        format!("{key}  {}", look.label(&name))
+                    } else {
+                        name.clone()
+                    };
+                    let mut item = menu::Item::action(&shown)
+                        .name(&name)
+                        .kept_out(index == lit);
+                    let upper = key.as_str().to_uppercase();
+                    if !look.terminal {
+                        item = item.detail(&upper);
+                    }
+                    if item.show(ui, &look, &palette).clicked() {
+                        actions.push(Action::CloseDialog);
+                        actions.push(Action::CopyAs {
+                            tab,
+                            pick,
+                            how: *how,
+                        });
+                    }
+                }
+                if look.terminal {
+                    foot_line(ui, paint);
+                }
+            });
+            let place = ui.min_rect();
+            widgets::announce(ui, place, &gettext(locale, name));
+        });
+    if area.response.clicked_elsewhere() && actions.is_empty() {
+        actions.push(Action::CloseDialog);
+    }
+    give_keyboard_back(ctx, &actions);
+    actions
+}
+
+/// Keeps where the keyboard was as the menu opened.
+fn keep_keyboard(ctx: &egui::Context) {
+    if ctx
+        .data(|data| data.get_temp::<Option<Id>>(keyboard_id()))
+        .is_none()
+    {
+        let focused = ctx.memory(|memory| memory.focused());
+        ctx.data_mut(|data| data.insert_temp(keyboard_id(), focused));
+    }
+}
+
+/// Gives the keyboard back where it was, as `actions` close the menu.
+fn give_keyboard_back(ctx: &egui::Context, actions: &[Action]) {
+    if actions
+        .iter()
+        .any(|action| matches!(action, Action::CloseDialog))
+    {
+        let back = ctx.data_mut(|data| data.remove_temp::<Option<Id>>(keyboard_id()));
+        if let Some(Some(id)) = back {
+            ctx.memory_mut(|memory| memory.request_focus(id));
+        }
+    }
+}
+
+/// Forgets where the keyboard was, with no menu open.
+pub fn fold(ctx: &egui::Context) {
+    ctx.data_mut(|data| data.remove_temp::<Option<Id>>(keyboard_id()));
+}
+
+fn keyboard_id() -> Id {
+    Id::new("copy-as-keyboard")
+}
+
+/// The terminal look's title: "copy as", then what it copies.
+fn title_line(ui: &mut Ui, paint: Paint, title: &str, size: &str) {
+    let Paint { palette, look, .. } = paint;
+    let role = TextRole::OBody;
+    let height = role.row_height(ui.ctx(), look.faces) + 2.0;
+    let (rect, _) =
+        ui.allocate_exact_size(vec2(ui.available_width(), height), egui::Sense::hover());
+    let y = rect.center().y;
+    let width = widgets::paint_text(
+        ui,
+        rect.left() + 10.0,
+        y,
+        Text::one(&look, TextRole::OGroup, title, palette.text),
+    );
+    widgets::paint_text(
+        ui,
+        rect.left() + 10.0 + width + 10.0,
+        y,
+        Text::one(&look, role, size, palette.dim),
+    );
+    widgets::hline(ui, rect.x_range(), rect.bottom() - 0.5, palette.outline);
+}
+
+/// The terminal look's keys under the rows: "one key picks · ctrl+n/p
+/// enter · esc close".
+fn foot_line(ui: &mut Ui, paint: Paint) {
+    let Paint {
+        locale,
+        palette,
+        look,
+    } = paint;
+    let ctx = ui.ctx().clone();
+    let moves = crate::ui::keys::written_together(
+        &ctx,
+        &look,
+        &[Command::CopyAsNext, Command::CopyAsPrevious],
+    );
+    let enter = crate::ui::keys::written(&ctx, &look, Command::CopyAsPick);
+    let esc = crate::ui::keys::written(&ctx, &look, Command::CloseCopyAs);
+    let picks = look.label(&gettext(locale, "one key picks"));
+    let close = look.label(&gettext(locale, "close"));
+    let role = widgets::secondary(&look);
+    let height = TextRole::OBody.row_height(&ctx, look.faces) + 2.0;
+    let (rect, _) =
+        ui.allocate_exact_size(vec2(ui.available_width(), height), egui::Sense::hover());
+    widgets::hline(ui, rect.x_range(), rect.top() + 0.5, palette.outline);
+    let y = rect.center().y;
+    let x = rect.left() + 10.0;
+    let width = widgets::paint_text(ui, x, y, Text::one(&look, role, &picks, palette.dim));
+    let hints: [widgets::Hint<'_>; 2] = [(&moves, enter.as_str(), true), (&esc, &close, true)];
+    widgets::key_hints(ui, (x + width + 14.0, y), &hints, 14.0, &look, &palette);
+}
+
+/// The grid's menu: Copy, Copy as › (its rows, with their letters), and
+/// Copy rows as TSV. Returns what was picked.
+pub fn context_rows(ui: &mut Ui, paint: Paint) -> Option<Chosen> {
+    let Paint {
+        locale,
+        palette,
+        look,
+    } = paint;
+    let ctx = ui.ctx().clone();
+    let mut chosen = None;
+    let label = |text: &'static str| look.label(&gettext(locale, text));
+    let copy_key = crate::ui::keys::written_in(&ctx, &look, Command::CopyCells, Scope::Grid);
+    let copy = label("Copy");
+    let mut item = menu::Item::action(&copy);
+    if !look.terminal {
+        item = item.detail(copy_key.as_str());
+    }
+    if item.show(ui, &look, &palette).clicked() {
+        chosen = Some(Chosen::Copy);
+    }
+    let copy_as = label("Copy as");
+    let button = menu::Item::action(&copy_as)
+        .detail("›")
+        .show(ui, &look, &palette);
+    egui::containers::menu::SubMenu::new().show(ui, &button, |ui| {
+        ui.spacing_mut().item_spacing = egui::Vec2::ZERO;
+        ui.set_min_width(180.0);
+        for (how, letter) in CopyHow::MENU {
+            let key = crate::ui::keys::written(&ctx, &look, letter);
+            // As the look writes a key: `C` on macOS, `c` in the terminal.
+            let letter = if look.terminal {
+                key.to_string()
+            } else {
+                key.as_str().to_uppercase()
+            };
+            let name = look.label(&row_name(how, false, locale));
+            let item = menu::Item::action(&name).detail(&letter);
+            if item.show(ui, &look, &palette).clicked() {
+                chosen = Some(Chosen::As(how));
+            }
+        }
+    });
+    let rows = label("Copy rows as TSV");
+    let rows_key = crate::ui::keys::written_in(&ctx, &look, Command::CopyRow, Scope::Grid);
+    let mut item = menu::Item::action(&rows);
+    if !rows_key.as_str().is_empty() {
+        item = item.detail(rows_key.as_str());
+    }
+    if item.show(ui, &look, &palette).clicked() {
+        chosen = Some(Chosen::CopyRows);
+    }
+    chosen
+}
diff --git a/src/ui/data_view.rs b/src/ui/data_view.rs
--- a/src/ui/data_view.rs
+++ b/src/ui/data_view.rs
@@ -1846,6 +1846,8 @@
         // grid of the fit before kept goes when this one is drawn.
         let id = grid_id(tab, object_tab, fit);
         grid::keep(ui.ctx(), table_id(tab, object_tab), id);
+        // A secondary click's menu: Copy, Copy as ›, Copy rows as TSV.
+        let menu_rows = |ui: &mut egui::Ui| crate::ui::copy_as::context_rows(ui, paint);
         let output = grid::show(
             ui,
             id,
@@ -1855,6 +1857,7 @@
             grid::Picked {
                 block,
                 said: said.as_deref(),
+                menu: Some(grid::Menu(&menu_rows)),
             },
             keys,
             &palette,
@@ -2028,6 +2031,13 @@
         if let Some(cell) = output.dragged.and_then(held) {
             actions.push(Action::ExtendTo { tab, id, cell });
         }
+        // A secondary click outside the selection selects its cell.
+        if let Some(cell) = output.context_clicked.and_then(held) {
+            actions.push(Action::SelectCell { tab, id, cell });
+        }
+        if let Some(chosen) = output.chosen {
+            actions.push(chosen.action(tab));
+        }
         if let Some((place, click)) = output.row_clicked
             && let Some(row) = order.row(place)
         {
diff --git a/src/ui/grid.rs b/src/ui/grid.rs
--- a/src/ui/grid.rs
+++ b/src/ui/grid.rs
@@ -112,6 +112,19 @@
     /// What a screen reader is told of the range: "4 rows by 2 columns
     /// selected".
     pub said: Option<&'a str>,
+    /// The rows of the menu a secondary click on a cell opens, and what
+    /// was picked in them.
+    pub menu: Option<Menu<'a>>,
+}
+
+/// The rows of a grid's menu: drawn while it is open, and what was picked.
+#[derive(Clone, Copy)]
+pub struct Menu<'a>(pub &'a dyn Fn(&mut Ui) -> Option<crate::ui::copy_as::Chosen>);
+
+impl std::fmt::Debug for Menu<'_> {
+    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
+        f.write_str("Menu")
+    }
 }
 
 /// How a cell draws its text.
@@ -204,6 +217,11 @@
     pub shift: bool,
     /// The cell a second click landed on, soon after the first.
     pub double_clicked: Option<CellPos>,
+    /// A cell outside the selection had a secondary click: it is selected
+    /// before its menu acts.
+    pub context_clicked: Option<CellPos>,
+    /// What was picked in the grid's menu.
+    pub chosen: Option<crate::ui::copy_as::Chosen>,
     /// Where the cell being edited is, in view or not: what an editor that
     /// does not sit on the cell is anchored to.
     pub editing_rect: Option<Rect>,
@@ -947,6 +965,32 @@
                 if response.double_clicked() {
                     output.double_clicked = cell;
                 }
+            }
+            // A secondary click on a cell keeps a selection it falls in,
+            // and selects the cell otherwise; on a row's number, the row.
+            // Then the menu opens, over what is selected.
+            if response.secondary_clicked()
+                && let Some(pointer) = pressed
+            {
+                if on_number(pointer) {
+                    if !ranged {
+                        output.row_clicked = Some((row, crate::selection::RowClick::Only));
+                    }
+                } else {
+                    let col = under(pointer).col;
+                    let covered = picked.block.is_some_and(|block| block.covers(row, col))
+                        || ranged
+                        || selection == Some(CellPos { row, col });
+                    if !covered {
+                        output.context_clicked = Some(CellPos { row, col });
+                    }
+                }
+            }
+            if let Some(menu) = picked.menu
+                && let Some(chosen) =
+                    crate::ui::menu::context(&response, look, palette, menu.0).flatten()
+            {
+                output.chosen = Some(chosen);
             }
             // A drag from a cell: the press is its click, and the cell the
             // pointer is over the other end of the range.
@@ -1120,6 +1164,10 @@
                 }
                 let content = cell(row, col);
                 let here = selection == Some(CellPos { row, col });
+                // Where Copy as opens from the keyboard.
+                if here {
+                    crate::ui::copy_as::keep_anchor(ui.ctx(), cell_rect);
+                }
                 let block = picked.block.filter(|block| block.covers(row, col));
                 let in_block = block.is_some();
                 // A range of cells: the accent's faint tint on macOS, the
diff --git a/src/ui/keys.rs b/src/ui/keys.rs
--- a/src/ui/keys.rs
+++ b/src/ui/keys.rs
@@ -1317,19 +1317,30 @@
         // They come as the window's copy, which `presses` reads. A look
         // with no chord for them copies nothing here. A table's grid or a
         // result's.
+        // Copy as's chord too: it holds Alt with the copy.
         let scope = if grid { Scope::Grid } else { Scope::Results };
         let copied = ctx.input_mut(|input| {
-            [Command::CopyRow, Command::CopyCells]
+            [Command::CopyAs, Command::CopyRow, Command::CopyCells]
                 .into_iter()
                 .find(|command| {
                     asked(input, &app.keymap, layout, *command, scope, normal).count > 0
                 })
         });
-        let row = copied == Some(Command::CopyRow);
-        if copied.is_some()
-            && let Some(text) = app.copy_text(row)
-        {
-            ctx.copy_text(text);
+        match copied {
+            Some(Command::CopyAs) => {
+                let pick = crate::model::CopyPick::Selection;
+                let tab = app.active_tab_id();
+                actions.push(Action::OpenCopyAs { tab, pick });
+            }
+            Some(command) => {
+                let pick = if command == Command::CopyRow {
+                    crate::model::CopyPick::Rows
+                } else {
+                    crate::model::CopyPick::Selection
+                };
+                copy_plain(ctx, app, pick);
+            }
+            None => {}
         }
     }
     // The Settings window, wherever the keyboard is.
@@ -2108,6 +2119,22 @@
     let place = object.selection.and_then(|cell| order.place(cell.row));
     object.view == crate::model::ObjectView::Data
         && place.is_some_and(|place| place + 1 == order.len())
+}
+
+/// Copies `pick` of the grid in front as ⌘C does, saying nothing: past the
+/// clipboard's limits it copies nothing and says why.
+fn copy_plain(ctx: &egui::Context, app: &App, pick: crate::model::CopyPick) {
+    let how = crate::model::CopyHow::Plain;
+    let Some(copying) = app.copy(pick, how) else {
+        return;
+    };
+    match copying.copied {
+        crate::export::Copied::Text { text, .. } => ctx.copy_text(text),
+        _ => {
+            let said = crate::app::copy_said(app.locale, &copying, how);
+            crate::ui::toast::say(ctx, &said);
+        }
+    }
 }
 
 /// Whether a visual selection holds `command` back: what changes or
@@ -3177,10 +3204,23 @@
             // is the page's.
             (Some(Command::YankRow), Some(id)) if rows_show && !tree && selection.is_some() => {
                 actions.push(Action::YankRow { tab, id });
-                if let Some(text) = app.yank_text() {
-                    ctx.copy_text(text);
-                }
-                let said = crate::i18n::gettext(app.locale, "Row yanked");
+                let how = crate::model::CopyHow::Plain;
+                let yanked = app.copy(crate::model::CopyPick::CursorRow, how);
+                // Past the clipboard's limits it is yanked, and not put
+                // there: the toast says why.
+                let said = match yanked {
+                    Some(copying)
+                        if !matches!(copying.copied, crate::export::Copied::Text { .. }) =>
+                    {
+                        crate::app::copy_said(app.locale, &copying, how)
+                    }
+                    yanked => {
+                        if let Some(text) = yanked.and_then(crate::app::Copying::text) {
+                            ctx.copy_text(text);
+                        }
+                        crate::i18n::gettext(app.locale, "Row yanked").into_owned()
+                    }
+                };
                 crate::ui::toast::say(ctx, &app.look.label(&said));
             }
             // A copy of the yanked row below the cursor's, or above it
@@ -3321,12 +3361,20 @@
             // A visual selection's cells, of the rows or a result, and as
             // vim's `y` does the selection is left.
             (Some(Command::CopySelection), _) if active.is_none() || rows_show => {
-                if let Some(text) = app.copy_text(false) {
-                    ctx.copy_text(text);
-                }
+                copy_plain(ctx, app, crate::model::CopyPick::Selection);
                 if let Some(id) = active.or(sql_grid).filter(|_| visual) {
                     actions.push(Action::LeaveVisual { tab, id });
                 }
+            }
+            // Copy as, over the selection of the rows or a result.
+            (Some(Command::CopyAs), _) if active.is_none() || rows_show => {
+                let pick = crate::model::CopyPick::Selection;
+                actions.push(Action::OpenCopyAs { tab, pick });
+            }
+            // Copy row as, from a field of the row panel.
+            (Some(Command::CopyRowAs), Some(_)) if rows_show && selection.is_some() => {
+                let pick = crate::model::CopyPick::CursorRow;
+                actions.push(Action::OpenCopyAs { tab, pick });
             }
             // The whole statements, never the lines as the panel shows
             // them. The copy shows nowhere, so the status line says it.
diff --git a/src/ui/mod.rs b/src/ui/mod.rs
--- a/src/ui/mod.rs
+++ b/src/ui/mod.rs
@@ -27,6 +27,7 @@
 mod complete_tests;
 pub mod conflict_prompt;
 pub mod connect_dialog;
+pub mod copy_as;
 #[cfg(test)]
 mod copy_as_tests;
 pub mod data_view;
@@ -164,6 +165,19 @@
 }
 
 pub fn show(app: &mut App, ui: &mut egui::Ui) {
+    // What the app's last actions copied and said: the clipboard and the
+    // toast are the frame's.
+    if let Some(text) = app.to_clipboard.take() {
+        ui.ctx().copy_text(text);
+    }
+    if let Some(said) = app.said.take() {
+        toast::say(ui.ctx(), &said);
+    }
+    // The Copy as menu's keys, before any view reads them.
+    if let Some(Dialog::CopyAs(open)) = &app.dialog {
+        let asked = copy_as::keys(ui.ctx(), Paint::of(app), open);
+        app.actions.extend(asked);
+    }
     let fill = app.palette.window;
     egui::CentralPanel::default()
         .frame(Frame::new().fill(fill))
@@ -248,8 +262,26 @@
         let asked = agent_access::show(&ui.ctx().clone(), paint, prompt, current);
         app.actions.extend(asked);
     }
+    if let Some(Dialog::CopyAs(open)) = &app.dialog {
+        let size = copy_as_size(app, open);
+        let asked = copy_as::show(&ui.ctx().clone(), paint, open, &size);
+        app.actions.extend(asked);
+    } else {
+        copy_as::fold(ui.ctx());
+    }
     // Last: it says what was done, whatever is on screen by now.
     toast::show(ui.ctx(), paint);
+}
+
+/// What the Copy as menu copies, in words: "4 rows × 2 columns".
+fn copy_as_size(app: &App, open: &crate::model::CopyAsMenu) -> String {
+    let how = crate::model::CopyHow::ColumnNames;
+    app.copy_in(open.tab, open.pick, how)
+        .map(|copying| {
+            let size = (copying.rows, copying.cols);
+            crate::selection::size_words(app.locale, size, copying.whole_rows)
+        })
+        .unwrap_or_default()
 }
 
 /// What quick open over `open.tab` asks of the app: none where that
diff --git a/src/ui/row_panel.rs b/src/ui/row_panel.rs
--- a/src/ui/row_panel.rs
+++ b/src/ui/row_panel.rs
@@ -894,6 +894,45 @@
                         }
                     });
                 }
+                // Copy row ▾, left of them: the row in a format. A new row
+                // is no row of the page's to copy.
+                if fresh.is_none() {
+                    let text = format!("{} ▾", gettext(locale, "Copy row"));
+                    let button = widgets::ButtonSpec::new(&text).icon(Icon::Copy);
+                    let width = button.width(ui, &look);
+                    let at = Rect::from_min_size(pos2(right - width, y - 14.0), vec2(width, 28.0));
+                    let response = button.show_at(ui, at, &look, &palette);
+                    // As the board lists them: TSV first, as ⇧⌘C copies it.
+                    let order = {
+                        use crate::export::Format;
+                        use crate::model::CopyHow::{As, Plain};
+                        [
+                            Plain,
+                            As(Format::Json),
+                            As(Format::Csv),
+                            As(Format::Markdown),
+                            As(Format::SqlInsert),
+                        ]
+                    };
+                    let rows = || {
+                        order
+                            .iter()
+                            .map(|how| crate::ui::menu::Choice {
+                                text: crate::ui::copy_as::row_name(*how, true, locale),
+                                name: None,
+                                selected: false,
+                                disabled: None,
+                            })
+                            .collect()
+                    };
+                    if let Some(picked) =
+                        crate::ui::menu::actions(&response, 0.0, &look, &palette, rows)
+                    {
+                        let pick = crate::model::CopyPick::CursorRow;
+                        let how = order[picked];
+                        actions.push(Action::CopyAs { tab, pick, how });
+                    }
+                }
             }
             // A row no field of which can be edited says why, once, in a
             // line under the header: a lock, and the reason.
diff --git a/src/ui/sql_results.rs b/src/ui/sql_results.rs
--- a/src/ui/sql_results.rs
+++ b/src/ui/sql_results.rs
@@ -1990,6 +1990,13 @@
         Some(crate::selection::said(locale, size, whole_rows))
     });
     // The grid asks for the cells in view only.
+    // A secondary click's menu: Copy, Copy as ›, Copy rows as TSV.
+    let paint = crate::ui::Paint {
+        locale,
+        palette: *palette,
+        look: *look,
+    };
+    let menu_rows = |ui: &mut egui::Ui| crate::ui::copy_as::context_rows(ui, paint);
     let output = grid::show(
         &mut child,
         id,
@@ -1999,6 +2006,7 @@
         grid::Picked {
             block,
             said: said.as_deref(),
+            menu: Some(grid::Menu(&menu_rows)),
         },
         keys,
         palette,
@@ -2043,6 +2051,14 @@
     if let Some(cell) = output.dragged {
         let id = sql.id;
         actions.push(Action::ExtendTo { tab, id, cell });
+    }
+    // A secondary click outside the selection selects its cell.
+    if let Some(cell) = output.context_clicked {
+        let id = sql.id;
+        actions.push(Action::SelectCell { tab, id, cell });
+    }
+    if let Some(chosen) = output.chosen {
+        actions.push(chosen.action(tab));
     }
     if let Some((row, click)) = output.row_clicked {
         let id = sql.id;
diff --git a/src/ui/workspace.rs b/src/ui/workspace.rs
--- a/src/ui/workspace.rs
+++ b/src/ui/workspace.rs
@@ -2605,6 +2605,7 @@
     let (visual_delete_key, leave_visual_key) =
         (key(Command::DeleteSelection), key(Command::LeaveVisual));
     let visual_copy_key = key(Command::CopySelection);
+    let copy_as_key = key(Command::CopyAs);
     // The keys of a foreign key's list, while one is open under the field.
     let pick_keys = keymap.together(layout, &[Command::NextPick, Command::PreviousPick]);
     let (set_key, close_key) = (key(Command::CommitDown), key(Command::ClosePicker));
@@ -2808,6 +2809,7 @@
                 (&filter_key, "filter", true),
                 (&sidebar_key, "tables", true),
                 (&copy_key, "copy", true),
+                (&copy_as_key, "copy as", true),
             ]);
             // The prompt's line that saves, where a save can be made.
             let write = gettext(locale, "write");
@@ -3085,12 +3087,17 @@
                     left += widgets::paint_label(ui, left, y, mode) + gap;
                     let text = Text::one(&look, role, held, palette.secondary);
                     left += widgets::paint_label(ui, left, y, text) + gap;
-                    let (copies, normal) = (gettext(locale, "copy"), gettext(locale, "normal"));
+                    let (copies, normal) = (gettext(locale, "copy tsv"), gettext(locale, "normal"));
+                    let copies_as = gettext(locale, "copy as");
                     let keys = || {
                         Text::new(&look)
                             .add(role, &visual_copy_key, palette.text)
                             .space(role, " ")
                             .add(role, &copies, palette.dim)
+                            .add(role, " · ", palette.dim)
+                            .add(role, &copy_as_key, palette.text)
+                            .space(role, " ")
+                            .add(role, &copies_as, palette.dim)
                             .add(role, " · ", palette.dim)
                             .add(role, &leave_visual_key, palette.text)
                             .space(role, " ")
@@ -3151,16 +3158,26 @@
                 let (act_key, acts) = if rows {
                     (&visual_delete_key, gettext(locale, "delete"))
                 } else {
-                    (&visual_copy_key, gettext(locale, "copy"))
+                    (&visual_copy_key, gettext(locale, "copy tsv"))
                 };
                 let normal = gettext(locale, "normal");
+                let copies_as = gettext(locale, "copy as");
                 let keys = || {
-                    Text::new(&look)
+                    let keys = Text::new(&look)
                         .add(role, act_key, palette.text)
                         .space(role, " ")
                         .add(role, &acts, palette.dim)
-                        .add(role, " · ", palette.dim)
-                        .add(role, &leave_visual_key, palette.text)
+                        .add(role, " · ", palette.dim);
+                    // A block's cells are copied in a format too.
+                    let keys = if rows {
+                        keys
+                    } else {
+                        keys.add(role, &copy_as_key, palette.text)
+                            .space(role, " ")
+                            .add(role, &copies_as, palette.dim)
+                            .add(role, " · ", palette.dim)
+                    };
+                    keys.add(role, &leave_visual_key, palette.text)
                         .space(role, " ")
                         .add(role, &normal, palette.dim)
                 };
```

- [ ] **Step 4: Run the checks**

Run: `~/.cargo/bin/cargo fmt --check && ~/.cargo/bin/cargo clippy --workspace --all-targets -- -D warnings && ~/.cargo/bin/cargo test --locked --workspace --all-targets`

Expected: every check passes: `fmt` and `clippy` say nothing, and every test passes in 30 test binaries, none failing (4,854: this task adds seventeen and takes out the old TSV writer's one).

- [ ] **Step 5: Commit**

```bash
git add -A crates src docs
git commit -m "Copy a selection as TSV, CSV, JSON, Markdown or SQL INSERT"
```

## After Task 2

Push the branch and open the pull request: its description gives a summary, the writers and their options, the keys and the keymap check, the departures (D1 to D9, B1 to B14) and the decisions (Q1 to Q13). No screenshot goes to GitHub unblurred.

## Checks by hand

On the Bookshop demo database, both looks: select a block, Mod+Alt+C or `gy`, each letter, and paste into a text editor; right-click a cell outside the selection; Copy row ▾ in the row panel; Mod+A on a page of more than 10,000 rows and Mod+C.
