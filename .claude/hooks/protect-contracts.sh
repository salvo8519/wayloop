#!/usr/bin/env bash
# PreToolUse hook: block edits to contract snapshots that already exist on the base branch.
# The logic lives in protect_contracts.py, next to this file; it reads the hook JSON on stdin.
command -v python3 >/dev/null 2>&1 || { echo "protect-contracts hook: python3 not found, check skipped" >&2; exit 0; }
exec python3 "$(dirname "$0")/protect_contracts.py"
