---
name: session-end
description: Closes a working session on Wayloop so that any agent can resume it. Use when the session is over, before switching issue, or before the context runs out.
---

Talk to the maintainer in their preferred language (`CLAUDE.local.md`); everything written to the repository or to GitHub is in English, without icons.

## Produces
A gate run, a journal hand-over, a commit and a pull request, a hand-over comment on the issue, and a merge when AGENTS.md allows it.

## Steps
1. **Gate.** Run `uv run just check`. If it fails, fix what belongs to this session's issue;
   otherwise record the failure under Not done and open the pull request as a draft.
2. **Scope.** `git status` and `git diff origin/main...HEAD --stat`. Every changed file belongs
   to the branch's issue; unrelated changes go to a separate branch.
3. **Roadmap.** Issues opened in this session were created with
   `uv run python scripts/roadmap.py new`. If the pull request completes the issue, mark its
   task `done` in `roadmap/roadmap.yaml`; `roadmap.py check --pr-body` must pass.
4. **Release note.** A change to `crates/*/src/` or `schemas/` adds a fragment in `changes/`
   (`<issue>.<type>.md`); otherwise the pull request gets `no-release-note`.
5. **Journal hand-over.** Write `.agents/journal/<YYYY-MM-DD>-<branch-slug>.md` from
   `_template.md`: Checkout (holder, branch, worktree, step reached, released or lease), Done
   with hashes, Not done (half done and where, failing checks, what was skipped or not
   verified and its risk), decisions, learnings that are reusable for the directory, open
   questions, and the very next step. Facts only, no client data, no secrets. Another agent of
   another vendor must be able to continue from this file alone.
6. **Commit.** Conventional Commit, English, signed off (`git commit -s`); the commit-msg hook
   checks it.
7. **Push.** `git push -u origin <branch>`. Never force-push a branch someone else reviewed
   without telling the maintainer.
8. **Pull request.** Create it, or update its body: title as the Conventional Commit summary
   of the whole change; body with Summary, `Closes #<n>` (or `Refs #<n>`), the requirements
   covered, changes by area, documentation and translation status, tests run, risks and
   follow-ups, tier and labels. Draft if the gate failed or tasks remain.
9. **Hand-over comment.** Post on the issue the journal entry's Checkout, Not done and Next
   steps under the heading `Hand-over`, with a link to the journal file and the pull request.
   Keep `in-progress` while the work is unfinished; remove it when the pull request is ready.
10. **Merge.** Follow AGENTS.md, Merging: when the pull request is small and its gate holds,
    merge it with its title and delete the branch; when it changes `crates/`, `schemas/` or an
    SDK, ask the maintainer to confirm first. Merge stacked pull requests bottom up, each
    rebased onto `main`. Never merge past a failing required check, a review finding or an
    open question.
11. **Report** to the maintainer: pull request and merge status, checks, what remains, next
    step.

## Stop and ask
- A pull request that changes `crates/`, `schemas/` or an SDK: ask the maintainer to confirm before merging.
- A failing required check, an open review finding or an open question: do not merge.
- Never force-push a branch someone else reviewed without telling the maintainer.

## Report
- Pull request and merge status, checks.
- What remains not done and not verified, and the risks.
- The very next step.
