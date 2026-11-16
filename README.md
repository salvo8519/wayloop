# Wayloop

**Spec-driven agentic loop.**

From idea to merged code, in your issues, with humans at the gates.

English | [Italiano](README.it.md) · [Documentation](https://salvo8519.github.io/wayloop/) · [Roadmap](https://salvo8519.github.io/wayloop/project/roadmap/)

> Status: pre-alpha, design phase. Nothing is installable yet. The architecture is described
> in the documentation and the first milestone is being planned in the open.

## Why Wayloop

Coding agents now write code quickly. Projects still fail in the work around the code: intent
that was never written down, plans that drift away from what is actually built, sessions that
cannot be resumed by the next agent, and costs that nobody measures.

Two families of tools address half of the problem each.

- **Spec-driven tools** capture intent before code is written, but stop at the plan file.
- **Agentic loops** keep an agent working until a goal is met, but do not know the plan of the
  project they are working on.

Wayloop joins the two at the scale of a whole project: a deterministic engine runs the loop,
the plan lives in your issue tracker, and people decide at the gates.

## How the loop works

1. **Decompose.** An idea, or an existing codebase, becomes milestones, epics, user stories,
   specifications, tasks and issues. Agents write each level; the engine validates it before
   the next level starts.
2. **Execute.** Every issue follows the same cycle: claim, re-analysis, critical
   counter-review, task checklist, build, deterministic verification, review by risk, pull
   request.
3. **Reconcile.** Every event (a merged pull request, a closed issue, a new request, work
   discovered during development) produces a proposed change to the roadmap. Nothing changes
   the plan silently.
4. **Re-plan.** Once a change is approved, the engine recomputes dependencies, the next tasks
   and the progress of each milestone, and updates the tracker.

## What makes it different

| Principle | In practice |
| --- | --- |
| The engine decides, agents write | A state machine written in Rust chooses every step and verifies it with deterministic checks. A model never declares its own work finished. |
| Your tracker is the database | Plan, progress, decisions and questions live in GitHub issues. Specifications live in the repository. Only session checkpoints stay on the local machine. Remove Wayloop and the project still reads. |
| Humans at the gates | Approval states that the loop cannot pass on its own: the council report, the roadmap, an approved specification, a merge. Agents ask questions on the issue, in a fixed format. |
| A deterministic ledger | Append-only, hash-chained events. The state is a function of the ledger. Session summaries are validated JSON, not free prose. |
| Tokens are a budget | Every phase has one, and the ledger measures the cost of each issue and each phase. |
| Greenfield and brownfield | For an existing codebase, a survey step maps the code deterministically and writes documentation that cites file and line for every claim. No embeddings. |

## Planned architecture

- **Core and CLI** in Rust: one engine, one binary, JSON in and out.
- **Python SDK** (PyO3) and **TypeScript SDK** (napi-rs, available through `npx`).
- **MCP server**, so that any compatible agent can use the engine.
- **Plugins and skills** for Claude Code and Codex, in the open Agent Skills format.
- **GitHub integration**: reusable workflows for validation, and remote execution of issues.

Adapters translate; only the engine decides. The reasoning behind each choice is recorded in
the [architecture decisions](https://salvo8519.github.io/wayloop/project/adr/).

## Roadmap

| Milestone | Scope |
| --- | --- |
| M0 Foundations | Repository, documentation site, schemas of the artifacts, `init` |
| M1 Execution | State machine, ledger, issue cycle, sprints with lanes, session hand-over |
| M2 Living specifications | Survey of existing codebases, specifications and change deltas, consistency checks |
| M3 Upstream | Brief, expert council, roadmap from the council report, user stories |
| M4 Distribution | Python and TypeScript SDKs, MCP server, plugins, release 0.1 |

The detailed plan lives in the repository issues and on the
[roadmap page](https://salvo8519.github.io/wayloop/project/roadmap/).

## Documentation

The documentation is published in English and Italian and rebuilt automatically on every
change to `main`: [English](https://salvo8519.github.io/wayloop/) ·
[Italiano](https://salvo8519.github.io/wayloop/it/). Agents can read
[`llms.txt`](https://salvo8519.github.io/wayloop/llms.txt).

## Contributing

Wayloop is built with the process it describes. Read [CONTRIBUTING.md](CONTRIBUTING.md) and
[AGENTS.md](AGENTS.md) before opening a pull request.

## License

[Apache License 2.0](LICENSE).
