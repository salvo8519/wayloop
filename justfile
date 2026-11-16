# Run with `uv run just <recipe>` (just is installed by uv).

default:
    @just --list

# Full local gate: run before every pull request (CI runs the same checks).
check: style i18n rust
    @echo "All checks passed."

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
    ruff format scripts

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
