"""Released schema snapshots never change (ADR 0009, C5; SPEC-0001 R1, R16, AC3).

A snapshot is a versioned JSON Schema under `schemas/` named `v<N>.json`, `v<N>.output.json`
or `v<N>.input.json`. A snapshot that exists on the base branch (at its merge base with HEAD)
is released: it stays in the working tree as a regular file with the same mode and content. A
schema change ships as a new snapshot v<N+1> beside the unchanged v<N>, and within one folder
the versions run from 1 without a gap or a leading zero. The hook
`.claude/hooks/protect_contracts.py` blocks the same edits before they happen.

    python scripts/check_schemas.py                    # against origin/main
    python scripts/check_schemas.py --base origin/x    # against another base
"""

from __future__ import annotations

import argparse
import re
import stat
import sys
from collections import defaultdict
from pathlib import Path

import _git

ROOT = Path(__file__).resolve().parents[1]

# The hook .claude/hooks/protect_contracts.py imports this pattern.
SNAPSHOT = re.compile(r"(^|/)schemas/(.+/)?v(?P<version>\d+)(\.output|\.input)?\.json$")


def released(root: Path, base: str) -> dict[str, tuple[str, str]]:
    """Snapshots at the merge base of `base` and HEAD, mapped to their mode and blob id.

    The merge base keeps a branch behind `base` from reporting as deleted a snapshot released
    after it forked. Without a merge base (unrelated history, a shallow clone) the tip of
    `base` is used.
    """
    not_a_commit = f"check_schemas: base '{base}' is not a commit; fetch it or pass --base."
    try:
        _git.revision(base)
    except ValueError:
        sys.exit(not_a_commit)
    if _git.run("rev-parse", "--verify", "--quiet", f"{base}^{{commit}}", cwd=root).returncode != 0:
        sys.exit(not_a_commit)
    fork = _git.run("merge-base", "--end-of-options", base, "HEAD", cwd=root)
    tree = fork.stdout.strip() if fork.returncode == 0 else base
    listing = _git.run("ls-tree", "-r", "-z", "--end-of-options", tree, "--", "schemas", cwd=root)
    if listing.returncode != 0:
        sys.exit(f"check_schemas: git ls-tree failed: {listing.stderr.strip()}")
    entries: dict[str, tuple[str, str]] = {}
    for entry in listing.stdout.split("\0"):
        meta, _, path = entry.partition("\t")
        if SNAPSHOT.search(path):
            mode, _, blob = meta.split()
            entries[path] = (mode, blob)
    return entries


def _mode(file: Path) -> str | None:
    """The git mode of a file in the working tree, or None when it is missing or a folder."""
    try:
        info = file.lstat()
    except FileNotFoundError:
        return None
    if stat.S_ISLNK(info.st_mode):
        return "120000"
    if stat.S_ISREG(info.st_mode):
        return "100755" if info.st_mode & stat.S_IXUSR else "100644"
    return None


def changed_or_deleted(root: Path, base: str) -> list[str]:
    entries = released(root, base)
    problems: list[str] = []
    regular: list[str] = []
    for path, (mode, _) in sorted(entries.items()):
        local = _mode(root / path)
        if local is None:
            problems.append(f"{path}: released on {base} and deleted")
        elif local == "120000":
            problems.append(f"{path}: released on {base} and replaced by a symlink")
        elif local != mode:
            problems.append(f"{path}: released on {base} and its mode changed ({mode} to {local})")
        else:
            regular.append(path)
    if regular:
        hashed = _git.run("hash-object", "--", *regular, cwd=root)
        if hashed.returncode != 0:
            sys.exit(f"check_schemas: git hash-object failed: {hashed.stderr.strip()}")
        blobs = hashed.stdout.split()
        if len(blobs) != len(regular):
            sys.exit(
                f"check_schemas: git hash-object returned {len(blobs)} ids for "
                f"{len(regular)} files."
            )
        for path, blob in zip(regular, blobs, strict=True):
            if blob != entries[path][1]:
                problems.append(f"{path}: released on {base} and changed")
    return sorted(problems)


def version_gaps(root: Path) -> list[str]:
    folders: dict[str, set[int]] = defaultdict(set)
    problems: list[str] = []
    for file in (root / "schemas").rglob("v*.json"):
        path = file.relative_to(root).as_posix()
        match = SNAPSHOT.search(path)
        if match is None or not file.is_file():
            continue
        digits = match["version"]
        if digits.startswith("0"):
            problems.append(f"{path}: the version is not a positive integer")
            continue
        folders[file.parent.relative_to(root).as_posix()].add(int(digits))
    for folder, versions in sorted(folders.items()):
        found = ", ".join(str(v) for v in sorted(versions))
        for missing in sorted(set(range(1, max(versions) + 1)) - versions):
            problems.append(f"{folder}: version {missing} is missing (versions found: {found})")
    return sorted(problems)


def problems(root: Path, base: str) -> list[str]:
    """Every released snapshot changed or deleted, and every gap in versions."""
    return changed_or_deleted(root, base) + version_gaps(root)


def main(argv: list[str] | None = None) -> None:
    parser = argparse.ArgumentParser(description=__doc__.split("\n", 1)[0])
    parser.add_argument("--base", default="origin/main", help="base ref (default: origin/main)")
    args = parser.parse_args(argv)
    found = problems(ROOT, args.base)
    if found:
        sys.exit(
            "Released schema snapshots must not change: write the next version v<N+1> beside "
            "the unchanged v<N> (ADR 0009, C5).\n  " + "\n  ".join(found)
        )
    print(f"Schema snapshots: none released on {args.base} changed, no gap in versions.")


if __name__ == "__main__":
    main()
