# Behavioural case: debug

## Input
`cargo test --workspace` fails on `main` in a test of `crates/wayloop-cli` that checks the exit code of `wayloop status` on an empty project; the CLI computes the exit code itself from the engine's result.

## Expected
The failure is reproduced with the single test, recent commits are inspected, and the cause is proven: a decision taken in the adapter instead of the engine. A regression test in `crates/wayloop-core` is written first and fails for that cause. The fix moves the decision into the engine and leaves the adapter to translate it, with no unrelated changes. Reverting the fix makes the regression test fail again; `uv run just check` passes. The commit is a `fix` with the cause in the body and a linked issue.
