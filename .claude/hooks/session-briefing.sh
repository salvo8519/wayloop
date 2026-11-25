#!/usr/bin/env bash
# SessionStart hook: put the state of the work in front of the agent before the first request.
# Offline and fast: the branch, its issue, the latest hand-over of the branch and its next step.
# The full briefing, with GitHub, is the session-start skill.
cd "${CLAUDE_PROJECT_DIR:-.}" || exit 0
command -v git >/dev/null 2>&1 || { echo "session-briefing hook: git not found" >&2; exit 0; }
branch=$(git rev-parse --abbrev-ref HEAD 2>/dev/null) || exit 0
slug=${branch//\//-}
echo "Wayloop session. Branch: $branch."
# Copy of branch_issue in scripts/conventions.py (the hook runs without the project environment);
# tests/process/test_claude_hooks.py keeps the two equal. The sed program reads the form; the
# shell test after it the length of the branch and the range of an issue number.
issue=$(echo "$branch" | LC_ALL=C sed -nE 's#^(feat|fix|perf|refactor|docs|test|build|ci|chore|revert|sprint/S[0-9]{2,})/([1-9][0-9]*)-[a-z0-9]+(-[a-z0-9]+){0,4}$#\2#p')
if [ -n "$issue" ] && { [ "${#branch}" -gt 100 ] || [ "${#issue}" -gt 16 ] \
  || [ "$issue" -gt 9007199254740991 ]; }; then
  issue=""
fi
[ -n "$issue" ] && echo "Issue: #$issue."
entry=$(ls -1 .agents/journal/*-"$slug".md 2>/dev/null | sort | tail -1)
[ -z "$entry" ] && entry=$(ls -1 .agents/journal/2*.md 2>/dev/null | sort | tail -1)
if [ -n "$entry" ]; then
  echo "Latest hand-over: $entry"
  awk '/^## Next steps/{flag=1; next} /^## /{flag=0} flag && NF' "$entry" | head -3
fi
echo "Run the session-start skill for the full briefing; check the issue out before working on it."
exit 0
