# SPEC-0003: Ledger events, hash chain and replay

| | |
| --- | --- |
| Status | Draft |
| Milestone | M1 · Engine core |
| Owner | @salvo8519 |
| Related | #12, #50, #51, #52, #161, #162, #309 |
| Commit scope | ledger |

## Problem
Sessions end, crash and hand over to each other. The state must survive them and be the same whoever reads it, and its history must show tampering.

## Scope
**In scope:** Event schema, hash chain, append-only writer, YAML projection, snapshots, roll-up of session, issue and milestone summaries, schema versions and migrations, location in the git common directory.

**Out of scope:** The search index (SPEC-0010 uses it) and the publication of summaries on issues (SPEC-0007).

## User stories
- As an agent arriving after weeks, I want a summary of known size, so that I resume without reading months of history.
- As a maintainer, I want to see every decision taken by agents with its reason, so that I can audit and undo it.

## Requirements (EARS)
- **R1**: THE ENGINE SHALL append every event to a JSON Lines ledger and never rewrite an existing line.
- **R2**: THE ENGINE SHALL store in each event the hash of the previous event.
- **R3**: WHEN the ledger is replayed from the start, THE ENGINE SHALL produce a state identical to the projection.
- **R4**: IF the hash chain is broken, THEN THE ENGINE SHALL stop and report the first broken event.
- **R5**: THE ENGINE SHALL record rulings as decision, reason and cost if wrong.
- **R6**: THE ENGINE SHALL produce validated JSON summaries per session, issue and milestone, each within a size limit in tokens.
- **R7**: THE ENGINE SHALL write snapshots and replay from the latest snapshot plus the following events.
- **R8**: THE ENGINE SHALL migrate events and summaries of older schema versions.
- **R9**: THE ENGINE SHALL record pull request events: opened, checks completed with the names of failing checks, review submitted, merged and closed without merge, each linked to its issue, branch and head commit.
- **R10**: THE ENGINE SHALL record a checkout of an issue with its holder, branch, worktree and lease expiry, and refuse a second checkout while the lease holds.
- **R11**: WHEN a lease expires without activity, THE ENGINE SHALL release the issue and record the expiry.
- **R12**: WHEN a holder ends a session, THE ENGINE SHALL record a hand-over with the step reached, files touched, what is not done, open questions, rulings and the next step, within a token budget.
- **R13**: WHEN an agent checks out an issue that has a hand-over, THE ENGINE SHALL start its brief from that hand-over, whatever agent wrote it.
- **R14**: THE ENGINE SHALL record one coverage state per coverage unit as events of the ledger, and derive the coverage of a requirement, a specification and a milestone from them.
- **R15**: THE ENGINE SHALL store with every verdict the commit it was produced on, a flag for a dirty working tree and the citations it rests on.
- **R16**: WHEN a merge changes a line cited by the evidence of a `covered` unit, THE ENGINE SHALL move that unit to `stale` and add it to the ready work.
- **R17**: IF a verdict was produced on a commit other than the head of the branch under verification, or on a dirty working tree, THEN THE ENGINE SHALL refuse it and record the reason.

## Acceptance criteria
- **AC1**: Given a ledger of 10,000 random events, when replayed twice, then both states are byte-identical. Verification: property test.
- **AC2**: Given a ledger with one edited line, when loaded, then the error names the line. Verification: TDD.
- **AC3**: Given a milestone with 200 sessions, when rolled up, then the milestone summary stays within its token limit. Verification: TDD.
- **AC4**: Given a merged pull request, when the issue summary is rolled up, then it states merged with commit and time. Verification: TDD.
- **AC5**: Given an expired lease, when the next step is computed, then the issue is ready and the expiry is recorded. Verification: property test.
- **AC6**: Given a hand-over written by one executor, when another executor checks out the issue, then its brief starts from the hand-over. Verification: TDD.
- **AC7** (R14): Given a ledger of coverage events, when replayed, then the coverage of each specification equals its `covered` units over its active units. Verification: property test.
- **AC8** (R16): Given a merge that edits a cited line, when reconciled, then the unit is `stale` and appears in the ready work. Verification: TDD.
- **AC9** (R17): Given a verdict carrying a commit older than the branch head, when submitted, then it is refused with reason `stale_commit`. Verification: TDD.

## Coverage and verification

Definitions shared by SPEC-0002, SPEC-0006 and SPEC-0009.

- **Coverage unit**: one acceptance criterion of an active requirement, identified by `<spec>/<requirement>/<criterion>`, for example `0009/R6/AC3`. It is the smallest thing the engine can declare covered.
- **Coverage state**: `planned`, `in_progress`, `covered`, `uncovered`, `needs_validation`, `stale`. Only the engine moves a unit between states.
- **Verdict**: the result of one verification of a unit: `covered`, `uncovered` or `needs_validation`. A `needs_validation` verdict names the exact fact that the repository cannot establish.
- **Lens**: the question a verifier answers about a unit: `presence` (is there code that implements it, cited by file and line), `behaviour` (does that code do what the criterion says, edge cases included), `proof` (does a test fail without that code and pass with it).
- **Evidence**: a verdict with its citations, the commit it was produced on and whether the working tree was dirty.

## Interfaces
- New event kinds `coverage_planned`, `coverage_verdict` and `coverage_stale` (additive, schema version 1).
- Ledger files under <git common dir>/wayloop/; wayloop history and wayloop rulings (SPEC-0004).

## Open questions
- [ ] Retention of archived events after a snapshot: keep, compress or delete?
