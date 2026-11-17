# SPEC-0014: Python SDK

| | |
| --- | --- |
| Status | Draft |
| Milestone | M5 · Adapters and distribution |
| Owner | @salvo8519 |
| Related | ADR 0018, #106, #111 |
| Commit scope | python |

## Problem
Teams automate in Python and run notebooks. They need the engine as a library, with the same behaviour as the CLI.

## Scope
**In scope:** Bindings with PyO3 and maturin; typed API mirroring the commands; JSON contracts; wheels for Linux, macOS and Windows; publication on PyPI.

**Out of scope:** Any decision logic in Python (ADR 0001).

## User stories
- As a developer, I want pip install wayloop and a typed API, so that I script the loop.

## Requirements (EARS)
- **R1**: THE SDK SHALL expose every command of SPEC-0004 with the same inputs and outputs.
- **R2**: THE SDK SHALL contain no decision logic.
- **R3**: THE SDK SHALL ship type information and pass strict type checking.
- **R4**: THE SDK SHALL publish wheels for the supported platforms and Python versions.

## Acceptance criteria
- **AC1**: Given the contract tests of the CLI, when run through the SDK, then they all pass. Verification: contract test.
- **AC2**: Given a clean environment on each platform, when installed from PyPI, then the smoke test passes. Verification: end-to-end.

## Interfaces
- Package wayloop on PyPI.

## Open questions
- [ ] Minimum Python version: 3.12, as in the reference project?
