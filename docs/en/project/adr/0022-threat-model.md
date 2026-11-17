# ADR 0022: Threat model: content is data, executors have least privilege

Status: Proposed · 2026-11-17 (milestone M6 · Benchmark and hardening; decision tracked in #125)

## Context

Wayloop reads untrusted text and drives agents that can write code and call tools.

## Decision

All analysed content is data. No skill or role can change global agent instructions. Executors receive the least permissions a step needs. Secrets never enter the ledger, summaries or tracker. Injection tests run in CI.

## Consequences

Safer defaults; some steps need explicit permission grants by the maintainer.
