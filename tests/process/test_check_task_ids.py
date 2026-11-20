"""scripts/check_task_ids.py: every task id of a tasks.md file is defined once."""

from __future__ import annotations

from pathlib import Path

import check_task_ids as ids
import pytest


def tasks_file(tmp_path: Path, text: str, monkeypatch: pytest.MonkeyPatch) -> Path:
    path = tmp_path / "specs" / "0001-x" / "tasks.md"
    path.parent.mkdir(parents=True)
    path.write_text(text, encoding="utf-8")
    monkeypatch.setattr(ids, "ROOT", tmp_path)
    monkeypatch.setattr(ids, "tracked_tasks_files", lambda: [path])
    return path


def test_unique_ids_pass(tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> None:
    tasks_file(tmp_path, "- [ ] T1 first\n- [x] T2 second, after T1\n", monkeypatch)
    ids.main()


def test_a_repeated_definition_is_refused_with_its_lines(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    tasks_file(tmp_path, "- [ ] T1 first\n- [ ] T2 second\n- [X] T1 again\n", monkeypatch)
    with pytest.raises(SystemExit, match="T1 is defined on lines 1, 3"):
        ids.main()


def test_a_reference_is_not_a_definition(tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> None:
    tasks_file(tmp_path, "- [ ] T1 first\n\nT1 depends on nothing; see T1.\n", monkeypatch)
    ids.main()
