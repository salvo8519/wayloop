"""Released schema snapshots never change (ADR 0009, C5; SPEC-0001 R1, R16, AC3).

A snapshot is a versioned JSON Schema under `schemas/` named `v<N>.json`, `v<N>.output.json`
or `v<N>.input.json`. A snapshot that exists on the base branch is released: it stays in the
working tree with the same content. A schema change ships as a new snapshot v<N+1> beside the
unchanged v<N>, and within one folder the versions run from 1 without a gap. The hook
`.claude/hooks/protect_contracts.py` blocks the same edits before they happen.

    python scripts/check_schemas.py                    # against origin/main
    python scripts/check_schemas.py --base origin/x    # against another base
"""

from __future__ import annotations

import argparse
import re
import subprocess
import sys
from collections import defaultdict
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]

# The same pattern as the hook; a test keeps the two equal.
SNAPSHOT = re.compile(r"(^|/)schemas/(.+/)?v\d+(\.output|\.input)?\.json$")
# The file name of a snapshot under `schemas/`, with its version.
VERSION = re.compile(r"v(\d+)(?:\.output|\.input)?\.json")


def _git(root: Path, *args: str) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        ["git", *args], cwd=root, capture_output=True, text=True, encoding="utf-8", check=False
    )


def released(root: Path, base: str) -> dict[str, str]:
    """Snapshots on `base`, mapped to their blob ids."""
    if _git(root, "rev-parse", "--verify", "--quiet", f"{base}^{{commit}}").returncode != 0:
        sys.exit(f"check_schemas: base '{base}' is not a commit; fetch it or pass --base.")
    listing = _git(root, "ls-tree", "-r", base, "--", "schemas")
    if listing.returncode != 0:
        sys.exit(f"check_schemas: git ls-tree failed: {listing.stderr.strip()}")
    blobs: dict[str, str] = {}
    for line in listing.stdout.splitlines():
        meta, _, path = line.partition("\t")
        if SNAPSHOT.search(path):
            blobs[path] = meta.split()[2]
    return blobs


def changed_or_deleted(root: Path, base: str) -> list[str]:
    blobs = released(root, base)
    present = [path for path in sorted(blobs) if (root / path).is_file()]
    problems = [
        f"{path}: released on {base} and deleted" for path in sorted(blobs) if path not in present
    ]
    if present:
        hashed = _git(root, "hash-object", "--", *present)
        if hashed.returncode != 0:
            sys.exit(f"check_schemas: git hash-object failed: {hashed.stderr.strip()}")
        for path, blob in zip(present, hashed.stdout.split(), strict=True):
            if blob != blobs[path]:
                problems.append(f"{path}: released on {base} and changed")
    return sorted(problems)


def version_gaps(root: Path) -> list[str]:
    folders: dict[str, set[int]] = defaultdict(set)
    problems: list[str] = []
    for file in (root / "schemas").rglob("v*.json"):
        path = file.relative_to(root).as_posix()
        match = VERSION.fullmatch(file.name)
        if match is None or not file.is_file():
            continue
        digits = match.group(1)
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
