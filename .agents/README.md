# .agents: the project's agent configuration

The single, tool-agnostic source of truth for AI agents working on Wayloop. Project-wide
instructions are in `AGENTS.md` (all agents) and `CLAUDE.md` (Claude Code specifics).

| Path | Content |
| --- | --- |
| `agents/<name>.md` | Base roles, used as subagents |
| `skills/<name>/SKILL.md` | Workflows in the open Agent Skills format, read natively by Codex, Gemini CLI, Copilot and others |
| `journal/` | Session hand-overs, one file per session |
| `cases/<name>.md` | Behavioural case of each skill and agent: an input and the expected result |

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
| `test-engineer` | Tests and contract cases from acceptance criteria, before the code | Tests |
| `docs-writer` | Concept pages, guides and reference, in step with the code | Documentation |
| `translator` | Italian edition from the English source, with the glossary | Documentation |
| `security-reviewer` | Dependencies, workflows, permissions, secrets, injection | No |
| `contract-auditor` | Schemas, envelope, error codes and tool definitions against ADR 0009 and ADR 0024 | No |
| `release-manager` | Release readiness, version, notes, release pull request | Release files |
| `ci-investigator` | Root cause of a red CI run and the smallest correct fix | No |
| `explorer` | Locates code and returns file and line tables | No |
| `simplifier` | Review lens: code that need not exist, missed reuse, over-engineering | No |

## Skills

| Skill | Use |
| --- | --- |
| `session-start` | Briefing, plan, and checkout of the chosen issue with a lease |
| `session-end` | Gate, journal hand-over, pull request, hand-over comment, merge when allowed |
| `implement-spec` | Tasks of an approved specification, test first, one task per commit |
| `run-sprint` | Lanes of a sprint delegated to agents in separate worktrees, merged bottom up |
| `pre-pr` | Full gate, review lenses and the pull request description |
| `commit` | Conventional Commit in English, with a translation for the maintainer |
| `verify` | The commands that prove a claim, run before the claim is made |
| `debug` | Reproduce, isolate, prove the cause, fix at the narrowest layer |
| `apply-review` | Verify each review finding, apply the confirmed ones, record the rejected ones |
| `roadmap-update` | The roadmap change a request implies, proposed and then applied with `roadmap.py` |
| `spec-new` | A specification from the template, with EARS requirements and bounded questions |
| `adr-new` | An ADR with options, decision and consequences, indexed and in the navigation |
| `plan-sprint` | A sprint file with lanes of disjoint paths, validated |
| `pr-triage` | The merge queue: checks, conventions, roadmap, review by tier, merge or confirmation request |
| `explain-changes` | Commits, pull requests or a diff explained in the reader's language |
| `fix-ci` | A red CI run diagnosed, external failures told apart, the smallest correct fix |
| `translate` | The Italian edition brought up to date with the glossary, and stamped |
| `release` | Readiness, version, release notes, changelog and the release pull request |
| `onboarding` | A guided tour for a contributor or evaluator, in their language |
| `retrospective` | A closed milestone reviewed against its evidence, with proposals |
| `learnings` | Reusable learnings of the journal proposed for the agent instruction files |
| `debt` | `shortcut:` comments collected into one list and proposed as tasks |
| `add-command` | Every file a new engine command touches, in order, contract cases first |
| `contract-case` | Golden inputs and outputs of a command, minimal, maximal and per error reason |
| `skill-new` | A new skill to the standard, with its behavioural case |
| `agent-new` | A new role with least privilege and a hand-off, with its behavioural case |

## The standard (ADR 0027)

`python scripts/check_agents.py` enforces it in CI, in `just check` and in the pre-commit hook.

- **Description**: what it does, then its trigger starting with "Use when". Agents pick a skill
  or a role from this line only.
- **Skill sections**, in order: `Produces` (the artifact and its state at the end), `Steps`
  (numbered; deterministic commands before judgement), `Stop and ask` (what goes to the
  maintainer), `Report` (done, not done or not verified, risks).
- **Agent sections**, in order: `Reads`, `Does`, `Never`, `Output`, `Hands off to`; front
  matter `tools` with least privilege. Reviewers, the critic, auditors, the explorer and the CI
  investigator are read-only. No role reviews its own work.
- **Size**: at most about 3000 tokens for a skill and 1500 for an agent.
- **Citations**: every script and repository path in backticks exists; placeholders such as
  `NNNN` or `<slug>` are allowed.
- **Behavioural case**: `cases/<name>.md` with `Input` and `Expected`, used to try the skill or
  role with an agent before it is merged.
- **Hooks**: one script per purpose in `.claude/hooks/`, registered in `.claude/settings.json`,
  executable, never blocking on a missing tool, tested on real input.

## New roles

A specialised role is added only through a pull request: generated from the common schema of
the files in `agents/`, tested on a small behavioural case, and recorded here. Unused roles are
proposed for removal.

## Mirrors

Claude Code reads `.claude/agents/` and `.claude/skills/`. Both are generated from this folder
by `python scripts/sync_agents.py`, with a notice at the top of each file, and are never edited
by hand: `sync_agents.py --check` fails in CI, in `just check` and in the pre-commit hook when a
mirror is stale, edited or has no source. Mirrors for other agents follow the same rule (#168).
