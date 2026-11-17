# ADR 0020: Plugins and skills generated from one source

Status: Proposed · 2026-11-17 (milestone M5 · Adapters and distribution; decision tracked in #114)

## Context

Each agent reads skills and roles in its own layout; hand-maintained copies drift.

## Decision

Roles and skills live in .agents/ in the open Agent Skills format. Plugins for Claude Code and Codex are generated from it, checked in CI and published to the marketplaces.

## Consequences

One source for every agent. Agent-specific features are limited to what generation supports.
