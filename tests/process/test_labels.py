"""Every label the process relies on is planned in the roadmap file (and so created on GitHub)."""

from __future__ import annotations

import re
from pathlib import Path

import yaml

ROOT = Path(__file__).resolve().parents[2]


def planned() -> set[str]:
    data = yaml.safe_load((ROOT / "roadmap" / "roadmap.yaml").read_text(encoding="utf-8"))
    return {label["name"] for label in data["labels"]}


def used_by_workflows() -> set[str]:
    names: set[str] = set()
    for path in (ROOT / ".github" / "workflows").glob("*.yml"):
        names |= set(re.findall(r"labels\.\*\.name, '([^']+)'", path.read_text(encoding="utf-8")))
    return names


def used_by_labeler() -> set[str]:
    return set(yaml.safe_load((ROOT / ".github" / "labeler.yml").read_text(encoding="utf-8")))


def used_by_conventions() -> set[str]:
    page = (ROOT / "docs" / "en" / "project" / "conventions.md").read_text(encoding="utf-8")
    section = page.split("## Labels", 1)[1].split("\n## ", 1)[0]
    return set(re.findall(r"`([a-z][a-z :-]+)`", section)) - {"area:*"}


def test_labels_gating_checks_are_planned() -> None:
    assert used_by_workflows() <= planned(), used_by_workflows() - planned()
    assert {"no-release-note", "no-docs-needed", "translation-pending"} <= used_by_workflows()


def test_labels_applied_by_the_labeler_are_planned() -> None:
    assert used_by_labeler() <= planned(), used_by_labeler() - planned()


def test_labels_documented_in_the_conventions_are_planned() -> None:
    assert used_by_conventions() <= planned(), used_by_conventions() - planned()


def test_planned_labels_are_documented() -> None:
    documented = used_by_conventions() | {n for n in planned() if n.startswith("area:")}
    assert planned() <= documented, planned() - documented
