# Architecture

One engine decides; thin adapters translate. The engine never calls a model, and agents never
write the state by hand. The rule is recorded in
[ADR 0001](../project/adr/0001-rust-engine-thin-adapters.md).

## What exists

| Component | Path | What it does today |
| --- | --- | --- |
| Core | `crates/wayloop-core` | Rust library with no I/O. It fixes the vocabulary of the issue cycle: the nine steps in order, their stable identifiers, and which step is an approval state. |
| Command line | `crates/wayloop-cli` | The `wayloop` binary, an adapter only: it parses arguments, calls the core and prints JSON. Commands: `version` and `steps` ([reference](../reference/index.md)). |
| Agent content | `.agents/` | Roles and skills in the open Agent Skills format, with generated mirrors for Claude Code. They are used to build Wayloop itself. |
| Process | `roadmap/`, `specs/`, `sprints/`, `scripts/` | The plan as code, synchronised with the GitHub issues; specifications with numbered requirements; sprint lanes of disjoint paths; the checks run by CI. |

Wayloop has no release yet. The components of the engine, and the order in which they are
built, are in the [roadmap](../project/roadmap.md).

## Synchronous core, asynchronous runtime

The split between the core and the runtime around it is the subject of
[ADR 0006](../project/adr/0006-synchronous-core-asynchronous-runtime.md). The core is
synchronous and pure: given a state and an event, it returns the next state and the work to
do. All concurrency lives in the runtime.

- **Queue.** Ready work is derived from the state (dependencies satisfied, lane free), not kept
  in an external broker. There is nothing to install and nothing to lose.
- **Parallel workers.** Issues of different lanes run in parallel, each in its own git
  worktree. Lanes declare disjoint paths, checked before the sprint starts, so parallel agents
  never edit the same files. A configurable limit caps the number of agents running at once.
- **Single writer.** Workers send their results to one writer that validates them and appends
  them to the ledger in order. Parallel execution never makes the state non-deterministic.
- **Batches.** Tracker calls are grouped to respect rate limits. Independent, non-interactive
  steps (council opinions, documentation of a module, reviews) can go through the batch APIs
  of model providers where they exist, which are cheaper and slower.

This repository already works this way by hand: sprint lanes declare disjoint paths, checked by
`scripts/sprint.py`, and every issue is worked in its own git worktree.

## Contracts

Every input and output crossing the engine boundary is JSON with a versioned schema. An
adapter that needs a new behaviour asks the engine for it; it never implements it. The
envelope shared by every command, with its error codes and exit codes, is the subject of
[ADR 0009](../project/adr/0009-contract-guidelines.md).
