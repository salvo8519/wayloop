# .agents: the project's agent configuration

The single, tool-agnostic source of truth for AI agents working on Wayloop. Project-wide
instructions are in `AGENTS.md` (all agents) and `CLAUDE.md` (Claude Code specifics).

| Path | Content |
| --- | --- |
| `agents/<name>.md` | Base roles, used as subagents |
| `skills/<name>/SKILL.md` | Workflows in the open Agent Skills format, read natively by Codex, Gemini CLI, Copilot and others |
| `journal/` | Session hand-overs, one file per session |

## Base roles

| Role | Responsibility | Writes code |
| --- | --- | --- |
| `analyst` | Briefs, user stories and acceptance criteria | No |
| `architect` | Structural decisions and ADRs | No |
| `spec-writer` | Specifications with numbered requirements, plans and task lists | No |
| `implementer` | Code and tests from a task list | Yes |
| `reviewer` | Review of a branch against AGENTS.md, the ADRs and the specification | No |
| `critic` | Counter-review: gaps, edge cases, contradictions | No |
| `planner` | Sprints, lanes with disjoint paths, roadmap proposals | Planning files |

## Skills

| Skill | Use |
| --- | --- |
| `session-start` | Briefing, plan, and checkout of the chosen issue with a lease |
| `session-end` | Gate, journal hand-over, pull request, hand-over comment, merge when allowed |

## New roles

A specialised role is added only through a pull request: generated from the common schema of
the files in `agents/`, tested on a small behavioural case, and recorded here. Unused roles are
proposed for removal.

## Mirrors

Claude Code reads `.claude/agents/` and `.claude/skills/`. Both are generated from this folder
by `python scripts/sync_agents.py`, with a notice at the top of each file, and are never edited
by hand: `sync_agents.py --check` fails in CI, in `just check` and in the pre-commit hook when a
mirror is stale, edited or has no source. Mirrors for other agents follow the same rule (#168).
