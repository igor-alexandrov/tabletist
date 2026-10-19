# Managed policy 1b: tables and columns from rules

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Enforce a rule's `hidden`, `visible_only` and `masked` entries beside the person's own, instead of turning agents Off on the connections the rule matches.

**Architecture:** The agent checks in `tabletist-db` take a `Sources`: the person's `Tables` setting and masked columns, plus one `Source` for each matching rule. `Hidden` hides a relation that any source hides and masks a column that any source masks. It also keeps who caused each refusal, as a `By`, so that a refusal can say "by your administrator's policy" with the same code. `Sources` replaces the pair `tables: &Tables, masked: &[ColumnName]` all the way up: in the drivers, in `Connection::run_agent_read` and `agent_hidden`, in the MCP crate's `Listed` and `Session`, and in the app's `Agentable`. `Managed::sources` hands `send_agents_policy` the rules' sources, and `Managed::floor` no longer turns access Off for them.

**Tech Stack:** Rust. No new crate.

Spec: `docs/superpowers/specs/2026-10-15-managed-policy-design.md`, run 1b of Phase 1. The MCP plan's B.8 and the C.3 note "With the managed policy" give the sentences. Written on 2026-10-18 against `claude/managed-policy-1a` at `7ae48e5` (PR 172). Line numbers are from that tree.

---

## What this run builds, and what it leaves

Builds:

- **`agent::By`** (`Person`, `Administrator`), **`agent::Source`** (one place's `tables` and `masked`) and **`agent::Sources`** (`person`, and `administrator`, one per matching rule).
- **A table is visible only if every source lets it be seen.** Several "Only" lists come to their intersection, since each source hides on its own. A view or a part of a hidden table is hidden as before. Its `Why` says whose entry hid it.
- **Masked columns are the sources' lists joined**, the administrator's first. A refusal that names an entry knows whose it is from its place in that list.
- **Stale entries.** A stale "All except" entry or masked entry of the person's still gives `SettingsNeedALook` with its sentence. One of a rule's gives a new `Refusal::PolicyNeedsFixing`: code `connection_unavailable`, sentence "Agents may not use this connection until your administrator's policy is fixed." That sentence names no entry, so no value from the file is told. Where both are stale, the administrator's comes first, since the person cannot fix it. A stale "Only" entry stops nothing, from any source.
- **`TableHidden` and `MaskedColumnUsed` gain `by: By`.** For `By::Administrator`, "on this connection" in each sentence becomes "by your administrator's policy". The codes stay the same.
- **`list_connections`.** It keeps the person's `hidden`, `only` and `masked` as they are. It adds `administrator`: a list with one object for each rule that hides or masks anything, each with its own `hidden`, `only` and `masked`. Rule names are not sent.
- **`Managed::sources(spec, home)`.** The matching rules' tables and columns. `send_agents_policy` puts them on each `Agentable`. `Managed::floor` stops turning access Off for a rule that names tables or columns, and `names_tables_or_columns` goes.
- **The reference page**, and the spec's run list.

Leaves:

- The editor still shows the person's own entries (run 1e).
- Watching the file, and the held change from an unlocked file (run 1d).
- Windows (run 1c).

## Files

- `crates/tabletist-db/src/agent/hidden.rs`: `By`, `Source`, `Sources`, and `Hidden` over several sources.
- `crates/tabletist-db/src/agent/hidden/masked.rs`: `Masks` keeps each entry's `By`, and `used` becomes `Masks::used`.
- `crates/tabletist-db/src/agent.rs`: the `by` fields, `PolicyNeedsFixing`, the sentences, and the exports.
- `crates/tabletist-db/src/{lib.rs,adapter.rs}`: `run_agent_read`, `agent_hidden` and `Adapter::run_read` take `&Sources`.
- `crates/tabletist-db/src/pg/{script.rs,hidden.rs}`, `mysql/{script.rs,hidden.rs}`, `sqlite.rs`, `sqlite/fence.rs`: the same, through `Hidden::of`. `planned_masks` and `through_view` keep the `by`.
- `crates/tabletist-db/tests/agent.rs`: a rule's entries on every driver. This file is already in ci.yml's PostgreSQL and MySQL jobs, so ci.yml does not change.
- `crates/tabletist-mcp/src/{gateway.rs,tools/connections.rs,tools/query.rs,tools/schema.rs}` and `tests/support/mod.rs`.
- `src/backend/agents.rs`: `Agentable.sources`.
- `src/managed.rs`: `sources`, and `floor` without the Off.
- `src/app.rs`: `send_agents_policy`.
- `docs/_reference/settings-and-files.md`, and the spec.

---

### Task 1: Several sources in the checks

**Files:** `crates/tabletist-db/src/agent/hidden.rs`, `hidden/masked.rs`, `agent.rs`

The types, in `hidden.rs`, are exported from `agent`:

```rust
/// Whose setting hides a table or masks a column: the person's own, or a
/// rule of an administrator's policy, which the person cannot change.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum By { #[default] Person, Administrator }

/// One place's say on what agents may read of a connection.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Source { pub tables: Tables, pub masked: Vec<ColumnName> }

/// Every place's say: the person's, and each matching rule's. A table is
/// visible only where each lets it be seen; a column is masked where any
/// masks it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Sources { pub person: Source, pub administrator: Vec<Source> }
```

`Sources` has these methods:
- `person(tables, masked)`: the person's alone.
- `hides_anything()`.
- `guards_anything()`: hides or masks.
- `masked()`: the joined list, the administrator's first.

`Hidden` changes:
- **`Hidden::of(dialect, catalog, &Sources)`** is the constructor. `new` and `masking` stay as the person's alone.
- **`tables`** becomes `Vec<(By, Tables)>`, the administrator's first. A relation's `Why` is `Named(By)` or `DependsOn(String, By)`.
- **An opaque relation** is hidden by `By::Administrator` where a rule guards anything, and by the person otherwise.
- **`masked_by: Vec<By>`** keeps the same order as `masked`. A generated column is the administrator's when one of the administrator's entries names its table.
- **`missing` and `masked_missing`** stay the person's. A new `policy_stale: bool` is true when a rule's "All except" entry or masked entry names nothing.
- **`check_settings`** gives `PolicyNeedsFixing` first, then the person's `SettingsNeedALook`.
- **`pub fn masked_by(&self, entry: &str) -> By`**, for refusals made outside the module (PostgreSQL's plan).

`Masks::new(masked, by, more)`. `Masks::used(entry, how)` finds the entry's place by address, as `masked_views` already does.

- [ ] **Step 1: Failing tests** in `hidden.rs`'s tests, over a Bookshop catalog:
  - A rule's "All except" hides `customers` with `By::Administrator`, and `recent_customers` through it. The person's hides with `By::Person`.
  - The person's "Only" list and a rule's "Only" list come to their intersection.
  - A rule's stale "All except" entry gives `PolicyNeedsFixing`, even beside a stale entry of the person's. A rule's stale "Only" entry stops nothing.
  - A rule's stale masked entry gives `PolicyNeedsFixing`.
  - A rule's masked column is refused with `By::Administrator`. The same entry from both sources is refused as the administrator's.
  - The sentences, in `agent.rs`'s tests, for `By::Administrator`: "customers is hidden for agents by your administrator's policy.", the `through` form, a masked one, and `PolicyNeedsFixing`'s.
- [ ] **Step 2:** Run `~/.cargo/bin/cargo test --locked -p tabletist-db --lib agent::`. Expected: it fails to compile.
- [ ] **Step 3: Implement.** Add `by` to every place a `TableHidden` or `MaskedColumnUsed` is built or matched. Tests that build one take `by: By::Person`.
- [ ] **Step 4:** Run it. Expected: PASS.

### Task 2: The drivers and `Connection` take `Sources`

**Files:** `lib.rs`, `adapter.rs`, `pg/script.rs`, `pg/hidden.rs`, `mysql/script.rs`, `mysql/hidden.rs`, `sqlite.rs`, `sqlite/fence.rs`, `tests/agent.rs`

- [ ] **Step 1:** Replace `tables: &Tables, masked: &[ColumnName]` with `sources: &Sources` in each of these:
  - `run_agent_read`, `agent_hidden` and `Adapter::run_read`.
  - Each driver's `run_read` and `agent_read`.
  - The `Hiding` structs.
  - `hidden::check` on PostgreSQL and MySQL.

  The catalog is read for `sources.masked()`. `planned_masks` builds its refusal with `hidden.masked_by(..)`. `through_view` keeps the `by` it is given.
- [ ] **Step 2: Failing tests** in `tests/agent.rs`, run on SQLite and, with the variables set, on PostgreSQL and MySQL. Add a new `a_rule_hides_and_masks_beside_the_person` to `an_agent_reads_the_bookshop`-style callers for each driver:
  - A rule's `customers` refused with the administrator's sentence. The person's `books` refused with the person's.
  - A rule's "Only" list over the person's "All except".
  - A rule's masked `orders.email` refused by the administrator, and answered `•••`.
  - A rule's stale `hidden` entry closing the connection with `PolicyNeedsFixing`, on reads and in `agent_hidden(..).check_settings()`.
  - A rule's stale `visible_only` entry stopping nothing.
- [ ] **Step 3:** Run `cargo test --locked -p tabletist-db`. Expected: PASS. Then run it with `TABLETIST_TEST_PG_URL` and `TABLETIST_TEST_MYSQL_URL` set, where the servers are up on 55432 and 53306.
- [ ] **Step 4:** Commit: "Let the agent checks take a rule's tables and columns beside the person's".

### Task 3: The server and the app pass the sources

**Files:** `crates/tabletist-mcp/src/gateway.rs`, `tools/{connections,query,schema}.rs`, `tests/support/mod.rs`, `tests/tools.rs`, `src/backend/agents.rs`

- [ ] **Step 1:** `Listed` gets `sources: Sources` in place of `tables` and `masked`. `Session::hidden(&Sources)` and `Session::read(read, limits, &Sources)`.
- [ ] **Step 2: Failing test.** `list_connections` with a rule's "All except", "Only" and masked lists gives `administrator: [{ "hidden": [..] }, { "only": [..], "masked": [..] }]`, beside the person's keys. A connection with no rule's entries has no `administrator`. The tool's description says what `administrator` is.
- [ ] **Step 3: Implement.** `Agentable.sources` replaces `tables` and `masked`. `agentable` fills `person`.
- [ ] **Step 4:** Run `cargo test --locked -p tabletist-mcp` and the lib tests. Expected: PASS.

### Task 4: The policy hands its rules' sources over

**Files:** `src/managed.rs`, `src/app.rs`

- [ ] **Step 1: Failing tests:**
  - `managed.rs`: `floor` leaves access as it is for a rule with `hidden`, `visible_only` or `masked`. `sources` gives one `Source` per matching rule that names tables or columns, `visible_only = []` as `Only([])`, and nothing for `None` or `Unreadable`.
  - `app.rs`: a rule's hidden table reaches the gateway in `Agentable.sources.administrator`, and the connection's access is the person's.
- [ ] **Step 2: Implement:**
  - In `floor`, remove the Off for tables and columns, and delete `names_tables_or_columns`.
  - Add `pub fn sources(&self, spec, home) -> Vec<tabletist_db::agent::Source>`.
  - In `send_agents_policy`, after `agentable`, set each one's `sources.administrator = self.managed.sources(&agentable.spec, home)`.
  - Update the module's doc.
- [ ] **Step 3:** Run the lib tests. Expected: PASS.
- [ ] **Step 4:** Commit: "Hide and mask a rule's tables and columns for agents, and say so".

### Task 5: The reference page and the spec

- [ ] **Step 1:** In "The administrator's policy" in `docs/_reference/settings-and-files.md`:
  - Add `hidden`, `visible_only` and `masked` to the example.
  - Say how they combine with your own settings.
  - Say what a stale entry does.
  - Say what agents are told.
  - Remove "are not yet applied".
- [ ] **Step 2:** Mark run 1b built in the spec, with what went otherwise.
- [ ] **Step 3:** Commit: "Say on the website what a rule's tables and columns do".

### Task 6: Checks

- [ ] `cargo fmt --all --check`
- [ ] `cargo clippy --locked --workspace --all-targets -- -D warnings`
- [ ] `cargo test --locked --workspace --all-targets`
- [ ] `RUSTDOCFLAGS='-D warnings' cargo doc --locked --workspace --no-deps`
- [ ] Push, and open a pull request on `claude/managed-policy-1a`.
