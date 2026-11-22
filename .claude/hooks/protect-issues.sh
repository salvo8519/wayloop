#!/usr/bin/env bash
# PreToolUse hook on Bash: refuse direct edits of issue titles and bodies (#312).
# The logic lives in protect_issues.py, next to this file; it reads the hook JSON on stdin.
# Without python3 the hook cannot read the command: it fails closed on anything that names gh.
if ! command -v python3 >/dev/null 2>&1; then
  if grep -q '\bgh\b' ; then
    echo "Blocked: python3 not found, so this gh command cannot be checked for issue edits. Use uv run python scripts/roadmap.py edit." >&2
    exit 2
  fi
  exit 0
fi
exec python3 "$(dirname "$0")/protect_issues.py"
