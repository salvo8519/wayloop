"""scripts/check_licences.py: the licences of deny.toml are allowed by Dependency review."""

from __future__ import annotations

from pathlib import Path

import check_licences as lic
import pytest

WORKFLOW = """\
jobs:
  review:
    steps:
      - uses: actions/checkout@abc
      - uses: actions/dependency-review-action@def # v5
        with:
          allow-licenses: {allowed}
"""


def repository(tmp_path: Path, deny: str, review: str, monkeypatch: pytest.MonkeyPatch) -> None:
    (tmp_path / "deny.toml").write_text(f"[licenses]\nallow = {deny}\n", encoding="utf-8")
    workflow = tmp_path / ".github" / "workflows" / "security.yml"
    workflow.parent.mkdir(parents=True)
    workflow.write_text(WORKFLOW.format(allowed=review), encoding="utf-8")
    monkeypatch.setattr(lic, "ROOT", tmp_path)


def test_the_repository_lists_agree() -> None:
    lic.main()


def test_review_may_allow_more(tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> None:
    repository(tmp_path, '["MIT", "MIT-0"]', "MIT, MIT-0, PSF-2.0", monkeypatch)
    lic.main()


def test_a_licence_missing_from_review_is_refused(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    repository(tmp_path, '["MIT", "MIT-0"]', "MIT", monkeypatch)
    with pytest.raises(SystemExit, match="MIT-0"):
        lic.main()


def test_an_exception_counts_by_its_base_identifier(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    repository(tmp_path, '["Apache-2.0 WITH LLVM-exception"]', "Apache-2.0", monkeypatch)
    lic.main()


def test_a_workflow_without_the_review_step_is_refused(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    repository(tmp_path, '["MIT"]', "MIT", monkeypatch)
    (tmp_path / ".github/workflows/security.yml").write_text("jobs: {}\n", encoding="utf-8")
    with pytest.raises(SystemExit, match="no actions/dependency-review-action"):
        lic.main()
