# wayloop validate

`wayloop validate` checks a set of artifacts (briefs, council reports, tasks and the other
types of the artifact model) and inputs (pull requests, commits, hand-overs) against their
schemas, in one pass, and reports every error with the pointer of the faulty value and the
place where it is stored. It changes nothing.

Today briefs, council reports, gate issues, milestones, epics, user stories, living
specifications in normalised form, change deltas, tasks, hand-over records and hand-over inputs
are validated against their schemas. A living specification sent as Markdown, a pull request
and a commit are accepted unchecked, and the checks not run are listed in
`diagnostics.not_done`; so are the checks across documents
(identifiers, links, parts, statuses, naming, areas).

This page describes version 1 of the command, as built from source. Its contract is the pair
of schema snapshots `schemas/commands/validate/v1.json` (input) and
`schemas/commands/validate/v1.output.json` (output), generated from the types of
`wayloop-schemas` (ADR 0030); the schema of each artifact type is
`schemas/artifacts/<kind>/v1.json`, for example `schemas/artifacts/task/v1.json`, and the schema
of the hand-over input is `schemas/artifacts/handover/v1.input.json`. This page explains them.

## Usage

```console
$ wayloop validate [<path>...] [--command-version <n>]
$ wayloop validate --input <file> [--command-version <n>]
$ wayloop validate --input - [--command-version <n>]
```

Without `--input`, the command reads the project in the current folder, which must be the root
of the project. It gives the folders of the locations it found in `locations`; run from a
subfolder, it finds none and validates an empty set, with exit code 0 and the warning
`artifact_location_missing`. With no path it reads every stored artifact: `plan/briefs/*.yaml`,
`plan/council/*.yaml`, `plan/items/*.yaml` and `specs/<NNNN>-<capability>/spec.md` (so not
`specs/_templates/`). A location reached through a symbolic link is treated as absent, and
links, special files and names that are not UTF-8 inside a location are skipped. With paths it
reads those files only, as one set: each path is relative to the current folder, has no `..`
segment and no `\`, and names a YAML file (`.yaml`) or a Markdown file (`.md`); a path whose
file, once its links are resolved, is outside the project or has another extension is refused
as a usage error, and a path that is not a regular file (a folder, a pipe, a device) cannot be
read (exit code 3). A set of more than 10,000 files, discovered or named, is refused as a usage
error. Either way the files are read in the byte order of their paths, each into one entry of
`documents` with its path as its `file` origin and, for YAML, the line of each value in
`lines`; so every error gives the file and the line of the faulty value in `source`.

YAML files are read with the YAML 1.2 core schema: `no`, `on`, `yes` and dates such as
`2026-11-17` are strings, and a quoted value is always a string. A file is refused, as an
`unreadable` entry with the line of the fault (code `artifact_schema`, reason `syntax`), when it
does not parse, gives a key twice in one mapping, holds more than one document or a document
that is not a mapping, holds a number JSON cannot hold (`.inf`, `.nan`, `1e400`) or a tag
outside the core schema, is not UTF-8, is larger than 1 MiB, nests values deeper than 64
levels, or would take more than 8 MiB of counted bytes once parsed. Counted bytes are an
estimate of memory: the text of the file, the pointers of its lines, a fixed 128 bytes for
each value and each line, every value its aliases repeat and every copy kept for an anchor. All
the files of one set may take at most 32 MiB of counted bytes, the bytes read and the
documents built; a file beyond that is refused the same way. Such a file is not loaded, so an
alias bomb or a large value repeated by aliases is refused before it is built, and the envelope
is always printed. A message quotes at most 80 characters of a key or a value.

`--input` names a file holding the canonical input as JSON, or `-` for standard input; it takes
no path. `--command-version` requests a version of the command; without it the current version
runs, and `meta.defaults_applied` says so. The command prints one envelope on one line of
standard output, exactly as the engine wrote it, and exits with the exit code of the envelope.
It prints an envelope also when the command line is not understood, a path outside the project
included, and when the input or a file cannot be read; the message for a person, with the
path, goes to standard error.

## What is checked

| Check | What it does |
| --- | --- |
| `document_kind` | Each document has a `kind` among `brief`, `council_report`, `gate`, `milestone`, `epic`, `story`, `specification`, `change`, `task` and `handover`, or is an input with `input_kind` |
| `schema_version` | The `schema_version` of each document is a version its type knows; a document of an earlier version is migrated to the current one after it passes the schema of its own version |
| `schema_<kind>`: `schema_brief`, `schema_change`, `schema_council_report`, `schema_epic`, `schema_gate`, `schema_handover`, `schema_milestone`, `schema_specification`, `schema_story`, `schema_task` | Each document of the type passes the schema of its version (`schemas/artifacts/<kind>/v1.json`): required members, unknown members, types, accepted values, lengths, formats, and the members a status, a verification or an operation requires (see below) |
| `schema_handover_input` | Each hand-over input passes the input schema of its version (`schemas/artifacts/handover/v1.input.json`), as a hand-over record does (see below) |
| `identifier_format` | The `id` is in the canonical form of its type, for example `BRIEF-0001` |
| `artifact_locations` | Some folder of a location exists in the project the documents were discovered in; with `locations` only, after the checks above |

Each schema is one check, `schema_<kind>`: in `diagnostics.checks` when it ran, in
`diagnostics.not_done` with the reason `not_built` when it is not in this version. A check the
engine did not run is never skipped silently:

| Check | Not run on |
| --- | --- |
| `schema_<input_kind>`: `schema_pull_request`, `schema_commit` | An input whose schema is not in this version |
| `markdown` | A living specification sent as Markdown |
| `identifier_duplicate` | The set: two artifacts with one identifier |
| `links` | The set: links between artifacts, hierarchy, ADR citations |
| `parts` | The set: parts of artifacts and operations of change deltas |
| `statuses` | The set: transitions and initial statuses against `base` |
| `naming` | The set: titles, branch names, commit headers and pull request links |
| `areas` | The set: the `area` of each epic and task against the areas of the project policy |

A schema also states the members that a status, a verification or an operation requires; a
document without one is refused with the reason `requires`, the member missing in `pointer` and
the member of the condition in `pointers`:

| Type | Member | Required when |
| --- | --- | --- |
| Every type | `superseded_by` | `status` is `superseded` |
| Gate issue | `approved_by`, `issue` | `status` is `done` |
| Milestone | `tracker_number` | `status` is `implementing` or `done` |
| Epic, user story, task | `issue` | `status` is `implementing` or `done` |
| User story | `specification` | `status` is `approved`, `implementing` or `done` |
| Task | `rubric` | `verification` contains `judge` |
| Living specification, each of `acceptance_criteria` | `rubric` | its `verification` contains `judge` |
| Change delta, each of `operations` | `requirement` | its `op` is `modified`, `removed` or `renamed` |
| Change delta, each of `operations` | `text` | its `op` is `added` or `modified` |
| Change delta, each of `operations` | `name` | its `op` is `renamed` |

A task or a user story without its epic, or an epic, a story or a task without its milestone,
gets one error, `missing` at `/epic` or `/milestone`, and no error of a link. The schema
checks the form of a link (for example `EPIC-0001`), not that its target exists: that is the
check `links`. The subject of a gate issue is a brief or a council report, in the form
`BRIEF-<seq>` or `COUNCIL-<seq>`. `blocked` is not a status of any artifact: it is a state of
the issue cycle.

Each type accepts only its own statuses. A council report has `draft`, `approved` and
`superseded`; a change delta `draft`, `approved`, `implementing`, `done` and `superseded`; a
living specification has six, `draft`, `approved`, `surveyed`, `implementing`, `done` and
`superseded`, and any other, for example `published`, is refused with `enum` and the six in
`expected`. The `capability` of a living specification is lower-case words of `a` to `z` and
`0` to `9` joined by single hyphens, for example `time-entries`; a requirement is `active` or
`removed`, and absent means `active`. The schema checks the form of the parts of an artifact
(`D<k>`, `Q<k>`, `R<k>`, `AC<k>`) and that a question of a council report has at least two
options, not that a part is unique, that a dissent names a decision of its report, that an
answer is one of the options or that a change delta has one operation per requirement: that is
the check `parts`.

A hand-over comes in two forms of one version. The input is what an agent submits at the end of
a session, with `input_kind: handover_input` and no `kind`: `schema_version`, `step`,
`files_touched` (project paths), `not_done`, `open_questions`, `rulings` (each `{decision,
reason, cost_if_wrong}`) and `next_step`, all required, the lists possibly `[]`, and
`language`, optional, as in every artifact. The record is what the engine writes from it: the
same members, with `kind: handover`, `id` (`HANDOVER-<issue>-<k>`), `issue`, `task`
(optional), `session`, `holder`, `executor`, `branch`, `recorded_at` (a UTC date-time with `Z`)
and `tokens` (an integer from 1). A member of the engine in an input, for example `tokens`, is
refused with `unknown_field`. Every error of an input has the code `handover_input`; the errors
of a record have the code `artifact_schema`. `step` is a step of the issue cycle, spelt in
`snake_case`, as printed by [`wayloop steps`](index.md#commands-of-a-build-from-source). A
hand-over has no title and no status; the summary of an input in `data.documents` holds its
`input_kind` and, when known, its `origin`; no `kind`, `id` or `status`.

`artifact_locations` is listed in `diagnostics.not_done` with the reason `input_absent` when
`locations` is not given: the documents were named or sent directly.

When the input itself is refused, or the command line is not understood, or the input cannot
be read, no check runs: `diagnostics.checks` and `diagnostics.not_done` are then empty.

## Input

The canonical input is one JSON object. A member set to `null` is not given, exactly as an
absent member. The input schema is closed: a member it does not list is refused.

| Member | Type | Required | Meaning |
| --- | --- | --- | --- |
| `documents` | list of objects | yes | One entry per stored artifact or input; `[]` means none |
| `base` | list of `{id, status}` | no | Statuses on the default branch of the artifacts of the set |
| `policy` | object `{types, scopes, areas}`, each a list of names of the policy, identifiers of at least one character | no | The subset of the project policy the model reads. No check of this version reads it: the check `areas` is listed as not done |
| `adrs` | list of integers from 1 | no | Numbers of the ADRs of the project |
| `locations` | list of `plan_briefs`, `plan_council`, `plan_items`, `specs`, for the folders `plan/briefs`, `plan/council`, `plan/items` and `specs` | no | The folders of the locations that exist in the project, when the documents were discovered there; their order and a repeated folder have no meaning, they are not compared with the `file` origins of the documents, and `[]` means none, which gives the warning `artifact_location_missing`. Absent: the documents were named or sent directly, and the check `artifact_locations` is not done |
| `language` | BCP 47 tag, for example `it` or `en-GB` | no | The language of the project. No check of this version reads it, so no default is applied or reported when it is absent; a check that needs it lists the default in `meta.defaults_applied` |

An entry of `documents`:

| Member | Type | Required | Meaning |
| --- | --- | --- | --- |
| `format` | `document`, `markdown` or `unreadable` | yes | `document`: a JSON value in the normalised form of its type; `markdown`: the text of a living specification; `unreadable`: a file that did not parse |
| `document` | object | with `document` | The document |
| `lines` | object | no; with `document` and a `file` origin only | The line, from 1, of each value of the document, keyed by its RFC 6901 pointer into the document; the schema states both rules: keys in the format `json-pointer`, values integers from 1. A line set to null is not given, as any member (ADR 0009 C2); a validator of plain draft 2020-12 refuses it |
| `text` | string | with `markdown` | The content of the file |
| `error` | object `{line, message}` | with `unreadable` | Where the parser stopped, from line 1, and its message |
| `input_kind` | `pull_request`, `commit` or `handover_input` | no; with `document` only | The type of an input, which has no `kind` member |
| `origin` | object, exactly one of `{file}`, `{issue}`, `{milestone}` | no; `{file}` with `unreadable` | Where the document was read; absent for a document sent directly |

`origin` has no member naming its form: the member present tells it, as for `source` in the
errors. This departs from the letter of ADR 0009 C4 (a choice is a kind plus fields) on
purpose, so that the input and the error report spell a place the same way. `file` is a path
relative to the root of the project in one spelling: segments joined by `/`, none of them
empty, `.` or `..`; no leading `/`, no `\` and no drive letter, so that it never leaves the
project and names a file one way only.

The command refuses an entry that gives a member of another format (for example `text` with
the format `document`), `lines` or the format `unreadable` without a `file` origin, an
`origin` with no member or more than one, a key of `lines` that is not an RFC 6901 pointer
(`syntax`, at the pointer of the member, with a message about the name of the member), and a
line that is not an integer from 1 (`type` or `range`).

The JSON text of the input must not repeat a key in one object: JSON parsers keep one of the
two values, so the engine refuses the text with the reason `duplicate` instead of choosing. A
number beyond the range of a double, such as `1e400`, is refused with the reason `range`.

## Output

The envelope of [ADR 0009](../project/adr/0009-contract-guidelines.md), version 1 of
`envelope/result`, described in [Architecture](../concepts/architecture.md#the-envelope). When
the set is accepted, `data` is:

| Member | Meaning |
| --- | --- |
| `documents` | One summary per entry of the input, in its order: `kind`, `input_kind`, `id`, `status` (after migration) and `origin`, each left out when the document does not give it as a string |

The output schema is open (ADR 0009 C5, amended on 2026-11-20): a client ignores members it
does not know and accepts enum values it does not know.

- `diagnostics.checks` lists the checks run and `diagnostics.not_done` the checks not run.
- `meta.command_version` is the version that ran. Without `--command-version`,
  `meta.defaults_applied` lists `{"pointer": "", "value": 1, "reason":
  "current_command_version"}`: the pointer is empty because the version is a setting of the
  context, not a member of the input. When `--command-version` names a version the engine
  does not run, the refusal has the pointer `""` too, and `meta.command_version` is the current
  version, whose shape the envelope has; when the input is also text the engine cannot read as
  JSON, the same report lists the version first, then the error of the text. A usage error
  reports the current version of the command it names, and 1 when it names no known command.
- `meta.input_hash` is the SHA-256 of the canonical JSON (RFC 8785) of the input without its
  members set to null, so an input with `"base": null` has the hash of the input without
  `base`. For text the engine refuses to read as JSON (not JSON, a key repeated in one object,
  a number beyond the range of a double), it is the SHA-256 of the bytes received; two channels
  that send the same text with other line endings or a byte order mark then get different
  hashes. When no input was read (usage, unreadable file), it is the SHA-256 of no bytes.

## Errors

Every error found is listed in `error.errors`; the first one is repeated at the top level of
`error`. The order is fixed:

1. The errors of the input itself (code `command_input`) come first and stop the command: no
   document is checked. They follow the schema pass of the input schema, then the entries in
   order for the rules the schema cannot state.
2. Otherwise, the errors of the documents, in the order of the documents; within a document,
   by stage (kind, version and schema first, then identifier); within a stage, in the order of
   the schema pass: the members of an object in the byte order of their keys (`/aa` before
   `/zz`), the elements of an array by index, absent members after present ones. The errors of
   the name of a member come before the errors of its value, at the same pointer.

Every `pointer` of the result is an RFC 6901 pointer; the schemas of the output and of the
envelope give it the format `json-pointer`, which a JSON Schema validator checks when format
assertion is enabled (in draft 2020-12 a format is an annotation by default). The `pointer` of
an error of the input points into the canonical input, for example `/documents/0/format`. The
`pointer` of an error of a document points into that document, for example `/problem`;
`document` gives the index, from 0, of that document in `documents`, and `source` says where
it is stored:

| Origin of the document | `source` |
| --- | --- |
| `{file}` | `{file, line}`: the line given in `lines` for the pointer of the error, or for the nearest value that contains it; `{file}` alone when `lines` gives neither, never a guessed line |
| `{issue}` | `{issue}` |
| `{milestone}` | `{milestone}` |
| absent | absent |

| Code | Reasons | Exit code | Meaning |
| --- | --- | --- | --- |
| `command_input` | `syntax`, `duplicate`, `missing`, `unknown_field`, `type`, `enum`, `length`, `range`, `requires`, `conflict` | 1 | The canonical input is refused: it is not JSON, repeats a key, holds a number beyond a double, or breaks its schema or a rule of the section Input; or `--command-version` requests a version the engine does not run (`enum`, the known versions in `expected`) |
| `artifact_schema` | `syntax`, `missing`, `unknown_field`, `type`, `enum`, `length`, `range`, `requires` | 1 | A document breaks the schema of its type; also a document without a `kind`, or with a `kind` outside the list (`enum`, with the types in `expected`), a `schema_version` that is missing, not an integer (reported with every other error against the current version) or beyond plus or minus (2^53 - 1) (`range`), and a file that did not parse (`syntax`, at the pointer of the whole document, with its line in `source`) |
| `handover_input` | `syntax`, `missing`, `unknown_field`, `type`, `enum`, `length`, `range`, `requires` | 1 | A hand-over input breaks its schema, for example by setting a member that only the engine sets (`unknown_field`) |
| `schema_version_unknown` | `enum` | 1 | The `schema_version` is not a version of the type or of the input; `expected` lists the known versions |
| `identifier_format` | `syntax` | 1 | The `id` is not in the canonical form of its type; `fix` gives the corrected identifier when one can be derived |
| `usage` | `syntax` | 2 | The command line is not understood; `command` is the command named, empty when none |
| `input_unreadable` | `missing`, `state` | 3 | The input file, a file named or a folder of the project does not exist (`missing`) or cannot be read (`state`); the envelope holds no path |
| `internal_error` | `internal` | 5 | The engine could not compute the result; the input may be valid |

Warnings are listed in `diagnostics.warnings`, in the shape of an error; the result is not
refused for them and the exit code does not change:

| Code | Reason | Pointer | Meaning |
| --- | --- | --- | --- |
| `artifact_location_missing` | `missing` | `/locations` | No folder of a location exists in the project where the documents were discovered, for example because the command ran in a subfolder of the project |

An unknown member gets the closest member, when one is close, in `fix`: for example `problem`
for `problema`.

## Exit codes

| Exit code | When |
| --- | --- |
| 0 | The set is accepted: the envelope has `data` |
| 1 | The input or a document is refused: the envelope has `error` |
| 2 | The command line is not understood, for example `--input` without a value, or a path outside the project: the envelope has `error` with the code `usage` |
| 3 | The input file, a file named or a folder of the project cannot be read (the envelope has `error` with the code `input_unreadable`), or standard output cannot be written (no envelope can be printed) |
| 5 | The engine failed: the envelope has `error` with the reason `internal` |

## Example

A brief that writes its problem under the Italian key `problema`, sent directly:

```json
{"documents": [{"format": "document", "document": {
  "kind": "brief", "schema_version": 1, "id": "BRIEF-0003",
  "title": "Freelancers record their working time from the terminal",
  "status": "draft", "origin": "light_requirements", "owner": "@example-owner",
  "problema": "Freelancers lose billable hours because recording time breaks their flow.",
  "goals": ["Record a time entry with one command"],
  "users": ["Freelance developers"], "scope_in": ["Time entries"]}}]}
```

The command exits with code 1. The error report holds two errors of the document at index 0:
the unknown key, with its fix, then the missing one:

```json
{"code": "artifact_schema", "pointer": "/problema", "reason": "unknown_field", "fix": "problem", "document": 0}
{"code": "artifact_schema", "pointer": "/problem", "reason": "missing", "document": 0}
```
