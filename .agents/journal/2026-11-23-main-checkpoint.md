# 2026-11-23 main

- Issue: none (checkpoint of the main session)
- Pull request: this entry; work merged in #327 to #331
- Spec: SPEC-0001 T1 to T6 done; #307 and #320 done; T7 (#258) next

## Checkout
- Holder: Claude Code (Anthropic), main session of Wayloop, 2026-11-23
- Branch: main, no worktree left
- Step reached: merge (every pull request of the session merged)
- Released: yes, with this hand-over

## Done
- Rust 1.100 (#327). It was merged before its checks had all started, because the merge helper of the session took the checks already reported for the full set; main went red on the Clippy lint `assert_is_empty`. Fixed in #329; the helpers now wait for every check of `.github/required-checks.txt`, a stable count, and an explicit all-green verdict.
- #307 typed map of `lines` (#330): `propertyNames` and `additionalProperties` as a schema in the interpreter subset; `Pointer` carries the named format `json-pointer`; typed maps accepted in both directions, a map of any value refused in an input, a map flattened beside members refused in an output; the values of a map count in the four levels of objects.
- #320 artifact locations (#331): input member `locations` (enum `ArtifactFolder`, snake_case, path in `ArtifactFolder::path`), check `artifact_locations`, warning `artifact_location_missing` (SPEC-0001 R26, AC37); the generator refuses any text enum value that is not snake_case; `schema_snapshots.py --diff-against` lists additive changes by file.

## Not done
- The ruleset "Protect main" still requires no status check; the session cannot change it (proxy 403). Until the maintainer adds the seven checks of `.github/required-checks.txt`, only the helpers of the session prevent a merge before CI ends.

## Decisions taken
- 2026-11-23, maintainer: the format `json-pointer` is on the type `Pointer`, so every pointer of the contracts carries it.
- 2026-11-23, maintainer: a project without artifact locations gives a warning, through a list of the folders found (`locations`); values in snake_case with a generator rule; a requirement in SPEC-0001 (R26); `locations` is documented, not compared with file origins; additive changes listed by `just schemas`.
- 2026-11-23, maintainer: no new features until the planned work is closed.
- 2026-11-23, maintainer: errors are surfaced and recorded, never passed over; checks are always run, evidence is never taken for granted.

## Learnings
- Contract changes that look mechanical touch the general rules of the generator; three independent reviewers (reviewer, simplifier, contract-auditor) each found defects the others did not.
- The orchestrator must not edit a worktree while a reviewer reads it.
- A commit trailer counts only in the last paragraph of the message: no blank line between `Docs-impact: none` and the attribution lines.
- Locally the pinned toolchain 1.100 cannot be downloaded: run with `RUSTUP_TOOLCHAIN=1.97.0`, and search for the new lints of the pinned version by hand.

## Open questions for the maintainer
- none

## Next steps
1. SPEC-0001 T7 (#258): milestones, epics, stories, tasks and gate issues.
2. Then T8 to T10 in parallel where their paths are disjoint, T11 to T19, T21, T22, as in `specs/0001-artifact-model/tasks.md`.
3. Maintainer: add the required status checks to the ruleset "Protect main".
