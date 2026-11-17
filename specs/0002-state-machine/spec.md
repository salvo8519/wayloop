# SPEC-0002: State machine of the project and of the issue cycle

| | |
| --- | --- |
| Status | Draft |
| Milestone | M1 · Engine core |
| Owner | @salvo8519 |
| Related | #11, #48, #49, #58 |
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

## Acceptance criteria
- **AC1**: Given every pair of state and event in the transition table, when applied, then the result matches the table. Verification: property test.
- **AC2**: Given an issue in the merge state, when an agent event arrives, then the state does not change and the refusal names the approval needed. Verification: TDD.
- **AC3**: Given a budget of N turns, when N verifications fail, then the issue is blocked with reason limit. Verification: TDD.

## Interfaces
- wayloop next and wayloop status (SPEC-0004) return the ready work and the stop reasons as JSON.

## Open questions
- [ ] Is the counter-review step mandatory for the smallest risk tier?
