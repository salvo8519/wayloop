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
- **R9**: WHEN a phase or a sprint starts, THE ENGINE SHALL set its verification reserve, in agent invocations, to the sum of the reserves of R11 of the work it plans, and SHALL spend those invocations only on the dispatches of `review`; the token budgets of its scopes are a separate limit (SPEC-0002 R29).
- **R10**: IF the verification reserve left in invocations (R11) cannot fund the reserve of the next issue to build, THEN THE ENGINE SHALL start no new build step, verify the work already built, and report the stop reason `budget_exhausted` of SPEC-0002 naming the phase or the sprint.
- **R11**: THE ENGINE SHALL count the verification reserve in agent invocations and the token budgets in tokens, as two separate limits with the units of SPEC-0002 section Limits and budgets, and SHALL size the reserve of an issue from its risk tier (SPEC-0002, Risk tier) to one complete review pass with its retries, with the formula of SPEC-0002 section Limits and budgets, Units, from the recorded `limits.executor_retries` (decided by the maintainer on 2026-11-22, SPEC-0002 Q13); with the default of 2, `contract` gives four invocations, `substantial` three and `small` none.
- **R12**: THE ENGINE SHALL record the tokens and the cost of every invocation for the cost report (SPEC-0004); WHEN the token budget of a scope of a step is exhausted, THE ENGINE SHALL start no new step in that scope, as SPEC-0002 R29 says, and SHALL NOT stop the steps of other scopes.

## Acceptance criteria
- **AC1**: Given three independent issues and a limit of two, when run, then never more than two workers are active. Verification: TDD.
- **AC2**: Given workers finishing in random order, when the ledger is replayed, then the state is the same for every order. Verification: property test.
- **AC3**: Given a budget of N tokens, when reached, then no new step starts and the stop reason is `budget_exhausted`. Verification: TDD.
- **AC4**: Given a failed attempt, when the next attempt starts, then its brief contains the note and the previous branch no longer exists. Verification: TDD.
- **AC5** (R9): Given a plan of several issues and its verification reserve, when the build steps use their invocations, then the dispatches of `review` of every built issue still run on the reserve, and no other step draws on it. Verification: TDD.
- **AC6** (R10): Given a verification reserve left that cannot fund the reserve of the next issue to build, when the next step is computed, then no build step starts, the review of the work already built is still proposed, and the stop reason is `budget_exhausted` naming the sprint. Verification: TDD.
- **AC7** (R11): Given a plan of one issue of each tier, `contract`, `substantial` and `small`, when the phase starts with the default `limits.executor_retries` of 2, then the reserve is seven invocations (4 + 3 + 0); given `limits.executor_retries` of 3, then it is eight (4 + 4 + 0). Verification: TDD.
- **AC8** (R12): Given no active sprint, the token budget of milestone M1 exhausted and a ready issue of milestone M2 whose budget is not, when the next step is computed, then no step of an issue of M1 is proposed and the stop reason is `budget_exhausted` naming M1, the claim of the issue of M2 is still proposed, and the tokens of every invocation appear in the cost report. Verification: TDD.

## Interfaces
- wayloop run [--unattended] [--max-workers N]; runtime crate built on an asynchronous executor.

## Open questions
- [ ] Default concurrency limit: three, as in the current process, or derived from the budget?
