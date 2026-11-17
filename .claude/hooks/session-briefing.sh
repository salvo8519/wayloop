#!/usr/bin/env bash
# SessionStart hook: put the state of the work in front of the agent before the first request.
# Offline and fast: the branch, its issue, the latest hand-over of the branch and its next step.
# The full briefing, with GitHub, is the session-start skill.
cd "${CLAUDE_PROJECT_DIR:-.}" || exit 0
command -v git >/dev/null 2>&1 || { echo "session-briefing hook: git not found" >&2; exit 0; }
branch=$(git rev-parse --abbrev-ref HEAD 2>/dev/null) || exit 0
slug=${branch//\//-}
echo "Wayloop session. Branch: $branch."
issue=$(echo "$branch" | sed -n 's|^[a-z]*/\([0-9][0-9]*\)-.*|\1|p; s|^sprint/S[0-9]*/\([0-9][0-9]*\)-.*|\1|p')
[ -n "$issue" ] && echo "Issue: #$issue."
entry=$(ls -1 .agents/journal/*-"$slug".md 2>/dev/null | sort | tail -1)
[ -z "$entry" ] && entry=$(ls -1 .agents/journal/2*.md 2>/dev/null | sort | tail -1)
if [ -n "$entry" ]; then
  echo "Latest hand-over: $entry"
  awk '/^## Next steps/{flag=1; next} /^## /{flag=0} flag && NF' "$entry" | head -3
fi
echo "Run the session-start skill for the full briefing; check the issue out before working on it."
exit 0
