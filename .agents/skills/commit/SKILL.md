---
name: commit
description: Prepares a commit of related changes with a Conventional Commit message in English, checks it with scripts/conventions.py, shows it to the maintainer with a translation in their language, and commits it signed off. Use when changes are ready to be committed.
---

Talk to the maintainer in their preferred language (`CLAUDE.local.md`); everything written to the repository or to GitHub is in English, without icons. The history is in English because it generates `CHANGELOG.md` and is read by every contributor; the maintainer reviews it in their own language.

## Produces
One signed-off commit of one concern, with a message that passes `scripts/conventions.py commit-msg`, approved by the maintainer when they asked to review commits.

## Steps
1. **Inspect.** Run `git status`, `git diff` and `git diff --staged`. Stage only the files of
   one concern (one task of a specification, or one fix), by path: `git add <path>`. Never
   stage secrets, local files such as `CLAUDE.local.md`, or generated output that the
   repository does not track.
2. **Pre-commit facts.** Read the issue number from the branch name
   (`<type>/<issue>-<slug>` or `sprint/SNN/<issue>-<slug>`). If staged files are under
   `docs/en/`, check the Italian page or plan the label `translation-pending`; if they are
   under `crates/*/src/` or `schemas/`, check for a fragment in `changes/`.
3. **Write the message** to a temporary file, following `docs/en/project/conventions.md`,
   Commits:
   - header `<type>(<scope>)[!]: <subject>`, type and scope from `scripts/conventions.py`,
     imperative subject with a lower-case first word and no full stop, at most 72 characters,
     with `[SPEC-NNNN Rk]` at the end when the commit implements a requirement;
   - body: what changed and why, wrapped at 72 characters; for a breaking change, `!` in the
     header and a `BREAKING CHANGE:` paragraph;
   - trailers: `Refs: #<issue>`; `Docs-impact: none` only when the change genuinely needs no
     documentation; any attribution trailers the session requires, last.
4. **Check.** Append the sign-off line that `git commit -s` will add
   (`Signed-off-by: <git config user.name> <git config user.email>`) to a copy of the file and
   run `uv run python scripts/conventions.py commit-msg <file>`. Fix and re-run until it
   passes. Run `uv run python scripts/check_style.py` if the message is unusual.
5. **Show the maintainer**, in their language: the English message exactly as it will be
   committed; a faithful translation of header and body; the list of staged files. When
   `CLAUDE.local.md` does not exist or names English, show the message only.
6. **Commit** after approval, or directly when the maintainer has said that commits in this
   session need no review: `git commit -s -F <file>`. The commit-msg hook runs the same check.
   The pre-commit hook formats and applies safe fixes: when it stops the commit once with
   files modified, review and stage them, then commit again. Never use `--no-verify`.
7. **Confirm.** Run `git log -1 --format=%B` and `git status`: the commit exists with the
   sign-off, and nothing intended for it was left unstaged.

## Stop and ask
- The staged change mixes concerns, or contains files of another issue: propose how to split it.
- No type or scope fits: a new scope is added in `scripts/conventions.py` with the conventions page, in its own change.
- A hook fails for a reason outside the message: report it, never bypass it.
- The maintainer changes the wording: apply it in English and show it again.

## Report
- The commit hash and header; the files committed.
- What was left unstaged and why; checks that were not run and the risk.
