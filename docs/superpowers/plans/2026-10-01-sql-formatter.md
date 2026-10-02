# SQL Editor: Format Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** `Mod+Shift+F` (and a Format button in the macOS and standard looks) lays the SQL editor's queries out in river style and uppercases reserved words, as one undo step, without changing any token but whitespace.

**Architecture:** `tabletist-db` gains `sql::format`, a formatter over the tokens `sql::tokenize` already makes: it never parses, it copies every token and writes the whitespace between them, and it checks its own result by tokenizing it again. In the app, `Action::FormatSql` sets a flag on the `SqlTab`; the editor's view (`ui::sql_text`) acts on it, because the selection and the undo history live in egui's memory, and writes the typed and the formatted text into the field's undo history.

**Tech Stack:** Rust 2024 (rust-version 1.98), egui 0.36 (crmne fork: `TextEditState::undoer`, `set_undoer`, `Undoer::add_undo`, `CCursor`, `CCursorRange`). No new dependency (`log` is already one of `tabletist-db`'s).

**Spec:** `docs/superpowers/specs/2026-10-01-sql-formatter-design.md`

## How this plan was checked

Every piece of code below was compiled and its tests were run in a scratch copy of the repository while planning: `cargo fmt --check`, `cargo clippy -D warnings`, `cargo doc -D warnings` and `cargo test --workspace --all-targets` (737 app tests, 202 `tabletist-db` unit tests, as the branch ended) pass with all of it applied. The formatter was also run over more than a million scripts built at random, with the safety check out of the way. That and a review of the plan found three faults, each fixed and under test: a space added before a `--` comment fused two minus signs into a comment on MySQL; a line break after two touching minus signs did the same; and a `WITH` statement formatted through a selection, where it did not start its line, gained two spaces on every press. The reviews of the tasks as they were built added more, all folded back into this plan: a cap on how deep nested queries are followed (the layout recurses once for each), a bound on a quadratic scan of join leaders, the redo stack that Format left in place, scrolling the cursor into view after Format, and, from the final review, a word parted from the sigil it touches (`@from` on MySQL), which the token check could not see.

Not checked, so check it when you get there:

- The MySQL server test (Task 2): no server was reachable while planning. It compiled and printed "skipped". The list of uppercased words was written from the reserved-word lists as remembered; the server is what settles it.
- The look of the Format button on macOS and Windows: only the standard look's layout was run, headless. Say which platforms were only compiled.

## Deviations from the spec

Found while building the prototype; the spec was changed to say the same.

- The indentation of the formatted part's first line is Format's: a statement indented on its line starts at column 0. (Without this a second pass moved a leading `WITH`, which is padded to the river.)
- A comment after code keeps a space only where one was typed. Adding one can turn MySQL's `1---- x` (two minus signs, then a comment) into a comment that starts earlier.
- "A word that touches a `.`" is any word beside a `.`, whitespace between them or not: the wider rule is the safe one.
- `STRAIGHT_JOIN` is not a head anywhere in the `SELECT` clause, not only before the list's first item.
- A `,` that would follow a `--` comment goes on the next line, as `)` and `;` do.
- `UPPERCASED` is public, so the MySQL integration test can ask the server about each word.
- A word that touches a `@`, a `:`, a `$` or a number before it is a name: no head, its case kept. The tokenizer cuts `@from` in two where a server reads one token. The same for a word that a `$` or a number starting with `.` touches from the right (`limit$x`, `offset.1st`).
- Queries nested more than 64 blocks deep stay on their line, and a join head has at most three leaders.
- On MySQL a token that touches two touching minus signs stays on their line, whatever it is: a line break after `--` would make a comment of them.
- A selected statement that does not start its line goes on from the column it stands at: its first head is not padded to the river, and columns on its first line count from there.

## Global Constraints

- Work on this worktree's branch (`claude/sql-editor-formatter-539388`). One topic per commit, each passing the checks.
- Checks before every commit (AGENTS.md): `cargo fmt --all --check`, `cargo clippy --locked --workspace --all-targets -- -D warnings`, `cargo test --locked --workspace --all-targets`, `RUSTDOCFLAGS='-D warnings' cargo doc --locked --workspace --no-deps`. Use `~/.cargo/bin/cargo` (the mise shim fails).
- No dependency changes: `Cargo.toml` and `Cargo.lock` stay as they are. If cargo wants to touch `Cargo.lock`, something is wrong; stop and look.
- Database integration tests need servers: `docker compose up -d --build --wait postgres mysql`, then `TABLETIST_TEST_PG_URL=postgres://tabletist:tabletist@localhost:55432/tabletist TABLETIST_TEST_MYSQL_URL=mysql://tabletist:tabletist@localhost:53306/tabletist ~/.cargo/bin/cargo test --locked --workspace`. Without the variables those tests print "skipped" and pass; say so when reporting.
- Commits are signed. If the SSH agent refuses, commit with `git -c commit.gpgsign=false commit` and note it; they are re-signed before push. Never set `SSH_AUTH_SOCK` in git commands.
- End every commit message with `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`. Subject lines follow the repo: an imperative sentence, no prefix ("Split SQL scripts into statements").
- Views push `Action`s; `App::apply` reduces them. A view may edit the text a field is editing (the SQL text, and the cursor offset it reports), nothing else. Format replaces that text from the view, which is this exception.
- Views draw text only through `TextRole`s; never name a font, family or size.
- User-facing strings go through `gettext`. Never use em dashes anywhere (code, strings, docs, commits).
- No design or pixel conformance checks in any test. Compare with the "SQL editor" artboards by hand; screenshots stay local and use the Bookshop demo data (never names from the design's data).
- The SQL text is never logged: `Formatted` has no `Debug`, and the one log line Format writes has no text in it.
- Add a focused regression test for every behaviour. Do not weaken lints or add `allow`s without saying why.

## Review Focus

1. **No token changes.** The formatter's tests check the layout itself, before the safety check that would hide a fault (`formatted` in `sql/format.rs`'s tests calls `laid_out`), over every case, the awkward scripts and the scripts built at random.
2. **Idempotent.** The same helper formats every result a second time and expects `None`.
3. **Case on MySQL.** A word is uppercased only from `UPPERCASED`, never beside a `.`, and the structural words (`OFFSET`, `FULL`, ...) keep their case there. Task 2 asks the server.
4. **One undo.** `one_undo_gives_back_the_script_as_typed` (Task 4): a `Mod+Z` right after Format restores the typed text, and formatting a formatted script adds no undo step. `format_leaves_nothing_to_redo_as_an_edit_does`: Format empties the redo stack.
5. **Keys.** `Mod+Shift+F` is consumed on every tab, so it never reaches `Mod+F` (`command_shift_f_does_nothing_on_a_table_tab`, Task 3).
6. **The toolbar gives way in order**: keys, note, Format, the menus' words, chevrons, menus (Task 5).

---

## File Structure

```
crates/tabletist-db/src/sql.rs          pub mod format
crates/tabletist-db/src/sql/format.rs   format, Formatted, UPPERCASED and the layout (new)
crates/tabletist-db/tests/mysql.rs      the server's word on UPPERCASED, and a name beside a dot
src/model.rs                            Action::FormatSql, SqlTab.format
src/app.rs                              the reducer: set the flag, focus the editor
src/ui/keys.rs                          Mod+Shift+F, the shortcuts table
src/ui/sql_text.rs                      the editor formats: selection, undo history, cursor
src/ui/widgets.rs                       ButtonSpec::quiet
src/ui/sql_editor.rs                    the Format button and the toolbar's give-way order
src/ui/mod.rs                           headless UI tests
README.md, the spec                     docs
```

`sql/format.rs` is one file on purpose: the layout is one recursive pass whose pieces (`Layout`, `Block`, `Writer`) are not used apart. `sql.rs` keeps the tokenizer and stays as it is, apart from the `mod` line.

---

### Task 1: The formatter (`sql::format`)

**Files:**
- Modify: `crates/tabletist-db/src/sql.rs` (one line)
- Create: `crates/tabletist-db/src/sql/format.rs`

**How it works**, so the code reads easily:

- `format` tokenizes the script, picks the bytes to lay out (`region`: the statements a selection overlaps, else first token to last, and back to the start of the first line when only blanks stand before it), lays them out (`laid_out`), puts the result between the untouched ends, and returns `None` when nothing changed or when `same_tokens` says a token did.
- The layout works on `Item`s: each token that is not whitespace, with the whitespace that stood before it (`space`). Nothing else is known about the original layout.
- `Layout::run` walks the script: comments between statements, and statements. A statement that starts with `SELECT`, `WITH` or a `(` holding a query goes to `query`; any other goes to `plain`, which copies its whitespace (`kept` drops what stood before a line's end).
- `block` lays out one query: a statement's (`top`) or one in parentheses. It stops at the `)` that ends it or at the statement's `;`, and the caller writes that. A `(` that holds a query recurses (`open`), with the column after the `(` as the new block's base.
- `Block.frames` holds what a token is inside of within its block: `Frame::Paren` (a parenthesis that is no block) and `Frame::Case`. Heads count only when it is empty.
- `keyword` handles `CASE`, its `WHEN`/`ELSE`/`END`, and asks `head` whether the word starts a clause; `head` also moves the block on (`clause`, `conditions`, `between`).
- `put` writes one token. Where it goes is a `Place`: `Line(column)` starts a line there; `Inline` follows the token before it, unless a list's comma (`broken`) or a comment that ended its line (`ended`) sends it to the content column. Inline spacing is the spec's "Spacing inside a line".
- `Writer` keeps the output and the current column, counted in characters. `line(column)` starts a line, or uses the current one when it is still empty.
- `cased` uppercases a keyword that is on `UPPERCASED` (or that `put` was told is structure) and does not stand beside a `.` (`named`).
- `dashes` is the one MySQL rule: no line starts right after two touching minus signs.
- `named` is what keeps a word from being read as structure: beside a `.`, touching a sigil or a number before it (`Item::binds`), or touched from the right by what goes on with it (`Item::extends`). The tests' `sigils_hold` checks that such a word still touches its sigil, which the token check cannot see.
- `moved` maps the cursor by counting token bytes before it.

- [ ] **Step 1: Declare the module**

In `crates/tabletist-db/src/sql.rs`, after `use crate::Dialect;`:

```rust
use crate::Dialect;

pub mod format;
```

- [ ] **Step 2: Write the failing tests**

Create `crates/tabletist-db/src/sql/format.rs` with the module's doc comment and its tests:

```rust
//! Format: lays a script's queries out in river style and uppercases
//! reserved words. It works on the tokens and never parses: only
//! whitespace and the case of keywords change, and a check at the end
//! holds it to that.

#[cfg(test)]
mod tests {
    use super::super::is_keyword;
    use super::*;

    const DIALECTS: [Dialect; 3] = [Dialect::Postgres, Dialect::MySql, Dialect::Sqlite];

    /// `script` with the statements `selection` overlaps laid out (all of
    /// them for `None`), and where the laid out part is in the result. The
    /// layout itself, before the check that would hide a fault: the tokens
    /// are compared here.
    fn laid(
        dialect: Dialect,
        script: &str,
        selection: Option<Range<usize>>,
    ) -> (String, Range<usize>) {
        let old = tokenize(dialect, script);
        let Some((region, laid)) = laid_out(dialect, script, &old, selection) else {
            return (script.to_owned(), 0..0);
        };
        let result = [&script[..region.start], &laid, &script[region.end..]].concat();
        let new = tokenize(dialect, &result);
        assert!(same_tokens(script, &old, &result, &new), "{script:?}");
        assert!(sigils_hold(script, &old, &result, &new), "{script:?}");
        (result, region.start..region.start + laid.len())
    }

    /// Whether every word that touched a sigil or a number before it
    /// (`@from`, `:limit`, `1st`) still touches it. The tokenizer cuts
    /// these apart where a server reads one token, so `same_tokens` cannot
    /// tell when the layout parts them.
    fn sigils_hold(old: &str, old_tokens: &[Token], new: &str, new_tokens: &[Token]) -> bool {
        let visible = |tokens: &[Token]| {
            let tokens = tokens
                .iter()
                .filter(|token| token.kind != TokenKind::Whitespace);
            tokens.cloned().collect::<Vec<Token>>()
        };
        let (old_tokens, new_tokens) = (visible(old_tokens), visible(new_tokens));
        let touching =
            |tokens: &[Token], at: usize| tokens[at - 1].range.end == tokens[at].range.start;
        let word = |token: &Token| matches!(token.kind, TokenKind::Keyword | TokenKind::Identifier);
        (1..old_tokens.len()).all(|at| {
            let (before, after) = (&old_tokens[at - 1], &old_tokens[at]);
            let (first, second) = (&old[before.range.clone()], &old[after.range.clone()]);
            // `@from`, `:limit`, `$offset`, `1st`: the word is the second.
            let follows = word(after)
                && match before.kind {
                    TokenKind::Number => true,
                    TokenKind::Operator => matches!(first, "@" | ":"),
                    TokenKind::Punctuation => first.starts_with('$'),
                    _ => false,
                };
            // `limit$x`, `offset.1st`: the word is the first.
            let leads = word(before)
                && match after.kind {
                    TokenKind::Number => second.starts_with('.'),
                    TokenKind::Punctuation => second.starts_with('$'),
                    _ => false,
                };
            let bound = (follows || leads) && touching(&old_tokens, at);
            // The words' case is as it was, too.
            !bound
                || (touching(&new_tokens, at)
                    && first == &new[new_tokens[at - 1].range.clone()]
                    && second == &new[new_tokens[at].range.clone()])
        })
    }

    /// The whole of `script` formatted, or the script itself when Format
    /// has nothing to change. Checks what every result owes: the same
    /// tokens, and nothing left for a second pass.
    fn formatted(dialect: Dialect, script: &str) -> String {
        let (result, _) = laid(dialect, script, None);
        let checked = format(dialect, script, None, 0).map(|formatted| formatted.text);
        assert_eq!(checked.as_deref().unwrap_or(script), result, "{script:?}");
        // The second pass is the layout's too: `format` returns `None` as
        // well for a layout its check refused.
        assert_eq!(laid(dialect, &result, None).0, result, "{script:?}");
        result
    }

    fn pg(script: &str) -> String {
        formatted(Dialect::Postgres, script)
    }

    fn mysql(script: &str) -> String {
        formatted(Dialect::MySql, script)
    }

    fn sqlite(script: &str) -> String {
        formatted(Dialect::Sqlite, script)
    }

    /// `lines` joined by line feeds: a formatted script, a line a string.
    fn lines(lines: &[&str]) -> String {
        lines.join("\n")
    }

    #[test]
    fn the_first_word_of_each_head_ends_at_the_river() {
        let script = "select a.name as author, b.genre, count(*) as books from books b \
            left join authors a on a.id = b.author_id where b.published >= 2020 \
            and b.genre <> 'draft' group by a.name, b.genre order by b.genre, books desc limit 10;";
        assert_eq!(
            pg(script),
            lines(&[
                "SELECT a.name AS author,",
                "       b.genre,",
                "       count(*) AS books",
                "  FROM books b",
                "  LEFT JOIN authors a ON a.id = b.author_id",
                " WHERE b.published >= 2020",
                "   AND b.genre <> 'draft'",
                " GROUP BY a.name, b.genre",
                " ORDER BY b.genre, books DESC",
                " LIMIT 10;",
            ])
        );
        // Every other head; a word longer than the river starts at its base.
        let script = "select genre from books group by genre having count(*) > 1 \
            window w as (partition by genre) order by 1 limit 5 offset 2 \
            fetch first 3 rows only for update";
        assert_eq!(
            pg(script),
            lines(&[
                "SELECT genre",
                "  FROM books",
                " GROUP BY genre",
                "HAVING count(*) > 1",
                "WINDOW w AS (PARTITION BY genre)",
                " ORDER BY 1",
                " LIMIT 5",
                "OFFSET 2",
                " FETCH first 3 rows only",
                "   FOR UPDATE",
            ])
        );
    }

    #[test]
    fn a_run_of_join_leaders_is_one_head() {
        let script = "select * from a join b on a.id = b.id inner join c using (id) \
            left outer join d on true right join e on true full outer join f on true \
            cross join g natural left outer join h";
        assert_eq!(
            pg(script),
            lines(&[
                "SELECT *",
                "  FROM a",
                "  JOIN b ON a.id = b.id",
                " INNER JOIN c USING (id)",
                "  LEFT OUTER JOIN d ON TRUE",
                " RIGHT JOIN e ON TRUE",
                "  FULL OUTER JOIN f ON TRUE",
                " CROSS JOIN g",
                "NATURAL LEFT OUTER JOIN h",
            ])
        );
        // A leader with no JOIN after it is a word on its line.
        assert_eq!(
            pg("select left(title, 2), right(title, 1) from books"),
            lines(&[
                "SELECT LEFT(title, 2),",
                "       RIGHT(title, 1)",
                "  FROM books"
            ])
        );
        assert_eq!(
            mysql("select * from a straight_join b on a.id = b.id"),
            lines(&["SELECT *", "  FROM a", "STRAIGHT_JOIN b ON a.id = b.id"])
        );
    }

    #[test]
    fn and_and_or_start_lines_in_conditions_only() {
        let script = "select a and b, c or d from t join u on t.id = u.id and u.x or u.y \
            where p between 1 and 2 and q or r group by s having v and w";
        assert_eq!(
            pg(script),
            lines(&[
                "SELECT a AND b,",
                "       c OR d",
                "  FROM t",
                "  JOIN u ON t.id = u.id",
                "   AND u.x",
                "    OR u.y",
                " WHERE p BETWEEN 1 AND 2",
                "   AND q",
                "    OR r",
                " GROUP BY s",
                "HAVING v",
                "   AND w",
            ])
        );
        // Inside parentheses they stay on their line.
        assert_eq!(
            pg("select 1 where (a and b) or (c or d)"),
            lines(&["SELECT 1", " WHERE (a AND b)", "    OR (c OR d)"])
        );
        assert_eq!(
            pg("select 1 from t group by a having b or c"),
            lines(&[
                "SELECT 1",
                "  FROM t",
                " GROUP BY a",
                "HAVING b",
                "    OR c"
            ])
        );
    }

    #[test]
    fn only_the_select_and_with_lists_break() {
        assert_eq!(
            pg("select distinct a, f(b, c), d from t, u group by a, b order by a, b"),
            lines(&[
                "SELECT DISTINCT a,",
                "       f(b, c),",
                "       d",
                "  FROM t, u",
                " GROUP BY a, b",
                " ORDER BY a, b",
            ])
        );
        assert_eq!(pg("select * from t"), lines(&["SELECT *", "  FROM t"]));
        assert_eq!(
            pg(
                "with recursive a as (select 1), b(n) as materialized (select 2) select * from a, b"
            ),
            lines(&[
                "  WITH RECURSIVE a AS (SELECT 1),",
                "       b(n) AS materialized (SELECT 2)",
                "SELECT *",
                "  FROM a, b",
            ])
        );
    }

    #[test]
    fn a_query_in_parentheses_has_its_own_river() {
        assert_eq!(
            pg(
                "select title from books where author_id in (select id from authors \
                where country = 'PL') order by title;"
            ),
            lines(&[
                "SELECT title",
                "  FROM books",
                " WHERE author_id IN (SELECT id",
                "                       FROM authors",
                "                      WHERE country = 'PL')",
                " ORDER BY title;",
            ])
        );
        assert_eq!(
            pg(
                "with recent as (select id, title from books where published >= 2020) \
                select title from recent;"
            ),
            lines(&[
                "  WITH recent AS (SELECT id,",
                "                         title",
                "                    FROM books",
                "                   WHERE published >= 2020)",
                "SELECT title",
                "  FROM recent;",
            ])
        );
        // Two deep, in FROM and in a condition.
        assert_eq!(
            pg("select * from (select * from (select 1) x) y where exists \
                (select 1 from books where id in (select book_id from loans))"),
            lines(&[
                "SELECT *",
                "  FROM (SELECT *",
                "          FROM (SELECT 1) x) y",
                " WHERE EXISTS (SELECT 1",
                "                 FROM books",
                "                WHERE id IN (SELECT book_id",
                "                               FROM loans))",
            ])
        );
        // A WITH inside parentheses starts right after them.
        assert_eq!(
            pg("select * from (with c as (select 2) select * from c) d"),
            lines(&[
                "SELECT *",
                "  FROM (WITH c AS (SELECT 2)",
                "        SELECT *",
                "          FROM c) d",
            ])
        );
        // Columns are characters, not bytes.
        assert_eq!(
            pg("select żółw from t where 'é' in (select 1 from u)"),
            lines(&[
                "SELECT żółw",
                "  FROM t",
                " WHERE 'é' IN (SELECT 1",
                "                 FROM u)",
            ])
        );
    }

    #[test]
    fn nesting_without_end_stops_being_laid_out_as_blocks() {
        // Deeper than any stack could follow: the layout must not recurse
        // for each.
        let deep = 100_000;
        let script = "(select 1 from ".repeat(deep) + "t" + &")".repeat(deep);
        let formatted = format(Dialect::Postgres, &script, None, 0).unwrap();
        // Each block puts its FROM on a line; past the depth it is all on
        // the last one.
        assert_eq!(formatted.text.lines().count(), MAX_DEPTH + 1);
        let last = "(SELECT 1 FROM ".repeat(deep - MAX_DEPTH) + "t" + &")".repeat(deep);
        assert!(formatted.text.ends_with(&last));
        // Nor is a long run of join leaders read to its end for each word.
        let script = "select ".to_owned() + &"left ".repeat(deep) + "join t";
        assert!(format(Dialect::Postgres, &script, None, 0).is_some());
    }

    #[test]
    fn set_operations_are_heads() {
        assert_eq!(
            pg("select 1 union select 2 union all select 3 except select 4 intersect select 5"),
            lines(&[
                "SELECT 1",
                " UNION",
                "SELECT 2",
                " UNION ALL",
                "SELECT 3",
                "EXCEPT",
                "SELECT 4",
                "INTERSECT",
                "SELECT 5",
            ])
        );
        assert_eq!(
            pg("(select 1) union (select 2)"),
            lines(&["(SELECT 1)", " UNION (SELECT 2)"])
        );
        assert_eq!(
            pg("select 1 union distinct select 2"),
            lines(&["SELECT 1", " UNION DISTINCT", "SELECT 2"])
        );
    }

    #[test]
    fn a_case_puts_its_parts_on_lines() {
        assert_eq!(
            pg(
                "select title, case when stock = 0 then 'out' when stock < 5 and stock > 0 \
                then 'low' else 'ok' end as availability from books;"
            ),
            lines(&[
                "SELECT title,",
                "       CASE",
                "           WHEN stock = 0 THEN 'out'",
                "           WHEN stock < 5 AND stock > 0 THEN 'low'",
                "           ELSE 'ok'",
                "       END AS availability",
                "  FROM books;",
            ])
        );
        // An operand stays on the CASE's line; a CASE in a CASE has its
        // own column.
        assert_eq!(
            pg("select case genre when 'a' then case when x then 1 else 2 end else 3 end from t"),
            lines(&[
                "SELECT CASE genre",
                "           WHEN 'a' THEN CASE",
                "                             WHEN x THEN 1",
                "                             ELSE 2",
                "                         END",
                "           ELSE 3",
                "       END",
                "  FROM t",
            ])
        );
        // An END closes only a CASE of its own block: the table `end` in
        // the block inside is a plain word, in every dialect.
        for dialect in DIALECTS {
            assert_eq!(
                formatted(
                    dialect,
                    "select case when x in (select id from end) then 1 end from t"
                ),
                lines(&[
                    "SELECT CASE",
                    "           WHEN x IN (SELECT id",
                    "                        FROM end) THEN 1",
                    "       END",
                    "  FROM t",
                ]),
                "{dialect:?}"
            );
        }
    }

    #[test]
    fn heads_inside_parentheses_stay_on_their_line() {
        assert_eq!(
            pg(
                "select extract(year from added), count(*) over (partition by genre \
                order by title), id in (1, 2), trim(both from title) from books"
            ),
            lines(&[
                "SELECT extract(year FROM added),",
                "       count(*) over (PARTITION BY genre ORDER BY title),",
                "       id IN (1, 2),",
                "       trim(both FROM title)",
                "  FROM books",
            ])
        );
    }

    #[test]
    fn a_word_beside_a_dot_and_three_more_are_not_heads() {
        // A qualified name, whatever it spells, in every dialect.
        for dialect in DIALECTS {
            assert_eq!(
                formatted(
                    dialect,
                    "select r.from, order.id from shop.order where r.select = 1"
                ),
                lines(&[
                    "SELECT r.from,",
                    "       order.id",
                    "  FROM shop.order",
                    " WHERE r.select = 1",
                ]),
                "{dialect:?}"
            );
        }
        assert_eq!(
            pg("select 1 where a is distinct from b and c is not distinct from d"),
            lines(&[
                "SELECT 1",
                " WHERE a IS DISTINCT FROM b",
                "   AND c IS NOT DISTINCT FROM d",
            ])
        );
        assert_eq!(
            pg("select * from unnest(x) with ordinality as t(a, n)"),
            lines(&["SELECT *", "  FROM unnest(x) WITH ordinality AS t(a, n)"])
        );
        assert_eq!(
            mysql("select distinct straight_join a from t"),
            lines(&["SELECT DISTINCT STRAIGHT_JOIN a", "  FROM t"])
        );
    }

    #[test]
    fn a_word_that_touches_a_sigil_or_a_number_is_a_name() {
        // The tokenizer cuts `@from` into `@` and `from`; a server reads
        // one token. Whatever it spells, the word stays where it is and
        // as it is: a MySQL user variable,
        assert_eq!(
            mysql("select * from t where d between @from and @to and x = @limit"),
            lines(&[
                "SELECT *",
                "  FROM t",
                " WHERE d BETWEEN @from AND @to",
                "   AND x = @limit",
            ])
        );
        assert_eq!(
            mysql("select case when d < @end then 1 end, @@global.select"),
            lines(&[
                "SELECT CASE",
                "           WHEN d < @end THEN 1",
                "       END,",
                "       @@global.select",
            ])
        );
        assert_eq!(mysql("set @from = 1"), "SET @from = 1");
        // a parameter,
        assert_eq!(
            sqlite("select :limit, @from, $offset from t where a = ?1"),
            lines(&[
                "SELECT :limit,",
                "       @from,",
                "       $offset",
                "  FROM t",
                " WHERE a = ?1",
            ])
        );
        // and a name that starts with digits.
        assert_eq!(
            mysql("select 1from, 2 from t"),
            lines(&["SELECT 1from,", "       2", "  FROM t"])
        );
        // A word may lead such a name as well: SQLite's `$` inside one,
        // and a qualified name whose second part starts with a digit,
        // which the tokenizer reads as a number with its `.`.
        assert_eq!(
            sqlite("select a from t where limit$x > 0"),
            lines(&["SELECT a", "  FROM t", " WHERE limit$x > 0"])
        );
        assert_eq!(
            mysql("select offset.1st, order.2nd from t"),
            lines(&["SELECT offset.1st,", "       order.2nd", "  FROM t"])
        );
        // Apart from the sigil it is the word it spells.
        assert_eq!(pg("select 1 from t"), lines(&["SELECT 1", "  FROM t"]));
        assert_eq!(
            pg("select a::interval from t"),
            lines(&["SELECT a::interval", "  FROM t"])
        );
    }

    #[test]
    fn spacing_follows_what_was_typed_where_no_rule_sets_it() {
        // Tokens that touched still touch; a space that was typed stays
        // one space; commas and parentheses have their own rule.
        assert_eq!(
            pg("select  count( * ) ,f (x),a>=1 , - -1, 1 / *, price::text, a ->> 'b'  from  t"),
            lines(&[
                "SELECT count(*),",
                "       f (x),",
                "       a>=1,",
                "       - -1,",
                "       1 / *,",
                "       price::text,",
                "       a ->> 'b'",
                "  FROM t",
            ])
        );
        // A head is followed by one space, typed or not.
        assert_eq!(
            pg("select(1)from(select 2)t group by(a)"),
            lines(&["SELECT (1)", "  FROM (SELECT 2)t", " GROUP BY (a)"])
        );
        // PostgreSQL joins two strings only across a line break: what
        // stands between two strings is copied.
        assert_eq!(
            pg("select 'a'\r\n   'b', 'c'  'd' from t"),
            lines(&["SELECT 'a'\r", "   'b',", "       'c'  'd'", "  FROM t"])
        );
    }

    #[test]
    fn comments_keep_their_lines() {
        let script = lines(&[
            "-- top",
            "select a, -- one",
            "  -- own",
            "  b /* in */ , c",
            "  from t -- tail",
            "  where x in (1 -- last",
            "  ) and y = (select 1 -- q",
            "  )",
            "  /* block",
            "     two */",
            " order by 1 -- end",
            ";",
            "-- after",
        ]);
        assert_eq!(
            pg(&script),
            lines(&[
                "-- top",
                "SELECT a, -- one",
                "       -- own",
                "       b /* in */,",
                "       c",
                "  FROM t -- tail",
                " WHERE x IN (1 -- last",
                "            )",
                "   AND y = (SELECT 1 -- q",
                "           )",
                "       /* block",
                "     two */",
                " ORDER BY 1 -- end",
                ";",
                "-- after",
            ])
        );
        // What follows a comment that ends its line starts at the content
        // column, unless it is a head.
        assert_eq!(
            pg("select a -- one\n+ b from t"),
            lines(&["SELECT a -- one", "       + b", "  FROM t"])
        );
        // A comment before a statement on its line stays before it.
        assert_eq!(pg("/* a */   select 1"), "/* a */ SELECT 1");
        // Touching the code before it, it still touches: on MySQL a space
        // before `--` can turn two minus signs into a comment.
        assert_eq!(pg("select 1--x"), "SELECT 1--x");
        assert_eq!(mysql("select 1---- x"), "SELECT 1---- x");
        // Nor does a line start right after them there: a head that
        // touches them stays on their line.
        assert_eq!(
            mysql("select 1--from t where a--and b"),
            lines(&["SELECT 1--FROM t", " WHERE a--AND b"])
        );
        assert_eq!(
            pg("select 1 - -from t"),
            lines(&["SELECT 1 - -", "  FROM t"])
        );
    }

    #[test]
    fn statements_are_a_blank_line_apart() {
        assert_eq!(
            pg("select 1; select 2;\n\n\n\nselect 3"),
            lines(&["SELECT 1;", "", "SELECT 2;", "", "SELECT 3"])
        );
        // Around a comment between statements the blank lines stay as
        // typed, one at most; a comment after a statement stays on its
        // line.
        assert_eq!(
            pg(
                "select 1; -- a\nselect 2;\n-- b\nselect 3;\n\n\n\n-- c\n\n\nselect 4; /* d */ select 5"
            ),
            lines(&[
                "SELECT 1; -- a",
                "SELECT 2;",
                "-- b",
                "SELECT 3;",
                "",
                "-- c",
                "",
                "SELECT 4; /* d */",
                "SELECT 5",
            ])
        );
        // Whitespace around the script stays; the first line's
        // indentation is Format's.
        assert_eq!(pg("\n\n   select 1 ;  \n"), "\n\nSELECT 1;  \n");
        assert_eq!(pg("select 1;;"), "SELECT 1;;");
    }

    #[test]
    fn a_statement_that_is_no_query_keeps_its_lines() {
        let script = lines(&[
            "create table t (  ",
            "  a int,\t",
            "",
            "  b text default 'x'",
            ");",
            "explain select * from t;",
            "insert into t",
            "   values (1)   \r",
            "  returning id;",
            "show tables; values (1), (2)",
        ]);
        assert_eq!(
            pg(&script),
            lines(&[
                "CREATE TABLE t (",
                "  a int,",
                "",
                "  b text DEFAULT 'x'",
                ");",
                "",
                "EXPLAIN SELECT * FROM t;",
                "",
                "INSERT INTO t",
                "   VALUES (1)",
                "  RETURNING id;",
                "",
                "SHOW tables;",
                "",
                "VALUES (1), (2)",
            ])
        );
    }

    #[test]
    fn each_dialect_keeps_its_own_tokens() {
        assert_eq!(
            pg(
                "select $$ select\n  from $$, $tag$ a ; b $tag$, E'x'\n'y', a[1], $1 from t; select 2"
            ),
            lines(&[
                "SELECT $$ select",
                "  from $$,",
                "       $tag$ a ; b $tag$,",
                "       E'x'",
                "'y',",
                "       a[1],",
                "       $1",
                "  FROM t;",
                "",
                "SELECT 2",
            ])
        );
        // A backslash escapes the quote in an E string.
        assert_eq!(
            pg("select E'\\'' as q, 'x' from t where a = E'it\\'s' and b"),
            lines(&[
                "SELECT E'\\'' AS q,",
                "       'x'",
                "  FROM t",
                " WHERE a = E'it\\'s'",
                "   AND b",
            ])
        );
        // MySQL: `#` comments, `--` only before a blank, the `\r` a line
        // comment keeps, and an executable comment, which is code: a
        // statement that starts with one is no query.
        assert_eq!(
            mysql(
                "select `from`, \"str\" from t # c\nwhere a = 1 -- x\r\nand b = 1--1 /*! straight_join */;\r\n/*! select */ select 1\n from t"
            ),
            lines(&[
                "SELECT `from`,",
                "       \"str\"",
                "  FROM t # c",
                " WHERE a = 1 -- x\r",
                "   AND b = 1--1 /*! straight_join */;",
                "",
                "/*! select */ SELECT 1",
                " FROM t",
            ])
        );
        assert_eq!(
            sqlite("select [from], glob('a', b) from t; pragma table_info(books)"),
            lines(&[
                "SELECT [from],",
                "       GLOB('a', b)",
                "  FROM t;",
                "",
                "PRAGMA table_info(books)",
            ])
        );
    }

    #[test]
    fn reserved_words_are_uppercased_and_names_are_not() {
        // Every listed word, where it is a keyword.
        for dialect in DIALECTS {
            for word in UPPERCASED {
                let lower = word.to_ascii_lowercase();
                let script = std::format!("show {lower}");
                let expected = if is_keyword(dialect, word) {
                    std::format!("SHOW {word}")
                } else {
                    std::format!("SHOW {lower}")
                };
                assert_eq!(formatted(dialect, &script), expected, "{dialect:?}");
            }
        }
        // The keywords that can be names keep their case.
        let names =
            "any begin cast commit current end filter first no only over rollback rows view";
        for dialect in DIALECTS {
            assert_eq!(
                formatted(dialect, &std::format!("show {names}")),
                std::format!("SHOW {names}"),
                "{dialect:?}"
            );
        }
        // Read as structure they are uppercased, but not on MySQL, where
        // such a word may be an alias and an alias's case can matter.
        let script = "with recursive a as (select 1) select * from a full join b on true \
            window w as () limit 1 offset 2";
        for dialect in [Dialect::Postgres, Dialect::Sqlite] {
            assert_eq!(
                formatted(dialect, script),
                lines(&[
                    "  WITH RECURSIVE a AS (SELECT 1)",
                    "SELECT *",
                    "  FROM a",
                    "  FULL JOIN b ON TRUE",
                    "WINDOW w AS ()",
                    " LIMIT 1",
                    "OFFSET 2",
                ])
            );
        }
        assert_eq!(
            mysql(script),
            lines(&[
                "  WITH recursive a AS (SELECT 1)",
                "SELECT *",
                "  FROM a",
                "  full JOIN b ON TRUE",
                "window w AS ()",
                " LIMIT 1",
                "offset 2",
            ])
        );
        assert_eq!(
            pg("select 1 except select 2 intersect select 3"),
            lines(&["SELECT 1", "EXCEPT", "SELECT 2", "INTERSECT", "SELECT 3"])
        );
        assert_eq!(
            mysql("select 1 except select 2"),
            lines(&["SELECT 1", "except", "SELECT 2"])
        );
        // The END of a CASE is uppercased in every dialect.
        assert_eq!(
            mysql("select case when a then 1 end"),
            lines(&["SELECT CASE", "           WHEN a THEN 1", "       END"])
        );
        // A quoted name and a string that spell a keyword are untouched.
        assert_eq!(
            pg("select \"from\", 'select' from t"),
            lines(&["SELECT \"from\",", "       'select'", "  FROM t"])
        );
    }

    #[test]
    fn the_lists_hold_only_keywords_the_highlighter_knows() {
        for word in UPPERCASED
            .iter()
            .chain(STRUCTURE)
            .chain(HEADS)
            .chain(JOIN_LEADERS)
        {
            assert!(
                DIALECTS.iter().any(|dialect| is_keyword(*dialect, word)),
                "{word}"
            );
        }
        // A word is uppercased by the list or as structure, not both.
        assert!(STRUCTURE.iter().all(|word| !UPPERCASED.contains(word)));
    }

    #[test]
    fn awkward_scripts_keep_their_tokens() {
        // `formatted` checks the tokens and the second pass.
        let scripts = [
            "",
            " ",
            ";",
            ";;",
            "-- only a comment",
            "/* open",
            "select 'unterminated",
            "select \"unterminated",
            "select 1 -- no newline",
            "select ) from ( t",
            "select ((select 1)), (((2)))",
            "select case when a then case when b then 1",
            "select a end end from t",
            "select 1;\r\n\r\n-- the count\r\nselect count(*)\r\n  from books\r\n;\nselect 2",
            "select a - -1, b - - 2, c / * d, e /* x */ * f, 1--1, 1 -- 1\n, 2",
            "select 'a' 'b'\n'c', $$x$$ $$y$$, E'a'\n'b'",
            "ż ó\r\n\tł;;",
            "SELECT 1; SELECT 'a;b'; SELECT $$c;d$$; SELECT `e;f`",
            "select * from t where a = 1 and b between 2 and 3 and c or d",
            "with a as (select 1), b as (select 2) (select * from a) union (select * from b)",
            "select /*! straight_join */ 1 # x\nfrom t -- y\r\nwhere a",
            "select 1--from t where a--and b--or c--,d from--(select 1)--union select--case--when 1",
            "insert into t (a, b) select 1, 2 from u on conflict do nothing returning *",
        ];
        for dialect in DIALECTS {
            for script in scripts {
                formatted(dialect, script);
            }
        }
    }

    #[test]
    fn scripts_built_at_random_keep_their_tokens() {
        // What a script is made of, the awkward pieces included. A fixed
        // seed: the same scripts on every run.
        let words = "select from where and or between group order by join left outer on with \
            as case when then else end union distinct limit offset in exists insert values \
            straight_join natural full cross inner having window fetch for intersect except \
            all recursive is not over a b.c t x. .y 's' \"q\" `q` $$d$$ E'e' $1 1 2.5 \
            @from :limit $offset @end 1from limit$x offset.1st @ : $ .5 \
            ( ) [ ] , ; . * - / = : < > ! | # @ /*c*/ /*!e*/";
        let awkward = [
            "--x\n",
            "-- y\r\n",
            "# z\n",
            "/* m\n n */",
            "'a'\n'b'",
            " ",
            "\n",
            "\r\n",
            "\n\n\n",
            "\t",
        ];
        let pieces: Vec<&str> = words.split_whitespace().chain(awkward).collect();
        let mut seed = 0x9e37_79b9_7f4a_7c15_u64;
        let mut next = |below: usize| {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            (seed % below as u64) as usize
        };
        for _ in 0..2_000 {
            let mut script = String::new();
            for _ in 0..next(40) {
                script.push_str(pieces[next(pieces.len())]);
                // Mostly apart, sometimes touching.
                if next(4) > 0 {
                    script.push(' ');
                }
            }
            // The pieces are ASCII, so every offset is a character's.
            let (from, to) = (next(script.len() + 1), next(script.len() + 1));
            let selection = from.min(to)..from.max(to);
            for dialect in DIALECTS {
                formatted(dialect, &script);
                // A selection's statements: the same tokens, and laid out
                // again they stay as they are.
                let (result, part) = laid(dialect, &script, Some(selection.clone()));
                if !part.is_empty() {
                    assert_eq!(laid(dialect, &result, Some(part)).0, result, "{script:?}");
                }
            }
        }
    }

    #[test]
    fn the_check_tells_a_changed_token() {
        let same = |old: &str, new: &str| {
            let dialect = Dialect::Postgres;
            same_tokens(old, &tokenize(dialect, old), new, &tokenize(dialect, new))
        };
        assert!(same("select  a\nfrom t", "SELECT a FROM t"));
        // Two operators fused into a comment; a name's case; a missing
        // token; a string's contents.
        assert!(!same("select - -1", "select --1"));
        assert!(!same("select Books", "select books"));
        assert!(!same("select 1, 2", "select 1 2"));
        assert!(!same("select 'a  b'", "select 'a b'"));
    }

    #[test]
    fn nothing_to_change_is_none() {
        for script in [
            "",
            "  \n",
            "-- a note\n",
            "SELECT 1;",
            "SELECT a,\n       b\n  FROM t;\n\nSELECT 2;\n",
        ] {
            assert!(
                format(Dialect::Postgres, script, None, 0).is_none(),
                "{script:?}"
            );
        }
    }

    #[test]
    fn a_selection_formats_the_statements_it_overlaps() {
        let script = "select 1 ;\nselect a,b from t ;\n  select 3 ;";
        let second = script.find("a,b").unwrap();
        let text = |selection: Range<usize>| {
            format(Dialect::Postgres, script, Some(selection), 0).map(|formatted| formatted.text)
        };
        // One statement of three: the others are as typed.
        assert_eq!(
            text(second..second + 1).as_deref(),
            Some("select 1 ;\nSELECT a,\n       b\n  FROM t;\n  select 3 ;")
        );
        // Across two statements, from anywhere in the first to anywhere
        // in the second.
        assert_eq!(
            text(second..script.len() - 3).as_deref(),
            Some("select 1 ;\nSELECT a,\n       b\n  FROM t;\n\nSELECT 3;")
        );
        // In the whitespace between statements: nothing.
        let between = script.find(";\n").unwrap() + 1;
        assert_eq!(text(between..between + 1), None);
        // An empty selection is no selection.
        assert_eq!(
            text(second..second).as_deref(),
            Some("SELECT 1;\n\nSELECT a,\n       b\n  FROM t;\n\nSELECT 3;")
        );
        // A statement that does not start its line is laid out from
        // column 0 and its first line stays where it stood.
        let script = "select 1; select a,b";
        let formatted = format(Dialect::Postgres, script, Some(12..13), 0).unwrap();
        assert_eq!(formatted.text, "select 1; SELECT a,\n       b");
        // It goes on from where it stands: a head the river would pad is
        // not padded there, so a second press finds nothing to change.
        let script = "select 1; with a as (select 2) select * from a";
        let selection = script.find("with").unwrap()..script.len();
        let formatted = format(Dialect::Postgres, script, Some(selection.clone()), 0).unwrap();
        assert_eq!(
            formatted.text,
            "select 1; WITH a AS (SELECT 2)\nSELECT *\n  FROM a"
        );
        let selection = selection.start..formatted.text.len();
        assert!(format(Dialect::Postgres, &formatted.text, Some(selection), 0).is_none());
    }

    #[test]
    fn the_cursor_keeps_its_place_among_the_tokens() {
        let script = "  select a,b\n\n from t ; select 2";
        let moved = |cursor: usize| {
            let formatted = format(Dialect::Postgres, script, None, cursor).unwrap();
            assert_eq!(formatted.text, "SELECT a,\n       b\n  FROM t;\n\nSELECT 2");
            formatted.cursor
        };
        // Before the script's first token, in what was its indentation.
        assert_eq!(moved(0), 0);
        assert_eq!(moved(1), 0);
        // In a word, and at a word's end.
        assert_eq!(moved(4), 2);
        assert_eq!(moved(8), 6);
        // Right after a token it stays after it; from whitespace it goes
        // before the next token.
        assert_eq!(moved(11), 9);
        assert_eq!(moved(12), 18);
        assert_eq!(moved(13), 21);
        assert_eq!(moved(15), 21);
        // At the script's end.
        assert_eq!(moved(script.len()), 38);
        // Outside a selection's statements it moves with the text.
        let script = "select 1 ;\nselect a,b ;\n-- end";
        let formatted = format(Dialect::Postgres, script, Some(12..13), script.len()).unwrap();
        assert_eq!(formatted.text, "select 1 ;\nSELECT a,\n       b;\n-- end");
        assert_eq!(formatted.cursor, formatted.text.len());
        let formatted = format(Dialect::Postgres, script, Some(12..13), 3).unwrap();
        assert_eq!(formatted.cursor, 3);
    }
}
```

The `formatted` helper is what makes these tests worth having: it runs the layout without the safety check, asserts the tokens are the same, and asserts a second pass changes nothing. Every expectation goes through it.

- [ ] **Step 3: Run the tests to see them fail**

Run: `~/.cargo/bin/cargo test --locked -p tabletist-db --lib sql::format`
Expected: does not compile. The compiler cannot find `format`, `laid_out`, `same_tokens`, `tokenize` or the lists (`UPPERCASED`, `STRUCTURE`, `HEADS`, `JOIN_LEADERS`).

- [ ] **Step 4: Write the formatter**

Replace the doc comment at the top of `crates/tabletist-db/src/sql/format.rs` with the whole implementation (the tests module stays below it):

```rust
//! Format: lays a script's queries out in river style and uppercases
//! reserved words. It works on the tokens and never parses: only
//! whitespace and the case of keywords change, and a check at the end
//! holds it to that.

use std::borrow::Cow;
use std::ops::Range;

use super::{Token, TokenKind, statements_from, tokenize};
use crate::Dialect;

/// A script after Format. It has no `Debug`: what a user types into the
/// editor is never logged.
#[derive(Clone, PartialEq, Eq)]
pub struct Formatted {
    pub text: String,
    /// Byte offset of the cursor in `text`.
    pub cursor: usize,
}

/// The keywords Format uppercases: the ones that cannot be an unquoted
/// table name or alias where case would matter. Every one the MySQL
/// dialect reads as a keyword is reserved in MySQL 5.7, MySQL 8 and
/// MariaDB (the MySQL tests ask the server they run against); the words
/// only PostgreSQL or SQLite read as keywords are here because those
/// compare unquoted names without case.
pub const UPPERCASED: &[&str] = &[
    "ALL",
    "ALTER",
    "AND",
    "AS",
    "ASC",
    "BETWEEN",
    "BY",
    "CASE",
    "CREATE",
    "CROSS",
    "DEFAULT",
    "DELETE",
    "DESC",
    "DESCRIBE",
    "DISTINCT",
    "DROP",
    "ELSE",
    "EXISTS",
    "EXPLAIN",
    "FALSE",
    "FETCH",
    "FOR",
    "FROM",
    "GLOB",
    "GROUP",
    "HAVING",
    "ILIKE",
    "IN",
    "INNER",
    "INSERT",
    "INTERVAL",
    "INTO",
    "IS",
    "JOIN",
    "LATERAL",
    "LEFT",
    "LIKE",
    "LIMIT",
    "NATURAL",
    "NOT",
    "NULL",
    "ON",
    "OR",
    "ORDER",
    "OUTER",
    "PARTITION",
    "PRAGMA",
    "REGEXP",
    "RETURNING",
    "RIGHT",
    "SELECT",
    "SET",
    "SHOW",
    "SIMILAR",
    "STRAIGHT_JOIN",
    "TABLE",
    "THEN",
    "TO",
    "TRUE",
    "UNION",
    "UPDATE",
    "USING",
    "VALUES",
    "WHEN",
    "WHERE",
    "WITH",
];

/// Keywords that can be names, uppercased where the layout read them as
/// structure, on the dialects that compare unquoted names without case.
const STRUCTURE: &[&str] = &[
    "EXCEPT",
    "FULL",
    "INTERSECT",
    "OFFSET",
    "RECURSIVE",
    "WINDOW",
];

/// The one-word heads that are a head wherever they stand at the top level
/// of a block.
const HEADS: &[&str] = &[
    "EXCEPT",
    "FETCH",
    "FOR",
    "INTERSECT",
    "LIMIT",
    "OFFSET",
    "UNION",
    "WINDOW",
];

/// The words that lead a join.
const JOIN_LEADERS: &[&str] = &[
    "CROSS", "FULL", "INNER", "LEFT", "NATURAL", "OUTER", "RIGHT",
];

/// The width of the river: `SELECT`.
const RIVER: usize = 6;

/// How far a `WHEN`, an `ELSE` and what follows them stand in from their
/// `CASE`.
const CASE_INDENT: usize = 4;

/// How deep queries in parentheses are laid out as blocks. The layout
/// recurses once for each, so a script nested without end (a generated
/// one, a hostile paste) must not be followed all the way down: past
/// this depth a parenthesis stays on its line, whatever it holds.
const MAX_DEPTH: usize = 64;

/// `script` with the statements `selection` overlaps formatted, or all of
/// them when it is `None` or empty, and where `cursor` went. All three are
/// byte offsets. `None` when there is nothing to change.
pub fn format(
    dialect: Dialect,
    script: &str,
    selection: Option<Range<usize>>,
    cursor: usize,
) -> Option<Formatted> {
    let tokens = tokenize(dialect, script);
    let (region, laid) = laid_out(dialect, script, &tokens, selection)?;
    let text = [&script[..region.start], &laid, &script[region.end..]].concat();
    if text == script {
        return None;
    }
    let after = tokenize(dialect, &text);
    if !same_tokens(script, &tokens, &text, &after) {
        log::warn!("Format would have changed a token: the text is left as typed");
        return None;
    }
    let cursor = if cursor <= region.start {
        cursor
    } else if cursor >= region.end {
        region.start + laid.len() + (cursor.min(script.len()) - region.end)
    } else {
        moved(&tokens, &after, cursor).unwrap_or(text.len())
    };
    Some(Formatted { text, cursor })
}

/// The bytes Format lays out and what it makes of them, unchecked.
fn laid_out(
    dialect: Dialect,
    script: &str,
    tokens: &[Token],
    selection: Option<Range<usize>>,
) -> Option<(Range<usize>, String)> {
    let region = region(script, tokens, selection)?;
    // The indentation of its first line is Format's to set.
    let region = line_start(script, region.start)..region.end;
    // A statement that does not start its line goes on from where it
    // stands: no line is started for it, and nothing is put before it.
    let line = script[..region.start].rfind('\n').map_or(0, |end| end + 1);
    let mut layout = Layout {
        dialect,
        items: items(script, tokens, &region),
        at: 0,
        writer: Writer {
            out: String::new(),
            column: script[line..region.start].chars().count(),
        },
        ended: false,
    };
    layout.run();
    Some((region, layout.writer.out))
}

/// The bytes Format lays out: from the first statement `selection`
/// overlaps to the last, or from the script's first token to its last.
fn region(script: &str, tokens: &[Token], selection: Option<Range<usize>>) -> Option<Range<usize>> {
    match selection.filter(|selection| !selection.is_empty()) {
        None => {
            let mut visible = tokens
                .iter()
                .filter(|token| token.kind != TokenKind::Whitespace);
            let first = visible.next()?;
            let last = visible.next_back().unwrap_or(first);
            Some(first.range.start..last.range.end)
        }
        Some(selection) => {
            let statements = statements_from(script, tokens);
            let mut touched = statements.iter().filter(|statement| {
                statement.range.start < selection.end && selection.start < statement.end
            });
            let first = touched.next()?;
            let last = touched.next_back().unwrap_or(first);
            Some(first.range.start..last.end)
        }
    }
}

/// The start of the line `at` is on, when only blanks stand between; else
/// `at`.
fn line_start(script: &str, at: usize) -> usize {
    let line = script[..at].rfind('\n').map_or(0, |end| end + 1);
    let blank = script[line..at]
        .bytes()
        .all(|byte| byte == b' ' || byte == b'\t');
    if blank { line } else { at }
}

/// A token that is not whitespace, and the whitespace before it.
#[derive(Clone, Copy)]
struct Item<'a> {
    kind: TokenKind,
    text: &'a str,
    /// Empty when the token touches the one before it, and for the first.
    space: &'a str,
}

impl Item<'_> {
    fn is(&self, punctuation: &str) -> bool {
        self.kind == TokenKind::Punctuation && self.text == punctuation
    }

    /// `--` and MySQL's `#`: a comment that ends its line.
    fn ends_line(&self) -> bool {
        self.kind == TokenKind::Comment && !self.text.starts_with("/*")
    }

    /// `@`, `:`, a `$` and a number: what a server reads as one token
    /// with a word that touches it (a user variable, a parameter, MySQL's
    /// names that start with digits), though the tokenizer cuts the two
    /// apart.
    fn binds(&self) -> bool {
        match self.kind {
            TokenKind::Number => true,
            TokenKind::Operator => matches!(self.text, "@" | ":"),
            TokenKind::Punctuation => self.text.starts_with('$'),
            _ => false,
        }
    }

    /// A `$`, and a number that starts with its `.`: what a server reads
    /// as more of the word that touches it from the left (SQLite's
    /// `limit$x`, the qualified name `offset.1st`).
    fn extends(&self) -> bool {
        match self.kind {
            TokenKind::Number => self.text.starts_with('.'),
            TokenKind::Punctuation => self.text.starts_with('$'),
            _ => false,
        }
    }
}

fn items<'a>(script: &'a str, tokens: &[Token], region: &Range<usize>) -> Vec<Item<'a>> {
    let mut items = Vec::new();
    let mut space = "";
    let inside = tokens
        .iter()
        .filter(|token| region.start <= token.range.start && token.range.end <= region.end);
    for token in inside {
        let text = &script[token.range.clone()];
        if token.kind == TokenKind::Whitespace {
            // What stands before the first item is not between two.
            if !items.is_empty() {
                space = text;
            }
        } else {
            items.push(Item {
                kind: token.kind,
                text,
                space,
            });
            space = "";
        }
    }
    items
}

fn newlines(space: &str) -> usize {
    space.bytes().filter(|byte| *byte == b'\n').count()
}

/// The whitespace of a statement that keeps its lines: its line breaks and
/// the indentation after the last, without what stood before a line's end.
fn kept(space: &str) -> Cow<'_, str> {
    match space.rfind('\n') {
        None => Cow::Borrowed(space),
        Some(last) => Cow::Owned("\n".repeat(newlines(space)) + &space[last + 1..]),
    }
}

/// The text laid out so far, and the column the next character lands on.
struct Writer {
    out: String,
    /// In characters, not bytes.
    column: usize,
}

impl Writer {
    fn push(&mut self, text: &str) {
        match text.rfind('\n') {
            Some(last) => self.column = text[last + 1..].chars().count(),
            None => self.column += text.chars().count(),
        }
        self.out.push_str(text);
    }

    /// Starts a line at `column`. A line with nothing on it yet is used.
    fn line(&mut self, column: usize) {
        if self.column > 0 {
            self.out.push('\n');
        }
        self.out.extend(std::iter::repeat_n(' ', column));
        self.column = column;
    }
}

/// What stands before the next statement or comment of a script.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Before {
    Nothing,
    Statement,
    /// A comment, and whether it ends its line.
    Comment(bool),
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Clause {
    Other,
    Select,
    With,
    Join,
}

/// What a token of a block is inside of, apart from the block.
#[derive(Clone, Copy)]
enum Frame {
    /// A parenthesis that opens no block, and the column of its `(`.
    Paren(usize),
    /// A `CASE`, and the column it starts at.
    Case(usize),
}

/// Where a token goes.
#[derive(Clone, Copy)]
enum Place {
    /// After the token before it, on its line.
    Inline,
    /// At the start of a line, at this column.
    Line(usize),
}

/// A query being laid out: a statement's, or one in parentheses.
struct Block {
    /// The column its river starts at.
    base: usize,
    /// How many blocks it is inside of. A statement's own block is
    /// inside none, and no `)` ends it.
    depth: usize,
    clause: Clause,
    /// `AND` and `OR` start lines: in a `WHERE`, a `HAVING`, a join's `ON`.
    conditions: bool,
    /// A `BETWEEN` waits for its `AND`.
    between: bool,
    frames: Vec<Frame>,
    /// No token of it is written yet.
    first: bool,
    /// A list's comma was written: its next item starts a line.
    broken: bool,
    /// A head was written: one space follows it.
    headed: bool,
}

impl Block {
    fn new(base: usize, depth: usize) -> Self {
        Self {
            base,
            depth,
            clause: Clause::Other,
            conditions: false,
            between: false,
            frames: Vec::new(),
            first: true,
            broken: false,
            headed: false,
        }
    }

    /// Where what follows a head's line goes: list items, comments.
    fn content(&self) -> usize {
        self.base + RIVER + 1
    }
}

struct Layout<'a> {
    dialect: Dialect,
    items: Vec<Item<'a>>,
    /// The item to write next.
    at: usize,
    writer: Writer,
    /// What was written last ends its line: a `--` comment, or a comment
    /// that had a line to itself.
    ended: bool,
}

impl<'a> Layout<'a> {
    /// Statements and the comments between them, each from column 0.
    fn run(&mut self) {
        let mut before = Before::Nothing;
        while let Some(item) = self.items.get(self.at).copied() {
            let comment = item.kind == TokenKind::Comment;
            let lines = newlines(item.space);
            let breaks = match before {
                Before::Nothing => 0,
                // A comment after a statement on its line stays there.
                Before::Statement if comment && lines == 0 => 0,
                Before::Statement if comment || item.kind == TokenKind::Semicolon => lines.min(2),
                Before::Statement => 2,
                Before::Comment(ends) if lines == 0 => usize::from(ends),
                Before::Comment(_) => lines.min(2),
            };
            self.writer.push(&"\n".repeat(breaks));
            if comment {
                let trails = before == Before::Statement && lines == 0;
                if self.writer.column > 0 && !item.space.is_empty() {
                    self.writer.push(" ");
                }
                self.writer.push(item.text);
                before = Before::Comment(trails || item.ends_line() || self.own_line(self.at));
                self.at += 1;
            } else {
                self.ended = false;
                if self.starts_block(self.at) || self.opens_block(self.at) {
                    self.query();
                } else {
                    self.plain();
                }
                before = Before::Statement;
            }
        }
    }

    /// A statement that is no query: its lines stay as typed.
    fn plain(&mut self) {
        let start = self.at;
        while let Some(item) = self.items.get(self.at).copied() {
            if self.at > start {
                let strings = item.kind == TokenKind::String
                    && self.items[self.at - 1].kind == TokenKind::String;
                if strings {
                    self.writer.push(item.space);
                } else {
                    self.writer.push(&kept(item.space));
                }
            } else if self.writer.column > 0 && !item.space.is_empty() {
                self.writer.push(" ");
            }
            let text = self.cased(self.at, false);
            self.writer.push(&text);
            self.at += 1;
            if item.kind == TokenKind::Semicolon {
                break;
            }
        }
    }

    /// A query statement and its `;`.
    fn query(&mut self) {
        self.block(0, 0);
        if self.items.get(self.at).is_some() {
            // The block stopped at the statement's `;`.
            if self.ended {
                self.writer.line(0);
                self.ended = false;
            }
            self.writer.push(";");
            self.at += 1;
        }
    }

    /// Lays out a block from its first token to the `)` that ends it (left
    /// for the caller), the statement's `;` (left too) or the end.
    fn block(&mut self, base: usize, depth: usize) {
        let mut block = Block::new(base, depth);
        while let Some(item) = self.items.get(self.at).copied() {
            match item.kind {
                TokenKind::Semicolon => return,
                TokenKind::Comment => self.comment(&block),
                TokenKind::Keyword if !self.named(self.at) => self.keyword(&mut block),
                _ if item.is("(") => self.open(&mut block),
                _ if item.is(")") => {
                    let open = block
                        .frames
                        .iter()
                        .rposition(|frame| matches!(frame, Frame::Paren(_)));
                    match open {
                        Some(open) => {
                            let Frame::Paren(column) = block.frames[open] else {
                                unreachable!("the frame found is a parenthesis");
                            };
                            block.frames.truncate(open);
                            let place = if self.ended {
                                Place::Line(column)
                            } else {
                                Place::Inline
                            };
                            self.put(&mut block, place, false);
                        }
                        None if block.depth == 0 => self.put(&mut block, Place::Inline, false),
                        None => return,
                    }
                }
                _ if item.is(",") => {
                    let list = block.frames.is_empty()
                        && matches!(block.clause, Clause::Select | Clause::With);
                    self.put(&mut block, Place::Inline, false);
                    block.broken = list;
                }
                _ => self.put(&mut block, Place::Inline, false),
            }
        }
    }

    /// A `(`: a block of its own, or a parenthesis on its line.
    fn open(&mut self, block: &mut Block) {
        let nested = block.depth < MAX_DEPTH && self.opens_block(self.at);
        self.put(block, Place::Inline, false);
        let column = self.writer.column;
        if !nested {
            block.frames.push(Frame::Paren(column - 1));
            return;
        }
        self.block(column, block.depth + 1);
        if self.items.get(self.at).is_some_and(|item| item.is(")")) {
            if self.ended {
                self.writer.line(column - 1);
                self.ended = false;
            }
            self.writer.push(")");
            self.at += 1;
        }
    }

    /// A keyword that is not a name: a head, a part of a `CASE`, or a word
    /// on its line.
    fn keyword(&mut self, block: &mut Block) {
        let at = self.at;
        let text = self.items[at].text;
        let is = |word: &str| text.eq_ignore_ascii_case(word);
        if is("CASE") {
            self.put(block, Place::Inline, false);
            let column = self.writer.column - text.len();
            block.frames.push(Frame::Case(column));
            return;
        }
        if let Some(Frame::Case(column)) = block.frames.last().copied() {
            if is("WHEN") || is("ELSE") {
                return self.put(block, Place::Line(column + CASE_INDENT), false);
            }
            if is("END") {
                block.frames.pop();
                return self.put(block, Place::Line(column), true);
            }
        }
        let head = if block.frames.is_empty() {
            self.head(block, at)
        } else {
            None
        };
        let Some(words) = head else {
            return self.put(block, Place::Inline, false);
        };
        let column = block.base + RIVER.saturating_sub(text.chars().count());
        // A block's first head follows what it is in: its `(`, or a
        // comment before the statement on its line.
        let place = if block.first && !self.ended && self.writer.column > 0 {
            Place::Inline
        } else {
            Place::Line(column)
        };
        self.put(block, place, self.structure(at));
        for word in at + 1..at + words {
            self.put(block, Place::Inline, self.structure(word));
        }
        block.headed = true;
    }

    /// The head the keyword at `at` starts at the top level of `block`, as
    /// the number of its words, with the block moved on to its clause.
    /// `None` for a word that stays on its line.
    fn head(&self, block: &mut Block, at: usize) -> Option<usize> {
        let text = self.items[at].text;
        let any = |words: &[&str]| words.iter().any(|word| word.eq_ignore_ascii_case(text));
        if any(&["AND"]) && block.between {
            block.between = false;
            return None;
        }
        if any(&["AND", "OR"]) {
            return block.conditions.then_some(1);
        }
        let distinct = at > 0 && self.word(at - 1, &["DISTINCT"]);
        let (words, clause, conditions) = if any(&["SELECT"]) {
            (1, Clause::Select, false)
        } else if any(&["WITH"]) && block.first {
            let recursive = self.word(at + 1, &["RECURSIVE"]);
            (1 + usize::from(recursive), Clause::With, false)
        } else if any(&["WHERE", "HAVING"]) {
            (1, Clause::Other, true)
        } else if any(HEADS) || (any(&["FROM"]) && !distinct) {
            (1, Clause::Other, false)
        } else if any(&["GROUP", "ORDER"]) && self.word(at + 1, &["BY"]) {
            (2, Clause::Other, false)
        } else if any(&["STRAIGHT_JOIN"]) && block.clause != Clause::Select {
            (1, Clause::Join, false)
        } else if let Some(words) = self.join(at) {
            (words, Clause::Join, false)
        } else {
            block.between |= any(&["BETWEEN"]);
            block.conditions |= any(&["ON"]) && block.clause == Clause::Join;
            return None;
        };
        block.clause = clause;
        block.conditions = conditions;
        Some(words)
    }

    /// How many words the join head at `at` has: leaders, then `JOIN`.
    fn join(&self, at: usize) -> Option<usize> {
        // No join has more leaders than `NATURAL LEFT OUTER`: a longer run
        // of them is not read to its end for every word of it.
        let mut end = at;
        while end - at < 3 && self.word(end, JOIN_LEADERS) {
            end += 1;
        }
        self.word(end, &["JOIN"]).then_some(end - at + 1)
    }

    /// Whether the keyword at `index`, read as structure, is uppercased
    /// though it can be a name: not on MySQL, where it may be an alias and
    /// an alias's case can matter.
    fn structure(&self, index: usize) -> bool {
        self.dialect != Dialect::MySql && self.word(index, STRUCTURE)
    }

    /// A comment inside a query.
    fn comment(&mut self, block: &Block) {
        let item = self.items[self.at];
        let own = self.own_line(self.at);
        if own || self.ended {
            self.writer.line(block.content());
        } else if self.writer.column > 0 && !item.space.is_empty() {
            self.writer.push(" ");
        }
        self.writer.push(item.text);
        self.ended = own || item.ends_line();
        self.at += 1;
    }

    /// Writes the item at `at` and what separates it from the one before.
    /// `structure`: it is a word read as structure, uppercased whatever
    /// the list says.
    fn put(&mut self, block: &mut Block, place: Place, structure: bool) {
        let at = self.at;
        let item = self.items[at];
        let line = match place {
            // A line break here would make a comment of two minus signs.
            _ if self.dashes(at) => None,
            Place::Line(column) => Some(column),
            Place::Inline if block.broken || self.ended => Some(block.content()),
            Place::Inline => None,
        };
        if let Some(column) = line {
            self.writer.line(column);
        } else if at > 0 && self.writer.column > 0 {
            let before = self.items[at - 1];
            let strings = item.kind == TokenKind::String && before.kind == TokenKind::String;
            if strings && !item.space.is_empty() {
                // PostgreSQL joins two strings only across a line break.
                self.writer.push(item.space);
            } else if item.is(",") || item.is(")") || before.is("(") {
                // Nothing stands before a `,` or a `)`, nor after a `(`.
            } else if block.headed || before.is(",") || !item.space.is_empty() {
                self.writer.push(" ");
            }
        }
        let text = self.cased(at, structure);
        self.writer.push(&text);
        self.at += 1;
        self.ended = false;
        block.first = false;
        block.broken = false;
        block.headed = false;
    }

    /// The item's text, uppercased when it is a keyword on the list (or
    /// `structure` says so) and not a name.
    fn cased(&self, index: usize, structure: bool) -> Cow<'a, str> {
        let item = self.items[index];
        let upper = item.kind == TokenKind::Keyword
            && !self.named(index)
            && (structure
                || UPPERCASED
                    .iter()
                    .any(|word| word.eq_ignore_ascii_case(item.text)));
        if upper && item.text.bytes().any(|byte| byte.is_ascii_lowercase()) {
            Cow::Owned(item.text.to_ascii_uppercase())
        } else {
            Cow::Borrowed(item.text)
        }
    }

    /// Whether the item at `index` touches two touching minus signs on
    /// MySQL, which reads `--` as a comment once whitespace follows it.
    fn dashes(&self, index: usize) -> bool {
        let minus = |index: usize| {
            let item = self.items[index];
            item.kind == TokenKind::Operator && item.text == "-"
        };
        self.dialect == Dialect::MySql
            && index >= 2
            && self.items[index].space.is_empty()
            && minus(index - 1)
            && self.items[index - 1].space.is_empty()
            && minus(index - 2)
    }

    /// Whether the item at `index` is a name, whatever it spells: it
    /// stands beside a `.` (a qualified name), it touches a sigil or a
    /// number before it (`@from`, `:limit`, `1st`), or what touches it
    /// from the right goes on with it (`limit$x`, `offset.1st`). Such a
    /// word is no head and keeps its case, so the layout never parts it
    /// from what it touches: the check at the end, which reads the two as
    /// the tokenizer does, would not see that.
    fn named(&self, index: usize) -> bool {
        let dot = |index: usize| self.items.get(index).is_some_and(|item| item.is("."));
        let follows =
            index > 0 && self.items[index].space.is_empty() && self.items[index - 1].binds();
        let leads = self
            .items
            .get(index + 1)
            .is_some_and(|next| next.space.is_empty() && next.extends());
        dot(index + 1) || (index > 0 && dot(index - 1)) || follows || leads
    }

    /// Whether the item at `index` is one of `words` as a keyword, not a
    /// name.
    fn word(&self, index: usize, words: &[&str]) -> bool {
        self.items.get(index).is_some_and(|item| {
            item.kind == TokenKind::Keyword
                && words
                    .iter()
                    .any(|word| word.eq_ignore_ascii_case(item.text))
        }) && !self.named(index)
    }

    /// Whether a query starts at `index`.
    fn starts_block(&self, index: usize) -> bool {
        self.word(index, &["SELECT", "WITH"])
    }

    /// Whether the item at `index` is a `(` with a query inside: its
    /// first token that is not a comment starts one.
    fn opens_block(&self, index: usize) -> bool {
        self.items.get(index).is_some_and(|item| item.is("("))
            && (index + 1..self.items.len())
                .find(|inner| self.items[*inner].kind != TokenKind::Comment)
                .is_some_and(|inner| self.starts_block(inner))
    }

    /// Whether the comment at `index` had a line to itself.
    fn own_line(&self, index: usize) -> bool {
        let item = self.items[index];
        let starts = index == 0 || newlines(item.space) > 0;
        let ends = item.ends_line()
            || self
                .items
                .get(index + 1)
                .is_none_or(|next| newlines(next.space) > 0);
        starts && ends
    }
}

/// Whether two texts hold the same tokens, whitespace aside and keywords
/// compared without case: what Format promises of its result.
fn same_tokens(old: &str, old_tokens: &[Token], new: &str, new_tokens: &[Token]) -> bool {
    let visible = |tokens: &'_ [Token]| {
        tokens
            .iter()
            .filter(|token| token.kind != TokenKind::Whitespace)
            .cloned()
            .collect::<Vec<Token>>()
    };
    let (old_tokens, new_tokens) = (visible(old_tokens), visible(new_tokens));
    old_tokens.len() == new_tokens.len()
        && old_tokens.iter().zip(&new_tokens).all(|(a, b)| {
            let (before, after) = (&old[a.range.clone()], &new[b.range.clone()]);
            a.kind == b.kind
                && if a.kind == TokenKind::Keyword {
                    before.eq_ignore_ascii_case(after)
                } else {
                    before == after
                }
        })
}

/// Where a cursor at byte `cursor` of the old text stands in the new one:
/// after the same byte of the same token, or, from whitespace, before the
/// next token.
fn moved(old: &[Token], new: &[Token], cursor: usize) -> Option<usize> {
    let visible = |tokens: &'_ [Token]| {
        tokens
            .iter()
            .filter(|token| token.kind != TokenKind::Whitespace)
            .map(|token| token.range.clone())
            .collect::<Vec<Range<usize>>>()
    };
    // The bytes of tokens before the cursor, and whether it stands in a
    // token or right after one.
    let mut count = 0;
    let mut inside = false;
    for range in visible(old) {
        if range.start >= cursor {
            break;
        }
        count += cursor.min(range.end) - range.start;
        inside = cursor <= range.end;
    }
    let mut passed = 0;
    for range in visible(new) {
        if inside && count <= passed + range.len() {
            return Some(range.start + (count - passed));
        }
        if !inside && passed == count {
            return Some(range.start);
        }
        passed += range.len();
    }
    None
}
```

- [ ] **Step 5: Run the tests**

Run: `~/.cargo/bin/cargo test --locked -p tabletist-db --lib sql::format`
Expected: `test result: ok. 24 passed`.

If an expectation fails, the layout is wrong, not the test: the expected scripts follow the spec's rules and were produced by this code. Do not edit an expected string to match.

- [ ] **Step 6: Format, lint, commit**

```bash
~/.cargo/bin/cargo fmt --all --check
~/.cargo/bin/cargo clippy --locked --workspace --all-targets -- -D warnings
~/.cargo/bin/cargo test --locked --workspace --all-targets
RUSTDOCFLAGS='-D warnings' ~/.cargo/bin/cargo doc --locked --workspace --no-deps
git add crates/tabletist-db/src/sql.rs crates/tabletist-db/src/sql/format.rs
git commit -m "Format SQL in river style over its tokens

Whitespace and the case of reserved words change and nothing else: the
result is tokenized again and refused if a token differs.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 2: Ask the MySQL server about the uppercased words

`UPPERCASED` must hold only words MySQL never reads as a table name or an alias: MySQL compares those by case on Linux, so uppercasing one that is a name would point the query at another table. This test asks the server.

**Files:**
- Modify: `crates/tabletist-db/tests/mysql.rs` (append)

- [ ] **Step 1: Write the test**

Append to `crates/tabletist-db/tests/mysql.rs`:

```rust
#[tokio::test]
async fn the_words_format_uppercases_cannot_be_table_aliases() {
    use tabletist_db::Dialect;
    use tabletist_db::sql::{self, format::UPPERCASED};
    // Loads the fixture, or says the test is skipped.
    let Some(_connection) = connect().await else {
        return;
    };
    let mut conn = admin().await;
    // A keyword that can be a name is an alias here: Format leaves its
    // case alone.
    conn.query_drop("SELECT 1 FROM users first").await.unwrap();
    // The ones Format uppercases are not: the server reads each as SQL,
    // not as a name, and the statement ends too soon.
    let listed = UPPERCASED
        .iter()
        .filter(|word| sql::is_keyword(Dialect::MySql, word));
    for word in listed {
        let result = conn.query_drop(format!("SELECT 1 FROM users {word}")).await;
        assert!(
            matches!(&result, Err(mysql_async::Error::Server(error)) if error.code == 1064),
            "{word}: {result:?}"
        );
    }
    // After a dot a reserved word is a name, and this server compares
    // table names by case: Format keeps the case of a word beside a dot,
    // so the query still finds its table.
    conn.query_drop("CREATE TEMPORARY TABLE `order` (id INT)")
        .await
        .unwrap();
    let script = "select id from tabletist.order";
    let formatted = sql::format::format(Dialect::MySql, script, None, 0).unwrap();
    assert_eq!(formatted.text, "SELECT id\n  FROM tabletist.order");
    conn.query_drop(&formatted.text).await.unwrap();
    // A user variable's name touches its `@`. Format leaves it there and as
    // it is, though it spells a clause: the server reads the two as one.
    conn.query_drop("SET @from = 1").await.unwrap();
    let formatted = sql::format::format(Dialect::MySql, "select @from", None, 0).unwrap();
    assert_eq!(formatted.text, "SELECT @from");
    let value: Option<i64> = conn.query_first(&formatted.text).await.unwrap();
    assert_eq!(value, Some(1));
    conn.disconnect().await.unwrap();
}
```

- [ ] **Step 2: Run it without a server**

Run: `~/.cargo/bin/cargo test --locked -p tabletist-db --test mysql the_words_format -- --nocapture`
Expected: `skipped: TABLETIST_TEST_MYSQL_URL is not set` and `1 passed`. (Without `--nocapture` the line is hidden, and a skipped run looks like a real one.)

- [ ] **Step 3: Run it against the server**

```bash
docker compose up -d --build --wait mysql
TABLETIST_TEST_MYSQL_URL=mysql://tabletist:tabletist@localhost:53306/tabletist \
  ~/.cargo/bin/cargo test --locked -p tabletist-db --test mysql the_words_format -- --nocapture
```

Expected: `1 passed`, with no "skipped" line.

If the assertion names a word (the server took it as an alias), that word can be a name and must keep its case:

1. Remove it from `UPPERCASED` in `crates/tabletist-db/src/sql/format.rs`.
2. Add it, lower case, to both strings of the "keywords that can be names keep their case" assertion in `reserved_words_are_uppercased_and_names_are_not` (the `show any begin ...` script and its expected result). If the layout reads it as structure (a head or a join leader), add it to `STRUCTURE` instead, and fix any expected script in the tests that has it in upper case on MySQL.
3. Move it from the first list to the second in the spec's "Case" section.
4. Run Task 1's tests and this one again.

If `SELECT 1 FROM users first` fails instead, the fixture has no `users` table in the default database: use a table the fixture has (`tabletist_db::fixtures::MYSQL_SQL`).

Report plainly whether this step ran against a server. If Docker is not available, say the list is unverified and leave Step 3 unticked.

- [ ] **Step 4: Commit**

```bash
~/.cargo/bin/cargo fmt --all --check
~/.cargo/bin/cargo clippy --locked --workspace --all-targets -- -D warnings
~/.cargo/bin/cargo test --locked --workspace --all-targets
RUSTDOCFLAGS='-D warnings' ~/.cargo/bin/cargo doc --locked --workspace --no-deps
git add crates/tabletist-db/tests/mysql.rs crates/tabletist-db/src/sql/format.rs docs/superpowers/specs/2026-10-01-sql-formatter-design.md
git commit -m "Ask MySQL whether the words Format uppercases can be names

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 3: The action, the flag and the key

`Action::FormatSql` only sets a flag: `App::apply` has no `egui::Context`, and the selection and the undo history are in egui's memory. The view acts on the flag in Task 4. After this task the key is taken and the flag is set, and nothing reads it yet.

**Files:**
- Modify: `src/model.rs` (`Action`, `SqlTab`)
- Modify: `src/app.rs` (`App::apply`, tests)
- Modify: `src/ui/keys.rs` (`SHORTCUTS`, `handle`, tests)
- Modify: `src/ui/mod.rs` (tests)

- [ ] **Step 1: Write the failing tests**

In `src/app.rs`, in the tests module, before `fn arrows_move_in_a_sql_result`:

```rust
    #[test]
    fn format_sql_asks_the_editor_to_format_and_take_the_keys() {
        let mut harness = Harness::new();
        let (tab, id) = new_sql(&mut harness);
        let sent = harness.app.backend.sent.len();
        let workspace = harness.app.workspace_mut(tab).unwrap();
        workspace.sql_tab_mut(id).unwrap().focus_editor = false;
        harness.app.apply(Action::FormatSql { tab, sql_tab: id });
        assert!(sql(&harness, tab, id).format);
        assert!(sql(&harness, tab, id).focus_editor);
        // Nothing is asked of the backend, and a tab that closed since is
        // left alone.
        harness.app.apply(Action::CloseTab { tab, id });
        harness.app.apply(Action::FormatSql { tab, sql_tab: id });
        assert_eq!(harness.app.backend.sent.len(), sent);
    }
```

In `src/ui/keys.rs`, in `the_shortcut_table_covers_the_spec_map`, add `"Format SQL",` after `"Run statement / run all",`; and at the end of `the_shortcut_table_names_the_keys_that_open_tabs`:

```rust
        assert_eq!(keys("Format SQL"), Some("Mod+Shift+F"));
```

In `src/ui/mod.rs`, in `question_mark_opens_the_shortcuts_and_escape_closes_them`, after `assert!(harness.has("Quick open"));`:

```rust
        assert!(harness.has("Format SQL"));
```

Also in `src/ui/mod.rs`, in the tests module, before `fn command_period_cancels_a_sql_run` (the `#[test]` line above it included):

```rust
    const COMMAND_SHIFT: Modifiers = Modifiers::COMMAND.plus(Modifiers::SHIFT);

    #[test]
    fn command_shift_f_does_nothing_on_a_table_tab() {
        let mut harness = Harness::new();
        let tab = harness.connect_fake();
        harness.app.apply(crate::model::Action::OpenObject {
            tab,
            object: tabletist_db::ObjectRef::new("main", "users"),
            kind: tabletist_db::ObjectKind::Table,
            pin: true,
        });
        harness.answer_rows(crate::testing::page(3, false));
        // Mod+F would take the press for its own were it not consumed.
        harness.press(Key::F, COMMAND_SHIFT);
        assert!(!harness.has("Apply"), "the filter bar stays shut");
        harness.press(Key::F, Modifiers::COMMAND);
        assert!(harness.has("Apply"));
    }
```

Before this change `Mod+Shift+F` on a table tab opens the filter bar: egui ignores an extra Shift when it matches `Mod+F`. The test pins that it no longer does.

- [ ] **Step 2: Run the tests to see them fail**

Run: `~/.cargo/bin/cargo test --locked -p tabletist --lib format`
Expected: does not compile. `Action` has no variant `FormatSql` and `SqlTab` has no field `format`.

- [ ] **Step 3: The action and the flag**

In `src/model.rs`, in `enum Action`, after the `RunSql` variant:

```rust
    /// Format the editor's script, or the statements its selection
    /// overlaps.
    FormatSql {
        tab: ConnTabId,
        sql_tab: TabId,
    },
```

In `struct SqlTab`, after `focus_editor`:

```rust
    /// Focus the editor on the next frame.
    pub focus_editor: bool,
    /// Format the script on the next frame: the editor does it, where the
    /// selection and the undo history are.
    pub format: bool,
```

In `impl std::fmt::Debug for SqlTab`, after the `focus_editor` field:

```rust
            .field("focus_editor", &self.focus_editor)
            .field("format", &self.format)
```

In `SqlTab::new`, after `focus_editor: true,`:

```rust
            focus_editor: true,
            format: false,
```

- [ ] **Step 4: The reducer**

In `src/app.rs`, in `App::apply`, after the `Action::RunSql` arm:

```rust
            Action::FormatSql { tab, sql_tab } => {
                if let Some(sql) = self.sql_tab_mut(tab, sql_tab) {
                    sql.format = true;
                    // The keys go back to the editor after a click on the
                    // toolbar's button.
                    sql.focus_editor = true;
                }
            }
```

- [ ] **Step 5: The key and the shortcuts table**

In `src/ui/keys.rs`, in `SHORTCUTS`, after the `Mod+Return` line:

```rust
    ("Mod+Return, Mod+Shift+Return", "Run statement / run all"),
    ("Mod+Shift+F", "Format SQL"),
```

In `handle`, inside `ctx.input_mut`, after the `if let Some((tab, sql_tab)) = sql { ... }` block that runs SQL and before `let mut key = ...`:

```rust
        // Format is a SQL editor's. The press is taken on every tab, or
        // Mod+F, below, would take it for its own.
        let format = consume_press(input, Modifiers::COMMAND | Modifiers::SHIFT, Key::F);
        if format && let Some((tab, sql_tab)) = sql {
            actions.push(Action::FormatSql { tab, sql_tab });
        }
```

`consume_press` (already in the file) takes the press whether or not the editor has the keys, and ignores repeats of a held chord. It requires Shift, so a plain `Mod+F` still reaches the filter bar's key below.

- [ ] **Step 6: Run the tests**

Run: `~/.cargo/bin/cargo test --locked -p tabletist --lib -- format shortcut command_shift_f`
Expected: all pass, among them `format_sql_asks_the_editor_to_format_and_take_the_keys`, `command_shift_f_does_nothing_on_a_table_tab`, `the_shortcut_table_covers_the_spec_map`, `the_shortcut_table_names_the_keys_that_open_tabs`, `question_mark_opens_the_shortcuts_and_escape_closes_them`.

- [ ] **Step 7: Format, lint, commit**

```bash
~/.cargo/bin/cargo fmt --all --check
~/.cargo/bin/cargo clippy --locked --workspace --all-targets -- -D warnings
~/.cargo/bin/cargo test --locked --workspace --all-targets
RUSTDOCFLAGS='-D warnings' ~/.cargo/bin/cargo doc --locked --workspace --no-deps
git add src/model.rs src/app.rs src/ui/keys.rs src/ui/mod.rs
git commit -m "Take Mod+Shift+F for formatting a SQL editor

The press sets a flag on the tab; the editor acts on it next.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 4: The editor formats

The view takes the flag before it draws the `TextEdit`. It reads the field's selection (egui counts cursors in characters; the formatter in bytes), formats, and writes two states into the field's undo history: the text as typed with its selection, then the formatted text with its cursor. egui's `Undoer::undo` pops a state equal to the current one and returns the one under it, so one `Mod+Z` right after Format gives the typed text back, and `Mod+Shift+Z` formats again. `Undoer::add_undo` adds nothing when the state equals the last one. It also never empties the redo stack (only `feed_state` does, when the state it is fed differs from the last undo point), so the formatted state is fed between the two: without that, "undo, Format, redo" brings back what the undo had put aside.

**Files:**
- Modify: `src/ui/sql_text.rs`
- Modify: `src/ui/mod.rs` (tests)

- [ ] **Step 1: Write the failing tests**

In `src/ui/sql_text.rs`, in the tests module, before `fn a_character_index_becomes_a_byte_offset` (its `#[test]` line included):

```rust
    #[test]
    fn a_byte_offset_becomes_a_character_index() {
        assert_eq!(char_index("SELECT 1", 0), 0);
        assert_eq!(char_index("SELECT 1", 8), 8);
        // Two bytes each.
        assert_eq!(char_index("żółw 1", 7), 4);
        assert_eq!(char_index("żółw 1", 9), 6);
        // There and back.
        for chars in 0..=6 {
            assert_eq!(char_index("żółw 1", byte_offset("żółw 1", chars)), chars);
        }
        // Past the end: the end.
        assert_eq!(char_index("żółw", 99), 4);
    }
```

In `src/ui/mod.rs`, in the tests module, before `const COMMAND_SHIFT` (added in Task 3), the helper:

```rust
    /// Selects the bytes `range` of the active SQL editor's text, as a
    /// drag over them would.
    fn select_sql(
        harness: &mut Harness,
        tab: crate::model::ConnTabId,
        range: std::ops::Range<usize>,
    ) {
        // A field without the keys drops its selection when it is drawn.
        assert!(harness.ctx.text_edit_focused());
        let sql = active_sql(harness, tab);
        let chars = |byte: usize| sql.text[..byte].chars().count();
        let selection = egui::text::CCursorRange::two(
            egui::text::CCursor::new(chars(range.start)),
            egui::text::CCursor::new(chars(range.end)),
        );
        let id = crate::ui::sql_text::editor_id(tab, sql.id);
        let mut state = egui::TextEdit::load_state(&harness.ctx, id).unwrap_or_default();
        state.cursor.set_char_range(Some(selection));
        egui::TextEdit::store_state(&harness.ctx, id, state);
        harness.settle();
    }
```

and after `const COMMAND_SHIFT`, the tests:

```rust
    #[test]
    fn command_shift_f_formats_the_script_and_the_editor_keeps_the_keys() {
        let mut harness = Harness::new();
        let tab = harness.connect_fake();
        harness.press(Key::T, Modifiers::COMMAND);
        type_text(&mut harness, "select a,b from t");
        harness.press(Key::F, COMMAND_SHIFT);
        let formatted = "SELECT a,\n       b\n  FROM t";
        let sql = active_sql(&harness, tab);
        assert_eq!(sql.text, formatted);
        assert_eq!(
            sql.cursor,
            formatted.len(),
            "the cursor is still at the end"
        );
        assert!(harness.ctx.text_edit_focused());
        // Typing goes on where the cursor is.
        type_text(&mut harness, ";");
        assert_eq!(active_sql(&harness, tab).text, format!("{formatted};"));
        // With the keys given up (Esc) it formats too, and takes them back.
        harness.press(Key::Escape, Modifiers::NONE);
        assert!(!harness.ctx.text_edit_focused());
        set_sql(&mut harness, tab, "select 1", 0);
        harness.press(Key::F, COMMAND_SHIFT);
        assert_eq!(active_sql(&harness, tab).text, "SELECT 1");
        assert!(harness.ctx.text_edit_focused());
    }

    #[test]
    fn one_undo_gives_back_the_script_as_typed() {
        let mut harness = Harness::new();
        let tab = harness.connect_fake();
        harness.press(Key::T, Modifiers::COMMAND);
        let typed = "select a,b from t";
        let formatted = "SELECT a,\n       b\n  FROM t";
        type_text(&mut harness, typed);
        harness.press(Key::F, COMMAND_SHIFT);
        assert_eq!(active_sql(&harness, tab).text, formatted);
        harness.press(Key::Z, Modifiers::COMMAND);
        let sql = active_sql(&harness, tab);
        assert_eq!(sql.text, typed);
        assert_eq!(sql.cursor, typed.len(), "and the cursor where it was");
        // Redo formats it again.
        harness.press(Key::Z, COMMAND_SHIFT);
        assert_eq!(active_sql(&harness, tab).text, formatted);
        // Formatting what is formatted adds nothing to undo: one undo is
        // still all it takes.
        harness.press(Key::F, COMMAND_SHIFT);
        harness.press(Key::F, COMMAND_SHIFT);
        assert_eq!(active_sql(&harness, tab).text, formatted);
        harness.press(Key::Z, Modifiers::COMMAND);
        assert_eq!(active_sql(&harness, tab).text, typed);
    }

    #[test]
    fn format_leaves_nothing_to_redo_as_an_edit_does() {
        let mut harness = Harness::new();
        let tab = harness.connect_fake();
        harness.press(Key::T, Modifiers::COMMAND);
        let typed = "select a,b from t";
        let formatted = "SELECT a,\n       b\n  FROM t";
        type_text(&mut harness, typed);
        harness.press(Key::F, COMMAND_SHIFT);
        harness.press(Key::Z, Modifiers::COMMAND);
        assert_eq!(active_sql(&harness, tab).text, typed);
        // Formatted again, what that undo left to redo is gone: a redo
        // changes nothing, and one undo is still all it takes.
        harness.press(Key::F, COMMAND_SHIFT);
        harness.press(Key::Z, COMMAND_SHIFT);
        assert_eq!(active_sql(&harness, tab).text, formatted);
        harness.press(Key::Z, Modifiers::COMMAND);
        assert_eq!(active_sql(&harness, tab).text, typed);
    }

    #[test]
    fn format_brings_the_cursor_into_view() {
        let mut harness = Harness::new();
        let tab = harness.connect_fake();
        harness.press(Key::T, Modifiers::COMMAND);
        // One line that Format lays out as more lines than the pane shows.
        let columns: Vec<String> = (0..80).map(|n| format!("c{n}")).collect();
        type_text(
            &mut harness,
            &format!("select {} from t", columns.join(",")),
        );
        let id = active_sql(&harness, tab).id;
        let scrolled = |harness: &Harness| {
            crate::ui::sql_text::scroll_offset(&harness.ctx, tab, id).expect("a scroll area")
        };
        assert_eq!(scrolled(&harness).y, 0.0);
        harness.press(Key::F, COMMAND_SHIFT);
        harness.finish_animations();
        let sql = active_sql(&harness, tab);
        assert_eq!(sql.text.lines().count(), 81);
        assert_eq!(sql.cursor, sql.text.len(), "the cursor is on the last line");
        assert!(scrolled(&harness).y > 0.0, "and that line is in view");
    }

    #[test]
    fn format_with_a_selection_formats_the_statements_it_touches() {
        let mut harness = Harness::new();
        let tab = harness.connect_fake();
        harness.press(Key::T, Modifiers::COMMAND);
        // Letters of two bytes each before the selection: egui counts a
        // cursor in characters, Format in bytes.
        let typed = "select 'żółw' ;\nselect a,b from t ;\nselect 3 ;";
        let formatted = "select 'żółw' ;\nSELECT a,\n       b\n  FROM t;\nselect 3 ;";
        type_text(&mut harness, typed);
        let list = typed.find("a,b").unwrap();
        select_sql(&mut harness, tab, list..list + 3);
        let id = crate::ui::sql_text::editor_id(tab, active_sql(&harness, tab).id);
        let selection = |harness: &Harness| {
            let state = egui::TextEdit::load_state(&harness.ctx, id).unwrap();
            state.cursor.char_range().unwrap()
        };
        let selected = selection(&harness);
        assert!(!selected.is_empty());
        harness.press(Key::F, COMMAND_SHIFT);
        let sql = active_sql(&harness, tab);
        assert_eq!(sql.text, formatted);
        // The cursor is where the selection ended, after the `b`, and
        // nothing is selected.
        let cursor = "select 'żółw' ;\nSELECT a,\n       b";
        assert_eq!(&sql.text[..sql.cursor], cursor);
        assert!(selection(&harness).is_empty());
        // Undo gives the selection back with the text.
        harness.press(Key::Z, Modifiers::COMMAND);
        assert_eq!(active_sql(&harness, tab).text, typed);
        assert_eq!(selection(&harness), selected);
        // Formatted again (redo), typing adds to the text at the cursor.
        harness.press(Key::Z, COMMAND_SHIFT);
        type_text(&mut harness, "2");
        assert_eq!(
            active_sql(&harness, tab).text,
            formatted.replace("       b\n", "       b2\n")
        );
    }
```

- [ ] **Step 2: Run the tests to see them fail**

Run: `~/.cargo/bin/cargo test --locked -p tabletist --lib -- format undo character_index`
Expected: does not compile. The compiler cannot find `char_index`. (With the unit test left out, the UI tests fail on their first `assert_eq!`: the text is still as typed.)

- [ ] **Step 3: The editor's format step**

In `src/ui/sql_text.rs`, change the import:

```rust
use egui::text::{CCursor, CCursorRange};
```

After `fn byte_offset`, add:

```rust
/// The character index, as egui counts a cursor, of the byte offset
/// `bytes` in `text`.
fn char_index(text: &str, bytes: usize) -> usize {
    text.char_indices()
        .take_while(|(byte, _)| *byte < bytes)
        .count()
}

/// Formats the script, or the statements the field's selection overlaps,
/// as one step of the field's undo history. Returns whether it changed
/// the text: it does not when Format has nothing to change.
fn format_script(ui: &Ui, sql_tab: &mut SqlTab, field: &Field<'_>) -> bool {
    let mut state = egui::TextEdit::load_state(ui.ctx(), field.id).unwrap_or_default();
    // A field that never had the keys has no cursor of its own.
    let typed = state.cursor.char_range();
    let (selection, cursor) = match typed {
        Some(range) => {
            let [start, end] = range.sorted_cursors();
            let byte = |cursor: CCursor| byte_offset(&sql_tab.text, cursor.index.0);
            (Some(byte(start)..byte(end)), byte(range.primary))
        }
        None => (None, sql_tab.cursor),
    };
    let Some(formatted) = sql::format::format(field.dialect, &sql_tab.text, selection, cursor)
    else {
        return false;
    };
    let at = |text: &str, byte: usize| CCursorRange::one(CCursor::new(char_index(text, byte)));
    let before = typed.unwrap_or_else(|| at(&sql_tab.text, sql_tab.cursor));
    let after = at(&formatted.text, formatted.cursor);
    // The text as typed and the text as formatted, both: undo gives the
    // first back whole, and redo the second. Between the two the formatted
    // text is fed as an edit is, which empties what an earlier undo left
    // to redo: adding an undo point alone would keep it. (The time fed
    // does not matter: the next line settles the state.)
    let formatted_state = (after, formatted.text.clone());
    let mut undoer = state.undoer();
    undoer.add_undo(&(before, sql_tab.text.clone()));
    undoer.feed_state(ui.input(|input| input.time), &formatted_state);
    undoer.add_undo(&formatted_state);
    state.set_undoer(undoer);
    state.cursor.set_char_range(Some(after));
    egui::TextEdit::store_state(ui.ctx(), field.id, state);
    sql_tab.text = formatted.text;
    sql_tab.cursor = formatted.cursor;
    true
}
```

In `fn edit`, right after its first line:

```rust
    let focus = std::mem::take(&mut sql_tab.focus_editor);
    // Before the field is drawn, so this frame shows the formatted text.
    let formatted = std::mem::take(&mut sql_tab.format) && format_script(ui, sql_tab, field);
```

and after the `if focus Ellipsis` block that follows the `TextEdit`:

```rust
    if formatted {
        // The cursor's line moved with the layout. The field scrolls to
        // its cursor after an edit of its own, not after this one, so
        // bring it into view here; and ask for the frame that shows the
        // footer and the gutter the new text.
        if let Some(range) = output.state.cursor.range(&output.galley) {
            let cursor = output.galley.pos_from_cursor(range.primary);
            ui.scroll_to_rect(cursor.translate(output.galley_pos.to_vec2()), None);
        }
        ui.ctx().request_repaint();
    }
```

The field scrolls to its cursor after an edit of its own (`response.changed()`), not after a change made for it, so without this a statement that Format lays out over many lines can leave the cursor below the pane.

Before `pub fn layouter`, a helper the scroll test reads:

```rust
/// How far the editor `id` is scrolled.
#[cfg(test)]
pub fn scroll_offset(ctx: &egui::Context, tab: ConnTabId, id: TabId) -> Option<egui::Vec2> {
    let ScrollId(area) = ctx.data(|data| data.get_temp(editor_id(tab, id)))?;
    egui::scroll_area::State::load(ctx, area).map(|state| state.offset)
}
```

`sql` here is `tabletist_db::sql`, already imported in this file. A field that never had the keys has no cursor in egui's state: `SqlTab.cursor` stands in for it, for Format and for the state undo restores.

- [ ] **Step 4: Run the tests**

Run: `~/.cargo/bin/cargo test --locked -p tabletist --lib -- format undo character_index`
Expected: all pass, among them `command_shift_f_formats_the_script_and_the_editor_keeps_the_keys`, `one_undo_gives_back_the_script_as_typed`, `format_leaves_nothing_to_redo_as_an_edit_does`, `format_brings_the_cursor_into_view`, `format_with_a_selection_formats_the_statements_it_touches`, `a_byte_offset_becomes_a_character_index`.

- [ ] **Step 5: Format, lint, commit**

```bash
~/.cargo/bin/cargo fmt --all --check
~/.cargo/bin/cargo clippy --locked --workspace --all-targets -- -D warnings
~/.cargo/bin/cargo test --locked --workspace --all-targets
RUSTDOCFLAGS='-D warnings' ~/.cargo/bin/cargo doc --locked --workspace --no-deps
git add src/ui/sql_text.rs src/ui/mod.rs
git commit -m "Format the SQL editor's script as one undo step

With a selection, only the statements it overlaps. The cursor keeps its
place among the tokens.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 5: The Format button

macOS and the standard look: after Run all, a divider and a borderless Format button with its key. The terminal look has no button (its artboard has none); the key works there.

**Files:**
- Modify: `src/ui/widgets.rs` (`ButtonSpec`)
- Modify: `src/ui/sql_editor.rs` (`format_keys`, `mac_toolbar`)
- Modify: `src/ui/mod.rs` (tests)

- [ ] **Step 1: Write the failing tests**

In `src/ui/mod.rs`, in the tests module, after `fn format_with_a_selection_formats_the_statements_it_touches`:

```rust
    #[test]
    fn the_format_button_formats_and_gives_the_keys_back() {
        for look in [crate::theme::Look::standard(), crate::theme::Look::macos()] {
            let (mut harness, tab) = sql_harness(look);
            // Wide enough for the buttons' keys.
            harness.size.x = 1600.0;
            harness.settle();
            type_text(&mut harness, "select 1");
            let keys = if look == crate::theme::Look::macos() {
                "⇧⌘F"
            } else {
                "Ctrl+Shift+F"
            };
            assert!(painted(&harness, keys), "{keys} in {}", look.name);
            // The editor gives the keys up (Esc); the button formats and
            // gives them back.
            harness.press(Key::Escape, Modifiers::NONE);
            assert!(!harness.ctx.text_edit_focused(), "{}", look.name);
            harness.click("Format");
            assert_eq!(active_sql(&harness, tab).text, "SELECT 1", "{}", look.name);
            assert!(harness.ctx.text_edit_focused(), "{}", look.name);
        }
        // The terminal has the key alone.
        let (mut harness, tab) = sql_harness(crate::theme::Look::omarchy());
        harness.settle();
        type_text(&mut harness, "select 1");
        assert!(!harness.has("Format"));
        harness.press(Key::F, COMMAND_SHIFT);
        assert_eq!(active_sql(&harness, tab).text, "SELECT 1");
    }
```

Replace `tab_walks_the_toolbar_in_the_order_it_reads` with (Format joins the order; `order` becomes a slice, as the two looks' lists differ in length):

```rust
    #[test]
    fn tab_walks_the_toolbar_in_the_order_it_reads() {
        for look in crate::theme::Look::ALL {
            let (mut harness, _tab) = sql_harness(look);
            // The terminal's toolbar leads with its menus, the others'
            // with Run.
            let order: &[&str] = if look.terminal {
                &["Limit", "Timeout", "Run", "Run all"]
            } else {
                &["Run", "Run all", "Format", "Limit", "Timeout"]
            };
            let role = if look.terminal {
                egui::accesskit::Role::ComboBox
            } else {
                egui::accesskit::Role::Button
            };
            focus(&mut harness, order[0], role);
            let mut reached = vec![focused_name(&harness.settle())];
            for _ in 1..order.len() {
                harness.press(Key::Tab, Modifiers::NONE);
                reached.push(focused_name(&harness.settle()));
            }
            assert_eq!(reached, order, "{}", look.name);
        }
    }
```

Replace `the_toolbar_gives_way_in_order_and_shortens_its_menus_last` with (a sixth state, the Format button, which goes after the note and before the menus shorten):

```rust
    #[test]
    fn the_toolbar_gives_way_in_order_and_shortens_its_menus_last() {
        // The keys go first: the help and the terminal's status line say
        // them too, while the menus' words are all that say what "1,000"
        // and "30 s" are. So the labels read in full until only they are
        // left to give way.
        for look in crate::theme::Look::ALL {
            let (mut harness, _tab) = sql_harness(look);
            let (keys, note, full, short) = if look.terminal {
                ("ctrl+enter", "read-only transaction", "limit 1000", "1000")
            } else if look == crate::theme::Look::macos() {
                ("⌘↩", "Read-only transaction", "Limit 1,000", "1,000")
            } else {
                (
                    "Ctrl+Enter",
                    "Read-only transaction",
                    "Limit 1,000",
                    "1,000",
                )
            };
            // Format's key, which goes when the run buttons' keys do.
            let format_keys = if look == crate::theme::Look::macos() {
                "⇧⌘F"
            } else {
                "Ctrl+Shift+F"
            };
            // Every state the toolbar passes through as the window narrows,
            // in the order it meets them.
            let mut states = Vec::new();
            for width in (500..=1600).rev().step_by(10) {
                harness.size.x = width as f32;
                let tree = harness.settle();
                let has = |role, name: &str| crate::testing::node(&tree, name, role).is_some();
                let state = [
                    painted(&harness, keys),
                    painted(&harness, note),
                    // The terminal has no Format button.
                    look.terminal || has(egui::accesskit::Role::Button, "Format"),
                    painted(&harness, full),
                    // The terminal's title, which the others do not have.
                    !look.terminal || has(egui::accesskit::Role::Label, "query 1"),
                    has(egui::accesskit::Role::ComboBox, "Limit"),
                ];
                assert!(
                    has(egui::accesskit::Role::Button, "Run")
                        && has(egui::accesskit::Role::Button, "Run all"),
                    "the run buttons at {width} in {}",
                    look.name
                );
                assert_eq!(
                    painted(&harness, format_keys),
                    state[0] && !look.terminal,
                    "{format_keys} at {width} in {}",
                    look.name
                );
                // A menu reads in full or short, never neither.
                assert_eq!(
                    state[5] && !state[3],
                    painted(&harness, short),
                    "{short} at {width} in {}",
                    look.name
                );
                if states.last() != Some(&state) {
                    states.push(state);
                }
            }
            // keys, note, Format, full labels, title, menus
            let mut expected = vec![
                [true, true, true, true, true, true],
                [false, true, true, true, true, true],
                [false, false, true, true, true, true],
            ];
            if look.terminal {
                // The title goes before the labels shorten: the tab says
                // it too.
                expected.push([false, false, true, true, false, true]);
            } else {
                // Format goes before them: its key formats too.
                expected.push([false, false, false, true, true, true]);
            }
            let (format, title) = (look.terminal, !look.terminal);
            expected.push([false, false, format, false, title, true]);
            expected.push([false, false, format, false, title, false]);
            assert_eq!(states, expected, "{}", look.name);
        }
    }
```

- [ ] **Step 2: Run the tests to see them fail**

Run: `~/.cargo/bin/cargo test --locked -p tabletist --lib -- toolbar format_button`
Expected: three failures: `nothing labelled "Format"` in `the_format_button_formats_and_gives_the_keys_back`; `tab_walks_the_toolbar_in_the_order_it_reads` reaches `Limit` where it expects `Format`; `the_toolbar_gives_way_in_order_and_shortens_its_menus_last` sees no Format button in the macOS and standard looks.

- [ ] **Step 3: A quiet button**

In `src/ui/widgets.rs`, in `struct ButtonSpec`, after `hint: bool,`:

```rust
    /// No border, and the text in the secondary colour.
    quiet: bool,
```

In `ButtonSpec::new`, after `hint: false,`:

```rust
            hint: false,
            quiet: false,
```

After the `hint` method:

```rust
    /// A secondary button that stands back from the ones beside it: no
    /// border, a fill only under the pointer, the text in the secondary
    /// colour.
    pub fn quiet(mut self) -> Self {
        self.quiet = true;
        self
    }
```

In `show_at`, after the `let (text, shortcut) = if self.hint ...` statement and before `let painter = ui.painter();`:

```rust
        let (fill, border, text) = if self.quiet && self.kind == ButtonKind::Secondary {
            // Under the pointer, the fill its look gives a secondary button.
            let fill = if hovered { fill } else { Color32::TRANSPARENT };
            (fill, None, palette.secondary)
        } else {
            (fill, border, text)
        };
```

- [ ] **Step 4: The button in the toolbar**

In `src/ui/sql_editor.rs`, after `fn run_keys`:

```rust
/// The keys that format the script, as the macOS and the standard look
/// spell shortcuts.
fn format_keys(look: &Look) -> String {
    let command = look.command_key();
    if command == "⌘" {
        format!("⇧{command}F")
    } else {
        format!("{command}Shift+F")
    }
}
```

Replace `fn mac_toolbar` (its doc comment included) with:

```rust
/// macOS: Run and Run all, a divider and Format; at the right the
/// transaction note, then the Limit and Timeout menus.
fn mac_toolbar(ui: &mut Ui, rect: Rect, bar: &Bar<'_>, actions: &mut Vec<Action>) {
    let Bar {
        locale,
        look,
        palette,
        ..
    } = *bar;
    let center = rect.top() + (rect.height() - 1.0) / 2.0;
    let (left, right) = (rect.left() + SIDE, rect.right() - SIDE);
    let labels = [
        gettext(locale, "Run"),
        gettext(locale, "Run all"),
        gettext(locale, "Format"),
    ];
    let (run_keys, all_keys) = run_keys(look);
    let format_keys = format_keys(look);
    // Run: 14 at its sides, a 12 pt arrow, 8 apart. Run all: 12 and 6.
    // Format: 10, and no border.
    let buttons = |keys: bool| {
        let run = ButtonSpec::new(&labels[0])
            .icon(Icon::Play)
            .icon_size(12.0)
            .primary()
            .padding(14.0)
            .gap(8.0);
        let all = ButtonSpec::new(&labels[1]);
        let format = ButtonSpec::new(&labels[2]).quiet().padding(10.0);
        if keys {
            [
                run.shortcut(&run_keys),
                all.shortcut(&all_keys),
                format.shortcut(&format_keys),
            ]
        } else {
            [run, all, format]
        }
    };
    // The note: 10 at its sides, an 11 pt lock, 6, the words.
    let note = gettext(locale, "Read-only transaction");
    let note_width =
        10.0 + 11.0 + 6.0 + TextRole::Secondary.width(ui.ctx(), look.faces, &note) + 10.0;
    // Everything 8 apart, and 16 between the two ends (8 at the tightest).
    // What gives way as the room runs out: the buttons' keys (the help
    // says them too), then the note, then Format (its key formats too),
    // and only then the menus' words, which alone say what "1,000" and
    // "30 s" are; then the menus' chevrons, and last the menus (the run
    // buttons stay).
    let shapes = [(false, true), (true, true), (true, false)]
        .map(|(short, chevron)| MenuShape { short, chevron });
    // Each width is measured once.
    let menu_sizes = shapes.map(|shape| menu_widths(ui, shape, bar));
    let widths = [false, true].map(|keys| buttons(keys).map(|b| b.width(ui, look)));
    let room = right - left;
    // The divider before Format: a rule 20 tall with 4 at its sides.
    let divider = 4.0 + 1.0 + 4.0;
    let needs = |shape: usize, badge: bool, keys: bool, format: bool| {
        let [run, all, format_width] = widths[usize::from(keys)];
        let format = if format {
            8.0 + divider + 8.0 + format_width
        } else {
            0.0
        };
        let badge = if badge { note_width + 8.0 } else { 0.0 };
        let between = if shapes[shape].chevron { 16.0 } else { 8.0 };
        run + 8.0 + all + format + between + badge + menu_sizes[shape].iter().sum::<f32>() + 8.0
    };
    let fit = [
        (0, true, true, true),
        (0, true, false, true),
        (0, false, false, true),
        (0, false, false, false),
        (1, false, false, false),
        (2, false, false, false),
    ]
    .into_iter()
    .find(|(shape, badge, keys, format)| needs(*shape, *badge, *keys, *format) <= room);
    let keys = fit.is_some_and(|(_, _, keys, _)| keys);
    // Where each piece sits, then the pieces from left to right: the Tab
    // key and screen readers meet them in the order they are made.
    let [run, all, format] = buttons(keys);
    let [run_width, all_width, format_width] = widths[usize::from(keys)];
    let place = |x: f32, width: f32| Rect::from_min_size(pos2(x, center - 16.0), vec2(width, 32.0));
    let run_place = place(left, run_width);
    let all_place = place(run_place.right() + 8.0, all_width);
    run_button(ui, run, run_place, false, bar, actions);
    run_button(ui, all, all_place, true, bar, actions);
    let Some((shape, badge, _, formats)) = fit else {
        return;
    };
    if formats {
        let rule = all_place.right() + 8.0 + divider / 2.0;
        widgets::vline(
            ui,
            rule,
            egui::Rangef::new(center - 10.0, center + 10.0),
            palette.outline,
        );
        let format_place = place(all_place.right() + 8.0 + divider + 8.0, format_width);
        let response = format.show_at(ui, format_place, look, palette);
        if response
            .on_hover_text(gettext(locale, "Format the SQL"))
            .clicked()
        {
            actions.push(Action::FormatSql {
                tab: bar.tab,
                sql_tab: bar.id,
            });
        }
    }
    let [limit_width, timeout_width] = menu_sizes[shape];
    let shape = shapes[shape];
    let menu_rect = |right: f32, width: f32| {
        Rect::from_min_size(pos2(right - width, center - 14.0), vec2(width, 28.0))
    };
    let timeout = menu_rect(right, timeout_width);
    let limit = menu_rect(timeout.left() - 8.0, limit_width);
    if badge {
        let pill = Rect::from_min_size(
            pos2(limit.left() - 8.0 - note_width, center - 13.0),
            vec2(note_width, 26.0),
        );
        ui.painter()
            .rect_filled(pill, CornerRadius::same(13), palette.surface);
        Icon::Lock.image(palette.secondary, 11.0).paint_at(
            ui,
            Rect::from_center_size(pos2(pill.left() + 15.5, center), vec2(11.0, 11.0)),
        );
        widgets::paint_text(
            ui,
            pill.left() + 27.0,
            center,
            Text::one(look, TextRole::Secondary, &note, palette.secondary),
        );
        explain_note(ui, pill, locale);
    }
    menus(ui, [limit, timeout], shape, bar, actions);
}
```

What changed in it: a third label and button (`quiet`, 10 of padding); `needs` and the `fit` table take a fourth flag, whether Format shows, placed so that Format goes after the note and before the menus shorten; the run buttons are placed by name instead of in a loop; and Format is drawn after them, so the Tab key and screen readers meet the bar from left to right.

The terminal's toolbar (`terminal_toolbar`) is not touched.

- [ ] **Step 5: Run the tests**

Run: `~/.cargo/bin/cargo test --locked -p tabletist --lib -- toolbar format menus_stay`
Expected: all pass, among them `the_format_button_formats_and_gives_the_keys_back`, `tab_walks_the_toolbar_in_the_order_it_reads`, `the_toolbar_gives_way_in_order_and_shortens_its_menus_last`, `a_toolbar_too_narrow_for_its_menus_keeps_run_and_run_all`, `the_menus_stay_in_reach_in_the_smallest_window_beside_the_widest_sidebar`.

- [ ] **Step 6: Look at it**

Run the app (`~/.cargo/bin/cargo run`), open a SQL editor (`Mod+T`) on any connection, and compare the toolbar with the macOS "SQL editor" artboard by eye: the divider after Run all, Format without a border in the muted colour, its key beside it. Narrow the window and watch the pieces give way in order. Type a query, press `Mod+Shift+F`, then `Mod+Z`. Nothing here is a test; screenshots stay local.

- [ ] **Step 7: Format, lint, commit**

```bash
~/.cargo/bin/cargo fmt --all --check
~/.cargo/bin/cargo clippy --locked --workspace --all-targets -- -D warnings
~/.cargo/bin/cargo test --locked --workspace --all-targets
RUSTDOCFLAGS='-D warnings' ~/.cargo/bin/cargo doc --locked --workspace --no-deps
git add src/ui/widgets.rs src/ui/sql_editor.rs src/ui/mod.rs
git commit -m "Add a Format button to the SQL editor's toolbar

In the macOS and standard looks, after a divider. It gives way after the
read-only note and before the menus shorten.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 6: Docs and the full check

**Files:**
- Modify: `README.md`
- Modify: `docs/superpowers/specs/2026-10-01-sql-formatter-design.md`

- [ ] **Step 1: README**

Under "What it does", extend the SQL editor's item:

```markdown
- A SQL editor per connection (Cmd/Ctrl+T): run the statement at the cursor
  (Cmd/Ctrl+Return) or the whole script, with a row limit and a timeout.
  Every run happens in a read-only transaction that is rolled back, and
  statements that would leave it are refused. Format (Cmd/Ctrl+Shift+F)
  lays queries out in river style and uppercases reserved words, in the
  selection's statements or the whole script.
```

- [ ] **Step 2: The spec's status**

In `docs/superpowers/specs/2026-10-01-sql-formatter-design.md`, change the status line (the third line of the file) to:

```markdown
Date: 2026-10-01. Status: implemented.
```

If anything was built differently from the spec (a word taken off the list in Task 2, say), make the spec say what was built.

- [ ] **Step 3: Full checks**

```bash
~/.cargo/bin/cargo fmt --all --check
~/.cargo/bin/cargo clippy --locked --workspace --all-targets -- -D warnings
~/.cargo/bin/cargo test --locked --workspace --all-targets
RUSTDOCFLAGS='-D warnings' ~/.cargo/bin/cargo doc --locked --workspace --no-deps
docker compose up -d --build --wait postgres mysql
TABLETIST_TEST_PG_URL=postgres://tabletist:tabletist@localhost:55432/tabletist TABLETIST_TEST_MYSQL_URL=mysql://tabletist:tabletist@localhost:53306/tabletist ~/.cargo/bin/cargo test --locked --workspace
```

Expected: all pass. Report which suites ran against servers, whether Task 2's server step ran, and that macOS and Windows were only compiled if so.

- [ ] **Step 4: Commit**

```bash
git add README.md docs/superpowers/specs/2026-10-01-sql-formatter-design.md
git commit -m "Describe Format in the README and the spec

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```
