"""Claude Code hooks in .claude/: registered, executable, and correct on real input."""

from __future__ import annotations

import json
import os
import shutil
import subprocess
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parents[2]
HOOKS = ROOT / ".claude" / "hooks"


def registered() -> list[str]:
    settings = json.loads((ROOT / ".claude" / "settings.json").read_text(encoding="utf-8"))
    return [
        hook["command"].split("/.claude/hooks/")[1]
        for groups in settings["hooks"].values()
        for group in groups
        for hook in group["hooks"]
    ]


def run(hook: str, payload: dict[str, object], cwd: Path) -> subprocess.CompletedProcess[str]:
    env = {**os.environ, "CLAUDE_PROJECT_DIR": str(cwd)}
    return subprocess.run(
        [str(cwd / ".claude" / "hooks" / hook)],
        input=json.dumps(payload),
        capture_output=True,
        text=True,
        env=env,
        cwd=cwd,
        check=False,
    )


def test_every_hook_is_registered_and_executable() -> None:
    files = sorted(p.name for p in HOOKS.glob("*.sh"))
    assert sorted(registered()) == files
    assert all(os.access(HOOKS / f, os.X_OK) for f in files)


@pytest.fixture
def project(tmp_path: Path) -> Path:
    """A git repository with the hooks and the documentation script."""
    shutil.copytree(ROOT / ".claude", tmp_path / ".claude")
    (tmp_path / "scripts").mkdir()
    shutil.copy(ROOT / "scripts" / "docs_sync.py", tmp_path / "scripts" / "docs_sync.py")
    env = {
        **os.environ,
        "GIT_AUTHOR_NAME": "t",
        "GIT_AUTHOR_EMAIL": "t@t",
        "GIT_COMMITTER_NAME": "t",
        "GIT_COMMITTER_EMAIL": "t@t",
    }
    for cmd in (["init", "-q", "-b", "main"], ["add", "."], ["commit", "-q", "-m", "base"]):
        subprocess.run(["git", *cmd], cwd=tmp_path, env=env, check=True)
    return tmp_path


def test_stop_hook_sends_missing_documentation_back(project: Path) -> None:
    (project / "schemas").mkdir()
    (project / "schemas" / "x.json").write_text("{}\n", encoding="utf-8")
    result = run("docs-impact.sh", {"stop_hook_active": False}, project)
    assert result.returncode == 2
    assert "schemas changed" in result.stderr


def test_stop_hook_passes_with_documentation_and_never_loops(project: Path) -> None:
    (project / "schemas").mkdir()
    (project / "schemas" / "x.json").write_text("{}\n", encoding="utf-8")
    assert run("docs-impact.sh", {"stop_hook_active": True}, project).returncode == 0
    (project / "docs" / "en" / "reference").mkdir(parents=True)
    (project / "docs" / "en" / "reference" / "x.md").write_text("# X\n", encoding="utf-8")
    assert run("docs-impact.sh", {"stop_hook_active": False}, project).returncode == 0


@pytest.mark.skipif(shutil.which("uv") is None, reason="uv is needed to run the formatter")
def test_format_hook_formats_python_and_ignores_other_files(project: Path) -> None:
    source = project / "x.py"
    source.write_text("x = {  'a':1 }\n", encoding="utf-8")
    note = project / "note.md"
    note.write_text("text  \n", encoding="utf-8")
    for path in (source, note, project / "missing.py"):
        assert run("format.sh", {"tool_input": {"file_path": str(path)}}, ROOT).returncode == 0
    assert source.read_text(encoding="utf-8") == 'x = {"a": 1}\n'
    assert note.read_text(encoding="utf-8") == "text  \n"


def test_format_hook_ignores_input_without_a_file(project: Path) -> None:
    assert run("format.sh", {"tool_input": {}}, project).returncode == 0
    assert run("format.sh", {}, project).returncode == 0
