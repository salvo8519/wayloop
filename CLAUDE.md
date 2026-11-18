# CLAUDE.md

@AGENTS.md

## Claude Code specifics

- Never use icons, emoji or decorative symbols, in files or in replies about this project.
- Talk to each person in the language they prefer: read `CLAUDE.local.md` if it exists. Write
  everything that goes into the repository in English.
- Roles live in `.agents/agents/`. Delegate instead of doing everything in the main context;
  subagents cannot start other subagents, so the main session is the orchestrator.
- Plan before multi-file changes; keep diffs small and focused on one issue.
- When a design choice is ambiguous, ask the maintainer instead of picking one.
- The main session merges the pull requests it opens, following the Merging section of
  AGENTS.md: small ones when their gate holds; engine, adapter and contract changes only after
  the maintainer confirms in the conversation.

## Agents (`.claude/agents/`, generated from `.agents/agents/`)

| Agent | Use it for |
| --- | --- |
| `analyst` | Stories with acceptance criteria from a brief or an epic |
| `architect` | Structural decisions and ADRs, before cross-cutting changes |
| `spec-writer` | Specifications with EARS requirements, then plan and tasks |
| `critic` | Counter-review of an analysis, a specification or a plan |
| `planner` | Sprints with lanes of disjoint paths, roadmap proposals |
| `test-engineer` | Tests and contract cases from acceptance criteria, before the code |
| `implementer` | Tasks of an approved specification, test first |
| `reviewer` | Review of a branch against AGENTS.md, the ADRs and its specification |
| `simplifier` | Review lens: code that need not exist, missed reuse |
| `contract-auditor` | Schemas, envelope, error codes and tool definitions |
| `security-reviewer` | Dependencies, workflows, permissions, secrets |
| `explorer` | Locating code without editing it |
| `ci-investigator` | Root cause of a red CI run |
| `docs-writer` | Concept pages, guides and reference |
| `translator` | The Italian edition, with the glossary |
| `release-manager` | Release readiness and the release pull request |

Standard loop: `analyst` → `spec-writer` → `critic` → (maintainer approves) → `test-engineer` →
`implementer` → review by tier with an agent other than the implementer → `docs-writer` →
pull request → merge under AGENTS.md (Merging). At most three agents in parallel.

## Skills (`.claude/skills/`, generated from `.agents/skills/`)

Session: `/session-start`, `/session-end`. Planning: `/roadmap-update`, `/spec-new`, `/adr-new`,
`/plan-sprint`. Delivery: `/implement-spec`, `/run-sprint`, `/pre-pr`, `/commit`, `/verify`,
`/debug`, `/apply-review`. Review and integration: `/pr-triage`, `/explain-changes`, `/fix-ci`.
Documentation and release: `/translate`, `/release`, `/onboarding`, `/retrospective`,
`/learnings`, `/debt`. Repeated work: `/add-command`, `/contract-case`, `/skill-new`,
`/agent-new`.

To change an agent or a skill, edit `.agents/` and run `uv run python scripts/sync_agents.py`;
never edit `.claude/agents` or `.claude/skills` directly.

## Hooks (`.claude/settings.json`)

- Session start: a short briefing with the branch, its issue and the next step of the latest
  hand-over.
- Every request: a reminder to propose roadmap changes first (`/roadmap-update`).
- Before an edit: released contract snapshots under `schemas/` cannot be changed; write the next
  version.
- After an edit: `.rs` and `.py` files are formatted.
- Before the session or a subagent ends: the fast checks on the changed files (rustfmt, clippy
  on the touched crates, ruff, mypy); findings go back to the agent, never twice the same and at
  most three times per session.
- Before the session ends: documentation impact, and a hand-over in `.agents/journal/` when the
  session committed.

## Session protocol

Begin with `/session-start`, end with `/session-end`, review the merge queue with `/pr-triage`.
The journal in `.agents/journal/` is how the next session, of any agent, knows where this one
stopped.
