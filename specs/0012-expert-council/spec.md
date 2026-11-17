# SPEC-0012: Expert council

| | |
| --- | --- |
| Status | Draft |
| Milestone | M4 · Upstream planning |
| Owner | @salvo8519 |
| Related | ADR 0016, #93, #96, #98 |
| Commit scope | agents |

## Problem
A new idea needs several points of view before it becomes a plan. A single context tends to anchor on its first opinion and to smooth over conflicts.

## Scope
**In scope:** Roles (product and business, software and AI architecture, design and UX, security and privacy, infrastructure and operations, quality, business committee, critic); independent first round; cross round on summaries; moderator report with decisions, rejected alternatives, dissent, risks and questions; one-mind and independent modes; roles configurable per domain.

**Out of scope:** The roadmap derived from the report (SPEC-0013).

## User stories
- As a project lead, I want conflicting expert opinions to reach me, so that I decide with the full picture.
- As a project lead, I want the council to cost little for small projects, so that I use it every time.

## Requirements (EARS)
- **R1**: THE ENGINE SHALL run the first round with each role reading only the brief.
- **R2**: THE ENGINE SHALL limit each opinion to a fixed format and a configured length.
- **R3**: THE ENGINE SHALL run the cross round on summaries, not on full opinions.
- **R4**: THE ENGINE SHALL produce a report with decisions, rejected alternatives, dissent, risks and open questions.
- **R5**: THE ENGINE SHALL leave unresolved conflicts to the maintainer.
- **R6**: THE ENGINE SHALL support a one-mind mode in a single context and an independent mode with one executor per role.

## Acceptance criteria
- **AC1**: Given two roles with opposite recommendations, when the report is produced, then the dissent is recorded and the question reaches the gate. Verification: TDD.
- **AC2**: Given the one-mind mode, when run on the reference brief, then the cost stays within the budget of the phase. Verification: benchmark.

## Interfaces
- wayloop council run [--mode one-mind|independent]; roles in .agents/agents/.

## Open questions
- [ ] Default set of roles for software projects: all eight or five?
