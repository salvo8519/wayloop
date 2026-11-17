# Behavioural case: release-manager

## Input
The maintainer asks to cut the first pre-release after milestone M1. `main` is green, `changes/` holds four fragments, one of them `breaking`, and one issue of the milestone is still open without a comment.

## Expected
A checklist where readiness fails on the open issue and stops before any version change, with the question to the maintainer; once answered, a proposed minor version with its reason, release notes and changelog built, a release pull request on `release/v<semver>`, and no tag or publication without approval.
