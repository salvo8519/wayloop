<div align="center">

<a href="https://salvo8519.github.io/wayloop/">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="docs/_shared/assets/brand/banner-dark.svg">
    <img alt="Wayloop. Spec-driven agentic loop." src="docs/_shared/assets/brand/banner-light.svg" width="100%">
  </picture>
</a>

**English** · [Italiano](README.it.md)

<h3>The spec-driven agentic loop for whole software projects.</h3>

Your agents write the code. Wayloop runs the project around it —<br/>
from idea to merged code, in your issues, with humans at the gates.

[![CI](https://github.com/salvo8519/wayloop/actions/workflows/ci.yml/badge.svg)](https://github.com/salvo8519/wayloop/actions/workflows/ci.yml)
[![Docs](https://github.com/salvo8519/wayloop/actions/workflows/docs.yml/badge.svg)](https://salvo8519.github.io/wayloop/)
[![Status](https://img.shields.io/badge/status-pre--alpha-0B1F3A)](https://salvo8519.github.io/wayloop/project/roadmap/)
[![License](https://img.shields.io/badge/license-Apache--2.0-F59E0B)](LICENSE)

[![Rust core](https://img.shields.io/badge/core-Rust-12325C?logo=rust)](crates/wayloop-core)
[![Python SDK](https://img.shields.io/badge/SDK-Python%20%C2%B7%20planned-12325C?logo=python&logoColor=white)](https://salvo8519.github.io/wayloop/project/roadmap/)
[![TypeScript SDK](https://img.shields.io/badge/SDK-TypeScript%20%C2%B7%20planned-12325C?logo=typescript&logoColor=white)](https://salvo8519.github.io/wayloop/project/roadmap/)
[![MCP](https://img.shields.io/badge/MCP-planned-12325C)](https://salvo8519.github.io/wayloop/concepts/architecture/)
[![uv](https://img.shields.io/badge/built%20with-uv-12325C)](https://docs.astral.sh/uv/)

[![Spec-driven](https://img.shields.io/badge/development-spec--driven-0F766E)](https://salvo8519.github.io/wayloop/concepts/the-loop/)
[![Agentic loop](https://img.shields.io/badge/agentic%20loop-human--gated-0F766E)](https://salvo8519.github.io/wayloop/concepts/gates/)
[![Docs EN IT](https://img.shields.io/badge/docs-English%20%C2%B7%20Italiano-0F766E)](https://salvo8519.github.io/wayloop/)

**[Documentation](https://salvo8519.github.io/wayloop/)** ·
**[How the loop works](https://salvo8519.github.io/wayloop/concepts/the-loop/)** ·
**[Architecture](https://salvo8519.github.io/wayloop/concepts/architecture/)** ·
**[Roadmap](https://salvo8519.github.io/wayloop/project/roadmap/)** ·
**[Contributing](CONTRIBUTING.md)**

</div>

---

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
