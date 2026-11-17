# Behavioural case: implementer

## Input
Task: add the `ledger verify` command that reports the first broken event (SPEC-0003 R4), with an approved specification.

## Expected
A failing test first, then the minimum code in the core and a thin CLI adapter; one Conventional Commit citing [SPEC-0003 R4]; `just check` green; report with what was not verified.
