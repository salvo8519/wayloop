# SPEC-0004: Command-line interface and JSON contracts

| | |
| --- | --- |
| Status | Draft |
| Milestone | M1 · Engine core |
| Owner | @salvo8519 |
| Related | ADR 0009, ADR 0010, #53, #54, #55, #56, #57, #163 |
| Commit scope | cli |

## Problem
Agents and people drive the engine through commands. Every command must speak the same JSON, fail the same way and stay stable across versions.

## Scope
**In scope:** Command set (init, validate, next, status, history, rulings, run, plan, issues, survey), JSON envelope, error model and exit codes (ADR 0009), configuration file and precedence (ADR 0010), output budget.

**Out of scope:** Language bindings (SPEC-0014, SPEC-0015) and the MCP server (SPEC-0016).

## User stories
- As an agent, I want every command to return JSON with a stable shape, so that I parse it without guessing.
- As a developer, I want clear exit codes, so that scripts and CI can react to failures.

## Requirements (EARS)
- **R1**: THE ENGINE SHALL return every result in the envelope of ADR 0009 C1, with meta.envelope_version and meta.command_version.
- **R2**: THE ENGINE SHALL report every refusal as {code, message, pointer, pointers, reason, expected, fix} and list every input error found in one pass in errors.
- **R3**: THE ENGINE SHALL treat absent and null as not given and list every default applied in meta.defaults_applied.
- **R4**: THE ENGINE SHALL snapshot every command input and output schema and bump the command version on a breaking change (ADR 0009 C5).
- **R5**: WHEN a command changes state, THE ENGINE SHALL require the expected revision and refuse a stale one with reason stale.
- **R6**: THE ENGINE SHALL keep the output of every command within a configurable token budget and say when it truncated.
- **R7**: THE ENGINE SHALL read the project file `wayloop.toml` and the user file `~/.config/wayloop/config.toml`, accept only keys marked personal in the user file, and apply the precedence flags, environment (`WAYLOOP_<SECTION>_<KEY>`), user file, project file, defaults (ADR 0010).
- **R8**: WHEN a command changes files or the tracker, THE ENGINE SHALL support a dry run that reports the changes without applying them.
- **R9**: THE ENGINE SHALL publish the JSON Schema of every command input and output.
- **R10**: THE ENGINE SHALL report tokens in and out, cost, duration and turns from the ledger, grouped by issue, phase, role and executor, for a time range (wayloop cost).
- **R11**: THE ENGINE SHALL compute `meta.input_hash` as the SHA-256 of the input serialised as canonical JSON (RFC 8785).
- **R12**: THE ENGINE SHALL report the revision, the hash of the latest ledger event, in `meta.revision` of every result that reads state (ADR 0009 C6).
- **R13**: THE ENGINE SHALL return the effective configuration with the source of every value with `wayloop config show`.

## Acceptance criteria
- **AC1**: Given each command, when run with an input holding three independent errors, then errors lists all three, each with pointer, reason, expected and fix. Verification: contract test.
- **AC2**: Given a golden input with each field changed or left out in turn, when validated, then every pointer names a field the client can act on. Verification: property test.
- **AC3**: Given a schema snapshot, when an output field is renamed without a version bump, then CI fails. Verification: contract test.
- **AC4**: Given an output larger than the budget, when returned, then it is truncated with a warning and a way to page. Verification: TDD.
- **AC5**: Given a dry run of init, when executed, then no file changes and the planned changes are listed. Verification: end-to-end.
- **AC6**: Given a ledger fixture, when wayloop cost runs, then totals equal the sum of the events in the range. Verification: TDD.
- **AC7**: Given the same input with keys in different orders, when hashed on two platforms, then `input_hash` is identical. Verification: property test.
- **AC8**: Given a project key in the user file, when the configuration is read, then it is refused with the pointer and the fix. Verification: TDD.
- **AC9**: Given a value set by an environment variable, when `wayloop config show` runs, then the value and the source environment are reported. Verification: TDD.

## Interfaces
- schemas/commands/<command>/v<N>.json and v<N>.output.json; schemas/index.json; error codes and reasons documented in the reference (ADR 0009).

## Open questions
- [ ] Human-readable output in addition to JSON, or JSON only with a separate renderer?
