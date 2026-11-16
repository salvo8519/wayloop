# The loop

The loop keeps the plan and the work aligned. It decomposes the project, runs the work and,
at every event, proposes how the plan should change. It never changes the plan on its own.

## Four movements

1. **Decompose.** From an idea, or from the survey of an existing codebase, the engine guides
   the descent through the levels of the project: milestones, epics, user stories,
   specifications, tasks and issues. Agents write each level; the engine validates it before
   the next level starts.
2. **Execute.** Issues move through the issue cycle below. Every step is recorded in the
   [ledger](ledger.md).
3. **Reconcile.** Every event produces a proposed change to the roadmap: an issue closed, a
   pull request merged, a new request from the maintainer, work discovered during
   development, a specification changed, a gap found by the survey. The proposal is a
   readable difference that a person approves or corrects.
4. **Re-plan.** After approval, the engine recomputes dependencies, next tasks, sprint lanes
   and the progress of each milestone, and updates the tracker.

## The issue cycle

| Step | What happens | Who |
| --- | --- | --- |
| Claim | The issue is assigned to one session, labelled in progress, given a branch and a worktree | Engine |
| Re-analysis | The issue, its story, its specification and the code involved are read again; the specification is checked against the current code | Agent |
| Counter-review | A second agent looks for gaps, edge cases and contradictions; each objection ends in a recorded ruling or a question | Agent |
| Checklist | The task checklist is generated from the specification and published on the issue | Agent, validated by the engine |
| Build | Tests where they pay, then the code | Agent |
| Verify | Deterministic checks decide whether the work is done | Engine |
| Review | Proportional to risk: none, one pass, or two | Agent |
| Pull request | Linked to the issue and to the requirements it implements | Engine |
| Merge | Approval state: a person decides | Maintainer |

## One turn of the loop

The engine owns the state machine; agents are workers called for one step at a time.

1. Read the state: tracker, repository and ledger.
2. Choose the next transition allowed by the state, with its dependencies satisfied.
3. Prepare a minimal brief: only the files and excerpts that the step needs, within the token
   budget of the phase.
4. Call an agent: Claude Code, Codex or another configured executor.
5. Verify with deterministic checks and record the outcome, the cost and the rulings.

A model never declares its own work finished: a check does.

## Where the loop stops

- **Approval states**: the council report, the roadmap, an approved specification, a merge.
- **Limits**: a maximum number of turns per issue and a token budget per phase and per sprint.
- **Repeated failure**: when the same check fails a configured number of times, the loop opens
  a question instead of trying again.

See [Gates](gates.md).
