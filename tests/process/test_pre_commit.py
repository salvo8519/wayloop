"""pre-commit: the formatters fix the staged files instead of only checking them (#240)."""

from __future__ import annotations

import os
import shutil
import subprocess
from pathlib import Path
from typing import Any

import pytest
import yaml

ROOT = Path(__file__).resolve().parents[2]
FIXING = ("cargo-fmt", "ruff-check", "ruff-format")
PRE_COMMIT = ROOT / ".venv" / "bin" / "pre-commit"


def hooks() -> dict[str, dict[str, Any]]:
    config = yaml.safe_load((ROOT / ".pre-commit-config.yaml").read_text(encoding="utf-8"))
    return {h["id"]: h for repo in config["repos"] for h in repo["hooks"]}


def test_the_formatting_hooks_fix_instead_of_checking() -> None:
    entries = {name: hooks()[name]["entry"] for name in FIXING}
    assert all("--check" not in entry for entry in entries.values()), entries
    assert "--fix" in entries["ruff-check"]


@pytest.mark.skipif(
    not PRE_COMMIT.is_file() or shutil.which("cargo") is None or shutil.which("uv") is None,
    reason="pre-commit, cargo and uv are needed",
)
def test_a_commit_of_unformatted_files_stops_once_with_the_files_fixed(tmp_path: Path) -> None:
    config = {"repos": [{"repo": "local", "hooks": [hooks()[name] for name in FIXING]}]}
    (tmp_path / ".pre-commit-config.yaml").write_text(yaml.safe_dump(config), encoding="utf-8")
    shutil.copy(ROOT / "pyproject.toml", tmp_path / "pyproject.toml")
    (tmp_path / "Cargo.toml").write_text(
        '[workspace]\nmembers = ["crates/demo"]\nresolver = "3"\n', encoding="utf-8"
    )
    crate = tmp_path / "crates" / "demo"
    (crate / "src").mkdir(parents=True)
    (crate / "Cargo.toml").write_text(
        '[package]\nname = "demo"\nversion = "0.0.0"\nedition = "2024"\n', encoding="utf-8"
    )
    rust = crate / "src" / "lib.rs"
    rust.write_text("pub fn  answer( )->u32{42}\n", encoding="utf-8")
    python = tmp_path / "scripts" / "tool.py"
    python.parent.mkdir()
    python.write_text('import os\nx = {  "a":1 }\n', encoding="utf-8")
    env = {
        **os.environ,
        "GIT_AUTHOR_NAME": "t",
        "GIT_AUTHOR_EMAIL": "t@t",
        "GIT_COMMITTER_NAME": "t",
        "GIT_COMMITTER_EMAIL": "t@t",
        # `uv run` in the entries uses the environment of the repository, without syncing.
        "UV_PROJECT": str(ROOT),
        "UV_NO_SYNC": "1",
    }
    for args in (["init", "-q", "-b", "main"], ["add", "."]):
        subprocess.run(["git", *args], cwd=tmp_path, env=env, check=True)
    first = subprocess.run(
        [str(PRE_COMMIT), "run"], cwd=tmp_path, env=env, capture_output=True, text=True
    )
    assert first.returncode == 1, first.stdout
    assert "files were modified by this hook" in first.stdout
    assert rust.read_text(encoding="utf-8") == "pub fn answer() -> u32 {\n    42\n}\n"
    assert python.read_text(encoding="utf-8") == 'x = {"a": 1}\n'
    subprocess.run(["git", "add", "."], cwd=tmp_path, env=env, check=True)
    second = subprocess.run(
        [str(PRE_COMMIT), "run"], cwd=tmp_path, env=env, capture_output=True, text=True
    )
    assert second.returncode == 0, second.stdout
