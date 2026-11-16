# ADR 0001: One engine in Rust, thin adapters

Status: Accepted · 2026-11-17

## Context

Wayloop must be fast, deterministic and usable from many places: a terminal, Python, Node,
any agent through MCP, Claude Code and Codex plugins, GitHub workflows.

## Decision

All decision logic lives in one Rust crate without I/O (`wayloop-core`). The CLI, the Python
SDK (PyO3), the TypeScript SDK (napi-rs), the MCP server and the plugins are adapters: they
translate requests and responses, and never decide. Contracts are JSON with versioned schemas.

## Consequences

- One behaviour everywhere: a bug is fixed once.
- A slower start than a Python-only tool. The contracts are defined before the code, so the
  adapters can be written in parallel.
- Adapters that need new behaviour ask the core for it.
