"""scripts/sync_agents.py: .agents/ is the only source; the .claude/ mirrors are generated."""

from __future__ import annotations

import importlib.util
import shutil
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parents[2]
_spec = importlib.util.spec_from_file_location("sync_agents", ROOT / "scripts/sync_agents.py")
assert _spec and _spec.loader
sa = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(sa)


@pytest.fixture
def repo(tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> Path:
    shutil.copytree(ROOT / ".agents", tmp_path / ".agents")
    monkeypatch.setattr(sa, "ROOT", tmp_path)
    monkeypatch.setattr(sa, "SRC", tmp_path / ".agents")
    monkeypatch.setattr(sa, "DST", tmp_path / ".claude")
    return tmp_path


def run(*args: str) -> None:
    import sys

    sys.argv = ["sync_agents.py", *args]
    sa.main()


def test_the_committed_mirrors_are_in_sync() -> None:
    files = sa.expected()
    assert files, "no roles or skills found"
    assert all(p.exists() and p.read_text(encoding="utf-8") == c for p, c in files.items())


def test_generation_then_check_passes(repo: Path) -> None:
    run()
    run("--check")
    mirror = repo / ".claude" / "agents" / "reviewer.md"
    text = mirror.read_text(encoding="utf-8")
    assert text.startswith("---\n") and "Generated from .agents/agents/reviewer.md" in text


def test_a_hand_edited_mirror_fails_the_check(repo: Path) -> None:
    run()
    mirror = repo / ".claude" / "agents" / "reviewer.md"
    mirror.write_text(mirror.read_text(encoding="utf-8") + "\nEdited by hand.\n", encoding="utf-8")
    with pytest.raises(SystemExit, match="out of sync"):
        run("--check")


def test_a_mirror_without_source_fails_the_check(repo: Path) -> None:
    run()
    (repo / ".claude" / "agents" / "orphan.md").write_text("x\n", encoding="utf-8")
    with pytest.raises(SystemExit, match="orphan.md"):
        run("--check")


def test_a_missing_mirror_fails_the_check(repo: Path) -> None:
    with pytest.raises(SystemExit, match="out of sync"):
        run("--check")
