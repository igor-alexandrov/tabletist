---
name: PR
description: Copilot reviews every pull request before anything else runs. The checks start only when it has not requested changes.
run-name: "PR #${{ github.event.pull_request.number }}"

on:
  pull_request:
    types: [opened, synchronize, reopened]

permissions:
  contents: read
  pull-requests: read

engine: copilot

# `cargo` on the runner is rustup: it fetches the toolchain `rust-toolchain.toml`
# pins, then the crates. egui, winit and fastframe are git dependencies on GitHub.
network:
  allowed:
    - defaults
    - rust
    - "github.com"

safe-outputs:
  create-pull-request-review-comment:
    max: 10
  submit-pull-request-review:
    max: 1
    allowed-events: [COMMENT, REQUEST_CHANGES]
    # A clean review of a later commit dismisses the older blocking ones.
    supersede-older-reviews: true

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
---

# Pull Request Review Assistant

Review the diff of pull request #${{ github.event.pull_request.number }} for correctness, security, maintainability, and test coverage. Read `AGENTS.md` first and hold the change to the rules it sets for this repository.

Create inline review comments only for specific problems or concrete improvements. Do not restate unchanged code or provide style-only feedback.

Finish by submitting exactly one review. Its body is the summary: group the findings by severity and note anything that needs human follow-up.

- Submit it as `REQUEST_CHANGES` only when the change has a problem that must be fixed before merging: a bug, a security problem, data loss, a broken build on Linux, macOS or Windows, or a behaviour change without a regression test. No other check runs on a commit you request changes on, so name each blocking problem and what would resolve it.
- Otherwise submit it as `COMMENT`. When you found nothing worth raising, say so in one sentence and add no inline comments.
