# ADR 0018: Bindings through one JSON boundary

Status: Proposed · 2026-11-17 (milestone M5 · Adapters and distribution; decision tracked in #105, #111, #112)

## Context

Python and Node users need the engine as a library without a second implementation.

## Decision

Python bindings use PyO3 and maturin; Node bindings use napi-rs. Both cross the boundary with the JSON contracts of the CLI, so every adapter is tested by the same contract tests.

## Consequences

One behaviour in every language. A small serialization cost at the boundary.
