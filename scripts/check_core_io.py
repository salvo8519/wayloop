"""The pure crates perform no I/O: allowlisted dependencies only, and compiler lints in place.

The core decides and the runtime and the adapters act (ADR 0001, ADR 0006). The pure crates
are wayloop-schemas, the contract types, and wayloop-core, built on it. Two guards keep that
boundary from eroding unnoticed:

  dependencies  every crate reachable from a pure crate through normal dependencies must be on
                its reviewed allowlist (PURE_CRATES). Each pure crate is resolved on its own by
                `cargo tree -p <crate> -e normal --target all --all-features --locked`: optional
                dependencies behind any feature and dependencies of every target are seen,
                features enabled only by other members of the workspace are not blamed on it,
                and the lock file is never rewritten. Development dependencies (tests only) and
                build dependencies (run at build time) are not shipped and are skipped. The
                allowlists also fix the direction of the workspace: schemas depends on no
                Wayloop crate, core on schemas only, and neither on the runtime or the CLI.
  source        I/O in the source is refused by Clippy, not by this script: each pure crate
                forbids the print, dbg, disallowed-methods and disallowed-types lints at its
                root, and its clippy.toml lists the parts of the standard library that perform
                I/O or read ambient state (files, network, processes, environment, standard
                streams, threads, clocks, randomly seeded hash maps). `cargo clippy` in
                `just check` and CI enforces them. This script only checks that the attribute
                and the clippy.toml entries are present, so that neither is removed silently.

IO_CRATES and the order of WORKSPACE only make the error message clearer; the allowlist
decides.

    python scripts/check_core_io.py
"""

from __future__ import annotations

import re
import subprocess
import sys
import tomllib
from collections import deque
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]

# Reviewed allowlist of each pure crate: every crate reachable through normal dependencies.
# serde with derive and serde_json perform no I/O and every contract type needs them (SPEC-0001
# plan, D2). The set lists every crate `cargo tree` reaches from them: serde_core, serde_derive
# and its macro crates (proc-macro2, quote, syn, unicode-ident), and the number and search
# helpers of serde_json (itoa, zmij, memchr).
SERDE = frozenset(
    {
        "serde",
        "serde_core",
        "serde_derive",
        "proc-macro2",
        "quote",
        "syn",
        "unicode-ident",
        "serde_json",
        "itoa",
        "memchr",
        "zmij",
    }
)
PURE_CRATES: dict[str, frozenset[str]] = {
    "wayloop-schemas": SERDE,
    "wayloop-core": SERDE | {"wayloop-schemas"},
}

# Dependencies point one way: a crate may depend only on the crates before it (ADR 0006).
WORKSPACE = ("wayloop-schemas", "wayloop-core", "wayloop-runtime", "wayloop-cli")

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

# Lints every pure crate forbids at its root; forbid, unlike deny, cannot be lifted by an
# inner allow.
REQUIRED_LINTS = (
    "clippy::print_stdout",
    "clippy::print_stderr",
    "clippy::dbg_macro",
    "clippy::disallowed_methods",
    "clippy::disallowed_types",
)


def _paths(module: str, *names: str) -> frozenset[str]:
    return frozenset(f"{module}::{name}" for name in names)


# Entries every clippy.toml of a pure crate must hold; it may hold more.
REQUIRED_METHODS = (
    _paths(
        "std::fs",
        "canonicalize",
        "copy",
        "create_dir",
        "create_dir_all",
        "exists",
        "hard_link",
        "metadata",
        "read",
        "read_dir",
        "read_link",
        "read_to_string",
        "remove_dir",
        "remove_dir_all",
        "remove_file",
        "rename",
        "set_permissions",
        "symlink_metadata",
        "write",
    )
    | _paths("std::net::ToSocketAddrs", "to_socket_addrs")
    | _paths("std::process", "abort", "exit", "id")
    | _paths(
        "std::env",
        "args",
        "args_os",
        "current_dir",
        "current_exe",
        "home_dir",
        "remove_var",
        "set_current_dir",
        "set_var",
        "temp_dir",
        "var",
        "var_os",
        "vars",
        "vars_os",
    )
    | _paths("std::io", "stdin", "stdout", "stderr")
    | _paths("std::thread", "scope", "sleep", "spawn")
    | _paths("std::time::Instant", "now")
    | _paths("std::time::SystemTime", "now")
)
REQUIRED_TYPES = (
    _paths("std::fs", "DirBuilder", "DirEntry", "File", "OpenOptions", "ReadDir")
    | _paths("std::net", "TcpListener", "TcpStream", "UdpSocket")
    | _paths("std::process", "Child", "Command")
    | _paths("std::env", "Args", "ArgsOs", "Vars", "VarsOs")
    | _paths("std::io", "Stderr", "Stdin", "Stdout")
    | _paths("std::thread", "Builder")
    | _paths("std::collections", "HashMap", "HashSet")
    | _paths("std::hash", "RandomState")
)

TREE_LINE = re.compile(r"^(\d+)(\S+) v\S+")
LINE_COMMENT = re.compile(r"//[^\n]*")
BLOCK_COMMENT = re.compile(r"/\*.*?\*/", re.DOTALL)
FORBID = re.compile(r"#!\[\s*forbid\s*\(([^)]*)\)\s*\]")


class CargoError(Exception):
    """cargo tree failed: the crate is unknown, or the lock file is out of date."""


class TreeLineError(ValueError):
    """A line of the output of cargo tree does not have the expected shape."""

    def __init__(self, line: str) -> None:
        super().__init__(f"unexpected line in the output of cargo tree: {line!r}")


def cargo_tree(crate: str, root: Path = ROOT) -> str:
    """The normal dependencies of one crate, resolved on its own with all features."""
    result = subprocess.run(
        [
            "cargo",
            "tree",
            "--manifest-path",
            str(root / "Cargo.toml"),
            "-p",
            crate,
            "-e",
            "normal",
            "--target",
            "all",
            "--all-features",
            "--locked",
            "--color",
            "never",
            "--prefix",
            "depth",
            "--format",
            "{p}",
        ],
        cwd=root,
        capture_output=True,
        text=True,
        encoding="utf-8",
        check=False,
    )
    if result.returncode != 0:
        lines = [line for line in result.stderr.splitlines() if line.strip()]
        errors = [line for line in lines if line.startswith("error")]
        raise CargoError((errors or lines or [f"exit status {result.returncode}"])[0])
    return result.stdout


def dependency_paths(text: str) -> dict[str, list[str]]:
    """Every crate below the root of a `cargo tree` output, with the shortest path to it."""
    edges: dict[str, list[str]] = {}
    stack: list[str] = []
    for line in filter(str.strip, text.splitlines()):
        match = TREE_LINE.match(line)
        if not match:
            raise TreeLineError(line)
        depth, name = int(match.group(1)), match.group(2)
        del stack[depth:]
        if stack:
            edges.setdefault(stack[-1], []).append(name)
        stack.append(name)
    if not stack:
        return {}
    root = stack[0]
    paths = {root: [root]}
    queue = deque([root])
    while queue:
        name = queue.popleft()
        for child in edges.get(name, []):
            if child not in paths:
                paths[child] = [*paths[name], child]
                queue.append(child)
    del paths[root]
    return paths


def _why(crate: str, dependency: str) -> str:
    if dependency.replace("-", "_") in IO_CRATES:
        return " (a known I/O crate)"
    if (
        dependency in WORKSPACE
        and crate in WORKSPACE
        and WORKSPACE.index(dependency) > WORKSPACE.index(crate)
    ):
        return " (against the dependency direction, ADR 0006)"
    return ""


def refused(crate: str, text: str, allowed: frozenset[str]) -> list[str]:
    """Every crate reached from a pure crate that is not on its allowlist, with its path."""
    return [
        f"{' -> '.join(path)}: {name} is not on the allowlist of {crate}{_why(crate, name)}"
        for name, path in dependency_paths(text).items()
        if name not in allowed
    ]


def _entries(config: dict[str, object], key: str) -> set[str]:
    entries = config.get(key, [])
    if not isinstance(entries, list):
        return set()
    return {
        entry if isinstance(entry, str) else str(entry.get("path", ""))
        for entry in entries
        if isinstance(entry, (str, dict))
    }


def missing_guards(crate_dir: Path) -> list[str]:
    """What is missing of the lint guards of a pure crate: its clippy.toml, its forbid lints."""
    found = []
    clippy = crate_dir / "clippy.toml"
    if not clippy.is_file():
        found.append("clippy.toml is missing")
    else:
        try:
            config = tomllib.loads(clippy.read_text(encoding="utf-8"))
        except tomllib.TOMLDecodeError as error:
            found.append(f"clippy.toml is not valid TOML: {error}")
        else:
            for key, required in (
                ("disallowed-methods", REQUIRED_METHODS),
                ("disallowed-types", REQUIRED_TYPES),
            ):
                found += [
                    f"clippy.toml: {key} lacks {path}"
                    for path in sorted(required - _entries(config, key))
                ]
    lib = crate_dir / "src" / "lib.rs"
    text = lib.read_text(encoding="utf-8") if lib.is_file() else ""
    code = LINE_COMMENT.sub("", BLOCK_COMMENT.sub("", text))
    forbidden = {
        lint.strip() for match in FORBID.finditer(code) for lint in match.group(1).split(",")
    }
    found += [
        f"src/lib.rs: #![forbid(...)] lacks {lint}"
        for lint in REQUIRED_LINTS
        if lint not in forbidden
    ]
    return found


def main(root: Path = ROOT) -> None:
    problems = []
    for crate, allowed in PURE_CRATES.items():
        try:
            problems += refused(crate, cargo_tree(crate, root), allowed)
        except (CargoError, TreeLineError) as error:
            problems.append(f"{crate}: cargo tree failed: {error}")
        problems += [
            f"crates/{crate}: {problem}" for problem in missing_guards(root / "crates" / crate)
        ]
    if problems:
        sys.exit(
            f"{' and '.join(PURE_CRATES)} must perform no I/O (ADR 0001, ADR 0006). Move I/O to "
            "wayloop-runtime or an adapter, or extend an allowlist in scripts/check_core_io.py "
            "after review:\n  " + "\n  ".join(problems)
        )
    print(
        f"{', '.join(PURE_CRATES)}: only their allowlisted dependencies; "
        "clippy.toml and forbidden lints in place."
    )


if __name__ == "__main__":
    main()
