# ADR 0013: One worktree per issue; lanes as the unit of parallelism

Status: Proposed · 2026-11-17 (milestone M2 · Tracker and loop; decision tracked in #67)

## Context

Parallel agents in one working tree overwrite each other's branch and index.

## Decision

Every running issue has its own git worktree. Parallel work is planned in lanes that declare disjoint paths, checked before the run and on every pull request. Generated files belong to one lane only.

## Consequences

Parallel agents never edit the same file. Disk usage grows with parallelism and is reported.
