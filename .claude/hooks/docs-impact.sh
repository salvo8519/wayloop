#!/usr/bin/env bash
# Stop hook: before Claude ends the session, changes must come with their documentation.
# Exit 2 sends the message back to Claude, which then updates the documentation.
set -uo pipefail
active=$(python3 -c 'import json,sys; print(json.load(sys.stdin).get("stop_hook_active", False))' 2>/dev/null || echo False)
[ "$active" = "True" ] && exit 0   # already continuing because of this hook: avoid a loop
cd "${CLAUDE_PROJECT_DIR:-.}" || exit 0
command -v git >/dev/null 2>&1 || { echo "docs-impact hook: git not found, check skipped" >&2; exit 0; }
python3 scripts/docs_sync.py impact --worktree --hook
