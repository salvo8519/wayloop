# ADR 0008: Identifiers and traceability between artifacts

Status: Accepted · 2026-11-17 (decided by the maintainer in #4)

## Context

Acceptance criteria, requirements, tasks, commits and pull requests must stay linked when files move or are renamed.

## Decision

Every artifact receives a stable identifier at creation, independent of its file name: a type prefix and a sequence (for example STORY-0042, SPEC-0003 R2, TASK-0107). Commits and pull requests cite identifiers. The engine checks links by identifier, never by path.

An artifact that is also an issue on the tracker (epic, story, task) has two identifiers with
different roles:

- the artifact identifier (`TASK-0107`) is its identity. It is written in the structured block
  of the artifact, survives a move to another tracker and is what the engine uses to link
  artifacts;
- the issue number (`#12`) is the key of the tracker. Branches, commits, pull requests and
  release notes cite it, because the tracker links it and closes it automatically.

The engine records the pair when it creates the issue and checks that it never changes.
Artifacts that live only in the repository (specification, change, brief, council report)
have the artifact identifier only.

## Amendments

- 2026-11-17, at acceptance: the roles of the artifact identifier and of the issue number were
  made explicit.

## Consequences

Links survive refactoring of folders. Identifiers are never reused. A rename is a change of title, not of identity.
