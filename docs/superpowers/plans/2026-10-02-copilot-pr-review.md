# Copilot pull request review Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Copilot reviews every pull request before any other pull request job runs, and when it requests changes nothing else runs for that commit.

**Architecture:** One GitHub Agentic Workflow (gh-aw), `.github/workflows/pr-review.md`, becomes the only workflow triggered by `pull_request`. Its agent posts one review (inline comments plus a summary, `COMMENT` or `REQUEST_CHANGES`) through gh-aw safe outputs. A small `gate` job then reads Copilot's review of the head commit from the GitHub API: it fails on `CHANGES_REQUESTED`, otherwise the existing `ci.yml` and `packaging.yml` run as reusable workflows (`workflow_call`) that `need` the gate. Ordering is real `needs:` ordering inside one run, not polling between workflows.

**Tech Stack:** GitHub Actions, gh-aw v0.89.21 (`gh aw`, already installed as a `gh` extension), Copilot engine, `gh api` + `jq` in the gate, actionlint 1.7.12.

**Source:** <https://github.github.com/gh-aw/gallery/automated-pr-review/> (the gallery's "Automated AI pull request review" workflow). What this plan changes from it is listed under "Decisions" below.

---

## Ground rules (read first)

- Work in `/home/igor/Work/tabletist/.claude/worktrees/copilot-pr-review-workflow-23159e`, branch `claude/copilot-pr-review-workflow-23159e`.
- **Never commit or push unless the user asks.** Each task ends with a "commit point": stop there and report. If the user has asked to commit: one topic per commit, signed (`git commit -S`). If signing fails, ask before committing unsigned. Never set `SSH_AUTH_SOCK` in git commands. End messages with `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.
- No em dashes anywhere (workflows, comments, docs, commit messages). Use a full stop, comma, colon or parentheses.
- `pr-review.lock.yml` is generated. Never edit it by hand: edit `pr-review.md`, then run `gh aw compile pr-review`. Commit the two together, always.
- No Rust changes in this plan, so no cargo runs. The checks here are `gh aw compile --actionlint` and actionlint on the hand-written workflows.
- Lint commands:

  ```bash
  gh aw compile pr-review --actionlint
  ~/.local/share/mise/installs/actionlint/1.7.12/actionlint .github/workflows/ci.yml .github/workflows/packaging.yml .github/workflows/release.yml
  ```

  The `actionlint` mise shim has no version set here, hence the full path. Do not run the standalone actionlint on `pr-review.lock.yml`: 1.7.12 rejects a `queue: max` key gh-aw generates (`unexpected key "queue" for "concurrency" section`). That is not ours to fix. The lock file is linted by `gh aw compile --actionlint`, which passes.

## Decisions (what differs from the gallery page, and why)

These are the product decisions. Say so before execution if any should go the other way.

1. **One pipeline instead of three workflows.** `ci.yml` and `packaging.yml` lose their `pull_request` trigger and gain `workflow_call`; `pr-review.md` calls them after the gate. This is the only way to get "runs before all other jobs" with plain `needs:`. The alternative (each workflow keeps `pull_request` and starts with a job that polls for the review) burns a runner per workflow while waiting and races at startup. Consequence: check names on a pull request change from `CI / quality` to `PR / ci / quality` and so on. `main` has no required status checks, so nothing else needs updating. `push` to `main` and `workflow_dispatch` are untouched.
2. **`REQUEST_CHANGES` is allowed.** The gallery page allows only `COMMENT`. The request needs a blocking verdict, so `allowed-events: [COMMENT, REQUEST_CHANGES]`, with `supersede-older-reviews: true` so a clean review of a later commit dismisses the older blocking ones (otherwise the pull request stays "changes requested" forever).
3. **The summary is the review body.** The gallery page posts a separate summary comment (`add-comment`) next to the review. With `submit-pull-request-review` the review has a body, so a second comment would only double the notifications on every push. `add-comment` is left out.
4. **Only an explicit request for changes blocks.** The checks still run when there is no review: pull requests from forks and from Dependabot (gh-aw skips the agent for them, they have no secrets), a Copilot outage, a missing token. In the failure cases the run is red at the agent, so it is visible, but the checks are not held hostage.
5. **A blocked commit is red, not green with skipped jobs.** The gate job fails with an explanation when Copilot requested changes. A green run whose checks were all skipped would read as "passed".
6. **The verdict is read from GitHub, not from the agent's output.** The gate takes the latest review by `github-actions[bot]` whose `commit_id` is the head commit (gh-aw pins the review to `github.event.pull_request.head.sha`). So it reflects what was posted, and a human can override: dismiss the review on the pull request, then re-run the `review gate` job. `gh workflow run ci.yml --ref <branch>` also still works.
7. **`reopened` is added to the trigger types** (the gallery page has `opened, synchronize`). The checks now hang off this workflow, and `ci.yml` used to run on reopen.
8. **The packaging path filter moves into the gate.** A called workflow cannot have a `paths:` filter, so the gate lists the pull request's files and matches the same paths `packaging.yml` watches on `main`. The list now lives twice (comments in both files say so).

## What the user has to do (cannot be done from here)

- **Add the `COPILOT_GITHUB_TOKEN` secret.** The repository has no secrets today. It belongs to a user account, so the keyless `copilot-requests: write` permission (organisation billing) is not available. Create a fine-grained PAT owned by the user account with **Account permissions: Copilot Requests: Read**, then:

  ```bash
  gh aw secrets set COPILOT_GITHUB_TOKEN --value "<the PAT>"
  ```

  Pre-filled PAT form: <https://github.com/settings/personal-access-tokens/new?name=COPILOT_GITHUB_TOKEN&description=GitHub+Agentic+Workflows+-+Copilot+engine+authentication&user_copilot_requests=read>. Each review spends Copilot premium requests from that account, once per push to a pull request.
- Without the secret the workflow's `activation` job fails, there is no review, and the checks run anyway (decision 4).

## Not verified until the first real run

Local checks cover compilation, lint and the gate's shell logic. These need the pull request in Task 4:

- That `GITHUB_TOKEN` may submit a `REQUEST_CHANGES` review. The repository setting "Allow GitHub Actions to create and approve pull requests" is off; it is documented as covering creating and approving, not requesting changes.
- That the review's author login is exactly `github-actions[bot]` (it is when no `GH_AW_GITHUB_TOKEN` secret is set, which is the case).
- That the checks run when there is no review (decision 4). Task 4 tests it first, before the secret exists.
- What Copilot actually chooses to block on. The prompt can be tuned afterwards.

## File structure

| File | Change | Responsibility |
|---|---|---|
| `.github/workflows/pr-review.md` | create | The workflow source: trigger, safe outputs, the gate and the two calls, the review prompt |
| `.github/workflows/pr-review.lock.yml` | create (generated) | What GitHub Actions runs. Output of `gh aw compile` |
| `.github/aw/actions-lock.json` | create (generated) | gh-aw's pin of its own setup action |
| `.gitattributes` | create (generated) | Marks `*.lock.yml` as generated for GitHub diffs |
| `.github/workflows/ci.yml` | modify lines 3-7 | `pull_request` becomes `workflow_call` |
| `.github/workflows/packaging.yml` | modify lines 3-11 | `pull_request` with paths becomes `workflow_call` |
| `AGENTS.md` | modify | How the pull request pipeline works and how to change it |

---

### Task 0: The plan

**Files:**
- Add: `docs/superpowers/plans/2026-10-02-copilot-pr-review.md` (this file, already written)

- [ ] **Step 1: Commit point**

```bash
git add docs/superpowers/plans/2026-10-02-copilot-pr-review.md
git commit -S -m "Add the plan for the Copilot pull request review

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 1: The review workflow

Copilot reviews pull requests. Nothing is gated yet: `ci.yml` and `packaging.yml` still run on `pull_request` beside it.

**Files:**
- Create: `.github/workflows/pr-review.md`
- Create (generated): `.github/workflows/pr-review.lock.yml`, `.github/aw/actions-lock.json`, `.gitattributes`

- [ ] **Step 1: Check the compiler version**

Run: `gh aw --version`
Expected: `gh aw version v0.89.21`. A different version compiles a different lock file. That is fine, but note it in the report.

- [ ] **Step 2: Write `.github/workflows/pr-review.md`**

````markdown
---
name: PR
description: Copilot reviews every pull request.
run-name: "PR #${{ github.event.pull_request.number }}"

on:
  pull_request:
    types: [opened, synchronize, reopened]

permissions:
  contents: read
  pull-requests: read

engine: copilot

safe-outputs:
  create-pull-request-review-comment:
    max: 10
  submit-pull-request-review:
    max: 1
    allowed-events: [COMMENT, REQUEST_CHANGES]
    # A clean review of a later commit dismisses the older blocking ones.
    supersede-older-reviews: true
---

# Pull Request Review Assistant

Review the diff of pull request #${{ github.event.pull_request.number }} for correctness, security, maintainability, and test coverage. Read `AGENTS.md` first and hold the change to the rules it sets for this repository.

Create inline review comments only for specific problems or concrete improvements. Do not restate unchanged code or provide style-only feedback.

Finish by submitting exactly one review. Its body is the summary: group the findings by severity and note anything that needs human follow-up.

- Submit it as `REQUEST_CHANGES` only when the change has a problem that must be fixed before merging: a bug, a security problem, data loss, a broken build on Linux, macOS or Windows, or a behaviour change without a regression test. Name each blocking problem and what would resolve it.
- Otherwise submit it as `COMMENT`. When you found nothing worth raising, say so in one sentence and add no inline comments.
````

Notes for the implementer:
- `name: PR` is the workflow name shown on checks. Without `run-name` gh-aw titles every run "PR"; the number makes the Actions list readable.
- The agent job itself only gets `contents: read` and `pull-requests: read`. The review is written by gh-aw's separate `safe_outputs` job, which validates it first.
- "Always exactly one review, also when clean" is deliberate: `supersede-older-reviews` only dismisses an older blocking review after a clean `COMMENT` review with no inline comments.

- [ ] **Step 3: Compile**

Run: `gh aw compile pr-review --actionlint`
Expected, among the output:

```
✓ .github/workflows/pr-review.md (124.0 KB)
✓ Compiled 1 workflow: 1 succeeded, 0 warnings
✓ Checked 1 workflow(s)
✓ No issues found
```

- [ ] **Step 4: Check what was generated**

Run: `git status --short`
Expected exactly (plus `?? docs/superpowers/plans/2026-10-02-copilot-pr-review.md` if the plan was not committed in Task 0):

```
?? .gitattributes
?? .github/aw/
?? .github/workflows/pr-review.lock.yml
?? .github/workflows/pr-review.md
```

Run: `cat .gitattributes; cat .github/aw/actions-lock.json`
Expected: `.gitattributes` is the single line `.github/workflows/*.lock.yml linguist-generated=true`; `actions-lock.json` has one entry, `github/gh-aw-actions/setup@v0.89.21`.

Run: `grep -nE '^  [a-z_]+:$' .github/workflows/pr-review.lock.yml`
Expected jobs: `activation`, `agent`, `conclusion`, `detection`, `pre_activation`, `safe_outputs` (plus the `pull_request:` trigger line).

- [ ] **Step 5: Commit point**

```bash
git add .gitattributes .github/aw/actions-lock.json .github/workflows/pr-review.md .github/workflows/pr-review.lock.yml
git commit -S -m "Review pull requests with Copilot

A gh-aw workflow: Copilot reads the diff and posts one review, as a
comment or as a request for changes. Nothing waits for it yet.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 2: Hold the checks behind the review

**Files:**
- Modify: `.github/workflows/pr-review.md` (description, a `jobs:` block, one sentence of the prompt)
- Modify: `.github/workflows/ci.yml:3-7`
- Modify: `.github/workflows/packaging.yml:3-11`
- Regenerate: `.github/workflows/pr-review.lock.yml`

There is no test harness for workflows in this repository, and this plan does not add one. Step 1 runs the gate's two shell snippets against made-up and real data before they go into the workflow. It is a throwaway check, not a committed test.

- [ ] **Step 1: Prove the gate's shell logic**

Run (prints a table, changes nothing; it reads three merged pull requests of this repository):

```bash
bash <<'EOF'
set -e
HEAD_SHA=bbb
verdict() {
  states=$(jq -r ".[] | select(.user.login == \"github-actions[bot]\" and .commit_id == \"$HEAD_SHA\") | .state" <<<"$1")
  tail -n 1 <<<"$states"
}
t() { printf '%-45s -> %s\n' "$1" "$(verdict "$2")"; }
bot='"user":{"login":"github-actions[bot]"}'
t "no reviews" '[]'
t "a human requested changes on head" '[{"user":{"login":"igor"},"commit_id":"bbb","state":"CHANGES_REQUESTED"}]'
t "copilot blocked an older commit only" "[{$bot,\"commit_id\":\"aaa\",\"state\":\"CHANGES_REQUESTED\"}]"
t "copilot blocked head" "[{$bot,\"commit_id\":\"aaa\",\"state\":\"COMMENTED\"},{$bot,\"commit_id\":\"bbb\",\"state\":\"CHANGES_REQUESTED\"}]"
t "blocked head, then a clean re-run" "[{$bot,\"commit_id\":\"bbb\",\"state\":\"DISMISSED\"},{$bot,\"commit_id\":\"bbb\",\"state\":\"COMMENTED\"}]"
t "blocked head, a human dismissed it" "[{$bot,\"commit_id\":\"bbb\",\"state\":\"DISMISSED\"}]"

watched='^(packaging/|\.cargo/|crates/tabletist-appkit/|(Cargo\.toml|Cargo\.lock|build\.rs|src/main\.rs|src/macos\.rs|\.github/workflows/(packaging|release)\.yml)$)'
for f in packaging/arch/render.sh .cargo/config.toml crates/tabletist-appkit/src/lib.rs Cargo.toml Cargo.lock build.rs src/main.rs src/macos.rs .github/workflows/packaging.yml .github/workflows/release.yml \
         src/app.rs crates/tabletist-db/Cargo.toml src/main.rs.bak docs/packaging/x.md .github/workflows/ci.yml src/ui/build.rs; do
  if grep -Eq "$watched" <<<"$f"; then echo "watched  $f"; else echo "ignored  $f"; fi
done
for n in 38 39 40; do
  files=$(gh api --paginate "repos/igor-alexandrov/tabletist/pulls/$n/files" --jq '.[] | .filename, (.previous_filename // empty)')
  if grep -Eq "$watched" <<<"$files"; then echo "PR $n: packaging=true"; else echo "PR $n: packaging=false"; fi
done
EOF
```

Expected:

```
no reviews                                    -> 
a human requested changes on head             -> 
copilot blocked an older commit only          -> 
copilot blocked head                          -> CHANGES_REQUESTED
blocked head, then a clean re-run             -> COMMENTED
blocked head, a human dismissed it            -> DISMISSED
watched  packaging/arch/render.sh
watched  .cargo/config.toml
watched  crates/tabletist-appkit/src/lib.rs
watched  Cargo.toml
watched  Cargo.lock
watched  build.rs
watched  src/main.rs
watched  src/macos.rs
watched  .github/workflows/packaging.yml
watched  .github/workflows/release.yml
ignored  src/app.rs
ignored  crates/tabletist-db/Cargo.toml
ignored  src/main.rs.bak
ignored  docs/packaging/x.md
ignored  .github/workflows/ci.yml
ignored  src/ui/build.rs
PR 38: packaging=false
PR 39: packaging=false
PR 40: packaging=false
```

Only "copilot blocked head" yields `CHANGES_REQUESTED`: that is the single case the gate fails on. The watched list is the `paths:` list of `packaging.yml` (`packaging/**`, `.cargo/**`, `Cargo.toml`, `Cargo.lock`, `build.rs`, `src/main.rs`, `src/macos.rs`, `crates/tabletist-appkit/**`, `.github/workflows/packaging.yml`, `.github/workflows/release.yml`), and the root-only files do not match in subdirectories. If any line differs, stop and fix the snippet before going on.

- [ ] **Step 2: Make `ci.yml` callable**

In `.github/workflows/ci.yml`, replace

```yaml
on:
  push:
    branches: [main]
  pull_request:
  workflow_dispatch:
```

with

```yaml
# Pull requests reach these jobs through `pr-review.md`, after Copilot's review.
on:
  push:
    branches: [main]
  workflow_call:
  workflow_dispatch:
```

Leave the `concurrency` block as it is. Inside a called workflow `github.workflow` is the caller's name and `github.event` the caller's event, so the group becomes `ci-PR-<number>`, which does not collide with the caller's own `gh-aw-PR-<number>`.

- [ ] **Step 3: Make `packaging.yml` callable**

In `.github/workflows/packaging.yml`, replace

```yaml
# Builds the macOS app and DMG and the Windows installer the way the release
# does, and installs them, whenever packaging changes. No signing here.
on:
  pull_request:
    paths: [packaging/**, .cargo/**, Cargo.toml, Cargo.lock, build.rs, src/main.rs, src/macos.rs, crates/tabletist-appkit/**, .github/workflows/packaging.yml, .github/workflows/release.yml]
  push:
```

with

```yaml
# Builds the macOS app and DMG and the Windows installer the way the release
# does, and installs them, whenever packaging changes. No signing here.
# Pull requests reach these jobs through `pr-review.md`, after Copilot's review;
# its gate watches the same paths, so keep the two lists the same.
on:
  workflow_call:
  push:
```

The `push:` block below it (branches and paths) and `workflow_dispatch:` stay.

- [ ] **Step 4: Add the gate and the two calls to `pr-review.md`**

Three edits.

(a) The description line becomes:

```yaml
description: Copilot reviews every pull request before anything else runs. The checks start only when it has not requested changes.
```

(b) Between `supersede-older-reviews: true` and the closing `---` of the frontmatter, add a blank line and:

```yaml
# gh-aw runs a custom job before the agent unless the job names `agent` in its
# `needs`, so every job below does, also the ones that only wait for the gate.
jobs:
  gate:
    name: review gate
    needs: [agent, safe_outputs]
    # Runs when the review was skipped (forks, Dependabot) or broke, too: only
    # a review that requests changes holds the checks back.
    if: ${{ !cancelled() }}
    runs-on: ubuntu-latest
    timeout-minutes: 5
    permissions:
      pull-requests: read
    outputs:
      packaging: ${{ steps.paths.outputs.packaging }}
    env:
      GH_TOKEN: ${{ github.token }}
      PR: ${{ github.event.pull_request.number }}
      HEAD_SHA: ${{ github.event.pull_request.head.sha }}
    steps:
      - name: Copilot's verdict on this commit
        run: |
          states=$(gh api --paginate "repos/$GITHUB_REPOSITORY/pulls/$PR/reviews" \
            --jq ".[] | select(.user.login == \"github-actions[bot]\" and .commit_id == \"$HEAD_SHA\") | .state")
          state=$(tail -n 1 <<<"$states")
          echo "Copilot's latest review of $HEAD_SHA: ${state:-none}"
          if [ "$state" = CHANGES_REQUESTED ]; then
            echo "::error::Copilot requested changes, so the checks did not run. Push a fix, or dismiss the review and re-run this job."
            exit 1
          fi
      # The paths `packaging.yml` watches on main. Keep the two lists the same.
      - name: Does this touch packaging?
        id: paths
        run: |
          files=$(gh api --paginate "repos/$GITHUB_REPOSITORY/pulls/$PR/files" \
            --jq '.[] | .filename, (.previous_filename // empty)')
          watched='^(packaging/|\.cargo/|crates/tabletist-appkit/|(Cargo\.toml|Cargo\.lock|build\.rs|src/main\.rs|src/macos\.rs|\.github/workflows/(packaging|release)\.yml)$)'
          if grep -Eq "$watched" <<<"$files"; then
            echo "packaging=true" >> "$GITHUB_OUTPUT"
          else
            echo "packaging=false" >> "$GITHUB_OUTPUT"
          fi

  ci:
    needs: [agent, gate]
    if: ${{ !cancelled() && needs.gate.result == 'success' }}
    permissions:
      contents: read
    uses: ./.github/workflows/ci.yml

  packaging:
    needs: [agent, gate]
    if: ${{ !cancelled() && needs.gate.result == 'success' && needs.gate.outputs.packaging == 'true' }}
    permissions:
      contents: read
    uses: ./.github/workflows/packaging.yml
```

(c) In the prompt's `REQUEST_CHANGES` bullet, replace the last sentence

```
Name each blocking problem and what would resolve it.
```

with

```
No other check runs on a commit you request changes on, so name each blocking problem and what would resolve it.
```

Why each detail is there (do not "simplify" these away):
- `needs: [agent, ...]` on all three jobs. Without `agent` gh-aw treats a custom job as a prerequisite of the agent and the compile fails with `cycle detected in job dependencies`.
- `!cancelled()` on all three. With the default (`success()`), a skipped or failed agent would skip the gate and the checks, which is the opposite of decision 4. `cancelled()` still lets a newer push cancel the whole run.
- `needs.gate.result == 'success'` on the calls. This is what makes "requested changes" stop everything, and it also stops the checks when the gate itself breaks (an API error), rather than running them unreviewed by accident.
- The API result goes into a variable before `tail`. GitHub runs `run:` steps with `bash -e` but without `pipefail`, so `gh api ... | tail -n 1` would swallow an API failure and report "no review".
- `permissions: contents: read` on the calls. The compiled workflow's top-level permissions are `{}`, and a called workflow may not ask for more than its caller job grants. Both called workflows declare `contents: read`.
- No `strategy:` here. gh-aw custom jobs do not support it. The matrices live inside `ci.yml` and `packaging.yml` and are unaffected.
- The jobs inside `ci.yml` and `packaging.yml` have no `needs:` of their own, and that matters. A job in a called workflow that has its own `needs` falls back to `success()`, which is reported to look at the caller's skipped or failed ancestors too, so it would be skipped on every pull request without a review (not tested here). If one of them ever gains a `needs`, give it `if: ${{ !cancelled() }}` as well.

- [ ] **Step 5: Compile and lint**

Run: `gh aw compile pr-review --actionlint`
Expected:

```
✓ .github/workflows/pr-review.md (126.8 KB)
✓ Compiled 1 workflow: 1 succeeded, 0 warnings
✓ Checked 1 workflow(s)
✓ No issues found
```

Run: `~/.local/share/mise/installs/actionlint/1.7.12/actionlint .github/workflows/ci.yml .github/workflows/packaging.yml .github/workflows/release.yml; echo "exit $?"`
Expected: `exit 0` and nothing else.

- [ ] **Step 6: Check the job graph in the lock file**

Run: `grep -nE '^  (gate|ci|packaging):' -A8 .github/workflows/pr-review.lock.yml | grep -E '(needs|if|uses|name):|- (agent|gate|safe_outputs)$' | sed -E 's/^[0-9]+[-:]//'`
Expected:

```
    needs:
      - agent
      - gate
    if: ${{ !cancelled() && needs.gate.result == 'success' }}
    uses: ./.github/workflows/ci.yml
    name: review gate
    needs:
      - agent
      - safe_outputs
    if: ${{ !cancelled() }}
    needs:
      - agent
      - gate
    if: ${{ !cancelled() && needs.gate.result == 'success' && needs.gate.outputs.packaging == 'true' }}
    uses: ./.github/workflows/packaging.yml
```

Run: `grep -nE '^  pull_request' .github/workflows/*.yml | grep -v lock.yml`
Expected: no output. `pr-review.lock.yml` is now the only workflow with a `pull_request` trigger. (The word still appears in `ci.yml`'s concurrency group. That line stays.)

Run: `git status --short`
Expected, when Task 1 was committed:

```
 M .github/workflows/ci.yml
 M .github/workflows/packaging.yml
 M .github/workflows/pr-review.lock.yml
 M .github/workflows/pr-review.md
```

When the user has not asked for commits, Task 1's files are still untracked: the two `pr-review` files show as `??`, next to `.gitattributes` and `.github/aw/`. The two ` M` lines for `ci.yml` and `packaging.yml` are the same either way.

- [ ] **Step 7: Commit point**

```bash
git add .github/workflows/ci.yml .github/workflows/packaging.yml .github/workflows/pr-review.md .github/workflows/pr-review.lock.yml
git commit -S -m "Hold the checks until Copilot has reviewed

CI and packaging no longer start on pull_request themselves. The review
workflow calls them after a gate job, and the gate fails when Copilot's
review of the head commit requests changes, so nothing else runs on it.
Without a review (forks, Dependabot, an outage) the checks still run.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 3: Document the pipeline

**Files:**
- Modify: `AGENTS.md` (new section between "Checks" and "Style")

- [ ] **Step 1: Add the section**

In `AGENTS.md`, insert before the line `## Style`:

```markdown
## Pull requests

`.github/workflows/pr-review.md` is a GitHub Agentic Workflow (gh-aw) and the
only workflow that starts on a pull request. Copilot reviews each push first.
Then the `review gate` job starts `ci.yml`, and `packaging.yml` when packaging
paths changed. A review that requests changes fails the gate and nothing else
runs on that commit: push a fix, or dismiss the review on the pull request and
re-run the `review gate` job. Pull requests from forks and from Dependabot get
no review and go straight to the checks.

- After editing `pr-review.md` run `gh aw compile pr-review --actionlint`
  (`gh extension install github/gh-aw`) and commit `pr-review.lock.yml` with
  it. Never edit the lock file by hand.
- The review needs the `COPILOT_GITHUB_TOKEN` secret: a fine-grained PAT of a
  user account with the Copilot Requests permission. Without it the review
  fails and the checks still run.
- The paths that start `packaging.yml` are listed twice: in its `push` trigger
  and in the gate of `pr-review.md`. Change them together.
- The gate looks only at the review of the head commit. An older request for
  changes is dismissed once a later commit gets a review with no inline
  comments. Until then the pull request can still show "changes requested"
  while its checks run: dismiss the old review by hand.
- A job in `ci.yml` or `packaging.yml` that gains a `needs:` also needs
  `if: ${{ !cancelled() }}`, or it is skipped on pull requests without a
  review.

```

- [ ] **Step 2: Check it**

Run: `LC_ALL=C.UTF-8 grep -nP '\x{2014}' AGENTS.md; grep -n '^## ' AGENTS.md`
Expected: no em dash lines, and the headings in this order: `Architecture`, `Checks`, `Pull requests`, `Style`, `Releasing`.

- [ ] **Step 3: Commit point**

```bash
git add AGENTS.md
git commit -S -m "Describe the pull request pipeline in the agent guide

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 4: Prove it on a real pull request

Everything here touches GitHub. Do none of it until the user asks to push. The pull request that adds the workflow runs it (a `pull_request` run uses the workflow from the pull request itself), so this branch is its own test. It changes `.github/workflows/packaging.yml`, so it exercises the packaging call too.

- [ ] **Step 1: Is the secret there yet?**

Run: `gh secret list`
If `COPILOT_GITHUB_TOKEN` is not listed (the state when this plan was written), do not ask for it yet: the first run without it is the test of the no-review path in Step 3. If it is listed, skip Step 3. Never create or handle the token yourself.

- [ ] **Step 2: Push and open the pull request** (only when asked)

```bash
git push -u origin claude/copilot-pr-review-workflow-23159e
gh pr create --title "Review pull requests with Copilot before the checks" --body "<summary of the commits, the decisions list, and what was only verified locally>

🤖 Generated with [Claude Code](https://claude.com/claude-code)"
```

- [ ] **Step 3: The no-review path** (only when the secret was missing in Step 1)

Run: `gh run list --branch claude/copilot-pr-review-workflow-23159e --event pull_request`, then `gh run view <run id>` once it has finished.
Expected: the review did not happen (`activation` failed on the missing secret, or `agent` was skipped), `review gate` succeeded with the log line `Copilot's latest review of <sha>: none`, and `ci / *` and `packaging / *` ran. The run as a whole is red because of the failed review. If the gate or the checks were skipped instead, stop: decision 4 is broken, report it with the job list.

Then ask the user to add the secret (see "What the user has to do") and re-run everything: `gh run rerun <run id>`.

- [ ] **Step 4: Watch the order**

Run: `gh run list --branch claude/copilot-pr-review-workflow-23159e --event pull_request`
Expected: runs of the workflow `PR` only. No separate `CI` or `Packaging` run.

Run: `gh run view <run id>` once it has finished.
Expected, in this order of start times: `pre_activation`, `activation`, `agent`, `detection`, `safe_outputs`, `review gate`, and only then `ci / quality`, `ci / test (...)`, `ci / postgres integration`, `ci / mysql integration`, `ci / ssh integration` and the `packaging / ...` jobs, with `conclusion` last.

Copilot reads `AGENTS.md` from `main` (gh-aw restores it from the base branch), so its review of this pull request does not know the section Task 3 adds.

- [ ] **Step 5: Check the review and the gate's reading of it**

```bash
gh api "repos/igor-alexandrov/tabletist/pulls/<number>/reviews" --jq '.[] | "\(.user.login) \(.state) \(.commit_id[0:7])"'
gh run view <run id> --log --job <review gate job id> | grep "Copilot's latest review"
```

Expected: one review by `github-actions[bot]` on the head commit, and the gate's log line names the same state. If the login is anything else, the gate's filter must be changed to match (Task 2 Step 4, then recompile) before this merges.

- [ ] **Step 6: The blocked path (ask the user first)**

Whether Copilot requests changes is its judgement, so the blocked path needs a commit it will object to. With the user's agreement: push a throwaway commit to the pull request with an obvious defect (for example a log line that prints a connection URL with its password, which `AGENTS.md` forbids), and expect: a `CHANGES_REQUESTED` review, `review gate` failed with the "Copilot requested changes" error, `ci` and `packaging` skipped. Then drop the commit (`git revert`, not a force push) and expect a clean `COMMENTED` review, the earlier review dismissed, and the checks running. If Copilot does not block the planted defect, report that and discuss the prompt instead of forcing it.

- [ ] **Step 7: Report**

Report what ran, in what order, with the run link, and state plainly which of the "Not verified until the first real run" items are now confirmed and which are not.
