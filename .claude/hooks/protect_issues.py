"""PreToolUse hook on Bash: refuse direct edits of the title or body of an issue (#312).

Issues of the roadmap are changed only with `scripts/roadmap.py edit`, which keeps the line
`Part of #<epic>.` and the roadmap file in step; on 2026-11-20 a direct `gh api` PATCH dropped
that line from three issues and Roadmap sync failed on main for hours. The rules live in
`scripts/_issue_edits.py`; this file only reads the hook input and reports.

Exit status 2 blocks the command; the message on stderr is shown to the agent. The hook fails
closed: when the shared module cannot be loaded or fails, a command that looks like an issue
edit is blocked, and every other command goes through.
"""

from __future__ import annotations

import json
import re
import sys
from pathlib import Path

# The hooks run with the system python3, outside the project environment: the shared helpers of
# the scripts (standard library only) are imported from scripts/ next to .claude/, appended to
# the path so that the standard library keeps precedence over a file of the same name.
sys.path.append(str(Path(__file__).resolve().parents[2] / "scripts"))

# Copy of _issue_edits.LOOKS_LIKE_EDIT, used only when the shared module cannot be imported, so
# that the hook still blocks what may be an issue edit (fail closed).
# tests/process/test_issue_edits.py keeps the copy equal to its source.
FALLBACK_LOOKS_LIKE_EDIT = re.compile(
    r"\bgh\b[\s\S]*\b(issue\s+edit|api\b[\s\S]*(issues/\d|graphql))"
)


def main() -> int:
    data = json.load(sys.stdin)
    command = str((data.get("tool_input") or {}).get("command") or "")
    try:
        import _issue_edits

        reason = _issue_edits.reason_to_block(command)
    except Exception as error:  # noqa: BLE001 (any failure of the shared module fails closed)
        if not FALLBACK_LOOKS_LIKE_EDIT.search(command):
            return 0
        sys.stderr.write(
            "Blocked: this command looks like a direct edit of an issue and the check could not "
            f"run ({type(error).__name__}: {error}). Use `uv run python scripts/roadmap.py edit`, "
            "or fix scripts/_issue_edits.py first.\n"
        )
        return 2
    if reason is None:
        return 0
    sys.stderr.write(_issue_edits.message(reason))
    return 2


if __name__ == "__main__":
    sys.exit(main())
