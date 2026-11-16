# ADR 0005: English source, documentation in English and Italian

Status: Accepted · 2026-11-17

## Context

Wayloop is an open-source project born in Italy. It needs an international community of
contributors and wants Italian as a first-class language for its users.

## Decision

Code, identifiers, commit messages, issues, specifications and the source documentation are in
English. The documentation site, the README and the templates are also published in Italian.
Every translation records the hash of its English source; `scripts/i18n.py check` fails when a
required translation is stale or missing. The Italian glossary is in `i18n/glossary.yaml`.
Artifacts of a project managed by Wayloop are written in the language chosen in its
configuration.

## Consequences

- One source of truth, two editions kept in step automatically.
- Every change to an English page requires its translation in the same pull request, or the
  label `translation-pending`.
