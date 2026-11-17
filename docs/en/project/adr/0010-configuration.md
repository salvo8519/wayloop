# ADR 0010: One configuration file with layered precedence

Status: Proposed · 2026-11-17 (milestone M1 · Engine core; decision tracked in #56)

## Context

Executors, budgets, roles, limits and tracker settings must be set per project and overridden per run.

## Decision

One file, wayloop.toml, at the repository root, with a published schema. Precedence: command flags, environment variables, project file, defaults. Secrets are never stored in the file.

## Consequences

One place to read and review. Unknown keys are rejected.
