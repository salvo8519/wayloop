# SPEC-0022: Skill and agent catalog of the system

| | |
| --- | --- |
| Status | Draft |
| Milestone | M5 · Adapters and distribution |
| Owner | @salvo8519 |
| Related | ADR 0027, SPEC-0017, SPEC-0008, #200, #201, #202, #203, #204 |
| Commit scope | plugins |

## Problem
The engine decides each step of the loop; a coding agent performs it. The agent needs, for
every step, a skill that tells it what to read, which commands to run, what to produce and how
to report, and roles for the work it delegates. Without a catalog that maps steps to skills
and roles, each project would write its own and the loop would behave differently everywhere.

## Scope
**In scope:** the skills and agents shipped with the plugins, one per step or family of steps
of the loop; their mapping to engine steps; their reports; the hooks that connect a session to
the engine.

**Out of scope:** the process skills of this repository (`.agents/`), which follow the same
standard (ADR 0027) but are not shipped.

## User stories
- As a user of any supported coding agent, I want the skill for the step the engine asks for,
  so that the loop behaves the same in my project as in any other.
- As an agent, I want a fixed report format, so that the engine can validate my result.

## Requirements (EARS)
- **R1**: THE PLUGINS SHALL ship one skill for each family of loop steps: planning (brief,
  council, roadmap, stories, specification, clarification, analysis, tasks, issues), delivery
  (checkout, re-analysis, counter-review, checklist, build, verify, review, pull request,
  merge, hand-over), existing code (survey, map, document, gaps), and support (debug, apply
  review findings, learnings, shortcuts, retrospective, status).
- **R2**: THE PLUGINS SHALL ship agents for the roles of the loop: analyst, architect,
  spec-writer, planner, implementer, test engineer, explorer, reviewer lenses (quality,
  intent, tests, silent failures, simplicity, documentation, security, contracts), critic,
  docs writer, translator, release manager, CI investigator.
- **R3**: THE ENGINE SHALL name, for each step it asks an agent to perform, the skill and the
  role to use.
- **R4**: EVERY SKILL SHALL end with a report validated by the schema of its step, stating what
  was done, what was not done or verified, and the risks.
- **R5**: THE PLUGINS SHALL ship hooks that start a session from the engine's briefing, run the
  next step on stop, and refuse edits to frozen contract snapshots.
- **R6**: EVERY SKILL AND AGENT SHALL pass `check_agents.py` and come with a behavioural case.

## Acceptance criteria
- **AC1**: Given each engine step, when the catalog is checked, then the step names a skill and
  a role that exist. Verification: contract test.
- **AC2**: Given a skill report missing the not-done section, when validated, then it is
  refused. Verification: TDD.
- **AC3**: Given each behavioural case, when run with the fake executor, then the expected
  result is produced. Verification: end-to-end.

## Interfaces
- `plugins/skills/<name>/SKILL.md`, `plugins/agents/<name>.md`, `plugins/hooks/`; the step to
  skill map in the engine; report schemas per step.

## Open questions
- [ ] One skill per step, or one skill per family with sections per step?
