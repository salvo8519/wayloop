#!/usr/bin/env bash
# PostToolUse hook: format the file Claude just wrote. Never blocks: a missing formatter is
# reported on stderr and the edit stands.
set -uo pipefail
file=$(python3 -c 'import json,sys; print(json.load(sys.stdin).get("tool_input",{}).get("file_path",""))' 2>/dev/null || true)
{ [ -z "$file" ] || [ ! -f "$file" ]; } && exit 0
case "$file" in
  *.rs)
    if command -v rustfmt >/dev/null 2>&1; then rustfmt --edition 2024 "$file" 2>/dev/null || true
    else echo "format hook: rustfmt not found, $file left as written" >&2; fi ;;
  *.py)
    if command -v uv >/dev/null 2>&1; then
      (uv run --quiet ruff format "$file" && uv run --quiet ruff check --fix --quiet "$file") >/dev/null 2>&1 || true
    else echo "format hook: uv not found, $file left as written" >&2; fi ;;
esac
exit 0
