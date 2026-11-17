# SPEC-0017: Plugins and skills for Claude Code and Codex

| | |
| --- | --- |
| Status | Draft |
| Milestone | M5 · Adapters and distribution |
| Owner | @salvo8519 |
| Related | ADR 0020, #108, #114, #115, #168 |
| Commit scope | plugins |

## Problem
Most users will meet Wayloop inside their coding agent. The process must arrive as installable plugins and skills, generated from one source.

## Scope
**In scope:** Claude Code plugin (skills, roles, hooks, MCP registration) published in a marketplace; Codex plugin and skills; skills in the open Agent Skills format; generation from .agents/; session start and stop hooks that call the engine.

**Out of scope:** Plugins for editors without an agent.

## User stories
- As a Claude Code user, I want to install Wayloop with two commands, so that the process works in my next session.
- As a Codex user, I want the same skills, so that my team can mix agents.

## Requirements (EARS)
- **R1**: THE PLUGINS SHALL be generated from .agents/ and never edited by hand.
- **R2**: THE CLAUDE CODE PLUGIN SHALL register the skills, the roles, the hooks and the MCP server.
- **R3**: THE CODEX PACKAGE SHALL provide the same skills in the format Codex reads.
- **R4**: THE HOOKS SHALL call the engine for the next step and never decide themselves.
- **R5**: NO SKILL SHALL change the global instructions of an agent.
- **R6**: THE PLUGINS SHALL include instruction files generated from .agents/ for coding agents beyond Claude Code and Codex: AGENTS.md, Cursor, GitHub Copilot, Gemini CLI, OpenCode, Windsurf and Cline.
- **R7**: THE BUILD SHALL fail when a generated instruction file differs from its source or exceeds its size budget in tokens.

## Acceptance criteria
- **AC1**: Given a clean project, when the plugin is installed and a session starts, then the briefing appears. Verification: end-to-end.
- **AC2**: Given a hand-edited generated file, when CI runs, then it fails. Verification: TDD.
- **AC3**: Given a changed role, when generation runs, then every target changes and CI fails until they are committed. Verification: TDD.

## Interfaces
- Marketplace manifest; installation guides.

## Open questions
- [ ] One marketplace repository or the main repository as marketplace?
