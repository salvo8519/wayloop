# SPEC-0018: Release pipeline and distribution channels

| | |
| --- | --- |
| Status | Draft |
| Milestone | M5 · Adapters and distribution |
| Owner | @salvo8519 |
| Related | ADR 0019, #109, #118 |
| Commit scope | release |

## Problem
One version must reach crates.io, PyPI, npm and the plugin marketplaces at the same time, verifiably, with notes for developers and for users.

## Scope
**In scope:** Single version source; release readiness check; build matrix; publication to crates.io, PyPI, npm and marketplaces; provenance and checksums; changelog from commits and release notes from fragments; smoke tests from every channel; manual approval by the maintainer.

**Out of scope:** Operating system package managers (after 1.0).

## User stories
- As a maintainer, I want to release with one approval, so that every channel gets the same version.

## Requirements (EARS)
- **R1**: THE PIPELINE SHALL derive every package version from the workspace version.
- **R2**: THE PIPELINE SHALL refuse to publish if any check, translation or release note is missing.
- **R3**: THE PIPELINE SHALL publish to crates.io, PyPI, npm and the plugin marketplaces in one run after maintainer approval.
- **R4**: THE PIPELINE SHALL attach provenance attestations and checksums.
- **R5**: THE PIPELINE SHALL install each published package on Linux, macOS and Windows and run a smoke test.

## Acceptance criteria
- **AC1**: Given a release candidate tag, when the pipeline runs on a test index, then every channel receives the same version. Verification: end-to-end.

## Interfaces
- Workflow release.yml; scripts/release.py ready <version>.

## Open questions
- [ ] Trusted publishing on every registry that supports it?
