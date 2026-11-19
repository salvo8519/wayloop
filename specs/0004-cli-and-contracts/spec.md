# SPEC-0004: Command-line interface and JSON contracts

| | |
| --- | --- |
| Status | Draft |
| Milestone | M1 · Engine core |
| Owner | @salvo8519 |
| Related | ADR 0009, ADR 0010, ADR 0028, #53, #54, #55, #56, #57, #163, #221, #222 |
| Commit scope | cli |

## Problem
Agents and people drive the engine through commands. Every command must speak the same JSON, fail the same way and stay stable across versions.

## Scope
**In scope:** Command set (init, validate, next, status, history, rulings, run, plan, issues, survey), JSON envelope, error model and exit codes (ADR 0009), configuration file and precedence (ADR 0010), output budget.

**Out of scope:** Language bindings (SPEC-0014, SPEC-0015) and the MCP server (SPEC-0016), except the rules every channel follows (R14 to R17, ADR 0028).

## User stories
- As an agent, I want every command to return JSON with a stable shape, so that I parse it without guessing.
- As a developer, I want clear exit codes, so that scripts and CI can react to failures.

## Requirements (EARS)
- **R1**: THE ENGINE SHALL return every result in the envelope of ADR 0009 C1, with meta.envelope_version and meta.command_version.
- **R2**: THE ENGINE SHALL report every refusal as {code, message, pointer, pointers, reason, expected, fix, source} and list every input error found in one pass in errors; `source` locates the faulty value in the repository (`{file, line}`, `{issue}` or `{milestone}`) and is absent for a document sent directly (SPEC-0001 Errors).
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
- **R14**: THE ENGINE SHALL expose one entry point that takes a command, its canonical input and a context (output budget, read-only mode, dry run) and returns the envelope; every channel SHALL call it and SHALL only translate its native input to the canonical input and the envelope to its native output (ADR 0028).
- **R15**: THE ENGINE SHALL return, for the same input and the same context, the same envelope byte for byte in canonical JSON on every channel and platform, and SHALL place no clock time, host name or absolute path in the envelope.
- **R16**: WHEN a command is refused, EVERY CHANNEL SHALL carry the whole envelope in its native error form: exit code of ADR 0009 C7 for the command line, a typed exception holding the envelope for the SDKs, an error result holding the envelope for the MCP server.
- **R17**: THE PROJECT SHALL check every channel with a channel reconciler that sends each golden contract case through the translation of the channel, rebuilds the envelope with the inverse translation and compares it with the envelope of the entry point, reporting the channel and the JSON pointer of every difference.
- **R18**: WHEN a command is refused, THE ENGINE SHALL put the error report in the top-level field `error` of the envelope and leave `data` out; a result that is not refused has `data` and no `error` (ADR 0009 C1).

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
- **AC10**: Given every golden contract case, when run through the entry point and through the command line, then the two envelopes are identical in canonical JSON (R14, R15). Verification: contract test.
- **AC11**: Given an envelope that holds a clock time, a host name or an absolute path, when checked by the contract runner, then the check fails with the pointer of the value (R15). Verification: contract test.
- **AC12**: Given a refused input, when sent through every channel, then each native error form carries the same envelope (R16). Verification: contract test.
- **AC13**: Given an adapter that drops one output field, when the channel reconciler runs, then it fails naming the channel and the pointer of the field (R17). Verification: contract test.
- **AC14**: Given the same input in the function-calling format of each supported provider, when translated, then every translation gives the same `input_hash` (R14, R15). Verification: contract test.
- **AC15**: Given an input with one unknown field and one missing field, when validated, then the envelope has `error` with both errors in `errors`, no `data`, and exit code 1 (R2, R18). Verification: contract test.

## Interfaces
- schemas/commands/<command>/v<N>.json and v<N>.output.json; schemas/index.json; error codes and reasons documented in the reference (ADR 0009).

## Open questions
- [ ] Human-readable output in addition to JSON, or JSON only with a separate renderer?
