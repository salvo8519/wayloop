"""scripts/pin_actions.py --check: every action is pinned to a commit."""

from __future__ import annotations

from pathlib import Path

import pin_actions as pins
import pytest

SHA = "3d3c42e5aac5ba805825da76410c181273ba90b1"


def workflow(tmp_path: Path, monkeypatch: pytest.MonkeyPatch, *uses: str) -> None:
    path = tmp_path / ".github" / "workflows" / "ci.yml"
    path.parent.mkdir(parents=True)
    steps = "\n".join(f"      - uses: {u}" for u in uses)
    path.write_text(f"jobs:\n  a:\n    steps:\n{steps}\n", encoding="utf-8")
    monkeypatch.setattr(pins, "ROOT", tmp_path)
    monkeypatch.setattr(pins, "WORKFLOWS", [path])


def test_the_committed_workflows_are_pinned() -> None:
    assert pins.unpinned() == []


def test_a_tag_or_a_branch_is_reported(tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> None:
    workflow(tmp_path, monkeypatch, f"actions/checkout@{SHA} # v7", "actions/cache@v4", "x/y@main")
    assert pins.unpinned() == [
        ".github/workflows/ci.yml:5: actions/cache@v4",
        ".github/workflows/ci.yml:6: x/y@main",
    ]


def test_local_actions_need_no_pin(tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> None:
    workflow(tmp_path, monkeypatch, "./.github/actions/setup")
    assert pins.unpinned() == []


def test_check_exits_with_an_error(tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> None:
    workflow(tmp_path, monkeypatch, "actions/cache@v4")
    monkeypatch.setattr("sys.argv", ["pin_actions.py", "--check"])
    with pytest.raises(SystemExit):
        pins.main()


def test_the_composite_actions_are_checked_too(tmp_path: Path) -> None:
    assert any(p.name == "action.yml" for p in pins.WORKFLOWS)
    for name in (
        "workflows/a.yml",
        "workflows/b.yaml",
        "actions/c/action.yml",
        "actions/d/action.yaml",
    ):
        (tmp_path / ".github" / name).parent.mkdir(parents=True, exist_ok=True)
        (tmp_path / ".github" / name).write_text("", encoding="utf-8")
    found = [p.relative_to(tmp_path / ".github").as_posix() for p in pins.workflow_files(tmp_path)]
    assert found == sorted(
        ["actions/c/action.yml", "actions/d/action.yaml", "workflows/a.yml", "workflows/b.yaml"]
    )
