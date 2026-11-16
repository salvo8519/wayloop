# .agents: the project's agent configuration

The single, tool-agnostic source of truth for AI agents working on Wayloop. Project-wide
instructions are in `AGENTS.md` (all agents) and `CLAUDE.md` (Claude Code specifics).

| Path | Content |
| --- | --- |
| `agents/<name>.md` | Base roles, used as subagents |
| `skills/<name>/SKILL.md` | Workflows in the open Agent Skills format (from milestone M1) |

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

## New roles

A specialised role is added only through a pull request: generated from the common schema of
the files in `agents/`, tested on a small behavioural case, and recorded here. Unused roles are
proposed for removal.

Mirrors for each agent tool (for example `.claude/agents/`) will be generated from this folder;
they are never edited by hand.
