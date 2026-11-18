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

The pre-commit hook formats the staged files and applies the safe fixes of ruff: a commit with
unformatted code stops once with the files already fixed; stage them and commit again. CI runs
the same tools in check mode. `uv run just coverage` measures line coverage and needs the
`llvm-tools-preview` component of rustup (`rustup component add llvm-tools-preview`).

## Working with Claude Code

The project settings in `.claude/settings.json` register these hooks:

- at session start, a short briefing: branch, issue, latest hand-over and its next step;
- before each request, a reminder to propose roadmap changes before doing planned work;
- before each edit, a released contract snapshot (`schemas/**/v<N>.json`) cannot be changed:
  the next version is written instead;
- after each edit, the written file is formatted (rustfmt for Rust, ruff for Python);
- before the session or a subagent ends, the fast checks run on the changed files (rustfmt and
  clippy on the touched crates, ruff and mypy on changed Python) and send their findings back,
  never twice for the same findings and at most three times per session;
- before the session ends, `scripts/docs_sync.py impact` lists changes that still need their
  documentation, and a session that committed today without a hand-over in
  `.agents/journal/` is asked to write it.

The hooks never block on a missing tool: they report it and the work goes on.

On GitHub, every pull request that is ready for review receives an automated first review
against `.agents/agents/reviewer.md` and AGENTS.md; the label `docs-sync` asks Claude to bring
the documentation of a pull request in line; maintainers and collaborators can mention
`@claude` in issues and pull requests. All three need the repository secret
`ANTHROPIC_API_KEY` and are skipped or inactive without it.

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
