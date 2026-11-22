# SPEC-0006: Loop runtime: scheduler, workers, worktrees and budgets

| | |
| --- | --- |
| Status | Draft |
| Milestone | M2 · Tracker and loop |
| Owner | @salvo8519 |
| Related | ADR 0013, #11, #61, #66, #67, #68, #76, #178, #309 |
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
- **R9**: WHEN a phase or a sprint starts, THE ENGINE SHALL reserve from its budget the cost of verifying the work it plans, and SHALL NOT spend the reserve on any other step.
- **R10**: IF the budget left cannot fund the reserve of the next step, THEN THE ENGINE SHALL start no new build step, verify the work already built, and stop with reason `budget`.
- **R11**: THE ENGINE SHALL count budgets and reserves in agent invocations, and SHALL size the verification reserve of an issue from its risk tier (SPEC-0002, Risk tier): `contract`, four invocations (three verifiers and one repetition, SPEC-0002 R8); `substantial`, one; `small`, none.
- **R12**: THE ENGINE SHALL record the tokens and the cost of every invocation for the cost report (SPEC-0004), without using them to stop the loop.

## Acceptance criteria
- **AC1**: Given three independent issues and a limit of two, when run, then never more than two workers are active. Verification: TDD.
- **AC2**: Given workers finishing in random order, when the ledger is replayed, then the state is the same for every order. Verification: property test.
- **AC3**: Given a budget of N tokens, when reached, then no new step starts and the stop reason is budget. Verification: TDD.
- **AC4**: Given a failed attempt, when the next attempt starts, then its brief contains the note and the previous branch no longer exists. Verification: TDD.
- **AC5** (R9): Given a budget and a plan of several issues, when the build steps exhaust their share, then the verification of every built issue still runs. Verification: TDD.
- **AC6** (R10): Given a budget that cannot fund the next reserve, when the next step is computed, then no build step starts and the stop reason is `budget`. Verification: TDD.
- **AC7** (R11): Given a plan of one issue of each tier, `contract`, `substantial` and `small`, when the phase starts, then the reserve is five invocations. Verification: TDD.
- **AC8** (R12): Given a run whose tokens exceed an expected count, when the next step is computed, then the loop does not stop for that reason and the tokens appear in the cost report. Verification: TDD.

## Interfaces
- wayloop run [--unattended] [--max-workers N]; runtime crate built on an asynchronous executor.

## Open questions
- [ ] Default concurrency limit: three, as in the current process, or derived from the budget?
