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
