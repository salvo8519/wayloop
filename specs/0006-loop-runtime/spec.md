# SPEC-0006: Loop runtime: scheduler, workers, worktrees and budgets

| | |
| --- | --- |
| Status | Draft |
| Milestone | M2 · Tracker and loop |
| Owner | @salvo8519 |
| Related | ADR 0013, #61, #66, #67, #68, #76, #178 |
| Commit scope | runtime |

## Problem
Independent work should run in parallel to save time, without making the state non-deterministic or letting agents edit the same files.

## Scope
**In scope:** Asynchronous runtime around the pure core (ADR 0006); queue derived from the state; worker pool with a concurrency limit; one git worktree per issue (ADR 0013); lanes with disjoint paths; single ledger writer; token budgets per phase and per sprint; batching of tracker calls; in-session and unattended modes.

**Out of scope:** The agent executors (SPEC-0008).

## User stories
- As a maintainer, I want independent issues to progress at the same time while I am away, so that the project moves faster.
- As a maintainer, I want a hard cap on cost, so that an unattended run never surprises me.

## Requirements (EARS)
- **R1**: THE ENGINE SHALL derive the queue of ready work from the state, without an external broker.
- **R2**: THE ENGINE SHALL run at most the configured number of workers at once.
- **R3**: THE ENGINE SHALL give each running issue its own git worktree and remove it after merge, with approval.
- **R4**: THE ENGINE SHALL reject a sprint whose lanes declare overlapping paths.
- **R5**: THE ENGINE SHALL send every worker result to one writer that validates it and appends it to the ledger in order.
- **R6**: WHEN a budget of a phase or a sprint is exhausted, THE ENGINE SHALL stop starting new steps and report it.
- **R7**: THE ENGINE SHALL run both inside a session, triggered by a stop hook, and unattended, through wayloop run.
- **R8**: WHEN an attempt fails verification, THE ENGINE SHALL record a failure note (what was tried, what failed, the first error, what to try next), discard the branch and worktree, start the next attempt from the notes, and open a question after the configured number of attempts.

## Acceptance criteria
- **AC1**: Given three independent issues and a limit of two, when run, then never more than two workers are active. Verification: TDD.
- **AC2**: Given workers finishing in random order, when the ledger is replayed, then the state is the same for every order. Verification: property test.
- **AC3**: Given a budget of N tokens, when reached, then no new step starts and the stop reason is budget. Verification: TDD.
- **AC4**: Given a failed attempt, when the next attempt starts, then its brief contains the note and the previous branch no longer exists. Verification: TDD.

## Interfaces
- wayloop run [--unattended] [--max-workers N]; runtime crate built on an asynchronous executor.

## Open questions
- [ ] Default concurrency limit: three, as in the current process, or derived from the budget?
