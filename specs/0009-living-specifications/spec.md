# SPEC-0009: Living specifications and change deltas

| | |
| --- | --- |
| Status | Draft |
| Milestone | M3 · Living specifications and survey |
| Owner | @salvo8519 |
| Related | ADR 0014, #79, #82, #83, #84, #85 |
| Commit scope | specs |

## Problem
Specifications written once stop governing the code. The current behaviour must always be readable, and every change must be a reviewable delta merged at closure.

## Scope
**In scope:** Specifications by capability; change folders with proposal, design, delta and tasks; delta operations (added, modified, removed, renamed); consistency analysis; bounded clarification questions; requirement-quality checklists; convergence of code and specification; verification technique per task.

**Out of scope:** The survey of existing code (SPEC-0010).

## User stories
- As an analyst, I want to read what the system does today in one place, so that I can plan the next change.
- As a reviewer, I want to see only what a change adds, modifies or removes, so that review is fast.

## Requirements (EARS)
- **R1**: THE ENGINE SHALL keep one living specification per capability.
- **R2**: THE ENGINE SHALL represent every change as a delta of requirements with the operations added, modified, removed and renamed.
- **R3**: WHEN a change closes, THE ENGINE SHALL merge its delta into the living specification and archive the change.
- **R4**: THE ENGINE SHALL analyse a change for consistency between specification, plan and tasks without modifying them.
- **R5**: THE ENGINE SHALL ask at most a configured number of clarification questions and write the answers back into the specification.
- **R6**: THE ENGINE SHALL compare the code with the specification and append the missing work as tasks.
- **R7**: THE ENGINE SHALL require a verification technique for every task.

## Acceptance criteria
- **AC1**: Given a delta that modifies a missing requirement, when validated, then it fails with the requirement id. Verification: TDD.
- **AC2**: Given a closed change, when archived, then the living specification contains the merged requirements. Verification: TDD.
- **AC3**: Given a plan task without a requirement, when analysed, then it is reported. Verification: TDD.

## Interfaces
- wayloop spec new | analyze | clarify | converge | archive.

## Open questions
- [ ] Filtered reading of specifications by requirement id only, or also by tag?
