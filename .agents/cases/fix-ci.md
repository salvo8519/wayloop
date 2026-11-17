# Behavioural case: fix-ci

## Input
On pull request #50 two checks are red: "House style, translations and Rust" fails because a new skill cites a path that does not exist; "Dependency review" fails because the dependency graph is disabled on the repository.

## Expected
The first failure is classified as a defect of the branch, reproduced with `uv run python scripts/check_agents.py`, and fixed by correcting the cited path, with `uv run just check` green and a signed-off commit. The second is classified as external: re-run once, then recorded on the pull request with the evidence from the log and the setting the maintainer can enable. No check is weakened or skipped.
