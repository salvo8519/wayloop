# ADR 0007: Files as the source, SQLite as a derived index

Status: Proposed · 2026-11-17

## Context

The ledger, the checkpoints and the session summaries must be found quickly by agents working
in parallel, one per git worktree. Two storage models are common: plain files, such as JSON Lines
transcripts, and an embedded SQLite database.

## Options

1. **Files only.** Readable, diffable, append-only, nothing to install. Searching many files is
   slow and every agent reinvents the search.
2. **SQLite as the source of truth.** Fast queries and full-text search. The data becomes
   binary and opaque, cannot be reviewed in a diff, and concurrent writers from several
   worktrees need care.
3. **Files as the source, SQLite as an index.** Every fact is written to files first; an
   SQLite database with full-text search (FTS5) is built from them, updated incrementally, and
   can be deleted and rebuilt at any time.

## Decision

Option 3.

- The ledger is JSON Lines, the state is a YAML projection, summaries are JSON: these files are
  the only source of truth.
- Runtime files live in the git directory shared by every worktree, so that the orchestrator and every lane read the same data.
- The index is an SQLite file next to them, with full-text search over events, rulings and
  summaries. It is a cache: it is never committed, never edited, and rebuilt with one command.
- Agents never open the database directly. They query it through the CLI and the MCP server
  (`find`, `history`, `rulings`), which return compact JSON within a token budget.

## Consequences

- Searches are fast, and every agent uses the same search instead of its own.
- A corrupted or stale index costs one rebuild, never data.
- The database is a single local file inside the engine: no server, consistent with ADR 0006.
