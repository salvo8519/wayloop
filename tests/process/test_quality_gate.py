"""Stop hook quality-gate: the fast checks on the files changed in the session (#240)."""

from __future__ import annotations

import json
import os
import shutil
import subprocess
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parents[2]
GIT_ENV = {
    "GIT_AUTHOR_NAME": "t",
    "GIT_AUTHOR_EMAIL": "t@t",
    "GIT_COMMITTER_NAME": "t",
    "GIT_COMMITTER_EMAIL": "t@t",
}
CLEAN_RUST = "/// Answer.\n#[must_use]\npub fn answer() -> u32 {\n    42\n}\n"
CLEAN_PYTHON = '"""Module."""\n\n\ndef double(x: int) -> int:\n    return 2 * x\n'


def git(cwd: Path, *args: str) -> None:
    subprocess.run(["git", *args], cwd=cwd, env={**os.environ, **GIT_ENV}, check=True)


def run(
    project: Path, active: bool = False, session: str = "s1", **env: str
) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        [str(project / ".claude" / "hooks" / "quality-gate.sh")],
        input=json.dumps({"stop_hook_active": active, "session_id": session}),
        capture_output=True,
        text=True,
        env={**os.environ, "CLAUDE_PROJECT_DIR": str(project), **env},
        cwd=project,
        check=False,
    )


@pytest.fixture
def project(tmp_path: Path) -> Path:
    """A committed project with one crate, one script and the Python tools of the repository."""
    shutil.copytree(ROOT / ".claude" / "hooks", tmp_path / ".claude" / "hooks")
    shutil.copy(ROOT / "pyproject.toml", tmp_path / "pyproject.toml")
    if (ROOT / ".venv").is_dir():
        (tmp_path / ".venv").symlink_to(ROOT / ".venv")
    (tmp_path / ".gitignore").write_text("/target\n.venv\n", encoding="utf-8")
    (tmp_path / "Cargo.toml").write_text(
        '[workspace]\nmembers = ["crates/demo"]\nresolver = "3"\n', encoding="utf-8"
    )
    crate = tmp_path / "crates" / "demo"
    (crate / "src").mkdir(parents=True)
    (crate / "Cargo.toml").write_text(
        '[package]\nname = "demo"\nversion = "0.0.0"\nedition = "2024"\n', encoding="utf-8"
    )
    (crate / "src" / "lib.rs").write_text(CLEAN_RUST, encoding="utf-8")
    (tmp_path / "scripts").mkdir()
    (tmp_path / "scripts" / "tool.py").write_text(CLEAN_PYTHON, encoding="utf-8")
    git(tmp_path, "init", "-q", "-b", "main")
    git(tmp_path, "add", ".")
    git(tmp_path, "commit", "-q", "-m", "base")
    return tmp_path


def test_silent_when_nothing_changed(project: Path) -> None:
    result = run(project)
    assert result.returncode == 0
    assert result.stderr == ""


def test_passes_when_the_changed_files_are_clean(project: Path) -> None:
    (project / "scripts" / "tool.py").write_text(
        CLEAN_PYTHON + "\n\ndef triple(x: int) -> int:\n    return 3 * x\n", encoding="utf-8"
    )
    (project / "scripts" / "new.py").write_text(CLEAN_PYTHON, encoding="utf-8")
    result = run(project)
    assert result.returncode == 0, result.stderr


@pytest.mark.skipif(not (ROOT / ".venv").is_dir(), reason="the Python tools are in .venv")
def test_blocks_on_a_ruff_error_in_a_changed_file(project: Path) -> None:
    (project / "scripts" / "tool.py").write_text(
        CLEAN_PYTHON + "\n\ndef unused() -> None:\n    import os\n", encoding="utf-8"
    )
    result = run(project)
    assert result.returncode == 2
    assert "ruff check" in result.stderr and "imported but unused" in result.stderr


@pytest.mark.skipif(not (ROOT / ".venv").is_dir(), reason="the Python tools are in .venv")
def test_blocks_on_unformatted_and_untyped_untracked_python(project: Path) -> None:
    (project / "scripts" / "new.py").write_text(
        '"""Module."""\n\n\ndef double(x):\n    return {  "a":x }\n', encoding="utf-8"
    )
    result = run(project)
    assert result.returncode == 2
    assert "ruff format" in result.stderr and "mypy" in result.stderr


@pytest.mark.skipif(shutil.which("cargo") is None, reason="cargo is needed for clippy")
def test_blocks_on_a_clippy_error_in_a_changed_crate(project: Path) -> None:
    (project / "crates" / "demo" / "src" / "lib.rs").write_text(
        CLEAN_RUST + "\n/// Empty.\n#[must_use]\npub fn empty(v: &[u32]) -> bool {\n"
        "    v.len() == 0\n}\n",
        encoding="utf-8",
    )
    result = run(project)
    assert result.returncode == 2
    assert "cargo clippy -p demo" in result.stderr and "len_zero" in result.stderr


@pytest.mark.skipif(shutil.which("rustfmt") is None, reason="rustfmt is needed")
def test_blocks_on_unformatted_rust(project: Path) -> None:
    (project / "crates" / "demo" / "src" / "lib.rs").write_text(
        CLEAN_RUST.replace("    42", "42"), encoding="utf-8"
    )
    result = run(project)
    assert result.returncode == 2
    assert "rustfmt --check" in result.stderr


def system_message(result: subprocess.CompletedProcess[str]) -> str:
    return str(json.loads(result.stdout)["systemMessage"])


@pytest.mark.skipif(not (ROOT / ".venv").is_dir(), reason="the Python tools are in .venv")
def test_changed_findings_block_again_even_when_a_stop_hook_is_active(project: Path) -> None:
    tool = project / "scripts" / "tool.py"
    tool.write_text("import os\n", encoding="utf-8")
    assert run(project).returncode == 2
    tool.write_text("import sys\n", encoding="utf-8")
    result = run(project, active=True)
    assert result.returncode == 2
    assert "imported but unused" in result.stderr


@pytest.mark.skipif(not (ROOT / ".venv").is_dir(), reason="the Python tools are in .venv")
def test_identical_findings_on_a_repeated_stop_do_not_block(project: Path) -> None:
    (project / "scripts" / "tool.py").write_text("import os\n", encoding="utf-8")
    assert run(project).returncode == 2
    result = run(project, active=True)
    assert result.returncode == 0
    assert "imported but unused" in system_message(result)
    assert run(project, session="s2").returncode == 2  # another session starts afresh


@pytest.mark.skipif(not (ROOT / ".venv").is_dir(), reason="the Python tools are in .venv")
def test_blocks_at_most_three_times_per_session(project: Path) -> None:
    tool = project / "scripts" / "tool.py"
    codes = []
    for module in ("os", "sys", "re", "json"):
        tool.write_text(f"import {module}\n", encoding="utf-8")
        codes.append(run(project).returncode)
    assert codes == [2, 2, 2, 0]


@pytest.mark.skipif(not (ROOT / ".venv").is_dir(), reason="the Python tools are in .venv")
def test_a_timeout_is_reported_without_blocking(project: Path) -> None:
    (project / "scripts" / "tool.py").write_text("import os\n", encoding="utf-8")
    for budget in ("0", "0.001"):
        result = run(project, WAYLOOP_QUALITY_BUDGET=budget)
        assert result.returncode == 0
        assert "time budget" in system_message(result)
        assert result.stderr == ""


@pytest.mark.skipif(not (ROOT / ".venv").is_dir(), reason="the Python tools are in .venv")
def test_an_invalid_budget_falls_back_to_the_default(project: Path) -> None:
    (project / "scripts" / "tool.py").write_text("import os\n", encoding="utf-8")
    assert run(project, WAYLOOP_QUALITY_BUDGET="soon").returncode == 2


def test_a_missing_clippy_or_rustfmt_component_never_blocks(
    project: Path, tmp_path_factory: pytest.TempPathFactory
) -> None:
    bin_dir = tmp_path_factory.mktemp("bin")
    for tool in ("cargo", "rustfmt"):
        fake = bin_dir / tool
        fake.write_text("#!/bin/sh\necho 'component not installed' >&2\nexit 1\n", encoding="utf-8")
        fake.chmod(0o755)
    (project / "crates" / "demo" / "src" / "lib.rs").write_text("pub fn  f(){}\n", encoding="utf-8")
    result = run(project, PATH=f"{bin_dir}:{os.environ['PATH']}")
    assert result.returncode == 0
    message = system_message(result)
    assert "rustfmt --check: tool not found" in message
    assert "cargo clippy -p demo: tool not found" in message


@pytest.mark.skipif(not (ROOT / ".venv").is_dir(), reason="the Python tools are in .venv")
def test_a_staged_rename_and_a_non_ascii_name_are_checked(project: Path) -> None:
    git(project, "mv", "scripts/tool.py", "scripts/renamed.py")
    (project / "scripts" / "renamed.py").write_text("import os\n", encoding="utf-8")
    git(project, "add", "-A")
    (project / "scripts" / "r\u00e9sum\u00e9.py").write_text("import sys\n", encoding="utf-8")
    result = run(project)
    assert result.returncode == 2
    assert "scripts/renamed.py" in result.stderr
    assert "r\u00e9sum\u00e9.py" in result.stderr


def test_deleted_files_and_other_files_are_ignored(project: Path) -> None:
    (project / "scripts" / "tool.py").unlink()
    (project / "notes.md").write_text("text\n", encoding="utf-8")
    result = run(project)
    assert result.returncode == 0
    assert result.stderr == ""


def test_never_blocks_outside_a_git_repository(tmp_path: Path) -> None:
    shutil.copytree(ROOT / ".claude" / "hooks", tmp_path / ".claude" / "hooks")
    assert run(tmp_path).returncode == 0
