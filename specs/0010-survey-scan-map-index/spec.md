# SPEC-0010: Survey of existing code: scan, map and index

| | |
| --- | --- |
| Status | Draft |
| Milestone | M3 · Living specifications and survey |
| Owner | @salvo8519 |
| Related | ADR 0007, ADR 0015, #80, #86, #87, #88 |
| Commit scope | survey |

## Problem
On an existing codebase agents waste tokens rediscovering the structure and invent what they do not find. A deterministic map and a precise index fix both.

## Scope
**In scope:** Scan (modules, languages, entry points, dependencies, token counts); symbol extraction with tree-sitter; import and call graph; ranking of central files; map within a token budget; lexical and structural index in SQLite with full-text search derived from files (ADR 0007, ADR 0015); incremental updates by file hash.

**Out of scope:** Embeddings (ADR 0004); documentation writing (SPEC-0011).

## User stories
- As an agent, I want a map of the codebase within my budget, so that I start from the right files.
- As a maintainer, I want the same question to return the same answer, so that results are reproducible.

## Requirements (EARS)
- **R1**: THE ENGINE SHALL produce the scan of a repository without network access.
- **R2**: THE ENGINE SHALL extract definitions and references for each supported language with tree-sitter.
- **R3**: THE ENGINE SHALL rank files by centrality in the reference graph, personalised by the files of the current task.
- **R4**: THE ENGINE SHALL render a map within a requested token budget.
- **R5**: THE ENGINE SHALL build a full-text and symbol index from files, update it incrementally and rebuild it on demand.
- **R6**: THE ENGINE SHALL answer find, references and history queries with compact JSON within a budget.
- **R7**: THE ENGINE SHALL count tokens with a calibrated estimate and state that it is an estimate.

## Acceptance criteria
- **AC1**: Given the same repository twice, when mapped, then the maps are identical. Verification: property test.
- **AC2**: Given a repository of 100,000 lines, when indexed, then the first index completes within the performance budget of SPEC-0020. Verification: end-to-end.
- **AC3**: Given one changed file, when re-indexed, then only that file is processed. Verification: TDD.

## Interfaces
- wayloop survey scan | map | index; wayloop find.

## Open questions
- [ ] Languages supported in 1.0: Rust, Python, TypeScript, Go, Java; others later?
