# ADR 0027: One standard and one source for skills, agents and hooks

Status: Proposed · 2026-11-17 (milestone M0 · Foundations; decision tracked in #191)

## Context

Wayloop needs two families of skills, agents and hooks. The process family runs the development
of this repository. The system family ships with the product and is called by the engine at
each step of the loop, in the projects of its users. Both must work with Claude Code, Codex and
the other coding agents that read the open Agent Skills format or an `AGENTS.md` file.

A survey of widely used skill and agent collections shows the same lessons: descriptions that
state when to use the skill decide whether an agent picks it; deterministic commands before
judgement make results repeatable; reviewers that only read and run as a separate agent catch
what the author misses; hooks that block on a missing tool break sessions; hand-written copies
per agent drift.

## Decision

- **One source.** Process skills and agents live in `.agents/skills/` and `.agents/agents/`;
  system skills and agents live in `plugins/skills/` and `plugins/agents/` from M5. Every copy
  for a specific agent (Claude Code, Codex, Cursor, Copilot, Gemini CLI and others) is
  generated and checked (`scripts/sync_agents.py`, #168).
- **Skill format.** Agent Skills `SKILL.md` with `name` and a `description` that names the
  action and starts its trigger with "Use when". The body has, in order: what the skill
  produces; numbered steps, with deterministic commands before judgement; stop conditions and
  the questions that go to the maintainer; the report, which always states what was done, what
  was not done or not verified, and the risks. A skill reads and writes only what its steps
  name, and never changes the global instructions of an agent.
- **Agent format.** `name`, a "Use when" `description`, and the tools it may use, least
  privilege first: reviewers, auditors and explorers are read-only. The body states what it
  reads, what it writes, what it never does, its output format, and to whom it hands off. An
  agent never reviews its own work: review runs in a different agent, of another vendor when
  one is configured (SPEC-0008).
- **Hook format.** One script per purpose, registered per event (SessionStart,
  UserPromptSubmit, PreToolUse, PostToolUse, Stop). A hook calls a deterministic command; it
  never blocks on a missing tool, it reports it; it exits 2 only to send a precise message back
  to the agent. Every hook is tested on real input.
- **Checks.** `scripts/check_agents.py` validates front matter, the "Use when" trigger, the
  required sections, a size budget in tokens, references to commands and files that exist,
  and the registration of every hook. CI runs it with the mirror check.
- **New skills and agents** are added by a pull request with a behavioural case: a small input
  and the expected result. Unused ones are proposed for removal.

## Consequences

Any agent finds and uses the same skills; reports have the same shape whatever skill wrote
them; drift between agents is impossible by construction. Writing a skill costs a behavioural
case and a pass of the check. Agent-specific features that the generators do not support are
not used.
