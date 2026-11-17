# SPEC-0001: Artifact model and schemas

| | |
| --- | --- |
| Status | Draft |
| Milestone | M0 · Foundations |
| Owner | @salvo8519 |
| Related | #3, #4, #5, #6, #95, #160; ADR 0002, 0003, 0007, 0008, 0009, 0014, 0016, 0017; SPEC-0002, SPEC-0003, SPEC-0004, SPEC-0005, SPEC-0009, SPEC-0012, SPEC-0013, SPEC-0021 |
| Commit scope | schemas |

## Problem
Every level of the loop must have a validated structure so that the engine can check it, link it to the other levels and refuse what is malformed before an agent builds on it.

## Scope
**In scope:** Brief, council report, milestone, epic, user story, living specification, change delta, task, hand-over record: fields, identifiers, statuses and allowed transitions, required links, and where each lives (repository or tracker). Naming rules of titles, branches, commits and pull requests (R7 to R11), reused by SPEC-0021.

**Out of scope:** The state machine (SPEC-0002): triggers, guards and actors of each transition. The tracker mapping (SPEC-0005): how fields map to issue fields, labels and the structured block. The content rules of hand-overs (SPEC-0003 R12, R13). Delta merge (SPEC-0009). Generation of artifacts from other artifacts (SPEC-0012, SPEC-0013).

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
- **R12**: WHEN an artifact stored as Markdown or as a tracker item is validated, THE ENGINE SHALL normalise it into the document model of its type before applying the schema, and report every error with a JSON pointer into the document model and the source location (file and line, or issue number).
- **R13**: IF a status change is not listed among the allowed transitions of the artifact type, THEN THE ENGINE SHALL refuse it with the reason `state` and the allowed target statuses in `expected`.
- **R14**: THE ENGINE SHALL refuse an identifier that does not match the format of its artifact type, and an identifier already assigned to another artifact of the project.
- **R15**: IF a task or a user story declares a milestone or an epic that differs from the one reached through its parent, THEN THE ENGINE SHALL refuse it and name both values.

## Acceptance criteria
- **AC1** (R1, R3, R5, R6, R9, R12, R14): Given one valid and one invalid example per artifact type, when validated, then only the invalid ones fail, each with its field. Verification: TDD, with the files of `examples/` as fixtures.
- **AC2** (R3, R4): Given a story linked to a missing specification, when validated, then the error names the story, the field and the missing identifier. Verification: TDD.
- **AC3** (R1): Given a schema change, when the schema version is not increased, then CI fails. Verification: contract test.
- **AC4** (R7, R8): Given titles with a prefix, a full stop or a duplicate, when validated, then each is reported with its pointer and fix. Verification: TDD.
- **AC5** (R9): Given a task citing a missing requirement, when validated, then the error names the task, the field and the requirement. Verification: TDD.
- **AC6** (R2, R14): Given a living specification moved to another folder and renamed, when the artifacts that link to it are validated, then every link still resolves; given two artifacts with the same identifier, then the second is refused with the reason `duplicate`. Verification: TDD.
- **AC7** (R4): Given a story linked to a superseded specification, when validated, then the error names the field, the superseded identifier and, as the fix, the identifier in its `superseded_by`. Verification: TDD.
- **AC8** (R5, R13): Given a task in status `draft`, when it is moved to `done`, then it is refused with the reason `state` and `expected` lists `approved` and `superseded`; given a brief with status `implementing`, then it is refused with the reason `enum`. Verification: TDD.
- **AC9** (R6): Given a story whose prose is in Italian and whose keys are in English, when validated, then it passes; given the key `titolo` instead of `title`, then it is refused with the reason `unknown_field` and the fix `title`. Verification: TDD.
- **AC10** (R7): Given the task title `wayloop init creates a project structure that validates`, when validated, then it passes. Verification: TDD.
- **AC11** (R10): Given a pull request that closes two planned tasks, or whose branch names an issue other than the one it closes, when validated, then it is refused with the pointer to the body or to the branch. Verification: TDD.
- **AC12** (R11): Given any issue number, type and title, when a branch name is produced, then it matches `<type>/<issue>-<slug>`, the slug has at most five words, and the produced name passes validation; given a commit header without a scope, then it is refused. Verification: property test for production, TDD for validation.
- **AC13** (R12): Given a living specification in Markdown with a requirement missing its identifier, when validated, then the error carries the pointer `/requirements/1/id` and the file and line of the requirement. Verification: TDD.
- **AC14** (R15): Given a task whose milestone is M2 under an epic of M1, when validated, then it is refused and the error names M2 and M1. Verification: TDD.

## Artifact model

### Common rules

Every artifact, whatever its storage, is validated as one document of the model below (R12).
Markdown files and tracker items are first normalised into this document; the examples in
`examples/` show the normalised form. Keys are English and in `snake_case`; enum values are
`snake_case`; dates follow ISO 8601 (ADR 0009 C4). Prose fields are in the language of the
project (R6).

Fields common to every type:

| Name | Type | Required | Meaning |
| --- | --- | --- | --- |
| `kind` | enum: `brief`, `council_report`, `milestone`, `epic`, `story`, `specification`, `change`, `task`, `handover` | yes | Artifact type; selects the schema |
| `schema_version` | integer, at least 1 | yes | Version of the schema of the type the document was written against (R1) |
| `id` | string, format of the type | yes | Stable identifier (R2, R14) |
| `title` | string, 1 to 120 characters | yes, except `handover` | Outcome or name, in sentence case without a final full stop |
| `status` | enum, values allowed by the type | yes, except `handover` | Current status (R5) |
| `language` | BCP 47 tag | no | Language of the prose; when absent, the language of the project, listed in `meta.defaults_applied` (ADR 0009 C2) |
| `superseded_by` | identifier of the same type | when `status` is `superseded` | Replacement, used as the fix of R4 |

Statuses in Markdown headers keep their capitalised form (`Draft`); the document model uses
the lower-case value (`draft`).

### Identifiers

The rules below are those of ADR 0008, which is Proposed; its acceptance (#4) is a dependency
of this specification (see Dependencies and Q2).

- An identifier is a type prefix and a sequence: four digits or more, zero-padded, per type
  and per project, for example `STORY-0042`, `TASK-0107`.
- It is assigned at creation, is independent of the file name, the path and the title, and is
  never reused, also after the artifact is superseded or deleted.
- Parts of an artifact are cited as `<id> <part>`: requirements `SPEC-0003 R2`, acceptance
  criteria of a specification `SPEC-0003 AC1`, of a story `STORY-0042 AC2`, decisions of a
  council report `COUNCIL-0001 D3`, questions `COUNCIL-0001 Q1`. Part numbers are never reused
  within their artifact.
- Links are checked by identifier, never by path (ADR 0008).
- Tracker items also carry their tracker number (`issue`) once created. The tracker number is
  what branches, commits and pull requests cite (conventions page); the artifact identifier is
  what generation uses to find the same issue again (ADR 0017).

| Type | Prefix and format | Example |
| --- | --- | --- |
| Brief | `BRIEF-NNNN` | `BRIEF-0001` |
| Council report | `COUNCIL-NNNN` | `COUNCIL-0001` |
| Milestone | `M<n>`, no padding (conventions page) | `M1` |
| Epic | `EPIC-NNNN` | `EPIC-0003` |
| User story | `STORY-NNNN` | `STORY-0042` |
| Living specification | `SPEC-NNNN` | `SPEC-0003` |
| Change delta | `CHANGE-NNNN` | `CHANGE-0012` |
| Task | `TASK-NNNN` | `TASK-0107` |
| Hand-over record | `HANDOVER-NNNN` | `HANDOVER-0230` |

### Where artifacts live

ADR 0002 puts the plan (roadmap, epics, stories, tasks, progress, decisions, questions) in the
tracker and the specifications in the repository. ADR 0007 makes runtime files the source of
truth of the ledger, the state and the summaries, with SQLite only as a rebuildable index.

| Type | Location | Reason |
| --- | --- | --- |
| Brief | Repository file, recommended (Q1) | Not settled by the ADRs; see Q1 |
| Council report | Repository file, recommended (Q1) | Not settled by the ADRs; see Q1 |
| Milestone | Tracker milestone | ADR 0002: the roadmap is in the tracker |
| Epic | Tracker issue with the label `epic`, structured block in the body | ADR 0002: epics are in the tracker |
| User story | Tracker sub-issue of its epic, structured block in the body | ADR 0002: stories are in the tracker; SPEC-0013 R5 |
| Living specification | Repository file `specs/<NNNN>-<capability>/spec.md` | ADR 0002: specifications are in the repository; ADR 0014: one per capability |
| Change delta | Repository folder of the change, layout fixed by SPEC-0009 | ADR 0002: specifications and their changes are reviewed with the code; ADR 0014 |
| Task | Tracker sub-issue of its epic, structured block in the body (see Q3) | ADR 0002: tasks are in the tracker |
| Hand-over record | JSON file under `<git common dir>/wayloop/`, published as a comment on its issue | ADR 0007: runtime files are the source; ADR 0003: summaries are published on the issue, which is the durable memory |

A plan item may exist as a file before its issue is generated (a proposal of SPEC-0013); the
same schema validates both forms. The local cache of the tracker is never a source of truth
(ADR 0002).

### Allowed transitions

The tables list the transitions the model allows (R13). Triggers and guards are specified in
SPEC-0002; a transition marked "person" is an approval state that the loop cannot pass
(constitution IV). `done` and `superseded` are terminal for every type.

### Brief

A short statement of an idea or a scope of work, input of the council (#95).

| Name | Type | Required | Meaning |
| --- | --- | --- | --- |
| `origin` | enum: `light_requirements`, `scope_of_work` | yes | What the brief was written from |
| `owner` | string, tracker handle | yes | Person accountable for the brief |
| `problem` | string | yes | Who needs this and why |
| `goals` | list of strings, at least 1 | yes | Outcomes the project must reach |
| `users` | list of strings, at least 1 | yes | Who uses the result |
| `scope_in` | list of strings, at least 1 | yes | What is included |
| `scope_out` | list of strings | no | What is excluded |
| `constraints` | list of strings | no | Budget, deadlines, technology, regulation |
| `open_questions` | list of strings | no | Questions the council must answer |

- Identifier: `BRIEF-NNNN`.
- Statuses: `draft`, `approved`, `superseded`.
- Transitions: `draft` to `approved` (person: the brief goes to the council); `draft` or `approved` to `superseded` (person).
- Required links: none. Optional: `superseded_by`.

### Council report

The moderator report of ADR 0016 and SPEC-0012 R4.

| Name | Type | Required | Meaning |
| --- | --- | --- | --- |
| `brief` | `BRIEF` identifier | yes | The brief the council answered |
| `mode` | enum: `one_mind`, `independent` | yes | How the council ran (SPEC-0012 R6) |
| `roles` | list of strings, at least 1 | yes | Roles that gave an opinion |
| `decisions` | list of objects `{id, statement, reason}`, at least 1 | yes | Decisions, cited as `COUNCIL-NNNN D<k>` |
| `rejected_alternatives` | list of objects `{option, reason}` | yes, may be empty | Options considered and rejected |
| `dissent` | list of objects `{role, position, decision}` | yes, may be empty | Positions that disagree with a decision, `decision` naming `D<k>` |
| `risks` | list of objects `{description, mitigation}` | yes, may be empty | Risks and their mitigation |
| `questions` | list of objects `{id, question, options, recommendation, answer}` | yes, may be empty | Unresolved conflicts for the maintainer, cited as `COUNCIL-NNNN Q<k>`; `answer` empty until decided |

- Identifier: `COUNCIL-NNNN`.
- Statuses: `draft`, `approved`, `superseded`.
- Transitions: `draft` to `approved` (person; guard in SPEC-0002: every question answered); `draft` or `approved` to `superseded` (person).
- Required links: `brief`, which must exist and not be superseded (R4).

### Milestone

| Name | Type | Required | Meaning |
| --- | --- | --- | --- |
| `gate` | string | yes | Observable condition that closes the milestone (release gate) |
| `council_report` | `COUNCIL` identifier | no | Report the milestone was generated from (SPEC-0013 R1) |
| `due` | date | no | Planned date |
| `tracker_number` | integer | when `status` is `implementing` or `done` | Number of the tracker milestone |

- Identifier: `M<n>`.
- Statuses: `draft`, `approved`, `implementing`, `done`, `superseded`.
- Transitions: `draft` to `approved` (person: roadmap approval); `approved` to `implementing` (engine: first task claimed); `implementing` to `done` (person: gate met); `draft`, `approved` or `implementing` to `superseded` (person).
- Required links: none. Optional: `council_report`, which must exist and be approved.

### Epic

| Name | Type | Required | Meaning |
| --- | --- | --- | --- |
| `milestone` | milestone identifier | yes | Milestone the epic belongs to |
| `outcome` | string | yes | What is true when the epic is done |
| `area` | string, an area of the project policy | yes | The `area:*` label; decides what can run in parallel |
| `issue` | integer | when `status` is `implementing` or `done` | Tracker number |

- Identifier: `EPIC-NNNN`. Title rules: R7 and R8.
- Statuses: `draft`, `approved`, `implementing`, `done`, `superseded`.
- Transitions: as for the milestone, with `implementing` to `done` taken by the engine when every task of the epic is done or superseded.
- Required links: `milestone`.

### User story

| Name | Type | Required | Meaning |
| --- | --- | --- | --- |
| `epic` | `EPIC` identifier | yes | Parent epic |
| `as_a` | string | yes | Role of the user |
| `i_want` | string | yes | Capability wanted |
| `so_that` | string | yes | Benefit |
| `acceptance_criteria` | list of objects `{id, given, when, then}`, at least 1 | yes | Criteria, cited as `STORY-NNNN AC<k>` |
| `specification` | `SPEC` identifier | when `status` is not `draft` | Specification that turns the story into requirements |
| `milestone` | milestone identifier | no | When present, must equal the milestone of the epic (R15) |
| `issue` | integer | when `status` is `implementing` or `done` | Tracker number |

- Identifier: `STORY-NNNN`.
- Statuses: `draft`, `approved`, `implementing`, `done`, `superseded`.
- Transitions: `draft` to `approved` (person); `approved` to `implementing` (engine: first task claimed); `implementing` to `done` (engine: every acceptance criterion verified); `draft`, `approved` or `implementing` to `superseded` (person).
- Required links: `epic`; `specification` once approved.

### Living specification

One per capability (ADR 0014). The Markdown layout is the template in `specs/_templates/spec.md`.

| Name | Type | Required | Meaning |
| --- | --- | --- | --- |
| `capability` | string, lower-case words separated by hyphens | yes | Capability described; the folder slug |
| `owner` | string, tracker handle | yes | Person who approves changes |
| `milestone` | milestone identifier | no | Milestone of the first delivery |
| `commit_scope` | string, a scope of the project policy | yes | Scope used by commits that implement it |
| `related` | list of strings | no | Identifiers of ADRs, issues and specifications |
| `problem` | string | yes | Who needs this and why |
| `scope_in` | list of strings, at least 1 | yes | What is specified |
| `scope_out` | list of strings | no | What is left to other specifications |
| `stories` | list of `STORY` identifiers or strings | no | Stories covered, by identifier when they exist |
| `requirements` | list of objects `{id, name, text}`, at least 1 | yes | EARS requirements; `id` is `R<k>`; `name` optional, changed by a rename (ADR 0008) |
| `acceptance_criteria` | list of objects `{id, requirements, text, verification}`, at least 1 | yes | `id` is `AC<k>`; `requirements` lists at least one existing `R<k>`; `verification` is one of `tdd`, `bdd`, `end_to_end`, `property`, `contract`, `benchmark`, `judge`, `none` |
| `interfaces` | list of strings | no | Commands, schemas and events, with versions |
| `open_questions` | list of objects `{text, resolved}` | no | Open questions; `resolved` is a boolean |

- Identifier: `SPEC-NNNN`; the folder number is not the identity, and a folder whose number differs from the identifier gives a warning.
- Statuses: `draft`, `approved`, `surveyed`, `implementing`, `done`, `superseded`.
- Transitions: `draft` to `approved` (person); `surveyed` to `approved` (person, SPEC-0011 R4); `approved` to `implementing` (engine: first task claimed); `implementing` to `done` (person, `specs/README.md`); any non-terminal status to `superseded` (person). A change delta updates the content of a specification in any status but `superseded`; it does not change the status.
- Required links: every `requirements` entry of an acceptance criterion exists in the same document (R3).

### Change delta

A reviewable change of a living specification (ADR 0014, SPEC-0009 R2).

| Name | Type | Required | Meaning |
| --- | --- | --- | --- |
| `specification` | `SPEC` identifier | yes | Living specification the delta changes |
| `proposal` | string | yes | Why the change is made |
| `operations` | list of objects `{op, requirement, name, text}`, at least 1 | yes | `op` is `added`, `modified`, `removed` or `renamed`; `requirement` is `R<k>`; `text` required for `added` and `modified`; `name` required for `renamed` |
| `tasks` | list of `TASK` identifiers | no | Tasks that implement the change |
| `issue` | integer | no | Tracker issue that requested the change |

- Identifier: `CHANGE-NNNN`.
- Statuses: `draft`, `approved`, `implementing`, `done`, `superseded`.
- Transitions: `draft` to `approved` (person); `approved` to `implementing` (engine); `implementing` to `done` (engine: delta merged and change archived, SPEC-0009 R3); any non-terminal status to `superseded` (person).
- Required links: `specification`, not superseded; for `modified`, `removed` and `renamed`, a requirement that exists in it; for `added`, an identifier never used in it (R14).

### Task

| Name | Type | Required | Meaning |
| --- | --- | --- | --- |
| `epic` | `EPIC` identifier | yes | Parent epic (R9) |
| `milestone` | milestone identifier | yes | Must equal the milestone of the epic (R9, R15) |
| `story` | `STORY` identifier | no | Story the task serves; its epic must be `epic` (R15) |
| `specification` | list of objects `{spec, requirements}` | no | References such as `SPEC-0003` with `R2`, `R4`; each must exist (R9) |
| `verification` | enum, as in the living specification | yes | Verification technique (SPEC-0009 R7) |
| `area` | string, an area of the project policy | yes | The `area:*` label |
| `depends_on` | list of `TASK` identifiers | no | Blocking tasks; mapped to native dependencies by SPEC-0005 |
| `complexity` | integer, 1 to 10 | no | Score of SPEC-0013 R3 |
| `parallel` | boolean | no | Can run in parallel with other tasks of the epic |
| `issue` | integer | when `status` is `implementing` or `done` | Tracker number, cited by branches and pull requests (R10, R11) |

- Identifier: `TASK-NNNN`. Title rules: R7 and R8.
- Statuses: `draft`, `approved`, `implementing`, `done`, `superseded`.
- Transitions: `draft` to `approved` (person: roadmap approval); `approved` to `implementing` (engine: claim); `implementing` to `approved` (engine: lease expired, SPEC-0003 R11); `implementing` to `done` (engine: closing pull request merged); `draft`, `approved` or `implementing` to `superseded` (person).
- Required links: `epic`, `milestone`; every cited specification and requirement.

### Hand-over record

The hand-over of SPEC-0003 R12, the start of the next brief (SPEC-0003 R13). Written by the
engine from agent output that passed this schema (ADR 0003).

| Name | Type | Required | Meaning |
| --- | --- | --- | --- |
| `issue` | integer | yes | Tracker number of the issue worked |
| `task` | `TASK` identifier | no | Task of the issue, when it has one |
| `session` | string | yes | Session identifier in the ledger |
| `holder` | string | yes | Holder of the checkout (SPEC-0003 R10) |
| `executor` | string | yes | Executor that ran the session, for example `claude_code` |
| `branch` | string | yes | Branch of the issue, valid under R11 |
| `step` | enum: `claim`, `reanalysis`, `counter_review`, `checklist`, `build`, `verify`, `review`, `pull_request`, `merge` | yes | Step reached in the issue cycle |
| `files_touched` | list of strings | yes, may be empty | Paths changed by the session |
| `not_done` | list of strings | yes, may be empty | Work left |
| `open_questions` | list of strings | yes, may be empty | Questions raised |
| `rulings` | list of objects `{decision, reason, cost_if_wrong}` | yes, may be empty | Rulings of the session (SPEC-0003 R5) |
| `next_step` | string | yes | First action of the next session |
| `recorded_at` | date-time | yes | When the engine recorded it |
| `tokens` | integer | yes | Size of the record, within the budget of SPEC-0003 R12 |

- Identifier: `HANDOVER-NNNN`, assigned by the engine when it records the event.
- Statuses: none. A hand-over is immutable; a later one for the same issue takes its place in the next brief.
- Required links: `issue`, a planned issue; `task`, when present, must exist and carry the same `issue`.

The `step` values are the identifiers of `IssueStep` in `crates/wayloop-core` written in
`snake_case` (ADR 0009 C4); the crate currently returns `counter-review` and `pull-request`,
which the implementation aligns.

## Examples

`examples/` holds one valid and one invalid document per type, in the normalised form, named
`<type>.valid.yaml` and `<type>.invalid.yaml`. The nine valid documents form one consistent
project and are validated together. Each invalid document is validated against the valid set;
its first comment states the requirement it breaks and the expected pointer and reason.

## Interfaces
- Schemas under `schemas/artifacts/<kind>/v<N>.json`, one per type, starting at `v1`; all new, so additive. A change follows ADR 0009 C5: a removed, renamed, retyped or newly nullable field is a new version.
- Errors use the report of ADR 0009 C3, with the source location of R12.
- Validation exposed by `wayloop validate` (SPEC-0004, #6); project structure created by `wayloop init` (#5).

## Dependencies
- Acceptance of ADR 0008 (#4): the identifier rules above follow it as written.
- ADR 0007, ADR 0009, ADR 0014, ADR 0016 and ADR 0017 are Proposed; the location of hand-overs, the error report, change deltas, the council report and the identifier in the structured block depend on them.

## Plan and tasks
`plan.md` and `tasks.md` are written after the maintainer approves this specification
(`specs/README.md`, Agent workflow).

## Open questions
- [ ] **Q1. Where do the council report and the brief live?** ADR 0002 places decisions and questions in the tracker and specifications in the repository; ADR 0016 does not name a location. The ADRs do not settle it.
  - A. Repository file, linked from a tracker issue that carries the gate. Diffable, reviewed in a pull request like a specification, a stable input for regenerating the roadmap (ADR 0017). Approval needs a pull request.
  - B. Issue body or comment, with the full report as an attachment. Closest to ADR 0002 and approvable from a phone. Attachments are not versioned, comments change without a diff, and bodies have a size limit.
  - C. A, plus a comment on the gate issue with the decisions and the questions, written by the engine, and approval given on the issue.
  - Recommendation: C for both artifacts. The file is the source and stays reviewable; the gate stays asynchronous and visible in the tracker, and the removal test holds.
- [ ] **Q2. Identifiers of tracker items.** ADR 0008 gives every artifact a prefixed identifier (`TASK-0107`); the conventions page says the issue number is the code of an issue and that a parallel scheme would drift.
  - A. Both, as written above: the artifact identifier is the identity, kept in the structured block; the issue number is the tracker key cited by branches, commits and pull requests.
  - B. Prefixed identifiers only for repository artifacts; tracker items are identified by their issue number.
  - Recommendation: A. ADR 0017 needs an identifier before the issue exists, so that generation finds the same issue again. To be settled with #4; if B is chosen, R14 and the identifier table change.
- [ ] **Q3. Is every task a tracker sub-issue?** The conventions page and R9 make every task a sub-issue of an epic; SPEC-0013 R5 turns a task below the complexity threshold into a checklist item of its story.
  - A. Every task is a sub-issue of its epic; the story is a field. R9 and R10 hold unchanged.
  - B. Tasks below the threshold are checklist items without an issue; R10 then applies only to tasks with an issue.
  - Recommendation: A, and align SPEC-0013 R5 to it: a pull request must close one planned task, which needs an issue.
