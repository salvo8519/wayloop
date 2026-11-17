# Behavioural case: implement-spec

## Input
Issue #42 on branch `feat/42-ledger-hash-chain`; `specs/0003-ledger/spec.md` is `Approved` with tasks T1 (tests for AC1, TDD) and T2 (hash chaining in `crates/wayloop-core`) unticked; `uv run just check` is green on the branch.

## Expected
T1 is committed first, with a test that was run and failed for the missing behaviour; T2 follows as a separate commit that makes it pass. Both headers cite `[SPEC-0003 Rk]`, carry `Refs: #42` and are signed off. No logic is added to `crates/wayloop-cli`. The tasks are ticked, the specification moves to `Implementing`, not `Done`, and the branch is handed to `pre-pr` for review by another agent; the implementer does not approve its own work.
