# Behavioural case: adr-new

## Input
The maintainer decides that the command line will read configuration only through the core, and asks for an ADR. The highest existing record is 0027.

## Expected
`docs/en/project/adr/0028-<slug>.md` with title, a valid `Status: Proposed` line, Context with the options considered, Decision and Consequences; its row in the ADR index and its entry in the "Decisions (ADR)" navigation of `zensical.toml`, both in numeric order; `uv run python scripts/docs_sync.py coverage` passes. The status stays `Proposed` until the maintainer accepts it.
