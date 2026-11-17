# Behavioural case: plan-sprint

## Input
Sprint S01 is done. Milestone M0 has five open issues: two in `area:core` that depend on each other, two in `area:agents`, one in `area:docs` blocked by an unapproved specification.

## Expected
`sprints/S02.yaml` with one goal and `status: planned`: a core lane with the two dependent issues in order, an agents lane with `generates: true`, and the blocked issue excluded with its reason. `uv run python scripts/sprint.py validate sprints/S02.yaml` passes. The proposal is shown to the maintainer, and `status: active` is set only after approval.
