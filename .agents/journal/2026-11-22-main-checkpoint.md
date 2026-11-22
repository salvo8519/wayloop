# 2026-11-22 main

- Issue: none (checkpoint of the main session)
- Pull request: this entry; work merged in #300 to #322
- Spec: SPEC-0001 T1 to T6; SPEC-0002 and SPEC-0003 approved

## Checkout
- Holder: Claude Code (Anthropic), main session of Wayloop (named "Wayloop" by the maintainer), 2026-11-20 to 2026-11-22
- Branch: main, no worktree left
- Step reached: merge (every pull request of the session merged)
- Released: yes, with this hand-over

## Done
- Contracts generated from Rust types, one registry, snapshots with a history per contract, released versions immutable (ADR 0030; #292, #301, #304, #305, #311)
- One source for every rule and value: shared script modules, jscpd gate with an empty baseline, closed-by-default I/O bans in the pure crates (#300, #302, #306)
- `wayloop validate` on briefs through generated contracts (#288, T5) and on repository files with bounded memory, files and links (#319, T6)
- Roadmap issues changed only through `roadmap.py` (`new`, `edit`), a hook refuses direct edits, no merge while main is red (#313); unknown epics refused (#322); pull requests waiting for approval use `Refs` (#315)
- SPEC-0002 state machine and SPEC-0003 ledger approved after three coherence checks (#235, #236)
- Planned: #307 typed map of lines, #316 deletion of closed ledger segments (M6), #320 report of a project without artifact locations

## Not done
- Nothing half done. Not verified: Windows paths for T6; the residual race between path resolution and open (documented in the runtime).

## Decisions taken
- 2026-11-20, maintainer: contracts generated from Rust types (ADR 0030); output schemas open, input closed; one history manifest per contract; rule (B) for changes; a version with `introduced_in: unreleased` not held by a release tag may change incompatibly until the first release.
- 2026-11-20, maintainer: one source for every rule and value in both projects (AGENTS.md hard rule).
- 2026-11-22, maintainer: SPEC-0002 Q10 to Q13 and SPEC-0003 Q9 to Q14 (recorded in the specifications and in the bodies of #235 and #236); a review covered by the verification reserve is exempt from an exhausted token budget.
- 2026-11-22, maintainer: this is the main session of Wayloop; other sessions take only one isolated issue in their own worktree, with a checkout lease.

## Learnings
- A limit against hostile input counts the resource it protects (bytes allocated, files opened, the resolved path), not the data easiest to count.
- A rule written in two places diverges: most review findings of the session were of this kind; the simplifier lens and `just dup` are both needed.
- Subagents may lose the shell in a session: they then edit files only and the orchestrator runs the gate, commits and pushes.

## Open questions for the maintainer
- none

## Next steps
1. #307: type the map of lines in the contract of validate (extend the interpreter subset with a schema in `additionalProperties` and `propertyNames`), then remove the rules of the core for `lines`.
2. #320: diagnostic when no artifact location exists.
3. SPEC-0001 T7 onwards (#258 to #270), in the order of `specs/0001-artifact-model/tasks.md`.
4. Then M1: #48 transition table and guards (SPEC-0002), #50 ledger writer (SPEC-0003).
