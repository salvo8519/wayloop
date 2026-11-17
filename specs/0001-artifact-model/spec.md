# SPEC-0001: Artifact model and schemas

| | |
| --- | --- |
| Status | Draft |
| Milestone | M0 · Foundations |
| Owner | @salvo8519 |
| Related | #3, #5, #6, #95, #160 |
| Commit scope | schemas |

## Problem
Every level of the loop must have a validated structure so that the engine can check it, link it to the other levels and refuse what is malformed before an agent builds on it.

## Scope
**In scope:** Brief, council report, milestone, epic, user story, living specification, change delta, task, hand-over summary: fields, identifiers, statuses, required links, and where each lives (repository or tracker).

**Out of scope:** The state machine (SPEC-0002) and the tracker mapping (SPEC-0005).

## User stories
- As a maintainer, I want every artifact to have a fixed structure, so that I review content and not formatting.
- As an agent, I want a schema for each artifact, so that my output is accepted or rejected with a precise reason.

## Requirements (EARS)
- **R1**: THE ENGINE SHALL define one versioned JSON Schema for each artifact type.
- **R2**: THE ENGINE SHALL give every artifact a stable identifier that survives renames and moves (ADR 0008).
- **R3**: WHEN an artifact is validated, THE ENGINE SHALL check its schema and every link it declares to other artifacts.
- **R4**: IF a link points to a missing or superseded artifact, THEN THE ENGINE SHALL report the field, the target and the fix.
- **R5**: THE ENGINE SHALL support the statuses Draft, Approved, Surveyed, Implementing, Done and Superseded where the artifact type allows them.
- **R6**: THE ENGINE SHALL accept artifacts written in the language of the project, with identifiers and keys in English.
- **R7**: THE ENGINE SHALL refuse an epic or task title with a type prefix, a bracket tag or a final full stop, and accept a title that starts with a command name.
- **R8**: THE ENGINE SHALL refuse two epic or task titles that are equal ignoring case within one project.
- **R9**: THE ENGINE SHALL refuse a task without a parent epic and a milestone, and a reference to a specification or requirement that does not exist.
- **R10**: WHEN a pull request is validated, THE ENGINE SHALL require that it closes exactly one planned task and that its branch names the same issue.
- **R11**: THE ENGINE SHALL validate and produce branch names `<type>/<issue>-<slug>` with at most five words in the slug, and commit headers `<type>(<scope>): <subject>` with an optional `[SPEC-NNNN Rk]` reference.

## Acceptance criteria
- **AC1**: Given one valid and one invalid example per artifact type, when validated, then only the invalid ones fail, each with its field. Verification: TDD.
- **AC2**: Given a story linked to a missing specification, when validated, then the error names the story, the field and the missing identifier. Verification: TDD.
- **AC3**: Given a schema change, when the schema version is not increased, then CI fails. Verification: contract test.
- **AC4**: Given titles with a prefix, a full stop or a duplicate, when validated, then each is reported with its pointer and fix. Verification: TDD.
- **AC5**: Given a task citing a missing requirement, when validated, then the error names the task, the field and the requirement. Verification: TDD.

## Interfaces
- Schemas under schemas/artifacts/<type>/v<N>.json; validation exposed by wayloop validate (SPEC-0004).

## Open questions
- [ ] Is the council report a repository file or an issue comment with an attachment?
