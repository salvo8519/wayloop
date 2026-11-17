"""Automated review workflows: guarded, least-privilege, and pointing at files that exist."""

from __future__ import annotations

import re
from pathlib import Path

import yaml

ROOT = Path(__file__).resolve().parents[2]
WORKFLOWS = {
    name: yaml.safe_load((ROOT / ".github" / "workflows" / name).read_text(encoding="utf-8"))
    for name in ("claude.yml", "claude-review.yml", "claude-docs.yml")
}


def text(name: str) -> str:
    return (ROOT / ".github" / "workflows" / name).read_text(encoding="utf-8")


def test_files_named_in_the_prompts_exist() -> None:
    for name in WORKFLOWS:
        for path in re.findall(r"(?:\.agents|docs|scripts|specs)/[\w./-]+\.(?:md|py)", text(name)):
            assert (ROOT / path).exists(), f"{name} names {path}, which does not exist"


def test_no_permission_at_workflow_level() -> None:
    assert all(w["permissions"] == {} for w in WORKFLOWS.values())


def test_review_is_skipped_without_the_secret() -> None:
    steps = WORKFLOWS["claude-review.yml"]["jobs"]["review"]["steps"]
    assert all(s.get("if") == "steps.secret.outputs.present == 'true'" for s in steps[1:])


def test_mentions_run_only_for_people_with_write_access() -> None:
    condition = WORKFLOWS["claude.yml"]["jobs"]["claude"]["if"]
    assert condition.count("author_association") == 3
    assert '"OWNER","MEMBER","COLLABORATOR"' in condition


def test_docs_sync_runs_only_on_branches_of_this_repository() -> None:
    condition = WORKFLOWS["claude-docs.yml"]["jobs"]["docs"]["if"]
    assert "head.repo.full_name == github.repository" in condition
    assert "'docs-sync'" in condition
