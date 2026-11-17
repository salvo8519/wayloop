# ADR 0028: One contract on every channel, translation adapters and a channel reconciler

Status: Proposed · 2026-11-17 (milestone M5 · Adapters and distribution; decision tracked in #220)

## Context

Wayloop is reached through many channels: the command line, the Python and Node SDKs, the MCP
server, the function-calling formats of model providers, and the plugins and skills of coding
agents. ADR 0001 keeps decision logic in the core and ADR 0018 makes the bindings cross a JSON
boundary, but neither states what every channel must return, and nothing checks it. A channel
that decides, filters or cuts a result gives a different answer for the same request, and the
difference is found by a user. A second contract per channel would be a maintenance cost that
grows with every channel and drifts from the first.

## Options

1. **A contract per channel**, each tested on its own. Many contracts to keep in step; the
   channels drift.
2. **One contract and adapters kept thin by review.** Cheap, but the rule is enforced by
   discipline only.
3. **One contract, adapters that are translation functions only, and a channel reconciler
   that checks every channel against the engine.**

## Decision

Option 3.

- **One contract.** The command schemas under `schemas/` (input, and output in the envelope of
  ADR 0009) are the only contract. Everything a channel exposes is generated from them (ADR
  0024) and never written by hand.
- **One entry point.** The core exposes one function: command, canonical input and context in,
  envelope out. Every channel calls it.
- **Adapters are translation functions.** Each channel has one translation from its native
  form (arguments, a Python dict, a JavaScript object, an MCP tool call, a provider function
  call) to the canonical input, and one inverse from the envelope to its native form (exit
  code, typed exception carrying the envelope, MCP error result carrying the envelope). An
  adapter never decides, filters, reorders or cuts a result.
- **Channel settings are engine parameters.** Output budget, read-only mode and dry run are
  part of the context passed to the entry point; their effect is computed by the engine and
  reported in `meta.defaults_applied` and `diagnostics`.
- **Parity.** The same input and the same context give the same envelope, byte for byte in
  canonical JSON (RFC 8785), on every channel and platform. The envelope therefore holds no
  value that depends on the machine or the moment: no clock time, host name or absolute path.
- **Channel reconciler.** One component sends every golden contract case through the
  translation of each channel, rebuilds the canonical envelope from the native output with the
  inverse translation, and compares it with the envelope of the engine. A difference fails
  with the channel and the JSON pointer. It runs in CI as a required check and locally with
  one command.

## Consequences

- A new channel costs one translation function and its inverse; the cases and the reconciler
  are shared.
- Logic that slips into an adapter is found in CI, not by a user.
- Some features usually built in adapters, such as truncation to a token budget, move into the
  engine.
- Events and results must carry clock times and paths outside the envelope, or as values the
  input supplies.
- ADR 0018 becomes the binding-specific case of this decision.
