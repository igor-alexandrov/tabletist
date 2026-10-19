# Managed policy 1a: the file and its limits, on Linux and macOS

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Read an administrator's `managed.toml` at a fixed system path and hold every agent limit and read-only box below what it sets. Nothing in the person's files can lift those limits.

**Architecture:** A new module, `src/managed.rs`, reads the file strictly into a `Policy`. It matches rules against a connection's `ConnectSpec` and checks whether the file is locked: owned by root, with no group or other write anywhere up its path. `App` keeps the result and applies it at four existing points:
- `send_agents_policy`: access, rows and timeout.
- `agents_on`: the server.
- `send_connect`: read-only.
- `start_agents` and `apply_settings`: log days.

The person's files are never rewritten with the floor.

**Tech Stack:** Rust, `serde` and `toml` (already dependencies), `std::os::unix::fs::MetadataExt`. No new crate.

Spec: `docs/superpowers/specs/2026-10-15-managed-policy-design.md`, run 1a of Phase 1. Written on 2026-10-18 against `main` at `58478ca` with PR 151 on top. Line numbers are from that tree.

---

## What this run builds, and what it leaves

Builds:

- `AppDirs::managed`, the policy file's path:
  - `/etc/tabletist/managed.toml` on Linux.
  - `/Library/Application Support/Tabletist/managed.toml` on macOS.
  - None on Windows until run 1c.
  - `<root>/managed.toml` under `AppDirs::at`, so tests and `--demo` never read the machine's file.
- The strict reader. It accepts every key of version 1, `hidden`, `visible_only` and `masked` included. It refuses `connections` and `service`, which later versions use.
- Matching by driver, host, port, database and path, with `*` globs.
- The locked check on Unix, logged at the start. Nothing in this run acts on it.
- The limits that need no new check:
  - `agents.allowed`, `agents.unmatched` and `agents.log_days_min`.
  - Each rule's `access_max`, `rows_max`, `timeout_max_secs` and `read_only`.
- **A rule with `hidden`, `visible_only` or `masked`.** It makes the connections it matches Off for agents until run 1b teaches the checks a rule's tables and columns.
- **Failure.** An unreadable file stops the server, makes every session read-only, and says so in `app.notice`.
- **Turning agents on while the policy forbids it.** `Turn on` and the Settings switch say why nothing started.
- A section in `docs/_reference/settings-and-files.md`.

Leaves:
- The editor, the Agents tab and the picker still show the person's own values (run 1e, after boards).
- The file is read once, at the start (run 1d).
- No held change from an unlocked file (run 1d).
- Nothing is read on Windows (run 1c).

## Files

- Modify `src/connections.rs:14`: `mod file;` becomes `pub(crate) mod file;`.
- Modify `src/connections/file.rs:65-71,156,165`: `Problem`, `line_of` and `without_values` become `pub(crate)`, so the new reader shares them.
- Modify `src/paths.rs`: the `managed` field, set in `discover` and `at`.
- Create `src/managed.rs`: the reader, matching, the locked check, and `Managed`.
- Modify `src/lib.rs`: `pub mod managed;`.
- Modify `src/app.rs`: the `managed` field, `App::new`, `agents_on`, `send_agents_policy`, `send_connect`, `start_agents`, `apply_settings`, `turn_agents_on`, and tests.
- Modify `docs/_reference/settings-and-files.md`.

---

### Task 1: Share the TOML problem helpers

**Files:** `src/connections.rs:14`, `src/connections/file.rs`

- [ ] **Step 1:** In `src/connections.rs`, change `mod file;` to `pub(crate) mod file;`.
- [ ] **Step 2:** In `src/connections/file.rs`, make these three `pub(crate)`:
  - `pub(super) struct Problem` and its two fields.
  - `fn line_of`.
  - `fn without_values`.
- [ ] **Step 3:** Run `~/.cargo/bin/cargo build --locked`. Expected: it builds, with no unused warnings.
- [ ] **Step 4:** Commit: "Let the connections file's reader share how it says a line is wrong".

### Task 2: The policy file's path

**Files:** `src/paths.rs`

- [ ] **Step 1: Failing test** in `paths.rs`'s tests:

```rust
#[test]
fn the_managed_policy_is_under_a_root_and_at_a_fixed_path_otherwise() {
    let root = Path::new("/r");
    assert_eq!(AppDirs::at(root).managed.as_deref(), Some(Path::new("/r/managed.toml")));
    #[cfg(target_os = "linux")]
    assert_eq!(
        AppDirs::discover().managed.as_deref(),
        Some(Path::new("/etc/tabletist/managed.toml"))
    );
    #[cfg(target_os = "macos")]
    assert_eq!(
        AppDirs::discover().managed.as_deref(),
        Some(Path::new("/Library/Application Support/Tabletist/managed.toml"))
    );
    #[cfg(windows)]
    assert_eq!(AppDirs::discover().managed, None);
}
```

- [ ] **Step 2:** Run `~/.cargo/bin/cargo test --locked --lib paths::`. Expected: it fails to compile, because there is no field `managed`.
- [ ] **Step 3: Implement.** Add the field after `runtime`:

```rust
    /// The administrator's policy file (`managed.toml`): at one fixed path
    /// on each system, never moved by a variable, since an agent may start
    /// Tabletist with variables of its choosing. Under the root in tests and
    /// the demo. None where it is not read yet (Windows).
    pub managed: Option<PathBuf>,
```

Add `managed: managed_file()` to both `Self { .. }` literals in `discover`. In `at`, add `managed: Some(root.join("managed.toml"))`. Then add:

```rust
/// Where an administrator's policy is kept on this system.
fn managed_file() -> Option<PathBuf> {
    #[cfg(target_os = "macos")]
    return Some(PathBuf::from("/Library/Application Support/Tabletist/managed.toml"));
    #[cfg(all(unix, not(target_os = "macos")))]
    return Some(PathBuf::from("/etc/tabletist/managed.toml"));
    #[cfg(not(unix))]
    return None;
}
```

The fallback in `discover` that calls `Self::at(...)` would put the policy under the working directory. Override it: `Self { managed: managed_file(), ..Self::at(..) }`.

- [ ] **Step 4:** Run the paths tests and `cargo build`. Fix every literal `AppDirs { .. }` the compiler names. Expected: PASS.
- [ ] **Step 5:** Commit: "Know where an administrator's policy file is on each system".

### Task 3: The reader

**Files:** create `src/managed.rs`; add `pub mod managed;` to `src/lib.rs` in alphabetical order.

`read(text) -> Result<Policy, Problem>` uses `#[serde(deny_unknown_fields)]` types and turns them into `Policy`.

- Every problem is the file's, as in `connections::file`:
  - A parser problem has its line, with the words `without_values` gives.
  - A meaning problem (a range, two lists, an empty `match`) names the rule by its number and never repeats a value.
- `connections` and `service` are read as `Option<serde::de::IgnoredAny>`, so a file that has them is refused with its own sentence, not "unknown field".
- `unmatched` defaults to `off` when there is a rule and to `as_set` when there is none.
- A table entry is read by `TableName::from_str` and a column entry by `ColumnName::from_str`. Their errors never echo the entry.

- [ ] **Step 1: Failing tests.** Inline `mod tests`, each a `read(...)` of a short file. The cases:
  - The spec's example file reads into the right `Policy`.
  - An unknown key fails at its line.
  - `access_max = "write"` fails with "a value the key cannot have", and the problem's text does not contain `write`.
  - `version = 2` says the file was written for a newer Tabletist. `version = 0` fails.
  - A file with `[[connections]]`, or with `[service]`, is refused, naming the table.
  - A rule without `match`, or with `match = {}`, fails.
  - `read_only = false` fails.
  - These are out of range: `rows_max` 0 and 5001, `timeout_max_secs` 0 and 61, `log_days_min` 0 and 3651.
  - `hidden` beside `visible_only` fails.
  - `hidden = ["a.b.c"]` fails without the entry in the text.
  - `driver = "postgresql"` fails. `driver = "*"` reads.
  - `unmatched` is `off` with a rule, `as_set` with none, and `as_set` when it is written.
- [ ] **Step 2:** Run `cargo test --locked --lib managed::`. Expected: it fails to compile.
- [ ] **Step 3: Implement** the types and `read` (`src/managed.rs`).
- [ ] **Step 4:** Run it. Expected: PASS.
- [ ] **Step 5:** Commit: "Read an administrator's policy file strictly".

### Task 4: Matching

**Files:** `src/managed.rs`

`glob(pattern, text)` handles `*` alone, with no other special character. `Match::matches(spec, home)` checks these keys:
- **`driver`**: matched against `postgres`, `mysql` or `sqlite`, by a match that names every `Driver`.
- **`host`**: compared without regard to case.
- **`port`**: compared as a number.
- **`database`**: compared with regard to case.
- **`path`**: only SQLite has one. It is compared after `~` is expanded with the person's home and the path is made absolute with `std::path::absolute`. Symlinks are not resolved.

A key the driver has no use for does not match.

- [ ] **Step 1: Failing tests:**
  - `glob` cases: none, `*` alone, a prefix, a suffix, a middle, two stars, an empty text, and backtracking (`a*b*c` against `aXbYbZc`).
  - Matching: a host in other case, a port, a database glob, a tunnel's database host, `path` against `~/x.db` with a home, `path` on PostgreSQL, `host` on SQLite.
- [ ] **Step 2:** Run it. Expected: it fails.
- [ ] **Step 3: Implement.**
- [ ] **Step 4:** Run it. Expected: PASS.
- [ ] **Step 5:** Commit: "Match a policy's rules by what a connection reaches".

### Task 5: Locked

**Files:** `src/managed.rs`

`held_by_root(link, uid, mode)` is true when the entry is not a link, `uid == 0` and `mode & 0o022 == 0`. `locked(path)` asks it of the file and of every directory above it, through `symlink_metadata`. It is `false` off Unix.

- [ ] **Step 1: Failing tests:**
  - `held_by_root` for root 0644, root 0664 (group writes), root 0646 (others write), a link, and uid 1000.
  - `locked` of a temporary file is false: the test's user owns it. The test returns early when it runs as root (`/proc/self` owned by 0 on Linux, or `uid == 0` read from the temp file's metadata).
- [ ] **Step 2 to 4:** Run, implement, and pass.
- [ ] **Step 5:** Commit: "Tell a policy only an administrator can change from one anyone can".

### Task 6: `Managed`, and what it lets agents do

**Files:** `src/managed.rs`

```rust
pub enum Managed {
    None,
    Read { path: PathBuf, policy: Policy, locked: bool },
    Unreadable { path: PathBuf, problem: Problem },
}
```

- **`load(path)`**:
  - A missing file is `None`.
  - Any other error reading it is `Unreadable`, worded by its `ErrorKind`.
  - A file that reads is `Read`, with `locked(path)`.
  - Each outcome is logged: info for `Read`, saying whether it is locked; error for `Unreadable`, with the problem.
- **`notice()`**: the sentence for `Unreadable`.
- **`server_allowed()`**.
- **`forces_read_only(spec, home)`**.
- **`floor(spec, home, &mut Agents)`**.
- **`log_days(days)`**.

- [ ] **Step 1: Failing tests:**
  - `load` of a path that isn't there is `None`. `load` of a file with a bad line is `Unreadable`, and its notice names the line and says agents are off and connections read-only.
  - `floor`:
    - Lowers access, rows and timeout to the lowest of the person's value and each matching rule's ceiling.
    - Leaves a connection no rule matches as it is under `as_set`, and turns it Off under `off`.
    - Turns access Off where a matching rule has a non-empty `hidden`, any `visible_only` (`[]` hides every table) or a non-empty `masked`.
    - Turns every connection Off when the file is `Unreadable`.
  - `server_allowed` is false for `allowed = false` and for `Unreadable`, and true for `None`.
  - `forces_read_only` is true for a matching rule's `read_only` and for `Unreadable`.
  - `log_days` is the larger of the two values.
- [ ] **Step 2 to 4:** Run, implement, and pass.
- [ ] **Step 5:** Commit: "Lower what agents may do to what an administrator's policy lets them".

### Task 7: The app holds to it

**Files:** `src/app.rs`

- [ ] **Step 1: Failing tests** in `app.rs`'s tests, `#[cfg(any(unix, windows))]` where they serve agents. A helper writes `managed.toml` into the temporary root before `App::new`:

```rust
fn app_under(policy: &str) -> (App, tempfile::TempDir) {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("managed.toml"), policy).unwrap();
    let app = App::new(AppDirs::at(dir.path()), Settings::default().into(), Backend::recording());
    (app, dir)
}
```

The tests:
- `a_policy_caps_the_access_rows_and_timeout_sent_to_the_gateway`.
- `a_policy_turns_agents_off_where_no_rule_matches`.
- `a_policy_that_forbids_agents_keeps_the_server_from_starting_and_says_so_when_turned_on`.
- `a_rule_makes_its_connections_read_only_for_the_person`: `asked_access` after `Action::Connect` on a writable connection.
- `an_unreadable_policy_stops_agents_makes_every_connection_read_only_and_says_so`.
- `a_policys_log_days_are_the_least_the_server_keeps_calls`.
- `the_person_s_files_never_take_the_floor`: after a save, `connections.toml` still has the person's Read.
- [ ] **Step 2:** Run `cargo test --locked --lib app::tests::a_policy`. Expected: it fails.
- [ ] **Step 3: Implement:**
  - **The field.** `pub managed: crate::managed::Managed`, documented, after `agents_held`.
  - **`App::new`.** `let managed = crate::managed::Managed::load(dirs.managed.as_deref());`, then `notice: managed.notice().or(loaded_connections.problem).or(loaded_connections.moved)`.
  - **`agents_on`.** `&& self.managed.server_allowed()`.
  - **`send_agents_policy`.** After the three lines that set `now.agents`, add `self.managed.floor(&saved.spec, self.dirs.home.as_deref(), &mut now.agents);`.
  - **`send_connect`.** Before taking `workspace_mut`, work out `forced` from `self.workspace(tab)`. After the box is read, `if forced { workspace.access = tabletist_db::Access::ReadOnly; }`. This holds for a session that keeps its access too (`keep_access`).
  - **Log days.** In `start_agents` and in `apply_settings`'s `LogDays`, use `self.managed.log_days(self.settings.mcp_log_days)`.
  - **`turn_agents_on`.** First: `if !self.managed.server_allowed() { self.notice = Some(AGENTS_NOT_ALLOWED.into()); return; }`.
  - **`apply_settings`.** When the settings turn the server on and the policy does not allow it, set the same notice.
- [ ] **Step 4:** Run the whole lib test suite. Expected: PASS.
- [ ] **Step 5:** Commit: "Hold agents and read-only boxes to an administrator's policy".

### Task 8: The reference page

**Files:** `docs/_reference/settings-and-files.md`

- [ ] **Step 1:** Add `managed.toml` to the Files table.
- [ ] **Step 2:** Add a section, "The administrator's policy". It covers:
  - The paths. Windows is not read yet.
  - The example and the keys table, from the spec.
  - Matching, combining and failure.
  - What the file does not do yet: a rule's tables and columns turn agents off on its connections; the file is read when Tabletist starts; the editor shows your own values.
- [ ] **Step 3:** Commit: "Say on the website what an administrator's policy file does".

### Task 9: Checks

- [ ] `cargo fmt --all --check`
- [ ] `cargo clippy --locked --workspace --all-targets -- -D warnings`
- [ ] `cargo test --locked --workspace --all-targets`
- [ ] `RUSTDOCFLAGS='-D warnings' cargo doc --locked --workspace --no-deps`
- [ ] Type-check the `cfg(windows)` and `cfg(target_os = "macos")` branches of `paths.rs` and `managed.rs` in a scratch crate. The workspace does not cross-compile here.
- [ ] Mark run 1a built in the spec's Phase 1 list, with what went otherwise.
