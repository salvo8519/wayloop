"""Stop hook: the fast checks on the files changed in the session, before the turn ends.

Changed files are those of the working tree and the index that differ from HEAD, plus untracked
files that git does not ignore. Only the relevant checks run:

- `.rs`: `rustfmt --check` on the files, then `cargo clippy -p <crate> --all-targets -- -D
  warnings` for every crate under `crates/` that a file belongs to;
- `.py`: `ruff check` and `ruff format --check` on the files, `mypy` on those inside the paths
  of `[tool.mypy] files` in `pyproject.toml`.

Exit status 2 sends the findings to the agent on stderr, so it fixes them first. When the hook
input says `stop_hook_active` (the agent is already continuing because of a Stop hook), the
findings are reported and the turn is not blocked a second time. The checks share a time budget
(`WAYLOOP_QUALITY_BUDGET` seconds, default 60); a check that does not finish in time is
reported and never blocks. A missing tool is reported and skipped (ADR 0027).
"""

from __future__ import annotations

import json
import os
import shutil
import subprocess
import sys
import time
import tomllib
from pathlib import Path

MAX_LINES = 40


def changed_files(root: Path) -> list[str] | None:
    """Existing files changed against HEAD, staged or not, and untracked ones; None outside git."""
    diff = subprocess.run(
        ["git", "diff", "--name-only", "HEAD"],
        cwd=root,
        capture_output=True,
        text=True,
        encoding="utf-8",
        check=False,
    )
    if diff.returncode != 0:
        return None
    untracked = subprocess.run(
        ["git", "ls-files", "--others", "--exclude-standard"],
        cwd=root,
        capture_output=True,
        text=True,
        encoding="utf-8",
        check=False,
    )
    names = {*diff.stdout.splitlines(), *untracked.stdout.splitlines()}
    return sorted(n for n in names if n and (root / n).is_file())


def crate_of(root: Path, path: str) -> str | None:
    """Package name of the crate under `crates/` that contains the path."""
    parts = Path(path).parts
    if len(parts) < 3 or parts[0] != "crates":
        return None
    manifest = root / "crates" / parts[1] / "Cargo.toml"
    if not manifest.is_file():
        return None
    with manifest.open("rb") as f:
        name = tomllib.load(f).get("package", {}).get("name")
    return name if isinstance(name, str) else None


def mypy_paths(root: Path) -> list[str]:
    """The paths that mypy checks, from `[tool.mypy] files` of pyproject.toml."""
    pyproject = root / "pyproject.toml"
    if not pyproject.is_file():
        return []
    with pyproject.open("rb") as f:
        files = tomllib.load(f).get("tool", {}).get("mypy", {}).get("files", [])
    return [str(p).rstrip("/") for p in files] if isinstance(files, list) else []


def python_tool(root: Path, name: str) -> list[str] | None:
    """The command of a Python tool: the project environment first, then uv."""
    local = root / ".venv" / "bin" / name
    if local.is_file():
        return [str(local)]
    if shutil.which("uv"):
        return ["uv", "run", "--quiet", name]
    return None


def plan(root: Path, files: list[str]) -> list[tuple[str, list[str] | None]]:
    """The checks to run, cheapest first: a label and a command (None when the tool is missing)."""
    checks: list[tuple[str, list[str] | None]] = []
    rust = [f for f in files if f.endswith(".rs")]
    python = [f for f in files if f.endswith(".py")]
    if rust:
        rustfmt = shutil.which("rustfmt")
        checks.append(
            (
                "rustfmt --check",
                [rustfmt, "--check", "--edition", "2024", *rust] if rustfmt else None,
            )
        )
    if python:
        ruff = python_tool(root, "ruff")
        checks.append(
            (
                "ruff check",
                [*ruff, "check", "--output-format", "concise", *python] if ruff else None,
            )
        )
        checks.append(
            ("ruff format --check", [*ruff, "format", "--check", *python] if ruff else None)
        )
        typed = [
            f for f in python if any(f == p or f.startswith(p + "/") for p in mypy_paths(root))
        ]
        if typed:
            mypy = python_tool(root, "mypy")
            checks.append(("mypy", [*mypy, *typed] if mypy else None))
    crates = sorted({c for c in (crate_of(root, f) for f in rust) if c})
    if crates:
        cargo = shutil.which("cargo")
        selected = [arg for c in crates for arg in ("-p", c)]
        label = "cargo clippy " + " ".join(selected)
        lint = ["clippy", "--quiet", *selected, "--all-targets", "--", "-D", "warnings"]
        checks.append((label, [cargo, *lint] if cargo else None))
    return checks


def tail(text: str) -> str:
    lines = text.strip().splitlines()
    if len(lines) <= MAX_LINES:
        return "\n".join(lines)
    return "\n".join([*lines[:MAX_LINES], f"... {len(lines) - MAX_LINES} more lines"])


def run_checks(
    root: Path, checks: list[tuple[str, list[str] | None]], budget: float
) -> tuple[list[str], list[str]]:
    """Findings that block, and notes that do not (missing tools, time budget spent)."""
    findings: list[str] = []
    notes: list[str] = []
    deadline = time.monotonic() + budget
    for label, command in checks:
        if command is None:
            notes.append(f"{label}: tool not found, check skipped.")
            continue
        remaining = deadline - time.monotonic()
        if remaining <= 0:
            notes.append(f"{label}: not run, the time budget of {budget:g} s is spent.")
            continue
        try:
            done = subprocess.run(
                command,
                cwd=root,
                capture_output=True,
                text=True,
                encoding="utf-8",
                timeout=remaining,
                check=False,
            )
        except subprocess.TimeoutExpired:
            notes.append(f"{label}: stopped, the time budget of {budget:g} s is spent.")
            continue
        if done.returncode != 0:
            findings.append(f"$ {label}\n{tail(done.stdout + done.stderr)}")
    return findings, notes


def main() -> int:
    try:
        payload = json.load(sys.stdin)
    except json.JSONDecodeError:
        payload = {}
    active = bool(payload.get("stop_hook_active")) if isinstance(payload, dict) else False
    root = Path(os.environ.get("CLAUDE_PROJECT_DIR") or ".").resolve()
    if shutil.which("git") is None:
        print("quality-gate hook: git not found, check skipped", file=sys.stderr)
        return 0
    files = changed_files(root)
    if not files:
        return 0
    checks = plan(root, files)
    if not checks:
        return 0
    budget = float(os.environ.get("WAYLOOP_QUALITY_BUDGET", "60"))
    findings, notes = run_checks(root, checks, budget)
    for note in notes:
        print(f"quality-gate hook: {note}", file=sys.stderr)
    if not findings:
        return 0
    print(
        "The files changed in this session fail the fast checks. Fix every finding before you "
        "end the turn (`uv run just fmt` applies the formatting and the safe fixes):\n\n"
        + "\n\n".join(findings),
        file=sys.stderr,
    )
    return 0 if active else 2


if __name__ == "__main__":
    sys.exit(main())
