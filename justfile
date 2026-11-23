# Run with `uv run just <recipe>` (just is installed by uv).

# Floor of line coverage for the pure crates (wayloop-core, wayloop-schemas), in percent (docs/en/project/conventions.md, Code quality).
pure_coverage_floor := "95"
# The pure crates: no I/O, all decision logic and contract types (AGENTS.md, Repository map).
pure_crates := "wayloop-core wayloop-schemas"
# The Python of the process: scripts, their tests and the Claude Code hooks.
py_paths := "scripts tests .claude/hooks"

default:
    @just --list

# Full local gate: run before every pull request (CI runs the same checks).
check: style i18n python process dup rust schemas
    @echo "All checks passed."

# Python of the process: lint (error handling included), format, strict types. CI passes
# `github` to annotate the findings on the pull request.
python output="full":
    ruff check --output-format {{output}} {{py_paths}}
    ruff format --check {{py_paths}}
    mypy

# Process: tests of the process scripts, YAML, task ids, roadmap, sprint files, no I/O in the core.
process: sprints
    python -m pytest tests/process
    python scripts/check_core_io.py
    python scripts/required_checks.py check
    python scripts/docs_sync.py coverage
    python scripts/docs_sync.py claims
    python scripts/sync_agents.py --check
    python scripts/check_agents.py
    python scripts/check_yaml.py
    python scripts/check_task_ids.py
    python scripts/check_licences.py
    python scripts/roadmap.py check

# Every sprint file valid (sprints/README.md).
sprints:
    for f in sprints/S*.yaml; do [ ! -e "$f" ] || python scripts/sprint.py validate "$f" || exit 1; done

# Duplicated code (jscpd, pinned in package.json and package-lock.json, settings in .jscpd.json).
# Pure crates: no clone beyond .jscpd-baseline.json, which holds only the clones that #299 deals
# with (tests/process/test_duplication.py lists them and keeps the baseline from growing).
# Everything else: at most 1 percent of duplicated lines.
dup:
    npm ls --depth=0 >/dev/null 2>&1 || npm ci --ignore-scripts --no-audit --no-fund
    node_modules/.bin/jscpd --baseline .jscpd-baseline.json --fail-on-new-clones=0 $(printf 'crates/%s ' {{pure_crates}})
    node_modules/.bin/jscpd --threshold 1 crates {{py_paths}}

# Contract snapshots, their histories and their embedded copies, written from the contract types
# (ADR 0030 G6). Deterministic: a second run changes no byte.
generate:
    python scripts/schema_snapshots.py

# Contract snapshots: the committed files equal the generated ones, and against the base only an
# additive change of a current version or a new version (ADR 0030 G5, rule B).
schemas base="origin/main":
    python scripts/schema_snapshots.py --check
    python scripts/schema_snapshots.py --diff-against {{base}}

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

# Rust: format, lints, tests, API documentation.
rust: && rustdoc
    cargo fmt --all --check
    cargo clippy --workspace --all-targets -- -D warnings
    cargo test --workspace

# API documentation of the Rust crates; a warning, such as a broken link, fails (CI runs it too).
rustdoc:
    RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps

# Line coverage: Rust tests instrumented (cargo-llvm-cov, needs the llvm-tools-preview component),
# the floor of the pure crates, and the process scripts (pytest-cov). `just coverage 100` tries a floor.
coverage floor=pure_coverage_floor:
    cargo llvm-cov --workspace --no-report
    cargo llvm-cov report --summary-only
    just coverage-floor {{floor}}
    python -m pytest tests/process --cov=scripts --cov-report=term

# Fails when the line coverage of a pure crate, measured by the last instrumented run, is below the
# floor; each crate is held to the floor on its own, so one cannot hide the gaps of the other. The
# first report of each crate lets an error of the tool through as itself, not as a coverage failure.
coverage-floor floor=pure_coverage_floor:
    #!/usr/bin/env bash
    set -euo pipefail
    for crate in {{pure_crates}}; do
        cargo llvm-cov report --package "$crate" --summary-only >/dev/null
        cargo llvm-cov report --package "$crate" --summary-only --fail-under-lines {{floor}} >/dev/null || { echo "Line coverage of $crate is below the floor of {{floor}} percent: add the missing tests." >&2; exit 1; }
    done

# Mutation testing of the pure crates (cargo-mutants); surviving mutants in target/mutants/mutants.out/missed.txt.
mutants:
    mkdir -p target/mutants
    cargo mutants $(printf -- '--package %s ' {{pure_crates}}) --no-shuffle --output target/mutants

# Format everything.
fmt:
    cargo fmt --all
    ruff check --fix {{py_paths}}
    ruff format {{py_paths}}

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
