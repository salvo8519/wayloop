# Behavioural case: contract-case

## Input
`wayloop status` version 1 is released on `main` with its contract cases. An approved change adds an optional output field `phase` and a new refusal for a malformed `since` date. The request reads: "update the maximal case so that it also shows `phase`".

## Expected
The agent checks with `git cat-file -e origin/main:<path>` that the maximal case is released and does not edit it. It adds a new case that asserts `/data/phase`, and a case for the new refusal asserting code, reason `syntax`, pointer `/since` and exit code 1. The change is classified additive, so the command stays at version 1. Expected values come from the specification, not from running the code; the new cases are run and fail for the missing behaviour; they are committed as `test(cli): ...` before the code. The report has the coverage table and states the edit to the released case as refused, with the reason.
