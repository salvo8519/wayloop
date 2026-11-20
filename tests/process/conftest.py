"""Shared helpers of the process tests: a throwaway git repository and the hooks with the
scripts they need. The scripts themselves are imported by name (`pythonpath` in
pyproject.toml), like `python scripts/<name>.py` does."""

from __future__ import annotations

import os
import shutil
import subprocess
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parents[2]
# A fixed identity and no user or system configuration (signing, hooks, default branch).
GIT_ENV = {
    "GIT_AUTHOR_NAME": "t",
    "GIT_AUTHOR_EMAIL": "t@example.invalid",
    "GIT_COMMITTER_NAME": "t",
    "GIT_COMMITTER_EMAIL": "t@example.invalid",
    "GIT_CONFIG_NOSYSTEM": "1",
    "GIT_CONFIG_GLOBAL": os.devnull,
}


def git(cwd: Path, *args: str) -> str:
    """Output of `git <args>` in `cwd`; a failure fails the test."""
    return subprocess.run(
        ["git", *args],
        cwd=cwd,
        env={**os.environ, **GIT_ENV},
        capture_output=True,
        text=True,
        encoding="utf-8",
        check=True,
    ).stdout


def copy_hooks(root: Path) -> None:
    """The Claude Code configuration and the scripts its hooks import or run, as on main."""
    ignore = shutil.ignore_patterns("__pycache__")
    shutil.copytree(ROOT / ".claude", root / ".claude", dirs_exist_ok=True, ignore=ignore)
    shutil.copytree(ROOT / "scripts", root / "scripts", dirs_exist_ok=True, ignore=ignore)


@pytest.fixture
def git_repo(tmp_path: Path) -> Path:
    """An empty git repository on `main`, in its own folder of the test's temporary directory."""
    repo = tmp_path / "repo"
    repo.mkdir()
    git(repo, "init", "-q", "-b", "main")
    return repo
