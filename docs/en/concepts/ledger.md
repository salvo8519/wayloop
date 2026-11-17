# Ledger

The ledger is an append-only log of events, written only by the engine and never by a model.
The state of the loop is a function of the ledger: replaying the events from the start always
gives the same state.

!!! note "Status"
    This page describes the design of the ledger, recorded in
    [ADR 0003](../project/adr/0003-deterministic-ledger.md). No code writes or reads a ledger
    today. What comes next is in the [roadmap](../project/roadmap.md).

| Element | Format | Rule |
| --- | --- | --- |
| Event | JSON Lines, one event per line | Fixed fields: sequence, UTC instant, actor, issue, step, outcome, tokens, hash of the input, hash of the previous event |
| Current state | YAML | A projection rebuilt from the ledger, never edited by hand |
| Session summary | JSON validated by a schema | Done, rulings, open questions, next steps, files touched, cost; published on the issue as the hand-over comment |
| Snapshot | JSON | The full state at a point of the ledger, so that months of events are not replayed |

## Why it is deterministic

- **Models do not write the ledger.** Their output enters only after schema validation; invalid
  output becomes an error event, never a corrupted state.
- **Hash chain.** Every event carries the hash of the previous one: an edit or a gap in the log
  is visible immediately.
- **Replay is a test.** The design requires a CI test that replays the ledger and compares
  the state with the YAML projection.

## Long life cycles

A project can last months and hundreds of sessions. Three mechanisms keep memory bounded.

1. **Hierarchical roll-up.** Session summaries merge into the issue summary when the issue
   closes; issue summaries merge into the milestone summary. Every level has a maximum size
   in tokens.
2. **Periodic snapshots.** After a snapshot, earlier events can be archived.
3. **Versioned schemas.** Every event and summary states its schema version; the engine
   migrates older versions.

An agent arriving after weeks reads the milestone summary, the issue summary and the last
hand-over comment: a small context of known size, whatever the history of the project.

## Finding things

The files are the source; an SQLite index with full-text search is built from them and can
always be rebuilt. Agents query it through the engine, never directly. See
[ADR 0007](../project/adr/0007-files-as-source-sqlite-as-index.md).

## Two memories, stated plainly

The local ledger is the operational log of a machine and is lost with it. The durable memory
of the project is the set of summaries published on the issues.
