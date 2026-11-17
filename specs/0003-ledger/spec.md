# SPEC-0003: Ledger events, hash chain and replay

| | |
| --- | --- |
| Status | Draft |
| Milestone | M1 · Engine core |
| Owner | @salvo8519 |
| Related | #12, #50, #51, #52, #161, #162 |
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

## Acceptance criteria
- **AC1**: Given a ledger of 10,000 random events, when replayed twice, then both states are byte-identical. Verification: property test.
- **AC2**: Given a ledger with one edited line, when loaded, then the error names the line. Verification: TDD.
- **AC3**: Given a milestone with 200 sessions, when rolled up, then the milestone summary stays within its token limit. Verification: TDD.
- **AC4**: Given a merged pull request, when the issue summary is rolled up, then it states merged with commit and time. Verification: TDD.
- **AC5**: Given an expired lease, when the next step is computed, then the issue is ready and the expiry is recorded. Verification: property test.
- **AC6**: Given a hand-over written by one executor, when another executor checks out the issue, then its brief starts from the hand-over. Verification: TDD.

## Interfaces
- Ledger files under <git common dir>/wayloop/; wayloop history and wayloop rulings (SPEC-0004).

## Open questions
- [ ] Retention of archived events after a snapshot: keep, compress or delete?
