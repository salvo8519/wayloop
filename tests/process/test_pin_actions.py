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
