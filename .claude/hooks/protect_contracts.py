"""PreToolUse hook: block edits to contract snapshots that already exist on the base branch.

A contract snapshot is a versioned JSON Schema under `schemas/`, named `v<N>.json`,
`v<N>.output.json` (ADR 0009, C5) or `v<N>.input.json` (hand-over input, SPEC-0001 R1). It
is protected when its path, relative to the root of the git worktree that contains it, exists
on the base branch (`origin/main`, or `WAYLOOP_CONTRACT_BASE`). Snapshots added on the current
branch stay editable until they merge.
Outside a git repository, or when the base branch is unknown, every existing snapshot is
protected. CI is the second line of defence (`scripts/check_schemas.py`, whose pattern this hook
imports).

Exit status 2 blocks the edit; the message on stderr is shown to the agent. The hook fails
closed: when its shared scripts cannot be loaded or fail, an edit of a path that looks like a
snapshot is blocked, and every other edit goes through.
"""

from __future__ import annotations

import json
import os
import re
import sys
from pathlib import Path

# The hooks run with the system python3, outside the project environment: the shared helpers of
# the scripts (standard library only) are imported from scripts/ next to .claude/, appended to
# the path so that the standard library keeps precedence over a file of the same name.
sys.path.append(str(Path(__file__).resolve().parents[2] / "scripts"))

# Copy of check_schemas.SNAPSHOT, used only when the shared scripts cannot be imported, so that
# the hook still blocks a snapshot edit (fail closed). tests/process/test_check_schemas.py keeps
# the copy equal to its source.
FALLBACK_SNAPSHOT = re.compile(r"(^|/)schemas/(.+/)?v(?P<version>\d+)(\.output|\.input)?\.json$")


def _existing_dir(path: str) -> str:
    directory = os.path.dirname(path)
    while directory and not os.path.isdir(directory):
        parent = os.path.dirname(directory)
        if parent == directory:
            break
        directory = parent
    return directory or "."


def reason_to_block(file_path: str, base: str) -> str | None:
    """Why an edit to `file_path` must be blocked, or None to allow it. Raises when the shared
    scripts cannot be imported; main() then fails closed."""
    import _git
    from check_schemas import SNAPSHOT

    if not SNAPSHOT.search(file_path.replace("\\", "/")):
        return None
    path = os.path.abspath(file_path)
    top = _git.run("rev-parse", "--show-toplevel", cwd=_existing_dir(path))
    if top.returncode != 0:
        return "is an existing contract snapshot" if os.path.exists(path) else None
    root = top.stdout.strip()
    relative = os.path.relpath(os.path.realpath(path), os.path.realpath(root)).replace(os.sep, "/")
    try:
        _git.revision(base)
        usable = _git.run("rev-parse", "--verify", "--quiet", f"{base}^{{commit}}", cwd=root)
    except ValueError:
        usable = None
    if usable is None or usable.returncode != 0:
        if os.path.exists(path):
            return f"is an existing contract snapshot ({base} is not available to compare against)"
        return None
    if (
        _git.run("cat-file", "-e", "--end-of-options", f"{base}:{relative}", cwd=root).returncode
        == 0
    ):
        return f"exists on {base}"
    return None


def main() -> int:
    data = json.load(sys.stdin)
    file_path = (data.get("tool_input") or {}).get("file_path") or ""
    try:
        reason = reason_to_block(file_path, os.environ.get("WAYLOOP_CONTRACT_BASE", "origin/main"))
    except Exception as error:  # noqa: BLE001 (any failure of the shared scripts fails closed)
        if not FALLBACK_SNAPSHOT.search(file_path.replace("\\", "/")):
            return 0
        sys.stderr.write(
            f"Blocked: {file_path} looks like a contract snapshot and the protection could not "
            f"load ({type(error).__name__}: {error}). Fix scripts/_git.py or "
            "scripts/check_schemas.py first.\n"
        )
        return 2
    if reason is None:
        return 0
    sys.stderr.write(
        f"Blocked: {file_path} {reason}. A released contract snapshot never changes: write the "
        "next version (v<N+1>) with a new snapshot and record it in schemas/index.json "
        "(ADR 0009, C5). A breaking change also needs the label breaking-change.\n"
    )
    return 2


if __name__ == "__main__":
    sys.exit(main())
