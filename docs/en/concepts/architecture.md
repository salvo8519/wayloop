# Architecture

One engine decides; thin adapters translate. The engine never calls a model, and agents never
write the state by hand.

| Component | Responsibility | Technology |
| --- | --- | --- |
| Core | Artifact model, schemas, validation, dependency graph, state machine, ledger, lanes, complexity analysis, next task, reconciliation logic | Rust, no I/O |
| Runtime | Queue of ready work, pool of workers, limits, batching of tracker calls, executors | Rust, asynchronous (tokio) |
| CLI | `init`, `validate`, `plan`, `issues sync`, `run`, `next`, `status`, JSON output | Rust, single binary |
| Python SDK | The same functions for scripts, notebooks and Python agents | PyO3 and maturin |
| TypeScript SDK | Node integration, `npx` | napi-rs |
| MCP server | The engine for any compatible agent | Rust |
| Agent content | Roles, skills, constitution, templates | `.agents/`, open Agent Skills format, generated mirrors for each agent |
| Tracker adapters | GitHub first, GitLab second | Rust |

## Synchronous core, asynchronous runtime

The core is synchronous and pure: given a state and an event, it returns the next state and
the work to do. All concurrency lives in the runtime around it.

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

See [ADR 0006](../project/adr/0006-synchronous-core-asynchronous-runtime.md).

## Contracts

Every input and output crossing the engine boundary is JSON with a versioned schema. An
adapter that needs a new behaviour asks the engine for it; it never implements it.
