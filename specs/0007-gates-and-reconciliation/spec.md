# SPEC-0007: Gates, questions and roadmap reconciliation

| | |
| --- | --- |
| Status | Draft |
| Milestone | M2 · Tracker and loop |
| Owner | @salvo8519 |
| Related | ADR 0012, #62, #72, #73, #74, #75, #167, #172, #175 |
| Commit scope | state |

## Problem
People decide at the gates, and the plan must follow reality. Questions must reach people where they work, and every event must turn into a proposed change of the plan, never a silent one.

## Scope
**In scope:** Approval states on the tracker (ADR 0012); the question format; answers and resumption; reconciliation of events into proposed roadmap changes; hand-over comments; the merge event that restarts the loop.

**Out of scope:** The council report gate content (SPEC-0012).

## User stories
- As a maintainer, I want questions in a fixed short format on the issue, so that I can answer from my phone.
- As a maintainer, I want every change to the plan proposed as a readable difference, so that I approve it in seconds.

## Requirements (EARS)
- **R1**: WHEN an agent needs a decision, THE ENGINE SHALL label the issue needs-decision and post context, options with cost and a recommendation.
- **R2**: WHEN an answer is posted by an authorised person, THE ENGINE SHALL record it and resume the issue from the waiting step.
- **R3**: WHEN an issue closes, a pull request merges, a request arrives or work is discovered, THE ENGINE SHALL produce a proposed roadmap change.
- **R4**: THE ENGINE SHALL never apply a roadmap change without approval.
- **R5**: THE ENGINE SHALL publish a hand-over comment at the end of every session, rendered from the validated summary.
- **R6**: THE ENGINE SHALL ignore events produced by its own bot except to create proposals, and group them.
- **R7**: WHEN a merge adds or removes code comments of the form `shortcut: <limit>, <when to revisit>`, THE ENGINE SHALL propose discovered work for the new ones and close the proposals of the removed ones.
- **R8**: WHEN a learning recorded in hand-overs is repeated or confirmed by a reviewer, THE ENGINE SHALL propose it as a pattern in the instruction files of the touched directories for every supported agent (AGENTS.md, CLAUDE.md and the others of SPEC-0017), refusing story-specific notes.
- **R9**: WHEN the loop opens a question, waits at a gate, finishes, fails or exhausts a budget, THE ENGINE SHALL send a notice in controlled English with the issue link to every configured channel.

## Acceptance criteria
- **AC1**: Given a question answered by a person without permission, when processed, then it is ignored and the reason recorded. Verification: TDD.
- **AC2**: Given a merged pull request that leaves a story incomplete, when reconciled, then a proposal adds the missing task. Verification: end-to-end.
- **AC3**: Given ten bot events in one minute, when processed, then one grouped proposal is created. Verification: TDD.
- **AC4**: Given a merge that adds two shortcuts, when reconciled, then one proposal lists both with file and line. Verification: TDD.
- **AC5**: Given two sessions recording the same learning for a directory, when reconciled, then one proposal adds it to the instruction files of that directory for every supported agent. Verification: TDD.
- **AC6**: Given a question opened in an unattended run, when processed, then one notice reaches each configured channel. Verification: TDD.

## Interfaces
- Labels needs-decision and approved; reusable GitHub workflow on pull request merge.

## Open questions
- [ ] Approvals by comment keyword, by reaction, or by label only?
