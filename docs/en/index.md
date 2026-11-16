---
hide:
  - navigation
  - toc
---

<div class="wl-hero" markdown>

![Wayloop](assets/brand/mark.svg){ width="96" }

# Wayloop

<p class="wl-tagline">Spec-driven agentic loop.</p>

<p class="wl-promise">From idea to merged code, in your issues, with humans at the gates.</p>

Wayloop runs a software project with coding agents, from the first idea to the merged pull
request. Specifications capture intent, a deterministic loop moves the work forward, the plan
lives in your issue tracker, and people decide at the gates.

[How the loop works](concepts/the-loop.md){ .md-button .md-button--primary }
[Architecture](concepts/architecture.md){ .md-button }

</div>

!!! note "Status"
    Pre-alpha, design phase. Nothing is installable yet. This site describes the architecture
    and follows the project as it is built.

<div class="grid cards wl-features" markdown>

-   __The engine decides__

    ---

    A state machine written in Rust chooses every step and verifies it with deterministic
    checks. A model never declares its own work finished.

    [Architecture](concepts/architecture.md)

-   __Your tracker is the database__

    ---

    Plan, progress, decisions and questions live in your issues; specifications live in the
    repository. Remove Wayloop and the project still reads.

    [State in the tracker](concepts/state-in-the-tracker.md)

-   __Humans at the gates__

    ---

    The loop runs unattended between approval states and stops at them, with a question in a
    fixed format on the issue.

    [Gates](concepts/gates.md)

-   __A deterministic ledger__

    ---

    Append-only, hash-chained events. The state is a function of the ledger, and every session
    hands over a validated summary.

    [Ledger](concepts/ledger.md)

-   __Greenfield and brownfield__

    ---

    On an existing codebase, a survey maps the code and writes documentation that cites file
    and line for every claim. No embeddings.

    [Brownfield survey](concepts/brownfield-survey.md)

-   __Tokens are a budget__

    ---

    Every phase has a budget and the ledger measures the cost of each issue, so that the cost of
    the loop is known, not guessed.

    [The loop](concepts/the-loop.md)

</div>

## For agents

This documentation is also published as plain text for agents:
[`llms.txt`](https://salvo8519.github.io/wayloop/llms.txt) and
[`llms-full.txt`](https://salvo8519.github.io/wayloop/llms-full.txt).
