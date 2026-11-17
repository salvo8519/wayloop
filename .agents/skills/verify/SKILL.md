---
name: verify
description: Proves a claim about the work by running the commands that establish it before the claim is stated, and reports exactly what was not verified. Use when about to say that something is done, fixed, passing or ready, before a commit, a hand-over or a pull request, and when an agent reports success.
---

Talk to the maintainer in their preferred language (`CLAUDE.local.md`); everything written to the repository or to GitHub is in English, without icons.

## Produces
For each claim: the command that proves it, its exit status and the relevant lines of output, run against the current state of the working tree, and a verdict of pass, fail, not run or blocked. No file is changed.

## Steps
1. **List the claims** you are about to make, one per line: "the tests pass", "the bug is
   fixed", "requirement R3 is implemented", "the lane agent finished".
2. **Map each claim to its proof**, the narrowest command first and the wider gate after:

   | Claim | Proof |
   | --- | --- |
   | Rust code builds and its tests pass | `cargo test -p <crate> <filter>`, then `cargo test --workspace` |
   | Process scripts work | `uv run python -m pytest tests/process -q` |
   | Lints and format | `cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --all --check` |
   | The branch is ready | `uv run just check` |
   | A bug is fixed | the reproduction from the `debug` skill passes, and its regression test fails without the fix |
   | A requirement is met | a test named for it passes; read the requirement again, line by line |
   | Documentation in sync | `uv run python scripts/docs_sync.py impact --base origin/main` |
   | Skills and roles follow the standard | `uv run python scripts/check_agents.py` |
   | A sprint lane is within its paths | `uv run python scripts/sprint.py check-diff --branch <branch> --base origin/main` |
   | An agent did the work | `git log --oneline origin/main..HEAD` and `git diff origin/main...HEAD --stat` in its worktree |
   | CI is green | `gh pr checks <n>`, not a local run |

3. **Run fresh.** Run each command now, in the worktree that holds the change, after the last
   edit. A result from before an edit, from another branch or from an agent's report is not
   evidence. Reuse a result only when `git status` and `git rev-parse HEAD` match the state it
   was run on.
4. **Read the output.** Check the exit status and count failures, skipped and ignored tests.
   A command that ran zero tests, or a filter that matched nothing, proves nothing.
5. **Verdict per claim.** Pass: state the claim with the command and the result. Fail: state
   the actual status with the failing lines. Not run or blocked (a missing tool, no network,
   no GitHub authentication): say so; never replace it with "should pass".
6. **Stop.** Once every claim has a verdict, stop. Do not edit code, add tests or widen the
   scope; fixes belong to the skill that called this one.

## Stop and ask
- A proof needs a tool, a secret or a service that is not available: report the gap and ask whether to proceed without it.
- The output contradicts what was reported earlier in the session: correct the earlier statement before anything else.

## Report
- Each claim with its command, exit status and verdict.
- What was not verified and why, and the risk it leaves; never words such as "should", "probably" or "seems" in place of a result.
