# Wayloop

**Spec-driven agentic loop.** From idea to merged code, in your issues, with humans at the
gates.

Wayloop is an open-source engine that runs a software project with coding agents, from the
first idea to the merged pull request. Specifications capture intent, a deterministic loop
moves the work forward, the plan lives in your issue tracker, and people decide at the gates.

!!! note "Status"
    Pre-alpha, design phase. Nothing is installable yet. This site describes the architecture
    and follows the project as it is built.

## The loop in one page

1. **Decompose**: idea or existing codebase, then milestones, epics, user stories,
   specifications, tasks and issues.
2. **Execute**: every issue follows the same cycle, from claim to pull request.
3. **Reconcile**: every event produces a proposed change to the roadmap.
4. **Re-plan**: after approval, dependencies and next tasks are recomputed and the tracker is
   updated.

Read [The loop](concepts/the-loop.md) for the full picture.

## Principles

| Principle | Where to read more |
| --- | --- |
| The engine decides, agents write | [Architecture](concepts/architecture.md) |
| Your tracker is the database | [State in the tracker](concepts/state-in-the-tracker.md) |
| Humans at the gates | [Gates](concepts/gates.md) |
| A deterministic ledger | [Ledger](concepts/ledger.md) |
| Greenfield and brownfield | [Brownfield survey](concepts/brownfield-survey.md) |

## For agents

This documentation is also published as plain text for agents:
[`llms.txt`](https://salvo8519.github.io/wayloop/llms.txt) and
[`llms-full.txt`](https://salvo8519.github.io/wayloop/llms-full.txt).
