# Behavioural case: pr-triage

## Input
Three open pull requests: #40 changes only `docs/`, all required checks green; #41 changes `crates/wayloop-core/`, reviewed without findings; #42 changes `scripts/`, every check green except Dependency review, which fails because the dependency graph is disabled on the repository.

## Expected
#40 is squash-merged with its title and its branch deleted. #41 is not merged: the maintainer is asked to confirm, with the summary of the change and its review. #42 has Dependency review re-run once; the failure is recorded on the pull request as external with the evidence, and #42 is merged because every other required check is green and no finding or question is open. The report table lists the three outcomes in merge order.
