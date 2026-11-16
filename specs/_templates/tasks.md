# SPEC-NNNN: Tasks

Rules: one task per commit; `[P]` tasks touch disjoint files and can run in parallel; every
task names its verification technique; tick a task only when `uv run just check` passes.

## Tests (written first where they pay)
- [ ] T1 [P] Tests for AC1 (TDD)
- [ ] T2 [P] Tests for AC2 (contract test)

## Implementation
- [ ] T3 Types with JSON Schema and a doc comment on every field
- [ ] T4 Behaviour, with every decision recorded where the specification requires it

## Adapters and documentation
- [ ] T5 [P] CLI command (end-to-end test)
- [ ] T6 [P] Concept page and Italian translation (none)

## Verification
- [ ] T7 Review by risk tier; every acceptance criterion ticked in spec.md
