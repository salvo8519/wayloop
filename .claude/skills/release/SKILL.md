---
name: release
description: Prepares a Wayloop release - readiness of the milestone, version in the workspace manifest, release notes with towncrier, changelog with git-cliff and the release pull request - and leaves the tag to the maintainer. Use when the maintainer asks to cut a release, or to check whether main is ready for one.
---

<!-- Generated from .agents/skills/release/SKILL.md by scripts/sync_agents.py. Edit the source, not this file. -->

Talk to the maintainer in their preferred language (`CLAUDE.local.md`); everything written to the repository or to GitHub is in English, without icons.

Policy: ADR 0019 (`docs/en/project/adr/0019-distribution-and-versioning.md`) and ADR 0023
(`docs/en/project/adr/0023-stability-policy.md`). The release pipeline
(`specs/0018-release-pipeline/spec.md`) does not exist yet: there is no release script and no
release workflow, so every readiness item below is checked by hand with the command named, and
nothing is published by this skill.

## Produces
A readiness checklist with pass, fail or not verified for each item; on a pass, a branch
`release/v<X.Y.Z>` with the version in `Cargo.toml`, `RELEASE_NOTES.md` built from the
fragments in `changes/`, `CHANGELOG.md` regenerated, one commit `chore(release): ...`, and a
release pull request waiting for the maintainer. The maintainer approves, merges and creates
the tag `v<X.Y.Z>`.

## Steps
1. **Version and milestone.** The release of each milestone is its `release` field in
   `roadmap/roadmap.yaml`. Last tag: `git describe --tags --abbrev=0` (none before the first
   release: the range starts at the root commit). Confirm the version with the maintainer
   when the field and the request disagree.
2. **Milestone done.** `uv run python scripts/roadmap.py sync`, then
   `uv run python scripts/roadmap.py list --milestone M<n> --open`: no task left open, or each
   one moved or explained by the maintainer. `uv run python scripts/roadmap.py check` and
   `uv run python scripts/roadmap.py links` pass.
3. **Main green.** `git fetch origin` and
   `gh run list --branch main --commit $(git rev-parse origin/main)`: every workflow that
   produces a check of `.github/required-checks.txt` succeeded on the head of `main`.
   `gh pr list --state open` shows no pull request of the milestone left unexplained.
4. **Security.** No open security issue:
   `gh issue list --state open --label area:security`,
   `gh api repos/salvo8519/wayloop/dependabot/alerts --jq '[.[] | select(.state=="open")] | length'`
   and `gh api repos/salvo8519/wayloop/security-advisories --jq '[.[] | select(.state!="published" and .state!="closed")] | length'`.
   A call refused for lack of permission is reported as not verified, never as clean. The
   `Dependencies` job of step 3 covers licences, bans and advisories.
5. **Release notes complete.** Every user-visible change since the last tag has a fragment:
   compare `git log <last-tag>..origin/main --oneline -- 'crates/*/src' schemas` with
   `ls changes/`. Preview with `uv run towncrier build --draft --version <X.Y.Z>`.
6. **Documentation.** `uv run python scripts/i18n.py check` and `uv run just docs`; every
   specification shipped in the milestone has its acceptance criteria verified (its tasks
   closed by pull requests that cite them).
7. **Readiness verdict.** Show the checklist. Any fail stops here: name the fix, and hand a
   red run to the `fix-ci` skill.
8. **Branch and version.** `git switch -c release/v<X.Y.Z> origin/main` (the `release/v*`
   prefix is exempt from the branch rule). Set `version` under `[workspace.package]` in
   `Cargo.toml`; run `cargo check --workspace` so that `Cargo.lock` follows. Before 1.0, a
   breaking change of a contract or a command needs at least a minor bump.
9. **Release notes.** `uv run towncrier build --version <X.Y.Z> --yes`: it writes
   `RELEASE_NOTES.md` and removes the fragments. Breaking changes ("Action required") come
   first; every line reads well for a project lead and announces only work in this release.
   Review the text with the maintainer: it is what users read.
10. **Changelog.** `uv run git-cliff --tag v<X.Y.Z> -o CHANGELOG.md`. Never edit a published
    entry by hand.
11. **Journal.** Entries older than two releases may be pruned (`.agents/journal/README.md`);
    propose the list, remove only what the maintainer confirms.
12. **Gate and pull request.** `uv run just check`; `git commit -s` with
    `chore(release): prepare <X.Y.Z>`; `git push -u origin release/v<X.Y.Z>`; open the pull
    request with the checklist, the version and its reason, and the linked release issue.
    Wait for the maintainer: a release pull request is never merged by the agent.
13. **After the tag**, once the maintainer has merged and tagged: confirm that the tag points
    at the merge commit (`git ls-remote --tags origin v<X.Y.Z>`) and that the documentation
    site shows the new release notes. Publication to registries is not verified until the
    release workflow exists.

## Stop and ask
- Any readiness item that fails or cannot be verified.
- The version, when the roadmap and the request disagree, or when a breaking change is in the
  range.
- The text of the release notes, before the commit.
- Merge of the release pull request and the tag: the maintainer only. Never create, move or
  delete a tag, and never publish.
- A release pull request with no release issue to link: ask which issue it closes.

## Report
- Done: the checklist with pass, fail or not verified per item; the version and its reason;
  the release pull request.
- Not done or not verified: items checked by hand only, security calls refused, publication
  and provenance (no release workflow yet).
- Risks: fragments missing or weak, open tasks moved out of the milestone, breaking changes in
  the range.
