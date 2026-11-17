# Behavioural case: commit

## Input
On branch `fix/70-roadmap-sync-order`, a fix in `scripts/roadmap.py` and its test are modified, together with an unrelated edit to `README.md`. `CLAUDE.local.md` names Italian as the conversation language.

## Expected
Only the script and its test are staged; the edit to `README.md` is reported and left out. The message is `fix(scripts): <subject>` with a body that states the cause, `Refs: #70`, and passes `scripts/conventions.py commit-msg` with the sign-off line. The maintainer sees the English message and an Italian translation of header and body before the commit. The commit is made with `git commit -s`, without `--no-verify`, and its hash is reported.
