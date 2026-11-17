# Run with `uv run just <recipe>` (just is installed by uv).

default:
    @just --list

# Full local gate: run before every pull request (CI runs the same checks).
check: style i18n python process rust
    @echo "All checks passed."

# Python of the process: lint (error handling included), format, strict types.
python:
    ruff check scripts tests .claude/hooks
    ruff format --check scripts tests .claude/hooks
    mypy

# Process: tests of the process scripts, YAML, task ids, roadmap, sprint files, no I/O in the core.
process:
    python -m pytest tests/process
    python scripts/check_core_io.py
    python scripts/required_checks.py check
    python scripts/docs_sync.py coverage
    python scripts/docs_sync.py claims
    python scripts/sync_agents.py --check
    python scripts/check_agents.py
    python scripts/check_yaml.py
    python scripts/check_task_ids.py
    python scripts/roadmap.py check
    for f in sprints/S*.yaml; do [ ! -e "$f" ] || python scripts/sprint.py validate "$f" || exit 1; done

# Regenerate the canonical roadmap file.
roadmap:
    python scripts/roadmap.py render

# Briefing for a new session: branch, issue, journal, pull requests, sprint.
state:
    python scripts/project_state.py

# House style: no icons or emoji anywhere in the repository.
style:
    python scripts/check_style.py

# Italian translations current against their English source.
i18n:
    python scripts/i18n.py check

# Rust: format, lints, tests.
rust:
    cargo fmt --all --check
    cargo clippy --workspace --all-targets -- -D warnings
    cargo test --workspace

# Format everything.
fmt:
    cargo fmt --all
    ruff check --fix scripts tests .claude/hooks
    ruff format scripts tests .claude/hooks

# Build both documentation sites (English and Italian), strict.
docs:
    python scripts/docs.py build

# Live preview of the documentation: `just docs-serve` or `just docs-serve it`.
docs-serve lang="en":
    python scripts/docs.py serve --lang {{lang}}

# Translation status of every page.
i18n-status:
    python scripts/i18n.py status

# Technical changelog from Conventional Commits.
changelog:
    git-cliff -o CHANGELOG.md

# Preview of the release notes built from the fragments in changes/.
release-notes:
    towncrier build --draft --version Unreleased
