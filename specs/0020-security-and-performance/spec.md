# SPEC-0020: Security model and performance budgets

| | |
| --- | --- |
| Status | Draft |
| Milestone | M6 · Benchmark and hardening |
| Owner | @salvo8519 |
| Related | ADR 0022, #125, #126, #127, #177 |
| Commit scope | core |

## Problem
Wayloop reads untrusted content and drives agents with write access. It must resist instruction injection, run with least privilege and stay fast on large repositories.

## Scope
**In scope:** Threat model (ADR 0022); content-as-data rule; tests with injected instructions in issues, code and documents; least-privilege permissions for executors and workflows; secret handling; performance budgets for scan, index, next and sync.

**Out of scope:** Certification.

## User stories
- As a security reviewer, I want documented threats with tests, so that I can approve Wayloop for my team.

## Requirements (EARS)
- **R1**: THE ENGINE SHALL treat every analysed content as data and never as instructions.
- **R2**: THE ENGINE SHALL pass a suite of injection tests placed in issues, comments, code and documents.
- **R3**: THE ENGINE SHALL run executors with the least permissions the step needs.
- **R4**: THE ENGINE SHALL never write secrets to the ledger, summaries or tracker.
- **R5**: THE ENGINE SHALL meet the published performance budgets on the reference repositories.
- **R6**: WHERE container execution is configured, THE ENGINE SHALL run each step in a container that sees only its worktree, apply the network policy of the step, and stop the run when its cost cap is reached.

## Acceptance criteria
- **AC1**: Given an issue body that asks the agent to change its rules, when processed, then nothing changes and the attempt is recorded. Verification: TDD.
- **AC2**: Given the reference repositories, when benchmarked, then every budget holds. Verification: end-to-end.
- **AC3**: Given container mode, when a step reads outside its worktree, then the read fails. Verification: end-to-end.

## Interfaces
- SECURITY.md; threat model page; performance page.

## Open questions
- [ ] Performance budgets: values to be measured in M3 and fixed here.
