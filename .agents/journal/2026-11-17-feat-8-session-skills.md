# 2026-11-17 feat/8-session-skills

- Issue: #8 Session-start and session-end skills
- Pull request: this branch (ready)
- Spec: none (process); product counterpart SPEC-0003 R10 to R13

## Checkout
- Holder: Claude Code (Anthropic), session of 2026-11-17
- Branch: feat/8-session-skills, worktree none
- Step reached: pull request
- Released: yes, with this hand-over

## Done
- Merged today: #14, #16, #135, #147 to #156 (process parity), #169 and #187 (plans), #170 (traceability rules, roadmap.py links), #188 (merge rule), #189 (Claude Code mirrors).
- This branch: session-start and session-end skills, journal template with Checkout, Not done and Learnings, scripts/journal.py with tests.

## Not done
- Repository settings that the agent cannot change: dependency graph (Dependency review fails until enabled), ruleset Protect main, secret ANTHROPIC_API_KEY, automatic deletion of merged branches, GitHub Pages (#1).
- Merged branches still on GitHub; delete them from the branches page.
- Other skills of the project template (pull request triage, pre-PR check, commit, ADR, specification, sprint planning, translation, CI fixes) are not planned yet.

## Decisions taken
- The agent merges its own pull requests; engine, adapter and contract changes wait for the maintainer (maintainer, 2026-11-17; AGENTS.md, Merging).
- Process policy of the project template becomes a capability of the engine (ADR 0026, Proposed).

## Learnings
- Stacked pull requests: rebase each branch onto main with its own commits only before merging, or the roadmap file conflicts on adjacent status lines.
- Squash merges of single-commit pull requests take the commit title unless commit_title is passed; pass the pull request title.

## Open questions for the maintainer
- ADR 0009, 0010 and 0024 (#46); ADR 0026 (#171 merged as Proposed); GitHub Projects board (#146).

## Next steps
1. Plan the remaining skills of the project template as tasks of the Agent process epic, then start M1 with #46 once the maintainer decides it.
