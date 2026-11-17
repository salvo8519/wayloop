---
name: debug
description: Finds the cause of a failure by reproducing it, isolating it and proving the mechanism, then fixes it at the narrowest responsible layer with a regression test written first. Use when a test, a check or CI fails, or behaviour differs from the specification and the cause is not proven.
---

Talk to the maintainer in their preferred language (`CLAUDE.local.md`); everything written to the repository or to GitHub is in English, without icons.

## Produces
A reproduction, a cause stated with its evidence, a regression test that fails before the fix and passes after it, and the smallest fix in the layer that owns the wrong behaviour. When the task is only to investigate, the cause and its proof, with no fix.

## Steps
1. **Record the symptom** exactly: the command, the full error output, the commit
   (`git rev-parse HEAD`), and whether it fails every time. Keep observation apart from
   inference.
2. **Reproduce** with the narrowest command: `cargo test -p <crate> <name>`,
   `uv run python -m pytest <path>::<test> -q`, or the failing step of `uv run just check`.
   For CI, read the log with `gh run view <id> --log-failed` and reproduce locally. If it does
   not reproduce, gather more evidence (environment, versions, order of tests); do not guess.
3. **Recent changes.** Run `git log --oneline -15` and `git diff origin/main...HEAD --stat`.
   When a known good commit exists, `git bisect run <command>` finds the commit that
   introduced the failure.
4. **Isolate.** Trace the wrong value back from where it is observed to where it is produced:
   adapter (`crates/wayloop-cli/`, scripts) or engine (`crates/wayloop-core/`). At each
   boundary, check what enters and what leaves. Rank hypotheses by evidence and by how cheaply
   each can be refuted; test one at a time with one change.
5. **Prove the cause.** One mechanism must explain every observation. Make no edit to product
   code before that. After three failed hypotheses, stop: the model of the system is wrong
   (see Stop and ask).
6. **Regression test first.** Write the test that encodes the expected behaviour at the layer
   that owns it, run it and confirm it fails for the proven cause. Delegate it to the
   test-engineer role when it is available.
7. **Fix at the narrowest layer.** Change only the code that owns the wrong behaviour.
   Decision logic is fixed in `crates/wayloop-core`, never patched around in an adapter. No
   renaming, clean-up or new abstraction outside the fix; preserve unrelated behaviour.
8. **Prove the fix** with the `verify` skill: the regression test passes; reverting only the
   fix makes it fail again; then `uv run just check`.
9. **Commit** with the `commit` skill, type `fix`, the cause in the body, `Refs: #<issue>`. A
   bug without an issue gets one first, through `uv run python scripts/roadmap.py new`.

## Stop and ask
- The cause lies in a contract, a specification or an ADR: the fix is a decision for the maintainer.
- Three hypotheses failed, or the fix would touch more than one layer.
- The failure is outside the repository (a vendor, GitHub, a dependency): report the evidence and propose a workaround, do not patch around it silently.
- The task was to investigate only: report the cause and wait.

## Report
- Symptom, reproduction command, cause and the evidence that proves it.
- Regression test and fix, with the commands run and their result.
- What was not verified (for example another platform, an intermittent failure seen once) and the risk.
