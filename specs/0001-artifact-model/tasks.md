# SPEC-0001: Tasks

Rules: one task per pull request; `[P]` tasks touch disjoint files and can run in parallel
(tasks that add a kind each add one line to the kind registry, a trivial rebase); every task
names its verification technique; tick a task only when `uv run just check` passes. Tests are
written and committed first, and fail for the expected reason before the code exists.

Size: S up to about 300 changed lines with tests, M up to about 800. No task is larger.

Merge: a task marked **Maintainer** changes `crates/` or `schemas/` and waits for the
maintainer's confirmation (AGENTS.md, Merging); one with new snapshots under `schemas/` is a
contract change and needs the maintainer's approval. The decisions D1 to D8 and the questions
Q1 to Q3 are in `plan.md`.

Prerequisites outside this specification:
- SPEC-0004 edited to add `source` to R2 and to place the refusal in the envelope (D6), before T1.
- #228, step identifiers in `snake_case`, before T9.
- #229, existing specifications migrated, before T19.

## Foundations

- [ ] **T1 Complete the error report of ADR 0009 in the core**
  - Covers: SPEC-0001 Errors; R12 (field `source`). Issue: under #6 (D7).
  - Files: `crates/wayloop-schemas/src/` (report and `source` types), `crates/wayloop-core/src/error.rs`, `crates/wayloop-core/src/lib.rs`, `crates/wayloop-*/Cargo.toml`, `Cargo.lock`, `scripts/check_core_io.py`, `tests/process/test_check_core_io.py`, `docs/en/concepts/architecture.md` and its Italian edition.
  - Tests first: an error serialises with `message`, `pointers`, `expected`, `fix` and each form of `source`, absent fields left out; a report keeps `errors[0]` equal to the top level; the allowlist test lists the new crates.
  - Verification: TDD.
  - Depends on: D2.
  - Size: S. Maintainer (crates).

- [ ] **T2 Add canonical JSON, SHA-256 and the envelope to the core**
  - Covers: ADR 0009 C1, C7; SPEC-0001 Interfaces (error report in the envelope). Issue: under #6.
  - Files: `crates/wayloop-core/src/canonical.rs`, `crates/wayloop-core/src/sha256.rs`, `crates/wayloop-core/src/envelope.rs`, `crates/wayloop-schemas/src/` (envelope type), `crates/wayloop-core/Cargo.toml` (development dependencies).
  - Tests first: RFC 8785 test vectors; keys sorted by UTF-16 code units; SHA-256 published vectors and agreement with a test-only hashing crate on random inputs; the same input with keys in any order gives one `input_hash`; `Envelope::exit_code()` gives 0, 1 and 5 for success, refusal and internal error.
  - Verification: TDD, property test.
  - Depends on: T1, D3, D6.
  - Size: M. Maintainer (crates).

- [ ] **T3 [P] Refuse in CI any change to a released schema snapshot**
  - Covers: R1, R16; AC3. Issue: under #6 (shared with #54).
  - Files: `scripts/check_schemas.py`, `tests/process/test_check_schemas.py`, `justfile` (recipe `process`), `.claude/hooks/protect_contracts.py` (pattern extended to `v<N>.input.json`), `tests/process/test_claude_hooks.py`, `scripts/required_checks.py` if a new required check is added.
  - Tests first: a snapshot present on the base branch and changed fails; a new `v<N+1>.json` beside an unchanged `v<N>.json` passes; `v1.input.json` is protected by the hook and by the check; a gap in versions fails.
  - Verification: contract test.
  - Depends on: none.
  - Size: S. Small tier (tooling).

- [ ] **T4 Interpret the JSON Schema subset in the core**
  - Covers: R1, R12 (pointers); ADR 0009 C3 (every error in one pass). Issue: under #6.
  - Files: `crates/wayloop-core/src/schema/` (interpreter, formats, closest key), `crates/wayloop-core/Cargo.toml` (test-only validator).
  - Tests first: each keyword of the subset maps to its reason (`missing`, `requires` inside `then`, `unknown_field` with `fix`, `type`, `enum` with `expected`, `length`, `range`, `syntax` for a format); three independent faults give three errors in document order; a schema with a keyword outside the subset is refused; the test-only validator agrees on the verdict of every test schema.
  - Verification: TDD, property test.
  - Depends on: T1, D5.
  - Size: M. Maintainer (crates).

## Slice 1: one kind end to end

- [ ] **T5 Validate a brief through the entry point and wayloop validate --input**
  - Covers: R1 (brief), R5 (statuses of the brief), R6, R12, R16; AC1 (brief pair), AC8 (brief in `implementing`), AC9 (key `problema` in place of `problem`, as in `brief.invalid.yaml`), AC25. Issue: under #6.
  - Files: `schemas/artifacts/brief/v1.json`, `schemas/commands/validate/v1.json`, `schemas/commands/validate/v1.output.json`, `crates/wayloop-schemas/src/` (embedded snapshots), `crates/wayloop-core/src/artifacts/` (kinds, versions and migrations, `validate`, codes), `crates/wayloop-core/src/lib.rs` (`run`), `crates/wayloop-cli/src/main.rs` (`validate --input`), `tests/contract/validate/v1/`, `docs/en/reference/validate.md` and its Italian edition, `changes/`.
  - Tests first: golden cases `brief-valid`, `brief-invalid` (unknown key with `fix`, then `missing`), `schema-version-unknown` (`/schema_version`, `enum`, known versions in `expected`), `brief-status-implementing` (`enum`); migration of a test-only kind from `v1` to `v2`; the CLI prints the envelope of the entry point byte for byte and exits 1 on a refusal, 0 otherwise.
  - Verification: contract test, end-to-end.
  - Depends on: T2, T3, T4, D6, Q3.
  - Size: M. Maintainer (crates, contract change).

- [ ] **T6 Load repository files into the canonical input of validate**
  - Covers: R12 (`source` with file and line); D1. Issue: under #6.
  - Files: `crates/wayloop-runtime/src/project.rs`, `crates/wayloop-runtime/Cargo.toml` (YAML parser), `crates/wayloop-core/src/artifacts/locations.rs`, `crates/wayloop-cli/src/main.rs` (`validate [<path>...]`), `Cargo.lock`, `docs/en/concepts/architecture.md` and its Italian edition.
  - Tests first: discovery of the locations of the core, sorted, with relative `/` paths; `source` carries the file and the line of the faulty value; `no`, `on` and a date stay strings; a duplicate key and a malformed file become `unreadable` entries reported with `syntax` and their line; an alias bomb and an oversized file are refused, not loaded; the examples load into the committed golden inputs.
  - Verification: TDD, end-to-end.
  - Depends on: T5, D1. Security review of the YAML parser.
  - Size: M. Maintainer (crates).

## Remaining kinds

- [ ] **T7 Validate milestones, epics, stories, tasks and gate issues**
  - Covers: R1, R5, R6, R9 (required parent and milestone); AC1 (these pairs), AC9 (story with Italian prose passes; key `titolo` refused with `fix` `title`), AC35 (task with `judge` and no `rubric`). Issue: under #6.
  - Files: `schemas/artifacts/{milestone,epic,story,task,gate}/v1.json`, `crates/wayloop-core/src/artifacts/` (kind registry, formats), `tests/contract/validate/v1/`.
  - Tests first: golden cases from `gate`, `task`, `epic` (schema part only), `story`, `milestone` examples; `approved_by` required when a gate is `done` (`requires`); `specification` required on a story from `approved` (`requires`); a task without `epic` gives one error (Q2).
  - Verification: contract test, TDD.
  - Depends on: T5, Q2.
  - Size: M. Maintainer (crates, contract change).

- [ ] **T8 [P] Validate council reports, living specifications and change deltas in normalised form**
  - Covers: R1, R5, R6; AC1 (these pairs, `specification.invalid.yaml`), AC35 (acceptance criterion with `judge` and no `rubric`). Issue: under #6.
  - Files: `schemas/artifacts/{council_report,specification,change}/v1.json`, `crates/wayloop-core/src/artifacts/` (registry line), `tests/contract/validate/v1/`.
  - Tests first: golden cases from these examples, including `specification.superseded.valid.yaml`; `status: published` refused with `enum` and the six statuses; operation fields required by `op` (`requires`).
  - Verification: contract test.
  - Depends on: T5.
  - Size: M. Maintainer (crates, contract change).

- [ ] **T9 [P] Validate hand-over records and hand-over inputs**
  - Covers: R1, R22 (input refused before engine fields are filled); AC1 (hand-over pairs), AC31 (first half: `tokens` in the input gives `unknown_field` with code `handover_input`). Issue: under #6.
  - Files: `schemas/artifacts/handover/v1.json`, `schemas/artifacts/handover/v1.input.json`, `crates/wayloop-core/src/artifacts/` (registry line), `tests/contract/validate/v1/`.
  - Tests first: golden cases from the hand-over examples; the `step` enum of both schemas equals `IssueStep::ALL` of the core.
  - Verification: contract test.
  - Depends on: T5, #228.
  - Size: S. Maintainer (crates, contract change).

- [ ] **T10 [P] Validate the shape of pull request and commit inputs**
  - Covers: R1 (inputs); AC1 (structural part of these pairs). Issue: under #6.
  - Files: `schemas/inputs/pull_request/v1.json`, `schemas/inputs/commit/v1.json`, `crates/wayloop-core/src/artifacts/` (input kinds), `tests/contract/validate/v1/`.
  - Tests first: `input_kind` selects the schema; a missing `branch` or `header` gives `missing`; an input without `input_kind` and without `kind` gives `missing` at `/kind`.
  - Verification: contract test.
  - Depends on: T5.
  - Size: S. Maintainer (crates, contract change).

## Rules across artifacts

- [ ] **T11 Check identifier forms and uniqueness across the set**
  - Covers: R2, R14; AC6 (second half: both duplicates refused with `duplicate`, each with the source of the other), AC1 (`milestone.invalid.yaml`: `identifier_format`, `fix` `M2`, no `expected`). Issue: under #6.
  - Files: `crates/wayloop-core/src/artifacts/identifiers.rs`, `tests/contract/validate/v1/`.
  - Tests first: `TASK-107` and `TASK-00107` refused, `TASK-10000` accepted; `HANDOVER-12-3` accepted; a file name or folder number that differs from the identifier gives a warning.
  - Verification: TDD, contract test.
  - Depends on: T7, T8.
  - Size: S. Maintainer (crates).

- [ ] **T12 Check links and the hierarchy between artifacts**
  - Covers: R3, R4, R9 (cited specification and requirement exist), R15; AC2, AC5, AC6 (first half: links resolve after a move and a rename), AC7, AC14, AC24; AC1 (`story.invalid`, `story.superseded-link.invalid`, `change.invalid`, `council_report.invalid`). Issue: under #6.
  - Files: `crates/wayloop-core/src/artifacts/links.rs`, `tests/contract/validate/v1/`.
  - Tests first: one test per AC above; a broken link of a `done` task is a warning `link_broken_terminal`; a superseded target gives `fix` with its `superseded_by`; an ADR citation with no ADR of that number gives a warning when `adrs` is given.
  - Verification: TDD, contract test.
  - Depends on: T11.
  - Size: M. Maintainer (crates).

- [ ] **T13 Check parts of artifacts and change deltas**
  - Covers: R18, R20; AC27, AC29 (first half; and the pure function that gives the next requirement identifier, used by the merge of SPEC-0009). Issue: under #6.
  - Files: `crates/wayloop-core/src/artifacts/parts.rs`, `tests/contract/validate/v1/`.
  - Tests first: duplicate `D1`, dissent naming `D4`, answer outside the options, task depending on itself, on a superseded task, and a cycle of three tasks; `change.conflict.invalid.yaml` with `pointers` to both operations; with R1 to R3 and R4 removed, the next identifier is R5.
  - Verification: TDD, contract test.
  - Depends on: T12.
  - Size: M. Maintainer (crates).

- [ ] **T14 Normalise living specifications from Markdown**
  - Covers: R6, R12; AC13, AC36. Issue: under #6.
  - Files: `crates/wayloop-core/src/artifacts/markdown.rs`, `crates/wayloop-runtime/src/project.rs` (pass `spec.md` as `markdown`), `tests/contract/validate/v1/`.
  - Tests first: AC13 (`/requirements/1/id` with file and line); AC36 (`## Notes` and `## Requisiti` kept as prose, verification `[property, tdd]`, `/requirements` missing); continuation lines of a list item; header rows split at commas and semicolons; verification text with no mapped word refused with `enum`; this specification normalises with no error.
  - Verification: TDD, contract test.
  - Depends on: T8, T6.
  - Size: M. Maintainer (crates).

- [ ] **T15 Check statuses against the default branch**
  - Covers: R5, R13, R19; AC8 (first half), AC28. Issue: closes #6.
  - Files: `crates/wayloop-core/src/artifacts/statuses.rs`, `crates/wayloop-runtime/src/project.rs` (`base` from the default branch through git), `crates/wayloop-cli/src/main.rs` (`--base <ref>`), `tests/contract/validate/v1/`.
  - Tests first: a task from `draft` to `done` refused with `state` and `expected` `approved`, `superseded`; a new task in `implementing` refused with `state` and `expected` `draft`; a new specification in `surveyed` passes; without `base`, the check is listed in `diagnostics.not_done`.
  - Verification: TDD, end-to-end.
  - Depends on: T7, T8, T6; merges after T12, T13 and T14, so that it closes #6 last.
  - Size: M. Maintainer (crates).

## Naming rules (#160)

- [ ] **T16 Check epic and task titles**
  - Covers: R7, R8; AC4, AC10; AC1 (`epic.invalid.yaml`). Issue: under #160.
  - Files: `crates/wayloop-core/src/naming/titles.rs`, `crates/wayloop-core/Cargo.toml`, `Cargo.lock`, `scripts/check_core_io.py`, `tests/process/test_check_core_io.py`.
  - Tests first: prefix, bracket tag at either end, final full stop, each with pointer and `fix`; titles equal after NFC, case folding and white space refused across epics and tasks, superseded ones ignored; `wayloop init creates a project structure that validates` passes.
  - Verification: TDD.
  - Depends on: T7, D4.
  - Size: S. Maintainer (crates).

- [ ] **T17 Validate and produce branch names and commit headers**
  - Covers: R11; AC12, AC15, AC16, AC17, AC18, AC19, AC20, AC21, AC22; AC1 (`commit.invalid.yaml`). Issue: under #160.
  - Files: `crates/wayloop-core/src/naming/branch.rs`, `crates/wayloop-core/src/naming/commit.rs`, `tests/contract/validate/v1/`.
  - Tests first: one test per AC; the property test of AC12 over any issue number, type and title, accented and non-ASCII titles included.
  - Verification: TDD, property test.
  - Depends on: T10, T16.
  - Size: M. Maintainer (crates).

- [ ] **T18 Check pull request links and apply the default policy**
  - Covers: R10, R21; AC11, AC23, AC30; AC1 (`pull_request.invalid.yaml`, and the whole example set passing). Issue: closes #160.
  - Files: `crates/wayloop-core/src/naming/pull_request.rs`, `crates/wayloop-core/src/policy.rs`, `tests/process/test_conventions.py` (default policy equals `scripts/conventions.py`), `tests/contract/validate/v1/`.
  - Tests first: one test per AC; the label `no-issue` waives the rule and the waiver is returned in `data` for the ledger writer (the event itself is recorded with SPEC-0003); without `policy`, `meta.defaults_applied` lists the default policy.
  - Verification: TDD, contract test.
  - Depends on: T17, T12.
  - Size: M. Maintainer (crates).

## Adoption and project structure

- [ ] **T19 Run wayloop validate on this repository in CI**
  - Covers: R3, R12 on real data; SPEC-0001 Markdown form (migration done by #229). Issue: new (D7).
  - Files: `justfile`, `.github/workflows/ci.yml`, `scripts/required_checks.py`, `tests/process/test_required_checks.py`.
  - Tests first: the recipe fails on a fixture repository with a broken link; passes on this repository.
  - Verification: end-to-end.
  - Depends on: T14, T15, #229.
  - Size: S. Small tier (CI).

- [ ] **T20 Create the project structure with wayloop init**
  - Covers: SPEC-0001 Where artifacts live (folders under `plan/`); #5 as scoped by D8. Issue: closes #5.
  - Files: `crates/wayloop-core/src/init.rs` (plan of files, embedded templates), `crates/wayloop-runtime/src/init.rs` (apply), `crates/wayloop-cli/src/main.rs`, `schemas/commands/init/v1.json`, `schemas/commands/init/v1.output.json`, `tests/contract/init/v1/`, `docs/en/reference/init.md` and its Italian edition, `changes/`.
  - Tests first: on an empty folder, `wayloop init` then `wayloop validate` exits 0; a second run changes nothing; an existing file with other content is kept and reported (Q1); a dry run writes nothing and lists the planned files (SPEC-0004 AC5); no write outside the root or through a symbolic link.
  - Verification: end-to-end, contract test.
  - Depends on: T6, T14, D8, Q1.
  - Size: M. Maintainer (crates, contract change).

- [ ] **T21 [P] Document the artifact model**
  - Covers: documentation of SPEC-0001. Issue: new (D7).
  - Files: `docs/en/concepts/artifacts.md`, `docs/it/concepts/artifacts.md`, the navigation of the site.
  - Tests first: none.
  - Verification: none (documentation); `just docs` builds.
  - Depends on: T12.
  - Size: S. Small tier.

## Later tasks (other issues, after their dependencies)

- [ ] **T22 Allocate the next identifier of a kind**
  - Covers: R23; AC32. Files: `crates/wayloop-core/src/artifacts/allocate.rs`. Verification: TDD. Depends on: T11. Size: S. Maintainer (crates).
- [ ] **T23 Move an epic with its open stories and tasks**
  - Covers: R24; AC33. Files: `crates/wayloop-core/src/plan/move_epic.rs`. Verification: TDD. Depends on: T12 and the plan change type of SPEC-0013. Size: S. Maintainer (crates).
- [ ] **T24 Replace a plan item file by its tracker item**
  - Covers: R25; AC34. Files: `crates/wayloop-core/src/plan/`, tracker adapter. Verification: TDD. Depends on: SPEC-0005 (#65). Size: S. Maintainer (crates).
- [ ] **T25 Record approvals of briefs and council reports**
  - Covers: R17; AC26. Files: `crates/wayloop-core/src/artifacts/approvals.rs`. Verification: TDD. Depends on: the ledger of SPEC-0003, ADR 0012. Size: M. Maintainer (crates).
- [ ] **T26 Fill the engine fields of a submitted hand-over**
  - Covers: R22; AC31 (second half). Files: `crates/wayloop-core/src/artifacts/handover.rs`. Verification: TDD. Depends on: T9 and SPEC-0003 R12. Size: S. Maintainer (crates).

## Verification

- [ ] T27 Review by risk tier of every task with an agent other than the implementer; the
  contract-auditor role on T5, T7 to T10 and T20; every acceptance criterion ticked in
  `spec.md` with the test that proves it; status `Done` set by the maintainer.
