# ADR 0024: Tool definitions for agents: MCP and function calling from one source

Status: Proposed · 2026-11-17 (milestone M1 · Engine core; decision tracked in #110)

## Context

An agent needs more than the list of commands: how to call each one, which parameters, which values each enum accepts and what they mean, and whether the call changes anything. Tool schemas are read by many clients and model providers, and some accept only a subset of JSON Schema.

## Decision

Tool definitions are generated from the command schemas, never written by hand. Every definition: is inlined without $ref; uses one type per type keyword, with an optional value written as anyOf of the type and null; carries the meaning of every enum value in the field description; carries a minimal and, where useful, a maximal input example taken from golden cases; states units and formats in descriptions; carries annotations readOnlyHint, destructiveHint and idempotentHint, and _meta with the command version and status. A compact profile removes only annotations, never a validation keyword. A portable variant serves providers with limited JSON Schema support. Errors are returned with isError and the whole error report as structured content. CI drives the MCP server with the MCP Inspector in strict mode and fails on a single warning.

## Consequences

Every agent and every provider sees the same contract and can correct a call in one round. Each provider format becomes a contract to test.
