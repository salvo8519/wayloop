# SPEC-0011: Documentation with evidence and gap report

| | |
| --- | --- |
| Status | Draft |
| Milestone | M3 · Living specifications and survey |
| Owner | @salvo8519 |
| Related | #81, #89, #90 |
| Commit scope | survey |

## Problem
Generated documentation is useful only if every claim can be checked. Unverifiable claims must be labelled, and existing documentation must not be rewritten.

## Scope
**In scope:** Architecture, conventions, glossary and main flows written by agents on top of the map; evidence model (observation, limit, unknown); citations of file and line; surveyed specifications in status Surveyed; gap report; preservation of existing documentation.

**Out of scope:** The site generator of the target project.

## User stories
- As a new team member, I want an architecture document where every claim cites the code, so that I can trust it.
- As a maintainer, I want a list of gaps only, so that I plan the work that matters.

## Requirements (EARS)
- **R1**: THE ENGINE SHALL require a file and line citation for every claim of the generated documentation.
- **R2**: IF a claim has no citation, THEN THE ENGINE SHALL move it to a section of unverified items.
- **R3**: THE ENGINE SHALL check every citation against the current code before publishing.
- **R4**: THE ENGINE SHALL write surveyed specifications in status Surveyed until approved.
- **R5**: THE ENGINE SHALL produce a report of gaps only, without documenting again what exists.
- **R6**: THE ENGINE SHALL never overwrite existing documentation; additions are marked.

## Acceptance criteria
- **AC1**: Given a document with a citation to a deleted line, when checked, then publication fails with the citation. Verification: TDD.
- **AC2**: Given an existing README, when the survey runs, then the README is unchanged. Verification: end-to-end.

## Interfaces
- wayloop survey document | gaps.

## Open questions
- [ ] Output format of the evidence: front matter, inline markers or both?
