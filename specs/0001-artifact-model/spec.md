# SPEC-0001: Artifact model and schemas

| | |
| --- | --- |
| Status | Approved |
| Milestone | M0 · Foundations |
| Owner | @salvo8519 |
| Related | #3, #4, #5, #6, #95, #160; ADR 0002, 0003, 0007, 0008, 0009, 0012, 0014, 0016, 0017; SPEC-0002, SPEC-0003, SPEC-0004, SPEC-0005, SPEC-0007, SPEC-0009, SPEC-0011, SPEC-0012, SPEC-0013, SPEC-0021 |
| Commit scope | schemas |

## Problem
Every level of the loop must have a validated structure so that the engine can check it, link it to the other levels and refuse what is malformed before an agent builds on it.

## Scope
**In scope:** Brief, council report, gate issue, milestone, epic, user story, living specification, change delta, task, hand-over record: fields, identifiers and their allocation, statuses, initial statuses and allowed transitions, required links, files and their format, and where each lives (repository or tracker). The Markdown form of a living specification. The inputs `pull_request` and `commit` and the naming rules of titles, branches, commits and pull requests (R7 to R11), reused by SPEC-0021. Schema versions and their migration.

**Out of scope:** The state machine (SPEC-0002): triggers, guards and actors of each transition, and the issue-cycle state, including `blocked`. The tracker mapping (SPEC-0005): how fields map to issue fields, labels and the structured block. The content rules of hand-overs (SPEC-0003 R12, R13). Delta merge and the layout of a change folder (SPEC-0009). Generation of artifacts from other artifacts (SPEC-0012, SPEC-0013). The content of gates (SPEC-0007). Commit trailers (`Signed-off-by`, `Refs`, `Docs-impact`), which are rules of the policy (SPEC-0021 R4).

## User stories
- As a maintainer, I want every artifact to have a fixed structure, so that I review content and not formatting.
- As an agent, I want a schema for each artifact, so that my output is accepted or rejected with a precise reason.

## Requirements (EARS)
- **R1**: THE ENGINE SHALL define one versioned JSON Schema for each artifact type, and for the inputs `pull_request`, `commit` and hand-over input.
- **R2**: THE ENGINE SHALL give every artifact a stable identifier that survives renames and moves (ADR 0008).
- **R3**: WHEN an artifact is validated, THE ENGINE SHALL check its schema and, if its status is not terminal, every link it declares to other artifacts; a broken link of an artifact in a terminal status is reported as a warning.
- **R4**: IF a link of an artifact in a non-terminal status points to a missing or superseded artifact, THEN THE ENGINE SHALL report the field, the target and the fix, which for a superseded target lists the identifiers of its `superseded_by`.
- **R5**: THE ENGINE SHALL support the statuses Draft, Approved, Surveyed, Implementing, Done and Superseded where the artifact type allows them, and keep them separate from the issue-cycle state of SPEC-0002.
- **R6**: THE ENGINE SHALL accept artifacts whose prose is written in the language of the project, with identifiers, keys, enum values, Markdown headings and status words in English.
- **R7**: THE ENGINE SHALL refuse an epic or task title with a type prefix, a bracket tag or a final full stop, and accept a title that starts with a command name.
- **R8**: THE ENGINE SHALL refuse two epic or task titles of one project, in any status except `superseded`, that are equal after normalisation.
- **R9**: THE ENGINE SHALL refuse a task in a non-terminal status that has no parent epic, a task in a non-terminal status that has no milestone, and a task in a non-terminal status that cites a specification or a requirement that does not exist.
- **R10**: WHEN a `pull_request` is validated, THE ENGINE SHALL require that its body links exactly one issue, which is a planned task or a gate issue, with a closing keyword or with `Refs`, and that its branch names the same issue, unless the rule is waived by the label `no-issue` (SPEC-0021 R2).
- **R11**: THE ENGINE SHALL validate and produce branch names and commit headers with the grammar of the section Naming grammar, with the types and scopes of the project policy.
- **R12**: WHEN an artifact stored as Markdown or as a tracker item is validated, THE ENGINE SHALL normalise it into the document model of its type before applying the schema, and report every error with a JSON pointer into the document model and the source location in the field `source`.
- **R13**: IF a status change is not listed among the allowed transitions of the artifact type, THEN THE ENGINE SHALL refuse it with the reason `state` and the allowed target statuses in `expected`.
- **R14**: THE ENGINE SHALL refuse an identifier that is not in the canonical form of its artifact type, and refuse both artifacts that carry the same identifier, each error pointing to the other.
- **R15**: IF a task or a user story in a non-terminal status declares a milestone or an epic that differs from the one reached through its parent, THEN THE ENGINE SHALL refuse it and name both values.
- **R16**: THE ENGINE SHALL validate a document against the schema of its `schema_version` for every known version and migrate it to the current version before checking links; IF the version is unknown, THEN THE ENGINE SHALL refuse it with the reason `enum` and the known versions in `expected`.
- **R17**: WHEN a person approves a brief or a council report on its gate issue, THE ENGINE SHALL record the approval as a ledger event, treat that event as the authority on the status, and write the status to the file through a pull request; IF the status of the file disagrees with the ledger, THEN THE ENGINE SHALL report it with the reason `conflict` and keep the ledger value.
- **R18**: THE ENGINE SHALL refuse a duplicate part identifier within an artifact, a dissent that names a decision that does not exist, a recommendation or an answer that is not among the options of its question, and a dependency of a task on itself, on a superseded task or that closes a cycle.
- **R19**: IF an artifact is created in a status that is not an initial status of its type, THEN THE ENGINE SHALL refuse it with the reason `state` and the initial statuses in `expected`.
- **R20**: THE ENGINE SHALL refuse a change delta with two operations on one requirement, and assign the identifier of every added requirement when the delta is merged, after the highest identifier ever used in the specification, removed ones included.
- **R21**: WHERE the project has no policy, THE ENGINE SHALL validate types, scopes and areas against the default policy of SPEC-0021 R1 and list it in `meta.defaults_applied`.
- **R22**: WHEN an agent submits a hand-over, THE ENGINE SHALL validate the agent input against the hand-over input schema before it fills the engine fields, and refuse an engine field in the input with the reason `unknown_field`.
- **R23**: THE ENGINE SHALL allocate the next identifier of a type as the highest sequence of that type found on the default branch and in the tracker plus one, and check again before merge that the identifier is still unique.
- **R24**: WHEN an epic moves to another milestone, THE ENGINE SHALL change the milestone of every story and task of the epic in a non-terminal status in the same plan change.
- **R25**: WHEN the tracker item of a plan item is created, THE ENGINE SHALL treat the tracker form as the source and remove the plan file in the same change.
- **R26**: WHEN the documents of a validation were discovered in a project and no folder of the locations of Where artifacts live exists there, THE ENGINE SHALL report the warning `artifact_location_missing` and SHALL NOT refuse the set for it (#320, decided by the maintainer on 2026-11-23).

## Acceptance criteria
- **AC1** (R1, R3, R5, R6, R9, R12, R14): Given one valid and one invalid example per artifact type and per input, when validated, then only the invalid ones fail, each with its field. Verification: TDD, with the files of `examples/` as fixtures.
- **AC2** (R3, R4): Given a story linked to a missing specification, when validated, then the error names the story, the field and the missing identifier. Verification: TDD.
- **AC3** (R1, R16): Given a released schema snapshot under `schemas/artifacts/`, when its content is changed in place, then CI fails; given a schema change, then it ships as a new `v<N+1>.json` beside the unchanged `v<N>.json`. Verification: contract test.
- **AC4** (R7, R8): Given titles with a prefix, a bracket tag, a full stop, or equal to another epic or task title after normalisation, when validated, then each is reported with its pointer and fix. Verification: TDD.
- **AC5** (R9): Given a task citing a missing requirement, when validated, then the error names the task, the field and the requirement. Verification: TDD.
- **AC6** (R2, R14): Given a living specification moved to another folder and renamed, when the artifacts that link to it are validated, then every link still resolves; given two artifacts with the same identifier, then both are refused with the reason `duplicate`, each with the source of the other. Verification: TDD.
- **AC7** (R4): Given a story linked to a superseded specification, when validated, then the error names the field, the superseded identifier and, as the fix, the identifiers in its `superseded_by`. Verification: TDD, with `story.superseded-link.invalid.yaml`.
- **AC8** (R5, R13): Given a task in status `draft`, when it is moved to `done`, then it is refused with the reason `state` and `expected` lists `approved` and `superseded`; given a brief with status `implementing`, then it is refused with the reason `enum`. Verification: TDD.
- **AC9** (R6): Given a story whose prose is in Italian and whose keys are in English, when validated, then it passes; given the key `titolo` instead of `title`, then it is refused with the reason `unknown_field` and the fix `title`. Verification: TDD.
- **AC10** (R7): Given the task title `wayloop init creates a project structure that validates`, when validated, then it passes. Verification: TDD.
- **AC11** (R10): Given a pull request that links two planned tasks, or whose branch names an issue other than the one it links, when validated, then it is refused with the pointer to the body or to the branch. Verification: TDD.
- **AC12** (R11): Given any issue number, type and title, including titles with accented letters and titles without ASCII letters or digits, when a branch name is produced, then it matches `<type>/<issue>-<slug>`, the slug is ASCII with at most five words, an empty slug is replaced by `work`, and the produced name passes validation. Verification: property test.
- **AC13** (R12): Given a living specification in Markdown with a requirement missing its identifier, when validated, then the error carries the pointer `/requirements/1/id` and `source` with the file and line of the requirement. Verification: TDD.
- **AC14** (R15): Given a task in status `approved` whose milestone is M2 under an epic of M1, when validated, then it is refused and the error names M2 and M1; given the same task in status `done`, then it passes. Verification: TDD.
- **AC15** (R11): Given the branch `sprint/S01/42-ledger-hash-chain`, when validated, then it passes; given `sprint/1/42-ledger-hash-chain`, then it is refused with the reason `syntax`. Verification: TDD.
- **AC16** (R11): Given the branches `dependabot/cargo/serde-1.0.200`, `release/v0.2.0` and `revert-57-feat/12-ledger-hash-chain`, when validated, then all three pass without further checks. Verification: TDD.
- **AC17** (R11): Given the branch `feat/42-one-two-three-four-five-six`, when validated, then it is refused with the reason `length`; given `feat/42-Ledger`, then it is refused with the reason `syntax`. Verification: TDD.
- **AC18** (R11): Given the header `feat(core)!: drop the v1 envelope` with a `BREAKING CHANGE:` paragraph in the body, when validated, then it passes; without that paragraph, then it is refused with the reason `requires`. Verification: TDD.
- **AC19** (R11): Given the header `revert: drop the hash chain`, when validated, then it passes; given `feat: chain events by hash`, then it is refused with the reason `missing` at the scope. Verification: TDD.
- **AC20** (R11): Given a header of 73 characters, when validated, then it is refused with the reason `length`; given one of 72 characters, then it passes. Verification: TDD.
- **AC21** (R11): Given the header `feature(core): add x` or `feat(kernel): add x`, when validated, then it is refused with the reason `enum` and `expected` lists the types or the scopes of the policy. Verification: TDD, with `examples/policy.yaml`.
- **AC22** (R11): Given the header `feat(core): Add the ledger` or `feat(core): add the ledger.`, when validated, then it is refused with the reason `syntax` at the subject; given `feat(ledger): chain events by hash [SPEC-0003 R2]`, then it passes. Verification: TDD.
- **AC23** (R10): Given a pull request that adds a brief and closes its gate issue, when validated, then it passes; given a pull request with `Refs #12` and the branch `feat/12-record-entry`, then it passes; given a pull request without a linked issue and with the label `no-issue`, then the rule is waived and a waiver event is recorded. Verification: TDD.
- **AC24** (R3, R4): Given a task in status `done` that cites a superseded specification, when validated, then it passes with a warning that names the field and the superseded identifier. Verification: TDD.
- **AC25** (R16): Given a document with `schema_version: 99`, when validated, then it is refused with the pointer `/schema_version`, the reason `enum` and the known versions in `expected`; given a document of an earlier known version, then it is migrated and validated. Verification: TDD.
- **AC26** (R17): Given an approval on the gate issue of a draft brief, when it is processed, then a ledger event is recorded and a pull request sets `status: approved` in the file; given a brief file set to `approved` by hand without an approval event, then it is reported with the reason `conflict` and the ledger status is kept. Verification: TDD.
- **AC27** (R18): Given a council report with two decisions `D1`, a dissent naming `D4`, and an answer outside the options, and a task that depends on itself, when validated, then each is refused with its pointer and the reasons `duplicate`, `missing`, `enum` and `conflict`. Verification: TDD.
- **AC28** (R19): Given a new task created in status `implementing`, when validated, then it is refused with the reason `state` and `expected` lists `draft`; given a new specification created in status `surveyed`, then it passes. Verification: TDD.
- **AC29** (R20): Given a delta with a `modified` and a `removed` operation on R2, when validated, then it is refused with the reason `conflict`; given a specification with R1 to R3 and R4 removed, when a delta that adds one requirement is merged, then the new requirement is R5. Verification: TDD.
- **AC30** (R21): Given a project without a policy, when a commit header with the scope `ledger` is validated, then it passes and `meta.defaults_applied` lists the default policy. Verification: TDD.
- **AC31** (R22): Given a hand-over input that sets `tokens`, when submitted, then it is refused with the reason `unknown_field`; given a valid input, then the record carries the engine fields and validates against the record schema. Verification: TDD.
- **AC32** (R23): Given `TASK-0007` on the default branch and `TASK-0009` in the tracker, when a task is allocated, then it gets `TASK-0010`; given the highest task `TASK-9999`, then the next is `TASK-10000`. Verification: TDD.
- **AC33** (R24): Given an epic of M1 with one done task and two approved tasks, when the epic moves to M2, then one plan change sets M2 on the two approved tasks and leaves the done task in M1. Verification: TDD.
- **AC34** (R25): Given a task proposal file under `plan/items/`, when its issue is created, then the change that records the issue removes the file and later validation reads the tracker form. Verification: TDD.
- **AC35** (R1): Given an acceptance criterion with the verification `judge` and no `rubric`, when validated, then it is refused with the reason `requires`. Verification: TDD.
- **AC36** (R6, R12): Given a living specification in Markdown with the extra section `## Notes`, the verification text `property test for production, TDD for validation`, and the heading `## Requisiti` instead of `## Requirements (EARS)`, when normalised, then `## Notes` and `## Requisiti` are kept as prose, `verification` is `[property, tdd]`, and `/requirements` is reported with the reason `missing`. Verification: TDD.
- **AC37** (R26): Given a discovery that found no folder of a location (`locations: []`), when validated, then the result succeeds with the warning `artifact_location_missing`, reason `missing`, at `/locations`, and the check `artifact_locations` is listed as run; given documents named or sent directly, without `locations`, then the check is listed as not done with the reason `input_absent`. Verification: TDD.

## Artifact model

### Common rules

Every artifact, whatever its storage, is validated as one document of the model below (R12).
Markdown files and tracker items are first normalised into this document; the examples in
`examples/` show the normalised form. Keys are English and in `snake_case`; dates follow ISO
8601 (ADR 0009 C4). Prose fields are in the language of the project (R6).

Enum values in the document model and in JSON are `snake_case`, for example `counter_review`
and `end_to_end`. Kebab-case forms such as the label `in-progress` or a command-line value
are spellings of the adapters, which map them to the enum value; the core never reads them.

Absent and null mean not given (ADR 0009 C2): an optional field is left out until it has a
value, never set to an empty string. A list that may be empty is written `[]`.

Fields common to every type (`Set by` says who writes the field: `author` is the person or
agent that writes the artifact, `engine` is the engine only):

| Name | Type | Required | Set by | Meaning |
| --- | --- | --- | --- | --- |
| `kind` | enum: `brief`, `council_report`, `gate`, `milestone`, `epic`, `story`, `specification`, `change`, `task`, `handover` | yes | author | Artifact type; selects the schema |
| `schema_version` | integer, at least 1 | yes | author | Version of the schema of the type the document was written against (R16) |
| `id` | string, canonical form of the type | yes | engine (R23) | Stable identifier (R2, R14) |
| `title` | string, 1 to 120 characters | yes, except `handover` | author | Outcome or name, in sentence case without a final full stop |
| `status` | enum, values allowed by the type | yes, except `handover` | author, then engine for approvals (R17) | Current status (R5) |
| `language` | BCP 47 tag | no | author | Language of the prose; when absent, the language of the project, listed in `meta.defaults_applied` (ADR 0009 C2) |
| `superseded_by` | list of at least one identifier of the same type | when `status` is `superseded` | author | Replacements, used as the fix of R4; an artifact may be replaced by several, for example a task expanded into subtasks |

Statuses in Markdown headers keep their capitalised English form (`Draft`); the document model
uses the lower-case value (`draft`). The status of an artifact is not the issue-cycle state of
SPEC-0002: a task in status `implementing` may be blocked, claimed or under review in its
issue cycle, and `blocked` is never an artifact status.

### Project policy

Types and scopes of commits, and the areas of epics and tasks, come from the project policy
(SPEC-0021 R1). When the project has no policy, the default policy of SPEC-0021 R1 applies,
which equals the policy of this repository, and it is listed in `meta.defaults_applied` (R21).
`examples/policy.yaml` holds the subset of the policy the model reads.

An `area` value is the area name without the label prefix: `core`, not `area:core`. The
tracker adapter adds the prefix when it writes the label (SPEC-0005).

### Identifiers

The rules below are those of ADR 0008, accepted with its amendment on the roles of the two
identifiers (#4).

- An identifier is a type prefix and a sequence, per type and per project, for example
  `STORY-0042`, `TASK-0107`. The canonical form of the sequence is `[0-9]{4}|[1-9][0-9]{4,}`:
  four digits, zero-padded, up to `9999`, then the number without padding (`TASK-10000`).
  `TASK-107` and `TASK-00107` are refused (R14).
- It is assigned at creation, is independent of the file name, the path and the title, and is
  never reused, also after the artifact is superseded or deleted.
- Parts of an artifact are cited as `<id> <part>`: requirements `SPEC-0003 R2`, acceptance
  criteria of a specification `SPEC-0003 AC1`, of a story `STORY-0042 AC2`, decisions of a
  council report `COUNCIL-0001 D3`, questions `COUNCIL-0001 Q1`. Part numbers are `[1-9][0-9]*`
  without padding, unique within their list (R18) and never reused within their artifact.
- Links are checked by identifier, never by path (ADR 0008).
- Tracker items also carry their tracker number (`issue`) once created. The artifact
  identifier is the identity, kept in the structured block, and what generation uses to find
  the same issue again (ADR 0017); the tracker number is what branches, commits, pull requests
  and release notes cite (ADR 0008, conventions page). The engine records the pair, which
  never changes once recorded.

| Type | Canonical form | Example |
| --- | --- | --- |
| Brief | `BRIEF-<seq>` | `BRIEF-0001` |
| Council report | `COUNCIL-<seq>` | `COUNCIL-0001` |
| Gate issue | `GATE-<seq>` | `GATE-0001` |
| Milestone | `M<n>`, `n` matching `0\|[1-9][0-9]*` | `M0`, `M12` |
| Epic | `EPIC-<seq>` | `EPIC-0003` |
| User story | `STORY-<seq>` | `STORY-0042` |
| Living specification | `SPEC-<seq>` | `SPEC-0003` |
| Change delta | `CHANGE-<seq>` | `CHANGE-0012` |
| Task | `TASK-<seq>` | `TASK-0107` |
| Hand-over record | `HANDOVER-<issue>-<k>`, both matching `[1-9][0-9]*` | `HANDOVER-12-3` |

Allocation (R23). The allocator of the engine gives a new artifact the highest sequence of its
type found on the default branch (files) and in the tracker (structured blocks and milestone
descriptions), plus one; removed and superseded artifacts count. Two branches may allocate the
same identifier; CI checks uniqueness again against the default branch and the tracker before
merge, and both artifacts that carry the identifier are refused, each error pointing to the
other (R14). The branch that is not yet merged then takes a new identifier before it merges (Q4).

Milestones are numbered from `M0`, without leading zeros. The number records the order of
creation, not the order of delivery: moving a milestone in the plan never renumbers it.

Hand-over identifiers need no allocator: the engine writes `HANDOVER-<issue>-<k>`, where `k`
is the next number of the hand-overs of that issue. Only the holder of the checkout can end a
session (SPEC-0003 R10), so two hand-overs of one issue are never recorded at the same time.

### Where artifacts live

ADR 0002 puts the plan (roadmap, epics, stories, tasks, progress, decisions, questions) in the
tracker and the specifications in the repository. ADR 0007 makes runtime files the source of
truth of the ledger, the state and the summaries, with SQLite only as a rebuildable index.

| Type | Location | Reason |
| --- | --- | --- |
| Brief | Repository file `plan/briefs/BRIEF-<seq>.yaml`; the engine posts its summary and open questions as a comment on its gate issue, where the person approves it | Decision of the maintainer (Q1): the file stays reviewable with a diff, the gate stays asynchronous in the tracker (ADR 0002) |
| Council report | Repository file `plan/council/COUNCIL-<seq>.yaml`; the engine posts its decisions and questions as a comment on its gate issue, where the person approves it | As for the brief (Q1); a stable input for regenerating the roadmap (ADR 0017) |
| Gate issue | Tracker issue, structured block in the body | ADR 0012: gates are tracker states; Q1: approval is given on the issue |
| Milestone | Tracker milestone, structured block in the description | ADR 0002: the roadmap is in the tracker |
| Epic | Tracker issue with the label `epic`, structured block in the body | ADR 0002: epics are in the tracker |
| User story | Tracker sub-issue of its epic, structured block in the body | ADR 0002: stories are in the tracker; SPEC-0013 R5 |
| Living specification | Repository file `specs/<NNNN>-<capability>/spec.md` | ADR 0002: specifications are in the repository; ADR 0014: one per capability |
| Change delta | Repository folder of the change; its layout is fixed by SPEC-0009, not by this specification | ADR 0002: specifications and their changes are reviewed with the code; ADR 0014 |
| Task | Tracker sub-issue of its epic, structured block in the body, whatever its complexity | ADR 0002: tasks are in the tracker; decision of the maintainer (Q3): a pull request closes one planned task, which needs an issue |
| Hand-over record | JSON file under `<git common dir>/wayloop/`, published as a comment on its issue | ADR 0007: runtime files are the source; ADR 0003: summaries are published on the issue, which is the durable memory |

Brief and council report files are YAML in the normalised form of their type, as in
`examples/`. YAML needs no second parser and no normalisation step, and an agent fills it
against the schema directly (ADR 0009 C4); long prose uses block scalars. The file name carries
the identifier so that a person finds the file; the identity is the `id` field, and a file name
that differs from it gives a warning.

Plan items before their tracker item exists. A milestone, epic, story or task proposed by
SPEC-0013 is a file `plan/items/<id>.yaml` in the normalised form, validated by the same
schema as the tracker form. When the tracker item is created, the tracker form is the source
(ADR 0002) and the change that records the issue or milestone number removes the file (R25):
two writable copies of one item would drift. `wayloop init` creates the folders `plan/briefs/`,
`plan/council/` and `plan/items/`. The local cache of the tracker is never a source of truth
(ADR 0002).

### Statuses and transitions

The tables list the transitions the model allows (R13) and the initial statuses (R19).
Triggers and guards are specified in SPEC-0002; a transition marked "person" is an approval
state that the loop cannot pass (constitution IV). `done` and `superseded` are terminal for
every type: an artifact in a terminal status is not changed again, and its broken links are
warnings, not errors (R3).

| Type | Initial statuses |
| --- | --- |
| Brief, council report, milestone, epic, user story, change delta, task, gate issue | `draft` |
| Living specification | `draft`, or `surveyed` when written by the survey (SPEC-0011 R4) |
| Hand-over record | none: it has no status |

A tracker issue of a `done` task that is reopened by a person does not reopen the task: `done`
is terminal, the engine reports the mismatch to reconciliation (SPEC-0007), and further work is
a new task. A reopened task would change the meaning of the pull request that closed it.

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

- Identifier: `BRIEF-<seq>`.
- Statuses: `draft`, `approved`, `superseded`.
- Transitions: `draft` to `approved` (person, on the gate issue, R17: the brief goes to the council); `draft` or `approved` to `superseded` (person).
- Required links: none. Optional: `superseded_by`.

### Council report

The moderator report of ADR 0016 and SPEC-0012 R4.

| Name | Type | Required | Meaning |
| --- | --- | --- | --- |
| `brief` | `BRIEF` identifier | yes | The brief the council answered |
| `mode` | enum: `one_mind`, `independent` | yes | How the council ran (SPEC-0012 R6) |
| `roles` | list of strings, at least 1 | yes | Roles that gave an opinion |
| `decisions` | list of objects `{id, statement, reason}`, at least 1 | yes | Decisions; `id` is `D<k>`, cited as `COUNCIL-<seq> D<k>` |
| `rejected_alternatives` | list of objects `{option, reason}` | yes, may be empty | Options considered and rejected |
| `dissent` | list of objects `{role, position, decision}` | yes, may be empty | Positions that disagree with a decision; `decision` is the `D<k>` of a decision of this report (R18) |
| `risks` | list of objects `{description, mitigation}` | yes, may be empty | Risks and their mitigation |
| `questions` | list of objects `{id, question, options, recommendation, answer}` | yes, may be empty | Unresolved conflicts for the maintainer; `id` is `Q<k>`, cited as `COUNCIL-<seq> Q<k>`; `options` has at least 2 entries; `recommendation` is one of `options`; `answer` is absent until decided, then one of `options` (R18) |

- Identifier: `COUNCIL-<seq>`.
- Statuses: `draft`, `approved`, `superseded`.
- Transitions: `draft` to `approved` (person, on the gate issue, R17; guard in SPEC-0002: every question answered); `draft` or `approved` to `superseded` (person).
- Required links: `brief`, which must exist and not be superseded (R4).

### Gate issue

The tracker issue on which a person approves a brief or a council report (Q1, ADR 0012). It
makes the approval asynchronous and visible, and gives the pull request that adds the file an
issue to close (R10).

| Name | Type | Required | Meaning |
| --- | --- | --- | --- |
| `subject` | `BRIEF` or `COUNCIL` identifier | yes | The brief or council report submitted for approval |
| `issue` | integer | when `status` is `done`; optional in `draft` and `superseded` (decided by the maintainer on 2026-11-23) | Tracker number |
| `approved_by` | string, tracker handle | when `status` is `done` | Person who approved, as recorded in the ledger (R17) |

- Identifier: `GATE-<seq>`.
- Statuses: `draft`, `done`, `superseded`.
- Transitions: `draft` to `done` (person: approval on the issue, recorded by the engine, R17); `draft` to `superseded` (person, or engine when the subject is superseded). When the subject is superseded, the engine always creates a new gate for the replacement subject; a gate still in `draft` becomes `superseded` with `superseded_by` naming the new gate, while a gate in `done` stays `done`, since it records an approval that took place (decided by the maintainer on 2026-11-23).
- Required links: `subject`, which must exist.

Flow. The pull request that adds the brief or the report closes the gate issue (R10). The
engine posts the summary and the questions as a comment on the gate issue. The person approves
there while the pull request is open; the engine records the approval event and commits the
status `approved` to the branch of that pull request, which can then merge. A later status
change, for example to `superseded`, reaches the file through a new pull request that refers to
the gate issue with `Refs #<n>`. Whatever the file says, the ledger event is the authority
(R17); a status set by hand in the file is reported as a conflict.

### Milestone

| Name | Type | Required | Meaning |
| --- | --- | --- | --- |
| `gate` | string | yes | Observable condition that closes the milestone (release gate) |
| `council_report` | `COUNCIL` identifier | no | Report the milestone was generated from (SPEC-0013 R1) |
| `due` | date | no | Planned date |
| `tracker_number` | integer | when `status` is `implementing` or `done` | Number of the tracker milestone |

- Identifier: `M<n>`, stored in the structured block of the milestone description.
- Statuses: `draft`, `approved`, `implementing`, `done`, `superseded`.
- Transitions: `draft` to `approved` (person: roadmap approval); `approved` to `implementing` (engine: first task claimed); `implementing` to `done` (person: gate met); `draft`, `approved` or `implementing` to `superseded` (person).
- Required links: none. Optional: `council_report`, which must exist and be approved.

The milestone uses `tracker_number`, not `issue`: a tracker milestone is not an issue, and its
number belongs to a sequence separate from issues and pull requests, so a field named `issue`
would invite citing it as `#<n>`, which links an unrelated issue.

### Epic

| Name | Type | Required | Meaning |
| --- | --- | --- | --- |
| `milestone` | milestone identifier | yes | Milestone the epic belongs to |
| `outcome` | string | yes | What is true when the epic is done |
| `area` | string, an area of the project policy, without the `area:` prefix | yes | Decides what can run in parallel |
| `issue` | integer | when `status` is `implementing` or `done` | Tracker number |

- Identifier: `EPIC-<seq>`. Title rules: R7 and R8.
- Statuses: `draft`, `approved`, `implementing`, `done`, `superseded`.
- Transitions: as for the milestone, except `implementing` to `done`, taken by the engine when every task of the epic is done or superseded and at least one is done. An epic without tasks, or whose tasks are all superseded, never moves to `done` by itself: nothing was delivered, so a person supersedes it or adds tasks.
- Required links: `milestone`.
- Moving the epic to another milestone changes the milestone of its stories and tasks in a non-terminal status in the same plan change (R24); done and superseded items keep the milestone in which they ended.

### User story

| Name | Type | Required | Meaning |
| --- | --- | --- | --- |
| `epic` | `EPIC` identifier | yes | Parent epic |
| `milestone` | milestone identifier | yes | Must equal the milestone of the epic (R15), as for the task |
| `as_a` | string | yes | Role of the user |
| `i_want` | string | yes | Capability wanted |
| `so_that` | string | yes | Benefit |
| `acceptance_criteria` | list of objects `{id, given, when, then}`, at least 1 | yes | Criteria; `id` is `AC<k>`, cited as `STORY-<seq> AC<k>` |
| `specification` | list of at least one `SPEC` identifier | when `status` is `approved`, `implementing` or `done` | Specifications that turn the story into requirements; a story may span capabilities (ADR 0014) |
| `issue` | integer | when `status` is `implementing` or `done` | Tracker number |

- Identifier: `STORY-<seq>`.
- Statuses: `draft`, `approved`, `implementing`, `done`, `superseded`.
- Transitions: `draft` to `approved` (person); `approved` to `implementing` (engine: first task claimed); `implementing` to `done` (engine: every task that names the story is done or superseded, and at least one is done); `draft`, `approved` or `implementing` to `superseded` (person).
- Required links: `epic`, `milestone`; `specification` from `approved` on.

The guard for `done` is the tasks of the story, not a verification record per criterion. Task
states are observable in the tracker; criteria of a story are refined into acceptance criteria
of its specifications, which the tasks verify (SPEC-0009 R7). A record per story criterion would
duplicate that chain and could disagree with it.

### Living specification

One per capability (ADR 0014). The Markdown layout is the template in
`specs/_templates/spec.md`; its normalisation is described below.

| Name | Type | Required | Meaning |
| --- | --- | --- | --- |
| `capability` | string, words of `[a-z0-9]+` joined by single hyphens, as the `<slug>` of a branch (Naming grammar), with no limit on the number of words (decided by the maintainer on 2026-11-24) | yes | Capability described; the folder slug |
| `owner` | string, tracker handle | yes | Person who approves changes |
| `milestone` | milestone identifier | no | Milestone of the first delivery |
| `commit_scope` | string, a scope of the project policy | yes | Scope used by commits that implement it |
| `related` | list of strings | no | Identifiers of ADRs, issues and specifications; a citation `ADR <NNNN>` with no ADR of that number gives a warning (Q5) |
| `problem` | string | yes | Who needs this and why |
| `scope_in` | list of strings, at least 1 | yes | What is specified |
| `scope_out` | list of strings | no | What is left to other specifications |
| `stories` | list of `STORY` identifiers | no | Stories covered, by identifier |
| `user_stories` | list of strings | no | Stories written as prose, before or without a story artifact |
| `requirements` | list of objects `{id, name, text, status}`, at least 1 | yes | EARS requirements; `id` is `R<k>`; `name` optional, changed by a rename (ADR 0008); `status` is `active` or `removed`; absent means `active`, which is not a default and gives no entry in `meta.defaults_applied` (decided by the maintainer on 2026-11-24) |
| `acceptance_criteria` | list of objects `{id, requirements, text, verification, rubric}`, at least 1 | yes | `id` is `AC<k>`; `requirements` lists at least one `R<k>` of an active requirement; `verification` is a list of at least one of `tdd`, `bdd`, `end_to_end`, `property`, `contract`, `benchmark`, `judge`, `none`; `rubric` is a string, required when `verification` contains `judge` (SPEC-0009 R8) |
| `interfaces` | list of strings | no | Commands, schemas and events, with versions |
| `open_questions` | list of objects `{id, text, resolved}` | no | `id` is `Q<k>`, optional; `resolved` is a boolean |
| `extra_sections` | list of objects `{heading, text}` | no | Sections outside the template, kept as prose and not validated; `heading` required and not empty, `text` optional, absent when the section has no content (decided by the maintainer on 2026-11-24) |

- Identifier: `SPEC-<seq>`; the folder number is not the identity, and a folder whose number differs from the identifier gives a warning.
- Statuses: `draft`, `approved`, `surveyed`, `implementing`, `done`, `superseded`.
- Transitions: `draft` to `approved` (person); `surveyed` to `approved` (person, SPEC-0011 R4); `approved` to `implementing` (engine: first task that cites the specification claimed); `implementing` to `done` (person, `specs/README.md`); any non-terminal status to `superseded` (person). A change delta updates the content of a specification in any status but `superseded`; it does not change the status.
- Required links: every `requirements` entry of an acceptance criterion names an active requirement of the same document (R3).
- Removed requirements stay in the list with `status: removed`. The record keeps the highest identifier ever used visible, so that an added requirement never reuses one (R20), and lets a task in a terminal status that cites it resolve with a warning.

#### Markdown form of a living specification

Headings, header rows and status words are always in English, whatever the language of the
project (R6); prose is in the language of the project. A section whose heading is not in the
table, including a translated heading, is kept in `extra_sections` as prose; a required field
whose section is missing is then reported with the reason `missing`.

| Markdown | Document model |
| --- | --- |
| `# SPEC-<seq>: <title>` | `id`, `title` |
| Folder `specs/<NNNN>-<capability>/` | `capability` |
| Header row `Status` (`Draft`, `Approved`, `Surveyed`, `Implementing`, `Done`, `Superseded`) | `status`, lower case |
| Header row `Milestone` (`M<n> · <name>`) | `milestone`, the `M<n>` part |
| Header row `Owner` | `owner` |
| Header row `Related` | `related`, split at commas and semicolons |
| Header row `Commit scope` | `commit_scope` |
| Header row `Superseded by` | `superseded_by`, split at commas |
| Header row `Language`, optional | `language` |
| `## Problem` | `problem` |
| `## Scope`, paragraphs `**In scope:**` and `**Out of scope:**` | `scope_in`, `scope_out`: one entry per list item, or the paragraph as one entry |
| `## User stories` | an item that is only a `STORY` identifier goes to `stories`; any other item to `user_stories` |
| `## Requirements (EARS)`, items `- **R<k>**: <text>`, `- **R<k>** *<name>*: <text>`, `- **R<k>** (removed): <text>` | `requirements` |
| `## Acceptance criteria`, items `- **AC<k>** (R<i>, R<j>): <text> Verification: <words>. Rubric: <text>` | `acceptance_criteria`; `Rubric:` is optional |
| `## Interfaces` | `interfaces` |
| `## Open questions`, items `- [ ] <text>` and `- [x] <text>`, optionally starting with `**Q<k>. ...**` | `open_questions` |
| Any other `##` section | `extra_sections` |

Verification words, matched without regard to case in the text after `Verification:` up to
the first full stop, in the order they appear; other words of that text are ignored. A text
with no mapped word is refused with the reason `enum` and the words below in `expected`.

| Markdown word | Enum value |
| --- | --- |
| `TDD` | `tdd` |
| `BDD`, `BDD scenario` | `bdd` |
| `end-to-end` | `end_to_end` |
| `property`, `property test` | `property` |
| `contract`, `contract test` | `contract` |
| `benchmark` | `benchmark` |
| `judge` | `judge` |
| `none` | `none` |

Most specifications of this repository predate these rules (acceptance criteria without their
requirements, open questions without identifiers, extra sections). A migration task that
brings them to this form (#229) must be done before `wayloop validate` runs on this repository.

### Change delta

A reviewable change of a living specification (ADR 0014, SPEC-0009 R2).

| Name | Type | Required | Meaning |
| --- | --- | --- | --- |
| `specification` | `SPEC` identifier | yes | Living specification the delta changes |
| `proposal` | string | yes | Why the change is made |
| `design` | string | no | How the change is made, when the proposal does not say it |
| `operations` | list of objects `{op, requirement, name, text}`, at least 1 | yes | `op` is `added`, `modified`, `removed` or `renamed`; `requirement` is the `R<k>` of an existing requirement, required for `modified`, `removed` and `renamed`, refused for `added`; `text` required for `added` and `modified`, refused for `removed` and `renamed`; `name` required for `renamed`, optional for `added` and `modified`, refused for `removed` (decided by the maintainer on 2026-11-24) |
| `tasks` | list of `TASK` identifiers | no | Tasks that implement the change |
| `issue` | integer | no | Tracker issue that requested the change |

- Identifier: `CHANGE-<seq>`.
- Statuses: `draft`, `approved`, `implementing`, `done`, `superseded`.
- Transitions: `draft` to `approved` (person); `approved` to `implementing` (engine); `implementing` to `done` (engine: delta merged and change archived, SPEC-0009 R3); any non-terminal status to `superseded` (person).
- Required links: `specification`, not superseded; for `modified`, `removed` and `renamed`, an active requirement of it. At most one operation per requirement (R20): a rename with a new text is one `modified` operation with `name`.
- A member refused by the `op` of an operation is an error at that member, `/operations/<i>/<member>`, with the code `artifact_schema`, the reason `unknown_field` and `/operations/<i>/op` in `pointers`. The schema cannot state the absence of a member under a condition, so the check of parts enforces it (decided by the maintainer on 2026-11-24).
- An `added` requirement gets its `R<k>` when the delta is merged, after the highest identifier of the specification, removed ones included (R20). Two deltas open at the same time therefore never claim the same number.

### Task

| Name | Type | Required | Meaning |
| --- | --- | --- | --- |
| `epic` | `EPIC` identifier | yes | Parent epic (R9) |
| `milestone` | milestone identifier | yes | Must equal the milestone of the epic (R9, R15) |
| `story` | `STORY` identifier | no | Story the task serves; its epic must be `epic` (R15) |
| `specification` | list of objects `{spec, requirements}` | no | References such as `SPEC-0003` with `R2`, `R4`; each must exist (R9) |
| `verification` | list of at least one enum value, as in the living specification | yes | Verification techniques (SPEC-0009 R7) |
| `rubric` | string | when `verification` contains `judge` | Rubric of the judge (SPEC-0009 R8) |
| `area` | string, an area of the project policy, without the `area:` prefix | yes | Decides what can run in parallel |
| `depends_on` | list of `TASK` identifiers | no | Blocking tasks; never the task itself, a superseded task, or a cycle (R18); mapped to native dependencies by SPEC-0005 |
| `complexity` | integer, 1 to 10 | no | Score of SPEC-0013 R3 |
| `expanded_from` | `TASK` identifier | no | Task this subtask was expanded from (SPEC-0013 R3); that task is superseded and lists this one in its `superseded_by` |
| `parallel` | boolean | no | Can run in parallel with other tasks of the epic |
| `issue` | integer | when `status` is `implementing` or `done` | Tracker number, cited by branches and pull requests (R10, R11) |

- Identifier: `TASK-<seq>`. Title rules: R7 and R8.
- Statuses: `draft`, `approved`, `implementing`, `done`, `superseded`.
- Transitions: `draft` to `approved` (person: roadmap approval); `approved` to `implementing` (engine: claim); `implementing` to `approved` (engine: lease expired, SPEC-0003 R11); `implementing` to `done` (engine: closing pull request merged); `draft`, `approved` or `implementing` to `superseded` (person, or engine when the task is expanded into subtasks).
- Required links: `epic`, `milestone`; every cited specification and requirement; `expanded_from`, when present.

### Hand-over record

The hand-over of SPEC-0003 R12, the start of the next brief (SPEC-0003 R13). The agent submits
a hand-over input; the engine validates it against the input schema first (R22), then adds the
fields it owns and records the result, which validates against the record schema (ADR 0003:
model output enters the state only after schema validation). The input schema has no engine
field, so an agent that sends one, for example `tokens`, is refused with `unknown_field`.

| Name | Type | Required | Set by | Meaning |
| --- | --- | --- | --- | --- |
| `kind` | `handover` | yes | engine | Artifact type |
| `schema_version` | integer | yes | agent in the input, engine in the record | Version of the input schema, then of the record schema |
| `id` | `HANDOVER-<issue>-<k>` | yes | engine | Identifier |
| `language` | BCP 47 tag | no | agent | Language of the prose fields (`not_done`, `open_questions`, `rulings`, `next_step`), as in every artifact (Common rules); when absent, the language of the project (decided by the maintainer on 2026-11-24) |
| `issue` | integer | yes | engine | Tracker number of the issue worked, from the checkout |
| `task` | `TASK` identifier | no | engine | Task of the issue, when it has one |
| `session` | string | yes | engine | Session identifier in the ledger |
| `holder` | string | yes | engine | Holder of the checkout (SPEC-0003 R10) |
| `executor` | string | yes | engine | Executor that ran the session, for example `claude_code` |
| `branch` | string | yes | engine | Branch of the checkout, valid under R11 |
| `step` | enum: `claim`, `reanalysis`, `counter_review`, `checklist`, `build`, `verify`, `review`, `pull_request`, `merge` | yes | agent | Step reached in the issue cycle |
| `files_touched` | list of project paths, in the format `project-path` of `origin.file` (plan, Canonical input of `wayloop validate`) | yes, may be empty | agent | Paths changed by the session, relative to the root of the project; a path with `..` or `\` is refused (decided by the maintainer on 2026-11-24) |
| `not_done` | list of strings | yes, may be empty | agent | Work left |
| `open_questions` | list of strings | yes, may be empty | agent | Questions raised |
| `rulings` | list of objects `{decision, reason, cost_if_wrong}` | yes, may be empty | agent | Rulings of the session (SPEC-0003 R5) |
| `next_step` | string | yes | agent | First action of the next session |
| `recorded_at` | date-time in UTC, with the suffix `Z` | yes | engine | When the engine recorded it |
| `tokens` | integer | yes | engine | Size of the record, within the budget of SPEC-0003 R12; counted by the engine as the length in bytes of the canonical JSON (RFC 8785) of the record without the member `tokens`, divided by four, rounded up (Q6), as SPEC-0003 counts a summary; the count never depends on itself (decided by the maintainer on 2026-11-24) |

- Identifier: `HANDOVER-<issue>-<k>`, assigned by the engine when it records the event.
- Statuses: none. A hand-over is immutable; a later one for the same issue takes its place in the next brief.
- Required links: `issue`, a planned issue; `task`, when present, must exist and carry the same `issue`.

The `step` values are the identifiers of `IssueStep` of `wayloop-schemas`, re-exported by
`wayloop-core` (decided by the maintainer on 2026-11-24), written in `snake_case`, for example
`counter_review` and `pull_request` (#228). Adding a step is an additive change of the enum;
removing or renaming a step is a breaking change and a new version of both hand-over schemas,
with a migration of older records (R16).


## Inputs

These inputs are not artifacts: they have no `kind`, no identifier and no status; the
`input_kind` of their entry selects their schema. Adapters assemble them from the tracker or
from git (SPEC-0021 R3); the core validates them.

Every input carries `schema_version`, like the artifacts: the version of the schema of the input
it was written against, required, an integer from 1 (decided by the maintainer on 2026-11-24).
An input is checked against the schema of its own version and migrated to the current one, as
an artifact is, so R16 holds for inputs too. The hand-over input carries it already (Hand-over
record).

Pull request and commit inputs carry no `language` (decided by the maintainer on 2026-11-25):
adapters build them from the tracker and from git, which give none, and their prose is in the
language of the project (R6). They therefore do not share the `Header` of the artifacts, and
`schema_version` is a member of each input type on purpose.

The texts `title`, `branch` and `header`, and `body` of a commit when present, have at least
one character; the `body` of a pull request may be empty and `labels` may be an empty list. An
optional text is left out when it has no value, never written empty (decided by the maintainer
on 2026-11-24). Each label is a non-empty text (decided by the maintainer on 2026-11-25). An
empty text where one is required is refused with the reason `length`.

Names are kept apart from prose (decided by the maintainer on 2026-11-25). The `branch` and each
label are names given by a tool, git or the tracker, spelt as the tool spells it: they are not
prose and have no language, and their type says so. The `title`, the `header` and the bodies are
prose, in the language of the project.

`pull_request` (R10):

| Name | Type | Required | Meaning |
| --- | --- | --- | --- |
| `schema_version` | integer, at least 1 | yes | Version of the schema of the input (R16) |
| `title` | string, at least 1 character | yes | Pull request title, a commit header under R11 |
| `body` | string | yes, may be empty | Pull request body, where the issue is linked |
| `branch` | name, at least 1 character | yes | Head branch, a name given by git |
| `labels` | list of names, each at least 1 character | yes, may be empty | Labels, names given by the tracker, in its spelling |

`commit` (R11):

| Name | Type | Required | Meaning |
| --- | --- | --- | --- |
| `schema_version` | integer, at least 1 | yes | Version of the schema of the input (R16) |
| `header` | string, at least 1 character | yes | First line of the message |
| `body` | string, at least 1 character | no | Lines after the blank line, trailers included; absent when the message has one line |

Issue links in the body (R10). A closing keyword (`Closes`, `Fixes`, `Resolves` and their
forms, without regard to case) followed by `#<n>` links an issue the pull request completes;
`Refs #<n>` links an issue it does not complete, as the conventions page says. Exactly one issue
is linked, and it is a planned task or a gate issue; with `Refs`, the task is not marked done.
The issue number of the branch equals the linked issue. The label `no-issue` is the waiver of
this rule under SPEC-0021 R2, for trivial changes; its use is recorded (SPEC-0021 R7).

## Naming grammar

The grammar of R11, which is the grammar of the conventions page, sections Branches and
Commits. `<type>` and `<scope>` are values of the project policy (R21).

Branches:

| Form | Rule |
| --- | --- |
| `<type>/<issue>-<slug>` | `<issue>` matching `[1-9][0-9]*`; `<slug>` of 1 to 5 words of `[a-z0-9]+` joined by single hyphens |
| `sprint/S<NN>/<issue>-<slug>` | as above; `<NN>` two digits or more, the number of the sprint file |
| `dependabot/<anything>`, `release/v<anything>`, `revert-<n>-<anything>` | exempt: accepted without further checks |

Commit header and pull request title:

```text
<type>(<scope>)[!]: <subject>[ [SPEC-<seq>[ R<k>[, R<k>]*]]]
```

- `<type>` is a type of the policy; `<scope>` is required and is a scope of the policy, except
  for `revert`, where it is optional.
- `!` marks a breaking change; the commit then needs a paragraph starting with
  `BREAKING CHANGE:` in its body (reason `requires`). For a pull request title, the label
  `breaking-change` is the evidence, checked by SPEC-0021.
- `<subject>` starts with a lower-case letter and has no final full stop.
- The header is at most 72 characters, reference included.

Producing a branch name: the type is given; the slug is taken from the title, transliterated to
ASCII (accents removed, for example `città` to `citta`), lower-cased, with every run of other
characters turned into one hyphen, and cut after five words. When nothing is left, the slug is
`work`.

Titles (R7, R8):

- A type prefix is a leading `<word>:` or `<word>(<anything>):` where `<word>` is a type of
  the policy, without regard to case.
- A bracket tag is a group `[...]` at the start or at the end of the title.
- A final full stop is a title that ends with `.`.
- Two titles are equal when they are equal after Unicode NFC normalisation, case folding,
  trimming and collapsing runs of white space into one space. Epic and task titles are compared
  with each other, in every status except `superseded`.

## Errors

Every refusal is the error report of ADR 0009 C3, with every error found in one pass. Pointers
follow RFC 6901 from the root of the document model sent to validation, which is the input of
C3 (R12); the field `document` of the error gives the index of that document in the input
(SPEC-0004 R2). Each error also carries `source`, the location in the stored form: `{file,
line}` for a repository file, with `line` only when it is known, `{issue}` for a tracker
issue, `{milestone}` for a tracker milestone.
`source` is absent when the document was sent directly in the normalised form. `source` has no
tag naming its form, on purpose: it is output only, and its field names tell the forms apart.

Each error of `identifier_duplicate` names every other artifact that carries the identifier in
`related` (SPEC-0004 R2): its `document`, the pointer `/id` and the members of its `source`
(decided by the maintainer on 2026-11-25). A file name or a folder number that differs from the
identifier (section Where artifacts live; living specification) is the warning
`identifier_location` at `/id`, with `fix` the identifier, given only for an artifact read
from a file of a location (decided by the maintainer on 2026-11-25).

`expected` lists accepted values, never a pattern. For a format error (`syntax`), `expected`
is absent and `fix` carries a corrected value when one can be derived, for example `M2` for
`Milestone 2`. A new meaning gets a new code, and codes are never renamed or reused (ADR 0009
C8).

| Requirement | Code | Reasons |
| --- | --- | --- |
| R1, R6, R12 | `artifact_schema` | `missing`, `unknown_field`, `type`, `enum`, `length`, `range`, `syntax`, `requires` |
| R3, R4, R9 | `link_missing` | `missing` |
| R4 | `link_superseded` | `state` |
| R3 (terminal holder) | `link_broken_terminal` (warning) | `missing`, `state` |
| R7 | `title_format` | `syntax` |
| R8 | `title_duplicate` | `duplicate` |
| R10 | `pull_request_link` | `missing`, `conflict` |
| R11 | `branch_name` | `syntax`, `length`, `enum` |
| R11 | `commit_header` | `syntax`, `missing`, `enum`, `length`, `requires` |
| R13 | `transition_refused` | `state` |
| R14 | `identifier_format` | `syntax` |
| R14, R23 | `identifier_duplicate` | `duplicate` |
| R14 | `identifier_location` (warning) | `conflict` |
| R15 | `hierarchy_mismatch` | `conflict` |
| R16 | `schema_version_unknown` | `enum` |
| R17 | `approval_conflict` | `conflict` |
| R18 | `part_invalid` | `duplicate`, `missing`, `enum`, `conflict` |
| R19 | `initial_status` | `state` |
| R20 | `delta_conflict` | `conflict` |
| R22 | `handover_input` | `unknown_field`, and the reasons of `artifact_schema` |
| R26 | `artifact_location_missing` (warning) | `missing` |

Every artifact type and input kind of this specification has its schema built. A document of a
type whose schema the engine has not loaded is a failure of the engine, the internal error of
ADR 0009 naming the check `schema_<kind>` or `schema_<input_kind>` it could not run, never a
check listed as not built; one rule holds for artifacts and inputs (decided by the maintainer on
2026-11-25).

## Examples

`examples/` holds the normalised form of every type and input, named `<type>.valid.yaml` and
`<type>.invalid.yaml`, with `<type>.<case>.valid.yaml` and `<type>.<case>.invalid.yaml` for
further cases. The valid documents form one consistent project and are validated together,
against `policy.yaml`, without a ledger, so R17 is not checked. Each invalid document is
validated against the valid set; its first comment states the requirement it breaks and the
expected pointer and reason.

## Interfaces
- Schemas under `schemas/artifacts/<kind>/v<N>.json`, one per type, starting at `v1`; the
  hand-over input under `schemas/artifacts/handover/v<N>.input.json`; the inputs under
  `schemas/inputs/pull_request/v<N>.json` and `schemas/inputs/commit/v<N>.json`. All new.
- Every schema change, additive or not, is a new snapshot `v<N+1>.json`; released snapshots
  are immutable, which the pre-edit hook and CI enforce. The engine validates every known
  version and migrates it to the current one (R16). Clients accept enum values they do not
  know (ADR 0009 C5).
- Errors use the report of ADR 0009 C3 with the field `source` and the codes of the section
  Errors.
- Validation exposed by `wayloop validate` (SPEC-0004, #6); project structure, including the
  folders under `plan/`, created by `wayloop init` (#5).

## Dependencies
- This pull request merges after #218 and #219, which accept ADR 0006, ADR 0007 and ADR 0008.
- ADR 0007, ADR 0008 and ADR 0009 are Accepted: the location of hand-overs, the identifier rules and the error report follow them.
- ADR 0012, ADR 0014, ADR 0016 and ADR 0017 are Proposed; the gate issue, change deltas, the council report and the identifier in the structured block depend on them.
- SPEC-0004 R2 needs the same optional field `source` in its error report; this specification depends on that change.
- SPEC-0005 maps every story and every task to a sub-issue of its epic (R1). The tracker limits the number of sub-issues of one parent issue (a platform limit); SPEC-0005 must handle epics larger than that limit.
- The migration of the existing specifications of this repository to the Markdown form above.

## Plan and tasks
`plan.md` and `tasks.md` are written after the maintainer approves this specification
(`specs/README.md`, Agent workflow).

## Open questions
- [x] **Q1. Where do the council report and the brief live?** Decided by the maintainer on 2026-11-17: a repository file, plus a comment by the engine on the gate issue with the decisions and the questions; approval is given on the issue. See Where artifacts live and Gate issue.
- [x] **Q2. Identifiers of tracker items.** Decided by the maintainer on 2026-11-17 with ADR 0008 (#4): both identifiers, with different roles. See Identifiers.
- [x] **Q3. Is every task a tracker sub-issue?** Decided by the maintainer on 2026-11-17: yes, whatever its complexity; SPEC-0013 R5 and SPEC-0005 R1 are aligned. R9 and R10 hold unchanged.
- [x] **Q4. How is a collision of identifiers resolved?** Decided by the maintainer on 2026-11-17: (a), the branch that is not yet merged takes a new identifier before it merges.
- [x] **Q5. Are ADRs a checked kind?** Decided by the maintainer on 2026-11-17: (b), no new kind; a citation `ADR <NNNN>` with no ADR of that number gives a warning.
- [x] **Q6. How are `tokens` counted?** Decided by the maintainer on 2026-11-17: (b), the deterministic estimate computed by the engine.
