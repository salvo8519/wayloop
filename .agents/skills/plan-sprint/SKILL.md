---
name: plan-sprint
description: Plans the next sprint from the open issues of the current milestone into sprints/SNN.yaml, with one goal and lanes of disjoint paths that agents can run in parallel, validated by scripts/sprint.py. Use when the maintainer asks to plan or prepare a sprint, or when the active sprint is closed.
---

Talk to the maintainer in their preferred language (`CLAUDE.local.md`); everything written to the repository or to GitHub is in English, without icons.

The rules of sprint files, lanes and paths are in `sprints/README.md`. A sprint only orders
work that is already in the roadmap; new work goes through the `roadmap-update` skill first.

## Produces
`sprints/SNN.yaml` with a goal, dates, milestone, `status: planned` and lanes whose paths no
file can match twice; `scripts/sprint.py validate` passing; after the maintainer approves,
`status: active` committed in its own pull request.

## Steps
1. **State, deterministically.** Run `uv run python scripts/project_state.py`,
   `uv run python scripts/roadmap.py sync` and `uv run python scripts/roadmap.py links`.
   Report drift with the command that fixes it; do not plan on top of unresolved drift.
2. **Previous sprint.** `ls sprints/` and read the latest `sprints/SNN.yaml`. If it is still
   `active`, run `uv run python scripts/sprint.py status sprints/SNN.yaml` and list its
   unfinished issues: they are the first candidates for the new sprint. Closing a sprint
   (`status: done`) is the maintainer's decision.
3. **Candidates.** `uv run python scripts/roadmap.py list --open --milestone M<n>` for the
   current milestone. For each candidate read the issue: its `## Depends on`, its `area:*`
   label, whether it needs an approved specification (`specs/NNNN-*/spec.md` status) and
   whether it is `in-progress` under someone else's lease. Exclude blocked issues and issues
   whose dependencies are not done or planned earlier in the same lane.
4. **Draft** with the `planner` role (`.agents/agents/planner.md`), passing the candidates,
   the maintainer's priorities and the previous sprint. Start from `sprints/_template.yaml`:
   - `id` the next `SNN`; one-sentence `goal` stating the outcome; `start` and `end`;
     `milestone` exactly as in the roadmap;
   - lanes by area and by files: issues within a lane run in order, lanes run in parallel;
   - `paths` per lane as narrow as the issues allow, glob patterns as defined in the README;
   - at most one lane with `generates: true`, the one that changes `.agents/` and regenerates
     `.claude/`;
   - at most three lanes, the limit of agents running in parallel (AGENTS.md).
5. **Validate.** `uv run python scripts/sprint.py validate sprints/SNN.yaml` must pass. On an
   overlap, narrow the paths or merge the two lanes; never drop the check.
6. **Propose** to the maintainer: goal, lanes with issues in order and paths, the issues
   excluded and why, issues that still need a specification, and the merge order where lanes
   depend on the generated mirror. Apply their changes and validate again.
7. **Approve.** Only on the maintainer's approval set `status: active`. Commit on a branch for
   the planning issue with `chore(project): plan sprint SNN`, signed off, and open its pull
   request. A later change of lane paths is again a planning change in its own pull request.
8. **Hand over.** Issues of the sprint are worked on branches `sprint/SNN/<issue>-<slug>`, one
   worktree each; the orchestrator records progress with `scripts/sprint.py state`.

## Stop and ask
- Before step 7, always: the sprint content and its goal are the maintainer's decision.
- A candidate needs a specification that is not approved, or a structural decision without an
  ADR: ask whether to include the specification work instead.
- An issue under a valid lease of another holder; a dependency that crosses lanes.
- A needed issue is missing from the roadmap: use `roadmap-update`, do not invent it here.

## Report
- Done: sprint file and goal, lanes with issues and paths, validation result, pull request.
- Not done or not verified: excluded issues and reasons, GitHub state not reachable, approval
  pending.
- Risks: dependencies across lanes, wide paths, the order of merges of the generating lane.
