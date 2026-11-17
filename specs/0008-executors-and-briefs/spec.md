# SPEC-0008: Agent executors and minimal briefs

| | |
| --- | --- |
| Status | Draft |
| Milestone | M2 · Tracker and loop |
| Owner | @salvo8519 |
| Related | ADR 0011, ADR 0025, #63, #69, #70, #71, #97 |
| Commit scope | runtime |

## Problem
The engine never calls a model itself (ADR 0011). It must drive existing coding agents with briefs small enough to be cheap and complete enough to be correct.

## Scope
**In scope:** Executor interface; executors for Claude Code (headless and agent SDK), Codex and a generic command; minimal brief builder within budget; role selection; capture of output, cost and rulings; schema validation of results.

**Out of scope:** Model provider accounts and credentials: provided by the user.

## User stories
- As a maintainer, I want to choose which coding agent runs each role, so that I use the tools I already pay for.
- As a maintainer, I want each step to receive only what it needs, so that cost stays low and quality high.

## Requirements (EARS)
- **R1**: THE ENGINE SHALL call agents only through the executor interface.
- **R2**: THE ENGINE SHALL build a brief from the step, the files and the excerpts it needs, within the token budget of the phase.
- **R3**: THE ENGINE SHALL validate every executor result against the schema of the step before it enters the ledger.
- **R4**: THE ENGINE SHALL record the tokens and the duration reported by each executor.
- **R5**: THE ENGINE SHALL allow one executor per role, configurable per project.
- **R6**: WHERE the API executor is configured, THE ENGINE SHALL use it only for steps that do not edit code, with the Wayloop commands as its only tools (ADR 0025).
- **R7**: IF an executor fails or times out, THEN THE ENGINE SHALL record the failure and apply the retry policy of the step.

## Acceptance criteria
- **AC1**: Given a fake executor, when the issue cycle runs, then every step receives a brief within budget. Verification: TDD.
- **AC2**: Given an invalid result, when returned, then an error event is recorded and the state is unchanged. Verification: TDD.
- **AC3**: Given Claude Code and Codex installed, when a demo issue runs, then both complete a step. Verification: end-to-end.

## Interfaces
- Executor trait; configuration executors.<role>.

## Open questions
- [ ] Support provider batch interfaces for non-interactive steps in 1.0 or later?
- [ ] Accept ADR 0025 (API executor) for 1.0 or move it after 1.0?
