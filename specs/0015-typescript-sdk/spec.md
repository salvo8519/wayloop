# SPEC-0015: TypeScript SDK and npx

| | |
| --- | --- |
| Status | Draft |
| Milestone | M5 · Adapters and distribution |
| Owner | @salvo8519 |
| Related | ADR 0018, #112 |
| Commit scope | typescript |

## Problem
Node and front-end teams need the engine without Rust or Python, and a one-line way to try it.

## Scope
**In scope:** Bindings with napi-rs; typed API; prebuilt binaries per platform; npx wayloop; publication on npm.

**Out of scope:** Browser support.

## User stories
- As a developer, I want npx wayloop init, so that I try Wayloop in one command.

## Requirements (EARS)
- **R1**: THE SDK SHALL expose every command of SPEC-0004 with the same contracts.
- **R2**: THE SDK SHALL ship TypeScript declarations generated from the schemas.
- **R3**: THE PACKAGE SHALL run with npx without a build step on the supported platforms.

## Acceptance criteria
- **AC1**: Given the contract tests, when run through the SDK, then they pass. Verification: contract test.
- **AC2**: Given a clean machine, when npx wayloop --version runs, then it prints the version as JSON. Verification: end-to-end.

## Interfaces
- Package wayloop on npm, with platform-specific optional dependencies.

## Open questions
- [ ] Package name on npm: wayloop or a scoped name?
