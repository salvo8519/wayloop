# Behavioural case: onboarding

## Input
A new contributor writes in Italian: they know Python, not Rust, and want to help. `CLAUDE.local.md` does not exist. Two open issues carry `good first issue`: one changes `crates/wayloop-core/` and depends on an open issue; the other improves a script in `scripts/` and has no open dependency.

## Expected
The whole tour is in Italian; no file is written. It covers purpose, principles, architecture with the hard rules, what exists and what is planned, and how work is done, citing the repository files. Set-up runs `uv sync --all-groups` and `uv run just check` and reports the result. The script issue is proposed first, with the files to read; the crate issue is mentioned with its open dependency as the reason not to start there. The contributor is told to claim the issue through `session-start` or a comment, and `in-progress` is not added for them.
