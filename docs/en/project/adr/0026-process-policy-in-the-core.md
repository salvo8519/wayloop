# ADR 0026: Process policy as registries, rules in the core and a ledger of results

Status: Proposed · 2026-11-17 (milestone M2 · Tracker and loop; decision tracked in #171)

## Context

This repository keeps its process traceable with scripts around the product: commit and branch
conventions, label waivers (release note fragments, documentation impact, translations, issue
links), a registry of labels checked by tests, roadmap consistency and tracker links, sprint
lanes and required checks. Wayloop promises the same discipline to every project it runs, so
the rules must become a capability of the engine, not a template to copy.

Three questions decide the design: where the rules and their lists live, where they are
evaluated, and what is remembered about each evaluation.

Options considered:

1. **Rules in the ledger.** The ledger is append-only history (ADR 0003). Rules must be edited,
   reviewed in a pull request and read in one place; deriving them from events would make the
   current policy a replay result instead of a file a person reads. Rejected as the source of
   rules.
2. **A rule language or user scripts.** Flexible, but evaluation would depend on user code:
   not deterministic across machines, a security surface (ADR 0022), and a second language to
   document. Rejected for 1.0; an external command returning the JSON envelope remains possible
   later as an extension point.
3. **Rules in the core, parameters in a registry, results in the ledger.** Chosen.

## Decision

- **Registry (source of the lists).** One reviewed file, `wayloop.toml` under `[policy]`, with
  a published schema, declares: labels with their role (type, area, status, waiver, gate),
  commit types and scopes, areas with their paths, required checks, and the rules enabled with
  their parameters and waivers. Its defaults are the policy of this repository. Every derived
  file (labeler configuration, issue form areas, required checks list, the labels table of the
  conventions page, the labels on the tracker) is generated from it and checked against it.
- **Rules (decision).** Each rule is a pure function in `wayloop-core`: input is a snapshot
  assembled by adapters (changed paths, commit messages, pull request title, body and labels,
  plan, tracker state, workflow definitions); output is the error report of ADR 0009 with
  pointer, reason and fix. A rule declares the paths it applies to, the evidence it requires
  and the waiver that lifts it (a label or a commit trailer, optionally with a required
  reason). Adapters fetch data; they never decide.
- **Ledger (memory).** Every evaluation on a pull request and every waiver used is an event:
  rule, result, pull request, commit, author, reason. Waivers become auditable, and a rule
  waived beyond a threshold produces a reconciliation proposal (SPEC-0007).
- **One command.** `wayloop check` runs the whole policy, offline or with the tracker, in the
  JSON envelope; a reusable CI workflow runs it. This repository migrates from its Python
  scripts to the engine; the scripts' tests become fixtures that the engine must pass.

## Consequences

The policy is one file to read and one command to run, identical locally and in CI, for every
project. Drift between labels, workflows and documentation is prevented by generation instead
of detected by tests. Waivers carry a history that can change the policy. New kinds of rules
need a release of the engine until the extension point exists; the defaults must stay strict
enough for this repository and loose enough to adopt in an existing project, so every rule can
be disabled in the registry with a recorded reason.
