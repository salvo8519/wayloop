# ADR 0003: An append-only, hash-chained ledger written by the engine

Status: Accepted · 2026-11-17

## Context

Agent sessions end, crash and hand over to each other. The state of the loop must survive
them, and must be the same whoever reads it.

## Decision

Every step produces an event appended to a JSON Lines ledger by the engine only. Each event
carries the hash of the previous one. The current state is a YAML projection rebuilt from the
ledger. Session summaries are JSON validated by a schema and published on the issue. Snapshots
and hierarchical roll-ups bound the size of the memory over long projects.

## Consequences

- The state is reproducible and testable by replay.
- Model output that does not validate becomes an error event, never a corrupted state.
- The local ledger is lost with the machine; the durable memory is the set of summaries on the
  issues.
