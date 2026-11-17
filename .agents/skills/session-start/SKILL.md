---
name: session-start
description: Open a working session on Wayloop with full context (current branch and issue, the latest hand-over, open pull requests and who merges them, CI, sprint, roadmap drift, stale checkouts) and propose a plan; on the maintainer's choice, check the issue out so that no other agent takes it. Use at the beginning of every session, or when the maintainer asks where the project stands.
---

Talk to the maintainer in their preferred language (`CLAUDE.local.md`). Until the maintainer
chooses what to work on, change no file and nothing on GitHub: this part only reads and
proposes.

1. **State, deterministically.** Run `uv run python scripts/project_state.py` (`--json` to
   process it). It prints the branch and its issue, uncommitted work, the latest journal
   entries, open pull requests with checks, reviews and who merges them, and the sprint. If the
   GitHub CLI is not authenticated, say so and continue with git only. Then run
   `uv run python scripts/roadmap.py sync` and `uv run python scripts/roadmap.py links`: report
   any difference with the command that fixes it.
2. **Hand-over.** Read the latest entry of `.agents/journal/` for the current branch, otherwise
   the latest overall. Its Checkout, Not done and Next steps are the default continuation,
   whatever agent wrote it. For the issue of the branch, and for every issue you will propose,
   run `uv run python scripts/journal.py search <issue>` and cite the entries by file name.
   Read the issue, its specification (`specs/NNNN-*/spec.md`) and the ADRs it references.
3. **Hygiene.** Report each problem with the command that fixes it:
   - uncommitted changes, or a branch behind `main` (`git rebase origin/main`);
   - work committed directly on `main`;
   - an open pull request of this branch with failing checks;
   - issues labelled `in-progress` whose last checkout comment is older than its lease and
     whose branch has no commit since then: stale checkouts, proposed for release.
4. **Plan**, short:
   - **Continue**: the next step of the hand-over, if a branch is in progress.
   - **Merge queue**: open pull requests whose gate holds. Those marked "agent" in the
     briefing are merged in this session (AGENTS.md, Merging); those marked "maintainer
     confirmation" are listed with a one-line summary for the maintainer to confirm.
   - **Start next**: up to three ready issues of the active sprint or the current milestone,
     ordered by dependency. Mark which can run in parallel: different `area:` labels,
     disjoint files, no dependency between them. Say whether each needs an approved
     specification first.
5. **Wait for the maintainer's choice.** Then check the issue out:
   - create the branch `<type>/<issue>-<slug>` from an up-to-date `main`, or
     `sprint/SNN/<issue>-<slug>` for an issue of the active sprint; for parallel work, one git
     worktree per issue (`git worktree add ../wayloop-<issue> -b <branch> origin/main`);
   - label the issue `in-progress`;
   - post the checkout comment on the issue, in this exact format, so that any agent can see
     who holds the work:

     ```text
     Checkout
     - Holder: <agent and vendor>, session <id or date-time>
     - Branch: <branch>, worktree <path or none>
     - Lease: until <YYYY-MM-DDTHH:MMZ, 24 hours from now>
     - Starting from: <journal entry, or "fresh start">
     ```

   An issue already `in-progress` with a valid lease belongs to its holder: do not take it.
   Ask the maintainer to release it, or wait for the lease to expire.
