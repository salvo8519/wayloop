# SPEC-0002: State machine of the project and of the issue cycle

| | |
| --- | --- |
| Status | Draft |
| Milestone | M1 · Engine core |
| Owner | @salvo8519 |
| Related | #11, #48, #49, #58, #309 |
| Commit scope | state |

## Problem
The engine decides every step of the loop. Without an explicit state machine the decision moves into prompts, where it cannot be tested, replayed or audited.

## Scope
**In scope:** States and transitions of a project (brief to closure) and of an issue (claim to merge); guards; approval states; limits on turns, budget and repeated failure; the next-step function.

**Out of scope:** Persistence of events (SPEC-0003) and execution of steps (SPEC-0006, SPEC-0008).

## User stories
- As a maintainer, I want the loop to stop exactly at the gates I own, so that nothing irreversible happens without me.
- As an agent, I want the engine to tell me the next allowed step, so that I never decide the process myself.

## Requirements (EARS)
- **R1**: THE ENGINE SHALL compute the next state as a pure function of the current state and one event.
- **R2**: THE ENGINE SHALL refuse a transition whose guard is not satisfied and return the unmet condition.
- **R3**: THE ENGINE SHALL never leave an approval state without an approval event from an authorised person.
- **R4**: WHEN the number of turns of an issue reaches its limit, THE ENGINE SHALL move the issue to blocked with the reason.
- **R5**: WHEN the same verification fails the configured number of times, THE ENGINE SHALL open a question instead of retrying.
- **R6**: THE ENGINE SHALL list the ready work: issues whose dependencies are done and whose lane is free.
- **R7**: THE ENGINE SHALL distinguish the stop reasons: approval, limit, repeated failure, irreversible action, security-sensitive action, broken plan.
- **R8**: WHERE the risk tier of an issue is high, THE ENGINE SHALL run the review step as a panel of three verifiers, one per lens (SPEC-0003, Coverage and verification), none of which wrote or counter-reviewed the change, each instructed that a coverage unit is `uncovered` unless it can cite the code and the test that cover it, and SHALL allow one repetition of the panel.
- **R9**: THE ENGINE SHALL compute the outcome of a panel from the votes alone: `covered` only when every lens votes `covered`; `needs_validation` when no lens votes `uncovered` and at least one votes `needs_validation`; `uncovered` otherwise. No model computes or reports the outcome.
- **R10**: IF a verifier returns a vote without a citation for each claim, THEN THE ENGINE SHALL count it as `uncovered` and record the reason `uncited`.
- **R11**: WHERE the risk tier of an issue is medium, THE ENGINE SHALL run the review step with one verifier on the `proof` lens under R9 and R10; WHERE it is low, THE ENGINE SHALL rely on the deterministic checks of the verify step alone.

## Acceptance criteria
- **AC1**: Given every pair of state and event in the transition table, when applied, then the result matches the table. Verification: property test.
- **AC2**: Given an issue in the merge state, when an agent event arrives, then the state does not change and the refusal names the approval needed. Verification: TDD.
- **AC3**: Given a budget of N turns, when N verifications fail, then the issue is blocked with reason limit. Verification: TDD.
- **AC4** (R8): Given a high-risk issue, when the review step starts, then three verifiers are dispatched with distinct lenses and none of them is the implementer or the counter-reviewer. Verification: TDD.
- **AC5** (R9): Given every combination of three votes, when tallied, then the outcome matches the rule of R9. Verification: property test.
- **AC6** (R10): Given a `covered` vote with no citation, when tallied, then it counts as `uncovered` with reason `uncited`. Verification: TDD.
- **AC7** (R11): Given a medium-risk issue, when the review step starts, then exactly one verifier is dispatched with the `proof` lens; given a low-risk issue, then none is. Verification: TDD.

## Interfaces
- wayloop next and wayloop status (SPEC-0004) return the ready work and the stop reasons as JSON.

## Open questions
- [ ] Is the counter-review step mandatory for the smallest risk tier?
