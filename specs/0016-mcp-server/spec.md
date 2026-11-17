# SPEC-0016: MCP server

| | |
| --- | --- |
| Status | Draft |
| Milestone | M5 · Adapters and distribution |
| Owner | @salvo8519 |
| Related | ADR 0024, ADR 0028, #107, #110, #113, #222 |
| Commit scope | mcp |

## Problem
Any compatible agent should use the engine without a dedicated plugin, with tools designed for agents rather than for people.

## Scope
**In scope:** Server over stdio and streamable HTTP; a small set of tools (next, status, find, history, rulings, validate, propose); progressive disclosure; budgets on outputs and read-only mode, both passed to the engine as context (ADR 0028).

**Out of scope:** Remote hosting.

## User stories
- As an agent, I want few, well-described tools, so that I choose the right one every time.

## Requirements (EARS)
- **R1**: THE SERVER SHALL generate every tool definition from the command schemas as ADR 0024 states, never by hand.
- **R2**: THE SERVER SHALL describe the meaning of every enum value, carry minimal and maximal examples, and mark each tool read-only, destructive and idempotent as applicable.
- **R3**: THE SERVER SHALL return results as structured content with the whole envelope, and errors with isError and the whole error report.
- **R4**: THE SERVER SHALL report the command version and status in the _meta of each tool.
- **R5**: THE SERVER SHALL offer a compact profile and a selection of tools by area that accept exactly the inputs of the full definitions.
- **R6**: THE SERVER SHALL expose at most a small, documented set of tools by default and more on request.
- **R7**: THE SERVER SHALL pass the output budget of every tool call to the engine as context and return the envelope of the engine unchanged; the engine applies the budget (SPEC-0004 R6, ADR 0028).
- **R8**: THE SERVER SHALL offer a read-only mode that exposes no writing tool.
- **R9**: THE SERVER SHALL pass the protocol conformance checks of the MCP inspector.

## Acceptance criteria
- **AC1**: Given the MCP Inspector in strict mode, when run against the server over stdio and HTTP, then it reports no error and no warning. Verification: contract test.
- **AC2**: Given every example and every copy with one field set to null, when validated by the tool schema and the command schema, then both agree. Verification: property test.
- **AC3**: Given read-only mode, when a writing tool is called, then it does not exist. Verification: TDD.

## Interfaces
- wayloop mcp [--read-only] [--compact] [--areas a,b] [--http]; installation guides per client; ADR 0024.

## Open questions
- [ ] Default tool set: five or seven tools?
