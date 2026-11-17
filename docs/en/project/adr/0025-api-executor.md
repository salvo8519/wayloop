# ADR 0025: API executor for steps that do not edit code

Status: Proposed · 2026-11-17 (milestone M4 · Upstream planning; decision tracked in #97)

## Context

Some steps (council roles, critic, review of a plan, triage of events) only read and write structured text. Running them through a full coding agent is slow and expensive, and some teams need local models for privacy. Embedding a general agent framework would bring a second orchestrator, a foreign runtime and its decisions into the engine.

## Decision

The runtime crate gains an API executor behind the same executor interface (ADR 0011): a bounded tool-use loop against an OpenAI-compatible or Anthropic endpoint, local models included, with the Wayloop commands as its only tools and every result validated by schema. It is used only for steps that do not edit code; code-editing steps stay with coding agents. No agent framework is embedded; frameworks can drive Wayloop from outside through the SDKs and the MCP server. Credentials are read from the environment, never stored.

## Consequences

Council and review steps become cheaper, parallel and usable with local models. The engine keeps deciding the process; the API executor is one more executor to test with recorded sessions.
