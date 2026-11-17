# Sprints

A sprint is a time-boxed set of GitHub issues with one goal. Sprint files in this folder are
the source of truth for agents working in parallel.

## Files

`sprints/SNN.yaml`, validated by `scripts/sprint.py`:

```yaml
id: S01
goal: Artifact model specified and validated
start: 2026-11-23
end: 2026-12-06
milestone: "M0 · Foundations"
status: planned            # planned | active | done
lanes:                     # lanes run in parallel; issues within a lane run in order
  - name: core
    issues: [3, 6]
    paths: ["crates/", "specs/"]
  - name: agents
    issues: [7]
    paths: [".agents/", "scripts/sync_agents.py"]
    generates: true
```

## Lane paths

Every lane declares `paths`, the files it may change, as glob patterns relative to the
repository root: `*` matches within one path segment, `**` any number of segments, `?` one
character, and a pattern that ends with `/` stands for the directory and everything below it.

`uv run python scripts/sprint.py validate sprints/SNN.yaml`, run by `uv run just check` and by
CI, rejects a sprint when:

- a lane has no `paths`;
- two lanes have patterns that some file could match both, naming the lanes and an example
  path;
- more than one lane has `generates: true`;
- a lane without `generates` covers a generated artifact (`.claude/`, the mirror of
  `.agents/`).

On a pull request from a sprint branch (`sprint/SNN/<issue>-<slug>`), CI runs
`scripts/sprint.py check-diff` and fails when a changed file is outside the paths of the
issue's lane. Every lane may also change `roadmap/roadmap.yaml`, `.agents/journal/` and
`changes/`. A file that a lane needs outside its paths is a planning change: update the sprint
file in its own pull request first.

## One worktree per session

One working tree has one branch, one HEAD and one index: two agent sessions in the same
directory overwrite each other's work. Parallel work runs in one git worktree per session,
created from an up-to-date `main`:

```bash
git fetch origin
git worktree add ../wayloop-<n> -b <type>/<n>-<slug> origin/main
```

The session lives in that directory for the whole issue and opens its pull request from it.
After the merge, the worktree is removed with `git worktree remove <path>`.

## Generated artifacts and the merge order

`.claude/` is generated from `.agents/` by `scripts/sync_agents.py`. At most one lane per
sprint changes `.agents/` and regenerates the mirror; its pull requests merge one at a time,
each rebased on `main` and regenerated before merging.

## Sprint state: resume mid-issue

The orchestrator writes its running table as it goes, one file per issue, in the git
directory shared by every worktree: `<git common dir>/wayloop/sprints/SNN/state/<issue>.yaml`.
These files are local runtime state, never committed. A context reset or a crash then costs
minutes: a new session reads the same files from `uv run python scripts/project_state.py` and
continues from the recorded step.

```bash
uv run python scripts/sprint.py state sprints/S01.yaml 42 \
    --branch feat/42-ledger-hash-chain --step tests \
    [--pr 33] [--blocker "waiting for the reviewer"] [--clear-blockers]
```

The step is one of `spec`, `tests`, `implementation`, `review`, `pr` or `done`.
`uv run python scripts/sprint.py status sprints/S01.yaml` prints each issue's local state next
to its state on GitHub.

## Workflow

1. **Plan**: the `planner` role proposes `sprints/SNN.yaml` from the open issues of the current
   milestone. The maintainer edits and approves it.
2. **Run**: the main session acts as orchestrator and delegates each issue, lane by lane.
   Every issue ends as a pull request, merged by the orchestrator under the rules of AGENTS.md
   (Merging): small ones when their gate holds, engine and contract changes after the
   maintainer confirms.
3. **Track**: `uv run python scripts/sprint.py status sprints/SNN.yaml`.
4. **Close**: the maintainer sets `status: done`; unfinished issues move to the next sprint.

This is the interim process of the repository, run with scripts until the Wayloop engine can
run it (milestone M1).
