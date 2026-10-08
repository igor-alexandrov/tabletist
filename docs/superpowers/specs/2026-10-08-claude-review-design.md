# Claude reviews pull requests

Date: 2026-10-08. Status: approved in conversation.

## Intent

Every pull request gets a review that knows this project's rules, in place of
Copilot's generic one. The review is advice. It never approves, never requests
changes, and no check waits for it (the gate of `df74dce` was tried and
removed in `557a566`).

## Decisions

1. **Own branches automatically, forks on demand.** A fork gets no secrets on
   `pull_request`, and `pull_request_target` would hand a secret to a job that
   reads text nobody has looked at. So a fork is reviewed when the owner
   comments `@claude review <sha>`.
2. **Claude subscription token**, `CLAUDE_CODE_OAUTH_TOKEN`, from
   `claude setup-token`. Reviews draw on the owner's plan.
3. **One reviewer with a prompt written for this repository**, not the
   multi-agent code-review plugin. It reads `AGENTS.md` and checks its rules,
   then correctness, security, tests and what has to move together.
4. **Copilot's review goes.** `copilot-review.yml` is deleted. The ruleset
   "Copilot review for default branch" and the `COPILOT_REVIEW_TOKEN` secret
   are repository settings, removed by the owner.

## The workflow

`.github/workflows/claude-review.yml`, one job, `anthropics/claude-code-action@v1`.

| Event | Reviewed when |
|---|---|
| `pull_request` (opened, reopened, ready_for_review, synchronize) | the head is a branch of this repository and the pull request is not a draft |
| `issue_comment` (created) | on a pull request, by the repository owner, starting with `@claude review` |

Steps:

1. Without the secret: a notice, and the job passes.
2. **Pull request.** Decides the commit to review. On `pull_request` it is the
   event's head. On a comment it is the current head, which for a fork must
   begin with the sha the comment names. Otherwise the job comments why and
   fails.
3. The base branch is checked out at the workspace root, the pull request's
   head in `pr-head/` with its history.
4. **What Claude reads.** Checks that `pr-head` is the commit decided in step
   2, then writes `.review/`: the title and description, the list of changed
   files, the diff from the merge base (without `Cargo.lock` and
   `docs/superpowers/`), and the inline comments earlier reviews left.
5. **Review.** Claude reads `AGENTS.md`, the diff and the code around it, and
   posts inline comments marked Blocking or Minor. It returns a summary as
   structured output.
6. **Summary.** A step posts the summary, or rewrites the one an earlier
   review posted (found by a marker).

## Prompt injection

It cannot be prevented, so the job is built to leave an injected instruction
nothing to do.

- **What the job holds.** `GITHUB_TOKEN` with `contents: read` and
  `pull-requests: write`, and `CLAUDE_CODE_OAUTH_TOKEN`. Nothing else.
  Checkouts do not persist credentials.
- **What Claude can do.** `Read`, `Grep` and `Glob` inside the workspace, and
  the inline comment tool. No shell, no network tool, no file writes. It does
  not fetch the diff or post the summary: steps do. Deny rules also name
  `/proc`, the runner's temporary directory and `.git`.
- **What reaches Claude.** A fork's text only after the owner named its
  commit. The rules come from `AGENTS.md` on the base branch.
- **What trusts the output.** Nothing. The summary is posted as a comment and
  parsed by no script beyond taking its text.

What remains: a misleading review of a fork, usage spent on the owner's plan,
and, if the read limits were bypassed, the subscription token in a public
comment.

## Verification

- `actionlint` 1.7.12 (with shellcheck) on the workflow.
- The tool limits were tried with a local headless `claude` 2.1.293 run using
  the workflow's flags: reads outside the workspace, in `.git`, a `Grep`
  outside the workspace and `Bash` were all refused, and the summary came
  back as structured output. A bare `Grep` in `--allowedTools` did search
  outside the workspace, which is why the three tools are written
  `Read(./**)`, `Grep(./**)`, `Glob(./**)`.
- The `pull_request` path runs on this change's own pull request once the
  secret exists. The comment path runs the default branch's workflow, so it
  can be tried only after the merge.

## Out of scope

- Reviewing `docs/superpowers/` and `Cargo.lock`.
- Building or testing in the review job. `ci.yml` does both.
- Any gate on the review's outcome.
