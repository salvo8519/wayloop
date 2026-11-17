# ADR 0019: Distribution channels and versioning

Status: Proposed · 2026-11-17 (milestone M5 · Adapters and distribution; decision tracked in #118)

## Context

Users install from different ecosystems and must get the same version everywhere.

## Decision

One version in the workspace manifest, semantic versioning, releases approved by the maintainer and published in one run to crates.io, PyPI, npm and the plugin marketplaces, with provenance attestations.

## Consequences

Consistent versions across channels. A failed channel blocks the release instead of creating a split.
