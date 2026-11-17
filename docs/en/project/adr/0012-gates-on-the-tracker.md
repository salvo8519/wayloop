# ADR 0012: Approval gates are tracker states, not chat prompts

Status: Proposed · 2026-11-17 (milestone M2 · Tracker and loop; decision tracked in #72)

## Context

A question asked inside a chat session is lost when the session closes, and nobody else on the team sees it.

## Decision

Every gate is a state of the issue with a label and a question in a fixed format. Approval is an event from an authorised person on the tracker. The loop resumes from the recorded step.

## Consequences

Gates are asynchronous, visible and auditable. Answering requires access to the tracker.
