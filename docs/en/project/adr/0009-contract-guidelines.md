# ADR 0009: Contract guidelines: envelope, errors and versioning

Status: Accepted · 2026-11-17 (decided by the maintainer in #46)

## Context

Wayloop is driven mainly by agents. An agent that has to guess the shape of a result, parse an English message to find the field to correct, or discover a breaking change at run time wastes turns and money. Every command must answer the same questions in the same place, from the first release.

## Decision

- **C1.** One envelope for every command: { command, data, diagnostics { warnings, not_done, checks }, meta { engine_version, envelope_version, command_version, input_hash, dry_run, defaults_applied } }; nothing command-specific outside data. `input_hash` is the SHA-256, in lower-case hexadecimal, of the input serialised as canonical JSON (RFC 8785), so that the same input gives the same hash on every machine.
- **C2.** Absent and null mean not given; [] means none; every default applied is listed in meta.defaults_applied as {pointer, value, reason}.
- **C3.** Every refusal is {code, message, pointer, pointers, reason, expected, fix} with errors[] listing every error found in one pass, first equal to the top level; pointers follow RFC 6901 from the root of the input sent; reason is a documented enum (syntax, missing, unknown_field, duplicate, type, enum, conflict, requires, range, length, state, stale); expected lists accepted values; fix names the closest field for a misspelt key.
- **C4.** Shapes agents fill well: flat objects at most four levels deep, a choice is a kind enum plus fields, no positional alignment between arrays, snake_case enum values each with a described meaning, ISO 8601 dates.
- **C5.** Every command input and output schema is snapshotted as v<N>.json and v<N>.output.json; the command version is bumped on a removed, renamed, retyped or newly nullable field; a new optional field or output field is additive; clients accept enum values they do not know.
- **C6.** A command that changes state takes an expected revision and refuses a stale one with reason stale. The revision is the hash of the latest event of the ledger (ADR 0003); every result that reads state reports it in `meta.revision`. A revision is stale only when an event recorded after it touches the scope of the command: its issue, or the project phase, the gates and the plan for a command on the project. The engine checks the guards again under the writer lock, so parallel lanes do not refuse each other.
- **C7.** Exit codes: 0 success, 1 input error, 2 usage error, 3 environment error, 4 stopped at a gate, 5 computation or internal error.
- **C8.** A new meaning gets a new error code; codes are never renamed or reused.

## Amendments

- 2026-11-17, at acceptance: the computation of `input_hash` (C1) and the definition of the revision (C6) were made explicit.
- 2026-11-19, decided by the maintainer with the plan of SPEC-0001 (D6): a refused result carries the error report in a top-level field `error` and has no `data` (C1); the error report gains `source`, the location of the faulty value in the repository (C3).
- 2026-11-17, decided by the maintainer in #233: staleness (C6) is scoped to the issue of the command, or to phase, gates and plan for a command on the project.

## Consequences

An agent learns one contract and corrects every input error in one round. The cost is more validation code in the core and one schema snapshot per command version, checked in CI from the first command.
