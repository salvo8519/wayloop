# Behavioural case: test-engineer

## Input
Task: the ledger refuses an append whose expected revision is stale (SPEC-0003), with an approved specification and no implementation yet.

## Expected
Tests in `wayloop-core` named after their requirements: a stale revision is refused with reason `stale` and its pointer, a current revision is accepted, an empty ledger accepts revision zero; each test run and failing for the missing implementation, not for a compile error elsewhere; a commit `test(ledger): ...`; a table requirement, test, status; no implementation code written.
