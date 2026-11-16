# ADR 0002: The issue tracker is the database of the plan

Status: Accepted · 2026-11-17

## Context

Comparable tools keep the plan in folders of files of their own. The plan is then invisible to
the team, and depends on the tool that wrote it.

## Decision

Roadmap, epics, stories, tasks, progress, decisions and questions live in the issue tracker
(GitHub first, GitLab second). Specifications live in the repository and are linked from the
issues. Only session checkpoints stay local. The engine keeps a local cache of the tracker that
can always be rebuilt and is never a source of truth.

## Consequences

- The project stays readable without Wayloop.
- Gates are asynchronous and visible to the whole team.
- The engine must handle rate limits, free text in issue bodies and concurrent edits; the
  mitigations are described in [State in the tracker](../../concepts/state-in-the-tracker.md).
- GitLab support is partial on the free tier, where dependency relations are not available.
