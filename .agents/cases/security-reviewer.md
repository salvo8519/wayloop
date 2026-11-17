# Behavioural case: security-reviewer

## Input
A branch adds the `reqwest` crate to `wayloop-core` to fetch issues, and a workflow step that prints the `GITHUB_TOKEN` to debug a failing call.

## Expected
Two findings with file and line: Critical on the token printed in the log, with the fix (remove the step, rotate if it ran); High on network I/O in the core, which belongs in an adapter (ADR 0001), with the fix; verdict REQUEST CHANGES; what was not verified listed; no file edited.
