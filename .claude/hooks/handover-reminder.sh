#!/usr/bin/env bash
# Stop hook: a session that committed on a branch today leaves a hand-over before it ends.
# Exit 2 sends the reminder back to the agent once; it does not run again while the agent is
# already continuing because of a stop hook.
active=$(python3 -c 'import json,sys; print(json.load(sys.stdin).get("stop_hook_active", False))' 2>/dev/null || echo False)
[ "$active" = "True" ] && exit 0
cd "${CLAUDE_PROJECT_DIR:-.}" || exit 0
command -v git >/dev/null 2>&1 || exit 0
branch=$(git rev-parse --abbrev-ref HEAD 2>/dev/null) || exit 0
case "$branch" in main|HEAD) exit 0 ;; esac
base=${WAYLOOP_CONTRACT_BASE:-origin/main}
git rev-parse --verify --quiet "$base" >/dev/null || base=main
today=$(date +%Y-%m-%d)
commits=$(git log "$base..HEAD" --since="$today 00:00" --format=%h 2>/dev/null | wc -l)
[ "$commits" -eq 0 ] && exit 0
slug=${branch//\//-}
[ -f ".agents/journal/$today-$slug.md" ] && exit 0
echo "This session committed on $branch today without a hand-over: write .agents/journal/$today-$slug.md from .agents/journal/_template.md (session-end skill) so that any agent can resume the work." >&2
exit 2
