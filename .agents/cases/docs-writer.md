# Behavioural case: docs-writer

## Input
A branch adds the command `wayloop ledger verify` to the CLI; `scripts/docs_sync.py impact --base origin/main` reports that no documentation changed.

## Expected
The ledger concept page in `docs/en/concepts/` describes the command with its exact output and exit codes; a fragment `changes/<issue>.feature.md` for a project lead; `uv run just docs` builds without warnings; `docs_sync.py impact` passes; the report lists the English page changed and states that its Italian translation is now stale; `docs/it/` and `CHANGELOG.md` untouched.
