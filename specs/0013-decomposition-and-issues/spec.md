# SPEC-0013: Decomposition, complexity and issue generation

| | |
| --- | --- |
| Status | Draft |
| Milestone | M4 · Upstream planning |
| Owner | @salvo8519 |
| Related | ADR 0017, #94, #99, #100, #101, #102 |
| Commit scope | tracker |

## Problem
From a plan to work that agents can take: stories, specifications and tasks must be generated, sized and turned into issues without duplicates.

## Scope
**In scope:** Roadmap from the council report; stories with acceptance criteria; tasks with dependencies, lanes and verification techniques; complexity score and expansion above a threshold; next task; idempotent issue generation by stable identifiers; two-way status.

**Out of scope:** Execution of the generated issues (M2).

## User stories
- As a maintainer, I want to regenerate the plan after a change without duplicate issues, so that the tracker stays clean.
- As an agent, I want tasks small enough to finish in one session, so that work is never half done.

## Requirements (EARS)
- **R1**: THE ENGINE SHALL generate milestones and epics from an approved council report as a proposal.
- **R2**: THE ENGINE SHALL generate stories with acceptance criteria from each epic as a proposal.
- **R3**: THE ENGINE SHALL score the complexity of each task from 1 to 10 and expand tasks above the configured threshold into subtasks.
- **R4**: THE ENGINE SHALL generate issues idempotently: the same identifier updates the same issue (ADR 0017).
- **R5**: THE ENGINE SHALL turn each story and each task into a sub-issue of its epic, whatever the complexity of the task; a task above the threshold is expanded into subtasks first (R3).
- **R6**: THE ENGINE SHALL keep status consistent in both directions between files and tracker.

## Acceptance criteria
- **AC1**: Given a plan generated twice, when synced, then no duplicate issue exists. Verification: end-to-end.
- **AC2**: Given a task scored 8 with threshold 6, when planned, then it is expanded into subtasks. Verification: TDD.

## Interfaces
- wayloop plan roadmap | stories | tasks; wayloop issues generate [--dry-run].

## Open questions
- [ ] Complexity scored by an agent with a rubric, or by a deterministic heuristic first?
