# ADR 0015: Index: tree-sitter symbols and SQLite full-text search

Status: Proposed · 2026-11-17 (milestone M3 · Living specifications and survey; decision tracked in #87)

## Context

The survey needs fast, repeatable answers on code structure (ADR 0004 excludes embeddings; ADR 0007 makes files the source).

## Decision

Symbols and references are extracted with tree-sitter; files are ranked by centrality in the reference graph; the searchable index is an SQLite database with full-text search, built from files, updated by file hash and rebuildable.

## Consequences

Answers are deterministic and fast. Natural-language search is weaker than with embeddings, by design.
