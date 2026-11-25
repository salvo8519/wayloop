# wayloop validate

`wayloop validate` checks a set of artifacts (briefs, council reports, tasks and the other
types of the artifact model) and inputs (pull requests, commits, hand-overs) against their
schemas, in one pass, and reports every error with the pointer of the faulty value and the
place where it is stored. It changes nothing.

Today briefs, council reports, gate issues, milestones, epics, user stories, living
specifications, change deltas, tasks, hand-over records and the inputs (pull requests, commits
and hand-over inputs) are validated against their schemas. A living specification written in
Markdown is first normalised into its document model (see Living specifications in Markdown
below). Across documents, identifiers are checked for uniqueness and against the path of their
file, the links between artifacts are resolved (see Links below), the parts of artifacts and the
operations of change deltas are checked (see Parts below), the titles of epics and tasks are
checked for their form and their uniqueness, and the branch names and commit headers of pull
requests and commits are checked against their grammar (see Branches and commit headers below);
the other checks across documents (statuses, the issue links of pull requests, areas) are not run
by this version and are listed in `diagnostics.not_done`.

This page describes version 1 of the command, as built from source. Its contract is the pair
of schema snapshots `schemas/commands/validate/v1.json` (input) and
`schemas/commands/validate/v1.output.json` (output), generated from the types of
`wayloop-schemas` (ADR 0030); the schema of each artifact type is
`schemas/artifacts/<kind>/v1.json`, for example `schemas/artifacts/task/v1.json`, and the schema
of each input is `schemas/inputs/<input_kind>/v1.json`, for example
`schemas/inputs/commit/v1.json`, except the hand-over input, whose schema is
`schemas/artifacts/handover/v1.input.json`. This page explains them.

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
`specs/_templates/`). A location reached through a symbolic link, or that is a file and not a
folder, is treated as absent, and links, special files and names that are not UTF-8 inside a
location are skipped. With paths it reads those files only, as one set: each path is relative
to the current folder, has no `..` segment and no `\`, and names a YAML file (`.yaml`) or a
Markdown file (`.md`), which is read as a living specification wherever it is (see Living
specifications in Markdown); a path whose file, once its links are resolved, is outside the
project or has another extension is refused as a usage error, and a path that is not a regular
file (a folder, a pipe, a device) cannot be read (exit code 3). A set of more than 10,000
files, discovered or named, is refused as a usage error; discovery stops listing at the first
file beyond that bound. Either way the files are read in the byte order of their paths, each
into one entry of `documents` with its path as its `file` origin and, for YAML, the line of
each value in `lines`; so every error gives the file and the line of the faulty value in
`source`.

Discovered or named, the command also gives in `adrs` the numbers of the ADRs of the project,
read from the names of the files directly in `docs/en/project/adr/`: a file
`<number>-<slug>.md` is the ADR of that number, where the number is a sequence number from
`0001` (four digits, or five or more without a leading zero) and the slug has at least one
character; any other file, such as `index.md`, is not an ADR and is ignored. Links, special
files and names that are not UTF-8 inside the folder are skipped, and a folder with no file of
an ADR gives `adrs: []`, so that every citation is warned. When the folder does not exist, is
a file or is reached through a symbolic link, `adrs` is not given and the check
`adr_citations` is not done. A folder with more than 10,000 files of ADRs is refused as a
usage error.

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
| `markdown` | Each living specification sent as Markdown is normalised into its document model, then checked as a document of the type `specification` (see Living specifications in Markdown below) |
| `document_kind` | Each document has a `kind` among `brief`, `council_report`, `gate`, `milestone`, `epic`, `story`, `specification`, `change`, `task` and `handover`, or is an input with `input_kind` |
| `schema_version` | The `schema_version` of each document is a version its type knows; a document of an earlier version is migrated to the current one after it passes the schema of its own version |
| `schema_<kind>`: `schema_brief`, `schema_change`, `schema_council_report`, `schema_epic`, `schema_gate`, `schema_handover`, `schema_milestone`, `schema_specification`, `schema_story`, `schema_task` | Each document of the type passes the schema of its version (`schemas/artifacts/<kind>/v1.json`): required members, unknown members, types, accepted values, lengths, formats, and the members a status, a verification or an operation requires (see below) |
| `schema_<input_kind>`: `schema_commit`, `schema_pull_request` | Each input of the type passes the schema of its version (`schemas/inputs/<input_kind>/v1.json`): required members, unknown members, types and lengths (see Pull request and commit inputs below) |
| `schema_handover_input` | Each hand-over input passes the input schema of its version (`schemas/artifacts/handover/v1.input.json`), as a hand-over record does (see below) |
| `identifier_format` | The `id` is in the canonical form of its type, for example `BRIEF-0001` |
| `identifier_duplicate` | No two artifacts of the set carry one identifier |
| `identifier_location` | The file name of a brief, a council report or a plan item, and the folder number of a living specification, name the `id` of the artifact; a warning otherwise |
| `links` | Each link of an artifact resolves to an artifact of the set, not superseded where that matters, each requirement it cites is in force, and a task or a story belongs to the milestone and the epic of its parents (see Links below) |
| `parts` | No part identifier is given twice in its list, a dissent names a decision of its report, a recommendation or an answer is one of the options of its question, a task depends neither on itself, nor on a superseded task, nor on a cycle, a change delta has at most one operation per requirement and no member its `op` refuses, and neither a task nor an acceptance criterion is verified by `none` beside a technique (see Parts below) |
| `adr_citations` | Each citation `ADR <NNNN>` in `related` of a living specification names an ADR of `adrs`; with `adrs` only |
| `title_format` | The title of each epic and task has no type prefix, no bracket tag at its start or end and no final full stop (see Titles of epics and tasks below) |
| `title_duplicate` | No two titles of epics and tasks, in any status except `superseded`, are equal after normalisation |
| `branch_name` | The `branch` of each pull request follows the grammar of branches, with a type of the policy (see Branches and commit headers below) |
| `commit_header` | The `header` of each commit and the `title` of each pull request follow the grammar of commit headers, with a type and a scope of the policy, and a commit whose header marks a breaking change has its paragraph |
| `artifact_locations` | Some folder of a location exists in the project the documents were discovered in; with `locations` only, after the checks above |

Each schema is one check, `schema_<kind>` or `schema_<input_kind>`, listed in
`diagnostics.checks` when it ran; every artifact type and input has its schema in this version.
A document of a type whose schema the engine has not loaded is a failure of the engine, the
error `internal_error` naming the check it could not run (exit code 5), never a check not run.
Any other check the engine did not run is never skipped silently:

| Check | Not run on |
| --- | --- |
| `statuses` | The set: transitions and initial statuses against `base` |
| `pull_request_link` | The set: the issue linked by the body of each pull request, and the issue of its branch |
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
options, not that a part is unique, that a dissent names a decision of its report, that a
recommendation or an answer is one of the options or that a change delta has one operation per
requirement: that is the check `parts` (see Parts below).

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
`locations` is not given: the documents were named or sent directly. `adr_citations` is listed
there with the same reason when `adrs` is not given: the input was sent directly without it,
or the project has no folder `docs/en/project/adr/`.

### Links

Every link is resolved by identifier through the set, never by path: a living specification
moved to another folder and renamed is still found. The links of an artifact are read only when
it has no error of its own, of its schema or of the normalisation of its Markdown, so that a
broken document causes no wave of link errors; such an artifact is still found as a target, as
long as its `id` raised no error. An item that lives only in the tracker is not in the set
until the tracker is read, so a link to it is missing.

| Type | Members that link |
| --- | --- |
| Council report | `brief` |
| Gate issue | `subject` |
| Milestone | `council_report` |
| Epic | `milestone` |
| User story | `epic`, `milestone`, `specification` |
| Living specification | `milestone`, `stories`; the `requirements` of each acceptance criterion name requirements of the same specification |
| Change delta | `specification`, `tasks`; the `requirement` of an operation `modified`, `removed` or `renamed` names a requirement of that specification |
| Task | `epic`, `milestone`, `story`, `specification` (each `spec` and its `requirements`), `depends_on`, `expanded_from` |
| Hand-over record | `task` |
| Every type | `superseded_by` |

- A link to an identifier that no artifact of the set carries is `link_missing`, reason
  `missing`, at the member of the link.
- A link to a superseded artifact is `link_superseded`, reason `state`. Its replacements are the
  identifiers of its `superseded_by`. With exactly one, that identifier is in `fix`, applicable
  as is, for example `"SPEC-0001"`, and `expected` lists it alone. With several, neither `fix`
  nor `expected` is given: no error copies a list that grows with the input, so one entry of
  `related` points to where the list is, `{document, pointer: "/superseded_by"}` of the target
  with the members of its `source`, and the message names only the target. When the target is
  the document of the error itself, as in a self-reference in its `superseded_by`, `pointers`
  holds `/superseded_by` instead, since `related` names only other documents. A `fix` is a value
  valid at its pointer, not a guarantee that the set becomes valid once it is applied.
- `superseded_by` names artifacts that are not superseded: a replacement that is itself
  superseded, as in a chain or a self-reference, is a broken link of its holder.
  `expanded_from` (the task a subtask was expanded from is superseded by it) and `depends_on`
  accept a superseded target, since they record the history of the work; a dependency on a
  superseded task is a rule of the check `parts`.
- The replacements never include the superseded target itself, nor the artifact that holds the
  link, which never names itself: `[holder, X]` and `[target, X]` give the fix `X`; when none is
  left, `expected`, `fix` and `related` are absent.
- A cited requirement that the specification does not have is `link_missing`; a removed one is
  `link_superseded`, reason `state`, with neither `expected` nor `fix`, since it has no
  replacement. Requirements are checked only when the specification is in the set and passed
  its schema.
- A task or a story whose `milestone` differs from that of its epic, or a task whose `epic`
  differs from that of its story, is `hierarchy_mismatch`, reason `conflict`, at `/milestone` or
  `/epic`, with the member of the parent (`/epic` or `/story`) in `pointers` and the value
  reached through the parent in `expected`.
- An artifact in status `done` or `superseded` is not changed again, and a hand-over record is
  never changed: each broken link of such an artifact is the warning `link_broken_terminal`,
  with the reason, `expected`, `fix` and `related` of the error it replaces, and the hierarchy
  is not checked.
- No other error of a link or of the hierarchy carries `related`: the pointer and the message
  name the target.
- A citation is the word `ADR` followed by a number, for example `ADR 0014`; `ADR 14` cites the
  same ADR. A bare number that follows a citation, in the same entry or in the entries right
  after it, cites an ADR too: `ADR 0002, 0003` cites two, also when the header row `Related`
  of a Markdown specification splits it into two entries. A word that opens a bracket ends the
  run: `ADR 0014 (2026)` cites 0014 only, while `(ADR 0008)` cites 0008. With `adrs`, a
  citation whose number is not listed is the warning `adr_citation_missing`, reason `missing`,
  at the entry of `related` that holds it.

This version does not check that a milestone links an approved council report, nor that the
task of a hand-over record carries its issue or that the issue is a task of the plan.

When the input itself is refused, or the command line is not understood, or the input cannot
be read, no check runs: `diagnostics.checks` and `diagnostics.not_done` are then empty.

### Parts

The parts of an artifact are read once it passed its schema. Each rule below refuses the value
at its pointer. The errors of this check come after those of the links and before those of the
titles, also the ones with the code `artifact_schema`. Only a dependency on a superseded task
with several replacements names another document in `related`, as a link does.

| Rule | Code | Reason | Pointer | Other members |
| --- | --- | --- | --- | --- |
| A part identifier given twice in its list: `decisions` and `questions` of a council report; `requirements`, `acceptance_criteria` and `open_questions` of a living specification; `acceptance_criteria` of a user story | `part_invalid` | `duplicate` | The `id` of each part after the first that carries it | `pointers`: the `id` of the first, then its own |
| A dissent that names a decision the report does not have | `part_invalid` | `missing` | `/dissent/<i>/decision` | `pointers`: `/decisions` |
| A recommendation or an answer that is not one of the options of its question | `part_invalid` | `enum` | `/questions/<i>/recommendation` or `/questions/<i>/answer` | `expected`: each option once, in its order; `pointers`: `/questions/<i>/options` |
| A task that depends on itself | `part_invalid` | `conflict` | `/depends_on/<i>` | `pointers`: `/id` |
| A task that depends on a superseded task | `part_invalid` | `conflict` | `/depends_on/<i>` | `expected`, `fix` or `related` as for a link to a superseded artifact (see Links) |
| A task that depends on a task that leads back to it, directly or through other tasks | `part_invalid` | `conflict` | `/depends_on/<i>` of each dependency on the cycle | none; no `related` |
| Two operations of a change delta on one requirement | `delta_conflict` | `conflict` | `/operations/<i>` of each operation after the first on that requirement | `pointers`: the first operation, then its own |
| A member that the `op` of an operation refuses: `requirement` on `added`, `text` on `removed` and `renamed` (a rename with a new text is one `modified` operation), `name` on `removed` | `artifact_schema` | `unknown_field` | `/operations/<i>/<member>` | `pointers`: `/operations/<i>/op` |
| A task or an acceptance criterion of a living specification whose `verification` holds `none` beside a technique | `artifact_schema` | `enum` | `/verification/<i>` or `/acceptance_criteria/<i>/verification/<j>` of each `none` | `expected`: the techniques; `pointers`: the first technique of the list |

The schema cannot state the absence of a member under a condition, so the members an `op`
refuses are refused by this check. A requirement refused on an `added` operation names no
requirement, so it conflicts with no other operation. The order of the verification values and
a repeated value carry no meaning: `[none, none]` and `[tdd, tdd]` pass.

The dependencies of a task are checked only when the task has no error of its own and is not
done or superseded, since such a task is not changed again; the dependencies of a task that is
done or superseded are not part of the graph of dependencies either. Every task of the set is a
target, as long as its `id` raised no error. Every dependency on a cycle is refused, not only
the one that closed it, since the order of the documents does not say which one did. A
dependency on a task that no artifact of the set carries is an error of the links only.

When a change delta is merged, an added requirement gets the identifier after the highest
number ever used in its living specification, removed requirements included: with `R1` to `R3`
and `R4` removed, it is `R5`. The engine gives that identifier with one function of its core;
the merge itself is not a command of this version.

### Living specifications in Markdown

A living specification is stored as `specs/<NNNN>-<capability>/spec.md`, in the layout of the
template `specs/_templates/spec.md`. Every Markdown file named on the command line is read as
a living specification, wherever it is; outside a folder `specs/<NNNN>-<capability>/` it has
no `capability`, which is then refused with the reason `missing`. The engine reads the file
line by line, without a Markdown parser, into the document model of the type `specification`,
then applies the schema; every error carries the line it was read from in `source`. The form
does not carry `kind` and `schema_version`: they are written as `specification` and the
current version, and listed in `meta.defaults_applied` with the index of the document in
`document`, the pointers `/kind` and `/schema_version` and the reason `implied_by_form`.
Headings, header rows and status words are English, whatever the language of the project;
prose is in the language of the project.

| Markdown | Member |
| --- | --- |
| `# SPEC-<seq>: <title>` | `id`, `title`; without a colon, the whole text is the title |
| Header rows, a table below the title | One row per member below; a row whose two cells are both empty or made only of `-`, `:` and spaces, such as the head and the separator of the table, is skipped |
| Folder `specs/<NNNN>-<capability>/` | `capability`; absent when the file is elsewhere or sent without a file |
| Header row `Status` | `status`, in lower case: `Draft` gives `draft` |
| Header row `Milestone` | `milestone`, the first word: `M0` of `M0 · Foundations` |
| Header rows `Owner`, `Commit scope`, `Language` | `owner`, `commit_scope`, `language`, as written |
| Header row `Related` | `related`, split at commas and semicolons |
| Header row `Superseded by` | `superseded_by`, split at commas |
| `## Problem` | `problem`, the text as written |
| `## Scope`, paragraphs `**In scope:**` and `**Out of scope:**` | `scope_in`, `scope_out`: one entry per list item, or the paragraph as one entry |
| `## User stories` | an item that is only a `STORY` identifier goes to `stories`, any other to `user_stories` |
| `## Requirements (EARS)`, items `- **R<k>**: <text>`, `- **R<k>** *<name>*: <text>`, `- **R<k>** (removed): <text>` | `requirements` |
| `## Acceptance criteria`, items `- **AC<k>** (R<i>, R<j>): <text> Verification: <words>. Rubric: <text>` | `acceptance_criteria`; `Rubric:` is optional |
| `## Interfaces` | `interfaces`, one entry per item |
| `## Open questions`, items `- [ ] <text>` and `- [x] <text>`, optionally starting with `**Q<k>. <question>**` | `open_questions` |
| Any other `##` section | `extra_sections`, `{heading, text}`, kept as prose and not validated |

A list item ends at the next item, the next heading or a blank line; its following lines, and
a line indented after a blank line, continue it and are joined with a space. A paragraph in a
section read as a list is one entry, so a requirement written without its `**R<k>**` is
refused at its `id`. A fenced block opens with three or more backticks or tildes and closes
only with a run of the same character at least as long, followed only by spaces; a line inside
it is never a title, a heading, a header row, an item or a marker of Scope.

A section whose heading is not in the table, a translated heading such as `## Requisiti`
included, is kept in `extra_sections`; a required member whose section is missing is then
refused with the reason `missing`, for example at `/requirements`. A section present but
empty gives an empty value, which the schema refuses when the member needs one.

The verification words are matched without regard to case in the text after `Verification:`
up to the first full stop, in the order they appear; other words are ignored, so
`property test for production, TDD for validation` gives `[property, tdd]`:

| Word | Value |
| --- | --- |
| `TDD` | `tdd` |
| `BDD`, `BDD scenario` | `bdd` |
| `end-to-end` | `end_to_end` |
| `property`, `property test` | `property` |
| `contract`, `contract test` | `contract` |
| `benchmark` | `benchmark` |
| `judge` | `judge` |
| `none` | `none` |

Four faults have no member in the document model and are refused by the normalisation, with
the code `artifact_schema` and their line: a verification text with no word of the table
(`enum` at `/acceptance_criteria/<i>/verification`, the words in `expected`; the schema then
reports nothing more at that member), a header row that is not one of the table or repeats one
(`unknown_field`, at the label as written, for example `/Stato`), a section of the template or
a paragraph of Scope given twice (`duplicate`, at the member it fills, `/scope_in` for the
section Scope and `/stories` for User stories, `/scope_in` or `/scope_out` for a repeated
`**In scope:**` or `**Out of scope:**`, with the line of the second heading or marker; its
content is not read), and text
outside the title, the header rows and the sections, or in the section Scope before its
paragraphs (`syntax`, at the root). Every other value is written as read and checked by the
schema at its member: text of a criterion after its verification words that does not start
with `Rubric:` is added to its `text`, and the root is at line 1, as for a YAML file, so a
missing member and `capability` carry line 1.

### Pull request and commit inputs

An input is not an artifact: it has no `kind`, no identifier and no status, and the
`input_kind` of its entry selects its schema. Like an artifact, it carries the `schema_version`
of its schema: it is checked against the schema of that version and migrated to the current
one, and an unknown version is refused with `schema_version_unknown`. A member of an artifact
such as `kind` is refused as `unknown_field`. A document with neither `kind` nor `input_kind`
is refused with the reason `missing` at `/kind`; the name of an input in `kind`, for example
`kind: commit`, is refused with the reason `enum`, since `kind` names artifact types only. The
command line reads stored artifacts only; inputs are sent with `--input`.

| Input | Member | Type | Required |
| --- | --- | --- | --- |
| `pull_request`, `commit` | `schema_version` | integer, the version of the schema, today `1` | yes |
| `pull_request` | `title` | string of at least one character, the pull request title | yes |
| `pull_request` | `body` | string, where the issue is linked; may be empty | yes |
| `pull_request` | `branch` | name of at least one character, the head branch as git spells it; not prose | yes |
| `pull_request` | `labels` | list of names, each of at least one character, as the tracker spells them; not prose; `[]` means none | yes |
| `commit` | `header` | string of at least one character, the first line of the message | yes |
| `commit` | `body` | string of at least one character, the lines after the blank line, trailers included; absent when the message has one line | no |

An empty text where one is required is refused with the reason `length`. The schema checks the
shape only. The grammar of a branch name and of a commit header, and the issue links of a pull
request, are later checks: the grammar is the checks `branch_name` and `commit_header` (see
Branches and commit headers below), and the links are the check `pull_request_link`, which this
version does not run.

### Titles of epics and tasks

The title of an epic or a task names the work, without its type and without the markers of a
tracker. Its schema gives its length, at most 120 characters; the checks `title_format` and
`title_duplicate` read every epic and task whose `title` raised no error and is within that
length, whatever its other errors.

| Rule | Example refused | `fix` |
| --- | --- | --- |
| No type prefix: a leading `<word>:`, `<word>(<anything>):`, `<word>!:` or `<word>(<anything>)!:` where `<word>` is a type of the policy, without regard to case | `feat(core)!: Record time entries` | `Record time entries` |
| No bracket tag `[...]` at the start or at the end, closed by its matching bracket, nested brackets counted; an unbalanced group is no tag | `[EPIC] Monthly export of time entries` | `Monthly export of time entries` |
| No final full stop | `Monthly export of time entries.` | `Monthly export of time entries` |

A word that is not a type of the policy is not a prefix: `wayloop init creates a project
structure that validates` passes. Each rule broken is one error `title_format`, in the order of
the table, and each `fix` is the title with every rule applied, so a rule revealed by removing
another counts too: `[EPIC] feat: Export.` gives three errors whose `fix` is `Export`. When
nothing is left of the title, the error has no `fix`.

Two titles are equal when they are equal after Unicode NFC normalisation, case folding, NFC
normalisation again, trimming and collapsing every run of white space into one space: `Export of
the café entries` and ` EXPORT of the  CAFÉ entries` are one title. Epic and task titles are
compared with each other, in every status except `superseded`. Case folding is the full folding
of Unicode: the sharp s `ß` equals `ss`, and the final sigma equals sigma.

### Branches and commit headers

The `branch` of a pull request is checked by `branch_name`, and the `header` of a commit and the
`title` of a pull request by `commit_header`, each when it is a string that raised no error,
whatever the other errors of its document. The types and scopes are those of the policy.

| Form of a branch | Rule |
| --- | --- |
| `<type>/<issue>-<slug>` | `<type>` a type of the policy; `<issue>` a number from 1 without a leading zero; `<slug>` one to five words of lower-case ASCII letters and digits joined by single hyphens |
| `sprint/S<NN>/<issue>-<slug>` | as above, with `S` and two digits or more, the number of the sprint |
| `dependabot/<anything>`, `release/v<anything>`, `revert-<n>-<anything>` | exempt: accepted without further checks |

A type outside the policy is the reason `enum`, with the types in `expected`; a slug of more
than five words is the reason `length`; any other fault of the form is the reason `syntax`. The
errors of the slug carry in `fix` the branch with its slug rewritten as a produced branch name
(below), when the type and the issue are valid and the slug leaves a word: `feat/42-Ledger`
gives `feat/42-ledger`, `feat/42-one-two-three-four-five-six` gives
`feat/42-one-two-three-four-five`.

A header is `<type>(<scope>)[!]: <subject>`, optionally followed by a reference
` [SPEC-<seq>]` or ` [SPEC-<seq> R<k>, R<k>]`:

| Rule | Example refused | Reason | `fix` |
| --- | --- | --- | --- |
| The form `<type>(<scope>)[!]: <subject>`, one space after the colon | `feat(core):add x` | `syntax` | none |
| The type is a type of the policy | `feature(core): add x` | `enum`, the types in `expected` | none |
| A scope is required, except for the type `revert` | `feat: add x` | `missing`, the scopes in `expected` | none |
| The scope is a scope of the policy | `feat(kernel): add x` | `enum`, the scopes in `expected` | none |
| The subject starts with a lower-case letter | `feat(core): Add the ledger` | `syntax` | `feat(core): add the ledger` |
| The subject has no final full stop, before the reference | `feat(core): add the ledger.` | `syntax` | `feat(core): add the ledger` |
| At most 72 characters, reference included | | `length` | none |

The `fix` of the subject lowers its first letter when it starts a capitalised word, not an
acronym, and removes its final full stops; it is given only when the corrected header breaks no
rule. A commit whose header marks a breaking change with `!` needs a paragraph of its body that
starts with `BREAKING CHANGE:`; without one, or without a body, it is refused with the reason
`requires` at `/body`, the header in `pointers`. A pull request title needs no such paragraph:
its evidence is the label `breaking-change`.

A produced branch name is `<type>/<issue>-<slug>`: the slug is the title compared as titles are
(Unicode NFC, case folding, NFC again), without its accents, with every run of other characters
turned into one hyphen, cut after five words, and `work` when nothing is left: `Città di
Salerno` gives `citta-di-salerno`, `Straße` gives `strasse`. A produced name always passes
`branch_name`.

## Input

The canonical input is one JSON object. A member set to `null` is not given, exactly as an
absent member. The input schema is closed: a member it does not list is refused.

| Member | Type | Required | Meaning |
| --- | --- | --- | --- |
| `documents` | list of objects | yes | One entry per stored artifact or input; `[]` means none |
| `base` | list of `{id, status}` | no | Statuses on the default branch of the artifacts of the set |
| `policy` | object `{types, scopes, areas}`, each a list of names of the policy, identifiers of at least one character | no | The subset of the project policy the model reads. The checks `title_format` and `branch_name` read its `types`, the check `commit_header` its `types` and `scopes`; the check `areas` is listed as not done. Absent: the default policy applies, the policy of the Wayloop repository, and `meta.defaults_applied` lists it when the set holds an epic, a task, a pull request or a commit, whose checks read it |
| `adrs` | list of integers from 1 | no | Numbers of the ADRs of the project, for the check `adr_citations`. Absent: citations are not checked, and the check is not done. The command line fills it from the file names of `docs/en/project/adr/` when that folder exists |
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
  "current_command_version", "message": "No version of the command was requested: version 1
  runs."}`: the pointer is empty because the version is a setting of the
  context, not a member of the input. When `--command-version` names a version the engine
  does not run, the refusal has the pointer `""` too, and `meta.command_version` is the current
  version, whose shape the envelope has; when the input is also text the engine cannot read as
  JSON, the same report lists the version first, then the error of the text. A usage error
  reports the current version of the command it names, and 1 when it names no known command.
- When the input has no `policy` and the set holds an epic, a task, a pull request or a
  commit, the default policy follows the default of the command version: `{"pointer":
  "/policy", "value": {"types": [...], "scopes": [...], "areas": [...]}, "reason":
  "default_policy", "message": ...}`, with no `document`, since it is a default of the input
  itself. Its types and scopes are those of the
  [conventions](../project/conventions.md#commits) of the Wayloop repository, and its areas
  those of its `area:` labels.
- The defaults of the documents follow the defaults of the command version and the policy, in
  the order of the documents, then in the byte order of their pointers. Each carries `document`,
  the index of its document from 0, as an error does, and its `pointer` is relative to that
  document: a living specification sent as Markdown gives `{"pointer": "/kind", "value":
  "specification", "reason": "implied_by_form", "message": "The Markdown form of a living
  specification does not carry `kind`.", "document": <i>}` and the same for `/schema_version`
  with the current version. The wording of `message` is not part of the contract.
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
2. Otherwise, the errors of the documents, in the order of the documents; within a document, by
   stage (kind, version and schema first, then identifier, then the identifiers across the set,
   then the links, then the parts, then the form of the title and the titles across the set);
   within a stage, in the order of the schema pass: the members of an object in the byte order
   of their keys (`/aa` before `/zz`), the elements of an array by index, absent members after
   present ones, in the order in which the schema lists them as required. An error of the code
   `artifact_schema` found by the check `parts` comes in the stage of the parts. The errors of
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

An error whose condition involves other documents, such as two artifacts with one identifier,
names one of them in `related`, so that no error copies a list that grows with the input:
`{document, pointer}`, with the members of the `source` of that document (`file` and `line`,
`issue` or `milestone`) in the entry itself when it was stored. They are not nested in a member
`source`, so that the envelope keeps at most four levels of objects (ADR 0009 C4). `related` is
absent when there is no other document, never `[]`.

| Code | Reasons | Exit code | Meaning |
| --- | --- | --- | --- |
| `command_input` | `syntax`, `duplicate`, `missing`, `unknown_field`, `type`, `enum`, `length`, `range`, `requires`, `conflict` | 1 | The canonical input is refused: it is not JSON, repeats a key, holds a number beyond a double, or breaks its schema or a rule of the section Input; or `--command-version` requests a version the engine does not run (`enum`, the known versions in `expected`) |
| `artifact_schema` | `syntax`, `missing`, `unknown_field`, `type`, `enum`, `length`, `range`, `requires`, `duplicate` | 1 | A document breaks the schema of its type or of its input (a pull request, a commit); also a document without a `kind`, or with a `kind` outside the list (`enum`, with the types in `expected`), a `schema_version` that is missing, not an integer (reported with every other error against the current version) or beyond plus or minus (2^53 - 1) (`range`), a file that did not parse (`syntax`, at the pointer of the whole document, with its line in `source`), the faults of a living specification in Markdown, among them a section given twice (`duplicate`; see Living specifications in Markdown), a member the `op` of an operation refuses (`unknown_field`) and `none` beside a technique in the `verification` of a task or of an acceptance criterion (`enum`; see Parts) |
| `handover_input` | `syntax`, `missing`, `unknown_field`, `type`, `enum`, `length`, `range`, `requires` | 1 | A hand-over input breaks its schema, for example by setting a member that only the engine sets (`unknown_field`) |
| `schema_version_unknown` | `enum` | 1 | The `schema_version` is not a version of the type or of the input; `expected` lists the known versions |
| `identifier_format` | `syntax` | 1 | The `id` is not in the canonical form of its type; `fix` gives the corrected identifier when one can be derived |
| `identifier_duplicate` | `duplicate` | 1 | Two artifacts or more of the set carry one identifier: each is refused at `/id`, and names one other in `related`, the first the second, each later one the first. Every document of a known `kind` whose `id` raised no error is compared, whatever its other errors, so the duplicates come in the same pass |
| `link_missing` | `missing` | 1 | A link names an identifier no artifact of the set carries, or a requirement its specification does not have (see Links) |
| `link_superseded` | `state` | 1 | A link names a superseded artifact: with exactly one replacement, it is in `expected` and `fix`; with several, `related` names the `superseded_by` of the target; or a removed requirement, with neither |
| `hierarchy_mismatch` | `conflict` | 1 | A task or a story names a milestone or an epic other than the one reached through its parent |
| `part_invalid` | `duplicate`, `missing`, `enum`, `conflict` | 1 | A part identifier given twice in its list, a dissent naming no decision of its report, a recommendation or an answer outside the options of its question, or a dependency of a task on itself, on a superseded task or on a cycle (see Parts) |
| `delta_conflict` | `conflict` | 1 | Two operations of a change delta or more on one requirement: each after the first is refused, with the first in `pointers` |
| `title_format` | `syntax` | 1 | The title of an epic or a task has a type prefix, a bracket tag at its start or end, or a final full stop: one error per rule at `/title`, each with `fix` the title with every rule applied, when anything is left |
| `title_duplicate` | `duplicate` | 1 | Two epic or task titles or more, in any status except `superseded`, are equal after normalisation: each is refused at `/title`, and names one other in `related`, the first the second, each later one the first |
| `branch_name` | `syntax`, `length`, `enum` | 1 | The `branch` of a pull request breaks the grammar of branches: its form (`syntax`), a slug of more than five words (`length`), or a type outside the policy (`enum`, with the types in `expected`); see Branches and commit headers |
| `commit_header` | `syntax`, `missing`, `enum`, `length`, `requires` | 1 | The `header` of a commit or the `title` of a pull request breaks the grammar of headers: its form or its subject (`syntax`), no scope (`missing`), a type or a scope outside the policy (`enum`), more than 72 characters (`length`); or a commit marks a breaking change without its paragraph (`requires` at `/body`); see Branches and commit headers |
| `usage` | `syntax` | 2 | The command line is not understood; `command` is the command named, empty when none |
| `input_unreadable` | `missing`, `state` | 3 | The input file, a file named or a folder of the project does not exist (`missing`) or cannot be read (`state`); the envelope holds no path |
| `internal_error` | `internal` | 5 | The engine could not compute the result; the input may be valid |

Warnings are listed in `diagnostics.warnings`, in the shape of an error; the result is not
refused for them and the exit code does not change. They come in a fixed order: the warnings
`identifier_location` in the order of the documents, then `link_broken_terminal`, then
`adr_citation_missing`, each in the order of the documents, then `artifact_location_missing`.

| Code | Reason | Pointer | Meaning |
| --- | --- | --- | --- |
| `identifier_location` | `conflict` | `/id` | The artifact was read from `plan/briefs/<name>.yaml`, `plan/council/<name>.yaml`, `plan/items/<name>.yaml` or `specs/<number>-<capability>/spec.md` (`<number>` of ASCII digits, in any form), and `<name>`, or `SPEC-<number>`, differs from its `id`; `fix` is the `id`, which is the identity. A document sent directly, or read from another path, gives none |
| `link_broken_terminal` | `missing`, `state` | The member of the link | A link of an artifact that is done or superseded, or of a hand-over record, is broken, as `link_missing` or `link_superseded` would say for another artifact, with the same `expected`, `fix` and `related` |
| `adr_citation_missing` | `missing` | `/related/<i>` | A living specification cites an ADR whose number is not in `adrs` |
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
