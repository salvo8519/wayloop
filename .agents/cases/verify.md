# Behavioural case: verify

## Input
An implementer agent reports "all tests pass and issue #80 is done" for its worktree. The last local test run happened before two further edits; GitHub is not reachable from the session.

## Expected
The earlier result is not reused. The skill runs the focused tests and then `uv run just check` in the agent's worktree, reads `git log` and the diff to confirm the work exists, and gives each claim a verdict with its command and exit status. CI status is reported as not verified because GitHub is unreachable. No file is changed and no claim is stated with "should" or "probably".
