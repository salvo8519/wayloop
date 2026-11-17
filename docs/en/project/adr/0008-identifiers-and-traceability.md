# ADR 0008: Identifiers and traceability between artifacts

Status: Proposed · 2026-11-17 (milestone M0 · Foundations; decision tracked in #4)

## Context

Acceptance criteria, requirements, tasks, commits and pull requests must stay linked when files move or are renamed.

## Decision

Every artifact receives a stable identifier at creation, independent of its file name: a type prefix and a sequence (for example STORY-0042, SPEC-0003 R2, TASK-0107). Commits and pull requests cite identifiers. The engine checks links by identifier, never by path.

## Consequences

Links survive refactoring of folders. Identifiers are never reused. A rename is a change of title, not of identity.
