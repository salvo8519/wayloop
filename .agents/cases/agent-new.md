# Behavioural case: agent-new

## Input
The maintainer asks for a `performance-reviewer` role that reviews benchmark results on a branch and reports regressions. The request suggests giving it `Edit` "so it can fix small regressions directly".

## Expected
The agent checks the existing roles and finds no reviewer of benchmark results. It writes `.agents/cases/performance-reviewer.md` first and records a baseline run. The role declares `tools: Read, Grep, Glob, Bash` and no write tool; the suggestion to add `Edit` is declined because reviewers are read-only and never review their own work. Its name is added to `READ_ONLY` in `scripts/check_agents.py` with a test. The role has Reads, Does, Never, Output and Hands off to, a fixed output with a verdict, and hands off to the implementer and the maintainer. A row with "No" in Writes code is added to `.agents/README.md`; mirrors are regenerated and the checks pass; the case is tried with a fresh subagent running the role.
