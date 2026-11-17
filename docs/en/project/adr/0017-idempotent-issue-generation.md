# ADR 0017: Generated issues are idempotent by stable identifiers

Status: Proposed · 2026-11-17 (milestone M4 · Upstream planning; decision tracked in #102)

## Context

Plans change and are regenerated; duplicated issues destroy trust in the tracker.

## Decision

Every generated issue carries the identifier of its artifact in its structured block. Generation updates the issue with the same identifier and never creates a second one.

## Consequences

Plans can be regenerated at any time. Deleting an issue by hand requires removing its artifact too.
