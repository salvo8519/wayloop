# Behavioural case: pre-pr

## Input
Branch `feat/61-state-transitions` for issue #61 changes `crates/wayloop-core/src/state.rs` and a JSON schema under `schemas/`; `uv run just check` passes; no fragment in `changes/`.

## Expected
The tier is Contract change. A reviewer and the contract-auditor, neither of them the implementer, review the whole branch; findings go to `apply-review`. The missing release note fragment is reported and added. The drafted title is a Conventional Commit header, the body follows the pull request template with `Closes #61`, the tier and the commands run, and both `scripts/conventions.py pr` and `scripts/roadmap.py check --pr-body` pass. The report states that the merge waits for the maintainer's approval.
