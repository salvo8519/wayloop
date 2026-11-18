# SPEC-0008: Agent executors and minimal briefs

| | |
| --- | --- |
| Status | Draft |
| Milestone | M2 · Tracker and loop |
| Owner | @salvo8519 |
| Related | ADR 0011, ADR 0025, #63, #69, #70, #71, #97, #164, #165, #166, #173, #176 |
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
- **R8**: THE ENGINE SHALL reduce test output, build logs, JSON, YAML and CSV returned to an agent deterministically, keep the original on disk and name it in the reduced output.
- **R9**: THE ENGINE SHALL choose model and effort for each step from a routing table by step type and complexity score, allow an override per issue and record the choice.
- **R10**: THE ENGINE SHALL build briefs with a stable prefix (role, rules, project facts) before the variable part, so that provider prompt caches are reused.
- **R11**: THE ENGINE SHALL instruct implementers to prefer, in order: no new code, code already in the project, the standard library, an installed dependency, the minimum new code; validation, error handling, security and accessibility are never cut.
- **R12**: THE ENGINE SHALL ask agents for summaries, questions and hand-overs in controlled English: answer first, one idea per sentence, negations and numbers kept, code and paths verbatim, full sentences for security and irreversible actions.
- **R13**: THE ENGINE SHALL pass text written by the maintainer to agents unchanged.
- **R14**: THE ENGINE SHALL assign every review to an agent other than the implementing session: another configured vendor when one exists, otherwise a separate executor instance with a fresh context and the reviewer role.
- **R15**: THE ENGINE SHALL verify each review finding before a fix is attempted and record rejected findings with the reason.
- **R16**: THE ENGINE SHALL detect usage and rate limits from structured executor output first and text second, tell them apart from timeouts, wait for the reset in unattended mode and ask in attended mode.
- **R17**: WHERE the user has a Claude subscription, THE ENGINE SHALL default to the in-session mode and document how headless, SDK and API executors are billed.

## Acceptance criteria
- **AC1**: Given a fake executor, when the issue cycle runs, then every step receives a brief within budget. Verification: TDD.
- **AC2**: Given an invalid result, when returned, then an error event is recorded and the state is unchanged. Verification: TDD.
- **AC3**: Given Claude Code and Codex installed, when a demo issue runs, then both complete a step. Verification: end-to-end.
- **AC4**: Given a test output with three failures in ten thousand lines, when reduced, then the result names the three failures and the path of the original. Verification: property test.
- **AC5**: Given two briefs for the same role, when built, then their prefixes are byte-identical. Verification: TDD.
- **AC6**: Given an override for an issue, when a step runs, then the executor receives the overridden model and effort and the ledger records it. Verification: TDD.
- **AC7**: Given one configured vendor, when a review starts, then it runs in a separate executor instance with a fresh context. Verification: TDD.
- **AC8**: Given a structured rate-limit event in an unattended run, when processed, then the run waits until the reset and resumes the same step. Verification: TDD.

## Interfaces
- Executor trait; configuration executors.<role>.

## Open questions
- [x] Support provider batch interfaces for non-interactive steps in 1.0 or later? Decided by the maintainer on 2026-11-17: yes, in M2 (#245).
- [ ] Accept ADR 0025 (API executor) for 1.0 or move it after 1.0?
