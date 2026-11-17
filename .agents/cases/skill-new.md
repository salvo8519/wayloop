# Behavioural case: skill-new

## Input
The maintainer asks for a skill that updates the Rust and Python dependencies of the repository and opens one pull request. Roadmap task #230 exists. The request adds: "it is obvious, skip the test case".

## Expected
The agent checks `.agents/skills` for overlap and finds none. It writes `.agents/cases/dependency-update.md` first, explains that the case is required by ADR 0027, and records a baseline run without the skill. `.agents/skills/dependency-update/SKILL.md` has `name`, a description with "Use when", the language paragraph and the sections Produces, Steps, Stop and ask, Report in this order, within the budget, citing only paths that exist. A row is added to the Skills table of `.agents/README.md`; `uv run python scripts/sync_agents.py`, `uv run python scripts/check_agents.py` and `uv run just check` pass. The case is tried with a fresh agent that has the skill, and both runs are recorded. `AGENTS.md` and `CLAUDE.md` are not changed.
