# ADR 0014: Living specifications by capability with change deltas

Status: Proposed · 2026-11-17 (milestone M3 · Living specifications and survey; decision tracked in #78, #82)

## Context

Specifications written per feature stop describing the system once the feature ships.

## Decision

The current behaviour lives in one specification per capability. Every change is a delta of requirements (added, modified, removed, renamed), reviewed with the code and merged into the living specification when the change closes.

## Consequences

The behaviour of the system is always readable in one place. Reviews read only the delta.
