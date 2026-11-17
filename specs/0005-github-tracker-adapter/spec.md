# SPEC-0005: GitHub tracker adapter and local cache

| | |
| --- | --- |
| Status | Draft |
| Milestone | M2 · Tracker and loop |
| Owner | @salvo8519 |
| Related | #13, #64, #65, #129 |
| Commit scope | tracker |

## Problem
The plan lives in GitHub issues (ADR 0002). The engine must read and write it reliably, quickly and without losing concurrent edits by people.

## Scope
**In scope:** Mapping of artifacts to issues, sub-issues, dependencies, issue types and milestones; structured block in issue bodies; incremental cache; rate limits; optimistic concurrency; dry run; GitLab as a later adapter behind the same interface.

**Out of scope:** Execution of work (SPEC-0006) and the content of gates (SPEC-0007).

## User stories
- As a team member, I want to see the plan in the issues we already use, so that I need no new tool.
- As a maintainer, I want my manual edits on an issue never to be overwritten by the engine.

## Requirements (EARS)
- **R1**: THE ENGINE SHALL map every epic to a parent issue, every story and every task to a sub-issue of its epic, and dependencies to native blocked-by relations.
- **R2**: THE ENGINE SHALL keep its own fields in a structured block of the issue body, validated by a schema.
- **R3**: IF the structured block does not validate, THEN THE ENGINE SHALL raise a question on the issue and leave it unchanged.
- **R4**: THE ENGINE SHALL maintain a local cache updated incrementally and rebuildable from the tracker at any time.
- **R5**: THE ENGINE SHALL write an issue only if it has not changed since the last read, otherwise re-read and propose again.
- **R6**: THE ENGINE SHALL group tracker calls and respect rate limits with backoff.
- **R7**: THE ENGINE SHALL expose one adapter interface so that GitLab can be added without changing the core.

## Acceptance criteria
- **AC1**: Given a recorded GitHub API session, when the cache is rebuilt, then it equals the incremental cache. Verification: contract test.
- **AC2**: Given an issue edited by a person between read and write, when the engine writes, then the write is refused and re-proposed. Verification: TDD.
- **AC3**: Given an invalid structured block, when synced, then a question is posted and nothing else changes. Verification: end-to-end.

## Interfaces
- Adapter trait in the runtime crate; wayloop issues sync with dry run.

## Open questions
- [ ] Use GitHub Projects fields in addition to issue fields, or issue fields only?
