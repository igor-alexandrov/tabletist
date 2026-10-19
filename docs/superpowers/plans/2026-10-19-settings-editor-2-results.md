# Settings › Editor, step 2: results Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** When the row limit cut a SQL editor result, the results say "Limited to 1,000 rows · Show all", and Show all runs that one statement again with no limit. When the timeout stopped a run, they say "Cancelled after 30 s · Run without timeout", and the link runs it again with no timeout. The Row limit row of Settings › Editor gains "Show all fetches the rest".

**Architecture:** A run remembers what it was sent with (`RunCaps`: the fetch cap and the timeout). `SqlTab` says what each link would send (`cut_statement`, `timed_out_run`), so the view and `App` agree on when a link is offered. Two new actions, `ShowAllRows` and `RunWithoutTimeout`, send through `App::send_run`, which now takes the caps; each link lifts its own cap and keeps the run's other one. In `src/ui/sql_results.rs` a `Link` is drawn after our words in three places: the strip under a cut result, the Results note of a stopped run, and the Messages line that says the cancel. In the terminal look each link is a key hint, and its letter (`a`, `t`) is a keymap command read as the refused-write card's letters are.

**Tech Stack:** Rust, egui (crmne fork) with AccessKit, headless tests through `src/testing.rs`.

**Spec:** `docs/superpowers/specs/2026-10-19-settings-editor-design.md`, section "Steps", item 2 "Results". Step 1 is PR igor-alexandrov/tabletist#181, still open when this was written: this plan starts from its head (`claude/settings-editor-tab-26bffe`, `0e73510`) and is rebased on `main` once #181 is merged.

---

## What the boards draw

Read again on 2026-10-19: the "SQL editor" boards (`macOS – SQL editor`, `Omarchy – SQL editor`), the transaction boards, the empty, loading and error states, Components and Awkward values. **No board draws a cut result or a run the timeout stopped.** The words are the spec's. The looks come from what the boards do draw elsewhere:

- macOS: a link in the accent colour after our words, as "Show all · open in viewer" (Awkward values), "Remove last filter" and "Cancel" (Empty and loading states). In the app that is `ButtonSpec::link()`, as the start screen's Clear has it.
- Omarchy: an action is its words with its key, `[enter] view all`, `[R] reload`, `[a] apply` (States). In the app that is `states::key_button`, as the refused-write card's `e` and `w` are, with a letter of the keymap that takes it while the link is on screen.

## Where this departs from the prompt and the boards

For a yes or a no before code:

| The prompt or a board asks | Step 2 builds | Why |
|---|---|---|
| "Both looks (macOS sheet and Omarchy status line)" from the SQL editor boards | The strip under a cut result and the cancel's words, where the app says them today, in each look's way (above). The Omarchy status line keeps its run summary (`ln 12:21 · 1,000 rows · 14 ms · rolled back`) | No board draws either state, and the status line is one line of the run's facts with no room for an action. Say if the words belong in the status line instead. |
| Omarchy: a link to click | A key hint, `show all a` and `run without timeout t`, which a click also takes. The letters are the keymap's (`ShowAllRows`, `RunWithoutTimeout`), read with the keyboard out of the editor's text, as the card's are | Every action on the Omarchy boards names its key. `a` for all, `t` for timeout; neither is taken where the results are. |
| Show all "runs again, with no limit, the statement that was cut" | Offered only for a run that only read. A run sent to write says "Limited to 1,000 rows" with no link | The prompt: "a write run must not be repeated". A run sent to write ran in a transaction a lone SELECT would not see. |
| (when the links are offered) | While the editor holds the text that ran and no run is in flight, on a session that takes commands: as the card's Run again | What would be sent is then what the editor shows, and a statement's line still names the editor's line. A link goes away as you type, and comes back if you undo. |
| "Run without timeout runs the cancelled run again (it was rolled back)" | Offered only where nothing of the run was written: a read-only run, or one sent to write whose transaction rolled back whole. Not a MySQL run that committed part of itself (`CREATE` and the like), nor one the database could not roll back fully. Sent in the transaction the tab's mode gives now, as Run is | Sending again what is written would write it twice. |
| (a link of a link) | Each link lifts its own cap and keeps the run's other one: Show all keeps the run's timeout; Run without timeout keeps the run's row limit. A Show all that timed out runs again with no limit and no timeout | Otherwise Run without timeout on a Show all would cut the rows again, and Show all on that would bring the timeout back: the case Show all is most likely to meet. |
| "Cancelled after 30 s" | `Settings`' words for a time, as the toolbar's Timeout menu writes it: "Cancelled after 30 s", "Cancelled after 1 min"; the terminal's "cancelled after 30s", "cancelled after 1m". "(timeout)" goes | One way of writing a time in the editor. |
| (rows of an earlier statement show over a timeout) | A timeout opens Messages, as today, and the link is on the cancelled statement's line. Results, when you switch to it, shows those rows with no link | Results says nothing of a cancel today when it has rows to show. Say if the strip under them should offer it too. |
| (Messages for a cut result) | Keeps "Line 1: 1,000 rows (limit reached) · 14 ms", with no link | The prompt names the strip. One Show all, under the rows it shows. |
| macOS small print "…; exports get everything", Omarchy "exports get everything" | "Caps the rows of results you browse; Show all fetches the rest" and "caps results you browse · show all fetches the rest" | The spec: "The row limit's words gain 'Show all fetches the rest'". There is still no export. |
| (a narrow pane) | The words give way first (cut with "…"), the link stays whole; in the Results note the link goes under the words when one row cannot hold both | The link is what is done; the words are said in full to screen readers anyway. |

## Review of this plan

A reviewer read the draft against the code (2026-10-19). Its three points that would have failed the checks are in: the tests' `sql` helper comes with its first use (Task 3), the " · " before a link is painted without a label (an existing test reads every label of the Messages as a line), and a timeout opens Messages whether rows show or not (`opens_messages`), so the test and the table say so. Also in: `RunCaps::default()` is no caps rather than a limit of no rows, the Results note puts the link under however many rows its words wrap to, the Settings comment, `cargo fmt`, and the guide's sentence that is split across two lines.

## Files

| File | What it does after this plan |
|---|---|
| `src/model.rs` | `RunCaps`; `RunInFlight::caps` and `SqlRun::caps`; `SqlRun::nothing_written`; `SqlTab::caps`, `start_run_with`, `cut_statement`, `timed_out_run`; `Action::ShowAllRows` and `Action::RunWithoutTimeout`. |
| `src/app.rs` | `send_run` takes the caps; `show_all_rows` and `run_without_timeout`. |
| `src/keymap.rs` | `Command::ShowAllRows` (`a`) and `Command::RunWithoutTimeout` (`t`), under `When::RowsCut` and `When::TimedOut`, Omarchy only. |
| `src/ui/keys.rs` | Reads the two letters while their links are on screen. |
| `src/ui/sql_results.rs` | `Link`, `Links`, `link_keys`; the strip's words and Show all; `cancel_text` without "(timeout)"; Run without timeout in the Results note and on the Messages line. |
| `src/ui/settings/mod.rs`, `src/ui/settings/terminal.rs` | The Row limit row's small print and hint. |
| `src/testing.rs` | `cut_outcome`. |
| `src/ui/editor_results_tests.rs` | New: the headless tests of this step. |
| `src/ui/mod.rs` | Declares the tests. |
| `src/ui/editor_settings_tests.rs`, `src/ui/sql_results.rs` (tests) | Tests that met the old words. |
| `docs/_guide/sql-editor.md`, `docs/_guide/omarchy.md`, `docs/_reference/settings-and-files.md` | Show all, Run without timeout, their letters. |

Every task ends with the repository's checks passing:

    cargo fmt --all --check
    cargo clippy --locked --workspace --all-targets -- -D warnings
    cargo test --locked --workspace --all-targets
    RUSTDOCFLAGS='-D warnings' cargo doc --locked --workspace --no-deps

(`cargo` is `~/.cargo/bin/cargo` in these sessions; the mise shim fails.) Commit after each task, signed as usual. If signing says "agent refused operation", 1Password is locked: leave the change staged and say so.

---

### Task 1: A run remembers its caps, and says what each link would send

**Files:**
- Modify: `src/model.rs` (`RunInFlight`, `SqlRun`, `SqlTab`, near 3917 to 4440)
- Modify: `src/testing.rs` (after `rows_outcome`, near 1438)
- Test: `src/model.rs` (`mod tests`, near 4968)

- [ ] **Step 1: Add `cut_outcome` to the test helpers**

In `src/testing.rs`, after `rows_outcome`:

```rust
/// A SQL editor statement's result that the row limit cut at `rows`.
pub fn cut_outcome(rows: usize) -> tabletist_db::StatementOutcome {
    let page = page(rows, false);
    tabletist_db::StatementOutcome::Rows {
        columns: page.columns,
        rows: page.rows,
        truncated: true,
    }
}
```

- [ ] **Step 2: Write the failing tests**

In `src/model.rs`'s `mod tests`:

```rust
    /// An editor that ran `text` (split as SQLite does) read-only and got
    /// `outcomes`, stopped by `cancel`, sent with `caps`.
    fn ran(
        text: &str,
        outcomes: Vec<tabletist_db::StatementOutcome>,
        cancel: Option<crate::backend::CancelReason>,
        (mode, caps): (tabletist_db::ScriptMode, RunCaps),
    ) -> SqlTab {
        let mut sql = SqlTab::new(TabId(1), 1, 1_000, Some(Duration::from_secs(30)));
        sql.text = text.into();
        let statements =
            tabletist_db::sql::statements(tabletist_db::Driver::Sqlite.dialect(), text);
        let request = crate::backend::RequestId(9);
        let _ = sql.start_run_with(request, statements, mode, caps);
        let outcome = crate::testing::script_outcome(outcomes);
        assert!(sql.finish_run(request, Ok(outcome), cancel));
        sql
    }

    #[test]
    fn a_run_keeps_the_caps_it_was_sent_with() {
        let mut sql = SqlTab::new(TabId(1), 1, 0, None);
        assert_eq!(
            sql.caps(),
            RunCaps {
                limit: u32::MAX,
                timeout: None
            }
        );
        sql.limit = 500;
        sql.timeout = Some(Duration::from_secs(5));
        let caps = sql.caps();
        assert_eq!((caps.limit, caps.timeout), (500, Some(Duration::from_secs(5))));
        let other = RunCaps {
            limit: u32::MAX,
            timeout: Some(Duration::from_secs(5)),
        };
        let sql = ran(
            "SELECT 1",
            vec![crate::testing::rows_outcome(1)],
            None,
            (tabletist_db::ScriptMode::ReadOnly, other),
        );
        assert_eq!(sql.last_run().unwrap().caps, other);
    }

    #[test]
    fn show_all_is_the_shown_statement_of_a_read_cut_by_the_limit() {
        use crate::testing::{cut_outcome, done_outcome, rows_outcome};
        use tabletist_db::ScriptMode::{ReadOnly, Write};
        let caps = RunCaps::default();
        let text = "SELECT 1;\nSELECT 2;\nSELECT 3";
        // The Results show the last statement that returned rows.
        let outcomes = vec![rows_outcome(1), cut_outcome(1_000), done_outcome(None)];
        let mut sql = ran(text, outcomes.clone(), None, (ReadOnly, caps));
        let cut = sql.cut_statement().expect("the cut statement");
        assert_eq!(cut.first_line, 2);
        // Not once the text is another, and again when it is that text.
        sql.text.push(' ');
        assert!(sql.cut_statement().is_none());
        sql.text.pop();
        assert!(sql.cut_statement().is_some());
        // Not while a run is in flight.
        let _ = sql.start_run(crate::backend::RequestId(10), Vec::new(), ReadOnly);
        assert!(sql.cut_statement().is_none());
        // Not for a run sent to write, nor for a whole result.
        let sql = ran(text, outcomes, None, (Write, caps));
        assert!(sql.cut_statement().is_none());
        let sql = ran(text, vec![rows_outcome(3)], None, (ReadOnly, caps));
        assert!(sql.cut_statement().is_none());
    }

    #[test]
    fn run_without_timeout_is_the_last_run_where_the_timeout_stopped_it_and_nothing_was_written()
    {
        use crate::backend::CancelReason;
        use crate::testing::rows_outcome;
        use tabletist_db::StatementOutcome::Cancelled;
        let timeout = Some(CancelReason::Timeout(Duration::from_secs(30)));
        let read = (tabletist_db::ScriptMode::ReadOnly, RunCaps::default());
        let text = "SELECT 1;\nSELECT 2";
        let outcomes = vec![rows_outcome(1), Cancelled];
        let mut sql = ran(text, outcomes.clone(), timeout, read);
        assert!(sql.timed_out_run().is_some());
        sql.text.push(' ');
        assert!(sql.timed_out_run().is_none());
        // Cancelled by hand, or stopped for no reason of ours: no.
        for cancel in [Some(CancelReason::User), None] {
            let sql = ran(text, outcomes.clone(), cancel, read);
            assert!(sql.timed_out_run().is_none(), "{cancel:?}");
        }
    }

    #[test]
    fn nothing_is_written_by_a_read_or_by_a_write_rolled_back_whole() {
        use tabletist_db::{ScriptEnd, ScriptMode};
        let run = |mode, end, warning: Option<&str>| SqlRun {
            statements: Vec::new(),
            mode,
            outcome: tabletist_db::ScriptOutcome {
                end,
                rollback_warning: warning.map(str::to_owned),
                ..Default::default()
            },
            cancel: None,
            error_mark: None,
            caps: RunCaps::default(),
        };
        let partly = |committed| ScriptEnd::Partly { committed };
        assert!(run(ScriptMode::ReadOnly, ScriptEnd::RolledBack, None).nothing_written());
        assert!(run(ScriptMode::Write, ScriptEnd::RolledBack, None).nothing_written());
        assert!(run(ScriptMode::Write, partly(0), None).nothing_written());
        assert!(!run(ScriptMode::Write, partly(1), None).nothing_written());
        assert!(!run(ScriptMode::Write, ScriptEnd::Committed, None).nothing_written());
        let kept = Some("could not roll back");
        assert!(!run(ScriptMode::Write, ScriptEnd::RolledBack, kept).nothing_written());
    }
```

If the tests module does not import `Duration`, `TabId` or `SqlRun`, add them to its `use super::*;` line's neighbours.

- [ ] **Step 3: Run them to see them fail**

Run: `cargo test --locked --lib -- model::tests::a_run_keeps model::tests::show_all model::tests::run_without_timeout model::tests::nothing_is_written`
Expected: FAIL to compile: no `RunCaps`, `start_run_with`, `caps`, `cut_statement`, `timed_out_run`, `nothing_written`.

- [ ] **Step 4: Add `RunCaps`, and keep it with the run**

In `src/model.rs`, before `pub struct SqlRun`:

```rust
/// What a SQL editor run is sent with beside its statements: the most rows
/// a statement keeps (`u32::MAX` keeps every one, see
/// `Settings::fetch_cap`) and how long the run may go.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RunCaps {
    pub limit: u32,
    pub timeout: Option<Duration>,
}

/// Neither cap: every row, and no timer. Never a limit of no rows.
impl Default for RunCaps {
    fn default() -> Self {
        Self {
            limit: u32::MAX,
            timeout: None,
        }
    }
}
```

`SqlRun` gains a field after `error_mark`:

```rust
    /// What the run was sent with. Show all and Run without timeout each
    /// lift one of the two and keep the other.
    pub caps: RunCaps,
```

`SqlRun::new` takes it with the statements and the mode:

```rust
    fn new(
        (statements, mode, caps): (
            Vec<tabletist_db::sql::Statement>,
            tabletist_db::ScriptMode,
            RunCaps,
        ),
        outcome: tabletist_db::ScriptOutcome,
        cancel: Option<crate::backend::CancelReason>,
    ) -> Self {
```

and sets `caps` in `Self { … }`. After `new`, in the same `impl SqlRun`:

```rust
    /// Whether nothing of the run stayed written: a read-only run, or one
    /// sent to write whose transaction rolled back whole. Not one MySQL
    /// committed part of by itself, nor one the database could not roll
    /// back fully.
    pub fn nothing_written(&self) -> bool {
        use tabletist_db::ScriptEnd;
        match self.mode {
            tabletist_db::ScriptMode::ReadOnly => true,
            tabletist_db::ScriptMode::Write => {
                self.outcome.rollback_warning.is_none()
                    && matches!(
                        self.outcome.end,
                        ScriptEnd::RolledBack | ScriptEnd::Partly { committed: 0 }
                    )
            }
        }
    }
```

`RunInFlight` gains, after `mode`:

```rust
    /// What the run was sent with.
    pub caps: RunCaps,
```

- [ ] **Step 5: Start a run with caps, and say what each link sends**

In `impl SqlTab`, replace `start_run` with the two below, and add the three after `shown_rows`:

```rust
    /// What a run of this editor is sent with: its limit, as the fetch cap,
    /// and its timeout.
    pub fn caps(&self) -> RunCaps {
        RunCaps {
            limit: crate::settings::Settings::fetch_cap(self.limit),
            timeout: self.timeout,
        }
    }

    /// Starts a run of `statements` as `request`, sent in a transaction
    /// of `mode` with the editor's caps. Returns the run it replaces, if
    /// one was still pending, for the caller to cancel.
    #[must_use]
    pub fn start_run(
        &mut self,
        request: RequestId,
        statements: Vec<tabletist_db::sql::Statement>,
        mode: tabletist_db::ScriptMode,
    ) -> Option<RequestId> {
        let caps = self.caps();
        self.start_run_with(request, statements, mode, caps)
    }

    /// [`Self::start_run`], sent with `caps` in place of the editor's.
    #[must_use]
    pub fn start_run_with(
        &mut self,
        request: RequestId,
        statements: Vec<tabletist_db::sql::Statement>,
        mode: tabletist_db::ScriptMode,
        caps: RunCaps,
    ) -> Option<RequestId> {
        let superseded = self.run.pending;
        self.run.start(request);
        self.in_flight = Some(RunInFlight {
            statements,
            mode,
            caps,
            started: std::time::Instant::now(),
            text: TextPrint::of(&self.text),
        });
        superseded
    }
```

In `finish_run`, carry the caps:

```rust
        let ran = in_flight
            .map(|run| (run.statements, run.mode, run.caps))
            .unwrap_or_default();
```

After `shown_rows`:

```rust
    /// The statement Show all sends again: the one whose rows the Results
    /// show, where the limit cut them, in a run that only read. Only while
    /// the editor holds the text that ran and no run is in flight, as the
    /// card's Run again: what would be sent is then what the editor shows.
    pub fn cut_statement(&self) -> Option<&tabletist_db::sql::Statement> {
        let run = self.last_run().filter(|_| self.ran_this_text())?;
        if run.mode != tabletist_db::ScriptMode::ReadOnly {
            return None;
        }
        let (index, result) = self.shown()?;
        let cut = matches!(
            result.outcome,
            tabletist_db::StatementOutcome::Rows {
                truncated: true,
                ..
            }
        );
        if cut { run.statements.get(index) } else { None }
    }

    /// The run Run without timeout sends again: the last one, where the
    /// timeout stopped it and nothing of it was written. Only while the
    /// editor holds the text that ran and no run is in flight.
    pub fn timed_out_run(&self) -> Option<&SqlRun> {
        self.last_run()
            .filter(|_| self.ran_this_text())
            .filter(|run| matches!(run.cancel, Some(crate::backend::CancelReason::Timeout(_))))
            .filter(|run| run.nothing_written())
    }
```

- [ ] **Step 6: Run the tests to see them pass**

Run: `cargo test --locked --lib -- model::tests`
Expected: PASS. Then the four checks.

- [ ] **Step 7: Commit**

```bash
git add src/model.rs src/testing.rs
git commit -m "Keep what a SQL run was sent with, and say what Show all and Run without timeout would send"
```

---

### Task 2: Show all and Run without timeout send their runs

**Files:**
- Modify: `src/model.rs` (`Action`, after `RunSqlAgain`, near 728)
- Modify: `src/app.rs` (`apply` near 2017; `run_sql`, `run_sql_again`, `send_run` near 5982 to 6100)
- Create: `src/ui/editor_results_tests.rs`
- Modify: `src/ui/mod.rs` (declare it, after `editor_settings_tests`)

- [ ] **Step 1: Write the failing tests**

Create `src/ui/editor_results_tests.rs`:

```rust
//! Settings › Editor, step 2: what the SQL editor's results offer when the
//! row limit cut a result (Show all) or the timeout stopped a run (Run
//! without timeout), and what each sends.

use std::time::Duration;

use egui::{Key, Modifiers};
use tabletist_db::{Driver, ScriptEnd, ScriptMode, StatementOutcome};

use crate::backend::{CancelReason, Command};
use crate::model::{Action, ConnTabId, SqlTab, TabId};
use crate::settings::NewTab;
use crate::testing::{
    Harness, cut_outcome, done_outcome, rows_outcome, script_outcome, write_outcome,
};
use crate::theme::Look;

const TIMEOUT: CancelReason = CancelReason::Timeout(Duration::from_secs(30));

/// A connection that takes writes and speaks as `driver`, in `look`, with
/// a new SQL editor opened as the connection has it (read-write) that
/// holds `text` and has the keyboard.
fn editor(look: Look, driver: Driver, text: &str) -> (Harness, ConnTabId, TabId) {
    let mut harness = Harness::new();
    harness.set_look(look);
    harness.app.settings.new_tab = NewTab::Connection;
    let tab = harness.connect_fake_as(false);
    harness.app.workspace_mut(tab).unwrap().driver = driver;
    harness.press(Key::T, Modifiers::COMMAND);
    harness.frame(vec![egui::Event::Paste(text.into())]);
    harness.settle();
    let id = harness.app.workspace(tab).unwrap().active_tab.unwrap();
    (harness, tab, id)
}

fn sql_mut(harness: &mut Harness, tab: ConnTabId, id: TabId) -> &mut SqlTab {
    harness
        .app
        .workspace_mut(tab)
        .unwrap()
        .sql_tab_mut(id)
        .unwrap()
}

fn run(harness: &mut Harness) {
    harness.press(Key::Enter, Modifiers::COMMAND);
}

fn run_all(harness: &mut Harness) {
    harness.press(Key::Enter, Modifiers::COMMAND | Modifiers::SHIFT);
}

/// A run as it was sent.
#[derive(Debug, PartialEq)]
struct Sent {
    statements: Vec<String>,
    mode: ScriptMode,
    limit: u32,
    timeout: Option<Duration>,
}

fn last_run(harness: &Harness) -> Sent {
    let mut sent = harness.app.backend.sent.iter().rev();
    sent.find_map(|command| match command {
        Command::RunSql {
            statements,
            mode,
            limit,
            timeout,
            ..
        } => Some(Sent {
            statements: statements.iter().map(|s| s.text.clone()).collect(),
            mode: *mode,
            limit: *limit,
            timeout: *timeout,
        }),
        _ => None,
    })
    .expect("a run was sent")
}

fn runs_sent(harness: &Harness) -> usize {
    let sent = harness.app.backend.sent.iter();
    sent.filter(|command| matches!(command, Command::RunSql { .. }))
        .count()
}

fn show_all(harness: &mut Harness, tab: ConnTabId, id: TabId) {
    harness.app.apply(Action::ShowAllRows { tab, sql_tab: id });
}

fn without_timeout(harness: &mut Harness, tab: ConnTabId, id: TabId) {
    harness
        .app
        .apply(Action::RunWithoutTimeout { tab, sql_tab: id });
}

#[test]
fn show_all_runs_the_cut_statement_alone_with_no_limit_and_the_runs_timeout() {
    let text = "SELECT 1;\nSELECT * FROM users;\nSELECT 3";
    let (mut harness, tab, id) = editor(Look::macos(), Driver::Sqlite, text);
    run_all(&mut harness);
    let outcomes = vec![rows_outcome(1), cut_outcome(1_000), done_outcome(None)];
    harness.answer_sql(Ok(script_outcome(outcomes)), None);
    let before = runs_sent(&harness);
    show_all(&mut harness, tab, id);
    assert_eq!(runs_sent(&harness), before + 1);
    let sent = last_run(&harness);
    assert_eq!(sent.statements.len(), 1);
    assert!(sent.statements[0].contains("FROM users"), "{sent:?}");
    assert_eq!(
        (sent.mode, sent.limit, sent.timeout),
        (ScriptMode::ReadOnly, u32::MAX, Some(Duration::from_secs(30)))
    );
    // In flight, it is not sent twice; once whole, there is nothing to show.
    show_all(&mut harness, tab, id);
    assert_eq!(runs_sent(&harness), before + 1);
    harness.answer_sql(Ok(script_outcome(vec![rows_outcome(3)])), None);
    show_all(&mut harness, tab, id);
    assert_eq!(runs_sent(&harness), before + 1);
}

#[test]
fn show_all_sends_nothing_for_a_write_run_or_once_the_text_is_another() {
    // A run sent to write is never sent again.
    let text = "UPDATE users SET email = 'x' WHERE id = 1 RETURNING id";
    let (mut harness, tab, id) = editor(Look::macos(), Driver::Sqlite, text);
    run(&mut harness);
    assert_eq!(last_run(&harness).mode, ScriptMode::Write);
    let outcome = write_outcome(vec![cut_outcome(1_000)], ScriptEnd::Committed);
    harness.answer_sql(Ok(outcome), None);
    let before = runs_sent(&harness);
    show_all(&mut harness, tab, id);
    assert_eq!(runs_sent(&harness), before);
    // A read, once the text is another than the one that ran.
    let (mut harness, tab, id) = editor(Look::macos(), Driver::Sqlite, "SELECT * FROM users");
    run(&mut harness);
    harness.answer_sql(Ok(script_outcome(vec![cut_outcome(1_000)])), None);
    sql_mut(&mut harness, tab, id).text.push(' ');
    let before = runs_sent(&harness);
    show_all(&mut harness, tab, id);
    assert_eq!(runs_sent(&harness), before);
    sql_mut(&mut harness, tab, id).text.pop();
    show_all(&mut harness, tab, id);
    assert_eq!(runs_sent(&harness), before + 1);
}

#[test]
fn run_without_timeout_sends_the_stopped_run_again_with_none() {
    let (mut harness, tab, id) = editor(Look::macos(), Driver::Sqlite, "SELECT 1;\nSELECT 2");
    run_all(&mut harness);
    let stopped = vec![rows_outcome(1), StatementOutcome::Cancelled];
    harness.answer_sql(Ok(script_outcome(stopped.clone())), Some(TIMEOUT));
    let before = runs_sent(&harness);
    without_timeout(&mut harness, tab, id);
    let sent = last_run(&harness);
    assert_eq!(runs_sent(&harness), before + 1);
    assert_eq!(sent.statements.len(), 2);
    assert_eq!(
        (sent.mode, sent.limit, sent.timeout),
        (ScriptMode::ReadOnly, 1_000, None)
    );
    // Cancelled by hand: nothing to send.
    harness.answer_sql(Ok(script_outcome(stopped)), Some(CancelReason::User));
    without_timeout(&mut harness, tab, id);
    assert_eq!(runs_sent(&harness), before + 1);
}

#[test]
fn each_link_lifts_its_own_cap_and_keeps_the_runs_other_one() {
    let (mut harness, tab, id) = editor(Look::macos(), Driver::Sqlite, "SELECT * FROM users");
    run(&mut harness);
    harness.answer_sql(Ok(script_outcome(vec![cut_outcome(1_000)])), None);
    show_all(&mut harness, tab, id);
    // Show all ran into the timeout: run again, it has neither cap.
    let stopped = script_outcome(vec![StatementOutcome::Cancelled]);
    harness.answer_sql(Ok(stopped), Some(TIMEOUT));
    without_timeout(&mut harness, tab, id);
    let sent = last_run(&harness);
    assert_eq!((sent.limit, sent.timeout), (u32::MAX, None));
    // A run of the editor has its caps again.
    harness.answer_sql(Ok(script_outcome(vec![rows_outcome(3)])), None);
    run(&mut harness);
    let sent = last_run(&harness);
    assert_eq!((sent.limit, sent.timeout), (1_000, Some(Duration::from_secs(30))));
}

#[test]
fn run_without_timeout_sends_a_write_again_only_where_nothing_was_written() {
    let text = "CREATE TABLE t (id int);\nUPDATE users SET email = 'x' WHERE id = 1";
    let stopped = vec![done_outcome(None), StatementOutcome::Cancelled];
    // MySQL committed the CREATE by itself: nothing is sent again.
    let (mut harness, tab, id) = editor(Look::macos(), Driver::MySql, text);
    run_all(&mut harness);
    assert_eq!(last_run(&harness).mode, ScriptMode::Write);
    let partly = ScriptEnd::Partly { committed: 1 };
    harness.answer_sql(Ok(write_outcome(stopped.clone(), partly)), Some(TIMEOUT));
    let before = runs_sent(&harness);
    without_timeout(&mut harness, tab, id);
    assert_eq!(runs_sent(&harness), before);
    // Rolled back whole: sent again, to write, as the tab's mode has it.
    let (mut harness, tab, id) = editor(Look::macos(), Driver::Postgres, text);
    run_all(&mut harness);
    let outcome = write_outcome(stopped, ScriptEnd::RolledBack);
    harness.answer_sql(Ok(outcome), Some(TIMEOUT));
    without_timeout(&mut harness, tab, id);
    let sent = last_run(&harness);
    assert_eq!((sent.mode, sent.timeout), (ScriptMode::Write, None));
}
```

In `src/ui/mod.rs`, after `mod editor_settings_tests;`:

```rust
#[cfg(test)]
mod editor_results_tests;
```

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test --locked --lib -- ui::editor_results_tests`
Expected: FAIL to compile: no `Action::ShowAllRows`, `Action::RunWithoutTimeout`.

- [ ] **Step 3: The two actions**

In `src/model.rs`, after `RunSqlAgain`:

```rust
    /// The results' Show all: run the statement whose rows the limit cut
    /// again, alone, with no limit (see `SqlTab::cut_statement`).
    ShowAllRows {
        tab: ConnTabId,
        sql_tab: TabId,
    },
    /// Run without timeout: run the last run, which the timeout stopped,
    /// again with no timeout (see `SqlTab::timed_out_run`).
    RunWithoutTimeout {
        tab: ConnTabId,
        sql_tab: TabId,
    },
```

In `src/app.rs`'s `apply`, after the `RunSqlAgain` arm:

```rust
            Action::ShowAllRows { tab, sql_tab } => self.show_all_rows(tab, sql_tab),
            Action::RunWithoutTimeout { tab, sql_tab } => self.run_without_timeout(tab, sql_tab),
```

- [ ] **Step 4: `send_run` takes the caps**

In `src/app.rs`, `send_run` gains `caps: crate::model::RunCaps` after `mode`, and sends them:

```rust
    /// Sends `statements` to the editor's session as its run, in a
    /// transaction of `mode`, with `caps`.
    fn send_run(
        &mut self,
        tab: ConnTabId,
        id: TabId,
        statements: Vec<tabletist_db::sql::Statement>,
        mode: tabletist_db::ScriptMode,
        caps: crate::model::RunCaps,
    ) {
```

and in its body replace

```rust
        // No limit keeps as many rows as there are.
        let (limit, timeout) = (Settings::fetch_cap(sql.limit), sql.timeout);
        let superseded = sql.start_run(request, statements.clone(), mode);
```

with

```rust
        let superseded = sql.start_run_with(request, statements.clone(), mode, caps);
```

and the command's fields with `limit: caps.limit, timeout: caps.timeout,`.

`run_sql` ends with:

```rust
        let mode = script_mode(workspace.run_mode(sql), dialect, &statements);
        let caps = sql.caps();
        self.send_run(tab, id, statements, mode, caps);
```

`run_sql_again` ends with:

```rust
        let caps = sql.caps();
        self.send_run(tab, id, statements, tabletist_db::ScriptMode::Write, caps);
```

- [ ] **Step 5: The two runs**

In `src/app.rs`, after `run_sql_again`:

```rust
    /// The results' Show all: the statement whose rows the limit cut, sent
    /// again alone and read-only, with no limit and the timeout its run
    /// had. Only what `SqlTab::cut_statement` offers: never a run that was
    /// sent to write, nor a statement of a text the editor no longer holds.
    fn show_all_rows(&mut self, tab: ConnTabId, id: TabId) {
        let Some(workspace) = self.workspace(tab) else {
            return;
        };
        if !workspace.takes_commands() {
            return;
        }
        let Some(sql) = workspace.sql_tab(id) else {
            return;
        };
        let (Some(statement), Some(run)) = (sql.cut_statement(), sql.last_run()) else {
            return;
        };
        let caps = crate::model::RunCaps {
            limit: u32::MAX,
            ..run.caps
        };
        let statements = vec![statement.clone()];
        let mode = tabletist_db::ScriptMode::ReadOnly;
        self.send_run(tab, id, statements, mode, caps);
    }

    /// Run without timeout: the last run, which the timeout stopped and
    /// which wrote nothing, sent again with no timeout and the row limit
    /// it had. Its transaction is the one Run would give it now: a tab
    /// switched to Read-only since sends it read-only.
    fn run_without_timeout(&mut self, tab: ConnTabId, id: TabId) {
        let Some(workspace) = self.workspace(tab) else {
            return;
        };
        if !workspace.takes_commands() {
            return;
        }
        let Some(sql) = workspace.sql_tab(id) else {
            return;
        };
        let Some(run) = sql.timed_out_run() else {
            return;
        };
        let statements = run.statements.clone();
        if statements.is_empty() {
            return;
        }
        let dialect = workspace.driver.dialect();
        let mode = script_mode(workspace.run_mode(sql), dialect, &statements);
        let caps = crate::model::RunCaps {
            timeout: None,
            ..run.caps
        };
        self.send_run(tab, id, statements, mode, caps);
    }
```

- [ ] **Step 6: Run the tests to see them pass**

Run: `cargo test --locked --lib -- ui::editor_results_tests`
Expected: PASS. Then the four checks (an unused `Settings` import in `app.rs`, if `fetch_cap` was its last use there, goes).

- [ ] **Step 7: Commit**

```bash
git add src/model.rs src/app.rs src/ui/editor_results_tests.rs src/ui/mod.rs
git commit -m "Run a cut statement again with no limit, or a run the timeout stopped with no timeout"
```

---

### Task 3: The terminal look's letters

**Files:**
- Modify: `src/keymap.rs` (`When` near 139, `Command` near 482, `info` near 804, `BINDINGS` near 1699)
- Modify: `src/ui/sql_results.rs` (after `card_key`, near 1467)
- Modify: `src/ui/keys.rs` (near 2200 and 2670)
- Test: `src/ui/editor_results_tests.rs`

- [ ] **Step 1: Write the failing test**

Add `ResultPane` to the file's `crate::model` import, and append:

```rust
fn sql(harness: &Harness, tab: ConnTabId, id: TabId) -> &SqlTab {
    harness.app.workspace(tab).unwrap().sql_tab(id).unwrap()
}

#[test]
fn the_terminal_looks_letters_take_the_links_on_screen() {
    let (mut harness, tab, id) = editor(Look::omarchy(), Driver::Sqlite, "SELECT * FROM users");
    run(&mut harness);
    harness.answer_sql(Ok(script_outcome(vec![cut_outcome(1_000)])), None);
    assert_eq!(sql(&harness, tab, id).pane, ResultPane::Results);
    let before = runs_sent(&harness);
    // In the editor's text a letter is typed; out of it, `t` offers
    // nothing here and `a` shows all.
    harness.press(Key::Escape, Modifiers::NONE);
    harness.press(Key::T, Modifiers::NONE);
    assert_eq!(runs_sent(&harness), before);
    harness.press(Key::A, Modifiers::NONE);
    assert_eq!(runs_sent(&harness), before + 1);
    assert_eq!(last_run(&harness).limit, u32::MAX);
    // Stopped by the timeout, with no rows: Messages, and `t`.
    let stopped = script_outcome(vec![StatementOutcome::Cancelled]);
    harness.answer_sql(Ok(stopped), Some(TIMEOUT));
    assert_eq!(sql(&harness, tab, id).pane, ResultPane::Messages);
    harness.press(Key::T, Modifiers::NONE);
    assert_eq!(runs_sent(&harness), before + 2);
    assert_eq!(last_run(&harness).timeout, None);
}

#[test]
fn the_other_looks_have_no_letters_for_the_links() {
    let (mut harness, _tab, _id) = editor(Look::macos(), Driver::Sqlite, "SELECT * FROM users");
    run(&mut harness);
    harness.answer_sql(Ok(script_outcome(vec![cut_outcome(1_000)])), None);
    let before = runs_sent(&harness);
    harness.press(Key::Escape, Modifiers::NONE);
    harness.press(Key::A, Modifiers::NONE);
    assert_eq!(runs_sent(&harness), before);
}
```

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test --locked --lib -- ui::editor_results_tests::the_terminal ui::editor_results_tests::the_other`
Expected: the terminal test FAILS at `before + 1` (no letter takes Show all); the other passes.

- [ ] **Step 3: The commands and their letters**

In `src/keymap.rs`'s `When`, after `RefusedWrite`:

```rust
    /// The results offer Show all: a cut result shows.
    RowsCut,
    /// The results offer Run without timeout.
    TimedOut,
```

In the `Command` list, after `AllowRefusedWrite`:

```rust
    ShowAllRows,
    RunWithoutTimeout,
```

In `info`, after `AllowRefusedWrite`'s line:

```rust
            C::ShowAllRows => info("Show all rows", "", Prompts),
            C::RunWithoutTimeout => info("Run without timeout", "", Prompts),
```

In `BINDINGS`, after `AllowRefusedWrite`'s line:

```rust
        bind(C::ShowAllRows, SQL_EDITOR, NONE, keys(&[k("a")])).when(When::RowsCut),
        bind(C::RunWithoutTimeout, SQL_EDITOR, NONE, keys(&[k("t")])).when(When::TimedOut),
```

- [ ] **Step 4: Which links are on screen**

In `src/ui/sql_results.rs`, after `card_key`:

```rust
/// A link the results offer after our words.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Link {
    /// Under a cut result: its statement again, with no limit.
    ShowAll,
    /// Beside the words of a run the timeout stopped: it again, with none.
    WithoutTimeout,
}

impl Link {
    /// The link's name, and the command whose letter takes it in the
    /// terminal look.
    fn names(self) -> (&'static str, crate::keymap::Command) {
        use crate::keymap::Command;
        match self {
            Self::ShowAll => ("Show all", Command::ShowAllRows),
            Self::WithoutTimeout => ("Run without timeout", Command::RunWithoutTimeout),
        }
    }

    fn action(self, tab: ConnTabId, sql_tab: TabId) -> Action {
        match self {
            Self::ShowAll => Action::ShowAllRows { tab, sql_tab },
            Self::WithoutTimeout => Action::RunWithoutTimeout { tab, sql_tab },
        }
    }
}

/// The links the editor's results offer now. `App` sends only what
/// `SqlTab::cut_statement` and `SqlTab::timed_out_run` allow, so a link is
/// drawn only where pressing it sends something.
#[derive(Clone, Copy, Default)]
struct Links {
    show_all: bool,
    without_timeout: bool,
}

impl Links {
    fn of(workspace: &Workspace, sql: &SqlTab) -> Self {
        let takes = workspace.takes_commands();
        Self {
            show_all: takes && sql.cut_statement().is_some(),
            without_timeout: takes && sql.timed_out_run().is_some(),
        }
    }

    /// The links on screen: Show all on the strip under the rows (Results);
    /// Run without timeout on the line of the cancel (Messages), and in
    /// Results where no rows show and the note says the cancel.
    fn on_screen(self, sql: &SqlTab) -> impl Iterator<Item = Link> {
        let results = sql.pane == ResultPane::Results;
        let rows = sql.shown_rows().is_some();
        [
            (self.show_all && results, Link::ShowAll),
            (self.without_timeout && (!results || !rows), Link::WithoutTimeout),
        ]
        .into_iter()
        .filter_map(|(shown, link)| shown.then_some(link))
    }
}

/// The links on screen in the SQL editor in front, with the command whose
/// letter takes each and what it does. Only in the terminal look, whose
/// links name their letters, as [`card_key`].
pub(crate) fn link_keys(app: &App, tab: ConnTabId) -> Vec<(crate::keymap::Command, Action)> {
    if !app.look.terminal {
        return Vec::new();
    }
    let Some(workspace) = app.workspace(tab) else {
        return Vec::new();
    };
    let Some(sql) = workspace.active_sql_tab() else {
        return Vec::new();
    };
    if !workspace.opened() || sql.run.error.is_some() {
        return Vec::new();
    }
    Links::of(workspace, sql)
        .on_screen(sql)
        .map(|link| (link.names().1, link.action(tab, sql.id)))
        .collect()
}
```

- [ ] **Step 5: Read the letters**

In `src/ui/keys.rs`, after `let has_card = card.is_some();`:

```rust
    // The results' links on screen: the commands their letters stand for.
    let mut links = crate::ui::sql_results::link_keys(app, tab);
    let offers = |command: Command| links.iter().any(|(offered, _)| *offered == command);
    let (cut, timed_out) = (
        offers(Command::ShowAllRows),
        offers(Command::RunWithoutTimeout),
    );
```

In `holds`, after `RefusedWrite`:

```rust
        crate::keymap::When::RowsCut => cut,
        crate::keymap::When::TimedOut => timed_out,
```

After the card's arm in the command `match`:

```rust
            // A link's letter, while its link is on screen. A press of its
            // own, as the card's.
            (Some(command @ (Command::ShowAllRows | Command::RunWithoutTimeout)), _) => {
                if fresh_key
                    && let Some(at) = links.iter().position(|(offered, _)| *offered == command)
                {
                    actions.push(links.swap_remove(at).1);
                }
            }
```

If the borrow checker holds `links` borrowed by `offers` past this point, end `offers`' use by computing `cut` and `timed_out` in a block as above (the closure is dropped after the tuple).

- [ ] **Step 6: Run the tests to see them pass**

Run: `cargo test --locked --lib -- ui::editor_results_tests keymap`
Expected: PASS, the keymap's generated tests too (`no_two_commands_share_a_key_where_both_could_be_read`, `every_command_has_its_keys_or_none_on_purpose`). Then `cargo test --locked --test keys` and the four checks.

- [ ] **Step 7: Commit**

```bash
git add src/keymap.rs src/ui/keys.rs src/ui/sql_results.rs src/ui/editor_results_tests.rs
git commit -m "Take Show all and Run without timeout with a and t in the Omarchy look"
```

---

### Task 4: "Limited to 1,000 rows · Show all"

**Files:**
- Modify: `src/ui/sql_results.rs` (`Place` near 184, `draw` near 217, `results` near 1633, tests)
- Test: `src/ui/sql_results.rs` (`mod tests`)

- [ ] **Step 1: Write the failing tests**

In `src/ui/sql_results.rs`'s tests, add `cut_outcome` to the `crate::testing` import, and replace `a_truncated_result_says_the_limit_was_reached` with:

```rust
    #[test]
    fn a_cut_result_says_its_limit_and_offers_show_all() {
        for look in Look::ALL {
            let (mut harness, tab) = editor(look, "SELECT 1");
            run(&mut harness);
            harness.answer_sql(Ok(script_outcome(vec![cut_outcome(1_000)])), None);
            let note = "Limited to 1,000 rows";
            assert!(harness.has(note), "{}", look.name);
            let reads = look.label(note);
            assert!(painted(&harness, &reads), "{reads}: {:?}", harness.painted);
            let tree = harness.settle();
            assert!(node(&tree, "Show all", Role::Button).is_some(), "{}", look.name);
            // The rows there are, not the limit the editor has by now.
            let sql_tab = sql(&harness, tab).id;
            harness.app.apply(Action::SetSqlLimit {
                tab,
                sql_tab,
                limit: 100,
            });
            assert!(harness.has(note), "{}", look.name);
            harness.click("Messages");
            assert!(harness.has("Line 1: 1,000 rows (limit reached) · 14 ms"));
            harness.click("Results");
            harness.click("Show all");
            let limit = match crate::testing::last_sent(&harness.app) {
                Command::RunSql { limit, .. } => *limit,
                other => panic!("{other:?}"),
            };
            assert_eq!(limit, u32::MAX, "{}", look.name);
            // In flight, the strip has no link: a second press sends nothing.
            let tree = harness.settle();
            assert!(node(&tree, "Show all", Role::Button).is_none());
            // A whole result has no strip.
            harness.answer_sql(Ok(script_outcome(vec![rows_outcome(3)])), None);
            assert!(harness.has("Row 3") && !harness.has(note));
        }
    }

    #[test]
    fn a_cut_result_of_a_run_sent_to_write_offers_no_show_all() {
        for look in Look::ALL {
            let text = "UPDATE users SET email = 'x' WHERE id = 1 RETURNING id";
            let (mut harness, _tab) = writing(look, Driver::Sqlite, text);
            let outcome = write_outcome(vec![cut_outcome(1_000)], ScriptEnd::Committed);
            harness.answer_sql(Ok(outcome), None);
            harness.click("Results");
            assert!(harness.has("Limited to 1,000 rows"), "{}", look.name);
            let tree = harness.settle();
            assert!(node(&tree, "Show all", Role::Button).is_none(), "{}", look.name);
        }
    }

    #[test]
    fn show_all_stays_whole_in_the_narrowest_pane() {
        for look in Look::ALL {
            let (mut harness, _tab) = editor(look, "SELECT 1");
            narrow_to(&mut harness, 240.0);
            run(&mut harness);
            harness.answer_sql(Ok(script_outcome(vec![cut_outcome(1_000)])), None);
            let tree = harness.settle();
            let band = bounds(&tree, "Resize the editor", Role::Unknown).unwrap();
            let link = bounds(&tree, "Show all", Role::Button)
                .unwrap_or_else(|| panic!("Show all in {}", look.name));
            assert!(
                link.left() >= band.left() && link.right() <= band.right() + 0.5,
                "{link:?} in {band:?} in {}",
                look.name
            );
            // The words give way, and are named in full.
            assert!(harness.has("Limited to 1,000 rows"), "{}", look.name);
        }
    }
```

In `the_header_keeps_its_buttons_apart_in_the_narrowest_pane` and `the_messages_keep_their_lines_apart_in_the_narrowest_pane`, the `StatementOutcome::Rows { … truncated: true }` built by hand stays as it is.

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test --locked --lib -- ui::sql_results::tests::a_cut_result ui::sql_results::tests::show_all_stays`
Expected: FAIL: "Limited to 1,000 rows" is not on screen (the strip says "First 1,000 rows (limit reached)").

- [ ] **Step 3: The links reach the panes**

`Place` gains, after `blocked`:

```rust
    /// The links the results offer now.
    links: Links,
```

and `draw` sets `links: Links::of(workspace, sql),`.

- [ ] **Step 4: A link's button**

After `impl Links` in `src/ui/sql_results.rs`:

```rust
/// How tall a link is: inside the strip's 28 and a line of the Messages.
const LINK_HEIGHT: f32 = 22.0;

/// A link laid out in the look: its name, its words as painted and the
/// key that takes it in the terminal look.
struct LinkLaid {
    link: Link,
    name: String,
    painted: String,
    keys: crate::keymap::Written,
}

impl LinkLaid {
    fn new(ctx: &egui::Context, link: Link, env: &Env<'_>) -> Self {
        let (name, command) = link.names();
        let name = gettext(env.locale, name).into_owned();
        Self {
            link,
            painted: env.look.label(&name),
            keys: crate::ui::keys::written(ctx, env.look, command),
            name,
        }
    }

    /// A link in the accent colour; in the terminal look a key hint that
    /// names the letter that takes it, as the card's buttons do.
    fn spec(&self, look: &Look) -> ButtonSpec<'_> {
        let spec = if look.terminal {
            states::key_button(&self.painted, &self.keys, look)
        } else {
            ButtonSpec::new(&self.painted)
                .link()
                .role(widgets::secondary(look))
                .padding(6.0)
        };
        spec.label(&self.name)
    }

    fn width(&self, ui: &Ui, look: &Look) -> f32 {
        self.spec(look).width(ui, look)
    }

    /// Draws the link from `left`, centred on `center`, and says what it
    /// does when it is pressed.
    fn show(
        &self,
        ui: &mut Ui,
        (left, center): (f32, f32),
        place: &Place<'_>,
        env: &Env<'_>,
        actions: &mut Vec<Action>,
    ) {
        let width = self.width(ui, env.look);
        let rect = Rect::from_min_size(
            pos2(left, center - LINK_HEIGHT / 2.0),
            vec2(width, LINK_HEIGHT),
        );
        if self.spec(env.look).show_at(ui, rect, env.look, env.palette).clicked() {
            actions.push(self.link.action(place.tab, place.sql.id));
        }
    }
}

/// What stands between our words and a link: " · ". Painted with
/// `widgets::paint_text`, never `paint_label`: a screen reader has the
/// words and the button, and the Messages' tests read every label as a
/// line.
const BEFORE_LINK: &str = " · ";
```

- [ ] **Step 5: The strip**

In `results`, replace the strip's text (from `// The rows there are, not the limit the editor has by now.` to the `paint_named` call) with:

```rust
        // The rows there are, not the limit the editor has by now.
        let said = env.said(|words| {
            format!(
                "{} {}",
                words.say("Limited to"),
                rows_text(rows.len() as u64, words)
            )
        });
        let role = widgets::secondary(look);
        let center = strip.top() + 1.0 + (strip.height() - 1.0) / 2.0;
        let (left, right) = (strip.left() + side(look), strip.right() - side(look));
        let link = place
            .links
            .show_all
            .then(|| LinkLaid::new(ui.ctx(), Link::ShowAll, env));
        let width = |text: &str| role.width(ui.ctx(), look.faces, text);
        let tail = link
            .as_ref()
            .map_or(0.0, |link| width(BEFORE_LINK) + link.width(ui, look));
        // The words give way to the link.
        let cut = grid::ellipsize(&said.painted, (right - left - tail).max(0.0), false, width);
        let text = Text::one(look, role, &cut, palette.secondary);
        let mut x = left + data_view::paint_named(ui, left, center, text, &said.name);
        if let Some(link) = link {
            let dot = Text::one(look, role, BEFORE_LINK, palette.dim);
            x += widgets::paint_text(ui, x, center, dot);
            link.show(ui, (x, center), place, env, actions);
        }
```

`results` already takes `actions`. If `grid::ellipsize`'s measure closure cannot take `width` by value twice, pass `|text| role.width(ui.ctx(), look.faces, text)` there instead.

- [ ] **Step 6: Run the tests to see them pass**

Run: `cargo test --locked --lib -- ui::sql_results ui::editor_results_tests`
Expected: PASS. Then the four checks.

- [ ] **Step 7: Commit**

```bash
git add src/ui/sql_results.rs
git commit -m "Say a cut result is limited, and offer Show all under it"
```

---

### Task 5: "Cancelled after 30 s · Run without timeout"

**Files:**
- Modify: `src/ui/sql_results.rs` (`cancel_text` near 708, `note` near 658, `messages` near 1150, `results` near 1603, tests)
- Test: `src/ui/sql_results.rs` (`mod tests`), `src/ui/editor_results_tests.rs`

- [ ] **Step 1: Write the failing tests**

In `src/ui/sql_results.rs`'s tests, replace `a_timeout_reads_as_cancelled_after_its_limit` with:

```rust
    #[test]
    fn a_timeout_says_when_it_stopped_and_offers_to_run_without_one() {
        for look in Look::ALL {
            let (mut harness, tab) = editor(look, "SELECT 1");
            run(&mut harness);
            let timeout = Some(CancelReason::Timeout(Duration::from_secs(30)));
            harness.answer_sql(
                Ok(script_outcome(vec![StatementOutcome::Cancelled])),
                timeout,
            );
            assert_eq!(sql(&harness, tab).pane, ResultPane::Messages);
            assert!(harness.has("Line 1: Cancelled after 30 s"), "{}", look.name);
            let tree = harness.settle();
            let link = "Run without timeout";
            assert!(node(&tree, link, Role::Button).is_some(), "{}", look.name);
            harness.click("Results");
            assert!(harness.has("Cancelled after 30 s"), "{}", look.name);
            assert!(!harness.has("Statement ran · no rows returned"));
            let reads = if look.terminal {
                "cancelled after 30s"
            } else {
                "Cancelled after 30 s"
            };
            assert!(painted(&harness, reads), "{reads}: {:?}", harness.painted);
            let tree = harness.settle();
            assert!(node(&tree, link, Role::Button).is_some(), "{}", look.name);
            harness.click(link);
            match crate::testing::last_sent(&harness.app) {
                Command::RunSql { timeout, .. } => assert_eq!(*timeout, None),
                other => panic!("{other:?}"),
            }
        }
    }

    #[test]
    fn a_timeout_of_whole_minutes_says_them_as_the_menu_does() {
        let (mut harness, _tab) = editor(Look::macos(), "SELECT 1");
        run(&mut harness);
        let minute = Some(CancelReason::Timeout(Duration::from_secs(60)));
        harness.answer_sql(
            Ok(script_outcome(vec![StatementOutcome::Cancelled])),
            minute,
        );
        assert!(harness.has("Line 1: Cancelled after 1 min"));
    }

    #[test]
    fn a_cancel_by_hand_offers_no_run_without_timeout() {
        for look in Look::ALL {
            let (mut harness, _tab) = editor(look, "SELECT 1");
            run(&mut harness);
            harness.answer_sql(
                Ok(script_outcome(vec![StatementOutcome::Cancelled])),
                Some(CancelReason::User),
            );
            let tree = harness.settle();
            assert!(node(&tree, "Run without timeout", Role::Button).is_none());
            harness.click("Results");
            let tree = harness.settle();
            assert!(node(&tree, "Run without timeout", Role::Button).is_none());
        }
    }
```

Change `"Cancelled after 30 s (timeout)"` to `"Cancelled after 30 s"` in `a_run_stopped_before_it_began_reads_as_cancelled`, `the_statements_after_a_cancelled_one_did_not_run` and `a_cancelled_or_timed_out_run_says_it_was_rolled_back`, and update `cancel_text`'s doc comment.

In `src/ui/editor_results_tests.rs`, add `use egui::accesskit::Role;` and `node` to the `crate::testing` import, and append:

```rust
#[test]
fn the_link_stands_on_the_line_of_the_cancel_only() {
    for look in [Look::macos(), Look::omarchy()] {
        let text = "SELECT 1;\nSELECT 2;\nSELECT 3";
        let (mut harness, tab, id) = editor(look, Driver::Sqlite, text);
        run_all(&mut harness);
        let outcomes = vec![rows_outcome(2), StatementOutcome::Cancelled];
        harness.answer_sql(Ok(script_outcome(outcomes)), Some(TIMEOUT));
        // A timeout opens Messages, rows or not. Results shows the rows,
        // and says nothing of the cancel.
        assert_eq!(sql(&harness, tab, id).pane, ResultPane::Messages);
        harness.click("Results");
        let tree = harness.settle();
        assert!(node(&tree, "Run without timeout", Role::Button).is_none());
        harness.click("Messages");
        let tree = harness.settle();
        let line = crate::testing::bounds(&tree, "Line 2: Cancelled after 30 s", Role::Label)
            .unwrap_or_else(|| panic!("the line in {}", look.name));
        let link = crate::testing::bounds(&tree, "Run without timeout", Role::Button)
            .unwrap_or_else(|| panic!("the link in {}", look.name));
        assert!(
            link.center().y >= line.top() && link.center().y <= line.bottom(),
            "{link:?} beside {line:?} in {}",
            look.name
        );
    }
}
```

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test --locked --lib -- ui::sql_results::tests::a_timeout ui::sql_results::tests::a_cancel_by_hand ui::editor_results_tests::the_link_stands`
Expected: FAIL: the words still end with "(timeout)", and no "Run without timeout" button.

- [ ] **Step 3: The cancel's words**

Replace `cancel_text`:

```rust
/// "Cancelled", or "Cancelled after 30 s" when the timeout stopped the
/// run, its time written as the Timeout menu writes it ("1 min", the
/// terminal's "30s"). A statement cancelled for no reason of ours (the
/// server's own `statement_timeout`) is cancelled all the same.
fn cancel_text(cancel: Option<CancelReason>, words: Words) -> String {
    match cancel {
        Some(CancelReason::Timeout(after)) => {
            let secs = u32::try_from(after.as_secs()).unwrap_or(u32::MAX);
            format!(
                "{} {}",
                words.say("Cancelled after"),
                super::sql_editor::time_words(secs, words.lower, words.locale)
            )
        }
        Some(CancelReason::User) | None => words.say("Cancelled"),
    }
}
```

- [ ] **Step 4: The Results note**

After `note_at`:

```rust
/// [`note`] with `link` after the words, " · " between them. Where one
/// row cannot hold both, the link goes under the words.
fn note_with_link(
    ui: &mut Ui,
    rect: Rect,
    said: &Said,
    link: Option<Link>,
    place: &Place<'_>,
    env: &Env<'_>,
    actions: &mut Vec<Action>,
) {
    let (look, palette) = (env.look, env.palette);
    let Some(link) = link.map(|link| LinkLaid::new(ui.ctx(), link, env)) else {
        note(ui, rect, said, palette.warning, env);
        return;
    };
    let role = widgets::body(look);
    let width = |text: &str| role.width(ui.ctx(), look.faces, text);
    let (words, dot, link_width) = (
        width(&said.painted),
        width(BEFORE_LINK),
        link.width(ui, look),
    );
    let room = (rect.width() - 2.0 * 24.0).max(0.0);
    let row = role.row_height(ui.ctx(), look.faces).max(LINK_HEIGHT);
    let center = rect.center().y.max(rect.top() + 8.0 + row / 2.0);
    let text = Text::one(look, role, &said.painted, palette.warning);
    if words + dot + link_width <= room {
        let left = rect.center().x - (words + dot + link_width) / 2.0;
        let mut x = left + data_view::paint_named(ui, left, center, text, &said.name);
        x += widgets::paint_text(ui, x, center, Text::one(look, role, BEFORE_LINK, palette.dim));
        link.show(ui, (x, center), place, env, actions);
    } else {
        // The words as `note` lays them, wrapped to the room, and the
        // link under however many rows they take.
        let laid = LaidMessage::new(
            ui.ctx(),
            ("", palette.warning),
            (&said.painted, palette.warning),
            (role, room, true),
            look,
        );
        let height = laid.size().y + 4.0 + LINK_HEIGHT;
        let top = (rect.center().y - height / 2.0).max(rect.top() + 8.0);
        let at = pos2(rect.center().x - laid.size().x / 2.0, top);
        laid.paint(ui.painter(), at);
        widgets::announce(ui, Rect::from_min_size(at, laid.size()), &said.name);
        let left = rect.center().x - link_width / 2.0;
        let center = top + laid.size().y + 4.0 + LINK_HEIGHT / 2.0;
        link.show(ui, (left, center), place, env, actions);
    }
}
```

In `results`, the stopped branch becomes:

```rust
        if run.outcome.was_cancelled() && !written {
            let said = env.said(|words| cancel_text(run.cancel, words));
            let link = place.links.without_timeout.then_some(Link::WithoutTimeout);
            note_with_link(ui, rect, &said, link, place, env, actions);
        } else if let Some(index) = failed {
```

- [ ] **Step 5: The Messages line**

After `message`:

```rust
/// Whether `line` says the cancel's words: the line of the statement the
/// run was stopped in or before, or the stop's own line.
fn says_cancel(line: Line<'_>, run: Option<&SqlRun>) -> bool {
    let Some(run) = run else {
        return false;
    };
    match line {
        Line::Stopped => true,
        Line::Statement(index) => match run.outcome.results.get(index) {
            Some(result) => result.outcome == StatementOutcome::Cancelled,
            None => stopped_at(run) == Some(index),
        },
        _ => false,
    }
}
```

In `messages`, before `let lay = …`, lay the link out once and keep room for it on its line:

```rust
    let link = place
        .links
        .without_timeout
        .then(|| LinkLaid::new(ui.ctx(), Link::WithoutTimeout, env));
    let link_room = link.as_ref().map_or(0.0, |link| {
        role.width(ui.ctx(), look.faces, BEFORE_LINK) + link.width(ui, look)
    });
    // The words of the line with the link give way to it.
    let tail = |line: Line<'_>| {
        if says_cancel(line, run) { link_room } else { 0.0 }
    };
```

`lay`'s room becomes `(room - indent(line) - tail(line)).max(0.0)`. In the row closure, after `lay(line).paint(ui.painter(), at);`:

```rust
                    if let Some(link) = link.as_ref().filter(|_| says_cancel(line, run)) {
                        let laid = lay(line);
                        let center = at.y + laid.size().y / 2.0;
                        let x = at.x + laid.size().x;
                        let dot = Text::one(look, role, BEFORE_LINK, palette.dim);
                        let x = x + widgets::paint_text(ui, x, center, dot);
                        link.show(ui, (x, center), place, env, actions);
                    }
```

(Lay the line once into a local and paint and measure that, rather than calling `lay` twice.) `messages` already takes `actions`; the row closure borrows it mutably, and `lay` only reads.

- [ ] **Step 6: Run the tests to see them pass**

Run: `cargo test --locked --lib -- ui::sql_results ui::editor_results_tests`
Expected: PASS, the narrow-pane tests too. Then the four checks.

- [ ] **Step 7: Commit**

```bash
git add src/ui/sql_results.rs src/ui/editor_results_tests.rs
git commit -m "Say when the timeout stopped a run, and offer to run it without one"
```

---

### Task 6: The Row limit row says Show all fetches the rest

**Files:**
- Modify: `src/ui/settings/mod.rs:158`
- Modify: `src/ui/settings/terminal.rs:1490`
- Test: `src/ui/editor_settings_tests.rs:517`, `:633`

- [ ] **Step 1: Write the failing test**

In `src/ui/editor_settings_tests.rs`, in the macOS list replace `"Caps the rows of results you browse"` with `"Caps the rows of results you browse; Show all fetches the rest"`, and in the Omarchy list `"caps results you browse"` with `"caps results you browse · show all fetches the rest"`.

- [ ] **Step 2: Run it to see it fail**

Run: `cargo test --locked --lib -- ui::editor_settings_tests`
Expected: FAIL on the macOS words.

- [ ] **Step 3: The words**

`src/ui/settings/mod.rs`, its comment too:

```rust
        // Not the board's "Adds LIMIT to results you browse; exports get
        // everything": the rows are capped as they come, the editor's
        // results have no export, and Show all under a cut result runs it
        // again with no limit.
        OptionId::RowLimit => Some("Caps the rows of results you browse; Show all fetches the rest"),
```

`src/ui/settings/terminal.rs`:

```rust
        OptionId::RowLimit => dim(&skin.say("caps results you browse · show all fetches the rest")),
```

Run `cargo fmt --all`: the terminal line is over 100 columns.

- [ ] **Step 4: Run it to see it pass**

Run: `cargo test --locked --lib -- ui::editor_settings_tests ui::settings`
Expected: PASS. The macOS small print fits its row at the board's 1440 by 1100 (the test's window); if a row-fit test fails, report it rather than shortening the words. Then the four checks.

- [ ] **Step 5: Commit**

```bash
git add src/ui/settings/mod.rs src/ui/settings/terminal.rs src/ui/editor_settings_tests.rs
git commit -m "Say in Settings › Editor that Show all fetches the rest"
```

---

### Task 7: The website

**Files:**
- Modify: `docs/_guide/sql-editor.md` (near 41 and 63)
- Modify: `docs/_guide/omarchy.md` (near 269)
- Modify: `docs/_reference/settings-and-files.md` (rows 52 and 53)

- [ ] **Step 1: The SQL editor's guide**

In `docs/_guide/sql-editor.md`, replace the sentence "When the limit cut a result short, the result says so." (lines 63 and 64: it is split across a line break) with:

```markdown
When the limit cut a result short, it says "Limited to 1,000 rows", and
**Show all** runs that statement again, alone, with no limit (it keeps
the timeout). It is offered for a run that only read, while the editor
holds the text that ran.

When the timeout stopped a run, Messages says "Cancelled after 30 s" on
the statement it stopped, and **Run without timeout** runs the whole run
again with none (it keeps the row limit). It is offered when nothing of
the run was written: a read, or a read-write run that was rolled back.
```

- [ ] **Step 2: The Omarchy guide**

After the paragraph on the card's letters in `docs/_guide/omarchy.md`:

```markdown
Under a result the limit cut, `a` shows all of it. When the timeout
stopped a run, `t` runs it again without one.
```

- [ ] **Step 3: Settings and files**

In `docs/_reference/settings-and-files.md`, the Row limit row's text ends "… nothing is added to your SQL. **Show all** under a cut result fetches the rest", and the Timeout row's "… Cancel works either way. **Run without timeout** runs a stopped run again with none".

- [ ] **Step 4: Check the site**

Run: `cd docs && bin/check` (skip if Ruby is not installed here, and say so in the pull request).
Expected: the build and its link check pass.

- [ ] **Step 5: Commit**

```bash
git add docs/_guide/sql-editor.md docs/_guide/omarchy.md docs/_reference/settings-and-files.md
git commit -m "Describe Show all and Run without timeout on the website"
```

---

### Task 8: The pull request

- [ ] **Step 1: Rebase once #181 is merged**

Run: `gh pr view 181 --json state,mergedAt`. When it is merged: `git fetch origin && git rebase origin/main`, then the four checks again. While it is open, wait: this branch holds #181's commits.

- [ ] **Step 2: Push and open the pull request**

Title: "Settings › Editor, step 2: Show all and Run without timeout". The body lists what was built; that a timeout which comes back as a whole run's error (nothing ran, no last run) offers no link; the table "Where this departs from the prompt and the boards" with each row's answer, and the tests. End it with the attribution lines the session gives.

- [ ] **Step 3: Bind it and read CI**

Use the `ccd_pr` tools: `get_status`, `bind_pr` if needed. Do not poll CI.
