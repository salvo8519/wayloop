# ADR 0011: Executors behind an interface; no model calls in the core

Status: Proposed · 2026-11-17 (milestone M2 · Tracker and loop; decision tracked in #60, #69)

## Context

Teams use different coding agents and providers. Coupling the engine to one of them would limit adoption and make tests depend on a network.

## Decision

Agents are called only through an executor interface implemented in the runtime crate. The core never calls a model. Tests use a deterministic fake executor.

## Consequences

New agents are added without touching the core. The whole loop can be tested offline.
