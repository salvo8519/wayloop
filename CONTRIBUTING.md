# Contributing to Wayloop

Thank you for your interest. Wayloop is in its design phase: the most useful contributions
today are questions, objections and real use cases, opened as issues.

## Before you start

- Read the [vision](https://salvo8519.github.io/wayloop/project/vision/) and the
  [architecture decisions](https://salvo8519.github.io/wayloop/project/adr/).
- Look for an existing issue, or open one describing the problem before writing code.
- Agents and humans follow the same rules: see [AGENTS.md](https://github.com/salvo8519/wayloop/blob/main/AGENTS.md).

## Set up

Install [uv](https://docs.astral.sh/uv/) and Rust stable, then:

```bash
uv sync --all-groups
uv run pre-commit install --hook-type pre-commit --hook-type commit-msg
uv run just check
```

## Working with Claude Code

The project settings in `.claude/settings.json` register three hooks:

- before each request, a reminder to propose roadmap changes before doing planned work;
- after each edit, the written file is formatted (rustfmt for Rust, ruff for Python);
- before the session ends, `scripts/docs_sync.py impact` lists changes that still need their
  documentation, and the session continues until they have it.

The hooks never block on a missing tool: they report it and the work goes on.

## Pull requests

- One issue per pull request; the body says `Closes #<issue>`.
- Conventional Commit title with a scope, for example `feat(ledger): chain events by hash`.
- `uv run just check` passes locally.
- English documentation changes come with the Italian translation, or the label
  `translation-pending`.
- A fragment in `changes/` for every user-facing change, or the label `no-release-note`.
- Commits are signed off (`git commit -s`), under the Developer Certificate of Origin.
- A change to a script in `scripts/` that enforces the process (roadmap, sprints, commit
  conventions, task ids, translations) comes with a test in `tests/process/` for the rule it
  adds or changes.

## Licence

By contributing you agree that your contribution is licensed under the Apache License 2.0.
