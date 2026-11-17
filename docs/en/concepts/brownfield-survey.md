# Brownfield survey

On an existing codebase the loop starts with one more step, the survey. The engine maps the
code deterministically; agents write documentation backed by evidence; the result becomes the
indexed context of every later step.

!!! note "Status"
    This page describes the design of the survey. No part of it is built today. What comes
    next is in the [roadmap](../project/roadmap.md).

| Step | Done by | Output |
| --- | --- | --- |
| Scan | Engine | Module tree, languages, entry points, dependencies, token count per file |
| Map | Engine | Symbols extracted with tree-sitter, import and call graph, ranking of central files, a map within a token budget |
| Documentation with evidence | Agents, on top of the map | Architecture, conventions, domain glossary, main flows; every claim cites file and line, and whatever cannot be shown goes into a section of unverified items |
| Surveyed specifications | Agents | Living specifications rebuilt from the current behaviour, in state `Surveyed` until the maintainer approves them |
| Gaps | Agents | A document of gaps only (missing tests, undocumented parts, risks), without documenting again what already exists |
| Index | Engine | A lexical and structural index, updated incrementally, queried through the engine |

## Evidence, not plausibility

Answers distinguish observations, limits and unknowns, and "not found" is a valid answer. This
is what keeps an agent from inventing an architecture.

## No embeddings

The index is lexical (BM25) and structural (tree-sitter symbols): fast, deterministic and
repeatable. Embeddings would add a model, a store to maintain and results that are not
repeatable, to help only with natural-language questions. Teams that want semantic search use
an external tool next to Wayloop. See [ADR 0004](../project/adr/0004-no-embeddings.md).

## Saving tokens

- A map within a budget instead of whole files.
- Filtered reads of specifications: only the requirements that matter to the step.
- Compact JSON output from every command.
- For shell output, integration with an existing output compressor rather than a rewrite.
