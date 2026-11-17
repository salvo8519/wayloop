# SPEC-0021: Process policy

| | |
| --- | --- |
| Status | Draft |
| Milestone | M2 · Tracker and loop |
| Owner | @salvo8519 |
| Related | ADR 0026, ADR 0009, ADR 0008 |
| Commit scope | core |

## Problem
A project run by agents stays traceable only if names, links, labels, release notes,
documentation and parallel work follow rules that are checked every time. Kept as scripts
around a project, these rules are copied, drift and stop being tested. Wayloop must carry them
as a capability: declared once, evaluated by the engine, remembered in the ledger.

## Scope
**In scope:** the policy registry, rule evaluation, waivers, generated files derived from the
registry, `wayloop check`, policy events in the ledger, migration of this repository from its
process scripts.

**Out of scope:** naming of artifacts and the chain of identifiers (SPEC-0001 R7 to R11, which
this specification reuses); user-defined rule code.

## User stories
- As a maintainer, I want one file that declares labels, scopes, areas, checks and rules, so
  that I review the policy of my project in one place.
- As an agent, I want every violation reported with the field and the fix, so that I correct it
  in one round.
- As a maintainer, I want to see how often each rule is waived, so that I change the rule or
  the habit.

## Requirements (EARS)
- **R1**: THE ENGINE SHALL read the policy from `wayloop.toml` under `[policy]`, validated by a
  published schema, with defaults equal to the policy of this repository.
- **R2**: THE ENGINE SHALL declare for each rule the paths it applies to, the evidence it
  requires and the waiver that lifts it, as a label or a commit trailer, with or without a
  required reason.
- **R3**: THE ENGINE SHALL evaluate rules as pure functions of a snapshot assembled by
  adapters: changed paths, commit messages, pull request title, body and labels, plan, tracker
  state and workflow definitions.
- **R4**: THE ENGINE SHALL provide rules for: release note fragments, documentation impact,
  translations current, the pull request closing one planned task, breaking changes declared,
  roadmap consistency, tracker links, sprint lanes and required checks.
- **R5**: THE ENGINE SHALL report every violation with the error report of ADR 0009.
- **R6**: THE ENGINE SHALL generate from the registry the labeler configuration, the area list
  of the issue forms, the required checks list, the labels table of the conventions page and
  the labels on the tracker, and refuse a derived file that differs from the registry.
- **R7**: THE ENGINE SHALL record every evaluation on a pull request and every waiver used as
  ledger events with rule, result, pull request, commit, author and reason.
- **R8**: WHEN a rule is waived more often than its threshold in a period, THE ENGINE SHALL
  produce a reconciliation proposal (SPEC-0007).
- **R9**: THE ENGINE SHALL allow a rule to be disabled in the registry only with a recorded
  reason.
- **R10**: THE ENGINE SHALL run the whole policy with `wayloop check`, offline or with the
  tracker, in the JSON envelope.

## Acceptance criteria
- **AC1**: Given a change to engine sources without a fragment and without the waiver, when
  checked, then the release note rule fails with pointer and fix. Verification: TDD.
- **AC2**: Given the waiver label, when checked, then the rule passes and a waiver event is
  recorded. Verification: TDD.
- **AC3**: Given a label added to the registry, when generation runs, then every derived file
  changes and a hand-edited derived file is refused. Verification: TDD.
- **AC4**: Given the fixtures of the process scripts of this repository, when the engine checks
  them, then it reports the same problems. Verification: contract test.
- **AC5**: Given three waivers of one rule in a month above a threshold of two, when
  reconciled, then one proposal reaches the maintainer. Verification: TDD.

## Interfaces
- `wayloop check [--online] [--pr <n>]`; `wayloop policy generate`; schema of `[policy]`;
  reusable CI workflow.

## Open questions
- [ ] Is the registry a section of `wayloop.toml` or a separate `policy.toml` beside it?
- [ ] Which waivers require a reason by default?
