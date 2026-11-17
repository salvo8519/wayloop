"""Issue forms in .github/ISSUE_TEMPLATE follow the GitHub form schema and the roadmap labels."""

from __future__ import annotations

from pathlib import Path
from typing import Any

import pytest
import yaml

ROOT = Path(__file__).resolve().parents[2]
FORMS = sorted(
    p for p in (ROOT / ".github" / "ISSUE_TEMPLATE").glob("*.yml") if p.name != "config.yml"
)
TYPES = {"markdown", "input", "textarea", "dropdown", "checkboxes"}


def labels() -> set[str]:
    data = yaml.safe_load((ROOT / "roadmap" / "roadmap.yaml").read_text(encoding="utf-8"))
    return {label["name"] for label in data["labels"]}


def load(path: Path) -> dict[str, Any]:
    data = yaml.safe_load(path.read_text(encoding="utf-8"))
    assert isinstance(data, dict), path.name
    return data


def test_the_three_forms_exist() -> None:
    assert [p.name for p in FORMS] == ["bug_report.yml", "spec_request.yml", "task.yml"]


@pytest.mark.parametrize("path", FORMS, ids=lambda p: p.name)
def test_form_follows_the_schema(path: Path) -> None:
    form = load(path)
    assert {"name", "description", "labels", "body"} <= set(form)
    ids = []
    for element in form["body"]:
        assert element["type"] in TYPES
        if element["type"] == "markdown":
            assert element["attributes"]["value"].strip()
            continue
        ids.append(element["id"])
        assert element["attributes"]["label"]
        if element["type"] in ("dropdown", "checkboxes"):
            assert element["attributes"]["options"]
    assert len(ids) == len(set(ids)), f"{path.name}: repeated ids"
    assert any(e.get("validations", {}).get("required") for e in form["body"])


@pytest.mark.parametrize("path", FORMS, ids=lambda p: p.name)
def test_form_labels_are_planned(path: Path) -> None:
    assert set(load(path)["labels"]) <= labels()


def test_task_areas_match_the_area_labels() -> None:
    task = load(ROOT / ".github" / "ISSUE_TEMPLATE" / "task.yml")
    area = next(e for e in task["body"] if e.get("id") == "area")
    options = {f"area:{o}" for o in area["attributes"]["options"]}
    assert options == {name for name in labels() if name.startswith("area:")}


def test_blank_issues_are_disabled() -> None:
    config = load(ROOT / ".github" / "ISSUE_TEMPLATE" / "config.yml")
    assert config["blank_issues_enabled"] is False
    assert all(link["url"].startswith("https://") for link in config["contact_links"])
