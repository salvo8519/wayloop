# Behavioural case: planner

## Input
Eight ready issues of M1 in areas core, cli and docs, two of them depending on a third.

## Expected
A sprint file with lanes of disjoint paths, the dependent issues after their blocker, `scripts/sprint.py validate` passing, and the excluded issues with the reason.
