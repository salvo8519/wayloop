---
name: add-command
description: Adds an engine command end to end - contract cases first, decision logic in the core, a thin CLI adapter, versioned input and output schema snapshots in the envelope of ADR 0009, error codes and reasons, the generated tool definition, documentation, a release fragment and a contract audit - and waits for the maintainer before merge. Use when an approved specification introduces a new `wayloop` command, or when the maintainer asks to add one.
---

<!-- Generated from .agents/skills/add-command/SKILL.md by scripts/sync_agents.py. Edit the source, not this file. -->

Talk to the maintainer in their preferred language (`CLAUDE.local.md`); everything written to the repository or to GitHub is in English, without icons.

A command is a contract (ADR 0009, ADR 0023): agents call it, parse its result and correct
their input from its errors. Its shape is decided before its code, and a released shape never
changes in place.

## Produces
- Contract cases for the command, committed before the code (the `contract-case` skill).
- In `crates/wayloop-core`: the input and output types, validation that reports every error in
  one pass, and the decision function, without I/O.
- In `crates/wayloop-cli`: a subcommand that reads the input, calls the core, prints the
  envelope and maps the outcome to an exit code. No decision.
- The snapshots `schemas/commands/<command>/v1.json` and `schemas/commands/<command>/v1.output.json`,
  recorded in the schema index (schemas/index.json).
- The reference page `docs/en/reference/<command>.md` with inputs, outputs, error codes and
  exit codes, and its Italian edition where `i18n/it.yaml` requires one.
- The tool definition generated from the snapshots (ADR 0024).
- The fragment `changes/<issue>.feature.md`.
- A contract-auditor verdict on the whole branch and a pull request that waits for the
  maintainer's confirmation, because it changes `crates/`.

## Steps
1. **Preconditions, deterministically.** Read the status line of `specs/NNNN-<slug>/spec.md`:
   `Approved` or `Implementing`, with numbered requirements that name the command. Run
   `git status` (clean tree), `uv run python scripts/journal.py search <issue>` and
   `uv run python scripts/roadmap.py list --open` (the issue is planned). Check that the name
   is free: `rg -n '"<command>"' crates/wayloop-cli/src` and `ls schemas/commands/<command>`.
2. **Baseline.** Run `uv run just check` before changing anything; record a failure that
   already exists on `main`.
3. **Contract on paper.** From the specification, write in the issue or the plan of the
   specification, before any code:
   - input fields: type, required or optional, default and its reason (C2); flat shapes, at
     most four levels, a choice as a kind enum plus fields, snake_case enum values each with a
     meaning, ISO 8601 dates (C4);
   - output fields of `data`, and what `diagnostics` may carry: warnings, `not_done` entries
     `{check, reason}` and the `checks` run, each check a `snake_case` identifier; nothing
     command-specific outside `data` (C1);
   - one row per refusal: code, reason from the C3 enum (syntax, missing, unknown_field,
     duplicate, type, enum, conflict, requires, range, length, state, stale), pointer,
     expected; a new meaning gets a new code, never a reused or renamed one (C8). Search the
     existing codes first: `rg -n '<code>' crates docs/en`. A failure of the engine itself
     uses the reason `internal` and exits with 5, never 1;
   - exit code per outcome (C7): 0 success, 1 input error, 2 usage error, 3 environment error,
     4 stopped at a gate, 5 computation or internal error;
   - whether the command changes state: then it takes an expected revision and refuses a stale
     one with reason `stale` (C6), and supports a dry run when it changes files or the tracker
     (SPEC-0004 R8);
   - the hints of its tool definition: read-only, destructive, idempotent (ADR 0024).
   Any point the specification leaves open goes to the maintainer (see Stop and ask).
4. **Contract cases first.** Apply the `contract-case` skill, delegating to the test-engineer
   role (`.agents/agents/test-engineer.md`): a minimal and a maximal case, one case per error
   reason, and one input with three independent errors (SPEC-0004 AC1). Run them and confirm
   they fail because the command does not exist, not because of a fixture. Commit them as
   `test(<scope>): ...`.
5. **Core.** Add the types and the function in a module of `crates/wayloop-core/src/`,
   exported from `crates/wayloop-core/src/lib.rs`. Validation collects every error in one
   pass, the first equal to the top level; pointers follow RFC 6901 from the root of the input
   sent; absent and null mean not given; every default applied is listed with pointer, value
   and reason. No file, network, clock or environment access: adapters pass those in. Unit
   tests for the edges; run `cargo test -p wayloop-core`.
6. **CLI adapter.** Add the subcommand in `crates/wayloop-cli/src/main.rs`: parse, call the
   core, print the envelope with `meta` (engine_version, envelope_version, command_version,
   input_hash, dry_run, defaults_applied), return the exit code of step 3. If the adapter needs
   a condition on a domain value, that condition belongs in the core. Run
   `cargo test -p wayloop-cli`.
7. **Snapshots.** Produce `schemas/commands/<command>/v1.json` and
   `schemas/commands/<command>/v1.output.json` from the core types and record them in the
   schema index. A new command starts at version 1. For a command that already exists, check
   `git cat-file -e origin/main:schemas/commands/<command>/v<N>.json`: a released snapshot is
   never edited (`.claude/hooks/protect_contracts.py` blocks it); a removed, renamed, retyped
   or newly nullable field needs `v<N+1>`, `!` in the header and the label `breaking-change`;
   a new optional field is additive and keeps the version (C5).
8. **Tool definition.** Generate it from the snapshots with the generator of SPEC-0016, with
   the minimal and maximal cases as examples, the hints of step 3, and `_meta` carrying the
   command version and status. Never write it by hand. If the generator does not exist yet,
   record it as not done in the report and on the issue.
9. **Documentation.** Delegate to the docs-writer role (`.agents/agents/docs-writer.md`): the
   reference page with an example taken from the cases, the error code table and the exit
   codes. Run `uv run python scripts/docs_sync.py coverage`,
   `uv run python scripts/docs_sync.py impact --base origin/main` and `uv run just docs`. The
   Italian edition goes to the translator role and is stamped with
   `uv run python scripts/i18n.py stamp <file>`, or the pull request carries
   `translation-pending`.
10. **Release fragment.** Write `changes/<issue>.feature.md` following `changes/README.md`
    (`changes/<issue>.breaking.md` for a breaking change).
11. **Gate.** Run the contract cases, then `uv run just check`. Apply the `verify` skill.
12. **Review.** Apply the `pre-pr` skill: the contract-auditor role
    (`.agents/agents/contract-auditor.md`) and the reviewer run on the whole branch; the
    author never reviews its own work. Fix every finding with the `apply-review` skill until
    the verdict is CONTRACT SOUND.
13. **Merge.** The pull request changes `crates/` and `schemas/`: ask the maintainer to
    confirm, with the summary of the command and the audit; merge only after the
    confirmation (AGENTS.md, Merging).

## Stop and ask
- No approved specification, or it leaves open the name, a field, an error code, the state
  change or the dry run of the command.
- The command would need a decision in the adapter, I/O in the core, or a new dependency.
- A released snapshot would change, or an error code would be renamed or reused.
- The contract-auditor reports a defect that cannot be fixed without changing the
  specification.
- The merge: always the maintainer's confirmation.

## Report
- Done: cases (with the commit that added them before the code), core and adapter commits,
  snapshots and their version, error codes, documentation pages, fragment, audit verdict.
- Not done or not verified: tool definition not generated, Italian edition pending, commands
  not run and why.
- Risks: fields whose shape may need a breaking change later, codes close in meaning to
  existing ones, clients that read the command today.
