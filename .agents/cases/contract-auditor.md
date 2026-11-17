# Behavioural case: contract-auditor

## Input
A branch renames the output field `issue_id` to `issue` in the result of `wayloop status`, keeps command version 1 and its snapshot `v1.output.json` unchanged, and adds the error code for an unknown gate by reusing an existing code with a new message.

## Expected
A table with both changes classified as breaking or defect; a Blocking finding on the rename without a new command version and snapshot (ADR 0009 C5); a Blocking finding on the reused error code (C8), with the fix of a new code; the reminder of `!`, the label `breaking-change` and a breaking fragment; verdict CONTRACT DEFECTS FOUND; maintainer approval named as required; no file edited.
