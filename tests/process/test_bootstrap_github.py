"""scripts/bootstrap_github.py: the plan is reproduced on a repository, offline, fake GitHub."""

from __future__ import annotations

import importlib.util
import sys
from pathlib import Path
from typing import Any

import pytest

ROOT = Path(__file__).resolve().parents[2]
_spec = importlib.util.spec_from_file_location("bootstrap", ROOT / "scripts/bootstrap_github.py")
assert _spec and _spec.loader
bs = importlib.util.module_from_spec(_spec)
sys.modules["bootstrap"] = bs  # dataclasses resolve their module by name
_spec.loader.exec_module(bs)

DATA: dict[str, Any] = {
    "labels": [{"name": "epic", "color": "000000"}, {"name": "area:core", "color": "111111"}],
    "milestones": [
        {
            "title": "M1 · Engine core",
            "gate": "Replay is stable.",
            "release": "0.1.0",
            "epics": [
                {
                    "title": "Ledger",
                    "issue": 10,
                    "labels": ["area:core"],
                    "tasks": [
                        {"issue": 11, "title": "Append-only writer", "status": "open"},
                        {
                            "issue": 12,
                            "title": "Replay and snapshots",
                            "tags": ["parallel"],
                            "status": "open",
                        },
                    ],
                }
            ],
        }
    ],
}


class FakeGitHub:
    """Issues, milestones, labels, sub-issues and dependencies kept in memory."""

    def __init__(self, issues: dict[int, dict[str, Any]] | None = None) -> None:
        self.dry_run = False
        self.issues_ = issues or {}
        self.labels: set[str] = set()
        self.milestones: dict[str, int] = {}
        self.subs: dict[int, set[int]] = {}
        self.deps: dict[int, set[int]] = {}
        self.writes = 0

    def issues(self) -> dict[int, dict[str, Any]]:
        return dict(self.issues_)

    def pages(self, path: str) -> list[dict[str, Any]]:
        if path.startswith("milestones"):
            return [{"title": t, "number": n} for t, n in self.milestones.items()]
        if path.endswith("/sub_issues"):
            return [{"number": n} for n in self.subs.get(int(path.split("/")[1]), set())]
        if path.endswith("/blocked_by"):
            return [{"number": n} for n in self.deps.get(int(path.split("/")[1]), set())]
        return [{"name": n} for n in self.labels]

    def api(self, method: str, path: str, payload: dict[str, Any] | None = None) -> Any:
        self.writes += 1
        payload = payload or {}
        by_id = {i["id"]: n for n, i in self.issues_.items()}
        if path == "labels":
            self.labels.add(payload["name"])
        elif path == "milestones":
            self.milestones[payload["title"]] = len(self.milestones) + 1
        elif path == "issues":
            number = max(self.issues_, default=0) + 1
            self.issues_[number] = {"number": number, "id": 1000 + number, **payload}
            return self.issues_[number]
        elif path.endswith("/sub_issues"):
            self.subs.setdefault(int(path.split("/")[1]), set()).add(by_id[payload["sub_issue_id"]])
        elif path.endswith("/blocked_by"):
            self.deps.setdefault(int(path.split("/")[1]), set()).add(by_id[payload["issue_id"]])
        elif method == "PATCH":
            self.issues_[int(path.split("/")[1])].update(payload)
        return {}


def run(target: FakeGitHub, source: FakeGitHub | None = None) -> int:
    p = bs.plan(DATA, set(target.labels), set(target.milestones), target.issues())
    changes: int = bs.apply(p, target, target.issues_, source)
    return changes


def source_repo() -> FakeGitHub:
    body = (
        "Part of #10.\n\n## Outcome\nReplay.\n\n## Depends on\n- #11\n\n"
        "## Definition of done\nCloses #12"
    )
    return FakeGitHub(
        {
            10: {"number": 10, "id": 10, "title": "Ledger", "body": "Epic."},
            11: {"number": 11, "id": 11, "title": "Append-only writer", "body": "Part of #10."},
            12: {"number": 12, "id": 12, "title": "Replay and snapshots", "body": body},
        }
    )


def test_an_empty_repository_receives_the_whole_plan_with_bodies_and_dependencies() -> None:
    target = FakeGitHub({1: {"number": 1, "id": 1001, "title": "Unrelated", "body": ""}})
    run(target, source_repo())
    by_title = {i["title"]: n for n, i in target.issues_.items()}
    epic, writer, replay = (
        by_title["Ledger"],
        by_title["Append-only writer"],
        by_title["Replay and snapshots"],
    )
    assert target.labels == {"epic", "area:core"}
    assert list(target.milestones) == ["M1 · Engine core"]
    assert target.subs[epic] == {writer, replay}
    assert target.deps[replay] == {writer}
    body = target.issues_[replay]["body"]
    assert f"Part of #{epic}." in body and f"- #{writer}" in body and f"Closes #{replay}" in body
    assert "parallel" in target.issues_[replay]["labels"]
    assert "epic" in target.issues_[epic]["labels"]


def test_a_second_run_changes_nothing() -> None:
    target = FakeGitHub()
    run(target, source_repo())
    before = target.writes
    assert run(target, source_repo()) == 0
    assert target.writes == before


def test_issues_match_by_number_then_by_title() -> None:
    issues = {
        10: {"number": 10, "id": 10, "title": "Ledger"},
        11: {"number": 11, "id": 11, "title": "Something else"},
        30: {"number": 30, "id": 30, "title": "append-only WRITER"},
    }
    p = bs.plan(DATA, set(), set(), issues)
    how = {i.title: (i.number, i.how) for i in p.items}
    assert how["Ledger"] == (10, "number")
    assert how["Append-only writer"] == (30, "title")
    assert how["Replay and snapshots"] == (None, "create")


def test_milestone_description_carries_gate_and_release() -> None:
    p = bs.plan(DATA, set(), set(), {})
    assert p.milestones == [
        {"title": "M1 · Engine core", "description": "Gate: Replay is stable. Release 0.1.0."}
    ]


def test_dependencies_are_read_only_under_their_heading() -> None:
    body = "See #4.\n\n## Depends on\n- #7\n- #9 (the ledger)\n\n## Definition of done\n- #5"
    assert bs.depends_on(body) == [7, 9]
    assert bs.depends_on("No heading, #3") == []
    assert bs.renumber("#7 and #70 and #9", {7: 17, 9: 19}) == "#17 and #70 and #19"


def test_an_unmatched_item_has_no_target() -> None:
    item = bs.Item(kind="task", planned=12, title="Ledger replay", milestone="M1", labels=[])
    with pytest.raises(bs.UnmatchedItemError, match="task 'Ledger replay' has no issue"):
        item.target()


def test_a_matched_item_targets_its_issue() -> None:
    item = bs.Item(kind="epic", planned=3, title="Ledger", milestone="M1", labels=[], number=57)
    assert item.target() == 57
