# ADR 0023: Stability policy for 1.0

Status: Proposed · 2026-11-17 (milestone M7 · Version 1.0; decision tracked in #131, #132)

## Context

Users and integrators need to know what can change after 1.0.

## Decision

From 1.0 the JSON contracts, error codes, configuration keys and CLI commands follow semantic versioning: breaking changes only in a major version, with one minor version of deprecation warnings first.

## Consequences

Integrations stay working across minor versions. Mistakes in contracts are fixed only with a deprecation cycle.
