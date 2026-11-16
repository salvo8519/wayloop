# ADR 0004: No embeddings in the product

Status: Accepted · 2026-11-17

## Context

Many code tools index repositories with embeddings for semantic search. Wayloop needs precise
answers (where a symbol is defined, who calls it, which files a specification touches), fast
and repeatable.

## Decision

The index is lexical (BM25) and structural (tree-sitter symbols and the dependency graph). No
embedding model and no vector store are part of the product. Teams that want semantic search
can connect an external tool through MCP.

## Consequences

- No model to run and no store to maintain for indexing; results are repeatable.
- Natural-language questions about the code are answered less well than with embeddings; the
  brownfield survey documentation and the map compensate for most of them.
