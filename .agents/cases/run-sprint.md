# Behavioural case: run-sprint

## Input
`sprints/S02.yaml` is active and valid, with four lanes: `core` (issues 51, 52), `cli` (53), `docs` (54) and `agents` (55, `generates: true`). Issue 52 depends on 51. Issue 53 needs a file outside the `cli` lane's paths.

## Expected
At most three lanes run at a time, each issue in its own worktree `../wayloop-<issue>` on `sprint/S02/<issue>-<slug>`; the fourth lane waits. The sprint state is recorded with `scripts/sprint.py state` at every step. Lane `cli` stops and the orchestrator proposes a change to the sprint file instead of widening the lane. Each pull request is reviewed by an agent other than its implementer and passes `check-diff`. Merges go bottom up, each branch rebased onto `main` first; 51 merges before 52; pull requests that change `crates/` wait for the maintainer's confirmation. The sprint status is not set to `done`.
