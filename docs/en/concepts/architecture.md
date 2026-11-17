# Architecture

One engine decides; thin adapters translate. The engine never calls a model, and agents never
write the state by hand. The rule is recorded in
[ADR 0001](../project/adr/0001-rust-engine-thin-adapters.md).

## What exists

| Component | Path | What it does today |
| --- | --- | --- |
| Schemas | `crates/wayloop-schemas` | Rust library with no I/O, home of the contract types that mirror the JSON schemas. It holds no type yet. |
| Core | `crates/wayloop-core` | Rust library with no I/O. It fixes the vocabulary of the issue cycle: the nine steps in order, their stable identifiers, and which step is an approval state. |
| Runtime | `crates/wayloop-runtime` | Rust library that runs the work the core decides ([below](#synchronous-core-asynchronous-runtime)). It holds no code yet. |
| Command line | `crates/wayloop-cli` | The `wayloop` binary, an adapter only: it parses arguments, calls the core and prints JSON. Commands: `version` and `steps` ([reference](../reference/index.md)). |
| Agent content | `.agents/` | Roles and skills in the open Agent Skills format, with generated mirrors for Claude Code. They are used to build Wayloop itself. |
| Process | `roadmap/`, `specs/`, `sprints/`, `scripts/` | The plan as code, synchronised with the GitHub issues; specifications with numbered requirements; sprint lanes of disjoint paths; the checks run by CI. |

Wayloop has no release yet. The components of the engine, and the order in which they are
built, are in the [roadmap](../project/roadmap.md).

## Crate layout

The engine is a Cargo workspace of four crates. Each one has a single role, and dependencies
point in one direction only: towards the pure crates.

```text
wayloop-cli        adapter: translates a channel, decides nothing
     |
     v
wayloop-runtime    I/O, no decisions
     |
     v
wayloop-core       decisions, no I/O
     |
     v
wayloop-schemas    contract types, no I/O
```

| Crate | Role | May depend on | Performs I/O |
| --- | --- | --- | --- |
| `wayloop-schemas` | Contract types of the JSON schemas under `schemas/` | no Wayloop crate | No |
| `wayloop-core` | All decision logic: artifact model, state machine, ledger rules | `wayloop-schemas` | No |
| `wayloop-runtime` | Queue, parallel workers, tracker calls, single writer to the ledger | `wayloop-core`, `wayloop-schemas` | Yes |
| `wayloop-cli` and the other adapters | Translation between a channel and the engine | any of the above | Yes |

- **One entry point.** The core exposes one function to every channel: command, canonical
  input and context in, envelope out
  ([ADR 0028](../project/adr/0028-one-contract-on-every-channel.md)). It belongs at the root
  of `wayloop-core`; adapters call it and only translate its input and its envelope.
- **Allowlisted dependencies.** Each pure crate may depend only on the crates of its reviewed
  allowlist: `wayloop-schemas` on none, `wayloop-core` on `wayloop-schemas` only.
  `scripts/check_core_io.py` resolves each pure crate on its own with `cargo tree` (normal
  dependencies, every target, all its features, the lock file left unchanged) and fails on
  every crate reached outside the allowlist, printing the path that reaches it and naming
  known I/O crates. The allowlists also enforce the direction of the table above: a pure crate
  never depends on the runtime or an adapter, and the schemas never depend on the core. A
  feature that another crate of the workspace enables on a shared dependency is not blamed on
  a pure crate. Development dependencies (tests only) and build dependencies (run at build
  time) are not part of the shipped crate and are skipped. Extending an allowlist is a
  reviewed change to the script.
- **No I/O in the source.** The compiler refuses it. Each pure crate forbids at its root the
  Clippy lints `print_stdout`, `print_stderr`, `dbg_macro`, `disallowed_methods` and
  `disallowed_types`, and its `clippy.toml` lists the parts of the standard library that
  perform I/O or read ambient state: files (`std::fs`), the network (`std::net`), processes
  (`std::process`), the environment (`std::env`), the standard streams, threads, clock reads
  (`SystemTime::now`, `Instant::now`) and hash maps seeded at random (`HashMap`, `HashSet`;
  `BTreeMap` and `BTreeSet` are used instead). The lints follow name resolution, so an alias
  or a nested import does not escape them, and `forbid`, unlike `deny`, cannot be lifted by an
  `allow` further in. `cargo clippy` applies them in `just check` and in CI;
  `scripts/check_core_io.py` fails when the attribute or an entry of a `clippy.toml` is
  removed.

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
