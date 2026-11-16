# ADR 0006: Synchronous pure core, asynchronous runtime

Status: Proposed · 2026-11-17

## Context

Many development tasks are independent and can run at the same time: issues in different
lanes, council opinions, documentation of separate modules, reviews. Tracker calls are slow and
rate-limited and benefit from batching. At the same time, the state of the loop must remain
deterministic.

## Options

1. **Everything asynchronous**, with the state machine itself built on async tasks. Concurrency
   bugs would reach the state, and replaying the ledger would no longer be a simple fold.
2. **An external queue** (a message broker or a database). One more service to install and
   operate, against the local-first goal.
3. **A synchronous pure core inside an asynchronous runtime.** The core computes the next state
   and the work to do; the runtime executes the work concurrently and returns results to a
   single writer.

## Decision

Option 3.

- The core stays synchronous, pure and without I/O.
- The runtime (tokio) derives the queue of ready work from the state, runs workers in parallel
  up to a configurable limit, each issue in its own git worktree, and groups tracker calls.
- One writer validates every result and appends it to the ledger in order.
- Independent, non-interactive steps may use the batch APIs of model providers where they
  exist.
- Parallel safety comes from the plan: sprint lanes declare disjoint paths, checked before the
  sprint starts and again on every pull request.

## Consequences

- Throughput grows with independent work without giving up replayable state.
- No external service to run.
- The limit on parallel agents is a cost decision as much as a technical one; it is set per
  project.
