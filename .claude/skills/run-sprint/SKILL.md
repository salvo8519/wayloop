---
name: run-sprint
description: Runs an approved sprint as orchestrator, delegating each lane to agents in separate git worktrees, at most three at a time, and merging the resulting pull requests bottom up after a rebase onto main. Use when the maintainer asks to run, resume or close the active sprint.
---

<!-- Generated from .agents/skills/run-sprint/SKILL.md by scripts/sync_agents.py. Edit the source, not this file. -->

Talk to the maintainer in their preferred language (`CLAUDE.local.md`); everything written to the repository or to GitHub is in English, without icons.

## Produces
One pull request per issue of the sprint, each from its own worktree and branch `sprint/SNN/<issue>-<slug>`, reviewed by an agent other than its implementer; the local sprint state updated at every step; merges in dependency order when the gate of each tier holds; a report per lane.

## Steps
1. **Preconditions, deterministically.** Run
   `uv run python scripts/sprint.py validate sprints/SNN.yaml` (the file exists and has
   `status: active`), `uv run python scripts/sprint.py status sprints/SNN.yaml` (local state
   next to GitHub) and `uv run python scripts/project_state.py`. `main` must be green. A
   session that resumes reads the recorded step of each issue and continues from there.
2. **You are the orchestrator.** The main session delegates and keeps the overview; subagents
   cannot start other subagents. Run at most three lanes in parallel; issues within a lane run
   in order. Each brief names the issue, the specification, the lane's `paths` and the files
   to read.
3. **One worktree per lane issue.** Before an issue starts:
   `git fetch origin` and
   `git worktree add ../wayloop-<issue> -b sprint/SNN/<issue>-<slug> origin/main`. The lane
   agent works only in that directory and changes only files of the lane's `paths` (plus
   `roadmap/roadmap.yaml`, `.agents/journal/` and `changes/`). Only the lane with
   `generates: true` runs `uv run python scripts/sync_agents.py`.
4. **Per issue, in order**, recording each step with
   `uv run python scripts/sprint.py state sprints/SNN.yaml <issue> --branch <branch> --step <step>`
   (`--pr <n>`, `--blocker "<why>"` or `--clear-blockers` when they apply):
   - `spec`: engine behaviour without an `Approved` specification goes to the spec-writer
     role, then the lane stops until the maintainer approves.
   - `tests`: the test-engineer role, or the implementer when it is not available, writes the
     failing tests for the acceptance criteria.
   - `implementation`: the implementer role runs the `implement-spec` skill; documentation or
     tooling issues are implemented directly, one concern per commit.
   - `review`: the `pre-pr` skill, by tier, with a reviewer that did not write the code; the
     `apply-review` skill fixes every confirmed finding in the same branch.
   - `pr`: before pushing,
     `uv run python scripts/sprint.py check-diff --branch <branch> --base origin/main` passes;
     then push and open the pull request with `Closes #<issue>`.
5. **Verify each lane's claim.** An agent's report of success is not evidence: read
   `git -C ../wayloop-<issue> log --oneline origin/main..HEAD` and the diff, and apply the
   `verify` skill before marking a step done.
6. **Merge bottom up.** Order the pull requests by dependency; the lane that regenerates
   `.claude/` merges one pull request at a time. Before each merge: rebase the branch onto
   `origin/main` with its own commits only, regenerate the mirror if it is the generate lane,
   retarget it to `main`, wait for its checks, and confirm `roadmap.py check --pr-body` passes.
   Small pull requests are merged by the agent when the gate holds; those that change
   `crates/`, `schemas/` or an SDK wait for the maintainer (AGENTS.md, Merging). Squash-merge
   with the title and delete the branch; record `--step done`.
7. **Clean up.** Propose `git worktree remove ../wayloop-<issue>` for every merged issue; remove
   only after the maintainer agrees.
8. **Close.** Unfinished issues are listed for the next sprint. Setting `status: done` in the
   sprint file belongs to the maintainer.

## Stop and ask
- A lane needs a file outside its `paths`: that is a planning change, proposed as a separate pull request on the sprint file.
- A specification awaits approval, or a design choice is ambiguous: the lane stops, the others continue.
- A merge of `crates/`, `schemas/` or an SDK; a failing required check; an open review finding; a rebase conflict that touches another lane's work.
- A fourth parallel agent would be needed: queue it instead.

## Report
- Per lane: issues, branches, pull requests and their state, merges done and their order.
- Issues blocked and why; what the maintainer must approve or decide.
- What was not verified (for example CI not yet finished on a pull request) and the risks; worktrees left to remove.
