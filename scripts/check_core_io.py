"""The pure crates perform no I/O: no I/O crate among their dependencies, none in their source.

The core decides and the runtime and the adapters act (ADR 0001, ADR 0006). The pure crates
are wayloop-core and wayloop-schemas, the contract types the core is built on. Two checks keep
that boundary from eroding unnoticed:

  dependencies  the normal dependencies of each pure crate, direct and transitive, on every
                target, as resolved by `cargo metadata`, contain no crate of IO_CRATES.
                Development dependencies (tests only) and build dependencies (run at build
                time) are not shipped with the crate and are not followed.
  source        the source of each pure crate, comments excluded, uses no part of the standard
                library that performs I/O: std::fs, std::net, std::process, std::env, the
                standard streams, and the print macros. Writing to a buffer the caller owns
                (std::io::Write, std::fmt::Write) is allowed.

IO_CRATES is a list of known crates, not a proof: a new I/O crate is added to it when found.

    python scripts/check_core_io.py
"""

from __future__ import annotations

import json
import re
import subprocess
import sys
from collections import deque
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[1]
CORE = "wayloop-core"
PURE_CRATES = (CORE, "wayloop-schemas")

# Crates whose purpose is I/O: async runtimes and event loops, networking and HTTP clients,
# databases and storage engines, files and processes, git and the tracker.
IO_CRATES = frozenset(
    name.replace("-", "_")
    for name in (
        # Asynchronous runtimes and event loops.
        "tokio",
        "tokio-util",
        "async-std",
        "smol",
        "async-io",
        "async-net",
        "async-fs",
        "async-process",
        "mio",
        "polling",
        # Networking and HTTP.
        "socket2",
        "hyper",
        "hyper-util",
        "h2",
        "reqwest",
        "ureq",
        "curl",
        "curl-sys",
        "isahc",
        "surf",
        "rustls",
        "native-tls",
        "openssl",
        "openssl-sys",
        "tungstenite",
        # Databases and storage.
        "rusqlite",
        "libsqlite3-sys",
        "sqlx",
        "diesel",
        "sled",
        "redb",
        "rocksdb",
        "memmap2",
        # Files, directories and processes.
        "tempfile",
        "walkdir",
        "notify",
        "fs-err",
        "fs_extra",
        "dirs",
        "directories",
        "duct",
        "nix",
        # Git and the tracker.
        "git2",
        "libgit2-sys",
        "gix",
        "octocrab",
    )
)

# Parts of the standard library that perform I/O, matched on source without comments.
IO_SOURCE = (
    re.compile(r"(?<![\w:])(?:::)?std::(?:fs|net|process|env)\b"),
    re.compile(r"\bstd::io::(?:stdin|stdout|stderr)\b"),
    re.compile(r"\buse\s+(?:::)?std::\{[^}]*?\b(?:fs|net|process|env)::"),
    re.compile(r"\buse\s+(?:::)?std::\{[^}]*?\b(?:fs|net|process|env)\s*[,}]"),
    re.compile(r"(?<![\w:])e?print(?:ln)?!"),
)
LINE_COMMENT = re.compile(r"//[^\n]*")
BLOCK_COMMENT = re.compile(r"/\*.*?\*/", re.DOTALL)


def cargo_metadata() -> dict[str, Any]:
    out = subprocess.run(
        ["cargo", "metadata", "--format-version", "1", "--manifest-path", str(ROOT / "Cargo.toml")],
        cwd=ROOT,
        capture_output=True,
        text=True,
        check=True,
        encoding="utf-8",
    ).stdout
    return json.loads(out)


def io_dependencies(metadata: dict[str, Any], crate: str = CORE) -> list[list[str]]:
    """Every I/O crate reached from a crate by normal dependencies, with the shortest path."""
    names = {package["id"]: package["name"] for package in metadata["packages"]}
    nodes = {node["id"]: node for node in metadata["resolve"]["nodes"]}
    root = next(pid for pid in metadata["workspace_members"] if names[pid] == crate)
    paths = {root: [names[root]]}
    queue = deque([root])
    found = []
    while queue:
        pid = queue.popleft()
        for dep in nodes[pid]["deps"]:
            if not any(kind["kind"] is None for kind in dep["dep_kinds"]):
                continue
            child = dep["pkg"]
            if child in paths:
                continue
            paths[child] = [*paths[pid], names[child]]
            if names[child].replace("-", "_") in IO_CRATES:
                found.append(paths[child])
            queue.append(child)
    return found


def _blank(match: re.Match[str]) -> str:
    """A comment replaced by its newlines, so that line numbers do not move."""
    return "\n" * match.group(0).count("\n")


def io_in_source(src: Path) -> list[tuple[Path, int, str]]:
    """Every line of the source, comments excluded, that uses I/O of the standard library."""
    found = []
    for path in sorted(src.rglob("*.rs")):
        text = path.read_text(encoding="utf-8")
        code = LINE_COMMENT.sub("", BLOCK_COMMENT.sub(_blank, text))
        lines = sorted(
            {
                code.count("\n", 0, m.start()) + 1
                for pattern in IO_SOURCE
                for m in pattern.finditer(code)
            }
        )
        original = text.splitlines()
        found.extend((path, number, original[number - 1].strip()) for number in lines)
    return found


def main() -> None:
    metadata = cargo_metadata()
    members = {
        package["name"]
        for package in metadata["packages"]
        if package["id"] in metadata["workspace_members"]
    }
    problems = [
        f"{crate} is not a member of the workspace" for crate in PURE_CRATES if crate not in members
    ]
    for crate in (crate for crate in PURE_CRATES if crate in members):
        problems += [
            "I/O crate in the dependencies: " + " -> ".join(path)
            for path in io_dependencies(metadata, crate)
        ]
        problems += [
            f"I/O in the source: {path.relative_to(ROOT).as_posix()}:{number}: {line}"
            for path, number, line in io_in_source(ROOT / "crates" / crate / "src")
        ]
    if problems:
        sys.exit(
            f"{' and '.join(PURE_CRATES)} must perform no I/O (ADR 0001, ADR 0006); move it to "
            "wayloop-runtime or an adapter:\n  " + "\n  ".join(problems)
        )
    print(f"{', '.join(PURE_CRATES)}: no I/O crate among their dependencies, none in their source.")


if __name__ == "__main__":
    main()
