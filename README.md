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

> Status: pre-alpha. There is no release yet. The design is recorded in the
> [architecture decisions](https://salvo8519.github.io/wayloop/project/adr/) and the
> [specifications](specs/README.md); what comes next is in the
> [roadmap](https://salvo8519.github.io/wayloop/project/roadmap/).

## Why Wayloop

Coding agents now write code quickly. Projects still fail in the work around the code: intent
that was never written down, plans that drift away from what is actually built, sessions that
cannot be resumed by the next agent, and costs that nobody measures.

Two families of tools address half of the problem each.

- **Spec-driven tools** capture intent before code is written, but stop at the plan file.
- **Agentic loops** keep an agent working until a goal is met, but do not know the plan of the
  project they are working on.

Wayloop is designed to join the two at the scale of a whole project: a deterministic engine
runs the loop, the plan lives in your issue tracker, and people decide at the gates. The loop
has four movements: decompose an idea or an existing codebase into milestones, epics, stories,
specifications and issues; execute every issue through the same cycle; reconcile every event
into a proposed change to the plan; re-plan once a person approves it. The design is described
in [How the loop works](https://salvo8519.github.io/wayloop/concepts/the-loop/).

## Install

Wayloop has no release yet, so there is nothing to install; the work towards the first
release is in the [roadmap](https://salvo8519.github.io/wayloop/project/roadmap/).

## Quickstart

Not available yet: there is no release to run; see the
[roadmap](https://salvo8519.github.io/wayloop/project/roadmap/).

## Use it from an AI agent

Not available yet: the engine exposes no tools to agents; see the
[roadmap](https://salvo8519.github.io/wayloop/project/roadmap/).

## Commands

No command is released yet. A build from source answers `wayloop version` and `wayloop steps`
([reference](https://salvo8519.github.io/wayloop/reference/)); the commands of the loop are in
the [roadmap](https://salvo8519.github.io/wayloop/project/roadmap/).

## Architecture

One engine decides; thin adapters translate. The engine never calls a model, and agents never
write the state by hand ([ADR 0001](https://salvo8519.github.io/wayloop/project/adr/0001-rust-engine-thin-adapters/)).

| Part | Path | What it does today |
| --- | --- | --- |
| Core | [`crates/wayloop-core`](crates/wayloop-core) | Rust library with no I/O. It fixes the vocabulary of the issue cycle: nine steps in order, their stable identifiers, and which step is an approval state. |
| Command line | [`crates/wayloop-cli`](crates/wayloop-cli) | The `wayloop` binary, an adapter only: it parses arguments, calls the core and prints JSON. |
| Agent content | [`.agents/`](.agents) | Roles and skills in the open Agent Skills format, with generated mirrors for Claude Code. They are used to build Wayloop itself. |
| Process | [`roadmap/`](roadmap), [`specs/`](specs), [`sprints/`](sprints), [`scripts/`](scripts) | The plan as code, synchronised with the GitHub issues; specifications with numbered requirements; sprint lanes of disjoint paths; the checks run by CI. |

Details in [Architecture](https://salvo8519.github.io/wayloop/concepts/architecture/).

## The loop contract

These rules bind every part of Wayloop. They come from the
[constitution](specs/constitution.md) and the architecture decisions, and a change to any of
them needs a new ADR and the maintainer's approval.

1. **Deterministic decisions.** All decision logic lives in the core. The same state and the
   same event always produce the same next state. A model never declares its own work
   finished: a deterministic check does.
2. **JSON contracts.** Every input and output crossing the engine boundary is JSON with a
   versioned schema; a breaking change is a new version, never an edit of the old one. One
   envelope for every command, with stable error codes and exit codes, is set out in
   [ADR 0009](https://salvo8519.github.io/wayloop/project/adr/0009-contract-guidelines/).
3. **Gates.** Approval states cannot be passed by the loop: the council report, the roadmap,
   an approved specification, a merge. Agents ask questions on the issue, in a fixed format.
   In the core today, `merge` is the approval step of the issue cycle.
4. **Ledger.** The ledger is append-only and hash-chained, and written only by the engine.
   The state is a function of the ledger and is tested by replay. Model output enters the
   state only after schema validation.
5. **The tracker is the database.** The plan lives in the issues, the specifications in the
   repository. A project stays readable without Wayloop.
6. **Cost is measured.** Every phase has a token budget. No embeddings and no vector store
   ([ADR 0004](https://salvo8519.github.io/wayloop/project/adr/0004-no-embeddings/)).

The status of each decision is in the
[ADR index](https://salvo8519.github.io/wayloop/project/adr/).

## Built in the open, spec-first

Wayloop is built with the process it describes. Every change starts from an issue in the
[roadmap](roadmap/roadmap.yaml); behaviour of the engine starts from a specification with
numbered requirements ([`specs/`](specs/README.md)); structural choices start from an ADR.
Agents and people follow the same rules ([AGENTS.md](AGENTS.md)), and CI checks the plan, the
conventions, the house style and the documentation on every pull request.

- Start with [CONTRIBUTING.md](CONTRIBUTING.md).
- The documentation is published in [English](https://salvo8519.github.io/wayloop/) and
  [Italian](https://salvo8519.github.io/wayloop/it/), rebuilt on every change to `main`.
  Agents can read [`llms.txt`](https://salvo8519.github.io/wayloop/llms.txt).

## License

[Apache License 2.0](LICENSE).
