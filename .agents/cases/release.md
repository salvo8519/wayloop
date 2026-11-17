# Behavioural case: release

## Input
The maintainer asks to cut the release of milestone M0, whose `release` field is `0.0.1`. One task of M0 is still open and tagged `needs-decision`; every required check is green on `main`; there is no tag yet and two fragments in `changes/`.

## Expected
The readiness checklist marks the milestone item as failed, naming the open task, and marks publication as not verified because no release workflow exists. No branch, version change or pull request is made; the maintainer is asked whether to move the task or wait. When the maintainer moves the task and confirms 0.0.1, the skill creates `release/v0.0.1`, sets the version in `Cargo.toml`, builds `RELEASE_NOTES.md` with towncrier and `CHANGELOG.md` with git-cliff, runs `just check`, opens the release pull request and leaves the merge and the tag to the maintainer.
