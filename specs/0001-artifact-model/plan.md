# SPEC-0001: Plan

| | |
| --- | --- |
| Status | Draft |
| Specification | SPEC-0001, Approved |
| Issues | #6 (`wayloop validate`), #160 (naming and linking rules), #5 (`wayloop init`) |
| Prerequisites | #228 (snake_case step identifiers), #229 (existing specifications migrated) |

This plan states how SPEC-0001 is built, in which order, and which choices need the
maintainer before the first task starts. The decisions D1 to D8 are recommendations until the
maintainer confirms them; every task that depends on one names it in `tasks.md`.

## Design

### Boundary: who reads, who parses, who decides

The core performs no I/O and decides everything (constitution I, ADR 0001). Reading files and
the tracker is I/O; turning a stored form into the document model is partly decision logic.
The plan splits the work as follows.

| Step | Where | Why |
| --- | --- | --- |
| Find the files to read | Core: a constant list of locations (`plan/briefs/*.yaml`, `plan/council/*.yaml`, `plan/items/*.yaml`, `specs/<NNNN>-<capability>/spec.md`, excluding `specs/_templates/`). Runtime: enumerates them. | Where artifacts live is a rule of the specification; walking a directory is I/O. |
| Read the files, sort them, make paths relative | Runtime, module `project` | I/O. Paths are relative to the project root with `/` separators and sorted by bytes, so that the canonical input is the same on every platform (SPEC-0004 R15: no absolute path). |
| Parse YAML into a JSON value, with a map from JSON pointer to line | Runtime (D1) | Parsing YAML is mechanical and needs a large dependency; keeping it out of the pure crates keeps their allowlist small. A file that does not parse is passed on as `unreadable` with its line and the parser message; the core turns it into the error. |
| Normalise a living specification from Markdown into its document model | Core (R12) | The mapping of sections, header rows and verification words, and the refusals it raises (`missing`, `enum`), are decision logic. |
| Schema validation, identifiers, links, statuses, naming rules | Core | Decision logic. |
| Turn the envelope into an exit code and an output stream | CLI | Translation only (ADR 0028); the exit code is computed by a core function. |

YAML files hold the normalised form already (SPEC-0001, Where artifacts live), so the runtime
does not normalise them: it only changes their syntax to JSON. The YAML parser must follow the
YAML 1.2 core schema (so `no`, `on` and `2026-11-17` stay strings), refuse duplicate keys as a
syntax error, limit alias expansion, depth and file size, and report the line of every node.
The choice of the crate is made in task T6 and reviewed by the security-reviewer role; the
runtime has no allowlist, but every new dependency passes `deny.toml`.

### Canonical input of `wayloop validate`

The entry point of ADR 0028 takes JSON. The input of `validate`, version 1, is:

| Field | Type | Meaning |
| --- | --- | --- |
| `documents` | list of objects, may be empty | One entry per stored artifact or input, in the order the runtime sorted them |
| `documents/<i>/origin` | object `{file}`, `{issue}`, `{milestone}`, or absent | Where the document was read; becomes `source` in every error (R12). Absent for a document sent directly. |
| `documents/<i>/format` | enum: `document`, `markdown`, `unreadable` | `document`: a JSON value in the normalised form; `markdown`: the text of a `spec.md`; `unreadable`: a file that did not parse |
| `documents/<i>/document` | object | With `document`: the value |
| `documents/<i>/lines` | object, JSON pointer to line number | With `document` and a file origin: where each value is in the file |
| `documents/<i>/text` | string | With `markdown`: the file content |
| `documents/<i>/error` | object `{line, message}` | With `unreadable`: what the parser reported |
| `documents/<i>/input_kind` | enum: `pull_request`, `commit`, `handover_input` | For inputs, which have no `kind` field; absent for artifacts |
| `base` | list of objects `{id, status}` | Statuses on the default branch of the artifacts of this set; absent when not compared (R13, R19) |
| `policy` | object `{types, scopes, areas}` | Subset of the project policy; absent means the default policy, listed in `meta.defaults_applied` (R21) |
| `adrs` | list of integers | Numbers of the ADRs of the project, for the warning on `related` (Q5 of the specification; Q3 below); absent means not checked, listed in `diagnostics.not_done` |

The whole version 1 is fixed in task T5, so that the command contract does not change at each
slice. A check of a field whose slice has not landed yet is listed in `diagnostics.not_done`,
never silently skipped. The command line translates `wayloop validate [<path>...]` (discovery,
or the given files as one set) and `wayloop validate --input <file|->` (canonical input as
JSON, used by the contract runner of #221) into this input.

Whoever parses JSON text into this input refuses duplicate keys before building the value, with
the reason `duplicate` of ADR 0009 C3 (T5, decided by the maintainer on 2026-11-20): `serde_json` keeps the last of two equal keys, and the canonical JSON
of RFC 8785 refuses them, so a duplicate would otherwise be dropped silently.

Tracker items (gates, milestones, epics, stories, tasks once their issue exists) reach the same
input as `document` entries with an `{issue}` or `{milestone}` origin when the tracker adapter
of SPEC-0005 exists. Until then `wayloop validate` reads repository files only, and a link to
an item that is only in the tracker is reported as missing, as R3 requires. This repository has
no such link today.

### Entry point and validation function

```rust
// crates/wayloop-core/src/lib.rs: the one entry point of ADR 0028 (#221 adds its runner).
pub fn run(command: Command, input: &serde_json::Value, context: &Context) -> Envelope;

// crates/wayloop-core/src/artifacts/validate.rs
pub fn validate(input: &ValidateInput) -> Validation;

pub struct Validation {
    pub documents: Vec<DocumentSummary>, // id, kind, status, origin of every document read
    pub errors: Vec<Error>,              // every error found, in one pass (ADR 0009 C3)
    pub warnings: Vec<Error>,            // link_broken_terminal, file-name mismatch, ADR citations
    pub defaults_applied: Vec<DefaultApplied>,
    pub not_done: Vec<String>,           // checks of slices not yet built
}
```

- `validate` is a pure, total function: it never panics, never stops at the first error, and
  returns the same `Validation` for the same input (constitution II). It uses `BTreeMap` and
  `BTreeSet` only: `HashMap` and `HashSet` are refused in the pure crates.
- It runs fixed stages over the whole set: (1) normalise Markdown, turn `unreadable` entries
  into errors; (2) per document: kind, `schema_version` (R16), migration to the current
  version, schema (R1); (3) identifiers (R14) and the index by identifier; (4) links and
  hierarchy (R3, R4, R9, R15); (5) parts and deltas (R18, R20); (6) statuses against `base`
  (R13, R19); (7) titles and naming (R7, R8, R10, R11). A document that fails stage 2 is kept
  out of the index, so that its broken shape does not cause a wave of link errors elsewhere.
- Error order is deterministic: by position of the document in the input, then by stage, then
  by pointer in document order. The first error is the top-level one (C3).
- `run` validates the canonical input against the input schema of the command, calls
  `validate`, and builds the envelope: a refusal when `errors` is not empty (exit code 1), a
  success with `data` and warnings otherwise. `Envelope::exit_code()` is the function the CLI
  calls; the CLI never maps outcomes itself.

### Schemas and their interpreter (D5)

Schemas are hand-written JSON Schema files (draft 2020-12) under `schemas/`, the snapshots of
ADR 0009 C5 and of the Interfaces of SPEC-0001. They are the source of the contract (ADR 0028)
and of the tool definitions generated from it (ADR 0024).

The core validates documents with a small interpreter of a documented subset of JSON Schema:
`type`, `properties`, `required`, `additionalProperties: false`, `enum`, `const`, `items`,
`minItems`, `minLength`, `maxLength`, `minimum`, `maximum`, `if`/`then`, `allOf`, `$ref` to
`$defs` of the same file, and `format` with named formats implemented in the core (identifier
forms, `M<n>`, part identifiers, dates, date-times with `Z`, BCP 47 tags, tracker handles).
No `pattern`: the core needs no regular expression engine, and formats give precise `fix`
values. A meta-test refuses a schema that uses a keyword outside the subset.

The interpreter maps keywords to the reasons of C3: `required` to `missing`, a `required`
inside `then` to `requires`, `additionalProperties` to `unknown_field` with `fix` the closest
declared key (Damerau-Levenshtein distance at most a third of the key length, ties by the
byte order of the key, because `serde_json` without `preserve_order` keeps no order of the
keys of a schema and that feature would add crates to the allowlist), `type` to `type`, `enum` and `const` to `enum` with `expected`, length keywords to
`length`, numeric bounds to `range`, `format` to `syntax`. Errors on `/id` are left to the
identifier stage, so that each fault gets one error with its own code (`identifier_format`).

Numbers (decided by the maintainer on 2026-11-20, for T4 and T5): a schema integer is bounded to
±(2^53 − 1), the range a double holds exactly, and a value beyond it is refused with the reason
`range`. Numbers are compared by value, never by their text: an integer is a finite number with
no fractional part, so `4364.0` is the integer `4364`, as JSON Schema and the canonical JSON of
RFC 8785 both read it. The same input therefore has one verdict and one `input_hash`.

The schema files are embedded in `wayloop-schemas` with `include_str!` at build time. A test
checks that every file under `schemas/artifacts/` and `schemas/inputs/` is embedded and that
the kinds of the core match the folders.

Schema versions (R16): each kind has a list of known versions and a chain of migrations
`v<N>` to `v<N+1>`, pure functions on the JSON value. Only `v1` exists; the mechanism is
proven by a test-only kind with `v1` and `v2`, so that AC25 is covered before a real `v2`.

### Error report and envelope (D6)

`wayloop_core::Error` gains the remaining fields of C3 (`message`, `pointers`, `expected`,
`fix`) and the field `source` of SPEC-0001 Errors: `{file, line}`, `{issue}` or `{milestone}`,
absent for a document sent directly. The serialisable shapes of the error report, the envelope
and `source` are contract types and move to `wayloop-schemas`; the core builds them and
re-exports them, so that the path `wayloop_core::Error` stays. Every code of the Errors table
of SPEC-0001 is a constant in `crates/wayloop-core/src/artifacts/codes.rs`.

`meta.input_hash` is the SHA-256 of the canonical JSON (RFC 8785) of the input. The core holds
a canonicaliser over `serde_json::Value` that sorts keys by UTF-16 code units, escapes strings
as RFC 8785 requires and writes numbers in the ECMAScript form; it never relies on the order
of the map. The envelope holds no clock time, host name or absolute path (SPEC-0004 R15).

### Naming rules (#160)

Titles, branches, commit headers and pull request links are pure functions in
`crates/wayloop-core/src/naming/`: a validator per grammar of the section Naming grammar,
returning errors with the codes `title_format`, `title_duplicate`, `branch_name`,
`commit_header` and `pull_request_link`, and a producer `branch_name(type, issue, title)` that
the property test of AC12 checks against the validator. Title equality (R8) uses Unicode NFC,
case folding, trimming and collapsed white space; the slug of a branch (R11) uses canonical
decomposition and drops combining marks before keeping `[a-z0-9]` (D4). The default policy of
R21 is a constant of the core equal to the policy of this repository, which a process test
compares with `scripts/conventions.py`.

### Project structure (`wayloop init`, #5)

The core computes an `InitPlan` from the list of paths that already exist and their content
hashes, given by the runtime: files to create with their content, files already identical
(skipped), and files that differ (kept, reported). Templates are embedded in the core. The
runtime applies the plan, or does nothing on a dry run (SPEC-0004 R8), writing only inside the
project root and never through a symbolic link. The created structure is `plan/briefs/`,
`plan/council/` and `plan/items/` (each with a short `README.md`, since git keeps no empty
folder), `specs/_templates/` and `specs/constitution.md`. An empty repository after
`wayloop init` passes `wayloop validate` (#5). The rest of the template of the #5 addendum
(policy registry, labeler, issue forms, required checks, conventions page, CI workflow) is
built on SPEC-0021 (D8).

### Dependencies

| Dependency | Crate | I/O | Status |
| --- | --- | --- | --- |
| `serde` with `derive` (and its macro crates) | `wayloop-schemas`, `wayloop-core` | No | Allowlist addition: D2 |
| `serde_json` (and its number and search helpers) | `wayloop-schemas`, `wayloop-core` | No | Allowlist addition: D2 |
| SHA-256 | `wayloop-core` | No | Hand-written, no dependency: D3 |
| Unicode normalisation (NFC, NFD) | `wayloop-core` | No | Allowlist addition: D4 |
| Unicode case folding | `wayloop-core` | No | Allowlist addition or none: D4 |
| YAML 1.2 parser with node positions | `wayloop-runtime` | No (parses text the runtime read) | No allowlist; security review in T6 |
| JSON Schema validator | none in the shipped crates | | Development dependency of `wayloop-core` only, as an oracle in tests (D5); not on the allowlist, which skips development dependencies |
| Property testing | development dependency of `wayloop-core` | | Not shipped |

Each allowlist entry lists every crate reached, transitive ones included, as
`scripts/check_core_io.py` requires; the exact list is fixed by the task that adds the
dependency, with `cargo tree` output in the pull request.

## Decisions

Decided by the maintainer on 2026-11-19: every recommendation below is adopted (D1 (b),
D2 (a), D3 (a), D4 (a) and (i), D5 (a), D6 (a), D7 (a), D8 (a)). The full template of #5 is
#275 (M2).

- **D1. Where YAML is parsed.** (a) In the core, from text: one parser for every channel, but a
  large parser and its dependencies on the pure allowlist. (b) In the runtime, into JSON values
  with a line map; the core receives values, and Markdown text which it normalises itself.
  (c) In each adapter: parsers differ between channels and break parity (ADR 0028).
  Recommendation: (b).
- **D2. serde and serde_json in the pure crates.** (a) Add both, with their transitive crates,
  to the allowlists of `wayloop-schemas` and `wayloop-core`. (b) Keep the allowlists empty and
  write a value type, a parser and a serialiser by hand. Recommendation: (a): both crates
  perform no I/O, every contract type needs them, and (b) duplicates mature code for no gain.
- **D3. SHA-256 for `input_hash`.** (a) A hand-written SHA-256 in the core, about 150 lines,
  checked against the published test vectors and, in a property test, against a hashing crate
  used as a development dependency. (b) A hashing crate on the core allowlist; on some targets
  it brings CPU feature detection and the C library bindings into the allowlist. The hash
  identifies content and protects no secret, so timing attacks are not a concern.
  Recommendation: (a).
- **D4. Unicode for R8 and R11.** NFC and canonical decomposition need the Unicode tables:
  (a) a normalisation crate on the core allowlist; (b) tables generated into the core by a
  script. Case folding: (i) a pure case-folding crate on the allowlist; (ii) lower-casing after
  NFC, which differs from case folding for a few characters (for example the sharp s and the
  final sigma) and needs an edit of SPEC-0001 to say so. Recommendation: (a) and (i), each
  crate checked for I/O and licence by the security-reviewer role in T16.
- **D5. How schemas are written and checked.** (a) Hand-written JSON Schema snapshots,
  interpreted by the core over a documented subset. (b) Rust types as the source, with schemas
  generated from them: the contract would change with a refactor of a type, against ADR 0028,
  and the generated shapes would follow the generator, not C4. (c) Hand-written snapshots
  checked by a JSON Schema validator crate in the core: a large dependency on the allowlist,
  errors that do not map to the reasons and fixes of C3, and no control over one-pass
  reporting. Recommendation: (a), with a validator crate as a test-only oracle that must agree
  with the interpreter on every golden case.
- **D6. The envelope before SPEC-0004 is approved.** SPEC-0004 is Draft; #55 (envelope) and
  #221 (entry point and runner) are in M1, while #6 is in M0. (a) Build now the subset of ADR
  0009 C1, C3 and C7 that `validate` needs, with the entry point of ADR 0028 holding one
  command; #55 and #221 extend it. (b) Wait for SPEC-0004 and its tasks. Recommendation: (a),
  with two edits of SPEC-0004 first: the field `source` in R2 (a dependency stated by
  SPEC-0001), and where a refusal sits in the envelope, which ADR 0009 C1 does not say (this
  plan assumes a top-level `error` holding the C3 report, with `data` absent). The folder of
  golden contract cases, which the `contract-case` skill leaves to the plan of SPEC-0004, is
  fixed here as `tests/contract/<command>/v<N>/<case>.json` unless the maintainer prefers
  another.
- **D7. Issues for the tasks.** A pull request closes exactly one issue (AGENTS.md). (a) One
  issue per task of `tasks.md`, created with `scripts/roadmap.py new` under the epic of #6,
  #160 and #5; each of those three is closed by its last task. (b) Intermediate pull requests
  cite `Refs #6`. Recommendation: (a): one issue per pull request keeps traceability and lets
  the planner place tasks in lanes.
- **D8. Scope of #5.** (a) #5 delivers the structure of SPEC-0001 (folders, templates,
  constitution) and the guarantee that the result validates; the template of the addendum
  becomes a task of SPEC-0021. (b) #5 waits for SPEC-0021. Recommendation: (a). The behaviour
  on a conflicting existing file (exit code, error code) is Q1 below.

## Contract changes

All new, version 1, no breaking change. Every file is a contract change: review plus the
maintainer's approval before merge.

- `schemas/artifacts/<kind>/v1.json` for `brief`, `council_report`, `gate`, `milestone`,
  `epic`, `story`, `specification`, `change`, `task`, `handover`; and
  `schemas/artifacts/handover/v1.input.json`.
- `schemas/inputs/pull_request/v1.json`, `schemas/inputs/commit/v1.json`.
- `schemas/commands/validate/v1.json` and `v1.output.json`; `schemas/commands/init/v1.json`
  and `v1.output.json` (SPEC-0004 R9).
- Error codes of SPEC-0001 Errors, first used by the tasks listed in `tasks.md`.

The protection of released snapshots does not cover the name `v<N>.input.json` that
SPEC-0001 gives to the hand-over input: the pattern of `.claude/hooks/protect_contracts.py`
matches `v<N>.json` and `v<N>.output.json` only, and no CI check exists yet. Task T3 fixes the
hook and adds the CI check that AC3 requires, before the first snapshot merges.

## Test strategy

- **Golden contract cases.** `specs/0001-artifact-model/examples/` is the starting set. The
  valid documents form one case that must pass with no error. Each invalid document is one
  case: the valid set plus that document, with the expectation stated in its first comment
  (requirement, pointer, code, reason, expected, fix). Cases are canonical inputs of
  `validate` in JSON under `tests/contract/validate/v1/`, generated once from the examples by a
  helper of the runtime tests and checked against them by a test, so that the examples and the
  cases cannot drift. Expected values come from the specification, never from running the
  code (`contract-case` skill). Released cases are never edited.
- **Acceptance criteria.** Each AC is a test named after it (`ac13_markdown_requirement_without_id`)
  in the module that implements it, written before the code. AC12 is a property test of the
  producer against the validator. AC3 is a process test of the snapshot check.
- **Oracles.** The schema interpreter agrees with a JSON Schema validator (test-only) on the
  verdict and the first pointer of every golden case; the SHA-256 agrees with a hashing crate
  (test-only) on random inputs; the canonicaliser passes the test vectors of RFC 8785.
- **End to end.** `wayloop validate` and `wayloop init` run as the built binary on temporary
  folders: exit code, envelope, and no file changed by a dry run.
- **Core quality.** The line coverage floor of `wayloop-core` (95 percent) holds after every
  task; `just mutants` runs on the schema interpreter and the naming module before their pull
  requests.

## Documentation

- Reference pages `docs/en/reference/validate.md` (T5) and `docs/en/reference/init.md` (T20),
  with inputs, outputs, error codes and exit codes; Italian editions stamped.
- Concept page `docs/en/concepts/artifacts.md` (T21): the kinds, identifiers, statuses, links
  and where each artifact lives; Italian edition.
- `docs/en/concepts/architecture.md`: the core no longer has an empty external allowlist (T1),
  the parsing boundary (T6); Italian edition.
- A fragment in `changes/` for every task with a visible effect.

## Risks and alternatives

- **The interpreter and the schemas drift from the specification.** Mitigation: the golden
  cases come from the specification, and the test-only validator checks the interpreter on
  every case.
- **Dependency growth in the pure crates.** Mitigation: D2 to D4 add only crates without I/O,
  each listed in the allowlist with its path, and checked by `deny.toml`.
- **YAML pitfalls** (implicit booleans and dates, duplicate keys, alias bombs). Mitigation: the
  parser requirements in Design, and golden cases for each pitfall in T6.
- **Contract churn of the `validate` input.** Mitigation: version 1 is fixed in T5 with every
  field of this plan; unfinished checks are listed in `diagnostics.not_done`.
- **Ahead of SPEC-0004.** If SPEC-0004 decides another envelope shape, the subset built here
  changes before its first release. Mitigation: D6 asks for the two SPEC-0004 edits first.
- **Packaging of embedded schemas.** `include_str!` reads `schemas/` outside the crate folder,
  which a published crate cannot do. Mitigation: the release pipeline of SPEC-0018 copies the
  snapshots into the package; until then the crates are not published.
- **This repository fails `wayloop validate` on the first day.** Mitigation: CI runs it only
  after #229 (task T19).
- **Rejected alternative: typed Rust structs per kind, deserialised with serde.** Deserialising
  stops at the first error and cannot report every error in one pass (C3); typed views are
  added later where a decision needs them.

## Open questions

Decided by the maintainer on 2026-11-19: the recommendation of each question is adopted.

- [x] **Q1. `wayloop init` on a file that exists with other content.** (a) Exit code 0, the
  file kept and listed as a warning with a new code `init_conflict`; (b) exit code 1, a
  refusal listing every conflict, nothing written. Recommendation: (a), since nothing is
  overwritten and the rest of the structure is useful; the code is added to SPEC-0004.
- [x] **Q2. A task without `epic`.** The schema requires `epic` in every status, and R9 refuses
  a missing parent in a non-terminal status. Recommendation: one error only, `artifact_schema`
  with reason `missing` at `/epic`, as in `task.invalid.yaml`; `link_missing` is kept for an
  identifier that is present but resolves to nothing.
- [x] **Q3. ADR citations (Q5 of the specification).** The warning for `ADR <NNNN>` needs the
  list of ADR numbers of the project, which SPEC-0001 does not locate. Recommendation: an
  optional field `adrs` (list of integers) in the canonical input, filled by the runtime from
  `docs/en/project/adr/` in this repository and from a configured folder later (ADR 0010).
