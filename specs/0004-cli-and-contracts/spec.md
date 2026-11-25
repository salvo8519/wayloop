# SPEC-0004: Command-line interface and JSON contracts

| | |
| --- | --- |
| Status | Draft |
| Milestone | M1 · Engine core |
| Owner | @salvo8519 |
| Related | ADR 0009, ADR 0010, ADR 0028, #12, #53, #54, #55, #56, #57, #163, #221, #222 |
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
- **R2**: THE ENGINE SHALL report every refusal as {code, message, pointer, pointers, reason, expected, fix, source, document, related} and list every input error found in one pass in errors; `source` locates the faulty value in the repository (`{file, line}` with `line` only when it is known (the optional line is a breaking change of version 1 of `envelope/result`, which no release holds, made under the amendment of ADR 0030 of 2026-11-20), `{issue}`, `{milestone}`, or `{ledger, line}` for a line of a ledger segment, with its path relative to the git common directory, an additive form decided by the maintainer on 2026-11-22, SPEC-0003 section Contracts of the ledger) and is absent for a document sent directly (SPEC-0001 Errors); `document`, an additive optional field (decided by the maintainer on 2026-11-20), is the index from 0 of the document in the input array `documents` for every error that belongs to a document, whose `pointer` is then relative to that document, and is absent for an error of the input itself; `related`, an additive optional field (decided by the maintainer on 2026-11-25, ADR 0009 amendment of that date), lists the other places of the same condition, each `{document, pointer}` in another document with the members of its `source` in the entry itself, flat so that the envelope keeps at most four levels of objects (ADR 0009 C4), and is absent when there is none, never `[]`.
- **R3**: THE ENGINE SHALL treat absent and null as not given and list every default applied in meta.defaults_applied; an entry of a default that belongs to a document carries `document`, an additive optional field with the meaning it has in R2, its `pointer` then relative to that document (decided by the maintainer on 2026-11-25).
- **R4**: THE ENGINE SHALL snapshot every command input and output schema and bump the command version on a breaking change (ADR 0009 C5).
- **R5**: WHEN a command changes state, THE ENGINE SHALL require the expected revision and refuse a stale one with reason stale.
- **R6**: THE ENGINE SHALL keep the output of every command within a configurable token budget and say when it truncated.
- **R7**: THE ENGINE SHALL read the project file `wayloop.toml` and the user file `~/.config/wayloop/config.toml`, accept only keys marked personal in the user file, and apply the precedence flags, environment (`WAYLOOP_<SECTION>_<KEY>`), user file, project file, defaults (ADR 0010).
- **R8**: WHEN a command changes files or the tracker, THE ENGINE SHALL support a dry run that reports the changes without applying them.
- **R9**: THE ENGINE SHALL publish the JSON Schema of every command input and output.
- **R10**: THE ENGINE SHALL report tokens in and out, cost, duration and turns from the ledger, grouped by issue, phase, role and executor, for a time range (wayloop cost).
- **R11**: THE ENGINE SHALL compute `meta.input_hash` as the SHA-256 of the input serialised as canonical JSON (RFC 8785), after removing the members of objects set to null at any depth (they are not given, ADR 0009 C2; elements of arrays are kept); for text the engine refuses to read as JSON (text that is not JSON, that repeats a key in one object, or that holds a number beyond the range of a double), or when no input was read, as the SHA-256 of the bytes received (decided by the maintainer on 2026-11-20). For such bytes parity across channels is not guaranteed: two channels that send the same text with other line endings or with a byte order mark get different hashes.
- **R12**: THE ENGINE SHALL report the revision, the hash of the latest ledger event, in `meta.revision` of every result that reads state (ADR 0009 C6).
- **R13**: THE ENGINE SHALL return the effective configuration with the source of every value with `wayloop config show`.
- **R14**: THE ENGINE SHALL expose one entry point that takes a command, its canonical input and a context (output budget, read-only mode, dry run) and returns the envelope; every channel SHALL call it and SHALL only translate its native input to the canonical input and the envelope to its native output (ADR 0028).
- **R15**: THE ENGINE SHALL return, for the same input and the same context, the same envelope byte for byte in canonical JSON on every channel and platform, and SHALL place in the envelope no clock time of the call, no host name and no absolute path; recorded times of the state at `meta.revision`, such as `recorded_at` and `merged_at` of the ledger, may be returned (decided by the maintainer on 2026-11-22, SPEC-0003 Q11).
- **R16**: WHEN a command is refused, EVERY CHANNEL SHALL carry the whole envelope in its native error form: exit code of ADR 0009 C7 for the command line, a typed exception holding the envelope for the SDKs, an error result holding the envelope for the MCP server.
- **R17**: THE PROJECT SHALL check every channel with a channel reconciler that sends each golden contract case through the translation of the channel, rebuilds the envelope with the inverse translation and compares it with the envelope of the entry point, reporting the channel and the JSON pointer of every difference.
- **R18**: WHEN a command is refused or fails, THE ENGINE SHALL put the error report in the top-level field `error` of the envelope and leave `data` out; a result that succeeded has `data` and no `error` (ADR 0009 C1, amended on 2026-11-20).

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
- **AC11**: Given an envelope that holds the clock time of the call, a host name or an absolute path, when checked by the contract runner, then the check fails with the pointer of the value; given the same command run at two clock times on the same revision, whose envelope holds the `recorded_at` of a ledger event, then the two envelopes are identical and the check passes (R15). Verification: contract test.
- **AC12**: Given a refused input, when sent through every channel, then each native error form carries the same envelope (R16). Verification: contract test.
- **AC13**: Given an adapter that drops one output field, when the channel reconciler runs, then it fails naming the channel and the pointer of the field (R17). Verification: contract test.
- **AC14**: Given the same input in the function-calling format of each supported provider, when translated, then every translation gives the same `input_hash` (R14, R15). Verification: contract test.
- **AC15**: Given an input with one unknown field and one missing field, when validated, then the envelope has `error` with both errors in `errors`, no `data`, and exit code 1 (R2, R18). Verification: contract test.

## Errors

Codes common to every command (decided by the maintainer on 2026-11-20). The codes of each
command are in its specification and its reference page.

| Code | Reasons | Exit code | Meaning |
| --- | --- | --- | --- |
| `command_input` | `syntax`, `duplicate`, `missing`, `unknown_field`, `type`, `enum`, `length`, `range`, `requires`, `conflict` | 1 | The input of the command is refused: it is not JSON (`syntax`), a number is beyond the range of a double (`range`), it repeats a key (`duplicate`), it breaks the input schema or a rule of the input, or the context requests a version of the command the engine does not run (`enum`, the known versions in `expected`) |
| `usage` | `syntax` | 2 | The channel was not used as documented, for example a command line its adapter does not understand; `command` is the command named, empty when none |
| `input_unreadable` | `missing`, `state` | 3 | The channel could not read the input from the environment: it does not exist (`missing`) or cannot be read (`state`); the envelope holds no path |
| `internal_error` | `internal` | 5 | The engine could not compute the result; the input may be valid |

The command line prints an envelope for every outcome of a command, exit codes 2 and 3
included, with `error` and no `data`; the message for a person goes to standard error. When no
input was read (usage, unreadable input), `meta.input_hash` is the SHA-256 of no bytes.

## Interfaces
- schemas/commands/<command>/v<N>.json and v<N>.output.json; schemas/index.json; error codes and reasons documented in the reference (ADR 0009).
- The context of the entry point carries the requested version of the command (ADR 0028, amended on 2026-11-20): absent, the current version runs and is listed in `meta.defaults_applied` with the reason `current_command_version` and the pointer `""`; a version the engine does not run is refused with `command_input`, reason `enum`, the known versions in `expected` and the pointer `""`, and its envelope reports the current version in `meta.command_version`, the version whose shape it has. When the input is also text the engine cannot read as JSON, one report lists the refusal of the version first, then the errors of the text. The pointer `""` of the default and of the refusal names no member, because the version is a setting of the context and has no place in the input. The envelope of a usage error reports in `meta.command_version` the current version of the command it names, and 1 when it names no known command.

## Open questions
- [ ] Human-readable output in addition to JSON, or JSON only with a separate renderer?
